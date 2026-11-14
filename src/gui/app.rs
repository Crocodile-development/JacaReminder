// The dashboard: the state of the window and how each frame is put together.

use std::sync::mpsc::Receiver;
use std::time::{Duration, Instant};

use chrono::Local;
use eframe::egui::{self, Margin, ViewportCommand};

use super::home::{self, Filter, HomeAction, HomeData};
use super::nav::{self, Page};
use super::reminder_editor::{self, EditorAction, Tab};
use super::settings::{self, SettingsAction};
use super::{history, theme, titlebar};
use crate::integration::{sound, startup};
use crate::shared::i18n::{self, t};
use crate::shared::reminder::{Config, Reminder, Sound};
use crate::shared::storage::{self, HistoryEntry, State};

#[derive(PartialEq, Clone, Copy)]
enum View {
    Home,
    Editor,
    History,
    Settings,
}

/// How far to the right a screen starts before it slides to its place, in pixels.
const SLIDE: f32 = 26.0;

pub struct DashboardApp {
    limiter: super::FrameLimiter,
    config: Config,
    state: State,
    history: Vec<HistoryEntry>,
    /// The reminder being edited: a copy, so nothing changes until Save.
    draft: Option<Reminder>,
    /// True while the draft has not been saved yet and has no id.
    draft_is_new: bool,
    /// A new draft as it was created, to know whether anything was changed before saving it.
    new_baseline: Option<Reminder>,
    selected: Option<u64>,
    view: View,
    category: settings::Category,
    /// What the content area showed last frame, to restart the fade-in when it changes.
    fade_key: (View, settings::Category),
    editor_tab: Tab,
    /// The tab the content of the editor is showing, and when it started to fade in.
    tab_seen: Tab,
    tab_start: Option<Instant>,
    fade_start: Option<Instant>,
    /// Where the content comes from when the fade-in starts.
    enter_from: egui::Vec2,
    /// True when the sidebar changes with the screen (pages <-> settings) and slides in too.
    sidebar_slides: bool,
    filter: Filter,
    confirming_delete: bool,
    confirming_clear: bool,
    last_refresh: Instant,
    show_requests: Option<Receiver<()>>,
    logo: Option<egui::TextureHandle>,
}

impl DashboardApp {
    pub fn new(show_requests: Option<Receiver<()>>) -> Self {
        let config = storage::load_config();
        i18n::set(config.language);
        Self {
            limiter: super::FrameLimiter::new(),
            sidebar_slides: false,
            draft: None,
            draft_is_new: false,
            new_baseline: None,
            config,
            state: storage::load_state(),
            history: storage::load_history(),
            selected: None,
            view: View::Home,
            category: settings::Category::General,
            fade_key: (View::Home, settings::Category::General),
            editor_tab: Tab::General,
            tab_seen: Tab::General,
            tab_start: None,
            fade_start: None,
            enter_from: egui::Vec2::ZERO,
            filter: Filter::All,
            confirming_delete: false,
            confirming_clear: false,
            last_refresh: Instant::now(),
            show_requests,
            logo: None,
        }
    }

    fn select(&mut self, id: u64) {
        self.selected = Some(id);
        self.draft = self.config.reminders.iter().find(|reminder| reminder.id == id).cloned();
        self.draft_is_new = false;
        self.confirming_delete = false;
        self.view = View::Editor;
    }

    fn start_new(&mut self) {
        self.selected = None;
        self.draft = Some(Reminder::new_default(t("New reminder")));
        self.new_baseline = self.draft.clone();
        self.draft_is_new = true;
        self.confirming_delete = false;
        self.editor_tab = Tab::General;
        self.view = View::Editor;
    }

    fn go_home(&mut self) {
        self.selected = None;
        self.draft = None;
        self.draft_is_new = false;
        self.confirming_delete = false;
        self.view = View::Home;
    }

    /// True when the draft differs from what is saved (or, for a new one, from how it started).
    fn dirty(&self) -> bool {
        match &self.draft {
            None => false,
            Some(draft) if self.draft_is_new => self.new_baseline.as_ref() != Some(draft),
            Some(draft) => {
                self.config.reminders.iter().find(|reminder| reminder.id == draft.id) != Some(draft)
            }
        }
    }

    fn save_draft(&mut self) {
        let Some(draft) = self.draft.clone() else { return };
        if self.draft_is_new {
            let id = self.config.add(draft);
            self.selected = Some(id);
            self.draft_is_new = false;
            self.draft = self.config.reminders.iter().find(|reminder| reminder.id == id).cloned();
        } else if let Some(slot) =
            self.config.reminders.iter_mut().find(|reminder| reminder.id == draft.id)
        {
            *slot = draft;
        }
        self.persist();
    }

