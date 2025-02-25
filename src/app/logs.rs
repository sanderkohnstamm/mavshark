use chrono::DateTime;
use ratatui::widgets::Block;
use ratatui::{
    layout::Constraint,
    style::{Color, Style},
    text::Span,
    widgets::{Borders, Row, Table},
};
use std::{
    sync::{
        mpsc::{self, Receiver},
        Arc, Mutex,
    },
    time::{Instant, SystemTime},
};

use super::LogLevel;

pub struct Logs {
    log_messages: Arc<Mutex<Vec<(Instant, LogLevel, String)>>>,
    logs_tx: mpsc::Sender<(Instant, LogLevel, String)>,
}

impl Logs {
    pub fn new() -> Self {
        let (logs_tx, logs_rx) = mpsc::channel();

        let logs = Logs {
            log_messages: Arc::new(Mutex::new(Vec::new())),
            logs_tx,
        };
        logs.spawn_update_thread(logs_rx);
        logs
    }

    pub fn logs_tx(&self) -> mpsc::Sender<(Instant, LogLevel, String)> {
        self.logs_tx.clone()
    }

    fn spawn_update_thread(&self, logs_rx: Receiver<(Instant, LogLevel, String)>) {
        let log_messages = Arc::clone(&self.log_messages);
        std::thread::spawn(move || loop {
            while let Ok((time, level, msg)) = logs_rx.recv() {
                let mut errors = log_messages.lock().unwrap();
                errors.push((time, level, msg));
            }
        });
    }

    pub fn log_error(&self, msg: &str) {
        self.logs_tx
            .send((Instant::now(), LogLevel::Error, msg.to_string()))
            .unwrap();
    }

    pub fn log_info(&self, msg: &str) {
        self.logs_tx
            .send((Instant::now(), LogLevel::Info, msg.to_string()))
            .unwrap();
    }

    pub fn to_tui_table(&self) -> Table {
        let errors = self.log_messages.lock().unwrap();
        let rows: Vec<Row> = errors
            .iter()
            .rev() // Reverse the order of log messages
            .map(|(time, level, msg)| {
                let duration = time.elapsed();
                let timestamp = SystemTime::now() - duration;
                let datetime: DateTime<chrono::Utc> = timestamp.into();
                let formatted_time = datetime.format("%Y-%m-%d %H:%M:%S").to_string();

                let color = match level {
                    LogLevel::Error => Color::Red,
                    LogLevel::Info => Color::White,
                };

                Row::new(vec![
                    Span::from(formatted_time),
                    Span::from(Span::styled(msg.clone(), Style::default().fg(color))),
                ])
            })
            .collect();

        Table::new(
            rows,
            &[Constraint::Percentage(20), Constraint::Percentage(80)],
        )
        .header(Row::new(vec![Span::from("Timestamp"), Span::from("Log")]))
        .block(Block::default().borders(Borders::ALL).title("Logs"))
    }
}
