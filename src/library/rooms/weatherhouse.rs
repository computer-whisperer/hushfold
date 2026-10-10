//! The Weatherhouse: department 10, top floor, south side. A long nave between two
//! rows of cases, wall cases down the side walls behind them and either side of a
//! tall window with rain running down it; a skylight over the nave. A wind rose is
//! set in the floor; a weathervane turning to the wind now and then, a table of
//! instruments and a cloud kept in a glass jar stand on it; wind chimes hang over
//! it, stirring; barometers, thermometers and a weather map hang by the door. The
//! cloud glows, lit now and then from inside, and a snowflake that will not melt
//! turns under a bell of glass.

use super::hearth_library::{hurricane_lamp, turned_leg};
use super::*;
use std::f32::consts::{FRAC_PI_2, PI};

/// The rows of cases down the room: how far either side of the middle, and where
/// along it the first one stands.
const ROW_A: f32 = RoomFrame::A_HALF - WALL_D - 3.0 - UNIT_DEPTH * 0.5;
const ROW_O: f32 = 17.3;
/// A wall case's depth from the wall to its face.
const WALL_D: f32 = 0.04 + FACE_DEPTH;
/// The great window's half-width.
const WINDOW: f32 = 1.3;
const BRASS: [f32; 3] = [0.7, 0.5, 0.2];
const IRON: [f32; 4] = [0.12, 0.12, 0.13, 1.0];
const PALE_STONE: [f32; 3] = [0.48, 0.45, 0.4];
const STORM_GLASS: [f32; 3] = [0.3, 0.38, 0.62];
/// Clear glass, as the instruments' tubes and bells are made of.
const CLEAR_GLASS: [f32; 4] = [0.85, 0.9, 1.0, 0.08];
/// Lettering inlaid in the floor: the wind rose's quarters.
const ROSE_LETTERS: Style = Style {
    em: 0.32,
    tracking: 0.0,
    lettering: Lettering::Gilt,
    color: GOLD_LEAF,
};

pub(super) fn furnish(lib: &mut Library, frame: &RoomFrame, rng: &mut StdRng, titles: &mut HashSet<String>) {
    debug_assert!(frame.door.abs() < 1.0, "the Weatherhouse wants its door in the middle");
    shelves(lib, frame, rng, titles);
    great_window(lib, frame);
    near_wall(lib, frame);
    wind_rose(lib, frame, 0.0, 20.6);
    cloud_jar(lib, frame, 0.0, 25.4);
    weathervane(lib, frame, 4.6, 22.6);
    instrument_table(lib, frame, -4.4, 22.6);
    for (k, (a, o)) in [(-2.6, 28.6), (2.6, 28.6), (0.0, 17.6)].into_iter().enumerate() {
        wind_chime(lib, frame, a, o, k as f32);
    }
    umbrella_stand(lib, frame, -3.5, 14.62);
    ceiling(lib, frame);
}

/// Six wall cases down each side wall, a row of five double-sided cases in front of
/// each (a category on either side of it), and four wall cases either side of the
/// window.
fn shelves(lib: &mut Library, frame: &RoomFrame, rng: &mut StdRng, titles: &mut HashSet<String>) {
    let (out, across) = (frame.out(), frame.across());
    let mut cats = frame.categories.clone();
    for s in [-1.0f32, 1.0] {
        let wall: Vec<_> = (0..6)
            .map(|k| {
                let o = RoomFrame::O_NEAR + UNIT_PITCH * (k as f32 + 0.5);
                lib.wall_bookcase(frame.at(s * RoomFrame::A_HALF, o, 0.0), -across * s)
            })
            .collect();
        lib.add_category(rng, cats.next().unwrap(), &left_to_right(wall), titles);
        let (mut outer, mut inner) = (Vec::new(), Vec::new());
        for k in 0..5 {
            let c = frame.at(s * ROW_A, ROW_O + k as f32 * UNIT_PITCH, 0.0);
            for f in lib.bookcase(c, frame.out_along_x()) {
                if f.1.dot(across) * s > 0.0 {
                    outer.push(f)
                } else {
                    inner.push(f)
                }
            }
        }
        lib.add_category(rng, cats.next().unwrap(), &left_to_right(outer), titles);
        lib.add_category(rng, cats.next().unwrap(), &left_to_right(inner), titles);
    }
    for s in [-1.0f32, 1.0] {
        let far: Vec<_> = (0..4)
            .map(|k| {
                let a = s * (WINDOW + 0.21 + UNIT_PITCH * (k as f32 + 0.5));
                lib.wall_bookcase(frame.at(a, RoomFrame::O_FAR, 0.0), -out)
            })
            .collect();
        lib.add_category(rng, cats.next().unwrap(), &left_to_right(far), titles);
    }
}

