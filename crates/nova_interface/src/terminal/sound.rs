//! The NOVA OS audio: keypress and control cues, the continuous power-on bed,
//! and the power-down sting.
//!
//! Every cue and the bed are [`SoundBus::Ui`](nova_sound::prelude::SoundBus)
//! categories - the terminal is chrome, not a thing in the world - so none of
//! them is placed, attenuated or panned, and the bed keeps humming while the
//! sim behind it is frozen.
//!
//! The bed is a single [`SoundLoop`] owner so the sound toggle can turn it on
//! and off live rather than restarting it.
//!
//! Touch this module when adding a sound the monitor makes.

use bevy::prelude::*;
use nova_sound::prelude::{PlaySound, SoundBank, SoundCategoryType, SoundLoop, UiSfx};

use super::components::*;

/// The looping ambient CRT bed, spawned while the NOVA OS monitor is open.
///
/// Its CATEGORY is what keeps it playing through the sim freeze on
/// `OnEnter(Commands)`: the engine pauses the world voices and leaves the
/// interface alone, so the bed is exempt by construction rather than by a guard
/// (`audit-state-gates-on-new-entry-path`). The SND toggle is applied live by
/// [`apply_nova_os_bed_volume`].
#[derive(Component)]
pub(crate) struct NovaOsBedSfx;

/// Fire a one-shot NOVA OS terminal cue, honoring the SND toggle
/// ([`NovaOsMonitorSettings::sound_enabled`]). The category sets the level;
/// the interface-bus and master gains are the engine's, like every other cue.
///
/// Public because the command dispatcher lives above this crate and cues its
/// own answer: a command's ok/error lands after the world has been touched, not
/// when the line was typed.
pub fn play_nova_os_cue(
    commands: &mut Commands,
    bank: &SoundBank<UiSfx>,
    settings: &NovaOsMonitorSettings,
    cue: UiSfx,
    category: SoundCategoryType,
) {
    if !settings.sound_enabled {
        return;
    }
    commands.trigger(PlaySound::ui(bank.get(cue), category));
}

/// Power-up sweep + start the ambient bed when the monitor opens
/// (`OnEnter(Commands)`). The bed spawns even when SND is off (silent) so toggling
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
        SoundCategoryType::UiConfirm,
    );
    commands.spawn((
        Name::new("NOVA OS Ambient Bed"),
        NovaOsBedSfx,
        SoundLoop::new(bank.get(UiSfx::NovaOsBed), SoundCategoryType::TerminalBed),
    ));
}

/// Despawn the ambient bed when the monitor closes (`OnExit(Commands)`, i.e. once
/// the power-down collapse finishes).
pub(crate) fn stop_nova_os_bed(mut commands: Commands, q_bed: Query<Entity, With<NovaOsBedSfx>>) {
    for entity in &q_bed {
        commands.entity(entity).despawn();
    }
}

/// Play the power-down sweep the instant a close is REQUESTED (the rising edge of
/// [`NovaOsCloseTransition::closing`]), so the sweep syncs with the raster
/// collapse that starts then - not `OnExit(Commands)`, which fires only after the
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
                SoundCategoryType::UiConfirm,
            );
        }
    }
    *was_closing = close.closing;
}

/// Turn the ambient bed on or off from the SND toggle, so muting SND silences
/// the hum live without despawning the loop. The level is the TerminalBed
/// row's; the interface-bus, master and harness-mute gains are the engine's.
pub(crate) fn apply_nova_os_bed_volume(
    settings: Res<NovaOsMonitorSettings>,
    mut q_bed: Query<&mut SoundLoop, With<NovaOsBedSfx>>,
) {
    let intensity = nova_os_bed_intensity(settings.sound_enabled);
    for mut bed in &mut q_bed {
        bed.intensity = intensity;
    }
}

/// The ambient bed's loop intensity: on, or ZERO when SND is muted. Pure so
/// the SND-off silence is testable without an audio device.
pub(crate) fn nova_os_bed_intensity(sound_enabled: bool) -> f32 {
    if sound_enabled {
        1.0
    } else {
        0.0
    }
}
