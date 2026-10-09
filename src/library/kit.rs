//! Pieces a layout places: furniture, lights, railings, panelling, door frames and
//! piers. Each draws itself and says what of it is in the way, so a layout only
//! decides where things go.

use super::*;

/// A railing's height, and the thickness of the guard standing for it.
const RAIL_H: f32 = 1.05;
pub(super) const RAIL_T: f32 = 0.12;
const BALUSTER: [f32; 3] = [0.2, 0.1, 0.045];
const PALE_STONE: [f32; 3] = [0.48, 0.45, 0.4];
const PIER_STONE: [f32; 3] = [0.42, 0.39, 0.35];
const STEP_STONE: [f32; 3] = [0.4, 0.37, 0.33];
/// Thickness of a stair's wooden tread.
const TREAD_T: f32 = 0.05;

impl Library {
    /// A brass wall sconce with a candle flame, lighting the room in front of it.
    pub(super) fn sconce(&mut self, p: Vec3, n: Vec3) {
        self.decor(Inst::aabox(p - n * 0.08, Vec3::new(0.12, 0.3, 0.12), kind::WOOD, BRASS));
        self.decor(Inst::aabox(
            p + Vec3::Y * 0.12,
            Vec3::new(0.06, 0.12, 0.06),
            kind::EMISSIVE,
            [0.9, 0.85, 0.7],
        ));
        let mut flame = Inst::aabox(
            p + Vec3::Y * 0.24,
            Vec3::new(0.05, 0.1, 0.05),
            kind::EMISSIVE,
            [1.0, 0.55, 0.18],
        );
        flame.color[3] = 12.0;
        flame.param = 1;
        self.decor(flame);
        self.light(
            p + n * 0.4 + Vec3::Y * 0.3,
            8.0,
            [1.0, 0.62, 0.3],
            1.3,
            LightKind::Flicker,
        );
    }

