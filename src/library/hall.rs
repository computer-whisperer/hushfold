//! The Hall: one storey, two wings of freestanding bookcases either side of a reading
//! room, the East Wing veiled until the West is mostly sorted. Its generation must
//! not change (saves name books by index; see `hall_generates_as_before`): what is
//! added to it is drawn only, or solid only where no pile could ever have lain.

use super::*;
use std::f32::consts::{FRAC_PI_2, FRAC_PI_4, PI};

pub const HALL_X: f32 = 24.0;
pub const HALL_Z: f32 = 18.0;
pub const HALL_H: f32 = 8.0;
pub const VEIL_X: f32 = 11.6;
const CASES_PER_CATEGORY: usize = 3;
const WALL_T: f32 = 0.5;
const WINDOWS_X: [f32; 8] = [-20.0, -15.5, -7.0, -2.3, 2.3, 7.0, 15.5, 20.0];
/// A north window's opening: half its width, its sill, and where its round head springs.
const WIN_HALF: f32 = 0.8;
const WIN_SILL: f32 = 2.3;
const WIN_SPRING: f32 = 5.6;
const WIN_TOP: f32 = WIN_SPRING + WIN_HALF;
/// Columns stand on the veil's line and its mirror at these z.
const COLUMNS_Z: [f32; 4] = [-16.0, -8.0, 8.0, 16.0];
/// Beams run east to west at these z, the deeper ones over the columns; girders carry
/// them north to south on the columns.
const BEAMS_Z: [f32; 9] = [-16.0, -12.0, -8.0, -4.0, 0.0, 4.0, 8.0, 12.0, 16.0];
/// Ribs coffer the reading room's ceiling between the girders; purlins halve the wings'.
const RIBS_X: [f32; 5] = [-7.733, -3.867, 0.0, 3.867, 7.733];
const PURLIN_X: f32 = 17.75;
/// Undersides of the timbers, deepest first; each its own height, so no two share a face.
const GIRDER_Y: f32 = 7.2;
const BEAM_MAIN_Y: f32 = 7.4;
const BEAM_Y: f32 = 7.55;
const RIB_Y: f32 = 7.65;
const PLATE_Y: f32 = 7.7;
/// Where the arcade's braces spring from the columns.
const SPRINGING: f32 = 4.95;

const BEAM_WOOD: [f32; 3] = [0.11, 0.055, 0.025];
const COLUMN: [f32; 3] = [0.42, 0.39, 0.35];
const COLUMN_TRIM: [f32; 3] = [0.3, 0.28, 0.25];
const DRESSED: [f32; 3] = [0.45, 0.42, 0.38];
const GILT: [f32; 3] = [0.78, 0.56, 0.2];
const IRON: [f32; 3] = [0.07, 0.065, 0.06];
/// The reading room's ceiling is painted as the night sky.
const SKY: [f32; 3] = [0.05, 0.075, 0.2];
/// The wings' ceilings, pale between dark timbers.
const PLASTER: [f32; 3] = [0.3, 0.27, 0.22];
const VELVET: [f32; 3] = [0.3, 0.035, 0.045];
const CREAM: [f32; 3] = [0.8, 0.74, 0.58];

/// An instance's colour for a kind that does not glow (whose w is unused).
fn rgba(c: [f32; 3]) -> [f32; 4] {
    [c[0], c[1], c[2], 1.0]
}

pub(super) fn build(lib: &mut Library, rng: &mut StdRng) {
    lib.world.levels.push(Level {
        y: 0.0,
        roof: HALL_H,
        min: Vec2::new(-HALL_X, -HALL_Z),
        max: Vec2::new(HALL_X, HALL_Z),
    });
    lib.world.slabs.push(Slab {
        level: 0,
        min: Vec2::new(-HALL_X, -HALL_Z),
        max: Vec2::new(HALL_X, HALL_Z),
    });
    lib.spawn = (Vec3::new(0.0, 0.0, 6.5), 0.0);
    // The middle of the reading room, between its tables.
    lib.hub = Vec3::ZERO;
    lib.build_hall();
    lib.build_stacks(rng);
    lib.build_center();
    lib.build_piles(rng);
    // Walls, ceiling and floor last: drawn behind everything else, they are mostly
    // hidden by it before they are shaded.
    lib.build_shell();
    // The wings are the rooms, the veil's line their doorway (the reading room is
    // the West Wing's).
    lib.rooms.push(Room {
        level: 0,
        min: Vec2::new(-HALL_X, -HALL_Z),
        max: Vec2::new(VEIL_X, HALL_Z),
        door: Vec3::new(VEIL_X - 0.6, 0.0, 0.0),
        drop: Vec3::new(-11.4, 0.0, 0.0),
    });
    lib.rooms.push(Room {
        level: 0,
        min: Vec2::new(VEIL_X, -HALL_Z),
        max: Vec2::new(HALL_X, HALL_Z),
        door: Vec3::new(VEIL_X + 0.4, 0.0, 0.0),
        drop: Vec3::new(12.6, 0.0, 0.0),
    });
    // The veil hangs between the east columns until 40% of the West Wing is home.
    lib.world.add_gate(Gate {
        level: 0,
        min: Vec3::new(VEIL_X - 0.05, 0.0, -HALL_Z),
        max: Vec3::new(VEIL_X + 0.05, HALL_H, HALL_Z),
        opens: 1,
        counts: vec![0],
        need: 0.4,
    });
    // Moonbeams through the north windows.
    let beam_dir = Vec3::new(0.0, -0.62, 1.0).normalize();
    for x in WINDOWS_X {
        let start = Vec3::new(x, 4.4, -HALL_Z + 0.1);
        let len = 9.0;
        let rot = Quat::from_rotation_arc(Vec3::Y, beam_dir);
        lib.static_trans.push(Inst::new(
            start + beam_dir * len * 0.5,
            rot,
            Vec3::new(1.6, len, 3.4),
            kind::SHAFT,
            [0.35, 0.45, 0.9, 0.05],
            0,
        ));
    }
}

/// A place on a wall's face: `at` on the face, `n` out of it into the room. Offsets
/// and sizes on it are (to the right as you face it, up, out).
#[derive(Clone, Copy)]
struct Mount {
    at: Vec3,
    n: Vec3,
}

impl Mount {
    fn right(self) -> Vec3 {
        Vec3::Y.cross(self.n)
    }

    fn at(self, o: Vec3) -> Vec3 {
        self.at + self.right() * o.x + Vec3::Y * o.y + self.n * o.z
    }

    /// Facing out of the wall, turned `spin` about its normal.
    fn rot(self, spin: f32) -> Quat {
        Quat::from_mat3(&Mat3::from_cols(self.right(), Vec3::Y, self.n)) * Quat::from_rotation_z(spin)
    }
}

impl Library {
    /// The columns, the girders and timbers they carry, the windows, sconces and
    /// everything hung on the walls.
    fn build_hall(&mut self) {
        for sx in [-VEIL_X, VEIL_X] {
            for z in COLUMNS_Z {
                self.column(Vec3::new(sx, 0.0, z));
            }
            self.arcade(sx);
        }
        self.ceiling();
        for x in WINDOWS_X {
            self.window(x);
        }
        self.wall_dressing();
        self.wainscot_all();

        // Sconces.
        let (hx, hz) = (HALL_X, HALL_Z);
        for z in [-14.0, -6.0, 6.0, 14.0] {
            self.sconce(Vec3::new(-hx + 0.15, 2.6, z), Vec3::X);
            self.sconce(Vec3::new(hx - 0.15, 2.6, z), -Vec3::X);
        }
        for x in [-PURLIN_X, PURLIN_X] {
            self.sconce(Vec3::new(x, 2.6, hz - 0.15), -Vec3::Z);
        }
    }

    /// A stone column at `c` on the floor: plinth, shaft and a moulded capital, and on
    /// it a timber bolster under the girder, along the girder's line.
    fn column(&mut self, c: Vec3) {
        let stone = |lib: &mut Self, y0: f32, y1: f32, half: f32, rgb| {
            lib.decor(Inst::minmax(
                c + Vec3::new(-half, y0, -half),
                c + Vec3::new(half, y1, half),
                kind::STONE,
                rgb,
            ));
        };
        stone(self, 0.0, 0.3, 0.475, COLUMN_TRIM);
        stone(self, 0.3, 0.45, 0.42, COLUMN_TRIM);
        stone(self, 0.45, 6.35, 0.35, COLUMN);
        // An impost band where the arcade's braces spring.
        stone(self, SPRINGING - 0.22, SPRINGING, 0.42, COLUMN_TRIM);
        stone(self, 6.35, 6.45, 0.39, COLUMN_TRIM);
        stone(self, 6.45, 6.65, 0.45, COLUMN_TRIM);
        stone(self, 6.65, 6.9, 0.52, DRESSED);
        // The bolster, stepped under the girder.
        self.decor(Inst::minmax(
            c + Vec3::new(-0.3, 6.9, -0.9),
            c + Vec3::new(0.3, 7.05, 0.9),
            kind::WOOD,
            BEAM_WOOD,
        ));
        self.decor(Inst::minmax(
            c + Vec3::new(-0.33, 7.05, -1.4),
            c + Vec3::new(0.33, GIRDER_Y, 1.4),
            kind::WOOD,
            BEAM_WOOD,
        ));
        self.collide(
            c - Vec3::new(0.48, 0.0, 0.48),
            c + Vec3::new(0.48, HALL_H, 0.48),
            Stops::ALL,
        );
    }

