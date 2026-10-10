//! The ambience: each place's bed of sound (its air, its weather, its bit of
//! magic), faded in as the listener comes into it and out as they leave; fires
//! heard from where they burn; familiars' wings where they fly.

use super::clips::{Banks, Grain};
use super::dsp::*;
use super::engine::{DT, Ears, Heard, Voice};
use super::{MAX_FIRES, MAX_WINGS, Place, Scene};
use glam::Vec3;
use rand::prelude::*;
use std::f32::consts::{PI, TAU};

/// A value wandering smoothly between random targets, for gusts and flicker.
struct Wander {
    value: f32,
    target: f32,
    /// Targets a second, about.
    rate: f32,
    wait: f32,
}

impl Wander {
    fn new(rate: f32) -> Self {
        Wander {
            value: 0.5,
            target: 0.5,
            rate,
            wait: 0.0,
        }
    }

    /// Step a block; returns the values at its start and end.
    fn step(&mut self, rng: &mut StdRng) -> (f32, f32) {
        self.wait -= DT;
        if self.wait <= 0.0 {
            self.target = rng.gen_range(0.0..1.0);
            self.wait = rng.gen_range(0.5..1.5) / self.rate;
        }
        let from = self.value;
        self.value += (self.target - self.value) * (1.0 - (-DT * self.rate * 1.5).exp());
        (from, self.value)
    }
}

/// Tones the ambience rings out as they sound, in the music's key.
#[derive(Clone, Copy, PartialEq, Debug)]
enum Ring {
    /// A hanging crystal: glassy, inharmonic, slow to fade.
    Crystal,
    /// A star's glint: high, soft and pure.
    Glint,
    /// A harp string sounding by itself: it swells rather than being plucked.
    String,
    /// One tube of a wind chime.
    Chime,
    /// The great clock striking once, far back in its case.
    Strike,
}

impl Ring {
    /// A tone of this ring on pentatonic degree `degree` (in D, raised by `shift`):
    /// its partials, its attack in seconds, and its tremolo (rate, depth).
    fn tone(self, degree: i32, shift: f32) -> (Partials, f32, (f32, f32)) {
        let mut p = Partials::default();
        let mut add = |f: f32, parts: &[(f32, f32, f32)]| {
            for &(ratio, amp, tau) in parts {
                p.add(f * ratio * shift, amp, tau);
            }
        };
        let (attack, tremolo) = match self {
            Ring::Crystal => {
                add(
                    midi_hz(penta(5, degree)),
                    &[
                        (1.0, 1.0, 1.0),
                        (2.32, 0.3, 0.5),
                        (4.25, 0.12, 0.25),
                        (6.63, 0.05, 0.12),
                    ],
                );
                (0.015, (0.0, 0.0))
            }
            Ring::Glint => {
                add(midi_hz(penta(5, degree)), &[(1.0, 1.0, 0.5), (2.0, 0.08, 0.2)]);
                (0.04, (0.0, 0.0))
            }
            Ring::String => {
                add(
                    midi_hz(penta(4, degree)),
                    &[(1.0, 1.0, 1.4), (2.0, 0.4, 1.0), (3.0, 0.15, 0.7), (4.0, 0.06, 0.5)],
                );
                (0.6, (0.7, 0.2))
            }
            Ring::Chime => {
                add(
                    midi_hz(penta(5, degree)),
                    &[(1.0, 1.0, 1.2), (2.76, 0.35, 0.5), (5.4, 0.12, 0.2), (8.93, 0.04, 0.08)],
                );
                (0.001, (0.0, 0.0))
            }
            Ring::Strike => {
                let parts = [
                    (0.5, 0.5, 1.8),
                    (1.0, 1.0, 1.4),
                    (1.19, 0.35, 1.0),
                    (1.5, 0.25, 0.8),
                    (2.0, 0.3, 0.6),
                    (2.52, 0.12, 0.4),
                ];
                add(midi_hz(penta(3, 0)), &parts);
                (0.002, (0.0, 0.0))
            }
        };
        (p, attack, tremolo)
    }
}

