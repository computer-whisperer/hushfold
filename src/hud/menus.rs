//! The title screen (with its saves), the pause screen, settings, a new game's
//! opening words, and the card shown while a library opens.

use super::*;
use crate::audio::{Outputs, Playing};
use crate::game::SaveSummary;
use std::time::SystemTime;

/// Darken the stacks behind a menu: `left` at the left edge to `right` at the right.
fn shade(ctx: &egui::Context, left: f32, right: f32) {
    let r = ctx.content_rect();
    let a = |x: f32| Color32::from_black_alpha((x * 255.0) as u8);
    let mut mesh = egui::Mesh::default();
    mesh.colored_vertex(r.left_top(), a(left));
    mesh.colored_vertex(r.right_top(), a(right));
    mesh.colored_vertex(r.right_bottom(), a(right));
    mesh.colored_vertex(r.left_bottom(), a(left));
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(0, 2, 3);
    ctx.layer_painter(egui::LayerId::background()).add(mesh);
}

/// One line of a menu, in ink; gold, with a star before it, while pointed at.
fn item(ui: &mut egui::Ui, text: &str, size: f32) -> egui::Response {
    let font = serif(size);
    let galley = ui.painter().layout_no_wrap(text.into(), font.clone(), INK);
    let pad = Vec2::new(size * 1.1, size * 0.2);
    let (rect, resp) = ui.allocate_exact_size(galley.size() + pad * 2.0, egui::Sense::click());
    ui.ctx().data_mut(|d| d.insert_temp(resp.id, ShowsFocus));
    let lit = resp.hovered() || resp.has_focus();
    let p = ui.painter();
    let at = rect.left_center() + Vec2::new(pad.x, 0.0);
    p.text(at, Align2::LEFT_CENTER, text, font, if lit { GOLD } else { INK });
    if lit {
        let star = rect.left_center() + Vec2::new(pad.x * 0.45, 0.0);
        p.text(star, Align2::CENTER_CENTER, "✦", serif(size * 0.5), GOLD);
    }
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// A line under a menu item, lined up with its text.
fn under(ui: &mut egui::Ui, text: impl Into<String>, item_size: f32) {
    ui.horizontal(|ui| {
        ui.add_space(item_size * 1.1);
        ui.label(RichText::new(text).font(serif(14.0)).color(DIM));
    });
}

fn button(ui: &mut egui::Ui, text: &str, color: Color32) -> egui::Response {
    ui.add(egui::Button::new(RichText::new(text).font(serif(15.0)).color(color)).min_size(Vec2::new(84.0, 30.0)))
}

/// Time in the stacks: "12 min", "3 h 5 min".
fn hours(secs: f64) -> String {
    let mins = (secs / 60.0) as u64;
    if mins >= 60 {
        format!("{} h {} min", mins / 60, mins % 60)
    } else {
        format!("{mins} min")
    }
}

/// A count with its thousands set off: 117,715.
fn count(n: u32) -> String {
    let digits = n.to_string();
    let mut out = String::new();
    for (i, ch) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(ch);
    }
    out
}

/// How long ago `when` was, in words.
fn ago(when: SystemTime) -> String {
    let secs = SystemTime::now().duration_since(when).map_or(0, |d| d.as_secs());
    let (n, unit) = match secs {
        0..60 => return "just now".into(),
        60..3600 => (secs / 60, "minute"),
        3600..86400 => (secs / 3600, "hour"),
        86400..172800 => return "yesterday".into(),
        _ => (secs / 86400, "day"),
    };
    format!("{n} {unit}{} ago", if n == 1 { "" } else { "s" })
}

fn progress(s: &SaveSummary) -> String {
    match s.books {
        0 => format!("{} books home", count(s.shelved)),
        n if n == s.shelved => "every book home".into(),
        n => format!("{} of {} books home", count(s.shelved), count(n)),
    }
}

