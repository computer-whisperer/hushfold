//! The Clockworks: department 8, top floor, north side. Bays of cases stand along
//! both side walls like the teeth of a wheel; two pillars of cases on the floor show
//! their works through open fronts under clock turrets; the great clock fills the
//! middle of the far wall with its pendulum below; a workbench and an automaton
//! stand by the door; gears turn (still) on the walls and under the ceiling.

use super::*;

const BRASS_LIT: [f32; 3] = [0.72, 0.5, 0.2];
const BRASS_OLD: [f32; 3] = [0.42, 0.28, 0.11];
const STEEL: [f32; 3] = [0.28, 0.31, 0.36];
const CASEWORK: [f32; 3] = [0.12, 0.06, 0.03];
/// The glow of the room's magic: the great clock's hours, the wheel that turns itself.
const GLOW: [f32; 3] = [0.45, 0.72, 1.0];
/// A wall case's depth from the wall to its face.
const WALL_D: f32 = 0.04 + FACE_DEPTH;
/// Where across the room (from the middle) the cases of a bay's arms stand: against
/// the side wall, and next to it.
const ARMS: [f32; 2] = [
    RoomFrame::A_HALF - UNIT_PITCH * 0.5,
    RoomFrame::A_HALF - UNIT_PITCH * 1.5,
];
/// The pillars: how far either side of the middle, where they start (toward the
/// door), how long and wide their cores are, and how deep their clock turrets.
const PILLAR_A: f32 = 3.7;
const PILLAR_O: f32 = 20.0;
const PILLAR_LEN: f32 = 2.0 * UNIT_PITCH;
const PILLAR_W: f32 = UNIT_PITCH;
const TURRET: f32 = 1.7;
/// The cases' height to the top of their caps.
const CASE_TOP: f32 = UNIT_H + 0.13;

pub(super) fn furnish(lib: &mut Library, frame: &RoomFrame, rng: &mut StdRng, titles: &mut HashSet<String>) {
    // The bays and the workshop are laid out round a door in the middle of the wall.
    debug_assert!(frame.door.abs() < 1.0, "the Clockworks wants its door in the middle");
    let mut cats = frame.categories.clone();
    bays(lib, frame, rng, titles, &mut cats);
    for s in [-1.0f32, 1.0] {
        pillar(lib, frame, rng, titles, cats.next().unwrap(), s * PILLAR_A);
    }
    near_wall(lib, frame);
    great_clock(lib, frame);
    workshop(lib, frame);
    ceiling(lib, frame);
}

/// A bay's faces in reading order, as someone standing in it sees them: along the
/// arm on their left, across the back, along the arm on their right.
fn bay_order(back: (Vec3, Vec3), a: Vec<(Vec3, Vec3)>, b: Vec<(Vec3, Vec3)>) -> Vec<(Vec3, Vec3)> {
    let right = Vec3::Y.cross(back.1);
    let (left, right_arm) = if (a[0].0 - back.0).dot(right) < 0.0 {
        (a, b)
    } else {
        (b, a)
    };
    let mut out = left_to_right(left);
    out.push(back);
    out.extend(left_to_right(right_arm));
    out
}

/// The rotation laying a part out on a plane facing `n`: x along `radial` (in the
/// plane), z along `n`.
fn on_plane(radial: Vec3, n: Vec3) -> Quat {
    Quat::from_mat3(&Mat3::from_cols(radial, n.cross(radial), n))
}

/// Three bays along each side wall, each a category: two short double-sided cases
/// stand out from the wall between them, and cases on the near and far walls close
/// the first and last; a case stands at the back of each.
fn bays(
    lib: &mut Library,
    frame: &RoomFrame,
    rng: &mut StdRng,
    titles: &mut HashSet<String>,
    cats: &mut std::ops::Range<usize>,
) {
    let (out, across) = (frame.out(), frame.across());
    let near = RoomFrame::O_NEAR + WALL_D;
    let far = RoomFrame::O_FAR - WALL_D;
    let bay = (far - near - 2.0 * UNIT_DEPTH) / 3.0;
    let stubs = [near + bay + UNIT_DEPTH * 0.5, far - bay - UNIT_DEPTH * 0.5];
    for s in [-1.0f32, 1.0] {
        let near_arm: Vec<_> = (ARMS.iter())
            .map(|&a| lib.wall_bookcase(frame.at(s * a, RoomFrame::O_NEAR, 0.0), out))
            .collect();
        let far_arm: Vec<_> = (ARMS.iter())
            .map(|&a| lib.wall_bookcase(frame.at(s * a, RoomFrame::O_FAR, 0.0), -out))
            .collect();
        // Each stub's faces toward the door and away from it.
        let mut toward = Vec::new();
        let mut away = Vec::new();
        for &o in &stubs {
            let (mut t, mut w) = (Vec::new(), Vec::new());
            for &a in &ARMS {
                for f in lib.bookcase(frame.at(s * a, o, 0.0), !frame.out_along_x()) {
                    if f.1.dot(out) > 0.0 { w.push(f) } else { t.push(f) }
                }
            }
            toward.push(t);
            away.push(w);
        }
        let [t0, t1] = [toward[0].clone(), toward[1].clone()];
        let [w0, w1] = [away[0].clone(), away[1].clone()];
        let mut lo = near;
        for (k, (a, b)) in [(near_arm, t0), (w0, t1), (w1, far_arm)].into_iter().enumerate() {
            let hi = if k < 2 { stubs[k] - UNIT_DEPTH * 0.5 } else { far };
            let back = lib.wall_bookcase(frame.at(s * RoomFrame::A_HALF, (lo + hi) * 0.5, 0.0), -across * s);
            lib.add_category(rng, cats.next().unwrap(), &bay_order(back, a, b), titles);
            bay_dressing(lib, frame, s, lo, hi);
            lo = hi + UNIT_DEPTH;
        }
    }
}

