//! Checks on the clips and the mix, and an export of every sound for listening:
//! `HUSHFOLD_SOUND_DIR=dir cargo test --release export_sounds -- --ignored`.

use super::clips::{Banks, GRAINS};
use super::dsp::*;
use super::engine::{BLOCK, Cmd, Engine};
use super::*;
use rand::prelude::*;
use std::path::Path;

/// An engine rendering offline, its clips ready.
struct Offline {
    engine: Engine,
    tx: SyncSender<Cmd>,
}

impl Offline {
    fn new(seed: u64, banks: &Arc<Banks>, music: f32) -> Self {
        let (tx, rx) = sync_channel(1024);
        let mut o = Offline {
            engine: Engine::new(rx, seed),
            tx,
        };
        o.send(Cmd::Banks(banks.clone()));
        o.send(Cmd::Gains { music, effects: 1.0 });
        o.render(0.05);
        o
    }

    fn send(&mut self, c: Cmd) {
        self.tx.send(c).unwrap();
    }

    fn scene(&mut self, scene: Scene) {
        self.send(Cmd::Scene(scene));
    }

    fn play(&mut self, sfx: Sfx, vol: f32, variant: Option<usize>, at: Option<Vec3>) {
        self.send(Cmd::Play { sfx, vol, variant, at });
    }

    /// `secs` of the mix, interleaved stereo.
    fn render(&mut self, secs: f32) -> Vec<f32> {
        let blocks = (secs * SRF / BLOCK as f32).ceil() as usize;
        let mut out = vec![0.0; blocks * BLOCK * 2];
        for chunk in out.chunks_mut(BLOCK * 2) {
            self.engine.render(chunk);
        }
        out
    }
}

/// A scene in one place, standing at the origin facing -z.
fn scene_in(place: Place) -> Scene {
    let mut s = Scene::default();
    s.places[place as usize] = 1.0;
    s
}

/// A familiar flying steadily, its wings beating as the game draws them: bouts of a
/// few beats with a glide between (a shelver's or a courier's).
struct Flier {
    beat: Wingbeat,
    /// Beats a second while beating.
    rate: f32,
    glide: f32,
    beats: u32,
    rng: StdRng,
}

impl Flier {
    fn new(id: usize, courier: bool) -> Self {
        let rate = if courier { 2.95 } else { 1.93 };
        Flier {
            beat: Wingbeat {
                id,
                at: Vec3::ZERO,
                phase: 0.0,
                rate,
                flap: 0.65,
                courier,
            },
            rate,
            glide: 0.0,
            beats: 3,
            rng: StdRng::seed_from_u64(id as u64),
        }
    }

    /// `dt` seconds on, at `at`.
    fn step(&mut self, dt: f32, at: Vec3) -> Wingbeat {
        let b = &mut self.beat;
        b.at = at;
        if self.glide > 0.0 {
            self.glide -= dt;
            b.flap -= b.flap * (1.0 - (-dt * 8.0).exp());
            return *b;
        }
        b.rate = self.rate;
        b.flap += (0.65 - b.flap) * (1.0 - (-dt * 8.0).exp());
        b.phase += b.rate * dt;
        if b.phase >= 1.0 {
            b.phase = b.phase.fract();
            if self.beats == 0 {
                let (short, long) = if b.courier { (0.35, 0.8) } else { (0.6, 1.3) };
                self.glide = self.rng.gen_range(short..long);
                self.beats = self.rng.gen_range(2..5);
                b.phase = 0.0;
                b.rate = 0.0;
            } else {
                self.beats -= 1;
            }
        }
        *b
    }
}

/// `secs` of familiars flying round the listener at the origin, each from where `at`
/// puts it at a time.
fn fliers(seed: u64, banks: &Arc<Banks>, secs: f32, flying: &mut [(Flier, &dyn Fn(f32) -> Vec3)]) -> Vec<f32> {
    let mut o = Offline::new(seed, banks, 0.0);
    let mut out = Vec::new();
    let dt = 1.0 / 60.0;
    for k in 0..(secs / dt) as usize {
        let t = k as f32 * dt;
        let mut s = Scene::default();
        for (slot, (f, at)) in s.wings.iter_mut().zip(flying.iter_mut()) {
            *slot = Some(f.step(dt, at(t)));
        }
        o.scene(s);
        out.extend(o.render(dt));
    }
    out
}

fn check_clip(name: &str, buf: &[f32]) {
    assert!(buf.len() > 10, "{name} is empty");
    assert!(buf.iter().all(|x| x.is_finite()), "{name} has non-finite samples");
    let p = peak(buf);
    assert!(p > 0.5 && p <= 1.0, "{name} peaks at {p}");
    let ends = (buf[0].abs(), buf[buf.len() - 1].abs());
    assert!(
        ends.0 < 0.02 && ends.1 < 0.02,
        "{name} starts or ends with a step: {ends:?}"
    );
    let dc = buf.iter().sum::<f32>() / buf.len() as f32;
    assert!(dc.abs() < 0.01, "{name} is off centre by {dc}");
}

