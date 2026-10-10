//! The Chronicle Hall: department 5, first floor, east side. A stately long room: a
//! nave from the door to the Great Chronicle on its stand, the stacks running out from
//! it either side with a marble bust at the end of each, heraldic banners hung down
//! the nave, and a timeline that writes itself along the walls.

use super::*;
use std::f32::consts::{FRAC_PI_2, FRAC_PI_4, PI};

/// The stacks: rows across the room at these `o`, each two cases either side of
/// the nave.
const ROWS: [f32; 4] = [19.2, 22.4, 25.6, 28.8];
/// Where a bank's two cases stand across the room, as |a|; a bank reaches from
/// `NAVE` to 8.28.
const UNITS: [f32; 2] = [4.24, 6.94];
const NAVE: f32 = 2.9;
/// The wall case closing each aisle, on the side wall at the right hand of the row
/// side facing into it: near sides end on the right wall (+a), far sides on the left.
const NEAR_ENDS: [f32; 4] = [16.5, 20.8, 24.0, 27.2];
const FAR_ENDS: [f32; 4] = [20.8, 24.0, 27.2, 30.45];
/// The timeline's band along the walls.
const FRIEZE: (f32, f32) = (3.72, 4.22);
const MARBLE: [f32; 3] = [0.62, 0.6, 0.56];
const GOLD: [f32; 4] = [0.66, 0.48, 0.14, 1.0];

pub(super) fn furnish(lib: &mut Library, frame: &RoomFrame, rng: &mut StdRng, titles: &mut HashSet<String>) {
    walls(lib, frame);
    stacks(lib, frame, rng, titles);
    for (i, &o) in ROWS.iter().enumerate() {
        for s in [-1.0f32, 1.0] {
            bust(lib, frame, s, o, i * 2 + (s > 0.0) as usize);
        }
    }
    for (i, o) in [17.4f32, 20.8, 24.0, 27.2].into_iter().enumerate() {
        for s in [-1.0f32, 1.0] {
            banner(lib, frame, s * 2.35, o, i * 2 + (s > 0.0) as usize);
        }
    }
    chronicle(lib, frame);
    armour(lib, frame, -1.0, 18.6);
    armour(lib, frame, 1.0, 30.3);
    let mut runner = Inst::aabox(
        frame.at(0.0, 22.9, 0.012),
        frame.size(2.3, 0.024, 15.0),
        kind::RUG,
        [0.32, 0.05, 0.05],
    );
    runner.param = 150 | (110 << 8) | (40 << 16);
    lib.rug(runner);
    for o in [20.8f32, 27.2] {
        lib.chandelier(frame.at(0.0, o, 4.1), frame.y + RoomFrame::HEIGHT);
    }
}

/// The stacks and their wall cases. Each side of a row is a category: its four faces
/// across the nave, then round the corner to the wall case closing its aisle on the
/// right. The categories run from the door to the far wall.
fn stacks(lib: &mut Library, f: &RoomFrame, rng: &mut StdRng, titles: &mut HashSet<String>) {
    let (out, across) = (f.out(), f.across());
    let along_a = !f.out_along_x();
    let mut cat = f.categories.start;
    for (i, &o) in ROWS.iter().enumerate() {
        let (mut near, mut far) = (Vec::new(), Vec::new());
        for s in [-1.0f32, 1.0] {
            for &u in &UNITS {
                for face in lib.bookcase(f.at(s * u, o, 0.0), along_a) {
                    if face.1.dot(out) < 0.0 {
                        near.push(face);
                    } else {
                        far.push(face);
                    }
                }
            }
        }
        let near_end = lib.wall_bookcase(f.at(RoomFrame::A_HALF, NEAR_ENDS[i], 0.0), -across);
        let far_end = lib.wall_bookcase(f.at(-RoomFrame::A_HALF, FAR_ENDS[i], 0.0), across);
        for (side, end) in [(near, near_end), (far, far_end)] {
            let mut faces = left_to_right(side);
            faces.push(end);
            lib.add_category(rng, cat, &faces, titles);
            cat += 1;
        }
    }
}

