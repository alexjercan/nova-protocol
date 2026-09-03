//! The loop primitive: record every rendered frame between two step calls and
//! encode the run into a short VP9 webm - the moving-picture twin of
//! [`capture_window`](crate::capture::capture_window).
//!
//! Same idiom, no second one: a loop is authored inside an ordinary autopilot
//! script. A step's `on_enter` calls [`loop_start`], the following beats play
//! the action, and a later step calls [`loop_end`] and holds on
//! [`loop_written`](crate::predicate::loop_written) - act, frame, record, in
//! step order, exactly like a shot. `NOVA_CAPTURE` arms it; unarmed, both
//! calls are no-ops and the same script is the smoke path.
//!
//! While a loop is open, [`LoopCapturePlugin`]'s driver requests one
//! primary-window readback per rendered frame (the same GPU path a screenshot
//! takes) into a staging folder of numbered PNGs under
//! [`CAPTURE_DIR_ENV`](crate::capture::CAPTURE_DIR_ENV). [`loop_end`] stops the
//! recording, waits for every in-flight readback to land, encodes the frames
//! with ffmpeg and acks `<name>.webm` into
//! [`CaptureLog`](crate::capture::CaptureLog) only once the file is on disk.
//!
//! ## One profile holds every size and rate
//!
//! The capture window, the encoded resolution, the frame clock, the quality
//! and the frame cap are ONE value: [`LoopProfile`]. The docs fleet takes
//! [`LoopCapturePlugin::default`] (the profile the [`LOOP_FPS`] family
//! documents); a production caller that needs another framing - a portrait
//! short, say - passes its own to [`LoopCapturePlugin::new`] and changes
//! nothing else:
//!
//! ```no_run
//! # use bevy::prelude::*;
//! # use nova_autopilot::loops::{LoopCapturePlugin, LoopProfile};
//! # fn add(app: &mut App) {
//! app.add_plugins(LoopCapturePlugin::new(LoopProfile {
//!     window_resolution: (1080, 1920),
//!     output_resolution: (1080, 1920),
//!     fps: 60,
//!     crf: 18,
//!     frame_cap: 1200,
//! }));
//! # }
//! ```
//!
//! The profile is inserted as a resource ARMED OR NOT, because the capture
//! window is dressing rather than recording: the harness system that sizes the
//! primary window reads it, so the smoke path renders the framing the capture
//! will. Only the recording itself and the frame clock are behind the arming
//! gate.
//!
//! ## Cadence: the armed run is frame-clocked
//!
//! An armed run pins [`TimeUpdateStrategy::ManualDuration`] to `1 /
//! [`LoopProfile::fps`]` seconds, so every rendered frame IS one profile frame
//! of game time and the encoded webm plays back at real speed whatever the
//! capture host's render rate was. Without the pin a software renderer would
//! stretch one sim-second over a handful of frames and the loop would play
//! back several times too fast. The pin also makes re-capture deterministic:
//! the same script produces the same frames. It pins the REAL clock, not only
//! the virtual one, so inside a loop a step deadline and the run-level
//! backstop in [`completion`](crate::completion) both count rendered frames
//! over [`LoopProfile::fps`] rather than wall seconds: budget a beat inside a
//! capture in frames, and expect a slow host to take longer in the room than
//! the deadline says. The default is 30 fps rather than
//! 60 on purpose - it halves the readback and encode cost, gives the default
//! frame cap a 20-second runway, and a web docs loop does not need more.
//!
//! ## Failing loudly
//!
//! A loop that goes wrong must fail the RUN, never truncate or silently drop:
//!
//! - a run whose collectors all finish while a loop is still open (the script
//!   forgot [`loop_end`]) error-exits naming the loop;
//! - a loop that exceeds the profile's frame cap error-exits naming the loop
//!   and the cap, rather than clipping the webm;
//! - a missing ffmpeg binary or a nonzero encode exit error-exits with
//!   ffmpeg's own output.
//!
//! Per the completion protocol these are ABORTS ([`AppExit::error`] directly);
//! the plugin's registered collector never reports done for a failed run.

use std::{
    path::{Path, PathBuf},
    process::Command,
    time::Duration,
};

use bevy::{
    prelude::*,
    render::view::screenshot::{save_to_disk, Screenshot, ScreenshotCaptured},
    time::TimeUpdateStrategy,
};

use crate::{
    capture::{self, CaptureLog},
    completion::{self, AutopilotCompletionSystems, HarnessCompletion},
};

/// Collector name the loop recorder registers under.
pub const LOOP_CAPTURE: &str = "loop_capture";

/// Default frames per second a loop is recorded and encoded at. The armed
/// run's frame clock is pinned to `1 / fps`, so this is the capture cadence
/// AND the playback rate - see the module docs on why 30.
pub const LOOP_FPS: u32 = 30;

