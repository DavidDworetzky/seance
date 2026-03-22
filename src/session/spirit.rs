use serde::{Deserialize, Serialize};

use crate::config::schema::StatusIcons;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpiritState {
    pub status: SpiritStatus,
    pub pane_id: Option<String>,
    pub last_activity: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum SpiritStatus {
    Starting,
    Working,
    Waiting,
    Done,
    Closed,
}

impl SpiritStatus {
    pub fn icon(&self, icons: &StatusIcons) -> String {
        match self {
            SpiritStatus::Starting => icons.starting.clone(),
            SpiritStatus::Working => icons.working.clone(),
            SpiritStatus::Waiting => icons.waiting.clone(),
            SpiritStatus::Done => icons.done.clone(),
            SpiritStatus::Closed => icons.closed.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spirit_status_icons() {
        let icons = StatusIcons::default();
        assert_eq!(SpiritStatus::Starting.icon(&icons), "◌");
        assert_eq!(SpiritStatus::Working.icon(&icons), "⚡");
        assert_eq!(SpiritStatus::Waiting.icon(&icons), "◎");
        assert_eq!(SpiritStatus::Done.icon(&icons), "✓");
        assert_eq!(SpiritStatus::Closed.icon(&icons), "✗");
    }

    #[test]
    fn test_spirit_status_serde() {
        let state = SpiritState {
            status: SpiritStatus::Working,
            pane_id: Some("abc".into()),
            last_activity: None,
        };
        let json = serde_json::to_string(&state).unwrap();
        let parsed: SpiritState = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.status, SpiritStatus::Working);
        assert_eq!(parsed.pane_id, Some("abc".into()));
    }

    #[test]
    fn test_spirit_status_equality() {
        assert_eq!(SpiritStatus::Done, SpiritStatus::Done);
        assert_ne!(SpiritStatus::Done, SpiritStatus::Working);
    }
}
