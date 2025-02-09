use mavlink::{common::MavMessage, MavConnection};
use std::{
    fs::File,
    io::Write,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

pub struct MavlinkListener {
    duration: Option<Duration>,
    include_system_ids: Vec<u8>,
    exclude_system_ids: Vec<u8>,
    include_component_ids: Vec<u8>,
    exclude_component_ids: Vec<u8>,
    output_file: Option<String>,
}

impl MavlinkListener {
    pub fn new(
        duration: Option<Duration>,
        include_system_ids: Vec<u8>,
        exclude_system_ids: Vec<u8>,
        include_component_ids: Vec<u8>,
        exclude_component_ids: Vec<u8>,
        output_file: Option<String>,
    ) -> Self {
        MavlinkListener {
            duration,
            include_system_ids,
            exclude_system_ids,
            include_component_ids,
            exclude_component_ids,
            output_file,
        }
    }

    pub fn listen(&self, connection: Arc<Mutex<Box<dyn MavConnection<MavMessage> + Send + Sync>>>) {
        println!("Listening for MAVLink messages...");
        let start_time = Instant::now();

        let mut output_writer = self
            .output_file
            .as_ref()
            .map(|filename| File::create(filename).expect("Failed to create output file"));

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

                    let log_message = format!(
                        "System ID: {}, Component ID: {} \n {:#?}\n",
                        header.system_id, header.component_id, message
                    );
                    println!("{}", log_message);

                    if let Some(writer) = output_writer.as_mut() {
                        let mut buffer = vec![];
                        mavlink::write_versioned_msg(
                            &mut buffer,
                            mavlink::MavlinkVersion::V2,
                            header,
                            &message,
                        )
                        .expect("Failed to encode MAVLink message");

                        writer
                            .write_all(&buffer)
                            .expect("Failed to write MAVLink binary");
                        writer.flush().expect("Failed to flush output file");
                    }
                }
                Err(e) => {
                    eprintln!("Error receiving MAVLink message: {}", e);
                    break;
                }
            }
        }
    }
}
