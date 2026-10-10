//! Generative music. Each piece is composed afresh as it begins: a key and a mode, a
//! pace and a metre, a few soft instruments, chord progressions, and a tune made of
//! a motif and its variations, in a song form (A A B A). A quiet spell follows each
//! piece, so the ambience carries the room between them and hours of play never hear
//! the same thing twice.

use super::dsp::*;
use super::engine::BLOCK;
use rand::prelude::*;

/// Seconds of quiet before the first piece, and between pieces.
const FIRST: (f64, f64) = (8.0, 16.0);
const BETWEEN: (f64, f64) = (35.0, 95.0);
/// Most notes sounding at once (the oldest give way).
const MAX_TONES: usize = 72;
/// The music's level in the mix.
const LEVEL: f32 = 0.45;

#[derive(Clone, Copy, PartialEq, Debug)]
enum Mode {
    Ionian,
    Lydian,
    Mixolydian,
    Dorian,
    Aeolian,
}

impl Mode {
    const ALL: [(Mode, f32); 5] = [
        (Mode::Ionian, 0.32),
        (Mode::Lydian, 0.14),
        (Mode::Mixolydian, 0.2),
        (Mode::Dorian, 0.22),
        (Mode::Aeolian, 0.12),
    ];

    fn scale(self) -> [i32; 7] {
        match self {
            Mode::Ionian => [0, 2, 4, 5, 7, 9, 11],
            Mode::Lydian => [0, 2, 4, 6, 7, 9, 11],
            Mode::Mixolydian => [0, 2, 4, 5, 7, 9, 10],
            Mode::Dorian => [0, 2, 3, 5, 7, 9, 10],
            Mode::Aeolian => [0, 2, 3, 5, 7, 8, 10],
        }
    }

    /// Semitones up from the tonic to the major key with the same notes.
    fn relative_major(self) -> i32 {
        match self {
            Mode::Ionian => 0,
            Mode::Lydian => 7,
            Mode::Mixolydian => 5,
            Mode::Dorian => 10,
            Mode::Aeolian => 3,
        }
    }

    /// Progressions of four chords, by the scale degree (from 0) of each root; none
    /// uses the mode's diminished chord.
    fn progressions(self) -> &'static [[i32; 4]] {
        match self {
            Mode::Ionian => &[
                [0, 5, 3, 4],
                [0, 3, 5, 4],
                [0, 2, 3, 4],
                [0, 3, 0, 4],
                [5, 3, 0, 4],
                [0, 4, 5, 3],
                [3, 4, 2, 5],
            ],
            Mode::Lydian => &[[0, 1, 0, 1], [0, 1, 4, 0], [0, 1, 2, 1], [0, 1, 5, 4], [5, 1, 0, 1]],
            Mode::Mixolydian => &[[0, 6, 3, 0], [0, 3, 6, 0], [0, 4, 6, 3], [0, 6, 4, 3], [3, 6, 0, 0]],
            Mode::Dorian => &[[0, 3, 0, 3], [0, 6, 3, 0], [0, 2, 3, 0], [0, 3, 6, 4], [2, 3, 0, 6]],
            Mode::Aeolian => &[[0, 5, 2, 6], [0, 3, 6, 0], [0, 5, 6, 0], [5, 6, 0, 0], [0, 2, 5, 6]],
        }
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Inst {
    MusicBox,
    Celesta,
    Piano,
    Harp,
    Pad,
    Bass,
}

/// Who plays a piece: the tune, the tune of its B section, broken chords, a pad,
/// and the bass.
#[derive(Clone, Copy, PartialEq)]
struct Ensemble {
    lead: Inst,
    alt: Inst,
    arp: Option<Inst>,
    pad: f32,
    bass: Option<Inst>,
}

const ENSEMBLES: [Ensemble; 4] = [
    Ensemble {
        lead: Inst::MusicBox,
        alt: Inst::Celesta,
        arp: None,
        pad: 1.0,
        bass: Some(Inst::Bass),
    },
    Ensemble {
        lead: Inst::Piano,
        alt: Inst::Piano,
        arp: Some(Inst::Piano),
        pad: 0.5,
        bass: Some(Inst::Piano),
    },
    Ensemble {
        lead: Inst::Celesta,
        alt: Inst::Harp,
        arp: Some(Inst::Harp),
        pad: 0.0,
        bass: Some(Inst::Harp),
    },
    Ensemble {
        lead: Inst::Harp,
        alt: Inst::Celesta,
        arp: None,
        pad: 1.3,
        bass: Some(Inst::Bass),
    },
];

#[derive(Clone, Copy)]
struct Note {
    /// Seconds from the piece's start.
    at: f64,
    /// Seconds held (for instruments that are damped or sustained).
    dur: f32,
    key: f32,
    vel: f32,
    inst: Inst,
    pan: f32,
}

/// A piece in its key: the tonic as a MIDI note, and the mode.
#[derive(Clone, Copy, Debug)]
struct Key {
    tonic: i32,
    mode: Mode,
}

impl Key {
    /// MIDI note of diatonic degree `d` (0 = the tonic in the tonic's octave).
    fn note(self, d: i32) -> f32 {
        let s = self.mode.scale();
        (self.tonic + 12 * d.div_euclid(7) + s[d.rem_euclid(7) as usize]) as f32
    }

