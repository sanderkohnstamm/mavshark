mod mavlink_listener;
mod mavlink_monitor;

use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use mavlink_monitor::MavlinkMonitor;
use std::{io, sync::Arc};
use tui::{backend::CrosstermBackend, Terminal};

fn main() {
    let monitor = Arc::new(MavlinkMonitor::new());
    enable_raw_mode().expect("Failed to enable raw mode");
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)
        .expect("Failed to enter alternate screen and enable mouse capture");
    let backend = CrosstermBackend::new(stdout);
    let mut terminal: Terminal<CrosstermBackend<io::Stdout>> =
        Terminal::new(backend).expect("Failed to create terminal");

    if let Err(e) = monitor.run(&mut terminal) {
        eprintln!("Error: {}", e);
    }
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )
    .expect("Failed to leave alternate screen and disable mouse capture");
}
