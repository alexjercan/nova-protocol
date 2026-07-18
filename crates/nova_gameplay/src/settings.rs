//! Player-facing game settings and the systems that apply them to the live
//! world (task 20260711-180511). Two small resources drive the settings menu
//! (`nova_menu`); the UI reads and writes them, and the systems here push the
//! changes onto the engine:
//!
//! - [`MasterVolume`] scales all audio. One-shot SFX pick it up at sink-spawn
//!   through bevy's [`GlobalVolume`] (`audio_output` multiplies
//!   `settings.volume * global_volume.volume`); the persistent thruster loop
//!   sets its own sink volume every frame, so [`crate::audio`] scales that by
//!   [`MasterVolume`] directly.
//! - [`GraphicsQuality`] is a three-tier preset. It maps onto two things through
//!   the single [`apply_graphics_quality`] seam: the combat juice
//!   ([`crate::juice::JuiceSettings`]) and the derived [`GraphicsBudget`] gate
//!   (task 20260525-133013, the low-end spawn-less mode). `GraphicsBudget` is
//!   what the expensive effect systems actually read - whether hanabi particles
//!   spawn (torpedo blast/launch, turret muzzle) - so the tier->cost policy lives
//!   in one place instead of being re-derived at every spawn site. Each tier
//!   stays genuinely distinct and observable across juice and particles. The
//!   particle cut point is the one the frame-time baseline (20260716-123551)
//!   validates as a real combat cost. Scatter/object counts are deliberately NOT
//!   a preset lever: asteroids, rocks and debris are gameplay content, so no
//!   quality tier thins them (task 20260718-004834).
//!
//! Persistence (native RON + web localStorage) lives in `nova_menu`, which owns
//! the load-at-startup and save-on-change wiring; this module only defines the
//! resources and their live application, so menu-less apps (the examples) get
//! sane defaults with no disk I/O.

use bevy::prelude::*;

use crate::juice::JuiceSettings;

/// Master audio volume, linear `0.0..=1.0`. Default full. Scales every sound in
/// the game (see the module docs for the two application paths).
#[derive(Resource, Clone, Copy, PartialEq, Debug, Reflect)]
#[reflect(Resource)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct MasterVolume(pub f32);

impl Default for MasterVolume {
    fn default() -> Self {
        Self(1.0)
    }
}

impl MasterVolume {
    /// The clamped linear factor, so a corrupt persisted value can never push
    /// the mixer out of range.
    pub fn factor(self) -> f32 {
        self.0.clamp(0.0, 1.0)
    }
}

/// The graphics-quality preset the settings menu exposes.
///
/// `Resource`-only on purpose: on Bevy 0.19 a `#[derive(Resource)]` type is
/// component-backed, so this doubles as the `Component` that
/// `nova_ui::widget::button_on_setting::<GraphicsQuality>` needs to drive the
/// segmented quality buttons - deriving `Component` too would conflict (mirrors
/// the editor's `SectionChoice` and nova_ui's own `Choice` test type).
#[derive(Resource, Clone, Copy, PartialEq, Eq, Debug, Default, Reflect)]
#[reflect(Resource)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum GraphicsQuality {
    /// Cheapest: all combat juice off (no camera shake, no hit flashes). The
    /// low-end task extends this to also skip particles.
    Low,
    /// Middle: hit flashes stay, camera shake off.
    Medium,
    /// Everything on (the default look).
    #[default]
    High,
}

impl GraphicsQuality {
    /// Short display label for the segmented button.
    pub fn label(self) -> &'static str {
        match self {
            GraphicsQuality::Low => "Low",
            GraphicsQuality::Medium => "Medium",
            GraphicsQuality::High => "High",
        }
    }

    /// The presets in menu order (worst -> best), so the UI can build the row
    /// from one source instead of hand-listing variants.
    pub const ALL: [GraphicsQuality; 3] = [
        GraphicsQuality::Low,
        GraphicsQuality::Medium,
        GraphicsQuality::High,
    ];
}

