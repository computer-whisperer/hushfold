//! The Herbarium: department 2, ground floor, south side. The stacks stand close in
//! the left half of the room, four rows running out from the door; the right half is
//! a glasshouse garden under a glazed side wall, paved in stone, with staging of
//! potted plants along the glass, small trees in tubs, flower-press tables, a bed of
//! moonflowers glowing faintly, and herbs hung to dry from every beam.

use super::*;
use std::f32::consts::{FRAC_PI_2, PI};

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
    let kit = Kit::new(lib);
    garden(lib, frame, &kit);
    herbs(lib, frame);
}

/// The garden's turned pieces, each made once.
struct Kit {
    /// A clay pot a unit across and high, its foot on the axis at the origin.
    pot: Shape,
    /// A disc of soil, a stem, a ball.
    disc: Shape,
    ball: Shape,
}

impl Kit {
    fn new(lib: &mut Library) -> Kit {
        let pot = [
            (0.0, 0.0),
            (0.34, 0.0),
            (0.36, 0.02),
            (0.43, 0.8),
            (0.48, 0.82),
            (0.5, 0.86),
            (0.5, 0.97),
            (0.46, 1.0),
            (0.43, 0.93),
            (0.0, 0.93),
        ];
        Kit {
            pot: lib.shape(Mesh::lathe(&pot.map(Vec2::from), 20)),
            disc: lib.shape(Mesh::cylinder(16)),
            ball: lib.shape(Mesh::sphere(16)),
        }
    }
}

/// A spray of leaves `s` across rising from `c`: two cut-out sprays crossed, each
/// leaning a little its own way.
fn spray(lib: &mut Library, c: Vec3, s: f32, rgb: [f32; 3], rng: &mut StdRng) {
    let yaw = rng.gen_range(0.0..PI);
    for k in 0..2 {
        let lean = rng.gen_range(-0.2..0.2f32);
        let rot = Quat::from_rotation_y(yaw + k as f32 * FRAC_PI_2) * Quat::from_rotation_z(lean);
        let shade = rng.gen_range(0.85..1.1f32);
        let rgba = [rgb[0] * shade, rgb[1] * shade, rgb[2] * shade, 1.0];
        let size = Vec3::new(s, s * rng.gen_range(0.9..1.2f32), 0.004);
        lib.decor(Inst::new(
            c + rot * Vec3::Y * (size.y * 0.5),
            rot,
            size,
            kind::FRET,
            rgba,
            0,
        ));
    }
}

/// A clay pot standing on `foot`, `w` across, with a plant in it; perhaps flowering.
fn pot(lib: &mut Library, kit: &Kit, foot: Vec3, w: f32, rng: &mut StdRng) {
    let h = w * 0.9;
    let turn = Quat::from_rotation_y(rng.gen_range(0.0..TAU));
    lib.decor(
        kit.pot
            .sized(Vec3::new(w, h, w))
            .at(foot, turn, kind::PLAIN, TERRACOTTA),
    );
    let top = foot + Vec3::Y * (h * 0.93);
    let soil = kit.disc.sized(Vec3::new(w * 0.84, 0.01, w * 0.84));
    lib.decor(soil.inst(top, Quat::IDENTITY, kind::PLAIN, SOIL));
    let green = GREENS[rng.gen_range(0..GREENS.len())];
    spray(lib, top, w * rng.gen_range(1.3..2.2f32), green, rng);
    if rng.gen_range(0.0..1.0f32) < 0.45 {
        let rgb = FLOWERS[rng.gen_range(0..FLOWERS.len())];
        let bloom = kit.ball.sized(Vec3::new(w * 0.26, w * 0.14, w * 0.26));
        for _ in 0..3 {
            let p = top
                + Vec3::new(
                    rng.gen_range(-0.5..0.5f32),
                    rng.gen_range(0.9..1.5f32),
                    rng.gen_range(-0.5..0.5f32),
                ) * w;
            let rot = Quat::from_rotation_y(rng.gen_range(0.0..3.0f32)) * Quat::from_rotation_x(0.5);
            lib.decor(bloom.inst(p, rot, kind::LEAF, [rgb[0], rgb[1], rgb[2], 1.0]));
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
fn garden(lib: &mut Library, frame: &RoomFrame, kit: &Kit) {
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
        staging(lib, frame, kit, (o0, o1), &mut rng);
    }
    // An orange, a lemon and a bay.
    let fruit = [Some([0.95, 0.45, 0.08]), Some([0.92, 0.82, 0.25]), None];
    for (spot, fruit) in [(4.7, 20.2), (10.0, 29.2), (4.5, 29.4)].into_iter().zip(fruit) {
        tree(lib, frame, kit, spot, fruit, &mut rng);
    }
    moonflowers(lib, frame, kit, &mut rng);
    for o in [20.0, 26.4] {
        press_table(lib, frame, kit, o, &mut rng);
    }
    potting_bench(lib, frame, kit, &mut rng);
}

/// Tiered staging against the glass from `o0` to `o1`, three steps of pots.
fn staging(lib: &mut Library, frame: &RoomFrame, kit: &Kit, (o0, o1): (f32, f32), rng: &mut StdRng) {
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
            pot(lib, kit, frame.at(a0 + 0.17, o, *y), w, rng);
            o += w + rng.gen_range(0.12..0.3);
        }
    }
}

