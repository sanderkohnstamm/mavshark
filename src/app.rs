use std::collections::{HashMap, VecDeque};
use std::time::Instant;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::widgets::TableState;

use mavlink::Message;

use crate::mavlink_io::ReceivedMessage;

const HZ_WINDOW_SECS: f64 = 5.0;

pub struct MessageEntry {
    pub name: String,
    pub sys_id: u8,
    pub comp_id: u8,
    pub count: u64,
    pub hz: f64,
    pub last_content: String,
    timestamps: VecDeque<Instant>,
}

pub enum SortMode {
    Alphabetical,
    Hz,
    Count,
}

pub struct App {
    pub uri: String,
    pub heartbeat: Option<(u8, u8)>,
    pub entries: Vec<MessageEntry>,
    index: HashMap<(u8, u8, String), usize>,
    pub selected: usize,
    pub detail_scroll: usize,
    pub filter: String,
    pub filter_active: bool,
    pub filtered_indices: Vec<usize>,
    pub total_count: u64,
    pub table_state: TableState,
    pub sort_mode: SortMode,
}

impl App {
    pub fn new(uri: String, heartbeat: Option<(u8, u8)>) -> Self {
        Self {
            uri,
            heartbeat,
            entries: Vec::new(),
            index: HashMap::new(),
            selected: 0,
            detail_scroll: 0,
            filter: String::new(),
            filter_active: false,
            filtered_indices: Vec::new(),
            total_count: 0,
            table_state: TableState::default(),
            sort_mode: SortMode::Alphabetical,
        }
    }

    pub fn on_message(&mut self, msg: ReceivedMessage) {
        self.total_count += 1;
        let name = msg.message.message_name().to_string();
        let key = (msg.header.system_id, msg.header.component_id, name.clone());
        let content = format!("{:#?}", msg.message);

        if let Some(&idx) = self.index.get(&key) {
            let entry = &mut self.entries[idx];
            entry.count += 1;
            entry.last_content = content;
            entry.timestamps.push_back(msg.received_at);
        } else {
            let idx = self.entries.len();
            let mut timestamps = VecDeque::new();
            timestamps.push_back(msg.received_at);
            self.entries.push(MessageEntry {
                name: name.clone(),
                sys_id: msg.header.system_id,
                comp_id: msg.header.component_id,
                count: 1,
                hz: 0.0,
                last_content: content,
                timestamps,
            });
            self.index.insert(key, idx);
        }

        self.rebuild_filter();
    }

    pub fn tick(&mut self) {
        let now = Instant::now();
        for entry in &mut self.entries {
            while let Some(&front) = entry.timestamps.front() {
                if now.duration_since(front).as_secs_f64() > HZ_WINDOW_SECS {
                    entry.timestamps.pop_front();
                } else {
                    break;
                }
            }
            entry.hz = entry.timestamps.len() as f64 / HZ_WINDOW_SECS;
        }
    }

    pub fn on_key(&mut self, key: KeyEvent) -> bool {
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            return true;
        }

        if self.filter_active {
            match key.code {
                KeyCode::Esc => {
                    self.filter.clear();
                    self.filter_active = false;
                    self.rebuild_filter();
                }
                KeyCode::Enter => {
                    self.filter_active = false;
                }
                KeyCode::Backspace => {
                    self.filter.pop();
                    self.rebuild_filter();
                }
                KeyCode::Char(c) => {
                    self.filter.push(c);
                    self.rebuild_filter();
                }
                _ => {}
            }
            return false;
        }

        match key.code {
            KeyCode::Char('q') => return true,
            KeyCode::Char('/') => {
                self.filter_active = true;
            }
            KeyCode::Char('s') => {
                self.sort_mode = match self.sort_mode {
                    SortMode::Alphabetical => SortMode::Hz,
                    SortMode::Hz => SortMode::Count,
                    SortMode::Count => SortMode::Alphabetical,
                };
                self.rebuild_filter();
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if self.selected > 0 {
                    self.selected -= 1;
                    self.detail_scroll = 0;
                }
                self.table_state.select(Some(self.selected));
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if !self.filtered_indices.is_empty()
                    && self.selected < self.filtered_indices.len() - 1
                {
                    self.selected += 1;
                    self.detail_scroll = 0;
                }
                self.table_state.select(Some(self.selected));
            }
            KeyCode::Esc => {
                if !self.filter.is_empty() {
                    self.filter.clear();
                    self.rebuild_filter();
                }
            }
            KeyCode::PageDown | KeyCode::Char('d') => {
                self.detail_scroll = self.detail_scroll.saturating_add(10);
            }
            KeyCode::PageUp | KeyCode::Char('u') => {
                self.detail_scroll = self.detail_scroll.saturating_sub(10);
            }
            _ => {}
        }
        false
    }

    fn rebuild_filter(&mut self) {
        let filter_upper = self.filter.to_uppercase();
        self.filtered_indices = self
            .entries
            .iter()
            .enumerate()
            .filter(|(_, e)| {
                if self.filter.is_empty() {
                    true
                } else {
                    e.name.to_uppercase().contains(&filter_upper)
                        || format!("{}:{}", e.sys_id, e.comp_id).contains(&self.filter)
                }
            })
            .map(|(i, _)| i)
            .collect();

        match self.sort_mode {
            SortMode::Alphabetical => {
                self.filtered_indices
                    .sort_by(|&a, &b| self.entries[a].name.cmp(&self.entries[b].name));
            }
            SortMode::Hz => {
                self.filtered_indices.sort_by(|&a, &b| {
                    self.entries[b]
                        .hz
                        .partial_cmp(&self.entries[a].hz)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
            }
            SortMode::Count => {
                self.filtered_indices
                    .sort_by(|&a, &b| self.entries[b].count.cmp(&self.entries[a].count));
            }
        }

        if self.selected >= self.filtered_indices.len() {
            self.selected = self.filtered_indices.len().saturating_sub(1);
        }
        if self.filtered_indices.is_empty() {
            self.table_state.select(None);
        } else {
            self.table_state.select(Some(self.selected));
        }
    }

    pub fn selected_entry(&self) -> Option<&MessageEntry> {
        self.filtered_indices
            .get(self.selected)
            .and_then(|&idx| self.entries.get(idx))
    }

    pub fn sort_label(&self) -> &str {
        match self.sort_mode {
            SortMode::Alphabetical => "A-Z",
            SortMode::Hz => "Hz",
            SortMode::Count => "Cnt",
        }
    }
}