/// Panelling on the side wall either side of a bay's back case.
fn bay_dressing(lib: &mut Library, frame: &RoomFrame, s: f32, lo: f32, hi: f32) {
    let wall = s * RoomFrame::A_HALF;
    let mid = (lo + hi) * 0.5;
    let half = UNIT_W * 0.5 + 0.06;
    let (a0, a1) = (wall - s * 0.07, wall);
    for (o0, o1) in [(lo + 0.02, mid - half), (mid + half, hi - 0.02)] {
        lib.wainscot(
            frame.y,
            frame.rect(a0.min(a1), a0.max(a1), o0, o1),
            frame.side.across * -s,
        );
    }
}

/// A pillar of cases on the floor at `a`: two wall cases down each side of a core
/// and one across its far end, a category round it. Its near end shows a pendulum
/// and gears in an open frame under a clock turret; gears stand on its top.
fn pillar(lib: &mut Library, frame: &RoomFrame, rng: &mut StdRng, titles: &mut HashSet<String>, cat: usize, a: f32) {
    let (out, across) = (frame.out(), frame.across());
    let hw = PILLAR_W * 0.5;
    let (o0, o1) = (PILLAR_O, PILLAR_O + PILLAR_LEN);
    let h = RoomFrame::HEIGHT;
    lib.solid(
        piece(
            frame,
            (a - hw, a + hw),
            (o0, o1),
            (0.0, CASE_TOP - 0.01),
            kind::WOOD,
            CASEWORK,
        ),
        Stops::ALL,
    );
    // No aiming over it at the far side's shelves, as over a double-sided case.
    let (lo, hi) = frame.boxed(a - hw, a + hw, o0, o1, CASE_TOP, CASE_TOP + 1.07);
    lib.collide(lo, hi, Stops::SIGHT);
    let mut plus = Vec::new();
    let mut minus = Vec::new();
    for k in 0..2 {
        let o = o0 + UNIT_PITCH * (k as f32 + 0.5);
        plus.push(lib.wall_bookcase(frame.at(a + hw, o, 0.0), across));
        minus.push(lib.wall_bookcase(frame.at(a - hw, o, 0.0), -across));
    }
    let end = lib.wall_bookcase(frame.at(a, o1, 0.0), out);
    // Round its far end: along one side, the end, back along the other.
    let mut faces = left_to_right(plus);
    faces.push(end);
    faces.extend(left_to_right(minus));
    lib.add_category(rng, cat, &faces, titles);
    // The corners at the far end, where side cases and end case leave a notch.
    for s in [-1.0f32, 1.0] {
        let (x0, x1) = (a + s * (hw + 0.005), a + s * (hw + WALL_D - 0.01));
        let corner = piece(
            frame,
            (x0.min(x1), x0.max(x1)),
            (o1 + 0.005, o1 + WALL_D - 0.01),
            (0.0, UNIT_H),
            kind::WOOD,
            WALNUT,
        );
        lib.decor(corner);
    }
    // The clock turret over the near end, up to the ceiling, with a dial on its face.
    let t = (a - hw + 0.3, a + hw - 0.3);
    lib.decor(piece(
        frame,
        t,
        (o0, o0 + TURRET),
        (CASE_TOP - 0.01, h - 0.01),
        kind::WOOD,
        CASEWORK,
    ));
    lib.decor(piece(
        frame,
        (t.0 - 0.06, t.1 + 0.06),
        (o0 - 0.06, o0 + TURRET + 0.06),
        (h - 0.24, h - 0.01),
        kind::WOOD,
        WALNUT,
    ));
    lib.decor(piece(
        frame,
        (t.0 - 0.05, t.1 + 0.05),
        (o0 - 0.05, o0 + TURRET + 0.05),
        (CASE_TOP - 0.01, CASE_TOP + 0.12),
        kind::WOOD,
        WALNUT,
    ));
    let n = -out;
    dial(
        lib,
        frame.at(a, o0 - 0.02, 4.25),
        n,
        0.68,
        (a > 0.0) as u32 * 5 + 2,
        0.0,
    );
    // The movement standing on the pillar's top, behind the turret.
    let top = CASE_TOP + 0.12;
    let mid = o0 + TURRET + (PILLAR_LEN - TURRET) * 0.5;
    lib.decor(piece(
        frame,
        (a - 0.5, a + 0.5),
        (o0 + TURRET + 0.1, o1 - 0.2),
        (CASE_TOP - 0.01, top),
        kind::WOOD,
        WALNUT,
    ));
    gear(
        lib,
        frame.at(a - 0.12, mid - 0.45, top + 0.95),
        across,
        wheel(0.85, 16, 0.07).old().turned(a),
    );
    gear(
        lib,
        frame.at(a + 0.12, mid + 0.95, top + 0.62),
        across,
        wheel(0.52, 10, 0.06).turned(a * 2.0),
    );
    for (d_o, y0) in [(-0.45, 0.95), (0.95, 0.62)] {
        lib.decor(piece(
            frame,
            (a - 0.03, a + 0.03),
            (mid + d_o - 0.03, mid + d_o + 0.03),
            (top, top + y0),
            kind::METAL,
            STEEL,
        ));
    }
    // The open front: a plinth and posts round the works, a pendulum hanging in it.
    let face = o0 - 0.01;
    lib.decor(piece(
        frame,
        (a - 1.05, a + 1.05),
        (o0 - 0.32, face),
        (0.0, 0.42),
        kind::WOOD,
        WALNUT,
    ));
    for s in [-1.0f32, 1.0] {
        let x = a + s * 0.95;
        lib.decor(piece(
            frame,
            (x - 0.1, x + 0.1),
            (o0 - 0.28, face),
            (0.42, 3.05),
            kind::WOOD,
            WALNUT,
        ));
    }
    lib.decor(piece(
        frame,
        (a - 1.08, a + 1.08),
        (o0 - 0.34, face),
        (3.05, 3.25),
        kind::WOOD,
        WALNUT,
    ));
    guard(lib, frame, (a - 1.08, a + 1.08), (o0 - 0.34, o0), 3.25);
    gear(
        lib,
        frame.at(a - 0.35, face - 0.03, 2.35),
        n,
        wheel(0.42, 12, 0.04).turned(0.2),
    );
    gear(
        lib,
        frame.at(a + 0.38, face - 0.03, 1.75),
        n,
        wheel(0.3, 8, 0.04).old().turned(0.5),
    );
    pendulum(lib, frame.at(a, face - 0.16, 2.9), n, 1.85, 0.2, 0.05);
}

