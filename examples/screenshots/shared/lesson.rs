//! The training handbook's demonstrations: the grid every lesson loop is cut
//! on, the profile a producer records at, and the two camera devices a lesson
//! sheet is shot with.
//!
//! A lesson's art is REAL FOOTAGE of the game at
//! `assets/base/training/<lesson>.webp`, and the game cuts a loop by the grid
//! the LESSON authors (`crates/nova_authoring/src/base_content/lessons.rs`),
//! not by anything in the file. [`LESSON_GRID`] is this side of that
//! agreement; `crates/nova_authoring/tests/lesson_media.rs` holds the other.
//!
//! ## Two kinds of loop, and why each one wraps cleanly
//!
//! Twenty frames at ten a second is TWO SECONDS, played back forever, so what
//! happens in those two seconds has to survive the wrap.
//!
//! A POSE loop shows a state - a ship holding a heading, a hull under power.
//! Nothing in the scene may move one way, or it jump-cuts every two seconds,
//! so the motion is the CAMERA on a path that returns to where it started:
//! [`LessonSweep`], one period of a sine arc about a bearing, with the subject
//! pinned by `freeze_bodies`.
//!
//! An ACTION loop shows a thing being DONE - a key pressed, a lock charging, a
//! bracket landing. Here the camera holds still ([`hold_lesson_camera`]) and
//! the action is the motion: a few cells of the state before, the act itself,
//! and the rest of the sheet holding the state after. It wraps the way a
//! tutorial clip wraps - back to the start to do it again - which reads as a
//! repeat rather than as a glitch precisely because the camera never moved.
//!
//! When the act is the SHIP MOVING, a camera fixed in the world loses it in
//! two seconds, and the player's own chase camera sits close astern where the
//! hull is a drive bell filling the cell. [`LessonChase`] is the third thing:
//! a camera that holds a fixed offset IN THE WORLD from the moving hull. The
//! ship keeps its place in the cell and its attitude on the screen, the rock
//! field goes past behind it, and a hull the flight computer turns end for end
//! is SEEN to turn, because the eye it turns under did not turn with it.

// Each producer includes the whole module and uses the part its lesson needs;
// the unused half is not dead code, it is another lesson's tool.
#![allow(
    dead_code,
    reason = "one source, many example targets: what one producer leaves unused another needs, so no single build can fulfil an expectation"
)]

use bevy::prelude::*;
use nova_protocol::{
    nova_debug::harness::{CAPTURE_RESOLUTION, LOOP_CRF},
    prelude::*,
};

/// The grid the base handbook authors for every loop lesson: twenty frames as
/// a 4x5 sheet of 960x540 cells. The cell is 16:9, so a producer records at a
/// 16:9 window and every frame scales without squashing.
///
/// The cell is HALF the capture window on each side, and that is the point:
/// the Lessons pane draws a demonstration about 900 logical pixels wide, so a
/// smaller cell would be shown as an upscale. The whole sheet is 3840x2700 -
/// one clean 2x downscale per frame, and both sides inside the 4096 texture
/// size a WebGL2 target is allowed to stop at. That ceiling is what sets the
/// LENGTH: twenty cells of this size is as long as a demonstration can be
/// without either shrinking the cell or splitting the sheet.
pub const LESSON_GRID: SheetGrid = SheetGrid {
    columns: 4,
    rows: 5,
    cell: (960, 540),
};

/// Frames a second a lesson sheet is recorded and played back at - the
/// `frames_per_second` the authored lesson carries. The recorder pins the
/// armed run's clock to it, so one rendered frame IS one cell, and a beat held
/// on `frames(n)` inside a recording is n cells of the sheet.
///
/// Ten rather than twelve: with the cell size fixed by the pane and the sheet
/// fixed by the texture ceiling, the frame RATE is the only knob left that
/// buys duration, and two seconds of demonstration is worth more than the
/// smoothness the last two frames a second would add.
pub const LESSON_FPS: u32 = 10;

/// The pixel size of a still lesson's demonstration: the capture window
/// itself, kept whole. A still has no grid to pay for, so there is nothing to
/// gain by shrinking it - `scripts/capture-lesson-media.sh` only changes the
/// codec.
pub const LESSON_STILL: (u32, u32) = CAPTURE_RESOLUTION;

/// What a lesson producer records at: a 16:9 master at the handbook's cadence,
/// with no output scale of its own - a sheet is scaled by its own cell size at
/// tile time, and a still by the packaging script.
///
/// The frame cap is four sheets' worth. A sheet closes itself at its own
/// twenty, so the cap only catches a producer that opened a loop it never
/// closes.
pub fn lesson_profile() -> LoopProfile {
    LoopProfile {
        window_resolution: (1920, 1080),
        output_resolution: (1920, 1080),
        fps: LESSON_FPS,
        crf: LOOP_CRF,
        frame_cap: LESSON_GRID.frames() * 4,
    }
}

