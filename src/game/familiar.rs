//! Familiars: folded-paper birds that work round a post you choose. A shelver (a
//! crane) files loose books lying near its post whose shelves are a short walk from
//! it; a courier (a swift) carries books lying near its post to where they are
//! wanted: a shelver that can file them, or else their room. Unposted, a familiar
//! rides on your shoulder.
//!
//! A familiar that finds nothing to do for two minutes folds up and comes back to
//! the shoulder.
//!
//! Every trip follows the walking routes of the nav grid from the post (its
//! `Field`), at flying height, straightened where the line is clear to sight, so a
//! familiar goes through doorways and up the stairs, or over a railing and across
//! the well, but never through a wall or a floor.
//!
//! Each is drawn as a folded-paper bird (`gfx::crane`): a shelver a crane, a courier a
//! smaller swift, each familiar of a role folded from a paper of its own. How it is
//! drawn (`Look`) follows what it does without touching it.

use super::*;
use crate::audio::DOWNSTROKE;
use crate::gfx::crane::{Crane, Fold, Paper, Print};
use std::collections::HashMap;
use std::f32::consts::{FRAC_PI_4, TAU};

/// Familiar Wings by level: (flight speed in m/s, books carried, how far round its
/// post a familiar takes books from, how far on foot from its post a shelver files).
/// Doubled from the first cut (2.6 m/s and 2 books at first), under which familiars
/// shelved ~6% of the Great Library and the familiars path was the slowest.
pub(super) const WINGS: [(f32, usize, f32, f32); 3] = [(3.5, 4, 4.0, 10.0), (4.8, 6, 5.0, 12.0), (6.0, 10, 6.0, 14.0)];
/// Flying height above the floor along a route.
const FLY_H: f32 = 1.7;
/// Height of a perch's top above the floor.
const PERCH_H: f32 = 1.25;
/// How far a familiar is from where it is going when it has arrived.
const ARRIVE: f32 = 0.25;
/// How far round the spot it flew to a familiar takes books from.
const GRASP: f32 = 0.8;
/// How near the shelver a courier would take it to a book already lies (a pile set
/// down there spreads this far).
const SETTLED: f32 = 2.5;
/// Seconds a posted familiar finds nothing to do before it comes back to the shoulder.
const IDLE_HOME: f32 = 120.0;
/// Height of the tray on top of a perch, which a perched bird sits in.
const TRAY: f32 = 1.16;
/// A riding familiar's size, to a posted one's.
const RIDING: f32 = 0.55;
/// The papers shelvers are folded from, one each: pages of the library's own, in
/// tints that tell them apart across a room (a page of writing, an old map, a song,
/// a star chart, a letter on rose paper).
const SHELVER_PAPERS: [Paper; 5] = [
    Paper {
        tint: [0.8, 0.74, 0.6],
        print: Print::Text,
    },
    Paper {
        tint: [0.66, 0.44, 0.2],
        print: Print::Map,
    },
    Paper {
        tint: [0.3, 0.62, 0.3],
        print: Print::Music,
    },
    Paper {
        tint: [0.25, 0.42, 0.8],
        print: Print::StarChart,
    },
    Paper {
        tint: [0.8, 0.3, 0.35],
        print: Print::Text,
    },
];
/// And couriers': deep blues printed with patterns in light ink.
const COURIER_PAPERS: [Paper; 4] = [
    Paper {
        tint: [0.05, 0.08, 0.32],
        print: Print::Dots,
    },
    Paper {
        tint: [0.02, 0.19, 0.24],
        print: Print::Waves,
    },
    Paper {
        tint: [0.1, 0.17, 0.45],
        print: Print::Stripes,
    },
    Paper {
        tint: [0.13, 0.06, 0.3],
        print: Print::Checks,
    },
];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Role {
    Shelver,
    Courier,
}

impl Role {
    pub fn name(self) -> &'static str {
        match self {
            Role::Shelver => "shelver",
            Role::Courier => "courier",
        }
    }

    /// The colour of its paper's glow and its light.
    pub(super) fn light(self) -> [f32; 3] {
        match self {
            Role::Shelver => [1.0, 0.82, 0.55],
            Role::Courier => [0.55, 0.68, 1.0],
        }
    }

    /// How it is folded, and its size posted.
    fn fold(self) -> (Fold, f32) {
        match self {
            Role::Shelver => (Fold::Crane, 1.2),
            Role::Courier => (Fold::Swift, 1.05),
        }
    }

    /// Its wingbeats a second flying steadily, and how long it glides between bouts of
    /// beating (seconds, least and most).
    fn wingbeat(self) -> (f32, (f32, f32)) {
        match self {
            Role::Shelver => (1.9, (0.6, 1.3)),
            Role::Courier => (2.9, (0.35, 0.8)),
        }
    }

    /// The papers its familiars are folded from, in the order they come.
    fn papers(self) -> &'static [Paper] {
        match self {
            Role::Shelver => &SHELVER_PAPERS,
            Role::Courier => &COURIER_PAPERS,
        }
    }
}

/// Where a posted familiar works from: a spot on the floor, and walking distances
/// from it.
pub(super) struct Post {
    pub(super) at: Vec3,
    level: usize,
    field: Field,
}

#[derive(Clone, Copy, PartialEq)]
enum Task {
    /// At its post (or riding), looking for work now and then.
    Idle,
    /// To the books round a point, to take those bound for `goal` (a courier) or
    /// those it can file (a shelver, `goal` unused).
    Fetch { at: Vec3, goal: Vec3 },
    /// Carrying books to set them down at a point.
    Deliver(Vec3),
    /// Filing what it carries, one slot after another.
    File,
    /// Back to its post.
    Home,
}

pub(super) struct Familiar {
    pub(super) role: Role,
    pub(super) post: Option<Post>,
    pub(super) pos: Vec3,
    vel: Vec3,
    task: Task,
    carry: Vec<u32>,
    /// Points still to fly through, the next last.
    path: Vec<Vec3>,
    phase: f32,
    rest: f32,
    /// Game time it last took to the shoulder; the longest riding is posted first.
    riding_since: f32,
    /// Seconds it has sat at its post finding nothing to do.
    pub(super) idle: f32,
    pub(super) look: Look,
    /// Books it has set down where they were wanted (a courier); for the pacing bot.
    #[cfg(test)]
    pub(super) delivered: u32,
}