/// The tall window in the far wall, rain running down it and beading on it, a stone
/// surround and a deep sill, the moon's light falling through.
fn great_window(lib: &mut Library, frame: &RoomFrame) {
    let out = frame.out();
    let n = -out;
    let wall = RoomFrame::O_FAR;
    let (y0, y1) = (0.75, 5.1);
    let c = frame.at(0.0, wall - 0.02, (y0 + y1) * 0.5);
    let mut glass = Inst::aabox(c, frame.size(2.0 * WINDOW, y1 - y0, 0.08), kind::GLASS, STORM_GLASS);
    glass.color[3] = 0.85;
    lib.decor(glass);
    // The surround: jambs, a head and a sill deep enough to sit on.
    for s in [-1.0f32, 1.0] {
        let x = s * (WINDOW + 0.1);
        lib.decor(piece(
            frame,
            (x - 0.1, x + 0.1),
            (wall - 0.2, wall),
            (0.0, y1 + 0.15),
            kind::STONE,
            PALE_STONE,
        ));
    }
    lib.decor(piece(
        frame,
        (-WINDOW - 0.25, WINDOW + 0.25),
        (wall - 0.24, wall),
        (y1, y1 + 0.3),
        kind::STONE,
        PALE_STONE,
    ));
    lib.solid(
        piece(
            frame,
            (-WINDOW, WINDOW),
            (wall - 0.45, wall),
            (0.0, y0 - 0.06),
            kind::STONE,
            PALE_STONE,
        ),
        Stops::ALL,
    );
    lib.decor(piece(
        frame,
        (-WINDOW - 0.05, WINDOW + 0.05),
        (wall - 0.5, wall),
        (y0 - 0.06, y0),
        kind::STONE,
        [0.52, 0.49, 0.44],
    ));
    // Rain running down the glass, and drops beaded on it catching the moonlight.
    lib.translucent(Inst::new(
        frame.at(0.0, wall - 0.075, (y0 + y1) * 0.5),
        frame.turn(),
        Vec3::new(2.0 * WINDOW, y1 - y0, 0.02),
        kind::MIST,
        [0.6, 0.68, 0.85, 0.5],
        2,
    ));
    let mut rng = StdRng::seed_from_u64(0x5241_494e);
    let turn = frame.turn();
    for k in 0..30 {
        let a = rng.gen_range(-WINDOW + 0.05..WINDOW - 0.05);
        let y = rng.gen_range(y0 + 0.1..y1 - 0.1);
        let r = rng.gen_range(0.012..0.026);
        let p = frame.at(a, wall - 0.066 - 0.002 * (k % 3) as f32, y);
        let bead = Inst::new(
            p,
            turn * Quat::from_rotation_z(0.785),
            Vec3::new(r, r, 0.004),
            kind::EMISSIVE,
            [0.75, 0.85, 1.0, 1.4],
            0,
        );
        lib.decor(bead);
    }
    lib.light(c + n * 1.8, 9.0, [0.45, 0.55, 1.0], 0.9, LightKind::Static);
    let beam = (n * 1.0 - Vec3::Y * 0.55).normalize();
    let len = 8.0;
    lib.static_trans.push(Inst::new(
        c + beam * len * 0.5,
        Quat::from_rotation_arc(Vec3::Y, beam),
        Vec3::new(2.2, len, 3.5),
        kind::SHAFT,
        [0.35, 0.45, 0.9, 0.04],
        0,
    ));
}

/// Panelling along the near wall; a weather map on it to one side of the door, and
/// barometers and thermometers to the other.
fn near_wall(lib: &mut Library, frame: &RoomFrame) {
    let near = RoomFrame::O_NEAR;
    let inner = RoomFrame::A_HALF - WALL_D - 0.06;
    let d = frame.door;
    for (a0, a1) in [
        (-inner, d - RoomFrame::DOOR_HALF - 0.32),
        (d + RoomFrame::DOOR_HALF + 0.32, inner),
    ] {
        lib.wainscot(frame.y, frame.rect(a0, a1, near, near + 0.07), frame.side.out);
    }
    let n = frame.out();
    // The map: a sheet of isobars in a frame.
    let (mc, mw, mh) = (d - 7.3, 3.4, 2.1);
    let my = 2.55;
    lib.decor(piece(
        frame,
        (mc - mw * 0.5 - 0.08, mc + mw * 0.5 + 0.08),
        (near, near + 0.05),
        (my - mh * 0.5 - 0.08, my + mh * 0.5 + 0.08),
        kind::WOOD,
        WALNUT,
    ));
    lib.decor(piece(
        frame,
        (mc - mw * 0.5, mc + mw * 0.5),
        (near + 0.05, near + 0.06),
        (my - mh * 0.5, my + mh * 0.5),
        kind::PARCHMENT,
        [0.8, 0.72, 0.55],
    ));
    let ink = rgba([0.18, 0.14, 0.12], 1.0);
    let sheet = |u: f32, v: f32| frame.at(mc + u, near + 0.065, my + v);
    let face = Quat::from_mat3(&Mat3::from_cols(Vec3::Y.cross(n), Vec3::Y, n));
    // Rings round a low and a high: inked ellipses.
    for (cu, cv, rings) in [
        (-0.7f32, 0.2f32, &[0.22f32, 0.42, 0.62, 0.82][..]),
        (0.95, -0.25, &[0.18, 0.36, 0.54][..]),
    ] {
        for &r in rings {
            let oval = |r: f32| -> Vec<Vec2> {
                circle(1.0, 48)
                    .into_iter()
                    .map(|p| p * Vec2::new(r, r * 0.75))
                    .collect()
            };
            let isobar = lib.shape(Mesh::ring(&oval(r + 0.006), &oval(r - 0.006), 0.004));
            lib.decor(isobar.at(sheet(cu, cv), face, kind::PLAIN, ink));
        }
    }
    // A front: a line across with teeth along it, inked over the rings.
    for k in 0..7 {
        let u = -1.5 + k as f32 * 0.45;
        let (p, q) = (
            sheet(u, -0.75 + 0.18 * libm::sinf(u * 1.3)) + n * 0.003,
            sheet(u + 0.45, -0.75 + 0.18 * libm::sinf((u + 0.45) * 1.3)) + n * 0.003,
        );
        rod_flat(lib, p, q, n, 0.016, rgba([0.2, 0.25, 0.6], 1.0));
        let mid = (p + q) * 0.5 + Vec3::Y * 0.05;
        let tooth = Inst::new(
            mid,
            face * Quat::from_rotation_z(0.785),
            Vec3::new(0.06, 0.06, 0.004),
            kind::PLAIN,
            [0.2, 0.25, 0.6, 1.0],
            0,
        );
        lib.decor(tooth);
    }
    // Barometers and thermometers to the other side.
    for (k, a) in [d + 3.6, d + 6.0, d + 8.4].into_iter().enumerate() {
        barometer(lib, frame.at(a, near + 0.02, 2.45 + 0.15 * (k % 2) as f32), n, k as f32);
    }
    for a in [d + 4.8, d + 7.2] {
        thermometer(lib, frame.at(a, near + 0.02, 2.3), n);
    }
}

