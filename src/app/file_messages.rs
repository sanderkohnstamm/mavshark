use std::{
    collections::HashMap,
    fs::File,
    io::{BufRead, BufReader},
};

use ratatui::{
    layout::Constraint,
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Cell, Row, Table, TableState},
};
use serde_json::Value;

#[derive(PartialEq)]
enum MessageTable {
    FullMessages,
    SelectedKeyMessages,
}

pub struct FileMessages {
    full_messages: HashMap<(u8, u8, String), Vec<Value>>,
    full_messages_index: TableState,
    selected_messages_index: TableState,
    active_message_table: MessageTable,
}

impl FileMessages {
    pub fn new() -> FileMessages {
        FileMessages {
            full_messages: HashMap::new(),
            full_messages_index: TableState::default(),
            selected_messages_index: TableState::default(),
            active_message_table: MessageTable::FullMessages,
        }
    }

    pub fn read_file(&mut self, file_path: &str) {
        let file = File::open(file_path).expect("Unable to open file");
        let reader = BufReader::new(file);

        for line in reader.lines() {
            let Some((system_id, component_id, message_type, message)) = parse_line(&line.unwrap())
            else {
                continue;
            };

            self.full_messages
                .entry((system_id, component_id, message_type.clone()))
                .or_insert_with(Vec::new)
                .push(message);
        }
    }

    pub fn clear_messages(&mut self) {
        self.full_messages.clear();
    }

    pub fn selected_messages_state(&self) -> TableState {
        self.selected_messages_index.clone()
    }

    pub fn full_messages_state(&self) -> TableState {
        self.full_messages_index.clone()
    }

    pub fn switch_selected_table(&mut self) {
        self.active_message_table = match self.active_message_table {
            MessageTable::FullMessages => MessageTable::SelectedKeyMessages,
            MessageTable::SelectedKeyMessages => MessageTable::FullMessages,
        };
    }

    pub fn key_up(&mut self) {
        match self.active_message_table {
            MessageTable::FullMessages => self.full_messages_index_up(),
            MessageTable::SelectedKeyMessages => self.selected_messages_index_up(),
        }
    }

    pub fn key_down(&mut self) {
        match self.active_message_table {
            MessageTable::FullMessages => self.full_messages_index_down(),
            MessageTable::SelectedKeyMessages => self.selected_messages_index_down(),
        }
    }

    pub fn full_messages_index_down(&mut self) {
        let i = match self.full_messages_index.selected() {
            Some(i) => {
                let len = self.full_messages.len();
                if len == 0 {
                    0
                } else {
                    0.max((i + 1) % len)
                }
            }
            None => 0,
        };
        self.full_messages_index.select(Some(i));
    }

    pub fn full_messages_index_up(&mut self) {
        let i = match self.full_messages_index.selected() {
            Some(i) => {
                let len = self.full_messages.len();
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
        self.full_messages_index.select(Some(i));
    }

    pub fn selected_messages_index_down(&mut self) {
        let Some(key) = self.get_selected_key() else {
            return;
        };

        let i = match self.selected_messages_index.selected() {
            Some(i) => {
                let len = self.full_messages.get(&key).unwrap().len();
                if len == 0 {
                    0
                } else {
                    0.max((i + 1) % len)
                }
            }
            None => 0,
        };
        self.selected_messages_index.select(Some(i));
    }

    pub fn selected_messages_index_up(&mut self) {
        let Some(key) = self.get_selected_key() else {
            return;
        };
        let i = match self.selected_messages_index.selected() {
            Some(i) => {
                let len = self.full_messages.get(&key).unwrap().len();
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
        self.selected_messages_index.select(Some(i));
    }

    pub fn get_selected_key(&self) -> Option<(u8, u8, String)> {
        let selected = self.full_messages_index.selected()?;
        self.full_messages.keys().nth(selected).cloned()
    }

    pub fn get_selected_message(&self) -> Option<Value> {
        let key = self.get_selected_key()?;
        let messages = self.full_messages.get(&key)?;
        let selected = self.selected_messages_index.selected()?;
        messages.get(selected).cloned()
    }

    pub fn get_selected_message_pretty(&self) -> Option<String> {
        let message = self.get_selected_message()?;
        match serde_json::to_string_pretty(&message) {
            Ok(s) => Some(s),
            Err(_) => None,
        }
    }

    pub fn to_tui_table(&self, active: bool) -> Table {
        let selected_style = Style::default().add_modifier(Modifier::REVERSED);
        let header_cells = ["System ID", "Component ID", "Message Type", "Count"]
            .iter()
            .map(|h| Cell::from(*h).style(Style::default().fg(Color::White)));
        let header = Row::new(header_cells).height(1).bottom_margin(1);

        let rows =
            self.full_messages
                .iter()
                .map(|((system_id, component_id, message_type), messages)| {
                    let height = 1;
                    let count_string = messages.len().to_string();
                    let cells = vec![
                        Cell::from(system_id.to_string()),
                        Cell::from(component_id.to_string()),
                        Cell::from(message_type.clone()),
                        Cell::from(count_string),
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
                .title("Read Messages")
                .border_style(Style::default().fg(if active {
                    if self.active_message_table == MessageTable::FullMessages {
                        Color::Green
                    } else {
                        Color::LightBlue
                    }
                } else {
                    Color::Gray
                })),
        )
        .row_highlight_style(selected_style);
        table
    }

    pub fn to_tui_table_selected_key(&self, active: bool) -> Table {
        let selected_style = Style::default().add_modifier(Modifier::REVERSED);
        let header_cells = ["Message"]
            .iter()
            .map(|h| Cell::from(*h).style(Style::default().fg(Color::White)));
        let header = Row::new(header_cells).height(1).bottom_margin(1);

        let key = self.get_selected_key();
        let rows = if let Some(key) = key {
            self.full_messages
                .get(&key)
                .unwrap()
                .iter()
                .map(|message| {
                    let height = 1;
                    let message_str = message.to_string();

                    let cells = vec![Cell::from(message_str)];
                    Row::new(cells).height(height as u16)
                })
                .collect::<Vec<_>>()
        } else {
            vec![]
        };

        let table = Table::new(rows, &[Constraint::Percentage(100)])
            .header(header)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Selected Messages")
                    .border_style(Style::default().fg(if active {
                        if self.active_message_table == MessageTable::SelectedKeyMessages {
                            Color::Green
                        } else {
                            Color::LightBlue
                        }
                    } else {
                        Color::Gray
                    })),
            )
            .row_highlight_style(selected_style);
        table
    }
}

fn parse_line(line: &str) -> Option<(u8, u8, String, Value)> {
    let parsed: Value = serde_json::from_str(line).unwrap_or(None)?;
    let system_id = parsed["system_id"].as_i64()? as u8;
    let component_id = parsed["component_id"].as_i64()? as u8;
    let message_str = parsed["message"].as_str()?;
    let message: Value = serde_json::from_str(message_str).unwrap_or(None)?;
    let message_type = message["type"].as_str()?.to_string();

    Some((system_id, component_id, message_type, message))
}
