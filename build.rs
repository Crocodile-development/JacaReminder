// Runs before the program is compiled. On Windows it puts the logo inside the `jaca-reminder.exe` file, so
// Explorer, the taskbar and the shortcuts show it.

fn main() {
    // Build scripts are compiled for the computer that builds, so the target is read from the
    // environment variable that cargo sets.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut resource = winresource::WindowsResource::new();
        resource.set_icon("assets/app-icon.ico");
        // A failure here must not stop the build: the program works the same without the icon.
        if let Err(err) = resource.compile() {
            println!("cargo:warning=could not put the icon in the exe: {err}");
        }
    }
    println!("cargo:rerun-if-changed=assets/app-icon.ico");
    println!("cargo:rerun-if-changed=build.rs");
}
