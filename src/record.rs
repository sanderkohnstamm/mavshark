use std::collections::HashSet;
use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::Path;

use anyhow::Result;
use chrono::{DateTime, Utc};
use mavlink::Message;
use mavlink_tlog::{TlogReader, TlogWriter};
use serde::{Deserialize, Serialize};

use crate::mavlink_io::ReceivedMessage;

#[derive(Serialize, Deserialize)]
pub struct RecordedHeader {
    pub system_id: u8,
    pub component_id: u8,
    pub sequence: u8,
}

#[derive(Serialize, Deserialize)]
pub struct RecordedMessage {
    pub timestamp: DateTime<Utc>,
    pub header: RecordedHeader,
    pub message_id: u32,
    pub message_name: String,
    pub message: String,
}

pub struct RecordFilter {
    names: HashSet<String>,
    ids: HashSet<u32>,
    accept_all: bool,
}

impl RecordFilter {
    pub fn new(spec: Option<&str>) -> Self {
        let spec = match spec {
            Some(s) if !s.is_empty() => s,
            _ => {
                return Self {
                    names: HashSet::new(),
                    ids: HashSet::new(),
                    accept_all: true,
                }
            }
        };

        let mut names = HashSet::new();
        let mut ids = HashSet::new();

        for token in spec.split(',') {
            let token = token.trim();
            if token.is_empty() {
                continue;
            }
            if let Ok(id) = token.parse::<u32>() {
                ids.insert(id);
            } else {
                names.insert(token.to_uppercase());
            }
        }

        Self {
            names,
            ids,
            accept_all: false,
        }
    }

    pub fn matches(&self, msg: &mavlink::ardupilotmega::MavMessage) -> bool {
        if self.accept_all {
            return true;
        }
        if self.ids.contains(&msg.message_id()) {
            return true;
        }
        if self.names.contains(&msg.message_name().to_uppercase()) {
            return true;
        }
        false
    }
}

pub struct Recorder {
    writer: BufWriter<File>,
    filter: RecordFilter,
    tlog: bool,
}

impl Recorder {
    pub fn new(path: &str, filter: RecordFilter) -> Result<Self> {
        let file = File::create(path)?;
        Ok(Self {
            writer: BufWriter::new(file),
            filter,
            tlog: is_tlog(Path::new(path)),
        })
    }

    pub fn record(&mut self, msg: &ReceivedMessage) {
        if !self.filter.matches(&msg.message) {
            return;
        }

        if self.tlog {
            let mut frame = Vec::new();
            if mavlink::write_v2_msg(&mut frame, msg.header, &msg.message).is_ok() {
                let _ = TlogWriter::new(&mut self.writer).write_frame_now(&frame);
            }
            return;
        }

        let recorded = RecordedMessage {
            timestamp: Utc::now(),
            header: RecordedHeader {
                system_id: msg.header.system_id,
                component_id: msg.header.component_id,
                sequence: msg.header.sequence,
            },
            message_id: msg.message.message_id(),
            message_name: msg.message.message_name().to_string(),
            message: format!("{:#?}", msg.message),
        };

        if let Ok(json) = serde_json::to_string(&recorded) {
            let _ = writeln!(self.writer, "{}", json);
        }
    }

    pub fn flush(&mut self) {
        let _ = self.writer.flush();
    }
}

fn is_tlog(path: &Path) -> bool {
    path.extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("tlog"))
}

pub fn load_recording(path: &Path) -> Result<Vec<RecordedMessage>> {
    if is_tlog(path) {
        return load_tlog(path);
    }
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let mut messages = Vec::new();

    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        match serde_json::from_str::<RecordedMessage>(&line) {
            Ok(msg) => messages.push(msg),
            Err(e) => log::warn!("Skipping invalid record: {}", e),
        }
    }

    Ok(messages)
}

fn load_tlog(path: &Path) -> Result<Vec<RecordedMessage>> {
    let mut log = TlogReader::new(BufReader::new(File::open(path)?));
    let mut messages = Vec::new();

    while let Some(rec) = log.next_record()? {
        let frame = rec.frame;
        let version = match frame.version() {
            mavlink_tlog::Version::V1 => mavlink::MavlinkVersion::V1,
            _ => mavlink::MavlinkVersion::V2,
        };
        let message = match mavlink::ardupilotmega::MavMessage::parse(
            version,
            frame.msg_id(),
            frame.payload(),
        ) {
            Ok(m) => m,
            Err(e) => {
                log::warn!("Skipping undecodable message {}: {}", frame.msg_id(), e);
                continue;
            }
        };
        messages.push(RecordedMessage {
            timestamp: DateTime::from_timestamp_micros(rec.timestamp_us as i64).unwrap_or_default(),
            header: RecordedHeader {
                system_id: frame.sys_id(),
                component_id: frame.comp_id(),
                sequence: frame.seq(),
            },
            message_id: message.message_id(),
            message_name: message.message_name().to_string(),
            message: format!("{:#?}", message),
        });
    }

    let stats = log.stats();
    if stats.bad_crc > 0 || stats.skipped_bytes > 0 {
        log::warn!(
            "{}: skipped {} damaged bytes, {} bad checksums",
            path.display(),
            stats.skipped_bytes,
            stats.bad_crc
        );
    }
    Ok(messages)
}

#[cfg(test)]
mod tests {
    use super::*;
    use mavlink::ardupilotmega::{MavMessage, ATTITUDE_DATA};
    use mavlink::MavHeader;
    use std::time::Instant;

    #[test]
    fn tlog_record_round_trips_through_replay() {
        let path = std::env::temp_dir().join("mavshark_roundtrip_test.tlog");
        let mut rec = Recorder::new(path.to_str().unwrap(), RecordFilter::new(None)).unwrap();
        let header = MavHeader {
            system_id: 1,
            component_id: 1,
            sequence: 7,
        };
        let attitude = MavMessage::ATTITUDE(ATTITUDE_DATA {
            roll: 0.25,
            ..Default::default()
        });
        rec.record(&ReceivedMessage {
            header,
            message: attitude.clone(),
            received_at: Instant::now(),
        });
        rec.flush();
        drop(rec);

        // The file is a standard tlog: every frame passes the MAVLink CRC check.
        let mut reader = TlogReader::new(File::open(&path).unwrap());
        while reader.next_record().unwrap().is_some() {}
        assert_eq!(reader.stats().records, 1);
        assert_eq!(reader.stats().bad_crc, 0);

        let loaded = load_recording(&path).unwrap();
        std::fs::remove_file(&path).unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].message_name, "ATTITUDE");
        assert_eq!(loaded[0].header.sequence, 7);
        assert_eq!(loaded[0].message, format!("{:#?}", attitude));
    }
}
