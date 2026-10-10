//! The Herbarium: department 2, ground floor, south side. The stacks stand close in
//! the left half of the room, four rows running out from the door; the right half is
//! a glasshouse garden under a glazed side wall, paved in stone, with staging of
//! potted plants along the glass, small trees in tubs, flower-press tables, a bed of
//! moonflowers glowing faintly, and herbs hung to dry from every beam.

use super::*;

/// Where the rows stand (a), and where they start (o).
const ROWS_A: [f32; 4] = [-10.5, -7.5, -4.5, -1.5];
const ROWS_O: f32 = 17.1;
/// Where the garden's paving starts (a and o).
const PAVING: (f32, f32) = (0.6, 17.6);
/// The moonflower bed's middle (a, o).
const BED: (f32, f32) = (6.4, 24.6);

const TERRACOTTA: [f32; 4] = [0.52, 0.24, 0.13, 1.0];
const SOIL: [f32; 4] = [0.1, 0.07, 0.045, 1.0];
const GREENS: [[f32; 3]; 4] = [
    [0.18, 0.4, 0.12],
    [0.25, 0.45, 0.15],
    [0.12, 0.32, 0.14],
    [0.33, 0.48, 0.18],
];
const FLOWERS: [[f32; 3]; 4] = [
    [0.9, 0.3, 0.35],
    [0.95, 0.78, 0.3],
    [0.88, 0.86, 0.92],
    [0.6, 0.35, 0.8],
];
/// Herbs drying: sage, lavender, straw, rosemary.
const DRYING: [[f32; 3]; 4] = [
    [0.4, 0.46, 0.32],
    [0.45, 0.36, 0.6],
    [0.5, 0.45, 0.22],
    [0.26, 0.36, 0.2],
];
const PALE_STONE: [f32; 3] = [0.48, 0.45, 0.4];

pub(super) fn furnish(lib: &mut Library, frame: &RoomFrame, rng: &mut StdRng, titles: &mut HashSet<String>) {
    shelves(lib, frame, rng, titles);
    walls(lib, frame);
    glasshouse(lib, frame);
    garden(lib, frame);
    herbs(lib, frame);
}

/// A tuft of leaves `s` across rising from `c`, the leaves splayed by `rng`.
fn tuft(lib: &mut Library, c: Vec3, s: f32, rgb: [f32; 3], rng: &mut StdRng) {
    for k in 0..4 {
        let yaw = k as f32 * 1.57 + rng.gen_range(-0.4..0.4f32);
        let lean = rng.gen_range(0.3..0.7f32);
        let rot = Quat::from_rotation_y(yaw) * Quat::from_rotation_x(lean);
        let shade = rng.gen_range(0.8..1.15f32);
        let rgba = [rgb[0] * shade, rgb[1] * shade, rgb[2] * shade, 1.0];
        let size = Vec3::new(s * 0.45, s * rng.gen_range(0.8..1.2f32), s * 0.08);
        lib.decor(Inst::new(
            c + rot * Vec3::Y * (size.y * 0.5),
            rot,
            size,
            kind::LEAF,
            rgba,
            0,
        ));
    }
}

/// A clay pot standing on `foot`, `w` across, with a plant in it; perhaps flowering.
fn pot(lib: &mut Library, foot: Vec3, w: f32, rng: &mut StdRng) {
    let h = w * 0.9;
    octo(lib, foot + Vec3::Y * (h * 0.5), w, h, kind::PLAIN, TERRACOTTA);
    octo(lib, foot + Vec3::Y * (h - 0.02), w * 1.1, 0.04, kind::PLAIN, TERRACOTTA);
    let green = GREENS[rng.gen_range(0..GREENS.len())];
    let top = foot + Vec3::Y * h;
    tuft(lib, top, w * rng.gen_range(1.3..2.2f32), green, rng);
    if rng.gen_range(0.0..1.0f32) < 0.45 {
        let rgb = FLOWERS[rng.gen_range(0..FLOWERS.len())];
        for _ in 0..3 {
            let p = top
                + Vec3::new(
                    rng.gen_range(-0.6..0.6f32),
                    rng.gen_range(1.0..1.6f32),
                    rng.gen_range(-0.6..0.6f32),
                ) * w;
            let rot = Quat::from_rotation_y(rng.gen_range(0.0..3.0f32)) * Quat::from_rotation_x(0.6);
            lib.decor(Inst::new(
                p,
                rot,
                Vec3::splat(w * 0.28),
                kind::LEAF,
                [rgb[0], rgb[1], rgb[2], 1.0],
                0,
            ));
        }
    }
}

