// Release builds on Windows run without a console window.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod audio;
mod catalog;
mod game;
mod gfx;
mod hud;
mod library;
mod loose;
mod nav;
mod platform;
mod scene;
mod settings;
mod text;
mod world;

use audio::Audio;
use game::{Game, Input};
use gfx::{FrameInput, PostParams, Renderer};
use hud::{Hud, HudAction};
use library::Layout;
use settings::Settings;
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
    /// Scroll not yet turned into whole cycle steps.
    scroll_acc: f32,
    /// Render without waiting for the compositor (`HUSHFOLD_UNPACED`).
    unpaced: bool,
    quit: bool,
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
        self.input.toss_down = false;
    }

    /// Start this library over; its save is overwritten at once so quitting now
    /// doesn't bring the old game back. Settings are kept.
    fn new_game(&mut self) {
        self.open_library(self.game.layout, false);
        self.game.save();
    }

    /// Put `layout` in place of the current library: its save if `resume` and it has
    /// one, else a fresh one. The current library is saved first.
    fn open_library(&mut self, layout: Layout, resume: bool) {
        self.game.save();
        let persist = self.game.persist;
        let t0 = Instant::now();
        self.game = Game::new(layout);
        self.game.persist = persist;
        let loaded = persist && resume && self.game.load();
        eprintln!(
            "hushfold: {}: {} books, generated in {:.0} ms{}",
            layout.name(),
            self.game.lib.books.len(),
            t0.elapsed().as_secs_f32() * 1000.0,
            if loaded { " (save loaded)" } else { "" }
        );
        self.renderer
            .set_static(&self.game.lib.static_insts, &self.game.lib.meshes);
        self.renderer.set_text(&self.game.lib.text);
        self.renderer.set_books(&self.game.lib.series);
        self.renderer.clear_chunks();
        // Fill its chunks now, so this frame doesn't draw an empty library.
        self.game.build_instances(self.renderer.aspect());
        self.apply_settings();
        self.hud.confirm_restart = false;
        self.hud.menu_open = false;
        self.hud.cheats_open = false;
        if self.hud.settings.library != layout {
            self.hud.settings.library = layout;
            self.save_settings();
        }
    }

    fn apply_settings(&mut self) {
        let s = self.hud.settings;
        self.game.fov = s.fov;
        self.audio.configure(s.music, s.music_volume, s.effects_volume);
    }

    /// Persist settings (never from scripted runs, which use the defaults).
    fn save_settings(&self) {
        if self.script.is_none() {
            self.hud.settings.save();
        }
    }

    fn playing(&self) -> bool {
        (self.grabbed || self.script.is_some()) && !self.hud.menu_open && !self.hud.cheats_open && !self.hud.show_intro
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
                self.hud.cheats_open = false;
                self.set_grab(!self.hud.menu_open);
            }
            KeyCode::F9 => {
                if self.hud.show_intro {
                    return;
                }
                self.hud.cheats_open = !self.hud.cheats_open;
                self.hud.menu_open = false;
                self.set_grab(!self.hud.cheats_open);
            }
            KeyCode::Escape => {
                if self.hud.menu_open || self.hud.cheats_open {
                    self.hud.menu_open = false;
                    self.hud.cheats_open = false;
                    self.set_grab(true);
                } else if self.hud.show_intro {
                    if self.game.total_shelved() > 0 || !self.game.held.is_empty() {
                        self.hud.show_intro = false;
                        self.save_settings();
                        self.set_grab(true);
                    }
                } else {
                    self.hud.show_intro = true;
                    self.set_grab(false);
                    self.game.save();
                }
            }
            KeyCode::KeyM => {
                self.hud.settings.music = !self.hud.settings.music;
                self.apply_settings();
                self.save_settings();
            }
            KeyCode::F3 => self.hud.show_stats = !self.hud.show_stats,
            KeyCode::F11 => {
                let full = self.renderer.window.fullscreen().is_none();
                self.renderer
                    .window
                    .set_fullscreen(full.then_some(Fullscreen::Borderless(None)));
                self.hud.settings.fullscreen = full;
                self.save_settings();
            }
            KeyCode::KeyF => self.input.seek = true,
            KeyCode::KeyR => self.input.regroup = true,
            KeyCode::KeyG => self.input.post = true,
            // Latched so a tap shorter than a frame still tosses.
            KeyCode::KeyX => self.input.armful_pressed = true,
            KeyCode::Digit1 => self.input.cast = Some(0),
            KeyCode::Digit2 => self.input.cast = Some(1),
            KeyCode::Digit3 => self.input.cast = Some(2),
            KeyCode::Digit4 => self.input.cast = Some(3),
            KeyCode::Digit5 => self.input.cast = Some(4),
            KeyCode::KeyQ => self.input.cycle -= 1,
            KeyCode::KeyE => self.input.cycle += 1,
            KeyCode::Space => self.input.jump = true,
            KeyCode::F12 => {
                let dir = platform::screenshot_dir();
                let _ = std::fs::create_dir_all(&dir);
                let stamp = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0);
                self.renderer.capture = Some(dir.join(format!("hushfold-{stamp}.png")));
            }
            KeyCode::F10 => {
                self.game.save();
                self.save_settings();
                event_loop.exit();
            }
            _ => {}
        }
    }

    fn frame(&mut self) {
        let now = Instant::now();
        let frame_start = now;
        // Scripted runs step a fixed 60 Hz so `wait N` means N/60 s of game time; the
        // frame rate shown is real either way.
        let real = (now - self.last).as_secs_f32();
        let dt = if self.script.is_some() {
            1.0 / 60.0
        } else {
            real.min(0.05)
        };
        self.last = now;
        self.fps_acc.0 += real;
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
        if self.script.is_some() {
            // Scripted runs drive movement themselves (`walk`).
        } else if playing {
            self.input.forward = (k(KeyCode::KeyW) as i32 - k(KeyCode::KeyS) as i32) as f32;
            self.input.strafe = (k(KeyCode::KeyD) as i32 - k(KeyCode::KeyA) as i32) as f32;
            self.input.sprint = k(KeyCode::ShiftLeft) || k(KeyCode::ShiftRight);
            self.input.armful_down = k(KeyCode::KeyX);
        } else {
            self.input.forward = 0.0;
            self.input.strafe = 0.0;
            self.input.armful_down = false;
            self.input.look = glam::Vec2::ZERO;
        }
        self.game.update(dt, &self.input, &mut self.audio, playing);
        self.game.update_sound(&mut self.audio);
        self.input.look = glam::Vec2::ZERO;
        self.input.primary_pressed = false;
        self.input.toss_pressed = false;
        self.input.armful_pressed = false;
        self.input.seek = false;
        self.input.cycle = 0;
        self.input.regroup = false;
        self.input.post = false;
        self.input.cast = None;
        self.input.jump = false;

        self.save_timer += dt;
        if self.save_timer > AUTOSAVE_SECS && self.game.dirty_save {
            self.save_timer = 0.0;
            self.game.save();
        }

        let aspect = self.renderer.aspect();
        self.game.build_instances(aspect);

        let raw = self.egui_state.take_egui_input(&self.renderer.window);
        let mut actions = Vec::new();
        let out = self.egui_ctx.run_ui(raw, |ui| {
            actions = hud::draw(ui.ctx(), &self.game, &mut self.hud, aspect);
        });
        self.egui_state
            .handle_platform_output(&self.renderer.window, out.platform_output);
        for a in actions {
            match a {
                HudAction::Buy(u) => {
                    self.game.buy(u, &mut self.audio);
                }
                HudAction::Use(u, l) => self.game.set_in_use(u, l, &mut self.audio),
                HudAction::Begin => {
                    self.hud.show_intro = false;
                    self.save_settings();
                    self.set_grab(true);
                }
                HudAction::SettingsChanged => self.apply_settings(),
                HudAction::Quit => self.quit = true,
                HudAction::NewGame => self.new_game(),
                HudAction::Switch(layout) => self.open_library(layout, true),
                HudAction::CloseMenu => {
                    self.hud.menu_open = false;
                    self.set_grab(true);
                }
                HudAction::Cheat(c) => self.game.cheat(c, &mut self.audio),
                HudAction::CloseCheats => {
                    self.hud.cheats_open = false;
                    self.set_grab(true);
                }
            }
        }
        let prims = self.egui_ctx.tessellate(out.shapes, out.pixels_per_point);

        let globals = self.game.globals(aspect);
        self.renderer.sync_chunks(&mut self.game.scene.chunks);
        let cpu_ms = frame_start.elapsed().as_secs_f32() * 1000.0;
        let render_start = Instant::now();
        self.renderer.sync_gpu = self.hud.show_stats;
        let shot = self.renderer.render(FrameInput {
            globals: &globals,
            post: PostParams {
                exposure: 1.15,
                bloom: 0.55,
                time: self.game.time,
                vignette: 0.55,
            },
            dynamic: &self.game.dyn_insts,
            trans: &self.game.trans_insts,
            held: &self.game.held_insts,
            clear: [0.035, 0.03, 0.04],
            egui_prims: &prims,
            egui_textures: &out.textures_delta,
            pixels_per_point: out.pixels_per_point,
        });
        // Say where an F12 screenshot went (not a scripted one's: the note would be in
        // the next shot).
        if let Some(shot) = shot
            && self.script.is_none()
        {
            let msg = match shot {
                Ok(path) => format!("Screenshot saved to {}", path.display()),
                Err(e) => format!("No screenshot: {e}."),
            };
            self.game.hint = Some((msg, 3.0));
        }
        let render_ms = render_start.elapsed().as_secs_f32() * 1000.0;
        self.hud.cpu_ms = self.hud.cpu_ms * 0.9 + cpu_ms * 0.1;
        self.hud.render_ms = self.hud.render_ms * 0.9 + render_ms * 0.1;
    }
}