/// A ring sounding.
struct Ringing {
    partials: Partials,
    /// Samples before it starts.
    wait: usize,
    env: f32,
    attack: f32,
    tremolo: (f32, f32),
    phase: f32,
    gains: (f32, f32),
    send: f32,
}

impl Ringing {
    /// Add this block to the dry pair and the room's send; false once it has died.
    fn render(&mut self, l: &mut [f32], r: &mut [f32], wet: &mut [f32]) -> bool {
        let start = self.wait.min(l.len());
        self.wait -= start;
        for i in start..l.len() {
            self.env = (self.env + self.attack).min(1.0);
            self.phase = (self.phase + self.tremolo.0 / SRF).fract();
            let trem = 1.0 + self.tremolo.1 * (TAU * self.phase).sin();
            let x = self.partials.next() * self.env * trem;
            l[i] += x * self.gains.0;
            r[i] += x * self.gains.1;
            wet[i] += x * self.send;
        }
        self.wait > 0 || self.partials.tidy() > 1e-4
    }
}

/// What a scatter scatters: grains from the banks, or rings.
#[derive(Clone, Copy)]
enum Sound {
    Grain(Grain),
    Ring(Ring),
}

/// A pair of filters, one per ear, fed decorrelated noise.
#[derive(Clone, Default)]
struct Pair([Svf; 2]);

impl Pair {
    fn new(cutoff: f32, q: f32) -> Self {
        Pair([Svf::new(cutoff, q), Svf::new(cutoff, q)])
    }

    fn set(&mut self, cutoff: f32, q: f32) {
        for f in &mut self.0 {
            f.set(cutoff, q);
        }
    }
}

/// One strand of a place's sound.
enum Layer {
    /// Still air: a low, soft hush, with `air` of a faint high one.
    Hush {
        level: f32,
        air: f32,
        low: Pair,
        high: Pair,
    },
    /// Moving air in gusts; `loft` is how much stronger it grows up under the
    /// atrium's roof.
    Wind {
        level: f32,
        band: f32,
        loft: f32,
        gust: Wander,
        f: Pair,
    },
    /// Waves breaking slowly, far off.
    Sea {
        level: f32,
        t: f32,
        period: f32,
        body: Pair,
        foam: Pair,
    },
    /// Rain's wash against the glass (the drops are a `Scatter`).
    Wash { level: f32, f: Pair, gust: Wander },
    /// A low hum of a few slowly beating tones, in the music's key.
    Hum { level: f32, phases: [f32; 6] },
    /// Grains at random, one every `every` seconds on average, `cluster` at a time
    /// `spacing` apart.
    Scatter {
        sound: Sound,
        every: f32,
        gain: (f32, f32),
        spread: f32,
        cluster: (usize, usize),
        spacing: f32,
        wait: f32,
    },
    /// Grains in a steady beat, turn about from `grains`.
    Beat {
        grains: [Grain; 2],
        period: f32,
        gain: f32,
        pan: f32,
        wait: f32,
        n: usize,
    },
}

fn hush(level: f32, cutoff: f32, air: f32) -> Layer {
    Layer::Hush {
        level,
        air,
        low: Pair::new(cutoff, 0.6),
        high: Pair::new(3500.0, 0.5),
    }
}

fn wind(level: f32, band: f32, loft: f32) -> Layer {
    Layer::Wind {
        level,
        band,
        loft,
        gust: Wander::new(0.15),
        f: Pair::new(band, 0.8),
    }
}

fn scatter(sound: Sound, every: f32, gain: (f32, f32), spread: f32) -> Layer {
    Layer::Scatter {
        sound,
        every,
        gain,
        spread,
        cluster: (1, 1),
        spacing: 0.0,
        wait: every * 0.5,
    }
}

/// `scatter`, a few at a time.
fn clusters(sound: Sound, every: f32, gain: (f32, f32), spread: f32, n: (usize, usize), spacing: f32) -> Layer {
    Layer::Scatter {
        sound,
        every,
        gain,
        spread,
        cluster: n,
        spacing,
        wait: every * 0.5,
    }
}

