//! Where the sound goes. A thread of its own keeps a stream open on the output
//! chosen in the settings (or the system's), fed by the engine, and opens it again
//! when the choice changes or the output is lost; the game never waits on a device.
//!
//! On Linux the game is a client of the sound server (PulseAudio, or PipeWire
//! standing in for it), so it is a stream there like any other: named, on the
//! system's output unless another is chosen, movable from a mixer. Elsewhere, or
//! with no server, cpal opens a device (WASAPI, CoreAudio, ALSA);
//! `HUSHFOLD_AUDIO=device` takes that way on Linux too.

use super::dsp::SR;
use super::engine::{BLOCK, Engine};
use serde::{Deserialize, Serialize};
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender, channel};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};

mod device;
#[cfg(target_os = "linux")]
mod server;

/// An output the player can choose.
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Device {
    /// What the choice is kept as: the sound server's name for it, or cpal's id.
    pub id: String,
    /// As the player sees it.
    pub name: String,
}

/// Where the sound is playing.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Playing {
    /// Nowhere: no output would open (tried again every couple of seconds).
    #[default]
    Nowhere,
    /// On the output chosen.
    Chosen,
    /// On the system's: none was chosen, or the one chosen isn't there.
    System,
}

/// The outputs, as the settings screen shows them.
#[derive(Clone, Default, Debug)]
pub struct Outputs {
    /// Those there are, as last listed (`Output::list`).
    pub devices: Vec<Device>,
    /// The system's output, by name, when known.
    pub system: Option<String>,
    pub playing: Playing,
}

/// The engine, read a frame at a time by an output asking for any length.
pub struct Feed {
    engine: Engine,
    block: [f32; BLOCK * 2],
    at: usize,
    /// When a block was last rendered.
    rendered: Instant,
    /// Read by an output as it plays, and so to catch up after a while unheard;
    /// not when taped (`Audio::taped`), read in step with the game however slowly.
    live: bool,
}

/// Longer than any output waits between asks: nothing is playing the mix.
const UNHEARD: Duration = Duration::from_millis(500);

impl Feed {
    pub fn new(engine: Engine) -> Self {
        Feed {
            engine,
            block: [0.0; BLOCK * 2],
            at: BLOCK * 2,
            rendered: Instant::now(),
            live: true,
        }
    }

    pub fn taped(engine: Engine) -> Self {
        Feed {
            live: false,
            ..Feed::new(engine)
        }
    }

    /// The next frame, left and right, at `SR`.
    pub fn frame(&mut self) -> [f32; 2] {
        if self.at == self.block.len() {
            if self.live {
                self.idle();
            }
            self.engine.render(&mut self.block);
            self.rendered = Instant::now();
            self.at = 0;
        }
        self.at += 2;
        [self.block[self.at - 2], self.block[self.at - 1]]
    }

    /// While nothing plays the mix, keep up with what the game asks of it (and so
    /// keep its queue from filling), passing over the effects.
    pub fn idle(&mut self) {
        if self.rendered.elapsed() > UNHEARD {
            self.engine.catch_up();
        }
    }
}

/// Shared between the stream's callbacks: one stream takes from it at a time, but
/// one being replaced may be called once more.
type Shared<T> = Arc<Mutex<T>>;

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

/// From `SR` to a device's rate, by cubic (Catmull-Rom) interpolation between the
/// frames about each one; at the same rate it passes them through.
pub struct Resample {
    /// Frames in per frame out.
    step: f64,
    /// How far past `near[1]` the next frame out lies, in frames in.
    frac: f64,
    near: [[f32; 2]; 4],
}

impl Resample {
    pub fn new(rate: u32) -> Self {
        Resample {
            step: f64::from(SR) / f64::from(rate),
            frac: 0.0,
            near: [[0.0; 2]; 4],
        }
    }

    pub fn frame(&mut self, feed: &mut Feed) -> [f32; 2] {
        while self.frac >= 1.0 {
            self.near.rotate_left(1);
            self.near[3] = feed.frame();
            self.frac -= 1.0;
        }
        let t = self.frac as f32;
        let [a, b, c, d] = self.near;
        self.frac += self.step;
        std::array::from_fn(|i| {
            let (p0, p1, p2, p3) = (a[i], b[i], c[i], d[i]);
            p1 + 0.5 * t * (p2 - p0 + t * (2.0 * p0 - 5.0 * p1 + 4.0 * p2 - p3 + t * (3.0 * (p1 - p2) + p3 - p0)))
        })
    }
}