/// Beams along and across the ceiling with gilt bosses where they cross, panelling
/// on the bare walls, sconces by the door and the far wall, windows high in the far
/// wall, the tapestry of the ages behind the Chronicle, and the timeline.
fn walls(lib: &mut Library, f: &RoomFrame) {
    let h = RoomFrame::HEIGHT;
    let ah = RoomFrame::A_HALF;
    let (near, far) = (RoomFrame::O_NEAR, RoomFrame::O_FAR);
    let beam = [0.11, 0.055, 0.025];
    for a in [-8.3f32, -NAVE, NAVE, 8.3] {
        slab(
            lib,
            f,
            (a - 0.12, a + 0.12),
            (near, far),
            (h - 0.28, h),
            kind::WOOD,
            beam,
        );
    }
    for o in [16.0f32, 19.2, 22.4, 25.6, 28.8] {
        slab(lib, f, (-ah, ah), (o - 0.12, o + 0.12), (h - 0.24, h), kind::WOOD, beam);
        for a in [-8.3f32, -NAVE, NAVE, 8.3] {
            slab(
                lib,
                f,
                (a - 0.15, a + 0.15),
                (o - 0.15, o + 0.15),
                (h - 0.33, h - 0.28),
                kind::METAL,
                [0.62, 0.45, 0.14],
            );
        }
    }
    // Panelling where no case stands: the near wall, the far wall, the left wall
    // before its first case and the right wall past its last.
    let t = 0.07;
    let d = RoomFrame::DOOR_HALF + 0.3;
    for (a0, a1) in [(-ah + t, f.door - d), (f.door + d, ah - t)] {
        lib.wainscot(f.y, f.rect(a0, a1, near, near + t), f.side.out);
    }
    lib.wainscot(f.y, f.rect(-ah, ah, far - t, far), -f.side.out);
    lib.wainscot(f.y, f.rect(-ah, -ah + t, near, FAR_ENDS[0] - 1.4), f.side.across);
    lib.wainscot(f.y, f.rect(ah - t, ah, NEAR_ENDS[3] + 1.4, far - t), -f.side.across);
    for a in [-5.5f32, 5.5] {
        lib.sconce(f.at(a, near + 0.15, 2.6), f.out());
    }
    for a in [-3.4f32, 3.4] {
        lib.sconce(f.at(a, far - 0.15, 2.6), -f.out());
    }
    for a in [-6.4f32, 6.4] {
        window(lib, Spot::new(f, a, far, 0.0), 1.8);
        let c = f.at(a, far - 0.1, 4.85);
        let dir = (-f.out() - Vec3::Y * 0.8).normalize();
        let len = 6.0;
        lib.static_trans.push(Inst::new(
            c + dir * len * 0.5,
            Quat::from_rotation_arc(Vec3::Y, dir),
            Vec3::new(1.5, len, 0.9),
            kind::SHAFT,
            [0.35, 0.45, 0.9, 0.035],
            0,
        ));
        lib.light(c - f.out() * 1.5, 8.0, [0.45, 0.55, 1.0], 0.7, LightKind::Static);
    }
    tapestry(lib, f);
    timeline(lib, f);
}

/// A wide window high in a wall, over the timeline, `w` wide with a stone surround;
/// `at` is on the wall's face, facing into the room.
fn window(lib: &mut Library, at: Spot, w: f32) {
    let (y0, y1) = (4.4, 5.3);
    let stone = [0.46, 0.43, 0.38, 1.0];
    at.part(
        lib,
        Vec3::new(0.0, (y0 + y1) * 0.5, -0.02),
        Vec3::new(w, y1 - y0, 0.08),
        Quat::IDENTITY,
        kind::GLASS,
        [0.32, 0.42, 0.85, 1.1],
    );
    at.part(
        lib,
        Vec3::new(0.0, y0 - 0.05, 0.06),
        Vec3::new(w + 0.3, 0.1, 0.12),
        Quat::IDENTITY,
        kind::STONE,
        stone,
    );
    for x in [-1.0f32, 1.0] {
        at.part(
            lib,
            Vec3::new(x * (w * 0.5 + 0.07), (y0 + y1) * 0.5 + 0.05, 0.05),
            Vec3::new(0.14, y1 - y0 + 0.1, 0.1),
            Quat::IDENTITY,
            kind::STONE,
            stone,
        );
    }
}

