//! The Chronicle Hall: department 5, first floor, east side. A stately long room: a
//! nave from the door to the Great Chronicle on its stand, the stacks running out from
//! it either side with a marble bust at the end of each, heraldic banners hung down
//! the nave, and a timeline that writes itself along the walls.

use super::*;
use std::f32::consts::{FRAC_PI_2, FRAC_PI_4};

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
    lib.decor(runner);
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
            at.part(
                lib,
                Vec3::new(x, ym + up * 0.11, 0.05),
                Vec3::new(0.15, 0.15, 0.025),
                Quat::from_rotation_z(FRAC_PI_4),
                kind::METAL,
                GOLD,
            );
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

/// A marble bust on a plinth at the nave end of a stack on side `s` of row `o`,
/// looking into the nave; `k` picks its fashion.
fn bust(lib: &mut Library, f: &RoomFrame, s: f32, o: f32, k: usize) {
    let a = s * (NAVE - 0.3);
    guard(lib, f, (a - 0.27, a + 0.27), (o - 0.27, o + 0.27), 1.95);
    let at = Spot::new(f, a, o, -s * FRAC_PI_2);
    let white = rgba(MARBLE, 1.0);
    let green = [0.2, 0.24, 0.21, 1.0];
    let block = |lib: &mut Library, p: Vec3, size: Vec3, q: Quat, color: [f32; 4]| {
        at.part(lib, p, size, q, kind::PLAIN, color);
    };
    let flat = Quat::IDENTITY;
    let eighth = Quat::from_rotation_y(FRAC_PI_4);
    // The plinth, of green marble.
    block(lib, Vec3::Y * 0.06, Vec3::new(0.54, 0.12, 0.54), flat, green);
    block(lib, Vec3::Y * 0.62, Vec3::new(0.4, 1.0, 0.4), flat, green);
    block(lib, Vec3::Y * 1.16, Vec3::new(0.5, 0.08, 0.5), flat, green);
    // A socle, the chest and shoulders, the neck, the head; each rounded by a twin
    // turned an eighth.
    block(lib, Vec3::Y * 1.25, Vec3::new(0.16, 0.1, 0.16), flat, white);
    block(lib, Vec3::Y * 1.25, Vec3::new(0.13, 0.1, 0.13), eighth, white);
    block(lib, Vec3::Y * 1.35, Vec3::new(0.3, 0.1, 0.17), flat, white);
    block(lib, Vec3::Y * 1.45, Vec3::new(0.4, 0.12, 0.2), flat, white);
    for x in [-1.0f32, 1.0] {
        block(
            lib,
            Vec3::new(x * 0.17, 1.49, 0.0),
            Vec3::new(0.1, 0.1, 0.19),
            Quat::from_rotation_z(FRAC_PI_4),
            white,
        );
    }
    block(lib, Vec3::Y * 1.56, Vec3::new(0.09, 0.08, 0.09), flat, white);
    block(lib, Vec3::Y * 1.71, Vec3::new(0.17, 0.23, 0.19), flat, white);
    block(lib, Vec3::Y * 1.71, Vec3::new(0.14, 0.22, 0.14), eighth, white);
    block(lib, Vec3::new(0.0, 1.7, 0.1), Vec3::new(0.03, 0.06, 0.04), flat, white);
    match k % 4 {
        // A laurel wreath.
        0 => block(
            lib,
            Vec3::Y * 1.79,
            Vec3::new(0.19, 0.035, 0.21),
            flat,
            [0.25, 0.35, 0.14, 1.0],
        ),
        // A beard.
        1 => block(
            lib,
            Vec3::new(0.0, 1.6, 0.05),
            Vec3::new(0.15, 0.12, 0.12),
            Quat::from_rotation_x(-0.25),
            white,
        ),
        // A crown.
        2 => {
            at.part(
                lib,
                Vec3::Y * 1.85,
                Vec3::new(0.18, 0.045, 0.2),
                flat,
                kind::METAL,
                GOLD,
            );
            for x in [-0.06f32, 0.0, 0.06] {
                at.part(
                    lib,
                    Vec3::new(x, 1.89, 0.09),
                    Vec3::new(0.03, 0.04, 0.03),
                    Quat::from_rotation_z(FRAC_PI_4),
                    kind::METAL,
                    GOLD,
                );
            }
        }
        // Hair to the shoulders.
        _ => block(
            lib,
            Vec3::new(0.0, 1.67, -0.03),
            Vec3::new(0.19, 0.28, 0.18),
            flat,
            white,
        ),
    }
}

