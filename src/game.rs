//! Game state: player, books in motion, upgrades, familiars, progression, save/load.

use crate::audio::{Audio, Sfx};
use crate::catalog::{self, CATEGORIES};
use crate::gfx::{Globals, GpuLight, Inst, MAX_LIGHTS, kind};
use crate::library::*;
use crate::nav::{Nav, Route};
use glam::{Mat4, Quat, Vec2, Vec3};
use rand::prelude::*;
use serde::{Deserialize, Serialize};
use std::f32::consts::PI;

pub const SEED: u64 = 0x4855_5348_464f_4c44;
const EYE: f32 = 1.62;
const SHELF_REACH: f32 = 4.2;
const PLAYER_R: f32 = 0.3;
const HAND_VISIBLE: usize = 8;

/// Vertical FOV: `v_deg` on wide windows, widened on narrow ones so the horizontal
/// FOV never drops below `v_deg` × 85/70 (85° at the default 70°).
fn vfov(aspect: f32, v_deg: f32) -> f32 {
    let min_h = (v_deg * 85.0 / 70.0).to_radians();
    let from_h = 2.0 * ((min_h * 0.5).tan() / aspect).atan();
    v_deg.to_radians().max(from_h).min(110f32.to_radians())
}

// ------------------------------------------------------------------ upgrades

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Upg {
    Arms,
    Slippers,
    SortSense,
    Bundle,
    Tidy,
    Lodestone,
    Aura,
    Familiar,
    Wings,
}

pub const UPGRADES: [Upg; 9] = [
    Upg::Arms,
    Upg::Slippers,
    Upg::SortSense,
    Upg::Tidy,
    Upg::Bundle,
    Upg::Lodestone,
    Upg::Familiar,
    Upg::Aura,
    Upg::Wings,
];

pub struct UpgDef {
    pub name: &'static str,
    pub blurb: &'static str,
    pub costs: &'static [u64],
}

pub fn upg_def(u: Upg) -> UpgDef {
    match u {
        Upg::Arms => UpgDef {
            name: "Deeper Arms",
            blurb: "Carry more books at once.",
            costs: &[20, 80, 250, 700],
        },
        Upg::Slippers => UpgDef {
            name: "Swift Slippers",
            blurb: "Pad through the stacks faster.",
            costs: &[30, 400],
        },
        Upg::SortSense => UpgDef {
            name: "Sort Sense",
            blurb: "The active book's exact slot pulses, and a compass points the way.",
            costs: &[60],
        },
        Upg::Tidy => UpgDef {
            name: "Tidy Arms",
            blurb: "Your armful sorts itself, and the book nearest its home rises to the top.",
            costs: &[90],
        },
        Upg::Bundle => UpgDef {
            name: "Series Bundle",
            blurb: "Filing one volume files every volume of that series you carry.",
            costs: &[120],
        },
        Upg::Lodestone => UpgDef {
            name: "Lodestone Grasp",
            blurb: "Gather from heaps at a distance; hold the button to keep gathering.",
            costs: &[300],
        },
        Upg::Familiar => UpgDef {
            name: "Paper Familiar",
            blurb: "A folded-paper crane that ferries books to their shelves on its own.",
            costs: &[250, 600, 1200, 2400, 4000],
        },
        Upg::Aura => UpgDef {
            name: "Shelving Aura",
            blurb: "Books you carry drift to their slots whenever you pass close by.",
            costs: &[500, 1500, 4000],
        },
        Upg::Wings => UpgDef {
            name: "Familiar Wings",
            blurb: "Familiars fly faster and carry more.",
            costs: &[800, 2000],
        },
    }
}

// ------------------------------------------------------------------ state

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum BookState {
    Heap,
    /// In the player's arms (possibly still flying there).
    Held,
    /// Flying to its slot.
    Filing,
    Shelved,
    /// Dangling from familiar i.
    Carried(u8),
}

#[derive(Clone, Copy)]
enum Dest {
    Hand,
    Shelf,
}

struct Flight {
    book: u32,
    from_pos: Vec3,
    from_rot: Quat,
    t: f32,
    dur: f32,
    delay: f32,
    dest: Dest,
}

#[derive(Clone, Copy, PartialEq)]
enum FamTask {
    Idle,
    ToHeap(u32),
    ToSlot,
}

struct Familiar {
    pos: Vec3,
    vel: Vec3,
    task: FamTask,
    carry: Vec<u32>,
    phase: f32,
    rest: f32,
}

struct Particle {
    pos: Vec3,
    vel: Vec3,
    life: f32,
    max: f32,
    color: [f32; 3],
    size: f32,
}

pub struct Toast {
    pub text: String,
    pub color: [f32; 3],
    pub age: f32,
    pub big: bool,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Aim {
    None,
    Heap {
        heap: u32,
        dist: f32,
    },
    Shelf {
        face: u32,
        row: u8,
        run: Option<Run>,
        dist: f32,
    },
}

#[derive(Default)]
pub struct Input {
    pub forward: f32,
    pub strafe: f32,
    pub sprint: bool,
    pub look: Vec2,
    pub primary_down: bool,
    pub primary_pressed: bool,
    pub cycle: i32,
}

pub struct Player {
    pub pos: Vec3,
    pub vel: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    pub bob: f32,
}

impl Player {
    pub fn eye(&self) -> Vec3 {
        self.pos + Vec3::Y * (EYE + (self.bob * 2.0).sin().abs() * 0.025)
    }
    pub fn forward(&self) -> Vec3 {
        Vec3::new(
            self.yaw.sin() * self.pitch.cos(),
            self.pitch.sin(),
            -self.yaw.cos() * self.pitch.cos(),
        )
    }
    pub fn right(&self) -> Vec3 {
        Vec3::new(self.yaw.cos(), 0.0, self.yaw.sin())
    }
}

#[derive(Serialize, Deserialize)]
struct SaveData {
    seed: u64,
    lumens: u64,
    earned: u64,
    upgrades: Vec<u32>,
    shelved: Vec<u32>,
    /// Books mid-flight to a shelf or carried by a familiar; they fly home on load
    /// so their rewards are paid through the normal path.
    #[serde(default)]
    pending: Vec<u32>,
    held: Vec<u32>,
    player: [f32; 5],
    east_unlocked: bool,
    play_time: f64,
}

pub struct Game {
    pub lib: Library,
    pub state: Vec<BookState>,
    pub pos: Vec<Vec3>,
    pub rot: Vec<Quat>,
    flights: Vec<Flight>,
    pub held: Vec<u32>,
    pub active: usize,
    /// A book the player chose by cycling; Tidy Arms leaves it active until it is
    /// filed or a new armful is gathered.
    manual_pick: Option<u32>,
    pub player: Player,
    pub lumens: u64,
    pub earned: u64,
    pub upg: [u32; UPGRADES.len()],
    pub series_shelved: Vec<u32>,
    pub category_shelved: Vec<u32>,
    pub wing_total: [u32; 2],
    pub wing_shelved: [u32; 2],
    pub east_unlocked: bool,
    veil: f32,
    familiars: Vec<Familiar>,
    particles: Vec<Particle>,
    motes: Vec<(Vec3, f32)>,
    pub toasts: Vec<Toast>,
    pub aim: Aim,
    pub time: f32,
    pub play_time: f64,
    grab_cooldown: f32,
    aura_budget: f32,
    pub lantern_glow: Vec<f32>,
    rng: StdRng,
    pub hint: Option<(String, f32)>,
    // Per-frame instance scratch.
    pub dyn_insts: Vec<Inst>,
    pub trans_insts: Vec<Inst>,
    pub held_insts: Vec<Inst>,
    pub heap_bounds: Vec<(Vec3, Vec3)>,
    nav: Nav,
    /// Sort Sense: the way to the active book's slot.
    pub sense_route: Option<Route>,
    /// Vertical field of view on wide windows, in degrees (a setting).
    pub fov: f32,
    pub dirty_save: bool,
    /// False in scripted runs so they never touch the player's save.
    pub persist: bool,
}

/// A highlight colour with the series' hue at a fixed brightness: the colour is
/// scaled to full value, then lifted toward white until its luminance reaches a
/// floor, so dark browns and blacks signal as clearly as golds against the wood.
fn signal_color(c: [f32; 3]) -> [f32; 3] {
    let m = c[0].max(c[1]).max(c[2]).max(1e-3);
    let n = c.map(|x| x / m);
    let lum = 0.2126 * n[0] + 0.7152 * n[1] + 0.0722 * n[2];
    let k = ((0.45 - lum) / (1.0 - lum)).max(0.0);
    n.map(|x| x + (1.0 - x) * k)
}

pub fn upg_index(u: Upg) -> usize {
    UPGRADES.iter().position(|&x| x == u).unwrap()
}

impl Game {
    pub fn new() -> Self {
        let lib = Library::generate(SEED);
        let n = lib.books.len();
        let mut wing_total = [0u32; 2];
        for b in &lib.books {
            wing_total[wing_of_category(lib.series[b.series as usize].category)] += 1;
        }
        let pos = lib.books.iter().map(|b| b.heap_pos).collect();
        let rot = lib.books.iter().map(|b| b.heap_rot).collect();
        let mut rng = StdRng::seed_from_u64(0x5eed);
        let motes = (0..700)
            .map(|_| {
                (
                    Vec3::new(
                        rng.gen_range(-HALL_X..HALL_X),
                        rng.gen_range(0.2..6.0),
                        rng.gen_range(-HALL_Z..HALL_Z),
                    ),
                    rng.gen_range(0.0..1.0f32) * 100.0,
                )
            })
            .collect();
        let mut g = Game {
            state: vec![BookState::Heap; n],
            pos,
            rot,
            flights: Vec::new(),
            held: Vec::new(),
            active: 0,
            manual_pick: None,
            player: Player {
                pos: Vec3::new(0.0, 0.0, 6.5),
                vel: Vec3::ZERO,
                yaw: 0.0,
                pitch: -0.05,
                bob: 0.0,
            },
            lumens: 0,
            earned: 0,
            upg: [0; UPGRADES.len()],
            series_shelved: vec![0; lib.series.len()],
            category_shelved: vec![0; CATEGORIES.len()],
            wing_total,
            wing_shelved: [0; 2],
            east_unlocked: false,
            veil: 1.0,
            familiars: Vec::new(),
            particles: Vec::new(),
            motes,
            toasts: Vec::new(),
            aim: Aim::None,
            time: 0.0,
            play_time: 0.0,
            grab_cooldown: 0.0,
            aura_budget: 0.0,
            lantern_glow: vec![0.0; CATEGORIES.len()],
            rng,
            hint: None,
            dyn_insts: Vec::with_capacity(n + 2048),
            trans_insts: Vec::with_capacity(n + 64),
            held_insts: Vec::new(),
            heap_bounds: Vec::new(),
            nav: Nav::new(&lib.obstacles),
            sense_route: None,
            fov: 70.0,
            dirty_save: false,
            persist: true,
            lib,
        };
        g.recompute_heap_bounds();
        g
    }

