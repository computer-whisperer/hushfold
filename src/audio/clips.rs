//! The clips: each effect's bank of variants and the short grains the ambience is
//! built from, all synthesised at startup. Tonal clips are composed in D (major
//! pentatonic) and transposed to the music's key as they play.

use super::Sfx;
use super::dsp::*;
use rand::prelude::*;
use std::f32::consts::{PI, TAU};
use std::sync::Arc;

pub type Clip = Arc<[f32]>;

/// The ambience's grains: short sounds the places scatter about.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Grain {
    /// A fire's tiny crackles.
    Crackle,
    /// A log settling, an ember popping.
    Pop,
    /// A raindrop on the glass.
    Drop,
    /// A drip into a water butt.
    Drip,
    /// An alembic bubbling.
    Bubble,
    /// The great clock's escapement, one way and the other.
    Tick,
    Tock,
    /// A self-writing quill, a few words at a time.
    Quill,
    /// A page turning, a paper crane stirring.
    Page,
    /// Knitting needles at work.
    Knit,
    /// Thunder in a jar.
    Rumble,
    /// Old timber or a chain creaking.
    Creak,
    /// A slow heartbeat.
    Heart,
    /// A cricket outside.
    Cricket,
    /// An owl, far off.
    Owl,
}

pub const GRAINS: [Grain; 15] = [
    Grain::Crackle,
    Grain::Pop,
    Grain::Drop,
    Grain::Drip,
    Grain::Bubble,
    Grain::Tick,
    Grain::Tock,
    Grain::Quill,
    Grain::Page,
    Grain::Knit,
    Grain::Rumble,
    Grain::Creak,
    Grain::Heart,
    Grain::Cricket,
    Grain::Owl,
];

/// Every clip, by effect and by grain.
pub struct Banks {
    pub sfx: Vec<Vec<Clip>>,
    pub grains: Vec<Vec<Clip>>,
}

impl Banks {
    pub fn synthesize() -> Self {
        let mut rng = StdRng::seed_from_u64(0x5348_5553);
        let mut r = || StdRng::seed_from_u64(rng.next_u64());
        let sfx = Sfx::ALL
            .iter()
            .map(|&s| {
                let mut rng = r();
                let bank: Vec<Vec<f32>> = match s {
                    Sfx::Thunk => (0..16).map(|i| thunk(&mut rng, i as f32 / 15.0)).collect(),
                    Sfx::Rustle => (0..10).map(|_| rustle(&mut rng)).collect(),
                    Sfx::Whoosh => (0..8).map(|_| whoosh(&mut rng)).collect(),
                    Sfx::Toss => (0..6).map(|_| toss(&mut rng)).collect(),
                    Sfx::Land => (0..10).map(|_| land(&mut rng)).collect(),
                    Sfx::Chime => (0..13).map(chime).collect(),
                    Sfx::Fanfare => (0..4).map(fanfare).collect(),
                    Sfx::Veil => vec![veil(&mut rng)],
                    Sfx::Purchase => (0..3).map(|v| purchase(&mut rng, v)).collect(),
                    Sfx::Spell => (0..5).map(|k| spell(&mut rng, k)).collect(),
                    Sfx::Unfold => (0..3).map(|_| unfold(&mut rng)).collect(),
                    Sfx::Fold => (0..3).map(|_| fold(&mut rng)).collect(),
                    Sfx::Nope => (0..2).map(|_| nope(&mut rng)).collect(),
                    Sfx::Tick => (0..4).map(|_| tick(&mut rng)).collect(),
                    Sfx::Step => (0..16).map(|_| step(&mut rng)).collect(),
                    Sfx::Hop => (0..6).map(|_| hop(&mut rng)).collect(),
                    Sfx::Alight => (0..6).map(|_| alight(&mut rng)).collect(),
                };
                bank.into_iter().map(Clip::from).collect()
            })
            .collect();
        let grains = GRAINS
            .iter()
            .map(|&g| {
                let mut rng = r();
                let bank: Vec<Vec<f32>> = match g {
                    Grain::Crackle => (0..24).map(|_| crackle(&mut rng)).collect(),
                    Grain::Pop => (0..6).map(|_| pop(&mut rng)).collect(),
                    Grain::Drop => (0..16).map(|_| raindrop(&mut rng)).collect(),
                    Grain::Drip => (0..6).map(|_| drip(&mut rng)).collect(),
                    Grain::Bubble => (0..12).map(|_| bubble(&mut rng)).collect(),
                    Grain::Tick => (0..3).map(|_| escapement(&mut rng, 1.0)).collect(),
                    Grain::Tock => (0..3).map(|_| escapement(&mut rng, 0.84)).collect(),
                    Grain::Quill => (0..6).map(|_| quill(&mut rng)).collect(),
                    Grain::Page => (0..6).map(|_| page(&mut rng)).collect(),
                    Grain::Knit => (0..6).map(|_| knit(&mut rng)).collect(),
                    Grain::Rumble => (0..3).map(|_| rumble(&mut rng)).collect(),
                    Grain::Creak => (0..6).map(|_| creak(&mut rng)).collect(),
                    Grain::Heart => (0..3).map(|_| heart(&mut rng)).collect(),
                    Grain::Cricket => (0..6).map(|_| cricket(&mut rng)).collect(),
                    Grain::Owl => (0..3).map(|_| owl(&mut rng)).collect(),
                };
                bank.into_iter().map(Clip::from).collect()
            })
            .collect();
        Banks { sfx, grains }
    }

