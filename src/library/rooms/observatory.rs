//! The Observatory: department 0, ground floor, north side. A court under a painted
//! night sky: two ranges of cases cross the room, each split by a nave that runs from
//! the door to a great brass telescope aimed out of a tall window in the far wall, and
//! an orrery stands in the court between them. Cases line the walls round about.
//!
//! Each category filed whole traces a constellation over it in gold. The room in order
//! wakes its sky: shooting stars, the zodiac's houses lit one by one, the orrery's sun
//! brighter and its planets' paths drawn out round it, light along the meridian in the
//! floor, and a new star in the window where the telescope looks.

use super::*;
use crate::audio::Sfx;
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
/// Gold light: the traced constellations, the zodiac's houses, the meridian.
const GOLD_LIGHT: [f32; 4] = [1.0, 0.66, 0.28, 1.4];

pub(super) fn furnish(lib: &mut Library, frame: &RoomFrame, rng: &mut StdRng, titles: &mut HashSet<String>) {
    shelves(lib, frame, rng, titles);
    walls(lib, frame);
    sky(lib, frame);
    orrery(lib, frame);
    telescope(lib, frame);
    // After the category's fanfare has begun, a swell with glints in it.
    lib.cue(Cue {
        finish: Finish::Room(frame.dept),
        delay: 1.0,
        sound: Sfx::Veil,
        at: frame.at(0.0, COURT, 3.0),
        volume: 0.7,
    });
}

/// Shelve `cat` across `faces` (all facing one way), left to right as you face them.
fn shelve(lib: &mut Library, rng: &mut StdRng, cat: usize, mut faces: Vec<(Vec3, Vec3)>, titles: &mut HashSet<String>) {
    let right = Vec3::Y.cross(faces[0].1);
    faces.sort_by(|p, q| p.0.dot(right).total_cmp(&q.0.dot(right)));
    lib.add_category(rng, cat, &faces, titles);
}