/// A thin flat strip from `p` to `q` lying on a wall facing `n`.
fn rod_flat(lib: &mut Library, p: Vec3, q: Vec3, n: Vec3, w: f32, col: [f32; 4]) {
    let v = q - p;
    let x = v.normalize();
    let rot = Quat::from_mat3(&Mat3::from_cols(x, n.cross(x), n));
    lib.decor(Inst::new(
        (p + q) * 0.5,
        rot,
        Vec3::new(v.length() + w * 0.5, w, 0.004),
        kind::PLAIN,
        col,
        0,
    ));
}

/// A banjo barometer on a wall at `c` (the middle of its dial), facing `n`: a dial
/// in a brass bezel under a long neck with a thermometer in it; `seed` sets its
/// needle.
fn barometer(lib: &mut Library, c: Vec3, n: Vec3, seed: f32) {
    let side = Vec3::Y.cross(n);
    let face = Quat::from_mat3(&Mat3::from_cols(side, Vec3::Y, n));
    let wood = rgba([0.32, 0.15, 0.06], 1.0);
    let flat = Quat::from_rotation_arc(Vec3::Y, n);
    // The round case, its dial and bezel.
    let disc = lib.shape(Mesh::cylinder(40));
    lib.decor(
        disc.sized(Vec3::new(0.44, 0.04, 0.44))
            .inst(c + n * 0.02, flat, kind::WOOD, wood),
    );
    lib.decor(
        disc.sized(Vec3::new(0.3, 0.01, 0.3))
            .inst(c + n * 0.045, flat, kind::PAPER, [0.0; 4]),
    );
    let bezel = lib.shape(Mesh::torus(0.12, 40, 8));
    lib.decor(
        bezel
            .sized(Vec3::new(0.33, 0.02, 0.33))
            .inst(c + n * 0.05, flat, kind::METAL, rgba(BRASS, 1.0)),
    );
    let needle = face * Quat::from_rotation_z(1.2 + seed * 0.7);
    lib.decor(Inst::new(
        c + n * 0.055,
        needle,
        Vec3::new(0.012, 0.26, 0.006),
        kind::METAL,
        [0.05, 0.05, 0.06, 1.0],
        0,
    ));
    let boss = lib.shape(Mesh::dome(12));
    lib.decor(
        boss.sized(Vec3::new(0.035, 0.012, 0.035))
            .inst(c + n * 0.062, flat, kind::METAL, rgba(BRASS, 1.0)),
    );
    // The neck, its thermometer, and a finial.
    let neck = c + Vec3::Y * 0.52;
    lib.decor(Inst::new(
        neck + n * 0.02,
        face,
        Vec3::new(0.12, 0.62, 0.04),
        kind::WOOD,
        wood,
        0,
    ));
    lib.decor(Inst::new(
        neck + n * 0.042,
        face,
        Vec3::new(0.05, 0.46, 0.006),
        kind::PAPER,
        [0.0; 4],
        0,
    ));
    let mercury = neck + n * 0.047 - Vec3::Y * 0.08;
    lib.decor(Inst::new(
        mercury,
        face,
        Vec3::new(0.012, 0.28, 0.006),
        kind::EMISSIVE,
        [0.8, 0.1, 0.06, 0.6],
        0,
    ));
    let finial = lib.turned(
        &[
            (0.0, 0.0),
            (0.5, 0.0),
            (0.5, 0.2),
            (0.25, 0.35),
            (0.4, 0.6),
            (0.15, 0.85),
            (0.0, 1.0),
        ],
        12,
    );
    lib.decor(finial.sized(Vec3::new(0.09, 0.14, 0.09)).at(
        c + Vec3::Y * 0.83 + n * 0.02,
        Quat::IDENTITY,
        kind::WOOD,
        wood,
    ));
}

/// A thermometer on a board at `c`, facing `n`.
fn thermometer(lib: &mut Library, c: Vec3, n: Vec3) {
    let side = Vec3::Y.cross(n);
    let face = Quat::from_mat3(&Mat3::from_cols(side, Vec3::Y, n));
    lib.decor(Inst::new(
        c + n * 0.015,
        face,
        Vec3::new(0.16, 0.9, 0.03),
        kind::WOOD,
        rgba(WALNUT, 1.0),
        0,
    ));
    lib.decor(Inst::new(
        c + n * 0.032,
        face,
        Vec3::new(0.1, 0.8, 0.004),
        kind::PARCHMENT,
        [0.85, 0.8, 0.65, 1.0],
        0,
    ));
    let tube = lib.shape(Mesh::cylinder(10));
    lib.decor(tube.sized(Vec3::new(0.018, 0.68, 0.018)).inst(
        c + n * 0.045 + Vec3::Y * 0.02,
        Quat::IDENTITY,
        kind::CRYSTAL,
        [0.5, 0.55, 0.6, 0.15],
    ));
    lib.decor(Inst::new(
        c + n * 0.051 - Vec3::Y * 0.12,
        face,
        Vec3::new(0.008, 0.44, 0.006),
        kind::EMISSIVE,
        [0.85, 0.1, 0.06, 0.7],
        0,
    ));
    let bulb = lib.shape(Mesh::sphere(12));
    lib.decor(bulb.sized(Vec3::splat(0.035)).inst(
        c + n * 0.045 - Vec3::Y * 0.34,
        Quat::IDENTITY,
        kind::EMISSIVE,
        [0.85, 0.1, 0.06, 0.7],
    ));
}

