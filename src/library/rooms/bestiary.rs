//! The Bestiary: department 3, ground floor, west side. A long hall runs from the
//! door end to the far wall under the hung skeleton of a dragon; either side of it
//! two cages of cases open onto the hall, iron-barred overhead, their gates standing
//! open, and cases line the walls round about under mounted antlers. An egg lies in
//! a nest at the hall's far end.

use super::*;
use std::f32::consts::PI;

/// How deep a wall case stands from its back.
const WALL_CASE: f32 = 0.04 + FACE_DEPTH;
/// The cages: their middles (o), how far across their mouths are (|a|), half their
/// clear width, and how far their back cases stand from the mouth.
const CAGES_O: [f32; 2] = [20.5, 26.6];
const MOUTH: f32 = 3.0;
const CAGE_HALF: f32 = 1.4;
const CAGE_DEEP: f32 = 2.0 * UNIT_PITCH + 0.15;
/// The egg's plinth (o) at the far end of the hall.
const EGG_O: f32 = 27.9;

const IRON: [f32; 4] = [0.07, 0.07, 0.08, 1.0];
const BONE: [f32; 4] = [0.7, 0.66, 0.56, 1.0];

pub(super) fn furnish(lib: &mut Library, frame: &RoomFrame, rng: &mut StdRng, titles: &mut HashSet<String>) {
    shelves(lib, frame, rng, titles);
    walls(lib, frame);
    skeleton(lib, frame);
    egg(lib, frame);
}

/// Four cages of five, two either side of the hall (read from the mouth on the left,
/// round the back and out), and the walls' cases: five on each.
fn shelves(lib: &mut Library, frame: &RoomFrame, rng: &mut StdRng, titles: &mut HashSet<String>) {
    let (out, across) = (frame.out(), frame.across());
    let mut cat = frame.categories.start;
    let wall = |lib: &mut Library, a: f32, o: f32, n: Vec3| lib.wall_bookcase(frame.at(a, o, 0.0), n);
    let run = |first: f32| (0..5).map(move |k| first + UNIT_PITCH * (k as f32 + 0.5));
    let (inner, near, far) = (RoomFrame::A_HALF, RoomFrame::O_NEAR, RoomFrame::O_FAR);
    let left: Vec<_> = run(17.0).map(|o| wall(lib, -inner, o, across)).collect();
    lib.add_category(rng, cat, &left_to_right(left), titles);
    cat += 1;
    for s in [-1.0f32, 1.0] {
        let order: &[f32] = if s < 0.0 { &CAGES_O } else { &[CAGES_O[1], CAGES_O[0]] };
        for &o in order {
            let faces = cage(lib, frame, s, o);
            lib.add_category(rng, cat, &faces, titles);
            cat += 1;
        }
        if s < 0.0 {
            let far_faces: Vec<_> = run(-6.75).map(|a| wall(lib, a, far, -out)).collect();
            lib.add_category(rng, cat, &left_to_right(far_faces), titles);
            cat += 1;
        }
    }
    let right: Vec<_> = run(16.75).map(|o| wall(lib, inner, o, -across)).collect();
    lib.add_category(rng, cat, &left_to_right(right), titles);
    let near_faces: Vec<_> = run(-4.9).map(|a| wall(lib, a, near, out)).collect();
    lib.add_category(rng, cat + 1, &left_to_right(near_faces), titles);
}

