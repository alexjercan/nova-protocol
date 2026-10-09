//! The player's chase-camera zoom: how far the wheel has dollied the chase rig
//! out from its hull-cleared framing.
//!
//! A level is a multiple of the active mode's hull-cleared rig distance, so
//! 1.0 is the ordinary framing and the closest permitted view. The wheel
//! observers add lines to [`ChaseZoom`]; [`update_camera_rig`](super::framing)
//! drains them each frame, because it is the only per-frame writer of the chase
//! offset and a level applied anywhere else would be overwritten.

use bevy::prelude::*;

/// The farthest manual zoom level: 8x the mode's hull-cleared rig distance.
/// At 8x a carrier sits whole in frame, and a skiff stays visible at its
/// centre.
pub(crate) const CHASE_ZOOM_MAX: f32 = 8.0;

/// How much one wheel line multiplies the camera distance: the fourth root of
/// 2, so four lines double the distance and twelve go from 1x to the cap.
const CHASE_ZOOM_FACTOR_PER_LINE: f32 = 1.189_207_1;

/// Session chase-camera dolly: lives for the app run. A saved world keeps
/// `manual` in its [`CameraView`](super::CameraView).
///
/// Normal and FreeLook share `manual`, so the level survives a mode switch and
/// a respawn. Turret ignores every level and keeps the authored combat rig. A
/// planned ORBIT opens on its survey framing; the first wheel line there starts
/// `orbit` from that distance, and the orbit's end drops it, so the manual
/// level comes back untouched.
#[derive(Resource, Debug, Clone, Copy, PartialEq)]
pub struct ChaseZoom {
    pub(crate) manual: f32,
    pub(crate) orbit: Option<OrbitZoom>,
    /// Wheel lines not yet applied, positive toward the ship.
    pub(crate) pending_lines: f32,
}

impl Default for ChaseZoom {
    fn default() -> Self {
        Self {
            manual: 1.0,
            orbit: None,
            pending_lines: 0.0,
        }
    }
}

/// The wheel's override of a planned ORBIT's survey framing.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct OrbitZoom {
    level: f32,
    /// The farthest level for this orbit: [`CHASE_ZOOM_MAX`], or the survey
    /// level at the first wheel line when that is farther. The survey is not
    /// clamped down to the manual cap, so wheel-out stops where it started.
    ceiling: f32,
}

/// A wheel delta that is NaN or infinite. Refused so that one bad event cannot
/// poison the session level.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct NonFiniteZoomInput(f32);

/// Pure: the level after `lines` (positive = toward the ship), clamped to
/// `[1, ceiling]`.
fn chase_zoom_step(level: f32, lines: f32, ceiling: f32) -> Result<f32, NonFiniteZoomInput> {
    if !lines.is_finite() {
        return Err(NonFiniteZoomInput(lines));
    }
    // min-then-max, not clamp: `f32::clamp` panics when min > max.
    Ok((level * CHASE_ZOOM_FACTOR_PER_LINE.powf(-lines))
        .min(ceiling)
        .max(1.0))
}

impl ChaseZoom {
    /// The shared Normal/FreeLook level, for a camera that opens before any
    /// frame has run the rig.
    pub fn manual(&self) -> f32 {
        self.manual
    }

