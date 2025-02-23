use std::time::{Duration, Instant};

pub struct RollingWindow {
    timestamps: Vec<Instant>,
    max_duration: Duration,
    hz: f64,
}

impl RollingWindow {
    pub fn new(max_duration: Duration) -> Self {
        RollingWindow {
            timestamps: Vec::new(),
            max_duration,
            hz: 0.0,
        }
    }

    pub fn add(&mut self, timestamp: Instant) {
        self.timestamps.push(timestamp);
        self.update();
    }

    pub fn update(&mut self) {
        self.clean_old_timestamps();
        self.calculate_hz();
    }

    fn clean_old_timestamps(&mut self) {
        let current_timestamp = Instant::now();

        self.timestamps
            .retain(|&t| current_timestamp.duration_since(t) <= self.max_duration);
    }

    pub fn calculate_hz(&mut self) {
        let current_timestamp = Instant::now();

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
        (self.hz * 100.0).round() / 100.0
    }
}
