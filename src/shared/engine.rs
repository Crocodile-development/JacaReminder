// Keeps the next firing time of every reminder and says which ones are due.

use std::collections::HashMap;

use chrono::{Duration, NaiveDateTime};
use serde::{Deserialize, Serialize};

use super::reminder::{Config, Kind};
use super::schedule::Schedule;

struct Entry {
    /// The schedule the time below was computed from, to notice when the user changes it.
    schedule: Schedule,
    next: Option<NaiveDateTime>,
}

/// A task that fired and has not been completed yet.
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Debug)]
pub enum Waiting {
    /// Its popup is on the screen.
    Showing,
    /// It was snoozed and shows again at this time.
    SnoozedUntil(NaiveDateTime),
}

/// A popup that has to be shown now.
#[derive(PartialEq, Debug)]
pub struct Due {
    pub id: u64,
    /// True when it comes back after a snooze (it is not a new firing).
    pub repeat: bool,
}

#[derive(Default)]
pub struct Engine {
    entries: HashMap<u64, Entry>,
    pending: HashMap<u64, Waiting>,
}

impl Engine {
    /// Matches the entries to the configuration. A reminder whose schedule did not change keeps
    /// its countdown; a new or edited one starts counting from `now`.
    pub fn sync(&mut self, config: &Config, now: NaiveDateTime) {
        self.entries.retain(|id, _| {
            config.reminders.iter().any(|reminder| reminder.enabled && reminder.id == *id)
        });
        self.pending.retain(|id, _| {
            config
                .reminders
                .iter()
                .any(|reminder| reminder.enabled && reminder.kind == Kind::Task && reminder.id == *id)
        });
        for reminder in config.reminders.iter().filter(|reminder| reminder.enabled) {
            let unchanged = self
                .entries
                .get(&reminder.id)
                .is_some_and(|entry| entry.schedule == reminder.schedule);
            if !unchanged {
                self.entries.insert(
                    reminder.id,
                    Entry {
                        schedule: reminder.schedule.clone(),
                        next: reminder.schedule.next_after(now),
                    },
                );
            }
        }
    }

    /// The ids of the reminders that must fire now. Each one already gets its next time.
    pub fn take_due(&mut self, now: NaiveDateTime) -> Vec<u64> {
        let mut fired = Vec::new();
        for (id, entry) in self.entries.iter_mut() {
            if entry.next.is_some_and(|next| next <= now) {
                fired.push(*id);
                entry.next = entry.schedule.next_after(now);
            }
        }
        fired.sort();
        fired
    }

    pub fn next_times(&self) -> Vec<(u64, NaiveDateTime)> {
        let mut times: Vec<_> =
            self.entries.iter().filter_map(|(id, entry)| Some((*id, entry.next?))).collect();
        times.sort();
        times
    }

    /// The popups to show now: the reminders that are due, and the snoozed tasks whose time came.
    /// A task that is still waiting for an answer is not shown again by its own schedule, so the
    /// popups never pile up. Nothing is shown while everything is paused; a snoozed task then
    /// waits and comes back once the pause is over.
    pub fn due_popups(&mut self, config: &Config, now: NaiveDateTime) -> Vec<Due> {
        let mut shown = Vec::new();
        for id in self.take_due(now) {
            if config.pause_all {
                continue;
            }
            let Some(reminder) = config.reminders.iter().find(|reminder| reminder.id == id) else {
                continue;
            };
            if reminder.kind == Kind::Task {
                if self.pending.contains_key(&id) {
                    continue;
                }
                self.pending.insert(id, Waiting::Showing);
            }
            shown.push(Due { id, repeat: false });
        }
        if !config.pause_all {
            let mut back: Vec<u64> = self
                .pending
                .iter()
                .filter_map(|(id, waiting)| match waiting {
                    Waiting::SnoozedUntil(time) if *time <= now => Some(*id),
                    _ => None,
                })
                .collect();
            back.sort();
            for id in back {
                self.pending.insert(id, Waiting::Showing);
                shown.push(Due { id, repeat: true });
            }
        }
        shown
    }

    /// The task shows again in `minutes`. Ignored if the task is not waiting for an answer.
    pub fn snooze(&mut self, id: u64, now: NaiveDateTime, minutes: u32) {
        if self.pending.contains_key(&id) {
            let until = now + Duration::minutes(minutes.max(1) as i64);
            self.pending.insert(id, Waiting::SnoozedUntil(until));
        }
    }

    /// The task is done: it only fires again at its next scheduled time.
    pub fn complete(&mut self, id: u64) {
        self.pending.remove(&id);
    }

    pub fn pending_list(&self) -> Vec<(u64, Waiting)> {
        let mut list: Vec<_> = self.pending.iter().map(|(id, waiting)| (*id, *waiting)).collect();
        list.sort_by_key(|(id, _)| *id);
        list
    }