fn beat(grains: [Grain; 2], period: f32, gain: f32, pan: f32) -> Layer {
    Layer::Beat {
        grains,
        period,
        gain,
        pan,
        wait: period * 0.5,
        n: 0,
    }
}

/// A place's sound and its acoustics.
struct Bed {
    layers: Vec<Layer>,
    /// Reverb decay (seconds) and damping.
    rt60: f32,
    damp: f32,
}

impl Bed {
    fn of(place: Place) -> Bed {
        use Grain::*;
        use Ring::{Chime, Crystal, Glint, Strike, String};
        let g = Sound::Grain;
        let r = Sound::Ring;
        let (layers, rt60, damp) = match place {
            Place::Hall => (
                vec![hush(0.07, 240.0, 0.12), scatter(g(Creak), 50.0, (0.05, 0.09), 0.7)],
                1.5,
                0.45,
            ),
            Place::Atrium => (
                vec![
                    hush(0.075, 190.0, 0.2),
                    wind(0.03, 520.0, 2.5),
                    scatter(g(Creak), 70.0, (0.04, 0.07), 0.8),
                ],
                2.3,
                0.3,
            ),
            Place::Observatory => (
                vec![
                    hush(0.065, 260.0, 0.18),
                    wind(0.02, 700.0, 0.0),
                    scatter(r(Glint), 5.0, (0.006, 0.015), 0.8),
                    scatter(g(Owl), 140.0, (0.035, 0.05), 0.5),
                ],
                1.3,
                0.4,
            ),
            Place::AlchemicalWing => (
                vec![
                    hush(0.065, 230.0, 0.1),
                    clusters(g(Bubble), 2.2, (0.012, 0.04), 0.35, (2, 7), 0.09),
                ],
                1.1,
                0.45,
            ),
            Place::Herbarium => (
                vec![
                    hush(0.06, 300.0, 0.15),
                    clusters(g(Cricket), 9.0, (0.006, 0.012), 0.6, (3, 7), 0.55),
                    scatter(g(Drip), 6.0, (0.02, 0.04), 0.5),
                ],
                1.0,
                0.5,
            ),
            Place::Bestiary => (
                vec![
                    hush(0.07, 170.0, 0.08),
                    beat([Heart, Heart], 1.7, 0.025, 0.1),
                    scatter(g(Creak), 30.0, (0.025, 0.05), 0.6),
                ],
                1.2,
                0.4,
            ),
            Place::MapRoom => (
                vec![
                    hush(0.06, 250.0, 0.12),
                    Layer::Sea {
                        level: 0.03,
                        t: 0.0,
                        period: 9.0,
                        body: Pair::new(420.0, 0.6),
                        foam: Pair::new(2200.0, 0.7),
                    },
                    scatter(g(Creak), 25.0, (0.015, 0.035), 0.6),
                ],
                1.1,
                0.45,
            ),
            Place::ChronicleHall => (
                vec![
                    hush(0.065, 230.0, 0.1),
                    scatter(g(Quill), 9.0, (0.02, 0.04), 0.8),
                    scatter(g(Page), 40.0, (0.02, 0.035), 0.6),
                ],
                1.4,
                0.4,
            ),
            Place::VerseGallery => (
                vec![
                    hush(0.06, 290.0, 0.1),
                    scatter(g(Page), 12.0, (0.02, 0.04), 0.7),
                    scatter(r(Glint), 9.0, (0.004, 0.009), 0.6),
                ],
                1.0,
                0.55,
            ),
            Place::Arcanum => (
                vec![
                    hush(0.055, 210.0, 0.1),
                    Layer::Hum {
                        level: 0.003,
                        phases: [0.0; 6],
                    },
                    scatter(r(Crystal), 6.0, (0.006, 0.015), 0.6),
                ],
                1.6,
                0.3,
            ),
            Place::Clockworks => (
                vec![
                    hush(0.06, 250.0, 0.1),
                    beat([Tick, Tock], 1.0, 0.05, 0.25),
                    beat([Tick, Tick], 0.37, 0.01, -0.5),
                    scatter(r(Strike), 240.0, (0.02, 0.025), 0.3),
                ],
                1.2,
                0.4,
            ),
            Place::MusicRoom => (
                vec![hush(0.06, 260.0, 0.12), scatter(r(String), 4.5, (0.004, 0.01), 0.5)],
                1.7,
                0.35,
            ),
            Place::Weatherhouse => (
                vec![
                    hush(0.05, 250.0, 0.1),
                    Layer::Wash {
                        level: 0.014,
                        f: Pair::new(2500.0, 0.5),
                        gust: Wander::new(0.1),
                    },
                    scatter(g(Drop), 1.0 / 35.0, (0.004, 0.02), 0.8),
                    clusters(r(Chime), 18.0, (0.004, 0.01), 0.5, (2, 4), 0.3),
                    scatter(g(Rumble), 50.0, (0.04, 0.07), 0.4),
                ],
                1.1,
                0.45,
            ),
            Place::HearthLibrary => (
                vec![hush(0.06, 230.0, 0.08), scatter(g(Knit), 30.0, (0.015, 0.03), 0.4)],
                0.8,
                0.6,
            ),
            Place::Room => (vec![hush(0.07, 250.0, 0.1)], 1.1, 0.45),
        };
        Bed { layers, rt60, damp }
    }
}

