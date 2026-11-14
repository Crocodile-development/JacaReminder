// The settings page: sound, pausing, startup and the saved data.

use eframe::egui::{self, RichText, Slider};

use super::widgets::{card, segmented, switch};
use super::{nav, theme};
use crate::shared::i18n::{self, Language, t};
use crate::shared::reminder::Config;
use crate::shared::storage;

pub enum SettingsAction {
    /// The configuration changed and has to be saved.
    Save,
    StartWithWindows(bool),
    TestSound,
    OpenFolder,
    ClearHistory,
}

#[derive(PartialEq, Clone, Copy)]
pub enum Category {
    General,
    Sound,
    Data,
}

impl Category {
    const ALL: [Category; 3] = [Category::General, Category::Sound, Category::Data];

    fn label(self) -> &'static str {
        match self {
            Category::General => t("General"),
            Category::Sound => t("Sound"),
            Category::Data => t("Data"),
        }
    }
}

/// What was clicked in the settings sidebar.
pub enum NavAction {
    Back,
    Select(Category),
}

/// The settings sidebar: a way back and the categories.
pub fn show_nav(ui: &mut egui::Ui, active: Category) -> Option<NavAction> {
    let mut action = None;
    ui.spacing_mut().item_spacing.y = 6.0;
    ui.add_space(10.0);
    ui.horizontal(|ui| {
        ui.add_space(4.0);
        if nav::back_button(ui).clicked() {
            action = Some(NavAction::Back);
        }
        ui.label(theme::bold(t("Settings"), 16.0));
    });
    ui.add_space(14.0);
    nav::slide_pill(ui, "categories", Category::ALL.iter().position(|item| *item == active).unwrap_or(0));
    for category in Category::ALL {
        if nav::text_item(ui, category.label(), category == active).clicked() {
            action = Some(NavAction::Select(category));
        }
    }
    action
}

pub fn show(
    ui: &mut egui::Ui,
    category: Category,
    config: &mut Config,
    history_count: usize,
    confirming_clear: &mut bool,
) -> Option<SettingsAction> {
    let mut action = None;
    ui.add_space(8.0);
    ui.label(theme::bold(category.label(), 30.0));
    ui.add_space(6.0);

    egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| match category {
        Category::Sound => {
        card(ui, "Sound", |ui| {
            row(
                ui,
                "Alert volume",
                "Applies to the water drop and to your own .wav files. The Windows beep follows the system volume.",
                |ui| {
                    if super::widgets::button(ui, "Test").clicked() {
                        action = Some(SettingsAction::TestSound);
                    }
                    // The value is a label of its own: the box that comes with the slider sits lower
                    // than the rail and the button.
                    ui.add_sized(
                        [44.0, 30.0],
                        egui::Label::new(theme::semibold(format!("{}%", config.alert_volume), 14.0)),
                    );
                    let slider = ui.add(Slider::new(&mut config.alert_volume, 0..=100).show_value(false));
                    if slider.drag_stopped() || (slider.changed() && !slider.dragged()) {
                        action = Some(SettingsAction::Save);
                    }
                },
            );
        });
        }
        Category::General => {
        card(ui, "Language", |ui| {
            row(ui, "App language", "Changes the language of the whole program.", |ui| {
                let before = config.language;
                segmented(
                    ui,
                    "language",
                    &mut config.language,
                    &[(Language::English, "English"), (Language::Portuguese, "Português")],
                );
                if config.language != before {
                    i18n::set(config.language);
                    action = Some(SettingsAction::Save);
                }
            });
        });
        ui.add_space(10.0);

        card(ui, "Reminders", |ui| {
            row(
                ui,
                "Pause all reminders",
                "No popup or sound while this is on. The schedules keep going, so reminders come back on their own times.",
                |ui| {
                    if switch(ui, &mut config.pause_all) {
                        action = Some(SettingsAction::Save);
                    }
                },
            );
        });
        ui.add_space(10.0);

        card(ui, "Startup", |ui| {
            row(
                ui,
                "Start with Windows",
                "Reminders keep working after you restart the computer.",
                |ui| {
                    let mut start = config.start_with_windows;
                    if switch(ui, &mut start) {
                        action = Some(SettingsAction::StartWithWindows(start));
                    }
                },
            );
        });
        }
        Category::Data => {
        card(ui, "Data", |ui| {
            row(
                ui,
                "History",
                &match history_count {
                    0 => t("Nothing has fired yet.").to_string(),
                    1 => t("1 reminder fired so far.").to_string(),
                    count => t("{count} reminders fired so far.").replace("{count}", &count.to_string()),
                },
                |ui| {
                    let label = if *confirming_clear { "Click again to clear" } else { "Clear history" };
                    let enabled = history_count > 0;
                    if super::widgets::button_enabled(ui, label, enabled).clicked() {
                        if *confirming_clear {
                            action = Some(SettingsAction::ClearHistory);
                        } else {
                            *confirming_clear = true;
                        }
                    }
                },
            );
            ui.add_space(6.0);
            row(ui, "Data folder", &storage::data_dir().display().to_string(), |ui| {
                if super::widgets::button(ui, "Open folder").clicked() {
                    action = Some(SettingsAction::OpenFolder);
                }
            });
        });
        }
    });

    action
}

/// A setting: its name and a line of help on the left, the control on the right.
fn row(ui: &mut egui::Ui, title: &str, help: &str, control: impl FnOnce(&mut egui::Ui)) {
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.set_max_width((ui.available_width() - 260.0).max(180.0));
            ui.spacing_mut().item_spacing.y = 2.0;
            ui.label(theme::semibold(t(title), 15.0));
            ui.label(RichText::new(t(help)).color(theme::MUTED).small());
        });
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), control);
    });
}
