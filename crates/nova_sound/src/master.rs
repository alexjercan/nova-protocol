//! The master volume and the harness mute: the two knobs on the engine's
//! output itself, below every bus.
//!
//! [`MasterVolume`] scales all audio. One-shot SFX pick it up at sink-spawn
//! through bevy's [`GlobalVolume`] (`audio_output` multiplies
//! `settings.volume * global_volume.volume`); a loop sets its own sink volume
//! every frame, so the per-frame pass in `voice` scales that by
//! [`MasterVolume`] directly. Both output paths go through
//! [`MasterVolume::output_gain`], which [`HarnessMute`] masks to silence in
//! scripted runs (probe sweeps, screenshot captures) - the SETTING stays
//! untouched, so persistence and the menu never see the mute.

use bevy::prelude::*;

/// The master slider setting, `0.0..=1.0`. Default full. Scales every sound in
/// the game through the slider dB taper (see the module docs for the two
/// application paths).
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
    /// The clamped setting, so a corrupt persisted value can never push the
    /// mixer out of range.
    ///
    /// This is the SETTING - what persistence saves and the menu slider shows.
    /// The gain the mixer actually applies is [`Self::output_gain`], which a
    /// harness run masks to silence; keeping the two apart is what stops a
    /// muted smoke run from persisting `0.0` over the player's real volume.
    pub fn factor(self) -> f32 {
        self.0.clamp(0.0, 1.0)
    }

    /// The gain audio output applies: the setting through
    /// [`slider_gain`](crate::bus::slider_gain), masked to silence when the
    /// run is [`HarnessMute`]d. ONLY the output sites (the `GlobalVolume`
    /// push and the per-frame loop-sink writes in `voice`) call this;
    /// everything that means "the player's chosen volume" reads
    /// [`Self::factor`].
    pub fn output_gain(self, mute: HarnessMute) -> f32 {
        if mute.0 {
            0.0
        } else {
            crate::bus::slider_gain(self.0)
        }
    }
}

/// Environment variable that turns the AUDIO output off.
///
/// One half of the outputs-off pair: this silences the speakers, and
/// `nova_core::NORENDER_ENV` (`NOVA_NORENDER`) drops the renderer. Each has a
/// matching debug-only flag on the game binary - `--mute` and `--norender` -
/// and each lives in the crate that owns the device it turns off. Set to
/// anything but `"0"` to mute; `"0"` forces sound even under a harness.
pub const MUTE_ENV: &str = "NOVA_MUTE";

/// Zero audio output for scripted runs - nobody listens to an autopilot run, and
/// Xvfb hides the window but not the speakers. Resolved ONCE per run (a run's
/// mute state cannot change mid-session) by [`HarnessMute::from_env`]:
/// [`MUTE_ENV`] set and not `"0"` mutes any run, `NOVA_MUTE=0` forces sound
/// even under a harness, and with [`MUTE_ENV`] unset a run is muted iff a
/// harness drives it.
///
/// [`NovaSoundPlugin`](crate::NovaSoundPlugin) starts it unmuted, because the
/// engine does not know what a harness is: `nova_gameplay`'s settings plugin
/// inserts the resolved value. The game binary's `--mute` inserts it directly,
/// and so do the tests (insert after the plugin) rather than touching process
/// env, which parallel tests would race on.
#[derive(Resource, Clone, Copy, Default, Debug, PartialEq)]
pub struct HarnessMute(pub bool);

impl HarnessMute {
    /// Resolve the mute from [`MUTE_ENV`], given whether a scripted run is
    /// driving this process.
    pub fn from_env(harness_active: bool) -> Self {
        let nova_mute = std::env::var(MUTE_ENV).ok();
        Self(harness_muted_from(nova_mute.as_deref(), harness_active))
    }
}

/// The mute decision as a pure function of its inputs: the env read stays in
/// the thin [`HarnessMute::from_env`] wrapper so this logic is unit-testable
/// without process-global env mutation.
fn harness_muted_from(nova_mute: Option<&str>, harness_active: bool) -> bool {
    match nova_mute {
        Some(explicit) => explicit != "0",
        None => harness_active,
    }
}

/// Say once whether this run is silent.
///
/// A muted run is indistinguishable from a working one until something should
/// have made a sound and did not, so the decision is stated where a log reader
/// can find it - which is also what makes `--mute` verifiable from outside the
/// process.
pub(crate) fn report_harness_mute(mute: Res<HarnessMute>) {
    if mute.0 {
        info!("nova audio: output muted for this run");
    }
}

/// Push [`MasterVolume`] onto bevy's [`GlobalVolume`], which scales every sound
/// played after this point (`audio_output` multiplies it into each new sink).
/// `Option` on the target: minimal/headless rigs without bevy's `AudioPlugin`
/// have no `GlobalVolume`, and this must not panic them. Pushes the
/// [`MasterVolume::output_gain`] (harness runs push silence), never the raw
/// setting.
pub(crate) fn apply_master_volume(
    volume: Res<MasterVolume>,
    mute: Res<HarnessMute>,
    global: Option<ResMut<GlobalVolume>>,
) {
    if let Some(mut global) = global {
        global.volume = bevy::audio::Volume::Linear(volume.output_gain(*mute));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::NovaSoundPlugin;

    /// A minimal app with the engine. Overrides the [`HarnessMute`] with an
    /// explicit unmuted one AFTER the plugin (insert wins), so these tests stay
    /// deterministic whatever the env - the mute test injects its own.
    fn app() -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.init_asset::<AudioSource>();
        app.insert_resource(GlobalVolume::default());
        app.add_plugins(NovaSoundPlugin);
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
            bevy::audio::Volume::Linear(crate::bus::slider_gain(0.3)),
            "changing MasterVolume pushes its tapered gain onto GlobalVolume"
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
        // Explicit NOVA_MUTE wins in both directions; otherwise the
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
}
