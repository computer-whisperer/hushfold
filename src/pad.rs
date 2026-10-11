//! Gamepads, through gilrs. The pad pressed last plays: its sticks walk and look, its
//! buttons do what the keys and mouse do, and in the menus it moves the focus (up and
//! down as Tab does, from one thing to the next as they are read; left and right as
//! the arrow keys do, which also move sliders) and presses what has it. The prompts name its buttons while it is
//! the last thing touched. Under Steam Input, the pads Steam stands in for are passed
//! over (SDL's `SDL_GAMECONTROLLER_IGNORE_DEVICES`, which Steam sets), or each press
//! would come twice: from the pad, and from Steam's.

use crate::Running;
use crate::controls::Prompts;
use crate::game::{Aim, SPELLS};
use crate::hud::{Screen, SettingsTab};
use gilrs::{Axis, Button, EventType, GamepadId, Gilrs, GilrsBuilder};
use glam::Vec2;

/// Below this far from its centre the left stick is at rest; past it, the walk grows
/// from nothing to full.
const WALK_REST: f32 = 0.2;
/// The right stick's, finer.
const LOOK_REST: f32 = 0.12;
/// Turning with the right stick full over, in radians a second at look speed 1;
/// looking up and down, this much of it.
const TURN: f32 = 2.8;
const TILT: f32 = 0.65;
/// The look is this much slower while the crosshair is on a book or a run.
const STICKY: f32 = 0.55;
/// The left stick tilted this far moves the menus' focus, and must come back inside
/// the second to move it again.
const NUDGE: (f32, f32) = (0.6, 0.4);
/// A direction held in the menus moves the focus on again after this long, then this
/// often.
const REPEAT: (f32, f32) = (0.4, 0.11);
/// A stick moved this far makes its pad the one played with.
const STIRRED: f32 = 0.5;

pub enum PadEvent {
    Down(Button),
    Up(Button),
    /// A stick moved on a pad, which is now the one played with.
    Stirred,
    /// The pad played with went.
    Gone,
}

pub struct Pads {
    gilrs: Option<Gilrs>,
    /// The pad played with: the one whose button was pressed, or stick moved, last.
    active: Option<GamepadId>,
    /// What the prompts call its buttons.
    kind: Prompts,
    ignore: Ignore,
    /// The buttons held on it, let go of when another pad takes over or it goes.
    held: Vec<Button>,
    /// The sticks as a script set them, in place of a pad's.
    fake: Option<[Vec2; 2]>,
    /// Hurrying: the left stick was pressed in, and hasn't come back to rest.
    hurry: bool,
    /// The spell the D-pad casts, by its hotbar slot.
    spell: usize,
    /// Keys for the menus, from the D-pad, the left stick and the face buttons (the
    /// arrows up and down given to egui as Shift+Tab and Tab, `pad_menus`).
    keys: Vec<egui::Key>,
    /// The direction held in the menus and how long until it moves the focus again;
    /// whether it is the stick's.
    repeat: Option<(egui::Key, f32, bool)>,
    /// The hover the mouse left on the menus is to be cleared.
    pointer_gone: bool,
    /// What had the menus' focus last frame, to scroll what takes it into view.
    focus: Option<egui::Id>,
    /// What had it before a drop-down's list opened (the drop-down), to have it back
    /// when the list closes; and the list is to close (a choice was made in it, or B).
    opener: Option<egui::Id>,
    close_list: bool,
}