    /// Takes back the tasks that were waiting when the daemon stopped. A popup that was on the
    /// screen is gone with the stop, so it comes back at once.
    pub fn restore_pending(&mut self, saved: Vec<(u64, Waiting)>, now: NaiveDateTime) {
        for (id, waiting) in saved {
            let waiting = match waiting {
                Waiting::Showing => Waiting::SnoozedUntil(now),
                snoozed => snoozed,
            };
            self.pending.insert(id, waiting);
        }
    }
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;

    use super::*;
    use crate::shared::reminder::Config;

    fn at(h: u32, m: u32, s: u32) -> NaiveDateTime {
        NaiveDate::from_ymd_opt(2026, 11, 13).unwrap().and_hms_opt(h, m, s).unwrap()
    }

    #[test]
    fn fires_once_and_schedules_the_next() {
        let config = Config::first_run();
        let mut engine = Engine::default();
        engine.sync(&config, at(10, 0, 0));

        assert!(engine.take_due(at(10, 59, 59)).is_empty());
        assert_eq!(engine.take_due(at(11, 0, 0)), vec![1]);
        assert!(engine.take_due(at(11, 0, 1)).is_empty());
        assert_eq!(engine.next_times(), vec![(1, at(12, 0, 0))]);
    }

    #[test]
    fn sync_keeps_the_countdown_when_nothing_changed() {
        let config = Config::first_run();
        let mut engine = Engine::default();
        engine.sync(&config, at(10, 0, 0));
        engine.sync(&config, at(10, 30, 0));
        assert_eq!(engine.next_times(), vec![(1, at(11, 0, 0))]);
    }

    fn task_config() -> Config {
        let mut config = Config::first_run();
        config.reminders[0].kind = Kind::Task;
        config.reminders[0].snooze_minutes = 10;
        config
    }

    #[test]
    fn a_task_snoozed_comes_back_until_it_is_completed() {
        let config = task_config();
        let mut engine = Engine::default();
        engine.sync(&config, at(10, 0, 0));

        assert_eq!(engine.due_popups(&config, at(11, 0, 0)), vec![Due { id: 1, repeat: false }]);
        engine.snooze(1, at(11, 0, 5), 10);
        assert!(engine.due_popups(&config, at(11, 9, 0)).is_empty());
        assert_eq!(engine.due_popups(&config, at(11, 10, 5)), vec![Due { id: 1, repeat: true }]);
        engine.snooze(1, at(11, 10, 9), 10);
        assert_eq!(engine.due_popups(&config, at(11, 20, 9)), vec![Due { id: 1, repeat: true }]);

        engine.complete(1);
        assert!(engine.due_popups(&config, at(11, 40, 0)).is_empty());
        assert!(engine.pending_list().is_empty());
        // The next scheduled time still comes.
        assert_eq!(engine.due_popups(&config, at(12, 0, 0)), vec![Due { id: 1, repeat: false }]);
    }

    #[test]
    fn a_waiting_task_is_not_shown_again_by_its_schedule() {
        let config = task_config();
        let mut engine = Engine::default();
        engine.sync(&config, at(10, 0, 0));

        assert_eq!(engine.due_popups(&config, at(11, 0, 0)).len(), 1);
        // The popup is still waiting for an answer when the next hour comes.
        assert!(engine.due_popups(&config, at(12, 0, 0)).is_empty());
    }

    #[test]
    fn pause_all_shows_nothing_and_keeps_a_snoozed_task() {
        let mut config = task_config();
        let mut engine = Engine::default();
        engine.sync(&config, at(10, 0, 0));
        assert_eq!(engine.due_popups(&config, at(11, 0, 0)).len(), 1);
        engine.snooze(1, at(11, 0, 5), 10);

        config.pause_all = true;
        assert!(engine.due_popups(&config, at(11, 30, 0)).is_empty());
        config.pause_all = false;
        assert_eq!(engine.due_popups(&config, at(11, 30, 1)), vec![Due { id: 1, repeat: true }]);
    }

    #[test]
    fn an_alert_never_waits_for_an_answer() {
        let config = Config::first_run();
        let mut engine = Engine::default();
        engine.sync(&config, at(10, 0, 0));
        assert_eq!(engine.due_popups(&config, at(11, 0, 0)), vec![Due { id: 1, repeat: false }]);
        assert_eq!(engine.due_popups(&config, at(12, 0, 0)), vec![Due { id: 1, repeat: false }]);
        assert!(engine.pending_list().is_empty());
    }

    #[test]
    fn a_shown_task_that_was_saved_comes_back_after_a_restart() {
        let config = task_config();
        let mut engine = Engine::default();
        engine.restore_pending(vec![(1, Waiting::Showing)], at(9, 0, 0));
        engine.sync(&config, at(9, 0, 0));
        assert_eq!(engine.due_popups(&config, at(9, 0, 1)), vec![Due { id: 1, repeat: true }]);
    }

    #[test]
    fn disabled_reminders_do_not_fire() {
        let mut config = Config::first_run();
        config.reminders[0].enabled = false;
        let mut engine = Engine::default();
        engine.sync(&config, at(10, 0, 0));
        assert!(engine.next_times().is_empty());
    }
}