    pub fn level(&self, u: Upg) -> u32 {
        self.upg[upg_index(u)]
    }

    pub fn capacity(&self) -> usize {
        [6, 12, 24, 48, 96][self.level(Upg::Arms) as usize]
    }

    fn heap_reach(&self) -> f32 {
        if self.level(Upg::Lodestone) > 0 { 11.0 } else { 3.6 }
    }

    pub fn total_books(&self) -> u32 {
        self.lib.books.len() as u32
    }

    pub fn total_shelved(&self) -> u32 {
        self.wing_shelved[0] + self.wing_shelved[1]
    }

    pub fn next_cost(&self, u: Upg) -> Option<u64> {
        upg_def(u).costs.get(self.level(u) as usize).copied()
    }

    pub fn buy(&mut self, u: Upg, audio: &mut Audio) -> bool {
        let Some(cost) = self.next_cost(u) else { return false };
        if self.lumens < cost {
            return false;
        }
        self.lumens -= cost;
        self.upg[upg_index(u)] += 1;
        audio.play(Sfx::Purchase, 0.7);
        if u == Upg::Familiar {
            self.spawn_familiar();
        }
        if u == Upg::Tidy {
            let prev = self.active_book();
            self.held.sort();
            self.follow_active(prev);
        }
        let def = upg_def(u);
        let name = if def.costs.len() > 1 {
            let roman = ["I", "II", "III", "IV", "V"];
            format!(
                "{} {}",
                def.name,
                roman[(self.level(u) as usize - 1).min(roman.len() - 1)]
            )
        } else {
            def.name.to_string()
        };
        self.toast(format!("{name} learned"), [0.95, 0.8, 0.45], false);
        self.dirty_save = true;
        true
    }

    fn spawn_familiar(&mut self) {
        let p = self.player.eye() + self.player.forward() * 1.0 + Vec3::Y * 0.5;
        self.familiars.push(Familiar {
            pos: p,
            vel: Vec3::ZERO,
            task: FamTask::Idle,
            carry: Vec::new(),
            phase: self.rng.gen_range(0.0..1.0f32) * 10.0,
            rest: 0.5,
        });
    }

    pub fn toast(&mut self, text: String, color: [f32; 3], big: bool) {
        self.toasts.push(Toast {
            text,
            color,
            age: 0.0,
            big,
        });
    }

    /// West Wing books still to shelve before the veil lifts (at 60%).
    pub fn veil_books_left(&self) -> u32 {
        let need = (self.wing_total[0] as f32 * 0.6).ceil() as u32;
        need.saturating_sub(self.wing_shelved[0])
    }

    pub fn wing_unlocked(&self, wing: usize) -> bool {
        wing == 0 || self.east_unlocked
    }

    fn recompute_heap_bounds(&mut self) {
        self.heap_bounds = self
            .lib
            .heaps
            .iter()
            .map(|h| {
                let mut mn = Vec3::splat(f32::MAX);
                let mut mx = Vec3::splat(f32::MIN);
                for &b in &h.books {
                    let r = self.lib.books[b as usize].size.max_element() * 0.5;
                    mn = mn.min(self.pos[b as usize] - Vec3::splat(r));
                    mx = mx.max(self.pos[b as usize] + Vec3::splat(r));
                }
                (mn, mx)
            })
            .collect();
    }

    fn update_heap_bound(&mut self, h: usize) {
        let mut mn = Vec3::splat(f32::MAX);
        let mut mx = Vec3::splat(f32::MIN);
        for &b in &self.lib.heaps[h].books {
            let r = self.lib.books[b as usize].size.max_element() * 0.5;
            mn = mn.min(self.lib.books[b as usize].heap_pos - Vec3::splat(r));
            mx = mx.max(self.lib.books[b as usize].heap_pos + Vec3::splat(r));
        }
        self.heap_bounds[h] = (mn, mx);
    }

    // -------------------------------------------------------------- update

    pub fn update(&mut self, dt: f32, input: &Input, audio: &mut Audio, can_play: bool) {
        self.time += dt;
        if can_play {
            self.play_time += dt as f64;
        }
        let bob_before = self.player.bob;
        self.update_player(dt, input);
        // A soft footfall each time the walking bob touches down.
        let footfall = |bob: f32| (bob * 2.0 / PI).floor();
        if can_play && footfall(self.player.bob) != footfall(bob_before) {
            audio.play(Sfx::Step, if input.sprint { 0.16 } else { 0.12 });
        }
        self.aim = if can_play { self.compute_aim() } else { Aim::None };

        if can_play {
            if input.cycle != 0 && !self.held.is_empty() {
                let n = self.held.len() as i32;
                self.active = ((self.active as i32 + input.cycle).rem_euclid(n)) as usize;
                self.manual_pick = self.active_book();
                audio.play(Sfx::Tick, 0.25);
            }
            self.grab_cooldown -= dt;
            let repeat = self.level(Upg::Lodestone) > 0 && input.primary_down && self.grab_cooldown <= 0.0;
            if input.primary_pressed || repeat {
                self.interact(audio, input.primary_pressed);
            }
        }
        if self.manual_pick.is_some_and(|b| !self.held.contains(&b)) {
            self.manual_pick = None;
        }
        let tidy = self.level(Upg::Tidy) > 0;
        let sense = self.level(Upg::SortSense) > 0;
        if (tidy || sense) && !self.held.is_empty() {
            self.nav.update(self.player.pos, self.east_unlocked);
        }
        if tidy && self.manual_pick.is_none() {
            self.pick_nearest_active();
        }
        self.sense_route = match self.active_book() {
            Some(b) if sense => self.nav.route(self.player.pos, self.approach(b)),
            _ => None,
        };
        self.update_aura(dt);
        self.update_flights(dt, audio);
        self.update_familiars(dt);
        self.update_particles(dt);

        // Lanterns ease toward their category completion.
        for c in 0..CATEGORIES.len() {
            let target = if self.category_shelved[c] == self.lib.category_books[c] {
                1.0
            } else {
                0.3 * self.category_shelved[c] as f32 / self.lib.category_books[c].max(1) as f32
            };
            self.lantern_glow[c] += (target - self.lantern_glow[c]) * (1.0 - (-dt * 1.5).exp());
        }
        if self.east_unlocked {
            self.veil = (self.veil - dt * 0.25).max(0.0);
        }
        for t in &mut self.toasts {
            t.age += dt;
        }
        self.toasts.retain(|t| t.age < if t.big { 6.0 } else { 3.5 });
        if let Some((_, t)) = &mut self.hint {
            *t -= dt;
            if *t <= 0.0 {
                self.hint = None;
            }
        }
    }