/// Four rows of five cases running out from the door, close together; each side of
/// a row is a category.
fn shelves(lib: &mut Library, frame: &RoomFrame, rng: &mut StdRng, titles: &mut HashSet<String>) {
    let mut cat = frame.categories.start;
    let along = frame.out_along_x();
    for a in ROWS_A {
        let (mut left, mut right) = (Vec::new(), Vec::new());
        for k in 0..5 {
            let o = ROWS_O + UNIT_PITCH * (k as f32 + 0.5);
            for face in lib.bookcase(frame.at(a, o, 0.0), along) {
                if face.1.dot(frame.across()) < 0.0 {
                    left.push(face);
                } else {
                    right.push(face);
                }
            }
        }
        for mut faces in [left, right] {
            let right = Vec3::Y.cross(faces[0].1);
            faces.sort_by(|p, q| p.0.dot(right).total_cmp(&q.0.dot(right)));
            lib.add_category(rng, cat, &faces, titles);
            cat += 1;
        }
    }
}

/// Panelling on the bare walls, sconces down the stacks' side, and beams across.
fn walls(lib: &mut Library, frame: &RoomFrame) {
    let (inner, near, far) = (RoomFrame::A_HALF, RoomFrame::O_NEAR, RoomFrame::O_FAR);
    let t = 0.07;
    let side = frame.side;
    lib.wainscot(frame.y, frame.rect(-inner, -inner + t, near, far - t), side.across);
    lib.wainscot(frame.y, frame.rect(-inner, 1.0, far - t, far), -side.out);
    for (a0, a1) in [(-inner + t, -1.8), (1.8, 5.85)] {
        lib.wainscot(frame.y, frame.rect(a0, a1, near, near + t), side.out);
    }
    for o in [19.5, 27.0] {
        lib.sconce(frame.at(-inner + 0.15, o, 2.6), frame.across());
    }
    for k in 0..6 {
        let o = near + 1.6 + k as f32 * 3.0;
        let h = RoomFrame::HEIGHT;
        lib.decor(piece(
            frame,
            (-inner, inner),
            (o - 0.13, o + 0.13),
            (h - 0.3, h),
            kind::WOOD,
            [0.11, 0.055, 0.025],
        ));
    }
}

