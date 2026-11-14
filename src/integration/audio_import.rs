// Turns the sound chosen by the user (MP3, OGG, FLAC, AAC/M4A or WAV) into a 16-bit WAV kept in the
// data folder. The popups only play WAV, so the conversion is done once, when the file is chosen,
// and the reminder keeps the path of the converted copy (which also keeps working if the original
// is moved or deleted).

use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};

use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::DecoderOptions;
use symphonia::core::errors::Error;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

use crate::shared::storage;

/// Longer sounds are cut here, so a long song does not fill the disk and the memory.
const MAX_SECONDS: u32 = 60;

/// Converts `source` and returns the path of the WAV that was written in the data folder.
pub fn import_to_wav(source: &Path) -> Result<PathBuf, String> {
    convert_into(source, &storage::data_dir().join("sounds"))
}

fn convert_into(source: &Path, folder: &Path) -> Result<PathBuf, String> {
    let (samples, rate, channels) = decode(source)?;
    if samples.is_empty() {
        return Err("The file has no sound".to_string());
    }
    fs::create_dir_all(folder).map_err(|error| error.to_string())?;
    let target = folder.join(format!("{}.wav", safe_name(source)));
    write_wav(&target, &samples, rate, channels).map_err(|error| error.to_string())?;
    Ok(target)
}

/// The name of the file without extension, with only letters, digits, spaces, dashes and
/// underscores, so it is a valid file name anywhere.
fn safe_name(source: &Path) -> String {
    let stem = source.file_stem().and_then(|stem| stem.to_str()).unwrap_or("sound");
    let name: String = stem
        .chars()
        .map(|c| if c.is_alphanumeric() || matches!(c, ' ' | '-' | '_') { c } else { '_' })
        .collect();
    let name = name.trim();
    if name.is_empty() { "sound".to_string() } else { name.chars().take(60).collect() }
}

/// The samples (16 bit, channels interleaved), the sample rate and the number of channels.
fn decode(source: &Path) -> Result<(Vec<i16>, u32, u16), String> {
    let unsupported = || "This file could not be read as a sound".to_string();
    let file = File::open(source).map_err(|error| error.to_string())?;
    let stream = MediaSourceStream::new(Box::new(file), Default::default());
    let mut hint = Hint::new();
    if let Some(extension) = source.extension().and_then(|extension| extension.to_str()) {
        hint.with_extension(extension);
    }
    let probed = symphonia::default::get_probe()
        .format(&hint, stream, &FormatOptions::default(), &MetadataOptions::default())
        .map_err(|_| unsupported())?;
    let mut format = probed.format;
    let track = format.default_track().ok_or_else(unsupported)?;
    let track_id = track.id;
    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &DecoderOptions::default())
        .map_err(|_| unsupported())?;

    let mut samples: Vec<i16> = Vec::new();
    let mut rate = 0;
    let mut channels = 0;
    loop {
        let packet = match format.next_packet() {
            Ok(packet) => packet,
            // The end of the file shows up as an I/O error.
            Err(Error::IoError(_)) | Err(Error::ResetRequired) => break,
            Err(error) => return Err(error.to_string()),
        };
        if packet.track_id() != track_id {
            continue;
        }
        match decoder.decode(&packet) {
            Ok(decoded) => {
                let spec = *decoded.spec();
                rate = spec.rate;
                channels = spec.channels.count() as u16;
                let mut buffer = SampleBuffer::<i16>::new(decoded.capacity() as u64, spec);
                buffer.copy_interleaved_ref(decoded);
                samples.extend_from_slice(buffer.samples());
                if samples.len() as u64 >= rate as u64 * channels as u64 * MAX_SECONDS as u64 {
                    break;
                }
            }
            // A damaged packet is skipped.
            Err(Error::DecodeError(_)) => continue,
            Err(error) => return Err(error.to_string()),
        }
    }
    if rate == 0 || channels == 0 {
        return Err(unsupported());
    }
    Ok((samples, rate, channels))
}

fn write_wav(path: &Path, samples: &[i16], rate: u32, channels: u16) -> std::io::Result<()> {
    let data_bytes = (samples.len() * 2) as u32;
    let block_align = channels * 2;
    let mut file = File::create(path)?;
    file.write_all(b"RIFF")?;
    file.write_all(&(36 + data_bytes).to_le_bytes())?;
    file.write_all(b"WAVEfmt ")?;
    file.write_all(&16u32.to_le_bytes())?;
    file.write_all(&1u16.to_le_bytes())?; // PCM
    file.write_all(&channels.to_le_bytes())?;
    file.write_all(&rate.to_le_bytes())?;
    file.write_all(&(rate * block_align as u32).to_le_bytes())?;
    file.write_all(&block_align.to_le_bytes())?;
    file.write_all(&16u16.to_le_bytes())?; // bits per sample
    file.write_all(b"data")?;
    file.write_all(&data_bytes.to_le_bytes())?;
    let mut bytes = Vec::with_capacity(samples.len() * 2);
    for sample in samples {
        bytes.extend_from_slice(&sample.to_le_bytes());
    }
    file.write_all(&bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A folder of its own under the temporary folder, so the tests never touch the real data.
    fn scratch(name: &str) -> PathBuf {
        let folder = std::env::temp_dir().join(format!("jaca-reminder-test-{name}"));
        let _ = fs::remove_dir_all(&folder);
        fs::create_dir_all(&folder).unwrap();
        folder
    }

    #[test]
    fn a_wav_is_read_and_written_back_with_the_same_samples() {
        let folder = scratch("wav-roundtrip");
        let source = folder.join("tone.wav");
        let samples: Vec<i16> = (0..4410).map(|n| ((n % 100) as i16 - 50) * 100).collect();
        write_wav(&source, &samples, 44_100, 1).unwrap();

        let converted = convert_into(&source, &folder.join("out")).unwrap();
        let (back, rate, channels) = decode(&converted).unwrap();
        assert_eq!((rate, channels), (44_100, 1));
        assert_eq!(back, samples);
    }

    #[test]
    fn a_file_that_is_not_a_sound_is_refused() {
        let folder = scratch("not-a-sound");
        let source = folder.join("notes.mp3");
        fs::write(&source, b"this is plain text, not audio").unwrap();
        assert!(convert_into(&source, &folder.join("out")).is_err());
    }

    #[test]
    fn the_name_of_the_converted_file_is_safe() {
        assert_eq!(safe_name(Path::new("C:/music/My song: part 1?.mp3")), "My song_ part 1_");
        assert_eq!(safe_name(Path::new("C:/music/???.mp3")), "___");
    }

    /// Run on purpose with a real file: JACA_TEST_AUDIO=path cargo test real_file -- --ignored
    #[test]
    #[ignore]
    fn real_file_is_converted() {
        let path = std::env::var("JACA_TEST_AUDIO").expect("set JACA_TEST_AUDIO to an audio file");
        let folder = scratch("real-file");
        let converted = convert_into(Path::new(&path), &folder).unwrap();
        let (samples, rate, channels) = decode(&converted).unwrap();
        println!("converted: {} samples, {} Hz, {} channels, {} bytes on disk", samples.len(), rate, channels,
            fs::metadata(&converted).unwrap().len());
        assert!(samples.len() > 1000);
    }
}
