//! The Music Room: department 9, top floor, east side. Laid out like a recital hall:
//! rows of cases across the room either side of a carpeted aisle, like pews facing
//! the organ in the middle of the far wall, cases flanking it and down the side
//! walls; a harp and a harpsichord before the organ, music stands, chimes and a
//! double bass; instruments hung on the walls, a chandelier over the aisle. The
//! harp's strings glow and a few notes drift in the air over it, as if it had just
//! been played; the chimes stir, and a metronome on the harpsichord keeps a slow
//! time.

use super::hearth_library::{candle, turned_leg};
use super::*;
use std::f32::consts::FRAC_PI_2;

/// Where the two rows of cases stand (outward), and where the aisle between their
/// ends begins either side of the middle.
const PEWS: [f32; 2] = [19.3, 23.4];
const AISLE: f32 = 2.7;
/// The organ's half-width: the far wall's cases start beside it.
const ORGAN: f32 = 2.0;
const PIPE_TIN: [f32; 3] = [0.6, 0.62, 0.66];
const LACQUER: [f32; 3] = [0.08, 0.13, 0.09];
const GILT: [f32; 3] = [0.7, 0.5, 0.18];
const EBONY: [f32; 4] = [0.04, 0.035, 0.03, 1.0];
/// The glow of the room's magic: the harp's strings and the notes over it.
const GLOW: [f32; 3] = [1.0, 0.82, 0.45];
/// Sheet music, on the stands and desks.
const SHEET: [f32; 4] = [0.88, 0.82, 0.66, 1.0];

pub(super) fn furnish(lib: &mut Library, frame: &RoomFrame, rng: &mut StdRng, titles: &mut HashSet<String>) {
    debug_assert!(frame.door.abs() < 1.0, "the Music Room wants its door in the middle");
    shelves(lib, frame, rng, titles);
    walls(lib, frame);
    organ(lib, frame);
    harp(lib, frame, -4.6, 27.2);
    harpsichord(lib, frame, 4.5, 26.4);
    for (a, o, turn) in [(-7.0, 26.9, 0.5), (-8.3, 27.8, 0.9), (7.4, 27.3, -0.6)] {
        music_stand(lib, frame, a, o, turn);
    }
    chimes(lib, frame, -6.6, 16.2);
    double_bass(lib, frame, 8.2, 16.1);
    ceiling(lib, frame);
}

/// A rotation turning y onto `dir` and z as near to `side` as it can.
fn aim(dir: Vec3, side: Vec3) -> Quat {
    let y = dir.normalize();
    let z = (side - y * side.dot(y)).normalize();
    Quat::from_mat3(&Mat3::from_cols(y.cross(z), y, z))
}

/// A sheet of music laid on a desk.
fn sheet(c: Vec3, rot: Quat, size: Vec3) -> Inst {
    Inst::new(c, rot, size, kind::PARCHMENT, SHEET, 4)
}

/// Two rows of three double-sided cases either side of the aisle, a category round
/// each (along its far side, round its end, back along its near side, so its sign
/// faces the door); four wall cases down each side wall, four either side of the
/// organ.
fn shelves(lib: &mut Library, frame: &RoomFrame, rng: &mut StdRng, titles: &mut HashSet<String>) {
    let (out, across) = (frame.out(), frame.across());
    let mut cats = frame.categories.clone();
    for &o in &PEWS {
        for s in [-1.0f32, 1.0] {
            let (mut far, mut near) = (Vec::new(), Vec::new());
            for k in 0..3 {
                let a = s * (AISLE + UNIT_PITCH * 0.5 + k as f32 * UNIT_PITCH);
                for f in lib.bookcase(frame.at(a, o, 0.0), !frame.out_along_x()) {
                    if f.1.dot(out) > 0.0 { far.push(f) } else { near.push(f) }
                }
            }
            let mut faces = left_to_right(far);
            faces.extend(left_to_right(near));
            lib.add_category(rng, cats.next().unwrap(), &faces, titles);
            pew_end(lib, frame, s, o);
        }
    }
    for s in [-1.0f32, 1.0] {
        let faces: Vec<_> = (0..4)
            .map(|k| {
                let o = RoomFrame::O_NEAR + 2.65 + k as f32 * UNIT_PITCH;
                lib.wall_bookcase(frame.at(s * RoomFrame::A_HALF, o, 0.0), -across * s)
            })
            .collect();
        lib.add_category(rng, cats.next().unwrap(), &left_to_right(faces), titles);
    }
    for s in [-1.0f32, 1.0] {
        let faces: Vec<_> = (0..4)
            .map(|k| {
                let a = s * (ORGAN + UNIT_PITCH * (k as f32 + 0.5));
                lib.wall_bookcase(frame.at(a, RoomFrame::O_FAR, 0.0), -out)
            })
            .collect();
        lib.add_category(rng, cats.next().unwrap(), &left_to_right(faces), titles);
    }
}

/// A panel on a row's end toward the aisle, a gilt lyre on it.
fn pew_end(lib: &mut Library, frame: &RoomFrame, s: f32, o: f32) {
    // The case's side faces the aisle AISLE + 0.05 from the middle.
    let (a0, a1) = (s * (AISLE + 0.02), s * (AISLE + 0.045));
    lib.decor(piece(
        frame,
        (a0.min(a1), a0.max(a1)),
        (o - 0.33, o + 0.33),
        (0.35, 2.75),
        kind::WOOD,
        [0.2, 0.1, 0.045],
    ));
    let at = |u: f32, v: f32| frame.at(s * (AISLE + 0.012), o + u, v);
    let gilt = rgba(GILT, 1.0);
    let line = |lib: &mut Library, p: (f32, f32), q: (f32, f32), w: f32| {
        rod(lib, at(p.0, p.1), at(q.0, q.1), w, w, kind::METAL, gilt);
    };
    line(lib, (-0.12, 1.45), (0.12, 1.45), 0.035);
    for m in [-1.0f32, 1.0] {
        line(lib, (m * 0.09, 1.45), (m * 0.2, 1.75), 0.03);
        line(lib, (m * 0.2, 1.75), (m * 0.13, 2.08), 0.03);
    }
    line(lib, (-0.17, 2.0), (0.17, 2.0), 0.025);
    for k in 0..4 {
        let u = -0.075 + k as f32 * 0.05;
        line(lib, (u, 1.47), (u, 1.99), 0.006);
    }
}

