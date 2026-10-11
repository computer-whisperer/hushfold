//! The mix, rendered on the audio device's thread a block at a time: effect voices
//! placed around the listener, the ambience of where they stand, the music, a
//! reverb that follows the room, and a gentle limiter over it all.

use super::ambience::Ambience;
use super::clips::{Banks, Clip};
use super::dsp::*;
use super::music::Music;
use super::{Scene, Sfx};
use glam::Vec3;
use rand::prelude::*;
use std::f32::consts::SQRT_2;
use std::sync::Arc;
use std::sync::mpsc::Receiver;

/// Frames rendered at a time.
pub const BLOCK: usize = 256;
/// Seconds a block lasts.
pub const DT: f32 = BLOCK as f32 / SRF;
const MAX_VOICES: usize = 96;
/// Seconds over which a crowd of one effect thins out again.
const CROWD_TAU: f32 = 0.6;
/// Heard at least this loud (about 13 m off or nearer), an effect counts towards
/// its kind's gap and crowd.
const NEAR: f32 = 0.3;

/// What the game asks of the engine. A scene comes by value, every frame: boxed, it
/// would be freed on the audio thread.
#[allow(clippy::large_enum_variant)]
pub enum Cmd {
    /// The clips, once synthesised; until then plays are dropped.
    Banks(Arc<Banks>),
    Play {
        sfx: Sfx,
        vol: f32,
        variant: Option<usize>,
        at: Option<Vec3>,
    },
    Scene(Scene),
    Gains {
        music: f32,
        effects: f32,
    },
}

/// How each effect sits in the mix.
struct Props {
    /// Gain on a full-scale clip.
    gain: f32,
    /// How much goes to the room's reverb.
    send: f32,
    /// Seconds before it may start again.
    gap: f32,
    /// Written in D, so transposed to the music's key.
    tonal: bool,
    /// Pan spread when played at the listener.
    spread: f32,
    /// Playback-rate spread, either way.
    detune: f32,
    /// How much softer each one is for others just before it (flurries of filing
    /// and chiming thin out rather than pile up).
    crowd: f32,
}

fn props(s: Sfx) -> Props {
    let p = |gain, send, gap, spread, detune, crowd| Props {
        gain,
        send,
        gap,
        tonal: false,
        spread,
        detune,
        crowd,
    };
    let t = |gain, send, gap, crowd| Props {
        tonal: true,
        ..p(gain, send, gap, 0.0, 0.0, crowd)
    };
    match s {
        Sfx::Thunk => p(0.25, 0.3, 0.03, 0.0, 0.04, 0.06),
        Sfx::Rustle => p(0.17, 0.12, 0.06, 0.2, 0.03, 0.1),
        Sfx::Whoosh => p(0.18, 0.15, 0.06, 0.15, 0.04, 0.1),
        Sfx::Toss => p(0.2, 0.15, 0.06, 0.1, 0.04, 0.1),
        Sfx::Land => p(0.4, 0.3, 0.02, 0.0, 0.05, 0.08),
        Sfx::Chime => t(0.18, 0.35, 0.08, 0.3),
        Sfx::Fanfare => t(0.3, 0.35, 0.5, 0.0),
        Sfx::Veil => t(0.28, 0.4, 0.5, 0.0),
        Sfx::Purchase => t(0.26, 0.3, 0.1, 0.0),
        Sfx::Spell => t(0.32, 0.35, 0.1, 0.0),
        Sfx::Unfold => t(0.3, 0.25, 0.1, 0.0),
        Sfx::Fold => p(0.11, 0.15, 0.1, 0.0, 0.02, 0.0),
        Sfx::Nope => t(0.24, 0.1, 0.1, 0.0),
        Sfx::Tick => p(0.18, 0.05, 0.03, 0.0, 0.03, 0.0),
        Sfx::Step => p(0.26, 0.15, 0.08, 0.1, 0.05, 0.0),
        Sfx::Hop => p(0.12, 0.1, 0.1, 0.0, 0.04, 0.0),
        Sfx::Alight => p(0.18, 0.18, 0.1, 0.0, 0.04, 0.0),
        Sfx::Wade => p(0.2, 0.12, 0.08, 0.1, 0.05, 0.0),
    }
}

/// The gain on `s`'s clips in the mix.
#[cfg(test)]
pub fn gain(s: Sfx) -> f32 {
    props(s).gain
}

/// The listener: where the ears are and which way is right and ahead (level).
#[derive(Clone, Copy)]
pub struct Ears {
    pub pos: Vec3,
    pub right: Vec3,
    pub ahead: Vec3,
}

