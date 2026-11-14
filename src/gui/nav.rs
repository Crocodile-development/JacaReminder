// The sidebar: the logo and the three pages, each with an icon and its name.

use eframe::egui::{self, Align2, Color32, FontFamily, FontId, Painter, Pos2, Rect, Sense, Stroke, TextureHandle, Vec2};

use super::theme;
use crate::shared::i18n::t;

pub const WIDTH: f32 = 210.0;
const ITEM_HEIGHT: f32 = 42.0;

#[derive(PartialEq, Clone, Copy)]
pub enum Page {
    Reminders,
    History,
    Settings,
}

impl Page {
    const ALL: [Page; 3] = [Page::Reminders, Page::History, Page::Settings];

    fn label(self) -> &'static str {
        match self {
            Page::Reminders => t("Reminders"),
            Page::History => t("History"),
            Page::Settings => t("Settings"),
        }
    }
}

/// Draws the sidebar content and returns the page that was clicked.
pub fn show(ui: &mut egui::Ui, active: Page, logo: &TextureHandle) -> Option<Page> {
    let mut clicked = None;
    ui.spacing_mut().item_spacing.y = 6.0;

    ui.add_space(10.0);
    ui.horizontal(|ui| {
        ui.add_space(6.0);
        let (logo_rect, _) = ui.allocate_exact_size(Vec2::splat(34.0), Sense::hover());
        ui.painter().image(
            logo.id(),
            logo_rect,
            Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
            Color32::WHITE,
        );
        ui.label(theme::bold("Jaca Reminder", 16.0));
    });
    ui.add_space(14.0);

    slide_pill(ui, "pages", Page::ALL.iter().position(|page| *page == active).unwrap_or(0));
    for page in Page::ALL {
        if item(ui, page, page == active).clicked() {
            clicked = Some(page);
        }
    }

    ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
        ui.add_space(8.0);
        ui.label(
            egui::RichText::new(format!("{} {}", t("Version"), env!("CARGO_PKG_VERSION")))
                .small()
                .color(theme::MUTED),
        );
    });
    clicked
}

fn item(ui: &mut egui::Ui, page: Page, active: bool) -> egui::Response {
    row(ui, page.label(), active, 50.0, |painter, center, color| {
        paint_icon(painter, page, center, color)
    })
}

/// The highlight behind the active row. It glides to the row at `index` (counting from the
/// current position of the cursor) instead of jumping, like the sliding selector of Jaca anti Janja.
pub fn slide_pill(ui: &mut egui::Ui, id: &str, index: usize) {
    let cursor = ui.cursor();
    let target = cursor.top() + index as f32 * (ITEM_HEIGHT + ui.spacing().item_spacing.y);
    let top = ui.ctx().animate_value_with_time(ui.scope_id().with(("pill", id)), target, 0.3);
    let rect = Rect::from_min_size(Pos2::new(cursor.left(), top), Vec2::new(ui.available_width(), ITEM_HEIGHT));
    ui.painter().rect_filled(rect, 12.0, theme::ACCENT_TINT);
}

/// A sidebar row without icon, used by the settings categories.
pub fn text_item(ui: &mut egui::Ui, label: &str, active: bool) -> egui::Response {
    row(ui, label, active, 18.0, |_, _, _| {})
}

/// A round "back" button with a chevron.
pub fn back_button(ui: &mut egui::Ui) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(34.0), Sense::click());
    let response = response.on_hover_cursor(egui::CursorIcon::PointingHand);
    let hover = ui.ctx().animate_bool_with_time(response.id, response.hovered(), 0.15);
    let painter = ui.painter();
    let c = rect.center();
    painter.circle_filled(c, 17.0, Color32::TRANSPARENT.lerp_to_gamma(theme::CARD, hover));
    painter.circle_stroke(c, 16.5, Stroke::new(1.5, theme::BORDER.lerp_to_gamma(theme::ACCENT, hover)));
    let color = theme::MUTED.lerp_to_gamma(theme::TEXT, hover);
    let stroke = Stroke::new(2.0, color);
    painter.line_segment([c + Vec2::new(2.5, -6.0), c + Vec2::new(-3.0, 0.0)], stroke);
    painter.line_segment([c + Vec2::new(-3.0, 0.0), c + Vec2::new(2.5, 6.0)], stroke);
    response
}

fn row(
    ui: &mut egui::Ui,
    label: &str,
    active: bool,
    text_x: f32,
    icon: impl FnOnce(&Painter, Pos2, Color32),
) -> egui::Response {
    let (rect, response) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), ITEM_HEIGHT), Sense::click());
    let response = response.on_hover_cursor(egui::CursorIcon::PointingHand);
    let hover = ui.ctx().animate_bool_with_time(response.id.with("hover"), response.hovered(), 0.15);
    let on = ui.ctx().animate_bool_with_time(response.id.with("on"), active, 0.2);
    let painter = ui.painter();
    // The active row is highlighted by the sliding pill, drawn before the rows.
    let fill = Color32::TRANSPARENT.lerp_to_gamma(theme::CARD, hover * (1.0 - on));
    painter.rect_filled(rect, 12.0, fill);
    let color = theme::MUTED.lerp_to_gamma(theme::TEXT, hover).lerp_to_gamma(theme::ACCENT_TEXT, on);
    icon(painter, Pos2::new(rect.left() + 26.0, rect.center().y), color);
    painter.text(
        Pos2::new(rect.left() + text_x, rect.center().y),
        Align2::LEFT_CENTER,
        label,
        FontId::new(14.5, FontFamily::Name("semibold".into())),
        color,
    );
    response
}

fn paint_icon(painter: &Painter, page: Page, c: Pos2, color: Color32) {
    let stroke = Stroke::new(1.9, color);
    match page {
        Page::Reminders => {
            for y in [-6.0, 0.0, 6.0] {
                painter.circle_filled(c + Vec2::new(-7.0, y), 1.8, color);
                painter.line_segment([c + Vec2::new(-2.0, y), c + Vec2::new(8.0, y)], stroke);
            }
        }
        Page::History => {
            painter.circle_stroke(c, 9.0, stroke);
            painter.line_segment([c, c + Vec2::new(0.0, -5.5)], stroke);
            painter.line_segment([c, c + Vec2::new(4.0, 2.5)], stroke);
        }
        Page::Settings => {
            painter.line_segment([c + Vec2::new(-9.0, -4.5), c + Vec2::new(9.0, -4.5)], stroke);
            painter.line_segment([c + Vec2::new(-9.0, 4.5), c + Vec2::new(9.0, 4.5)], stroke);
            painter.circle_filled(c + Vec2::new(-3.0, -4.5), 3.2, color);
            painter.circle_filled(c + Vec2::new(3.0, 4.5), 3.2, color);
        }
    }
}