/// The glasshouse wall: the right wall glazed nearly floor to ceiling between stone
/// piers, and the far wall's right half likewise, moonlight falling through.
fn glasshouse(lib: &mut Library, frame: &RoomFrame) {
    let (inner, far) = (RoomFrame::A_HALF, RoomFrame::O_FAR);
    let (y0, y1) = (0.7, 5.05);
    // The glass is leaded across its own x, so a pane is turned to lie along its wall.
    let pane = |lib: &mut Library, a: (f32, f32), o: (f32, f32)| {
        let (lo, hi) = frame.boxed(a.0, a.1, o.0, o.1, y0, y1);
        let size = hi - lo;
        let (rot, size) = if size.z > size.x {
            (
                Quat::from_rotation_y(std::f32::consts::FRAC_PI_2),
                Vec3::new(size.z, size.y, size.x),
            )
        } else {
            (Quat::IDENTITY, size)
        };
        lib.decor(Inst::new(
            (lo + hi) * 0.5,
            rot,
            size,
            kind::GLASS,
            [0.3, 0.42, 0.8, 1.0],
            0,
        ));
    };
    // Down the right wall: five bays, piers between and a sill under.
    let bays = [(15.3, 18.0), (18.4, 21.1), (21.5, 24.2), (24.6, 27.3), (27.7, 30.4)];
    for &(o0, o1) in &bays {
        pane(lib, (inner - 0.04, inner + 0.04), (o0, o1));
    }
    let mut o = 14.9;
    for &(o0, o1) in &bays {
        lib.decor(piece(
            frame,
            (inner - 0.12, inner),
            (o, o0),
            (0.0, RoomFrame::HEIGHT),
            kind::STONE,
            PALE_STONE,
        ));
        o = o1;
    }
    lib.decor(piece(
        frame,
        (inner - 0.12, inner),
        (o, 31.1),
        (0.0, RoomFrame::HEIGHT),
        kind::STONE,
        PALE_STONE,
    ));
    // Sill and head stop short of the end piers, and the head stands back from the
    // piers' faces, so none shares a face with them.
    lib.decor(piece(
        frame,
        (inner - 0.25, inner),
        (15.0, 31.0),
        (y0 - 0.12, y0),
        kind::STONE,
        PALE_STONE,
    ));
    lib.decor(piece(
        frame,
        (inner - 0.1, inner),
        (15.0, 31.0),
        (y1, y1 + 0.2),
        kind::STONE,
        PALE_STONE,
    ));
    // Across the far wall's right half.
    let far_bays = [(1.5, 4.7), (5.1, 8.3), (8.7, 11.9)];
    for &(a0, a1) in &far_bays {
        pane(lib, (a0, a1), (far - 0.04, far + 0.04));
    }
    let mut a = 1.1;
    for &(a0, a1) in &far_bays {
        lib.decor(piece(
            frame,
            (a, a0),
            (far - 0.12, far),
            (0.0, RoomFrame::HEIGHT),
            kind::STONE,
            PALE_STONE,
        ));
        a = a1;
    }
    lib.decor(piece(
        frame,
        (a, inner - 0.12),
        (far - 0.12, far),
        (0.0, RoomFrame::HEIGHT),
        kind::STONE,
        PALE_STONE,
    ));
    lib.decor(piece(
        frame,
        (1.2, inner - 0.3),
        (far - 0.25, far),
        (y0 - 0.12, y0),
        kind::STONE,
        PALE_STONE,
    ));
    lib.decor(piece(
        frame,
        (1.2, inner - 0.3),
        (far - 0.1, far),
        (y1, y1 + 0.2),
        kind::STONE,
        PALE_STONE,
    ));
    // Moonlight.
    let (out, across) = (frame.out(), frame.across());
    for (c, dir) in [
        (frame.at(inner, 19.75, 3.2), -across),
        (frame.at(inner, 25.95, 3.2), -across),
        (frame.at(6.7, far, 3.2), -out),
    ] {
        lib.light(c + dir * 2.0, 9.0, [0.45, 0.55, 1.0], 0.8, LightKind::Static);
        let beam = (dir - Vec3::Y * 0.7).normalize();
        let len = 7.0;
        lib.static_trans.push(Inst::new(
            c + beam * len * 0.5,
            Quat::from_rotation_arc(Vec3::Y, beam),
            Vec3::new(2.2, len, 2.2),
            kind::SHAFT,
            [0.35, 0.45, 0.9, 0.035],
            0,
        ));
    }
}

/// The garden: stone paving, staging of pots along the glass, trees in tubs, the
/// moonflower bed, two flower-press tables by the stacks, and a potting bench by the door.
fn garden(lib: &mut Library, frame: &RoomFrame) {
    let (inner, far) = (RoomFrame::A_HALF, RoomFrame::O_FAR);
    lib.decor(piece(
        frame,
        (PAVING.0, inner - 0.12),
        (PAVING.1, far - 0.12),
        (0.0, 0.004),
        kind::STONE,
        [0.4, 0.38, 0.34],
    ));
    let mut rng = StdRng::seed_from_u64(0x4e7b_a201);
    for (o0, o1) in [(15.6, 20.4), (21.8, 26.6)] {
        staging(lib, frame, o0, o1, &mut rng);
    }
    for (a, o) in [(4.7, 20.2), (10.0, 29.2), (4.5, 29.4)] {
        tree(lib, frame, a, o, &mut rng);
    }
    moonflowers(lib, frame, &mut rng);
    for o in [20.0, 26.4] {
        press_table(lib, frame, o, &mut rng);
    }
    potting_bench(lib, frame, &mut rng);
}

