use crossterm::event::{self, Event, KeyCode, KeyEvent};
use mavlink::common::MavMessage;
use mavlink::MavConnection;
use ratatui::widgets::{Table, TableState};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph},
    Terminal,
};
use std::io::{Error, Stdout};
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use crate::app::{FileMessages, Logger, MavlinkSender};

pub struct SenderApp {
    file_messages: FileMessages,
    mavlink_sender: Option<MavlinkSender>,
    logger: Logger,
    current_process_stop_signal: Option<Arc<AtomicBool>>,
    input_address: String,
    input_file: String,
    input_heartbeat_id: String,
    input_system_id_override: String,
    input_component_id_override: String,
    active_input: InputField,
    selected_file_message: Option<String>,
}

#[derive(PartialEq)]
enum InputField {
    Address,
    File,
    HeartbeatId,
    SystemId,
    ComponentId,
}

impl SenderApp {
    pub fn new() -> Self {
        let messages = FileMessages::new();
        let logs = Logger::new();

        SenderApp {
            file_messages: messages,
            logger: logs,
            mavlink_sender: None,
            current_process_stop_signal: None,
            input_address: "udpin:0.0.0.0:14550".to_string(),
            input_file: "output.txt".to_string(),
            input_heartbeat_id: String::new(),
            input_system_id_override: String::new(),
            input_component_id_override: String::new(),
            active_input: InputField::Address,
            selected_file_message: None,
        }
    }

    pub fn run(&mut self, terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<(), Error> {
        loop {
            terminal.draw(|f| self.draw_ui(f))?;

            if event::poll(Duration::from_millis(100))? {
                if let Event::Key(key) = event::read()? {
                    if self.current_process_stop_signal.is_none() {
                        if self.handle_key_event_idle(key) {
                            return Ok(());
                        }
                    } else {
                        if self.handle_key_event_running(key) {
                            return Ok(());
                        }
                    }
                }
            }
        }
    }
}

/// Handle key events
impl SenderApp {
    fn handle_key_event_idle(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Char('q') => return true,
            KeyCode::Char(c) => {
                self.handle_char_input(c);
            }
            KeyCode::Backspace => {
                self.handle_backspace_key();
            }
            KeyCode::Enter => {
                self.handle_enter_key_idle();
            }
            KeyCode::Tab => {
                self.active_input = match self.active_input {
                    InputField::Address => InputField::File,
                    InputField::File => InputField::HeartbeatId,
                    InputField::HeartbeatId => InputField::SystemId,
                    InputField::SystemId => InputField::ComponentId,
                    InputField::ComponentId => InputField::Address,
                };
            }
            KeyCode::Esc => self.stop_if_process_running(),
            _ => {}
        }
        return false;
    }

