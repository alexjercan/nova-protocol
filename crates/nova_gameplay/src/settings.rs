//! Player-facing game settings and the systems that apply them to the live
//! world (task 20260711-180511). Two small resources drive the settings menu
//! (`nova_menu`); the UI reads and writes them, and the systems here push the
//! changes onto the engine:
//!
//! - [`MasterVolume`] scales all audio. One-shot SFX pick it up at sink-spawn
//!   through bevy's [`GlobalVolume`] (`audio_output` multiplies
//!   `settings.volume * global_volume.volume`); the persistent thruster loop
//!   sets its own sink volume every frame, so [`crate::audio`] scales that by
//!   [`MasterVolume`] directly. Both output paths go through
//!   [`MasterVolume::output_gain`], which [`HarnessMute`] masks to silence in
//!   scripted runs (smoke suite, probe, screenshot captures) - the SETTING
//!   stays untouched, so persistence and the menu never see the mute.
//! - [`GraphicsQuality`] is a three-tier preset. It maps onto two things through
//!   the single `apply_graphics_quality` seam: the combat juice
//!   ([`crate::juice::JuiceSettings`]) and the derived [`GraphicsBudget`] gate
//!   (task 20260525-133013, the low-end spawn-less mode). `GraphicsBudget` is
//!   what the expensive effect systems actually read - whether hanabi particles
//!   spawn (torpedo blast/launch, turret muzzle) and the render-scale fraction
//!   the scenario view is drawn at before upscaling (task 20260718-004723, the
//!   fill lever for the web target; applied by `nova_scenario::render_scale`) -
//!   so the tier->cost policy lives in one place instead of being re-derived at
//!   every spawn site. Each tier stays genuinely distinct and observable across
//!   juice, particles and resolution. The particle cut is the one the frame-time
//!   baseline (20260716-123551) validates as a real combat cost. Scatter/object
//!   counts are deliberately NOT a preset lever: asteroids, rocks and debris are
//!   gameplay content, so no quality tier thins them (task 20260718-004834).
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
    ///
    /// This is the SETTING - what persistence saves and the menu slider shows.
    /// The gain the mixer actually applies is [`Self::output_gain`], which a
    /// harness run masks to silence; keeping the two apart is what stops a
    /// muted smoke run from persisting `0.0` over the player's real volume.
    pub fn factor(self) -> f32 {
        self.0.clamp(0.0, 1.0)
    }

    /// The gain audio output applies: the setting, masked to silence when the
    /// run is [`HarnessMute`]d. ONLY the output sites (the `GlobalVolume`
    /// push and the per-frame loop-sink writes in `audio`) call this;
    /// everything that means "the player's chosen volume" reads
    /// [`Self::factor`].
    pub fn output_gain(self, mute: HarnessMute) -> f32 {
        if mute.0 {
            0.0
        } else {
            self.factor()
        }
    }
}

/// Zero audio output for scripted runs - nobody listens to a smoke test, and
/// Xvfb hides the window but not the speakers. Resolved from the environment
/// ONCE at [`NovaSettingsPlugin`] build (a run's mute state cannot change
/// mid-session): `NOVA_MUTE` set and not `"0"` mutes any run, `NOVA_MUTE=0`
/// forces sound even under a harness, and with `NOVA_MUTE` unset a run is
/// muted iff a bcs harness env (`BCS_AUTOPILOT`/`BCS_SHOT`/`BCS_REEL`) is
/// active - which covers the smoke suite and probe with no changes there.
/// Tests inject the resource directly (insert after the plugin) instead of
/// touching process env, so parallel tests cannot race on it.
#[derive(Resource, Clone, Copy, Default, Debug, PartialEq)]
pub struct HarnessMute(pub bool);

impl HarnessMute {
    fn from_env() -> Self {
        let nova_mute = std::env::var("NOVA_MUTE").ok();
        let harness_env_active = ["BCS_AUTOPILOT", "BCS_SHOT", "BCS_REEL"]
            .iter()
            .any(|key| std::env::var_os(key).is_some());
        Self(harness_muted_from(nova_mute.as_deref(), harness_env_active))
    }
}

