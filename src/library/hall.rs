//! The Hall: one storey, two wings of freestanding bookcases either side of a reading
//! room, the East Wing veiled until the West is mostly sorted. Its generation must
//! not change (saves name books by index; see `hall_generates_as_before`): what is
//! added to it is drawn only, or solid only where no pile could ever have lain.

use super::*;
use crate::audio::Sfx;
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
/// Where the chandelier hangs over the middle of the reading room.
const CHANDELIER: Vec3 = Vec3::new(0.0, 5.6, 0.0);
/// The reading room's ceiling is painted as the night sky.
const SKY: [f32; 3] = [0.05, 0.075, 0.2];
/// The wings' ceilings, pale between dark timbers.
const PLASTER: [f32; 3] = [0.3, 0.27, 0.22];
const VELVET: [f32; 3] = [0.3, 0.035, 0.045];
const CREAM: [f32; 3] = [0.8, 0.74, 0.58];
/// The reading room's rug, and the runners laid down the wings to match it once they
/// are in order: its half size, its red, and its border's navy (`kind::RUG`'s param).
const RUG_HALF: Vec2 = Vec2::new(5.5, 4.0);
const RUG_RED: [f32; 3] = [0.28, 0.035, 0.03];
const RUG_BORDER: u32 = 22 | (30 << 8) | (70 << 16);

/// When a wing is put in order: how long before its runner starts, its rugs, how long
/// each takes to settle and how far apart they start; then the wall at its end wakes.
const RUNNER_AFTER: f32 = 1.0;
const RUNNER_RUGS: usize = 6;
const RUG_SETTLES: f32 = 1.4;
const RUG_STAGGER: f32 = 0.5;
const WING_WAKES: f32 = RUNNER_AFTER + (RUNNER_RUGS - 1) as f32 * RUG_STAGGER + RUG_SETTLES;
/// When the whole Hall is: how long its finale waits, for the last wing's moment.
const FINALE: f32 = WING_WAKES + 3.0;

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
    // What waits for a category, a wing (the walls at their ends wake as well:
    // `wall_map`, `clock`) and the whole Hall to be put in order.
    lib.crests();
    for wing in 0..2 {
        lib.runner(wing);
    }
    lib.finale();
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

    /// A box on it at `off`, turned `spin` about its normal.
    fn boxed(self, off: Vec3, size: Vec3, spin: f32, kind: u32, color: [f32; 4]) -> Inst {
        Inst::new(self.at(off), self.rot(spin), size, kind, color, 0)
    }
}

/// A mover that stirs now and then about `axis` through `pivot`: two swings `amp`
/// either way, one every `period` and one a little slower, so that it sways while
/// they are in step and rests while they cancel, once every `beat` seconds.
fn stir(lib: &mut Library, pivot: Vec3, axis: Vec3, (amp, period, beat): (f32, f32, f32), phase: f32) -> u32 {
    let first = lib.mover(pivot, Motion::Swing { axis, amp, period }, phase, None);
    let period = 1.0 / (1.0 / period - 1.0 / beat);
    lib.mover(pivot, Motion::Swing { axis, amp, period }, phase, Some(first))
}

/// A small gilt finial, a ball on a collar, its foot at the origin and pointing up y.
fn finial(lib: &mut Library) -> Shape {
    let p = [
        (0.0, 0.0),
        (0.03, 0.0),
        (0.03, 0.012),
        (0.018, 0.022),
        (0.022, 0.032),
        (0.05, 0.06),
        (0.045, 0.085),
        (0.024, 0.105),
        (0.008, 0.125),
        (0.0, 0.13),
    ];
    lib.turned(&p, 16)
}