#[test]
fn every_clip_is_sane() {
    let banks = Banks::synthesize();
    for (s, bank) in Sfx::ALL.iter().zip(&banks.sfx) {
        assert!(!bank.is_empty(), "{s:?} has no clips");
        for (i, c) in bank.iter().enumerate() {
            check_clip(&format!("{s:?} {i}"), c);
        }
    }
    for (g, bank) in GRAINS.iter().zip(&banks.grains) {
        assert!(!bank.is_empty(), "{g:?} has no clips");
        for (i, c) in bank.iter().enumerate() {
            check_clip(&format!("{g:?} {i}"), c);
        }
    }
    assert_eq!(banks.sfx[Sfx::Spell as usize].len(), crate::game::SPELLS.len());
}

#[test]
fn the_mix_stays_in_bounds_and_moves_smoothly() {
    let banks = Arc::new(Banks::synthesize());
    let mut o = Offline::new(3, &banks, 1.0);
    // Every place in turn, a pile-up of effects in each.
    for place in Place::ALL {
        let mut scene = scene_in(place);
        scene.fires[0] = (Vec3::new(2.0, 0.5, -3.0), 1.0);
        scene.wings[0] = Some(Flier::new(0, false).step(0.1, Vec3::new(-1.0, 1.8, -2.0)));
        o.scene(scene);
        for i in 0..20 {
            o.play(Sfx::Thunk, 0.5, None, Some(Vec3::new(i as f32 * 0.4 - 4.0, 1.0, -1.5)));
            o.play(Sfx::Rustle, 0.6, None, None);
            o.render(0.02);
        }
        o.play(Sfx::Fanfare, 0.7, None, None);
        o.play(Sfx::Veil, 0.8, None, None);
        o.play(Sfx::Chime, 0.45, Some(7), None);
        let out = o.render(2.0);
        assert!(out.iter().all(|x| x.is_finite()), "{place:?}: non-finite");
        let p = peak(&out);
        assert!(p <= 1.0, "{place:?}: peak {p}");
    }
    // Wandering between places with nothing else playing: the crossfades and the
    // beds themselves never jump.
    let mut o = Offline::new(4, &banks, 0.0);
    let mut worst = 0.0f32;
    for k in 0..40 {
        o.scene(scene_in(Place::ALL[k % Place::ALL.len()]));
        let out = o.render(0.5);
        let quiet = out.chunks(2).map(|f| f[0]).collect::<Vec<_>>();
        let rms = (quiet.iter().map(|x| x * x).sum::<f32>() / quiet.len() as f32).sqrt();
        let jump = quiet.windows(2).map(|w| (w[1] - w[0]).abs()).fold(0.0, f32::max);
        worst = worst.max(jump / rms.max(1e-4));
    }
    assert!(worst < 40.0, "a jump {worst}× the bed's RMS");
}

#[test]
fn far_off_sounds_fade_out_of_the_room_too() {
    let ears = engine::Ears::new(Vec3::ZERO, Vec3::X);
    let near = engine::Heard::from(&ears, Vec3::new(0.0, 0.0, -2.0));
    let far = engine::Heard::from(&ears, Vec3::new(0.0, 0.0, -70.0));
    assert!(near.left > 0.5 && near.wet > 0.3);
    assert_eq!((far.left, far.right, far.wet), (0.0, 0.0, 0.0));
}

#[test]
fn nothing_not_finite_gets_into_the_mix() {
    let banks = Arc::new(Banks::synthesize());
    let mut o = Offline::new(12, &banks, 1.0);
    let mut s = scene_in(Place::Arcanum);
    o.scene(s);
    o.render(1.0);
    s.places[Place::Hall as usize] = f32::NAN;
    o.scene(s);
    o.play(Sfx::Thunk, f32::NAN, None, None);
    o.play(Sfx::Land, 0.4, None, Some(Vec3::NAN));
    o.send(Cmd::Gains {
        music: f32::INFINITY,
        effects: 1.0,
    });
    o.play(Sfx::Chime, 0.45, Some(3), None);
    let out = o.render(3.0);
    assert!(out.iter().all(|x| x.is_finite()));
    assert!(peak(&out[out.len() / 2..]) > 1e-4, "the mix fell silent");
}

// ---------------------------------------------------------------- export

