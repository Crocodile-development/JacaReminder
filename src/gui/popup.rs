// The popup window: a small box that slides in from the side of the screen it is on, stays on top
// and shows the title and message of a reminder (and a picture, if it has one). An alert slides out
// by itself (or when clicked). A task stays until one of its two buttons is pressed: Snooze brings
// it back later, Complete ends it.

use std::time::{Duration, Instant};

use eframe::egui::{
    self, Align2, Color32, ColorImage, CursorIcon, FontFamily, FontId, Pos2, Rect, Sense, TextureHandle,
    Vec2, ViewportBuilder, ViewportCommand,
};

use super::theme;
use crate::integration::window_shape::{self, Shape};
use crate::integration::sound;
use crate::shared::i18n::t;
use crate::shared::reminder::{ImageMode, PopupRequest, PopupStyle, TaskPopup};
use crate::shared::storage::{self, Action, Answer};

/// Space around the text and the buttons.
const PADDING: f32 = 16.0;
const BUTTON_HEIGHT: f32 = 32.0;
/// A task needs room for its buttons.
const TASK_MIN_HEIGHT: f32 = 136.0;
/// How long the popup takes to slide in and out, in seconds.
const SLIDE_IN: f32 = 0.34;
const SLIDE_OUT: f32 = 0.26;
/// The biggest side of the picture kept in memory, in pixels.
const MAX_IMAGE_SIDE: u32 = 800;

fn popup_size(style: &PopupStyle, task: bool) -> Vec2 {
    let min_height = if task { TASK_MIN_HEIGHT } else { 60.0 };
    Vec2::new(style.width.max(160) as f32, (style.height as f32).max(min_height))
}

/// The picture of the popup, made small enough to keep in memory. None if there is no picture or
/// it cannot be read.
fn load_image(path: &str) -> Option<ColorImage> {
    let picture = image::open(path).ok()?.thumbnail(MAX_IMAGE_SIDE, MAX_IMAGE_SIDE).to_rgba8();
    let size = [picture.width() as usize, picture.height() as usize];
    Some(ColorImage::from_rgba_unmultiplied(size, picture.as_raw()))
}

pub fn run(request: PopupRequest) -> eframe::Result {
    let PopupRequest { style, task } = request;
    // The sound lasts as long as the popup is on the screen (see `begin_leaving`).
    sound::start_looping(&style.sound);

    let size = popup_size(&style, task.is_some());
    let image = style.image.as_deref().filter(|path| !path.is_empty()).and_then(load_image);
    eframe::run_native(
        "jaca-reminder-popup",
        eframe::NativeOptions {
            viewport: ViewportBuilder::default()
                .with_title("Jaca Reminder")
                .with_decorations(false)
                .with_always_on_top()
                .with_taskbar(false)
                .with_resizable(false)
                .with_inner_size(size)
                // Hidden, on the main screen (its top-left corner is the origin), until the
                // window is moved beyond the edge of the screen (see `ui`).
                .with_visible(false)
                .with_position([0.0, 0.0]),
            // The popup must not remember a position from the last one.
            persist_window: false,
            ..Default::default()
        },
        Box::new(move |cc| {
            theme::load_fonts(&cc.egui_ctx);
            Ok(Box::new(PopupApp {
                limiter: super::FrameLimiter::new(),
                style,
                task,
                size,
                image,
                texture: None,
                stage: Stage::New,
                phase: Phase::Entering(Instant::now()),
                start: Pos2::ZERO,
                target: Pos2::ZERO,
                answered: false,
            }))
        }),
    )
}

/// Getting the window ready: it is moved out of sight first and shown one frame later, so it never
/// flashes at the top left of the screen.
enum Stage {
    New,
    Moved,
    Running,
}

enum Phase {
    /// Sliding in; since when.
    Entering(Instant),
    /// On its place; since when.
    Shown(Instant),
    /// Sliding out; since when. The window closes at the end.
    Leaving(Instant),
}