impl Pads {
    /// None for scripted runs: their pad is the script's (`pad`).
    pub fn new(scripted: bool) -> Pads {
        let gilrs = if scripted {
            None
        } else {
            // Analog triggers press at under half travel.
            match GilrsBuilder::new().set_axis_to_btn(0.4, 0.3).build() {
                Ok(g) => Some(g),
                Err(gilrs::Error::NotImplemented(g)) => Some(g),
                Err(e) => {
                    eprintln!("hushfold: no gamepads: {e}");
                    None
                }
            }
        };
        let env = |name| std::env::var(name).ok();
        let ignore = Ignore {
            skip: env("SDL_GAMECONTROLLER_IGNORE_DEVICES").map_or(Vec::new(), |l| ids(&l)),
            only: env("SDL_GAMECONTROLLER_IGNORE_DEVICES_EXCEPT").map(|l| ids(&l)),
        };
        if let Some(g) = &gilrs {
            for (_, pad) in g.gamepads() {
                if !ignore.skips(&pad) {
                    eprintln!("hushfold: gamepad: {}", pad.name());
                }
            }
        }
        Pads {
            gilrs,
            active: None,
            kind: Prompts::Xbox,
            ignore,
            held: Vec::new(),
            fake: None,
            hurry: false,
            spell: 0,
            keys: Vec::new(),
            repeat: None,
            pointer_gone: false,
            focus: None,
            opener: None,
            close_list: false,
        }
    }

    /// What the pads did since the last call. A pad that stops being the one played
    /// with lets go of what it held first, as its own releases are no longer heeded.
    fn poll(&mut self) -> Vec<PadEvent> {
        let Some(gilrs) = &mut self.gilrs else {
            return Vec::new();
        };
        let mut out = Vec::new();
        while let Some(ev) = gilrs.next_event() {
            let pad = gilrs.gamepad(ev.id);
            if self.ignore.skips(&pad) {
                continue;
            }
            let ours = self.active == Some(ev.id);
            let stirred = matches!(ev.event, EventType::AxisChanged(a, v, _) if axis_is_stick(a) && v.abs() > STIRRED);
            let pressed = matches!(ev.event, EventType::ButtonPressed(b, _) if b != Button::Unknown);
            if !ours && (pressed || stirred) {
                out.extend(self.held.drain(..).map(PadEvent::Up));
                (self.active, self.kind) = (Some(ev.id), kind_of(&pad));
            }
            match ev.event {
                EventType::ButtonPressed(b, _) if pressed => {
                    self.held.push(b);
                    out.push(PadEvent::Down(b));
                }
                EventType::ButtonReleased(b, _) if ours => {
                    self.held.retain(|&h| h != b);
                    out.push(PadEvent::Up(b));
                }
                _ if stirred => out.push(PadEvent::Stirred),
                EventType::Connected => eprintln!("hushfold: gamepad: {}", pad.name()),
                EventType::Disconnected if ours => {
                    self.active = None;
                    out.extend(self.held.drain(..).map(PadEvent::Up));
                    out.push(PadEvent::Gone);
                }
                _ => {}
            }
        }
        out
    }

    /// Pass over what the pads did (while the window hasn't the keyboard: presses
    /// made in another program aren't the game's, now or when it is back).
    pub fn pass_over(&mut self) {
        drop(self.poll());
    }

    /// The left stick and the right, of the pad played with (or the script's).
    fn sticks(&self) -> [Vec2; 2] {
        if let Some(fake) = self.fake {
            return fake;
        }
        let (Some(g), Some(id)) = (&self.gilrs, self.active) else {
            return [Vec2::ZERO; 2];
        };
        let pad = g.gamepad(id);
        let v = |x, y| Vec2::new(pad.value(x), pad.value(y));
        [
            v(Axis::LeftStickX, Axis::LeftStickY),
            v(Axis::RightStickX, Axis::RightStickY),
        ]
    }
}

/// What the prompts call a pad's buttons, by who made it.
fn kind_of(pad: &gilrs::Gamepad) -> Prompts {
    match pad.vendor_id() {
        Some(0x054c) => Prompts::PlayStation,
        Some(0x057e) => Prompts::Nintendo,
        _ => Prompts::Xbox,
    }
}

fn axis_is_stick(axis: Axis) -> bool {
    matches!(
        axis,
        Axis::LeftStickX | Axis::LeftStickY | Axis::RightStickX | Axis::RightStickY
    )
}

/// SDL's lists of pads to pass over, or to keep to, by USB vendor and product.
struct Ignore {
    skip: Vec<(u16, u16)>,
    only: Option<Vec<(u16, u16)>>,
}

