//! The Arcanum: department 7, first floor, west side. A dim room hung with dark
//! drapes, its walls lined with cases; in the middle a ward of cases stands square
//! round a rune circle set glowing in the floor, a gate in each side and a stone
//! pillar at each corner, crystals hanging still in the air over the circle.
//! Grimoires lie chained to lecterns in the aisles.

use super::*;
use std::f32::consts::{FRAC_PI_2, FRAC_PI_4, PI};

/// The ward's middle, its sides' distance from it (to their cases' middles), and
/// where its cases stand along a side (either side of the gate).
const WARD: f32 = 23.6;
const SIDE: f32 = 4.37;
const UNIT: f32 = 2.54;
/// Its corner pillars span this far from the middle.
const PILLAR: (f32, f32) = (3.87, 4.87);
/// The cases on the side walls and on the near wall either side of the door.
const SIDE_CASES: [f32; 5] = [17.65, 20.35, 23.05, 25.75, 28.45];
const NEAR_LEFT: [f32; 3] = [-10.6, -7.9, -5.2];
const NEAR_RIGHT: [f32; 2] = [7.0, 9.7];
const RUNE: [f32; 4] = [0.75, 0.35, 1.0, 1.6];
const CRYSTAL_RGBA: [f32; 4] = [0.5, 0.26, 0.88, 1.1];
const DRAPE: [f32; 3] = [0.1, 0.04, 0.13];
const IRON: [f32; 4] = [0.16, 0.16, 0.18, 1.0];

pub(super) fn furnish(lib: &mut Library, frame: &RoomFrame, rng: &mut StdRng, titles: &mut HashSet<String>) {
    walls(lib, frame);
    shelves(lib, frame, rng, titles);
    pillars(lib, frame);
    rune_circle(lib, frame);
    crystals(lib, frame);
    for (i, (s, o)) in [(-1.0f32, 19.9f32), (-1.0, 27.3), (1.0, 19.9), (1.0, 27.3)]
        .into_iter()
        .enumerate()
    {
        lectern(lib, frame, s, o, i);
    }
}

/// The wall cases, read round the room from the door's left as one chain (near
/// wall, left wall, far wall, right wall, near wall) cut into four categories of six;
/// and the ward, whose inner faces make two categories of four (each reading round
/// a corner of the court) and whose outer faces make two more.
fn shelves(lib: &mut Library, f: &RoomFrame, rng: &mut StdRng, titles: &mut HashSet<String>) {
    let (out, across) = (f.out(), f.across());
    let ah = RoomFrame::A_HALF;
    let near_left = NEAR_LEFT.map(|a| lib.wall_bookcase(f.at(a, RoomFrame::O_NEAR, 0.0), out));
    let left = SIDE_CASES.map(|o| lib.wall_bookcase(f.at(-ah, o, 0.0), across));
    let far: Vec<(Vec3, Vec3)> = (0..9)
        .map(|k| lib.wall_bookcase(f.at(-10.8 + 2.7 * k as f32, RoomFrame::O_FAR, 0.0), -out))
        .collect();
    let right = SIDE_CASES.map(|o| lib.wall_bookcase(f.at(ah, o, 0.0), -across));
    let near_right = NEAR_RIGHT.map(|a| lib.wall_bookcase(f.at(a, RoomFrame::O_NEAR, 0.0), out));
    let chain = [
        left_to_right(near_left.to_vec()),
        left_to_right(left.to_vec()),
        left_to_right(far),
        left_to_right(right.to_vec()),
        left_to_right(near_right.to_vec()),
    ]
    .concat();
    // The ward's sides (far, right, near, left as `a` and `o` run), each side's faces
    // looking into the court and out of it.
    let mut inner: [Vec<(Vec3, Vec3)>; 4] = Default::default();
    let mut outer: [Vec<(Vec3, Vec3)>; 4] = Default::default();
    let middle = f.at(0.0, WARD, 0.0);
    let along_a = !f.out_along_x();
    for (i, (u, v, along)) in [
        (0.0f32, SIDE, along_a),
        (SIDE, 0.0, !along_a),
        (0.0, -SIDE, along_a),
        (-SIDE, 0.0, !along_a),
    ]
    .into_iter()
    .enumerate()
    {
        for k in [-1.0f32, 1.0] {
            let (a, o) = if u == 0.0 {
                (k * UNIT, WARD + v)
            } else {
                (u, WARD + k * UNIT)
            };
            for face in lib.bookcase(f.at(a, o, 0.0), along) {
                if (middle - face.0).dot(face.1) > 0.0 {
                    inner[i].push(face);
                } else {
                    outer[i].push(face);
                }
            }
        }
    }
    let [far_in, right_in, near_in, left_in] = inner.map(left_to_right);
    let [far_out, right_out, near_out, left_out] = outer.map(left_to_right);
    let mut groups: Vec<Vec<(Vec3, Vec3)>> = chain.chunks(6).map(|c| c.to_vec()).collect();
    groups.push([far_out, left_out].concat());
    groups.push([near_out, right_out].concat());
    groups.push([far_in, right_in].concat());
    groups.push([near_in, left_in].concat());
    for (cat, faces) in f.categories.clone().zip(groups) {
        lib.add_category(rng, cat, &faces, titles);
    }
}

