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
/// Its middle, from the face's.
pub(super) const STILE_MID: f32 = (STILE_IN + STILE_OUT) * 0.5;
const STILE_PROUD: f32 = 0.025;
/// The frieze across the top of a face, clear of the tallest book on the top shelf.
const FRIEZE_Y: f32 = 2.98;
const CASE_FIELD: [f32; 3] = [0.25, 0.12, 0.05];
const CASE_TRIM: [f32; 3] = [0.12, 0.06, 0.028];
/// A range-end plaque's board (across, up, thick), on the upper field of a case's
/// end, and how high its middle is: at eye height.
const END_BOARD: Vec3 = Vec3::new(0.66, 0.5, 0.03);
const END_Y: f32 = 1.55;
/// A category's name on a range-end plaque, and its filing keys under it.
const END_NAME: Style = Style {
    em: 0.065,
    ..SIGN_LETTERS
};
const END_KEYS: Style = Style {
    em: 0.045,
    ..SIGN_LETTERS
};
const LEATHER: [f32; 3] = [0.05, 0.1, 0.06];
/// A shelf's label holder: brass, glowing a little (color.w is its brightness).
const LABEL_GLOW: [f32; 4] = [1.0, 0.7, 0.3, 0.3];
/// A face's ladder rail: the middle of its rod, up from the floor and out of the
/// face's plane (clear of the stiles, under their capitals).
const RAIL_Y: f32 = 2.895;
const RAIL_OUT: f32 = STILE_PROUD + 0.027;
/// About one face in this many has a ladder on its rail.
const LADDER_ONE_IN: u32 = 12;
/// Half a ladder's width, and how far out of the face its foot stands.
const LADDER_HALF: f32 = 0.23;
const LADDER_FOOT: f32 = 0.5;
const LADDER_WOOD: [f32; 3] = [0.22, 0.12, 0.055];

impl Library {
    /// A turned piece: (radius, height) points up its outside (`Mesh::lathe`).
    pub(super) fn turned(&mut self, profile: &[(f32, f32)], segs: usize) -> Shape {
        self.shape(Mesh::lathe(profile, segs))
    }

    /// An open shell turned from `profile` (a shade, a bowl), seen from both sides. (Not
    /// for clear glass, which is drawn from both sides as it is.)
    pub(super) fn turned_open(&mut self, profile: &[(f32, f32)], segs: usize) -> Shape {
        self.shape(Mesh::lathe(profile, segs).two_sided())
    }

    /// A round rod from `a` to `b`, `d` across (rounder the thicker it is).
    pub(super) fn pole_inst(&mut self, a: Vec3, b: Vec3, d: f32, kind: u32, color: [f32; 4]) -> Inst {
        let segs = if d < 0.03 {
            8
        } else if d < 0.1 {
            12
        } else {
            24
        };
        self.shape(Mesh::cylinder(segs)).between(a, b, d, kind, color)
    }

    /// A round rod from `a` to `b`, `d` across, drawn only.
    pub(super) fn pole(&mut self, a: Vec3, b: Vec3, d: f32, kind: u32, color: [f32; 4]) {
        let i = self.pole_inst(a, b, d, kind, color);
        self.decor(i);
    }

