//! Clips (a scripted run's `record`): each frame drawn is piped to ffmpeg as it is
//! drawn, and the mix as the game made it during that frame is written to a WAV
//! beside the video, in step. `promo/clip.sh` puts the two together. Both files are
//! finished when the recording is dropped, however the run ends.

use std::io::{BufWriter, Seek, SeekFrom, Write};
use std::path::Path;
use std::process::{Child, ChildStdin, Command, Stdio};

pub struct Recording {
    ffmpeg: Child,
    frames: Option<BufWriter<ChildStdin>>,
    wav: BufWriter<std::fs::File>,
    /// Samples written to the WAV (both channels), at `rate`.
    samples: u32,
    rate: u32,
    size: (u32, u32),
    /// Frames passed over for being of another size than the first.
    skipped: u32,
}

impl Recording {
    /// Record to `<stem>.mkv` (frames of `size`, at `fps`) and `<stem>.wav` (stereo
    /// at `rate`).
    pub fn start(stem: &Path, size: (u32, u32), fps: u32, rate: u32) -> Result<Recording, String> {
        let video = stem.with_extension("mkv");
        let file = std::fs::File::create(stem.with_extension("wav")).map_err(|e| e.to_string())?;
        let mut wav = BufWriter::new(file);
        wav_header(&mut wav, rate, 0).map_err(|e| e.to_string())?;
        // Near lossless: it is cut and encoded again for the store.
        let mut ffmpeg = Command::new("ffmpeg")
            .args(["-y", "-loglevel", "error", "-f", "rawvideo", "-pix_fmt", "rgba"])
            .args([
                "-s",
                &format!("{}x{}", size.0, size.1),
                "-r",
                &fps.to_string(),
                "-i",
                "-",
            ])
            .args([
                "-c:v", "libx264", "-preset", "medium", "-crf", "12", "-pix_fmt", "yuv444p",
            ])
            .arg(&video)
            .stdin(Stdio::piped())
            .spawn()
            .map_err(|e| format!("ffmpeg: {e}"))?;
        let frames = ffmpeg.stdin.take().map(BufWriter::new);
        eprintln!(
            "hushfold: recording {} ({}x{} at {fps} fps)",
            video.display(),
            size.0,
            size.1
        );
        Ok(Recording {
            ffmpeg,
            frames,
            wav,
            samples: 0,
            rate,
            size,
            skipped: 0,
        })
    }

    /// A frame drawn (RGBA rows), and the mix made with it. A frame of another size
    /// (the window resized) is passed over.
    pub fn frame(&mut self, pixels: &[u8], size: (u32, u32), mix: &[f32]) {
        if size != self.size {
            if self.skipped == 0 {
                eprintln!("hushfold: the window is no longer the clip's size; its frames are passed over");
            }
            self.skipped += 1;
            return;
        }
        if let Some(f) = &mut self.frames
            && f.write_all(pixels).is_err()
        {
            eprintln!("hushfold: ffmpeg stopped taking frames");
            self.frames = None;
        }
        for s in mix {
            let _ = self.wav.write_all(&s.to_le_bytes());
        }
        self.samples += mix.len() as u32;
    }
}

impl Drop for Recording {
    /// Finish both files: the video's encoding, and the WAV's header with its
    /// length.
    fn drop(&mut self) {
        drop(self.frames.take());
        if let Err(e) = self.ffmpeg.wait() {
            eprintln!("hushfold: ffmpeg: {e}");
        }
        let (rate, samples) = (self.rate, self.samples);
        let wav = &mut self.wav;
        let header = wav
            .seek(SeekFrom::Start(0))
            .and_then(|_| wav_header(wav, rate, samples));
        if let Err(e) = header.and_then(|()| wav.flush()) {
            eprintln!("hushfold: the clip's sound: {e}");
        }
        if self.skipped > 0 {
            eprintln!(
                "hushfold: {} frames of another size were left out of the clip",
                self.skipped
            );
        }
    }
}

/// A WAV header for 32-bit float stereo at `rate`, `samples` samples long.
fn wav_header(w: &mut impl Write, rate: u32, samples: u32) -> std::io::Result<()> {
    let data = samples * 4;
    w.write_all(b"RIFF")?;
    w.write_all(&(36 + data).to_le_bytes())?;
    w.write_all(b"WAVEfmt ")?;
    w.write_all(&16u32.to_le_bytes())?;
    // IEEE float, two channels.
    w.write_all(&3u16.to_le_bytes())?;
    w.write_all(&2u16.to_le_bytes())?;
    w.write_all(&rate.to_le_bytes())?;
    w.write_all(&(rate * 8).to_le_bytes())?;
    w.write_all(&8u16.to_le_bytes())?;
    w.write_all(&32u16.to_le_bytes())?;
    w.write_all(b"data")?;
    w.write_all(&data.to_le_bytes())
}
