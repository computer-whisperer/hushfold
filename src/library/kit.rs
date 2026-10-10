//! Pieces a layout places: furniture, lights, railings, panelling, door frames and
//! piers. Each draws itself and says what of it is in the way, so a layout only
//! decides where things go.

use super::*;
use std::ops::Range;

/// A railing's height, and the thickness of the guard standing for it.
const RAIL_H: f32 = 1.05;
pub(super) const RAIL_T: f32 = 0.12;
const BALUSTER: [f32; 3] = [0.2, 0.1, 0.045];
const PALE_STONE: [f32; 3] = [0.48, 0.45, 0.4];
const PIER_STONE: [f32; 3] = [0.42, 0.39, 0.35];
const STEP_STONE: [f32; 3] = [0.4, 0.37, 0.33];
/// Thickness of a stair's wooden tread.
const TREAD_T: f32 = 0.05;
/// A wall case's back board, between the wall and its shelves.
const WALL_BACK: f32 = 0.04;
/// A bookcase's stiles, its ends: wider than a side board, reaching in to just
/// outside the books, and standing proud of the faces like pilasters.
const STILE_IN: f32 = UNIT_W * 0.5 - SIDE_T - 0.03;
const STILE_OUT: f32 = UNIT_W * 0.5 + 0.01;
const STILE_PROUD: f32 = 0.025;
/// The frieze across the top of a face, clear of the tallest book on the top shelf.
const FRIEZE_Y: f32 = 2.98;
const CASE_FIELD: [f32; 3] = [0.25, 0.12, 0.05];
const CASE_TRIM: [f32; 3] = [0.12, 0.06, 0.028];

impl Library {
    /// A brass wall sconce with a candle flame, lighting the room in front of it: a
    /// backplate on the wall 0.15 behind `p`, an arm out to a drip pan and a candle,
    /// and the glow of its glass chimney round the flame.
    pub(super) fn sconce(&mut self, p: Vec3, n: Vec3) {
        let across = Vec3::Y.cross(n).abs();
        // A box `w` across the wall, `out0..out1` out from `p`, `up0..up1` above it.
        let part = |lib: &mut Self, w: f32, (out0, out1): (f32, f32), (up0, up1): (f32, f32), k: u32, rgb| {
            let c = p + n * ((out0 + out1) * 0.5) + Vec3::Y * ((up0 + up1) * 0.5);
            let size = across * w + Vec3::Y * (up1 - up0) + n.abs() * (out1 - out0);
            lib.decor(Inst::aabox(c, size, k, rgb));
        };
        let mut flame = Inst::aabox(
            p + Vec3::Y * 0.24,
            Vec3::new(0.035, 0.08, 0.035),
            kind::EMISSIVE,
            [1.0, 0.55, 0.18],
        );
        flame.color[3] = 12.0;
        flame.param = 1;
        self.decor(flame);
        part(
            self,
            0.045,
            (-0.0225, 0.0225),
            (0.055, 0.2),
            kind::EMISSIVE,
            [0.9, 0.85, 0.7],
        );
        part(self, 0.11, (-0.055, 0.055), (0.035, 0.05), kind::METAL, BRASS);
        part(self, 0.055, (-0.0275, 0.0275), (0.0, 0.055), kind::METAL, BRASS);
        part(self, 0.026, (-0.013, 0.013), (-0.075, 0.0), kind::METAL, BRASS);
        part(self, 0.02, (-0.13, 0.0), (-0.07, -0.048), kind::METAL, BRASS);
        part(self, 0.065, (-0.138, -0.12), (-0.095, -0.025), kind::METAL, BRASS);
        part(self, 0.1, (-0.15, -0.138), (-0.2, 0.08), kind::METAL, BRASS);
        part(self, 0.05, (-0.15, -0.135), (0.08, 0.13), kind::METAL, BRASS);
        self.static_trans.push(Inst::new(
            p + Vec3::Y * 0.22,
            Quat::IDENTITY,
            Vec3::new(0.16, 0.38, 0.16),
            kind::SHAFT,
            [1.0, 0.62, 0.28, 0.35],
            0,
        ));
        self.light(
            p + n * 0.4 + Vec3::Y * 0.3,
            8.0,
            [1.0, 0.62, 0.3],
            1.3,
            LightKind::Flicker,
        );
    }

