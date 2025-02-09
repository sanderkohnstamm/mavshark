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
    duration: Option<Duration>,
    heartbeat_id: Option<u8>, // Optional heartbeat system ID
    include_system_ids: Vec<u8>,
    exclude_system_ids: Vec<u8>,
    include_component_ids: Vec<u8>,
    exclude_component_ids: Vec<u8>,
}

impl MavlinkListener {
    pub fn new(
        address: String,
        duration: Option<Duration>,
        heartbeat_id: Option<u8>, // Optional heartbeat system ID
        include_system_ids: Vec<u8>,
        exclude_system_ids: Vec<u8>,
        include_component_ids: Vec<u8>,
        exclude_component_ids: Vec<u8>,
    ) -> Self {
        MavlinkListener {
            address,
            duration,
            heartbeat_id,
            include_system_ids,
            exclude_system_ids,
            include_component_ids,
            exclude_component_ids,
        }
    }

    fn start_heartbeat_loop(
        connection: Arc<Mutex<Box<dyn MavConnection<MavMessage> + Send + Sync>>>,
        heartbeat_id: u8,
    ) {
        let heartbeat_interval = Duration::from_secs(1);
        thread::spawn(move || {
            let mut has_printed = false;
            let mut has_printed_err = false;
            loop {
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

                // Lock the connection, send the heartbeat, then release the lock
                {
                    let conn = connection.lock().unwrap();
                    if let Err(e) = conn.send(&header, &heartbeat) {
                        if !has_printed_err {
                            eprintln!("⚠️ Failed to send heartbeat: {}", e);
                            has_printed = false;
                            has_printed_err = true;
                        }
                    } else {
                        if !has_printed {
                            println!("Sent heartbeat as System ID 240. Repeating ...");
                            has_printed = true;
                            has_printed_err = false;
                        }
                    }
                } // Mutex is released here!

                thread::sleep(heartbeat_interval);
            }
        });
    }

    pub fn listen(&self) {
        println!("Starting MAVLink listener with configuration:");
        println!(
            "Address: {}\nDuration: {:?}\nHeartbeat ID: {:?}\nInclude System IDs: {:?}\n\
            Exclude System IDs: {:?}\nInclude Component IDs: {:?}\nExclude Component IDs: {:?}",
            self.address,
            self.duration,
            self.heartbeat_id,
            self.include_system_ids,
            self.exclude_system_ids,
            self.include_component_ids,
            self.exclude_component_ids
        );

        println!("Waiting for MAVLink connection ...");

        let connection = mavlink::connect::<MavMessage>(&self.address).expect(&format!(
            "Couldn't open MAVLink connection at {}",
            self.address
        ));

        let connection = Arc::new(Mutex::new(connection));

        if let Some(heartbeat_id) = self.heartbeat_id {
            Self::start_heartbeat_loop(connection.clone(), heartbeat_id);
        }

        let start_time = Instant::now();

        loop {
            if let Some(duration) = self.duration {
                if start_time.elapsed() > duration {
                    break;
                }
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
