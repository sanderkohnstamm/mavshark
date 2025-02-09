use mavlink::{
    common::{MavAutopilot, MavMessage, MavModeFlag, MavState, MavType},
    MavConnection, MavHeader,
};
use std::{
    sync::{Arc, Mutex},
    thread,
    time::{Duration, Instant},
};

pub struct MavlinkListener {
    address: String,
    duration: Duration,
    include_system_ids: Vec<u8>,
    exclude_system_ids: Vec<u8>,
    include_component_ids: Vec<u8>,
    exclude_component_ids: Vec<u8>,
}

impl MavlinkListener {
    pub fn new(
        address: String,
        duration: Duration,
        include_system_ids: Vec<u8>,
        exclude_system_ids: Vec<u8>,
        include_component_ids: Vec<u8>,
        exclude_component_ids: Vec<u8>,
    ) -> Self {
        MavlinkListener {
            address,
            duration,
            include_system_ids,
            exclude_system_ids,
            include_component_ids,
            exclude_component_ids,
        }
    }

    /// Starts a separate thread to send heartbeats every 1 second.
    fn start_heartbeat_loop(
        connection: Arc<Mutex<Box<dyn MavConnection<MavMessage> + Send + Sync>>>,
    ) {
        let heartbeat_interval = Duration::from_secs(1); // 1-second heartbeat interval
        thread::spawn(move || {
            loop {
                let heartbeat = MavMessage::HEARTBEAT(mavlink::common::HEARTBEAT_DATA {
                    custom_mode: 0,
                    mavtype: MavType::MAV_TYPE_GENERIC, // Identifies as an onboard system
                    autopilot: MavAutopilot::MAV_AUTOPILOT_INVALID, // No autopilot
                    base_mode: MavModeFlag::empty(),
                    system_status: MavState::MAV_STATE_ACTIVE, // Active state
                    mavlink_version: 3,
                });

                let header = MavHeader {
                    system_id: 240, // Sniffer SysID
                    component_id: 1,
                    sequence: 0,
                };

                // Lock the connection, send the heartbeat, then release the lock
                {
                    let conn = connection.lock().unwrap();
                    if let Err(e) = conn.send(&header, &heartbeat) {
                        eprintln!("⚠️ Failed to send heartbeat: {}", e);
                    } else {
                        println!("✅ Sent heartbeat as System ID 240");
                    }
                } // Mutex is released here!

                thread::sleep(heartbeat_interval);
            }
        });
    }

    /// Listens for MAVLink messages and prints them
    pub fn listen(&self) {
        let connection = mavlink::connect::<MavMessage>(&self.address).expect(&format!(
            "❌ Couldn't open MAVLink connection at {}",
            self.address
        ));

        let connection = Arc::new(Mutex::new(connection));

        // Start heartbeat loop in a separate thread
        Self::start_heartbeat_loop(connection.clone());

        let start_time = Instant::now();

        loop {
            if start_time.elapsed() > self.duration {
                break;
            }

            let conn = connection.lock().unwrap();
            match conn.recv() {
                Ok((header, message)) => {
                    if self.exclude_system_ids.contains(&header.system_id) {
                        continue;
                    }

                    if !self.include_system_ids.is_empty()
                        && !self.include_system_ids.contains(&header.system_id)
                    {
                        continue;
                    }

                    if self.exclude_component_ids.contains(&header.component_id) {
                        continue;
                    }

                    if !self.include_component_ids.is_empty()
                        && !self.include_component_ids.contains(&header.component_id)
                    {
                        continue;
                    }

                    println!(
                        "----------------------------------------\n\
                            📡 Received MAVLink Message\n\
                            System ID: {}, Component ID: {}\n\
                            {:#?}\n",
                        header.system_id, header.component_id, message
                    );
                }
                Err(e) => {
                    eprintln!("⚠️ Error receiving MAVLink message: {}", e);
                    break;
                }
            }
        }
    }
}
