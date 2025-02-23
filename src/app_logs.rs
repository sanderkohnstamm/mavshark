use chrono::DateTime;
use std::{
    sync::{Arc, Mutex},
    time::{Instant, SystemTime},
};
use tui::{
    layout::Constraint,
    style::{Color, Style},
    text::Span,
    widgets::{Borders, Row, Table},
};
use tui::{text::Spans, widgets::Block};

pub enum LogLevel {
    Info,
    Error,
}

pub struct AppLogs {
    log_messages: Arc<Mutex<Vec<(Instant, LogLevel, String)>>>,
}

impl AppLogs {
    pub fn new_with(log_messages: Arc<Mutex<Vec<(Instant, LogLevel, String)>>>) -> Self {
        AppLogs { log_messages }
    }

    pub fn get_errors(&self) -> Arc<Mutex<Vec<(Instant, LogLevel, String)>>> {
        Arc::clone(&self.log_messages)
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
                    Spans::from(formatted_time),
                    Spans::from(Span::styled(msg.clone(), Style::default().fg(color))),
                ])
            })
            .collect();

        Table::new(rows)
            .header(Row::new(vec![
                Spans::from("Timestamp"),
                Spans::from("Error Message"),
            ]))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Error Messages"),
            )
            .widths(&[Constraint::Percentage(30), Constraint::Percentage(70)])
    }
}