impl Running {
    fn run_script(&mut self) {
        // Out of `self` while it runs, so commands can call methods on `self`.
        let Some(mut sc) = self.script.take() else { return };
        self.step_script(&mut sc);
        self.script = Some(sc);
    }

    fn step_script(&mut self, sc: &mut Script) {
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
                // click: a tap (takes one book, or files); press … wait N … release: a hold.
                Some("click") => self.input.primary_pressed = true,
                Some("press") => {
                    self.input.primary_pressed = true;
                    self.input.primary_down = true;
                }
                Some("release") => self.input.primary_down = false,
                Some("facepile") => self.game.script_face_pile(),
                // lookat <x> <y> <z> <dist>: stand <dist> m back (toward where you are) and look.
                Some("lookat") => self
                    .game
                    .script_look_at(glam::Vec3::new(num(1), num(2), num(3)), num(4)),
                // charge [all] … wait N … throw: hold and release the toss button (with
                // `all`, X: the whole armful).
                Some("charge") if w.get(1) == Some(&"all") => self.input.armful_down = true,
                Some("charge") => self.input.toss_down = true,
                Some("throw") => (self.input.toss_down, self.input.armful_down) = (false, false),
                Some("seek") => self.input.seek = true,
                Some("regroup") => self.input.regroup = true,
                Some("post") => self.input.post = true,
                // cast <n>: the spell on key n.
                Some("cast") => self.input.cast = Some((num(1) as usize).saturating_sub(1)),
                Some("unveil") => self.game.open_all_gates(),
                Some("facefam") => self.game.script_face_familiar(num(1) as usize),
                Some("fill") => self.game.script_fill_category(num(1) as usize, num(2)),
                Some("take") => self.game.script_take(num(1) as usize, num(2) as usize),
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
                    if let Some(u) = skill_named(w.get(1).copied().unwrap_or("")) {
                        self.game.lumens += self.game.next_cost(u).unwrap_or(0);
                        self.game.buy(u, &mut self.audio);
                    }
                }
                // use <skill> <level>: as the Grimoire's "In use" row (0: off).
                Some("use") => {
                    if let Some(u) = skill_named(w.get(1).copied().unwrap_or("")) {
                        self.game.set_in_use(u, num(2) as u32, &mut self.audio);
                    }
                }
                Some("menu") => self.hud.menu_open = w.get(1) == Some(&"on"),
                Some("cheats") => self.hud.cheats_open = w.get(1) == Some(&"on"),
                // cheat lumens <n> | skills | veils | shelve <fraction> | goto [<department>]
                Some("cheat") => {
                    let c = match w.get(1).copied() {
                        Some("lumens") => Some(game::Cheat::Lumens(num(2) as u64)),
                        Some("skills") => Some(game::Cheat::AllSkills),
                        Some("veils") => Some(game::Cheat::LiftVeils),
                        Some("shelve") => Some(game::Cheat::Shelve(num(2))),
                        Some("goto") => Some(game::Cheat::GoTo(w.get(2).and_then(|d| d.parse().ok()))),
                        _ => None,
                    };
                    if let Some(c) = c {
                        self.game.cheat(c, &mut self.audio);
                    }
                }
                // intro on|off|confirm (confirm: the pause screen asking whether to start over)
                Some("intro") => {
                    self.hud.show_intro = matches!(w.get(1), Some(&"on" | &"confirm"));
                    self.hud.confirm_restart = w.get(1) == Some(&"confirm");
                }
                Some("stats") => self.hud.show_stats = w.get(1) == Some(&"on"),
                Some("shot") => {
                    let name = w.get(1).copied().unwrap_or("shot");
                    self.renderer.capture = Some(sc.dir.join(format!("{name}.png")));
                    sc.wait = 1;
                    return;
                }
                Some("print") => eprintln!(
                    "script: fps {:.0} cpu {:.2}ms render {:.2}ms (acquire {:.2} gpu {:.2} scene {:.2} chunks {} up {}) | shelved {} held {} lumens {} aim {:?} pos {:?} toss lands {:?}",
                    self.hud.fps,
                    self.hud.cpu_ms,
                    self.hud.render_ms,
                    self.renderer.acquire_ms,
                    self.renderer.gpu_ms,
                    self.renderer.gpu_scene_ms,
                    self.renderer.drawn_chunks,
                    self.renderer.uploaded_chunks,
                    self.game.total_shelved(),
                    self.game.held.len(),
                    self.game.lumens,
                    self.game.aim,
                    self.game.player.pos,
                    self.game.toss_land
                ),
                Some("printactive") => {
                    if let Some(b) = self.game.active_book() {
                        let info = &self.game.lib.books[b as usize];
                        eprintln!(
                            "script: active {} face {} row {} slot {:?} size {:?} eye {:?} fwd {:?}",
                            b,
                            info.face,
                            info.row,
                            info.slot_pos,
                            info.size,
                            self.game.player.eye(),
                            self.game.player.forward()
                        );
                    }
                }
                Some("newgame") => self.new_game(),
                // library hall|great: switch to a fresh copy of that library.
                Some("library") => {
                    let layout = if w.get(1) == Some(&"great") {
                        Layout::Great
                    } else {
                        Layout::Hall
                    };
                    self.open_library(layout, false);
                }
                Some("quit") => std::process::exit(0),
                _ => {}
            }
        }
    }
}