/// A heraldic banner hung over the nave at `a`, `o`, facing the door; `k` picks its
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
    at.part(
        lib,
        Vec3::Y * (top + 0.04),
        Vec3::new(w + 0.25, 0.04, 0.04),
        Quat::IDENTITY,
        kind::METAL,
        GOLD,
    );
    for x in [-1.0f32, 1.0] {
        at.part(
            lib,
            Vec3::new(x * (w * 0.5 + 0.14), top + 0.04, 0.0),
            Vec3::new(0.06, 0.06, 0.06),
            Quat::IDENTITY,
            kind::METAL,
            GOLD,
        );
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
    at.part(
        lib,
        Vec3::Y * ym,
        Vec3::new(w, top - bottom, 0.025),
        Quat::IDENTITY,
        kind::CLOTH,
        field,
    );
    // The swallowtail.
    for x in [-1.0f32, 1.0] {
        at.part(
            lib,
            Vec3::new(x * w * 0.27, bottom - 0.2, 0.0),
            Vec3::new(w * 0.36, 0.48, 0.025),
            Quat::from_rotation_z(x * 0.32),
            kind::CLOTH,
            field,
        );
    }
    let z = 0.016;
    match k % 3 {
        // A chevron.
        0 => {
            for x in [-1.0f32, 1.0] {
                at.part(
                    lib,
                    Vec3::new(x * 0.2, ym - 0.05, z),
                    Vec3::new(0.56, 0.12, 0.008),
                    Quat::from_rotation_z(-x * 0.7),
                    kind::CLOTH,
                    charge,
                );
            }
        }
        // A cross.
        1 => {
            at.part(
                lib,
                Vec3::Y * ym,
                Vec3::new(0.12, 1.2, 0.008),
                Quat::IDENTITY,
                kind::CLOTH,
                charge,
            );
            at.part(
                lib,
                Vec3::Y * (ym + 0.15),
                Vec3::new(w - 0.1, 0.12, 0.008),
                Quat::IDENTITY,
                kind::CLOTH,
                charge,
            );
        }
        // A lozenge between two bars.
        _ => {
            at.part(
                lib,
                Vec3::Y * ym,
                Vec3::new(0.4, 0.4, 0.008),
                Quat::from_rotation_z(FRAC_PI_4),
                kind::CLOTH,
                charge,
            );
            for y in [-0.55f32, 0.55] {
                at.part(
                    lib,
                    Vec3::Y * (ym + y),
                    Vec3::new(w - 0.1, 0.07, 0.008),
                    Quat::IDENTITY,
                    kind::CLOTH,
                    charge,
                );
            }
        }
    }
}