    fn update_player(&mut self, dt: f32, input: &Input) {
        let p = &mut self.player;
        p.yaw += input.look.x;
        p.pitch = (p.pitch - input.look.y).clamp(-1.45, 1.45);
        let fwd = Vec3::new(p.yaw.sin(), 0.0, -p.yaw.cos());
        let right = p.right();
        let mut wish = fwd * input.forward + right * input.strafe;
        if wish.length_squared() > 1.0 {
            wish = wish.normalize();
        }
        let base = [4.0, 5.5, 7.0][self.upg[upg_index(Upg::Slippers)] as usize];
        let speed = if input.sprint { base * 1.6 } else { base };
        let k = 1.0 - (-dt * 10.0).exp();
        p.vel = p.vel.lerp(wish * speed, k);
        let mut np = p.pos + p.vel * dt;
        // Collide with obstacles (circle vs AABB in xz).
        for _ in 0..2 {
            for o in &self.lib.obstacles {
                let q = Vec2::new(np.x.clamp(o.min.x, o.max.x), np.z.clamp(o.min.z, o.max.z));
                let d = Vec2::new(np.x, np.z) - q;
                let l = d.length();
                if l < PLAYER_R {
                    let push = if l > 1e-5 {
                        d / l * (PLAYER_R - l)
                    } else {
                        Vec2::new(PLAYER_R, 0.0)
                    };
                    np.x += push.x;
                    np.z += push.y;
                }
            }
        }
        np.x = np.x.clamp(-HALL_X + PLAYER_R + 0.1, HALL_X - PLAYER_R - 0.1);
        np.z = np.z.clamp(-HALL_Z + PLAYER_R + 0.1, HALL_Z - PLAYER_R - 0.1);
        let veil_x = VEIL_X - PLAYER_R - 0.2;
        let at_veil = !self.east_unlocked && np.x > veil_x && wish.x > 0.3;
        if !self.east_unlocked {
            np.x = np.x.min(veil_x);
        }
        let moved = Vec2::new(np.x - p.pos.x, np.z - p.pos.z).length();
        p.bob += moved * 1.4;
        p.pos = np;
        if at_veil && self.hint.is_none() {
            let left = self.veil_books_left();
            self.hint = Some((
                format!("The veil holds. Shelve {left} more West Wing books to lift it."),
                2.5,
            ));
        }
    }

    /// True if nothing that blocks rays lies between `a` and `b`.
    pub fn line_of_sight(&self, a: Vec3, b: Vec3) -> bool {
        let d = b - a;
        let len = d.length();
        let dir = d / len.max(1e-5);
        !self
            .lib
            .obstacles
            .iter()
            .any(|o| o.blocks_ray && Self::ray_aabb(a, dir, o.min, o.max).is_some_and(|t| t < len - 0.15))
    }

    fn ray_aabb(o: Vec3, d: Vec3, mn: Vec3, mx: Vec3) -> Option<f32> {
        let inv = d.recip();
        let t1 = (mn - o) * inv;
        let t2 = (mx - o) * inv;
        let tmin = t1.min(t2).max_element();
        let tmax = t1.max(t2).min_element();
        (tmax >= tmin.max(0.0)).then_some(tmin.max(0.0))
    }

    fn ray_obb(o: Vec3, d: Vec3, pos: Vec3, rot: Quat, size: Vec3) -> Option<f32> {
        let inv = rot.inverse();
        let lo = inv * (o - pos);
        let ld = inv * d;
        Self::ray_aabb(lo, ld, -size * 0.5, size * 0.5)
    }

    fn compute_aim(&self) -> Aim {
        let o = self.player.eye();
        let d = self.player.forward();
        let mut block = f32::MAX;
        for ob in &self.lib.obstacles {
            if ob.blocks_ray
                && let Some(t) = Self::ray_aabb(o, d, ob.min, ob.max)
            {
                block = block.min(t);
            }
        }
        let mut best = Aim::None;
        let mut best_t = f32::MAX;
        // Shelves.
        for (fi, f) in self.lib.faces.iter().enumerate() {
            let dn = d.dot(f.normal);
            if dn >= -1e-4 {
                continue;
            }
            let t = (f.center - o).dot(f.normal) / dn;
            // The bookcase's own blocking box sits a few cm proud of the shelf front;
            // that gap grows along the ray at grazing angles, so scale the allowance.
            let allowance = (0.06 / -dn + 0.01).min(0.3);
            if !(0.0..=SHELF_REACH).contains(&t) || t > block + allowance || t > best_t {
                continue;
            }
            let p = o + d * t;
            let x = (p - f.center).dot(f.right);
            if x.abs() > USABLE_W * 0.5 + 0.02 {
                continue;
            }
            let wing = wing_of_category(f.category);
            if !self.wing_unlocked(wing) {
                continue;
            }
            let mut row = None;
            for r in 0..ROWS {
                let y0 = row_floor_y(r);
                if p.y >= y0 - 0.02 && p.y < y0 + SHELF_STEP - BOARD_T {
                    row = Some(r);
                }
            }
            let Some(row) = row else { continue };
            let run = self.lib.runs[fi][row]
                .iter()
                .copied()
                .find(|r| x >= r.x0 - 0.03 && x <= r.x1 + 0.03);
            best = Aim::Shelf {
                face: fi as u32,
                row: row as u8,
                run,
                dist: t,
            };
            best_t = t;
        }
        // Heaps (test individual books inside heaps whose bounds the ray crosses).
        let reach = self.heap_reach();
        for (hi, h) in self.lib.heaps.iter().enumerate() {
            if h.books.is_empty() || !self.wing_unlocked(h.wing) {
                continue;
            }
            let (mn, mx) = self.heap_bounds[hi];
            let Some(t0) = Self::ray_aabb(o, d, mn, mx) else {
                continue;
            };
            if t0 > reach || t0 > best_t || t0 > block + 0.3 {
                continue;
            }
            for &b in &h.books {
                let info = &self.lib.books[b as usize];
                if let Some(t) = Self::ray_obb(o, d, info.heap_pos, info.heap_rot, info.size * 1.2)
                    && t < best_t
                    && t <= reach
                {
                    best_t = t;
                    best = Aim::Heap {
                        heap: hi as u32,
                        dist: t,
                    };
                }
            }
        }
        best
    }

    fn interact(&mut self, audio: &mut Audio, fresh_press: bool) {
        match self.aim {
            Aim::Heap { heap, .. } => {
                let room = self.capacity().saturating_sub(self.held.len());
                if room == 0 {
                    if fresh_press {
                        audio.play(Sfx::Nope, 0.4);
                        self.hint = Some(("Your arms are full — file some books first.".into(), 2.0));
                    }
                    self.grab_cooldown = 0.3;
                    return;
                }
                let h = heap as usize;
                let take = room.min(self.lib.heaps[h].books.len());
                for i in 0..take {
                    let b = self.lib.heaps[h].books.pop().unwrap();
                    self.state[b as usize] = BookState::Held;
                    self.held.push(b);
                    self.start_flight(b, Dest::Hand, 0.32 + i as f32 * 0.004, i as f32 * 0.025);
                }
                self.update_heap_bound(h);
                self.manual_pick = None;
                if self.level(Upg::Tidy) > 0 {
                    let prev = self.active_book();
                    self.held.sort();
                    self.follow_active(prev);
                } else {
                    self.active = self.held.len() - 1;
                }
                audio.play(Sfx::Rustle, 0.6);
                self.grab_cooldown = 0.35;
            }
            Aim::Shelf { run: Some(run), .. } => {
                if !fresh_press {
                    return;
                }
                let s = run.series;
                let matching: Vec<usize> = (0..self.held.len())
                    .filter(|&i| self.lib.books[self.held[i] as usize].series == s)
                    .collect();
                if matching.is_empty() {
                    let ser = &self.lib.series[s as usize];
                    let done = self.series_shelved[s as usize] == ser.volumes;
                    let msg = if done {
                        format!("“{}” is complete.", ser.title)
                    } else {
                        format!("You aren't carrying any of “{}”.", ser.title)
                    };
                    self.hint = Some((msg, 2.0));
                    audio.play(Sfx::Nope, 0.3);
                    return;
                }
                let chosen: Vec<usize> = if self.level(Upg::Bundle) > 0 {
                    matching
                } else if matching.contains(&self.active) {
                    vec![self.active]
                } else {
                    vec![matching[0]]
                };
                let books: Vec<u32> = chosen.iter().map(|&i| self.held[i]).collect();
                let prev = self.active_book();
                self.held.retain(|b| !books.contains(b));
                self.follow_active(prev);
                for (i, &b) in books.iter().enumerate() {
                    self.state[b as usize] = BookState::Filing;
                    self.start_flight(b, Dest::Shelf, 0.42, i as f32 * 0.06);
                }
                audio.play(Sfx::Whoosh, 0.35);
            }
            _ => {}
        }
    }

    fn start_flight(&mut self, book: u32, dest: Dest, dur: f32, delay: f32) {
        self.flights.retain(|f| f.book != book);
        let (from_pos, from_rot) = (self.pos[book as usize], self.rot[book as usize]);
        self.flights.push(Flight {
            book,
            from_pos,
            from_rot,
            t: 0.0,
            dur,
            delay,
            dest,
        });
    }

    fn hand_xform(&self, slot: usize) -> (Vec3, Quat) {
        let eye = self.player.eye();
        let f = self.player.forward();
        let r = self.player.right();
        let u = r.cross(f);
        let anchor = eye + f * 0.7 + r * 0.32 - u * 0.4;
        let rot = Quat::from_mat3(&glam::Mat3::from_cols(u, -r, -f));
        (anchor + u * (slot as f32 * 0.05), rot)
    }