/// The applied per-frame visual-cost gates a [`GraphicsQuality`] preset produces
/// (task 20260525-133013, the low-end spawn-less mode). `GraphicsQuality` is the
/// player's *choice*; this is the *derived budget* the expensive effect systems
/// read, so the tier->cost policy lives only in [`GraphicsBudget::for_quality`]
/// (driven by [`apply_graphics_quality`]) instead of being re-derived at every
/// spawn site. Mirrors how [`crate::juice::JuiceSettings`] is the resource the
/// juice systems read while the preset just flips its toggles.
///
/// A settings-less app (examples, headless tools) never inserts this; those
/// systems read it through `Option`/`get_resource` and fall back to
/// [`GraphicsBudget::default`] (full quality), so particles render normally
/// when the preset is absent.
#[derive(Resource, Clone, Copy, PartialEq, Debug, Reflect)]
#[reflect(Resource)]
pub struct GraphicsBudget {
    /// Whether hanabi particle effects spawn at all. Off on `Low` - the "spawn-less"
    /// in the task name; particle spawns are the biggest per-event cost the
    /// baseline flags.
    pub particles: bool,
}

impl GraphicsBudget {
    /// The one place the tier->cost policy lives. High and Medium keep particles;
    /// Low drops them entirely (the "spawn-less" low-end mode). Particles are the
    /// only per-frame cost the preset gates - scatter/object counts are gameplay
    /// content and are never thinned by a quality tier (task 20260718-004834).
    pub fn for_quality(quality: GraphicsQuality) -> Self {
        match quality {
            GraphicsQuality::High => Self { particles: true },
            GraphicsQuality::Medium => Self { particles: true },
            GraphicsQuality::Low => Self { particles: false },
        }
    }
}

impl Default for GraphicsBudget {
    /// Full quality, matching [`GraphicsQuality::default`], so an app without the
    /// settings plugin renders everything.
    fn default() -> Self {
        Self::for_quality(GraphicsQuality::default())
    }
}

/// Registers the settings resources and the systems that apply them live.
/// Added by [`crate::plugin::NovaGameplayPlugin`] so every app (menu or not)
/// has the resources and the apply wiring; the menu adds persistence on top.
pub struct NovaSettingsPlugin;

impl Plugin for NovaSettingsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MasterVolume>();
        app.init_resource::<GraphicsQuality>();
        app.init_resource::<GraphicsBudget>();
        app.register_type::<MasterVolume>();
        app.register_type::<GraphicsQuality>();
        app.register_type::<GraphicsBudget>();

        // Apply on change only. `resource_changed` is true on the first frame
        // too (a freshly-inserted resource counts as changed), so the defaults
        // - and any persisted values a startup load writes in - are pushed onto
        // the engine exactly once without a dedicated startup system.
        app.add_systems(
            Update,
            (
                apply_master_volume.run_if(resource_changed::<MasterVolume>),
                apply_graphics_quality.run_if(resource_changed::<GraphicsQuality>),
            ),
        );
    }
}

/// Push [`MasterVolume`] onto bevy's [`GlobalVolume`], which scales every sound
/// played after this point (`audio_output` multiplies it into each new sink).
/// `Option` on the target: minimal/headless rigs without bevy's `AudioPlugin`
/// have no `GlobalVolume`, and this must not panic them.
fn apply_master_volume(volume: Res<MasterVolume>, global: Option<ResMut<GlobalVolume>>) {
    if let Some(mut global) = global {
        global.volume = bevy::audio::Volume::Linear(volume.factor());
    }
}

