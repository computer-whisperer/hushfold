//! The Music Room: department 9, top floor, east side. Laid out like a recital hall:
//! rows of cases across the room either side of a carpeted aisle, like pews facing
//! the organ in the middle of the far wall, cases flanking it and down the side
//! walls; a harp and a harpsichord before the organ, music stands, chimes and a
//! double bass; instruments hung on the walls, a chandelier over the aisle. The
//! harp's strings glow, and a few notes hang in the air over it.

use super::*;

/// Where the two rows of cases stand (outward), and where the aisle between their
/// ends begins either side of the middle.
const PEWS: [f32; 2] = [19.3, 23.4];
const AISLE: f32 = 2.7;
/// The organ's half-width: the far wall's cases start beside it.
const ORGAN: f32 = 2.0;
const PIPE_TIN: [f32; 3] = [0.6, 0.62, 0.66];
const LACQUER: [f32; 3] = [0.08, 0.13, 0.09];
const GILT: [f32; 3] = [0.7, 0.5, 0.18];
/// The glow of the room's magic: the harp's strings and the notes over it.
const GLOW: [f32; 3] = [1.0, 0.82, 0.45];

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

/// Faces of a straight run, left to right as you face them.
fn run(mut faces: Vec<(Vec3, Vec3)>) -> Vec<(Vec3, Vec3)> {
    faces.sort_by(|p, q| {
        let right = Vec3::Y.cross(p.1);
        p.0.dot(right).total_cmp(&q.0.dot(right))
    });
    faces
}

/// A box over `a` × `o` (each low, high), `y` above the floor.
fn part(frame: &RoomFrame, a: (f32, f32), o: (f32, f32), y: (f32, f32), kind: u32, rgb: [f32; 3]) -> Inst {
    let (lo, hi) = frame.boxed(a.0, a.1, o.0, o.1, y.0, y.1);
    Inst::minmax(lo, hi, kind, rgb)
}

/// In the way of everything over `a` × `o`, from the floor to `h`.
fn block(lib: &mut Library, frame: &RoomFrame, a: (f32, f32), o: (f32, f32), h: f32) {
    let (lo, hi) = frame.boxed(a.0, a.1, o.0, o.1, 0.0, h);
    lib.collide(lo, hi, Stops::ALL);
}

fn rgba(rgb: [f32; 3], w: f32) -> [f32; 4] {
    [rgb[0], rgb[1], rgb[2], w]
}

/// The room's frame as a rotation: x across, y up, z toward the door.
fn turn(frame: &RoomFrame) -> Quat {
    Quat::from_mat3(&Mat3::from_cols(frame.across(), Vec3::Y, -frame.out()))
}

/// A slim box from `p` to `q` (world), `w` × `d` across.
fn rod(lib: &mut Library, p: Vec3, q: Vec3, w: f32, d: f32, kind: u32, col: [f32; 4]) {
    let v = q - p;
    let rot = Quat::from_rotation_arc(Vec3::Y, v.normalize());
    lib.decor(Inst::new((p + q) * 0.5, rot, Vec3::new(w, v.length(), d), kind, col, 0));
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
            let mut faces = run(far);
            faces.extend(run(near));
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
        lib.add_category(rng, cats.next().unwrap(), &run(faces), titles);
    }
    for s in [-1.0f32, 1.0] {
        let faces: Vec<_> = (0..4)
            .map(|k| {
                let a = s * (ORGAN + UNIT_PITCH * (k as f32 + 0.5));
                lib.wall_bookcase(frame.at(a, RoomFrame::O_FAR, 0.0), -out)
            })
            .collect();
        lib.add_category(rng, cats.next().unwrap(), &run(faces), titles);
    }
}