/// The tapestry of the ages on the far wall behind the Chronicle: a crimson field in a
/// gold border, a golden tree of the dynasties on it.
fn tapestry(lib: &mut Library, f: &RoomFrame) {
    let at = Spot::new(f, 0.0, RoomFrame::O_FAR, 0.0);
    let (w, y0, y1) = (4.0f32, 1.3f32, 3.55f32);
    let ym = (y0 + y1) * 0.5;
    at.part(
        lib,
        Vec3::new(0.0, ym, 0.02),
        Vec3::new(w, y1 - y0, 0.03),
        Quat::IDENTITY,
        kind::CLOTH,
        [0.36, 0.05, 0.06, 1.0],
    );
    at.part(
        lib,
        Vec3::new(0.0, y1 + 0.04, 0.05),
        Vec3::new(w + 0.4, 0.07, 0.07),
        Quat::IDENTITY,
        kind::METAL,
        GOLD,
    );
    let z = 0.04;
    let edge = 0.1;
    for s in [-1.0f32, 1.0] {
        at.part(
            lib,
            Vec3::new(0.0, ym + s * ((y1 - y0) * 0.5 - edge * 0.5), z),
            Vec3::new(w, edge, 0.01),
            Quat::IDENTITY,
            kind::CLOTH,
            [0.62, 0.45, 0.14, 1.0],
        );
        at.part(
            lib,
            Vec3::new(s * (w * 0.5 - edge * 0.5), ym, z),
            Vec3::new(edge, y1 - y0 - 2.0 * edge, 0.01),
            Quat::IDENTITY,
            kind::CLOTH,
            [0.62, 0.45, 0.14, 1.0],
        );
    }
    // The tree: a trunk, boughs in pairs, a crown of leaves.
    let gold = [0.7, 0.52, 0.18, 1.0];
    let z = 0.045;
    at.part(
        lib,
        Vec3::new(0.0, y0 + 0.95, z),
        Vec3::new(0.12, 1.3, 0.01),
        Quat::IDENTITY,
        kind::CLOTH,
        gold,
    );
    for (k, y) in [1.0f32, 1.35, 1.7].into_iter().enumerate() {
        let len = 1.2 - k as f32 * 0.25;
        for s in [-1.0f32, 1.0] {
            at.part(
                lib,
                Vec3::new(s * len * 0.42, y0 + y, z + 0.001),
                Vec3::new(len, 0.06, 0.01),
                Quat::from_rotation_z(s * 0.45),
                kind::CLOTH,
                gold,
            );
            at.part(
                lib,
                Vec3::new(s * len * 0.82, y0 + y + len * 0.36, z + 0.002),
                Vec3::new(0.14, 0.14, 0.01),
                Quat::from_rotation_z(FRAC_PI_4),
                kind::CLOTH,
                [0.75, 0.6, 0.25, 1.0],
            );
        }
    }
    at.part(
        lib,
        Vec3::new(0.0, y0 + 1.95, z + 0.002),
        Vec3::new(0.22, 0.22, 0.01),
        Quat::from_rotation_z(FRAC_PI_4),
        kind::CLOTH,
        [0.75, 0.6, 0.25, 1.0],
    );
}

/// The timeline: a band round the side and far walls over the cases, a gilt rule
/// along it that glows faintly, as if still being written, and a medallion for each
/// age above or below it.
fn timeline(lib: &mut Library, f: &RoomFrame) {
    let ah = RoomFrame::A_HALF;
    let (near, far) = (RoomFrame::O_NEAR, RoomFrame::O_FAR);
    let (y0, y1) = FRIEZE;
    let ym = (y0 + y1) * 0.5;
    let band = [0.5, 0.42, 0.3];
    // Each run: the wall's spot (facing into the room), and its length.
    let runs = [
        (Spot::new(f, -ah, (near + far) * 0.5, FRAC_PI_2), far - near),
        (Spot::new(f, 0.0, far, 0.0), 2.0 * ah),
        (Spot::new(f, ah, (near + far) * 0.5, -FRAC_PI_2), far - near),
    ];
    let coin = [
        (0.0, 0.0),
        (0.075, 0.0),
        (0.075, 0.022),
        (0.064, 0.026),
        (0.058, 0.018),
        (0.0, 0.018),
    ];
    let medallion = lib.shape(Mesh::lathe(&coin.map(Vec2::from), 24));
    for (i, (at, len)) in runs.into_iter().enumerate() {
        // The far wall's run stops short of the side walls' bands.
        let len = if i == 1 { len - 0.2 } else { len - 0.1 };
        at.part(
            lib,
            Vec3::new(0.0, ym, 0.02),
            Vec3::new(len, y1 - y0, 0.04),
            Quat::IDENTITY,
            kind::PARCHMENT,
            rgba(band, 1.0),
        );
        for s in [-1.0f32, 1.0] {
            at.part(
                lib,
                Vec3::new(0.0, ym + s * ((y1 - y0) * 0.5 + 0.025), 0.03),
                Vec3::new(len, 0.05, 0.06),
                Quat::IDENTITY,
                kind::WOOD,
                rgba(WALNUT, 1.0),
            );
        }
        at.part(
            lib,
            Vec3::new(0.0, ym, 0.043),
            Vec3::new(len - 0.1, 0.018, 0.008),
            Quat::IDENTITY,
            kind::EMISSIVE,
            [1.0, 0.72, 0.3, 1.6],
        );
        let n = (len / 1.6) as usize;
        for k in 0..n {
            let x = -len * 0.5 + (k as f32 + 0.5) * len / n as f32;
            let up = if k % 2 == 0 { 1.0 } else { -1.0 };
            lib.decor(medallion.at(
                at.at + at.q * Vec3::new(x, ym + up * 0.11, 0.038),
                at.q * Quat::from_rotation_x(FRAC_PI_2),
                kind::METAL,
                GOLD,
            ));
            at.part(
                lib,
                Vec3::new(x, ym + up * 0.045, 0.046),
                Vec3::new(0.012, 0.06, 0.006),
                Quat::IDENTITY,
                kind::METAL,
                GOLD,
            );
        }
    }
}