/// The two ranges across the room (three cases either side of the nave, a category
/// on each face of each), with a plaque on the nave's side of each half saying what
/// is down either side of it; and cases on the walls: four down each side, two either
/// side of the far window and two either side of the door.
fn shelves(lib: &mut Library, frame: &RoomFrame, rng: &mut StdRng, titles: &mut HashSet<String>) {
    let mut cat = frame.categories.start;
    let across = !frame.out_along_x();
    for o in [RANGE_NEAR, RANGE_FAR] {
        let first = lib.faces.len();
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
        let n = near.len();
        for faces in [near, far] {
            shelve(lib, rng, cat, faces, titles);
            cat += 1;
        }
        // Each face's half on side `s` of the nave (its faces run left to right, so
        // the half is together).
        let middle = frame.at(0.0, o, 0.0);
        for s in [-1.0f32, 1.0] {
            let half = |faces: Range<usize>| {
                let on: Vec<usize> = faces
                    .filter(|&i| (lib.faces[i].center - middle).dot(frame.across()) * s > 0.0)
                    .collect();
                on[0]..on[on.len() - 1] + 1
            };
            let sides = [half(first..first + n), half(first + n..lib.faces.len())];
            let end = frame.at(s * (NAVE + UNIT_PITCH * 0.5), o, 0.0);
            lib.range_end(end, -frame.across() * s, sides);
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
    let mut zodiac = Mesh::ring(&circle(6.46, 128), &circle(6.34, 128), 0.05);
    zodiac.add(
        &Mesh::ring(&circle(5.93, 128), &circle(5.87, 128), 0.035),
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
    let room = Finish::Room(frame.dept);
    // Once the room is in order its houses light one by one round the ring, a gem under
    // each boss, and turn with it.
    let gem = lib.shape(Mesh::sphere(16)).sized(Vec3::splat(0.1));
    for k in 0..12 {
        let t = (k as f32 + 0.5) / 12.0 * TAU;
        let p = centre + (u * libm::cosf(t) + v * libm::sinf(t)) * 6.13 - Vec3::Y * 0.055;
        let house = lib.mover(p, Motion::Still, 0.0, Some(m));
        lib.awaiting(
            house,
            Awaits::new(room, grows(0.0)).after(1.0 + 0.45 * k as f32).over(0.8),
        );
        let warm = [GOLD_LIGHT[0], GOLD_LIGHT[1], GOLD_LIGHT[2], 2.5];
        lib.moving(house, gem.inst(p, Quat::IDENTITY, kind::EMISSIVE, warm));
    }
    let line = [0.5, 0.42, 0.25, 0.35];
    for f in &FIGURES {
        let at = |s: &(f32, f32, f32)| frame.at(s.0, s.1, y - 0.01);
        for &(i, j) in f.joins {
            let (p, q) = (at(&f.stars[i]), at(&f.stars[j]));
            rod(lib, p, q, 0.012, 0.012, kind::EMISSIVE, line);
        }
        for s in f.stars {
            star(lib, at(s), 0.03 + 0.03 * s.2, 2.5 + 4.5 * s.2);
        }
        match f.by {
            Some(c) => trace(lib, frame, f, Finish::Category(frame.categories.start + c), 0.8),
            None => trace(lib, frame, f, room, 2.0),
        }
    }
    // A shower of shooting stars across the sky when the room is in order, from either
    // side and from the door's end; and after it, one now and then.
    let meteor = |from: Vec3, toward: Vec3, spread: Vec3| Sparks {
        at: from,
        spread,
        rate: 1.6,
        vel: toward * 6.5,
        jitter: toward.abs() * 1.5,
        lift: 0.0,
        drag: 0.3,
        life: 1.5,
        rgba: [0.85, 0.9, 1.0, 6.0],
        size: (toward.abs() * 0.7).max(Vec3::splat(0.02)),
        ..Sparks::burst(room, from)
    };
    let low = h - 0.18;
    let athwart = frame.size(2.0, 0.05, 6.0);
    for (from, toward, spread, delay) in [
        (frame.at(-7.0, COURT, low), u, athwart, 1.5),
        (frame.at(7.0, COURT, low), -u, athwart, 2.4),
        (frame.at(0.0, 17.5, low), v, frame.size(6.0, 0.05, 1.5), 3.2),
    ] {
        lib.sparks(Sparks {
            delay,
            lasts: Some(6.0),
            ..meteor(from, toward, spread)
        });
    }
    for (from, toward, spread, rate) in [
        (frame.at(-7.0, COURT, low), u, athwart, 0.05),
        (frame.at(0.0, 17.5, low), v, frame.size(6.0, 0.05, 1.5), 0.04),
    ] {
        lib.sparks(Sparks {
            delay: 9.0,
            lasts: None,
            rate,
            ..meteor(from, toward, spread)
        });
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

/// A constellation on the painted sky: its stars in the room's frame (a, o) with their
/// brightness, the stars joined (in the order they are traced), and which of the
/// room's categories traces it in gold when filed whole, over where that category
/// stands (`None`: the room itself).
struct Figure {
    stars: &'static [(f32, f32, f32)],
    joins: &'static [(usize, usize)],
    by: Option<usize>,
}

const FIGURES: [Figure; 10] = [
    // The Crown, over the near range's door side.
    Figure {
        stars: &[
            (3.6, 15.4, 0.55),
            (3.8, 15.95, 0.6),
            (4.25, 16.3, 0.7),
            (4.75, 16.4, 1.0),
            (5.2, 16.25, 0.65),
            (5.5, 15.9, 0.55),
            (5.6, 15.4, 0.5),
        ],
        joins: &[(0, 1), (1, 2), (2, 3), (3, 4), (4, 5), (5, 6)],
        by: Some(0),
    },
    // The Plough.
    Figure {
        stars: &[
            (-3.0, 20.0, 1.0),
            (-3.0, 21.0, 0.8),
            (-1.7, 21.2, 0.7),
            (-1.5, 20.2, 0.6),
            (-0.4, 19.9, 0.8),
            (0.6, 19.6, 0.9),
            (1.7, 19.9, 0.8),
        ],
        joins: &[(6, 5), (5, 4), (4, 3), (3, 0), (0, 1), (1, 2), (2, 3)],
        by: Some(1),
    },
    // Cassiopeia.
    Figure {
        stars: &[
            (1.4, 26.6, 0.9),
            (2.2, 25.9, 0.8),
            (3.0, 26.5, 1.0),
            (3.9, 25.8, 0.7),
            (4.6, 26.6, 0.8),
        ],
        joins: &[(0, 1), (1, 2), (2, 3), (3, 4)],
        by: Some(2),
    },
    // The Hunter.
    Figure {
        stars: &[
            (-4.2, 27.4, 1.0),
            (-2.2, 27.6, 0.8),
            (-3.6, 26.2, 0.7),
            (-3.25, 26.05, 0.7),
            (-2.9, 25.9, 0.7),
            (-4.4, 24.6, 0.8),
            (-2.0, 24.8, 1.0),
            (-3.2, 28.3, 0.5),
        ],
        joins: &[(7, 0), (7, 1), (0, 2), (1, 4), (2, 3), (3, 4), (2, 5), (4, 6)],
        by: Some(3),
    },
    // The Eagle, over the far wall.
    Figure {
        stars: &[
            (0.0, 30.65, 1.0),
            (-0.4, 30.95, 0.6),
            (0.4, 30.35, 0.55),
            (-1.4, 30.2, 0.6),
            (-2.5, 30.45, 0.5),
            (1.5, 31.05, 0.6),
            (2.6, 30.85, 0.5),
        ],
        joins: &[(1, 0), (0, 2), (0, 3), (3, 4), (0, 5), (5, 6)],
        by: Some(4),
    },
    // The Lion, along the right wall: the sickle, then the body to its tail.
    Figure {
        stars: &[
            (9.0, 21.0, 1.0),
            (8.9, 21.8, 0.5),
            (9.3, 22.6, 0.85),
            (9.1, 23.4, 0.5),
            (8.5, 23.75, 0.45),
            (8.0, 23.25, 0.55),
            (10.4, 23.3, 0.55),
            (10.6, 24.5, 0.6),
            (11.3, 25.9, 0.8),
        ],
        joins: &[(0, 1), (1, 2), (2, 3), (3, 4), (4, 5), (2, 7), (7, 8), (8, 6), (6, 0)],
        by: Some(5),
    },
    // The River, winding along the near wall.
    Figure {
        stars: &[
            (-2.6, 15.2, 0.6),
            (-3.8, 15.6, 0.5),
            (-4.9, 15.3, 0.55),
            (-6.0, 15.8, 0.5),
            (-6.8, 16.6, 0.6),
            (-7.9, 16.9, 0.5),
            (-9.0, 16.5, 0.55),
            (-10.2, 16.9, 0.5),
            (-11.2, 17.7, 0.9),
        ],
        joins: &[(0, 1), (1, 2), (2, 3), (3, 4), (4, 5), (5, 6), (6, 7), (7, 8)],
        by: Some(6),
    },
    // The Scorpion, along the left wall: its claws, its heart and its tail.
    Figure {
        stars: &[
            (-8.6, 19.5, 0.55),
            (-9.3, 19.3, 0.6),
            (-10.1, 19.6, 0.5),
            (-9.6, 20.7, 1.0),
            (-9.8, 21.7, 0.6),
            (-10.1, 22.7, 0.55),
            (-10.7, 23.7, 0.6),
            (-11.2, 24.7, 0.6),
            (-11.1, 25.7, 0.55),
            (-10.4, 26.3, 0.6),
            (-9.6, 26.05, 0.7),
            (-9.35, 25.5, 0.45),
        ],
        joins: &[
            (0, 1),
            (2, 1),
            (1, 3),
            (3, 4),
            (4, 5),
            (5, 6),
            (6, 7),
            (7, 8),
            (8, 9),
            (9, 10),
            (10, 11),
        ],
        by: Some(7),
    },
    // The Swan and the Sisters, in the court: the room's own.
    Figure {
        stars: &[
            (3.2, 22.0, 1.0),
            (3.2, 23.1, 0.7),
            (3.2, 24.3, 0.6),
            (2.0, 23.0, 0.6),
            (4.4, 23.2, 0.6),
        ],
        joins: &[(0, 1), (1, 2), (3, 1), (1, 4)],
        by: None,
    },
    Figure {
        stars: &[
            (-5.0, 22.4, 0.6),
            (-4.8, 22.2, 0.5),
            (-4.9, 22.0, 0.6),
            (-5.15, 22.15, 0.45),
            (-4.65, 22.45, 0.45),
            (-5.1, 22.6, 0.4),
        ],
        joins: &[],
        by: None,
    },
];

/// Figure `f` traced in gold once `fin` is finished, from `start` seconds after: each
/// line drawn out from its first star to its second in turn, a little under the
/// painted one, and dust glinting from each star as the gold reaches it (from each in
/// turn, for a figure with no lines). It stays traced.
fn trace(lib: &mut Library, frame: &RoomFrame, f: &Figure, fin: Finish, start: f32) {
    const STEP: f32 = 0.45;
    let at = |s: (f32, f32, f32)| frame.at(s.0, s.1, RoomFrame::HEIGHT - 0.086);
    let glint = |lib: &mut Library, p: Vec3, delay: f32| {
        lib.sparks(Sparks {
            delay,
            lasts: Some(0.25),
            rate: 40.0,
            spread: Vec3::splat(0.03),
            vel: Vec3::ZERO,
            jitter: Vec3::splat(0.2),
            lift: -0.15,
            life: 1.2,
            rgba: [1.0, 0.9, 0.65, 3.0],
            size: Vec3::splat(0.014),
            ..Sparks::burst(fin, p)
        });
    };
    let mut reached = vec![false; f.stars.len()];
    for (k, &(i, j)) in f.joins.iter().enumerate() {
        let (p, q) = (at(f.stars[i]), at(f.stars[j]));
        let delay = start + k as f32 * STEP;
        let along = Quat::from_rotation_arc(Vec3::Y, (q - p).normalize());
        let line = Inst::new(
            (p + q) * 0.5,
            along,
            Vec3::new(0.018, p.distance(q), 0.018),
            kind::EMISSIVE,
            GOLD_LIGHT,
            0,
        );
        lib.waiting(p, Awaits::new(fin, grows(0.0)).after(delay).over(STEP * 1.3), [line]);
        for (s, when) in [(i, delay), (j, delay + STEP * 1.1)] {
            if !reached[s] {
                reached[s] = true;
                glint(lib, at(f.stars[s]), when);
            }
        }
    }
    if f.joins.is_empty() {
        for (k, &s) in f.stars.iter().enumerate() {
            glint(lib, at(s), start + k as f32 * STEP * 0.5);
        }
    }
}

/// Coming in where it rests, growing from `grow` of its size about its pivot.
fn grows(grow: f32) -> Change {
    Change::Arrives { from: Vec3::ZERO, grow }
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
    // Once the room is in order, light comes out from under the drum round the ring and
    // runs out along the line to the door and the window, let into the brass's top, and
    // stays.
    let room = Finish::Room(frame.dept);
    let lit = [GOLD_LIGHT[0], GOLD_LIGHT[1], GOLD_LIGHT[2], 0.6];
    let rim = base + up * 0.00825;
    let ring = band(lib, rim, flat, (1.25, 0.02, 0.0025), kind::EMISSIVE, lit);
    lib.waiting(rim, Awaits::new(room, grows(0.6)).over(1.2), [ring]);
    for (o0, o1, from) in [
        (RoomFrame::O_NEAR + 1.0, COURT - 1.25, COURT - 1.25),
        (COURT + 1.25, RoomFrame::O_FAR - 0.3, COURT + 1.25),
    ] {
        let mut strip = piece(frame, (-0.015, 0.015), (o0, o1), (0.006, 0.0085), kind::EMISSIVE, gold);
        strip.color = lit;
        let pivot = frame.at(0.0, from, 0.00725);
        lib.waiting(pivot, Awaits::new(room, grows(0.0)).after(0.6).over(2.5), [strip]);
    }
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
    // Burning brighter once the room is in order, its light with it.
    let (glow, was) = (2.0, 1.4);
    let flares = Awaits::new(room, Change::Kindles { dim: was / glow })
        .after(0.3)
        .over(3.0);
    let blaze = ball
        .sized(Vec3::splat(0.24))
        .inst(sun, Quat::IDENTITY, kind::CRYSTAL, [1.0, 0.72, 0.3, glow]);
    let burns = lib.waiting(sun, flares, [blaze]);
    // Its light a little above it, so it lights the court rather than flooding itself.
    lib.light_riding(
        burns,
        sun + up * 0.7,
        5.5,
        [1.0, 0.7, 0.35],
        0.9 * glow / was,
        LightKind::Static,
    );
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
    // Each world's path, drawn out from the sun in faint gold once the room is in
    // order, the nearest first.
    for (k, &(_, r, h, s, _, _)) in planets.iter().enumerate() {
        let c = base + up * (h + 0.14 + s * 0.5);
        let path = band(lib, c, flat, (r, 0.008, 0.004), kind::EMISSIVE, [1.0, 0.82, 0.5, 0.45]);
        let drawn = Awaits::new(room, grows(0.0)).after(0.6 + 0.45 * k as f32).over(1.8);
        lib.waiting(c, drawn, [path]);
    }
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
        lib.moving(
            arm,
            stick.between(hub, tip + dir * 0.01, 0.02, kind::METAL, BRASS_BRIGHT),
        );
        lib.moving(
            arm,
            stick.between(tip, tip + up * 0.14, 0.016, kind::METAL, BRASS_BRIGHT),
        );
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
            lib.moving(moon, stick.between(p, m, 0.01, kind::METAL, BRASS_BRIGHT));
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
        stick.between(pivot - axle * 0.3, pivot + axle * 0.3, 0.05, kind::METAL, IRON),
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
        stick.between(finder - d * 0.2, finder + d * 0.7, 0.06, kind::METAL, SCOPE_BRASS),
    );
    for t in [-0.1f32, 0.55] {
        lib.moving(
            scope,
            stick.between(
                pivot + d * t + above * 0.13,
                pivot + d * t + above * 0.21,
                0.02,
                kind::METAL,
                IRON,
            ),
        );
    }
    // What it was looking for: once the room is in order, a new star in the window where
    // it looks, coming out of the night and glinting there for good.
    let room = Finish::Room(frame.dept);
    let found = frame.at(-0.3, RoomFrame::O_FAR, 4.4) - frame.out() * 0.046;
    let starlight = [1.0, 0.92, 0.75, 7.0];
    let face = frame.turn();
    let glints = [
        Inst::new(found, face, Vec3::new(0.36, 0.012, 0.004), kind::EMISSIVE, starlight, 1),
        Inst::new(found, face, Vec3::new(0.012, 0.36, 0.004), kind::EMISSIVE, starlight, 1),
        Inst::new(
            found - frame.out() * 0.002,
            face * Quat::from_rotation_z(PI / 4.0),
            Vec3::new(0.06, 0.06, 0.004),
            kind::EMISSIVE,
            starlight,
            1,
        ),
    ];
    lib.waiting(found, Awaits::new(room, grows(0.0)).after(5.5).over(2.5), glints);
    lib.sparks(Sparks {
        delay: 6.0,
        lasts: Some(0.6),
        rate: 30.0,
        spread: Vec3::splat(0.04),
        vel: Vec3::ZERO,
        jitter: Vec3::splat(0.25),
        lift: -0.1,
        rgba: [0.85, 0.9, 1.0, 3.0],
        ..Sparks::burst(room, found - frame.out() * 0.1)
    });
}
