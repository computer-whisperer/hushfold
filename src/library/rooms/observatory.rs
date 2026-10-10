//! The Observatory: department 0, ground floor, north side. A court under a painted
//! night sky: two ranges of cases cross the room, each split by a nave that runs from
//! the door to a great brass telescope aimed out of a tall window in the far wall, and
//! an orrery stands in the court between them. Cases line the walls round about.

use super::*;

/// Half the width of the nave through the ranges.
const NAVE: f32 = 2.0;
/// Where the near and far ranges stand (o), and the court between them.
const RANGE_NEAR: f32 = 19.0;
const RANGE_FAR: f32 = 27.6;
const COURT: f32 = (RANGE_NEAR + RANGE_FAR) * 0.5;
/// Where the telescope's pivot is (a, o, height): beside the window, not before it,
/// so the window is seen whole down the nave.
const SCOPE: (f32, f32, f32) = (-1.25, 30.05, 2.15);

const BRASS_BRIGHT: [f32; 4] = [0.62, 0.43, 0.17, 1.0];
const SCOPE_BRASS: [f32; 4] = [0.5, 0.33, 0.11, 1.0];
const IRON: [f32; 4] = [0.08, 0.08, 0.09, 1.0];
const NIGHT: [f32; 3] = [0.022, 0.03, 0.085];
const PALE_STONE: [f32; 3] = [0.48, 0.45, 0.4];

pub(super) fn furnish(lib: &mut Library, frame: &RoomFrame, rng: &mut StdRng, titles: &mut HashSet<String>) {
    shelves(lib, frame, rng, titles);
    walls(lib, frame);
    sky(lib, frame);
    orrery(lib, frame);
    telescope(lib, frame);
}

/// A ring of `n` rods round `c` in the plane of `u` and `v` (unit, at right angles).
#[allow(clippy::too_many_arguments)]
fn ring(lib: &mut Library, c: Vec3, u: Vec3, v: Vec3, r: f32, n: usize, w: f32, kind: u32, rgba: [f32; 4]) {
    let at = |k: usize| {
        let t = k as f32 / n as f32 * TAU;
        c + (u * libm::cosf(t) + v * libm::sinf(t)) * r
    };
    for k in 0..n {
        // A touch long, so the rods close at the joints.
        let (p, q) = (at(k), at(k + 1));
        let m = (p - q) * 0.04;
        rod(lib, p + m, q - m, w, w, kind, rgba);
    }
}

/// A rough ball: a cube and the same cube turned two ways.
fn ball(lib: &mut Library, c: Vec3, s: f32, kind: u32, rgba: [f32; 4]) {
    for rot in [
        Quat::from_rotation_y(0.4),
        Quat::from_rotation_y(1.2) * Quat::from_rotation_x(0.785),
        Quat::from_rotation_z(0.785) * Quat::from_rotation_y(0.3),
    ] {
        lib.decor(Inst::new(c, rot, Vec3::splat(s), kind, rgba, 0));
    }
}

/// Shelve `cat` across `faces` (all facing one way), left to right as you face them.
fn shelve(lib: &mut Library, rng: &mut StdRng, cat: usize, mut faces: Vec<(Vec3, Vec3)>, titles: &mut HashSet<String>) {
    let right = Vec3::Y.cross(faces[0].1);
    faces.sort_by(|p, q| p.0.dot(right).total_cmp(&q.0.dot(right)));
    lib.add_category(rng, cat, &faces, titles);
}

