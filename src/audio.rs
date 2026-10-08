//! Procedurally synthesised sound: book thunks, page rustles, chimes, fire and a
//! generative music-box loop. Nothing is loaded from disk.

use rand::prelude::*;
use rodio::{ChannelCount, Sample, SampleRate, Source};
use std::num::NonZero;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

const SR: u32 = 44100;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Sfx {
    Thunk,
    Rustle,
    Whoosh,
    Chime,
    Fanfare,
    Nope,
    Tick,
    Purchase,
}

const SFX_COUNT: usize = 8;

fn sfx_index(s: Sfx) -> usize {
    s as usize
}

/// A mono clip, optionally looping, with an optional shared live gain.
struct Clip {
    data: Arc<[f32]>,
    pos: usize,
    gain: f32,
    looping: bool,
    live_gain: Option<Arc<AtomicU32>>,
    smoothed: f32,
}

impl Iterator for Clip {
    type Item = Sample;
    fn next(&mut self) -> Option<Sample> {
        if self.pos >= self.data.len() {
            if !self.looping {
                return None;
            }
            self.pos = 0;
        }
        let mut g = self.gain;
        if let Some(live) = &self.live_gain {
            let target = f32::from_bits(live.load(Ordering::Relaxed));
            self.smoothed += (target - self.smoothed) * 0.0005;
            g *= self.smoothed;
        }
        let s = self.data[self.pos] * g;
        self.pos += 1;
        Some(s as Sample)
    }
}

impl Source for Clip {
    fn current_span_len(&self) -> Option<usize> {
        None
    }
    fn channels(&self) -> ChannelCount {
        NonZero::new(1).unwrap()
    }
    fn sample_rate(&self) -> SampleRate {
        NonZero::new(SR).unwrap()
    }
    fn total_duration(&self) -> Option<Duration> {
        None
    }
}

pub struct Audio {
    sink: Option<rodio::MixerDeviceSink>,
    banks: Vec<Vec<Arc<[f32]>>>,
    last: [Option<Instant>; SFX_COUNT],
    rng: StdRng,
    pub music_gain: Arc<AtomicU32>,
    pub music_on: bool,
}

impl Audio {
    pub fn new() -> Self {
        let sink = match rodio::DeviceSinkBuilder::open_default_sink() {
            Ok(mut s) => {
                s.log_on_drop(false);
                Some(s)
            }
            Err(e) => {
                eprintln!("hushfold: audio unavailable ({e}); continuing silently");
                None
            }
        };
        let mut rng = StdRng::seed_from_u64(7);
        let mut banks: Vec<Vec<Arc<[f32]>>> = vec![Vec::new(); SFX_COUNT];
        for v in 0..6 {
            let p = 0.85 + v as f32 * 0.06;
            banks[sfx_index(Sfx::Thunk)].push(thunk(&mut rng, p).into());
            banks[sfx_index(Sfx::Rustle)].push(rustle(&mut rng).into());
            banks[sfx_index(Sfx::Whoosh)].push(whoosh(&mut rng, p).into());
        }
        // Chime variants are indexed by pitch step.
        for step in 0..13 {
            banks[sfx_index(Sfx::Chime)].push(chime(step).into());
        }
        banks[sfx_index(Sfx::Fanfare)].push(fanfare().into());
        banks[sfx_index(Sfx::Nope)].push(nope().into());
        banks[sfx_index(Sfx::Tick)].push(tick().into());
        banks[sfx_index(Sfx::Purchase)].push(purchase().into());

        let music_gain = Arc::new(AtomicU32::new(1.0f32.to_bits()));
        if let Some(s) = &sink {
            let mixer = s.mixer().clone();
            let gain = music_gain.clone();
            std::thread::spawn(move || {
                let amb: Arc<[f32]> = ambience().into();
                mixer.add(Clip { data: amb, pos: 0, gain: 0.35, looping: true, live_gain: None, smoothed: 1.0 });
                let music: Arc<[f32]> = music().into();
                mixer.add(Clip { data: music, pos: 0, gain: 0.5, looping: true, live_gain: Some(gain), smoothed: 0.0 });
            });
        }
        Audio { sink, banks, last: [None; SFX_COUNT], rng, music_gain, music_on: true }
    }

    pub fn toggle_music(&mut self) {
        self.music_on = !self.music_on;
        let g: f32 = if self.music_on { 1.0 } else { 0.0 };
        self.music_gain.store(g.to_bits(), Ordering::Relaxed);
    }

