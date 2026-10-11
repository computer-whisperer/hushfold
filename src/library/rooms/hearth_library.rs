//! The Hearth Library: department 11, top floor, west side. Wall cases all round
//! the room; at the far wall an inglenook, two runs of cases standing out either
//! side of a great fireplace, its fire burning and its smoke going up the flue,
//! armchairs and a tea table on a rug before it, a sofa across its mouth; two short
//! screens of cases by the door; reading chairs, a spinning wheel turning and a
//! barrel about the room, herbs drying from the beams and stirring in the warm air.
//! Candles float over the hearth, bobbing, and a pair of needles knits on its own by
//! a chair. A candle stands unlit on each case side's cornice and lights when that
//! side is filled; once the room is in order the fire leaps up and settles fuller, the
//! kettle sings, and a cat comes to sleep in the chair by the knitting.
//!
//! The candles, flames and turned pieces the top floor's rooms share are made here.

use super::*;
use crate::audio::Sfx;
use std::f32::consts::{FRAC_PI_2, PI};

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
const COPPER: [f32; 3] = [0.55, 0.3, 0.15];
const CHINA: [f32; 3] = [0.85, 0.85, 0.8];
const UPHOLSTERY: [f32; 3] = [0.28, 0.065, 0.05];
const GREEN_CLOTH: [f32; 3] = [0.14, 0.26, 0.14];
/// Candle wax, and a candle's flame: its colour and brightness.
pub(super) const WAX: [f32; 3] = [0.85, 0.8, 0.65];
pub(super) const FLAME: [f32; 4] = [1.0, 0.62, 0.25, 4.0];

pub(super) fn furnish(lib: &mut Library, frame: &RoomFrame, rng: &mut StdRng, titles: &mut HashSet<String>) {
    debug_assert!(
        frame.door.abs() < 1.0,
        "the Hearth Library wants its door in the middle"
    );
    shelves(lib, frame, rng, titles);
    case_candles(lib, frame);
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
    sleeping_cat(lib, frame, -8.3, 26.6, 1.0);
    lamp_table(lib, frame, -8.3, 25.45);
    knitting(lib, frame, -7.35, 26.3);
    armchair(lib, frame, 8.3, 27.4, -1.0, GREEN_CLOTH);
    lamp_table(lib, frame, 8.3, 28.55);
    spinning_wheel(lib, frame, 7.9, 24.4);
    barrel(lib, frame, -6.2, 14.75);
    floating_candles(lib, frame);
    ceiling(lib, frame);
}

/// Down where its kind is drawn: in the translucent pass (flames, mist, clear glass)
/// or with the rest.
pub(super) fn lay(lib: &mut Library, i: Inst) {
    if kind::translucent(i.kind) {
        lib.translucent(i);
    } else {
        lib.decor(i);
    }
}

/// A turned leg 1 high and 1 across at its widest (the block at its top), to be sized.
pub(super) fn turned_leg(lib: &mut Library) -> Shape {
    let profile = [
        (0.0, 0.0),
        (0.3, 0.0),
        (0.34, 0.03),
        (0.26, 0.07),
        (0.2, 0.2),
        (0.24, 0.34),
        (0.36, 0.44),
        (0.24, 0.55),
        (0.19, 0.68),
        (0.24, 0.8),
        (0.34, 0.84),
        (0.5, 0.86),
        (0.5, 1.0),
        (0.0, 1.0),
    ];
    lib.turned(&profile, 12)
}

/// A bun foot 1 high and 1 across, to be sized.
fn bun_foot(lib: &mut Library) -> Shape {
    let profile = [
        (0.0, 0.0),
        (0.3, 0.0),
        (0.46, 0.12),
        (0.5, 0.32),
        (0.44, 0.52),
        (0.32, 0.62),
        (0.3, 0.82),
        (0.4, 0.86),
        (0.4, 1.0),
        (0.0, 1.0),
    ];
    lib.turned(&profile, 12)
}

/// A candle standing at `p`, `h` tall: its wax and its flame, for laying down (`lay`)
/// or for a mover to carry.
fn candle_parts(lib: &mut Library, p: Vec3, h: f32) -> [Inst; 2] {
    [
        wax(lib)
            .sized(Vec3::new(0.045, h, 0.045))
            .at(p, Quat::IDENTITY, kind::PLAIN, rgba(WAX, 1.0)),
        flame(p + Vec3::Y * (h + 0.004), (0.026, 0.06), FLAME),
    ]
}

/// A candle's wax 1 high and 1 across, to be sized.
fn wax(lib: &mut Library) -> Shape {
    lib.turned(&[(0.0, 0.0), (0.5, 0.0), (0.5, 0.95), (0.4, 1.0), (0.0, 0.97)], 12)
}

/// A candle standing at `p`, `h` tall, lit.
pub(super) fn candle(lib: &mut Library, p: Vec3, h: f32) {
    for i in candle_parts(lib, p, h) {
        lay(lib, i);
    }
}

/// A turned candlestick 1 high and 1 across at its foot, to be sized.
fn candlestick(lib: &mut Library) -> Shape {
    lib.turned(
        &[
            (0.0, 0.0),
            (0.5, 0.0),
            (0.5, 0.08),
            (0.2, 0.18),
            (0.14, 0.5),
            (0.2, 0.74),
            (0.16, 0.84),
            (0.36, 0.92),
            (0.36, 1.0),
            (0.0, 1.0),
        ],
        16,
    )
}

