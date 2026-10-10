//! The Alchemical Wing: department 1, ground floor, east side. Three alcoves of cases
//! open off each side of a working nave. Down the nave stand two long workbenches
//! crowded with glassware and, beyond them, a brick athanor with its fire alight and
//! copper pipes running from its flue over to the benches; a transmutation circle
//! glows faintly in the floor before it.

use super::*;
use std::f32::consts::{FRAC_PI_2, PI};

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
const BRASS_LIT: [f32; 4] = [0.6, 0.42, 0.15, 1.0];
const CORK: [f32; 4] = [0.42, 0.28, 0.14, 1.0];
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
    let glass = Glassware::new(lib);
    for s in [-1.0f32, 1.0] {
        bench(lib, frame, s, &glass);
    }
    pipes(lib, frame, &glass);
    transmutation(lib, frame);
}

/// The path through `pts` with each corner rounded off, up to `r` either side of it.
fn rounded(pts: &[Vec3], r: f32) -> Vec<Vec3> {
    let mut out = vec![pts[0]];
    for w in pts.windows(3) {
        let (a, b, c) = (w[0], w[1], w[2]);
        let (to_a, to_c) = ((a - b).normalize(), (c - b).normalize());
        let r = r.min(a.distance(b) * 0.45).min(b.distance(c) * 0.45);
        let bend = PI - libm::acosf(to_a.dot(to_c).clamp(-1.0, 1.0));
        let n = libm::ceilf(bend / 0.2).max(1.0) as usize;
        let (p, q) = (b + to_a * r, b + to_c * r);
        out.extend((0..=n).map(|k| {
            let t = k as f32 / n as f32;
            p * ((1.0 - t) * (1.0 - t)) + b * (2.0 * t * (1.0 - t)) + q * (t * t)
        }));
    }
    out.push(pts[pts.len() - 1]);
    out
}

/// Clear glass, faintly blue.
const GLASS: [f32; 4] = [0.8, 0.9, 1.0, 0.08];

/// The room's glassware, each a vessel of clear glass and what it holds, made with its
/// foot on the axis and a unit across: `scaled` to its width.
struct Glassware {
    round: [Shape; 2],
    cone: [Shape; 2],
    bottle: [Shape; 2],
    tube: [Shape; 2],
    retort: [Shape; 2],
    /// A cork ring for a round-bottomed vessel to stand in.
    cork: Shape,
}

