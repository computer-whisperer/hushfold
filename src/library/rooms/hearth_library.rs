//! The Hearth Library: department 11, top floor, west side. Wall cases all round
//! the room; at the far wall an inglenook, two runs of cases standing out either
//! side of a great fireplace, armchairs and a tea table on a rug before the fire, a
//! sofa across its mouth; two short screens of cases by the door; reading chairs, a
//! spinning wheel and a barrel about the room, herbs drying from the beams. Candles
//! float over the hearth, and a pair of needles knits on its own by a chair.

use super::*;

/// A wall case's depth from the wall to its face.
const WALL_D: f32 = 0.04 + FACE_DEPTH;
/// How far either side of the middle the inglenook's runs of three cases stand.
const NOOK_A: f32 = 4.0;
/// The screens by the door: how far out, and where across they start (their inner
/// ends).
const SCREEN_O: f32 = 19.5;
const SCREEN_A: f32 = RoomFrame::A_HALF - WALL_D - 1.5 - 2.0 * UNIT_PITCH;
/// The chimney breast's half-width and how far it stands out from the far wall.
const BREAST: f32 = 1.9;
const BREAST_D: f32 = 0.5;
const STONE_WARM: [f32; 3] = [0.46, 0.4, 0.34];
const SOOT: [f32; 3] = [0.05, 0.04, 0.035];
const IRON: [f32; 3] = [0.1, 0.1, 0.11];
const BRASS: [f32; 3] = [0.7, 0.5, 0.2];
const UPHOLSTERY: [f32; 3] = [0.28, 0.065, 0.05];
const GREEN_CLOTH: [f32; 3] = [0.14, 0.26, 0.14];

pub(super) fn furnish(lib: &mut Library, frame: &RoomFrame, rng: &mut StdRng, titles: &mut HashSet<String>) {
    debug_assert!(
        frame.door.abs() < 1.0,
        "the Hearth Library wants its door in the middle"
    );
    shelves(lib, frame, rng, titles);
    walls(lib, frame);
    fireplace(lib, frame);
    let far = RoomFrame::O_FAR;
    for s in [-1.0f32, 1.0] {
        armchair(lib, frame, s * 1.2, far - 2.7, -s, UPHOLSTERY);
    }
    tea_table(lib, frame, 0.0, far - 2.75);
    sofa(lib, frame, 0.0, 23.1);
    rugs(lib, frame);
    // Reading corners either side of the inglenook.
    armchair(lib, frame, -8.3, 26.6, 1.0, GREEN_CLOTH);
    lamp_table(lib, frame, -8.3, 25.45);
    knitting(lib, frame, -7.35, 26.3);
    armchair(lib, frame, 8.3, 27.4, -1.0, GREEN_CLOTH);
    lamp_table(lib, frame, 8.3, 28.55);
    spinning_wheel(lib, frame, 7.9, 24.4);
    barrel(lib, frame, -6.2, 14.75);
    floating_candles(lib, frame);
    ceiling(lib, frame);
}

/// A candle standing at `p`, `h` tall, its flame lit.
fn candle(lib: &mut Library, p: Vec3, h: f32) {
    lib.decor(Inst::aabox(
        p + Vec3::Y * (h * 0.5),
        Vec3::new(0.045, h, 0.045),
        kind::PLAIN,
        [0.85, 0.8, 0.65],
    ));
    let mut flame = Inst::aabox(
        p + Vec3::Y * (h + 0.04),
        Vec3::new(0.025, 0.055, 0.025),
        kind::EMISSIVE,
        [1.0, 0.55, 0.18],
    );
    flame.color[3] = 14.0;
    flame.param = 1;
    lib.decor(flame);
}

