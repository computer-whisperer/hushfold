//! egui HUD: crosshair, armful, active-book card, signposts, toasts, Grimoire.

use crate::catalog;
use crate::game::{Aim, Cheat, ECHO_SPEED, Game, Group, SPELLS, Tier, UPGRADES, Upg, spell_times, upg_costs, upg_def};
use crate::library::{Finish, LabelKind, Layout, ROWS};
use crate::saves::Save;
use crate::settings::Settings;
use egui::{Align2, Color32, FontFamily, FontId, Pos2, Rect, RichText, Stroke, Vec2};
use glam::{Mat4, Vec3, Vec4};
use std::path::PathBuf;

mod menus;

pub struct Hud {
    pub settings: Settings,
    pub screen: Screen,
    /// The saves, for the title screen (the one written last first).
    pub saves: Vec<Save>,
    /// The save the title screen is asking whether to delete.
    pub confirm_delete: Option<PathBuf>,
    /// What is being opened, shown while it is (from the frame before, as it blocks).
    pub loading: Option<String>,
    pub settings_tab: SettingsTab,
    /// The sound's outputs, for the settings screen (kept up while its Audio tab is up).
    pub outputs: crate::audio::Outputs,
    pub fps: f32,
    pub frame_ms: f32,
    pub cpu_ms: f32,
    pub render_ms: f32,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Screen {
    /// The title screen, over the library last played.
    Title(TitlePage),
    /// A new game's opening words.
    Intro,
    Play,
    /// Esc while playing.
    Pause,
    /// Tab while playing.
    Grimoire,
    /// F9 while playing.
    Cheats,
    /// Reached from the title screen or the pause screen, where it goes back to.
    Settings {
        from_title: bool,
    },
}

impl Screen {
    /// Where Esc goes from here (from the stacks, to the pause screen); none from the
    /// title screen's first page.
    pub fn back(self) -> Option<Screen> {
        Some(match self {
            Screen::Play => Screen::Pause,
            Screen::Pause | Screen::Grimoire | Screen::Cheats | Screen::Intro => Screen::Play,
            Screen::Settings { from_title: false } => Screen::Pause,
            Screen::Settings { from_title: true } | Screen::Title(TitlePage::NewGame | TitlePage::Load) => {
                Screen::Title(TitlePage::Main)
            }
            Screen::Title(TitlePage::Main) => return None,
        })
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum SettingsTab {
    #[default]
    Video,
    Audio,
    Controls,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TitlePage {
    Main,
    /// Which library a new game is of.
    NewGame,
    Load,
}

pub enum HudAction {
    Buy(Upg),
    /// Use a skill at this level (0: off), up to the level learned.
    Use(Upg, u32),
    Cheat(Cheat),
    Show(Screen),
    /// Play on the save written last: the one open, or else it opened.
    Continue,
    /// Begin a new save of this library.
    NewGame(Layout),
    /// Open this save and play it.
    Open(Layout, PathBuf),
    Delete(PathBuf),
    /// Save, and go to the title screen.
    ToTitle,
    SettingsChanged,
    /// Back from the settings screen.
    SettingsDone,
    Quit,
}

const INK: Color32 = Color32::from_rgb(236, 224, 200);
const DIM: Color32 = Color32::from_rgb(170, 155, 130);
const GOLD: Color32 = Color32::from_rgb(232, 186, 96);

fn serif(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("serif".into()))
}

/// The symbols the HUD's face lacks (✦, ▼), from Noto Sans Symbols 2.
const SYMBOLS: &[u8] = include_bytes!("../assets/fonts/NotoSansSymbols2/NotoSansSymbols2-Subset.ttf");

/// The HUD is set in Libre Caslon Text, the Caslon of the books' spines, with
/// `SYMBOLS` for what it lacks: both built in (assets/fonts/<Family>: their licences
/// and sources), so the HUD reads the same on every system.
pub fn install_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    let mut family: Vec<String> = Vec::new();
    for (name, data) in [("serif", crate::text::CASLON), ("symbols", SYMBOLS)] {
        let data = std::sync::Arc::new(egui::FontData::from_static(data));
        fonts.font_data.insert(name.into(), data);
        family.push(name.into());
    }
    // Then egui's own proportional stack, for anything else.
    family.extend(fonts.families[&FontFamily::Proportional].iter().cloned());
    fonts.families.insert(FontFamily::Name("serif".into()), family);
    ctx.set_fonts(fonts);
}

fn rgb(c: [f32; 3]) -> Color32 {
    let f = |x: f32| (libm::powf(x.max(0.0), 1.0 / 2.2).min(1.0) * 255.0) as u8;
    Color32::from_rgb(f(c[0]), f(c[1]), f(c[2]))
}

fn bright(c: [f32; 3]) -> Color32 {
    let m = c[0].max(c[1]).max(c[2]).max(1e-3);
    let k = 0.85 / m;
    rgb([c[0] * k, c[1] * k, c[2] * k])
}

/// A group's name (a department's or category's, a series' title) and colour.
fn group_label(g: &Game, group: Group, key: usize) -> (String, Color32) {
    let name = match group {
        Group::Department => &g.lib.catalog.departments[key].name,
        Group::Category => &g.lib.catalog.categories[key].name,
        Group::Series => &g.lib.series[key].title,
    };
    (name.clone(), bright(g.group_color(group, key)))
}

fn project(vp: &Mat4, p: Vec3, size: Vec2) -> Option<Pos2> {
    let c = *vp * Vec4::new(p.x, p.y, p.z, 1.0);
    if c.w <= 0.05 {
        return None;
    }
    let n = c.truncate() / c.w;
    Some(Pos2::new(
        (n.x * 0.5 + 0.5) * size.x,
        (1.0 - (n.y * 0.5 + 0.5)) * size.y,
    ))
}

fn panel_frame() -> egui::Frame {
    egui::Frame::new()
        .fill(Color32::from_rgba_unmultiplied(18, 12, 10, 200))
        .stroke(Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(200, 160, 90, 90)))
        .corner_radius(6.0)
        .inner_margin(10.0)
}

pub fn draw(ctx: &egui::Context, g: &Game, hud: &mut Hud, aspect: f32) -> Vec<HudAction> {
    let mut actions = Vec::new();
    if hud.settings.show_fps {
        stats(ctx, g, hud);
    }
    if let Some(what) = &hud.loading {
        menus::loading(ctx, what);
        return actions;
    }
    match hud.screen {
        Screen::Play => play(ctx, g, hud, aspect),
        Screen::Grimoire => {
            play(ctx, g, hud, aspect);
            grimoire(ctx, g, &mut actions);
        }
        Screen::Cheats => {
            play(ctx, g, hud, aspect);
            cheat_window(ctx, g, &mut actions);
        }
        Screen::Title(page) => menus::title(ctx, hud, page, &mut actions),
        Screen::Intro => menus::intro(ctx, &mut actions),
        Screen::Pause => menus::pause(ctx, g, &mut actions),
        Screen::Settings { .. } => menus::settings(ctx, hud, &mut actions),
    }
    actions
}

/// What is drawn over the stacks while playing.
fn play(ctx: &egui::Context, g: &Game, hud: &Hud, aspect: f32) {
    let screen = ctx.content_rect().size();
    let vp = g.view_proj(aspect);
    let painter = ctx.layer_painter(egui::LayerId::background());
    let eye = g.player.eye();

    // ---- Signposts: the signs of the categories (and departments) you carry, from
    // anywhere on your floor (dimmed when hidden). Signs are otherwise lettered in the
    // world, so one in plain view is not lettered over: its name stands clear above
    // it, and once its own letters are big enough to read, only the marker does.
    let mut carrying = vec![false; g.lib.catalog.categories.len()];
    let mut carrying_dept = vec![false; g.lib.catalog.departments.len()];
    for &b in &g.held {
        let c = g.lib.category_of_book(b);
        carrying[c] = true;
        carrying_dept[g.lib.dept_of(c)] = true;
    }
    let here = g.lib.world.level_at(g.player.pos);
    for l in (g.lib.labels.iter()).filter(|_| g.level(Upg::Signposts) > 0) {
        let guide = match l.kind {
            LabelKind::Category(c) => carrying[c],
            LabelKind::Department(d) => carrying_dept[d],
        };
        if !guide || l.level != here {
            continue;
        }
        let to_eye = eye - l.pos;
        let facing = to_eye.dot(l.normal) > 0.0;
        let seen = facing && g.lib.world.line_of_sight(eye, l.pos);
        let Some(p) = project(&vp, l.pos, screen) else { continue };
        let size = (22.0 * 6.0 / to_eye.length().max(3.0)).clamp(16.0, 26.0);
        let pulse = (0.8 + 0.2 * libm::sinf(g.time * 3.0)) * if seen { 1.0 } else { 0.55 };
        let gold = GOLD.gamma_multiply(pulse);
        let marker = if seen {
            let top = project(&vp, l.pos + glam::Vec3::Y * (l.half + 0.03), screen).unwrap_or(p);
            let em_px = project(&vp, l.pos + glam::Vec3::Y * l.em, screen).map_or(0.0, |q| p.y - q.y);
            if em_px >= 12.0 {
                top
            } else {
                painter.text(top, Align2::CENTER_BOTTOM, &l.text, serif(size), gold);
                top + Vec2::new(0.0, -size)
            }
        } else {
            painter.text(p, Align2::CENTER_CENTER, &l.text, serif(size), gold);
            if !facing {
                // Cases are double-sided: say when the shelves are round the back.
                painter.text(
                    p + Vec2::new(0.0, size * 0.75),
                    Align2::CENTER_TOP,
                    "other side",
                    serif(size * 0.6),
                    GOLD.gamma_multiply(pulse * 0.8),
                );
            }
            p + Vec2::new(0.0, -size)
        };
        painter.text(marker, Align2::CENTER_BOTTOM, "▼", serif(size * 0.6), gold);
    }

    // ---- The compass: Sort Sense's marker on the slot, or Wayfinding's on the way
    // to the room.
    if let Some(sense) = g.sense
        && let Some(b) = g.active_book()
    {
        let s = &g.lib.series[g.lib.books[b as usize].series as usize];
        let (slot, col) = match sense.room {
            None => (g.lib.books[b as usize].slot_pos, bright(s.color)),
            Some(d) => {
                let first = g.lib.catalog.departments[d].categories.start;
                let c = catalog::category_color(&g.lib.catalog.categories[first]);
                (g.lib.rooms[d].door + glam::Vec3::Y * 1.6, bright(c))
            }
        };
        let center = Pos2::new(screen.x * 0.5, screen.y * 0.5);
        // When the slot is round the back of a case, point along the walk instead.
        let r = sense.route;
        let (target, dist, direct) = if r.direct {
            (slot, r.dist, true)
        } else {
            (r.waypoint + glam::Vec3::Y * 1.1, r.dist, false)
        };
        let label = match sense.room {
            Some(d) => format!("{} · {dist:.0} m", g.lib.catalog.departments[d].name),
            None => format!("{dist:.0} m"),
        };
        match project(&vp, target, screen) {
            Some(p) if p.x > 30.0 && p.x < screen.x - 30.0 && p.y > 30.0 && p.y < screen.y - 30.0 => {
                if direct {
                    let r = 7.0;
                    let pts = vec![
                        p + Vec2::new(0.0, -r),
                        p + Vec2::new(r, 0.0),
                        p + Vec2::new(0.0, r),
                        p + Vec2::new(-r, 0.0),
                    ];
                    painter.add(egui::Shape::convex_polygon(
                        pts,
                        Color32::TRANSPARENT,
                        Stroke::new(2.0_f32, col),
                    ));
                } else {
                    // A waypoint: a ring with a chevron toward the slot beyond it.
                    painter.circle_stroke(p, 8.0, Stroke::new(2.0_f32, col));
                    let ahead = project(&vp, slot, screen)
                        .map(|q| (q - p).normalized())
                        .filter(|d| d.is_finite() && d.length() > 0.5)
                        .unwrap_or(Vec2::new(0.0, -1.0));
                    let perp = Vec2::new(-ahead.y, ahead.x);
                    let tip = p + ahead * 18.0;
                    painter.line_segment([tip, tip - ahead * 6.0 + perp * 5.0], Stroke::new(2.0_f32, col));
                    painter.line_segment([tip, tip - ahead * 6.0 - perp * 5.0], Stroke::new(2.0_f32, col));
                }
                if dist > 3.0 || !direct || sense.room.is_some() {
                    painter.text(p + Vec2::new(0.0, 12.0), Align2::CENTER_TOP, label, serif(13.0), col);
                }
            }
            proj => {
                // Point along the screen edge.
                let dir = match proj {
                    Some(p) => (p - center).normalized(),
                    None => {
                        let rel = target - eye;
                        let r = g.player.right();
                        let x = rel.dot(r);
                        Vec2::new(if x >= 0.0 { 1.0 } else { -1.0 }, 0.0)
                    }
                };
                let edge = center + dir * (screen.y * 0.36);
                let perp = Vec2::new(-dir.y, dir.x);
                let tip = edge + dir * 14.0;
                painter.add(egui::Shape::convex_polygon(
                    vec![tip, edge - perp * 9.0, edge + perp * 9.0],
                    col.gamma_multiply(0.85),
                    Stroke::NONE,
                ));
                painter.text(edge - dir * 16.0, Align2::CENTER_CENTER, label, serif(13.0), col);
            }
        }
    }

    // ---- Glimpse: a marker on each carried series' run, through the walls (dimmer
    // on other floors).
    if g.spell_on(Upg::Glimpse) {
        let mut marked = std::collections::HashSet::new();
        for &b in &g.held {
            let info = &g.lib.books[b as usize];
            if !marked.insert(info.series) {
                continue;
            }
            let Some(p) = project(&vp, info.slot_pos, screen) else {
                continue;
            };
            let other_floor = g.lib.world.level_at(info.slot_pos) != here;
            let col =
                bright(g.lib.series[info.series as usize].color).gamma_multiply(if other_floor { 0.45 } else { 1.0 });
            let r = 6.0;
            let pts = vec![
                p + Vec2::new(0.0, -r),
                p + Vec2::new(r, 0.0),
                p + Vec2::new(0.0, r),
                p + Vec2::new(-r, 0.0),
            ];
            painter.add(egui::Shape::convex_polygon(
                pts,
                col,
                Stroke::new(1.0_f32, Color32::from_black_alpha(170)),
            ));
        }
    }

    // ---- Echo: a ring on each heap holding the kind listened for, once the sweep has
    // reached it, with how many lie there (dimmer on other floors).
    if let Some(t) = g.echo_age()
        && let Some((group, key)) = g.echo_kind
    {
        let col = bright(g.group_color(group, key));
        for &(p, n) in &g.echo {
            if p.distance(g.echo_from) > t * ECHO_SPEED {
                continue;
            }
            let Some(q) = project(&vp, p, screen) else { continue };
            let other_floor = g.lib.world.level_at(p) != here;
            let col = col.gamma_multiply(if other_floor { 0.45 } else { 1.0 });
            painter.circle_stroke(q, 7.0, Stroke::new(2.0_f32, col));
            if n > 1 {
                painter.text(
                    q + Vec2::new(0.0, 9.0),
                    Align2::CENTER_TOP,
                    n.to_string(),
                    serif(12.0),
                    col,
                );
            }
        }
    }

    // ---- Crosshair and aim tip.
    let c = Pos2::new(screen.x * 0.5, screen.y * 0.5);
    let mut tip: Option<(String, Color32)> = None;
    match g.aim {
        Aim::Loose { book, .. } => {
            painter.circle_stroke(c, 7.0, Stroke::new(1.5_f32, GOLD));
            let cat = g.lib.series[g.lib.books[book as usize].series as usize].category;
            let room = g.room_in_arms();
            let what = if room == 0 {
                "arms full".to_string()
            } else if g.level(Upg::Sift) == 0 {
                "tap take · hold scoop".to_string()
            } else if g.sought().is_some_and(|(group, key)| group.of(&g.lib, book) == key) {
                "tap take · hold scoop · F stop seeking".to_string()
            } else {
                format!("tap take · hold scoop · F seek {}", g.group.name())
            };
            let name = &g.lib.catalog.categories[cat].name;
            let many = g.lib.catalog.departments.len() > 2;
            let place = if many {
                format!("{name} ({})", g.lib.catalog.departments[g.lib.dept_of(cat)].name)
            } else {
                name.clone()
            };
            tip = Some((format!("{place} · {what}"), INK));
        }
        Aim::Shelf { face, row, run, .. } => {
            let f = &g.lib.faces[face as usize];
            match run {
                Some(r) => {
                    let s = &g.lib.series[r.series as usize];
                    let have = g
                        .held
                        .iter()
                        .filter(|&&b| g.lib.books[b as usize].series == r.series)
                        .count();
                    let done = g.series_shelved[r.series as usize];
                    let col = if have > 0 { GOLD } else { INK };
                    painter.circle_stroke(c, 6.0, Stroke::new(1.5_f32, col));
                    let mut extra = if have > 0 {
                        format!(" · file {have}")
                    } else {
                        String::new()
                    };
                    if g.level(Upg::Sift) > 0 {
                        let book = g.lib.series_books[r.series as usize][0];
                        if g.sought().is_some_and(|(group, key)| group.of(&g.lib, book) == key) {
                            extra.push_str(" · F stop seeking");
                        } else {
                            extra.push_str(&format!(" · F seek {}", g.group.name()));
                        }
                    }
                    tip = Some((format!("{} · {}/{} shelved{}", s.title, done, s.volumes, extra), col));
                }
                None => {
                    painter.circle_filled(c, 2.0, INK);
                    let mut t = format!(
                        "{} · case {} · shelf {}",
                        g.lib.catalog.categories[f.category].name,
                        f.case_idx + 1,
                        row as usize + 1
                    );
                    if let Some(keys) = g.lib.shelf_keys(face as usize, row as usize) {
                        t.push_str(&format!(" · {keys}"));
                    }
                    tip = Some((t, DIM));
                }
            }
        }
        Aim::Familiar(i) => {
            painter.circle_stroke(c, 7.0, Stroke::new(1.5_f32, GOLD));
            if let Some((role, _)) = g.familiars().nth(i) {
                tip = Some((format!("A {} at its post · G take it back", role.name()), INK));
            }
        }
        Aim::Clock(k) => {
            painter.circle_stroke(c, 7.0, Stroke::new(1.5_f32, INK));
            let key = g.lib.stopped[k].key;
            let s = &g.lib.series[key as usize];
            let done = g.series_shelved[key as usize];
            let t = if done == s.volumes {
                "A clock keeping time again".to_string()
            } else {
                // A key's volume may hang by another clock still stopped.
                let hung = (g.lib.series_books[key as usize].iter()).any(|&b| g.loose.is_frozen(b));
                let more = if hung {
                    " · a volume hangs by another clock"
                } else {
                    ""
                };
                format!(
                    "Stopped: starts when “{}” is filed · {done}/{} shelved{more}",
                    s.title, s.volumes
                )
            };
            tip = Some((t, INK));
        }
        Aim::None => {
            painter.circle_filled(c, 2.0, INK.gamma_multiply(0.7));
        }
    }
    if let Some((t, col)) = tip {
        let galley = painter.layout_no_wrap(t, serif(15.0), col);
        let r = Rect::from_center_size(c + Vec2::new(0.0, 30.0), galley.size() + Vec2::new(14.0, 6.0));
        painter.rect_filled(r, 4.0, Color32::from_rgba_unmultiplied(10, 8, 6, 150));
        painter.galley(r.min + Vec2::new(7.0, 3.0), galley, col);
    }
    // Lower down, out of the way of what you aim at, where the active book goes.
    let dest = g.active_book().map(|b| {
        let info = &g.lib.books[b as usize];
        let s = &g.lib.series[info.series as usize];
        let cat = &g.lib.catalog.categories[s.category];
        let mut t = cat.name.clone();
        if g.lib.room_at(g.player.pos) != Some(cat.dept) {
            t.push_str(&format!(" ({})", g.lib.catalog.departments[cat.dept].name));
        }
        t.push_str(&format!(" · {}", s.shelf_key()));
        if g.level(Upg::SortSense) > 0 {
            let f = &g.lib.faces[info.face as usize];
            t.push_str(&format!(" · case {}, shelf {}", f.case_idx + 1, info.row as usize + 1));
        }
        (t, bright(catalog::category_color(cat)))
    });
    if let Some((t, col)) = dest {
        let galley = painter.layout_no_wrap(t, serif(14.0), col);
        let at = c + Vec2::new(0.0, screen.y * 0.2);
        let r = Rect::from_center_size(at, galley.size() + Vec2::new(12.0, 4.0));
        painter.rect_filled(r, 4.0, Color32::from_rgba_unmultiplied(10, 8, 6, 90));
        painter.galley(r.min + Vec2::new(6.0, 2.0), galley, col);
    }

    // ---- Top-left: lumens and progress.
    egui::Area::new("stats".into())
        .anchor(Align2::LEFT_TOP, [16.0, 16.0])
        .interactable(false)
        .show(ctx, |ui| {
            panel_frame().show(ui, |ui| {
                ui.label(RichText::new(format!("✦ {}", g.lumens)).font(serif(24.0)).color(GOLD));
                let total = g.total_books();
                let done = g.total_shelved();
                ui.label(
                    RichText::new(format!("{done} / {total} books home"))
                        .font(serif(14.0))
                        .color(INK),
                );
                // A few departments are all listed; many, only the open ones and the next.
                let many = g.lib.catalog.departments.len() > 4;
                let next = g.next_gate().map(|gate| g.lib.world.gates()[gate].opens);
                for (d, dept) in g.lib.catalog.departments.iter().enumerate() {
                    if many && !g.dept_open(d) && Some(d) != next {
                        continue;
                    }
                    let name = &dept.name;
                    let frac = g.dept_shelved[d] as f32 / g.dept_total[d].max(1) as f32;
                    let label = if g.dept_open(d) {
                        format!("{name}  {:.0}%", frac * 100.0)
                    } else {
                        format!("{name}  (veiled)")
                    };
                    let (rect, _) = ui.allocate_exact_size(Vec2::new(220.0, 18.0), egui::Sense::hover());
                    let p = ui.painter();
                    p.rect_filled(rect, 3.0, Color32::from_rgba_unmultiplied(60, 45, 30, 160));
                    let mut fill = rect;
                    fill.set_width(rect.width() * frac.clamp(0.0, 1.0));
                    p.rect_filled(fill, 3.0, Color32::from_rgb(150, 105, 45));
                    p.text(
                        rect.left_center() + Vec2::new(6.0, 0.0),
                        Align2::LEFT_CENTER,
                        label,
                        serif(12.0),
                        INK,
                    );
                }
                if let Some(gate) = g.next_gate() {
                    let left = g.gate_books_left(gate);
                    let name = &g.lib.catalog.departments[g.lib.world.gates()[gate].opens].name;
                    ui.label(
                        RichText::new(format!("{left} more to open the {name}"))
                            .font(serif(12.0))
                            .color(DIM),
                    );
                }
            });
        });

    // ---- Bottom-left: active book card and armful.
    egui::Area::new("arms".into())
        .anchor(Align2::LEFT_BOTTOM, [16.0, -16.0])
        .interactable(false)
        .show(ctx, |ui| {
            panel_frame().show(ui, |ui| {
                ui.set_max_width(380.0);
                let mut status = format!(
                    "Arms  {}/{}  ·  sorting by {}",
                    g.carried(),
                    g.capacity(),
                    g.group.name()
                );
                if g.groups().len() > 1 {
                    status.push_str(" (R)");
                }
                let riding = g.familiars().filter(|&(_, posted)| !posted).count();
                if riding > 0 {
                    status.push_str(&format!("  ·  {riding} riding (G posts)"));
                }
                ui.label(RichText::new(status).font(serif(13.0)).color(DIM));
                if let Some((group, key)) = g.sought() {
                    let (name, col) = group_label(g, group, key);
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Seeking").font(serif(13.0)).color(DIM));
                        ui.label(RichText::new(name).font(serif(14.0)).color(col));
                        ui.label(RichText::new("(F at nothing stops)").font(serif(12.0)).color(DIM));
                    });
                }
                if let Some(b) = g.active_book() {
                    let info = &g.lib.books[b as usize];
                    let s = &g.lib.series[info.series as usize];
                    ui.label(RichText::new(&s.title).font(serif(19.0)).color(INK));
                    let vol = if s.volumes > 1 {
                        format!("Volume {} of {}", info.volume, s.volumes)
                    } else {
                        "Single volume".into()
                    };
                    ui.label(RichText::new(vol).font(serif(14.0)).color(INK));
                    let cat = &g.lib.catalog.categories[s.category];
                    let dept = &g.lib.catalog.departments[cat.dept].name;
                    let place = if g.level(Upg::SortSense) > 0 {
                        let f = &g.lib.faces[info.face as usize];
                        format!(
                            "{} · {} · case {}, shelf {} of {}",
                            cat.name,
                            dept,
                            f.case_idx + 1,
                            info.row as usize + 1,
                            ROWS
                        )
                    } else {
                        format!("{} · {} · under {}", cat.name, dept, s.shelf_key())
                    };
                    ui.label(
                        RichText::new(place)
                            .font(serif(14.0))
                            .color(bright(catalog::category_color(cat))),
                    );
                    // Strip of held spines.
                    let (rect, _) = ui.allocate_exact_size(Vec2::new(360.0, 26.0), egui::Sense::hover());
                    let p = ui.painter();
                    let n = g.held.len();
                    let w = (360.0 / n.max(1) as f32).min(14.0);
                    for (i, &hb) in g.held.iter().enumerate() {
                        let hs = &g.lib.series[g.lib.books[hb as usize].series as usize];
                        let x = rect.min.x + i as f32 * w;
                        let r =
                            Rect::from_min_size(Pos2::new(x, rect.min.y + 4.0), Vec2::new((w - 1.0).max(1.0), 20.0));
                        p.rect_filled(r, 1.0, bright(hs.color).gamma_multiply(0.8));
                        if i == g.active {
                            p.rect_stroke(
                                r.expand(1.5),
                                1.0,
                                Stroke::new(1.5_f32, GOLD),
                                egui::StrokeKind::Outside,
                            );
                        }
                    }
                } else {
                    ui.label(
                        RichText::new("Empty-handed. Tap a book to take it, or hold on a pile to scoop.")
                            .font(serif(15.0))
                            .color(INK),
                    );
                }
            });
        });

    // ---- Bottom middle: the spells learned, by key, with what is left of their
    // effect (gold) or how far they have recharged.
    let learned: Vec<(usize, Upg)> = (SPELLS.iter().copied().enumerate())
        .filter(|&(_, u)| g.level(u) > 0)
        .collect();
    if !learned.is_empty() {
        egui::Area::new("spells".into())
            .anchor(Align2::CENTER_BOTTOM, [0.0, -64.0])
            .interactable(false)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    for (i, u) in learned {
                        let (lasts, recharge) = spell_times(u, g.level(u));
                        let (left, wait) = (g.spell_left[i], g.spell_wait[i]);
                        panel_frame().show(ui, |ui| {
                            let ready = wait <= 0.0 || left > 0.0;
                            let name = format!("{}  {}", i + 1, upg_def(u).name);
                            ui.label(
                                RichText::new(name)
                                    .font(serif(14.0))
                                    .color(if ready { INK } else { DIM }),
                            );
                            let (rect, _) = ui.allocate_exact_size(Vec2::new(96.0, 4.0), egui::Sense::hover());
                            let (frac, col) = if left > 0.0 {
                                (left / lasts, GOLD)
                            } else {
                                (1.0 - wait / recharge, if wait > 0.0 { DIM } else { GOLD })
                            };
                            let p = ui.painter();
                            p.rect_filled(rect, 2.0, Color32::from_black_alpha(120));
                            let mut fill = rect;
                            fill.set_width(rect.width() * frac.clamp(0.0, 1.0));
                            p.rect_filled(fill, 2.0, col);
                        });
                    }
                });
            });
    }

    // ---- Bottom-right: controls reminder.
    egui::Area::new("keys".into())
        .anchor(Align2::RIGHT_BOTTOM, [-16.0, -16.0])
        .interactable(false)
        .show(ctx, |ui| {
            // Wrapped clear of the armful's card on a narrow screen (or a big interface).
            ui.set_max_width((screen.x - 460.0).max(220.0));
            let m = if hud.settings.music { "on" } else { "off" };
            ui.label(
                RichText::new(format!(
                    "Tap take · Hold scoop · Click a run file · Hold right toss · X throw all · F seek · R sort by · 1–5 spells · Scroll cycle · Tab Grimoire · M music ({m}) · Esc pause"
                ))
                .font(serif(12.0))
                .color(DIM.gamma_multiply(0.8)),
            );
        });

    // ---- Toasts and hints.
    egui::Area::new("toasts".into())
        .anchor(Align2::CENTER_TOP, [0.0, 24.0])
        .interactable(false)
        .show(ctx, |ui| {
            ui.set_width(screen.x * 0.8);
            ui.vertical_centered(|ui| {
                for t in g.toasts.iter().rev().take(5) {
                    let life = if t.big { 6.0 } else { 3.5 };
                    let a = (t.age / 0.3).min(1.0) * ((life - t.age) / 0.8).clamp(0.0, 1.0);
                    let size = if t.big { 26.0 } else { 17.0 };
                    ui.label(
                        RichText::new(&t.text)
                            .font(serif(size))
                            .color(bright(t.color).gamma_multiply(a)),
                    );
                }
            });
        });
    if let Some((h, t)) = &g.hint {
        let a = t.min(1.0);
        let galley = painter.layout_no_wrap(h.clone(), serif(15.0), INK.gamma_multiply(a));
        let r = Rect::from_center_size(c + Vec2::new(0.0, 70.0), galley.size() + Vec2::new(14.0, 6.0));
        painter.rect_filled(r, 4.0, Color32::from_rgba_unmultiplied(10, 8, 6, (150.0 * a) as u8));
        painter.galley(r.min + Vec2::new(7.0, 3.0), galley, INK);
    }
}

