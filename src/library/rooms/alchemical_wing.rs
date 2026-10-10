//! The Alchemical Wing: department 1, ground floor, east side. Three alcoves of cases
//! open off each side of a working nave. Down the nave stand two long workbenches
//! crowded with glassware and, beyond them, a brick athanor with its fire alight and
//! copper pipes running from its flue over to the benches; a transmutation circle
//! glows faintly in the floor before it.

use super::*;

/// Clear width of an alcove, between the faces either side of it.
const BAY: f32 = 3.0;
/// How deep a wall case stands from its wall.
const WALL_CASE: f32 = 0.04 + FACE_DEPTH;
/// How far across (|a|) the two cases of an alcove's side stand: at its mouth, by the wall.
const ROW: [f32; 2] = [8.75, 11.45];
/// The benches: how far across (|a|) their middles are, and where they run (o).
const BENCH_A: f32 = 3.6;
const BENCH_O: (f32, f32) = (19.4, 26.2);
/// The athanor's front and back (o), and the circle before it.
const FURNACE_O: (f32, f32) = (25.9, 27.7);
const CIRCLE_O: f32 = 23.0;

const BRICK: [f32; 3] = [0.42, 0.2, 0.13];
const COPPER: [f32; 4] = [0.62, 0.3, 0.14, 1.0];
const IRON: [f32; 4] = [0.07, 0.07, 0.08, 1.0];
const OAK: [f32; 3] = [0.2, 0.11, 0.05];
/// What the flasks on the benches hold.
const POTIONS: [[f32; 3]; 6] = [
    [0.25, 0.85, 0.35],
    [1.0, 0.55, 0.12],
    [0.55, 0.25, 0.95],
    [0.25, 0.55, 1.0],
    [0.95, 0.18, 0.15],
    [0.8, 0.75, 0.3],
];
/// The glass of the jars on the shelves: dark greens, browns and blues, and clay.
const JARS: [[f32; 3]; 6] = [
    [0.16, 0.3, 0.12],
    [0.4, 0.22, 0.08],
    [0.12, 0.17, 0.32],
    [0.32, 0.1, 0.08],
    [0.5, 0.42, 0.3],
    [0.25, 0.28, 0.2],
];

pub(super) fn furnish(lib: &mut Library, frame: &RoomFrame, rng: &mut StdRng, titles: &mut HashSet<String>) {
    shelves(lib, frame, rng, titles);
    walls(lib, frame);
    furnace(lib, frame);
    for s in [-1.0f32, 1.0] {
        bench(lib, frame, s);
    }
    pipes(lib, frame);
    circle(lib, frame);
}

/// A ring of `n` rods round `c` in the plane of `u` and `v` (unit, at right angles).
#[allow(clippy::too_many_arguments)]
fn ring(lib: &mut Library, c: Vec3, u: Vec3, v: Vec3, r: f32, n: usize, w: f32, kind: u32, rgba: [f32; 4]) {
    let at = |k: usize| {
        let t = k as f32 / n as f32 * TAU;
        c + (u * libm::cosf(t) + v * libm::sinf(t)) * r
    };
    for k in 0..n {
        let (p, q) = (at(k), at(k + 1));
        let m = (p - q) * 0.04;
        rod(lib, p + m, q - m, w, w, kind, rgba);
    }
}

/// A vessel standing on `foot`: a round belly `w` across and a narrower shoulder,
/// and a neck `neck` long above them.
fn vessel(lib: &mut Library, foot: Vec3, w: f32, neck: f32, kind: u32, rgba: [f32; 4]) {
    octo(lib, foot + Vec3::Y * (w * 0.35), w, w * 0.7, kind, rgba);
    octo(lib, foot + Vec3::Y * (w * 0.5), w * 0.7, w, kind, rgba);
    if neck > 0.0 {
        let top = foot + Vec3::Y * w;
        octo(lib, top + Vec3::Y * (neck * 0.5), w * 0.22, neck, kind, rgba);
    }
}

