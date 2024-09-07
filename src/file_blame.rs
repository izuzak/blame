use regex::Regex;
use std::collections::HashMap;
use std::error::Error;
use std::fmt::Display;
use std::path::{Path, PathBuf};
use std::process::Command;

use syntect::easy::HighlightLines;
use syntect::highlighting::ThemeSet;
use syntect::parsing::SyntaxSet;
use syntect::util::as_24_bit_terminal_escaped;

// Regex for parsing a line of git blame output.
// A line of git blame output looks like this:
//
// 1e1d1c3c (John Doe 2019-01-01 12:00:00 -0400 142) This is the code
//
// ^^^^^^^^  ^^^^^^^^ ^^^^^^^^^^^^^^^^^^^^^^^^^ ^^^  ^^^^^^^^^^^^^^^^
//    |          |         |                     |     |
//    |          |         |                     |     +-- file content
//    |          |         |                     |
//    |          |         +-- timestamp         +-- line number
//    |          |
//    |          +-- author name
//    |
//    +-- commit hash
const BLAME_LINE_REGEX: &str = r"(?x)
^
  (\^?[0-9a-f]{4,40})     # commit sha
  (?: [^(]+)?             # optional file name
  [\ ]
  \(                      # open (
  ([^\ ].*[^\ ])          # author name
  [\ ]+
  (
    \d{4}-\d{2}-\d{2}\    # timestamp date
    \d{2}:\d{2}:\d{2}\    # timestamp time
    [-+]\d{4}             # timestamp offset
  )
  [\ ]+
  (\d+)                   # line number
  \)                      # close )
  [\ ]
  (.*)                    # file content
$";

// Metadata for a single Git commit. All commits have a parent,
// except the initial commit.
#[derive(PartialEq, Default, Clone, Debug)]
pub struct Commit {
    pub sha: String,
    pub author: String,
    pub commit_message: String,
    pub parent_commit_sha: Option<String>,
    pub timestamp: String,
}

// A single line for a Git blame of a specific file at a specific commit.
// Most information in a blame line is about the commit which introduced
// or last changed the line. And that commit information can be reused
// across multiple blame lines. So, this structure only holds the SHA
// of the commit which will be used to pull up the full information from
// a cache.
#[derive(PartialEq, Clone, Debug)]
pub struct BlameLine {
    pub commit_sha: String,
    pub contents: String,
    pub line_number: String,
}

// All lines for a Git blame of a specific file at a specific commit.
// The blame_lines vector contains the individual lines, while the
// filepath and commit_sha say which file and commit it is about.
#[derive(PartialEq, Clone, Debug)]
pub struct FileBlame {
    pub blame_lines: Vec<BlameLine>,
    pub filepath: String,
    pub commit_sha: String,
}

// Possible errors that can be returned when building a blame for a file.
#[derive(Debug, Clone)]
pub enum FileBlameError {
    NotExist,
    NotFile,
    NotGit,
    MissingAtCommit,
    Unknown(String),
}

impl Display for FileBlameError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FileBlameError::NotExist => write!(f, "File doesn't exist"),
            FileBlameError::NotFile => write!(f, "Provided path is not a regular file"),
            FileBlameError::NotGit => write!(f, "File is not in a git repository"),
            FileBlameError::MissingAtCommit => write!(f, "File does not exist at commit"),
            FileBlameError::Unknown(s) => write!(f, "Unknown error: {}", s),
        }
    }
}

impl Error for FileBlameError {}

impl FileBlame {
    // Check if a file exists at a specific commit.
    pub fn exists_at_commit(filepath: &str, commit_sha: &str) -> bool {
        // Split the filepath into two parts:
        //   - git_root_dir  - the root of the Git repository which contains the file
        //   - relative_path - the file's path relative to the root of the repository
        let path = Path::new(filepath).canonicalize().unwrap();
        let git_root_dir = FileBlame::git_root_dir(&path);
        let relative_path = path.strip_prefix(&git_root_dir).unwrap().to_str().unwrap();

        // Run the Git command for the check. If the file exists, there will be no
        // output and the status will be success (0). Otherwise, the output will
        // be an error message "fatal: invalid object name '32c2e2df'" if the commit
        // doesn't exist, or "fatal: path 'foobar.rs' does not exist in '32c2e2df'"
        // if the file doesn't exist at that commit. In both cases, an unsuccessful
        // status is returned (>0).
        return Command::new("git")
            .arg("cat-file")
            .current_dir(&git_root_dir)
            .arg("-e")
            .arg(format!("{}:{}", commit_sha, relative_path))
            .output()
            .unwrap()
            .status
            .success();
    }

    // Determine the root directory of a file in a Git repository. We
    // do this by first determining the parent directory containing the file
    // and then running a Git command in that directory to reveal the
    // root of the repository.
    fn git_root_dir(path: &PathBuf) -> String {
        let parent = path.parent().unwrap();

        let root_output = Command::new("git")
            .current_dir(parent)
            .arg("rev-parse")
            .arg("--show-toplevel")
            .output()
            .unwrap();

        if !root_output.status.success() {
            let stderr = String::from_utf8(root_output.stderr).unwrap();
            panic!("Error when determining root directory: {}", stderr);
        }

        return String::from_utf8(root_output.stdout)
            .unwrap()
            .trim_end()
            .to_string();
    }