/// The two ranges across the room (three cases either side of the nave, a category
/// on each face of each), and cases on the walls: four down each side, two either
/// side of the far window and two either side of the door.
fn shelves(lib: &mut Library, frame: &RoomFrame, rng: &mut StdRng, titles: &mut HashSet<String>) {
    let mut cat = frame.categories.start;
    let across = !frame.out_along_x();
    for o in [RANGE_NEAR, RANGE_FAR] {
        let (mut near, mut far) = (Vec::new(), Vec::new());
        for s in [-1.0f32, 1.0] {
            for k in 0..3 {
                let a = s * (NAVE + UNIT_PITCH * (k as f32 + 0.5));
                for face in lib.bookcase(frame.at(a, o, 0.0), across) {
                    if face.1.dot(frame.out()) < 0.0 {
                        near.push(face);
                    } else {
                        far.push(face);
                    }
                }
            }
        }
        for faces in [near, far] {
            shelve(lib, rng, cat, faces, titles);
            cat += 1;
        }
    }
    let wall = |lib: &mut Library, a: f32, o: f32, n: Vec3| lib.wall_bookcase(frame.at(a, o, 0.0), n);
    let (inner, far) = (RoomFrame::A_HALF, RoomFrame::O_FAR);
    let pairs = |first: f32| [-first - UNIT_PITCH, -first, first, first + UNIT_PITCH];
    let far_faces = pairs(NAVE + UNIT_PITCH * 0.5).map(|a| wall(lib, a, far, -frame.out()));
    shelve(lib, rng, cat, far_faces.to_vec(), titles);
    let side = |k: usize| COURT + (k as f32 - 1.5) * UNIT_PITCH;
    let right: Vec<_> = (0..4).map(|k| wall(lib, inner, side(k), -frame.across())).collect();
    shelve(lib, rng, cat + 1, right, titles);
    let near = pairs(5.6 + UNIT_PITCH * 0.5).map(|a| wall(lib, a, RoomFrame::O_NEAR, frame.out()));
    shelve(lib, rng, cat + 2, near.to_vec(), titles);
    let left: Vec<_> = (0..4).map(|k| wall(lib, -inner, side(k), frame.across())).collect();
    shelve(lib, rng, cat + 3, left, titles);
}

/// Panelling where no case stands, sconces at the ends of the side walls, the tall
/// window the telescope looks out of, and star charts.
fn walls(lib: &mut Library, frame: &RoomFrame) {
    let (inner, near, far) = (RoomFrame::A_HALF, RoomFrame::O_NEAR, RoomFrame::O_FAR);
    let t = 0.07;
    let side = frame.side;
    let (o0, o1) = (COURT - 2.0 * UNIT_PITCH - 0.05, COURT + 2.0 * UNIT_PITCH + 0.05);
    for (s, toward) in [(-1.0f32, side.across), (1.0, -side.across)] {
        let (a0, a1) = if s < 0.0 {
            (-inner, -inner + t)
        } else {
            (inner - t, inner)
        };
        lib.wainscot(frame.y, frame.rect(a0, a1, near, o0), toward);
        lib.wainscot(frame.y, frame.rect(a0, a1, o1, far - t), toward);
        for o in [16.6, 30.4] {
            lib.sconce(frame.at(s * (inner - 0.15), o, 2.6), -frame.across() * s);
        }
    }
    // The near wall between the door and its cases, and past them; the far wall
    // either side of the window, inside its cases, and past them.
    let d = RoomFrame::DOOR_HALF + 0.3;
    for (a0, a1) in [(-inner + t, -11.05), (-5.55, -d), (d, 5.55), (11.05, inner - t)] {
        lib.wainscot(frame.y, frame.rect(a0, a1, near, near + t), side.out);
    }
    for (a0, a1) in [(-inner + t, -7.45), (-1.95, -1.6), (1.6, 1.95), (7.45, inner - t)] {
        lib.wainscot(frame.y, frame.rect(a0, a1, far - t, far), -side.out);
    }
    // The tall window: leaded glass in a stone surround, with a sill.
    let (out, up) = (frame.out(), Vec3::Y);
    let glass = |lib: &mut Library, hw: f32, y0: f32, y1: f32| {
        let (lo, hi) = frame.boxed(-hw, hw, far - 0.04, far + 0.04, y0, y1);
        let mut g = Inst::minmax(lo, hi, kind::GLASS, [0.32, 0.42, 0.85]);
        g.color[3] = 1.25;
        lib.decor(g);
    };
    // A round head, in steps, each step's glass and the stone beside it.
    for (hw, y0, y1, stone) in [
        (1.2, 1.0, 4.4, 1.5),
        (1.05, 4.4, 4.75, 1.45),
        (0.8, 4.75, 4.98, 1.3),
        (0.45, 4.98, 5.1, 1.05),
    ] {
        glass(lib, hw, y0, y1);
        for (a0, a1) in [(-stone, -hw), (hw, stone)] {
            lib.decor(piece(
                frame,
                (a0, a1),
                (far - 0.08, far),
                (y0, y1),
                kind::STONE,
                PALE_STONE,
            ));
        }
    }
    lib.decor(piece(
        frame,
        (-0.75, 0.75),
        (far - 0.08, far),
        (5.1, 5.35),
        kind::STONE,
        PALE_STONE,
    ));
    lib.decor(piece(
        frame,
        (-1.55, 1.55),
        (far - 0.2, far),
        (0.85, 1.0),
        kind::STONE,
        PALE_STONE,
    ));
    let c = frame.at(0.0, far - 0.05, 3.2);
    lib.light(c - out * 2.0, 10.0, [0.45, 0.55, 1.0], 1.1, LightKind::Static);
    let beam = (-out * 1.0 - up * 0.75).normalize();
    let len = 8.0;
    lib.static_trans.push(Inst::new(
        c + beam * len * 0.5,
        Quat::from_rotation_arc(Vec3::Y, beam),
        Vec3::new(2.0, len, 2.0),
        kind::SHAFT,
        [0.35, 0.45, 0.9, 0.045],
        0,
    ));
    // Star charts above the end cases down each side, and either side of the door.
    for s in [-1.0f32, 1.0] {
        let n = -frame.across() * s;
        for o in [COURT - 1.5 * UNIT_PITCH, COURT + 1.5 * UNIT_PITCH] {
            chart(lib, frame.at(s * inner, o, 4.35), n, (o * 7.3 + s) as u32);
        }
        chart(lib, frame.at(s * 3.6, near, 2.3), out, (s + 2.0) as u32 * 11);
        chart(lib, frame.at(s * 4.7, far, 4.35), -out, (s + 2.0) as u32 * 13);
    }
}