/// The frame rate and timings (F3).
fn stats(ctx: &egui::Context, g: &Game, hud: &Hud) {
    egui::Area::new("perf".into())
        .anchor(Align2::RIGHT_TOP, [-16.0, 16.0])
        .interactable(false)
        .show(ctx, |ui| {
            ui.label(
                RichText::new(format!(
                    "{:.0} fps · {:.2} ms\ncpu {:.2} ms · render {:.2} ms\n{} instances",
                    hud.fps,
                    hud.frame_ms,
                    hud.cpu_ms,
                    hud.render_ms,
                    g.dyn_insts.len() + g.trans_insts.len()
                ))
                .monospace()
                .color(INK),
            );
        });
}

/// The Grimoire (Tab), where lumens buy skills.
fn grimoire(ctx: &egui::Context, g: &Game, actions: &mut Vec<HudAction>) {
    let screen = ctx.content_rect().size();
    let mut open = true;
    egui::Window::new(RichText::new("The Grimoire").font(serif(24.0)).color(GOLD))
        .collapsible(false)
        .resizable(false)
        .anchor(Align2::CENTER_CENTER, [0.0, 0.0])
        .frame(panel_frame().fill(Color32::from_rgba_unmultiplied(24, 16, 12, 240)))
        .open(&mut open)
        .show(ctx, |ui| {
            ui.set_width(560.0);
            ui.label(
                RichText::new(format!("You have ✦ {} lumens.", g.lumens))
                    .font(serif(16.0))
                    .color(INK),
            );
            ui.add_space(6.0);
            egui::ScrollArea::vertical().max_height(screen.y * 0.7).show(ui, |ui| {
                for tier in Tier::ALL {
                    let skills: Vec<Upg> = (UPGRADES.iter().copied())
                        .filter(|&u| upg_def(u).tier == tier && !upg_costs(g.layout, u).is_empty())
                        .collect();
                    if skills.is_empty() {
                        continue;
                    }
                    ui.add_space(6.0);
                    ui.label(RichText::new(tier.name()).font(serif(19.0)).color(GOLD));
                    ui.label(RichText::new(tier.blurb()).font(serif(12.0)).color(DIM).italics());
                    ui.separator();
                    for u in skills {
                        grimoire_entry(ui, g, u, actions);
                    }
                }
            });
            ui.add_space(8.0);
            ui.label(
                RichText::new("Press Tab to return to the stacks.")
                    .font(serif(13.0))
                    .color(DIM),
            );
        });
    if !open {
        actions.push(HudAction::Show(Screen::Play));
    }
}