/// Wall cases round the room: two either side of the door, six down each side wall,
/// two in each far corner; the inglenook's runs of three, a category round the fire
/// on their inner faces and one round each corner on their outer; two screens of two
/// double-sided cases by the door.
fn shelves(lib: &mut Library, frame: &RoomFrame, rng: &mut StdRng, titles: &mut HashSet<String>) {
    let (out, across) = (frame.out(), frame.across());
    let ah = RoomFrame::A_HALF;
    let (near, far) = (RoomFrame::O_NEAR, RoomFrame::O_FAR);
    let cats: Vec<usize> = frame.categories.clone().collect();
    let [
        hearthcraft,
        cookery,
        brewing,
        baking,
        weaving,
        candlemaking,
        remedies,
        feast_days,
    ] = cats[..]
    else {
        unreachable!("a department has eight categories")
    };
    // Either side of the door, clear of the side walls' first cases.
    let corner = ah - WALL_D - UNIT_PITCH * 0.5 - 0.1;
    let door: Vec<_> = [-corner, -corner + UNIT_PITCH, corner - UNIT_PITCH, corner]
        .into_iter()
        .map(|a| lib.wall_bookcase(frame.at(a, near, 0.0), out))
        .collect();
    lib.add_category(rng, feast_days, &left_to_right(door), titles);
    // The side walls.
    for (s, cat) in [(-1.0f32, cookery), (1.0, remedies)] {
        let wall: Vec<_> = (0..6)
            .map(|k| lib.wall_bookcase(frame.at(s * ah, near + UNIT_PITCH * (k as f32 + 0.5), 0.0), -across * s))
            .collect();
        lib.add_category(rng, cat, &left_to_right(wall), titles);
    }
    // The inglenook's runs: each case's face toward the fire, and away from it.
    let mut inner = Vec::new();
    for (s, cat) in [(-1.0f32, baking), (1.0, brewing)] {
        let (mut toward, mut away) = (Vec::new(), Vec::new());
        for k in 0..3 {
            let o = far - UNIT_PITCH * (k as f32 + 0.5);
            for f in lib.bookcase(frame.at(s * NOOK_A, o, 0.0), frame.out_along_x()) {
                if f.1.dot(across) * s < 0.0 {
                    toward.push(f)
                } else {
                    away.push(f)
                }
            }
        }
        inner.push(left_to_right(toward));
        // The far corner's two wall cases and the run's outer face, round the inside
        // corner between them.
        let corner: Vec<_> = (0..2)
            .map(|k| lib.wall_bookcase(frame.at(s * (ah - UNIT_PITCH * (k as f32 + 0.5)), far, 0.0), -out))
            .collect();
        let faces: Vec<_> = if s < 0.0 {
            left_to_right(corner).into_iter().chain(left_to_right(away)).collect()
        } else {
            left_to_right(away).into_iter().chain(left_to_right(corner)).collect()
        };
        lib.add_category(rng, cat, &faces, titles);
    }
    let round_fire: Vec<_> = inner.concat();
    lib.add_category(rng, hearthcraft, &round_fire, titles);
    // The screens: round each one's end by the wall, so its sign faces the door.
    for (s, cat) in [(-1.0f32, candlemaking), (1.0, weaving)] {
        let (mut back, mut front) = (Vec::new(), Vec::new());
        for k in 0..2 {
            let a = s * (SCREEN_A + UNIT_PITCH * (k as f32 + 0.5));
            for f in lib.bookcase(frame.at(a, SCREEN_O, 0.0), !frame.out_along_x()) {
                if f.1.dot(out) > 0.0 {
                    back.push(f)
                } else {
                    front.push(f)
                }
            }
        }
        let faces: Vec<_> = left_to_right(back).into_iter().chain(left_to_right(front)).collect();
        lib.add_category(rng, cat, &faces, titles);
    }
}

/// Panelling where the walls show, and a painting over the door's panelling.
fn walls(lib: &mut Library, frame: &RoomFrame) {
    let side = frame.side;
    let (near, far) = (RoomFrame::O_NEAR, RoomFrame::O_FAR);
    let ah = RoomFrame::A_HALF;
    let d = frame.door;
    let corner = ah - WALL_D - UNIT_PITCH * 0.5 - 0.1;
    let inner = corner - UNIT_PITCH * 1.5 - 0.02;
    for (a0, a1) in [
        (-inner, d - RoomFrame::DOOR_HALF - 0.32),
        (d + RoomFrame::DOOR_HALF + 0.32, inner),
    ] {
        lib.wainscot(frame.y, frame.rect(a0, a1, near, near + 0.07), side.out);
    }
    for s in [-1.0f32, 1.0] {
        let (a0, a1) = (
            s * (NOOK_A + UNIT_DEPTH * 0.5 + 0.05),
            s * (ah - 2.0 * UNIT_PITCH - 0.02),
        );
        lib.wainscot(frame.y, frame.rect(a0.min(a1), a0.max(a1), far - 0.07, far), -side.out);
    }
}

