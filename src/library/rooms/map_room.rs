//! The Map Room: department 4, first floor, north side. An open chart floor down the
//! middle, a great globe over a compass rose set in the floor and a long map table
//! under a little ship sailing the air; the books stand in alcoves off both side walls
//! (racks of rolled maps on the backs of the first ones) and along the far and near
//! walls, with maps hung over them.

use super::*;
use crate::gfx::Vertex;
use std::f32::consts::{FRAC_PI_2, FRAC_PI_4, PI};

/// Where the dividers between the side alcoves stand (the middle of each one's
/// footprint): the first a pair of wall cases backed by a map rack, the others double
/// cases. All reach from the side walls to |a| = `INNER`.
const DIVIDERS: [f32; 3] = [20.41, 24.1, 27.8];
/// The back of the first divider's cases (its rack stands before it).
const RACK_BACK: f32 = 20.38;
const RACK_FRONT: f32 = 19.93;
/// Where the cases of a divider stand across the room, as |a|.
const UNITS: [f32; 2] = [10.95, 8.25];
/// Where the dividers end toward the middle of the room, as |a|.
const INNER: f32 = 6.91;
/// The alcoves' back cases, on the side walls.
const BACKS: [f32; 3] = [22.25, 25.95, 29.79];
/// The globe and its compass rose, and the map table.
const GLOBE: (f32, f32) = (0.0, 21.0);
const TABLE: (f32, f32) = (0.0, 27.2);
const BRASS_RGBA: [f32; 4] = [0.62, 0.45, 0.16, 1.0];
const SEPIA: [f32; 3] = [0.22, 0.13, 0.06];

pub(super) fn furnish(lib: &mut Library, frame: &RoomFrame, rng: &mut StdRng, titles: &mut HashSet<String>) {
    walls(lib, frame);
    shelves(lib, frame, rng, titles);
    rack(lib, frame);
    press(lib, frame);
    compass_rose(lib, frame);
    globe(lib, frame);
    table(lib, frame);
    ship(lib, frame);
}

/// A double case's two faces as (the one facing `dir`, the other).
fn facing(faces: [(Vec3, Vec3); 2], dir: Vec3) -> ((Vec3, Vec3), (Vec3, Vec3)) {
    if faces[0].1.dot(dir) > 0.0 {
        (faces[0], faces[1])
    } else {
        (faces[1], faces[0])
    }
}

/// Every category's cases. Down each side wall three alcoves of five faces: the
/// near divider's side, the back case, the far divider's side (the last alcove's far
/// side is the far wall), read round from the mouth. Then the middle of the far wall,
/// and the near wall right of the door.
fn shelves(lib: &mut Library, f: &RoomFrame, rng: &mut StdRng, titles: &mut HashSet<String>) {
    let (out, across) = (f.out(), f.across());
    let along_a = !f.out_along_x();
    let far: Vec<(Vec3, Vec3)> = (0..9)
        .map(|k| lib.wall_bookcase(f.at(-10.8 + 2.7 * k as f32, RoomFrame::O_FAR, 0.0), -out))
        .collect();
    let near: Vec<(Vec3, Vec3)> = (0..5)
        .map(|k| lib.wall_bookcase(f.at(0.65 + 2.7 * k as f32, RoomFrame::O_NEAR, 0.0), out))
        .collect();
    // Each side's alcoves, in order from the door's end.
    let mut alcoves: Vec<Vec<Vec<(Vec3, Vec3)>>> = Vec::new();
    for s in [-1.0f32, 1.0] {
        // Faces looking outward (+o) and inward (-o) of each divider.
        let mut up: Vec<Vec<(Vec3, Vec3)>> = Vec::new();
        let mut down: Vec<Vec<(Vec3, Vec3)>> = Vec::new();
        up.push(
            UNITS
                .iter()
                .map(|&a| lib.wall_bookcase(f.at(s * a, RACK_BACK, 0.0), out))
                .collect(),
        );
        for &o in &DIVIDERS[1..] {
            let (mut u, mut d) = (Vec::new(), Vec::new());
            for &a in &UNITS {
                let (p, q) = facing(lib.bookcase(f.at(s * a, o, 0.0), along_a), out);
                u.push(p);
                d.push(q);
            }
            up.push(u);
            down.push(d);
        }
        let corner = if s < 0.0 { far[..2].to_vec() } else { far[7..].to_vec() };
        down.push(corner);
        let side: Vec<Vec<(Vec3, Vec3)>> = (0..3)
            .map(|i| {
                let back = lib.wall_bookcase(f.at(s * RoomFrame::A_HALF, BACKS[i], 0.0), -across * s);
                // Facing the back, the alcove's left is toward +o on the right side of
                // the room and toward -o on the left.
                let (first, last) = if s < 0.0 {
                    (up[i].clone(), down[i].clone())
                } else {
                    (down[i].clone(), up[i].clone())
                };
                [left_to_right(first), vec![back], left_to_right(last)].concat()
            })
            .collect();
        alcoves.push(side);
    }
    let cats: Vec<usize> = f.categories.clone().collect();
    let [left, right] = [&alcoves[0], &alcoves[1]];
    let order: [Vec<(Vec3, Vec3)>; 8] = [
        left[0].clone(),
        left[1].clone(),
        left[2].clone(),
        left_to_right(far[2..7].to_vec()),
        right[2].clone(),
        right[1].clone(),
        right[0].clone(),
        left_to_right(near),
    ];
    for (cat, faces) in cats.into_iter().zip(order) {
        lib.add_category(rng, cat, &faces, titles);
    }
}

