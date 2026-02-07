use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::app::App;
use crate::replay::ReplayApp;

pub fn draw(f: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // title bar
            Constraint::Min(0),   // main content
            Constraint::Length(3), // filter / help bar
        ])
        .split(f.area());

    draw_title_bar(f, app, chunks[0]);
    draw_main(f, app, chunks[1]);
    draw_filter_bar(f, app, chunks[2]);
}

fn draw_title_bar(f: &mut Frame, app: &App, area: Rect) {
    let heartbeat_info = match app.heartbeat {
        Some((sys, comp)) => format!(" | HB: {}:{}", sys, comp),
        None => String::new(),
    };

    let title = Line::from(vec![
        Span::styled(
            " mavshark ",
            Style::default().fg(Color::Black).bg(Color::Cyan).bold(),
        ),
        Span::raw(format!(
            " {} | {} msgs{} | sort: {} ",
            app.uri, app.total_count, heartbeat_info, app.sort_label()
        )),
    ]);

    f.render_widget(
        Paragraph::new(title).style(Style::default().bg(Color::DarkGray)),
        area,
    );
}

fn draw_main(f: &mut Frame, app: &mut App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(45), Constraint::Percentage(55)])
        .split(area);

    draw_message_list(f, app, chunks[0]);
    draw_message_detail(f, app, chunks[1]);
}

fn draw_message_list(f: &mut Frame, app: &mut App, area: Rect) {
    let header = Row::new(vec![
        Cell::from("Message"),
        Cell::from("Src"),
        Cell::from("Hz"),
        Cell::from("Count"),
    ])
    .style(Style::default().bold().fg(Color::Yellow))
    .bottom_margin(1);

    let rows: Vec<Row> = app
        .filtered_indices
        .iter()
        .map(|&idx| {
            let entry = &app.entries[idx];
            Row::new(vec![
                Cell::from(entry.name.clone()),
                Cell::from(format!("{}:{}", entry.sys_id, entry.comp_id)),
                Cell::from(format!("{:.1}", entry.hz)),
                Cell::from(format_count(entry.count)),
            ])
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Min(20),
            Constraint::Length(7),
            Constraint::Length(7),
            Constraint::Length(7),
        ],
    )
    .header(header)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title(format!(" Messages ({}) ", app.filtered_indices.len()))
            .border_style(Style::default().fg(Color::Cyan)),
    )
    .row_highlight_style(Style::default().bg(Color::DarkGray).fg(Color::White))
    .highlight_symbol(" > ");

    f.render_stateful_widget(table, area, &mut app.table_state);
}

fn draw_message_detail(f: &mut Frame, app: &App, area: Rect) {
    let (title, content) = match app.selected_entry() {
        Some(entry) => {
            let title = format!(" {} [{}:{}] ", entry.name, entry.sys_id, entry.comp_id);
            (title, entry.last_content.clone())
        }
        None => (" Detail ".to_string(), "No message selected".to_string()),
    };

    let lines: Vec<Line> = content
        .lines()
        .skip(app.detail_scroll)
        .map(|line| {
            if let Some(colon_pos) = line.find(':') {
                let (key, val) = line.split_at(colon_pos);
                Line::from(vec![
                    Span::styled(key.to_string(), Style::default().fg(Color::Green)),
                    Span::raw(val.to_string()),
                ])
            } else {
                Line::from(Span::styled(
                    line.to_string(),
                    Style::default().fg(Color::DarkGray),
                ))
            }
        })
        .collect();

    let detail = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .title(title)
            .border_style(Style::default().fg(Color::Cyan)),
    );

    f.render_widget(detail, area);
}

fn draw_filter_bar(f: &mut Frame, app: &App, area: Rect) {
    let (style, border_style) = if app.filter_active {
        (
            Style::default().fg(Color::Yellow),
            Style::default().fg(Color::Yellow),
        )
    } else {
        (
            Style::default().fg(Color::DarkGray),
            Style::default().fg(Color::DarkGray),
        )
    };

    let filter_text = if app.filter_active {
        format!(" / {}_", app.filter)
    } else if app.filter.is_empty() {
        " / search | j/k navigate | s sort | d/u scroll detail | q quit".to_string()
    } else {
        format!(" / {} | Esc clear", app.filter)
    };

    let block = Block::default().borders(Borders::ALL).border_style(border_style);
    let paragraph = Paragraph::new(filter_text).style(style).block(block);
    f.render_widget(paragraph, area);
}

fn format_count(count: u64) -> String {
    if count >= 1_000_000 {
        format!("{:.1}M", count as f64 / 1_000_000.0)
    } else if count >= 1_000 {
        format!("{:.1}k", count as f64 / 1_000.0)
    } else {
        count.to_string()
    }
}

