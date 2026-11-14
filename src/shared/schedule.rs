// When a reminder fires.

use chrono::{Datelike, Duration, NaiveDate, NaiveDateTime, NaiveTime};
use serde::{Deserialize, Serialize};

use super::i18n::{t, weekday_names};

#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
pub enum Schedule {
    /// Every `minutes` minutes. With `window`, only between the two times of the day.
    Interval {
        minutes: u32,
        window: Option<(NaiveTime, NaiveTime)>,
    },
    /// Days chosen by rules, at the given times.
    Calendar(Calendar),
    /// A single moment.
    Once { at: NaiveDateTime },
}

/// Every list is a filter: empty means "any". A day must pass all the filters.
#[derive(Clone, PartialEq, Serialize, Deserialize, Debug, Default)]
pub struct Calendar {
    pub times: Vec<NaiveTime>,
    /// 0 = Monday ... 6 = Sunday.
    pub weekdays: Vec<u8>,
    /// 1..=31.
    pub month_days: Vec<u8>,
    /// Also accept the last day of each month, whatever its number is.
    pub last_day_of_month: bool,
    /// 1..=12.
    pub months: Vec<u8>,
    pub years: Vec<i32>,
    /// First and last day (inclusive) of a sequence of days.
    pub from: Option<NaiveDate>,
    pub to: Option<NaiveDate>,
}

/// How far ahead a calendar is searched. Beyond this a rule is treated as "never".
const SEARCH_DAYS: i64 = 366 * 12;

impl Calendar {
    pub fn matches_day(&self, day: NaiveDate) -> bool {
        let weekday = day.weekday().num_days_from_monday() as u8;
        let day_number = day.day() as u8;
        let is_last_day = (day + Duration::days(1)).month() != day.month();

        let day_of_month_ok = if self.month_days.is_empty() && !self.last_day_of_month {
            true
        } else {
            self.month_days.contains(&day_number) || (self.last_day_of_month && is_last_day)
        };

        (self.weekdays.is_empty() || self.weekdays.contains(&weekday))
            && day_of_month_ok
            && (self.months.is_empty() || self.months.contains(&(day.month() as u8)))
            && (self.years.is_empty() || self.years.contains(&day.year()))
            && self.from.is_none_or(|from| day >= from)
            && self.to.is_none_or(|to| day <= to)
    }

    fn next_after(&self, after: NaiveDateTime) -> Option<NaiveDateTime> {
        let mut times = self.times.clone();
        times.sort();
        for offset in 0..SEARCH_DAYS {
            let day = after.date() + Duration::days(offset);
            if !self.matches_day(day) {
                continue;
            }
            if let Some(time) = times.iter().find(|time| day.and_time(**time) > after) {
                return Some(day.and_time(*time));
            }
        }
        None
    }
}

impl Schedule {
    /// The first moment strictly after `after`. For an interval, `after` is when the countdown
    /// started (the last firing). `None` means it never fires again.
    pub fn next_after(&self, after: NaiveDateTime) -> Option<NaiveDateTime> {
        match self {
            Schedule::Interval { minutes, window } => {
                let next = after + Duration::minutes((*minutes).max(1) as i64);
                Some(match window {
                    Some((start, end)) => clamp_to_window(next, *start, *end),
                    None => next,
                })
            }
            Schedule::Calendar(calendar) => calendar.next_after(after),
            Schedule::Once { at } => (*at > after).then_some(*at),
        }
    }