/// A wind rose inlaid in the floor round `a`, `o`: a brass ring and eight points,
/// each half light and half dark, the north one picked out, and the quarters'
/// letters in gold.
fn wind_rose(lib: &mut Library, frame: &RoomFrame, a: f32, o: f32) {
    let c = frame.at(a, o, 0.0);
    let r = 2.0;
    let level = Quat::from_rotation_arc(Vec3::Z, Vec3::Y);
    let ring = lib.shape(Mesh::ring(&circle(r + 0.03, 96), &circle(r - 0.03, 96), 0.006));
    lib.decor(ring.inst(c + Vec3::Y * 0.004, level, kind::METAL, rgba(BRASS, 1.0)));
    // Each point is two triangles either side of its spine, laid flat (made in xy,
    // turned so z is up and x runs out along the point).
    let half = |lib: &mut Library, w: f32, side: f32| {
        lib.shape(Mesh::prism(
            &[Vec2::ZERO, Vec2::new(1.0, 0.0), Vec2::new(0.18, side * w)],
            0.004,
        ))
    };
    for k in 0..8 {
        let th = 0.01 + k as f32 / 8.0 * TAU;
        let radial = frame.across() * libm::cosf(th) + frame.out() * libm::sinf(th);
        let long = k % 2 == 0;
        let (len, w) = if long { (r * 1.25, 0.13) } else { (r * 0.75, 0.08) };
        let lay = Quat::from_mat3(&Mat3::from_cols(radial, Vec3::Y.cross(radial), Vec3::Y));
        let (light, dark) = if k == 2 {
            ([0.75, 0.18, 0.12], [0.45, 0.08, 0.05])
        } else if long {
            ([0.7, 0.58, 0.36], [0.36, 0.26, 0.14])
        } else {
            ([0.55, 0.42, 0.24], [0.28, 0.19, 0.1])
        };
        for (side, col) in [(1.0f32, light), (-1.0, dark)] {
            let shape = half(lib, w / len, side);
            lib.decor(
                shape
                    .sized(Vec3::new(len, w, 0.004))
                    .at(c + Vec3::Y * 0.003, lay, kind::WOOD, rgba(col, 1.0)),
            );
        }
    }
    let boss = lib.shape(Mesh::cylinder(24));
    lib.decor(boss.sized(Vec3::new(0.22, 0.01, 0.22)).inst(
        c + Vec3::Y * 0.006,
        Quat::IDENTITY,
        kind::METAL,
        rgba(BRASS, 1.0),
    ));
    // The quarters, read from the door: north out toward the window.
    let (right, up) = (frame.across(), frame.out());
    for (letter, dir) in [("N", up), ("E", right), ("S", -up), ("W", -right)] {
        let at = c + dir * (r * 1.25 + 0.3) + Vec3::Y * 0.002;
        text::set_line(&mut lib.text, letter, ROSE_LETTERS, at, right, up, 1.0);
    }
}

/// A glass jar on a stone pedestal at `a`, `o`, a small cloud kept inside it: the
/// room's bit of magic, glowing softly, a flicker of lightning in its belly.
fn cloud_jar(lib: &mut Library, frame: &RoomFrame, a: f32, o: f32) {
    let ped = 1.05;
    lib.decor(piece(
        frame,
        (a - 0.36, a + 0.36),
        (o - 0.36, o + 0.36),
        (0.0, 0.14),
        kind::STONE,
        PALE_STONE,
    ));
    lib.decor(piece(
        frame,
        (a - 0.24, a + 0.24),
        (o - 0.24, o + 0.24),
        (0.14, ped - 0.1),
        kind::STONE,
        [0.42, 0.39, 0.35],
    ));
    lib.decor(piece(
        frame,
        (a - 0.33, a + 0.33),
        (o - 0.33, o + 0.33),
        (ped - 0.1, ped),
        kind::STONE,
        PALE_STONE,
    ));
    guard(lib, frame, (a - 0.36, a + 0.36), (o - 0.36, o + 0.36), ped);
    let base = frame.at(a, o, ped);
    // A brass foot and lid, and the glass between them.
    let (r, h) = (0.26, 0.62);
    let brass = rgba(BRASS, 1.0);
    let foot = lib.shape(Mesh::lathe(
        &[
            (0.0, 0.0),
            (0.25, 0.0),
            (0.25, 0.025),
            (0.23, 0.045),
            (0.235, 0.06),
            (0.0, 0.06),
        ]
        .map(Vec2::from),
        40,
    ));
    lib.decor(foot.at(base, Quat::IDENTITY, kind::METAL, brass));
    let lid = lib.shape(Mesh::lathe(
        &[
            (0.0, 0.0),
            (0.24, 0.0),
            (0.24, 0.03),
            (0.2, 0.05),
            (0.06, 0.08),
            (0.05, 0.1),
            (0.045, 0.13),
            (0.0, 0.14),
        ]
        .map(Vec2::from),
        40,
    ));
    lib.decor(lid.at(base + Vec3::Y * (h + 0.06), Quat::IDENTITY, kind::METAL, brass));
    let jar = Mesh::lathe(
        &[
            (r - 0.02, 0.0),
            (r, 0.04),
            (r, h - 0.1),
            (r - 0.04, h - 0.03),
            (r - 0.05, h),
        ]
        .map(Vec2::from),
        48,
    );
    let jar = lib.shape(jar);
    lib.translucent(jar.at(
        base + Vec3::Y * 0.06,
        Quat::IDENTITY,
        kind::CLEAR,
        [0.75, 0.85, 1.0, 0.08],
    ));
    // The cloud: puffs of white, faintly lit from within, drifting in the jar.
    let cloud = base + Vec3::Y * 0.36;
    let drift = Motion::Bob {
        amp: 0.035,
        period: 7.0,
        rock: 0.15,
    };
    let m = lib.mover(cloud, drift, 0.0, None);
    let puff = lib.shape(Mesh::sphere(16));
    let puffs = [
        (Vec3::new(0.0, 0.0, 0.0), 0.2, 0.3),
        (Vec3::new(0.1, 0.04, 0.03), 0.15, 1.1),
        (Vec3::new(-0.11, 0.02, -0.02), 0.16, 0.7),
        (Vec3::new(0.02, 0.09, -0.06), 0.14, 1.9),
        (Vec3::new(-0.04, -0.06, 0.08), 0.13, 2.4),
        (Vec3::new(0.07, -0.05, -0.09), 0.12, 0.2),
    ];
    for (off, s, spin) in puffs {
        let q = Quat::from_rotation_y(spin) * Quat::from_rotation_x(spin * 0.7);
        let size = Vec3::new(s * 1.25, s, s * 1.1);
        lib.moving(
            m,
            puff.sized(size)
                .inst(cloud + off, q, kind::MIST, [0.8, 0.84, 0.95, 0.9]),
        );
    }
    // A fork of lightning in it.
    let bolt = [
        Vec3::new(0.02, -0.04, 0.0),
        Vec3::new(-0.03, -0.12, 0.01),
        Vec3::new(0.02, -0.17, 0.0),
        Vec3::new(-0.02, -0.27, 0.0),
    ];
    let fork = lib.shape(Mesh::sweep(
        &bolt.map(|p| cloud + p),
        &[0.008, 0.004],
        &circle(1.0, 8),
        true,
    ));
    lib.moving(
        m,
        fork.at(Vec3::ZERO, Quat::IDENTITY, kind::EMISSIVE, [0.85, 0.9, 1.0, 6.0]),
    );
    lib.light_riding(m, cloud, 3.5, [0.7, 0.8, 1.0], 0.7, LightKind::Flicker);
}

