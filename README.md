<p align="center">
  <img src="assets/app-icon.png" width="160" height="160" alt="Jaca Reminder logo">
</p>

<h1 align="center">Jaca Reminder</h1>

<p align="center">
  A native Windows reminder app, written in Rust with egui.<br>
  Set a reminder to fire once, every few minutes, or on chosen days and times, and design the popup that shows it.<br>
  A reminder can also be a task: its popup stays until you complete it, and comes back when you snooze it.
</p>

| Type | What happens when it fires |
|---|---|
| **Alert** | The popup slides in, plays its sound, and slides out by itself after the time you chose (or when clicked) |
| **Task** | The popup stays on the screen with its sound until you press **Snooze** (it comes back after the time you chose) or **Complete** (it only fires again at its next scheduled time) |

| Schedule | Meaning |
|---|---|
| **One time** | A single date and time (the default for a new reminder) |
| **Times** | Every X hours and minutes, optionally only between two hours of the day |
| **Days and times** | One or more times of the day, on every day, Mon-Fri, weekends, the last day of the month, or chosen weekdays, days of the month, months, years and a date range |

## Features

- **Dashboard** with a card for each reminder: its next time (a countdown), an on/off switch, and a filter for All, Active and Paused. A task that is waiting for you shows "Waiting for you" or "Snoozed until 14:30".
- **Tray icon** with the scheduler behind it. Closing the dashboard never stops the reminders.
- **Popup you can design**: title and message, three colors, the corner of the screen (it slides in from that side), the distance from the edge (horizontal and vertical), size, which corners are rounded and by how much, an image (as an icon or as the background), and the sound.
- **Sound**: none, a water drop, the Windows beep, or your own file. MP3, OGG, FLAC, M4A/AAC and WAV are accepted and converted to WAV when chosen. The sound repeats while the popup is on the screen, and stops as soon as a task is answered.
- **History** of what fired (last 7 days and the latest firings).
- **Settings**: language (English or Português), alert volume, pause all reminders, start with Windows, and the data folder.
- **Single instance**: opening the program again brings the running window to the front.
- **Limited to 30 frames per second**, so it stays light while it sits in the background.

## Install

Jaca Reminder targets Windows 10 and 11. Download `JacaReminder-Setup-<version>.exe` from the
[Releases](../../releases) page and run it. It installs for the current user, with no administrator
rights, and everything the program needs is inside the one exe.

Updating is the same: run the new installer over the old one. Your reminders are kept.

### Build the installer

The installer is made with [Inno Setup 6](https://jrsoftware.org/isinfo.php). Build the release first,
then compile the script:

```powershell
cargo build --release
ISCC.exe installer\jaca-reminder.iss
```

The setup file is written to `dist\`. To name another version: `ISCC.exe /DAppVersion=1.2.3 installer\jaca-reminder.iss`.

### From source

You need the [Rust toolchain](https://rustup.rs/) (edition 2024) with the MSVC target. No other
system library is required.

```powershell
cargo run                    # opens the dashboard (and starts the background part if needed)
cargo build --release        # produces target\release\jaca-reminder.exe
cargo test                   # the schedule, engine and conversion tests
```

## How it runs

One exe, three modes. The background part shows the popups as separate processes, so a popup never
depends on the dashboard being open.

| Command | What it does |
|---|---|
| `jaca-reminder` | Starts the background part if it is not running and opens the dashboard |
| `jaca-reminder --daemon` | The tray icon and the scheduler, with no window |
| `jaca-reminder --dashboard` | Only the dashboard |
| `jaca-reminder --popup <json>` | One popup (started by the background part and by the **Test popup** button) |

## Where the data is

Everything is in `%APPDATA%\JacaReminder`:

| File | What it holds |
|---|---|
| `config.json` | The reminders and the settings (written by the dashboard, reloaded by the background part when it changes) |
| `state.json` | The next time of each reminder and the tasks that are waiting for an answer (written by the background part) |
| `history.jsonl` | One line for every time a reminder fired |
| `actions.jsonl` | The answers (Snooze, Complete) left by the popups of tasks, read by the background part within a moment |
| `sounds\` | The WAV copies of the sounds you chose |

## Decisions so far

| Decision | Why |
|---|---|
| egui / eframe, pure Rust, the C runtime linked in | One small `jaca-reminder.exe` that starts fast and runs on a clean Windows with nothing to install |
| The scheduler and the dashboard are separate processes | Reminders keep firing when the dashboard is closed, and the dashboard can be reopened at any time |
| Popups are separate processes too | A popup (and its sound) lives and dies on its own, and **Test popup** can replace the previous test |
| The popup is cut to its shape with a window region, not a transparent window | A transparent window left light pixels in the corners; the region also hides the part of the popup that is beyond the edge of the monitor while it slides |
| The popup answers a task through `actions.jsonl` | A file survives a restart and needs no connection between the two processes |
| A task that is still waiting is not shown again by its schedule | The popups never pile up; the same task just stays pending |
| Closing the popup of a task without pressing a button counts as a snooze | A task is never lost by closing a window |
| Sounds are converted to WAV when chosen, up to 60 seconds | Windows plays WAV with no extra parts, and the copy keeps working if the original is moved |
| Windows beep is repeated from a thread | It has no loop of its own |
| New reminders must be changed before they can be saved | Save is disabled when nothing is different from the starting values |
| Every text goes through one function, with the Portuguese texts in one file | Adding a language is adding one file's worth of texts, and the code stays in English |
| The installer is built only on request and `dist\` is not versioned | `dist\` is generated output; installers go on the Releases page |

## Layout

```
src/
  main.rs            entry point and the three modes
  gui/               windows (egui)
    app.rs           the dashboard: the state of the window and how each frame is put together
    home.rs          the reminders: cards, filter and status
    reminder_editor.rs  the editor of one reminder (General and Popup tabs)
    schedule_editor.rs  the schedule part of the editor
    popup_editor.rs  the popup part of the editor
    popup.rs         the popup window: slide in and out, image, buttons of a task
    history.rs       the history page
    settings.rs      the settings pages
    nav.rs           the sidebar
    titlebar.rs      the top bar that replaces the system one
    widgets.rs       buttons, switches, selectors and the other small inputs
    theme.rs         colors, fonts and sizes
  integration/       glue with Windows
    daemon.rs        tray icon and scheduler
    sound.rs         plays the sounds, once or looping
    audio_import.rs  turns MP3, OGG, FLAC, M4A/AAC into WAV
    window_shape.rs  rounded corners and the part of the window that is on screen
    startup.rs       the "start with Windows" option
    single_instance.rs, detach.rs
  shared/            code every mode uses
    reminder.rs      what a reminder, a popup and the settings are
    schedule.rs      when a reminder fires
    engine.rs        the next time of each reminder and the tasks that wait for an answer
    storage.rs       the files in %APPDATA%\JacaReminder
    i18n.rs          English and Portuguese
assets/              logo (png, ico) and the water drop sound
installer/           Inno Setup script
build.rs             puts the logo inside jaca-reminder.exe
.cargo/config.toml   links the C runtime into the exe
```