/// Panelling where no case stands, a moonlit window in each side wall past the
/// cases with instruments hung either side of it, and the aisle's carpet.
fn walls(lib: &mut Library, frame: &RoomFrame) {
    let side = frame.side;
    let (near, far) = (RoomFrame::O_NEAR, RoomFrame::O_FAR);
    let inner = RoomFrame::A_HALF;
    let d = frame.door;
    for (a0, a1) in [
        (-inner + 0.01, d - RoomFrame::DOOR_HALF - 0.32),
        (d + RoomFrame::DOOR_HALF + 0.32, inner - 0.01),
    ] {
        lib.wainscot(frame.y, frame.rect(a0, a1, near, near + 0.07), side.out);
    }
    let past = near + 2.65 + 3.5 * UNIT_PITCH + 0.05;
    let out = frame.out();
    for s in [-1.0f32, 1.0] {
        let wall = s * inner;
        let (a0, a1) = (wall - s * 0.07, wall);
        lib.wainscot(
            frame.y,
            frame.rect(a0.min(a1), a0.max(a1), past, far - 0.01),
            side.across * -s,
        );
        let n = -frame.across() * s;
        let o = 28.9;
        let wc = frame.at(wall - s * 0.02, o, 2.9);
        let mut glass = Inst::aabox(wc, frame.size(0.08, 3.0, 1.3), kind::GLASS, [0.32, 0.42, 0.85]);
        glass.color[3] = 1.2;
        lib.decor(glass);
        let (s0, s1) = (wall - s * 0.14, wall);
        let sill = piece(
            frame,
            (s0.min(s1), s0.max(s1)),
            (o - 0.78, o + 0.78),
            (1.3, 1.4),
            kind::STONE,
            [0.45, 0.42, 0.38],
        );
        lib.decor(sill);
        lib.light(wc + n * 1.5, 7.0, [0.45, 0.55, 1.0], 0.7, LightKind::Static);
        let beam = (n * 1.0 - Vec3::Y * 0.6 - out * 0.25).normalize();
        let len = 6.0;
        lib.static_trans.push(Inst::new(
            wc + beam * len * 0.5,
            Quat::from_rotation_arc(Vec3::Y, beam),
            Vec3::new(1.0, len, 2.0),
            kind::SHAFT,
            [0.35, 0.45, 0.9, 0.04],
            0,
        ));
        violin(lib, &Hung::new(frame.at(wall - s * 0.08, 27.4, 2.3), n, 1.2, 1.5));
        lute(lib, &Hung::new(frame.at(wall - s * 0.1, 30.4, 2.25), n, -0.9, 1.5));
        horn(lib, &Hung::new(frame.at(wall - s * 0.06, 27.3, 3.55), n, 0.3, 1.5));
    }
    // A carpet down the aisle from the door to the organ.
    let mut rug = piece(
        frame,
        (-1.3, 1.3),
        (near + 0.4, 30.3),
        (0.0, 0.02),
        kind::RUG,
        [0.32, 0.05, 0.07],
    );
    rug.param = 200 | (150 << 8) | (70 << 16);
    lib.rug(rug);
}

/// How a hung instrument sits on a wall facing `n`, turned `tilt` in the wall's
/// plane, and drawn `k` times its own size.
struct Hung {
    c: Vec3,
    q: Quat,
    k: f32,
}

impl Hung {
    fn new(c: Vec3, n: Vec3, tilt: f32, k: f32) -> Self {
        let q = Quat::from_mat3(&Mat3::from_cols(Vec3::Y.cross(n), Vec3::Y, n)) * Quat::from_rotation_z(tilt);
        Hung { c, q, k }
    }

    /// The point `at` in the instrument's own frame (x across it, y up its length, z
    /// out of the wall).
    fn at(&self, at: [f32; 3]) -> Vec3 {
        self.c + self.q * (Vec3::from(at) * self.k)
    }

    /// A box at `at`, `size`, turned `r` more in its plane.
    fn put(&self, lib: &mut Library, at: [f32; 3], size: [f32; 3], r: f32, kind: u32, col: [f32; 4]) {
        let rot = self.q * Quat::from_rotation_z(r);
        lib.decor(Inst::new(self.at(at), rot, Vec3::from(size) * self.k, kind, col, 0));
    }

    /// `shape` with the origin it was made about at `at`, turned by `rot` in the
    /// instrument's frame, at the instrument's size.
    fn shaped(&self, lib: &mut Library, shape: Shape, at: [f32; 3], rot: Quat, kind: u32, col: [f32; 4]) {
        let i = shape.scaled(self.k).at(self.at(at), self.q * rot, kind, col);
        lib.decor(i);
    }
}

/// The outline of a fiddle's body (a violin's, a bass's), `len` long and `w` wide,
/// its foot at the origin and its neck up y: round bouts either side of a waist.
fn fiddle(len: f32, w: f32) -> Vec<Vec2> {
    let right = [
        (0.2, 0.012),
        (0.36, 0.045),
        (0.46, 0.1),
        (0.5, 0.18),
        (0.49, 0.27),
        (0.43, 0.36),
        (0.33, 0.42),
        (0.28, 0.47),
        (0.28, 0.53),
        (0.32, 0.58),
        (0.38, 0.64),
        (0.41, 0.72),
        (0.39, 0.8),
        (0.33, 0.88),
        (0.23, 0.95),
        (0.11, 0.99),
    ];
    let mut pts = vec![Vec2::ZERO];
    pts.extend(right.iter().map(|&(x, y)| Vec2::new(x * w, y * len)));
    pts.push(Vec2::new(0.0, len));
    pts.extend(right.iter().rev().map(|&(x, y)| Vec2::new(-x * w, y * len)));
    pts
}

/// A fiddle's scroll, `r` round: a curl tapering as it winds in, made in the yz plane
/// from its root at the origin, forward (+z) and up over the top.
fn scroll(lib: &mut Library, r: f32) -> Shape {
    let path: Vec<Vec3> = (0..=28)
        .map(|k| {
            let t = k as f32 / 28.0;
            let a = t * TAU * 1.4;
            let rr = r * (1.0 - 0.7 * t);
            Vec3::new(0.0, r - rr * libm::cosf(a), rr * libm::sinf(a))
        })
        .collect();
    lib.shape(Mesh::sweep(&path, &[r * 0.5, r * 0.18], &circle(1.0, 8), true))
}

