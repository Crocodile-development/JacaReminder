// The popup part of the reminder editor.

use std::time::{Duration, Instant};

use eframe::egui::{self, ComboBox, DragValue, RichText};

use super::schedule_editor::{label_cell, labeled_width};
use super::theme;
use super::widgets::{Icon, button, card, corner_toggles, icon_button, segmented};
use crate::integration::{audio_import, daemon, sound};
use crate::shared::i18n::t;
use crate::shared::reminder::{Corner, ImageMode, PopupRequest, PopupStyle, Sound, TaskPopup};

/// Width of the column with the names of the settings.
const LABEL_WIDTH: f32 = 150.0;

/// A row: the name on the left, the controls next to it, everything centered on the same line.
fn row(ui: &mut egui::Ui, label: &str, controls: impl FnOnce(&mut egui::Ui)) {
    labeled_width(ui, LABEL_WIDTH, label, controls);
}

/// What the popup says.
pub fn show_text(ui: &mut egui::Ui, style: &mut PopupStyle) {
    card(ui, "Popup text", |ui| {
        row(ui, "Title", |ui| {
            ui.add(egui::TextEdit::singleline(&mut style.title).desired_width(300.0));
        });
        // The message has several lines, so its name stays at the top.
        ui.horizontal_top(|ui| {
            label_cell(ui, LABEL_WIDTH, 30.0, "Message");
            ui.add(egui::TextEdit::multiline(&mut style.message).desired_width(300.0).desired_rows(2));
        });
    });
}

/// How the popup looks and behaves, with a button to test it.
///
/// `snooze_minutes` is set when the reminder is a task: its popup stays until answered, so the
/// time on screen does not apply, and the test shows the buttons.
pub fn show_look(ui: &mut egui::Ui, style: &mut PopupStyle, snooze_minutes: Option<u32>) {
    card(ui, "Look", |ui| {
        row(ui, "Colors", |ui| {
            ui.color_edit_button_srgb(&mut style.background);
            ui.label(t("background"));
            ui.color_edit_button_srgb(&mut style.text);
            ui.label(t("text"));
            ui.color_edit_button_srgb(&mut style.accent);
            ui.label(t("bar"));
        });

        row(ui, "Position", |ui| {
            ComboBox::from_id_salt("corner").selected_text(style.corner.label()).show_ui(ui, |ui| {
                for corner in Corner::ALL {
                    ui.selectable_value(&mut style.corner, corner, corner.label());
                }
            });
        });

        row(ui, "Distance from edge", |ui| {
            ui.label(RichText::new("H").color(theme::MUTED)).on_hover_text(t("Horizontal"));
            ui.add(DragValue::new(&mut style.edge_distance).range(0..=800).suffix(" px"));
            ui.add_space(6.0);
            ui.label(RichText::new("V").color(theme::MUTED)).on_hover_text(t("Vertical"));
            ui.add(DragValue::new(&mut style.edge_distance_y).range(0..=800).suffix(" px"));
        });

        row(ui, "Size", |ui| {
            ui.add(DragValue::new(&mut style.width).range(160..=900).suffix(" px"));
            ui.label("x");
            ui.add(DragValue::new(&mut style.height).range(60..=600).suffix(" px"));
        });

        row(ui, "Rounded corners", |ui| {
            corner_toggles(ui, &mut style.round_corners);
        });

        row(ui, "Corner radius", |ui| {
            ui.add(DragValue::new(&mut style.rounding).range(0..=60).suffix(" px"));
        });

        row(ui, "Image", |ui| {
            let file_name = style
                .image
                .as_deref()
                .and_then(|path| std::path::Path::new(path).file_name())
                .map(|name| name.to_string_lossy().into_owned());
            if button(ui, "Browse").clicked() {
                let picked = rfd::FileDialog::new()
                    .add_filter(t("Image"), &["png", "jpg", "jpeg", "gif", "webp", "bmp"])
                    .pick_file();
                if let Some(file) = picked {
                    style.image = Some(file.display().to_string());
                }
            }
            if style.image.is_some() && button(ui, "Remove").clicked() {
                style.image = None;
            }
            ui.label(RichText::new(file_name.unwrap_or_else(|| t("None").to_string())).color(theme::MUTED));
        });

        if style.image.is_some() {
            row(ui, "Image style", |ui| {
                segmented(
                    ui,
                    "image_mode",
                    &mut style.image_mode,
                    &[(ImageMode::Icon, "Icon"), (ImageMode::Background, "Background")],
                );
            });
        }
    });
    ui.add_space(10.0);
    card(ui, "Behavior", |ui| {
        if snooze_minutes.is_none() {
            row(ui, "Stays for", |ui| {
                ui.add(DragValue::new(&mut style.seconds).range(0..=3600).suffix(" s"));
                ui.label(t("(0 = until clicked)"));
                ui.checkbox(&mut style.progress_bar, t("Progress bar"));
            });
        }

        row(ui, "Sound", |ui| sound_controls(ui, &mut style.sound));

        ui.add_space(6.0);
        if button(ui, "Test popup").clicked() {
            let task = snooze_minutes.map(|snooze_minutes| TaskPopup { id: 0, snooze_minutes });
            daemon::show_test(&PopupRequest { style: style.clone(), task });
        }
    });
}

