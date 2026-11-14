// The home page: a title bar with the filter and the "New reminder" button, the grid with every
// reminder, and a strip with the state of the background service.

use chrono::NaiveDateTime;
use eframe::egui::{
    self, Align2, Color32, FontFamily, FontId, Pos2, Rect, RichText, Sense, Stroke, StrokeKind, Vec2,
};

use super::widgets::{paint_switch, segmented};
use super::theme;
use crate::shared::engine::Waiting;
use crate::shared::i18n::t;
use crate::shared::reminder::{Config, Kind, Reminder};
use crate::shared::storage::State;

/// The status of a task that waits for an answer.
const WAITING_COLOR: Color32 = Color32::from_rgb(255, 196, 77);
const GAP: f32 = 16.0;
const CARD_MIN_WIDTH: f32 = 280.0;
const CARD_HEIGHT: f32 = 132.0;

#[derive(PartialEq, Clone, Copy)]
pub enum Filter {
    All,
    Active,
    Paused,
}

pub enum HomeAction {
    New,
    Edit(u64),
    Toggle(u64, bool),
}

pub struct HomeData<'a> {
    pub config: &'a Config,
    pub state: &'a State,
    pub now: NaiveDateTime,
}

pub fn show(ui: &mut egui::Ui, data: &HomeData, filter: &mut Filter) -> Option<HomeAction> {
    let mut action = None;

    egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
        ui.spacing_mut().item_spacing = Vec2::new(GAP, GAP);
        ui.add_space(8.0);
        title_bar(ui, data, filter, &mut action);

        let shown: Vec<&Reminder> = data
            .config
            .reminders
            .iter()
            .filter(|reminder| match filter {
                Filter::All => true,
                Filter::Active => reminder.enabled,
                Filter::Paused => !reminder.enabled,
            })
            .collect();

        if shown.is_empty() {
            ui.add_space(40.0);
            ui.vertical_centered(|ui| {
                let (title, help) = if data.config.reminders.is_empty() {
                    ("No reminders yet", "Press New reminder to create the first one.")
                } else {
                    ("Nothing here", "No reminder matches this filter.")
                };
                ui.label(theme::semibold(t(title), 20.0));
                ui.label(RichText::new(t(help)).color(theme::MUTED));
            });
        }

        let columns = (((ui.available_width() + GAP) / (CARD_MIN_WIDTH + GAP)).floor() as usize).max(1);
        let card_width = (ui.available_width() - GAP * (columns as f32 - 1.0)) / columns as f32;
        for row in shown.chunks(columns) {
            ui.horizontal(|ui| {
                for reminder in row {
                    if let Some(card_action) = reminder_card(ui, card_width, reminder, data) {
                        action = Some(card_action);
                    }
                }
            });
        }
        ui.add_space(12.0);
    });

    action
}

fn title_bar(ui: &mut egui::Ui, data: &HomeData, filter: &mut Filter, action: &mut Option<HomeAction>) {
    let active = data.config.reminders.iter().filter(|reminder| reminder.enabled).count();
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = 2.0;
            ui.label(theme::bold(t("Reminders"), 30.0));
            if data.config.pause_all {
                ui.label(RichText::new(t("All reminders are paused (Settings)")).color(theme::DANGER));
            } else {
                let summary = t("{active} active of {total}")
                    .replace("{active}", &active.to_string())
                    .replace("{total}", &data.config.reminders.len().to_string());
                ui.label(RichText::new(summary).color(theme::MUTED));
            }
        });
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if super::widgets::primary_button(ui, "+ New reminder", true).clicked() {
                *action = Some(HomeAction::New);
            }
            // The row is built left to right inside a box of its own size, so the order stays All, Active, Paused.
            ui.allocate_ui_with_layout(
                Vec2::new(260.0, 36.0),
                egui::Layout::left_to_right(egui::Align::Center),
                |ui| {
                    segmented(
                        ui,
                        "home_filter",
                        filter,
                        &[(Filter::All, "All"), (Filter::Active, "Active"), (Filter::Paused, "Paused")],
                    );
                },
            );
        });
    });
}

