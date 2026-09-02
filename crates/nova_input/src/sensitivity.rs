//! Mouse sensitivity: how far one frame of mouse motion drives each of the
//! three mouse paths, and the seam that pushes a change onto bindings that
//! already exist.
//!
//! The paths are independent because they want different feel: a look gain
//! that is comfortable for steering is far too coarse for docking, and the
//! free camera is a creator tool rather than a helm. They live in this crate
//! for the same reason the bindings table does - the settings panel renders in
//! the main menu, where no rig exists.
//!
//! What is STORED and applied is the RAW engine gain, the factor a mouse-motion
//! binding's [`Scale`] carries. The percentage the settings panel shows is a
//! projection of it through [`MouseSensitivityRange`], so nothing below the
//! menu has to know a percentage exists.
//!
//! Gamepad sticks are never scaled by these. A stick binding carries its own
//! fixed modifiers and no [`MousePath`] tag, so [`apply_mouse_sensitivity`]
//! cannot see it.

use bevy::prelude::*;
use bevy_enhanced_input::prelude::{EnhancedInputSystems, Scale};

/// Glob-import surface: `use nova_input::sensitivity::prelude::*`.
pub mod prelude {
    pub use super::{
        apply_mouse_sensitivity, mouse_sensitivity, MousePath, MouseSensitivity,
        MouseSensitivityPlugin, MouseSensitivityRange, MouseSensitivitySystems,
    };
}

/// One mouse path with a sensitivity of its own.
///
/// Also the COMPONENT that tags a mouse-motion binding as following that path's
/// gain. Tagging the binding rather than rebuilding the rig is what lets a
/// slider moved from the pause menu reach a ship that is already flying.
#[derive(Component, Clone, Copy, PartialEq, Eq, Hash, Debug, Reflect)]
#[reflect(Component)]
pub enum MousePath {
    /// The mouse half of `camera_rotate`: normal ship steering, free look and
    /// turret aim, which are one action read three ways.
    Look,
    /// Mouse-driven RCS translation, the docking fine-adjust.
    Rcs,
    /// Mouse look on the free (WASD) camera. Keyboard movement is not affected.
    FreeCamera,
}

impl MousePath {
    /// The paths in settings-row order.
    pub const ALL: [Self; 3] = [Self::Look, Self::Rcs, Self::FreeCamera];

    /// What the settings row reads.
    pub fn label(self) -> &'static str {
        match self {
            Self::Look => "Look Sensitivity",
            Self::Rcs => "RCS Sensitivity",
            Self::FreeCamera => "Free Camera Sensitivity",
        }
    }

    /// The player-facing range this path offers, and the raw gain behind it.
    pub fn range(self) -> MouseSensitivityRange {
        match self {
            // 100% is a third of what mouse look shipped as, so the default
            // 200% is deliberately about two-thirds of the pre-setting gain -
            // the old value was fast enough that most of the useful range sat
            // below it.
            Self::Look => MouseSensitivityRange {
                base: 0.001 / 3.0,
                max_percent: 300.0,
                default_percent: 200.0,
            },
            // 100% is the shipped gain, and the range opens upward only: the
            // complaint was that fine-adjust is too slow to cross the range,
            // never that it is too fast.
            Self::Rcs => MouseSensitivityRange {
                base: 0.03,
                max_percent: 500.0,
                default_percent: 100.0,
            },
            Self::FreeCamera => MouseSensitivityRange {
                base: 0.005,
                max_percent: 300.0,
                default_percent: 200.0,
            },
        }
    }

    /// The raw gain a fresh install starts on.
    pub fn default_raw(self) -> f32 {
        self.range().default_raw()
    }
}

/// The player-facing range of one mouse path, and the raw gain its `100%`
/// means.
///
/// Every path starts at `100%` - the lowest gain any of them offers is the one
/// it is measured against - so only the top and the default vary.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct MouseSensitivityRange {
    /// The raw engine gain `100%` projects to.
    pub base: f32,
    /// The highest percentage the slider offers.
    pub max_percent: f32,
    /// Where a fresh install starts, as a percentage.
    pub default_percent: f32,
}

impl MouseSensitivityRange {
    /// The percentage every path's slider starts at. `100%` is each path's own
    /// baseline, not a shared gain - the three raw values behind it differ by
    /// two orders of magnitude.
    pub const MIN_PERCENT: f32 = 100.0;

    /// How many equal steps a slider offers across its range. The detent the
    /// track ticks at, and the arrow-key step.
    pub const INTERVALS: f32 = 20.0;

    /// One detent, in percentage points.
    pub fn percent_step(self) -> f32 {
        (self.max_percent - Self::MIN_PERCENT) / Self::INTERVALS
    }