impl Ignore {
    fn skips(&self, pad: &gilrs::Gamepad) -> bool {
        self.skips_id((pad.vendor_id().unwrap_or(0), pad.product_id().unwrap_or(0)))
    }

    fn skips_id(&self, id: (u16, u16)) -> bool {
        self.skip.contains(&id) || self.only.as_ref().is_some_and(|only| !only.contains(&id))
    }
}

/// `0xVVVV/0xPPPP` pairs, comma separated.
fn ids(list: &str) -> Vec<(u16, u16)> {
    let hex = |s: &str| {
        let s = s.trim();
        let s = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")).unwrap_or(s);
        u16::from_str_radix(s, 16).ok()
    };
    (list.split(','))
        .filter_map(|pair| {
            let (v, p) = pair.split_once('/')?;
            Some((hex(v)?, hex(p)?))
        })
        .collect()
}

/// A stick's tilt past `rest`, grown from nothing there to one at the rim.
fn past(stick: Vec2, rest: f32) -> Vec2 {
    let m = stick.length();
    if m <= rest {
        return Vec2::ZERO;
    }
    stick / m * ((m - rest) / (1.0 - rest)).min(1.0)
}

/// The right stick's tilt as a rate of turn: squared, so small tilts are fine.
fn look_rate(stick: Vec2) -> Vec2 {
    let t = past(stick, LOOK_REST);
    t * t.length()
}

/// The menus' direction the left stick points, if it is tilted far enough (`held`:
/// the one it pointed last frame, kept while it is past the inner bound).
fn nudged(stick: Vec2, held: Option<egui::Key>) -> Option<egui::Key> {
    let bound = if held.is_some() { NUDGE.1 } else { NUDGE.0 };
    if stick.length() < bound {
        return None;
    }
    Some(if stick.x.abs() > stick.y.abs() {
        if stick.x > 0.0 {
            egui::Key::ArrowRight
        } else {
            egui::Key::ArrowLeft
        }
    } else if stick.y > 0.0 {
        egui::Key::ArrowUp
    } else {
        egui::Key::ArrowDown
    })
}

/// A button by its name in scripts, where an Xbox pad has it.
pub fn button_named(name: &str) -> Option<Button> {
    Some(match name {
        "a" => Button::South,
        "b" => Button::East,
        "x" => Button::West,
        "y" => Button::North,
        "lb" => Button::LeftTrigger,
        "rb" => Button::RightTrigger,
        "lt" => Button::LeftTrigger2,
        "rt" => Button::RightTrigger2,
        "ls" => Button::LeftThumb,
        "rs" => Button::RightThumb,
        "view" => Button::Select,
        "menu" => Button::Start,
        "up" => Button::DPadUp,
        "down" => Button::DPadDown,
        "left" => Button::DPadLeft,
        "right" => Button::DPadRight,
        _ => return None,
    })
}

const TABS: [SettingsTab; 3] = [SettingsTab::Video, SettingsTab::Audio, SettingsTab::Controls];

