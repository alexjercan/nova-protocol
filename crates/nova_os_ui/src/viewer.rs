//! The one 3D viewer both NOVA OS apps run: the orbit camera's spherical math,
//! the feel of its controls, the wrapping selection cycle, and the unlit
//! material its proxy meshes are drawn with.
//!
//! `map` and `ship` are the same viewer pointed at different things - a system
//! map and a hull - so everything that decides how a panel ANSWERS the player
//! lives here, and everything that decides what it FRAMES stays with the app:
//! each viewer passes in its own opening angles and its own zoom clamp. Written
//! twice, a change to the way the map orbits silently does not reach the way the
//! ship orbits, and the player reads the two panels as two different controls
//! for no stated reason.
//!
//! Both cameras drive these angles directly rather than routing through the
//! shared `SphereOrbit` plugin: its smoothed input path does not rotate a
//! render-to-texture camera (`verify-reused-driver-actually-moves`).

use bevy::prelude::*;

use crate::terminal::NovaOsAppInput;

/// Radians per second a held turn or tilt action moves the orbit.
const ORBIT_TURN_RATE: f32 = 1.6;

/// Highest the eye may be tilted, in radians above the focus plane. Short of
/// straight overhead (`PI / 2`), where the look-at up vector degenerates and the
/// view rolls.
const ORBIT_PHI_MAX: f32 = 1.45;

/// Lowest the eye may be tilted, in radians above the focus plane. Short of the
/// plane itself, where the scene collapses to an edge-on line.
const ORBIT_PHI_MIN: f32 = 0.12;

/// Radians of orbit per pixel of right-button drag. Gentle on purpose, so a
/// small drag is a small turn.
const ORBIT_DRAG_RADIANS_PER_PX: f32 = 0.0024;

/// Fraction of the orbit radius already held that one wheel notch adds or
/// removes. Proportional, so one notch covers ground at 20 km that it must not
/// cover at 30 m.
const ORBIT_ZOOM_PER_NOTCH: f32 = 0.12;

/// The camera eye offset from the focus for a given orbit, on a Y-up sphere.
pub(crate) fn orbit_eye(radius: f32, theta: f32, phi: f32) -> Vec3 {
    let horizontal = radius * phi.cos();
    Vec3::new(
        horizontal * theta.sin(),
        radius * phi.sin(),
        horizontal * theta.cos(),
    )
}

/// One frame of a viewer's orbit gesture, as it comes off the app input.
///
/// A plain value rather than a read straight off [`NovaOsAppInput`], so the
/// feel in [`OrbitGesture::apply`] - every rate and rail the player can sense -
/// is one testable answer instead of two copies inside two systems.
#[derive(Clone, Copy, Default, PartialEq, Debug)]
pub(crate) struct OrbitGesture {
    /// `novaos_orbit_left` held: turn the eye left around the focus.
    pub(crate) turn_left: bool,
    /// `novaos_orbit_right` held.
    pub(crate) turn_right: bool,
    /// `novaos_orbit_up` held: tilt the eye up off the focus plane.
    pub(crate) tilt_up: bool,
    /// `novaos_orbit_down` held.
    pub(crate) tilt_down: bool,
    /// The frame's mouse travel, in physical pixels, while the RIGHT button is
    /// down. `None` means it is up, and the drag contributes nothing.
    pub(crate) drag: Option<Vec2>,
}

impl OrbitGesture {
    /// Read this frame's gesture.
    ///
    /// The KEYS are the reliable path: mouse-drag look is unreliable through
    /// the NOVA OS pointer forwarding, so `novaos_orbit_*` is what a player can
    /// count on and the drag is the convenience on top.
    ///
    /// The drag is the RIGHT button only. LMB is the blip-select click (the
    /// `Button` widget), so letting it orbit turned a small press-with-motion
    /// into a drag that slid the blip out from under the cursor and ate the
    /// selection.
    ///
    /// `motion_delta` is the frame's mouse travel, already drained by the
    /// caller: both viewers drain it unconditionally, so a frame with the right
    /// button up does not hand its motion to the next one.
    pub(crate) fn read(input: &NovaOsAppInput<'_, '_>, motion_delta: Vec2) -> Self {
        Self {
            turn_left: input.pressed("novaos_orbit_left"),
            turn_right: input.pressed("novaos_orbit_right"),
            tilt_up: input.pressed("novaos_orbit_up"),
            tilt_down: input.pressed("novaos_orbit_down"),
            drag: input
                .mouse_pressed(MouseButton::Right)
                .then_some(motion_delta),
        }
    }

    /// Whether the player touched the orbit at all this frame. An untouched
    /// orbit must not be written, so an idle frame leaves the camera's orbit
    /// component unchanged.
    pub(crate) fn is_idle(self) -> bool {
        !self.turn_left
            && !self.turn_right
            && !self.tilt_up
            && !self.tilt_down
            && self.drag.is_none()
    }

