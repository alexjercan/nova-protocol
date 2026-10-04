//! The movie: the frames the game's `--record` left, with the run's own
//! action rail drawn over them, encoded to `<dir>.mp4`. When the recording
//! has `bench.webm`, its Opus track accompanies the movie. One tick is one
//! frame and one tick is 1/60 s, so the movie runs in real time however
//! slowly the agent thought.
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
    fs::File,
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

/// Find the audio left by a new recording. A legacy bare-PNG recording has
/// no sidecar and can still be stitched offline without sound.
fn audio_source(frames: &Path) -> Result<Option<PathBuf>, String> {
    let marker = frames.join("bench.jsonl");
    let expected = marker
        .try_exists()
        .map_err(|error| format!("could not inspect {}: {error}", marker.display()))?;
    let audio = frames.join("bench.webm");
    match File::open(&audio) {
        Ok(_) => Ok(Some(audio)),
        Err(error) if expected => Err(format!(
            "recording {} requires readable audio {}: {error}",
            marker.display(),
            audio.display()
        )),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(format!("could not read audio {}: {error}", audio.display())),
    }
}

/// The ffmpeg command line for the plain stitch: what the bench runs, and
/// what a reader without ffmpeg is told to run by hand. A new recording
/// without readable audio cannot produce a frame-only command.
pub fn command_line(frames: &Path) -> Result<Vec<String>, String> {
    let count = recording_frames(frames)?.len();
    command_line_for_count(frames, count)
}

fn command_line_for_count(frames: &Path, count: usize) -> Result<Vec<String>, String> {
    let mut argv: Vec<String> = [
        "ffmpeg",
        "-y",
        "-loglevel",
        "error",
        "-framerate",
        &FRAMES_PER_SECOND.to_string(),
        "-i",
        &frames.join(FRAME_PATTERN).display().to_string(),
    ]
    .into_iter()
    .map(str::to_string)
    .collect();
    if let Some(audio) = audio_source(frames)? {
        argv.extend([
            "-i".to_string(),
            audio.display().to_string(),
            "-map".to_string(),
            "0:v:0".to_string(),
            "-map".to_string(),
            "1:a:0".to_string(),
            "-c:a".to_string(),
            "aac".to_string(),
            "-af".to_string(),
            format!(
                "atrim=duration={:.9}",
                count as f64 / f64::from(FRAMES_PER_SECOND)
            ),
        ]);
    }
    argv.extend([
        "-pix_fmt".to_string(),
        "yuv420p".to_string(),
        movie_path(frames).display().to_string(),
    ]);
    Ok(argv)
}