// --- Replay UI ---

pub fn draw_replay(f: &mut Frame, app: &mut ReplayApp) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // title bar
            Constraint::Min(0),   // main content
            Constraint::Length(3), // filter / help bar
        ])
        .split(f.area());

    draw_replay_title_bar(f, app, chunks[0]);
    draw_replay_main(f, app, chunks[1]);
    draw_replay_filter_bar(f, app, chunks[2]);
}

fn draw_replay_title_bar(f: &mut Frame, app: &ReplayApp, area: Rect) {
    let position = if app.filtered_indices.is_empty() {
        "0/0".to_string()
    } else {
        format!("{}/{}", app.selected + 1, app.filtered_indices.len())
    };

    let title = Line::from(vec![
        Span::styled(
            " mavshark replay ",
            Style::default().fg(Color::Black).bg(Color::Magenta).bold(),
        ),
        Span::raw(format!(" {} | {} ", app.file_path, position)),
    ]);

    f.render_widget(
        Paragraph::new(title).style(Style::default().bg(Color::DarkGray)),
        area,
    );
}

fn draw_replay_main(f: &mut Frame, app: &mut ReplayApp, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(45), Constraint::Percentage(55)])
        .split(area);

    draw_replay_message_list(f, app, chunks[0]);
    draw_replay_message_detail(f, app, chunks[1]);
}

fn draw_replay_message_list(f: &mut Frame, app: &mut ReplayApp, area: Rect) {
    let header = Row::new(vec![
        Cell::from("#"),
        Cell::from("Time"),
        Cell::from("Message"),
        Cell::from("Src"),
    ])
    .style(Style::default().bold().fg(Color::Yellow))
    .bottom_margin(1);

    let rows: Vec<Row> = app
        .filtered_indices
        .iter()
        .map(|&idx| {
            let msg = &app.messages[idx];
            let time = msg.timestamp.format("%H:%M:%S%.3f").to_string();
            Row::new(vec![
                Cell::from(format!("{}", idx + 1)),
                Cell::from(time),
                Cell::from(msg.message_name.clone()),
                Cell::from(format!(
                    "{}:{}",
                    msg.header.system_id, msg.header.component_id
                )),
            ])
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Length(6),
            Constraint::Length(13),
            Constraint::Min(15),
            Constraint::Length(7),
        ],
    )
    .header(header)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title(format!(" Messages ({}) ", app.filtered_indices.len()))
            .border_style(Style::default().fg(Color::Magenta)),
    )
    .row_highlight_style(Style::default().bg(Color::DarkGray).fg(Color::White))
    .highlight_symbol(" > ");

    f.render_stateful_widget(table, area, &mut app.table_state);
}

fn draw_replay_message_detail(f: &mut Frame, app: &ReplayApp, area: Rect) {
    let (title, content) = match app.selected_message() {
        Some(msg) => {
            let title = format!(
                " {} [{}:{}] ",
                msg.message_name, msg.header.system_id, msg.header.component_id
            );
            (title, msg.message.clone())
        }
        None => (" Detail ".to_string(), "No message selected".to_string()),
    };

    let lines: Vec<Line> = content
        .lines()
        .skip(app.detail_scroll)
        .map(|line| {
            if let Some(colon_pos) = line.find(':') {
                let (key, val) = line.split_at(colon_pos);
                Line::from(vec![
                    Span::styled(key.to_string(), Style::default().fg(Color::Green)),
                    Span::raw(val.to_string()),
                ])
            } else {
                Line::from(Span::styled(
                    line.to_string(),
                    Style::default().fg(Color::DarkGray),
                ))
            }
        })
        .collect();

    let detail = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .title(title)
            .border_style(Style::default().fg(Color::Magenta)),
    );

    f.render_widget(detail, area);
}

fn draw_replay_filter_bar(f: &mut Frame, app: &ReplayApp, area: Rect) {
    let (style, border_style) = if app.filter_active {
        (
            Style::default().fg(Color::Yellow),
            Style::default().fg(Color::Yellow),
        )
    } else {
        (
            Style::default().fg(Color::DarkGray),
            Style::default().fg(Color::DarkGray),
        )
    };

    let filter_text = if app.filter_active {
        format!(" / {}_", app.filter)
    } else if app.filter.is_empty() {
        " / search | j/k navigate | g/G start/end | d/u scroll detail | q quit".to_string()
    } else {
        format!(" / {} | Esc clear", app.filter)
    };

    let block = Block::default().borders(Borders::ALL).border_style(border_style);
    let paragraph = Paragraph::new(filter_text).style(style).block(block);
    f.render_widget(paragraph, area);
}