pub(super) fn title(ctx: &egui::Context, hud: &mut Hud, page: TitlePage, actions: &mut Vec<HudAction>) {
    shade(ctx, 0.85, 0.15);
    let screen = ctx.content_rect();
    egui::Area::new("title".into())
        .anchor(Align2::LEFT_CENTER, [screen.width() * 0.08, 0.0])
        .show(ctx, |ui| {
            ui.label(RichText::new("Hushfold Library").font(serif(64.0)).color(GOLD));
            ui.label(
                RichText::new("A cozy game of putting a magical library back in order")
                    .font(serif(17.0))
                    .italics()
                    .color(DIM),
            );
            ui.add_space(40.0);
            match page {
                TitlePage::Main => title_menu(ui, hud, actions),
                TitlePage::NewGame => new_game(ui, actions),
                TitlePage::Load => load(ui, hud, actions),
            }
        });
    egui::Area::new("version".into())
        .anchor(Align2::RIGHT_BOTTOM, [-16.0, -12.0])
        .interactable(false)
        .show(ctx, |ui| {
            let v = concat!("version ", env!("CARGO_PKG_VERSION"));
            ui.label(RichText::new(v).font(serif(12.0)).color(DIM.gamma_multiply(0.7)));
        });
}

fn title_menu(ui: &mut egui::Ui, hud: &Hud, actions: &mut Vec<HudAction>) {
    const SIZE: f32 = 30.0;
    if let Some(latest) = crate::saves::latest(&hud.saves) {
        if item(ui, "Continue", SIZE).clicked() {
            actions.push(HudAction::Continue);
        }
        let s = &latest.summary;
        under(ui, format!("{} · {}", s.layout.name(), progress(s)), SIZE);
        ui.add_space(6.0);
    }
    if item(ui, "New Game", SIZE).clicked() {
        actions.push(HudAction::Show(Screen::Title(TitlePage::NewGame)));
    }
    if !hud.saves.is_empty() && item(ui, "Load", SIZE).clicked() {
        actions.push(HudAction::Show(Screen::Title(TitlePage::Load)));
    }
    if item(ui, "Settings", SIZE).clicked() {
        actions.push(HudAction::Show(Screen::Settings { from_title: true }));
    }
    if item(ui, "Quit", SIZE).clicked() {
        actions.push(HudAction::Quit);
    }
}

fn new_game(ui: &mut egui::Ui, actions: &mut Vec<HudAction>) {
    const SIZE: f32 = 28.0;
    ui.label(RichText::new("Which library?").font(serif(20.0)).color(INK));
    ui.add_space(10.0);
    for layout in Layout::ALL {
        if item(ui, layout.name(), SIZE).clicked() {
            actions.push(HudAction::NewGame(layout));
        }
        under(ui, layout.blurb(), SIZE);
        ui.add_space(8.0);
    }
    ui.add_space(16.0);
    if item(ui, "Back", 22.0).clicked() {
        actions.push(HudAction::Show(Screen::Title(TitlePage::Main)));
    }
}

fn load(ui: &mut egui::Ui, hud: &mut Hud, actions: &mut Vec<HudAction>) {
    let tall = ui.ctx().content_rect().height() * 0.45;
    let (mut ask, mut keep) = (None, false);
    egui::ScrollArea::vertical().max_height(tall).show(ui, |ui| {
        ui.set_width(560.0);
        for save in &hud.saves {
            let s = &save.summary;
            panel_frame().show(ui, |ui| {
                ui.set_width(540.0);
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label(RichText::new(s.layout.name()).font(serif(20.0)).color(GOLD));
                        let line = format!("{} · {} in the stacks", progress(s), hours(s.play_time));
                        ui.label(RichText::new(line).font(serif(14.0)).color(INK));
                        let when = format!("Last played {}", ago(save.when));
                        ui.label(RichText::new(when).font(serif(13.0)).color(DIM));
                        if !s.fits {
                            let note = "Made by another version of the game: it opens as a fresh library.";
                            ui.label(RichText::new(note).font(serif(13.0)).italics().color(DIM));
                        }
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if hud.confirm_delete.as_ref() == Some(&save.path) {
                            return;
                        }
                        // Open made first, so a pad's focus comes to it before Delete.
                        ui.horizontal(|ui| {
                            if button(ui, "Open", GOLD).clicked() {
                                actions.push(HudAction::Open(s.layout, save.path.clone()));
                            }
                            if button(ui, "Delete", DIM).clicked() {
                                ask = Some(save.path.clone());
                            }
                        });
                    });
                });
                if hud.confirm_delete.as_ref() == Some(&save.path) {
                    ui.add_space(4.0);
                    ui.label(
                        RichText::new("Delete this game? It can't be undone.")
                            .font(serif(14.0))
                            .color(INK),
                    );
                    ui.horizontal(|ui| {
                        let k = button(ui, "Keep", GOLD);
                        // A pad's focus, lost with the Delete pressed to ask, comes to Keep.
                        if hud.prompts.pad() && ui.memory(|m| m.focused().is_none()) {
                            k.request_focus();
                        }
                        if k.clicked() {
                            keep = true;
                        }
                        if button(ui, "Delete", DIM).clicked() {
                            actions.push(HudAction::Delete(save.path.clone()));
                        }
                    });
                }
            });
            ui.add_space(6.0);
        }
    });
    if ask.is_some() {
        hud.confirm_delete = ask;
    } else if keep {
        hud.confirm_delete = None;
    }
    ui.add_space(16.0);
    if item(ui, "Back", 22.0).clicked() {
        actions.push(HudAction::Show(Screen::Title(TitlePage::Main)));
    }
}