/// The ffmpeg command line for the composed movie: raw RGB on stdin, because
/// the frames have already been drawn on in this process.
pub fn pipe_line(
    frames: &Path,
    out: &Path,
    width: u32,
    height: u32,
    count: usize,
) -> Result<Vec<String>, String> {
    let checked_count = recording_frames(frames)?.len();
    let marker = frames.join("bench.jsonl");
    let marker_exists = marker
        .try_exists()
        .map_err(|error| format!("could not inspect {}: {error}", marker.display()))?;
    let count = if marker_exists {
        if checked_count != count {
            return Err(format!(
                "frame count changed under {}: validated {checked_count}, compose expected {count}",
                frames.display()
            ));
        }
        checked_count
    } else {
        count
    };
    let mut argv: Vec<String> = [
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
    ]
    .into_iter()
    .map(str::to_string)
    .collect();
    if let Some(audio) = audio_source(frames)? {
        argv.extend([
            "-i".to_string(),
            audio.display().to_string(),
            "-map".to_string(),
            "0:v:0".to_string(),
            "-map".to_string(),
            "1:a:0".to_string(),
            "-c:a".to_string(),
            "aac".to_string(),
            "-af".to_string(),
            format!(
                "atrim=duration={:.9}",
                count as f64 / f64::from(FRAMES_PER_SECOND)
            ),
        ]);
    }
    argv.extend([
        "-pix_fmt".to_string(),
        "yuv420p".to_string(),
        out.display().to_string(),
    ]);
    Ok(argv)
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

/// Validate the numbered frames of a new recording. Older offline directories
/// keep the original permissive frame discovery.
fn recording_frames(frames: &Path) -> Result<Vec<PathBuf>, String> {
    let marker = frames.join("bench.jsonl");
    let strict = marker
        .try_exists()
        .map_err(|error| format!("could not inspect {}: {error}", marker.display()))?;
    if !strict {
        return Ok(frame_files(frames));
    }

    let entries = std::fs::read_dir(frames)
        .map_err(|error| format!("could not list {}: {error}", frames.display()))?;
    let mut indexed = Vec::new();
    for entry in entries {
        let entry =
            entry.map_err(|error| format!("could not read {}: {error}", frames.display()))?;
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if !name
            .get(..5)
            .is_some_and(|prefix| prefix.eq_ignore_ascii_case("frame"))
        {
            continue;
        }
        let Some(digits) = name
            .strip_prefix("frame_")
            .and_then(|name| name.strip_suffix(".png"))
        else {
            return Err(format!(
                "invalid recording frame {}: expected frame_%06d.png",
                path.display()
            ));
        };
        if digits.len() != 6 || !digits.bytes().all(|digit| digit.is_ascii_digit()) {
            return Err(format!(
                "invalid recording frame {}: expected six decimal digits",
                path.display()
            ));
        }
        let index = digits
            .parse::<usize>()
            .map_err(|error| format!("invalid recording frame {}: {error}", path.display()))?;
        indexed.push((index, path));
    }
    indexed.sort_unstable_by_key(|(index, _)| *index);

    if indexed.is_empty() {
        return Err(format!(
            "no frames under {}: expected {}",
            frames.display(),
            frames.join("frame_000000.png").display()
        ));
    }
    let mut files = Vec::with_capacity(indexed.len());
    for (expected, (actual, path)) in indexed.into_iter().enumerate() {
        if actual != expected {
            let missing = frames.join(format!("frame_{expected:06}.png"));
            return Err(format!(
                "missing recording frame {} before {}; frames must be contiguous from zero",
                missing.display(),
                path.display()
            ));
        }
        files.push(path);
    }
    Ok(files)
}

/// How many frames the recorder left in the directory.
pub fn frame_count(frames: &Path) -> usize {
    frame_files(frames).len()
}

/// Stitch the frames untouched. The error names what went wrong: no frames,
/// no ffmpeg, or ffmpeg refusing, and carries the command line to run by hand.
pub fn stitch(frames: &Path) -> Result<Movie, String> {
    let count = recording_frames(frames)?.len();
    if count == 0 {
        return Err(format!("no frames under {}", frames.display()));
    }
    let argv = command_line_for_count(frames, count)?;
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
    let files = recording_frames(frames)?;
    let total = files.len();
    if total == 0 {
        return Err(format!("no frames under {}", frames.display()));
    }
    let film = rail::film(&rail::read(audit)?, total as u64);
    let first = read_frame(&files[0])?;
    let (width, height) = (first.width(), first.height());
    let overlay = Overlay::new(font, width)?;

    let argv = pipe_line(frames, out, width, height, total)?;
    let mut child = Command::new(&argv[0])
        .args(&argv[1..])
        .stdin(Stdio::piped())
        .spawn()
        .map_err(|error| {
            format!("could not run ffmpeg ({error}); it encodes the composed frames")
        })?;
    let tail = film.tail(total as u64);
    // Keep the encoder owned until every read and write finishes. A failure
    // mid-stream must close its pipe, kill it, and reap it before returning.
    let drawn = (|| -> Result<(), String> {
        let mut sink = child
            .stdin
            .take()
            .ok_or_else(|| "ffmpeg gave the composer no stdin".to_string())?;
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
        Ok(())
    })();
    if let Err(error) = drawn {
        let _ = child.kill();
        let _ = child.wait();
        return Err(error);
    }
    let status = child
        .wait()
        .map_err(|error| format!("ffmpeg did not finish: {error}"))?;
    if !status.success() {
        return Err(format!(
            "ffmpeg failed ({status}); command: {}",
            argv.join(" ")
        ));
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
/// A missing or unusable font falls back to a plain stitch, retaining audio.
/// Recording, encoding, and mux failures instead reach the run's caller.
pub fn report(bus: &Bus, frames: &Path, audit: &Path) -> Result<String, String> {
    let out = movie_path(frames);
    let font = default_font();
    let made = if let Err(error) = Overlay::new(&font, 1280) {
        bus.emit(BenchEvent::Note {
            text: format!("movie                no rail: {error}"),
        });
        stitch(frames)?
    } else {
        compose(audit, frames, &out, &font, |_, _| {})?
    };
    let line = format!("movie                {}", made.line());
    bus.emit(BenchEvent::Note { text: line.clone() });
    Ok(line)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_movie_sits_beside_its_frames_and_runs_one_tick_per_frame() {
        let frames = Path::new("/runs/rec-1");
        assert_eq!(movie_path(frames), PathBuf::from("/runs/rec-1.mp4"));
        let argv = command_line(frames).unwrap();
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
        let argv = pipe_line(
            Path::new("/nonexistent/frames"),
            Path::new("/runs/rec-1.mp4"),
            1280,
            720,
            120,
        )
        .unwrap();
        assert!(argv.contains(&"rawvideo".to_string()));
        assert!(argv.contains(&"1280x720".to_string()));
        assert!(argv.contains(&"rgb24".to_string()));
        assert_eq!(argv.last().unwrap(), "/runs/rec-1.mp4");
    }

    #[test]
    fn new_recordings_map_audio_and_trim_it_without_truncating_the_video_tail() {
        let dir = std::env::temp_dir().join(format!("nova-bench-audio-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("frame_000000.png"), b"").unwrap();
        std::fs::write(dir.join("frame_000001.png"), b"").unwrap();
        std::fs::write(dir.join("bench.jsonl"), b"").unwrap();
        let error = pipe_line(&dir, Path::new("/tmp/railed.mp4"), 1280, 720, 2).unwrap_err();
        assert!(error.contains("requires readable audio"), "{error}");
        assert!(command_line(&dir)
            .unwrap_err()
            .contains("requires readable audio"));
        assert!(stitch(&dir)
            .unwrap_err()
            .contains("requires readable audio"));
        std::fs::write(dir.join("bench.webm"), b"webm").unwrap();
        let audio = dir.join("bench.webm").display().to_string();
        for argv in [
            command_line(&dir).unwrap(),
            pipe_line(&dir, Path::new("/tmp/railed.mp4"), 1280, 720, 2).unwrap(),
        ] {
            let expected = [
                "-i",
                audio.as_str(),
                "-map",
                "0:v:0",
                "-map",
                "1:a:0",
                "-c:a",
                "aac",
                "-af",
                "atrim=duration=0.033333333",
            ];
            assert!(
                argv.windows(expected.len()).any(|args| args == expected),
                "{argv:?}"
            );
            assert!(
                !argv.iter().any(|arg| arg == "-shortest" || arg == "-t"),
                "{argv:?}"
            );
        }
        std::fs::remove_file(dir.join("bench.jsonl")).unwrap();
        std::fs::remove_file(dir.join("bench.webm")).unwrap();
        assert!(!command_line(&dir).unwrap().iter().any(|arg| arg == "-map"));
        assert!(!pipe_line(&dir, Path::new("/tmp/railed.mp4"), 1280, 720, 2)
            .unwrap()
            .iter()
            .any(|arg| arg == "-map"));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn failed_frame_reads_and_encoder_writes_reap_the_encoder() {
        let dir =
            std::env::temp_dir().join(format!("nova-bench-failed-encode-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let audit = dir.join("audit.jsonl");
        std::fs::write(&audit, b"").unwrap();
        let font = default_font();
        image::RgbImage::new(640, 360)
            .save(dir.join("frame_000000.png"))
            .unwrap();
        std::fs::write(dir.join("bench.jsonl"), b"").unwrap();
        let error = report(&Bus::quiet(), &dir, &audit).unwrap_err();
        assert!(error.contains("requires readable audio"), "{error}");
        std::fs::remove_file(dir.join("bench.jsonl")).unwrap();
        std::fs::write(dir.join("frame_000001.png"), b"broken PNG").unwrap();
        let error = compose(&audit, &dir, &dir.join("read.mp4"), &font, |_, _| {}).unwrap_err();
        assert!(error.contains("could not read"), "{error}");
        std::fs::remove_file(dir.join("frame_000001.png")).unwrap();
        let error = compose(
            &audit,
            &dir,
            &dir.join("missing/write.mp4"),
            &font,
            |_, _| {},
        )
        .unwrap_err();
        assert!(error.contains("ffmpeg"), "{error}");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn audio_recordings_reject_missing_and_malformed_frames_before_encoding() {
        let dir = std::env::temp_dir().join(format!("nova-bench-frame-gap-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("bench.jsonl"), b"").unwrap();
        std::fs::write(dir.join("frame_000000.png"), b"").unwrap();
        std::fs::write(dir.join("frame_000002.png"), b"").unwrap();

        let missing = dir.join("frame_000001.png").display().to_string();
        let error = stitch(&dir).unwrap_err();
        assert!(error.contains(&missing), "{error}");
        let error = pipe_line(&dir, &dir.join("movie.mp4"), 1280, 720, 1).unwrap_err();
        assert!(error.contains(&missing), "{error}");
        let error = compose(
            &dir.join("audit.jsonl"),
            &dir,
            &dir.join("movie.mp4"),
            &dir.join("missing-font.ttf"),
            |_, _| {},
        )
        .unwrap_err();
        assert!(error.contains(&missing), "{error}");

        std::fs::remove_file(dir.join("frame_000002.png")).unwrap();
        let error = pipe_line(&dir, &dir.join("movie.mp4"), 1280, 720, 2).unwrap_err();
        assert!(error.contains("frame count changed"), "{error}");
        let malformed = dir.join("frame_00000x.png");
        std::fs::write(&malformed, b"").unwrap();
        let error = stitch(&dir).unwrap_err();
        assert!(error.contains(&malformed.display().to_string()), "{error}");
        assert!(error.contains("invalid recording frame"), "{error}");

        std::fs::remove_file(malformed).unwrap();
        std::fs::remove_file(dir.join("frame_000000.png")).unwrap();
        let error = stitch(&dir).unwrap_err();
        assert!(error.contains("no frames"), "{error}");
        assert!(error.contains(&dir.join("frame_000000.png").display().to_string()));
        std::fs::remove_dir_all(&dir).unwrap();
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