/// The great fireplace: a stone chimney breast to the ceiling, a firebox with logs
/// burning on firedogs and a kettle hung over them, a hearthstone, a mantel with
/// candles and a clock, a painting over it.
fn fireplace(lib: &mut Library, frame: &RoomFrame) {
    let far = RoomFrame::O_FAR;
    let h = RoomFrame::HEIGHT;
    let face = far - BREAST_D;
    let (open, top) = (1.05, 1.35);
    // The breast: jambs either side of the opening, a lintel, the stack above.
    for s in [-1.0f32, 1.0] {
        let (a0, a1) = (s * open, s * BREAST);
        lib.decor(piece(
            frame,
            (a0.min(a1), a0.max(a1)),
            (face, far),
            (0.0, top),
            kind::STONE,
            STONE_WARM,
        ));
    }
    lib.decor(piece(
        frame,
        (-BREAST, BREAST),
        (face, far),
        (top, top + 0.35),
        kind::STONE,
        [0.5, 0.44, 0.37],
    ));
    lib.decor(piece(
        frame,
        (-BREAST + 0.1, BREAST - 0.1),
        (face + 0.12, far),
        (top + 0.35, h - 0.01),
        kind::STONE,
        STONE_WARM,
    ));
    lib.decor(piece(
        frame,
        (-open, open),
        (far - 0.06, far),
        (0.0, top),
        kind::STONE,
        SOOT,
    ));
    lib.decor(piece(
        frame,
        (-open, open),
        (face + 0.02, far - 0.06),
        (top - 0.04, top),
        kind::STONE,
        SOOT,
    ));
    guard(lib, frame, (-BREAST, BREAST), (face, far), top + 0.35);
    // The hearthstone, flush enough to walk on.
    lib.decor(piece(
        frame,
        (-1.6, 1.6),
        (face - 0.55, face),
        (0.0, 0.03),
        kind::STONE,
        [0.38, 0.33, 0.28],
    ));
    lib.decor(piece(
        frame,
        (-open + 0.02, open - 0.02),
        (face, far - 0.06),
        (0.0, 0.03),
        kind::STONE,
        [0.12, 0.09, 0.07],
    ));
    // Firedogs, logs, embers and flames.
    let fire_o = far - 0.27;
    for s in [-1.0f32, 1.0] {
        let x = s * 0.5;
        lib.decor(piece(
            frame,
            (x - 0.03, x + 0.03),
            (fire_o - 0.2, fire_o + 0.15),
            (0.03, 0.12),
            kind::METAL,
            IRON,
        ));
        lib.decor(piece(
            frame,
            (x - 0.035, x + 0.035),
            (fire_o - 0.24, fire_o - 0.17),
            (0.03, 0.34),
            kind::METAL,
            IRON,
        ));
        lib.decor(piece(
            frame,
            (x - 0.045, x + 0.045),
            (fire_o - 0.25, fire_o - 0.16),
            (0.34, 0.39),
            kind::METAL,
            BRASS,
        ));
    }
    let q = frame.turn();
    for (k, (da, dy, yaw)) in [(0.0f32, 0.17f32, 0.08f32), (-0.12, 0.27, -0.25), (0.15, 0.26, 0.3)]
        .into_iter()
        .enumerate()
    {
        let log = Inst::new(
            frame.at(da, fire_o + 0.02 * k as f32, dy),
            q * Quat::from_rotation_y(1.571 + yaw) * Quat::from_rotation_z(0.05 * k as f32),
            Vec3::new(0.15, 0.13, 1.0),
            kind::WOOD,
            [0.12, 0.06, 0.03, 1.0],
            0,
        );
        lib.decor(log);
    }
    let mut embers = piece(
        frame,
        (-0.55, 0.55),
        (fire_o - 0.2, fire_o + 0.2),
        (0.03, 0.09),
        kind::EMISSIVE,
        [1.0, 0.3, 0.06],
    );
    embers.color[3] = 1.6;
    embers.param = 1;
    lib.decor(embers);
    let flames = [
        (0.0, 0.62, 0.22, 0.0),
        (-0.22, 0.5, 0.17, 0.25),
        (0.24, 0.48, 0.16, -0.3),
        (-0.4, 0.34, 0.12, 0.4),
        (0.42, 0.36, 0.12, -0.35),
        (0.1, 0.42, 0.14, 0.6),
    ];
    for (k, (da, tall, w, lean)) in flames.into_iter().enumerate() {
        let col = if k % 2 == 0 {
            [1.0, 0.42, 0.08]
        } else {
            [1.0, 0.62, 0.18]
        };
        let base = frame.at(da, fire_o - 0.02 + 0.03 * (k % 3) as f32, 0.28);
        let rot = q * Quat::from_rotation_z(lean * 0.4) * Quat::from_rotation_y(0.785);
        let f = Inst::new(
            base + Vec3::Y * (tall * 0.5),
            rot,
            Vec3::new(w, tall, w),
            kind::EMISSIVE,
            [col[0], col[1], col[2], 2.6],
            1,
        );
        lib.decor(f);
    }
    lib.light(
        frame.at(0.0, face - 0.6, 0.8),
        10.0,
        [1.0, 0.55, 0.22],
        2.0,
        LightKind::Flicker,
    );
    // A kettle on a crane over the fire.
    let pivot = frame.at(-open + 0.08, fire_o, 0.0);
    rod(
        lib,
        pivot,
        pivot + Vec3::Y * 1.1,
        0.04,
        0.04,
        kind::METAL,
        rgba(IRON, 1.0),
    );
    rod(
        lib,
        pivot + Vec3::Y * 1.05,
        frame.at(-0.15, fire_o, 1.05),
        0.035,
        0.035,
        kind::METAL,
        rgba(IRON, 1.0),
    );
    rod(
        lib,
        frame.at(-0.2, fire_o, 1.05),
        frame.at(-0.2, fire_o, 0.86),
        0.015,
        0.015,
        kind::METAL,
        rgba(IRON, 1.0),
    );
    let kettle = frame.at(-0.2, fire_o, 0.74);
    lib.decor(Inst::new(
        kettle,
        Quat::from_rotation_y(0.4),
        Vec3::new(0.26, 0.2, 0.26),
        kind::METAL,
        [0.55, 0.3, 0.15, 1.0],
        0,
    ));
    lib.decor(Inst::new(
        kettle + Vec3::Y * 0.12,
        Quat::from_rotation_y(0.4),
        Vec3::new(0.14, 0.05, 0.14),
        kind::METAL,
        [0.55, 0.3, 0.15, 1.0],
        0,
    ));
    rod(
        lib,
        kettle + q * Vec3::new(0.12, 0.0, 0.0),
        kettle + q * Vec3::new(0.24, 0.1, 0.0),
        0.03,
        0.03,
        kind::METAL,
        [0.55, 0.3, 0.15, 1.0],
    );
    // The mantel: a shelf on corbels, candles, a clock, jars of remedies.
    let my = top + 0.45;
    lib.decor(piece(
        frame,
        (-BREAST - 0.2, BREAST + 0.2),
        (face - 0.28, face + 0.12),
        (my - 0.08, my),
        kind::WOOD,
        WALNUT,
    ));
    for s in [-1.0f32, 1.0] {
        let x = s * (BREAST - 0.15);
        lib.decor(piece(
            frame,
            (x - 0.07, x + 0.07),
            (face - 0.2, face),
            (my - 0.32, my - 0.08),
            kind::WOOD,
            WALNUT,
        ));
        candle(lib, frame.at(s * 1.75, face - 0.12, my), 0.24);
        candle(lib, frame.at(s * 1.55, face - 0.12, my), 0.17);
    }
    lib.decor(piece(
        frame,
        (-0.2, 0.2),
        (face - 0.18, face - 0.02),
        (my, my + 0.34),
        kind::WOOD,
        [0.22, 0.1, 0.04],
    ));
    lib.decor(piece(
        frame,
        (-0.13, 0.13),
        (face - 0.185, face - 0.17),
        (my + 0.12, my + 0.3),
        kind::PAPER,
        [0.0; 3],
    ));
    for (k, a) in [-1.1f32, -0.9, 0.85, 1.05].into_iter().enumerate() {
        let col = [[0.3, 0.45, 0.25], [0.5, 0.3, 0.15], [0.35, 0.3, 0.5], [0.55, 0.45, 0.2]][k];
        lib.decor(piece(
            frame,
            (a - 0.06, a + 0.06),
            (face - 0.16, face - 0.04),
            (my, my + 0.16 + 0.03 * k as f32),
            kind::CRYSTAL,
            col,
        ));
        lib.static_insts.last_mut().unwrap().color[3] = 0.1;
    }
    // A painting over it: hills under a moon.
    let (py, ph, pw) = (3.6, 1.3, 2.0);
    lib.decor(piece(
        frame,
        (-pw * 0.5 - 0.1, pw * 0.5 + 0.1),
        (face + 0.07, face + 0.12),
        (py - ph * 0.5 - 0.1, py + ph * 0.5 + 0.1),
        kind::WOOD,
        [0.3, 0.2, 0.06],
    ));
    lib.decor(piece(
        frame,
        (-pw * 0.5, pw * 0.5),
        (face + 0.06, face + 0.07),
        (py - ph * 0.5, py + ph * 0.5),
        kind::PARCHMENT,
        [0.2, 0.25, 0.4],
    ));
    lib.decor(piece(
        frame,
        (-pw * 0.5, pw * 0.5),
        (face + 0.055, face + 0.06),
        (py - ph * 0.5, py - 0.15),
        kind::PARCHMENT,
        [0.2, 0.3, 0.18],
    ));
    let moon = Inst::new(
        frame.at(0.5, face + 0.05, py + 0.3),
        q * Quat::from_rotation_z(0.785),
        Vec3::new(0.16, 0.16, 0.01),
        kind::PARCHMENT,
        [0.9, 0.85, 0.6, 1.0],
        0,
    );
    lib.decor(moon);
}

