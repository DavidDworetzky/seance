use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};

static DEBUG_GHOSTTY: AtomicBool = AtomicBool::new(false);

pub fn set_debug_ghostty(enabled: bool) {
    DEBUG_GHOSTTY.store(enabled, Ordering::Relaxed);
    if enabled {
        log("diagnostic", "diagnostic mode enabled");
    }
}

pub fn debug_ghostty() -> bool {
    DEBUG_GHOSTTY.load(Ordering::Relaxed)
}

pub fn log(category: &str, message: &str) {
    if !debug_ghostty() {
        return;
    }

    let Ok(path) = diagnostic_log_path() else {
        return;
    };

    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
    {
        let timestamp = chrono::Local::now().to_rfc3339();
        let _ = writeln!(file, "[{}] {} {}", timestamp, category, message);
    }
}

pub fn diagnostic_log_path() -> anyhow::Result<PathBuf> {
    let dir = dirs::state_dir()
        .or_else(|| dirs::data_local_dir())
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join("seance");
    std::fs::create_dir_all(&dir)?;
    Ok(dir.join("diagnostic.log"))
}