/// The alcoves: each side wall has three, walled by short rows of cases running in
/// from it, the last against the far wall and the first single-sided, its back to
/// the door; a category round each, read from its mouth on the left, round the back
/// and out. Then the near wall's cases, and the far wall's either side of the chart.
fn shelves(lib: &mut Library, frame: &RoomFrame, rng: &mut StdRng, titles: &mut HashSet<String>) {
    let out = frame.out();
    let across = !frame.out_along_x();
    let mut cat = frame.categories.start;
    let far_face = RoomFrame::O_FAR - WALL_CASE;
    for s in [-1.0f32, 1.0] {
        let mut alcoves = Vec::new();
        // The faces on the far side of the next alcove in (facing -o).
        let mut beyond: Vec<(Vec3, Vec3)> = (ROW.iter())
            .map(|&x| lib.wall_bookcase(frame.at(s * x, RoomFrame::O_FAR, 0.0), -out))
            .collect();
        for j in 0..3 {
            let near = far_face - j as f32 * (BAY + UNIT_DEPTH) - BAY;
            let mut toward = Vec::new();
            let mut away = Vec::new();
            for &x in &ROW {
                if j < 2 {
                    for face in lib.bookcase(frame.at(s * x, near - UNIT_DEPTH * 0.5, 0.0), across) {
                        if face.1.dot(out) > 0.0 {
                            toward.push(face);
                        } else {
                            away.push(face);
                        }
                    }
                } else {
                    let at = frame.at(s * x, near - WALL_CASE, 0.0);
                    toward.push(lib.wall_bookcase(at, out));
                    backboard(lib, frame, s * x, near - WALL_CASE);
                }
            }
            let back = frame.at(s * RoomFrame::A_HALF, near + BAY * 0.5, 0.0);
            let back = lib.wall_bookcase(back, -frame.across() * s);
            let (first, last) = if s < 0.0 { (toward, beyond) } else { (beyond, toward) };
            let mut faces = left_to_right(first);
            faces.push(back);
            faces.extend(left_to_right(last));
            alcoves.push(faces);
            beyond = away;
        }
        // Round the room: the left side from the door in, the right from the far end back.
        if s < 0.0 {
            alcoves.reverse();
        }
        for faces in alcoves {
            lib.add_category(rng, cat, &faces, titles);
            cat += 1;
        }
    }
    let near: Vec<_> = (0..6)
        .map(|k| {
            let a = -4.9 + UNIT_PITCH * (k as f32 + 0.5);
            lib.wall_bookcase(frame.at(a, RoomFrame::O_NEAR, 0.0), out)
        })
        .collect();
    lib.add_category(rng, cat, &left_to_right(near), titles);
    let far: Vec<_> = [-6.05f32, -3.35, 3.35, 6.05]
        .iter()
        .map(|&a| lib.wall_bookcase(frame.at(a, RoomFrame::O_FAR, 0.0), -out))
        .collect();
    lib.add_category(rng, cat + 1, &left_to_right(far), titles);
}

/// Panelling over the back of a single-sided case standing free at `a`, its back at
/// `o`, facing the door: shelves of jars on it, a few aglow.
fn backboard(lib: &mut Library, frame: &RoomFrame, a: f32, o: f32) {
    let hw = UNIT_PITCH * 0.5;
    lib.decor(piece(
        frame,
        (a - hw, a + hw),
        (o - 0.05, o),
        (0.0, UNIT_H + 0.13),
        kind::WOOD,
        DARK_WOOD,
    ));
    let mut rng = StdRng::seed_from_u64(0xa1c4_e31a ^ (a * 10.0) as i64 as u64);
    for y in [0.95f32, 1.65, 2.35] {
        let board = (a - hw + 0.08, a + hw - 0.08);
        lib.decor(piece(
            frame,
            board,
            (o - 0.3, o - 0.05),
            (y - 0.04, y),
            kind::WOOD,
            WALNUT,
        ));
        let mut x = a - hw + 0.2;
        while x < a + hw - 0.25 {
            let w = rng.gen_range(0.12..0.2f32);
            let foot = frame.at(x + w * 0.5, o - 0.17, y);
            let (rgb, glow) = if rng.gen_range(0.0..1.0f32) < 0.12 {
                (POTIONS[rng.gen_range(0..POTIONS.len())], 0.7)
            } else {
                (JARS[rng.gen_range(0..JARS.len())], 0.0)
            };
            let h = rng.gen_range(0.9..1.6f32) * w;
            octo(
                lib,
                foot + Vec3::Y * (h * 0.5),
                w,
                h,
                kind::CRYSTAL,
                [rgb[0], rgb[1], rgb[2], glow],
            );
            let lid = [0.28, 0.16, 0.07, 1.0];
            octo(lib, foot + Vec3::Y * (h + 0.02), w * 0.8, 0.04, kind::WOOD, lid);
            x += w + rng.gen_range(0.06..0.16);
        }
    }
}

