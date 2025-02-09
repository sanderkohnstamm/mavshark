mod mavlink_listener;

use clap::{Arg, Command};
use mavlink_listener::MavlinkListener;
use std::time::Duration;

fn main() {
    let matches = Command::new("mavshark")
        .version("1.0")
        .author("Sander Kohnstamm sanderkohnstamm@gmail.com")
        .about("MAVLink Listener CLI")
        .subcommand(
            Command::new("listen")
                .about("Listens to MAVLink messages from various connection types")
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
                    Arg::new("heartbeat-id")
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
                ),
        )
        .get_matches();

    if let Some(matches) = matches.subcommand_matches("listen") {
        let address = matches.get_one::<String>("ADDRESS").unwrap().to_string();
        let time = matches.get_one::<u64>("time").copied();

        // Get optional heartbeat system ID
        let heartbeat_id = matches.get_one::<u8>("heartbeat-id").copied();

        // Get include and exclude system IDs
        let include_system_ids = matches
            .get_many::<u8>("include-system-id")
            .map(|values| values.cloned().collect())
            .unwrap_or_else(Vec::new);

        let exclude_system_ids = matches
            .get_many::<u8>("exclude-system-id")
            .map(|values| values.cloned().collect())
            .unwrap_or_else(Vec::new);

        // Get include and exclude component IDs
        let include_component_ids = matches
            .get_many::<u8>("include-component-id")
            .map(|values| values.cloned().collect())
            .unwrap_or_else(Vec::new);

        let exclude_component_ids = matches
            .get_many::<u8>("exclude-component-id")
            .map(|values| values.cloned().collect())
            .unwrap_or_else(Vec::new);

        let duration =  time.map(|t|Duration::new(t, 0));
    

        let listener = MavlinkListener::new(
            address,
            duration,
            heartbeat_id,
            include_system_ids,
            exclude_system_ids,
            include_component_ids,
            exclude_component_ids,
        );

        listener.listen();
    }
}
