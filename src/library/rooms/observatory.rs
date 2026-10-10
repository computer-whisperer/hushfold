//! The Observatory: department 0, ground floor, north side. A court under a painted
//! night sky: two ranges of cases cross the room, each split by a nave that runs from
//! the door to a great brass telescope aimed out of a tall window in the far wall, and
//! an orrery stands in the court between them. Cases line the walls round about.

use super::*;
use std::f32::consts::{FRAC_PI_2, PI};

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

/// `n` points round a circle `r` across in xy, anticlockwise.
fn circle(n: usize, r: f32) -> Vec<Vec2> {
    (0..n)
        .map(|k| {
            let t = k as f32 / n as f32 * TAU;
            Vec2::new(libm::cosf(t), libm::sinf(t)) * r
        })
        .collect()
}

/// A flat band round `c` in the plane of `u` and `v` (unit, at right angles), `r` to
/// its middle, `w` wide and `d` thick.
fn band(
    lib: &mut Library,
    c: Vec3,
    (u, v): (Vec3, Vec3),
    (r, w, d): (f32, f32, f32),
    kind: u32,
    rgba: [f32; 4],
) -> Inst {
    let segs = if r < 0.5 { 32 } else { 64 };
    let ring = Mesh::ring(&circle(segs, r + w * 0.5), &circle(segs, r - w * 0.5), d);
    let turn = Quat::from_mat3(&Mat3::from_cols(u, v, u.cross(v)));
    lib.shape(ring).at(c, turn, kind, rgba)
}