/// Stone piers between the alcoves rising to beams across the ceiling, windows high
/// in the alcoves and the far corners, panelling and sconces where the walls are
/// bare, and maps hung over the cases.
fn walls(lib: &mut Library, f: &RoomFrame) {
    let h = RoomFrame::HEIGHT;
    let ah = RoomFrame::A_HALF;
    let (near, far) = (RoomFrame::O_NEAR, RoomFrame::O_FAR);
    let pale = [0.46, 0.43, 0.38];
    for s in [-1.0f32, 1.0] {
        for &o in &DIVIDERS {
            // Wide enough to take the meeting cornices of a divider and its neighbours.
            let (lo, hi) = f.boxed(s * 12.2, s * ah, o - 0.55, o + 0.55, 0.0, h);
            lib.solid(Inst::minmax(lo, hi, kind::STONE, pale), Stops::ALL);
        }
        for &o in &BACKS {
            window(lib, Spot::new(f, s * ah, o, -s * FRAC_PI_2), 1.3);
        }
        window(lib, Spot::new(f, s * 9.45, far, 0.0), 1.6);
        // The bare wall by the door, panelled, with a sconce.
        let t = 0.07;
        lib.wainscot(f.y, f.rect(s * ah, s * (ah - t), near, RACK_FRONT), f.side.across * -s);
        lib.sconce(f.at(s * (ah - 0.15), 17.0, 2.6), -f.across() * s);
    }
    let d = f.door + RoomFrame::DOOR_HALF + 0.3;
    lib.wainscot(f.y, f.rect(d, -0.75, near, near + 0.07), f.side.out);
    for &o in &[16.9, DIVIDERS[0], DIVIDERS[1], DIVIDERS[2], 30.6] {
        slab(
            lib,
            f,
            (-ah, ah),
            (o - 0.13, o + 0.13),
            (h - 0.3, h),
            kind::WOOD,
            [0.11, 0.055, 0.025],
        );
    }
    // Moonlight in at the right-hand windows, slanting down into the alcoves.
    for &o in &BACKS {
        let c = f.at(ah - 0.1, o, 4.5);
        let beam = (-f.across() * 1.0 - Vec3::Y * 0.75 + f.out() * 0.15).normalize();
        let len = 6.5;
        lib.static_trans.push(Inst::new(
            c + beam * len * 0.5,
            Quat::from_rotation_arc(Vec3::Y, beam),
            Vec3::new(1.1, len, 1.1),
            kind::SHAFT,
            [0.35, 0.45, 0.9, 0.035],
            0,
        ));
    }
    for s in [-1.0f32, 1.0] {
        lib.light(
            f.at(s * 10.5, BACKS[1], 4.0),
            9.0,
            [0.45, 0.55, 1.0],
            0.6,
            LightKind::Static,
        );
    }
    // Maps: two over the middle of the far wall (the category's sign hangs between),
    // two over the near cases, and a great sea chart beside the door.
    for (i, a) in [-4.3f32, 4.3].into_iter().enumerate() {
        wall_map(lib, Spot::new(f, a, far, 0.0), (2.6, 1.5), 4.45, i as u64);
    }
    for (i, a) in [2.0f32, 10.1].into_iter().enumerate() {
        wall_map(lib, Spot::new(f, a, near, PI), (2.2, 1.3), 4.35, 2 + i as u64);
    }
    wall_map(lib, Spot::new(f, -4.7, near, PI), (3.6, 1.9), 2.3, 4);
}

/// A tall moonlit window high in a wall, `w` wide, with a stone surround; `at` is on
/// the wall's face, facing into the room.
fn window(lib: &mut Library, at: Spot, w: f32) {
    let (y0, y1) = (3.75, 5.25);
    let stone = [0.46, 0.43, 0.38, 1.0];
    let glass = [0.32, 0.42, 0.85, 1.1];
    at.part(
        lib,
        Vec3::new(0.0, (y0 + y1) * 0.5, -0.02),
        Vec3::new(w, y1 - y0, 0.08),
        Quat::IDENTITY,
        kind::GLASS,
        glass,
    );
    // Sill and lintel, and the jambs between them.
    for y in [y0 - 0.06, y1 + 0.06] {
        at.part(
            lib,
            Vec3::new(0.0, y, 0.06),
            Vec3::new(w + 0.36, 0.12, 0.12),
            Quat::IDENTITY,
            kind::STONE,
            stone,
        );
    }
    for x in [-1.0f32, 1.0] {
        at.part(
            lib,
            Vec3::new(x * (w * 0.5 + 0.08), (y0 + y1) * 0.5, 0.05),
            Vec3::new(0.16, y1 - y0, 0.1),
            Quat::IDENTITY,
            kind::STONE,
            stone,
        );
    }
}

/// A framed map `size` wide and high, its middle `y` up the wall at `at` (on the
/// wall's face, facing into the room): a sheet inked as a map, and drawn boldly over
/// it (to read from across the room) a coast or two, lines of latitude, a route.
fn wall_map(lib: &mut Library, at: Spot, (w, h): (f32, f32), y: f32, seed: u64) {
    let frame = rgba([0.1, 0.05, 0.022], 1.0);
    let bar = 0.07;
    for s in [-1.0f32, 1.0] {
        at.part(
            lib,
            Vec3::new(0.0, y + s * (h * 0.5 + bar * 0.5), 0.035),
            Vec3::new(w + 2.0 * bar, bar, 0.07),
            Quat::IDENTITY,
            kind::WOOD,
            frame,
        );
        at.part(
            lib,
            Vec3::new(s * (w * 0.5 + bar * 0.5), y, 0.035),
            Vec3::new(bar, h, 0.07),
            Quat::IDENTITY,
            kind::WOOD,
            frame,
        );
    }
    let mut sheet = Inst::new(
        at.at + at.q * Vec3::new(0.0, y, 0.029),
        at.q,
        Vec3::new(w, h, 0.012),
        kind::PARCHMENT,
        [0.78, 0.68, 0.48, 1.0],
        0,
    );
    sheet.param = 1;
    lib.decor(sheet);
    let z = 0.037;
    let ink = rgba(SEPIA, 1.0);
    for k in 1..4 {
        let yy = y - h * 0.5 + h * k as f32 / 4.0;
        at.part(
            lib,
            Vec3::new(0.0, yy, z - 0.002),
            Vec3::new(w * 0.94, 0.008, 0.004),
            Quat::IDENTITY,
            kind::PLAIN,
            [0.45, 0.36, 0.24, 1.0],
        );
    }
    let mut r = StdRng::seed_from_u64(0x4d61_7073 ^ seed);
    let blobs = if w > 3.0 { 3 } else { 2 };
    for b in 0..blobs {
        let cx = (b as f32 + 0.5) / blobs as f32 * w - w * 0.5 + r.gen_range(-0.15..0.15) * w / blobs as f32;
        let cy = y + r.gen_range(-0.15..0.15) * h;
        let rad = Vec2::new(w / blobs as f32 * 0.32, h * 0.3);
        let n = 9;
        let pts: Vec<Vec2> = (0..n)
            .map(|i| {
                let t = i as f32 / n as f32 * TAU;
                let k = r.gen_range(0.65..1.15);
                Vec2::new(cx + libm::cosf(t) * rad.x * k, cy + libm::sinf(t) * rad.y * k)
            })
            .collect();
        for i in 0..n {
            stroke(lib, &at, pts[i], pts[(i + 1) % n], z, ink);
        }
    }
    // A dotted route between the coasts.
    let (p, q) = (Vec2::new(-w * 0.35, y - h * 0.3), Vec2::new(w * 0.35, y + h * 0.25));
    for i in 0..6 {
        let t = (i as f32 + 0.25) / 6.0;
        stroke(
            lib,
            &at,
            p.lerp(q, t),
            p.lerp(q, t + 0.08),
            z + 0.001,
            [0.55, 0.1, 0.06, 1.0],
        );
    }
}