/// A cage on side `s` of the hall about `o`, open to the hall: two single-sided cases
/// down each side facing in, one across the back, panelled behind, roofed with iron
/// bars, its gates swung open against its sides. Returns its faces in reading order.
fn cage(lib: &mut Library, frame: &RoomFrame, s: f32, o: f32) -> Vec<(Vec3, Vec3)> {
    let out = frame.out();
    let back_face = MOUTH + CAGE_DEEP;
    let back = back_face + WALL_CASE;
    let side_back = CAGE_HALF + WALL_CASE;
    let (mut near, mut far) = (Vec::new(), Vec::new());
    for x in [MOUTH + UNIT_PITCH * 0.5, MOUTH + UNIT_PITCH * 1.5] {
        near.push(lib.wall_bookcase(frame.at(s * x, o - side_back, 0.0), out));
        far.push(lib.wall_bookcase(frame.at(s * x, o + side_back, 0.0), -out));
    }
    let end = lib.wall_bookcase(frame.at(s * back, o, 0.0), -frame.across() * s);
    // Panelling over the backs, which are seen from outside; the corners behind the
    // back case closed in, and in the way.
    let top = UNIT_H + 0.13;
    let wood = [0.13, 0.065, 0.03];
    for e in [-1.0f32, 1.0] {
        let (o0, o1) = (o + e * side_back, o + e * (side_back + 0.05));
        lib.decor(piece(
            frame,
            (s * MOUTH, s * back),
            (o0.min(o1), o0.max(o1)),
            (0.0, top),
            kind::WOOD,
            wood,
        ));
        let (o0, o1) = (o + e * (CAGE_HALF - 0.08), o + e * side_back);
        let (lo, hi) = frame.boxed(
            (s * (back_face - 0.2)).min(s * back),
            (s * (back_face - 0.2)).max(s * back),
            o0.min(o1),
            o0.max(o1),
            0.0,
            top,
        );
        lib.collide(lo, hi, Stops::ALL);
    }
    let rear = (o - side_back - 0.05, o + side_back + 0.05);
    lib.decor(piece(
        frame,
        (s * back, s * (back + 0.05)),
        rear,
        (0.0, top),
        kind::WOOD,
        wood,
    ));
    // The mouth: iron posts at its corners and a bar over it.
    let edge = side_back + 0.03;
    for e in [-1.0f32, 1.0] {
        let p = frame.at(s * (MOUTH - 0.05), o + e * edge, 0.0);
        rod(lib, p, p + Vec3::Y * 3.4, 0.09, 0.09, kind::METAL, IRON);
    }
    let lintel = |e: f32| frame.at(s * (MOUTH - 0.05), o + e * edge, 3.32);
    rod(lib, lintel(-1.0), lintel(1.0), 0.07, 0.07, kind::METAL, IRON);
    // The roof: ribs arched across, and bars along.
    let arch = |x: f32, t: f32| frame.at(s * x, o - edge * libm::cosf(t), top + 0.02 + 1.0 * libm::sinf(t));
    let steps = 6;
    for k in 0..5 {
        let x = MOUTH + 0.1 + k as f32 * (back + 0.02 - MOUTH - 0.1) / 4.0;
        for j in 0..steps {
            let (t0, t1) = (j as f32 / steps as f32 * PI, (j + 1) as f32 / steps as f32 * PI);
            rod(lib, arch(x, t0), arch(x, t1), 0.035, 0.035, kind::METAL, IRON);
        }
    }
    for t in [PI * 0.25, PI * 0.5, PI * 0.75] {
        rod(
            lib,
            arch(MOUTH + 0.05, t),
            arch(back + 0.05, t),
            0.03,
            0.03,
            kind::METAL,
            IRON,
        );
    }
    // The gates, swung open flat against the outsides of the sides.
    for e in [-1.0f32, 1.0] {
        let g = |x: f32, y: f32| frame.at(s * x, o + e * (side_back + 0.09), y);
        let (x0, x1) = (MOUTH + 0.02, MOUTH + 1.35);
        for y in [0.15f32, 2.45] {
            rod(lib, g(x0, y), g(x1, y), 0.05, 0.05, kind::METAL, IRON);
        }
        for k in 0..6 {
            let x = x0 + k as f32 * (x1 - x0) / 5.0;
            rod(lib, g(x, 0.1), g(x, 2.55), 0.03, 0.03, kind::METAL, IRON);
        }
    }
    // Bars round the outside, past the gates: down both sides and across the back.
    let bar = |lib: &mut Library, p: Vec3| {
        rod(
            lib,
            p + Vec3::Y * 0.05,
            p + Vec3::Y * (top + 0.02),
            0.03,
            0.03,
            kind::METAL,
            IRON,
        )
    };
    let outside = side_back + 0.11;
    let (x0, x1) = (MOUTH + 1.5, back + 0.11);
    for e in [-1.0f32, 1.0] {
        let n = 15;
        for k in 0..n {
            let x = x0 + (x1 - 0.1 - x0) * k as f32 / (n - 1) as f32;
            bar(lib, frame.at(s * x, o + e * outside, 0.0));
        }
        for y in [0.15f32, top - 0.05] {
            rod(
                lib,
                frame.at(s * x0, o + e * outside, y),
                frame.at(s * x1, o + e * outside, y),
                0.04,
                0.04,
                kind::METAL,
                IRON,
            );
        }
    }
    for k in 0..12 {
        let oo = o - outside + 0.25 + (2.0 * outside - 0.5) * k as f32 / 11.0;
        bar(lib, frame.at(s * x1, oo, 0.0));
    }
    // A little higher than the sides' rails, which they cross at the corners.
    for y in [0.17f32, top - 0.03] {
        rod(
            lib,
            frame.at(s * x1, o - outside, y),
            frame.at(s * x1, o + outside, y),
            0.04,
            0.04,
            kind::METAL,
            IRON,
        );
    }
    // A sconce on the near post, lighting the hall.
    lib.sconce(frame.at(s * (MOUTH - 0.2), o - edge, 2.6), -frame.across() * s);
    let (first, last) = if s < 0.0 { (near, far) } else { (far, near) };
    let mut faces = left_to_right(first);
    faces.push(end);
    faces.extend(left_to_right(last));
    faces
}

