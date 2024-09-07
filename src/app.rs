use crate::file_blame::{Commit, FileBlame, FileBlameError};
use ratatui::layout::Constraint;
use ratatui::style::{Color, Style};
use ratatui::widgets::TableState;
use std::collections::HashMap;
use std::error;

/// Application result type.
pub type AppResult<T> = std::result::Result<T, Box<dyn error::Error>>;

/// Application.
#[derive(Debug)]
pub struct App {
    /// Is the application running?
    pub running: bool,

    pub state: TableState,
    pub file_path: String,
    pub commit_sha: String,
    pub file_blame: Option<FileBlame>,
    pub commit_cache: HashMap<String, Commit>,
    pub commit_stack: Vec<String>,
    pub load_err: Option<FileBlameError>,
    pub columns: Vec<Column>,
}

// Column definition including the column width, style, and header name.
#[derive(Debug)]
pub struct Column {
    pub width: Constraint,
    pub style: Style,
    pub name: String,
}

impl Column {
    pub fn header_name(&self) -> String {
        self.name.to_owned()
    }
}

impl App {
    /// Constructs a new instance of [`App`].
    pub fn new(file_path: String, commit_sha: String) -> Self {
        let mut app = App {
            state: TableState::default(),
            file_path: file_path.clone(),
            commit_sha: commit_sha.clone(),
            commit_cache: HashMap::new(),
            file_blame: None,
            commit_stack: Vec::new(),
            load_err: None,
            running: true,
            columns: vec![
                // All columns have fixed width except the last one which is for the contents.
                // The last column will take up the remaining width of the table.
                Column {
                    width: Constraint::Max(10),
                    style: Style::default().fg(Color::Blue),
                    name: "TIME".to_string(),
                },
                Column {
                    width: Constraint::Max(15),
                    style: Style::default().fg(Color::Red),
                    name: "AUTHOR".to_string(),
                },
                Column {
                    width: Constraint::Max(8),
                    style: Style::default().fg(Color::Green),
                    name: "COMMIT".to_string(),
                },
                Column {
                    width: Constraint::Max(30),
                    style: Style::default().fg(Color::Green),
                    name: "MESSAGE".to_string(),
                },
                Column {
                    width: Constraint::Max(5),
                    style: Style::default().fg(Color::Yellow),
                    name: "LINE".to_string(),
                },
                Column {
                    width: Constraint::Fill(1000),
                    style: Style::default(),
                    name: "CONTENTS".to_string(),
                },
            ],
        };

        app.load_blame(file_path, commit_sha);
        app
    }

    /// Handles the tick event of the terminal.
    pub fn tick(&self) {}

    /// Set running to false to quit the application.
    pub fn quit(&mut self) {
        self.running = false;
    }

    // Load the blame information for the given file path and commit sha.
    // Keep the line with the same number selected if it's still around after
    // loading the new blame information.
    fn load_blame(&mut self, file_path: String, commit_sha: String) {
        let file_blame = match FileBlame::parse(&file_path, &commit_sha, &mut self.commit_cache) {
            Ok(f) => f,
            Err(e) => {
                self.load_err = Some(e);
                self.quit();
                return;
            }
        };

        self.file_blame = Some(file_blame);
        self.file_path = file_path;
        self.commit_sha = commit_sha;

        let i = match self.state.selected() {
            Some(i) => {
                let len = self.file_blame.as_ref().unwrap().blame_lines.len();
                if i >= len - 1 {
                    len - 1
                } else {
                    i
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    // Move selection to the first line of the next block. A block is a group of lines
    // with the same commit sha. Since only the first line of a block shows the commit
    // information, moving to the next block basically means moving to the next line
    // with a different commit sha i.e the next line with visible commit information.
    pub fn next_block(&mut self) {
        let next_index = match self.state.selected() {
            Some(mut current_index) => {
                if current_index >= self.file_blame.as_ref().unwrap().blame_lines.len() - 1 {
                    self.file_blame.as_ref().unwrap().blame_lines.len() - 1
                } else {
                    let current_sha = &self
                        .file_blame
                        .as_ref()
                        .unwrap()
                        .blame_lines
                        .get(current_index)
                        .unwrap()
                        .commit_sha;

                    current_index += 1;

                    while (current_index < self.file_blame.as_ref().unwrap().blame_lines.len())
                        && (current_sha
                            == &self
                                .file_blame
                                .as_ref()
                                .unwrap()
                                .blame_lines
                                .get(current_index)
                                .unwrap()
                                .commit_sha)
                    {
                        current_index += 1;
                    }

                    current_index
                }
            }
            None => 0,
        };
        self.state.select(Some(next_index));
    }

    // Move selection to the first line of the previous block.
    pub fn previous_block(&mut self) {
        let next_index = match self.state.selected() {
            Some(mut current_index) => {
                if current_index <= 1 {
                    0
                } else {
                    let current_sha = &self
                        .file_blame
                        .as_ref()
                        .unwrap()
                        .blame_lines
                        .get(current_index - 1)
                        .unwrap()
                        .commit_sha;

                    current_index -= 1;

                    while (current_index > 0)
                        && (current_sha
                            == &self
                                .file_blame
                                .as_ref()
                                .unwrap()
                                .blame_lines
                                .get(current_index - 1)
                                .unwrap()
                                .commit_sha)
                    {
                        current_index -= 1;
                    }

                    current_index
                }
            }
            None => 0,
        };
        self.state.select(Some(next_index));
    }

    // Move selection to the next line.
    pub fn next_line(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.file_blame.as_ref().unwrap().blame_lines.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    // Move selection to the previous line.
    pub fn previous_line(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    self.file_blame.as_ref().unwrap().blame_lines.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    // Show the blame information for the same file, but at the parent commit of the
    // currently selected line's commit. In other words, show how the file looked like
    // prior to the change that last modified the currently selected line. This is useful
    // for incrementally understanding how a file evolved over time. We keep track of the
    // current commit sha on a stack so that we can easily go back and take a different
    // path through the file's history if needed.
    pub fn next_commit(&mut self) {
        let i = self.state.selected().unwrap();
        let blame_line = self
            .file_blame
            .as_ref()
            .unwrap()
            .blame_lines
            .get(i)
            .unwrap();
        let commit_context = self.commit_cache.get(&blame_line.commit_sha).unwrap();

        // If the commit doesn't have a parent (i.e it's the initial commit), or if the file
        // didn't exist at the parent commit, then we can't show the blame at the parent commit.
        if commit_context.parent_commit_sha.is_none()
            || !FileBlame::exists_at_commit(
                &self.file_path,
                commit_context.parent_commit_sha.as_ref().unwrap(),
            )
        {
            return;
        }

        self.commit_stack.push(self.commit_sha.clone());

        self.load_blame(
            self.file_path.clone(),
            commit_context.parent_commit_sha.as_ref().unwrap().clone(),
        );
    }

    // Go back to the previous commit in the commit stack.
    pub fn previous_commit(&mut self) {
        if let Some(sha) = self.commit_stack.pop() {
            self.load_blame(self.file_path.clone(), sha)
        }
    }
}