/// An inked line on a map hanging at `at`, from `p` to `q` (across, up).
fn stroke(lib: &mut Library, at: &Spot, p: Vec2, q: Vec2, z: f32, ink: [f32; 4]) {
    let d = q - p;
    let mid = (p + q) * 0.5;
    at.part(
        lib,
        Vec3::new(mid.x, mid.y, z),
        Vec3::new(d.length() + 0.012, 0.012, 0.004),
        Quat::from_rotation_z(libm::atan2f(d.y, d.x)),
        kind::PLAIN,
        ink,
    );
}

/// The span `x0..x1` of |a| on side `s`, as an `a` range.
fn span(s: f32, x0: f32, x1: f32) -> (f32, f32) {
    ((s * x0).min(s * x1), (s * x0).max(s * x1))
}

/// A rack of pigeonholes stacked with rolled maps on the back of the left first
/// divider, facing the door.
fn rack(lib: &mut Library, f: &RoomFrame) {
    let s = -1.0;
    let (a0, a1) = (INNER + 0.04, 12.2f32);
    let (o0, o1) = (RACK_FRONT, RACK_BACK);
    let top = 2.3;
    let wood = [0.13, 0.065, 0.03];
    guard(lib, f, span(s, a0, a1), (o0, o1), top);
    slab(
        lib,
        f,
        span(s, a0, a1),
        (o0, o1),
        (top - 0.05, top),
        kind::WOOD,
        [0.16, 0.08, 0.035],
    );
    slab(
        lib,
        f,
        span(s, a0 + 0.05, a1 - 0.05),
        (o0 + 0.02, o1),
        (0.0, 0.1),
        kind::WOOD,
        [0.1, 0.05, 0.022],
    );
    for x in [a0, a1 - 0.05] {
        slab(
            lib,
            f,
            span(s, x, x + 0.05),
            (o0 + 0.02, o1),
            (0.0, top - 0.05),
            kind::WOOD,
            wood,
        );
    }
    let (cols, rows) = (6, 4);
    let cw = (a1 - a0 - 0.1) / cols as f32;
    let rh = (top - 0.15) / rows as f32;
    for k in 1..rows {
        let y = 0.1 + k as f32 * rh;
        slab(
            lib,
            f,
            span(s, a0 + 0.05, a1 - 0.05),
            (o0 + 0.02, o1),
            (y - 0.015, y + 0.015),
            kind::WOOD,
            wood,
        );
    }
    for k in 1..cols {
        let x = a0 + 0.05 + k as f32 * cw;
        slab(
            lib,
            f,
            span(s, x - 0.015, x + 0.015),
            (o0 + 0.03, o1),
            (0.1, top - 0.05),
            kind::WOOD,
            wood,
        );
    }
    let tints = [
        [0.62, 0.52, 0.36],
        [0.55, 0.45, 0.3],
        [0.66, 0.58, 0.42],
        [0.5, 0.5, 0.42],
        [0.6, 0.48, 0.33],
    ];
    let mut r = StdRng::seed_from_u64(0x5261_636b);
    let roll = lib.shape(Mesh::cylinder(12));
    // Each pigeonhole a little pyramid of rolls lying end on, their turns showing.
    for col in 0..cols {
        for row in 0..rows {
            let x0 = a0 + 0.05 + col as f32 * cw + 0.03;
            let y0 = 0.1 + row as f32 * rh + 0.015;
            let d = r.gen_range(0.11..0.135f32);
            let n = r.gen_range(2..=((cw - 0.06) / d) as usize);
            for (tier, count) in [n, n - 1].into_iter().enumerate() {
                for i in 0..count {
                    let x = x0 + d * (0.5 + i as f32 + tier as f32 * 0.5);
                    let y = y0 + tier as f32 * d * 0.86;
                    let len = r.gen_range(0.36..0.47f32);
                    let c = f.at(s * x, o1 - 0.02 - len * 0.5, y + d * 0.5);
                    let end_on = Quat::from_rotation_x(FRAC_PI_2) * Quat::from_rotation_y(r.gen_range(-1.0..1.0));
                    lib.decor(roll.sized(Vec3::new(d * 0.95, len, d * 0.95)).inst(
                        c,
                        f.turn() * end_on,
                        kind::PARCHMENT,
                        rgba(tints[r.gen_range(0..tints.len())], 1.0),
                    ));
                }
            }
        }
    }
}

