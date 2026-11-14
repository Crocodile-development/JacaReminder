// The top bar. The window has no system title bar, so this bar moves the window when dragged
// and has the minimize, maximize and close buttons.

use eframe::egui::{
    self, Align, Color32, Layout, PointerButton, Rect, Sense, Stroke, StrokeKind, Vec2,
    ViewportCommand,
};

use super::theme;

const BUTTON_WIDTH: f32 = 42.0;

pub fn show(ui: &mut egui::Ui, title: &str, logo: &egui::TextureHandle) {
    let ctx = ui.ctx().clone();

    let bar = ui.interact(ui.max_rect(), ui.scope_id().with("titlebar_drag"), Sense::click_and_drag());
    let maximized = ctx.input(|input| input.viewport().maximized.unwrap_or(false));
    if bar.drag_started_by(PointerButton::Primary) {
        ctx.send_viewport_cmd(ViewportCommand::StartDrag);
    }
    if bar.double_clicked() {
        ctx.send_viewport_cmd(ViewportCommand::Maximized(!maximized));
    }

    ui.horizontal_centered(|ui| {
        ui.add_space(10.0);
        ui.add(egui::Image::new(logo).fit_to_exact_size(Vec2::splat(18.0)));
        ui.add_space(2.0);
        ui.label(egui::RichText::new(title).color(Color32::WHITE));

        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            if window_button(ui, Kind::Close).clicked() {
                ctx.send_viewport_cmd(ViewportCommand::Close);
            }
            if window_button(ui, Kind::Maximize).clicked() {
                ctx.send_viewport_cmd(ViewportCommand::Maximized(!maximized));
            }
            if window_button(ui, Kind::Minimize).clicked() {
                ctx.send_viewport_cmd(ViewportCommand::Minimized(true));
            }
        });
    });
}

#[derive(Clone, Copy, PartialEq)]
enum Kind {
    Minimize,
    Maximize,
    Close,
}

fn window_button(ui: &mut egui::Ui, kind: Kind) -> egui::Response {
    let size = Vec2::new(BUTTON_WIDTH, ui.available_height());
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    let response = response.on_hover_cursor(egui::CursorIcon::PointingHand);
    let hover = ui.ctx().animate_bool_with_time(response.id, response.hovered(), 0.12);
    let target = if kind == Kind::Close { theme::DANGER } else { Color32::from_white_alpha(25) };
    ui.painter().rect_filled(rect, 0.0, Color32::TRANSPARENT.lerp_to_gamma(target, hover));
    let stroke = Stroke::new(1.2, Color32::WHITE);
    let center = rect.center();
    let half = 5.0;
    let painter = ui.painter();
    match kind {
        Kind::Minimize => {
            painter.line_segment(
                [center - Vec2::new(half, 0.0), center + Vec2::new(half, 0.0)],
                stroke,
            );
        }
        Kind::Maximize => {
            painter.rect_stroke(
                Rect::from_center_size(center, Vec2::splat(half * 2.0)),
                0.0,
                stroke,
                StrokeKind::Inside,
            );
        }
        Kind::Close => {
            painter.line_segment([center - Vec2::splat(half), center + Vec2::splat(half)], stroke);
            painter.line_segment(
                [center + Vec2::new(-half, half), center + Vec2::new(half, -half)],
                stroke,
            );
        }
    }
    response
}
