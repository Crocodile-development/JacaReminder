// The files in %APPDATA%\JacaReminder:
//   config.json   the reminders and the options (written by the dashboard)
//   state.json    the next time of each reminder (written by the daemon, read by the dashboard)
//   history.jsonl one line for every time a reminder fired
//   actions.jsonl the answers (snooze, complete) left by the popups of tasks, read by the daemon

use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::time::SystemTime;

use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

use super::engine::Waiting;
use super::reminder::Config;

pub fn data_dir() -> PathBuf {
    let base = std::env::var_os("APPDATA").map(PathBuf::from).unwrap_or_else(std::env::temp_dir);
    let dir = base.join("JacaReminder");
    let _ = fs::create_dir_all(&dir);
    dir
}

fn config_path() -> PathBuf {
    data_dir().join("config.json")
}

/// Reads the configuration. A missing file gives the first-run configuration; a damaged one is
/// moved aside (config.damaged.json) so it is not lost, and the first-run one is used.
pub fn load_config() -> Config {
    let path = config_path();
    let Ok(text) = fs::read_to_string(&path) else {
        return Config::first_run();
    };
    match serde_json::from_str(&text) {
        Ok(config) => config,
        Err(_) => {
            let _ = fs::rename(&path, data_dir().join("config.damaged.json"));
            Config::first_run()
        }
    }
}

/// Writes to a temporary file first and then renames it, so the daemon never reads half a file.
pub fn save_config(config: &Config) -> std::io::Result<()> {
    write_atomic(&config_path(), &serde_json::to_string_pretty(config)?)
}

/// When config.json was last changed; the daemon reloads when this value changes.
pub fn config_stamp() -> Option<SystemTime> {
    fs::metadata(config_path()).ok()?.modified().ok()
}

fn write_atomic(path: &PathBuf, text: &str) -> std::io::Result<()> {
    let temporary = path.with_extension("tmp");
    fs::write(&temporary, text)?;
    fs::rename(&temporary, path)
}

#[derive(Serialize, Deserialize, Default)]
pub struct State {
    pub next: Vec<(u64, NaiveDateTime)>,
    /// The tasks that are waiting for an answer, shown or snoozed.
    #[serde(default)]
    pub pending: Vec<(u64, Waiting)>,
}

/// The button pressed on the popup of a task.
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Debug)]
pub enum Answer {
    Snooze,
    Complete,
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Debug)]
pub struct Action {
    pub id: u64,
    pub answer: Answer,
}

/// Called by the popup process: leaves its answer for the daemon, which reads it within a moment.
pub fn append_action(action: &Action) {
    let Ok(line) = serde_json::to_string(action) else {
        return;
    };
    if let Ok(mut file) = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(data_dir().join("actions.jsonl"))
    {
        let _ = writeln!(file, "{line}");
    }
}

/// Called by the daemon: the answers left since the last call, oldest first.
pub fn take_actions() -> Vec<Action> {
    let path = data_dir().join("actions.jsonl");
    if !path.exists() {
        return Vec::new();
    }
    // Renamed first, so an answer written meanwhile goes to a new file instead of being lost.
    let reading = data_dir().join("actions.reading");
    if fs::rename(&path, &reading).is_err() {
        return Vec::new();
    }
    let text = fs::read_to_string(&reading).unwrap_or_default();
    let _ = fs::remove_file(&reading);
    text.lines().filter_map(|line| serde_json::from_str(line).ok()).collect()
}

pub fn save_state(state: &State) {
    if let Ok(text) = serde_json::to_string(state) {
        let _ = write_atomic(&data_dir().join("state.json"), &text);
    }
}

pub fn load_state() -> State {
    fs::read_to_string(data_dir().join("state.json"))
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

#[derive(Serialize, Deserialize, Clone)]
pub struct HistoryEntry {
    pub id: u64,
    pub name: String,
    pub at: NaiveDateTime,
}

pub fn append_history(entry: &HistoryEntry) {
    let Ok(line) = serde_json::to_string(entry) else {
        return;
    };
    if let Ok(mut file) = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(data_dir().join("history.jsonl"))
    {
        let _ = writeln!(file, "{line}");
    }
}

pub fn load_history() -> Vec<HistoryEntry> {
    let Ok(text) = fs::read_to_string(data_dir().join("history.jsonl")) else {
        return Vec::new();
    };
    text.lines().filter_map(|line| serde_json::from_str(line).ok()).collect()
}

pub fn clear_history() {
    let _ = fs::remove_file(data_dir().join("history.jsonl"));
}
