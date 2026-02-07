use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use mavlink::ardupilotmega::*;
use mavlink::{MavConnection, MavHeader, Message};

pub struct ReceivedMessage {
    pub header: MavHeader,
    pub message: MavMessage,
    pub received_at: Instant,
}

pub fn listener_loop(
    conn: Arc<Box<dyn MavConnection<MavMessage> + Sync + Send>>,
    tx: std::sync::mpsc::Sender<ReceivedMessage>,
    stop: Arc<AtomicBool>,
) {
    while !stop.load(Ordering::Relaxed) {
        match conn.recv() {
            Ok((header, message)) => {
                log::info!(
                    "recv {} from {}:{}",
                    message.message_name(),
                    header.system_id,
                    header.component_id
                );
                let msg = ReceivedMessage {
                    header,
                    message,
                    received_at: Instant::now(),
                };
                if tx.send(msg).is_err() {
                    break;
                }
            }
            Err(e) => {
                log::error!("MAVLink recv error: {}", e);
                if stop.load(Ordering::Relaxed) {
                    break;
                }
                std::thread::sleep(Duration::from_millis(100));
            }
        }
    }
    log::info!("Listener stopped");
}

pub fn heartbeat_loop(
    conn: Arc<Box<dyn MavConnection<MavMessage> + Sync + Send>>,
    sys_id: u8,
    comp_id: u8,
    stop: Arc<AtomicBool>,
) {
    let header = MavHeader {
        system_id: sys_id,
        component_id: comp_id,
        sequence: 0,
    };
    let msg = MavMessage::HEARTBEAT(HEARTBEAT_DATA {
        custom_mode: 0,
        mavtype: MavType::MAV_TYPE_GCS,
        autopilot: MavAutopilot::MAV_AUTOPILOT_INVALID,
        base_mode: MavModeFlag::empty(),
        system_status: MavState::MAV_STATE_ACTIVE,
        mavlink_version: 3,
    });

    log::info!("Sending heartbeats as {}:{}", sys_id, comp_id);
    while !stop.load(Ordering::Relaxed) {
        if let Err(e) = conn.send(&header, &msg) {
            log::error!("Heartbeat send error: {}", e);
        }
        std::thread::sleep(Duration::from_secs(1));
    }
    log::info!("Heartbeat stopped");
}
