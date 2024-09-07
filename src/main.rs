use blame::app::{App, AppResult};
use blame::event::{Event, EventHandler};
use blame::handler::handle_key_events;
use blame::tui::Tui;
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::io;

use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// File path to display blame for.
    filepath: String,

    /// Ref for which to show blame for.
    #[arg(short, long, default_value = "HEAD")]
    gitref: String,
}

fn main() -> AppResult<()> {
    let args = Args::parse();

    // Create an application.
    let mut app = App::new(args.filepath, args.gitref);

    // Initialize the terminal user interface.
    let backend = CrosstermBackend::new(io::stderr());
    let terminal = Terminal::new(backend)?;
    let events = EventHandler::new(250);
    let mut tui = Tui::new(terminal, events);
    tui.init()?;

    // Start the main loop.
    while app.running {
        // Render the user interface.
        tui.draw(&mut app)?;
        // Handle events.
        match tui.events.next()? {
            Event::Tick => app.tick(),
            Event::Key(key_event) => handle_key_events(key_event, &mut app)?,
            Event::Mouse(_) => {}
            Event::Resize(_, _) => {}
        }
    }

    // Exit the user interface.
    tui.exit()?;

    if app.load_err.is_some() {
        println!("Error: {}", app.load_err.as_ref().unwrap());
    }
    Ok(())
}