/// What a layer renders with: the block's ramps and where to put it.
struct Ctx<'a> {
    /// The place's weight at the block's start and end.
    w: (f32, f32),
    loft: f32,
    /// Playback rate putting D at the music's key.
    shift: f32,
    banks: Option<&'a Banks>,
    noise: &'a mut Noise,
    rng: &'a mut StdRng,
    voices: &'a mut Vec<Voice>,
    rings: &'a mut Vec<Ringing>,
    l: &'a mut [f32],
    r: &'a mut [f32],
    wet: &'a mut [f32],
}

impl Ctx<'_> {
    fn ramp(&self, i: usize) -> f32 {
        self.w.0 + (self.w.1 - self.w.0) * (i + 1) as f32 / self.l.len() as f32
    }

    /// Start `sound` `delay` seconds from now.
    fn start(&mut self, sound: Sound, gain: f32, pan: f32, delay: f32, send: f32) {
        let wait = (delay * SRF) as usize;
        match sound {
            Sound::Grain(g) => {
                let Some(banks) = self.banks else { return };
                let clip = banks.grain(g).choose(self.rng).unwrap().clone();
                let rate = self.rng.gen_range(0.96..1.04);
                self.voices
                    .push(Voice::new(clip, gain, rate, send, None, pan).after(wait));
            }
            Sound::Ring(ring) => {
                let (partials, attack, tremolo) = ring.tone(self.rng.gen_range(0..10), self.shift);
                let (l, r) = pan_gains(pan);
                self.rings.push(Ringing {
                    partials,
                    wait,
                    env: 0.0,
                    attack: 1.0 / (attack * SRF),
                    tremolo,
                    phase: self.rng.gen_range(0.0..1.0),
                    gains: (l * gain, r * gain),
                    send: send * gain,
                });
            }
        }
    }
}