    fn update_flights(&mut self, dt: f32, audio: &mut Audio) {
        let mut landed = Vec::new();
        let hand = self.hand_xform(HAND_VISIBLE / 2);
        let mut flights = std::mem::take(&mut self.flights);
        flights.retain_mut(|f| {
            if f.delay > 0.0 {
                f.delay -= dt;
                return true;
            }
            f.t += dt / f.dur;
            let b = f.book as usize;
            let (to_pos, to_rot) = match f.dest {
                Dest::Hand => hand,
                Dest::Shelf => (self.lib.books[b].slot_pos, self.lib.books[b].slot_rot),
            };
            let e = {
                let t = f.t.min(1.0);
                t * t * (3.0 - 2.0 * t)
            };
            let arc = match f.dest {
                Dest::Hand => 0.35,
                Dest::Shelf => 0.25,
            };
            self.pos[b] = f.from_pos.lerp(to_pos, e) + Vec3::Y * arc * (PI * e).sin();
            self.rot[b] = f.from_rot.slerp(to_rot, e);
            if f.t >= 1.0 {
                landed.push((f.book, f.dest));
                return false;
            }
            true
        });
        self.flights = flights;
        let mut thunks = 0;
        for (b, dest) in landed {
            if let Dest::Shelf = dest {
                self.on_shelved(b, audio, &mut thunks);
            }
        }
    }

    fn on_shelved(&mut self, b: u32, audio: &mut Audio, thunks: &mut u32) {
        let bi = b as usize;
        self.state[bi] = BookState::Shelved;
        self.pos[bi] = self.lib.books[bi].slot_pos;
        self.rot[bi] = self.lib.books[bi].slot_rot;
        let s = self.lib.books[bi].series as usize;
        let cat = self.lib.series[s].category;
        let wing = wing_of_category(cat);
        self.series_shelved[s] += 1;
        self.category_shelved[cat] += 1;
        self.wing_shelved[wing] += 1;
        self.lumens += 1;
        self.earned += 1;
        self.dirty_save = true;
        if *thunks < 3 {
            audio.play(Sfx::Thunk, 0.5);
            *thunks += 1;
        }
        let col = self.lib.series[s].color;
        let bright = [col[0] * 3.0 + 0.4, col[1] * 3.0 + 0.35, col[2] * 3.0 + 0.2];
        self.burst(self.lib.books[bi].slot_pos, 5, bright, 0.5);

        let ser = &self.lib.series[s];
        if self.series_shelved[s] == ser.volumes {
            let bonus = ser.volumes as u64;
            self.lumens += bonus;
            self.earned += bonus;
            if ser.volumes > 1 {
                audio.play_pitched(Sfx::Chime, 0.45, ser.volumes as usize);
                let title = ser.title.clone();
                self.toast(
                    format!("Series complete: {title}  (+{bonus} ✦)"),
                    [0.95, 0.85, 0.6],
                    false,
                );
                for &ob in &self.lib.series_books[s].clone() {
                    let p = self.lib.books[ob as usize].slot_pos;
                    self.burst(p, 6, [1.6, 1.2, 0.5], 0.9);
                }
            }
        }
        if self.category_shelved[cat] == self.lib.category_books[cat] {
            self.lumens += 100;
            self.earned += 100;
            audio.play(Sfx::Fanfare, 0.7);
            let c = catalog::category_color(cat);
            self.toast(
                format!("{} is fully catalogued!  (+100 ✦)", CATEGORIES[cat].name),
                c,
                true,
            );
            let center = self.lib.category_center[cat];
            self.burst(center + Vec3::Y * 1.5, 120, [c[0] * 4.0, c[1] * 4.0, c[2] * 4.0], 2.0);
        }
        if !self.east_unlocked && self.veil_books_left() == 0 {
            self.east_unlocked = true;
            audio.play(Sfx::Fanfare, 0.8);
            self.toast("The veil over the East Wing lifts…".into(), [0.75, 0.6, 1.0], true);
        }
        if self.total_shelved() == self.total_books() {
            self.toast(
                "Every book is home. The library sighs, contented.".into(),
                [1.0, 0.95, 0.8],
                true,
            );
        }
    }

    fn burst(&mut self, p: Vec3, n: usize, color: [f32; 3], life: f32) {
        for _ in 0..n {
            let v = Vec3::new(
                self.rng.gen_range(-1.0..1.0),
                self.rng.gen_range(0.2..1.5),
                self.rng.gen_range(-1.0..1.0),
            );
            self.particles.push(Particle {
                pos: p + v * 0.05,
                vel: v * 0.6,
                life: 0.0,
                max: life * self.rng.gen_range(0.6..1.2),
                color,
                size: self.rng.gen_range(0.012..0.03),
            });
        }
    }

    fn update_particles(&mut self, dt: f32) {
        for p in &mut self.particles {
            p.life += dt;
            p.pos += p.vel * dt;
            p.vel *= 1.0 - dt * 1.5;
            p.vel.y += dt * 0.15;
        }
        self.particles.retain(|p| p.life < p.max);
    }

    /// Where to stand to file book `b`: in the aisle in front of its slot, at floor level.
    fn approach(&self, b: u32) -> Vec3 {
        let info = &self.lib.books[b as usize];
        let p = info.slot_pos + self.lib.faces[info.face as usize].normal * 0.75;
        Vec3::new(p.x, 0.0, p.z)
    }

    /// Make the held book nearest its home on foot the active one. A bookcase's far
    /// side is a walk around the row, not the width of the case.
    fn pick_nearest_active(&mut self) {
        if self.held.is_empty() {
            return;
        }
        let from = self.player.pos;
        let cost = |g: &Self, b: u32| g.nav.distance(from, g.approach(b));
        let mut best = (f32::MAX, 0);
        for (i, &b) in self.held.iter().enumerate() {
            let d = cost(self, b);
            if d < best.0 {
                best = (d, i);
            }
        }
        // Hysteresis, so near-ties don't flicker as you walk.
        let current = self.active_book().map_or(f32::MAX, |b| cost(self, b));
        if best.0 < current - 0.6 {
            self.active = best.1;
        }
    }

    fn update_aura(&mut self, dt: f32) {
        let lvl = self.level(Upg::Aura);
        if lvl == 0 {
            return;
        }
        let (radius, rate) = [(0.0, 0.0), (3.0, 6.0), (6.0, 15.0), (11.0, 40.0)][lvl as usize];
        self.aura_budget = (self.aura_budget + rate * dt).min(rate * 0.5 + 1.0);
        let eye = self.player.eye();
        let prev = self.active_book();
        let mut i = 0;
        while i < self.held.len() && self.aura_budget >= 1.0 {
            let b = self.held[i];
            let arrived = !self.flights.iter().any(|f| f.book == b);
            if arrived && self.lib.books[b as usize].slot_pos.distance(eye) < radius {
                self.held.remove(i);
                self.state[b as usize] = BookState::Filing;
                self.start_flight(b, Dest::Shelf, 0.6, 0.0);
                self.aura_budget -= 1.0;
            } else {
                i += 1;
            }
        }
        self.follow_active(prev);
    }