/// A camera path that closes: one period of a sine sweep about `bearing`,
/// spread over the sheet's own cell count, looking at `subject` throughout.
///
/// The sweep is the whole motion in a lesson loop (see the module docs). Its
/// amplitude is deliberately small - this is a parallax drift that shows the
/// subject has depth, not an orbit that tours it.
#[derive(Resource, Debug, Clone, Copy)]
pub struct LessonSweep {
    /// What the camera looks at, and the centre of the arc.
    pub subject: Meters3,
    /// How far the camera stands off the subject.
    pub range: Meters,
    /// How far above the subject the camera rides.
    pub height: Meters,
    /// The bearing the sweep is centred on, in radians clockwise from world
    /// +Z. The subject's own framing: pick the side of it worth showing.
    pub bearing: f32,
    /// Half the sweep's width, in radians.
    pub arc: f32,
    /// Frames in one period. [`LESSON_GRID`]'s cell count, so the last cell
    /// hands back to the first.
    pub period: u32,
    /// Frames driven so far.
    frame: u32,
}

impl LessonSweep {
    /// A sweep of `arc_degrees` either side of `bearing_degrees`, one period
    /// per sheet.
    pub fn new(
        subject: Meters3,
        range: Meters,
        height: Meters,
        bearing_degrees: f32,
        arc_degrees: f32,
    ) -> Self {
        Self {
            subject,
            range,
            height,
            bearing: bearing_degrees.to_radians(),
            arc: arc_degrees.to_radians(),
            period: LESSON_GRID.frames(),
            frame: 0,
        }
    }

    /// Where the camera stands on frame `frame` of the period.
    pub fn position(&self) -> Meters3 {
        let phase = std::f32::consts::TAU * self.frame as f32 / self.period as f32;
        let angle = self.bearing + self.arc * phase.sin();
        Meters3::new(
            self.subject.0.x + self.range.get() * angle.sin(),
            self.subject.0.y + self.height.get(),
            self.subject.0.z + self.range.get() * angle.cos(),
        )
    }
}

/// Pose the camera where the sweep will START, without starting it.
///
/// A producer that has to AIM before it records - a radar sweep picks by the
/// camera's own look ray - frames the shot with this first, so the beats that
/// aim and the frames that record see the same view and the sweep begins with
/// no jump.
pub fn hold_lesson_camera(world: &mut World, sweep: LessonSweep) {
    pose_camera(world, sweep.position(), sweep.subject);
}

/// Ride the sweep: an `Update` system a lesson producer adds once, inert until
/// its step inserts a [`LessonSweep`].
///
/// One step of the arc per rendered frame, and the armed run's clock is pinned
/// to [`LESSON_FPS`], so a rendered frame is a cell however slow the capture
/// host renders. The phase the sheet starts on does not matter - a full period
/// closes wherever it began - so the sweep needs no handshake with the
/// recorder.
pub fn sweep_lesson_camera(world: &mut World) {
    let Some(sweep) = world.get_resource::<LessonSweep>().copied() else {
        return;
    };
    pose_camera(world, sweep.position(), sweep.subject);
    world.resource_mut::<LessonSweep>().frame += 1;
}

/// A camera that rides beside the player's hull: the same offset from it in
/// WORLD axes on every frame, looking at it.
///
/// World axes, not ship axes, and that is the whole design. A rig bolted to
/// the hull's own frame shows a flip as the background spinning round a still
/// ship, which is the opposite of what a braking order looks like to fly. Held
/// in world axes, the hull turns on the screen and the field behind it only
/// slides, so the sheet reads the way the maneuver feels.
#[derive(Resource, Debug, Clone, Copy)]
pub struct LessonChase {
    /// Where the eye stands relative to the hull, in world axes.
    pub offset: Meters3,
    /// What it looks at, relative to the hull, in world axes.
    ///
    /// Zero for the lessons whose subject IS the ship. A lesson about the ship
    /// against something far bigger than it - a hull on a ring around a
    /// planetoid - aims PAST the hull instead, so the body takes the middle of
    /// the cell and the ship falls out to a corner on the end of its spoke.
    pub aim: Meters3,
}

impl LessonChase {
    /// An eye `offset` from the hull, looking at the hull.
    pub fn new(offset: Meters3) -> Self {
        Self {
            offset,
            aim: Meters3::ZERO,
        }
    }

    /// Aim `aim` from the hull rather than at it.
    pub fn looking(mut self, aim: Meters3) -> Self {
        self.aim = aim;
        self
    }
}

/// Ride beside the hull: an `Update` system a lesson producer adds once, inert
/// until its step inserts a [`LessonChase`].
///
/// The hull it follows is the player's, which is the subject of every lesson
/// shot this way. A frame with no player ship yet poses nothing and says so
/// once per frame is too noisy to warn about, so it simply waits.
pub fn chase_lesson_camera(world: &mut World) {
    let Some(chase) = world.get_resource::<LessonChase>().copied() else {
        return;
    };
    let mut hulls = world.query_filtered::<&GlobalTransform, (
        With<SpaceshipRootMarker>,
        With<PlayerSpaceshipMarker>,
    )>();
    let Some(hull) = hulls.iter(world).next() else {
        return;
    };
    let subject = Meters3::from_engine(hull.translation());
    pose_camera(
        world,
        Meters3(subject.0 + chase.offset.0),
        Meters3(subject.0 + chase.aim.0),
    );
}
