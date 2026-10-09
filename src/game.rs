//! Game state: player, books in motion, upgrades, familiars, progression, save/load.

use crate::audio::{Audio, Sfx};
use crate::catalog;
use crate::gfx::{Globals, GpuLight, Inst, MAX_LIGHTS, kind};
use crate::library::*;
use crate::loose::{Loose, Rest};
use crate::nav::{Nav, Route};
use crate::scene::Scene;
use glam::{Mat4, Quat, Vec2, Vec3};
use rand::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::f32::consts::PI;

pub const SEED: u64 = 0x4855_5348_464f_4c44;
const EYE: f32 = 1.62;
const SHELF_REACH: f32 = 4.2;
const PLAYER_R: f32 = 0.3;
/// Highest rise the player walks up (a stair); anything higher is a wall.
const STEP_UP: f32 = 0.45;
/// The player's height, for what they bump into.
const BODY_H: f32 = 1.8;
const HAND_VISIBLE: usize = 8;
/// Shelving Aura by level: (radius in metres, books per second).
const AURA_LEVELS: [(f32, f32); 4] = [(0.0, 0.0), (3.0, 2.0), (4.5, 4.0), (6.0, 8.0)];
/// Familiar Wings by level: (flight speed in m/s, books carried).
const WINGS_LEVELS: [(f32, usize); 3] = [(2.6, 2), (3.8, 3), (5.0, 5)];
/// Sifting Touch by level: radius in metres.
const SIFT_RADIUS: [f32; 3] = [0.0, 0.9, 1.8];
/// Seconds to charge a toss fully.
const TOSS_CHARGE: f32 = 0.9;
/// Toss speed in m/s, uncharged and fully charged.
const TOSS_SPEED: (f32, f32) = (3.0, 12.0);

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
    Sift,
    Bundle,
    Tidy,
    Lodestone,
    Aura,
    Familiar,
    Wings,
}