pub(super) fn pause(ctx: &egui::Context, g: &Game, actions: &mut Vec<HudAction>) {
    shade(ctx, 0.6, 0.6);
    egui::Area::new("pause".into())
        .anchor(Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.label(RichText::new(g.layout.name()).font(serif(36.0)).color(GOLD));
                let done = g.total_shelved() == g.total_books();
                let books = if done {
                    "Every book is home".to_string()
                } else {
                    format!(
                        "{} of {} books are home",
                        count(g.total_shelved()),
                        count(g.total_books())
                    )
                };
                let summary = format!("{books} · {} in the stacks", hours(g.play_time));
                ui.label(RichText::new(summary).font(serif(16.0)).color(INK));
                ui.add_space(28.0);
                const SIZE: f32 = 26.0;
                if item(ui, "Resume", SIZE).clicked() {
                    actions.push(HudAction::Show(Screen::Play));
                }
                if item(ui, "Settings", SIZE).clicked() {
                    actions.push(HudAction::Show(Screen::Settings { from_title: false }));
                }
                if item(ui, "Save and return to title", SIZE).clicked() {
                    actions.push(HudAction::ToTitle);
                }
                if item(ui, "Save and quit", SIZE).clicked() {
                    actions.push(HudAction::Quit);
                }
                ui.add_space(12.0);
                ui.label(
                    RichText::new("Your place is kept; progress saves as you go.")
                        .font(serif(13.0))
                        .color(DIM),
                );
            });
        });
}

pub(super) fn intro(ctx: &egui::Context, p: Prompts, actions: &mut Vec<HudAction>) {
    shade(ctx, 0.6, 0.6);
    egui::Area::new("intro".into())
        .anchor(Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            panel_frame()
                .fill(Color32::from_rgba_unmultiplied(24, 16, 12, 240))
                .show(ui, |ui| {
                    ui.set_width(540.0_f32.min(ctx.content_rect().width() - 48.0));
                    let body = "The night librarian has gone missing, and the library has come undone. \
                    Thousands of books lie in piles across the floor.\n\n\
                    Tap a book to take it, or hold on a pile to scoop an armful. Then find each book's home: \
                    its category's bookcases (look for the signs), the shelf whose letters take its title, \
                    and its series' run of outlines in its colour. Tap anywhere on the run to file a volume.\n\n\
                    Every book you file earns a lumen ✦. Spend them in the Grimoire to make the work lighter.";
                    // Scrolled on a short screen, so Begin stays in view.
                    let tall = (ctx.content_rect().height() - 140.0).max(160.0);
                    egui::ScrollArea::vertical().max_height(tall).show(ui, |ui| {
                        ui.label(RichText::new(body).font(serif(15.0)).color(INK));
                        ui.add_space(8.0);
                        controls(ui, "intro keys", p);
                    });
                    ui.add_space(10.0);
                    ui.vertical_centered(|ui| {
                        if item(ui, "Begin", 26.0).clicked() {
                            actions.push(HudAction::Show(Screen::Play));
                        }
                    });
                });
        });
}

/// Every control and what it does.
fn controls(ui: &mut egui::Ui, id: &str, p: Prompts) {
    egui::Grid::new(id).num_columns(2).spacing([18.0, 3.0]).show(ui, |ui| {
        for (key, what) in p.list() {
            ui.label(RichText::new(key).font(serif(13.0)).color(INK));
            ui.label(RichText::new(what).font(serif(13.0)).color(DIM));
            ui.end_row();
        }
    });
}

