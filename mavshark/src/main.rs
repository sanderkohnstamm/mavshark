mod mavlink_listener;
mod mavlink_sender;

use clap::{Arg, Command};
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

fn main() {
    let matches = Command::new("mavshark")
        .version("1.0")
        .author("Sander Kohnstamm sanderkohnstamm@gmail.com")
        .about("MAVLink recorder and replayer CLI")
        .subcommand(
            Command::new("record")
                .about("Records MAVLink messages")
                .arg(
                    Arg::new("ADDRESS")
                        .help("The MAVLink connection address")
                        .required(true)
                        .index(1),
                )
                .arg(
                    Arg::new("time")
                        .short('t')
                        .long("time")
                        .value_name("TIME")
                        .help("Optional: Amount of time to listen [s]")
                        .value_parser(clap::value_parser!(u64)),
                )
                .arg(
                    Arg::new("output-file")
                        .short('o')
                        .value_name("OUTPUT_FILE")
                        .help("Output file for message logging. This allows for the replay functionality")
                        .long("output-file")
                        .value_parser(clap::value_parser!(String)),
                )
                .arg(
                    Arg::new("output-file-binary")
                        .value_name("OUTPUT_FILE_BINARY")
                        .help("Output file for binary message logging. Allows for the connection to be made on the file later.")
                        .long("output-file-binary")
                        .value_parser(clap::value_parser!(String)),
                )
                .arg(
                    Arg::new("heartbeat-id")
                        .short('i')
                        .long("heartbeat-id")
                        .value_name("HEARTBEAT_ID")
                        .help("Optional: System ID from which to send a heartbeat. If omitted, no heartbeat is sent")
                        .value_parser(clap::value_parser!(u8)), 
                )
                .arg(
                    Arg::new("include-system-id")
                        .long("include-system-id")
                        .value_name("INCLUDE_SYSTEM_ID")
                        .help("Only include messages from specified system IDs")
                        .num_args(1..)
                        .value_parser(clap::value_parser!(u8)),
                )
                .arg(
                    Arg::new("exclude-system-id")
                        .long("exclude-system-id")
                        .value_name("EXCLUDE_SYSTEM_ID")
                        .help("Exclude messages from specified system IDs")
                        .num_args(1..)
                        .value_parser(clap::value_parser!(u8)),
                )
                .arg(
                    Arg::new("include-component-id")
                        .long("include-component-id")
                        .value_name("INCLUDE_COMPONENT_ID")
                        .help("Only include messages from specified component IDs")
                        .num_args(1..)
                        .value_parser(clap::value_parser!(u8)),
                )
                .arg(
                    Arg::new("exclude-component-id")
                        .long("exclude-component-id")
                        .value_name("EXCLUDE_COMPONENT_ID")
                        .help("Exclude messages from specified component IDs")
                        .num_args(1..)
                        .value_parser(clap::value_parser!(u8)),
                )
        )
        .subcommand(
            Command::new("replay")
                .about("Replays MAVLink messages from a file")
                .arg(
                    Arg::new("ADDRESS")
                        .help("The MAVLink connection address")
                        .required(true)
                        .index(1),
                )
                .arg(
                    Arg::new("INPUT_FILE")
                        .help("Path to the input file containing MAVLink messages")
                        .required(true)
                        .index(2),
                ),
        )
        .get_matches();

    if let Some(matches) = matches.subcommand_matches("record") {
        let address = matches.get_one::<String>("ADDRESS").unwrap().to_string();
        let time = matches.get_one::<u64>("time").copied();
        let heartbeat_id = matches.get_one::<u8>("heartbeat-id").copied();
        let include_system_ids = matches
            .get_many::<u8>("include-system-id")
            .map(|values| values.cloned().collect())
            .unwrap_or_else(Vec::new);
        let exclude_system_ids = matches
            .get_many::<u8>("exclude-system-id")
            .map(|values| values.cloned().collect())
            .unwrap_or_else(Vec::new);
        let include_component_ids = matches
            .get_many::<u8>("include-component-id")
            .map(|values| values.cloned().collect())
            .unwrap_or_else(Vec::new);
        let exclude_component_ids = matches
            .get_many::<u8>("exclude-component-id")
            .map(|values| values.cloned().collect())
            .unwrap_or_else(Vec::new);
        let output_file = matches.get_one::<String>("output-file").cloned();
        let output_file_binary = matches.get_one::<String>("output-file-binary").cloned();

        let duration = time.map(Duration::from_secs);

        let connection = mavlink::connect::<MavMessage>(&address)
            .expect(&format!("Couldn't open MAVLink connection at {}", address));
        let connection = Arc::new(Mutex::new(connection));

        if let Some(heartbeat_id) = heartbeat_id {
            start_heartbeat_loop(connection.clone(), heartbeat_id);
        }

        let listener = MavlinkListener::new(
            duration,
            include_system_ids,
            exclude_system_ids,
            include_component_ids,
            exclude_component_ids,
            output_file,
            output_file_binary,
        );
        listener.record(connection);
    }

    if let Some(matches) = matches.subcommand_matches("replay") {
        let address = matches.get_one::<String>("ADDRESS").unwrap().to_string();
        let input_file = matches.get_one::<String>("INPUT_FILE").unwrap();
        let connection = mavlink::connect::<MavMessage>(&address)
            .expect(&format!("Couldn't open MAVLink connection at {}", address));
        let connection = Arc::new(Mutex::new(connection));

        MavlinkSender::replay(connection, input_file);
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
