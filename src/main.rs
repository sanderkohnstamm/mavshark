mod app;
mod mavlink_io;
mod record;
mod replay;
mod ui;

use std::fs::File;
use std::io;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use anyhow::Result;
use clap::{Parser, Subcommand};
use crossterm::event::{self, Event, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use log::LevelFilter;
use ratatui::prelude::*;
use simplelog::{Config as LogConfig, WriteLogger};

use app::App;
use record::{RecordFilter, Recorder};

#[derive(Parser)]
#[command(name = "mavshark", version, about = "MAVLink message inspector")]
struct Cli {
    /// Connection URI (e.g. udpin:0.0.0.0:14550, tcpout:127.0.0.1:5760, serial:/dev/ttyUSB0:57600)
    #[arg(default_value = "udpin:0.0.0.0:14550")]
    uri: String,

    /// Send heartbeats with this system ID
    #[arg(long)]
    heartbeat_sys_id: Option<u8>,

    /// Heartbeat component ID (used with --heartbeat-sys-id)
    #[arg(long, default_value_t = 1)]
    heartbeat_comp_id: u8,

    /// Log file path
    #[arg(long, default_value = "mavshark.log")]
    log_file: String,

    /// Record messages to a JSON Lines file
    #[arg(long)]
    record: Option<String>,

    /// Comma-separated message names or numeric IDs to record (default: all)
    #[arg(long)]
    record_filter: Option<String>,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Open a recorded JSON Lines file in the replay TUI
    Replay {
        /// Path to the JSON Lines recording file
        file: String,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Replay mode — no logging, no connection
    if let Some(Commands::Replay { file }) = cli.command {
        return replay::run_replay(&file);
    }

    // File logging (keeps logs out of the TUI)
    let log_file = File::create(&cli.log_file)?;
    WriteLogger::init(LevelFilter::Info, LogConfig::default(), log_file)?;
    log::info!("mavshark starting, connecting to {}", cli.uri);

    // Connect MAVLink
    let conn = mavlink::connect::<mavlink::ardupilotmega::MavMessage>(&cli.uri)
        .map_err(|e| anyhow::anyhow!("Failed to connect to {}: {}", cli.uri, e))?;
    let conn = Arc::new(conn);

    let stop = Arc::new(AtomicBool::new(false));
    let (tx, rx) = std::sync::mpsc::channel();

    // Listener thread
    let listener_conn = conn.clone();
    let listener_stop = stop.clone();
    std::thread::spawn(move || {
        mavlink_io::listener_loop(listener_conn, tx, listener_stop);
    });

    // Heartbeat thread (optional)
    let heartbeat_handle = cli.heartbeat_sys_id.map(|sys_id| {
        let hb_conn = conn.clone();
        let hb_stop = stop.clone();
        let comp_id = cli.heartbeat_comp_id;
        std::thread::spawn(move || {
            mavlink_io::heartbeat_loop(hb_conn, sys_id, comp_id, hb_stop);
        })
    });

    // Set up recorder (optional)
    let mut recorder = match &cli.record {
        Some(path) => {
            let filter = RecordFilter::new(cli.record_filter.as_deref());
            let r = Recorder::new(path, filter)?;
            log::info!("Recording to {}", path);
            Some(r)
        }
        None => None,
    };

    // Restore terminal on panic
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
        original_hook(info);
    }));

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Run app
    let heartbeat_info = cli.heartbeat_sys_id.map(|s| (s, cli.heartbeat_comp_id));
    let mut app = App::new(cli.uri.clone(), heartbeat_info);
    let result = run_app(&mut terminal, &mut app, rx, &mut recorder);

    // Cleanup
    stop.store(true, Ordering::Relaxed);
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    if let Some(h) = heartbeat_handle {
        let _ = h.join();
    }

    result
}

fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
    rx: std::sync::mpsc::Receiver<mavlink_io::ReceivedMessage>,
    recorder: &mut Option<Recorder>,
) -> Result<()> {
    loop {
        while let Ok(msg) = rx.try_recv() {
            if let Some(rec) = recorder.as_mut() {
                rec.record(&msg);
            }
            app.on_message(msg);
        }

        if let Some(rec) = recorder.as_mut() {
            rec.flush();
        }

        app.tick();
        terminal.draw(|f| ui::draw(f, app))?;

        if event::poll(std::time::Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press && app.on_key(key) {
                    return Ok(());
                }
            }
        }
    }
}