    /// Arched timber braces between the columns on the line `x = sx`, under its girder:
    /// they frame the reading room, and the veil on the east side.
    fn arcade(&mut self, sx: f32) {
        // (from, to) between the faces of the columns.
        for (z0, z1) in [(-15.65, -8.35), (-7.65, 7.65), (8.35, 15.65)] {
            let span = z1 - z0;
            let rise = GIRDER_Y - 0.05 - SPRINGING;
            let r = (span * span * 0.25 + rise * rise) / (2.0 * rise);
            let centre = Vec3::new(sx, GIRDER_Y - 0.05 - r, (z0 + z1) * 0.5);
            let half = (span * 0.5 / r).asin();
            let n = if span > 10.0 { 14 } else { 8 };
            self.arc(
                centre,
                (Vec3::Z, Vec3::Y),
                (r - 0.14, 0.28, 0.24),
                (FRAC_PI_2 - half, FRAC_PI_2 + half, n),
                (kind::WOOD, rgba(BEAM_WOOD)),
            );
        }
        // A gilt crescent on a dark roundel hung at the crown of the middle arch, on
        // both its faces.
        for n in [Vec3::X, -Vec3::X] {
            let m = Mount {
                at: Vec3::new(sx, 0.0, 0.0) + n * 0.12,
                n,
            };
            let at = Vec3::new(0.0, GIRDER_Y - 0.62, 0.0);
            self.disc(m, at, (0.4, 0.02), kind::WOOD, rgba(BEAM_WOOD));
            self.disc(m, at + Vec3::Z * 0.025, (0.3, 0.03), kind::METAL, rgba(GILT));
            let shadow = at + Vec3::new(0.1, 0.07, 0.035);
            self.disc(m, shadow, (0.25, 0.03), kind::WOOD, rgba(BEAM_WOOD));
        }
    }

    /// Boxes round an arc about `c` in the plane of `u` and `v` (unit, at right angles),
    /// `r` from it to their middles, `w` across the arc and `d` through its plane; from
    /// angle `a0` to `a1` (0 along `u`, a quarter turn along `v`) in `n` pieces.
    fn arc(
        &mut self,
        c: Vec3,
        (u, v): (Vec3, Vec3),
        (r, w, d): (f32, f32, f32),
        (a0, a1, n): (f32, f32, usize),
        (kind, color): (u32, [f32; 4]),
    ) {
        let step = (a1 - a0) / n as f32;
        let normal = u.cross(v);
        // Long enough that neighbours meet at the outer edge.
        let len = 2.0 * (r + w * 0.5) * (step * 0.5).abs().tan();
        for k in 0..n {
            let a = a0 + (k as f32 + 0.5) * step;
            let radial = u * a.cos() + v * a.sin();
            let rot = Quat::from_mat3(&Mat3::from_cols(radial, normal, radial.cross(normal)));
            self.decor(Inst::new(c + radial * r, rot, Vec3::new(w, d, len), kind, color, 0));
        }
    }

    /// A box on a wall (see `Mount`).
    fn mounted(&mut self, m: Mount, off: Vec3, size: Vec3, spin: f32, kind: u32, color: [f32; 4]) {
        self.decor(Inst::new(m.at(off), m.rot(spin), size, kind, color, 0));
    }

    /// A disc on a wall: a fan of strips, none of them square to the wall's edges (so
    /// no two share a face), `r` round and `d` thick at `off`; fewer strips when small.
    fn disc(&mut self, m: Mount, off: Vec3, (r, d): (f32, f32), kind: u32, color: [f32; 4]) {
        let n = match r {
            r if r < 0.08 => 4,
            r if r < 0.45 => 6,
            _ => 8,
        };
        let step = PI / n as f32;
        let w = 2.0 * r * (step * 0.5).sin() * 1.08;
        for k in 0..n {
            let spin = (k as f32 + 0.5) * step;
            self.mounted(m, off, Vec3::new(2.0 * r, w, d), spin, kind, color);
        }
    }

    /// The timbers: girders on the columns, beams across the hall, and between them
    /// the reading room's coffers (painted as the night sky, gilt-ribbed, with its
    /// stars) and the wings' plain bays; a plate round the tops of the walls, and
    /// corbels and pilasters under the timbers' ends.
    fn ceiling(&mut self) {
        let (hx, hz, hh) = (HALL_X, HALL_Z, HALL_H);
        let wood = |lib: &mut Self, min: Vec3, max: Vec3| lib.decor(Inst::minmax(min, max, kind::WOOD, BEAM_WOOD));
        let gilt = |lib: &mut Self, min: Vec3, max: Vec3| lib.decor(Inst::minmax(min, max, kind::METAL, GILT));
        let inner = VEIL_X - 0.4;
        for sx in [-VEIL_X, VEIL_X] {
            wood(self, Vec3::new(sx - 0.4, GIRDER_Y, -hz), Vec3::new(sx + 0.4, hh, hz));
        }
        for z in BEAMS_Z {
            let main = COLUMNS_Z.contains(&z);
            let (y, hw) = if main { (BEAM_MAIN_Y, 0.24) } else { (BEAM_Y, 0.19) };
            wood(self, Vec3::new(-hx, y, z - hw), Vec3::new(hx, hh, z + hw));
            gilt(
                self,
                Vec3::new(-inner, y - 0.012, z - 0.05),
                Vec3::new(inner, y, z + 0.05),
            );
            // Gilt bosses where the ribs cross.
            for x in RIBS_X {
                let boss = Inst::new(
                    Vec3::new(x, y - 0.035, z),
                    Quat::from_rotation_y(FRAC_PI_4),
                    Vec3::new(0.3, 0.07, 0.3),
                    kind::METAL,
                    rgba(GILT),
                    0,
                );
                self.decor(boss);
            }
            // Under its ends, a pilaster (main beams) or a corbel on the end walls.
            for s in [-1.0f32, 1.0] {
                let m = Mount {
                    at: Vec3::new(s * hx, 0.0, z),
                    n: Vec3::new(-s, 0.0, 0.0),
                };
                if main {
                    self.pilaster(m, y);
                } else {
                    self.corbel(m, y);
                }
            }
        }
        for x in RIBS_X {
            wood(self, Vec3::new(x - 0.13, RIB_Y, -hz), Vec3::new(x + 0.13, hh, hz));
            gilt(
                self,
                Vec3::new(x - 0.04, RIB_Y - 0.012, -hz),
                Vec3::new(x + 0.04, RIB_Y, hz),
            );
        }
        for x in [-PURLIN_X, PURLIN_X] {
            wood(self, Vec3::new(x - 0.16, RIB_Y, -hz), Vec3::new(x + 0.16, hh, hz));
        }
        // Pilasters under the girders' ends, and corbels under the purlins'.
        for sx in [-VEIL_X, VEIL_X] {
            for sz in [-1.0f32, 1.0] {
                let m = Mount {
                    at: Vec3::new(sx, 0.0, sz * hz),
                    n: Vec3::new(0.0, 0.0, -sz),
                };
                self.pilaster(m, GIRDER_Y);
            }
        }
        for x in [-PURLIN_X, PURLIN_X] {
            for sz in [-1.0f32, 1.0] {
                let m = Mount {
                    at: Vec3::new(x, 0.0, sz * hz),
                    n: Vec3::new(0.0, 0.0, -sz),
                };
                self.corbel(m, RIB_Y);
            }
        }
        // The wall plate, a moulding under it; the end walls' butt against the long ones.
        for (min, max) in [
            (Vec3::new(-hx, PLATE_Y, -hz), Vec3::new(hx, hh, -hz + 0.22)),
            (Vec3::new(-hx, PLATE_Y, hz - 0.22), Vec3::new(hx, hh, hz)),
            (
                Vec3::new(-hx, PLATE_Y, -hz + 0.22),
                Vec3::new(-hx + 0.22, hh, hz - 0.22),
            ),
            (Vec3::new(hx - 0.22, PLATE_Y, -hz + 0.22), Vec3::new(hx, hh, hz - 0.22)),
        ] {
            wood(self, min, max);
        }
        for (min, max) in [
            (Vec3::new(-hx, PLATE_Y - 0.08, -hz), Vec3::new(hx, PLATE_Y, -hz + 0.12)),
            (Vec3::new(-hx, PLATE_Y - 0.08, hz - 0.12), Vec3::new(hx, PLATE_Y, hz)),
            (
                Vec3::new(-hx, PLATE_Y - 0.08, -hz + 0.12),
                Vec3::new(-hx + 0.12, PLATE_Y, hz - 0.12),
            ),
            (
                Vec3::new(hx - 0.12, PLATE_Y - 0.08, -hz + 0.12),
                Vec3::new(hx, PLATE_Y, hz - 0.12),
            ),
        ] {
            self.decor(Inst::minmax(min, max, kind::WOOD, WALNUT));
        }
        self.painted_sky();
    }