/// A framed star chart hung on the wall at `c`, facing `n`: parchment inked with a
/// few stars and the lines between them, and a ring.
fn chart(lib: &mut Library, c: Vec3, n: Vec3, seed: u32) {
    let right = Vec3::Y.cross(n);
    let (w, h) = (1.5, 1.05);
    let size = |x: f32, y: f32, z: f32| (right * x + Vec3::Y * y + n * z).abs();
    lib.decor(Inst::aabox(
        c + n * 0.02,
        size(w + 0.12, h + 0.12, 0.04),
        kind::WOOD,
        DARK_WOOD,
    ));
    lib.decor(Inst::aabox(
        c + n * 0.045,
        size(w, h, 0.02),
        kind::PARCHMENT,
        [0.72, 0.62, 0.42],
    ));
    let mut rng = StdRng::seed_from_u64(0x0b5e_7a10 ^ seed as u64);
    let face = c + n * 0.056;
    let ink = [0.16, 0.1, 0.06, 1.0];
    let pts: Vec<Vec3> = (0..7)
        .map(|_| face + right * rng.gen_range(-0.6..0.6) + Vec3::Y * rng.gen_range(-0.4..0.4))
        .collect();
    for p in &pts {
        lib.decor(Inst::aabox(
            *p,
            size(0.035, 0.035, 0.006),
            kind::PLAIN,
            [ink[0], ink[1], ink[2]],
        ));
    }
    for k in 0..4 {
        rod(lib, pts[k], pts[k + 1], 0.008, 0.008, kind::PLAIN, ink);
    }
    ring(
        lib,
        face,
        right,
        Vec3::Y,
        0.42,
        16,
        0.01,
        kind::PLAIN,
        [0.45, 0.22, 0.08, 1.0],
    );
}