    pub fn grain(&self, g: Grain) -> &[Clip] {
        &self.grains[g as usize]
    }
}

// ---------------------------------------------------------------- pitch

/// Add a struck or plucked tone at `start`: partials as (ratio, amplitude, decay in
/// seconds), eased in over 2 ms.
fn tone(out: &mut [f32], start: usize, freq: f32, amp: f32, partials: &[(f32, f32, f32)]) {
    let mut t = vec![0.0; out.len() - start];
    for &(ratio, a, tau) in partials {
        if freq * ratio < SRF * 0.45 {
            mode(&mut t, 0, freq * ratio, a * amp, tau);
        }
    }
    let ramp = secs(0.002) as f32;
    for (i, (o, x)) in out[start..].iter_mut().zip(&t).enumerate() {
        *o += x * (i as f32 / ramp).min(1.0);
    }
}

/// A celesta: mostly fundamental, a little octave.
fn celesta(out: &mut [f32], start: usize, note: f32, amp: f32, tau: f32) {
    let f = midi_hz(note);
    tone(
        out,
        start,
        f,
        amp,
        &[(1.0, 1.0, tau), (2.0, 0.12, tau * 0.35), (3.0, 0.03, tau * 0.2)],
    );
}

/// A harp string: harmonics shaped by where it is plucked, upper ones dying first.
fn harp(out: &mut [f32], start: usize, note: f32, amp: f32, tau: f32) {
    let f = midi_hz(note);
    let parts: Vec<(f32, f32, f32)> = (1..=8)
        .map(|n| {
            let n = n as f32;
            (
                n,
                (n * PI * 0.17).sin().abs() / n.powf(1.2),
                tau / (1.0 + 0.45 * (n - 1.0)),
            )
        })
        .collect();
    tone(out, start, f, amp, &parts);
}

/// A bell: inharmonic partials, a long hum.
fn bell(out: &mut [f32], start: usize, note: f32, amp: f32, tau: f32) {
    let f = midi_hz(note);
    tone(
        out,
        start,
        f,
        amp,
        &[
            (1.0, 1.0, tau),
            (2.0, 0.4, tau * 0.6),
            (2.76, 0.22, tau * 0.4),
            (5.4, 0.06, tau * 0.15),
        ],
    );
}

// ---------------------------------------------------------------- shared shapes

/// Envelope: linear rise over `a` seconds, then decay with time constant `d`.
fn ar(t: f32, a: f32, d: f32) -> f32 {
    if t < a { t / a } else { (-(t - a) / d).exp() }
}

/// Filtered noise shaped by an envelope, added at `start`.
fn burst(
    out: &mut [f32],
    rng: &mut StdRng,
    start: usize,
    len: f32,
    band: (f32, f32),
    amp: f32,
    mut env: impl FnMut(f32) -> f32,
) {
    let n = secs(len).min(out.len().saturating_sub(start));
    let mut b = noise(n, rng.next_u32());
    highpass(&mut b, band.0);
    lowpass2(&mut b, band.1);
    for (i, x) in b.iter().enumerate() {
        out[start + i] += x * amp * env(i as f32 / SRF);
    }
}

/// Paper: a scatter of tiny crinkles over `len` seconds, `rate` per second.
fn crinkles(out: &mut [f32], rng: &mut StdRng, start: usize, len: f32, rate: f32, amp: f32, band: (f32, f32)) {
    let mut t = 0.0;
    while t < len {
        t += -(1.0 - rng.gen_range(0.0..1.0f32)).ln() / rate;
        let at = start + secs(t);
        if at >= out.len() {
            break;
        }
        let a = amp * rng.gen_range(0.2..1.0f32).powi(2) * (PI * (t / len).min(1.0)).sin().max(0.15);
        let f = rng.gen_range(band.0..band.1);
        let d = rng.gen_range(0.0015..0.005);
        burst(out, rng, at, d * 4.0, (f * 0.6, f * 1.4), a, |x| ar(x, 0.0004, d));
    }
}

// ---------------------------------------------------------------- effects

/// A book slid home: a short slide along the board, then a soft wooden knock as it
/// meets the back; `size` 0..1 runs from slim to stout (stouter is lower).
fn thunk(rng: &mut StdRng, size: f32) -> Vec<f32> {
    let mut out = vec![0.0; secs(0.5)];
    let slide = rng.gen_range(0.03..0.07);
    burst(&mut out, rng, 0, slide * 1.2, (300.0, 1800.0), 0.12, |t| {
        (t / (slide * 0.7)).min(1.0) * ((slide * 1.2 - t) / (slide * 0.5)).clamp(0.0, 1.0)
    });
    let hit = secs(slide);
    let low = 1.12 - 0.3 * size;
    let mut knock = vec![0.0; out.len() - hit];
    mode(&mut knock, 0, rng.gen_range(88.0..104.0) * low, 0.9, 0.032);
    let board = rng.gen_range(185.0..250.0) * low;
    mode(&mut knock, 0, board, 0.5, rng.gen_range(0.04..0.06));
    mode(&mut knock, 0, board * 2.31, 0.18, 0.025);
    mode(&mut knock, 0, rng.gen_range(500.0..720.0) * low, 0.22, 0.016);
    burst(&mut knock, rng, 0, 0.012, (200.0, 2600.0), 0.3, |t| {
        ar(t, 0.0003, 0.0025)
    });
    // Some books settle with a second, smaller tap.
    if rng.gen_bool(0.5) {
        let at = secs(rng.gen_range(0.025..0.05));
        mode(&mut knock, at, board * 1.1, 0.16, 0.025);
        mode(&mut knock, at, 95.0 * low, 0.2, 0.02);
    }
    lowpass2(&mut knock, 3200.0);
    for (o, k) in out[hit..].iter_mut().zip(&knock) {
        *o += k;
    }
    finish(out, 1.0)
}