/// A violin hung aslant: a waisted body, a neck and scroll, its strings.
fn violin(lib: &mut Library, h: &Hung) {
    let body = rgba([0.45, 0.17, 0.05], 1.0);
    let shell = lib.shape(Mesh::prism(&fiddle(0.36, 0.21), 0.05));
    h.shaped(lib, shell, [0.0, -0.2, 0.0], Quat::IDENTITY, kind::WOOD, body);
    h.put(lib, [0.0, 0.25, 0.03], [0.035, 0.3, 0.012], 0.0, kind::WOOD, EBONY);
    h.put(
        lib,
        [0.0, 0.33, 0.015],
        [0.03, 0.32, 0.03],
        0.0,
        kind::WOOD,
        rgba(DARK_WOOD, 1.0),
    );
    h.put(lib, [0.0, 0.5, 0.012], [0.03, 0.08, 0.035], 0.0, kind::WOOD, body);
    let curl = scroll(lib, 0.022);
    h.shaped(lib, curl, [0.0, 0.535, 0.012], Quat::IDENTITY, kind::WOOD, body);
    h.put(lib, [0.0, -0.08, 0.03], [0.07, 0.012, 0.02], 0.0, kind::WOOD, body);
    for s in [-1.0f32, 1.0] {
        h.put(
            lib,
            [s * 0.04, -0.05, 0.026],
            [0.006, 0.06, 0.004],
            s * 0.15,
            kind::PLAIN,
            EBONY,
        );
    }
    h.put(lib, [0.0, -0.15, 0.03], [0.04, 0.09, 0.008], 0.0, kind::WOOD, EBONY);
    h.put(
        lib,
        [0.0, 0.1, 0.034],
        [0.02, 0.48, 0.002],
        0.0,
        kind::PLAIN,
        [0.75, 0.72, 0.65, 1.0],
    );
}

/// A lute: a pear-shaped bowl behind a flat soundboard with a rose, a short neck and
/// a bent-back pegbox.
fn lute(lib: &mut Library, h: &Hung) {
    let wood = rgba([0.5, 0.3, 0.12], 1.0);
    let pear = [
        (0.0, 0.0),
        (0.1, 0.01),
        (0.145, 0.05),
        (0.16, 0.12),
        (0.15, 0.2),
        (0.12, 0.29),
        (0.08, 0.37),
        (0.04, 0.43),
        (0.0, 0.46),
    ];
    // The bowl: the pear turned, flattened, wholly behind the soundboard.
    let bowl = lib.turned(&pear, 24);
    h.shaped(
        lib,
        bowl.sized(Vec3::new(0.32, 0.46, 0.17)),
        [0.0, -0.23, -0.06],
        Quat::IDENTITY,
        kind::WOOD,
        wood,
    );
    let mut face: Vec<Vec2> = pear.iter().map(|&(r, y)| Vec2::new(r, y)).collect();
    face.extend(pear[1..pear.len() - 1].iter().rev().map(|&(r, y)| Vec2::new(-r, y)));
    let board = lib.shape(Mesh::prism(&face, 0.01));
    h.shaped(
        lib,
        board,
        [0.0, -0.23, 0.026],
        Quat::IDENTITY,
        kind::WOOD,
        [0.72, 0.55, 0.32, 1.0],
    );
    let rose = lib.shape(Mesh::cylinder(16));
    let flat = Quat::from_rotation_x(FRAC_PI_2);
    h.shaped(
        lib,
        rose.sized(Vec3::new(0.065, 0.004, 0.065)),
        [0.0, 0.0, 0.031],
        flat,
        kind::PLAIN,
        [0.04, 0.03, 0.02, 1.0],
    );
    let ring = lib.shape(Mesh::torus(0.12, 24, 6));
    h.shaped(
        lib,
        ring.sized(Vec3::new(0.075, 0.006, 0.075)),
        [0.0, 0.0, 0.032],
        flat,
        kind::METAL,
        rgba(GILT, 1.0),
    );
    h.put(lib, [0.0, -0.17, 0.034], [0.07, 0.015, 0.012], 0.0, kind::WOOD, EBONY);
    h.put(
        lib,
        [0.0, 0.3, 0.02],
        [0.05, 0.3, 0.03],
        0.0,
        kind::WOOD,
        rgba(DARK_WOOD, 1.0),
    );
    h.put(
        lib,
        [0.03, 0.5, 0.0],
        [0.05, 0.14, 0.03],
        0.5,
        kind::WOOD,
        rgba(DARK_WOOD, 1.0),
    );
}

/// A horn: a coiled brass tube, a flaring bell and a mouthpipe.
fn horn(lib: &mut Library, h: &Hung) {
    let brass = rgba(GILT, 1.0);
    let coil = lib.shape(Mesh::torus(0.2, 40, 10));
    let flat = Quat::from_rotation_x(FRAC_PI_2);
    for (r, z) in [(0.17f32, 0.0f32), (0.13, 0.025)] {
        h.shaped(
            lib,
            coil.sized(Vec3::new(2.2 * r, 0.034, 2.2 * r)),
            [0.0, 0.0, z],
            flat,
            kind::METAL,
            brass,
        );
    }
    let bell = lib.turned_open(
        &[
            (0.02, 0.0),
            (0.022, 0.08),
            (0.03, 0.15),
            (0.05, 0.21),
            (0.08, 0.25),
            (0.12, 0.27),
        ],
        24,
    );
    let dir = Vec3::new(0.62, -0.62, 0.48).normalize();
    h.shaped(
        lib,
        bell,
        [0.12, -0.1, 0.02],
        Quat::from_rotation_arc(Vec3::Y, dir),
        kind::METAL,
        brass,
    );
    let (p, q) = (h.at([-0.1, 0.13, 0.02]), h.at([-0.2, 0.28, 0.04]));
    lib.pole(p, q, 0.02 * h.k, kind::METAL, brass);
}

