// What a reminder is and how its popup looks.

use serde::{Deserialize, Serialize};

use super::i18n::{Language, t};
use super::schedule::Schedule;

/// What happens when the reminder fires.
#[derive(Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, Debug)]
pub enum Kind {
    /// A popup that closes by itself.
    #[default]
    Alert,
    /// A popup that stays until it is completed; snoozing brings it back after a while.
    Task,
}

/// How long a snoozed task waits before it shows again.
pub const DEFAULT_SNOOZE_MINUTES: u32 = 10;

fn default_snooze() -> u32 {
    DEFAULT_SNOOZE_MINUTES
}

#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
pub struct Reminder {
    pub id: u64,
    pub name: String,
    pub enabled: bool,
    pub schedule: Schedule,
    pub popup: PopupStyle,
    #[serde(default)]
    pub kind: Kind,
    /// Minutes until a snoozed task shows again.
    #[serde(default = "default_snooze")]
    pub snooze_minutes: u32,
}

/// What the popup process needs to know to show itself.
#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
pub struct PopupRequest {
    pub style: PopupStyle,
    /// Present for a task: the popup then has the Snooze and Complete buttons.
    #[serde(default)]
    pub task: Option<TaskPopup>,
}

#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
pub struct TaskPopup {
    /// The reminder to answer for. 0 is a preview: its buttons change nothing.
    pub id: u64,
    pub snooze_minutes: u32,
}

impl Reminder {
    /// A new reminder fires once, in one hour; the schedule is chosen in the editor.
    pub fn new_default(name: &str) -> Self {
        use chrono::Timelike;
        let in_an_hour = (chrono::Local::now() + chrono::Duration::hours(1)).naive_local();
        let at = in_an_hour.with_second(0).and_then(|time| time.with_nanosecond(0)).unwrap_or(in_an_hour);
        Self {
            id: 0,
            name: name.into(),
            enabled: true,
            schedule: Schedule::Once { at },
            popup: PopupStyle { title: name.into(), ..PopupStyle::default() },
            kind: Kind::Alert,
            snooze_minutes: DEFAULT_SNOOZE_MINUTES,
        }
    }
}

/// Where on the screen the popup appears. It slides in from the side it is on.
#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Debug)]
pub enum Corner {
    /// "Center" was an option once; reminders saved with it now use this corner.
    #[serde(alias = "Center")]
    TopRight,
    TopLeft,
    BottomRight,
    BottomLeft,
}

impl Corner {
    pub const ALL: [Corner; 4] =
        [Corner::TopRight, Corner::TopLeft, Corner::BottomRight, Corner::BottomLeft];

    pub fn label(self) -> &'static str {
        match self {
            Corner::TopRight => t("Top right"),
            Corner::TopLeft => t("Top left"),
            Corner::BottomRight => t("Bottom right"),
            Corner::BottomLeft => t("Bottom left"),
        }
    }

    /// True for the corners on the right side of the screen.
    pub fn on_right(self) -> bool {
        matches!(self, Corner::TopRight | Corner::BottomRight)
    }

    pub fn at_bottom(self) -> bool {
        matches!(self, Corner::BottomRight | Corner::BottomLeft)
    }
}

/// How the image of a popup is used.
#[derive(Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, Debug)]
pub enum ImageMode {
    /// A square picture on the left, next to the text.
    #[default]
    Icon,
    /// The picture fills the popup, darkened so the text can be read.
    Background,
}

#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
pub enum Sound {
    None,
    /// The water drop that comes with the program.
    Water,
    /// The Windows "exclamation" sound.
    System,
    /// A .wav file chosen by the user.
    File(String),
}

#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
pub struct PopupStyle {
    pub title: String,
    pub message: String,
    pub accent: [u8; 3],
    pub background: [u8; 3],
    pub text: [u8; 3],
    /// How long the popup stays. 0 keeps it until clicked.
    pub seconds: u32,
    pub corner: Corner,
    pub width: u32,
    pub height: u32,
    pub rounding: u8,
    pub progress_bar: bool,
    pub sound: Sound,
    /// Path of a picture to show in the popup.
    #[serde(default)]
    pub image: Option<String>,
    #[serde(default)]
    pub image_mode: ImageMode,
    /// Which corners are rounded: top left, top right, bottom right, bottom left.
    #[serde(default = "all_corners")]
    pub round_corners: [bool; 4],
    /// Distance from the left or right edge of the screen (the side the popup is on), in points.
    #[serde(default = "default_edge_distance")]
    pub edge_distance: u32,
    /// Distance from the top or bottom edge of the screen, in points.
    #[serde(default = "default_edge_distance")]
    pub edge_distance_y: u32,
}

fn all_corners() -> [bool; 4] {
    [true; 4]
}

pub const DEFAULT_EDGE_DISTANCE: u32 = 12;

fn default_edge_distance() -> u32 {
    DEFAULT_EDGE_DISTANCE
}

impl Default for PopupStyle {
    fn default() -> Self {
        Self {
            title: "Reminder".into(),
            message: String::new(),
            accent: [0, 120, 215],
            background: [32, 32, 32],
            text: [255, 255, 255],
            seconds: 5,
            corner: Corner::TopRight,
            width: 320,
            height: 100,
            rounding: 14,
            progress_bar: true,
            sound: Sound::System,
            image: None,
            image_mode: ImageMode::default(),
            round_corners: all_corners(),
            edge_distance: DEFAULT_EDGE_DISTANCE,
            edge_distance_y: DEFAULT_EDGE_DISTANCE,
        }
    }
}

/// Everything saved in config.json.
#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
pub struct Config {
    pub reminders: Vec<Reminder>,
    pub next_id: u64,
    pub start_with_windows: bool,
    /// Volume of the alert sounds, 0 to 100.
    #[serde(default = "full_volume")]
    pub alert_volume: u8,
    /// While true no reminder shows its popup; they keep their schedule.
    #[serde(default)]
    pub pause_all: bool,
    #[serde(default)]
    pub language: Language,
}

fn full_volume() -> u8 {
    100
}

impl Config {
    /// The first run starts with the water reminder of the old Water Reminder program.
    pub fn first_run() -> Self {
        let water = Reminder {
            id: 1,
            name: "Water".into(),
            enabled: true,
            schedule: Schedule::Interval { minutes: 60, window: None },
            popup: PopupStyle {
                title: "Time to drink water!".into(),
                message: "Take a big sip".into(),
                sound: Sound::Water,
                seconds: 3,
                ..PopupStyle::default()
            },
            kind: Kind::Alert,
            snooze_minutes: DEFAULT_SNOOZE_MINUTES,
        };
        Self { reminders: vec![water], next_id: 2, start_with_windows: false, alert_volume: 100,
            pause_all: false,
            language: Language::default(),
        }
    }

    pub fn add(&mut self, mut reminder: Reminder) -> u64 {
        reminder.id = self.next_id;
        self.next_id += 1;
        let id = reminder.id;
        self.reminders.push(reminder);
        id
    }
}