/// A small tree in a hooped wooden tub at `a`, `o`: a crooked trunk, a few boughs, a
/// round crown of leaves and perhaps fruit among them.
fn tree(
    lib: &mut Library,
    frame: &RoomFrame,
    kit: &Kit,
    (a, o): (f32, f32),
    fruit: Option<[f32; 3]>,
    rng: &mut StdRng,
) {
    guard(lib, frame, (a - 0.45, a + 0.45), (o - 0.45, o + 0.45), 0.62);
    let up = Vec3::Y;
    let foot = frame.at(a, o, 0.0);
    let tub = [
        (0.0, 0.0),
        (0.4, 0.0),
        (0.43, 0.03),
        (0.47, 0.62),
        (0.44, 0.62),
        (0.43, 0.59),
        (0.0, 0.59),
    ];
    let tub = lib.shape(Mesh::lathe(&tub.map(Vec2::from), 40));
    lib.decor(tub.at(foot, Quat::IDENTITY, kind::WOOD, [0.3, 0.18, 0.08, 1.0]));
    let hoop = lib.shape(Mesh::torus(0.06, 40, 8));
    for (y, r) in [(0.12f32, 0.44f32), (0.5, 0.466)] {
        lib.decor(
            hoop.scaled(r)
                .at(foot + up * y, Quat::IDENTITY, kind::METAL, [0.1, 0.1, 0.1, 1.0]),
        );
    }
    let soil = kit.disc.sized(Vec3::new(0.86, 0.02, 0.86));
    lib.decor(soil.inst(foot + up * 0.6, Quat::IDENTITY, kind::PLAIN, SOIL));
    // Trunk and boughs, tapering, bending a little on their way.
    let bark = [0.22, 0.14, 0.08, 1.0];
    let round = |n: usize| -> Vec<Vec2> {
        (0..n)
            .map(|k| {
                let t = k as f32 / n as f32 * TAU;
                Vec2::new(libm::cosf(t), libm::sinf(t))
            })
            .collect()
    };
    let bent = |p: Vec3, q: Vec3, bow: Vec3| -> Vec<Vec3> {
        (0..=6)
            .map(|k| {
                let t = k as f32 / 6.0;
                p + (q - p) * t + bow * (4.0 * t * (1.0 - t))
            })
            .collect()
    };
    let soil_top = foot + up * 0.61;
    let crown = soil_top + Vec3::new(rng.gen_range(-0.15..0.15f32), 1.6, rng.gen_range(-0.15..0.15f32));
    let lean = Vec3::new(rng.gen_range(-0.1..0.1f32), 0.0, rng.gen_range(-0.1..0.1f32));
    let trunk = bent(soil_top, crown, lean)
        .iter()
        .map(|p| *p - soil_top)
        .collect::<Vec<_>>();
    let trunk = lib.shape(Mesh::sweep(&trunk, &[0.06, 0.035], &round(10), true));
    lib.decor(trunk.at(soil_top, Quat::IDENTITY, kind::WOOD, bark));
    for _ in 0..3 {
        let from = crown - up * 0.3;
        let tip = crown
            + Vec3::new(
                rng.gen_range(-0.6..0.6f32),
                rng.gen_range(0.3..0.7f32),
                rng.gen_range(-0.6..0.6f32),
            );
        let bough: Vec<Vec3> = bent(from, tip, up * 0.08).iter().map(|p| *p - from).collect();
        let bough = lib.shape(Mesh::sweep(&bough, &[0.03, 0.012], &round(8), true));
        lib.decor(bough.at(from, Quat::IDENTITY, kind::WOOD, bark));
    }
    // The crown: masses of leaves, sprays standing out of them, and the fruit.
    let heart = crown + up * 0.5;
    let green = GREENS[rng.gen_range(0..GREENS.len())];
    for _ in 0..7 {
        let c = heart
            + Vec3::new(
                rng.gen_range(-0.4..0.4f32),
                rng.gen_range(-0.25..0.3f32),
                rng.gen_range(-0.4..0.4f32),
            );
        let s = rng.gen_range(0.55..0.8f32);
        let shade = rng.gen_range(0.8..1.1f32);
        let rgba = [green[0] * shade, green[1] * shade, green[2] * shade, 1.0];
        let mass = kit.ball.sized(Vec3::new(s, s * 0.8, s));
        lib.decor(mass.inst(c, Quat::from_rotation_y(rng.gen_range(0.0..TAU)), kind::LEAF, rgba));
    }
    let around = |rng: &mut StdRng, low: f32| {
        let t = rng.gen_range(0.0..TAU);
        let y = rng.gen_range(low..0.8f32);
        let r = libm::sqrtf(1.0 - y * y);
        Vec3::new(libm::cosf(t) * r, y, libm::sinf(t) * r)
    };
    for _ in 0..8 {
        let d = around(rng, -0.2);
        let rot = Quat::from_rotation_arc(Vec3::Z, d) * Quat::from_rotation_z(rng.gen_range(0.0..TAU));
        let s = rng.gen_range(0.35..0.5f32);
        let rgba = [green[0] * 1.1, green[1] * 1.1, green[2] * 1.1, 1.0];
        lib.decor(Inst::new(
            heart + d * 0.62,
            rot,
            Vec3::new(s, s, 0.004),
            kind::FRET,
            rgba,
            0,
        ));
    }
    if let Some(rgb) = fruit {
        let ball = kit.ball.sized(Vec3::splat(0.075));
        for _ in 0..9 {
            let d = around(rng, -0.5);
            lib.decor(ball.inst(heart + d * 0.56, Quat::IDENTITY, kind::PLAIN, rgba(rgb, 1.0)));
        }
    }
}

