//! The NOVA OS audio: keypress and control cues, the continuous power-on bed,
//! and the power-down sting.
//!
//! Every cue and the bed are on [`AudioRoute::Interface`] - the terminal is
//! chrome, not a thing in the world - so none of them is placed, attenuated or
//! panned, and the bed keeps humming while the sim behind it is frozen.
//!
//! The bed is a single entity so its gain can track the sound toggle live
//! rather than being restarted.
//!
//! Touch this module when adding a sound the monitor makes.

use bevy::prelude::*;
use nova_gameplay::audio::prelude::{
    AudioRoute, SfxCommandsExt, SfxVoice, SoundBank, UiSfx, NOVA_OS_BED_VOLUME,
    NOVA_OS_POWER_VOLUME,
};

use super::components::*;

/// The looping ambient CRT bed, spawned while the NOVA OS computer is open.
///
/// Its ROUTE is what keeps it playing through the sim freeze on
/// `OnEnter(NovaOs)`: the engine pauses the world voices and leaves the
/// interface alone, so the bed is exempt by construction rather than by a guard
/// (`audit-state-gates-on-new-entry-path`). The SND toggle is applied live by
/// [`apply_nova_os_bed_volume`].
#[derive(Component)]
pub(crate) struct NovaOsBedSfx;

/// Fire a one-shot NOVA OS terminal cue, honoring the SND toggle
/// ([`NovaOsMonitorSettings::sound_enabled`]). The interface-bus and master
/// gains are applied downstream by the engine, like every other cue.
///
/// Public because the command dispatcher lives above this crate and cues its
/// own answer: a command's ok/error lands after the world has been touched, not
/// when the line was typed.
pub fn play_nova_os_cue(
    commands: &mut Commands,
    bank: &SoundBank<UiSfx>,
    settings: &NovaOsMonitorSettings,
    cue: UiSfx,
    volume: f32,
) {
    if !settings.sound_enabled {
        return;
    }
    commands.play_sfx(bank.get(cue), AudioRoute::Interface, volume);
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
        SfxVoice::looping(bank.get(UiSfx::NovaOsBed), AudioRoute::Interface),
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

/// Drive the ambient bed's level from the SND toggle, so muting SND silences the
/// hum live without despawning the loop. The interface-bus, master and
/// harness-mute gains are the engine's, applied on top of what this writes.
pub(crate) fn apply_nova_os_bed_volume(
    settings: Res<NovaOsMonitorSettings>,
    mut q_bed: Query<&mut SfxVoice, With<NovaOsBedSfx>>,
) {
    let target = nova_os_bed_gain(settings.sound_enabled);
    for mut voice in &mut q_bed {
        voice.volume = target;
    }
}

/// The ambient bed's level: its base volume, or ZERO when SND is muted. Pure so
/// the SND-off silence is testable without an audio device.
pub(crate) fn nova_os_bed_gain(sound_enabled: bool) -> f32 {
    if sound_enabled {
        NOVA_OS_BED_VOLUME
    } else {
        0.0
    }
}