/// A press of wide shallow map drawers on the back of the right first divider, facing
/// the door, with charts left lying on it.
fn press(lib: &mut Library, f: &RoomFrame) {
    let s = 1.0;
    let (a0, a1) = (INNER + 0.04, 12.2f32);
    let (o0, o1) = (RACK_FRONT, RACK_BACK);
    let top = 1.95;
    guard(lib, f, span(s, a0, a1), (o0, o1), top);
    slab(
        lib,
        f,
        span(s, a0 - 0.03, a1),
        (o0 - 0.03, o1),
        (top - 0.05, top),
        kind::WOOD,
        WALNUT,
    );
    slab(
        lib,
        f,
        span(s, a0, a1),
        (o0 + 0.03, o1),
        (0.0, top - 0.05),
        kind::WOOD,
        [0.11, 0.055, 0.025],
    );
    let rows = 7;
    let rh = (top - 0.17) / rows as f32;
    let cw = (a1 - a0 - 0.15) / 2.0;
    for col in 0..2 {
        let x0 = a0 + 0.05 + col as f32 * (cw + 0.05);
        for row in 0..rows {
            let y0 = 0.1 + row as f32 * rh;
            slab(
                lib,
                f,
                span(s, x0, x0 + cw),
                (o0, o0 + 0.03),
                (y0 + 0.01, y0 + rh - 0.01),
                kind::WOOD,
                [0.15, 0.075, 0.033],
            );
            let xm = x0 + cw * 0.5;
            let ym = y0 + rh * 0.5;
            slab(
                lib,
                f,
                span(s, xm - 0.12, xm + 0.12),
                (o0 - 0.03, o0),
                (ym - 0.015, ym + 0.015),
                kind::METAL,
                [0.62, 0.45, 0.16],
            );
        }
    }
    wall_map(lib, Spot::new(f, s * 9.58, RACK_BACK, 0.0), (3.8, 0.8), 2.62, 5);
    // Charts left out on top: a loose sheet or two and a few rolls.
    let lay = Spot {
        at: f.at(s * 9.6, (o0 + o1) * 0.5, top),
        q: f.turn() * Quat::from_rotation_y(0.06),
    };
    lay.part(
        lib,
        Vec3::Y * 0.004,
        Vec3::new(1.1, 0.008, 0.36),
        Quat::IDENTITY,
        kind::PARCHMENT,
        [0.74, 0.64, 0.45, 1.0],
    );
    lay.part(
        lib,
        Vec3::new(0.25, 0.011, 0.02),
        Vec3::new(0.8, 0.006, 0.3),
        Quat::from_rotation_y(-0.2),
        kind::PARCHMENT,
        [0.8, 0.72, 0.52, 1.0],
    );
    // Two side by side, the third in the groove on top of them.
    let roll = lib.shape(Mesh::cylinder(16));
    for (i, (o, y)) in [(0.12f32, 0.04f32), (0.22, 0.04), (0.17, 0.102)]
        .into_iter()
        .enumerate()
    {
        let c = f.at(s * (8.0 + i as f32 * 0.03), o0 + o, top + y);
        lib.decor(roll.sized(Vec3::new(0.08, 0.95 - i as f32 * 0.1, 0.08)).inst(
            c,
            f.turn() * Quat::from_rotation_z(FRAC_PI_2) * Quat::from_rotation_y(i as f32),
            kind::PARCHMENT,
            [0.66, 0.56, 0.4, 1.0],
        ));
    }
}

/// A compass rose inlaid in the floor round the globe: two brass rings, and the eight
/// points of the winds, each split down its middle into a bright half and a dark.
fn compass_rose(lib: &mut Library, f: &RoomFrame) {
    let spot = Spot::new(f, GLOBE.0, GLOBE.1, 0.0);
    let flat = Quat::from_rotation_x(FRAC_PI_2);
    for (r, h) in [(2.75f32, 0.004f32), (2.5, 0.005)] {
        let ring = lib.shape(Mesh::ring(&circle(r + 0.025, 96), &circle(r - 0.025, 96), h));
        lib.decor(ring.inst(spot.at + Vec3::Y * (h * 0.5), spot.q * flat, kind::METAL, BRASS_RGBA));
    }
    let (mut bright, mut dark) = (Mesh::default(), Mesh::default());
    for k in 0..8 {
        let t = k as f32 / 8.0 * TAU;
        // The cardinal points stand a hair above the others where they cross.
        let (len, half, h) = if k % 2 == 0 {
            (3.3f32, 0.34f32, 0.006f32)
        } else {
            (2.3, 0.22, 0.004)
        };
        let dir = Vec2::from_angle(t);
        let waist = dir * (len * 0.2);
        for (e, half_point) in [(1.0f32, &mut bright), (-1.0, &mut dark)] {
            let outline = [Vec2::ZERO, dir * len, waist + dir.perp() * half * e];
            half_point.add(&Mesh::prism(&outline, h), Vec3::Y * (h * 0.5), flat, Vec3::ONE);
        }
    }
    let bright = lib.shape(bright);
    lib.decor(bright.at(spot.at, spot.q, kind::METAL, BRASS_RGBA));
    let dark = lib.shape(dark);
    lib.decor(dark.at(spot.at, spot.q, kind::METAL, [0.3, 0.2, 0.08, 1.0]));
}

