mod listener;
mod logs;
mod messages;
mod rolling_window;

use crossterm::event::{self, Event, KeyCode};
use listener::Listener;
use logs::Logs;
use mavlink::common::MavMessage;
use messages::Messages;
use ratatui::symbols;
use ratatui::widgets::{Axis, Chart, Dataset, TableState};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph},
    Terminal,
};
use std::io;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

pub enum LogLevel {
    Info,
    Error,
}

pub struct App {
    messages: Messages,
    logs: Logs,
    current_listener_stop_signal: Option<Arc<AtomicBool>>,
}

impl App {
    pub fn new() -> Self {
        let messages = Messages::new();
        let logs = Logs::new();

        App {
            messages,
            logs,
            current_listener_stop_signal: None,
        }
    }

    pub fn run(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    ) -> Result<(), io::Error> {
        let mut input_address = "udpin:0.0.0.0:14550".to_string();
        let mut input_output_file = "output.txt".to_string();
        let mut input_heartbeat_id = String::new();
        let mut input_system_id_filter = String::new();
        let mut input_component_id_filter = String::new();
        let mut active_input = 1; // 1 for input_address, 2 for input_output_file, 3 for input_heartbeat_id, 4 for include_system_id

        loop {
            terminal.draw(|f| {
                let size = f.area();
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints(
                        [
                            Constraint::Length(3), // Adjusted to ensure one line height
                            Constraint::Percentage(75),
                            Constraint::Percentage(15),
                        ]
                        .as_ref(),
                    )
                    .split(size);

                let top_chunks = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints(
                        [
                            Constraint::Percentage(35),
                            Constraint::Percentage(35),
                            Constraint::Percentage(10),
                            Constraint::Percentage(10),
                            Constraint::Percentage(10),
                        ]
                        .as_ref(),
                    )
                    .split(chunks[0]);
                let middle_chunks = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([Constraint::Percentage(70), Constraint::Percentage(30)].as_ref())
                    .split(chunks[1]);
                let bottom_chunks = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([Constraint::Percentage(70), Constraint::Percentage(30)].as_ref())
                    .split(chunks[2]);

                let selected_message_chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Percentage(70), Constraint::Percentage(30)].as_ref())
                    .split(middle_chunks[1]);

                let input_address_paragraph = Paragraph::new(input_address.clone())
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .title("Connection Address"),
                    )
                    .style(
                        Style::default().fg(if self.current_listener_stop_signal.is_some() {
                            Color::Gray
                        } else if active_input == 1 {
                            if validate_connection_address_input(&input_address) {
                                Color::Green
                            } else {
                                Color::Red
                            }
                        } else {
                            Color::White
                        }),
                    );
                f.render_widget(input_address_paragraph, top_chunks[0]);

                let input_output_file_paragraph = Paragraph::new(input_output_file.clone())
                    .block(Block::default().borders(Borders::ALL).title("Output file"))
                    .style(
                        Style::default().fg(if self.current_listener_stop_signal.is_some() {
                            Color::Gray
                        } else if active_input == 2 {
                            if input_output_file.is_empty() {
                                Color::Blue
                            } else if validate_output_file_input(&input_output_file) {
                                Color::Green
                            } else {
                                Color::Red
                            }
                        } else {
                            Color::White
                        }),
                    );
                f.render_widget(input_output_file_paragraph, top_chunks[1]);

                let input_heartbeat_id_paragraph = Paragraph::new(input_heartbeat_id.clone())
                    .block(Block::default().borders(Borders::ALL).title("Heartbeat ID"))
                    .style(
                        Style::default().fg(if self.current_listener_stop_signal.is_some() {
                            Color::Gray
                        } else if active_input == 3 {
                            if input_heartbeat_id.is_empty() {
                                Color::Blue
                            } else if validate_u8_input(&input_heartbeat_id) {
                                Color::Green
                            } else {
                                Color::Red
                            }
                        } else {
                            Color::White
                        }),
                    );
                f.render_widget(input_heartbeat_id_paragraph, top_chunks[2]);

                let include_system_id_paragraph = Paragraph::new(input_system_id_filter.clone())
                    .block(Block::default().borders(Borders::ALL).title("Sys ID"))
                    .style(
                        Style::default().fg(if self.current_listener_stop_signal.is_some() {
                            Color::Gray
                        } else if active_input == 4 {
                            if input_system_id_filter.is_empty() {
                                Color::Blue
                            } else if validate_u8_input(&input_system_id_filter) {
                                Color::Green
                            } else {
                                Color::Red
                            }
                        } else {
                            Color::White
                        }),
                    );
                f.render_widget(include_system_id_paragraph, top_chunks[3]);

                let include_component_id_paragraph =
                    Paragraph::new(input_component_id_filter.clone())
                        .block(Block::default().borders(Borders::ALL).title("Comp ID"))
                        .style(Style::default().fg(
                            if self.current_listener_stop_signal.is_some() {
                                Color::Gray
                            } else if active_input == 5 {
                                if input_component_id_filter.is_empty() {
                                    Color::Blue
                                } else if validate_u8_input(&input_component_id_filter) {
                                    Color::Green
                                } else {
                                    Color::Red
                                }
                            } else {
                                Color::White
                            },
                        ));
                f.render_widget(include_component_id_paragraph, top_chunks[4]);

                let table = self
                    .messages
                    .to_tui_table(self.current_listener_stop_signal.is_some());
                let mut state = self.messages.state();

                f.render_stateful_widget(table, middle_chunks[0], &mut state);

                let selected_message_json = self
                    .messages
                    .get_selected_message_string()
                    .unwrap_or("No selected message".to_string());
                let selected_message_paragraph = Paragraph::new(selected_message_json)
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .title("Selected Message"),
                    )
                    .style(
                        Style::default().fg(if self.current_listener_stop_signal.is_some() {
                            Color::LightBlue
                        } else {
                            Color::Gray
                        }),
                    );
                f.render_widget(selected_message_paragraph, selected_message_chunks[0]);

