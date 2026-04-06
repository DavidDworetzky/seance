use ratatui::prelude::*;
use ratatui::widgets::*;

use super::app::App;

const ASCII_BANNER: &str = r#"
       .-.    .--.    .-.
      ( o )  ( oo )  ( o )
       '-'    '--'    '-'       _~^~^~_
      /||\   /||\   /||\    \)/  o  o  \(/
   ~~~~~~~~~~~~~~~~~~~~~~~~~~  '_  -  _'
                               / '---' \
     s  e  a  n  c  e        (  (   )  )
     a gathering of spirits    \_|   |_/
"#;

pub fn render(frame: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // header
            Constraint::Min(8),     // spirit table
            Constraint::Length(12), // preview
            Constraint::Length(1),  // status bar
        ])
        .split(frame.area());

    render_header(frame, chunks[0], app);
    render_spirit_table(frame, chunks[1], app);
    render_preview(frame, chunks[2], app);
    render_status_bar(frame, chunks[3], app);
}

fn render_header(frame: &mut Frame, area: Rect, app: &App) {
    let session_name = app
        .quadrants
        .first()
        .map(|q| q.branch.as_str())
        .unwrap_or("no session");

    let header = Paragraph::new(format!(
        "  seance                              session: {}",
        session_name
    ))
    .style(
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    )
    .block(Block::default().borders(Borders::BOTTOM));

    frame.render_widget(header, area);
}

fn render_spirit_table(frame: &mut Frame, area: Rect, app: &mut App) {
    if app.quadrants.is_empty() {
        let empty = Paragraph::new(ASCII_BANNER)
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL).title(" spirits "));
        frame.render_widget(empty, area);
        return;
    }

    let header_cells = ["Q", "Branch", "Claude", "Codex", "PR", "Mon"]
        .iter()
        .map(|h| {
            Cell::from(*h).style(
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )
        });
    let header = Row::new(header_cells).height(1);

    let rows: Vec<Row> = app
        .quadrants
        .iter()
        .enumerate()
        .map(|(i, q)| {
            let style = if i == app.selected {
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Gray)
            };

            let agent_statuses: Vec<String> = app
                .config
                .group
                .iter()
                .map(|name| {
                    q.agents
                        .get(name)
                        .map(|s| s.status.icon(&app.config.status_icons))
                        .unwrap_or_else(|| "--".into())
                })
                .collect();

            let marker = if i == app.selected { "▸ " } else { "  " };

            Row::new(vec![
                Cell::from(format!("{}{}", marker, q.quadrant)),
                Cell::from(q.branch.clone()),
                Cell::from(agent_statuses.first().cloned().unwrap_or_default()),
                Cell::from(agent_statuses.get(1).cloned().unwrap_or_default()),
                Cell::from(q.pr_status.as_deref().unwrap_or("--").to_string()),
                Cell::from(q.monitor.to_string()),
            ])
            .style(style)
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Length(5),
            Constraint::Min(20),
            Constraint::Length(8),
            Constraint::Length(8),
            Constraint::Length(8),
            Constraint::Length(5),
        ],
    )
    .header(header)
    .block(Block::default().borders(Borders::ALL).title(" spirits "));

    frame.render_widget(table, area);
}

fn render_preview(frame: &mut Frame, area: Rect, app: &App) {
    let agent_name = app
        .config
        .group
        .get(app.preview_agent)
        .map(|s| s.as_str())
        .unwrap_or("--");

    let title = if let Some(q) = app.quadrants.get(app.selected) {
        format!(" Q{} · {} · {} ", q.quadrant, q.branch, agent_name)
    } else {
        " preview ".to_string()
    };

    let mode_indicator = if app.input_mode { " [INPUT MODE] " } else { "" };

    let content = if app.preview_content.is_empty() {
        "No output captured yet.".to_string()
    } else {
        // Show last N lines that fit
        let lines: Vec<&str> = app.preview_content.lines().collect();
        let max_lines = area.height.saturating_sub(2) as usize;
        let start = lines.len().saturating_sub(max_lines);
        lines[start..].join("\n")
    };

    let preview = Paragraph::new(format!("{}{}", mode_indicator, content))
        .style(Style::default().fg(Color::White))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(title)
                .border_style(if app.input_mode {
                    Style::default().fg(Color::Green)
                } else {
                    Style::default()
                }),
        );

    frame.render_widget(preview, area);
}

fn render_status_bar(frame: &mut Frame, area: Rect, app: &App) {
    let help = if app.input_mode {
        "Esc: exit input | type to send to spirit"
    } else {
        "j/k: navigate  Enter: jump  Tab: toggle agent  i: input  x/Del: delete  m: merge  d: diff  q: quit"
    };

    let bar = Paragraph::new(help).style(Style::default().fg(Color::DarkGray));

    frame.render_widget(bar, area);
}