/// Polished marble, as `i` is shaped and coloured.
fn marble(i: Inst) -> Inst {
    Inst { param: 1, ..i }
}

/// A marble bust on a turned pedestal of green marble at the nave end of a stack on
/// side `s` of row `o`, looking into the nave; `k` picks its fashion.
fn bust(lib: &mut Library, f: &RoomFrame, s: f32, o: f32, k: usize) {
    let a = s * (NAVE - 0.3);
    guard(lib, f, (a - 0.27, a + 0.27), (o - 0.27, o + 0.27), 1.95);
    let at = Spot::new(f, a, o, -s * FRAC_PI_2);
    let white = rgba(MARBLE, 1.0);
    let green = [0.2, 0.26, 0.22, 1.0];
    let pedestal = [
        (0.0, 0.0),
        (0.26, 0.0),
        (0.26, 0.07),
        (0.235, 0.09),
        (0.235, 0.12),
        (0.2, 0.14),
        (0.17, 0.17),
        (0.15, 0.22),
        (0.13, 0.92),
        (0.15, 0.96),
        (0.15, 0.99),
        (0.2, 1.03),
        (0.24, 1.08),
        (0.25, 1.12),
        (0.25, 1.2),
        (0.0, 1.2),
    ];
    let pedestal = lib.shape(Mesh::lathe(&pedestal.map(Vec2::from), 40));
    lib.decor(marble(pedestal.at(at.at, at.q, kind::STONE, green)));
    let socle = [
        (0.0, 0.0),
        (0.1, 0.0),
        (0.1, 0.025),
        (0.065, 0.045),
        (0.055, 0.08),
        (0.07, 0.1),
        (0.0, 0.1),
    ];
    let socle = lib.shape(Mesh::lathe(&socle.map(Vec2::from), 24));
    lib.decor(marble(socle.at(at.at + Vec3::Y * 1.2, at.q, kind::STONE, white)));
    let round = lib.shape(Mesh::sphere(24));
    let neck = lib.shape(Mesh::cylinder(16));
    let put = |lib: &mut Library, s: Shape, size: Vec3, p: Vec3, q: Quat, kind: u32, color: [f32; 4]| {
        let i = s.sized(size).inst(at.at + at.q * p, at.q * q, kind, color);
        lib.decor(if kind == kind::STONE { marble(i) } else { i });
    };
    let flat = Quat::IDENTITY;
    // The chest and shoulders, cut away below; the neck, the head (an egg, narrowing
    // to the chin), a nose.
    let chest = [
        (0.0, 0.0),
        (0.09, 0.0),
        (0.15, 0.025),
        (0.19, 0.07),
        (0.21, 0.12),
        (0.2, 0.16),
        (0.15, 0.195),
        (0.07, 0.215),
        (0.0, 0.22),
    ];
    let chest = lib.shape(Mesh::lathe(&chest.map(Vec2::from), 32));
    lib.decor(marble(chest.sized(Vec3::new(0.42, 0.22, 0.24)).at(
        at.at + at.q * Vec3::Y * 1.3,
        at.q,
        kind::STONE,
        white,
    )));
    put(
        lib,
        neck,
        Vec3::new(0.085, 0.14, 0.085),
        Vec3::new(0.0, 1.55, 0.0),
        flat,
        kind::STONE,
        white,
    );
    let head = [
        (0.0, 0.0),
        (0.04, 0.0),
        (0.058, 0.025),
        (0.074, 0.075),
        (0.085, 0.135),
        (0.084, 0.18),
        (0.07, 0.222),
        (0.04, 0.25),
        (0.0, 0.258),
    ];
    let head = lib.shape(Mesh::lathe(&head.map(Vec2::from), 24));
    lib.decor(marble(head.sized(Vec3::new(0.165, 0.258, 0.2)).at(
        at.at + at.q * Vec3::Y * 1.585,
        at.q,
        kind::STONE,
        white,
    )));
    lib.decor(marble(Inst::new(
        at.at + at.q * Vec3::new(0.0, 1.69, 0.098),
        at.q * Quat::from_rotation_x(-0.25),
        Vec3::new(0.026, 0.05, 0.035),
        kind::STONE,
        white,
        0,
    )));
    match k % 4 {
        // A gilt laurel wreath.
        0 => {
            let wreath = lib.shape(Mesh::torus(0.3, 24, 8));
            put(
                lib,
                wreath,
                Vec3::new(0.205, 0.03, 0.24),
                Vec3::new(0.0, 1.79, -0.01),
                Quat::from_rotation_x(-0.18),
                kind::METAL,
                GOLD,
            );
        }
        // A full beard.
        1 => put(
            lib,
            round,
            Vec3::new(0.135, 0.12, 0.09),
            Vec3::new(0.0, 1.605, 0.035),
            flat,
            kind::STONE,
            white,
        ),
        // A crown: a gold band and its points.
        2 => {
            let band = [(0.098, 0.0), (0.108, 0.0), (0.108, 0.04), (0.098, 0.04), (0.098, 0.0)];
            let band = lib.shape(Mesh::lathe(&band.map(Vec2::from), 24));
            lib.decor(band.at(at.at + at.q * Vec3::Y * 1.77, at.q, kind::METAL, GOLD));
            let point = lib.shape(Mesh::cone(8));
            for i in 0..6 {
                let t = i as f32 / 6.0 * TAU;
                let p = Vec3::new(libm::cosf(t) * 0.103, 1.83, libm::sinf(t) * 0.103);
                put(lib, point, Vec3::new(0.025, 0.04, 0.025), p, flat, kind::METAL, GOLD);
            }
        }
        // Hair to the shoulders.
        _ => put(
            lib,
            round,
            Vec3::new(0.2, 0.3, 0.19),
            Vec3::new(0.0, 1.66, -0.035),
            flat,
            kind::STONE,
            white,
        ),
    }
}