/// The great globe on its stand over the compass rose, turning slowly on its tilted
/// axis in the meridian ring: seas that glow faintly from within, lands and ice caps
/// standing a little proud of them, a brass horizon on four curved legs.
fn globe(lib: &mut Library, f: &RoomFrame) {
    let spot = Spot::new(f, GLOBE.0, GLOBE.1, 0.0);
    let (gx, go) = GLOBE;
    guard(lib, f, (gx - 1.05, gx + 1.05), (go - 1.05, go + 1.05), 2.45);
    let dark = rgba([0.09, 0.045, 0.02], 1.0);
    let wood = rgba(WALNUT, 1.0);
    let foot = [
        (0.0, 0.0),
        (0.64, 0.0),
        (0.64, 0.05),
        (0.6, 0.08),
        (0.5, 0.1),
        (0.46, 0.1),
        (0.46, 0.14),
        (0.42, 0.18),
        (0.0, 0.18),
    ];
    let foot = lib.shape(Mesh::lathe(&foot.map(Vec2::from), 64));
    lib.decor(foot.at(spot.at, spot.q, kind::WOOD, dark));
    let column = [
        (0.0, 0.0),
        (0.1, 0.0),
        (0.1, 0.04),
        (0.07, 0.06),
        (0.06, 0.12),
        (0.085, 0.2),
        (0.07, 0.3),
        (0.045, 0.36),
        (0.06, 0.4),
        (0.06, 0.44),
        (0.0, 0.44),
    ];
    let column = lib.shape(Mesh::lathe(&column.map(Vec2::from), 32));
    lib.decor(column.at(spot.at + Vec3::Y * 0.18, spot.q, kind::WOOD, wood));
    let hc = 1.5;
    let r = 0.82;
    // Legs from the foot out to the horizon, bowed outward at the knee.
    let leg: Vec<Vec3> = (0..=12)
        .map(|k| {
            let s = k as f32 / 12.0;
            let out = 0.51 + 0.42 * s + 0.07 * libm::sinf(s * PI);
            Vec3::new(out, 0.18 + (hc - 0.21) * s, 0.0)
        })
        .collect();
    let leg = lib.shape(Mesh::sweep(&leg, &[0.045, 0.03, 0.035], &circle(1.0, 10), true));
    for k in 0..4 {
        let turn = Quat::from_rotation_y(-(FRAC_PI_4 + k as f32 * FRAC_PI_2));
        lib.decor(leg.at(spot.at, spot.q * turn, kind::WOOD, wood));
    }
    let centre = spot.at + Vec3::Y * hc;
    let horizon = lib.shape(Mesh::ring(&circle(1.045, 64), &circle(0.915, 64), 0.035));
    let flat = Quat::from_rotation_x(FRAC_PI_2);
    lib.decor(horizon.inst(centre, spot.q * flat, kind::METAL, BRASS_RGBA));
    // The meridian stands round the axis, tilted as the earth's is.
    let tilt = spot.q * Quat::from_rotation_z(0.41);
    let meridian = lib.shape(Mesh::ring(&circle(0.975, 64), &circle(0.925, 64), 0.06));
    lib.decor(meridian.inst(centre, tilt, kind::METAL, BRASS_RGBA));
    let rod = lib.shape(Mesh::cylinder(12));
    let rod_between = |p: Vec3, q: Vec3, d: f32| {
        let v = q - p;
        rod.sized(Vec3::new(d, v.length(), d)).inst(
            (p + q) * 0.5,
            Quat::from_rotation_arc(Vec3::Y, v.normalize()),
            kind::METAL,
            BRASS_RGBA,
        )
    };
    lib.decor(rod_between(
        spot.at + Vec3::Y * 0.6,
        centre + tilt * Vec3::new(0.0, -0.96, 0.0),
        0.05,
    ));
    for p in [-1.0f32, 1.0] {
        lib.decor(rod_between(
            centre + tilt * (Vec3::Y * p * (r - 0.02)),
            centre + tilt * (Vec3::Y * p * 0.95),
            0.045,
        ));
    }
    // The ball and what stands on it, turning once in two minutes.
    let axis = tilt * Vec3::Y;
    let m = lib.mover(
        centre,
        Motion::Spin {
            axis,
            rate: TAU / 120.0,
        },
        0.0,
        None,
    );
    let ball = lib.shape(Mesh::sphere(64));
    lib.moving(
        m,
        ball.sized(Vec3::splat(2.0 * r))
            .inst(centre, tilt, kind::CRYSTAL, [0.015, 0.07, 0.13, 0.45]),
    );
    let (lands, ice) = lands_of_the_globe(r);
    let lands = lib.shape(lands);
    lib.moving(m, lands.at(centre, tilt, kind::CRYSTAL, [0.34, 0.31, 0.17, 0.15]));
    let ice = lib.shape(ice);
    lib.moving(m, ice.at(centre, tilt, kind::CRYSTAL, [0.62, 0.66, 0.66, 0.2]));
}

/// The globe's lands and its ice caps, on a ball of radius `r` about the origin with
/// its axis up y: patches of its surface with ragged coasts, standing a few
/// millimetres proud of the sea (each land a hair higher than the last, so none fight
/// where they meet).
fn lands_of_the_globe(r: f32) -> (Mesh, Mesh) {
    // Latitude, longitude, and how far a land reaches (radians).
    const LANDS: [(f32, f32, f32); 7] = [
        (0.75, 0.2, 0.42),
        (0.05, 0.75, 0.36),
        (-0.55, 0.45, 0.28),
        (0.55, 2.3, 0.55),
        (-0.35, 2.95, 0.3),
        (0.3, 4.3, 0.34),
        (-0.6, 5.1, 0.3),
    ];
    let (mut lands, mut ice) = (Mesh::default(), Mesh::default());
    for (k, &(lat, lon, size)) in LANDS.iter().enumerate() {
        let c = Vec3::new(
            libm::cosf(lat) * libm::cosf(lon),
            libm::sinf(lat),
            libm::cosf(lat) * libm::sinf(lon),
        );
        let coast = ragged(0x476c_6f62 ^ k as u64, size, 0.3);
        patch(&mut lands, r, 0.005 + 0.001 * k as f32, c, coast);
    }
    for (k, pole) in [Vec3::Y, -Vec3::Y].into_iter().enumerate() {
        let coast = ragged(0x4963_6563 ^ k as u64, 0.3, 0.2);
        patch(&mut ice, r, 0.004, pole, coast);
    }
    (lands, ice)
}

/// A ragged outline round a point: how far it reaches at each bearing, about `size`
/// and varying by up to about `rough` of it, from seed `seed`.
fn ragged(seed: u64, size: f32, rough: f32) -> impl Fn(f32) -> f32 {
    let mut r = StdRng::seed_from_u64(seed);
    let waves = [(2.0f32, 1.0f32), (3.0, 0.7), (5.0, 0.4), (8.0, 0.2)]
        .map(|(n, amp)| (n, amp * r.gen_range(0.5..1.0f32), r.gen_range(0.0..TAU)));
    move |b: f32| {
        let w: f32 = waves.iter().map(|&(n, amp, ph)| amp * libm::sinf(n * b + ph)).sum();
        size * (1.0 + rough * w / 1.5)
    }
}

