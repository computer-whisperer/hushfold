//! Player preferences, kept apart from the save so starting over keeps them.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// Multiplier on the base look speed.
    pub look_speed: f32,
    pub invert_look: bool,
    /// Vertical field of view on wide windows, in degrees.
    pub fov: f32,
    /// The eye rising and falling with each step while walking.
    pub head_bob: bool,
    /// Toggled with M.
    pub music: bool,
    pub music_volume: f32,
    pub effects_volume: f32,
    /// Toggled with F11; restored at launch.
    pub fullscreen: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            look_speed: 1.0,
            invert_look: false,
            fov: 70.0,
            head_bob: true,
            music: true,
            music_volume: 0.8,
            effects_volume: 1.0,
            fullscreen: false,
        }
    }
}

impl Settings {
    fn path() -> std::path::PathBuf {
        crate::platform::data_dir().join("settings.json")
    }

    pub fn load() -> Self {
        let s: Settings = std::fs::read(Self::path())
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default();
        Settings {
            look_speed: s.look_speed.clamp(0.2, 4.0),
            fov: s.fov.clamp(55.0, 90.0),
            music_volume: s.music_volume.clamp(0.0, 1.0),
            effects_volume: s.effects_volume.clamp(0.0, 1.0),
            ..s
        }
    }

    pub fn save(&self) {
        let path = Self::path();
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        if let Ok(bytes) = serde_json::to_vec_pretty(self)
            && std::fs::write(&path, bytes).is_err()
        {
            eprintln!("hushfold: failed to write settings to {}", path.display());
        }
    }
}