/// The moonflower bed: a stone-kerbed bed of dark soil, leaves, and pale blossoms on
/// their stems, giving off a faint light of their own and stirring as if in a breeze.
fn moonflowers(lib: &mut Library, frame: &RoomFrame, kit: &Kit, rng: &mut StdRng) {
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
    // An open trumpet, seen inside and out.
    let trumpet = [(0.0, 0.0), (0.06, 0.06), (0.14, 0.3), (0.32, 0.62), (0.5, 0.78)];
    let bloom = lib.shape(Mesh::lathe(&trumpet.map(Vec2::from), 20).two_sided());
    let up = Vec3::Y;
    for k in 0..14 {
        let p = frame.at(a + rng.gen_range(-1.0..1.0f32), o + rng.gen_range(-0.65..0.65f32), 0.47);
        spray(lib, p, rng.gen_range(0.35..0.6f32), GREENS[k % 3], rng);
        if k % 3 != 2 {
            let h = rng.gen_range(0.45..0.8f32);
            let t = rng.gen_range(0.0..TAU);
            let sway = Motion::Swing {
                axis: Vec3::new(libm::cosf(t), 0.0, libm::sinf(t)),
                amp: 0.05,
                period: rng.gen_range(5.0..8.0),
            };
            let m = lib.mover(p, sway, rng.gen_range(0.0..8.0), None);
            let stem = kit.disc.sized(Vec3::new(0.015, h, 0.015));
            lib.moving(
                m,
                stem.inst(p + up * (h * 0.5), Quat::IDENTITY, kind::LEAF, [0.2, 0.38, 0.15, 1.0]),
            );
            let rot = Quat::from_rotation_y(rng.gen_range(0.0..TAU)) * Quat::from_rotation_x(0.35);
            let flower = bloom.sized(Vec3::new(0.15, 0.1, 0.15));
            lib.moving(m, flower.at(p + up * h, rot, kind::CRYSTAL, [0.82, 0.9, 1.0, 1.6]));
        }
    }
    lib.light(frame.at(a, o, 1.3), 4.5, [0.7, 0.85, 1.0], 0.7, LightKind::Static);
}