    // Construct the blame for a file at a specific commit, and use a
    // cache for making things faster and not duplicating the same
    // commit information for multiple blame lines.
    pub fn parse(
        filepath: &str,
        commit_sha: &str,
        commit_cache: &mut HashMap<String, Commit>,
    ) -> Result<FileBlame, FileBlameError> {
        let path = Path::new(filepath).canonicalize().unwrap();

        if !path.exists() {
            return Err(FileBlameError::NotExist);
        }

        if !path.is_file() {
            return Err(FileBlameError::NotFile);
        }

        let parent = path.parent().unwrap();
        let git_root_dir = FileBlame::git_root_dir(&path);
        let filename = path.strip_prefix(&git_root_dir).unwrap().to_str().unwrap();

        // check if the file is in a Git repository
        if !Command::new("git")
            .current_dir(&git_root_dir)
            .arg("rev-parse")
            .arg("--is-inside-work-tree")
            .output()
            .unwrap()
            .status
            .success()
        {
            return Err(FileBlameError::NotGit);
        }

        // check if the file exists at the selected commit
        if !FileBlame::exists_at_commit(&filepath, &commit_sha) {
            return Err(FileBlameError::MissingAtCommit);
        }

        // fetch git blame for the file and commit
        let blame_output = Command::new("git")
            .arg("blame")
            .current_dir(&git_root_dir)
            .arg(commit_sha)
            .arg(filename)
            .output()
            .unwrap();

        if !blame_output.status.success() {
            let stderr = String::from_utf8(blame_output.stderr).unwrap();
            return Err(FileBlameError::Unknown(stderr));
        }

        // Prepare syntax highlighter
        let theme_set;
        let mut highlighter = None;
        let mut syntax_set = None;
        let extension = path.extension();

        match extension {
            None => {}
            Some(ext) => {
                syntax_set = Some(SyntaxSet::load_defaults_newlines());
                theme_set = Some(ThemeSet::load_defaults());
                let syntax = syntax_set
                    .as_mut()
                    .unwrap()
                    .find_syntax_by_extension(ext.to_str().unwrap())
                    .unwrap();
                highlighter = Some(HighlightLines::new(
                    syntax,
                    &theme_set.as_ref().unwrap().themes["base16-ocean.dark"],
                ));
            }
        }

        // Parse each line of blame output and apply syntax highlighting
        let blame_output = String::from_utf8(blame_output.stdout).unwrap();
        let blame_lines = blame_output.lines();
        let mut parsed_blame_lines: Vec<BlameLine> = vec![];

        for blame_line in blame_lines {
            let pattern = Regex::new(BLAME_LINE_REGEX).unwrap();
            let captures = pattern.captures(blame_line).unwrap();

            let commit = captures.get(1).unwrap().as_str();
            let author = captures.get(2).unwrap().as_str();
            let timestamp = captures.get(3).unwrap().as_str();
            let line_number = captures.get(4).unwrap().as_str();
            let mut line_contents = captures.get(5).unwrap().as_str().to_owned();

            if highlighter.is_some() {
                let ranges = highlighter
                    .as_mut()
                    .unwrap()
                    .highlight_line(&line_contents, &(syntax_set.as_mut().unwrap()))
                    .unwrap();
                line_contents = as_24_bit_terminal_escaped(&ranges[..], false);
            }

            // if commit starts with ^ it is a boundary commit
            // so we should remove that character
            let commit = commit.trim_start_matches("^");

            // Check the commit cache first to see if we've already fetched
            // the information for this commit. If not, then fetch the info
            // and store it in the cache.
            if !commit_cache.contains_key(commit) {
                let output = String::from_utf8(
                    Command::new("git")
                        .current_dir(parent)
                        .arg("show")
                        .arg(commit)
                        .arg("--pretty=format:%p-%s")
                        .arg("--no-patch")
                        .output()
                        .expect("failed to execute process")
                        .stdout,
                )
                .unwrap();

                let (parent_commit, commit_message) = output.split_once("-").unwrap();

                let parent_commit_sha = if parent_commit.is_empty() {
                    None
                } else {
                    Some(parent_commit.to_owned())
                };

                commit_cache.insert(
                    commit.to_owned(),
                    Commit {
                        author: author.to_owned(),
                        commit_message: commit_message.to_owned(),
                        timestamp: timestamp.to_owned(),
                        sha: commit.to_owned(),
                        parent_commit_sha,
                    },
                );
            }

            parsed_blame_lines.push(BlameLine {
                line_number: line_number.to_owned(),
                contents: line_contents,
                commit_sha: commit.to_owned(),
            });
        }

        Ok(FileBlame {
            commit_sha: commit_sha.to_owned(),
            filepath: filepath.to_owned(),
            blame_lines: parsed_blame_lines,
        })
    }
}