/// A pendulum hanging from `top`, facing `n`: a rod `len` long and a brass bob of
/// radius `r`, `t` thick.
fn pendulum(lib: &mut Library, top: Vec3, n: Vec3, len: f32, r: f32, t: f32) {
    lib.decor(Inst::aabox(
        top - Vec3::Y * (len * 0.5),
        Vec3::new(0.03, len, 0.03),
        kind::METAL,
        STEEL,
    ));
    let side = Vec3::Y.cross(n);
    // A round bob from four slabs crossed in its plane.
    let slabs = if r < 0.15 { 2 } else { 4 };
    for k in 0..slabs {
        let th = 0.3 + k as f32 * TAU / (2 * slabs) as f32;
        lib.decor(Inst::new(
            top - Vec3::Y * len,
            on_plane(side * th.cos() + Vec3::Y * th.sin(), n),
            Vec3::new(r * 2.0, r * 0.85, t + k as f32 * 0.006),
            kind::METAL,
            rgba(BRASS_LIT, 1.0),
            0,
        ));
    }
}

/// A gear's make: radius to the roots of its teeth, how many teeth, thickness, its
/// colour and material, and how far it is turned (radians).
#[derive(Clone, Copy)]
struct Wheel {
    r: f32,
    teeth: usize,
    t: f32,
    col: [f32; 4],
    kind: u32,
    phase: f32,
}

/// A polished brass wheel.
fn wheel(r: f32, teeth: usize, t: f32) -> Wheel {
    Wheel {
        r,
        teeth,
        t,
        col: rgba(BRASS_LIT, 1.0),
        kind: kind::METAL,
        phase: 0.0,
    }
}

