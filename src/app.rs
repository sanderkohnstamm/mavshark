mod listener;
mod logs;
mod messages;
mod rolling_window;

use crossterm::event::{self, Event, KeyCode};
use listener::Listener;
use logs::Logs;
use mavlink::common::MavMessage;
use messages::Messages;
use std::io;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use tui::widgets::TableState;
use tui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph},
    Terminal,
};

pub enum LogLevel {
    Info,
    Error,
}

pub struct App {
    messages: Messages,
    logs: Logs,
}

impl App {
    pub fn new() -> Self {
        let messages = Messages::new();
        let logs = Logs::new();

        App { messages, logs }
    }

    pub fn run(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    ) -> Result<(), io::Error> {
        let mut input_address = "udpin:0.0.0.0:14550".to_string();
        let mut input_output_file = String::new();
        let mut input_heartbeat_id = String::new();
        let mut filter_system_id = String::new();
        let mut active_input = 1; // 1 for input_address, 2 for input_output_file, 3 for input_heartbeat_id, 4 for include_system_id

        loop {
            terminal.draw(|f| {
                let size = f.size();
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
                            Constraint::Percentage(40),
                            Constraint::Percentage(40),
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

                let input_address_paragraph = Paragraph::new(input_address.as_ref())
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .title("Connection Address"),
                    )
                    .style(Style::default().fg(if active_input == 1 {
                        Color::Yellow
                    } else {
                        Color::White
                    }));
                f.render_widget(input_address_paragraph, top_chunks[0]);

                let input_output_file_paragraph = Paragraph::new(input_output_file.as_ref())
                    .block(Block::default().borders(Borders::ALL).title("Output file"))
                    .style(Style::default().fg(if active_input == 2 {
                        Color::Yellow
                    } else {
                        Color::White
                    }));
                f.render_widget(input_output_file_paragraph, top_chunks[1]);

                let input_heartbeat_id_paragraph = Paragraph::new(input_heartbeat_id.as_ref())
                    .block(Block::default().borders(Borders::ALL).title("Heartbeat ID"))
                    .style(Style::default().fg(if active_input == 3 {
                        Color::Yellow
                    } else {
                        Color::White
                    }));
                f.render_widget(input_heartbeat_id_paragraph, top_chunks[2]);

                let include_system_id_paragraph = Paragraph::new(filter_system_id.as_ref())
                    .block(Block::default().borders(Borders::ALL).title("Sys ID"))
                    .style(Style::default().fg(if active_input == 4 {
                        Color::Yellow
                    } else {
                        Color::White
                    }));
                f.render_widget(include_system_id_paragraph, top_chunks[3]);

                let table = self.messages.to_tui_table();
                let mut state = self.messages.state();
                f.render_stateful_widget(table, middle_chunks[0], &mut state);

                let selected_message_json = self
                    .messages
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

                let error_table = self.logs.to_tui_table();
                let mut error_state = TableState::default();
                f.render_stateful_widget(error_table, chunks[2], &mut error_state);
            })?;

            if event::poll(Duration::from_millis(100))? {
                if let Event::Key(key) = event::read()? {
                    match key.code {
                        KeyCode::Char('q') => break,
                        KeyCode::Char(c) => match active_input {
                            1 => input_address.push(c),
                            2 => input_output_file.push(c),
                            3 => input_heartbeat_id.push(c),
                            4 => filter_system_id.push(c),
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
                                filter_system_id.pop();
                            }
                            _ => {}
                        },
                        KeyCode::Enter => {
                            let address = input_address.clone();
                            let output_file = match input_output_file.clone() {
                                s if s.is_empty() => None,
                                s => Some(s),
                            };
                            let heartbeat_id = match input_heartbeat_id.parse::<u8>() {
                                Ok(id) => Some(id),
                                Err(_) => None,
                            };
                            let filter_system_id = match filter_system_id.parse::<u8>() {
                                Ok(id) => Some(id),
                                Err(_) => None,
                            };
                            self.start_listener(
                                address,
                                output_file,
                                heartbeat_id,
                                filter_system_id,
                            );
                        }
                        KeyCode::Tab => {
                            active_input = if active_input == 4 {
                                1
                            } else {
                                active_input + 1
                            };
                        }
                        KeyCode::Down => self.messages.select_down(),
                        KeyCode::Up => self.messages.select_up(),
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
        output_file: Option<String>,
        heartbeat_id: Option<u8>,
        system_id_filter: Option<u8>,
    ) {
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
        );

        thread::spawn(move || {
            listener.listen();
        });
    }
}
