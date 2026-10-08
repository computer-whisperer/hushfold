mod audio;
mod catalog;
mod game;
mod gfx;
mod hud;
mod library;

use audio::Audio;
use game::{Game, Input};
use gfx::{FrameInput, PostParams, Renderer};
use hud::{Hud, HudAction};
use std::collections::HashSet;
use std::sync::Arc;
use std::time::Instant;
use winit::application::ApplicationHandler;
use winit::event::{DeviceEvent, DeviceId, ElementState, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{CursorGrabMode, Fullscreen, Window, WindowId};

const MOUSE_SENS: f32 = 0.0022;
const AUTOSAVE_SECS: f32 = 20.0;

struct Running {
    renderer: Renderer,
    egui_ctx: egui::Context,
    egui_state: egui_winit::State,
    game: Game,
    audio: Audio,
    hud: Hud,
    input: Input,
    keys: HashSet<KeyCode>,
    grabbed: bool,
    last: Instant,
    save_timer: f32,
    fps_acc: (f32, u32),
    script: Option<Script>,
}

/// A scripted camera/action sequence for screenshots and smoke tests
/// (`HUSHFOLD_SCRIPT=file HUSHFOLD_SHOT_DIR=dir`).
struct Script {
    lines: Vec<String>,
    idx: usize,
    wait: u32,
    dir: std::path::PathBuf,
}

#[derive(Default)]
struct App {
    run: Option<Running>,
}

impl Running {
    fn set_grab(&mut self, grab: bool) {
        let w = &self.renderer.window;
        if grab {
            let ok = w
                .set_cursor_grab(CursorGrabMode::Locked)
                .or_else(|_| w.set_cursor_grab(CursorGrabMode::Confined))
                .is_ok();
            w.set_cursor_visible(false);
            self.grabbed = ok;
            if !ok {
                // Still allow play without a grab; the cursor is merely visible.
                self.grabbed = true;
            }
        } else {
            let _ = w.set_cursor_grab(CursorGrabMode::None);
            w.set_cursor_visible(true);
            self.grabbed = false;
        }
        self.input.primary_down = false;
    }

    fn playing(&self) -> bool {
        (self.grabbed || self.script.is_some()) && !self.hud.menu_open && !self.hud.show_intro
    }

    fn on_key(&mut self, code: KeyCode, pressed: bool, event_loop: &ActiveEventLoop) {
        if pressed {
            if self.keys.contains(&code) {
                return; // key repeat
            }
            self.keys.insert(code);
        } else {
            self.keys.remove(&code);
            return;
        }
        match code {
            KeyCode::Tab => {
                if self.hud.show_intro {
                    return;
                }
                self.hud.menu_open = !self.hud.menu_open;
                self.set_grab(!self.hud.menu_open);
            }
            KeyCode::Escape => {
                if self.hud.menu_open {
                    self.hud.menu_open = false;
                    self.set_grab(true);
                } else if self.hud.show_intro {
                    if self.game.total_shelved() > 0 || self.game.held.len() > 0 {
                        self.hud.show_intro = false;
                        self.set_grab(true);
                    }
                } else {
                    self.hud.show_intro = true;
                    self.set_grab(false);
                    self.game.save();
                }
            }
            KeyCode::KeyM => self.audio.toggle_music(),
            KeyCode::F3 => self.hud.show_stats = !self.hud.show_stats,
            KeyCode::F11 => {
                let w = &self.renderer.window;
                if w.fullscreen().is_some() {
                    w.set_fullscreen(None);
                } else {
                    w.set_fullscreen(Some(Fullscreen::Borderless(None)));
                }
            }
            KeyCode::KeyQ => self.input.cycle -= 1,
            KeyCode::KeyE => self.input.cycle += 1,
            KeyCode::F12 => {
                let dir = std::env::var_os("HOME").map(std::path::PathBuf::from).unwrap_or_default().join("Pictures");
                let _ = std::fs::create_dir_all(&dir);
                let stamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
                self.renderer.capture = Some(dir.join(format!("hushfold-{stamp}.png")));
            }
            KeyCode::F10 => {
                self.game.save();
                event_loop.exit();
            }
            _ => {}
        }
    }

    fn frame(&mut self) {
        let now = Instant::now();
        let dt = (now - self.last).as_secs_f32().min(0.05);
        self.last = now;
        self.fps_acc.0 += dt;
        self.fps_acc.1 += 1;
        if self.fps_acc.0 >= 0.5 {
            self.hud.fps = self.fps_acc.1 as f32 / self.fps_acc.0;
            self.hud.frame_ms = 1000.0 * self.fps_acc.0 / self.fps_acc.1 as f32;
            self.fps_acc = (0.0, 0);
        }

        if self.script.is_some() {
            self.run_script();
        }
        let k = |c| self.keys.contains(&c);
        let playing = self.playing();
        if playing {
            self.input.forward = (k(KeyCode::KeyW) as i32 - k(KeyCode::KeyS) as i32) as f32;
            self.input.strafe = (k(KeyCode::KeyD) as i32 - k(KeyCode::KeyA) as i32) as f32;
            self.input.sprint = k(KeyCode::ShiftLeft) || k(KeyCode::ShiftRight);
        } else {
            self.input.forward = 0.0;
            self.input.strafe = 0.0;
            self.input.look = glam::Vec2::ZERO;
        }
        self.game.update(dt, &self.input, &mut self.audio, playing);
        self.input.look = glam::Vec2::ZERO;
        self.input.primary_pressed = false;
        self.input.cycle = 0;

        self.save_timer += dt;
        if self.save_timer > AUTOSAVE_SECS && self.game.dirty_save {
            self.save_timer = 0.0;
            self.game.save();
        }

        self.game.build_instances();
        let aspect = self.renderer.aspect();

        let raw = self.egui_state.take_egui_input(&self.renderer.window);
        let mut actions = Vec::new();
        let music_on = self.audio.music_on;
        let out = self.egui_ctx.run_ui(raw, |ui| {
            actions = hud::draw(ui.ctx(), &self.game, &mut self.hud, aspect, music_on);
        });
        self.egui_state.handle_platform_output(&self.renderer.window, out.platform_output);
        for a in actions {
            match a {
                HudAction::Buy(u) => {
                    self.game.buy(u, &mut self.audio);
                }
                HudAction::Begin => {
                    self.hud.show_intro = false;
                    self.set_grab(true);
                }
                HudAction::CloseMenu => {
                    self.hud.menu_open = false;
                    self.set_grab(true);
                }
            }
        }
        let prims = self.egui_ctx.tessellate(out.shapes, out.pixels_per_point);

        let globals = self.game.globals(aspect);
        self.renderer.render(FrameInput {
            globals: &globals,
            post: PostParams { exposure: 1.15, bloom: 0.55, time: self.game.time, vignette: 0.55 },
            dynamic: &self.game.dyn_insts,
            trans: &self.game.trans_insts,
            held: &self.game.held_insts,
            clear: [0.035, 0.03, 0.04],
            egui_prims: &prims,
            egui_textures: &out.textures_delta,
            pixels_per_point: out.pixels_per_point,
        });
    }
}

impl Running {
    fn run_script(&mut self) {
        let Some(sc) = self.script.as_mut() else { return };
        if sc.wait > 0 {
            sc.wait -= 1;
            return;
        }
        while sc.idx < sc.lines.len() {
            let line = sc.lines[sc.idx].clone();
            sc.idx += 1;
            let w: Vec<&str> = line.split_whitespace().collect();
            let num = |i: usize| w.get(i).and_then(|x| x.parse::<f32>().ok()).unwrap_or(0.0);
            match w.first().copied() {
                Some("pose") => {
                    let p = &mut self.game.player;
                    p.pos = glam::Vec3::new(num(1), num(2), num(3));
                    p.yaw = num(4).to_radians();
                    p.pitch = num(5).to_radians();
                }
                Some("wait") => {
                    sc.wait = num(1) as u32;
                    return;
                }
                Some("click") => self.input.primary_pressed = true,
                Some("faceheap") => self.game.script_face_heap(),
                Some("fill") => self.game.script_fill_category(num(1) as usize, num(2)),
                Some("faceslot") => self.game.script_face_active_slot(),
                Some("cycle") => self.input.cycle += num(1) as i32,
                Some("walk") => {
                    // walk <forward> <frames>
                    self.input.forward = num(1);
                    sc.wait = num(2) as u32;
                    return;
                }
                Some("grant") => self.game.lumens += num(1) as u64,
                Some("buy") => {
                    let name = w.get(1).copied().unwrap_or("");
                    if let Some(u) = game::UPGRADES.iter().find(|u| format!("{u:?}").eq_ignore_ascii_case(name)) {
                        self.game.lumens += self.game.next_cost(*u).unwrap_or(0);
                        self.game.buy(*u, &mut self.audio);
                    }
                }
                Some("menu") => self.hud.menu_open = w.get(1) == Some(&"on"),
                Some("intro") => self.hud.show_intro = w.get(1) == Some(&"on"),
                Some("stats") => self.hud.show_stats = w.get(1) == Some(&"on"),
                Some("shot") => {
                    let name = w.get(1).copied().unwrap_or("shot");
                    self.renderer.capture = Some(sc.dir.join(format!("{name}.png")));
                    sc.wait = 1;
                    return;
                }
                Some("print") => eprintln!(
                    "script: shelved {} held {} lumens {} aim {:?} pos {:?}",
                    self.game.total_shelved(),
                    self.game.held.len(),
                    self.game.lumens,
                    self.game.aim,
                    self.game.player.pos
                ),
                Some("quit") => std::process::exit(0),
                _ => {}
            }
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.run.is_some() {
            return;
        }
        let attrs = Window::default_attributes()
            .with_title("Hushfold Library")
            .with_inner_size(winit::dpi::LogicalSize::new(1600.0, 900.0));
        let window = Arc::new(event_loop.create_window(attrs).expect("create window"));
        let mut renderer = Renderer::new(window.clone());
        let t0 = Instant::now();
        let mut game = Game::new();
        let script = std::env::var("HUSHFOLD_SCRIPT").ok().map(|path| Script {
            lines: std::fs::read_to_string(&path).expect("read script").lines().map(String::from).collect(),
            idx: 0,
            wait: 10,
            dir: std::env::var("HUSHFOLD_SHOT_DIR").unwrap_or_else(|_| ".".into()).into(),
        });
        let loaded = script.is_none() && game.load();
        game.persist = script.is_none();
        eprintln!(
            "hushfold: {} books in {} series, {} heaps, generated in {:.0} ms{}",
            game.lib.books.len(),
            game.lib.series.len(),
            game.lib.heaps.len(),
            t0.elapsed().as_secs_f32() * 1000.0,
            if loaded { " (save loaded)" } else { "" }
        );
        renderer.set_static(&game.lib.static_insts);
        let egui_ctx = egui::Context::default();
        hud::install_fonts(&egui_ctx);
        egui_ctx.set_visuals(egui::Visuals::dark());
        let egui_state = egui_winit::State::new(
            egui_ctx.clone(),
            egui::ViewportId::ROOT,
            &window,
            Some(window.scale_factor() as f32),
            None,
            None,
        );
        self.run = Some(Running {
            renderer,
            egui_ctx,
            egui_state,
            game,
            audio: Audio::new(),
            hud: Hud { menu_open: false, show_intro: script.is_none(), show_stats: false, fps: 0.0, frame_ms: 0.0 },
            input: Input::default(),
            keys: HashSet::new(),
            grabbed: false,
            last: Instant::now(),
            save_timer: 0.0,
            fps_acc: (0.0, 0),
            script,
        });
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        let Some(r) = self.run.as_mut() else { return };
        let consumed = if r.grabbed { false } else { r.egui_state.on_window_event(&r.renderer.window, &event).consumed };
        match event {
            WindowEvent::CloseRequested => {
                r.game.save();
                event_loop.exit();
            }
            WindowEvent::Resized(s) => r.renderer.resize(s.width, s.height),
            WindowEvent::RedrawRequested => r.frame(),
            WindowEvent::Focused(false) => {
                if r.grabbed {
                    r.set_grab(false);
                    r.hud.show_intro = true;
                }
                r.keys.clear();
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if let PhysicalKey::Code(code) = event.physical_key {
                    r.on_key(code, event.state == ElementState::Pressed, event_loop);
                }
            }
            WindowEvent::MouseInput { state, button: MouseButton::Left, .. } => {
                let pressed = state == ElementState::Pressed;
                if r.grabbed {
                    r.input.primary_down = pressed;
                    if pressed {
                        r.input.primary_pressed = true;
                    }
                } else if pressed && !consumed && !r.hud.menu_open && !r.hud.show_intro {
                    r.set_grab(true);
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                if r.playing() {
                    let d = match delta {
                        MouseScrollDelta::LineDelta(_, y) => y,
                        MouseScrollDelta::PixelDelta(p) => (p.y / 40.0) as f32,
                    };
                    if d > 0.0 {
                        r.input.cycle -= 1;
                    } else if d < 0.0 {
                        r.input.cycle += 1;
                    }
                }
            }
            _ => {}
        }
    }

    fn device_event(&mut self, _el: &ActiveEventLoop, _id: DeviceId, event: DeviceEvent) {
        let Some(r) = self.run.as_mut() else { return };
        if let DeviceEvent::MouseMotion { delta } = event {
            if r.playing() {
                r.input.look += glam::Vec2::new(delta.0 as f32, delta.1 as f32) * MOUSE_SENS;
            }
        }
    }

    fn about_to_wait(&mut self, _el: &ActiveEventLoop) {
        if let Some(r) = &self.run {
            r.renderer.window.request_redraw();
        }
    }
}

fn main() {
    let event_loop = EventLoop::new().expect("event loop");
    event_loop.set_control_flow(ControlFlow::Poll);
    let mut app = App::default();
    event_loop.run_app(&mut app).expect("run");
}
