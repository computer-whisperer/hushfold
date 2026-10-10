//! egui HUD: crosshair, armful, active-book card, world labels, toasts, Grimoire.

use crate::catalog;
use crate::game::{Aim, Cheat, ECHO_SPEED, Game, Group, SPELLS, Tier, UPGRADES, Upg, spell_times, upg_costs, upg_def};
use crate::library::{LabelKind, Layout, ROWS};
use crate::settings::Settings;
use egui::{Align2, Color32, FontFamily, FontId, Pos2, Rect, RichText, Stroke, Vec2};
use glam::{Mat4, Vec3, Vec4};

pub struct Hud {
    pub settings: Settings,
    pub menu_open: bool,
    /// The cheat window (F9).
    pub cheats_open: bool,
    pub show_intro: bool,
    /// The pause screen is asking whether to start over.
    pub confirm_restart: bool,
    pub show_stats: bool,
    pub fps: f32,
    pub frame_ms: f32,
    pub cpu_ms: f32,
    pub render_ms: f32,
}

pub enum HudAction {
    Buy(Upg),
    Cheat(Cheat),
    CloseCheats,
    Begin,
    CloseMenu,
    SettingsChanged,
    Quit,
    /// Start this library over (confirmed).
    NewGame,
    /// Go to the other library (its save, or a fresh one).
    Switch(Layout),
}

const INK: Color32 = Color32::from_rgb(236, 224, 200);
const DIM: Color32 = Color32::from_rgb(170, 155, 130);
const GOLD: Color32 = Color32::from_rgb(232, 186, 96);

fn serif(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("serif".into()))
}

pub fn install_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    let mut family: Vec<String> = Vec::new();
    for &path in crate::platform::SERIF_FONTS {
        if let Ok(bytes) = std::fs::read(path) {
            fonts
                .font_data
                .insert("serif".into(), std::sync::Arc::new(egui::FontData::from_owned(bytes)));
            family.push("serif".into());
            break;
        }
    }
    for &path in crate::platform::SYMBOL_FONTS {
        if let Ok(bytes) = std::fs::read(path) {
            fonts
                .font_data
                .insert(path.into(), std::sync::Arc::new(egui::FontData::from_owned(bytes)));
            family.push(path.into());
        }
    }
    // Fall back to egui's proportional stack.
    family.extend(fonts.families[&FontFamily::Proportional].iter().cloned());
    fonts.families.insert(FontFamily::Name("serif".into()), family);
    ctx.set_fonts(fonts);
}

