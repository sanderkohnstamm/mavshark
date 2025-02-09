use mavlink::{common::MavMessage, MavConnection, MavHeader};
use serde_json::Value;
use std::{
    fs::File,
    io::{BufRead, BufReader},
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};

pub struct MavlinkSender;

impl MavlinkSender {
    pub fn replay(
        connection: Arc<Mutex<Box<dyn MavConnection<MavMessage> + Send + Sync>>>,
        input_file: &str,
    ) {
        let file = File::open(input_file).expect("Failed to open input file");
        let reader = BufReader::new(file);

        println!("Sending MAVLink messages...");

        for line in reader.lines() {
            let line = line.expect("Failed to read line");
            let json_msg: Value = serde_json::from_str(&line).expect("Failed to parse JSON");

            // time message
            if let Some(time_since_last) = json_msg["time_s"].as_f64() {
                println!("Waiting for {time_since_last} s");
                thread::sleep(Duration::from_secs_f64(time_since_last));
                continue;
            }

            // mavlink message
            let system_id = json_msg["system_id"].as_u64().expect("Invalid system_id") as u8;
            let component_id = json_msg["component_id"]
                .as_u64()
                .expect("Invalid component_id") as u8;

            let message_str = json_msg["message"]
                .as_str()
                .expect("Invalid message format");

            // Replace null values with 0.0 in the JSON string
            let cleaned_message_str = message_str.replace(":null", ":0.0");

            let mav_message: MavMessage = serde_json::from_str(&cleaned_message_str)
                .expect("Failed to parse MAVLink message");

            let header = MavHeader {
                system_id,
                component_id,
                sequence: 0,
            };

            let conn = connection.lock().unwrap();
            conn.send(&header, &mav_message)
                .expect("Failed to send MAVLink message");
            println!("Sent: {:?}, {}", header, message_str);
        }

        println!("Finished sending MAVLink messages.");
    }
}
