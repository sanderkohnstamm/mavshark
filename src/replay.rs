use std::io;
use std::path::Path;

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::prelude::*;
use ratatui::widgets::TableState;

use crate::record::{load_recording, RecordedMessage};

pub struct ReplayApp {
    pub file_path: String,
    pub messages: Vec<RecordedMessage>,
    pub selected: usize,
    pub detail_scroll: usize,
    pub filter: String,
    pub filter_active: bool,
    pub filtered_indices: Vec<usize>,
    pub table_state: TableState,
}

impl ReplayApp {
    pub fn new(file_path: String, messages: Vec<RecordedMessage>) -> Self {
        let filtered_indices: Vec<usize> = (0..messages.len()).collect();
        let mut table_state = TableState::default();
        if !filtered_indices.is_empty() {
            table_state.select(Some(0));
        }
        Self {
            file_path,
            messages,
            selected: 0,
            detail_scroll: 0,
            filter: String::new(),
            filter_active: false,
            filtered_indices,
            table_state,
        }
    }

    pub fn on_key(&mut self, key: crossterm::event::KeyEvent) -> bool {
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
            KeyCode::Char('g') => {
                self.selected = 0;
                self.detail_scroll = 0;
                self.table_state.select(Some(0));
            }
            KeyCode::Char('G') => {
                if !self.filtered_indices.is_empty() {
                    self.selected = self.filtered_indices.len() - 1;
                    self.detail_scroll = 0;
                    self.table_state.select(Some(self.selected));
                }
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
            .messages
            .iter()
            .enumerate()
            .filter(|(_, m)| {
                if self.filter.is_empty() {
                    true
                } else {
                    m.message_name.to_uppercase().contains(&filter_upper)
                        || format!("{}:{}", m.header.system_id, m.header.component_id)
                            .contains(&self.filter)
                }
            })
            .map(|(i, _)| i)
            .collect();

        if self.selected >= self.filtered_indices.len() {
            self.selected = self.filtered_indices.len().saturating_sub(1);
        }
        if self.filtered_indices.is_empty() {
            self.table_state.select(None);
        } else {
            self.table_state.select(Some(self.selected));
        }
    }

    pub fn selected_message(&self) -> Option<&RecordedMessage> {
        self.filtered_indices
            .get(self.selected)
            .and_then(|&idx| self.messages.get(idx))
    }
}

pub fn run_replay(file: &str) -> Result<()> {
    let path = Path::new(file);
    let messages = load_recording(path)?;

    if messages.is_empty() {
        anyhow::bail!("No messages found in {}", file);
    }

    let mut app = ReplayApp::new(file.to_string(), messages);

    // Restore terminal on panic
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
        original_hook(info);
    }));

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    loop {
        terminal.draw(|f| crate::ui::draw_replay(f, &mut app))?;

        if let Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Press && app.on_key(key) {
                break;
            }
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}
