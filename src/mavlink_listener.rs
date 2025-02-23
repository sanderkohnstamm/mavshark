use mavlink::{
    common::{MavAutopilot, MavMessage, MavModeFlag, MavState, MavType},
    MavConnection, MavHeader,
};
use serde_json::json;
use std::io::Write;
use std::sync::mpsc::Sender;
use std::thread;
use std::{
    fs::File,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

pub struct MavlinkListener {}

impl MavlinkListener {
    pub fn new() -> Self {
        MavlinkListener {}
    }

    pub fn listen(
        &self,
        connection: Arc<Mutex<Box<dyn MavConnection<MavMessage> + Send + Sync>>>,
        output_file: Option<String>,
        message_sender: Sender<(MavHeader, MavMessage)>,
        error_sender: Sender<(Instant, String)>,
        heartbeat_id: Option<u8>,
        filter_system_id: Option<u8>,
    ) {
        let output_writer = match output_file.as_ref().map(|filename| File::create(filename)) {
            Some(Ok(writer)) => Some(writer),
            Some(Err(e)) => {
                error_sender
                    .send((Instant::now(), format!("Failed to create output file: {e}")))
                    .unwrap();
                None
            }
            None => None,
        };

        if let Some(heartbeat_id) = heartbeat_id {
            let conn = connection.clone();
            start_heartbeat_loop(conn, heartbeat_id);
        }

        let start_time = Instant::now();
        let mut last_timestamp = start_time;

        loop {
            let conn = connection.lock().unwrap();
            match conn.recv() {
                Ok((header, message)) => {
                    if self.should_filter_message(header.system_id, filter_system_id) {
                        continue;
                    }

                    let current_timestamp = Instant::now();
                    let time_diff = current_timestamp.duration_since(last_timestamp);
                    last_timestamp = current_timestamp;

                    self.write_logs(&header, &message, time_diff, output_writer.as_ref());
                    message_sender
                        .send((header, message))
                        .expect("Failed to send message to monitor");
                }
                Err(e) => {
                    error_sender
                        .send((Instant::now(), e.to_string()))
                        .expect("Failed to send error to monitor");
                }
            }
        }
    }

    fn write_logs(
        &self,
        header: &MavHeader,
        message: &MavMessage,
        time_diff: Duration,
        output_writer: Option<&File>,
    ) {
        let message_json = serde_json::to_string(&message).unwrap_or_else(|_| "{}".to_string());

        let time_message = json!({ "time_s": time_diff.as_secs_f64() }).to_string();
        let message_content = json!({
            "system_id": header.system_id,
            "component_id": header.component_id,
            "message": message_json,
        })
        .to_string();

        if let Some(mut writer) = output_writer {
            writeln!(writer, "{}\n{}", time_message, message_content)
                .expect("Failed to write to output file");
            writer.flush().expect("Failed to flush output file");
        }
    }

    fn should_filter_message(
        &self,
        system_id: u8,
        // component_id: u8,
        include_system_ids: Option<u8>,
    ) -> bool {
        // if self.exclude_system_ids.contains(&system_id) {
        //     return true;
        // }
        if let Some(sys_id) = include_system_ids {
            if sys_id != system_id {
                return true;
            }
        }
        // if self.exclude_component_ids.contains(&component_id) {
        //     return true;
        // }
        // if !self.include_component_ids.is_empty()
        //     && !self.include_component_ids.contains(&component_id)
        // {
        //     return true;
        // }
        false
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