    /// Semitones (-6..=5) from D to the major pentatonic whose notes are all in
    /// this key: what the tonal effects, written in D, are moved by.
    fn shift(self) -> i32 {
        (self.tonic + self.mode.relative_major() - 62 + 6).rem_euclid(12) - 6
    }
}

/// The chord on degree `root`: its degrees, root position, with a seventh if asked.
fn chord(root: i32, seventh: bool) -> Vec<i32> {
    let mut c = vec![root, root + 2, root + 4];
    if seventh {
        c.push(root + 6);
    }
    c
}

/// The degree nearest `d` within `range` that is a tone of `chord` (in any
/// octave).
fn snap(d: i32, chord: &[i32], (lo, hi): (i32, i32)) -> i32 {
    (0..=4)
        .flat_map(|k| [d - k, d + k])
        .filter(|x| (lo..=hi).contains(x))
        .find(|x| chord.iter().any(|c| (x - c).rem_euclid(7) == 0))
        .unwrap_or(d.clamp(lo, hi))
}

/// `d` turned back into `lo..=hi` at whichever end it ran past.
fn fold(d: i32, (lo, hi): (i32, i32)) -> i32 {
    if d > hi {
        (2 * hi - d).max(lo)
    } else if d < lo {
        (2 * lo - d).min(hi)
    } else {
        d
    }
}

/// Rhythm cells, one bar each, in beats; a negative length is a rest.
const CELLS_4: [&[f32]; 11] = [
    &[1.0, 1.0, 2.0],
    &[2.0, 1.0, 1.0],
    &[1.5, 0.5, 2.0],
    &[1.0, 1.0, 1.0, 1.0],
    &[3.0, 1.0],
    &[2.0, 2.0],
    &[0.5, 0.5, 1.0, 2.0],
    &[1.0, 0.5, 0.5, 2.0],
    &[-1.0, 1.0, 2.0],
    &[2.0, -1.0, 1.0],
    &[-2.0, 1.0, 1.0],
];
const CELLS_3: [&[f32]; 7] = [
    &[1.0, 1.0, 1.0],
    &[2.0, 1.0],
    &[1.0, 2.0],
    &[1.5, 0.5, 1.0],
    &[-1.0, 1.0, 1.0],
    &[0.5, 0.5, 2.0],
    &[2.0, -1.0],
];

/// A tune's seed: two bars of rhythm, and the steps (in scale degrees) between
/// its notes.
#[derive(Clone)]
struct Motif {
    bars: [&'static [f32]; 2],
    steps: Vec<i32>,
}

impl Motif {
    fn new(rng: &mut StdRng, meter: usize) -> Self {
        let cells: &[&'static [f32]] = if meter == 3 { &CELLS_3 } else { &CELLS_4 };
        let bars = [
            cells[rng.gen_range(0..cells.len())],
            cells[rng.gen_range(0..cells.len())],
        ];
        let n = bars.iter().map(|b| b.len()).sum();
        Motif {
            bars,
            steps: (0..n).map(|_| step(rng)).collect(),
        }
    }

    /// The same rhythm, its second half answering with new steps.
    fn answer(&self, rng: &mut StdRng) -> Self {
        let mut m = self.clone();
        let half = self.bars[0].len();
        for s in &mut m.steps[half..] {
            *s = step(rng);
        }
        m
    }
}

/// A melodic step: mostly by step, now and then a leap.
fn step(rng: &mut StdRng) -> i32 {
    *[-2, -1, -1, -1, 1, 1, 1, 2, 2, -3, 3, 4, 0].choose(rng).unwrap()
}

/// How the broken chords go through a section.
#[derive(Clone, Copy, PartialEq)]
enum Texture {
    /// None: the pad and the bass alone.
    Still,
    /// The chord rolled upward once a bar.
    Roll,
    /// A chord tone a beat.
    Walk,
    /// Broken chords in quavers.
    Flow,
}

/// A voice in the piece: who plays it, how loud, where, and the degree its tonic
/// sits at.
#[derive(Clone, Copy)]
struct Part {
    inst: Inst,
    vel: f32,
    pan: f32,
    octave: i32,
}

/// The degree (a multiple of 7) putting `key`'s tonic at or just above `floor`.
fn register(key: Key, floor: i32) -> i32 {
    let mut o = 0;
    while key.tonic + 12 * o < floor {
        o += 1;
    }
    while key.tonic + 12 * (o - 1) >= floor {
        o -= 1;
    }
    7 * o
}

/// Bells sit higher than strings.
fn floor_of(inst: Inst) -> i32 {
    match inst {
        Inst::MusicBox | Inst::Celesta => 67,
        _ => 60,
    }
}

/// Composing a piece: the notes, the key, and when it ends.
struct Composer<'a> {
    rng: &'a mut StdRng,
    key: Key,
    meter: usize,
    beat: f64,
    notes: Vec<Note>,
}

impl Composer<'_> {
    fn bar_at(&self, bar: usize) -> f64 {
        bar as f64 * self.bar_len()
    }

