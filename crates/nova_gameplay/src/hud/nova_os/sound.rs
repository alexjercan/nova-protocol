use bevy::{audio::Volume, prelude::*};

use super::components::*;
use crate::{
    audio::{SfxCommandsExt, SoundBank, UiSfx, NOVA_OS_BED_VOLUME, NOVA_OS_POWER_VOLUME},
    settings::{HarnessMute, MasterVolume},
};

/// The looping ambient CRT bed audio entity, spawned while the NOVA OS computer
/// is open. NOTE: its OWN marker keeps it out of
/// [`crate::audio`]'s `pause_loops` thruster/RCS queries, so the sim-freeze
/// loop-pause on `OnEnter(NovaOs)` never silences it - the bed is exempt by
/// construction, not by a guard (`audit-state-gates-on-new-entry-path`). Volume
/// (and the live SND mute) is applied by [`apply_nova_os_bed_volume`].
#[derive(Component)]
pub(crate) struct NovaOsBedSfx;

/// Fire a one-shot NOVA OS terminal cue, honoring the SND toggle
/// ([`NovaOsMonitorSettings::sound_enabled`]). Master volume is applied
/// downstream by the SFX plugin's `SfxMasterVolume` path, like every other cue.
pub(crate) fn play_nova_os_cue(
    commands: &mut Commands,
    bank: &SoundBank<UiSfx>,
    settings: &NovaOsMonitorSettings,
    cue: UiSfx,
    volume: f32,
) {
    if !settings.sound_enabled {
        return;
    }
    commands.play_sfx_volume(bank.get(cue), volume);
}

/// Power-up sweep + start the ambient bed when the computer opens
/// (`OnEnter(NovaOs)`). The bed spawns even when SND is off (silent) so toggling
/// SND on mid-session brings the hum in without reopening.
pub(crate) fn start_nova_os_sound(
    mut commands: Commands,
    bank: Option<Res<SoundBank<UiSfx>>>,
    settings: Res<NovaOsMonitorSettings>,
) {
    let Some(bank) = bank else {
        return;
    };
    play_nova_os_cue(
        &mut commands,
        &bank,
        &settings,
        UiSfx::NovaOsPowerUp,
        NOVA_OS_POWER_VOLUME,
    );
    commands.spawn((
        Name::new("NOVA OS Ambient Bed"),
        NovaOsBedSfx,
        AudioPlayer(bank.get(UiSfx::NovaOsBed)),
        PlaybackSettings::LOOP.with_volume(Volume::Linear(0.0)),
    ));
}

/// Despawn the ambient bed when the computer closes (`OnExit(NovaOs)`, i.e. once
/// the power-down collapse finishes).
pub(crate) fn stop_nova_os_bed(mut commands: Commands, q_bed: Query<Entity, With<NovaOsBedSfx>>) {
    for entity in &q_bed {
        commands.entity(entity).despawn();
    }
}

/// Play the power-down sweep the instant a close is REQUESTED (the rising edge of
/// [`NovaOsCloseTransition::closing`]), so the sweep syncs with the raster
/// collapse that starts then - not `OnExit(NovaOs)`, which fires only after the
/// collapse animation completes.
pub(crate) fn play_nova_os_power_down(
    mut commands: Commands,
    bank: Option<Res<SoundBank<UiSfx>>>,
    settings: Res<NovaOsMonitorSettings>,
    close: Res<NovaOsCloseTransition>,
    mut was_closing: Local<bool>,
) {
    if close.closing && !*was_closing {
        if let Some(bank) = &bank {
            play_nova_os_cue(
                &mut commands,
                bank,
                &settings,
                UiSfx::NovaOsPowerDown,
                NOVA_OS_POWER_VOLUME,
            );
        }
    }
    *was_closing = close.closing;
}

/// Drive the ambient bed sink volume from [`MasterVolume`] and the SND toggle, so
/// muting SND (or the master) silences the hum live without despawning the loop.
/// Uses `output_gain(mute)` like the thruster/RCS loop sinks, so a `HarnessMute`d
/// smoke/probe run silences the bed too (a per-frame sink write bypasses the
/// `GlobalVolume` path that mute otherwise masks).
pub(crate) fn apply_nova_os_bed_volume(
    settings: Res<NovaOsMonitorSettings>,
    master: Option<Res<MasterVolume>>,
    mute: Option<Res<HarnessMute>>,
    mut q_bed: Query<&mut AudioSink, With<NovaOsBedSfx>>,
) {
    let mute = mute.map(|m| *m).unwrap_or_default();
    let master = master.map(|m| m.output_gain(mute)).unwrap_or(1.0);
    let target = nova_os_bed_gain(settings.sound_enabled, master);
    for mut sink in &mut q_bed {
        sink.set_volume(Volume::Linear(target));
    }
}

/// The ambient bed's target sink gain: the base volume scaled by the master
/// output gain, or ZERO when SND is muted. Pure so the SND-off / master / mute
/// silence logic is testable without an `AudioSink` (which needs an audio
/// device). `master` is already the `output_gain(mute)`, so a harness-muted run
/// (master 0) silences the bed too.
pub(crate) fn nova_os_bed_gain(sound_enabled: bool, master: f32) -> f32 {
    if sound_enabled {
        NOVA_OS_BED_VOLUME * master
    } else {
        0.0
    }
}