/// An open stream, whichever way it was opened; dropping it closes it.
trait Stream {
    /// It has stopped for good (its device or server gone).
    fn lost(&self) -> bool;
    /// The system output it was opened on, by id, when it stays there though the
    /// system's output changes (a server moves its streams itself).
    fn system(&self) -> Option<&str> {
        None
    }
}

/// The way to the outputs, picked once.
enum Way {
    #[cfg(target_os = "linux")]
    Server(Option<server::Server>),
    Device(device::Devices),
}

impl Way {
    fn pick() -> Way {
        #[cfg(target_os = "linux")]
        if std::env::var("HUSHFOLD_AUDIO").as_deref() != Ok("device") {
            match server::Server::connect() {
                Ok(s) => return Way::Server(Some(s)),
                Err(e) => eprintln!("hushfold: no sound server ({e}); opening a device instead"),
            }
        }
        Way::Device(device::Devices::new())
    }

    /// The outputs, and the system's by name.
    fn list(&mut self) -> (Vec<Device>, Option<String>) {
        match self {
            #[cfg(target_os = "linux")]
            Way::Server(s) => s.as_ref().map(server::Server::list).unwrap_or_default(),
            Way::Device(d) => d.list(),
        }
    }

    fn has(&mut self, id: &str) -> bool {
        match self {
            #[cfg(target_os = "linux")]
            Way::Server(s) => s.as_ref().is_some_and(|s| s.has(id)),
            Way::Device(d) => d.has(id),
        }
    }

    /// The system's output, by id, as `Stream::system` gives it.
    fn system(&self) -> Option<String> {
        match self {
            #[cfg(target_os = "linux")]
            Way::Server(_) => None,
            Way::Device(d) => d.system_id(),
        }
    }

    /// A stream on `id` (None: the system's output).
    fn open(&mut self, id: Option<&str>, feed: &Shared<Feed>) -> Result<Box<dyn Stream>, String> {
        match self {
            #[cfg(target_os = "linux")]
            Way::Server(s) => {
                if s.as_ref().is_none_or(server::Server::spoiled) {
                    *s = Some(server::Server::connect()?);
                }
                let opened = s.as_ref().map(|s| s.open(id, feed.clone()));
                let opened = opened.unwrap_or_else(|| Err("no sound server".into()));
                if opened.is_err() {
                    // Perhaps the server went; connect again next time.
                    *s = None;
                }
                opened
            }
            Way::Device(d) => d.open(id, feed),
        }
    }

    /// The stream was lost (its connection with it, for a server).
    fn lost(&mut self) {
        #[cfg(target_os = "linux")]
        if let Way::Server(s) = self {
            *s = None;
        }
    }
}

enum Ctl {
    Choose(Option<String>),
    List,
}

/// How often the stream is looked at: whether it was lost, whether the output
/// chosen has come or gone, whether the system's output changed.
const LOOK: Duration = Duration::from_secs(2);

/// The game's end of the sound thread.
pub struct Output {
    ctl: Sender<Ctl>,
    outputs: Shared<Outputs>,
    listed: Option<Instant>,
}

impl Output {
    pub fn start(engine: Engine, chosen: Option<String>) -> Output {
        let (ctl, rx) = channel();
        let outputs = Shared::default();
        let shown = outputs.clone();
        let feed = Arc::new(Mutex::new(Feed::new(engine)));
        let spawned = std::thread::Builder::new()
            .name("sound".into())
            .spawn(move || run(rx, feed, shown, chosen));
        if let Err(e) = spawned {
            eprintln!("hushfold: no sound ({e})");
        }
        Output {
            ctl,
            outputs,
            listed: None,
        }
    }

    pub fn choose(&self, chosen: Option<String>) {
        let _ = self.ctl.send(Ctl::Choose(chosen));
    }

    pub fn list(&mut self) {
        if self.listed.is_none_or(|t| t.elapsed() >= LOOK) {
            self.listed = Some(Instant::now());
            let _ = self.ctl.send(Ctl::List);
        }
    }

    pub fn outputs(&self) -> Outputs {
        lock(&self.outputs).clone()
    }
}

