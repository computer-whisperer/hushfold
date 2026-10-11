//! Things in the library that move: gears turning, pendulums swinging, lanterns
//! floating. Looks only: nothing moving is in anyone's way, and the world, books and
//! saves never see it.
//!
//! A mover is a pivot and a `Motion` about it, and its parts are laid down where they
//! are at rest (`Library::moving`), as fixed pieces are; it can ride another mover (a
//! moon on a planet's arm), and a light can ride it. A mover can wait for a part of
//! the library to be finished (`celebrate::Awaits`): hidden until then, or still, or
//! dark. Each frame the game poses every mover from the clock and what is finished
//! (`pose`) and draws the parts of those in view.

use super::Library;
use super::celebrate::{Awaits, Change, Finish};
use crate::gfx::{Inst, kind};
use glam::{Quat, Vec3};
use std::f64::consts::TAU;
use std::ops::Range;

/// How a mover moves about its pivot. Rates and periods are in seconds; turns are
/// right-handed about their axis, so a negative rate turns the other way.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Motion {
    /// Turning steadily about `axis`, `rate` radians a second: a gear, an orrery's arm,
    /// a clock's hand.
    Spin { axis: Vec3, rate: f32 },
    /// Swinging to and fro about `axis`, `amp` radians either way, once every `period`:
    /// a pendulum, a hanging lamp, a banner stirring.
    Swing { axis: Vec3, amp: f32, period: f32 },
    /// Floating: rising and falling `amp` metres once every `period`, rocking `rock`
    /// radians as it goes: a lantern in the air, a candle, a ship on no sea.
    Bob { amp: f32, period: f32, rock: f32 },
    /// Turning about `axis` a step at a time, `steps` to a turn and a turn every
    /// `period`, each step a quick swing into place: a second hand, an escapement.
    Tick { axis: Vec3, steps: u32, period: f32 },
    /// Sliding out along `travel` and back once every `period`, resting at each end: a
    /// library ladder along its rail.
    Slide { travel: Vec3, period: f32 },
    /// Not moving of itself: a piece that only waits (`celebrate`).
    Still,
}

impl Motion {
    /// Where it has moved its parts at time `t` (seconds into its cycle), about the
    /// pivot: a turn, then an offset.
    fn at(self, t: f64) -> (Quat, Vec3) {
        let cycle = |period: f32| (t / period as f64).rem_euclid(1.0) as f32;
        let wave = |period: f32| libm::sinf(cycle(period) * std::f32::consts::TAU);
        match self {
            Motion::Spin { axis, rate } => {
                let a = (t * rate as f64).rem_euclid(TAU) as f32;
                (Quat::from_axis_angle(axis.normalize(), a), Vec3::ZERO)
            }
            Motion::Swing { axis, amp, period } => {
                (Quat::from_axis_angle(axis.normalize(), amp * wave(period)), Vec3::ZERO)
            }
            Motion::Bob { amp, period, rock } => {
                let c = cycle(period) * std::f32::consts::TAU;
                // Rocking a quarter-cycle behind the rise, about two axes, as a float does.
                let tilt = Quat::from_rotation_x(rock * libm::cosf(c))
                    * Quat::from_rotation_z(rock * 0.6 * libm::sinf(c * 0.5));
                (tilt, Vec3::Y * (amp * libm::sinf(c)))
            }
            Motion::Tick { axis, steps, period } => {
                let f = cycle(period) * steps as f32;
                let (whole, part) = (libm::floorf(f), f - libm::floorf(f));
                // Each step taken in the first tenth of its time, overshooting a little.
                let s = (part / 0.1).min(1.0);
                let ease = 1.0 - (1.0 - s) * (1.0 - s) + 0.08 * libm::sinf(s * std::f32::consts::PI);
                let a = (whole + ease) / steps as f32 * std::f32::consts::TAU;
                (Quat::from_axis_angle(axis.normalize(), a), Vec3::ZERO)
            }
            Motion::Slide { travel, period } => {
                // Out, rest, back, rest: a smoothstep each way.
                let c = cycle(period) * 4.0;
                let leg = |x: f32| {
                    let x = x.clamp(0.0, 1.0);
                    x * x * (3.0 - 2.0 * x)
                };
                let out = if c < 2.0 { leg(c) } else { 1.0 - leg(c - 2.0) };
                (Quat::IDENTITY, travel * out)
            }
            Motion::Still => (Quat::IDENTITY, Vec3::ZERO),
        }
    }
}