    pub fn play(&mut self, s: Sfx, vol: f32) {
        let n = self.banks[sfx_index(s)].len();
        let v = self.rng.gen_range(0..n);
        self.play_variant(s, vol, v);
    }

    pub fn play_pitched(&mut self, s: Sfx, vol: f32, step: usize) {
        let n = self.banks[sfx_index(s)].len();
        self.play_variant(s, vol, step.min(n - 1));
    }

    fn play_variant(&mut self, s: Sfx, vol: f32, v: usize) {
        let Some(sink) = &self.sink else { return };
        // Avoid machine-gunning identical sounds.
        let min_gap = match s {
            Sfx::Thunk => 0.025,
            Sfx::Rustle | Sfx::Whoosh => 0.06,
            Sfx::Chime => 0.08,
            _ => 0.03,
        };
        let now = Instant::now();
        if let Some(t) = self.last[sfx_index(s)] {
            if now.duration_since(t).as_secs_f32() < min_gap {
                return;
            }
        }
        self.last[sfx_index(s)] = Some(now);
        let data = self.banks[sfx_index(s)][v].clone();
        sink.mixer().add(Clip { data, pos: 0, gain: vol, looping: false, live_gain: None, smoothed: 1.0 });
    }
}

// ---------------------------------------------------------------- synthesis

fn secs(s: f32) -> usize {
    (s * SR as f32) as usize
}

fn lowpass(buf: &mut [f32], cutoff: f32) {
    let a = 1.0 - (-2.0 * std::f32::consts::PI * cutoff / SR as f32).exp();
    let mut y = 0.0;
    for x in buf.iter_mut() {
        y += a * (*x - y);
        *x = y;
    }
}

fn highpass(buf: &mut [f32], cutoff: f32) {
    let mut lp = buf.to_vec();
    lowpass(&mut lp, cutoff);
    for (x, l) in buf.iter_mut().zip(lp) {
        *x -= l;
    }
}

fn normalize(buf: &mut [f32], peak: f32) {
    let m = buf.iter().fold(0.0f32, |a, &x| a.max(x.abs()));
    if m > 0.0 {
        for x in buf.iter_mut() {
            *x *= peak / m;
        }
    }
}

fn thunk(rng: &mut StdRng, pitch: f32) -> Vec<f32> {
    let n = secs(0.18);
    let mut out = vec![0.0; n];
    let mut noise: Vec<f32> = (0..n).map(|_| rng.gen_range(-1.0..1.0)).collect();
    lowpass(&mut noise, 1800.0 * pitch);
    let mut ph = 0.0f32;
    let mut ph2 = 0.0f32;
    for i in 0..n {
        let t = i as f32 / SR as f32;
        let f = (150.0 + 120.0 * (-t * 60.0).exp()) * pitch;
        ph += f / SR as f32;
        ph2 += 620.0 * pitch / SR as f32;
        let body = (ph * std::f32::consts::TAU).sin() * (-t * 38.0).exp();
        let knock = (ph2 * std::f32::consts::TAU).sin() * (-t * 90.0).exp() * 0.35;
        let click = noise[i] * (-t * 160.0).exp() * 1.2;
        out[i] = body + knock + click;
    }
    normalize(&mut out, 0.8);
    out
}

fn rustle(rng: &mut StdRng) -> Vec<f32> {
    let n = secs(0.32);
    let mut noise: Vec<f32> = (0..n).map(|_| rng.gen_range(-1.0..1.0)).collect();
    highpass(&mut noise, 1500.0);
    lowpass(&mut noise, 6000.0);
    let bursts: Vec<(f32, f32)> = (0..7).map(|_| (rng.gen_range(0.0..0.22), rng.gen_range(0.3..1.0))).collect();
    for i in 0..n {
        let t = i as f32 / SR as f32;
        let mut env = 0.0;
        for &(bt, amp) in &bursts {
            if t >= bt {
                env += amp * (-(t - bt) * 40.0).exp();
            }
        }
        noise[i] *= env * (1.0 - t / 0.32).max(0.0);
    }
    normalize(&mut noise, 0.6);
    noise
}

fn whoosh(rng: &mut StdRng, pitch: f32) -> Vec<f32> {
    let n = secs(0.3);
    let mut out = vec![0.0; n];
    let mut lp = 0.0f32;
    let mut lp2 = 0.0f32;
    for i in 0..n {
        let t = i as f32 / n as f32;
        let cutoff = (400.0 + 2400.0 * (t * std::f32::consts::PI).sin()) * pitch;
        let a = 1.0 - (-2.0 * std::f32::consts::PI * cutoff / SR as f32).exp();
        let x: f32 = rng.gen_range(-1.0..1.0);
        lp += a * (x - lp);
        lp2 += a * 0.5 * (lp - lp2);
        out[i] = (lp - lp2) * (t * std::f32::consts::PI).sin().powf(1.5);
    }
    normalize(&mut out, 0.4);
    out
}

