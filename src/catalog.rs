//! Procedural catalogue: categories, series and their visual identity.

use rand::prelude::*;
use std::collections::HashSet;

/// One category: where its books are shelved, and the colours and words its series
/// are made from.
pub struct Category {
    pub name: String,
    pub dept: usize,
    /// Hue in degrees.
    pub hue: f32,
    pub hue_spread: f32,
    pub sat: (f32, f32),
    pub val: (f32, f32),
    pub topics: Vec<String>,
    pub things: Vec<String>,
    /// Spine pattern shared by every series in the category (the Great Library's
    /// categories are told apart by spine within a department's colour family);
    /// `None` picks one per series.
    pub pattern: Option<u32>,
}

pub struct Department {
    pub name: String,
    pub categories: std::ops::Range<usize>,
}

/// Departments and their categories, read from a table (`catalog/*.txt`).
pub struct Catalogue {
    pub categories: Vec<Category>,
    pub departments: Vec<Department>,
}

impl Catalogue {
    pub fn hall() -> Self {
        Self::parse(include_str!("catalog/hall.txt"))
    }

    pub fn great() -> Self {
        Self::parse(include_str!("catalog/great.txt"))
    }

    /// Lines: `dept <name> [| colours]`, `cat <name> [| colours]`, `topics a; b; …`,
    /// `things a; b; …`, `spines by category`; `#` starts a comment. Colours are
    /// `<hue> <hue spread> | <sat lo> <sat hi> | <val lo> <val hi>`; a category
    /// without them takes its department's.
    fn parse(text: &str) -> Self {
        type Colours = (f32, f32, (f32, f32), (f32, f32));
        let colours = |fields: &[&str]| -> Option<Colours> {
            let nums = |s: &str| -> Vec<f32> { s.split_whitespace().map(|x| x.parse().expect("number")).collect() };
            let [h, s, v] = fields else { return None };
            let (h, s, v) = (nums(h), nums(s), nums(v));
            Some((h[0], h[1], (s[0], s[1]), (v[0], v[1])))
        };
        let mut cat = Catalogue {
            categories: Vec::new(),
            departments: Vec::new(),
        };
        let mut dept_colours: Option<Colours> = None;
        let mut by_category = false;
        let words = |rest: &str| rest.split(';').map(|w| w.trim().to_string()).collect::<Vec<_>>();
        for line in text.lines().map(str::trim) {
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let (key, rest) = line.split_once(' ').unwrap_or((line, ""));
            let fields: Vec<&str> = rest.split('|').map(str::trim).collect();
            match key {
                "spines" => by_category = rest == "by category",
                "dept" => {
                    dept_colours = colours(&fields[1..]);
                    let at = cat.categories.len();
                    cat.departments.push(Department {
                        name: fields[0].to_string(),
                        categories: at..at,
                    });
                }
                "cat" => {
                    let d = cat.departments.len() - 1;
                    let (hue, hue_spread, sat, val) = colours(&fields[1..])
                        .or(dept_colours)
                        .unwrap_or_else(|| panic!("no colours for {}", fields[0]));
                    let index = cat.departments[d].categories.len() as u32;
                    cat.categories.push(Category {
                        name: fields[0].to_string(),
                        dept: d,
                        hue,
                        hue_spread,
                        sat,
                        val,
                        topics: Vec::new(),
                        things: Vec::new(),
                        pattern: by_category.then_some(index % 8),
                    });
                    cat.departments[d].categories.end += 1;
                }
                "topics" => cat.categories.last_mut().expect("topics before cat").topics = words(rest),
                "things" => cat.categories.last_mut().expect("things before cat").things = words(rest),
                _ => panic!("catalogue: unknown line {line:?}"),
            }
        }
        // A book's instance carries both for the Seeker's Lens, in 5 and 8 bits.
        assert!(cat.departments.len() <= 32 && cat.categories.len() <= 256);
        cat
    }
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
    pub look: Look,
}

/// What a series is bound in.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Binding {
    /// Full leather over raised bands.
    Leather = 0,
    /// A leather spine and corners, marbled paper sides.
    Half = 1,
    /// Publisher's cloth, lettered by the stamp.
    Cloth = 2,
    /// Limp vellum, lettered by hand in ink.
    Vellum = 3,
    /// Paper over boards with printed paper labels.
    Boards = 4,
}

