use mavlink::{common::MavMessage, MavConnection, MavHeader};
use serde_json::json;
use std::io::Write;
use std::sync::mpsc::Sender;
use std::{
    fs::File,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

pub struct MavlinkListener {
    duration: Option<Duration>,
    output_writer: Option<File>,
    message_sender: Sender<(MavHeader, MavMessage)>,
    error_sender: Sender<(Instant, String)>,
}

impl MavlinkListener {
    pub fn new(
        duration: Option<Duration>,
        output_file: Option<String>,
        message_sender: Sender<(MavHeader, MavMessage)>,
        error_sender: Sender<(Instant, String)>,
    ) -> Self {
        let output_writer = output_file
            .as_ref()
            .map(|filename| File::create(filename).expect("Failed to create output file"));

        MavlinkListener {
            duration,
            output_writer,
            message_sender,
            error_sender,
        }
    }

    pub fn listen(&self, connection: Arc<Mutex<Box<dyn MavConnection<MavMessage> + Send + Sync>>>) {
        let start_time = Instant::now();
        let mut last_timestamp = start_time;

        loop {
            if let Some(duration) = self.duration {
                if start_time.elapsed() > duration {
                    break;
                }
            }

            let conn = connection.lock().unwrap();
            match conn.recv() {
                Ok((header, message)) => {
                    let current_timestamp = Instant::now();
                    let time_diff = current_timestamp.duration_since(last_timestamp);
                    last_timestamp = current_timestamp;

                    self.write_logs(&header, &message, time_diff);
                    self.message_sender
                        .send((header, message))
                        .expect("Failed to send message to monitor");
                }
                Err(e) => {
                    self.error_sender
                        .send((Instant::now(), e.to_string()))
                        .expect("Failed to send error to monitor");
                }
            }
        }
    }

    fn write_logs(&self, header: &MavHeader, message: &MavMessage, time_diff: Duration) {
        let message_json = serde_json::to_string(&message).unwrap_or_else(|_| "{}".to_string());

        let time_message = json!({ "time_s": time_diff.as_secs_f64() }).to_string();
        let message_content = json!({
            "system_id": header.system_id,
            "component_id": header.component_id,
            "message": message_json,
        })
        .to_string();

        if let Some(mut writer) = self.output_writer.as_ref() {
            writeln!(writer, "{}\n{}", time_message, message_content)
                .expect("Failed to write to output file");
            writer.flush().expect("Failed to flush output file");
        }
    }

    // fn should_filter_message(&self, system_id: u8, component_id: u8) -> bool {
    //     if self.exclude_system_ids.contains(&system_id) {
    //         return true;
    //     }
    //     if !self.include_system_ids.is_empty() && !self.include_system_ids.contains(&system_id) {
    //         return true;
    //     }
    //     if self.exclude_component_ids.contains(&component_id) {
    //         return true;
    //     }
    //     if !self.include_component_ids.is_empty()
    //         && !self.include_component_ids.contains(&component_id)
    //     {
    //         return true;
    //     }
    //     false
    // }
}
