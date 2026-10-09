//! Procedural catalogue: categories, series and their visual identity.

use rand::prelude::*;
use std::collections::HashSet;

pub struct Category {
    pub name: &'static str,
    /// Hue in degrees.
    pub hue: f32,
    pub hue_spread: f32,
    pub sat: (f32, f32),
    pub val: (f32, f32),
    pub topics: &'static [&'static str],
    pub things: &'static [&'static str],
}

pub struct Series {
    pub category: usize,
    pub title: String,
    pub sort_key: String,
    pub volumes: u32,
    pub color: [f32; 3],
    pub pattern: u32,
    pub trim: u32,
    pub height: f32,
    pub depth: f32,
    /// Thickness of each volume, in metres.
    pub thickness: Vec<f32>,
}

impl Series {
    pub fn width(&self) -> f32 {
        self.thickness.iter().sum()
    }

    /// The two-letter filing key shown on shelf edges ("Al" for "The Almanac…").
    pub fn shelf_key(&self) -> String {
        let mut c = self.sort_key.chars();
        let a = c.next().unwrap_or('?').to_ascii_uppercase();
        let b = c.next().unwrap_or(' ');
        format!("{a}{b}")
    }
}

const PEOPLE: &[&str] = &[
    "Aldous Venn",
    "Brannoc",
    "Calloway",
    "Dame Iselde",
    "Elowen Marsh",
    "Fennick",
    "Grisolde",
    "Hesper Quill",
    "Ignatius Pell",
    "Jory Ashdown",
    "Kestrel",
    "Lisbet Orrin",
    "Magister Thule",
    "Nimue Farrow",
    "Oswin",
    "Perpetua Gale",
    "Quentin Mallow",
    "Rosalind Thane",
    "Sabeline",
    "Tobiah Crane",
    "Ulric",
    "Vesper Lark",
    "Wendeline",
    "Xanthe Moor",
    "Yarrow",
    "Zephyrine",
];

const FORMS: &[&str] = &[
    "Treatise on",
    "Notes on",
    "Compendium of",
    "Field Guide to",
    "Primer of",
    "Annals of",
    "Lexicon of",
    "Meditations on",
    "Almanac of",
    "Codex of",
    "Catalogue of",
    "Letters on",
    "Grammar of",
    "Anatomy of",
    "Book of",
];

const ADJ: &[&str] = &[
    "Lesser",
    "Greater",
    "Hidden",
    "Silent",
    "Wandering",
    "Forgotten",
    "Gilded",
    "Hollow",
    "Lantern-lit",
    "Moonward",
    "Patient",
    "Quiet",
    "Radiant",
    "Salt-worn",
    "Tidal",
    "Umber",
    "Velvet",
    "Winter",
    "Amber",
    "Brindled",
    "Cinder",
    "Drowsy",
    "Errant",
    "Folded",
    "Glass",
    "Hearth",
    "Ivory",
    "Juniper",
    "Kindled",
    "Lost",
    "Mossy",
    "Nocturnal",
];