/// Dark beams, drapes hung above the cases all round, and one tall window in the far
/// wall between heavy curtains, its moonlight falling toward the ward.
fn walls(lib: &mut Library, f: &RoomFrame) {
    let h = RoomFrame::HEIGHT;
    let ah = RoomFrame::A_HALF;
    let (near, far) = (RoomFrame::O_NEAR, RoomFrame::O_FAR);
    for o in [16.0f32, 19.6, 23.6, 27.6, 30.8] {
        slab(
            lib,
            f,
            (-ah, ah),
            (o - 0.14, o + 0.14),
            (h - 0.3, h),
            kind::WOOD,
            [0.06, 0.03, 0.02],
        );
    }
    // Swagged drapes over the side cases, a fold to each case.
    for s in [-1.0f32, 1.0] {
        let wall = |o: f32| Spot::new(f, s * ah, o, -s * FRAC_PI_2);
        for &o in &SIDE_CASES {
            folds(lib, &wall(o), 2.7, 3.4);
        }
    }
    for k in 0..9 {
        let a = -10.8 + 2.7 * k as f32;
        if k != 4 {
            folds(lib, &Spot::new(f, a, far, 0.0), 2.7, 3.4);
        }
    }
    // The window over the middle of the far wall, its curtains half drawn.
    let at = Spot::new(f, 0.0, far, 0.0);
    let stone = [0.3, 0.28, 0.26, 1.0];
    at.block(
        lib,
        Vec3::new(0.0, 4.5, -0.02),
        Vec3::new(1.0, 1.6, 0.08),
        kind::GLASS,
        [0.32, 0.42, 0.85, 1.0],
    );
    at.block(
        lib,
        Vec3::new(0.0, 3.64, 0.06),
        Vec3::new(1.4, 0.12, 0.12),
        kind::STONE,
        stone,
    );
    for x in [-1.0f32, 1.0] {
        at.block(
            lib,
            Vec3::new(x * 0.58, 4.5, 0.05),
            Vec3::new(0.16, 1.6, 0.1),
            kind::STONE,
            stone,
        );
        for (i, dx) in [0.0f32, 0.18, 0.36].into_iter().enumerate() {
            at.block(
                lib,
                Vec3::new(x * (0.78 + dx), 4.42, 0.14 + i as f32 * 0.04),
                Vec3::new(0.22, 2.36 - i as f32 * 0.08, 0.06),
                kind::CLOTH,
                rgba(DRAPE, 1.0),
            );
        }
    }
    at.block(
        lib,
        Vec3::new(0.0, 5.45, 0.23),
        Vec3::new(3.0, 0.3, 0.1),
        kind::CLOTH,
        rgba(DRAPE, 1.0),
    );
    let c = f.at(0.0, far - 0.1, 4.5);
    let dir = (-f.out() * 1.0 - Vec3::Y * 0.55).normalize();
    let len = 9.0;
    lib.static_trans.push(Inst::new(
        c + dir * len * 0.5,
        Quat::from_rotation_arc(Vec3::Y, dir),
        Vec3::new(0.9, len, 0.9),
        kind::SHAFT,
        [0.35, 0.45, 0.9, 0.03],
        0,
    ));
    lib.light(c - f.out() * 1.5, 7.0, [0.45, 0.55, 1.0], 0.5, LightKind::Static);
    // The near wall, bare but for panelling either side of the door.
    let d = RoomFrame::DOOR_HALF + 0.3;
    let t = 0.07;
    lib.wainscot(f.y, f.rect(NEAR_LEFT[2] + 1.4, f.door - d, near, near + t), f.side.out);
    lib.wainscot(f.y, f.rect(f.door + d, NEAR_RIGHT[0] - 1.4, near, near + t), f.side.out);
}