/// A patch of a ball of radius `r` round the direction `centre`, reaching `reach(bearing)`
/// radians from it, standing `rise` proud of the ball, with a little cliff round its
/// edge; added to `m`.
fn patch(m: &mut Mesh, r: f32, rise: f32, centre: Vec3, reach: impl Fn(f32) -> f32) {
    let (spokes, rings) = (64usize, 6usize);
    let e1 = centre.any_orthonormal_vector();
    let e2 = centre.cross(e1);
    let dir = |b: f32, d: f32| {
        let t = e1 * libm::cosf(b) + e2 * libm::sinf(b);
        centre * libm::cosf(d) + t * libm::sinf(d)
    };
    let top = r + rise;
    let at = |m: &mut Mesh, p: Vec3, n: Vec3| {
        m.verts.push(Vertex {
            pos: p.into(),
            normal: n.into(),
        });
        m.verts.len() as u32 - 1
    };
    let hub = at(m, centre * top, centre);
    let mut rows = Vec::new();
    for j in 1..=rings {
        let row: Vec<u32> = (0..spokes)
            .map(|i| {
                let b = i as f32 / spokes as f32 * TAU;
                let n = dir(b, reach(b) * j as f32 / rings as f32);
                at(m, n * top, n)
            })
            .collect();
        rows.push(row);
    }
    for i in 0..spokes {
        let i1 = (i + 1) % spokes;
        facet(m, [hub, rows[0][i], rows[0][i1]]);
        for j in 1..rings {
            let (a, b) = (&rows[j - 1], &rows[j]);
            facet(m, [a[i], b[i], b[i1]]);
            facet(m, [a[i], b[i1], a[i1]]);
        }
    }
    // The cliff: from the coast down under the sea's surface, facing outward.
    let coast: Vec<(u32, u32)> = (0..spokes)
        .map(|i| {
            let b = i as f32 / spokes as f32 * TAU;
            let n = dir(b, reach(b));
            let out = (n * n.dot(centre) - centre).normalize();
            (at(m, n * top, out), at(m, n * (r - 0.003), out))
        })
        .collect();
    for i in 0..spokes {
        let ((a, b), (c, d)) = (coast[i], coast[(i + 1) % spokes]);
        facet(m, [a, b, d]);
        facet(m, [a, d, c]);
    }
}

/// A triangle of `m`'s, wound to face the way its corners' normals do.
fn facet(m: &mut Mesh, [a, b, c]: [u32; 3]) {
    let p = |i: u32| Vec3::from(m.verts[i as usize].pos);
    let n = |i: u32| Vec3::from(m.verts[i as usize].normal);
    let face = (p(b) - p(a)).cross(p(c) - p(a));
    if face.dot(n(a) + n(b) + n(c)) >= 0.0 {
        m.idx.extend_from_slice(&[a, b, c]);
    } else {
        m.idx.extend_from_slice(&[a, c, b]);
    }
}

/// `n` points round a circle of radius `r` in xy, anticlockwise.
fn circle(r: f32, n: usize) -> Vec<Vec2> {
    (0..n)
        .map(|k| Vec2::from_angle(k as f32 / n as f32 * TAU) * r)
        .collect()
}