/// A table for pressing flowers at `o` beside the stacks: a press with its screws,
/// herbarium sheets with specimens laid on them, a lens and an ink pot.
fn press_table(lib: &mut Library, frame: &RoomFrame, kit: &Kit, o: f32, rng: &mut StdRng) {
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
    let up = Vec3::Y;
    // The press: two boards, the screws at the corners, a pile of sheets between.
    let press = frame.at(mid, o - 0.5, top);
    for (y, h) in [(0.02f32, 0.04f32), (0.16, 0.05)] {
        lib.decor(Inst::new(
            press + up * (y + h * 0.5),
            Quat::from_rotation_y(0.1),
            Vec3::new(0.42, h, 0.42),
            kind::WOOD,
            [0.32, 0.2, 0.1, 1.0],
            0,
        ));
    }
    lib.decor(Inst::new(
        press + up * 0.09,
        Quat::from_rotation_y(0.1),
        Vec3::new(0.36, 0.1, 0.36),
        kind::PAPER,
        [0.0, 0.0, 0.0, 0.0],
        0,
    ));
    let turn = Quat::from_rotation_y(0.1);
    let iron = [0.15, 0.14, 0.13, 1.0];
    let screw = kit.disc.sized(Vec3::new(0.025, 0.3, 0.025));
    let wheel = lib.shape(Mesh::torus(0.3, 20, 8)).scaled(0.03);
    for (x, z) in [(-0.17f32, -0.17f32), (0.17, -0.17), (-0.17, 0.17), (0.17, 0.17)] {
        let p = press + turn * Vec3::new(x, 0.0, z);
        lib.decor(screw.inst(p + up * 0.15, Quat::IDENTITY, kind::METAL, iron));
        lib.decor(wheel.at(p + up * 0.29, turn, kind::METAL, iron));
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
            c + up * 0.005,
            rot * Quat::from_rotation_y(0.3),
            leaf,
            kind::LEAF,
            [green[0], green[1], green[2], 1.0],
            0,
        ));
        let rgb = FLOWERS[(k + 1) % FLOWERS.len()];
        let bloom = c + up * 0.006 + rot * Vec3::new(0.02, 0.0, 0.12);
        lib.decor(Inst::new(
            bloom,
            rot * Quat::from_rotation_y(0.785),
            Vec3::new(0.06, 0.004, 0.06),
            kind::LEAF,
            [rgb[0], rgb[1], rgb[2], 1.0],
            0,
        ));
    }
    // A reading glass, and an ink pot with a quill.
    let lens = frame.at(mid - 0.3, o + 0.7, top + 0.012);
    let brass = [0.6, 0.42, 0.15, 1.0];
    let rim = lib.shape(Mesh::torus(0.17, 32, 8)).scaled(0.07);
    lib.decor(rim.at(lens, Quat::IDENTITY, kind::METAL, brass));
    let glass = kit.disc.sized(Vec3::new(0.13, 0.006, 0.13));
    lib.translucent(glass.inst(lens, Quat::IDENTITY, kind::CLEAR, [0.85, 0.92, 1.0, 0.12]));
    let across = frame.across();
    let handle = kit.disc.sized(Vec3::new(0.022, 0.14, 0.022));
    let along = Quat::from_rotation_arc(up, across);
    lib.decor(handle.inst(lens + across * 0.145, along, kind::WOOD, [0.2, 0.1, 0.05, 1.0]));
    let ink = frame.at(mid + 0.35, o + 0.75, top);
    let pot = [
        (0.0, 0.0),
        (0.035, 0.0),
        (0.04, 0.01),
        (0.04, 0.05),
        (0.02, 0.07),
        (0.015, 0.08),
        (0.0, 0.08),
    ];
    let pot = lib.shape(Mesh::lathe(&pot.map(Vec2::from), 16));
    lib.decor(pot.at(ink, Quat::IDENTITY, kind::CRYSTAL, [0.03, 0.03, 0.06, 0.0]));
    rod(
        lib,
        ink + up * 0.06,
        ink + Vec3::new(0.06, 0.28, -0.04),
        0.012,
        0.012,
        kind::PAPER,
        [0.0, 0.0, 0.0, 0.0],
    );
}