    fn bar_len(&self) -> f64 {
        self.meter as f64 * self.beat
    }

    fn push(&mut self, at: f64, dur: f64, key: f32, vel: f32, inst: Inst, pan: f32) {
        let jitter = self.rng.gen_range(-0.008..0.008);
        let vel = vel * self.rng.gen_range(0.88..1.0);
        self.notes.push(Note {
            at: (at + jitter).max(0.0),
            dur: dur as f32,
            key,
            vel,
            inst,
            pan,
        });
    }

    /// Two bars of `motif` from `bar` for `part`, starting on degree `from` (snapped
    /// to the chord) and kept within a ninth of its tonic; returns the last degree
    /// played.
    fn phrase(&mut self, m: &Motif, bar: usize, from: i32, chords: &[Vec<i32>], part: Part) -> i32 {
        const RANGE: (i32, i32) = (-1, 8);
        let mut d = from;
        let mut k = 0;
        for (b, cell) in m.bars.iter().enumerate() {
            let chord = &chords[b];
            let mut beat = 0.0f32;
            for &len in cell.iter() {
                let step = m.steps[k];
                k += 1;
                if len < 0.0 {
                    beat -= len;
                    continue;
                }
                let next = fold(if k == 1 { d } else { d + step }, RANGE);
                // Downbeats and long notes sit on the chord.
                d = if beat == 0.0 || len >= 2.0 {
                    snap(next, chord, RANGE)
                } else {
                    next
                };
                let at = self.bar_at(bar + b) + beat as f64 * self.beat;
                let accent = if beat == 0.0 { 1.0 } else { 0.85 };
                let key = self.key.note(d + part.octave);
                self.push(at, len as f64 * self.beat, key, part.vel * accent, part.inst, part.pan);
                beat += len;
            }
        }
        d
    }