/// Books gathered up: a rustle of pages and covers, and a soft knock or two as they
/// stack.
fn rustle(rng: &mut StdRng) -> Vec<f32> {
    let len = rng.gen_range(0.26..0.36);
    let mut out = vec![0.0; secs(len + 0.15)];
    let rate = rng.gen_range(110.0..170.0);
    crinkles(&mut out, rng, 0, len, rate, 0.6, (1400.0, 4200.0));
    burst(&mut out, rng, 0, len, (250.0, 1100.0), 0.22, |t| {
        (PI * t / len).sin().max(0.0).powf(1.5)
    });
    for _ in 0..rng.gen_range(1..=2) {
        let at = secs(rng.gen_range(0.05..len * 0.8));
        mode(&mut out, at, rng.gen_range(330.0..480.0), 0.35, 0.014);
        mode(&mut out, at, rng.gen_range(140.0..180.0), 0.4, 0.025);
    }
    lowpass2(&mut out, 6000.0);
    finish(out, 1.0)
}

/// Books gliding off to their shelves: a soft rush of air.
fn whoosh(rng: &mut StdRng) -> Vec<f32> {
    let len = rng.gen_range(0.28..0.38);
    let n = secs(len);
    let (f0, f1) = (rng.gen_range(300.0..420.0), rng.gen_range(1100.0..1600.0));
    let peak_at = rng.gen_range(0.35..0.5);
    let mut g = Noise::new(rng.next_u32());
    let mut bp = Svf::new(f0, 1.1);
    let mut body = OnePole::new(450.0);
    let out: Vec<f32> = (0..n)
        .map(|i| {
            let t = i as f32 / n as f32;
            // A bell-shaped swell, skewed to peak at `peak_at`.
            let u = if t < peak_at {
                t / peak_at * 0.5
            } else {
                0.5 + (t - peak_at) / (1.0 - peak_at) * 0.5
            };
            let e = (PI * u).sin().powf(1.6);
            bp.set(f0 + (f1 - f0) * e, 1.1);
            let x = g.white();
            (bp.band(x) + body.lp(x) * 0.5) * e
        })
        .collect();
    finish(out, 1.0)
}

/// Books thrown: a fuller rush of air, the pages fluttering as they go.
fn toss(rng: &mut StdRng) -> Vec<f32> {
    let n = secs(rng.gen_range(0.42..0.52) + 0.5);
    let mut g = Noise::new(rng.next_u32());
    let mut bp = Svf::new(300.0, 0.9);
    let mut flut = Svf::new(1900.0, 1.2);
    let rate = rng.gen_range(20.0..28.0);
    let out: Vec<f32> = (0..n)
        .map(|i| {
            let t = i as f32 / SRF;
            let e = ar(t, 0.07, 0.11);
            bp.set(260.0 + 700.0 * e, 0.9);
            let flap = (0.5 + 0.5 * (TAU * rate * t).sin()).powi(3) * ar(t, 0.05, 0.16);
            bp.band(g.white()) * e + flut.band(g.white()) * flap * 0.35
        })
        .collect();
    finish(out, 1.0)
}

/// A tossed book coming down: a papery slap over a dull thud, sometimes a tip.
fn land(rng: &mut StdRng) -> Vec<f32> {
    let mut out = vec![0.0; secs(0.5)];
    burst(&mut out, rng, 0, 0.03, (500.0, 3200.0), 0.55, |t| ar(t, 0.0004, 0.005));
    mode(&mut out, 0, rng.gen_range(92.0..125.0), 1.0, 0.04);
    mode(&mut out, 0, rng.gen_range(220.0..300.0), 0.4, 0.025);
    let flap = secs(rng.gen_range(0.01..0.03));
    burst(&mut out, rng, flap, 0.04, (1500.0, 3500.0), 0.12, |t| {
        ar(t, 0.003, 0.012)
    });
    if rng.gen_bool(0.4) {
        let at = secs(rng.gen_range(0.06..0.1));
        mode(&mut out, at, rng.gen_range(110.0..140.0), 0.35, 0.03);
        burst(&mut out, rng, at, 0.02, (500.0, 2500.0), 0.15, |t| ar(t, 0.0004, 0.004));
    }
    lowpass2(&mut out, 4500.0);
    finish(out, 1.0)
}