    /// Stars on the reading room's coffers: painted in gilt, and in every few coffers
    /// a constellation whose stars shine, joined by gilt lines.
    fn painted_sky(&mut self) {
        let mut xs = vec![-VEIL_X + 0.4];
        xs.extend(RIBS_X);
        xs.push(VEIL_X - 0.4);
        let mut zs = vec![-HALL_Z + 0.22];
        zs.extend(BEAMS_Z);
        zs.push(HALL_Z - 0.22);
        let y = HALL_H - 0.004;
        let gilt = rgba(GILT);
        // A four-pointed star: two crossed bars.
        let star = |lib: &mut Self, p: Vec3, s: f32, turn: f32, kind: u32, color: [f32; 4]| {
            for q in [0.0, FRAC_PI_2] {
                let rot = Quat::from_rotation_y(turn + q);
                lib.decor(Inst::new(p, rot, Vec3::new(s, 0.006, s * 0.22), kind, color, 0));
            }
        };
        let mut i = 0u64;
        for bx in xs.windows(2) {
            for bz in zs.windows(2) {
                i += 1;
                let mut r = StdRng::seed_from_u64(0x57a2_5eed ^ i);
                let (x0, x1) = (bx[0] + 0.35, bx[1] - 0.35);
                let (z0, z1) = (bz[0] + 0.4, bz[1] - 0.4);
                let n = ((x1 - x0) * (z1 - z0) * 0.5) as usize + r.gen_range(0..3);
                for _ in 0..n {
                    let p = Vec3::new(r.gen_range(x0..x1), y, r.gen_range(z0..z1));
                    let turn = FRAC_PI_4 + r.gen_range(-0.2..0.2);
                    star(self, p, r.gen_range(0.08..0.17), turn, kind::METAL, gilt);
                }
                if z1 - z0 < 2.0 || r.gen_range(0.0..1.0f32) > 0.3 {
                    continue;
                }
                let mut pts = vec![Vec2::new(r.gen_range(x0..x1), r.gen_range(z0..z1))];
                for _ in 1..r.gen_range(4..7) {
                    let (p, a) = (pts[pts.len() - 1], r.gen_range(0.0..TAU));
                    let q = (p + Vec2::new(a.cos(), a.sin()) * r.gen_range(0.5..0.9))
                        .clamp(Vec2::new(x0, z0), Vec2::new(x1, z1));
                    if q.distance(p) > 0.3 {
                        pts.push(q);
                    }
                }
                for &p in &pts {
                    let glow = r.gen_range(2.5..5.0);
                    let s = r.gen_range(0.12..0.2);
                    star(
                        self,
                        Vec3::new(p.x, y, p.y),
                        s,
                        FRAC_PI_4,
                        kind::CRYSTAL,
                        [0.75, 0.85, 1.0, glow],
                    );
                }
                for w in pts.windows(2) {
                    let d = w[1] - w[0];
                    let mid = (w[0] + w[1]) * 0.5;
                    let rot = Quat::from_rotation_y((-d.y).atan2(d.x));
                    let line = Vec3::new(d.length() - 0.12, 0.004, 0.018);
                    self.decor(Inst::new(
                        Vec3::new(mid.x, y + 0.001, mid.y),
                        rot,
                        line,
                        kind::METAL,
                        gilt,
                        0,
                    ));
                }
            }
        }
    }

    /// A stone pilaster on a wall from the floor up to `top`, where a timber rests on
    /// its capital.
    fn pilaster(&mut self, m: Mount, top: f32) {
        let st = |lib: &mut Self, (y0, y1): (f32, f32), half: f32, out: f32, rgb| {
            lib.mounted(
                m,
                Vec3::new(0.0, (y0 + y1) * 0.5, out * 0.5),
                Vec3::new(2.0 * half, y1 - y0, out),
                0.0,
                kind::STONE,
                rgba(rgb),
            );
        };
        st(self, (0.0, 0.32), 0.52, 0.26, COLUMN_TRIM);
        st(self, (0.32, top - 0.45), 0.42, 0.18, COLUMN);
        st(self, (top - 0.45, top - 0.33), 0.47, 0.22, COLUMN_TRIM);
        st(self, (top - 0.33, top), 0.56, 0.3, DRESSED);
    }

    /// A stepped stone corbel on a wall under a timber whose underside is at `top`.
    fn corbel(&mut self, m: Mount, top: f32) {
        for (y0, y1, half, out) in [(top - 0.7, top - 0.35, 0.22, 0.2), (top - 0.35, top, 0.3, 0.4)] {
            self.mounted(
                m,
                Vec3::new(0.0, (y0 + y1) * 0.5, out * 0.5),
                Vec3::new(2.0 * half, y1 - y0, out),
                0.0,
                kind::STONE,
                rgba(DRESSED),
            );
        }
    }

    /// A north window at `x`: glass at the back of a deep reveal with a round head,
    /// an arch of dressed stone round it with a keystone, a sill, and velvet curtains
    /// drawn back to either side.
    fn window(&mut self, x: f32) {
        let face = -HALL_Z;
        self.decor(Inst::new(
            Vec3::new(x, (WIN_SILL + WIN_TOP) * 0.5, face - 0.42),
            Quat::IDENTITY,
            Vec3::new(2.0 * WIN_HALF + 0.04, WIN_TOP - WIN_SILL + 0.04, 0.04),
            kind::GLASS,
            [0.32, 0.42, 0.85, 1.2],
            0,
        ));
        // Violet glass in the round head, over a stone transom; a mullion parts the
        // lights below.
        self.decor(Inst::new(
            Vec3::new(x, (WIN_SPRING + WIN_TOP) * 0.5, face - 0.392),
            Quat::IDENTITY,
            Vec3::new(2.0 * WIN_HALF + 0.02, WIN_TOP - WIN_SPRING + 0.02, 0.01),
            kind::GLASS,
            [0.4, 0.3, 0.82, 1.1],
            0,
        ));
        self.decor(Inst::minmax(
            Vec3::new(x - WIN_HALF - 0.05, WIN_SPRING - 0.05, face - 0.4),
            Vec3::new(x + WIN_HALF + 0.05, WIN_SPRING + 0.05, face - 0.3),
            kind::STONE,
            DRESSED,
        ));
        self.decor(Inst::minmax(
            Vec3::new(x - 0.05, WIN_SILL, face - 0.4),
            Vec3::new(x + 0.05, WIN_SPRING - 0.05, face - 0.32),
            kind::STONE,
            DRESSED,
        ));
        // The head's corners, filled at 45° in the reveal behind the arch.
        for s in [-1.0f32, 1.0] {
            let d = Vec2::new(s, 1.0).normalize() * (WIN_HALF + 0.2);
            self.decor(Inst::new(
                Vec3::new(x + d.x, WIN_SPRING + d.y, face - 0.25),
                Quat::from_rotation_z(-s * FRAC_PI_4),
                Vec3::new(0.7, 0.4, 0.46),
                kind::STONE,
                rgba(STONE),
                0,
            ));
        }
        let m = Mount {
            at: Vec3::new(x, 0.0, face),
            n: Vec3::Z,
        };
        self.arc(
            m.at(Vec3::new(0.0, WIN_SPRING, 0.07)),
            (Vec3::X, Vec3::Y),
            (WIN_HALF + 0.15, 0.3, 0.14),
            (0.0, PI, 7),
            (kind::STONE, rgba(DRESSED)),
        );
        let dressed = rgba(DRESSED);
        let keystone = Vec3::new(0.0, WIN_SPRING + WIN_HALF + 0.2, 0.1);
        self.mounted(m, keystone, Vec3::new(0.32, 0.5, 0.2), 0.0, kind::STONE, dressed);
        for s in [-1.0f32, 1.0] {
            let jamb = Vec3::new(s * (WIN_HALF + 0.15), (WIN_SILL + 0.03 + WIN_SPRING - 0.12) * 0.5, 0.06);
            let size = Vec3::new(0.3, WIN_SPRING - 0.12 - WIN_SILL - 0.03, 0.12);
            self.mounted(m, jamb, size, 0.0, kind::STONE, dressed);
            let impost = Vec3::new(s * (WIN_HALF + 0.17), WIN_SPRING - 0.06, 0.09);
            self.mounted(m, impost, Vec3::new(0.46, 0.12, 0.18), 0.0, kind::STONE, dressed);
        }
        self.decor(Inst::minmax(
            Vec3::new(x - 1.15, WIN_SILL - 0.12, face - 0.42),
            Vec3::new(x + 1.15, WIN_SILL + 0.03, face + 0.2),
            kind::STONE,
            DRESSED,
        ));
        // A valance over the arch, and curtains hanging from it in pleats, drawn
        // back to tiebacks either side.
        let velvet = rgba(VELVET);
        let gilt = rgba(GILT);
        self.mounted(
            m,
            Vec3::new(0.0, 7.15, 0.33),
            Vec3::new(3.7, 0.4, 0.06),
            0.0,
            kind::CLOTH,
            velvet,
        );
        self.mounted(
            m,
            Vec3::new(0.0, 6.93, 0.355),
            Vec3::new(3.66, 0.05, 0.02),
            0.0,
            kind::METAL,
            gilt,
        );
        let (top, tie, foot) = (6.95, 3.4, 2.05);
        for s in [-1.0f32, 1.0] {
            for k in 0..4 {
                let kf = k as f32;
                let at = |along: f32, y: f32| Vec2::new(s * along, y);
                let (a, b, c) = (
                    at(1.0 + 0.2 * kf, top),
                    at(1.3 + 0.045 * kf, tie),
                    at(1.12 + 0.17 * kf, foot),
                );
                let out = 0.22 + 0.03 * (k % 2) as f32;
                let fold = if k % 2 == 0 { 0.35 } else { -0.35 };
                for (p, q, w) in [(a, b, 0.2), (b, c, 0.17)] {
                    let d = q - p;
                    let mid = (p + q) * 0.5;
                    let pleat = Inst::new(
                        m.at(Vec3::new(mid.x, mid.y, out)),
                        m.rot(d.y.atan2(d.x)) * Quat::from_rotation_x(fold),
                        Vec3::new(d.length() + 0.04, w, 0.03),
                        kind::CLOTH,
                        velvet,
                        0,
                    );
                    self.decor(pleat);
                }
            }
            let band = Vec3::new(s * 1.37, tie, 0.27);
            self.mounted(m, band, Vec3::new(0.36, 0.08, 0.1), 0.0, kind::METAL, gilt);
            let tassel = Vec3::new(s * 1.52, tie - 0.16, 0.3);
            self.mounted(m, tassel, Vec3::new(0.05, 0.22, 0.05), 0.0, kind::METAL, gilt);
        }
        self.light(
            Vec3::new(x, 3.2, face + 1.5),
            8.0,
            [0.45, 0.55, 1.0],
            0.9,
            LightKind::Static,
        );
    }