/// The ceiling painted as the night sky: a zodiac ring of brass, constellations
/// picked out in points of light with faint lines between, and stars strewn about.
fn sky(lib: &mut Library, frame: &RoomFrame) {
    let h = RoomFrame::HEIGHT;
    let (inner, near, far) = (RoomFrame::A_HALF, RoomFrame::O_NEAR, RoomFrame::O_FAR);
    lib.decor(piece(
        frame,
        (-inner, inner),
        (near, far),
        (h - 0.05, h),
        kind::PLAIN,
        NIGHT,
    ));
    // A cornice round the edge.
    for (a, o) in [
        ((-inner, -inner + 0.22), (near, far)),
        ((inner - 0.22, inner), (near, far)),
        ((-inner + 0.22, inner - 0.22), (near, near + 0.22)),
        ((-inner + 0.22, inner - 0.22), (far - 0.22, far)),
    ] {
        lib.decor(piece(frame, a, o, (h - 0.25, h - 0.05), kind::WOOD, WALNUT));
    }
    let (u, v) = (frame.across(), frame.out());
    let y = h - 0.07;
    let centre = frame.at(0.0, COURT, y);
    let gold = [0.7, 0.5, 0.2, 1.0];
    ring(lib, centre, u, v, 6.4, 48, 0.07, kind::METAL, gold);
    ring(lib, centre, u, v, 5.9, 48, 0.035, kind::METAL, gold);
    for k in 0..12 {
        let t = k as f32 / 12.0 * TAU;
        let dir = u * libm::cosf(t) + v * libm::sinf(t);
        rod(
            lib,
            centre + dir * 5.9,
            centre + dir * 6.4,
            0.05,
            0.05,
            kind::METAL,
            gold,
        );
    }
    // Constellations in the room's frame (a, o), their stars' brightness, and which
    // stars are joined.
    type Figure = (&'static [(f32, f32, f32)], &'static [(usize, usize)]);
    const PLOUGH: Figure = (
        &[
            (-3.0, 20.0, 1.0),
            (-3.0, 21.0, 0.8),
            (-1.7, 21.2, 0.7),
            (-1.5, 20.2, 0.6),
            (-0.4, 19.9, 0.8),
            (0.6, 19.6, 0.9),
            (1.7, 19.9, 0.8),
        ],
        &[(0, 1), (1, 2), (2, 3), (3, 0), (3, 4), (4, 5), (5, 6)],
    );
    const CASSIOPEIA: Figure = (
        &[
            (1.4, 26.6, 0.9),
            (2.2, 25.9, 0.8),
            (3.0, 26.5, 1.0),
            (3.9, 25.8, 0.7),
            (4.6, 26.6, 0.8),
        ],
        &[(0, 1), (1, 2), (2, 3), (3, 4)],
    );
    const HUNTER: Figure = (
        &[
            (-4.2, 27.4, 1.0),
            (-2.2, 27.6, 0.8),
            (-3.6, 26.2, 0.7),
            (-3.25, 26.05, 0.7),
            (-2.9, 25.9, 0.7),
            (-4.4, 24.6, 0.8),
            (-2.0, 24.8, 1.0),
            (-3.2, 28.3, 0.5),
        ],
        &[(0, 2), (1, 4), (2, 3), (3, 4), (2, 5), (4, 6), (0, 7), (1, 7)],
    );
    const SWAN: Figure = (
        &[
            (3.2, 22.0, 1.0),
            (3.2, 23.1, 0.7),
            (3.2, 24.3, 0.6),
            (2.0, 23.0, 0.6),
            (4.4, 23.2, 0.6),
        ],
        &[(0, 1), (1, 2), (3, 1), (1, 4)],
    );
    const SISTERS: Figure = (
        &[
            (-5.0, 22.4, 0.6),
            (-4.8, 22.2, 0.5),
            (-4.9, 22.0, 0.6),
            (-5.15, 22.15, 0.45),
            (-4.65, 22.45, 0.45),
            (-5.1, 22.6, 0.4),
        ],
        &[],
    );
    let line = [0.5, 0.42, 0.25, 0.35];
    for (stars, joins) in [PLOUGH, CASSIOPEIA, HUNTER, SWAN, SISTERS] {
        let at = |s: &(f32, f32, f32)| frame.at(s.0, s.1, y - 0.01);
        for &(i, j) in joins {
            rod(lib, at(&stars[i]), at(&stars[j]), 0.012, 0.012, kind::EMISSIVE, line);
        }
        for s in stars {
            star(lib, at(s), 0.03 + 0.03 * s.2, 2.5 + 4.5 * s.2);
        }
    }
    // The rest, strewn by a fixed hand.
    let mut rng = StdRng::seed_from_u64(0x5747_5bad);
    for _ in 0..110 {
        let a = rng.gen_range(-inner + 0.4..inner - 0.4);
        let o = rng.gen_range(near + 0.4..far - 0.4);
        let b: f32 = rng.gen_range(0.0..1.0f32).powi(3);
        star(lib, frame.at(a, o, y), 0.02 + 0.03 * b, 1.2 + 5.0 * b);
    }
}

/// A point of light on the painted sky, twinkling.
fn star(lib: &mut Library, p: Vec3, s: f32, bright: f32) {
    let rot = Quat::from_rotation_y(0.785);
    lib.decor(Inst::new(
        p,
        rot,
        Vec3::new(s, 0.02, s),
        kind::EMISSIVE,
        [0.85, 0.88, 1.0, bright],
        1,
    ));
}

