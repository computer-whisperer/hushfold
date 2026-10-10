//! The Verse Gallery: department 6, first floor, south side. A warm gallery hung with
//! portraits of poets: the books stand in a row of alcoves along the far wall, and
//! the near half of the room is a lounge of armchairs, window seats and lecterns,
//! where words drift up out of an open book and paper cranes hang in the air.

use super::*;
use crate::gfx::Vertex;
use crate::gfx::crane::{Crane, Fold, Paper, Print};
use std::f32::consts::{FRAC_PI_2, FRAC_PI_4, PI};

/// The double cases running out from the far wall between the alcoves, by `a`; each
/// two cases long, from the far wall's back cases to `MOUTH`.
const SPURS: [f32; 6] = [-9.25, -5.55, -1.85, 1.85, 5.55, 9.25];
const UNITS: [f32; 2] = [29.95, 27.25];
const MOUTH: f32 = 25.91;
/// The cases along the side walls in the corner alcoves.
const SIDE_CASES: [f32; 3] = [25.05, 27.75, 30.45];
/// Window seats in the side walls.
const SEAT: f32 = 19.5;
const GOLD: [f32; 4] = [0.66, 0.48, 0.14, 1.0];
const DAMASK: [f32; 3] = [0.3, 0.1, 0.12];
/// What rises from the open book in the lounge, a word at a time.
const VERSE: &str = "Shelve me gently, said the book, and I will keep a light for you through the long night.";

pub(super) fn furnish(lib: &mut Library, frame: &RoomFrame, rng: &mut StdRng, titles: &mut HashSet<String>) {
    walls(lib, frame);
    alcoves(lib, frame, rng, titles);
    lounge(lib, frame);
    cranes(lib, frame);
}

/// The alcoves along the far wall. Each between two spurs is a category of five, read
/// round from the mouth: the left spur's side, the back case, the right spur's side.
/// The corner ones have the side wall's three cases for a back and a side. Then the
/// near wall right of the door.
fn alcoves(lib: &mut Library, f: &RoomFrame, rng: &mut StdRng, titles: &mut HashSet<String>) {
    let (out, across) = (f.out(), f.across());
    // Each spur's faces looking -a and +a.
    let mut sides: Vec<[Vec<(Vec3, Vec3)>; 2]> = Vec::new();
    for &a in &SPURS {
        let mut pair = [Vec::new(), Vec::new()];
        for &o in &UNITS {
            for face in lib.bookcase(f.at(a, o, 0.0), f.out_along_x()) {
                pair[(face.1.dot(across) > 0.0) as usize].push(face);
            }
        }
        sides.push(pair);
    }
    let backs: Vec<(Vec3, Vec3)> = (0..5)
        .map(|i| {
            let a = (SPURS[i] + SPURS[i + 1]) * 0.5;
            lib.wall_bookcase(f.at(a, RoomFrame::O_FAR, 0.0), -out)
        })
        .collect();
    let wall = |lib: &mut Library, s: f32| -> Vec<(Vec3, Vec3)> {
        (SIDE_CASES.iter())
            .map(|&o| lib.wall_bookcase(f.at(s * RoomFrame::A_HALF, o, 0.0), -across * s))
            .collect()
    };
    let left_wall = wall(lib, -1.0);
    let right_wall = wall(lib, 1.0);
    let near: Vec<(Vec3, Vec3)> = (0..5)
        .map(|k| lib.wall_bookcase(f.at(0.65 + 2.7 * k as f32, RoomFrame::O_NEAR, 0.0), out))
        .collect();
    let mut groups: Vec<Vec<(Vec3, Vec3)>> = Vec::new();
    // Facing the far wall, an alcove's left is toward -a.
    groups.push([left_to_right(left_wall), left_to_right(sides[0][0].clone())].concat());
    for i in 0..5 {
        groups.push(
            [
                left_to_right(sides[i][1].clone()),
                vec![backs[i]],
                left_to_right(sides[i + 1][0].clone()),
            ]
            .concat(),
        );
    }
    groups.push([left_to_right(sides[5][1].clone()), left_to_right(right_wall)].concat());
    groups.push(left_to_right(near));
    for (cat, faces) in f.categories.clone().zip(groups) {
        lib.add_category(rng, cat, &faces, titles);
    }
}