impl Ears {
    pub fn new(pos: Vec3, right: Vec3) -> Self {
        let right = Vec3::new(right.x, 0.0, right.z).normalize_or(Vec3::X);
        Ears {
            pos,
            right,
            ahead: Vec3::Y.cross(right),
        }
    }
}

/// How something at a place is heard: the gain in each ear, how dull distance
/// makes it, and how much more of it reaches the ears by way of the room.
#[derive(Clone, Copy)]
pub struct Heard {
    pub left: f32,
    pub right: f32,
    pub cutoff: f32,
    pub wet: f32,
}

impl Heard {
    /// Heard at the ears, panned by `pan`.
    pub fn near(pan: f32) -> Self {
        let (l, r) = pan_gains(pan);
        Heard {
            left: l * SQRT_2,
            right: r * SQRT_2,
            cutoff: f32::INFINITY,
            wet: 1.0,
        }
    }

    /// How loud it is overall (1 for something at arm's length ahead).
    fn gain(&self) -> f32 {
        ((self.left * self.left + self.right * self.right) * 0.5).sqrt()
    }

    /// Heard from `at`: quieter and duller with distance (and a little duller from
    /// behind), panned by its bearing but never wholly into one ear, and more of it
    /// by way of the room the further it is.
    pub fn from(ears: &Ears, at: Vec3) -> Self {
        let rel = at - ears.pos;
        let d = rel.length();
        let fade = 1.0 - ((d - 40.0) / 20.0).clamp(0.0, 1.0);
        let gain = fade / (1.0 + (d - 1.5).max(0.0) / 5.0);
        let dir = rel / d.max(0.01);
        let width = (d / 1.5).min(1.0);
        let (l, r) = pan_gains(dir.dot(ears.right) * 0.8 * width);
        let behind = (-dir.dot(ears.ahead)).max(0.0) * width;
        Heard {
            left: l * SQRT_2 * gain,
            right: r * SQRT_2 * gain,
            cutoff: 16000.0 * 0.15f32.powf((d / 40.0).min(1.0)) * (1.0 - 0.4 * behind),
            wet: (0.6 + 1.4 * (d / 25.0).min(1.0)) * gain.max(0.15 * fade),
        }
    }
}

/// A clip playing.
pub struct Voice {
    clip: Clip,
    pos: f32,
    rate: f32,
    gain: f32,
    send: f32,
    at: Option<Vec3>,
    pan: f32,
    /// Samples before it starts.
    wait: usize,
    lp: OnePole,
    gains: (f32, f32),
    started: bool,
    /// Fading out, made way for: its gain now, falling to nothing.
    fading: Option<f32>,
}

impl Voice {
    /// `clip` at `gain` and playback `rate`, heard at the ears (panned) or from a
    /// place in the world, sending `send` of itself to the room.
    pub fn new(clip: Clip, gain: f32, rate: f32, send: f32, at: Option<Vec3>, pan: f32) -> Self {
        Voice {
            clip,
            pos: 0.0,
            rate,
            gain,
            send,
            at,
            pan,
            wait: 0,
            lp: OnePole::new(20000.0),
            gains: (0.0, 0.0),
            started: false,
            fading: None,
        }
    }

    /// Start `samples` into the coming block.
    pub fn after(mut self, samples: usize) -> Self {
        self.wait = samples;
        self
    }

    /// Roughly how loud it is now as heard: its gain where it is, less as it nears
    /// its end.
    fn level(&self, ears: &Ears) -> f32 {
        let heard = self.at.map_or(1.0, |at| Heard::from(ears, at).gain());
        self.gain * heard * (1.0 - self.pos / self.clip.len() as f32)
    }

    /// Make way: fade out over a few milliseconds and stop.
    fn fade_out(&mut self) {
        self.fading.get_or_insert(1.0);
    }

    /// Add this block to the dry pair and the room's send; false once it is done.
    fn render(&mut self, ears: &Ears, l: &mut [f32], r: &mut [f32], wet: &mut [f32]) -> bool {
        let h = match self.at {
            Some(at) => Heard::from(ears, at),
            None => Heard::near(self.pan),
        };
        if !self.started {
            self.gains = (h.left, h.right);
            self.started = true;
        }
        let filtered = h.cutoff.is_finite();
        if filtered {
            self.lp.set(h.cutoff);
        }
        let (gl0, gr0) = self.gains;
        let send = self.send * h.wet;
        let data = &self.clip;
        let last = data.len() - 1;
        let start = self.wait.min(l.len());
        self.wait -= start;
        let span = (l.len() - start) as f32;
        for i in start..l.len() {
            let idx = self.pos as usize;
            if idx >= last {
                self.gains = (h.left, h.right);
                return false;
            }
            let frac = self.pos - idx as f32;
            let mut x = (data[idx] + (data[idx + 1] - data[idx]) * frac) * self.gain;
            if let Some(f) = &mut self.fading {
                *f -= 1.0 / (0.005 * SRF);
                if *f <= 0.0 {
                    return false;
                }
                x *= *f;
            }
            if filtered {
                x = self.lp.lp(x);
            }
            let k = (i - start + 1) as f32 / span;
            l[i] += x * (gl0 + (h.left - gl0) * k);
            r[i] += x * (gr0 + (h.right - gr0) * k);
            wet[i] += x * send;
            self.pos += self.rate;
        }
        self.gains = (h.left, h.right);
        true
    }
}

