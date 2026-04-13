use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};

use crate::command::detect::DetectedRepo;
use crate::config::schema::Config;
use crate::ghostty::{TerminalInput, WindowId, WindowTitle};
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
    pub status_message: Option<String>,
    pub repo_picker: Option<RepoPickerState>,
    last_refresh: std::time::Instant,
}

pub struct RepoPickerState {
    pub repos: Vec<DetectedRepo>,
    pub selected: usize,
    pub error: Option<String>,
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
            status_message: None,
            repo_picker: None,
            last_refresh: std::time::Instant::now(),
        })
    }

    /// Handle a key event. Returns true if the app should quit.
    pub async fn handle_key(&mut self, key: KeyEvent) -> Result<bool> {
        if self.repo_picker.is_some() {
            self.handle_repo_picker_key(key)?;
            return Ok(false);
        }

        if self.input_mode {
            match key.code {
                KeyCode::Esc => {
                    self.input_mode = false;
                }
                KeyCode::Char(c) => {
                    // Send character to the selected spirit's pane
                    if let Some(q) = self.quadrants.get(self.selected) {
                        let agents = q.ordered_agent_names(&self.config.group);
                        if let Some(agent_name) = agents.get(self.preview_agent) {
                            let ghostty = crate::ghostty::GhosttyBackend::new();
                            let _ = match q.window_id.as_deref() {
                                Some(window_id) => WindowId::new(window_id.to_string()).and_then(
                                    |window_id| {
                                        let text = TerminalInput::new(c.to_string());
                                        ghostty.send_text_to_window_id(&window_id, &text)
                                    },
                                ),
                                None => WindowTitle::new(q.window_title(agent_name)).and_then(
                                    |window_title| {
                                        let text = TerminalInput::new(c.to_string());
                                        ghostty.send_text_to_window(&window_title, &text)
                                    },
                                ),
                            };
                        }
                    }
                }
                KeyCode::Enter => {
                    if let Some(q) = self.quadrants.get(self.selected) {
                        let agents = q.ordered_agent_names(&self.config.group);
                        if let Some(agent_name) = agents.get(self.preview_agent) {
                            let ghostty = crate::ghostty::GhosttyBackend::new();
                            let _ = match q.window_id.as_deref() {
                                Some(window_id) => WindowId::new(window_id.to_string()).and_then(
                                    |window_id| {
                                        let text = TerminalInput::new("\n");
                                        ghostty.send_text_to_window_id(&window_id, &text)
                                    },
                                ),
                                None => WindowTitle::new(q.window_title(agent_name)).and_then(
                                    |window_title| {
                                        let text = TerminalInput::new("\n");
                                        ghostty.send_text_to_window(&window_title, &text)
                                    },
                                ),
                            };
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
                    self.clamp_preview_agent();
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if !self.quadrants.is_empty() {
                    self.selected = self
                        .selected
                        .checked_sub(1)
                        .unwrap_or(self.quadrants.len() - 1);
                    self.clamp_preview_agent();
                }
            }
            KeyCode::Tab => {
                // Toggle preview between agents in the group
                let agent_count = self
                    .quadrants
                    .get(self.selected)
                    .map(|q| q.ordered_agent_names(&self.config.group).len())
                    .unwrap_or(1)
                    .max(1);
                self.preview_agent = (self.preview_agent + 1) % agent_count;
            }
            KeyCode::Enter => {
                // Jump to the quadrant's Ghostty window
                if let Some(q) = self.quadrants.get(self.selected) {
                    let ghostty = crate::ghostty::GhosttyBackend::new();
                    let _ = match q.window_id.as_deref() {
                        Some(window_id) => WindowId::new(window_id.to_string())
                            .and_then(|window_id| ghostty.focus_window(&window_id)),
                        None => WindowTitle::new(q.main_window_title())
                            .and_then(|window_title| ghostty.focus_window_title(&window_title)),
                    };
                }
            }
            KeyCode::Char('i') => {
                self.input_mode = true;
            }
            KeyCode::Char('a') => {
                self.open_repo_picker();
            }
            KeyCode::Char('m') => {
                // TODO: merge selected
            }
            KeyCode::Char('x') | KeyCode::Delete => {
                // Delete selected branch: remove worktree, delete branch, update session
                if let Some(q) = self.quadrants.get(self.selected).cloned() {
                    let ghostty = crate::ghostty::GhosttyBackend::new();
                    let mut store = SessionStore::load()?;
                    crate::command::remove::delete_quadrant(
                        &self.config,
                        &ghostty,
                        &mut store,
                        &q,
                    )?;
                    // Refresh local state
                    self.quadrants = store.active_quadrants();
                    if self.selected >= self.quadrants.len() && !self.quadrants.is_empty() {
                        self.selected = self.quadrants.len() - 1;
                    }
                    self.clamp_preview_agent();
                }
            }
            KeyCode::Char('d') => {
                // TODO: diff view
            }
            KeyCode::Char(c @ '1'..='8') => {
                let n = c.to_digit(10).unwrap_or(1) as u8;
                if let Some(index) = self.quadrants.iter().position(|q| q.quadrant == n) {
                    self.selected = index;
                    self.clamp_preview_agent();
                }
            }
            _ => {}
        }

        Ok(false)
    }

    fn handle_repo_picker_key(&mut self, key: KeyEvent) -> Result<()> {
        let Some(mut picker) = self.repo_picker.take() else {
            return Ok(());
        };

        match key.code {
            KeyCode::Esc => {
                self.status_message = Some("Add worktree cancelled.".to_string());
            }
            KeyCode::Char('j') | KeyCode::Down => {
                if !picker.repos.is_empty() {
                    picker.selected = (picker.selected + 1) % picker.repos.len();
                }
                self.repo_picker = Some(picker);
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if !picker.repos.is_empty() {
                    picker.selected = picker
                        .selected
                        .checked_sub(1)
                        .unwrap_or(picker.repos.len().saturating_sub(1));
                }
                self.repo_picker = Some(picker);
            }
            KeyCode::Enter => {
                let repo = picker.repos[picker.selected].clone();
                match self.create_worktree_for_repo(&repo) {
                    Ok(()) => {}
                    Err(err) => {
                        picker.error = Some(err.to_string());
                        self.repo_picker = Some(picker);
                    }
                }
            }
            _ => {
                self.repo_picker = Some(picker);
            }
        }

        Ok(())
    }

    fn clamp_preview_agent(&mut self) {
        let count = self
            .quadrants
            .get(self.selected)
            .map(|q| q.ordered_agent_names(&self.config.group).len())
            .unwrap_or(1)
            .max(1);
        if self.preview_agent >= count {
            self.preview_agent = 0;
        }
    }

    fn open_repo_picker(&mut self) {
        match crate::command::detect::discover_repositories(&[]) {
            Ok(repos) if repos.is_empty() => {
                self.status_message = Some("No autodetected repos found.".to_string());
            }
            Ok(mut repos) => {
                repos.sort_by(|left, right| {
                    right
                        .has_config
                        .cmp(&left.has_config)
                        .then_with(|| left.path.cmp(&right.path))
                });
                self.repo_picker = Some(RepoPickerState {
                    repos,
                    selected: 0,
                    error: None,
                });
                self.status_message = Some("Select a repo to create a worktree.".to_string());
            }
            Err(err) => {
                self.status_message = Some(format!("Repo scan failed: {}", err));
            }
        }
    }

    fn create_worktree_for_repo(&mut self, repo: &DetectedRepo) -> Result<()> {
        let created = crate::command::add::run_in_repo(
            crate::command::add::AddArgs {
                branch: None,
                prompt: None,
                prompt_file: None,
                agent: None,
                quadrant: None,
                monitor: 0,
                base: None,
                auto_name: false,
                no_file_ops: false,
                circle: false,
            },
            &repo.path,
        )?;

        if let Some(created) = created.first() {
            self.status_message = Some(format!(
                "Created {} in Q{} on monitor {}.",
                created.branch, created.quadrant, created.monitor
            ));
        }
        self.repo_picker = None;
        self.refresh_now()?;

        if let Some(created) = created.first() {
            if let Some(index) = self
                .quadrants
                .iter()
                .position(|q| q.branch == created.branch && q.monitor == created.monitor)
            {
                self.selected = index;
                self.clamp_preview_agent();
            }
        }

        Ok(())
    }

    fn refresh_now(&mut self) -> Result<()> {
        self.last_refresh = std::time::Instant::now();
        let store = SessionStore::load()?;
        self.quadrants = store.active_quadrants();
        if self.selected >= self.quadrants.len() && !self.quadrants.is_empty() {
            self.selected = self.quadrants.len() - 1;
        }
        self.clamp_preview_agent();
        self.update_preview();
        Ok(())
    }

    fn update_preview(&mut self) {
        if let Some(q) = self.quadrants.get(self.selected) {
            let agents = q.ordered_agent_names(&self.config.group);
            if let Some(agent_name) = agents.get(self.preview_agent) {
                if self.config.dashboard.live_preview {
                    let ghostty = crate::ghostty::GhosttyBackend::new();
                    self.preview_content = match q.window_id.as_deref() {
                        Some(window_id) => WindowId::new(window_id.to_string())
                            .and_then(|window_id| ghostty.capture_window(&window_id))
                            .unwrap_or_else(|_| {
                                format!(
                                    "Live preview failed for Q{} · {} · {}",
                                    q.quadrant, q.branch, agent_name
                                )
                            }),
                        None => WindowTitle::new(q.window_title(agent_name))
                            .and_then(|window_title| ghostty.capture_pane_title(&window_title))
                            .unwrap_or_else(|_| {
                                format!(
                                    "Live preview failed for Q{} · {} · {}",
                                    q.quadrant, q.branch, agent_name
                                )
                            }),
                    };
                } else {
                    self.preview_content = format!(
                        "Live preview is disabled.\nSet `dashboard.live_preview: true` to enable it.\nSelected: Q{} · {} · {}",
                        q.quadrant, q.branch, agent_name
                    );
                }
                return;
            }
        }

        self.preview_content.clear();
    }

    /// Periodic refresh of spirit status.
    pub async fn refresh(&mut self) -> Result<()> {
        if self.last_refresh.elapsed() < std::time::Duration::from_secs(2) {
            return Ok(());
        }
        self.refresh_now()
    }
}