/// A panel on a row's end toward the aisle, a gilt lyre on it.
fn pew_end(lib: &mut Library, frame: &RoomFrame, s: f32, o: f32) {
    // The case's side faces the aisle AISLE + 0.05 from the middle.
    let (a0, a1) = (s * (AISLE + 0.02), s * (AISLE + 0.045));
    lib.decor(part(
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
        let sill = part(
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
    let mut rug = part(
        frame,
        (-1.3, 1.3),
        (near + 0.4, 30.3),
        (0.0, 0.02),
        kind::RUG,
        [0.32, 0.05, 0.07],
    );
    rug.param = 200 | (150 << 8) | (70 << 16);
    lib.decor(rug);
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

    /// A box at `x`, `y`, `z` in the instrument's own frame, `size`, turned `r` more
    /// in its plane.
    fn put(&self, lib: &mut Library, at: [f32; 3], size: [f32; 3], r: f32, kind: u32, col: [f32; 4]) {
        let pos = self.c + self.q * (Vec3::from(at) * self.k);
        let rot = self.q * Quat::from_rotation_z(r);
        lib.decor(Inst::new(pos, rot, Vec3::from(size) * self.k, kind, col, 0));
    }
}

/// A violin hung aslant: a waisted body, a neck and scroll, strings.
fn violin(lib: &mut Library, h: &Hung) {
    let body = rgba([0.45, 0.17, 0.05], 1.0);
    h.put(lib, [0.0, -0.12, 0.0], [0.2, 0.17, 0.05], 0.0, kind::WOOD, body);
    h.put(lib, [0.0, 0.1, 0.0], [0.17, 0.17, 0.05], 0.0, kind::WOOD, body);
    h.put(lib, [0.0, 0.0, 0.0], [0.12, 0.08, 0.05], 0.0, kind::WOOD, body);
    h.put(
        lib,
        [0.0, 0.33, 0.02],
        [0.035, 0.32, 0.03],
        0.0,
        kind::WOOD,
        rgba(DARK_WOOD, 1.0),
    );
    h.put(lib, [0.0, 0.52, 0.01], [0.05, 0.08, 0.05], 0.0, kind::WOOD, body);
    h.put(
        lib,
        [0.0, -0.02, 0.03],
        [0.012, 0.4, 0.01],
        0.0,
        kind::PLAIN,
        [0.1, 0.1, 0.1, 1.0],
    );
}

/// A lute: a round back, a rose, a short neck and a bent-back pegbox.
fn lute(lib: &mut Library, h: &Hung) {
    let wood = rgba([0.5, 0.3, 0.12], 1.0);
    for k in 0..3 {
        let z = -0.02 + k as f32 * 0.004;
        h.put(
            lib,
            [0.0, 0.0, z],
            [0.3, 0.22, 0.1],
            k as f32 * 0.6 - 0.6,
            kind::WOOD,
            wood,
        );
    }
    h.put(
        lib,
        [0.0, 0.0, 0.035],
        [0.06, 0.06, 0.01],
        0.0,
        kind::PLAIN,
        [0.04, 0.03, 0.02, 1.0],
    );
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

/// A horn: a coiled brass ring, a flaring bell and a mouthpipe.
fn horn(lib: &mut Library, h: &Hung) {
    let brass = rgba(GILT, 1.0);
    let n = 10;
    for k in 0..n {
        let th = k as f32 / n as f32 * TAU + 0.1;
        let at = [th.cos() * 0.17, th.sin() * 0.17, 0.0];
        h.put(lib, at, [0.035, 0.12, 0.035], th, kind::METAL, brass);
    }
    h.put(lib, [0.2, -0.12, 0.02], [0.1, 0.16, 0.06], 0.6, kind::METAL, brass);
    h.put(lib, [0.27, -0.21, 0.03], [0.17, 0.08, 0.08], 0.6, kind::METAL, brass);
    h.put(lib, [-0.14, 0.2, 0.0], [0.025, 0.18, 0.025], -0.5, kind::METAL, brass);
}

/// The organ in the middle of the far wall: a case of pipes rising nearly to the
/// ceiling over two keyboards, a music desk with candles, a bench before it.
fn organ(lib: &mut Library, frame: &RoomFrame) {
    let wall = RoomFrame::O_FAR;
    let h = RoomFrame::HEIGHT;
    let w = ORGAN - 0.1;
    // The base, shallow enough to stand behind (where the far views are taken from).
    lib.solid(
        part(frame, (-w, w), (wall - 0.28, wall), (0.0, 1.05), kind::WOOD, WALNUT),
        Stops::ALL,
    );
    lib.decor(part(
        frame,
        (-w - 0.03, w + 0.03),
        (wall - 0.31, wall),
        (1.05, 1.13),
        kind::WOOD,
        [0.12, 0.06, 0.028],
    ));
    lib.decor(part(
        frame,
        (-w + 0.15, w - 0.15),
        (wall - 0.03, wall),
        (1.13, h - 0.35),
        kind::WOOD,
        DARK_WOOD,
    ));
    for s in [-1.0f32, 1.0] {
        let x = s * (w - 0.1);
        lib.decor(part(
            frame,
            (x - 0.12, x + 0.12),
            (wall - 0.3, wall),
            (1.13, h - 0.3),
            kind::WOOD,
            WALNUT,
        ));
    }
    lib.decor(part(
        frame,
        (-w - 0.05, w + 0.05),
        (wall - 0.36, wall),
        (h - 0.3, h - 0.12),
        kind::WOOD,
        [0.12, 0.06, 0.028],
    ));
    // Pipes in three flats: a middle one falling away from its tallest, and towers
    // either side rising outward.
    let mut pipe = |a: f32, top: f32, wide: f32| {
        lib.decor(part(
            frame,
            (a - wide, a + wide),
            (wall - 0.22, wall - 0.22 + 2.0 * wide),
            (1.13, top),
            kind::METAL,
            PIPE_TIN,
        ));
        let mouth = part(
            frame,
            (a - wide * 0.6, a + wide * 0.6),
            (wall - 0.225, wall - 0.215),
            (1.42, 1.52),
            kind::PLAIN,
            [0.02, 0.02, 0.02],
        );
        lib.decor(mouth);
    };
    for k in -3i32..=3 {
        let a = k as f32 * 0.16;
        pipe(a, 4.2 - 0.18 * (k.abs() as f32).powf(1.3), 0.06);
    }
    for s in [-1.0f32, 1.0] {
        for k in 0..4 {
            let a = s * (0.75 + k as f32 * 0.2);
            pipe(a, 3.75 + k as f32 * 0.38, 0.075);
        }
    }
    // The keyboards: a shelf with white keys and their black ones, a second above
    // and behind it; stops either side; the music desk.
    for (k, (o0, y)) in [(wall - 0.62, 0.78), (wall - 0.46, 0.92)].into_iter().enumerate() {
        let (o1, kw) = (o0 + 0.24, 0.85 - k as f32 * 0.05);
        lib.decor(part(
            frame,
            (-kw - 0.08, kw + 0.08),
            (o0, wall - 0.28),
            (y - 0.05, y),
            kind::WOOD,
            [0.12, 0.06, 0.028],
        ));
        lib.decor(part(
            frame,
            (-kw, kw),
            (o0 + 0.01, o1 - 0.06),
            (y, y + 0.02),
            kind::PAPER,
            [0.0; 3],
        ));
        lib.decor(part(
            frame,
            (-kw, kw),
            (o0 + 0.1, o1 - 0.06),
            (y + 0.02, y + 0.035),
            kind::PLAIN,
            [0.03, 0.03, 0.03],
        ));
    }
    for s in [-1.0f32, 1.0] {
        for (k, y) in [1.0f32, 1.12, 1.24].into_iter().enumerate() {
            let a = s * (1.05 + 0.04 * k as f32);
            let col = if k == 1 { [0.6, 0.1, 0.08] } else { [0.85, 0.82, 0.72] };
            lib.decor(part(
                frame,
                (a - 0.035, a + 0.035),
                (wall - 0.36, wall - 0.28),
                (y, y + 0.07),
                kind::PLAIN,
                col,
            ));
        }
    }
    let desk = Quat::from_rotation_x(-0.25);
    let q = turn(frame) * desk;
    lib.decor(Inst::new(
        frame.at(0.0, wall - 0.36, 1.32),
        q,
        Vec3::new(0.9, 0.42, 0.03),
        kind::WOOD,
        rgba(WALNUT, 1.0),
        0,
    ));
    for s in [-1.0f32, 1.0] {
        let sheet = frame.at(s * 0.2, wall - 0.38, 1.34);
        lib.decor(Inst::new(
            sheet,
            q,
            Vec3::new(0.36, 0.34, 0.02),
            kind::PARCHMENT,
            [0.85, 0.78, 0.6, 1.0],
            0,
        ));
        // A candle each side, on the case beside the towers.
        let base = frame.at(s * 1.55, wall - 0.18, 1.13);
        lib.decor(Inst::aabox(
            base + Vec3::Y * 0.02,
            Vec3::new(0.12, 0.04, 0.12),
            kind::METAL,
            GILT,
        ));
        lib.decor(Inst::aabox(
            base + Vec3::Y * 0.13,
            Vec3::new(0.05, 0.18, 0.05),
            kind::PLAIN,
            [0.85, 0.8, 0.65],
        ));
        let mut flame = Inst::aabox(
            base + Vec3::Y * 0.26,
            Vec3::new(0.025, 0.05, 0.025),
            kind::EMISSIVE,
            [1.0, 0.55, 0.18],
        );
        flame.color[3] = 14.0;
        flame.param = 1;
        lib.decor(flame);
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
    lib.decor(part(frame, (-0.8, 0.8), (o0, o1), (0.46, 0.52), kind::WOOD, WALNUT));
    for s in [-1.0f32, 1.0] {
        lib.decor(part(
            frame,
            (s * 0.72 - 0.04, s * 0.72 + 0.04),
            (o0 + 0.02, o1 - 0.02),
            (0.0, 0.46),
            kind::WOOD,
            DARK_WOOD,
        ));
    }
    block(lib, frame, (-0.8, 0.8), (o0, o1), 0.52);
}

/// A harp standing at `a`, `o` (its foot's front), its strings facing across the
/// room: a column, a slanting soundbox, a curved neck, and strings that glow.
fn harp(lib: &mut Library, frame: &RoomFrame, a: f32, o: f32) {
    let out = frame.out();
    // Points in the harp's plane: `u` outward from its foot's front, `v` up.
    let p = |u: f32, v: f32| frame.at(a, o + u, v);
    let gilt = rgba(GILT, 1.0);
    let wood = rgba([0.3, 0.14, 0.05], 1.0);
    lib.decor(part(
        frame,
        (a - 0.2, a + 0.2),
        (o - 0.05, o + 0.75),
        (0.0, 0.12),
        kind::WOOD,
        [0.3, 0.14, 0.05],
    ));
    rod(lib, p(0.0, 0.12), p(-0.04, 1.82), 0.07, 0.07, kind::METAL, gilt);
    lib.decor(Inst::aabox(p(-0.04, 1.86), Vec3::splat(0.11), kind::METAL, GILT));
    rod(lib, p(0.08, 0.12), p(0.72, 1.5), 0.16, 0.2, kind::WOOD, wood);
    rod(lib, p(-0.04, 1.84), p(0.34, 1.76), 0.06, 0.08, kind::WOOD, wood);
    rod(lib, p(0.34, 1.76), p(0.74, 1.52), 0.06, 0.08, kind::WOOD, wood);
    // The strings, from the soundbox up to the neck.
    for k in 0..11 {
        let u = 0.1 + k as f32 * 0.055;
        let bottom = 0.12 + (1.5 - 0.12) * ((u - 0.08) / 0.64) + 0.06;
        let neck = if u < 0.34 {
            1.84 - 0.08 * (u + 0.04) / 0.38
        } else {
            1.76 - 0.24 * (u - 0.34) / 0.4
        };
        let top = neck - 0.04;
        if top - bottom > 0.05 {
            rod(
                lib,
                p(u, bottom),
                p(u, top),
                0.008,
                0.008,
                kind::CRYSTAL,
                rgba(GLOW, 0.9),
            );
        }
    }
    block(lib, frame, (a - 0.22, a + 0.22), (o - 0.1, o + 0.8), 1.9);
    // Notes hanging in the air over it, as if the strings had just been played.
    let n = frame.across() * if a < 0.0 { 1.0 } else { -1.0 };
    for (k, (du, v)) in [(-0.3, 2.2), (0.15, 2.45), (0.55, 2.3), (0.95, 2.6)]
        .into_iter()
        .enumerate()
    {
        let c = p(du, v) + n * 0.1;
        let q = Quat::from_mat3(&Mat3::from_cols(out, Vec3::Y, out.cross(Vec3::Y).normalize()));
        let tilt = q * Quat::from_rotation_z(0.35);
        lib.decor(Inst::new(
            c,
            tilt,
            Vec3::new(0.09, 0.065, 0.02),
            kind::CRYSTAL,
            rgba(GLOW, 1.6),
            0,
        ));
        let stem = c + out * 0.04 + Vec3::Y * 0.11;
        lib.decor(Inst::new(
            stem,
            q,
            Vec3::new(0.012, 0.2, 0.012),
            kind::CRYSTAL,
            rgba(GLOW, 1.6),
            0,
        ));
        if k % 2 == 1 {
            let flag = stem + Vec3::Y * 0.07 + out * 0.04;
            lib.decor(Inst::new(
                flag,
                q * Quat::from_rotation_z(-0.7),
                Vec3::new(0.012, 0.1, 0.012),
                kind::CRYSTAL,
                rgba(GLOW, 1.6),
                0,
            ));
        }
    }
    lib.light(p(0.4, 2.2), 3.0, GLOW, 0.4, LightKind::Static);
}

/// A harpsichord with its keyboard toward the door at `o`, its body running out to
/// a narrow tail, lid propped open, a bench before it.
fn harpsichord(lib: &mut Library, frame: &RoomFrame, a: f32, o: f32) {
    let (len, wide, tail) = (2.2, 0.48, 0.22);
    let (y0, y1) = (0.72, 0.98);
    // The body: wide by the keyboard, narrowing along the bentside toward the aisle.
    lib.decor(part(
        frame,
        (a - wide, a + wide),
        (o, o + 1.2),
        (y0, y1),
        kind::WOOD,
        LACQUER,
    ));
    lib.decor(part(
        frame,
        (a - wide + 0.25, a + wide),
        (o + 1.2, o + 1.75),
        (y0, y1),
        kind::WOOD,
        LACQUER,
    ));
    lib.decor(part(
        frame,
        (a + wide - 2.0 * tail, a + wide),
        (o + 1.75, o + len),
        (y0, y1),
        kind::WOOD,
        LACQUER,
    ));
    let board = part(
        frame,
        (a - wide + 0.03, a + wide - 0.03),
        (o + 0.25, o + 1.18),
        (y1, y1 + 0.005),
        kind::PARCHMENT,
        [0.75, 0.62, 0.4],
    );
    lib.decor(board);
    // A gilt band along its front.
    lib.decor(part(
        frame,
        (a - wide - 0.01, a + wide + 0.01),
        (o - 0.01, o + 0.03),
        (y1 - 0.04, y1 + 0.01),
        kind::METAL,
        GILT,
    ));
    // The keyboard, standing out from the front between cheeks.
    let (k0, k1) = (o - 0.24, o - 0.01);
    lib.decor(part(
        frame,
        (a - 0.42, a + 0.42),
        (k0, k1),
        (y0 + 0.1, y0 + 0.12),
        kind::PAPER,
        [0.0; 3],
    ));
    lib.decor(part(
        frame,
        (a - 0.42, a + 0.42),
        (k0 + 0.1, k1),
        (y0 + 0.12, y0 + 0.135),
        kind::PLAIN,
        [0.03, 0.03, 0.03],
    ));
    lib.decor(part(
        frame,
        (a - 0.42, a + 0.42),
        (k0, k1),
        (y0 + 0.02, y0 + 0.1),
        kind::WOOD,
        LACQUER,
    ));
    for x in [a - 0.46, a + 0.46] {
        lib.decor(part(
            frame,
            (x - 0.04, x + 0.04),
            (k0 - 0.01, k1),
            (y0 + 0.02, y0 + 0.19),
            kind::WOOD,
            LACQUER,
        ));
    }
    // The lid, hinged along the straight side and propped open toward the aisle, its
    // painted inside to the room.
    let q = turn(frame) * Quat::from_rotation_z(-0.75);
    let hinge = frame.at(a + wide, o + len * 0.48, y1);
    let (dir, up) = (-(q * Vec3::X), q * Vec3::Y);
    let lid = hinge + dir * 0.5;
    lib.decor(Inst::new(
        lid,
        q,
        Vec3::new(1.0, 0.025, len * 0.92),
        kind::WOOD,
        rgba(LACQUER, 1.0),
        0,
    ));
    let painted = Inst::new(
        lid - up * 0.016,
        q,
        Vec3::new(0.94, 0.006, len * 0.86),
        kind::PARCHMENT,
        [0.62, 0.7, 0.55, 1.0],
        0,
    );
    lib.decor(painted);
    let stick = (frame.at(a - wide + 0.2, o + 1.0, y1), hinge + dir * 0.85 - up * 0.03);
    rod(lib, stick.0, stick.1, 0.02, 0.02, kind::WOOD, rgba(DARK_WOOD, 1.0));
    // Legs, a music sheet on the desk, and the bench.
    for (da, d_o) in [
        (-wide + 0.06, 0.06),
        (wide - 0.06, 0.06),
        (wide - 0.06, len - 0.1),
        (-wide + 0.3, 1.6),
    ] {
        let (la, lo) = (a + da, o + d_o);
        lib.decor(part(
            frame,
            (la - 0.04, la + 0.04),
            (lo - 0.04, lo + 0.04),
            (0.0, y0),
            kind::WOOD,
            [0.05, 0.08, 0.06],
        ));
    }
    let desk = turn(frame) * Quat::from_rotation_x(-0.3);
    lib.decor(Inst::new(
        frame.at(a, o + 0.3, y1 + 0.17),
        desk,
        Vec3::new(0.42, 0.3, 0.02),
        kind::PARCHMENT,
        [0.85, 0.78, 0.6, 1.0],
        0,
    ));
    block(lib, frame, (a - wide, a + wide), (o - 0.25, o + len), y1);
    let (b0, b1) = (o - 0.72, o - 0.4);
    lib.decor(part(
        frame,
        (a - 0.45, a + 0.45),
        (b0, b1),
        (0.44, 0.5),
        kind::WOOD,
        WALNUT,
    ));
    for s in [-1.0f32, 1.0] {
        let x = a + s * 0.38;
        lib.decor(part(
            frame,
            (x - 0.04, x + 0.04),
            (b0 + 0.02, b1 - 0.02),
            (0.0, 0.44),
            kind::WOOD,
            DARK_WOOD,
        ));
    }
    block(lib, frame, (a - 0.45, a + 0.45), (b0, b1), 0.5);
}

/// A music stand at `a`, `o` (on the floor), its desk turned `turn_by` from facing
/// the door, a sheet of music on it.
fn music_stand(lib: &mut Library, frame: &RoomFrame, a: f32, o: f32, turn_by: f32) {
    let base = frame.at(a, o, 0.0);
    let q = turn(frame) * Quat::from_rotation_y(turn_by);
    for k in 0..3 {
        let leg = turn(frame) * Quat::from_rotation_y(k as f32 * TAU / 3.0 + turn_by);
        let tip = base + leg * Vec3::new(0.0, 0.0, 0.22);
        rod(
            lib,
            base + Vec3::Y * 0.3,
            tip,
            0.02,
            0.02,
            kind::METAL,
            [0.08, 0.08, 0.09, 1.0],
        );
    }
    lib.decor(Inst::aabox(
        base + Vec3::Y * 0.7,
        Vec3::new(0.025, 0.8, 0.025),
        kind::METAL,
        [0.08, 0.08, 0.09],
    ));
    let tilt = q * Quat::from_rotation_x(-0.35);
    let desk = base + Vec3::Y * 1.18;
    lib.decor(Inst::new(
        desk,
        tilt,
        Vec3::new(0.5, 0.36, 0.02),
        kind::METAL,
        [0.08, 0.08, 0.09, 1.0],
        0,
    ));
    let sheet = desk + tilt * Vec3::new(0.0, 0.02, 0.015);
    lib.decor(Inst::new(
        sheet,
        tilt,
        Vec3::new(0.44, 0.3, 0.01),
        kind::PARCHMENT,
        [0.88, 0.82, 0.66, 1.0],
        0,
    ));
    block(lib, frame, (a - 0.24, a + 0.24), (o - 0.24, o + 0.24), 1.35);
}

/// Tubular chimes hung in a frame at `a`, `o`, longest to shortest.
fn chimes(lib: &mut Library, frame: &RoomFrame, a: f32, o: f32) {
    let (w, top) = (1.0, 2.05);
    for s in [-1.0f32, 1.0] {
        let x = a + s * w;
        lib.decor(part(
            frame,
            (x - 0.05, x + 0.05),
            (o - 0.05, o + 0.05),
            (0.0, top),
            kind::WOOD,
            WALNUT,
        ));
        lib.decor(part(
            frame,
            (x - 0.08, x + 0.08),
            (o - 0.25, o + 0.25),
            (0.0, 0.08),
            kind::WOOD,
            WALNUT,
        ));
    }
    lib.decor(part(
        frame,
        (a - w - 0.08, a + w + 0.08),
        (o - 0.06, o + 0.06),
        (top, top + 0.1),
        kind::WOOD,
        WALNUT,
    ));
    for k in 0..8 {
        let x = a - w + 0.2 + k as f32 * 0.228;
        let len = 1.45 - k as f32 * 0.1;
        lib.decor(part(
            frame,
            (x - 0.032, x + 0.032),
            (o - 0.032, o + 0.032),
            (top - 0.05 - len, top - 0.05),
            kind::METAL,
            GILT,
        ));
    }
    block(lib, frame, (a - w - 0.1, a + w + 0.1), (o - 0.26, o + 0.26), top + 0.1);
}

/// A double bass leaning in its stand at `a`, `o`.
fn double_bass(lib: &mut Library, frame: &RoomFrame, a: f32, o: f32) {
    let q = turn(frame) * Quat::from_rotation_x(0.12);
    let base = frame.at(a, o, 0.0);
    let at = |x: f32, y: f32, z: f32| base + q * Vec3::new(x, y, z);
    let body = rgba([0.4, 0.15, 0.05], 1.0);
    for (y, w) in [(0.45, 0.66), (0.9, 0.52), (0.68, 0.4)] {
        lib.decor(Inst::new(
            at(0.0, y, 0.0),
            q,
            Vec3::new(w, 0.36, 0.2 + w * 0.05),
            kind::WOOD,
            body,
            0,
        ));
    }
    lib.decor(Inst::new(
        at(0.0, 1.45, 0.06),
        q,
        Vec3::new(0.07, 0.75, 0.05),
        kind::WOOD,
        rgba(DARK_WOOD, 1.0),
        0,
    ));
    lib.decor(Inst::new(
        at(0.0, 1.88, 0.03),
        q,
        Vec3::new(0.08, 0.14, 0.08),
        kind::WOOD,
        body,
        0,
    ));
    lib.decor(Inst::new(
        at(0.0, 0.18, 0.0),
        q,
        Vec3::new(0.02, 0.25, 0.02),
        kind::METAL,
        [0.1, 0.1, 0.1, 1.0],
        0,
    ));
    lib.decor(Inst::new(
        at(0.0, 1.0, 0.13),
        q,
        Vec3::new(0.06, 1.3, 0.01),
        kind::PLAIN,
        [0.1, 0.1, 0.1, 1.0],
        0,
    ));
    lib.decor(part(
        frame,
        (a - 0.35, a + 0.35),
        (o - 0.3, o + 0.3),
        (0.0, 0.08),
        kind::WOOD,
        WALNUT,
    ));
    block(lib, frame, (a - 0.36, a + 0.36), (o - 0.32, o + 0.36), 2.0);
}

/// Beams across the ceiling and a chandelier over the aisle.
fn ceiling(lib: &mut Library, frame: &RoomFrame) {
    let h = RoomFrame::HEIGHT;
    let inner = RoomFrame::A_HALF;
    for o in [16.6, 21.35, 26.3, 30.9] {
        lib.decor(part(
            frame,
            (-inner, inner),
            (o - 0.13, o + 0.13),
            (h - 0.3, h),
            kind::WOOD,
            [0.11, 0.055, 0.025],
        ));
    }
    // A chandelier of two rings of candles round a stem.
    let (ca, co) = (0.0, 24.0);
    let gilt = rgba(GILT, 1.0);
    rod(
        lib,
        frame.at(ca, co, 3.75),
        frame.at(ca, co, h),
        0.04,
        0.04,
        kind::METAL,
        gilt,
    );
    let finial = Quat::from_rotation_y(0.785) * Quat::from_rotation_x(0.6);
    lib.decor(Inst::new(
        frame.at(ca, co, 3.75),
        finial,
        Vec3::splat(0.12),
        kind::METAL,
        gilt,
        0,
    ));
    for (r, y, segs) in [(1.0f32, 4.0f32, 12), (0.55, 4.45, 8)] {
        let ring = |th: f32| frame.at(ca + r * th.cos(), co + r * th.sin(), y);
        let step = TAU / segs as f32;
        for k in 0..segs {
            let th = 0.13 + k as f32 * step;
            let (p, q) = (ring(th), ring(th + step));
            let d = (q - p).normalize();
            rod(lib, p - d * 0.02, q + d * 0.02, 0.035, 0.035, kind::METAL, gilt);
            if k % 2 == 0 {
                let cup = ring(th + step * 0.5);
                lib.decor(Inst::aabox(
                    cup + Vec3::Y * 0.02,
                    Vec3::new(0.09, 0.04, 0.09),
                    kind::METAL,
                    GILT,
                ));
                lib.decor(Inst::aabox(
                    cup + Vec3::Y * 0.12,
                    Vec3::new(0.04, 0.16, 0.04),
                    kind::PLAIN,
                    [0.85, 0.8, 0.65],
                ));
                let mut flame = Inst::aabox(
                    cup + Vec3::Y * 0.23,
                    Vec3::new(0.025, 0.055, 0.025),
                    kind::EMISSIVE,
                    [1.0, 0.55, 0.18],
                );
                flame.color[3] = 14.0;
                flame.param = 1;
                lib.decor(flame);
            }
        }
        for k in 0..4 {
            let th = 0.13 + k as f32 * TAU / 4.0;
            rod(
                lib,
                frame.at(ca, co, y + 0.25),
                ring(th),
                0.025,
                0.025,
                kind::METAL,
                gilt,
            );
        }
    }
    let c = frame.at(ca, co, 4.1);
    lib.light(c + Vec3::Y * 0.2, 11.0, [1.0, 0.68, 0.38], 1.6, LightKind::Flicker);
}