    /// The raw gain a percentage projects to, with the percentage clamped into
    /// range first.
    pub fn raw(self, percent: f32) -> f32 {
        self.base * self.clamp_percent(percent) / 100.0
    }

    /// The percentage a raw gain reads as, clamped into range.
    pub fn percent(self, raw: f32) -> f32 {
        self.clamp_percent(raw / self.base * 100.0)
    }

    /// A percentage held inside the range. A value that is not a number - which
    /// a hand-edited store can hold - reads as the default rather than
    /// propagating a NaN into the mixer of modifiers.
    pub fn clamp_percent(self, percent: f32) -> f32 {
        if percent.is_finite() {
            percent.clamp(Self::MIN_PERCENT, self.max_percent)
        } else {
            self.default_percent
        }
    }

    /// A raw gain held inside the range, so a corrupt or out-of-range persisted
    /// number can never reach a binding.
    pub fn clamp_raw(self, raw: f32) -> f32 {
        self.raw(self.percent(raw))
    }

    /// The raw gain a fresh install starts on.
    pub fn default_raw(self) -> f32 {
        self.raw(self.default_percent)
    }
}

/// The three live mouse gains, in raw engine units.
///
/// Written by the settings sliders and by the startup store load; read by
/// [`apply_mouse_sensitivity`], which is the only thing that puts them on a
/// binding.
#[derive(Resource, Clone, Copy, PartialEq, Debug, Reflect)]
#[reflect(Resource)]
pub struct MouseSensitivity {
    /// Ship steering, free look and turret aim.
    pub look: f32,
    /// Mouse-driven RCS translation.
    pub rcs: f32,
    /// Free-camera mouse look.
    pub free_camera: f32,
}

impl Default for MouseSensitivity {
    fn default() -> Self {
        Self {
            look: MousePath::Look.default_raw(),
            rcs: MousePath::Rcs.default_raw(),
            free_camera: MousePath::FreeCamera.default_raw(),
        }
    }
}

impl MouseSensitivity {
    /// One path's raw gain, clamped - so a value written straight from a store
    /// can never leave the range the slider offers.
    pub fn raw(&self, path: MousePath) -> f32 {
        path.range().clamp_raw(match path {
            MousePath::Look => self.look,
            MousePath::Rcs => self.rcs,
            MousePath::FreeCamera => self.free_camera,
        })
    }

    /// One path's gain as the percentage the settings row shows.
    pub fn percent(&self, path: MousePath) -> f32 {
        path.range().percent(self.raw(path))
    }

    /// Move one path, in raw engine units.
    pub fn set_raw(&mut self, path: MousePath, raw: f32) {
        let raw = path.range().clamp_raw(raw);
        match path {
            MousePath::Look => self.look = raw,
            MousePath::Rcs => self.rcs = raw,
            MousePath::FreeCamera => self.free_camera = raw,
        }
    }

    /// Move one path from the percentage a slider reports.
    pub fn set_percent(&mut self, path: MousePath, percent: f32) {
        self.set_raw(path, path.range().raw(percent));
    }
}

/// The gain components a mouse-motion binding wears: the live [`Scale`] and the
/// [`MousePath`] tag [`apply_mouse_sensitivity`] finds it by.
///
/// Spawned at the path's DEFAULT rather than at the live value, because a rig
/// bundle is built without world access. The apply system corrects it on the
/// next frame, which is the same path a slider moved mid-flight takes.
pub fn mouse_sensitivity(path: MousePath) -> impl Bundle {
    (path, Scale::splat(path.default_raw()))
}

/// Ordering handle for [`apply_mouse_sensitivity`]. Runs in `PreUpdate` before
/// `bevy_enhanced_input` evaluates the frame's bindings, so a sensitivity
/// changed this frame is the one this frame's motion is read through.
#[derive(SystemSet, Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct MouseSensitivitySystems;

/// Push the live gains onto every tagged mouse-motion binding.
///
/// Unconditional rather than change-gated: the set is three entities at most,
/// and the two events that must both reach it - the player moving a slider, and
/// a rig respawning after a rebind or a scenario load - would otherwise need a
/// resource-change run condition AND an `Added` override. Writes only on a real
/// difference, so it arms no change detection of its own.
pub fn apply_mouse_sensitivity(
    sensitivity: Res<MouseSensitivity>,
    mut bindings: Query<(&MousePath, &mut Scale)>,
) {
    for (path, mut scale) in &mut bindings {
        let factor = Vec3::splat(sensitivity.raw(*path));
        if scale.factor != factor {
            scale.factor = factor;
        }
    }
}

/// Installs [`MouseSensitivity`] and the system that applies it. Added by
/// [`NovaInputPlugin`](crate::NovaInputPlugin) beside the bindings table.
pub struct MouseSensitivityPlugin;

