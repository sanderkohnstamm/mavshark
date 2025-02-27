#[derive(PartialEq)]
pub enum InputField {
    Address,
    File,
    HeartbeatId,
    SystemId,
    ComponentId,
}

pub const RECORDER_CHEATSHEET: &str = "q: Quit\n\
Enter: Start Listener\n\
Tab: Switch Input\n\
Up/Down: Navigate Messages\n\
Esc: Stop Listener\n\
Allowed connection address formats:udpin, udpout, tcpin, tcpout\n\
Allowed output file formats: *.txt\n\
Heartbeat ID: loop heartbeat with id (0-255)\n\
Sys ID/Comp ID: filter messages by id (0-255)\n\
";

pub const SENDER_CHEATSHEET: &str = "q: Quit\n\
Enter: Start connection or send message\n\
Tab: Switch Input\n\
Up/Down/Right/Left: Navigate Messages\n\
Esc: Stop Listener\n\
Allowed connection address formats:udpin, udpout, tcpin, tcpout\n\
Allowed input file formats: *.txt\n\
Heartbeat ID: loop heartbeat with id (0-255)\n\
Sys/Comp ID: overrides for message sending (0-255)\n\
";

pub fn validate_u8_input(input: &str) -> bool {
    input.parse::<u8>().is_ok()
}

pub fn validate_file_input(input: &str) -> bool {
    input.ends_with(".txt")
        && input
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == '.' || c == '/')
}

pub fn validate_connection_address_input(input: &str) -> bool {
    let parts: Vec<&str> = input.split(':').collect();
    if parts.len() != 3 {
        return false;
    }
    let protocol = parts[0];
    let ip = parts[1];
    let port = parts[2];

    if protocol != "udpin" && protocol != "udpout" && protocol != "tcpin" && protocol != "tcpout" {
        return false;
    }

    if !ip.parse::<std::net::Ipv4Addr>().is_ok() {
        return false;
    }

    if !port.parse::<u16>().is_ok() {
        return false;
    }

    true
}
