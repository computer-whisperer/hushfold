//! Things in the library that move: gears turning, pendulums swinging, lanterns
//! floating. Looks only: nothing moving is in anyone's way, and the world, books and
//! saves never see it.
//!
//! A mover is a pivot and a `Motion` about it, and its parts are laid down where they
//! are at rest (`Library::moving`), as fixed pieces are; it can ride another mover (a
//! moon on a planet's arm), and a light can ride it. Each frame the game poses every
//! mover from the clock (`pose`) and draws the parts of those in view.

use super::Library;
use crate::gfx::Inst;
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
        }
    }
}

/// A moving thing: its pivot and motion, where it starts in its cycle (seconds), the
/// mover it rides (an earlier one), its parts in `Library::moving_insts`, and how far
/// they reach from the pivot (for culling).
#[derive(Clone, Debug)]
pub struct Mover {
    pub pivot: Vec3,
    pub motion: Motion,
    pub phase: f32,
    pub rides: Option<u32>,
    pub parts: Range<u32>,
    pub reach: f32,
}

/// A mover's pose at some time: a point at rest at `p` is then at `turn * p + shift`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pose {
    pub turn: Quat,
    pub shift: Vec3,
}

impl Pose {
    pub fn point(&self, p: Vec3) -> Vec3 {
        self.turn * p + self.shift
    }

    /// A part laid down at rest as `i`, posed.
    pub fn inst(&self, i: &Inst) -> Inst {
        let (rest, pos) = (Vec3::from(i.pos), self.point(Vec3::from(i.pos)));
        Inst {
            pos: pos.into(),
            rot: (self.turn * Quat::from_array(i.rot)).normalize().into(),
            moved: (Vec3::from(i.moved) + pos - rest).into(),
            ..*i
        }
    }

    /// This pose after `first` (`first` moves the point, then this moves it on).
    fn after(&self, first: &Pose) -> Pose {
        Pose {
            turn: self.turn * first.turn,
            shift: self.turn * first.shift + self.shift,
        }
    }
}

/// Mover `m`'s pose at time `t`, with those it rides (one mover's, without posing the
/// rest).
pub fn pose_of(movers: &[Mover], m: u32, t: f64) -> Pose {
    let mv = &movers[m as usize];
    let (turn, offset) = mv.motion.at(t + mv.phase as f64);
    let own = Pose {
        turn,
        shift: mv.pivot - turn * mv.pivot + offset,
    };
    match mv.rides {
        Some(r) => pose_of(movers, r, t).after(&own),
        None => own,
    }
}

/// Every mover's pose at time `t` (seconds), into `out`.
pub fn pose(movers: &[Mover], t: f64, out: &mut Vec<Pose>) {
    out.clear();
    for m in movers {
        let (turn, offset) = m.motion.at(t + m.phase as f64);
        let own = Pose {
            turn,
            shift: m.pivot - turn * m.pivot + offset,
        };
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
            pose(&[mover(motion, None)], 0.0, &mut out);
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
        pose(&riders, 2.5, &mut out);
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
            pose(&movers, t, &mut out);
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