/// The organ in the middle of the far wall: a case of pipes rising nearly to the
/// ceiling over two keyboards, fretted shades over the pipes, a music desk with
/// candles, a bench before it.
fn organ(lib: &mut Library, frame: &RoomFrame) {
    let wall = RoomFrame::O_FAR;
    let h = RoomFrame::HEIGHT;
    let w = ORGAN - 0.1;
    // The base, shallow enough to stand behind (where the far views are taken from).
    lib.solid(
        piece(frame, (-w, w), (wall - 0.28, wall), (0.0, 1.05), kind::WOOD, WALNUT),
        Stops::ALL,
    );
    lib.decor(piece(
        frame,
        (-w - 0.03, w + 0.03),
        (wall - 0.31, wall),
        (1.05, 1.13),
        kind::WOOD,
        [0.12, 0.06, 0.028],
    ));
    lib.decor(piece(
        frame,
        (-w + 0.15, w - 0.15),
        (wall - 0.03, wall),
        (1.13, h - 0.35),
        kind::WOOD,
        DARK_WOOD,
    ));
    for s in [-1.0f32, 1.0] {
        let x = s * (w - 0.1);
        lib.decor(piece(
            frame,
            (x - 0.12, x + 0.12),
            (wall - 0.3, wall),
            (1.13, h - 0.3),
            kind::WOOD,
            WALNUT,
        ));
    }
    lib.decor(piece(
        frame,
        (-w - 0.05, w + 0.05),
        (wall - 0.36, wall),
        (h - 0.3, h - 0.12),
        kind::WOOD,
        [0.12, 0.06, 0.028],
    ));
    // Pipes in three flats: a middle one falling away from its tallest, and towers
    // either side rising outward. Each stands on a cone of a foot; its mouth is a dark
    // slot under a pointed lip.
    let body = lib.turned(&[(0.0, 0.0), (0.12, 0.0), (0.5, 0.09), (0.5, 1.0), (0.0, 1.0)], 16);
    let lip = lib.shape(Mesh::prism(
        &[Vec2::new(-0.5, 0.0), Vec2::new(0.5, 0.0), Vec2::new(0.0, 1.0)],
        0.01,
    ));
    let front = wall - 0.22;
    let tin = rgba(PIPE_TIN, 1.0);
    let mut pipe = |a: f32, top: f32, wide: f32| {
        let foot = frame.at(a, front + wide, 1.13);
        lib.decor(
            body.sized(Vec3::new(2.0 * wide, top - 1.13, 2.0 * wide))
                .at(foot, Quat::IDENTITY, kind::METAL, tin),
        );
        let mouth = piece(
            frame,
            (a - wide * 0.4, a + wide * 0.4),
            (front - wide * 0.03, front + wide * 0.3),
            (1.42, 1.5),
            kind::PLAIN,
            [0.02, 0.02, 0.02],
        );
        lib.decor(mouth);
        lib.decor(lip.sized(Vec3::new(wide * 0.8, wide * 0.9, 0.006)).inst(
            frame.at(a, front - wide * 0.03, 1.5 + wide * 0.45),
            frame.turn(),
            kind::METAL,
            [0.75, 0.76, 0.8, 1.0],
        ));
    };
    let middle = |a: f32| 4.2 - 0.18 * libm::powf((a / 0.16).abs(), 1.3);
    for k in -3i32..=3 {
        let a = k as f32 * 0.16;
        pipe(a, middle(a), 0.06);
    }
    let tower = |k: f32| 3.75 + k * 0.38;
    for s in [-1.0f32, 1.0] {
        for k in 0..4 {
            pipe(s * (0.75 + k as f32 * 0.2), tower(k as f32), 0.075);
        }
    }
    // The shades: gilt fretwork filling the case above the pipes, its lower edge
    // following their tops.
    let mut edge: Vec<Vec2> = vec![Vec2::new(-1.66, 5.05), Vec2::new(-1.45, 5.05)];
    for k in (0..4).rev() {
        let a = -(0.75 + k as f32 * 0.2);
        edge.push(Vec2::new(a, tower(k as f32) + 0.1));
    }
    edge.push(Vec2::new(-0.62, tower(0.0) + 0.1));
    for k in -3i32..=3 {
        let a = k as f32 * 0.16;
        if k == -3 {
            edge.push(Vec2::new(-0.58, middle(a) + 0.08));
        }
        edge.push(Vec2::new(a, middle(a) + 0.08));
    }
    // And the same again on the right, mirrored.
    let left: Vec<Vec2> = edge.iter().filter(|p| p.x < -0.5).copied().collect();
    edge.extend(left.iter().rev().map(|p| Vec2::new(-p.x, p.y)));
    edge.push(Vec2::new(1.66, h - 0.3));
    edge.push(Vec2::new(-1.66, h - 0.3));
    let shades = lib.shape(Mesh::prism(&edge, 0.02));
    lib.decor(Inst {
        param: 2,
        ..shades.at(
            frame.at(0.0, front - 0.02, 0.0),
            frame.turn(),
            kind::FRET,
            rgba(GILT, 1.0),
        )
    });
    // The keyboards: a shelf with white keys and their black ones, a second above
    // and behind it; stops either side; the music desk.
    for (k, (o0, y)) in [(wall - 0.62, 0.78), (wall - 0.46, 0.92)].into_iter().enumerate() {
        let (o1, kw) = (o0 + 0.24, 0.85 - k as f32 * 0.05);
        lib.decor(piece(
            frame,
            (-kw - 0.08, kw + 0.08),
            (o0, wall - 0.28),
            (y - 0.05, y),
            kind::WOOD,
            [0.12, 0.06, 0.028],
        ));
        lib.decor(piece(
            frame,
            (-kw, kw),
            (o0 + 0.01, o1 - 0.06),
            (y, y + 0.02),
            kind::PAPER,
            [0.0; 3],
        ));
        lib.decor(piece(
            frame,
            (-kw, kw),
            (o0 + 0.1, o1 - 0.06),
            (y + 0.02, y + 0.035),
            kind::PLAIN,
            [0.03, 0.03, 0.03],
        ));
    }
    // The stop jambs either side of the keyboards, turned knobs in them, the middle
    // row red.
    let knob = lib.turned(
        &[(0.0, 0.0), (0.2, 0.0), (0.2, 0.6), (0.5, 0.7), (0.5, 0.95), (0.0, 1.0)],
        12,
    );
    let jamb = wall - 0.3;
    for s in [-1.0f32, 1.0] {
        let (a0, a1) = (s * 0.93, s * 1.23);
        lib.decor(piece(
            frame,
            (a0.min(a1), a0.max(a1)),
            (jamb, wall - 0.24),
            (0.92, 1.33),
            kind::WOOD,
            [0.12, 0.06, 0.028],
        ));
        for (k, y) in [1.02f32, 1.12, 1.22].into_iter().enumerate() {
            let col = if k == 1 { [0.6, 0.1, 0.08] } else { [0.85, 0.82, 0.72] };
            for a in [1.02f32, 1.14] {
                lib.decor(knob.sized(Vec3::new(0.05, 0.06, 0.05)).at(
                    frame.at(s * a, jamb, y),
                    Quat::from_rotation_arc(Vec3::Y, -frame.out()),
                    kind::PLAIN,
                    rgba(col, 1.0),
                ));
            }
        }
    }
    let desk = Quat::from_rotation_x(-0.25);
    let q = frame.turn() * desk;
    lib.decor(Inst::new(
        frame.at(0.0, wall - 0.36, 1.32),
        q,
        Vec3::new(0.9, 0.42, 0.03),
        kind::WOOD,
        rgba(WALNUT, 1.0),
        0,
    ));
    let stick = lib.turned(
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
    );
    for s in [-1.0f32, 1.0] {
        let page = frame.at(s * 0.2, wall - 0.38, 1.34);
        lib.decor(sheet(page, q, Vec3::new(0.36, 0.34, 0.02)));
        // A candle each side, on the case beside the towers.
        let base = frame.at(s * 1.55, wall - 0.18, 1.13);
        lib.decor(
            stick
                .sized(Vec3::new(0.1, 0.12, 0.1))
                .at(base, Quat::IDENTITY, kind::METAL, rgba(GILT, 1.0)),
        );
        candle(lib, base + Vec3::Y * 0.12, 0.18);
    }
    lib.light(
        frame.at(0.0, wall - 0.9, 1.6),
        6.0,
        [1.0, 0.62, 0.3],
        0.9,
        LightKind::Flicker,
    );
    // The bench.
    let (o0, o1) = (wall - 1.25, wall - 0.92);
    lib.decor(piece(frame, (-0.8, 0.8), (o0, o1), (0.46, 0.52), kind::WOOD, WALNUT));
    let leg = turned_leg(lib);
    for s in [-1.0f32, 1.0] {
        for d_o in [0.06, (o1 - o0) - 0.06] {
            lib.decor(leg.sized(Vec3::new(0.06, 0.46, 0.06)).at(
                frame.at(s * 0.72, o0 + d_o, 0.0),
                Quat::IDENTITY,
                kind::WOOD,
                rgba(DARK_WOOD, 1.0),
            ));
        }
    }
    guard(lib, frame, (-0.8, 0.8), (o0, o1), 0.52);
}

