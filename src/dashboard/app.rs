use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};

use crate::config::schema::Config;
use crate::session::store::{QuadrantState, SessionStore};

pub struct App {
    pub config: Config,
    pub quadrants: Vec<QuadrantState>,
    pub selected: usize,
    pub preview_agent: usize, // index into the group for preview
    pub preview_content: String,
    pub filter: Option<String>,
    pub input_mode: bool,
    pub should_quit: bool,
    last_refresh: std::time::Instant,
}

impl App {
    pub async fn new() -> Result<Self> {
        let config = Config::load(None)?;
        let store = SessionStore::load()?;
        let quadrants = store.active_quadrants();

        Ok(Self {
            config,
            quadrants,
            selected: 0,
            preview_agent: 0,
            preview_content: String::new(),
            filter: None,
            input_mode: false,
            should_quit: false,
            last_refresh: std::time::Instant::now(),
        })
    }

    /// Handle a key event. Returns true if the app should quit.
    pub async fn handle_key(&mut self, key: KeyEvent) -> Result<bool> {
        if self.input_mode {
            match key.code {
                KeyCode::Esc => {
                    self.input_mode = false;
                }
                KeyCode::Char(c) => {
                    // Send character to the selected spirit's pane
                    if let Some(q) = self.quadrants.get(self.selected) {
                        let agents: Vec<&String> = q.agents.keys().collect();
                        if let Some(agent_name) = agents.get(self.preview_agent) {
                            let ghostty = crate::ghostty::GhosttyBackend::new();
                            let title = q.window_title(agent_name);
                            let _ = ghostty.send_text_to_window(&title, &c.to_string());
                        }
                    }
                }
                KeyCode::Enter => {
                    if let Some(q) = self.quadrants.get(self.selected) {
                        let agents: Vec<&String> = q.agents.keys().collect();
                        if let Some(agent_name) = agents.get(self.preview_agent) {
                            let ghostty = crate::ghostty::GhosttyBackend::new();
                            let title = q.window_title(agent_name);
                            let _ = ghostty.send_text_to_window(&title, "\n");
                        }
                    }
                }
                _ => {}
            }
            return Ok(false);
        }

        match key.code {
            KeyCode::Char('q') => return Ok(true),
            KeyCode::Char('j') | KeyCode::Down => {
                if !self.quadrants.is_empty() {
                    self.selected = (self.selected + 1) % self.quadrants.len();
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if !self.quadrants.is_empty() {
                    self.selected = self.selected.checked_sub(1).unwrap_or(self.quadrants.len() - 1);
                }
            }
            KeyCode::Tab => {
                // Toggle preview between agents in the group
                let agent_count = self
                    .quadrants
                    .get(self.selected)
                    .map(|q| q.agents.len())
                    .unwrap_or(1);
                self.preview_agent = (self.preview_agent + 1) % agent_count;
            }
            KeyCode::Enter => {
                // Jump to the quadrant's Ghostty window
                if let Some(q) = self.quadrants.get(self.selected) {
                    let ghostty = crate::ghostty::GhosttyBackend::new();
                    let title = format!("seance-q{}", q.quadrant);
                    let _ = ghostty.focus_window(&title);
                }
            }
            KeyCode::Char('i') => {
                self.input_mode = true;
            }
            KeyCode::Char('a') => {
                // TODO: interactive add
            }
            KeyCode::Char('m') => {
                // TODO: merge selected
            }
            KeyCode::Char('x') => {
                // TODO: remove selected
            }
            KeyCode::Char('d') => {
                // TODO: diff view
            }
            KeyCode::Char(c @ '1'..='8') => {
                let n = c.to_digit(10).unwrap_or(1) as usize;
                if n > 0 && n <= self.quadrants.len() {
                    self.selected = n - 1;
                }
            }
            _ => {}
        }

        Ok(false)
    }

    /// Periodic refresh of spirit status.
    pub async fn refresh(&mut self) -> Result<()> {
        if self.last_refresh.elapsed() < std::time::Duration::from_secs(2) {
            return Ok(());
        }
        self.last_refresh = std::time::Instant::now();

        // Reload session store
        let store = SessionStore::load()?;
        self.quadrants = store.active_quadrants();

        // Update preview content
        if let Some(q) = self.quadrants.get(self.selected) {
            let agents: Vec<&String> = q.agents.keys().collect();
            if let Some(agent_name) = agents.get(self.preview_agent) {
                let ghostty = crate::ghostty::GhosttyBackend::new();
                let title = q.window_title(agent_name);
                self.preview_content = ghostty.capture_pane(&title).unwrap_or_default();
            }
        }

        Ok(())
    }
}
