//! The Bestiary: department 3, ground floor, west side. A long hall runs from the
//! door end to the far wall under the hung skeleton of a dragon; either side of it
//! two cages of cases open onto the hall, iron-barred overhead, their gates standing
//! open, and cases line the walls round about under mounted antlers. An egg lies in
//! a nest at the hall's far end.
//!
//! Each of the room's categories finished stirs the egg a little more and lights it a
//! little brighter. The room finished, it hatches (`hatch`).

use super::*;
use crate::audio::Sfx;
use std::f32::consts::{FRAC_PI_2, PI};

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
/// The egg's shell, glowing faintly as it lies before anything is finished.
const SHELL: [f32; 4] = [0.08, 0.26, 0.18, 0.35];
/// How big it is: across, and high.
const EGG: Vec2 = Vec2::new(0.27, 0.44);
/// Seconds after the room is finished that the egg hatches.
const HATCH: f32 = 3.2;
/// How much brighter the egg glows for each of the room's categories finished, and
/// as it is about to hatch.
const STIR: f32 = 1.09;
const QUICKEN: f32 = 2.5;
/// Glints the egg gives off: pale green and gold.
const GLINT: [f32; 4] = [0.7, 1.4, 0.9, 2.2];

pub(super) fn furnish(lib: &mut Library, frame: &RoomFrame, rng: &mut StdRng, titles: &mut HashSet<String>) {
    shelves(lib, frame, rng, titles);
    walls(lib, frame);
    let (hung, eyes) = skeleton(lib, frame);
    egg(lib, frame);
    hatch(lib, frame, hung, eyes);
}

/// A smooth path through `pts` (Catmull-Rom), `n` steps between each two.
fn smooth(pts: &[Vec3], n: usize) -> Vec<Vec3> {
    let at = |k: isize| pts[k.clamp(0, pts.len() as isize - 1) as usize];
    let mut out = Vec::new();
    for k in 0..pts.len() as isize - 1 {
        let (p0, p1, p2, p3) = (at(k - 1), at(k), at(k + 1), at(k + 2));
        for j in 0..n {
            let t = j as f32 / n as f32;
            let (t2, t3) = (t * t, t * t * t);
            out.push(
                (p1 * 2.0
                    + (p2 - p0) * t
                    + (p0 * 2.0 - p1 * 5.0 + p2 * 4.0 - p3) * t2
                    + (p1 * 3.0 - p0 - p2 * 3.0 + p3) * t3)
                    * 0.5,
            );
        }
    }
    out.push(pts[pts.len() - 1]);
    out
}

/// Bones and horn gathered into one mesh about `origin`: tapering limbs swept along
/// paths, and turned or round pieces merged in.
struct Bones {
    mesh: Mesh,
    origin: Vec3,
}

impl Bones {
    fn new(origin: Vec3) -> Bones {
        Bones {
            mesh: Mesh::default(),
            origin,
        }
    }

    /// A bone along `path` (world), `radii` (a few, spread along it) round.
    fn limb(&mut self, path: &[Vec3], radii: &[f32]) {
        let path: Vec<Vec3> = path.iter().map(|p| *p - self.origin).collect();
        let bone = Mesh::sweep(&path, radii, &circle(1.0, 10), true);
        self.mesh.add(&bone, Vec3::ZERO, Quat::IDENTITY, Vec3::ONE);
    }