    /// Fold this gesture into an orbit's `theta` (azimuth around the focus) and
    /// `phi` (elevation above the focus plane) over a frame of `dt` seconds,
    /// and answer the new `(theta, phi)`.
    ///
    /// Applied straight to the angles, with no smoothing layer between the
    /// press and the camera.
    pub(crate) fn apply(self, dt: f32, mut theta: f32, mut phi: f32) -> (f32, f32) {
        let turn = ORBIT_TURN_RATE * dt;
        if self.turn_left {
            theta += turn;
        }
        if self.turn_right {
            theta -= turn;
        }
        if self.tilt_up {
            phi = (phi + turn).min(ORBIT_PHI_MAX);
        }
        if self.tilt_down {
            phi = (phi - turn).max(ORBIT_PHI_MIN);
        }
        if let Some(drag) = self.drag {
            theta -= drag.x * ORBIT_DRAG_RADIANS_PER_PX;
            phi = (phi + drag.y * ORBIT_DRAG_RADIANS_PER_PX).clamp(ORBIT_PHI_MIN, ORBIT_PHI_MAX);
        }
        (theta, phi)
    }
}

/// The orbit radius after `wheel` notches of zoom, held between `min` and
/// `max`.
///
/// The bounds are the viewers' one deliberate difference here: the map's
/// ceiling tracks the live contact spread (a fixed one left a contact 20 km out
/// permanently off the map), the ship's is the fixed reach of a hull.
pub(crate) fn zoom_radius(radius: f32, wheel: f32, min: f32, max: f32) -> f32 {
    (radius * (1.0 - wheel * ORBIT_ZOOM_PER_NOTCH)).clamp(min, max)
}

/// The index a `novaos_next` / `novaos_prev` press moves a selection to, in a
/// list of `len` items, wrapping at both ends.
///
/// `None` for an empty list - there is nothing to select. Index 0 when nothing
/// is selected yet, so the first press in either direction picks the first item.
pub(crate) fn cycle_index(current: Option<usize>, len: usize, forward: bool) -> Option<usize> {
    if len == 0 {
        return None;
    }
    Some(match current {
        Some(i) if forward => (i + 1) % len,
        Some(i) => (i + len - 1) % len,
        None => 0,
    })
}