pub const UPGRADES: [Upg; 10] = [
    Upg::Arms,
    Upg::Slippers,
    Upg::SortSense,
    Upg::Sift,
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
        Upg::Sift => UpgDef {
            name: "Sifting Touch",
            blurb: "Press F at a pile to draw out only books of the category you aim at.",
            costs: &[40, 400],
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
            blurb: "Gather from piles at a distance; hold the button to keep gathering.",
            costs: &[300],
        },
        Upg::Familiar => UpgDef {
            name: "Paper Familiar",
            blurb: "A folded-paper crane that ferries books to their shelves on its own.",
            costs: &[250, 600, 1200, 2200, 3200],
        },
        Upg::Aura => UpgDef {
            name: "Shelving Aura",
            blurb: "Books you carry drift to their slots whenever you pass close by.",
            costs: &[500, 1500, 3000],
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
    /// Lying in a pile, or in the air after a toss.
    Loose,
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
    /// To the top of a pile (on a level).
    ToPile(usize, Vec3),
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
    /// A loose book; `point` is where the ray meets it.
    Loose {
        book: u32,
        point: Vec3,
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
    /// Held to charge a toss; released to throw.
    pub toss_down: bool,
    /// Pressed this frame (a press and release within one frame still tosses).
    pub toss_pressed: bool,
    pub sift_down: bool,
    pub sift_pressed: bool,
    pub cycle: i32,
}

pub struct Player {
    /// Feet.
    pub pos: Vec3,
    /// Walking velocity (horizontal) and falling speed.
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
    /// Upgrade levels by name.
    #[serde(default)]
    levels: BTreeMap<String, u32>,
    /// 0.1.x saves: levels by position in the 0.1 upgrade order.
    #[serde(default, skip_serializing)]
    upgrades: Vec<u32>,
    shelved: Vec<u32>,
    /// Books mid-flight to a shelf or carried by a familiar; they fly home on load
    /// so their rewards are paid through the normal path.
    #[serde(default)]
    pending: Vec<u32>,
    held: Vec<u32>,
    player: [f32; 5],
    play_time: f64,
    /// Loose books that left their generated pose: (book, [x, y, z, ux, uz, su, sv]).
    /// Absent in 0.1.x saves, whose piles re-settle around the books taken from them.
    #[serde(default)]
    loose: Option<Vec<(u32, [f32; 7])>>,
    /// The level of each book in `loose` that lies above the ground floor.
    #[serde(default)]
    loose_levels: Vec<(u32, u8)>,
    /// Books in the air: (book, position). They fall from there on load.
    #[serde(default)]
    airborne: Vec<(u32, [f32; 3])>,
}

/// The 0.1.x upgrade order, for reading old saves.
const LEGACY_ORDER: [Upg; 9] = [
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

pub struct Game {
    pub layout: Layout,
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
    pub dept_total: Vec<u32>,
    pub dept_shelved: Vec<u32>,
    pub gate_open: Vec<bool>,
    /// How much of each gate's veil is still drawn (1 closed, 0 gone).
    gate_fade: Vec<f32>,
    familiars: Vec<Familiar>,
    particles: Vec<Particle>,
    motes: Vec<(Vec3, f32)>,
    pub toasts: Vec<Toast>,
    pub aim: Aim,
    pub time: f32,
    pub play_time: f64,
    grab_cooldown: f32,
    sift_cooldown: f32,
    /// Toss charge, 0..1, while the toss button is held.
    pub toss_charge: f32,
    toss_was_down: bool,
    /// While charging: the predicted arc and where it lands.
    pub toss_arc: Vec<Vec3>,
    pub toss_land: Option<Vec3>,
    aura_budget: f32,
    pub lantern_glow: Vec<f32>,
    rng: StdRng,
    pub hint: Option<(String, f32)>,
    // Per-frame instance scratch.
    pub dyn_insts: Vec<Inst>,
    pub trans_insts: Vec<Inst>,
    pub held_insts: Vec<Inst>,
    /// Books lying still and slot ghosts, by chunk.
    pub scene: Scene,
    /// Gate states the scene's ghosts were drawn with.
    scene_gates: Vec<bool>,
    pub loose: Loose,
    nav: Nav,
    /// Sort Sense: the way to the active book's slot.
    pub sense_route: Option<Route>,
    /// Vertical field of view on wide windows, in degrees (a setting).
    pub fov: f32,
    pub dirty_save: bool,
    /// False in scripted runs so they never touch the player's save.
    pub persist: bool,
}

/// True (and `seen` updated) if the gates have changed since `seen`.
fn scene_gates_changed(seen: &mut Vec<bool>, now: &[bool]) -> bool {
    if seen.as_slice() == now {
        return false;
    }
    *seen = now.to_vec();
    true
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
    pub fn new(layout: Layout) -> Self {
        let lib = Library::generate(layout, SEED);
        let n = lib.books.len();
        let mut dept_total = vec![0u32; lib.catalog.departments.len()];
        for b in 0..n as u32 {
            dept_total[lib.dept_of(lib.category_of_book(b))] += 1;
        }
        let loose = Loose::new(&lib);
        let pos = (0..n as u32).map(|b| loose.pose(b).0).collect();
        let rot = (0..n as u32).map(|b| loose.pose(b).1).collect();
        let mut rng = StdRng::seed_from_u64(0x5eed);
        let ground = &lib.levels[0];
        let motes = (0..700)
            .map(|_| {
                (
                    Vec3::new(
                        rng.gen_range(ground.min.x..ground.max.x),
                        rng.gen_range(0.2..6.0),
                        rng.gen_range(ground.min.y..ground.max.y),
                    ),
                    rng.gen_range(0.0..1.0f32) * 100.0,
                )
            })
            .collect();
        Game {
            state: vec![BookState::Loose; n],
            pos,
            rot,
            flights: Vec::new(),
            held: Vec::new(),
            active: 0,
            manual_pick: None,
            player: Player {
                pos: lib.spawn.0,
                vel: Vec3::ZERO,
                yaw: lib.spawn.1,
                pitch: -0.05,
                bob: 0.0,
            },
            lumens: 0,
            earned: 0,
            upg: [0; UPGRADES.len()],
            series_shelved: vec![0; lib.series.len()],
            category_shelved: vec![0; lib.catalog.categories.len()],
            dept_shelved: vec![0; dept_total.len()],
            dept_total,
            gate_open: vec![false; lib.gates.len()],
            gate_fade: vec![1.0; lib.gates.len()],
            familiars: Vec::new(),
            particles: Vec::new(),
            motes,
            toasts: Vec::new(),
            aim: Aim::None,
            time: 0.0,
            play_time: 0.0,
            grab_cooldown: 0.0,
            sift_cooldown: 0.0,
            toss_charge: 0.0,
            toss_was_down: false,
            toss_arc: Vec::new(),
            toss_land: None,
            aura_budget: 0.0,
            lantern_glow: vec![0.0; lib.catalog.categories.len()],
            rng,
            hint: None,
            dyn_insts: Vec::with_capacity(4096),
            trans_insts: Vec::with_capacity(256),
            held_insts: Vec::new(),
            scene: Scene::new(
                (lib.books.iter().enumerate()).map(|(b, i)| (b as u32, i.slot_pos)),
                |b| lib.books[b as usize].series,
                lib.series.len(),
            ),
            scene_gates: vec![false; lib.gates.len()],
            loose,
            nav: Nav::new(&lib),
            sense_route: None,
            fov: 70.0,
            dirty_save: false,
            persist: true,
            layout,
            lib,
        }
    }

    pub fn level(&self, u: Upg) -> u32 {
        self.upg[upg_index(u)]
    }

    pub fn capacity(&self) -> usize {
        [6, 12, 24, 48, 96][self.level(Upg::Arms) as usize]
    }

    fn gather_reach(&self) -> f32 {
        if self.level(Upg::Lodestone) > 0 { 11.0 } else { 3.6 }
    }

    pub fn total_books(&self) -> u32 {
        self.lib.books.len() as u32
    }

    pub fn total_shelved(&self) -> u32 {
        self.dept_shelved.iter().sum()
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

    /// Books of the departments gate `g` counts still to shelve before it lifts.
    pub fn gate_books_left(&self, g: usize) -> u32 {
        let gate = &self.lib.gates[g];
        let total: u32 = gate.counts.iter().map(|&d| self.dept_total[d]).sum();
        let done: u32 = gate.counts.iter().map(|&d| self.dept_shelved[d]).sum();
        let need = (total as f32 * gate.need).ceil() as u32;
        need.saturating_sub(done)
    }

    /// The first gate still closed.
    pub fn next_gate(&self) -> Option<usize> {
        self.gate_open.iter().position(|&o| !o)
    }

    /// A department is open once the gate before it has lifted.
    pub fn dept_open(&self, d: usize) -> bool {
        (self.lib.gates.iter().zip(&self.gate_open)).all(|(g, &open)| open || g.opens != d)
    }

    fn open_gate(&mut self, g: usize) {
        self.gate_open[g] = true;
        self.loose.set_gate(g, false);
        self.nav.set_gate(g, false);
    }

    /// Lift every gate whose departments are shelved far enough.
    fn check_gates(&mut self, audio: &mut Audio) {
        for g in 0..self.lib.gates.len() {
            if !self.gate_open[g] && self.gate_books_left(g) == 0 {
                self.open_gate(g);
                audio.play(Sfx::Fanfare, 0.8);
                let name = &self.lib.catalog.departments[self.lib.gates[g].opens].name;
                self.toast(format!("The veil over the {name} lifts…"), [0.75, 0.6, 1.0], true);
            }
        }
    }

    /// Script aid: lift every gate.
    pub fn open_all_gates(&mut self) {
        for g in 0..self.lib.gates.len() {
            self.open_gate(g);
        }
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
            self.sift_cooldown -= dt;
            if input.sift_pressed || (input.sift_down && self.sift_cooldown <= 0.0) {
                self.sift(audio, input.sift_pressed);
            }
            self.update_toss(dt, input, audio);
        } else {
            self.toss_charge = 0.0;
            self.toss_was_down = false;
            self.toss_arc.clear();
            self.toss_land = None;
        }
        if self.manual_pick.is_some_and(|b| !self.held.contains(&b)) {
            self.manual_pick = None;
        }
        let tidy = self.level(Upg::Tidy) > 0;
        let sense = self.level(Upg::SortSense) > 0;
        if (tidy || sense) && !self.held.is_empty() {
            self.nav.update(self.player.pos);
        }
        if tidy && self.manual_pick.is_none() {
            self.pick_nearest_active();
        }
        self.update_aura(dt);
        // After the aura, which may have filed the active book.
        self.sense_route = match self.active_book() {
            Some(b) if sense => self.nav.route(self.player.pos, self.approach(b)),
            _ => None,
        };
        self.update_flights(dt, audio);
        let landings = self.loose.update(dt, &mut self.pos, &mut self.rot);
        self.dirty_save |= !landings.is_empty();
        let eye = self.player.eye();
        for l in landings.iter().take(3) {
            let near = (1.0 - l.at.distance(eye) / 25.0).max(0.0f32);
            let v = (0.15 + l.speed * 0.04).min(0.45f32) * near;
            if v > 0.01 {
                audio.play(Sfx::Land, v);
            }
        }
        self.update_familiars(dt);
        self.update_particles(dt);

        // Lanterns ease toward their category completion.
        for c in 0..self.lib.catalog.categories.len() {
            let target = if self.category_shelved[c] == self.lib.category_books[c] {
                1.0
            } else {
                0.3 * self.category_shelved[c] as f32 / self.lib.category_books[c].max(1) as f32
            };
            self.lantern_glow[c] += (target - self.lantern_glow[c]) * (1.0 - (-dt * 1.5).exp());
        }
        for (f, &open) in self.gate_fade.iter_mut().zip(&self.gate_open) {
            if open {
                *f = (*f - dt * 0.25).max(0.0);
            }
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

    /// Boxes the player bumps into standing with their feet at `feet`: furniture and
    /// walls overlapping the body (not what they can step over or walk under), and
    /// closed gates (with the gate's index).
    fn solids_in_way(&self, feet: f32) -> impl Iterator<Item = (Vec3, Vec3, Option<usize>)> + '_ {
        let body = move |mn: Vec3, mx: Vec3| mx.y > feet + STEP_UP && mn.y < feet + BODY_H;
        let furniture = (self.lib.obstacles.iter())
            .filter(move |o| body(o.min, o.max))
            .map(|o| (o.min, o.max, None));
        let gates = (self.lib.gates.iter().enumerate())
            .filter(move |&(g, gate)| !self.gate_open[g] && body(gate.min, gate.max))
            .map(|(g, gate)| (gate.min, gate.max, Some(g)));
        furniture.chain(gates)
    }

    fn update_player(&mut self, dt: f32, input: &Input) {
        let (yaw, pitch) = (self.player.yaw + input.look.x, self.player.pitch - input.look.y);
        self.player.yaw = yaw;
        self.player.pitch = pitch.clamp(-1.45, 1.45);
        let fwd = Vec3::new(yaw.sin(), 0.0, -yaw.cos());
        let right = self.player.right();
        let mut wish = fwd * input.forward + right * input.strafe;
        if wish.length_squared() > 1.0 {
            wish = wish.normalize();
        }
        let base = [4.0, 5.5, 7.0][self.upg[upg_index(Upg::Slippers)] as usize];
        let speed = if input.sprint { base * 1.6 } else { base };
        let k = 1.0 - (-dt * 10.0).exp();
        let walk = Vec3::new(self.player.vel.x, 0.0, self.player.vel.z).lerp(wish * speed, k);
        let pos = self.player.pos;
        let lv = &self.lib.levels[self.lib.level_at(pos)];
        let (lo, hi) = (lv.min + PLAYER_R + 0.1, lv.max - PLAYER_R - 0.1);
        // In steps short enough that nothing thin (a veil, a rail) is stepped over.
        let total = Vec2::new(walk.x, walk.z) * dt;
        let steps = (total.length() / 0.1).ceil().max(1.0) as usize;
        let sub = dt / steps as f32;
        let mut np = pos;
        let mut vy = self.player.vel.y;
        let mut pressed_gate = None;
        for _ in 0..steps {
            let (from, feet) = (np, np.y);
            np.x += total.x / steps as f32;
            np.z += total.y / steps as f32;
            // Collide with what stands in the way (circle vs box in xz).
            for _ in 0..2 {
                for (mn, mx, gate) in self.solids_in_way(feet) {
                    let q = Vec2::new(np.x.clamp(mn.x, mx.x), np.z.clamp(mn.z, mx.z));
                    let d = Vec2::new(np.x, np.z) - q;
                    let l = d.length();
                    if l >= PLAYER_R {
                        continue;
                    }
                    if l <= 1e-5 {
                        // Inside it (only if it moved in): back to where this step began.
                        (np.x, np.z) = (from.x, from.z);
                        continue;
                    }
                    let push = d / l * (PLAYER_R - l);
                    np.x += push.x;
                    np.z += push.y;
                    if gate.is_some() && Vec2::new(wish.x, wish.z).dot(-push.normalize_or_zero()) > 0.3 {
                        pressed_gate = gate;
                    }
                }
            }
            np.x = np.x.clamp(lo.x, hi.x);
            np.z = np.z.clamp(lo.y, hi.y);
            // A rise too high to step onto is a wall: slide along it if one axis is
            // clear, else stay.
            let walled = |x: f32, z: f32| self.lib.rises_at(Vec2::new(x, z), feet, STEP_UP, BODY_H);
            if walled(np.x, np.z) {
                if !walled(np.x, from.z) {
                    np.z = from.z;
                } else if !walled(from.x, np.z) {
                    np.x = from.x;
                } else {
                    (np.x, np.z) = (from.x, from.z);
                }
            }
            // Stand on the floor or stair underfoot; step down stairs, fall off edges.
            match self.lib.ground_at(Vec2::new(np.x, np.z), feet + STEP_UP) {
                Some(g) if g >= feet - 0.6 && vy <= 0.0 => {
                    np.y = g;
                    vy = 0.0;
                }
                ground => {
                    vy -= crate::loose::GRAVITY * sub;
                    np.y = feet + vy * sub;
                    if let Some(g) = ground
                        && np.y <= g
                    {
                        np.y = g;
                        vy = 0.0;
                    }
                }
            }
        }
        let p = &mut self.player;
        p.vel = Vec3::new(walk.x, vy, walk.z);
        let moved = Vec2::new(np.x - p.pos.x, np.z - p.pos.z).length();
        p.bob += moved * 1.4;
        p.pos = np;
        if let Some(g) = pressed_gate
            && self.hint.is_none()
        {
            let left = self.gate_books_left(g);
            let names = self.lib.gate_counts_names(g);
            self.hint = Some((
                format!("The veil holds. Shelve {left} more {names} books to lift it."),
                2.5,
            ));
        }
    }

    /// The first thing that blocks sight along a ray (furniture, closed gates, floors),
    /// within `max_t`.
    fn ray_block(&self, o: Vec3, d: Vec3, max_t: f32) -> f32 {
        let furniture = (self.lib.obstacles.iter())
            .filter(|ob| ob.blocks_ray)
            .map(|ob| (ob.min, ob.sight_max()));
        let gates = (self.lib.gates.iter().zip(&self.gate_open))
            .filter(|&(_, &open)| !open)
            .map(|(g, _)| (g.min, g.max));
        let floors = self.lib.slabs.iter().map(|s| {
            let y = self.lib.levels[s.level].y;
            (Vec3::new(s.min.x, y - SLAB, s.min.y), Vec3::new(s.max.x, y, s.max.y))
        });
        (furniture.chain(gates).chain(floors))
            .filter_map(|(mn, mx)| Self::ray_aabb(o, d, mn, mx))
            .fold(max_t, f32::min)
    }

    /// True if nothing that blocks rays lies between `a` and `b`.
    pub fn line_of_sight(&self, a: Vec3, b: Vec3) -> bool {
        let d = b - a;
        let len = d.length();
        let dir = d / len.max(1e-5);
        self.ray_block(a, dir, f32::MAX) >= len - 0.15
    }

    fn ray_aabb(o: Vec3, d: Vec3, mn: Vec3, mx: Vec3) -> Option<f32> {
        let inv = d.recip();
        let t1 = (mn - o) * inv;
        let t2 = (mx - o) * inv;
        let tmin = t1.min(t2).max_element();
        let tmax = t1.max(t2).min_element();
        (tmax >= tmin.max(0.0)).then_some(tmin.max(0.0))
    }

    fn compute_aim(&self) -> Aim {
        let o = self.player.eye();
        let d = self.player.forward();
        let block = self.ray_block(o, d, f32::MAX);
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
            if !self.dept_open(self.lib.dept_of(f.category)) {
                continue;
            }
            let mut row = None;
            for r in 0..ROWS {
                let y0 = f.center.y + row_floor_y(r);
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
        // Loose books, short of whatever blocks the view.
        let max_t = self.gather_reach().min(block + 0.3).min(best_t);
        if let Some((book, t)) = self.loose.pick(o, d, max_t) {
            best = Aim::Loose {
                book,
                point: o + d * t,
                dist: t,
            };
        }
        best
    }

    fn interact(&mut self, audio: &mut Audio, fresh_press: bool) {
        match self.aim {
            Aim::Loose { book, point, .. } => {
                let room = self.capacity().saturating_sub(self.held.len());
                if room == 0 {
                    if fresh_press {
                        audio.play(Sfx::Nope, 0.4);
                        self.hint = Some(("Your arms are full — file some books first.".into(), 2.0));
                    }
                    self.grab_cooldown = 0.3;
                    return;
                }
                let radius = 0.35 + 0.05 * (room as f32).sqrt();
                let lv = self.loose.level_of(book);
                let books = self
                    .loose
                    .gather(lv, Vec2::new(point.x, point.z), radius, room, |_| true);
                self.take_into_arms(&books, audio);
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
                self.leave_hand(&books);
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

    /// Books just lifted from a pile fly to the arms.
    fn take_into_arms(&mut self, books: &[u32], audio: &mut Audio) {
        if books.is_empty() {
            return;
        }
        for (i, &b) in books.iter().enumerate() {
            self.state[b as usize] = BookState::Held;
            self.held.push(b);
            self.start_flight(b, Dest::Hand, 0.32 + i as f32 * 0.004, i as f32 * 0.025);
        }
        self.manual_pick = None;
        if self.level(Upg::Tidy) > 0 {
            let prev = self.active_book();
            self.held.sort();
            self.follow_active(prev);
        } else {
            self.active = self.held.len() - 1;
        }
        audio.play(Sfx::Rustle, 0.6);
        self.dirty_save = true;
    }

    fn category_of(&self, b: u32) -> usize {
        self.lib.series[self.lib.books[b as usize].series as usize].category
    }

    /// Sifting Touch: draw books of the aimed book's category out of the pile around it.
    fn sift(&mut self, audio: &mut Audio, fresh_press: bool) {
        let lvl = self.level(Upg::Sift) as usize;
        let Aim::Loose { book, point, .. } = self.aim else {
            return;
        };
        if lvl == 0 {
            if fresh_press {
                self.hint = Some(("Sifting is learned in the Grimoire (Tab).".into(), 2.0));
            }
            return;
        }
        let room = self.capacity().saturating_sub(self.held.len());
        if room == 0 {
            if fresh_press {
                audio.play(Sfx::Nope, 0.4);
                self.hint = Some(("Your arms are full — file some books first.".into(), 2.0));
            }
            self.sift_cooldown = 0.3;
            return;
        }
        let cat = self.category_of(book);
        let lib = &self.lib;
        let same = |b: u32| lib.category_of_book(b) == cat;
        let lv = self.loose.level_of(book);
        let books = self
            .loose
            .gather(lv, Vec2::new(point.x, point.z), SIFT_RADIUS[lvl], room, same);
        self.take_into_arms(&books, audio);
        self.sift_cooldown = 0.4;
    }

    /// Where a toss at the current charge leaves the hand, and how fast.
    fn toss_launch(&self) -> (Vec3, Vec3) {
        let (mut hand, _) = self.hand_xform(0);
        // Flush against furniture or a closed gate the hand is inside it; leave from
        // just short of it.
        let eye = self.player.eye();
        let reach = hand - eye;
        let dir = reach.normalize();
        let gates = (self.lib.gates.iter().zip(&self.gate_open))
            .filter(|&(_, &open)| !open)
            .map(|(g, _)| (g.min, g.max));
        let hit = (self.lib.obstacles.iter().map(|o| (o.min, o.max)))
            .chain(gates)
            .filter_map(|(mn, mx)| Self::ray_aabb(eye, dir, mn, mx))
            .fold(f32::MAX, f32::min);
        if hit < reach.length() {
            hand = eye + dir * (hit - 0.08).max(0.0);
        }
        // A little above the crosshair, so a level throw carries.
        let dir = (self.player.forward() + Vec3::Y * 0.18).normalize();
        let speed = TOSS_SPEED.0 + (TOSS_SPEED.1 - TOSS_SPEED.0) * self.toss_charge;
        (hand, dir * speed)
    }

    /// Charge while the toss button is held (showing the arc); throw on release.
    fn update_toss(&mut self, dt: f32, input: &Input, audio: &mut Audio) {
        let down = input.toss_down || input.toss_pressed;
        let released = self.toss_was_down && !down;
        self.toss_was_down = down;
        self.toss_arc.clear();
        self.toss_land = None;
        if down && !self.held.is_empty() {
            self.toss_charge = (self.toss_charge + dt / TOSS_CHARGE).min(1.0);
            let (from, vel) = self.toss_launch();
            let mut arc = std::mem::take(&mut self.toss_arc);
            self.toss_land = self.loose.predict(from, vel, &mut arc);
            self.toss_arc = arc;
        } else if down && self.hint.is_none() {
            self.hint = Some(("Nothing to toss — gather some books first.".into(), 1.5));
        }
        if released && !self.held.is_empty() {
            self.toss(audio);
        }
        if !down {
            self.toss_charge = 0.0;
        }
    }

    /// Throw every held book of the active book's category, as a loose stream.
    fn toss(&mut self, audio: &mut Audio) {
        let Some(active) = self.active_book() else { return };
        let cat = self.category_of(active);
        let group: Vec<u32> = self
            .held
            .iter()
            .copied()
            .filter(|&b| self.category_of(b) == cat)
            .collect();
        let prev = self.active_book();
        self.held.retain(|b| !group.contains(b));
        self.follow_active(prev);
        let (from, vel) = self.toss_launch();
        let speed = vel.length();
        let gap = (0.5 / group.len() as f32).min(0.04);
        for (i, &b) in group.iter().enumerate() {
            self.flights.retain(|f| f.book != b);
            self.state[b as usize] = BookState::Loose;
            let mut j = || self.rng.gen_range(-1.0..1.0f32);
            let scatter = Vec3::new(j(), j(), j()) * (0.06 * speed + 0.3);
            let spin = Vec3::new(j(), j(), j()) * 9.0;
            let rot = self.hand_xform(0).1;
            self.loose.throw(b, from, vel + scatter, rot, spin, i as f32 * gap);
        }
        audio.play_pitched(Sfx::Whoosh, 0.45, 2);
        self.toss_charge = 0.0;
        self.dirty_save = true;
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

    /// Where each held book is drawn: the window of the stack in view, with the books
    /// outside it tucked at the nearer end. Books still flying to the hand take no
    /// place in the stack.
    fn held_layout(&self) -> (std::ops::Range<usize>, Vec<(Vec3, Quat)>) {
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
        let mut out = vec![(base, rot); n];
        let mut y = 0.0;
        for (i, slot) in out.iter_mut().enumerate().take(end).skip(start) {
            let b = self.held[i];
            if self.flights.iter().any(|fl| fl.book == b) {
                continue;
            }
            let info = &self.lib.books[b as usize];
            let jitter = ((b as f32 * 12.9898).sin() * 43758.5).fract() - 0.5;
            let mut p = base + u * (y + info.size.x * 0.5) + r * (jitter * 0.03);
            if i == self.active {
                p += -r * 0.06 - f * 0.02;
            }
            *slot = (p, Quat::from_axis_angle(u, jitter * 0.12 - 0.25) * rot);
            y += info.size.x;
        }
        for slot in &mut out[end..] {
            *slot = (base + u * y, rot);
        }
        (start..end, out)
    }

    /// Books about to leave the arms start from where they are drawn in the hand (their
    /// world pose is otherwise stale from when they arrived). Books still flying to the
    /// hand already have a live pose.
    fn leave_hand(&mut self, books: &[u32]) {
        let (_, layout) = self.held_layout();
        for &b in books {
            let in_flight = self.flights.iter().any(|f| f.book == b);
            if let Some(i) = self.held.iter().position(|&h| h == b)
                && !in_flight
            {
                (self.pos[b as usize], self.rot[b as usize]) = layout[i];
            }
        }
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
        self.series_shelved[s] += 1;
        self.category_shelved[cat] += 1;
        self.dept_shelved[self.lib.dept_of(cat)] += 1;
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
            let c = catalog::category_color(&self.lib.catalog.categories[cat]);
            self.toast(
                format!(
                    "{} is fully catalogued!  (+100 ✦)",
                    self.lib.catalog.categories[cat].name
                ),
                c,
                true,
            );
            let center = self.lib.category_center[cat];
            self.burst(center + Vec3::Y * 1.5, 120, [c[0] * 4.0, c[1] * 4.0, c[2] * 4.0], 2.0);
        }
        self.check_gates(audio);
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
        let face = &self.lib.faces[info.face as usize];
        let p = info.slot_pos + face.normal * 0.75;
        Vec3::new(p.x, face.center.y, p.z)
    }

    /// Make the held book nearest its home on foot the active one. A bookcase's far
    /// side is a walk around the row, not the width of the case.
    fn pick_nearest_active(&mut self) {
        if self.held.is_empty() {
            return;
        }
        let from = self.player.pos;
        let cost = |g: &Self, b: u32| g.nav.distance(from, g.approach(b));
        let mut best = (f32::INFINITY, 0);
        for (i, &b) in self.held.iter().enumerate() {
            let d = cost(self, b);
            if d < best.0 {
                best = (d, i);
            }
        }
        // Hysteresis, so near-ties don't flicker as you walk. Unreachable books never win.
        let current = self.active_book().map_or(f32::INFINITY, |b| cost(self, b));
        if best.0.is_finite() && best.0 < current - 0.6 {
            self.active = best.1;
        }
    }

    fn update_aura(&mut self, dt: f32) {
        let lvl = self.level(Upg::Aura);
        if lvl == 0 {
            return;
        }
        let (radius, rate) = AURA_LEVELS[lvl as usize];
        self.aura_budget = (self.aura_budget + rate * dt).min(rate * 0.5 + 1.0);
        let eye = self.player.eye();
        let prev = self.active_book();
        let mut i = 0;
        while i < self.held.len() && self.aura_budget >= 1.0 {
            let b = self.held[i];
            let arrived = !self.flights.iter().any(|f| f.book == b);
            if arrived && self.lib.books[b as usize].slot_pos.distance(eye) < radius {
                self.leave_hand(&[b]);
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
        let (speed, cap) = WINGS_LEVELS[wings];
        let cruise = 4.9f32;
        let open: Vec<bool> = (0..self.lib.catalog.departments.len())
            .map(|d| self.dept_open(d))
            .collect();
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
                    let (nav, rng) = (&self.nav, &mut self.rng);
                    let at = fam.pos;
                    let fileable = |b: u32| open[self.lib.dept_of(self.lib.category_of_book(b))];
                    let pile = (self.loose).best_pile(|_, c, books| {
                        (nav.open_at(c) && books.iter().any(|&b| fileable(b)))
                            .then(|| (c + Vec3::Y * 1.5).distance(at) + rng.gen_range(0.0..4.0))
                    });
                    match pile {
                        Some((lv, top)) => fam.task = FamTask::ToPile(lv, top),
                        None => {
                            fam.rest = 1.0;
                            continue;
                        }
                    }
                }
            }
            let target = match fam.task {
                FamTask::ToPile(_, top) => top + Vec3::Y * 0.45,
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
                    FamTask::ToPile(lv, top) => {
                        // Someone may have cleared it meanwhile; then this comes back empty.
                        // Only books whose shelves are open: the rest stay put.
                        let lib = &self.lib;
                        let fileable = |b: u32| open[lib.dept_of(lib.category_of_book(b))];
                        for b in self.loose.gather(lv, Vec2::new(top.x, top.z), 0.45, cap, fileable) {
                            self.state[b as usize] = BookState::Carried(fi as u8);
                            fam.carry.push(b);
                        }
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

    /// Stand beside the nearest reachable pile and look at its top.
    pub fn script_face_pile(&mut self) {
        let p = self.player.pos;
        let nav = &self.nav;
        // Nearby and substantial: the cell with the most books for the walk.
        let Some((_, top)) =
            (self.loose).best_pile(|_, c, n| nav.open_at(c).then(|| (c.distance(p) + 3.0) / n.len() as f32))
        else {
            return;
        };
        let mut dir = Vec3::new(p.x - top.x, 0.0, p.z - top.z).normalize_or_zero();
        if dir == Vec3::ZERO {
            dir = Vec3::Z;
        }
        let floor = self.lib.ground_at(Vec2::new(top.x, top.z), top.y).unwrap_or(0.0);
        let from = Vec3::new(top.x, floor, top.z) + dir * 1.6;
        self.look_at(from, top);
    }

    /// Stand `dist` m from `target` (on the side you are on) and look at it.
    pub fn script_look_at(&mut self, target: Vec3, dist: f32) {
        let p = self.player.pos;
        let mut dir = Vec3::new(p.x - target.x, 0.0, p.z - target.z).normalize_or_zero();
        if dir == Vec3::ZERO {
            dir = Vec3::Z;
        }
        let at = Vec3::new(target.x, 0.0, target.z) + dir * dist;
        let floor = self.lib.ground_at(Vec2::new(at.x, at.z), target.y + 0.5).unwrap_or(0.0);
        self.look_at(Vec3::new(at.x, floor, at.z), target);
    }

    /// Instantly shelve a fraction of a category (debug/screenshot aid; no rewards).
    pub fn script_fill_category(&mut self, cat: usize, frac: f32) {
        let ids: Vec<u32> = (0..self.lib.books.len() as u32)
            .filter(|&b| self.category_of(b) == cat)
            .collect();
        let n = (ids.len() as f32 * frac) as usize;
        let ids: Vec<u32> = ids
            .into_iter()
            .take(n)
            .filter(|&b| self.state[b as usize] == BookState::Loose && self.loose.is_resting(b))
            .collect();
        self.loose.take(&ids);
        for &b in &ids {
            let bi = b as usize;
            self.state[bi] = BookState::Shelved;
            self.pos[bi] = self.lib.books[bi].slot_pos;
            self.rot[bi] = self.lib.books[bi].slot_rot;
            let s = self.lib.books[bi].series as usize;
            self.series_shelved[s] += 1;
            self.category_shelved[cat] += 1;
            self.dept_shelved[self.lib.dept_of(cat)] += 1;
        }
    }

    /// Put `n` books of a category, spread across its shelves, into the arms (debug/screenshot aid).
    pub fn script_take(&mut self, cat: usize, n: usize) {
        let ids: Vec<u32> = (0..self.lib.books.len() as u32)
            .filter(|&b| self.loose.is_resting(b) && self.category_of(b) == cat)
            .collect();
        let step = (ids.len() / n.max(1)).max(1);
        let room = self.capacity().saturating_sub(self.held.len());
        let ids: Vec<u32> = ids.iter().copied().step_by(step).take(n.min(room)).collect();
        self.loose.take(&ids);
        for &b in &ids {
            self.state[b as usize] = BookState::Held;
            self.held.push(b);
        }
    }

    /// Look at familiar `i` from a little way off.
    pub fn script_face_familiar(&mut self, i: usize) {
        if let Some(f) = self.familiars.get(i) {
            let p = f.pos;
            let floor = self.lib.ground_at(Vec2::new(p.x + 1.2, p.z + 1.2), p.y).unwrap_or(0.0);
            let from = Vec3::new(p.x + 1.2, floor, p.z + 1.2);
            self.look_at(from, p);
        }
    }

    /// Stand in front of the active book's slot and look at it.
    pub fn script_face_active_slot(&mut self) {
        if let Some(b) = self.active_book() {
            let info = &self.lib.books[b as usize];
            let f = &self.lib.faces[info.face as usize];
            let from = Vec3::new(info.slot_pos.x, f.center.y, info.slot_pos.z) + f.normal * 1.8;
            self.look_at(from, info.slot_pos);
        }
    }

    // -------------------------------------------------------------- rendering

    pub fn globals(&self, aspect: f32) -> Globals {
        let eye = self.player.eye();
        let mut list: Vec<GpuLight> = Vec::with_capacity(MAX_LIGHTS);
        let push = |list: &mut Vec<GpuLight>, p: Vec3, r: f32, c: [f32; 3], i: f32| {
            list.push(GpuLight {
                pos_radius: [p.x, p.y, p.z, r],
                color: [c[0], c[1], c[2], i],
            });
        };
        // The librarian's own candle: a soft warm glow that keeps nearby spines readable.
        push(
            &mut list,
            eye - self.player.forward() * 0.6 + Vec3::Y * 0.35,
            7.0,
            [1.0, 0.72, 0.45],
            0.6,
        );
        for f in &self.familiars {
            push(&mut list, f.pos, 3.5, [0.75, 0.85, 1.0], 0.8);
        }
        // The library's lights nearest the eye (by how far their reach falls short of
        // it), as many as are left.
        let t = self.time;
        let room = MAX_LIGHTS - list.len();
        let mut near: Vec<(f32, usize)> = (self.lib.lights.iter().enumerate())
            .map(|(i, l)| (l.pos.distance(eye) - l.radius, i))
            .collect();
        if near.len() > room {
            near.select_nth_unstable_by(room, |a, b| a.0.total_cmp(&b.0));
            near.truncate(room);
        }
        for (_, li) in near {
            let l = &self.lib.lights[li];
            match l.kind {
                LightKind::Static => push(&mut list, l.pos, l.radius, l.color, l.intensity),
                LightKind::Flicker => {
                    let f = 0.88 + 0.08 * (t * 7.3 + li as f32 * 1.7).sin() + 0.05 * (t * 13.1 + li as f32).sin();
                    push(&mut list, l.pos, l.radius, l.color, l.intensity * f);
                }
                LightKind::Lantern(c) => {
                    let g = self.lantern_glow[c];
                    let cc = catalog::category_color(&self.lib.catalog.categories[c]);
                    // Completed categories glow in their colour, softened toward warm white.
                    let k = g * 0.6;
                    let col = [
                        l.color[0] * (1.0 - k) + cc[0] * k,
                        l.color[1] * (1.0 - k) + cc[1] * k,
                        l.color[2] * (1.0 - k) + cc[2] * k,
                    ];
                    push(&mut list, l.pos, l.radius + g * 2.0, col, l.intensity + g * 1.6);
                }
            }
        }
        let n = list.len() as f32;
        let mut lights = [GpuLight::default(); MAX_LIGHTS];
        lights[..list.len()].copy_from_slice(&list);
        Globals {
            view_proj: self.view_proj(aspect).to_cols_array_2d(),
            cam_pos: [eye.x, eye.y, eye.z, self.time],
            fog: [0.035, 0.03, 0.04, 0.022],
            ambient_top: [0.19, 0.18, 0.24, 0.0],
            ambient_bottom: [0.09, 0.065, 0.05, n],
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
        let mut scene = std::mem::take(&mut self.scene);
        // Books lying still are drawn from their chunks; books on the move each frame.
        for b in 0..self.lib.books.len() {
            let (st, bu) = (self.state[b], b as u32);
            let still = match st {
                BookState::Shelved => Some(self.lib.books[b].slot_pos),
                BookState::Loose if self.loose.settled(bu) => Some(self.loose.pose(bu).0),
                _ => None,
            };
            if scene.place(bu, st, self.loose.version(bu), still) {
                scene.ghost_changed(self.lib.books[b].slot_pos);
            }
            let moving = match st {
                BookState::Loose => still.is_none(),
                BookState::Held => in_flight.contains(&bu),
                BookState::Filing | BookState::Carried(_) => true,
                BookState::Shelved => false,
            };
            if moving {
                dyn_insts.push(self.book_inst(b, self.pos[b], self.rot[b], 0));
            }
        }
        // Ghosts: the series you carry glow, the one you aim at brightens, and with
        // Sort Sense the active book's slot pulses.
        let mut lit = std::collections::HashMap::new();
        for &b in &self.held {
            lit.insert(self.lib.books[b as usize].series, (1, false));
        }
        if let Aim::Shelf { run: Some(r), .. } = self.aim {
            lit.entry(r.series).or_insert((0, false)).1 = true;
        }
        scene.highlight(lit);
        let active = self.active_book().filter(|_| self.level(Upg::SortSense) > 0);
        scene.set_active(active.map(|b| (b, self.lib.books[b as usize].slot_pos)));
        if scene_gates_changed(&mut self.scene_gates, &self.gate_open) {
            scene.redraw_all();
        }
        for c in 0..scene.chunks.len() {
            if scene.chunks[c].rebuild {
                self.rebuild_chunk(&mut scene, c);
            }
        }
        self.scene = scene;

        // Held stack in view.
        let (window, layout) = self.held_layout();
        for i in window {
            let b = self.held[i];
            if !in_flight.contains(&b) {
                let (p, q) = layout[i];
                held_insts.push(self.book_inst(b as usize, p, q, (i == self.active) as u32));
            }
        }

        // Lanterns.
        for &(c, p) in &self.lib.lanterns {
            let g = self.lantern_glow[c];
            let cc = catalog::category_color(&self.lib.catalog.categories[c]);
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
        // A charging toss: a dotted arc and a ring where it lands.
        if !self.toss_arc.is_empty() {
            let warm = [1.0, 0.8, 0.5];
            for (i, &p) in self.toss_arc.iter().enumerate().skip(1) {
                let fade = 1.0 - i as f32 / (self.toss_arc.len() as f32 + 4.0);
                dyn_insts.push(Inst::new(
                    p,
                    Quat::IDENTITY,
                    Vec3::splat(0.018),
                    kind::EMISSIVE,
                    [warm[0], warm[1], warm[2], 1.6 * fade],
                    0,
                ));
            }
            if let Some(land) = self.toss_land {
                let r = 0.25 + 0.35 * self.toss_charge;
                for k in 0..16 {
                    let a = k as f32 / 16.0 * std::f32::consts::TAU + self.time * 0.6;
                    let p = land + Vec3::new(a.cos() * r, 0.03, a.sin() * r);
                    dyn_insts.push(Inst::new(
                        p,
                        Quat::from_rotation_y(-a),
                        Vec3::new(0.012, 0.012, 0.07),
                        kind::EMISSIVE,
                        [warm[0], warm[1], warm[2], 2.2],
                        0,
                    ));
                }
            }
        }

        let eye = self.player.eye();
        for (i, &(base, ph)) in self.motes.iter().enumerate() {
            let t = self.time * 0.05 + ph;
            let mut p = base + Vec3::new((t * 1.3).sin() * 0.8, (t * 0.7).sin() * 0.6, (t * 1.1).cos() * 0.8);
            // Keep motes near the player by wrapping into a 24m box around them, at the
            // height of the floor they stand on.
            let rel = p - eye;
            p.x = eye.x + (rel.x + 12.0).rem_euclid(24.0) - 12.0;
            p.z = eye.z + (rel.z + 12.0).rem_euclid(24.0) - 12.0;
            p.y += self.player.pos.y;
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

        // Moonbeams and veils.
        trans.extend_from_slice(&self.lib.static_trans);
        for (gate, &fade) in self.lib.gates.iter().zip(&self.gate_fade) {
            if fade > 0.0 {
                let size = gate.max - gate.min;
                // Drawn thinner than the solid it is.
                let thin = if size.x < size.z {
                    Vec3::new(0.05, size.y, size.z)
                } else {
                    Vec3::new(size.x, size.y, 0.05)
                };
                trans.push(Inst::new(
                    (gate.min + gate.max) * 0.5,
                    Quat::IDENTITY,
                    thin,
                    kind::VEIL,
                    [1.0, 1.0, 1.0, fade],
                    0,
                ));
            }
        }

        self.dyn_insts = dyn_insts;
        self.trans_insts = trans;
        self.held_insts = held_insts;
    }

    /// Rebuild chunk `c`'s instances: its books lying still, and the ghosts of its
    /// empty slots in open departments.
    fn rebuild_chunk(&self, scene: &mut Scene, c: usize) {
        let mut opaque = std::mem::take(&mut scene.chunks[c].opaque);
        let mut ghosts = std::mem::take(&mut scene.chunks[c].trans);
        opaque.clear();
        ghosts.clear();
        for &b in scene.books(c) {
            let (p, q) = match self.state[b as usize] {
                BookState::Shelved => (self.lib.books[b as usize].slot_pos, self.lib.books[b as usize].slot_rot),
                _ => self.loose.pose(b),
            };
            opaque.push(self.book_inst(b as usize, p, q, 0));
        }
        let active = scene.active();
        for &b in &scene.chunks[c].ghosts {
            let st = self.state[b as usize];
            let info = &self.lib.books[b as usize];
            let series = &self.lib.series[info.series as usize];
            if st == BookState::Shelved || !self.dept_open(self.lib.dept_of(series.category)) {
                continue;
            }
            let (lit, hover) = scene.lit(info.series);
            // Ghost levels (see fs_trans): waiting, carried series, active slot.
            let level = if active == Some(b) { 2 } else { lit };
            let c = series.color;
            let (col, mut a) = if level == 0 {
                let boost = 1.0 + 0.6 / (c[0] + c[1] + c[2] + 0.2);
                (c.map(|x| x * boost), if hover { 0.135 } else { 0.035 })
            } else {
                (signal_color(c), if hover { 1.3 } else { 1.0 })
            };
            if st == BookState::Filing {
                a *= 0.5;
            }
            ghosts.push(Inst::new(
                info.slot_pos,
                info.slot_rot,
                info.size * 0.995,
                kind::GHOST,
                [col[0], col[1], col[2], a],
                level,
            ));
        }
        let chunk = &mut scene.chunks[c];
        chunk.opaque = opaque;
        chunk.trans = ghosts;
        chunk.rebuild = false;
        chunk.upload = true;
    }

    // -------------------------------------------------------------- save / load

    /// Each library keeps its own save (the Hall's under its 0.1 name).
    fn save_path(&self) -> std::path::PathBuf {
        let name = match self.layout {
            Layout::Hall => "save.json",
            Layout::Great => "great.json",
        };
        crate::platform::data_dir().join(name)
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
                BookState::Loose | BookState::Held => {}
            }
        }
        let p = &self.player;
        let data = SaveData {
            seed: SEED,
            lumens: self.lumens,
            earned: self.earned,
            levels: UPGRADES
                .iter()
                .map(|&u| (format!("{u:?}"), self.level(u)))
                .filter(|&(_, l)| l > 0)
                .collect(),
            upgrades: Vec::new(),
            shelved,
            pending,
            held: self.held.clone(),
            player: [p.pos.x, p.pos.y, p.pos.z, p.yaw, p.pitch],
            play_time: self.play_time,
            loose: Some(
                self.loose
                    .moved()
                    .map(|(b, _, r)| (b, [r.c.x, r.c.y, r.c.z, r.u.x, r.u.y, r.su, r.sv]))
                    .collect(),
            ),
            loose_levels: (self.loose.moved())
                .filter(|&(_, l, _)| l > 0)
                .map(|(b, l, _)| (b, l as u8))
                .collect(),
            airborne: self.loose.airborne().map(|(b, p)| (b, p.to_array())).collect(),
        };
        let path = self.save_path();
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
        let Ok(bytes) = std::fs::read(self.save_path()) else {
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
        // Books no longer loose leave their piles; nothing is re-settled until all are placed.
        let mut lifted = Vec::new();
        for &b in &data.shelved {
            if b < n && self.state[b as usize] == BookState::Loose {
                lifted.push(b);
                self.state[b as usize] = BookState::Shelved;
                self.pos[b as usize] = self.lib.books[b as usize].slot_pos;
                self.rot[b as usize] = self.lib.books[b as usize].slot_rot;
                let s = self.lib.books[b as usize].series as usize;
                let c = self.lib.series[s].category;
                self.series_shelved[s] += 1;
                self.category_shelved[c] += 1;
                self.dept_shelved[self.lib.dept_of(c)] += 1;
            }
        }
        // Gates follow from what is shelved; lift them before anything is laid down.
        for g in 0..self.lib.gates.len() {
            if self.gate_books_left(g) == 0 {
                self.open_gate(g);
                self.gate_fade[g] = 0.0;
            }
        }
        for &b in &data.held {
            if b < n && self.state[b as usize] == BookState::Loose {
                lifted.push(b);
                self.state[b as usize] = BookState::Held;
                self.held.push(b);
            }
        }
        for &b in &data.pending {
            if b < n && self.state[b as usize] == BookState::Loose {
                lifted.push(b);
                let bi = b as usize;
                let info = &self.lib.books[bi];
                let normal = self.lib.faces[info.face as usize].normal;
                self.state[bi] = BookState::Filing;
                self.pos[bi] = info.slot_pos + normal * 0.8 + Vec3::Y * 0.4;
                self.rot[bi] = info.slot_rot;
                self.start_flight(b, Dest::Shelf, 0.8, 0.5);
            }
        }
        let loose_ok = |b: u32| b < n && self.state[b as usize] == BookState::Loose;
        // Airborne books leave their generated spot too (they fall in from where they were).
        let airborne: Vec<(u32, Vec3)> = data
            .airborne
            .iter()
            .filter(|&&(b, _)| loose_ok(b))
            .map(|&(b, p)| (b, Vec3::from_array(p)))
            .collect();
        for &b in lifted.iter().chain(airborne.iter().map(|(b, _)| b)) {
            self.loose.forget(b);
        }
        match &data.loose {
            Some(rests) => {
                let levels: std::collections::HashMap<u32, u8> = data.loose_levels.iter().copied().collect();
                for &(b, [x, y, z, ux, uz, su, sv]) in rests {
                    if loose_ok(b) {
                        let r = Rest {
                            c: Vec3::new(x, y, z),
                            u: Vec2::new(ux, uz),
                            su,
                            sv,
                        };
                        self.loose.restore(b, levels.get(&b).map_or(0, |&l| l as usize), r);
                    }
                }
                self.loose.rebuild_all();
            }
            None => {
                // A 0.1.x save: piles keep their generated shape and fall into the gaps.
                self.loose.rebuild_all();
                self.loose.resettle(&lifted);
            }
        }
        for (b, p) in airborne {
            self.loose
                .throw(b, p, Vec3::ZERO, self.rot[b as usize], Vec3::ZERO, 0.0);
        }
        for b in 0..n {
            if self.loose.is_resting(b) {
                (self.pos[b as usize], self.rot[b as usize]) = self.loose.pose(b);
            }
        }
        if data.levels.is_empty() {
            for (u, &l) in LEGACY_ORDER.iter().zip(&data.upgrades) {
                self.upg[upg_index(*u)] = l;
            }
        } else {
            for (i, u) in UPGRADES.iter().enumerate() {
                self.upg[i] = data.levels.get(&format!("{u:?}")).copied().unwrap_or(0);
            }
        }
        for (i, u) in UPGRADES.iter().enumerate() {
            self.upg[i] = self.upg[i].min(upg_def(*u).costs.len() as u32);
        }
        self.lumens = data.lumens;
        self.earned = data.earned;
        self.play_time = data.play_time;
        let [x, y, z, yaw, pitch] = data.player;
        self.player.pos = Vec3::new(x, y, z);
        self.player.yaw = yaw;
        self.player.pitch = pitch;
        for _ in 0..self.level(Upg::Familiar) {
            self.spawn_familiar();
        }
        for c in 0..self.lib.catalog.categories.len() {
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
        let g = Game::new(Layout::Hall);
        let lib = &g.lib;
        // Every book starts loose, on its own wing's side of the veil.
        for b in 0..lib.books.len() as u32 {
            assert!(g.loose.is_resting(b));
            let east = g.loose.rest(b).c.x > VEIL_X;
            assert_eq!(east, lib.dept_of(g.category_of(b)) == 1, "book {b}");
        }
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
        for b in 0..lib.books.len() as u32 {
            assert!(g.loose.rest(b).c.y < 2.0, "book {b} at {:?}", g.loose.rest(b));
        }
        // Generation is deterministic (saves depend on it).
        let g2 = Game::new(Layout::Hall);
        assert_eq!(g2.lib.books.len(), lib.books.len());
        assert!((0..lib.books.len() as u32).all(|b| g2.loose.rest(b) == g.loose.rest(b)));
        assert!(
            g2.lib
                .books
                .iter()
                .zip(&lib.books)
                .all(|(a, b)| a.slot_pos == b.slot_pos)
        );
    }

    /// Saves name books by index and rebuild unmoved piles from generation, so the
    /// Hall must generate exactly as it did (0.1.x saves included).
    #[test]
    fn hall_generates_as_before() {
        let g = Game::new(Layout::Hall);
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        let mut eat = |x: u32| {
            h ^= x as u64;
            h = h.wrapping_mul(0x0100_0000_01b3);
        };
        for (b, info) in g.lib.books.iter().enumerate() {
            eat(info.series);
            eat(info.volume);
            for v in [info.size, info.slot_pos] {
                v.to_array().iter().for_each(|f| eat(f.to_bits()));
            }
            let r = g.loose.rest(b as u32);
            for f in [r.c.x, r.c.y, r.c.z, r.u.x, r.u.y, r.su, r.sv] {
                eat(f.to_bits());
            }
        }
        for s in &g.lib.series {
            s.title.bytes().for_each(|c| eat(c as u32));
            s.color.iter().for_each(|f| eat(f.to_bits()));
            eat(s.pattern);
            eat(s.trim);
        }
        assert_eq!(g.lib.books.len(), 11690);
        assert_eq!(h, 7532805995221069936, "hall fingerprint");
    }

    /// Pacing estimate: a bot that walks (at real speeds, round furniture and up
    /// stairs) to the pile with the least walking per book, gathers an armful, files
    /// it nearest slot first, and greedily buys the cheapest upgrade. It never tosses
    /// or sifts, but it only picks up books whose shelves are open, as if sorting by
    /// eye cost nothing: in the Great Library that makes it a lower bound. Familiars
    /// shelve at the rates `familiar_throughput` measures; the aura is approximated.
    /// Returns hours.
    fn simulate(layout: Layout) -> f32 {
        let mut g = Game::new(layout);
        let mut audio = crate::audio::Audio::silent();
        let mut t = 0.0f32;
        let mut pos = g.player.pos;
        let mut next_report = 0.0;
        let speed = |g: &Game| [4.0, 5.5, 7.0][g.level(Upg::Slippers) as usize] * 1.3; // some hurrying
        let book_dept: Vec<usize> = (0..g.lib.books.len() as u32)
            .map(|b| g.lib.dept_of(g.lib.category_of_book(b)))
            .collect();
        while g.total_shelved() < g.total_books() && t < 60.0 * 60.0 * 30.0 {
            let open: Vec<bool> = (0..g.dept_total.len()).map(|d| g.dept_open(d)).collect();
            let fileable = |b: u32| open[book_dept[b as usize]];
            let count = |books: &[u32]| books.iter().filter(|&&b| fileable(b)).count();
            // Familiars: each files ~one book per (flight time + handling), approximated.
            let fams = g.level(Upg::Familiar) as f32;
            let trip_start = t;
            // Where a player would go: the fewest seconds of walking per book gathered.
            let (cap, walk) = (g.capacity(), speed(&g));
            g.nav.update(pos);
            let Some((lv, top)) = g.loose.best_pile(|_, c, books| {
                let n = count(books);
                (n > 0 && g.nav.open_at(c)).then(|| (g.nav.distance(pos, c) / walk + 1.0) / n.min(cap) as f32)
            }) else {
                break;
            };
            let floor = g.lib.ground_at(Vec2::new(top.x, top.z), top.y).unwrap_or(0.0);
            let at = g.nav.standable_near(Vec3::new(top.x, floor, top.z));
            let reach = g.gather_reach();
            let d = g.nav.distance(pos, at);
            if !d.is_finite() {
                break;
            }
            t += (d - reach).max(0.0) / walk + 1.0;
            pos = at;
            // Click, and click again at whatever is within reach until the arms are full.
            let mut target = Some(top);
            while let Some(p) = target {
                let room = cap - g.held.len();
                let radius = 0.35 + 0.05 * (room as f32).sqrt();
                for b in g.loose.gather(lv, Vec2::new(p.x, p.z), radius, room, fileable) {
                    g.state[b as usize] = BookState::Held;
                    g.held.push(b);
                }
                if g.held.len() == cap {
                    break;
                }
                target = (g.loose)
                    .best_pile(|l, c, books| {
                        (l == lv && c.distance(pos) < reach && count(books) > 0).then(|| -(count(books) as f32))
                    })
                    .map(|(_, top)| top);
                t += 0.4;
            }
            // File, nearest slot on foot first; a click files one book, or a whole
            // series with Bundle. Walks between slots are measured on the field from
            // where the armful was gathered.
            g.nav.update(pos);
            let (aura_r, aura_rate) = AURA_LEVELS[g.level(Upg::Aura) as usize];
            let mut here = pos;
            let mut here_d = 0.0f32;
            while !g.held.is_empty() {
                let (i, d) = (g.held.iter().enumerate())
                    .map(|(i, &b)| {
                        let to = g.approach(b);
                        let d = g.nav.distance(pos, to);
                        (i, if d.is_finite() { d } else { pos.distance(to) })
                    })
                    .min_by(|a, b| (a.1 - here_d).abs().total_cmp(&(b.1 - here_d).abs()))
                    .unwrap();
                let b = g.held[i];
                let to = g.approach(b);
                let same_floor = (to.y - here.y).abs() < 1.0;
                let flat = Vec2::new(to.x - here.x, to.z - here.z).length();
                let hop = if same_floor {
                    flat.max((d - here_d).abs())
                } else {
                    (d - here_d).abs().max(flat)
                };
                let walk_m = (hop - aura_r.max(1.8)).max(0.0);
                (here, here_d) = (to, d);
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
                // One click files the batch; the aura files it at its rate. Whichever is quicker.
                let click = 0.9f32;
                let file = if aura_rate > 0.0 {
                    click.min(batch.len() as f32 / aura_rate)
                } else {
                    click
                };
                t += walk_m / walk + file;
                g.held.retain(|x| !batch.contains(x));
                let mut thunks = 0;
                for x in batch {
                    g.on_shelved(x, &mut audio, &mut thunks);
                }
            }
            pos = here;
            // Familiars over the elapsed trip, at measured rates per Wings level.
            let rate = [0.095, 0.17, 0.25][g.level(Upg::Wings) as usize] * fams;
            let mut left = (rate * (t - trip_start)) as usize;
            let open: Vec<bool> = (0..g.dept_total.len()).map(|d| g.dept_open(d)).collect();
            let fileable = |b: u32| open[book_dept[b as usize]];
            while left > 0 {
                let Some((lv, top)) = g.loose.best_pile(|_, c, books| {
                    (g.nav.open_at(c) && books.iter().any(|&b| fileable(b))).then(|| c.distance(pos))
                }) else {
                    break;
                };
                let books = g.loose.gather(lv, Vec2::new(top.x, top.z), 0.45, left, fileable);
                if books.is_empty() {
                    break;
                }
                left -= books.len();
                for b in books {
                    let mut thunks = 0;
                    g.on_shelved(b, &mut audio, &mut thunks);
                }
            }
            // Spend.
            loop {
                // The bot never sifts or tosses, so it doesn't buy them.
                let cheapest = UPGRADES
                    .iter()
                    .copied()
                    .filter(|&u| u != Upg::Sift)
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
                let depts: Vec<String> = (g.dept_shelved.iter().zip(&g.dept_total).enumerate())
                    .filter(|&(d, _)| g.dept_open(d))
                    .map(|(_, (&s, &n))| format!("{:.0}%", 100.0 * s as f32 / n as f32))
                    .collect();
                println!(
                    "{:6.1} min  shelved {:6} / {}  open departments {}",
                    t / 60.0,
                    g.total_shelved(),
                    g.total_books(),
                    depts.join(" ")
                );
                next_report += 15.0 * 60.0;
            }
        }
        println!("done at {:.1} h, shelved {}", t / 3600.0, g.total_shelved());
        t / 3600.0
    }

    /// Every slot's approach (the aisle in front of it) is a walk away from the start,
    /// with every gate open.
    #[test]
    fn every_slot_can_be_walked_to() {
        for layout in Layout::ALL {
            let mut g = Game::new(layout);
            g.open_all_gates();
            let from = g.player.pos;
            g.nav.update(from);
            let stuck: Vec<u32> = (0..g.lib.books.len() as u32)
                .filter(|&b| !g.nav.distance(from, g.approach(b)).is_finite())
                .collect();
            assert!(
                stuck.is_empty(),
                "{layout:?}: {} slots unreachable, e.g. book {} approach {}",
                stuck.len(),
                stuck[0],
                g.approach(stuck[0])
            );
        }
    }

    /// Run with `--ignored --nocapture`.
    #[test]
    #[ignore]
    fn pacing_simulation() {
        simulate(Layout::Hall);
    }

    /// Run with `--ignored --nocapture`; takes a while.
    #[test]
    #[ignore]
    fn pacing_great_library() {
        simulate(Layout::Great);
    }

    /// Measures what familiars actually shelve per second, by running the real familiar
    /// code headless. Feeds the rates in `pacing_simulation`. Run with `--ignored --nocapture`.
    #[test]
    #[ignore]
    fn familiar_throughput() {
        let mut audio = crate::audio::Audio::silent();
        for wings in 0..=2 {
            for fams in [1, 5] {
                let mut g = Game::new(Layout::Hall);
                g.upg[upg_index(Upg::Wings)] = wings;
                for _ in 0..fams {
                    g.spawn_familiar();
                }
                let secs = 180.0;
                for _ in 0..(secs * 60.0) as usize {
                    g.update(1.0 / 60.0, &Input::default(), &mut audio, false);
                }
                let rate = g.total_shelved() as f32 / secs / fams as f32;
                println!("wings {wings} familiars {fams}: {rate:.3} books/s each");
            }
        }
    }

    /// Every slot must be fileable when looked at from a natural oblique standpoint
    /// (regression: a bookcase's own collision box used to block grazing rays).
    #[test]
    fn slots_aimable_at_angles() {
        let mut g = Game::new(Layout::Hall);
        g.open_all_gates();
        for b in 0..g.lib.books.len() as u32 {
            g.loose.forget(b);
        }
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
        let mut g = Game::new(Layout::Hall);
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

    /// Filing a book, by click or by the aura, flies it from the hand, not from wherever
    /// it was when it reached the arms (regression).
    #[test]
    fn filing_flies_from_the_hand() {
        let mut audio = crate::audio::Audio::silent();
        for by_aura in [false, true] {
            let mut g = Game::new(Layout::Hall);
            let b = 0u32;
            g.loose.take(&[b]);
            g.state[b as usize] = BookState::Held;
            g.held.push(b);
            // Picked up across the hall, long ago.
            g.pos[b as usize] = Vec3::new(-20.0, 0.4, 15.0);
            let at = g.approach(b);
            g.look_at(at, g.lib.books[b as usize].slot_pos);
            if by_aura {
                g.upg[upg_index(Upg::Aura)] = 1;
                g.update_aura(1.0);
            } else {
                let info = &g.lib.books[b as usize];
                let series = info.series;
                let run = g.lib.runs[info.face as usize][info.row as usize]
                    .iter()
                    .copied()
                    .find(|r| r.series == series)
                    .unwrap();
                g.aim = Aim::Shelf {
                    face: info.face,
                    row: info.row,
                    run: Some(run),
                    dist: 1.0,
                };
                g.interact(&mut audio, true);
            }
            let f = g.flights.iter().find(|f| f.book == b).expect("filing flight");
            let d = f.from_pos.distance(g.player.eye());
            assert!(d < 1.2, "aura {by_aura}: flight starts {d:.1} m from the eye");
        }
    }

    /// Every book in the Great Library starts where it can be walked to by the time
    /// its department opens (its own room, the atrium, or a room that opens before).
    #[test]
    fn great_library_books_are_reachable() {
        let mut g = Game::new(Layout::Great);
        let n = g.lib.books.len() as u32;
        assert!(n > 100_000, "{n} books");
        assert_eq!(g.lib.catalog.departments.len(), 12);
        assert!((0..n).all(|b| g.loose.is_resting(b)));
        assert_eq!(g.loose.heightfield_errors(), 0);
        let depts = g.lib.catalog.departments.len();
        for d in 0..depts {
            if let Some(gate) = g.lib.gates.iter().position(|gate| gate.opens == d) {
                g.open_gate(gate);
            }
            let stranded = (0..n)
                .filter(|&b| g.lib.dept_of(g.category_of(b)) == d && !g.nav.open_at(g.loose.pose(b).0))
                .count();
            assert_eq!(stranded, 0, "department {d}: {stranded} books out of reach");
        }
    }

    /// The player walks up a flight of stairs to the first floor, and down again.
    #[test]
    fn stairs_climb_between_floors() {
        let mut g = Game::new(Layout::Great);
        let mut audio = crate::audio::Audio::silent();
        // The foot of the west flight, facing up it (north).
        g.player.pos = Vec3::new(-10.2, 0.0, 7.5);
        g.player.yaw = 0.0;
        let walk = Input {
            forward: 1.0,
            ..Input::default()
        };
        for _ in 0..60 * 5 {
            g.update(1.0 / 60.0, &walk, &mut audio, true);
        }
        let p = g.player.pos;
        assert!((p.y - 6.0).abs() < 0.01 && p.z < -6.0, "up: {p}");
        assert_eq!(g.lib.level_at(p), 1);
        g.player.yaw = PI;
        for _ in 0..60 * 6 {
            g.update(1.0 / 60.0, &walk, &mut audio, true);
        }
        let p = g.player.pos;
        assert!(p.y.abs() < 0.01 && p.z > 6.0, "down: {p}");
    }

    /// Books tossed over a gallery railing fall down the well to the ground floor.
    #[test]
    fn books_fall_down_the_well() {
        let mut g = Game::new(Layout::Great);
        let mut audio = crate::audio::Audio::silent();
        let (lv, top) = g.loose.best_pile(|l, c, _| (l == 0).then(|| c.length())).unwrap();
        let books = g.loose.gather(lv, Vec2::new(top.x, top.z), 1.0, 12, |_| true);
        assert_eq!(books.len(), 12);
        for (i, &b) in books.iter().enumerate() {
            let from = Vec3::new(11.5, 13.6, -1.2 + i as f32 * 0.2);
            g.loose
                .throw(b, from, Vec3::new(-5.0, 3.0, 0.0), Quat::IDENTITY, Vec3::ONE, 0.0);
        }
        for _ in 0..60 * 6 {
            g.update(1.0 / 60.0, &Input::default(), &mut audio, false);
        }
        for &b in &books {
            let p = g.loose.pose(b).0;
            assert!(g.loose.is_resting(b) && g.loose.level_of(b) == 0, "book {b} at {p}");
            assert!(p.y < 3.5 && p.x.abs() < 9.0 && p.z.abs() < 9.0, "book {b} at {p}");
        }
        assert_eq!(g.loose.heightfield_errors(), 0);
    }

    /// The way to a slot upstairs goes by the stairs.
    #[test]
    fn routes_climb_the_stairs() {
        let mut g = Game::new(Layout::Great);
        g.open_all_gates();
        let from = g.player.pos;
        g.nav.update(from);
        for dept in [4, 8] {
            let b = (0..g.lib.books.len() as u32)
                .find(|&b| g.lib.dept_of(g.category_of(b)) == dept)
                .unwrap();
            let to = g.approach(b);
            let d = g.nav.distance(from, to);
            let r = g.nav.route(from, to).expect("a route");
            // Straight-line distance is no measure of a walk up a storey or two.
            assert!(
                d.is_finite() && d > from.distance(to) + 5.0 && d < 120.0,
                "dept {dept}: {d}"
            );
            assert!(!r.direct && r.waypoint.y < 0.5, "{r:?}");
        }
    }

    /// Chunks hold exactly the books lying still (in piles or on shelves), each where
    /// it lies, through gathering, tossing, landing and filing.
    #[test]
    fn chunks_hold_the_still_books() {
        let mut g = Game::new(Layout::Hall);
        let mut audio = crate::audio::Audio::silent();
        let check = |g: &mut Game| {
            g.build_instances();
            let mut seen = vec![0u32; g.lib.books.len()];
            for c in 0..g.scene.chunks.len() {
                let ch = &g.scene.chunks[c];
                assert!(!ch.rebuild);
                assert_eq!(ch.opaque.len(), g.scene.books(c).len());
                for &b in g.scene.books(c) {
                    seen[b as usize] += 1;
                    let at = match g.state[b as usize] {
                        BookState::Shelved => g.lib.books[b as usize].slot_pos,
                        _ => g.loose.pose(b).0,
                    };
                    assert!(
                        at.cmpge(ch.min).all() && at.cmple(ch.max).all(),
                        "book {b} outside its chunk"
                    );
                }
            }
            for b in 0..g.lib.books.len() as u32 {
                let still = match g.state[b as usize] {
                    BookState::Shelved => true,
                    BookState::Loose => g.loose.settled(b),
                    _ => false,
                };
                assert_eq!(seen[b as usize], still as u32, "book {b} {:?}", g.state[b as usize]);
            }
        };
        check(&mut g);
        g.upg[upg_index(Upg::Arms)] = 3;
        g.script_face_pile();
        for _ in 0..3 {
            g.update(1.0 / 60.0, &Input::default(), &mut audio, true);
            g.interact(&mut audio, true);
        }
        check(&mut g);
        assert!(g.held.len() > 10, "gathered {}", g.held.len());
        // Toss some, file one, and check mid-flight and after everything has landed.
        g.toss_charge = 0.6;
        g.toss(&mut audio);
        if let Some(b) = g.active_book() {
            g.leave_hand(&[b]);
            g.held.retain(|&h| h != b);
            g.state[b as usize] = BookState::Filing;
            g.start_flight(b, Dest::Shelf, 0.42, 0.0);
        }
        for k in 0..240 {
            g.update(1.0 / 60.0, &Input::default(), &mut audio, true);
            if k % 20 == 0 {
                check(&mut g);
            }
        }
        check(&mut g);
        assert!(g.total_shelved() >= 1);
    }

    /// Pressing into a thin solid (a closed veil, a gallery rail) never carries the
    /// player through it, even sprinting at the longest frame step (regression: one
    /// 50 ms frame stepped through a gate).
    #[test]
    fn thin_walls_hold_at_long_frames() {
        let mut audio = crate::audio::Audio::silent();
        let sprint = Input {
            forward: 1.0,
            sprint: true,
            ..Input::default()
        };
        // (layout, start, facing, the x the player must stay short of)
        let cases = [
            (Layout::Hall, Vec3::new(10.5, 0.0, 3.0), PI * 0.5, VEIL_X),
            (Layout::Great, Vec3::new(12.0, 0.0, 0.2), PI * 0.5, 14.0),
            (Layout::Great, Vec3::new(12.0, 6.0, 0.0), -PI * 0.5, 9.0),
        ];
        for (layout, start, yaw, edge) in cases {
            let mut g = Game::new(layout);
            g.upg[upg_index(Upg::Slippers)] = 2;
            g.player.pos = start;
            g.player.yaw = yaw;
            for _ in 0..60 {
                g.update(0.05, &sprint, &mut audio, true);
            }
            let p = g.player.pos;
            let held = if yaw > 0.0 { p.x < edge } else { p.x > edge };
            assert!(
                held && (p.y - start.y).abs() < 0.01,
                "{layout:?} from {start}: ended at {p}"
            );
        }
    }

    /// The stairs are solid stone: walking into a flight from its high end on the
    /// floor below (under the gallery) is blocked.
    #[test]
    fn stairs_are_solid_from_below() {
        let mut g = Game::new(Layout::Great);
        let mut audio = crate::audio::Audio::silent();
        // Under the first-floor gallery, north of the west flight's top, facing south.
        g.player.pos = Vec3::new(-10.2, 0.0, -8.0);
        g.player.yaw = PI;
        let walk = Input {
            forward: 1.0,
            ..Input::default()
        };
        for _ in 0..60 * 4 {
            g.update(1.0 / 60.0, &walk, &mut audio, true);
        }
        let p = g.player.pos;
        assert!(p.z < -5.5 - 0.25 && p.y == 0.0, "walked into the stairs: {p}");
    }

    #[test]
    fn save_round_trip() {
        let dir = std::env::temp_dir().join(format!("hushfold-test-{}", std::process::id()));
        // SAFETY: tests in this module don't read the environment concurrently.
        unsafe { std::env::set_var("XDG_DATA_HOME", &dir) };
        let mut g = Game::new(Layout::Hall);
        let mut audio = crate::audio::Audio::silent();
        let pile = |g: &Game| {
            let from = Vec3::new(-16.0, 0.0, 0.0);
            g.loose
                .best_pile(|_, c, _| (c.x < VEIL_X).then(|| c.distance(from)))
                .unwrap()
                .1
        };
        // Shelve the tops of a few piles (as play does), leave one book mid-flight.
        let mut thunks = 0;
        for _ in 0..5 {
            let top = pile(&g);
            for b in g.loose.gather(0, Vec2::new(top.x, top.z), 0.6, 20, |_| true) {
                g.on_shelved(b, &mut audio, &mut thunks);
            }
        }
        let top = pile(&g);
        let filing = g.loose.gather(0, Vec2::new(top.x, top.z), 0.5, 1, |_| true)[0];
        g.state[filing as usize] = BookState::Filing;
        // Three books in the arms, two tossed: one has landed, one is in the air at save time.
        let top = pile(&g);
        let books = g.loose.gather(0, Vec2::new(top.x, top.z), 0.6, 5, |_| true);
        g.take_into_arms(&books, &mut audio);
        let (landed, flying) = (books[3], books[4]);
        g.held.retain(|&b| b != landed && b != flying);
        g.active = 0;
        let throw = |g: &mut Game, b: u32| {
            g.state[b as usize] = BookState::Loose;
            g.flights.retain(|f| f.book != b);
            g.loose.throw(
                b,
                Vec3::new(-6.0, 1.5, 0.0),
                Vec3::new(-3.0, 2.0, 1.0),
                Quat::IDENTITY,
                Vec3::X * 5.0,
                0.0,
            );
        };
        throw(&mut g, landed);
        for _ in 0..180 {
            g.update(1.0 / 60.0, &Input::default(), &mut audio, false);
        }
        assert!(g.loose.is_resting(landed));
        throw(&mut g, flying);
        g.update(1.0 / 60.0, &Input::default(), &mut audio, false);
        assert!(g.loose.is_flying(flying));
        g.lumens = 42;
        g.upg[upg_index(Upg::Arms)] = 2;
        g.upg[upg_index(Upg::Sift)] = 1;
        g.save();

        let mut l = Game::new(Layout::Hall);
        assert!(l.load());
        assert_eq!(l.total_shelved(), g.total_shelved());
        assert_eq!(l.lumens, 42);
        assert_eq!(l.capacity(), 24);
        assert_eq!(l.level(Upg::Sift), 1);
        assert_eq!(l.held, g.held);
        // Every book lying still lies exactly where it did, and the ground under the
        // airborne book's old spot is gone with it.
        assert_eq!(l.loose.heightfield_errors(), 0);
        for b in 0..g.lib.books.len() as u32 {
            if g.loose.is_resting(b) {
                assert_eq!(l.loose.rest(b), g.loose.rest(b), "book {b}");
            }
        }
        // The in-flight book resumes its flight and is paid for when it lands; the
        // airborne one falls to the floor.
        assert_eq!(l.state[filing as usize], BookState::Filing);
        assert!(l.loose.is_flying(flying));
        for _ in 0..120 {
            l.update(1.0 / 60.0, &Input::default(), &mut audio, false);
        }
        assert_eq!(l.state[filing as usize], BookState::Shelved);
        assert_eq!(l.total_shelved(), g.total_shelved() + 1);
        assert_eq!(l.lumens, 43);
        assert!(l.loose.is_resting(flying));

        // The Great Library saves beside it: books upstairs come back on their floors,
        // and gates open as far as what is shelved says.
        let mut g = Game::new(Layout::Great);
        let up = |g: &Game| g.loose.best_pile(|l, c, _| (l == 2).then(|| c.length())).unwrap();
        for _ in 0..3 {
            let (lv, top) = up(&g);
            for b in g.loose.gather(lv, Vec2::new(top.x, top.z), 0.6, 15, |_| true) {
                g.on_shelved(b, &mut audio, &mut thunks);
            }
        }
        let (lv, top) = up(&g);
        let tossed = g.loose.gather(lv, Vec2::new(top.x, top.z), 0.6, 6, |_| true);
        for (i, &b) in tossed.iter().enumerate() {
            // Onto the second-floor gallery, and the first-floor one below it.
            let (from, vel) = if i % 2 == 0 {
                (Vec3::new(12.0, 13.5, -3.0 + i as f32 * 0.3), Vec3::new(0.0, 1.0, 1.0))
            } else {
                (Vec3::new(12.0, 7.5, -3.0 + i as f32 * 0.3), Vec3::new(0.0, 1.0, 1.0))
            };
            g.loose.throw(b, from, vel, Quat::IDENTITY, Vec3::ONE, 0.0);
        }
        for _ in 0..180 {
            g.update(1.0 / 60.0, &Input::default(), &mut audio, false);
        }
        let levels: Vec<usize> = tossed.iter().map(|&b| g.loose.level_of(b)).collect();
        assert!(levels.contains(&1) && levels.contains(&2), "{levels:?}");
        // Enough of the first department home to lift the first gate.
        let first: Vec<u32> = (0..g.lib.books.len() as u32)
            .filter(|&b| g.lib.dept_of(g.category_of(b)) == 0 && g.loose.is_resting(b))
            .take(g.gate_books_left(0) as usize)
            .collect();
        g.loose.take(&first);
        for &b in &first {
            g.on_shelved(b, &mut audio, &mut thunks);
        }
        assert!(g.gate_open[0] && !g.gate_open[1]);
        g.save();
        let mut l = Game::new(Layout::Great);
        assert!(l.load());
        assert_eq!(l.total_shelved(), g.total_shelved());
        assert_eq!(l.gate_open, g.gate_open);
        assert_eq!(l.loose.heightfield_errors(), 0);
        for b in 0..g.lib.books.len() as u32 {
            if g.loose.is_resting(b) {
                assert_eq!(l.loose.rest(b), g.loose.rest(b), "book {b}");
                assert_eq!(l.loose.level_of(b), g.loose.level_of(b), "book {b}");
            }
        }
        // The Hall's save is still there, apart.
        let mut hall = Game::new(Layout::Hall);
        assert!(hall.load());
        assert_eq!(hall.lumens, 42);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
