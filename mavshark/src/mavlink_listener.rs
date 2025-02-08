use serde_json::json;
use std::net::UdpSocket;
use std::str;
use std::time::Duration;

pub struct MavlinkListener {
    address: String,
    duration: Duration,
    target_system_id: Option<u8>,
    component_id: Option<u8>,
}

impl MavlinkListener {
    pub fn new(
        address: String,
        duration: Duration,
        target_system_id: Option<u8>,
        component_id: Option<u8>,
    ) -> Self {
        MavlinkListener {
            address,
            duration,
            target_system_id,
            component_id,
        }
    }

    pub fn listen(&self) {
        let socket = UdpSocket::bind(&self.address).expect("Couldn't bind to address");
        socket
            .set_read_timeout(Some(self.duration))
            .expect("Couldn't set read timeout");

        let mut buf = [0; 1024];
        loop {
            match socket.recv_from(&mut buf) {
                Ok((amt, src)) => {
                    let msg = str::from_utf8(&buf[..amt]).expect("Couldn't parse message");
                    let json_msg = json!({
                        "source": src.to_string(),
                        "message": msg,
                        "target_system_id": self.target_system_id,
                        "component_id": self.component_id,
                    });
                    println!("{}", json_msg.to_string());
                }
                Err(e) => {
                    eprintln!("Error receiving message: {}", e);
                    break;
                }
            }
        }
    }
}
