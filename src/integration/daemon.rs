// The background process: it keeps the tray icon and checks the reminders every moment.
// It has no window of its own. The dashboard and the popups are separate copies of the program,
// started from here, so closing the dashboard never stops the reminders.

use std::process::Child;
use std::sync::Mutex;
use std::thread;
use std::time::{Duration, SystemTime};

use chrono::Local;
use tray_icon::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tray_icon::{Icon, MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, MSG, PM_REMOVE, PeekMessageW, TranslateMessage,
};

use super::{detach, single_instance};
use crate::shared::engine::{Engine, Waiting};
use crate::shared::i18n::{self, t};
use crate::shared::reminder::{Config, Kind, PopupRequest, Reminder, TaskPopup};
use crate::shared::storage::{self, Answer, HistoryEntry, State};

/// How often the reminders are checked.
const TICK: Duration = Duration::from_millis(250);

/// Shows a popup (a separate process, so it does not depend on this one).
pub fn show_request(request: &PopupRequest) {
    if let Ok(json) = serde_json::to_string(request) {
        detach::spawn(&["--popup".to_string(), json]);
    }
}

/// The popup of the last test, so the next test can take its place.
static TEST_POPUP: Mutex<Option<Child>> = Mutex::new(None);

/// Shows a popup for a test: the one from the previous test is closed first, so the tests never
/// pile up one over the other.
pub fn show_test(request: &PopupRequest) {
    let Ok(json) = serde_json::to_string(request) else {
        return;
    };
    let Ok(mut previous) = TEST_POPUP.lock() else {
        return;
    };
    if let Some(mut child) = previous.take() {
        // Its sound stops with it.
        let _ = child.kill();
        let _ = child.wait();
    }
    *previous = detach::spawn_child(&["--popup".to_string(), json]);
}

/// Shows the popup of a reminder. A new firing is recorded in the history; a task that comes back
/// after a snooze (`repeat`) is not, because it is still the same firing.
fn fire(reminder: &Reminder, repeat: bool) {
    if !repeat {
        storage::append_history(&HistoryEntry {
            id: reminder.id,
            name: reminder.name.clone(),
            at: Local::now().naive_local(),
        });
    }
    let task = (reminder.kind == Kind::Task)
        .then(|| TaskPopup { id: reminder.id, snooze_minutes: reminder.snooze_minutes });
    show_request(&PopupRequest { style: reminder.popup.clone(), task });
}

pub fn open_dashboard() {
    detach::spawn(&["--dashboard".to_string()]);
}

/// The icon is the one put in the exe by build.rs (resource 1). A plain square is the fallback.
fn tray_icon_image() -> Icon {
    Icon::from_resource(1, None)
        .or_else(|_| Icon::from_rgba(vec![0, 120, 215, 255].repeat(16 * 16), 16, 16))
        .expect("a 16x16 icon is always valid")
}

pub fn run() -> std::process::ExitCode {
    // Only one daemon. The listener is kept alive until the end of the function.
    let Some(_claim) = single_instance::claim_daemon() else {
        return std::process::ExitCode::SUCCESS;
    };

    let mut config = storage::load_config();
    i18n::set(config.language);

    let menu = Menu::new();
    let open_item = MenuItem::new(t("Open dashboard"), true, None);
    let quit_item = MenuItem::new(t("Quit"), true, None);
    let _ = menu.append_items(&[&open_item, &PredefinedMenuItem::separator(), &quit_item]);

    let tray = match TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_menu_on_left_click(false)
        .with_tooltip("Jaca Reminder")
        .with_icon(tray_icon_image())
        .build()
    {
        Ok(tray) => tray,
        Err(error) => {
            eprintln!("cannot create the tray icon: {error}");
            return std::process::ExitCode::FAILURE;
        }
    };

    let mut stamp = storage::config_stamp();
    let mut engine = Engine::default();
    let started = Local::now().naive_local();
    // The tasks that were waiting for an answer before this start come back.
    engine.restore_pending(storage::load_state().pending, started);
    engine.sync(&config, started);

    let mut last_saved_state: (Vec<(u64, chrono::NaiveDateTime)>, Vec<(u64, Waiting)>) = Default::default();
    loop {
        pump_windows_messages();

        while let Ok(event) = MenuEvent::receiver().try_recv() {
            if event.id == *open_item.id() {
                open_dashboard();
            } else if event.id == *quit_item.id() {
                return std::process::ExitCode::SUCCESS;
            }
        }
        while let Ok(event) = TrayIconEvent::receiver().try_recv() {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                open_dashboard();
            }
        }

        let now = Local::now().naive_local();
        if reload_if_changed(&mut config, &mut stamp, &mut engine, now) {
            // The language may have changed: the menu and the tooltip follow it.
            i18n::set(config.language);
            open_item.set_text(t("Open dashboard"));
            quit_item.set_text(t("Quit"));
            last_saved_state = Default::default();
        }

        // The buttons pressed on the popups of tasks.
        for action in storage::take_actions() {
            match action.answer {
                Answer::Snooze => {
                    if let Some(reminder) = config.reminders.iter().find(|reminder| reminder.id == action.id) {
                        engine.snooze(action.id, now, reminder.snooze_minutes);
                    }
                }
                Answer::Complete => engine.complete(action.id),
            }
        }

        // A paused app still moves every schedule forward, it just shows nothing.
        for due in engine.due_popups(&config, now) {
            if let Some(reminder) = config.reminders.iter().find(|reminder| reminder.id == due.id) {
                fire(reminder, due.repeat);
            }
        }

        let snapshot = (engine.next_times(), engine.pending_list());
        if snapshot != last_saved_state {
            storage::save_state(&State { next: snapshot.0.clone(), pending: snapshot.1.clone() });
            let tooltip = tooltip_text(&config, &snapshot.0);
            let _ = tray.set_tooltip(Some(tooltip));
            last_saved_state = snapshot;
        }

        thread::sleep(TICK);
    }
}

/// Re-reads config.json when the dashboard saved a new version.
fn reload_if_changed(
    config: &mut Config,
    stamp: &mut Option<SystemTime>,
    engine: &mut Engine,
    now: chrono::NaiveDateTime,
) -> bool {
    let current = storage::config_stamp();
    if current == *stamp {
        return false;
    }
    *stamp = current;
    *config = storage::load_config();
    engine.sync(config, now);
    true
}

fn tooltip_text(config: &Config, next: &[(u64, chrono::NaiveDateTime)]) -> String {
    let first = next.iter().min_by_key(|(_, time)| *time).and_then(|(id, time)| {
        let reminder = config.reminders.iter().find(|reminder| reminder.id == *id)?;
        Some(format!("{}: {}", reminder.name, time.format("%d/%m %H:%M")))
    });
    match first {
        Some(text) => t("Jaca Reminder - next {text}").replace("{text}", &text),
        None => "Jaca Reminder".to_string(),
    }
}

/// The tray icon needs its Windows messages to be handled by the thread that created it.
fn pump_windows_messages() {
    // SAFETY: `message` is a plain struct that Windows fills in; the loop only passes it back to
    // Windows.
    unsafe {
        let mut message: MSG = std::mem::zeroed();
        while PeekMessageW(&mut message, std::ptr::null_mut(), 0, 0, PM_REMOVE) != 0 {
            TranslateMessage(&message);
            DispatchMessageW(&message);
        }
    }
}