/// An armchair at `a`, `o` facing along `facing` (±1, across the room): a seat,
/// a high back, arms and short legs, upholstered in `rgb`.
fn armchair(lib: &mut Library, frame: &RoomFrame, a: f32, o: f32, facing: f32, rgb: [f32; 3]) {
    // Laid out with the chair facing +x (across), then turned.
    let q = frame.turn()
        * if facing > 0.0 {
            Quat::IDENTITY
        } else {
            Quat::from_rotation_y(std::f32::consts::PI)
        };
    let c = frame.at(a, o, 0.0);
    let put = |lib: &mut Library, off: Vec3, size: Vec3, kind: u32, col: [f32; 3]| {
        lib.decor(Inst::new(c + q * off, q, size, kind, rgba(col, 1.0), 0));
    };
    put(
        lib,
        Vec3::new(0.0, 0.32, 0.0),
        Vec3::new(0.7, 0.22, 0.62),
        kind::CLOTH,
        rgb,
    );
    put(
        lib,
        Vec3::new(0.03, 0.47, 0.0),
        Vec3::new(0.6, 0.1, 0.54),
        kind::CLOTH,
        [rgb[0] * 1.15, rgb[1] * 1.15, rgb[2] * 1.15],
    );
    put(
        lib,
        Vec3::new(-0.33, 0.78, 0.0),
        Vec3::new(0.18, 0.95, 0.8),
        kind::CLOTH,
        rgb,
    );
    put(
        lib,
        Vec3::new(-0.36, 1.27, 0.0),
        Vec3::new(0.16, 0.14, 0.88),
        kind::CLOTH,
        rgb,
    );
    for s in [-1.0f32, 1.0] {
        put(
            lib,
            Vec3::new(0.03, 0.55, s * 0.38),
            Vec3::new(0.74, 0.2, 0.15),
            kind::CLOTH,
            rgb,
        );
        put(
            lib,
            Vec3::new(0.37, 0.6, s * 0.38),
            Vec3::new(0.08, 0.12, 0.17),
            kind::WOOD,
            WALNUT,
        );
        for x in [-0.3f32, 0.3] {
            put(
                lib,
                Vec3::new(x, 0.1, s * 0.34),
                Vec3::new(0.07, 0.2, 0.07),
                kind::WOOD,
                DARK_WOOD,
            );
        }
    }
    guard(lib, frame, (a - 0.45, a + 0.45), (o - 0.47, o + 0.47), 1.35);
}