/// A look-ahead peak limiter, then a soft knee for anything that still gets past.
struct Limiter {
    delay: [Vec<f32>; 2],
    idx: usize,
    env: f32,
    gain: f32,
}

impl Limiter {
    const LOOK: usize = 64;
    const CEILING: f32 = 0.5;

    fn new() -> Self {
        Limiter {
            delay: [vec![0.0; Self::LOOK], vec![0.0; Self::LOOK]],
            idx: 0,
            env: 0.0,
            gain: 1.0,
        }
    }

    fn process(&mut self, l: f32, r: f32) -> (f32, f32) {
        let release = 1.0 - 1.0 / (0.25 * SRF);
        let attack = 1.0 - (-4.0 / Self::LOOK as f32).exp();
        self.env = (self.env * release).max(l.abs().max(r.abs()));
        if self.env < 1e-9 {
            self.env = 0.0;
        }
        let target = if self.env > Self::CEILING {
            Self::CEILING / self.env
        } else {
            1.0
        };
        self.gain += (target - self.gain) * if target < self.gain { attack } else { 1.0 - release };
        let (dl, dr) = (self.delay[0][self.idx], self.delay[1][self.idx]);
        self.delay[0][self.idx] = l;
        self.delay[1][self.idx] = r;
        self.idx = (self.idx + 1) % Self::LOOK;
        (soft_clip(dl * self.gain), soft_clip(dr * self.gain))
    }
}

/// Unchanged below 0.9; above, eased so nothing passes full scale.
fn soft_clip(x: f32) -> f32 {
    let a = x.abs();
    if a <= 0.9 {
        x
    } else {
        x.signum() * (0.9 + 0.1 * ((a - 0.9) / 0.1).tanh())
    }
}

/// The whole mix.
pub struct Engine {
    rx: Receiver<Cmd>,
    banks: Option<Arc<Banks>>,
    rng: StdRng,
    /// Samples rendered so far.
    clock: u64,
    /// When each effect last started, and how crowded it has been lately.
    last: Vec<Option<(u64, f32)>>,
    /// Which foot fell last.
    foot: bool,
    voices: Vec<Voice>,
    scene: Scene,
    ears: Ears,
    music_gain: Smooth,
    effects_gain: Smooth,
    ambience: Ambience,
    music: Music,
    room: Reverb,
    limiter: Limiter,
    /// Scratch: the effects' dry pair and room send, and the music.
    fx: [Vec<f32>; 3],
    mus: [Vec<f32>; 2],
}

impl Engine {
    pub fn new(rx: Receiver<Cmd>, seed: u64) -> Self {
        let scene = Scene::default();
        Engine {
            rx,
            banks: None,
            rng: StdRng::seed_from_u64(seed),
            clock: 0,
            last: vec![None; Sfx::ALL.len()],
            foot: false,
            voices: Vec::new(),
            ears: Ears::new(scene.ears, scene.right),
            scene,
            music_gain: Smooth::new(0.0),
            effects_gain: Smooth::new(0.0),
            ambience: Ambience::new(seed),
            music: Music::new(seed.rotate_left(17)),
            room: Reverb::new(1.2, 0.4),
            limiter: Limiter::new(),
            fx: [vec![0.0; BLOCK], vec![0.0; BLOCK], vec![0.0; BLOCK]],
            mus: [vec![0.0; BLOCK], vec![0.0; BLOCK]],
        }
    }

    /// Take a command, refusing anything not finite: one NaN in a filter or the
    /// reverb would silence the mix for good.
    /// Take in what was asked while nothing played the mix: where the listener is,
    /// the gains, the clips. The effects asked for then are passed over, not played
    /// all at once when sound comes back.
    pub fn catch_up(&mut self) {
        while let Ok(c) = self.rx.try_recv() {
            if !matches!(c, Cmd::Play { .. }) {
                self.command(c);
            }
        }
    }