impl Glassware {
    fn new(lib: &mut Library) -> Glassware {
        let v = Vec2::new;
        // A bowl's curve from its foot round to `to` radians above the level of its
        // middle, `r` round about the height `y`.
        let bowl = |r: f32, y: f32, to: f32| -> Vec<Vec2> {
            (0..=12)
                .map(|k| {
                    let t = -FRAC_PI_2 + (to + FRAC_PI_2) * k as f32 / 12.0;
                    v(libm::cosf(t) * r, y + libm::sinf(t) * r)
                })
                .collect()
        };
        let lathe = |lib: &mut Library, p: &[Vec2]| lib.shape(Mesh::lathe(p, 24));
        // Liquid filling a bowl up to `to`.
        let fill = |r: f32, y: f32, to: f32| {
            let mut p = bowl(r, y, to);
            p.push(v(0.0, y + libm::sinf(to) * r));
            p
        };
        // A round-bottomed flask: a ball and a long neck, open at its lip.
        let mut p = bowl(0.5, 0.5, 1.22);
        p.extend([v(0.16, 1.05), v(0.15, 1.55), v(0.19, 1.6), v(0.18, 1.64)]);
        let round = [lathe(lib, &p), lathe(lib, &fill(0.47, 0.5, -0.1))];
        // A conical flask.
        let p = [
            (0.0, 0.0),
            (0.44, 0.0),
            (0.5, 0.04),
            (0.49, 0.1),
            (0.17, 0.82),
            (0.15, 0.9),
            (0.15, 1.12),
            (0.18, 1.15),
            (0.17, 1.18),
        ]
        .map(Vec2::from);
        let liquid = [
            (0.0, 0.02),
            (0.44, 0.02),
            (0.465, 0.05),
            (0.455, 0.1),
            (0.36, 0.32),
            (0.0, 0.32),
        ];
        let cone = [lathe(lib, &p), lathe(lib, &liquid.map(Vec2::from))];
        // A reagent bottle with a narrow mouth.
        let p = [
            (0.0, 0.0),
            (0.46, 0.0),
            (0.5, 0.05),
            (0.5, 1.15),
            (0.44, 1.3),
            (0.24, 1.42),
            (0.2, 1.5),
            (0.2, 1.68),
            (0.24, 1.72),
            (0.22, 1.76),
        ]
        .map(Vec2::from);
        let liquid = [(0.0, 0.02), (0.46, 0.02), (0.47, 0.06), (0.47, 0.8), (0.0, 0.8)];
        let bottle = [lathe(lib, &p), lathe(lib, &liquid.map(Vec2::from))];
        // A test tube.
        let mut p = bowl(0.5, 0.5, 0.0);
        p.extend([v(0.5, 7.0), v(0.58, 7.08)]);
        let mut liquid = bowl(0.42, 0.5, 0.0);
        liquid.extend([v(0.42, 5.4), v(0.0, 5.4)]);
        let tube = [lathe(lib, &p), lathe(lib, &liquid)];
        // A retort: a ball, its neck drawn up, over and down, narrowing as it goes.
        let mut retort = Mesh::lathe(&bowl(0.5, 0.5, FRAC_PI_2), 24);
        let (p0, c, p1) = (v(0.18, 0.92), v(0.75, 1.45), v(1.75, 0.45));
        let neck: Vec<Vec3> = (0..=12)
            .map(|k| {
                let t = k as f32 / 12.0;
                let q = p0 * ((1.0 - t) * (1.0 - t)) + c * (2.0 * t * (1.0 - t)) + p1 * (t * t);
                q.extend(0.0)
            })
            .collect();
        retort.add(
            &Mesh::sweep(&neck, &[0.15, 0.05], &circle(1.0, 12), false),
            Vec3::ZERO,
            Quat::IDENTITY,
            Vec3::ONE,
        );
        let retort = [lib.shape(retort), round[1]];
        let cork = lib.shape(Mesh::torus(0.6, 16, 8));
        Glassware {
            round,
            cone,
            bottle,
            tube,
            retort,
            cork,
        }
    }

    /// A vessel (`glass` and what it holds) standing on `foot`, `w` across, turned
    /// `yaw`, holding `liquid` (a crystal's colour and glow).
    fn stand(&self, lib: &mut Library, [glass, fill]: [Shape; 2], foot: Vec3, yaw: f32, w: f32, liquid: [f32; 4]) {
        let turn = Quat::from_rotation_y(yaw);
        lib.translucent(glass.scaled(w).at(foot, turn, kind::CLEAR, GLASS));
        lib.decor(fill.scaled(w).at(foot, turn, kind::CRYSTAL, liquid));
    }