fn write_wav(path: &Path, channels: u16, buf: &[f32]) {
    let mut b = Vec::with_capacity(44 + buf.len() * 4);
    let data = (buf.len() * 4) as u32;
    b.extend_from_slice(b"RIFF");
    b.extend_from_slice(&(36 + data).to_le_bytes());
    b.extend_from_slice(b"WAVEfmt ");
    b.extend_from_slice(&16u32.to_le_bytes());
    b.extend_from_slice(&3u16.to_le_bytes());
    b.extend_from_slice(&channels.to_le_bytes());
    b.extend_from_slice(&SR.to_le_bytes());
    b.extend_from_slice(&(SR * 4 * channels as u32).to_le_bytes());
    b.extend_from_slice(&(4 * channels).to_le_bytes());
    b.extend_from_slice(&32u16.to_le_bytes());
    b.extend_from_slice(b"data");
    b.extend_from_slice(&data.to_le_bytes());
    for x in buf {
        b.extend_from_slice(&x.to_le_bytes());
    }
    std::fs::write(path, b).unwrap();
}

/// Each effect's usual volume at its call sites.
fn usual_volume(s: Sfx) -> f32 {
    match s {
        Sfx::Thunk => 0.5,
        Sfx::Rustle => 0.6,
        Sfx::Whoosh => 0.35,
        Sfx::Toss => 0.45,
        Sfx::Land => 0.3,
        Sfx::Chime => 0.45,
        Sfx::Fanfare => 0.7,
        Sfx::Veil => 0.8,
        Sfx::Purchase => 0.7,
        Sfx::Spell => 0.5,
        Sfx::Unfold => 0.3,
        Sfx::Fold => 0.4,
        Sfx::Nope => 0.3,
        Sfx::Tick => 0.25,
        Sfx::Step => 0.12,
        Sfx::Hop => 0.3,
        Sfx::Alight => 0.5,
        Sfx::Wade => 0.2,
    }
}

