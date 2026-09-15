//! The movie: the frames the game's `--record` left, with the run's own
//! action rail drawn over them, encoded to `<dir>.mp4`. One tick is one frame
//! and one tick is 1/60 s, so the movie runs in real time however slowly the
//! agent thought.
//!
//! Two paths out of here:
//!
//! - [`compose`] reads `audit.jsonl` beside the frames, builds the
//!   [`rail`], draws it with the [`overlay`] compositor, and pipes raw RGB to
//!   ffmpeg. Layout is this crate's; ffmpeg is the codec backend and nothing
//!   more, so the rail can be read and tested as code.
//! - [`stitch`] hands the untouched PNGs to ffmpeg, which is what a run gets
//!   when the face is missing or `--plain` was asked for.

pub mod overlay;
pub mod rail;

use std::{
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use image::RgbImage;

use crate::{
    audit::{BenchEvent, Bus},
    movie::{overlay::Overlay, rail::At},
    paths::repo_root,
};

/// The frame pattern the channel's recorder writes.
pub const FRAME_PATTERN: &str = "frame_%06d.png";

/// Frames per second of the movie: one per tick, at the game's tick rate.
pub const FRAMES_PER_SECOND: u32 = 60;

/// The face the rail is set in: the game's own terminal font, so a movie and
/// the CRT it shows agree.
pub const FONT: &str = "assets/fonts/SGr-IosevkaTerm-Medium.ttf";

/// A stitched movie.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Movie {
    /// The file.
    pub path: PathBuf,
    /// How many recorded frames went in.
    pub frames: usize,
    /// Held frames added after them so the rail could finish.
    pub tail: usize,
    /// Whether the action rail was drawn over them.
    pub railed: bool,
}

impl Movie {
    /// Seconds of real time the movie runs, held tail included.
    pub fn seconds(&self) -> f64 {
        (self.frames + self.tail) as f64 / f64::from(FRAMES_PER_SECOND)
    }

    /// The line a command prints about this movie.
    pub fn line(&self) -> String {
        let tail = if self.tail == 0 {
            String::new()
        } else {
            format!(
                ", {:.1} s held",
                self.tail as f64 / f64::from(FRAMES_PER_SECOND)
            )
        };
        format!(
            "{} ({} frames, {:.1} s{}{})",
            self.path.display(),
            self.frames,
            self.seconds(),
            tail,
            if self.railed { ", action rail" } else { "" }
        )
    }
}

/// Where the movie of a frames directory goes: beside it, same stem.
pub fn movie_path(frames: &Path) -> PathBuf {
    frames.with_extension("mp4")
}

/// The face the compositor uses unless one is named: the game's, in the repo
/// this dev tool was built from.
pub fn default_font() -> PathBuf {
    repo_root().join(FONT)
}

/// The ffmpeg command line for the plain stitch: what the bench runs, and
/// what a reader without ffmpeg is told to run by hand.
pub fn command_line(frames: &Path) -> Vec<String> {
    [
        "ffmpeg",
        "-y",
        "-loglevel",
        "error",
        "-framerate",
        &FRAMES_PER_SECOND.to_string(),
        "-i",
        &frames.join(FRAME_PATTERN).display().to_string(),
        "-pix_fmt",
        "yuv420p",
        &movie_path(frames).display().to_string(),
    ]
    .into_iter()
    .map(str::to_string)
    .collect()
}

/// The ffmpeg command line for the composed movie: raw RGB on stdin, because
/// the frames have already been drawn on in this process.
pub fn pipe_line(out: &Path, width: u32, height: u32) -> Vec<String> {
    [
        "ffmpeg",
        "-y",
        "-loglevel",
        "error",
        "-f",
        "rawvideo",
        "-pixel_format",
        "rgb24",
        "-video_size",
        &format!("{width}x{height}"),
        "-framerate",
        &FRAMES_PER_SECOND.to_string(),
        "-i",
        "-",
        "-pix_fmt",
        "yuv420p",
        &out.display().to_string(),
    ]
    .into_iter()
    .map(str::to_string)
    .collect()
}

/// The recorder's frames, in the order it wrote them.
pub fn frame_files(frames: &Path) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(frames)
        .into_iter()
        .flatten()
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with("frame_") && name.ends_with(".png"))
        })
        .collect();
    files.sort();
    files
}

/// How many frames the recorder left in the directory.
pub fn frame_count(frames: &Path) -> usize {
    frame_files(frames).len()
}

/// Stitch the frames untouched. The error names what went wrong: no frames,
/// no ffmpeg, or ffmpeg refusing, and carries the command line to run by hand.
pub fn stitch(frames: &Path) -> Result<Movie, String> {
    let count = frame_count(frames);
    if count == 0 {
        return Err(format!("no frames under {}", frames.display()));
    }
    let argv = command_line(frames);
    let by_hand = argv.join(" ");
    let status = Command::new(&argv[0])
        .args(&argv[1..])
        .status()
        .map_err(|error| format!("could not run ffmpeg ({error}); stitch by hand: {by_hand}"))?;
    if !status.success() {
        return Err(format!(
            "ffmpeg failed ({status}); stitch by hand: {by_hand}"
        ));
    }
    Ok(Movie {
        path: movie_path(frames),
        frames: count,
        tail: 0,
        railed: false,
    })
}