    /// What hangs on the walls other than windows: tapestries either side of the
    /// hearth, moonlit paintings in the wings, a great map at the west end and a
    /// clock at the east end.
    fn wall_dressing(&mut self) {
        let (hx, hz) = (HALL_X, HALL_Z);
        for s in [-1.0f32, 1.0] {
            let m = Mount {
                at: Vec3::new(s * 7.0, 0.0, hz),
                n: -Vec3::Z,
            };
            self.tapestry(m, s);
            for (k, x) in [15.5f32, 20.0].into_iter().enumerate() {
                let m = Mount {
                    at: Vec3::new(s * x, 0.0, hz),
                    n: -Vec3::Z,
                };
                self.painting(m, Vec3::new(1.7, 1.25, 3.5), (s > 0.0) as u64 * 2 + k as u64);
            }
        }
        self.wall_map(Mount {
            at: Vec3::new(-hx, 0.0, 0.0),
            n: Vec3::X,
        });
        self.clock(Mount {
            at: Vec3::new(hx, 0.0, 0.0),
            n: -Vec3::X,
        });
    }

    /// A tapestry on a rod: a border, a field strewn with small gilt lozenges, and a
    /// roundel with a crescent moon over an open book.
    fn tapestry(&mut self, m: Mount, s: f32) {
        let (w, h, mid) = (3.0f32, 4.0f32, 4.45f32);
        let field = if s < 0.0 { [0.06, 0.16, 0.12] } else { [0.08, 0.1, 0.22] };
        self.mounted(
            m,
            Vec3::new(0.0, mid, 0.08),
            Vec3::new(w, h, 0.02),
            0.0,
            kind::CLOTH,
            rgba([0.3, 0.07, 0.05]),
        );
        let inner = Vec3::new(w - 0.36, h - 0.36, 0.02);
        self.mounted(m, Vec3::new(0.0, mid, 0.09), inner, 0.0, kind::CLOTH, rgba(field));
        let mut r = StdRng::seed_from_u64(0x7a9e ^ (s > 0.0) as u64);
        for i in 0..6 {
            for j in 0..7 {
                let p = Vec2::new(-1.1 + i as f32 * 0.44, mid - 1.6 + j as f32 * 0.53);
                if (p - Vec2::new(0.0, mid)).length() < 1.05 {
                    continue;
                }
                let off = Vec3::new(p.x + (j % 2) as f32 * 0.22 - 0.11, p.y, 0.1);
                let c = if r.gen_range(0.0..1.0f32) < 0.75 { GILT } else { CREAM };
                self.mounted(m, off, Vec3::new(0.09, 0.09, 0.01), FRAC_PI_4, kind::CLOTH, rgba(c));
            }
        }
        let centre = Vec3::new(0.0, mid, 0.1);
        self.disc(m, centre, (0.85, 0.012), kind::CLOTH, rgba(GILT));
        self.disc(
            m,
            centre + Vec3::Z * 0.006,
            (0.75, 0.012),
            kind::CLOTH,
            rgba([0.04, 0.05, 0.12]),
        );
        let moon = centre + Vec3::new(0.0, 0.25, 0.012);
        self.disc(m, moon, (0.24, 0.012), kind::CLOTH, rgba(CREAM));
        self.disc(
            m,
            moon + Vec3::new(0.1, 0.06, 0.006),
            (0.21, 0.012),
            kind::CLOTH,
            rgba([0.04, 0.05, 0.12]),
        );
        for side in [-1.0f32, 1.0] {
            let page = centre + Vec3::new(side * 0.22, -0.3, 0.012);
            self.mounted(
                m,
                page,
                Vec3::new(0.4, 0.26, 0.012),
                side * 0.2,
                kind::CLOTH,
                rgba(CREAM),
            );
        }
        // Rod and tassels.
        let rod = Vec3::new(0.0, mid + h * 0.5 + 0.06, 0.09);
        self.mounted(m, rod, Vec3::new(w + 0.5, 0.06, 0.06), 0.0, kind::WOOD, rgba(WALNUT));
        for side in [-1.0f32, 1.0] {
            let end = rod + Vec3::X * side * (w * 0.5 + 0.28);
            self.mounted(m, end, Vec3::new(0.12, 0.12, 0.12), FRAC_PI_4, kind::METAL, rgba(GILT));
            let tassel = Vec3::new(side * (w * 0.5 - 0.1), mid - h * 0.5 - 0.12, 0.09);
            self.mounted(m, tassel, Vec3::new(0.06, 0.24, 0.06), 0.0, kind::CLOTH, rgba(GILT));
        }
    }

    /// A moonlit landscape in a gilt frame, `size` (width, height, centre's height):
    /// hills against the night, a lake with the moon in it, trees on the near shore.
    fn painting(&mut self, m: Mount, size: Vec3, seed: u64) {
        let (w, h, y) = (size.x, size.y, size.z);
        let mut r = StdRng::seed_from_u64(0x9a17 ^ seed);
        let gilt = rgba(GILT);
        for s in [-1.0f32, 1.0] {
            let rail = Vec3::new(0.0, y + s * (h * 0.5 + 0.06), 0.04);
            self.mounted(m, rail, Vec3::new(w + 0.24, 0.12, 0.08), 0.0, kind::METAL, gilt);
            let stile = Vec3::new(s * (w * 0.5 + 0.06), y, 0.04);
            self.mounted(m, stile, Vec3::new(0.12, h, 0.08), 0.0, kind::METAL, gilt);
        }
        // Each layer a little in front of the one it overlaps.
        let layer = |lib: &mut Self, (y0, y1): (f32, f32), z: f32, rgb: [f32; 3]| {
            let off = Vec3::new(0.0, y + h * (y0 + y1) * 0.5, z);
            lib.mounted(m, off, Vec3::new(w, h * (y1 - y0), 0.004), 0.0, kind::PLAIN, rgba(rgb));
        };
        layer(self, (-0.5, 0.5), 0.01, [0.06, 0.08, 0.2]);
        layer(self, (-0.12, 0.08), 0.016, [0.1, 0.12, 0.25]);
        // Hills: lozenges behind the lake, so only their peaks show.
        for k in 0..3 {
            let sz = r.gen_range(0.35..0.6f32);
            let x = r.gen_range(-w * 0.5 + sz * 0.75..w * 0.5 - sz * 0.75);
            let c = [0.035 + 0.015 * k as f32, 0.055 + 0.015 * k as f32, 0.08];
            let off = Vec3::new(x, y - h * 0.12, 0.022 + 0.002 * k as f32);
            self.mounted(m, off, Vec3::new(sz, sz, 0.002), FRAC_PI_4, kind::PLAIN, rgba(c));
        }
        layer(self, (-0.3, -0.12), 0.03, [0.04, 0.07, 0.17]);
        layer(self, (-0.5, -0.3), 0.036, [0.025, 0.04, 0.035]);
        let mx = r.gen_range(-w * 0.35..w * 0.35);
        let moon = Vec3::new(mx, y + h * r.gen_range(0.18..0.3), 0.02);
        self.disc(m, moon, (0.1, 0.004), kind::PLAIN, rgba([0.95, 0.9, 0.7]));
        // Its path on the water, and trees on the shore.
        let glint = rgba([0.55, 0.55, 0.45]);
        for k in 0..3 {
            let gy = y - h * (0.15 + 0.045 * k as f32);
            let at = Vec3::new(mx + r.gen_range(-0.02..0.02), gy, 0.034);
            self.mounted(
                m,
                at,
                Vec3::new(0.12 - 0.025 * k as f32, 0.014, 0.002),
                0.01,
                kind::PLAIN,
                glint,
            );
        }
        for k in 0..r.gen_range(3..6) {
            let th = r.gen_range(0.1..0.22);
            let x = r.gen_range(-w * 0.45..w * 0.45);
            let at = Vec3::new(x, y - h * 0.3 + th * 0.4, 0.04 + 0.001 * k as f32);
            let tree = rgba([0.02, 0.035, 0.03]);
            let lean = r.gen_range(-0.04..0.04);
            self.mounted(m, at, Vec3::new(th * 0.28, th, 0.002), lean, kind::PLAIN, tree);
            // A cypress's point.
            let tip = at + Vec3::new(0.0, th * 0.5, 0.0);
            let side = th * 0.2;
            self.mounted(
                m,
                tip,
                Vec3::new(side, side, 0.002),
                FRAC_PI_4 + lean,
                kind::PLAIN,
                tree,
            );
        }
    }