impl Running {
    /// The pads since the last frame: their buttons, and their sticks walking and
    /// looking while playing, or moving the menus' focus.
    pub(crate) fn pads(&mut self, dt: f32) {
        for ev in self.pads.poll() {
            if !self.focused {
                continue;
            }
            self.use_pad();
            match ev {
                PadEvent::Down(b) => self.pad_button(b, true),
                PadEvent::Up(b) => self.pad_button(b, false),
                PadEvent::Stirred => {}
                PadEvent::Gone => {
                    self.hud.prompts = Prompts::Keys;
                    if self.hud.screen == Screen::Play {
                        self.save_game();
                        self.show(Screen::Pause);
                    }
                }
            }
        }
        let [left, right] = self.pads.sticks();
        if !self.focused {
            self.pads.repeat = None;
        } else if self.playing() {
            let walk = past(left, WALK_REST);
            if walk == Vec2::ZERO {
                self.pads.hurry = false;
            }
            let input = &mut self.input;
            input.forward = (input.forward + walk.y).clamp(-1.0, 1.0);
            input.strafe = (input.strafe + walk.x).clamp(-1.0, 1.0);
            input.sprint |= self.pads.hurry;
            let s = &self.hud.settings;
            let rate = look_rate(right);
            let y = if s.invert_look { rate.y } else { -rate.y };
            let sticky = matches!(self.game.aim, Aim::Loose { .. } | Aim::Shelf { run: Some(_), .. });
            let slow = if sticky { STICKY } else { 1.0 };
            input.look += Vec2::new(rate.x * TURN, y * TURN * TILT) * dt * s.look_speed * slow;
            self.pads.repeat = None;
        } else {
            self.pads.hurry = false;
            let stick = self.pads.repeat.filter(|r| r.2).map(|r| r.0);
            match nudged(left, stick) {
                Some(k) if stick != Some(k) => {
                    self.pads.keys.push(k);
                    self.pads.repeat = Some((k, REPEAT.0, true));
                }
                None if stick.is_some() => self.pads.repeat = None,
                _ => {}
            }
            if let Some((k, wait, _)) = &mut self.pads.repeat {
                *wait -= dt;
                if *wait <= 0.0 {
                    *wait = REPEAT.1;
                    self.pads.keys.push(*k);
                }
            }
        }
    }

    /// The pad was the last thing touched: the prompts name its buttons, and the
    /// mouse's pointer leaves the menus.
    fn use_pad(&mut self) {
        if !self.hud.prompts.pad() {
            self.pads.pointer_gone = true;
        }
        self.hud.prompts = self.pads.kind;
    }

    /// The keyboard or mouse was the last thing touched.
    pub(crate) fn use_keys(&mut self) {
        self.hud.prompts = Prompts::Keys;
    }

    pub(crate) fn pad_button(&mut self, b: Button, down: bool) {
        if self.playing() {
            self.pad_play(b, down);
            return;
        }
        let dir = match b {
            Button::DPadUp => Some(egui::Key::ArrowUp),
            Button::DPadDown => Some(egui::Key::ArrowDown),
            Button::DPadLeft => Some(egui::Key::ArrowLeft),
            Button::DPadRight => Some(egui::Key::ArrowRight),
            _ => None,
        };
        if let Some(k) = dir {
            if down {
                self.pads.keys.push(k);
                self.pads.repeat = Some((k, REPEAT.0, false));
            } else if self.pads.repeat.is_some_and(|r| r.0 == k) {
                self.pads.repeat = None;
            }
            return;
        }
        if !down {
            return;
        }
        let list = egui::Popup::is_any_open(&self.egui_ctx);
        match b {
            Button::South => {
                self.pads.keys.push(egui::Key::Enter);
                // A choice in a drop-down's list closes it, as a click does (not A on
                // the drop-down itself, which closes it, nor on what lies under it).
                self.pads.close_list = list && self.in_list();
            }
            // A list open from a drop-down is closed first.
            Button::East if list => self.pads.close_list = true,
            Button::East | Button::Start => self.back(),
            Button::Select => self.grimoire(),
            Button::LeftTrigger | Button::RightTrigger if matches!(self.hud.screen, Screen::Settings { .. }) => {
                let at = TABS.iter().position(|&t| t == self.hud.settings_tab).unwrap_or(0);
                let step = if b == Button::RightTrigger { 1 } else { TABS.len() - 1 };
                self.hud.settings_tab = TABS[(at + step) % TABS.len()];
            }
            _ => {}
        }
    }

