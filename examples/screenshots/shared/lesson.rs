//! The training handbook's demonstrations: the grid every lesson loop is cut
//! on, the profile a producer records at, and the camera device that makes a
//! one-second sheet close.
//!
//! A lesson's art is REAL FOOTAGE of the game at
//! `assets/base/training/<lesson>.webp`, and the game cuts a loop by the grid
//! the LESSON authors (`crates/nova_authoring/src/base_content/lessons.rs`),
//! not by anything in the file. [`LESSON_GRID`] is this side of that
//! agreement; `crates/nova_authoring/tests/lesson_media.rs` holds the other.
//!
//! ## A sheet plays on a cycle, so its motion has to close
//!
//! Twelve frames at twelve a second is ONE SECOND, played back forever. A
//! one-way move - a burn across the frame, a ship pulling away - jump-cuts
//! every second on the wrap, on a screen a new player is sent to first. So a
//! lesson loop is not a clip of gameplay: it is a still scene the camera
//! moves over, on a path that returns to where it started.
//!
//! [`LessonSweep`] is that path: one period of a sine arc, centred on a
//! bearing, that the last cell hands back to the first. The subject holds its
//! pose (`freeze_bodies` pins the scene) while the camera - and only the
//! camera - moves.

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

/// The grid the base handbook authors for every loop lesson: twelve frames as
/// a 4x3 sheet of 960x540 cells. The cell is 16:9, so a producer records at a
/// 16:9 window and every frame scales without squashing.
///
/// The cell is HALF the capture window on each side, and that is the point:
/// the Lessons pane draws a demonstration about 900 logical pixels wide, so a
/// smaller cell would be shown as an upscale. The whole sheet is 3840x1620,
/// which is one clean 2x downscale per frame and inside the texture size every
/// target supports.
pub const LESSON_GRID: SheetGrid = SheetGrid {
    columns: 4,
    rows: 3,
    cell: (960, 540),
};

/// Frames a second a lesson sheet is recorded and played back at - the
/// `frames_per_second` the authored lesson carries. The recorder pins the
/// armed run's clock to it, so one rendered frame IS one cell.
pub const LESSON_FPS: u32 = 12;

/// The pixel size of a still lesson's demonstration: the capture window
/// itself, kept whole. A still has no grid to pay for, so there is nothing to
/// gain by shrinking it - `scripts/capture-lesson-media.sh` only changes the
/// codec.
pub const LESSON_STILL: (u32, u32) = CAPTURE_RESOLUTION;

/// What a lesson producer records at: a 16:9 master at the handbook's cadence,
/// with no output scale of its own - a sheet is scaled by its own cell size at
/// tile time, and a still by the packaging script.
///
/// The frame cap is four sheets' worth. A sheet closes itself at twelve, so
/// the cap only catches a producer that opened a loop it never closes.
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
