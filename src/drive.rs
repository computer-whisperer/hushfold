//! A stand-in player for scripted runs. It walks and looks through the inputs a
//! person's hands give (the view turned by so much a frame, as a mouse turns it; the
//! walk pushed as a stick pushes it) toward whatever the script points it at, and the
//! game moves it as it moves anyone: footfalls, the head's bob, the furniture in the
//! way. So a clip shows play rather than a camera on rails.
//!
//! The eyes move as a hand on a mouse aims: a quick sweep most of the way, easing in
//! and out (minimum jerk), taking longer the farther it goes, but only as the
//! logarithm of the distance (Fitts); falling a little short or now and then past it,
//! the pitch trailing the yaw so the path bows. Then a beat to see where it landed,
//! and a smaller sweep to put it right. Small slips are followed smoothly, as the eye
//! follows something moving, and a hand at rest is still.

use crate::game::Game;
use glam::{Vec2, Vec3};
use rand::{Rng, SeedableRng, rngs::StdRng};
use std::f32::consts::{PI, TAU};

const DEG: f32 = PI / 180.0;
/// Errors past this are swept to; those under it, followed.
const SWEEP_FROM: f32 = 3.0 * DEG;
/// A hand at rest: errors under this are left alone.
const REST: f32 = 0.3 * DEG;
/// How quickly small errors are followed (seconds).
const FOLLOW: f32 = 0.17;
/// The eyes keep pace with a mark moving slower than this (radians a second; past
/// it, the mark jumped, as the way ahead does round a corner), at this much of its
/// speed. Something passed at arm's length on the walk swings by at 150 deg/s.
const KEEP_PACE: f32 = 240.0 * DEG;
const PACE_GAIN: f32 = 0.9;
/// A sweep takes `A + B log2(1 + distance / W)` seconds.
const FITTS: (f32, f32, f32) = (0.12, 0.11, 2.0 * DEG);
/// Looking round slowly, sweeps are this long (radians), the least to the most.
const SLOW_STEP: (f32, f32) = (12.0 * DEG, 30.0 * DEG);
/// On target, for `until aimed`.
const AIMED: f32 = 1.5 * DEG;
/// Looking where the walk goes: a little below the horizon.
const AHEAD_PITCH: f32 = -6.0 * DEG;
/// Near enough a goal to stop (m), and how far out the walk begins to slow for it.
const ARRIVE: f32 = 0.25;
const SLOWING: f32 = 1.0;
/// A walk asked to hurry hurries only while the goal is farther than this.
const HURRY_FROM: f32 = 3.0;
/// Within this of the next turn of the way, looking ahead keeps the heading it had
/// (the way to a point nearly underfoot swings about), and the walk goes on without
/// waiting to face it.
const KEEP_HEADING: f32 = 1.2;
/// A walk that comes no nearer for this long (s) is given up.
const STALLED: f32 = 1.5;

/// What the eyes are kept on.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Aim {
    /// Nothing: the view stays where it is.
    Off,
    /// Where the walk goes (and, once there, the way it came).
    Ahead,
    At(Vec3),
    /// A familiar, wherever it flies.
    Familiar(usize),
}

/// What a script waits for.
#[derive(Clone, Copy, Debug)]
pub enum Until {
    /// The end of the walk.
    There,
    /// The eyes on what they are kept on.
    Aimed,
    /// The goal within so many metres' walk.
    Near(f32),
}

/// What the hands do this frame: the view turned (as `Input::look`), the walk.
#[derive(Default)]
pub struct Steer {
    pub look: Vec2,
    pub forward: f32,
    pub strafe: f32,
    pub sprint: bool,
}

/// One movement of the hand: the view turned by `by` (yaw, pitch) over `secs`, the
/// pitch `lag` behind.
struct Sweep {
    by: Vec2,
    secs: f32,
    lag: f32,
    t: f32,
}

impl Sweep {
    /// How far it has turned `t` seconds in.
    fn at(&self, t: f32) -> Vec2 {
        let s = |u: f32| {
            let u = u.clamp(0.0, 1.0);
            u * u * u * (10.0 - 15.0 * u + 6.0 * u * u)
        };
        Vec2::new(self.by.x * s(t / self.secs), self.by.y * s((t - self.lag) / self.secs))
    }
}

pub struct Driver {
    rng: StdRng,
    aim: Aim,
    /// Looking round (`aim`'s `slow`).
    slow: bool,
    sweep: Option<Sweep>,
    /// Until another sweep may begin (s): the eye taking in where the last one landed.
    pause: f32,
    /// How far the eyes were off their mark last frame (radians), and where it was
    /// (yaw, pitch).
    off: f32,
    last_mark: Option<Vec2>,
    goal: Option<Vec3>,
    hurry: bool,
    /// The walk's goal reached (or none set).
    there: bool,
    /// Walking distance left to it, the least it has been, and how long since it
    /// last came down.
    left: f32,
    best: f32,
    stalled: f32,
    /// The yaw of the way the walk last went.
    heading: Option<f32>,
}