/// A series completed: a little rising run on the celesta, longer for a longer
/// series.
fn chime(step: usize) -> Vec<f32> {
    let notes = 2 + (step / 3).min(2) as i32;
    let first = (step as i32 * 2) % 5;
    let mut out = vec![0.0; secs(4.5)];
    for k in 0..notes {
        let amp = 1.0 - k as f32 * 0.12;
        celesta(&mut out, secs(k as f32 * 0.075), penta(5, first + k), amp, 0.55);
    }
    // The last note glints an octave up, faintly.
    let last = penta(6, first + notes - 1);
    celesta(&mut out, secs((notes - 1) as f32 * 0.075), last, 0.06, 0.3);
    finish(out, 1.0)
}

/// A category fully catalogued: a harp rising through D major under a warm swell,
/// a celesta answering at the top.
fn fanfare(variant: usize) -> Vec<f32> {
    let mut out = vec![0.0; secs(8.0)];
    let runs: [&[f32]; 4] = [
        &[50.0, 54.0, 57.0, 62.0, 66.0, 69.0, 74.0],
        &[50.0, 57.0, 62.0, 64.0, 66.0, 69.0, 74.0],
        &[45.0, 50.0, 54.0, 57.0, 62.0, 66.0, 69.0],
        &[50.0, 54.0, 57.0, 59.0, 62.0, 66.0, 71.0],
    ];
    let run = runs[variant % runs.len()];
    let gap = 0.095;
    for (k, &n) in run.iter().enumerate() {
        harp(&mut out, secs(k as f32 * gap), n, 0.5 + k as f32 * 0.04, 1.4);
    }
    let top = secs(run.len() as f32 * gap + 0.05);
    let crown = run[run.len() - 1];
    celesta(&mut out, top, crown + 12.0, 0.4, 1.0);
    celesta(&mut out, top + secs(0.11), crown + 7.0, 0.25, 0.9);
    // The swell: a soft chord rising in and dying away.
    for (note, a) in [(38.0, 0.25), (45.0, 0.18), (54.0, 0.14), (57.0, 0.1)] {
        let f = midi_hz(note);
        for (i, o) in out.iter_mut().enumerate() {
            let t = i as f32 / SRF;
            let e = (t / 0.5).min(1.0) * (-(t - 0.5).max(0.0) / 1.1).exp();
            *o += a * e * ((TAU * f * t).sin() + 0.25 * (TAU * 2.0 * f * t).sin());
        }
    }
    finish(out, 1.0)
}

/// A department's veil lifting: a warm low swell, air rising through it, and a
/// sprinkle of glints growing thicker as it goes.
fn veil(rng: &mut StdRng) -> Vec<f32> {
    let mut out = vec![0.0; secs(8.0)];
    for (note, a) in [(38.0, 0.3), (45.0, 0.2), (50.0, 0.12)] {
        let f = midi_hz(note);
        for (i, o) in out.iter_mut().enumerate() {
            let t = i as f32 / SRF;
            let e = (t / 1.2).min(1.0).powi(2) * (-(t - 1.2).max(0.0) / 1.2).exp();
            *o += a * e * ((TAU * f * t).sin() + 0.2 * (TAU * 3.0 * f * t).sin());
        }
    }
    let mut g = Noise::new(rng.next_u32());
    let mut bp = Svf::new(400.0, 1.4);
    for (i, o) in out.iter_mut().enumerate() {
        let t = i as f32 / SRF;
        let e = (t / 1.5).min(1.0) * (-(t - 1.5).max(0.0) / 0.9).exp();
        bp.set(400.0 + 2600.0 * (t / 2.5).min(1.0), 1.4);
        *o += bp.band(g.white()) * e * 0.18;
    }
    let mut t = 0.2f32;
    while t < 3.0 {
        let d = rng.gen_range(0..8);
        celesta(&mut out, secs(t), penta(6, d), rng.gen_range(0.06..0.14), 0.5);
        t += rng.gen_range(0.04..0.22) * (1.2 - t / 3.0);
    }
    finish(out, 1.0)
}

/// A skill learned: a page turns and a quick run rises to a bell.
fn purchase(rng: &mut StdRng, variant: usize) -> Vec<f32> {
    let mut out = vec![0.0; secs(6.5)];
    burst(&mut out, rng, 0, 0.25, (900.0, 3500.0), 0.12, |t| {
        (PI * t / 0.25).sin().max(0.0)
    });
    let start = (variant * 2) as i32 % 5;
    for k in 0..5 {
        celesta(
            &mut out,
            secs(0.08 + k as f32 * 0.055),
            penta(5, start + k),
            0.35 + k as f32 * 0.05,
            0.45,
        );
    }
    let top = penta(5, start + 5);
    bell(&mut out, secs(0.08 + 5.0 * 0.055), top, 0.4, 0.9);
    celesta(&mut out, secs(0.08 + 5.0 * 0.055), top + 12.0, 0.05, 0.5);
    finish(out, 1.0)
}