pub static CATEGORIES: &[Category] = &[
    // ---- West wing ----
    Category {
        name: "Astronomy",
        hue: 225.0,
        hue_spread: 18.0,
        sat: (0.45, 0.8),
        val: (0.2, 0.5),
        topics: &[
            "Comets",
            "the Night Sky",
            "Wandering Stars",
            "Eclipses",
            "the Seventh Moon",
            "Nebulae",
            "Starlight",
            "Orbits",
            "the Zodiac",
            "Meteors",
        ],
        things: &[
            "Orrery",
            "Star Chart",
            "Telescope",
            "Lodestar",
            "Firmament",
            "Constellation",
            "Aurora",
            "Equinox",
        ],
    },
    Category {
        name: "Alchemy",
        hue: 12.0,
        hue_spread: 16.0,
        sat: (0.55, 0.85),
        val: (0.3, 0.6),
        topics: &[
            "Salts",
            "Quicksilver",
            "Transmutation",
            "the Red Tincture",
            "Distillation",
            "Calcination",
            "Sulphur",
            "the Athanor",
            "Elixirs",
            "Vitriol",
        ],
        things: &[
            "Crucible",
            "Alembic",
            "Retort",
            "Philosopher's Egg",
            "Cinnabar",
            "Aqua Regia",
            "Ouroboros",
            "Furnace",
        ],
    },
    Category {
        name: "Botany",
        hue: 115.0,
        hue_spread: 25.0,
        sat: (0.35, 0.7),
        val: (0.22, 0.5),
        topics: &[
            "Mosses",
            "Night-blooming Flowers",
            "Ferns",
            "Mandrakes",
            "Hedgerows",
            "Seeds",
            "Fungi",
            "Orchards",
            "Bark",
            "Vines",
        ],
        things: &[
            "Herbarium",
            "Greenhouse",
            "Thornwood",
            "Mandrake",
            "Lichen",
            "Rootcellar",
            "Willow",
            "Seedvault",
        ],
    },
    Category {
        name: "Bestiary",
        hue: 175.0,
        hue_spread: 15.0,
        sat: (0.4, 0.7),
        val: (0.22, 0.48),
        topics: &[
            "Griffins",
            "River Serpents",
            "Moths",
            "Owls",
            "Small Dragons",
            "Hedge Wyrms",
            "Basilisks",
            "Kelpies",
            "Familiars",
            "Salamanders",
        ],
        things: &[
            "Wyvern",
            "Hippogriff",
            "Cockatrice",
            "Selkie",
            "Manticore",
            "Barghest",
            "Phoenix",
            "Leviathan",
        ],
    },
    Category {
        name: "Cartography",
        hue: 40.0,
        hue_spread: 10.0,
        sat: (0.45, 0.75),
        val: (0.4, 0.7),
        topics: &[
            "Coastlines",
            "the Northern Marches",
            "Lost Roads",
            "Islands",
            "Mountain Passes",
            "Underground Rivers",
            "the Borderlands",
            "Trade Winds",
            "Harbours",
            "Deserts",
        ],
        things: &[
            "Atlas",
            "Compass Rose",
            "Sea Chart",
            "Gazetteer",
            "Waymark",
            "Meridian",
            "Portolan",
            "Itinerary",
        ],
    },
    Category {
        name: "History",
        hue: 25.0,
        hue_spread: 10.0,
        sat: (0.3, 0.55),
        val: (0.15, 0.35),
        topics: &[
            "the Old Kingdoms",
            "the Glass War",
            "Fallen Houses",
            "the Long Winter",
            "Guilds",
            "Coronations",
            "Sieges",
            "the Salt Empire",
            "Treaties",
            "Pilgrims",
        ],
        things: &[
            "Chronicle",
            "Dynasty",
            "Regency",
            "Crown",
            "Citadel",
            "Rebellion",
            "Archive",
            "Heirloom",
        ],
    },
    Category {
        name: "Poetry",
        hue: 285.0,
        hue_spread: 20.0,
        sat: (0.3, 0.6),
        val: (0.3, 0.6),
        topics: &[
            "Small Hours",
            "Rain",
            "Lanterns",
            "the Sea",
            "Absent Friends",
            "Snowfall",
            "Candle Smoke",
            "Gardens",
            "Bells",
            "Thresholds",
        ],
        things: &[
            "Sonnets",
            "Ballads",
            "Elegies",
            "Odes",
            "Nocturnes",
            "Villanelles",
            "Hymns",
            "Lullabies",
        ],
    },
    Category {
        name: "Arcana",
        hue: 320.0,
        hue_spread: 15.0,
        sat: (0.5, 0.85),
        val: (0.18, 0.42),
        topics: &[
            "Wards",
            "Sigils",
            "Binding",
            "Glamours",
            "Counterspells",
            "Circles",
            "Illusions",
            "Summoning",
            "Runes",
            "Enchantment",
        ],
        things: &[
            "Grimoire",
            "Pentacle",
            "Cantrip",
            "Hex",
            "Talisman",
            "Incantation",
            "Spellbook",
            "Athame",
        ],
    },
    // ---- East wing ----
    Category {
        name: "Mathematics",
        hue: 205.0,
        hue_spread: 8.0,
        sat: (0.1, 0.3),
        val: (0.35, 0.65),
        topics: &[
            "Prime Numbers",
            "Curves",
            "Infinite Series",
            "Knots",
            "Tilings",
            "Proportion",
            "Spirals",
            "Symmetry",
            "Polyhedra",
            "Lattices",
        ],
        things: &[
            "Abacus",
            "Theorem",
            "Conjecture",
            "Geometry",
            "Calculus",
            "Algebra",
            "Axiom",
            "Quadrature",
        ],
    },
    Category {
        name: "Music",
        hue: 350.0,
        hue_spread: 12.0,
        sat: (0.5, 0.8),
        val: (0.25, 0.5),
        topics: &[
            "Harmony",
            "the Lute",
            "Bellringing",
            "Counterpoint",
            "Lullabies",
            "Wind Instruments",
            "Cadence",
            "Choirs",
            "Drums",
            "Tuning",
        ],
        things: &[
            "Fugue", "Canticle", "Sonata", "Madrigal", "Overture", "Rondo", "Carillon", "Requiem",
        ],
    },
    Category {
        name: "Divination",
        hue: 265.0,
        hue_spread: 15.0,
        sat: (0.4, 0.75),
        val: (0.2, 0.45),
        topics: &[
            "Tea Leaves",
            "Dreams",
            "Mirrors",
            "Omens",
            "Cards",
            "Flight of Birds",
            "Bones",
            "Candle Flames",
            "Tides",
            "Smoke",
        ],
        things: &[
            "Oracle",
            "Augury",
            "Scrying Glass",
            "Tarot",
            "Horoscope",
            "Portent",
            "Pendulum",
            "Crystal",
        ],
    },
    Category {
        name: "Mythology",
        hue: 50.0,
        hue_spread: 12.0,
        sat: (0.6, 0.9),
        val: (0.35, 0.6),
        topics: &[
            "Tricksters",
            "Sky Gods",
            "the Underworld",
            "Heroes",
            "Giants",
            "Sea Kings",
            "Fae Courts",
            "the First Fire",
            "Floods",
            "Twins",
        ],
        things: &[
            "Saga",
            "Epic",
            "Legend",
            "Eddas",
            "Pantheon",
            "Odyssey",
            "Fable",
            "Myth-cycle",
        ],
    },
    Category {
        name: "Linguistics",
        hue: 150.0,
        hue_spread: 12.0,
        sat: (0.25, 0.5),
        val: (0.3, 0.55),
        topics: &[
            "Dead Tongues",
            "Ciphers",
            "Dialects",
            "Etymology",
            "Scripts",
            "Silent Letters",
            "Riddles",
            "Phonetics",
            "Translation",
            "Glyphs",
        ],
        things: &[
            "Rosetta",
            "Glossary",
            "Thesaurus",
            "Phrasebook",
            "Concordance",
            "Palimpsest",
            "Alphabet",
            "Cipher-wheel",
        ],
    },
    Category {
        name: "Clockwork",
        hue: 30.0,
        hue_spread: 8.0,
        sat: (0.6, 0.9),
        val: (0.45, 0.7),
        topics: &[
            "Escapements",
            "Springs",
            "Automata",
            "Gears",
            "Pendulums",
            "Orreries",
            "Music Boxes",
            "Locks",
            "Balance Wheels",
            "Cogs",
        ],
        things: &[
            "Mechanism",
            "Automaton",
            "Chronometer",
            "Gearwork",
            "Ratchet",
            "Mainspring",
            "Tourbillon",
            "Engine",
        ],
    },
    Category {
        name: "Tempestry",
        hue: 195.0,
        hue_spread: 15.0,
        sat: (0.4, 0.7),
        val: (0.3, 0.55),
        topics: &[
            "Fog",
            "Thunder",
            "Gentle Rain",
            "Hail",
            "Monsoons",
            "Weathervanes",
            "Clouds",
            "Frost",
            "Rainbows",
            "Gales",
        ],
        things: &[
            "Barometer",
            "Stormglass",
            "Windrose",
            "Squall",
            "Cloudbank",
            "Thunderhead",
            "Drizzle",
            "Zephyr",
        ],
    },
    Category {
        name: "Hearthcraft",
        hue: 70.0,
        hue_spread: 15.0,
        sat: (0.35, 0.6),
        val: (0.3, 0.55),
        topics: &[
            "Bread",
            "Preserves",
            "Teas",
            "Soups",
            "Candles",
            "Honey",
            "Cheeses",
            "Pies",
            "Spices",
            "Feast Days",
        ],
        things: &[
            "Larder",
            "Cookbook",
            "Kettle",
            "Pantry",
            "Hearth",
            "Stillroom",
            "Bakehouse",
            "Cauldron",
        ],
    },
];