    fn command(&mut self, c: Cmd) {
        let finite = |v: Vec3| v.is_finite();
        match c {
            Cmd::Banks(b) => self.banks = Some(b),
            Cmd::Play { sfx, vol, variant, at } => {
                if vol.is_finite() && at.is_none_or(finite) {
                    self.play(sfx, vol, variant, at);
                }
            }
            Cmd::Scene(mut s) => {
                let fine = finite(s.ears)
                    && finite(s.right)
                    && s.fires.iter().all(|&(at, v)| finite(at) && v.is_finite())
                    && s.wings
                        .iter()
                        .flatten()
                        .all(|w| finite(w.at) && (w.phase + w.rate + w.flap).is_finite());
                if fine {
                    for w in &mut s.places {
                        *w = if w.is_finite() { w.clamp(0.0, 1.0) } else { 0.0 };
                    }
                    self.ears = Ears::new(s.ears, s.right);
                    self.scene = s;
                }
            }
            Cmd::Gains { music, effects } => {
                if music.is_finite() && effects.is_finite() {
                    self.music_gain.target = music.clamp(0.0, 1.0);
                    self.effects_gain.target = effects.clamp(0.0, 1.0);
                }
            }
        }
    }

    fn play(&mut self, sfx: Sfx, vol: f32, variant: Option<usize>, at: Option<Vec3>) {
        let Some(banks) = &self.banks else { return };
        let bank = &banks.sfx[sfx as usize];
        let p = props(sfx);
        let heard = at.map_or(1.0, |at| Heard::from(&self.ears, at).gain());
        if vol <= 0.0 || heard <= 0.0 {
            return;
        }
        // Only what is heard near at hand keeps others of its kind apart and thins
        // them out; far-off ones (a familiar filing across the library) neither.
        let gap = (p.gap * SRF) as u64;
        let last = &mut self.last[sfx as usize];
        let crowd = last.map_or(0.0, |(t, n)| n * (-((self.clock - t) as f32 / SRF) / CROWD_TAU).exp());
        if heard >= NEAR {
            if last.is_some_and(|(t, _)| self.clock < t + gap) {
                return;
            }
            *last = Some((self.clock, crowd + 1.0));
        }
        let clip = match variant {
            Some(v) => bank[v.min(bank.len() - 1)].clone(),
            None => bank.choose(&mut self.rng).unwrap().clone(),
        };
        let mut rate = 1.0 + self.rng.gen_range(-1.0..=1.0) * p.detune;
        if p.tonal {
            rate *= 2f32.powf(self.music.shift() as f32 / 12.0);
        }
        let pan = if sfx == Sfx::Step {
            // Footfalls a touch to one side and then the other.
            self.foot = !self.foot;
            if self.foot { 0.08 } else { -0.08 }
        } else {
            self.rng.gen_range(-1.0..=1.0) * p.spread
        };
        let live = self.voices.iter().filter(|v| v.fading.is_none());
        if live.count() >= MAX_VOICES {
            // Make way by fading out whichever is quietest by now.
            let ears = self.ears;
            let quietest = (self.voices.iter_mut())
                .filter(|v| v.fading.is_none())
                .min_by(|a, b| a.level(&ears).total_cmp(&b.level(&ears)));
            quietest.unwrap().fade_out();
        }
        let gain = vol * p.gain / (1.0 + p.crowd * crowd);
        self.voices.push(Voice::new(clip, gain, rate, p.send, at, pan));
    }

    /// Render the next block into `out`, interleaved stereo.
    pub fn render(&mut self, out: &mut [f32]) {
        while let Ok(c) = self.rx.try_recv() {
            self.command(c);
        }
        for b in &mut self.fx {
            b.fill(0.0);
        }
        let shift = 2f32.powf(self.music.shift() as f32 / 12.0);
        (self.ambience).render(
            &self.scene,
            self.banks.as_deref(),
            shift,
            &mut self.rng,
            &mut self.voices,
            &mut self.fx,
        );
        let [l, r, wet] = &mut self.fx;
        let ears = self.ears;
        self.voices.retain_mut(|v| v.render(&ears, l, r, wet));
        let (rt60, damp) = self.ambience.acoustics();
        self.room.set(rt60, damp);
        self.room.process(wet, l, r);

        let (m0, m1) = self.music_gain.step(DT, 0.4);
        let [ml, mr] = &mut self.mus;
        ml.fill(0.0);
        mr.fill(0.0);
        if m0 > 0.0 || m1 > 0.0 {
            self.music.render(ml, mr);
        }
        let (e0, e1) = self.effects_gain.step(DT, 0.1);
        for i in 0..BLOCK {
            let k = (i + 1) as f32 / BLOCK as f32;
            let (e, m) = (e0 + (e1 - e0) * k, m0 + (m1 - m0) * k);
            let (a, b) = self.limiter.process(l[i] * e + ml[i] * m, r[i] * e + mr[i] * m);
            out[2 * i] = a;
            out[2 * i + 1] = b;
        }
        self.clock += BLOCK as u64;
    }
}
