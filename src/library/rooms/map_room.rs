//! The Map Room: department 4, first floor, north side. An open chart floor down the
//! middle, a great globe over a compass rose set in the floor and a long map table
//! under a little ship sailing the air; the books stand in alcoves off both side walls
//! (racks of rolled maps on the backs of the first ones) and along the far and near
//! walls, with maps hung over them.

use super::*;
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

/// The room's frame as a rotation: local x along `a`, y up, z along -`o` (toward the
/// door's wall).
fn turn(f: &RoomFrame) -> Quat {
    Quat::from_mat3(&Mat3::from_cols(f.across(), Vec3::Y, -f.out()))
}

/// Drawn only: the box over `a`, `o`, and heights `y` above the floor.
fn slab(lib: &mut Library, f: &RoomFrame, a: (f32, f32), o: (f32, f32), y: (f32, f32), kind: u32, rgb: [f32; 3]) {
    let (lo, hi) = f.boxed(a.0, a.1, o.0, o.1, y.0, y.1);
    lib.decor(Inst::minmax(lo, hi, kind, rgb));
}

/// In the way of everything: the floor box over `a`, `o`, `h` high.
fn guard(lib: &mut Library, f: &RoomFrame, a: (f32, f32), o: (f32, f32), h: f32) {
    let (lo, hi) = f.boxed(a.0, a.1, o.0, o.1, 0.0, h);
    lib.collide(lo, hi, Stops::ALL);
}

/// A piece of furniture's place: its origin on the floor and its turn. Its parts are
/// given in its own frame (x across the room, y up, z toward the door's wall, unless
/// it is turned).
struct Spot {
    at: Vec3,
    q: Quat,
}

impl Spot {
    fn new(f: &RoomFrame, a: f32, o: f32, yaw: f32) -> Spot {
        Spot {
            at: f.at(a, o, 0.0),
            q: turn(f) * Quat::from_rotation_y(yaw),
        }
    }

    fn part(&self, lib: &mut Library, p: Vec3, size: Vec3, q: Quat, kind: u32, color: [f32; 4]) {
        lib.decor(Inst::new(self.at + self.q * p, self.q * q, size, kind, color, 0));
    }
}

fn rgba(rgb: [f32; 3], w: f32) -> [f32; 4] {
    [rgb[0], rgb[1], rgb[2], w]
}