/// A potting bench along the near wall right of the door, with pots, a watering can,
/// a seedling under a bell glass and a sack of soil.
fn potting_bench(lib: &mut Library, frame: &RoomFrame, kit: &Kit, rng: &mut StdRng) {
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
    let up = Vec3::Y;
    let mut a = a0 + 0.3;
    while a < a1 - 1.4 {
        let w = rng.gen_range(0.14..0.24f32);
        if rng.gen_range(0.0..1.0f32) < 0.6 {
            pot(lib, kit, frame.at(a, o0 + 0.4, top), w, rng);
        } else {
            // Empty pots, stacked one in another.
            let empty = kit.pot.sized(Vec3::new(w, w * 0.9, w));
            for k in 0..3 {
                let foot = frame.at(a, o0 + 0.4, top + k as f32 * 0.07);
                lib.decor(empty.at(foot, Quat::from_rotation_y(k as f32), kind::PLAIN, TERRACOTTA));
            }
        }
        a += w + rng.gen_range(0.15..0.35);
    }
    // A watering can: its body, the spout rising from its foot to the rose, and a
    // handle over the top.
    let can = frame.at(a1 - 0.9, o0 + 0.4, top);
    let tin = [0.32, 0.36, 0.3, 1.0];
    let across = frame.across();
    let body = [
        (0.0, 0.0),
        (0.1, 0.0),
        (0.11, 0.01),
        (0.11, 0.23),
        (0.09, 0.26),
        (0.0, 0.26),
    ];
    let body = lib.shape(Mesh::lathe(&body.map(Vec2::from), 24));
    lib.decor(body.at(can, Quat::IDENTITY, kind::METAL, tin));
    let round: Vec<Vec2> = (0..8)
        .map(|k| {
            let t = k as f32 / 8.0 * TAU;
            Vec2::new(libm::cosf(t), libm::sinf(t))
        })
        .collect();
    let spout = [
        up * 0.06 + across * 0.1,
        up * 0.16 + across * 0.2,
        up * 0.3 + across * 0.32,
    ];
    let spout = lib.shape(Mesh::sweep(&spout, &[0.02, 0.012], &round, true));
    lib.decor(spout.at(can, Quat::IDENTITY, kind::METAL, tin));
    let rose = kit.disc.sized(Vec3::new(0.05, 0.02, 0.05));
    let tilt = Quat::from_rotation_arc(up, (up + across).normalize());
    lib.decor(rose.inst(can + up * 0.31 + across * 0.33, tilt, kind::METAL, tin));
    let handle: Vec<Vec3> = (0..=8)
        .map(|k| {
            let t = k as f32 / 8.0 * PI;
            up * (0.25 + 0.1 * libm::sinf(t)) + across * (-0.02 - 0.07 * libm::cosf(t))
        })
        .collect();
    let handle = lib.shape(Mesh::sweep(&handle, &[0.012], &round, true));
    lib.decor(handle.at(can, Quat::IDENTITY, kind::METAL, tin));
    // A seedling in a saucer under a bell glass.
    let cloche = frame.at(a1 - 0.35, o0 + 0.38, top);
    let saucer = kit.disc.sized(Vec3::new(0.2, 0.015, 0.2));
    lib.decor(saucer.inst(cloche + up * 0.0075, Quat::IDENTITY, kind::PLAIN, TERRACOTTA));
    spray(lib, cloche + up * 0.015, 0.14, GREENS[1], rng);
    let bell = [
        (0.1, 0.0),
        (0.1, 0.13),
        (0.09, 0.18),
        (0.06, 0.21),
        (0.02, 0.225),
        (0.015, 0.24),
        (0.025, 0.26),
        (0.0, 0.27),
    ];
    let bell = lib.shape(Mesh::lathe(&bell.map(Vec2::from), 32));
    lib.translucent(bell.at(cloche, Quat::IDENTITY, kind::CLEAR, [0.8, 0.9, 1.0, 0.08]));
    // A sack of soil slumped under the bench.
    let sack = frame.at(a1 - 0.6, o0 + 0.35, 0.0);
    let rot = Quat::from_rotation_y(0.3) * Quat::from_rotation_z(0.15);
    lib.decor(Inst::new(
        sack + up * 0.2,
        rot,
        Vec3::new(0.45, 0.4, 0.35),
        kind::CLOTH,
        [0.45, 0.38, 0.25, 1.0],
        0,
    ));
}