/// The long map table, with a chart spread on it, weights, dividers, rolled charts
/// and a lamp.
fn table(lib: &mut Library, f: &RoomFrame) {
    let (ta, to) = TABLE;
    let (hw, hd, h) = (2.3f32, 0.65f32, 0.92f32);
    guard(lib, f, (ta - hw, ta + hw), (to - hd, to + hd), h);
    slab(
        lib,
        f,
        (ta - hw, ta + hw),
        (to - hd, to + hd),
        (h - 0.06, h),
        kind::WOOD,
        WALNUT,
    );
    slab(
        lib,
        f,
        (ta - hw + 0.12, ta + hw - 0.12),
        (to - hd + 0.1, to + hd - 0.1),
        (h - 0.22, h - 0.06),
        kind::WOOD,
        DARK_WOOD,
    );
    let leg = [
        (0.0, 0.0),
        (0.05, 0.0),
        (0.05, 0.03),
        (0.036, 0.05),
        (0.03, 0.12),
        (0.056, 0.26),
        (0.05, 0.34),
        (0.032, 0.42),
        (0.03, 0.56),
        (0.046, 0.6),
        (0.058, 0.64),
        (0.058, h - 0.22),
        (0.0, h - 0.22),
    ];
    let leg = lib.shape(Mesh::lathe(&leg.map(Vec2::from), 24));
    for x in [-1.0f32, 1.0] {
        for z in [-1.0f32, 1.0] {
            let (a, o) = (ta + x * (hw - 0.2), to + z * (hd - 0.15));
            lib.decor(leg.at(f.at(a, o, 0.0), Quat::IDENTITY, kind::WOOD, rgba(WALNUT, 1.0)));
        }
        let a = ta + x * (hw - 0.2);
        slab(
            lib,
            f,
            (a - 0.04, a + 0.04),
            (to - hd + 0.15, to + hd - 0.15),
            (0.14, 0.22),
            kind::WOOD,
            DARK_WOOD,
        );
    }
    slab(
        lib,
        f,
        (ta - hw + 0.2, ta + hw - 0.2),
        (to - 0.04, to + 0.04),
        (0.14, 0.2),
        kind::WOOD,
        DARK_WOOD,
    );
    // The chart, a little askew, with its coasts and a route.
    let chart = Spot {
        at: f.at(ta + 0.1, to, h),
        q: f.turn() * Quat::from_rotation_y(0.035),
    };
    let mut sheet = Inst::new(
        chart.at + Vec3::Y * 0.004,
        chart.q,
        Vec3::new(3.7, 0.008, 1.05),
        kind::PARCHMENT,
        [0.8, 0.7, 0.5, 1.0],
        0,
    );
    sheet.param = 1;
    lib.decor(sheet);
    let ink = rgba(SEPIA, 1.0);
    let coast = [
        (-1.75, 0.3),
        (-1.3, 0.12),
        (-0.9, 0.28),
        (-0.5, 0.05),
        (-0.1, 0.18),
        (0.3, -0.12),
        (0.7, 0.06),
        (1.1, -0.2),
        (1.5, -0.08),
        (1.78, -0.3),
    ];
    for w in coast.windows(2) {
        flat_stroke(lib, &chart, Vec2::from(w[0]), Vec2::from(w[1]), 0.0095, ink);
    }
    let isle = [(0.5, 0.3), (0.85, 0.36), (1.0, 0.22), (0.75, 0.15), (0.5, 0.3)];
    for w in isle.windows(2) {
        flat_stroke(lib, &chart, Vec2::from(w[0]), Vec2::from(w[1]), 0.0095, ink);
    }
    for i in 0..7 {
        let p = Vec2::new(-1.4, -0.25).lerp(Vec2::new(0.6, 0.27), i as f32 / 7.0);
        let q = p + (Vec2::new(0.6, 0.27) - Vec2::new(-1.4, -0.25)) * 0.07;
        flat_stroke(lib, &chart, p, q, 0.0098, [0.55, 0.1, 0.06, 1.0]);
    }
    // Lead weights on its corners.
    let weight = [
        (0.0, 0.0),
        (0.045, 0.0),
        (0.045, 0.045),
        (0.035, 0.055),
        (0.012, 0.06),
        (0.012, 0.075),
        (0.0, 0.08),
    ];
    let weight = lib.shape(Mesh::lathe(&weight.map(Vec2::from), 16));
    for (x, z) in [(-1.8f32, -0.47f32), (1.8, -0.47), (-1.8, 0.47), (1.8, 0.47)] {
        lib.decor(weight.at(
            chart.at + chart.q * Vec3::new(x, 0.008, z),
            chart.q,
            kind::METAL,
            [0.35, 0.33, 0.3, 1.0],
        ));
    }
    // Dividers lying open on the chart.
    for e in [-1.0f32, 1.0] {
        chart.part(
            lib,
            Vec3::new(-0.6 + e * 0.05, 0.014, -0.2),
            Vec3::new(0.012, 0.01, 0.34),
            Quat::from_rotation_y(e * 0.16),
            kind::METAL,
            BRASS_RGBA,
        );
    }
    // Rolled charts at the far end.
    let roll = lib.shape(Mesh::cylinder(16));
    for i in 0..2 {
        let c = f.at(ta - hw + 0.45 + i as f32 * 0.11, to, h + 0.04);
        lib.decor(roll.sized(Vec3::new(0.08, 0.9, 0.08)).inst(
            c,
            f.turn() * Quat::from_rotation_x(FRAC_PI_2) * Quat::from_rotation_y(i as f32 * 2.0),
            kind::PARCHMENT,
            [0.74, 0.62, 0.42, 1.0],
        ));
    }
    // A brass oil lamp at the near end, its flame in a glass chimney.
    let lamp = Spot::new(f, ta + hw - 0.35, to - 0.2, 0.0);
    let base = lamp.at + Vec3::Y * h;
    let font = [
        (0.0, 0.0),
        (0.08, 0.0),
        (0.08, 0.012),
        (0.05, 0.03),
        (0.035, 0.07),
        (0.06, 0.1),
        (0.065, 0.13),
        (0.05, 0.155),
        (0.038, 0.16),
        (0.038, 0.175),
        (0.0, 0.175),
    ];
    let font = lib.shape(Mesh::lathe(&font.map(Vec2::from), 32));
    lib.decor(font.at(base, Quat::IDENTITY, kind::METAL, BRASS_RGBA));
    let chimney = [(0.03, 0.0), (0.05, 0.04), (0.048, 0.09), (0.026, 0.15), (0.025, 0.22)];
    let chimney = lib.shape(Mesh::lathe(&chimney.map(Vec2::from), 32).two_sided());
    lib.translucent(chimney.at(
        base + Vec3::Y * 0.172,
        Quat::IDENTITY,
        kind::CLEAR,
        [0.95, 0.92, 0.85, 0.08],
    ));
    flame(lib, base + Vec3::Y * 0.18, 0.06);
    lib.light(
        lamp.at + Vec3::Y * (h + 0.45),
        5.5,
        [1.0, 0.62, 0.3],
        0.8,
        LightKind::Flicker,
    );
}

/// An inked line on the table's chart, from `p` to `q` (across, toward the door).
fn flat_stroke(lib: &mut Library, chart: &Spot, p: Vec2, q: Vec2, y: f32, ink: [f32; 4]) {
    let d = q - p;
    let m = (p + q) * 0.5;
    chart.part(
        lib,
        Vec3::new(m.x, y, m.y),
        Vec3::new(d.length() + 0.012, 0.003, 0.014),
        Quat::from_rotation_y(-libm::atan2f(d.y, d.x)),
        kind::PLAIN,
        ink,
    );
}