/// A moving thing: its pivot and motion, where it starts in its cycle (seconds), the
/// mover it rides (an earlier one), its parts in `Library::moving_insts`, how far
/// they reach from the pivot (for culling), and what it waits for, if anything.
#[derive(Clone, Debug)]
pub struct Mover {
    pub pivot: Vec3,
    pub motion: Motion,
    pub phase: f32,
    pub rides: Option<u32>,
    pub parts: Range<u32>,
    pub reach: f32,
    pub awaits: Option<Awaits>,
}

/// How long ago each part of the library was finished (seconds; `None`: not yet).
pub type Since<'a> = &'a dyn Fn(Finish) -> Option<f64>;

impl Mover {
    /// Its own pose at time `t`, not counting what it rides.
    fn own_pose(&self, t: f64, since: Since) -> Pose {
        let Some(a) = self.awaits else {
            return Pose::about(self.pivot, self.motion.at(t + self.phase as f64));
        };
        let s = since(a.finish);
        let (turn, offset) = if a.still {
            self.motion.at(a.motion_time(s))
        } else {
            self.motion.at(t + self.phase as f64)
        };
        let k = a.progress(s);
        let begun = s.is_some_and(|s| s >= a.delay as f64);
        // The change, about the pivot, after its own motion.
        let (change, scale, by, glow) = match a.change {
            Change::Arrives { .. } if !begun => (Quat::IDENTITY, 0.0, Vec3::ZERO, 1.0),
            Change::Arrives { from, grow } => (Quat::IDENTITY, grow + (1.0 - grow) * k, from * (1.0 - k), 1.0),
            Change::Shifts { by, turn, scale } => (Quat::IDENTITY.slerp(turn, k), 1.0 + (scale - 1.0) * k, by * k, 1.0),
            Change::Kindles { dim } => (Quat::IDENTITY, 1.0, Vec3::ZERO, dim + (1.0 - dim) * k),
            Change::Stays => (Quat::IDENTITY, 1.0, Vec3::ZERO, 1.0),
        };
        let whole = change * turn;
        Pose {
            turn: whole,
            scale,
            shift: self.pivot + by + change * (offset * scale) - whole * (self.pivot * scale),
            glow,
        }
    }
}

/// A mover's pose at some time: a point at rest at `p` is then at
/// `turn * (scale * p) + shift`, and what glows of it glows `glow` times as bright.
/// At `scale` 0 it is not there.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pose {
    pub turn: Quat,
    pub shift: Vec3,
    pub scale: f32,
    pub glow: f32,
}

impl Pose {
    /// Turned and moved by `(turn, offset)` about `pivot`.
    pub fn about(pivot: Vec3, (turn, offset): (Quat, Vec3)) -> Pose {
        Pose {
            turn,
            shift: pivot - turn * pivot + offset,
            scale: 1.0,
            glow: 1.0,
        }
    }

    pub fn point(&self, p: Vec3) -> Vec3 {
        self.turn * (p * self.scale) + self.shift
    }

    /// Whether what it poses is there to be drawn (or lit).
    pub fn shown(&self) -> bool {
        self.scale > 1e-4
    }

    /// A part laid down at rest as `i`, posed.
    pub fn inst(&self, i: &Inst) -> Inst {
        let (rest, pos) = (Vec3::from(i.pos), self.point(Vec3::from(i.pos)));
        let mut color = i.color;
        if kind::glows(i.kind) {
            color[3] *= self.glow;
        }
        Inst {
            pos: pos.into(),
            rot: (self.turn * Quat::from_array(i.rot)).normalize().into(),
            scale: (Vec3::from(i.scale) * self.scale).into(),
            color,
            moved: (Vec3::from(i.moved) + pos - rest).into(),
            ..*i
        }
    }

    /// This pose after `first` (`first` moves the point, then this moves it on).
    fn after(&self, first: &Pose) -> Pose {
        Pose {
            turn: self.turn * first.turn,
            shift: self.turn * (first.shift * self.scale) + self.shift,
            scale: self.scale * first.scale,
            glow: self.glow * first.glow,
        }
    }
}

