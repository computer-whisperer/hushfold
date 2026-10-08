//! egui HUD: crosshair, armful, active-book card, world labels, toasts, Grimoire.

use crate::catalog::{self, CATEGORIES};
use crate::game::{upg_def, Aim, Game, Upg, UPGRADES};
use crate::library::{wing_of_category, ROWS};
use egui::{Align2, Color32, FontFamily, FontId, Pos2, Rect, RichText, Stroke, Vec2};
use glam::{Mat4, Vec3, Vec4};

pub struct Hud {
    pub menu_open: bool,
    pub show_intro: bool,
    pub show_stats: bool,
    pub fps: f32,
    pub frame_ms: f32,
}

pub enum HudAction {
    Buy(Upg),
    Begin,
    CloseMenu,
}

const INK: Color32 = Color32::from_rgb(236, 224, 200);
const DIM: Color32 = Color32::from_rgb(170, 155, 130);
const GOLD: Color32 = Color32::from_rgb(232, 186, 96);

fn serif(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("serif".into()))
}

pub fn install_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    let candidates = [
        "/usr/share/fonts/noto/NotoSerif-Regular.ttf",
        "/usr/share/fonts/TTF/DejaVuSerif.ttf",
        "/usr/share/fonts/liberation/LiberationSerif-Regular.ttf",
        "/usr/share/fonts/truetype/dejavu/DejaVuSerif.ttf",
    ];
    let mut family: Vec<String> = Vec::new();
    for path in candidates {
        if let Ok(bytes) = std::fs::read(path) {
            fonts.font_data.insert("serif".into(), std::sync::Arc::new(egui::FontData::from_owned(bytes)));
            family.push("serif".into());
            break;
        }
    }
    for path in ["/usr/share/fonts/noto/NotoSansSymbols2-Regular.ttf", "/usr/share/fonts/noto/NotoSansSymbols-Regular.ttf"] {
        if let Ok(bytes) = std::fs::read(path) {
            fonts.font_data.insert(path.into(), std::sync::Arc::new(egui::FontData::from_owned(bytes)));
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

fn project(vp: &Mat4, p: Vec3, size: Vec2) -> Option<Pos2> {
    let c = *vp * Vec4::new(p.x, p.y, p.z, 1.0);
    if c.w <= 0.05 {
        return None;
    }
    let n = c.truncate() / c.w;
    Some(Pos2::new((n.x * 0.5 + 0.5) * size.x, (1.0 - (n.y * 0.5 + 0.5)) * size.y))
}

fn panel_frame() -> egui::Frame {
    egui::Frame::new()
        .fill(Color32::from_rgba_unmultiplied(18, 12, 10, 200))
        .stroke(Stroke::new(1.0, Color32::from_rgba_unmultiplied(200, 160, 90, 90)))
        .corner_radius(6.0)
        .inner_margin(10.0)
}

pub fn draw(ctx: &egui::Context, g: &Game, hud: &mut Hud, aspect: f32, music_on: bool) -> Vec<HudAction> {
    let mut actions = Vec::new();
    let screen = ctx.content_rect().size();
    let vp = g.view_proj(aspect);
    let painter = ctx.layer_painter(egui::LayerId::background());
    let eye = g.player.eye();

    // ---- World labels.
    for l in &g.lib.labels {
        let to_eye = eye - l.pos;
        let d = to_eye.length();
        if to_eye.dot(l.normal) <= 0.0 {
            continue;
        }
        let (max_d, size) = if l.is_sign { (34.0, (22.0 * 6.0 / d.max(3.0)).clamp(10.0, 26.0)) } else { (3.2, 12.0) };
        if d > max_d {
            continue;
        }
        if !g.line_of_sight(eye, l.pos) {
            continue;
        }
        let Some(p) = project(&vp, l.pos, screen) else { continue };
        let fade = ((max_d - d) / (max_d * 0.3)).clamp(0.0, 1.0);
        if l.is_sign {
            let done = g.category_shelved[l.category] == g.lib.category_books[l.category];
            let col = if done { Color32::from_rgb(255, 236, 190) } else { INK };
            let col = col.gamma_multiply(fade);
            painter.text(p, Align2::CENTER_CENTER, &l.text, serif(size), col);
        } else {
            let col = DIM.gamma_multiply(fade);
            painter.text(p + Vec2::new(0.0, 2.0), Align2::CENTER_TOP, &l.text, serif(size), col);
        }
    }

    // ---- Sort Sense marker.
    if g.level(Upg::SortSense) > 0 {
        if let Some(b) = g.active_book() {
            let target = g.lib.books[b as usize].slot_pos;
            let s = &g.lib.series[g.lib.books[b as usize].series as usize];
            let col = bright(s.color);
            let center = Pos2::new(screen.x * 0.5, screen.y * 0.5);
            let dist = target.distance(eye);
            match project(&vp, target, screen) {
                Some(p) if p.x > 30.0 && p.x < screen.x - 30.0 && p.y > 30.0 && p.y < screen.y - 30.0 => {
                    let r = 7.0;
                    let pts = vec![p + Vec2::new(0.0, -r), p + Vec2::new(r, 0.0), p + Vec2::new(0.0, r), p + Vec2::new(-r, 0.0)];
                    painter.add(egui::Shape::convex_polygon(pts, Color32::TRANSPARENT, Stroke::new(2.0, col)));
                    if dist > 3.0 {
                        painter.text(p + Vec2::new(0.0, 12.0), Align2::CENTER_TOP, format!("{dist:.0} m"), serif(13.0), col);
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
                    painter.text(edge - dir * 16.0, Align2::CENTER_CENTER, format!("{dist:.0} m"), serif(13.0), col);
                }
            }
        }
    }

    // ---- Crosshair and aim tip.
    let c = Pos2::new(screen.x * 0.5, screen.y * 0.5);
    let mut tip: Option<(String, Color32)> = None;
    match g.aim {
        Aim::Heap { heap, .. } => {
            painter.circle_stroke(c, 7.0, Stroke::new(1.5, GOLD));
            let n = g.lib.heaps[heap as usize].books.len();
            let room = g.capacity().saturating_sub(g.held.len());
            let t = if room == 0 { format!("Heap · {n} books · arms full") } else { format!("Heap · {n} books · gather {}", room.min(n)) };
            tip = Some((t, INK));
        }
        Aim::Shelf { face, row, run, .. } => {
            let f = &g.lib.faces[face as usize];
            match run {
                Some(r) => {
                    let s = &g.lib.series[r.series as usize];
                    let have = g.held.iter().filter(|&&b| g.lib.books[b as usize].series == r.series).count();
                    let done = g.series_shelved[r.series as usize];
                    let col = if have > 0 { GOLD } else { INK };
                    painter.circle_stroke(c, 6.0, Stroke::new(1.5, col));
                    let extra = if have > 0 { format!(" · file {have}") } else { String::new() };
                    tip = Some((format!("{} · {}/{} shelved{}", s.title, done, s.volumes, extra), col));
                }
                None => {
                    painter.circle_filled(c, 2.0, INK);
                    tip = Some((format!("{} · case {} · shelf {}", CATEGORIES[f.category].name, f.case_idx + 1, row as usize + 1), DIM));
                }
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
    egui::Area::new("stats".into()).anchor(Align2::LEFT_TOP, [16.0, 16.0]).interactable(false).show(ctx, |ui| {
        panel_frame().show(ui, |ui| {
            ui.label(RichText::new(format!("✦ {}", g.lumens)).font(serif(24.0)).color(GOLD));
            let total = g.total_books();
            let done = g.total_shelved();
            ui.label(RichText::new(format!("{done} / {total} books home")).font(serif(14.0)).color(INK));
            for (w, name) in ["West Wing", "East Wing"].iter().enumerate() {
                let frac = g.wing_shelved[w] as f32 / g.wing_total[w].max(1) as f32;
                let label = if g.wing_unlocked(w) { format!("{name}  {:.0}%", frac * 100.0) } else { format!("{name}  (veiled)") };
                let (rect, _) = ui.allocate_exact_size(Vec2::new(220.0, 18.0), egui::Sense::hover());
                let p = ui.painter();
                p.rect_filled(rect, 3.0, Color32::from_rgba_unmultiplied(60, 45, 30, 160));
                let mut fill = rect;
                fill.set_width(rect.width() * frac.clamp(0.0, 1.0));
                p.rect_filled(fill, 3.0, Color32::from_rgb(150, 105, 45));
                p.text(rect.left_center() + Vec2::new(6.0, 0.0), Align2::LEFT_CENTER, label, serif(12.0), INK);
            }
            if !g.east_unlocked {
                let need = (g.wing_total[0] as f32 * 0.6).ceil() as u32;
                let left = need.saturating_sub(g.wing_shelved[0]);
                ui.label(RichText::new(format!("{left} more to lift the veil")).font(serif(12.0)).color(DIM));
            }
        });
    });

    // ---- Bottom-left: active book card and armful.
    egui::Area::new("arms".into()).anchor(Align2::LEFT_BOTTOM, [16.0, -16.0]).interactable(false).show(ctx, |ui| {
        panel_frame().show(ui, |ui| {
            ui.set_max_width(380.0);
            ui.label(RichText::new(format!("Arms  {}/{}", g.held.len(), g.capacity())).font(serif(13.0)).color(DIM));
            if let Some(b) = g.active_book() {
                let info = &g.lib.books[b as usize];
                let s = &g.lib.series[info.series as usize];
                let f = &g.lib.faces[info.face as usize];
                ui.label(RichText::new(&s.title).font(serif(19.0)).color(INK));
                let vol = if s.volumes > 1 { format!("Volume {} of {}", info.volume, s.volumes) } else { "Single volume".into() };
                ui.label(RichText::new(vol).font(serif(14.0)).color(INK));
                let cat = &CATEGORIES[s.category];
                let wing = if wing_of_category(s.category) == 0 { "West" } else { "East" };
                let place = if g.level(Upg::SortSense) > 0 {
                    format!("{} · {} Wing · case {}, shelf {} of {}", cat.name, wing, f.case_idx + 1, info.row as usize + 1, ROWS)
                } else {
                    format!("{} · {} Wing", cat.name, wing)
                };
                ui.label(RichText::new(place).font(serif(14.0)).color(bright(catalog::category_color(s.category))));
                // Strip of held spines.
                let (rect, _) = ui.allocate_exact_size(Vec2::new(360.0, 26.0), egui::Sense::hover());
                let p = ui.painter();
                let n = g.held.len();
                let w = (360.0 / n.max(1) as f32).min(14.0);
                for (i, &hb) in g.held.iter().enumerate() {
                    let hs = &g.lib.series[g.lib.books[hb as usize].series as usize];
                    let x = rect.min.x + i as f32 * w;
                    let r = Rect::from_min_size(Pos2::new(x, rect.min.y + 4.0), Vec2::new((w - 1.0).max(1.0), 20.0));
                    p.rect_filled(r, 1.0, bright(hs.color).gamma_multiply(0.8));
                    if i == g.active {
                        p.rect_stroke(r.expand(1.5), 1.0, Stroke::new(1.5, GOLD), egui::StrokeKind::Outside);
                    }
                }
            } else {
                ui.label(RichText::new("Empty-handed. Click a heap to gather books.").font(serif(15.0)).color(INK));
            }
        });
    });

    // ---- Bottom-right: controls reminder.
    egui::Area::new("keys".into()).anchor(Align2::RIGHT_BOTTOM, [-16.0, -16.0]).interactable(false).show(ctx, |ui| {
        let m = if music_on { "on" } else { "off" };
        ui.label(
            RichText::new(format!("Click gather/file · Scroll cycle · Tab Grimoire · M music ({m}) · Esc menu"))
                .font(serif(12.0))
                .color(DIM.gamma_multiply(0.8)),
        );
    });

    // ---- Toasts and hints.
    egui::Area::new("toasts".into()).anchor(Align2::CENTER_TOP, [0.0, 24.0]).interactable(false).show(ctx, |ui| {
        ui.set_width(screen.x * 0.8);
        ui.vertical_centered(|ui| {
            for t in g.toasts.iter().rev().take(5) {
                let life = if t.big { 6.0 } else { 3.5 };
                let a = (t.age / 0.3).min(1.0) * ((life - t.age) / 0.8).clamp(0.0, 1.0);
                let size = if t.big { 26.0 } else { 17.0 };
                ui.label(RichText::new(&t.text).font(serif(size)).color(bright(t.color).gamma_multiply(a)));
            }
        });
    });
    if let Some((h, t)) = &g.hint {
        let a = t.min(1.0);
        painter.text(c + Vec2::new(0.0, 60.0), Align2::CENTER_TOP, h, serif(15.0), DIM.gamma_multiply(a));
    }

    if hud.show_stats {
        egui::Area::new("perf".into()).anchor(Align2::RIGHT_TOP, [-16.0, 16.0]).interactable(false).show(ctx, |ui| {
            ui.label(
                RichText::new(format!("{:.0} fps · {:.2} ms · {} instances", hud.fps, hud.frame_ms, g.dyn_insts.len() + g.trans_insts.len()))
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
                ui.label(RichText::new(format!("You have ✦ {} lumens.", g.lumens)).font(serif(16.0)).color(INK));
                ui.add_space(6.0);
                egui::ScrollArea::vertical().max_height(screen.y * 0.7).show(ui, |ui| {
                    for u in UPGRADES {
                        let def = upg_def(u);
                        let lvl = g.level(u);
                        let max = def.costs.len() as u32;
                        ui.horizontal(|ui| {
                            ui.allocate_ui_with_layout(Vec2::new(420.0, 0.0), egui::Layout::top_down(egui::Align::Min), |ui| {
                                ui.set_width(420.0);
                                let title = if max > 1 { format!("{}  ·  {}/{}", def.name, lvl, max) } else { def.name.to_string() };
                                ui.label(RichText::new(title).font(serif(17.0)).color(INK));
                                ui.label(RichText::new(def.blurb).font(serif(13.0)).color(DIM));
                            });
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| match g.next_cost(u) {
                                Some(cost) => {
                                    let afford = g.lumens >= cost;
                                    let btn = egui::Button::new(RichText::new(format!("✦ {cost}")).font(serif(16.0)).color(if afford { GOLD } else { DIM }))
                                        .min_size(Vec2::new(96.0, 32.0));
                                    if ui.add_enabled(afford, btn).clicked() {
                                        actions.push(HudAction::Buy(u));
                                    }
                                }
                                None => {
                                    ui.label(RichText::new("mastered").font(serif(14.0)).color(GOLD));
                                }
                            });
                        });
                        ui.add_space(4.0);
                        ui.separator();
                    }
                });
                ui.add_space(8.0);
                ui.label(RichText::new("Press Tab to return to the stacks.").font(serif(13.0)).color(DIM));
            });
        if !open {
            actions.push(HudAction::CloseMenu);
        }
    }

    // ---- Intro / pause.
    if hud.show_intro {
        egui::Window::new(RichText::new("Hushfold Library").font(serif(30.0)).color(GOLD))
            .collapsible(false)
            .resizable(false)
            .title_bar(true)
            .anchor(Align2::CENTER_CENTER, [0.0, 0.0])
            .frame(panel_frame().fill(Color32::from_rgba_unmultiplied(24, 16, 12, 240)))
            .show(ctx, |ui| {
                ui.set_width(520.0);
                let body = "The night librarian has gone missing, and the library has come undone. \
                    Thousands of books lie in heaps across the floor.\n\n\
                    Gather an armful from any heap, then find each book's home: its category's \
                    bookcases (look for the signs), its series (the glowing outlines in its colour), \
                    and its place in the series. Click anywhere on a series' run of outlines to file a volume.\n\n\
                    Every book you file earns a lumen ✦. Spend them in the Grimoire to make the work lighter.";
                ui.label(RichText::new(body).font(serif(15.0)).color(INK));
                ui.add_space(8.0);
                ui.label(RichText::new("WASD walk · Shift hurry · Mouse look · Click gather/file\nScroll or Q/E cycle the active book · Tab Grimoire · M music · F3 stats · Esc pause").font(serif(13.0)).color(DIM));
                ui.add_space(10.0);
                ui.vertical_centered(|ui| {
                    let label = if g.total_shelved() > 0 || g.lumens > 0 { "Return to the stacks" } else { "Begin" };
                    if ui.add(egui::Button::new(RichText::new(label).font(serif(20.0)).color(GOLD)).min_size(Vec2::new(220.0, 40.0))).clicked() {
                        actions.push(HudAction::Begin);
                    }
                });
            });
    }
    actions
}
