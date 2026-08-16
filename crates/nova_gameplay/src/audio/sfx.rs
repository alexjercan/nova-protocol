//! Fire-and-forget one-shot sound effects for Bevy games.
//!
//! Games trigger a [`PlaySfx`] (or call [`SfxCommandsExt::play_sfx`]) with an
//! [`AudioSource`] handle and [`SfxPlugin`] spawns a self-despawning
//! [`AudioPlayer`] for it, so game code never repeats the `AudioPlayer` /
//! `PlaybackSettings::DESPAWN` boilerplate. A global [`SfxMasterVolume`]
//! resource scales every sound, giving one place to wire a volume slider or a
//! mute toggle.
//!
//! This is deliberately just one concern: transient, non-looping SFX. It is
//! not a music player or a mixer; for looping background music spawn an
//! `AudioPlayer` with `PlaybackSettings::LOOP` directly.
//!
//! ```rust
//! # use bevy::prelude::*;
//! # use nova_gameplay::prelude::*;
//! # fn demo(mut commands: Commands, slice: Handle<AudioSource>) {
//! // Simplest: play once at master volume.
//! commands.play_sfx(slice.clone());
//!
//! // Or trigger directly for per-shot volume / pitch control.
//! commands.trigger(PlaySfx::new(slice).with_volume(0.8).with_speed(1.2));
//! # }
//! ```
//!
//! Nova owns this because every cue in the game - the ship's combat
//! one-shots, the HUD clicks and the menu blips - is fired through [`PlaySfx`],
//! and the
//! master volume it scales by is the slider in nova's settings screen.

use bevy::{audio::Volume, prelude::*};

/// Request to play a one-shot sound effect.
///
/// Trigger it with `commands.trigger(PlaySfx::new(handle))`; [`SfxPlugin`]
/// observes it and spawns the audio entity. Prefer [`SfxCommandsExt`] for the
/// common cases.
#[derive(Event, Clone, Debug, Reflect)]
pub struct PlaySfx {
    /// The sound to play.
    pub handle: Handle<AudioSource>,

    /// Per-shot linear volume multiplier (1.0 leaves the clip unchanged). It is
    /// multiplied by [`SfxMasterVolume`] before playback.
    pub volume: f32,

    /// Playback speed, which also shifts pitch (1.0 is normal). Handy for
    /// adding variation, e.g. nudging the pitch up as a combo grows.
    pub speed: f32,
}

impl PlaySfx {
    /// A sound at full per-shot volume and normal speed.
    pub fn new(handle: Handle<AudioSource>) -> Self {
        Self {
            handle,
            volume: 1.0,
            speed: 1.0,
        }
    }

    /// Set the per-shot linear volume multiplier.
    pub fn with_volume(mut self, volume: f32) -> Self {
        self.volume = volume;
        self
    }

    /// Set the playback speed (and pitch).
    pub fn with_speed(mut self, speed: f32) -> Self {
        self.speed = speed;
        self
    }
}

/// Marks the audio entity behind one [`PlaySfx`] one-shot.
///
/// Exists for owners OUTSIDE this crate: the entity's own despawn rides its
/// audio sink (`PlaybackSettings::DESPAWN`), which plays on the wall clock and
/// is never created without an output device, so nothing else can recognize a
/// clip that outlived whoever asked for it. nova_scenario scopes on this
/// marker so a one-shot spawned by a live scenario dies with the teardown
/// instead of playing into the next scenario.
#[derive(Component, Debug, Clone, Reflect)]
pub struct SfxAudioMarker;

/// Master linear volume applied to every sound effect (default 1.0).
///
/// Set it to scale all SFX at once (a volume slider), or to 0.0 to mute.
#[derive(Resource, Clone, Debug, Reflect, Deref, DerefMut)]
pub struct SfxMasterVolume(pub f32);

impl Default for SfxMasterVolume {
    fn default() -> Self {
        Self(1.0)
    }
}

/// Ergonomic [`Commands`] extension for firing sound effects.
pub trait SfxCommandsExt {
    /// Play `handle` once at master volume.
    fn play_sfx(&mut self, handle: Handle<AudioSource>);

    /// Play `handle` once with a per-shot volume multiplier.
    fn play_sfx_volume(&mut self, handle: Handle<AudioSource>, volume: f32);
}

impl SfxCommandsExt for Commands<'_, '_> {
    fn play_sfx(&mut self, handle: Handle<AudioSource>) {
        self.trigger(PlaySfx::new(handle));
    }

    fn play_sfx_volume(&mut self, handle: Handle<AudioSource>, volume: f32) {
        self.trigger(PlaySfx::new(handle).with_volume(volume));
    }
}

/// Plugin that enables fire-and-forget SFX playback via [`PlaySfx`].
#[derive(Default)]
pub struct SfxPlugin;

impl Plugin for SfxPlugin {
    fn build(&self, app: &mut App) {
        debug!("SfxPlugin: build");

        app.init_resource::<SfxMasterVolume>();
        app.register_type::<SfxMasterVolume>();
        app.register_type::<SfxAudioMarker>();
        app.add_observer(on_play_sfx);
    }
}

/// Spawn a self-despawning [`AudioPlayer`] for each [`PlaySfx`], scaled by the
/// master volume. `PlaybackSettings::DESPAWN` retires the entity once the clip
/// finishes, so callers never have to clean it up.
fn on_play_sfx(event: On<PlaySfx>, mut commands: Commands, master: Res<SfxMasterVolume>) {
    let volume = (event.volume * master.0).max(0.0);
    // NOTE: rodio does not accept a non-positive playback rate.
    let speed = event.speed.max(f32::MIN_POSITIVE);
    trace!("on_play_sfx: volume {volume}, speed {speed}");

    commands.spawn((
        Name::new("Sfx"),
        SfxAudioMarker,
        AudioPlayer(event.handle.clone()),
        PlaybackSettings::DESPAWN
            .with_volume(Volume::Linear(volume))
            .with_speed(speed),
    ));
}
