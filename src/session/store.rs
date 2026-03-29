use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

use super::spirit::{SpiritState, SpiritStatus};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionStore {
    pub sessions: Vec<Session>,
    pub current: Option<String>,
    #[serde(skip)]
    store_path: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub name: String,
    pub status: SessionStatus,
    pub repo_path: String,
    pub created_at: String,
    pub slept_at: Option<String>,
    pub quadrants: Vec<QuadrantState>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum SessionStatus {
    Active,
    Sleeping,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuadrantState {
    pub quadrant: u8,
    pub monitor: u8,
    pub branch: String,
    pub worktree_path: PathBuf,
    #[serde(default)]
    pub window_id: Option<String>,
    pub agents: HashMap<String, SpiritState>,
    pub pr_status: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SleepingSummary {
    pub id: String,
    pub name: String,
    pub quadrant_count: usize,
    pub slept_at: String,
}

impl QuadrantState {
    pub fn window_title(&self, agent_name: &str) -> String {
        format!("seance-q{}-{}", self.quadrant, agent_name)
    }

    pub fn main_window_title(&self) -> String {
        format!("seance-q{}", self.quadrant)
    }

    pub fn ordered_agent_names(&self, preferred: &[String]) -> Vec<String> {
        let mut names: Vec<String> = preferred
            .iter()
            .filter(|name| self.agents.contains_key(*name))
            .cloned()
            .collect();

        let mut extras: Vec<String> = self
            .agents
            .keys()
            .filter(|name| !preferred.contains(*name))
            .cloned()
            .collect();
        extras.sort();
        names.extend(extras);
        names
    }

    pub fn pane_id(&self, agent_name: &str) -> Option<&str> {
        self.agents.get(agent_name)?.pane_id.as_deref()
    }
}

impl SessionStore {
    fn state_dir() -> Result<PathBuf> {
        let dir = dirs::state_dir()
            .or_else(|| dirs::data_local_dir())
            .unwrap_or_else(|| PathBuf::from("/tmp"))
            .join("seance");
        std::fs::create_dir_all(&dir)?;
        Ok(dir)
    }

    fn store_path() -> Result<PathBuf> {
        Ok(Self::state_dir()?.join("sessions.json"))
    }

    /// Create a new empty store (for testing or fresh starts).
    pub fn empty() -> Self {
        Self {
            sessions: vec![],
            current: None,
            store_path: None,
        }
    }

    fn with_store_path(path: PathBuf) -> Self {
        Self {
            sessions: vec![],
            current: None,
            store_path: Some(path),
        }
    }

    /// Load from disk or return empty store.
    pub fn load() -> Result<Self> {
        let path = Self::store_path()?;
        if !path.exists() {
            return Ok(Self::with_store_path(path));
        }

        let contents = std::fs::read_to_string(&path)
            .with_context(|| format!("reading session store: {}", path.display()))?;
        let mut store: Self =
            serde_json::from_str(&contents).with_context(|| "parsing session store")?;
        store.store_path = Some(path);
        Ok(store)
    }

    /// Save to disk.
    pub fn save(&self) -> Result<()> {
        let Some(path) = self.store_path.as_ref() else {
            return Ok(());
        };
        let contents = serde_json::to_string_pretty(self)?;
        std::fs::write(path, contents)?;
        Ok(())
    }

    fn normalize_repo_path(repo_path: &str) -> String {
        crate::git::repo_root(std::path::Path::new(repo_path))
            .unwrap_or_else(|_| std::path::PathBuf::from(repo_path))
            .to_string_lossy()
            .to_string()
    }

    /// Get or create the current active session, returning its id.
    pub fn ensure_active_session(&mut self, name: &str, repo_path: &str) -> Result<String> {
        let normalized_repo_path = Self::normalize_repo_path(repo_path);

        // If there's a current active session, return it
        if let Some(ref current_id) = self.current {
            if let Some(session) = self.sessions.iter().find(|s| s.id == *current_id) {
                if session.status == SessionStatus::Active
                    && Self::normalize_repo_path(&session.repo_path) == normalized_repo_path
                {
                    return Ok(current_id.clone());
                }
            }
        }

        if let Some(index) = self.sessions.iter().position(|session| {
            session.status == SessionStatus::Active
                && Self::normalize_repo_path(&session.repo_path) == normalized_repo_path
        }) {
            let session_id = self.sessions[index].id.clone();
            let mut changed = false;

            if self.sessions[index].repo_path != normalized_repo_path {
                self.sessions[index].repo_path = normalized_repo_path.clone();
                changed = true;
            }
            if self.current.as_deref() != Some(&session_id) {
                self.current = Some(session_id.clone());
                changed = true;
            }
            if changed {
                self.save()?;
            }

            return Ok(session_id);
        }

        // Create a new session
        let id = uuid::Uuid::new_v4().to_string()[..8].to_string();
        let session = Session {
            id: id.clone(),
            name: name.to_string(),
            status: SessionStatus::Active,
            repo_path: normalized_repo_path,
            created_at: chrono::Utc::now().to_rfc3339(),
            slept_at: None,
            quadrants: vec![],
        };

        self.sessions.push(session);
        self.current = Some(id.clone());
        self.save()?;

        Ok(id)
    }

    /// Add a quadrant to the current active session.
    pub fn add_quadrant(&mut self, session_id: &str, state: QuadrantState) -> Result<()> {
        let session = self
            .sessions
            .iter_mut()
            .find(|s| s.id == session_id)
            .context("Session not found")?;

        // Replace if quadrant already exists
        session
            .quadrants
            .retain(|q| q.quadrant != state.quadrant || q.monitor != state.monitor);
        session.quadrants.push(state);

        self.save()
    }

    /// Remove a quadrant by branch name from the active session.
    pub fn remove_quadrant(&mut self, branch: &str) -> Result<Option<QuadrantState>> {
        for session in &mut self.sessions {
            if session.status != SessionStatus::Active {
                continue;
            }
            if let Some(pos) = session.quadrants.iter().position(|q| q.branch == branch) {
                let removed = session.quadrants.remove(pos);
                let session_empty = session.quadrants.is_empty();
                let session_id = session.id.clone();
                if session_empty {
                    if self.current.as_deref() == Some(&session_id) {
                        self.current = None;
                    }
                    self.sessions.retain(|s| s.id != session_id);
                }
                self.save()?;
                return Ok(Some(removed));
            }
        }
        Ok(None)
    }

    /// Find a quadrant by branch name or quadrant number.
    pub fn find_quadrant(&self, target: &str) -> Option<QuadrantState> {
        let quadrants = self.active_quadrants();
        // Try as quadrant number first
        if let Ok(num) = target.parse::<u8>() {
            return quadrants.into_iter().find(|q| q.quadrant == num);
        }
        // Then try as branch name
        quadrants.into_iter().find(|q| q.branch == target)
    }

    pub fn current_session_id(&self) -> Option<String> {
        self.current.clone().or_else(|| {
            self.sessions
                .iter()
                .find(|session| session.status == SessionStatus::Active)
                .map(|session| session.id.clone())
        })
    }

    pub fn active_quadrants(&self) -> Vec<QuadrantState> {
        self.sessions
            .iter()
            .filter(|s| s.status == SessionStatus::Active)
            .flat_map(|s| s.quadrants.clone())
            .collect()
    }

    /// Get occupied quadrant numbers on a given monitor.
    pub fn occupied_quadrants(&self, monitor: u8) -> Vec<u8> {
        self.active_quadrants()
            .iter()
            .filter(|q| q.monitor == monitor)
            .map(|q| q.quadrant)
            .collect()
    }

    pub fn sleeping_sessions(&self) -> Vec<SleepingSummary> {
        self.sessions
            .iter()
            .filter(|s| s.status == SessionStatus::Sleeping)
            .map(|s| SleepingSummary {
                id: s.id.clone(),
                name: s.name.clone(),
                quadrant_count: s.quadrants.len(),
                slept_at: s.slept_at.clone().unwrap_or_default(),
            })
            .collect()
    }

    pub fn set_status(&mut self, session_id: &str, status: SessionStatus) -> Result<()> {
        if let Some(session) = self.sessions.iter_mut().find(|s| s.id == session_id) {
            if status == SessionStatus::Sleeping {
                session.slept_at = Some(chrono::Utc::now().to_rfc3339());
            } else {
                self.current = Some(session_id.to_string());
            }
            session.status = status;
            self.save()?;
        }
        Ok(())
    }

    pub fn quadrants_for(&self, session_id: &str) -> Result<Vec<QuadrantState>> {
        let session = self
            .sessions
            .iter()
            .find(|s| s.id == session_id)
            .context("Session not found")?;
        Ok(session.quadrants.clone())
    }

    pub fn save_snapshot(
        &mut self,
        session_id: &str,
        quadrant: u8,
        agent: &str,
        snapshot: &str,
    ) -> Result<()> {
        let dir = Self::state_dir()?.join("snapshots");
        std::fs::create_dir_all(&dir)?;
        let filename = format!("{}__{}__{}.txt", session_id, quadrant, agent);
        std::fs::write(dir.join(filename), snapshot)?;
        Ok(())
    }

    pub fn load_snapshot(session_id: &str, quadrant: u8, agent: &str) -> Result<Option<String>> {
        let dir = Self::state_dir()?.join("snapshots");
        let filename = format!("{}__{}__{}.txt", session_id, quadrant, agent);
        let path = dir.join(filename);
        if path.exists() {
            Ok(Some(std::fs::read_to_string(path)?))
        } else {
            Ok(None)
        }
    }

    pub fn next_quadrant(&self) -> u8 {
        let active = self.active_quadrants();
        if active.is_empty() {
            return 1;
        }
        // Find the next quadrant after the current selection
        let max = active.iter().map(|q| q.quadrant).max().unwrap_or(0);
        if max < 8 { max + 1 } else { 1 }
    }

    pub fn prev_quadrant(&self) -> u8 {
        let active = self.active_quadrants();
        if active.is_empty() {
            return 1;
        }
        let min = active.iter().map(|q| q.quadrant).min().unwrap_or(1);
        if min > 1 {
            min - 1
        } else {
            active.iter().map(|q| q.quadrant).max().unwrap_or(1)
        }
    }

    pub fn clean_closed(&self) -> Result<usize> {
        let dir = Self::state_dir()?.join("snapshots");
        if !dir.exists() {
            return Ok(0);
        }

        let mut count = 0;
        for entry in std::fs::read_dir(&dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() {
                std::fs::remove_file(&path)?;
                count += 1;
            }
        }
        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn test_store() -> SessionStore {
        SessionStore::empty()
    }

    fn init_git_repo(path: &Path) {
        let status = std::process::Command::new("git")
            .args(["init", "-q"])
            .current_dir(path)
            .status()
            .unwrap();
        assert!(status.success());
    }

    #[test]
    fn test_empty_store() {
        let store = test_store();
        assert!(store.sessions.is_empty());
        assert!(store.current.is_none());
        assert!(store.active_quadrants().is_empty());
    }

    #[test]
    fn test_ensure_active_session() {
        let mut store = test_store();
        let id = store
            .ensure_active_session("test-session", "/tmp/repo")
            .unwrap();
        assert!(!id.is_empty());
        assert_eq!(store.sessions.len(), 1);
        assert_eq!(store.sessions[0].name, "test-session");
        assert_eq!(store.sessions[0].status, SessionStatus::Active);
        assert_eq!(store.current, Some(id.clone()));

        // Calling again returns same session
        let id2 = store.ensure_active_session("another", "/tmp/repo").unwrap();
        assert_eq!(id, id2);
        assert_eq!(store.sessions.len(), 1);
    }

    #[test]
    fn test_ensure_active_session_different_repo() {
        let mut store = test_store();
        let id1 = store
            .ensure_active_session("session-a", "/tmp/repo-a")
            .unwrap();
        assert_eq!(store.sessions.len(), 1);

        // Different repo_path should create a new session
        let id2 = store
            .ensure_active_session("session-b", "/tmp/repo-b")
            .unwrap();
        assert_ne!(id1, id2);
        assert_eq!(store.sessions.len(), 2);
        assert_eq!(store.current, Some(id2));
    }

    #[test]
    fn test_ensure_active_session_reuses_existing_repo_after_switching() {
        let repo_a = tempfile::tempdir().unwrap();
        let repo_b = tempfile::tempdir().unwrap();
        init_git_repo(repo_a.path());
        init_git_repo(repo_b.path());

        let mut store = test_store();
        let id1 = store
            .ensure_active_session("session-a", repo_a.path().to_str().unwrap())
            .unwrap();
        let id2 = store
            .ensure_active_session("session-b", repo_b.path().to_str().unwrap())
            .unwrap();
        let id3 = store
            .ensure_active_session("session-a-again", repo_a.path().to_str().unwrap())
            .unwrap();

        assert_ne!(id1, id2);
        assert_eq!(id1, id3);
        assert_eq!(store.sessions.len(), 2);
        assert_eq!(store.current, Some(id1));
    }

    #[test]
    fn test_ensure_active_session_normalizes_repo_subdir_to_same_session() {
        let repo = tempfile::tempdir().unwrap();
        init_git_repo(repo.path());
        let subdir = repo.path().join("src/nested");
        std::fs::create_dir_all(&subdir).unwrap();

        let mut store = test_store();
        let id1 = store
            .ensure_active_session("session-root", repo.path().to_str().unwrap())
            .unwrap();
        let id2 = store
            .ensure_active_session("session-subdir", subdir.to_str().unwrap())
            .unwrap();

        assert_eq!(id1, id2);
        assert_eq!(store.sessions.len(), 1);
        assert_eq!(
            store.sessions[0].repo_path,
            repo.path().canonicalize().unwrap().to_string_lossy()
        );
    }

    #[test]
    fn test_add_and_remove_quadrant() {
        let mut store = test_store();
        let id = store.ensure_active_session("test", "/tmp").unwrap();

        let q1 = new_quadrant_state(
            1,
            0,
            "feat/auth",
            "/tmp/auth".into(),
            &["claude".into(), "codex".into()],
        );
        store.add_quadrant(&id, q1).unwrap();

        assert_eq!(store.active_quadrants().len(), 1);
        assert_eq!(store.active_quadrants()[0].branch, "feat/auth");
        assert_eq!(store.active_quadrants()[0].agents.len(), 2);

        // Add second quadrant
        let q2 = new_quadrant_state(2, 0, "feat/api", "/tmp/api".into(), &["claude".into()]);
        store.add_quadrant(&id, q2).unwrap();
        assert_eq!(store.active_quadrants().len(), 2);

        // Remove first
        let removed = store.remove_quadrant("feat/auth").unwrap();
        assert!(removed.is_some());
        assert_eq!(store.active_quadrants().len(), 1);
        assert_eq!(store.active_quadrants()[0].branch, "feat/api");
    }

    #[test]
    fn test_remove_last_quadrant_closes_session() {
        let mut store = test_store();
        let id = store.ensure_active_session("test", "/tmp").unwrap();

        let q = new_quadrant_state(1, 0, "feat/only", "/tmp/only".into(), &["claude".into()]);
        store.add_quadrant(&id, q).unwrap();
        assert_eq!(store.current, Some(id.clone()));

        // Removing the last quadrant should remove the session and clear current
        store.remove_quadrant("feat/only").unwrap();
        assert_eq!(store.active_quadrants().len(), 0);
        assert_eq!(store.current, None);
        assert!(store.sessions.is_empty());
    }

    #[test]
    fn test_replace_existing_quadrant() {
        let mut store = test_store();
        let id = store.ensure_active_session("test", "/tmp").unwrap();

        let q1 = new_quadrant_state(1, 0, "feat/old", "/tmp/old".into(), &["claude".into()]);
        store.add_quadrant(&id, q1).unwrap();

        // Replace same quadrant
        let q1_new = new_quadrant_state(1, 0, "feat/new", "/tmp/new".into(), &["codex".into()]);
        store.add_quadrant(&id, q1_new).unwrap();

        assert_eq!(store.active_quadrants().len(), 1);
        assert_eq!(store.active_quadrants()[0].branch, "feat/new");
    }

    #[test]
    fn test_find_quadrant_by_number() {
        let mut store = test_store();
        let id = store.ensure_active_session("test", "/tmp").unwrap();

        let q = new_quadrant_state(3, 0, "feat/x", "/tmp/x".into(), &["claude".into()]);
        store.add_quadrant(&id, q).unwrap();

        assert!(store.find_quadrant("3").is_some());
        assert_eq!(store.find_quadrant("3").unwrap().branch, "feat/x");
        assert!(store.find_quadrant("1").is_none());
    }

    #[test]
    fn test_find_quadrant_by_branch() {
        let mut store = test_store();
        let id = store.ensure_active_session("test", "/tmp").unwrap();

        let q = new_quadrant_state(1, 0, "feat/auth", "/tmp/auth".into(), &["claude".into()]);
        store.add_quadrant(&id, q).unwrap();

        assert!(store.find_quadrant("feat/auth").is_some());
        assert!(store.find_quadrant("feat/nope").is_none());
    }

    #[test]
    fn test_sleep_wake_cycle() {
        let mut store = test_store();
        let id = store.ensure_active_session("test", "/tmp").unwrap();

        let q = new_quadrant_state(1, 0, "feat/x", "/tmp/x".into(), &["claude".into()]);
        store.add_quadrant(&id, q).unwrap();

        // Sleep
        store.set_status(&id, SessionStatus::Sleeping).unwrap();
        assert!(store.active_quadrants().is_empty());
        assert_eq!(store.sleeping_sessions().len(), 1);
        assert!(store.sessions[0].slept_at.is_some());

        // Wake
        store.set_status(&id, SessionStatus::Active).unwrap();
        assert_eq!(store.active_quadrants().len(), 1);
        assert!(store.sleeping_sessions().is_empty());
    }

    #[test]
    fn test_occupied_quadrants() {
        let mut store = test_store();
        let id = store.ensure_active_session("test", "/tmp").unwrap();

        store
            .add_quadrant(
                &id,
                new_quadrant_state(1, 0, "b1", "/tmp/1".into(), &["c".into()]),
            )
            .unwrap();
        store
            .add_quadrant(
                &id,
                new_quadrant_state(3, 0, "b3", "/tmp/3".into(), &["c".into()]),
            )
            .unwrap();
        store
            .add_quadrant(
                &id,
                new_quadrant_state(5, 1, "b5", "/tmp/5".into(), &["c".into()]),
            )
            .unwrap();

        assert_eq!(store.occupied_quadrants(0), vec![1, 3]);
        assert_eq!(store.occupied_quadrants(1), vec![5]);
        assert!(store.occupied_quadrants(2).is_empty());
    }

    #[test]
    fn test_next_prev_quadrant() {
        let mut store = test_store();
        let id = store.ensure_active_session("test", "/tmp").unwrap();

        store
            .add_quadrant(
                &id,
                new_quadrant_state(1, 0, "b1", "/tmp/1".into(), &["c".into()]),
            )
            .unwrap();
        store
            .add_quadrant(
                &id,
                new_quadrant_state(3, 0, "b3", "/tmp/3".into(), &["c".into()]),
            )
            .unwrap();

        assert_eq!(store.next_quadrant(), 4); // max(1,3) + 1
        assert_eq!(store.prev_quadrant(), 3); // wraps: min is 1, so prev = max = 3... actually min > 1 is false
        // min=1, so prev wraps to max=3
    }

    #[test]
    fn test_window_title() {
        let q = new_quadrant_state(2, 0, "feat/x", "/tmp/x".into(), &["claude".into()]);
        assert_eq!(q.window_title("claude"), "seance-q2-claude");
        assert_eq!(q.main_window_title(), "seance-q2");
    }

    #[test]
    fn test_new_quadrant_state_agents() {
        let q = new_quadrant_state(1, 0, "b", "/tmp".into(), &["claude".into(), "codex".into()]);
        assert_eq!(q.agents.len(), 2);
        assert!(q.agents.contains_key("claude"));
        assert!(q.agents.contains_key("codex"));
        assert_eq!(q.agents["claude"].status, SpiritStatus::Starting);
    }

    #[test]
    fn test_serialization_roundtrip() {
        let mut store = test_store();
        let id = store.ensure_active_session("test", "/tmp").unwrap();
        let q = new_quadrant_state(1, 0, "feat/auth", "/tmp/auth".into(), &["claude".into()]);
        store.add_quadrant(&id, q).unwrap();

        let json = serde_json::to_string(&store).unwrap();
        let parsed: SessionStore = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.sessions.len(), 1);
        assert_eq!(parsed.active_quadrants().len(), 1);
        assert_eq!(parsed.active_quadrants()[0].branch, "feat/auth");
    }
}

/// Build a QuadrantState from add parameters.
pub fn new_quadrant_state(
    quadrant: u8,
    monitor: u8,
    branch: &str,
    worktree_path: PathBuf,
    agents: &[String],
) -> QuadrantState {
    let agent_map: HashMap<String, SpiritState> = agents
        .iter()
        .map(|name| {
            (
                name.clone(),
                SpiritState {
                    status: SpiritStatus::Starting,
                    pane_id: None,
                    last_activity: Some(chrono::Utc::now().to_rfc3339()),
                },
            )
        })
        .collect();

    QuadrantState {
        quadrant,
        monitor,
        branch: branch.to_string(),
        worktree_path,
        window_id: None,
        agents: agent_map,
        pr_status: None,
    }
}