    fn update_familiars(&mut self, dt: f32) {
        let wings = self.level(Upg::Wings) as usize;
        let speed = [2.6, 4.0, 6.0][wings];
        let cap = [2, 4, 8][wings];
        let cruise = 4.9f32;
        let mut fams = std::mem::take(&mut self.familiars);
        for (fi, fam) in fams.iter_mut().enumerate() {
            fam.phase += dt * 9.0;
            if fam.rest > 0.0 {
                fam.rest -= dt;
                fam.vel *= 1.0 - dt * 3.0;
                fam.pos += fam.vel * dt + Vec3::Y * (fam.phase * 0.3).sin() * 0.002;
                continue;
            }
            // Choose a task.
            if fam.task == FamTask::Idle {
                if !fam.carry.is_empty() {
                    fam.task = FamTask::ToSlot;
                } else {
                    let mut best: Option<(f32, u32)> = None;
                    for (hi, h) in self.lib.heaps.iter().enumerate() {
                        if h.books.is_empty() || !self.wing_unlocked(h.wing) {
                            continue;
                        }
                        let d = h.base.distance(fam.pos) + self.rng.gen_range(0.0..4.0);
                        if best.is_none_or(|(bd, _)| d < bd) {
                            best = Some((d, hi as u32));
                        }
                    }
                    match best {
                        Some((_, h)) => fam.task = FamTask::ToHeap(h),
                        None => {
                            fam.rest = 1.0;
                            continue;
                        }
                    }
                }
            }
            let target = match fam.task {
                FamTask::ToHeap(h) => {
                    let heap = &self.lib.heaps[h as usize];
                    let top = heap
                        .books
                        .last()
                        .map(|&b| self.lib.books[b as usize].heap_pos)
                        .unwrap_or(heap.base);
                    top + Vec3::Y * 0.45
                }
                FamTask::ToSlot => {
                    let b = fam.carry[0] as usize;
                    let f = &self.lib.faces[self.lib.books[b].face as usize];
                    self.lib.books[b].slot_pos + f.normal * 0.55 + Vec3::Y * 0.1
                }
                FamTask::Idle => fam.pos,
            };
            let to = target - fam.pos;
            let horiz = Vec2::new(to.x, to.z).length();
            let goal = if horiz > 1.2 {
                Vec3::new(target.x, cruise.max(target.y), target.z)
            } else {
                target
            };
            // Climb first when far below cruise altitude and far away.
            let goal = if horiz > 1.2 && fam.pos.y < cruise - 0.6 {
                Vec3::new(fam.pos.x + to.x * 0.15, cruise, fam.pos.z + to.z * 0.15)
            } else {
                goal
            };
            let dir = goal - fam.pos;
            let desired = dir.normalize_or_zero() * speed * (dir.length() / 0.8).min(1.0);
            fam.vel = fam.vel.lerp(desired, 1.0 - (-dt * 4.0).exp());
            fam.pos += fam.vel * dt;
            if fam.pos.distance(target) < 0.25 {
                match fam.task {
                    FamTask::ToHeap(h) => {
                        let heap = &mut self.lib.heaps[h as usize];
                        let n = cap.min(heap.books.len());
                        for _ in 0..n {
                            let b = heap.books.pop().unwrap();
                            self.state[b as usize] = BookState::Carried(fi as u8);
                            fam.carry.push(b);
                        }
                        self.update_heap_bound(h as usize);
                        // Visit slots in shelf order.
                        fam.carry.sort();
                        fam.task = FamTask::Idle;
                        fam.rest = 0.25;
                    }
                    FamTask::ToSlot => {
                        let b = fam.carry.remove(0);
                        self.state[b as usize] = BookState::Filing;
                        self.flights.retain(|f| f.book != b);
                        self.flights.push(Flight {
                            book: b,
                            from_pos: self.pos[b as usize],
                            from_rot: self.rot[b as usize],
                            t: 0.0,
                            dur: 0.35,
                            delay: 0.0,
                            dest: Dest::Shelf,
                        });
                        fam.task = FamTask::Idle;
                        fam.rest = 0.15;
                    }
                    FamTask::Idle => {}
                }
            }
            // Carried books dangle beneath.
            for (k, &b) in fam.carry.iter().enumerate() {
                let sway = (fam.phase * 0.5 + k as f32).sin() * 0.1;
                self.pos[b as usize] = fam.pos - Vec3::Y * (0.18 + k as f32 * 0.05);
                self.rot[b as usize] =
                    Quat::from_rotation_y(fam.phase * 0.05 + k as f32) * Quat::from_rotation_z(PI * 0.5 + sway);
            }
        }
        self.familiars = fams;
    }

    // -------------------------------------------------------------- scripting aids

    fn look_at(&mut self, from: Vec3, target: Vec3) {
        self.player.pos = from;
        let d = (target - self.player.eye()).normalize();
        self.player.yaw = d.x.atan2(-d.z);
        self.player.pitch = d.y.asin();
    }

    /// Stand beside the nearest non-empty, reachable heap and look at its top.
    pub fn script_face_heap(&mut self) {
        let p = self.player.pos;
        let best = (0..self.lib.heaps.len())
            .filter(|&h| !self.lib.heaps[h].books.is_empty() && self.wing_unlocked(self.lib.heaps[h].wing))
            .min_by(|&a, &b| {
                self.lib.heaps[a]
                    .base
                    .distance(p)
                    .total_cmp(&self.lib.heaps[b].base.distance(p))
            });
        if let Some(h) = best {
            let heap = &self.lib.heaps[h];
            let top = self.lib.books[*heap.books.last().unwrap() as usize].heap_pos;
            let mut dir = Vec3::new(p.x - heap.base.x, 0.0, p.z - heap.base.z).normalize_or_zero();
            if dir == Vec3::ZERO {
                dir = Vec3::Z;
            }
            let from = Vec3::new(heap.base.x, 0.0, heap.base.z) + dir * (heap.radius + 1.0);
            self.look_at(from, top);
        }
    }

    /// Instantly shelve a fraction of a category (debug/screenshot aid; no rewards).
    /// Unlike play, this removes books from the middle of heaps, so books above
    /// them are left hovering; use it for shelf shots, not heap shots.
    pub fn script_fill_category(&mut self, cat: usize, frac: f32) {
        let ids: Vec<u32> = (0..self.lib.books.len() as u32)
            .filter(|&b| self.lib.series[self.lib.books[b as usize].series as usize].category == cat)
            .collect();
        let n = (ids.len() as f32 * frac) as usize;
        for &b in ids.iter().take(n) {
            let bi = b as usize;
            if self.state[bi] != BookState::Heap {
                continue;
            }
            self.state[bi] = BookState::Shelved;
            self.pos[bi] = self.lib.books[bi].slot_pos;
            self.rot[bi] = self.lib.books[bi].slot_rot;
            let s = self.lib.books[bi].series as usize;
            self.series_shelved[s] += 1;
            self.category_shelved[cat] += 1;
            self.wing_shelved[wing_of_category(cat)] += 1;
        }
        for h in &mut self.lib.heaps {
            h.books.retain(|&b| self.state[b as usize] == BookState::Heap);
        }
        self.recompute_heap_bounds();
    }

    /// Put `n` books of a category, spread across its shelves, into the arms (debug/screenshot
    /// aid). Like `script_fill_category`, it pulls books from the middle of heaps.
    pub fn script_take(&mut self, cat: usize, n: usize) {
        let ids: Vec<u32> = (0..self.lib.books.len() as u32)
            .filter(|&b| {
                self.state[b as usize] == BookState::Heap
                    && self.lib.series[self.lib.books[b as usize].series as usize].category == cat
            })
            .collect();
        let step = (ids.len() / n.max(1)).max(1);
        let room = self.capacity().saturating_sub(self.held.len());
        for &b in ids.iter().step_by(step).take(n.min(room)) {
            self.state[b as usize] = BookState::Held;
            self.held.push(b);
        }
        for h in &mut self.lib.heaps {
            h.books.retain(|&b| self.state[b as usize] == BookState::Heap);
        }
        self.recompute_heap_bounds();
    }

    /// Look at familiar `i` from a little way off.
    pub fn script_face_familiar(&mut self, i: usize) {
        if let Some(f) = self.familiars.get(i) {
            let p = f.pos;
            let from = Vec3::new(p.x + 1.2, 0.0, p.z + 1.2);
            self.look_at(from, p);
        }
    }

    /// Stand in front of the active book's slot and look at it.
    pub fn script_face_active_slot(&mut self) {
        if let Some(b) = self.active_book() {
            let info = &self.lib.books[b as usize];
            let f = &self.lib.faces[info.face as usize];
            let from = Vec3::new(info.slot_pos.x, 0.0, info.slot_pos.z) + f.normal * 1.8;
            self.look_at(from, info.slot_pos);
        }
    }

    // -------------------------------------------------------------- rendering

    pub fn globals(&self, aspect: f32) -> Globals {
        let eye = self.player.eye();
        let mut lights = [GpuLight::default(); MAX_LIGHTS];
        let mut n = 0;
        let mut push = |p: Vec3, r: f32, c: [f32; 3], i: f32| {
            if n < MAX_LIGHTS {
                lights[n] = GpuLight {
                    pos_radius: [p.x, p.y, p.z, r],
                    color: [c[0], c[1], c[2], i],
                };
                n += 1;
            }
        };
        let t = self.time;
        for (li, l) in self.lib.lights.iter().enumerate() {
            match l.kind {
                LightKind::Static => push(l.pos, l.radius, l.color, l.intensity),
                LightKind::Flicker => {
                    let f = 0.88 + 0.08 * (t * 7.3 + li as f32 * 1.7).sin() + 0.05 * (t * 13.1 + li as f32).sin();
                    push(l.pos, l.radius, l.color, l.intensity * f);
                }
                LightKind::Lantern(c) => {
                    let g = self.lantern_glow[c];
                    let cc = catalog::category_color(c);
                    // Completed categories glow in their colour, softened toward warm white.
                    let k = g * 0.6;
                    let col = [
                        l.color[0] * (1.0 - k) + cc[0] * k,
                        l.color[1] * (1.0 - k) + cc[1] * k,
                        l.color[2] * (1.0 - k) + cc[2] * k,
                    ];
                    push(l.pos, l.radius + g * 2.0, col, l.intensity + g * 1.6);
                }
            }
        }
        // The librarian's own candle: a soft warm glow that keeps nearby spines readable.
        push(
            eye - self.player.forward() * 0.6 + Vec3::Y * 0.35,
            7.0,
            [1.0, 0.72, 0.45],
            0.6,
        );
        for f in &self.familiars {
            push(f.pos, 3.5, [0.75, 0.85, 1.0], 0.8);
        }
        let n = n as f32;
        Globals {
            view_proj: self.view_proj(aspect).to_cols_array_2d(),
            cam_pos: [eye.x, eye.y, eye.z, self.time],
            fog: [0.035, 0.03, 0.04, 0.022],
            ambient_top: [0.19, 0.18, 0.24, 0.0],
            ambient_bottom: [0.09, 0.065, 0.05, n],
            misc: [self.veil, 0.0, 0.0, 0.0],
            lights,
        }
    }

