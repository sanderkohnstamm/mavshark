use crossterm::event::{self, Event, KeyCode, KeyEvent};
use mavlink::common::MavMessage;
use mavlink::MavConnection;
use ratatui::symbols;
use ratatui::widgets::{Axis, Chart, Dataset, Table, TableState};
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

use crate::app::utils::{
    validate_connection_address_input, validate_file_input, validate_u8_input, InputField,
    RECORDER_CHEATSHEET,
};
use crate::app::{IncomingMessages, Logger, MavlinkListener, MavlinkSender};

pub struct RecorderApp {
    messages: IncomingMessages,
    logger: Logger,
    current_process_stop_signal: Option<Arc<AtomicBool>>,
    input_address: String,
    input_output_file: String,
    input_heartbeat_id: String,
    input_system_id_filter: String,
    input_component_id_filter: String,
    active_input: InputField,
}

impl RecorderApp {
    pub fn new() -> Self {
        let messages = IncomingMessages::new();
        let logs = Logger::new();

        RecorderApp {
            messages,
            logger: logs,
            current_process_stop_signal: None,
            input_address: "udpin:0.0.0.0:14550".to_string(),
            input_output_file: "output.txt".to_string(),
            input_heartbeat_id: String::new(),
            input_system_id_filter: String::new(),
            input_component_id_filter: String::new(),
            active_input: InputField::Address,
        }
    }

    pub fn run(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    ) -> Result<(), io::Error> {
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
impl RecorderApp {
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
                self.handle_enter_key();
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
            KeyCode::Down => self.messages.select_down(),
            KeyCode::Up => self.messages.select_up(),
            KeyCode::Esc => self.stop_if_process_running(),
            _ => {}
        }
        return false;
    }