/// The spells, in their hotbar order: Glimpse, Gathering, Echo, Summon, Rush.
fn spell(rng: &mut StdRng, slot: usize) -> Vec<f32> {
    match slot {
        // Kindred Glow at full: an airy bloom of high harmonics.
        0 => {
            let mut out = vec![0.0; secs(4.0)];
            for (k, &note) in [74.0, 78.0, 81.0, 86.0, 90.0].iter().enumerate() {
                let f = midi_hz(note);
                let ph = rng.gen_range(0.0..TAU);
                for (i, o) in out.iter_mut().enumerate() {
                    let t = i as f32 / SRF;
                    let e = (t / (0.25 + k as f32 * 0.08)).min(1.0) * (-(t - 0.5).max(0.0) / 0.6).exp();
                    let shimmer = 1.0 + 0.3 * (TAU * (5.0 + k as f32) * t + ph).sin();
                    *o += (TAU * f * t + ph).sin() * e * shimmer * (0.3 - k as f32 * 0.04);
                }
            }
            burst(&mut out, rng, 0, 1.6, (1800.0, 4000.0), 0.08, |t| {
                (t / 0.3).min(1.0) * (-t / 0.5).exp()
            });
            finish(out, 1.0)
        }
        // The Gathering Charm: a swirl of air and a harp sweeping upward.
        1 => {
            let mut out = vec![0.0; secs(4.0)];
            let mut g = Noise::new(rng.next_u32());
            let mut bp = Svf::new(600.0, 2.0);
            for (i, o) in out.iter_mut().enumerate() {
                let t = i as f32 / SRF;
                bp.set(700.0 + 500.0 * (TAU * 3.0 * t).sin() + 600.0 * t, 2.0);
                *o += bp.band(g.white()) * (t / 0.2).min(1.0) * (-t / 0.6).exp() * 0.5;
            }
            for k in 0..9 {
                harp(
                    &mut out,
                    secs(0.05 + k as f32 * 0.045),
                    penta(4, k + 1),
                    0.3 + k as f32 * 0.02,
                    1.1,
                );
            }
            finish(out, 1.0)
        }
        // Echo: one clear ping, ringing back fainter and duller.
        2 => {
            let mut ping = vec![0.0; secs(2.5)];
            bell(&mut ping, 0, 81.0, 1.0, 0.35);
            let mut out = vec![0.0; secs(3.5)];
            for k in 0..4 {
                let at = secs(k as f32 * 0.3);
                let a = 0.5f32.powi(k);
                let mut p = ping.clone();
                if k > 0 {
                    lowpass(&mut p, 5000.0 / k as f32);
                }
                for (o, x) in out[at..].iter_mut().zip(&p) {
                    *o += x * a;
                }
            }
            finish(out, 1.0)
        }
        // Summon Series: a swell drawn in, then a two-note chime.
        3 => {
            let mut out = vec![0.0; secs(3.5)];
            let mut g = Noise::new(rng.next_u32());
            let mut bp = Svf::new(500.0, 1.5);
            let mut ph = 0.0f32;
            for (i, o) in out.iter_mut().enumerate() {
                let t = i as f32 / SRF;
                let e = if t < 0.55 {
                    (t / 0.55).powi(3)
                } else {
                    (-(t - 0.55) / 0.06).exp()
                };
                bp.set(500.0 + 2500.0 * (t / 0.55).min(1.0), 1.5);
                ph += midi_hz(69.0 + 12.0 * (t / 0.55).min(1.0)) / SRF;
                *o += (bp.band(g.white()) * 0.4 + (TAU * ph).sin() * 0.12) * e;
            }
            celesta(&mut out, secs(0.55), 81.0, 0.6, 0.6);
            celesta(&mut out, secs(0.64), 86.0, 0.5, 0.7);
            finish(out, 1.0)
        }
        // Rush: a quick flurry of plucks over a rush of air.
        _ => {
            let mut out = vec![0.0; secs(3.0)];
            for k in 0..10 {
                harp(&mut out, secs(k as f32 * 0.035), penta(5, k), 0.28, 0.7);
            }
            let w = whoosh(rng);
            for (o, x) in out.iter_mut().zip(&w) {
                *o += x * 0.25;
            }
            finish(out, 1.0)
        }
    }
}

/// A familiar posted: paper unfolding into wings, a flutter, a small bright note.
fn unfold(rng: &mut StdRng) -> Vec<f32> {
    let mut out = vec![0.0; secs(3.5)];
    crinkles(&mut out, rng, 0, 0.22, 60.0, 0.7, (1800.0, 4500.0));
    let rate = rng.gen_range(11.0..14.0);
    burst(&mut out, rng, secs(0.18), 0.35, (700.0, 2600.0), 0.3, |t| {
        (0.5 + 0.5 * (TAU * rate * t).sin()).powi(4) * ar(t, 0.03, 0.12)
    });
    celesta(&mut out, secs(0.3), 78.0, 0.3, 0.45);
    celesta(&mut out, secs(0.37), 81.0, 0.25, 0.5);
    finish(out, 1.0)
}

/// A familiar taken back: paper creasing as it folds, a soft tap.
fn fold(rng: &mut StdRng) -> Vec<f32> {
    let mut out = vec![0.0; secs(0.8)];
    for k in 0..3 {
        let at = secs(k as f32 * rng.gen_range(0.07..0.1));
        burst(&mut out, rng, at, 0.05, (1200.0, 4000.0), 0.5 - k as f32 * 0.1, |t| {
            ar(t, 0.001, 0.008)
        });
    }
    crinkles(&mut out, rng, 0, 0.25, 50.0, 0.3, (1500.0, 3500.0));
    mode(&mut out, secs(0.27), 420.0, 0.25, 0.015);
    finish(out, 1.0)
}

