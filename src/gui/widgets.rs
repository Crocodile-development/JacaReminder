// Small inputs used by the editors.

use chrono::{Datelike, NaiveDate, NaiveTime, Timelike};
use eframe::egui::{self, Color32, DragValue};

use super::theme;
use crate::shared::i18n::t;

/// Hour and minute boxes. Returns true when the value changed.
pub fn time_edit(ui: &mut egui::Ui, time: &mut NaiveTime) -> bool {
    let mut hour = time.hour();
    let mut minute = time.minute();
    let mut changed = ui.add(DragValue::new(&mut hour).range(0..=23).custom_formatter(two_digits)).changed();
    ui.label(":");
    changed |= ui.add(DragValue::new(&mut minute).range(0..=59).custom_formatter(two_digits)).changed();
    if changed {
        *time = NaiveTime::from_hms_opt(hour, minute, 0).unwrap_or(*time);
    }
    changed
}

/// Day, month and year boxes. A day that does not exist in the month is moved to its last day.
pub fn date_edit(ui: &mut egui::Ui, date: &mut NaiveDate) -> bool {
    let mut day = date.day();
    let mut month = date.month();
    let mut year = date.year();
    let mut changed = ui.add(DragValue::new(&mut day).range(1..=31).custom_formatter(two_digits)).changed();
    ui.label("/");
    changed |= ui.add(DragValue::new(&mut month).range(1..=12).custom_formatter(two_digits)).changed();
    ui.label("/");
    changed |= ui.add(DragValue::new(&mut year).range(2000..=2200)).changed();
    if changed {
        let mut candidate = day;
        while candidate > 28 && NaiveDate::from_ymd_opt(year, month, candidate).is_none() {
            candidate -= 1;
        }
        *date = NaiveDate::from_ymd_opt(year, month, candidate).unwrap_or(*date);
    }
    changed
}

fn two_digits(value: f64, _range: std::ops::RangeInclusive<usize>) -> String {
    format!("{:02}", value as i64)
}

fn semibold_font() -> egui::FontId {
    egui::FontId::new(13.5, egui::FontFamily::Name("semibold".into()))
}

#[derive(PartialEq, Clone, Copy)]
enum ButtonStyle {
    Normal,
    Primary,
    Danger,
}

/// A button whose colors fade when the mouse enters and leaves it.
pub fn button(ui: &mut egui::Ui, label: &str) -> egui::Response {
    draw_button(ui, label, ButtonStyle::Normal, true, egui::Vec2::ZERO)
}

/// A button that can be disabled.
pub fn button_enabled(ui: &mut egui::Ui, label: &str, enabled: bool) -> egui::Response {
    draw_button(ui, label, ButtonStyle::Normal, enabled, egui::Vec2::ZERO)
}

/// The accent colored button for the main action.
pub fn primary_button(ui: &mut egui::Ui, label: &str, enabled: bool) -> egui::Response {
    draw_button(ui, label, ButtonStyle::Primary, enabled, egui::vec2(0.0, 36.0))
}

/// The pictures an icon button can show.
#[derive(Clone, Copy, PartialEq)]
pub enum Icon {
    Play,
    Stop,
}

/// A square button with a picture instead of text; `tooltip` says what it does.
pub fn icon_button(ui: &mut egui::Ui, icon: Icon, tooltip: &str) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(34.0, 30.0), egui::Sense::click());
    let response = response
        .on_hover_cursor(egui::CursorIcon::PointingHand)
        .on_hover_text(t(tooltip));
    let hover = ui.ctx().animate_bool_with_time(response.id, response.hovered(), 0.15);
    let painter = ui.painter();
    painter.rect_filled(rect, 8.0, theme::CARD.lerp_to_gamma(theme::HOVER, hover));
    painter.rect_stroke(
        rect,
        8.0,
        egui::Stroke::new(1.0, theme::BORDER.lerp_to_gamma(Color32::from_rgb(48, 140, 134), hover)),
        egui::StrokeKind::Inside,
    );
    let color = theme::TEXT.lerp_to_gamma(theme::ACCENT, hover);
    let c = rect.center();
    match icon {
        Icon::Play => {
            // A triangle pointing right, a little to the right so it looks centered.
            let points = vec![
                egui::pos2(c.x - 4.0, c.y - 6.0),
                egui::pos2(c.x - 4.0, c.y + 6.0),
                egui::pos2(c.x + 6.5, c.y),
            ];
            painter.add(egui::Shape::convex_polygon(points, color, egui::Stroke::NONE));
        }
        Icon::Stop => {
            painter.rect_filled(egui::Rect::from_center_size(c, egui::Vec2::splat(11.0)), 2.5, color);
        }
    }
    response
}