impl Plugin for MouseSensitivityPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MouseSensitivity>();
        app.register_type::<MouseSensitivity>();
        app.register_type::<MousePath>();
        app.configure_sets(
            PreUpdate,
            MouseSensitivitySystems.before(EnhancedInputSystems::Update),
        );
        app.add_systems(
            PreUpdate,
            apply_mouse_sensitivity.in_set(MouseSensitivitySystems),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The three ranges, as the settings table states them: each starts at its
    /// own `100%`, and the pre-setting gain is where the table says it is.
    #[test]
    fn each_path_projects_percentages_onto_its_own_raw_range() {
        let look = MousePath::Look.range();
        assert!((look.raw(100.0) - 0.001 / 3.0).abs() < 1e-9);
        assert!(
            (look.raw(300.0) - 0.001).abs() < 1e-9,
            "300% is the old gain"
        );
        assert!((look.default_raw() - 0.002 / 3.0).abs() < 1e-9);
        assert!((look.percent_step() - 10.0).abs() < 1e-4);

        let rcs = MousePath::Rcs.range();
        assert!((rcs.raw(100.0) - 0.03).abs() < 1e-9, "100% is the old gain");
        assert!((rcs.raw(500.0) - 0.15).abs() < 1e-9);
        assert!((rcs.default_raw() - 0.03).abs() < 1e-9);
        assert!((rcs.percent_step() - 20.0).abs() < 1e-4);

        let free = MousePath::FreeCamera.range();
        assert!((free.raw(100.0) - 0.005).abs() < 1e-9);
        assert!((free.raw(300.0) - 0.015).abs() < 1e-9);
        assert!(
            (free.default_raw() - 0.01).abs() < 1e-9,
            "200% is the old gain"
        );
        assert!((free.percent_step() - 10.0).abs() < 1e-4);
    }

    /// A store a player never wrote - hand-edited, truncated, or from a build
    /// with a different range - can only ever produce a value the slider could
    /// have produced.
    #[test]
    fn a_corrupt_or_out_of_range_value_is_clamped() {
        let mut sensitivity = MouseSensitivity::default();

        sensitivity.set_raw(MousePath::Look, 1.0);
        assert!((sensitivity.look - 0.001).abs() < 1e-9, "clamped to 300%");
        sensitivity.set_raw(MousePath::Look, -5.0);
        assert!(
            (sensitivity.look - 0.001 / 3.0).abs() < 1e-9,
            "clamped to 100%"
        );

        sensitivity.set_raw(MousePath::Rcs, f32::NAN);
        assert!(
            (sensitivity.rcs - MousePath::Rcs.default_raw()).abs() < 1e-9,
            "a value that is not a number reads as the default"
        );

        // The READ clamps too, so a resource written past the setters - which is
        // what a `*resource = ..` from the store load is - still cannot escape.
        let corrupt = MouseSensitivity {
            look: 99.0,
            rcs: 0.0,
            free_camera: f32::INFINITY,
        };
        assert!((corrupt.raw(MousePath::Look) - 0.001).abs() < 1e-9);
        assert!((corrupt.raw(MousePath::Rcs) - 0.03).abs() < 1e-9);
        assert!(
            (corrupt.raw(MousePath::FreeCamera) - MousePath::FreeCamera.default_raw()).abs() < 1e-9
        );
    }

    /// The apply seam reaches a binding that ALREADY exists, which is the whole
    /// claim behind "a slider moved while paused takes effect on resume".
    #[test]
    fn changing_a_gain_reaches_a_binding_that_already_exists() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(MouseSensitivityPlugin);

        let look = app
            .world_mut()
            .spawn(mouse_sensitivity(MousePath::Look))
            .id();
        let rcs = app
            .world_mut()
            .spawn(mouse_sensitivity(MousePath::Rcs))
            .id();
        // A stick binding: a `Scale` with no path tag, which is how a gamepad
        // stays out of these settings entirely.
        let stick = app.world_mut().spawn(Scale::splat(2.0)).id();
        app.update();

        app.world_mut()
            .resource_mut::<MouseSensitivity>()
            .set_percent(MousePath::Look, 300.0);
        app.update();

        assert!(
            (app.world().get::<Scale>(look).unwrap().factor.x - 0.001).abs() < 1e-9,
            "the live binding follows the setting"
        );
        assert!(
            (app.world().get::<Scale>(rcs).unwrap().factor.x - MousePath::Rcs.default_raw()).abs()
                < 1e-9,
            "the other paths are untouched"
        );
        assert_eq!(
            app.world().get::<Scale>(stick).unwrap().factor,
            Vec3::splat(2.0),
            "an untagged binding - every gamepad stick - is never scaled by these"
        );
    }
}
