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
    render_repo_picker(frame, app);
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
    let help = if app.repo_picker.is_some() {
        "Repo picker: j/k navigate  Enter: create worktree  Esc: cancel"
    } else if app.input_mode {
        "Esc: exit input | type to send to spirit"
    } else {
        "j/k: navigate  Enter: jump  Tab: toggle agent  a: add worktree  i: input  x/Del: delete  m: merge  d: diff  q: quit"
    };

    let mut text = help.to_string();
    if let Some(message) = &app.status_message {
        text.push_str("  |  ");
        text.push_str(message);
    }

    let bar = Paragraph::new(text).style(Style::default().fg(Color::DarkGray));

    frame.render_widget(bar, area);
}

fn render_repo_picker(frame: &mut Frame, app: &App) {
    let Some(picker) = &app.repo_picker else {
        return;
    };

    let width = frame.area().width.saturating_sub(12).min(100).max(40);
    let height = frame.area().height.saturating_sub(8).min(18).max(8);
    let popup = centered_rect(width, height, frame.area());

    frame.render_widget(Clear, popup);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(4), Constraint::Length(2)])
        .split(popup);

    let max_items = rows[0].height.saturating_sub(2) as usize;
    let start = if picker.selected >= max_items && max_items > 0 {
        picker.selected + 1 - max_items
    } else {
        0
    };
    let end = if max_items > 0 {
        (start + max_items).min(picker.repos.len())
    } else {
        picker.repos.len()
    };

    let items: Vec<ListItem> = picker
        .repos
        .iter()
        .enumerate()
        .skip(start)
        .take(end.saturating_sub(start))
        .enumerate()
        .map(|(offset, (_index, repo))| {
            let absolute = start + offset;
            let marker = if absolute == picker.selected {
                "▸"
            } else {
                " "
            };
            let config = if repo.has_config { "cfg" } else { "default" };
            ListItem::new(format!(
                "{} {}  [{} | {}]",
                marker,
                repo.path.display(),
                repo.project_type,
                config
            ))
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" add worktree "),
        )
        .highlight_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        );
    frame.render_widget(list, rows[0]);

    let detail = picker
        .error
        .as_deref()
        .map(|error| format!("Error: {}", error))
        .unwrap_or_else(|| "Choose an autodetected repo and press Enter.".to_string());

    let footer = Paragraph::new(detail)
        .style(Style::default().fg(if picker.error.is_some() {
            Color::Red
        } else {
            Color::DarkGray
        }))
        .block(Block::default().borders(Borders::LEFT | Borders::RIGHT | Borders::BOTTOM));
    frame.render_widget(footer, rows[1]);
}

fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Fill(1),
            Constraint::Length(height),
            Constraint::Fill(1),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Fill(1),
            Constraint::Length(width),
            Constraint::Fill(1),
        ])
        .split(vertical[1])[1]
}
