// Plays the sound of a popup at the volume chosen in the settings.
// Windows has no volume for PlaySound, so for WAV sounds the volume is applied by scaling the
// samples before playing. The Windows beep always follows the system volume.

use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use windows_sys::Win32::Media::Audio::{PlaySoundW, SND_ASYNC, SND_FILENAME, SND_LOOP, SND_MEMORY};
use windows_sys::Win32::System::Diagnostics::Debug::MessageBeep;
use windows_sys::Win32::UI::WindowsAndMessaging::MB_ICONEXCLAMATION;

use crate::shared::reminder::Sound;
use crate::shared::storage;

/// The water drop that comes with the program.
static WATER: &[u8] = include_bytes!("../../assets/water.wav");

/// The sound being played from memory. `SND_ASYNC` reads it while it plays, so it has to stay
/// alive until the next sound replaces it.
static PLAYING: Mutex<Vec<u8>> = Mutex::new(Vec::new());

/// True while the Windows beep is being repeated (see `start_looping`).
static BEEPING: AtomicBool = AtomicBool::new(false);

/// How long the Windows beep waits before it sounds again while a popup is open.
const BEEP_INTERVAL: Duration = Duration::from_millis(1400);

/// Plays `sound` once, at the volume saved in the settings.
pub fn play(sound: &Sound) {
    play_at(sound, storage::load_config().alert_volume);
}

/// Plays `sound` once, at `volume` (0 to 100).
pub fn play_at(sound: &Sound, volume: u8) {
    play_with(sound, volume, false);
}

/// Plays `sound` again and again, at the volume saved in the settings, until `stop`. This is the
/// sound of a popup: it lasts as long as the popup is on the screen.
pub fn start_looping(sound: &Sound) {
    play_with(sound, storage::load_config().alert_volume, true);
}

/// Stops the sound that `start_looping` started.
pub fn stop() {
    BEEPING.store(false, Ordering::Relaxed);
    // SAFETY: a null sound stops the one that is playing.
    unsafe {
        PlaySoundW(std::ptr::null(), std::ptr::null_mut(), 0);
    }
}

fn play_with(sound: &Sound, volume: u8, looped: bool) {
    match sound {
        Sound::None => {}
        Sound::System => {
            if looped {
                beep_until_stopped();
            } else {
                beep();
            }
        }
        Sound::Water => play_wav(WATER.to_vec(), volume, looped),
        Sound::File(path) => match std::fs::read(path) {
            Ok(bytes) if scale_wav(&bytes, 100).is_some() => play_wav(bytes, volume, looped),
            _ => play_file(path, looped),
        },
    }
}

/// How long `sound` lasts when played once, to know when a test is over.
pub fn duration(sound: &Sound) -> Duration {
    match sound {
        Sound::None => Duration::ZERO,
        Sound::System => Duration::from_millis(700),
        Sound::Water => wav_duration(WATER).unwrap_or_default(),
        Sound::File(path) => std::fs::read(path).ok().and_then(|bytes| wav_duration(&bytes)).unwrap_or_default(),
    }
}

/// The length of a WAV, read from its header. None when the data is not a WAV.
fn wav_duration(wav: &[u8]) -> Option<Duration> {
    if wav.len() < 12 || &wav[0..4] != b"RIFF" || &wav[8..12] != b"WAVE" {
        return None;
    }
    let mut bytes_per_second = 0u32;
    let mut position = 12;
    while position + 8 <= wav.len() {
        let id = &wav[position..position + 4];
        let size = u32::from_le_bytes(wav[position + 4..position + 8].try_into().ok()?) as usize;
        let body = position + 8;
        if id == b"fmt " && body + 12 <= wav.len() {
            bytes_per_second = u32::from_le_bytes(wav[body + 8..body + 12].try_into().ok()?);
        } else if id == b"data" {
            let data = size.min(wav.len() - body) as f64;
            return (bytes_per_second > 0).then(|| Duration::from_secs_f64(data / bytes_per_second as f64));
        }
        position = body + size + (size & 1);
    }
    None
}

fn beep() {
    // SAFETY: MessageBeep takes only a flag.
    unsafe {
        MessageBeep(MB_ICONEXCLAMATION);
    }
}

/// The Windows beep has no loop of its own, so it is repeated from a thread until `stop`.
fn beep_until_stopped() {
    if BEEPING.swap(true, Ordering::Relaxed) {
        return;
    }
    std::thread::spawn(|| {
        while BEEPING.load(Ordering::Relaxed) {
            beep();
            std::thread::sleep(BEEP_INTERVAL);
        }
    });
}

fn play_wav(bytes: Vec<u8>, volume: u8, looped: bool) {
    let data = scale_wav(&bytes, volume).unwrap_or(bytes);
    let Ok(mut playing) = PLAYING.lock() else { return };
    let old = std::mem::replace(&mut *playing, data);
    let flags = SND_MEMORY | SND_ASYNC | if looped { SND_LOOP } else { 0 };
    // SAFETY: the buffer lives in PLAYING until the next sound replaces it, and the flags match
    // a memory pointer.
    unsafe {
        PlaySoundW(playing.as_ptr() as *const u16, std::ptr::null_mut(), flags);
    }
    drop(old);
}

/// A file that is not 16-bit PCM WAV: played as it is, without volume control.
fn play_file(path: &str, looped: bool) {
    let wide: Vec<u16> = path.encode_utf16().chain(Some(0)).collect();
    let flags = SND_FILENAME | SND_ASYNC | if looped { SND_LOOP } else { 0 };
    // SAFETY: `wide` is a valid zero-ended string for the call.
    unsafe {
        PlaySoundW(wide.as_ptr(), std::ptr::null_mut(), flags);
    }
}

/// A copy of a 16-bit PCM WAV with every sample multiplied by `volume`/100. None when the data
/// is not that kind of WAV.
fn scale_wav(wav: &[u8], volume: u8) -> Option<Vec<u8>> {
    if wav.len() < 12 || &wav[0..4] != b"RIFF" || &wav[8..12] != b"WAVE" {
        return None;
    }
    let mut pcm_16 = false;
    let mut position = 12;
    let mut data_range = None;
    while position + 8 <= wav.len() {
        let id = &wav[position..position + 4];
        let size = u32::from_le_bytes(wav[position + 4..position + 8].try_into().ok()?) as usize;
        let body = position + 8;
        let end = (body + size).min(wav.len());
        if id == b"fmt " && end - body >= 16 {
            let tag = u16::from_le_bytes([wav[body], wav[body + 1]]);
            let bits = u16::from_le_bytes([wav[body + 14], wav[body + 15]]);
            pcm_16 = tag == 1 && bits == 16;
        } else if id == b"data" {
            data_range = Some(body..end);
            break;
        }
        position = body + size + (size & 1);
    }
    let range = data_range.filter(|_| pcm_16)?;
    let mut copy = wav.to_vec();
    let factor = f32::from(volume.min(100)) / 100.0;
    for sample in copy[range].chunks_exact_mut(2) {
        let value = i16::from_le_bytes([sample[0], sample[1]]);
        sample.copy_from_slice(&((f32::from(value) * factor) as i16).to_le_bytes());
    }
    Some(copy)
}