/// A heraldic banner hung over the nave at `a`, `o`, facing the door, stirring now
/// and then (two slow swings about its pole, in and out of step); `k` picks its
/// colours and charge.
fn banner(lib: &mut Library, f: &RoomFrame, a: f32, o: f32, k: usize) {
    let at = Spot::new(f, a, o, 0.0);
    let fields = [
        [0.42, 0.04, 0.05],
        [0.05, 0.1, 0.34],
        [0.05, 0.24, 0.1],
        [0.26, 0.06, 0.28],
    ];
    let field = rgba(fields[k % 4], 1.0);
    let charge = if k % 3 == 1 {
        [0.78, 0.76, 0.7, 1.0]
    } else {
        [0.72, 0.54, 0.16, 1.0]
    };
    let (w, top, bottom) = (0.95f32, 5.15f32, 3.55f32);
    let ym = (top + bottom) * 0.5;
    // The pole and its cords to the ceiling.
    let pole = lib.shape(Mesh::cylinder(12));
    lib.decor(pole.sized(Vec3::new(0.04, w + 0.25, 0.04)).inst(
        at.at + Vec3::Y * (top + 0.04),
        at.q * Quat::from_rotation_z(FRAC_PI_2),
        kind::METAL,
        GOLD,
    ));
    let knob = lib.shape(Mesh::sphere(12));
    for x in [-1.0f32, 1.0] {
        lib.decor(knob.sized(Vec3::splat(0.065)).inst(
            at.at + at.q * Vec3::new(x * (w * 0.5 + 0.15), top + 0.04, 0.0),
            at.q,
            kind::METAL,
            GOLD,
        ));
        let cord = RoomFrame::HEIGHT - top;
        at.part(
            lib,
            Vec3::new(x * w * 0.4, top + cord * 0.5, 0.0),
            Vec3::new(0.012, cord, 0.012),
            Quat::IDENTITY,
            kind::PLAIN,
            [0.25, 0.2, 0.12, 1.0],
        );
    }
    let hang = at.at + Vec3::Y * (top + 0.02);
    let axis = at.q * Vec3::X;
    let swing = |amp: f32, period: f32| Motion::Swing { axis, amp, period };
    let first = lib.mover(hang, swing(0.045, 7.0 + 0.6 * k as f32), 2.3 * k as f32, None);
    let m = lib.mover(hang, swing(0.045, 9.1 + 0.4 * k as f32), 1.1 * k as f32, Some(first));
    // The field, swallowtailed.
    let tail = 0.42;
    let outline = [
        Vec2::new(-w * 0.5, top),
        Vec2::new(-w * 0.5, bottom - tail),
        Vec2::new(0.0, bottom - 0.04),
        Vec2::new(w * 0.5, bottom - tail),
        Vec2::new(w * 0.5, top),
    ];
    let cloth = lib.shape(Mesh::prism(&outline, 0.025));
    lib.moving(m, cloth.at(at.at, at.q, kind::CLOTH, field));
    let z = 0.016;
    let mut put = |p: Vec3, size: Vec3, q: Quat| {
        lib.moving(m, Inst::new(at.at + at.q * p, at.q * q, size, kind::CLOTH, charge, 0));
    };
    match k % 3 {
        // A chevron.
        0 => {
            for x in [-1.0f32, 1.0] {
                put(
                    Vec3::new(x * 0.2, ym - 0.05, z),
                    Vec3::new(0.56, 0.12, 0.008),
                    Quat::from_rotation_z(-x * 0.7),
                );
            }
        }
        // A cross.
        1 => {
            put(Vec3::new(0.0, ym, z), Vec3::new(0.12, 1.2, 0.008), Quat::IDENTITY);
            put(
                Vec3::new(0.0, ym + 0.15, z),
                Vec3::new(w - 0.1, 0.12, 0.008),
                Quat::IDENTITY,
            );
        }
        // A lozenge between two bars.
        _ => {
            put(
                Vec3::new(0.0, ym, z),
                Vec3::new(0.4, 0.4, 0.008),
                Quat::from_rotation_z(FRAC_PI_4),
            );
            for y in [-0.55f32, 0.55] {
                put(
                    Vec3::new(0.0, ym + y, z),
                    Vec3::new(w - 0.1, 0.07, 0.008),
                    Quat::IDENTITY,
                );
            }
        }
    }
}

