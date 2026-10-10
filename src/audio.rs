//! Sound, all synthesised: effects heard from where they happen, the ambience of
//! each place, and generative music, mixed on the audio device's thread by
//! `engine`. Nothing is loaded from disk; the clips are synthesised on a thread at
//! startup, and effects asked for before they are ready are skipped.

mod ambience;
mod clips;
mod dsp;
mod engine;
mod music;

use engine::{BLOCK, Cmd, Engine};
use glam::Vec3;
use rodio::{ChannelCount, SampleRate, Source};
use std::num::NonZero;
use std::sync::Arc;
use std::sync::mpsc::{SyncSender, sync_channel};
use std::time::Duration;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Sfx {
    /// A book slid home onto its shelf.
    Thunk,
    /// Books taken up into the arms.
    Rustle,
    /// Books gliding off to their shelves.
    Whoosh,
    /// Books thrown.
    Toss,
    /// A tossed book landing.
    Land,
    /// A series completed; `play_pitched` by its length.
    Chime,
    /// A category fully catalogued.
    Fanfare,
    /// A department's veil lifting.
    Veil,
    /// A skill learned in the Grimoire.
    Purchase,
    /// A spell cast; `play_pitched` by its hotbar slot.
    Spell,
    /// A familiar posted: it unfolds. `play_pitched` by its voice: 0 to 2 a
    /// shelver's, 3 to 5 a courier's.
    Unfold,
    /// A familiar taken back: it folds up. `play_pitched` by its voice, as `Unfold`.
    Fold,
    /// Refused.
    Nope,
    /// Cycling, regrouping, seeking.
    Tick,
    /// A footfall.
    Step,
    /// Pushing off to jump.
    Hop,
    /// Landing on the floor; louder the harder the fall.
    Alight,
    /// A footfall into a heap of books; louder the deeper it is.
    Wade,
}

impl Sfx {
    pub const ALL: [Sfx; 18] = [
        Sfx::Thunk,
        Sfx::Rustle,
        Sfx::Whoosh,
        Sfx::Toss,
        Sfx::Land,
        Sfx::Chime,
        Sfx::Fanfare,
        Sfx::Veil,
        Sfx::Purchase,
        Sfx::Spell,
        Sfx::Unfold,
        Sfx::Fold,
        Sfx::Nope,
        Sfx::Tick,
        Sfx::Step,
        Sfx::Hop,
        Sfx::Alight,
        Sfx::Wade,
    ];
}

/// The places with an ambience of their own.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Place {
    /// The Hall, wings and all.
    Hall,
    /// The Great Library's atrium, its galleries and stairs.
    Atrium,
    Observatory,
    AlchemicalWing,
    Herbarium,
    Bestiary,
    MapRoom,
    ChronicleHall,
    VerseGallery,
    Arcanum,
    Clockworks,
    MusicRoom,
    Weatherhouse,
    HearthLibrary,
    /// A department room without one of its own.
    Room,
}

impl Place {
    pub const ALL: [Place; 15] = [
        Place::Hall,
        Place::Atrium,
        Place::Observatory,
        Place::AlchemicalWing,
        Place::Herbarium,
        Place::Bestiary,
        Place::MapRoom,
        Place::ChronicleHall,
        Place::VerseGallery,
        Place::Arcanum,
        Place::Clockworks,
        Place::MusicRoom,
        Place::Weatherhouse,
        Place::HearthLibrary,
        Place::Room,
    ];

    /// A Great Library department's room, by the department's name.
    pub fn department(name: &str) -> Place {
        match name {
            "Observatory" => Place::Observatory,
            "Alchemical Wing" => Place::AlchemicalWing,
            "Herbarium" => Place::Herbarium,
            "Bestiary" => Place::Bestiary,
            "Map Room" => Place::MapRoom,
            "Chronicle Hall" => Place::ChronicleHall,
            "Verse Gallery" => Place::VerseGallery,
            "Arcanum" => Place::Arcanum,
            "Clockworks" => Place::Clockworks,
            "Music Room" => Place::MusicRoom,
            "Weatherhouse" => Place::Weatherhouse,
            "Hearth Library" => Place::HearthLibrary,
            _ => Place::Room,
        }
    }
}

/// Fires heard at once, and familiars' wings.
pub const MAX_FIRES: usize = 2;
pub const MAX_WINGS: usize = 3;
/// How much of a familiar's wingbeat is the downstroke, which pushes the air.
pub const DOWNSTROKE: f32 = 0.42;

/// A familiar on the wing, as its wings are heard.
#[derive(Clone, Copy, Debug)]
pub struct Wingbeat {
    /// Its own number, so it keeps its own wings from frame to frame.
    pub id: usize,
    pub at: Vec3,
    /// Where its wings are in their beat (0 to 1 from the top, the downstroke first,
    /// as it is drawn), beats a second (0 while it glides), and how hard they beat (0
    /// held still).
    pub phase: f32,
    pub rate: f32,
    pub flap: f32,
    /// A courier's wings: smaller, quicker and lighter than a shelver's.
    pub courier: bool,
}