/// The red button for an action that cannot be undone.
pub fn danger_button(ui: &mut egui::Ui, label: &str) -> egui::Response {
    draw_button(ui, label, ButtonStyle::Danger, true, egui::vec2(0.0, 30.0))
}

fn draw_button(ui: &mut egui::Ui, label: &str, style: ButtonStyle, enabled: bool, min: egui::Vec2) -> egui::Response {
    let galley = ui.painter().layout_no_wrap(t(label).to_owned(), semibold_font(), Color32::WHITE);
    let size = (galley.size() + egui::vec2(28.0, 14.0)).max(min).max(egui::vec2(0.0, 30.0));
    let sense = if enabled { egui::Sense::click() } else { egui::Sense::hover() };
    let (rect, response) = ui.allocate_exact_size(size, sense);
    let response = if enabled { response.on_hover_cursor(egui::CursorIcon::PointingHand) } else { response };
    let hover = ui.ctx().animate_bool_with_time(response.id, enabled && response.hovered(), 0.15);
    let (fill, stroke, text) = match style {
        ButtonStyle::Primary => (
            theme::ACCENT.lerp_to_gamma(theme::ACCENT_TEXT, hover * 0.4),
            theme::ACCENT,
            theme::ON_ACCENT,
        ),
        ButtonStyle::Danger => (
            theme::DANGER.lerp_to_gamma(Color32::WHITE, hover * 0.2),
            theme::DANGER,
            Color32::WHITE,
        ),
        ButtonStyle::Normal => (
            theme::CARD.lerp_to_gamma(theme::HOVER, hover),
            theme::BORDER.lerp_to_gamma(Color32::from_rgb(48, 140, 134), hover),
            theme::TEXT,
        ),
    };
    let fade = if enabled { 1.0 } else { 0.4 };
    let painter = ui.painter();
    painter.rect_filled(rect, 8.0, fill.gamma_multiply(fade));
    painter.rect_stroke(rect, 8.0, egui::Stroke::new(1.0, stroke.gamma_multiply(fade)), egui::StrokeKind::Inside);
    painter.galley(rect.center() - galley.size() / 2.0, galley, text.gamma_multiply(fade));
    response
}

/// A row of toggle buttons, one per label. `selected` holds the chosen values; `first_value` is
/// the value of the first label (0 for weekdays, 1 for months and days).
pub fn toggle_row(ui: &mut egui::Ui, labels: &[&str], first_value: u8, selected: &mut Vec<u8>) -> bool {
    let mut changed = false;
    ui.horizontal_wrapped(|ui| {
        for (index, label) in labels.iter().enumerate() {
            let value = first_value + index as u8;
            let on = selected.contains(&value);
            let galley = ui.painter().layout_no_wrap(t(label).to_owned(), semibold_font(), Color32::WHITE);
            let size = (galley.size() + egui::vec2(16.0, 10.0)).max(egui::vec2(34.0, 26.0));
            let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());
            let response = response.on_hover_cursor(egui::CursorIcon::PointingHand);
            // Named by the row and the value, so adding widgets above does not mix them up.
            let chip = egui::Id::unique(("toggle_chip", first_value, labels.len(), index));
            let hover = ui.ctx().animate_bool_with_time(chip.with("hover"), response.hovered(), 0.15);
            let active = ui.ctx().animate_bool_with_time(chip.with("on"), on, 0.2);
            let fill = theme::CARD.lerp_to_gamma(theme::HOVER, hover).lerp_to_gamma(theme::ACCENT, active);
            let text = theme::TEXT.lerp_to_gamma(theme::ON_ACCENT, active);
            ui.painter().rect_filled(rect, 8.0, fill);
            ui.painter().galley(rect.center() - galley.size() / 2.0, galley, text);
            if response.clicked() {
                if on {
                    selected.retain(|item| *item != value);
                } else {
                    selected.push(value);
                    selected.sort();
                }
                changed = true;
            }
        }
    });
    changed
}

