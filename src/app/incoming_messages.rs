use std::{
    collections::HashMap,
    sync::{
        mpsc::{self, Receiver},
        Arc, Mutex,
    },
    thread,
    time::{Duration, Instant},
};

use mavlink::{common::MavMessage, MavHeader};
use ratatui::{
    layout::Constraint,
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Cell, Row, Table, TableState},
};
use serde_json::Value;

use super::rolling_window::RollingWindow;

pub struct IncomingMessages {
    message_counts: Arc<Mutex<HashMap<(u8, u8, String), RollingWindow>>>,
    last_messages: Arc<Mutex<HashMap<(u8, u8, String), String>>>,
    message_tx: mpsc::Sender<(MavHeader, MavMessage)>,
    state: TableState,
}

impl IncomingMessages {
    pub fn new() -> IncomingMessages {
        let (message_tx, message_rx) = mpsc::channel();

        let messages = IncomingMessages {
            message_counts: Arc::new(Mutex::new(HashMap::new())),
            last_messages: Arc::new(Mutex::new(HashMap::new())),
            message_tx,
            state: TableState::default(),
        };

        messages.spawn_hz_calculations();
        messages.spawn_update_thread(message_rx);
        messages
    }

    pub fn message_tx(&self) -> mpsc::Sender<(MavHeader, MavMessage)> {
        self.message_tx.clone()
    }

    pub fn state(&self) -> TableState {
        self.state.clone()
    }

    pub fn clear(&mut self) {
        self.message_counts.lock().unwrap().clear();
        self.last_messages.lock().unwrap().clear();
    }

    pub fn get_selected_message_string(&self) -> Option<String> {
        let message_counts = self.message_counts.lock().unwrap();
        let selected = self.state.selected()?;
        let key = message_counts.keys().nth(selected)?;
        let last_message = self.last_messages.lock().unwrap().get(key).cloned()?;
        Some(pretty_print_json(&last_message))
    }

    pub fn get_selected_message_hz_history(&self) -> Vec<f64> {
        let message_counts = self.message_counts.lock().unwrap();
        let Some(selected) = self.state.selected() else {
            return vec![];
        };
        let Some(key) = message_counts.keys().nth(selected) else {
            return vec![];
        };
        let Some(window) = message_counts.get(key) else {
            return vec![];
        };
        window.get_history()
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

    pub fn to_tui_table(&self, active: bool, selected: bool) -> Table {
        let selected_style = Style::default().add_modifier(Modifier::REVERSED);
        let header_cells = ["System ID", "Component ID", "Message Type", "Hz"]
            .iter()
            .map(|h| Cell::from(*h).style(Style::default().fg(Color::White)));
        let header = Row::new(header_cells).height(1).bottom_margin(1);

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

        let table = Table::new(
            rows,
            &[
                Constraint::Percentage(5),
                Constraint::Percentage(5),
                Constraint::Percentage(80),
                Constraint::Percentage(10),
            ],
        )
        .header(header)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Message Counts")
                .border_style(Style::default().fg(if active {
                    if selected {
                        Color::Green
                    } else {
                        Color::LightCyan
                    }
                } else {
                    Color::Gray
                })),
        )
        .row_highlight_style(selected_style);
        table
    }

    fn spawn_update_thread(&self, message_rx: Receiver<(MavHeader, MavMessage)>) {
        let message_counts = Arc::clone(&self.message_counts);
        let last_messages = Arc::clone(&self.last_messages);
        thread::spawn(move || {
            while let Ok((header, message)) = message_rx.recv() {
                let timestamp = Instant::now();

                let message_json =
                    serde_json::to_string(&message).unwrap_or_else(|_| "{}".to_string());
                let message_type = serde_json::from_str::<serde_json::Value>(&message_json)
                    .ok()
                    .and_then(|msg| msg.get("type").and_then(|t| t.as_str()).map(String::from))
                    .unwrap_or_else(|| "UNKNOWN".to_string());

                message_counts
                    .lock()
                    .unwrap()
                    .entry((header.system_id, header.component_id, message_type.clone()))
                    .or_insert_with(|| RollingWindow::new(Duration::from_secs(10)))
                    .add(timestamp);

                last_messages.lock().unwrap().insert(
                    (header.system_id, header.component_id, message_type),
                    message_json,
                );
            }
        });
    }

    fn spawn_hz_calculations(&self) {
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
}

fn pretty_print_json(json_str: &str) -> String {
    serde_json::from_str::<Value>(json_str)
        .map(|json_value| {
            serde_json::to_string_pretty(&json_value).unwrap_or_else(|_| json_str.to_string())
        })
        .unwrap_or_else(|_| json_str.to_string())
}
