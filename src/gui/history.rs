// The history tab: how many times each reminder fired lately, and the latest firings.

use std::collections::BTreeMap;

use chrono::{Duration, NaiveDate};
use eframe::egui::{self, RichText};

use super::{theme, widgets::card};
use crate::shared::i18n::t;
use crate::shared::storage::HistoryEntry;

pub fn show(ui: &mut egui::Ui, history: &[HistoryEntry], today: NaiveDate) {
    egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
        if history.is_empty() {
            ui.label(RichText::new(t("Nothing has fired yet.")).color(theme::MUTED));
            return;
        }

        card(ui, "Last 7 days", |ui| {
            let first_day = today - Duration::days(6);
            let mut per_reminder: BTreeMap<&str, [u32; 7]> = BTreeMap::new();
            for entry in history {
                let day = entry.at.date();
                if day >= first_day && day <= today {
                    let index = (day - first_day).num_days() as usize;
                    per_reminder.entry(&entry.name).or_insert([0; 7])[index] += 1;
                }
            }
            if per_reminder.is_empty() {
                ui.label(RichText::new(t("Nothing in the last 7 days.")).color(theme::MUTED));
            }
            for (name, counts) in per_reminder {
                ui.horizontal(|ui| {
                    ui.label(theme::semibold(name, 14.0));
                    let text: Vec<String> = counts.iter().map(|count| count.to_string()).collect();
                    let total = t("(total {total})").replace("{total}", &counts.iter().sum::<u32>().to_string());
                    ui.label(format!("{}  {total}", text.join(" · ")));
                });
            }
            ui.label(
                RichText::new(t("days from {day} to today").replace("{day}", &first_day.format("%d/%m").to_string()))
                    .small()
                    .color(theme::MUTED),
            );
        });

        ui.add_space(8.0);
        card(ui, "Latest", |ui| {
            for entry in history.iter().rev().take(200) {
                ui.label(format!("{}   {}", entry.at.format("%d/%m/%Y %H:%M:%S"), entry.name));
            }
        });
    });
}