/// Refused: a muted two-note knock, falling, like a soft mallet on wood.
fn nope(rng: &mut StdRng) -> Vec<f32> {
    let mut out = vec![0.0; secs(0.8)];
    for (k, note) in [69.0, 66.0].into_iter().enumerate() {
        let at = secs(k as f32 * 0.13);
        let f = midi_hz(note);
        tone(
            &mut out,
            at,
            f,
            1.0,
            &[(1.0, 1.0, 0.07), (4.0, 0.08, 0.015), (0.5, 0.15, 0.04)],
        );
        burst(&mut out, rng, at, 0.01, (100.0, 900.0), 0.2, |t| ar(t, 0.0005, 0.003));
    }
    lowpass2(&mut out, 2500.0);
    finish(out, 1.0)
}

/// A UI tick: a wooden bead clicking along.
fn tick(rng: &mut StdRng) -> Vec<f32> {
    let mut out = vec![0.0; secs(0.08)];
    let f = rng.gen_range(1700.0..2100.0);
    mode(&mut out, 0, f, 1.0, 0.005);
    mode(&mut out, 0, f * 2.3, 0.25, 0.003);
    mode(&mut out, 0, rng.gen_range(560.0..680.0), 0.3, 0.008);
    burst(&mut out, rng, 0, 0.004, (800.0, 5000.0), 0.3, |t| ar(t, 0.0002, 0.0008));
    lowpass2(&mut out, 6000.0);
    finish(out, 1.0)
}

/// A footfall: a soft heel on the boards and the scuff of the sole.
fn step(rng: &mut StdRng) -> Vec<f32> {
    let mut out = vec![0.0; secs(0.35)];
    mode(&mut out, 0, rng.gen_range(72.0..92.0), 1.0, 0.028);
    mode(&mut out, 0, rng.gen_range(150.0..200.0), 0.3, 0.02);
    burst(&mut out, rng, 0, 0.02, (120.0, 1200.0), 0.3, |t| ar(t, 0.001, 0.004));
    let toe = secs(rng.gen_range(0.04..0.07));
    mode(&mut out, toe, rng.gen_range(100.0..125.0), 0.3, 0.018);
    let (scuff, amp) = (rng.gen_range(0.025..0.05), rng.gen_range(0.08..0.16));
    burst(&mut out, rng, toe, 0.15, (400.0, 1600.0), amp, |t| ar(t, 0.006, scuff));
    lowpass2(&mut out, 2400.0);
    finish(out, 1.0)
}

/// Pushing off: the sole scuffing and clothes swishing.
fn hop(rng: &mut StdRng) -> Vec<f32> {
    let mut out = vec![0.0; secs(0.45)];
    mode(&mut out, 0, rng.gen_range(65.0..80.0), 0.6, 0.025);
    burst(&mut out, rng, 0, 0.1, (350.0, 1300.0), 0.5, |t| ar(t, 0.004, 0.035));
    let at = secs(rng.gen_range(0.015..0.03));
    let mut g = Noise::new(rng.next_u32());
    let mut bp = Svf::new(900.0, 1.0);
    for (i, o) in out[at..].iter_mut().enumerate() {
        let t = i as f32 / SRF;
        bp.set(900.0 + 3000.0 * t, 1.0);
        *o += bp.band(g.white()) * ar(t, 0.03, 0.07) * 0.3;
    }
    lowpass2(&mut out, 3500.0);
    finish(out, 1.0)
}

/// Landing on the floor: both heels nearly together, a settle of clothes.
fn alight(rng: &mut StdRng) -> Vec<f32> {
    let mut out = vec![0.0; secs(0.6)];
    let second = secs(rng.gen_range(0.012..0.03));
    for (at, a) in [(0, 1.0), (second, 0.7)] {
        mode(&mut out, at, rng.gen_range(58.0..72.0), a, 0.055);
        mode(&mut out, at, rng.gen_range(135.0..170.0), a * 0.4, 0.035);
        burst(&mut out, rng, at, 0.03, (100.0, 1000.0), a * 0.35, |t| {
            ar(t, 0.001, 0.006)
        });
    }
    mode(&mut out, 0, rng.gen_range(290.0..350.0), 0.1, 0.05);
    burst(&mut out, rng, secs(0.03), 0.2, (700.0, 2000.0), 0.12, |t| {
        ar(t, 0.01, 0.05)
    });
    lowpass2(&mut out, 3000.0);
    finish(out, 1.0)
}

// ---------------------------------------------------------------- grains

/// The smallest crackle of a fire.
fn crackle(rng: &mut StdRng) -> Vec<f32> {
    let mut out = vec![0.0; secs(0.04)];
    let d = rng.gen_range(0.0008..0.003);
    burst(&mut out, rng, 0, 0.02, (900.0, 6000.0), 1.0, |t| ar(t, 0.0002, d));
    mode(
        &mut out,
        0,
        rng.gen_range(1800.0..4500.0),
        0.3,
        rng.gen_range(0.002..0.005),
    );
    finish(out, 1.0)
}

