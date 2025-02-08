use mavlink::common::MavMessage;
use serde_json::json;
use std::time::Duration;

pub struct MavlinkListener {
    address: String,
    duration: Duration,
    system_id: Option<u8>,
    component_id: Option<u8>,
}

impl MavlinkListener {
    pub fn new(
        address: String,
        duration: Duration,
        system_id: Option<u8>,
        component_id: Option<u8>,
    ) -> Self {
        MavlinkListener {
            address,
            duration,
            system_id,
            component_id,
        }
    }

    pub fn listen(&self) {
        let connection = mavlink::connect::<MavMessage>(&self.address).expect(&format!(
            "Couldn't open MAVLink UDP connection at {}",
            self.address
        ));

        let start_time = std::time::Instant::now();

        loop {
            if start_time.elapsed() > self.duration {
                break;
            }

            match connection.recv() {
                Ok((header, message)) => {
                    if let Some(system_id) = self.system_id {
                        if header.system_id != system_id {
                            continue;
                        }
                    }
                    if let Some(comp_id) = self.component_id {
                        if header.component_id != comp_id {
                            continue;
                        }
                    }

                    // nicely formatted JSON output
                    let formatted_message = json!({
                        "source": {
                            "system_id": header.system_id,
                            "component_id": header.component_id
                        },
                        "message_type": format!("{:?}", message),
                        "message_data": message
                    });

                    println!(
                        "{}",
                        serde_json::to_string_pretty(&formatted_message).unwrap()
                    );
                }
                Err(e) => {
                    eprintln!("Error receiving MAVLink message: {}", e);
                    break;
                }
            }
        }
    }
}