/// Stone piers where the spurs meet the far wall, windows high over three alcoves and
/// tall ones with seats in the side walls, damask on the lounge's walls over the
/// panelling, portraits hung salon fashion, sconces, and beams across the ceiling.
fn walls(lib: &mut Library, f: &RoomFrame) {
    let h = RoomFrame::HEIGHT;
    let ah = RoomFrame::A_HALF;
    let (near, far) = (RoomFrame::O_NEAR, RoomFrame::O_FAR);
    for &a in &SPURS {
        let (lo, hi) = f.boxed(a - 0.55, a + 0.55, far - 0.6, far, 0.0, h);
        lib.solid(Inst::minmax(lo, hi, kind::STONE, [0.46, 0.43, 0.38]), Stops::ALL);
    }
    for a in [-7.4f32, 0.0, 7.4] {
        clerestory(lib, Spot::new(f, a, far, 0.0), 1.2);
    }
    lib.light(f.at(0.0, 29.0, 4.2), 9.0, [0.45, 0.55, 1.0], 0.5, LightKind::Static);
    for (i, a) in [-3.7f32, 3.7].into_iter().enumerate() {
        portrait(lib, &Spot::new(f, a, far, 0.0), (1.0, 1.25), 4.5, 10 + i);
    }
    for (i, a) in [-11.05f32, 11.05].into_iter().enumerate() {
        portrait(lib, &Spot::new(f, a, far, 0.0), (1.1, 1.4), 2.2, 12 + i);
    }
    let t = 0.07;
    let side_end = SIDE_CASES[0] - 1.4;
    for s in [-1.0f32, 1.0] {
        let wall = Spot::new(f, s * ah, SEAT, -s * FRAC_PI_2);
        seat_window(lib, f, s, &wall);
        // Damask above the panelling, either side of the window.
        for (o0, o1) in [(near, SEAT - 1.25), (SEAT + 1.25, side_end)] {
            damask(lib, f, (s * ah, s * (ah - 0.02)), (o0, o1), 1.16);
            lib.wainscot(f.y, f.rect(s * ah, s * (ah - t), o0, o1), f.side.across * -s);
        }
        // Portraits either side of the window, two high, two low.
        let hangs = [
            (16.2, 2.0, (0.9, 1.15)),
            (17.4, 3.75, (0.75, 0.95)),
            (22.0, 2.1, (1.0, 1.3)),
            (23.0, 3.85, (0.7, 0.85)),
        ];
        for (i, (o, y, size)) in hangs.into_iter().enumerate() {
            let at = Spot::new(f, s * ah, o, -s * FRAC_PI_2);
            portrait(lib, &at, size, y, i * 2 + (s > 0.0) as usize);
        }
        lib.sconce(f.at(s * (ah - 0.15), 21.0, 2.9), -f.across() * s);
    }
    // The near wall: panelling by the door, damask and portraits above the near cases.
    let d = f.door + RoomFrame::DOOR_HALF + 0.3;
    lib.wainscot(f.y, f.rect(d, -0.75, near, near + t), f.side.out);
    damask(lib, f, (d + 0.1, -0.75), (near, near + 0.02), 1.16);
    damask(lib, f, (-0.7, ah), (near, near + 0.02), 3.3);
    for (i, a) in [2.0f32, 10.1].into_iter().enumerate() {
        portrait(lib, &Spot::new(f, a, near, PI), (1.1, 1.3), 4.45, 14 + i);
    }
    portrait(lib, &Spot::new(f, -4.6, near, PI), (1.3, 1.6), 2.45, 16);
    lib.sconce(f.at(-2.3, near + 0.15, 2.6), f.out());
    lib.sconce(f.at(-6.9, near + 0.15, 2.6), f.out());
    for o in [16.4f32, 19.5, 22.6, 25.7, 28.8] {
        slab(
            lib,
            f,
            (-ah, ah),
            (o - 0.12, o + 0.12),
            (h - 0.26, h),
            kind::WOOD,
            [0.11, 0.055, 0.025],
        );
    }
}

/// Damask on a wall over `a`, `o` (a strip 0.02 thick on its face) from `y0` up to
/// a moulding under the ceiling.
fn damask(lib: &mut Library, f: &RoomFrame, a: (f32, f32), o: (f32, f32), y0: f32) {
    let top = RoomFrame::HEIGHT - 0.36;
    slab(lib, f, a, o, (y0, top), kind::CLOTH, DAMASK);
    // The moulding stands 0.05 proud, along whichever span is the wall's.
    let grow = |(x0, x1): (f32, f32)| {
        if (x1 - x0).abs() > 0.1 {
            (x0, x1)
        } else {
            (x0, x0 + (x1 - x0).signum() * 0.05)
        }
    };
    slab(lib, f, grow(a), grow(o), (top, top + 0.08), kind::WOOD, WALNUT);
}

/// A window high in a wall over an alcove's case, `w` wide; `at` is on the wall's
/// face, facing into the room.
fn clerestory(lib: &mut Library, at: Spot, w: f32) {
    let (y0, y1) = (3.8, 5.2);
    let stone = [0.46, 0.43, 0.38, 1.0];
    at.block(
        lib,
        Vec3::new(0.0, (y0 + y1) * 0.5, -0.02),
        Vec3::new(w, y1 - y0, 0.08),
        kind::GLASS,
        [0.32, 0.42, 0.85, 1.0],
    );
    for y in [y0 - 0.06, y1 + 0.06] {
        at.block(
            lib,
            Vec3::new(0.0, y, 0.06),
            Vec3::new(w + 0.36, 0.12, 0.12),
            kind::STONE,
            stone,
        );
    }
    for x in [-1.0f32, 1.0] {
        at.block(
            lib,
            Vec3::new(x * (w * 0.5 + 0.08), (y0 + y1) * 0.5, 0.05),
            Vec3::new(0.16, y1 - y0, 0.1),
            kind::STONE,
            stone,
        );
    }
}