impl Familiar {
    /// Settle on its perch at once (on loading), with nothing to do yet.
    pub(super) fn perch(&mut self) {
        if let Some(post) = &self.post {
            self.pos = post.at + Vec3::Y * PERCH_H;
            self.path.clear();
            self.task = Task::Idle;
            self.rest = 0.5;
            self.look.size = self.role.fold().1;
            self.look.settled = 1.0;
        }
    }

    /// Posted and off its perch, or beating its wings: heard.
    pub(super) fn on_the_wing(&self) -> bool {
        self.post.is_some() && (self.look.settled < 0.5 || self.look.flap > 0.01)
    }

    /// Which of the Unfold and Fold sounds it makes (`Sfx`): its role's, each
    /// familiar of a role one of three.
    fn voice(&self, i: usize) -> usize {
        self.role as usize * 3 + i % 3
    }

    /// Ease how familiar `i` is drawn toward what it is doing, `dt` seconds on, at
    /// game time `t`: `speed` is how fast familiars fly, `(eye, yaw)` the player's.
    fn animate(&mut self, i: usize, dt: f32, speed: f32, (eye, yaw): (Vec3, f32), t: f32) {
        let (cruise, (short, long)) = self.role.wingbeat();
        let ease = |rate: f32| 1.0 - (-dt * rate).exp();
        let v = self.vel;
        let l = &mut self.look;
        let accel = (v - l.was) / dt.max(1e-4);
        l.was = v;
        let level = Vec2::new(v.x, v.z).length();
        // Working hard (speeding up, climbing, swinging round a corner), or slowing
        // to arrive, which it flares for.
        let effort = (accel.length() / (speed * 3.0)).max(v.y / speed * 2.0).clamp(0.0, 1.0);
        let flare = (-accel.dot(v.normalize_or_zero()) / (speed * 2.5)).clamp(0.0, 1.0);
        let perch = self.post.as_ref().map(|p| p.at + Vec3::Y * PERCH_H);
        let riding = perch.is_none();
        // Idle at its perch (it stops within `ARRIVE` of the point over it).
        let perched = perch.is_some_and(|p| self.task == Task::Idle && self.pos.distance(p) < ARRIVE * 2.0);
        let hovering = !riding && !perched && self.rest > 0.0;
        let flying = !riding && !perched && !hovering;
        // It faces along its flight; on its perch half toward the player nearby (to
        // one side or the other, so its shape shows), or else about it; riding, the
        // way the player looks.
        let target = if riding {
            yaw
        } else if level > 0.3 {
            v.x.atan2(-v.z)
        } else if perched {
            let to = eye - self.pos;
            if to.length() < 8.0 {
                to.x.atan2(-to.z) + if hash(i, 0) < 0.5 { 0.9 } else { -0.9 }
            } else {
                hash(i, (t / 6.0) as u32) * TAU
            }
        } else {
            l.heading
        };
        let rate = if flying {
            8.0
        } else if riding {
            6.0
        } else {
            2.0
        };
        let turn = wrap(target - l.heading) * ease(rate);
        l.heading = wrap(l.heading + turn);
        // Banking into a turn as it would to make it at its speed; nose up climbing,
        // flaring to land and hovering.
        let bank = if flying {
            (-turn / dt.max(1e-4) * level * 0.08).clamp(-0.7, 0.7)
        } else {
            0.0
        };
        l.bank += (bank - l.bank) * ease(6.0);
        let pitch = if hovering {
            0.3
        } else if flying {
            v.y.atan2(level) * 0.6 + 0.35 * flare
        } else {
            0.0
        };
        l.pitch += (pitch - l.pitch) * ease(5.0);
        // Wings: beating in bouts with a glide between when it flies steadily, quicker
        // and harder when it works, fast to hover, held still on the perch but for a
        // stretch now and then, and riding.
        if l.glide > 0.0 && (!flying || effort > 0.3 || flare > 0.2) {
            l.glide = 0.0;
        }
        let stretch = perched && hash(i ^ 0x51, (t / 7.0) as u32) < 0.3 && (t / 7.0).fract() < 0.2;
        let (flap, rate) = if hovering {
            (1.0, cruise * 1.5)
        } else if flying && l.glide <= 0.0 {
            let work = effort.max(flare);
            (0.65 + 0.35 * work, cruise * (0.8 + 0.06 * level) * (1.0 + 0.5 * work))
        } else if stretch {
            (0.6, cruise * 0.7)
        } else {
            (0.0, l.rate)
        };
        l.flap += (flap - l.flap) * ease(8.0);
        l.rate += (rate - l.rate) * ease(4.0);
        if l.glide > 0.0 {
            l.glide -= dt;
        } else {
            l.phase += l.rate * dt;
            if l.phase >= 1.0 {
                l.phase = l.phase.fract();
                // A beat done: flying steadily, it glides after a few.
                if flying && effort < 0.15 && flare < 0.1 && level > speed * 0.7 {
                    if l.beats == 0 {
                        l.glide = short + (long - short) * hash(i, l.glides);
                        l.glides += 1;
                        l.beats = 2 + (hash(i ^ 0xb7, l.glides) * 3.0) as u32;
                        l.phase = 0.0;
                    } else {
                        l.beats -= 1;
                    }
                }
            }
        }
        let (raise, wrist) = if riding {
            (0.75, 0.2)
        } else if perched {
            (0.3, -0.35)
        } else if hovering {
            (0.25, 0.0)
        } else {
            (0.12, 0.06)
        };
        l.raise += (raise - l.raise) * ease(6.0);
        l.wrist += (wrist - l.wrist) * ease(6.0);
        let size = self.role.fold().1 * if riding { RIDING } else { 1.0 };
        l.size += (size - l.size) * ease(3.0);
        let settled = if perched { 1.0 } else { 0.0 };
        l.settled += (settled - l.settled) * ease(6.0);
    }
}