/// Tiered staging against the glass from `o0` to `o1`, three steps of pots.
fn staging(lib: &mut Library, frame: &RoomFrame, o0: f32, o1: f32, rng: &mut StdRng) {
    let back = RoomFrame::A_HALF - 0.14;
    let (lo, hi) = frame.boxed(back - 1.0, back, o0, o1, 0.0, 1.2);
    lib.collide(lo, hi, Stops::ALL);
    for (k, y) in [0.42f32, 0.78, 1.14].iter().enumerate() {
        let a0 = back - 1.0 + k as f32 * 0.33;
        lib.decor(piece(frame, (a0, back), (o0, o1), (y - 0.04, *y), kind::WOOD, WALNUT));
        for o in [o0 + 0.05, (o0 + o1) * 0.5, o1 - 0.05] {
            lib.decor(piece(
                frame,
                (a0 + 0.03, a0 + 0.09),
                (o - 0.03, o + 0.03),
                (0.0, y - 0.04),
                kind::WOOD,
                DARK_WOOD,
            ));
        }
        let mut o = o0 + 0.25;
        while o < o1 - 0.2 {
            let w = rng.gen_range(0.16..0.26f32);
            pot(lib, frame.at(a0 + 0.17, o, *y), w, rng);
            o += w + rng.gen_range(0.12..0.3);
        }
    }
}

/// A small tree in a square wooden tub at `a`, `o`.
fn tree(lib: &mut Library, frame: &RoomFrame, a: f32, o: f32, rng: &mut StdRng) {
    let tub = piece(
        frame,
        (a - 0.45, a + 0.45),
        (o - 0.45, o + 0.45),
        (0.0, 0.62),
        kind::WOOD,
        [0.3, 0.18, 0.08],
    );
    lib.solid(tub, Stops::ALL);
    for y in [0.12f32, 0.5] {
        lib.decor(piece(
            frame,
            (a - 0.47, a + 0.47),
            (o - 0.47, o + 0.47),
            (y, y + 0.05),
            kind::METAL,
            [0.1, 0.1, 0.1],
        ));
    }
    lib.decor(piece(
        frame,
        (a - 0.4, a + 0.4),
        (o - 0.4, o + 0.4),
        (0.62, 0.64),
        kind::PLAIN,
        [SOIL[0], SOIL[1], SOIL[2]],
    ));
    let foot = frame.at(a, o, 0.62);
    let crown = foot + Vec3::new(rng.gen_range(-0.15..0.15f32), 1.6, rng.gen_range(-0.15..0.15f32));
    rod(lib, foot, crown, 0.11, 0.11, kind::WOOD, [0.22, 0.14, 0.08, 1.0]);
    for _ in 0..3 {
        let tip = crown
            + Vec3::new(
                rng.gen_range(-0.7..0.7f32),
                rng.gen_range(0.3..0.8f32),
                rng.gen_range(-0.7..0.7f32),
            );
        rod(
            lib,
            crown - Vec3::Y * 0.3,
            tip,
            0.05,
            0.05,
            kind::WOOD,
            [0.22, 0.14, 0.08, 1.0],
        );
    }
    let green = GREENS[rng.gen_range(0..GREENS.len())];
    for _ in 0..16 {
        let c = crown
            + Vec3::new(
                rng.gen_range(-0.8..0.8f32),
                rng.gen_range(0.1..1.2f32),
                rng.gen_range(-0.8..0.8f32),
            );
        let rot =
            Quat::from_rotation_y(rng.gen_range(0.0..3.0f32)) * Quat::from_rotation_x(rng.gen_range(-0.5..0.5f32));
        let s = rng.gen_range(0.3..0.6f32);
        let shade = rng.gen_range(0.75..1.1f32);
        let rgba = [green[0] * shade, green[1] * shade, green[2] * shade, 1.0];
        lib.decor(Inst::new(c, rot, Vec3::new(s, s * 0.7, s), kind::LEAF, rgba, 0));
    }
}