impl Layer {
    fn render(&mut self, c: &mut Ctx) {
        let n = c.l.len();
        match self {
            Layer::Hush { level, air, low, high } => {
                for i in 0..n {
                    let g = c.ramp(i) * *level;
                    let (a, b) = (c.noise.white(), c.noise.white());
                    let (x, y) = (c.noise.white(), c.noise.white());
                    c.l[i] += (low.0[0].low(a) + high.0[0].band(x) * *air) * g;
                    c.r[i] += (low.0[1].low(b) + high.0[1].band(y) * *air) * g;
                }
            }
            Layer::Wind {
                level,
                band,
                loft,
                gust,
                f,
            } => {
                let (g0, g1) = gust.step(c.rng);
                let lift = 1.0 + *loft * c.loft;
                f.set(*band * (0.7 + 0.8 * g1), 0.8);
                for i in 0..n {
                    let k = (i + 1) as f32 / n as f32;
                    let g = c.ramp(i) * *level * lift * (0.25 + (g0 + (g1 - g0) * k).powi(2));
                    c.l[i] += f.0[0].band(c.noise.white()) * g;
                    c.r[i] += f.0[1].band(c.noise.white()) * g;
                }
            }
            Layer::Sea {
                level,
                t,
                period,
                body,
                foam,
            } => {
                for i in 0..n {
                    *t += 1.0 / SRF;
                    if *t >= *period {
                        *t = 0.0;
                        *period = c.rng.gen_range(7.0..12.0);
                    }
                    let u = *t / *period;
                    let e = if u < 0.4 {
                        (PI * 0.5 * u / 0.4).sin()
                    } else {
                        ((1.0 - u) / 0.6).powf(1.5)
                    };
                    let g = c.ramp(i) * *level;
                    let (a, b) = (c.noise.white(), c.noise.white());
                    c.l[i] += (body.0[0].low(a) * e + foam.0[0].band(b) * e.powi(3) * 0.3) * g;
                    c.r[i] += (body.0[1].low(b) * e + foam.0[1].band(a) * e.powi(3) * 0.3) * g;
                }
            }
            Layer::Wash { level, f, gust } => {
                let (g0, g1) = gust.step(c.rng);
                for i in 0..n {
                    let k = (i + 1) as f32 / n as f32;
                    let g = c.ramp(i) * *level * (0.6 + 0.4 * (g0 + (g1 - g0) * k));
                    c.l[i] += f.0[0].band(c.noise.white()) * g;
                    c.r[i] += f.0[1].band(c.noise.white()) * g;
                }
            }
            Layer::Hum { level, phases } => {
                // D2 and its fifth and octave, each beside a twin a fraction off.
                let base = midi_hz(38.0) * c.shift;
                let tones = [(1.0, 1.0), (1.0, 0.9), (1.5, 0.35), (1.5, 0.3), (2.0, 0.25), (2.0, 0.2)];
                let beats = [0.0, 0.13, 0.0, 0.21, 0.0, 0.17];
                for i in 0..n {
                    let mut s = 0.0;
                    for (k, ((ratio, a), beat)) in tones.iter().zip(beats).enumerate() {
                        phases[k] = (phases[k] + (base * ratio + beat) / SRF).fract();
                        s += a * (TAU * phases[k]).sin();
                    }
                    let g = c.ramp(i) * *level * s;
                    c.l[i] += g;
                    c.r[i] += g;
                }
            }
            Layer::Scatter {
                sound,
                every,
                gain,
                spread,
                cluster,
                spacing,
                wait,
            } => {
                let w = c.w.1;
                while *wait < DT {
                    let count = c.rng.gen_range(cluster.0..=cluster.1);
                    let pan = c.rng.gen_range(-1.0..=1.0) * *spread;
                    for k in 0..count {
                        let a = c.rng.gen_range(gain.0..=gain.1) * w;
                        let jitter = c.rng.gen_range(0.6..1.4);
                        let at = wait.max(0.0) + k as f32 * *spacing * jitter;
                        let pan = (pan + c.rng.gen_range(-0.15..=0.15)).clamp(-1.0, 1.0);
                        if a > 1e-4 {
                            c.start(*sound, a, pan, at, 0.4);
                        }
                    }
                    *wait += -(1.0 - c.rng.gen_range(0.0..1.0f32)).ln() * *every;
                }
                *wait -= DT;
            }
            Layer::Beat {
                grains,
                period,
                gain,
                pan,
                wait,
                n: count,
            } => {
                let w = c.w.1;
                while *wait < DT {
                    let g = grains[*count % 2];
                    *count += 1;
                    let a = *gain * w * c.rng.gen_range(0.85..1.0);
                    if a > 1e-4 {
                        c.start(Sound::Grain(g), a, *pan, wait.max(0.0), 0.3);
                    }
                    *wait += *period * c.rng.gen_range(0.99..1.01);
                }
                *wait -= DT;
            }
        }
    }
}

/// A fire: the low roar and hiss of its flames, crackles about it, a pop now and
/// then; heard from where it burns.
struct Fire {
    level: Smooth,
    at: Vec3,
    roar: Svf,
    hiss: Svf,
    lp: OnePole,
    flicker: Wander,
    crackle: f32,
    pop: f32,
    gains: Option<(f32, f32)>,
}