impl ApplicationHandler for App {
    /// Drop everything while the event loop, and with it the Wayland connection, is
    /// still alive: `run_app` drops the loop before it returns, and egui's clipboard
    /// (a thread on that connection) destroying its objects after it was gone
    /// crashed every exit.
    fn exiting(&mut self, _: &ActiveEventLoop) {
        self.run = None;
    }

    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.run.is_some() {
            return;
        }
        let scripted = std::env::var_os("HUSHFOLD_SCRIPT").is_some();
        // Scripted runs use default settings so they are reproducible.
        let settings = if scripted {
            Settings::default()
        } else {
            Settings::load()
        };
        let attrs = Window::default_attributes()
            .with_title("Hushfold Library")
            .with_inner_size(winit::dpi::LogicalSize::new(1600.0, 900.0))
            .with_fullscreen(settings.fullscreen.then_some(Fullscreen::Borderless(None)));
        // Name the app for the compositor (Wayland app_id / X11 WM_CLASS).
        #[cfg(all(unix, not(target_vendor = "apple")))]
        let attrs = {
            use winit::platform::{wayland::WindowAttributesExtWayland, x11::WindowAttributesExtX11};
            let attrs = WindowAttributesExtWayland::with_name(attrs, "hushfold", "hushfold");
            WindowAttributesExtX11::with_name(attrs, "hushfold", "hushfold")
        };
        let window = Arc::new(event_loop.create_window(attrs).expect("create window"));
        let mut renderer = Renderer::new(window.clone());
        let t0 = Instant::now();
        // Scripts start in the Hall (or HUSHFOLD_LIBRARY=great).
        let layout = match std::env::var("HUSHFOLD_LIBRARY").as_deref() {
            Ok("great") => Layout::Great,
            Ok(_) => Layout::Hall,
            Err(_) if scripted => Layout::Hall,
            Err(_) => settings.library,
        };
        let mut game = Game::new(layout);
        let script = std::env::var("HUSHFOLD_SCRIPT").ok().map(|path| Script {
            lines: std::fs::read_to_string(&path)
                .expect("read script")
                .lines()
                .map(String::from)
                .collect(),
            idx: 0,
            wait: 10,
            dir: std::env::var("HUSHFOLD_SHOT_DIR").unwrap_or_else(|_| ".".into()).into(),
        });
        let loaded = script.is_none() && game.load();
        game.persist = script.is_none();
        eprintln!(
            "hushfold: {}: {} books in {} series, generated in {:.0} ms{}",
            layout.name(),
            game.lib.books.len(),
            game.lib.series.len(),
            t0.elapsed().as_secs_f32() * 1000.0,
            if loaded { " (save loaded)" } else { "" }
        );
        renderer.set_static(&game.lib.static_insts, &game.lib.meshes);
        renderer.set_text(&game.lib.text);
        renderer.set_books(&game.lib.series);
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
        let mut run = Running {
            renderer,
            egui_ctx,
            egui_state,
            game,
            audio: Audio::new(),
            hud: Hud {
                settings,
                menu_open: false,
                cheats_open: false,
                show_intro: script.is_none(),
                confirm_restart: false,
                show_stats: false,
                fps: 0.0,
                frame_ms: 0.0,
                cpu_ms: 0.0,
                render_ms: 0.0,
            },
            input: Input::default(),
            keys: HashSet::new(),
            grabbed: false,
            last: Instant::now(),
            save_timer: 0.0,
            fps_acc: (0.0, 0),
            scroll_acc: 0.0,
            unpaced: std::env::var_os("HUSHFOLD_UNPACED").is_some(),
            quit: false,
            script,
        };
        run.apply_settings();
        self.run = Some(run);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        let Some(r) = self.run.as_mut() else { return };
        // Scripted runs are deterministic: ignore the real keyboard and mouse.
        if r.script.is_some()
            && matches!(
                event,
                WindowEvent::KeyboardInput { .. }
                    | WindowEvent::MouseInput { .. }
                    | WindowEvent::MouseWheel { .. }
                    | WindowEvent::Focused(_)
            )
        {
            return;
        }
        let consumed = if r.grabbed {
            false
        } else {
            r.egui_state.on_window_event(&r.renderer.window, &event).consumed
        };
        match event {
            WindowEvent::CloseRequested => {
                r.game.save();
                r.save_settings();
                event_loop.exit();
            }
            WindowEvent::Resized(s) => r.renderer.resize(s.width, s.height),
            WindowEvent::RedrawRequested => r.frame(),
            WindowEvent::Focused(false) => {
                // A release that happens in another window never arrives.
                r.input.toss_down = false;
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
            WindowEvent::MouseInput {
                state,
                button: MouseButton::Left,
                ..
            } => {
                let pressed = state == ElementState::Pressed;
                if r.grabbed {
                    r.input.primary_down = pressed;
                    if pressed {
                        r.input.primary_pressed = true;
                    }
                } else if pressed && !consumed && !r.hud.menu_open && !r.hud.cheats_open && !r.hud.show_intro {
                    r.set_grab(true);
                }
            }
            WindowEvent::MouseInput {
                state,
                button: MouseButton::Right,
                ..
            } => {
                let pressed = state == ElementState::Pressed && r.grabbed;
                r.input.toss_down = pressed;
                // Latched so a click shorter than a frame still tosses.
                r.input.toss_pressed |= pressed;
            }
            WindowEvent::MouseWheel { delta, .. } if r.playing() => {
                // One book per wheel notch (a line) or per 40 px of touchpad travel. Smooth
                // scrolling reports fractions of either, so accumulate; horizontal is ignored.
                r.scroll_acc += match delta {
                    MouseScrollDelta::LineDelta(_, y) => y,
                    MouseScrollDelta::PixelDelta(p) => p.y as f32 / 40.0,
                };
                let steps = r.scroll_acc.trunc();
                r.scroll_acc -= steps;
                r.input.cycle -= steps as i32;
            }
            _ => {}
        }
    }