    /// The pad, the bass and the broken chords under `chords`, a bar each from `bar`.
    fn accompany(
        &mut self,
        bar: usize,
        chords: &[Vec<i32>],
        ens: Ensemble,
        texture: Texture,
        voicing: &mut Vec<f32>,
        vel: f32,
    ) {
        let bar_len = self.bar_len();
        for (b, ch) in chords.iter().enumerate() {
            let at = self.bar_at(bar + b);
            if ens.pad > 0.0 {
                // Voice-lead: each chord tone near where the last chord's were.
                let mut next = Vec::new();
                for (i, &d) in ch.iter().enumerate() {
                    let t = self.key.note(d);
                    let target = voicing.get(i).copied().unwrap_or(58.0);
                    let best = [-24.0, -12.0, 0.0, 12.0]
                        .into_iter()
                        .map(|o| t + o)
                        .filter(|n| (50.0..=69.0).contains(n))
                        .min_by(|a, b| (a - target).abs().total_cmp(&(b - target).abs()))
                        .unwrap_or(t);
                    next.push(best);
                }
                for (i, &n) in next.iter().enumerate() {
                    let pan = if i % 2 == 0 { -0.35 } else { 0.35 };
                    self.push(at, bar_len + 0.4, n, ens.pad * vel, Inst::Pad, pan);
                }
                *voicing = next;
            }
            if let Some(bass) = ens.bass {
                let mut root = self.key.note(ch[0]);
                while root > 50.0 {
                    root -= 12.0;
                }
                while root < 38.0 {
                    root += 12.0;
                }
                self.push(at, bar_len * 0.95, root, vel, bass, 0.0);
                if self.meter == 4 && self.rng.gen_bool(0.3) {
                    self.push(at + 2.0 * self.beat, bar_len * 0.45, root + 7.0, vel * 0.7, bass, 0.0);
                }
            }
            let Some(arp) = ens.arp else { continue };
            // The chord from its root up through an octave, round middle C.
            let mut root = self.key.note(ch[0]);
            while root >= 60.0 {
                root -= 12.0;
            }
            while root < 48.0 {
                root += 12.0;
            }
            let lift = root - self.key.note(ch[0]);
            let tones: Vec<f32> = [0, 2, 4, 7].iter().map(|&i| self.key.note(ch[0] + i) + lift).collect();
            let (pattern, step): (&[usize], f64) = match texture {
                Texture::Still => (&[], 0.0),
                Texture::Roll => (&[0, 1, 2, 3], 0.07),
                Texture::Walk if self.meter == 3 => (&[0, 2, 1], self.beat),
                Texture::Walk => (&[0, 2, 1, 2], self.beat),
                Texture::Flow if self.meter == 3 => (&[0, 1, 2, 3, 2, 1], self.beat * 0.5),
                Texture::Flow => (&[0, 2, 1, 2, 3, 2, 1, 2], self.beat * 0.5),
            };
            for (i, &p) in pattern.iter().enumerate() {
                let accent = if i == 0 { 1.0 } else { 0.7 };
                let dur = if texture == Texture::Roll { bar_len } else { step * 1.6 };
                self.push(at + i as f64 * step, dur, tones[p], vel * accent * 0.8, arp, -0.3);
            }
        }
    }
}

/// Compose a piece after `prev` (whose key and players it avoids repeating): its
/// notes, its key, its length in seconds and its ensemble.
fn compose(rng: &mut StdRng, prev: Option<(Key, usize)>) -> (Vec<Note>, Key, f64, usize) {
    let tonic = loop {
        let t = *[60, 62, 63, 65, 67].choose(rng).unwrap();
        if prev.is_none_or(|(k, _)| k.tonic != t) {
            break t;
        }
    };
    let mode = Mode::ALL.choose_weighted(rng, |m| m.1).unwrap().0;
    let ens_i = loop {
        let e = rng.gen_range(0..ENSEMBLES.len());
        if prev.is_none_or(|(_, p)| p != e) {
            break e;
        }
    };
    let ens = ENSEMBLES[ens_i];
    let meter = if rng.gen_bool(0.3) { 3 } else { 4 };
    let bpm = if meter == 3 {
        rng.gen_range(66.0..80.0)
    } else {
        rng.gen_range(56.0..70.0)
    };
    let key = Key { tonic, mode };
    let mut c = Composer {
        rng,
        key,
        meter,
        beat: 60.0 / bpm,
        notes: Vec::new(),
    };
    let progs = mode.progressions();
    let pa = progs[c.rng.gen_range(0..progs.len())];
    let pb = progs[c.rng.gen_range(0..progs.len())];
    let sevenths = c.rng.gen_bool(0.5);
    let chords_of = |p: [i32; 4], rng: &mut StdRng| -> Vec<Vec<i32>> {
        // Eight bars: the progression twice, a bar a chord, or once, two bars a chord.
        let slow = rng.gen_bool(0.4);
        (0..8)
            .map(|b| {
                let root = if slow { p[b / 2] } else { p[b % 4] };
                chord(root, sevenths && root != 4 && rng.gen_bool(0.5))
            })
            .collect()
    };
    let a_chords = chords_of(pa, c.rng);
    let mut b_chords = chords_of(pb, c.rng);
    // The B section hands back to A on the dominant or the tonic.
    b_chords[7] = chord(if mode == Mode::Ionian { 4 } else { 0 }, false);
    let motif_a = Motif::new(c.rng, meter);
    let answer_a = motif_a.answer(c.rng);
    let motif_b = Motif::new(c.rng, meter);
    let answer_b = motif_b.answer(c.rng);
    let lead = Part {
        inst: ens.lead,
        vel: 1.0,
        pan: 0.15,
        octave: register(key, floor_of(ens.lead)),
    };
    let alt = Part {
        inst: ens.alt,
        vel: 0.8,
        pan: 0.1,
        octave: register(key, floor_of(ens.alt)),
    };
    let form: &[bool] = if c.rng.gen_bool(0.7) {
        &[false, false, true, false]
    } else {
        &[false, true, false]
    };
    // The broken chords: sparse to begin, at most one section flowing.
    let mut textures: Vec<Texture> = (0..form.len())
        .map(|s| match s {
            0 => *[Texture::Still, Texture::Roll].choose(c.rng).unwrap(),
            _ => *[Texture::Roll, Texture::Walk, Texture::Still].choose(c.rng).unwrap(),
        })
        .collect();
    if c.rng.gen_bool(0.5) {
        textures[form.len() - 1] = Texture::Flow;
    }
    let mut voicing = Vec::new();
    // A bar of the accompaniment alone to begin.
    c.accompany(0, &a_chords[..1], ens, Texture::Roll, &mut voicing, 0.7);
    let mut bar = 1;
    for (s, &is_b) in form.iter().enumerate() {
        let last = s == form.len() - 1;
        let (chords, vel) = if is_b {
            (&b_chords, 0.8)
        } else {
            (&a_chords, if s == 0 { 0.85 } else { 1.0 })
        };
        c.accompany(bar, chords, ens, textures[s], &mut voicing, vel);
        if is_b {
            let d = c.phrase(&motif_b, bar, 4, &chords[0..2], alt);
            let d = c.phrase(&answer_b, bar + 2, d, &chords[2..4], alt);
            let d = c.phrase(&motif_b, bar + 4, d + 1, &chords[4..6], alt);
            c.phrase(&answer_b, bar + 6, d, &chords[6..8], alt);
        } else {
            let d = c.phrase(&motif_a, bar, 2, &chords[0..2], lead);
            let d = c.phrase(&answer_a, bar + 2, d, &chords[2..4], lead);
            let d = c.phrase(&motif_a, bar + 4, d + 2, &chords[4..6], lead);
            c.phrase(if last { &answer_a } else { &motif_a }, bar + 6, d, &chords[6..8], lead);
        }
        bar += 8;
    }
    // Close on the tonic, left to ring.
    let end = c.bar_at(bar);
    c.accompany(bar, &[chord(0, false)], ens, Texture::Roll, &mut voicing, 0.8);
    let close = key.note(lead.octave);
    c.push(end, c.bar_len() * 2.0, close, 0.8, lead.inst, lead.pan);
    let len = c.bar_at(bar + 1) + 5.0;
    c.notes.sort_by(|a, b| a.at.total_cmp(&b.at));
    (c.notes, key, len, ens_i)
}

// ---------------------------------------------------------------- the players

/// A sounding note.
struct Tone {
    partials: Partials,
    /// Samples until it starts, then until it is released.
    wait: usize,
    hold: usize,
    env: f32,
    attack: f32,
    release: f32,
    gains: (f32, f32),
}

impl Tone {
    fn new(note: &Note, wait: usize) -> Self {
        let f = midi_hz(note.key);
        let mut p = Partials::default();
        let (gain, attack, release) = match note.inst {
            Inst::MusicBox => {
                let tau = (1.7 * (523.0 / f).powf(0.5)).clamp(0.6, 3.0);
                p.add(f, 1.0, tau);
                p.add(f * 2.0, 0.16, tau * 0.4);
                p.add(f * 4.08, 0.05, tau * 0.15);
                p.add(f * 6.1, 0.035, tau * 0.08);
                (0.2, 0.001, f32::INFINITY)
            }
            Inst::Celesta => {
                let tau = (1.4 * (523.0 / f).powf(0.5)).clamp(0.5, 2.6);
                p.add(f, 1.0, tau);
                p.add(f * 2.0, 0.1, tau * 0.35);
                p.add(f * 3.0, 0.025, tau * 0.2);
                (0.19, 0.003, f32::INFINITY)
            }
            Inst::Piano => {
                let tau = (2.6 * (262.0 / f).powf(0.45)).clamp(0.6, 5.0);
                for k in 1..=7 {
                    let k = k as f32;
                    let felt = if k > 4.0 { 0.5f32.powf(k - 4.0) } else { 1.0 };
                    let stretch = (1.0 + 0.0004 * k * k).sqrt();
                    p.add(f * k * stretch, felt / k.powf(1.5), tau / (1.0 + 0.55 * (k - 1.0)));
                }
                // A second string a hair sharp, dying sooner: the piano's two-stage decay.
                p.add(f * 1.0009, 0.6, tau * 0.35);
                p.add(f * 2.0016, 0.25, tau * 0.25);
                (0.11, 0.004, 0.25)
            }
            Inst::Harp => {
                let tau = (2.6 * (220.0 / f).powf(0.5)).clamp(0.6, 4.5);
                for k in 1..=8 {
                    let k = k as f32;
                    let pluck = (k * std::f32::consts::PI * 0.17).sin().abs() / k.powf(1.2);
                    p.add(f * k, pluck, tau / (1.0 + 0.45 * (k - 1.0)));
                }
                (0.2, 0.0015, f32::INFINITY)
            }
            Inst::Pad => {
                for (k, a) in [(1.0, 1.0), (2.0, 0.3), (3.0, 0.1), (4.0, 0.04)] {
                    p.add(f * k, a, f32::INFINITY);
                    p.add(f * k * 1.0012, a * 0.8, f32::INFINITY);
                }
                (0.026, 1.6, 1.6)
            }
            Inst::Bass => {
                let tau = 2.2;
                p.add(f, 1.0, tau);
                p.add(f * 2.0, 0.25, tau * 0.5);
                p.add(f * 3.0, 0.06, tau * 0.3);
                (0.2, 0.012, 0.4)
            }
        };
        let (l, r) = pan_gains(note.pan);
        let g = gain * note.vel;
        Tone {
            partials: p,
            wait,
            hold: secs(note.dur),
            env: 0.0,
            attack: 1.0 / (attack * SRF),
            release: if release.is_finite() {
                decay_per_sample(release)
            } else {
                1.0
            },
            gains: (l * g, r * g),
        }
    }

