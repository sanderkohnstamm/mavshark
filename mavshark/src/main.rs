mod mavlink_listener;

use clap::{Arg, Command};
use mavlink_listener::MavlinkListener;
use std::time::Duration;

fn main() {
    let matches = Command::new("mavshark")
        .version("1.0")
        .author("Your Name <your.email@example.com>")
        .about("Mavlink Listener CLI")
        .subcommand(
            Command::new("listen")
                .about("Listens to Mavlink messages")
                .arg(
                    Arg::new("ADDRESS")
                        .help("The connection address")
                        .required(true)
                        .index(1),
                )
                .arg(
                    Arg::new("time")
                        .short('t')
                        .long("time")
                        .value_name("TIME")
                        .help("Amount of time to listen")
                        .value_parser(clap::value_parser!(u64)),
                )
                .arg(
                    Arg::new("target-id")
                        .long("target-id")
                        .value_name("TARGET_ID")
                        .help("Target system ID filter")
                        .value_parser(clap::value_parser!(u8)),
                )
                .arg(
                    Arg::new("component-id")
                        .long("component-id")
                        .value_name("COMPONENT_ID")
                        .help("Component ID filter")
                        .value_parser(clap::value_parser!(u8)),
                ),
        )
        .get_matches();

    if let Some(matches) = matches.subcommand_matches("listen") {
        let address = matches.get_one::<String>("ADDRESS").unwrap().to_string();
        let time = *matches.get_one::<u64>("time").unwrap_or(&60);
        let target_id = matches.get_one::<u8>("target-id").copied();
        let component_id = matches.get_one::<u8>("component-id").copied();

        let listener =
            MavlinkListener::new(address, Duration::new(time, 0), target_id, component_id);
        listener.listen();
    }
}