/// Default per-loop frame cap: 600 frames is 20 seconds at [`LOOP_FPS`], well
/// over the 4-8 seconds a docs loop wants. EXCEEDING IT FAILS THE RUN - a
/// loop that long is an authoring bug (a missed end condition), and a
/// silently truncated webm would hide it.
pub const LOOP_FRAME_CAP: u32 = 600;

/// Default resolution the webm is scaled to at encode time. Capture happens at
/// the profile's window size (the fleet pins 1920x1080); 720p is plenty for an
/// inline docs figure and roughly quarters the bitrate.
pub const LOOP_RESOLUTION: (u32, u32) = (1280, 720);

/// Default libvpx-vp9 constant-quality CRF. At 720p30 a 4-8 second space scene
/// lands far under the 2-3 MB budget here (the pilots measured ~200 KB at CRF
/// 40, so this spends some of that headroom on quality); raise it if a busier
/// loop ever crowds the budget.
pub const LOOP_CRF: u32 = 34;

/// Every size and rate one loop capture runs at: the window it renders into,
/// the resolution it encodes to, the frame clock, the quality and the cap.
///
/// [`LoopProfile::default`] is the documentation-loop profile the example
/// fleet captures with - the [`LOOP_FPS`] family IS its field list. A caller
/// selects another framing by building the whole struct, so a profile is
/// always readable as one complete answer rather than a default plus edits.
///
/// The plugin inserts it as a resource armed or not (see the module docs), so
/// the harness sizes the primary window from the same value the encoder reads.
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq)]
pub struct LoopProfile {
    /// Pixel size the primary capture window is pinned to, and therefore the
    /// size every readback arrives at.
    pub window_resolution: (u32, u32),
    /// Pixel size the webm is encoded at. Equal to `window_resolution` the
    /// encode carries no scale filter at all.
    pub output_resolution: (u32, u32),
    /// Frames per second: the manual frame clock AND the encoded cadence.
    pub fps: u32,
    /// libvpx-vp9 constant-quality CRF. Lower is better and bigger.
    pub crf: u32,
    /// Recorded frames one loop may reach before the run ABORTS.
    pub frame_cap: u32,
}

impl Default for LoopProfile {
    fn default() -> Self {
        Self {
            window_resolution: capture::CAPTURE_RESOLUTION,
            output_resolution: LOOP_RESOLUTION,
            fps: LOOP_FPS,
            crf: LOOP_CRF,
            frame_cap: LOOP_FRAME_CAP,
        }
    }
}

impl LoopProfile {
    /// Check the invariants a recorder cannot work around: a zero dimension,
    /// a zero frame clock or a zero cap. The error NAMES the field, because
    /// this is read while a caller is writing the profile.
    ///
    /// Nothing else is bounded here. A production caller choosing 4K60 is
    /// making a cost decision, not a mistake, and the frame cap is the knob
    /// that already fails a runaway loop.
    pub fn validate(&self) -> Result<(), String> {
        let zero = |(width, height): (u32, u32)| width == 0 || height == 0;
        if zero(self.window_resolution) {
            return Err(format!(
                "window_resolution {:?} has a zero dimension",
                self.window_resolution
            ));
        }
        if zero(self.output_resolution) {
            return Err(format!(
                "output_resolution {:?} has a zero dimension",
                self.output_resolution
            ));
        }
        if self.fps == 0 {
            return Err("fps is zero - a loop needs a frame clock".to_string());
        }
        if self.frame_cap == 0 {
            return Err("frame_cap is zero - no loop could record a frame".to_string());
        }
        Ok(())
    }

    /// The manual frame duration an armed run pins its clock to.
    pub fn frame_duration(&self) -> Duration {
        Duration::from_secs_f64(1.0 / f64::from(self.fps))
    }

    /// Seconds `frames` of this profile play back as.
    fn seconds(&self, frames: u32) -> f32 {
        frames as f32 / self.fps as f32
    }
}

/// The webm file name a loop acks and writes: `<name>.webm`.
pub fn loop_file_name(name: &str) -> String {
    format!("{name}.webm")
}

/// Where a loop's numbered staging frames go, under the shot dir.
fn staging_dir(name: &str) -> PathBuf {
    capture::capture_path(&format!(".loop-frames/{name}"))
}

/// What the recorder is doing right now.
#[derive(Default, Debug, Clone, PartialEq, Eq)]
enum LoopPhase {
    /// No loop open. The collector reports done from here once every other
    /// collector has finished.
    #[default]
    Idle,
    /// A loop is open: one frame readback is requested per rendered frame.
    Recording(String),
    /// [`loop_end`] was called: no new frames, waiting for the in-flight
    /// readbacks to land before encoding.
    Draining(String),
    /// A failure was reported and [`AppExit::error`] written. Inert so the
    /// exiting frame does not stack further errors.
    Failed,
}