fn bell(out: &mut [f32], start: usize, freq: f32, amp: f32, decay: f32) {
    let partials = [(1.0, 1.0), (2.0, 0.45), (2.76, 0.25), (5.4, 0.08)];
    let n = secs(decay * 5.0).min(out.len().saturating_sub(start));
    for i in 0..n {
        let t = i as f32 / SR as f32;
        let attack = (t / 0.004).min(1.0);
        let mut s = 0.0;
        for (k, (mul, a)) in partials.iter().enumerate() {
            let d = decay / (1.0 + k as f32 * 1.4);
            s += a * (std::f32::consts::TAU * freq * mul * t).sin() * (-t / d).exp();
        }
        out[start + i] += s * amp * attack;
    }
}

const PENTA: [f32; 5] = [0.0, 2.0, 4.0, 7.0, 9.0];

fn penta_freq(base: f32, step: i32) -> f32 {
    let oct = step.div_euclid(5);
    let deg = PENTA[step.rem_euclid(5) as usize];
    base * 2f32.powf(oct as f32 + deg / 12.0)
}

fn chime(step: usize) -> Vec<f32> {
    let mut out = vec![0.0; secs(2.2)];
    let base = 587.33; // D5
    let s = step as i32;
    bell(&mut out, 0, penta_freq(base, s % 6), 0.6, 0.5);
    bell(&mut out, secs(0.09), penta_freq(base, s % 6 + 2), 0.45, 0.5);
    bell(&mut out, secs(0.18), penta_freq(base, s % 6 + 4), 0.35, 0.6);
    reverb(&mut out, 0.25);
    normalize(&mut out, 0.55);
    out
}

fn fanfare() -> Vec<f32> {
    let mut out = vec![0.0; secs(4.5)];
    let base = 293.66;
    for (k, step) in [0, 2, 3, 5, 7, 10].iter().enumerate() {
        bell(&mut out, secs(k as f32 * 0.14), penta_freq(base, *step), 0.5, 0.9);
    }
    bell(&mut out, secs(0.9), penta_freq(base, 0), 0.35, 1.4);
    bell(&mut out, secs(0.9), penta_freq(base, 3), 0.3, 1.4);
    bell(&mut out, secs(0.9), penta_freq(base, 5), 0.3, 1.4);
    reverb(&mut out, 0.4);
    normalize(&mut out, 0.6);
    out
}

fn nope() -> Vec<f32> {
    let n = secs(0.22);
    let mut out = vec![0.0; n];
    for i in 0..n {
        let t = i as f32 / SR as f32;
        let (f, t0) = if t < 0.09 { (240.0, 0.0) } else { (190.0, 0.09) };
        let tt = t - t0;
        out[i] = (std::f32::consts::TAU * f * tt).sin() * (-tt * 30.0).exp() * (tt / 0.003).min(1.0);
    }
    normalize(&mut out, 0.35);
    out
}

fn tick() -> Vec<f32> {
    let n = secs(0.03);
    (0..n)
        .map(|i| {
            let t = i as f32 / SR as f32;
            (std::f32::consts::TAU * 2600.0 * t).sin() * (-t * 300.0).exp() * 0.3
        })
        .collect()
}

fn purchase() -> Vec<f32> {
    let mut out = vec![0.0; secs(2.0)];
    for k in 0..6 {
        bell(&mut out, secs(k as f32 * 0.06), penta_freq(587.33, k + 3), 0.3, 0.35);
    }
    reverb(&mut out, 0.35);
    normalize(&mut out, 0.5);
    out
}

/// Small Schroeder reverb, mixed in place.
fn reverb(buf: &mut [f32], wet: f32) {
    let combs = [1557usize, 1617, 1491, 1422];
    let fb = 0.78;
    let mut acc = vec![0.0f32; buf.len()];
    for &d in &combs {
        let mut line = vec![0.0f32; d];
        let mut idx = 0;
        let mut lp = 0.0f32;
        for i in 0..buf.len() {
            let y = line[idx];
            lp += 0.4 * (y - lp);
            line[idx] = buf[i] + lp * fb;
            idx = (idx + 1) % d;
            acc[i] += y * 0.25;
        }
    }
    for &d in &[225usize, 556] {
        let mut line = vec![0.0f32; d];
        let mut idx = 0;
        for x in acc.iter_mut() {
            let b = line[idx];
            let y = -*x + b;
            line[idx] = *x + b * 0.5;
            idx = (idx + 1) % d;
            *x = y;
        }
    }
    for (x, r) in buf.iter_mut().zip(acc) {
        *x = *x * (1.0 - wet * 0.5) + r * wet;
    }
}

