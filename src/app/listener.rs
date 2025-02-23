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

use crate::app::LogLevel;

pub struct Listener {
    connection: Arc<Mutex<Box<dyn MavConnection<MavMessage> + Send + Sync>>>,
    output_file: Option<String>,
    message_tx: Sender<(MavHeader, MavMessage)>,
    logs_tx: Sender<(Instant, LogLevel, String)>,
    heartbeat_id: Option<u8>,
    system_id_filter: Option<u8>,
    component_id_filter: Option<u8>,
}

impl Listener {
    pub fn new(
        connection: Arc<Mutex<Box<dyn MavConnection<MavMessage> + Send + Sync>>>,
        output_file: Option<String>,
        message_tx: Sender<(MavHeader, MavMessage)>,
        logs_tx: Sender<(Instant, LogLevel, String)>,
        heartbeat_id: Option<u8>,
        system_id_filter: Option<u8>,
        component_id_filter: Option<u8>,
    ) -> Self {
        Listener {
            connection,
            output_file,
            message_tx,
            logs_tx,
            heartbeat_id,
            system_id_filter,
            component_id_filter,
        }
    }

    pub fn listen(&self) {
        self.log_info("Starting listener");

        let output_writer = self.get_output_file_writer();

        self.start_heartbeat_loop();

        if let Some(filter) = self.system_id_filter {
            self.log_info(&format!("Filtering messages for system ID: {}", filter));
        }

        let start_time = Instant::now();
        let mut last_timestamp = start_time;

        loop {
            let conn = self.connection.lock().unwrap();
            match conn.recv() {
                Ok((header, message)) => {
                    if self.should_filter_message(header.system_id, header.component_id) {
                        continue;
                    }

                    let current_timestamp = Instant::now();
                    let time_diff = current_timestamp.duration_since(last_timestamp);
                    last_timestamp = current_timestamp;

                    self.write_message_to_file(
                        &header,
                        &message,
                        time_diff,
                        output_writer.as_ref(),
                    );
                    self.send_message(header, message);
                }
                Err(e) => {
                    self.log_error(&format!("Failed to receive message: {}", e));
                }
            }
        }
    }

    fn get_output_file_writer(&self) -> Option<File> {
        self.output_file
            .as_ref()
            .map(|filename| match File::create(filename) {
                Ok(file) => {
                    self.log_info(&format!("Successfully created output file: {}", filename));
                    file
                }
                Err(e) => {
                    self.log_error(&format!("Failed to create output file: {}", e));
                    panic!("Failed to create output file");
                }
            })
    }

    fn send_message(&self, header: MavHeader, message: MavMessage) {
        self.message_tx
            .send((header, message))
            .expect("Failed to send message to monitor");
    }

    fn log_info(&self, message: &str) {
        self.logs_tx
            .send((Instant::now(), LogLevel::Info, message.to_string()))
            .expect("Failed to send info message to monitor");
    }

    fn log_error(&self, message: &str) {
        self.logs_tx
            .send((Instant::now(), LogLevel::Error, message.to_string()))
            .expect("Failed to send info message to monitor");
    }

    fn write_message_to_file(
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

    fn should_filter_message(&self, system_id: u8, component_id: u8) -> bool {
        if let Some(sys_id) = self.system_id_filter {
            if sys_id != system_id {
                return true;
            }
        }

        if let Some(comp_id) = self.component_id_filter {
            if comp_id != component_id {
                return true;
            }
        }

        false
    }
    /// Only starts if heartbeat_id is Some
    fn start_heartbeat_loop(&self) {
        if let Some(heartbeat_id) = self.heartbeat_id {
            let connection_clone = self.connection.clone();
            let log_sender = self.logs_tx.clone();
            self.log_info(&format!(
                "Starting heartbeat loop for system ID: {}",
                heartbeat_id
            ));
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

                let conn = connection_clone.lock().unwrap();
                if let Err(e) = conn.send(&header, &heartbeat) {
                    log_sender
                        .send((
                            Instant::now(),
                            LogLevel::Error,
                            format!("Failed to send heartbeat: {}", e),
                        ))
                        .expect("Failed to send error message to monitor");
                }
                drop(conn);

                thread::sleep(heartbeat_interval);
            });
        }
    }
}
