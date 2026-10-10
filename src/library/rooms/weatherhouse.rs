//! The Weatherhouse: department 10, top floor, south side. A long nave between two
//! rows of cases, wall cases down the side walls behind them and either side of a
//! tall window streaked with rain; a skylight over the nave. A wind rose is set in
//! the floor; a weathervane, a table of instruments and a cloud kept in a glass jar
//! stand on it; wind chimes hang over it; barometers, thermometers and a weather
//! map hang by the door. The cloud glows, lit now and then from inside.

use super::*;

/// The rows of cases down the room: how far either side of the middle, and where
/// along it the first one stands.
const ROW_A: f32 = RoomFrame::A_HALF - WALL_D - 3.0 - UNIT_DEPTH * 0.5;
const ROW_O: f32 = 17.3;
/// A wall case's depth from the wall to its face.
const WALL_D: f32 = 0.04 + FACE_DEPTH;
/// The great window's half-width.
const WINDOW: f32 = 1.3;
const BRASS: [f32; 3] = [0.7, 0.5, 0.2];
const PALE_STONE: [f32; 3] = [0.48, 0.45, 0.4];
const STORM_GLASS: [f32; 3] = [0.3, 0.38, 0.62];

pub(super) fn furnish(lib: &mut Library, frame: &RoomFrame, rng: &mut StdRng, titles: &mut HashSet<String>) {
    debug_assert!(frame.door.abs() < 1.0, "the Weatherhouse wants its door in the middle");
    shelves(lib, frame, rng, titles);
    great_window(lib, frame);
    near_wall(lib, frame);
    wind_rose(lib, frame, 0.0, 20.6);
    cloud_jar(lib, frame, 0.0, 25.4);
    weathervane(lib, frame, 4.6, 22.6);
    instrument_table(lib, frame, -4.4, 22.6);
    for (a, o) in [(-2.6, 28.6), (2.6, 28.6), (0.0, 17.6)] {
        wind_chime(lib, frame, a, o);
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
    // Rain: streaks down the glass, catching the moonlight, and beads between them.
    let mut rng = StdRng::seed_from_u64(0x5241_494e);
    let face = |rng: &mut StdRng, y_lo: f32| {
        (
            rng.gen_range(-WINDOW + 0.05..WINDOW - 0.05),
            rng.gen_range(y_lo..y1 - 0.1),
        )
    };
    let turn = frame.turn();
    for k in 0..34 {
        let (a, y) = face(&mut rng, y0 + 0.3);
        let len = rng.gen_range(0.12..0.55);
        let lean = Quat::from_rotation_z(rng.gen_range(-0.06..0.06));
        let p = frame.at(a, wall - 0.065 - 0.002 * (k % 3) as f32, y - len * 0.5);
        let streak = Inst::new(
            p,
            turn * lean,
            Vec3::new(0.012, len, 0.004),
            kind::EMISSIVE,
            [0.12, 0.15, 0.25, 1.0],
            0,
        );
        lib.decor(streak);
    }
    for k in 0..30 {
        let (a, y) = face(&mut rng, y0 + 0.1);
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
/// barometers, thermometers and a storm glass to the other.
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
    // Rings round a low and a high, drawn as polygons and clipped to the sheet.
    for (cu, cv, rings) in [
        (-0.7f32, 0.2f32, &[0.22f32, 0.42, 0.62, 0.82][..]),
        (0.95, -0.25, &[0.18, 0.36, 0.54][..]),
    ] {
        for &r in rings {
            let segs = 12;
            let at = |k: usize| {
                let t = k as f32 / segs as f32 * TAU;
                (cu + r * t.cos(), cv + r * 0.75 * t.sin())
            };
            for k in 0..segs {
                let (p, q) = (at(k), at(k + 1));
                let (mu, mv) = ((p.0 + q.0) * 0.5, (p.1 + q.1) * 0.5);
                if mu.abs() < mw * 0.5 - 0.05 && mv.abs() < mh * 0.5 - 0.05 {
                    rod_flat(lib, sheet(p.0, p.1), sheet(q.0, q.1), n, 0.012, ink);
                }
            }
        }
    }
    // A front: a line across with teeth along it.
    for k in 0..7 {
        let u = -1.5 + k as f32 * 0.45;
        let (p, q) = (
            sheet(u, -0.75 + 0.18 * (u * 1.3).sin()),
            sheet(u + 0.45, -0.75 + 0.18 * ((u + 0.45) * 1.3).sin()),
        );
        rod_flat(lib, p, q, n, 0.016, rgba([0.2, 0.25, 0.6], 1.0));
        let mid = (p + q) * 0.5 + Vec3::Y * 0.05;
        let tooth = Inst::new(
            mid,
            Quat::from_mat3(&Mat3::from_cols(Vec3::Y.cross(n), Vec3::Y, n)) * Quat::from_rotation_z(0.785),
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
/// under a long neck with a thermometer in it; `seed` sets its needle.
fn barometer(lib: &mut Library, c: Vec3, n: Vec3, seed: f32) {
    let side = Vec3::Y.cross(n);
    let face = Quat::from_mat3(&Mat3::from_cols(side, Vec3::Y, n));
    let wood = rgba([0.32, 0.15, 0.06], 1.0);
    // The round case and its dial.
    disc(lib, c + n * 0.02, n, 0.22, 0.04, kind::WOOD, wood);
    disc(lib, c + n * 0.045, n, 0.15, 0.01, kind::PAPER, [0.0; 4]);
    let needle = face * Quat::from_rotation_z(1.2 + seed * 0.7);
    lib.decor(Inst::new(
        c + n * 0.055,
        needle,
        Vec3::new(0.012, 0.26, 0.006),
        kind::METAL,
        [0.05, 0.05, 0.06, 1.0],
        0,
    ));
    lib.decor(Inst::new(
        c + n * 0.06,
        face * Quat::from_rotation_z(0.785),
        Vec3::new(0.03, 0.03, 0.01),
        kind::METAL,
        rgba(BRASS, 1.0),
        0,
    ));
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
    lib.decor(Inst::new(
        c + Vec3::Y * 0.88 + n * 0.02,
        face * Quat::from_rotation_z(0.785),
        Vec3::new(0.09, 0.09, 0.04),
        kind::WOOD,
        wood,
        0,
    ));
}

/// A round plate at `c` facing `n`, radius `r`, `t` thick: wedges from its middle,
/// alternately a hair forward so none lie flat on another.
fn disc(lib: &mut Library, c: Vec3, n: Vec3, r: f32, t: f32, kind: u32, col: [f32; 4]) {
    let side = Vec3::Y.cross(n);
    let segs = 12;
    let step = TAU / segs as f32;
    for k in 0..segs {
        let th = 0.05 + k as f32 * step;
        let radial = side * th.cos() + Vec3::Y * th.sin();
        let q = Quat::from_mat3(&Mat3::from_cols(radial, n.cross(radial), n));
        let off = if k % 2 == 0 { 0.0 } else { 0.002 };
        let size = Vec3::new(r, 2.0 * r * (step * 0.5).sin() * 1.08, t);
        lib.decor(Inst::new(c + radial * (r * 0.5) + n * off, q, size, kind, col, 0));
    }
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
    lib.decor(Inst::new(
        c + n * 0.04 + Vec3::Y * 0.02,
        face,
        Vec3::new(0.018, 0.68, 0.01),
        kind::CRYSTAL,
        [0.5, 0.55, 0.6, 0.15],
        0,
    ));
    lib.decor(Inst::new(
        c + n * 0.046 - Vec3::Y * 0.12,
        face,
        Vec3::new(0.01, 0.44, 0.006),
        kind::EMISSIVE,
        [0.85, 0.1, 0.06, 0.7],
        0,
    ));
    lib.decor(Inst::new(
        c + n * 0.045 - Vec3::Y * 0.36,
        face * Quat::from_rotation_z(0.785),
        Vec3::new(0.035, 0.035, 0.012),
        kind::EMISSIVE,
        [0.85, 0.1, 0.06, 0.7],
        0,
    ));
}

/// A wind rose inlaid in the floor round `a`, `o`: a brass ring and eight points,
/// the north one picked out.
fn wind_rose(lib: &mut Library, frame: &RoomFrame, a: f32, o: f32) {
    let c = frame.at(a, o, 0.0);
    let r = 2.0;
    let segs = 24;
    for k in 0..segs {
        let th = 0.05 + k as f32 / segs as f32 * TAU;
        let radial = frame.across() * th.cos() + frame.out() * th.sin();
        let q = Quat::from_mat3(&Mat3::from_cols(radial, Vec3::Y, radial.cross(Vec3::Y)));
        let len = 2.0 * r * (TAU / segs as f32 * 0.5).tan() + 0.01;
        lib.decor(Inst::new(
            c + radial * r + Vec3::Y * 0.016,
            q,
            Vec3::new(0.06, 0.006, len),
            kind::METAL,
            rgba(BRASS, 1.0),
            0,
        ));
    }
    // Eight points, each tapering in three steps; north's red.
    for k in 0..8 {
        let th = 0.01 + k as f32 / 8.0 * TAU;
        let radial = frame.across() * th.cos() + frame.out() * th.sin();
        let long = k % 2 == 0;
        let (len, w) = if long { (r * 1.25, 0.26) } else { (r * 0.75, 0.16) };
        let q = Quat::from_mat3(&Mat3::from_cols(radial.cross(Vec3::Y), Vec3::Y, -radial));
        let col = if k == 2 {
            [0.6, 0.12, 0.08]
        } else if long {
            [0.62, 0.5, 0.3]
        } else {
            [0.4, 0.3, 0.17]
        };
        for j in 0..3 {
            let (d0, d1) = (len * j as f32 / 3.0, len * (j + 1) as f32 / 3.0);
            let p = c + radial * ((d0 + d1) * 0.5) + Vec3::Y * (0.004 + 0.0007 * k as f32);
            let size = Vec3::new(w * (1.0 - j as f32 / 3.2), 0.004, d1 - d0);
            lib.decor(Inst::new(p, q, size, kind::WOOD, rgba(col, 1.0), 0));
        }
    }
    lib.decor(Inst::new(
        c + Vec3::Y * 0.014,
        Quat::from_rotation_y(0.8),
        Vec3::new(0.22, 0.01, 0.22),
        kind::METAL,
        rgba(BRASS, 1.0),
        0,
    ));
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
    // A brass foot and lid; the glass between them is drawn see-through, as faint
    // panes with bright edges, an octagon round.
    let (r, h) = (0.26, 0.62);
    for k in 0..2 {
        let q = Quat::from_rotation_y(k as f32 * 0.785 + 0.2);
        lib.decor(Inst::new(
            base + Vec3::Y * (0.03 + 0.002 * k as f32),
            q,
            Vec3::new(r * 1.85, 0.06, r * 1.85),
            kind::METAL,
            rgba(BRASS, 1.0),
            0,
        ));
        lib.decor(Inst::new(
            base + Vec3::Y * (h + 0.07 + 0.002 * k as f32),
            q,
            Vec3::new(r * 1.75, 0.05, r * 1.75),
            kind::METAL,
            rgba(BRASS, 1.0),
            0,
        ));
    }
    lib.decor(Inst::new(
        base + Vec3::Y * (h + 0.14),
        Quat::from_rotation_y(0.985),
        Vec3::new(0.08, 0.08, 0.08),
        kind::METAL,
        rgba(BRASS, 1.0),
        0,
    ));
    for k in 0..8 {
        let th = 0.2 + (k as f32 + 0.5) / 8.0 * TAU;
        let radial = Vec3::new(th.cos(), 0.0, th.sin());
        let q = Quat::from_rotation_arc(Vec3::Z, radial);
        let pane = 2.0 * r * (TAU / 16.0).tan();
        lib.static_trans.push(Inst::new(
            base + Vec3::Y * (0.06 + h * 0.5) + radial * r,
            q,
            Vec3::new(pane, h, 0.004),
            kind::GHOST,
            [0.6, 0.75, 0.95, 0.35],
            0,
        ));
    }
    // The cloud: puffs of white, faintly lit from within.
    let cloud = base + Vec3::Y * 0.36;
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
        lib.decor(Inst::new(
            cloud + off,
            q,
            Vec3::splat(s),
            kind::PAPER,
            [0.55, 0.62, 0.8, 0.35],
            0,
        ));
    }
    // A fork of lightning in it.
    let bolt = [
        Vec3::new(0.02, -0.04, 0.0),
        Vec3::new(-0.03, -0.12, 0.01),
        Vec3::new(0.02, -0.17, 0.0),
        Vec3::new(-0.02, -0.27, 0.0),
    ];
    for w in bolt.windows(2) {
        rod(
            lib,
            cloud + w[0],
            cloud + w[1],
            0.012,
            0.012,
            kind::EMISSIVE,
            [0.85, 0.9, 1.0, 6.0],
        );
    }
    lib.light(cloud, 3.5, [0.7, 0.8, 1.0], 0.7, LightKind::Flicker);
}

/// A weathervane on a post at `a`, `o`: the arms of the four winds and an arrow
/// with a cockerel's tail turning above them.
fn weathervane(lib: &mut Library, frame: &RoomFrame, a: f32, o: f32) {
    let iron = rgba([0.12, 0.12, 0.13], 1.0);
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
    rod(
        lib,
        frame.at(a, o, 0.3),
        frame.at(a, o, top + 0.25),
        0.05,
        0.05,
        kind::METAL,
        iron,
    );
    let arms = 1.9;
    for (da, d_o) in [(1.0f32, 0.0f32), (0.0, 1.0)] {
        let (p, q) = (
            frame.at(a - da * 0.45, o - d_o * 0.45, arms),
            frame.at(a + da * 0.45, o + d_o * 0.45, arms),
        );
        rod(lib, p, q, 0.025, 0.025, kind::METAL, iron);
        for end in [p, q] {
            lib.decor(Inst::new(
                end,
                Quat::from_rotation_y(0.785),
                Vec3::splat(0.07),
                kind::METAL,
                gilt,
                0,
            ));
        }
    }
    // The arrow, pointing off to the window's side of the room.
    let dir = (frame.out() * 0.8 - frame.across() * 0.6).normalize();
    let mid = frame.at(a, o, top);
    rod(lib, mid - dir * 0.5, mid + dir * 0.45, 0.025, 0.025, kind::METAL, gilt);
    let flat = Quat::from_mat3(&Mat3::from_cols(dir, Vec3::Y, dir.cross(Vec3::Y)));
    lib.decor(Inst::new(
        mid + dir * 0.5,
        flat * Quat::from_rotation_z(0.785),
        Vec3::new(0.12, 0.12, 0.02),
        kind::METAL,
        gilt,
        0,
    ));
    // The cockerel: a body, a tail fanned up behind, a comb.
    let bird = mid - dir * 0.25 + Vec3::Y * 0.16;
    lib.decor(Inst::new(
        bird,
        flat * Quat::from_rotation_z(0.15),
        Vec3::new(0.3, 0.16, 0.02),
        kind::METAL,
        gilt,
        0,
    ));
    for k in 0..3 {
        let fan = flat * Quat::from_rotation_z(0.9 + k as f32 * 0.35);
        lib.decor(Inst::new(
            bird - dir * 0.16 + Vec3::Y * 0.08,
            fan,
            Vec3::new(0.24, 0.05, 0.018),
            kind::METAL,
            gilt,
            0,
        ));
    }
    lib.decor(Inst::new(
        bird + dir * 0.13 + Vec3::Y * 0.12,
        flat,
        Vec3::new(0.08, 0.1, 0.02),
        kind::METAL,
        gilt,
        0,
    ));
}

/// A table of instruments at `a`, `o` running outward: an anemometer, a rain gauge,
/// a storm glass, a lamp, and under a small bell of glass a snowflake that will not
/// melt.
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
    for (da, d_o) in [(-1.0, -1.0), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)] {
        let (x, y) = (a + da * (hw - 0.07), o + d_o * (hl - 0.07));
        lib.decor(piece(
            frame,
            (x - 0.045, x + 0.045),
            (y - 0.045, y + 0.045),
            (0.0, top - 0.05),
            kind::WOOD,
            DARK_WOOD,
        ));
    }
    guard(lib, frame, (a - hw, a + hw), (o - hl, o + hl), top);
    let on = |da: f32, d_o: f32, dy: f32| frame.at(a + da, o + d_o, top + dy);
    let gilt = rgba(BRASS, 1.0);
    // The anemometer: four cups on arms over a stem.
    let hub = on(0.0, -0.75, 0.55);
    rod(lib, on(0.0, -0.75, 0.0), hub, 0.025, 0.025, kind::METAL, gilt);
    for k in 0..4 {
        let th = 0.3 + k as f32 * TAU / 4.0;
        let radial = frame.across() * th.cos() + frame.out() * th.sin();
        let tip = hub + radial * 0.22;
        rod(lib, hub, tip, 0.012, 0.012, kind::METAL, gilt);
        let tangent = Vec3::Y.cross(radial);
        lib.decor(Inst::new(
            tip + tangent * 0.03,
            Quat::from_rotation_arc(Vec3::Z, tangent),
            Vec3::new(0.08, 0.08, 0.05),
            kind::METAL,
            gilt,
            0,
        ));
    }
    // The rain gauge: a brass funnel on a glass tube, a little water in it.
    let tube = |c: Vec3, size: Vec3| Inst::new(c, Quat::IDENTITY, size, kind::CRYSTAL, [0.45, 0.55, 0.7, 0.15], 0);
    lib.decor(tube(on(0.0, -0.05, 0.16), Vec3::new(0.09, 0.32, 0.09)));
    lib.decor(Inst::aabox(
        on(0.0, -0.05, 0.36),
        Vec3::new(0.16, 0.08, 0.16),
        kind::METAL,
        BRASS,
    ));
    // The storm glass: a tall tube, feathery crystals settled in it.
    lib.decor(tube(on(0.25, 0.3, 0.2), Vec3::new(0.06, 0.4, 0.06)));
    lib.decor(Inst::aabox(
        on(0.25, 0.3, 0.02),
        Vec3::new(0.1, 0.04, 0.1),
        kind::METAL,
        BRASS,
    ));
    // The snowflake under its bell: six spokes of ice, glowing.
    let flake = on(-0.15, 0.55, 0.24);
    let face = Quat::from_mat3(&Mat3::from_cols(frame.across(), Vec3::Y, -frame.out()));
    for k in 0..3 {
        let q = face * Quat::from_rotation_y(1.571) * Quat::from_rotation_z(k as f32 * TAU / 6.0);
        lib.decor(Inst::new(
            flake,
            q,
            Vec3::new(0.22, 0.018, 0.006),
            kind::CRYSTAL,
            [0.7, 0.9, 1.0, 1.5],
            0,
        ));
    }
    lib.decor(Inst::aabox(
        on(-0.15, 0.55, 0.02),
        Vec3::new(0.2, 0.04, 0.2),
        kind::METAL,
        BRASS,
    ));
    lib.static_trans.push(Inst::new(
        on(-0.15, 0.55, 0.22),
        Quat::from_rotation_y(0.3),
        Vec3::new(0.18, 0.36, 0.18),
        kind::GHOST,
        [0.6, 0.75, 0.95, 0.35],
        0,
    ));
    // A brass lamp at the far end.
    let lamp = on(0.2, 1.0, 0.0);
    lib.decor(Inst::aabox(
        lamp + Vec3::Y * 0.04,
        Vec3::new(0.16, 0.08, 0.16),
        kind::METAL,
        BRASS,
    ));
    let mut glow = Inst::aabox(
        lamp + Vec3::Y * 0.2,
        Vec3::new(0.1, 0.18, 0.1),
        kind::EMISSIVE,
        [1.0, 0.72, 0.4],
    );
    glow.color[3] = 1.8;
    glow.param = 1;
    lib.decor(glow);
    lib.light(lamp + Vec3::Y * 0.45, 4.5, [1.0, 0.65, 0.32], 0.8, LightKind::Flicker);
}

/// Wind chimes hung from the ceiling over `a`, `o`.
fn wind_chime(lib: &mut Library, frame: &RoomFrame, a: f32, o: f32) {
    let h = RoomFrame::HEIGHT;
    let disc = 3.55;
    rod(
        lib,
        frame.at(a, o, disc),
        frame.at(a, o, h),
        0.012,
        0.012,
        kind::PLAIN,
        [0.2, 0.18, 0.15, 1.0],
    );
    lib.decor(Inst::new(
        frame.at(a, o, disc),
        Quat::from_rotation_y(0.4),
        Vec3::new(0.34, 0.03, 0.34),
        kind::WOOD,
        rgba(WALNUT, 1.0),
        0,
    ));
    for k in 0..6 {
        let th = 0.4 + k as f32 * TAU / 6.0;
        let p = frame.at(a + 0.13 * th.cos(), o + 0.13 * th.sin(), disc);
        let len = 0.35 + 0.07 * k as f32;
        lib.decor(Inst::aabox(
            p - Vec3::Y * (0.05 + len * 0.5),
            Vec3::new(0.03, len, 0.03),
            kind::METAL,
            [0.62, 0.64, 0.68],
        ));
    }
    lib.decor(Inst::aabox(
        frame.at(a, o, disc - 0.45),
        Vec3::new(0.04, 0.12, 0.04),
        kind::WOOD,
        WALNUT,
    ));
}

/// A brass umbrella stand by the door, two umbrellas furled in it.
fn umbrella_stand(lib: &mut Library, frame: &RoomFrame, a: f32, o: f32) {
    lib.decor(piece(
        frame,
        (a - 0.17, a + 0.17),
        (o - 0.17, o + 0.17),
        (0.0, 0.62),
        kind::METAL,
        BRASS,
    ));
    lib.decor(piece(
        frame,
        (a - 0.14, a + 0.14),
        (o - 0.14, o + 0.14),
        (0.62, 0.625),
        kind::PLAIN,
        [0.03, 0.03, 0.03],
    ));
    guard(lib, frame, (a - 0.17, a + 0.17), (o - 0.17, o + 0.17), 0.62);
    for (k, (da, lean, col)) in [(-0.06, -0.12, [0.1, 0.12, 0.3]), (0.07, 0.1, [0.35, 0.08, 0.08])]
        .into_iter()
        .enumerate()
    {
        let foot = frame.at(a + da, o, 0.3);
        let tip = foot + (Vec3::Y + frame.across() * lean + frame.out() * 0.05 * k as f32).normalize() * 0.85;
        rod(lib, foot, tip, 0.07, 0.07, kind::CLOTH, rgba(col, 1.0));
        let handle = tip + Vec3::Y * 0.08;
        rod(lib, tip, handle, 0.025, 0.025, kind::WOOD, rgba(DARK_WOOD, 1.0));
        lib.decor(Inst::aabox(
            handle + frame.across() * 0.04,
            frame.size(0.08, 0.02, 0.02),
            kind::WOOD,
            DARK_WOOD,
        ));
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
    let flat = frame.turn() * Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2);
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