/// A harp standing at `a`, `o` (its foot's front), its strings facing across the
/// room: a turned gilt column, a soundbox tapering up from the foot, a neck curving
/// between them, and strings that glow; notes drifting over it.
fn harp(lib: &mut Library, frame: &RoomFrame, a: f32, o: f32) {
    let (out, across) = (frame.out(), frame.across());
    // Points in the harp's plane: `u` outward from its foot's front, `v` up.
    let p = |u: f32, v: f32| frame.at(a, o + u, v);
    let gilt = rgba(GILT, 1.0);
    let wood = rgba([0.3, 0.14, 0.05], 1.0);
    lib.decor(piece(
        frame,
        (a - 0.2, a + 0.2),
        (o - 0.05, o + 0.75),
        (0.0, 0.12),
        kind::WOOD,
        [0.3, 0.14, 0.05],
    ));
    let column = lib.turned(
        &[
            (0.0, 0.0),
            (0.5, 0.0),
            (0.5, 0.04),
            (0.36, 0.07),
            (0.3, 0.12),
            (0.27, 0.8),
            (0.3, 0.85),
            (0.42, 0.89),
            (0.5, 0.93),
            (0.42, 0.97),
            (0.3, 1.0),
            (0.0, 1.0),
        ],
        16,
    );
    let (c0, c1) = (p(0.0, 0.12), p(-0.04, 1.9));
    lib.decor(
        column
            .sized(Vec3::new(0.11, (c1 - c0).length(), 0.11))
            .at(c0, aim(c1 - c0, across), kind::METAL, gilt),
    );
    // The soundbox, round-backed, broad at the foot and narrow at the top.
    let (s0, s1) = (p(0.08, 0.1), p(0.74, 1.52));
    let sb = lib.turned(&[(0.0, 0.0), (0.5, 0.0), (0.2, 1.0), (0.0, 1.0)], 24);
    lib.decor(
        sb.sized(Vec3::new(0.18, (s1 - s0).length(), 0.26))
            .at(s0, aim(s1 - s0, across), kind::WOOD, wood),
    );
    // The neck, falling from the column's head to the soundbox's in an S, made in
    // the harp's plane (x outward, y up) and slimmer across it.
    let neck = |u: f32| {
        let s = (u + 0.04) / 0.82;
        1.9 - 0.36 * s + 0.06 * libm::sinf(s * TAU)
    };
    let path: Vec<Vec3> = (0..=16)
        .map(|k| {
            let u = -0.04 + 0.82 * k as f32 / 16.0;
            Vec3::new(u, neck(u), 0.0)
        })
        .collect();
    let neck_shape = lib.shape(Mesh::sweep(&path, &[0.05, 0.04, 0.035], &circle(1.0, 10), true));
    let plane = Quat::from_mat3(&Mat3::from_cols(out, Vec3::Y, out.cross(Vec3::Y)));
    lib.decor(
        neck_shape
            .sized(neck_shape.size * Vec3::new(1.0, 1.0, 0.6))
            .at(p(0.0, 0.0), plane, kind::WOOD, wood),
    );
    // A gilt crown on the column's head.
    let crown = lib.turned(
        &[
            (0.0, 0.0),
            (0.5, 0.0),
            (0.42, 0.3),
            (0.2, 0.55),
            (0.25, 0.8),
            (0.0, 1.0),
        ],
        12,
    );
    lib.decor(
        crown
            .sized(Vec3::new(0.1, 0.12, 0.1))
            .at(c1 + Vec3::Y * 0.02, Quat::IDENTITY, kind::METAL, gilt),
    );
    // The strings, from the soundboard up to the neck.
    let board = |u: f32| {
        let f = (u - 0.08) / 0.66;
        // Its middle line, out to its face (a vertical meets the slanting face this
        // far above the middle line, for the box's depth there).
        0.1 + 1.42 * f + 2.38 * (0.09 - 0.054 * f) - 0.02
    };
    for k in 0..11 {
        let u = 0.1 + k as f32 * 0.055;
        let (bottom, top) = (board(u), neck(u) - 0.035);
        if top - bottom > 0.05 {
            let i = lib.pole_inst(p(u, bottom), p(u, top), 0.007, kind::CRYSTAL, rgba(GLOW, 0.9));
            lib.decor(i);
        }
    }
    guard(lib, frame, (a - 0.22, a + 0.22), (o - 0.1, o + 0.8), 1.9);
    // Notes drifting in the air over it, as if the strings had just been played:
    // the lot wandering slowly to and fro, each rising and falling in its own time.
    let wander = Motion::Swing {
        axis: Vec3::Y,
        amp: 0.3,
        period: 23.0,
    };
    let drift = lib.mover(p(0.35, 2.4), wander, 0.0, None);
    let head = lib.shape(Mesh::sphere(16));
    let flag = lib.shape(Mesh::sweep(
        &[
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(0.025, -0.02, 0.0),
            Vec3::new(0.05, -0.05, 0.0),
            Vec3::new(0.055, -0.09, 0.0),
            Vec3::new(0.045, -0.12, 0.0),
        ],
        &[0.008, 0.005],
        &circle(1.0, 6),
        true,
    ));
    let n = across * if a < 0.0 { 1.0 } else { -1.0 };
    let face = Quat::from_mat3(&Mat3::from_cols(out, Vec3::Y, out.cross(Vec3::Y)));
    let notes = [
        (-0.25, 2.35, 0.0),
        (0.15, 2.6, 2.1),
        (0.55, 2.45, 4.3),
        (0.95, 2.75, 1.2),
    ];
    for (k, (du, v, phase)) in notes.into_iter().enumerate() {
        let c = p(du, v) + n * 0.1;
        let bob = Motion::Bob {
            amp: 0.05,
            period: 6.0 + k as f32 * 0.9,
            rock: 0.12,
        };
        let m = lib.mover(c, bob, phase, Some(drift));
        let glow = rgba(GLOW, 1.6);
        lib.moving(
            m,
            head.sized(Vec3::new(0.085, 0.06, 0.03))
                .inst(c, face * Quat::from_rotation_z(0.35), kind::CRYSTAL, glow),
        );
        let stem = (c + out * 0.038 + Vec3::Y * 0.01, c + out * 0.038 + Vec3::Y * 0.21);
        let i = lib.pole_inst(stem.0, stem.1, 0.01, kind::CRYSTAL, glow);
        lib.moving(m, i);
        if k % 2 == 1 {
            lib.moving(m, flag.at(stem.1, face, kind::CRYSTAL, glow));
        }
    }
    lib.light(p(0.4, 2.2), 3.0, GLOW, 0.4, LightKind::Static);
}