fn rgb(c: [f32; 3]) -> Color32 {
    let f = |x: f32| (x.max(0.0).powf(1.0 / 2.2).min(1.0) * 255.0) as u8;
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
    let screen = ctx.content_rect().size();
    let vp = g.view_proj(aspect);
    let painter = ctx.layer_painter(egui::LayerId::background());
    let eye = g.player.eye();

    // ---- World labels.
    let mut carrying = vec![false; g.lib.catalog.categories.len()];
    let mut carrying_dept = vec![false; g.lib.catalog.departments.len()];
    for &b in &g.held {
        let c = g.lib.category_of_book(b);
        carrying[c] = true;
        carrying_dept[g.lib.dept_of(c)] = true;
    }
    let here = g.lib.world.level_at(g.player.pos);
    // Gather first so guide signs can be drawn last and clear the space under them.
    let mut plain: Vec<(Pos2, &str, FontId, Color32, Align2)> = Vec::new();
    let mut guides: Vec<(Pos2, &str, f32, f32, bool)> = Vec::new();
    for l in &g.lib.labels {
        let to_eye = eye - l.pos;
        let d = to_eye.length();
        // With Signposts, signs for the categories (and departments) you carry show
        // from anywhere on your floor (dimmed when hidden).
        let (is_sign, guide, done) = match l.kind {
            LabelKind::Shelf => (false, false, false),
            LabelKind::Category(c) => (true, carrying[c], g.category_shelved[c] == g.lib.category_books[c]),
            LabelKind::Department(d) => (true, carrying_dept[d], g.dept_shelved[d] == g.dept_total[d]),
        };
        let guide = guide && l.level == here && g.level(Upg::Signposts) > 0;
        let facing = to_eye.dot(l.normal) > 0.0;
        if !facing && !guide {
            continue;
        }
        let (max_d, size) = if is_sign {
            (34.0, (22.0 * 6.0 / d.max(3.0)).clamp(10.0, 26.0))
        } else {
            (3.2, 12.0)
        };
        if d > max_d && !guide {
            continue;
        }
        let seen = facing && g.lib.world.line_of_sight(eye, l.pos);
        if !seen && !guide {
            continue;
        }
        let Some(p) = project(&vp, l.pos, screen) else { continue };
        let fade = ((max_d - d) / (max_d * 0.3)).clamp(0.0, 1.0);
        if guide {
            let pulse = (0.8 + 0.2 * (g.time * 3.0).sin()) * if seen { 1.0 } else { 0.55 };
            guides.push((p, &l.text, size.max(16.0), pulse, facing));
        } else if is_sign {
            let col = if done { Color32::from_rgb(255, 236, 190) } else { INK };
            plain.push((p, &l.text, serif(size), col.gamma_multiply(fade), Align2::CENTER_CENTER));
        } else {
            plain.push((
                p + Vec2::new(0.0, 2.0),
                &l.text,
                serif(size),
                DIM.gamma_multiply(fade),
                Align2::CENTER_TOP,
            ));
        }
    }
    for (p, text, font, col, align) in plain {
        if guides
            .iter()
            .any(|(gp, _, size, _, _)| (gp.x - p.x).abs() < size * 5.0 && (gp.y - p.y).abs() < size * 1.6)
        {
            continue;
        }
        painter.text(p, align, text, font, col);
    }
    for (p, text, size, pulse, facing) in guides {
        painter.text(p, Align2::CENTER_CENTER, text, serif(size), GOLD.gamma_multiply(pulse));
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
        painter.text(
            p + Vec2::new(0.0, -size),
            Align2::CENTER_BOTTOM,
            "▼",
            serif(size * 0.6),
            GOLD.gamma_multiply(pulse),
        );
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
            let room = g.capacity().saturating_sub(g.held.len());
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
                    let extra = if have > 0 {
                        format!(" · file {have}")
                    } else {
                        String::new()
                    };
                    tip = Some((format!("{} · {}/{} shelved{}", s.title, done, s.volumes, extra), col));
                }
                None => {
                    painter.circle_filled(c, 2.0, INK);
                    tip = Some((
                        format!(
                            "{} · case {} · shelf {}",
                            g.lib.catalog.categories[f.category].name,
                            f.case_idx + 1,
                            row as usize + 1
                        ),
                        DIM,
                    ));
                }
            }
        }
        Aim::Familiar(i) => {
            painter.circle_stroke(c, 7.0, Stroke::new(1.5_f32, GOLD));
            if let Some((role, _)) = g.familiars().nth(i) {
                tip = Some((format!("A {} at its post · G take it back", role.name()), INK));
            }
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
                    g.held.len(),
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
                    let f = &g.lib.faces[info.face as usize];
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

    if hud.show_stats {
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

    // ---- Grimoire.
    if hud.menu_open {
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
                            grimoire_entry(ui, g, u, &mut actions);
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
            actions.push(HudAction::CloseMenu);
        }
    }

    if hud.cheats_open {
        cheat_window(ctx, g, &mut actions);
    }

    // ---- Intro / pause.
    if hud.show_intro {
        let fresh = g.total_shelved() == 0 && g.held.is_empty() && g.lumens == 0;
        egui::Window::new(RichText::new("Hushfold Library").font(serif(30.0)).color(GOLD))
            .collapsible(false)
            .resizable(false)
            .title_bar(true)
            .anchor(Align2::CENTER_CENTER, [0.0, 0.0])
            .frame(panel_frame().fill(Color32::from_rgba_unmultiplied(24, 16, 12, 240)))
            .show(ctx, |ui| {
                ui.set_width(520.0);
                for style in [egui::TextStyle::Body, egui::TextStyle::Button] {
                    ui.style_mut().text_styles.insert(style, serif(15.0));
                }
                // Which library: each keeps its own place.
                ui.horizontal(|ui| {
                    for layout in Layout::ALL {
                        let here = layout == g.layout;
                        let text = RichText::new(layout.name())
                            .font(serif(15.0))
                            .color(if here { GOLD } else { DIM });
                        let b = egui::Button::new(text).selected(here).frame(here);
                        if ui.add(b).on_hover_text(layout.blurb()).clicked() && !here {
                            actions.push(HudAction::Switch(layout));
                        }
                    }
                });
                ui.add_space(6.0);
                if fresh {
                    let body = "The night librarian has gone missing, and the library has come undone. \
                        Thousands of books lie in piles across the floor.\n\n\
                        Tap a book to take it, or hold on a pile to scoop an armful. Then find each book's home: \
                        its category's bookcases (look for the signs), the shelf whose letters take its title, \
                        and its series' run of outlines in its colour. Click anywhere on the run to file a volume.\n\n\
                        Every book you file earns a lumen ✦. Spend them in the Grimoire to make the work lighter.";
                    ui.label(RichText::new(body).font(serif(15.0)).color(INK));
                } else {
                    let mins = (g.play_time / 60.0) as u64;
                    let time = if mins >= 60 {
                        format!("{} h {} min", mins / 60, mins % 60)
                    } else {
                        format!("{mins} min")
                    };
                    let done = g.total_shelved() == g.total_books();
                    let summary = if done {
                        format!("Every book is home · {time} in the stacks")
                    } else {
                        format!(
                            "{} of {} books are home · {time} in the stacks",
                            g.total_shelved(),
                            g.total_books()
                        )
                    };
                    ui.label(RichText::new(summary).font(serif(16.0)).color(INK));
                    let note = if done {
                        "The library is in order. Start over for a fresh one."
                    } else {
                        "Your place is kept; progress saves as you go."
                    };
                    ui.label(RichText::new(note).font(serif(13.0)).color(DIM));
                }
                ui.add_space(8.0);
                ui.label(
                    RichText::new(
                        "WASD walk · Shift hurry · Space jump · Mouse look\n\
                         Tap a book to take it · Hold on a pile to scoop · Click a run to file\n\
                         Hold right mouse to toss a group, release to throw · X the whole armful\n\
                         F seek a book's kind · R sort by department, category or series\n\
                         1–5 cast the spells you have learned\n\
                         G post a familiar where you aim, or take one back\n\
                         Scroll or Q/E cycle the active book · Tab Grimoire\n\
                         M music · F11 fullscreen · F12 screenshot · Esc pause",
                    )
                    .font(serif(13.0))
                    .color(DIM),
                );
                ui.add_space(6.0);
                egui::CollapsingHeader::new(RichText::new("Settings").font(serif(15.0)).color(INK))
                    .default_open(false)
                    .show(ui, |ui| {
                        let st = &mut hud.settings;
                        let before = *st;
                        ui.add(
                            egui::Slider::new(&mut st.look_speed, 0.2..=4.0)
                                .logarithmic(true)
                                .fixed_decimals(2)
                                .text("Look speed"),
                        );
                        ui.checkbox(&mut st.invert_look, "Invert vertical look");
                        ui.add(
                            egui::Slider::new(&mut st.fov, 55.0..=90.0)
                                .suffix("°")
                                .integer()
                                .text("Field of view"),
                        );
                        fn pct(v: &mut f32) -> egui::Slider<'_> {
                            egui::Slider::new(v, 0.0..=1.0).custom_formatter(|x, _| format!("{:.0}%", x * 100.0))
                        }
                        let music = if st.music { "Music" } else { "Music (off: press M)" };
                        ui.add(pct(&mut st.music_volume).text(music));
                        ui.add(pct(&mut st.effects_volume).text("Effects and ambience"));
                        if *st != before {
                            actions.push(HudAction::SettingsChanged);
                        }
                    });
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    let label = if fresh { "Begin" } else { "Return to the stacks" };
                    let w = ui.available_width();
                    ui.add_space((w - 220.0 - 160.0 - 12.0).max(0.0) * 0.5);
                    if ui
                        .add(
                            egui::Button::new(RichText::new(label).font(serif(20.0)).color(GOLD))
                                .min_size(Vec2::new(220.0, 40.0)),
                        )
                        .clicked()
                    {
                        actions.push(HudAction::Begin);
                    }
                    ui.add_space(12.0);
                    if ui
                        .add(
                            egui::Button::new(RichText::new("Save and quit").font(serif(16.0)).color(DIM))
                                .min_size(Vec2::new(160.0, 40.0)),
                        )
                        .clicked()
                    {
                        actions.push(HudAction::Quit);
                    }
                });
                if !fresh {
                    ui.add_space(8.0);
                    restart_controls(ui, hud, &mut actions);
                }
            });
    }
    actions
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
        actions.push(HudAction::CloseCheats);
    }
}

