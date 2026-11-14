// The schedule part of the reminder editor.

use chrono::{Datelike, Local, NaiveDate, NaiveTime};
use eframe::egui::{self, DragValue, RichText};

use super::widgets::{button, card, date_edit, segmented, time_edit, toggle_row};
use super::theme;
use crate::shared::i18n::{month_names, t, weekday_names};
use crate::shared::schedule::{Calendar, Schedule};

const DAY_LABELS: [&str; 31] = [
    "1", "2", "3", "4", "5", "6", "7", "8", "9", "10", "11", "12", "13", "14", "15", "16", "17",
    "18", "19", "20", "21", "22", "23", "24", "25", "26", "27", "28", "29", "30", "31",
];

/// Width of the name on the left of a row, so all the controls start in the same column.
const LABEL_WIDTH: f32 = 80.0;
const ROW_HEIGHT: f32 = 32.0;

#[derive(PartialEq, Clone, Copy)]
enum Kind {
    Interval,
    Calendar,
    Once,
}

impl Kind {
    fn of(schedule: &Schedule) -> Kind {
        match schedule {
            Schedule::Interval { .. } => Kind::Interval,
            Schedule::Calendar(_) => Kind::Calendar,
            Schedule::Once { .. } => Kind::Once,
        }
    }

    fn default_schedule(self) -> Schedule {
        match self {
            Kind::Interval => Schedule::Interval { minutes: 60, window: None },
            Kind::Calendar => Schedule::Calendar(Calendar {
                times: vec![NaiveTime::from_hms_opt(9, 0, 0).unwrap_or_default()],
                ..Calendar::default()
            }),
            Kind::Once => Schedule::Once {
                at: (Local::now() + chrono::Duration::hours(1)).naive_local().with_second_zero(),
            },
        }
    }
}

trait WithSecondZero {
    fn with_second_zero(self) -> Self;
}

impl WithSecondZero for chrono::NaiveDateTime {
    fn with_second_zero(self) -> Self {
        use chrono::Timelike;
        self.with_second(0).and_then(|value| value.with_nanosecond(0)).unwrap_or(self)
    }
}

/// A row: the name of the setting on the left, its controls in the next column.
pub(super) fn labeled(ui: &mut egui::Ui, label: &str, controls: impl FnOnce(&mut egui::Ui)) {
    labeled_width(ui, LABEL_WIDTH, label, controls);
}

/// Like `labeled`, with a wider column for the names.
pub(super) fn labeled_width(ui: &mut egui::Ui, width: f32, label: &str, controls: impl FnOnce(&mut egui::Ui)) {
    ui.horizontal(|ui| {
        ui.set_min_height(ROW_HEIGHT);
        label_cell(ui, width, ROW_HEIGHT, label);
        controls(ui);
    });
}

/// The name of a setting in a column of exactly `width`, so the controls of every row start at
/// the same place whatever the length of the name.
pub(super) fn label_cell(ui: &mut egui::Ui, width: f32, height: f32, label: &str) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::hover());
    let font = egui::TextStyle::Body.resolve(ui.style());
    ui.painter().text(rect.left_center(), egui::Align2::LEFT_CENTER, t(label), font, theme::MUTED);
}

/// A small title that opens a group of rows.
fn heading(ui: &mut egui::Ui, text: &str) {
    ui.add_space(8.0);
    ui.label(theme::semibold(t(text), 13.5).color(theme::ACCENT_TEXT));
    ui.add_space(2.0);
}