/// Drapes hung over a case `w` wide on a wall at `at`, from under the ceiling down to
/// `y0`, gathered in folds and swagged up at the middle.
fn folds(lib: &mut Library, at: &Spot, w: f32, y0: f32) {
    let top = RoomFrame::HEIGHT - 0.02;
    let n = 3;
    for i in 0..n {
        let x = -w * 0.5 + (i as f32 + 0.5) * w / n as f32;
        let middle = i == 1;
        let bottom = if middle { y0 + 0.5 } else { y0 };
        at.block(
            lib,
            Vec3::new(x, (top + bottom) * 0.5, 0.06 + 0.03 * (i % 2) as f32),
            Vec3::new(w / n as f32 - 0.02, top - bottom, 0.05),
            kind::CLOTH,
            rgba(DRAPE, 1.0),
        );
    }
    at.block(
        lib,
        Vec3::new(0.0, top - 0.15, 0.14),
        Vec3::new(w, 0.26, 0.06),
        kind::CLOTH,
        [0.16, 0.06, 0.18, 1.0],
    );
}

/// The ward's corner pillars, each with a small crystal hanging over it.
fn pillars(lib: &mut Library, f: &RoomFrame) {
    let (p0, p1) = PILLAR;
    let stone = [0.3, 0.28, 0.27];
    for (sa, so) in [(-1.0f32, -1.0f32), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)] {
        let (a0, a1) = (sa * p0, sa * p1);
        let (o0, o1) = (WARD + so * p0, WARD + so * p1);
        let (lo, hi) = f.boxed(a0, a1, o0, o1, 0.0, 3.4);
        lib.solid(Inst::minmax(lo, hi, kind::STONE, stone), Stops::ALL);
        let (am, om) = ((a0 + a1) * 0.5, (o0 + o1) * 0.5);
        slab(
            lib,
            f,
            (am - 0.6, am + 0.6),
            (om - 0.6, om + 0.6),
            (3.4, 3.55),
            kind::STONE,
            [0.36, 0.34, 0.32],
        );
        crystal(lib, f, Vec3::new(am, 4.05, om), 0.6, sa * so * 0.3);
    }
}

/// A crystal `size` long hanging still at `p` (a, height, o), leaning by `lean`.
fn crystal(lib: &mut Library, f: &RoomFrame, p: Vec3, size: f32, lean: f32) {
    let at = Spot {
        at: f.at(p.x, p.z, p.y),
        q: f.turn() * Quat::from_rotation_z(lean) * Quat::from_rotation_y(lean * 2.0),
    };
    let w = size * 0.3;
    let body = size * 0.55;
    for spin in [0.0f32, FRAC_PI_4] {
        at.part(
            lib,
            Vec3::ZERO,
            Vec3::new(w, body, w),
            Quat::from_rotation_y(spin),
            kind::CRYSTAL,
            CRYSTAL_RGBA,
        );
    }
    // Points: cubes stood on a corner.
    let corner = Quat::from_rotation_arc(Vec3::ONE.normalize(), Vec3::Y);
    for y in [-1.0f32, 1.0] {
        at.part(
            lib,
            Vec3::Y * (y * body * 0.5),
            Vec3::splat(w * 0.82),
            corner,
            kind::CRYSTAL,
            CRYSTAL_RGBA,
        );
    }
}