/// Panelling on the bare walls, windows in the far corners with specimen jars on
/// their sills, antlers mounted over the side walls' cases, a runner down the hall,
/// and beams across.
fn walls(lib: &mut Library, frame: &RoomFrame) {
    let (inner, near, far) = (RoomFrame::A_HALF, RoomFrame::O_NEAR, RoomFrame::O_FAR);
    let t = 0.07;
    let side = frame.side;
    let d = frame.door;
    for (a0, a1) in [(-inner + t, d - 1.85), (d + 1.85, -4.95), (8.65, inner - t)] {
        lib.wainscot(frame.y, frame.rect(a0, a1, near, near + t), side.out);
    }
    lib.wainscot(frame.y, frame.rect(-inner, -inner + t, near, 16.95), side.across);
    lib.wainscot(frame.y, frame.rect(inner - t, inner, near, 16.7), -side.across);
    for (a0, a1) in [(-inner + t, -6.8), (6.8, inner - t)] {
        lib.wainscot(frame.y, frame.rect(a0, a1, far - t, far), -side.out);
    }
    // Tall windows in the far corners, moonlight falling through.
    let out = frame.out();
    for s in [-1.0f32, 1.0] {
        let (a0, a1) = (s * 8.8, s * 10.4);
        // The glass is leaded across its own x, so it is turned to lie along the wall.
        let (lo, hi) = frame.boxed(a0.min(a1), a0.max(a1), far - 0.04, far + 0.04, 1.26, 4.5);
        let size = hi - lo;
        let (rot, size) = if size.z > size.x {
            (
                Quat::from_rotation_y(std::f32::consts::FRAC_PI_2),
                Vec3::new(size.z, size.y, size.x),
            )
        } else {
            (Quat::IDENTITY, size)
        };
        lib.decor(Inst::new(
            (lo + hi) * 0.5,
            rot,
            size,
            kind::GLASS,
            [0.32, 0.42, 0.85, 1.1],
            0,
        ));
        // The sill sits on the panelling's cap.
        let stone = [0.48, 0.45, 0.4];
        lib.decor(piece(
            frame,
            (s * 8.6, s * 10.6),
            (far - 0.2, far),
            (1.16, 1.26),
            kind::STONE,
            stone,
        ));
        lib.decor(piece(
            frame,
            (s * 8.6, s * 10.6),
            (far - 0.1, far),
            (4.5, 4.75),
            kind::STONE,
            stone,
        ));
        let c = frame.at(s * 9.6, far - 0.05, 2.8);
        lib.light(c - out * 1.5, 8.0, [0.45, 0.55, 1.0], 0.8, LightKind::Static);
        let beam = (-out - Vec3::Y * 0.6).normalize();
        let len = 7.0;
        lib.static_trans.push(Inst::new(
            c + beam * len * 0.5,
            Quat::from_rotation_arc(Vec3::Y, beam),
            Vec3::new(1.6, len, 1.6),
            kind::SHAFT,
            [0.35, 0.45, 0.9, 0.04],
            0,
        ));
    }
    // Antlers over the end cases of each side wall, clear of the signs in the middle.
    for (s, os) in [(-1.0f32, [18.35f32, 29.15]), (1.0, [18.1, 28.9])] {
        for (k, o) in os.into_iter().enumerate() {
            antlers(lib, frame, s, o, k);
        }
    }
    for o in [15.6f32, 31.0] {
        lib.sconce(frame.at(inner - 0.15, o, 2.6), -frame.across());
    }
    lib.sconce(frame.at(-inner + 0.15, 31.0, 2.6), frame.across());
    for s in [-1.0f32, 1.0] {
        specimens(lib, frame, s);
    }
    // A runner down the hall.
    let mut rug = piece(
        frame,
        (-1.7, 1.7),
        (16.6, 29.6),
        (0.0, 0.012),
        kind::RUG,
        [0.28, 0.06, 0.05],
    );
    rug.param = 120 | (95 << 8) | (40 << 16);
    lib.rug(rug);
    for k in 0..6 {
        let o = near + 1.6 + k as f32 * 3.0;
        let h = RoomFrame::HEIGHT;
        lib.decor(piece(
            frame,
            (-inner, inner),
            (o - 0.13, o + 0.13),
            (h - 0.3, h),
            kind::WOOD,
            [0.09, 0.045, 0.022],
        ));
    }
}

