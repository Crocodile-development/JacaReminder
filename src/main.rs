// In the release version the program has no console window.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

// Program layout:
//   gui/          the dashboard window and the popup (egui)
//   integration/  connections with Windows: tray + scheduler (daemon), sound, startup, one instance
//   shared/       reminders, schedules, the engine and the saved files, used by all of them
//
// The same program runs in three modes:
//   jaca-reminder              starts the daemon if needed and opens the dashboard
//   jaca-reminder --daemon     tray icon and scheduler, no window
//   jaca-reminder --popup JSON shows one popup (started by the daemon)
mod gui;
mod integration;
mod shared;

use std::process::ExitCode;

use integration::{daemon, detach, single_instance};

fn main() -> ExitCode {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    let detached = take_flag(&mut args, detach::DETACHED_FLAG);

    if take_flag(&mut args, "--daemon") {
        return daemon::run();
    }

    if take_flag(&mut args, "--popup") {
        let Some(request) = args.first().and_then(|json| serde_json::from_str(json).ok()) else {
            eprintln!("--popup needs the popup request as JSON");
            return ExitCode::FAILURE;
        };
        return report(gui::run_popup(request));
    }

    // A reminder is only useful if the daemon is running, so make sure it is.
    if !single_instance::daemon_running() {
        detach::spawn(&["--daemon".to_string()]);
    }
    // Free the terminal that typed the command, unless this already is the detached copy.
    let dashboard_flag = take_flag(&mut args, "--dashboard");
    let wait = take_flag(&mut args, "--wait");
    if !detached && !dashboard_flag && !wait && detach::spawn(&["--dashboard".to_string()]) {
        return ExitCode::SUCCESS;
    }
    report(gui::run_dashboard())
}

fn take_flag(args: &mut Vec<String>, flag: &str) -> bool {
    let found = args.iter().any(|arg| arg == flag);
    args.retain(|arg| arg != flag);
    found
}

fn report(result: eframe::Result) -> ExitCode {
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("cannot open window: {error}");
            ExitCode::FAILURE
        }
    }
}