/// One skill in the Grimoire: its name, level and blurb, and the button to learn
/// the next level.
fn grimoire_entry(ui: &mut egui::Ui, g: &Game, u: Upg, actions: &mut Vec<HudAction>) {
    let def = upg_def(u);
    let lvl = g.level(u);
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

/// "Start over…", then a confirmation, on the pause screen.
fn restart_controls(ui: &mut egui::Ui, hud: &mut Hud, actions: &mut Vec<HudAction>) {
    ui.vertical_centered(|ui| {
        if !hud.confirm_restart {
            let link = egui::Button::new(RichText::new("Start over…").font(serif(14.0)).color(DIM)).frame(false);
            if ui.add(link).clicked() {
                hud.confirm_restart = true;
            }
            return;
        }
        ui.label(
            RichText::new(
                "Start this library over? Every book goes back on the floor, and its lumens and \
                 upgrades are lost. Settings, and the other library, are kept.",
            )
            .font(serif(14.0))
            .color(INK),
        );
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            let w = ui.available_width();
            ui.add_space((w - 160.0 * 2.0 - 12.0).max(0.0) * 0.5);
            let size = Vec2::new(160.0, 34.0);
            let start = egui::Button::new(RichText::new("Start over").font(serif(16.0)).color(GOLD)).min_size(size);
            if ui.add(start).clicked() {
                actions.push(HudAction::NewGame);
            }
            ui.add_space(12.0);
            let keep = egui::Button::new(RichText::new("Keep playing").font(serif(16.0)).color(DIM)).min_size(size);
            if ui.add(keep).clicked() {
                hud.confirm_restart = false;
            }
        });
    });
}