    /// A brass wall sconce with a candle flame, lighting the room in front of it: a
    /// backplate on the wall 0.15 behind `p`, and a scrolled arm out and up to a drip
    /// pan and the cup a candle stands in.
    pub(super) fn sconce(&mut self, p: Vec3, n: Vec3) {
        // Laid out across (x), up (y) and out from the wall (z) about `p`, all one
        // piece of brass.
        let mut brass = Mesh::default();
        let plate: Vec<Vec2> = (0..16)
            .map(|k| {
                let a = k as f32 / 16.0 * TAU;
                // Taller than wide, drawn to a point at its top and bottom.
                let (s, c) = (libm::sinf(a), libm::cosf(a));
                Vec2::new(0.05 * c * (1.0 - 0.35 * s * s), -0.035 + 0.165 * s)
            })
            .collect();
        brass.add(
            &Mesh::prism(&plate, 0.012),
            Vec3::new(0.0, 0.0, -0.144),
            Quat::IDENTITY,
            Vec3::ONE,
        );
        let boss = Mesh::lathe(
            &[(0.0, 0.0), (0.03, 0.0), (0.024, 0.012), (0.012, 0.02), (0.0, 0.022)],
            10,
        );
        brass.add(
            &boss,
            Vec3::new(0.0, -0.04, -0.138),
            Quat::from_rotation_x(TAU / 4.0),
            Vec3::ONE,
        );
        // The arm leaves the boss downward and outward and curls up under the stem.
        let arm = bezier(
            [
                Vec3::new(0.0, -0.04, -0.13),
                Vec3::new(0.0, -0.12, -0.1),
                Vec3::new(0.0, -0.15, -0.01),
                Vec3::new(0.0, -0.105, 0.0),
            ],
            10,
        );
        brass.add(
            &Mesh::sweep(&arm, &[0.02, 0.014], &circle(0.5, 6), false),
            Vec3::ZERO,
            Quat::IDENTITY,
            Vec3::ONE,
        );
        let mut stem = vec![(0.0, -0.135), (0.006, -0.13)];
        stem.extend(ball(-0.11, 0.015, (0.008, 0.008), 4));
        stem.extend([
            (0.007, -0.03),
            (0.014, -0.022),
            (0.014, -0.012),
            (0.008, -0.004),
            (0.012, 0.012),
            (0.055, 0.02),
            (0.058, 0.028),
            (0.05, 0.032),
            (0.022, 0.034),
            (0.021, 0.06),
            (0.026, 0.068),
            (0.023, 0.072),
            (0.0, 0.072),
        ]);
        brass.add(&Mesh::lathe(&stem, 10), Vec3::ZERO, Quat::IDENTITY, Vec3::ONE);
        let rot = facing(n);
        let brass = self.shape(brass);
        self.decor(brass.at(p, rot, kind::METAL, rgba(BRASS, 1.0)));
        self.candle(p + Vec3::Y * 0.06, 0.14, 0.036);
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
        // (centre above `c`, size, kind, colour), mirrored across x by `s` where it has
        // one: the leather a hair proud of the top, the top, a moulding under its edge,
        // the apron, and an H stretcher.
        for s in [-1.0f32, 1.0] {
            for (at, size, k, rgb) in [
                (
                    Vec3::new(0.0, 0.635, s * 0.45),
                    Vec3::new(2.0, 0.11, 0.03),
                    kind::WOOD,
                    WALNUT,
                ),
                (
                    Vec3::new(s * 1.05, 0.635, 0.0),
                    Vec3::new(0.03, 0.11, 0.8),
                    kind::WOOD,
                    WALNUT,
                ),
                (
                    Vec3::new(s * 1.05, 0.12, 0.0),
                    Vec3::new(0.035, 0.04, 0.9),
                    kind::WOOD,
                    DARK_WOOD,
                ),
            ] {
                self.decor(Inst::aabox(c + at, size, k, rgb));
            }
        }
        for (y, size, k, rgb) in [
            (0.7775, Vec3::new(2.0, 0.003, 0.84), kind::CLOTH, LEATHER),
            (0.748, Vec3::new(2.4, 0.056, 1.2), kind::WOOD, WALNUT),
            (0.705, Vec3::new(2.34, 0.03, 1.14), kind::WOOD, CASE_TRIM),
            (0.1275, Vec3::new(2.1, 0.035, 0.035), kind::WOOD, DARK_WOOD),
        ] {
            self.decor(Inst::aabox(c + Vec3::Y * y, size, k, rgb));
        }
        for (sx, sz) in [(-1.0, -1.0), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)] {
            self.turned_leg(c + Vec3::new(sx * 1.05, 0.0, sz * 0.45), 0.69, 0.1, DARK_WOOD);
        }
        self.collide(
            c - Vec3::new(1.25, 0.0, 0.65),
            c + Vec3::new(1.25, TABLE_H, 0.65),
            Stops::ALL,
        );
        let chair = [0.2, 0.09, 0.04];
        // The chairs' front legs, turned: a foot, a ring where the side stretchers meet
        // them, a taper, and a square block under the seat.
        let mut front = Mesh::lathe(
            &[
                (0.0, 0.0),
                (0.017, 0.0),
                (0.02, 0.015),
                (0.015, 0.035),
                (0.014, 0.135),
                (0.02, 0.14),
                (0.02, 0.17),
                (0.014, 0.175),
                (0.018, 0.27),
                (0.016, 0.31),
                (0.021, 0.325),
                (0.018, 0.335),
            ],
            12,
        );
        front.add(
            &Mesh::cube(),
            Vec3::Y * 0.38,
            Quat::IDENTITY,
            Vec3::new(0.045, 0.09, 0.045),
        );
        let front = self.shape(front);
        for sz in [-1.0f32, 1.0] {
            for sx in [-0.6f32, 0.6] {
                let ch = c + Vec3::new(sx, 0.0, sz * 1.0);
                for x in [-0.2, 0.2] {
                    let leg = front.at(
                        ch + Vec3::new(x, 0.0, sz * -0.2),
                        Quat::IDENTITY,
                        kind::WOOD,
                        rgba(DARK_WOOD, 1.0),
                    );
                    self.decor(leg);
                }
                // Laid out with its back toward +z, away from the table: a cushion on
                // the seat, a crest rail and a splat between back posts that are the back
                // legs, and stretchers along the sides.
                for (x, y, z, size, k, rgb) in [
                    (
                        0.0,
                        0.4875,
                        -0.02,
                        Vec3::new(0.4, 0.035, 0.36),
                        kind::CLOTH,
                        [0.24, 0.05, 0.04],
                    ),
                    (0.0, 0.4475, 0.0, Vec3::new(0.46, 0.045, 0.44), kind::WOOD, chair),
                    (0.0, 0.95, 0.2, Vec3::new(0.5, 0.1, 0.04), kind::WOOD, chair),
                    (0.0, 0.685, 0.2, Vec3::new(0.14, 0.43, 0.02), kind::WOOD, chair),
                    (-0.2, 0.46, 0.2, Vec3::new(0.045, 0.92, 0.045), kind::WOOD, DARK_WOOD),
                    (0.2, 0.46, 0.2, Vec3::new(0.045, 0.92, 0.045), kind::WOOD, DARK_WOOD),
                    (-0.2, 0.155, 0.0, Vec3::new(0.025, 0.03, 0.36), kind::WOOD, DARK_WOOD),
                    (0.2, 0.155, 0.0, Vec3::new(0.025, 0.03, 0.36), kind::WOOD, DARK_WOOD),
                ] {
                    self.decor(Inst::aabox(ch + Vec3::new(x, y, sz * z), size, k, rgb));
                }
                self.collide(
                    ch - Vec3::new(0.26, 0.0, 0.26),
                    ch + Vec3::new(0.26, 1.05, 0.26),
                    Stops::RAIL,
                );
            }
        }
        // A turned brass candlestick (a domed foot, a knopped stem, a drip pan and a
        // socket) with its candle in a glass chimney standing in the pan.
        let candle = c + Vec3::new(0.7, TABLE_H, 0.0);
        let mut stick = vec![
            (0.0, 0.0),
            (0.06, 0.0),
            (0.06, 0.007),
            (0.05, 0.016),
            (0.024, 0.032),
            (0.012, 0.046),
        ];
        stick.extend(ball(0.068, 0.02, (0.01, 0.01), 6));
        stick.extend([
            (0.009, 0.11),
            (0.012, 0.118),
            (0.046, 0.124),
            (0.048, 0.131),
            (0.042, 0.134),
            (0.02, 0.136),
            (0.019, 0.16),
            (0.023, 0.166),
            (0.0, 0.166),
        ]);
        let stick = self.shape(Mesh::lathe(&stick, 16));
        self.decor(stick.at(candle, Quat::IDENTITY, kind::METAL, rgba(BRASS, 1.0)));
        self.candle(candle + Vec3::Y * 0.14, 0.13, 0.034);
        let chimney = Mesh::lathe(
            &[
                (0.036, 0.0),
                (0.042, 0.025),
                (0.055, 0.08),
                (0.054, 0.13),
                (0.041, 0.2),
                (0.038, 0.24),
                (0.043, 0.255),
            ],
            24,
        );
        let chimney = self.shape(chimney);
        let glass = chimney.at(
            candle + Vec3::Y * 0.134,
            Quat::IDENTITY,
            kind::CLEAR,
            [0.9, 0.95, 1.0, 0.1],
        );
        self.translucent(glass);
        self.light(candle + Vec3::Y * 0.35, 4.5, [1.0, 0.6, 0.28], 0.7, LightKind::Flicker);
    }

    /// A lit candle standing on `foot`, `h` tall and `d` across: wax glowing a little,
    /// its top cupped round the wick, and its flame over it.
    fn candle(&mut self, foot: Vec3, h: f32, d: f32) {
        let r = d * 0.5;
        let wax = Mesh::lathe(
            &[(0.0, 0.0), (r, 0.0), (r, h - 0.008), (r * 0.8, h), (0.0, h - 0.004)],
            12,
        );
        let wax = self.shape(wax);
        self.decor(wax.at(foot, Quat::IDENTITY, kind::PAPER, [1.0, 0.72, 0.42, 0.18]));
        let flame = Inst::new(
            foot + Vec3::Y * (h + d * 0.95),
            Quat::IDENTITY,
            Vec3::new(d * 0.75, d * 2.0, d * 0.75),
            kind::FIRE,
            [1.0, 0.55, 0.18, 10.0],
            0,
        );
        self.translucent(flame);
    }

    /// A two-tier brass chandelier at `ch` hung from `top_y`: a hoop of candles, a
    /// smaller hoop above it, scrolled arms from a turned stem, and crystal drops that
    /// glow faintly under the hoop, stirring.
    pub(super) fn chandelier(&mut self, ch: Vec3, top_y: f32) {
        let brass = rgba(BRASS, 1.0);
        let iron = rgba([0.1, 0.08, 0.05], 1.0);
        let cup = Mesh::lathe(
            &[
                (0.0, 0.0),
                (0.012, 0.0),
                (0.01, 0.01),
                (0.04, 0.018),
                (0.042, 0.024),
                (0.036, 0.027),
                (0.02, 0.029),
                (0.019, 0.05),
                (0.024, 0.058),
                (0.0, 0.058),
            ],
            12,
        );
        let cup = self.shape(cup);
        // A cut drop, widest low down: eight facets round.
        let drop = Mesh::lathe(
            &[
                (0.0, 0.0),
                (0.014, 0.006),
                (0.02, 0.02),
                (0.018, 0.04),
                (0.01, 0.065),
                (0.003, 0.08),
                (0.0, 0.082),
            ],
            8,
        );
        let drop = self.shape(drop);
        let wire = self.shape(Mesh::cylinder(6)).sized(Vec3::new(0.004, 0.032, 0.004));
        // (radius, height above `ch`, candles).
        for (r, y, n) in [(1.3f32, 0.0f32, 12usize), (0.7, 0.55, 8)] {
            // The hoop: a band beaded along its top and bottom edges.
            let (t, w) = (0.025, 0.016);
            let section = [
                (r - w, -t),
                (r + w, -t),
                (r + w + 0.005, -t + 0.008),
                (r + w, -t + 0.016),
                (r + w, t - 0.016),
                (r + w + 0.005, t - 0.008),
                (r + w, t),
                (r - w, t),
                (r - w, -t),
            ];
            let hoop = self.shape(Mesh::lathe(&section, if r > 1.0 { 48 } else { 32 }));
            self.decor(hoop.at(ch + Vec3::Y * y, Quat::IDENTITY, kind::METAL, brass));
            // Arms from the stem, dipping and curling up under every other candle.
            let path = bezier(
                [
                    Vec3::new(0.03, -0.27, 0.0),
                    Vec3::new(r * 0.45, -0.42, 0.0),
                    Vec3::new(r * 0.85, -0.3, 0.0),
                    Vec3::new(r, -t, 0.0),
                ],
                14,
            );
            let arm = Mesh::sweep(&path, &[0.026, 0.02], &circle(0.5, 8), false);
            let mut arms = Mesh::default();
            for k in (0..n).step_by(2) {
                let a = (k as f32 + 0.5) / n as f32 * TAU;
                arms.add(&arm, Vec3::Y * y, Quat::from_rotation_y(-a), Vec3::ONE);
            }
            let arms = self.shape(arms);
            self.decor(arms.at(ch, Quat::IDENTITY, kind::METAL, brass));
            for k in 0..n {
                let a = (k as f32 + 0.5) / n as f32 * TAU;
                let foot = ch + Vec3::new(libm::cosf(a), 0.0, libm::sinf(a)) * r + Vec3::Y * (y + t);
                self.decor(cup.at(foot, Quat::IDENTITY, kind::METAL, brass));
                self.candle(foot + Vec3::Y * 0.03, 0.13, 0.034);
            }
            if r > 1.0 {
                // A drop under the hoop between each candle and the next, on a wire,
                // swinging a little in its own time.
                for k in 0..n {
                    let b = (k as f32 + 1.0) / n as f32 * TAU;
                    let hook = ch + Vec3::new(libm::cosf(b), 0.0, libm::sinf(b)) * r - Vec3::Y * t;
                    let stir = Motion::Swing {
                        axis: Quat::from_rotation_y(k as f32 * 1.9) * Vec3::X,
                        amp: 0.08,
                        period: 3.6 + (k % 5) as f32 * 0.45,
                    };
                    let m = self.mover(hook, stir, k as f32 * 1.3, None);
                    self.moving(m, wire.inst(hook - Vec3::Y * 0.016, Quat::IDENTITY, kind::METAL, brass));
                    let at = hook - Vec3::Y * 0.03;
                    let turn = Quat::from_rotation_y(b);
                    self.moving(
                        m,
                        drop.at(at - Vec3::Y * 0.082, turn, kind::CRYSTAL, [0.65, 0.78, 1.0, 0.7]),
                    );
                }
            }
        }
        // The stem, turned with knops along it and a crown at its top.
        let mut stem = vec![
            (0.0, -0.47),
            (0.012, -0.468),
            (0.022, -0.458),
            (0.03, -0.44),
            (0.03, -0.33),
        ];
        stem.extend(ball(-0.22, 0.09, (0.03, 0.03), 10));
        stem.push((0.03, 0.18));
        stem.extend(ball(0.24, 0.065, (0.03, 0.03), 8));
        stem.push((0.03, 0.62));
        stem.extend(ball(0.66, 0.045, (0.03, 0.03), 6));
        stem.extend([
            (0.03, 1.3),
            (0.05, 1.33),
            (0.12, 1.37),
            (0.125, 1.4),
            (0.11, 1.42),
            (0.04, 1.43),
            (0.03, 1.46),
            (0.025, 1.5),
            (0.0, 1.5),
        ]);
        let stem = self.shape(Mesh::lathe(&stem, 24));
        self.decor(stem.at(ch, Quat::IDENTITY, kind::METAL, brass));
        // The big drop under the stem.
        let hook = ch - Vec3::Y * 0.47;
        let stir = Motion::Swing {
            axis: Vec3::X,
            amp: 0.04,
            period: 6.5,
        };
        let m = self.mover(hook, stir, 0.0, None);
        self.moving(m, wire.inst(hook - Vec3::Y * 0.016, Quat::IDENTITY, kind::METAL, brass));
        let big = drop.sized(Vec3::new(0.08, 0.2, 0.08));
        self.moving(
            m,
            big.at(
                hook - Vec3::Y * 0.23,
                Quat::IDENTITY,
                kind::CRYSTAL,
                [0.65, 0.78, 1.0, 1.3],
            ),
        );
        // Chains from the hoop to the crown, and one from the crown to the ceiling.
        let mut chains = Mesh::default();
        for k in 0..3 {
            let a = k as f32 / 3.0 * TAU;
            let bottom = Vec3::new(libm::cosf(a), 0.0, libm::sinf(a)) * 1.27;
            let top = Vec3::new(libm::cosf(a), 0.0, libm::sinf(a)) * 0.1 + Vec3::Y * 1.4;
            chains.add(&Mesh::rod(bottom, top, 0.016, 6), Vec3::ZERO, Quat::IDENTITY, Vec3::ONE);
        }
        chains.add(
            &Mesh::rod(Vec3::Y * 1.48, Vec3::Y * (top_y - ch.y), 0.02, 6),
            Vec3::ZERO,
            Quat::IDENTITY,
            Vec3::ONE,
        );
        let chains = self.shape(chains);
        self.decor(chains.at(ch, Quat::IDENTITY, kind::METAL, iron));
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
        let faces = [(c - n * hd, -n), (c + n * hd, n)];
        for (at, normal) in faces {
            self.case_face(at, normal);
        }
        faces
    }

    /// A single-sided bookcase with its back on a wall: `at` is the middle of its back
    /// at floor level, `normal` (along x or z) the way it faces. Returns its face, as
    /// `bookcase` does.
    pub(super) fn wall_bookcase(&mut self, at: Vec3, normal: Vec3) -> (Vec3, Vec3) {
        // Laid out across (x), up (y) and out from the wall (z), then turned to face
        // `normal`.
        let right = Vec3::Y.cross(normal);
        let place = |off: Vec3| at + right * off.x + Vec3::Y * off.y + normal * off.z;
        let hw = UNIT_W * 0.5;
        let d = WALL_BACK + FACE_DEPTH;
        let mut boxes = case_boxes(0.0, d, false);
        if self.back_seen(at, normal) {
            boxes.extend(finished_back());
        }
        for (mn, mx, k, color) in boxes {
            let (p, q) = (place(mn), place(mx));
            self.decor(Inst::new((p + q) * 0.5, Quat::IDENTITY, (q - p).abs(), k, color, 0));
        }
        self.case_face(at + normal * d, normal);
        let (a, b) = (
            place(Vec3::new(-hw - 0.04, 0.0, 0.0)),
            place(Vec3::new(hw + 0.04, UNIT_H + 0.13, d + 0.04)),
        );
        self.collide(a.min(b), a.max(b), Stops::ALL);
        (at + normal * d, normal)
    }

    /// A plaque on the end of a double-sided case, at eye height, telling what is
    /// shelved down either side of the range it ends, as a library's range guides do:
    /// `c` is the case's centre at floor level and `out` the way along it to that end;
    /// `sides`, the faces down each side (by index: one category's, in reading order).
    /// For each, its category's name, its filing keys there, and an arrow round to it:
    /// the side on the left above, the side on the right below.
    pub(super) fn range_end(&mut self, c: Vec3, out: Vec3, mut sides: [Range<usize>; 2]) {
        let right = Vec3::Y.cross(out);
        sides.sort_by_key(|s| self.faces[s.start].normal.dot(right) > 0.0);
        let rot = Quat::from_mat3(&Mat3::from_cols(right, Vec3::Y, out));
        let at = c + out * (STILE_OUT + 0.015 + END_BOARD.z * 0.5) + Vec3::Y * END_Y;
        let ccol = catalog::category_color(&self.catalog.categories[self.faces[sides[0].start].category]);
        self.decor(Inst::new(at, rot, END_BOARD, kind::PLAQUE, rgba(ccol, 0.08), 0));
        let front = at + out * (END_BOARD.z * 0.5);
        // Brass standing a hair proud of the board: a rule between the sides, and the
        // arrows (made pointing left).
        let brass = rgba(BRASS, 1.0);
        let proud = front + out * 0.003;
        self.decor(Inst::new(
            proud,
            rot,
            Vec3::new(0.5, 0.006, 0.006),
            kind::METAL,
            brass,
            0,
        ));
        let tip = [Vec2::new(-0.03, 0.0), Vec2::new(0.025, -0.04), Vec2::new(0.025, 0.04)];
        let arrow = self.shape(Mesh::prism(&tip, 0.006));
        for (side, e, y) in [(&sides[0], -1.0f32, 0.105), (&sides[1], 1.0, -0.115)] {
            let turn = rot * Quat::from_rotation_z(if e < 0.0 { 0.0 } else { std::f32::consts::PI });
            self.decor(arrow.inst(proud + right * (e * 0.235) + Vec3::Y * y, turn, kind::METAL, brass));
            let name = self.catalog.categories[self.faces[side.start].category].name.clone();
            let mid = front - right * (e * 0.045) + Vec3::Y * y;
            self.inscribe(&name, END_NAME, mid + Vec3::Y * 0.045, out, 0.45);
            if let Some(keys) = self.faces_keys(side.clone()) {
                self.inscribe(&keys, END_KEYS, mid - Vec3::Y * 0.045, out, 0.45);
            }
        }
    }

    /// Whether a wall case's back (`at`, facing `normal`) stands free, nothing that
    /// stops sight just behind it (laid down before it: a wall, a core of cases).
    fn back_seen(&self, at: Vec3, normal: Vec3) -> bool {
        [0.06f32, 0.25].iter().all(|&d| {
            let p = at - normal * d + Vec3::Y * 1.5;
            !(self.world.solids().iter())
                .any(|s| s.stops.has(Stops::SIGHT) && p.cmpgt(s.min).all() && p.cmplt(s.max).all())
        })
    }

    /// A case face's ladder rail: a brass rod the width of the case's pitch, standing
    /// off the stiles on a bracket each. Now and then a library ladder hangs on it,
    /// leaning on the case, and slides along it from time to time. `at` is the face's
    /// plane at floor level, `n` its normal.
    fn case_face(&mut self, at: Vec3, n: Vec3) {
        let rot = facing(n);
        // Every face has one, so few triangles: its rod eight round, each bracket (a
        // rose on the stile and a stem out to the rod) six.
        let mut rail = Mesh::rod(
            Vec3::new(-UNIT_PITCH * 0.5, RAIL_Y, RAIL_OUT),
            Vec3::new(UNIT_PITCH * 0.5, RAIL_Y, RAIL_OUT),
            0.022,
            8,
        );
        let bracket = Mesh::lathe(
            &[
                (0.0, 0.0),
                (0.017, 0.0),
                (0.017, 0.004),
                (0.006, 0.01),
                (0.006, RAIL_OUT - STILE_PROUD),
            ],
            6,
        );
        for e in [-1.0f32, 1.0] {
            let stile = Vec3::new(e * STILE_MID, RAIL_Y, STILE_PROUD);
            rail.add(&bracket, stile, Quat::from_rotation_x(TAU / 4.0), Vec3::ONE);
        }
        let rail = self.shape(rail);
        self.decor(rail.at(at, rot, kind::METAL, rgba(BRASS, 1.0)));
        if !spot_hash(at).is_multiple_of(LADDER_ONE_IN) || !self.room_for_ladder(at) {
            return;
        }
        // Where it rests along the face and where it slides to, never past the stiles;
        // slowly, resting long at each end. Each choice its own hash of the spot.
        let unit = |k: f32| (spot_hash(at + Vec3::Y * k) & 1023) as f32 / 1023.0;
        let reach = STILE_IN - LADDER_HALF;
        let from = (unit(1.0) * 2.0 - 1.0) * reach;
        let span = 0.7 + 0.6 * unit(2.0);
        let to = if from > 0.0 { from - span } else { from + span }.clamp(-reach, reach);
        let period = 110.0 + 60.0 * unit(3.0);
        let (wood, brass) = ladder();
        let (wood, brass) = (self.shape(wood), self.shape(brass));
        let foot = at + rot * Vec3::X * from;
        let slide = Motion::Slide {
            travel: rot * Vec3::X * (to - from),
            period,
        };
        let m = self.mover(foot, slide, period * unit(4.0), None);
        self.moving(m, wood.at(foot, rot, kind::WOOD, rgba(LADDER_WOOD, 1.0)));
        self.moving(m, brass.at(foot, rot, kind::METAL, rgba(BRASS, 1.0)));
    }

    /// The label holder on shelf `row` of face `f`, as its case has it (`case_boxes`) but
    /// a hair larger each way, so that drawn over the case's own, in another glow, it
    /// hides it.
    pub fn label_cover(&self, f: usize, row: usize) -> Inst {
        let face = &self.faces[f];
        let ((y0, y1), (o0, o1)) = label_holder(row);
        let at = face.center + Vec3::Y * ((y0 + y1) * 0.5) + face.normal * ((o0 + o1) * 0.5);
        let size = Vec3::new(LABEL_W, y1 - y0, o1 - o0) + Vec3::splat(0.0006);
        Inst::new(at, face.rot(), size, kind::EMISSIVE, LABEL_GLOW, 0)
    }

    /// Whether a ladder may hang on a face whose plane is at `at` (floor level): well
    /// away from the doorways, where couriers set books down, and the stairs, and no
    /// more than two to a room.
    fn room_for_ladder(&self, at: Vec3) -> bool {
        let flat = |p: Vec3| Vec2::new(p.x, p.z);
        let here = flat(at);
        let level = |p: Vec3, r: &Room| (p.y - r.door.y).abs() < 1.0;
        let inside = |p: Vec3, r: &Room| level(p, r) && flat(p).cmpge(r.min).all() && flat(p).cmple(r.max).all();
        let rooms_clear = (self.rooms.iter())
            .filter(|r| level(at, r))
            .all(|r| here.distance(flat(r.door)) > 5.5 && here.distance(flat(r.drop)) > 3.5);
        let stairs_clear = (self.world.stairs().iter()).all(|s| {
            let nearest = here.clamp(s.min, s.max);
            here.distance(nearest) > 3.0
        });
        let few = (self.rooms.iter().find(|r| inside(at, r))).is_none_or(|r| {
            let ladders = self
                .movers
                .iter()
                .filter(|m| matches!(m.motion, Motion::Slide { .. }) && inside(m.pivot, r));
            ladders.count() < 2
        });
        rooms_clear && stairs_clear && few
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
            let turned = self.baluster_shape();
            for k in (0..f.steps).step_by(2) {
                let a = -r + (k as f32 + 0.5) * f.run();
                let foot = f.step_top(k);
                self.baluster(turned, at(a, u_rail, foot), slope(a) + 0.92 - foot);
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

    /// A clock's hands over the face of a dial (`face` its middle, `n` out of it, `r`
    /// its radius) set at `hour`:`minute` and keeping time from there, clockwise as
    /// seen; spades, the hour hand short and broad, the minute hand long; with a second
    /// hand ticking round if `seconds`; and a cap over their arbor.
    pub(super) fn clock_hands(
        &mut self,
        (face, n, r): (Vec3, Vec3, f32),
        (hour, minute): (f32, f32),
        (ink, cap): ([f32; 4], [f32; 4]),
        seconds: bool,
    ) {
        let hand = |lib: &mut Library, turns: f32, size: (f32, f32), z: f32, motion: Motion| {
            let part = lib.clock_hand((face, n), turns, size, z, ink);
            let m = lib.mover(face, motion, 0.0, None);
            lib.moving(m, part);
        };
        let spin = |hours: f32| Motion::Spin {
            axis: n,
            rate: -TAU / (hours * 3600.0),
        };
        let [hour_hand, minute_hand] = Self::CLOCK_HANDS;
        hand(
            self,
            (hour % 12.0) / 12.0 + minute / 720.0,
            (r * hour_hand.0, r * hour_hand.1),
            hour_hand.2,
            spin(12.0),
        );
        hand(
            self,
            minute / 60.0,
            (r * minute_hand.0, r * minute_hand.1),
            minute_hand.2,
            spin(1.0),
        );
        if seconds {
            let tick = Motion::Tick {
                axis: -n,
                steps: 60,
                period: 60.0,
            };
            hand(self, 0.0, (r * 0.86, r * 0.012), 0.042, tick);
        }
        self.clock_cap((face, n, r), cap);
    }

    /// The hour and minute hands: length and width (by the dial's radius), and how far
    /// each stands off the face.
    pub(super) const CLOCK_HANDS: [(f32, f32, f32); 2] = [(0.52, 0.07, 0.02), (0.78, 0.045, 0.031)];

    /// A clock hand over a dial (`face` its middle, `n` out of it) pointing `turns` of
    /// the way round from twelve, clockwise as seen: a spade `len` long and `w` wide,
    /// standing `z` off the face.
    pub(super) fn clock_hand(
        &mut self,
        (face, n): (Vec3, Vec3),
        turns: f32,
        (len, w): (f32, f32),
        z: f32,
        ink: [f32; 4],
    ) -> Inst {
        let side = Vec3::Y.cross(n);
        let facing = Quat::from_rotation_arc(Vec3::Z, n);
        // Where `side` is in the plane `facing` lays a hand's length (x) in.
        let x = facing * Vec3::X;
        let side_at = libm::atan2f(side.dot(n.cross(x)), side.dot(x));
        let v = Vec2::new;
        let spade = [
            v(-0.16 * len, -w * 0.5),
            v(0.62 * len, -w * 0.35),
            v(0.72 * len, -w * 1.3),
            v(0.86 * len, -w * 0.4),
            v(len, 0.0),
            v(0.86 * len, w * 0.4),
            v(0.72 * len, w * 1.3),
            v(0.62 * len, w * 0.35),
            v(-0.16 * len, w * 0.5),
        ];
        // Up at twelve, a turn clockwise as seen.
        let th = std::f32::consts::FRAC_PI_2 - turns * TAU;
        let shape = self.shape(Mesh::prism(&spade, 0.01));
        shape.at(
            face + n * z,
            facing * Quat::from_rotation_z(th + side_at),
            kind::METAL,
            ink,
        )
    }

    /// The cap over a clock's arbor, on a dial of radius `r`.
    pub(super) fn clock_cap(&mut self, (face, n, r): (Vec3, Vec3, f32), cap: [f32; 4]) {
        let dome = self.shape(Mesh::dome(16));
        let k = r.max(0.3);
        let up = Quat::from_rotation_arc(Vec3::Y, n);
        self.decor(dome.sized(Vec3::new(k * 0.1, k * 0.05, k * 0.1)).inst(
            face + n * (0.048 + k * 0.025),
            up,
            kind::METAL,
            cap,
        ));
    }

    /// A wooden box along the line `a`→`b` (not a vertical one), `w` across and `h`
    /// tall, kept upright.
    pub(super) fn beam(&mut self, a: Vec3, b: Vec3, w: f32, h: f32, rgb: [f32; 3]) {
        let d = b - a;
        let len = d.length();
        let rot = Quat::from_mat3(&upright_frame(d / len));
        self.decor(Inst::new(
            (a + b) * 0.5,
            rot,
            Vec3::new(len, h, w),
            kind::WOOD,
            rgba(rgb, 1.0),
            0,
        ));
    }

    /// A turned leg standing on `foot`, `h` tall and `w` square at its top: a square
    /// block where the apron meets it, and below it a ring, a vase, a collar where the
    /// stretchers meet it, and a bun foot.
    fn turned_leg(&mut self, foot: Vec3, h: f32, w: f32, rgb: [f32; 3]) {
        let top = h - 0.13;
        let r = w * 0.5;
        let mut leg = vec![(0.0, 0.0), (r * 0.6, 0.0)];
        leg.extend(ball(0.035, r * 0.82, (r * 0.6, r * 0.5), 5));
        leg.extend([(r * 0.5, 0.09), (r * 0.7, 0.1), (r * 0.7, 0.145), (r * 0.45, 0.16)]);
        // A vase, fullest a little below its middle.
        leg.extend((0..=8).map(|k| {
            let t = k as f32 / 8.0;
            let swell = libm::sinf(std::f32::consts::PI * libm::powf(t, 0.8));
            (r * (0.42 + 0.46 * swell), top * (0.3 + 0.56 * t))
        }));
        leg.extend([(r * 0.75, top - 0.03), (r * 0.75, top - 0.012), (r * 0.6, top)]);
        let mut m = Mesh::lathe(&leg, 16);
        m.add(
            &Mesh::cube(),
            Vec3::Y * (top + 0.065),
            Quat::IDENTITY,
            Vec3::new(w, 0.13, w),
        );
        let leg = self.shape(m);
        self.decor(leg.at(foot, Quat::IDENTITY, kind::WOOD, rgba(rgb, 1.0)));
    }

    /// A wooden post `w` square standing `h` tall on `foot`.
    fn post(&mut self, foot: Vec3, h: f32, w: f32, rgb: [f32; 3]) {
        self.decor(Inst::aabox(
            foot + Vec3::Y * (h * 0.5),
            Vec3::new(w, h, w),
            kind::WOOD,
            rgb,
        ));
    }

    /// A turned baluster (made once per run of them, sized to each), standing `h` tall
    /// on `foot`.
    fn baluster(&mut self, turned: Shape, foot: Vec3, h: f32) {
        let s = turned.sized(Vec3::new(turned.size.x, h, turned.size.z));
        self.decor(s.at(foot, Quat::IDENTITY, kind::WOOD, rgba(BALUSTER, 1.0)));
    }

    /// The baluster's shape, made 0.89 tall about its foot: a block, a vase low on it,
    /// a slim shaft and a collar near its top. Its ends are hidden in the rails.
    fn baluster_shape(&mut self) -> Shape {
        let profile = [
            (0.022, 0.0),
            (0.022, 0.06),
            (0.03, 0.2),
            (0.013, 0.4),
            (0.012, 0.66),
            (0.022, 0.7),
            (0.015, 0.74),
            (0.018, 0.89),
        ];
        self.shape(Mesh::lathe(&profile, 8))
    }

    /// A newel post at `foot`, `h` tall to its cap, unless one already stands there: a
    /// plinth, a turned shaft, a square block where the rails meet it with a collar
    /// under the cap, and a ball finial.
    fn newel(&mut self, foot: Vec3, h: f32, newels: &mut Vec<Vec3>) {
        if newels.iter().any(|n| n.distance(foot) < 0.3) {
            return;
        }
        newels.push(foot);
        // The block reaches down past the bottom of the handrail.
        let block = h - 0.2;
        self.post(foot, 0.22, 0.19, CASE_TRIM);
        let shaft = [
            (0.066, 0.22),
            (0.066, 0.26),
            (0.05, 0.28),
            (0.058, 0.3),
            (0.058, 0.33),
            (0.045, 0.36),
            (0.064, 0.52),
            (0.058, 0.62),
            (0.042, 0.76),
            (0.04, block - 0.08),
            (0.055, block - 0.05),
            (0.055, block - 0.02),
            (0.045, block),
        ];
        let shaft = self.shape(Mesh::lathe(&shaft, 14));
        self.decor(shaft.at(foot, Quat::IDENTITY, kind::WOOD, rgba(WALNUT, 1.0)));
        self.post(foot + Vec3::Y * block, h - block, 0.15, WALNUT);
        self.post(foot + Vec3::Y * (h - 0.09), 0.05, 0.18, CASE_TRIM);
        self.post(foot + Vec3::Y * h, 0.06, 0.21, CASE_TRIM);
        let mut finial = vec![(0.0, 0.0), (0.06, 0.0), (0.06, 0.012), (0.034, 0.03), (0.024, 0.045)];
        finial.extend(ball(0.105, 0.055, (0.024, 0.0), 10));
        let finial = self.shape(Mesh::lathe(&finial, 14));
        self.decor(finial.at(
            foot + Vec3::Y * (h + 0.06),
            Quat::IDENTITY,
            kind::WOOD,
            rgba(WALNUT, 1.0),
        ));
    }

    /// A railing on the floor along the axis-aligned line `a`→`b`: a bottom rail,
    /// balusters, a moulded handrail, newel posts at its ends, and a guard in the way
    /// of bodies and books (seen past).
    pub(super) fn railing(&mut self, a: Vec3, b: Vec3, newels: &mut Vec<Vec3>) {
        let up = Vec3::Y;
        self.beam(a + up * 0.05, b + up * 0.05, 0.1, 0.1, WALNUT);
        let n = (a.distance(b) / 0.28).round().max(1.0) as usize;
        let turned = self.baluster_shape();
        for i in 1..n {
            let p = a.lerp(b, i as f32 / n as f32);
            self.baluster(turned, p + up * 0.1, RAIL_H - 0.16);
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

/// Width of a shelf's label holder.
pub(super) const LABEL_W: f32 = 0.16;

/// Where shelf `row`'s label holder is on a bookcase face: from what height to what,
/// and from how far out of the face's plane to how far. The bottom shelf's is on the
/// plinth.
pub(super) fn label_holder(row: usize) -> ((f32, f32), (f32, f32)) {
    if row + 1 == ROWS {
        ((0.07, 0.13), (0.04, 0.046))
    } else {
        let top = row_floor_y(row);
        // On the board's edge, hanging a little below it.
        ((top - 0.042, top - 0.001), (0.0, 0.006))
    }
}

/// A library ladder laid out on a face (x along it, y up, z out of its plane, from
/// the middle of its plane at floor level), hooked on the rail and leaning on the
/// case: its wood (stiles and round rungs), and its brass (the hooks over the rail
/// and rollers at its feet).
fn ladder() -> (Mesh, Mesh) {
    let half = LADDER_HALF - 0.02;
    let foot = Vec3::new(0.0, 0.035, LADDER_FOOT);
    let top = Vec3::new(0.0, RAIL_Y - 0.03, RAIL_OUT + 0.034);
    let d = top - foot;
    let lean = Quat::from_rotation_arc(Vec3::Y, d.normalize());
    let mut wood = Mesh::default();
    let mut brass = Mesh::default();
    // Round the rail with room to spare: the rod's radius and the hook's, and a gap.
    let r = 0.011 + 0.006 + 0.002;
    for e in [-1.0f32, 1.0] {
        let x = Vec3::X * (e * half);
        wood.add(
            &Mesh::cube(),
            (foot + top) * 0.5 + x,
            lean,
            Vec3::new(0.035, d.length(), 0.06),
        );
        brass.add(
            &Mesh::rod(foot + x - Vec3::X * 0.02, foot + x + Vec3::X * 0.02, 0.07, 12),
            Vec3::ZERO,
            Quat::IDENTITY,
            Vec3::ONE,
        );
        let mut hook = vec![
            x + Vec3::new(0.0, RAIL_Y - 0.06, RAIL_OUT + r),
            x + Vec3::new(0.0, RAIL_Y, RAIL_OUT + r),
        ];
        hook.extend((1..=8).map(|k| {
            let a = k as f32 / 8.0 * 3.8;
            x + Vec3::new(0.0, RAIL_Y + r * libm::sinf(a), RAIL_OUT + r * libm::cosf(a))
        }));
        brass.add(
            &Mesh::sweep(&hook, &[0.012], &circle(0.5, 8), true),
            Vec3::ZERO,
            Quat::IDENTITY,
            Vec3::ONE,
        );
    }
    let rungs = 9;
    for k in 1..=rungs {
        let p = foot + d * (k as f32 / (rungs + 1) as f32);
        wood.add(
            &Mesh::rod(p - Vec3::X * half, p + Vec3::X * half, 0.03, 8),
            Vec3::ZERO,
            Quat::IDENTITY,
            Vec3::ONE,
        );
    }
    (wood, brass)
}

/// A wall case's back finished where it stands free, in its frame (as `case_boxes`,
/// its back at z 0): panelled behind as its ends are, two raised fields to a bay over
/// a plinth, under a frieze and a return of the cornice. It stands no more than 0.045
/// behind the back, inside the boards some rooms put there.
fn finished_back() -> Vec<(Vec3, Vec3, u32, [f32; 4])> {
    let mut out = vec![
        (
            Vec3::new(-STILE_OUT, 0.0, -0.04),
            Vec3::new(STILE_OUT, SHELF_BASE, 0.008),
            kind::WOOD,
            rgba([0.11, 0.055, 0.025], 1.0),
        ),
        (
            Vec3::new(-STILE_OUT, SHELF_BASE, -0.018),
            Vec3::new(STILE_OUT, UNIT_H, 0.0),
            kind::WOOD,
            rgba(WALNUT, 1.0),
        ),
        (
            Vec3::new(-STILE_OUT + 0.012, FRIEZE_Y, -0.03),
            Vec3::new(STILE_OUT - 0.012, UNIT_H, -0.018),
            kind::WOOD,
            rgba(WALNUT, 1.0),
        ),
        (
            Vec3::new(-UNIT_W * 0.5 - 0.04, UNIT_H, -0.045),
            Vec3::new(UNIT_W * 0.5 + 0.04, UNIT_H + 0.07, 0.002),
            kind::WOOD,
            rgba(CASE_TRIM, 1.0),
        ),
    ];
    for (x0, x1) in [(-STILE_OUT + 0.1, -0.07), (0.07, STILE_OUT - 0.1)] {
        for (y0, y1) in [(0.34, 0.98), (1.12, 2.8)] {
            out.push((
                Vec3::new(x0, y0, -0.03),
                Vec3::new(x1, y1, -0.018),
                kind::WOOD,
                rgba(CASE_FIELD, 1.0),
            ));
        }
    }
    out
}

/// Axes for a box along the unit direction `x` (not vertical): its y as near up as
/// `x` allows, its z level.
fn upright_frame(x: Vec3) -> Mat3 {
    let z = x.cross(Vec3::Y).normalize();
    Mat3::from_cols(x, z.cross(x), z)
}

/// The turn that faces a piece laid out facing +z toward `n` (level).
fn facing(n: Vec3) -> Quat {
    Quat::from_rotation_y(libm::atan2f(n.x, n.z))
}

/// A turned ball's outline: radius `r` about height `y` on the axis, from where it
/// meets a neck of radius `below` under it round to one of `above` over it, in `n`
/// steps (a neck of 0 is the ball's pole).
fn ball(y: f32, r: f32, (below, above): (f32, f32), n: usize) -> impl Iterator<Item = (f32, f32)> {
    let (a0, a1) = (-libm::acosf(below / r), libm::acosf(above / r));
    (0..=n).map(move |k| {
        let a = a0 + (a1 - a0) * k as f32 / n as f32;
        (r * libm::cosf(a), y + r * libm::sinf(a))
    })
}

/// Points along a cubic Bézier curve from `p[0]` to `p[3]`, `n` steps.
fn bezier(p: [Vec3; 4], n: usize) -> Vec<Vec3> {
    (0..=n)
        .map(|k| {
            let t = k as f32 / n as f32;
            let u = 1.0 - t;
            p[0] * (u * u * u) + p[1] * (3.0 * u * u * t) + p[2] * (3.0 * u * t * t) + p[3] * (t * t * t)
        })
        .collect()
}

/// A small hash of a point (to the centimetre), for choosing among pieces without
/// touching the generation's random numbers.
fn spot_hash(p: Vec3) -> u32 {
    let mut h: u32 = 0x811c_9dc5;
    for v in [p.x, p.y, p.z] {
        h ^= (libm::roundf(v * 100.0) as i32) as u32;
        h = h.wrapping_mul(0x0100_0193);
        h ^= h >> 15;
    }
    h.wrapping_mul(0x2c1b_3c6d) ^ (h >> 12)
}

/// A bookcase's boxes in its own frame (x along it from its middle, y up from the
/// floor, z through it) as (min, max, kind, colour): the carcass spans `z0..z1` with
/// shelves opening toward +z, and toward -z too if `both`; otherwise its back stands
/// on a wall at `z0`. The nearest parts come first, so they hide what is behind them
/// before it is shaded.
fn case_boxes(z0: f32, z1: f32, both: bool) -> Vec<(Vec3, Vec3, u32, [f32; 4])> {
    let mut out = Vec::with_capacity(41);
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
    // Label holders where each shelf's label hangs, brass faintly aglow: the shelves'
    // one touch of magic.
    for &s in faces {
        for row in 0..ROWS {
            let ((y0, y1), (o0, o1)) = label_holder(row);
            let (lo, hi) = bx(wide(LABEL_W * 0.5), y0, y1, before(s, o0, o1));
            out.push((lo, hi, kind::EMISSIVE, LABEL_GLOW));
        }
    }
    let mut add = |(mn, mx): (Vec3, Vec3), k: u32, rgb: [f32; 3]| out.push((mn, mx, k, rgba(rgb, 1.0)));
    for &s in faces {
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