/// The loop recorder's state: the driver of [`LoopCapturePlugin`] walks it,
/// [`loop_start`] / [`loop_end`] transition it, and the per-frame capture
/// observers count into it.
#[derive(Resource, Default)]
pub struct LoopRecorder {
    phase: LoopPhase,
    /// Where the open loop's frames stage.
    staging: PathBuf,
    /// Frames requested for the open loop (1-based; also the last frame's
    /// number in the staging dir).
    requested: u32,
    /// Frames whose PNG write has completed.
    written: u32,
    /// Whether the collector has reported done (guards double reporting).
    reported_done: bool,
}

impl LoopRecorder {
    /// Open the loop `name`, staging into `staging`. Clears any stale frames
    /// from an earlier run of the same loop - leftovers past this run's
    /// frame count would otherwise be encoded into the tail of the webm.
    fn start(&mut self, name: &str, staging: PathBuf) -> Result<(), String> {
        if name.is_empty() || name.contains(['/', '\\']) {
            return Err(format!(
                "loop capture: `{name}` is not a loop name (empty or contains a path separator)"
            ));
        }
        match &self.phase {
            LoopPhase::Idle => {
                if staging.exists() {
                    std::fs::remove_dir_all(&staging).map_err(|error| {
                        format!("loop capture: `{name}` could not clear stale staging: {error}")
                    })?;
                }
                std::fs::create_dir_all(&staging).map_err(|error| {
                    format!("loop capture: `{name}` could not create staging: {error}")
                })?;
                self.phase = LoopPhase::Recording(name.to_string());
                self.staging = staging;
                self.requested = 0;
                self.written = 0;
                Ok(())
            }
            LoopPhase::Recording(open) | LoopPhase::Draining(open) => Err(format!(
                "loop capture: loop_start(\"{name}\") while `{open}` is still open"
            )),
            LoopPhase::Failed => Err(format!(
                "loop capture: loop_start(\"{name}\") after an earlier loop failure"
            )),
        }
    }

    /// Close the loop `name`: stop recording and move to the drain that
    /// precedes the encode.
    fn end(&mut self, name: &str) -> Result<(), String> {
        match &self.phase {
            LoopPhase::Recording(open) if open == name => {
                if self.requested == 0 {
                    return Err(format!(
                        "loop capture: loop_end(\"{name}\") with zero frames recorded"
                    ));
                }
                self.phase = LoopPhase::Draining(name.to_string());
                Ok(())
            }
            LoopPhase::Recording(open) => Err(format!(
                "loop capture: loop_end(\"{name}\") but the open loop is `{open}`"
            )),
            LoopPhase::Idle => Err(format!(
                "loop capture: loop_end(\"{name}\") with no loop open"
            )),
            LoopPhase::Draining(open) => Err(format!(
                "loop capture: loop_end(\"{name}\") but `{open}` is already ending"
            )),
            LoopPhase::Failed => Err(format!(
                "loop capture: loop_end(\"{name}\") after an earlier loop failure"
            )),
        }
    }
}

/// Env-gated recorder plugin, carrying the [`LoopProfile`] the whole capture
/// runs at. A loop-capturing example adds it unconditionally; unarmed (no
/// `NOVA_CAPTURE`) it records NOTHING and keeps real time, so the smoke path
/// pays only the window sizing.
///
/// Armed, it pins the frame clock (see the module docs), registers the
/// [`LOOP_CAPTURE`] completion collector and adds the per-frame driver. It
/// expects a scripted autopilot in the same run: "the run is complete" is
/// read from the completion protocol's other collectors.
#[derive(Default)]
pub struct LoopCapturePlugin {
    profile: LoopProfile,
}

impl LoopCapturePlugin {
    /// Record at `profile` instead of the documentation-loop default.
    pub fn new(profile: LoopProfile) -> Self {
        Self { profile }
    }
}

impl Plugin for LoopCapturePlugin {
    fn build(&self, app: &mut App) {
        // A bad profile is a build-time authoring error, and the run it would
        // produce is unusable (a zero dimension is not an encodable frame), so
        // it stops the app here rather than at the first readback.
        if let Err(error) = self.profile.validate() {
            panic!("loop capture: invalid LoopProfile: {error}");
        }
        // Inserted before the gate: the window size is dressing the smoke path
        // wants too, and being a plugin-build insert it is already there when
        // any `Startup` system reads it.
        app.insert_resource(self.profile);
        if !capture::capturing() {
            return;
        }
        arm_loop_capture(app, self.profile);
        // One rendered frame = one loop frame. Pinned only on the ARMED run:
        // the smoke path keeps wall time.
        app.insert_resource(TimeUpdateStrategy::ManualDuration(
            self.profile.frame_duration(),
        ));
    }
}