    /// A reading table centred at `c` (on the floor), with four chairs and a candle: a
    /// leather-topped table on turned legs with an apron and an H stretcher, and
    /// chairs with cushioned seats and a splat in the back.
    pub(super) fn reading_table(&mut self, c: Vec3) {
        let part = |lib: &mut Self, at: Vec3, size: Vec3, k: u32, rgb| lib.decor(Inst::aabox(at, size, k, rgb));
        // Its top, a moulding under the edge, and the leather, a hair proud of the wood.
        part(
            self,
            c + Vec3::Y * 0.7775,
            Vec3::new(2.0, 0.003, 0.84),
            kind::CLOTH,
            [0.05, 0.1, 0.06],
        );
        part(
            self,
            c + Vec3::Y * 0.748,
            Vec3::new(2.4, 0.056, 1.2),
            kind::WOOD,
            WALNUT,
        );
        part(
            self,
            c + Vec3::Y * 0.705,
            Vec3::new(2.34, 0.03, 1.14),
            kind::WOOD,
            CASE_TRIM,
        );
        for s in [-1.0f32, 1.0] {
            part(
                self,
                c + Vec3::new(0.0, 0.635, s * 0.45),
                Vec3::new(2.0, 0.11, 0.03),
                kind::WOOD,
                WALNUT,
            );
            part(
                self,
                c + Vec3::new(s * 1.05, 0.635, 0.0),
                Vec3::new(0.03, 0.11, 0.8),
                kind::WOOD,
                WALNUT,
            );
            part(
                self,
                c + Vec3::new(s * 1.05, 0.12, 0.0),
                Vec3::new(0.035, 0.04, 0.9),
                kind::WOOD,
                DARK_WOOD,
            );
        }
        part(
            self,
            c + Vec3::Y * 0.1275,
            Vec3::new(2.1, 0.035, 0.035),
            kind::WOOD,
            DARK_WOOD,
        );
        for (sx, sz) in [(-1.0, -1.0), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)] {
            let foot = c + Vec3::new(sx * 1.05, 0.0, sz * 0.45);
            self.turned_leg(foot, 0.69, 0.1, DARK_WOOD);
        }
        self.collide(
            c - Vec3::new(1.25, 0.0, 0.65),
            c + Vec3::new(1.25, TABLE_H, 0.65),
            Stops::ALL,
        );
        let chair = [0.2, 0.09, 0.04];
        for sz in [-1.0f32, 1.0] {
            for sx in [-0.6f32, 0.6] {
                let ch = c + Vec3::new(sx, 0.0, sz * 1.0);
                // The back is on the side away from the table (+sz).
                let at = |x: f32, y: f32, z: f32| ch + Vec3::new(x, y, sz * z);
                part(
                    self,
                    at(0.0, 0.4875, -0.02),
                    Vec3::new(0.4, 0.035, 0.36),
                    kind::CLOTH,
                    [0.24, 0.05, 0.04],
                );
                part(
                    self,
                    at(0.0, 0.4475, 0.0),
                    Vec3::new(0.46, 0.045, 0.44),
                    kind::WOOD,
                    chair,
                );
                part(self, at(0.0, 0.95, 0.2), Vec3::new(0.5, 0.1, 0.04), kind::WOOD, chair);
                part(
                    self,
                    at(0.0, 0.685, 0.2),
                    Vec3::new(0.14, 0.43, 0.02),
                    kind::WOOD,
                    chair,
                );
                for s in [-1.0f32, 1.0] {
                    part(
                        self,
                        at(s * 0.2, 0.46, 0.2),
                        Vec3::new(0.045, 0.92, 0.045),
                        kind::WOOD,
                        DARK_WOOD,
                    );
                    part(
                        self,
                        at(s * 0.2, 0.2125, -0.2),
                        Vec3::new(0.045, 0.425, 0.045),
                        kind::WOOD,
                        DARK_WOOD,
                    );
                    part(
                        self,
                        at(s * 0.2, 0.155, 0.0),
                        Vec3::new(0.025, 0.03, 0.36),
                        kind::WOOD,
                        DARK_WOOD,
                    );
                }
                self.collide(
                    ch - Vec3::new(0.26, 0.0, 0.26),
                    ch + Vec3::new(0.26, 1.05, 0.26),
                    Stops::RAIL,
                );
            }
        }
        // A brass candlestick.
        let candle = c + Vec3::new(0.7, TABLE_H, 0.0);
        part(
            self,
            candle + Vec3::Y * 0.009,
            Vec3::new(0.12, 0.018, 0.12),
            kind::METAL,
            BRASS,
        );
        part(
            self,
            candle + Vec3::Y * 0.069,
            Vec3::new(0.025, 0.102, 0.025),
            kind::METAL,
            BRASS,
        );
        part(
            self,
            candle + Vec3::Y * 0.125,
            Vec3::new(0.08, 0.01, 0.08),
            kind::METAL,
            BRASS,
        );
        part(
            self,
            candle + Vec3::Y * 0.195,
            Vec3::new(0.04, 0.13, 0.04),
            kind::PLAIN,
            [0.8, 0.75, 0.6],
        );
        let mut flame = Inst::aabox(
            candle + Vec3::Y * 0.285,
            Vec3::new(0.025, 0.05, 0.025),
            kind::EMISSIVE,
            [1.0, 0.55, 0.18],
        );
        flame.color[3] = 14.0;
        flame.param = 1;
        self.decor(flame);
        self.light(candle + Vec3::Y * 0.35, 4.5, [1.0, 0.6, 0.28], 0.7, LightKind::Flicker);
    }

    /// A two-tier brass chandelier at `ch` hung from `top_y`: a ring of candles, a
    /// smaller ring above it, arms from a central stem, and crystal drops that glow
    /// faintly under the ring.
    pub(super) fn chandelier(&mut self, ch: Vec3, top_y: f32) {
        let brass = [BRASS[0], BRASS[1], BRASS[2], 1.0];
        let candles = |lib: &mut Self, p: Vec3| {
            lib.decor(Inst::aabox(
                p + Vec3::Y * 0.04,
                Vec3::new(0.08, 0.02, 0.08),
                kind::METAL,
                BRASS,
            ));
            lib.decor(Inst::aabox(
                p + Vec3::Y * 0.12,
                Vec3::new(0.04, 0.14, 0.04),
                kind::PLAIN,
                [0.8, 0.75, 0.6],
            ));
            let mut flame = Inst::aabox(
                p + Vec3::Y * 0.22,
                Vec3::new(0.03, 0.06, 0.03),
                kind::EMISSIVE,
                [1.0, 0.55, 0.18],
            );
            flame.color[3] = 14.0;
            flame.param = 1;
            lib.decor(flame);
        };
        // (radius, height above `ch`, segments, segment length).
        for (r, y, n, len) in [(1.3f32, 0.0f32, 12, 0.72f32), (0.7, 0.55, 8, 0.58)] {
            for k in 0..n {
                let a = (k as f32 + 0.5) / n as f32 * TAU;
                let dir = Vec3::new(a.cos(), 0.0, a.sin());
                let p = ch + dir * r + Vec3::Y * y;
                self.decor(Inst::new(
                    p,
                    Quat::from_rotation_y(-a),
                    Vec3::new(0.07, 0.05, len),
                    kind::METAL,
                    brass,
                    0,
                ));
                candles(self, p + Vec3::Y * 0.025);
                if k % 2 == 0 {
                    // An arm out from the stem.
                    self.bar(
                        ch + Vec3::Y * (y - 0.25),
                        p - dir * 0.04,
                        0.03,
                        0.03,
                        kind::METAL,
                        BRASS,
                    );
                }
                if r > 1.0 {
                    // A crystal drop between this candle and the next.
                    let b = (k as f32 + 1.0) / n as f32 * TAU;
                    let q = ch + Vec3::new(b.cos(), 0.0, b.sin()) * (r * (TAU / n as f32 * 0.5).cos());
                    self.decor(Inst::new(
                        q - Vec3::Y * 0.12,
                        Quat::from_rotation_y(b + TAU / 8.0),
                        Vec3::new(0.035, 0.13, 0.035),
                        kind::CRYSTAL,
                        [0.65, 0.78, 1.0, 0.9],
                        0,
                    ));
                }
            }
        }
        // The stem, with knops along it, a finial and a big drop below, and a crown.
        let stem = |y0: f32, y1: f32, w: f32| {
            Inst::minmax(ch + Vec3::new(-w, y0, -w), ch + Vec3::new(w, y1, w), kind::METAL, BRASS)
        };
        for (y0, y1, w) in [
            (-0.45, 1.35, 0.03),
            (-0.32, -0.12, 0.09),
            (0.18, 0.3, 0.07),
            (0.62, 0.7, 0.06),
            (1.35, 1.45, 0.12),
        ] {
            self.decor(stem(y0, y1, w));
        }
        self.decor(Inst::new(
            ch - Vec3::Y * 0.56,
            Quat::from_rotation_y(TAU / 8.0),
            Vec3::new(0.08, 0.2, 0.08),
            kind::CRYSTAL,
            [0.65, 0.78, 1.0, 1.3],
            0,
        ));
        // Chains from the ring to the crown, and one from the crown to the ceiling.
        for k in 0..3 {
            let a = k as f32 / 3.0 * TAU;
            let top = ch + Vec3::Y * 1.4;
            let bottom = ch + Vec3::new(a.cos(), 0.0, a.sin()) * 1.25;
            let d = top - bottom;
            self.decor(Inst::new(
                (top + bottom) * 0.5,
                Quat::from_rotation_arc(Vec3::Y, d.normalize()),
                Vec3::new(0.02, d.length(), 0.02),
                kind::METAL,
                [0.1, 0.08, 0.05, 1.0],
                0,
            ));
        }
        self.decor(Inst::minmax(
            ch + Vec3::new(-0.015, 1.45, -0.015),
            Vec3::new(ch.x + 0.015, top_y, ch.z + 0.015),
            kind::METAL,
            [0.1, 0.08, 0.05],
        ));
        self.light(ch + Vec3::Y * 0.2, 20.0, [1.0, 0.68, 0.38], 4.0, LightKind::Flicker);
    }

    /// A double-sided bookcase standing at `c` (floor level), running along x with its
    /// faces toward ±z, or (`along_x` false) running along z, faces toward ±x. Returns
    /// its faces, toward - and + of that axis: (front plane centre at floor level,
    /// outward normal), as `add_category` takes them.
    pub(super) fn bookcase(&mut self, c: Vec3, along_x: bool) -> [(Vec3, Vec3); 2] {
        // Boxes are laid out for a case along x; `sw` turns offsets and sizes.
        let sw = |v: Vec3| if along_x { v } else { Vec3::new(v.z, v.y, v.x) };
        let hw = UNIT_W * 0.5;
        let hd = UNIT_DEPTH * 0.5;
        for (mn, mx, k, color) in case_boxes(-hd, hd, true) {
            self.decor(Inst::new(
                c + sw((mn + mx) * 0.5),
                Quat::IDENTITY,
                sw(mx - mn),
                k,
                color,
                0,
            ));
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
        let n = sw(Vec3::Z);
        [(c - n * hd, -n), (c + n * hd, n)]
    }

    /// A single-sided bookcase with its back on a wall: `at` is the middle of its back
    /// at floor level, `normal` (along x or z) the way it faces. Returns its face, as
    /// `bookcase` does.
    #[allow(dead_code)] // TEMP(visual pass): until a department stands one on a wall.
    pub(super) fn wall_bookcase(&mut self, at: Vec3, normal: Vec3) -> (Vec3, Vec3) {
        // Laid out across (x), up (y) and out from the wall (z), then turned to face
        // `normal`.
        let right = Vec3::Y.cross(normal);
        let place = |off: Vec3| at + right * off.x + Vec3::Y * off.y + normal * off.z;
        let hw = UNIT_W * 0.5;
        let d = WALL_BACK + FACE_DEPTH;
        for (mn, mx, k, color) in case_boxes(0.0, d, false) {
            let (p, q) = (place(mn), place(mx));
            self.decor(Inst::new((p + q) * 0.5, Quat::IDENTITY, (q - p).abs(), k, color, 0));
        }
        let (a, b) = (
            place(Vec3::new(-hw - 0.04, 0.0, 0.0)),
            place(Vec3::new(hw + 0.04, UNIT_H + 0.13, d + 0.04)),
        );
        self.collide(a.min(b), a.max(b), Stops::ALL);
        (at + normal * d, normal)
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
            // It has a moulding along its top edge and a bead along its bottom one.
            let end = r - 0.46;
            let (lo, hi) = (at(-r, u_string, slope(-r) + 0.05), at(end, u_string, slope(end) + 0.05));
            self.beam(lo, hi, 0.07, 0.42, WALNUT);
            let up = upright_frame((hi - lo).normalize()).y_axis;
            self.beam(lo + up * 0.225, hi + up * 0.225, 0.1, 0.035, CASE_TRIM);
            self.beam(lo - up * 0.22, hi - up * 0.22, 0.09, 0.03, CASE_TRIM);
            // Balusters on every other tread, a handrail along the slope.
            for k in (0..f.steps).step_by(2) {
                let a = -r + (k as f32 + 0.5) * f.run();
                let foot = f.step_top(k);
                self.baluster(at(a, u_rail, foot), slope(a) + 0.92 - foot);
            }
            let (lo, hi) = (at(-r, u_rail, slope(-r) + 0.95), at(r, u_rail, slope(r) + 0.95));
            self.beam(lo, hi, 0.1, 0.07, WALNUT);
            let up = upright_frame((hi - lo).normalize()).y_axis;
            self.beam(lo + up * 0.0475, hi + up * 0.0475, 0.065, 0.025, WALNUT);
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
        self.bar(a, b, w, h, kind::WOOD, rgb);
    }

    /// A box of `k` along the line `a`→`b` (not a vertical one), `w` across and `h`
    /// tall, kept upright.
    fn bar(&mut self, a: Vec3, b: Vec3, w: f32, h: f32, k: u32, rgb: [f32; 3]) {
        let d = b - a;
        let len = d.length();
        let rot = Quat::from_mat3(&upright_frame(d / len));
        self.decor(Inst::new(
            (a + b) * 0.5,
            rot,
            Vec3::new(len, h, w),
            k,
            [rgb[0], rgb[1], rgb[2], 1.0],
            0,
        ));
    }

    /// A turned leg standing on `foot`, `h` tall and `w` square at its top: a square
    /// block where rails meet it, a swelling below, a slim shaft and a foot.
    fn turned_leg(&mut self, foot: Vec3, h: f32, w: f32, rgb: [f32; 3]) {
        for (y0, y1, s) in [
            (h - 0.13, h, 1.0),
            (h * 0.4, h * 0.66, 0.85),
            (0.06, h - 0.13, 0.55),
            (0.0, 0.06, 0.75),
        ] {
            let half = Vec3::new(w * s * 0.5, 0.0, w * s * 0.5);
            self.decor(Inst::minmax(
                foot - half + Vec3::Y * y0,
                foot + half + Vec3::Y * y1,
                kind::WOOD,
                rgb,
            ));
        }
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

    /// A turned baluster standing `h` tall on `foot`: a slim shaft with a vase low on
    /// it and a collar near its top.
    fn baluster(&mut self, foot: Vec3, h: f32) {
        self.post(foot, h, 0.034, BALUSTER);
        self.post(foot + Vec3::Y * (h * 0.1), h * 0.32, 0.058, BALUSTER);
        self.post(foot + Vec3::Y * (h * 0.78), h * 0.06, 0.05, BALUSTER);
    }

    /// A newel post at `foot`, `h` tall to its cap, unless one already stands there: a
    /// plinth, a shaft with a collar under the cap, and a round finial (two boxes, one
    /// turned an eighth).
    pub(super) fn newel(&mut self, foot: Vec3, h: f32, newels: &mut Vec<Vec3>) {
        if newels.iter().any(|n| n.distance(foot) < 0.3) {
            return;
        }
        newels.push(foot);
        self.post(foot, 0.22, 0.19, CASE_TRIM);
        self.post(foot + Vec3::Y * 0.22, h - 0.22, 0.15, WALNUT);
        self.post(foot + Vec3::Y * (h - 0.09), 0.05, 0.18, CASE_TRIM);
        self.post(foot + Vec3::Y * h, 0.06, 0.21, CASE_TRIM);
        for turn in [0.0, TAU / 8.0] {
            self.decor(Inst::new(
                foot + Vec3::Y * (h + 0.12),
                Quat::from_rotation_y(turn),
                Vec3::new(0.1, 0.12, 0.1),
                kind::WOOD,
                [WALNUT[0], WALNUT[1], WALNUT[2], 1.0],
                0,
            ));
        }
    }

    /// A railing on the floor along the axis-aligned line `a`→`b`: a bottom rail,
    /// balusters, a moulded handrail, newel posts at its ends, and a guard in the way
    /// of bodies and books (seen past).
    pub(super) fn railing(&mut self, a: Vec3, b: Vec3, newels: &mut Vec<Vec3>) {
        let up = Vec3::Y;
        self.beam(a + up * 0.05, b + up * 0.05, 0.1, 0.1, WALNUT);
        let n = (a.distance(b) / 0.28).round().max(1.0) as usize;
        for i in 1..n {
            let p = a.lerp(b, i as f32 / n as f32);
            self.baluster(p + up * 0.1, RAIL_H - 0.16);
        }
        self.beam(a + up * (RAIL_H - 0.03), b + up * (RAIL_H - 0.03), 0.12, 0.07, WALNUT);
        self.beam(
            a + up * (RAIL_H + 0.0175),
            b + up * (RAIL_H + 0.0175),
            0.07,
            0.025,
            WALNUT,
        );
        self.newel(a, RAIL_H + 0.06, newels);
        self.newel(b, RAIL_H + 0.06, newels);
        let d = (b - a).normalize();
        let across = Vec3::new(d.z, 0.0, d.x).abs() * (RAIL_T * 0.5);
        self.collide(a.min(b) - across, a.max(b) + across + up * RAIL_H, Stops::RAIL);
    }

    /// Wooden panelling 1.1 m high on the face of a wall, standing on the floor at
    /// `y`: a strip `(mn, mx)` in front of the face, facing `toward` the room. Its cap
    /// fills the strip; the panel below is set back from the room, with raised fields
    /// in bays about 1.25 m wide over a skirting, and a bead under the cap. Nothing
    /// stands out of the strip.
    pub(super) fn wainscot(&mut self, y: f32, (mn, mx): (Vec2, Vec2), toward: Vec2) {
        let along = Vec2::new(toward.y.abs(), toward.x.abs());
        let (u0, u1) = (mn.dot(along), mx.dot(along));
        let front = mn.dot(toward).max(mx.dot(toward));
        let back = mn.dot(toward).min(mx.dot(toward));
        // `u0..u1` along the wall, `d0..d1` behind the strip's front, `y0..y1` up.
        let part = |lib: &mut Self, (u0, u1): (f32, f32), (d0, d1): (f32, f32), (y0, y1): (f32, f32), rgb| {
            let (p, q) = (along * u0 + toward * (front - d1), along * u1 + toward * (front - d0));
            let (lo, hi) = (p.min(q), p.max(q));
            lib.decor(Inst::minmax(
                Vec3::new(lo.x, y + y0, lo.y),
                Vec3::new(hi.x, y + y1, hi.y),
                kind::WOOD,
                rgb,
            ));
        };
        let bays = ((u1 - u0) / 1.25).round().max(1.0);
        let bay = (u1 - u0) / bays;
        for k in 0..bays as usize {
            let m = u0 + (k as f32 + 0.5) * bay;
            let half = (bay * 0.5 - 0.12).max(0.05);
            part(
                self,
                (m - half, m + half),
                (0.009, 0.025),
                (0.27, 0.93),
                [0.17, 0.085, 0.037],
            );
        }
        let inside = (u0 + 0.01, u1 - 0.01);
        part(self, inside, (0.006, 0.025), (0.0, 0.16), [0.08, 0.04, 0.018]);
        part(self, inside, (0.004, 0.025), (1.04, 1.095), WALNUT);
        part(self, (u0, u1), (0.0, front - back), (1.1, 1.16), WALNUT);
        part(self, (u0, u1), (0.02, front - back), (0.0, 1.1), [0.11, 0.055, 0.025]);
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
        // On each face an architrave: plinth blocks, jambs with a bead on their inner
        // edge, a lintel with a keystone, and a cornice over it, no wider than the
        // jambs where panelling meets them.
        let pale = [0.53, 0.5, 0.45];
        for face in [-1.0f32, 1.0] {
            let o = |d: f32| {
                let (u, v) = (face * t * 0.5, face * (t * 0.5 + d));
                (u.min(v), u.max(v))
            };
            part(self, (-0.16, 0.16), o(0.12), (h - 0.04, h + 0.36), pale);
            part(self, (-half - 0.42, half + 0.42), o(0.13), (h + 0.33, h + 0.43), pale);
            part(
                self,
                (-half - 0.36, half + 0.36),
                o(0.1),
                (h + 0.28, h + 0.33),
                PALE_STONE,
            );
            part(self, (-half - 0.07, half + 0.07), o(0.1), (h, h + 0.075), PALE_STONE);
            for side in [-1.0f32, 1.0] {
                let a = |a0: f32, a1: f32| ((side * a0).min(side * a1), (side * a0).max(side * a1));
                part(self, a(half, half + 0.3), o(0.11), (0.0, 0.34), pale);
                part(self, a(half, half + 0.07), o(0.1), (0.34, h), PALE_STONE);
                part(self, a(half + 0.02, half + 0.3), o(0.07), (0.34, h + 0.07), PALE_STONE);
            }
            part(
                self,
                (-half - 0.3, half + 0.3),
                o(0.07),
                (h + 0.07, h + 0.28),
                PALE_STONE,
            );
        }
    }

    /// A stone pier at `c` (on the ground) rising to `top`, with a moulded base and
    /// capital on each storey (floors at `floors`, `storey_h` tall), in the way on each.
    pub(super) fn pier(&mut self, c: Vec3, top: f32, floors: &[f32], storey_h: f32) {
        for &y in floors {
            let base = c + Vec3::Y * y;
            // (bottom, top, width) of a plinth and a torus, then a necking, an echinus
            // and an abacus under the floor above.
            let h = storey_h;
            for (y0, y1, w) in [
                (0.0, 0.22, 0.9),
                (0.22, 0.3, 0.8),
                (h - 0.36, h - 0.3, 0.78),
                (h - 0.3, h - 0.18, 0.86),
                (h - 0.18, h, 0.96),
            ] {
                let half = Vec3::new(w * 0.5, 0.0, w * 0.5);
                self.decor(Inst::minmax(
                    base - half + Vec3::Y * y0,
                    base + half + Vec3::Y * y1,
                    kind::STONE,
                    PALE_STONE,
                ));
            }
            self.collide(
                base - Vec3::new(0.45, 0.0, 0.45),
                base + Vec3::new(0.45, storey_h, 0.45),
                Stops::ALL,
            );
        }
        self.decor(Inst::minmax(
            c - Vec3::new(0.35, 0.0, 0.35),
            c + Vec3::new(0.35, top, 0.35),
            kind::STONE,
            PIER_STONE,
        ));
    }
}

/// Axes for a box along the unit direction `x` (not vertical): its y as near up as
/// `x` allows, its z level.
fn upright_frame(x: Vec3) -> Mat3 {
    let z = x.cross(Vec3::Y).normalize();
    Mat3::from_cols(x, z.cross(x), z)
}

/// A bookcase's boxes in its own frame (x along it from its middle, y up from the
/// floor, z through it) as (min, max, kind, colour): the carcass spans `z0..z1` with
/// shelves opening toward +z, and toward -z too if `both`; otherwise its back stands
/// on a wall at `z0`. The nearest parts come first, so they hide what is behind them
/// before it is shaded.
fn case_boxes(z0: f32, z1: f32, both: bool) -> Vec<(Vec3, Vec3, u32, [f32; 4])> {
    let mut out = Vec::with_capacity(40);
    let mut add = |(mn, mx): (Vec3, Vec3), k: u32, rgb: [f32; 3]| out.push((mn, mx, k, [rgb[0], rgb[1], rgb[2], 1.0]));
    let faces: &[f32] = if both { &[-1.0, 1.0] } else { &[1.0] };
    // `a..b` out from face `s`'s plane.
    let before = |s: f32, a: f32, b: f32| {
        let f = if s > 0.0 { z1 } else { z0 };
        (f + s * a).min(f + s * b)..(f + s * a).max(f + s * b)
    };
    // Through the carcass, standing `proud` out of each face; on a wall case `off`
    // the wall, so no two of its parts share the wall's plane.
    let through = |proud: f32, off: f32| (if both { z0 - proud } else { z0 + off })..z1 + proud;
    let bx =
        |x: Range<f32>, y0: f32, y1: f32, z: Range<f32>| (Vec3::new(x.start, y0, z.start), Vec3::new(x.end, y1, z.end));
    let wide = |hx: f32| -hx..hx;
    let pitch = UNIT_PITCH * 0.5;
    for &s in faces {
        // Brass label holders where each shelf's label hangs, the bottom one on the plinth.
        for row in 0..ROWS {
            let top = row_floor_y(row);
            let holder = if row + 1 == ROWS {
                bx(wide(0.06), 0.07, 0.13, before(s, 0.04, 0.046))
            } else {
                bx(wide(0.06), top - 0.029, top - 0.001, before(s, 0.0, 0.006))
            };
            add(holder, kind::METAL, BRASS);
        }
        // A rail for a library ladder, resting on the stiles under their capitals.
        add(
            bx(wide(pitch), 2.885, 2.905, before(s, STILE_PROUD, STILE_PROUD + 0.02)),
            kind::METAL,
            BRASS,
        );
        add(
            bx(wide(STILE_IN), FRIEZE_Y, UNIT_H, before(s, 0.0, 0.015)),
            kind::WOOD,
            WALNUT,
        );
    }
    // Each stile is a pilaster on both faces: a capital under the frieze and an inlaid
    // flute, each one box through the case. Its end has two raised fields.
    for e in [-1.0f32, 1.0] {
        let x = |a: f32, b: f32| (e * a).min(e * b)..(e * a).max(e * b);
        let mid = (STILE_IN + STILE_OUT) * 0.5;
        add(
            bx(
                x(STILE_IN - 0.012, STILE_OUT + 0.008),
                FRIEZE_Y - 0.06,
                FRIEZE_Y,
                through(STILE_PROUD + 0.012, 0.01),
            ),
            kind::WOOD,
            CASE_TRIM,
        );
        add(
            bx(
                x(mid - 0.018, mid + 0.018),
                0.3,
                FRIEZE_Y - 0.14,
                through(STILE_PROUD + 0.003, 0.01),
            ),
            kind::WOOD,
            [0.07, 0.034, 0.016],
        );
        add(
            bx(x(STILE_IN, STILE_OUT), 0.0, UNIT_H, through(STILE_PROUD, 0.004)),
            kind::WOOD,
            WALNUT,
        );
        for (y0, y1) in [(0.34, 0.98), (1.12, 2.8)] {
            add(
                bx(x(STILE_OUT, STILE_OUT + 0.015), y0, y1, z0 + 0.09..z1 - 0.09),
                kind::WOOD,
                CASE_FIELD,
            );
        }
    }
    // Plinth (the bottom shelf) over a dark toe-kick; cornice in two steps over the
    // frieze, its crown as wide as the pitch so a row's crowns run on unbroken.
    add(
        bx(wide(STILE_OUT + 0.015), 0.04, SHELF_BASE, through(0.04, 0.008)),
        kind::WOOD,
        [0.11, 0.055, 0.025],
    );
    add(
        bx(wide(pitch), UNIT_H + 0.07, UNIT_H + 0.13, through(0.085, 0.006)),
        kind::WOOD,
        CASE_TRIM,
    );
    add(
        bx(wide(UNIT_W * 0.5 + 0.04), UNIT_H, UNIT_H + 0.07, through(0.055, 0.002)),
        kind::WOOD,
        WALNUT,
    );
    for &s in faces {
        for i in 1..ROWS {
            let y = SHELF_BASE + i as f32 * SHELF_STEP;
            add(
                bx(wide(STILE_IN), y - BOARD_T, y, before(s, -FACE_DEPTH, 0.0)),
                kind::WOOD,
                WALNUT,
            );
        }
    }
    add(
        bx(wide(UNIT_W * 0.5 - 0.02), 0.0, 0.04, through(-0.02, 0.012)),
        kind::WOOD,
        [0.035, 0.018, 0.009],
    );
    let back = if both {
        let m = (z0 + z1) * 0.5;
        m - 0.02..m + 0.02
    } else {
        z0..z0 + WALL_BACK
    };
    add(bx(wide(STILE_IN), SHELF_BASE, UNIT_H, back), kind::WOOD, DARK_WOOD);
    out
}