/// The cheat window (F9), for trying late-game situations.
fn cheat_window(ctx: &egui::Context, g: &Game, actions: &mut Vec<HudAction>) {
    let mut open = true;
    egui::Window::new(RichText::new("Cheats").font(serif(22.0)).color(GOLD))
        .collapsible(false)
        .resizable(false)
        .anchor(Align2::RIGHT_CENTER, [-24.0, 0.0])
        .frame(panel_frame().fill(Color32::from_rgba_unmultiplied(24, 16, 12, 240)))
        .open(&mut open)
        .show(ctx, |ui| {
            ui.set_width(340.0);
            for style in [egui::TextStyle::Body, egui::TextStyle::Button] {
                ui.style_mut().text_styles.insert(style, serif(14.0));
            }
            let note = |ui: &mut egui::Ui, text: &str| {
                ui.label(RichText::new(text).font(serif(12.0)).color(DIM));
            };
            let heading = |ui: &mut egui::Ui, text: String| {
                ui.add_space(4.0);
                ui.label(RichText::new(text).font(serif(16.0)).color(INK));
            };
            let mut button = |ui: &mut egui::Ui, text: &str, c: Cheat| {
                if ui.button(text).clicked() {
                    actions.push(HudAction::Cheat(c));
                }
            };
            note(ui, "These change this library's save; Start over (Esc) undoes them.");
            heading(ui, format!("Lumens  ✦ {}", g.lumens));
            ui.horizontal(|ui| {
                for n in [1_000, 10_000, 100_000] {
                    button(ui, &format!("+{n}"), Cheat::Lumens(n));
                }
            });
            heading(ui, "Skills and veils".into());
            ui.horizontal(|ui| {
                button(ui, "Learn every skill", Cheat::AllSkills);
                button(ui, "Lift every veil", Cheat::LiftVeils);
            });
            note(ui, "Lifted veils return when the library is next loaded; shelving keeps them open.");
            let done = 100.0 * g.total_shelved() as f32 / g.total_books().max(1) as f32;
            heading(ui, format!("Shelve the library  ({done:.0}% home)"));
            ui.horizontal(|ui| {
                for f in [0.25, 0.5, 0.8, 0.95] {
                    button(ui, &format!("{:.0}%", f * 100.0), Cheat::Shelve(f));
                }
            });
            note(
                ui,
                "Files books from the piles until every open department is that far home, then the ones whose veils that lifts. The big steps take a few seconds.",
            );
            heading(ui, "Finish".into());
            ui.horizontal(|ui| {
                if let Some(d) = g.lib.room_at(g.player.pos) {
                    button(ui, "The room you are in", Cheat::Finish(Finish::Room(d)));
                }
                if g.lib.storeys() > 1 {
                    let l = g.lib.world.level_at(g.player.pos);
                    button(ui, "This storey", Cheat::Finish(Finish::Storey(l)));
                }
                button(ui, "Everything", Cheat::Finish(Finish::Library));
            });
            note(ui, "Shelves every book of it at once, so it is finished and celebrates.");
            heading(ui, "Go to".into());
            egui::ScrollArea::vertical().max_height(260.0).show(ui, |ui| {
                button(ui, "Where the game starts", Cheat::GoTo(None));
                for (d, dept) in g.lib.catalog.departments.iter().enumerate() {
                    let name = if g.dept_open(d) {
                        dept.name.clone()
                    } else {
                        format!("{} (veiled)", dept.name)
                    };
                    button(ui, &name, Cheat::GoTo(Some(d)));
                }
            });
        });
    if !open {
        actions.push(HudAction::Show(Screen::Play));
    }
}