struct PopupApp {
    limiter: super::FrameLimiter,
    style: PopupStyle,
    /// Set for a task: the popup then has the Snooze and Complete buttons and does not slide out alone.
    task: Option<TaskPopup>,
    size: Vec2,
    /// The picture, until it becomes a texture.
    image: Option<ColorImage>,
    texture: Option<TextureHandle>,
    stage: Stage,
    phase: Phase,
    /// Where the window starts, beyond the edge of the screen, and where it stays.
    start: Pos2,
    target: Pos2,
    /// True once a button was pressed, so closing the window afterwards does not snooze.
    answered: bool,
}

impl PopupApp {
    /// Works out where the popup stays and where it starts. The distance from the edge is counted
    /// from the working area, so the popup stays clear of the taskbar.
    fn compute_positions(&mut self, pixels_per_point: f32) {
        let screen = window_shape::primary_screen();
        let to_points = |pixels: i32| pixels as f32 / pixels_per_point;
        let [left, top, right, bottom] = screen.work.map(to_points);
        let across = self.style.edge_distance as f32;
        let down = self.style.edge_distance_y as f32;
        let on_right = self.style.corner.on_right();
        let x = if on_right { right - self.size.x - across } else { left + across };
        let y = if self.style.corner.at_bottom() { bottom - self.size.y - down } else { top + down };
        self.target = Pos2::new(x, y);
        let start_x = if on_right { to_points(screen.full[2]) } else { -self.size.x };
        self.start = Pos2::new(start_x, y);
    }

    /// Starts to slide out. The sound stops right away: a task is answered, an alert is leaving.
    fn begin_leaving(&mut self) {
        if !matches!(self.phase, Phase::Leaving(_)) {
            sound::stop();
            self.phase = Phase::Leaving(Instant::now());
        }
    }

    /// Leaves the answer for the daemon and slides out. A preview (id 0) answers nothing.
    fn answer(&mut self, answer: Answer) {
        self.answered = true;
        if let Some(task) = self.task.as_ref().filter(|task| task.id != 0) {
            storage::append_action(&Action { id: task.id, answer });
        }
        self.begin_leaving();
    }
}

fn color(rgb: [u8; 3]) -> Color32 {
    Color32::from_rgb(rgb[0], rgb[1], rgb[2])
}

/// The part of a picture (in 0..1 of its size) that fills a box of `target` size without being
/// stretched: the picture is cut at the sides or at the top and bottom.
fn cover_uv(image: Vec2, target: Vec2) -> Rect {
    let image_ratio = image.x / image.y;
    let target_ratio = target.x / target.y;
    if image_ratio > target_ratio {
        let width = target_ratio / image_ratio;
        Rect::from_min_max(Pos2::new((1.0 - width) / 2.0, 0.0), Pos2::new((1.0 + width) / 2.0, 1.0))
    } else {
        let height = image_ratio / target_ratio;
        Rect::from_min_max(Pos2::new(0.0, (1.0 - height) / 2.0), Pos2::new(1.0, (1.0 + height) / 2.0))
    }
}

/// A button painted in the colors of the popup. The colors fade when the mouse enters and leaves.
fn popup_button(ui: &mut egui::Ui, rect: Rect, label: &str, filled: bool, style: &PopupStyle) -> bool {
    let response = ui
        .interact(rect, ui.scope_id().with(label), Sense::click())
        .on_hover_cursor(CursorIcon::PointingHand);
    let hover = ui.ctx().animate_bool_with_time(response.id, response.hovered(), 0.15);
    let text = color(style.text);
    let accent = color(style.accent);
    let (fill, border, label_color) = if filled {
        let fill = accent.lerp_to_gamma(Color32::WHITE, hover * 0.18);
        (fill, fill, color(style.background))
    } else {
        (
            Color32::TRANSPARENT.lerp_to_gamma(text.gamma_multiply(0.14), hover),
            text.gamma_multiply(0.35),
            text,
        )
    };
    let painter = ui.painter();
    painter.rect_filled(rect, 8.0, fill);
    painter.rect_stroke(rect, 8.0, egui::Stroke::new(1.0, border), egui::StrokeKind::Inside);
    painter.text(
        rect.center(),
        Align2::CENTER_CENTER,
        label,
        FontId::new(13.5, FontFamily::Name("semibold".into())),
        label_color,
    );
    response.clicked()
}