impl Default for Driver {
    fn default() -> Self {
        Self {
            rng: StdRng::seed_from_u64(1),
            aim: Aim::Off,
            slow: false,
            sweep: None,
            pause: 0.0,
            off: 0.0,
            last_mark: None,
            goal: None,
            hurry: false,
            there: true,
            left: 0.0,
            best: 0.0,
            stalled: 0.0,
            heading: None,
        }
    }
}

impl Driver {
    /// Keep the eyes on `aim` from now on, after a moment to react; `slow`ly, as
    /// someone looking round a room, a part of the way at a time.
    pub fn aim(&mut self, aim: Aim, slow: bool) {
        self.aim = aim;
        self.slow = slow;
        self.off = f32::INFINITY;
        self.last_mark = None;
        let react = self.rng.gen_range(0.12..0.22);
        self.pause = self.pause.max(react);
    }

    /// Walk to `to`, round whatever is in the way.
    pub fn go(&mut self, to: Vec3, hurry: bool) {
        self.goal = Some(to);
        self.hurry = hurry;
        self.there = false;
        (self.left, self.best, self.stalled) = (f32::INFINITY, f32::INFINITY, 0.0);
    }

    pub fn met(&self, until: Until) -> bool {
        match until {
            Until::There => self.there,
            Until::Aimed => self.sweep.is_none() && self.off < AIMED,
            Until::Near(m) => self.there || self.left <= m,
        }
    }

