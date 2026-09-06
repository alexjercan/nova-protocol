//! The movie: the frames the game's `--record` left, stitched into
//! `<dir>.mp4` with ffmpeg. One tick is one frame and one tick is 1/60 s,
//! so the movie runs in real time however slowly the agent thought.

use std::{
    path::{Path, PathBuf},
    process::Command,
};

use crate::audit::{BenchEvent, Bus};

/// The frame pattern the channel's recorder writes.
pub const FRAME_PATTERN: &str = "frame_%06d.png";

/// Frames per second of the movie: one per tick, at the game's tick rate.
pub const FRAMES_PER_SECOND: u32 = 60;

/// A stitched movie.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Movie {
    /// The file.
    pub path: PathBuf,
    /// How many frames went in.
    pub frames: usize,
}

impl Movie {
    /// Seconds of real time the movie runs.
    pub fn seconds(&self) -> f64 {
        self.frames as f64 / f64::from(FRAMES_PER_SECOND)
    }
}

/// Where the movie of a frames directory goes: beside it, same stem.
pub fn movie_path(frames: &Path) -> PathBuf {
    frames.with_extension("mp4")
}

/// The ffmpeg command line: what the bench runs, and what a reader without
/// ffmpeg is told to run by hand.
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

/// How many frames the recorder left in the directory.
pub fn frame_count(frames: &Path) -> usize {
    std::fs::read_dir(frames)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|entry| {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            name.starts_with("frame_") && name.ends_with(".png")
        })
        .count()
}

/// Stitch the frames. The error names what went wrong: no frames, no
/// ffmpeg, or ffmpeg refusing, and carries the command line to run by hand.
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
    })
}

/// Stitch the frames after a run, note the outcome in the audit, and hand
/// back the line the command prints. A stitch that fails does not fail the
/// run: the frames are still on disk and the line says how to finish.
pub fn report(bus: &Bus, frames: &Path) -> String {
    let line = match stitch(frames) {
        Ok(movie) => format!(
            "movie                {} ({} frames, {:.1} s)",
            movie.path.display(),
            movie.frames,
            movie.seconds()
        ),
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
        };
        assert!((movie.seconds() - 11.016).abs() < 0.001);
    }

    #[test]
    fn only_frames_count_and_an_empty_directory_does_not_stitch() {
        let dir = std::env::temp_dir().join(format!("nova-bench-movie-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        assert_eq!(frame_count(&dir), 0);
        assert!(stitch(&dir).unwrap_err().starts_with("no frames"));
        std::fs::write(dir.join("frame_000000.png"), b"").unwrap();
        std::fs::write(dir.join("frame_000001.png"), b"").unwrap();
        std::fs::write(dir.join("notes.txt"), b"").unwrap();
        assert_eq!(frame_count(&dir), 2);
        assert_eq!(frame_count(Path::new("/nonexistent/frames")), 0);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