                let (hz_history, current_hz) = self
                    .messages
                    .get_selected_message_frequency()
                    .unwrap_or((vec![], 0.1));

                // Add a graph here
                let history_data: Vec<(f64, f64)> = hz_history
                    .iter()
                    .enumerate()
                    .map(|(i, &value)| (i as f64, value as f64))
                    .collect();

                let history_chart = Chart::new(vec![Dataset::default()
                    .marker(symbols::Marker::Dot)
                    .style(Style::default().fg(Color::Cyan))
                    .data(&history_data)])
                .block(Block::default().borders(Borders::ALL).title("History"))
                .style(
                    Style::default().fg(if self.current_listener_stop_signal.is_some() {
                        Color::LightBlue
                    } else {
                        Color::Gray
                    }),
                )
                .x_axis(
                    Axis::default()
                        .title("Time")
                        .bounds([0.0, history_data.len() as f64]),
                )
                .y_axis(
                    Axis::default()
                        .title("Frequency")
                        .bounds([current_hz * 0.9, current_hz * 1.1]),
                );

                f.render_widget(history_chart, selected_message_chunks[1]);

                let logs_table = self.logs.to_tui_table();
                let mut logs_state = TableState::default();
                f.render_stateful_widget(logs_table, bottom_chunks[0], &mut logs_state);