pub fn show(ui: &mut egui::Ui, schedule: &mut Schedule) {
    card(ui, "Repeat", |ui| {
        let current = Kind::of(schedule);
        let mut chosen = current;
        segmented(
            ui,
            "schedule_kind",
            &mut chosen,
            &[(Kind::Once, "One time"), (Kind::Interval, "Times"), (Kind::Calendar, "Days and times")],
        );
        if chosen != current {
            *schedule = chosen.default_schedule();
        }
        ui.add_space(6.0);

        match schedule {
            Schedule::Interval { minutes, window } => interval(ui, minutes, window),
            Schedule::Calendar(calendar) => calendar_editor(ui, calendar),
            Schedule::Once { at } => {
                let mut date = at.date();
                let mut time = at.time();
                let mut changed = false;
                labeled(ui, "Date", |ui| changed |= date_edit(ui, &mut date));
                labeled(ui, "Time", |ui| changed |= time_edit(ui, &mut time));
                if changed {
                    *at = date.and_time(time);
                }
            }
        }

        let next = schedule.next_after(Local::now().naive_local());
        ui.add_space(10.0);
        ui.label(
            RichText::new(match next {
                Some(next) => {
                    let weekday = weekday_names()[next.weekday().num_days_from_monday() as usize];
                    let when = format!("{weekday} {}", next.format("%d/%m/%Y %H:%M"));
                    t("Next: {time}").replace("{time}", &when)
                }
                None => t("Next: never (nothing left in the future)").to_string(),
            })
            .color(theme::MUTED),
        );
    });
}

fn interval(ui: &mut egui::Ui, minutes: &mut u32, window: &mut Option<(NaiveTime, NaiveTime)>) {
    labeled(ui, "Every", |ui| {
        let mut hours = *minutes / 60;
        let mut rest = *minutes % 60;
        let changed = ui.add(DragValue::new(&mut hours).range(0..=999).suffix(" h")).changed()
            | ui.add(DragValue::new(&mut rest).range(0..=59).suffix(" min")).changed();
        if changed {
            *minutes = (hours * 60 + rest).max(1);
        }
    });
    labeled(ui, "Quick", |ui| {
        for (label, value) in [("30 min", 30), ("1 h", 60), ("1h30", 90), ("2 h", 120)] {
            if button(ui, label).clicked() {
                *minutes = value;
            }
        }
    });
    labeled(ui, "Hours", |ui| {
        let mut limited = window.is_some();
        if ui.checkbox(&mut limited, t("Only between")).changed() {
            *window = limited.then(|| {
                (
                    NaiveTime::from_hms_opt(9, 0, 0).unwrap_or_default(),
                    NaiveTime::from_hms_opt(18, 0, 0).unwrap_or_default(),
                )
            });
        }
        if let Some((start, end)) = window {
            time_edit(ui, start);
            ui.label(t("and"));
            time_edit(ui, end);
        }
    });
}

fn calendar_editor(ui: &mut egui::Ui, calendar: &mut Calendar) {
    heading(ui, "Times of the day");
    let mut remove = None;
    let many = calendar.times.len() > 1;
    for (index, time) in calendar.times.iter_mut().enumerate() {
        ui.horizontal(|ui| {
            ui.set_min_height(ROW_HEIGHT);
            time_edit(ui, time);
            if many && button(ui, "Remove").clicked() {
                remove = Some(index);
            }
        });
    }
    if let Some(index) = remove {
        calendar.times.remove(index);
    }
    if button(ui, "+ Add time").clicked() {
        let last = calendar.times.last().copied().unwrap_or_default();
        calendar.times.push(last);
    }

    heading(ui, "Days");
    let current = Preset::of(calendar);
    let mut chosen = current;
    segmented(
        ui,
        "day_preset",
        &mut chosen,
        &[
            (Preset::EveryDay, "Every day"),
            (Preset::MonFri, "Mon-Fri"),
            (Preset::Weekend, "Weekend"),
            (Preset::LastDay, "Last day of the month"),
        ],
    );
    if chosen != current {
        clear_day_filters(calendar);
        match chosen {
            Preset::MonFri => calendar.weekdays = vec![0, 1, 2, 3, 4],
            Preset::Weekend => calendar.weekdays = vec![5, 6],
            Preset::LastDay => calendar.last_day_of_month = true,
            Preset::EveryDay | Preset::Custom => {}
        }
    }
    ui.add_space(4.0);
    ui.label(RichText::new(t("Days of the week (none selected = every day)")).color(theme::MUTED));
    toggle_row(ui, &weekday_names(), 0, &mut calendar.weekdays);

    let has_more = !calendar.month_days.is_empty()
        || !calendar.months.is_empty()
        || !calendar.years.is_empty()
        || calendar.last_day_of_month
        || calendar.from.is_some()
        || calendar.to.is_some();
    ui.add_space(8.0);
    egui::CollapsingHeader::new(theme::semibold(t("More filters"), 13.5).color(theme::ACCENT_TEXT))
        .default_open(has_more)
        .show(ui, |ui| {
            ui.label(RichText::new(t("Days of the month (none selected = every day)")).color(theme::MUTED));
            toggle_row(ui, &DAY_LABELS, 1, &mut calendar.month_days);
            ui.checkbox(&mut calendar.last_day_of_month, t("Also the last day of each month"));

            ui.add_space(6.0);
            ui.label(RichText::new(t("Months (none selected = every month)")).color(theme::MUTED));
            toggle_row(ui, &month_names(), 1, &mut calendar.months);

            ui.add_space(6.0);
            years_row(ui, &mut calendar.years);
            ui.add_space(6.0);
            range_row(ui, "Starting on", &mut calendar.from);
            range_row(ui, "Until", &mut calendar.to);
        });
}

