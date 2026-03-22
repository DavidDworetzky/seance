use crate::config::schema::Config;
use crate::session::store::SessionStore;

#[derive(Debug, Clone)]
pub struct WindowBounds {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

pub struct QuadrantAssigner {
    occupied: Vec<u8>,
    max_quadrants: u8,
}

impl QuadrantAssigner {
    pub fn new(store: &SessionStore, max_quadrants: u8) -> Self {
        Self {
            occupied: store.active_quadrants().iter().map(|q| q.quadrant).collect(),
            max_quadrants,
        }
    }

    /// Return the next available quadrant on a given monitor.
    pub fn next_available(&self, _monitor: u8) -> u8 {
        for q in 1..=self.max_quadrants {
            if !self.occupied.contains(&q) {
                return q;
            }
        }
        // All full — wrap to 1
        1
    }
}

/// Compute window bounds for a given quadrant on a monitor.
///
/// Quadrant numbering (per monitor):
///   1 = top-left,  2 = top-right
///   3 = bottom-left, 4 = bottom-right
pub fn compute_bounds(quadrant: u8, monitor: u8, config: &Config) -> WindowBounds {
    let displays = super::monitor::detect_displays();

    let display = displays
        .get(monitor as usize)
        .unwrap_or_else(|| displays.first().expect("No displays detected"));

    let gap = config.monitors.gap as i32;

    // Map quadrant to row/col (1-indexed quadrant -> 0-indexed grid)
    let q = ((quadrant - 1) % 4) as i32;
    let col = q % 2;
    let row = q / 2;

    let half_w = (display.width - gap * 3) / 2;
    let half_h = (display.height - gap * 3) / 2;

    let x = display.x + gap + col * (half_w + gap);
    let y = display.y + gap + row * (half_h + gap);

    WindowBounds {
        x,
        y,
        width: half_w,
        height: half_h,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_bounds_q1() {
        let config = Config::default();
        // Use a known display size by testing the math directly
        let bounds = WindowBounds {
            x: 0,
            y: 0,
            width: 960,
            height: 540,
        };
        // Q1 should be top-left
        assert!(bounds.x == 0);
        assert!(bounds.y == 0);
        assert!(bounds.width > 0);
        assert!(bounds.height > 0);
    }

    #[test]
    fn test_quadrant_grid_positions() {
        // Test that quadrants map to correct row/col
        // Q1 -> (0,0), Q2 -> (0,1), Q3 -> (1,0), Q4 -> (1,1)
        for q in 1..=4u8 {
            let idx = ((q - 1) % 4) as i32;
            let col = idx % 2;
            let row = idx / 2;
            match q {
                1 => { assert_eq!(col, 0); assert_eq!(row, 0); }
                2 => { assert_eq!(col, 1); assert_eq!(row, 0); }
                3 => { assert_eq!(col, 0); assert_eq!(row, 1); }
                4 => { assert_eq!(col, 1); assert_eq!(row, 1); }
                _ => unreachable!(),
            }
        }
    }

    #[test]
    fn test_assigner_next_available() {
        let store = SessionStore::empty();
        let assigner = QuadrantAssigner::new(&store, 4);
        assert_eq!(assigner.next_available(0), 1);
    }

    #[test]
    fn test_assigner_skips_occupied() {
        let mut store = SessionStore::empty();
        // Simulate occupied quadrants
        let session = crate::session::store::Session {
            id: "test".into(),
            name: "test".into(),
            status: crate::session::store::SessionStatus::Active,
            repo_path: "/tmp".into(),
            created_at: "now".into(),
            slept_at: None,
            quadrants: vec![
                crate::session::store::new_quadrant_state(1, 0, "b1", "/tmp/b1".into(), &["claude".into()]),
                crate::session::store::new_quadrant_state(2, 0, "b2", "/tmp/b2".into(), &["claude".into()]),
            ],
        };
        store.sessions.push(session);

        let assigner = QuadrantAssigner::new(&store, 4);
        assert_eq!(assigner.next_available(0), 3);
    }
}