    pub fn view_proj(&self, aspect: f32) -> Mat4 {
        let view = Mat4::look_to_rh(self.player.eye(), self.player.forward(), Vec3::Y);
        let proj = Mat4::perspective_rh(vfov(aspect, self.fov), aspect, 0.03, 140.0);
        proj * view
    }

    fn book_inst(&self, b: usize, pos: Vec3, rot: Quat, flags: u32) -> Inst {
        let info = &self.lib.books[b];
        let s = &self.lib.series[info.series as usize];
        let param = s.pattern | (s.trim << 4) | (info.volume << 8) | (flags << 16);
        Inst::new(
            pos,
            rot,
            info.size,
            kind::BOOK,
            [s.color[0], s.color[1], s.color[2], 1.0],
            param,
        )
    }

    /// After `held` changes, point `active` back at the book it was on (`prev`),
    /// or clamp if that book has left the arms.
    fn follow_active(&mut self, prev: Option<u32>) {
        if let Some(i) = prev.and_then(|p| self.held.iter().position(|&b| b == p)) {
            self.active = i;
        } else {
            self.active = self.active.min(self.held.len().saturating_sub(1));
        }
    }

    pub fn active_book(&self) -> Option<u32> {
        self.held.get(self.active).copied()
    }

    pub fn build_instances(&mut self) {
        let mut dyn_insts = std::mem::take(&mut self.dyn_insts);
        let mut trans = std::mem::take(&mut self.trans_insts);
        let mut held_insts = std::mem::take(&mut self.held_insts);
        dyn_insts.clear();
        trans.clear();
        held_insts.clear();

        let in_flight: std::collections::HashSet<u32> = self.flights.iter().map(|f| f.book).collect();
        let held_series: std::collections::HashSet<u32> =
            self.held.iter().map(|&b| self.lib.books[b as usize].series).collect();
        let active = self.active_book();
        let sense = self.level(Upg::SortSense) > 0;
        let hover_series = match self.aim {
            Aim::Shelf { run: Some(r), .. } => Some(r.series),
            _ => None,
        };

        for b in 0..self.lib.books.len() {
            let st = self.state[b];
            let info = &self.lib.books[b];
            match st {
                BookState::Heap | BookState::Filing | BookState::Shelved | BookState::Carried(_) => {
                    dyn_insts.push(self.book_inst(b, self.pos[b], self.rot[b], 0));
                }
                BookState::Held => {
                    if in_flight.contains(&(b as u32)) {
                        dyn_insts.push(self.book_inst(b, self.pos[b], self.rot[b], 0));
                    }
                }
            }
            // Ghost slot for anything not yet home.
            if st != BookState::Shelved {
                let wing = wing_of_category(self.lib.series[info.series as usize].category);
                if !self.wing_unlocked(wing) {
                    continue;
                }
                let c = self.lib.series[info.series as usize].color;
                let hover = hover_series == Some(info.series);
                // Ghost levels (see fs_trans): waiting, carried series, active slot.
                let level = if sense && active == Some(b as u32) {
                    2
                } else if held_series.contains(&info.series) {
                    1
                } else {
                    0
                };
                let (col, mut a) = if level == 0 {
                    let boost = 1.0 + 0.6 / (c[0] + c[1] + c[2] + 0.2);
                    (c.map(|x| x * boost), if hover { 0.135 } else { 0.035 })
                } else {
                    (signal_color(c), if hover { 1.3 } else { 1.0 })
                };
                if st == BookState::Filing {
                    a *= 0.5;
                }
                trans.push(Inst::new(
                    info.slot_pos,
                    info.slot_rot,
                    info.size * 0.995,
                    kind::GHOST,
                    [col[0], col[1], col[2], a],
                    level,
                ));
            }
        }

        // Held stack in view.
        if !self.held.is_empty() {
            let n = self.held.len();
            let start = if n <= HAND_VISIBLE {
                0
            } else {
                self.active.saturating_sub(HAND_VISIBLE / 2).min(n - HAND_VISIBLE)
            };
            let end = (start + HAND_VISIBLE).min(n);
            let (base, rot) = self.hand_xform(0);
            let f = self.player.forward();
            let r = self.player.right();
            let u = r.cross(f);
            let mut y = 0.0;
            for i in start..end {
                let b = self.held[i];
                if in_flight.contains(&b) {
                    continue;
                }
                let info = &self.lib.books[b as usize];
                let is_active = i == self.active;
                let jitter = ((b as f32 * 12.9898).sin() * 43758.5).fract() - 0.5;
                let mut p = base + u * (y + info.size.x * 0.5) + r * (jitter * 0.03);
                if is_active {
                    p += -r * 0.06 - f * 0.02;
                }
                let yaw = Quat::from_axis_angle(u, jitter * 0.12 - 0.25);
                held_insts.push(self.book_inst(b as usize, p, yaw * rot, if is_active { 1 } else { 0 }));
                y += info.size.x;
            }
        }

        // Lanterns.
        for &(c, p) in &self.lib.lanterns {
            let g = self.lantern_glow[c];
            let cc = catalog::category_color(c);
            let col = [
                0.9 * (1.0 - g) + cc[0] * g,
                0.6 * (1.0 - g) + cc[1] * g,
                0.3 * (1.0 - g) + cc[2] * g,
            ];
            dyn_insts.push(Inst::new(
                p,
                Quat::IDENTITY,
                Vec3::new(0.16, 0.28, 0.16),
                kind::EMISSIVE,
                [col[0], col[1], col[2], 0.25 + g * 9.0],
                0,
            ));
        }

        // Familiars: folded-paper cranes with a soft inner glow.
        for f in &self.familiars {
            let heading = if f.vel.length_squared() > 0.01 {
                f.vel.x.atan2(-f.vel.z)
            } else {
                0.0
            };
            let body_rot = Quat::from_rotation_y(-heading);
            let k = 1.6;
            let glow = [0.55, 0.65, 1.0, 0.3];
            dyn_insts.push(Inst::new(
                f.pos,
                body_rot,
                Vec3::new(0.05, 0.05, 0.3) * k,
                kind::PAPER,
                glow,
                0,
            ));
            let flap = f.phase.sin() * 0.6;
            for s in [-1.0f32, 1.0] {
                let wing_rot = body_rot * Quat::from_rotation_z(s * (0.2 + flap));
                let off = wing_rot * Vec3::new(s * 0.14 * k, 0.0, 0.0);
                dyn_insts.push(Inst::new(
                    f.pos + off,
                    wing_rot,
                    Vec3::new(0.26, 0.008, 0.14) * k,
                    kind::PAPER,
                    glow,
                    0,
                ));
            }
            // Neck and tail.
            dyn_insts.push(Inst::new(
                f.pos + body_rot * Vec3::new(0.0, 0.06, -0.16) * k,
                body_rot * Quat::from_rotation_x(0.6),
                Vec3::new(0.025, 0.12, 0.025) * k,
                kind::PAPER,
                glow,
                0,
            ));
            dyn_insts.push(Inst::new(
                f.pos + body_rot * Vec3::new(0.0, 0.05, 0.15) * k,
                body_rot * Quat::from_rotation_x(-0.7),
                Vec3::new(0.02, 0.1, 0.02) * k,
                kind::PAPER,
                glow,
                0,
            ));
        }

        // Particles and dust motes.
        for p in &self.particles {
            let k = 1.0 - p.life / p.max;
            dyn_insts.push(Inst::new(
                p.pos,
                Quat::IDENTITY,
                Vec3::splat(p.size * k.max(0.1)),
                kind::EMISSIVE,
                [p.color[0], p.color[1], p.color[2], 2.5 * k],
                0,
            ));
        }
        let eye = self.player.eye();
        for (i, &(base, ph)) in self.motes.iter().enumerate() {
            let t = self.time * 0.05 + ph;
            let mut p = base + Vec3::new((t * 1.3).sin() * 0.8, (t * 0.7).sin() * 0.6, (t * 1.1).cos() * 0.8);
            // Keep motes near the player by wrapping into a 24m box around them.
            let rel = p - eye;
            p.x = eye.x + (rel.x + 12.0).rem_euclid(24.0) - 12.0;
            p.z = eye.z + (rel.z + 12.0).rem_euclid(24.0) - 12.0;
            let tw = 0.5 + 0.5 * (self.time * 0.8 + i as f32).sin();
            dyn_insts.push(Inst::new(
                p,
                Quat::IDENTITY,
                Vec3::splat(0.006),
                kind::EMISSIVE,
                [1.0, 0.85, 0.6, 0.6 + 1.2 * tw],
                0,
            ));
        }

        // Moonbeams and veil.
        let beam_dir = Vec3::new(0.0, -0.62, 1.0).normalize();
        for x in [-20.0f32, -15.5, -7.0, -2.3, 2.3, 7.0, 15.5, 20.0] {
            let start = Vec3::new(x, 4.4, -HALL_Z + 0.1);
            let len = 9.0;
            let rot = Quat::from_rotation_arc(Vec3::Y, beam_dir);
            trans.push(Inst::new(
                start + beam_dir * len * 0.5,
                rot,
                Vec3::new(1.6, len, 3.4),
                kind::SHAFT,
                [0.35, 0.45, 0.9, 0.05],
                0,
            ));
        }
        if self.veil > 0.0 {
            trans.push(Inst::new(
                Vec3::new(VEIL_X, HALL_H * 0.5, 0.0),
                Quat::IDENTITY,
                Vec3::new(0.05, HALL_H, HALL_Z * 2.0),
                kind::VEIL,
                [1.0, 1.0, 1.0, 1.0],
                0,
            ));
        }

        self.dyn_insts = dyn_insts;
        self.trans_insts = trans;
        self.held_insts = held_insts;
    }