pub const WEST_CATEGORIES: usize = 8;

fn hsv(h: f32, s: f32, v: f32) -> [f32; 3] {
    let h = h.rem_euclid(360.0) / 60.0;
    let c = v * s;
    let x = c * (1.0 - ((h % 2.0) - 1.0).abs());
    let (r, g, b) = match h as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let m = v - c;
    // Return linear-ish colour (square approximates sRGB->linear).
    let lin = |u: f32| (u + m).powf(2.2);
    [lin(r), lin(g), lin(b)]
}

pub fn category_color(c: usize) -> [f32; 3] {
    let cat = &CATEGORIES[c];
    hsv(cat.hue, 0.75, 0.85)
}

fn sort_key(title: &str) -> String {
    let t = title
        .strip_prefix("The ")
        .or_else(|| title.strip_prefix("A "))
        .or_else(|| title.strip_prefix("An "))
        .or_else(|| title.strip_prefix("On "))
        .unwrap_or(title);
    t.to_lowercase()
}

fn make_title(rng: &mut StdRng, cat: &Category) -> String {
    let topic = *cat.topics.choose(rng).unwrap();
    let thing = *cat.things.choose(rng).unwrap();
    let adj = *ADJ.choose(rng).unwrap();
    let person = *PEOPLE.choose(rng).unwrap();
    let form = *FORMS.choose(rng).unwrap();
    match rng.gen_range(0..9) {
        0 => format!(
            "{} {form} {topic}",
            if form.starts_with(['A', 'E', 'I', 'O', 'U']) {
                "An"
            } else {
                "A"
            }
        ),
        1 => format!("The {adj} {thing}"),
        2 => format!("{person}'s {thing}"),
        3 => format!("On {adj} {topic}"),
        4 => format!("The {thing} of {topic}"),
        5 => format!("{adj} {thing}s"),
        6 => format!("{person} on {topic}"),
        7 => format!("The {form} {topic}"),
        _ => format!("{thing} & {topic}"),
    }
}