#[test]
#[ignore]
fn export_sounds() {
    let dir = std::path::PathBuf::from(std::env::var("HUSHFOLD_SOUND_DIR").expect("HUSHFOLD_SOUND_DIR"));
    std::fs::create_dir_all(&dir).unwrap();
    let banks = Arc::new(Banks::synthesize());
    // Every clip, dry and mono, at its usual level in the mix.
    for (&s, bank) in Sfx::ALL.iter().zip(&banks.sfx) {
        for (i, c) in bank.iter().enumerate() {
            let g = usual_volume(s) * engine::gain(s);
            let v: Vec<f32> = c.iter().map(|x| x * g).collect();
            write_wav(&dir.join(format!("clip_{s:?}_{i:02}.wav").to_lowercase()), 1, &v);
        }
    }
    for (g, bank) in GRAINS.iter().zip(&banks.grains) {
        for (i, c) in bank.iter().enumerate() {
            write_wav(&dir.join(format!("grain_{g:?}_{i:02}.wav").to_lowercase()), 1, c);
        }
    }
    // Each effect through the mix, its variants in turn, in a quiet room: those that
    // happen in the world from a couple of metres off, a little to the right.
    for (&s, bank) in Sfx::ALL.iter().zip(&banks.sfx) {
        let mut o = Offline::new(1, &banks, 0.0);
        o.scene(scene_in(Place::Room));
        o.render(1.0);
        let mut out = Vec::new();
        let gap = match s {
            Sfx::Fanfare | Sfx::Veil | Sfx::Spell | Sfx::Purchase => 3.5,
            Sfx::Chime | Sfx::Unfold => 1.4,
            Sfx::Step => 0.45,
            _ => 0.6,
        };
        let world = matches!(s, Sfx::Thunk | Sfx::Land);
        for i in 0..bank.len().min(12) {
            let at = world.then(|| Vec3::new(0.8 + (i % 3) as f32 * 0.4, 1.0, -2.0));
            let variant = matches!(s, Sfx::Chime | Sfx::Spell | Sfx::Fanfare | Sfx::Purchase).then_some(i);
            o.play(s, usual_volume(s), variant, at);
            out.extend(o.render(gap));
        }
        out.extend(o.render(1.5));
        write_wav(&dir.join(format!("sfx_{s:?}.wav").to_lowercase()), 2, &out);
    }
    // Each place's ambience, with its fire if it has one, from inside it.
    for place in Place::ALL {
        let mut o = Offline::new(2, &banks, 0.0);
        let mut scene = scene_in(place);
        if matches!(place, Place::Hall | Place::AlchemicalWing | Place::HearthLibrary) {
            scene.fires[0] = (Vec3::new(3.0, 0.8, -4.0), 1.0);
        }
        o.scene(scene);
        o.render(2.0);
        write_wav(
            &dir.join(format!("ambience_{place:?}.wav").to_lowercase()),
            2,
            &o.render(22.0),
        );
    }
    // Walking from the atrium into the Clockworks and on into the Weatherhouse.
    let mut o = Offline::new(5, &banks, 0.0);
    let mut out = Vec::new();
    for k in 0..=60 {
        let u = k as f32 / 60.0;
        let mut s = Scene::default();
        let (a, b, c) = (
            (1.0 - u * 2.0).clamp(0.0, 1.0),
            1.0 - (u * 2.0 - 1.0).abs(),
            (u * 2.0 - 1.0).clamp(0.0, 1.0),
        );
        s.places[Place::Atrium as usize] = a;
        s.places[Place::Clockworks as usize] = b;
        s.places[Place::Weatherhouse as usize] = c;
        o.scene(s);
        out.extend(o.render(0.5));
    }
    write_wav(&dir.join("walk_atrium_clockworks_weatherhouse.wav"), 2, &out);
    // A shelver circling the listener 3 m off, a courier, and the two side by side,
    // passing to and fro on either hand.
    let round = |t: f32| Vec3::new(3.0 * t.sin(), 1.8, -3.0 * t.cos());
    for (name, courier) in [("familiar_circling", false), ("familiar_courier_circling", true)] {
        let out = fliers(6, &banks, 15.0, &mut [(Flier::new(0, courier), &round)]);
        write_wav(&dir.join(format!("{name}.wav")), 2, &out);
    }
    let left = |t: f32| Vec3::new(-2.5, 1.8, -4.0 * (t * 0.5).sin());
    let right = |t: f32| Vec3::new(2.5, 1.8, 4.0 * (t * 0.45).sin());
    let both: &mut [(Flier, &dyn Fn(f32) -> Vec3)] =
        &mut [(Flier::new(0, false), &left), (Flier::new(1, true), &right)];
    write_wav(
        &dir.join("familiars_side_by_side.wav"),
        2,
        &fliers(6, &banks, 15.0, both),
    );
    // Busy shelving in the Hall: a Shelving Aura's thunks all round, books taken up
    // and tossed, chimes, steps, and the music.
    let busy = |gain: f32| {
        let mut o = Offline::new(7, &banks, 0.8);
        o.send(Cmd::Gains {
            music: 0.8 * gain,
            effects: gain,
        });
        let mut scene = scene_in(Place::Hall);
        scene.fires[0] = (Vec3::new(0.0, 0.8, -12.0), 1.0);
        o.scene(scene);
        let mut rng = StdRng::seed_from_u64(9);
        let mut out = o.render(12.0);
        for k in 0..400 {
            let at = Vec3::new(
                rng.gen_range(-5.0..5.0),
                rng.gen_range(0.2..3.0),
                rng.gen_range(-5.0..5.0),
            );
            o.play(Sfx::Thunk, 0.5, None, Some(at));
            if k % 8 == 0 {
                o.play(Sfx::Chime, 0.45, Some(rng.gen_range(2..12)), None);
            }
            if k % 30 == 0 {
                o.play(Sfx::Rustle, 0.6, None, None);
            }
            if k % 3 == 0 {
                o.play(Sfx::Step, 0.12, None, None);
            }
            if k == 200 {
                o.play(Sfx::Fanfare, 0.7, None, None);
                o.play(Sfx::Veil, 0.8, None, None);
            }
            out.extend(o.render(0.125));
        }
        out
    };
    write_wav(&dir.join("busy_hall.wav"), 2, &busy(1.0));
    write_wav(&dir.join("busy_hall_quarter.wav"), 2, &busy(0.25));
    // The Rush spell: thirty books a second filed all round, series chiming home.
    let rush = |gain: f32| {
        let mut o = Offline::new(8, &banks, 0.8);
        o.send(Cmd::Gains {
            music: 0.8 * gain,
            effects: gain,
        });
        o.scene(scene_in(Place::Clockworks));
        let mut rng = StdRng::seed_from_u64(10);
        let mut out = o.render(14.0);
        o.play(Sfx::Spell, 0.5, Some(4), None);
        for k in 0..240 {
            let at = Vec3::new(
                rng.gen_range(-6.0..6.0),
                rng.gen_range(0.2..3.0),
                rng.gen_range(-6.0..6.0),
            );
            o.play(Sfx::Thunk, 0.5, None, Some(at));
            if k % 5 == 0 {
                o.play(Sfx::Chime, 0.45, Some(rng.gen_range(2..12)), None);
            }
            out.extend(o.render(1.0 / 30.0));
        }
        out.extend(o.render(4.0));
        out
    };
    write_wav(&dir.join("rush.wav"), 2, &rush(1.0));
    write_wav(&dir.join("rush_quarter.wav"), 2, &rush(0.25));
    // The music: ten minutes, pieces and the quiet between them.
    let mut o = Offline::new(11, &banks, 0.8);
    write_wav(&dir.join("music.wav"), 2, &o.render(600.0));
}
