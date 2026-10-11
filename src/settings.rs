//! Player preferences, kept apart from the save so starting over keeps them.

use serde::{Deserialize, Serialize};

#[derive(Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// Multiplier on the base look speed.
    pub look_speed: f32,
    pub invert_look: bool,
    /// Vertical field of view on wide windows, in degrees.
    pub fov: f32,
    /// The eye rising and falling with each step while walking.
    pub head_bob: bool,
    /// A multiplier on the exposure.
    pub brightness: f32,
    /// The size of the menus and what is drawn over the stacks (egui's zoom).
    pub ui_scale: f32,
    /// The frame rate and timings in a corner; toggled with F3.
    pub show_fps: bool,
    /// Under the music and the effects both.
    pub master_volume: f32,
    /// Toggled with M.
    pub music: bool,
    pub music_volume: f32,
    pub effects_volume: f32,
    /// The output the sound plays on; None: the system's.
    pub output: Option<crate::audio::Device>,
    /// Toggled with F11; restored at launch.
    pub fullscreen: bool,
}

/// The interface sizes offered (`ui_scale`).
pub const UI_SCALES: [f32; 8] = [0.75, 0.9, 1.0, 1.15, 1.3, 1.5, 1.75, 2.0];

impl Default for Settings {
    fn default() -> Self {
        Settings {
            look_speed: 1.0,
            invert_look: false,
            fov: 70.0,
            head_bob: true,
            brightness: 1.0,
            ui_scale: 1.0,
            show_fps: false,
            master_volume: 1.0,
            music: true,
            music_volume: 0.8,
            effects_volume: 1.0,
            output: None,
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
            brightness: s.brightness.clamp(0.6, 1.6),
            ui_scale: s.ui_scale.clamp(UI_SCALES[0], UI_SCALES[UI_SCALES.len() - 1]),
            master_volume: s.master_volume.clamp(0.0, 1.0),
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

#[cfg(test)]
mod tests {
    use super::*;

    /// A settings file from before the settings screen keeps what it set; what it
    /// predates takes its default, and what has gone (the library played last) is
    /// passed over.
    #[test]
    fn older_settings_still_load() {
        let old = r#"{"look_speed": 1.0, "invert_look": false, "fov": 70.0, "head_bob": false,
            "music": true, "music_volume": 0.73, "effects_volume": 1.0, "fullscreen": false,
            "library": "Great"}"#;
        let s: Settings = serde_json::from_str(old).unwrap();
        assert!(!s.head_bob && s.music_volume == 0.73);
        let new = Settings::default();
        assert_eq!(
            (s.brightness, s.ui_scale, s.show_fps, s.master_volume),
            (new.brightness, new.ui_scale, new.show_fps, new.master_volume)
        );
        assert_eq!(s.output, None);
    }

    /// An output chosen is kept by its id, with its name to show while it isn't there.
    #[test]
    fn the_output_chosen_is_kept() {
        let s = Settings {
            output: Some(crate::audio::Device {
                id: "alsa_output.usb-headset.analog-stereo".into(),
                name: "Headset".into(),
            }),
            ..Settings::default()
        };
        let back: Settings = serde_json::from_slice(&serde_json::to_vec_pretty(&s).unwrap()).unwrap();
        assert!(back == s);
    }
}