/// A tall window in side wall `s` at `SEAT`, with a cushioned seat in its sill and
/// curtains drawn back either side.
fn seat_window(lib: &mut Library, f: &RoomFrame, s: f32, wall: &Spot) {
    let (w, y0, y1) = (1.7f32, 0.85f32, 4.1f32);
    let stone = [0.46, 0.43, 0.38, 1.0];
    wall.block(
        lib,
        Vec3::new(0.0, (y0 + y1) * 0.5, -0.02),
        Vec3::new(w, y1 - y0, 0.08),
        kind::GLASS,
        [0.32, 0.42, 0.85, 1.1],
    );
    wall.block(
        lib,
        Vec3::new(0.0, y1 + 0.08, 0.07),
        Vec3::new(w + 0.4, 0.16, 0.14),
        kind::STONE,
        stone,
    );
    for x in [-1.0f32, 1.0] {
        wall.block(
            lib,
            Vec3::new(x * (w * 0.5 + 0.1), (y0 + y1) * 0.5, 0.06),
            Vec3::new(0.2, y1 - y0, 0.12),
            kind::STONE,
            stone,
        );
    }
    // The seat: a panelled box, a long cushion, two pillows.
    let depth = 0.6;
    let (a0, a1) = (s * RoomFrame::A_HALF, s * (RoomFrame::A_HALF - depth));
    let (a0, a1) = (a0.min(a1), a0.max(a1));
    guard(lib, f, (a0, a1), (SEAT - 1.0, SEAT + 1.0), 0.6);
    wall.block(
        lib,
        Vec3::new(0.0, 0.22, depth * 0.5),
        Vec3::new(2.0, 0.44, depth),
        kind::WOOD,
        rgba(WALNUT, 1.0),
    );
    wall.block(
        lib,
        Vec3::new(0.0, 0.5, depth * 0.5 - 0.01),
        Vec3::new(1.9, 0.12, depth - 0.06),
        kind::CLOTH,
        [0.45, 0.16, 0.2, 1.0],
    );
    for (x, c) in [(-0.7f32, [0.6, 0.45, 0.18, 1.0]), (0.62, [0.25, 0.3, 0.42, 1.0])] {
        wall.part(
            lib,
            Vec3::new(x, 0.7, 0.2),
            Vec3::new(0.4, 0.32, 0.12),
            Quat::from_rotation_x(-0.35) * Quat::from_rotation_z(x * 0.2),
            kind::CLOTH,
            c,
        );
    }
    // Curtains gathered at either side, a pelmet over them.
    let curtain = [0.36, 0.08, 0.1, 1.0];
    for x in [-1.0f32, 1.0] {
        for (i, dx) in [0.0f32, 0.12, 0.24].into_iter().enumerate() {
            wall.block(
                lib,
                Vec3::new(x * (w * 0.5 + 0.3 + dx), 2.75, 0.2 + i as f32 * 0.03),
                Vec3::new(0.16, 3.7 - i as f32 * 0.1, 0.06),
                kind::CLOTH,
                curtain,
            );
        }
    }
    wall.block(
        lib,
        Vec3::new(0.0, 4.68, 0.25),
        Vec3::new(w + 1.4, 0.32, 0.1),
        kind::CLOTH,
        curtain,
    );
    let moon = wall.at + wall.q * Vec3::new(0.0, 2.6, 0.8);
    lib.light(moon, 6.0, [0.45, 0.55, 1.0], 0.5, LightKind::Static);
}

