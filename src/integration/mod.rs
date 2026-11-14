// Connections with the operating system.
//   audio_import.rs     turns a chosen sound (MP3, OGG, FLAC, ...) into a WAV
//   daemon.rs           the background process: tray icon and the scheduler
//   detach.rs           starts a copy of the program that is free of the terminal
//   single_instance.rs  makes sure only one daemon and one dashboard are open
//   sound.rs            plays the sound of a popup
//   startup.rs          the "start with Windows" option
//   window_shape.rs     the shape of borderless windows: rounded corners and the part on screen
pub mod audio_import;
pub mod daemon;
pub mod detach;
pub mod single_instance;
pub mod sound;
pub mod startup;
pub mod window_shape;
