//! The library's layout: bookcase faces, slot assignment, heaps, static geometry and lights.

use crate::catalog::{self, Series, CATEGORIES, WEST_CATEGORIES};
use crate::gfx::{kind, Inst};
use glam::{Mat3, Quat, Vec2, Vec3};
use rand::prelude::*;
use std::collections::HashSet;
use std::f32::consts::TAU;

pub const ROWS: usize = 6;
pub const CASES_PER_CATEGORY: usize = 3;
pub const UNIT_W: f32 = 2.6;
pub const UNIT_PITCH: f32 = 2.7;
pub const SIDE_T: f32 = 0.08;
pub const USABLE_W: f32 = 2.36;
pub const FACE_DEPTH: f32 = 0.43;
pub const UNIT_DEPTH: f32 = 0.9;
pub const UNIT_H: f32 = 3.1;
pub const SHELF_BASE: f32 = 0.14;
pub const SHELF_STEP: f32 = 0.48;
pub const BOARD_T: f32 = 0.03;
pub const HALL_X: f32 = 24.0;
pub const HALL_Z: f32 = 18.0;
pub const HALL_H: f32 = 8.0;
pub const VEIL_X: f32 = 11.6;
pub const TABLE_H: f32 = 0.78;

const WALNUT: [f32; 3] = [0.16, 0.08, 0.035];
const DARK_WOOD: [f32; 3] = [0.07, 0.035, 0.018];
const STONE: [f32; 3] = [0.36, 0.33, 0.29];
const BRASS: [f32; 3] = [0.55, 0.36, 0.12];

/// Height of the shelf surface for reading-order row (0 = top).
pub fn row_floor_y(row: usize) -> f32 {
    SHELF_BASE + (ROWS - 1 - row) as f32 * SHELF_STEP
}

pub struct Face {
    /// Centre of the front plane, at floor level.
    pub center: Vec3,
    pub normal: Vec3,
    pub right: Vec3,
    pub category: usize,
    pub case_idx: usize,
}