/// A tassel: a knob on a cord, and the threads falling from it in a skirt; its head
/// at the origin, hanging down y.
fn tassel(lib: &mut Library) -> Shape {
    let p = [
        (0.0, -0.22),
        (0.03, -0.22),
        (0.027, -0.1),
        (0.013, -0.065),
        (0.021, -0.045),
        (0.016, -0.02),
        (0.006, -0.008),
        (0.0, 0.0),
    ];
    lib.turned(&p, 12)
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
            let half = libm::asinf(span * 0.5 / r);
            let n = if span > 10.0 { 14 } else { 8 };
            self.arc(
                centre,
                (Vec3::Z, Vec3::Y),
                (r - 0.14, 0.28, 0.24),
                (FRAC_PI_2 - half, FRAC_PI_2 + half, n),
                (kind::WOOD, rgba(BEAM_WOOD, 1.0)),
            );
            if sx > 0.0 && span > 10.0 {
                self.wing_names(centre, r - 0.14);
            }
        }
        // A gilt crescent on a dark roundel hung at the crown of the middle arch, on
        // both its faces.
        for n in [Vec3::X, -Vec3::X] {
            let m = Mount {
                at: Vec3::new(sx, 0.0, 0.0) + n * 0.12,
                n,
            };
            let at = Vec3::new(0.0, GIRDER_Y - 0.62, 0.0);
            self.disc(m, at, (0.4, 0.02), kind::WOOD, rgba(BEAM_WOOD, 1.0));
            self.disc(m, at + Vec3::Z * 0.025, (0.3, 0.03), kind::METAL, rgba(GILT, 1.0));
            let shadow = at + Vec3::new(0.1, 0.07, 0.035);
            self.disc(m, shadow, (0.25, 0.03), kind::WOOD, rgba(BEAM_WOOD, 1.0));
        }
    }

    /// The wings named in gold leaf on the faces of the arch over the veil, either side
    /// of its roundel: the East Wing from the reading room, the West from the East. The
    /// arch is about `c` (on the veil's line), `r` to the middle of its band. Lettering
    /// only, not a `Label`: the Signposts' guides are the boards', and this leaves them
    /// as they were.
    fn wing_names(&mut self, c: Vec3, r: f32) {
        const LETTERS: Style = Style {
            em: 0.23,
            tracking: 0.12,
            lettering: Lettering::Gilt,
            color: GOLD_LEAF,
        };
        for (n, words) in [(-Vec3::X, ["EAST", "WING"]), (Vec3::X, ["WEST", "WING"])] {
            let right = Vec3::Y.cross(n);
            for (word, u) in words.into_iter().zip([-0.95f32, 0.95]) {
                // Along the arch's curve and up out of it, at `u` along the reading way.
                let z = right.z * u;
                let rise = libm::sqrtf(r * r - z * z);
                let at = Vec3::new(c.x, c.y + rise, c.z + z) + n * (0.12 + 0.0015);
                let along = (right + Vec3::Y * (-z / rise * right.z)).normalize();
                text::set_line(&mut self.text, word, LETTERS, at, along, n.cross(along), 0.9);
            }
        }
    }

    /// A band round an arc about `c` in the plane of `u` and `v` (unit, at right
    /// angles), `r` from it to the band's middle, `w` across the arc and `d` through
    /// its plane; from angle `a0` to `a1` (0 along `u`, a quarter turn along `v`), or
    /// all the way round. `n` says how finely: at least that many facets.
    fn arc(
        &mut self,
        c: Vec3,
        (u, v): (Vec3, Vec3),
        (r, w, d): (f32, f32, f32),
        (a0, a1, n): (f32, f32, usize),
        (kind, color): (u32, [f32; 4]),
    ) {
        let i = self.arc_inst(c, (u, v), (r, w, d), (a0, a1, n), (kind, color));
        self.decor(i);
    }

    /// The band `arc` lays down.
    fn arc_inst(
        &mut self,
        c: Vec3,
        (u, v): (Vec3, Vec3),
        (r, w, d): (f32, f32, f32),
        (a0, a1, n): (f32, f32, usize),
        (kind, color): (u32, [f32; 4]),
    ) -> Inst {
        let span = a1 - a0;
        let segs = n.max((span.abs() / (TAU / 64.0)).ceil() as usize);
        let at = |k: usize, rr: f32| {
            let a = a0 + span * k as f32 / segs as f32;
            Vec2::new(libm::cosf(a), libm::sinf(a)) * rr
        };
        let (outer, inner) = (r + w * 0.5, r - w * 0.5);
        let mesh = if (span.abs() - TAU).abs() < 1e-4 {
            let ring = |rr: f32| (0..segs).map(|k| at(k, rr)).collect::<Vec<_>>();
            Mesh::ring(&ring(outer), &ring(inner), d)
        } else {
            let mut band: Vec<Vec2> = (0..=segs).map(|k| at(k, outer)).collect();
            band.extend((0..=segs).rev().map(|k| at(k, inner)));
            Mesh::prism(&band, d)
        };
        let rot = Quat::from_mat3(&Mat3::from_cols(u, v, u.cross(v)));
        self.shape(mesh).at(c, rot, kind, color)
    }

    /// A box on a wall (see `Mount`).
    fn mounted(&mut self, m: Mount, off: Vec3, size: Vec3, spin: f32, kind: u32, color: [f32; 4]) {
        self.decor(m.boxed(off, size, spin, kind, color));
    }

    /// A disc on a wall, `r` round and `d` thick at `off`.
    fn disc(&mut self, m: Mount, off: Vec3, (r, d): (f32, f32), kind: u32, color: [f32; 4]) {
        let i = self.disc_inst(m, off, (r, d), kind, color);
        self.decor(i);
    }

    /// The disc `disc` lays down.
    fn disc_inst(&mut self, m: Mount, off: Vec3, (r, d): (f32, f32), kind: u32, color: [f32; 4]) -> Inst {
        let segs = match r {
            r if r < 0.08 => 16,
            r if r < 0.45 => 32,
            _ => 64,
        };
        let disc = self.shape(Mesh::cylinder(segs)).sized(Vec3::new(2.0 * r, d, 2.0 * r));
        disc.inst(m.at(off), m.rot(0.0) * Quat::from_rotation_x(FRAC_PI_2), kind, color)
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
        // Gilt bosses where the ribs cross the beams, turned, hanging from them.
        let boss = self.turned(
            &[
                (0.0, 0.0),
                (0.15, 0.0),
                (0.15, 0.012),
                (0.128, 0.026),
                (0.11, 0.03),
                (0.075, 0.05),
                (0.04, 0.066),
                (0.03, 0.07),
                (0.0, 0.08),
            ],
            24,
        );
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
            for x in RIBS_X {
                let hang = Quat::from_rotation_x(PI);
                self.decor(boss.at(Vec3::new(x, y, z), hang, kind::METAL, rgba(GILT, 1.0)));
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
        let gilt = rgba(GILT, 1.0);
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
                    let q = (p + Vec2::new(libm::cosf(a), libm::sinf(a)) * r.gen_range(0.5..0.9))
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
                    let rot = Quat::from_rotation_y(libm::atan2f(-d.y, d.x));
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
                rgba(rgb, 1.0),
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
                rgba(DRESSED, 1.0),
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
                rgba(STONE, 1.0),
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
            (kind::STONE, rgba(DRESSED, 1.0)),
        );
        let dressed = rgba(DRESSED, 1.0);
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
        let velvet = rgba(VELVET, 1.0);
        let gilt = rgba(GILT, 1.0);
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
        // Falling to the sill, clear above anyone's head where they stir.
        let (top, tie, foot) = (6.95, 3.4, 2.25);
        let tassel = tassel(self);
        for s in [-1.0f32, 1.0] {
            let mut below = Vec::new();
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
                        m.rot(libm::atan2f(d.y, d.x)) * Quat::from_rotation_x(fold),
                        Vec3::new(d.length() + 0.04, w, 0.03),
                        kind::CLOTH,
                        velvet,
                        0,
                    );
                    if q == c {
                        below.push(pleat);
                    } else {
                        self.decor(pleat);
                    }
                }
            }
            let band = Vec3::new(s * 1.37, tie, 0.27);
            self.mounted(m, band, Vec3::new(0.36, 0.08, 0.1), 0.0, kind::METAL, gilt);
            below.push(tassel.at(
                m.at(Vec3::new(s * 1.52, tie - 0.05, 0.3)),
                Quat::IDENTITY,
                kind::CLOTH,
                gilt,
            ));
            // Below the tieback each curtain sways a little in a draught off the glass,
            // now and then, each in its own time.
            let mut r = StdRng::seed_from_u64(0xd2a9 ^ x.to_bits() as u64 ^ (s > 0.0) as u64);
            let sway = (0.011, r.gen_range(5.5..7.5), r.gen_range(50.0..90.0));
            let phase = r.gen_range(0.0..90.0);
            let mover = stir(self, m.at(Vec3::new(s * 1.37, tie, 0.24)), m.n, sway, phase);
            for pleat in below {
                self.moving(mover, pleat);
            }
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
    /// roundel with a crescent moon over an open book. It stirs on its rod now and then.
    fn tapestry(&mut self, m: Mount, s: f32) {
        let (w, h, mid) = (3.0f32, 4.0f32, 4.45f32);
        let field = if s < 0.0 { [0.06, 0.16, 0.12] } else { [0.08, 0.1, 0.22] };
        let mut parts = vec![
            m.boxed(
                Vec3::new(0.0, mid, 0.08),
                Vec3::new(w, h, 0.02),
                0.0,
                kind::CLOTH,
                rgba([0.3, 0.07, 0.05], 1.0),
            ),
            m.boxed(
                Vec3::new(0.0, mid, 0.09),
                Vec3::new(w - 0.36, h - 0.36, 0.02),
                0.0,
                kind::CLOTH,
                rgba(field, 1.0),
            ),
        ];
        // The lozenges, gilt and cream, each colour one piece (it moves).
        let mut r = StdRng::seed_from_u64(0x7a9e ^ (s > 0.0) as u64);
        let (mut gilt, mut cream) = (Mesh::default(), Mesh::default());
        let lozenge = Mesh::cube();
        for i in 0..6 {
            for j in 0..7 {
                let p = Vec2::new(-1.1 + i as f32 * 0.44, mid - 1.6 + j as f32 * 0.53);
                if (p - Vec2::new(0.0, mid)).length() < 1.05 {
                    continue;
                }
                let at = Vec3::new(p.x + (j % 2) as f32 * 0.22 - 0.11, p.y, 0.0);
                let to = if r.gen_range(0.0..1.0f32) < 0.75 {
                    &mut gilt
                } else {
                    &mut cream
                };
                to.add(
                    &lozenge,
                    at,
                    Quat::from_rotation_z(FRAC_PI_4),
                    Vec3::new(0.09, 0.09, 0.01),
                );
            }
        }
        for (mesh, c) in [(gilt, GILT), (cream, CREAM)] {
            if mesh.idx.is_empty() {
                continue;
            }
            let shape = self.shape(mesh);
            parts.push(shape.at(m.at(Vec3::Z * 0.1), m.rot(0.0), kind::CLOTH, rgba(c, 1.0)));
        }
        let centre = Vec3::new(0.0, mid, 0.1);
        let night = rgba([0.04, 0.05, 0.12], 1.0);
        let moon = centre + Vec3::new(0.0, 0.25, 0.012);
        for (at, r, c) in [
            (centre, 0.85, rgba(GILT, 1.0)),
            (centre + Vec3::Z * 0.006, 0.75, night),
            (moon, 0.24, rgba(CREAM, 1.0)),
            (moon + Vec3::new(0.1, 0.06, 0.006), 0.21, night),
        ] {
            parts.push(self.disc_inst(m, at, (r, 0.012), kind::CLOTH, c));
        }
        for side in [-1.0f32, 1.0] {
            let page = centre + Vec3::new(side * 0.22, -0.3, 0.012);
            let size = Vec3::new(0.4, 0.26, 0.012);
            parts.push(m.boxed(page, size, side * 0.2, kind::CLOTH, rgba(CREAM, 1.0)));
        }
        let tassel = tassel(self);
        for side in [-1.0f32, 1.0] {
            let head = m.at(Vec3::new(side * (w * 0.5 - 0.1), mid - h * 0.5, 0.09));
            parts.push(tassel.at(head, Quat::IDENTITY, kind::CLOTH, rgba(GILT, 1.0)));
        }
        // The rod, with finials, it hangs from.
        let rod_at = Vec3::new(0.0, mid + h * 0.5 + 0.06, 0.09);
        let ends = (
            m.at(rod_at - Vec3::X * (w * 0.5 + 0.22)),
            m.at(rod_at + Vec3::X * (w * 0.5 + 0.22)),
        );
        let i = self.pole_inst(ends.0, ends.1, 0.06, kind::WOOD, rgba(WALNUT, 1.0));
        self.decor(i);
        let finial = finial(self);
        for (end, out) in [(ends.0, -m.right()), (ends.1, m.right())] {
            let turn = Quat::from_rotation_arc(Vec3::Y, out);
            self.decor(finial.at(end, turn, kind::METAL, rgba(GILT, 1.0)));
        }
        let mut r = StdRng::seed_from_u64(0x7a9f ^ (s > 0.0) as u64);
        let sway = (0.0045, r.gen_range(7.5..9.0), r.gen_range(80.0..110.0));
        let mover = stir(self, m.at(rod_at), m.right(), sway, r.gen_range(0.0..100.0));
        for p in parts {
            self.moving(mover, p);
        }
    }

    /// A moonlit landscape in a gilt frame, `size` (width, height, centre's height):
    /// hills against the night, a lake with the moon in it, trees on the near shore.
    fn painting(&mut self, m: Mount, size: Vec3, seed: u64) {
        let (w, h, y) = (size.x, size.y, size.z);
        let mut r = StdRng::seed_from_u64(0x9a17 ^ seed);
        let gilt = rgba(GILT, 1.0);
        for s in [-1.0f32, 1.0] {
            let rail = Vec3::new(0.0, y + s * (h * 0.5 + 0.06), 0.04);
            self.mounted(m, rail, Vec3::new(w + 0.24, 0.12, 0.08), 0.0, kind::METAL, gilt);
            let stile = Vec3::new(s * (w * 0.5 + 0.06), y, 0.04);
            self.mounted(m, stile, Vec3::new(0.12, h, 0.08), 0.0, kind::METAL, gilt);
        }
        // Each layer a little in front of the one it overlaps.
        let layer = |lib: &mut Self, (y0, y1): (f32, f32), z: f32, rgb: [f32; 3]| {
            let off = Vec3::new(0.0, y + h * (y0 + y1) * 0.5, z);
            lib.mounted(
                m,
                off,
                Vec3::new(w, h * (y1 - y0), 0.004),
                0.0,
                kind::PLAIN,
                rgba(rgb, 1.0),
            );
        };
        layer(self, (-0.5, 0.5), 0.01, [0.06, 0.08, 0.2]);
        layer(self, (-0.12, 0.08), 0.016, [0.1, 0.12, 0.25]);
        // Hills: lozenges behind the lake, so only their peaks show.
        for k in 0..3 {
            let sz = r.gen_range(0.35..0.6f32);
            let x = r.gen_range(-w * 0.5 + sz * 0.75..w * 0.5 - sz * 0.75);
            let c = [0.035 + 0.015 * k as f32, 0.055 + 0.015 * k as f32, 0.08];
            let off = Vec3::new(x, y - h * 0.12, 0.022 + 0.002 * k as f32);
            self.mounted(m, off, Vec3::new(sz, sz, 0.002), FRAC_PI_4, kind::PLAIN, rgba(c, 1.0));
        }
        layer(self, (-0.3, -0.12), 0.03, [0.04, 0.07, 0.17]);
        layer(self, (-0.5, -0.3), 0.036, [0.025, 0.04, 0.035]);
        let mx = r.gen_range(-w * 0.35..w * 0.35);
        let moon = Vec3::new(mx, y + h * r.gen_range(0.18..0.3), 0.02);
        self.disc(m, moon, (0.1, 0.004), kind::PLAIN, rgba([0.95, 0.9, 0.7], 1.0));
        // Its path on the water, and trees on the shore.
        let glint = rgba([0.55, 0.55, 0.45], 1.0);
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
            let tree = rgba([0.02, 0.035, 0.03], 1.0);
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
            rgba([0.62, 0.5, 0.32], 1.0),
        );
        let (sea_c, sea_half) = (Vec2::new(0.0, y - 0.05), Vec2::new(w * 0.5 - 0.2, h * 0.5 - 0.25));
        let sea = Vec3::new(sea_c.x, sea_c.y, 0.038);
        let sea_size = Vec3::new(sea_half.x * 2.0, sea_half.y * 2.0, 0.006);
        self.mounted(m, sea, sea_size, 0.0, kind::PARCHMENT, rgba([0.3, 0.38, 0.36], 1.0));
        // Rhumb lines out from the rose to the edges of the sea.
        let rose = Vec2::new(1.2, y - 0.85);
        let ink = rgba([0.17, 0.2, 0.18], 1.0);
        for k in 0..16 {
            let a = k as f32 * TAU / 16.0 + 0.02;
            let d = Vec2::new(libm::cosf(a), libm::sinf(a));
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
                    rgba([0.36, 0.27, 0.15], 1.0),
                );
            }
            for (j, &(at, size, spin)) in patches.iter().enumerate() {
                let z = 0.047 + 0.0005 * j as f32;
                let tint = [0.62 - 0.04 * k as f32, 0.52 - 0.01 * j as f32, 0.31];
                let patch = Vec3::new(size.x, size.y, 0.0004);
                self.mounted(
                    m,
                    Vec3::new(at.x, at.y, z),
                    patch,
                    spin,
                    kind::PARCHMENT,
                    rgba(tint, 1.0),
                );
            }
        }
        // The compass rose: long points north-south and east-west, short ones between.
        let rose = Vec3::new(rose.x, rose.y, 0.052);
        let gilt = rgba(GILT, 1.0);
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
            rgba([0.4, 0.3, 0.16], 1.0),
        );
        let label = cart + Vec3::Z * 0.004;
        self.mounted(
            m,
            label,
            Vec3::new(0.8, 0.26, 0.004),
            0.0,
            kind::PARCHMENT,
            rgba([0.7, 0.6, 0.4], 1.0),
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
        // The rollers, with gilt finials, and a cord up to a hook.
        let roller = rgba(DARK_WOOD, 1.0);
        let finial = finial(self);
        for ry in [y + h * 0.5 + 0.03, y - h * 0.5 - 0.03] {
            let half = Vec3::X * (w * 0.5 + 0.15);
            let at = Vec3::new(0.0, ry, 0.06);
            let ends = (m.at(at - half), m.at(at + half));
            let i = self.pole_inst(ends.0, ends.1, 0.1, kind::WOOD, roller);
            self.decor(i);
            for (end, out) in [(ends.0, -m.right()), (ends.1, m.right())] {
                let turn = Quat::from_rotation_arc(Vec3::Y, out);
                self.decor(finial.scaled(1.3).at(end, turn, kind::METAL, gilt));
            }
        }
        let hook = m.at(Vec3::new(0.0, y + h * 0.5 + 0.75, 0.05));
        for s in [-1.0f32, 1.0] {
            let foot = m.at(Vec3::new(s * (w * 0.5 - 0.1), y + h * 0.5 + 0.06, 0.05));
            let i = self.pole_inst(foot, hook, 0.018, kind::CLOTH, rgba([0.35, 0.25, 0.1], 1.0));
            self.decor(i);
        }
        let knob = self.shape(Mesh::sphere(16)).sized(Vec3::splat(0.07));
        self.decor(knob.inst(hook, Quat::IDENTITY, kind::METAL, gilt));
        self.map_wakes(m, Vec2::new(rose.x, rose.y), y);
    }

    /// A great clock face on a walnut board, at ten to midnight as the library opens.
    fn clock(&mut self, m: Mount) {
        let y = 4.2;
        let centre = Vec3::new(0.0, y, 0.0);
        let board = Vec3::new(3.0, 3.0, 0.08);
        self.mounted(m, centre + Vec3::Z * 0.04, board, 0.0, kind::WOOD, rgba(WALNUT, 1.0));
        let gilt = rgba(GILT, 1.0);
        for (sx, sy) in [(-1.0f32, -1.0f32), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)] {
            let corner = centre + Vec3::new(sx * 1.2, sy * 1.2, 0.085);
            self.mounted(m, corner, Vec3::new(0.32, 0.32, 0.01), FRAC_PI_4, kind::METAL, gilt);
        }
        self.disc(m, centre + Vec3::Z * 0.095, (1.2, 0.03), kind::PLAIN, rgba(CREAM, 1.0));
        let face = m.at(centre + Vec3::Z * 0.12);
        self.arc(
            face,
            (m.right(), Vec3::Y),
            (1.27, 0.12, 0.08),
            (0.0, TAU, 24),
            (kind::METAL, gilt),
        );
        let ink = rgba(IRON, 1.0);
        for k in 0..12 {
            let a = FRAC_PI_2 - k as f32 * TAU / 12.0;
            let (len, wid) = if k % 3 == 0 { (0.24, 0.07) } else { (0.15, 0.04) };
            let at = centre + Vec3::new(libm::cosf(a), libm::sinf(a), 0.0) * 1.0 + Vec3::Z * 0.12;
            self.mounted(m, at, Vec3::new(len, wid, 0.01), a, kind::METAL, ink);
        }
        // Hands at ten to midnight when the library opens, and keeping time from there.
        self.clock_hands(
            (m.at(centre + Vec3::Z * 0.11), m.n, 1.25),
            (11.0, 50.0),
            (ink, gilt),
            false,
        );
        self.moon_dial(m, y + board.y * 0.5);
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
            lib.mounted(
                m,
                off,
                Vec3::new(0.08, wh - sk, 0.02),
                0.0,
                kind::WOOD,
                rgba(WALNUT, 1.0),
            );
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
        let mut rug = Inst::aabox(
            Vec3::new(0.0, 0.012, 0.0),
            Vec3::new(RUG_HALF.x * 2.0, 0.024, RUG_HALF.y * 2.0),
            kind::RUG,
            RUG_RED,
        );
        rug.param = RUG_BORDER;
        self.rug(rug);

        // Reading tables with chairs and candles, and a green-shaded lamp on each.
        for (tx, tz) in [(-4.5f32, -2.6f32), (4.5, -2.6), (-4.5, 2.6), (4.5, 2.6)] {
            self.reading_table(Vec3::new(tx, 0.0, tz));
            self.desk_lamp(Vec3::new(tx + 0.25, TABLE_H, tz - tz.signum() * 0.38));
        }

        // Chandelier, from the boss where the middle rib crosses the middle beam.
        self.chandelier(CHANDELIER, BEAM_Y - 0.07);

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
        hearth_fire(self, fc + Vec3::new(0.0, 0.0, -0.75));
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
        let brass = rgba(BRASS, 1.0);
        let iron = rgba(IRON, 1.0);
        let metal =
            |lib: &mut Self, min: Vec3, max: Vec3, rgb| lib.decor(Inst::minmax(fc + min, fc + max, kind::METAL, rgb));
        metal(self, Vec3::new(-2.2, 0.08, -1.56), Vec3::new(2.2, 0.2, -1.5), BRASS);
        let i = self.pole_inst(
            fc + Vec3::new(-2.2, 0.215, -1.53),
            fc + Vec3::new(2.2, 0.215, -1.53),
            0.035,
            kind::METAL,
            brass,
        );
        self.decor(i);
        let knob = self.turned(
            &[
                (0.0, 0.0),
                (0.02, 0.0),
                (0.016, 0.012),
                (0.04, 0.04),
                (0.036, 0.062),
                (0.012, 0.078),
                (0.0, 0.082),
            ],
            16,
        );
        for s in [-1.0f32, 1.0] {
            let (a, b) = (s * 2.17, s * 2.23);
            metal(
                self,
                Vec3::new(a.min(b), 0.08, -1.5),
                Vec3::new(a.max(b), 0.2, -0.9),
                BRASS,
            );
            // Firedogs under the logs: an upright with a brass knob, and a bar back
            // under the fire.
            let x = s * 0.55;
            let foot = fc + Vec3::new(x, 0.0, -1.05);
            let i = self.pole_inst(foot + Vec3::Y * 0.08, foot + Vec3::Y * 0.34, 0.05, kind::METAL, iron);
            self.decor(i);
            self.decor(knob.at(foot + Vec3::Y * 0.33, Quat::IDENTITY, kind::METAL, brass));
            let bar = (foot + Vec3::new(0.0, 0.08, 0.03), foot + Vec3::new(0.0, 0.08, 0.6));
            let i = self.pole_inst(bar.0, bar.1, 0.035, kind::METAL, iron);
            self.decor(i);
        }
        // Fire irons on a stand by the right jamb, on the hearthstone: a poker, a
        // shovel and a brush, hanging by their brass handles.
        let stand = fc + Vec3::new(2.32, 0.08, -1.3);
        let base = self.turned(
            &[(0.0, 0.0), (0.08, 0.0), (0.08, 0.012), (0.05, 0.025), (0.0, 0.025)],
            16,
        );
        self.decor(base.at(stand, Quat::IDENTITY, kind::METAL, iron));
        let i = self.pole_inst(stand, stand + Vec3::Y * 0.8, 0.03, kind::METAL, iron);
        self.decor(i);
        let top = stand + Vec3::Y * 0.78;
        let handle = self.turned(
            &[
                (0.0, 0.0),
                (0.012, 0.0),
                (0.02, 0.03),
                (0.016, 0.07),
                (0.022, 0.085),
                (0.0, 0.1),
            ],
            12,
        );
        for (k, lean) in [-0.14f32, 0.0, 0.14].into_iter().enumerate() {
            let rot = Quat::from_rotation_x(lean);
            let i = self.pole_inst(top + rot * Vec3::Y * -0.62, top, 0.018, kind::METAL, iron);
            self.decor(i);
            self.decor(handle.at(top, rot, kind::METAL, brass));
            let end = top + rot * Vec3::Y * -0.62;
            let head = match k {
                0 => Inst::new(
                    end + rot * Vec3::new(0.0, 0.02, 0.03),
                    rot,
                    Vec3::new(0.016, 0.016, 0.07),
                    kind::METAL,
                    iron,
                    0,
                ),
                1 => Inst::new(end, rot, Vec3::new(0.11, 0.13, 0.008), kind::METAL, iron, 0),
                _ => self.shape(Mesh::cylinder(12)).sized(Vec3::new(0.06, 0.1, 0.06)).inst(
                    end,
                    rot,
                    kind::CLOTH,
                    rgba([0.12, 0.08, 0.05], 1.0),
                ),
            };
            self.decor(head);
        }
        // On the mantel: candlesticks under glass chimneys at its ends, and a clock
        // between.
        let shelf = fc + Vec3::new(0.0, 2.85, -0.85);
        let stick = self.turned(
            &[
                (0.0, 0.0),
                (0.068, 0.0),
                (0.068, 0.01),
                (0.055, 0.022),
                (0.03, 0.032),
                (0.015, 0.05),
                (0.013, 0.11),
                (0.024, 0.13),
                (0.013, 0.15),
                (0.011, 0.27),
                (0.02, 0.29),
                (0.05, 0.305),
                (0.05, 0.315),
                (0.024, 0.32),
                (0.024, 0.345),
                (0.0, 0.345),
            ],
            24,
        );
        let candle = self.shape(Mesh::cylinder(16)).sized(Vec3::new(0.04, 0.2, 0.04));
        let chimney = Mesh::lathe(
            &[(0.044, 0.0), (0.058, 0.06), (0.066, 0.13), (0.06, 0.2), (0.047, 0.28)].map(Vec2::from),
            24,
        );
        let chimney = self.shape(chimney);
        for s in [-1.0f32, 1.0] {
            let p = shelf + Vec3::X * s * 2.1;
            self.decor(stick.at(p, Quat::IDENTITY, kind::METAL, brass));
            self.decor(candle.inst(
                p + Vec3::Y * 0.42,
                Quat::IDENTITY,
                kind::PLAIN,
                rgba([0.8, 0.75, 0.6], 1.0),
            ));
            self.translucent(Inst::new(
                p + Vec3::Y * 0.555,
                Quat::IDENTITY,
                Vec3::new(0.026, 0.06, 0.026),
                kind::FIRE,
                [1.0, 0.6, 0.22, 3.0],
                0,
            ));
            self.translucent(chimney.at(p + Vec3::Y * 0.315, Quat::IDENTITY, kind::CLEAR, [0.9, 0.95, 1.0, 0.08]));
        }
        mantel_clock(self, shelf);
        // The emblem over the mantel: a crescent moon over an open book, gilt on a
        // blue field in a stone frame.
        let m = Mount {
            at: fc + Vec3::new(0.0, 0.0, -0.6),
            n: -Vec3::Z,
        };
        let y = 4.5;
        let field = rgba(SKY, 1.0);
        self.mounted(
            m,
            Vec3::new(0.0, y, 0.03),
            Vec3::new(1.6, 1.4, 0.06),
            0.0,
            kind::PLAIN,
            field,
        );
        let dressed = rgba(DRESSED, 1.0);
        for s in [-1.0f32, 1.0] {
            let rail = Vec3::new(0.0, y + s * 0.8, 0.06);
            self.mounted(m, rail, Vec3::new(2.0, 0.2, 0.12), 0.0, kind::STONE, dressed);
            let stile = Vec3::new(s * 0.9, y, 0.06);
            self.mounted(m, stile, Vec3::new(0.2, 1.4, 0.12), 0.0, kind::STONE, dressed);
        }
        let gilt = rgba(GILT, 1.0);
        let moon = Vec3::new(0.0, y + 0.25, 0.065);
        self.disc(m, moon, (0.26, 0.01), kind::METAL, gilt);
        self.disc(m, moon + Vec3::new(0.11, 0.07, 0.005), (0.23, 0.01), kind::PLAIN, field);
        // Once the whole Hall is in order the emblem kindles, after the stars have
        // gathered: each gilt piece gets a glowing one just before it, growing from its
        // middle (the crescent's, as the gilt one's, cut by the field's disc before it).
        let glow = [1.0, 0.7, 0.3, 0.9];
        let mut kindles = vec![self.disc_inst(m, moon + Vec3::Z * 0.0065, (0.26, 0.002), kind::CRYSTAL, glow)];
        for s in [-1.0f32, 1.0] {
            let page = Vec3::new(s * 0.25, y - 0.3, 0.068);
            self.mounted(m, page, Vec3::new(0.46, 0.3, 0.01), s * 0.18, kind::METAL, gilt);
            let lit = page + Vec3::Z * 0.0065;
            kindles.push(m.boxed(lit, Vec3::new(0.46, 0.3, 0.002), s * 0.18, kind::CRYSTAL, glow));
        }
        for (sx, sy) in [(-0.5f32, 0.35f32), (0.5, 0.45), (-0.4, 0.05), (0.45, 0.0)] {
            let star = Vec3::new(sx, y + sy, 0.065);
            self.mounted(m, star, Vec3::new(0.07, 0.07, 0.01), FRAC_PI_4, kind::METAL, gilt);
            let lit = star + Vec3::Z * 0.0065;
            kindles.push(m.boxed(lit, Vec3::new(0.07, 0.07, 0.002), FRAC_PI_4, kind::CRYSTAL, glow));
        }
        for (k, i) in kindles.into_iter().enumerate() {
            let grows = Change::Arrives {
                from: Vec3::ZERO,
                grow: 0.0,
            };
            let a = Awaits::new(Finish::Library, grows).after(FINALE + 5.0 + k as f32 * 0.25);
            self.waiting(Vec3::from(i.pos), a.over(2.0), [i]);
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

    /// A banker's lamp standing on a table at `p`: a brass stem on a stepped foot
    /// under a half-round shade of green cased glass, white inside, lit from under it.
    fn desk_lamp(&mut self, p: Vec3) {
        let brass = rgba(BRASS, 1.0);
        let foot = self.turned(
            &[
                (0.0, 0.0),
                (0.11, 0.0),
                (0.11, 0.008),
                (0.1, 0.016),
                (0.07, 0.024),
                (0.035, 0.03),
                (0.02, 0.04),
                (0.0, 0.04),
            ],
            32,
        );
        self.decor(
            foot.sized(Vec3::new(0.22, 0.04, 0.15))
                .at(p, Quat::IDENTITY, kind::METAL, brass),
        );
        let i = self.pole_inst(p + Vec3::Y * 0.035, p + Vec3::Y * 0.335, 0.022, kind::METAL, brass);
        self.decor(i);
        let knop = self.shape(Mesh::sphere(12)).sized(Vec3::new(0.036, 0.03, 0.036));
        self.decor(knop.inst(p + Vec3::Y * 0.16, Quat::IDENTITY, kind::METAL, brass));
        // The shade, along x over the stem, open below with its ends closed.
        let axis = p + Vec3::Y * 0.35;
        let along = Quat::from_rotation_y(FRAC_PI_2);
        let half_round = |r: f32| -> Vec<Vec2> {
            (0..=16)
                .map(|k| Vec2::new(libm::cosf(PI * k as f32 / 16.0), libm::sinf(PI * k as f32 / 16.0)) * r)
                .collect()
        };
        let band = |outer: f32, inner: f32, len: f32| {
            let mut o = half_round(outer);
            o.extend(half_round(inner).into_iter().rev());
            Mesh::prism(&o, len)
        };
        let mut glass = band(0.085, 0.079, 0.4);
        for z in [-0.196, 0.196] {
            glass.add(
                &Mesh::prism(&half_round(0.08), 0.006),
                Vec3::Z * z,
                Quat::IDENTITY,
                Vec3::ONE,
            );
        }
        let glass = self.shape(glass);
        self.decor(glass.at(axis, along, kind::CRYSTAL, [0.02, 0.17, 0.065, 0.3]));
        let lining = self.shape(band(0.0785, 0.0765, 0.388));
        self.decor(lining.at(axis, along, kind::PAPER, [1.0, 0.86, 0.6, 0.9]));
        let collar = self.shape(Mesh::cylinder(16)).sized(Vec3::new(0.05, 0.03, 0.05));
        self.decor(collar.inst(axis - Vec3::Y * 0.005, Quat::IDENTITY, kind::METAL, brass));
        let tube = self.shape(Mesh::cylinder(12)).sized(Vec3::new(0.028, 0.26, 0.028));
        let lit = [1.0, 0.82, 0.55, 4.0];
        self.decor(tube.inst(
            axis + Vec3::Y * 0.035,
            along * Quat::from_rotation_x(FRAC_PI_2),
            kind::EMISSIVE,
            lit,
        ));
        self.light(p + Vec3::Y * 0.25, 3.2, [1.0, 0.82, 0.55], 0.7, LightKind::Static);
    }

    /// An armillary sphere on a turned stand at `c`: horizon, meridian, equator and
    /// ecliptic rings round a small globe on its polar axis; the heavens (the equator,
    /// the ecliptic and the globe) turn slowly about the pole.
    fn armillary(&mut self, c: Vec3) {
        // A round foot, a step, a turned baluster and a brass cup.
        let foot = self.turned(&[(0.0, 0.0), (0.35, 0.0), (0.35, 0.05), (0.33, 0.08), (0.0, 0.08)], 48);
        self.decor(foot.at(c, Quat::IDENTITY, kind::WOOD, rgba(WALNUT, 1.0)));
        let step = self.turned(&[(0.0, 0.0), (0.21, 0.0), (0.21, 0.055), (0.18, 0.08), (0.0, 0.08)], 32);
        self.decor(step.at(c + Vec3::Y * 0.08, Quat::IDENTITY, kind::WOOD, rgba(DARK_WOOD, 1.0)));
        let baluster = self.turned(
            &[
                (0.0, 0.0),
                (0.075, 0.0),
                (0.075, 0.03),
                (0.055, 0.05),
                (0.075, 0.12),
                (0.095, 0.22),
                (0.09, 0.3),
                (0.06, 0.38),
                (0.042, 0.5),
                (0.06, 0.56),
                (0.06, 0.6),
                (0.045, 0.62),
                (0.045, 0.65),
                (0.0, 0.65),
            ],
            24,
        );
        self.decor(baluster.at(c + Vec3::Y * 0.16, Quat::IDENTITY, kind::WOOD, rgba(WALNUT, 1.0)));
        let cup = self.turned(&[(0.0, 0.0), (0.05, 0.0), (0.11, 0.045), (0.12, 0.06), (0.0, 0.06)], 24);
        self.decor(cup.at(c + Vec3::Y * 0.81, Quat::IDENTITY, kind::METAL, rgba(BRASS, 1.0)));
        let o = c + Vec3::Y * 1.32;
        let r = 0.42;
        let brass = rgba(BRASS, 1.0);
        // The horizon, a wooden ring on four legs.
        self.arc(
            o,
            (Vec3::X, Vec3::Z),
            (0.52, 0.09, 0.035),
            (0.0, TAU, 20),
            (kind::WOOD, rgba(WALNUT, 1.0)),
        );
        for k in 0..4 {
            let a = FRAC_PI_4 + k as f32 * FRAC_PI_2;
            let d = Vec3::new(libm::cosf(a), 0.0, libm::sinf(a));
            let leg = (c + d * 0.3 + Vec3::Y * 0.08, o + d * 0.52 - Vec3::Y * 0.02);
            let i = self.pole_inst(leg.0, leg.1, 0.035, kind::WOOD, rgba(WALNUT, 1.0));
            self.decor(i);
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
        let pole = Vec3::Y * libm::sinf(lat) - along * libm::cosf(lat);
        let across = pole.cross(x).normalize();
        let heavens = self.mover(o, Motion::Spin { axis: pole, rate: 0.12 }, 0.0, None);
        let equator = self.arc_inst(
            o,
            (x, across),
            (r - 0.04, 0.04, 0.025),
            (0.0, TAU, 20),
            (kind::METAL, brass),
        );
        self.moving(heavens, equator);
        let tilt = Quat::from_axis_angle(x, 0.41);
        let gilt = rgba(GILT, 1.0);
        let ecliptic = self.arc_inst(
            o,
            (x, tilt * across),
            (r - 0.04, 0.07, 0.02),
            (0.0, TAU, 20),
            (kind::METAL, gilt),
        );
        self.moving(heavens, ecliptic);
        let axis = Quat::from_rotation_arc(Vec3::Y, pole);
        let earth = [0.1, 0.24, 0.32, 1.0];
        let globe = self.shape(Mesh::sphere(24)).sized(Vec3::splat(0.16));
        self.moving(heavens, globe.inst(o, axis, kind::PLAIN, earth));
        let rod = self
            .shape(Mesh::cylinder(8))
            .sized(Vec3::new(0.02, 2.0 * r + 0.16, 0.02));
        self.decor(rod.inst(o, axis, kind::METAL, brass));
        self.collide(
            c - Vec3::new(0.62, 0.0, 0.62),
            c + Vec3::new(0.62, 1.86, 0.62),
            Stops::RAIL,
        );
    }

    /// A lectern at `c` facing south: a turned walnut column on a round stone foot,
    /// and on it a great book open, its writing glowing faintly, a leaf of it turning
    /// over and back now and then and motes of light floating over its pages: the
    /// reading room's one enchantment within reach.
    fn lectern(&mut self, c: Vec3) {
        let foot = self.turned(
            &[
                (0.0, 0.0),
                (0.3, 0.0),
                (0.3, 0.07),
                (0.27, 0.1),
                (0.19, 0.1),
                (0.19, 0.17),
                (0.16, 0.2),
                (0.0, 0.2),
            ],
            40,
        );
        self.decor(foot.at(c, Quat::IDENTITY, kind::STONE, rgba(DRESSED, 1.0)));
        let column = self.turned(
            &[
                (0.0, 0.0),
                (0.1, 0.0),
                (0.1, 0.035),
                (0.08, 0.05),
                (0.088, 0.11),
                (0.078, 0.18),
                (0.05, 0.26),
                (0.045, 0.32),
                (0.072, 0.37),
                (0.072, 0.4),
                (0.045, 0.45),
                (0.04, 0.6),
                (0.07, 0.64),
                (0.07, 0.67),
                (0.05, 0.69),
                (0.11, 0.74),
                (0.11, 0.76),
                (0.0, 0.76),
            ],
            24,
        );
        let walnut = rgba(WALNUT, 1.0);
        self.decor(column.at(c + Vec3::Y * 0.2, Quat::IDENTITY, kind::WOOD, walnut));
        let desk_at = c + Vec3::Y * 1.05;
        let desk = Quat::from_rotation_x(0.38);
        let on_desk = |p: Vec3| desk_at + desk * p;
        let bracket = Vec3::new(0.22, 0.06, 0.3);
        self.decor(Inst::new(
            on_desk(Vec3::Y * -0.05),
            desk,
            bracket,
            kind::WOOD,
            walnut,
            0,
        ));
        let board = Vec3::new(0.8, 0.04, 0.56);
        self.decor(Inst::new(desk_at, desk, board, kind::WOOD, walnut, 0));
        let lip = Vec3::new(0.8, 0.06, 0.03);
        self.decor(Inst::new(
            on_desk(Vec3::new(0.0, 0.04, 0.27)),
            desk,
            lip,
            kind::WOOD,
            walnut,
            0,
        ));
        // The book: its boards, and the pages on them falling away from the gutter to
        // their fore-edges (placed by `top`, on the boards in the book's frame).
        let book = Vec3::new(0.0, 0.0, -0.02);
        let cover = Vec3::new(0.74, 0.02, 0.5);
        let boards = [0.26, 0.05, 0.04, 1.0];
        self.decor(Inst::new(
            on_desk(book + Vec3::Y * 0.03),
            desk,
            cover,
            kind::PLAIN,
            boards,
            0,
        ));
        let top = |p: Vec3| on_desk(book + Vec3::Y * 0.04 + p);
        let slope = 0.0095 / 0.265;
        let surface = |x: f32| 0.049 - (x.abs() - 0.07) * slope;
        let page = [
            (0.355, 0.032),
            (0.335, 0.0395),
            (0.07, 0.049),
            (0.035, 0.046),
            (0.015, 0.038),
        ];
        let mut block = vec![Vec2::new(-0.355, 0.0), Vec2::new(0.355, 0.0)];
        block.extend(page.iter().map(|&(x, y)| Vec2::new(x, y)));
        block.push(Vec2::new(0.0, 0.03));
        block.extend(page.iter().rev().map(|&(x, y)| Vec2::new(-x, y)));
        let block = self.shape(Mesh::prism(&block, 0.46));
        let paper = [0.72, 0.66, 0.5, 1.0];
        self.decor(block.at(top(Vec3::ZERO), desk, kind::PARCHMENT, paper));
        // Its writing glows.
        for s in [-1.0f32, 1.0] {
            let lie = desk * Quat::from_rotation_z(-s * libm::atanf(slope));
            for line in 0..7 {
                let len = if line % 3 == 2 { 0.15 } else { 0.22 };
                let x = s * (0.2 - (0.22 - len) * 0.5);
                let at = top(Vec3::new(x, surface(x) + 0.001, -0.17 + line as f32 * 0.055));
                let glow = [0.3, 0.55, 1.0, 1.6];
                self.decor(Inst::new(at, lie, Vec3::new(len, 0.003, 0.012), kind::CRYSTAL, glow, 0));
            }
        }
        // A ribbon down the gutter and over the end of the pages.
        let red = [0.5, 0.06, 0.06, 1.0];
        for (a, b) in [
            (Vec3::new(0.0, 0.032, 0.06), Vec3::new(0.0, 0.032, 0.23)),
            (Vec3::new(0.0, 0.032, 0.23), Vec3::new(0.0, -0.018, 0.255)),
        ] {
            let d = b - a;
            let lie = desk * Quat::from_rotation_x(libm::atan2f(-d.y, d.z));
            let size = Vec3::new(0.025, 0.003, d.length() + 0.004);
            self.decor(Inst::new(top((a + b) * 0.5), lie, size, kind::CLOTH, red, 0));
        }
        // A leaf turning over from one side to the other and back, resting a while
        // between, sunk a hair into the pages: a swing, and one of a ninth of its reach
        // at three times its pace, whose sum dwells at its ends (sin x + sin 3x / 9 is
        // flat to the fourth order at its peak, 8/9).
        let spine = top(Vec3::Y * 0.047);
        let (reach, period) = (1.71 * 9.0 / 8.0, 34.0);
        let axis = desk * Vec3::Z;
        let swing = |amp: f32, period: f32| Motion::Swing { axis, amp, period };
        let turn = self.mover(spine, swing(reach, period), 6.0, None);
        let turn = self.mover(spine, swing(reach / 9.0, period / 3.0), 6.0, Some(turn));
        let leaf = Vec3::new(0.33, 0.002, 0.44);
        let upright = desk * Quat::from_rotation_z(FRAC_PI_2);
        let at = top(Vec3::Y * (0.047 + 0.165));
        self.moving(turn, Inst::new(at, upright, leaf, kind::PARCHMENT, paper, 0));
        // Motes of light over it, each bobbing in its own time, the lot turning slowly.
        let over = on_desk(book) + Vec3::Y * 0.05;
        let cloud = self.mover(
            over,
            Motion::Spin {
                axis: Vec3::Y,
                rate: 0.15,
            },
            0.0,
            None,
        );
        let mut r = StdRng::seed_from_u64(0x1ec7);
        for k in 0..11 {
            let up = 0.1 + k as f32 * 0.07;
            let a = k as f32 * 2.4;
            let p = over + Vec3::new(libm::cosf(a) * 0.14, up, libm::sinf(a) * 0.1);
            let s = r.gen_range(0.012..0.022);
            let rot = Quat::from_rotation_y(a) * Quat::from_rotation_x(FRAC_PI_4);
            let glow = r.gen_range(1.5..3.0) * (1.0 - k as f32 * 0.06);
            let bob = Motion::Bob {
                amp: r.gen_range(0.03..0.06),
                period: r.gen_range(5.0..9.0),
                rock: 0.6,
            };
            let mote = self.mover(p, bob, r.gen_range(0.0..9.0), Some(cloud));
            let i = Inst::new(p, rot, Vec3::splat(s), kind::CRYSTAL, [0.55, 0.75, 1.0, glow], 0);
            self.moving(mote, i);
        }
        self.light(over + Vec3::Y * 0.8, 3.0, [0.55, 0.72, 1.0], 0.5, LightKind::Flicker);
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
                .filter(|&b| self.dept_of_book(b) == wing)
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

/// What the Hall does when a category, a wing or the whole of it is put in order
/// (`celebrate`): the West Wing ends at a map and the East at a clock, and the reading
/// room between them is the night sky.
impl Library {
    /// When a category is in order, a gilt crescent (the library's emblem) on a turned
    /// finial rises out of the cornice at either end of its run, on its side: over the
    /// stiles there, which hide it until then.
    fn crests(&mut self) {
        let knop = self.turned(
            &[
                (0.0, 0.0),
                (0.045, 0.0),
                (0.045, 0.012),
                (0.032, 0.024),
                (0.04, 0.05),
                (0.028, 0.075),
                (0.011, 0.09),
                (0.011, 0.15),
                (0.0, 0.15),
            ],
            16,
        );
        let ball = self.shape(Mesh::sphere(12)).sized(Vec3::splat(0.045));
        let moon = self.shape(Mesh::prism(&crescent(0.085, 0.068, 0.04, 14), 0.012));
        let gilt = rgba(GILT, 1.0);
        for cat in 0..self.catalog.categories.len() {
            let faces: Vec<usize> = self.faces_of(cat).collect();
            let (Some(&first), Some(&last)) = (faces.first(), faces.last()) else {
                continue;
            };
            let fin = Finish::Category(cat);
            for (f, end) in [(first, -1.0f32), (last, 1.0)] {
                let face = &self.faces[f];
                // Over the end stile's middle, just behind the face, on the crown.
                let along = face.right * (end * (UNIT_W * 0.5 - 0.05));
                let foot = face.center + along - face.normal * 0.03 + Vec3::Y * (UNIT_H + 0.13);
                let horns_up = face.rot() * Quat::from_rotation_z(FRAC_PI_2);
                let parts = [
                    knop.at(foot, Quat::IDENTITY, kind::METAL, gilt),
                    ball.inst(foot + Vec3::Y * 0.165, Quat::IDENTITY, kind::METAL, gilt),
                    moon.at(foot + Vec3::Y * 0.27, horns_up, kind::METAL, gilt),
                ];
                let rises = Change::Arrives {
                    from: Vec3::Y * -0.32,
                    grow: 0.5,
                };
                self.waiting(foot, Awaits::new(fin, rises).after(1.5).over(2.2), parts);
                self.sparks(Sparks {
                    delay: 2.6,
                    lasts: Some(0.5),
                    rate: 40.0,
                    spread: Vec3::splat(0.06),
                    ..Sparks::burst(fin, foot + Vec3::Y * 0.3)
                });
            }
        }
    }

    /// A runner down wing `wing`'s middle aisle once it is in order, from the reading
    /// room's rug to the wall at its end: rugs like that one settling one after another
    /// outward, each with a breath of gold as it lands. Flat on the floor, under any
    /// books lying there.
    fn runner(&mut self, wing: usize) {
        let fin = Finish::Room(wing);
        let s = if wing == 0 { -1.0f32 } else { 1.0 };
        // From just off the rug's edge to the skirting.
        let (near, far) = (RUG_HALF.x + 0.02, HALL_X - 0.1);
        let (len, wide, thick) = ((far - near) / RUNNER_RUGS as f32, 2.0, 0.003);
        for k in 0..RUNNER_RUGS {
            let at = Vec3::new(s * (near + len * (k as f32 + 0.5)), thick * 0.5, 0.0);
            let delay = RUNNER_AFTER + k as f32 * RUG_STAGGER;
            // Each floats down from the reading room's side, turning a little as it
            // settles.
            let falls = Change::Arrives {
                from: Vec3::new(-s * 0.6, 0.5, 0.0),
                grow: 0.4,
            };
            let twist = if k % 2 == 0 { 0.1 } else { -0.08 };
            let settles = Change::Shifts {
                by: Vec3::ZERO,
                turn: Quat::from_rotation_y(twist),
                scale: 1.0,
            };
            let m = self.mover(at, Motion::Still, 0.0, None);
            self.awaiting(m, Awaits::new(fin, falls).after(delay).over(RUG_SETTLES));
            let m = self.mover(at, Motion::Still, 0.0, Some(m));
            self.awaiting(m, Awaits::new(fin, settles).after(delay).over(RUG_SETTLES));
            let (size, lie) = (Vec3::new(len, thick, wide), Quat::from_rotation_y(-twist));
            let rug = Inst::new(at, lie, size, kind::RUG, rgba(RUG_RED, 1.0), RUG_BORDER);
            self.moving(m, rug);
            self.sparks(Sparks {
                delay: delay + RUG_SETTLES * 0.8,
                lasts: Some(0.35),
                spread: Vec3::new(len * 0.5, 0.0, wide * 0.5),
                rate: 50.0,
                vel: Vec3::Y * 0.3,
                jitter: Vec3::new(0.25, 0.15, 0.25),
                life: 1.3,
                size: Vec3::splat(0.014),
                ..Sparks::burst(fin, at + Vec3::Y * 0.02)
            });
        }
    }

    /// When the West Wing is in order the map at its end wakes, once its runner is
    /// down: the compass rose kindles, turning into place as it grows, and a model ship
    /// rises out of the sea and sails across it from then on, resting at either end
    /// and turning about there. `rose` is the rose's middle on the map and `y` the
    /// map's (`m`'s offsets).
    fn map_wakes(&mut self, m: Mount, rose: Vec2, y: f32) {
        let fin = Finish::Room(0);
        let gold = [1.0, 0.72, 0.32, 0.75];
        let centre = m.at(Vec3::new(rose.x, rose.y, 0.0645));
        self.cue(Cue {
            finish: fin,
            delay: WING_WAKES,
            sound: Sfx::Chime,
            at: centre,
            volume: 0.8,
        });
        self.sparks(Sparks {
            delay: WING_WAKES,
            ..Sparks::burst(fin, centre + m.n * 0.1)
        });
        // Laid turned back an eighth, before the gilt rose.
        let turn = FRAC_PI_4;
        let grows = Change::Arrives {
            from: Vec3::ZERO,
            grow: 0.1,
        };
        let turns = Change::Shifts {
            by: Vec3::ZERO,
            turn: Quat::from_axis_angle(m.n, turn),
            scale: 1.0,
        };
        let g = self.mover(centre, Motion::Still, 0.0, None);
        self.awaiting(g, Awaits::new(fin, grows).after(WING_WAKES).over(2.0));
        let g = self.mover(centre, Motion::Still, 0.0, Some(g));
        self.awaiting(g, Awaits::new(fin, turns).after(WING_WAKES).over(2.6));
        let star = self.shape(Mesh::prism(&star_outline(&[0.36, 0.24].repeat(4), 0.05), 0.003));
        self.moving(g, star.inst(centre, m.rot(0.01 - turn), kind::CRYSTAL, gold));
        let ring = self.arc_inst(
            centre - m.n * 0.001,
            (m.right(), Vec3::Y),
            (0.27, 0.018, 0.003),
            (0.0, TAU, 48),
            (kind::CRYSTAL, gold),
        );
        self.moving(g, ring);

        // The ship sails out along the open water between the lands and back, off the
        // map by more than its half length, so its bow clears the map as it turns. It
        // rests 40 s at either end, where it turns about (each half turn of the tick
        // timed into a rest); it is laid down heading home, the tick having turned it
        // about as it sets out. It pitches a little as it goes.
        let (from, to) = (-1.35f32, 1.35f32);
        let start = m.at(Vec3::new(from, y - 0.1, 0.33));
        let period = 160.0;
        let slide = Motion::Slide {
            travel: m.right() * (to - from),
            period,
        };
        let about = Motion::Tick {
            axis: Vec3::Y,
            steps: 2,
            period,
        };
        let pitch = Motion::Swing {
            axis: m.n,
            amp: 0.05,
            period: 5.5,
        };
        let rises = Change::Arrives {
            from: -m.n * 0.27,
            grow: 0.0,
        };
        let ship = self.mover(start, slide, 0.0, None);
        let ship = self.mover(start, about, period * 3.0 / 16.0, Some(ship));
        let ship = self.mover(start, Motion::Still, 0.0, Some(ship));
        self.awaiting(ship, Awaits::new(fin, rises).after(WING_WAKES + 0.6).over(2.4));
        let ship = self.mover(start, pitch, 0.0, Some(ship));
        for part in self.model_ship(start, -m.right()) {
            self.moving(ship, part);
        }
    }

    /// A model ship about `c` heading `fwd` (level): a gilt hull, two masts with cream
    /// sails set fore and aft, and a red pennant streaming from the main.
    fn model_ship(&mut self, c: Vec3, fwd: Vec3) -> Vec<Inst> {
        // Laid out along x (the bow), up y and across z, at 1.4 times these sizes.
        let k = 1.4;
        let rot = Quat::from_mat3(&Mat3::from_cols(fwd, Vec3::Y, fwd.cross(Vec3::Y)));
        let at = |x: f32, y: f32| c + rot * (Vec3::new(x, y, 0.0) * k);
        let hull = [
            (-0.15, 0.04),
            (-0.14, 0.0),
            (-0.11, -0.03),
            (-0.05, -0.045),
            (0.06, -0.045),
            (0.12, -0.02),
            (0.16, 0.02),
            (0.2, 0.05),
            (0.13, 0.03),
            (-0.09, 0.03),
            (-0.1, 0.06),
            (-0.155, 0.06),
        ]
        .map(|(x, y)| Vec2::new(x, y) * k);
        let hull = self.shape(Mesh::prism(&hull, 0.075 * k));
        let mut parts = vec![hull.at(c, rot, kind::METAL, rgba(GILT, 1.0))];
        for (x, top) in [(0.0, 0.29), (0.1, 0.22)] {
            let mast = self.pole_inst(at(x, 0.02), at(x, top), 0.01 * k, kind::WOOD, rgba(DARK_WOOD, 1.0));
            parts.push(mast);
        }
        let set = rot * Quat::from_rotation_y(0.2);
        for (x, y, w, h) in [
            (0.0, 0.13, 0.15, 0.11),
            (0.0, 0.235, 0.11, 0.07),
            (0.1, 0.12, 0.1, 0.09),
        ] {
            let size = Vec3::new(w, h, 0.004) * k;
            parts.push(Inst::new(at(x, y), set, size, kind::PAPER, [0.92, 0.86, 0.72, 0.25], 0));
        }
        let pennant = [Vec2::new(0.0, 0.0), Vec2::new(0.0, 0.025), Vec2::new(-0.07, 0.0125)].map(|p| p * k);
        let pennant = self.shape(Mesh::prism(&pennant, 0.003));
        parts.push(pennant.at(at(0.0, 0.29), rot, kind::CLOTH, rgba([0.5, 0.06, 0.06], 1.0)));
        parts
    }

    /// When the East Wing is in order a moon dial rises from behind the clock at its
    /// end, once its runner is down: a lunette of night sky strewn with gilt stars and
    /// two moons, turning slowly from then on (a moon rising at the left as the other
    /// sets at the right), a pale light about it. `top` is the top of the clock's board,
    /// which hides it until then and its lower half after.
    fn moon_dial(&mut self, m: Mount, top: f32) {
        let fin = Finish::Room(1);
        let r = 0.96;
        let centre = m.at(Vec3::new(0.0, top, 0.045));
        self.cue(Cue {
            finish: fin,
            delay: WING_WAKES,
            sound: Sfx::Chime,
            at: centre,
            volume: 0.8,
        });
        self.sparks(Sparks {
            delay: WING_WAKES + 2.2,
            rgba: [0.8, 0.9, 1.3, 2.2],
            spread: Vec3::splat(0.5),
            ..Sparks::burst(fin, centre + Vec3::Y * 0.5 + m.n * 0.15)
        });
        // All of it within the board's thickness, so that inside it, it is hidden.
        let rises = Change::Arrives {
            from: Vec3::Y * -(r + 0.1),
            grow: 1.0,
        };
        let dial = self.mover(centre, Motion::Still, 0.0, None);
        self.awaiting(dial, Awaits::new(fin, rises).after(WING_WAKES).over(3.0));
        let frame = self.arc_inst(
            centre,
            (m.right(), Vec3::Y),
            (r + 0.03, 0.08, 0.05),
            (0.0, PI, 24),
            (kind::METAL, rgba(GILT, 1.0)),
        );
        self.moving(dial, frame);
        self.light_riding(
            dial,
            m.at(Vec3::new(0.0, top + 0.4, 0.8)),
            5.0,
            [0.62, 0.72, 1.0],
            0.5,
            LightKind::Static,
        );
        let spin = Motion::Spin {
            axis: m.n,
            rate: -TAU / 240.0,
        };
        let turns = self.mover(centre, spin, 0.0, Some(dial));
        self.awaiting(
            turns,
            Awaits::new(fin, Change::Kindles { dim: 1.0 })
                .after(WING_WAKES + 2.5)
                .over(6.0)
                .still(),
        );
        let sky = self.disc_inst(m, Vec3::new(0.0, top, 0.04), (r, 0.01), kind::PLAIN, rgba(SKY, 1.0));
        self.moving(turns, sky);
        // Gilt stars about the sky, clear of the moons, and the moons (glowing a little)
        // in gilt rims at its top and bottom: each kind one piece.
        let moon_at = [Vec2::new(0.0, 0.62), Vec2::new(0.0, -0.62)];
        let (mut stars, mut moons, mut rims) = (Mesh::default(), Mesh::default(), Mesh::default());
        let point = Mesh::prism(&star_outline(&[1.0; 4], 0.3), 1.0);
        let mut rng = StdRng::seed_from_u64(0x3007);
        while stars.idx.len() < 18 * point.idx.len() {
            let p = Vec2::new(rng.gen_range(-0.85..0.85), rng.gen_range(-0.85..0.85));
            if p.length() > 0.85 || moon_at.iter().any(|&q| q.distance(p) < 0.26) {
                continue;
            }
            let s = rng.gen_range(0.035..0.07);
            let spin = Quat::from_rotation_z(rng.gen_range(-0.3..0.3));
            stars.add(&point, p.extend(0.0), spin, Vec3::new(s, s, 0.004));
        }
        let face = Quat::from_rotation_x(FRAC_PI_2);
        for q in moon_at {
            moons.add(&Mesh::cylinder(32), q.extend(0.0), face, Vec3::new(0.32, 0.006, 0.32));
            rims.add(
                &Mesh::ring(&circle(0.18, 32), &circle(0.155, 32), 0.01),
                q.extend(0.0),
                Quat::IDENTITY,
                Vec3::ONE,
            );
        }
        let on_sky = |z: f32| m.at(Vec3::new(0.0, top, z));
        for (mesh, z, kind, color) in [
            (stars, 0.0465, kind::METAL, rgba(GILT, 1.0)),
            (moons, 0.0475, kind::PAPER, [1.0, 0.88, 0.66, 0.3]),
            (rims, 0.049, kind::METAL, rgba(GILT, 1.0)),
        ] {
            let shape = self.shape(mesh);
            self.moving(turns, shape.at(on_sky(z), m.rot(0.0), kind, color));
        }
    }

    /// When the whole Hall is in order, after the last wing's moment: stars come down
    /// from the reading room's painted sky to circle its chandelier, gold sifting down
    /// after them, and the rings they make turn slowly from then on, a starlight about
    /// them and motes drifting there. (The emblem over the hearth kindles too:
    /// `hearth_dressing`.)
    fn finale(&mut self) {
        let fin = Finish::Library;
        let ch = CHANDELIER;
        self.cue(Cue {
            finish: fin,
            delay: FINALE,
            sound: Sfx::Veil,
            at: ch,
            volume: 0.8,
        });
        let star = self.shape({
            let flat = Mesh::prism(&star_outline(&[0.15, 0.075].repeat(4), 0.027), 0.014);
            let mut star = flat.clone();
            star.add(&flat, Vec3::ZERO, Quat::from_rotation_y(FRAC_PI_2), Vec3::ONE);
            star
        });
        // Two rings, as an armillary's: twelve stars about the hoop turning one way, and
        // seven smaller about the crown turning slower the other, once all have come.
        // Each comes from a coffer of its own, and bobs in its own time.
        let gathered = FINALE + 4.3;
        let turns = Awaits::new(fin, Change::Kindles { dim: 1.0 })
            .after(gathered)
            .over(8.0)
            .still();
        for (count, r, up, period, size, (first, apart), sky_r) in [
            (12, 2.3, 0.3, 200.0, 1.0, (0.2, 0.15), (2.6, 2.2)),
            (7, 1.25, 1.2, -300.0, 0.75, (0.4, 0.25), (1.0, 1.2)),
        ] {
            let spin = Motion::Spin {
                axis: Vec3::Y,
                rate: TAU / period,
            };
            let ring = self.mover(ch, spin, 0.0, None);
            self.awaiting(ring, turns);
            for k in 0..count {
                let a = (k as f32 + 0.25) / count as f32 * TAU;
                let out = Vec3::new(libm::cosf(a), 0.0, libm::sinf(a));
                let rest = ch + out * r + Vec3::Y * (up + 0.12 * libm::sinf(3.0 * a));
                let h = (k as f32 * 0.618).fract();
                let sky = out * (sky_r.0 + sky_r.1 * h) + Vec3::Y * (HALL_H - 0.05);
                let comes = Change::Arrives {
                    from: sky - rest,
                    grow: 0.3,
                };
                let bob = Motion::Bob {
                    amp: 0.06,
                    period: 6.0 + 3.0 * h,
                    rock: 0.3,
                };
                let m = self.mover(rest, bob, 9.0 * h, Some(ring));
                let after = FINALE + first + k as f32 * apart;
                self.awaiting(m, Awaits::new(fin, comes).after(after).over(2.4));
                let (turn, light) = (Quat::from_rotation_y(a), [0.75, 0.85, 1.0, 3.5]);
                let i = star.scaled(size).inst(rest, turn, kind::CRYSTAL, light);
                self.moving(m, i);
            }
        }
        let light = self.mover(ch, Motion::Still, 0.0, None);
        self.awaiting(
            light,
            Awaits::new(fin, Change::Kindles { dim: 0.0 })
                .after(FINALE + 1.0)
                .over(6.0),
        );
        self.light_riding(
            light,
            ch + Vec3::Y * 0.8,
            11.0,
            [0.62, 0.72, 1.0],
            0.9,
            LightKind::Static,
        );
        // Gold sifting down from the painted sky as they come, fading as it falls; a
        // burst from the chandelier when they have gathered, and motes about it after.
        self.sparks(Sparks {
            delay: FINALE + 0.5,
            lasts: Some(6.0),
            spread: Vec3::new(8.0, 0.2, 8.0),
            rate: 60.0,
            vel: Vec3::Y * -0.45,
            jitter: Vec3::splat(0.08),
            lift: -0.05,
            drag: 0.1,
            wander: 0.15,
            life: 6.0,
            rgba: [1.3, 1.0, 0.55, 2.2],
            size: Vec3::splat(0.024),
            ..Sparks::burst(fin, Vec3::new(0.0, HALL_H - 0.4, 0.0))
        });
        self.sparks(Sparks {
            delay: gathered,
            lasts: Some(1.5),
            rate: 80.0,
            spread: Vec3::new(1.2, 0.2, 1.2),
            ..Sparks::burst(fin, ch + Vec3::Y * 0.3)
        });
        self.sparks(Sparks {
            delay: gathered,
            rgba: [0.85, 0.9, 1.0, 1.5],
            ..Sparks::motes(fin, ch + Vec3::Y * 0.3, Vec3::new(2.8, 0.7, 2.8))
        });
    }
}

/// A crescent's outline: a disc `r` round, less one `cut` round whose middle is `off`
/// along +x, so its horns point that way; `n` steps along each edge.
fn crescent(r: f32, cut: f32, off: f32, n: usize) -> Vec<Vec2> {
    // Where the edges meet, as angles about each disc's middle.
    let x = (r * r - cut * cut + off * off) / (2.0 * off);
    let h = libm::sqrtf(r * r - x * x);
    let (a, b) = (libm::atan2f(h, x), libm::atan2f(h, x - off));
    let round = |c: Vec2, radius: f32, t: f32| c + Vec2::new(libm::cosf(t), libm::sinf(t)) * radius;
    let outer = (0..n).map(|k| round(Vec2::ZERO, r, a + (TAU - 2.0 * a) * k as f32 / n as f32));
    let inner = (0..n).map(|k| round(Vec2::X * off, cut, -b - (TAU - 2.0 * b) * k as f32 / n as f32));
    outer.chain(inner).collect()
}

/// A star's outline: a point `tips[k]` out for each of `tips`, going round from +x,
/// and `inner` between them.
fn star_outline(tips: &[f32], inner: f32) -> Vec<Vec2> {
    let n = tips.len() * 2;
    (0..n)
        .map(|k| {
            let r = if k % 2 == 0 { tips[k / 2] } else { inner };
            let a = k as f32 / n as f32 * TAU;
            Vec2::new(libm::cosf(a), libm::sinf(a)) * r
        })
        .collect()
}

/// The fire on the hearth, `c` under its middle: logs on a bed of embers, flames
/// licking up off them, and a wisp of smoke going up the flue.
fn hearth_fire(lib: &mut Library, c: Vec3) {
    let log = lib.shape(Mesh::cylinder(12)).sized(Vec3::new(0.14, 0.9, 0.14));
    for (i, (x, ang)) in [(-0.3f32, 0.3f32), (0.25, -0.25), (0.0, 0.05)].into_iter().enumerate() {
        let p = c + Vec3::new(x, 0.12 + i as f32 * 0.07, 0.0);
        let lie = Quat::from_rotation_y(ang) * Quat::from_rotation_z(FRAC_PI_2);
        lib.decor(log.inst(p, lie, kind::WOOD, [0.08, 0.04, 0.02, 1.0]));
    }
    let mut embers = Inst::aabox(
        c + Vec3::new(0.0, 0.035, 0.03),
        Vec3::new(1.1, 0.05, 0.32),
        kind::EMISSIVE,
        [1.0, 0.28, 0.05],
    );
    embers.color[3] = 2.5;
    embers.param = 1;
    lib.decor(embers);
    // A broad sheet of flame along the logs, and tongues off it at either end.
    for (at, size, turn) in [
        (Vec3::new(0.0, 0.62, 0.0), Vec3::new(1.2, 0.85, 0.34), 0.0f32),
        (Vec3::new(-0.38, 0.5, 0.04), Vec3::new(0.5, 0.55, 0.24), 0.5),
        (Vec3::new(0.4, 0.47, -0.03), Vec3::new(0.45, 0.5, 0.24), -0.4),
    ] {
        lib.translucent(Inst::new(
            c + at,
            Quat::from_rotation_y(turn),
            size,
            kind::FIRE,
            [1.0, 0.5, 0.16, 2.5],
            1,
        ));
    }
    // Up into the dark under the lintel.
    lib.translucent(Inst::new(
        c + Vec3::new(0.0, 1.45, 0.08),
        Quat::IDENTITY,
        Vec3::new(0.5, 1.1, 0.26),
        kind::MIST,
        [0.4, 0.38, 0.36, 0.5],
        0,
    ));
}

/// A clock on the mantel at `shelf` (the shelf's top, under the clock's middle): a
/// walnut case with a round head, its dial under a domed glass, keeping the great
/// clock's time.
fn mantel_clock(lib: &mut Library, shelf: Vec3) {
    let m = Mount {
        at: shelf - Vec3::Z * 0.1,
        n: -Vec3::Z,
    };
    let walnut = rgba(WALNUT, 1.0);
    let gilt = rgba(GILT, 1.0);
    lib.mounted(
        m,
        Vec3::new(0.0, 0.015, -0.1),
        Vec3::new(0.5, 0.03, 0.24),
        0.0,
        kind::WOOD,
        walnut,
    );
    let (half, shoulder) = (0.21f32, 0.27f32);
    let mut outline = vec![Vec2::new(-half, 0.0), Vec2::new(half, 0.0)];
    outline.extend((0..=16).map(|k| {
        let a = PI * k as f32 / 16.0;
        Vec2::new(half * libm::cosf(a), shoulder + half * libm::sinf(a))
    }));
    let case = lib.shape(Mesh::prism(&outline, 0.2));
    lib.decor(case.at(m.at(Vec3::new(0.0, 0.03, -0.1)), m.rot(0.0), kind::WOOD, walnut));
    let knob = lib.shape(Mesh::sphere(12)).sized(Vec3::splat(0.04));
    lib.decor(knob.inst(
        m.at(Vec3::new(0.0, 0.03 + shoulder + half + 0.015, -0.1)),
        Quat::IDENTITY,
        kind::METAL,
        gilt,
    ));
    let centre = Vec3::new(0.0, 0.03 + shoulder, 0.0);
    lib.disc(m, centre + Vec3::Z * 0.01, (0.15, 0.01), kind::PLAIN, rgba(CREAM, 1.0));
    let face = m.at(centre + Vec3::Z * 0.015);
    lib.arc(
        face,
        (m.right(), Vec3::Y),
        (0.157, 0.016, 0.012),
        (0.0, TAU, 32),
        (kind::METAL, gilt),
    );
    let ink = rgba(IRON, 1.0);
    for k in 0..12 {
        let a = FRAC_PI_2 - k as f32 * TAU / 12.0;
        let at = centre + Vec3::new(libm::cosf(a), libm::sinf(a), 0.0) * 0.125 + Vec3::Z * 0.016;
        let len = if k % 3 == 0 { 0.03 } else { 0.018 };
        lib.mounted(m, at, Vec3::new(len, 0.007, 0.002), a, kind::METAL, ink);
    }
    lib.clock_hands((face, m.n, 0.15), (11.0, 50.0), (ink, gilt), false);
    let glass = Mesh::lathe(
        &[
            (0.15, 0.0),
            (0.146, 0.02),
            (0.133, 0.04),
            (0.105, 0.058),
            (0.06, 0.07),
            (0.0, 0.074),
        ]
        .map(Vec2::from),
        32,
    );
    let glass = lib.shape(glass);
    lib.translucent(glass.at(
        face,
        Quat::from_rotation_arc(Vec3::Y, m.n),
        kind::CLEAR,
        [0.92, 0.95, 1.0, 0.08],
    ));
}
