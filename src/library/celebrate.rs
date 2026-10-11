//! What the library does when a part of it is put in order: pieces that wait for
//! it (hidden until it, then coming in; or there, then moving off, turning or going;
//! or dark, then lit), sparks and sounds. Generation lays them down here, where it
//! lays down everything else; the game knows when each part was finished
//! (`game::celebrate`) and plays them. Looks and sounds only, as motion is: nothing
//! that waits is in anyone's way, and saves never see it.

use super::Library;
use super::motion::Motion;
use crate::audio::Sfx;
use crate::gfx::kind;
use glam::{Quat, Vec3};

/// A part of the library that can be finished: every book that belongs in it home.
/// Shelving is for good, so finishing is too.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Finish {
    /// One side of a bookcase (`Library::faces`).
    Face(usize),
    Category(usize),
    /// A department's room (in the Hall, its wing).
    Room(usize),
    /// Every room on a storey (`Room::level`); the Hall has one.
    Storey(usize),
    /// Every book in the library.
    Library,
}

/// What a mover waits for, and how it changes once that is finished: `delay` seconds
/// after it, over `secs`, eased in and out. Riders of a mover share its change, so a
/// piece can wait inside one that waits.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Awaits {
    pub finish: Finish,
    pub delay: f32,
    pub secs: f32,
    pub change: Change,
    /// Whether its own motion waits too: at rest until the change begins, then easing
    /// up to speed over `secs` (a still orrery starting to turn).
    pub still: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Change {
    /// Not there until then; then it comes in from `from` (an offset from where it
    /// rests, in the world), growing from `grow` of its size (about its pivot) to all
    /// of it.
    Arrives { from: Vec3, grow: f32 },
    /// There at rest until then; then it moves `by`, turns by `turn` about its pivot and
    /// grows or shrinks to `scale` of its size, and stays so; at 0 it is gone (a dust
    /// sheet lifted away).
    Shifts { by: Vec3, turn: Quat, scale: f32 },
    /// Its glowing parts (`kind::glows`) and the lights it carries at `dim` of their
    /// brightness until then (0: dark), then full.
    Kindles { dim: f32 },
    /// Nothing about it changes: only its own motion waits (`Awaits::starts`).
    Stays,
}

impl Change {
    /// Coming in where it rests, growing from `grow` of its size about its pivot.
    pub fn grows(grow: f32) -> Change {
        Change::Arrives { from: Vec3::ZERO, grow }
    }
}

impl Awaits {
    /// Waiting for `finish` to change as `change`: at once, over two and a half
    /// seconds, its own motion running all along.
    pub fn new(finish: Finish, change: Change) -> Awaits {
        Awaits {
            finish,
            delay: 0.0,
            secs: 2.5,
            change,
            still: false,
        }
    }

    pub fn after(self, delay: f32) -> Awaits {
        Awaits { delay, ..self }
    }

    pub fn over(self, secs: f32) -> Awaits {
        Awaits { secs, ..self }
    }

    /// Its own motion waits too (see `still`).
    pub fn still(self) -> Awaits {
        Awaits { still: true, ..self }
    }

    /// Only its own motion waits for `finish`: at rest until then, then easing up to
    /// speed.
    pub fn starts(finish: Finish) -> Awaits {
        Awaits::new(finish, Change::Stays).still()
    }

    /// How far through its change it is `since` seconds after its finish (`None`: not
    /// finished): from 0 to 1, eased.
    pub fn progress(&self, since: Option<f64>) -> f32 {
        let Some(s) = since else { return 0.0 };
        let x = (((s - self.delay as f64) / self.secs.max(1e-3) as f64) as f32).clamp(0.0, 1.0);
        x * x * (3.0 - 2.0 * x)
    }

    /// Seconds into its own motion `since` seconds after its finish, while it waits
    /// still: 0 until the change begins, then speeding up evenly over `secs`.
    pub fn motion_time(&self, since: Option<f64>) -> f64 {
        let s = since.map_or(0.0, |s| (s - self.delay as f64).max(0.0));
        let ramp = self.secs.max(1e-3) as f64;
        if s < ramp { s * s / (2.0 * ramp) } else { s - ramp * 0.5 }
    }
}