/// Fire crackle and room tone, ~24 s loop.
fn ambience() -> Vec<f32> {
    let mut rng = StdRng::seed_from_u64(99);
    let n = secs(24.0);
    let mut rumble: Vec<f32> = (0..n).map(|_| rng.gen_range(-1.0..1.0)).collect();
    lowpass(&mut rumble, 180.0);
    lowpass(&mut rumble, 180.0);
    normalize(&mut rumble, 0.12);
    let mut crackle = vec![0.0f32; n];
    let mut t = 0usize;
    while t < n {
        t += secs(rng.gen_range(0.02..0.5));
        let len = secs(rng.gen_range(0.002..0.012));
        let amp = rng.gen_range(0.05..0.5f32).powf(2.0);
        for i in 0..len {
            if t + i < n {
                crackle[t + i] += rng.gen_range(-1.0..1.0) * amp * (1.0 - i as f32 / len as f32);
            }
        }
    }
    highpass(&mut crackle, 900.0);
    let mut out: Vec<f32> = rumble.iter().zip(&crackle).map(|(a, b)| a + b * 0.6).collect();
    // Crossfade the loop seam.
    let fade = secs(0.5);
    for i in 0..fade {
        let k = i as f32 / fade as f32;
        let tail = out[n - fade + i];
        out[i] = out[i] * k + tail * (1.0 - k);
    }
    out.truncate(n - fade);
    out
}

/// A slow, generative music-box piece in D major pentatonic over a soft pad.
fn music() -> Vec<f32> {
    let mut rng = StdRng::seed_from_u64(2024);
    let len_s = 128.0;
    let n = secs(len_s);
    let tail = secs(6.0);
    let mut out = vec![0.0f32; n + tail];
    // Pad chords, 16 s each.
    let chords: [[i32; 3]; 8] = [[0, 2, 4], [-2, 0, 3], [1, 3, 5], [-1, 2, 4], [0, 3, 5], [-2, 1, 3], [1, 2, 4], [-1, 0, 3]];
    let pad_base = 146.83; // D3
    for (ci, ch) in chords.iter().enumerate() {
        let start = secs(ci as f32 * 16.0);
        let dur = secs(18.0);
        for &step in ch {
            let f = penta_freq(pad_base, step);
            for i in 0..dur {
                let idx = start + i;
                if idx >= out.len() {
                    break;
                }
                let t = i as f32 / SR as f32;
                let env = (t / 4.0).min(1.0) * ((18.0 - t) / 4.0).clamp(0.0, 1.0);
                let s = (std::f32::consts::TAU * f * t).sin() + 0.3 * (std::f32::consts::TAU * f * 2.0 * t + 0.4).sin();
                let wob = 1.0 + 0.15 * (t * 0.7 + step as f32).sin();
                out[idx] += s * env * 0.05 * wob;
            }
        }
    }
    // Plucked melody.
    let mel_base = 587.33;
    let mut t = 1.0;
    let mut step: i32 = 2;
    while t < len_s - 1.0 {
        let chord = chords[((t / 16.0) as usize).min(7)];
        // Wander, gently attracted to chord tones.
        step += rng.gen_range(-2..=2);
        if rng.gen_bool(0.35) {
            step = chord[rng.gen_range(0..3)] + 5 * rng.gen_range(0..2);
        }
        step = step.clamp(-3, 9);
        let amp = rng.gen_range(0.12..0.22);
        bell(&mut out, secs(t), penta_freq(mel_base, step), amp, 0.9);
        if rng.gen_bool(0.2) {
            bell(&mut out, secs(t + 0.18), penta_freq(mel_base, step + 2), amp * 0.6, 0.9);
        }
        t += *[0.75f32, 1.0, 1.5, 1.5, 2.0, 3.0].choose(&mut rng).unwrap();
    }
    reverb(&mut out, 0.5);
    // Wrap the tail onto the start for a seamless loop.
    for i in 0..tail {
        out[i] += out[n + i];
    }
    out.truncate(n);
    normalize(&mut out, 0.3);
    out
}