/// The day shortcuts. `Custom` is when the chosen days match none of them.
#[derive(PartialEq, Clone, Copy)]
enum Preset {
    EveryDay,
    MonFri,
    Weekend,
    LastDay,
    Custom,
}

impl Preset {
    fn of(calendar: &Calendar) -> Preset {
        if !calendar.month_days.is_empty() {
            return Preset::Custom;
        }
        match (calendar.weekdays.as_slice(), calendar.last_day_of_month) {
            ([], false) => Preset::EveryDay,
            ([0, 1, 2, 3, 4], false) => Preset::MonFri,
            ([5, 6], false) => Preset::Weekend,
            ([], true) => Preset::LastDay,
            _ => Preset::Custom,
        }
    }
}

/// How many years ahead can be picked.
const YEARS_AHEAD: i32 = 50;

/// The chosen years as chips (click one to remove it) and a list to add more, from this year on.
fn years_row(ui: &mut egui::Ui, years: &mut Vec<i32>) {
    ui.label(RichText::new(t("Years (none selected = every year)")).color(theme::MUTED));
    let this_year = Local::now().year();
    ui.horizontal_wrapped(|ui| {
        let mut remove = None;
        for year in years.iter() {
            if button(ui, &format!("{year}  ×")).clicked() {
                remove = Some(*year);
            }
        }
        if let Some(year) = remove {
            years.retain(|item| *item != year);
        }
        egui::ComboBox::from_id_salt("add_year").selected_text(t("Add year")).show_ui(ui, |ui| {
            egui::ScrollArea::vertical().max_height(220.0).show(ui, |ui| {
                for year in this_year..=this_year + YEARS_AHEAD {
                    if !years.contains(&year) && ui.selectable_label(false, year.to_string()).clicked() {
                        years.push(year);
                        years.sort();
                    }
                }
            });
        });
    });
}

/// Only the day filters: the months, years and date range are chosen apart from the shortcuts.
fn clear_day_filters(calendar: &mut Calendar) {
    calendar.weekdays.clear();
    calendar.month_days.clear();
    calendar.last_day_of_month = false;
}

fn range_row(ui: &mut egui::Ui, label: &str, value: &mut Option<NaiveDate>) {
    labeled(ui, label, |ui| {
        let mut on = value.is_some();
        if ui.checkbox(&mut on, "").changed() {
            *value = on.then(|| Local::now().date_naive());
        }
        if let Some(date) = value {
            date_edit(ui, date);
            ui.label(weekday_names()[date.weekday().num_days_from_monday() as usize]);
        }
    });
}