/// A sofa across the inglenook's mouth at `a`, `o`, facing the fire, cushions on it.
fn sofa(lib: &mut Library, frame: &RoomFrame, a: f32, o: f32) {
    let (hw, hd) = (1.5, 0.45);
    lib.decor(piece(
        frame,
        (a - hw, a + hw),
        (o - hd, o + hd),
        (0.1, 0.42),
        kind::CLOTH,
        UPHOLSTERY,
    ));
    for k in 0..3 {
        let x = a - hw + 0.18 + k as f32 * (2.0 * hw - 0.36) / 3.0;
        let cushion = (x + 0.01, x + (2.0 * hw - 0.36) / 3.0 - 0.01);
        lib.decor(piece(
            frame,
            cushion,
            (o - hd + 0.24, o + hd - 0.02),
            (0.42, 0.53),
            kind::CLOTH,
            [0.5, 0.13, 0.09],
        ));
    }
    lib.decor(piece(
        frame,
        (a - hw, a + hw),
        (o - hd - 0.02, o - hd + 0.22),
        (0.42, 0.98),
        kind::CLOTH,
        UPHOLSTERY,
    ));
    for s in [-1.0f32, 1.0] {
        let x = a + s * (hw - 0.09);
        lib.decor(piece(
            frame,
            (x - 0.1, x + 0.1),
            (o - hd, o + hd),
            (0.42, 0.66),
            kind::CLOTH,
            UPHOLSTERY,
        ));
        for y in [o - hd + 0.06, o + hd - 0.06] {
            lib.decor(piece(
                frame,
                (x - 0.035, x + 0.035),
                (y - 0.035, y + 0.035),
                (0.0, 0.1),
                kind::WOOD,
                DARK_WOOD,
            ));
        }
    }
    // Cushions leaning in its corners, a throw over its back.
    for (s, col) in [(-1.0f32, [0.75, 0.6, 0.25]), (1.0, [0.2, 0.32, 0.22])] {
        let p = frame.at(a + s * (hw - 0.4), o - hd + 0.32, 0.72);
        let q = frame.turn() * Quat::from_rotation_x(-0.3) * Quat::from_rotation_z(s * 0.2);
        lib.decor(Inst::new(
            p,
            q,
            Vec3::new(0.4, 0.38, 0.12),
            kind::CLOTH,
            rgba(col, 1.0),
            0,
        ));
    }
    lib.decor(piece(
        frame,
        (a - 0.9, a - 0.1),
        (o - hd - 0.03, o - hd + 0.25),
        (0.6, 1.0),
        kind::CLOTH,
        [0.55, 0.45, 0.3],
    ));
    guard(lib, frame, (a - hw, a + hw), (o - hd - 0.03, o + hd), 1.0);
}

