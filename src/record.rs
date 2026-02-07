use std::collections::HashSet;
use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::Path;

use anyhow::Result;
use chrono::{DateTime, Utc};
use mavlink::Message;
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
}

impl Recorder {
    pub fn new(path: &str, filter: RecordFilter) -> Result<Self> {
        let file = File::create(path)?;
        Ok(Self {
            writer: BufWriter::new(file),
            filter,
        })
    }

    pub fn record(&mut self, msg: &ReceivedMessage) {
        if !self.filter.matches(&msg.message) {
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

pub fn load_recording(path: &Path) -> Result<Vec<RecordedMessage>> {
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
