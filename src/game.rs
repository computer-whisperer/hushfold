//! Game state: player, books in motion, upgrades, progression, save/load (familiars
//! in `familiar`, cheats in `cheats`).

mod cheats;
mod familiar;
#[cfg(test)]
mod pacing;

pub use cheats::Cheat;
use familiar::Familiar;
pub use familiar::Role;
mod spells;
use spells::Stream;
pub use spells::{ECHO_SPEED, SPELLS, spell_times};

use crate::audio::{Audio, Sfx};
use crate::catalog;
use crate::gfx::{Globals, GpuLight, Inst, MAX_LIGHTS, kind};
use crate::library::*;
use crate::loose::{Loose, Rest};
use crate::nav::{Field, Nav, Route};
use crate::scene::Scene;
use crate::world::{BODY_R, STEP_UP, Stops};
use glam::{Mat4, Quat, Vec2, Vec3};
use rand::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::f32::consts::PI;

pub const SEED: u64 = 0x4855_5348_464f_4c44;
const EYE: f32 = 1.62;
const SHELF_REACH: f32 = 4.2;
const HAND_VISIBLE: usize = 8;
/// Shelving Aura by level: (radius in metres, books per second).
const AURA_LEVELS: [(f32, f32); 4] = [(0.0, 0.0), (3.0, 2.0), (4.5, 4.0), (6.0, 8.0)];
/// Sifting Touch by level: the largest scoop while seeking, in metres.
const SIFT_RADIUS: [f32; 5] = [0.0, 0.9, 1.8, 2.7, 3.6];
/// Seconds a press on a pile may last and still take only the book pressed on.
const TAP: f32 = 0.18;
/// Seconds over which a held scoop's ring then grows to its largest.
const SCOOP_GROW: f32 = 0.4;
/// Seconds to charge a toss fully.
const TOSS_CHARGE: f32 = 0.9;
/// Toss speed in m/s, uncharged.
const TOSS_MIN: f32 = 3.0;
/// Long Toss by level: (fully charged speed in m/s, how widely the books scatter).
const LONG_TOSS: [(f32, f32); 3] = [(12.0, 1.0), (16.0, 0.7), (20.0, 0.45)];
/// Homing Toss by level: how far from its slot (along the floor, in front of its
/// shelf) a tossed book may land and still slip into it.
const HOMING: [f32; 3] = [0.0, 2.0, 3.5];

/// Vertical FOV: `v_deg` on wide windows, widened on narrow ones so the horizontal
/// FOV never drops below `v_deg` × 85/70 (85° at the default 70°).
fn vfov(aspect: f32, v_deg: f32) -> f32 {
    let min_h = (v_deg * 85.0 / 70.0).to_radians();
    let from_h = 2.0 * ((min_h * 0.5).tan() / aspect).atan();
    v_deg.to_radians().max(from_h).min(110f32.to_radians())
}

// ------------------------------------------------------------------ upgrades

/// What a skill works on, for the Grimoire's sections: finding where books go,
/// moving many books roughly, filing them exactly, and the helpers.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tier {
    Reading,
    Moving,
    Filing,
    Familiars,
    Spells,
}

impl Tier {
    pub const ALL: [Tier; 5] = [Tier::Reading, Tier::Moving, Tier::Filing, Tier::Familiars, Tier::Spells];

    pub fn name(self) -> &'static str {
        match self {
            Tier::Reading => "Reading",
            Tier::Moving => "Moving",
            Tier::Filing => "Filing",
            Tier::Familiars => "Familiars",
            Tier::Spells => "Spells",
        }
    }

    pub fn blurb(self) -> &'static str {
        match self {
            Tier::Reading => "Knowing where a book belongs.",
            Tier::Moving => "Moving many books a long way, roughly.",
            Tier::Filing => "Putting books exactly in their place.",
            Tier::Familiars => "Paper helpers that work where you post them.",
            Tier::Spells => "Bursts of help, each with its own recharge; cast with the number keys.",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Upg {
    Signposts,
    Glow,
    SortSense,
    Wayfinding,
    Lens,
    Sift,
    Broad,
    Fine,
    LongToss,
    Homing,
    Lodestone,
    Arms,
    Slippers,
    Tidy,
    Bundle,
    Aura,
    Shelver,
    Courier,
    Wings,
    Glimpse,
    Gathering,
    Echo,
    Summon,
    Rush,
}

/// Every skill, in the Grimoire's order within its tier.
pub const UPGRADES: [Upg; 24] = [
    Upg::Signposts,
    Upg::Glow,
    Upg::SortSense,
    Upg::Wayfinding,
    Upg::Lens,
    Upg::Sift,
    Upg::Broad,
    Upg::Fine,
    Upg::LongToss,
    Upg::Homing,
    Upg::Lodestone,
    Upg::Arms,
    Upg::Slippers,
    Upg::Tidy,
    Upg::Bundle,
    Upg::Aura,
    Upg::Shelver,
    Upg::Courier,
    Upg::Wings,
    Upg::Glimpse,
    Upg::Gathering,
    Upg::Echo,
    Upg::Summon,
    Upg::Rush,
];

/// Kindred Glow by level: (reach in metres, strength, glint).
const GLOW_LEVELS: [[f32; 3]; 4] = [[0.0, 0.0, 0.0], [3.0, 0.45, 0.0], [12.0, 0.8, 0.0], [1.0e4, 1.0, 1.0]];

pub struct UpgDef {
    pub name: &'static str,
    pub blurb: &'static str,
    pub tier: Tier,
}

pub fn upg_def(u: Upg) -> UpgDef {
    let (name, blurb, tier) = match u {
        Upg::Signposts => (
            "Signposts",
            "The signs of the categories you carry show through the stacks, and say when their shelves face away.",
            Tier::Reading,
        ),
        Upg::Glow => (
            "Kindred Glow",
            "The runs of the series you carry glow: faintly on the case in front of you, then across the room, then everywhere.",
            Tier::Reading,
        ),
        Upg::SortSense => (
            "Sort Sense",
            "The active book's slot pulses and a compass points the way: while you are in its room, then anywhere.",
            Tier::Reading,
        ),
        Upg::Wayfinding => (
            "Wayfinding",
            "Out of Sort Sense's reach, the compass points the way to the active book's room.",
            Tier::Reading,
        ),
        Upg::Lens => (
            "Seeker's Lens",
            "While you seek, loose books of the kind you seek glow and the rest fade.",
            Tier::Reading,
        ),
        Upg::Sift => (
            "Sifting Touch",
            "Press F at a book to seek its kind: a held scoop then takes only those books, from any depth. Each level scoops wider.",
            Tier::Moving,
        ),
        Upg::Broad => (
            "Broad Strokes",
            "Press R to seek by department: scoop and toss whole departments at once.",
            Tier::Moving,
        ),
        Upg::Fine => (
            "Fine Fingers",
            "Press R to seek by series: scoop and toss a single series.",
            Tier::Moving,
        ),
        Upg::LongToss => ("Long Toss", "Throw harder, and the books scatter less.", Tier::Moving),
        Upg::Homing => (
            "Homing Toss",
            "A book you toss that lands in front of its own shelf, within a couple of metres of its slot and in sight of it, slips into it. The second level reaches further.",
            Tier::Moving,
        ),
        Upg::Lodestone => ("Lodestone Grasp", "Take books from piles at a distance.", Tier::Moving),
        Upg::Arms => ("Deeper Arms", "Carry more books at once.", Tier::Filing),
        Upg::Slippers => ("Swift Slippers", "Pad through the stacks faster.", Tier::Filing),
        Upg::Tidy => (
            "Tidy Arms",
            "Your armful sorts itself, and the book nearest its home rises to the top.",
            Tier::Filing,
        ),
        Upg::Bundle => (
            "Series Bundle",
            "Filing one volume files every volume of that series you carry.",
            Tier::Filing,
        ),
        Upg::Aura => (
            "Shelving Aura",
            "Books you carry drift to their slots whenever you pass close in front of their shelves.",
            Tier::Filing,
        ),
        Upg::Shelver => (
            "Shelver",
            "A paper crane that rides with you until you post it (G). It files loose books lying near its post whose shelves are close by.",
            Tier::Familiars,
        ),
        Upg::Courier => (
            "Courier",
            "A paper crane that carries loose books lying near its post (G) to a shelver that can file them, or else to their room.",
            Tier::Familiars,
        ),
        Upg::Wings => (
            "Familiar Wings",
            "Familiars fly faster, carry more, and reach further from their posts.",
            Tier::Familiars,
        ),
        Upg::Glimpse => (
            "Glimpse",
            "For a few seconds, every series you carry glows wherever it is, and a marker shows each through the walls.",
            Tier::Spells,
        ),
        Upg::Echo => (
            "Echo",
            "A ring sweeps out from you: the heaps holding the kind of book you seek (or else the active book's kind) are marked through the walls for a while.",
            Tier::Spells,
        ),
        Upg::Summon => (
            "Summon Series",
            "Loose volumes of the active book's series fly to your arms: from the room you are in, then from the whole floor, then from anywhere.",
            Tier::Spells,
        ),
        Upg::Rush => (
            "Shelving Rush",
            "For a few seconds, every book you carry within a few metres of its slot, and in front of it, flies home as fast as you pass.",
            Tier::Spells,
        ),
        Upg::Gathering => (
            "Gathering Charm",
            "Cast at a heap: its books rise and stream to the hub, the heap in the middle of the library. While you seek, only those books go. Each level takes a wider heap and recharges sooner.",
            Tier::Spells,
        ),
    };
    UpgDef { name, blurb, tier }
}

/// What a skill costs in `layout`, level by level; empty where that library doesn't
/// offer it. The same skill does the same thing in both.
pub fn upg_costs(layout: Layout, u: Upg) -> &'static [u64] {
    match layout {
        Layout::Hall => match u {
            Upg::Signposts => &[8],
            Upg::Glow => &[10, 89, 370],
            Upg::Arms => &[20, 48, 130, 440],
            Upg::Sift => &[26, 171],
            Upg::Slippers => &[41, 1201],
            Upg::Glimpse => &[49, 241],
            Upg::Tidy => &[53],
            Upg::Bundle => &[77],
            Upg::Lens => &[86],
            Upg::Shelver => &[110, 220, 1200],
            Upg::Summon => &[120, 530],
            Upg::Fine => &[160],
            Upg::Echo => &[170, 1100],
            Upg::LongToss => &[200],
            Upg::Aura => &[201, 470, 1400],
            Upg::Courier => &[240, 430],
            Upg::Rush => &[260, 1300],
            Upg::SortSense => &[280, 1401],
            Upg::Homing => &[290, 1301],
            Upg::Wings => &[400, 1500],
            Upg::Lodestone => &[560],
            // Every book starts in its own wing: no rooms to find, nothing to gather in.
            Upg::Wayfinding | Upg::Broad | Upg::Gathering => &[],
        },
        Layout::Great => match u {
            Upg::Signposts => &[11],
            Upg::Glow => &[20, 300, 980],
            Upg::Arms => &[38, 120, 520, 5100],
            Upg::Sift => &[72, 1000, 9800, 11006],
            Upg::Slippers => &[82, 9801],
            Upg::Glimpse => &[150, 1900],
            Upg::Tidy => &[210],
            Upg::Bundle => &[290],
            Upg::Lens => &[291],
            Upg::Fine => &[320],
            Upg::Shelver => &[360, 1400, 7800, 11004, 11008],
            Upg::Summon => &[450, 5200, 11002],
            Upg::Aura => &[610, 5101, 5800],
            Upg::Courier => &[720, 2500, 9803, 11007],
            Upg::Gathering => &[1001, 3900, 9805],
            Upg::LongToss => &[1100, 11000],
            Upg::Broad => &[1200],
            Upg::Wayfinding => &[1800],
            Upg::Rush => &[1901, 9802],
            Upg::SortSense => &[2000, 6200],
            Upg::Homing => &[2200, 9804],
            Upg::Echo => &[2400, 11005],
            Upg::Wings => &[2900, 11003],
            Upg::Lodestone => &[11001],
        },
    }
}