/// The moonflower bed: a stone-kerbed bed of dark soil, leaves, and pale blossoms
/// giving off a faint light of their own.
fn moonflowers(lib: &mut Library, frame: &RoomFrame, rng: &mut StdRng) {
    let (a, o) = BED;
    let (ha, ho) = (1.3, 0.9);
    lib.solid(
        piece(
            frame,
            (a - ha, a + ha),
            (o - ho, o + ho),
            (0.0, 0.45),
            kind::STONE,
            PALE_STONE,
        ),
        Stops::ALL,
    );
    lib.decor(piece(
        frame,
        (a - ha + 0.12, a + ha - 0.12),
        (o - ho + 0.12, o + ho - 0.12),
        (0.45, 0.47),
        kind::PLAIN,
        [SOIL[0], SOIL[1], SOIL[2]],
    ));
    for k in 0..14 {
        let p = frame.at(a + rng.gen_range(-1.0..1.0f32), o + rng.gen_range(-0.65..0.65f32), 0.47);
        tuft(lib, p, rng.gen_range(0.35..0.6f32), GREENS[k % 3], rng);
        if k % 3 != 2 {
            let stem = p + Vec3::Y * rng.gen_range(0.45..0.8f32);
            rod(lib, p, stem, 0.015, 0.015, kind::LEAF, [0.2, 0.38, 0.15, 1.0]);
            let rot = Quat::from_rotation_y(rng.gen_range(0.0..1.5f32)) * Quat::from_rotation_x(0.3);
            let bloom = [0.82, 0.9, 1.0, 1.6];
            lib.decor(Inst::new(
                stem,
                rot,
                Vec3::new(0.12, 0.05, 0.12),
                kind::CRYSTAL,
                bloom,
                0,
            ));
            let rot = rot * Quat::from_rotation_y(0.785);
            lib.decor(Inst::new(
                stem + Vec3::Y * 0.02,
                rot,
                Vec3::new(0.08, 0.05, 0.08),
                kind::CRYSTAL,
                bloom,
                0,
            ));
        }
    }
    lib.light(frame.at(a, o, 1.3), 4.5, [0.7, 0.85, 1.0], 0.7, LightKind::Static);
}

