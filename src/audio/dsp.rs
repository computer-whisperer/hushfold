//! Signal-processing pieces shared by the clips, the ambience and the music: noise,
//! filters, smoothing, panning and a stereo reverb.

use std::f32::consts::{PI, TAU};

pub const SR: u32 = 44100;
pub const SRF: f32 = SR as f32;

/// Samples in `s` seconds.
pub fn secs(s: f32) -> usize {
    (s * SRF) as usize
}

/// Frequency of a MIDI note (69 = A4 = 440 Hz); fractional notes allowed.
pub fn midi_hz(note: f32) -> f32 {
    440.0 * 2f32.powf((note - 69.0) / 12.0)
}

/// MIDI note of degree `d` of D major pentatonic, from D in octave `oct` (4 = D4):
/// the key the tonal sounds are written in.
pub fn penta(oct: i32, d: i32) -> f32 {
    const STEPS: [f32; 5] = [0.0, 2.0, 4.0, 7.0, 9.0];
    62.0 + 12.0 * (oct - 4 + d.div_euclid(5)) as f32 + STEPS[d.rem_euclid(5) as usize]
}

/// Per-sample multiplier that decays by 1/e over `tau` seconds.
pub fn decay_per_sample(tau: f32) -> f32 {
    (-1.0 / (tau * SRF)).exp()
}

/// A fast xorshift noise source for audio-rate use.
#[derive(Clone)]
pub struct Noise(u32);

impl Noise {
    pub fn new(seed: u32) -> Self {
        Noise(seed.wrapping_mul(0x9E37_79B9) | 1)
    }

    /// Uniform in 0..1.
    pub fn unit(&mut self) -> f32 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.0 = x;
        (x >> 8) as f32 / (1u32 << 24) as f32
    }

    /// Uniform in -1..1.
    pub fn white(&mut self) -> f32 {
        self.unit() * 2.0 - 1.0
    }
}

/// One-pole lowpass (its complement is the matching highpass).
#[derive(Clone, Default)]
pub struct OnePole {
    a: f32,
    y: f32,
}

impl OnePole {
    pub fn new(cutoff: f32) -> Self {
        let mut f = OnePole::default();
        f.set(cutoff);
        f
    }

    pub fn set(&mut self, cutoff: f32) {
        self.a = 1.0 - (-TAU * cutoff.min(SRF * 0.45) / SRF).exp();
    }

    pub fn lp(&mut self, x: f32) -> f32 {
        self.y += self.a * (x - self.y);
        self.y
    }

    pub fn hp(&mut self, x: f32) -> f32 {
        x - self.lp(x)
    }
}

/// State-variable filter (the trapezoidal form), stable while its cutoff moves.
#[derive(Clone, Default)]
pub struct Svf {
    ic1: f32,
    ic2: f32,
    a1: f32,
    a2: f32,
    a3: f32,
    k: f32,
}

impl Svf {
    pub fn new(cutoff: f32, q: f32) -> Self {
        let mut f = Svf::default();
        f.set(cutoff, q);
        f
    }

    pub fn set(&mut self, cutoff: f32, q: f32) {
        let g = (PI * cutoff.clamp(10.0, SRF * 0.45) / SRF).tan();
        self.k = 1.0 / q;
        self.a1 = 1.0 / (1.0 + g * (g + self.k));
        self.a2 = g * self.a1;
        self.a3 = g * self.a2;
    }

    /// One sample in; (low, band, high) out.
    pub fn tick(&mut self, x: f32) -> (f32, f32, f32) {
        let v3 = x - self.ic2;
        let v1 = self.a1 * self.ic1 + self.a2 * v3;
        let v2 = self.ic2 + self.a2 * self.ic1 + self.a3 * v3;
        self.ic1 = 2.0 * v1 - self.ic1;
        self.ic2 = 2.0 * v2 - self.ic2;
        (v2, v1, x - self.k * v1 - v2)
    }

    pub fn band(&mut self, x: f32) -> f32 {
        self.tick(x).1
    }

    pub fn low(&mut self, x: f32) -> f32 {
        self.tick(x).0
    }
}

/// A value easing toward its target with a time constant, stepped once per block.
#[derive(Clone, Copy)]
pub struct Smooth {
    pub value: f32,
    pub target: f32,
}

impl Smooth {
    pub fn new(v: f32) -> Self {
        Smooth { value: v, target: v }
    }

