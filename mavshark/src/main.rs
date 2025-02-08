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
                .long_about(
                    "Listens for MAVLink messages on a specified connection.\n\n\
                    Supported connection types:\n  \
                      - tcpin:<addr>:<port>  (TCP server, listening for connections)\n  \
                      - tcpout:<addr>:<port> (TCP client)\n  \
                      - udpin:<addr>:<port>  (UDP server, listening for packets)\n  \
                      - udpout:<addr>:<port> (UDP client)\n  \
                      - udpbcast:<addr>:<port> (UDP broadcast)\n  \
                      - serial:<port>:<baudrate> (Serial connection)\n  \
                      - file:<path> (Extract MAVLink data from a file)\n\n\
                    Example usage:\n  \
                      mavshark listen udpin:0.0.0.0:14550 --time 120 --system-id 1",
                )
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
                        .help("Amount of time to listen (seconds)")
                        .value_parser(clap::value_parser!(u64)),
                )
                .arg(
                    Arg::new("system-id")
                        .long("system-id")
                        .value_name("SYSTEM_ID")
                        .help("Filter by sender system ID")
                        .value_parser(clap::value_parser!(u8)),
                )
                .arg(
                    Arg::new("component-id")
                        .long("component-id")
                        .value_name("COMPONENT_ID")
                        .help("Filter by component ID")
                        .value_parser(clap::value_parser!(u8)),
                ),
        )
        .get_matches();

    if let Some(matches) = matches.subcommand_matches("listen") {
        let address = matches.get_one::<String>("ADDRESS").unwrap().to_string();
        let time = *matches.get_one::<u64>("time").unwrap_or(&60);
        let system_id = matches.get_one::<u8>("system-id").copied();
        let component_id = matches.get_one::<u8>("component-id").copied();

        let listener =
            MavlinkListener::new(address, Duration::new(time, 0), system_id, component_id);
        listener.listen();
    }
}