impl Fire {
    fn new() -> Self {
        Fire {
            level: Smooth::new(0.0),
            at: Vec3::ZERO,
            roar: Svf::new(260.0, 0.6),
            hiss: Svf::new(1500.0, 0.7),
            lp: OnePole::new(20000.0),
            flicker: Wander::new(0.8),
            crackle: 0.0,
            pop: 3.0,
            gains: None,
        }
    }

    fn render(&mut self, ears: &Ears, c: &mut Ctx) {
        let (v0, v1) = self.level.step(DT, 0.6);
        if v0 < 1e-4 && v1 < 1e-4 {
            self.gains = None;
            return;
        }
        let h = Heard::from(ears, self.at);
        let (gl0, gr0) = self.gains.unwrap_or((h.left, h.right));
        self.gains = Some((h.left, h.right));
        self.lp.set(h.cutoff.min(20000.0));
        let (f0, f1) = self.flicker.step(c.rng);
        let n = c.l.len();
        for i in 0..n {
            let k = (i + 1) as f32 / n as f32;
            let v = (v0 + (v1 - v0) * k) * 0.09;
            let fl = 0.65 + 0.35 * (f0 + (f1 - f0) * k);
            let x = c.noise.white();
            let s = self
                .lp
                .lp((self.roar.low(x) + self.hiss.band(c.noise.white()) * 0.12) * v * fl);
            c.l[i] += s * (gl0 + (h.left - gl0) * k);
            c.r[i] += s * (gr0 + (h.right - gr0) * k);
            c.wet[i] += s * 0.3 * h.wet;
        }
        let Some(banks) = c.banks else { return };
        // Crackles come in flurries as the flames flare.
        while self.crackle < DT {
            let at = self.at
                + Vec3::new(
                    c.rng.gen_range(-0.4..0.4),
                    c.rng.gen_range(0.0..0.4),
                    c.rng.gen_range(-0.3..0.3),
                );
            let clip = banks.grain(Grain::Crackle).choose(c.rng).unwrap().clone();
            let a = v1 * c.rng.gen_range(0.01..0.05f32) * (0.4 + f1);
            let v = Voice::new(clip, a, c.rng.gen_range(0.9..1.1), 0.25, Some(at), 0.0);
            c.voices.push(v.after((self.crackle.max(0.0) * SRF) as usize));
            self.crackle += -(1.0 - c.rng.gen_range(0.0..1.0f32)).ln() / (2.0 + 14.0 * f1 * f1);
        }
        self.crackle -= DT;
        self.pop -= DT;
        if self.pop <= 0.0 {
            let clip = banks.grain(Grain::Pop).choose(c.rng).unwrap().clone();
            let v = Voice::new(
                clip,
                v1 * c.rng.gen_range(0.04..0.08),
                c.rng.gen_range(0.9..1.1),
                0.3,
                Some(self.at),
                0.0,
            );
            c.voices.push(v);
            self.pop = c.rng.gen_range(4.0..16.0);
        }
    }
}

/// A paper familiar's wings, beating as it flies.
struct Wings {
    level: Smooth,
    at: Vec3,
    band: Svf,
    phase: f32,
    rate: f32,
    beat: f32,
    gains: Option<(f32, f32)>,
}

impl Wings {
    fn new() -> Self {
        Wings {
            level: Smooth::new(0.0),
            at: Vec3::ZERO,
            band: Svf::new(1400.0, 0.8),
            phase: 0.0,
            rate: 7.0,
            beat: 1.0,
            gains: None,
        }
    }