/// One skill in the Grimoire: its name, level and blurb, the level it is used at
/// (once learned; any level up to that, or off), and the button to learn the next.
fn grimoire_entry(ui: &mut egui::Ui, g: &Game, u: Upg, actions: &mut Vec<HudAction>) {
    let def = upg_def(u);
    let lvl = g.learned(u);
    let max = upg_costs(g.layout, u).len() as u32;
    ui.horizontal(|ui| {
        ui.allocate_ui_with_layout(Vec2::new(420.0, 0.0), egui::Layout::top_down(egui::Align::Min), |ui| {
            ui.set_width(420.0);
            let title = if max > 1 {
                format!("{}  ·  {}/{}", def.name, lvl, max)
            } else {
                def.name.to_string()
            };
            ui.label(RichText::new(title).font(serif(17.0)).color(INK));
            ui.label(RichText::new(def.blurb).font(serif(13.0)).color(DIM));
            if lvl > 0 {
                ui.horizontal(|ui| {
                    // The level in use marked in the progress bars' brown, not egui's blue.
                    ui.visuals_mut().selection.bg_fill = Color32::from_rgb(110, 76, 34);
                    ui.label(RichText::new("In use:").font(serif(13.0)).color(DIM));
                    let roman = ["I", "II", "III", "IV", "V"];
                    for l in 0..=lvl {
                        let name = match l {
                            0 => "off",
                            _ if max == 1 => "on",
                            _ => roman[(l as usize - 1).min(roman.len() - 1)],
                        };
                        let on = g.level(u) == l;
                        let text = RichText::new(name).font(serif(13.0)).color(if on { GOLD } else { DIM });
                        if ui.selectable_label(on, text).clicked() && !on {
                            actions.push(HudAction::Use(u, l));
                        }
                    }
                });
            }
        });
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            match g.next_cost(u) {
                Some(cost) => {
                    let afford = g.lumens >= cost;
                    let btn =
                        egui::Button::new(RichText::new(format!("✦ {cost}")).font(serif(16.0)).color(if afford {
                            GOLD
                        } else {
                            DIM
                        }))
                        .min_size(Vec2::new(96.0, 32.0));
                    if ui.add_enabled(afford, btn).clicked() {
                        actions.push(HudAction::Buy(u));
                    }
                }
                None => {
                    let done = if max > 1 { "mastered" } else { "learned" };
                    ui.label(RichText::new(done).font(serif(14.0)).color(GOLD));
                }
            }
        });
    });
    ui.add_space(4.0);
}