/// A round rod (`stick`, a cylinder) from `p` to `q`, `w` across.
fn bar(stick: Shape, p: Vec3, q: Vec3, w: f32, kind: u32, rgba: [f32; 4]) -> Inst {
    let v = q - p;
    let turn = Quat::from_rotation_arc(Vec3::Y, v.normalize());
    stick
        .sized(Vec3::new(w, v.length(), w))
        .inst((p + q) * 0.5, turn, kind, rgba)
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
    // The tall window: leaded glass under a round head in a stone surround, with a
    // keystone and a sill. Outlines across (x) and up (y) from the floor at the wall.
    let (out, up) = (frame.out(), Vec3::Y);
    let wall = Quat::from_mat3(&Mat3::from_cols(frame.across(), up, frame.across().cross(up)));
    let (spring, r) = (4.0, 1.2);
    let head = |r: f32, from: f32, to: f32| -> Vec<Vec2> {
        (0..=32)
            .map(|k| {
                let t = from + (to - from) * k as f32 / 32.0;
                Vec2::new(libm::cosf(t) * r, spring + libm::sinf(t) * r)
            })
            .collect()
    };
    let mut opening = vec![Vec2::new(-r - 0.02, 1.0), Vec2::new(r + 0.02, 1.0)];
    opening.extend(head(r + 0.02, 0.0, PI));
    let glass = lib.shape(Mesh::prism(&opening, 0.08));
    lib.decor(glass.at(frame.at(0.0, far, 0.0), wall, kind::GLASS, [0.32, 0.42, 0.85, 1.25]));
    let mut arch = vec![Vec2::new(-1.5, spring)];
    arch.extend(head(r, PI, 0.0));
    arch.extend([Vec2::new(1.5, spring), Vec2::new(1.5, 5.36), Vec2::new(-1.5, 5.36)]);
    let arch = lib.shape(Mesh::prism(&arch, 0.08));
    lib.decor(arch.at(frame.at(0.0, far - 0.04, 0.0), wall, kind::STONE, rgba(PALE_STONE, 1.0)));
    for (a0, a1) in [(-1.5, -r), (r, 1.5)] {
        lib.decor(piece(
            frame,
            (a0, a1),
            (far - 0.08, far),
            (1.0, spring),
            kind::STONE,
            PALE_STONE,
        ));
    }
    lib.decor(piece(
        frame,
        (-0.13, 0.13),
        (far - 0.12, far),
        (spring + r - 0.08, 5.36),
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
    let hoop = band(
        lib,
        face,
        (right, Vec3::Y),
        (0.42, 0.012, 0.004),
        kind::PLAIN,
        [0.45, 0.22, 0.08, 1.0],
    );
    lib.decor(hoop);
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
    // The zodiac: two gilt bands with the twelve houses between them, each marked with
    // a boss, turning once in half an hour.
    let mut zodiac = Mesh::ring(&circle(128, 6.46), &circle(128, 6.34), 0.05);
    zodiac.add(
        &Mesh::ring(&circle(128, 5.93), &circle(128, 5.87), 0.035),
        Vec3::ZERO,
        Quat::IDENTITY,
        Vec3::ONE,
    );
    let (spoke, boss) = (Mesh::cube(), Mesh::cylinder(16));
    for k in 0..12 {
        let t = k as f32 / 12.0 * TAU;
        let dir = Vec3::new(libm::cosf(t), libm::sinf(t), 0.0);
        zodiac.add(
            &spoke,
            dir * 6.13,
            Quat::from_rotation_z(t),
            Vec3::new(0.44, 0.045, 0.04),
        );
        let mid = Quat::from_rotation_z(t + TAU / 24.0);
        let at = mid * Vec3::X * 6.13;
        zodiac.add(&boss, at, Quat::from_rotation_x(FRAC_PI_2), Vec3::new(0.12, 0.03, 0.12));
    }
    let zodiac = lib.shape(zodiac);
    let turn = Quat::from_mat3(&Mat3::from_cols(u, v, u.cross(v)));
    let m = lib.mover(
        centre,
        Motion::Spin {
            axis: Vec3::Y,
            rate: TAU / 1800.0,
        },
        0.0,
        None,
    );
    let gold = [0.7, 0.5, 0.2, 1.0];
    lib.moving(m, zodiac.at(centre, turn, kind::METAL, gold));
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

/// How long the orrery's earth takes to go round its sun (seconds); the other planets
/// keep their years to it.
const YEAR: f32 = 120.0;

/// The orrery in the court: a marble drum, a brass column, the sun glowing at the top
/// and the planets going round it on their arms, each in its own year, the moon round
/// the earth.
fn orrery(lib: &mut Library, frame: &RoomFrame) {
    let base = frame.at(0.0, COURT, 0.0);
    let up = Vec3::Y;
    guard(lib, frame, (-0.7, 0.7), (COURT - 0.7, COURT + 0.7), 0.85);
    let v = Vec2::new;
    let drum = lib.shape(Mesh::lathe(
        &[
            v(0.0, 0.0),
            v(0.8, 0.0),
            v(0.8, 0.1),
            v(0.74, 0.13),
            v(0.7, 0.18),
            v(0.7, 0.8),
            v(0.74, 0.83),
            v(0.8, 0.86),
            v(0.8, 0.94),
            v(0.0, 0.94),
        ],
        64,
    ));
    let mut marble = drum.at(base, Quat::IDENTITY, kind::STONE, rgba(PALE_STONE, 1.0));
    marble.param = 1;
    lib.decor(marble);
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
    let flat = (Vec3::X, Vec3::Z);
    let inlay = band(
        lib,
        base + up * 0.004,
        flat,
        (1.25, 0.04, 0.008),
        kind::METAL,
        rgba(gold, 1.0),
    );
    lib.decor(inlay);
    // A brass hoop round the top of the drum, and the column.
    let hoop = lib.shape(Mesh::torus(0.08, 64, 10)).scaled(0.62);
    lib.decor(hoop.at(base + up * 0.965, Quat::IDENTITY, kind::METAL, BRASS_BRIGHT));
    let column = lib.shape(Mesh::lathe(
        &[
            v(0.0, 0.94),
            v(0.1, 0.94),
            v(0.1, 0.97),
            v(0.06, 1.0),
            v(0.04, 1.06),
            v(0.03, 1.2),
            v(0.03, 1.74),
            v(0.05, 1.76),
            v(0.06, 1.79),
            v(0.0, 1.79),
        ],
        24,
    ));
    lib.decor(column.at(base, Quat::IDENTITY, kind::METAL, BRASS_BRIGHT));
    let sun = base + up * 1.9;
    let ball = lib.shape(Mesh::sphere(32));
    lib.decor(
        ball.sized(Vec3::splat(0.24))
            .inst(sun, Quat::IDENTITY, kind::CRYSTAL, [1.0, 0.72, 0.3, 1.4]),
    );
    // Its light a little above it, so it lights the court rather than flooding itself.
    lib.light(sun + up * 0.7, 5.5, [1.0, 0.7, 0.35], 0.9, LightKind::Static);
    // The sun's armillary ring, tipped to the ecliptic.
    let tilt = Quat::from_rotation_x(0.41);
    let ecliptic = lib.shape(Mesh::torus(0.06, 48, 8)).scaled(0.36);
    lib.decor(ecliptic.at(sun, tilt, kind::METAL, BRASS_BRIGHT));
    // (angle, reach, height, size, colour, years) of each planet.
    let planets: [(f32, f32, f32, f32, [f32; 3], f32); 6] = [
        (0.3, 0.42, 1.72, 0.06, [0.6, 0.56, 0.52], 0.24),
        (2.5, 0.62, 1.64, 0.09, [0.92, 0.82, 0.55], 0.62),
        (4.4, 0.84, 1.56, 0.1, [0.22, 0.42, 0.85], 1.0),
        (1.2, 1.05, 1.48, 0.08, [0.82, 0.3, 0.14], 1.88),
        (5.4, 1.3, 1.4, 0.17, [0.8, 0.64, 0.44], 11.9),
        (3.4, 1.52, 1.32, 0.14, [0.86, 0.76, 0.5], 29.5),
    ];
    let stick = lib.shape(Mesh::cylinder(12));
    let sleeve = lib.shape(Mesh::cylinder(24)).sized(Vec3::new(0.085, 0.035, 0.085));
    for (k, &(t, r, h, s, rgb, years)) in planets.iter().enumerate() {
        let spin = Motion::Spin {
            axis: up,
            rate: TAU / (YEAR * years),
        };
        let arm = lib.mover(base, spin, 0.0, None);
        let dir = Vec3::new(libm::cosf(t), 0.0, libm::sinf(t));
        let hub = base + up * h;
        let tip = hub + dir * r;
        lib.moving(arm, sleeve.inst(hub, Quat::IDENTITY, kind::METAL, BRASS_BRIGHT));
        lib.moving(arm, bar(stick, hub, tip + dir * 0.01, 0.02, kind::METAL, BRASS_BRIGHT));
        lib.moving(arm, bar(stick, tip, tip + up * 0.14, 0.016, kind::METAL, BRASS_BRIGHT));
        let p = tip + up * (0.14 + s * 0.5);
        // Each world turned a little on its axis, so its shading isn't the same.
        let lean = Quat::from_rotation_z(0.2 + k as f32 * 0.3);
        lib.moving(
            arm,
            ball.sized(Vec3::splat(s)).inst(p, lean, kind::METAL, rgba(rgb, 1.0)),
        );
        if k == 5 {
            let rings = Quat::from_rotation_y(0.7) * Quat::from_rotation_x(0.45);
            let ring = band(
                lib,
                p,
                (rings * Vec3::X, rings * Vec3::Z),
                (0.15, 0.07, 0.006),
                kind::METAL,
                [0.7, 0.62, 0.42, 1.0],
            );
            lib.moving(arm, ring);
        }
        if k == 2 {
            // The moon round the earth on its own little arm, a month to the year.
            let month = Motion::Spin {
                axis: up,
                rate: TAU / (YEAR / 12.0),
            };
            let moon = lib.mover(p, month, 0.0, Some(arm));
            let m = p + Vec3::new(libm::sinf(t), 0.0, -libm::cosf(t)) * 0.16;
            lib.moving(moon, bar(stick, p, m, 0.01, kind::METAL, BRASS_BRIGHT));
            lib.moving(
                moon,
                ball.sized(Vec3::splat(0.035))
                    .inst(m, Quat::IDENTITY, kind::METAL, [0.7, 0.7, 0.68, 1.0]),
            );
        }
    }
}

/// The great telescope on its cabinet beside the window, looking up and out of it,
/// and now and then sweeping a little across the sky and back.
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
    let foot = frame.at(a, o, 0.96);
    let v = Vec2::new;
    let pillar = lib.shape(Mesh::lathe(
        &[
            v(0.0, 0.0),
            v(0.2, 0.0),
            v(0.2, 0.04),
            v(0.12, 0.08),
            v(0.08, 0.16),
            v(0.065, 0.3),
            v(0.065, 0.86),
            v(0.09, 0.89),
            v(0.09, 0.94),
            v(0.0, 0.94),
        ],
        32,
    ));
    lib.decor(pillar.at(foot, Quat::IDENTITY, kind::METAL, IRON));
    // The tube looks at the top of the window, turned toward it.
    let d = (frame.at(-0.3, RoomFrame::O_FAR, 4.4) - pivot).normalize();
    let axle = d.cross(up).normalize();
    // The fork turns on the pillar's head, the tube tips in the fork: each swings
    // slowly, out of step with the other.
    let sweep = Motion::Swing {
        axis: up,
        amp: 0.09,
        period: 71.0,
    };
    let fork = lib.mover(pivot, sweep, 0.0, None);
    let stick = lib.shape(Mesh::cylinder(16));
    let collar = stick.sized(Vec3::new(0.2, 0.06, 0.2));
    lib.moving(fork, collar.inst(pivot - up * 0.22, Quat::IDENTITY, kind::METAL, IRON));
    let turn = Quat::from_rotation_arc(across, axle);
    for s in [-1.0f32, 1.0] {
        let c = pivot - up * 0.1 + axle * s * 0.24;
        lib.moving(
            fork,
            Inst::new(c, turn, frame.size(0.04, 0.42, 0.2), kind::METAL, IRON, 0),
        );
    }
    lib.moving(
        fork,
        bar(stick, pivot - axle * 0.3, pivot + axle * 0.3, 0.05, kind::METAL, IRON),
    );
    let tip = Motion::Swing {
        axis: axle,
        amp: 0.02,
        period: 43.0,
    };
    let scope = lib.mover(pivot, tip, 0.0, Some(fork));
    // Tapering a little toward the eye, along the tube's own y from the pivot.
    let tube = lib.shape(Mesh::lathe(
        &[v(0.0, -1.3), v(0.13, -1.3), v(0.15, 1.65), v(0.0, 1.65)],
        40,
    ));
    let rot = Quat::from_rotation_arc(Vec3::Y, d);
    lib.moving(scope, tube.at(pivot, rot, kind::METAL, SCOPE_BRASS));
    let bands = [0.36, 0.23, 0.07, 1.0];
    let ring = |t: f32, w: f32, len: f32, rgba: [f32; 4]| {
        let at = pivot + d * t;
        stick.sized(Vec3::new(w, len, w)).inst(at, rot, kind::METAL, rgba)
    };
    for (t, w) in [(-1.0f32, 0.31), (0.35, 0.335), (1.55, 0.35)] {
        lib.moving(scope, ring(t, w, 0.08, bands));
    }
    // The dew cap, the eyepiece in its drawtube, and the finder alongside on brackets.
    lib.moving(scope, ring(1.69, 0.33, 0.08, IRON));
    lib.moving(scope, ring(-1.36, 0.09, 0.12, IRON));
    lib.moving(scope, ring(-1.45, 0.06, 0.08, SCOPE_BRASS));
    let above = axle.cross(d).normalize();
    let finder = pivot + above * 0.24;
    lib.moving(
        scope,
        bar(
            stick,
            finder - d * 0.2,
            finder + d * 0.7,
            0.06,
            kind::METAL,
            SCOPE_BRASS,
        ),
    );
    for t in [-0.1f32, 0.55] {
        lib.moving(
            scope,
            bar(
                stick,
                pivot + d * t + above * 0.13,
                pivot + d * t + above * 0.21,
                0.02,
                kind::METAL,
                IRON,
            ),
        );
    }
}