    fn pad_play(&mut self, b: Button, down: bool) {
        let spell = self.spell_picked();
        let input = &mut self.input;
        match b {
            Button::RightTrigger2 => {
                input.primary_down = down;
                input.primary_pressed |= down;
            }
            Button::LeftTrigger2 => {
                input.toss_down = down;
                input.toss_pressed |= down;
            }
            Button::East => {
                input.armful_down = down;
                input.armful_pressed |= down;
            }
            _ if !down => {}
            Button::South => input.jump = true,
            Button::West => input.seek = true,
            Button::North => input.regroup = true,
            Button::LeftTrigger => input.cycle -= 1,
            Button::RightTrigger => input.cycle += 1,
            Button::LeftThumb => self.pads.hurry = true,
            Button::DPadLeft => self.pick_spell(-1),
            Button::DPadRight => self.pick_spell(1),
            Button::DPadUp => input.cast = Some(spell),
            Button::DPadDown => input.post = true,
            Button::Select => self.grimoire(),
            Button::Start => self.back(),
            _ => {}
        }
    }

    /// The hotbar slots of the spells learned.
    fn spells_learned(&self) -> Vec<usize> {
        (0..SPELLS.len()).filter(|&i| self.game.level(SPELLS[i]) > 0).collect()
    }

    /// The spell the D-pad casts: the one chosen, while it is learned, else the first
    /// learned.
    pub(crate) fn spell_picked(&self) -> usize {
        let learned = self.spells_learned();
        if learned.contains(&self.pads.spell) {
            self.pads.spell
        } else {
            learned.first().copied().unwrap_or(0)
        }
    }

    fn pick_spell(&mut self, step: i32) {
        let learned = self.spells_learned();
        let picked = self.spell_picked();
        let at = learned.iter().position(|&i| i == picked).unwrap_or(0) as i32;
        if let Some(&slot) = learned.get((at + step).rem_euclid(learned.len().max(1) as i32) as usize) {
            self.pads.spell = slot;
        }
    }

    /// Whether the menus' focus is in an open drop-down's list: on another layer than
    /// the drop-down it opened from.
    fn in_list(&self) -> bool {
        let (Some(focus), Some(opener)) = (self.egui_ctx.memory(|m| m.focused()), self.pads.opener) else {
            return false;
        };
        let layer = |id| self.egui_ctx.read_response(id).map(|r| r.layer_id);
        layer(focus) != layer(opener)
    }

    /// The menus' keys from the pad, into egui's input for this frame. With nothing
    /// focused, the pad's first press focuses the first thing that can be, and does
    /// nothing else (on a fresh screen, nothing is pressed unseen); what takes the
    /// focus is scrolled into view.
    pub(crate) fn pad_menus(&mut self, raw: &mut egui::RawInput) {
        if std::mem::take(&mut self.pads.pointer_gone) {
            raw.events.push(egui::Event::PointerGone);
        }
        let mut focus = self.egui_ctx.memory(|m| m.focused());
        if !egui::Popup::is_any_open(&self.egui_ctx) {
            self.pads.opener = focus;
            self.pads.close_list = false;
        } else if self.pads.keys.is_empty() && std::mem::take(&mut self.pads.close_list) {
            // Closed (once the choice made in it is in), and the focus back on the
            // drop-down, not lost with the list.
            egui::Popup::close_all(&self.egui_ctx);
            if let Some(id) = self.pads.opener {
                self.egui_ctx.memory_mut(|m| m.request_focus(id));
                focus = Some(id);
            }
        }
        let moved = std::mem::replace(&mut self.pads.focus, focus) != focus;
        if self.hud.screen == Screen::Play || !self.hud.prompts.pad() {
            self.pads.keys.clear();
            return;
        }
        if let Some(id) = focus.filter(|_| moved) {
            use egui::accesskit::{Action, ActionRequest, TreeId};
            raw.events.push(egui::Event::AccessKitActionRequest(ActionRequest {
                action: Action::ScrollIntoView,
                target_tree: TreeId::ROOT,
                target_node: id.accesskit_id(),
                data: None,
            }));
        }
        let press = |key, modifiers| {
            let event = |pressed| egui::Event::Key {
                key,
                physical_key: None,
                pressed,
                repeat: false,
                modifiers,
            };
            [event(true), event(false)]
        };
        // One a frame: egui moves the focus once a frame, however often it is asked.
        if self.pads.keys.is_empty() {
            return;
        }
        // The press that brings the focus up does only that.
        if focus.is_none() {
            raw.events.extend(press(egui::Key::Tab, egui::Modifiers::NONE));
            self.pads.keys.clear();
            return;
        }
        // egui's arrows look for the nearest thing in a narrow cone, which in a grid
        // of names and their settings skips rows; Tab goes through them in order.
        raw.events.extend(match self.pads.keys.remove(0) {
            egui::Key::ArrowDown => press(egui::Key::Tab, egui::Modifiers::NONE),
            egui::Key::ArrowUp => press(egui::Key::Tab, egui::Modifiers::SHIFT),
            k => press(k, egui::Modifiers::NONE),
        });
    }

