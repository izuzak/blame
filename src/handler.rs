use crate::app::{App, AppResult};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// Handles the key events and updates the state of [`App`].
pub fn handle_key_events(key_event: KeyEvent, app: &mut App) -> AppResult<()> {
    match key_event.code {
        // Exit application on `ESC` or `q`
        KeyCode::Esc | KeyCode::Char('q') => {
            app.quit();
        }
        // Exit application on `Ctrl-C`
        KeyCode::Char('c') | KeyCode::Char('C') => {
            if key_event.modifiers == KeyModifiers::CONTROL {
                app.quit();
            }
        }

        KeyCode::Char('{') => app.previous_block(),
        KeyCode::Char('}') => app.next_block(),
        KeyCode::Down => app.next_line(),
        KeyCode::Up => app.previous_line(),
        KeyCode::Left => app.next_commit(),
        KeyCode::Right => app.previous_commit(),

        _ => {}
    }
    Ok(())
}
