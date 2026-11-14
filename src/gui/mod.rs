// Window mode, built with egui.
//   app.rs              the dashboard: state of the window and how each frame is put together
//   titlebar.rs         the top bar that replaces the system one
//   theme.rs            colors and sizes
//   nav.rs              the sidebar with the pages (Reminders, History, Settings)
//   home.rs             the home page: the grid of reminders and the state of the service
//   reminder_editor.rs  the editor page: name, schedule and popup of one reminder
//   schedule_editor.rs  the schedule part of the editor (interval, calendar, one time)
//   popup_editor.rs     the popup part of the editor
//   history.rs          the history page
//   settings.rs         the settings page (volume, pause, start with Windows, data)
//   widgets.rs          small inputs used by the editors (date, time, toggles)
//   popup.rs            the popup window itself
mod app;
mod history;
mod home;
mod popup;
mod popup_editor;
mod reminder_editor;
mod settings;
mod nav;
mod schedule_editor;
mod theme;
mod titlebar;
mod widgets;

use eframe::egui::ViewportBuilder;

use crate::integration::single_instance::{self, Dashboard};
use crate::shared::reminder::PopupRequest;

/// The most frames per second the windows draw.
const MAX_FPS: u64 = 30;

/// Keeps a window at `MAX_FPS`: called at the start of each frame, it waits if the frame came
/// sooner than a frame time after the one before. A frame that comes after a pause (a click, a
/// key) is not delayed.
pub struct FrameLimiter {
    last: Option<std::time::Instant>,
}

impl FrameLimiter {
    pub const fn new() -> Self {
        Self { last: None }
    }

    pub fn wait(&mut self) {
        let frame = std::time::Duration::from_micros(1_000_000 / MAX_FPS);
        if let Some(last) = self.last {
            let since = last.elapsed();
            if since < frame {
                std::thread::sleep(frame - since);
            }
        }
        self.last = Some(std::time::Instant::now());
    }
}

fn window_icon(viewport: ViewportBuilder) -> ViewportBuilder {
    match eframe::icon_data::from_png_bytes(include_bytes!("../../assets/app-icon.png")) {
        Ok(icon) => viewport.with_icon(icon),
        Err(_) => viewport,
    }
}

pub fn run_dashboard() -> eframe::Result {
    // Another dashboard open: ask it to come to the front and quit.
    let repaint_handle = std::sync::Arc::new(std::sync::Mutex::new(None::<eframe::egui::Context>));
    let handle_for_thread = repaint_handle.clone();
    let requests = match single_instance::claim_dashboard(move || {
        if let Some(ctx) = handle_for_thread.lock().ok().and_then(|guard| guard.clone()) {
            ctx.request_repaint();
        }
    }) {
        Dashboard::Forwarded => return Ok(()),
        Dashboard::First(receiver) => Some(receiver),
        Dashboard::Alone => None,
    };

    eframe::run_native(
        "jaca-reminder",
        eframe::NativeOptions {
            viewport: window_icon(
                ViewportBuilder::default()
                    .with_title("Jaca Reminder")
                    .with_decorations(false)
                    .with_inner_size([980.0, 680.0])
                    .with_min_inner_size([760.0, 520.0]),
            ),
            ..Default::default()
        },
        Box::new(move |cc| {
            if let Ok(mut guard) = repaint_handle.lock() {
                *guard = Some(cc.egui_ctx.clone());
            }
            theme::apply(&cc.egui_ctx);
            Ok(Box::new(app::DashboardApp::new(requests)))
        }),
    )
}

pub fn run_popup(request: PopupRequest) -> eframe::Result {
    popup::run(request)
}
