use mavlink::{
    common::{MavAutopilot, MavMessage, MavModeFlag, MavState, MavType},
    MavConnection, MavHeader,
};
use serde_json::Value;
use std::sync::{
    mpsc::{Receiver, Sender},
    Arc, Mutex,
};
use std::thread;
use std::{sync::atomic::AtomicBool, time::Duration};

use super::Logger;

#[derive(Clone)]
pub struct MavlinkSender {
    connection: Arc<Mutex<Box<dyn MavConnection<MavMessage> + Send + Sync>>>,
    logger: Logger,
    component_id_override: Option<u8>,
    system_id_override: Option<u8>,
    stop_signal: Arc<AtomicBool>,
    message_tx: Sender<(u8, u8, Value)>,
}

impl MavlinkSender {
    pub fn new(
        connection: Arc<Mutex<Box<dyn MavConnection<MavMessage> + Send + Sync>>>,
        logger: Logger,
        component_id_override: Option<u8>,
        system_id_override: Option<u8>,
        stop_signal: Arc<AtomicBool>,
    ) -> Self {
        let (message_tx, message_rx) = std::sync::mpsc::channel();
        let sender = MavlinkSender {
            connection,
            logger,
            component_id_override,
            system_id_override,
            stop_signal,
            message_tx,
        };

        sender.start_recv_loop(message_rx);
        sender
    }

    pub fn send(&self, message: (u8, u8, Value)) {
        if let Err(e) = self.message_tx.send(message.clone()) {
            self.logger
                .log_error(&format!("Failed to send message: {}", e));
        }
    }

    fn start_recv_loop(&self, message_rx: Receiver<(u8, u8, Value)>) {
        let connection = Arc::clone(&self.connection);
        let logger = self.logger.clone();
        let stop_signal = Arc::clone(&self.stop_signal);
        let system_id_override = self.system_id_override;
        let component_id_override = self.component_id_override;
        thread::spawn(move || {
            while !stop_signal.load(std::sync::atomic::Ordering::Relaxed) {
                match message_rx.recv_timeout(Duration::from_millis(100)) {
                    Ok((system_id, component_id, message)) => {
                        let system_id = match system_id_override {
                            Some(id) => id,
                            None => system_id,
                        };
                        let component_id = match component_id_override {
                            Some(id) => id,
                            None => component_id,
                        };

                        let mav_message: MavMessage = match serde_json::from_value(message) {
                            Ok(msg) => msg,
                            Err(e) => {
                                logger.log_error(&format!("Failed to parse MAV message: {}", e));
                                continue;
                            }
                        };

                        let header = MavHeader {
                            system_id,
                            component_id,
                            sequence: 0,
                        };

                        let conn = connection.lock().unwrap();
                        if let Err(e) = conn.send(&header, &mav_message) {
                            logger.log_error(&format!("Failed to send MAV message: {}", e));
                        } else {
                            logger.log_info(&format!(
                                "Message sent to system ID: {} and component ID: {}",
                                system_id, component_id
                            ));
                        }
                    }
                    Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                        // Continue the loop if timeout occurs
                        continue;
                    }
                    Err(e) => {
                        logger.log_error(&format!("Failed to receive message: {}", e));
                        break;
                    }
                }
            }
        });
    }

    pub fn start_heartbeat_loop(&self) {
        let Some(system_id) = self.system_id_override else {
            self.logger.log_error("Need a system ID for heartbeat loop");
            return;
        };
        let component_id = self.component_id_override.unwrap_or_default();
        let connection_clone = self.connection.clone();
        let logger = self.logger.clone();
        let stop_signal = self.stop_signal.clone();

        self.logger.log_info(&format!(
            "Starting heartbeat loop for system ID: {} and component ID: {}",
            system_id, component_id
        ));

        let heartbeat_interval = Duration::from_millis(500);
        thread::spawn(move || loop {
            if stop_signal.load(std::sync::atomic::Ordering::Relaxed) {
                break;
            }

            let heartbeat = MavMessage::HEARTBEAT(mavlink::common::HEARTBEAT_DATA {
                custom_mode: 0,
                mavtype: MavType::MAV_TYPE_GENERIC,
                autopilot: MavAutopilot::MAV_AUTOPILOT_INVALID,
                base_mode: MavModeFlag::empty(),
                system_status: MavState::MAV_STATE_ACTIVE,
                mavlink_version: 3,
            });

            let header = MavHeader {
                system_id,
                component_id,
                sequence: 0,
            };

            let conn = connection_clone.lock().unwrap();
            if let Err(e) = conn.send(&header, &heartbeat) {
                logger.log_error(&format!("Failed to send heartbeat: {}", e));
            }
            drop(conn);

            thread::sleep(heartbeat_interval);
        });
    }
}
