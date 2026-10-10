//! The Clockworks: department 8, top floor, north side. Bays of cases stand along
//! both side walls like the teeth of a wheel; two pillars of cases on the floor show
//! their works through open fronts under clock turrets; the great clock fills the
//! middle of the far wall with its pendulum below; a workbench and an automaton (a
//! brass owl looking about the room) stand by the door; gears turn on the walls and
//! under the ceiling, meshing in trains that keep their teeth together, and the
//! clocks keep time.

use super::hearth_library::{FLAME, turned_leg};
use super::*;
use std::f32::consts::{FRAC_PI_2, PI};

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
    // The clock turret over the near end, up to the ceiling, with a dial on its face
    // between turned columns.
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
    let column = lib.turned(
        &[
            (0.0, 0.0),
            (0.5, 0.0),
            (0.5, 0.03),
            (0.4, 0.05),
            (0.34, 0.08),
            (0.3, 0.9),
            (0.36, 0.93),
            (0.5, 0.96),
            (0.5, 1.0),
            (0.0, 1.0),
        ],
        16,
    );
    for s in [-1.0f32, 1.0] {
        lib.decor(column.sized(Vec3::new(0.16, h - 0.24 - CASE_TOP - 0.12, 0.16)).at(
            frame.at(a + s * (hw - 0.42), o0 - 0.02, CASE_TOP + 0.12),
            Quat::IDENTITY,
            kind::WOOD,
            rgba(WALNUT, 1.0),
        ));
    }
    let n = -out;
    dial(
        lib,
        frame.at(a, o0 - 0.02, 4.25),
        n,
        0.68,
        (a > 0.0) as u32 * 5 + 2,
        0.0,
    );
    // The movement standing on the pillar's top, behind the turret: a great wheel
    // driving a smaller one, each on a post.
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
    let great = gear(
        lib,
        wheel(0.85, 16, 0.07)
            .old()
            .turned(a)
            .at(frame.at(a, mid - 0.45, top + 0.95), across),
    );
    let small = gear(lib, great.mesh(out * 1.4 - Vec3::Y * 0.35, 0.06, wheel(0.0, 10, 0.06)));
    for p in [great.c, small.c] {
        let foot = Vec3::new(p.x, frame.y + top, p.z);
        lib.pole(foot, p, 0.06, kind::METAL, rgba(STEEL, 1.0));
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
    let first = gear(
        lib,
        wheel(0.42, 12, 0.04)
            .turned(0.2)
            .at(frame.at(a - 0.35, face - 0.03, 2.35), n),
    );
    gear(
        lib,
        first.mesh(across * 0.73 - Vec3::Y * 0.6, 0.0, wheel(0.0, 8, 0.04).old()),
    );
    pendulum(lib, frame.at(a, face - 0.16, 2.9), n, 1.85, 0.2, 0.05);
}

/// A pendulum hanging from `top`, facing `n`: a rod `len` long and a brass bob of
/// radius `r`, `t` thick, swinging in the plane it faces as a pendulum that long does.
fn pendulum(lib: &mut Library, top: Vec3, n: Vec3, len: f32, r: f32, t: f32) {
    let period = TAU * libm::sqrtf(len / 9.81);
    let m = lib.mover(
        top,
        Motion::Swing {
            axis: n,
            amp: 0.1,
            period,
        },
        0.0,
        None,
    );
    let rod = lib.shape(Mesh::cylinder(8));
    lib.moving(
        m,
        rod.sized(Vec3::new(0.025, len, 0.025)).inst(
            top - Vec3::Y * (len * 0.5),
            Quat::IDENTITY,
            kind::METAL,
            rgba(STEEL, 1.0),
        ),
    );
    // A lens of brass, seen face on.
    let bob = lib.shape(Mesh::sphere(24));
    let face = Quat::from_rotation_arc(Vec3::Y, n);
    lib.moving(
        m,
        bob.sized(Vec3::new(r * 2.0, t, r * 2.0))
            .inst(top - Vec3::Y * len, face, kind::METAL, rgba(BRASS_LIT, 1.0)),
    );
}

