// The editor page: everything about one reminder.

use eframe::egui::{self, Color32, RichText};

use super::widgets::{card, segmented};
use super::schedule_editor::labeled;
use super::{nav, popup_editor, schedule_editor, theme};
use crate::shared::i18n::t;
use crate::shared::reminder::{Kind, Reminder};

/// The parts of the editor, shown one at a time.
#[derive(PartialEq, Clone, Copy)]
pub enum Tab {
    General,
    Popup,
}

pub enum EditorAction {
    Save,
    Delete,
    Back,
}

/// A trash can button; the deletion itself is confirmed in a dialog.
fn trash_button(ui: &mut egui::Ui) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(egui::Vec2::splat(36.0), egui::Sense::click());
    let response = response
        .on_hover_cursor(egui::CursorIcon::PointingHand)
        .on_hover_text(t("Delete reminder"));
    let hover = ui.ctx().animate_bool_with_time(response.id, response.hovered(), 0.15);
    let painter = ui.painter();
    painter.rect_filled(rect, 10.0, Color32::TRANSPARENT.lerp_to_gamma(theme::CARD, hover));
    let color = theme::MUTED.lerp_to_gamma(theme::DANGER, hover);
    let stroke = egui::Stroke::new(1.8, color);
    let c = rect.center();
    let v = |x: f32, y: f32| c + egui::vec2(x, y);
    // lid and handle
    painter.line_segment([v(-8.0, -5.5), v(8.0, -5.5)], stroke);
    painter.line_segment([v(-2.5, -5.5), v(-2.5, -8.0)], stroke);
    painter.line_segment([v(-2.5, -8.0), v(2.5, -8.0)], stroke);
    painter.line_segment([v(2.5, -8.0), v(2.5, -5.5)], stroke);
    // body
    painter.line_segment([v(-6.0, -5.5), v(-5.0, 8.0)], stroke);
    painter.line_segment([v(-5.0, 8.0), v(5.0, 8.0)], stroke);
    painter.line_segment([v(5.0, 8.0), v(6.0, -5.5)], stroke);
    // slats
    painter.line_segment([v(-1.8, -2.0), v(-1.8, 4.5)], stroke);
    painter.line_segment([v(1.8, -2.0), v(1.8, 4.5)], stroke);
    response
}

/// Height kept free below the scroll area for the bottom bar.
const BAR_HEIGHT: f32 = 72.0;

pub fn show(
    ui: &mut egui::Ui,
    draft: &mut Reminder,
    tab: &mut Tab,
    content_fade: f32,
    is_new: bool,
    dirty: bool,
    confirming_delete: &mut bool,
) -> Option<EditorAction> {
    let mut action = None;

    ui.add_space(6.0);
    ui.horizontal(|ui| {
        if nav::back_button(ui).clicked() {
            action = Some(EditorAction::Back);
        }
        let title = if draft.name.trim().is_empty() { t("Untitled reminder") } else { draft.name.trim() };
        ui.label(theme::bold(title, 26.0));
    });
    ui.add_space(4.0);

    ui.horizontal(|ui| {
        segmented(
            ui,
            "editor_tabs",
            tab,
            &[(Tab::General, "General"), (Tab::Popup, "Popup")],
        );
        if !is_new {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if trash_button(ui).clicked() {
                    *confirming_delete = true;
                }
            });
        }
    });
    ui.add_space(6.0);

    let scroll_height = (ui.available_height() - BAR_HEIGHT).max(80.0);
    ui.scope(|ui| {
    ui.set_opacity(content_fade);
    egui::ScrollArea::vertical().max_height(scroll_height).auto_shrink([false, false]).show(ui, |ui| {
        match *tab {
            Tab::General => {
                card(ui, "Reminder", |ui| {
                    labeled(ui, "Name", |ui| {
                        ui.add(egui::TextEdit::singleline(&mut draft.name).desired_width(260.0));
                    });
                    labeled(ui, "Type", |ui| {
                        segmented(
                            ui,
                            "reminder_kind",
                            &mut draft.kind,
                            &[(Kind::Alert, "Alert"), (Kind::Task, "Task")],
                        );
                    });
                    let help = match draft.kind {
                        Kind::Alert => "Shows the popup and closes by itself.",
                        Kind::Task => {
                            "Stays on the screen until you complete it. If you snooze, it comes back after the time below, until you complete it."
                        }
                    };
                    ui.label(RichText::new(t(help)).color(theme::MUTED).small());
                    if draft.kind == Kind::Task {
                        labeled(ui, "Snooze for", |ui| {
                            ui.add(egui::DragValue::new(&mut draft.snooze_minutes).range(1..=1440).suffix(" min"));
                        });
                    }
                });
                ui.add_space(10.0);
                schedule_editor::show(ui, &mut draft.schedule);
            }
            Tab::Popup => {
                popup_editor::show_text(ui, &mut draft.popup);
                ui.add_space(10.0);
                let snooze = (draft.kind == Kind::Task).then_some(draft.snooze_minutes);
                popup_editor::show_look(ui, &mut draft.popup, snooze);
            }
        }
        ui.add_space(12.0);
    });
    });

    // The bar at the bottom with the save button.
    ui.separator();
    ui.add_space(6.0);
    ui.horizontal(|ui| {
        ui.set_min_height(36.0);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if super::widgets::primary_button(ui, "Save", dirty).clicked() {
                action = Some(EditorAction::Save);
            }
            if dirty {
                ui.label(RichText::new(t("unsaved changes")).color(theme::DANGER));
            }
        });
    });
    ui.add_space(10.0);

    if *confirming_delete && !is_new {
        let name = if draft.name.trim().is_empty() { t("Untitled reminder") } else { draft.name.trim() };
        let modal = egui::Modal::new(egui::Id::unique("delete_reminder_dialog")).show(ui.ctx(), |ui| {
            ui.set_width(340.0);
            ui.label(theme::bold(t("Delete this reminder?"), 20.0));
            ui.add_space(6.0);
            ui.label(
                RichText::new(t("\"{name}\" will be removed for good.").replace("{name}", name))
                    .color(theme::MUTED),
            );
            ui.add_space(16.0);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if super::widgets::danger_button(ui, "Delete").clicked() {
                    action = Some(EditorAction::Delete);
                }
                if super::widgets::button(ui, "Cancel").clicked() {
                    *confirming_delete = false;
                }
            });
        });
        // Escape or a click outside the dialog cancels it too.
        if modal.should_close() {
            *confirming_delete = false;
        }
    }

    action
}