    /// A short text for the list of reminders.
    pub fn summary(&self) -> String {
        match self {
            Schedule::Interval { minutes, window } => {
                let every = if minutes % 60 == 0 {
                    t("Every {hours}h").replace("{hours}", &(minutes / 60).to_string())
                } else if *minutes > 60 {
                    t("Every {hours}h{minutes}")
                        .replace("{hours}", &(minutes / 60).to_string())
                        .replace("{minutes}", &format!("{:02}", minutes % 60))
                } else {
                    t("Every {minutes} min").replace("{minutes}", &minutes.to_string())
                };
                match window {
                    Some((start, end)) => t("{every}, {start} to {end}")
                        .replace("{every}", &every)
                        .replace("{start}", &start.format("%H:%M").to_string())
                        .replace("{end}", &end.format("%H:%M").to_string()),
                    None => every,
                }
            }
            Schedule::Calendar(calendar) => {
                let times: Vec<String> =
                    calendar.times.iter().map(|t| t.format("%H:%M").to_string()).collect();
                t("{days} at {times}")
                    .replace("{days}", &calendar_days_summary(calendar))
                    .replace("{times}", &times.join(", "))
            }
            Schedule::Once { at } => {
                t("Once, {when}").replace("{when}", &at.format("%d/%m/%Y %H:%M").to_string())
            }
        }
    }
}

fn calendar_days_summary(calendar: &Calendar) -> String {
    let mut parts = Vec::new();
    if !calendar.weekdays.is_empty() {
        let weekday_names = weekday_names();
        let names: Vec<&str> = calendar
            .weekdays
            .iter()
            .filter_map(|day| weekday_names.get(*day as usize).copied())
            .collect();
        parts.push(names.join(" "));
    }
    if !calendar.month_days.is_empty() || calendar.last_day_of_month {
        let mut days: Vec<String> = calendar.month_days.iter().map(|d| d.to_string()).collect();
        if calendar.last_day_of_month {
            days.push(t("last").into());
        }
        parts.push(t("day {days}").replace("{days}", &days.join(",")));
    }
    if !calendar.months.is_empty() {
        let months: Vec<String> = calendar.months.iter().map(|m| m.to_string()).collect();
        parts.push(t("month {months}").replace("{months}", &months.join(",")));
    }
    if !calendar.years.is_empty() {
        let years: Vec<String> = calendar.years.iter().map(|y| y.to_string()).collect();
        parts.push(t("year {years}").replace("{years}", &years.join(",")));
    }
    match (calendar.from, calendar.to) {
        (Some(from), Some(to)) => parts.push(
            t("{from} to {to}")
                .replace("{from}", &from.format("%d/%m/%Y").to_string())
                .replace("{to}", &to.format("%d/%m/%Y").to_string()),
        ),
        (Some(from), None) => {
            parts.push(t("from {from}").replace("{from}", &from.format("%d/%m/%Y").to_string()))
        }
        (None, Some(to)) => {
            parts.push(t("until {to}").replace("{to}", &to.format("%d/%m/%Y").to_string()))
        }
        (None, None) => {}
    }
    if parts.is_empty() { t("Every day").into() } else { parts.join(", ") }
}