/// A log settling or an ember popping: a sharper snap with a crackling tail.
fn pop(rng: &mut StdRng) -> Vec<f32> {
    let mut out = vec![0.0; secs(0.6)];
    burst(&mut out, rng, 0, 0.03, (600.0, 5000.0), 1.0, |t| ar(t, 0.0003, 0.004));
    mode(&mut out, 0, rng.gen_range(110.0..160.0), 0.4, 0.03);
    crinkles(&mut out, rng, secs(0.01), 0.3, 40.0, 0.3, (1500.0, 5000.0));
    finish(out, 1.0)
}

/// A raindrop on a pane: a tiny tick with a glassy ring.
fn raindrop(rng: &mut StdRng) -> Vec<f32> {
    let mut out = vec![0.0; secs(0.04)];
    mode(
        &mut out,
        0,
        rng.gen_range(2200.0..5000.0),
        1.0,
        rng.gen_range(0.0015..0.004),
    );
    burst(&mut out, rng, 0, 0.004, (1500.0, 8000.0), 0.5, |t| {
        ar(t, 0.0002, 0.0006)
    });
    finish(out, 1.0)
}

/// A drip into still water: the rising "plink" of a bubble's ring.
fn drip(rng: &mut StdRng) -> Vec<f32> {
    let n = secs(0.2);
    let (f0, f1) = (rng.gen_range(800.0..1100.0), rng.gen_range(1500.0..2100.0));
    let mut ph = 0.0f32;
    let out: Vec<f32> = (0..n)
        .map(|i| {
            let t = i as f32 / SRF;
            ph += (f0 + (f1 - f0) * (t / 0.03).min(1.0)) / SRF;
            (TAU * ph).sin() * ar(t, 0.0008, 0.022)
        })
        .collect();
    finish(out, 1.0)
}

/// An alembic's bubble: a small rising blip.
fn bubble(rng: &mut StdRng) -> Vec<f32> {
    let n = secs(0.15);
    let f0 = rng.gen_range(350.0..900.0);
    let rise = rng.gen_range(1.2..1.6);
    let tau = rng.gen_range(0.008..0.018);
    let mut ph = 0.0f32;
    let out: Vec<f32> = (0..n)
        .map(|i| {
            let t = i as f32 / SRF;
            ph += f0 * (1.0 + (rise - 1.0) * (t / 0.02).min(1.0)) / SRF;
            (TAU * ph).sin() * ar(t, 0.001, tau)
        })
        .collect();
    finish(out, 1.0)
}

/// The great clock's escapement: a woody click ringing in the case. `pitch` sets
/// tick against tock.
fn escapement(rng: &mut StdRng, pitch: f32) -> Vec<f32> {
    let mut out = vec![0.0; secs(0.5)];
    mode(&mut out, 0, rng.gen_range(2100.0..2400.0) * pitch, 0.6, 0.004);
    mode(&mut out, 0, rng.gen_range(850.0..950.0) * pitch, 0.5, 0.01);
    mode(&mut out, 0, 310.0 * pitch, 0.4, 0.025);
    mode(&mut out, 0, 175.0, 0.25, 0.06);
    burst(&mut out, rng, 0, 0.004, (1000.0, 6000.0), 0.3, |t| {
        ar(t, 0.0002, 0.0007)
    });
    lowpass2(&mut out, 4500.0);
    finish(out, 1.0)
}

/// A quill writing a few words by itself: scratchy strokes with pauses.
fn quill(rng: &mut StdRng) -> Vec<f32> {
    let mut out = vec![0.0; secs(1.4)];
    let mut t = 0.02;
    while t < 1.1 {
        let len = rng.gen_range(0.05..0.18);
        let rate = rng.gen_range(8.0..18.0);
        let a = rng.gen_range(0.4..1.0);
        let mut g = Noise::new(rng.next_u32());
        burst(&mut out, rng, secs(t), len, (2500.0, 6500.0), a, |x| {
            let shape = (PI * x / len).sin().max(0.0);
            shape * (0.55 + 0.45 * (TAU * rate * x).sin()) * (0.7 + 0.3 * g.unit())
        });
        t += len
            + if rng.gen_bool(0.3) {
                rng.gen_range(0.12..0.25)
            } else {
                rng.gen_range(0.01..0.05)
            };
    }
    finish(out, 1.0)
}

/// A page turning or a paper crane stirring: a soft swish with a crisp edge.
fn page(rng: &mut StdRng) -> Vec<f32> {
    let len = rng.gen_range(0.25..0.4);
    let mut out = vec![0.0; secs(len + 0.05)];
    let mut g = Noise::new(rng.next_u32());
    let mut bp = Svf::new(1000.0, 0.9);
    for (i, o) in out.iter_mut().enumerate().take(secs(len)) {
        let t = i as f32 / SRF;
        let u = t / len;
        bp.set(900.0 + 1800.0 * u, 0.9);
        *o += bp.band(g.white()) * (PI * u).sin().powf(2.0);
    }
    crinkles(&mut out, rng, secs(len * 0.6), len * 0.4, 60.0, 0.5, (2000.0, 4500.0));
    finish(out, 1.0)
}