impl Wheel {
    /// Dull with age.
    fn old(self) -> Self {
        Wheel {
            col: rgba(BRASS_OLD, 1.0),
            ..self
        }
    }

    fn turned(self, phase: f32) -> Self {
        Wheel { phase, ..self }
    }

    /// Glowing from inside, `w` brightly.
    fn glowing(self, w: f32) -> Self {
        Wheel {
            col: rgba(GLOW, w),
            kind: kind::CRYSTAL,
            ..self
        }
    }
}

/// A gear centred on `c` turning about `axis`: a toothed rim, spokes and a hub.
fn gear(lib: &mut Library, c: Vec3, axis: Vec3, g: Wheel) {
    let up = if axis.y.abs() > 0.9 { Vec3::X } else { Vec3::Y };
    let u = (up - axis * up.dot(axis)).normalize();
    let v = axis.cross(u);
    let (r, t) = (g.r, g.t);
    let rim = (r * 0.16).clamp(0.025, 0.12);
    let h = (r * 0.13).clamp(0.025, 0.09);
    let teeth = g.teeth.max(6);
    let step = TAU / teeth as f32;
    // The rim in half as many lengths as there are teeth.
    let segs = (teeth / 2).max(6);
    let seg = TAU / segs as f32;
    let dir = |th: f32| u * th.cos() + v * th.sin();
    let mut put = |pos: Vec3, radial: Vec3, size: Vec3, col: [f32; 4]| {
        lib.decor(Inst::new(pos, on_plane(radial, axis), size, g.kind, col, 0));
    };
    // Never square to the axes, so no face lies flat on another's.
    let phase = g.phase + 0.071;
    let mid = r - rim * 0.5;
    for k in 0..segs {
        let th = phase + k as f32 * seg;
        put(
            c + dir(th) * mid,
            dir(th),
            Vec3::new(rim, 2.0 * mid * (seg * 0.5).tan() + 0.01, t),
            g.col,
        );
    }
    for k in 0..teeth {
        let tooth = dir(phase + (k as f32 + 0.5) * step);
        put(
            c + tooth * (r + h * 0.5 - 0.005),
            tooth,
            Vec3::new(h + 0.01, r * step * 0.5, t * 0.9),
            g.col,
        );
    }
    let spokes = if r > 0.4 { 3 } else { 2 };
    for k in 0..spokes {
        let radial = dir(phase + 0.3 + k as f32 * TAU / (2 * spokes) as f32);
        put(
            c,
            radial,
            Vec3::new(2.0 * (r - rim), (r * 0.1).max(0.025), t * 0.7),
            g.col,
        );
    }
    put(
        c,
        dir(phase + 0.785),
        Vec3::new(r * 0.24, r * 0.24, t * 1.8),
        rgba(STEEL, 1.0),
    );
}

/// A clock dial at `c` facing `n` (level): a cream face, a brass bezel, hour marks
/// and hands at `hour`:`minute`; `r` its radius.
fn dial(lib: &mut Library, c: Vec3, n: Vec3, r: f32, hour: u32, minute: f32) {
    let up = Vec3::Y;
    let side = up.cross(n);
    let segs = if r > 1.0 {
        24
    } else if r > 0.5 {
        12
    } else {
        8
    };
    let step = TAU / segs as f32;
    let dir = |th: f32| side * th.cos() + up * th.sin();
    // The face: wedges from the middle, alternately a hair forward so none fight.
    for k in 0..segs {
        let radial = dir(0.05 + k as f32 * step);
        let off = if k % 2 == 0 { 0.0 } else { 0.002 };
        lib.decor(Inst::new(
            c + radial * (r * 0.5) + n * off,
            on_plane(radial, n),
            Vec3::new(r, 2.0 * r * (step * 0.5).sin() * 1.08, 0.02),
            kind::PAPER,
            [0.0; 4],
            0,
        ));
    }
    let rim = (r * 0.09).max(0.04);
    for k in 0..segs {
        let radial = dir(0.05 + (k as f32 + 0.5) * step);
        lib.decor(Inst::new(
            c + radial * (r + rim * 0.5) + n * 0.01,
            on_plane(radial, n),
            Vec3::new(rim, (r + rim * 0.5) * step * 1.1, 0.06),
            kind::METAL,
            rgba(BRASS_LIT, 1.0),
            0,
        ));
    }
    // Hour marks, long at the quarters.
    // Small dials only at the quarters.
    let every = if r > 0.5 { 1 } else { 3 };
    for h in (0..12).step_by(every) {
        let radial = dir(std::f32::consts::FRAC_PI_2 - h as f32 * TAU / 12.0);
        let len = if h % 3 == 0 { r * 0.2 } else { r * 0.11 };
        lib.decor(Inst::new(
            c + radial * (r * 0.9 - len * 0.5) + n * 0.016,
            on_plane(radial, n),
            Vec3::new(len, r * 0.035 + 0.008, 0.01),
            kind::PLAIN,
            [0.05, 0.04, 0.035, 1.0],
            0,
        ));
    }
    // Hands: the hour hand short and broad, the minute hand long.
    let hand = |lib: &mut Library, turns: f32, len: f32, w: f32, z: f32| {
        let radial = dir(std::f32::consts::FRAC_PI_2 - turns * TAU);
        lib.decor(Inst::new(
            c + radial * (len * 0.5 - r * 0.08) + n * z,
            on_plane(radial, n),
            Vec3::new(len + r * 0.16, w, 0.012),
            kind::METAL,
            [0.06, 0.06, 0.08, 1.0],
            0,
        ));
    };
    hand(
        lib,
        (hour % 12) as f32 / 12.0 + minute / 720.0,
        r * 0.52,
        r * 0.07,
        0.024,
    );
    hand(lib, minute / 60.0, r * 0.78, r * 0.045, 0.034);
    lib.decor(Inst::new(
        c + n * 0.04,
        on_plane(dir(0.785), n),
        Vec3::new(r * 0.1, r * 0.1, 0.03),
        kind::METAL,
        rgba(BRASS_LIT, 1.0),
        0,
    ));
}