/// Mover `m`'s pose at time `t`, with those it rides (one mover's, without posing the
/// rest).
pub fn pose_of(movers: &[Mover], m: u32, t: f64, since: Since) -> Pose {
    let mv = &movers[m as usize];
    let own = mv.own_pose(t, since);
    match mv.rides {
        Some(r) => pose_of(movers, r, t, since).after(&own),
        None => own,
    }
}

/// Every mover's pose at time `t` (seconds), into `out`.
pub fn pose(movers: &[Mover], t: f64, since: Since, out: &mut Vec<Pose>) {
    out.clear();
    for m in movers {
        let own = m.own_pose(t, since);
        let posed = match m.rides {
            Some(r) => out[r as usize].after(&own),
            None => own,
        };
        out.push(posed);
    }
}

impl Library {
    /// A mover about `pivot`, riding `rides` if given; lay its parts down with
    /// `moving` and it reaches as far as they do.
    pub fn mover(&mut self, pivot: Vec3, motion: Motion, phase: f32, rides: Option<u32>) -> u32 {
        if let Some(r) = rides {
            assert!((r as usize) < self.movers.len(), "a mover rides one made before it");
        }
        let at = self.moving_insts.len() as u32;
        self.movers.push(Mover {
            pivot,
            motion,
            phase,
            rides,
            parts: at..at,
            reach: 0.0,
            awaits: None,
        });
        self.movers.len() as u32 - 1
    }