/// A table for pressing flowers at `o` beside the stacks: a press with its screws,
/// herbarium sheets with specimens laid on them, a lens and an ink pot.
fn press_table(lib: &mut Library, frame: &RoomFrame, o: f32, rng: &mut StdRng) {
    let (a0, a1) = (1.2, 2.4);
    let (o0, o1) = (o - 1.0, o + 1.0);
    let top = 0.82;
    let (lo, hi) = frame.boxed(a0, a1, o0, o1, 0.0, top);
    lib.collide(lo, hi, Stops::ALL);
    lib.decor(piece(frame, (a0, a1), (o0, o1), (top - 0.06, top), kind::WOOD, WALNUT));
    for (a, oo) in [
        (a0 + 0.06, o0 + 0.06),
        (a1 - 0.06, o0 + 0.06),
        (a0 + 0.06, o1 - 0.06),
        (a1 - 0.06, o1 - 0.06),
    ] {
        lib.decor(piece(
            frame,
            (a - 0.04, a + 0.04),
            (oo - 0.04, oo + 0.04),
            (0.0, top - 0.06),
            kind::WOOD,
            DARK_WOOD,
        ));
    }
    let mid = (a0 + a1) * 0.5;
    // The press: two boards, the screws at the corners, a pile of sheets between.
    let press = frame.at(mid, o - 0.5, top);
    for (y, h) in [(0.02f32, 0.04f32), (0.16, 0.05)] {
        lib.decor(Inst::new(
            press + Vec3::Y * (y + h * 0.5),
            Quat::from_rotation_y(0.1),
            Vec3::new(0.42, h, 0.42),
            kind::WOOD,
            [0.32, 0.2, 0.1, 1.0],
            0,
        ));
    }
    lib.decor(Inst::new(
        press + Vec3::Y * 0.09,
        Quat::from_rotation_y(0.1),
        Vec3::new(0.36, 0.1, 0.36),
        kind::PAPER,
        [0.0, 0.0, 0.0, 0.0],
        0,
    ));
    let turn = Quat::from_rotation_y(0.1);
    for (x, z) in [(-0.17f32, -0.17f32), (0.17, -0.17), (-0.17, 0.17), (0.17, 0.17)] {
        let p = press + turn * Vec3::new(x, 0.0, z);
        rod(
            lib,
            p,
            p + Vec3::Y * 0.3,
            0.025,
            0.025,
            kind::METAL,
            [0.15, 0.14, 0.13, 1.0],
        );
        octo(lib, p + Vec3::Y * 0.3, 0.07, 0.02, kind::METAL, [0.15, 0.14, 0.13, 1.0]);
    }
    // Sheets with a pressed specimen on each.
    for k in 0..3 {
        let c = frame.at(
            mid + rng.gen_range(-0.2..0.2f32),
            o + 0.15 + k as f32 * 0.32,
            top + 0.005 + k as f32 * 0.002,
        );
        let rot = Quat::from_rotation_y(rng.gen_range(-0.25..0.25f32));
        lib.decor(Inst::new(
            c,
            rot,
            Vec3::new(0.32, 0.006, 0.42),
            kind::PARCHMENT,
            [0.86, 0.8, 0.64, 1.0],
            0,
        ));
        let green = GREENS[k % GREENS.len()];
        let leaf = Vec3::new(0.05, 0.004, 0.24);
        lib.decor(Inst::new(
            c + Vec3::Y * 0.005,
            rot * Quat::from_rotation_y(0.3),
            leaf,
            kind::LEAF,
            [green[0], green[1], green[2], 1.0],
            0,
        ));
        let rgb = FLOWERS[(k + 1) % FLOWERS.len()];
        let bloom = c + Vec3::Y * 0.006 + rot * Vec3::new(0.02, 0.0, 0.12);
        lib.decor(Inst::new(
            bloom,
            rot * Quat::from_rotation_y(0.785),
            Vec3::new(0.06, 0.004, 0.06),
            kind::LEAF,
            [rgb[0], rgb[1], rgb[2], 1.0],
            0,
        ));
    }
    // A lens, and an ink pot with a quill.
    let lens = frame.at(mid - 0.3, o + 0.7, top + 0.01);
    let brass = [0.6, 0.42, 0.15, 1.0];
    let n = 12;
    for k in 0..n {
        let (t0, t1) = (k as f32 / n as f32 * TAU, (k + 1) as f32 / n as f32 * TAU);
        let p = |t: f32| lens + Vec3::new(t.cos(), 0.0, t.sin()) * 0.07;
        rod(lib, p(t0), p(t1), 0.012, 0.012, kind::METAL, brass);
    }
    rod(
        lib,
        lens + Vec3::new(0.07, 0.0, 0.0),
        lens + Vec3::new(0.2, 0.0, 0.05),
        0.02,
        0.02,
        kind::WOOD,
        [0.2, 0.1, 0.05, 1.0],
    );
    let ink = frame.at(mid + 0.35, o + 0.75, top);
    octo(
        lib,
        ink + Vec3::Y * 0.04,
        0.07,
        0.08,
        kind::PLAIN,
        [0.05, 0.05, 0.08, 1.0],
    );
    rod(
        lib,
        ink + Vec3::Y * 0.06,
        ink + Vec3::new(0.06, 0.28, -0.04),
        0.012,
        0.012,
        kind::PAPER,
        [0.0, 0.0, 0.0, 0.0],
    );
}