    fn device_event(&mut self, _el: &ActiveEventLoop, _id: DeviceId, event: DeviceEvent) {
        let Some(r) = self.run.as_mut() else { return };
        if let DeviceEvent::MouseMotion { delta } = event
            && r.playing()
            && r.script.is_none()
        {
            let s = r.hud.settings;
            let y = if s.invert_look { -delta.1 } else { delta.1 };
            r.input.look += glam::Vec2::new(delta.0 as f32, y as f32) * MOUSE_SENS * s.look_speed;
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if let Some(r) = &mut self.run {
            if r.quit {
                r.game.save();
                r.save_settings();
                event_loop.exit();
                return;
            }
            // Benchmark switch: render as fast as possible instead of on the compositor's frame callback.
            if r.unpaced {
                r.frame();
            } else {
                r.renderer.window.request_redraw();
            }
        }
    }
}

fn main() {
    let event_loop = EventLoop::new().expect("event loop");
    event_loop.set_control_flow(ControlFlow::Poll);
    let mut app = App::default();
    event_loop.run_app(&mut app).expect("run");
}

/// A skill by its name in scripts (`Upg`'s, any case).
fn skill_named(name: &str) -> Option<game::Upg> {
    (game::UPGRADES.iter().copied()).find(|u| format!("{u:?}").eq_ignore_ascii_case(name))
}
