//! Fire-and-forget caller-levelled interface one-shots.
//!
//! Trigger a [`PlaySfx`] (or call [`SfxCommandsExt::play_sfx`]) and
//! [`SfxPlugin`] spawns the [`SfxVoice`] entity for it and retires it when the
//! clip ends. The cue is heard flat on the interface slider at the caller's
//! volume. A world sound is a [`PlaySound`](super::PlaySound), whose category
//! sets the level.
//!
//! ```rust
//! # use bevy::prelude::*;
//! # use nova_sound::prelude::*;
//! # fn demo(mut commands: Commands, click: Handle<AudioSource>) {
//! commands.play_sfx(click, AudioRoute::Interface, 0.3);
//! # }
//! ```

use bevy::prelude::*;

use super::{bus::AudioRoute, voice::SfxVoice};

/// Request to play a one-shot sound effect.
///
/// Trigger it with `commands.trigger(PlaySfx::new(handle, route))`; [`SfxPlugin`]
/// observes it and spawns the voice. Prefer [`SfxCommandsExt`] for the common
/// cases.
#[derive(Event, Clone, Debug)]
pub struct PlaySfx {
    /// The sound to play.
    pub handle: Handle<AudioSource>,

    /// Which track scales the cue.
    pub route: AudioRoute,

    /// Per-shot linear volume multiplier (1.0 leaves the clip unchanged). The
    /// bus gain and the master are applied on top.
    pub volume: f32,

    /// Playback speed, which also shifts pitch (1.0 is normal). Handy for
    /// adding variation, e.g. nudging the pitch up as a combo grows.
    pub speed: f32,
}

impl PlaySfx {
    /// A cue on `route`, at full per-shot volume and normal speed.
    pub fn new(handle: Handle<AudioSource>, route: AudioRoute) -> Self {
        Self {
            handle,
            route,
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

/// Marks the voice entity behind one [`PlaySfx`] one-shot.
///
/// Exists for owners OUTSIDE this crate: the entity's own despawn rides its
/// audio sink (`PlaybackMode::Despawn`), which plays on the wall clock and is
/// never created without an output device, so nothing else can recognize a clip
/// that outlived whoever asked for it. nova_scenario scopes on this marker so a
/// one-shot spawned by a live scenario dies with the teardown instead of
/// playing into the next scenario.
#[derive(Component, Debug, Clone, Reflect)]
pub struct SfxAudioMarker;

/// Ergonomic [`Commands`] extension for firing one-shot cues.
pub trait SfxCommandsExt {
    /// Play `handle` once on `route`, at `volume`.
    fn play_sfx(&mut self, handle: Handle<AudioSource>, route: AudioRoute, volume: f32);
}

impl SfxCommandsExt for Commands<'_, '_> {
    fn play_sfx(&mut self, handle: Handle<AudioSource>, route: AudioRoute, volume: f32) {
        self.trigger(PlaySfx::new(handle, route).with_volume(volume));
    }
}

/// Plugin that enables fire-and-forget one-shots via [`PlaySfx`].
///
/// Playback itself belongs to [`NovaSoundPlugin`](super::NovaSoundPlugin),
/// which adds this one; on its own this only turns triggers into voices.
#[derive(Default)]
pub struct SfxPlugin;

impl Plugin for SfxPlugin {
    fn build(&self, app: &mut App) {
        trace!("SfxPlugin: build");

        app.register_type::<SfxAudioMarker>();
        app.add_observer(on_play_sfx);
    }
}

/// Turn each [`PlaySfx`] into a self-retiring [`SfxVoice`]. The mixing - bus
/// gain and dropping a cue nobody could hear - is the engine's, one system
/// later.
fn on_play_sfx(event: On<PlaySfx>, mut commands: Commands) {
    trace!("on_play_sfx: {:?}", event.route);

    commands.spawn((
        Name::new("Sfx"),
        SfxAudioMarker,
        SfxVoice {
            handle: event.handle.clone(),
            route: event.route,
            volume: event.volume,
            speed: event.speed,
            looping: false,
        },
    ));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sfx_app() -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.init_asset::<AudioSource>();
        app.add_plugins(SfxPlugin);
        app
    }

    #[test]
    fn a_triggered_cue_becomes_a_one_shot_voice_at_its_volume() {
        let mut app = sfx_app();
        app.world_mut()
            .commands()
            .play_sfx(Handle::default(), AudioRoute::Interface, 0.4);
        app.world_mut().flush();
        app.update();

        let mut voices = app.world_mut().query::<&SfxVoice>();
        let voice = voices
            .iter(app.world())
            .next()
            .expect("the trigger spawned a voice");
        assert_eq!(voice.route, AudioRoute::Interface);
        assert_eq!(voice.volume, 0.4);
        assert!(!voice.looping, "a PlaySfx is always a one-shot");
    }
}
