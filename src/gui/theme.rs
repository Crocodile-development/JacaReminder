// Colors and sizes of the dashboard: near-black sea blues, an aqua accent, Segoe UI and 8px corners.

use eframe::egui::{self, Color32, Context, CornerRadius, FontFamily, FontId, RichText, Stroke, TextStyle};

pub const BACKGROUND: Color32 = Color32::from_rgb(12, 20, 24);
pub const PANEL: Color32 = Color32::from_rgb(18, 29, 34);
pub const CARD: Color32 = Color32::from_rgb(26, 42, 49);
pub const HOVER: Color32 = Color32::from_rgb(36, 58, 67);
pub const TITLE_BAR: Color32 = Color32::from_rgb(8, 14, 17);
pub const BORDER: Color32 = Color32::from_rgb(42, 66, 76);
pub const ACCENT: Color32 = Color32::from_rgb(61, 214, 198);
/// Text drawn on top of an ACCENT fill.
pub const ON_ACCENT: Color32 = Color32::from_rgb(6, 32, 34);
pub const TEXT: Color32 = Color32::from_rgb(242, 243, 247);
pub const MUTED: Color32 = Color32::from_rgb(155, 157, 171);
/// Fill of a ghost button and of the accent badges.
pub const ACCENT_TINT: Color32 = Color32::from_rgb(28, 72, 76);
pub const ACCENT_LINE: Color32 = Color32::from_rgb(44, 128, 126);
pub const ACCENT_TEXT: Color32 = Color32::from_rgb(168, 245, 235);
pub const DANGER: Color32 = Color32::from_rgb(255, 84, 104);

/// The surface of the page next to the rail.
pub const CONTENT: Color32 = Color32::from_rgb(20, 32, 38);

fn semibold_family() -> FontFamily {
    FontFamily::Name("semibold".into())
}

fn bold_family() -> FontFamily {
    FontFamily::Name("bold".into())
}

/// Text in the semibold weight.
pub fn semibold(text: impl Into<String>, size: f32) -> RichText {
    RichText::new(text).font(FontId::new(size, semibold_family()))
}

/// Text in the bold weight, for the big headings.
pub fn bold(text: impl Into<String>, size: f32) -> RichText {
    RichText::new(text).font(FontId::new(size, bold_family()))
}

/// The color of a reminder's tile, always the same for the same id.
pub fn tile_color(id: u64) -> Color32 {
    let hue = (id.wrapping_mul(67) % 360) as f32 / 360.0;
    egui::ecolor::Hsva::new(hue, 0.55, 0.78, 1.0).into()
}

/// Loads Segoe UI from Windows; if it is missing the built-in egui font stays in use.
pub fn load_fonts(ctx: &Context) {
    let read = |name: &str| std::fs::read(format!(r"C:\Windows\Fonts\{name}")).ok();
    let mut fonts = egui::FontDefinitions::default();
    if let Some(regular) = read("segoeui.ttf") {
        fonts.font_data.insert("segoe".to_owned(), std::sync::Arc::new(egui::FontData::from_owned(regular)));
        fonts.families.entry(FontFamily::Proportional).or_default().insert(0, "segoe".to_owned());
    }
    let base = fonts.families[&FontFamily::Proportional].clone();
    for (family, key, file) in [
        (semibold_family(), "segoe-semibold", "seguisb.ttf"),
        (bold_family(), "segoe-bold", "segoeuib.ttf"),
    ] {
        let mut chain = base.clone();
        if let Some(bytes) = read(file) {
            fonts.font_data.insert(key.to_owned(), std::sync::Arc::new(egui::FontData::from_owned(bytes)));
            chain.insert(0, key.to_owned());
        }
        fonts.families.insert(family, chain);
    }
    ctx.set_fonts(fonts);
}

pub fn apply(ctx: &Context) {
    load_fonts(ctx);
    let mut visuals = egui::Visuals::dark();
    visuals.panel_fill = BACKGROUND;
    visuals.window_fill = PANEL;
    visuals.extreme_bg_color = BACKGROUND;
    visuals.override_text_color = Some(TEXT);
    visuals.selection.bg_fill = Color32::from_rgb(34, 108, 108);
    visuals.slider_trailing_fill = true;
    visuals.interact_cursor = Some(egui::CursorIcon::PointingHand);
    visuals.selection.stroke = Stroke::new(1.0, ACCENT);
    visuals.hyperlink_color = ACCENT;
    visuals.widgets.noninteractive.bg_fill = PANEL;
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, BORDER);
    visuals.widgets.inactive.bg_fill = CARD;
    visuals.widgets.inactive.weak_bg_fill = CARD;
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, BORDER);
    visuals.widgets.hovered.bg_fill = HOVER;
    visuals.widgets.hovered.weak_bg_fill = HOVER;
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, Color32::from_rgb(48, 140, 134));
    visuals.widgets.active.weak_bg_fill = HOVER;
    visuals.widgets.active.bg_stroke = Stroke::new(1.0, ACCENT);
    let rounding = CornerRadius::same(8);
    visuals.widgets.noninteractive.corner_radius = rounding;
    visuals.widgets.inactive.corner_radius = rounding;
    visuals.widgets.hovered.corner_radius = rounding;
    visuals.widgets.active.corner_radius = rounding;
    ctx.set_visuals(visuals);

    ctx.global_style_mut(|style| {
        // Only text fields are selectable, like in a desktop app.
        style.interaction.selectable_labels = false;
        style.animation_time = 0.16;
        style.spacing.item_spacing = egui::vec2(10.0, 9.0);
        style.spacing.button_padding = egui::vec2(14.0, 7.0);
        style.text_styles.insert(TextStyle::Heading, FontId::new(24.0, semibold_family()));
        style.text_styles.insert(TextStyle::Body, FontId::proportional(14.0));
        style.text_styles.insert(TextStyle::Button, FontId::new(13.5, semibold_family()));
        style.text_styles.insert(TextStyle::Small, FontId::proportional(12.0));
    });
}
