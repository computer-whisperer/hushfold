//! Per-OS locations for saves and screenshots.

use std::path::PathBuf;

fn env_path(var: &str) -> Option<PathBuf> {
    std::env::var_os(var).filter(|v| !v.is_empty()).map(PathBuf::from)
}

fn home() -> Option<PathBuf> {
    env_path("HOME").or_else(|| env_path("USERPROFILE"))
}

/// Where the save file lives: `%APPDATA%\hushfold` on Windows,
/// `$XDG_DATA_HOME/hushfold` (default `~/.local/share/hushfold`) elsewhere.
pub fn data_dir() -> PathBuf {
    let base = if cfg!(windows) {
        env_path("APPDATA")
    } else {
        env_path("XDG_DATA_HOME").or_else(|| home().map(|h| h.join(".local/share")))
    };
    base.unwrap_or_else(|| PathBuf::from(".")).join("hushfold")
}

pub fn screenshot_dir() -> PathBuf {
    home().map(|h| h.join("Pictures")).unwrap_or_else(|| PathBuf::from("."))
}