/// Four icon buttons, one per corner (top left, top right, bottom right, bottom left), like the
/// independent corners of Figma. A corner that is on is drawn rounded, one that is off is drawn
/// square. Returns true when one was clicked.
pub fn corner_toggles(ui: &mut egui::Ui, corners: &mut [bool; 4]) -> bool {
    const NAMES: [&str; 4] = ["Top left", "Top right", "Bottom right", "Bottom left"];
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 6.0;
        for (index, on) in corners.iter_mut().enumerate() {
            let (rect, response) = ui.allocate_exact_size(egui::Vec2::splat(34.0), egui::Sense::click());
            let response = response
                .on_hover_cursor(egui::CursorIcon::PointingHand)
                .on_hover_text(t(NAMES[index]));
            if response.clicked() {
                *on = !*on;
                changed = true;
            }
            let id = egui::Id::unique(("corner_toggle", index));
            let hover = ui.ctx().animate_bool_with_time(id.with("hover"), response.hovered(), 0.15);
            let active = ui.ctx().animate_bool_with_time(id.with("on"), *on, 0.2);
            let fill = theme::CARD.lerp_to_gamma(theme::HOVER, hover).lerp_to_gamma(theme::ACCENT_TINT, active);
            let border = theme::BORDER.lerp_to_gamma(theme::ACCENT_LINE, active);
            let line = theme::MUTED.lerp_to_gamma(theme::TEXT, hover).lerp_to_gamma(theme::ACCENT_TEXT, active);
            let painter = ui.painter();
            painter.rect_filled(rect, 9.0, fill);
            painter.rect_stroke(rect, 9.0, egui::Stroke::new(1.0, border), egui::StrokeKind::Inside);
            paint_corner_icon(painter, rect.center(), index, *on, line);
        }
    });
    changed
}

/// The corner of a square: an arc when `rounded`, a right angle when not. `index` is the corner:
/// 0 top left, 1 top right, 2 bottom right, 3 bottom left.
fn paint_corner_icon(painter: &egui::Painter, center: egui::Pos2, index: usize, rounded: bool, color: Color32) {
    const HALF: f32 = 7.5;
    const RADIUS: f32 = 6.0;
    let left = index == 0 || index == 3;
    let top = index == 0 || index == 1;
    // The corner itself, and the direction of its two edges (towards the inside of the square).
    let corner = egui::pos2(
        center.x + if left { -HALF } else { HALF },
        center.y + if top { -HALF } else { HALF },
    );
    let dx = if left { 1.0 } else { -1.0 };
    let dy = if top { 1.0 } else { -1.0 };
    let length = HALF * 2.0;
    let radius = if rounded { RADIUS } else { 0.0 };

    let mut points = vec![egui::pos2(corner.x, corner.y + dy * length), egui::pos2(corner.x, corner.y + dy * radius)];
    if rounded {
        let arc_center = egui::pos2(corner.x + dx * radius, corner.y + dy * radius);
        for step in 1..=8 {
            let angle = std::f32::consts::FRAC_PI_2 * step as f32 / 8.0;
            points.push(egui::pos2(arc_center.x - dx * radius * angle.cos(), arc_center.y - dy * radius * angle.sin()));
        }
    } else {
        points.push(corner);
    }
    points.push(egui::pos2(corner.x + dx * length, corner.y));
    painter.add(egui::Shape::line(points, egui::Stroke::new(2.0, color)));
}

/// An on/off switch like the one of Jaca anti Janja. Returns true when it was clicked.
pub fn switch(ui: &mut egui::Ui, on: &mut bool) -> bool {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(40.0, 22.0), egui::Sense::click());
    let response = response.on_hover_cursor(egui::CursorIcon::PointingHand);
    let clicked = response.clicked();
    if clicked {
        *on = !*on;
    }
    let t = ui.ctx().animate_bool_with_time(response.id, *on, 0.2);
    paint_switch(ui.painter(), rect, t);
    clicked
}