/// A low round tea table at `a`, `o`: a teapot and cups, a candle.
fn tea_table(lib: &mut Library, frame: &RoomFrame, a: f32, o: f32) {
    let top = 0.5;
    for k in 0..2 {
        let q = Quat::from_rotation_y(0.2 + k as f32 * 0.785);
        lib.decor(Inst::new(
            frame.at(a, o, top - 0.025 + 0.002 * k as f32),
            q,
            Vec3::new(0.62, 0.05, 0.62),
            kind::WOOD,
            rgba(WALNUT, 1.0),
            0,
        ));
    }
    lib.decor(piece(
        frame,
        (a - 0.05, a + 0.05),
        (o - 0.05, o + 0.05),
        (0.06, top - 0.05),
        kind::WOOD,
        DARK_WOOD,
    ));
    lib.decor(Inst::new(
        frame.at(a, o, 0.03),
        Quat::from_rotation_y(0.5),
        Vec3::new(0.4, 0.06, 0.4),
        kind::WOOD,
        rgba(DARK_WOOD, 1.0),
        0,
    ));
    guard(lib, frame, (a - 0.36, a + 0.36), (o - 0.36, o + 0.36), top);
    let china = [0.85, 0.85, 0.8];
    let pot = frame.at(a - 0.08, o - 0.05, top + 0.09);
    lib.decor(Inst::new(
        pot,
        Quat::from_rotation_y(0.6),
        Vec3::new(0.16, 0.14, 0.16),
        kind::PLAIN,
        rgba(china, 1.0),
        0,
    ));
    lib.decor(Inst::new(
        pot + Vec3::Y * 0.09,
        Quat::from_rotation_y(0.6),
        Vec3::new(0.06, 0.04, 0.06),
        kind::PLAIN,
        rgba(china, 1.0),
        0,
    ));
    rod(
        lib,
        pot + frame.turn() * Vec3::new(0.07, -0.02, 0.0),
        pot + frame.turn() * Vec3::new(0.15, 0.05, 0.0),
        0.025,
        0.025,
        kind::PLAIN,
        rgba(china, 1.0),
    );
    for (da, d_o) in [(0.17f32, 0.12f32), (0.1, -0.2)] {
        lib.decor(piece(
            frame,
            (a + da - 0.04, a + da + 0.04),
            (o + d_o - 0.04, o + d_o + 0.04),
            (top, top + 0.06),
            kind::PLAIN,
            china,
        ));
    }
    candle(lib, frame.at(a + 0.15, o + 0.2, top), 0.12);
}

/// A small side table at `a`, `o` with a candle lamp on it.
fn lamp_table(lib: &mut Library, frame: &RoomFrame, a: f32, o: f32) {
    let top = 0.66;
    lib.decor(piece(
        frame,
        (a - 0.25, a + 0.25),
        (o - 0.25, o + 0.25),
        (top - 0.04, top),
        kind::WOOD,
        WALNUT,
    ));
    for (da, d_o) in [(-0.2, -0.2), (0.2, -0.2), (-0.2, 0.2), (0.2, 0.2)] {
        let (x, y) = (a + da, o + d_o);
        lib.decor(piece(
            frame,
            (x - 0.025, x + 0.025),
            (y - 0.025, y + 0.025),
            (0.0, top - 0.04),
            kind::WOOD,
            DARK_WOOD,
        ));
    }
    guard(lib, frame, (a - 0.26, a + 0.26), (o - 0.26, o + 0.26), top);
    let base = frame.at(a, o, top);
    lib.decor(Inst::aabox(
        base + Vec3::Y * 0.02,
        Vec3::new(0.14, 0.04, 0.14),
        kind::METAL,
        BRASS,
    ));
    candle(lib, base + Vec3::Y * 0.04, 0.2);
    lib.decor(Inst::aabox(
        base + Vec3::Y * 0.36,
        Vec3::new(0.16, 0.04, 0.16),
        kind::METAL,
        BRASS,
    ));
    for (dx, dz) in [(-0.07, -0.07), (0.07, -0.07), (-0.07, 0.07), (0.07, 0.07)] {
        lib.decor(Inst::aabox(
            base + Vec3::new(dx, 0.2, dz),
            Vec3::new(0.012, 0.32, 0.012),
            kind::METAL,
            BRASS,
        ));
    }
    lib.light(base + Vec3::Y * 0.3, 4.5, [1.0, 0.62, 0.3], 0.7, LightKind::Flicker);
}