/// Panelling where the walls are bare, sconces, a small window high in the back of
/// each alcove, a chart of the Great Work on the far wall, and beams overhead.
fn walls(lib: &mut Library, frame: &RoomFrame) {
    let (inner, near, far) = (RoomFrame::A_HALF, RoomFrame::O_NEAR, RoomFrame::O_FAR);
    let t = 0.07;
    let side = frame.side;
    // Up to the backboards of the first rows in.
    let first_row = far - WALL_CASE - 2.0 * (BAY + UNIT_DEPTH) - BAY - WALL_CASE - 0.05;
    let d = frame.door;
    for (a0, a1) in [(-inner + t, d - 1.85), (d + 1.85, -4.95), (11.35, inner - t)] {
        lib.wainscot(frame.y, frame.rect(a0, a1, near, near + t), side.out);
    }
    lib.wainscot(frame.y, frame.rect(-1.95, 1.95, far - t, far), -side.out);
    for s in [-1.0f32, 1.0] {
        let (a0, a1) = if s < 0.0 {
            (-inner, -inner + t)
        } else {
            (inner - t, inner)
        };
        lib.wainscot(frame.y, frame.rect(a0, a1, near, first_row), side.across * -s);
        lib.sconce(frame.at(s * (inner - 0.15), 17.6, 2.6), -frame.across() * s);
        // On the ends of the middle rows, facing the nave.
        let mid = far - WALL_CASE - (BAY + UNIT_DEPTH) - BAY - UNIT_DEPTH * 0.5;
        lib.sconce(
            frame.at(s * (ROW[0] - UNIT_PITCH * 0.5 - 0.15), mid, 2.6),
            -frame.across() * s,
        );
        for j in 0..3 {
            let o = far - WALL_CASE - j as f32 * (BAY + UNIT_DEPTH) - BAY * 0.5;
            let a = (s * inner - 0.04, s * inner + 0.04);
            let mut g = piece(
                frame,
                a,
                (o - 0.55, o + 0.55),
                (4.3, 5.2),
                kind::GLASS,
                [0.32, 0.42, 0.85],
            );
            g.color[3] = 0.9;
            lib.decor(g);
            let a = (s * inner - 0.1, s * inner + 0.1);
            lib.decor(piece(
                frame,
                a,
                (o - 0.7, o + 0.7),
                (4.18, 4.3),
                kind::STONE,
                [0.48, 0.45, 0.4],
            ));
            if j == 1 {
                let c = frame.at(s * (inner - 1.2), o, 4.6);
                lib.light(c, 7.0, [0.45, 0.55, 1.0], 0.5, LightKind::Static);
            }
        }
    }
    great_work(lib, frame);
    // Sooty beams across, the pipes slung under them.
    for k in 0..6 {
        let o = near + 1.6 + k as f32 * 3.0;
        let h = RoomFrame::HEIGHT;
        lib.decor(piece(
            frame,
            (-inner, inner),
            (o - 0.13, o + 0.13),
            (h - 0.3, h),
            kind::WOOD,
            [0.08, 0.045, 0.025],
        ));
    }
}

/// A great framed chart on the far wall between its cases: the circle of the work
/// with its triangle, and the four stages round it, black, white, yellow and red.
fn great_work(lib: &mut Library, frame: &RoomFrame) {
    let far = RoomFrame::O_FAR;
    let n = -frame.out();
    let (u, up) = (frame.across(), Vec3::Y);
    lib.decor(piece(
        frame,
        (-1.45, 1.45),
        (far - 0.05, far),
        (1.35, 3.65),
        kind::WOOD,
        DARK_WOOD,
    ));
    lib.decor(piece(
        frame,
        (-1.32, 1.32),
        (far - 0.07, far - 0.05),
        (1.48, 3.52),
        kind::PARCHMENT,
        [0.7, 0.58, 0.38],
    ));
    let c = frame.at(0.0, far - 0.075, 2.5);
    let ink = [0.15, 0.08, 0.05, 1.0];
    ring(lib, c, u, up, 0.75, 28, 0.02, kind::PLAIN, ink);
    let r = 0.75;
    for k in 0..3 {
        let t = TAU / 4.0 + k as f32 * TAU / 3.0;
        let p = |t: f32| c + (u * libm::cosf(t) + up * libm::sinf(t)) * r;
        rod(lib, p(t), p(t + TAU / 3.0), 0.018, 0.018, kind::PLAIN, ink);
    }
    for (k, rgb) in [
        [0.04, 0.04, 0.04],
        [0.9, 0.88, 0.8],
        [0.85, 0.65, 0.12],
        [0.6, 0.08, 0.05],
    ]
    .iter()
    .enumerate()
    {
        let t = TAU / 8.0 + k as f32 * TAU / 4.0;
        let p = c + (u * libm::cosf(t) + up * libm::sinf(t)) * 1.02 + n * 0.005;
        let rot = Quat::from_rotation_arc(Vec3::Z, n) * Quat::from_rotation_z(0.785);
        lib.decor(Inst::new(
            p,
            rot,
            Vec3::new(0.22, 0.22, 0.01),
            kind::PLAIN,
            [rgb[0], rgb[1], rgb[2], 1.0],
            0,
        ));
    }
}

