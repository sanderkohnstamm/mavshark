use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::Instant,
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
                Row::new(vec![
                    Spans::from(msg.clone()),
                    Spans::from(format!("{:?}", time)),
                ])
            })
            .collect();

        Table::new(rows)
            .header(Row::new(vec![
                Spans::from("Error Message"),
                Spans::from("Timestamp"),
            ]))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Error Messages"),
            )
            .widths(&[Constraint::Percentage(70), Constraint::Percentage(30)])
    }
}