                let cheatsheet = Paragraph::new(
                    "q: Quit\n\
                    Enter: Start Listener\n\
                    Tab: Switch Input\n\
                    Up/Down: Navigate Messages\n\
                    Esc: Stop Listener\n\
                    Allowed connection address formats:udpin, udpout, tcpin, tcpout\n\
                    Allowed output file formats: *.txt\n\
                    Heartbeat ID: send heartbeat with id (0-255)\n\
                    Sys ID: filter messages by system id (0-255)\n\
                    Comp ID: filter messages by component id (0-255)
                    ",
                )
                .block(Block::default().borders(Borders::ALL).title("Cheatsheet"))
                .style(Style::default().fg(Color::White));
                f.render_widget(cheatsheet, bottom_chunks[1]);
            })?;

            if event::poll(Duration::from_millis(100))? {
                if let Event::Key(key) = event::read()? {
                    if self.current_listener_stop_signal.is_none() {
                        match key.code {
                            KeyCode::Char('q') => break,
                            KeyCode::Char(c) => match active_input {
                                1 => input_address.push(c),
                                2 => input_output_file.push(c),
                                3 => input_heartbeat_id.push(c),
                                4 => input_system_id_filter.push(c),
                                5 => input_component_id_filter.push(c),
                                _ => {}
                            },
                            KeyCode::Backspace => match active_input {
                                1 => {
                                    input_address.pop();
                                }
                                2 => {
                                    input_output_file.pop();
                                }
                                3 => {
                                    input_heartbeat_id.pop();
                                }
                                4 => {
                                    input_system_id_filter.pop();
                                }
                                5 => {
                                    input_component_id_filter.pop();
                                }
                                _ => {}
                            },
                            KeyCode::Enter => {
                                let address = input_address.clone();
                                if !validate_connection_address_input(&address) {
                                    self.logs.log_error("Invalid connection address");
                                    continue;
                                }

                                let output_file = match input_output_file.clone() {
                                    s if s.is_empty() => {
                                        self.logs.log_info("No output file specified");
                                        None
                                    }
                                    s => Some(s),
                                };

                                let heartbeat_id = match input_heartbeat_id.parse::<u8>() {
                                    Ok(id) => Some(id),
                                    Err(_) => {
                                        self.logs.log_info("Invalid or no heartbeat ID");
                                        None
                                    }
                                };
                                let system_id_filter = match input_system_id_filter.parse::<u8>() {
                                    Ok(id) => Some(id),
                                    Err(_) => {
                                        self.logs.log_info("Invalid or no system ID filter");
                                        None
                                    }
                                };
                                let component_id_filter =
                                    match input_component_id_filter.parse::<u8>() {
                                        Ok(id) => Some(id),
                                        Err(_) => {
                                            self.logs.log_info("Invalid or no component ID filter");
                                            None
                                        }
                                    };
                                self.start_listener(
                                    address,
                                    output_file,
                                    heartbeat_id,
                                    system_id_filter,
                                    component_id_filter,
                                );
                            }
                            KeyCode::Tab => {
                                active_input = if active_input == 5 {
                                    1
                                } else {
                                    active_input + 1
                                };
                            }
                            KeyCode::Down => self.messages.select_down(),
                            KeyCode::Up => self.messages.select_up(),
                            KeyCode::Esc => self.stop_if_listener_running(),
                            _ => {}
                        }
                    } else {
                        match key.code {
                            KeyCode::Char('q') => break,
                            KeyCode::Down => self.messages.select_down(),
                            KeyCode::Up => self.messages.select_up(),
                            KeyCode::Esc => self.stop_if_listener_running(),
                            _ => {}
                        }
                    }
                }
            }
        }

        Ok(())
    }

    fn stop_if_listener_running(&mut self) {
        if let Some(stop_signal) = self.current_listener_stop_signal.clone() {
            self.logs.log_info("Stopping listener");
            stop_signal.store(true, std::sync::atomic::Ordering::Relaxed);
            // small sleep to allow listener to stop
            thread::sleep(Duration::from_millis(100));
            self.logs.log_info("Clearing messages");
            self.messages.clear();
            self.current_listener_stop_signal = None;
        }
    }

    fn start_listener(
        &mut self,
        address: String,
        output_file: Option<String>,
        heartbeat_id: Option<u8>,
        system_id_filter: Option<u8>,
        component_id_filter: Option<u8>,
    ) {
        self.stop_if_listener_running();

        let connection = match std::panic::catch_unwind(|| mavlink::connect::<MavMessage>(&address))
        {
            Ok(Ok(connection)) => {
                self.logs.log_info(&format!("Connected to {}", address));
                connection
            }
            Ok(Err(e)) => {
                self.logs
                    .log_error(&format!("Failed to connect to {address}: {e}"));

                return;
            }
            Err(_) => {
                self.logs
                    .log_error(&format!("Panic occurred while connecting to {address}"));
                return;
            }
        };

        let connection = Arc::new(Mutex::new(connection));
        let listener = Listener::new(
            connection.clone(),
            output_file.clone(),
            self.messages.message_tx(),
            self.logs.logs_tx(),
            heartbeat_id,
            system_id_filter,
            component_id_filter,
        );
        let stop_signal = listener.get_stop_signal();
        self.current_listener_stop_signal = Some(stop_signal.clone());
        thread::spawn(move || {
            listener.listen();
        });
    }
}

fn validate_u8_input(input: &str) -> bool {
    input.parse::<u8>().is_ok()
}

fn validate_output_file_input(input: &str) -> bool {
    input.ends_with(".txt")
        && input
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == '.' || c == '/')
}

fn validate_connection_address_input(input: &str) -> bool {
    // Basic validation for MAVLink connection address (e.g., "udpin:0.0.0.0:14550")
    let parts: Vec<&str> = input.split(':').collect();
    if parts.len() != 3 {
        return false;
    }
    let protocol = parts[0];
    let ip = parts[1];
    let port = parts[2];

    if protocol != "udpin" && protocol != "udpout" && protocol != "tcpin" && protocol != "tcpout" {
        return false;
    }

    if !ip.parse::<std::net::Ipv4Addr>().is_ok() {
        return false;
    }

    if !port.parse::<u16>().is_ok() {
        return false;
    }

    true
}