/// A potting bench along the near wall right of the door, with pots, a watering can
/// and a sack of soil.
fn potting_bench(lib: &mut Library, frame: &RoomFrame, rng: &mut StdRng) {
    let near = RoomFrame::O_NEAR;
    let (a0, a1) = (6.0, 11.6);
    let (o0, o1) = (near, near + 0.75);
    let top = 0.86;
    let (lo, hi) = frame.boxed(a0, a1, o0, o1, 0.0, top);
    lib.collide(lo, hi, Stops::ALL);
    lib.decor(piece(
        frame,
        (a0, a1),
        (o0, o1),
        (top - 0.06, top),
        kind::WOOD,
        [0.26, 0.15, 0.07],
    ));
    for a in [a0 + 0.08, (a0 + a1) * 0.5, a1 - 0.08] {
        lib.decor(piece(
            frame,
            (a - 0.04, a + 0.04),
            (o1 - 0.12, o1 - 0.04),
            (0.0, top - 0.06),
            kind::WOOD,
            DARK_WOOD,
        ));
    }
    lib.decor(piece(
        frame,
        (a0 + 0.05, a1 - 0.05),
        (o0, o1 - 0.05),
        (0.2, 0.24),
        kind::WOOD,
        WALNUT,
    ));
    let mut a = a0 + 0.3;
    while a < a1 - 1.4 {
        let w = rng.gen_range(0.14..0.24f32);
        if rng.gen_range(0.0..1.0f32) < 0.6 {
            pot(lib, frame.at(a, o0 + 0.4, top), w, rng);
        } else {
            // Empty pots, stacked.
            for k in 0..3 {
                octo(
                    lib,
                    frame.at(a, o0 + 0.4, top + 0.09 + k as f32 * 0.08),
                    w,
                    0.17,
                    kind::PLAIN,
                    TERRACOTTA,
                );
            }
        }
        a += w + rng.gen_range(0.15..0.35);
    }
    // A watering can.
    let can = frame.at(a1 - 0.9, o0 + 0.4, top);
    let tin = [0.32, 0.36, 0.3, 1.0];
    octo(lib, can + Vec3::Y * 0.13, 0.22, 0.26, kind::METAL, tin);
    rod(
        lib,
        can + Vec3::Y * 0.08 + frame.across() * 0.1,
        can + Vec3::Y * 0.32 + frame.across() * 0.32,
        0.03,
        0.03,
        kind::METAL,
        tin,
    );
    rod(
        lib,
        can + Vec3::Y * 0.3 - frame.across() * 0.08,
        can + Vec3::Y * 0.3 + frame.across() * 0.04,
        0.025,
        0.025,
        kind::METAL,
        tin,
    );
    // A sack of soil slumped under the bench.
    let sack = frame.at(a1 - 0.6, o0 + 0.35, 0.0);
    let rot = Quat::from_rotation_y(0.3) * Quat::from_rotation_z(0.15);
    lib.decor(Inst::new(
        sack + Vec3::Y * 0.2,
        rot,
        Vec3::new(0.45, 0.4, 0.35),
        kind::CLOTH,
        [0.45, 0.38, 0.25, 1.0],
        0,
    ));
}

/// Herbs hung to dry from the beams over the garden: bunches tied with twine.
fn herbs(lib: &mut Library, frame: &RoomFrame) {
    let mut rng = StdRng::seed_from_u64(0x4e7b_0d47);
    let beam = RoomFrame::HEIGHT - 0.3;
    for k in 0..6 {
        let o = RoomFrame::O_NEAR + 1.6 + k as f32 * 3.0;
        let mut a = 0.8 + rng.gen_range(0.0..0.6f32);
        while a < RoomFrame::A_HALF - 0.8 {
            let drop = rng.gen_range(0.25..0.6f32);
            let tie = frame.at(a, o, beam - drop);
            rod(
                lib,
                tie,
                frame.at(a, o, beam),
                0.012,
                0.012,
                kind::PLAIN,
                [0.55, 0.5, 0.38, 1.0],
            );
            let rgb = DRYING[rng.gen_range(0..DRYING.len())];
            let len = rng.gen_range(0.3..0.5f32);
            for j in 0..3 {
                let yaw = j as f32 * 2.1 + rng.gen_range(0.0..0.5f32);
                let rot = Quat::from_rotation_y(yaw) * Quat::from_rotation_x(rng.gen_range(0.1..0.3f32));
                let shade = rng.gen_range(0.85..1.1f32);
                let rgba = [rgb[0] * shade, rgb[1] * shade, rgb[2] * shade, 1.0];
                let size = Vec3::new(0.09, len, 0.05);
                lib.decor(Inst::new(
                    tie - rot * Vec3::Y * (len * 0.5),
                    rot,
                    size,
                    kind::LEAF,
                    rgba,
                    0,
                ));
            }
            octo(lib, tie, 0.05, 0.04, kind::PLAIN, [0.55, 0.5, 0.38, 1.0]);
            a += rng.gen_range(0.9..1.5);
        }
    }
}