/// A basket of wool by a chair at `a`, `o`, and over it a pair of needles knitting
/// a scarf by themselves: one of the room's two bits of magic.
fn knitting(lib: &mut Library, frame: &RoomFrame, a: f32, o: f32) {
    lib.decor(piece(
        frame,
        (a - 0.2, a + 0.2),
        (o - 0.2, o + 0.2),
        (0.0, 0.26),
        kind::WOOD,
        [0.45, 0.32, 0.15],
    ));
    guard(lib, frame, (a - 0.21, a + 0.21), (o - 0.21, o + 0.21), 0.3);
    for (k, col) in [[0.55, 0.15, 0.12], [0.2, 0.3, 0.5], [0.75, 0.65, 0.3]]
        .into_iter()
        .enumerate()
    {
        let p = frame.at(a - 0.08 + 0.09 * k as f32, o - 0.04 + 0.05 * (k % 2) as f32, 0.28);
        lib.decor(Inst::new(
            p,
            Quat::from_rotation_y(k as f32),
            Vec3::splat(0.12),
            kind::CLOTH,
            rgba(col, 1.0),
            0,
        ));
    }
    let q = frame.turn();
    let hold = frame.at(a, o, 1.25);
    for s in [-1.0f32, 1.0] {
        let tip = hold + q * Vec3::new(s * 0.1, 0.1, 0.0);
        let end = hold + q * Vec3::new(-s * 0.12, -0.08, 0.03 * s);
        rod(lib, end, tip, 0.012, 0.012, kind::CRYSTAL, [0.85, 0.75, 1.0, 1.2]);
    }
    // The scarf so far, hanging down to the basket, and its thread.
    lib.decor(Inst::new(
        hold - Vec3::Y * 0.28,
        q * Quat::from_rotation_z(0.06),
        Vec3::new(0.18, 0.42, 0.02),
        kind::CLOTH,
        [0.55, 0.15, 0.12, 1.0],
        0,
    ));
    rod(
        lib,
        hold - Vec3::Y * 0.5,
        frame.at(a - 0.08, o - 0.04, 0.32),
        0.006,
        0.006,
        kind::CLOTH,
        [0.55, 0.15, 0.12, 1.0],
    );
    lib.light(hold, 2.0, [0.8, 0.7, 1.0], 0.35, LightKind::Static);
}

/// A spinning wheel at `a`, `o`, its wheel turning across the room.
fn spinning_wheel(lib: &mut Library, frame: &RoomFrame, a: f32, o: f32) {
    let wood = rgba([0.35, 0.2, 0.08], 1.0);
    lib.decor(piece(
        frame,
        (a - 0.15, a + 0.15),
        (o - 0.5, o + 0.5),
        (0.25, 0.33),
        kind::WOOD,
        [0.35, 0.2, 0.08],
    ));
    for (da, d_o) in [(0.0, -0.45), (-0.13, 0.4), (0.13, 0.4)] {
        rod(
            lib,
            frame.at(a + da, o + d_o, 0.0),
            frame.at(a + da * 0.6, o + d_o * 0.8, 0.3),
            0.04,
            0.04,
            kind::WOOD,
            wood,
        );
    }
    guard(lib, frame, (a - 0.2, a + 0.2), (o - 0.55, o + 0.55), 1.2);
    // The wheel turns about the across axis, upright along the room.
    let hub = frame.at(a, o - 0.15, 0.78);
    let axis = frame.across();
    let r = 0.36;
    let segs = 14;
    for k in 0..segs {
        let th = 0.1 + k as f32 / segs as f32 * TAU;
        let radial = frame.out() * libm::cosf(th) + Vec3::Y * libm::sinf(th);
        let rot = Quat::from_mat3(&Mat3::from_cols(radial, axis.cross(radial), axis));
        let len = 2.0 * r * libm::tanf(TAU / segs as f32 * 0.5) + 0.01;
        lib.decor(Inst::new(
            hub + radial * r,
            rot,
            Vec3::new(0.04, len, 0.035),
            kind::WOOD,
            wood,
            0,
        ));
        if k % 2 == 0 {
            lib.decor(Inst::new(
                hub + radial * (r * 0.5),
                rot,
                Vec3::new(r, 0.015, 0.015),
                kind::WOOD,
                wood,
                0,
            ));
        }
    }
    rod(lib, frame.at(a, o - 0.15, 0.3), hub, 0.05, 0.05, kind::WOOD, wood);
    rod(
        lib,
        frame.at(a, o + 0.35, 0.33),
        frame.at(a, o + 0.4, 0.95),
        0.04,
        0.04,
        kind::WOOD,
        wood,
    );
    lib.decor(Inst::aabox(
        frame.at(a, o + 0.4, 0.98),
        Vec3::splat(0.08),
        kind::CLOTH,
        [0.85, 0.8, 0.7],
    ));
}

/// A barrel of ale by the door, a tap in it.
fn barrel(lib: &mut Library, frame: &RoomFrame, a: f32, o: f32) {
    let r = 0.33;
    let segs = 8;
    for k in 0..segs {
        let th = 0.2 + k as f32 / segs as f32 * TAU;
        let radial = frame.across() * libm::cosf(th) + frame.out() * libm::sinf(th);
        let q = Quat::from_mat3(&Mat3::from_cols(radial, Vec3::Y, radial.cross(Vec3::Y)));
        let stave = 2.0 * r * libm::tanf(TAU / segs as f32 * 0.5) + 0.02;
        lib.decor(Inst::new(
            frame.at(a, o, 0.45) + radial * r,
            q,
            Vec3::new(0.05, 0.9, stave),
            kind::WOOD,
            [0.38, 0.22, 0.1, 1.0],
            0,
        ));
        for y in [0.15f32, 0.75] {
            lib.decor(Inst::new(
                frame.at(a, o, y) + radial * (r + 0.02),
                q,
                Vec3::new(0.02, 0.06, stave + 0.01),
                kind::METAL,
                rgba(IRON, 1.0),
                0,
            ));
        }
    }
    lib.decor(Inst::new(
        frame.at(a, o, 0.88),
        Quat::from_rotation_y(0.2),
        Vec3::new(0.62, 0.03, 0.62),
        kind::WOOD,
        [0.3, 0.17, 0.08, 1.0],
        0,
    ));
    rod(
        lib,
        frame.at(a, o + r, 0.3),
        frame.at(a, o + r + 0.1, 0.3),
        0.04,
        0.04,
        kind::METAL,
        rgba(BRASS, 1.0),
    );
    guard(
        lib,
        frame,
        (a - r - 0.03, a + r + 0.03),
        (o - r - 0.03, o + r + 0.03),
        0.9,
    );
}

