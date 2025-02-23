use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};

use serde_json::Value;
use tui::{
    layout::Constraint,
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Cell, Row, Table, TableState},
};

use super::rolling_window::RollingWindow;

pub struct WidgetFrequencies {
    message_counts: Arc<Mutex<HashMap<(u8, u8, String), RollingWindow>>>,
    last_messages: Arc<Mutex<HashMap<(u8, u8, String), String>>>,
    pub state: TableState,
}

impl WidgetFrequencies {
    pub fn new_with(
        message_counts: Arc<Mutex<HashMap<(u8, u8, String), RollingWindow>>>,
        last_messages: Arc<Mutex<HashMap<(u8, u8, String), String>>>,
    ) -> WidgetFrequencies {
        let widget_frequencies = WidgetFrequencies {
            message_counts,
            last_messages,
            state: TableState::default(),
        };

        widget_frequencies.spawn_update_thread();
        widget_frequencies
    }

    fn spawn_update_thread(&self) {
        let message_counts = Arc::clone(&self.message_counts);
        thread::spawn(move || loop {
            {
                let mut counts = message_counts.lock().unwrap();
                for window in counts.values_mut() {
                    window.update();
                }
            }
            thread::sleep(Duration::from_millis(100));
        });
    }

    pub fn get_selected_message_string(&self) -> Option<String> {
        let message_counts = self.message_counts.lock().unwrap();
        let selected = self.state.selected()?;
        let key = message_counts.keys().nth(selected)?;
        let last_message = self.last_messages.lock().unwrap().get(key).cloned()?;
        Some(pretty_print_json(&last_message))
    }

    pub fn select_down(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                let len = self.message_counts.lock().unwrap().len();
                if len == 0 {
                    0
                } else {
                    0.max((i + 1) % len)
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    pub fn select_up(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                let len = self.message_counts.lock().unwrap().len();
                if len == 0 || i == 0 || i == 1 {
                    0
                } else if i == 0 {
                    len - 1
                } else {
                    0.max((i - 1) % len)
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    pub fn to_tui_table(&self) -> Table {
        let selected_style = Style::default().add_modifier(Modifier::REVERSED);
        let normal_style = Style::default().bg(Color::Blue);
        let header_cells = ["System ID", "Component ID", "Message Type", "Hz"]
            .iter()
            .map(|h| Cell::from(*h).style(Style::default().fg(Color::Red)));
        let header = Row::new(header_cells)
            .style(normal_style)
            .height(1)
            .bottom_margin(1);

        let message_counts = self.message_counts.lock().unwrap();
        let rows =
            message_counts
                .iter()
                .map(|((system_id, component_id, message_type), window)| {
                    let height = 1;
                    let hz_string = window.get_hz().to_string();
                    let cells = vec![
                        Cell::from(system_id.to_string()),
                        Cell::from(component_id.to_string()),
                        Cell::from(message_type.clone()),
                        Cell::from(hz_string),
                    ];
                    Row::new(cells).height(height as u16)
                });

        let table = Table::new(rows)
            .header(header)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Message Counts"),
            )
            .highlight_style(selected_style)
            .highlight_symbol(">")
            .widths(&[
                Constraint::Percentage(5),
                Constraint::Percentage(5),
                Constraint::Percentage(80),
                Constraint::Percentage(10),
            ]);
        table
    }
}

fn pretty_print_json(json_str: &str) -> String {
    serde_json::from_str::<Value>(json_str)
        .map(|json_value| {
            serde_json::to_string_pretty(&json_value).unwrap_or_else(|_| json_str.to_string())
        })
        .unwrap_or_else(|_| json_str.to_string())
}