/// A poet's portrait `size` wide and high in a moulded gilt frame, its middle `y` up
/// the wall at `at` (on the wall's face, facing into the room); `k` picks the sitter.
/// The small ones are ovals.
fn portrait(lib: &mut Library, at: &Spot, (w, h): (f32, f32), y: f32, k: usize) {
    let oval = h < 1.0;
    let bar = 0.09;
    // The frame's section, from its outer edge inward (x) and out from the wall (y): a
    // flat, a bead, a hollow, a raised inner lip, lapping the canvas.
    let profile = [
        (0.0, 0.0),
        (0.0, 0.045),
        (0.012, 0.06),
        (0.026, 0.07),
        (0.04, 0.062),
        (0.058, 0.05),
        (0.074, 0.058),
        (0.086, 0.066),
        (bar + 0.012, 0.05),
        (bar + 0.012, 0.0),
    ]
    .map(Vec2::from);
    let (ow, oh) = (w * 0.5 + bar, h * 0.5 + bar);
    let path: Vec<Vec2> = if oval {
        (0..48)
            .map(|i| {
                let t = i as f32 / 48.0 * TAU;
                Vec2::new(libm::cosf(t) * ow, libm::sinf(t) * oh)
            })
            .collect()
    } else {
        vec![
            Vec2::new(-ow, -oh),
            Vec2::new(ow, -oh),
            Vec2::new(ow, oh),
            Vec2::new(-ow, oh),
        ]
    };
    let frame = lib.shape(moulding(&path, &profile));
    let middle = at.at + at.q * Vec3::Y * y;
    lib.decor(frame.at(middle, at.q, kind::METAL, GOLD));
    let grounds = [[0.07, 0.08, 0.06], [0.12, 0.07, 0.05], [0.05, 0.06, 0.09]];
    let ground = rgba(grounds[k % 3], 1.0);
    if oval {
        let canvas = lib.shape(Mesh::cylinder(48));
        lib.decor(canvas.sized(Vec3::new(w + 0.03, 0.02, h + 0.03)).inst(
            at.at + at.q * Vec3::new(0.0, y, 0.03),
            at.q * Quat::from_rotation_x(FRAC_PI_2),
            kind::PLAIN,
            ground,
        ));
    } else {
        at.block(lib, Vec3::new(0.0, y, 0.03), Vec3::new(w, h, 0.02), kind::PLAIN, ground);
    }
    let coats = [
        [0.06, 0.05, 0.05],
        [0.32, 0.06, 0.07],
        [0.07, 0.1, 0.24],
        [0.12, 0.2, 0.12],
        [0.28, 0.22, 0.12],
    ];
    let hairs = [
        [0.12, 0.07, 0.04],
        [0.5, 0.48, 0.45],
        [0.45, 0.32, 0.12],
        [0.05, 0.04, 0.03],
    ];
    let skin = [0.72, 0.55, 0.42, 1.0];
    let base = y - h * 0.5;
    let z = 0.045;
    let round = lib.shape(Mesh::sphere(24));
    let dome = lib.shape(Mesh::dome(24));
    let paint = |lib: &mut Library, s: Shape, p: Vec3, size: Vec3, rot: f32, kind: u32, color: [f32; 4]| {
        lib.decor(
            s.sized(size)
                .inst(at.at + at.q * p, at.q * Quat::from_rotation_z(rot), kind, color),
        );
    };
    // Shoulders and coat (rounded, rising from the bottom of the canvas or from low in
    // the oval), a white collar, the head with its hair behind it, the eyes.
    let (from, broad) = if oval { (0.1, 0.56) } else { (0.0, 0.74) };
    paint(
        lib,
        dome,
        Vec3::new(0.0, base + h * (from + 0.2), z),
        Vec3::new(w * broad, h * 0.4, 0.012),
        0.0,
        kind::CLOTH,
        rgba(coats[k % 5], 1.0),
    );
    at.part(
        lib,
        Vec3::new(0.0, base + h * (from + 0.38), z + 0.008),
        Vec3::new(w * 0.18, w * 0.18, 0.008),
        Quat::from_rotation_z(FRAC_PI_4),
        kind::PLAIN,
        [0.85, 0.83, 0.78, 1.0],
    );
    let head = Vec3::new(0.0, base + h * (from + 0.55), z + 0.004);
    let hair = rgba(hairs[(k / 2) % 4], 1.0);
    paint(
        lib,
        round,
        head + Vec3::new(0.0, h * 0.03, -0.006),
        Vec3::new(w * 0.31, h * 0.3, 0.014),
        0.0,
        kind::PLAIN,
        hair,
    );
    paint(
        lib,
        round,
        head,
        Vec3::new(w * 0.22, h * 0.25, 0.02),
        0.0,
        kind::PLAIN,
        skin,
    );
    for x in [-1.0f32, 1.0] {
        at.block(
            lib,
            head + Vec3::new(x * w * 0.045, h * 0.025, 0.009),
            Vec3::new(w * 0.03, w * 0.018, 0.004),
            kind::PLAIN,
            [0.08, 0.05, 0.04, 1.0],
        );
    }
    if k % 3 == 2 {
        // A beret.
        paint(
            lib,
            round,
            head + Vec3::new(w * 0.03, h * 0.15, 0.002),
            Vec3::new(w * 0.36, h * 0.08, 0.018),
            -0.12,
            kind::CLOTH,
            [0.08, 0.06, 0.06, 1.0],
        );
    }
}