    fn delete_selected(&mut self) {
        if let Some(id) = self.selected {
            self.config.reminders.retain(|reminder| reminder.id != id);
            self.persist();
        }
        self.go_home();
    }

    fn persist(&mut self) {
        let _ = storage::save_config(&self.config);
    }

    fn refresh(&mut self) {
        self.state = storage::load_state();
        self.history = storage::load_history();
        self.last_refresh = Instant::now();
    }

    /// The page highlighted in the navigation row; the editor belongs to Reminders.
    fn page(&self) -> Page {
        match self.view {
            View::Home | View::Editor => Page::Reminders,
            View::History => Page::History,
            View::Settings => Page::Settings,
        }
    }

    fn logo(&mut self, ctx: &egui::Context) -> egui::TextureHandle {
        self.logo
            .get_or_insert_with(|| {
                let image = eframe::icon_data::from_png_bytes(include_bytes!("../../assets/app-icon.png"))
                    .map(|icon| {
                        egui::ColorImage::from_rgba_unmultiplied(
                            [icon.width as usize, icon.height as usize],
                            &icon.rgba,
                        )
                    })
                    .unwrap_or_else(|_| egui::ColorImage::filled([1, 1], theme::ACCENT));
                ctx.load_texture("logo", image, egui::TextureOptions::LINEAR)
            })
            .clone()
    }
}