    /// A part of mover `m` (the last one made), laid down where it is at rest.
    pub fn moving(&mut self, m: u32, i: Inst) {
        let mover = &mut self.movers[m as usize];
        assert_eq!(
            mover.parts.end as usize,
            self.moving_insts.len(),
            "a mover's parts are laid down together"
        );
        let half = Vec3::from(i.scale).length() * 0.5;
        mover.reach = mover.reach.max(Vec3::from(i.pos).distance(mover.pivot) + half);
        mover.parts.end += 1;
        self.moving_insts.push(i);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mover(motion: Motion, rides: Option<u32>) -> Mover {
        Mover {
            pivot: Vec3::new(1.0, 2.0, 3.0),
            motion,
            phase: 0.0,
            rides,
            parts: 0..0,
            reach: 1.0,
            awaits: None,
        }
    }

    /// At time 0 everything is where it was laid down, but what floats or slides
    /// (whose rest is the middle of its swing, or its start).
    #[test]
    fn rest_at_time_zero() {
        let y = Vec3::Y;
        for motion in [
            Motion::Spin { axis: y, rate: 1.3 },
            Motion::Swing {
                axis: Vec3::X,
                amp: 0.3,
                period: 2.0,
            },
            Motion::Tick {
                axis: y,
                steps: 60,
                period: 60.0,
            },
            Motion::Slide {
                travel: Vec3::X,
                period: 8.0,
            },
        ] {
            let mut out = Vec::new();
            pose(&[mover(motion, None)], 0.0, &|_| None, &mut out);
            let p = out[0].point(Vec3::new(3.0, 1.0, 0.0));
            assert!(p.abs_diff_eq(Vec3::new(3.0, 1.0, 0.0), 1e-4), "{motion:?}: {p}");
        }
    }

    /// A spin keeps its pivot still and turns once a period, smoothly even after a
    /// long time; a rider is carried round with what it rides.
    #[test]
    fn spins_and_rides() {
        let rate = std::f32::consts::TAU / 10.0;
        let spin = Motion::Spin { axis: Vec3::Y, rate };
        let mut riders = vec![mover(spin, None), mover(spin, Some(0))];
        riders[1].pivot = Vec3::new(3.0, 2.0, 3.0);
        let mut out = Vec::new();
        pose(&riders, 2.5, &|_| None, &mut out);
        // A quarter turn: the rider's pivot has gone round the first's.
        assert!(out[0].point(riders[0].pivot).abs_diff_eq(riders[0].pivot, 1e-5));
        let moved = out[1].point(riders[1].pivot);
        assert!(moved.abs_diff_eq(Vec3::new(1.0, 2.0, 1.0), 1e-4), "{moved}");
        // Ten hours in, a frame's turn is still a frame's turn.
        let t = 36000.0;
        let (a, b) = (Motion::at(spin, t).0, Motion::at(spin, t + 1.0 / 60.0).0);
        let step = a.angle_between(b);
        assert!((step - rate / 60.0).abs() < 1e-4, "{step}");
    }

    /// A mover waiting for a room: hidden, then coming in to rest; there, then moved,
    /// turned and shrunk, and staying so; dark, then lit; still, then turning. A rider
    /// shares what it rides changes.
    #[test]
    fn a_waiting_mover_changes_once_finished() {
        let room = Finish::Room(3);
        let pose_at = |a: Awaits, motion: Motion, since: Option<f64>| {
            let mut m = mover(motion, None);
            m.awaits = Some(a);
            let mut out = Vec::new();
            pose(&[m], 7.0, &move |f| (f == room).then_some(since).flatten(), &mut out);
            out[0]
        };
        let p = Vec3::new(2.0, 2.0, 3.0);
        let arrives = Awaits::new(
            room,
            Change::Arrives {
                from: Vec3::Y * 2.0,
                grow: 0.5,
            },
        )
        .after(1.0);
        for since in [None, Some(0.5)] {
            assert!(!pose_at(arrives, Motion::Still, since).shown(), "{since:?}");
        }
        let half = pose_at(arrives, Motion::Still, Some(2.25));
        assert!(half.shown() && half.scale > 0.5 && half.scale < 1.0);
        assert!(half.point(p).y > p.y);
        let done = pose_at(arrives, Motion::Still, Some(10.0));
        assert!(done.point(p).abs_diff_eq(p, 1e-5), "{}", done.point(p));
        let gone = Change::Shifts {
            by: Vec3::X,
            turn: Quat::from_rotation_y(1.0),
            scale: 0.0,
        };
        assert!(
            pose_at(Awaits::new(room, gone), Motion::Still, None)
                .point(p)
                .abs_diff_eq(p, 1e-5)
        );
        assert!(!pose_at(Awaits::new(room, gone), Motion::Still, Some(3.0)).shown());
        let kindles = Awaits::new(room, Change::Kindles { dim: 0.2 });
        assert!((pose_at(kindles, Motion::Still, None).glow - 0.2).abs() < 1e-6);
        assert!((pose_at(kindles, Motion::Still, Some(3.0)).glow - 1.0).abs() < 1e-6);
        let spin = Motion::Spin {
            axis: Vec3::Y,
            rate: 1.0,
        };
        let still = kindles.still();
        assert!(pose_at(still, spin, None).point(p).abs_diff_eq(p, 1e-5));
        assert!(pose_at(still, spin, Some(0.2)).point(p).distance(p) < 0.05);
        assert!(pose_at(still, spin, Some(4.0)).point(p).distance(p) > 0.5);
        // A rider on a mover half grown is half grown about that mover's pivot.
        let mut movers = vec![mover(Motion::Still, None), mover(Motion::Still, Some(0))];
        movers[0].awaits = Some(Awaits::new(
            room,
            Change::Arrives {
                from: Vec3::ZERO,
                grow: 0.0,
            },
        ));
        let mut out = Vec::new();
        pose(&movers, 0.0, &|_| Some(1.25), &mut out);
        let pivot = movers[0].pivot;
        assert!((out[1].scale - 0.5).abs() < 1e-5);
        assert!(out[1].point(p).abs_diff_eq(pivot + (p - pivot) * 0.5, 1e-5));
    }

    /// A posed part is drawn where it has moved to, and remembers where it was laid
    /// down (its patterns are anchored there): flicker on a swinging thing otherwise.
    #[test]
    fn a_posed_part_remembers_its_rest() {
        let swing = Motion::Swing {
            axis: Vec3::X,
            amp: 0.3,
            period: 2.0,
        };
        let movers = vec![mover(swing, None), mover(swing, Some(0))];
        let part = Inst::new(Vec3::new(1.0, 0.5, 3.0), Quat::IDENTITY, Vec3::ONE, 0, [1.0; 4], 0);
        let mut out = Vec::new();
        for t in [0.3, 0.7, 1.9] {
            pose(&movers, t, &|_| None, &mut out);
            let posed = out[1].inst(&part);
            let rest = Vec3::from(posed.pos) - Vec3::from(posed.moved);
            assert!(
                Vec3::from(posed.pos).distance(Vec3::from(part.pos)) > 0.1,
                "t {t}: didn't move"
            );
            assert!(rest.abs_diff_eq(Vec3::from(part.pos), 1e-5), "t {t}: {rest}");
        }
    }
}