/// The settings, by tab; each applies as it changes, and they are written down on
/// the way out.
pub(super) fn settings(ctx: &egui::Context, hud: &mut Hud, actions: &mut Vec<HudAction>) {
    shade(ctx, 0.7, 0.7);
    egui::Area::new("settings".into())
        .anchor(Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            panel_frame()
                .fill(Color32::from_rgba_unmultiplied(24, 16, 12, 240))
                .show(ui, |ui| {
                    ui.set_width(600.0_f32.min(ctx.content_rect().width() - 48.0));
                    for style in [egui::TextStyle::Body, egui::TextStyle::Button] {
                        ui.style_mut().text_styles.insert(style, serif(15.0));
                    }
                    ui.spacing_mut().slider_width = 240.0;
                    ui.label(RichText::new("Settings").font(serif(28.0)).color(GOLD));
                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        for (tab, name) in [
                            (SettingsTab::Video, "Video"),
                            (SettingsTab::Audio, "Audio"),
                            (SettingsTab::Controls, "Controls"),
                        ] {
                            let on = hud.settings_tab == tab;
                            let text = RichText::new(name).font(serif(19.0)).color(if on { GOLD } else { DIM });
                            let r = ui.add(egui::Button::new(text).frame(false));
                            if on {
                                let line = [r.rect.left_bottom(), r.rect.right_bottom()];
                                ui.painter().line_segment(line, Stroke::new(1.5_f32, GOLD));
                            }
                            if r.clicked() {
                                hud.settings_tab = tab;
                            }
                            ui.add_space(10.0);
                        }
                    });
                    ui.separator();
                    ui.add_space(6.0);
                    let tab = hud.settings_tab;
                    // The keys that change a setting too, named while they are in hand.
                    let keys = !hud.prompts.pad();
                    let st = &mut hud.settings;
                    let before = st.clone();
                    let tall = (ctx.content_rect().height() * 0.6).max(200.0);
                    egui::ScrollArea::vertical().max_height(tall).show(ui, |ui| {
                        ui.set_min_height(280.0);
                        let grid = egui::Grid::new("settings").num_columns(2).spacing([28.0, 12.0]);
                        grid.show(ui, |ui| match tab {
                            SettingsTab::Video => video(ui, st, keys),
                            SettingsTab::Audio => audio(ui, st, &hud.outputs, keys),
                            SettingsTab::Controls => look(ui, st),
                        });
                        if tab == SettingsTab::Controls {
                            ui.add_space(10.0);
                            ui.separator();
                            ui.add_space(4.0);
                            controls(ui, "keys", hud.prompts);
                        }
                    });
                    if *st != before {
                        actions.push(HudAction::SettingsChanged);
                    }
                    ui.add_space(10.0);
                    ui.vertical_centered(|ui| {
                        if item(ui, "Back", 22.0).clicked() {
                            actions.push(HudAction::SettingsDone);
                        }
                    });
                });
        });
}

/// A setting's name, and the key that changes it too.
fn row(ui: &mut egui::Ui, name: &str, key: Option<&str>) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(name).font(serif(15.0)).color(INK));
        if let Some(key) = key {
            ui.label(RichText::new(key).font(serif(12.0)).color(DIM));
        }
    });
}