    /// The hands for this frame.
    pub fn steer(&mut self, game: &mut Game, dt: f32) -> Steer {
        let mut out = Steer::default();
        let (yaw, pitch) = (game.player.yaw, game.player.pitch);
        let flat = |v: Vec3| Vec3::new(v.x, 0.0, v.z);
        let yaw_of = |d: Vec3| libm::atan2f(d.x, -d.z);
        let wrap = |a: f32| libm::remainderf(a, TAU);

        if let Some(goal) = self.goal.filter(|_| !self.there) {
            let p = game.player.pos;
            let (way, left) = match game.route_to(goal) {
                Some(r) => (r.waypoint, r.dist),
                None => (goal, flat(goal - p).length()),
            };
            self.left = left;
            // A walk that has stopped getting anywhere (a heap in the way) is over.
            if left < self.best - 0.05 {
                (self.best, self.stalled) = (left, 0.0);
            } else {
                self.stalled += dt;
            }
            let dir = flat(way - p).normalize_or_zero();
            if left <= ARRIVE || dir == Vec3::ZERO || self.stalled > STALLED {
                self.there = true;
            } else {
                let mut push = 0.3 + 0.7 * ((left - ARRIVE) / SLOWING).clamp(0.0, 1.0);
                if flat(way - p).length() > KEEP_HEADING {
                    self.heading = Some(yaw_of(dir));
                    if self.aim == Aim::Ahead {
                        // Turned to the way first, then off along it.
                        let c = libm::cosf(wrap(yaw_of(dir) - yaw));
                        let u = ((c - 0.5) / 0.4).clamp(0.0, 1.0);
                        push *= u * u * (3.0 - 2.0 * u);
                    }
                }
                let ahead = Vec3::new(libm::sinf(yaw), 0.0, -libm::cosf(yaw));
                out.forward = dir.dot(ahead) * push;
                out.strafe = dir.dot(game.player.right()) * push;
                out.sprint = self.hurry && left > HURRY_FROM;
            }
        }

        let eye = game.player.eye();
        let toward = |p: Vec3| {
            let d = (p - eye).normalize_or_zero();
            (yaw_of(d), libm::asinf(d.y))
        };
        let mark = match self.aim {
            Aim::Off => None,
            Aim::At(p) => Some(toward(p)),
            Aim::Familiar(i) => game.familiar_at(i).map(toward),
            Aim::Ahead => self.heading.map(|y| (y, AHEAD_PITCH)),
        };
        let mark = mark.map(|(y, p)| Vec2::new(y, p.clamp(-1.45, 1.45)));
        let err = mark.map(|m| Vec2::new(wrap(m.x - yaw), m.y - pitch));
        self.off = err.map_or(0.0, |e| e.length());
        // How the mark moved since the last frame (as the walk brings it nearer, or it
        // flies), unless it jumped: the eyes, once on it, keep pace with that.
        let drift = match (mark, self.last_mark) {
            (Some(m), Some(l)) => Vec2::new(wrap(m.x - l.x), m.y - l.y),
            _ => Vec2::ZERO,
        };
        let drift = if drift.length() < KEEP_PACE * dt {
            drift
        } else {
            Vec2::ZERO
        };
        self.last_mark = mark;

        let mut turn = Vec2::ZERO;
        if let Some(s) = &mut self.sweep {
            let before = s.at(s.t);
            s.t += dt;
            turn = s.at(s.t) - before + drift * PACE_GAIN;
            if s.t >= s.secs + s.lag {
                self.sweep = None;
                self.pause = if self.slow {
                    self.rng.gen_range(0.12..0.4)
                } else {
                    self.rng.gen_range(0.08..0.16)
                };
            }
        } else if let Some(e) = err {
            self.pause = (self.pause - dt).max(0.0);
            let off = e.length();
            // Far off, the hand waits out the pause (still taking it in), then sweeps.
            if off > SWEEP_FROM && self.pause == 0.0 {
                // Long sweeps fall short more, and sometimes go past.
                let gain = if off > 10.0 * DEG {
                    self.rng.gen_range(0.86..1.03)
                } else {
                    self.rng.gen_range(0.93..1.02)
                };
                // Looking round, a part of the way at a time.
                let by = if self.slow && off > SLOW_STEP.1 {
                    e * self.rng.gen_range(SLOW_STEP.0..SLOW_STEP.1) / off
                } else {
                    e * gain
                };
                let (a, b, w) = FITTS;
                let secs = a + b * libm::log2f(1.0 + by.length() / w);
                let lag = secs * self.rng.gen_range(0.04..0.14);
                let s = Sweep { by, secs, lag, t: dt };
                turn = s.at(dt);
                self.sweep = Some(s);
            } else if off <= SWEEP_FROM {
                turn = drift * PACE_GAIN;
                if off > REST {
                    turn += e * (1.0 - REST / off) * (1.0 - libm::expf(-dt / FOLLOW));
                }
            }
        }
        // As the mouse gives it: the yaw added, the pitch taken away.
        out.look = Vec2::new(turn.x, -turn.y);
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio::Audio;
    use crate::game::Input;
    use crate::library::Layout;

    /// Run `frames` frames of the stand-in; the yaw and pitch after each.
    fn drive(g: &mut Game, d: &mut Driver, frames: usize) -> Vec<(f32, f32)> {
        let mut audio = Audio::silent();
        let dt = 1.0 / 60.0;
        (0..frames)
            .map(|_| {
                let s = d.steer(g, dt);
                let input = Input {
                    forward: s.forward,
                    strafe: s.strafe,
                    sprint: s.sprint,
                    look: s.look,
                    ..Default::default()
                };
                g.update(dt, &input, &mut audio, true);
                (g.player.yaw, g.player.pitch)
            })
            .collect()
    }

    /// A turn to something behind the shoulder: a sweep there in under a second and
    /// a half, smooth (no jolt between frames), not far past it, and on it after.
    #[test]
    fn the_eyes_sweep_and_settle() {
        let mut g = Game::new(Layout::Hall);
        let mut d = Driver::default();
        let eye = g.player.eye();
        let (y0, p0) = (g.player.yaw, g.player.pitch);
        let goal = (y0 + 100.0 * DEG, p0 - 10.0 * DEG);
        let dir = Vec3::new(
            libm::sinf(goal.0) * libm::cosf(goal.1),
            libm::sinf(goal.1),
            -libm::cosf(goal.0) * libm::cosf(goal.1),
        );
        d.aim(Aim::At(eye + dir * 8.0), false);
        let path = drive(&mut g, &mut d, 150);
        let speed: Vec<f32> = path.windows(2).map(|w| (w[1].0 - w[0].0) * 60.0).collect();
        let peak = speed.iter().fold(0.0f32, |m, &v| m.max(v.abs()));
        let jolt = speed.windows(2).fold(0.0f32, |m, w| m.max((w[1] - w[0]).abs()));
        let past = path.iter().fold(f32::MIN, |m, &(y, _)| m.max(y - goal.0));
        let near = path.iter().position(|&(y, _)| (y - goal.0).abs() < 2.0 * DEG).unwrap();
        assert!((100.0 * DEG..400.0 * DEG).contains(&peak), "peak {}", peak / DEG);
        assert!(jolt < 25.0 * DEG, "jolt {}", jolt / DEG);
        assert!(past < 6.0 * DEG, "past {}", past / DEG);
        assert!(near < 90, "near after {near} frames");
        let (y, p) = path[path.len() - 1];
        assert!((y - goal.0).abs() < AIMED && (p - goal.1).abs() < AIMED, "{y} {p}");
        assert!(d.met(Until::Aimed));
    }

    /// A walk down the Hall, round the cases in the way, gets there and stops.
    #[test]
    fn the_walk_gets_there_and_stops() {
        let mut g = Game::new(Layout::Hall);
        let mut d = Driver::default();
        let from = g.player.pos;
        let stand = g.standable(Vec3::new(-20.0, 0.0, 12.0));
        d.aim(Aim::Ahead, false);
        d.go(stand, true);
        drive(&mut g, &mut d, 60 * 20);
        assert!(d.met(Until::There), "{} m left", d.left);
        let at = g.player.pos;
        assert!(
            Vec2::new(at.x - stand.x, at.z - stand.z).length() < 0.5,
            "{at} vs {stand}"
        );
        assert!(at.distance(from) > 15.0);
    }
}