/// A gilt cockerel's outline, standing on the origin and facing +x, 1.5 times its
/// size in metres.
fn cockerel() -> Vec<Vec2> {
    [
        (-0.02, 0.0),
        (0.03, 0.0),
        (0.03, 0.05),
        (0.08, 0.07),
        (0.12, 0.12),
        (0.13, 0.17),
        (0.13, 0.19),
        (0.17, 0.225),
        (0.135, 0.245),
        (0.13, 0.27),
        (0.115, 0.285),
        (0.1, 0.27),
        (0.085, 0.29),
        (0.07, 0.265),
        (0.065, 0.235),
        (0.05, 0.19),
        (0.0, 0.15),
        (-0.05, 0.155),
        (-0.08, 0.26),
        (-0.11, 0.3),
        (-0.14, 0.285),
        (-0.12, 0.23),
        (-0.16, 0.21),
        (-0.17, 0.15),
        (-0.13, 0.11),
        (-0.09, 0.07),
        (-0.03, 0.055),
    ]
    .map(|(x, y)| Vec2::new(x, y) * 1.5)
    .to_vec()
}

/// A weathervane on a post at `a`, `o`: the arms of the four winds, and an arrow with
/// a cockerel on it turning above them to a wind that veers now one way, now the
/// other, the vane quivering as it holds.
fn weathervane(lib: &mut Library, frame: &RoomFrame, a: f32, o: f32) {
    let gilt = rgba(BRASS, 1.0);
    lib.decor(piece(
        frame,
        (a - 0.32, a + 0.32),
        (o - 0.32, o + 0.32),
        (0.0, 0.3),
        kind::STONE,
        PALE_STONE,
    ));
    guard(lib, frame, (a - 0.32, a + 0.32), (o - 0.32, o + 0.32), 0.3);
    let top = 2.5;
    lib.pole(frame.at(a, o, 0.3), frame.at(a, o, top + 0.25), 0.05, kind::METAL, IRON);
    let ball = lib.shape(Mesh::sphere(16));
    lib.decor(
        ball.sized(Vec3::splat(0.08))
            .inst(frame.at(a, o, top + 0.28), Quat::IDENTITY, kind::METAL, gilt),
    );
    let arms = 1.9;
    for (da, d_o) in [(1.0f32, 0.0f32), (0.0, 1.0)] {
        let (p, q) = (
            frame.at(a - da * 0.45, o - d_o * 0.45, arms),
            frame.at(a + da * 0.45, o + d_o * 0.45, arms),
        );
        lib.pole(p, q, 0.025, kind::METAL, IRON);
        for end in [p, q] {
            lib.decor(
                ball.sized(Vec3::splat(0.07))
                    .inst(end, Quat::IDENTITY, kind::METAL, gilt),
            );
        }
    }
    // The wind veers a twelfth of the way round now and then, and backs again more
    // seldom; the vane turns to it, overshooting a little, and quivers.
    let pivot = frame.at(a, o, top);
    let veer = Motion::Tick {
        axis: Vec3::Y,
        steps: 12,
        period: 12.0 * 19.0,
    };
    let back = Motion::Tick {
        axis: -Vec3::Y,
        steps: 12,
        period: 12.0 * 27.0,
    };
    let quiver = Motion::Swing {
        axis: Vec3::Y,
        amp: 0.05,
        period: 3.7,
    };
    let veering = lib.mover(pivot, veer, 0.0, None);
    let backing = lib.mover(pivot, back, 11.0, Some(veering));
    let m = lib.mover(pivot, quiver, 0.0, Some(backing));
    // The arrow, pointing off to the window's side of the room at rest. It points into
    // the wind, which blows through the room (`game`'s draught).
    let dir = (frame.out() * 0.8 - frame.across() * 0.6).normalize();
    lib.vane = Some((m, dir));
    let flat = Quat::from_mat3(&Mat3::from_cols(dir, Vec3::Y, dir.cross(Vec3::Y)));
    let collar = lib.pole_inst(pivot - Vec3::Y * 0.05, pivot + Vec3::Y * 0.05, 0.06, kind::METAL, gilt);
    lib.moving(m, collar);
    let shaft = lib.pole_inst(pivot - dir * 0.5, pivot + dir * 0.42, 0.022, kind::METAL, gilt);
    lib.moving(m, shaft);
    let head = lib.shape(Mesh::prism(
        &[Vec2::new(0.0, -0.06), Vec2::new(0.15, 0.0), Vec2::new(0.0, 0.06)],
        0.016,
    ));
    lib.moving(m, head.at(pivot + dir * 0.4, flat, kind::METAL, gilt));
    let tail = lib.shape(Mesh::prism(
        &[
            Vec2::new(0.0, -0.02),
            Vec2::new(-0.06, -0.12),
            Vec2::new(-0.22, -0.12),
            Vec2::new(-0.17, 0.0),
            Vec2::new(-0.22, 0.12),
            Vec2::new(-0.06, 0.12),
            Vec2::new(0.0, 0.02),
        ],
        0.012,
    ));
    lib.moving(m, tail.at(pivot - dir * 0.36, flat, kind::METAL, gilt));
    let bird = lib.shape(Mesh::prism(&cockerel(), 0.016));
    lib.moving(m, bird.at(pivot - dir * 0.12 + Vec3::Y * 0.01, flat, kind::METAL, gilt));
}