    /// Ease over `dt` seconds with time constant `tau`; returns (start, end) of the
    /// step so a block can ramp between them.
    pub fn step(&mut self, dt: f32, tau: f32) -> (f32, f32) {
        let from = self.value;
        self.value += (self.target - self.value) * (1.0 - (-dt / tau).exp());
        if (self.value - self.target).abs() < 1e-5 {
            self.value = self.target;
        }
        (from, self.value)
    }
}

/// Equal-power stereo gains for a pan in -1 (left) ..= 1 (right).
pub fn pan_gains(pan: f32) -> (f32, f32) {
    let a = (pan.clamp(-1.0, 1.0) + 1.0) * PI * 0.25;
    (a.cos(), a.sin())
}

/// One partial of `Partials`: a rotating phasor and its decaying amplitude.
#[derive(Clone, Copy, Default)]
struct Partial {
    c: f32,
    s: f32,
    rc: f32,
    rs: f32,
    amp: f32,
    decay: f32,
}

/// A sum of sine partials, each decaying at its own rate: struck and plucked tones,
/// cheap to run in real time.
#[derive(Clone, Copy, Default)]
pub struct Partials {
    p: [Partial; 12],
    n: usize,
}

impl Partials {
    /// A partial at `freq` Hz and amplitude `amp`, dying away by 1/e over `tau`
    /// seconds (never, if `tau` is infinite). Partials too high to sound cleanly, or
    /// past the twelfth, are left out.
    pub fn add(&mut self, freq: f32, amp: f32, tau: f32) {
        if self.n == self.p.len() || freq > SRF * 0.4 {
            return;
        }
        let w = TAU * freq / SRF;
        self.p[self.n] = Partial {
            c: 1.0,
            s: 0.0,
            rc: w.cos(),
            rs: w.sin(),
            amp,
            decay: if tau.is_finite() { decay_per_sample(tau) } else { 1.0 },
        };
        self.n += 1;
    }

    /// The next sample.
    pub fn next(&mut self) -> f32 {
        let mut out = 0.0;
        for p in &mut self.p[..self.n] {
            out += p.s * p.amp;
            (p.c, p.s) = (p.c * p.rc - p.s * p.rs, p.c * p.rs + p.s * p.rc);
            p.amp *= p.decay;
        }
        out
    }

    /// Keep the phasors on the unit circle and silence partials that have died
    /// (before they sink into slow denormal arithmetic); call once a block or so.
    /// Returns the loudest partial's amplitude.
    pub fn tidy(&mut self) -> f32 {
        let mut loudest = 0.0f32;
        for p in &mut self.p[..self.n] {
            let m = 1.5 - 0.5 * (p.c * p.c + p.s * p.s);
            p.c *= m;
            p.s *= m;
            if p.amp < 1e-12 {
                p.amp = 0.0;
            }
            loudest = loudest.max(p.amp);
        }
        loudest
    }
}

/// A lowpass comb filter (Freeverb's), its feedback set for a decay time.
struct Comb {
    line: Vec<f32>,
    idx: usize,
    store: f32,
    feedback: f32,
}

impl Comb {
    fn new(len: usize) -> Self {
        Comb {
            line: vec![0.0; len],
            idx: 0,
            store: 0.0,
            feedback: 0.0,
        }
    }

    fn set_decay(&mut self, rt60: f32) {
        self.feedback = 10f32.powf(-3.0 * self.line.len() as f32 / (SRF * rt60));
    }

    fn tick(&mut self, x: f32, damp: f32) -> f32 {
        let y = self.line[self.idx];
        // The tiny offset keeps the loop out of denormals in silence.
        self.store = y + (self.store - y) * damp + 1e-18;
        self.line[self.idx] = x + self.store * self.feedback;
        self.idx = (self.idx + 1) % self.line.len();
        y
    }
}

struct Allpass {
    line: Vec<f32>,
    idx: usize,
}

impl Allpass {
    fn tick(&mut self, x: f32) -> f32 {
        let b = self.line[self.idx];
        self.line[self.idx] = x + b * 0.5;
        self.idx = (self.idx + 1) % self.line.len();
        b - x
    }
}

/// A stereo reverb (Freeverb's tunings) whose decay time and damping can change
/// while it runs.
pub struct Reverb {
    combs: [Vec<Comb>; 2],
    allpasses: [Vec<Allpass>; 2],
    damp: f32,
    rt60: f32,
}