    fn render(&mut self, ears: &Ears, c: &mut Ctx) {
        let (v0, v1) = self.level.step(DT, 0.25);
        if v0 < 1e-4 && v1 < 1e-4 {
            self.gains = None;
            return;
        }
        let h = Heard::from(ears, self.at);
        let (gl0, gr0) = self.gains.unwrap_or((h.left, h.right));
        self.gains = Some((h.left, h.right));
        let n = c.l.len();
        for i in 0..n {
            let k = (i + 1) as f32 / n as f32;
            self.phase += self.rate / SRF;
            if self.phase >= 1.0 {
                self.phase -= 1.0;
                self.rate = c.rng.gen_range(6.0..8.5);
                self.beat = c.rng.gen_range(0.6..1.0);
            }
            // The downstroke whispers; the upstroke is nearly silent.
            let stroke = if self.phase < 0.55 {
                (PI * self.phase / 0.55).sin().powi(3)
            } else {
                0.0
            };
            let v = (v0 + (v1 - v0) * k) * 0.1 * stroke * self.beat;
            let s = self.band.band(c.noise.white()) * v;
            c.l[i] += s * (gl0 + (h.left - gl0) * k);
            c.r[i] += s * (gr0 + (h.right - gr0) * k);
            c.wet[i] += s * 0.2 * h.wet;
        }
    }
}

/// Every place's bed, the fires and the wings, and how much of each is heard.
pub struct Ambience {
    beds: Vec<Bed>,
    rings: Vec<Ringing>,
    weights: Vec<Smooth>,
    loft: Smooth,
    fires: Vec<Fire>,
    wings: Vec<Wings>,
    noise: Noise,
}

impl Ambience {
    pub fn new(seed: u64) -> Self {
        Ambience {
            beds: Place::ALL.iter().map(|&p| Bed::of(p)).collect(),
            rings: Vec::new(),
            weights: vec![Smooth::new(0.0); Place::ALL.len()],
            loft: Smooth::new(0.0),
            fires: (0..MAX_FIRES).map(|_| Fire::new()).collect(),
            wings: (0..MAX_WINGS).map(|_| Wings::new()).collect(),
            noise: Noise::new(seed as u32 ^ 0xa3b1),
        }
    }

    /// The room's reverb: each place's, by how much of it is heard.
    pub fn acoustics(&self) -> (f32, f32) {
        let total: f32 = self.weights.iter().map(|w| w.value).sum();
        if total < 1e-3 {
            return (1.1, 0.45);
        }
        let (rt, damp) = (self.beds.iter().zip(&self.weights)).fold((0.0, 0.0), |(rt, d), (b, w)| {
            (rt + b.rt60 * w.value, d + b.damp * w.value)
        });
        (rt / total, damp / total)
    }

    /// Add a block of ambience to `out` (the dry pair and the room's send), and
    /// start its grains among `voices`. `shift` is the rate putting D at the
    /// music's key.
    pub fn render(
        &mut self,
        scene: &Scene,
        banks: Option<&Banks>,
        shift: f32,
        rng: &mut StdRng,
        voices: &mut Vec<Voice>,
        out: &mut [Vec<f32>; 3],
    ) {
        let ears = Ears::new(scene.ears, scene.right);
        let [l, r, wet] = out;
        self.loft.target = scene.loft;
        let (_, loft) = self.loft.step(DT, 2.0);
        let mut c = Ctx {
            w: (0.0, 0.0),
            loft,
            shift,
            banks,
            noise: &mut self.noise,
            rng,
            voices,
            rings: &mut self.rings,
            l,
            r,
            wet,
        };
        for (i, bed) in self.beds.iter_mut().enumerate() {
            let w = &mut self.weights[i];
            w.target = scene.places[i];
            c.w = w.step(DT, 0.8);
            if c.w.0 > 1e-4 || c.w.1 > 1e-4 {
                for layer in &mut bed.layers {
                    layer.render(&mut c);
                }
            }
        }
        for (fire, &(at, level)) in self.fires.iter_mut().zip(&scene.fires) {
            // A fire going out of hearing fades from where it was.
            if level > 0.0 {
                fire.at = at;
            }
            fire.level.target = level;
            fire.render(&ears, &mut c);
        }
        for (wings, at) in self.wings.iter_mut().zip(&scene.wings) {
            match at {
                Some(at) => {
                    wings.at = *at;
                    wings.level.target = 1.0;
                }
                None => wings.level.target = 0.0,
            }
            wings.render(&ears, &mut c);
        }
        c.rings.retain_mut(|ring| ring.render(c.l, c.r, c.wet));
    }
}