/// The great crystal over a low stone dais in the middle of the circle, and its
/// lesser ones round it.
fn crystals(lib: &mut Library, f: &RoomFrame) {
    guard(lib, f, (-0.65, 0.65), (WARD - 0.65, WARD + 0.65), 0.4);
    let at = Spot::new(f, 0.0, WARD, 0.0);
    let stone = [0.3, 0.28, 0.27, 1.0];
    at.block(lib, Vec3::Y * 0.15, Vec3::new(1.2, 0.3, 1.2), kind::STONE, stone);
    at.part(
        lib,
        Vec3::Y * 0.15,
        Vec3::new(0.9, 0.3, 0.9),
        Quat::from_rotation_y(FRAC_PI_4),
        kind::STONE,
        stone,
    );
    at.block(
        lib,
        Vec3::Y * 0.34,
        Vec3::new(0.8, 0.08, 0.8),
        kind::STONE,
        [0.36, 0.34, 0.32, 1.0],
    );
    at.block(lib, Vec3::Y * 0.385, Vec3::new(0.3, 0.01, 0.3), kind::EMISSIVE, RUNE);
    crystal(lib, f, Vec3::new(0.0, 3.3, WARD), 1.1, 0.08);
    for (k, (r, y, size)) in [
        (1.5f32, 2.6f32, 0.45f32),
        (1.3, 3.7, 0.4),
        (1.7, 3.1, 0.5),
        (1.2, 2.3, 0.35),
    ]
    .into_iter()
    .enumerate()
    {
        let t = k as f32 / 4.0 * TAU + 0.4;
        crystal(
            lib,
            f,
            Vec3::new(t.cos() * r, y, WARD + t.sin() * r),
            size,
            0.25 + k as f32 * 0.2,
        );
    }
    lib.light(f.at(0.0, WARD, 1.4), 7.5, [0.65, 0.4, 1.0], 1.1, LightKind::Static);
}

/// The rune circle in the floor of the court: two rings with runes between them, a
/// seven-pointed star inside, a small ring at its heart, all glowing faintly.
fn rune_circle(lib: &mut Library, f: &RoomFrame) {
    let at = Spot::new(f, 0.0, WARD, 0.0);
    let line = |lib: &mut Library, p: Vec2, q: Vec2, w: f32, y: f32| {
        let d = q - p;
        let m = (p + q) * 0.5;
        at.part(
            lib,
            Vec3::new(m.x, y * 0.5, m.y),
            Vec3::new(d.length(), y, w),
            Quat::from_rotation_y(-d.y.atan2(d.x)),
            kind::EMISSIVE,
            RUNE,
        );
    };
    let ring = |lib: &mut Library, r: f32, n: usize, w: f32, y: f32| {
        for k in 0..n {
            let (t0, t1) = (k as f32 / n as f32 * TAU, (k + 1) as f32 / n as f32 * TAU);
            line(lib, Vec2::from_angle(t0) * r, Vec2::from_angle(t1) * r, w, y);
        }
    };
    ring(lib, 2.9, 36, 0.05, 0.004);
    ring(lib, 2.35, 30, 0.035, 0.005);
    ring(lib, 1.0, 16, 0.03, 0.006);
    // The star: each point joined to the point two on.
    for k in 0..7 {
        let p = Vec2::from_angle(k as f32 / 7.0 * TAU + FRAC_PI_2) * 2.3;
        let q = Vec2::from_angle((k + 2) as f32 / 7.0 * TAU + FRAC_PI_2) * 2.3;
        line(lib, p, q, 0.028, 0.007);
    }
    // Runes: a stave and a stroke or two each, standing round between the rings.
    let mut r = StdRng::seed_from_u64(0x5275_6e65);
    for k in 0..14 {
        let t = (k as f32 + 0.5) / 14.0 * TAU;
        let c = Vec2::from_angle(t) * 2.62;
        let up = Vec2::from_angle(t) * 0.15;
        let side = Vec2::new(-up.y, up.x);
        line(lib, c - up, c + up, 0.022, 0.006);
        for _ in 0..r.gen_range(1..3) {
            let a = r.gen_range(-1.0..1.0f32);
            let b = r.gen_range(-1.0..1.0f32);
            let s = if r.gen_bool(0.5) { 1.0 } else { -1.0 };
            line(lib, c + up * a, c + up * b + side * s * 0.6, 0.02, 0.0065);
        }
    }
}