/// A table of instruments at `a`, `o` running outward: an anemometer turning, a rain
/// gauge, a storm glass, a lamp, and under a bell of glass a snowflake that will not
/// melt, slowly turning.
fn instrument_table(lib: &mut Library, frame: &RoomFrame, a: f32, o: f32) {
    let (hw, hl, top) = (0.5, 1.2, TABLE_H);
    lib.decor(piece(
        frame,
        (a - hw, a + hw),
        (o - hl, o + hl),
        (top - 0.05, top),
        kind::WOOD,
        WALNUT,
    ));
    let leg = turned_leg(lib);
    for (da, d_o) in [(-1.0, -1.0), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)] {
        let (x, y) = (a + da * (hw - 0.07), o + d_o * (hl - 0.07));
        lib.decor(leg.sized(Vec3::new(0.09, top - 0.05, 0.09)).at(
            frame.at(x, y, 0.0),
            Quat::IDENTITY,
            kind::WOOD,
            rgba(DARK_WOOD, 1.0),
        ));
    }
    guard(lib, frame, (a - hw, a + hw), (o - hl, o + hl), top);
    let on = |da: f32, d_o: f32, dy: f32| frame.at(a + da, o + d_o, top + dy);
    let gilt = rgba(BRASS, 1.0);
    let foot = lib.turned(
        &[(0.0, 0.0), (0.5, 0.0), (0.5, 0.3), (0.3, 0.6), (0.25, 1.0), (0.0, 1.0)],
        24,
    );
    // The anemometer: four cups on arms over a stem, turning in a wind that is not
    // there.
    lib.decor(
        foot.sized(Vec3::new(0.14, 0.04, 0.14))
            .at(on(0.0, -0.75, 0.0), Quat::IDENTITY, kind::METAL, gilt),
    );
    let hub = on(0.0, -0.75, 0.55);
    lib.pole(on(0.0, -0.75, 0.04), hub, 0.025, kind::METAL, gilt);
    let spin = lib.mover(
        hub,
        Motion::Spin {
            axis: Vec3::Y,
            rate: -1.0,
        },
        0.0,
        None,
    );
    let cap = lib.shape(Mesh::dome(16));
    lib.moving(
        spin,
        cap.sized(Vec3::new(0.05, 0.03, 0.05))
            .inst(hub + Vec3::Y * 0.01, Quat::IDENTITY, kind::METAL, gilt),
    );
    let cup = lib.turned_open(&[(0.0, 0.0), (0.25, 0.03), (0.4, 0.12), (0.48, 0.28), (0.5, 0.45)], 16);
    for k in 0..4 {
        let th = 0.3 + k as f32 * TAU / 4.0;
        let radial = frame.across() * libm::cosf(th) + frame.out() * libm::sinf(th);
        let tip = hub + radial * 0.22;
        let arm = lib.pole_inst(hub, tip, 0.012, kind::METAL, gilt);
        lib.moving(spin, arm);
        let tangent = Vec3::Y.cross(radial);
        lib.moving(
            spin,
            cup.sized(cup.size * 0.1).at(
                tip - tangent * 0.022,
                Quat::from_rotation_arc(Vec3::Y, tangent),
                kind::METAL,
                gilt,
            ),
        );
    }
    // The rain gauge: a brass funnel on a glass tube, a little water in it.
    let tube = lib.shape(Mesh::cylinder(16));
    let glass_tube = lib.turned(&[(0.5, 0.0), (0.5, 1.0)], 16);
    lib.translucent(glass_tube.sized(Vec3::new(0.09, 0.32, 0.09)).at(
        on(0.0, -0.05, 0.0),
        Quat::IDENTITY,
        kind::CLEAR,
        CLEAR_GLASS,
    ));
    lib.decor(tube.sized(Vec3::new(0.08, 0.07, 0.08)).inst(
        on(0.0, -0.05, 0.04),
        Quat::IDENTITY,
        kind::CRYSTAL,
        [0.35, 0.5, 0.75, 0.05],
    ));
    let funnel = lib.turned_open(&[(0.045, 0.0), (0.045, 0.04), (0.085, 0.09), (0.085, 0.11)], 24);
    lib.decor(funnel.at(on(0.0, -0.05, 0.31), Quat::IDENTITY, kind::METAL, gilt));
    // The storm glass: a tall stoppered tube, feathery crystals settled in it.
    let storm = lib.turned(&[(0.5, 0.0), (0.5, 0.85), (0.4, 0.95), (0.25, 1.0)], 16);
    lib.translucent(storm.sized(Vec3::new(0.06, 0.4, 0.06)).at(
        on(0.25, 0.3, 0.03),
        Quat::IDENTITY,
        kind::CLEAR,
        CLEAR_GLASS,
    ));
    lib.decor(
        foot.sized(Vec3::new(0.1, 0.03, 0.1))
            .at(on(0.25, 0.3, 0.0), Quat::IDENTITY, kind::METAL, gilt),
    );
    lib.decor(tube.sized(Vec3::new(0.03, 0.04, 0.03)).inst(
        on(0.25, 0.3, 0.44),
        Quat::IDENTITY,
        kind::WOOD,
        [0.5, 0.36, 0.2, 1.0],
    ));
    for k in 0..4 {
        let q = frame.turn() * Quat::from_rotation_y(k as f32 * 0.8) * Quat::from_rotation_z(0.3 - 0.2 * k as f32);
        lib.decor(Inst::new(
            on(0.25, 0.3, 0.07 + 0.012 * k as f32),
            q,
            Vec3::new(0.035, 0.05, 0.003),
            kind::CRYSTAL,
            [0.85, 0.9, 0.95, 0.3],
            0,
        ));
    }
    // The snowflake under its bell: six spokes of ice, glowing, turning slowly.
    let flake = on(-0.15, 0.55, 0.24);
    let face = Quat::from_mat3(&Mat3::from_cols(frame.across(), Vec3::Y, -frame.out()));
    let turning = lib.mover(
        flake,
        Motion::Spin {
            axis: Vec3::Y,
            rate: 0.25,
        },
        0.0,
        None,
    );
    for k in 0..3 {
        let q = face * Quat::from_rotation_y(FRAC_PI_2) * Quat::from_rotation_z(k as f32 * TAU / 6.0);
        lib.moving(
            turning,
            Inst::new(
                flake,
                q,
                Vec3::new(0.22, 0.018, 0.006),
                kind::CRYSTAL,
                [0.7, 0.9, 1.0, 1.5],
                0,
            ),
        );
    }
    lib.decor(
        foot.sized(Vec3::new(0.22, 0.03, 0.22))
            .at(on(-0.15, 0.55, 0.0), Quat::IDENTITY, kind::METAL, gilt),
    );
    let bell = lib.turned(
        &[
            (0.5, 0.0),
            (0.5, 0.7),
            (0.45, 0.82),
            (0.33, 0.92),
            (0.12, 0.98),
            (0.06, 1.0),
            (0.0, 1.0),
        ],
        32,
    );
    lib.translucent(bell.sized(Vec3::new(0.19, 0.38, 0.19)).at(
        on(-0.15, 0.55, 0.03),
        Quat::IDENTITY,
        kind::CLEAR,
        CLEAR_GLASS,
    ));
    let knob = lib.shape(Mesh::sphere(12));
    lib.decor(knob.sized(Vec3::splat(0.035)).inst(
        on(-0.15, 0.55, 0.42),
        Quat::IDENTITY,
        kind::CRYSTAL,
        [0.7, 0.85, 1.0, 0.1],
    ));
    // A lamp at the far end.
    let lamp = on(0.2, 1.0, 0.0);
    hurricane_lamp(lib, lamp);
    lib.light(lamp + Vec3::Y * 0.45, 4.5, [1.0, 0.65, 0.32], 0.8, LightKind::Flicker);
}

