use std::time::{Duration, Instant};

pub struct RollingWindow {
    timestamps: Vec<Instant>,
    max_size: usize,
}

impl RollingWindow {
    pub fn new(max_size: usize) -> Self {
        RollingWindow {
            timestamps: Vec::with_capacity(max_size),
            max_size,
        }
    }

    pub fn add(&mut self, timestamp: Instant) {
        if self.timestamps.len() >= self.max_size {
            self.timestamps.remove(0);
        }
        self.timestamps.push(timestamp);
    }

    pub fn calculate_hz(&self, current_timestamp: Instant) -> f64 {
        if self.timestamps.len() < 2 {
            return 0.0;
        }
        let first = self.timestamps.first().unwrap();
        let duration = current_timestamp.duration_since(*first).as_secs_f64();
        if duration > 0.0 {
            (self.timestamps.len() as f64 - 1.0) / duration
        } else {
            0.0
        }
    }

    pub fn should_be_cleared(&self, threshold: Duration) -> bool {
        if let Some(last) = self.timestamps.last() {
            last.elapsed() > threshold
        } else {
            false
        }
    }
}
