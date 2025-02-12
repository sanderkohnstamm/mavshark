mod mavlink_listener;
mod mavlink_monitor;
mod mavlink_sender;

use clap::{Parser, Subcommand};
use mavlink::{
    common::{MavAutopilot, MavMessage, MavModeFlag, MavState, MavType},
    MavConnection, MavHeader,
};
use mavlink_listener::MavlinkListener;
use mavlink_sender::MavlinkSender;
use std::{
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};

const README: &str = include_str!("../README.md");

#[derive(Parser)]
#[command(name = "mavshark")]
#[command(version = "0.1.2")]
#[command(author = "Sander Kohnstamm <sanderkohnstamm@gmail.com>")]
#[command(about = "MAVLink recorder and replayer CLI")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Record MAVLink messages from a connection
    Record {
        #[arg(help = "The MAVLink connection address")]
        address: String,
        #[arg(
            short,
            long,
            value_name = "TIME",
            help = "Optional: Amount of time to listen [s]"
        )]
        time: Option<u64>,
        #[arg(
            short,
            long,
            value_name = "OUTPUT_FILE",
            help = "Output file for message logging. This allows for the replay functionality"
        )]
        output_file: Option<String>,
        #[arg(
            long,
            value_name = "OUTPUT_FILE_BINARY",
            help = "Output file for binary message logging. Allows for the connection to be made on the file later."
        )]
        output_file_binary: Option<String>,
        #[arg(
            short = 'i',
            long,
            value_name = "HEARTBEAT_ID",
            help = "Optional: System ID from which to send a heartbeat. If omitted, no heartbeat is sent"
        )]
        heartbeat_id: Option<u8>,
        #[arg(long, value_name = "INCLUDE_SYSTEM_ID", help = "Only include messages from specified system IDs", num_args = 1..)]
        include_system_id: Vec<u8>,
        #[arg(long, value_name = "EXCLUDE_SYSTEM_ID", help = "Exclude messages from specified system IDs", num_args = 1..)]
        exclude_system_id: Vec<u8>,
        #[arg(long, value_name = "INCLUDE_COMPONENT_ID", help = "Only include messages from specified component IDs", num_args = 1..)]
        include_component_id: Vec<u8>,
        #[arg(long, value_name = "EXCLUDE_COMPONENT_ID", help = "Exclude messages from specified component IDs", num_args = 1..)]
        exclude_component_id: Vec<u8>,
        #[arg(short, long, help = "Print messages instead of showing monitor")]
        print: bool,
    },
    /// Replay MAVLink messages from a file towards a connection
    Replay {
        #[arg(help = "The MAVLink connection address")]
        address: String,
        #[arg(help = "Path to the input file containing MAVLink messages")]
        input_file: String,
    },
    /// Print the README
    Explain,
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Record {
            address,
            time,
            output_file,
            output_file_binary,
            heartbeat_id,
            include_system_id,
            exclude_system_id,
            include_component_id,
            exclude_component_id,
            print,
        } => {
            let duration = time.map(Duration::from_secs);

            let connection = mavlink::connect::<MavMessage>(&address)
                .expect(&format!("Couldn't open MAVLink connection at {}", address));
            let connection = Arc::new(Mutex::new(connection));

            if let Some(heartbeat_id) = heartbeat_id {
                start_heartbeat_loop(connection.clone(), heartbeat_id);
            }

            let listener = MavlinkListener::new(
                duration,
                include_system_id,
                exclude_system_id,
                include_component_id,
                exclude_component_id,
                output_file,
                output_file_binary,
                print,
            );
            listener.record(connection);
        }
        Commands::Replay {
            address,
            input_file,
        } => {
            let connection = mavlink::connect::<MavMessage>(&address)
                .expect(&format!("Couldn't open MAVLink connection at {}", address));
            let connection = Arc::new(Mutex::new(connection));

            MavlinkSender::replay(connection, &input_file);
        }
        Commands::Explain => {
            termimad::print_text(README);
        }
    }
}

fn start_heartbeat_loop(
    connection: Arc<Mutex<Box<dyn MavConnection<MavMessage> + Send + Sync>>>,
    heartbeat_id: u8,
) {
    let heartbeat_interval = Duration::from_millis(500);
    thread::spawn(move || loop {
        let heartbeat = MavMessage::HEARTBEAT(mavlink::common::HEARTBEAT_DATA {
            custom_mode: 0,
            mavtype: MavType::MAV_TYPE_GENERIC,
            autopilot: MavAutopilot::MAV_AUTOPILOT_INVALID,
            base_mode: MavModeFlag::empty(),
            system_status: MavState::MAV_STATE_ACTIVE,
            mavlink_version: 3,
        });

        let header = MavHeader {
            system_id: heartbeat_id,
            component_id: 1,
            sequence: 0,
        };

        let conn = connection.lock().unwrap();
        if let Err(e) = conn.send(&header, &heartbeat) {
            eprintln!("Failed to send heartbeat: {}", e);
        }
        drop(conn);

        thread::sleep(heartbeat_interval);
    });
}