/// The athanor, standing free at the far end of the nave: a brick firebox with the
/// fire alight in its mouth, a tower over it with glowing vents, and a copper hood
/// stepping up to a flue that rises to the ceiling.
fn furnace(lib: &mut Library, frame: &RoomFrame) {
    let (f, b) = FURNACE_O;
    let mid = (f + b) * 0.5;
    let (lo, hi) = frame.boxed(-1.1, 1.1, f, b, 0.0, 1.4);
    lib.collide(lo, hi, Stops::ALL);
    // The body, built round the firebox, which is open at the front.
    for (a, o, y) in [
        ((-1.1, 1.1), (f + 0.8, b), (0.0, 1.4)),
        ((-1.1, -0.45), (f, f + 0.8), (0.0, 1.4)),
        ((0.45, 1.1), (f, f + 0.8), (0.0, 1.4)),
        ((-0.45, 0.45), (f, f + 0.8), (1.05, 1.4)),
        ((-0.45, 0.45), (f, f + 0.8), (0.0, 0.3)),
    ] {
        lib.decor(piece(frame, a, o, y, kind::STONE, BRICK));
    }
    let iron = [IRON[0], IRON[1], IRON[2]];
    for (a, y) in [
        ((-0.5, 0.5), (1.05, 1.1)),
        ((-0.5, 0.5), (0.25, 0.3)),
        ((-0.5, -0.45), (0.3, 1.05)),
        ((0.45, 0.5), (0.3, 1.05)),
    ] {
        lib.decor(piece(frame, a, (f - 0.03, f), y, kind::METAL, iron));
    }
    // The ash door behind.
    lib.decor(piece(
        frame,
        (-0.35, 0.35),
        (b, b + 0.03),
        (0.15, 0.6),
        kind::METAL,
        iron,
    ));
    lib.decor(piece(
        frame,
        (-1.2, 1.2),
        (f - 0.1, b + 0.1),
        (1.4, 1.48),
        kind::STONE,
        [0.36, 0.33, 0.29],
    ));
    // The fire: a bed of coals and a few tongues of flame, flickering.
    let mut coals = piece(
        frame,
        (-0.4, 0.4),
        (f + 0.15, f + 0.7),
        (0.3, 0.4),
        kind::EMISSIVE,
        [1.0, 0.35, 0.08],
    );
    coals.color[3] = 5.0;
    coals.param = 1;
    lib.decor(coals);
    for (k, a) in [-0.22f32, 0.0, 0.2].iter().enumerate() {
        let h = 0.3 + 0.1 * k as f32;
        lib.decor(Inst::new(
            frame.at(*a, f + 0.45, 0.4 + h * 0.5),
            Quat::from_rotation_y(0.6 + k as f32),
            Vec3::new(0.13, h, 0.13),
            kind::EMISSIVE,
            [1.0, 0.55, 0.15, 7.0],
            1,
        ));
    }
    lib.light(
        frame.at(0.0, f - 0.7, 0.9),
        9.5,
        [1.0, 0.5, 0.2],
        2.2,
        LightKind::Flicker,
    );
    // The tower, its vents aglow front and back.
    lib.decor(piece(
        frame,
        (-0.75, 0.75),
        (f + 0.2, b - 0.2),
        (1.48, 2.6),
        kind::STONE,
        BRICK,
    ));
    for (o0, o1) in [(f + 0.17, f + 0.2), (b - 0.2, b - 0.17)] {
        for a in [-0.4f32, 0.0, 0.4] {
            let mut vent = piece(
                frame,
                (a - 0.07, a + 0.07),
                (o0, o1),
                (1.85, 2.15),
                kind::EMISSIVE,
                [1.0, 0.4, 0.1],
            );
            vent.color[3] = 2.5;
            vent.param = 1;
            lib.decor(vent);
        }
    }
    let copper = [COPPER[0], COPPER[1], COPPER[2]];
    let half = (b - f) * 0.5;
    for (k, (hw, ho)) in [
        (0.9f32, half - 0.05),
        (0.7, half - 0.25),
        (0.5, half - 0.45),
        (0.32, half - 0.6),
    ]
    .iter()
    .enumerate()
    {
        let y0 = 2.6 + k as f32 * 0.22;
        lib.decor(piece(
            frame,
            (-hw, *hw),
            (mid - ho, mid + ho),
            (y0, y0 + 0.22),
            kind::METAL,
            copper,
        ));
    }
    lib.decor(piece(
        frame,
        (-0.2, 0.2),
        (mid - 0.2, mid + 0.2),
        (3.48, RoomFrame::HEIGHT),
        kind::METAL,
        copper,
    ));
}