    /// `m` (fitting the unit box) at `at`, turned by `rot` and `size` big.
    fn piece(&mut self, m: &Mesh, at: Vec3, rot: Quat, size: Vec3) {
        self.mesh.add(m, at - self.origin, rot, size);
    }
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
    let stick = lib.shape(Mesh::cylinder(10));
    let iron = |lib: &mut Library, p: Vec3, q: Vec3, w: f32| lib.decor(stick.between(p, q, w, kind::METAL, IRON));
    for e in [-1.0f32, 1.0] {
        let p = frame.at(s * (MOUTH - 0.05), o + e * edge, 0.0);
        iron(lib, p, p + Vec3::Y * 3.4, 0.09);
    }
    let lintel = |e: f32| frame.at(s * (MOUTH - 0.05), o + e * edge, 3.32);
    iron(lib, lintel(-1.0), lintel(1.0), 0.07);
    // The roof: ribs arched across, each bent round in one piece, and bars along.
    let arch = |x: f32, t: f32| frame.at(s * x, o - edge * libm::cosf(t), top + 0.02 + 1.0 * libm::sinf(t));
    let rib: Vec<Vec3> = (0..=24)
        .map(|j| arch(MOUTH, j as f32 / 24.0 * PI) - arch(MOUTH, 0.0))
        .collect();
    let rib = lib.shape(Mesh::sweep(&rib, &[1.0], &circle(0.0175, 8), true));
    for k in 0..5 {
        let x = MOUTH + 0.1 + k as f32 * (back + 0.02 - MOUTH - 0.1) / 4.0;
        lib.decor(rib.at(arch(x, 0.0), Quat::IDENTITY, kind::METAL, IRON));
    }
    for t in [PI * 0.25, PI * 0.5, PI * 0.75] {
        iron(lib, arch(MOUTH + 0.05, t), arch(back + 0.05, t), 0.03);
    }
    // The gates, swung open flat against the outsides of the sides.
    for e in [-1.0f32, 1.0] {
        let g = |x: f32, y: f32| frame.at(s * x, o + e * (side_back + 0.09), y);
        let (x0, x1) = (MOUTH + 0.02, MOUTH + 1.35);
        for y in [0.15f32, 2.45] {
            iron(lib, g(x0, y), g(x1, y), 0.05);
        }
        for k in 0..6 {
            let x = x0 + k as f32 * (x1 - x0) / 5.0;
            iron(lib, g(x, 0.1), g(x, 2.55), 0.03);
        }
    }
    // Bars round the outside, past the gates: down both sides and across the back.
    let upright = |lib: &mut Library, p: Vec3| iron(lib, p + Vec3::Y * 0.05, p + Vec3::Y * (top + 0.02), 0.03);
    let outside = side_back + 0.11;
    let (x0, x1) = (MOUTH + 1.5, back + 0.11);
    for e in [-1.0f32, 1.0] {
        let n = 15;
        for k in 0..n {
            let x = x0 + (x1 - 0.1 - x0) * k as f32 / (n - 1) as f32;
            upright(lib, frame.at(s * x, o + e * outside, 0.0));
        }
        for y in [0.15f32, top - 0.05] {
            iron(
                lib,
                frame.at(s * x0, o + e * outside, y),
                frame.at(s * x1, o + e * outside, y),
                0.04,
            );
        }
    }
    for k in 0..12 {
        let oo = o - outside + 0.25 + (2.0 * outside - 0.5) * k as f32 / 11.0;
        upright(lib, frame.at(s * x1, oo, 0.0));
    }
    // A little higher than the sides' rails, which they cross at the corners.
    for y in [0.17f32, top - 0.03] {
        iron(
            lib,
            frame.at(s * x1, o - outside, y),
            frame.at(s * x1, o + outside, y),
            0.04,
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
        let a = (a0.min(a1), a0.max(a1));
        pane(
            lib,
            frame,
            a,
            (far - 0.04, far + 0.04),
            (1.26, 4.5),
            [0.32, 0.42, 0.85, 1.1],
        );
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

/// A pair of antlers on a shield, mounted high on side wall `s` at `o`: the skull, and
/// each antler a beam curving up and out with its tines, all of a piece.
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
    let mut bone = Bones::new(skull);
    bone.piece(
        &Mesh::sphere(16),
        skull - Vec3::Y * 0.03,
        face,
        Vec3::new(0.16, 0.26, 0.17),
    );
    let up = Vec3::Y;
    let along = Vec3::Y.cross(n);
    let spread = if k == 0 { 1.0 } else { 0.8 };
    for e in [-1.0f32, 1.0] {
        let side = along * e;
        let base = skull + up * 0.1 + side * 0.06;
        let p1 = base + side * 0.35 * spread + up * 0.25 + n * 0.08;
        let p2 = p1 + side * 0.2 * spread + up * 0.35 - n * 0.02;
        let p3 = p2 - side * 0.05 + up * 0.3;
        bone.limb(&smooth(&[base, p1, p2, p3], 5), &[0.028, 0.02, 0.011]);
        for (from, dir) in [
            (p1, up * 0.25 - side * 0.08),
            (p2, up * 0.22 + side * 0.1),
            (p2, up * 0.15 - side * 0.12 + n * 0.08),
        ] {
            let mid = from + dir * 0.5 + up * 0.03;
            bone.limb(&smooth(&[from, mid, from + dir], 3), &[0.017, 0.004]);
        }
    }
    let bone = lib.shape(bone.mesh);
    lib.decor(bone.at(skull, Quat::IDENTITY, kind::PLAIN, BONE));
}

/// Specimens in jars of murky glass along the sill of the far window on side `s`, dark
/// against the moonlit glass: snakes coiled in some, small things curled in others.
fn specimens(lib: &mut Library, frame: &RoomFrame, s: f32) {
    let o = RoomFrame::O_FAR - 0.11;
    let mut rng = StdRng::seed_from_u64(0xbe57_1a77 ^ (s > 0.0) as u64);
    let v = Vec2::new;
    let jar = [
        v(0.0, 0.0),
        v(0.45, 0.0),
        v(0.5, 0.04),
        v(0.5, 0.92),
        v(0.44, 0.97),
        v(0.44, 1.0),
    ];
    let jar = lib.shape(Mesh::lathe(&jar, 20));
    let lid = [
        v(0.0, 0.0),
        v(0.5, 0.0),
        v(0.5, 0.6),
        v(0.35, 0.8),
        v(0.1, 0.9),
        v(0.0, 1.0),
    ];
    let lid = lib.shape(Mesh::lathe(&lid, 16));
    let coil: Vec<Vec3> = (0..=40)
        .map(|k| {
            let t = k as f32 / 40.0;
            let (th, r) = (t * 2.5 * TAU, 0.3 - 0.1 * t);
            Vec3::new(libm::cosf(th) * r, t * 0.8, libm::sinf(th) * r)
        })
        .collect();
    let snake = lib.shape(Mesh::sweep(&coil, &[0.06, 0.09, 0.02], &circle(1.0, 8), true));
    let curl: Vec<Vec3> = (0..=24)
        .map(|k| {
            let t = k as f32 / 24.0;
            let (th, r) = (t * 1.6 * TAU, 0.3 * (1.0 - 0.6 * t));
            Vec3::new(libm::cosf(th) * r, libm::sinf(th) * r, 0.0)
        })
        .collect();
    let curled = lib.shape(Mesh::sweep(&curl, &[0.16, 0.1, 0.02], &circle(1.0, 8), true));
    let mut x = 8.75;
    let mut k = 0;
    while x < 10.3 {
        let w = rng.gen_range(0.11..0.15f32);
        let h = w * rng.gen_range(1.3..1.9f32);
        let foot = frame.at(s * (x + w * 0.5), o, 1.26);
        let murk = [[0.32, 0.4, 0.18], [0.45, 0.38, 0.15], [0.25, 0.32, 0.3]][rng.gen_range(0..3)];
        let glass = jar.sized(Vec3::new(w, h, w));
        lib.translucent(glass.at(foot, Quat::IDENTITY, kind::CLEAR, rgba(murk, 0.28)));
        let cap = lid.sized(Vec3::new(w * 0.95, 0.04, w * 0.95));
        lib.decor(cap.at(foot + Vec3::Y * h, Quat::IDENTITY, kind::METAL, [0.3, 0.3, 0.28, 1.0]));
        let flesh = [0.55, 0.5, 0.36, 1.0];
        let turn = Quat::from_rotation_y(rng.gen_range(0.0..TAU));
        match k % 3 {
            0 => {
                let snake = snake.sized(Vec3::new(w * 0.7, h * 0.6, w * 0.7));
                lib.decor(snake.inst(foot + Vec3::Y * (h * 0.4), turn, kind::PLAIN, flesh));
            }
            1 => {
                let curled = curled.sized(Vec3::new(w * 0.6, w * 0.55, w * 0.2));
                lib.decor(curled.inst(foot + Vec3::Y * (h * 0.45), turn, kind::PLAIN, flesh));
            }
            _ => {}
        }
        x += w + rng.gen_range(0.06..0.14);
        k += 1;
    }
}

/// The skeleton of a dragon hung on chains from the beams along the hall, its head
/// toward the door, a dull light in its eyes: the spine and its vertebrae tapering to
/// the tail, ribs curving down and in, legs dangling, the bones of its half-spread
/// wings, and the skull with its jaw hanging open. It sways a hair on its chains.
/// Returns its mover and where its eyes are.
fn skeleton(lib: &mut Library, frame: &RoomFrame) -> (u32, [Vec3; 2]) {
    let (u, v, up) = (frame.across(), frame.out(), Vec3::Y);
    let fwd = -v;
    // The spine through (o, height, sway across), from inside the skull to the tip of
    // the tail.
    let spine: [(f32, f32, f32); 10] = [
        (17.95, 3.88, 0.0),
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
    // Along the spine from the back of the skull (0) to the tail's tip (1).
    let point = |t: f32| {
        let o = 18.2 + t * 11.0;
        let k = (spine.iter().rposition(|s| s.0 <= o).unwrap_or(1)).clamp(1, spine.len() - 2);
        let (p, q) = (spine[k], spine[k + 1]);
        let w = (o - p.0) / (q.0 - p.0);
        at(o, p.1 + (q.1 - p.1) * w, p.2 + (q.2 - p.2) * w)
    };
    let beam = RoomFrame::HEIGHT - 0.3;
    let pivot = frame.at(0.0, 23.0, beam);
    let mut bones = Bones::new(pivot);
    let (disc, cone, ball) = (Mesh::cylinder(12), Mesh::cone(8), Mesh::sphere(12));
    let knots: Vec<Vec3> = spine.iter().map(|&(o, y, a)| at(o, y, a)).collect();
    let path = smooth(&knots, 8);
    let radii = [0.05, 0.07, 0.075, 0.065, 0.05, 0.035, 0.024, 0.014];
    bones.limb(&path, &radii);
    let radius = |f: f32| {
        let x = f * (radii.len() - 1) as f32;
        let i = (x as usize).min(radii.len() - 2);
        radii[i] + (radii[i + 1] - radii[i]) * (x - i as f32)
    };
    // Vertebrae, each a disc on the spine, and spines along the back over the body.
    let n = 44;
    for k in 1..n {
        let f = k as f32 / (n - 1) as f32;
        let i = ((f * (path.len() - 1) as f32) as usize).min(path.len() - 2);
        let (p, along) = (path[i], (path[i + 1] - path[i]).normalize());
        let r = radius(f);
        let turn = Quat::from_rotation_arc(Vec3::Y, along);
        bones.piece(&disc, p, turn, Vec3::new(r * 2.6, 0.035, r * 2.6));
        if (0.12..0.62).contains(&f) && k % 2 == 0 {
            let dir = (up + v * 0.25).normalize();
            let turn = Quat::from_rotation_arc(Vec3::Y, dir);
            bones.piece(&cone, p + dir * (r + 0.12), turn, Vec3::new(0.05, 0.26, 0.05));
        }
    }
    // Ribs, curving down and in, longest mid-chest.
    for k in 0..8 {
        let p = point((0.42 * k as f32 + 2.1) / 11.0);
        let len = 1.0 - ((k as f32 - 3.5) / 4.5).powi(2);
        let curve = [(0.0f32, 0.0f32), (0.4, 0.12), (0.62, 0.45), (0.6, 0.85), (0.38, 1.12)];
        let (wide, deep) = (0.55 + 0.45 * len, 0.6 + 0.4 * len);
        for e in [-1.0f32, 1.0] {
            let pts: Vec<Vec3> = (curve.iter())
                .map(|&(x, y)| p + u * (e * x * wide) - up * (y * deep))
                .collect();
            bones.limb(&smooth(&pts, 4), &[0.03, 0.026, 0.016]);
        }
    }
    // Legs, dangling, with claws.
    for (o, back) in [(20.6f32, false), (24.0, true)] {
        let hip = point((o - 18.2) / 11.0);
        for e in [-1.0f32, 1.0] {
            let side = u * e;
            let lean = if back { v * 0.3 } else { -v * 0.2 };
            let knee = hip + side * 0.45 - up * 0.65 + lean;
            let ankle = knee + side * 0.05 - up * 0.55 - lean * 1.2;
            let toe = ankle - up * 0.12 + fwd * 0.15;
            bones.limb(&[hip, knee], &[0.05, 0.036]);
            bones.limb(&[knee, ankle], &[0.034, 0.026]);
            bones.limb(&[ankle, toe], &[0.025, 0.018]);
            bones.piece(&ball, knee, Quat::IDENTITY, Vec3::splat(0.085));
            bones.piece(&ball, ankle, Quat::IDENTITY, Vec3::splat(0.065));
            for c in [-1.0f32, 0.0, 1.0] {
                let claw = toe + fwd * 0.16 + side * (c * 0.07) - up * 0.05;
                let mid = (toe + claw) * 0.5 + up * 0.02;
                bones.limb(&smooth(&[toe, mid, claw], 3), &[0.016, 0.003]);
            }
        }
    }
    // The wings, half spread: arm and forearm, and three long fingers.
    let shoulder = point(0.22);
    for e in [-1.0f32, 1.0] {
        let side = u * e;
        let elbow = shoulder + side * 1.3 + up * 0.45 + v * 0.3;
        let wrist = elbow + side * 1.2 - up * 0.1 + v * 0.55;
        bones.limb(&[shoulder, elbow], &[0.055, 0.04]);
        bones.limb(&[elbow, wrist], &[0.04, 0.03]);
        bones.piece(&ball, elbow, Quat::IDENTITY, Vec3::splat(0.09));
        bones.piece(&ball, wrist, Quat::IDENTITY, Vec3::splat(0.07));
        for (reach, back, drop) in [(1.1f32, 0.3f32, 0.2f32), (0.7, 1.2, 0.35), (0.2, 1.9, 0.45)] {
            let tip = wrist + side * reach + v * back - up * drop;
            let mid = (wrist + tip) * 0.5 + up * 0.08;
            bones.limb(&smooth(&[wrist, mid, tip], 4), &[0.028, 0.018, 0.005]);
        }
    }
    // The skull: the brow, the snout narrowing to the nose, the jaw hanging open,
    // teeth, and horns swept back.
    let head = at(17.75, 3.85, 0.0);
    let face = Quat::from_rotation_arc(Vec3::Z, fwd);
    bones.piece(
        &ball,
        head,
        face * Quat::from_rotation_x(-0.15),
        Vec3::new(0.42, 0.32, 0.46),
    );
    let snout = [
        (0.0, 0.0),
        (0.13, 0.0),
        (0.125, 0.15),
        (0.1, 0.35),
        (0.075, 0.5),
        (0.05, 0.56),
        (0.0, 0.58),
    ];
    let snout = Mesh::lathe(&snout.map(Vec2::from), 16);
    let nose = face * Quat::from_rotation_x(FRAC_PI_2 + 0.1);
    bones.piece(&snout, head + fwd * 0.12 - up * 0.04, nose, Vec3::new(1.0, 1.0, 0.75));
    let down = Quat::from_rotation_x(PI);
    for e in [-1.0f32, 1.0] {
        let side = u * e;
        let hinge = head - up * 0.1 - fwd * 0.06 + side * 0.13;
        let chin = head + fwd * 0.6 - up * 0.36 + side * 0.03;
        let sag = (hinge + chin) * 0.5 - up * 0.03;
        bones.limb(&smooth(&[hinge, sag, chin], 4), &[0.035, 0.028, 0.018]);
        for j in 0..3 {
            let tooth = head + fwd * (0.32 + j as f32 * 0.12) + side * 0.08 - up * 0.13;
            bones.piece(&cone, tooth, down, Vec3::new(0.025, 0.08, 0.025));
            let f = 0.5 + 0.15 * j as f32;
            let low = hinge + (chin - hinge) * f + up * 0.035;
            bones.piece(&cone, low, Quat::IDENTITY, Vec3::new(0.02, 0.055, 0.02));
        }
        let root = head + up * 0.12 + side * 0.14 - fwd * 0.12;
        let mid = root + up * 0.3 - fwd * 0.35 + side * 0.12;
        let tip = mid + up * 0.08 - fwd * 0.35 + side * 0.05;
        bones.limb(&smooth(&[root, mid, tip], 5), &[0.045, 0.03, 0.005]);
    }
    // The chains it hangs from, from three beams, link by link.
    let (link, _, _) = Mesh::torus(0.3, 12, 6).fitted();
    let mut chains = Mesh::default();
    for o in [18.8f32, 21.8, 24.8] {
        let p = point((o - 18.2) / 11.0) + up * 0.06;
        let top = Vec3::new(p.x, frame.y + beam, p.z);
        let count = ((top.y - p.y) / 0.065) as usize;
        for k in 0..count {
            let c = top - up * (0.035 + k as f32 * 0.065);
            let turn = Quat::from_rotation_y(FRAC_PI_2 * (k % 2) as f32) * Quat::from_rotation_x(FRAC_PI_2);
            chains.add(&link, c - pivot, turn, Vec3::new(0.045, 0.012, 0.085));
        }
        let plate = Inst::new(top, Quat::IDENTITY, Vec3::new(0.14, 0.02, 0.14), kind::METAL, IRON, 0);
        lib.decor(plate);
    }
    // Once the egg has hatched it stirs a little more, nodding slowly on its chains,
    // and keeps on.
    let nod = Motion::Swing {
        axis: u,
        amp: 0.01,
        period: 9.0,
    };
    let stirred = lib.mover(pivot, nod, 0.0, None);
    lib.awaiting(stirred, Awaits::starts(Finish::Room(frame.dept)).after(HATCH).over(4.0));
    let sway = Motion::Swing {
        axis: v,
        amp: 0.012,
        period: 13.0,
    };
    let hung = lib.mover(pivot, sway, 0.0, Some(stirred));
    let rock = Motion::Swing {
        axis: u,
        amp: 0.005,
        period: 19.0,
    };
    let hung = lib.mover(pivot, rock, 0.0, Some(hung));
    let bones = lib.shape(bones.mesh);
    lib.moving(hung, bones.at(pivot, Quat::IDENTITY, kind::PLAIN, BONE));
    let chains = lib.shape(chains);
    lib.moving(hung, chains.at(pivot, Quat::IDENTITY, kind::METAL, IRON));
    let eyes = [-1.0f32, 1.0].map(|e| head + fwd * 0.12 + up * 0.05 + u * (e * 0.2));
    for eye in eyes {
        lib.moving(
            hung,
            Inst::new(
                eye,
                face,
                Vec3::new(0.04, 0.07, 0.09),
                kind::CRYSTAL,
                [1.0, 0.55, 0.15, 1.8],
                0,
            ),
        );
    }
    (hung, eyes)
}

/// A nest of twigs on a stone plinth at the far end of the hall, an egg in it with a
/// faint glow of its own, rocking now and then as if something stirred inside.
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
    let up = Vec3::Y;
    // Twigs bent round in a ring, all of a piece.
    let mut rng = StdRng::seed_from_u64(0xe66);
    let mut nest = Mesh::default();
    for k in 0..18 {
        let t = k as f32 / 18.0 * TAU;
        let r = rng.gen_range(0.24..0.34f32);
        let (out, along) = (
            Vec3::new(libm::cosf(t), 0.0, libm::sinf(t)),
            Vec3::new(-libm::sinf(t), 0.0, libm::cosf(t)),
        );
        let p = out * r + up * rng.gen_range(0.03..0.1f32);
        let half = along * 0.22 + up * rng.gen_range(-0.04..0.04f32);
        let twig = Mesh::sweep(
            &[p - half, p + out * 0.03, p + half],
            &[0.014, 0.009],
            &circle(1.0, 8),
            true,
        );
        nest.add(&twig, Vec3::ZERO, Quat::IDENTITY, Vec3::ONE);
    }
    let nest = lib.shape(nest);
    lib.decor(nest.at(c, Quat::IDENTITY, kind::WOOD, [0.3, 0.22, 0.12, 1.0]));
    let shell: Vec<Vec2> = (0..=16).map(|k| egg_outline(egg_t(k as f32 / 16.0))).collect();
    let shell = lib.shape(Mesh::lathe(&shell, 32)).sized(Vec3::new(EGG.x, EGG.y, EGG.x));
    let foot = c + up * 0.05;
    let (out, across) = (frame.out(), frame.across());
    let fin = Finish::Room(frame.dept);
    // Gone when it hatches; then its halves are there (`hatch`).
    let gone = Change::Shifts {
        by: Vec3::ZERO,
        turn: Quat::IDENTITY,
        scale: 0.0,
    };
    let mut rides = lib.waiting(foot, Awaits::new(fin, gone).after(HATCH).over(0.06), []);
    // Each category finished stirs it a little more, lights it a little brighter, and
    // it answers with a few glints.
    for (k, cat) in frame.categories.clone().enumerate() {
        let stir = Motion::Swing {
            axis: if k % 2 == 0 { out } else { across },
            amp: 0.012,
            period: 2.1 + 0.07 * k as f32,
        };
        let m = lib.mover(foot, stir, 0.0, Some(rides));
        let brighter = Change::Kindles { dim: 1.0 / STIR };
        lib.awaiting(m, Awaits::new(Finish::Category(cat), brighter).over(2.0).still());
        rides = m;
        lib.sparks(Sparks {
            lasts: Some(1.2),
            rate: 30.0,
            rgba: GLINT,
            spread: Vec3::splat(0.12),
            vel: up * 0.4,
            ..Sparks::burst(Finish::Category(cat), foot + up * 0.25)
        });
    }
    // About to hatch: rocking hard and quick, glowing brighter.
    let quick = Motion::Swing {
        axis: out,
        amp: 0.07,
        period: 1.0,
    };
    let quicken = lib.mover(foot, quick, 0.0, Some(rides));
    let brighter = Change::Kindles { dim: 1.0 / QUICKEN };
    lib.awaiting(quicken, Awaits::new(fin, brighter).over(HATCH).still());
    // Two rockings a little out of step: together they swell and die away in turn.
    let rock = |period: f32| Motion::Swing {
        axis: out,
        amp: 0.035,
        period,
    };
    let first = lib.mover(foot, rock(2.3), 0.0, Some(quicken));
    let rocking = lib.mover(foot, rock(2.5), 0.0, Some(first));
    let glow = SHELL[3] * STIR.powi(frame.categories.len() as i32) * QUICKEN;
    lib.moving(
        rocking,
        shell.at(
            foot,
            Quat::IDENTITY,
            kind::CRYSTAL,
            [SHELL[0], SHELL[1], SHELL[2], glow],
        ),
    );
    lib.light(c + up * 0.8, 3.5, [0.4, 0.9, 0.8], 0.5, LightKind::Static);
}

/// Along the egg's outline from its foot (0) to its tip (1).
fn egg_t(f: f32) -> f32 {
    -FRAC_PI_2 + PI * f
}

/// The egg's outline at `t` (from -π/2 at its foot to π/2 at its tip): its radius and
/// height, a unit high; broader below than above.
fn egg_outline(t: f32) -> Vec2 {
    let r = 0.5 * libm::cosf(t) * (1.0 - 0.12 * libm::sinf(t));
    Vec2::new(r, 0.5 + 0.5 * libm::sinf(t))
}

/// Part of the egg's shell, its foot at the origin: from `lo` to `hi` of its height
/// at each turn round it (radians from across). Seen inside and out.
fn shell_part(lo: impl Fn(f32) -> f32, hi: impl Fn(f32) -> f32) -> Mesh {
    // As big as the egg drawn whole, which is its outline turned at 17 points.
    let widest = (0..=16)
        .map(|k| egg_outline(egg_t(k as f32 / 16.0)).x)
        .fold(0.0f32, f32::max);
    let scale = Vec2::new(EGG.x / (2.0 * widest), EGG.y);
    let (segs, rows) = (42, 12);
    let mut m = Mesh::default();
    let mut grid = Vec::new();
    for i in 0..=segs {
        let th = i as f32 / segs as f32 * TAU;
        let (c, s) = (libm::cosf(th), libm::sinf(th));
        let (y0, y1) = (lo(th), hi(th));
        for j in 0..=rows {
            let y = y0 + (y1 - y0) * j as f32 / rows as f32;
            let t = libm::asinf((2.0 * y - 1.0).clamp(-1.0, 1.0));
            let p = egg_outline(t) * scale;
            // Outward, square to the outline where it is.
            let d = (egg_outline(t + 1e-3) - egg_outline(t - 1e-3)) * scale;
            let n = Vec2::new(d.y, -d.x).normalize_or(Vec2::Y);
            grid.push(m.vert(Vec3::new(p.x * c, p.y, p.x * s), Vec3::new(n.x * c, n.y, n.x * s)));
        }
    }
    let at = |i: usize, j: usize| grid[i * (rows + 1) + j];
    for i in 0..segs {
        for j in 0..rows {
            m.tri(at(i, j), at(i + 1, j), at(i + 1, j + 1));
            m.tri(at(i, j), at(i + 1, j + 1), at(i, j + 1));
        }
    }
    m.two_sided()
}

/// Where the shell cracked, round it: a jagged line about its middle.
fn crack(th: f32) -> f32 {
    let u = th * 7.0 / TAU;
    let zig = 1.0 - 4.0 * (u - libm::floorf(u) - 0.5).abs();
    0.5 + zig * (0.045 + 0.015 * libm::sinf(th * 3.0))
}

/// The egg hatching, the room finished. It has rocked harder and glowed brighter
/// (`egg`); then it cracks: its cap tumbles off onto the plinth, shards and glints fly,
/// and a hatchling rises in the cup of the shell. It lifts its head, stretches its
/// wings once and folds them, and stays: breathing, looking slowly about. A warm light
/// kindles in the nest and glints rise from it toward the old dragon's tail overhead;
/// the old dragon's eyes flare as the shell cracks, and keep a little of it.
fn hatch(lib: &mut Library, frame: &RoomFrame, hung: u32, eyes: [Vec3; 2]) {
    let fin = Finish::Room(frame.dept);
    let up = Vec3::Y;
    let (u, fwd) = (frame.across(), -frame.out());
    let c = frame.at(0.0, EGG_O, 0.93);
    let foot = c + up * 0.05;
    let there = Change::Arrives {
        from: Vec3::ZERO,
        grow: 1.0,
    };
    let cracked = Awaits::new(fin, there).after(HATCH).over(0.05);
    let cup = lib.shape(shell_part(|_| 0.0, crack));
    lib.waiting(foot, cracked, [cup.at(foot, Quat::IDENTITY, kind::CRYSTAL, SHELL)]);
    // The cap tumbles off to lie open beside the nest, a little on the twigs.
    let cap = shell_part(crack, |_| 1.0);
    let middle = foot + up * (EGG.y * 0.75);
    let tip = Quat::from_axis_angle(frame.out(), 1.8);
    let lowest = (cap.verts.iter())
        .map(|v| (tip * (foot + Vec3::from(v.pos) - middle)).y)
        .fold(f32::MAX, f32::min);
    let rest = c + u * 0.36 + fwd * 0.1;
    let by = Vec3::new(rest.x - middle.x, c.y + 0.025 - middle.y - lowest, rest.z - middle.z);
    let cap = lib.shape(cap);
    let shown = lib.waiting(middle, cracked, []);
    let tumbles = lib.mover(middle, Motion::Still, 0.0, Some(shown));
    let off = Change::Shifts {
        by,
        turn: tip,
        scale: 1.0,
    };
    lib.awaiting(tumbles, Awaits::new(fin, off).after(HATCH).over(0.7));
    lib.moving(tumbles, cap.at(foot, Quat::IDENTITY, kind::CRYSTAL, SHELL));
    hatchling(lib, frame, foot);
    // The nest's light kindles as it rocks, flares as it cracks, and settles.
    let warm = foot + up * 0.45;
    let kindle = lib.waiting(warm, Awaits::new(fin, Change::Kindles { dim: 0.0 }).over(HATCH), []);
    let settle = lib.mover(warm, Motion::Still, 0.0, Some(kindle));
    lib.awaiting(
        settle,
        Awaits::new(fin, Change::Kindles { dim: 3.0 }).after(HATCH).over(4.0),
    );
    lib.light_riding(settle, warm, 3.5, [1.0, 0.72, 0.42], 0.45, LightKind::Static);
    // The old dragon's eyes: a warm light on its skull, flaring and settling.
    let gaze = (eyes[0] + eyes[1]) * 0.5 + fwd * 0.25;
    let wake = lib.mover(gaze, Motion::Still, 0.0, Some(hung));
    lib.awaiting(
        wake,
        Awaits::new(fin, Change::Kindles { dim: 0.0 })
            .after(HATCH - 0.8)
            .over(1.2),
    );
    let calm = lib.mover(gaze, Motion::Still, 0.0, Some(wake));
    lib.awaiting(
        calm,
        Awaits::new(fin, Change::Kindles { dim: 6.0 })
            .after(HATCH + 0.8)
            .over(4.0),
    );
    lib.light_riding(calm, gaze, 3.0, [1.0, 0.55, 0.2], 0.25, LightKind::Static);
    // And a glow over each eye, swelling and then sinking back into it.
    let there = Change::Arrives {
        from: Vec3::ZERO,
        grow: 1.0,
    };
    let sink = Change::Shifts {
        by: Vec3::ZERO,
        turn: Quat::IDENTITY,
        scale: 0.0,
    };
    let face = Quat::from_rotation_arc(Vec3::Z, fwd);
    for eye in eyes {
        let shown = lib.mover(eye, Motion::Still, 0.0, Some(hung));
        lib.awaiting(shown, Awaits::new(fin, there).after(HATCH - 0.8).over(0.05));
        let swells = lib.mover(eye, Motion::Still, 0.0, Some(shown));
        lib.awaiting(
            swells,
            Awaits::new(fin, Change::Kindles { dim: 0.0 })
                .after(HATCH - 0.8)
                .over(1.0),
        );
        let sinks = lib.mover(eye, Motion::Still, 0.0, Some(swells));
        lib.awaiting(sinks, Awaits::new(fin, sink).after(HATCH + 0.6).over(2.5));
        let glow = Inst::new(
            eye,
            face,
            Vec3::new(0.06, 0.1, 0.11),
            kind::CRYSTAL,
            [1.0, 0.55, 0.15, 3.5],
            0,
        );
        lib.moving(sinks, glow);
    }
    for eye in eyes {
        lib.sparks(Sparks {
            delay: HATCH - 0.4,
            lasts: Some(1.5),
            rate: 14.0,
            rgba: [1.6, 0.8, 0.3, 2.5],
            size: Vec3::splat(0.016),
            spread: Vec3::splat(0.03),
            vel: up * 0.2,
            jitter: Vec3::splat(0.15),
            life: 1.4,
            ..Sparks::burst(fin, eye)
        });
    }
    let heart = foot + up * 0.22;
    lib.sparks(Sparks {
        lasts: Some(HATCH),
        rate: 14.0,
        rgba: GLINT,
        spread: Vec3::splat(0.1),
        vel: up * 0.35,
        jitter: Vec3::new(0.15, 0.1, 0.15),
        ..Sparks::burst(fin, heart)
    });
    // The crack: glints burst and shards of shell fly.
    lib.sparks(Sparks {
        delay: HATCH,
        lasts: Some(0.4),
        rate: 150.0,
        vel: up * 0.9,
        jitter: Vec3::new(0.9, 0.6, 0.9),
        ..Sparks::burst(fin, heart)
    });
    lib.sparks(Sparks {
        delay: HATCH,
        lasts: Some(0.2),
        rate: 150.0,
        kind: kind::CRYSTAL,
        rgba: [SHELL[0], SHELL[1], SHELL[2], 0.5],
        size: Vec3::new(0.028, 0.004, 0.022),
        vel: up * 1.2,
        jitter: Vec3::new(1.0, 0.5, 1.0),
        lift: -5.0,
        drag: 0.6,
        life: 1.0,
        ..Sparks::burst(fin, foot + up * 0.3)
    });
    // Glints rising from the nest toward the old dragon's tail, and a few warm motes
    // about it for good.
    lib.sparks(Sparks {
        delay: HATCH + 0.5,
        lasts: Some(4.0),
        rate: 9.0,
        rgba: [1.4, 1.0, 0.5, 2.2],
        spread: Vec3::splat(0.1),
        vel: up * 0.8,
        jitter: Vec3::new(0.1, 0.15, 0.1),
        lift: 0.05,
        drag: 0.05,
        life: 3.8,
        ..Sparks::burst(fin, foot + up * 0.4)
    });
    lib.sparks(Sparks {
        delay: HATCH + 4.0,
        rate: 1.2,
        life: 5.0,
        ..Sparks::motes(fin, foot + up * 0.35, Vec3::splat(0.35))
    });
    for (delay, sound, volume) in [(HATCH, Sfx::Chime, 0.5), (HATCH + 2.8, Sfx::Unfold, 0.6)] {
        lib.cue(Cue {
            finish: fin,
            delay,
            sound,
            at: heart,
            volume,
        });
    }
}

/// The hatchling, in the cup of its shell at `foot`, facing down the hall: scaled like
/// its egg, a curled tail over the rim and round the shell, forepaws on the rim, folded
/// wings, a lifted head with nubs of horn and warm eyes. It rises in as the shell
/// cracks, its head bowed; lifts its head, stretches its wings and folds them; then
/// breathes and looks slowly about.
fn hatchling(lib: &mut Library, frame: &RoomFrame, foot: Vec3) {
    let fin = Finish::Room(frame.dept);
    let up = Vec3::Y;
    let (u, fwd) = (frame.across(), -frame.out());
    // Across, up and forward from the foot of the egg.
    let at = |a: f32, y: f32, f: f32| u * a + up * y + fwd * f;
    let facing = Quat::from_mat3(&Mat3::from_cols(u, up, fwd));
    let (ball, round) = (Mesh::sphere(16), circle(1.0, 8));
    let scales = [0.13, 0.4, 0.33, 1.0];
    let mut body = Mesh::default();
    body.add(&ball, at(0.0, 0.165, -0.01), facing, Vec3::new(0.15, 0.13, 0.19));
    let tail = smooth(
        &[
            at(0.0, 0.15, -0.08),
            at(0.0, 0.26, -0.14),
            at(0.02, 0.18, -0.19),
            at(0.12, 0.1, -0.15),
            at(0.19, 0.08, -0.04),
            at(0.17, 0.08, 0.06),
        ],
        5,
    );
    let sweep = |path: &[Vec3], radii: &[f32]| Mesh::sweep(path, radii, &round, true);
    body.add(
        &sweep(&tail, &[0.035, 0.026, 0.014, 0.004]),
        Vec3::ZERO,
        Quat::IDENTITY,
        Vec3::ONE,
    );
    for e in [-1.0f32, 1.0] {
        let paw = [
            at(e * 0.05, 0.17, 0.06),
            at(e * 0.06, 0.21, 0.1),
            at(e * 0.062, 0.235, 0.118),
        ];
        body.add(&sweep(&paw, &[0.022, 0.016]), Vec3::ZERO, Quat::IDENTITY, Vec3::ONE);
        body.add(&ball, paw[2] + up * 0.003, facing, Vec3::splat(0.03));
    }
    let breathe = Motion::Bob {
        amp: 0.004,
        period: 3.4,
        rock: 0.0,
    };
    let held = lib.mover(foot, breathe, 0.0, None);
    let rise = Change::Arrives {
        from: -up * 0.1,
        grow: 0.6,
    };
    lib.awaiting(held, Awaits::new(fin, rise).after(HATCH + 0.15).over(1.4));
    let body = lib.shape(body);
    lib.moving(held, body.at(foot, Quat::IDENTITY, kind::PLAIN, scales));
    // The head on its neck: bowed as it rises (turned down at once, while it is not
    // there yet, then back up), then looking about.
    let neck = foot + at(0.0, 0.2, 0.06);
    let bowed = Quat::from_axis_angle(u, 0.8);
    let turn = |q: Quat| Change::Shifts {
        by: Vec3::ZERO,
        turn: q,
        scale: 1.0,
    };
    let bow = lib.mover(neck, Motion::Still, 0.0, Some(held));
    lib.awaiting(bow, Awaits::new(fin, turn(bowed)).over(0.01));
    let lift = lib.mover(neck, Motion::Still, 0.0, Some(bow));
    lib.awaiting(
        lift,
        Awaits::new(fin, turn(bowed.inverse())).after(HATCH + 1.6).over(1.2),
    );
    let look = Motion::Swing {
        axis: up,
        amp: 0.4,
        period: 11.0,
    };
    let looking = lib.mover(neck, look, 0.0, Some(lift));
    lib.awaiting(looking, Awaits::starts(fin).after(HATCH + 3.2).over(3.0));
    let mut head = sweep(
        &[at(0.0, 0.2, 0.06), at(0.0, 0.25, 0.085), at(0.0, 0.29, 0.105)],
        &[0.034, 0.026],
    );
    head.add(&ball, at(0.0, 0.325, 0.12), facing, Vec3::new(0.085, 0.075, 0.1));
    head.add(&ball, at(0.0, 0.315, 0.175), facing, Vec3::new(0.055, 0.045, 0.08));
    let horn = (up - fwd * 0.9).normalize();
    for e in [-1.0f32, 1.0] {
        let root = at(e * 0.022, 0.355, 0.1);
        head.add(
            &Mesh::cone(8),
            root + horn * 0.02,
            Quat::from_rotation_arc(Vec3::Y, horn),
            Vec3::new(0.016, 0.045, 0.016),
        );
    }
    let head = lib.shape(head);
    lib.moving(looking, head.at(foot, Quat::IDENTITY, kind::PLAIN, scales));
    let eye = lib.shape(Mesh::sphere(12)).sized(Vec3::splat(0.022));
    for e in [-1.0f32, 1.0] {
        lib.moving(
            looking,
            eye.inst(
                foot + at(e * 0.034, 0.338, 0.148),
                facing,
                kind::CRYSTAL,
                [1.0, 0.6, 0.2, 2.0],
            ),
        );
    }
    // Wings folded back along its sides, each stretched out once and folded again
    // (turned out, then back by as much).
    let folded = Vec2::from_angle(-0.7);
    let wing = [
        (0.0, 0.0),
        (0.03, 0.08),
        (0.08, 0.13),
        (0.08, 0.08),
        (0.12, 0.08),
        (0.09, 0.04),
        (0.11, 0.01),
        (0.05, 0.01),
    ]
    .map(|p| folded.rotate(Vec2::from(p)));
    let back = -fwd;
    let flat = Quat::from_mat3(&Mat3::from_cols(back, up, back.cross(up)));
    let along = (back * libm::cosf(0.35) + up * libm::sinf(0.35)).normalize();
    for e in [-1.0f32, 1.0] {
        let shoulder = foot + at(e * 0.06, 0.215, 0.01);
        let mut w = Mesh::default();
        w.add(&Mesh::prism(&wing, 0.004), shoulder - foot, flat, Vec3::ONE);
        let edge = [wing[0], wing[1], wing[2]].map(|p| shoulder - foot + back * p.x + up * p.y);
        w.add(&sweep(&edge, &[0.009, 0.005]), Vec3::ZERO, Quat::IDENTITY, Vec3::ONE);
        let spread = (u * (e * 0.8) + up * 0.5 + back * 0.3).normalize();
        let open = Quat::from_rotation_arc(along, spread);
        let stretch = lib.mover(shoulder, Motion::Still, 0.0, Some(held));
        lib.awaiting(stretch, Awaits::new(fin, turn(open)).after(HATCH + 2.8).over(0.8));
        let fold = lib.mover(shoulder, Motion::Still, 0.0, Some(stretch));
        lib.awaiting(
            fold,
            Awaits::new(fin, turn(open.inverse())).after(HATCH + 4.6).over(1.6),
        );
        let w = lib.shape(w);
        lib.moving(fold, w.at(foot, Quat::IDENTITY, kind::PLAIN, [0.2, 0.5, 0.42, 1.0]));
    }
}