impl eframe::App for DashboardApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.limiter.wait();
        let ctx = ui.ctx().clone();
        if self.show_requests.as_ref().is_some_and(|receiver| receiver.try_recv().is_ok()) {
            ctx.send_viewport_cmd(ViewportCommand::Minimized(false));
            ctx.send_viewport_cmd(ViewportCommand::Focus);
        }
        if self.last_refresh.elapsed() >= Duration::from_secs(1) {
            self.refresh();
        }
        ctx.request_repaint_after(Duration::from_secs(1));

        let now = Local::now().naive_local();
        let logo = self.logo(&ctx);

        egui::Panel::top("titlebar")
            .exact_size(32.0)
            .frame(egui::Frame::new().fill(theme::TITLE_BAR))
            .show(ui, |ui| titlebar::show(ui, "Jaca Reminder", &logo));

        // A new screen starts a little to the side, slides to its place and stops there; the old
        // one is gone at once, so two screens never overlap. A change inside a screen only rises 8px.
        let key = (self.view, self.category);
        if key != self.fade_key {
            if self.view != self.fade_key.0 {
                self.tab_seen = self.editor_tab;
                self.enter_from = egui::vec2(SLIDE, 0.0);
                self.sidebar_slides = (self.fade_key.0 == View::Settings) != (self.view == View::Settings);
            } else {
                self.enter_from = egui::vec2(0.0, 8.0);
                self.sidebar_slides = false;
            }
            self.fade_key = key;
            self.fade_start = None;
        }
        // The clock starts on the frame after the change: the first frame is slow (new layout)
        // and would eat the beginning of the movement, which looks like a jump.
        let elapsed = self.fade_start.map_or(0.0, |start| start.elapsed().as_secs_f32());
        let duration = if self.enter_from.x != 0.0 { 0.36 } else { 0.26 };
        let progress = (elapsed / duration).clamp(0.0, 1.0);
        if progress < 1.0 {
            ctx.request_repaint();
        }
        if self.fade_start.is_none() {
            self.fade_start = Some(Instant::now());
        }
        let eased = 1.0 - (1.0 - progress).powi(3);

        // Switching the tabs of the editor only fades the content under the tab bar, so the
        // selector of the tabs stays visible while it glides.
        if self.editor_tab != self.tab_seen {
            self.tab_seen = self.editor_tab;
            self.tab_start = None;
        }
        let tab_progress = self.tab_start.map_or(0.0, |start| (start.elapsed().as_secs_f32() / 0.22).clamp(0.0, 1.0));
        if tab_progress < 1.0 {
            ctx.request_repaint();
        }
        if self.tab_start.is_none() {
            self.tab_start = Some(Instant::now());
        }
        let tab_fade = 1.0 - (1.0 - tab_progress).powi(3);

        let mut clicked_page = None;
        egui::Panel::left("sidebar")
            .exact_size(nav::WIDTH)
            .resizable(false)
            .frame(
                egui::Frame::new()
                    .fill(theme::PANEL)
                    .stroke(egui::Stroke::new(1.0, theme::BORDER))
                    .inner_margin(Margin::symmetric(12, 6)),
            )
            .show(ui, |ui| {
                // The sidebar changes between the pages and the settings categories: the new one
                // slides in with the screen.
                let shift = if self.sidebar_slides {
                    egui::vec2(self.enter_from.x * (1.0 - eased), 0.0)
                } else {
                    egui::Vec2::ZERO
                };
                let area = ui.max_rect().translate(shift);
                ui.scope_builder(egui::UiBuilder::new().max_rect(area), |ui| {
                    ui.set_opacity(if self.sidebar_slides { eased } else { 1.0 });
                    if self.view == View::Settings {
                        match settings::show_nav(ui, self.category) {
                            Some(settings::NavAction::Back) => clicked_page = Some(Page::Reminders),
                            Some(settings::NavAction::Select(category)) => {
                                self.category = category;
                                self.confirming_clear = false;
                            }
                            None => {}
                        }
                    } else {
                        clicked_page = nav::show(ui, self.page(), &logo);
                    }
                });
            });

        let mut home_action = None;
        let mut editor_action = None;
        let mut settings_action = None;
        let dirty = self.dirty();
        let is_new = self.draft_is_new;
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(theme::CONTENT).inner_margin(Margin::symmetric(28, 12)))
            .show(ui, |ui| {
                let area = ui.max_rect().translate(self.enter_from * (1.0 - eased));
                ui.scope_builder(egui::UiBuilder::new().max_rect(area), |ui| {
                ui.set_opacity(eased);
                match self.view {
                View::Home => {
                    let data = HomeData {
                        config: &self.config,
                        state: &self.state,
                        now,
                    };
                    home_action = home::show(ui, &data, &mut self.filter);
                }
                View::Editor => {
                    if let Some(draft) = self.draft.as_mut() {
                        editor_action =
                            reminder_editor::show(
                                ui,
                                draft,
                                &mut self.editor_tab,
                                tab_fade,
                                is_new,
                                dirty,
                                &mut self.confirming_delete,
                            );
                    }
                }
                View::History => {
                    ui.add_space(8.0);
                    ui.label(theme::bold(t("History"), 30.0));
                    ui.add_space(6.0);
                    history::show(ui, &self.history, now.date());
                }
                View::Settings => {
                    settings_action = settings::show(
                        ui,
                        self.category,
                        &mut self.config,
                        self.history.len(),
                        &mut self.confirming_clear,
                    );
                }
                }
                });
            });

        // A change of screen is seen at the start of the next frame, so ask for it.
        if clicked_page.is_some() || home_action.is_some() || editor_action.is_some() {
            ctx.request_repaint();
        }
        match clicked_page {
            Some(Page::Reminders) => self.go_home(),
            Some(Page::History) => {
                self.go_home();
                self.view = View::History;
            }
            Some(Page::Settings) => {
                self.go_home();
                self.confirming_clear = false;
                self.view = View::Settings;
            }
            None => {}
        }

        match home_action {
            Some(HomeAction::New) => self.start_new(),
            Some(HomeAction::Edit(id)) => {
                self.editor_tab = Tab::General;
                self.select(id);
            }
            Some(HomeAction::Toggle(id, enabled)) => {
                if let Some(reminder) =
                    self.config.reminders.iter_mut().find(|reminder| reminder.id == id)
                {
                    reminder.enabled = enabled;
                }
                self.persist();
            }
            None => {}
        }

        match settings_action {
            Some(SettingsAction::Save) => self.persist(),
            Some(SettingsAction::StartWithWindows(start)) => {
                if startup::set(start) {
                    self.config.start_with_windows = start;
                    self.persist();
                }
            }
            Some(SettingsAction::TestSound) => {
                sound::play_at(&Sound::Water, self.config.alert_volume);
            }
            Some(SettingsAction::OpenFolder) => {
                let _ = std::process::Command::new("explorer").arg(storage::data_dir()).spawn();
            }
            Some(SettingsAction::ClearHistory) => {
                storage::clear_history();
                self.history.clear();
                self.confirming_clear = false;
            }
            None => {}
        }

        match editor_action {
            Some(EditorAction::Save) => self.save_draft(),
            Some(EditorAction::Delete) => self.delete_selected(),
            Some(EditorAction::Back) => self.go_home(),
            None => {}
        }
    }
}