    /// Add this block's samples to `l` and `r`; false once it has died away.
    fn render(&mut self, l: &mut [f32], r: &mut [f32]) -> bool {
        let start = self.wait.min(l.len());
        self.wait -= start;
        for i in start..l.len() {
            if self.hold > 0 || self.release == 1.0 {
                self.hold = self.hold.saturating_sub(1);
                self.env = (self.env + self.attack).min(1.0);
            } else {
                self.env *= self.release;
            }
            let s = self.partials.next() * self.env;
            l[i] += s * self.gains.0;
            r[i] += s * self.gains.1;
        }
        self.wait > 0 || self.partials.tidy() * self.env > 1e-3
    }
}

enum State {
    Quiet {
        left: f64,
    },
    Playing {
        notes: Vec<Note>,
        next: usize,
        t: f64,
        len: f64,
    },
}

/// The music player: composes, then plays, then rests, and so on.
pub struct Music {
    rng: StdRng,
    state: State,
    tones: Vec<Tone>,
    reverb: Reverb,
    /// Scratch: the dry mix, the reverb's send and its return.
    dry: [Vec<f32>; 2],
    send: Vec<f32>,
    wet: [Vec<f32>; 2],
    prev: Option<(Key, usize)>,
    shift: i32,
}

impl Music {
    pub fn new(seed: u64) -> Self {
        let mut rng = StdRng::seed_from_u64(seed);
        let left = rng.gen_range(FIRST.0..FIRST.1);
        Music {
            rng,
            state: State::Quiet { left },
            tones: Vec::new(),
            reverb: Reverb::new(2.6, 0.35),
            dry: [vec![0.0; BLOCK], vec![0.0; BLOCK]],
            send: vec![0.0; BLOCK],
            wet: [vec![0.0; BLOCK], vec![0.0; BLOCK]],
            prev: None,
            shift: 0,
        }
    }