/// The Great Chronicle, open on its turned stand at the end of the nave, a quill
/// writing in it by itself, and the newest line of ink still bright.
fn chronicle(lib: &mut Library, f: &RoomFrame) {
    let o = 29.85;
    guard(lib, f, (-0.45, 0.45), (o - 0.35, o + 0.35), 1.2);
    let at = Spot::new(f, 0.0, o, 0.0);
    let wood = rgba(WALNUT, 1.0);
    let dark = rgba(DARK_WOOD, 1.0);
    at.part(
        lib,
        Vec3::Y * 0.05,
        Vec3::new(0.8, 0.1, 0.62),
        Quat::IDENTITY,
        kind::WOOD,
        dark,
    );
    let column = [
        (0.0, 0.0),
        (0.17, 0.0),
        (0.17, 0.04),
        (0.13, 0.07),
        (0.105, 0.12),
        (0.085, 0.3),
        (0.105, 0.45),
        (0.085, 0.6),
        (0.07, 0.7),
        (0.09, 0.76),
        (0.14, 0.8),
        (0.14, 0.84),
        (0.0, 0.84),
    ];
    let column = lib.shape(Mesh::lathe(&column.map(Vec2::from), 32));
    lib.decor(column.at(at.at + Vec3::Y * 0.1, at.q, kind::WOOD, wood));
    // The desk slopes toward the door, on a block over the column.
    let slope = Quat::from_rotation_x(0.38);
    let desk = Vec3::new(0.0, 1.0, 0.0);
    at.part(
        lib,
        desk - slope * Vec3::Y * 0.06,
        Vec3::new(0.34, 0.1, 0.34),
        slope,
        kind::WOOD,
        dark,
    );
    at.part(lib, desk, Vec3::new(0.9, 0.05, 0.6), slope, kind::WOOD, wood);
    at.part(
        lib,
        desk + slope * Vec3::new(0.0, 0.03, 0.31),
        Vec3::new(0.9, 0.06, 0.03),
        slope,
        kind::WOOD,
        dark,
    );
    // The book: its boards, a thick block of pages each side of the gutter.
    at.part(
        lib,
        desk + slope * Vec3::new(0.0, 0.035, 0.0),
        Vec3::new(0.86, 0.02, 0.56),
        slope,
        kind::CLOTH,
        [0.3, 0.06, 0.05, 1.0],
    );
    for x in [-1.0f32, 1.0] {
        at.part(
            lib,
            desk + slope * Vec3::new(x * 0.205, 0.07, 0.0),
            Vec3::new(0.39, 0.05, 0.52),
            slope * Quat::from_rotation_z(-x * 0.05),
            kind::PAPER,
            [0.0, 0.0, 0.0, 0.0],
        );
    }
    // Lines written, and the newest one glowing.
    for i in 0..6 {
        for x in [-1.0f32, 1.0] {
            let w = if x > 0.0 && i == 5 { 0.12 } else { 0.3 };
            let is_new = x > 0.0 && i == 4;
            at.part(
                lib,
                desk + slope * Vec3::new(x * 0.2 - (0.3 - w) * 0.5, 0.098, -0.2 + i as f32 * 0.07),
                Vec3::new(w, 0.004, 0.012),
                slope,
                if is_new { kind::EMISSIVE } else { kind::PLAIN },
                if is_new {
                    [1.0, 0.75, 0.35, 2.5]
                } else {
                    [0.15, 0.1, 0.08, 1.0]
                },
            );
        }
    }
    // The quill, upright at the end of the line it is writing as if in a hand: its nib
    // scratching to and fro, the whole of it moving on along the line and back.
    let tip = at.at + at.q * (desk + slope * Vec3::new(0.17, 0.104, 0.15));
    let lean = at.q * Quat::from_rotation_z(-0.55) * Quat::from_rotation_x(-0.25);
    let along = Motion::Slide {
        travel: at.q * slope * Vec3::X * 0.06,
        period: 14.0,
    };
    let hand = lib.mover(tip, along, 0.0, None);
    let scratch = Motion::Swing {
        axis: at.q * slope * Vec3::Y,
        amp: 0.06,
        period: 1.9,
    };
    let m = lib.mover(tip + lean * Vec3::Y * 0.22, scratch, 0.0, Some(hand));
    let shaft = lib.shape(Mesh::cylinder(8));
    lib.moving(
        m,
        shaft.sized(Vec3::new(0.006, 0.3, 0.006)).inst(
            tip + lean * Vec3::Y * 0.15,
            lean,
            kind::PLAIN,
            [0.85, 0.82, 0.75, 1.0],
        ),
    );
    let vane = [
        (0.0, 0.06),
        (0.014, 0.1),
        (0.028, 0.18),
        (0.026, 0.26),
        (0.008, 0.31),
        (0.0, 0.3),
        (-0.012, 0.27),
        (-0.018, 0.18),
        (-0.01, 0.1),
    ];
    let vane = lib.shape(Mesh::prism(&vane.map(Vec2::from), 0.003));
    lib.moving(m, vane.at(tip, lean, kind::CLOTH, [0.88, 0.86, 0.8, 1.0]));
    let nib = lib.shape(Mesh::sphere(8));
    lib.moving(
        m,
        nib.sized(Vec3::splat(0.014))
            .inst(tip, lean, kind::EMISSIVE, [1.0, 0.75, 0.35, 4.0]),
    );
    lib.light(
        at.at + Vec3::Y * 1.6 + at.q * Vec3::Z * 0.6,
        4.0,
        [1.0, 0.7, 0.35],
        0.6,
        LightKind::Static,
    );
}