    fn handle_key_event_running(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Char('q') => return true,
            KeyCode::Esc => self.stop_if_process_running(),
            KeyCode::Right => {
                self.file_messages.switch_selected_table();
            }
            KeyCode::Left => {
                self.file_messages.switch_selected_table();
            }
            KeyCode::Down => {
                self.file_messages.key_down();
            }
            KeyCode::Up => {
                self.file_messages.key_up();
            }
            KeyCode::Enter => {
                self.handle_enter_key_running();
            }
            _ => {}
        }
        return false;
    }

    fn handle_enter_key_running(&mut self) {
        let Some((system_id, component_id, _)) = self.file_messages.get_selected_key() else {
            self.logger.log_info("No selected key");
            return;
        };

        let Some(message) = self.file_messages.get_selected_message() else {
            self.logger.log_info("No selected message");
            return;
        };
        let Some(mavlink_sender) = self.mavlink_sender.clone() else {
            self.logger.log_error("No sender");
            return;
        };

        let message = (system_id, component_id, message);
        mavlink_sender.send(message);
    }

    fn handle_enter_key_idle(&mut self) {
        let address = self.input_address.clone();
        if !validate_connection_address_input(&address) {
            self.logger.log_error("Invalid connection address");
            return;
        }

        if !validate_file_input(&self.input_file) {
            self.logger.log_error("Invalid or no input file");
            return;
        };

        let heartbeat_id = match self.input_heartbeat_id.parse::<u8>() {
            Ok(id) => Some(id),
            Err(_) => {
                self.logger.log_info("Invalid or no heartbeat ID");
                None
            }
        };
        let system_id_override = match self.input_system_id_override.parse::<u8>() {
            Ok(id) => Some(id),
            Err(_) => {
                self.logger.log_info("Invalid or no system ID filter");
                None
            }
        };
        let component_id_override = match self.input_component_id_override.parse::<u8>() {
            Ok(id) => Some(id),
            Err(_) => {
                self.logger.log_info("Invalid or no component ID filter");
                None
            }
        };
        let connection = match std::panic::catch_unwind(|| mavlink::connect::<MavMessage>(&address))
        {
            Ok(Ok(connection)) => {
                self.logger.log_info(&format!("Connected to {}", address));
                connection
            }
            Ok(Err(e)) => {
                self.logger
                    .log_error(&format!("Failed to connect to {address}: {e}"));

                return;
            }
            Err(_) => {
                self.logger
                    .log_error(&format!("Panic occurred while connecting to {address}"));
                return;
            }
        };
        let connection = Arc::new(Mutex::new(connection));
        self.stop_if_process_running();

        let stop_signal = Arc::new(AtomicBool::new(false));
        self.current_process_stop_signal = Some(stop_signal.clone());

        if let Some(heartbeat_id) = heartbeat_id {
            self.start_heartbeat_sender(
                connection.clone(),
                heartbeat_id,
                component_id_override.unwrap_or_default(),
                stop_signal.clone(),
            );
        }

        self.file_messages.read_file(&self.input_file);

        self.mavlink_sender = Some(MavlinkSender::new(
            connection.clone(),
            self.logger.clone(),
            component_id_override,
            system_id_override,
            stop_signal.clone(),
        ));
    }

    fn handle_backspace_key(&mut self) {
        match self.active_input {
            InputField::Address => {
                self.input_address.pop();
            }
            InputField::File => {
                self.input_file.pop();
            }
            InputField::HeartbeatId => {
                self.input_heartbeat_id.pop();
            }
            InputField::SystemId => {
                self.input_system_id_override.pop();
            }
            InputField::ComponentId => {
                self.input_component_id_override.pop();
            }
        }
    }

    fn handle_char_input(&mut self, c: char) {
        match self.active_input {
            InputField::Address => {
                self.input_address.push(c);
            }
            InputField::File => {
                self.input_file.push(c);
            }
            InputField::HeartbeatId => {
                self.input_heartbeat_id.push(c);
            }
            InputField::SystemId => {
                self.input_system_id_override.push(c);
            }
            InputField::ComponentId => {
                self.input_component_id_override.push(c);
            }
        }
    }

    fn stop_if_process_running(&mut self) {
        if let Some(stop_signal) = self.current_process_stop_signal.clone() {
            self.logger.log_info("Stopping current process");
            stop_signal.store(true, std::sync::atomic::Ordering::Relaxed);
            self.mavlink_sender = None;
            // small sleep to allow listener and sender to stop
            thread::sleep(Duration::from_millis(100));
            self.logger.log_info("Clearing messages");
            self.file_messages.clear_messages();
            self.current_process_stop_signal = None;
        }
    }

    fn start_heartbeat_sender(
        &mut self,
        connection: Arc<Mutex<Box<dyn MavConnection<MavMessage> + Send + Sync>>>,
        system_id: u8,
        component_id: u8,
        stop_signal: Arc<AtomicBool>,
    ) {
        let sender = MavlinkSender::new(
            connection.clone(),
            self.logger.clone(),
            Some(component_id),
            Some(system_id),
            stop_signal,
        );
        sender.start_heartbeat_loop();
    }
}