/// Knitting needles going by themselves: a bout of soft wooden clicks.
fn knit(rng: &mut StdRng) -> Vec<f32> {
    let n = rng.gen_range(8..16);
    let mut out = vec![0.0; secs(n as f32 * 0.24 + 0.2)];
    let mut t = 0.0;
    for k in 0..n {
        let a = rng.gen_range(0.5..1.0) * if k % 2 == 0 { 1.0 } else { 0.7 };
        let at = secs(t);
        let f = rng.gen_range(2600.0..3100.0);
        mode(&mut out, at, f, a, 0.004);
        mode(&mut out, at, f * 1.47, a * 0.3, 0.003);
        mode(&mut out, at, rng.gen_range(700.0..800.0), a * 0.25, 0.008);
        t += if k % 2 == 0 {
            rng.gen_range(0.08..0.11)
        } else {
            rng.gen_range(0.16..0.24)
        };
    }
    lowpass2(&mut out, 7000.0);
    finish(out, 1.0)
}

/// Thunder no bigger than a jar: a low grumble rolling round.
fn rumble(rng: &mut StdRng) -> Vec<f32> {
    let len = rng.gen_range(2.0..3.2);
    let mut out = noise(secs(len), rng.next_u32());
    lowpass2(&mut out, 160.0);
    lowpass2(&mut out, 160.0);
    let rolls: Vec<(f32, f32)> = (0..4)
        .map(|_| (rng.gen_range(0.0..len * 0.6), rng.gen_range(0.4..1.0)))
        .collect();
    for (i, x) in out.iter_mut().enumerate() {
        let t = i as f32 / SRF;
        let e: f32 = rolls
            .iter()
            .map(|&(at, a)| if t > at { a * ar(t - at, 0.12, 0.5) } else { 0.0 })
            .sum();
        *x *= e * (1.0 - t / len);
    }
    finish(out, 1.0)
}

/// Old wood under load, or a chain taking the strain: a slow stick-slip creak.
fn creak(rng: &mut StdRng) -> Vec<f32> {
    let len = rng.gen_range(0.4..0.9);
    let n = secs(len);
    let (r0, r1) = (rng.gen_range(25.0..40.0), rng.gen_range(45.0..80.0));
    let mut bp = Svf::new(rng.gen_range(380.0..650.0), 9.0);
    let mut bp2 = Svf::new(rng.gen_range(900.0..1300.0), 6.0);
    let mut ph = 0.0f32;
    let mut g = Noise::new(rng.next_u32());
    let out: Vec<f32> = (0..n)
        .map(|i| {
            let t = i as f32 / n as f32;
            ph += (r0 + (r1 - r0) * t) / SRF;
            let x = if ph >= 1.0 {
                ph -= 1.0;
                1.0 + 0.3 * g.white()
            } else {
                0.0
            };
            (bp.band(x) + 0.4 * bp2.band(x)) * (PI * t).sin().powf(0.7)
        })
        .collect();
    finish(out, 1.0)
}

/// A slow heartbeat, felt more than heard.
fn heart(rng: &mut StdRng) -> Vec<f32> {
    let mut out = vec![0.0; secs(1.0)];
    mode(&mut out, 0, rng.gen_range(52.0..58.0), 1.0, 0.07);
    mode(&mut out, 0, 110.0, 0.15, 0.03);
    let dub = secs(rng.gen_range(0.24..0.3));
    mode(&mut out, dub, rng.gen_range(45.0..50.0), 0.6, 0.09);
    lowpass2(&mut out, 300.0);
    finish(out, 1.0)
}

/// A cricket's chirp: three or four quick pulses of a high, pure note.
fn cricket(rng: &mut StdRng) -> Vec<f32> {
    let f = rng.gen_range(4100.0..4700.0);
    let pulses = rng.gen_range(3..=4);
    let n = secs(pulses as f32 * 0.03 + 0.02);
    let out: Vec<f32> = (0..n)
        .map(|i| {
            let t = i as f32 / SRF;
            let k = (t / 0.03) as usize;
            let u = (t % 0.03) / 0.017;
            let gate = if k < pulses && u < 1.0 { (PI * u).sin() } else { 0.0 };
            (TAU * f * t).sin() * gate
        })
        .collect();
    finish(out, 1.0)
}

/// An owl far off through the window: two soft, breathy hoots.
fn owl(rng: &mut StdRng) -> Vec<f32> {
    let mut out = vec![0.0; secs(1.6)];
    let base = rng.gen_range(350.0..400.0);
    let mut g = Noise::new(rng.next_u32());
    for (at, len, f) in [(0.0, 0.32, base), (0.55, 0.6, base * 0.92)] {
        let s = secs(at);
        let mut bp = Svf::new(f, 6.0);
        let mut ph = 0.0f32;
        for i in 0..secs(len) {
            let t = i as f32 / SRF;
            let e = (PI * t / len).sin().powf(1.5);
            // Each hoot rises a little and falls back.
            ph += f * (1.0 + 0.04 * (PI * t / len).sin()) / SRF;
            out[s + i] += ((TAU * ph).sin() + bp.band(g.white()) * 0.6) * e;
        }
    }
    lowpass2(&mut out, 1500.0);
    finish(out, 1.0)
}