/// A heavy lectern in the aisle on side `s` at `o`, facing the ward, a grimoire on it
/// chained to its post; `k` picks the grimoire.
fn lectern(lib: &mut Library, f: &RoomFrame, s: f32, o: f32, k: usize) {
    let a = s * 8.5;
    guard(lib, f, (a - 0.4, a + 0.4), (o - 0.4, o + 0.4), 1.3);
    // Facing the ward: local z toward the middle of the room.
    let at = Spot::new(f, a, o, -s * FRAC_PI_2);
    let wood = [0.08, 0.04, 0.025, 1.0];
    at.block(lib, Vec3::Y * 0.05, Vec3::new(0.66, 0.1, 0.6), kind::WOOD, wood);
    for x in [-1.0f32, 1.0] {
        at.part(
            lib,
            Vec3::new(x * 0.2, 0.5, 0.0),
            Vec3::new(0.08, 0.95, 0.08),
            Quat::from_rotation_z(-x * 0.12),
            kind::WOOD,
            wood,
        );
    }
    at.block(lib, Vec3::Y * 0.5, Vec3::new(0.36, 0.06, 0.12), kind::METAL, IRON);
    let slope = Quat::from_rotation_x(0.45);
    let desk = Vec3::Y * 1.05;
    at.part(lib, desk, Vec3::new(0.66, 0.05, 0.5), slope, kind::WOOD, wood);
    at.part(
        lib,
        desk + slope * Vec3::new(0.0, 0.04, 0.26),
        Vec3::new(0.66, 0.08, 0.03),
        slope,
        kind::METAL,
        IRON,
    );
    // The grimoire, shut, its boards bound in iron and a sigil on its cover.
    let covers = [
        [0.2, 0.05, 0.08],
        [0.06, 0.08, 0.16],
        [0.16, 0.1, 0.05],
        [0.08, 0.12, 0.08],
    ];
    let book = desk + slope * Vec3::new(0.0, 0.1, -0.02);
    at.part(
        lib,
        book,
        Vec3::new(0.42, 0.1, 0.34),
        slope,
        kind::CLOTH,
        rgba(covers[k % 4], 1.0),
    );
    at.part(
        lib,
        book + slope * Vec3::new(0.0, 0.0, 0.004),
        Vec3::new(0.4, 0.08, 0.34),
        slope,
        kind::PAPER,
        [0.0, 0.0, 0.0, 0.0],
    );
    for x in [-1.0f32, 1.0] {
        at.part(
            lib,
            book + slope * Vec3::new(x * 0.19, 0.0, 0.0),
            Vec3::new(0.04, 0.11, 0.35),
            slope,
            kind::METAL,
            IRON,
        );
    }
    at.part(
        lib,
        book + slope * Vec3::new(0.0, 0.052, 0.0),
        Vec3::new(0.1, 0.004, 0.1),
        slope * Quat::from_rotation_y(FRAC_PI_4),
        kind::EMISSIVE,
        [0.8, 0.4, 1.0, 1.2],
    );
    // The chain, from the book's spine down to a ring on the post.
    let from = at.at + at.q * (book + slope * Vec3::new(-0.21, 0.0, -0.1));
    let to = at.at + at.q * Vec3::new(-0.18, 0.55, 0.0);
    let links = 8;
    let dir = (to - from).normalize();
    for i in 0..links {
        let t = (i as f32 + 0.5) / links as f32;
        // A little sag in the middle.
        let p = from.lerp(to, t) - Vec3::Y * (t * (1.0 - t) * 0.25);
        let twist =
            Quat::from_rotation_arc(Vec3::Y, dir) * Quat::from_rotation_y(if i % 2 == 0 { 0.0 } else { FRAC_PI_2 });
        lib.decor(Inst::new(
            p,
            twist,
            Vec3::new(0.035, 0.07, 0.012),
            kind::METAL,
            [0.3, 0.3, 0.32, 1.0],
            0,
        ));
    }
    if k.is_multiple_of(2) {
        // A guttering candle.
        let c = Vec3::new(0.26, 1.15, -0.05);
        at.block(lib, c, Vec3::new(0.05, 0.12, 0.05), kind::PLAIN, [0.75, 0.7, 0.58, 1.0]);
        lib.decor(Inst::new(
            at.at + at.q * (c + Vec3::Y * 0.09),
            at.q * Quat::from_rotation_y(PI * 0.25),
            Vec3::new(0.025, 0.05, 0.025),
            kind::EMISSIVE,
            [1.0, 0.55, 0.2, 12.0],
            1,
        ));
        lib.light(
            at.at + at.q * (c + Vec3::Y * 0.4),
            4.0,
            [1.0, 0.6, 0.3],
            0.6,
            LightKind::Flicker,
        );
    }
}