/// Draws the switch in `rect`; the caller handles the click.
/// Draws the switch `t` of the way from off (0) to on (1).
pub fn paint_switch(painter: &egui::Painter, rect: egui::Rect, t: f32) {
    let off_knob = Color32::from_rgb(207, 208, 218);
    let fill = theme::HOVER.lerp_to_gamma(theme::ACCENT, t);
    let knob = off_knob.lerp_to_gamma(theme::ON_ACCENT, t);
    painter.rect_filled(rect, 99.0, fill);
    painter.rect_stroke(rect, 99.0, egui::Stroke::new(1.0, theme::BORDER.lerp_to_gamma(theme::ACCENT, t)), egui::StrokeKind::Inside);
    let x = egui::lerp((rect.left() + 11.5)..=(rect.right() - 11.5), t);
    painter.circle_filled(egui::pos2(x, rect.center().y), 7.5, knob);
}

/// A pill with one button per option; the chosen one is filled with the accent.
///
/// `id_salt` names the selector: its animations are kept under that name, so they do not get mixed
/// up when widgets are added or removed above it. When `value` is none of the options (a custom
/// choice) the labels stay and the highlight fades out.
pub fn segmented<T: PartialEq + Copy>(ui: &mut egui::Ui, id_salt: &str, value: &mut T, options: &[(T, &str)]) {
    let id = egui::Id::unique(("segmented", id_salt));
    egui::Frame::new()
        .fill(theme::BACKGROUND)
        .stroke(egui::Stroke::new(1.0, theme::BORDER))
        .corner_radius(10)
        .inner_margin(3)
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.x = 2.0;
            let font = egui::FontId::new(13.5, egui::FontFamily::Name("semibold".into()));
            ui.horizontal(|ui| {
                // First place every option, then draw the selector under the texts: it glides to
                // the chosen option instead of jumping.
                let mut slots = Vec::new();
                for (option, label) in options {
                    let galley = ui.painter().layout_no_wrap(t(label).to_owned(), font.clone(), Color32::WHITE);
                    let size = galley.size() + egui::vec2(28.0, 14.0);
                    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());
                    let response = response.on_hover_cursor(egui::CursorIcon::PointingHand);
                    if response.clicked() {
                        *value = *option;
                    }
                    slots.push((*option, rect, response.hovered(), galley));
                }
                // The positions are relative to the start of the row, so a row that slides in
                // with its screen does not make the selector chase it.
                let origin = slots[0].1.left();
                let top = slots[0].1.top();
                let bottom = slots[0].1.bottom();
                let last_id = id.with("last");
                let chosen_slot = slots.iter().find(|slot| slot.0 == *value).map(|slot| slot.1);
                if let Some(rect) = chosen_slot {
                    ui.data_mut(|data| data.insert_temp(last_id, (rect.left() - origin, rect.right() - origin)));
                }
                let remembered = ui.data(|data| data.get_temp::<(f32, f32)>(last_id));
                if let Some((from, to)) = remembered {
                    let shown = ui.ctx().animate_bool_with_time(id.with("shown"), chosen_slot.is_some(), 0.2);
                    let left = origin + ui.ctx().animate_value_with_time(id.with("left"), from, 0.25);
                    let right = origin + ui.ctx().animate_value_with_time(id.with("right"), to, 0.25);
                    let pill = egui::Rect::from_min_max(egui::pos2(left, top), egui::pos2(right, bottom));
                    ui.painter().rect_filled(pill, 8.0, theme::ACCENT.gamma_multiply(shown));
                }
                for (index, (option, rect, hovered, galley)) in slots.into_iter().enumerate() {
                    let chosen = ui.ctx().animate_bool_with_time(id.with(("on", index)), option == *value, 0.2);
                    let hover = if hovered { 1.0 } else { 0.0 };
                    let color = theme::MUTED.lerp_to_gamma(theme::TEXT, hover * 0.8).lerp_to_gamma(theme::ON_ACCENT, chosen);
                    ui.painter().galley(rect.center() - galley.size() / 2.0, galley, color);
                }
            });
        });
}

/// A titled block with a background, to separate the parts of the editor.
pub fn card(ui: &mut egui::Ui, title: &str, content: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::new()
        .fill(theme::PANEL)
        .stroke(egui::Stroke::new(1.0, theme::BORDER))
        .corner_radius(14)
        .inner_margin(16)
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(theme::semibold(t(title), 16.0).color(theme::ACCENT));
            ui.add_space(6.0);
            content(ui);
        });
}