/// The level of the hierarchy that toss and sift work at: every book of the active
/// (or aimed) book's department, category or series.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Group {
    Department,
    Category,
    Series,
}

impl Group {
    /// Which department, category or series book `b` is in.
    pub fn of(self, lib: &Library, b: u32) -> usize {
        let series = lib.books[b as usize].series as usize;
        match self {
            Group::Department => lib.dept_of(lib.series[series].category),
            Group::Category => lib.series[series].category,
            Group::Series => series,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Group::Department => "department",
            Group::Category => "category",
            Group::Series => "series",
        }
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
    /// Flying to the hub on a Gathering Charm.
    Streaming,
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

struct Particle {
    pos: Vec3,
    vel: Vec3,
    life: f32,
    max: f32,
    color: [f32; 3],
    size: f32,
}

/// The compass: the way to the active book's slot (Sort Sense), or to its
/// department's room (Wayfinding).
#[derive(Clone, Copy, Debug)]
pub struct Sense {
    pub route: Route,
    /// The department whose doorway the route leads to; `None` for the slot.
    pub room: Option<usize>,
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
    /// A posted familiar (its crane or its perch).
    Familiar(usize),
}

/// A press on a pile while the button is held: released quickly it takes the book
/// pressed on; held, a ring grows round the aim and releasing takes what is in it.
#[derive(Clone, Copy, Debug)]
pub struct Scoop {
    /// The book under the crosshair when pressed.
    book: u32,
    /// Seconds held.
    held: f32,
    /// The ring's centre and level: the aim point, followed while it is on a pile.
    pub at: Vec3,
    lv: usize,
}

#[derive(Default)]
pub struct Input {
    pub forward: f32,
    pub strafe: f32,
    pub sprint: bool,
    pub look: Vec2,
    /// Left button: a tap takes one book, a hold scoops; a press on a run files.
    pub primary_down: bool,
    pub primary_pressed: bool,
    /// Right button: held to charge a toss of a group; released to throw.
    pub toss_down: bool,
    /// Pressed this frame (a press and release within one frame still tosses).
    pub toss_pressed: bool,
    /// X: the same for the whole armful.
    pub armful_down: bool,
    pub armful_pressed: bool,
    /// F: seek the aimed book's group, or stop.
    pub seek: bool,
    pub cycle: i32,
    /// R: the next sorting group.
    pub regroup: bool,
    /// G: post a familiar, or take one back.
    pub post: bool,
    /// 1–5: cast the spell in that hotbar slot.
    pub cast: Option<usize>,
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

/// Saves before phase 3's skills are version 0 (the field is absent); version 1
/// had the guide signs and the glow of carried series free.
const SAVE_VERSION: u32 = 2;

/// Which generation of its library a save was made in: saves name books by index,
/// so one made in another generation would put the wrong books on the shelves. The
/// Hall's is pinned (`hall_generates_as_before`), so its stamp stays 0, as 0.1.x
/// saves have it. Bump the Great Library's whenever where its books go changes.
fn generation(layout: Layout) -> u32 {
    match layout {
        Layout::Hall => 0,
        Layout::Great => 2,
    }
}

#[derive(Serialize, Deserialize)]
struct SaveData {
    #[serde(default)]
    version: u32,
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
    /// Books mid-flight to a shelf; they fly home on load so their rewards are paid
    /// through the normal path.
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
    /// Each familiar's role and where it is posted (none: riding).
    #[serde(default)]
    familiars: Vec<(String, Option<[f32; 3]>)>,
    /// See `generation`.
    #[serde(default)]
    generation: u32,
}

/// The 0.1.x upgrade order, for reading old saves.
const LEGACY_ORDER: [Upg; 9] = [
    Upg::Arms,
    Upg::Slippers,
    Upg::SortSense,
    Upg::Tidy,
    Upg::Bundle,
    Upg::Lodestone,
    Upg::Shelver,
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
    /// How much of each gate's veil is still drawn (1 closed, 0 gone).
    gate_fade: Vec<f32>,
    familiars: Vec<Familiar>,
    particles: Vec<Particle>,
    motes: Vec<(Vec3, f32)>,
    pub toasts: Vec<Toast>,
    pub aim: Aim,
    pub time: f32,
    pub play_time: f64,
    /// A press on a pile, while the button is held.
    pub scoop: Option<Scoop>,
    /// Toss charge, 0..1, while the toss button is held.
    pub toss_charge: f32,
    toss_was_down: bool,
    /// The toss charging throws the whole armful (X) rather than a group.
    toss_armful: bool,
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
    /// Walking distances from the player.
    walk: Field,
    /// Where the compass points, if anywhere.
    pub sense: Option<Sense>,
    /// The level toss and seeking work at: department, category or series.
    pub group: Group,
    /// While seeking: a book of the group sought (at `group`'s level, so R changes
    /// what is sought).
    pub seek: Option<u32>,
    /// Each spell's seconds until it can be cast again, and seconds its effect has
    /// left, by hotbar slot.
    pub spell_wait: [f32; SPELLS.len()],
    pub spell_left: [f32; SPELLS.len()],
    /// Books on their way to the hub, and walking distances from it.
    streams: Vec<Stream>,
    hub_field: Field,
    /// Books the player has tossed that have not landed yet, by book.
    tossed: Vec<bool>,
    /// Echo: the heaps marked (above their top, how many books of the kind), where
    /// it rang out from, and the kind.
    pub echo: Vec<(Vec3, u32)>,
    pub echo_from: Vec3,
    pub echo_kind: Option<(Group, usize)>,
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
        let nav = Nav::new(&lib.world, lib.spawn.0);
        let hub_field = nav.field();
        let pos = (0..n as u32).map(|b| loose.pose(b).0).collect();
        let rot = (0..n as u32).map(|b| loose.pose(b).1).collect();
        let mut rng = StdRng::seed_from_u64(0x5eed);
        let ground = &lib.world.levels[0];
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
            gate_fade: vec![1.0; lib.world.gates().len()],
            familiars: Vec::new(),
            particles: Vec::new(),
            motes,
            toasts: Vec::new(),
            aim: Aim::None,
            time: 0.0,
            play_time: 0.0,
            scoop: None,
            toss_charge: 0.0,
            toss_was_down: false,
            toss_armful: false,
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
            scene_gates: vec![false; lib.world.gates().len()],
            loose,
            walk: nav.field(),
            nav,
            sense: None,
            group: Group::Category,
            seek: None,
            spell_wait: [0.0; SPELLS.len()],
            spell_left: [0.0; SPELLS.len()],
            streams: Vec::new(),
            hub_field,
            tossed: vec![false; n],
            echo: Vec::new(),
            echo_from: Vec3::ZERO,
            echo_kind: None,
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
        upg_costs(self.layout, u).get(self.level(u) as usize).copied()
    }

    pub fn buy(&mut self, u: Upg, audio: &mut Audio) -> bool {
        let Some(cost) = self.next_cost(u) else { return false };
        if self.lumens < cost {
            return false;
        }
        self.lumens -= cost;
        self.upg[upg_index(u)] += 1;
        audio.play(Sfx::Purchase, 0.7);
        let role = match u {
            Upg::Shelver => Some(Role::Shelver),
            Upg::Courier => Some(Role::Courier),
            _ => None,
        };
        if let Some(role) = role {
            self.spawn_familiar(role);
            let msg = format!(
                "Your {} rides with you: aim at the floor and press G to post it.",
                role.name()
            );
            self.hint = Some((msg, 4.0));
        }
        if u == Upg::Tidy {
            let prev = self.active_book();
            self.held.sort();
            self.follow_active(prev);
        }
        let def = upg_def(u);
        let news = if let Some(role) = role {
            format!("A {} joins you", role.name())
        } else if upg_costs(self.layout, u).len() > 1 {
            let roman = ["I", "II", "III", "IV", "V"];
            let level = roman[(self.level(u) as usize - 1).min(roman.len() - 1)];
            format!("{} {level} learned", def.name)
        } else {
            format!("{} learned", def.name)
        };
        self.toast(news, [0.95, 0.8, 0.45], false);
        self.dirty_save = true;
        true
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
        let gate = &self.lib.world.gates()[g];
        let total: u32 = gate.counts.iter().map(|&d| self.dept_total[d]).sum();
        let done: u32 = gate.counts.iter().map(|&d| self.dept_shelved[d]).sum();
        let need = (total as f32 * gate.need).ceil() as u32;
        need.saturating_sub(done)
    }

    /// The first gate still closed.
    pub fn next_gate(&self) -> Option<usize> {
        self.lib.world.gates_open().iter().position(|&o| !o)
    }

    /// A department is open once the gate before it has lifted.
    pub fn dept_open(&self, d: usize) -> bool {
        self.lib.world.closed_gates().all(|(_, g)| g.opens != d)
    }

    fn open_gate(&mut self, g: usize) {
        self.lib.world.open_gate(g);
        self.loose.set_gate(g, false);
        self.nav.set_gate(g, false);
    }

    /// Lift every gate whose departments are shelved far enough.
    fn check_gates(&mut self, audio: &mut Audio) {
        for g in 0..self.lib.world.gates().len() {
            if !self.lib.world.gates_open()[g] && self.gate_books_left(g) == 0 {
                self.open_gate(g);
                audio.play(Sfx::Fanfare, 0.8);
                let name = &self.lib.catalog.departments[self.lib.world.gates()[g].opens].name;
                self.toast(format!("The veil over the {name} lifts…"), [0.75, 0.6, 1.0], true);
            }
        }
    }

    /// Script aid: lift every gate.
    pub fn open_all_gates(&mut self) {
        for g in 0..self.lib.world.gates().len() {
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
            if input.regroup {
                let groups = self.groups();
                let at = groups.iter().position(|&g| g == self.group).unwrap_or(0);
                self.group = groups[(at + 1) % groups.len()];
                audio.play(Sfx::Tick, 0.25);
                if groups.len() == 1 {
                    self.hint = Some((
                        "Broad Strokes and Fine Fingers (Grimoire) sort by more than category.".into(),
                        2.5,
                    ));
                }
            }
            if input.seek {
                self.press_seek(audio);
            }
            if input.post {
                self.post_or_recall(audio);
            }
            if let Some(slot) = input.cast {
                self.cast(slot, audio);
            }
            self.update_spells(dt);
            if input.cycle != 0 && !self.held.is_empty() {
                let n = self.held.len() as i32;
                self.active = ((self.active as i32 + input.cycle).rem_euclid(n)) as usize;
                self.manual_pick = self.active_book();
                audio.play(Sfx::Tick, 0.25);
            }
            if input.primary_pressed {
                self.press(audio);
            }
            self.update_scoop(dt, input, audio);
            self.update_toss(dt, input, audio);
        } else {
            self.scoop = None;
            self.toss_charge = 0.0;
            self.toss_was_down = false;
            self.toss_arc.clear();
            self.toss_land = None;
        }
        if self.manual_pick.is_some_and(|b| !self.held.contains(&b)) {
            self.manual_pick = None;
        }
        let tidy = self.level(Upg::Tidy) > 0;
        let sense = self.level(Upg::SortSense) > 0 || self.level(Upg::Wayfinding) > 0;
        if (tidy || sense) && !self.held.is_empty() {
            self.nav.fill(&mut self.walk, self.player.pos);
        }
        if tidy && self.manual_pick.is_none() {
            self.pick_nearest_active();
        }
        self.update_aura(dt);
        // After the aura, which may have filed the active book.
        self.sense = self.active_book().and_then(|b| self.sense_for(b));
        self.update_flights(dt, audio);
        self.update_streams(dt);
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
        self.home_tossed(&landings);
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
        for (f, &open) in self.gate_fade.iter_mut().zip(self.lib.world.gates_open()) {
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
        let world = &self.lib.world;
        let lv = &world.levels[world.level_at(pos)];
        let (lo, hi) = (lv.min + BODY_R + 0.1, lv.max - BODY_R - 0.1);
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
                for (mn, mx, gate) in world.in_way(feet) {
                    let q = Vec2::new(np.x.clamp(mn.x, mx.x), np.z.clamp(mn.z, mx.z));
                    let d = Vec2::new(np.x, np.z) - q;
                    let l = d.length();
                    if l >= BODY_R {
                        continue;
                    }
                    if l <= 1e-5 {
                        // Inside it (only if it moved in): back to where this step began.
                        (np.x, np.z) = (from.x, from.z);
                        continue;
                    }
                    let push = d / l * (BODY_R - l);
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
            let walled = |x: f32, z: f32| world.rises_at(Vec2::new(x, z), feet);
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
            match world.ground(Vec2::new(np.x, np.z), feet + STEP_UP) {
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

    fn compute_aim(&self) -> Aim {
        let o = self.player.eye();
        let d = self.player.forward();
        let block = self.lib.world.ray(o, d, f32::MAX, Stops::SIGHT);
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
            best_t = t;
        }
        // Posted familiars, nearer still.
        if let Some((i, _)) = self.aimed_familiar(o, d, best_t.min(block).min(SHELF_REACH * 2.0)) {
            best = Aim::Familiar(i);
        }
        best
    }

    /// The left button goes down: on a pile it starts a scoop (taken on release), on
    /// a series' run it files.
    fn press(&mut self, audio: &mut Audio) {
        match self.aim {
            Aim::Loose { book, point, .. } => {
                if self.held.len() >= self.capacity() {
                    audio.play(Sfx::Nope, 0.4);
                    self.hint = Some(("Your arms are full — file some books first.".into(), 2.0));
                    return;
                }
                self.scoop = Some(Scoop {
                    book,
                    held: 0.0,
                    at: point,
                    lv: self.loose.level_of(book),
                });
            }
            Aim::Shelf { run: Some(run), .. } => {
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

    /// Kindred Glow as drawn (see `Globals::glow`): Glimpse lights at full.
    fn glow(&self) -> [f32; 4] {
        let lvl = if self.spell_on(Upg::Glimpse) {
            3
        } else {
            self.level(Upg::Glow) as usize
        };
        let [reach, strength, glint] = GLOW_LEVELS[lvl];
        [reach, strength, glint, 0.0]
    }

    /// Which department, category or series (by the sorting group) book `b` is in.
    fn group_of(&self, b: u32) -> usize {
        self.group.of(&self.lib, b)
    }

    /// The groups R cycles through, coarsest first: category always, department with
    /// Broad Strokes, series with Fine Fingers.
    pub fn groups(&self) -> Vec<Group> {
        let mut out = Vec::new();
        if self.level(Upg::Broad) > 0 {
            out.push(Group::Department);
        }
        out.push(Group::Category);
        if self.level(Upg::Fine) > 0 {
            out.push(Group::Series);
        }
        out
    }

    /// A group's colour: a department's is its first category's.
    pub fn group_color(&self, group: Group, key: usize) -> [f32; 3] {
        let cats = &self.lib.catalog.categories;
        match group {
            Group::Department => catalog::category_color(&cats[self.lib.catalog.departments[key].categories.start]),
            Group::Category => catalog::category_color(&cats[key]),
            Group::Series => self.lib.series[key].color,
        }
    }

    /// What is sought: the level and which group of it.
    pub fn sought(&self) -> Option<(Group, usize)> {
        self.seek.map(|b| (self.group, self.group_of(b)))
    }

    /// F (with Sifting Touch): seek the aimed book's group, or stop if it is the one
    /// sought; aimed at no book, stop, or seek the active book's group.
    fn press_seek(&mut self, audio: &mut Audio) {
        if self.level(Upg::Sift) == 0 {
            self.hint = Some((
                "Seeking is learned with Sifting Touch, in the Grimoire (Tab).".into(),
                2.0,
            ));
            return;
        }
        let sought = self.sought().map(|(_, key)| key);
        self.seek = match self.aim {
            Aim::Loose { book, .. } if sought != Some(self.group_of(book)) => Some(book),
            Aim::Loose { .. } => None,
            _ if sought.is_some() => None,
            _ => self.active_book(),
        };
        if sought.is_none() && self.seek.is_none() {
            self.hint = Some(("Aim at a book to seek its kind.".into(), 2.0));
            return;
        }
        audio.play(Sfx::Tick, 0.3);
    }

    /// The largest a scoop's ring grows: Sifting Touch's reach while seeking, else
    /// wider the more the arms have room for.
    fn scoop_max(&self) -> f32 {
        let room = self.capacity().saturating_sub(self.held.len());
        let plain = 0.35 + 0.05 * (room as f32).sqrt();
        match self.sought() {
            Some(_) => SIFT_RADIUS[self.level(Upg::Sift) as usize].max(plain),
            None => plain,
        }
    }

    /// The ring of a scoop held past a tap.
    pub fn scoop_radius(&self, s: &Scoop) -> Option<f32> {
        let t = s.held - TAP;
        (t >= 0.0).then(|| self.scoop_max() * (0.25 + 0.75 * (t / SCOOP_GROW).min(1.0)))
    }

    /// Grow a held scoop, its ring following the aim across piles; on release take
    /// what it holds.
    fn update_scoop(&mut self, dt: f32, input: &Input, audio: &mut Audio) {
        let Some(mut s) = self.scoop else { return };
        if !input.primary_down {
            self.scoop = None;
            self.take_scoop(&s, audio);
            return;
        }
        s.held += dt;
        if let Aim::Loose { book, point, .. } = self.aim {
            s.at = point;
            s.lv = self.loose.level_of(book);
        }
        self.scoop = Some(s);
    }

    /// A tap takes the book pressed on; a hold, the top of the pile inside the ring,
    /// or while seeking only the sought group's books there, from any depth.
    fn take_scoop(&mut self, s: &Scoop, audio: &mut Audio) {
        let books = match self.scoop_radius(s) {
            None if self.loose.is_resting(s.book) => {
                self.loose.take(&[s.book]);
                vec![s.book]
            }
            None => return,
            Some(r) => {
                let room = self.capacity().saturating_sub(self.held.len());
                let (lib, sought) = (&self.lib, self.sought());
                let keep = |b: u32| sought.is_none_or(|(group, key)| group.of(lib, b) == key);
                self.loose.gather(s.lv, Vec2::new(s.at.x, s.at.z), r, room, keep)
            }
        };
        self.take_into_arms(&books, audio);
    }

    /// Where a toss at the current charge leaves the hand, and how fast.
    fn toss_launch(&self) -> (Vec3, Vec3) {
        let (mut hand, _) = self.hand_xform(0);
        // Flush against something books can't pass (furniture, a closed gate, a floor
        // or stair) the hand is inside it; leave from just short of it.
        let eye = self.player.eye();
        let reach = hand - eye;
        let dir = reach.normalize();
        let hit = self.lib.world.ray(eye, dir, f32::MAX, Stops::BOOKS);
        if hit < reach.length() {
            hand = eye + dir * (hit - 0.08).max(0.0);
        }
        // A little above the crosshair, so a level throw carries.
        let dir = (self.player.forward() + Vec3::Y * 0.18).normalize();
        let top = LONG_TOSS[self.level(Upg::LongToss) as usize].0;
        let speed = TOSS_MIN + (top - TOSS_MIN) * self.toss_charge;
        (hand, dir * speed)
    }

    /// Charge while a toss button is held (showing the arc); throw on release.
    fn update_toss(&mut self, dt: f32, input: &Input, audio: &mut Audio) {
        let armful = input.armful_down || input.armful_pressed;
        let down = input.toss_down || input.toss_pressed || armful;
        if down && !self.toss_was_down {
            self.toss_armful = armful;
        }
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
            self.toss(self.toss_armful, audio);
        }
        if !down {
            self.toss_charge = 0.0;
        }
    }

    /// Throw every held book of the group sought, or if none is held the active
    /// book's group, as a loose stream; with `armful`, everything held.
    fn toss(&mut self, armful: bool, audio: &mut Audio) {
        let Some(active) = self.active_book() else { return };
        let group: Vec<u32> = if armful {
            self.held.clone()
        } else {
            let key = match self.sought() {
                Some((_, key)) if self.held.iter().any(|&b| self.group_of(b) == key) => key,
                _ => self.group_of(active),
            };
            self.held.iter().copied().filter(|&b| self.group_of(b) == key).collect()
        };
        let prev = self.active_book();
        self.held.retain(|b| !group.contains(b));
        self.follow_active(prev);
        let (from, vel) = self.toss_launch();
        let speed = vel.length();
        let gap = (0.5 / group.len() as f32).min(0.04);
        let spread = LONG_TOSS[self.level(Upg::LongToss) as usize].1;
        for (i, &b) in group.iter().enumerate() {
            self.flights.retain(|f| f.book != b);
            self.state[b as usize] = BookState::Loose;
            self.tossed[b as usize] = true;
            let mut j = || self.rng.gen_range(-1.0..1.0f32);
            let scatter = Vec3::new(j(), j(), j()) * (0.06 * speed + 0.3) * spread;
            let spin = Vec3::new(j(), j(), j()) * 9.0;
            let rot = self.hand_xform(0).1;
            self.loose.throw(b, from, vel + scatter, rot, spin, i as f32 * gap);
        }
        audio.play_pitched(Sfx::Whoosh, 0.45, 2);
        self.toss_charge = 0.0;
        self.dirty_save = true;
    }

    /// Homing Toss: a tossed book that landed in front of its own shelf, near its
    /// slot and in sight of it, slips into it.
    pub(super) fn home_tossed(&mut self, landings: &[crate::loose::Landing]) {
        let reach = HOMING[self.level(Upg::Homing) as usize];
        for l in landings {
            let b = l.book as usize;
            if !std::mem::take(&mut self.tossed[b]) || reach == 0.0 {
                continue;
            }
            let info = &self.lib.books[b];
            let rel = l.at - info.slot_pos;
            let world = &self.lib.world;
            let home = world.level_at(l.at) == world.level_at(info.slot_pos)
                && Vec2::new(rel.x, rel.z).length() <= reach
                && self.lib.reaches_front(l.book, l.at + Vec3::Y * 0.1);
            if home && self.loose.is_resting(l.book) {
                self.loose.take(&[l.book]);
                self.state[b] = BookState::Filing;
                self.start_flight(l.book, Dest::Shelf, 0.45, 0.0);
            }
        }
    }

    /// Turn to `target` and set the toss's pitch and charge so it lands as near it as
    /// a throw from here can; returns how far off that is. For the pacing bot and
    /// tests.
    #[cfg(test)]
    pub(super) fn aim_toss(&mut self, target: Vec3) -> f32 {
        let flat = Vec2::new(target.x - self.player.pos.x, target.z - self.player.pos.z);
        self.player.yaw = flat.x.atan2(-flat.y);
        let mut best = (f32::INFINITY, 0.0, 0.0);
        let mut arc = Vec::new();
        for pi in 0..16 {
            for ci in 1..=5 {
                self.player.pitch = -0.6 + pi as f32 * 0.09;
                self.toss_charge = ci as f32 / 5.0;
                let (from, vel) = self.toss_launch();
                arc.clear();
                if let Some(land) = self.loose.predict(from, vel, &mut arc) {
                    let off = Vec2::new(land.x - target.x, land.z - target.z).length() + (land.y - target.y).abs();
                    if off < best.0 {
                        best = (off, self.player.pitch, self.toss_charge);
                    }
                }
            }
        }
        (self.player.pitch, self.toss_charge) = (best.1, best.2);
        best.0
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
            self.pos[b] = match f.dest {
                Dest::Hand => f.from_pos.lerp(to_pos, e) + Vec3::Y * 0.35 * (PI * e).sin(),
                // A cubic curve with both inner points at the front of the slot: the
                // book lines up there, then goes in straight.
                Dest::Shelf => {
                    let front = self.lib.slot_front(f.book);
                    let (a, c) = (f.from_pos.lerp(front, e), front.lerp(to_pos, e));
                    a.lerp(front, e).lerp(front.lerp(c, e), e) + Vec3::Y * 0.25 * (PI * e).sin()
                }
            };
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

    /// Book `b` is home: its state, the counts, and the lumens it earns with any
    /// series or category bonus. Returns whether it completed its series and its
    /// category. No sound or sparkle; `on_shelved` adds those.
    fn credit_shelved(&mut self, b: u32) -> (bool, bool) {
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
        let series_done = self.series_shelved[s] == self.lib.series[s].volumes;
        if series_done {
            let bonus = self.lib.series[s].volumes as u64;
            self.lumens += bonus;
            self.earned += bonus;
        }
        let category_done = self.category_shelved[cat] == self.lib.category_books[cat];
        if category_done {
            self.lumens += 100;
            self.earned += 100;
        }
        (series_done, category_done)
    }

    fn on_shelved(&mut self, b: u32, audio: &mut Audio, thunks: &mut u32) {
        let (series_done, category_done) = self.credit_shelved(b);
        let bi = b as usize;
        let s = self.lib.books[bi].series as usize;
        let cat = self.lib.series[s].category;
        if *thunks < 3 {
            audio.play(Sfx::Thunk, 0.5);
            *thunks += 1;
        }
        let col = self.lib.series[s].color;
        let bright = [col[0] * 3.0 + 0.4, col[1] * 3.0 + 0.35, col[2] * 3.0 + 0.2];
        self.burst(self.lib.books[bi].slot_pos, 5, bright, 0.5);
        let ser = &self.lib.series[s];
        if series_done && ser.volumes > 1 {
            let bonus = ser.volumes;
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
        if category_done {
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

    /// Where to stand to file book `b`: in the aisle in front of its slot, at floor
    /// level; where something stands there (in an alcove's back corner, the case
    /// across the back), the nearest place free to stand.
    fn approach(&self, b: u32) -> Vec3 {
        let info = &self.lib.books[b as usize];
        let face = &self.lib.faces[info.face as usize];
        let p = info.slot_pos + face.normal * 0.75;
        let at = Vec3::new(p.x, face.center.y, p.z);
        if self.nav.free_at(at) {
            at
        } else {
            self.nav.standable_near(at)
        }
    }

    /// Where the compass points for book `b`: its slot while Sort Sense reaches (in
    /// its room at I, anywhere at II), else with Wayfinding the doorway of its room.
    fn sense_for(&self, b: u32) -> Option<Sense> {
        let dept = self.lib.dept_of(self.lib.category_of_book(b));
        let here = self.lib.room_at(self.player.pos);
        let sense = self.level(Upg::SortSense);
        let from = self.player.pos;
        if sense >= 2 || (sense == 1 && here == Some(dept)) {
            let route = self.nav.route(&self.walk, from, self.approach(b))?;
            return Some(Sense { route, room: None });
        }
        if self.level(Upg::Wayfinding) > 0 && here != Some(dept) {
            let route = self.nav.route(&self.walk, from, self.lib.rooms[dept].door)?;
            return Some(Sense {
                route,
                room: Some(dept),
            });
        }
        None
    }

    /// Make the held book nearest its home on foot the active one. A bookcase's far
    /// side is a walk around the row, not the width of the case.
    fn pick_nearest_active(&mut self) {
        if self.held.is_empty() {
            return;
        }
        let from = self.player.pos;
        let cost = |g: &Self, b: u32| g.nav.distance(&g.walk, from, g.approach(b));
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

    /// Shelving Aura, and Shelving Rush while it lasts (wider, and as fast as you pass):
    /// carried books near their slots fly home, from in front of their cases.
    fn update_aura(&mut self, dt: f32) {
        let (mut radius, mut rate) = AURA_LEVELS[self.level(Upg::Aura) as usize];
        if self.spell_on(Upg::Rush) {
            radius = radius.max(spells::rush_reach(self.level(Upg::Rush)));
            rate = spells::RUSH_RATE;
        }
        if rate == 0.0 {
            return;
        }
        self.aura_budget = (self.aura_budget + rate * dt).min(rate * 0.5 + 1.0);
        let eye = self.player.eye();
        let prev = self.active_book();
        let mut i = 0;
        while i < self.held.len() && self.aura_budget >= 1.0 {
            let b = self.held[i];
            let arrived = !self.flights.iter().any(|f| f.book == b);
            if arrived && self.lib.books[b as usize].slot_pos.distance(eye) < radius && self.lib.reaches_front(b, eye) {
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
        let floor = self.lib.world.ground(Vec2::new(top.x, top.z), top.y).unwrap_or(0.0);
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
        let floor = self
            .lib
            .world
            .ground(Vec2::new(at.x, at.z), target.y + 0.5)
            .unwrap_or(0.0);
        self.look_at(Vec3::new(at.x, floor, at.z), target);
    }

    /// Instantly shelve a fraction of a category (debug/screenshot aid; no rewards).
    pub fn script_fill_category(&mut self, cat: usize, frac: f32) {
        let ids: Vec<u32> = (0..self.lib.books.len() as u32)
            .filter(|&b| self.lib.category_of_book(b) == cat)
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
            .filter(|&b| self.loose.is_resting(b) && self.lib.category_of_book(b) == cat)
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
            let floor = self
                .lib
                .world
                .ground(Vec2::new(p.x + 1.2, p.z + 1.2), p.y)
                .unwrap_or(0.0);
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
            push(&mut list, f.pos, 3.5, f.role.light(), 0.8);
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
            lens: self.lens_sought(),
            glow: self.glow(),
            lights,
        }
    }

    /// What the Seeker's Lens lights (see `Globals::lens`): the group sought.
    fn lens_sought(&self) -> [f32; 4] {
        match self.sought() {
            Some((group, key)) if self.level(Upg::Lens) > 0 => {
                let mode = match group {
                    Group::Department => 1.0,
                    Group::Category => 2.0,
                    Group::Series => 3.0,
                };
                [mode, key as f32, 0.0, 0.0]
            }
            _ => [0.0; 4],
        }
    }

    pub fn view_proj(&self, aspect: f32) -> Mat4 {
        let view = Mat4::look_to_rh(self.player.eye(), self.player.forward(), Vec3::Y);
        let proj = Mat4::perspective_rh(vfov(aspect, self.fov), aspect, 0.03, 140.0);
        proj * view
    }

    /// A book's instance; `active` marks the active book in hand. Loose books carry
    /// their department, category and series for the Seeker's Lens (see the shader's
    /// book flags).
    fn book_inst(&self, b: usize, pos: Vec3, rot: Quat, active: bool) -> Inst {
        let info = &self.lib.books[b];
        let s = &self.lib.series[info.series as usize];
        let loose = self.state[b] == BookState::Loose;
        let flags =
            active as u32 | (loose as u32) << 1 | (self.lib.dept_of(s.category) as u32) << 2 | (s.category as u32) << 7;
        let param = s.pattern | (s.trim << 4) | (info.volume << 8) | (flags << 16);
        Inst::new(
            pos,
            rot,
            info.size,
            kind::BOOK,
            [s.color[0], s.color[1], s.color[2], info.series as f32],
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
                BookState::Filing | BookState::Carried(_) | BookState::Streaming => true,
                BookState::Shelved => false,
            };
            if moving {
                dyn_insts.push(self.book_inst(b, self.pos[b], self.rot[b], false));
            }
        }
        // Ghosts: with Kindred Glow (or Glimpse) the series you carry glow, the one
        // you aim at brightens, and with Sort Sense the active book's slot pulses.
        let mut lit = std::collections::HashMap::new();
        if self.level(Upg::Glow) > 0 || self.spell_on(Upg::Glimpse) {
            for &b in &self.held {
                lit.insert(self.lib.books[b as usize].series, (1, false));
            }
        }
        if let Aim::Shelf { run: Some(r), .. } = self.aim {
            lit.entry(r.series).or_insert((0, false)).1 = true;
        }
        scene.highlight(lit);
        let active = self
            .active_book()
            .filter(|_| self.sense.is_some_and(|s| s.room.is_none()));
        scene.set_active(active.map(|b| (b, self.lib.books[b as usize].slot_pos)));
        if scene_gates_changed(&mut self.scene_gates, self.lib.world.gates_open()) {
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
                held_insts.push(self.book_inst(b as usize, p, q, i == self.active));
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

        self.draw_familiars(&mut dyn_insts);

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

        // Echo: a ring sweeping out across the floor from where it rang.
        if let Some(t) = self.echo_age()
            && let Some((group, key)) = self.echo_kind
        {
            let r = t * ECHO_SPEED;
            let col = signal_color(self.group_color(group, key));
            let n = (r * 2.0).clamp(24.0, 160.0) as usize;
            let fade = (1.0 - t / 2.0).max(0.0);
            for k in 0..n {
                let a = k as f32 / n as f32 * std::f32::consts::TAU;
                let p = self.echo_from + Vec3::new(a.cos() * r, 0.05, a.sin() * r);
                dyn_insts.push(Inst::new(
                    p,
                    Quat::from_rotation_y(-a),
                    Vec3::new(0.04, 0.03, 0.35),
                    kind::EMISSIVE,
                    [col[0], col[1], col[2], 3.0 * fade],
                    0,
                ));
            }
        }

        // A held scoop: a ring on the pile, in the sought group's colour.
        if let Some(s) = self.scoop
            && let Some(r) = self.scoop_radius(&s)
        {
            let col = match self.sought() {
                Some((group, key)) => signal_color(self.group_color(group, key)),
                None => [1.0, 0.8, 0.5],
            };
            let n = (r * 30.0).clamp(16.0, 96.0) as usize;
            for k in 0..n {
                let a = k as f32 / n as f32 * std::f32::consts::TAU + self.time * 0.6;
                let xz = Vec2::new(s.at.x + a.cos() * r, s.at.z + a.sin() * r);
                // On the pile's surface, or level with the aim where that falls away.
                let h = self.loose.height_at(s.lv, xz);
                let y = if (h - s.at.y).abs() < 1.0 { h } else { s.at.y };
                dyn_insts.push(Inst::new(
                    Vec3::new(xz.x, y + 0.07, xz.y),
                    Quat::from_rotation_y(-a),
                    Vec3::new(0.012, 0.012, 0.07),
                    kind::EMISSIVE,
                    [col[0], col[1], col[2], 2.2],
                    0,
                ));
            }
        }

        let eye = self.player.eye();
        for (i, &(base, ph)) in self.motes.iter().enumerate() {
            let t = self.time * 0.05 + ph;
            let mut p = base + Vec3::new((t * 1.3).sin() * 0.8, (t * 0.7).sin() * 0.6, (t * 1.1).cos() * 0.8);
            // Motes stay put in the air; those left behind wrap round into a box about
            // the player, 24 m across and from just under their feet to 4.2 m over
            // their eyes, fading in at its top.
            let rel = p - eye;
            p.x = eye.x + (rel.x + 12.0).rem_euclid(24.0) - 12.0;
            p.z = eye.z + (rel.z + 12.0).rem_euclid(24.0) - 12.0;
            p.y = eye.y - 1.8 + (rel.y + 1.8).rem_euclid(6.0);
            let top = (eye.y + 4.2 - p.y).min(1.0);
            let tw = 0.5 + 0.5 * (self.time * 0.8 + i as f32).sin();
            dyn_insts.push(Inst::new(
                p,
                Quat::IDENTITY,
                Vec3::splat(0.006),
                kind::EMISSIVE,
                [1.0, 0.85, 0.6, (0.6 + 1.2 * tw) * top],
                0,
            ));
        }

        // Moonbeams and veils.
        trans.extend_from_slice(&self.lib.static_trans);
        for (gate, &fade) in self.lib.world.gates().iter().zip(&self.gate_fade) {
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
            opaque.push(self.book_inst(b as usize, p, q, false));
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
                BookState::Filing => pending.push(b as u32),
                BookState::Loose | BookState::Held | BookState::Carried(_) | BookState::Streaming => {}
            }
        }
        let p = &self.player;
        let data = SaveData {
            version: SAVE_VERSION,
            generation: generation(self.layout),
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
            // Books a familiar carries, or a charm, fall from where they are.
            airborne: (self.loose.airborne())
                .chain(
                    (0..self.state.len() as u32).filter_map(|b| match self.state[b as usize] {
                        BookState::Carried(_) | BookState::Streaming => Some((b, self.pos[b as usize])),
                        _ => None,
                    }),
                )
                .map(|(b, p)| (b, p.to_array()))
                .collect(),
            familiars: (self.familiars.iter())
                .map(|f| (f.role.name().to_string(), f.post.as_ref().map(|p| p.at.to_array())))
                .collect(),
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
        if data.generation != generation(self.layout) {
            // Kept aside rather than overwritten by the fresh game's first save.
            let old = self.save_path().with_extension("old.json");
            eprintln!(
                "hushfold: the save is from another version of this library; starting fresh (kept as {})",
                old.display()
            );
            let _ = std::fs::rename(self.save_path(), old);
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
        for g in 0..self.lib.world.gates().len() {
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
        if data.version == 0 {
            // Sort Sense found the slot from anywhere before it had levels.
            if self.level(Upg::SortSense) == 1 {
                self.upg[upg_index(Upg::SortSense)] = 2;
            }
            // Paper Familiars filed books from anywhere; they come back as shelvers.
            if let Some(&n) = data.levels.get("Familiar") {
                self.upg[upg_index(Upg::Shelver)] = n;
            }
        }
        if data.version < 2 {
            // Guide signs and the glow of carried series were free before they were
            // skills; a game begun then keeps them.
            self.upg[upg_index(Upg::Signposts)] = 1;
            self.upg[upg_index(Upg::Glow)] = 3;
        }
        for (i, &u) in UPGRADES.iter().enumerate() {
            self.upg[i] = self.upg[i].min(upg_costs(self.layout, u).len() as u32);
        }
        self.lumens = data.lumens;
        self.earned = data.earned;
        self.play_time = data.play_time;
        let [x, y, z, yaw, pitch] = data.player;
        self.player.pos = Vec3::new(x, y, z);
        self.player.yaw = yaw;
        self.player.pitch = pitch;
        for role in [Role::Shelver, Role::Courier] {
            let upg = if role == Role::Shelver {
                Upg::Shelver
            } else {
                Upg::Courier
            };
            for _ in 0..self.level(upg) {
                self.spawn_familiar(role);
            }
        }
        for (name, at) in &data.familiars {
            let Some(at) = at.map(Vec3::from_array) else { continue };
            let free = (0..self.familiars.len())
                .find(|&i| self.familiars[i].role.name() == name && self.familiars[i].post.is_none());
            if let Some(i) = free
                && self.nav.open_at(at)
            {
                self.post_familiar(i, at);
                self.familiars[i].perch();
            }
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
            assert_eq!(east, lib.dept_of(g.lib.category_of_book(b)) == 1, "book {b}");
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

    /// Every slot's approach (the aisle in front of it) is a walk away from the start,
    /// with every gate open.
    #[test]
    fn every_slot_can_be_walked_to() {
        for layout in Layout::ALL {
            let mut g = Game::new(layout);
            g.open_all_gates();
            let from = g.player.pos;
            g.nav.fill(&mut g.walk, from);
            let stuck: Vec<u32> = (0..g.lib.books.len() as u32)
                .filter(|&b| !g.nav.distance(&g.walk, from, g.approach(b)).is_finite())
                .collect();
            assert!(
                stuck.is_empty(),
                "{layout:?}: {} slots unreachable, e.g. book {} approach {}",
                stuck.len(),
                stuck[0],
                g.approach(stuck[0])
            );
            // And from where one stands to file each case, the way back is open: a
            // spot inside furniture next to an aisle counts as reached above, but
            // walking out from it finds nothing (the pacing bot was stranded so).
            let mut back = g.nav.field();
            let mut seen = std::collections::HashSet::new();
            for b in 0..g.lib.books.len() as u32 {
                if !seen.insert(g.lib.books[b as usize].face) {
                    continue;
                }
                let at = g.approach(b);
                g.nav.fill(&mut back, at);
                assert!(
                    g.nav.distance(&back, at, from).is_finite(),
                    "{layout:?}: stranded standing to file book {b}, at {at}"
                );
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
        assert!(g.sense.is_some_and(|s| s.room.is_none() && s.route.direct));
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
        let r = g.sense.unwrap().route;
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
                g.press(&mut audio);
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
            if let Some(gate) = g.lib.world.gates().iter().position(|gate| gate.opens == d) {
                g.open_gate(gate);
            }
            let stranded = (0..n)
                .filter(|&b| g.lib.dept_of(g.lib.category_of_book(b)) == d && !g.nav.open_at(g.loose.pose(b).0))
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
        assert_eq!(g.lib.world.level_at(p), 1);
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

    /// Books dropped on the stairs come to rest on the treads, not sunk into them.
    #[test]
    fn books_rest_on_the_treads() {
        let mut g = Game::new(Layout::Great);
        let mut audio = crate::audio::Audio::silent();
        let (lv, top) = g.loose.best_pile(|l, c, _| (l == 0).then(|| c.length())).unwrap();
        let books = g.loose.gather(lv, Vec2::new(top.x, top.z), 1.5, 40, |_| true);
        assert_eq!(books.len(), 40);
        let flights = g.lib.world.stairs().to_vec();
        for (i, &b) in books.iter().enumerate() {
            let r = &flights[i % flights.len()];
            let (c, half) = ((r.min + r.max) * 0.5, (r.max - r.min) * 0.5);
            let t = (i / flights.len()) as f32 / 10.0 * 1.6 - 0.8;
            let p = c + r.axis * half.dot(r.axis.abs()) * t;
            let from = Vec3::new(p.x, r.height(p) + 1.0, p.y);
            let rot = Quat::from_rotation_y(i as f32 * 0.7);
            g.loose.throw(b, from, Vec3::ZERO, rot, Vec3::ZERO, 0.0);
        }
        for _ in 0..60 * 6 {
            g.update(1.0 / 60.0, &Input::default(), &mut audio, false);
        }
        let solid: Vec<(Vec3, Vec3)> = (g.lib.static_insts.iter())
            .filter(|i| [kind::STONE, kind::WOOD].contains(&i.kind))
            .filter_map(|i| i.axis_box())
            .collect();
        let mut sunk = Vec::new();
        for &b in &books {
            assert!(g.loose.is_resting(b), "book {b} still moving");
            let (pos, rot) = g.loose.pose(b);
            let half = g.lib.books[b as usize].size * 0.5 - 0.01;
            for k in 0..8 {
                let s = Vec3::new([-1.0, 1.0][k & 1], [-1.0, 1.0][(k >> 1) & 1], [-1.0, 1.0][k >> 2]);
                let q = pos + rot * (half * s);
                // How far inside a step it is: the shortest way out, so a corner just
                // past a riser's face counts as the millimetres it is, not as its depth
                // below the step's top.
                let depth = (solid.iter())
                    .filter(|(mn, mx)| q.cmpgt(*mn).all() && q.cmplt(*mx).all())
                    .map(|(mn, mx)| (q - *mn).min(*mx - q).min_element())
                    .fold(0.0, f32::max);
                if depth > 0.005 {
                    sunk.push(format!("book {b} corner {q} {depth:.3} deep"));
                }
            }
        }
        assert!(
            sunk.is_empty(),
            "{} corners sunk: {:#?}",
            sunk.len(),
            &sunk[..sunk.len().min(8)]
        );
    }

    /// The way to a slot upstairs goes by the stairs.
    #[test]
    fn routes_climb_the_stairs() {
        let mut g = Game::new(Layout::Great);
        g.open_all_gates();
        let from = g.player.pos;
        g.nav.fill(&mut g.walk, from);
        for dept in [4, 8] {
            let b = (0..g.lib.books.len() as u32)
                .find(|&b| g.lib.dept_of(g.lib.category_of_book(b)) == dept)
                .unwrap();
            let to = g.approach(b);
            let d = g.nav.distance(&g.walk, from, to);
            let r = g.nav.route(&g.walk, from, to).expect("a route");
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
            // A full scoop.
            g.press(&mut audio);
            if let Some(mut s) = g.scoop.take() {
                s.held = TAP + SCOOP_GROW;
                g.take_scoop(&s, &mut audio);
            }
        }
        check(&mut g);
        assert!(g.held.len() > 10, "gathered {}", g.held.len());
        // Toss some, file one, and check mid-flight and after everything has landed.
        g.toss_charge = 0.6;
        g.toss(false, &mut audio);
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

    /// The flights on the first floor are overhead on the ground floor: walking under
    /// them to the north and south rooms is clear (regression: an invisible wall at
    /// the gallery edge, under the stairs above).
    #[test]
    fn walking_under_upper_stairs() {
        let mut g = Game::new(Layout::Great);
        g.open_all_gates();
        let mut audio = crate::audio::Audio::silent();
        let walk = Input {
            forward: 1.0,
            ..Input::default()
        };
        for (start, yaw) in [(Vec3::new(0.0, 0.0, -6.0), 0.0), (Vec3::new(0.5, 0.0, 6.0), PI)] {
            g.player.pos = start;
            g.player.yaw = yaw;
            for _ in 0..60 * 4 {
                g.update(1.0 / 60.0, &walk, &mut audio, true);
            }
            let p = g.player.pos;
            assert!(p.z.abs() > 15.0 && p.y == 0.0, "from {start}: stopped at {p}");
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
            .filter(|&b| g.lib.dept_of(g.lib.category_of_book(b)) == 0 && g.loose.is_resting(b))
            .take(g.gate_books_left(0) as usize)
            .collect();
        g.loose.take(&first);
        for &b in &first {
            g.on_shelved(b, &mut audio, &mut thunks);
        }
        assert!(g.lib.world.gates_open()[0] && !g.lib.world.gates_open()[1]);
        g.spawn_familiar(Role::Courier);
        g.spawn_familiar(Role::Shelver);
        g.upg[upg_index(Upg::Courier)] = 1;
        g.upg[upg_index(Upg::Shelver)] = 1;
        let post = g.nav.standable_near(Vec3::new(4.0, 0.0, 6.0));
        g.post_familiar(1, post);
        g.save();
        let mut l = Game::new(Layout::Great);
        assert!(l.load());
        let roles: Vec<(Role, bool)> = l.familiars().collect();
        assert_eq!(roles, vec![(Role::Shelver, true), (Role::Courier, false)]);
        assert_eq!(l.familiars[0].post.as_ref().map(|p| p.at), Some(post));
        assert_eq!(l.total_shelved(), g.total_shelved());
        assert_eq!(l.lib.world.gates_open(), g.lib.world.gates_open());
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
        // A save from before phase 3: Paper Familiars come back as shelvers (riding),
        // and Sort Sense, which then reached anywhere, as Sort Sense II.
        let json = format!(
            r#"{{"seed":{SEED},"lumens":5,"earned":900,"levels":{{"Familiar":3,"SortSense":1,"Arms":2}},
                "shelved":[],"held":[],"player":[0.0,0.0,6.5,0.0,0.0],"play_time":60.0}}"#
        );
        std::fs::write(hall.save_path(), json).unwrap();
        let mut old = Game::new(Layout::Hall);
        assert!(old.load());
        assert_eq!(old.level(Upg::Shelver), 3);
        assert_eq!(old.level(Upg::SortSense), 2);
        assert_eq!(old.level(Upg::Arms), 2);
        let riding = old.familiars().filter(|&(r, posted)| r == Role::Shelver && !posted);
        assert_eq!(riding.count(), 3);
        // Saves before reading was earned keep the guide signs and the full glow.
        assert_eq!((old.level(Upg::Signposts), old.level(Upg::Glow)), (1, 3));
        assert_eq!((hall.level(Upg::Signposts), hall.level(Upg::Glow)), (0, 0));
        // A Great Library save from another generation of it starts fresh, kept aside.
        let great = Game::new(Layout::Great);
        let json = format!(
            r#"{{"seed":{SEED},"lumens":5,"earned":900,"shelved":[1,2,3],"held":[],"player":[0.0,0.0,6.5,0.0,0.0],"play_time":60.0}}"#
        );
        std::fs::write(great.save_path(), json).unwrap();
        let mut other = Game::new(Layout::Great);
        assert!(!other.load());
        assert!(!great.save_path().exists() && great.save_path().with_extension("old.json").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// However a Great Library department lays out its room, it shelves as many
    /// books as the others (40 faces) and leaves the floor room enough that its
    /// books lie in heaps, not towers.
    #[test]
    fn great_library_rooms_keep_their_shape() {
        let g = Game::new(Layout::Great);
        let n = g.lib.books.len();
        assert!((110_000..126_000).contains(&n), "{n} books");
        for (d, r) in g.lib.rooms.iter().enumerate() {
            let faces = g.lib.faces.iter().filter(|f| g.lib.dept_of(f.category) == d).count();
            assert_eq!(faces, 40, "room {d}");
            let floor = g.lib.world.levels[r.level].y;
            let top = (0..n as u32)
                .filter(|&b| g.loose.is_resting(b))
                .map(|b| g.loose.rest(b).c)
                .filter(|&c| g.lib.room_at(c) == Some(d))
                .map(|c| c.y - floor)
                .fold(0.0f32, f32::max);
            assert!(top < 2.0, "room {d}: books heaped {top:.1} m high");
        }
    }

    /// Every department has a room holding all its slots, entered by a doorway that is
    /// a walk from the start once the gates are open.
    #[test]
    fn rooms_hold_their_departments() {
        for layout in Layout::ALL {
            let mut g = Game::new(layout);
            g.open_all_gates();
            let from = g.player.pos;
            g.nav.fill(&mut g.walk, from);
            assert_eq!(g.lib.rooms.len(), g.lib.catalog.departments.len());
            for (d, r) in g.lib.rooms.iter().enumerate() {
                for (what, p) in [("door", r.door), ("drop", r.drop)] {
                    assert_eq!(g.lib.room_at(p), Some(d), "{layout:?}: room {d}'s {what}");
                    let walk = g.nav.distance(&g.walk, from, p);
                    assert!(walk.is_finite(), "{layout:?}: room {d}'s {what}");
                }
                // Couriers' piles lie clear of the cases.
                let clear = g.nav.standable_near(r.drop);
                assert!(clear.distance(r.drop) < 0.4, "{layout:?}: room {d}'s drop {}", r.drop);
            }
            for b in 0..g.lib.books.len() as u32 {
                let d = g.lib.dept_of(g.lib.category_of_book(b));
                assert_eq!(g.lib.room_at(g.approach(b)), Some(d), "{layout:?}: book {b}");
            }
        }
    }

    /// Sort Sense I finds the slot only in the book's room, Wayfinding points to the
    /// room from elsewhere, and Sort Sense II finds the slot from anywhere.
    #[test]
    fn sort_sense_is_scoped_to_the_room() {
        let mut g = Game::new(Layout::Great);
        let mut audio = crate::audio::Audio::silent();
        let b = (0..g.lib.books.len() as u32)
            .find(|&b| g.lib.dept_of(g.lib.category_of_book(b)) == 0)
            .unwrap();
        g.state[b as usize] = BookState::Held;
        g.held.push(b);
        let atrium = g.player.pos;
        assert_eq!(g.lib.room_at(atrium), None);
        let mut step = |g: &mut Game| g.update(1.0 / 60.0, &Input::default(), &mut audio, true);
        g.upg[upg_index(Upg::SortSense)] = 1;
        step(&mut g);
        assert!(g.sense.is_none(), "{:?}", g.sense);
        g.upg[upg_index(Upg::Wayfinding)] = 1;
        step(&mut g);
        let s = g.sense.expect("wayfinding");
        assert_eq!(s.room, Some(0));
        assert!(s.route.dist.is_finite() && s.route.dist > 3.0, "{s:?}");
        g.player.pos = g.lib.rooms[0].door;
        step(&mut g);
        assert!(g.sense.is_some_and(|s| s.room.is_none()), "{:?}", g.sense);
        g.player.pos = atrium;
        g.upg[upg_index(Upg::SortSense)] = 2;
        step(&mut g);
        assert!(g.sense.is_some_and(|s| s.room.is_none()), "{:?}", g.sense);
    }

    /// Toss throws the active book's group, or the group sought; X throws the whole
    /// armful; R cycles the groups learned; F seeks, the lens lights what is sought,
    /// and a held scoop takes only it.
    #[test]
    fn toss_and_seek_take_the_group() {
        let mut g = Game::new(Layout::Great);
        let mut audio = crate::audio::Audio::silent();
        let dept = |g: &Game, b: u32| g.lib.dept_of(g.lib.category_of_book(b));
        let cat = |g: &Game, b: u32| g.lib.category_of_book(b);
        let n = g.lib.books.len() as u32;
        let a = 0;
        let same_dept = (0..n)
            .find(|&b| dept(&g, b) == dept(&g, a) && cat(&g, b) != cat(&g, a))
            .unwrap();
        let other = (0..n).find(|&b| dept(&g, b) != dept(&g, a)).unwrap();
        let hold = |g: &mut Game| {
            for b in [a, same_dept, other] {
                g.loose.take(&[b]);
                g.state[b as usize] = BookState::Held;
                g.held.push(b);
            }
            g.active = 0;
        };
        hold(&mut g);
        g.toss_charge = 0.5;
        g.toss(false, &mut audio);
        assert_eq!(g.held, vec![same_dept, other]);
        // A tap of X (pressed one frame, up the next) throws everything.
        let x = Input {
            armful_pressed: true,
            ..Input::default()
        };
        for input in [x, Input::default()] {
            g.update(1.0 / 60.0, &input, &mut audio, true);
        }
        assert!(g.held.is_empty());
        // Learn Broad Strokes; R moves from category to department.
        g.upg[upg_index(Upg::Broad)] = 1;
        let r = Input {
            regroup: true,
            ..Input::default()
        };
        g.update(1.0 / 60.0, &r, &mut audio, true);
        assert_eq!(g.group, Group::Department);
        hold(&mut g);
        g.toss(false, &mut audio);
        assert_eq!(g.held, vec![other]);
        // F at nothing seeks the active book's department, and the lens lights it.
        g.upg[upg_index(Upg::Sift)] = 4;
        g.upg[upg_index(Upg::Lens)] = 1;
        g.upg[upg_index(Upg::Arms)] = 4;
        g.aim = Aim::None;
        g.press_seek(&mut audio);
        assert_eq!(g.sought(), Some((Group::Department, dept(&g, other))));
        assert_eq!(g.lens_sought(), [1.0, dept(&g, other) as f32, 0.0, 0.0]);
        // Holding one of the group sought, toss throws it rather than the active book's.
        g.held.clear();
        hold(&mut g);
        g.toss(false, &mut audio);
        assert_eq!(g.held, vec![a, same_dept]);
        // F at a book of another department seeks that one instead.
        g.held.clear();
        let aimed = (0..n)
            .find(|&b| g.loose.is_resting(b) && g.loose.level_of(b) == 0 && dept(&g, b) != dept(&g, other))
            .unwrap();
        let point = g.loose.pose(aimed).0;
        g.aim = Aim::Loose {
            book: aimed,
            point,
            dist: 2.0,
        };
        g.press_seek(&mut audio);
        assert_eq!(g.sought(), Some((Group::Department, dept(&g, aimed))));
        // A full scoop takes only that department, across its categories.
        let scoop = Scoop {
            book: aimed,
            held: TAP + SCOOP_GROW,
            at: point,
            lv: 0,
        };
        g.take_scoop(&scoop, &mut audio);
        assert!(g.held.len() > 1);
        assert!(g.held.iter().all(|&b| dept(&g, b) == dept(&g, aimed)));
        assert!(
            g.held.iter().any(|&b| cat(&g, b) != cat(&g, aimed)),
            "{} books",
            g.held.len()
        );
        // F at a book of the group sought stops seeking.
        g.press_seek(&mut audio);
        assert_eq!(g.sought(), None);
        assert_eq!(g.lens_sought(), [0.0; 4]);
    }

    /// A tap takes just the book pressed on, even from under others; a hold takes
    /// the top of the pile inside its ring, up to what the arms hold.
    #[test]
    fn a_tap_takes_one_book_and_a_hold_scoops() {
        let mut g = Game::new(Layout::Hall);
        let mut audio = crate::audio::Audio::silent();
        let dt = 1.0 / 60.0;
        let aim_at = |g: &mut Game, b: u32| {
            g.aim = Aim::Loose {
                book: b,
                point: g.loose.pose(b).0,
                dist: 2.0,
            };
        };
        let (_, _, books) = (g.loose.piles()).max_by_key(|(_, _, books)| books.len()).unwrap();
        let books = books.to_vec();
        let y = |g: &Game, b: u32| g.loose.pose(b).0.y;
        let bottom = *books.iter().min_by(|&&a, &&b| y(&g, a).total_cmp(&y(&g, b))).unwrap();
        let top = *books.iter().max_by(|&&a, &&b| y(&g, a).total_cmp(&y(&g, b))).unwrap();
        assert!(y(&g, top) > y(&g, bottom) + 0.1, "a pile with books on books");
        // Pressed and released within a frame: a tap.
        aim_at(&mut g, bottom);
        g.press(&mut audio);
        g.update_scoop(dt, &Input::default(), &mut audio);
        assert_eq!(g.held, vec![bottom]);
        // Held past a tap, the ring grows; released, it takes the top of the pile.
        aim_at(&mut g, top);
        g.press(&mut audio);
        let down = Input {
            primary_down: true,
            ..Input::default()
        };
        let mut t = 0.0;
        while t < TAP + SCOOP_GROW {
            g.update_scoop(dt, &down, &mut audio);
            t += dt;
        }
        assert!(g.scoop.as_ref().and_then(|s| g.scoop_radius(s)).is_some());
        g.update_scoop(dt, &Input::default(), &mut audio);
        assert!(g.held.contains(&top));
        assert_eq!(g.held.len(), g.capacity(), "a full scoop fills six-book arms");
        assert!(g.scoop.is_none());
        assert_eq!(g.loose.heightfield_errors(), 0);
    }

    /// The runs of carried series glow only with Kindred Glow, at its level's reach,
    /// or at full while Glimpse lasts; Glimpse then waits out its recharge.
    #[test]
    fn reading_is_earned() {
        let mut g = Game::new(Layout::Hall);
        let mut audio = crate::audio::Audio::silent();
        let b = (0..g.lib.books.len() as u32).find(|&b| g.loose.is_resting(b)).unwrap();
        g.loose.take(&[b]);
        g.state[b as usize] = BookState::Held;
        g.held.push(b);
        let series = g.lib.books[b as usize].series;
        let tick = |g: &mut Game, cast: Option<usize>, audio: &mut crate::audio::Audio| {
            let input = Input {
                cast,
                ..Input::default()
            };
            g.update(0.5, &input, audio, true);
            g.build_instances();
        };
        tick(&mut g, None, &mut audio);
        assert_eq!(g.scene.lit(series).0, 0, "no glow before it is learned");
        g.upg[upg_index(Upg::Glow)] = 1;
        tick(&mut g, None, &mut audio);
        assert_eq!(g.scene.lit(series).0, 1);
        assert_eq!(g.glow()[0], GLOW_LEVELS[1][0]);
        // Glimpse lights at full while it lasts, and can't be cast again until it
        // has recharged.
        g.upg[upg_index(Upg::Glimpse)] = 1;
        let (lasts, recharge) = spell_times(Upg::Glimpse, 1);
        tick(&mut g, Some(0), &mut audio);
        assert!(g.spell_on(Upg::Glimpse));
        assert_eq!(g.glow()[0], GLOW_LEVELS[3][0]);
        let mut t = 0.5;
        while t < lasts + 0.5 {
            tick(&mut g, None, &mut audio);
            t += 0.5;
        }
        assert!(!g.spell_on(Upg::Glimpse));
        tick(&mut g, Some(0), &mut audio);
        assert!(!g.spell_on(Upg::Glimpse), "still recharging");
        while t < recharge + 0.5 {
            tick(&mut g, None, &mut audio);
            t += 0.5;
        }
        tick(&mut g, Some(0), &mut audio);
        assert!(g.spell_on(Upg::Glimpse));
    }

    /// A series tossed at its own shelf with Homing Toss slips into its slots where it
    /// lands near them; without it, nothing does.
    #[test]
    fn homing_toss_files_what_lands_near() {
        for homing in [0, 2] {
            let mut g = Game::new(Layout::Hall);
            let mut audio = crate::audio::Audio::silent();
            g.upg[upg_index(Upg::Arms)] = 4;
            g.upg[upg_index(Upg::Homing)] = homing;
            // A series with several loose volumes, all in the arms.
            let series = (0..g.lib.series.len() as u32)
                .find(|&s| {
                    (0..g.lib.books.len() as u32)
                        .filter(|&b| g.lib.books[b as usize].series == s && g.loose.is_resting(b))
                        .count()
                        >= 4
                })
                .unwrap();
            let books: Vec<u32> = (0..g.lib.books.len() as u32)
                .filter(|&b| g.lib.books[b as usize].series == series && g.loose.is_resting(b))
                .collect();
            for &b in &books {
                g.loose.take(&[b]);
                g.state[b as usize] = BookState::Held;
                g.held.push(b);
            }
            // Six metres out in front of the run, tossing at the floor below it.
            let slot = g.lib.books[books[0] as usize].slot_pos;
            let normal = g.lib.faces[g.lib.books[books[0] as usize].face as usize].normal;
            let floor = Vec3::new(slot.x, 0.0, slot.z);
            g.player.pos = g.nav.standable_near(floor + normal * 6.0);
            let off = g.aim_toss(floor + normal * 0.6);
            assert!(off < 1.0, "the toss lands {off:.2} m off");
            g.toss(false, &mut audio);
            for _ in 0..240 {
                g.update(1.0 / 60.0, &Input::default(), &mut audio, false);
            }
            let filed = books
                .iter()
                .filter(|&&b| g.state[b as usize] == BookState::Shelved)
                .count();
            if homing == 0 {
                assert_eq!(filed, 0);
            } else {
                assert!(filed * 2 >= books.len(), "{filed} of {} filed", books.len());
            }
        }
    }

    /// A book flying home from along its case lines up in front of its slot and goes
    /// in straight, rather than cutting through the case and the books beside it.
    #[test]
    fn books_go_into_their_slots_from_the_front() {
        let mut g = Game::new(Layout::Hall);
        let mut audio = crate::audio::Audio::silent();
        let b = (0..g.lib.books.len() as u32).find(|&b| g.loose.is_resting(b)).unwrap();
        let info = &g.lib.books[b as usize];
        let face = &g.lib.faces[info.face as usize];
        let (slot, plane, normal, right) = (info.slot_pos, face.center, face.normal, face.right);
        g.loose.take(&[b]);
        g.state[b as usize] = BookState::Filing;
        g.pos[b as usize] = slot + right * 3.0 + normal;
        g.start_flight(b, Dest::Shelf, 0.6, 0.0);
        let mut inside = 0;
        for _ in 0..60 {
            g.update(1.0 / 60.0, &Input::default(), &mut audio, false);
            let p = g.pos[b as usize];
            if (p - plane).dot(normal) < 0.0 {
                inside += 1;
                let off = (p - slot).dot(right).abs();
                assert!(off < 0.05, "{off:.2} m to the side of its slot inside the case");
            }
        }
        assert!(inside > 0);
        assert_eq!(g.state[b as usize], BookState::Shelved);
    }

    /// A book thrown hard at a wall during long frames (a hitch, or the pacing bot's
    /// steps) bounces off it rather than coming out the far side.
    #[test]
    fn hard_throws_stay_inside_at_long_frames() {
        let mut g = Game::new(Layout::Great);
        let mut audio = crate::audio::Audio::silent();
        g.open_all_gates();
        // In a room on the ground floor, thrown level at its outer wall (x = 32) from
        // across one step's travel, so some throw reaches the wall mid-step.
        let books: Vec<u32> = (0..g.lib.books.len() as u32)
            .filter(|&b| g.loose.is_resting(b))
            .take(10)
            .collect();
        g.loose.take(&books);
        for (k, &b) in books.iter().enumerate() {
            let from = Vec3::new(29.0 + k as f32 * 0.04, 1.5, 0.5 + k as f32 * 0.3);
            g.loose
                .throw(b, from, Vec3::new(22.0, 0.0, 0.0), Quat::IDENTITY, Vec3::ZERO, 0.0);
        }
        for _ in 0..200 {
            g.update(0.05, &Input::default(), &mut audio, false);
        }
        for b in books {
            let p = g.loose.pose(b).0;
            assert!(g.loose.is_resting(b) && p.x < 31.8, "book {b} came to rest at {p}");
        }
    }
}