/// Map the [`GraphicsQuality`] preset onto the two things it drives: the derived
/// [`GraphicsBudget`] gate (particle cost, read by the effect systems)
/// and the combat-juice toggles. This is the single seam - the low-end spawn-less
/// mode (20260525-133013) hooks the budget half here rather than re-deriving the
/// tier at every spawn site. The budget is written unconditionally (this plugin
/// owns it); the juice half is `Option`-guarded for headless juice-less rigs and
/// only touches the fields it owns (the master switch and the two per-effect
/// enables), leaving juice tunables like the distance falloff alone.
fn apply_graphics_quality(
    quality: Res<GraphicsQuality>,
    mut budget: ResMut<GraphicsBudget>,
    juice: Option<ResMut<JuiceSettings>>,
) {
    *budget = GraphicsBudget::for_quality(*quality);

    let Some(mut juice) = juice else {
        return;
    };
    match *quality {
        GraphicsQuality::High => {
            juice.master_enabled = true;
            juice.shake.enabled = true;
            juice.flash.enabled = true;
        }
        GraphicsQuality::Medium => {
            juice.master_enabled = true;
            juice.shake.enabled = false;
            juice.flash.enabled = true;
        }
        GraphicsQuality::Low => {
            juice.master_enabled = false;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A minimal app with just the settings plugin and a `JuiceSettings` to
    /// receive the graphics preset (the plugin does not own it; the juice
    /// plugin does, so a production app always has one).
    fn app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.init_resource::<JuiceSettings>();
        app.insert_resource(GlobalVolume::default());
        app.add_plugins(NovaSettingsPlugin);
        app
    }

    #[test]
    fn master_volume_drives_global_volume() {
        let mut app = app();
        // First update applies the default (1.0).
        app.update();
        assert_eq!(
            app.world().resource::<GlobalVolume>().volume,
            bevy::audio::Volume::Linear(1.0)
        );

        app.insert_resource(MasterVolume(0.3));
        app.update();
        assert_eq!(
            app.world().resource::<GlobalVolume>().volume,
            bevy::audio::Volume::Linear(0.3),
            "changing MasterVolume pushes onto GlobalVolume"
        );
    }

    #[test]
    fn master_volume_is_clamped_into_range() {
        let mut app = app();
        app.insert_resource(MasterVolume(5.0));
        app.update();
        assert_eq!(
            app.world().resource::<GlobalVolume>().volume,
            bevy::audio::Volume::Linear(1.0),
            "an out-of-range persisted value can never over-drive the mixer"
        );
    }

    #[test]
    fn each_quality_tier_maps_to_a_distinct_juice_config() {
        let mut app = app();

        app.insert_resource(GraphicsQuality::High);
        app.update();
        let j = app.world().resource::<JuiceSettings>();
        assert!(
            j.master_enabled && j.shake.enabled && j.flash.enabled,
            "High: all on"
        );

        app.insert_resource(GraphicsQuality::Medium);
        app.update();
        let j = app.world().resource::<JuiceSettings>();
        assert!(
            j.master_enabled && !j.shake.enabled && j.flash.enabled,
            "Medium: flash on, shake off - a real, observable step down from High"
        );

        app.insert_resource(GraphicsQuality::Low);
        app.update();
        let j = app.world().resource::<JuiceSettings>();
        assert!(!j.master_enabled, "Low: juice master switch off");
    }

    #[test]
    fn graphics_budget_gates_particles_only_by_tier() {
        // The tier->cost policy is a pure function, so assert it directly rather
        // than only through the app. Particles are the ONLY per-frame cost the
        // preset gates: High and Medium keep them, Low is spawn-less. Scatter and
        // object counts are gameplay content and are never a preset lever
        // (task 20260718-004834), so there is no density field to assert on.
        let high = GraphicsBudget::for_quality(GraphicsQuality::High);
        let medium = GraphicsBudget::for_quality(GraphicsQuality::Medium);
        let low = GraphicsBudget::for_quality(GraphicsQuality::Low);

        assert!(high.particles, "High: particles on");
        assert!(medium.particles, "Medium: particles on");
        assert!(!low.particles, "Low: spawn-less (no particles)");

        // The default matches the default preset (full quality), so a
        // settings-less app renders everything.
        assert_eq!(
            GraphicsBudget::default(),
            GraphicsBudget::for_quality(GraphicsQuality::default())
        );
    }

    #[test]
    fn changing_quality_pushes_onto_the_graphics_budget() {
        // The apply seam writes the budget even on a juice-less rig (the `app()`
        // here has JuiceSettings, but the write happens before that Option guard).
        let mut app = app();

        app.insert_resource(GraphicsQuality::Low);
        app.update();
        assert_eq!(
            *app.world().resource::<GraphicsBudget>(),
            GraphicsBudget::for_quality(GraphicsQuality::Low),
            "selecting Low pushes the Low budget onto the resource"
        );

        app.insert_resource(GraphicsQuality::High);
        app.update();
        assert_eq!(
            *app.world().resource::<GraphicsBudget>(),
            GraphicsBudget::for_quality(GraphicsQuality::High),
            "selecting High pushes the High budget back on"
        );
    }
}