impl Face {
    pub fn rot(&self) -> Quat {
        Quat::from_mat3(&Mat3::from_cols(self.right, Vec3::Y, self.normal))
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Run {
    pub series: u32,
    pub x0: f32,
    pub x1: f32,
}

pub struct BookInfo {
    pub series: u32,
    pub volume: u32,
    /// (thickness, height, depth)
    pub size: Vec3,
    pub face: u32,
    pub row: u8,
    pub slot_pos: Vec3,
    pub slot_rot: Quat,
    pub heap: u32,
    pub heap_pos: Vec3,
    pub heap_rot: Quat,
}

pub struct Heap {
    pub base: Vec3,
    pub radius: f32,
    /// Bottom to top.
    pub books: Vec<u32>,
    pub wing: usize,
}

#[derive(Clone, Copy)]
pub struct Obstacle {
    pub min: Vec3,
    pub max: Vec3,
    pub blocks_ray: bool,
}

#[derive(Clone, Copy, PartialEq)]
pub enum LightKind {
    Static,
    Flicker,
    Lantern(usize),
}

pub struct LightDef {
    pub pos: Vec3,
    pub radius: f32,
    pub color: [f32; 3],
    pub intensity: f32,
    pub kind: LightKind,
}

pub struct Label {
    pub pos: Vec3,
    pub normal: Vec3,
    pub text: String,
    pub category: usize,
    pub is_sign: bool,
}

pub struct Library {
    pub series: Vec<Series>,
    pub series_books: Vec<Vec<u32>>,
    pub books: Vec<BookInfo>,
    pub faces: Vec<Face>,
    pub runs: Vec<[Vec<Run>; ROWS]>,
    pub heaps: Vec<Heap>,
    pub obstacles: Vec<Obstacle>,
    pub static_insts: Vec<Inst>,
    pub lights: Vec<LightDef>,
    pub labels: Vec<Label>,
    pub category_books: Vec<u32>,
    pub category_center: Vec<Vec3>,
    /// (category, position) of each category lantern; drawn per frame since they change.
    pub lanterns: Vec<(usize, Vec3)>,
}

fn quat_from_cols(x: Vec3, y: Vec3, z: Vec3) -> Quat {
    Quat::from_mat3(&Mat3::from_cols(x, y, z)).normalize()
}

pub fn wing_of_category(c: usize) -> usize {
    c / WEST_CATEGORIES
}

impl Library {
    pub fn generate(seed: u64) -> Self {
        let mut rng = StdRng::seed_from_u64(seed);
        let mut lib = Library {
            series: Vec::new(),
            series_books: Vec::new(),
            books: Vec::new(),
            faces: Vec::new(),
            runs: Vec::new(),
            heaps: Vec::new(),
            obstacles: Vec::new(),
            static_insts: Vec::new(),
            lights: Vec::new(),
            labels: Vec::new(),
            category_books: vec![0; CATEGORIES.len()],
            category_center: vec![Vec3::ZERO; CATEGORIES.len()],
            lanterns: Vec::new(),
        };
        lib.build_hall();
        lib.build_stacks(&mut rng);
        lib.build_center();
        lib.build_heaps(&mut rng);
        lib
    }

    fn push(&mut self, i: Inst) {
        self.static_insts.push(i);
    }

    fn obstacle(&mut self, min: Vec3, max: Vec3, blocks_ray: bool) {
        self.obstacles.push(Obstacle { min, max, blocks_ray });
    }

    fn light(&mut self, pos: Vec3, radius: f32, color: [f32; 3], intensity: f32, kind: LightKind) {
        self.lights.push(LightDef { pos, radius, color, intensity, kind });
    }

    fn build_hall(&mut self) {
        let (hx, hz, hh) = (HALL_X, HALL_Z, HALL_H);
        let floor_col = [0.30, 0.16, 0.075];
        self.push(Inst::minmax(Vec3::new(-hx, -0.2, -hz), Vec3::new(hx, 0.0, hz), kind::FLOOR, floor_col));
        // Walls.
        let t = 0.5;
        self.push(Inst::minmax(Vec3::new(-hx - t, 0.0, -hz - t), Vec3::new(hx + t, hh, -hz), kind::STONE, STONE));
        self.push(Inst::minmax(Vec3::new(-hx - t, 0.0, hz), Vec3::new(hx + t, hh, hz + t), kind::STONE, STONE));
        self.push(Inst::minmax(Vec3::new(-hx - t, 0.0, -hz), Vec3::new(-hx, hh, hz), kind::STONE, STONE));
        self.push(Inst::minmax(Vec3::new(hx, 0.0, -hz), Vec3::new(hx + t, hh, hz), kind::STONE, STONE));
        // Ceiling and beams.
        self.push(Inst::minmax(Vec3::new(-hx - t, hh, -hz - t), Vec3::new(hx + t, hh + 0.3, hz + t), kind::WOOD, DARK_WOOD));
        let mut x = -hx + 2.0;
        while x < hx {
            self.push(Inst::minmax(Vec3::new(x - 0.18, hh - 0.45, -hz), Vec3::new(x + 0.18, hh, hz), kind::WOOD, [0.11, 0.055, 0.025]));
            x += 4.0;
        }
        // Wainscot panelling with a rail.
        let wh = 1.15;
        let wc = [0.13, 0.065, 0.03];
        self.push(Inst::minmax(Vec3::new(-hx, 0.0, -hz), Vec3::new(hx, wh, -hz + 0.06), kind::WOOD, wc));
        self.push(Inst::minmax(Vec3::new(-hx, 0.0, hz - 0.06), Vec3::new(hx, wh, hz), kind::WOOD, wc));
        self.push(Inst::minmax(Vec3::new(-hx, 0.0, -hz), Vec3::new(-hx + 0.06, wh, hz), kind::WOOD, wc));
        self.push(Inst::minmax(Vec3::new(hx - 0.06, 0.0, -hz), Vec3::new(hx, wh, hz), kind::WOOD, wc));
        for (a, b) in [
            (Vec3::new(-hx, wh, -hz), Vec3::new(hx, wh + 0.06, -hz + 0.1)),
            (Vec3::new(-hx, wh, hz - 0.1), Vec3::new(hx, wh + 0.06, hz)),
            (Vec3::new(-hx, wh, -hz), Vec3::new(-hx + 0.1, wh + 0.06, hz)),
            (Vec3::new(hx - 0.1, wh, -hz), Vec3::new(hx, wh + 0.06, hz)),
        ] {
            self.push(Inst::minmax(a, b, kind::WOOD, WALNUT));
        }

        // Columns flanking the reading room (the veil hangs between the east ones).
        for sx in [-VEIL_X, VEIL_X] {
            for z in [-16.0, -8.0, 8.0, 16.0] {
                let c = Vec3::new(sx, 0.0, z);
                self.push(Inst::aabox(c + Vec3::Y * hh * 0.5, Vec3::new(0.7, hh, 0.7), kind::STONE, [0.42, 0.39, 0.35]));
                self.push(Inst::aabox(c + Vec3::Y * 0.2, Vec3::new(0.95, 0.4, 0.95), kind::STONE, [0.3, 0.28, 0.25]));
                self.push(Inst::aabox(c + Vec3::Y * (hh - 0.6), Vec3::new(0.95, 0.3, 0.95), kind::STONE, [0.3, 0.28, 0.25]));
                self.obstacle(c - Vec3::new(0.48, 0.0, 0.48), c + Vec3::new(0.48, hh, 0.48), true);
            }
        }

        // Moonlit windows on the north wall.
        for x in [-20.0, -15.5, -7.0, -2.3, 2.3, 7.0, 15.5, 20.0] {
            let c = Vec3::new(x, 4.4, -hz + 0.02);
            let glass = Inst::new(c, Quat::IDENTITY, Vec3::new(1.5, 4.2, 0.08), kind::GLASS, [0.32, 0.42, 0.85, 1.2], 0);
            self.push(glass);
            // Frame and sill.
            self.push(Inst::aabox(c + Vec3::new(0.0, -2.2, 0.12), Vec3::new(1.9, 0.14, 0.3), kind::STONE, [0.45, 0.42, 0.38]));
            self.push(Inst::aabox(c + Vec3::new(0.0, 2.2, 0.08), Vec3::new(1.9, 0.2, 0.2), kind::STONE, [0.45, 0.42, 0.38]));
            for s in [-1.0, 1.0] {
                self.push(Inst::aabox(c + Vec3::new(s * 0.85, 0.0, 0.08), Vec3::new(0.2, 4.6, 0.2), kind::STONE, [0.45, 0.42, 0.38]));
            }
            self.light(Vec3::new(x, 3.2, -hz + 1.5), 8.0, [0.45, 0.55, 1.0], 0.9, LightKind::Static);
        }

        // Sconces.
        let sconce = |lib: &mut Self, p: Vec3, n: Vec3| {
            lib.push(Inst::aabox(p - n * 0.08, Vec3::new(0.12, 0.3, 0.12), kind::WOOD, BRASS));
            lib.push(Inst::aabox(p + Vec3::Y * 0.12, Vec3::new(0.06, 0.12, 0.06), kind::EMISSIVE, [0.9, 0.85, 0.7]));
            let mut flame = Inst::aabox(p + Vec3::Y * 0.24, Vec3::new(0.05, 0.1, 0.05), kind::EMISSIVE, [1.0, 0.55, 0.18]);
            flame.color[3] = 12.0;
            flame.param = 1;
            lib.push(flame);
            lib.light(p + n * 0.4 + Vec3::Y * 0.3, 8.0, [1.0, 0.62, 0.3], 1.3, LightKind::Flicker);
        };
        for z in [-14.0, -6.0, 6.0, 14.0] {
            sconce(self, Vec3::new(-hx + 0.15, 2.6, z), Vec3::X);
            sconce(self, Vec3::new(hx - 0.15, 2.6, z), -Vec3::X);
        }
        for x in [-17.0, 17.0] {
            sconce(self, Vec3::new(x, 2.6, hz - 0.15), -Vec3::Z);
        }
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
                    self.build_unit(Vec3::new(cx, 0.0, rz));
                }
            }
            let mut cat = wing * WEST_CATEGORIES;
            for &rz in &rows_z {
                for nz in [-1.0f32, 1.0] {
                    let normal = Vec3::new(0.0, 0.0, nz);
                    let right = Vec3::Y.cross(normal);
                    let mut order: Vec<f32> = xs.to_vec();
                    order.sort_by(|a, b| (a * right.x).partial_cmp(&(b * right.x)).unwrap());
                    let first_face = self.faces.len();
                    for (ci, &cx) in order.iter().enumerate() {
                        self.faces.push(Face {
                            center: Vec3::new(cx, 0.0, rz + nz * UNIT_DEPTH * 0.5),
                            normal,
                            right,
                            category: cat,
                            case_idx: ci,
                        });
                        self.runs.push(Default::default());
                    }
                    self.fill_category(rng, cat, first_face, &mut titles);
                    // Sign + lantern above the middle case.
                    let (mid_center, mid_rot) = (self.faces[first_face + 1].center, self.faces[first_face + 1].rot());
                    let sign_pos = mid_center + Vec3::Y * (UNIT_H + 0.38) + normal * 0.02;
                    let ccol = catalog::category_color(cat);
                    let plaque = Inst::new(
                        sign_pos,
                        mid_rot,
                        Vec3::new(1.5, 0.34, 0.05),
                        kind::PLAQUE,
                        [ccol[0], ccol[1], ccol[2], 0.08],
                        0,
                    );
                    self.push(plaque);
                    self.push(Inst::aabox(sign_pos - Vec3::Y * 0.21 - normal * 0.02, Vec3::new(0.06, 0.12, 0.06), kind::WOOD, BRASS));
                    self.labels.push(Label {
                        pos: sign_pos + normal * 0.04,
                        normal,
                        text: CATEGORIES[cat].name.to_string(),
                        category: cat,
                        is_sign: true,
                    });
                    let lantern = mid_center + Vec3::Y * (UNIT_H + 0.95) + normal * 0.05;
                    self.push(Inst::aabox(lantern + Vec3::Y * 0.16, Vec3::new(0.22, 0.04, 0.22), kind::WOOD, BRASS));
                    self.push(Inst::aabox(lantern - Vec3::Y * 0.16, Vec3::new(0.2, 0.04, 0.2), kind::WOOD, BRASS));
                    self.push(Inst::aabox(lantern + Vec3::Y * 0.5, Vec3::new(0.02, 0.6, 0.02), kind::WOOD, BRASS));
                    self.lanterns.push((cat, lantern));
                    self.light(lantern + normal * 0.5, 7.5, [1.0, 0.7, 0.4], 0.35, LightKind::Lantern(cat));
                    self.category_center[cat] = mid_center + normal * 1.5 + Vec3::Y * 1.0;
                    cat += 1;
                }
            }
        }
    }

    fn build_unit(&mut self, c: Vec3) {
        let hw = UNIT_W * 0.5;
        let hd = UNIT_DEPTH * 0.5;
        for s in [-1.0, 1.0] {
            self.push(Inst::aabox(
                c + Vec3::new(s * (hw - SIDE_T * 0.5), UNIT_H * 0.5, 0.0),
                Vec3::new(SIDE_T, UNIT_H, UNIT_DEPTH),
                kind::WOOD,
                WALNUT,
            ));
        }
        self.push(Inst::aabox(c + Vec3::Y * (UNIT_H + 0.04), Vec3::new(UNIT_W + 0.08, 0.08, UNIT_DEPTH + 0.08), kind::WOOD, WALNUT));
        self.push(Inst::aabox(c + Vec3::Y * (UNIT_H + 0.105), Vec3::new(UNIT_W + 0.16, 0.05, UNIT_DEPTH + 0.16), kind::WOOD, [0.12, 0.06, 0.028]));
        self.push(Inst::aabox(c + Vec3::Y * (SHELF_BASE * 0.5), Vec3::new(UNIT_W - 0.02, SHELF_BASE, UNIT_DEPTH - 0.04), kind::WOOD, [0.1, 0.05, 0.022]));
        self.push(Inst::aabox(c + Vec3::Y * (UNIT_H * 0.5), Vec3::new(UNIT_W - 2.0 * SIDE_T, UNIT_H, 0.04), kind::WOOD, DARK_WOOD));
        for nz in [-1.0f32, 1.0] {
            for i in 1..ROWS {
                let y = SHELF_BASE + i as f32 * SHELF_STEP;
                self.push(Inst::aabox(
                    c + Vec3::new(0.0, y - BOARD_T * 0.5, nz * (0.02 + FACE_DEPTH * 0.5)),
                    Vec3::new(UNIT_W - 2.0 * SIDE_T, BOARD_T, FACE_DEPTH),
                    kind::WOOD,
                    WALNUT,
                ));
            }
        }
        self.obstacle(c - Vec3::new(hw + 0.04, 0.0, hd + 0.04), c + Vec3::new(hw + 0.04, UNIT_H + 1.2, hd + 0.04), true);
    }

    /// Generate this category's series and lay them out across its three faces.
    fn fill_category(&mut self, rng: &mut StdRng, cat: usize, first_face: usize, titles: &mut HashSet<String>) {
        let shelves = CASES_PER_CATEGORY * ROWS;
        let capacity = shelves as f32 * USABLE_W;
        let fill = rng.gen_range(0.74..0.84);
        let series = catalog::generate_series(rng, cat, capacity * fill, titles);
        let gap_min = 0.015;
        let total: f32 = series.iter().map(|s| s.width() + gap_min).sum();
        let per_shelf = total / shelves as f32;

        // Assign each series to the shelf containing its midpoint, then repair overflow.
        let mut assign: Vec<Vec<usize>> = vec![Vec::new(); shelves];
        let mut cum = 0.0;
        for (i, s) in series.iter().enumerate() {
            let w = s.width() + gap_min;
            let shelf = (((cum + w * 0.5) / per_shelf) as usize).min(shelves - 1);
            assign[shelf].push(i);
            cum += w;
        }
        for k in 0..shelves {
            loop {
                let used: f32 = assign[k].iter().map(|&i| series[i].width() + gap_min).sum();
                if used <= USABLE_W || assign[k].len() <= 1 {
                    break;
                }
                let last = assign[k].pop().unwrap();
                if k + 1 < shelves {
                    assign[k + 1].insert(0, last);
                }
            }
        }
        let mut keep = vec![false; series.len()];
        for k in 0..shelves {
            for &i in &assign[k] {
                keep[i] = true;
            }
        }

        let mut remap = vec![u32::MAX; series.len()];
        for (i, s) in series.into_iter().enumerate() {
            if keep[i] {
                remap[i] = self.series.len() as u32;
                self.series.push(s);
                self.series_books.push(Vec::new());
            }
        }

        for k in 0..shelves {
            let face_i = first_face + k / ROWS;
            let row = k % ROWS;
            let ids: Vec<u32> = assign[k].iter().map(|&i| remap[i]).collect();
            if ids.is_empty() {
                continue;
            }
            let used: f32 = ids.iter().map(|&s| self.series[s as usize].width()).sum();
            let gap = ((USABLE_W - used) / ids.len() as f32).min(0.35);
            let span = used + gap * ids.len() as f32;
            let mut x = -span * 0.5 + gap * 0.5;
            let (fc, fr) = (self.faces[face_i].center, self.faces[face_i].rot());
            let (fright, fnormal) = (self.faces[face_i].right, self.faces[face_i].normal);
            for sid in ids {
                let x0 = x;
                let s = &self.series[sid as usize];
                let (h, d) = (s.height, s.depth);
                let thick = s.thickness.clone();
                for (v, &t) in thick.iter().enumerate() {
                    let cx = x + t * 0.5;
                    let pos = fc + fright * cx + Vec3::Y * (row_floor_y(row) + h * 0.5) - fnormal * (d * 0.5 + 0.015);
                    let id = self.books.len() as u32;
                    self.books.push(BookInfo {
                        series: sid,
                        volume: v as u32 + 1,
                        size: Vec3::new(t * 0.985, h, d),
                        face: face_i as u32,
                        row: row as u8,
                        slot_pos: pos,
                        slot_rot: fr,
                        heap: u32::MAX,
                        heap_pos: Vec3::ZERO,
                        heap_rot: Quat::IDENTITY,
                    });
                    self.series_books[sid as usize].push(id);
                    self.category_books[cat] += 1;
                    x += t;
                }
                self.runs[face_i][row].push(Run { series: sid, x0, x1: x });
                x += gap;
            }
        }
        // Shelf range labels.
        for f in first_face..first_face + CASES_PER_CATEGORY {
            for row in 0..ROWS {
                let runs = &self.runs[f][row];
                if runs.is_empty() {
                    continue;
                }
                let key = |s: u32| {
                    let k = &self.series[s as usize].sort_key;
                    let mut c = k.chars();
                    let a = c.next().unwrap_or('?').to_ascii_uppercase();
                    let b = c.next().unwrap_or(' ');
                    format!("{a}{b}")
                };
                let text = format!("{} – {}", key(runs[0].series), key(runs[runs.len() - 1].series));
                let face = &self.faces[f];
                let pos = face.center + Vec3::Y * (row_floor_y(row) - BOARD_T * 0.5) + face.normal * 0.02;
                self.labels.push(Label { pos, normal: face.normal, text, category: face.category, is_sign: false });
            }
        }
    }

    fn build_center(&mut self) {
        // Rug.
        let second = [22u32, 30, 70];
        let mut rug = Inst::aabox(Vec3::new(0.0, 0.012, 0.0), Vec3::new(11.0, 0.024, 8.0), kind::RUG, [0.28, 0.035, 0.03]);
        rug.param = second[0] | (second[1] << 8) | (second[2] << 16);
        self.push(rug);

        // Reading tables with chairs and candles.
        for (tx, tz) in [(-4.5f32, -2.6f32), (4.5, -2.6), (-4.5, 2.6), (4.5, 2.6)] {
            let c = Vec3::new(tx, 0.0, tz);
            self.push(Inst::aabox(c + Vec3::Y * (TABLE_H - 0.03), Vec3::new(2.4, 0.06, 1.2), kind::WOOD, WALNUT));
            for (sx, sz) in [(-1.0, -1.0), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)] {
                self.push(Inst::aabox(
                    c + Vec3::new(sx * 1.08, (TABLE_H - 0.06) * 0.5, sz * 0.48),
                    Vec3::new(0.09, TABLE_H - 0.06, 0.09),
                    kind::WOOD,
                    DARK_WOOD,
                ));
            }
            self.obstacle(c - Vec3::new(1.25, 0.0, 0.65), c + Vec3::new(1.25, TABLE_H, 0.65), true);
            for sz in [-1.0f32, 1.0] {
                for sx in [-0.6f32, 0.6] {
                    let ch = c + Vec3::new(sx, 0.0, sz * 1.0);
                    self.push(Inst::aabox(ch + Vec3::Y * 0.45, Vec3::new(0.48, 0.05, 0.46), kind::WOOD, [0.2, 0.09, 0.04]));
                    self.push(Inst::aabox(ch + Vec3::new(0.0, 0.75, sz * 0.21), Vec3::new(0.48, 0.6, 0.04), kind::WOOD, [0.2, 0.09, 0.04]));
                    for (lx, lz) in [(-0.2, -0.2), (0.2, -0.2), (-0.2, 0.2), (0.2, 0.2)] {
                        self.push(Inst::aabox(ch + Vec3::new(lx, 0.22, lz), Vec3::new(0.04, 0.44, 0.04), kind::WOOD, DARK_WOOD));
                    }
                    self.obstacle(ch - Vec3::new(0.26, 0.0, 0.26), ch + Vec3::new(0.26, 1.05, 0.26), false);
                }
            }
            let candle = c + Vec3::new(0.7, TABLE_H, 0.0);
            self.push(Inst::aabox(candle + Vec3::Y * 0.02, Vec3::new(0.14, 0.04, 0.14), kind::WOOD, BRASS));
            self.push(Inst::aabox(candle + Vec3::Y * 0.11, Vec3::new(0.05, 0.16, 0.05), kind::PLAIN, [0.8, 0.75, 0.6]));
            let mut flame = Inst::aabox(candle + Vec3::Y * 0.22, Vec3::new(0.025, 0.05, 0.025), kind::EMISSIVE, [1.0, 0.55, 0.18]);
            flame.color[3] = 14.0;
            flame.param = 1;
            self.push(flame);
            self.light(candle + Vec3::Y * 0.35, 4.5, [1.0, 0.6, 0.28], 0.7, LightKind::Flicker);
        }

        // Chandelier.
        let ch = Vec3::new(0.0, 5.6, 0.0);
        for k in 0..12 {
            let a = k as f32 / 12.0 * TAU;
            let p = ch + Vec3::new(a.cos(), 0.0, a.sin()) * 1.3;
            self.push(Inst::new(p, Quat::from_rotation_y(-a), Vec3::new(0.08, 0.06, 0.72), kind::WOOD, [BRASS[0], BRASS[1], BRASS[2], 1.0], 0));
            if k % 2 == 0 {
                self.push(Inst::aabox(p + Vec3::Y * 0.1, Vec3::new(0.05, 0.14, 0.05), kind::PLAIN, [0.8, 0.75, 0.6]));
                let mut flame = Inst::aabox(p + Vec3::Y * 0.21, Vec3::new(0.03, 0.06, 0.03), kind::EMISSIVE, [1.0, 0.55, 0.18]);
                flame.color[3] = 14.0;
                flame.param = 1;
                self.push(flame);
            }
        }
        for k in 0..3 {
            let a = k as f32 / 3.0 * TAU;
            let top = Vec3::new(0.0, HALL_H, 0.0);
            let bottom = ch + Vec3::new(a.cos(), 0.0, a.sin()) * 1.3;
            let d = top - bottom;
            let rot = Quat::from_rotation_arc(Vec3::Y, d.normalize());
            self.push(Inst::new((top + bottom) * 0.5, rot, Vec3::new(0.025, d.length(), 0.025), kind::WOOD, [0.1, 0.08, 0.05, 1.0], 0));
        }
        self.light(ch + Vec3::Y * 0.2, 20.0, [1.0, 0.68, 0.38], 4.0, LightKind::Flicker);

        // Fireplace on the south wall.
        let fz = HALL_Z;
        let fc = Vec3::new(0.0, 0.0, fz);
        let st = [0.4, 0.37, 0.33];
        self.push(Inst::minmax(fc + Vec3::new(-2.2, 0.0, -0.9), fc + Vec3::new(-1.4, 2.6, 0.0), kind::STONE, st));
        self.push(Inst::minmax(fc + Vec3::new(1.4, 0.0, -0.9), fc + Vec3::new(2.2, 2.6, 0.0), kind::STONE, st));
        self.push(Inst::minmax(fc + Vec3::new(-2.4, 2.0, -1.0), fc + Vec3::new(2.4, 2.7, 0.0), kind::STONE, st));
        self.push(Inst::minmax(fc + Vec3::new(-2.5, 2.7, -1.1), fc + Vec3::new(2.5, 2.85, 0.0), kind::WOOD, WALNUT));
        self.push(Inst::minmax(fc + Vec3::new(-1.6, 2.85, -0.6), fc + Vec3::new(1.6, HALL_H, 0.0), kind::STONE, [0.33, 0.3, 0.27]));
        self.push(Inst::minmax(fc + Vec3::new(-2.4, 0.0, -1.6), fc + Vec3::new(2.4, 0.08, -0.9), kind::STONE, [0.25, 0.23, 0.21]));
        self.push(Inst::minmax(fc + Vec3::new(-1.4, 0.0, -0.5), fc + Vec3::new(1.4, 2.0, -0.4), kind::PLAIN, [0.02, 0.015, 0.012]));
        for (i, (x, ang)) in [(-0.3f32, 0.3f32), (0.25, -0.25), (0.0, 0.05)].iter().enumerate() {
            let p = fc + Vec3::new(*x, 0.12 + i as f32 * 0.07, -0.75);
            self.push(Inst::new(p, Quat::from_rotation_y(*ang), Vec3::new(0.9, 0.14, 0.14), kind::WOOD, [0.08, 0.04, 0.02, 1.0], 0));
        }
        for k in 0..7 {
            let x = -0.45 + k as f32 * 0.15;
            let h = 0.25 + 0.2 * ((k as f32 * 1.7).sin() * 0.5 + 0.5);
            let mut flame = Inst::aabox(fc + Vec3::new(x, 0.25 + h * 0.5, -0.75), Vec3::new(0.09, h, 0.09), kind::EMISSIVE, [1.0, 0.42, 0.1]);
            flame.color[3] = 8.0;
            flame.param = 1;
            self.push(flame);
        }
        self.obstacle(fc + Vec3::new(-2.5, 0.0, -1.7), fc + Vec3::new(2.5, 3.0, 0.0), true);
        self.light(fc + Vec3::new(0.0, 0.8, -1.6), 11.0, [1.0, 0.45, 0.15], 3.0, LightKind::Flicker);
    }

    fn build_heaps(&mut self, rng: &mut StdRng) {
        for wing in 0..2 {
            let mut pool: Vec<u32> = (0..self.books.len() as u32)
                .filter(|&b| wing_of_category(self.series[self.books[b as usize].series as usize].category) == wing)
                .collect();
            pool.shuffle(rng);
            // A few small piles on the reading tables (west only).
            if wing == 0 {
                for (tx, tz) in [(-4.5f32, -2.6f32), (4.5, -2.6), (-4.5, 2.6), (4.5, 2.6)] {
                    let n = rng.gen_range(14..28).min(pool.len());
                    let books: Vec<u32> = pool.drain(..n).collect();
                    let base = Vec3::new(tx - 0.35, TABLE_H, tz);
                    self.add_heap(rng, base, 0.32, books, wing);
                }
            }
            let (x0, x1) = if wing == 0 { (-HALL_X + 0.8, VEIL_X - 0.8) } else { (VEIL_X + 0.8, HALL_X - 0.8) };
            // Choose spots first, then share the wing's books evenly between them,
            // so every pile goes through the settling code and none towers.
            let spot_r = 1.0;
            let mut spots: Vec<Vec3> = Vec::new();
            for _ in 0..20000 {
                let p = Vec3::new(rng.gen_range(x0 + spot_r..x1 - spot_r), 0.0, rng.gen_range(-HALL_Z + 0.8 + spot_r..HALL_Z - 0.8 - spot_r));
                if self.heap_spot_ok(p, spot_r, &spots) {
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
                self.add_heap(rng, p, radius, books, wing);
            }
        }
    }

    fn heap_spot_ok(&self, p: Vec3, r: f32, spots: &[Vec3]) -> bool {
        for o in &self.obstacles {
            let q = Vec2::new(p.x.clamp(o.min.x, o.max.x), p.z.clamp(o.min.z, o.max.z));
            if q.distance(Vec2::new(p.x, p.z)) < r + 1.1 {
                return false;
            }
        }
        if p.x.abs() < 6.0 && p.z.abs() < 8.5 {
            return false; // keep the reading room's middle (and the spawn point) walkable
        }
        if p.z > HALL_Z - 3.0 && p.x.abs() < 3.5 {
            return false; // hearth
        }
        if (p.x - VEIL_X).abs() < r + 1.0 {
            return false;
        }
        if spots.iter().any(|q| Vec2::new(q.x - p.x, q.z - p.z).length() < r * 2.0 + 1.1) {
            return false;
        }
        true
    }

    fn add_heap(&mut self, rng: &mut StdRng, base: Vec3, radius: f32, books: Vec<u32>, wing: usize) {
        let h = self.heaps.len() as u32;
        let cell = 0.035;
        let n = ((radius * 2.0 + 0.8) / cell).ceil() as i32;
        let mut hm = vec![0.0f32; (n * n) as usize];
        let idx = |p: Vec2| -> Option<usize> {
            let g = (p - Vec2::new(base.x, base.z)) / cell + Vec2::splat(n as f32 * 0.5);
            let (x, y) = (g.x.floor() as i32, g.y.floor() as i32);
            (x >= 0 && y >= 0 && x < n && y < n).then(|| (y * n + x) as usize)
        };
        let count = books.len().max(1) as f32;
        for (i, &b) in books.iter().enumerate() {
            let frac = i as f32 / count;
            let r = radius * (1.0 - 0.5 * frac) * rng.gen_range(0.0..1.0f32).sqrt();
            let a = rng.gen_range(0.0..1.0f32) * TAU;
            let c = Vec2::new(base.x, base.z) + Vec2::new(a.cos(), a.sin()) * r;
            let yaw = rng.gen_range(0.0..1.0f32) * TAU;
            let size = self.books[b as usize].size;
            let u = Vec2::new(yaw.cos(), yaw.sin());
            let v = Vec2::new(-u.y, u.x);
            let na = (size.y / cell).ceil() as i32 + 1;
            let nb = (size.z / cell).ceil() as i32 + 1;
            // Rest the book on what is below: find the support under each half
            // along both axes and tilt to fit, so piles settle rather than float.
            let mut pts = Vec::with_capacity(((na + 1) * (nb + 1)) as usize);
            let (mut ha, mut hb, mut hc, mut hd) = (0.0f32, 0.0f32, 0.0f32, 0.0f32);
            for ia in 0..=na {
                for ib in 0..=nb {
                    let fa = ia as f32 / na as f32 - 0.5;
                    let fb = ib as f32 / nb as f32 - 0.5;
                    let p = c + u * (fa * size.y) + v * (fb * size.z);
                    if let Some(k) = idx(p) {
                        let hh = hm[k];
                        if fa < 0.0 { ha = ha.max(hh) } else { hb = hb.max(hh) }
                        if fb < 0.0 { hc = hc.max(hh) } else { hd = hd.max(hh) }
                        pts.push((k, fa * size.y, fb * size.z));
                    }
                }
            }
            let slope_u = ((hb - ha) / (size.y * 0.5)).clamp(-0.45, 0.45);
            let slope_v = ((hd - hc) / (size.z * 0.5)).clamp(-0.45, 0.45);
            let mut y = ((ha + hb) * 0.5).max((hc + hd) * 0.5);
            // Never sink below any support point under the tilted plane.
            for &(k, su, sv) in &pts {
                y = y.max(hm[k] - su * slope_u - sv * slope_v);
            }
            for &(k, su, sv) in &pts {
                hm[k] = hm[k].max(y + su * slope_u + sv * slope_v + size.x);
            }
            let y = y + size.x * 0.5;
            let wy = Vec3::new(u.x, 0.0, u.y);
            let wz = Vec3::Y.cross(wy);
            let mut rot = quat_from_cols(Vec3::Y, wy, wz);
            rot = Quat::from_axis_angle(wy, slope_v.atan()) * Quat::from_axis_angle(wz, -slope_u.atan()) * rot;
            rot = Quat::from_axis_angle(wy, rng.gen_range(-0.03..0.03)) * rot;
            let info = &mut self.books[b as usize];
            info.heap = h;
            info.heap_pos = Vec3::new(c.x, base.y + y, c.y);
            info.heap_rot = rot;
        }
        self.heaps.push(Heap { base, radius, books, wing });
    }
}
