//! What the library is to everything that moves through it or looks across it: its
//! levels and floors, flights of stairs, solids, and gates. Bodies (the player, and
//! the routes the nav grid plans for them), books and sight all ask here, so they
//! agree on what is in the way.

use glam::{Vec2, Vec3};
use std::ops::BitOr;

/// The highest rise a body steps onto; anything higher is a wall (or the floor above).
pub const STEP_UP: f32 = 0.45;
/// A body's height and radius.
pub const BODY_H: f32 = 1.8;
pub const BODY_R: f32 = 0.3;
/// Thickness of a floor slab (its top is the level's floor).
pub const SLAB: f32 = 0.35;
/// How far a stair's tread overhangs the riser below it.
pub const NOSING: f32 = 0.03;

/// A storey: its floor elevation, how high its space reaches where no floor above
/// caps it (unless a raised roof says otherwise), and the rectangle everything on
/// it lies within (its walls, to bodies and books).
pub struct Level {
    pub y: f32,
    pub roof: f32,
    pub min: Vec2,
    pub max: Vec2,
}

/// A rectangle of floor on a level (its top at the level's elevation). Wherever a
/// level has no slab is a hole: books fall through it and bodies can't stand.
pub struct Slab {
    pub level: usize,
    pub min: Vec2,
    pub max: Vec2,
}

/// A flight of `steps` stairs, solid stone from its level's floor up to its treads,
/// rising from `y0` at its low end to `y1` along `axis` (a unit axis direction)
/// across its rectangle. Bodies walk it as a smooth slope; books rest on its treads.
#[derive(Clone, Copy)]
pub struct Stairs {
    pub level: usize,
    pub min: Vec2,
    pub max: Vec2,
    pub axis: Vec2,
    pub y0: f32,
    pub y1: f32,
    pub steps: usize,
}

impl Stairs {
    pub fn contains(&self, p: Vec2) -> bool {
        p.cmpge(self.min).all() && p.cmple(self.max).all()
    }

    /// Its length along `axis`, and its width across.
    pub fn len(&self) -> f32 {
        (self.max - self.min).dot(self.axis.abs())
    }

    pub fn width(&self) -> f32 {
        (self.max - self.min).dot(Vec2::new(self.axis.y, self.axis.x).abs())
    }

    /// Length of a step's stretch of the run, and its rise.
    pub fn run(&self) -> f32 {
        self.len() / self.steps as f32
    }

    pub fn rise(&self) -> f32 {
        (self.y1 - self.y0) / self.steps as f32
    }

    /// How far along the flight `p` is (not clamped).
    fn along(&self, p: Vec2) -> f32 {
        (p - (self.min + self.max) * 0.5).dot(self.axis) + self.len() * 0.5
    }

    /// Height of the slope above `p` (clamped to the ends).
    pub fn height(&self, p: Vec2) -> f32 {
        self.y0 + (self.y1 - self.y0) * (self.along(p) / self.len()).clamp(0.0, 1.0)
    }

    /// Height of the top of step `k` (from 0, at the low end).
    pub fn step_top(&self, k: usize) -> f32 {
        self.y0 + (k + 1) as f32 * self.rise()
    }

    /// The rectangle of step `k`'s stretch of the run, reaching `back` beyond its
    /// riser (over the step below).
    pub fn step_rect(&self, k: usize, back: f32) -> (Vec2, Vec2) {
        let start = (self.min + self.max) * 0.5 - self.axis * self.len() * 0.5;
        let across = Vec2::new(self.axis.y, self.axis.x).abs() * (self.max - self.min) * 0.5;
        let a = start + self.axis * (k as f32 * self.run() - back) - across;
        let b = start + self.axis * ((k + 1) as f32 * self.run()) + across;
        (a.min(b), a.max(b))
    }

    /// Height of the tread above `p`: the step whose stretch (with its nosing) it is
    /// over.
    pub fn tread(&self, p: Vec2) -> f32 {
        let k = ((self.along(p) + NOSING) / self.run()).floor();
        self.step_top(k.clamp(0.0, (self.steps - 1) as f32) as usize)
    }
}