/// A slider, its value beside it as words rather than in a box to type into: one stop
/// fewer for a pad's focus, and nothing a pad can't fill.
fn slider(
    ui: &mut egui::Ui,
    v: &mut f32,
    range: std::ops::RangeInclusive<f32>,
    shape: impl FnOnce(egui::Slider<'_>) -> egui::Slider<'_>,
    text: impl Fn(f32) -> String,
) {
    ui.horizontal(|ui| {
        ui.add(shape(egui::Slider::new(&mut *v, range).show_value(false)));
        ui.label(text(*v));
    });
}

fn pct(ui: &mut egui::Ui, v: &mut f32, range: std::ops::RangeInclusive<f32>) {
    slider(ui, v, range, |s| s, |x| format!("{:.0}%", x * 100.0));
}

fn video(ui: &mut egui::Ui, st: &mut Settings, keys: bool) {
    row(ui, "Fullscreen", keys.then_some("F11"));
    ui.checkbox(&mut st.fullscreen, "");
    ui.end_row();
    row(ui, "Field of view", None);
    slider(ui, &mut st.fov, 55.0..=90.0, |s| s.integer(), |x| format!("{x:.0}°"));
    ui.end_row();
    row(ui, "Head bob while walking", None);
    ui.checkbox(&mut st.head_bob, "");
    ui.end_row();
    row(ui, "Brightness", None);
    pct(ui, &mut st.brightness, 0.6..=1.6);
    ui.end_row();
    row(ui, "Interface size", None);
    let size = |x: f32| format!("{:.0}%", x * 100.0);
    egui::ComboBox::from_id_salt("ui scale")
        .selected_text(size(st.ui_scale))
        .show_ui(ui, |ui| {
            for x in crate::settings::UI_SCALES {
                ui.selectable_value(&mut st.ui_scale, x, size(x));
            }
        });
    ui.end_row();
    row(ui, "Show frame rate", keys.then_some("F3"));
    ui.checkbox(&mut st.show_fps, "");
    ui.end_row();
}

fn audio(ui: &mut egui::Ui, st: &mut Settings, outputs: &Outputs, keys: bool) {
    row(ui, "Output", None);
    ui.vertical(|ui| {
        let system = match &outputs.system {
            Some(name) => format!("System default ({name})"),
            None => "System default".into(),
        };
        let shown = st.output.as_ref().map_or(system.clone(), |d| d.name.clone());
        egui::ComboBox::from_id_salt("output")
            .width(ui.spacing().slider_width)
            .selected_text(shown)
            .truncate()
            .show_ui(ui, |ui| {
                if ui.selectable_label(st.output.is_none(), system).clicked() {
                    st.output = None;
                }
                for d in &outputs.devices {
                    let on = st.output.as_ref().is_some_and(|o| o.id == d.id);
                    if ui.selectable_label(on, &d.name).clicked() {
                        st.output = Some(d.clone());
                    }
                }
            });
        let note = match (outputs.playing, &st.output) {
            (Playing::Nowhere, _) => Some("No output would open; trying again."),
            (Playing::System, Some(_)) => Some("Not connected: playing on the system default."),
            _ => None,
        };
        if let Some(note) = note {
            ui.label(RichText::new(note).font(serif(12.0)).color(DIM));
        }
    });
    ui.end_row();
    row(ui, "Master volume", None);
    pct(ui, &mut st.master_volume, 0.0..=1.0);
    ui.end_row();
    row(ui, "Music", keys.then_some("M"));
    ui.checkbox(&mut st.music, "");
    ui.end_row();
    row(ui, "Music volume", None);
    ui.add_enabled_ui(st.music, |ui| pct(ui, &mut st.music_volume, 0.0..=1.0));
    ui.end_row();
    row(ui, "Effects and ambience", None);
    pct(ui, &mut st.effects_volume, 0.0..=1.0);
    ui.end_row();
}

fn look(ui: &mut egui::Ui, st: &mut Settings) {
    row(ui, "Look speed", None);
    slider(
        ui,
        &mut st.look_speed,
        0.2..=4.0,
        |s| s.logarithmic(true),
        |x| format!("{x:.2}"),
    );
    ui.end_row();
    row(ui, "Invert vertical look", None);
    ui.checkbox(&mut st.invert_look, "");
    ui.end_row();
}

/// Painted, not laid out in an area: the card is up for two frames, and a new area
/// draws nothing in its first.
pub(super) fn loading(ctx: &egui::Context, what: &str) {
    shade(ctx, 0.8, 0.8);
    let p = ctx.layer_painter(egui::LayerId::new(egui::Order::Foreground, "loading".into()));
    p.text(
        ctx.content_rect().center(),
        Align2::CENTER_CENTER,
        what,
        serif(26.0),
        INK,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_and_times_read_as_words() {
        assert_eq!(count(7), "7");
        assert_eq!(count(1000), "1,000");
        assert_eq!(count(117_715), "117,715");
        assert_eq!(count(1_234_567), "1,234,567");
        assert_eq!(hours(59.0 * 60.0), "59 min");
        assert_eq!(hours(35472.3), "9 h 51 min");
        let back = |secs| SystemTime::now() - std::time::Duration::from_secs(secs);
        assert_eq!(ago(back(5)), "just now");
        assert_eq!(ago(back(61)), "1 minute ago");
        assert_eq!(ago(back(3 * 3600 + 5)), "3 hours ago");
        assert_eq!(ago(back(30 * 3600)), "yesterday");
        assert_eq!(ago(back(5 * 86400)), "5 days ago");
    }
}