    /// Drain the pending lines into the right level and return the rig scale
    /// for this frame. `turret` keeps the authored combat rig and drops the
    /// lines. `survey` is the planned ORBIT's survey scale, `None` outside a
    /// planned orbit.
    pub(crate) fn drain_scale(&mut self, turret: bool, survey: Option<f32>) -> f32 {
        let lines = std::mem::take(&mut self.pending_lines);
        if survey.is_none() {
            self.orbit = None;
        }
        if turret {
            return 1.0;
        }
        let step = |level: f32, ceiling: f32| {
            chase_zoom_step(level, lines, ceiling).unwrap_or_else(|err| {
                error!(
                    "ChaseZoom: refusing wheel delta {}; keeping level {level}",
                    err.0
                );
                level
            })
        };
        match survey {
            None => {
                self.manual = step(self.manual, CHASE_ZOOM_MAX);
                self.manual
            }
            Some(survey) if lines == 0.0 => self.orbit.map_or(survey, |orbit| orbit.level),
            Some(survey) => {
                let orbit = self.orbit.get_or_insert(OrbitZoom {
                    level: survey,
                    ceiling: survey.max(CHASE_ZOOM_MAX),
                });
                orbit.level = step(orbit.level, orbit.ceiling);
                orbit.level
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Lines that take the level from `from` to `to`, positive toward the ship.
    fn lines_between(from: f32, to: f32) -> f32 {
        -(to / from).ln() / CHASE_ZOOM_FACTOR_PER_LINE.ln()
    }

    #[test]
    fn wheel_up_moves_closer_and_wheel_down_farther_between_one_and_eight() {
        let out = chase_zoom_step(1.0, -1.0, CHASE_ZOOM_MAX).unwrap();
        assert!(
            (out - CHASE_ZOOM_FACTOR_PER_LINE).abs() < 1e-5,
            "one line out"
        );
        let back = chase_zoom_step(out, 1.0, CHASE_ZOOM_MAX).unwrap();
        assert!((back - 1.0).abs() < 1e-5, "one line back in");

        // The ordinary framing is the closest view, and 8x the farthest.
        assert_eq!(chase_zoom_step(1.0, 5.0, CHASE_ZOOM_MAX).unwrap(), 1.0);
        assert_eq!(
            chase_zoom_step(1.0, -1_000.0, CHASE_ZOOM_MAX).unwrap(),
            CHASE_ZOOM_MAX
        );
    }

    #[test]
    fn a_non_finite_wheel_delta_is_refused_and_keeps_the_level() {
        for lines in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            assert!(chase_zoom_step(2.0, lines, CHASE_ZOOM_MAX).is_err());

            let mut zoom = ChaseZoom {
                manual: 2.0,
                pending_lines: lines,
                ..default()
            };
            assert_eq!(zoom.drain_scale(false, None), 2.0, "{lines}");
            assert_eq!(zoom.pending_lines, 0.0);
        }
    }

    /// A planned ORBIT opens on its survey framing whatever the manual level,
    /// the wheel adjusts from the distance on screen, and leaving the orbit
    /// restores the manual level. A survey past the manual cap is not clamped
    /// down, and wheel-out stops there.
    #[test]
    fn orbit_zoom_starts_from_the_survey_and_leaving_restores_the_manual_level() {
        let mut zoom = ChaseZoom {
            manual: 2.0,
            ..default()
        };
        assert_eq!(zoom.drain_scale(false, None), 2.0);

        // Entry: the survey framing, not the manual level.
        assert_eq!(zoom.drain_scale(false, Some(12.0)), 12.0);

        // The first line adjusts from the survey distance.
        zoom.pending_lines = 1.0;
        let first = zoom.drain_scale(false, Some(12.0));
        assert!((first - 12.0 / CHASE_ZOOM_FACTOR_PER_LINE).abs() < 1e-4);
        assert_eq!(
            zoom.drain_scale(false, Some(12.0)),
            first,
            "the override holds"
        );

        // Wheel-out stops at the survey distance, wheel-in at the hull-cleared
        // minimum.
        zoom.pending_lines = -1_000.0;
        assert_eq!(zoom.drain_scale(false, Some(12.0)), 12.0);
        zoom.pending_lines = lines_between(12.0, 0.5);
        assert_eq!(zoom.drain_scale(false, Some(12.0)), 1.0);

        // Combat inside the orbit keeps the authored rig and drops the lines.
        zoom.pending_lines = -3.0;
        assert_eq!(zoom.drain_scale(true, Some(12.0)), 1.0);
        assert_eq!(zoom.drain_scale(false, Some(12.0)), 1.0);

        // Exit: the manual level comes back, and the next orbit opens on its
        // survey again.
        assert_eq!(zoom.drain_scale(false, None), 2.0);
        assert_eq!(zoom.drain_scale(false, Some(12.0)), 12.0);
    }
}