/// Faces left to right as you face them (all facing the same way).
fn lr(mut faces: Vec<(Vec3, Vec3)>) -> Vec<(Vec3, Vec3)> {
    let right = Vec3::Y.cross(faces[0].1);
    faces.sort_by(|p, q| p.0.dot(right).total_cmp(&q.0.dot(right)));
    faces
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
                [lr(first), vec![back], lr(last)].concat()
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
        lr(far[2..7].to_vec()),
        right[2].clone(),
        right[1].clone(),
        right[0].clone(),
        lr(near),
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
/// wall's face, facing into the room): a coast or two, lines of latitude, a route.
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
    at.part(
        lib,
        Vec3::new(0.0, y, 0.025),
        Vec3::new(w, h, 0.02),
        Quat::IDENTITY,
        kind::PARCHMENT,
        [0.78, 0.68, 0.48, 1.0],
    );
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
                Vec2::new(cx + t.cos() * rad.x * k, cy + t.sin() * rad.y * k)
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
        Quat::from_rotation_z(d.y.atan2(d.x)),
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
    // Each pigeonhole a little pyramid of rolls lying end on.
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
                    lib.decor(Inst::new(
                        c,
                        turn(f) * Quat::from_rotation_z(FRAC_PI_4 + r.gen_range(-0.2..0.2)),
                        Vec3::new(d * 0.72, d * 0.72, len),
                        kind::PARCHMENT,
                        rgba(tints[r.gen_range(0..tints.len())], 1.0),
                        0,
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
        q: turn(f) * Quat::from_rotation_y(0.06),
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
    for i in 0..3 {
        let c = f.at(
            s * (8.0 + i as f32 * 0.03),
            o0 + 0.12 + i as f32 * 0.1,
            top + 0.045 + (i == 2) as u8 as f32 * 0.08,
        );
        lib.decor(Inst::new(
            c,
            turn(f) * Quat::from_rotation_y(FRAC_PI_2) * Quat::from_rotation_z(FRAC_PI_4),
            Vec3::new(0.075, 0.075, 0.95 - i as f32 * 0.1),
            kind::PARCHMENT,
            [0.66, 0.56, 0.4, 1.0],
            0,
        ));
    }
}

/// Brass lines set in the floor round the globe: two rings and the eight points of
/// the winds.
fn compass_rose(lib: &mut Library, f: &RoomFrame) {
    let spot = Spot::new(f, GLOBE.0, GLOBE.1, 0.0);
    for (r, n, y) in [(2.75f32, 28, 0.004f32), (2.5, 26, 0.005)] {
        let len = TAU * r / n as f32 + 0.02;
        for k in 0..n {
            let t = k as f32 / n as f32 * TAU;
            spot.part(
                lib,
                Vec3::new(t.cos() * r, y * 0.5, t.sin() * r),
                Vec3::new(0.05, y, len),
                Quat::from_rotation_y(-t),
                kind::METAL,
                BRASS_RGBA,
            );
        }
    }
    for k in 0..8 {
        let t = k as f32 / 8.0 * TAU;
        let main = k % 2 == 0;
        let (len, half) = if main { (3.3f32, 0.34f32) } else { (2.3, 0.22) };
        let dir = Vec2::new(t.cos(), t.sin());
        let side = Vec2::new(-dir.y, dir.x);
        let tip = dir * len;
        let foot = dir * 0.95;
        for e in [-1.0f32, 1.0] {
            let base = foot + side * half * e;
            line_on_floor(lib, &spot, base, tip, 0.035, 0.007);
        }
        if main {
            line_on_floor(lib, &spot, foot, tip, 0.02, 0.006);
        }
    }
}

fn line_on_floor(lib: &mut Library, spot: &Spot, p: Vec2, q: Vec2, w: f32, h: f32) {
    let d = q - p;
    let m = (p + q) * 0.5;
    spot.part(
        lib,
        Vec3::new(m.x, h * 0.5, m.y),
        Vec3::new(d.length(), h, w),
        Quat::from_rotation_y(-d.y.atan2(d.x)),
        kind::METAL,
        BRASS_RGBA,
    );
}

/// The great globe on its stand over the compass rose: seas that glow faintly from
/// within, a few lands, a brass horizon and meridian.
fn globe(lib: &mut Library, f: &RoomFrame) {
    let spot = Spot::new(f, GLOBE.0, GLOBE.1, 0.0);
    let (gx, go) = GLOBE;
    guard(lib, f, (gx - 1.05, gx + 1.05), (go - 1.05, go + 1.05), 2.45);
    let dark = rgba([0.09, 0.045, 0.02], 1.0);
    let wood = rgba(WALNUT, 1.0);
    spot.part(
        lib,
        Vec3::Y * 0.05,
        Vec3::new(1.3, 0.1, 1.3),
        Quat::IDENTITY,
        kind::WOOD,
        dark,
    );
    spot.part(
        lib,
        Vec3::Y * 0.14,
        Vec3::new(0.9, 0.08, 0.9),
        Quat::IDENTITY,
        kind::WOOD,
        wood,
    );
    spot.part(
        lib,
        Vec3::Y * 0.4,
        Vec3::new(0.16, 0.45, 0.16),
        Quat::IDENTITY,
        kind::WOOD,
        wood,
    );
    let hc = 1.5;
    let r = 0.82;
    // Legs from the plinth out to the horizon ring.
    for (x, z) in [(-1.0f32, -1.0f32), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)] {
        let foot = spot.at + spot.q * Vec3::new(x * 0.36, 0.18, z * 0.36);
        let head = spot.at + spot.q * Vec3::new(x * 0.66, hc - 0.03, z * 0.66);
        lib.beam(foot, head, 0.07, 0.07, WALNUT);
    }
    // The horizon ring, and the meridian ring standing round the tilted axis.
    let n = 20;
    for k in 0..n {
        let t = k as f32 / n as f32 * TAU;
        let rr = 0.98;
        spot.part(
            lib,
            Vec3::new(t.cos() * rr, hc, t.sin() * rr),
            Vec3::new(0.13, 0.035, TAU * rr / n as f32 + 0.01),
            Quat::from_rotation_y(-t),
            kind::METAL,
            BRASS_RGBA,
        );
    }
    let tilt = Quat::from_rotation_z(0.41);
    let centre = Vec3::Y * hc;
    let n = 22;
    for k in 0..n {
        let t = k as f32 / n as f32 * TAU;
        let rr = r + 0.13;
        spot.part(
            lib,
            centre + tilt * Vec3::new(t.cos() * rr, t.sin() * rr, 0.0),
            Vec3::new(0.045, TAU * rr / n as f32 + 0.01, 0.06),
            tilt * Quat::from_rotation_z(t),
            kind::METAL,
            BRASS_RGBA,
        );
    }
    let foot = tilt * Vec3::new(0.0, -(r + 0.13), 0.0) + centre;
    lib.beam(
        spot.at + spot.q * (Vec3::Y * 0.62),
        spot.at + spot.q * foot,
        0.05,
        0.05,
        BRASS,
    );
    // The ball: slices along its axis, each six turned squares whose corners round it
    // out. Here and there one square is land, showing where its corners stand proud
    // (a hair thinner, so its top stays under the sea's); the slices at the poles are
    // ice.
    let sea = [0.03, 0.13, 0.2, 1.1];
    let land = [0.34, 0.31, 0.17, 0.15];
    let ice = [0.62, 0.66, 0.66, 0.2];
    let slices = 12;
    let t = 2.0 * r / slices as f32;
    let lands: [u8; 12] = [
        0, 0b000100, 0b100100, 0b000110, 0b010010, 0b110000, 0b011000, 0b001001, 0b000011, 0b000010, 0b100000, 0,
    ];
    for j in 0..slices {
        let h = -r + (j as f32 + 0.5) * t;
        let rad = (r * r - h * h).max(0.0).sqrt() * 1.03;
        let polar = j == 0 || j == slices - 1;
        for k in 0..6 {
            let is_land = lands[j] & (1 << k) != 0;
            let (color, thick) = match (polar, is_land) {
                (true, _) => (ice, t + 0.002),
                (false, true) => (land, t - 0.004),
                (false, false) => (sea, t + 0.002),
            };
            spot.part(
                lib,
                centre + tilt * Vec3::new(0.0, h, 0.0),
                Vec3::new(rad * 1.457, thick, rad * 1.457),
                tilt * Quat::from_rotation_y(k as f32 * PI / 12.0),
                kind::CRYSTAL,
                color,
            );
        }
    }
    for p in [-1.0f32, 1.0] {
        spot.part(
            lib,
            centre + tilt * Vec3::new(0.0, p * (r + 0.07), 0.0),
            Vec3::new(0.06, 0.16, 0.06),
            tilt,
            kind::METAL,
            BRASS_RGBA,
        );
    }
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
    for x in [-1.0f32, 1.0] {
        for z in [-1.0f32, 1.0] {
            let (a, o) = (ta + x * (hw - 0.2), to + z * (hd - 0.15));
            slab(
                lib,
                f,
                (a - 0.06, a + 0.06),
                (o - 0.06, o + 0.06),
                (0.0, h - 0.22),
                kind::WOOD,
                WALNUT,
            );
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
        q: turn(f) * Quat::from_rotation_y(0.035),
    };
    chart.part(
        lib,
        Vec3::Y * 0.004,
        Vec3::new(3.7, 0.008, 1.05),
        Quat::IDENTITY,
        kind::PARCHMENT,
        [0.8, 0.7, 0.5, 1.0],
    );
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
    for (x, z) in [(-1.8f32, -0.47f32), (1.8, -0.47), (-1.8, 0.47), (1.8, 0.47)] {
        chart.part(
            lib,
            Vec3::new(x, 0.035, z),
            Vec3::new(0.09, 0.06, 0.09),
            Quat::IDENTITY,
            kind::METAL,
            [0.35, 0.33, 0.3, 1.0],
        );
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
    for i in 0..2 {
        let c = f.at(ta - hw + 0.45 + i as f32 * 0.11, to, h + 0.04);
        lib.decor(Inst::new(
            c,
            turn(f) * Quat::from_rotation_z(FRAC_PI_4 + i as f32 * 0.3),
            Vec3::new(0.075, 0.075, 0.9),
            kind::PARCHMENT,
            [0.74, 0.62, 0.42, 1.0],
            0,
        ));
    }
    // A brass lamp at the near end.
    let lamp = Spot::new(f, ta + hw - 0.35, to - 0.2, 0.0);
    let base = Vec3::Y * h;
    lamp.part(
        lib,
        base + Vec3::Y * 0.015,
        Vec3::new(0.16, 0.03, 0.16),
        Quat::IDENTITY,
        kind::METAL,
        BRASS_RGBA,
    );
    lamp.part(
        lib,
        base + Vec3::Y * 0.09,
        Vec3::new(0.09, 0.12, 0.09),
        Quat::IDENTITY,
        kind::METAL,
        BRASS_RGBA,
    );
    lamp.part(
        lib,
        base + Vec3::Y * 0.2,
        Vec3::new(0.035, 0.07, 0.035),
        Quat::IDENTITY,
        kind::EMISSIVE,
        [1.0, 0.6, 0.22, 10.0],
    );
    lamp.part(
        lib,
        base + Vec3::Y * 0.3,
        Vec3::new(0.11, 0.015, 0.11),
        Quat::IDENTITY,
        kind::METAL,
        BRASS_RGBA,
    );
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
        Quat::from_rotation_y(-d.y.atan2(d.x)),
        kind::PLAIN,
        ink,
    );
}

/// A model ship sailing the air over the map table, heeled a little in a wind no one
/// else feels.
fn ship(lib: &mut Library, f: &RoomFrame) {
    let s = Spot {
        at: f.at(TABLE.0 + 0.7, TABLE.1 - 0.1, 2.55),
        q: turn(f) * Quat::from_rotation_y(0.5) * Quat::from_rotation_x(0.1),
    };
    let hull = rgba([0.22, 0.11, 0.05], 1.0);
    let sail = [0.88, 0.84, 0.74, 1.0];
    s.part(
        lib,
        Vec3::ZERO,
        Vec3::new(0.9, 0.16, 0.24),
        Quat::IDENTITY,
        kind::WOOD,
        hull,
    );
    s.part(
        lib,
        Vec3::new(0.45, 0.0, 0.0),
        Vec3::new(0.17, 0.16, 0.17),
        Quat::from_rotation_y(FRAC_PI_4),
        kind::WOOD,
        hull,
    );
    s.part(
        lib,
        Vec3::new(-0.36, 0.13, 0.0),
        Vec3::new(0.22, 0.1, 0.25),
        Quat::IDENTITY,
        kind::WOOD,
        hull,
    );
    s.part(
        lib,
        Vec3::new(0.0, -0.11, 0.0),
        Vec3::new(0.8, 0.06, 0.05),
        Quat::IDENTITY,
        kind::WOOD,
        rgba(DARK_WOOD, 1.0),
    );
    s.part(
        lib,
        Vec3::new(0.62, 0.12, 0.0),
        Vec3::new(0.4, 0.02, 0.02),
        Quat::from_rotation_z(0.35),
        kind::WOOD,
        hull,
    );
    for (x, tall) in [(0.18f32, 0.75f32), (-0.15, 0.62)] {
        s.part(
            lib,
            Vec3::new(x, 0.08 + tall * 0.5, 0.0),
            Vec3::new(0.025, tall, 0.025),
            Quat::IDENTITY,
            kind::WOOD,
            hull,
        );
        for (y, w) in [(0.32f32, 0.36f32), (0.58, 0.28)] {
            if y > tall {
                continue;
            }
            s.part(
                lib,
                Vec3::new(x + 0.02, 0.08 + y * tall / 0.75, 0.0),
                Vec3::new(0.012, 0.2, w),
                Quat::from_rotation_y(0.25),
                kind::CLOTH,
                sail,
            );
        }
    }
    s.part(
        lib,
        Vec3::new(0.24, 0.86, 0.0),
        Vec3::new(0.14, 0.035, 0.008),
        Quat::IDENTITY,
        kind::CLOTH,
        [0.6, 0.08, 0.06, 1.0],
    );
    s.part(
        lib,
        Vec3::new(-0.49, 0.16, 0.0),
        Vec3::new(0.03, 0.04, 0.03),
        Quat::IDENTITY,
        kind::EMISSIVE,
        [1.0, 0.65, 0.3, 6.0],
    );
}