/// Wind chimes hung from the ceiling over `a`, `o`: tubes round a disc, a clapper
/// among them and a sail under it, the whole swaying a little and each tube stirring
/// on its own; `k` sets them apart from the other chimes.
fn wind_chime(lib: &mut Library, frame: &RoomFrame, a: f32, o: f32, k: f32) {
    let h = RoomFrame::HEIGHT;
    let disc = 3.55;
    let top = frame.at(a, o, h);
    let sway = Motion::Swing {
        axis: Quat::from_rotation_y(k * 1.3) * Vec3::X,
        amp: 0.02,
        period: 6.0 + k,
    };
    let whole = lib.mover(top, sway, k * 2.0, None);
    let cord = [0.2, 0.18, 0.15, 1.0];
    let string = lib.pole_inst(frame.at(a, o, disc), top, 0.008, kind::PLAIN, cord);
    lib.moving(whole, string);
    let plate = lib.shape(Mesh::cylinder(24));
    lib.moving(
        whole,
        plate.sized(Vec3::new(0.34, 0.03, 0.34)).inst(
            frame.at(a, o, disc),
            Quat::IDENTITY,
            kind::WOOD,
            rgba(WALNUT, 1.0),
        ),
    );
    let tube = lib.shape(Mesh::cylinder(12));
    for j in 0..6 {
        let th = 0.4 + j as f32 * TAU / 6.0;
        let hang = frame.at(a + 0.13 * libm::cosf(th), o + 0.13 * libm::sinf(th), disc - 0.015);
        let len = 0.35 + 0.07 * j as f32;
        let stir = Motion::Swing {
            axis: Vec3::Y.cross(hang - frame.at(a, o, disc)).normalize(),
            amp: 0.05,
            period: 2.2 + 0.25 * j as f32,
        };
        let m = lib.mover(hang, stir, j as f32 * 0.7 + k, Some(whole));
        lib.moving(
            m,
            tube.sized(Vec3::new(0.03, len, 0.03)).inst(
                hang - Vec3::Y * (0.04 + len * 0.5),
                Quat::IDENTITY,
                kind::METAL,
                [0.62, 0.64, 0.68, 1.0],
            ),
        );
    }
    // The clapper and the sail under it, on a cord through the disc.
    let swing = Motion::Swing {
        axis: Quat::from_rotation_y(k * 1.3 + 0.8) * Vec3::X,
        amp: 0.06,
        period: 3.1,
    };
    let below = frame.at(a, o, disc - 0.015);
    let m = lib.mover(below, swing, k, Some(whole));
    let thread = lib.pole_inst(below, below - Vec3::Y * 0.7, 0.005, kind::PLAIN, cord);
    lib.moving(m, thread);
    lib.moving(
        m,
        plate.sized(Vec3::new(0.09, 0.025, 0.09)).inst(
            below - Vec3::Y * 0.42,
            Quat::IDENTITY,
            kind::WOOD,
            rgba(WALNUT, 1.0),
        ),
    );
    lib.moving(
        m,
        Inst::new(
            below - Vec3::Y * 0.78,
            frame.turn() * Quat::from_rotation_y(k),
            Vec3::new(0.12, 0.16, 0.008),
            kind::WOOD,
            rgba(WALNUT, 1.0),
            0,
        ),
    );
}

