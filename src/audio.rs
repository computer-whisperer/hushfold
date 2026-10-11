//! Sound, all synthesised: effects heard from where they happen, the ambience of
//! each place, and generative music, mixed by `engine` on the thread of the stream
//! `output` keeps open. Nothing is loaded from disk; the clips are synthesised on a
//! thread at startup, and effects asked for before they are ready are skipped.
//!
//! Sound is the one place the platform's float maths may be used (`clippy.toml`):
//! nothing heard is saved or shapes the world.
#![allow(clippy::disallowed_methods)]

mod ambience;
mod clips;
mod dsp;
mod engine;
mod music;
mod output;

pub use dsp::SR;
use engine::{Cmd, Engine};
use glam::Vec3;
pub use output::{Device, Outputs, Playing};
use output::{Feed, Output};
use std::sync::Arc;
use std::sync::mpsc::{SyncSender, sync_channel};

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

/// The game's handle on the sound.
pub struct Audio {
    tx: Option<SyncSender<Cmd>>,
    out: Option<Output>,
    /// The output last asked for (an id; None: the system's).
    chosen: Option<String>,
    /// The mix, when it is rendered as the game asks (`taped`) rather than played.
    tape: Option<Feed>,
}

impl Audio {
    /// Sound on the output `chosen` (an id from `outputs`; None: the system's), or on
    /// the system's while that one isn't there.
    pub fn new(chosen: Option<String>) -> Self {
        let (tx, rx) = sync_channel(1024);
        let seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(1, |d| d.as_nanos() as u64);
        let out = Output::start(Engine::new(rx, seed), chosen.clone());
        let banks = tx.clone();
        std::thread::spawn(move || {
            let _ = banks.send(Cmd::Banks(Arc::new(clips::Banks::synthesize())));
        });
        Audio {
            tx: Some(tx),
            out: Some(out),
            chosen,
            tape: None,
        }
    }

    /// Nothing played: the mix is rendered as the game goes (`tape`), to be recorded
    /// with a clip or let go. Scripted runs' sound, so it is the same each run, and no
    /// run made for pictures or tests is heard.
    pub fn taped(seed: u64) -> Self {
        let (tx, rx) = sync_channel(1024);
        // The clips at once, so the first effects a clip asks for are in it.
        let _ = tx.send(Cmd::Banks(Arc::new(clips::Banks::synthesize())));
        Audio {
            tx: Some(tx),
            out: None,
            chosen: None,
            tape: Some(Feed::taped(Engine::new(rx, seed))),
        }
    }

    /// No output and no synthesis; every play is a no-op. Tests', and
    /// `HUSHFOLD_AUDIO=none`.
    pub fn silent() -> Self {
        Audio {
            tx: None,
            out: None,
            chosen: None,
            tape: None,
        }
    }

    /// The next `frames` frames of a taped mix (`taped`), interleaved stereo at
    /// `SR`; none from one played on an output.
    pub fn tape(&mut self, frames: usize) -> Vec<f32> {
        let Some(feed) = &mut self.tape else {
            return Vec::new();
        };
        (0..frames).flat_map(|_| feed.frame()).collect()
    }

    /// Move the sound to the output `chosen` (as for `new`).
    pub fn choose(&mut self, chosen: Option<String>) {
        if chosen != self.chosen
            && let Some(out) = &self.out
        {
            out.choose(chosen.clone());
        }
        self.chosen = chosen;
    }

    /// The outputs as last listed, and where the sound is playing (a taped mix's
    /// as if on the system's output, the settings saying nothing of it).
    pub fn outputs(&self) -> Outputs {
        match &self.out {
            Some(out) => out.outputs(),
            None if self.tape.is_some() => Outputs {
                playing: Playing::System,
                ..Outputs::default()
            },
            None => Outputs::default(),
        }
    }

    /// Ask for the outputs to be listed again (while they are on show; at most
    /// every couple of seconds).
    pub fn list_outputs(&mut self) {
        if let Some(out) = &mut self.out {
            out.list();
        }
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