/// The mute decision as a pure function of its inputs (the probe-env pattern:
/// the env read stays in the thin [`HarnessMute::from_env`] wrapper so this
/// logic is unit-testable without process-global env mutation).
fn harness_muted_from(nova_mute: Option<&str>, harness_env_active: bool) -> bool {
    match nova_mute {
        Some(explicit) => explicit != "0",
        None => harness_env_active,
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
/// (driven by `apply_graphics_quality`) instead of being re-derived at every
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
    /// Internal render-resolution fraction (`0.0..=1.0`) the scenario view is
    /// drawn at before being upscaled to the window for presentation (task
    /// 20260718-004723). `1.0` is native window resolution (no intermediate
    /// image, the default direct-to-window path). Below `1.0` the 3D scene
    /// renders into a smaller offscreen target and a blit camera scales it up
    /// (the HUD stays crisp and clickable on the window) - the one lever that
    /// bites on the fill/overhead-bound web target the
    /// frame-time baseline (20260716-123551) flagged, where dropping pixels
    /// shaded buys more than the particle toggle. Only `Low` drops it;
    /// `Medium`/`High` stay at native resolution.
    pub render_scale: f32,
}

impl GraphicsBudget {
    /// The one place the tier->cost policy lives. High and Medium keep particles
    /// and native resolution; Low drops particles (the "spawn-less" low-end mode)
    /// and renders at a reduced `render_scale`. Particles and render-scale are the
    /// per-frame costs the preset gates - scatter/object counts are gameplay
    /// content and are never thinned by a quality tier (task 20260718-004834).
    pub fn for_quality(quality: GraphicsQuality) -> Self {
        match quality {
            GraphicsQuality::High => Self {
                particles: true,
                render_scale: 1.0,
            },
            GraphicsQuality::Medium => Self {
                particles: true,
                render_scale: 1.0,
            },
            GraphicsQuality::Low => Self {
                particles: false,
                // 0.7 draws ~49% of the pixels (0.7^2). Measured (task
                // 20260718-004723, report in that folder): on the RTX 3060 Ti
                // web/WebGPU rig the win at 0.7 is ~neutral - that GPU is
                // overhead-bound, not fill-bound, so the upscale pass roughly
                // cancels the fill saved. Kept at 0.7 (user decision) as a
                // conservative, still-readable drop aimed at the weaker
                // fill-bound web hardware the Low preset exists for (iGPUs,
                // phones) that the available rig cannot stand in for. Retune
                // with the `render_scale` perf override if such a rig appears.
                render_scale: 0.7,
            },
        }
    }

    /// The lowest render-scale a persisted or authored value may take, so a
    /// corrupt setting can never collapse the target to a zero-area texture
    /// (which is a fatal wgpu allocation) or an absurdly tiny, unreadable frame.
    pub const MIN_RENDER_SCALE: f32 = 0.25;

    /// Whether the scenario view renders at full native window resolution - the
    /// direct-to-window path with no intermediate target or blit camera. True
    /// for `Medium`/`High`; the render-scale reconcile only restructures the
    /// camera stack when this is false.
    pub fn is_native_resolution(self) -> bool {
        self.render_scale >= 1.0
    }

    /// The offscreen render-target size for a given window physical size, with
    /// the fraction clamped into `[MIN_RENDER_SCALE, 1.0]` and each axis kept at
    /// least one pixel, so the texture is always a valid, non-empty allocation.
    pub fn render_target_size(self, window_physical: UVec2) -> UVec2 {
        let scale = self.render_scale.clamp(Self::MIN_RENDER_SCALE, 1.0);
        UVec2::new(
            ((window_physical.x as f32 * scale).round() as u32).max(1),
            ((window_physical.y as f32 * scale).round() as u32).max(1),
        )
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
        app.insert_resource(HarnessMute::from_env());
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
/// have no `GlobalVolume`, and this must not panic them. Pushes the
/// [`MasterVolume::output_gain`] (harness runs push silence), never the raw
/// setting.
fn apply_master_volume(
    volume: Res<MasterVolume>,
    mute: Res<HarnessMute>,
    global: Option<ResMut<GlobalVolume>>,
) {
    if let Some(mut global) = global {
        global.volume = bevy::audio::Volume::Linear(volume.output_gain(*mute));
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
    /// plugin does, so a production app always has one). Overrides the
    /// env-derived [`HarnessMute`] with an explicit unmuted one AFTER the
    /// plugin (insert wins), so these tests stay deterministic even under
    /// `BCS_AUTOPILOT=1 cargo test` - the mute test injects its own.
    fn app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.init_resource::<JuiceSettings>();
        app.insert_resource(GlobalVolume::default());
        app.add_plugins(NovaSettingsPlugin);
        app.insert_resource(HarnessMute(false));
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
    fn harness_mute_silences_output_but_not_the_setting() {
        let mut app = app();
        app.insert_resource(HarnessMute(true));
        app.insert_resource(MasterVolume(0.3));
        app.update();
        assert_eq!(
            app.world().resource::<GlobalVolume>().volume,
            bevy::audio::Volume::Linear(0.0),
            "a muted run pushes silence onto the mixer"
        );
        assert_eq!(
            app.world().resource::<MasterVolume>().factor(),
            0.3,
            "the SETTING is untouched - persistence and the menu never see the mute"
        );
    }

    #[test]
    fn mute_policy_resolves_env_precedence() {
        // Explicit NOVA_MUTE wins in both directions; otherwise the bcs
        // harness envs decide. Pure inputs - no process-env mutation.
        assert!(harness_muted_from(Some("1"), false), "NOVA_MUTE=1 mutes");
        assert!(
            harness_muted_from(Some("1"), true),
            "NOVA_MUTE=1 mutes under a harness too"
        );
        assert!(
            !harness_muted_from(Some("0"), true),
            "NOVA_MUTE=0 forces sound through a harness run"
        );
        assert!(
            harness_muted_from(None, true),
            "a harness run mutes by default"
        );
        assert!(
            !harness_muted_from(None, false),
            "a normal run keeps its sound"
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

        assert!(
            high.particles && high.render_scale == 1.0,
            "High: particles on, native resolution"
        );
        assert!(
            medium.particles && medium.render_scale == 1.0,
            "Medium: particles on, native resolution"
        );
        assert!(
            !low.particles && low.render_scale < 1.0,
            "Low: spawn-less (no particles) and sub-native resolution"
        );

        // Only Low leaves native resolution - the render-scale lever is aimed at
        // the over-budget web target and Medium/High keep the crisp look.
        assert!(high.is_native_resolution() && medium.is_native_resolution());
        assert!(!low.is_native_resolution());

        // The default matches the default preset (full quality), so a
        // settings-less app renders everything.
        assert_eq!(
            GraphicsBudget::default(),
            GraphicsBudget::for_quality(GraphicsQuality::default())
        );
    }

    #[test]
    fn render_target_size_scales_clamps_and_never_zeroes() {
        // High renders at the native window size (no downscale).
        let high = GraphicsBudget::for_quality(GraphicsQuality::High);
        assert_eq!(
            high.render_target_size(UVec2::new(1280, 720)),
            UVec2::new(1280, 720),
            "native resolution keeps the window size exactly"
        );

        // Low draws fewer pixels per axis, rounded to whole pixels.
        let low = GraphicsBudget::for_quality(GraphicsQuality::Low);
        let target = low.render_target_size(UVec2::new(1280, 720));
        assert!(
            target.x < 1280 && target.y < 720,
            "Low shrinks the render target (got {target:?})"
        );
        assert_eq!(
            target,
            UVec2::new(
                (1280.0 * low.render_scale).round() as u32,
                (720.0 * low.render_scale).round() as u32
            )
        );

        // A corrupt sub-minimum fraction is clamped, never producing a
        // zero-area (fatal wgpu) or absurdly tiny target.
        let corrupt = GraphicsBudget {
            render_scale: 0.0,
            ..high
        };
        let clamped = corrupt.render_target_size(UVec2::new(1280, 720));
        assert_eq!(
            clamped,
            UVec2::new(
                (1280.0 * GraphicsBudget::MIN_RENDER_SCALE).round() as u32,
                (720.0 * GraphicsBudget::MIN_RENDER_SCALE).round() as u32
            ),
            "an out-of-range fraction clamps to MIN_RENDER_SCALE"
        );
        // Even a 1px window survives (both axes stay >= 1).
        assert_eq!(
            low.render_target_size(UVec2::new(1, 1)),
            UVec2::new(1, 1),
            "a tiny window never rounds an axis to zero"
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