/// A long workbench down the nave on side `s`: legs and a shelf under, and on it an
/// alembic over a brazier, flasks, a rack of tubes, a mortar, books, scales and a
/// candle; a copper worm-tub at its far end.
fn bench(lib: &mut Library, frame: &RoomFrame, s: f32) {
    let (o0, o1) = BENCH_O;
    let a = s * BENCH_A;
    let top = 0.94;
    let (lo, hi) = frame.boxed(a - 0.52, a + 0.52, o0, o1, 0.0, top);
    lib.collide(lo, hi, Stops::ALL);
    lib.decor(piece(
        frame,
        (a - 0.52, a + 0.52),
        (o0, o1),
        (top - 0.08, top),
        kind::WOOD,
        OAK,
    ));
    for o in [o0 + 0.1, (o0 + o1) * 0.5, o1 - 0.1] {
        for e in [-0.42f32, 0.42] {
            let leg = (a + e - 0.05, a + e + 0.05);
            lib.decor(piece(
                frame,
                leg,
                (o - 0.05, o + 0.05),
                (0.0, top - 0.08),
                kind::WOOD,
                DARK_WOOD,
            ));
        }
    }
    lib.decor(piece(
        frame,
        (a - 0.45, a + 0.45),
        (o0 + 0.15, o1 - 0.15),
        (0.22, 0.26),
        kind::WOOD,
        WALNUT,
    ));
    let at = |da: f32, o: f32, y: f32| frame.at(a + s * da, o, top + y);
    let mut rng = StdRng::seed_from_u64(0xbe4c_0000 ^ (s > 0.0) as u64);
    // The alembic: a brazier, the gourd on it, the head and its beak running down to
    // a receiver.
    let brazier = at(0.0, o0 + 0.8, 0.07);
    octo(lib, brazier, 0.32, 0.14, kind::METAL, IRON);
    let embers = brazier + Vec3::Y * 0.075;
    let rot = Quat::from_rotation_y(0.6);
    lib.decor(Inst::new(
        embers,
        rot,
        Vec3::new(0.24, 0.02, 0.24),
        kind::EMISSIVE,
        [1.0, 0.3, 0.06, 3.0],
        1,
    ));
    let gourd = brazier + Vec3::Y * 0.07;
    vessel(lib, gourd, 0.32, 0.22, kind::METAL, COPPER);
    let head = gourd + Vec3::Y * 0.6;
    octo(lib, head, 0.17, 0.13, kind::METAL, COPPER);
    let receiver = at(0.05, o0 + 1.55, 0.0);
    rod(lib, head, receiver + Vec3::Y * 0.26, 0.03, 0.03, kind::METAL, COPPER);
    vessel(lib, receiver, 0.2, 0.08, kind::CRYSTAL, [1.0, 0.6, 0.15, 0.5]);
    // Flasks, some with something in them that glows.
    for k in 0..5 {
        let o = o0 + 2.0 + k as f32 * 0.22 + rng.gen_range(-0.04..0.04);
        let da = rng.gen_range(-0.25..0.25f32);
        let w = rng.gen_range(0.1..0.17f32);
        let rgb = POTIONS[(k + (s > 0.0) as usize * 3) % POTIONS.len()];
        let glow = if k % 2 == 0 { 1.0 } else { 0.12 };
        vessel(
            lib,
            at(da, o, 0.0),
            w,
            0.12,
            kind::CRYSTAL,
            [rgb[0], rgb[1], rgb[2], glow],
        );
    }
    // A rack of tubes.
    let rack = at(-0.2, o0 + 3.3, 0.08);
    lib.decor(Inst::aabox(rack, frame.size(0.12, 0.16, 0.5), kind::WOOD, WALNUT));
    for k in 0..5 {
        let p = rack + frame.out() * (-0.2 + k as f32 * 0.1);
        let rgb = POTIONS[(k * 2 + 1) % POTIONS.len()];
        rod(
            lib,
            p - Vec3::Y * 0.05,
            p + Vec3::Y * 0.2,
            0.04,
            0.04,
            kind::CRYSTAL,
            [rgb[0], rgb[1], rgb[2], 0.3],
        );
    }
    // A mortar and pestle.
    let mortar = at(0.2, o0 + 3.9, 0.0);
    vessel(lib, mortar, 0.2, 0.0, kind::STONE, [0.5, 0.47, 0.42, 1.0]);
    rod(
        lib,
        mortar + Vec3::Y * 0.15,
        mortar + Vec3::new(0.08, 0.32, 0.05),
        0.035,
        0.035,
        kind::STONE,
        [0.55, 0.5, 0.45, 1.0],
    );
    // Books in a pile, one lying open.
    let mut y = 0.0;
    for (k, rgb) in [[0.35, 0.1, 0.06], [0.12, 0.2, 0.1], [0.3, 0.2, 0.08]]
        .iter()
        .enumerate()
    {
        let h = 0.05 + 0.01 * k as f32;
        let c = at(-0.15, o0 + 4.5, y + h * 0.5);
        let rot = Quat::from_rotation_y(0.1 + 0.25 * k as f32);
        lib.decor(Inst::new(
            c,
            rot,
            Vec3::new(0.24, h, 0.32),
            kind::PLAIN,
            [rgb[0], rgb[1], rgb[2], 1.0],
            0,
        ));
        y += h;
    }
    for e in [-1.0f32, 1.0] {
        let page = at(0.2, o0 + 4.6 + e * 0.13, 0.012);
        let rot = Quat::from_rotation_y(0.1) * Quat::from_rotation_x(e * 0.08);
        lib.decor(Inst::new(
            page,
            rot,
            Vec3::new(0.24, 0.02, 0.25),
            kind::PARCHMENT,
            [0.85, 0.78, 0.6, 1.0],
            0,
        ));
    }
    // A brass balance.
    let scale = at(-0.2, o0 + 5.2, 0.0);
    let brass = [0.6, 0.42, 0.15, 1.0];
    rod(lib, scale, scale + Vec3::Y * 0.4, 0.025, 0.025, kind::METAL, brass);
    let beam = frame.out() * 0.2;
    rod(
        lib,
        scale + Vec3::Y * 0.4 - beam,
        scale + Vec3::Y * 0.4 + beam,
        0.02,
        0.02,
        kind::METAL,
        brass,
    );
    for e in [-1.0f32, 1.0] {
        let pan = scale + Vec3::Y * 0.22 + beam * e;
        rod(lib, pan, pan + Vec3::Y * 0.18, 0.006, 0.006, kind::METAL, brass);
        octo(lib, pan, 0.12, 0.015, kind::METAL, brass);
    }
    // A candle.
    let candle = at(0.3, o0 + 0.25, 0.0);
    lib.decor(Inst::aabox(
        candle + Vec3::Y * 0.09,
        Vec3::new(0.05, 0.18, 0.05),
        kind::PLAIN,
        [0.8, 0.75, 0.6],
    ));
    let flame = Vec3::new(0.025, 0.05, 0.025);
    lib.decor(Inst::new(
        candle + Vec3::Y * 0.21,
        Quat::IDENTITY,
        flame,
        kind::EMISSIVE,
        [1.0, 0.55, 0.18, 12.0],
        1,
    ));
    // Crates on the shelf below.
    for k in 0..4 {
        let o = o0 + 0.7 + k as f32 * 1.4;
        let c = at(rng.gen_range(-0.15..0.15f32), o, 0.0) - Vec3::Y * (top - 0.26);
        let h = rng.gen_range(0.25..0.4f32);
        let rot = Quat::from_rotation_y(rng.gen_range(0.05..0.3f32) * if k % 2 == 0 { 1.0 } else { -1.0 });
        lib.decor(Inst::new(
            c + Vec3::Y * h * 0.5,
            rot,
            Vec3::new(0.45, h, 0.4),
            kind::WOOD,
            [0.24, 0.14, 0.06, 1.0],
            0,
        ));
    }
}

