mod rolling_window;
use std::{
    collections::HashMap,
    io::{stdout, Write},
    sync::{Arc, Mutex},
    thread,
    time::{Duration, Instant},
};

use crossterm::{
    cursor::{Hide, MoveTo},
    execute,
    terminal::{Clear, ClearType},
};
use rolling_window::RollingWindow;
pub struct MavlinkMonitor {
    message_counts: Arc<Mutex<HashMap<(u8, u8, String), RollingWindow>>>,
    monitor_clear_threshold: Duration,
    monitor_interval: Duration,
    hz_window_size: usize,
}

impl MavlinkMonitor {
    pub fn new() -> Self {
        MavlinkMonitor {
            message_counts: Arc::new(Mutex::new(HashMap::new())),
            monitor_clear_threshold: Duration::from_secs(2),
            monitor_interval: Duration::from_millis(200),
            hz_window_size: 10,
        }
    }

    pub fn start(&self) {
        let message_counts = Arc::clone(&self.message_counts);
        let monitor_clear_threshold = self.monitor_clear_threshold;
        let monitor_interval = self.monitor_interval;

        thread::spawn(move || {
            let mut stdout = stdout();
            execute!(stdout, Hide).unwrap();

            loop {
                thread::sleep(monitor_interval);
                let current_timestamp = Instant::now();

                let mut message_counts = message_counts.lock().unwrap();
                message_counts
                    .retain(|_, window| !window.should_be_cleared(monitor_clear_threshold));

                execute!(stdout, MoveTo(0, 0), Clear(ClearType::FromCursorDown)).unwrap();
                println!(
                    "{:<10} {:<15} {:<35} {:<10}",
                    "System ID", "Component ID", "Message Type", "Hz"
                );
                println!("{}", "-".repeat(75));

                for ((system_id, component_id, msg_type), window) in message_counts.iter() {
                    let hz = window.calculate_hz(current_timestamp);
                    println!(
                        "{:<10} {:<15} {:<35} {:<10.2}",
                        system_id, component_id, msg_type, hz
                    );
                }
                stdout.flush().unwrap();
            }
        });
    }

    pub fn update(
        &self,
        system_id: u8,
        component_id: u8,
        message_type: String,
        timestamp: Instant,
    ) {
        let mut message_counts = self.message_counts.lock().unwrap();
        message_counts
            .entry((system_id, component_id, message_type))
            .or_insert_with(|| RollingWindow::new(self.hz_window_size))
            .add(timestamp);
    }
}