/// A source of sparks (or petals, scraps of paper, motes) that waits for `finish`:
/// from `delay` seconds after it, for `lasts` seconds (the moment) or for good
/// (`None`, a lasting change). They start anywhere in the box `at` ± `spread`, `rate`
/// a second, moving at `vel` give or take `jitter` each way; they rise at `lift`
/// m/s² (falling if negative), their speed dies away by `drag` a second, they are
/// pushed about at random by up to `wander` m/s², and each lives `life` seconds. An
/// `EMISSIVE` one glows `rgba` (w its brightness) and fades as it goes; any other
/// kind (a petal of `LEAF`, a scrap of `PAPER`) tumbles and shrinks away at the end.
/// `size` is a box's. `spread`, `vel` and `jitter` are in the world's axes (a room's
/// from `RoomFrame::size`). Build from `burst` or `motes` and change what differs.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Sparks {
    pub finish: Finish,
    pub delay: f32,
    pub lasts: Option<f32>,
    pub at: Vec3,
    pub spread: Vec3,
    pub rate: f32,
    pub vel: Vec3,
    pub jitter: Vec3,
    pub lift: f32,
    pub drag: f32,
    pub wander: f32,
    pub life: f32,
    pub kind: u32,
    pub rgba: [f32; 4],
    pub size: Vec3,
}

impl Sparks {
    /// The moment: for two seconds, glints of gold rising from round `at` and
    /// spreading as they go.
    pub fn burst(finish: Finish, at: Vec3) -> Sparks {
        Sparks {
            finish,
            delay: 0.0,
            lasts: Some(2.0),
            at,
            spread: Vec3::splat(0.3),
            rate: 60.0,
            vel: Vec3::Y * 0.8,
            jitter: Vec3::new(0.6, 0.4, 0.6),
            lift: 0.15,
            drag: 1.5,
            wander: 0.0,
            life: 1.6,
            kind: kind::EMISSIVE,
            rgba: [1.6, 1.15, 0.5, 2.5],
            size: Vec3::splat(0.02),
        }
    }

    /// For good: a few slow motes of warm light drifting about the box `at` ±
    /// `spread`.
    pub fn motes(finish: Finish, at: Vec3, spread: Vec3) -> Sparks {
        Sparks {
            lasts: None,
            spread,
            rate: 3.0,
            vel: Vec3::ZERO,
            jitter: Vec3::splat(0.05),
            lift: 0.0,
            drag: 0.5,
            wander: 0.15,
            life: 6.0,
            rgba: [1.0, 0.85, 0.55, 1.5],
            size: Vec3::splat(0.012),
            ..Sparks::burst(finish, at)
        }
    }
}

/// A sound that waits for `finish`: `sound` from `at`, `delay` seconds after it, at
/// `volume` (0 to 1). Heard only when it is finished in play, never on loading.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Cue {
    pub finish: Finish,
    pub delay: f32,
    pub sound: Sfx,
    pub at: Vec3,
    pub volume: f32,
}

impl Library {
    /// Mover `m` waits for `a.finish` and changes as `a` says. A mover that only waits
    /// is made with `Motion::Still`.
    pub fn awaiting(&mut self, m: u32, a: Awaits) {
        self.movers[m as usize].awaits = Some(a);
    }

    /// A mover that only waits, with no parts of its own (for lights and movers to
    /// ride): about `pivot`, changing as `a` says.
    pub fn waiter(&mut self, pivot: Vec3, a: Awaits) -> u32 {
        self.waiting(pivot, a, [])
    }

    /// A piece (or several, as one mover's parts) that only waits: `parts` about
    /// `pivot`, changing as `a` says.
    pub fn waiting(&mut self, pivot: Vec3, a: Awaits, parts: impl IntoIterator<Item = crate::gfx::Inst>) -> u32 {
        let m = self.mover(pivot, Motion::Still, 0.0, None);
        self.awaiting(m, a);
        for p in parts {
            self.moving(m, p);
        }
        m
    }

    pub fn sparks(&mut self, s: Sparks) {
        self.sparks.push(s);
    }

    pub fn cue(&mut self, c: Cue) {
        self.cues.push(c);
    }

    /// How many storeys the library has (one more than its rooms' highest level).
    pub fn storeys(&self) -> usize {
        self.rooms.iter().map(|r| r.level + 1).max().unwrap_or(1)
    }

    /// The bookcase sides category `cat` is shelved on, in reading order.
    pub fn faces_of(&self, cat: usize) -> impl Iterator<Item = usize> + '_ {
        (self.faces.iter().enumerate())
            .filter(move |(_, f)| f.category == cat)
            .map(|(i, _)| i)
    }
}
