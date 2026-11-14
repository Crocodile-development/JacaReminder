// Code used by the daemon (tray + scheduler) and by the windows.
//   reminder.rs  what a reminder is and how its popup looks
//   schedule.rs  when a reminder fires (interval, calendar rules, one time)
//   engine.rs    keeps the next time of each reminder and says which ones are due
//   storage.rs   the files in %APPDATA%\JacaReminder
//   i18n.rs      the language of the interface (English and Portuguese)
pub mod engine;
pub mod i18n;
pub mod reminder;
pub mod schedule;
pub mod storage;