/// Herbs hung to dry from the beams over the garden: bunches on twine, each stirring
/// on its string in its own time.
fn herbs(lib: &mut Library, frame: &RoomFrame) {
    let mut rng = StdRng::seed_from_u64(0x4e7b_0d47);
    let beam = RoomFrame::HEIGHT - 0.3;
    let up = Vec3::Y;
    let twine = lib.shape(Mesh::cylinder(6));
    // Hanging from its tie at the top, the leaves' tips at the bottom.
    let bunch = [
        (0.0, 0.0),
        (0.3, 0.06),
        (0.5, 0.3),
        (0.42, 0.62),
        (0.22, 0.86),
        (0.08, 0.97),
        (0.0, 1.0),
    ];
    let bunch = lib.shape(Mesh::lathe(&bunch.map(Vec2::from), 12));
    for k in 0..6 {
        let o = RoomFrame::O_NEAR + 1.6 + k as f32 * 3.0;
        let mut a = 0.8 + rng.gen_range(0.0..0.6f32);
        while a < RoomFrame::A_HALF - 0.8 {
            let drop = rng.gen_range(0.25..0.6f32);
            let hook = frame.at(a, o, beam);
            let tie = hook - up * drop;
            let stir = Motion::Swing {
                axis: frame.across(),
                amp: rng.gen_range(0.03..0.06),
                period: rng.gen_range(5.0..9.0),
            };
            let m = lib.mover(hook, stir, rng.gen_range(0.0..9.0), None);
            let string = twine.sized(Vec3::new(0.012, drop, 0.012));
            lib.moving(
                m,
                string.inst(
                    hook - up * (drop * 0.5),
                    Quat::IDENTITY,
                    kind::PLAIN,
                    [0.55, 0.5, 0.38, 1.0],
                ),
            );
            let rgb = DRYING[rng.gen_range(0..DRYING.len())];
            let shade = rng.gen_range(0.85..1.1f32);
            let (len, w) = (rng.gen_range(0.3..0.5f32), rng.gen_range(0.1..0.15f32));
            let herb = bunch.sized(Vec3::new(w, len, w));
            lib.moving(
                m,
                herb.at(
                    tie - up * len,
                    Quat::from_rotation_y(rng.gen_range(0.0..TAU)),
                    kind::LEAF,
                    [rgb[0] * shade, rgb[1] * shade, rgb[2] * shade, 1.0],
                ),
            );
            a += rng.gen_range(0.9..1.5);
        }
    }
}