/// A pair of antlers on a shield, mounted high on side wall `s` at `o`.
fn antlers(lib: &mut Library, frame: &RoomFrame, s: f32, o: f32, k: usize) {
    let n = -frame.across() * s;
    let wall = frame.at(s * RoomFrame::A_HALF, o, 4.2);
    let shield = wall + n * 0.04;
    let face = Quat::from_rotation_arc(Vec3::Z, n);
    lib.decor(Inst::new(
        shield,
        face * Quat::from_rotation_z(0.785),
        Vec3::new(0.42, 0.42, 0.08),
        kind::WOOD,
        [0.2, 0.1, 0.045, 1.0],
        0,
    ));
    let skull = shield + n * 0.12 - Vec3::Y * 0.05;
    lib.decor(Inst::new(skull, face, Vec3::new(0.16, 0.22, 0.2), kind::PLAIN, BONE, 0));
    let up = Vec3::Y;
    let along = Vec3::Y.cross(n);
    let spread = if k == 0 { 1.0 } else { 0.8 };
    for e in [-1.0f32, 1.0] {
        let side = along * e;
        let base = skull + up * 0.1 + side * 0.06;
        let p1 = base + side * 0.35 * spread + up * 0.25 + n * 0.08;
        let p2 = p1 + side * 0.2 * spread + up * 0.35 - n * 0.02;
        let p3 = p2 - side * 0.05 + up * 0.3;
        for (a, b) in [(base, p1), (p1, p2), (p2, p3)] {
            rod(lib, a, b, 0.045, 0.045, kind::PLAIN, BONE);
        }
        for (from, dir) in [
            (p1, up * 0.25 - side * 0.08),
            (p2, up * 0.22 + side * 0.1),
            (p2, up * 0.15 - side * 0.12 + n * 0.08),
        ] {
            rod(lib, from, from + dir, 0.03, 0.03, kind::PLAIN, BONE);
        }
    }
}

/// Specimens in jars along the sill of the far window on side `s`, dark against the
/// moonlit glass.
fn specimens(lib: &mut Library, frame: &RoomFrame, s: f32) {
    let o = RoomFrame::O_FAR - 0.11;
    let mut rng = StdRng::seed_from_u64(0xbe57_1a77 ^ (s > 0.0) as u64);
    let mut x = 8.75;
    while x < 10.3 {
        let w = rng.gen_range(0.11..0.15f32);
        let h = w * rng.gen_range(1.3..1.9f32);
        let foot = frame.at(s * (x + w * 0.5), o, 1.26);
        let murk = [[0.32, 0.4, 0.18], [0.45, 0.38, 0.15], [0.25, 0.32, 0.3]][rng.gen_range(0..3)];
        octo(
            lib,
            foot + Vec3::Y * (h * 0.5),
            w,
            h,
            kind::CRYSTAL,
            [murk[0], murk[1], murk[2], 0.08],
        );
        octo(
            lib,
            foot + Vec3::Y * (h + 0.02),
            w * 0.85,
            0.04,
            kind::METAL,
            [0.3, 0.3, 0.28, 1.0],
        );
        x += w + rng.gen_range(0.06..0.14);
    }
}