fn sound_controls(ui: &mut egui::Ui, sound: &mut Sound) {
    let label = match sound {
        Sound::None => t("None"),
        Sound::Water => t("Water drop"),
        Sound::System => t("Windows beep"),
        Sound::File(_) => t("Your file"),
    };
    ComboBox::from_id_salt("sound").selected_text(label).show_ui(ui, |ui| {
        ui.selectable_value(sound, Sound::None, t("None"));
        ui.selectable_value(sound, Sound::Water, t("Water drop"));
        ui.selectable_value(sound, Sound::System, t("Windows beep"));
        if ui.selectable_label(matches!(sound, Sound::File(_)), t("Your file")).clicked()
            && !matches!(sound, Sound::File(_))
        {
            *sound = Sound::File(String::new());
        }
    });
    if let Sound::File(path) = sound {
        let error_id = egui::Id::unique("sound_import_error");
        let name = std::path::Path::new(path.as_str())
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| t("None").to_string());
        if button(ui, "Browse").clicked() {
            let picked = rfd::FileDialog::new()
                .add_filter(t("Sound"), &["wav", "mp3", "ogg", "oga", "flac", "m4a", "aac"])
                .pick_file();
            if let Some(file) = picked {
                // The popups play WAV, so the chosen file is converted now and the converted
                // copy is the one the reminder keeps.
                match audio_import::import_to_wav(&file) {
                    Ok(wav) => {
                        *path = wav.display().to_string();
                        ui.data_mut(|data| data.remove::<String>(error_id));
                    }
                    Err(message) => ui.data_mut(|data| {
                        data.insert_temp(error_id, message);
                    }),
                }
            }
        }
        ui.label(RichText::new(name).color(theme::MUTED));
        if let Some(message) = ui.data(|data| data.get_temp::<String>(error_id)) {
            ui.label(RichText::new(t(&message)).color(theme::DANGER));
        }
    }
    // The button only shows when there is a sound to play: not "None", and a file already chosen.
    let has_sound = match sound {
        Sound::None => false,
        Sound::File(path) => !path.is_empty(),
        Sound::Water | Sound::System => true,
    };
    if !has_sound {
        return;
    }
    // One button: Play, which becomes Stop while the sound is playing and goes back to Play when
    // the sound ends.
    let playing_id = egui::Id::unique("sound_test_ends_at");
    let ends_at = ui.data(|data| data.get_temp::<Instant>(playing_id));
    let playing = ends_at.is_some_and(|end| Instant::now() < end);
    if playing {
        ui.ctx().request_repaint_after(Duration::from_millis(100));
    }
    let (icon, tooltip) = if playing { (Icon::Stop, "Stop") } else { (Icon::Play, "Play") };
    if icon_button(ui, icon, tooltip).clicked() {
        if playing {
            sound::stop();
            ui.data_mut(|data| data.remove::<Instant>(playing_id));
        } else {
            sound::play(sound);
            let end = Instant::now() + sound::duration(sound);
            ui.data_mut(|data| data.insert_temp(playing_id, end));
        }
    }
}