/// The armed wiring, split from the env gate so the driver is testable
/// without mutating the process environment (which would race the other
/// tests in this binary).
fn arm_loop_capture(app: &mut App, profile: LoopProfile) {
    app.insert_resource(profile);
    app.init_resource::<LoopRecorder>();
    app.init_resource::<CaptureLog>();
    completion::register(app, LOOP_CAPTURE);
    // Before the watcher, so a failure exit and the done report land in the
    // same frame they are decided.
    app.add_systems(
        Last,
        loop_capture_drive.before(AutopilotCompletionSystems::Watch),
    );
}

/// Open the loop `name` from a step's `on_enter`. Recording covers every
/// rendered frame from this step through the step that calls [`loop_end`].
/// A no-op on the smoke path.
///
/// Requires [`LoopCapturePlugin`]; an armed run without it is a hard failure
/// rather than a silently unrecorded loop.
pub fn loop_start(world: &mut World, name: &str) {
    if !capture::capturing() {
        return;
    }
    if world.get_resource::<LoopRecorder>().is_none() {
        fail(
            world,
            &format!("loop capture: loop_start(\"{name}\") without LoopCapturePlugin"),
        );
        return;
    }
    let staging = staging_dir(name);
    let result = world.resource_mut::<LoopRecorder>().start(name, staging);
    match result {
        Ok(()) => info!("loop capture: `{name}` opens"),
        Err(message) => fail(world, &message),
    }
}

/// Close the loop `name` from a step's `on_enter`: stop recording, and once
/// every in-flight frame has landed, encode `<name>.webm` and ack it into
/// [`CaptureLog`]. The closing step holds on
/// [`loop_written`](crate::predicate::loop_written), which is an await of
/// that ack, not a guess. A no-op on the smoke path.
pub fn loop_end(world: &mut World, name: &str) {
    if !capture::capturing() {
        return;
    }
    if world.get_resource::<LoopRecorder>().is_none() {
        fail(
            world,
            &format!("loop capture: loop_end(\"{name}\") without LoopCapturePlugin"),
        );
        return;
    }
    let result = world.resource_mut::<LoopRecorder>().end(name);
    match result {
        Ok(()) => info!("loop capture: `{name}` closes, draining in-flight frames"),
        Err(message) => fail(world, &message),
    }
}

/// Report a loop failure: log it, write the ERROR exit (an abort does not
/// negotiate with the other collectors) and park the recorder so the exiting
/// frames stay quiet.
fn fail(world: &mut World, message: &str) {
    error!("{message}");
    if let Some(mut recorder) = world.get_resource_mut::<LoopRecorder>() {
        recorder.phase = LoopPhase::Failed;
    }
    world.write_message(AppExit::error());
}

/// Per-frame driver, in `Last`: request the open loop's next frame, encode a
/// drained loop, report the collector done from idle, and turn every wrong
/// state into a named error exit.
fn loop_capture_drive(world: &mut World) {
    // "The run is complete" = no OTHER collector is pending. The scripted
    // autopilot holds its collector until its last step, so while the script
    // runs this is false.
    let run_complete = world
        .get_resource::<HarnessCompletion>()
        .is_some_and(|completion| !completion.others_pending(LOOP_CAPTURE));

    let phase = world.resource::<LoopRecorder>().phase.clone();
    match phase {
        LoopPhase::Recording(name) => {
            if run_complete {
                fail(
                    world,
                    &format!(
                        "loop capture: run completed with loop `{name}` still open - \
                         the script never called loop_end(\"{name}\")"
                    ),
                );
                return;
            }
            let profile = *world.resource::<LoopProfile>();
            let requested = world.resource::<LoopRecorder>().requested;
            if requested >= profile.frame_cap {
                let cap = profile.frame_cap;
                fail(
                    world,
                    &format!(
                        "loop capture: loop `{name}` exceeded the {cap}-frame cap \
                         ({} seconds at {} fps) - shorten the loop, do not raise the cap",
                        cap / profile.fps,
                        profile.fps
                    ),
                );
                return;
            }
            let frame = {
                let mut recorder = world.resource_mut::<LoopRecorder>();
                recorder.requested += 1;
                recorder.requested
            };
            let path = world
                .resource::<LoopRecorder>()
                .staging
                .join(format!("frame_{frame:05}.png"));
            request_frame(world, path);
        }
        LoopPhase::Draining(name) => {
            let (written, requested) = {
                let recorder = world.resource::<LoopRecorder>();
                (recorder.written, recorder.requested)
            };
            if written < requested {
                return;
            }
            let profile = *world.resource::<LoopProfile>();
            let staging = world.resource::<LoopRecorder>().staging.clone();
            let file = loop_file_name(&name);
            let output = capture::capture_path(&file);
            match encode_frames("ffmpeg", &profile, &staging, &output) {
                Ok(()) => {
                    if let Err(error) = std::fs::remove_dir_all(&staging) {
                        warn!("loop capture: could not clean staging {staging:?}: {error}");
                    }
                    world.resource_mut::<CaptureLog>().mark(&file);
                    world.resource_mut::<LoopRecorder>().phase = LoopPhase::Idle;
                    info!(
                        "loop capture: `{name}` written: {requested} frames -> {output:?} \
                         ({:.1}s at {} fps)",
                        profile.seconds(requested),
                        profile.fps
                    );
                }
                Err(message) => {
                    fail(
                        world,
                        &format!("loop capture: loop `{name}` failed to encode: {message}"),
                    );
                }
            }
        }
        LoopPhase::Idle => {
            if run_complete && !world.resource::<LoopRecorder>().reported_done {
                world.resource_mut::<LoopRecorder>().reported_done = true;
                world.resource_mut::<HarnessCompletion>().done(LOOP_CAPTURE);
            }
        }
        LoopPhase::Failed => {}
    }
}