/// A moulding run round a closed outline in xy (`path`, anticlockwise), its section
/// `profile` given as points (inward from the outline, out along z) round a closed
/// outline of its own, mitred at the outline's corners (smooth round a curve): a
/// picture's frame.
fn moulding(path: &[Vec2], profile: &[Vec2]) -> Mesh {
    let n = path.len();
    // Each corner's mitre: the way inward along it, how far that runs per unit inward
    // from the edges either side, and whether it is a corner or a bend in a curve.
    let corners: Vec<(Vec2, f32, bool)> = (0..n)
        .map(|k| {
            let d0 = (path[k] - path[(k + n - 1) % n]).normalize();
            let d1 = (path[(k + 1) % n] - path[k]).normalize();
            let m = (d0.perp() + d1.perp()).normalize();
            (m, 1.0 / m.dot(d1.perp()), d0.dot(d1) < 0.8)
        })
        .collect();
    let sides = profile.len();
    // Which hand of the section's outline is out of it.
    let turning: f32 = (0..sides).map(|j| profile[j].perp_dot(profile[(j + 1) % sides])).sum();
    let out = if turning > 0.0 { 1.0 } else { -1.0 };
    let mut mesh = Mesh::default();
    for k in 0..n {
        let k1 = (k + 1) % n;
        let inward = (path[k1] - path[k]).normalize().perp();
        for j in 0..sides {
            let (a, b) = (profile[j], profile[(j + 1) % sides]);
            let d = (b - a).normalize();
            let pn = Vec2::new(d.y, -d.x) * out;
            let first = mesh.verts.len() as u32;
            for (c, p) in [(k, a), (k, b), (k1, b), (k1, a)] {
                let (m, stretch, sharp) = corners[c];
                let across = if sharp { inward } else { m };
                let at = path[c] + m * stretch * p.x;
                mesh.verts.push(Vertex {
                    pos: [at.x, at.y, p.y],
                    normal: (across * pn.x).extend(pn.y).normalize().into(),
                });
            }
            mesh.tri(first, first + 1, first + 2);
            mesh.tri(first, first + 2, first + 3);
        }
    }
    mesh
}

/// The lounge: a reading group on a rug, two lecterns at the mouth of the middle
/// alcove (the words of one rising from its page), and lamps.
fn lounge(lib: &mut Library, f: &RoomFrame) {
    let (ca, co) = (1.6f32, 20.4f32);
    let mut rug = Inst::aabox(
        f.at(ca, co, 0.012),
        f.size(6.2, 0.024, 4.6),
        kind::RUG,
        [0.34, 0.1, 0.12],
    );
    rug.param = 180 | (140 << 8) | (70 << 16);
    lib.rug(rug);
    low_table(lib, f, ca, co);
    armchair(lib, f, (ca - 1.75, co), -FRAC_PI_2, [0.42, 0.12, 0.16]);
    armchair(lib, f, (ca + 1.75, co), FRAC_PI_2, [0.2, 0.26, 0.36]);
    settee(lib, f, (ca, co + 1.65), [0.36, 0.3, 0.14]);
    floor_lamp(lib, f, ca - 2.6, co + 1.0);
    floor_lamp(lib, f, ca + 2.6, co + 1.0);
    // A third chair by the left window, with its own lamp.
    armchair(lib, f, (-10.6, SEAT + 1.9), -0.6, [0.3, 0.22, 0.1]);
    floor_lamp(lib, f, -11.7, SEAT + 2.9);
    lectern(lib, f, -1.85, true);
    lectern(lib, f, 1.85, false);
}

/// The low table in the middle of the reading group, with a book and a candle on it.
fn low_table(lib: &mut Library, f: &RoomFrame, a: f32, o: f32) {
    let h = 0.45;
    guard(lib, f, (a - 0.55, a + 0.55), (o - 0.35, o + 0.35), h);
    let at = Spot::new(f, a, o, 0.0);
    at.block(
        lib,
        Vec3::Y * (h - 0.025),
        Vec3::new(1.1, 0.05, 0.7),
        kind::WOOD,
        rgba(WALNUT, 1.0),
    );
    let leg = [
        (0.0, 0.0),
        (0.026, 0.0),
        (0.03, 0.02),
        (0.02, 0.05),
        (0.018, 0.12),
        (0.034, 0.2),
        (0.022, 0.29),
        (0.03, 0.34),
        (0.032, h - 0.05),
        (0.0, h - 0.05),
    ];
    let leg = lib.shape(Mesh::lathe(&leg.map(Vec2::from), 16));
    for (x, z) in [(-0.48f32, -0.28f32), (0.48, -0.28), (-0.48, 0.28), (0.48, 0.28)] {
        lib.decor(leg.at(
            at.at + at.q * Vec3::new(x, 0.0, z),
            at.q,
            kind::WOOD,
            rgba(DARK_WOOD, 1.0),
        ));
    }
    at.part(
        lib,
        Vec3::new(-0.15, h + 0.02, 0.05),
        Vec3::new(0.24, 0.04, 0.32),
        Quat::from_rotation_y(0.3),
        kind::CLOTH,
        [0.3, 0.08, 0.2, 1.0],
    );
    // A candle in a brass dish.
    let foot = at.at + at.q * Vec3::new(0.3, h, -0.1);
    let dish = [
        (0.0, 0.0),
        (0.06, 0.0),
        (0.065, 0.012),
        (0.05, 0.015),
        (0.03, 0.02),
        (0.03, 0.03),
        (0.0, 0.03),
    ];
    let dish = lib.shape(Mesh::lathe(&dish.map(Vec2::from), 24));
    lib.decor(dish.at(foot, at.q, kind::METAL, [0.55, 0.38, 0.13, 1.0]));
    let wax = lib.shape(Mesh::cylinder(16));
    lib.decor(wax.sized(Vec3::new(0.042, 0.14, 0.042)).inst(
        foot + Vec3::Y * 0.1,
        at.q,
        kind::PLAIN,
        [0.85, 0.8, 0.65, 1.0],
    ));
    flame(lib, foot + Vec3::Y * 0.172, 0.05);
    let tip = Vec3::new(0.3, h + 0.19, -0.1);
    lib.light(
        at.at + at.q * tip + Vec3::Y * 0.2,
        4.5,
        [1.0, 0.6, 0.28],
        0.6,
        LightKind::Flicker,
    );
}