/// The orrery in the court: a stone plinth, a brass column, the sun glowing at the
/// top and the planets on their arms about it.
fn orrery(lib: &mut Library, frame: &RoomFrame) {
    let base = frame.at(0.0, COURT, 0.0);
    let up = Vec3::Y;
    lib.solid(
        piece(
            frame,
            (-0.7, 0.7),
            (COURT - 0.7, COURT + 0.7),
            (0.0, 0.85),
            kind::STONE,
            PALE_STONE,
        ),
        Stops::ALL,
    );
    for (hw, y0, y1) in [(0.8, 0.0, 0.12), (0.8, 0.85, 0.94)] {
        lib.decor(piece(
            frame,
            (-hw, hw),
            (COURT - hw, COURT + hw),
            (y0, y1),
            kind::STONE,
            PALE_STONE,
        ));
    }
    // A meridian line of brass let into the floor, from the door to the window, with
    // a ring round the plinth.
    let gold = [0.55, 0.38, 0.13];
    for (o0, o1) in [
        (RoomFrame::O_NEAR + 1.0, COURT - 1.25),
        (COURT + 1.25, RoomFrame::O_FAR - 0.3),
    ] {
        lib.decor(piece(frame, (-0.03, 0.03), (o0, o1), (0.0, 0.008), kind::METAL, gold));
    }
    for k in 0..16 {
        let o = RoomFrame::O_NEAR + 1.5 + k as f32;
        if (o - COURT).abs() > 1.3 {
            let w = if k % 5 == 0 { 0.22 } else { 0.12 };
            lib.decor(piece(
                frame,
                (-w, w),
                (o - 0.012, o + 0.012),
                (0.0, 0.006),
                kind::METAL,
                gold,
            ));
        }
    }
    let rgba = [gold[0], gold[1], gold[2], 1.0];
    ring(
        lib,
        base + up * 0.004,
        Vec3::X,
        Vec3::Z,
        1.25,
        32,
        0.03,
        kind::METAL,
        rgba,
    );
    // A brass ring round the top of the plinth, and the column.
    ring(
        lib,
        base + up * 0.97,
        Vec3::X,
        Vec3::Z,
        0.62,
        24,
        0.05,
        kind::METAL,
        BRASS_BRIGHT,
    );
    rod(
        lib,
        base + up * 0.94,
        base + up * 1.78,
        0.07,
        0.07,
        kind::METAL,
        BRASS_BRIGHT,
    );
    let sun = base + up * 1.9;
    ball(lib, sun, 0.24, kind::CRYSTAL, [1.0, 0.72, 0.3, 1.0]);
    // Its light a little above it, so it lights the court rather than flooding itself.
    lib.light(sun + up * 0.7, 5.5, [1.0, 0.7, 0.35], 0.9, LightKind::Static);
    // The sun's armillary ring, tipped to the ecliptic.
    let tilt = Quat::from_rotation_x(0.41);
    ring(
        lib,
        sun,
        tilt * Vec3::X,
        tilt * Vec3::Z,
        0.36,
        20,
        0.02,
        kind::METAL,
        BRASS_BRIGHT,
    );
    // (angle, reach, height, size, colour) of each planet.
    let planets: [(f32, f32, f32, f32, [f32; 3]); 6] = [
        (0.3, 0.42, 1.72, 0.06, [0.6, 0.56, 0.52]),
        (2.5, 0.62, 1.64, 0.09, [0.92, 0.82, 0.55]),
        (4.4, 0.84, 1.56, 0.1, [0.22, 0.42, 0.85]),
        (1.2, 1.05, 1.48, 0.08, [0.82, 0.3, 0.14]),
        (5.4, 1.3, 1.4, 0.17, [0.8, 0.64, 0.44]),
        (3.4, 1.52, 1.32, 0.14, [0.86, 0.76, 0.5]),
    ];
    for (k, &(t, r, h, s, rgb)) in planets.iter().enumerate() {
        let dir = Vec3::new(libm::cosf(t), 0.0, libm::sinf(t));
        let hub = base + up * h;
        let tip = hub + dir * r;
        rod(lib, hub, tip, 0.022, 0.022, kind::METAL, BRASS_BRIGHT);
        rod(lib, tip, tip + up * 0.14, 0.018, 0.018, kind::METAL, BRASS_BRIGHT);
        let p = tip + up * (0.14 + s * 0.5);
        ball(lib, p, s, kind::METAL, [rgb[0], rgb[1], rgb[2], 1.0]);
        if k == 2 {
            // The earth's moon on its own little arm.
            let m = p + Vec3::new(libm::sinf(t), 0.0, -libm::cosf(t)) * 0.16;
            rod(lib, p, m, 0.01, 0.01, kind::METAL, BRASS_BRIGHT);
            ball(lib, m, 0.035, kind::METAL, [0.7, 0.7, 0.68, 1.0]);
        }
        if k == 5 {
            let rot = Quat::from_rotation_y(0.7) * Quat::from_rotation_x(0.45);
            lib.decor(Inst::new(
                p,
                rot,
                Vec3::new(0.38, 0.012, 0.38),
                kind::METAL,
                [0.7, 0.62, 0.42, 1.0],
                0,
            ));
        }
    }
}