/// Rugs: one before the fire, one under the sofa, one in each reading corner.
fn rugs(lib: &mut Library, frame: &RoomFrame) {
    let far = RoomFrame::O_FAR;
    let rug = |a: (f32, f32), o: (f32, f32), base: [f32; 3], second: u32, y: f32| {
        let mut r = piece(frame, a, o, (0.0, y), kind::RUG, base);
        r.param = second;
        r
    };
    let deep_red = [0.32, 0.06, 0.05];
    let gold = 190 | (140 << 8) | (60 << 16);
    let green = 70 | (110 << 8) | (70 << 16);
    lib.rug(rug((-1.75, 1.75), (25.9, far - BREAST_D - 0.6), deep_red, gold, 0.02));
    lib.rug(rug((-2.2, 2.2), (21.4, 24.6), [0.12, 0.18, 0.12], gold, 0.024));
    lib.rug(rug((-9.6, -6.6), (24.6, 28.2), [0.25, 0.12, 0.05], green, 0.02));
    lib.rug(rug((6.6, 9.6), (25.6, 29.2), [0.25, 0.12, 0.05], green, 0.02));
}

/// Candles hanging in the air over the hearth and the sofa, unsupported: the room's
/// chief bit of magic.
fn floating_candles(lib: &mut Library, frame: &RoomFrame) {
    let mut rng = StdRng::seed_from_u64(0x4341_4e44);
    for _ in 0..14 {
        let a = rng.gen_range(-2.4..2.4);
        let o = rng.gen_range(22.0..29.5);
        let y = rng.gen_range(3.1..4.3);
        let h = rng.gen_range(0.14..0.3);
        candle(lib, frame.at(a, o, y), h);
    }
    lib.light(frame.at(0.0, 25.5, 3.6), 6.0, [1.0, 0.7, 0.4], 0.8, LightKind::Flicker);
}

/// Heavy beams across the ceiling, bundles of herbs hung drying from two of them.
fn ceiling(lib: &mut Library, frame: &RoomFrame) {
    let h = RoomFrame::HEIGHT;
    let inner = RoomFrame::A_HALF;
    let beams = [16.0, 18.9, 21.8, 24.7, 27.6, 30.5];
    for o in beams {
        lib.decor(piece(
            frame,
            (-inner, inner),
            (o - 0.16, o + 0.16),
            (h - 0.36, h),
            kind::WOOD,
            [0.09, 0.045, 0.02],
        ));
    }
    let mut rng = StdRng::seed_from_u64(0x4845_5242);
    for o in [beams[1], beams[2]] {
        for k in 0..7 {
            let a = -5.2 + k as f32 * 1.7 + rng.gen_range(-0.3..0.3);
            let len = rng.gen_range(0.3..0.55);
            let top = h - 0.36;
            rod(
                lib,
                frame.at(a, o, top),
                frame.at(a, o, top - 0.15),
                0.01,
                0.01,
                kind::CLOTH,
                [0.5, 0.42, 0.3, 1.0],
            );
            let green = [
                rng.gen_range(0.18..0.3),
                rng.gen_range(0.3..0.42),
                rng.gen_range(0.12..0.2),
            ];
            // A bunch hung head down: two crossed sprays, narrow at the tie.
            let spin = rng.gen_range(0.0..3.0f32);
            let mid = frame.at(a, o, top - 0.17 - len * 0.5);
            for k in 0..2 {
                let q = Quat::from_rotation_y(spin + k as f32 * 1.571);
                lib.decor(Inst::new(
                    mid,
                    q,
                    Vec3::new(0.17, len, 0.025),
                    kind::LEAF,
                    rgba(green, 1.0),
                    0,
                ));
                let neck = mid + Vec3::Y * (len * 0.5 + 0.01);
                lib.decor(Inst::new(
                    neck,
                    q,
                    Vec3::new(0.07, 0.06, 0.03),
                    kind::LEAF,
                    rgba(green, 1.0),
                    0,
                ));
            }
            let tie = frame.at(a, o, top - 0.15);
            lib.decor(Inst::new(
                tie,
                Quat::from_rotation_y(spin),
                Vec3::new(0.05, 0.03, 0.05),
                kind::CLOTH,
                [0.5, 0.42, 0.3, 1.0],
                0,
            ));
        }
    }
}