/// A suit of armour on a low stand against side wall `s` at `o`, facing across the
/// room: turned plates, a round helm, a shield bearing a coat of arms, and a sword
/// point down before it.
fn armour(lib: &mut Library, f: &RoomFrame, s: f32, o: f32) {
    let a = s * (RoomFrame::A_HALF - 0.55);
    guard(lib, f, (a - 0.35, a + 0.35), (o - 0.35, o + 0.35), 2.0);
    let at = Spot::new(f, a, o, -s * FRAC_PI_2);
    let steel = [0.42, 0.44, 0.47, 1.0];
    let dark = rgba(DARK_WOOD, 1.0);
    let lathe = |lib: &mut Library, profile: &[(f32, f32)], segs: usize| {
        let profile: Vec<Vec2> = profile.iter().map(|&p| Vec2::from(p)).collect();
        lib.shape(Mesh::lathe(&profile, segs))
    };
    let put = |lib: &mut Library, s: Shape, p: Vec3, q: Quat, color: [f32; 4]| {
        lib.decor(s.at(at.at + at.q * p, at.q * q, kind::METAL, color));
    };
    at.part(
        lib,
        Vec3::Y * 0.05,
        Vec3::new(0.66, 0.1, 0.66),
        Quat::IDENTITY,
        kind::WOOD,
        dark,
    );
    let leg = lathe(
        lib,
        &[
            (0.0, 0.0),
            (0.05, 0.0),
            (0.058, 0.1),
            (0.052, 0.3),
            (0.064, 0.38),
            (0.058, 0.45),
            (0.07, 0.7),
            (0.075, 0.84),
            (0.0, 0.84),
        ],
        16,
    );
    let pauldron = lib.shape(Mesh::dome(16));
    let arm = lathe(
        lib,
        &[
            (0.0, 0.0),
            (0.045, 0.0),
            (0.05, 0.25),
            (0.058, 0.32),
            (0.05, 0.4),
            (0.055, 0.62),
            (0.0, 0.62),
        ],
        12,
    );
    let fist = lib.shape(Mesh::sphere(12));
    for x in [-1.0f32, 1.0] {
        at.part(
            lib,
            Vec3::new(x * 0.11, 0.14, 0.04),
            Vec3::new(0.12, 0.08, 0.24),
            Quat::IDENTITY,
            kind::METAL,
            steel,
        );
        put(lib, leg, Vec3::new(x * 0.11, 0.18, 0.0), Quat::IDENTITY, steel);
        // The arm hangs from the shoulder, its foot the wrist.
        let hang = Quat::from_rotation_z(x * 0.12) * Quat::from_rotation_x(PI);
        let shoulder = Vec3::new(x * 0.27, 1.51, 0.02);
        put(lib, arm, shoulder, hang, steel);
        let wrist = shoulder + hang * Vec3::Y * 0.62;
        lib.decor(fist.sized(Vec3::new(0.09, 0.1, 0.1)).inst(
            at.at + at.q * (wrist - Vec3::Y * 0.04),
            at.q,
            kind::METAL,
            steel,
        ));
        lib.decor(pauldron.sized(Vec3::new(0.21, 0.11, 0.23)).inst(
            at.at + at.q * Vec3::new(x * 0.25, 1.53, 0.0),
            at.q * Quat::from_rotation_z(-x * 0.3),
            kind::METAL,
            steel,
        ));
    }
    let fauld = lathe(lib, &[(0.0, 0.0), (0.19, 0.0), (0.17, 0.2), (0.0, 0.2)], 24);
    lib.decor(
        fauld
            .sized(Vec3::new(0.38, 0.2, 0.26))
            .inst(at.at + at.q * Vec3::Y * 1.1, at.q, kind::METAL, steel),
    );
    let breast = lathe(
        lib,
        &[
            (0.0, 0.0),
            (0.17, 0.0),
            (0.21, 0.12),
            (0.21, 0.22),
            (0.16, 0.33),
            (0.08, 0.36),
            (0.0, 0.36),
        ],
        32,
    );
    lib.decor(
        breast
            .sized(Vec3::new(0.42, 0.36, 0.27))
            .inst(at.at + at.q * Vec3::Y * 1.38, at.q, kind::METAL, steel),
    );
    let helm = lathe(
        lib,
        &[
            (0.0, 0.0),
            (0.105, 0.0),
            (0.11, 0.14),
            (0.1, 0.21),
            (0.065, 0.265),
            (0.0, 0.28),
        ],
        24,
    );
    lib.decor(
        helm.sized(Vec3::new(0.22, 0.28, 0.25))
            .inst(at.at + at.q * Vec3::Y * 1.73, at.q, kind::METAL, steel),
    );
    at.part(
        lib,
        Vec3::new(0.0, 1.75, 0.119),
        Vec3::new(0.08, 0.016, 0.01),
        Quat::IDENTITY,
        kind::PLAIN,
        [0.02, 0.02, 0.02, 1.0],
    );
    // A plume rising from the crest and falling back.
    let plume: Vec<Vec3> = (0..=10)
        .map(|k| {
            let t = k as f32 / 10.0;
            Vec3::new(0.0, 1.85 + 0.12 * libm::sinf(t * 2.4), 0.02 - 0.24 * t)
        })
        .collect();
    let plume = lib.shape(Mesh::sweep(&plume, &[0.02, 0.04, 0.035, 0.01], &circle(10), true));
    lib.decor(plume.at(at.at, at.q, kind::CLOTH, [0.5, 0.06, 0.06, 1.0]));
    // A heater shield on the left arm, turned toward the room, its arms painted on
    // parchment in a steel rim.
    let half = [
        (0.18, 0.25),
        (0.18, 0.05),
        (0.165, -0.06),
        (0.13, -0.15),
        (0.075, -0.215),
    ];
    let mut outline: Vec<Vec2> = half.iter().map(|&p| Vec2::from(p)).collect();
    outline.push(Vec2::new(0.0, -0.25));
    outline.extend(half.iter().rev().map(|&(x, y)| Vec2::new(-x, y)));
    let face = lib.shape(Mesh::prism(&outline, 0.015));
    let rim: Vec<Vec2> = outline.iter().map(|p| *p * 1.08).collect();
    let rim = lib.shape(Mesh::prism(&rim, 0.02));
    let turn = Quat::from_rotation_y(-0.64);
    let shield = Vec3::new(-0.42, 1.05, 0.12);
    put(lib, rim, shield - turn * Vec3::Z * 0.006, turn, steel);
    let mut arms = face.at(
        at.at + at.q * shield,
        at.q * turn,
        kind::PARCHMENT,
        [0.8, 0.72, 0.55, 1.0],
    );
    arms.param = 5;
    lib.decor(arms);
    // The sword, point down: blade, cross-guard, grip and pommel.
    at.part(
        lib,
        Vec3::new(0.0, 0.75, 0.2),
        Vec3::new(0.04, 1.3, 0.012),
        Quat::IDENTITY,
        kind::METAL,
        [0.6, 0.62, 0.66, 1.0],
    );
    at.part(
        lib,
        Vec3::new(0.0, 1.38, 0.2),
        Vec3::new(0.24, 0.03, 0.03),
        Quat::IDENTITY,
        kind::METAL,
        GOLD,
    );
    let grip = lib.shape(Mesh::cylinder(8));
    lib.decor(grip.sized(Vec3::new(0.026, 0.13, 0.026)).inst(
        at.at + at.q * Vec3::new(0.0, 1.46, 0.2),
        at.q,
        kind::CLOTH,
        [0.18, 0.08, 0.04, 1.0],
    ));
    lib.decor(
        fist.sized(Vec3::splat(0.045))
            .inst(at.at + at.q * Vec3::new(0.0, 1.545, 0.2), at.q, kind::METAL, GOLD),
    );
}

/// `n` points round the unit circle in xy, anticlockwise.
fn circle(n: usize) -> Vec<Vec2> {
    (0..n).map(|k| Vec2::from_angle(k as f32 / n as f32 * TAU)).collect()
}