/// An unlit emissive-ish material, so a viewer's proxy meshes read at full
/// colour without a light on the render layer they live on.
pub(crate) fn unlit(color: Color) -> StandardMaterial {
    StandardMaterial {
        base_color: color,
        unlit: true,
        alpha_mode: AlphaMode::Blend,
        ..default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The map and the ship are ONE viewer. They open at different framings on
    /// purpose - a system map is not a hull - so the assertion is the DELTA:
    /// whatever angles and distance a panel sits at, one gesture has to move it
    /// by the same amount as the other, or the player is holding two controls.
    ///
    /// Written against the shared step rather than against copied numbers: a
    /// rate pasted into this test would drift exactly the way the second copy
    /// of the orbit block did.
    #[test]
    fn one_gesture_moves_both_viewers_through_the_same_angles() {
        let gesture = OrbitGesture {
            turn_left: true,
            tilt_up: true,
            drag: Some(Vec2::new(24.0, -18.0)),
            ..default()
        };
        let dt = 1.0 / 60.0;
        // Neither start is near a tilt rail, so this measures the feel and not
        // the clamps.
        let step = |theta: f32, phi: f32| {
            let (turned, tilted) = gesture.apply(dt, theta, phi);
            (turned - theta, tilted - phi)
        };

        let (map_turn, map_tilt) = step(0.80, 0.62);
        let (ship_turn, ship_tilt) = step(0.70, 0.50);

        assert!(
            (map_turn - ship_turn).abs() < 1e-6,
            "one gesture must turn both viewers alike: map {map_turn}, ship {ship_turn}"
        );
        assert!(
            (map_tilt - ship_tilt).abs() < 1e-6,
            "one gesture must tilt both viewers alike: map {map_tilt}, ship {ship_tilt}"
        );
    }

    /// A second of held turn is a second of the named rate, and turning never
    /// tilts.
    #[test]
    fn a_held_turn_moves_the_orbit_at_the_named_rate() {
        let (theta, phi) = OrbitGesture {
            turn_left: true,
            ..default()
        }
        .apply(1.0, 0.0, 0.5);

        assert!((theta - ORBIT_TURN_RATE).abs() < 1e-6, "theta: {theta}");
        assert_eq!(phi, 0.5, "a turn is not a tilt");
    }

    /// Drag sensitivity is gentle by the pixel, and a drag to the RIGHT swings
    /// the eye the other way around the focus - the scene follows the hand.
    #[test]
    fn a_drag_turns_by_the_named_radians_per_pixel() {
        let (theta, _) = OrbitGesture {
            drag: Some(Vec2::new(100.0, 0.0)),
            ..default()
        }
        .apply(1.0 / 60.0, 0.0, 0.5);

        assert!(
            (theta + 100.0 * ORBIT_DRAG_RADIANS_PER_PX).abs() < 1e-6,
            "theta: {theta}"
        );
    }

    /// Both tilt rails hold, by key and by drag. Straight overhead degenerates
    /// the look-at up vector and rolls the view; flat on the plane collapses
    /// the scene to an edge-on line.
    #[test]
    fn the_tilt_never_reaches_overhead_or_the_focus_plane() {
        let held_up = OrbitGesture {
            tilt_up: true,
            ..default()
        };
        let held_down = OrbitGesture {
            tilt_down: true,
            ..default()
        };

        let (mut theta, mut phi) = (0.0, ORBIT_PHI_MAX);
        for _ in 0..240 {
            (theta, phi) = held_up.apply(1.0 / 60.0, theta, phi);
        }
        assert!(phi <= ORBIT_PHI_MAX, "held tilt up: {phi}");

        phi = ORBIT_PHI_MIN;
        for _ in 0..240 {
            (theta, phi) = held_down.apply(1.0 / 60.0, theta, phi);
        }
        assert!(phi >= ORBIT_PHI_MIN, "held tilt down: {phi}");

        let (_, flung_up) = OrbitGesture {
            drag: Some(Vec2::new(0.0, 10_000.0)),
            ..default()
        }
        .apply(1.0 / 60.0, theta, 1.0);
        assert!(flung_up <= ORBIT_PHI_MAX, "flung drag up: {flung_up}");

        let (_, flung_down) = OrbitGesture {
            drag: Some(Vec2::new(0.0, -10_000.0)),
            ..default()
        }
        .apply(1.0 / 60.0, theta, 1.0);
        assert!(flung_down >= ORBIT_PHI_MIN, "flung drag down: {flung_down}");
    }

    /// A frame the player did not touch the orbit on must not write the
    /// camera's angles, so a still panel does not mark its orbit changed every
    /// frame it is open. Holding the RIGHT button IS a touch, even at rest -
    /// that is what re-seats a drag against the tilt rails.
    #[test]
    fn an_untouched_orbit_is_idle_but_a_held_drag_is_not() {
        let turning = OrbitGesture {
            turn_left: true,
            ..default()
        };
        let tilting = OrbitGesture {
            tilt_down: true,
            ..default()
        };
        let holding_rmb = OrbitGesture {
            drag: Some(Vec2::ZERO),
            ..default()
        };

        assert!(OrbitGesture::default().is_idle(), "nothing held");
        assert!(!turning.is_idle());
        assert!(!tilting.is_idle());
        assert!(!holding_rmb.is_idle(), "the right button alone is a touch");
    }

    /// The eye lands on the sphere it was asked for, and a positive elevation
    /// looks DOWN at the focus from above.
    #[test]
    fn the_eye_sits_on_the_orbit_sphere_it_was_asked_for() {
        let eye = orbit_eye(120.0, 0.8, 0.62);

        assert!((eye.length() - 120.0).abs() < 1e-3, "eye: {eye}");
        assert!(eye.y > 0.0, "eye: {eye}");
    }

    /// One notch moves a fraction of the distance already held, so the wheel
    /// covers ground far out that it must not cover up close - and the two
    /// clamps each viewer passes in hold at both ends.
    #[test]
    fn a_wheel_notch_zooms_by_a_fraction_of_the_distance_already_held() {
        let near = zoom_radius(100.0, 1.0, 1.0, 10_000.0);
        let far = zoom_radius(1000.0, 1.0, 1.0, 10_000.0);

        assert!(
            (near - 100.0 * (1.0 - ORBIT_ZOOM_PER_NOTCH)).abs() < 1e-3,
            "near: {near}"
        );
        assert!(
            (far - 1000.0 * (1.0 - ORBIT_ZOOM_PER_NOTCH)).abs() < 1e-3,
            "far: {far}"
        );
        assert_eq!(zoom_radius(100.0, 50.0, 30.0, 400.0), 30.0, "floor holds");
        assert_eq!(
            zoom_radius(100.0, -50.0, 30.0, 400.0),
            400.0,
            "ceiling holds"
        );
    }

    /// The cycle wraps at both ends, and an empty list has nothing to pick.
    #[test]
    fn the_selection_cycle_wraps_at_both_ends() {
        assert_eq!(cycle_index(Some(2), 3, true), Some(0), "off the end");
        assert_eq!(cycle_index(Some(0), 3, false), Some(2), "off the front");
        assert_eq!(cycle_index(Some(0), 3, true), Some(1));
        assert_eq!(cycle_index(Some(2), 3, false), Some(1));
        assert_eq!(cycle_index(None, 3, true), Some(0), "first press picks one");
        assert_eq!(cycle_index(None, 3, false), Some(0));
        assert_eq!(cycle_index(None, 0, true), None, "nothing to select");
        assert_eq!(cycle_index(Some(0), 0, true), None);
    }
}