    fn handle_key_event_running(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Char('q') => return true,
            KeyCode::Down => self.messages.select_down(),
            KeyCode::Up => self.messages.select_up(),
            KeyCode::Esc => self.stop_if_process_running(),
            _ => {}
        }
        return false;
    }

    fn handle_enter_key(&mut self) {
        let address = self.input_address.clone();
        if !validate_connection_address_input(&address) {
            self.logger.log_error("Invalid connection address");
            return;
        }

        let output_file = match self.input_output_file.clone() {
            s if s.is_empty() => {
                self.logger.log_info("No output file specified");
                None
            }
            s => Some(s),
        };

        let heartbeat_id = match self.input_heartbeat_id.parse::<u8>() {
            Ok(id) => Some(id),
            Err(_) => {
                self.logger.log_info("Invalid or no heartbeat ID");
                None
            }
        };
        let system_id_filter = match self.input_system_id_filter.parse::<u8>() {
            Ok(id) => Some(id),
            Err(_) => {
                self.logger.log_info("Invalid or no system ID filter");
                None
            }
        };
        let component_id_filter = match self.input_component_id_filter.parse::<u8>() {
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
            self.start_heartbeat_sender(connection.clone(), heartbeat_id, 0, stop_signal.clone());
        }

        self.start_listener(
            connection,
            output_file,
            system_id_filter,
            component_id_filter,
            stop_signal,
        );
    }

    fn handle_backspace_key(&mut self) {
        match self.active_input {
            InputField::Address => {
                self.input_address.pop();
            }
            InputField::File => {
                self.input_output_file.pop();
            }
            InputField::HeartbeatId => {
                self.input_heartbeat_id.pop();
            }
            InputField::SystemId => {
                self.input_system_id_filter.pop();
            }
            InputField::ComponentId => {
                self.input_component_id_filter.pop();
            }
        }
    }

    fn handle_char_input(&mut self, c: char) {
        match self.active_input {
            InputField::Address => {
                self.input_address.push(c);
            }
            InputField::File => {
                self.input_output_file.push(c);
            }
            InputField::HeartbeatId => {
                self.input_heartbeat_id.push(c);
            }
            InputField::SystemId => {
                self.input_system_id_filter.push(c);
            }
            InputField::ComponentId => {
                self.input_component_id_filter.push(c);
            }
        }
    }

    fn stop_if_process_running(&mut self) {
        if let Some(stop_signal) = self.current_process_stop_signal.clone() {
            self.logger.log_info("Stopping current process");
            stop_signal.store(true, std::sync::atomic::Ordering::Relaxed);
            // small sleep to allow listener and sender to stop
            thread::sleep(Duration::from_millis(100));
            self.logger.log_info("Clearing messages");
            self.messages.clear();
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

    fn start_listener(
        &mut self,
        connection: Arc<Mutex<Box<dyn MavConnection<MavMessage> + Send + Sync>>>,
        output_file: Option<String>,
        system_id_filter: Option<u8>,
        component_id_filter: Option<u8>,
        stop_signal: Arc<AtomicBool>,
    ) {
        let listener = MavlinkListener::new(
            connection.clone(),
            output_file.clone(),
            self.messages.message_tx(),
            self.logger.clone(),
            system_id_filter,
            component_id_filter,
            stop_signal,
        );

        thread::spawn(move || {
            listener.record();
        });
    }
}

impl RecorderApp {
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

        f.render_widget(self.get_input_address_paragraph(), top_chunks[0]);
        f.render_widget(self.get_input_output_file_paragraph(), top_chunks[1]);
        f.render_widget(self.get_input_heartbeat_id_paragraph(), top_chunks[2]);
        f.render_widget(self.get_input_system_id_paragraph(), top_chunks[3]);
        f.render_widget(self.get_input_component_id_paragraph(), top_chunks[4]);

        let table = self.get_messages_table();
        let mut state = self.messages.state();
        f.render_stateful_widget(table, middle_chunks[0], &mut state);

        let selected_message_paragraph = self.get_selected_message_paragraph();
        f.render_widget(selected_message_paragraph, selected_message_chunks[0]);

        let data: Vec<(f64, f64)> = self
            .messages
            .get_selected_message_hz_history()
            .into_iter()
            .enumerate()
            .map(|(i, v)| (i as f64, v))
            .collect();
        let history_chart = self.get_history_chart(&data);
        f.render_widget(history_chart, selected_message_chunks[1]);

        let logs_table = self.get_logs_table();
        let mut logs_state = TableState::default();
        f.render_stateful_widget(logs_table, bottom_chunks[0], &mut logs_state);

        let cheatsheet = self.get_cheatsheet_paragraph();
        f.render_widget(cheatsheet, bottom_chunks[1]);
    }

    pub fn get_messages_table(&self) -> Table {
        self.messages
            .to_tui_table(self.current_process_stop_signal.is_some(), false)
    }

    pub fn get_selected_message_paragraph(&self) -> Paragraph {
        let selected_message_json = self
            .messages
            .get_selected_message_string()
            .unwrap_or("No selected message".to_string());
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

    pub fn get_history_chart<'a>(&self, data: &'a [(f64, f64)]) -> Chart<'a> {
        let dataset = Dataset::default()
            .name("Hz History")
            .marker(symbols::Marker::Dot)
            .style(Style::default().fg(Color::Cyan))
            .data(data);

        Chart::new(vec![dataset])
            .block(Block::default().borders(Borders::ALL).title("Hz History"))
            .x_axis(
                Axis::default()
                    .title("Time")
                    .style(Style::default().fg(Color::Gray)),
            )
            .y_axis(
                Axis::default()
                    .title("Hz")
                    .style(Style::default().fg(Color::Gray)),
            )
    }

    pub fn get_logs_table(&self) -> Table {
        self.logger.to_tui_table()
    }

    pub fn get_cheatsheet_paragraph(&self) -> Paragraph {
        Paragraph::new(RECORDER_CHEATSHEET)
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

    pub fn get_input_output_file_paragraph(&self) -> Paragraph {
        Paragraph::new(self.input_output_file.clone())
            .block(Block::default().borders(Borders::ALL).title("Output file"))
            .style(
                Style::default().fg(if self.current_process_stop_signal.is_some() {
                    Color::Gray
                } else if self.active_input == InputField::File {
                    if self.input_output_file.is_empty() {
                        Color::Blue
                    } else if validate_file_input(&self.input_output_file) {
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
        Paragraph::new(self.input_system_id_filter.clone())
            .block(Block::default().borders(Borders::ALL).title("Sys ID"))
            .style(
                Style::default().fg(if self.current_process_stop_signal.is_some() {
                    Color::Gray
                } else if self.active_input == InputField::SystemId {
                    if self.input_system_id_filter.is_empty() {
                        Color::Blue
                    } else if validate_u8_input(&self.input_system_id_filter) {
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
        Paragraph::new(self.input_component_id_filter.clone())
            .block(Block::default().borders(Borders::ALL).title("Comp ID"))
            .style(
                Style::default().fg(if self.current_process_stop_signal.is_some() {
                    Color::Gray
                } else if self.active_input == InputField::ComponentId {
                    if self.input_component_id_filter.is_empty() {
                        Color::Blue
                    } else if validate_u8_input(&self.input_component_id_filter) {
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
