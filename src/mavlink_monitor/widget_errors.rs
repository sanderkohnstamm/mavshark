use chrono::DateTime;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::{Instant, SystemTime},
};
use tui::{
    layout::Constraint,
    widgets::{Borders, Row, Table},
};
use tui::{text::Spans, widgets::Block};

pub struct WidgetErrors {
    error_messages: Arc<Mutex<HashMap<String, Instant>>>,
}

impl WidgetErrors {
    pub fn new_with(error_messages: Arc<Mutex<HashMap<String, Instant>>>) -> Self {
        WidgetErrors { error_messages }
    }

    pub fn get_errors(&self) -> Arc<Mutex<HashMap<String, Instant>>> {
        Arc::clone(&self.error_messages)
    }

    pub fn to_tui_table(&self) -> Table {
        let errors = self.error_messages.lock().unwrap();
        let rows: Vec<Row> = errors
            .iter()
            .map(|(msg, time)| {
                let duration = time.elapsed();
                let timestamp = SystemTime::now() - duration;
                let datetime: DateTime<chrono::Utc> = timestamp.into();
                let formatted_time = datetime.format("%Y-%m-%d %H:%M:%S").to_string();

                Row::new(vec![Spans::from(formatted_time), Spans::from(msg.clone())])
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