    /// A reading table centred at `c` (on the floor), with four chairs and a candle.
    pub(super) fn reading_table(&mut self, c: Vec3) {
        self.decor(Inst::aabox(
            c + Vec3::Y * (TABLE_H - 0.03),
            Vec3::new(2.4, 0.06, 1.2),
            kind::WOOD,
            WALNUT,
        ));
        for (sx, sz) in [(-1.0, -1.0), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)] {
            self.decor(Inst::aabox(
                c + Vec3::new(sx * 1.08, (TABLE_H - 0.06) * 0.5, sz * 0.48),
                Vec3::new(0.09, TABLE_H - 0.06, 0.09),
                kind::WOOD,
                DARK_WOOD,
            ));
        }
        self.collide(
            c - Vec3::new(1.25, 0.0, 0.65),
            c + Vec3::new(1.25, TABLE_H, 0.65),
            Stops::ALL,
        );
        for sz in [-1.0f32, 1.0] {
            for sx in [-0.6f32, 0.6] {
                let ch = c + Vec3::new(sx, 0.0, sz * 1.0);
                self.decor(Inst::aabox(
                    ch + Vec3::Y * 0.45,
                    Vec3::new(0.48, 0.05, 0.46),
                    kind::WOOD,
                    [0.2, 0.09, 0.04],
                ));
                self.decor(Inst::aabox(
                    ch + Vec3::new(0.0, 0.75, sz * 0.2),
                    Vec3::new(0.44, 0.6, 0.04),
                    kind::WOOD,
                    [0.2, 0.09, 0.04],
                ));
                for (lx, lz) in [(-0.2, -0.2), (0.2, -0.2), (-0.2, 0.2), (0.2, 0.2)] {
                    self.decor(Inst::aabox(
                        ch + Vec3::new(lx, 0.22, lz),
                        Vec3::new(0.04, 0.44, 0.04),
                        kind::WOOD,
                        DARK_WOOD,
                    ));
                }
                self.collide(
                    ch - Vec3::new(0.26, 0.0, 0.26),
                    ch + Vec3::new(0.26, 1.05, 0.26),
                    Stops::RAIL,
                );
            }
        }
        let candle = c + Vec3::new(0.7, TABLE_H, 0.0);
        self.decor(Inst::aabox(
            candle + Vec3::Y * 0.02,
            Vec3::new(0.14, 0.04, 0.14),
            kind::WOOD,
            BRASS,
        ));
        self.decor(Inst::aabox(
            candle + Vec3::Y * 0.11,
            Vec3::new(0.05, 0.16, 0.05),
            kind::PLAIN,
            [0.8, 0.75, 0.6],
        ));
        let mut flame = Inst::aabox(
            candle + Vec3::Y * 0.22,
            Vec3::new(0.025, 0.05, 0.025),
            kind::EMISSIVE,
            [1.0, 0.55, 0.18],
        );
        flame.color[3] = 14.0;
        flame.param = 1;
        self.decor(flame);
        self.light(candle + Vec3::Y * 0.35, 4.5, [1.0, 0.6, 0.28], 0.7, LightKind::Flicker);
    }

    /// A ring chandelier at `ch` hung on chains from `top_y`.
    pub(super) fn chandelier(&mut self, ch: Vec3, top_y: f32) {
        for k in 0..12 {
            let a = k as f32 / 12.0 * TAU;
            let p = ch + Vec3::new(a.cos(), 0.0, a.sin()) * 1.3;
            self.decor(Inst::new(
                p,
                Quat::from_rotation_y(-a),
                Vec3::new(0.08, 0.06, 0.72),
                kind::WOOD,
                [BRASS[0], BRASS[1], BRASS[2], 1.0],
                0,
            ));
            if k % 2 == 0 {
                self.decor(Inst::aabox(
                    p + Vec3::Y * 0.1,
                    Vec3::new(0.05, 0.14, 0.05),
                    kind::PLAIN,
                    [0.8, 0.75, 0.6],
                ));
                let mut flame = Inst::aabox(
                    p + Vec3::Y * 0.21,
                    Vec3::new(0.03, 0.06, 0.03),
                    kind::EMISSIVE,
                    [1.0, 0.55, 0.18],
                );
                flame.color[3] = 14.0;
                flame.param = 1;
                self.decor(flame);
            }
        }
        for k in 0..3 {
            let a = k as f32 / 3.0 * TAU;
            let top = Vec3::new(ch.x, top_y, ch.z);
            let bottom = ch + Vec3::new(a.cos(), 0.0, a.sin()) * 1.3;
            let d = top - bottom;
            let rot = Quat::from_rotation_arc(Vec3::Y, d.normalize());
            self.decor(Inst::new(
                (top + bottom) * 0.5,
                rot,
                Vec3::new(0.025, d.length(), 0.025),
                kind::WOOD,
                [0.1, 0.08, 0.05, 1.0],
                0,
            ));
        }
        self.light(ch + Vec3::Y * 0.2, 20.0, [1.0, 0.68, 0.38], 4.0, LightKind::Flicker);
    }

    /// A double-sided bookcase standing at `c` (floor level), running along x with its
    /// faces toward ±z, or (`along_x` false) running along z, faces toward ±x.
    pub(super) fn bookcase(&mut self, c: Vec3, along_x: bool) {
        // Boxes are laid out for a case along x; `sw` turns offsets and sizes.
        let sw = |v: Vec3| if along_x { v } else { Vec3::new(v.z, v.y, v.x) };
        let part = |lib: &mut Self, off: Vec3, size: Vec3, rgb: [f32; 3]| {
            lib.decor(Inst::aabox(c + sw(off), sw(size), kind::WOOD, rgb));
        };
        let hw = UNIT_W * 0.5;
        let hd = UNIT_DEPTH * 0.5;
        for s in [-1.0, 1.0] {
            part(
                self,
                Vec3::new(s * (hw - SIDE_T * 0.5), UNIT_H * 0.5, 0.0),
                Vec3::new(SIDE_T, UNIT_H, UNIT_DEPTH),
                WALNUT,
            );
        }
        part(
            self,
            Vec3::Y * (UNIT_H + 0.04),
            Vec3::new(UNIT_W + 0.08, 0.08, UNIT_DEPTH + 0.08),
            WALNUT,
        );
        part(
            self,
            Vec3::Y * (UNIT_H + 0.105),
            // As wide as the pitch, so the caps of a row of cases meet without overlapping.
            Vec3::new(UNIT_PITCH, 0.05, UNIT_DEPTH + 0.16),
            [0.12, 0.06, 0.028],
        );
        part(
            self,
            Vec3::Y * (SHELF_BASE * 0.5),
            Vec3::new(UNIT_W - 0.02, SHELF_BASE, UNIT_DEPTH - 0.04),
            [0.1, 0.05, 0.022],
        );
        part(
            self,
            Vec3::Y * (UNIT_H * 0.5),
            Vec3::new(UNIT_W - 2.0 * SIDE_T, UNIT_H, 0.04),
            DARK_WOOD,
        );
        for nz in [-1.0f32, 1.0] {
            for i in 1..ROWS {
                let y = SHELF_BASE + i as f32 * SHELF_STEP;
                part(
                    self,
                    Vec3::new(0.0, y - BOARD_T * 0.5, nz * (0.02 + FACE_DEPTH * 0.5)),
                    Vec3::new(UNIT_W - 2.0 * SIDE_T, BOARD_T, FACE_DEPTH),
                    WALNUT,
                );
            }
        }
        // Solid to the cornice, and no aiming over its top at the far side's shelves.
        let half = sw(Vec3::new(hw + 0.04, 0.0, hd + 0.04));
        let top = c + half + Vec3::Y * (UNIT_H + 0.13);
        self.collide(c - half, top, Stops::ALL);
        self.collide(
            Vec3::new(c.x - half.x, top.y, c.z - half.z),
            top + Vec3::Y * 1.07,
            Stops::SIGHT,
        );
    }

    /// A flight of stairs, declared in the world and drawn from its profile: solid
    /// stone steps with wooden treads, a stringer and a balustrade on either side, and
    /// guards so nothing goes over the sides or into its high end.
    pub(super) fn stairs(&mut self, f: Stairs, newels: &mut Vec<Vec3>) {
        self.world.add_stairs(f);
        let c = (f.min + f.max) * 0.5;
        let n = Vec2::new(-f.axis.y, f.axis.x);
        let (r, w) = (f.len() * 0.5, f.width() * 0.5);
        // A point `a` along the flight from its middle, `u` across it, at height `y`.
        let at = |a: f32, u: f32, y: f32| {
            let p = c + f.axis * a + n * u;
            Vec3::new(p.x, y, p.y)
        };
        let slope = |a: f32| f.height(c + f.axis * a);
        let rect = |a0: f32, a1: f32, u0: f32, u1: f32| {
            let (p, q) = (at(a0, u0, 0.0), at(a1, u1, 0.0));
            (
                Vec2::new(p.x, p.z).min(Vec2::new(q.x, q.z)),
                Vec2::new(p.x, p.z).max(Vec2::new(q.x, q.z)),
            )
        };
        let floor = self.world.levels[f.level].y;
        for k in 0..f.steps {
            let top = f.step_top(k);
            let (mn, mx) = f.step_rect(k, 0.0);
            let stone = Inst::minmax(
                Vec3::new(mn.x, floor, mn.y),
                Vec3::new(mx.x, top - TREAD_T, mx.y),
                kind::STONE,
                STEP_STONE,
            );
            self.decor(stone);
            // The tread overhangs its riser a little (as books find it: `Stairs::tread`).
            let (mn, mx) = f.step_rect(k, NOSING);
            self.decor(Inst::minmax(
                Vec3::new(mn.x, top - TREAD_T, mn.y),
                Vec3::new(mx.x, top, mx.y),
                kind::WOOD,
                WALNUT,
            ));
        }
        for s in [-1.0f32, 1.0] {
            let (u_rail, u_string) = (s * (w - 0.07), s * (w + 0.035));
            // A stringer over the ends of the steps, stopping short of the floor above.
            let end = r - 0.46;
            self.beam(
                at(-r, u_string, slope(-r) + 0.05),
                at(end, u_string, slope(end) + 0.05),
                0.07,
                0.42,
                WALNUT,
            );
            // Balusters on every other tread, a handrail along the slope.
            for k in (0..f.steps).step_by(2) {
                let a = -r + (k as f32 + 0.5) * f.run();
                let foot = f.step_top(k);
                self.post(at(a, u_rail, foot), slope(a) + 0.92 - foot, 0.045, BALUSTER);
            }
            self.beam(
                at(-r, u_rail, slope(-r) + 0.95),
                at(r, u_rail, slope(r) + 0.95),
                0.1,
                0.07,
                WALNUT,
            );
            self.newel(at(-r, u_rail, f.y0), RAIL_H + 0.06, newels);
            self.newel(at(r, u_rail, f.y1), RAIL_H + 0.06, newels);
            // What stops a body or a book going over the side: a guard from the floor
            // to a railing's height above the flight, in five steps.
            let segs = 5;
            for k in 0..segs {
                let a0 = -r + k as f32 * 2.0 * r / segs as f32;
                let a1 = a0 + 2.0 * r / segs as f32;
                let (mn, mx) = rect(a0, a1, s * w, s * (w - RAIL_T));
                let top = slope(a1) + RAIL_H;
                self.collide(Vec3::new(mn.x, floor, mn.y), Vec3::new(mx.x, top, mx.y), Stops::RAIL);
            }
        }
        // Its high end, a stone face down to the floor below.
        let (mn, mx) = rect(r, r + 0.1, -w, w);
        self.collide(Vec3::new(mn.x, floor, mn.y), Vec3::new(mx.x, f.y1, mx.y), Stops::ALL);
    }

    /// A wooden box along the line `a`→`b`, `w` across and `h` tall, kept upright.
    pub(super) fn beam(&mut self, a: Vec3, b: Vec3, w: f32, h: f32, rgb: [f32; 3]) {
        let d = b - a;
        let len = d.length();
        let x = d / len;
        let z = x.cross(Vec3::Y).normalize();
        let y = z.cross(x);
        let rot = Quat::from_mat3(&Mat3::from_cols(x, y, z));
        self.decor(Inst::new(
            (a + b) * 0.5,
            rot,
            Vec3::new(len, h, w),
            kind::WOOD,
            [rgb[0], rgb[1], rgb[2], 1.0],
            0,
        ));
    }

    /// A wooden post `w` square standing `h` tall on `foot`.
    pub(super) fn post(&mut self, foot: Vec3, h: f32, w: f32, rgb: [f32; 3]) {
        self.decor(Inst::aabox(
            foot + Vec3::Y * (h * 0.5),
            Vec3::new(w, h, w),
            kind::WOOD,
            rgb,
        ));
    }

    /// A newel post with a cap at `foot`, unless one already stands there.
    pub(super) fn newel(&mut self, foot: Vec3, h: f32, newels: &mut Vec<Vec3>) {
        if newels.iter().any(|n| n.distance(foot) < 0.3) {
            return;
        }
        newels.push(foot);
        self.post(foot, h, 0.15, WALNUT);
        self.decor(Inst::aabox(
            foot + Vec3::Y * (h + 0.03),
            Vec3::new(0.2, 0.06, 0.2),
            kind::WOOD,
            [0.12, 0.06, 0.028],
        ));
    }

    /// A railing on the floor along the axis-aligned line `a`→`b`: a bottom rail,
    /// balusters, a handrail, newel posts at its ends, and a guard in the way of
    /// bodies and books (seen past).
    pub(super) fn railing(&mut self, a: Vec3, b: Vec3, newels: &mut Vec<Vec3>) {
        let up = Vec3::Y;
        self.beam(a + up * 0.05, b + up * 0.05, 0.1, 0.1, WALNUT);
        let n = (a.distance(b) / 0.28).round().max(1.0) as usize;
        for i in 1..n {
            let p = a.lerp(b, i as f32 / n as f32);
            self.post(p + up * 0.1, RAIL_H - 0.16, 0.045, BALUSTER);
        }
        self.beam(a + up * (RAIL_H - 0.03), b + up * (RAIL_H - 0.03), 0.12, 0.07, WALNUT);
        self.newel(a, RAIL_H + 0.06, newels);
        self.newel(b, RAIL_H + 0.06, newels);
        let d = (b - a).normalize();
        let across = Vec3::new(d.z, 0.0, d.x).abs() * (RAIL_T * 0.5);
        self.collide(a.min(b) - across, a.max(b) + across + up * RAIL_H, Stops::RAIL);
    }

    /// Wooden panelling 1.1 m high on the face of a wall, standing on the floor at
    /// `y`: a strip `(mn, mx)` in front of the face, facing `toward` the room. Its cap
    /// fills the strip; the panel below is set back from the room.
    pub(super) fn wainscot(&mut self, y: f32, (mn, mx): (Vec2, Vec2), toward: Vec2) {
        let back = toward * 0.02;
        let (pmn, pmx) = (mn - back.min(Vec2::ZERO), mx - back.max(Vec2::ZERO));
        let panel = (Vec3::new(pmn.x, y, pmn.y), Vec3::new(pmx.x, y + 1.1, pmx.y));
        self.decor(Inst::minmax(panel.0, panel.1, kind::WOOD, [0.13, 0.065, 0.03]));
        let cap = (Vec3::new(mn.x, y + 1.1, mn.y), Vec3::new(mx.x, y + 1.16, mx.y));
        self.decor(Inst::minmax(cap.0, cap.1, kind::WOOD, WALNUT));
    }

    /// The stone over a doorway and a frame round it on both faces of its wall. The
    /// opening is `2 * half` wide and `h` tall, centred on `c` (on the floor, in the
    /// middle of the wall's thickness `t`) across a wall running along `along`; the
    /// wall stands `wall_h` tall.
    pub(super) fn doorway(&mut self, c: Vec3, along: Vec2, half: f32, h: f32, t: f32, wall_h: f32) {
        let n = Vec2::new(-along.y, along.x);
        // A box over `a0..a1` along the wall, `o0..o1` through it, `y0..y1` up.
        let part = |lib: &mut Self, (a0, a1): (f32, f32), (o0, o1): (f32, f32), (y0, y1): (f32, f32), rgb| {
            let p = Vec2::new(c.x, c.z) + along * a0 + n * o0;
            let q = Vec2::new(c.x, c.z) + along * a1 + n * o1;
            let (mn, mx) = (p.min(q), p.max(q));
            let i = Inst::minmax(
                Vec3::new(mn.x, c.y + y0, mn.y),
                Vec3::new(mx.x, c.y + y1, mx.y),
                kind::STONE,
                rgb,
            );
            lib.decor(i);
        };
        // Over the opening: books can't be stopped by something off the floor (see
        // `World::add_solid`), but sight is.
        part(self, (-half, half), (-t * 0.5, t * 0.5), (h, wall_h), PIER_STONE);
        let p = Vec2::new(c.x, c.z);
        let (a, b) = (p - along * half - n * t * 0.5, p + along * half + n * t * 0.5);
        let (mn, mx) = (a.min(b), a.max(b));
        self.collide(
            Vec3::new(mn.x, c.y + h, mn.y),
            Vec3::new(mx.x, c.y + wall_h, mx.y),
            Stops::SIGHT,
        );
        for face in [-1.0f32, 1.0] {
            let o = |d: f32| {
                let (u, v) = (face * t * 0.5, face * (t * 0.5 + d));
                (u.min(v), u.max(v))
            };
            for side in [-1.0f32, 1.0] {
                let (u, v) = (side * half, side * (half + 0.3));
                part(self, (u.min(v), u.max(v)), o(0.07), (0.0, h + 0.3), PALE_STONE);
            }
            part(self, (-half - 0.36, half + 0.36), o(0.09), (h, h + 0.38), PALE_STONE);
        }
    }

    /// A stone pier at `c` (on the ground) rising to `top`, with a base and a capital
    /// on each storey (floors at `floors`, `storey_h` tall), in the way on each.
    pub(super) fn pier(&mut self, c: Vec3, top: f32, floors: &[f32], storey_h: f32) {
        self.decor(Inst::minmax(
            c - Vec3::new(0.35, 0.0, 0.35),
            c + Vec3::new(0.35, top, 0.35),
            kind::STONE,
            PIER_STONE,
        ));
        for &y in floors {
            let base = c + Vec3::Y * y;
            self.decor(Inst::aabox(
                base + Vec3::Y * 0.15,
                Vec3::new(0.9, 0.3, 0.9),
                kind::STONE,
                PALE_STONE,
            ));
            self.decor(Inst::aabox(
                base + Vec3::Y * (storey_h - 0.12),
                Vec3::new(0.95, 0.24, 0.95),
                kind::STONE,
                PALE_STONE,
            ));
            self.collide(
                base - Vec3::new(0.45, 0.0, 0.45),
                base + Vec3::new(0.45, storey_h, 0.45),
                Stops::ALL,
            );
        }
    }
}