/// A gear's make: radius to the roots of its teeth, how many teeth, thickness, its
/// colour and material, how far it is turned (radians), how fast it turns (radians a
/// second; see `wheel`), and the mover that carries it, if any.
#[derive(Clone, Copy)]
struct Wheel {
    r: f32,
    teeth: usize,
    t: f32,
    col: [f32; 4],
    kind: u32,
    phase: f32,
    rate: f32,
    rides: Option<u32>,
}

/// How fast the teeth of the room's wheels go by (metres a second at their roots):
/// meshing wheels turn at the rates that keep their teeth together.
const TOOTH_SPEED: f32 = 0.035;
/// How far a wheel's teeth stand proud of its roots, for its radius.
const TOOTH: f32 = 0.13;

/// A polished brass wheel, turning as one in a train would.
fn wheel(r: f32, teeth: usize, t: f32) -> Wheel {
    Wheel {
        r,
        teeth,
        t,
        col: rgba(BRASS_LIT, 1.0),
        kind: kind::METAL,
        phase: 0.0,
        rate: TOOTH_SPEED / r,
        rides: None,
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

    /// Lying still.
    fn still(self) -> Self {
        Wheel { rate: 0.0, ..self }
    }

    /// Carried by mover `m` as it turns.
    fn riding(self, m: u32) -> Self {
        Wheel { rides: Some(m), ..self }
    }

    /// Glowing from inside, `w` brightly.
    fn glowing(self, w: f32) -> Self {
        Wheel {
            col: rgba(GLOW, w),
            kind: kind::CRYSTAL,
            ..self
        }
    }

    /// Centred on `c`, turning about `axis`.
    fn at(self, c: Vec3, axis: Vec3) -> Placed {
        Placed { c, axis, w: self }
    }
}

/// A wheel in its place: its middle, and the axis it turns about.
#[derive(Clone, Copy)]
struct Placed {
    c: Vec3,
    axis: Vec3,
    w: Wheel,
}

impl Placed {
    /// `next` meshing with this wheel off toward `dir` (in its plane), `lift` along
    /// the axis: its radius set by its teeth to this one's pitch, its teeth between
    /// this one's where they meet, turning the other way at the rate that keeps them
    /// there.
    fn mesh(self, dir: Vec3, lift: f32, next: Wheel) -> Placed {
        let (a, na, nb) = (self.w, self.w.teeth as f32, next.teeth as f32);
        let r = a.r * nb / na;
        let d = (dir - self.axis * dir.dot(self.axis)).normalize();
        let c = self.c + d * (a.r + r + TOOTH * a.r.max(r)) + self.axis * lift;
        // The way to the other wheel, as an angle in the plane the teeth are laid out
        // in (see `gear`).
        let local = Quat::from_rotation_arc(Vec3::Z, self.axis).inverse() * d;
        let th = libm::atan2f(local.y, local.x);
        let (step_a, step_b) = (TAU / na, TAU / nb);
        // How far past that line this wheel's nearest tooth stands (its middle a
        // quarter step on from where it starts); the other's gap stands as far the
        // other way, in its own steps.
        let past = a.phase + 0.25 * step_a - th;
        let phase = th + PI - past * step_b / step_a - 0.75 * step_b;
        Placed {
            c,
            axis: self.axis,
            w: Wheel {
                r,
                phase,
                rate: -a.rate * na / nb,
                rides: a.rides,
                ..next
            },
        }
    }
}

/// A gear in its place, turning: a toothed rim, spokes and a hub (whole, when small),
/// its teeth starting at its phase and going round anticlockwise about its axis.
fn gear(lib: &mut Library, p: Placed) -> Placed {
    let (c, axis, g) = (p.c, p.axis, p.w);
    let spokes = if g.r > 0.4 {
        4
    } else if g.r > 0.12 {
        3
    } else {
        0
    };
    let shape = lib.shape(Mesh::gear(g.teeth.max(6), TOOTH, 0.1, spokes));
    // Made 1 to the roots, 0.1 thick (the hub standing proud of that); placed by its
    // axis, which an odd number of teeth leaves off the middle of its bounds.
    let size = Vec3::new(shape.size.x * g.r, shape.size.y * g.r, shape.size.z / 0.1 * g.t);
    let rot = Quat::from_rotation_arc(Vec3::Z, axis) * Quat::from_rotation_z(g.phase);
    let i = shape.sized(size).at(c, rot, g.kind, g.col);
    if g.rate == 0.0 && g.rides.is_none() {
        lib.decor(i);
    } else {
        let m = lib.mover(c, Motion::Spin { axis, rate: g.rate }, 0.0, g.rides);
        lib.moving(m, i);
    }
    p
}

/// A clock dial at `c` facing `n` (level): a cream face, a brass bezel, hour marks
/// and hands at `hour`:`minute`, keeping time from there; `r` its radius. A great dial
/// has a second hand too.
fn dial(lib: &mut Library, c: Vec3, n: Vec3, r: f32, hour: u32, minute: f32) {
    dial_face(lib, c, n, r);
    lib.clock_hands(
        (c + n * 0.01, n, r),
        (hour as f32, minute),
        ([0.06, 0.06, 0.08, 1.0], rgba(BRASS_LIT, 1.0)),
        r > 1.0,
    );
}

/// A dial without its hands: a cream face, a brass bezel and the hour marks.
fn dial_face(lib: &mut Library, c: Vec3, n: Vec3, r: f32) {
    let up = Vec3::Y;
    let side = up.cross(n);
    let dir = |th: f32| side * libm::cosf(th) + up * libm::sinf(th);
    let facing = Quat::from_rotation_arc(Vec3::Z, n);
    let segs = if r > 1.0 { 64 } else { 40 };
    let disc = lib.shape(Mesh::cylinder(segs));
    lib.decor(disc.sized(Vec3::new(r * 2.0, 0.02, r * 2.0)).inst(
        c,
        Quat::from_rotation_arc(Vec3::Y, n),
        kind::PAPER,
        [0.0; 4],
    ));
    let rim = (r * 0.09).max(0.04);
    let bezel = lib.shape(Mesh::ring(&circle(r + rim, segs), &circle(r - 0.01, segs), 0.06));
    lib.decor(bezel.inst(c + n * 0.01, facing, kind::METAL, rgba(BRASS_LIT, 1.0)));
    // Hour marks, long at the quarters; small dials only at the quarters.
    let every = if r > 0.5 { 1 } else { 3 };
    for h in (0..12).step_by(every) {
        let radial = dir(FRAC_PI_2 - h as f32 * TAU / 12.0);
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
}

/// The ink a stopped clock's key is lettered in on its face.
const KEY_LETTERS: Style = Style {
    em: 0.05,
    tracking: 0.0,
    lettering: Lettering::Ink,
    color: [0.06, 0.04, 0.03, 1.0],
};

/// A stopped clock floating at `c` (generation 5's stasis, `upstairs`): a dial of
/// radius `r` facing `n` on a round walnut case, the title of the series that starts it
/// again lettered across its face below the arbor as a maker's name is. Returns its
/// hands at twelve (hour, minute), which the game turns.
pub(in crate::library) fn stopped_clock(lib: &mut Library, c: Vec3, n: Vec3, r: f32, key: &str) -> [Inst; 2] {
    let case = lib.shape(Mesh::cylinder(40));
    lib.decor(case.sized(Vec3::new(r * 2.3, 0.12, r * 2.3)).inst(
        c - n * 0.07,
        Quat::from_rotation_arc(Vec3::Y, n),
        kind::WOOD,
        rgba(CASEWORK, 1.0),
    ));
    dial_face(lib, c, n, r);
    lib.inscribe(key, KEY_LETTERS, c - Vec3::Y * (r * 0.42) + n * 0.011, n, r * 1.3);
    let face = c + n * 0.01;
    lib.clock_cap((face, n, r), rgba(BRASS_LIT, 1.0));
    let ink = [0.06, 0.06, 0.08, 1.0];
    Library::CLOCK_HANDS.map(|(len, w, z)| lib.clock_hand((face, n), 0.0, (r * len, r * w), z, ink))
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

/// The great clock's hours in Roman figures, its four as clockmakers write it.
const NUMERALS: [&str; 12] = [
    "XII", "I", "II", "III", "IIII", "V", "VI", "VII", "VIII", "IX", "X", "XI",
];

/// The far wall's great clock: a dial over three metres across, its hours glowing
/// and figured, a pendulum in an open case under it, and moonlit windows either side
/// with gears meshing over them.
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
    // Glowing hours: the clock's touch of magic, the time it keeps for the books;
    // their figures inside them.
    let side = Vec3::Y.cross(n);
    let figures = Style {
        em: 0.17,
        tracking: 0.02,
        lettering: Lettering::Ink,
        color: [0.06, 0.045, 0.035, 1.0],
    };
    for (h, figure) in NUMERALS.into_iter().enumerate() {
        let th = h as f32 * TAU / 12.0;
        let radial = side * libm::sinf(th) + Vec3::Y * libm::cosf(th);
        let gem = Inst::new(
            c + radial * (r * 0.7) + n * 0.03,
            on_plane(side + Vec3::Y, n),
            Vec3::new(0.1, 0.1, 0.02),
            kind::CRYSTAL,
            rgba(GLOW, 2.5),
            0,
        );
        lib.decor(gem);
        lib.inscribe(figure, figures, c + radial * (r * 0.53) + n * 0.01, n, 0.4);
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
    // Moonlit windows either side, with a pair of wheels meshing above each.
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
        let big = gear(
            lib,
            wheel(0.5, 12, 0.05)
                .old()
                .turned(0.4)
                .at(frame.at(a - s * 0.3, wall - 0.05, 4.85), n),
        );
        gear(
            lib,
            big.mesh(frame.across() * (s * 0.72) - Vec3::Y * 0.23, 0.03, wheel(0.0, 8, 0.05)),
        );
    }
}

/// Panelling, a great wheel driving a small one and a clock either side of the door.
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
        let great = gear(
            lib,
            wheel(1.15, 18, 0.08)
                .old()
                .turned(0.2)
                .at(frame.at(a, near + 0.06, 3.9), n),
        );
        gear(
            lib,
            great.mesh(frame.across() * (s * 1.45) + Vec3::Y * 0.9, 0.04, wheel(0.0, 8, 0.06)),
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

/// The workshop by the door: a workbench with a vice, tools, an oil lamp and a
/// mechanism half built, a stool; and an automaton (a brass owl) on a plinth across
/// the way.
fn workshop(lib: &mut Library, frame: &RoomFrame) {
    let (out, across) = (frame.out(), frame.across());
    let (a0, a1, o0, o1) = (-6.6, -3.8, 16.75, 17.65);
    let top = 0.92;
    let steel = rgba(STEEL, 1.0);
    let brass = rgba(BRASS_LIT, 1.0);
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
    // A vice at the end, its screw and tommy bar; tools laid along the edge.
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
    let screw = (on(a1 - 0.5, o0 + 0.175, 0.08), on(a1 - 0.3, o0 + 0.175, 0.08));
    lib.pole(screw.0, screw.1, 0.025, kind::METAL, steel);
    lib.pole(screw.0 - out * 0.1, screw.0 + out * 0.1, 0.012, kind::METAL, steel);
    let wood = rgba([0.4, 0.22, 0.09], 1.0);
    let edge = |a: f32| on(a, o0 + 0.2, 0.014);
    // A screwdriver: a turned handle and a steel blade.
    let handle = lib.turned(
        &[
            (0.0, 0.0),
            (0.3, 0.0),
            (0.45, 0.1),
            (0.5, 0.5),
            (0.42, 0.85),
            (0.25, 1.0),
            (0.0, 1.0),
        ],
        12,
    );
    let (p, dir) = (edge(-6.3), (-out * 0.95 + across * 0.3).normalize());
    let along = Quat::from_rotation_arc(Vec3::Y, dir);
    lib.decor(handle.sized(Vec3::new(0.03, 0.11, 0.03)).at(p, along, kind::WOOD, wood));
    lib.pole(p + dir * 0.11, p + dir * 0.27, 0.006, kind::METAL, steel);
    // A hammer: a handle and a head across its end.
    let (p, dir) = (edge(-6.05), (-out * 0.9 - across * 0.4).normalize());
    lib.pole(p, p + dir * 0.28, 0.022, kind::WOOD, wood);
    lib.decor(Inst::new(
        p + dir * 0.28,
        Quat::from_rotation_arc(Vec3::Y, dir.cross(Vec3::Y)),
        Vec3::new(0.03, 0.1, 0.03),
        kind::METAL,
        steel,
        0,
    ));
    // Brass dividers, opened a little.
    let (p, dir) = (edge(-5.8), (-out * 0.95 + across * 0.3).normalize());
    for s in [-1.0f32, 1.0] {
        let leg = (dir + Vec3::Y.cross(dir) * (s * 0.12)).normalize();
        lib.pole(p, p + leg * 0.2, 0.007, kind::METAL, brass);
    }
    let pin = lib.shape(Mesh::cylinder(12));
    lib.decor(
        pin.sized(Vec3::new(0.02, 0.01, 0.02))
            .inst(p, Quat::IDENTITY, kind::METAL, brass),
    );
    // The mechanism: an open brass frame with a pair of wheels in it, a loose wheel
    // lying beside it.
    let m = (-5.2, (o0 + o1) * 0.5);
    for (da, d_o) in [(-0.22, -0.16), (0.22, -0.16), (-0.22, 0.16), (0.22, 0.16)] {
        let foot = on(m.0 + da, m.1 + d_o, 0.0);
        lib.pole(foot, foot + Vec3::Y * 0.42, 0.025, kind::METAL, brass);
    }
    lib.decor(piece(
        frame,
        (m.0 - 0.25, m.0 + 0.25),
        (m.1 - 0.19, m.1 + 0.19),
        (top + 0.42, top + 0.44),
        kind::METAL,
        BRASS_LIT,
    ));
    let driver = gear(lib, wheel(0.11, 8, 0.02).at(on(m.0 - 0.07, m.1 - 0.05, 0.2), out));
    gear(
        lib,
        driver.mesh(across * 0.8 + Vec3::Y * 0.45, 0.02, wheel(0.0, 6, 0.02).old()),
    );
    gear(
        lib,
        wheel(0.12, 9, 0.02)
            .turned(0.4)
            .still()
            .at(on(-4.45, o0 + 0.3, 0.012), Vec3::Y),
    );
    // A wheel that turns itself, floating over the work: the bench's bit of magic.
    let hover = on(-4.6, m.1, 0.7);
    let float = Motion::Bob {
        amp: 0.03,
        period: 5.0,
        rock: 0.05,
    };
    let floating = lib.mover(hover, float, 0.0, None);
    gear(
        lib,
        wheel(0.17, 8, 0.03)
            .glowing(0.9)
            .turned(0.6)
            .riding(floating)
            .at(hover, out),
    );
    lib.light_riding(floating, hover, 2.5, GLOW, 0.5, LightKind::Static);
    // An oil lamp: a brass font, a glass chimney, its flame.
    let lamp = on(-6.35, o1 - 0.2, 0.0);
    let font = lib.turned(
        &[
            (0.0, 0.0),
            (0.07, 0.0),
            (0.07, 0.01),
            (0.04, 0.03),
            (0.035, 0.05),
            (0.065, 0.08),
            (0.06, 0.11),
            (0.035, 0.13),
            (0.0, 0.13),
        ],
        24,
    );
    lib.decor(font.at(lamp, Quat::IDENTITY, kind::METAL, brass));
    let chimney = lib.turned(
        &[(0.032, 0.0), (0.045, 0.04), (0.045, 0.08), (0.025, 0.14), (0.025, 0.2)],
        24,
    );
    lib.translucent(chimney.at(
        lamp + Vec3::Y * 0.13,
        Quat::IDENTITY,
        kind::CLEAR,
        [0.95, 0.9, 0.8, 0.08],
    ));
    lib.translucent(flame(lamp + Vec3::Y * 0.135, (0.022, 0.05), FLAME));
    lib.light(lamp + Vec3::Y * 0.4, 4.5, [1.0, 0.62, 0.3], 0.8, LightKind::Flicker);
    // A stool at the bench: a round seat on turned legs.
    let st = (-5.0, o0 - 0.5);
    let seat = lib.shape(Mesh::cylinder(24));
    lib.decor(seat.sized(Vec3::new(0.38, 0.05, 0.38)).inst(
        frame.at(st.0, st.1, 0.645),
        Quat::IDENTITY,
        kind::WOOD,
        rgba(WALNUT, 1.0),
    ));
    let leg = turned_leg(lib);
    for (da, d_o) in [(-0.12, -0.12), (0.12, -0.12), (-0.12, 0.12), (0.12, 0.12)] {
        lib.decor(leg.sized(Vec3::new(0.05, 0.62, 0.05)).at(
            frame.at(st.0 + da, st.1 + d_o, 0.0),
            Quat::IDENTITY,
            kind::WOOD,
            rgba(DARK_WOOD, 1.0),
        ));
    }
    guard(lib, frame, (st.0 - 0.22, st.0 + 0.22), (st.1 - 0.22, st.1 + 0.22), 0.67);
    owl(lib, frame, 5.6, 18.6);
}

/// The automaton: a brass owl on a plinth at `pa`, `po`, its eyes lit, turning its
/// head now and then to look about the room.
fn owl(lib: &mut Library, frame: &RoomFrame, pa: f32, po: f32) {
    let across = frame.across();
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
    // It faces the door, as the room's frame does (z toward the door).
    let face = -frame.out();
    let turn = frame.turn();
    let (old, lit, steel) = (rgba(BRASS_OLD, 1.0), rgba(BRASS_LIT, 1.0), rgba(STEEL, 1.0));
    let perch = lib.turned(
        &[(0.0, 0.0), (0.5, 0.0), (0.5, 0.3), (0.4, 0.5), (0.42, 1.0), (0.0, 1.0)],
        24,
    );
    lib.decor(
        perch
            .sized(Vec3::new(0.3, 0.05, 0.3))
            .at(base, Quat::IDENTITY, kind::METAL, old),
    );
    // The body, an egg of old brass, a brighter breast on it, wings folded at its
    // sides, talons gripping the perch.
    let body = lib.turned(
        &[
            (0.0, 0.0),
            (0.3, 0.0),
            (0.42, 0.08),
            (0.49, 0.25),
            (0.5, 0.42),
            (0.46, 0.6),
            (0.38, 0.76),
            (0.25, 0.9),
            (0.0, 0.96),
        ],
        32,
    );
    let foot = base + Vec3::Y * 0.05;
    lib.decor(body.sized(Vec3::new(0.32, 0.46, 0.28)).at(foot, turn, kind::METAL, old));
    let round = lib.shape(Mesh::sphere(24));
    lib.decor(
        round
            .sized(Vec3::new(0.22, 0.3, 0.1))
            .inst(foot + Vec3::Y * 0.21 + face * 0.1, turn, kind::METAL, lit),
    );
    for s in [-1.0f32, 1.0] {
        // Folded along its side, leaning out a little at the top and trailing behind.
        let fold = turn * Quat::from_rotation_z(-s * 0.12) * Quat::from_rotation_x(-0.25);
        lib.decor(round.sized(Vec3::new(0.07, 0.36, 0.22)).inst(
            foot + Vec3::Y * 0.21 + across * (s * 0.135) - face * 0.03,
            fold,
            kind::METAL,
            lit,
        ));
        for k in 0..3 {
            let toe = foot + across * (s * 0.05 + (k as f32 - 1.0) * 0.022) + face * 0.11;
            lib.pole(toe, toe + face * 0.05 - Vec3::Y * 0.04, 0.014, kind::METAL, steel);
        }
    }
    // The head, looking about: a slow turn, and a glance on top of it.
    let neck = foot + Vec3::Y * 0.44;
    let look = Motion::Swing {
        axis: Vec3::Y,
        amp: 0.6,
        period: 13.0,
    };
    let glance = Motion::Swing {
        axis: Vec3::Y,
        amp: 0.3,
        period: 5.3,
    };
    let looking = lib.mover(neck, look, 0.0, None);
    let head = lib.mover(neck, glance, 1.7, Some(looking));
    let crown = neck + Vec3::Y * 0.12;
    lib.moving(
        head,
        round
            .sized(Vec3::new(0.28, 0.23, 0.25))
            .inst(crown, turn, kind::METAL, old),
    );
    let disc = lib.shape(Mesh::cylinder(24));
    let flat = Quat::from_rotation_arc(Vec3::Y, face);
    let cone = lib.shape(Mesh::cone(12));
    for s in [-1.0f32, 1.0] {
        let eye = crown + Vec3::Y * 0.015 + across * (s * 0.065) + face * 0.105;
        lib.moving(
            head,
            disc.sized(Vec3::new(0.12, 0.012, 0.12))
                .inst(eye, flat, kind::METAL, lit),
        );
        lib.moving(
            head,
            disc.sized(Vec3::new(0.055, 0.012, 0.055)).inst(
                eye + face * 0.008,
                flat,
                kind::CRYSTAL,
                [1.0, 0.75, 0.25, 2.5],
            ),
        );
        let tuft = (Vec3::Y + across * (s * 0.45)).normalize();
        lib.moving(
            head,
            cone.sized(Vec3::new(0.05, 0.1, 0.035)).inst(
                crown + Vec3::Y * 0.11 + across * (s * 0.085),
                Quat::from_rotation_arc(Vec3::Y, tuft),
                kind::METAL,
                lit,
            ),
        );
    }
    let beak = (face * 0.6 - Vec3::Y * 0.8).normalize();
    lib.moving(
        head,
        cone.sized(Vec3::new(0.035, 0.06, 0.035)).inst(
            crown - Vec3::Y * 0.035 + face * 0.13,
            Quat::from_rotation_arc(Vec3::Y, beak),
            kind::METAL,
            steel,
        ),
    );
}

/// Beams across the ceiling, and trains of flat gears meshing under it, each wheel on
/// an arbor up into the ceiling.
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
    let y = h - 0.42;
    let (out, across) = (frame.out(), frame.across());
    // Down the middle between the pillars, and across the far end: each wheel set a
    // little to one side of the last and a hair above or below it, old brass and
    // polished in turn.
    let trains: [(Vec3, Vec3, Vec3, f32, &[usize]); 2] = [
        (frame.at(0.0, 22.0, y), out, across, 0.7, &[14, 8, 18]),
        (frame.at(-2.15, 29.1, y), across, out, 0.55, &[12, 8, 16, 8, 12]),
    ];
    for (start, along, aside, r, teeth) in trains {
        let mut p = wheel(r, teeth[0], 0.05).old().at(start, Vec3::Y);
        for (k, &n) in teeth.iter().enumerate() {
            if k > 0 {
                let (side, lift) = if k % 2 == 1 { (0.19, -0.03) } else { (-0.19, 0.03) };
                let next = wheel(0.0, n, 0.05);
                let next = if k % 2 == 0 { next.old() } else { next };
                p = p.mesh(along + aside * side, lift, next);
            }
            gear(lib, p);
            let arbor = Vec3::new(p.c.x, frame.y + h, p.c.z);
            lib.pole(p.c, arbor, 0.06, kind::METAL, rgba(STEEL, 1.0));
        }
    }
}