/// A harpsichord with its keyboard toward the door at `o`: its case running out to
/// a narrow tail along a curving bentside, its lid propped open, on turned legs; a
/// metronome on it keeping a slow time, a bench before it.
fn harpsichord(lib: &mut Library, frame: &RoomFrame, a: f32, o: f32) {
    let (len, wide, tail) = (2.2, 0.48, 0.22);
    let (y0, y1) = (0.72, 0.98);
    // Outlines in the instrument's plan: x across (the straight spine at +wide), y
    // outward from the keyboard's end.
    let bent = |inset: f32| -> Vec<Vec2> {
        let (p0, p1, p2, p3) = (
            Vec2::new(-wide + inset, 0.9),
            Vec2::new(-wide + inset, 1.5),
            Vec2::new(-0.1 + inset, 1.6 - inset),
            Vec2::new(wide - 2.0 * tail + inset * 0.5, len - inset),
        );
        (0..=12)
            .map(|k| {
                let t = k as f32 / 12.0;
                let u = 1.0 - t;
                p0 * (u * u * u) + p1 * (3.0 * u * u * t) + p2 * (3.0 * u * t * t) + p3 * (t * t * t)
            })
            .collect()
    };
    let plan = |inset: f32, from: f32| -> Vec<Vec2> {
        let mut pts = vec![
            Vec2::new(-wide + inset, from),
            Vec2::new(wide - inset, from),
            Vec2::new(wide - inset, len - inset),
        ];
        pts.extend(bent(inset).into_iter().rev());
        pts
    };
    // Laid out in the plan's plane: x across, y outward, z up.
    let flat = Quat::from_mat3(&Mat3::from_cols(frame.across(), frame.out(), Vec3::Y));
    let case = lib.shape(Mesh::prism(&plan(0.0, 0.0), y1 - y0));
    lib.decor(case.at(frame.at(a, o, (y0 + y1) * 0.5), flat, kind::WOOD, rgba(LACQUER, 1.0)));
    let board = lib.shape(Mesh::prism(&plan(0.03, 0.25), 0.006));
    lib.decor(board.at(
        frame.at(a, o, y1 + 0.003),
        flat,
        kind::PARCHMENT,
        [0.75, 0.62, 0.4, 1.0],
    ));
    // A gilt band along its front.
    lib.decor(piece(
        frame,
        (a - wide - 0.01, a + wide + 0.01),
        (o - 0.01, o + 0.03),
        (y1 - 0.04, y1 + 0.01),
        kind::METAL,
        GILT,
    ));
    // The keyboard, standing out from the front between cheeks.
    let (k0, k1) = (o - 0.24, o - 0.01);
    lib.decor(piece(
        frame,
        (a - 0.42, a + 0.42),
        (k0, k1),
        (y0 + 0.1, y0 + 0.12),
        kind::PAPER,
        [0.0; 3],
    ));
    lib.decor(piece(
        frame,
        (a - 0.42, a + 0.42),
        (k0 + 0.1, k1),
        (y0 + 0.12, y0 + 0.135),
        kind::PLAIN,
        [0.03, 0.03, 0.03],
    ));
    lib.decor(piece(
        frame,
        (a - 0.42, a + 0.42),
        (k0, k1),
        (y0 + 0.02, y0 + 0.1),
        kind::WOOD,
        LACQUER,
    ));
    for x in [a - 0.46, a + 0.46] {
        lib.decor(piece(
            frame,
            (x - 0.04, x + 0.04),
            (k0 - 0.01, k1),
            (y0 + 0.02, y0 + 0.19),
            kind::WOOD,
            LACQUER,
        ));
    }
    // The lid, hinged along the spine and propped open toward the aisle, its painted
    // inside to the room: made about the hinge, then lifted about it.
    let hinged = |pts: Vec<Vec2>| -> Vec<Vec2> { pts.into_iter().map(|p| p - Vec2::new(wide, 0.0)).collect() };
    let lift = flat * Quat::from_rotation_y(0.8);
    let hinge = frame.at(a + wide, o, y1 + 0.0125);
    let lid = lib.shape(Mesh::prism(&hinged(plan(0.0, 0.25)), 0.025));
    lib.decor(lid.at(hinge, lift, kind::WOOD, rgba(LACQUER, 1.0)));
    let painted = lib.shape(Mesh::prism(&hinged(plan(0.04, 0.29)), 0.004));
    lib.decor(painted.at(
        hinge + lift * Vec3::new(0.0, 0.0, -0.0145),
        lift,
        kind::PARCHMENT,
        [0.62, 0.7, 0.55, 1.0],
    ));
    let under = hinge + lift * Vec3::new(-0.72, 1.05, -0.0125);
    lib.pole(
        frame.at(a - wide + 0.22, o + 1.05, y1),
        under,
        0.02,
        kind::WOOD,
        rgba(DARK_WOOD, 1.0),
    );
    // Turned legs, a music sheet on the desk, and the bench.
    let leg = turned_leg(lib);
    for (da, d_o) in [
        (-wide + 0.06, 0.06),
        (wide - 0.06, 0.06),
        (wide - 0.06, len - 0.1),
        (-wide + 0.3, 1.6),
    ] {
        lib.decor(leg.sized(Vec3::new(0.08, y0, 0.08)).at(
            frame.at(a + da, o + d_o, 0.0),
            Quat::IDENTITY,
            kind::WOOD,
            [0.05, 0.08, 0.06, 1.0],
        ));
    }
    let desk = frame.turn() * Quat::from_rotation_x(-0.3);
    let page = frame.at(a, o + 0.3, y1 + 0.17);
    lib.decor(sheet(page, desk, Vec3::new(0.42, 0.3, 0.01)));
    lib.decor(Inst::new(
        page + desk * Vec3::new(0.0, -0.01, -0.012),
        desk,
        Vec3::new(0.48, 0.34, 0.015),
        kind::WOOD,
        rgba(LACQUER, 1.0),
        0,
    ));
    guard(lib, frame, (a - wide, a + wide), (o - 0.25, o + len), y1);
    metronome(lib, frame, frame.at(a - wide + 0.12, o + 0.12, y1));
    let (b0, b1) = (o - 0.72, o - 0.4);
    lib.decor(piece(
        frame,
        (a - 0.45, a + 0.45),
        (b0, b1),
        (0.44, 0.5),
        kind::WOOD,
        WALNUT,
    ));
    for s in [-1.0f32, 1.0] {
        for d_o in [0.05, (b1 - b0) - 0.05] {
            lib.decor(leg.sized(Vec3::new(0.06, 0.44, 0.06)).at(
                frame.at(a + s * 0.38, b0 + d_o, 0.0),
                Quat::IDENTITY,
                kind::WOOD,
                rgba(DARK_WOOD, 1.0),
            ));
        }
    }
    guard(lib, frame, (a - 0.45, a + 0.45), (b0, b1), 0.5);
}