/// A small wall clock (a dial on a short case with a pendulum under it), its dial's
/// middle at `c`, facing `n`.
fn wall_clock(lib: &mut Library, c: Vec3, n: Vec3, r: f32, hour: u32) {
    let side = Vec3::Y.cross(n);
    let w = r * 2.3;
    let size = |sa: f32, sy: f32, sn: f32| (side * sa + Vec3::Y * sy + n * sn).abs();
    let case = Inst::aabox(
        c - n * 0.03 - Vec3::Y * (r * 0.6),
        size(w, w + r * 1.4, 0.1),
        kind::WOOD,
        CASEWORK,
    );
    lib.decor(case);
    pendulum(lib, c - Vec3::Y * (r * 1.25) + n * 0.03, n, r * 1.1, r * 0.22, 0.02);
    dial(lib, c + n * 0.03, n, r, hour, (hour * 17 % 60) as f32);
}

/// The far wall's great clock: a dial over three metres across, its hours glowing,
/// a pendulum in an open case under it, and moonlit windows either side with gears
/// over them.
fn great_clock(lib: &mut Library, frame: &RoomFrame) {
    let out = frame.out();
    let n = -out;
    let wall = RoomFrame::O_FAR;
    let c = frame.at(0.0, wall - 0.05, 3.85);
    let r = 1.55;
    lib.decor(piece(
        frame,
        (-1.85, 1.85),
        (wall - 0.04, wall),
        (2.15, 5.55),
        kind::WOOD,
        CASEWORK,
    ));
    dial(lib, c, n, r, 10, 9.0);
    // Glowing hours: the clock's touch of magic, the time it keeps for the books.
    let side = Vec3::Y.cross(n);
    for h in 0..12 {
        let th = h as f32 * TAU / 12.0;
        let p = c + (side * th.sin() + Vec3::Y * th.cos()) * (r * 0.7) + n * 0.03;
        let gem = Inst::new(
            p,
            on_plane(side + Vec3::Y, n),
            Vec3::new(0.1, 0.1, 0.02),
            kind::CRYSTAL,
            rgba(GLOW, 2.5),
            0,
        );
        lib.decor(gem);
    }
    // The pendulum's case: a plinth, posts, a back panel and a crown under the dial.
    let d = wall - 0.28;
    lib.decor(piece(frame, (-0.85, 0.85), (d, wall), (0.0, 0.35), kind::WOOD, WALNUT));
    for s in [-1.0f32, 1.0] {
        lib.decor(piece(
            frame,
            (s * 0.75 - 0.09, s * 0.75 + 0.09),
            (d + 0.02, wall),
            (0.35, 2.08),
            kind::WOOD,
            WALNUT,
        ));
    }
    lib.decor(piece(
        frame,
        (-0.66, 0.66),
        (wall - 0.03, wall),
        (0.35, 2.08),
        kind::WOOD,
        DARK_WOOD,
    ));
    lib.decor(piece(
        frame,
        (-0.9, 0.9),
        (d - 0.02, wall),
        (2.08, 2.22),
        kind::WOOD,
        WALNUT,
    ));
    guard(lib, frame, (-0.9, 0.9), (d - 0.02, wall), 2.22);
    pendulum(lib, frame.at(0.0, wall - 0.15, 2.08), n, 1.42, 0.25, 0.06);
    lib.light(c + n * 1.6, 6.0, [1.0, 0.8, 0.55], 0.6, LightKind::Static);
    // Moonlit windows either side, with gears above them.
    for s in [-1.0f32, 1.0] {
        let a = s * 4.75;
        let wc = frame.at(a, wall - 0.02, 2.75);
        let mut glass = Inst::aabox(wc, frame.size(1.3, 3.1, 0.08), kind::GLASS, [0.32, 0.42, 0.85]);
        glass.color[3] = 1.2;
        lib.decor(glass);
        lib.decor(piece(
            frame,
            (a - 0.78, a + 0.78),
            (wall - 0.14, wall),
            (1.05, 1.15),
            kind::STONE,
            [0.45, 0.42, 0.38],
        ));
        lib.light(wc - out * 1.5, 7.0, [0.45, 0.55, 1.0], 0.7, LightKind::Static);
        let beam = (-out * 1.0 - Vec3::Y * 0.6).normalize();
        let len = 6.0;
        lib.static_trans.push(Inst::new(
            wc + beam * len * 0.5,
            Quat::from_rotation_arc(Vec3::Y, beam),
            Vec3::new(1.1, len, 2.2),
            kind::SHAFT,
            [0.35, 0.45, 0.9, 0.04],
            0,
        ));
        gear(
            lib,
            frame.at(a - s * 0.3, wall - 0.05, 4.85),
            n,
            wheel(0.5, 12, 0.05).old().turned(0.4),
        );
        gear(
            lib,
            frame.at(a + s * 0.42, wall - 0.09, 4.62),
            n,
            wheel(0.28, 8, 0.05).turned(1.1),
        );
    }
}