/// Copper pipes from the athanor's flue, out under the ceiling and over each bench,
/// down into a worm of coiled pipe in a tub at its end.
fn pipes(lib: &mut Library, frame: &RoomFrame) {
    let y = RoomFrame::HEIGHT - 0.42;
    let flue = (FURNACE_O.0 + FURNACE_O.1) * 0.5;
    let turn = BENCH_O.1 - 0.45;
    for s in [-1.0f32, 1.0] {
        let pts = [
            frame.at(s * 0.2, flue, 4.4),
            frame.at(s * 1.0, flue, 4.4),
            frame.at(s * 1.0, flue, y),
            frame.at(s * BENCH_A, flue, y),
            frame.at(s * BENCH_A, turn, y),
            frame.at(s * BENCH_A, turn, 1.75),
        ];
        // Every other length a touch thicker, so none shares a face with the next.
        for (k, w) in pts.windows(2).enumerate() {
            rod(
                lib,
                w[0],
                w[1],
                0.09 + 0.006 * (k % 2) as f32,
                0.09 + 0.006 * (k % 2) as f32,
                kind::METAL,
                COPPER,
            );
        }
        for p in &pts[1..5] {
            let flange = [0.5, 0.25, 0.1, 1.0];
            lib.decor(Inst::new(
                *p,
                Quat::from_rotation_y(0.785),
                Vec3::splat(0.15),
                kind::METAL,
                flange,
                0,
            ));
        }
        // The worm: a coil in a tub on the bench.
        let tub = frame.at(s * BENCH_A, turn, 0.94);
        octo(lib, tub + Vec3::Y * 0.3, 0.5, 0.6, kind::WOOD, [0.26, 0.15, 0.07, 1.0]);
        for k in 0..4 {
            let c = tub + Vec3::Y * (0.65 + k as f32 * 0.09);
            ring(lib, c, Vec3::X, Vec3::Z, 0.2, 10, 0.035, kind::METAL, COPPER);
        }
    }
}