/// An upholstered armchair at `(a, o)` turned by `yaw` (0 faces the door's wall).
fn armchair(lib: &mut Library, f: &RoomFrame, (a, o): (f32, f32), yaw: f32, cloth: [f32; 3]) {
    guard(lib, f, (a - 0.5, a + 0.5), (o - 0.5, o + 0.5), 1.0);
    let at = Spot::new(f, a, o, yaw);
    let c = rgba(cloth, 1.0);
    for (x, z) in [(-0.3f32, -0.28f32), (0.3, -0.28), (-0.3, 0.28), (0.3, 0.28)] {
        bun_foot(lib, &at, Vec3::new(x, 0.0, z));
    }
    at.block(lib, Vec3::Y * 0.26, Vec3::new(0.78, 0.28, 0.72), kind::CLOTH, c);
    at.block(
        lib,
        Vec3::new(0.0, 0.44, 0.06),
        Vec3::new(0.56, 0.1, 0.56),
        kind::CLOTH,
        c,
    );
    at.part(
        lib,
        Vec3::new(0.0, 0.72, -0.3),
        Vec3::new(0.74, 0.62, 0.16),
        Quat::from_rotation_x(-0.12),
        kind::CLOTH,
        c,
    );
    for x in [-1.0f32, 1.0] {
        at.block(
            lib,
            Vec3::new(x * 0.33, 0.52, 0.02),
            Vec3::new(0.13, 0.22, 0.66),
            kind::CLOTH,
            c,
        );
        rolled_arm(lib, &at, Vec3::new(x * 0.335, 0.62, 0.02), 0.66, c);
    }
}

/// A settee for two at `(a, o)`, its back to the alcoves.
fn settee(lib: &mut Library, f: &RoomFrame, (a, o): (f32, f32), cloth: [f32; 3]) {
    guard(lib, f, (a - 0.95, a + 0.95), (o - 0.45, o + 0.45), 1.0);
    let at = Spot::new(f, a, o, 0.0);
    let c = rgba(cloth, 1.0);
    for (x, z) in [(-0.8f32, -0.3f32), (0.8, -0.3), (-0.8, 0.3), (0.8, 0.3)] {
        bun_foot(lib, &at, Vec3::new(x, 0.0, z));
    }
    at.block(lib, Vec3::Y * 0.26, Vec3::new(1.8, 0.28, 0.76), kind::CLOTH, c);
    for x in [-0.42f32, 0.42] {
        at.block(
            lib,
            Vec3::new(x, 0.44, -0.04),
            Vec3::new(0.78, 0.1, 0.6),
            kind::CLOTH,
            c,
        );
    }
    at.part(
        lib,
        Vec3::new(0.0, 0.74, 0.32),
        Vec3::new(1.76, 0.6, 0.14),
        Quat::from_rotation_x(0.12),
        kind::CLOTH,
        c,
    );
    for x in [-1.0f32, 1.0] {
        at.block(
            lib,
            Vec3::new(x * 0.84, 0.54, -0.02),
            Vec3::new(0.12, 0.26, 0.7),
            kind::CLOTH,
            c,
        );
        rolled_arm(lib, &at, Vec3::new(x * 0.845, 0.66, -0.02), 0.7, c);
    }
}

/// A turned bun foot under a seat, standing on `p` in its frame.
fn bun_foot(lib: &mut Library, at: &Spot, p: Vec3) {
    let bun = [
        (0.0, 0.0),
        (0.022, 0.0),
        (0.036, 0.025),
        (0.04, 0.055),
        (0.032, 0.085),
        (0.024, 0.1),
        (0.03, 0.12),
        (0.0, 0.12),
    ];
    let bun = lib.shape(Mesh::lathe(&bun.map(Vec2::from), 16));
    lib.decor(bun.at(at.at + at.q * p, at.q, kind::WOOD, rgba(DARK_WOOD, 1.0)));
}

