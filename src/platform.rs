//! Per-OS locations for saves, screenshots and symbol fonts.

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

/// Fonts that supply symbols such as ✦ and ▼; every one found is added as a fallback.
pub const SYMBOL_FONTS: &[&str] = &[
    "/usr/share/fonts/noto/NotoSansSymbols2-Regular.ttf",
    "/usr/share/fonts/truetype/noto/NotoSansSymbols2-Regular.ttf",
    "/usr/share/fonts/noto/NotoSansSymbols-Regular.ttf",
    "/usr/share/fonts/truetype/noto/NotoSansSymbols-Regular.ttf",
    "/usr/share/fonts/TTF/DejaVuSans.ttf",
    "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
    "C:\\Windows\\Fonts\\seguisym.ttf",
];