    /// A great wall map on rollers: a sea crossed by rhumb lines from a compass rose,
    /// and three lands with darker coasts.
    fn wall_map(&mut self, m: Mount) {
        let (w, h, y) = (4.0f32, 3.0f32, 3.7f32);
        let sheet = Vec3::new(w, h, 0.012);
        self.mounted(
            m,
            Vec3::new(0.0, y, 0.03),
            sheet,
            0.0,
            kind::PARCHMENT,
            rgba([0.62, 0.5, 0.32]),
        );
        let (sea_c, sea_half) = (Vec2::new(0.0, y - 0.05), Vec2::new(w * 0.5 - 0.2, h * 0.5 - 0.25));
        let sea = Vec3::new(sea_c.x, sea_c.y, 0.038);
        let sea_size = Vec3::new(sea_half.x * 2.0, sea_half.y * 2.0, 0.006);
        self.mounted(m, sea, sea_size, 0.0, kind::PARCHMENT, rgba([0.3, 0.38, 0.36]));
        // Rhumb lines out from the rose to the edges of the sea.
        let rose = Vec2::new(1.2, y - 0.85);
        let ink = rgba([0.17, 0.2, 0.18]);
        for k in 0..16 {
            let a = k as f32 * TAU / 16.0 + 0.02;
            let d = Vec2::new(a.cos(), a.sin());
            let reach = |p: f32, dir: f32, c: f32, half: f32| {
                if dir.abs() < 1e-4 {
                    f32::MAX
                } else {
                    ((c + half * dir.signum()) - p) / dir
                }
            };
            let len = reach(rose.x, d.x, sea_c.x, sea_half.x).min(reach(rose.y, d.y, sea_c.y, sea_half.y));
            let mid = rose + d * len * 0.5;
            let line = Vec3::new(len, 0.008, 0.001);
            self.mounted(m, Vec3::new(mid.x, mid.y, 0.0415), line, a, kind::PARCHMENT, ink);
        }
        // Each land a cluster of patches, over a slightly larger, darker cluster for its coast.
        let mut r = StdRng::seed_from_u64(0x3a9);
        for (k, (cx, cy)) in [(-1.0f32, 0.45f32), (0.6, 0.3), (-0.3, -0.7)].into_iter().enumerate() {
            let patches: Vec<(Vec2, Vec2, f32)> = (0..7)
                .map(|_| {
                    let at = Vec2::new(cx + r.gen_range(-0.45..0.45), y + cy + r.gen_range(-0.3..0.3));
                    let size = Vec2::new(r.gen_range(0.2..0.55), r.gen_range(0.15..0.4));
                    (at, size, r.gen_range(-0.8..0.8))
                })
                .collect();
            for (j, &(at, size, spin)) in patches.iter().enumerate() {
                let z = 0.0425 + 0.0005 * j as f32;
                let coast = Vec3::new(size.x + 0.05, size.y + 0.05, 0.0004);
                self.mounted(
                    m,
                    Vec3::new(at.x, at.y, z),
                    coast,
                    spin,
                    kind::PARCHMENT,
                    rgba([0.36, 0.27, 0.15]),
                );
            }
            for (j, &(at, size, spin)) in patches.iter().enumerate() {
                let z = 0.047 + 0.0005 * j as f32;
                let tint = [0.62 - 0.04 * k as f32, 0.52 - 0.01 * j as f32, 0.31];
                let patch = Vec3::new(size.x, size.y, 0.0004);
                self.mounted(m, Vec3::new(at.x, at.y, z), patch, spin, kind::PARCHMENT, rgba(tint));
            }
        }
        // The compass rose: long points north-south and east-west, short ones between.
        let rose = Vec3::new(rose.x, rose.y, 0.052);
        let gilt = rgba(GILT);
        for k in 0..4 {
            let (len, wid) = if k % 2 == 0 { (0.62, 0.07) } else { (0.4, 0.05) };
            let spin = k as f32 * FRAC_PI_4 + 0.01;
            let at = rose + Vec3::Z * 0.0015 * k as f32;
            self.mounted(m, at, Vec3::new(len, wid, 0.001), spin, kind::METAL, gilt);
        }
        self.disc(m, rose + Vec3::Z * 0.008, (0.05, 0.002), kind::METAL, gilt);
        // A cartouche for its title, top left.
        let cart = Vec3::new(-1.25, y + 1.0, 0.057);
        self.mounted(
            m,
            cart,
            Vec3::new(0.9, 0.34, 0.004),
            0.0,
            kind::PARCHMENT,
            rgba([0.4, 0.3, 0.16]),
        );
        let label = cart + Vec3::Z * 0.004;
        self.mounted(
            m,
            label,
            Vec3::new(0.8, 0.26, 0.004),
            0.0,
            kind::PARCHMENT,
            rgba([0.7, 0.6, 0.4]),
        );
        for k in 0..2 {
            let text = label + Vec3::new(-0.05 * k as f32, 0.05 - 0.1 * k as f32, 0.003);
            self.mounted(
                m,
                text,
                Vec3::new(0.55 - 0.15 * k as f32, 0.025, 0.002),
                0.0,
                kind::PARCHMENT,
                ink,
            );
        }
        // The rollers, with gilt ends, and a cord up to a hook.
        let roller = rgba(DARK_WOOD);
        for ry in [y + h * 0.5 + 0.03, y - h * 0.5 - 0.03] {
            self.mounted(
                m,
                Vec3::new(0.0, ry, 0.06),
                Vec3::new(w + 0.3, 0.1, 0.1),
                0.0,
                kind::WOOD,
                roller,
            );
            for s in [-1.0f32, 1.0] {
                let end = Vec3::new(s * (w * 0.5 + 0.2), ry, 0.06);
                self.mounted(m, end, Vec3::new(0.1, 0.14, 0.14), FRAC_PI_4, kind::METAL, gilt);
            }
        }
        let hook = Vec3::new(0.0, y + h * 0.5 + 0.75, 0.05);
        for s in [-1.0f32, 1.0] {
            let foot = Vec3::new(s * (w * 0.5 - 0.1), y + h * 0.5 + 0.06, 0.05);
            let d = hook - foot;
            let spin = d.y.atan2(d.x);
            let cord = Vec3::new(d.length(), 0.02, 0.02);
            self.mounted(m, (hook + foot) * 0.5, cord, spin, kind::CLOTH, rgba([0.35, 0.25, 0.1]));
        }
        self.mounted(m, hook, Vec3::new(0.08, 0.08, 0.1), FRAC_PI_4, kind::METAL, gilt);
    }