impl SenderApp {
    fn draw_ui(&mut self, f: &mut ratatui::Frame) {
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
            .constraints(
                [
                    Constraint::Percentage(33),
                    Constraint::Percentage(33),
                    Constraint::Percentage(34),
                ]
                .as_ref(),
            )
            .split(chunks[1]);
        let bottom_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(70), Constraint::Percentage(30)].as_ref())
            .split(chunks[2]);

        f.render_widget(self.get_input_address_paragraph(), top_chunks[0]);
        f.render_widget(self.get_input_file_paragraph(), top_chunks[1]);
        f.render_widget(self.get_input_heartbeat_id_paragraph(), top_chunks[2]);
        f.render_widget(self.get_input_system_id_paragraph(), top_chunks[3]);
        f.render_widget(self.get_input_component_id_paragraph(), top_chunks[4]);

        let full_messages_table = self.get_full_messages_table();
        let mut state = self.file_messages.full_messages_state();
        f.render_stateful_widget(full_messages_table, middle_chunks[0], &mut state);

        let selected_key_messages_table = self.get_selected_messages_table();
        let mut state = self.file_messages.selected_messages_state();
        f.render_stateful_widget(selected_key_messages_table, middle_chunks[1], &mut state);

        let selected_message_paragraph = self.get_selected_message_paragraph();
        f.render_widget(selected_message_paragraph, middle_chunks[2]);

        let logs_table = self.get_logs_table();
        let mut logs_state = TableState::default();
        f.render_stateful_widget(logs_table, bottom_chunks[0], &mut logs_state);

        let cheatsheet = self.get_cheatsheet_paragraph();
        f.render_widget(cheatsheet, bottom_chunks[1]);
    }

    pub fn get_full_messages_table(&self) -> Table {
        self.file_messages
            .to_tui_table(self.current_process_stop_signal.is_some())
    }

    pub fn get_selected_messages_table(&self) -> Table {
        self.file_messages
            .to_tui_table_selected_key(self.current_process_stop_signal.is_some())
    }

    pub fn get_selected_message_paragraph(&self) -> Paragraph {
        let (sys_id, comp_id, message_type) =
            self.file_messages
                .get_selected_key()
                .unwrap_or((0, 0, "".to_owned()));

        let selected_message_json = self.selected_file_message.clone().unwrap_or_else(|| {
            self.file_messages
                .get_selected_message_pretty()
                .unwrap_or("No selected message".to_string())
        });
        let selected_message_json = format!(
            "System ID: {}\nComponent ID: {}\nType: {}\n{}\n",
            sys_id, comp_id, message_type, selected_message_json
        );

        Paragraph::new(selected_message_json)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Selected Message"),
            )
            .style(
                Style::default().fg(if self.current_process_stop_signal.is_some() {
                    Color::LightBlue
                } else {
                    Color::Gray
                }),
            )
    }

    pub fn get_logs_table(&self) -> Table {
        self.logger.to_tui_table()
    }

    pub fn get_cheatsheet_paragraph(&self) -> Paragraph {
        Paragraph::new(
            "q: Quit\n\
            Enter: Start connection or send message\n\
            Tab: Switch Input\n\
            Up/Down/Right/Left: Navigate Messages\n\
            Esc: Stop Listener\n\
            Allowed connection address formats:udpin, udpout, tcpin, tcpout\n\
            Allowed input file formats: *.txt\n\
            Heartbeat ID: send heartbeat with id (0-255)\n\
            Sys/Comp ID: overrides for message sending (0-255)\n\
            ",
        )
        .block(Block::default().borders(Borders::ALL).title("Cheatsheet"))
        .style(Style::default().fg(Color::White))
    }

    pub fn get_input_address_paragraph(&self) -> Paragraph {
        Paragraph::new(self.input_address.clone())
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Connection Address"),
            )
            .style(
                Style::default().fg(if self.current_process_stop_signal.is_some() {
                    Color::Gray
                } else if self.active_input == InputField::Address {
                    if validate_connection_address_input(&self.input_address) {
                        Color::Green
                    } else {
                        Color::Red
                    }
                } else {
                    Color::White
                }),
            )
    }

    pub fn get_input_file_paragraph(&self) -> Paragraph {
        Paragraph::new(self.input_file.clone())
            .block(Block::default().borders(Borders::ALL).title("Input file"))
            .style(
                Style::default().fg(if self.current_process_stop_signal.is_some() {
                    Color::Gray
                } else if self.active_input == InputField::File {
                    if self.input_file.is_empty() {
                        Color::Blue
                    } else if validate_file_input(&self.input_file) {
                        Color::Green
                    } else {
                        Color::Red
                    }
                } else {
                    Color::White
                }),
            )
    }

    pub fn get_input_heartbeat_id_paragraph(&self) -> Paragraph {
        Paragraph::new(self.input_heartbeat_id.clone())
            .block(Block::default().borders(Borders::ALL).title("Heartbeat ID"))
            .style(
                Style::default().fg(if self.current_process_stop_signal.is_some() {
                    Color::Gray
                } else if self.active_input == InputField::HeartbeatId {
                    if self.input_heartbeat_id.is_empty() {
                        Color::Blue
                    } else if validate_u8_input(&self.input_heartbeat_id) {
                        Color::Green
                    } else {
                        Color::Red
                    }
                } else {
                    Color::White
                }),
            )
    }

    pub fn get_input_system_id_paragraph(&self) -> Paragraph {
        Paragraph::new(self.input_system_id_override.clone())
            .block(Block::default().borders(Borders::ALL).title("Sys ID"))
            .style(
                Style::default().fg(if self.current_process_stop_signal.is_some() {
                    Color::Gray
                } else if self.active_input == InputField::SystemId {
                    if self.input_system_id_override.is_empty() {
                        Color::Blue
                    } else if validate_u8_input(&self.input_system_id_override) {
                        Color::Green
                    } else {
                        Color::Red
                    }
                } else {
                    Color::White
                }),
            )
    }

    pub fn get_input_component_id_paragraph(&self) -> Paragraph {
        Paragraph::new(self.input_component_id_override.clone())
            .block(Block::default().borders(Borders::ALL).title("Comp ID"))
            .style(
                Style::default().fg(if self.current_process_stop_signal.is_some() {
                    Color::Gray
                } else if self.active_input == InputField::ComponentId {
                    if self.input_component_id_override.is_empty() {
                        Color::Blue
                    } else if validate_u8_input(&self.input_component_id_override) {
                        Color::Green
                    } else {
                        Color::Red
                    }
                } else {
                    Color::White
                }),
            )
    }
}

fn validate_u8_input(input: &str) -> bool {
    input.parse::<u8>().is_ok()
}

fn validate_file_input(input: &str) -> bool {
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