/// A brass umbrella stand by the door, two umbrellas furled in it.
fn umbrella_stand(lib: &mut Library, frame: &RoomFrame, a: f32, o: f32) {
    let stand = lib.turned_open(
        &[
            (0.0, 0.0),
            (0.16, 0.0),
            (0.17, 0.03),
            (0.15, 0.08),
            (0.15, 0.56),
            (0.17, 0.6),
            (0.16, 0.62),
        ],
        32,
    );
    lib.decor(stand.at(frame.at(a, o, 0.0), Quat::IDENTITY, kind::METAL, rgba(BRASS, 1.0)));
    guard(lib, frame, (a - 0.17, a + 0.17), (o - 0.17, o + 0.17), 0.62);
    // A furled umbrella: its ferrule down, the cloth wound round its stick, a crook.
    let furled = lib.turned(
        &[
            (0.0, 0.0),
            (0.08, 0.04),
            (0.3, 0.3),
            (0.5, 0.75),
            (0.42, 0.85),
            (0.1, 0.9),
            (0.08, 1.0),
            (0.0, 1.0),
        ],
        12,
    );
    let crook: Vec<Vec3> = (0..=10)
        .map(|j| {
            let t = j as f32 / 10.0 * PI;
            Vec3::new(0.04 - 0.04 * libm::cosf(t), 0.08 + 0.04 * libm::sinf(t), 0.0)
        })
        .collect();
    let mut hook = vec![Vec3::ZERO];
    hook.extend(crook);
    let handle = lib.shape(Mesh::sweep(&hook, &[0.012], &circle(1.0, 8), true));
    for (k, (da, lean, col)) in [(-0.06, -0.12, [0.1, 0.12, 0.3]), (0.07, 0.1, [0.35, 0.08, 0.08])]
        .into_iter()
        .enumerate()
    {
        let foot = frame.at(a + da, o, 0.05);
        let up = (Vec3::Y + frame.across() * lean + frame.out() * 0.05 * k as f32).normalize();
        let tilt = Quat::from_rotation_arc(Vec3::Y, up);
        lib.decor(
            furled
                .sized(Vec3::new(0.08, 0.9, 0.08))
                .at(foot, tilt, kind::CLOTH, rgba(col, 1.0)),
        );
        let turn = tilt * Quat::from_rotation_y(0.6 + 2.0 * k as f32);
        lib.decor(handle.at(foot + up * 0.9, turn, kind::WOOD, rgba(DARK_WOOD, 1.0)));
    }
}

/// A skylight over the nave (the room is under the roof), framed in beams, and
/// beams across the rest of the ceiling.
fn ceiling(lib: &mut Library, frame: &RoomFrame) {
    let h = RoomFrame::HEIGHT;
    let inner = RoomFrame::A_HALF;
    let (s0, s1) = (19.0, 27.0);
    let sw = 2.2;
    // Laid flat with its own x and y along the ceiling, so its leading is a grid.
    let flat = frame.turn() * Quat::from_rotation_x(-FRAC_PI_2);
    let mid = frame.at(0.0, (s0 + s1) * 0.5, h - 0.02);
    let glass = Inst::new(
        mid,
        flat,
        Vec3::new(2.0 * sw, s1 - s0, 0.02),
        kind::GLASS,
        [0.2, 0.27, 0.55, 0.6],
        0,
    );
    lib.decor(glass);
    let dark = [0.11, 0.055, 0.025];
    for o in [16.6, s0 - 0.13, s1 + 0.13, 30.6] {
        lib.decor(piece(
            frame,
            (-inner, inner),
            (o - 0.13, o + 0.13),
            (h - 0.3, h),
            kind::WOOD,
            dark,
        ));
    }
    for a in [-sw - 0.12, sw + 0.12] {
        lib.decor(piece(
            frame,
            (a - 0.12, a + 0.12),
            (s0, s1),
            (h - 0.3, h),
            kind::WOOD,
            dark,
        ));
    }
    // Glazing bars over the glass.
    for k in 1..4 {
        let o = s0 + k as f32 * (s1 - s0) / 4.0;
        lib.decor(piece(
            frame,
            (-sw, sw),
            (o - 0.04, o + 0.04),
            (h - 0.1, h - 0.03),
            kind::WOOD,
            dark,
        ));
    }
    lib.decor(piece(
        frame,
        (-0.04, 0.04),
        (s0, s1),
        (h - 0.09, h - 0.03),
        kind::WOOD,
        dark,
    ));
    lib.light(
        frame.at(0.0, (s0 + s1) * 0.5, h - 1.0),
        9.0,
        [0.45, 0.55, 1.0],
        0.7,
        LightKind::Static,
    );
    let beam = (Vec3::Y * -1.0 + frame.out() * 0.18).normalize();
    for o in [21.0, 25.0] {
        let len = 5.4;
        let start = frame.at(0.0, o, h - 0.05);
        lib.static_trans.push(Inst::new(
            start + beam * len * 0.5,
            Quat::from_rotation_arc(Vec3::Y, beam),
            Vec3::new(2.4, len, 2.4),
            kind::SHAFT,
            [0.35, 0.45, 0.9, 0.03],
            0,
        ));
    }
}