/// How the text block's edges are finished.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Edges {
    Plain = 0,
    /// Fine flecks of the accent colour.
    Sprinkled = 1,
    /// Gold leaf on the head (on all three edges of leather).
    Gilt = 2,
    /// Marbled in the accent colour.
    Marbled = 3,
    /// Coloured with the accent colour.
    Stained = 4,
}

/// A series' binding, beyond its colour, pattern and trim. Drawn from a random stream of
/// its own (seeded from its title and category), never the generation's, so changing
/// how books look never moves them.
#[derive(Clone, Copy, Debug)]
pub struct Look {
    pub binding: Binding,
    /// Raised bands across the spine; 0 for a smooth one.
    pub bands: u32,
    pub edges: Edges,
    /// The spine label's leather (leather and half bindings).
    pub label: [f32; 3],
    /// The edges' sprinkle, stain or marbling, and the marbled sides' second colour.
    pub accent: [f32; 3],
    /// How worn it is, 0 to 1.
    pub age: f32,
}

/// Leathers for spine labels, chosen to stand out from the binding.
const LABELS: &[[f32; 3]] = &[
    [0.30, 0.035, 0.025],
    [0.018, 0.016, 0.014],
    [0.025, 0.08, 0.035],
    [0.24, 0.11, 0.04],
    [0.025, 0.035, 0.11],
];

/// Sprinkles, stains and marbling.
const ACCENTS: &[[f32; 3]] = &[
    [0.42, 0.05, 0.03],
    [0.05, 0.09, 0.22],
    [0.40, 0.24, 0.05],
    [0.06, 0.20, 0.10],
    [0.18, 0.08, 0.03],
];