/// Moves a moment that falls outside the daily window to the start of the next window.
/// A window whose end is before its start crosses midnight (for example 22:00 to 06:00).
fn clamp_to_window(moment: NaiveDateTime, start: NaiveTime, end: NaiveTime) -> NaiveDateTime {
    let time = moment.time();
    let inside = if start <= end {
        time >= start && time <= end
    } else {
        time >= start || time <= end
    };
    if inside {
        moment
    } else if time < start {
        moment.date().and_time(start)
    } else {
        (moment.date() + Duration::days(1)).and_time(start)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }
    fn time(h: u32, m: u32) -> NaiveTime {
        NaiveTime::from_hms_opt(h, m, 0).unwrap()
    }
    fn at(y: i32, mo: u32, d: u32, h: u32, mi: u32) -> NaiveDateTime {
        date(y, mo, d).and_time(time(h, mi))
    }

    #[test]
    fn interval_adds_minutes() {
        let schedule = Schedule::Interval { minutes: 90, window: None };
        assert_eq!(schedule.next_after(at(2026, 11, 13, 10, 0)), Some(at(2026, 11, 13, 11, 30)));
    }

    #[test]
    fn interval_waits_for_the_window() {
        let schedule = Schedule::Interval { minutes: 60, window: Some((time(9, 0), time(18, 0))) };
        assert_eq!(schedule.next_after(at(2026, 11, 13, 17, 30)), Some(at(2026, 11, 14, 9, 0)));
        assert_eq!(schedule.next_after(at(2026, 11, 13, 6, 0)), Some(at(2026, 11, 13, 9, 0)));
    }

    #[test]
    fn interval_window_across_midnight() {
        let schedule = Schedule::Interval { minutes: 60, window: Some((time(22, 0), time(6, 0))) };
        assert_eq!(schedule.next_after(at(2026, 11, 13, 23, 30)), Some(at(2026, 11, 14, 0, 30)));
        assert_eq!(schedule.next_after(at(2026, 11, 13, 10, 0)), Some(at(2026, 11, 13, 22, 0)));
    }

    #[test]
    fn every_day_at_two_times() {
        let calendar = Calendar { times: vec![time(18, 0), time(8, 0)], ..Default::default() };
        let schedule = Schedule::Calendar(calendar);
        assert_eq!(schedule.next_after(at(2026, 11, 13, 7, 0)), Some(at(2026, 11, 13, 8, 0)));
        assert_eq!(schedule.next_after(at(2026, 11, 13, 8, 0)), Some(at(2026, 11, 13, 18, 0)));
        assert_eq!(schedule.next_after(at(2026, 11, 13, 18, 0)), Some(at(2026, 11, 14, 8, 0)));
    }

    #[test]
    fn weekdays_only() {
        // 2026-11-13 is a Friday.
        let calendar =
            Calendar { times: vec![time(9, 0)], weekdays: vec![0, 2], ..Default::default() };
        let schedule = Schedule::Calendar(calendar);
        assert_eq!(schedule.next_after(at(2026, 11, 13, 10, 0)), Some(at(2026, 11, 16, 9, 0)));
    }

    #[test]
    fn day_of_month_skips_short_months() {
        let calendar =
            Calendar { times: vec![time(9, 0)], month_days: vec![31], ..Default::default() };
        let schedule = Schedule::Calendar(calendar);
        // November has no 31st: next is December 31.
        assert_eq!(schedule.next_after(at(2026, 11, 1, 0, 0)), Some(at(2026, 12, 31, 9, 0)));
    }

    #[test]
    fn last_day_of_month() {
        let calendar =
            Calendar { times: vec![time(9, 0)], last_day_of_month: true, ..Default::default() };
        let schedule = Schedule::Calendar(calendar);
        assert_eq!(schedule.next_after(at(2026, 2, 1, 0, 0)), Some(at(2026, 2, 28, 9, 0)));
        assert_eq!(schedule.next_after(at(2028, 2, 1, 0, 0)), Some(at(2028, 2, 29, 9, 0)));
    }

    #[test]
    fn one_day_of_one_month_of_one_year() {
        let calendar = Calendar {
            times: vec![time(8, 0)],
            month_days: vec![15],
            months: vec![3],
            years: vec![2027],
            ..Default::default()
        };
        let schedule = Schedule::Calendar(calendar);
        assert_eq!(schedule.next_after(at(2026, 11, 13, 0, 0)), Some(at(2027, 3, 15, 8, 0)));
        assert_eq!(schedule.next_after(at(2027, 3, 15, 8, 0)), None);
    }

    #[test]
    fn sequence_of_days() {
        let calendar = Calendar {
            times: vec![time(9, 0)],
            from: Some(date(2026, 12, 1)),
            to: Some(date(2026, 12, 3)),
            ..Default::default()
        };
        let schedule = Schedule::Calendar(calendar);
        assert_eq!(schedule.next_after(at(2026, 11, 13, 0, 0)), Some(at(2026, 12, 1, 9, 0)));
        assert_eq!(schedule.next_after(at(2026, 12, 3, 9, 0)), None);
    }

    #[test]
    fn once_fires_only_in_the_future() {
        let schedule = Schedule::Once { at: at(2026, 11, 20, 15, 0) };
        assert_eq!(schedule.next_after(at(2026, 11, 13, 0, 0)), Some(at(2026, 11, 20, 15, 0)));
        assert_eq!(schedule.next_after(at(2026, 11, 20, 15, 0)), None);
    }
}