/// How a familiar's bird is drawn, eased from frame to frame toward what it is doing:
/// for drawing, and the sound of its wings, only (nothing it does depends on it).
#[derive(Clone, Copy)]
pub(super) struct Look {
    /// Where its wings are in their beat (0 to 1 from the top of the stroke, the
    /// downstroke first), beats a second, and how hard they beat (0 held still).
    phase: f32,
    rate: f32,
    flap: f32,
    /// Seconds left of the glide it is in, beats to go before it may glide again, and
    /// glides so far (which pick the next one's length).
    glide: f32,
    beats: u32,
    glides: u32,
    /// Which way it faces (radians from -z toward +x), how far it banks (the right
    /// wing down negative), and how far its nose is up.
    heading: f32,
    bank: f32,
    pitch: f32,
    /// Its wings at rest (raised at the root, and at the wrist), its size, and how far
    /// it has settled into its perch's tray (0 to 1).
    raise: f32,
    wrist: f32,
    size: f32,
    settled: f32,
    /// Its velocity a frame ago.
    was: Vec3,
}

impl Look {
    /// Its beat as heard: where the wings are in it, how fast that moves on (not at
    /// all while it glides, the wings held), and how hard they beat.
    pub(super) fn beat(&self) -> (f32, f32, f32) {
        let rate = if self.glide > 0.0 { 0.0 } else { self.rate };
        (self.phase, rate, self.flap)
    }

    /// Riding, wings raised, facing `heading`.
    fn new(role: Role, heading: f32) -> Self {
        Look {
            phase: 0.0,
            rate: role.wingbeat().0,
            flap: 0.0,
            glide: 0.0,
            beats: 3,
            glides: 0,
            heading,
            bank: 0.0,
            pitch: 0.0,
            raise: 0.75,
            wrist: 0.2,
            size: role.fold().1 * RIDING,
            settled: 0.0,
            was: Vec3::ZERO,
        }
    }
}

/// Where wings are in a beat at `phase` (0 to 1 from the top): from 1 at the top down
/// to -1 at the bottom over the downstroke, and back up.
fn stroke(phase: f32) -> f32 {
    let p = phase.rem_euclid(1.0);
    if p < DOWNSTROKE {
        (PI * p / DOWNSTROKE).cos()
    } else {
        -(PI * (p - DOWNSTROKE) / (1.0 - DOWNSTROKE)).cos()
    }
}

/// A number in 0..1 picked by `a` and `b`, the same each time: familiars' looks take
/// nothing from the game's random numbers, which their work draws on.
fn hash(a: usize, b: u32) -> f32 {
    let mut x = (a as u32).wrapping_mul(0x9E37_79B9) ^ b.wrapping_mul(0x85EB_CA6B);
    x ^= x >> 15;
    x = x.wrapping_mul(0x2C1B_3C6D);
    x ^= x >> 12;
    x = x.wrapping_mul(0x297A_2D39);
    x ^= x >> 15;
    (x >> 8) as f32 / (1u32 << 24) as f32
}

/// An angle brought into -π..π.
fn wrap(a: f32) -> f32 {
    (a + PI).rem_euclid(TAU) - PI
}