    /// A cork ring on `foot` for a round-bottomed vessel `w` across to stand in.
    fn cork(&self, lib: &mut Library, foot: Vec3, w: f32) {
        let ring = self.cork.scaled(w * 0.28);
        lib.decor(ring.inst(foot + Vec3::Y * (ring.size.y * 0.5), Quat::IDENTITY, kind::WOOD, CORK));
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
    // Squat jars, tall ones and bottles, a lid or a stopper on each.
    let jars = [
        lib.turned(
            &[
                (0.0, 0.0),
                (0.44, 0.0),
                (0.5, 0.06),
                (0.5, 0.86),
                (0.42, 0.96),
                (0.4, 1.0),
                (0.0, 1.0),
            ],
            20,
        ),
        lib.turned(
            &[
                (0.0, 0.0),
                (0.46, 0.0),
                (0.5, 0.05),
                (0.48, 0.7),
                (0.3, 0.85),
                (0.22, 0.9),
                (0.22, 1.0),
                (0.0, 1.0),
            ],
            20,
        ),
        lib.turned(
            &[
                (0.0, 0.0),
                (0.42, 0.0),
                (0.5, 0.15),
                (0.5, 0.55),
                (0.38, 0.75),
                (0.3, 0.85),
                (0.33, 1.0),
                (0.0, 1.0),
            ],
            20,
        ),
    ];
    let lid = lib.turned(
        &[
            (0.0, 0.0),
            (0.5, 0.0),
            (0.5, 0.5),
            (0.3, 0.8),
            (0.12, 0.85),
            (0.1, 1.0),
            (0.0, 1.0),
        ],
        16,
    );
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
            let k = rng.gen_range(0..jars.len());
            let jar = jars[k].sized(Vec3::new(w, h, w));
            let turn = Quat::from_rotation_y(rng.gen_range(0.0..TAU));
            lib.decor(jar.at(foot, turn, kind::CRYSTAL, [rgb[0], rgb[1], rgb[2], glow]));
            // Its lid as wide as its mouth.
            let mouth = [0.88, 0.48, 0.66][k] * w;
            let lid = lid.sized(Vec3::new(mouth, 0.04, mouth));
            lib.decor(lid.at(foot + Vec3::Y * (h - 0.01), turn, kind::WOOD, [0.28, 0.16, 0.07, 1.0]));
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
    let r = 0.75;
    let hoop = band(lib, c, (u, up), (r, 0.02, 0.012), kind::PLAIN, ink);
    lib.decor(hoop);
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
    // The fire: a bed of coals and the flames over it.
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
    let mut fire = piece(
        frame,
        (-0.36, 0.36),
        (f + 0.18, f + 0.66),
        (0.38, 1.0),
        kind::FIRE,
        [1.0, 0.5, 0.15],
    );
    fire.color[3] = 2.5;
    fire.param = 1;
    lib.translucent(fire);
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
    // A round copper hood flaring over the tower, and the flue up from it, banded.
    let top = frame.at(0.0, mid, 2.6);
    let hood = lib.turned(
        &[
            (0.0, 0.0),
            (1.05, 0.0),
            (1.05, 0.06),
            (0.97, 0.1),
            (0.62, 0.42),
            (0.34, 0.7),
            (0.24, 0.8),
            (0.24, 0.88),
            (0.0, 0.88),
        ],
        48,
    );
    lib.decor(hood.at(top, Quat::IDENTITY, kind::METAL, COPPER));
    let flue = lib.shape(Mesh::cylinder(32));
    let (y0, y1) = (top.y + 0.88, frame.y + RoomFrame::HEIGHT);
    let at = |y: f32| Vec3::new(top.x, y, top.z);
    lib.decor(flue.sized(Vec3::new(0.38, y1 - y0, 0.38)).inst(
        at((y0 + y1) * 0.5),
        Quat::IDENTITY,
        kind::METAL,
        COPPER,
    ));
    for y in [y0 + 0.4, frame.y + 4.4, y1 - 0.3] {
        lib.decor(flue.sized(Vec3::new(0.43, 0.06, 0.43)).inst(
            at(y),
            Quat::IDENTITY,
            kind::METAL,
            [0.5, 0.25, 0.1, 1.0],
        ));
    }
}

/// A long workbench down the nave on side `s`: legs and a shelf under, and on it a
/// copper alembic over a brazier, a flask bubbling over a spirit lamp, glassware, a
/// rack of tubes, a mortar, books, a balance and a candle.
fn bench(lib: &mut Library, frame: &RoomFrame, s: f32, glass: &Glassware) {
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
    let (up, out) = (Vec3::Y, frame.out());
    let mut rng = StdRng::seed_from_u64(0xbe4c_0000 ^ (s > 0.0) as u64);
    let stick = lib.shape(Mesh::cylinder(12));
    // The alembic: an iron brazier with its fire, the copper gourd on it, the head
    // over that and its beak running down to a glass receiver.
    let brazier = at(0.0, o0 + 0.8, 0.0);
    let bowl = lib.turned(
        &[
            (0.0, 0.0),
            (0.09, 0.0),
            (0.09, 0.015),
            (0.05, 0.03),
            (0.05, 0.07),
            (0.15, 0.09),
            (0.17, 0.14),
            (0.155, 0.14),
            (0.0, 0.11),
        ],
        32,
    );
    lib.decor(bowl.at(brazier, Quat::IDENTITY, kind::METAL, IRON));
    let coals = stick.sized(Vec3::new(0.29, 0.01, 0.29)).inst(
        brazier + up * 0.113,
        Quat::IDENTITY,
        kind::EMISSIVE,
        [1.0, 0.3, 0.06, 3.0],
    );
    lib.decor(Inst { param: 1, ..coals });
    lib.translucent(Inst::new(
        brazier + up * 0.15,
        Quat::from_rotation_y(0.6),
        Vec3::new(0.2, 0.07, 0.2),
        kind::FIRE,
        [1.0, 0.5, 0.15, 2.0],
        1,
    ));
    let gourd = brazier + up * 0.135;
    let still = lib.turned(
        &[
            (0.0, 0.0),
            (0.08, 0.0),
            (0.14, 0.04),
            (0.165, 0.12),
            (0.15, 0.22),
            (0.09, 0.3),
            (0.065, 0.36),
            (0.075, 0.4),
            (0.0, 0.4),
        ],
        32,
    );
    lib.decor(still.at(gourd, Quat::IDENTITY, kind::METAL, COPPER));
    let head = gourd + up * 0.4;
    let cap = lib.turned(
        &[
            (0.0, 0.0),
            (0.075, 0.0),
            (0.12, 0.03),
            (0.125, 0.07),
            (0.1, 0.12),
            (0.05, 0.16),
            (0.02, 0.18),
            (0.0, 0.19),
        ],
        32,
    );
    lib.decor(cap.at(head, Quat::IDENTITY, kind::METAL, COPPER));
    let receiver = at(0.05, o0 + 1.6, 0.0);
    glass.cork(lib, receiver, 0.2);
    let foot = receiver + up * 0.02;
    glass.stand(lib, glass.round, foot, 0.0, 0.2, [1.0, 0.6, 0.15, 0.5]);
    // The beak, drooping a little from the head into the receiver's neck.
    let toward = (receiver - head).with_y(0.0).normalize();
    let spout = head + up * 0.05 + toward * 0.11;
    let mouth = foot + up * 0.31;
    let droop = (spout + mouth) * 0.5 + up * 0.03;
    let beak: Vec<Vec3> = (0..=8)
        .map(|k| {
            let t = k as f32 / 8.0;
            spout * ((1.0 - t) * (1.0 - t)) + droop * (2.0 * t * (1.0 - t)) + mouth * (t * t) - spout
        })
        .collect();
    let beak = lib.shape(Mesh::sweep(&beak, &[1.0, 0.6], &circle(0.022, 10), true));
    lib.decor(beak.at(spout, Quat::IDENTITY, kind::METAL, COPPER));
    // A flask bubbling on a tripod over a spirit lamp, its steam rising.
    let lamp = at(-0.22, o0 + 2.2, 0.0);
    let burner = lib.turned(
        &[
            (0.0, 0.0),
            (0.05, 0.0),
            (0.055, 0.01),
            (0.055, 0.04),
            (0.035, 0.055),
            (0.012, 0.062),
            (0.0, 0.062),
        ],
        24,
    );
    lib.decor(burner.at(lamp, Quat::IDENTITY, kind::METAL, BRASS_LIT));
    let wick = stick.sized(Vec3::new(0.008, 0.016, 0.008));
    lib.decor(wick.inst(lamp + up * 0.068, Quat::IDENTITY, kind::PLAIN, [0.1, 0.08, 0.06, 1.0]));
    lib.translucent(Inst::new(
        lamp + up * 0.1,
        Quat::IDENTITY,
        Vec3::new(0.022, 0.055, 0.022),
        kind::FIRE,
        [1.0, 0.6, 0.25, 2.0],
        0,
    ));
    lib.light(lamp + up * 0.12, 2.2, [1.0, 0.65, 0.3], 0.35, LightKind::Flicker);
    let hoop = lamp + up * 0.2;
    let ring = lib.shape(Mesh::torus(0.12, 32, 8)).scaled(0.055);
    lib.decor(ring.at(hoop, Quat::IDENTITY, kind::METAL, IRON));
    for k in 0..3 {
        let t = k as f32 / 3.0 * TAU + 0.4;
        let d = Vec3::new(libm::cosf(t), 0.0, libm::sinf(t));
        lib.decor(stick.between(hoop + d * 0.055, lamp + d * 0.1, 0.008, kind::METAL, IRON));
    }
    let w = 0.13;
    let flask = lamp + up * 0.17;
    let brew = POTIONS[if s > 0.0 { 2 } else { 0 }];
    glass.stand(lib, glass.round, flask, 0.0, w, rgba(brew, 1.0));
    let steam = stick.sized(Vec3::new(0.1, 0.5, 0.1));
    lib.translucent(steam.inst(
        flask + up * (w * 1.64 + 0.27),
        Quat::IDENTITY,
        kind::MIST,
        [0.85, 0.88, 0.9, 0.5],
    ));
    // Flasks, a bottle and a retort, some with something in them that glows.
    let vessels = [
        (glass.cone, 0.12),
        (glass.round, 0.11),
        (glass.bottle, 0.08),
        (glass.retort, 0.12),
    ];
    for k in 0..4 {
        let o = o0 + 2.6 + k as f32 * 0.24 + rng.gen_range(-0.03..0.03);
        let da = rng.gen_range(-0.2..0.25f32);
        let (vessel, w) = vessels[(k + (s > 0.0) as usize) % vessels.len()];
        let rgb = POTIONS[(k + (s > 0.0) as usize * 3) % POTIONS.len()];
        let glow = if k % 2 == 0 { 1.0 } else { 0.12 };
        let mut foot = at(da, o, 0.0);
        // Round-bottomed ones stand in cork rings.
        if vessel[1] == glass.round[1] {
            glass.cork(lib, foot, w);
            foot += up * (0.1 * w);
        }
        glass.stand(lib, vessel, foot, rng.gen_range(0.0..TAU), w, rgba(rgb, glow));
    }
    // A rack of tubes.
    let rack = at(-0.2, o0 + 3.3, 0.08);
    lib.decor(Inst::aabox(rack, frame.size(0.12, 0.16, 0.5), kind::WOOD, WALNUT));
    for k in 0..5 {
        let p = rack + out * (-0.2 + k as f32 * 0.1);
        let rgb = POTIONS[(k * 2 + 1) % POTIONS.len()];
        glass.stand(lib, glass.tube, p - up * 0.05, 0.0, 0.035, rgba(rgb, 0.3));
    }
    // A mortar and pestle.
    let mortar = at(0.2, o0 + 3.9, 0.0);
    let bowl = lib.turned(
        &[
            (0.0, 0.0),
            (0.07, 0.0),
            (0.09, 0.015),
            (0.1, 0.06),
            (0.105, 0.1),
            (0.085, 0.1),
            (0.07, 0.05),
            (0.0, 0.035),
        ],
        24,
    );
    lib.decor(bowl.at(mortar, Quat::IDENTITY, kind::STONE, [0.5, 0.47, 0.42, 1.0]));
    let pestle = lib.turned(
        &[
            (0.0, 0.0),
            (0.022, 0.0),
            (0.026, 0.02),
            (0.02, 0.06),
            (0.013, 0.2),
            (0.018, 0.24),
            (0.0, 0.25),
        ],
        16,
    );
    let lean = Quat::from_rotation_arc(Vec3::Y, Vec3::new(0.3, 1.0, 0.2).normalize());
    lib.decor(pestle.at(mortar + up * 0.04, lean, kind::STONE, [0.55, 0.5, 0.45, 1.0]));
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
    // A brass balance, its beam rocking a hair and the pans hanging level from it.
    let scale = at(-0.2, o0 + 5.2, 0.0);
    let pillar = lib.turned(
        &[
            (0.0, 0.0),
            (0.07, 0.0),
            (0.07, 0.015),
            (0.03, 0.03),
            (0.012, 0.05),
            (0.012, 0.39),
            (0.02, 0.4),
            (0.0, 0.41),
        ],
        20,
    );
    lib.decor(pillar.at(scale, Quat::IDENTITY, kind::METAL, BRASS_LIT));
    let pivot = scale + up * 0.4;
    let beam = out * 0.2;
    let phase = if s > 0.0 { 4.0 } else { 0.0 };
    let rock = |axis: Vec3| Motion::Swing {
        axis,
        amp: 0.05,
        period: 9.0,
    };
    let m = lib.mover(pivot, rock(frame.across()), phase, None);
    lib.moving(
        m,
        stick.between(pivot - beam, pivot + beam, 0.014, kind::METAL, BRASS_LIT),
    );
    lib.moving(
        m,
        stick.between(pivot - up * 0.01, pivot + up * 0.09, 0.008, kind::METAL, BRASS_LIT),
    );
    let dish = lib.turned(
        &[
            (0.0, 0.0),
            (0.03, 0.0),
            (0.06, 0.012),
            (0.065, 0.02),
            (0.058, 0.02),
            (0.0, 0.008),
        ],
        24,
    );
    for e in [-1.0f32, 1.0] {
        let hook = pivot + beam * e;
        let pan = lib.mover(hook, rock(-frame.across()), phase, Some(m));
        let low = hook - up * 0.18;
        lib.moving(pan, stick.between(low, hook, 0.005, kind::METAL, BRASS_LIT));
        lib.moving(pan, dish.at(low, Quat::IDENTITY, kind::METAL, BRASS_LIT));
    }
    // A candle in its stick.
    let candle = at(0.3, o0 + 0.25, 0.0);
    let holder = lib.turned(
        &[
            (0.0, 0.0),
            (0.045, 0.0),
            (0.05, 0.01),
            (0.045, 0.015),
            (0.015, 0.02),
            (0.015, 0.04),
            (0.022, 0.045),
            (0.0, 0.045),
        ],
        20,
    );
    lib.decor(holder.at(candle, Quat::IDENTITY, kind::METAL, BRASS_LIT));
    let wax = lib.turned(&[(0.0, 0.0), (0.02, 0.0), (0.02, 0.15), (0.015, 0.16), (0.0, 0.16)], 16);
    lib.decor(wax.at(candle + up * 0.045, Quat::IDENTITY, kind::PLAIN, [0.8, 0.75, 0.6, 1.0]));
    lib.translucent(Inst::new(
        candle + up * 0.235,
        Quat::IDENTITY,
        Vec3::new(0.022, 0.05, 0.022),
        kind::FIRE,
        [1.0, 0.6, 0.25, 2.0],
        0,
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
/// down into a worm: a coil standing in a tub of water on the bench, the pipe out at
/// its foot and down to a flask.
fn pipes(lib: &mut Library, frame: &RoomFrame, glass: &Glassware) {
    let y = RoomFrame::HEIGHT - 0.42;
    let flue = (FURNACE_O.0 + FURNACE_O.1) * 0.5;
    let turn = BENCH_O.1 - 0.45;
    let (u, v, up) = (frame.across(), frame.out(), Vec3::Y);
    let stick = lib.shape(Mesh::cylinder(16));
    let tub = lib.turned(
        &[
            (0.0, 0.0),
            (0.23, 0.0),
            (0.25, 0.02),
            (0.27, 0.28),
            (0.26, 0.55),
            (0.24, 0.55),
            (0.235, 0.5),
            (0.0, 0.5),
        ],
        40,
    );
    let hoop = lib.shape(Mesh::torus(0.06, 40, 8));
    let (r, turns) = (0.17, 3.5);
    for s in [-1.0f32, 1.0] {
        let base = frame.at(s * BENCH_A, turn, 0.94);
        // Round and round from the top, ending toward the door.
        let coil = |t: f32| {
            let th = FRAC_PI_2 + s * t * TAU;
            base + (u * libm::cosf(th) + v * libm::sinf(th)) * r + up * (0.92 - 0.12 * t)
        };
        let mut pts = vec![
            frame.at(s * 0.12, flue, 4.4),
            frame.at(s * 1.0, flue, 4.4),
            frame.at(s * 1.0, flue, y),
            frame.at(s * BENCH_A, flue, y),
            frame.at(s * BENCH_A, turn + r, y),
        ];
        let n = (turns * 8.0) as usize;
        pts.extend((0..=n).map(|k| coil(k as f32 / 8.0)));
        let spout = base - v * 0.37 + up * 0.2;
        pts.extend([base - v * 0.34 + up * 0.5, spout]);
        let path: Vec<Vec3> = rounded(&pts, 0.25).iter().map(|p| *p - base).collect();
        let pipe = lib.shape(Mesh::sweep(&path, &[1.0], &circle(0.045, 12), true));
        lib.decor(pipe.at(base, Quat::IDENTITY, kind::METAL, COPPER));
        // A coupling halfway along each straight run.
        for w in pts[..5].windows(2) {
            let (mid, d) = ((w[0] + w[1]) * 0.5, (w[1] - w[0]).normalize());
            lib.decor(stick.between(mid - d * 0.03, mid + d * 0.03, 0.12, kind::METAL, [0.5, 0.25, 0.1, 1.0]));
        }
        // The tub, hooped, its water dark.
        lib.decor(tub.at(base, Quat::IDENTITY, kind::WOOD, [0.26, 0.15, 0.07, 1.0]));
        for (h, rr) in [(0.08f32, 0.255f32), (0.44, 0.27)] {
            lib.decor(hoop.scaled(rr).at(base + up * h, Quat::IDENTITY, kind::METAL, IRON));
        }
        let water = stick.sized(Vec3::new(0.47, 0.01, 0.47));
        lib.decor(water.inst(base + up * 0.505, Quat::IDENTITY, kind::CRYSTAL, [0.05, 0.09, 0.1, 0.0]));
        // The flask it drips into.
        let flask = spout.with_y(base.y);
        glass.cork(lib, flask, 0.1);
        glass.stand(lib, glass.round, flask + up * 0.01, 0.0, 0.1, [0.85, 0.75, 0.35, 0.3]);
    }
}

/// A transmutation circle let into the floor before the athanor, its lines faintly
/// alight: a circle, a square in it, a triangle in that, a circle in that; and over it,
/// out of reach, a red stone turning slowly as it floats.
fn transmutation(lib: &mut Library, frame: &RoomFrame) {
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
    for radius in [r, triangle] {
        let ring = band(lib, c, (u, v), (radius, 0.03, 0.006), kind::EMISSIVE, glow);
        lib.decor(Inst { param: 1, ..ring });
    }
    for k in 0..4 {
        let t = q + k as f32 * TAU / 4.0;
        line(lib, pt(r, t), pt(r, t + TAU / 4.0));
    }
    for k in 0..3 {
        let t = -TAU / 4.0 + k as f32 * TAU / 3.0;
        line(lib, pt(square, t), pt(square, t + TAU / 3.0));
    }
    let stone = frame.at(0.0, CIRCLE_O, 2.6);
    let float = Motion::Bob {
        amp: 0.06,
        period: 7.0,
        rock: 0.0,
    };
    let float = lib.mover(stone, float, 0.0, None);
    lib.light_riding(float, stone, 3.5, [1.0, 0.3, 0.2], 0.45, LightKind::Static);
    let spin = lib.mover(
        stone,
        Motion::Spin {
            axis: Vec3::Y,
            rate: 0.25,
        },
        0.0,
        Some(float),
    );
    // A cube stood on its corner.
    let corner = Quat::from_rotation_x(0.6155) * Quat::from_rotation_z(std::f32::consts::FRAC_PI_4);
    lib.moving(
        spin,
        Inst::new(
            stone,
            corner,
            Vec3::splat(0.13),
            kind::CRYSTAL,
            [0.95, 0.12, 0.08, 1.6],
            0,
        ),
    );
}