/// Request one frame readback into `path` - the same asynchronous
/// `Screenshot` path [`capture_window`](crate::capture::capture_window)
/// takes. The write count is chained BEHIND the save in one observer for the
/// same reason the shot ack is: `save_to_disk` writes synchronously inside
/// its closure, so by the time the count moves the PNG is on disk.
fn request_frame(world: &mut World, path: PathBuf) {
    let mut save = save_to_disk(path);
    world.spawn(Screenshot::primary_window()).observe(
        move |captured: On<ScreenshotCaptured>, mut recorder: ResMut<LoopRecorder>| {
            save(captured);
            recorder.written += 1;
        },
    );
}

/// The full ffmpeg argument list for one loop encode: numbered staging PNGs
/// in, a VP9 webm at the profile's output resolution and CRF out. Split from
/// [`encode_frames`] so the command line is assertable without running
/// anything.
///
/// A profile whose output matches its window carries NO scale filter: the
/// frames are already that size, and resampling them to themselves would only
/// soften a source that is meant to be the master.
fn encode_args(profile: &LoopProfile, staging: &Path, output: &Path) -> Vec<std::ffi::OsString> {
    let input = staging.join("frame_%05d.png");
    let mut args: Vec<std::ffi::OsString> = vec![
        "-y".into(),
        "-framerate".into(),
        profile.fps.to_string().into(),
        "-start_number".into(),
        "1".into(),
        "-i".into(),
        input.into_os_string(),
    ];
    if profile.output_resolution != profile.window_resolution {
        args.push("-vf".into());
        args.push(
            format!(
                "scale={}:{}:flags=lanczos",
                profile.output_resolution.0, profile.output_resolution.1
            )
            .into(),
        );
    }
    args.extend([
        "-c:v".into(),
        "libvpx-vp9".into(),
        "-crf".into(),
        profile.crf.to_string().into(),
        "-b:v".into(),
        "0".into(),
        "-row-mt".into(),
        "1".into(),
        "-deadline".into(),
        "good".into(),
        "-cpu-used".into(),
        "2".into(),
        "-pix_fmt".into(),
        "yuv420p".into(),
        "-an".into(),
        output.as_os_str().to_os_string(),
    ]);
    args
}

