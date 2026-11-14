// Starts this same program again, without any connection to the terminal that started it.
// Used to free the terminal when the dashboard opens, to start the daemon, and to show popups.

use std::process::{Child, Command, Stdio};

/// The extra argument that marks the copy, so it does not start yet another copy.
pub const DETACHED_FLAG: &str = "--detached";

/// Starts the program with `args`. True if it was started.
pub fn spawn(args: &[String]) -> bool {
    spawn_child(args).is_some()
}

/// Starts the program with `args` and returns the process, so the caller can stop it later.
pub fn spawn_child(args: &[String]) -> Option<Child> {
    let Ok(exe) = std::env::current_exe() else {
        return None;
    };

    let mut command = Command::new(exe);
    command
        .args(args)
        .arg(DETACHED_FLAG)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const DETACHED_PROCESS: u32 = 0x0000_0008;
        const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
        command.creation_flags(DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP);
    }

    command.spawn().ok()
}