/// Generate series for a category until their total width reaches `target_width`.
pub fn generate_series(
    rng: &mut StdRng,
    category: usize,
    target_width: f32,
    titles: &mut HashSet<String>,
) -> Vec<Series> {
    let cat = &CATEGORIES[category];
    let mut out = Vec::new();
    let mut width = 0.0;
    let mut attempts = 0;
    while width < target_width && attempts < 10_000 {
        attempts += 1;
        let title = make_title(rng, cat);
        if !titles.insert(title.clone()) {
            continue;
        }
        let volumes = match rng.gen_range(0..100) {
            0..=17 => 1,
            18..=35 => rng.gen_range(2..=3),
            36..=75 => rng.gen_range(4..=7),
            _ => rng.gen_range(8..=12),
        };
        let hue = cat.hue + rng.gen_range(-cat.hue_spread..=cat.hue_spread);
        let color = hsv(
            hue,
            rng.gen_range(cat.sat.0..=cat.sat.1),
            rng.gen_range(cat.val.0..=cat.val.1),
        );
        let height = rng.gen_range(0.2..0.36f32);
        let depth = (height * rng.gen_range(0.62..0.78)).min(0.33);
        let base_t = rng.gen_range(0.024..0.06f32);
        let thickness = (0..volumes)
            .map(|_| base_t * rng.gen_range(0.88..1.12))
            .collect::<Vec<_>>();
        let s = Series {
            category,
            sort_key: sort_key(&title),
            title,
            volumes,
            color,
            pattern: rng.gen_range(0..8),
            trim: rng.gen_range(0..4),
            height,
            depth,
            thickness,
        };
        width += s.width() + 0.02;
        out.push(s);
    }
    out.sort_by(|a, b| a.sort_key.cmp(&b.sort_key));
    out
}
