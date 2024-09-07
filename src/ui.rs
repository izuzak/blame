use crate::{app::App, app::Column, file_blame::BlameLine, file_blame::Commit};
use ratatui::{
    layout::*,
    prelude::*,
    style::{Color, Style},
    widgets::*,
    Frame,
};
use std::collections::HashMap;
use std::str::FromStr;

// Divider cell between columns in a row.
fn divider_cell<'a>() -> Cell<'a> {
    Cell::from("│")
}

fn empty_cell<'a>() -> Cell<'a> {
    Cell::from("")
}

// Inserts a new item between each item in a vector.
// This is used for adding dividers between cells in a row.
fn insert_between<T>(items: Vec<T>, new_item: T) -> Vec<T>
where
    T: Clone,
{
    let mut out = Vec::new();
    for c in items {
        out.push(c);
        out.push(new_item.clone());
    }
    out.pop();
    out
}

/// Renders the user interface widgets.
pub fn render(app: &mut App, frame: &mut Frame) {
    let rects = Layout::default()
        .constraints([Constraint::Percentage(100)])
        .split(frame.size());

    let selected_style = Style::default().bg(Color::from_str("#3f3f3f").unwrap());

    // Set up the header row.
    let mut header_cells = app
        .columns
        .iter()
        .map(|c| c.header_name())
        .map(|h| Cell::from(h).style(Style::default().fg(Color::Red).bold()))
        .collect();
    header_cells = insert_between(header_cells, divider_cell());
    let header = Row::new(header_cells).height(1).bottom_margin(1);

    // Set up blame line rows
    let mut previous_sha = "".to_string();
    let file_blame = app.file_blame.as_ref().unwrap();
    let rows = file_blame.blame_lines.iter().map(|item| {
        let row = table_row_for_blame_line(
            &previous_sha,
            &item.commit_sha,
            item,
            &app.commit_cache,
            &app.columns,
        );
        previous_sha = item.commit_sha.clone();
        row
    });

    // Set up the column widths
    let mut widths: Vec<Constraint> = app.columns.iter().map(|c| c.width).collect();
    widths = insert_between(widths, Constraint::Max(1));

    // Create the whole table using the header, rows and column widths.
    let t = Table::new(rows, widths)
        .header(header)
        .column_spacing(1)
        .block(Block::default().borders(Borders::ALL).title(format!(
            "Blame for file: {} at ref: {}",
            app.file_path, app.commit_sha
        )))
        .highlight_style(selected_style);
    frame.render_stateful_widget(t, rects[0], &mut app.state);
}

// Creates a table row for a blame line and the previous line's commit sha
fn table_row_for_blame_line<'a>(
    previous_ref: &str,
    commit_sha: &'a str,
    item: &'a BlameLine,
    commit_cache: &'a HashMap<String, Commit>,
    columns: &[Column],
) -> Row<'a> {
    // If the commit sha of the current line matches the commit sha of the
    // previous line, then use empty cells for the timestamp, author, sha and
    // commit message. The effect of this is that only the first line of a block
    // of lines with the same commit will have the info shown which makes
    // for a cleaner UI experience.
    let mut cells = if item.commit_sha == previous_ref {
        vec![empty_cell(), empty_cell(), empty_cell(), empty_cell()]
    } else {
        let commit_context = commit_cache.get(&item.commit_sha).unwrap();

        vec![
            Cell::from(commit_context.timestamp.as_str()).style(columns[0].style),
            Cell::from(commit_context.author.as_str()).style(columns[1].style),
            Cell::from(commit_sha).green().style(columns[2].style),
            Cell::from(commit_context.commit_message.as_str()).style(columns[3].style),
        ]
    };

    let highlighted_text = ansi_to_tui::IntoText::to_text(&(item.contents)).unwrap();
    cells.push(Cell::from(item.line_number.as_str()).style(columns[4].style));
    cells.push(Cell::from(highlighted_text).style(columns[5].style));
    cells = insert_between(cells, divider_cell());
    Row::new(cells).height(1).bottom_margin(0)
}
