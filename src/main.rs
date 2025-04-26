mod app;
mod recorder_app;
mod sender_app;

use clap::{Parser, Subcommand};
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use recorder_app::RecorderApp;
use sender_app::SenderApp;
use std::{
    io,
    sync::{Arc, Mutex},
};

#[derive(Parser)]
#[command(name = "mavshark")]
#[command(about = "A MAVLink monitoring and sending tool", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    #[command(about = "Run the recorder app", alias = "r")]
    Recorder {
        #[arg(
            short,
            long,
            value_name = "ADDRESS",
            help = "Prefilled mavlink connection address"
        )]
        address: Option<String>,
        #[arg(
            short,
            long,
            value_name = "OUTPUT_FILE",
            help = "File for logging messages output"
        )]
        output_file: Option<String>,
        #[arg(
            short = 'b',
            long,
            value_name = "HEARTBEAT_ID",
            help = "System id to send heartbeats with"
        )]
        heartbeat_id: Option<String>,
        #[arg(
            short,
            long,
            value_name = "SYSTEM_ID_FILTER",
            help = "Filter messages on this system id"
        )]
        system_id_filter: Option<String>,
        #[arg(
            short,
            long,
            value_name = "COMPONENT_ID_FILTER",
            help = "Filter messages on this component id"
        )]
        component_id_filter: Option<String>,
    },
    #[command(about = "Run the sender app", alias = "s")]
    Sender {
        #[arg(
            short,
            long,
            value_name = "ADDRESS",
            help = "Prefilled mavlink connection address"
        )]
        address: Option<String>,
        #[arg(
            short,
            long,
            value_name = "INPUT_FILE",
            help = "Parse this file for recorded messages"
        )]
        input_file: Option<String>,
        #[arg(
            short = 'b',
            long,
            value_name = "HEARTBEAT_ID",
            help = "System id to send heartbeats with"
        )]
        heartbeat_id: Option<String>,
        #[arg(
            short,
            long,
            value_name = "SYSTEM_ID_OVERRIDE",
            help = "Send messages with this system id"
        )]
        system_id_override: Option<String>,
        #[arg(
            short,
            long,
            value_name = "COMPONENT_ID_OVERRIDE",
            help = "Send messages with this component id"
        )]
        component_id_override: Option<String>,
    },
}

fn main() {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Recorder {
            address,
            output_file,
            heartbeat_id,
            system_id_filter,
            component_id_filter,
        } => run_app(RecorderApp::new(
            address.clone(),
            output_file.clone(),
            heartbeat_id.clone(),
            system_id_filter.clone(),
            component_id_filter.clone(),
        )),
        Commands::Sender {
            address,
            input_file,
            heartbeat_id,
            system_id_override,
            component_id_override,
        } => run_app(SenderApp::new(
            address.clone(),
            input_file.clone(),
            heartbeat_id.clone(),
            system_id_override.clone(),
            component_id_override.clone(),
        )),
    }
}

fn run_app<T: App>(app: T) {
    let app = Arc::new(Mutex::new(app));
    enable_raw_mode().expect("Failed to enable raw mode");
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)
        .expect("Failed to enter alternate screen and enable mouse capture");
    let backend = CrosstermBackend::new(stdout);
    let mut terminal: Terminal<CrosstermBackend<io::Stdout>> =
        Terminal::new(backend).expect("Failed to create terminal");

    if let Err(e) = app.lock().unwrap().run(&mut terminal) {
        eprintln!("Error: {}", e);
    }
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )
    .expect("Failed to leave alternate screen and disable mouse capture");
    disable_raw_mode().expect("Failed to disable raw mode");
}

trait App {
    fn run(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    ) -> Result<(), io::Error>;
}

impl App for RecorderApp {
    fn run(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    ) -> Result<(), io::Error> {
        RecorderApp::run(self, terminal)
    }
}

impl App for SenderApp {
    fn run(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    ) -> Result<(), io::Error> {
        SenderApp::run(self, terminal)
    }
}