/// A transmutation circle let into the floor before the athanor, its lines faintly
/// alight: a circle, a square in it, a triangle in that, a circle in that.
fn circle(lib: &mut Library, frame: &RoomFrame) {
    let c = frame.at(0.0, CIRCLE_O, 0.006);
    let (u, v) = (frame.across(), frame.out());
    let glow = [0.55, 0.35, 1.0, 1.4];
    // The square's sides lie along the axes; each a little higher than the last, so
    // they don't share a face where they cross.
    let mut lift = 0.0;
    let mut line = |lib: &mut Library, p: Vec3, q: Vec3| {
        let rot = Quat::from_rotation_arc(Vec3::Y, (q - p).normalize());
        let c = (p + q) * 0.5 + Vec3::Y * lift;
        lib.decor(Inst::new(
            c,
            rot,
            Vec3::new(0.03, p.distance(q), 0.03),
            kind::EMISSIVE,
            glow,
            1,
        ));
        lift = (lift + 0.002) % 0.008;
    };
    let r = 1.5;
    let pt = |r: f32, t: f32| c + (u * libm::cosf(t) + v * libm::sinf(t)) * r;
    let q = std::f32::consts::FRAC_PI_4;
    // The square's own circle, which the triangle stands in, and the triangle's.
    let (square, triangle) = (r * libm::sinf(q), r * libm::sinf(q) * 0.5);
    let n = 28;
    for k in 0..n {
        let (t0, t1) = (k as f32 / n as f32 * TAU, (k + 1) as f32 / n as f32 * TAU);
        line(lib, pt(r, t0), pt(r, t1));
        line(lib, pt(triangle, t0), pt(triangle, t1));
    }
    for k in 0..4 {
        let t = q + k as f32 * TAU / 4.0;
        line(lib, pt(r, t), pt(r, t + TAU / 4.0));
    }
    for k in 0..3 {
        let t = -TAU / 4.0 + k as f32 * TAU / 3.0;
        line(lib, pt(square, t), pt(square, t + TAU / 3.0));
    }
}