/// Where the listener is and what is around them, each frame.
#[derive(Clone, Copy)]
pub struct Scene {
    pub ears: Vec3,
    /// The listener's right, level.
    pub right: Vec3,
    /// How much of each place's ambience reaches the ears, by `Place`.
    pub places: [f32; Place::ALL.len()],
    /// Fires: where each burns and how much of it is heard (0 for none).
    pub fires: [(Vec3, f32); MAX_FIRES],
    /// Familiars on the wing, nearest first.
    pub wings: [Option<Wingbeat>; MAX_WINGS],
}

impl Default for Scene {
    fn default() -> Self {
        Scene {
            ears: Vec3::ZERO,
            right: Vec3::X,
            places: [0.0; Place::ALL.len()],
            fires: [(Vec3::ZERO, 0.0); MAX_FIRES],
            wings: [None; MAX_WINGS],
        }
    }
}

/// The engine as a rodio source: interleaved stereo, rendered a block at a time.
struct Output {
    engine: Engine,
    buf: [f32; BLOCK * 2],
    at: usize,
}

impl Iterator for Output {
    type Item = rodio::Sample;
    fn next(&mut self) -> Option<rodio::Sample> {
        if self.at == self.buf.len() {
            self.engine.render(&mut self.buf);
            self.at = 0;
        }
        self.at += 1;
        Some(self.buf[self.at - 1])
    }
}

impl Source for Output {
    fn current_span_len(&self) -> Option<usize> {
        None
    }
    fn channels(&self) -> ChannelCount {
        NonZero::new(2).unwrap()
    }
    fn sample_rate(&self) -> SampleRate {
        NonZero::new(dsp::SR).unwrap()
    }
    fn total_duration(&self) -> Option<Duration> {
        None
    }
}

/// The game's handle on the sound.
pub struct Audio {
    /// Held to keep the device open.
    _sink: Option<rodio::MixerDeviceSink>,
    tx: Option<SyncSender<Cmd>>,
}

impl Audio {
    pub fn new() -> Self {
        let sink = match rodio::DeviceSinkBuilder::open_default_sink() {
            Ok(mut s) => {
                s.log_on_drop(false);
                s
            }
            Err(e) => {
                eprintln!("hushfold: audio unavailable ({e}); continuing silently");
                return Audio { _sink: None, tx: None };
            }
        };
        let (tx, rx) = sync_channel(1024);
        let seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(1, |d| d.as_nanos() as u64);
        sink.mixer().add(Output {
            engine: Engine::new(rx, seed),
            buf: [0.0; BLOCK * 2],
            at: BLOCK * 2,
        });
        let banks = tx.clone();
        std::thread::spawn(move || {
            let _ = banks.send(Cmd::Banks(Arc::new(clips::Banks::synthesize())));
        });
        Audio {
            _sink: Some(sink),
            tx: Some(tx),
        }
    }

    /// No output device and no synthesis; every play is a no-op. Used by tests.
    #[cfg(test)]
    pub fn silent() -> Self {
        Audio { _sink: None, tx: None }
    }

    fn send(&self, c: Cmd) {
        if let Some(tx) = &self.tx {
            // A full queue means the device has stalled; drop rather than pile up.
            let _ = tx.try_send(c);
        }
    }

    /// Volumes in 0..=1. Effects cover sound effects and the ambience.
    pub fn configure(&mut self, music_on: bool, music_volume: f32, effects_volume: f32) {
        let music = if music_on { music_volume } else { 0.0 };
        self.send(Cmd::Gains {
            music,
            effects: effects_volume,
        });
    }

    /// Play `s` at the listener (the player's own doings, and the interface).
    pub fn play(&mut self, s: Sfx, vol: f32) {
        self.send(Cmd::Play {
            sfx: s,
            vol,
            variant: None,
            at: None,
        });
    }

    /// Play variant `step` of `s` at the listener (see `Sfx`).
    pub fn play_pitched(&mut self, s: Sfx, vol: f32, step: usize) {
        self.send(Cmd::Play {
            sfx: s,
            vol,
            variant: Some(step),
            at: None,
        });
    }

    /// Play `s` from `at` in the world: panned by its bearing, quieter and duller
    /// with distance.
    pub fn play_at(&mut self, s: Sfx, vol: f32, at: Vec3) {
        self.send(Cmd::Play {
            sfx: s,
            vol,
            variant: None,
            at: Some(at),
        });
    }

    /// Where the listener is now and what surrounds them (each frame).
    pub fn set_scene(&mut self, scene: Scene) {
        self.send(Cmd::Scene(scene));
    }
}

#[cfg(test)]
mod tests;