impl Look {
    /// `shared`: the category's books share a binding (and label), as they share a
    /// spine pattern, but for the odd one.
    fn new(title: &str, category: usize, color: [f32; 3], height: f32, shared: bool) -> Self {
        // FNV-1a of the title, then the category.
        let h = (title.bytes().chain((category as u32).to_le_bytes())).fold(0xcbf2_9ce4_8422_2325u64, |h, b| {
            (h ^ b as u64).wrapping_mul(0x0100_0000_01b3)
        });
        let rng = &mut StdRng::seed_from_u64(h);
        // Each category favours a binding of its own.
        let favoured = category % 5;
        let weights: Vec<u32> = (0..5)
            .map(|b| [32, 14, 32, 8, 14][b] + if b == favoured { 30 } else { 0 })
            .collect();
        let bindings = [
            Binding::Leather,
            Binding::Half,
            Binding::Cloth,
            Binding::Vellum,
            Binding::Boards,
        ];
        let binding = if shared && rng.gen_bool(0.85) {
            bindings[favoured]
        } else {
            bindings[rand::distributions::WeightedIndex::new(&weights)
                .expect("weights")
                .sample(rng)]
        };
        let bands = match binding {
            Binding::Leather => rng.gen_range(4..=5) + (height > 0.27) as u32,
            Binding::Half if rng.gen_bool(0.5) => rng.gen_range(4..=5),
            _ => 0,
        };
        use Edges::*;
        let edges: &[(Edges, u32)] = match binding {
            Binding::Leather => &[(Plain, 25), (Sprinkled, 30), (Gilt, 30), (Marbled, 15)],
            Binding::Half => &[(Plain, 35), (Sprinkled, 30), (Marbled, 35)],
            Binding::Cloth => &[(Plain, 45), (Sprinkled, 15), (Gilt, 15), (Stained, 25)],
            Binding::Vellum => &[(Plain, 70), (Stained, 30)],
            Binding::Boards => &[(Plain, 60), (Sprinkled, 25), (Stained, 15)],
        };
        let edges = edges.choose_weighted(rng, |e| e.1).expect("edges").0;
        let far = |c: &[f32; 3]| -> f32 { c.iter().zip(color).map(|(a, b)| (a.sqrt() - b.sqrt()).powi(2)).sum() };
        let mut labels: Vec<&[f32; 3]> = LABELS.iter().collect();
        labels.sort_by(|a, b| far(b).total_cmp(&far(a)));
        let label = if shared { category % 2 } else { rng.gen_range(0..3) };
        Look {
            binding,
            bands,
            edges,
            label: *labels[label],
            accent: *ACCENTS.choose(rng).expect("accents"),
            age: 0.15 + 0.85 * libm::powf(rng.r#gen::<f32>(), 1.3),
        }
    }
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

/// Kinds of book, with the word that leads to their subject. `make_title` draws from
/// lists as long as they have always been (15 of these, 32 `ADJ`, 26 `PEOPLE`), so the
/// draws after a title (volumes, colours, sizes) stay as they were.
const FORMS: &[(&str, &str)] = &[
    ("Treatise", "on"),
    ("Notes", "on"),
    ("Compendium", "of"),
    ("Field Guide", "to"),
    ("Primer", "of"),
    ("Annals", "of"),
    ("Lexicon", "of"),
    ("Meditations", "on"),
    ("Almanac", "of"),
    ("Natural History", "of"),
    ("Catalogue", "of"),
    ("Letters", "on"),
    ("Grammar", "of"),
    ("Anatomy", "of"),
    ("Commonplace Book", "of"),
];

/// Plurals the rules in `plural` would get wrong, in lower case.
const PLURALS: &[(&str, &str)] = &[
    ("abacus", "abacuses"),
    ("atlas", "atlases"),
    ("aurora", "aurorae"),
    ("automaton", "automata"),
    ("chrysalis", "chrysalises"),
    ("codex", "codices"),
    ("ephemeris", "ephemerides"),
    ("herbarium", "herbaria"),
    ("lens", "lenses"),
    ("loaf", "loaves"),
    ("lunarium", "lunaria"),
    ("nautilus", "nautili"),
    ("penumbra", "penumbrae"),
    ("thesaurus", "thesauri"),
    ("trellis", "trellises"),
    ("umbra", "umbrae"),
];

/// Things that are not counted: a title keeps them as they are.
const UNCOUNTED: &[&str] = &[
    "algebra",
    "aqua regia",
    "calculus",
    "calx",
    "cinnabar",
    "drizzle",
    "firmament",
    "gearwork",
    "geometry",
    "lapis",
    "mycelium",
    "nativity",
    "ouroboros",
    "quadrature",
    "regency",
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
    let lin = |u: f32| libm::powf(u + m, 2.2);
    [lin(r), lin(g), lin(b)]
}

pub fn category_color(cat: &Category) -> [f32; 3] {
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

/// "A" or "An" before `word`.
fn article(word: &str) -> &'static str {
    if word.starts_with(['A', 'E', 'I', 'O', 'U']) {
        "An"
    } else {
        "A"
    }
}

/// Whether a kind of book is named in the plural ("Notes", "Annals").
fn plural_form(form: &str) -> bool {
    form.ends_with('s')
}

/// A thing's plural ("Orreries", "Compass Roses", "Rings of Fire"); things already
/// plural, or not counted, stay as they are.
fn plural(thing: &str) -> String {
    let lower = thing.to_lowercase();
    if UNCOUNTED.contains(&lower.as_str()) {
        return thing.to_string();
    }
    // The noun is the last word, or the one before "of" or "in".
    let (head, tail) = [" of ", " in "]
        .iter()
        .find_map(|sep| thing.find(sep).map(|at| thing.split_at(at)))
        .unwrap_or((thing, ""));
    let (lead, noun) = head.rsplit_once(' ').map_or(("", head), |(l, n)| (l, n));
    let low = noun.to_lowercase();
    let many = if let Some(&(_, p)) = PLURALS.iter().find(|(one, _)| *one == low) {
        // Keep the noun's own capital.
        noun[..1].to_string() + &p[1..]
    } else if low.ends_with("ss") || low.ends_with('x') || low.ends_with("ch") || low.ends_with("sh") {
        format!("{noun}es")
    } else if low.ends_with('s') {
        noun.to_string()
    } else if low.ends_with('y') && !low[..low.len() - 1].ends_with(['a', 'e', 'o', 'u']) {
        format!("{}ies", &noun[..noun.len() - 1])
    } else {
        format!("{noun}s")
    };
    let sep = if lead.is_empty() { "" } else { " " };
    format!("{lead}{sep}{many}{tail}")
}

/// A number made from `words`, to vary a title's turn of phrase by what it says rather
/// than by another draw.
fn turn(words: &[&str], n: usize) -> usize {
    let h = (words.iter().flat_map(|w| w.bytes().chain([0])))
        .fold(0x811c_9dc5u32, |h, b| (h ^ b as u32).wrapping_mul(0x0100_0193));
    (h % n as u32) as usize
}

/// A series' title. Each shape uses the same words drawn as it always has, and no two
/// sets of them give the same title, so titles clash exactly when they used to: the
/// draws that follow (volumes, colours, sizes) are as they were.
fn make_title(rng: &mut StdRng, cat: &Category) -> String {
    let topic = cat.topics.choose(rng).unwrap();
    let thing = cat.things.choose(rng).unwrap();
    let adj = *ADJ.choose(rng).unwrap();
    let person = *PEOPLE.choose(rng).unwrap();
    let (form, by) = *FORMS.choose(rng).unwrap();
    match rng.gen_range(0..9) {
        0 => {
            let lead = ["", "", "Short ", "New ", "Little "][turn(&[form, topic], 5)];
            if plural_form(form) {
                format!("{lead}{form} {by} {topic}")
            } else {
                let first = if lead.is_empty() { form } else { lead };
                format!("{} {lead}{form} {by} {topic}", article(first))
            }
        }
        1 => format!("The {adj} {thing}"),
        2 => format!("{person}'s {thing}"),
        3 => {
            let on = ["On", "On", "Concerning"][turn(&[adj, topic], 3)];
            match topic.strip_prefix("the ") {
                Some(rest) => format!("{on} the {adj} {rest}"),
                None => format!("{on} {adj} {topic}"),
            }
        }
        4 => format!("The {thing} of {topic}"),
        5 => format!("{adj} {}", plural(thing)),
        6 => match turn(&[person, topic], 3) {
            0 => format!("{person}'s Notes on {topic}"),
            _ => format!("{person} on {topic}"),
        },
        7 => {
            let mut chars = topic.chars();
            let topic: String = chars
                .next()
                .map(|c| c.to_ascii_uppercase())
                .into_iter()
                .chain(chars)
                .collect();
            if plural_form(form) {
                format!("{topic}: {form}")
            } else {
                format!("{topic}: {} {form}", article(form))
            }
        }
        _ => format!("{thing} & {topic}"),
    }
}

/// Generate series for a category until their total width reaches `target_width`.
pub fn generate_series(
    rng: &mut StdRng,
    cat: &Category,
    category: usize,
    target_width: f32,
    titles: &mut HashSet<String>,
) -> Vec<Series> {
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
            look: Look::new(&title, category, color, height, cat.pattern.is_some()),
            title,
            volumes,
            color,
            pattern: cat.pattern.unwrap_or_else(|| rng.gen_range(0..8)),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn things_go_into_the_plural() {
        for (one, many) in [
            ("Orrery", "Orreries"),
            ("Odyssey", "Odysseys"),
            ("Compass Rose", "Compass Roses"),
            ("Ring of Fire", "Rings of Fire"),
            ("Message in a Bottle", "Messages in a Bottle"),
            ("Envoy's Pouch", "Envoy's Pouches"),
            ("Hex", "Hexes"),
            ("Spyglass", "Spyglasses"),
            ("Harvest Loaf", "Harvest Loaves"),
            ("Automaton", "Automata"),
            ("Sonnets", "Sonnets"),
            ("Calx", "Calx"),
        ] {
            assert_eq!(plural(one), many);
        }
        assert_eq!((article("Almanac"), article("Treatise")), ("An", "A"));
    }

    /// Titles read as English: no article before a plural, no stray plurals, no "the"
    /// after an adjective.
    #[test]
    fn titles_read_well() {
        let cats = Catalogue::great();
        let rng = &mut StdRng::seed_from_u64(7);
        for cat in &cats.categories {
            for _ in 0..200 {
                let t = make_title(rng, cat);
                for bad in ["A Notes", "A Annals", "An Annals", " the the ", "sss", "ys ", "Calxs"] {
                    assert!(!t.contains(bad), "{t}");
                }
                assert!(!ADJ.iter().any(|a| t.contains(&format!("{a} the "))), "{t}");
            }
        }
    }
}
