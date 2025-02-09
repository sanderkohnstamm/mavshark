use mavlink::{common::MavMessage, MavConnection};
use serde_json::json;
use std::io::Write;
use std::{
    fs::File,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use crate::mavlink_monitor::MavlinkMonitor;

pub struct MavlinkListener {
    duration: Option<Duration>,
    include_system_ids: Vec<u8>,
    exclude_system_ids: Vec<u8>,
    include_component_ids: Vec<u8>,
    exclude_component_ids: Vec<u8>,
    output_file: Option<String>,
    output_file_binary: Option<String>,
    enable_print: bool,
    monitor: Arc<MavlinkMonitor>,
}

impl MavlinkListener {
    pub fn new(
        duration: Option<Duration>,
        include_system_ids: Vec<u8>,
        exclude_system_ids: Vec<u8>,
        include_component_ids: Vec<u8>,
        exclude_component_ids: Vec<u8>,
        output_file: Option<String>,
        output_file_binary: Option<String>,
        enable_print: bool,
    ) -> Self {
        MavlinkListener {
            duration,
            include_system_ids,
            exclude_system_ids,
            include_component_ids,
            exclude_component_ids,
            output_file,
            output_file_binary,
            enable_print,
            monitor: Arc::new(MavlinkMonitor::new()),
        }
    }

    pub fn record(&self, connection: Arc<Mutex<Box<dyn MavConnection<MavMessage> + Send + Sync>>>) {
        let start_time = Instant::now();
        let mut last_timestamp = start_time;

        let mut output_writer = self
            .output_file
            .as_ref()
            .map(|filename| File::create(filename).expect("Failed to create output file"));

        let mut binary_output_writer = self
            .output_file_binary
            .as_ref()
            .map(|filename| File::create(filename).expect("Failed to create binary output file"));

        if !self.enable_print {
            self.monitor.start();
        }

        loop {
            if let Some(duration) = self.duration {
                if start_time.elapsed() > duration {
                    break;
                }
            }

            let conn = connection.lock().unwrap();
            match conn.recv() {
                Ok((header, message)) => {
                    if self.should_filter_message(header.system_id, header.component_id) {
                        continue;
                    }
                    let current_timestamp = Instant::now();
                    let time_diff = current_timestamp.duration_since(last_timestamp);
                    last_timestamp = current_timestamp;

                    let message_json =
                        serde_json::to_string(&message).unwrap_or_else(|_| "{}".to_string());
                    let message_type = serde_json::from_str::<serde_json::Value>(&message_json)
                        .ok()
                        .and_then(|msg| msg.get("type").and_then(|t| t.as_str()).map(String::from))
                        .unwrap_or_else(|| "UNKNOWN".to_string());

                    let time_message = json!({ "time_s": time_diff.as_secs_f64() }).to_string();
                    let log_message = json!({
                        "system_id": header.system_id,
                        "component_id": header.component_id,
                        "message": message_json,
                    })
                    .to_string();

                    if self.enable_print {
                        println!("{}", time_message);
                        println!("{}", log_message);
                    } else {
                        self.monitor.update(
                            header.system_id,
                            header.component_id,
                            message_type,
                            current_timestamp,
                        );
                    }

                    self.write_logs(
                        &mut output_writer,
                        &mut binary_output_writer,
                        &time_message,
                        &log_message,
                        header,
                        &message,
                    );
                }
                Err(e) => {
                    eprintln!("Error receiving MAVLink message: {}", e);
                    break;
                }
            }
        }
    }

    fn write_logs(
        &self,
        output_writer: &mut Option<File>,
        binary_output_writer: &mut Option<File>,
        time_message: &str,
        log_message: &str,
        header: mavlink::MavHeader,
        message: &MavMessage,
    ) {
        if let Some(writer) = output_writer.as_mut() {
            writeln!(writer, "{}\n{}", time_message, log_message)
                .expect("Failed to write to output file");
            writer.flush().expect("Failed to flush output file");
        }

        if let Some(writer) = binary_output_writer.as_mut() {
            let mut buffer = vec![];
            mavlink::write_versioned_msg(&mut buffer, mavlink::MavlinkVersion::V2, header, message)
                .expect("Failed to encode MAVLink message");

            writer
                .write_all(&buffer)
                .expect("Failed to write MAVLink binary");
            writer.flush().expect("Failed to flush binary output file");
        }
    }

    fn should_filter_message(&self, system_id: u8, component_id: u8) -> bool {
        if self.exclude_system_ids.contains(&system_id) {
            return true;
        }
        if !self.include_system_ids.is_empty() && !self.include_system_ids.contains(&system_id) {
            return true;
        }
        if self.exclude_component_ids.contains(&component_id) {
            return true;
        }
        if !self.include_component_ids.is_empty()
            && !self.include_component_ids.contains(&component_id)
        {
            return true;
        }
        false
    }
}