/// Panelling, great gears and a clock either side of the door.
fn near_wall(lib: &mut Library, frame: &RoomFrame) {
    let near = RoomFrame::O_NEAR;
    let inner = RoomFrame::A_HALF - UNIT_PITCH * 2.0;
    let d = frame.door;
    for (a0, a1) in [
        (-inner + 0.02, d - RoomFrame::DOOR_HALF - 0.32),
        (d + RoomFrame::DOOR_HALF + 0.32, inner - 0.02),
    ] {
        lib.wainscot(frame.y, frame.rect(a0, a1, near, near + 0.07), frame.side.out);
    }
    let n = frame.out();
    for s in [-1.0f32, 1.0] {
        let a = d + s * 4.6;
        gear(
            lib,
            frame.at(a, near + 0.06, 3.9),
            n,
            wheel(1.15, 18, 0.08).old().turned(0.2),
        );
        gear(
            lib,
            frame.at(a + s * 1.45, near + 0.1, 4.8),
            n,
            wheel(0.52, 10, 0.06).turned(0.5),
        );
        wall_clock(
            lib,
            frame.at(d + s * 2.55, near + 0.06, 2.6),
            n,
            0.3,
            (s > 0.0) as u32 * 7 + 3,
        );
    }
}

/// The workshop by the door: a workbench with a vice, tools, a lamp and a mechanism
/// half built, a stool; and an automaton (a brass owl) on a plinth across the way.
fn workshop(lib: &mut Library, frame: &RoomFrame) {
    let (out, across) = (frame.out(), frame.across());
    let (a0, a1, o0, o1) = (-6.6, -3.8, 16.75, 17.65);
    let top = 0.92;
    lib.decor(piece(
        frame,
        (a0, a1),
        (o0, o1),
        (top - 0.07, top),
        kind::WOOD,
        [0.22, 0.12, 0.05],
    ));
    for (a, o) in [
        (a0 + 0.1, o0 + 0.1),
        (a1 - 0.1, o0 + 0.1),
        (a0 + 0.1, o1 - 0.1),
        (a1 - 0.1, o1 - 0.1),
    ] {
        lib.decor(piece(
            frame,
            (a - 0.06, a + 0.06),
            (o - 0.06, o + 0.06),
            (0.0, top - 0.07),
            kind::WOOD,
            DARK_WOOD,
        ));
    }
    lib.decor(piece(
        frame,
        (a0 + 0.08, a1 - 0.08),
        (o0 + 0.15, o1 - 0.15),
        (0.18, 0.22),
        kind::WOOD,
        WALNUT,
    ));
    guard(lib, frame, (a0, a1), (o0, o1), top);
    let on = |a: f32, o: f32, dy: f32| frame.at(a, o, top + dy);
    // A vice at the end, tools laid along the edge.
    lib.decor(piece(
        frame,
        (a1 - 0.35, a1 - 0.05),
        (o0 + 0.05, o0 + 0.3),
        (top, top + 0.16),
        kind::METAL,
        STEEL,
    ));
    lib.decor(piece(
        frame,
        (a1 - 0.4, a1 - 0.31),
        (o0 + 0.1, o0 + 0.25),
        (top + 0.04, top + 0.12),
        kind::METAL,
        STEEL,
    ));
    let turn = Quat::from_mat3(&Mat3::from_cols(across, Vec3::Y, -out));
    let tools = [
        (-6.3, 0.32, 0.03, STEEL),
        (-6.05, 0.26, 0.02, BRASS_LIT),
        (-5.85, 0.3, 0.025, STEEL),
    ];
    for (k, (a, len, w, rgb)) in tools.into_iter().enumerate() {
        let q = turn * Quat::from_rotation_y(0.2 * k as f32 - 0.2);
        lib.decor(Inst::new(
            on(a, o0 + 0.2, 0.012),
            q,
            Vec3::new(w, 0.02, len),
            kind::METAL,
            rgba(rgb, 1.0),
            0,
        ));
    }
    // The mechanism: an open brass frame with gears in it, loose wheels beside it.
    let m = (-5.2, (o0 + o1) * 0.5);
    for (da, d_o) in [(-0.22, -0.16), (0.22, -0.16), (-0.22, 0.16), (0.22, 0.16)] {
        let (pa, po) = (m.0 + da, m.1 + d_o);
        lib.decor(piece(
            frame,
            (pa - 0.015, pa + 0.015),
            (po - 0.015, po + 0.015),
            (top, top + 0.42),
            kind::METAL,
            BRASS_LIT,
        ));
    }
    lib.decor(piece(
        frame,
        (m.0 - 0.25, m.0 + 0.25),
        (m.1 - 0.19, m.1 + 0.19),
        (top + 0.42, top + 0.44),
        kind::METAL,
        BRASS_LIT,
    ));
    gear(lib, on(m.0, m.1 - 0.05, 0.22), out, wheel(0.15, 8, 0.02));
    gear(
        lib,
        on(m.0 + 0.1, m.1 + 0.08, 0.3),
        out,
        wheel(0.09, 8, 0.02).old().turned(0.3),
    );
    gear(
        lib,
        on(-4.45, o0 + 0.3, 0.012),
        Vec3::Y,
        wheel(0.12, 9, 0.02).turned(0.4),
    );
    // A wheel that turns itself, hovering over the work: the bench's bit of magic.
    gear(
        lib,
        on(-4.6, m.1, 0.7),
        out,
        wheel(0.17, 8, 0.03).glowing(0.9).turned(0.6),
    );
    lib.light(on(-4.6, m.1, 0.7), 2.5, GLOW, 0.5, LightKind::Static);
    // An oil lamp.
    let lamp = on(-6.35, o1 - 0.2, 0.0);
    lib.decor(Inst::aabox(
        lamp + Vec3::Y * 0.05,
        Vec3::new(0.14, 0.1, 0.14),
        kind::METAL,
        BRASS_LIT,
    ));
    let mut chimney = Inst::aabox(
        lamp + Vec3::Y * 0.17,
        Vec3::new(0.09, 0.14, 0.09),
        kind::EMISSIVE,
        [1.0, 0.7, 0.35],
    );
    chimney.color[3] = 2.0;
    chimney.param = 1;
    lib.decor(chimney);
    lib.light(lamp + Vec3::Y * 0.4, 4.5, [1.0, 0.62, 0.3], 0.8, LightKind::Flicker);
    // A stool at the bench.
    let st = (-5.0, o0 - 0.5);
    lib.decor(piece(
        frame,
        (st.0 - 0.2, st.0 + 0.2),
        (st.1 - 0.2, st.1 + 0.2),
        (0.62, 0.67),
        kind::WOOD,
        WALNUT,
    ));
    for (da, d_o) in [(-0.15, -0.15), (0.15, -0.15), (-0.15, 0.15), (0.15, 0.15)] {
        let (pa, po) = (st.0 + da, st.1 + d_o);
        lib.decor(piece(
            frame,
            (pa - 0.025, pa + 0.025),
            (po - 0.025, po + 0.025),
            (0.0, 0.62),
            kind::WOOD,
            DARK_WOOD,
        ));
    }
    guard(lib, frame, (st.0 - 0.22, st.0 + 0.22), (st.1 - 0.22, st.1 + 0.22), 0.67);
    // The automaton: a brass owl on a plinth, its eyes lit.
    let (pa, po) = (5.6, 18.6);
    lib.solid(
        piece(
            frame,
            (pa - 0.4, pa + 0.4),
            (po - 0.4, po + 0.4),
            (0.0, 1.0),
            kind::STONE,
            [0.42, 0.39, 0.35],
        ),
        Stops::ALL,
    );
    lib.decor(piece(
        frame,
        (pa - 0.46, pa + 0.46),
        (po - 0.46, po + 0.46),
        (1.0, 1.06),
        kind::STONE,
        [0.48, 0.45, 0.4],
    ));
    let base = frame.at(pa, po, 1.06);
    let face = -out;
    let owl = |lib: &mut Library, off: Vec3, size: Vec3, rgb: [f32; 3], q: Quat| {
        lib.decor(Inst::new(base + off, turn * q, size, kind::METAL, rgba(rgb, 1.0), 0));
    };
    owl(
        lib,
        Vec3::Y * 0.28,
        Vec3::new(0.34, 0.46, 0.3),
        BRASS_OLD,
        Quat::IDENTITY,
    );
    owl(
        lib,
        Vec3::Y * 0.27 + face * 0.04,
        Vec3::new(0.26, 0.4, 0.3),
        BRASS_LIT,
        Quat::from_rotation_y(0.785),
    );
    owl(
        lib,
        Vec3::Y * 0.62,
        Vec3::new(0.3, 0.24, 0.26),
        BRASS_OLD,
        Quat::from_rotation_y(0.785),
    );
    for s in [-1.0f32, 1.0] {
        // Ear tufts and folded wings.
        owl(
            lib,
            Vec3::Y * 0.77 + across * (s * 0.1),
            Vec3::new(0.05, 0.12, 0.05),
            BRASS_LIT,
            Quat::from_rotation_z(-s * 0.4),
        );
        owl(
            lib,
            Vec3::Y * 0.3 + across * (s * 0.17),
            Vec3::new(0.05, 0.36, 0.24),
            BRASS_LIT,
            Quat::from_rotation_z(s * 0.15),
        );
        let eye = base + Vec3::Y * 0.64 + across * (s * 0.075) + face * 0.19;
        let lit = turn * Quat::from_rotation_z(0.785);
        lib.decor(Inst::new(
            eye,
            lit,
            Vec3::new(0.06, 0.06, 0.03),
            kind::CRYSTAL,
            [1.0, 0.75, 0.25, 2.5],
            0,
        ));
    }
    owl(
        lib,
        Vec3::Y * 0.57 + face * 0.17,
        Vec3::new(0.05, 0.08, 0.06),
        STEEL,
        Quat::from_rotation_x(0.5),
    );
}