fn reminder_card(ui: &mut egui::Ui, width: f32, reminder: &Reminder, data: &HomeData) -> Option<HomeAction> {
    let mut action = None;
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, CARD_HEIGHT), Sense::click());
    let response = response.on_hover_cursor(egui::CursorIcon::PointingHand);
    let hover = ui.ctx().animate_bool_with_time(response.id, response.hovered(), 0.16);
    if response.clicked() {
        action = Some(HomeAction::Edit(reminder.id));
    }

    let painter = ui.painter().clone();
    painter.rect_filled(rect, 16.0, theme::CARD.lerp_to_gamma(theme::HOVER, hover * 0.35));
    let border = theme::BORDER.lerp_to_gamma(theme::ACCENT, hover);
    painter.rect_stroke(rect, 16.0, Stroke::new(1.0, border), StrokeKind::Inside);

    // The color tile with the first letter.
    let tile = Rect::from_min_size(rect.min + Vec2::new(16.0, 16.0), Vec2::splat(48.0));
    let (tile_fill, letter_color) =
        if reminder.enabled { (theme::tile_color(reminder.id), Color32::WHITE) } else { (theme::HOVER, theme::MUTED) };
    painter.rect_filled(tile, 16.0, tile_fill);
    let letter = reminder.name.chars().next().map(|c| c.to_uppercase().to_string()).unwrap_or_default();
    painter.text(tile.center(), Align2::CENTER_CENTER, letter, FontId::new(22.0, FontFamily::Name("semibold".into())), letter_color);

    // Name and status.
    let text_x = tile.right() + 12.0;
    let text_width = rect.right() - 70.0 - text_x;
    let clip = Rect::from_min_size(Pos2::new(text_x, rect.top() + 12.0), Vec2::new(text_width, 56.0));
    let text_painter = painter.with_clip_rect(clip);
    text_painter.text(
        Pos2::new(text_x, rect.top() + 26.0),
        Align2::LEFT_CENTER,
        &reminder.name,
        FontId::new(16.0, FontFamily::Name("semibold".into())),
        theme::TEXT,
    );
    let (status, status_color) = status_line(reminder, data);
    let dot_color = if reminder.enabled { theme::ACCENT } else { theme::MUTED };
    painter.circle_filled(Pos2::new(text_x + 4.0, rect.top() + 50.0), 3.5, dot_color);
    text_painter.text(
        Pos2::new(text_x + 14.0, rect.top() + 50.0),
        Align2::LEFT_CENTER,
        status,
        FontId::proportional(13.0),
        status_color,
    );

    // The switch, on top of the card.
    let switch_rect = Rect::from_min_size(Pos2::new(rect.right() - 16.0 - 40.0, rect.top() + 22.0), Vec2::new(40.0, 22.0));
    let switch_response = ui
        .interact(switch_rect, ui.scope_id().with(("switch", reminder.id)), Sense::click())
        .on_hover_cursor(egui::CursorIcon::PointingHand);
    if switch_response.clicked() {
        action = Some(HomeAction::Toggle(reminder.id, !reminder.enabled));
    }
    let on = ui.ctx().animate_bool_with_time(switch_response.id, reminder.enabled, 0.2);
    paint_switch(&painter, switch_rect, on);

    // Footer: the schedule and the edit button.
    let footer_y = rect.bottom() - 16.0 - 28.0;
    let edit_rect = Rect::from_min_size(Pos2::new(rect.right() - 16.0 - 60.0, footer_y), Vec2::new(60.0, 28.0));
    let chip_width = (edit_rect.left() - 10.0 - (rect.left() + 16.0)).max(40.0);
    let galley = painter.layout_no_wrap(reminder.schedule.summary(), FontId::new(12.0, FontFamily::Name("semibold".into())), theme::MUTED);
    let chip = Rect::from_min_size(
        Pos2::new(rect.left() + 16.0, footer_y + 2.0),
        Vec2::new((galley.size().x + 18.0).min(chip_width), 24.0),
    );
    painter.rect_filled(chip, 8.0, theme::HOVER);
    painter.with_clip_rect(chip.shrink2(Vec2::new(9.0, 0.0))).galley(
        Pos2::new(chip.left() + 9.0, chip.center().y - galley.size().y / 2.0),
        galley,
        theme::MUTED,
    );

    let edit_response = ui
        .interact(edit_rect, ui.scope_id().with(("edit", reminder.id)), Sense::click())
        .on_hover_cursor(egui::CursorIcon::PointingHand);
    if edit_response.clicked() {
        action = Some(HomeAction::Edit(reminder.id));
    }
    let edit_hover = ui.ctx().animate_bool_with_time(edit_response.id, edit_response.hovered(), 0.15);
    let edit_fill = theme::ACCENT_TINT.lerp_to_gamma(theme::ACCENT_LINE, edit_hover);
    painter.rect_filled(edit_rect, 8.0, edit_fill);
    painter.rect_stroke(edit_rect, 8.0, Stroke::new(1.0, theme::ACCENT_LINE), StrokeKind::Inside);
    painter.text(
        edit_rect.center(),
        Align2::CENTER_CENTER,
        t("Edit"),
        FontId::new(13.0, FontFamily::Name("semibold".into())),
        theme::ACCENT_TEXT,
    );

    action
}

fn status_line(reminder: &Reminder, data: &HomeData) -> (String, Color32) {
    if !reminder.enabled {
        return (t("Paused").to_string(), theme::MUTED);
    }
    // A task that fired is waiting for an answer until it is completed.
    if reminder.kind == Kind::Task {
        if let Some((_, waiting)) = data.state.pending.iter().find(|(id, _)| *id == reminder.id) {
            return match waiting {
                Waiting::Showing => (t("Waiting for you").to_string(), WAITING_COLOR),
                Waiting::SnoozedUntil(time) => (
                    t("Snoozed until {time}").replace("{time}", &time.format("%H:%M").to_string()),
                    WAITING_COLOR,
                ),
            };
        }
    }
    match data.state.next.iter().find(|(id, _)| *id == reminder.id) {
        Some((_, time)) => (
            t("Next {time}").replace("{time}", &countdown_text(*time - data.now)),
            theme::ACCENT,
        ),
        None => (t("No more firings").to_string(), theme::MUTED),
    }
}

fn countdown_text(remaining: chrono::Duration) -> String {
    let seconds = remaining.num_seconds().max(0);
    let days = seconds / 86_400;
    let hours = (seconds % 86_400) / 3600;
    let minutes = (seconds % 3600) / 60;
    let rest = seconds % 60;
    if days > 0 {
        t("in {days}d {hours}h").replace("{days}", &days.to_string()).replace("{hours}", &hours.to_string())
    } else if hours > 0 {
        t("in {hours}h {minutes}m")
            .replace("{hours}", &hours.to_string())
            .replace("{minutes}", &format!("{minutes:02}"))
    } else {
        t("in {minutes}:{seconds}")
            .replace("{minutes}", &minutes.to_string())
            .replace("{seconds}", &format!("{rest:02}"))
    }
}