impl Game {
    /// Each familiar's role, and whether it is posted.
    pub fn familiars(&self) -> impl Iterator<Item = (Role, bool)> + '_ {
        self.familiars.iter().map(|f| (f.role, f.post.is_some()))
    }

    /// A new familiar of `role`, riding on the shoulder.
    pub(super) fn spawn_familiar(&mut self, role: Role) {
        let p = self.player.eye() + self.player.right() * 0.5;
        self.familiars.push(Familiar {
            role,
            post: None,
            pos: p,
            vel: Vec3::ZERO,
            task: Task::Idle,
            carry: Vec::new(),
            path: Vec::new(),
            phase: self.rng.gen_range(0.0..10.0f32),
            rest: 0.5,
            riding_since: self.time,
            idle: 0.0,
            look: Look::new(role, self.player.yaw),
            #[cfg(test)]
            delivered: 0,
        });
    }

    /// G: take back the familiar aimed at, or else post the one riding longest at the
    /// floor aimed at.
    pub(super) fn post_or_recall(&mut self, audio: &mut Audio) {
        if let Aim::Familiar(i) = self.aim {
            self.recall(i);
            audio.play_pitched(Sfx::Fold, 0.4, self.familiars[i].voice(i));
            return;
        }
        let riding = (0..self.familiars.len())
            .filter(|&i| self.familiars[i].post.is_none())
            .min_by(|&a, &b| {
                self.familiars[a]
                    .riding_since
                    .total_cmp(&self.familiars[b].riding_since)
            });
        let Some(i) = riding else {
            let msg = if self.familiars.is_empty() {
                "Familiars are learned in the Grimoire (Tab)."
            } else {
                "No familiar is riding with you — G at a perch takes one back."
            };
            self.hint = Some((msg.into(), 2.5));
            return;
        };
        let Some(at) = self.aimed_floor() else {
            self.hint = Some(("Aim at open floor nearby to post a familiar.".into(), 2.0));
            audio.play(Sfx::Nope, 0.3);
            return;
        };
        self.post_familiar(i, at);
        audio.play_pitched(Sfx::Unfold, 0.3, self.familiars[i].voice(i));
    }

    /// Post familiar `i` at `at` (a free spot on a floor).
    pub(super) fn post_familiar(&mut self, i: usize, at: Vec3) {
        let mut field = self.nav.field();
        self.nav.fill(&mut field, at);
        let level = self.lib.world.level_at(at);
        let f = &mut self.familiars[i];
        f.post = Some(Post { at, level, field });
        f.rest = 0.0;
        f.idle = 0.0;
        self.set_course(i, Task::Home);
    }

    /// Familiar `i` leaves its post for the shoulder, setting down what it carries.
    pub(super) fn recall(&mut self, i: usize) {
        let carry = std::mem::take(&mut self.familiars[i].carry);
        let from = self.familiars[i].pos;
        self.set_down(&carry, from);
        let f = &mut self.familiars[i];
        f.post = None;
        f.task = Task::Idle;
        f.path.clear();
        f.riding_since = self.time;
    }

    /// Familiar `i` has had nothing to do for a while: it folds up at its post and
    /// unfolds on the shoulder, riding until it is posted again.
    fn fold_home(&mut self, i: usize) {
        let (role, at) = (self.familiars[i].role, self.familiars[i].pos);
        self.burst(at, 14, role.light(), 0.8);
        self.recall(i);
        self.familiars[i].pos = self.player.eye() + self.player.right() * 0.5;
        let msg = format!(
            "A {} found nothing to do and came back to you (G posts it).",
            role.name()
        );
        self.hint = Some((msg, 3.0));
    }

    /// The floor where the crosshair points, within reach, as a free spot a familiar
    /// can be posted at (one a walk away, past no closed gate).
    fn aimed_floor(&self) -> Option<Vec3> {
        let o = self.player.eye();
        let d = self.player.forward();
        let t = match self.aim {
            Aim::Loose { dist, .. } => dist,
            _ => self.lib.world.ray(o, d, 12.0, Stops::SIGHT),
        };
        if !t.is_finite() || t > 12.0 {
            return None;
        }
        let p = o + d * t;
        let floor = self.lib.world.ground(Vec2::new(p.x, p.z), p.y + 0.05)?;
        let at = self.nav.standable_near(Vec3::new(p.x, floor, p.z));
        (self.nav.open_at(at) && at.distance(Vec3::new(p.x, floor, p.z)) < 2.0).then_some(at)
    }

    /// The familiar the crosshair points at (its crane or its perch), within reach.
    pub(super) fn aimed_familiar(&self, o: Vec3, d: Vec3, max_t: f32) -> Option<(usize, f32)> {
        let mut best: Option<(usize, f32)> = None;
        for (i, f) in self.familiars.iter().enumerate() {
            let Some(post) = &f.post else { continue };
            // The crane, and three points up the perch.
            let perch = [0.3, 0.75, 1.2].map(|h| (post.at + Vec3::Y * h, 0.25));
            for (p, r) in std::iter::once((f.pos, 0.35)).chain(perch) {
                let t = (p - o).dot(d);
                if t > 0.0 && t < max_t && (o + d * t).distance(p) < r && best.is_none_or(|(_, bt)| t < bt) {
                    best = Some((i, t));
                }
            }
        }
        best
    }

    /// Books leave a familiar's grasp at `from` and drop as a little loose pile.
    fn set_down(&mut self, books: &[u32], from: Vec3) {
        for (k, &b) in books.iter().enumerate() {
            self.state[b as usize] = BookState::Loose;
            let mut j = || self.rng.gen_range(-1.0..1.0f32);
            let vel = Vec3::new(j(), 0.0, j()) * 0.6;
            let spin = Vec3::new(j(), j(), j()) * 3.0;
            self.loose
                .throw(b, from, vel, self.rot[b as usize], spin, k as f32 * 0.05);
        }
    }

    /// The way from `from` to `to` over walking `field` (a familiar's post's, or the
    /// hub's): back along the field toward its source until the way to `to` branches
    /// off, then out to `to`, at flying height and straightened where the line is
    /// clear. Points in reverse (the next last), ending at `to` itself; `None` if the
    /// field doesn't reach one of the ends.
    pub(super) fn plan(&self, field: &Field, from: Vec3, to: Vec3) -> Option<Vec<Vec3>> {
        let key = |p: Vec3| {
            (
                (p.x * 4.0).round() as i32,
                (p.y * 2.0).round() as i32,
                (p.z * 4.0).round() as i32,
            )
        };
        let floor_of = |p: Vec3| self.lib.world.ground(Vec2::new(p.x, p.z), p.y + 0.3).unwrap_or(p.y);
        let back = (self.nav).way_back(field, Vec3::new(from.x, floor_of(from), from.z))?;
        let out = (self.nav).way_back(field, Vec3::new(to.x, floor_of(to), to.z))?;
        // Both ways end at the field's source cell, so they meet.
        let on_out: HashMap<_, usize> = out.iter().enumerate().map(|(k, &p)| (key(p), k)).collect();
        let mut way = vec![from];
        for p in back {
            if let Some(&k) = on_out.get(&key(p)) {
                way.extend(out[..=k].iter().rev().map(|&q| q + Vec3::Y * FLY_H));
                break;
            }
            way.push(p + Vec3::Y * FLY_H);
        }
        way.push(to);
        // Straighten: from each point kept, skip ahead while the line stays clear.
        let mut kept = Vec::new();
        let mut k = 0;
        while k + 1 < way.len() {
            let mut j = k + 1;
            while j + 1 < way.len() && self.flight_clear(way[k], way[j + 1]) {
                j += 1;
            }
            kept.push(way[j]);
            k = j;
        }
        kept.reverse();
        Some(kept)
    }

    /// Nothing a crane would hit lies between `a` and `b`: nothing that stops sight
    /// (walls, cases) or bodies (railings, chairs).
    fn flight_clear(&self, a: Vec3, b: Vec3) -> bool {
        let w = &self.lib.world;
        w.clear(a, b, Stops::SIGHT) && w.clear(a, b, Stops::BODIES)
    }

    /// Where a courier at `post` would take book `b`: the post of a shelver that can
    /// file it, else its room's drop spot; `None` if its department is still veiled,
    /// the way there is closed, or the book is already there (by that shelver, or
    /// anywhere in its room).
    fn courier_goal(&self, post: &Post, b: u32) -> Option<Vec3> {
        let dept = self.lib.dept_of(self.lib.category_of_book(b));
        if !self.dept_open(dept) {
            return None;
        }
        let file_reach = WINGS[self.level(Upg::Wings) as usize].3;
        let approach = self.approach(b);
        let shelver = (self.familiars.iter())
            .filter(|f| f.role == Role::Shelver)
            .filter_map(|f| f.post.as_ref())
            .map(|p| (self.nav.distance(&p.field, p.at, approach), p.at))
            .filter(|&(d, _)| d <= file_reach)
            .min_by(|a, b| a.0.total_cmp(&b.0));
        let at = self.loose.pose(b).0;
        let (goal, there) = match shelver {
            Some((_, post)) => {
                let near = Vec2::new(post.x - at.x, post.z - at.z).length() <= SETTLED && (post.y - at.y).abs() < 1.0;
                (post, near)
            }
            None => (self.lib.rooms[dept].drop, self.lib.room_at(at) == Some(dept)),
        };
        let open = self.nav.distance(&post.field, post.at, goal).is_finite();
        (!there && open).then_some(goal)
    }

    /// Whether a shelver at `post` can file book `b`: its department is open and its
    /// slot a short walk from the post.
    fn shelver_files(&self, post: &Post, b: u32) -> bool {
        let file_reach = WINGS[self.level(Upg::Wings) as usize].3;
        self.dept_open(self.lib.dept_of(self.lib.category_of_book(b)))
            && self.nav.distance(&post.field, post.at, self.approach(b)) <= file_reach
    }

    /// Work for familiar `i` at its post: the books nearest the post it can take.
    fn find_work(&mut self, i: usize) -> Option<Task> {
        let reach = WINGS[self.level(Upg::Wings) as usize].2;
        let post = self.familiars[i].post.as_ref()?;
        let (lv, at) = (post.level, Vec2::new(post.at.x, post.at.z));
        let mut near = self.loose.near(lv, at, reach, |_| true);
        near.sort_by(|&a, &b| {
            let d = |x: u32| self.loose.pose(x).0.distance_squared(post.at);
            d(a).total_cmp(&d(b))
        });
        let post = self.familiars[i].post.as_ref()?;
        for b in near {
            let p = self.loose.pose(b).0;
            match self.familiars[i].role {
                Role::Shelver if self.shelver_files(post, b) => return Some(Task::Fetch { at: p, goal: p }),
                Role::Courier => {
                    if let Some(goal) = self.courier_goal(post, b) {
                        return Some(Task::Fetch { at: p, goal });
                    }
                }
                Role::Shelver => {}
            }
        }
        None
    }

    /// On arriving at `at`: take what it came for from the books there.
    fn take_up(&mut self, i: usize, at: Vec3, goal: Vec3) {
        let (_, cap, ..) = WINGS[self.level(Upg::Wings) as usize];
        let Some(post) = self.familiars[i].post.as_ref() else {
            return;
        };
        let (lv, spot) = (post.level, Vec2::new(at.x, at.z));
        let wanted: Vec<u32> = (self.loose.near(lv, spot, GRASP, |_| true).into_iter())
            .filter(|&b| match self.familiars[i].role {
                Role::Shelver => self.shelver_files(post, b),
                Role::Courier => self.courier_goal(post, b) == Some(goal),
            })
            .collect();
        let books = self.loose.gather(lv, spot, GRASP, cap, |b| wanted.contains(&b));
        for &b in &books {
            self.state[b as usize] = BookState::Carried(i as u8);
        }
        let f = &mut self.familiars[i];
        f.carry = books;
        // Visit slots in shelf order.
        f.carry.sort();
    }

    pub(super) fn update_familiars(&mut self, dt: f32) {
        let (speed, ..) = WINGS[self.level(Upg::Wings) as usize];
        // A gate may have opened since a post's field was filled.
        for f in &mut self.familiars {
            if let Some(post) = &mut f.post {
                self.nav.fill(&mut post.field, post.at);
            }
        }
        let eye = self.player.eye();
        let (right, fwd) = (self.player.right(), self.player.forward());
        let mut riders = 0;
        for i in 0..self.familiars.len() {
            self.familiars[i].phase += dt * 9.0;
            if self.familiars[i].post.is_none() {
                // Riding: bob along beside the shoulder, out of the way of the view.
                let f = &mut self.familiars[i];
                let side = if riders % 2 == 0 { 1.0 } else { -1.0 };
                let slot = eye + right * side * (0.55 + 0.2 * (riders / 2) as f32) - fwd * 0.25 + Vec3::Y * 0.12;
                let bob = Vec3::Y * (f.phase * 0.25).sin() * 0.04;
                f.pos = f.pos.lerp(slot + bob, 1.0 - (-dt * 8.0).exp());
                f.vel = Vec3::ZERO;
                riders += 1;
                continue;
            }
            if self.familiars[i].rest > 0.0 {
                self.familiars[i].rest -= dt;
                if self.familiars[i].task == Task::Idle && self.familiars[i].rest <= 0.0 {
                    match self.find_work(i) {
                        Some(task) => {
                            self.familiars[i].idle = 0.0;
                            self.set_course(i, task);
                        }
                        None => {
                            self.familiars[i].rest = 1.0;
                            self.familiars[i].idle += 1.0;
                            if self.familiars[i].idle >= IDLE_HOME {
                                self.fold_home(i);
                            }
                        }
                    }
                }
                self.carry_dangle(i);
                continue;
            }
            // Fly along the path. A point is passed once reached, or a little before
            // when the way on to the next is clear (so corners are cut only in the open).
            let world = &self.lib.world;
            let f = &mut self.familiars[i];
            if let Some(&next) = f.path.last() {
                let to = next - f.pos;
                let last = f.path.len() == 1;
                let slow = if last { (to.length() / 0.8).min(1.0) } else { 1.0 };
                let desired = to.normalize_or_zero() * speed * slow.max(0.15);
                f.vel = f.vel.lerp(desired, 1.0 - (-dt * 10.0).exp());
                f.pos += f.vel * dt;
                let d = f.pos.distance(next);
                let after = f.path.len().checked_sub(2).map(|k| f.path[k]);
                let clear = |a: Vec3| world.clear(f.pos, a, Stops::SIGHT) && world.clear(f.pos, a, Stops::BODIES);
                if d < ARRIVE || (d < 0.6 && after.is_some_and(clear)) {
                    f.path.pop();
                }
                self.carry_dangle(i);
                continue;
            }
            self.arrive(i);
            self.carry_dangle(i);
        }
        self.animate_familiars(dt);
    }

    /// Ease how each familiar is drawn toward what it is doing (drawing only).
    fn animate_familiars(&mut self, dt: f32) {
        let (speed, ..) = WINGS[self.level(Upg::Wings) as usize];
        let player = (self.player.eye(), self.player.yaw);
        for (i, f) in self.familiars.iter_mut().enumerate() {
            f.animate(i, dt, speed, player, self.time);
        }
    }

    /// Set familiar `i` off on `task`, planning its path.
    fn set_course(&mut self, i: usize, task: Task) {
        let Some(post) = self.familiars[i].post.as_ref() else {
            return;
        };
        let from = self.familiars[i].pos;
        let to = match task {
            Task::Fetch { at, .. } => at + Vec3::Y * 0.45,
            Task::Deliver(at) => at + Vec3::Y * 1.0,
            Task::File => {
                let b = self.familiars[i].carry[0] as usize;
                let f = &self.lib.faces[self.lib.books[b].face as usize];
                self.lib.books[b].slot_pos + f.normal * 0.55 + Vec3::Y * 0.1
            }
            Task::Home => post.at + Vec3::Y * PERCH_H,
            Task::Idle => from,
        };
        match self.plan(&post.field, from, to) {
            Some(path) => {
                let f = &mut self.familiars[i];
                f.task = task;
                f.path = path;
            }
            // Somewhere its post's walks don't reach: give up and go home, or if even
            // home is out of reach, fold up and reappear on the perch.
            None if task == Task::Home => self.familiars[i].perch(),
            None => {
                let carry = std::mem::take(&mut self.familiars[i].carry);
                self.set_down(&carry, from);
                self.set_course(i, Task::Home);
            }
        }
    }

    /// Familiar `i` has reached the end of its path: do what it came to do. It stops
    /// there, so it sets off on the next leg from still (carried on, its speed would
    /// swing it off a line that may pass close by a case).
    fn arrive(&mut self, i: usize) {
        self.familiars[i].vel = Vec3::ZERO;
        match self.familiars[i].task {
            Task::Fetch { at, goal } => {
                self.take_up(i, at, goal);
                let f = &self.familiars[i];
                let next = match (f.carry.is_empty(), f.role) {
                    (true, _) => Task::Home,
                    (false, Role::Shelver) => Task::File,
                    (false, Role::Courier) => Task::Deliver(goal),
                };
                self.familiars[i].rest = 0.2;
                self.set_course(i, next);
            }
            Task::Deliver(at) => {
                let carry = std::mem::take(&mut self.familiars[i].carry);
                #[cfg(test)]
                {
                    self.familiars[i].delivered += carry.len() as u32;
                }
                // Beside the goal, so a pile there grows rather than buries the perch.
                let jitter = Vec3::new(self.rng.gen_range(-0.4..0.4), 0.0, self.rng.gen_range(-0.4..0.4));
                self.set_down(&carry, at + Vec3::Y * 0.9 + jitter);
                self.familiars[i].rest = 0.3;
                self.set_course(i, Task::Home);
            }
            Task::File => {
                let b = self.familiars[i].carry.remove(0);
                self.state[b as usize] = BookState::Filing;
                self.start_flight(b, Dest::Shelf, 0.35, 0.0);
                self.familiars[i].rest = 0.15;
                let next = if self.familiars[i].carry.is_empty() {
                    Task::Home
                } else {
                    Task::File
                };
                self.set_course(i, next);
            }
            Task::Home => {
                let f = &mut self.familiars[i];
                f.task = Task::Idle;
                f.vel = Vec3::ZERO;
                f.rest = 0.4;
            }
            Task::Idle => self.familiars[i].rest = 0.5,
        }
    }

    /// Carried books dangle beneath familiar `i`.
    fn carry_dangle(&mut self, i: usize) {
        let f = &self.familiars[i];
        for (k, &b) in f.carry.iter().enumerate() {
            let sway = (f.phase * 0.5 + k as f32).sin() * 0.1;
            self.pos[b as usize] = f.pos - Vec3::Y * (0.18 + k as f32 * 0.05);
            self.rot[b as usize] =
                Quat::from_rotation_y(f.phase * 0.05 + k as f32) * Quat::from_rotation_z(PI * 0.5 + sway);
        }
    }

    /// The familiars (folded-paper birds with a soft glow inside, in their role's
    /// colour) and the perches they are posted at.
    pub(super) fn draw_familiars(&self, out: &mut Vec<Inst>) {
        let mut nth = [0; 2];
        for f in &self.familiars {
            let l = &f.look;
            let (fold, _) = f.role.fold();
            let c = f.role.light();
            let mut at = f.pos;
            if let Some(post) = &f.post {
                self.draw_perch(out, post.at, c);
                at = at.lerp(post.at + Vec3::Y * (TRAY + fold.keel() * l.size), l.settled);
            }
            // The body lifts as the wings beat down.
            at.y -= l.flap * l.size * 0.012 * stroke(l.phase - 0.15);
            let s = stroke(l.phase);
            let papers = f.role.papers();
            let k = &mut nth[f.role as usize];
            let bird = Crane {
                fold,
                at,
                rot: Quat::from_rotation_y(-l.heading) * Quat::from_rotation_x(l.pitch) * Quat::from_rotation_z(l.bank),
                size: l.size,
                // The outer half of each wing lags the inner a little, so the wing
                // bends through the beat.
                raise: l.raise + l.flap * (0.12 + 0.6 * s),
                wrist: l.wrist + l.flap * 0.45 * (stroke(l.phase - 0.1) - s),
                paper: papers[*k % papers.len()],
                glow: [c[0], c[1], c[2], 0.15],
            };
            *k += 1;
            out.extend(bird.parts());
        }
    }

    /// A post's perch: a brass stand on a stepped foot with a tray on top for its bird
    /// to sit in, and a gem of its role's colour turning slowly over it.
    fn draw_perch(&self, out: &mut Vec<Inst>, at: Vec3, light: [f32; 3]) {
        let brass = [0.72, 0.5, 0.2, 1.0];
        let mut part = |p: Vec3, size: Vec3, turn: f32| {
            out.push(Inst::new(
                at + p,
                Quat::from_rotation_y(turn),
                size,
                kind::METAL,
                brass,
                0,
            ));
        };
        part(Vec3::Y * 0.015, Vec3::new(0.26, 0.03, 0.26), 0.0);
        part(Vec3::Y * 0.045, Vec3::new(0.17, 0.03, 0.17), FRAC_PI_4);
        part(Vec3::Y * (TRAY + 0.06) * 0.5, Vec3::new(0.026, TRAY - 0.06, 0.026), 0.0);
        for y in [0.33, TRAY - 0.04] {
            part(Vec3::Y * y, Vec3::new(0.05, 0.035, 0.05), FRAC_PI_4);
        }
        // The tray, and its rim.
        part(Vec3::Y * (TRAY - 0.008), Vec3::new(0.17, 0.016, 0.17), 0.0);
        for (dx, dz) in [(1.0, 0.0), (-1.0, 0.0), (0.0, 1.0), (0.0, -1.0)] {
            let size = if dx == 0.0 {
                Vec3::new(0.172, 0.03, 0.012)
            } else {
                Vec3::new(0.012, 0.03, 0.148)
            };
            part(Vec3::new(dx * 0.08, TRAY + 0.007, dz * 0.08), size, 0.0);
        }
        let t = self.time;
        let spin = Quat::from_rotation_y(t * 0.8) * Quat::from_rotation_x(FRAC_PI_4) * Quat::from_rotation_z(FRAC_PI_4);
        out.push(Inst::new(
            at + Vec3::Y * (TRAY + 0.36 + 0.02 * (t * 1.3).sin()),
            spin,
            Vec3::splat(0.035),
            kind::EMISSIVE,
            [light[0], light[1], light[2], 0.8],
            0,
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::SLAB;

    /// How far past a corner a crane's centre may brush.
    const BRUSH: f32 = 0.03;

    /// Run the game headless for `secs`, checking every frame that no posted familiar
    /// is inside a wall, a case or a floor (more than `BRUSH`: momentum may carry a
    /// crane's centre a little past a corner its path clears). Riding ones glide to
    /// the shoulder in a straight line; G takes one back only in plain sight.
    fn run(g: &mut Game, secs: f32) {
        let mut audio = Audio::silent();
        let walls: Vec<(Vec3, Vec3)> = (g.lib.world.solids().iter())
            .filter(|s| s.stops.has(Stops::SIGHT) && s.stops.has(Stops::BODIES))
            .map(|s| (s.min, s.max))
            .collect();
        let floors: Vec<(Vec3, Vec3)> = (g.lib.world.slabs.iter())
            .map(|s| {
                let y = g.lib.world.levels[s.level].y;
                (Vec3::new(s.min.x, y - SLAB, s.min.y), Vec3::new(s.max.x, y, s.max.y))
            })
            .collect();
        for frame in 0..(secs * 60.0) as usize {
            g.update(1.0 / 60.0, &Input::default(), &mut audio, false);
            for (i, f) in g.familiars.iter().enumerate().filter(|(_, f)| f.post.is_some()) {
                let inside = |&(mn, mx): &(Vec3, Vec3)| {
                    let (mn, mx) = (mn + Vec3::splat(BRUSH), mx - Vec3::splat(BRUSH));
                    f.pos.cmpgt(mn).all() && f.pos.cmplt(mx).all()
                };
                assert!(
                    !walls.iter().any(inside) && !floors.iter().any(inside),
                    "frame {frame}: familiar {i} inside something at {}, flying {:?}",
                    f.pos,
                    f.path
                );
            }
        }
    }

    /// Take every resting book within 6 m of `at` but those in `keep` off the floor
    /// (marked shelved), so a familiar there works on just those.
    fn clear_round(g: &mut Game, at: Vec3, keep: &[u32]) {
        let others: Vec<u32> = (0..g.lib.books.len() as u32)
            .filter(|&b| g.loose.is_resting(b) && g.loose.pose(b).0.distance(at) < 6.0 && !keep.contains(&b))
            .collect();
        g.loose.take(&others);
        for &b in &others {
            g.state[b as usize] = BookState::Shelved;
        }
    }

    /// Toss `books` (taken from wherever they lie) to land round `at`.
    fn drop_round(g: &mut Game, books: &[u32], at: Vec3) {
        g.loose.take(books);
        for (k, &b) in books.iter().enumerate() {
            let a = k as f32 * 2.4;
            let p = at + Vec3::new(a.cos(), 0.0, a.sin()) * 0.5 + Vec3::Y * 0.8;
            g.loose.throw(b, p, Vec3::ZERO, Quat::IDENTITY, Vec3::ZERO, 0.0);
        }
    }

    /// A shelver posted at a category's cases files the books of it lying near its
    /// post, and leaves alone those whose shelves are far away.
    #[test]
    fn a_shelver_files_what_lies_near_it() {
        // At the slowest wings and the fastest.
        for wings in [0, 2] {
            let mut g = Game::new(Layout::Hall);
            g.upg[upg_index(Upg::Wings)] = wings;
            let cat = 0;
            let face = (0..g.lib.faces.len()).find(|&f| g.lib.faces[f].category == cat && g.lib.faces[f].case_idx == 1);
            let face = &g.lib.faces[face.unwrap()];
            let post = g.nav.standable_near(face.center + face.normal * 1.2);
            let ours: Vec<u32> = (0..g.lib.books.len() as u32)
                .filter(|&b| g.lib.category_of_book(b) == cat && g.loose.is_resting(b))
                .take(8)
                .collect();
            // The East Wing's books are far from these cases (and veiled).
            let far: Vec<u32> = (0..g.lib.books.len() as u32)
                .filter(|&b| g.lib.dept_of(g.lib.category_of_book(b)) == 1 && g.loose.is_resting(b))
                .take(4)
                .collect();
            drop_round(&mut g, &[ours.clone(), far.clone()].concat(), post);
            g.spawn_familiar(Role::Shelver);
            g.post_familiar(0, post);
            run(&mut g, 40.0);
            let filed = ours
                .iter()
                .filter(|&&b| g.state[b as usize] == BookState::Shelved)
                .count();
            assert_eq!(filed, ours.len(), "{filed} of {} filed", ours.len());
            assert!(far.iter().all(|&b| g.state[b as usize] == BookState::Loose));
        }
    }

    /// A courier posted at the great heap carries the Observatory's books through its
    /// doorway into its room, and those of a category with a shelver to the shelver,
    /// who files them; veiled departments' books stay where they are.
    #[test]
    fn a_courier_carries_books_to_their_room() {
        // At the slowest wings and the fastest.
        for wings in [0, 2] {
            let mut g = Game::new(Layout::Great);
            g.upg[upg_index(Upg::Wings)] = wings;
            let heap = g.nav.standable_near(Vec3::new(0.0, 0.0, 3.2));
            let room = &g.lib.rooms[0];
            let drop = room.drop;
            let n = g.lib.books.len() as u32;
            let dept = |g: &Game, b: u32| g.lib.dept_of(g.lib.category_of_book(b));
            // A shelver at the cases of the room's first category.
            let cat = g.lib.catalog.departments[0].categories.start;
            let face = (0..g.lib.faces.len()).find(|&f| g.lib.faces[f].category == cat && g.lib.faces[f].case_idx == 2);
            let face = &g.lib.faces[face.unwrap()];
            let perch = g.nav.standable_near(face.center + face.normal * 1.2);
            g.spawn_familiar(Role::Shelver);
            g.post_familiar(0, perch);
            let to_shelver: Vec<u32> = (0..n).filter(|&b| g.lib.category_of_book(b) == cat).take(4).collect();
            // Books of the room the shelver can't file (beyond its reach on foot).
            let shelver = g.familiars[0].post.as_ref().unwrap();
            let to_room: Vec<u32> = (0..n)
                .filter(|&b| dept(&g, b) == 0 && !g.shelver_files(shelver, b))
                .take(4)
                .collect();
            let veiled: Vec<u32> = (0..n).filter(|&b| dept(&g, b) == 5).take(4).collect();
            let near_heap = |g: &Game, b: u32| g.loose.pose(b).0.distance(heap) < 4.0;
            drop_round(
                &mut g,
                &[to_shelver.clone(), to_room.clone(), veiled.clone()].concat(),
                heap,
            );
            // Clear everything else from round the post, so the courier has just these.
            clear_round(
                &mut g,
                heap,
                &[to_shelver.clone(), to_room.clone(), veiled.clone()].concat(),
            );
            g.spawn_familiar(Role::Courier);
            g.post_familiar(1, heap);
            run(&mut g, 180.0);
            let shelved = to_shelver
                .iter()
                .filter(|&&b| g.state[b as usize] == BookState::Shelved)
                .count();
            assert_eq!(shelved, to_shelver.len(), "the shelver filed {shelved}");
            for &b in &to_room {
                let p = g.loose.pose(b).0;
                assert!(
                    g.loose.is_resting(b) && p.distance(drop) < 2.5,
                    "book {b} at {p}, the room's drop at {drop}"
                );
            }
            assert!(veiled.iter().all(|&b| near_heap(&g, b)));
        }
    }

    /// A shelver posted at the end case of a row files the books whose slots are on
    /// the back of that case, flying round the end of the row, not through it
    /// (regression: the way out joined from the near side of the case).
    #[test]
    fn a_shelver_flies_round_to_the_back_of_its_case() {
        // At the slowest wings and the fastest.
        for (layout, wings) in Layout::ALL.into_iter().flat_map(|l| [(l, 0), (l, 2)]) {
            let mut g = Game::new(layout);
            g.upg[upg_index(Upg::Wings)] = wings;
            g.open_all_gates();
            let nf = g.lib.faces.len();
            let mut tried = 0;
            let ends: Vec<usize> = (0..nf).filter(|&f| g.lib.faces[f].case_idx == 0).take(4).collect();
            for fi in ends {
                let (fc, fnrm) = (g.lib.faces[fi].center, g.lib.faces[fi].normal);
                let back = (0..nf).find(|&k| {
                    let b = &g.lib.faces[k];
                    b.normal.dot(fnrm) < -0.9 && (b.center - fc).length() < 1.5
                });
                let Some(back) = back else { continue };
                let post = g.nav.standable_near(fc + fnrm * 1.2);
                let books: Vec<u32> = (0..g.lib.books.len() as u32)
                    .filter(|&b| g.lib.books[b as usize].face as usize == back && g.loose.is_resting(b))
                    .step_by(7)
                    .take(4)
                    .collect();
                drop_round(&mut g, &books, post);
                clear_round(&mut g, post, &books);
                let i = g.familiars.len();
                g.spawn_familiar(Role::Shelver);
                g.post_familiar(i, post);
                run(&mut g, 30.0);
                let filed = books
                    .iter()
                    .filter(|&&b| g.state[b as usize] == BookState::Shelved)
                    .count();
                assert_eq!(filed, books.len(), "{layout:?} face {fi} wings {wings}");
                g.recall(i);
                tried += 1;
            }
            assert!(tried >= 2, "{layout:?}: {tried} faces tried");
        }
    }

    /// A courier posted a little way from its room's drop leaves the pile there alone
    /// (regression: it carried books it had just set down back and forth).
    #[test]
    fn a_courier_leaves_books_already_home() {
        let mut g = Game::new(Layout::Great);
        let drop = g.lib.rooms[0].drop;
        let post = g.nav.standable_near(drop + Vec3::new(0.0, 0.0, -3.2));
        let books: Vec<u32> = (0..g.lib.books.len() as u32)
            .filter(|&b| g.lib.dept_of(g.lib.category_of_book(b)) == 0)
            .step_by(13)
            .take(40)
            .collect();
        drop_round(&mut g, &books, drop);
        g.spawn_familiar(Role::Courier);
        g.post_familiar(0, post);
        let mut carried = 0;
        for _ in 0..60 {
            run(&mut g, 1.0);
            carried += g.familiars[0].carry.len();
        }
        assert_eq!(carried, 0);
    }

    /// G at the floor posts the familiar riding longest; G at it takes it back.
    #[test]
    fn g_posts_and_takes_back() {
        let mut g = Game::new(Layout::Hall);
        let mut audio = Audio::silent();
        g.spawn_familiar(Role::Shelver);
        g.spawn_familiar(Role::Courier);
        g.player.pos = Vec3::new(0.0, 0.0, 6.5);
        g.player.yaw = 0.0;
        g.player.pitch = -0.5;
        let g_key = Input {
            post: true,
            ..Input::default()
        };
        g.update(1.0 / 60.0, &g_key, &mut audio, true);
        let at = g.familiars[0].post.as_ref().expect("posted").at;
        assert!(at.distance(Vec3::new(0.0, 0.0, 3.5)) < 1.0, "posted at {at}");
        assert!(g.familiars[1].post.is_none());
        // Let it settle on its perch, then look at it and take it back.
        run(&mut g, 3.0);
        g.player.pitch = ((g.familiars[0].pos.y - g.player.eye().y) / at.distance(g.player.pos)).atan();
        g.update(1.0 / 60.0, &Input::default(), &mut audio, true);
        assert_eq!(g.aim, Aim::Familiar(0));
        g.update(1.0 / 60.0, &g_key, &mut audio, true);
        assert!(g.familiars[0].post.is_none());
    }

    /// A familiar with nothing to do round its post comes back to the shoulder after
    /// two minutes, and can be posted again.
    #[test]
    fn an_idle_familiar_comes_back() {
        let mut g = Game::new(Layout::Hall);
        g.spawn_familiar(Role::Courier);
        // The reading room's middle is kept clear of piles.
        let at = g.nav.standable_near(Vec3::new(0.0, 0.0, 3.5));
        clear_round(&mut g, at, &[]);
        g.post_familiar(0, at);
        run(&mut g, IDLE_HOME - 5.0);
        assert!(g.familiars[0].post.is_some(), "still posted before two minutes");
        run(&mut g, 10.0);
        assert!(g.familiars[0].post.is_none(), "back on the shoulder");
        g.post_familiar(0, at);
        assert_eq!(g.familiars[0].idle, 0.0);
    }
}