/// Beams across the ceiling, and trains of flat gears meshing under it.
fn ceiling(lib: &mut Library, frame: &RoomFrame) {
    let h = RoomFrame::HEIGHT;
    let inner = RoomFrame::A_HALF;
    for o in [16.6, 27.6, 30.6] {
        lib.decor(piece(
            frame,
            (-inner, inner),
            (o - 0.13, o + 0.13),
            (h - 0.3, h),
            kind::WOOD,
            [0.11, 0.055, 0.025],
        ));
    }
    // Down the middle between the pillars, and across the far end.
    let y = h - 0.42;
    let middle: &[(f32, usize)] = &[(0.7, 14), (0.38, 8), (0.9, 18)];
    let far: &[(f32, usize)] = &[(0.55, 12), (0.3, 8), (0.7, 16), (0.3, 8), (0.55, 12)];
    for (wheels, along_o) in [(middle, true), (far, false)] {
        // Centres along the train, each wheel meshing with the last.
        let mut at = vec![0.0f32];
        for w in wheels.windows(2) {
            at.push(at[at.len() - 1] + w[0].0 + w[1].0 + 0.1);
        }
        let shift = if along_o { 22.0 } else { -at[at.len() - 1] * 0.5 };
        for (k, (&(r, teeth), &p)) in wheels.iter().zip(&at).enumerate() {
            let wobble = if k % 2 == 0 { 0.0 } else { 0.22 };
            let (a, o) = if along_o {
                (wobble, shift + p)
            } else {
                (shift + p, 29.1 + wobble)
            };
            let g = wheel(r, teeth, 0.05).turned(k as f32 * 0.4);
            let g = if k % 2 == 0 { g.old() } else { g };
            gear(lib, frame.at(a, o, y - 0.03 * (k % 2) as f32), Vec3::Y, g);
            // Its arbor, up into the ceiling.
            lib.decor(piece(
                frame,
                (a - 0.03, a + 0.03),
                (o - 0.03, o + 0.03),
                (y, h),
                kind::METAL,
                STEEL,
            ));
        }
    }
}