    // -------------------------------------------------------------- save / load

    fn save_path() -> std::path::PathBuf {
        crate::platform::data_dir().join("save.json")
    }

    pub fn save(&mut self) {
        if !self.persist {
            return;
        }
        let mut shelved = Vec::new();
        let mut pending = Vec::new();
        for (b, st) in self.state.iter().enumerate() {
            match st {
                BookState::Shelved => shelved.push(b as u32),
                BookState::Filing | BookState::Carried(_) => pending.push(b as u32),
                BookState::Heap | BookState::Held => {}
            }
        }
        let p = &self.player;
        let data = SaveData {
            seed: SEED,
            lumens: self.lumens,
            earned: self.earned,
            upgrades: self.upg.to_vec(),
            shelved,
            pending,
            held: self.held.clone(),
            player: [p.pos.x, p.pos.y, p.pos.z, p.yaw, p.pitch],
            east_unlocked: self.east_unlocked,
            play_time: self.play_time,
        };
        let path = Self::save_path();
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let tmp = path.with_extension("json.tmp");
        match serde_json::to_vec(&data) {
            Ok(bytes) => {
                if std::fs::write(&tmp, bytes)
                    .and_then(|_| std::fs::rename(&tmp, &path))
                    .is_err()
                {
                    eprintln!("hushfold: failed to write save to {}", path.display());
                }
            }
            Err(e) => eprintln!("hushfold: failed to serialise save: {e}"),
        }
        self.dirty_save = false;
    }

