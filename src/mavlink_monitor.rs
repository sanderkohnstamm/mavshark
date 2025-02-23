mod rolling_window;
mod widget_errors;
mod widget_frequencies;

use crate::mavlink_listener::MavlinkListener;

use crossterm::event::{self, Event, KeyCode};
use mavlink::common::MavMessage;
use rolling_window::RollingWindow;
use std::collections::HashMap;
use std::io;
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::Duration;
use std::time::Instant;
use tui::widgets::TableState;
use tui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph},
    Terminal,
};
use widget_errors::WidgetErrors;
use widget_frequencies::WidgetFrequencies;

pub struct MavlinkMonitor {
    hz_window_size: usize,
    message_tx: mpsc::Sender<(mavlink::MavHeader, MavMessage)>,
    error_tx: mpsc::Sender<(Instant, String)>,
    message_counts: Arc<Mutex<HashMap<(u8, u8, String), RollingWindow>>>,
    last_messages: Arc<Mutex<HashMap<(u8, u8, String), String>>>,
    error_messages: Arc<Mutex<HashMap<String, Instant>>>,
}

impl MavlinkMonitor {
    pub fn new() -> Self {
        let (message_tx, message_rx) = mpsc::channel();
        let (error_tx, error_rx) = mpsc::channel();
        let message_counts = Arc::new(Mutex::new(HashMap::new()));
        let last_messages = Arc::new(Mutex::new(HashMap::new()));
        let error_messages = Arc::new(Mutex::new(HashMap::new()));

        let monitor = MavlinkMonitor {
            hz_window_size: 10,
            message_tx,
            error_tx,
            message_counts: message_counts.clone(),
            last_messages: last_messages.clone(),
            error_messages: error_messages.clone(),
        };

        monitor.listen_to_channels(
            message_counts,
            last_messages,
            error_messages,
            message_rx,
            error_rx,
        );
        monitor
    }

    pub fn run(
        &self,
        terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    ) -> Result<(), io::Error> {
        let mut input_address = String::new();
        let mut input_output_file = String::new();
        let mut active_input = 1; // 1 for input_address, 2 for input_output_file
        let mut widget_frequencies =
            WidgetFrequencies::new_with(self.message_counts.clone(), self.last_messages.clone());
        let widget_errors = WidgetErrors::new_with(self.error_messages.clone());

        loop {
            terminal.draw(|f| {
                let size = f.size();
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints(
                        [
                            Constraint::Percentage(10),
                            Constraint::Percentage(75),
                            Constraint::Percentage(15),
                        ]
                        .as_ref(),
                    )
                    .split(size);

                let top_chunks = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([Constraint::Percentage(70), Constraint::Percentage(30)].as_ref())
                    .split(chunks[0]);
                let middle_chunks = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([Constraint::Percentage(70), Constraint::Percentage(30)].as_ref())
                    .split(chunks[1]);

                let input_address_paragraph =
                    Paragraph::new(input_address.as_ref())
                        .block(Block::default().borders(Borders::ALL).title(
                            "Enter Connection Address (e.g. udpin:0.0.0.0:14550) or q to quit",
                        ))
                        .style(Style::default().fg(if active_input == 1 {
                            Color::Yellow
                        } else {
                            Color::White
                        }));
                f.render_widget(input_address_paragraph, top_chunks[0]);

                let input_output_file_paragraph = Paragraph::new(input_output_file.as_ref())
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .title("Enter Optional Output File"),
                    )
                    .style(Style::default().fg(if active_input == 2 {
                        Color::Yellow
                    } else {
                        Color::White
                    }));
                f.render_widget(input_output_file_paragraph, top_chunks[1]);

                let table = widget_frequencies.to_tui_table();
                let mut state = widget_frequencies.state.clone();
                f.render_stateful_widget(table, middle_chunks[0], &mut state);

                let selected_message_json = widget_frequencies
                    .get_selected_message_string()
                    .unwrap_or("No selected message".to_string());
                let selected_message_paragraph = Paragraph::new(selected_message_json.as_ref())
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .title("Selected Message"),
                    )
                    .style(Style::default().fg(Color::White));
                f.render_widget(selected_message_paragraph, middle_chunks[1]);

                let error_table = widget_errors.to_tui_table();
                let mut error_state = TableState::default();
                f.render_stateful_widget(error_table, chunks[2], &mut error_state);
            })?;

            if event::poll(Duration::from_millis(100))? {
                if let Event::Key(key) = event::read()? {
                    match key.code {
                        KeyCode::Char('q') => break,
                        KeyCode::Char(c) => {
                            if active_input == 1 {
                                input_address.push(c);
                            } else {
                                input_output_file.push(c);
                            }
                        }
                        KeyCode::Backspace => {
                            if active_input == 1 {
                                input_address.pop();
                            } else {
                                input_output_file.pop();
                            }
                        }
                        KeyCode::Enter => {
                            let address = input_address.clone();
                            // Output file is option if empty
                            let output_file = match input_output_file.clone() {
                                s if s.is_empty() => None,
                                s => Some(s),
                            };
                            self.start_listener(address, widget_errors.get_errors(), output_file);
                        }
                        KeyCode::Tab => {
                            active_input = if active_input == 1 { 2 } else { 1 };
                        }
                        KeyCode::Down => widget_frequencies.select_down(),
                        KeyCode::Up => widget_frequencies.select_up(),
                        _ => {}
                    }
                }
            }
        }

        Ok(())
    }

    fn start_listener(
        &self,
        address: String,
        errors: Arc<Mutex<HashMap<String, Instant>>>,
        output_file: Option<String>,
    ) {
        let connection = match std::panic::catch_unwind(|| mavlink::connect::<MavMessage>(&address))
        {
            Ok(Ok(connection)) => connection,
            Ok(Err(e)) => {
                let mut errors = errors.lock().unwrap();
                errors.insert(e.to_string(), Instant::now());
                return;
            }
            Err(_) => {
                let mut errors = errors.lock().unwrap();
                errors.insert(
                    "Panic occurred while trying to connect".to_string(),
                    Instant::now(),
                );
                return;
            }
        };

        let connection = Arc::new(Mutex::new(connection));

        let listener = MavlinkListener::new(
            None,
            output_file,
            self.message_tx.clone(),
            self.error_tx.clone(),
        );

        thread::spawn(move || {
            listener.listen(connection);
        });
    }

    fn listen_to_channels(
        &self,
        message_counts: Arc<Mutex<HashMap<(u8, u8, String), RollingWindow>>>,
        last_messages: Arc<Mutex<HashMap<(u8, u8, String), String>>>,
        error_messages: Arc<Mutex<HashMap<String, Instant>>>,
        message_rx: mpsc::Receiver<(mavlink::MavHeader, MavMessage)>,
        error_rx: mpsc::Receiver<(Instant, String)>,
    ) {
        let hz_window_size = self.hz_window_size;

        // Get messages
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
                    .or_insert_with(|| {
                        RollingWindow::new(Duration::from_secs(hz_window_size as u64))
                    })
                    .add(timestamp);

                last_messages.lock().unwrap().insert(
                    (header.system_id, header.component_id, message_type),
                    message_json,
                );
            }
        });

        // Get errors
        thread::spawn(move || {
            while let Ok((timestamp, error_message)) = error_rx.recv() {
                let mut errors = error_messages.lock().unwrap();
                errors.insert(error_message, timestamp);
            }
        });
    }
}