/// Draw the run's action rail over its frames and encode the result.
///
/// `progress` is called with the frames done and the total, often enough to
/// keep a several-thousand-frame compose from looking wedged.
pub fn compose(
    audit: &Path,
    frames: &Path,
    out: &Path,
    font: &Path,
    mut progress: impl FnMut(usize, usize),
) -> Result<Movie, String> {
    let files = frame_files(frames);
    let total = files.len();
    if total == 0 {
        return Err(format!("no frames under {}", frames.display()));
    }
    let film = rail::film(&rail::read(audit)?, total as u64);
    let first = read_frame(&files[0])?;
    let (width, height) = (first.width(), first.height());
    let overlay = Overlay::new(font, width)?;

    let argv = pipe_line(out, width, height);
    let mut child = Command::new(&argv[0])
        .args(&argv[1..])
        .stdin(Stdio::piped())
        .spawn()
        .map_err(|error| {
            format!("could not run ffmpeg ({error}); it encodes the composed frames")
        })?;
    let mut sink = child
        .stdin
        .take()
        .ok_or_else(|| "ffmpeg gave the composer no stdin".to_string())?;

    let tail = film.tail(total as u64);
    let mut image = first;
    let mut last = None;
    for (index, file) in files.iter().enumerate() {
        if index > 0 {
            image = read_frame(file)?;
        }
        if image.width() != width || image.height() != height {
            return Err(format!(
                "{} is {}x{}, but the recording starts at {width}x{height}",
                file.display(),
                image.width(),
                image.height()
            ));
        }
        if index + 1 == total && tail > 0 {
            last = Some(image.clone());
        }
        overlay.draw(&mut image, &film, At::live(index as u64));
        sink.write_all(image.as_raw())
            .map_err(|error| format!("ffmpeg stopped reading frames: {error}"))?;
        progress(index + 1, total);
    }

    // The footage has run out but the rail has not. Hold the last frame the
    // game drew, its clock with it, while the remaining rows scroll past.
    if let Some(last) = last {
        let frozen = total as u64 - 1;
        for held in 0..tail {
            let mut image = last.clone();
            overlay.draw(
                &mut image,
                &film,
                At {
                    rail: total as u64 + held,
                    clock: frozen,
                },
            );
            sink.write_all(image.as_raw())
                .map_err(|error| format!("ffmpeg stopped reading frames: {error}"))?;
        }
    }
    drop(sink);

    let status = child
        .wait()
        .map_err(|error| format!("ffmpeg did not finish: {error}"))?;
    if !status.success() {
        return Err(format!("ffmpeg failed ({status})"));
    }
    Ok(Movie {
        path: out.to_path_buf(),
        frames: total,
        tail: tail as usize,
        railed: true,
    })
}

/// One recorded frame as RGB8.
fn read_frame(path: &Path) -> Result<RgbImage, String> {
    Ok(image::open(path)
        .map_err(|error| format!("could not read {}: {error}", path.display()))?
        .to_rgb8())
}

/// Make the movie of a finished run, note the outcome in the audit, and hand
/// back the line the command prints.
///
/// A failure here does not fail the run: the frames are still on disk and the
/// line says what is missing. A compose that cannot find the face falls back
/// to the plain stitch rather than leaving no movie at all.
pub fn report(bus: &Bus, frames: &Path, audit: &Path) -> String {
    let out = movie_path(frames);
    let made = compose(audit, frames, &out, &default_font(), |_, _| {}).or_else(|error| {
        bus.emit(BenchEvent::Note {
            text: format!("movie                no rail: {error}"),
        });
        stitch(frames)
    });
    let line = match made {
        Ok(movie) => format!("movie                {}", movie.line()),
        Err(error) => format!("movie                none: {error}"),
    };
    bus.emit(BenchEvent::Note { text: line.clone() });
    line
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_movie_sits_beside_its_frames_and_runs_one_tick_per_frame() {
        let frames = Path::new("/runs/rec-1");
        assert_eq!(movie_path(frames), PathBuf::from("/runs/rec-1.mp4"));
        let argv = command_line(frames);
        assert_eq!(argv[0], "ffmpeg");
        assert!(argv.contains(&"60".to_string()));
        assert!(argv.contains(&"/runs/rec-1/frame_%06d.png".to_string()));
        assert_eq!(argv.last().unwrap(), "/runs/rec-1.mp4");
        let movie = Movie {
            path: movie_path(frames),
            frames: 661,
            tail: 0,
            railed: true,
        };
        assert!((movie.seconds() - 11.016).abs() < 0.001);
        let held = Movie { tail: 300, ..movie };
        assert!((held.seconds() - 16.016).abs() < 0.001);
        assert_eq!(
            held.line(),
            "/runs/rec-1.mp4 (661 frames, 16.0 s, 5.0 s held, action rail)"
        );
    }

    #[test]
    fn the_composed_movie_is_fed_raw_frames_of_the_recordings_own_size() {
        let argv = pipe_line(Path::new("/runs/rec-1.mp4"), 1280, 720);
        assert!(argv.contains(&"rawvideo".to_string()));
        assert!(argv.contains(&"1280x720".to_string()));
        assert!(argv.contains(&"rgb24".to_string()));
        assert_eq!(argv.last().unwrap(), "/runs/rec-1.mp4");
    }

    #[test]
    fn only_frames_count_and_an_empty_directory_does_not_stitch() {
        let dir = std::env::temp_dir().join(format!("nova-bench-movie-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        assert_eq!(frame_count(&dir), 0);
        assert!(stitch(&dir).unwrap_err().starts_with("no frames"));
        std::fs::write(dir.join("frame_000001.png"), b"").unwrap();
        std::fs::write(dir.join("frame_000000.png"), b"").unwrap();
        std::fs::write(dir.join("notes.txt"), b"").unwrap();
        assert_eq!(frame_count(&dir), 2);
        let files = frame_files(&dir);
        assert!(files[0].ends_with("frame_000000.png"));
        assert!(files[1].ends_with("frame_000001.png"));
        assert_eq!(frame_count(Path::new("/nonexistent/frames")), 0);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