/// A model ship sailing the air over the map table, heeled a little in a wind no one
/// else feels, rising and falling and rocking as if on a slow swell, its pennant
/// streaming: a round hull, two masts with their yards and bellied sails, a bowsprit
/// and stays, and a lantern at the stern.
fn ship(lib: &mut Library, f: &RoomFrame) {
    let s = Spot {
        at: f.at(TABLE.0 + 0.7, TABLE.1 - 0.1, 2.55),
        q: f.turn() * Quat::from_rotation_y(0.5) * Quat::from_rotation_x(0.1),
    };
    // In the ship's frame: x toward the bow, y up, z abeam.
    let p = |v: Vec3| s.at + s.q * v;
    let hull_rgba = rgba([0.22, 0.11, 0.05], 1.0);
    let canvas = [0.88, 0.84, 0.74, 1.0];
    let rope = [0.2, 0.15, 0.09, 1.0];
    let swell = Motion::Bob {
        amp: 0.05,
        period: 8.0,
        rock: 0.06,
    };
    let m = lib.mover(s.at, swell, 0.0, None);
    let (stern, bow) = (-0.42f32, 0.44f32);
    let deck = |x: f32| {
        let t = (x - stern) / (bow - stern) * 2.0 - 1.0;
        0.07 + 0.05 * t * t
    };
    let body = hull(lib, stern, bow, deck);
    lib.moving(m, body.at(s.at, s.q, kind::WOOD, hull_rgba));
    lib.moving(
        m,
        Inst::new(
            p(Vec3::new(-0.32, deck(-0.32) + 0.04, 0.0)),
            s.q,
            Vec3::new(0.18, 0.08, 0.19),
            kind::WOOD,
            hull_rgba,
            0,
        ),
    );
    let spar = lib.shape(Mesh::cylinder(10));
    let spar_between = |a: Vec3, b: Vec3, d: f32, color: [f32; 4]| {
        let v = b - a;
        spar.sized(Vec3::new(d, v.length(), d)).inst(
            p((a + b) * 0.5),
            s.q * Quat::from_rotation_arc(Vec3::Y, v.normalize()),
            kind::WOOD,
            color,
        )
    };
    let bowsprit = Vec3::new(0.8, 0.24, 0.0);
    lib.moving(
        m,
        spar_between(Vec3::new(0.36, deck(0.36), 0.0), bowsprit, 0.018, hull_rgba),
    );
    let mut tops = Vec::new();
    for (x, tall) in [(0.18f32, 0.75f32), (-0.15, 0.62)] {
        let foot = Vec3::new(x, deck(x), 0.0);
        let top = foot + Vec3::Y * tall;
        tops.push(top);
        lib.moving(m, spar_between(foot - Vec3::Y * 0.02, top, 0.022, hull_rgba));
        for (y, w) in [(0.32f32, 0.36f32), (0.58, 0.28)] {
            if y > tall {
                continue;
            }
            let yard = foot + Vec3::Y * (0.1 + y * tall / 0.75);
            // Braced round a little, as the sails are.
            let brace = Quat::from_rotation_y(0.25);
            let half = brace * Vec3::new(0.0, 0.0, w * 0.5 + 0.02);
            lib.moving(m, spar_between(yard - half, yard + half, 0.012, hull_rgba));
            let sail = lib.shape(bellied(w, 0.2));
            lib.moving(
                m,
                sail.at(
                    p(yard + Vec3::new(0.012, -0.1, 0.0)),
                    s.q * brace * Quat::from_mat3(&Mat3::from_cols(Vec3::Z, Vec3::X, Vec3::Y)),
                    kind::CLOTH,
                    canvas,
                ),
            );
        }
    }
    // Stays from the mastheads to the bowsprit and the stern.
    for (a, b) in [
        (tops[0], bowsprit),
        (tops[1], tops[0] - Vec3::Y * 0.2),
        (tops[1], Vec3::new(stern + 0.02, deck(stern) + 0.02, 0.0)),
    ] {
        lib.moving(m, spar_between(a, b, 0.006, rope));
    }
    let lantern = lib.shape(Mesh::sphere(12));
    lib.moving(
        m,
        lantern.sized(Vec3::new(0.04, 0.05, 0.04)).inst(
            p(Vec3::new(stern - 0.01, deck(stern) + 0.07, 0.0)),
            s.q,
            kind::EMISSIVE,
            [1.0, 0.65, 0.3, 6.0],
        ),
    );
    // The pennant at the foremast's head, streaming forward and stirring.
    let head = tops[0];
    let stream = Motion::Swing {
        axis: s.q * Vec3::Y,
        amp: 0.2,
        period: 4.5,
    };
    let pennant = lib.mover(p(head), stream, 1.3, Some(m));
    lib.moving(
        pennant,
        Inst::new(
            p(head + Vec3::new(0.08, -0.02, 0.0)),
            s.q,
            Vec3::new(0.15, 0.035, 0.006),
            kind::CLOTH,
            [0.6, 0.08, 0.06, 1.0],
            0,
        ),
    );
}

/// The ship's hull from `stern` to `bow` along x, its deck at `deck(x)`: a round
/// bottom swept along the deck, full aft and fining to a stem at the bow (the swept
/// section keeps its deck on the path, so the bottom rises as it narrows).
fn hull(lib: &mut Library, stern: f32, bow: f32, deck: impl Fn(f32) -> f32) -> Shape {
    let n = 16;
    let xs: Vec<f32> = (0..n)
        .map(|k| stern + (bow - stern) * k as f32 / (n - 1) as f32)
        .collect();
    // The first step level, so the section starts (and stays) upright.
    let path: Vec<Vec3> = (xs.iter().enumerate())
        .map(|(k, &x)| Vec3::new(x, deck(xs[k.max(1)]), 0.0))
        .collect();
    let scale: Vec<f32> = (xs.iter())
        .map(|&x| {
            let t = (x - stern) / (bow - stern);
            if t < 0.4 {
                0.85 + 0.15 * t / 0.4
            } else {
                1.0 - 0.9 * libm::powf((t - 0.4) / 0.6, 1.6)
            }
        })
        .collect();
    // Across the sweep: x is up, y abeam. The deck closes the half-round section.
    let section: Vec<Vec2> = (0..=12)
        .map(|k| {
            let a = k as f32 / 12.0 * PI;
            Vec2::new(-0.15 * libm::sinf(a), 0.12 * libm::cosf(a))
        })
        .collect();
    lib.shape(Mesh::sweep(&path, &scale, &section, true))
}

/// A sail `w` wide and `h` tall bellied out by the wind: a crescent across it (x),
/// bulging toward +y, made `h` deep along z.
fn bellied(w: f32, h: f32) -> Mesh {
    let (belly, t) = (0.11 * w, 0.006);
    let n = 10;
    let bulge = |k: usize| {
        let x = -w * 0.5 + w * k as f32 / n as f32;
        Vec2::new(x, belly * (1.0 - (2.0 * x / w).powi(2)))
    };
    let mut outline: Vec<Vec2> = (0..=n).map(bulge).collect();
    outline.extend((0..=n).rev().map(|k| bulge(k) - Vec2::Y * t));
    Mesh::prism(&outline, h)
}

/// A candle's or lamp's flame standing `h` tall on `at`.
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