/// The skeleton of a dragon hung on chains along the hall, its head toward the door,
/// a dull light in its eyes: spine, ribs, legs, the bones of its wings, and its tail.
fn skeleton(lib: &mut Library, frame: &RoomFrame) {
    let (u, v, up) = (frame.across(), frame.out(), Vec3::Y);
    // The spine through (o, height, sway across), from the back of the skull to the
    // tip of the tail.
    let spine: [(f32, f32, f32); 9] = [
        (18.2, 3.95, 0.0),
        (19.2, 4.25, 0.1),
        (20.3, 4.4, 0.05),
        (22.0, 4.45, -0.1),
        (23.8, 4.35, 0.0),
        (25.2, 4.2, 0.25),
        (26.6, 4.0, 0.55),
        (28.0, 3.85, 0.3),
        (29.2, 3.75, -0.3),
    ];
    let at = |o: f32, y: f32, a: f32| frame.at(a, o, y);
    let point = |t: f32| {
        let f = t * (spine.len() - 1) as f32;
        let i = (f as usize).min(spine.len() - 2);
        let (p, q) = (spine[i], spine[i + 1]);
        let w = f - i as f32;
        at(p.0 + (q.0 - p.0) * w, p.1 + (q.1 - p.1) * w, p.2 + (q.2 - p.2) * w)
    };
    let n = 46;
    for k in 0..n {
        let t = k as f32 / (n - 1) as f32;
        let p = point(t);
        let size = if t < 0.15 {
            0.13
        } else {
            0.19 - 0.15 * ((t - 0.15) / 0.85).max(0.0)
        };
        let turn = Quat::from_rotation_y(0.5 + t * 3.0) * Quat::from_rotation_x(0.3);
        lib.decor(Inst::new(p, turn, Vec3::splat(size), kind::PLAIN, BONE, 0));
        // Spines along the back, over the body.
        if (0.15..0.6).contains(&t) && k % 2 == 0 {
            rod(lib, p, p + up * 0.28 + v * 0.06, 0.035, 0.035, kind::PLAIN, BONE);
        }
    }
    // Ribs, curving down and in, longest mid-chest.
    for k in 0..8 {
        let o = 20.3 + k as f32 * 0.42;
        let t = (o - 18.2) / (29.2 - 18.2);
        let p = point(t);
        let len = 1.0 - ((k as f32 - 3.5) / 4.5).powi(2);
        // A rib's curve, (outward, down), scaled by its length.
        let curve = [(0.0f32, 0.0f32), (0.4, 0.12), (0.62, 0.45), (0.6, 0.85), (0.38, 1.12)];
        let (wide, deep) = (0.55 + 0.45 * len, 0.6 + 0.4 * len);
        for e in [-1.0f32, 1.0] {
            let at = |(x, y): (f32, f32)| p + u * (e * x * wide) - up * (y * deep);
            for w in curve.windows(2) {
                rod(lib, at(w[0]), at(w[1]), 0.04, 0.04, kind::PLAIN, BONE);
            }
        }
    }
    // The skull: brow, snout and a jaw hanging open, horns swept back, and the eyes.
    let head = at(17.75, 3.85, 0.0);
    let fwd = -v;
    let face = Quat::from_rotation_arc(Vec3::Z, fwd);
    lib.decor(Inst::new(
        head,
        face * Quat::from_rotation_x(-0.15),
        Vec3::new(0.42, 0.32, 0.46),
        kind::PLAIN,
        BONE,
        0,
    ));
    let snout = head + fwd * 0.45 - up * 0.04;
    lib.decor(Inst::new(
        snout,
        face * Quat::from_rotation_x(0.1),
        Vec3::new(0.26, 0.18, 0.55),
        kind::PLAIN,
        BONE,
        0,
    ));
    let jaw = head + fwd * 0.35 - up * 0.26;
    lib.decor(Inst::new(
        jaw,
        face * Quat::from_rotation_x(0.42),
        Vec3::new(0.24, 0.06, 0.6),
        kind::PLAIN,
        BONE,
        0,
    ));
    for e in [-1.0f32, 1.0] {
        let root = head + up * 0.12 + u * (e * 0.14) - fwd * 0.12;
        let mid = root + up * 0.3 - fwd * 0.35 + u * (e * 0.12);
        rod(lib, root, mid, 0.07, 0.07, kind::PLAIN, BONE);
        rod(
            lib,
            mid,
            mid + up * 0.08 - fwd * 0.35 + u * (e * 0.05),
            0.045,
            0.045,
            kind::PLAIN,
            BONE,
        );
        let eye = head + fwd * 0.12 + up * 0.05 + u * (e * 0.2);
        lib.decor(Inst::new(
            eye,
            face,
            Vec3::new(0.04, 0.07, 0.09),
            kind::CRYSTAL,
            [1.0, 0.55, 0.15, 1.8],
            0,
        ));
        // Teeth along the snout.
        for j in 0..3 {
            let tooth = snout + fwd * (0.1 + j as f32 * 0.12) + u * (e * 0.1) - up * 0.11;
            rod(
                lib,
                tooth,
                tooth - up * 0.08 - fwd * 0.02,
                0.025,
                0.025,
                kind::PLAIN,
                BONE,
            );
        }
    }
    // Legs, dangling.
    for (o, back) in [(20.6f32, false), (24.0, true)] {
        let t = (o - 18.2) / (29.2 - 18.2);
        let hip = point(t);
        for e in [-1.0f32, 1.0] {
            let side = u * e;
            let lean = if back { v * 0.3 } else { -v * 0.2 };
            let knee = hip + side * 0.45 - up * 0.65 + lean;
            let ankle = knee + side * 0.05 - up * 0.55 - lean * 1.2;
            let toe = ankle - up * 0.12 + fwd * 0.15;
            for (a, b) in [(hip, knee), (knee, ankle), (ankle, toe)] {
                rod(lib, a, b, 0.06, 0.06, kind::PLAIN, BONE);
            }
            for c in [-1.0f32, 0.0, 1.0] {
                rod(
                    lib,
                    toe,
                    toe + fwd * 0.16 + side * (c * 0.07) - up * 0.05,
                    0.025,
                    0.025,
                    kind::PLAIN,
                    BONE,
                );
            }
        }
    }
    // The wings, half spread: arm and forearm, and three long fingers.
    let shoulder = point(0.22);
    for e in [-1.0f32, 1.0] {
        let side = u * e;
        let elbow = shoulder + side * 1.3 + up * 0.45 + v * 0.3;
        let wrist = elbow + side * 1.2 - up * 0.1 + v * 0.55;
        for (a, b) in [(shoulder, elbow), (elbow, wrist)] {
            rod(lib, a, b, 0.07, 0.07, kind::PLAIN, BONE);
        }
        for (reach, back, drop) in [(1.1f32, 0.3f32, 0.2f32), (0.7, 1.2, 0.35), (0.2, 1.9, 0.45)] {
            rod(
                lib,
                wrist,
                wrist + side * reach + v * back - up * drop,
                0.04,
                0.04,
                kind::PLAIN,
                BONE,
            );
        }
    }
    // The chains it hangs from.
    for t in [0.12f32, 0.42, 0.75] {
        let p = point(t);
        let top = Vec3::new(p.x, frame.y + RoomFrame::HEIGHT - 0.3, p.z);
        rod(lib, p + up * 0.1, top, 0.03, 0.03, kind::METAL, IRON);
    }
}