/// A metronome standing at `p`, facing the door: a tapering case, its pendulum
/// rocking to a slow beat.
fn metronome(lib: &mut Library, frame: &RoomFrame, p: Vec3) {
    let n = -frame.out();
    let case = lib.shape(Mesh::prism(
        &[
            Vec2::new(-0.045, 0.0),
            Vec2::new(0.045, 0.0),
            Vec2::new(0.02, 0.2),
            Vec2::new(-0.02, 0.2),
        ],
        0.07,
    ));
    lib.decor(case.at(p, frame.turn(), kind::WOOD, rgba(DARK_WOOD, 1.0)));
    // The pendulum, pivoted low on the case's face, its weight up the rod.
    let pivot = p + Vec3::Y * 0.035 + n * 0.037;
    let beat = Motion::Swing {
        axis: n,
        amp: 0.32,
        period: 2.4,
    };
    let m = lib.mover(pivot, beat, 0.0, None);
    let i = lib.pole_inst(
        pivot,
        pivot + Vec3::Y * 0.17,
        0.005,
        kind::METAL,
        [0.75, 0.75, 0.78, 1.0],
    );
    lib.moving(m, i);
    lib.moving(
        m,
        Inst::new(
            pivot + Vec3::Y * 0.12,
            frame.turn(),
            Vec3::new(0.022, 0.02, 0.012),
            kind::METAL,
            rgba(GILT, 1.0),
            0,
        ),
    );
}

/// A music stand at `a`, `o` (on the floor), its desk turned `turn_by` from facing
/// the door, a sheet of music on it.
fn music_stand(lib: &mut Library, frame: &RoomFrame, a: f32, o: f32, turn_by: f32) {
    let base = frame.at(a, o, 0.0);
    let q = frame.turn() * Quat::from_rotation_y(turn_by);
    let iron = [0.08, 0.08, 0.09, 1.0];
    for k in 0..3 {
        let leg = frame.turn() * Quat::from_rotation_y(k as f32 * TAU / 3.0 + turn_by);
        let tip = base + leg * Vec3::new(0.0, 0.0, 0.22);
        lib.pole(base + Vec3::Y * 0.3, tip, 0.018, kind::METAL, iron);
    }
    lib.pole(base + Vec3::Y * 0.28, base + Vec3::Y * 1.1, 0.022, kind::METAL, iron);
    let tilt = q * Quat::from_rotation_x(-0.35);
    let desk = base + Vec3::Y * 1.18;
    lib.decor(Inst::new(desk, tilt, Vec3::new(0.5, 0.36, 0.02), kind::METAL, iron, 0));
    let page = desk + tilt * Vec3::new(0.0, 0.02, 0.015);
    lib.decor(sheet(page, tilt, Vec3::new(0.44, 0.3, 0.01)));
    guard(lib, frame, (a - 0.24, a + 0.24), (o - 0.24, o + 0.24), 1.35);
}

/// Tubular chimes hung in a frame at `a`, `o`, longest to shortest, each stirring on
/// its cord in its own time.
fn chimes(lib: &mut Library, frame: &RoomFrame, a: f32, o: f32) {
    let (w, top) = (1.0, 2.05);
    let ball = lib.shape(Mesh::sphere(12));
    for s in [-1.0f32, 1.0] {
        let x = a + s * w;
        lib.decor(piece(
            frame,
            (x - 0.05, x + 0.05),
            (o - 0.05, o + 0.05),
            (0.0, top),
            kind::WOOD,
            WALNUT,
        ));
        lib.decor(piece(
            frame,
            (x - 0.08, x + 0.08),
            (o - 0.25, o + 0.25),
            (0.0, 0.08),
            kind::WOOD,
            WALNUT,
        ));
        lib.decor(ball.sized(Vec3::splat(0.11)).inst(
            frame.at(x, o, top + 0.15),
            Quat::IDENTITY,
            kind::WOOD,
            rgba(WALNUT, 1.0),
        ));
    }
    lib.decor(piece(
        frame,
        (a - w - 0.08, a + w + 0.08),
        (o - 0.06, o + 0.06),
        (top, top + 0.1),
        kind::WOOD,
        WALNUT,
    ));
    let tube = lib.shape(Mesh::cylinder(12));
    for k in 0..8 {
        let x = a - w + 0.2 + k as f32 * 0.228;
        let len = 1.45 - k as f32 * 0.1;
        let hang = frame.at(x, o, top);
        // Slower than a pendulum that long, and only a little way.
        let period = 1.6 * TAU * libm::sqrtf((0.05 + len * 0.66) / 9.81);
        let stir = Motion::Swing {
            axis: frame.across(),
            amp: 0.03,
            period,
        };
        let m = lib.mover(hang, stir, k as f32 * 0.37, None);
        let cord = lib.pole_inst(hang, hang - Vec3::Y * 0.06, 0.006, kind::PLAIN, [0.1, 0.08, 0.06, 1.0]);
        lib.moving(m, cord);
        lib.moving(
            m,
            tube.sized(Vec3::new(0.064, len, 0.064)).inst(
                hang - Vec3::Y * (0.05 + len * 0.5),
                Quat::IDENTITY,
                kind::METAL,
                rgba(GILT, 1.0),
            ),
        );
    }
    guard(lib, frame, (a - w - 0.1, a + w + 0.1), (o - 0.26, o + 0.26), top + 0.1);
}