    /// Semitones from D to the major pentatonic that sits with the music now (the
    /// last piece's, between pieces).
    pub fn shift(&self) -> i32 {
        self.shift
    }

    /// Add a block of music to `l` and `r`.
    pub fn render(&mut self, l: &mut [f32], r: &mut [f32]) {
        let dt = BLOCK as f64 / SRF as f64;
        match &mut self.state {
            State::Quiet { left } => {
                *left -= dt;
                if *left <= 0.0 {
                    let (notes, key, len, ens) = compose(&mut self.rng, self.prev);
                    self.prev = Some((key, ens));
                    self.shift = key.shift();
                    self.state = State::Playing {
                        notes,
                        next: 0,
                        t: 0.0,
                        len,
                    };
                }
            }
            State::Playing { notes, next, t, len } => {
                while *next < notes.len() && notes[*next].at < *t + dt {
                    let n = &notes[*next];
                    let wait = ((n.at - *t).max(0.0) * SRF as f64) as usize;
                    if self.tones.len() >= MAX_TONES {
                        self.tones.remove(0);
                    }
                    self.tones.push(Tone::new(n, wait));
                    *next += 1;
                }
                *t += dt;
                if *t >= *len {
                    let left = self.rng.gen_range(BETWEEN.0..BETWEEN.1);
                    self.state = State::Quiet { left };
                }
            }
        }
        let [dl, dr] = &mut self.dry;
        let [wl, wr] = &mut self.wet;
        dl.fill(0.0);
        dr.fill(0.0);
        wl.fill(0.0);
        wr.fill(0.0);
        self.tones.retain_mut(|t| t.render(dl, dr));
        for (s, (a, b)) in self.send.iter_mut().zip(dl.iter().zip(dr.iter())) {
            *s = (a + b) * 0.5;
        }
        self.reverb.process(&self.send, wl, wr);
        for i in 0..BLOCK {
            l[i] += (dl[i] + wl[i] * 0.55) * LEVEL;
            r[i] += (dr[i] + wr[i] * 0.55) * LEVEL;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pitch_classes(key: Key) -> Vec<i32> {
        (0..7).map(|d| (key.note(d) as i32).rem_euclid(12)).collect()
    }

    #[test]
    fn the_effects_moved_to_the_key_sit_in_it() {
        for tonic in 55..72 {
            for (mode, _) in Mode::ALL {
                let key = Key { tonic, mode };
                let scale = pitch_classes(key);
                for d in 0..5 {
                    let pc = (penta(4, d) as i32 + key.shift()).rem_euclid(12);
                    assert!(scale.contains(&pc), "{key:?}: degree {d} lands off the scale");
                }
            }
        }
    }

    #[test]
    fn pieces_keep_to_their_key_and_a_gentle_register() {
        let mut rng = StdRng::seed_from_u64(5);
        let mut prev: Option<(Key, usize)> = None;
        for _ in 0..60 {
            let (notes, key, len, ens) = compose(&mut rng, prev);
            assert!(prev.is_none_or(|(k, e)| k.tonic != key.tonic && e != ens));
            prev = Some((key, ens));
            assert!((50.0..200.0).contains(&len), "a piece {len:.0} s long");
            let scale = pitch_classes(key);
            for n in &notes {
                assert!(
                    scale.contains(&(n.key as i32).rem_euclid(12)),
                    "{key:?}: {} is off the scale",
                    n.key
                );
                assert!((38.0..=91.0).contains(&n.key), "{:?} at {}", n.inst, n.key);
                assert!(n.at < len);
            }
        }
    }
}