/// What a solid is in the way of: any of bodies, books and sight.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Stops(u8);

impl Stops {
    pub const BODIES: Stops = Stops(1);
    pub const BOOKS: Stops = Stops(2);
    pub const SIGHT: Stops = Stops(4);
    /// Railings and chairs: in the way of bodies and books, but seen past.
    pub const RAIL: Stops = Stops(1 | 2);
    pub const ALL: Stops = Stops(1 | 2 | 4);

    /// True if it stops every kind of thing in `what`.
    pub fn has(self, what: Stops) -> bool {
        self.0 & what.0 == what.0
    }
}

impl BitOr for Stops {
    type Output = Stops;
    fn bitor(self, o: Stops) -> Stops {
        Stops(self.0 | o.0)
    }
}

/// A box in the way of what `stops` says (furniture, a wall, a railing's guard). One
/// that stops bodies or books stands on a floor; it may rise through the floor above.
#[derive(Clone, Copy)]
pub struct Solid {
    pub min: Vec3,
    pub max: Vec3,
    pub stops: Stops,
}

/// A veil across the way into a department: a thin solid, in the way of everything,
/// until the departments it counts are shelved to `need` of their books.
pub struct Gate {
    pub level: usize,
    pub min: Vec3,
    pub max: Vec3,
    /// The department it opens.
    pub opens: usize,
    pub counts: Vec<usize>,
    pub need: f32,
}

#[derive(Default)]
pub struct World {
    pub levels: Vec<Level>,
    pub slabs: Vec<Slab>,
    stairs: Vec<Stairs>,
    solids: Vec<Solid>,
    /// Where a level's space reaches higher than its roof: (level, min, max, height).
    pub raised_roofs: Vec<(usize, Vec2, Vec2, f32)>,
    gates: Vec<Gate>,
    /// Which gates are open (all closed to begin with).
    open: Vec<bool>,
}

/// True if a box from `mn` to `mx` is in the way of a body standing at `feet`: it
/// overlaps the body above what can be stepped over.
pub fn in_body(mn: Vec3, mx: Vec3, feet: f32) -> bool {
    mx.y > feet + STEP_UP && mn.y < feet + BODY_H
}

/// Where a ray from `o` along `d` enters the box, if it does (0 if it starts inside).
pub fn ray_box(o: Vec3, d: Vec3, mn: Vec3, mx: Vec3) -> Option<f32> {
    let inv = d.recip();
    let t1 = (mn - o) * inv;
    let t2 = (mx - o) * inv;
    let tmin = t1.min(t2).max_element();
    let tmax = t1.max(t2).min_element();
    (tmax >= tmin.max(0.0)).then_some(tmin.max(0.0))
}

impl World {
    /// The highest level whose floor is at or below `y`.
    pub fn level_below(&self, y: f32) -> usize {
        (0..self.levels.len())
            .rev()
            .find(|&l| self.levels[l].y <= y + 1e-3)
            .unwrap_or(0)
    }

    /// A solid. One that stops books must stand on a floor: books know a level as a
    /// heightfield under its roof, so nothing can stop them from above.
    pub fn add_solid(&mut self, min: Vec3, max: Vec3, stops: Stops) {
        let floor = self.levels[self.level_below(min.y)].y;
        assert!(
            !stops.has(Stops::BOOKS) || (min.y - floor).abs() < 1e-3,
            "a solid that stops books off the floor, at {min}"
        );
        self.solids.push(Solid { min, max, stops });
    }

    /// A flight of stairs. Its stone stands on its level's floor; it rises from `y0`.
    pub fn add_stairs(&mut self, s: Stairs) {
        let unit_axis = (s.axis.abs() - Vec2::X).length() < 1e-6 || (s.axis.abs() - Vec2::Y).length() < 1e-6;
        assert!(unit_axis && s.steps > 0, "stairs along {} in {} steps", s.axis, s.steps);
        assert!(
            s.y0 >= self.levels[s.level].y - 1e-3,
            "stairs starting below their floor"
        );
        self.stairs.push(s);
    }