/// The Great Chronicle, open on its stand at the end of the nave, a quill hanging
/// over it in mid-stroke and the newest line of ink still bright.
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
    at.part(
        lib,
        Vec3::Y * 0.5,
        Vec3::new(0.22, 0.8, 0.22),
        Quat::IDENTITY,
        kind::WOOD,
        wood,
    );
    for x in [-1.0f32, 1.0] {
        at.part(
            lib,
            Vec3::new(x * 0.2, 0.5, 0.0),
            Vec3::new(0.06, 0.7, 0.3),
            Quat::IDENTITY,
            kind::WOOD,
            dark,
        );
    }
    // The desk slopes toward the door.
    let slope = Quat::from_rotation_x(0.38);
    let desk = Vec3::new(0.0, 1.0, 0.0);
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
    // The quill, upright over the page as if in a hand.
    let tip = desk + slope * Vec3::new(0.14, 0.13, 0.08);
    let lean = Quat::from_rotation_z(-0.55) * Quat::from_rotation_x(-0.25);
    at.part(
        lib,
        tip + lean * Vec3::Y * 0.15,
        Vec3::new(0.008, 0.3, 0.008),
        lean,
        kind::PLAIN,
        [0.85, 0.82, 0.75, 1.0],
    );
    at.part(
        lib,
        tip + lean * Vec3::Y * 0.2,
        Vec3::new(0.06, 0.22, 0.006),
        lean,
        kind::CLOTH,
        [0.88, 0.86, 0.8, 1.0],
    );
    at.part(
        lib,
        tip,
        Vec3::new(0.02, 0.02, 0.02),
        Quat::IDENTITY,
        kind::EMISSIVE,
        [1.0, 0.75, 0.35, 4.0],
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
/// room.
fn armour(lib: &mut Library, f: &RoomFrame, s: f32, o: f32) {
    let a = s * (RoomFrame::A_HALF - 0.55);
    guard(lib, f, (a - 0.35, a + 0.35), (o - 0.35, o + 0.35), 2.0);
    let at = Spot::new(f, a, o, -s * FRAC_PI_2);
    let steel = [0.42, 0.44, 0.47, 1.0];
    let dark = rgba(DARK_WOOD, 1.0);
    at.part(
        lib,
        Vec3::Y * 0.05,
        Vec3::new(0.66, 0.1, 0.66),
        Quat::IDENTITY,
        kind::WOOD,
        dark,
    );
    for x in [-1.0f32, 1.0] {
        at.part(
            lib,
            Vec3::new(x * 0.11, 0.14, 0.04),
            Vec3::new(0.12, 0.08, 0.24),
            Quat::IDENTITY,
            kind::METAL,
            steel,
        );
        at.part(
            lib,
            Vec3::new(x * 0.11, 0.6, 0.0),
            Vec3::new(0.13, 0.84, 0.15),
            Quat::IDENTITY,
            kind::METAL,
            steel,
        );
        at.part(
            lib,
            Vec3::new(x * 0.31, 1.25, 0.02),
            Vec3::new(0.1, 0.62, 0.12),
            Quat::from_rotation_z(x * 0.12),
            kind::METAL,
            steel,
        );
        at.part(
            lib,
            Vec3::new(x * 0.26, 1.53, 0.0),
            Vec3::new(0.2, 0.12, 0.22),
            Quat::IDENTITY,
            kind::METAL,
            steel,
        );
    }
    at.part(
        lib,
        Vec3::Y * 1.1,
        Vec3::new(0.36, 0.2, 0.22),
        Quat::IDENTITY,
        kind::METAL,
        steel,
    );
    at.part(
        lib,
        Vec3::Y * 1.36,
        Vec3::new(0.42, 0.36, 0.26),
        Quat::IDENTITY,
        kind::METAL,
        steel,
    );
    at.part(
        lib,
        Vec3::Y * 1.73,
        Vec3::new(0.22, 0.28, 0.25),
        Quat::IDENTITY,
        kind::METAL,
        steel,
    );
    at.part(
        lib,
        Vec3::new(0.0, 1.75, 0.126),
        Vec3::new(0.16, 0.02, 0.004),
        Quat::IDENTITY,
        kind::PLAIN,
        [0.02, 0.02, 0.02, 1.0],
    );
    at.part(
        lib,
        Vec3::new(0.0, 1.93, -0.02),
        Vec3::new(0.03, 0.12, 0.2),
        Quat::IDENTITY,
        kind::CLOTH,
        [0.5, 0.06, 0.06, 1.0],
    );
    // A shield on the left arm, a sword point-down before it.
    at.part(
        lib,
        Vec3::new(-0.42, 1.05, 0.12),
        Vec3::new(0.05, 0.5, 0.36),
        Quat::from_rotation_y(-0.5),
        kind::WOOD,
        [0.4, 0.05, 0.05, 1.0],
    );
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
}