/// A candle in a hurricane glass on a brass foot, standing at `p`.
pub(super) fn hurricane_lamp(lib: &mut Library, p: Vec3) {
    let foot = lib.turned(
        &[
            (0.0, 0.0),
            (0.075, 0.0),
            (0.075, 0.012),
            (0.06, 0.025),
            (0.052, 0.04),
            (0.0, 0.04),
        ],
        24,
    );
    lib.decor(foot.at(p, Quat::IDENTITY, kind::METAL, rgba(BRASS, 1.0)));
    candle(lib, p + Vec3::Y * 0.04, 0.16);
    let glass = lib.turned(
        &[
            (0.046, 0.0),
            (0.06, 0.05),
            (0.07, 0.13),
            (0.058, 0.22),
            (0.045, 0.28),
            (0.046, 0.3),
        ],
        24,
    );
    lib.translucent(glass.at(p + Vec3::Y * 0.04, Quat::IDENTITY, kind::CLEAR, [0.9, 0.85, 0.75, 0.08]));
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

/// On each case side's cornice, at an end clear of the signs, a candle in a brass
/// stick, unlit until that side is filled; then its wick catches and it burns on, so
/// the room lights up case by case as it is put in order.
fn case_candles(lib: &mut Library, frame: &RoomFrame) {
    let stick = candlestick(lib).sized(Vec3::new(0.08, 0.11, 0.08));
    let wax = wax(lib);
    let brass = rgba(BRASS, 1.0);
    // The room's signs, each 2 m wide: over a case, or between two where a category
    // stands in parts apart.
    let signs: Vec<(Vec3, Vec3)> = (lib.labels.iter())
        .filter(|l| matches!(l.kind, LabelKind::Category(c) if frame.categories.contains(&c)))
        .map(|l| (l.pos, l.normal))
        .collect();
    let clear = |p: Vec3| {
        (signs.iter()).all(|&(at, n)| (p - at).dot(Vec3::Y.cross(n)).abs() > 1.1 || (p - at).dot(n).abs() > 0.2)
    };
    for cat in frame.categories.clone() {
        for f in lib.faces_of(cat).collect::<Vec<_>>() {
            let face = &lib.faces[f];
            let foot = [1.15, -1.15]
                .map(|e| face.center + face.right * e + Vec3::Y * (UNIT_H + 0.13))
                .into_iter()
                .find(|&p| clear(p))
                .expect("a sign covers only one end of a case");
            lib.decor(stick.at(foot, Quat::IDENTITY, kind::METAL, brass));
            let (base, tall) = (foot + Vec3::Y * 0.11, 0.22);
            lib.decor(
                wax.sized(Vec3::new(0.06, tall, 0.06))
                    .at(base, Quat::IDENTITY, kind::PLAIN, rgba(WAX, 1.0)),
            );
            let wick = base + Vec3::Y * (tall + 0.004);
            let catch = Change::Arrives {
                from: Vec3::ZERO,
                grow: 0.1,
            };
            let side = Finish::Face(f);
            let lit = flame(wick, (0.046, 0.11), [FLAME[0], FLAME[1], FLAME[2], 6.0]);
            lib.waiting(wick, Awaits::new(side, catch).after(0.4).over(0.8), [lit]);
            lib.sparks(Sparks {
                delay: 0.35,
                lasts: Some(0.4),
                spread: Vec3::splat(0.015),
                rate: 40.0,
                vel: Vec3::Y * 0.5,
                jitter: Vec3::splat(0.3),
                life: 0.9,
                size: Vec3::splat(0.012),
                ..Sparks::burst(side, wick)
            });
        }
    }
}

/// Panelling where the walls show.
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
/// burning on firedogs, smoke going up the flue and a kettle hung over the fire, a
/// hearthstone, a mantel with candles, a clock and jars, a painting over it.
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
    // Firedogs with brass knobs, logs, embers, the fire and its smoke.
    let fire_o = far - 0.27;
    let knob = lib.shape(Mesh::sphere(12));
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
        lib.decor(knob.sized(Vec3::splat(0.085)).inst(
            frame.at(x, fire_o - 0.205, 0.375),
            Quat::IDENTITY,
            kind::METAL,
            rgba(BRASS, 1.0),
        ));
    }
    let q = frame.turn();
    let log = lib.shape(Mesh::cylinder(12));
    for (k, (da, dy, yaw)) in [(0.0f32, 0.17f32, 0.08f32), (-0.12, 0.27, -0.25), (0.15, 0.26, 0.3)]
        .into_iter()
        .enumerate()
    {
        let lie = q
            * Quat::from_rotation_y(FRAC_PI_2 + yaw)
            * Quat::from_rotation_z(0.05 * k as f32)
            * Quat::from_rotation_x(FRAC_PI_2);
        lib.decor(log.sized(Vec3::new(0.15, 1.0, 0.13)).inst(
            frame.at(da, fire_o + 0.02 * k as f32, dy),
            lie,
            kind::WOOD,
            [0.12, 0.06, 0.03, 1.0],
        ));
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
    // A broad body of flame over the logs, a tongue either side.
    for (da, d_o, w, tall, yaw) in [
        (0.0f32, 0.0f32, 0.95f32, 0.72f32, 0.0f32),
        (-0.34, 0.03, 0.42, 0.5, 0.35),
        (0.32, -0.02, 0.4, 0.56, -0.3),
    ] {
        lib.translucent(Inst::new(
            frame.at(da, fire_o + d_o, 0.12 + tall * 0.5),
            q * Quat::from_rotation_y(yaw),
            Vec3::new(w, tall, 0.34),
            kind::FIRE,
            [1.0, 0.48, 0.14, 1.5],
            1,
        ));
    }
    lib.translucent(Inst::new(
        frame.at(0.0, far - 0.25, 1.08),
        q,
        Vec3::new(0.8, 0.5, 0.3),
        kind::MIST,
        [0.3, 0.28, 0.27, 0.6],
        0,
    ));
    lib.light(
        frame.at(0.0, face - 0.6, 0.8),
        10.0,
        [1.0, 0.55, 0.22],
        2.0,
        LightKind::Flicker,
    );
    // A kettle on a crane over the fire.
    let iron = rgba(IRON, 1.0);
    let pivot = frame.at(-open + 0.08, fire_o, 0.0);
    lib.pole(pivot, pivot + Vec3::Y * 1.1, 0.04, kind::METAL, iron);
    lib.pole(
        pivot + Vec3::Y * 1.05,
        frame.at(-0.15, fire_o, 1.05),
        0.035,
        kind::METAL,
        iron,
    );
    lib.pole(
        frame.at(-0.2, fire_o, 1.07),
        frame.at(-0.2, fire_o, 0.9),
        0.015,
        kind::METAL,
        iron,
    );
    let (kettle_at, kettle_turn) = (frame.at(-0.2, fire_o, 0.6), q * Quat::from_rotation_y(0.4));
    kettle(lib, kettle_at, kettle_turn);
    // Once the room is in order the fire leaps up, embers swirling up the flue, and
    // sinks back, leaving it burning fuller than before and the room the warmer for it;
    // the kettle over it begins to sing.
    let fin = Finish::Room(frame.dept);
    let logs = frame.at(0.0, fire_o, 0.12);
    let tongue = |w: f32, tall: f32, glow: f32| {
        Inst::new(
            logs + Vec3::Y * (tall * 0.5),
            q,
            Vec3::new(w, tall, 0.36),
            kind::FIRE,
            [1.0, 0.5, 0.16, glow],
            1,
        )
    };
    let up = Change::Arrives {
        from: Vec3::ZERO,
        grow: 0.1,
    };
    let leapt = lib.waiter(logs, Awaits::new(fin, up).over(1.2));
    let sink = Change::Shifts {
        by: Vec3::ZERO,
        turn: Quat::IDENTITY,
        scale: 0.0,
    };
    let sunk = lib.mover(logs, Motion::Still, 0.0, Some(leapt));
    lib.awaiting(sunk, Awaits::new(fin, sink).after(2.2).over(3.0));
    lib.moving(sunk, tongue(1.1, 1.15, 2.2));
    lib.waiting(
        logs,
        Awaits::new(fin, up).after(1.5).over(3.0),
        [tongue(0.6, 0.62, 1.3)],
    );
    let warm = Change::Kindles { dim: 0.0 };
    let warmer = lib.waiter(logs, Awaits::new(fin, warm).over(2.0));
    lib.light_riding(
        warmer,
        frame.at(0.0, face - 0.7, 0.9),
        9.0,
        [1.0, 0.55, 0.22],
        1.0,
        LightKind::Flicker,
    );
    lib.sparks(Sparks {
        lasts: Some(3.5),
        spread: frame.size(0.4, 0.08, 0.12),
        rate: 45.0,
        vel: Vec3::Y * 1.2,
        jitter: Vec3::new(0.3, 0.4, 0.3),
        lift: 0.6,
        drag: 0.8,
        wander: 1.2,
        life: 1.8,
        rgba: [1.6, 0.55, 0.12, 3.0],
        size: Vec3::splat(0.018),
        ..Sparks::burst(fin, frame.at(0.0, fire_o - 0.05, 0.45))
    });
    // And from then on motes of warm light drifting about the inglenook.
    lib.sparks(Sparks::motes(
        fin,
        frame.at(0.0, far - 3.0, 1.6),
        frame.size(2.6, 0.9, 2.2),
    ));
    let spout = kettle_at + kettle_turn * Vec3::new(0.22, 0.19, 0.0);
    let steam = Inst::new(
        spout + Vec3::Y * 0.2,
        Quat::IDENTITY,
        Vec3::new(0.09, 0.45, 0.09),
        kind::MIST,
        [0.85, 0.85, 0.85, 0.6],
        0,
    );
    let sing = Change::Arrives {
        from: Vec3::ZERO,
        grow: 0.0,
    };
    lib.waiting(spout, Awaits::new(fin, sing).after(2.5).over(2.5), [steam]);
    lib.cue(Cue {
        finish: fin,
        delay: 1.0,
        sound: Sfx::Chime,
        at: logs,
        volume: 0.5,
    });
    // The mantel: a shelf on corbels, candles in brass sticks, a clock, jars of
    // remedies.
    let my = top + 0.45;
    lib.decor(piece(
        frame,
        (-BREAST - 0.2, BREAST + 0.2),
        (face - 0.28, face + 0.12),
        (my - 0.08, my),
        kind::WOOD,
        WALNUT,
    ));
    let stick = candlestick(lib);
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
        for (a, tall) in [(1.75, 0.24), (1.55, 0.17)] {
            let foot = frame.at(s * a, face - 0.12, my);
            lib.decor(
                stick
                    .sized(Vec3::new(0.09, 0.14, 0.09))
                    .at(foot, Quat::IDENTITY, kind::METAL, rgba(BRASS, 1.0)),
            );
            candle(lib, foot + Vec3::Y * 0.14, tall);
        }
    }
    mantel_clock(lib, frame, frame.at(0.0, face - 0.1, my), -frame.out());
    let jar = lib.turned(
        &[
            (0.0, 0.0),
            (0.47, 0.0),
            (0.5, 0.06),
            (0.5, 0.84),
            (0.4, 0.92),
            (0.36, 1.0),
        ],
        16,
    );
    let plug = lib.shape(Mesh::cylinder(12));
    for (k, a) in [-1.1f32, -0.9, 0.85, 1.05].into_iter().enumerate() {
        let col = [[0.3, 0.45, 0.25], [0.5, 0.3, 0.15], [0.35, 0.3, 0.5], [0.55, 0.45, 0.2]][k];
        let tall = 0.16 + 0.03 * k as f32;
        let foot = frame.at(a, face - 0.1, my);
        lib.translucent(jar.sized(Vec3::new(0.11, tall, 0.11)).at(
            foot,
            Quat::IDENTITY,
            kind::CLEAR,
            [0.8, 0.88, 0.8, 0.12],
        ));
        let fill = tall * (0.45 + 0.15 * (k % 2) as f32);
        lib.decor(plug.sized(Vec3::new(0.095, fill, 0.095)).inst(
            foot + Vec3::Y * (0.006 + fill * 0.5),
            Quat::IDENTITY,
            kind::PLAIN,
            rgba(col, 1.0),
        ));
        lib.decor(plug.sized(Vec3::new(0.08, 0.035, 0.08)).inst(
            foot + Vec3::Y * (tall + 0.008),
            Quat::IDENTITY,
            kind::WOOD,
            [0.5, 0.36, 0.2, 1.0],
        ));
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

/// A copper kettle with its foot at `p`, turned by `rot` (its spout along x): a round
/// body, a lid with a knob, a spout, and a bail over it to hang it by.
fn kettle(lib: &mut Library, p: Vec3, rot: Quat) {
    let mut m = Mesh::lathe(
        &[
            (0.0, 0.0),
            (0.09, 0.0),
            (0.12, 0.03),
            (0.135, 0.08),
            (0.125, 0.13),
            (0.09, 0.17),
            (0.05, 0.185),
            (0.05, 0.2),
            (0.03, 0.205),
            (0.025, 0.225),
            (0.0, 0.23),
        ],
        24,
    );
    let spout: Vec<Vec3> = (0..6)
        .map(|k| {
            let t = k as f32 / 5.0;
            Vec3::new(0.1 + 0.12 * t, 0.05 + 0.1 * t + 0.04 * t * t, 0.0)
        })
        .collect();
    m.add(
        &Mesh::sweep(&spout, &[0.022, 0.011], &circle(1.0, 8), true),
        Vec3::ZERO,
        Quat::IDENTITY,
        Vec3::ONE,
    );
    let bail: Vec<Vec3> = (0..=12)
        .map(|k| {
            let a = k as f32 / 12.0 * PI;
            Vec3::new(0.0, 0.17 + 0.13 * libm::sinf(a), 0.13 * libm::cosf(a))
        })
        .collect();
    m.add(
        &Mesh::sweep(&bail, &[0.006], &circle(1.0, 6), true),
        Vec3::ZERO,
        Quat::IDENTITY,
        Vec3::ONE,
    );
    let kettle = lib.shape(m);
    lib.decor(kettle.at(p, rot, kind::METAL, rgba(COPPER, 1.0)));
}

/// A mantel clock with its foot's middle at `p`, facing `n` (toward the room): an
/// arched case on a plinth, a dial keeping time.
fn mantel_clock(lib: &mut Library, frame: &RoomFrame, p: Vec3, n: Vec3) {
    let q = frame.turn();
    let (w, shoulder, d) = (0.18f32, 0.17f32, 0.15f32);
    let mut outline = vec![Vec2::new(-w, 0.0)];
    outline.extend((0..=16).map(|k| {
        let a = k as f32 / 16.0 * PI;
        Vec2::new(w * libm::cosf(a), shoulder + w * libm::sinf(a))
    }));
    outline.insert(1, Vec2::new(w, 0.0));
    let case = lib.shape(Mesh::prism(&outline, d));
    lib.decor(case.at(p + Vec3::Y * 0.03, q, kind::WOOD, [0.22, 0.1, 0.04, 1.0]));
    lib.decor(Inst::new(
        p + Vec3::Y * 0.015,
        q,
        Vec3::new(2.0 * w + 0.05, 0.03, d + 0.04),
        kind::WOOD,
        rgba(WALNUT, 1.0),
        0,
    ));
    let c = p + Vec3::Y * (0.03 + shoulder) + n * (d * 0.5);
    let up = Quat::from_rotation_arc(Vec3::Y, n);
    let disc = lib.shape(Mesh::cylinder(32));
    lib.decor(
        disc.sized(Vec3::new(0.2, 0.01, 0.2))
            .inst(c + n * 0.005, up, kind::PAPER, [0.0; 4]),
    );
    let bezel = lib.shape(Mesh::torus(0.2, 32, 8));
    let brass = rgba(BRASS, 1.0);
    lib.decor(
        bezel
            .sized(Vec3::new(0.23, 0.02, 0.23))
            .inst(c + n * 0.01, up, kind::METAL, brass),
    );
    lib.clock_hands(
        (c + n * 0.01, n, 0.09),
        (11.0, 20.0),
        ([0.06, 0.06, 0.08, 1.0], brass),
        false,
    );
}

/// An armchair at `a`, `o` facing along `facing` (±1, across the room): a seat,
/// a high back, rolled arms and turned feet, upholstered in `rgb`.
fn armchair(lib: &mut Library, frame: &RoomFrame, a: f32, o: f32, facing: f32, rgb: [f32; 3]) {
    // Laid out with the chair facing +x (across), then turned.
    let q = frame.turn()
        * if facing > 0.0 {
            Quat::IDENTITY
        } else {
            Quat::from_rotation_y(PI)
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
    let roll = lib.shape(Mesh::cylinder(16));
    let foot = bun_foot(lib);
    let along = q * Quat::from_rotation_z(FRAC_PI_2);
    for s in [-1.0f32, 1.0] {
        put(
            lib,
            Vec3::new(0.03, 0.53, s * 0.38),
            Vec3::new(0.74, 0.16, 0.15),
            kind::CLOTH,
            rgb,
        );
        lib.decor(roll.sized(Vec3::new(0.17, 0.76, 0.17)).inst(
            c + q * Vec3::new(0.03, 0.62, s * 0.38),
            along,
            kind::CLOTH,
            rgba(rgb, 1.0),
        ));
        // The arm's turned face, a disc of wood on the roll's end.
        lib.decor(roll.sized(Vec3::new(0.15, 0.025, 0.15)).inst(
            c + q * Vec3::new(0.415, 0.62, s * 0.38),
            along,
            kind::WOOD,
            rgba(WALNUT, 1.0),
        ));
        for x in [-0.3f32, 0.3] {
            lib.decor(foot.sized(Vec3::new(0.09, 0.21, 0.09)).at(
                c + q * Vec3::new(x, 0.0, s * 0.34),
                q,
                kind::WOOD,
                rgba(DARK_WOOD, 1.0),
            ));
        }
    }
    guard(lib, frame, (a - 0.45, a + 0.45), (o - 0.47, o + 0.47), 1.35);
}

/// A ginger cat curled asleep in the chair at `a`, `o` (facing as `armchair`), once the
/// room is in order: it gathers out of a swirl of warm motes, and sleeps on there,
/// breathing.
fn sleeping_cat(lib: &mut Library, frame: &RoomFrame, a: f32, o: f32, facing: f32) {
    let q = frame.turn()
        * if facing > 0.0 {
            Quat::IDENTITY
        } else {
            Quat::from_rotation_y(PI)
        };
    let seat = frame.at(a, o, 0.0) + q * Vec3::new(0.06, 0.52, 0.0);
    // Its parts laid out in the chair's frame, a little over life size in a chair this
    // size, so it reads across the room.
    let on = |p: Vec3| seat + q * (p * 1.15);
    let fin = Finish::Room(frame.dept);
    let breathe = Motion::Bob {
        amp: 0.004,
        period: 3.6,
        rock: 0.0,
    };
    let m = lib.mover(seat, breathe, 0.0, None);
    let gather = Change::Arrives {
        from: Vec3::ZERO,
        grow: 0.0,
    };
    lib.awaiting(m, Awaits::new(fin, gather).after(1.5).over(2.5));
    let fur = rgba([0.6, 0.32, 0.12], 1.0);
    let ball = lib.shape(Mesh::sphere(20));
    // Curled round on itself, its head on its flank toward the chair's front.
    lib.moving(
        m,
        ball.sized(Vec3::new(0.36, 0.15, 0.31) * 1.15).inst(
            on(Vec3::new(0.0, 0.07, 0.0)),
            q * Quat::from_rotation_y(0.3),
            kind::CLOTH,
            fur,
        ),
    );
    let head = on(Vec3::new(0.12, 0.12, 0.09));
    let look = q * Quat::from_rotation_y(-0.5);
    lib.moving(
        m,
        ball.sized(Vec3::new(0.12, 0.1, 0.12) * 1.15)
            .inst(head, look, kind::CLOTH, fur),
    );
    lib.moving(
        m,
        ball.sized(Vec3::new(0.05, 0.04, 0.06) * 1.15).inst(
            head + look * Vec3::new(0.058, -0.023, 0.0),
            look,
            kind::CLOTH,
            [0.85, 0.75, 0.6, 1.0],
        ),
    );
    let ear = lib.shape(Mesh::cone(8));
    for s in [-1.0f32, 1.0] {
        lib.moving(
            m,
            ear.sized(Vec3::new(0.035, 0.05, 0.022) * 1.15).inst(
                head + look * (Vec3::new(-0.01, 0.055, s * 0.035) * 1.15),
                look * Quat::from_rotation_x(s * 0.3),
                kind::CLOTH,
                fur,
            ),
        );
    }
    // Its tail round in front of it.
    let tail: Vec<Vec3> = (0..=12)
        .map(|k| {
            let t = k as f32 / 12.0;
            let th = -2.4 + t * 2.7;
            on(Vec3::new(0.2 * libm::cosf(th), 0.05 + 0.02 * t, 0.18 * libm::sinf(th)))
        })
        .collect();
    let tail = lib.shape(Mesh::sweep(&tail, &[0.03, 0.018], &circle(1.0, 8), true));
    lib.moving(m, tail.at(Vec3::ZERO, Quat::IDENTITY, kind::CLOTH, fur));
    lib.sparks(Sparks {
        delay: 1.0,
        lasts: Some(2.8),
        spread: Vec3::splat(0.2),
        rate: 25.0,
        vel: Vec3::Y * 0.25,
        jitter: Vec3::splat(0.25),
        lift: 0.05,
        wander: 0.6,
        life: 2.0,
        rgba: [1.4, 0.95, 0.5, 1.8],
        ..Sparks::burst(fin, seat + Vec3::Y * 0.1)
    });
}

/// A sofa across the inglenook's mouth at `a`, `o`, facing the fire, on turned feet,
/// its arms rolled, cushions on it.
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
    let roll = lib.shape(Mesh::cylinder(16));
    let foot = bun_foot(lib);
    let along = Quat::from_rotation_arc(Vec3::Y, frame.out());
    for s in [-1.0f32, 1.0] {
        let x = a + s * (hw - 0.09);
        lib.decor(piece(
            frame,
            (x - 0.1, x + 0.1),
            (o - hd, o + hd),
            (0.42, 0.6),
            kind::CLOTH,
            UPHOLSTERY,
        ));
        lib.decor(roll.sized(Vec3::new(0.22, 2.0 * hd + 0.02, 0.22)).inst(
            frame.at(x, o + 0.01, 0.63),
            along,
            kind::CLOTH,
            rgba(UPHOLSTERY, 1.0),
        ));
        for y in [o - hd + 0.06, o + hd - 0.06] {
            lib.decor(foot.sized(Vec3::new(0.08, 0.1, 0.08)).at(
                frame.at(x, y, 0.0),
                Quat::IDENTITY,
                kind::WOOD,
                rgba(DARK_WOOD, 1.0),
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

/// A low round tea table at `a`, `o` on a turned pedestal: a teapot steaming, cups
/// on saucers, a candle.
fn tea_table(lib: &mut Library, frame: &RoomFrame, a: f32, o: f32) {
    let top = 0.5;
    let board = lib.turned(
        &[
            (0.0, 0.0),
            (0.29, 0.0),
            (0.31, 0.012),
            (0.32, 0.03),
            (0.3, 0.045),
            (0.0, 0.045),
        ],
        40,
    );
    lib.decor(board.at(
        frame.at(a, o, top - 0.045),
        Quat::IDENTITY,
        kind::WOOD,
        rgba(WALNUT, 1.0),
    ));
    let stem = turned_leg(lib);
    lib.decor(stem.sized(Vec3::new(0.12, top - 0.1, 0.12)).at(
        frame.at(a, o, 0.055),
        Quat::IDENTITY,
        kind::WOOD,
        rgba(DARK_WOOD, 1.0),
    ));
    let base = lib.turned(
        &[
            (0.0, 0.0),
            (0.2, 0.0),
            (0.2, 0.02),
            (0.16, 0.04),
            (0.07, 0.055),
            (0.0, 0.055),
        ],
        32,
    );
    lib.decor(base.at(frame.at(a, o, 0.0), Quat::IDENTITY, kind::WOOD, rgba(DARK_WOOD, 1.0)));
    guard(lib, frame, (a - 0.36, a + 0.36), (o - 0.36, o + 0.36), top);
    let turn = frame.turn() * Quat::from_rotation_y(0.6);
    let pot = frame.at(a - 0.08, o - 0.05, top);
    teapot(lib, pot, turn);
    let steam = |lib: &mut Library, p: Vec3, w: f32, h: f32| {
        lib.translucent(Inst::new(
            p + Vec3::Y * (h * 0.5),
            Quat::IDENTITY,
            Vec3::new(w, h, w),
            kind::MIST,
            [0.8, 0.8, 0.8, 0.25],
            0,
        ));
    };
    steam(lib, pot + turn * Vec3::new(0.15, 0.125, 0.0), 0.05, 0.2);
    let v = Vec2::new;
    let cup = {
        let mut m = Mesh::lathe(
            &[
                v(0.0, 0.0),
                v(0.022, 0.0),
                v(0.03, 0.01),
                v(0.038, 0.035),
                v(0.041, 0.05),
            ],
            16,
        )
        .two_sided();
        m.add(
            &Mesh::torus(0.35, 12, 6),
            Vec3::new(0.045, 0.028, 0.0),
            Quat::from_rotation_x(FRAC_PI_2),
            Vec3::splat(0.014),
        );
        lib.shape(m)
    };
    let saucer = lib.turned(
        &[
            (0.0, 0.0),
            (0.045, 0.0),
            (0.065, 0.012),
            (0.062, 0.016),
            (0.04, 0.008),
            (0.0, 0.008),
        ],
        24,
    );
    for (k, (da, d_o)) in [(0.17f32, 0.12f32), (0.1, -0.2)].into_iter().enumerate() {
        let p = frame.at(a + da, o + d_o, top);
        let china = rgba(CHINA, 1.0);
        lib.decor(saucer.at(p, Quat::IDENTITY, kind::PLAIN, china));
        let spin = frame.turn() * Quat::from_rotation_y(1.2 + 2.1 * k as f32);
        lib.decor(cup.at(p + Vec3::Y * 0.008, spin, kind::PLAIN, china));
        if k == 0 {
            steam(lib, p + Vec3::Y * 0.06, 0.04, 0.12);
        }
    }
    let holder = frame.at(a - 0.18, o + 0.17, top);
    lib.decor(
        saucer
            .sized(saucer.size * 0.8)
            .at(holder, Quat::IDENTITY, kind::METAL, rgba(BRASS, 1.0)),
    );
    candle(lib, holder + Vec3::Y * 0.006, 0.12);
}

/// A china teapot with its foot at `p`, turned by `rot` (its spout along x): a round
/// body, a lid and knob, a spout and a handle.
fn teapot(lib: &mut Library, p: Vec3, rot: Quat) {
    let mut m = Mesh::lathe(
        &[
            (0.0, 0.0),
            (0.05, 0.0),
            (0.055, 0.006),
            (0.075, 0.025),
            (0.085, 0.055),
            (0.078, 0.088),
            (0.055, 0.11),
            (0.04, 0.115),
            (0.042, 0.12),
            (0.02, 0.125),
            (0.012, 0.14),
            (0.0, 0.145),
        ],
        24,
    );
    let spout: Vec<Vec3> = (0..=6)
        .map(|k| {
            let t = k as f32 / 6.0;
            Vec3::new(0.06 + 0.09 * t, 0.03 + 0.06 * t + 0.035 * t * t, 0.0)
        })
        .collect();
    m.add(
        &Mesh::sweep(&spout, &[0.016, 0.008], &circle(1.0, 8), true),
        Vec3::ZERO,
        Quat::IDENTITY,
        Vec3::ONE,
    );
    let handle: Vec<Vec3> = (0..=10)
        .map(|k| {
            let a = k as f32 / 10.0 * PI;
            Vec3::new(-0.072 - 0.042 * libm::sinf(a), 0.058 + 0.035 * libm::cosf(a), 0.0)
        })
        .collect();
    m.add(
        &Mesh::sweep(&handle, &[0.007], &circle(1.0, 6), true),
        Vec3::ZERO,
        Quat::IDENTITY,
        Vec3::ONE,
    );
    let pot = lib.shape(m);
    lib.decor(pot.at(p, rot, kind::PLAIN, rgba(CHINA, 1.0)));
}

/// A small side table at `a`, `o` on turned legs, a hurricane lamp on it.
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
    let leg = turned_leg(lib);
    for (da, d_o) in [(-0.2, -0.2), (0.2, -0.2), (-0.2, 0.2), (0.2, 0.2)] {
        lib.decor(leg.sized(Vec3::new(0.06, top - 0.04, 0.06)).at(
            frame.at(a + da, o + d_o, 0.0),
            Quat::IDENTITY,
            kind::WOOD,
            rgba(DARK_WOOD, 1.0),
        ));
    }
    guard(lib, frame, (a - 0.26, a + 0.26), (o - 0.26, o + 0.26), top);
    let base = frame.at(a, o, top);
    hurricane_lamp(lib, base);
    lib.light(base + Vec3::Y * 0.3, 4.5, [1.0, 0.62, 0.3], 0.7, LightKind::Flicker);
}

/// A basket of wool by a chair at `a`, `o`, and over it a pair of needles knitting
/// a scarf by themselves, floating: one of the room's two bits of magic.
fn knitting(lib: &mut Library, frame: &RoomFrame, a: f32, o: f32) {
    let basket = {
        let mut m = Mesh::lathe(&[(0.0, 0.0), (0.16, 0.0), (0.19, 0.06), (0.205, 0.2), (0.21, 0.26)], 24).two_sided();
        m.add(
            &Mesh::torus(0.1, 24, 6),
            Vec3::new(0.0, 0.26, 0.0),
            Quat::IDENTITY,
            Vec3::splat(0.21),
        );
        lib.shape(m)
    };
    lib.decor(basket.at(frame.at(a, o, 0.0), Quat::IDENTITY, kind::WOOD, [0.45, 0.32, 0.15, 1.0]));
    guard(lib, frame, (a - 0.21, a + 0.21), (o - 0.21, o + 0.21), 0.3);
    let ball = lib.shape(Mesh::sphere(16));
    for (k, col) in [[0.55, 0.15, 0.12], [0.2, 0.3, 0.5], [0.75, 0.65, 0.3]]
        .into_iter()
        .enumerate()
    {
        let p = frame.at(a - 0.08 + 0.09 * k as f32, o - 0.04 + 0.05 * (k % 2) as f32, 0.23);
        lib.decor(
            ball.sized(Vec3::splat(0.13))
                .inst(p, Quat::from_rotation_y(k as f32), kind::CLOTH, rgba(col, 1.0)),
        );
    }
    let q = frame.turn();
    let hold = frame.at(a, o, 1.25);
    // The work floats, rising and falling a little; the needles knit on it, crossing
    // and parting, the scarf hanging from them.
    let float = Motion::Bob {
        amp: 0.015,
        period: 4.5,
        rock: 0.04,
    };
    let held = lib.mover(hold, float, 0.0, None);
    lib.moving(
        held,
        Inst::new(
            hold - Vec3::Y * 0.28,
            q * Quat::from_rotation_z(0.06),
            Vec3::new(0.18, 0.42, 0.02),
            kind::CLOTH,
            [0.55, 0.15, 0.12, 1.0],
            0,
        ),
    );
    let needle = lib.turned(
        &[
            (0.0, 0.0),
            (0.12, 0.05),
            (0.12, 0.93),
            (0.3, 0.95),
            (0.32, 0.99),
            (0.0, 1.0),
        ],
        8,
    );
    for s in [-1.0f32, 1.0] {
        let tip = hold + q * Vec3::new(s * 0.1, 0.1, 0.0);
        let end = hold + q * Vec3::new(-s * 0.12, -0.08, 0.03 * s);
        let knit = Motion::Swing {
            axis: frame.out(),
            amp: 0.12,
            period: 1.6,
        };
        let m = lib.mover(end, knit, if s < 0.0 { 0.0 } else { 0.8 }, Some(held));
        let len = (end - tip).length();
        lib.moving(
            m,
            needle.sized(Vec3::new(0.02, len, 0.02)).at(
                tip,
                Quat::from_rotation_arc(Vec3::Y, (end - tip) / len),
                kind::CRYSTAL,
                [0.85, 0.75, 1.0, 1.2],
            ),
        );
    }
    // The yarn, from the scarf down to the basket.
    lib.pole(
        hold - Vec3::Y * 0.5,
        frame.at(a - 0.08, o - 0.04, 0.3),
        0.006,
        kind::CLOTH,
        [0.55, 0.15, 0.12, 1.0],
    );
    lib.light(hold, 2.0, [0.8, 0.7, 1.0], 0.35, LightKind::Static);
}

/// A spinning wheel at `a`, `o`, its wheel turning slowly across the room between
/// two uprights.
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
        lib.pole(
            frame.at(a + da, o + d_o, 0.0),
            frame.at(a + da * 0.6, o + d_o * 0.8, 0.3),
            0.04,
            kind::WOOD,
            wood,
        );
    }
    guard(lib, frame, (a - 0.2, a + 0.2), (o - 0.55, o + 0.55), 1.2);
    let (o_hub, y_hub) = (o - 0.15, 0.78);
    let hub = frame.at(a, o_hub, y_hub);
    let axis = frame.across();
    // The wheel: a rim, eight spokes and a hub, made about y, stood on edge.
    let mut m = Mesh::torus(0.1, 48, 8);
    for k in 0..8 {
        let th = k as f32 / 8.0 * TAU;
        let dir = Vec3::new(libm::cosf(th), 0.0, libm::sinf(th));
        m.add(
            &Mesh::cylinder(6),
            dir * 0.5,
            Quat::from_rotation_arc(Vec3::Y, dir),
            Vec3::new(0.04, 0.96, 0.04),
        );
    }
    m.add(
        &Mesh::cylinder(12),
        Vec3::ZERO,
        Quat::IDENTITY,
        Vec3::new(0.22, 0.3, 0.22),
    );
    let wheel = lib.shape(m);
    let turning = lib.mover(hub, Motion::Spin { axis, rate: 0.5 }, 0.0, None);
    lib.moving(
        turning,
        wheel
            .scaled(0.36)
            .at(hub, Quat::from_rotation_arc(Vec3::Y, axis), kind::WOOD, wood),
    );
    // Its axle between two uprights.
    for s in [-1.0f32, 1.0] {
        lib.pole(
            frame.at(a + s * 0.07, o_hub, 0.3),
            frame.at(a + s * 0.07, o_hub, y_hub + 0.04),
            0.04,
            kind::WOOD,
            wood,
        );
    }
    lib.pole(
        frame.at(a - 0.09, o_hub, y_hub),
        frame.at(a + 0.09, o_hub, y_hub),
        0.02,
        kind::METAL,
        rgba(IRON, 1.0),
    );
    // The distaff, flax wound on it.
    lib.pole(
        frame.at(a, o + 0.35, 0.33),
        frame.at(a, o + 0.4, 0.95),
        0.035,
        kind::WOOD,
        wood,
    );
    let flax = lib.turned(&[(0.0, 0.0), (0.3, 0.05), (0.5, 0.4), (0.35, 0.8), (0.0, 1.0)], 12);
    lib.decor(flax.sized(Vec3::new(0.1, 0.2, 0.1)).at(
        frame.at(a, o + 0.39, 0.82),
        Quat::IDENTITY,
        kind::CLOTH,
        [0.85, 0.8, 0.7, 1.0],
    ));
}

/// A barrel of ale by the door, iron-hooped, a tap in it.
fn barrel(lib: &mut Library, frame: &RoomFrame, a: f32, o: f32) {
    let r = 0.33;
    let body = lib.turned(
        &[
            (0.0, 0.0),
            (0.29, 0.0),
            (0.305, 0.08),
            (0.325, 0.25),
            (0.335, 0.45),
            (0.325, 0.65),
            (0.305, 0.82),
            (0.29, 0.9),
            (0.0, 0.9),
        ],
        32,
    );
    lib.decor(body.at(frame.at(a, o, 0.0), Quat::IDENTITY, kind::WOOD, [0.38, 0.22, 0.1, 1.0]));
    let hoop = lib.shape(Mesh::torus(0.08, 32, 6));
    for (y, rr) in [(0.1f32, 0.307f32), (0.28, 0.327), (0.62, 0.327), (0.8, 0.307)] {
        lib.decor(hoop.sized(Vec3::new(2.0 * rr + 0.02, 0.035, 2.0 * rr + 0.02)).inst(
            frame.at(a, o, y),
            Quat::IDENTITY,
            kind::METAL,
            rgba(IRON, 1.0),
        ));
    }
    let tap = (frame.at(a, o + 0.3, 0.25), frame.at(a, o + 0.43, 0.25));
    lib.pole(tap.0, tap.1, 0.035, kind::METAL, rgba(BRASS, 1.0));
    lib.decor(Inst::aabox(
        tap.1 - frame.out() * 0.03 + Vec3::Y * 0.035,
        Vec3::new(0.025, 0.05, 0.025),
        kind::METAL,
        BRASS,
    ));
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

/// Candles floating over the hearth and the sofa, unsupported, each bobbing in its
/// own time, a few of them lighting the air round them: the room's chief bit of
/// magic.
fn floating_candles(lib: &mut Library, frame: &RoomFrame) {
    let mut rng = StdRng::seed_from_u64(0x4341_4e44);
    for k in 0..14 {
        let a = rng.gen_range(-2.4..2.4);
        let o = rng.gen_range(22.0..29.5);
        let y = rng.gen_range(3.1..4.3);
        let h = rng.gen_range(0.14..0.3);
        let float = Motion::Bob {
            amp: rng.gen_range(0.03..0.06),
            period: rng.gen_range(5.0..9.0),
            rock: 0.06,
        };
        let p = frame.at(a, o, y);
        let m = lib.mover(p + Vec3::Y * (h * 0.5), float, rng.gen_range(0.0..9.0), None);
        for i in candle_parts(lib, p, h) {
            lib.moving(m, i);
        }
        if k % 5 == 0 {
            let at = p + Vec3::Y * (h + 0.05);
            lib.light_riding(m, at, 4.5, [1.0, 0.7, 0.4], 0.35, LightKind::Flicker);
        }
    }
}

/// Heavy beams across the ceiling, bunches of herbs hung drying from two of them,
/// stirring in the warm air.
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
    // A bunch's sprays: narrow at the tie, spreading below it.
    let spray = lib.shape(Mesh::prism(
        &[
            Vec2::new(-0.03, 0.0),
            Vec2::new(0.03, 0.0),
            Vec2::new(0.09, -0.75),
            Vec2::new(0.07, -1.0),
            Vec2::new(-0.07, -1.0),
            Vec2::new(-0.09, -0.75),
        ],
        0.01,
    ));
    let mut rng = StdRng::seed_from_u64(0x4845_5242);
    for (j, o) in [beams[1], beams[2]].into_iter().enumerate() {
        for k in 0..7 {
            let a = -5.2 + k as f32 * 1.7 + rng.gen_range(-0.3..0.3);
            let len = rng.gen_range(0.3..0.55);
            let top = h - 0.36;
            lib.pole(
                frame.at(a, o, top),
                frame.at(a, o, top - 0.15),
                0.01,
                kind::CLOTH,
                [0.5, 0.42, 0.3, 1.0],
            );
            let green = [
                rng.gen_range(0.18..0.3),
                rng.gen_range(0.3..0.42),
                rng.gen_range(0.12..0.2),
            ];
            let spin = rng.gen_range(0.0..3.0f32);
            let tie = frame.at(a, o, top - 0.15);
            // Each in its own time, from where it hangs.
            let f = ((j * 7 + k) as f32 * 0.618_034).fract();
            let stir = Motion::Swing {
                axis: Quat::from_rotation_y(f * TAU) * Vec3::X,
                amp: 0.035,
                period: 5.0 + 2.5 * f,
            };
            let m = lib.mover(tie, stir, f * 7.0, None);
            for k in 0..2 {
                let q = Quat::from_rotation_y(spin + k as f32 * FRAC_PI_2);
                let i = spray.sized(Vec3::new(0.18, len + 0.04, 0.01)).at(
                    tie - Vec3::Y * 0.02,
                    q,
                    kind::FRET,
                    rgba(green, 1.0),
                );
                lib.moving(m, i);
            }
            lib.moving(
                m,
                Inst::new(
                    tie,
                    Quat::from_rotation_y(spin),
                    Vec3::new(0.05, 0.03, 0.05),
                    kind::CLOTH,
                    [0.5, 0.42, 0.3, 1.0],
                    0,
                ),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// No case side's candle stands in a category's sign, wherever the sign hangs: over
    /// a case, or between two where a category stands in parts apart.
    #[test]
    fn case_candles_clear_the_signs() {
        let lib = Library::generate(Layout::Great, crate::game::SEED);
        let hearth = |c: usize| lib.dept_of(c) == dept::HEARTH_LIBRARY;
        let wicks: Vec<Vec3> = (lib.movers.iter())
            .filter(|m| match m.awaits {
                Some(Awaits {
                    finish: Finish::Face(f),
                    ..
                }) => hearth(lib.faces[f].category),
                _ => false,
            })
            .map(|m| m.pivot)
            .collect();
        assert_eq!(wicks.len(), 40, "a candle on every side");
        for l in &lib.labels {
            let LabelKind::Category(c) = l.kind else { continue };
            if !hearth(c) {
                continue;
            }
            for &w in &wicks {
                let d = w - l.pos;
                let along = d.dot(Vec3::Y.cross(l.normal)).abs();
                assert!(
                    along > 1.06 || d.dot(l.normal).abs() > 0.2,
                    "a candle at {w} in {}'s sign",
                    l.text
                );
            }
        }
    }
}