/// Run `ffmpeg` over the staging frames, blocking until it exits. A launch
/// failure (the binary is not on PATH) and a nonzero exit are both errors
/// carrying ffmpeg's own output; so is an exit that left no file behind.
fn encode_frames(
    ffmpeg: &str,
    profile: &LoopProfile,
    staging: &Path,
    output: &Path,
) -> Result<(), String> {
    if let Some(parent) = output.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)
                .map_err(|error| format!("could not create {parent:?}: {error}"))?;
        }
    }
    let result = Command::new(ffmpeg)
        .args(encode_args(profile, staging, output))
        .output()
        .map_err(|error| format!("could not run `{ffmpeg}`: {error} (is ffmpeg installed?)"))?;
    if !result.status.success() {
        return Err(format!(
            "`{ffmpeg}` exited with {}: {}",
            result.status,
            String::from_utf8_lossy(&result.stderr)
        ));
    }
    if !output.exists() {
        return Err(format!(
            "`{ffmpeg}` exited cleanly but wrote no file at {output:?}"
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use bevy::ecs::schedule::SingleThreadedExecutor;

    use super::*;
    use crate::log_capture::capturing_logs;

    fn temp_staging(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("nova-loop-test-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    /// The armed rig, built WITHOUT touching the env (the plugin's gate would
    /// race the other tests in this binary): minimal app plus the same wiring
    /// `arm_loop_capture` gives a real armed run, executing on this thread so
    /// the log capture sees the driver.
    fn armed_app() -> App {
        armed_app_with(LoopProfile::default())
    }

    /// The same rig at a chosen profile, for the tests that assert a
    /// production caller's settings reach the driver.
    fn armed_app_with(profile: LoopProfile) -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        arm_loop_capture(&mut app, profile);
        app.edit_schedule(Last, |schedule| {
            schedule.set_executor(SingleThreadedExecutor::new());
        });
        app
    }

    /// The portrait short `nova-showcase` captures with: a real 9:16 master,
    /// no scale step, twice the cadence and twice the runway.
    fn portrait() -> LoopProfile {
        LoopProfile {
            window_resolution: (1080, 1920),
            output_resolution: (1080, 1920),
            fps: 60,
            crf: 18,
            frame_cap: 1200,
        }
    }

    fn exits(app: &mut App) -> Vec<AppExit> {
        app.world_mut()
            .resource_mut::<Messages<AppExit>>()
            .drain()
            .collect()
    }

    fn open_loop(app: &mut App, name: &str, tag: &str) {
        let staging = temp_staging(tag);
        app.world_mut()
            .resource_mut::<LoopRecorder>()
            .start(name, staging)
            .expect("the loop opens from idle");
    }

    /// The recorder's transitions, straight on the state machine: the calls a
    /// script can get wrong are errors that NAME the loop, never silent
    /// no-ops.
    #[test]
    fn the_recorder_rejects_out_of_order_calls_by_name() {
        let mut recorder = LoopRecorder::default();
        assert!(
            recorder.end("early").unwrap_err().contains("no loop open"),
            "ending before any start is an error"
        );

        recorder
            .start("first", temp_staging("transitions"))
            .expect("a loop opens from idle");
        assert!(
            recorder
                .start("second", temp_staging("transitions-second"))
                .unwrap_err()
                .contains("`first` is still open"),
            "a second start while one is open names the open loop"
        );
        assert!(
            recorder.end("other").unwrap_err().contains("`first`"),
            "ending the wrong name names the open loop"
        );

        recorder.requested = 3;
        recorder.end("first").expect("the open loop closes");
        assert_eq!(recorder.phase, LoopPhase::Draining("first".into()));
        assert!(
            recorder
                .end("first")
                .unwrap_err()
                .contains("already ending"),
            "a double end is an error, not a shrug"
        );
    }

    #[test]
    fn a_loop_name_is_not_a_path_and_a_loop_is_not_empty() {
        let mut recorder = LoopRecorder::default();
        assert!(recorder.start("", temp_staging("empty")).is_err());
        assert!(recorder.start("a/b", temp_staging("slash")).is_err());

        recorder
            .start("ok", temp_staging("zero-frames"))
            .expect("a valid name opens");
        assert!(
            recorder.end("ok").unwrap_err().contains("zero frames"),
            "closing a loop that recorded nothing is an error"
        );
    }

    /// Opening a loop clears STALE frames from an earlier run: a leftover
    /// numbered PNG past this run's count would silently join the encode.
    #[test]
    fn opening_a_loop_clears_stale_staging_frames() {
        let staging = temp_staging("stale");
        std::fs::create_dir_all(&staging).unwrap();
        let stale = staging.join("frame_00099.png");
        std::fs::write(&stale, b"stale").unwrap();

        let mut recorder = LoopRecorder::default();
        recorder.start("fresh", staging.clone()).unwrap();

        assert!(staging.exists(), "the staging dir itself is recreated");
        assert!(!stale.exists(), "the stale frame is gone");
        let _ = std::fs::remove_dir_all(&staging);
    }

    /// Failure (a): every other collector done while a loop is still open
    /// means the script forgot `loop_end` - an ERROR exit naming the loop,
    /// not a passing run minus one webm.
    #[test]
    fn a_loop_still_open_at_run_completion_fails_the_run() {
        let (observed, logs) = capturing_logs(|| {
            let mut app = armed_app();
            // No other collector at all: the "run" is complete immediately,
            // which is exactly the state after a script's driver reports done.
            open_loop(&mut app, "forgotten", "open-at-completion");
            app.update();
            exits(&mut app)
        });
        assert_eq!(observed.len(), 1, "the failure exits exactly once");
        assert_ne!(observed[0], AppExit::Success);
        assert!(
            logs.contains("forgotten") && logs.contains("loop_end"),
            "the failure names the loop and the missing call; logged: {logs}"
        );
    }

    /// While the script is still running (another collector pending), an open
    /// loop records: one frame readback per driven frame, no exit.
    #[test]
    fn an_open_loop_requests_one_frame_per_driven_frame() {
        let mut app = armed_app();
        completion::register(&mut app, completion::AUTOPILOT);
        open_loop(&mut app, "live", "recording");

        app.update();
        app.update();
        app.update();

        assert!(exits(&mut app).is_empty(), "recording is not an exit");
        assert_eq!(app.world().resource::<LoopRecorder>().requested, 3);
        let spawned = app
            .world_mut()
            .query::<&Screenshot>()
            .iter(app.world())
            .count();
        assert_eq!(spawned, 3, "each request is a real readback entity");
    }

    /// Failure (b): the frame cap ABORTS the run rather than truncating the
    /// loop.
    #[test]
    fn exceeding_the_frame_cap_fails_the_run() {
        let (observed, logs) = capturing_logs(|| {
            let mut app = armed_app();
            completion::register(&mut app, completion::AUTOPILOT);
            open_loop(&mut app, "runaway", "cap");
            app.world_mut().resource_mut::<LoopRecorder>().requested = LOOP_FRAME_CAP;
            app.update();
            exits(&mut app)
        });
        assert_eq!(observed.len(), 1);
        assert_ne!(observed[0], AppExit::Success, "a capped loop is an ABORT");
        assert!(
            logs.contains("runaway") && logs.contains(&LOOP_FRAME_CAP.to_string()),
            "the failure names the loop and the cap; logged: {logs}"
        );
    }

    /// ...and the cap it enforces is the PROFILE's, not the default: a
    /// production loop with a longer runway records past 600 frames and aborts
    /// at its own number.
    #[test]
    fn the_frame_cap_enforced_is_the_profile_cap() {
        let (observed, logs) = capturing_logs(|| {
            let mut app = armed_app_with(portrait());
            completion::register(&mut app, completion::AUTOPILOT);
            open_loop(&mut app, "short", "profile-cap");

            app.world_mut().resource_mut::<LoopRecorder>().requested = LOOP_FRAME_CAP;
            app.update();
            let past_the_default = exits(&mut app).is_empty();

            app.world_mut().resource_mut::<LoopRecorder>().requested = portrait().frame_cap;
            app.update();
            (past_the_default, exits(&mut app))
        });
        assert!(
            observed.0,
            "the default cap does not stop a profile that raised it"
        );
        assert_eq!(observed.1.len(), 1);
        assert_ne!(observed.1[0], AppExit::Success);
        assert!(
            logs.contains("short") && logs.contains("1200-frame cap") && logs.contains("60 fps"),
            "the failure names the profile's cap and cadence; logged: {logs}"
        );
    }

    /// An invalid profile stops the app while it is being BUILT, not at the
    /// first readback of a run that already spent minutes getting there.
    #[test]
    #[should_panic(expected = "fps is zero")]
    fn an_invalid_profile_panics_the_plugin_build() {
        App::new().add_plugins(LoopCapturePlugin::new(LoopProfile {
            fps: 0,
            ..LoopProfile::default()
        }));
    }

    /// The unarmed path stays a smoke walk: the plugin publishes the profile
    /// (the window dressing wants it) and nothing else - no recorder, no
    /// manual clock, and the step calls write no frames.
    #[test]
    fn an_unarmed_run_publishes_the_profile_and_records_nothing() {
        // The test binary never sets `NOVA_CAPTURE`, so this IS the unarmed
        // branch by construction.
        assert!(
            !capture::capturing(),
            "the test binary must not arm capture"
        );

        let mut app = App::new();
        app.add_plugins(LoopCapturePlugin::new(portrait()));
        assert_eq!(
            *app.world().resource::<LoopProfile>(),
            portrait(),
            "the window resolution is available to the smoke path too"
        );
        assert!(
            app.world().get_resource::<LoopRecorder>().is_none(),
            "an unarmed run has no recorder"
        );
        assert!(
            app.world().get_resource::<TimeUpdateStrategy>().is_none(),
            "an unarmed run keeps wall time"
        );

        loop_start(app.world_mut(), "quiet");
        loop_end(app.world_mut(), "quiet");
        let requests = app
            .world_mut()
            .query::<&Screenshot>()
            .iter(app.world())
            .count();
        assert_eq!(requests, 0, "an unarmed loop spawns no readback");
    }

    /// Failure (c), the launch half: a missing ffmpeg binary is an error
    /// carrying the binary's name, not a hang or a bare unwrap.
    #[test]
    fn a_missing_ffmpeg_binary_is_a_named_error() {
        let staging = temp_staging("no-ffmpeg");
        std::fs::create_dir_all(&staging).unwrap();
        let error = encode_frames(
            "/nonexistent/ffmpeg-for-the-loop-test",
            &LoopProfile::default(),
            &staging,
            &staging.join("out.webm"),
        )
        .unwrap_err();
        assert!(
            error.contains("ffmpeg-for-the-loop-test") && error.contains("could not run"),
            "the error names the binary: {error}"
        );
        let _ = std::fs::remove_dir_all(&staging);
    }

    fn command_line(profile: &LoopProfile) -> String {
        encode_args(
            profile,
            Path::new("/stage/torpedo"),
            Path::new("/shots/torpedo.webm"),
        )
        .iter()
        .map(|arg| arg.to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join(" ")
    }

    /// The encode command is the documented one: numbered PNGs at the loop
    /// cadence in, scaled constant-quality VP9 out.
    #[test]
    fn the_encode_command_pins_cadence_scale_and_quality() {
        let line = command_line(&LoopProfile::default());
        assert!(line.contains(&format!("-framerate {LOOP_FPS}")));
        assert!(line.contains("/stage/torpedo/frame_%05d.png"));
        assert!(line.contains(&format!(
            "scale={}:{}",
            LOOP_RESOLUTION.0, LOOP_RESOLUTION.1
        )));
        assert!(line.contains("libvpx-vp9"));
        assert!(line.contains(&format!("-crf {LOOP_CRF} -b:v 0")));
        assert!(line.ends_with("/shots/torpedo.webm"));
    }

    /// The default profile IS the documented docs-loop capture, so a fleet
    /// example that names nothing still records 1080p -> 720p30 at CRF 34.
    #[test]
    fn the_default_profile_is_the_documentation_loop_capture() {
        let profile = LoopProfile::default();
        assert_eq!(profile.window_resolution, (1920, 1080));
        assert_eq!(profile.output_resolution, (1280, 720));
        assert_eq!(profile.fps, 30);
        assert_eq!(profile.crf, 34);
        assert_eq!(profile.frame_cap, 600);
        assert_eq!(profile, LoopCapturePlugin::default().profile);
    }

    /// A production profile reaches ffmpeg whole: its cadence, its quality,
    /// and - because the window already IS the output - no scale filter to
    /// resample a master through.
    #[test]
    fn a_portrait_profile_encodes_at_its_own_size_without_a_scale_step() {
        let line = command_line(&portrait());
        assert!(
            line.contains("-framerate 60"),
            "the profile's cadence: {line}"
        );
        assert!(
            line.contains("-crf 18 -b:v 0"),
            "the profile's quality: {line}"
        );
        assert!(
            !line.contains("scale="),
            "window == output must not resample: {line}"
        );

        let downscaled = LoopProfile {
            output_resolution: (540, 960),
            ..portrait()
        };
        assert!(
            command_line(&downscaled).contains("scale=540:960:flags=lanczos"),
            "a different output resolution still scales"
        );
    }

    /// The armed frame clock comes from the profile, so a 60 fps capture
    /// advances game time in 60ths and plays back at real speed.
    #[test]
    fn the_manual_frame_duration_comes_from_the_profile_fps() {
        assert_eq!(
            LoopProfile::default().frame_duration(),
            Duration::from_secs_f64(1.0 / 30.0)
        );
        assert_eq!(
            portrait().frame_duration(),
            Duration::from_secs_f64(1.0 / 60.0)
        );
    }

    /// Validation names the field a caller got wrong, and every zero is a
    /// rejection rather than a division or an unencodable frame later on.
    #[test]
    fn a_zero_valued_profile_is_rejected_by_name() {
        LoopProfile::default()
            .validate()
            .expect("the documented default is valid");
        portrait()
            .validate()
            .expect("a production profile is valid");

        let cases = [
            (
                LoopProfile {
                    window_resolution: (0, 1080),
                    ..LoopProfile::default()
                },
                "window_resolution",
            ),
            (
                LoopProfile {
                    output_resolution: (1280, 0),
                    ..LoopProfile::default()
                },
                "output_resolution",
            ),
            (
                LoopProfile {
                    fps: 0,
                    ..LoopProfile::default()
                },
                "fps",
            ),
            (
                LoopProfile {
                    frame_cap: 0,
                    ..LoopProfile::default()
                },
                "frame_cap",
            ),
        ];
        for (profile, field) in cases {
            let error = profile.validate().unwrap_err();
            assert!(error.contains(field), "the error names `{field}`: {error}");
        }
    }

    /// The collector negotiates: idle at run completion reports done exactly
    /// once and the watcher exits Success.
    #[test]
    fn an_idle_recorder_reports_done_and_the_run_exits_clean() {
        let mut app = armed_app();
        completion::register(&mut app, completion::AUTOPILOT);
        app.update();
        assert!(
            exits(&mut app).is_empty(),
            "no exit while the script still runs"
        );

        app.world_mut()
            .resource_mut::<HarnessCompletion>()
            .done(completion::AUTOPILOT);
        app.update();
        assert_eq!(
            exits(&mut app),
            vec![AppExit::Success],
            "idle recorder + finished script = negotiated success"
        );
    }
}

/// The loop calls, their plugin, the capture profile and its defaults.
pub mod prelude {
    pub use super::{
        loop_end, loop_file_name, loop_start, LoopCapturePlugin, LoopProfile, LoopRecorder,
        LOOP_CAPTURE, LOOP_CRF, LOOP_FPS, LOOP_FRAME_CAP, LOOP_RESOLUTION,
    };
}