fn run(ctl: Receiver<Ctl>, feed: Shared<Feed>, outputs: Shared<Outputs>, mut chosen: Option<String>) {
    let mut way = Way::pick();
    let mut stream: Option<Box<dyn Stream>> = None;
    let mut next = Instant::now();
    // Whether the output chosen was there when last looked for: it is opened again
    // when it comes back, not every look while it is there but won't open (busy, or
    // held by another program).
    let mut there: Option<bool> = None;
    // The last trouble told, so it is told once.
    let mut told = String::new();
    let mut tell = |e: String| {
        if e != told {
            eprintln!("hushfold: sound: {e}");
            told = e;
        }
    };
    loop {
        lock(&feed).idle();
        if Instant::now() >= next {
            next = Instant::now() + LOOK;
            let playing = lock(&outputs).playing;
            let was_there = there;
            there = chosen.as_deref().map(|id| way.has(id));
            let open = match &stream {
                None => true,
                Some(s) if s.lost() => {
                    way.lost();
                    true
                }
                Some(s) => {
                    let moved = s.system().is_some_and(|id| way.system().as_deref() != Some(id));
                    // The output chosen came back while on the system's, or went while on it.
                    let back = playing == Playing::System && there == Some(true) && was_there == Some(false);
                    let gone = playing == Playing::Chosen && there == Some(false);
                    moved || back || gone
                }
            };
            if open {
                // Closed before the next is opened, so the two never both play.
                stream = None;
                let mut playing = Playing::Nowhere;
                if let Some(id) = &chosen {
                    match way.open(Some(id), &feed) {
                        Ok(s) => (stream, playing) = (Some(s), Playing::Chosen),
                        Err(e) => tell(format!("can't open the output chosen ({e}); trying the system's")),
                    }
                }
                if stream.is_none() {
                    match way.open(None, &feed) {
                        Ok(s) => (stream, playing) = (Some(s), Playing::System),
                        Err(e) => tell(format!("can't open the system's output ({e})")),
                    }
                }
                lock(&outputs).playing = playing;
            }
        }
        match ctl.recv_timeout(next.saturating_duration_since(Instant::now())) {
            Ok(Ctl::Choose(c)) => {
                if c != chosen {
                    chosen = c;
                    stream = None;
                    there = None;
                    next = Instant::now();
                }
            }
            Ok(Ctl::List) => {
                let (devices, system) = way.list();
                let mut o = lock(&outputs);
                (o.devices, o.system) = (devices, system);
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => return,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc::sync_channel;

    fn feed() -> Feed {
        let (_tx, rx) = sync_channel(1);
        Feed::new(Engine::new(rx, 1))
    }

    /// A feed whose engine's block is a ramp, frame `n` holding `n` (and `-n`).
    fn ramp() -> Feed {
        let mut f = feed();
        for (i, s) in f.block.iter_mut().enumerate() {
            *s = if i % 2 == 0 { (i / 2) as f32 } else { -((i / 2) as f32) };
        }
        f.at = 0;
        f
    }

    /// At the engine's own rate the frames come through as they were, three late.
    #[test]
    fn the_same_rate_passes_through() {
        let mut f = ramp();
        let mut r = Resample::new(SR);
        let out: Vec<[f32; 2]> = (0..12).map(|_| r.frame(&mut f)).collect();
        assert_eq!(out[..3], [[0.0, 0.0]; 3]);
        for (n, o) in out[3..].iter().enumerate() {
            assert_eq!(*o, [n as f32, -(n as f32)]);
        }
    }

    /// To 48 kHz a ramp stays a ramp, its step shrunk by the rates' ratio (a cubic
    /// through points on a line lies on it).
    #[test]
    fn a_ramp_is_resampled_onto_its_line() {
        let mut f = ramp();
        let mut r = Resample::new(48_000);
        let out: Vec<f32> = (0..200).map(|_| r.frame(&mut f)[0]).collect();
        let step = SR as f32 / 48_000.0;
        // Past the start, where the zeros before the ramp bend it.
        for w in out[8..].windows(2) {
            assert!((w[1] - w[0] - step).abs() < 1e-3, "{} -> {}", w[0], w[1]);
        }
    }

    /// A tone keeps its pitch: 1 kHz at 44.1 kHz resampled to 48 kHz crosses zero
    /// as often per second, and its peaks keep their height.
    #[test]
    fn a_tone_keeps_its_pitch_and_level() {
        let mut f = feed();
        let mut r = Resample::new(48_000);
        let tone = |n: usize| (std::f32::consts::TAU * 1000.0 * n as f32 / SR as f32).sin();
        let mut n = 0;
        let mut out = Vec::new();
        for _ in 0..48_000 {
            if f.at == f.block.len() {
                for s in f.block.chunks_mut(2) {
                    s.fill(tone(n));
                    n += 1;
                }
                f.at = 0;
            }
            out.push(r.frame(&mut f)[0]);
        }
        let ups = out.windows(2).filter(|w| w[0] < 0.0 && w[1] >= 0.0).count();
        assert!((999..=1001).contains(&ups), "{ups} rising crossings in a second");
        let peak = out.iter().fold(0.0f32, |m, s| m.max(s.abs()));
        assert!((0.99..=1.01).contains(&peak), "peak {peak}");
    }
}