impl eframe::App for PopupApp {
    /// The same color as the box, so no light edge shows if a pixel is left unpainted.
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        color(self.style.background).to_normalized_gamma_f32()
    }

    /// Closing the window of a task without pressing a button counts as a snooze, so the task
    /// is never lost.
    fn on_exit(&mut self) {
        if !self.answered {
            if let Some(task) = self.task.as_ref().filter(|task| task.id != 0) {
                storage::append_action(&Action { id: task.id, answer: Answer::Snooze });
            }
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        self.limiter.wait();
        let ctx = ui.ctx().clone();
        let pixels_per_point = ctx.pixels_per_point();
        let shape = Shape {
            radius: self.style.rounding as f32 * pixels_per_point,
            rounded: self.style.round_corners,
        };
        let screen = window_shape::primary_screen();

        match self.stage {
            Stage::New => {
                self.compute_positions(pixels_per_point);
                ctx.send_viewport_cmd(ViewportCommand::OuterPosition(self.start));
                // Nothing of the window shows until it is beyond the edge.
                window_shape::apply(frame, &shape, Some([0, 0, 0, 0]));
                self.stage = Stage::Moved;
                ctx.request_repaint();
                return;
            }
            Stage::Moved => {
                ctx.send_viewport_cmd(ViewportCommand::Visible(true));
                self.phase = Phase::Entering(Instant::now());
                self.stage = Stage::Running;
            }
            Stage::Running => {}
        }

        // Where the window is now: sliding in slows down at the end, sliding out speeds up.
        let mut x = self.target.x;
        let mut animating = false;
        match self.phase {
            Phase::Entering(since) => {
                let progress = (since.elapsed().as_secs_f32() / SLIDE_IN).min(1.0);
                x = self.start.x + (self.target.x - self.start.x) * (1.0 - (1.0 - progress).powi(3));
                animating = true;
                if progress >= 1.0 {
                    self.phase = Phase::Shown(Instant::now());
                }
            }
            Phase::Shown(since) => {
                // A task never goes away by itself.
                let timed = self.task.is_none() && self.style.seconds > 0;
                if timed && since.elapsed() >= Duration::from_secs(self.style.seconds as u64) {
                    self.begin_leaving();
                }
            }
            Phase::Leaving(since) => {
                let progress = (since.elapsed().as_secs_f32() / SLIDE_OUT).min(1.0);
                x = self.target.x + (self.start.x - self.target.x) * progress.powi(3);
                animating = true;
                if progress >= 1.0 {
                    ctx.send_viewport_cmd(ViewportCommand::Close);
                }
            }
        }
        if animating {
            ctx.send_viewport_cmd(ViewportCommand::OuterPosition(Pos2::new(x, self.target.y)));
        }
        // While it slides, only the part inside the screen shows, so it never appears on the
        // monitor next to this one.
        window_shape::apply(frame, &shape, animating.then_some(screen.full));

        if self.texture.is_none() {
            if let Some(image) = self.image.take() {
                self.texture = Some(ctx.load_texture("popup-image", image, egui::TextureOptions::LINEAR));
            }
        }

        let is_task = self.task.is_some();
        let leaving = matches!(self.phase, Phase::Leaving(_));
        let mut answer = None;
        let mut close_requested = false;
        egui::CentralPanel::default().frame(egui::Frame::NONE).show(ui, |ui| {
            let full = ui.max_rect();
            // An alert slides out when clicked anywhere; a task only through its buttons.
            let clicked_anywhere = !is_task
                && !leaving
                && ui.interact(full, ui.scope_id().with("popup"), Sense::click()).clicked();
            let background = color(self.style.background);
            // The corners are cut by the window itself (see `window_shape`).
            ui.painter().rect_filled(full, 0.0, background);

            let mut text_left = full.left() + PADDING;
            if let Some(texture) = &self.texture {
                match self.style.image_mode {
                    ImageMode::Background => {
                        let uv = cover_uv(texture.size_vec2(), full.size());
                        ui.painter().image(texture.id(), full, uv, Color32::WHITE);
                        // Darkened, so the text stays readable over any picture.
                        ui.painter().rect_filled(full, 0.0, background.gamma_multiply(0.62));
                    }
                    ImageMode::Icon => {
                        let buttons = if is_task { BUTTON_HEIGHT + 8.0 } else { 0.0 };
                        let side = (full.height() - PADDING * 2.0 - buttons).clamp(32.0, 84.0);
                        let rect = Rect::from_min_size(
                            Pos2::new(full.left() + PADDING, full.top() + PADDING),
                            Vec2::splat(side),
                        );
                        let uv = cover_uv(texture.size_vec2(), rect.size());
                        ui.painter().add(
                            egui::epaint::RectShape::filled(rect, 10.0, Color32::WHITE)
                                .with_texture(texture.id(), uv),
                        );
                        text_left += side + 12.0;
                    }
                }
            }

            let text_color = color(self.style.text);
            let text_width = (full.right() - PADDING - text_left).max(40.0);
            let title = ui.painter().layout(
                self.style.title.clone(),
                FontId::new(17.0, FontFamily::Name("semibold".into())),
                text_color,
                text_width,
            );
            let title_height = title.size().y;
            ui.painter().galley(Pos2::new(text_left, full.top() + 14.0), title, text_color);
            let message = ui.painter().layout(
                self.style.message.clone(),
                FontId::proportional(14.0),
                text_color.gamma_multiply(0.85),
                text_width,
            );
            ui.painter().galley(Pos2::new(text_left, full.top() + 14.0 + title_height + 6.0), message, text_color);

            if let Some(task) = &self.task {
                let top = full.bottom() - PADDING + 2.0 - BUTTON_HEIGHT;
                let complete_label = t("Complete");
                let snooze_label = t("Snooze {minutes} min").replace("{minutes}", &task.snooze_minutes.to_string());
                let font = FontId::new(13.5, FontFamily::Name("semibold".into()));
                let measure = |ui: &egui::Ui, text: &str| {
                    ui.painter().layout_no_wrap(text.to_owned(), font.clone(), Color32::WHITE).size().x + 28.0
                };
                let complete_width = measure(ui, complete_label);
                let snooze_width = measure(ui, &snooze_label);
                let complete = Rect::from_min_size(
                    Pos2::new(full.right() - PADDING - complete_width, top),
                    Vec2::new(complete_width, BUTTON_HEIGHT),
                );
                let snooze = Rect::from_min_size(
                    Pos2::new(complete.left() - 8.0 - snooze_width, top),
                    Vec2::new(snooze_width, BUTTON_HEIGHT),
                );
                if popup_button(ui, snooze, &snooze_label, false, &self.style) && !leaving {
                    answer = Some(Answer::Snooze);
                }
                if popup_button(ui, complete, complete_label, true, &self.style) && !leaving {
                    answer = Some(Answer::Complete);
                }
            }

            if let (Phase::Shown(since), true) = (&self.phase, self.style.progress_bar && !is_task && self.style.seconds > 0) {
                let total = self.style.seconds as f32;
                let left = (1.0 - since.elapsed().as_secs_f32() / total).clamp(0.0, 1.0);
                let bar = Rect::from_min_max(
                    Pos2::new(full.right() - full.width() * left, full.bottom() - 4.0),
                    full.right_bottom(),
                );
                ui.painter().rect_filled(bar, 0.0, color(self.style.accent));
            }

            close_requested = clicked_anywhere;
        });
        if let Some(answer) = answer {
            self.answer(answer);
        }
        if close_requested {
            self.begin_leaving();
        }

        ctx.request_repaint_after(Duration::from_millis(33));
    }
}
