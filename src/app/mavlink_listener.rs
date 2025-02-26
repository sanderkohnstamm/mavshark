use mavlink::{common::MavMessage, MavConnection, MavHeader};

use serde_json::json;
use std::sync::{atomic::Ordering, mpsc::Sender};
use std::{
    fs::File,
    sync::{Arc, Mutex},
};
use std::{io::Write, sync::atomic::AtomicBool};

use super::Logger;

pub struct MavlinkListener {
    connection: Arc<Mutex<Box<dyn MavConnection<MavMessage> + Send + Sync>>>,
    output_file: Option<String>,
    message_tx: Sender<(MavHeader, MavMessage)>,
    logger: Logger,
    system_id_filter: Option<u8>,
    component_id_filter: Option<u8>,
    stop_signal: Arc<AtomicBool>, // Add stop signal
}

impl MavlinkListener {
    pub fn new(
        connection: Arc<Mutex<Box<dyn MavConnection<MavMessage> + Send + Sync>>>,
        output_file: Option<String>,
        message_tx: Sender<(MavHeader, MavMessage)>,
        logger: Logger,
        system_id_filter: Option<u8>,
        component_id_filter: Option<u8>,
        stop_signal: Arc<AtomicBool>,
    ) -> Self {
        MavlinkListener {
            connection,
            output_file,
            message_tx,
            logger,
            system_id_filter,
            component_id_filter,
            stop_signal,
        }
    }

    pub fn record(&self) {
        self.logger.log_info("Starting recorder");

        let output_writer = self.get_output_file_writer();
        let stop_signal = self.stop_signal.clone();

        if let Some(filter) = self.system_id_filter {
            self.logger
                .log_info(&format!("Filtering messages for system ID: {}", filter));
        }

        loop {
            if stop_signal.load(Ordering::Relaxed) {
                self.logger.log_info("Stopping recorder");
                break;
            }

            let conn = self.connection.lock().unwrap();
            match conn.recv() {
                Ok((header, message)) => {
                    if self.should_filter_message(header.system_id, header.component_id) {
                        continue;
                    }

                    self.write_message_to_file(&header, &message, output_writer.as_ref());
                    self.send_message(header, message);
                }
                Err(e) => {
                    self.logger
                        .log_error(&format!("Failed to receive message: {}", e));
                }
            }
        }
    }

    fn get_output_file_writer(&self) -> Option<File> {
        self.output_file
            .as_ref()
            .map(|filename| match File::create(filename) {
                Ok(file) => {
                    self.logger
                        .log_info(&format!("Successfully created output file: {}", filename));
                    file
                }
                Err(e) => {
                    self.logger
                        .log_error(&format!("Failed to create output file: {}", e));
                    panic!("Failed to create output file");
                }
            })
    }

    fn send_message(&self, header: MavHeader, message: MavMessage) {
        self.message_tx
            .send((header, message))
            .expect("Failed to send message to monitor");
    }

    fn write_message_to_file(
        &self,
        header: &MavHeader,
        message: &MavMessage,
        output_writer: Option<&File>,
    ) {
        if let Some(mut writer) = output_writer {
            let message_json = serde_json::to_string(&message).unwrap_or_else(|_| "{}".to_string());

            let message_content = json!({
                "system_id": header.system_id,
                "component_id": header.component_id,
                "message": message_json,
            })
            .to_string();

            if let Err(e) = writeln!(writer, "{}", message_content) {
                self.logger
                    .log_error(&format!("Failed to write message to output file: {}", e));
            };
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
}