impl Reverb {
    pub fn new(rt60: f32, damp: f32) -> Self {
        const COMBS: [usize; 8] = [1116, 1188, 1277, 1356, 1422, 1491, 1557, 1617];
        const ALLPASSES: [usize; 4] = [556, 441, 341, 225];
        const SPREAD: usize = 23;
        let side = |s: usize| {
            (
                COMBS.iter().map(|&l| Comb::new(l + s)).collect::<Vec<_>>(),
                (ALLPASSES.iter())
                    .map(|&l| Allpass {
                        line: vec![0.0; l + s],
                        idx: 0,
                    })
                    .collect::<Vec<_>>(),
            )
        };
        let (cl, al) = side(0);
        let (cr, ar) = side(SPREAD);
        let mut r = Reverb {
            combs: [cl, cr],
            allpasses: [al, ar],
            damp,
            rt60: 0.0,
        };
        r.set(rt60, damp);
        r
    }

    pub fn set(&mut self, rt60: f32, damp: f32) {
        self.damp = damp;
        if (rt60 - self.rt60).abs() > 0.005 {
            self.rt60 = rt60;
            for c in self.combs.iter_mut().flatten() {
                c.set_decay(rt60);
            }
        }
    }

    /// Reverberate a mono send into a stereo pair, added to `l` and `r`.
    pub fn process(&mut self, send: &[f32], l: &mut [f32], r: &mut [f32]) {
        for (i, &x) in send.iter().enumerate() {
            let x = x * 0.03;
            for (side, out) in [(0, &mut l[i]), (1, &mut r[i])] {
                let mut y = 0.0;
                for c in &mut self.combs[side] {
                    y += c.tick(x, self.damp);
                }
                for a in &mut self.allpasses[side] {
                    y = a.tick(y);
                }
                *out += y;
            }
        }
    }
}

// ------------------------------------------------------------ offline helpers

/// White noise, `n` samples.
pub fn noise(n: usize, seed: u32) -> Vec<f32> {
    let mut g = Noise::new(seed);
    (0..n).map(|_| g.white()).collect()
}

pub fn lowpass(buf: &mut [f32], cutoff: f32) {
    let mut f = OnePole::new(cutoff);
    for x in buf.iter_mut() {
        *x = f.lp(*x);
    }
}

pub fn highpass(buf: &mut [f32], cutoff: f32) {
    let mut f = OnePole::new(cutoff);
    for x in buf.iter_mut() {
        *x = f.hp(*x);
    }
}

/// A two-pole lowpass (Butterworth-like) for steeper tops than `lowpass`.
pub fn lowpass2(buf: &mut [f32], cutoff: f32) {
    let mut f = Svf::new(cutoff, std::f32::consts::FRAC_1_SQRT_2);
    for x in buf.iter_mut() {
        *x = f.low(*x);
    }
}

pub fn peak(buf: &[f32]) -> f32 {
    buf.iter().fold(0.0f32, |a, &x| a.max(x.abs()))
}

pub fn normalize(buf: &mut [f32], to: f32) {
    let m = peak(buf);
    if m > 0.0 {
        for x in buf.iter_mut() {
            *x *= to / m;
        }
    }
}

/// Add a damped sine (one mode of something struck) starting at `start`.
pub fn mode(out: &mut [f32], start: usize, freq: f32, amp: f32, tau: f32) {
    let n = secs(tau * 7.0).min(out.len().saturating_sub(start));
    let (mut c, mut s) = (1.0f32, 0.0f32);
    let (rc, rs) = ((TAU * freq / SRF).cos(), (TAU * freq / SRF).sin());
    let d = decay_per_sample(tau);
    let mut a = amp;
    for o in &mut out[start..start + n] {
        *o += s * a;
        (c, s) = (c * rc - s * rs, c * rs + s * rc);
        a *= d;
    }
}

/// Ramp the first `fade_in` and last `fade_out` seconds, so a clip starts and ends
/// at zero.
pub fn fade_ends(buf: &mut [f32], fade_in: f32, fade_out: f32) {
    let n = buf.len();
    let (a, b) = (secs(fade_in).min(n), secs(fade_out).min(n));
    for (i, x) in buf[..a].iter_mut().enumerate() {
        *x *= i as f32 / a as f32;
    }
    for (i, x) in buf[n - b..].iter_mut().rev().enumerate() {
        *x *= i as f32 / b as f32;
    }
}

/// The usual last steps for a clip: no DC, its peak at `to`, and its tail cut where
/// it has died away (60 dB down), so it is never cut off while still heard.
pub fn finish(mut buf: Vec<f32>, to: f32) -> Vec<f32> {
    highpass(&mut buf, 25.0);
    normalize(&mut buf, to);
    let quiet = to * 1e-3;
    let end = buf.iter().rposition(|x| x.abs() > quiet).map_or(0, |i| i + 1);
    buf.truncate(end);
    fade_ends(&mut buf, 0.0005, 0.03);
    buf
}