    pub fn load(&mut self) -> bool {
        let Ok(bytes) = std::fs::read(Self::save_path()) else {
            return false;
        };
        let Ok(data) = serde_json::from_slice::<SaveData>(&bytes) else {
            eprintln!("hushfold: save file unreadable, starting fresh");
            return false;
        };
        if data.seed != SEED {
            return false;
        }
        let n = self.lib.books.len() as u32;
        for &b in &data.shelved {
            if b < n && self.state[b as usize] == BookState::Heap {
                self.state[b as usize] = BookState::Shelved;
                self.pos[b as usize] = self.lib.books[b as usize].slot_pos;
                self.rot[b as usize] = self.lib.books[b as usize].slot_rot;
                let s = self.lib.books[b as usize].series as usize;
                let c = self.lib.series[s].category;
                self.series_shelved[s] += 1;
                self.category_shelved[c] += 1;
                self.wing_shelved[wing_of_category(c)] += 1;
            }
        }
        for &b in &data.held {
            if b < n && self.state[b as usize] == BookState::Heap {
                self.state[b as usize] = BookState::Held;
                self.held.push(b);
            }
        }
        for &b in &data.pending {
            if b < n && self.state[b as usize] == BookState::Heap {
                let bi = b as usize;
                let info = &self.lib.books[bi];
                let normal = self.lib.faces[info.face as usize].normal;
                self.state[bi] = BookState::Filing;
                self.pos[bi] = info.slot_pos + normal * 0.8 + Vec3::Y * 0.4;
                self.rot[bi] = info.slot_rot;
                self.start_flight(b, Dest::Shelf, 0.8, 0.5);
            }
        }
        for h in &mut self.lib.heaps {
            h.books.retain(|&b| self.state[b as usize] == BookState::Heap);
        }
        self.recompute_heap_bounds();
        for (i, l) in data.upgrades.iter().enumerate().take(UPGRADES.len()) {
            self.upg[i] = (*l).min(upg_def(UPGRADES[i]).costs.len() as u32);
        }
        self.lumens = data.lumens;
        self.earned = data.earned;
        self.east_unlocked = data.east_unlocked;
        if self.east_unlocked {
            self.veil = 0.0;
        }
        self.play_time = data.play_time;
        let [x, y, z, yaw, pitch] = data.player;
        self.player.pos = Vec3::new(x, y, z);
        self.player.yaw = yaw;
        self.player.pitch = pitch;
        for _ in 0..self.level(Upg::Familiar) {
            self.spawn_familiar();
        }
        for c in 0..CATEGORIES.len() {
            if self.category_shelved[c] == self.lib.category_books[c] {
                self.lantern_glow[c] = 1.0;
            }
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generation_invariants() {
        let g = Game::new();
        let lib = &g.lib;
        // Every book is in exactly one heap, and heaps never mix wings.
        let mut seen = vec![0u32; lib.books.len()];
        for h in &lib.heaps {
            for &b in &h.books {
                seen[b as usize] += 1;
                let cat = lib.series[lib.books[b as usize].series as usize].category;
                assert_eq!(wing_of_category(cat), h.wing);
            }
        }
        assert!(seen.iter().all(|&c| c == 1), "each book must sit in exactly one heap");
        // Volumes run left to right along each shelf run.
        for runs in &lib.runs {
            for row in runs.iter() {
                for r in row {
                    let ids = &lib.series_books[r.series as usize];
                    for w in ids.windows(2) {
                        let (a, b) = (&lib.books[w[0] as usize], &lib.books[w[1] as usize]);
                        let f = &lib.faces[a.face as usize];
                        assert!(b.volume == a.volume + 1);
                        assert!((b.slot_pos - a.slot_pos).dot(f.right) > 0.0);
                    }
                }
            }
        }
        // No pile is unreasonably tall.
        for b in &lib.books {
            assert!(b.heap_pos.y < 3.2, "heap book at {:?}", b.heap_pos);
        }
        // Generation is deterministic (saves depend on it).
        let g2 = Game::new();
        assert_eq!(g2.lib.books.len(), lib.books.len());
        assert!(
            g2.lib
                .books
                .iter()
                .zip(&lib.books)
                .all(|(a, b)| a.heap_pos == b.heap_pos && a.slot_pos == b.slot_pos)
        );
    }

    /// Pacing estimate: a bot that walks (at real speeds) to the nearest heap, gathers an
    /// armful, files it in nearest-slot order, and greedily buys the cheapest upgrade.
    /// Familiars and the aura are approximated. Run with `--ignored --nocapture`.
    #[test]
    #[ignore]
    fn pacing_simulation() {
        let mut g = Game::new();
        let mut audio = crate::audio::Audio::silent();
        let mut t = 0.0f32;
        let mut pos = g.player.pos;
        let mut next_report = 0.0;
        let speed = |g: &Game| [4.0, 5.5, 7.0][g.level(Upg::Slippers) as usize] * 1.3; // some hurrying
        while g.total_shelved() < g.total_books() && t < 60.0 * 60.0 * 20.0 {
            // Familiars: each files ~one book per (flight time + handling), approximated.
            let fams = g.level(Upg::Familiar) as f32;
            // Gather.
            let Some(h) = (0..g.lib.heaps.len())
                .filter(|&h| !g.lib.heaps[h].books.is_empty() && g.wing_unlocked(g.lib.heaps[h].wing))
                .min_by(|&a, &b| {
                    g.lib.heaps[a]
                        .base
                        .distance(pos)
                        .total_cmp(&g.lib.heaps[b].base.distance(pos))
                })
            else {
                break;
            };
            let reach = g.heap_reach();
            t += (g.lib.heaps[h].base.distance(pos) - reach).max(0.0) / speed(&g) + 1.0;
            pos = g.lib.heaps[h].base;
            let take = g.capacity().min(g.lib.heaps[h].books.len());
            for _ in 0..take {
                let b = g.lib.heaps[h].books.pop().unwrap();
                g.state[b as usize] = BookState::Held;
                g.held.push(b);
            }
            // File, nearest slot first; a click files one book, or a whole series with Bundle.
            let aura_r: f32 = [0.0, 3.0, 6.0, 11.0][g.level(Upg::Aura) as usize];
            while !g.held.is_empty() {
                let (i, _) = g
                    .held
                    .iter()
                    .enumerate()
                    .map(|(i, &b)| (i, g.lib.books[b as usize].slot_pos.distance(pos)))
                    .min_by(|a, b| a.1.total_cmp(&b.1))
                    .unwrap();
                let b = g.held[i];
                let slot = g.lib.books[b as usize].slot_pos;
                let d = Vec3::new(slot.x - pos.x, 0.0, slot.z - pos.z).length();
                let walk = (d - aura_r.max(1.8)).max(0.0);
                t += walk / speed(&g) + if aura_r > 0.0 { 0.15 } else { 0.9 };
                pos += Vec3::new(slot.x - pos.x, 0.0, slot.z - pos.z).normalize_or_zero() * walk;
                let s = g.lib.books[b as usize].series;
                let batch: Vec<u32> = if g.level(Upg::Bundle) > 0 {
                    g.held
                        .iter()
                        .copied()
                        .filter(|&x| g.lib.books[x as usize].series == s)
                        .collect()
                } else {
                    vec![b]
                };
                g.held.retain(|x| !batch.contains(x));
                let mut thunks = 0;
                for x in batch {
                    g.on_shelved(x, &mut audio, &mut thunks);
                }
            }
            // Familiars over the elapsed trip: ~0.12 books/s each, more with Wings.
            let rate = [0.12, 0.2, 0.32][g.level(Upg::Wings) as usize] * fams;
            let fam_books = (rate * 30.0) as usize;
            for _ in 0..fam_books {
                if let Some(h) = (0..g.lib.heaps.len())
                    .find(|&h| !g.lib.heaps[h].books.is_empty() && g.wing_unlocked(g.lib.heaps[h].wing))
                {
                    let b = g.lib.heaps[h].books.pop().unwrap();
                    let mut thunks = 0;
                    g.on_shelved(b, &mut audio, &mut thunks);
                }
            }
            // Spend.
            loop {
                let cheapest = UPGRADES
                    .iter()
                    .copied()
                    .filter_map(|u| g.next_cost(u).map(|c| (c, u)))
                    .min_by_key(|x| x.0);
                match cheapest {
                    Some((c, u)) if c <= g.lumens => {
                        g.buy(u, &mut audio);
                        println!(
                            "{:6.1} min  bought {:?} -> {}  (shelved {})",
                            t / 60.0,
                            u,
                            g.level(u),
                            g.total_shelved()
                        );
                    }
                    _ => break,
                }
            }
            if t >= next_report {
                println!(
                    "{:6.1} min  shelved {:5} / {}  west {:.0}%  east {:.0}%",
                    t / 60.0,
                    g.total_shelved(),
                    g.total_books(),
                    100.0 * g.wing_shelved[0] as f32 / g.wing_total[0] as f32,
                    100.0 * g.wing_shelved[1] as f32 / g.wing_total[1] as f32
                );
                next_report += 15.0 * 60.0;
            }
        }
        println!("done at {:.1} h, shelved {}", t / 3600.0, g.total_shelved());
    }

    /// Every slot must be fileable when looked at from a natural oblique standpoint
    /// (regression: a bookcase's own collision box used to block grazing rays).
    #[test]
    fn slots_aimable_at_angles() {
        let mut g = Game::new();
        g.east_unlocked = true;
        for h in &mut g.lib.heaps {
            h.books.clear();
        }
        g.recompute_heap_bounds();
        let mut failures = 0;
        let mut tried = 0;
        for b in (0..g.lib.books.len()).step_by(37) {
            let (normal, right, face_id, row, series, slot) = {
                let i = &g.lib.books[b];
                let f = &g.lib.faces[i.face as usize];
                // Aim at the middle of the spine, which is what a player sees and clicks.
                let spine = i.slot_pos + f.normal * (i.size.z * 0.5);
                (f.normal, f.right, i.face, i.row, i.series, spine)
            };
            for side in [-1.0f32, 1.0] {
                let dir = (normal + right * side * 0.9).normalize();
                let from = Vec3::new(slot.x, 0.0, slot.z) + dir * 1.6;
                g.look_at(from, slot);
                if g.player.pos.distance(from) > 1e-3 {
                    continue;
                }
                tried += 1;
                match g.compute_aim() {
                    Aim::Shelf { face, run: Some(r), .. } if face == face_id && r.series == series => {}
                    other => {
                        failures += 1;
                        if failures < 5 {
                            eprintln!("book {b} face {face_id} row {row}: {other:?}");
                        }
                    }
                }
            }
        }
        assert!(tried > 300);
        assert_eq!(failures, 0, "{failures}/{tried} oblique aims failed");
    }

    /// Tidy Arms must prefer a slot on the face you stand at over one that is nearer in a
    /// straight line but on the far side of the same double-sided case.
    #[test]
    fn nearest_is_measured_on_foot() {
        let mut g = Game::new();
        g.upg[upg_index(Upg::Tidy)] = 1;
        g.upg[upg_index(Upg::SortSense)] = 1;
        let stand = Vec3::new(-16.3, 0.0, 5.6);
        g.player.pos = stand;
        let faces_toward = |g: &Game, b: usize, want: f32| {
            let p = g.lib.books[b].slot_pos;
            g.lib.faces[g.lib.books[b].face as usize].normal.z == want && p.x < 0.0 && (3.0..5.0).contains(&p.z)
        };
        // Behind the case, straight ahead (the row at z = 4 faces north on that side).
        let far = (0..g.lib.books.len())
            .filter(|&b| faces_toward(&g, b, -1.0))
            .min_by(|&a, &b| {
                let d = |i: usize| (g.lib.books[i].slot_pos.x - stand.x).abs();
                d(a).total_cmp(&d(b))
            })
            .unwrap();
        // On the near face, a few metres along the aisle.
        let near = (0..g.lib.books.len())
            .find(|&b| faces_toward(&g, b, 1.0) && (g.lib.books[b].slot_pos.x - stand.x).abs() > 2.4)
            .unwrap();
        let eye = g.player.eye();
        assert!(
            g.lib.books[far].slot_pos.distance(eye) < g.lib.books[near].slot_pos.distance(eye),
            "test premise: the far-side slot is nearer in a straight line"
        );
        for b in [far as u32, near as u32] {
            g.state[b as usize] = BookState::Held;
            g.held.push(b);
        }
        g.active = 0;
        let mut audio = crate::audio::Audio::silent();
        g.update(1.0 / 60.0, &Input::default(), &mut audio, true);
        assert_eq!(g.active_book(), Some(near as u32));
        assert!(g.sense_route.is_some_and(|r| r.direct));
        // A book chosen by cycling stays active while you walk to it.
        let cycle = Input {
            cycle: 1,
            ..Input::default()
        };
        g.update(1.0 / 60.0, &cycle, &mut audio, true);
        assert_eq!(g.active_book(), Some(far as u32));
        for _ in 0..600 {
            g.update(1.0 / 60.0, &Input::default(), &mut audio, true);
        }
        assert_eq!(g.active_book(), Some(far as u32));
        // With only the far-side book in hand, Sort Sense routes round the end of the row.
        g.held.retain(|&b| b == far as u32);
        g.active = 0;
        g.update(1.0 / 60.0, &Input::default(), &mut audio, true);
        let r = g.sense_route.unwrap();
        assert!(!r.direct);
        assert!(r.dist > 4.0, "route {r:?}");
        assert!(
            r.waypoint.x < -20.0 || r.waypoint.x > -12.6,
            "waypoint should be at a row end: {r:?}"
        );
    }

    #[test]
    fn save_round_trip() {
        let dir = std::env::temp_dir().join(format!("hushfold-test-{}", std::process::id()));
        // SAFETY: tests in this module don't read the environment concurrently.
        unsafe { std::env::set_var("XDG_DATA_HOME", &dir) };
        let mut g = Game::new();
        let mut audio = crate::audio::Audio::silent();
        // Shelve the tops of a few heaps (as play does), leave one book mid-flight.
        let mut thunks = 0;
        for h in 4..9 {
            for _ in 0..20 {
                let b = g.lib.heaps[h].books.pop().unwrap();
                g.on_shelved(b, &mut audio, &mut thunks);
            }
        }
        let flying = g.lib.heaps[10].books.pop().unwrap();
        g.state[flying as usize] = BookState::Filing;
        // Take a few books from the top of a heap into the arms.
        let h = g.lib.heaps.iter().position(|h| h.books.len() > 3).unwrap();
        for _ in 0..3 {
            let b = g.lib.heaps[h].books.pop().unwrap();
            g.state[b as usize] = BookState::Held;
            g.held.push(b);
        }
        g.lumens = 42;
        g.upg[upg_index(Upg::Arms)] = 2;
        g.save();

        let mut l = Game::new();
        assert!(l.load());
        assert_eq!(l.total_shelved(), g.total_shelved());
        assert_eq!(l.lumens, 42);
        // The in-flight book resumes its flight and is paid for when it lands.
        assert_eq!(l.state[flying as usize], BookState::Filing);
        for _ in 0..120 {
            l.update(1.0 / 60.0, &Input::default(), &mut audio, false);
        }
        assert_eq!(l.state[flying as usize], BookState::Shelved);
        assert_eq!(l.total_shelved(), g.total_shelved() + 1);
        assert_eq!(l.lumens, 43);
        assert_eq!(l.held, g.held);
        assert_eq!(l.capacity(), 24);
        for (a, b) in l.lib.heaps.iter().zip(&g.lib.heaps) {
            assert_eq!(a.books, b.books);
        }
        let _ = std::fs::remove_dir_all(&dir);
    }
}