/// A nest of twigs on a stone plinth at the far end of the hall, an egg in it with a
/// faint glow of its own.
fn egg(lib: &mut Library, frame: &RoomFrame) {
    let o = EGG_O;
    let stone = [0.42, 0.39, 0.35];
    lib.solid(
        piece(
            frame,
            (-0.45, 0.45),
            (o - 0.45, o + 0.45),
            (0.0, 0.85),
            kind::STONE,
            stone,
        ),
        Stops::ALL,
    );
    lib.decor(piece(
        frame,
        (-0.52, 0.52),
        (o - 0.52, o + 0.52),
        (0.85, 0.93),
        kind::STONE,
        [0.48, 0.45, 0.4],
    ));
    let c = frame.at(0.0, o, 0.93);
    let mut rng = StdRng::seed_from_u64(0xe66);
    for k in 0..14 {
        let t = k as f32 / 14.0 * TAU;
        let r = rng.gen_range(0.25..0.36f32);
        let p = c + Vec3::new(libm::cosf(t), 0.0, libm::sinf(t)) * r + Vec3::Y * rng.gen_range(0.02..0.1f32);
        let d = Vec3::new(-libm::sinf(t), rng.gen_range(-0.2..0.2f32), libm::cosf(t)) * 0.22;
        rod(lib, p - d, p + d, 0.025, 0.025, kind::WOOD, [0.3, 0.22, 0.12, 1.0]);
    }
    let shell = [0.08, 0.26, 0.18, 0.35];
    for (y, w) in [
        (0.09f32, 0.16f32),
        (0.16, 0.24),
        (0.24, 0.27),
        (0.32, 0.26),
        (0.4, 0.22),
        (0.47, 0.15),
    ] {
        octo(lib, c + Vec3::Y * y, w, 0.09, kind::CRYSTAL, shell);
    }
    lib.light(c + Vec3::Y * 0.8, 3.5, [0.4, 0.9, 0.8], 0.5, LightKind::Static);
}