/// A seat's arm rolled over along its top, centred on `p` (in its frame), `len` long
/// front to back.
fn rolled_arm(lib: &mut Library, at: &Spot, p: Vec3, len: f32, cloth: [f32; 4]) {
    let roll = lib.shape(Mesh::cylinder(24));
    lib.decor(roll.sized(Vec3::new(0.15, len, 0.15)).inst(
        at.at + at.q * p,
        at.q * Quat::from_rotation_x(FRAC_PI_2),
        kind::CLOTH,
        cloth,
    ));
}

/// A standard lamp with a glowing shade.
fn floor_lamp(lib: &mut Library, f: &RoomFrame, a: f32, o: f32) {
    guard(lib, f, (a - 0.2, a + 0.2), (o - 0.2, o + 0.2), 1.7);
    let at = Spot::new(f, a, o, 0.0);
    let brass = [0.55, 0.38, 0.13, 1.0];
    let foot = [
        (0.0, 0.0),
        (0.17, 0.0),
        (0.17, 0.02),
        (0.12, 0.04),
        (0.05, 0.07),
        (0.03, 0.1),
        (0.0, 0.1),
    ];
    let foot = lib.shape(Mesh::lathe(&foot.map(Vec2::from), 32));
    lib.decor(foot.at(at.at, at.q, kind::METAL, brass));
    let stem = lib.shape(Mesh::cylinder(12));
    lib.decor(
        stem.sized(Vec3::new(0.03, 1.45, 0.03))
            .inst(at.at + Vec3::Y * 0.82, at.q, kind::METAL, brass),
    );
    let shade = Mesh::lathe(&[Vec2::new(0.2, 0.0), Vec2::new(0.13, 0.27)], 32);
    let shade = lib.shape(shade.two_sided());
    lib.decor(shade.at(at.at + Vec3::Y * 1.49, at.q, kind::EMISSIVE, [1.0, 0.55, 0.24, 0.7]));
    let finial = lib.shape(Mesh::sphere(12));
    lib.decor(
        finial
            .sized(Vec3::splat(0.04))
            .inst(at.at + Vec3::Y * 1.78, at.q, kind::METAL, brass),
    );
    lib.light(at.at + Vec3::Y * 1.45, 6.0, [1.0, 0.62, 0.32], 1.0, LightKind::Static);
}

/// A lectern at the mouth of the middle alcove, facing the door, an open book on it;
/// from one (`words`) the lines lift off the page and drift up, glowing.
fn lectern(lib: &mut Library, f: &RoomFrame, a: f32, words: bool) {
    let o = MOUTH - 0.55;
    guard(lib, f, (a - 0.35, a + 0.35), (o - 0.3, o + 0.3), 1.3);
    let at = Spot::new(f, a, o, 0.0);
    let wood = rgba(WALNUT, 1.0);
    at.block(
        lib,
        Vec3::Y * 0.04,
        Vec3::new(0.56, 0.08, 0.5),
        kind::WOOD,
        rgba(DARK_WOOD, 1.0),
    );
    let post = [
        (0.0, 0.0),
        (0.08, 0.0),
        (0.08, 0.03),
        (0.055, 0.06),
        (0.045, 0.2),
        (0.062, 0.45),
        (0.042, 0.7),
        (0.036, 0.85),
        (0.05, 0.92),
        (0.065, 0.97),
        (0.0, 0.97),
    ];
    let post = lib.shape(Mesh::lathe(&post.map(Vec2::from), 24));
    lib.decor(post.at(at.at + Vec3::Y * 0.08, at.q, kind::WOOD, wood));
    let slope = Quat::from_rotation_x(0.42);
    let desk = Vec3::Y * 1.1;
    at.part(lib, desk, Vec3::new(0.62, 0.04, 0.46), slope, kind::WOOD, wood);
    for x in [-1.0f32, 1.0] {
        at.part(
            lib,
            desk + slope * Vec3::new(x * 0.15, 0.04, 0.0),
            Vec3::new(0.28, 0.035, 0.38),
            slope * Quat::from_rotation_z(-x * 0.06),
            kind::PAPER,
            [0.0, 0.0, 0.0, 0.0],
        );
    }
    let ink = [0.2, 0.12, 0.1, 1.0];
    for i in 0..5 {
        for x in [-1.0f32, 1.0] {
            at.part(
                lib,
                desk + slope * Vec3::new(x * 0.15, 0.06, -0.13 + i as f32 * 0.065),
                Vec3::new(0.2, 0.003, 0.01),
                slope,
                kind::PLAIN,
                ink,
            );
        }
    }
    if !words {
        return;
    }
    // The words, rising in a slow curl and fading as they go.
    let mut r = StdRng::seed_from_u64(0x5665_7273);
    let words = VERSE.split(' ').collect::<Vec<_>>();
    for (i, word) in words.iter().enumerate() {
        let t = i as f32 / words.len() as f32;
        let p = desk
            + Vec3::new(
                libm::sinf(t * 7.0) * 0.22 * t + 0.05,
                0.12 + t * 1.5,
                libm::cosf(t * 7.0) * 0.15 * t,
            );
        // Each turned and tipped a little, but all toward the reader.
        let q = at.q * Quat::from_rotation_y(r.gen_range(-0.6..0.6)) * Quat::from_rotation_z(r.gen_range(-0.4..0.4));
        let style = Style {
            em: 0.045,
            tracking: 0.0,
            lettering: Lettering::Glow,
            color: [1.0, 0.82, 0.5, 3.0 * (1.0 - t) + 0.4],
        };
        text::set_line(
            &mut lib.text,
            word,
            style,
            at.at + at.q * p,
            q * Vec3::X,
            q * Vec3::Y,
            1.0,
        );
    }
    lib.light(at.at + Vec3::Y * 1.6, 3.5, [1.0, 0.78, 0.45], 0.5, LightKind::Static);
}

