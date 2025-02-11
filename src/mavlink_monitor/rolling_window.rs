use std::time::{Duration, Instant};

pub struct RollingWindow {
    timestamps: Vec<Instant>,
    max_size: usize,
    hz: f64,
}

impl RollingWindow {
    pub fn new(max_size: usize) -> Self {
        RollingWindow {
            timestamps: Vec::with_capacity(max_size),
            max_size,
            hz: 0.0,
        }
    }

    pub fn add(&mut self, timestamp: Instant) {
        if self.timestamps.len() >= self.max_size {
            self.timestamps.remove(0);
        }
        self.timestamps.push(timestamp);
        self.calculate_hz(timestamp);
    }

    fn calculate_hz(&mut self, current_timestamp: Instant) {
        if self.timestamps.len() < 2 {
            self.hz = 0.0;
            return;
        }
        let first = self.timestamps.first().unwrap();
        let duration = current_timestamp.duration_since(*first).as_secs_f64();
        if duration > 0.0 {
            self.hz = (self.timestamps.len() as f64 - 1.0) / duration;
        } else {
            self.hz = 0.0;
        }
    }

    pub fn get_hz(&self) -> f64 {
        self.hz
    }

    pub fn should_be_cleared(&self, threshold: Duration) -> bool {
        if let Some(last) = self.timestamps.last() {
            last.elapsed() > threshold
        } else {
            false
        }
    }
}