/// A double bass leaning in its stand at `a`, `o`.
fn double_bass(lib: &mut Library, frame: &RoomFrame, a: f32, o: f32) {
    let q = frame.turn() * Quat::from_rotation_x(0.12);
    let base = frame.at(a, o, 0.0);
    let at = |x: f32, y: f32, z: f32| base + q * Vec3::new(x, y, z);
    let body = rgba([0.4, 0.15, 0.05], 1.0);
    let shell = lib.shape(Mesh::prism(&fiddle(1.08, 0.66), 0.2));
    lib.decor(shell.at(at(0.0, 0.24, 0.0), q, kind::WOOD, body));
    lib.decor(Inst::new(
        at(0.0, 1.5, 0.06),
        q,
        Vec3::new(0.07, 0.5, 0.05),
        kind::WOOD,
        rgba(DARK_WOOD, 1.0),
        0,
    ));
    lib.decor(Inst::new(
        at(0.0, 1.15, 0.115),
        q,
        Vec3::new(0.07, 0.85, 0.02),
        kind::WOOD,
        EBONY,
        0,
    ));
    lib.decor(Inst::new(
        at(0.0, 1.82, 0.03),
        q,
        Vec3::new(0.07, 0.16, 0.08),
        kind::WOOD,
        body,
        0,
    ));
    let curl = scroll(lib, 0.05);
    lib.decor(curl.at(at(0.0, 1.89, 0.03), q, kind::WOOD, body));
    // The bridge, the tailpiece, the f-holes and the strings.
    lib.decor(Inst::new(
        at(0.0, 0.62, 0.12),
        q,
        Vec3::new(0.16, 0.012, 0.06),
        kind::WOOD,
        [0.75, 0.6, 0.4, 1.0],
        0,
    ));
    let tailpiece = lib.shape(Mesh::prism(
        &[
            Vec2::new(-0.03, 0.0),
            Vec2::new(0.03, 0.0),
            Vec2::new(0.06, 0.28),
            Vec2::new(-0.06, 0.28),
        ],
        0.015,
    ));
    lib.decor(tailpiece.at(at(0.0, 0.3, 0.11), q, kind::WOOD, EBONY));
    for s in [-1.0f32, 1.0] {
        lib.decor(Inst::new(
            at(s * 0.12, 0.65, 0.101),
            q * Quat::from_rotation_z(s * 0.12),
            Vec3::new(0.012, 0.2, 0.004),
            kind::PLAIN,
            EBONY,
            0,
        ));
    }
    for k in 0..4 {
        let x = -0.024 + k as f32 * 0.016;
        let (lo, hi) = (at(x * 1.6, 0.58, 0.122), at(x, 1.58, 0.128));
        rod(lib, lo, hi, 0.003, 0.003, kind::METAL, [0.7, 0.68, 0.6, 1.0]);
    }
    lib.pole(
        at(0.0, 0.0, 0.0),
        at(0.0, 0.25, 0.0),
        0.02,
        kind::METAL,
        [0.1, 0.1, 0.1, 1.0],
    );
    lib.decor(piece(
        frame,
        (a - 0.35, a + 0.35),
        (o - 0.3, o + 0.3),
        (0.0, 0.08),
        kind::WOOD,
        WALNUT,
    ));
    guard(lib, frame, (a - 0.36, a + 0.36), (o - 0.32, o + 0.36), 2.0);
}

/// Beams across the ceiling and a chandelier over the aisle: two gilt hoops of
/// candles round a stem.
fn ceiling(lib: &mut Library, frame: &RoomFrame) {
    let h = RoomFrame::HEIGHT;
    let inner = RoomFrame::A_HALF;
    for o in [16.6, 21.35, 26.3, 30.9] {
        lib.decor(piece(
            frame,
            (-inner, inner),
            (o - 0.13, o + 0.13),
            (h - 0.3, h),
            kind::WOOD,
            [0.11, 0.055, 0.025],
        ));
    }
    let (ca, co) = (0.0, 24.0);
    let gilt = rgba(GILT, 1.0);
    lib.pole(frame.at(ca, co, 3.75), frame.at(ca, co, h), 0.04, kind::METAL, gilt);
    let drop = lib.turned(
        &[
            (0.0, 0.0),
            (0.2, 0.1),
            (0.45, 0.4),
            (0.5, 0.55),
            (0.3, 0.8),
            (0.18, 0.9),
            (0.0, 1.0),
        ],
        16,
    );
    lib.decor(
        drop.sized(Vec3::new(0.13, 0.2, 0.13))
            .at(frame.at(ca, co, 3.6), Quat::IDENTITY, kind::METAL, gilt),
    );
    let hoop = lib.shape(Mesh::torus(0.04, 64, 8));
    let cup = lib.turned(
        &[(0.0, 0.0), (0.15, 0.0), (0.2, 0.4), (0.5, 0.75), (0.5, 1.0), (0.0, 1.0)],
        12,
    );
    for (r, y, segs) in [(1.0f32, 4.0f32, 12), (0.55, 4.45, 8)] {
        lib.decor(hoop.sized(Vec3::new(2.0 * r + 0.035, 0.035, 2.0 * r + 0.035)).inst(
            frame.at(ca, co, y),
            Quat::IDENTITY,
            kind::METAL,
            gilt,
        ));
        let ring = |th: f32| frame.at(ca + r * libm::cosf(th), co + r * libm::sinf(th), y);
        let step = TAU / segs as f32;
        for k in (0..segs).step_by(2) {
            let at = ring(0.13 + (k as f32 + 0.5) * step);
            lib.decor(
                cup.sized(Vec3::new(0.08, 0.05, 0.08))
                    .at(at, Quat::IDENTITY, kind::METAL, gilt),
            );
            candle(lib, at + Vec3::Y * 0.05, 0.16);
        }
        for k in 0..4 {
            let th = 0.13 + k as f32 * TAU / 4.0;
            lib.pole(frame.at(ca, co, y + 0.25), ring(th), 0.022, kind::METAL, gilt);
        }
    }
    let c = frame.at(ca, co, 4.1);
    lib.light(c + Vec3::Y * 0.2, 11.0, [1.0, 0.68, 0.38], 1.6, LightKind::Flicker);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A fiddle's outline is a simple polygon its prism fills: the body is closed.
    #[test]
    fn fiddle_outline_fills() {
        let m = Mesh::prism(&fiddle(1.0, 0.6), 0.1);
        let area: f32 = m
            .idx
            .as_chunks::<3>()
            .0
            .iter()
            .map(|t| {
                let p = |k: usize| Vec3::from(m.verts[t[k] as usize].pos);
                (p(1) - p(0)).cross(p(2) - p(0)).z.abs() * 0.5
            })
            .sum();
        // Both faces' triangles, the walls' standing edge-on (no area in z).
        assert!(area > 2.0 * 0.6 * 0.6 && area < 2.0 * 0.6 * 0.95, "{area}");
    }
}