    pub fn stairs(&self) -> &[Stairs] {
        &self.stairs
    }

    pub fn solids(&self) -> &[Solid] {
        &self.solids
    }

    /// The solids in level `l`'s space (above its floor, below the next), each as it
    /// stands there: one rising through the floor from the level below stands on it.
    pub fn solids_on(&self, l: usize) -> impl Iterator<Item = Solid> + '_ {
        let floor = self.levels[l].y;
        let next = self.levels.get(l + 1).map_or(f32::MAX, |n| n.y);
        (self.solids.iter())
            .filter(move |s| s.min.y < next - 1e-3 && s.max.y > floor + 1e-3)
            .map(move |s| Solid {
                min: Vec3::new(s.min.x, s.min.y.max(floor), s.min.z),
                ..*s
            })
    }

    pub fn add_gate(&mut self, gate: Gate) {
        self.gates.push(gate);
        self.open.push(false);
    }

    pub fn gates(&self) -> &[Gate] {
        &self.gates
    }

    /// Which gates are open, by index.
    pub fn gates_open(&self) -> &[bool] {
        &self.open
    }

    /// Open gate `g` here. `Loose` and `Nav` keep their own account of the gates;
    /// `Game::open_gate` tells all three.
    pub fn open_gate(&mut self, g: usize) {
        self.open[g] = true;
    }

    pub fn closed_gates(&self) -> impl Iterator<Item = (usize, &Gate)> {
        (self.gates.iter().enumerate()).filter(|&(g, _)| !self.open[g])
    }

    /// Heights of every floor and stair surface over `p`, on `level` or (None) any.
    pub fn surfaces(&self, p: Vec2, level: Option<usize>) -> impl Iterator<Item = f32> + '_ {
        let on = move |l: usize| level.is_none_or(|want| want == l);
        let slabs = (self.slabs.iter())
            .filter(move |s| on(s.level) && p.cmpge(s.min).all() && p.cmple(s.max).all())
            .map(|s| self.levels[s.level].y);
        let stairs = (self.stairs.iter())
            .filter(move |f| on(f.level) && f.contains(p))
            .map(move |f| f.height(p));
        slabs.chain(stairs)
    }

    /// The highest floor or stair surface under `p` that is no higher than `below`.
    pub fn ground(&self, p: Vec2, below: f32) -> Option<f32> {
        self.surfaces(p, None).filter(|&y| y <= below).max_by(f32::total_cmp)
    }

    /// The level whose floor (or stair) `p` stands on or above.
    pub fn level_at(&self, p: Vec3) -> usize {
        // A flight belongs to the level it rises from.
        self.ground(Vec2::new(p.x, p.z), p.y + 0.5)
            .map_or(0, |g| self.level_below(g))
    }

    /// True if something at `p` rises in the way of a body standing at `feet`: the
    /// edge of a floor higher than a step but below the head, or a flight of stairs
    /// whose stone (from its own floor up to its treads) fills the body's height above
    /// a step. A flight on the floor above is overhead, not in the way.
    pub fn rises_at(&self, p: Vec2, feet: f32) -> bool {
        let slab = (self.slabs.iter())
            .filter(|s| p.cmpge(s.min).all() && p.cmple(s.max).all())
            .any(|s| (feet + STEP_UP..feet + BODY_H).contains(&self.levels[s.level].y));
        let stairs = (self.stairs.iter())
            .any(|f| f.contains(p) && f.height(p) > feet + STEP_UP && self.levels[f.level].y < feet + BODY_H);
        slab || stairs
    }

    /// Boxes in the way of a body standing at `feet` (see `in_body`): solids that stop
    /// bodies, and closed gates (with the gate's index).
    pub fn in_way(&self, feet: f32) -> impl Iterator<Item = (Vec3, Vec3, Option<usize>)> + '_ {
        let solids = (self.solids.iter())
            .filter(move |s| s.stops.has(Stops::BODIES) && in_body(s.min, s.max, feet))
            .map(|s| (s.min, s.max, None));
        let gates = (self.closed_gates())
            .filter(move |(_, g)| in_body(g.min, g.max, feet))
            .map(|(i, g)| (g.min, g.max, Some(i)));
        solids.chain(gates)
    }

    /// How far a ray from `o` along unit `d` goes (up to `max_t`) before something
    /// that stops `what` (sight, say; see `Stops::has`) is in its way: solids that do,
    /// floors, stair stone and closed gates.
    pub fn ray(&self, o: Vec3, d: Vec3, max_t: f32, what: Stops) -> f32 {
        let solids = (self.solids.iter())
            .filter(|s| s.stops.has(what))
            .map(|s| (s.min, s.max));
        let gates = self.closed_gates().map(|(_, g)| (g.min, g.max));
        let floors = self.slabs.iter().map(|s| {
            let y = self.levels[s.level].y;
            (Vec3::new(s.min.x, y - SLAB, s.min.y), Vec3::new(s.max.x, y, s.max.y))
        });
        let boxes = (solids.chain(gates).chain(floors))
            .filter_map(|(mn, mx)| ray_box(o, d, mn, mx))
            .fold(max_t, f32::min);
        let stairs = (self.stairs.iter())
            .filter_map(|f| {
                let floor = self.levels[f.level].y;
                let bounds = (Vec3::new(f.min.x, floor, f.min.y), Vec3::new(f.max.x, f.y1, f.max.y));
                ray_box(o, d, bounds.0, bounds.1)?;
                (0..f.steps)
                    .filter_map(|k| {
                        let (mn, mx) = f.step_rect(k, 0.0);
                        let top = f.step_top(k);
                        ray_box(o, d, Vec3::new(mn.x, floor, mn.y), Vec3::new(mx.x, top, mx.y))
                    })
                    .min_by(f32::total_cmp)
            })
            .fold(max_t, f32::min);
        boxes.min(stairs)
    }

    /// True if nothing that stops sight lies between `a` and `b`.
    pub fn line_of_sight(&self, a: Vec3, b: Vec3) -> bool {
        let d = b - a;
        let len = d.length();
        self.ray(a, d / len.max(1e-5), f32::MAX, Stops::SIGHT) >= len - 0.15
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::library::{Layout, Library};

    /// A solid rising through the floor above stands on that floor too; one that
    /// stops at it does not.
    #[test]
    fn solids_rising_through_a_floor_stand_on_it() {
        let mut w = World::default();
        for y in [0.0, 6.0] {
            let (min, max) = (Vec2::splat(-5.0), Vec2::splat(5.0));
            w.levels.push(Level {
                y,
                roof: y + 5.6,
                min,
                max,
            });
        }
        w.add_solid(Vec3::new(0.0, 0.0, 0.0), Vec3::new(1.0, 7.0, 1.0), Stops::RAIL);
        w.add_solid(Vec3::new(2.0, 0.0, 0.0), Vec3::new(3.0, 6.0, 1.0), Stops::ALL);
        let up: Vec<Solid> = w.solids_on(1).collect();
        assert_eq!(up.len(), 1);
        assert_eq!((up[0].min.y, up[0].max.y), (6.0, 7.0));
        assert_eq!(w.solids_on(0).count(), 2);
    }

    /// A sight line through the stone of a flight of stairs is blocked; one along
    /// the open floor beside it is not.
    #[test]
    fn stairs_block_sight() {
        let lib = Library::generate(Layout::Great, crate::game::SEED);
        let w = &lib.world;
        // Across the west flight at 1.5 m, where its treads are ~4 m up.
        assert!(!w.line_of_sight(Vec3::new(-7.0, 1.5, -2.0), Vec3::new(-13.0, 1.5, -2.0)));
        // Past its low end, where nothing stands.
        assert!(w.line_of_sight(Vec3::new(-7.0, 1.5, 7.5), Vec3::new(-13.0, 1.5, 7.5)));
    }
}