/// Paper cranes hanging on threads from the ceiling over the lounge, faintly aglow,
/// each folded from a sheet of its own (pages of verse, one of a song, one of plain
/// rose paper), swaying a little in the air and turning slowly on their threads.
fn cranes(lib: &mut Library, f: &RoomFrame) {
    let spots = [
        (-2.0f32, 18.0f32, 3.7f32, 0.4f32),
        (0.6, 17.2, 4.1, 2.0),
        (3.5, 18.6, 3.5, 4.1),
        (5.2, 21.3, 3.9, 1.2),
        (-0.8, 22.4, 3.6, 3.3),
        (2.4, 23.4, 4.2, 5.5),
        (-4.2, 20.8, 4.0, 2.6),
    ];
    let papers = [
        ([0.94, 0.9, 0.8], Print::Text),
        ([0.86, 0.9, 0.82], Print::Text),
        ([0.95, 0.92, 0.84], Print::Music),
        ([0.84, 0.86, 0.93], Print::Text),
        ([0.93, 0.8, 0.78], Print::Plain),
        ([0.95, 0.9, 0.74], Print::Text),
        ([0.9, 0.84, 0.92], Print::Text),
    ];
    let thread = lib.shape(Mesh::cylinder(6));
    for (k, ((a, o, y, yaw), (tint, print))) in spots.into_iter().zip(papers).enumerate() {
        let bird = Crane {
            fold: Fold::Crane,
            at: f.at(a, o, y),
            rot: f.turn() * Quat::from_rotation_y(yaw) * Quat::from_rotation_z(0.15),
            size: 0.85,
            raise: 0.1 + 0.08 * (k % 3) as f32,
            wrist: -0.15,
            paper: Paper { tint, print },
            glow: [1.0, 0.85, 0.6, 0.12],
        };
        // Hung from the ceiling over its back; it sways from there, and turns about
        // its thread.
        let back = bird.at + bird.rot * Vec3::Y * 0.017;
        let hook = Vec3::new(back.x, f.y + RoomFrame::HEIGHT, back.z);
        let k = k as f32;
        let sway = Motion::Swing {
            axis: Quat::from_rotation_y(k * 1.3) * Vec3::X,
            amp: 0.025,
            period: 6.0 + 0.7 * k,
        };
        let air = lib.mover(hook, sway, k * 1.9, None);
        let twist = Motion::Swing {
            axis: Vec3::Y,
            amp: 0.35,
            period: 13.0 + 1.6 * k,
        };
        let m = lib.mover(hook, twist, k * 3.1, Some(air));
        lib.moving(
            m,
            thread.sized(Vec3::new(0.003, hook.y - back.y, 0.003)).inst(
                (hook + back) * 0.5,
                Quat::IDENTITY,
                kind::PLAIN,
                [0.8, 0.76, 0.66, 1.0],
            ),
        );
        for part in bird.parts() {
            lib.moving(m, part);
        }
    }
}

/// A candle's flame standing `h` tall on `at`.
fn flame(lib: &mut Library, at: Vec3, h: f32) {
    let drop = [
        (0.0, 0.0),
        (0.22, 0.06),
        (0.34, 0.2),
        (0.3, 0.42),
        (0.16, 0.72),
        (0.0, 1.0),
    ];
    let drop = lib.shape(Mesh::lathe(&drop.map(Vec2::from), 12));
    lib.translucent(
        drop.scaled(h)
            .at(at, Quat::IDENTITY, kind::FIRE, [1.0, 0.62, 0.25, 4.0]),
    );
}