/// The great telescope on its cabinet beside the window, looking up and out of it.
fn telescope(lib: &mut Library, frame: &RoomFrame) {
    let (a, o, h) = SCOPE;
    let (up, across) = (Vec3::Y, frame.across());
    let wood = |lib: &mut Library, hw: f32, o0: f32, o1: f32, y0: f32, y1: f32, rgb: [f32; 3]| {
        lib.decor(piece(frame, (a - hw, a + hw), (o0, o1), (y0, y1), kind::WOOD, rgb));
    };
    lib.solid(
        piece(
            frame,
            (a - 0.55, a + 0.55),
            (o - 0.85, o + 0.85),
            (0.0, 0.9),
            kind::WOOD,
            WALNUT,
        ),
        Stops::ALL,
    );
    wood(lib, 0.6, o - 0.9, o + 0.9, 0.9, 0.96, DARK_WOOD);
    // Drawers down the front of the cabinet.
    for y in [0.2, 0.5] {
        wood(lib, 0.45, o - 0.88, o - 0.85, y, y + 0.24, [0.2, 0.1, 0.045]);
    }
    let pivot = frame.at(a, o, h);
    rod(
        lib,
        frame.at(a, o, 0.96),
        pivot - up * 0.25,
        0.16,
        0.16,
        kind::METAL,
        IRON,
    );
    // The tube looks at the top of the window, turned toward it.
    let d = (frame.at(-0.3, RoomFrame::O_FAR, 4.4) - pivot).normalize();
    let axle = d.cross(up).normalize();
    let turn = Quat::from_rotation_arc(across, axle);
    for s in [-1.0f32, 1.0] {
        let c = pivot - up * 0.1 + axle * s * 0.24;
        lib.decor(Inst::new(c, turn, frame.size(0.04, 0.42, 0.2), kind::METAL, IRON, 0));
    }
    rod(
        lib,
        pivot - axle * 0.3,
        pivot + axle * 0.3,
        0.05,
        0.05,
        kind::METAL,
        IRON,
    );
    // Eight-sided, banded, with an eyepiece below and a finder alongside.
    let rot = Quat::from_rotation_arc(Vec3::Y, d);
    let tube = |lib: &mut Library, t: f32, len: f32, w: f32, rgba: [f32; 4]| {
        for twist in [0.0f32, 0.785] {
            let r = rot * Quat::from_rotation_y(twist);
            lib.decor(Inst::new(pivot + d * t, r, Vec3::new(w, len, w), kind::METAL, rgba, 0));
        }
    };
    tube(lib, 0.2, 3.0, 0.3, SCOPE_BRASS);
    for t in [-1.0f32, 0.35, 1.55] {
        tube(lib, t, 0.08, 0.34, [0.36, 0.23, 0.07, 1.0]);
    }
    tube(lib, 1.68, 0.06, 0.33, IRON);
    let low = pivot + d * -1.3;
    rod(lib, low, low - d * 0.25, 0.09, 0.09, kind::METAL, IRON);
    let above = axle.cross(d).normalize();
    rod(
        lib,
        pivot + above * 0.24 - d * 0.2,
        pivot + above * 0.24 + d * 0.7,
        0.06,
        0.06,
        kind::METAL,
        SCOPE_BRASS,
    );
    for t in [-0.1f32, 0.55] {
        rod(
            lib,
            pivot + d * t + above * 0.13,
            pivot + d * t + above * 0.21,
            0.02,
            0.02,
            kind::METAL,
            IRON,
        );
    }
}