    /// A great clock face on a walnut board, at ten to midnight.
    fn clock(&mut self, m: Mount) {
        let y = 4.2;
        let centre = Vec3::new(0.0, y, 0.0);
        let board = Vec3::new(3.0, 3.0, 0.08);
        self.mounted(m, centre + Vec3::Z * 0.04, board, 0.0, kind::WOOD, rgba(WALNUT));
        let gilt = rgba(GILT);
        for (sx, sy) in [(-1.0f32, -1.0f32), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)] {
            let corner = centre + Vec3::new(sx * 1.2, sy * 1.2, 0.085);
            self.mounted(m, corner, Vec3::new(0.32, 0.32, 0.01), FRAC_PI_4, kind::METAL, gilt);
        }
        self.disc(m, centre + Vec3::Z * 0.095, (1.2, 0.03), kind::PLAIN, rgba(CREAM));
        let face = m.at(centre + Vec3::Z * 0.12);
        self.arc(
            face,
            (m.right(), Vec3::Y),
            (1.27, 0.12, 0.08),
            (0.0, TAU, 24),
            (kind::METAL, gilt),
        );
        let ink = rgba(IRON);
        for k in 0..12 {
            let a = FRAC_PI_2 - k as f32 * TAU / 12.0;
            let (len, wid) = if k % 3 == 0 { (0.24, 0.07) } else { (0.15, 0.04) };
            let at = centre + Vec3::new(a.cos(), a.sin(), 0.0) * 1.0 + Vec3::Z * 0.12;
            self.mounted(m, at, Vec3::new(len, wid, 0.01), a, kind::METAL, ink);
        }
        // Hands: the hour just short of twelve, the minute at ten to.
        for (turn, len, wid, z) in [
            (11.83f32 / 12.0, 0.7f32, 0.08f32, 0.13f32),
            (50.0 / 60.0, 1.0, 0.05, 0.14),
        ] {
            let a = FRAC_PI_2 - turn * TAU;
            let at = centre + Vec3::new(a.cos(), a.sin(), 0.0) * len * 0.4 + Vec3::Z * z;
            self.mounted(m, at, Vec3::new(len, wid, 0.01), a, kind::METAL, ink);
        }
        self.disc(m, centre + Vec3::Z * 0.15, (0.07, 0.02), kind::METAL, gilt);
    }

    /// Panelling round the walls below a rail: a skirting, the panels with stiles
    /// between them, and the rail.
    fn wainscot_all(&mut self) {
        let (hx, hz) = (HALL_X, HALL_Z);
        let wh = 1.15;
        let wc = [0.13, 0.065, 0.03];
        for (min, max) in [
            (Vec3::new(-hx, 0.0, -hz), Vec3::new(hx, wh, -hz + 0.06)),
            (Vec3::new(-hx, 0.0, hz - 0.06), Vec3::new(hx, wh, hz)),
            (Vec3::new(-hx, 0.0, -hz), Vec3::new(-hx + 0.06, wh, hz)),
            (Vec3::new(hx - 0.06, 0.0, -hz), Vec3::new(hx, wh, hz)),
        ] {
            self.decor(Inst::minmax(min, max, kind::WOOD, wc));
        }
        for (a, b) in [
            (Vec3::new(-hx, wh, -hz), Vec3::new(hx, wh + 0.06, -hz + 0.1)),
            (Vec3::new(-hx, wh, hz - 0.1), Vec3::new(hx, wh + 0.06, hz)),
            (Vec3::new(-hx, wh, -hz), Vec3::new(-hx + 0.1, wh + 0.06, hz)),
            (Vec3::new(hx - 0.1, wh, -hz), Vec3::new(hx, wh + 0.06, hz)),
        ] {
            self.decor(Inst::minmax(a, b, kind::WOOD, WALNUT));
        }
        // Skirtings: the end walls' butt against the long walls'.
        let sk = 0.14;
        for (min, max) in [
            (Vec3::new(-hx, 0.0, -hz), Vec3::new(hx, sk, -hz + 0.09)),
            (Vec3::new(-hx, 0.0, hz - 0.09), Vec3::new(hx, sk, hz)),
            (Vec3::new(-hx, 0.0, -hz + 0.09), Vec3::new(-hx + 0.09, sk, hz - 0.09)),
            (Vec3::new(hx - 0.09, 0.0, -hz + 0.09), Vec3::new(hx, sk, hz - 0.09)),
        ] {
            self.decor(Inst::minmax(min, max, kind::WOOD, DARK_WOOD));
        }
        // Stiles every 1.2 m or so along each wall.
        let stile = |lib: &mut Self, m: Mount| {
            let off = Vec3::new(0.0, (sk + wh) * 0.5, 0.07);
            lib.mounted(m, off, Vec3::new(0.08, wh - sk, 0.02), 0.0, kind::WOOD, rgba(WALNUT));
        };
        for k in 0..40 {
            let x = -hx + 0.6 + k as f32 * (2.0 * hx - 1.2) / 39.0;
            for (z, n) in [(-hz, Vec3::Z), (hz, -Vec3::Z)] {
                stile(
                    self,
                    Mount {
                        at: Vec3::new(x, 0.0, z),
                        n,
                    },
                );
            }
        }
        for k in 0..30 {
            let z = -hz + 0.6 + k as f32 * (2.0 * hz - 1.2) / 29.0;
            for (x, n) in [(-hx, Vec3::X), (hx, -Vec3::X)] {
                stile(
                    self,
                    Mount {
                        at: Vec3::new(x, 0.0, z),
                        n,
                    },
                );
            }
        }
    }

    /// The floor, the walls (the north one round its windows) and the ceiling: the
    /// reading room's painted, the wings' plastered between their timbers.
    fn build_shell(&mut self) {
        let (hx, hz, hh, t) = (HALL_X, HALL_Z, HALL_H, WALL_T);
        // Walls (drawn only: the level's bounds hold bodies and books at them).
        let wall = |lib: &mut Self, min: Vec3, max: Vec3| lib.decor(Inst::minmax(min, max, kind::STONE, STONE));
        let north = |lib: &mut Self, x0: f32, x1: f32, y0: f32, y1: f32| {
            wall(lib, Vec3::new(x0, y0, -hz - t), Vec3::new(x1, y1, -hz));
        };
        north(self, -hx - t, hx + t, 0.0, WIN_SILL);
        north(self, -hx - t, hx + t, WIN_TOP, hh);
        let mut x = -hx - t;
        for wx in WINDOWS_X {
            north(self, x, wx - WIN_HALF, WIN_SILL, WIN_TOP);
            x = wx + WIN_HALF;
        }
        north(self, x, hx + t, WIN_SILL, WIN_TOP);
        wall(self, Vec3::new(-hx - t, 0.0, hz), Vec3::new(hx + t, hh, hz + t));
        wall(self, Vec3::new(-hx - t, 0.0, -hz), Vec3::new(-hx, hh, hz));
        wall(self, Vec3::new(hx, 0.0, -hz), Vec3::new(hx + t, hh, hz));
        let (y0, y1) = (hh, hh + 0.3);
        self.decor(Inst::minmax(
            Vec3::new(-VEIL_X, y0, -hz - t),
            Vec3::new(VEIL_X, y1, hz + t),
            kind::PLAIN,
            SKY,
        ));
        for s in [-1.0f32, 1.0] {
            let (a, b) = (s * VEIL_X, s * (hx + t));
            self.decor(Inst::minmax(
                Vec3::new(a.min(b), y0, -hz - t),
                Vec3::new(a.max(b), y1, hz + t),
                kind::PLAIN,
                PLASTER,
            ));
        }
        self.decor(Inst::minmax(
            Vec3::new(-hx, -0.2, -hz),
            Vec3::new(hx, 0.0, hz),
            kind::FLOOR,
            [0.30, 0.16, 0.075],
        ));
    }

    fn build_stacks(&mut self, rng: &mut StdRng) {
        let mut titles = HashSet::new();
        let rows_z = [-12.0f32, -4.0, 4.0, 12.0];
        for wing in 0..2 {
            let xs: [f32; 3] = if wing == 0 {
                [-19.0, -19.0 + UNIT_PITCH, -19.0 + 2.0 * UNIT_PITCH]
            } else {
                [19.0 - 2.0 * UNIT_PITCH, 19.0 - UNIT_PITCH, 19.0]
            };
            // Unit geometry.
            for &rz in &rows_z {
                for &cx in &xs {
                    self.bookcase(Vec3::new(cx, 0.0, rz), true);
                }
            }
            let mut cat = self.catalog.departments[wing].categories.start;
            for &rz in &rows_z {
                for nz in [-1.0f32, 1.0] {
                    let normal = Vec3::new(0.0, 0.0, nz);
                    let right = Vec3::Y.cross(normal);
                    let mut order: Vec<f32> = xs.to_vec();
                    order.sort_by(|a, b| (a * right.x).partial_cmp(&(b * right.x)).unwrap());
                    let faces: Vec<(Vec3, Vec3)> = order
                        .iter()
                        .map(|&cx| (Vec3::new(cx, 0.0, rz + nz * UNIT_DEPTH * 0.5), normal))
                        .collect();
                    debug_assert_eq!(faces.len(), CASES_PER_CATEGORY);
                    self.add_category(rng, cat, &faces, &mut titles);
                    cat += 1;
                }
            }
        }
    }

    fn build_center(&mut self) {
        // Rug.
        let second = [22u32, 30, 70];
        let mut rug = Inst::aabox(
            Vec3::new(0.0, 0.012, 0.0),
            Vec3::new(11.0, 0.024, 8.0),
            kind::RUG,
            [0.28, 0.035, 0.03],
        );
        rug.param = second[0] | (second[1] << 8) | (second[2] << 16);
        self.rug(rug);

        // Reading tables with chairs and candles, and a green-shaded lamp on each.
        for (tx, tz) in [(-4.5f32, -2.6f32), (4.5, -2.6), (-4.5, 2.6), (4.5, 2.6)] {
            self.reading_table(Vec3::new(tx, 0.0, tz));
            self.desk_lamp(Vec3::new(tx + 0.25, TABLE_H, tz - tz.signum() * 0.38));
        }

        // Chandelier, from the boss where the middle rib crosses the middle beam.
        self.chandelier(Vec3::new(0.0, 5.6, 0.0), BEAM_Y - 0.07);

        // Fireplace on the south wall.
        let fz = HALL_Z;
        let fc = Vec3::new(0.0, 0.0, fz);
        let st = [0.4, 0.37, 0.33];
        self.decor(Inst::minmax(
            fc + Vec3::new(-2.2, 0.0, -0.9),
            fc + Vec3::new(-1.4, 2.6, 0.0),
            kind::STONE,
            st,
        ));
        self.decor(Inst::minmax(
            fc + Vec3::new(1.4, 0.0, -0.9),
            fc + Vec3::new(2.2, 2.6, 0.0),
            kind::STONE,
            st,
        ));
        self.decor(Inst::minmax(
            fc + Vec3::new(-2.4, 2.0, -1.0),
            fc + Vec3::new(2.4, 2.7, 0.0),
            kind::STONE,
            st,
        ));
        self.decor(Inst::minmax(
            fc + Vec3::new(-2.5, 2.7, -1.1),
            fc + Vec3::new(2.5, 2.85, 0.0),
            kind::WOOD,
            WALNUT,
        ));
        self.decor(Inst::minmax(
            fc + Vec3::new(-1.6, 2.85, -0.6),
            fc + Vec3::new(1.6, HALL_H, 0.0),
            kind::STONE,
            [0.33, 0.3, 0.27],
        ));
        self.decor(Inst::minmax(
            fc + Vec3::new(-2.4, 0.0, -1.6),
            fc + Vec3::new(2.4, 0.08, -0.9),
            kind::STONE,
            [0.25, 0.23, 0.21],
        ));
        self.decor(Inst::minmax(
            fc + Vec3::new(-1.4, 0.0, -0.5),
            fc + Vec3::new(1.4, 2.0, -0.4),
            kind::PLAIN,
            [0.02, 0.015, 0.012],
        ));
        for (i, (x, ang)) in [(-0.3f32, 0.3f32), (0.25, -0.25), (0.0, 0.05)].iter().enumerate() {
            let p = fc + Vec3::new(*x, 0.12 + i as f32 * 0.07, -0.75);
            self.decor(Inst::new(
                p,
                Quat::from_rotation_y(*ang),
                Vec3::new(0.9, 0.14, 0.14),
                kind::WOOD,
                [0.08, 0.04, 0.02, 1.0],
                0,
            ));
        }
        for k in 0..7 {
            let x = -0.45 + k as f32 * 0.15;
            let h = 0.25 + 0.2 * ((k as f32 * 1.7).sin() * 0.5 + 0.5);
            let mut flame = Inst::aabox(
                fc + Vec3::new(x, 0.25 + h * 0.5, -0.75),
                Vec3::new(0.09, h, 0.09),
                kind::EMISSIVE,
                [1.0, 0.42, 0.1],
            );
            flame.color[3] = 8.0;
            flame.param = 1;
            self.decor(flame);
        }
        self.collide(
            fc + Vec3::new(-2.5, 0.0, -1.7),
            fc + Vec3::new(2.5, 3.0, 0.0),
            Stops::ALL,
        );
        // The chimney breast, up to the ceiling.
        self.collide(
            fc + Vec3::new(-1.6, 0.0, -0.6),
            fc + Vec3::new(1.6, HALL_H, 0.0),
            Stops::ALL,
        );
        self.light(
            fc + Vec3::new(0.0, 0.8, -1.6),
            11.0,
            [1.0, 0.45, 0.15],
            3.0,
            LightKind::Flicker,
        );
        self.hearth_dressing(fc);

        // An armillary sphere and a lectern with an open book: floor pieces, so solid,
        // where no pile ever lay (the reading room's middle is kept clear of piles,
        // and these stand more than a spot's clearance inside it) and clear of the
        // Gathering Charm's heap at the hub.
        self.armillary(Vec3::new(-2.0, 0.0, -5.4));
        self.lectern(Vec3::new(2.0, 0.0, -5.4));
    }

    /// What dresses the hearth: a fender, firedogs and fire irons inside the
    /// fireplace's guard; candlesticks and a clock on the mantel; the library's emblem
    /// over it; a rug before it.
    fn hearth_dressing(&mut self, fc: Vec3) {
        let brass = rgba(BRASS);
        let iron = rgba(IRON);
        let metal =
            |lib: &mut Self, min: Vec3, max: Vec3, rgb| lib.decor(Inst::minmax(fc + min, fc + max, kind::METAL, rgb));
        metal(self, Vec3::new(-2.2, 0.08, -1.56), Vec3::new(2.2, 0.2, -1.5), BRASS);
        for s in [-1.0f32, 1.0] {
            let (a, b) = (s * 2.17, s * 2.23);
            metal(
                self,
                Vec3::new(a.min(b), 0.08, -1.5),
                Vec3::new(a.max(b), 0.2, -0.9),
                BRASS,
            );
            // Firedogs under the logs.
            let x = s * 0.55;
            metal(
                self,
                Vec3::new(x - 0.03, 0.08, -1.08),
                Vec3::new(x + 0.03, 0.34, -1.02),
                IRON,
            );
            metal(
                self,
                Vec3::new(x - 0.02, 0.06, -1.02),
                Vec3::new(x + 0.02, 0.1, -0.45),
                IRON,
            );
            let knob = fc + Vec3::new(x, 0.38, -1.05);
            self.decor(Inst::new(
                knob,
                Quat::from_rotation_y(FRAC_PI_4),
                Vec3::splat(0.08),
                kind::METAL,
                brass,
                0,
            ));
        }
        // Fire irons on a stand by the right jamb, on the hearthstone.
        let stand = fc + Vec3::new(2.32, 0.08, -1.3);
        self.decor(Inst::aabox(
            stand + Vec3::Y * 0.01,
            Vec3::new(0.16, 0.02, 0.16),
            kind::METAL,
            IRON,
        ));
        self.decor(Inst::aabox(
            stand + Vec3::Y * 0.4,
            Vec3::new(0.03, 0.78, 0.03),
            kind::METAL,
            IRON,
        ));
        let top = stand + Vec3::Y * 0.78;
        for lean in [-0.14f32, 0.0, 0.14] {
            let rot = Quat::from_rotation_x(lean);
            let tool = top + rot * Vec3::new(0.0, -0.33, 0.0);
            self.decor(Inst::new(tool, rot, Vec3::new(0.02, 0.6, 0.02), kind::METAL, iron, 0));
            let handle = top + rot * Vec3::new(0.0, 0.04, 0.0);
            self.decor(Inst::new(
                handle,
                rot,
                Vec3::new(0.035, 0.09, 0.035),
                kind::METAL,
                brass,
                0,
            ));
        }
        // On the mantel: candlesticks at its ends and a clock between.
        let shelf = fc + Vec3::new(0.0, 2.85, -0.85);
        for s in [-1.0f32, 1.0] {
            let p = shelf + Vec3::X * s * 2.1;
            self.decor(Inst::aabox(
                p + Vec3::Y * 0.015,
                Vec3::new(0.14, 0.03, 0.14),
                kind::METAL,
                BRASS,
            ));
            self.decor(Inst::aabox(
                p + Vec3::Y * 0.17,
                Vec3::new(0.04, 0.28, 0.04),
                kind::METAL,
                BRASS,
            ));
            self.decor(Inst::aabox(
                p + Vec3::Y * 0.32,
                Vec3::new(0.1, 0.02, 0.1),
                kind::METAL,
                BRASS,
            ));
            self.decor(Inst::aabox(
                p + Vec3::Y * 0.43,
                Vec3::new(0.045, 0.2, 0.045),
                kind::PLAIN,
                [0.8, 0.75, 0.6],
            ));
            let mut flame = Inst::aabox(
                p + Vec3::Y * 0.56,
                Vec3::new(0.025, 0.05, 0.025),
                kind::EMISSIVE,
                [1.0, 0.55, 0.18],
            );
            flame.color[3] = 14.0;
            flame.param = 1;
            self.decor(flame);
        }
        self.decor(Inst::aabox(
            shelf + Vec3::Y * 0.22,
            Vec3::new(0.44, 0.44, 0.2),
            kind::WOOD,
            WALNUT,
        ));
        self.decor(Inst::aabox(
            shelf + Vec3::Y * 0.48,
            Vec3::new(0.3, 0.08, 0.16),
            kind::WOOD,
            WALNUT,
        ));
        let mantel = Mount {
            at: shelf - Vec3::Z * 0.1,
            n: -Vec3::Z,
        };
        self.disc(
            mantel,
            Vec3::new(0.0, 0.24, 0.01),
            (0.15, 0.01),
            kind::PLAIN,
            rgba(CREAM),
        );
        for (a, len) in [(1.9f32, 0.09f32), (0.5, 0.12)] {
            let at = Vec3::new(0.0, 0.24, 0.022) + Vec3::new(a.cos(), a.sin(), 0.0) * len * 0.45;
            self.mounted(mantel, at, Vec3::new(len, 0.012, 0.004), a, kind::METAL, iron);
        }
        // The emblem over the mantel: a crescent moon over an open book, gilt on a
        // blue field in a stone frame.
        let m = Mount {
            at: fc + Vec3::new(0.0, 0.0, -0.6),
            n: -Vec3::Z,
        };
        let y = 4.5;
        let field = rgba(SKY);
        self.mounted(
            m,
            Vec3::new(0.0, y, 0.03),
            Vec3::new(1.6, 1.4, 0.06),
            0.0,
            kind::PLAIN,
            field,
        );
        let dressed = rgba(DRESSED);
        for s in [-1.0f32, 1.0] {
            let rail = Vec3::new(0.0, y + s * 0.8, 0.06);
            self.mounted(m, rail, Vec3::new(2.0, 0.2, 0.12), 0.0, kind::STONE, dressed);
            let stile = Vec3::new(s * 0.9, y, 0.06);
            self.mounted(m, stile, Vec3::new(0.2, 1.4, 0.12), 0.0, kind::STONE, dressed);
        }
        let gilt = rgba(GILT);
        let moon = Vec3::new(0.0, y + 0.25, 0.065);
        self.disc(m, moon, (0.26, 0.01), kind::METAL, gilt);
        self.disc(m, moon + Vec3::new(0.11, 0.07, 0.005), (0.23, 0.01), kind::PLAIN, field);
        for s in [-1.0f32, 1.0] {
            let page = Vec3::new(s * 0.25, y - 0.3, 0.068);
            self.mounted(m, page, Vec3::new(0.46, 0.3, 0.01), s * 0.18, kind::METAL, gilt);
        }
        for (sx, sy) in [(-0.5f32, 0.35f32), (0.5, 0.45), (-0.4, 0.05), (0.45, 0.0)] {
            let star = Vec3::new(sx, y + sy, 0.065);
            self.mounted(m, star, Vec3::new(0.07, 0.07, 0.01), FRAC_PI_4, kind::METAL, gilt);
        }
        // A rug before the hearth.
        let mut rug = Inst::aabox(
            fc + Vec3::new(0.0, 0.009, -3.0),
            Vec3::new(3.2, 0.018, 1.4),
            kind::RUG,
            [0.1, 0.13, 0.2],
        );
        rug.param = 120 | (30 << 8) | (25 << 16);
        self.rug(rug);
    }

    /// A banker's lamp standing on a table at `p`: a brass stem under a green glass
    /// shade, lit from under it.
    fn desk_lamp(&mut self, p: Vec3) {
        let brass =
            |lib: &mut Self, y: f32, size: Vec3| lib.decor(Inst::aabox(p + Vec3::Y * y, size, kind::METAL, BRASS));
        brass(self, 0.015, Vec3::new(0.22, 0.03, 0.14));
        brass(self, 0.19, Vec3::new(0.025, 0.32, 0.025));
        brass(self, 0.33, Vec3::new(0.05, 0.025, 0.05));
        // The shade, a half-round of green glass: a crown and two slopes.
        let glass = [0.02, 0.17, 0.065, 0.3];
        let shade = p + Vec3::Y * 0.37;
        self.decor(Inst::new(
            shade + Vec3::Y * 0.035,
            Quat::IDENTITY,
            Vec3::new(0.4, 0.012, 0.07),
            kind::CRYSTAL,
            glass,
            0,
        ));
        for s in [-1.0f32, 1.0] {
            let rot = Quat::from_rotation_x(s * 0.8);
            let at = shade + Vec3::new(0.0, 0.0, s * 0.055);
            self.decor(Inst::new(at, rot, Vec3::new(0.4, 0.012, 0.1), kind::CRYSTAL, glass, 0));
        }
        let mut bulb = Inst::aabox(
            shade - Vec3::Y * 0.02,
            Vec3::new(0.3, 0.02, 0.06),
            kind::EMISSIVE,
            [1.0, 0.8, 0.5],
        );
        bulb.color[3] = 4.0;
        self.decor(bulb);
        self.light(p + Vec3::Y * 0.25, 3.2, [1.0, 0.82, 0.55], 0.7, LightKind::Static);
    }

    /// An armillary sphere on a turned stand at `c`: horizon, meridian, equator and
    /// ecliptic rings round a small globe on its polar axis.
    fn armillary(&mut self, c: Vec3) {
        let wood =
            |lib: &mut Self, y: f32, size: Vec3, rgb| lib.decor(Inst::aabox(c + Vec3::Y * y, size, kind::WOOD, rgb));
        wood(self, 0.04, Vec3::new(0.7, 0.08, 0.7), WALNUT);
        wood(self, 0.12, Vec3::new(0.42, 0.08, 0.42), DARK_WOOD);
        wood(self, 0.5, Vec3::new(0.12, 0.7, 0.12), WALNUT);
        wood(self, 0.45, Vec3::new(0.2, 0.12, 0.2), WALNUT);
        self.decor(Inst::aabox(
            c + Vec3::Y * 0.84,
            Vec3::new(0.24, 0.06, 0.24),
            kind::METAL,
            BRASS,
        ));
        let o = c + Vec3::Y * 1.32;
        let r = 0.42;
        let brass = rgba(BRASS);
        // The horizon, a wooden ring on four legs.
        self.arc(
            o,
            (Vec3::X, Vec3::Z),
            (0.52, 0.09, 0.035),
            (0.0, TAU, 20),
            (kind::WOOD, rgba(WALNUT)),
        );
        for k in 0..4 {
            let a = FRAC_PI_4 + k as f32 * FRAC_PI_2;
            let d = Vec3::new(a.cos(), 0.0, a.sin());
            self.beam(
                c + d * 0.3 + Vec3::Y * 0.08,
                o + d * 0.52 - Vec3::Y * 0.02,
                0.035,
                0.035,
                WALNUT,
            );
        }
        // The meridian stands square to the horizon, turned off the room's axes so it
        // shows as a ring from the doorway and the hearth; the pole leans along it.
        let turn = Quat::from_rotation_y(0.6);
        let (along, x) = (turn * Vec3::Z, turn * Vec3::X);
        self.arc(
            o,
            (along, Vec3::Y),
            (r, 0.05, 0.03),
            (0.0, TAU, 20),
            (kind::METAL, brass),
        );
        let lat = 0.9f32;
        let pole = Vec3::Y * lat.sin() - along * lat.cos();
        let across = pole.cross(x).normalize();
        self.arc(
            o,
            (x, across),
            (r - 0.04, 0.04, 0.025),
            (0.0, TAU, 20),
            (kind::METAL, brass),
        );
        let tilt = Quat::from_axis_angle(x, 0.41);
        let gilt = rgba(GILT);
        self.arc(
            o,
            (x, tilt * across),
            (r - 0.04, 0.07, 0.02),
            (0.0, TAU, 20),
            (kind::METAL, gilt),
        );
        let axis = Quat::from_rotation_arc(Vec3::Y, pole);
        self.decor(Inst::new(
            o,
            axis,
            Vec3::new(0.02, 2.0 * r + 0.16, 0.02),
            kind::METAL,
            brass,
            0,
        ));
        let earth = [0.1, 0.24, 0.32, 1.0];
        self.decor(Inst::new(o, axis, Vec3::splat(0.13), kind::PLAIN, earth, 0));
        let tumbled = axis * Quat::from_rotation_y(FRAC_PI_4) * Quat::from_rotation_x(FRAC_PI_4);
        self.decor(Inst::new(o, tumbled, Vec3::splat(0.13), kind::PLAIN, earth, 0));
        self.collide(
            c - Vec3::new(0.62, 0.0, 0.62),
            c + Vec3::new(0.62, 1.86, 0.62),
            Stops::RAIL,
        );
    }

    /// A lectern at `c` facing south, its great book open and glowing faintly, motes
    /// of light rising off the pages: the reading room's one enchantment within reach.
    fn lectern(&mut self, c: Vec3) {
        self.decor(Inst::aabox(
            c + Vec3::Y * 0.05,
            Vec3::new(0.6, 0.1, 0.5),
            kind::STONE,
            DRESSED,
        ));
        self.decor(Inst::aabox(
            c + Vec3::Y * 0.15,
            Vec3::new(0.34, 0.1, 0.3),
            kind::STONE,
            DRESSED,
        ));
        let wood =
            |lib: &mut Self, y: f32, size: Vec3| lib.decor(Inst::aabox(c + Vec3::Y * y, size, kind::WOOD, WALNUT));
        wood(self, 0.55, Vec3::new(0.15, 0.7, 0.15));
        for y in [0.32, 0.62] {
            wood(self, y, Vec3::new(0.2, 0.05, 0.2));
        }
        wood(self, 0.92, Vec3::new(0.22, 0.06, 0.22));
        let desk_at = c + Vec3::Y * 1.05;
        let desk = Quat::from_rotation_x(0.38);
        let walnut = rgba(WALNUT);
        self.decor(Inst::new(
            desk_at,
            desk,
            Vec3::new(0.8, 0.04, 0.56),
            kind::WOOD,
            walnut,
            0,
        ));
        let lip = desk_at + desk * Vec3::new(0.0, 0.04, 0.27);
        self.decor(Inst::new(lip, desk, Vec3::new(0.8, 0.06, 0.03), kind::WOOD, walnut, 0));
        // The book: its boards, the pages rising to the spine, and a ribbon.
        let book = desk_at + desk * Vec3::new(0.0, 0.03, -0.02);
        let boards = [0.26, 0.05, 0.04, 1.0];
        self.decor(Inst::new(
            book,
            desk,
            Vec3::new(0.74, 0.02, 0.5),
            kind::PLAIN,
            boards,
            0,
        ));
        for s in [-1.0f32, 1.0] {
            let at = book + desk * Vec3::new(s * 0.18, 0.035, 0.0);
            let rot = desk * Quat::from_rotation_z(-s * 0.12);
            let page = [0.72, 0.66, 0.5, 1.0];
            self.decor(Inst::new(
                at,
                rot,
                Vec3::new(0.34, 0.045, 0.46),
                kind::PARCHMENT,
                page,
                0,
            ));
            // Its writing glows.
            for line in 0..7 {
                let len = if line % 3 == 2 { 0.16 } else { 0.24 };
                let off = Vec3::new(-0.01 * s, 0.024, -0.17 + line as f32 * 0.055);
                let glow = [0.3, 0.55, 1.0, 1.6];
                let ink = Vec3::new(len, 0.003, 0.012);
                self.decor(Inst::new(at + rot * off, rot, ink, kind::CRYSTAL, glow, 0));
            }
        }
        let ribbon = book + desk * Vec3::new(0.02, 0.06, 0.27);
        let red = [0.5, 0.06, 0.06, 1.0];
        self.decor(Inst::new(
            ribbon,
            desk,
            Vec3::new(0.025, 0.004, 0.14),
            kind::CLOTH,
            red,
            0,
        ));
        let mut r = StdRng::seed_from_u64(0x1ec7);
        for k in 0..11 {
            let up = 0.1 + k as f32 * 0.07;
            let a = k as f32 * 2.4;
            let at = book + Vec3::new(a.cos() * 0.14, up, a.sin() * 0.1);
            let s = r.gen_range(0.012..0.022);
            let rot = Quat::from_rotation_y(a) * Quat::from_rotation_x(FRAC_PI_4);
            let glow = r.gen_range(1.5..3.0) * (1.0 - k as f32 * 0.06);
            self.decor(Inst::new(
                at,
                rot,
                Vec3::splat(s),
                kind::CRYSTAL,
                [0.55, 0.75, 1.0, glow],
                0,
            ));
        }
        self.light(book + Vec3::Y * 0.8, 3.0, [0.55, 0.72, 1.0], 0.5, LightKind::Flicker);
        self.collide(
            c - Vec3::new(0.42, 0.0, 0.36),
            c + Vec3::new(0.42, 1.3, 0.36),
            Stops::RAIL,
        );
    }

    /// Scatter the books in piles: a few on the reading tables, the rest on the floor
    /// of their own wing. Only drop points are chosen here; `Loose` settles them.
    fn build_piles(&mut self, rng: &mut StdRng) {
        for wing in 0..2 {
            let mut pool: Vec<u32> = (0..self.books.len() as u32)
                .filter(|&b| self.dept_of(self.category_of_book(b)) == wing)
                .collect();
            pool.shuffle(rng);
            if wing == 0 {
                for (tx, tz) in [(-4.5f32, -2.6f32), (4.5, -2.6), (-4.5, 2.6), (4.5, 2.6)] {
                    let n = rng.gen_range(14..28).min(pool.len());
                    let books: Vec<u32> = pool.drain(..n).collect();
                    self.add_pile(rng, 0, Vec2::new(tx - 0.35, tz), 0.32, books);
                }
            }
            let (x0, x1) = if wing == 0 {
                (-HALL_X + 0.8, VEIL_X - 0.8)
            } else {
                (VEIL_X + 0.8, HALL_X - 0.8)
            };
            // Choose spots first, then share the wing's books evenly between them.
            let spot_r = 1.0;
            let mut spots: Vec<Vec2> = Vec::new();
            for _ in 0..20000 {
                let p = Vec2::new(
                    rng.gen_range(x0 + spot_r..x1 - spot_r),
                    rng.gen_range(-HALL_Z + 0.8 + spot_r..HALL_Z - 0.8 - spot_r),
                );
                if self.pile_spot_ok(p, spot_r, &spots) {
                    spots.push(p);
                }
            }
            let count = spots.len();
            for (i, p) in spots.into_iter().enumerate() {
                let left = count - i;
                let n = if left == 1 {
                    pool.len()
                } else {
                    ((pool.len() as f32 / left as f32) * rng.gen_range(0.75..1.25)) as usize
                };
                let n = n.min(pool.len());
                if n == 0 {
                    continue;
                }
                let radius = 0.4 + 0.06 * (n as f32).sqrt();
                let books: Vec<u32> = pool.drain(..n).collect();
                self.add_pile(rng, 0, p, radius, books);
            }
        }
    }

    fn pile_spot_ok(&self, p: Vec2, r: f32, spots: &[Vec2]) -> bool {
        for o in self.world.solids().iter().filter(|o| o.stops.has(Stops::BODIES)) {
            let q = Vec2::new(p.x.clamp(o.min.x, o.max.x), p.y.clamp(o.min.z, o.max.z));
            if q.distance(p) < r + 1.1 {
                return false;
            }
        }
        if p.x.abs() < 6.0 && p.y.abs() < 8.5 {
            return false; // keep the reading room's middle (and the spawn point) walkable
        }
        if p.y > HALL_Z - 3.0 && p.x.abs() < 3.5 {
            return false; // hearth
        }
        if (p.x - VEIL_X).abs() < r + 1.0 {
            return false;
        }
        !spots.iter().any(|q| q.distance(p) < r * 2.0 + 1.1)
    }
}