    /// Script commands for the pad: `pad press|release <button>`, `pad stick
    /// left|right <x> <y>`, `pad kind xbox|playstation|nintendo`.
    pub(crate) fn script_pad(&mut self, w: &[&str]) {
        let num = |i: usize| w.get(i).and_then(|x| x.parse::<f32>().ok()).unwrap_or(0.0);
        match w.first().copied() {
            Some(c @ ("press" | "release")) => {
                if let Some(b) = w.get(1).copied().and_then(button_named) {
                    self.use_pad();
                    self.pad_button(b, c == "press");
                }
            }
            Some("stick") => {
                self.use_pad();
                let sticks = self.pads.fake.get_or_insert([Vec2::ZERO; 2]);
                let i = usize::from(w.get(1) == Some(&"right"));
                sticks[i] = Vec2::new(num(2), num(3));
            }
            Some("kind") => {
                self.pads.kind = match w.get(1).copied() {
                    Some("playstation") => Prompts::PlayStation,
                    Some("nintendo") => Prompts::Nintendo,
                    _ => Prompts::Xbox,
                };
                self.use_pad();
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Steam's lists of pads it stands in for are read as SDL reads them.
    #[test]
    fn steams_pads_are_passed_over() {
        let ignore = Ignore {
            skip: ids("0x045e/0x028e, 0x054C/0x0CE6,bad,0x1/"),
            only: None,
        };
        assert_eq!(ignore.skip, [(0x045e, 0x028e), (0x054c, 0x0ce6)]);
        assert!(ignore.skips_id((0x054c, 0x0ce6)) && !ignore.skips_id((0x28de, 0x11ff)));
        let only = Ignore {
            skip: Vec::new(),
            only: Some(ids("0x28de/0x11ff")),
        };
        assert!(!only.skips_id((0x28de, 0x11ff)) && only.skips_id((0x045e, 0x028e)));
    }

    /// The sticks rest at their centres, reach full at the rim, and the look is
    /// finer near the centre than a straight line.
    #[test]
    fn sticks_rest_and_reach_full() {
        assert_eq!(past(Vec2::new(0.15, 0.1), WALK_REST), Vec2::ZERO);
        assert!((past(Vec2::new(0.0, 1.0), WALK_REST).y - 1.0).abs() < 1e-6);
        assert!((past(Vec2::new(0.8, 0.8), WALK_REST).length() - 1.0).abs() < 1e-6);
        let half = look_rate(Vec2::new(0.5 + LOOK_REST / 2.0, 0.0)).x;
        assert!(half > 0.2 && half < 0.3, "{half}");
        assert!((look_rate(Vec2::X).x - 1.0).abs() < 1e-6);
    }

    #[test]
    fn the_stick_nudges_the_menus_once_per_tilt() {
        let up = nudged(Vec2::new(0.1, 0.7), None);
        assert_eq!(up, Some(egui::Key::ArrowUp));
        // Kept while past the inner bound; let go inside it.
        assert_eq!(nudged(Vec2::new(0.0, 0.45), up), up);
        assert_eq!(nudged(Vec2::new(0.0, 0.45), None), None);
        assert_eq!(nudged(Vec2::new(-0.9, 0.3), None), Some(egui::Key::ArrowLeft));
    }
}
