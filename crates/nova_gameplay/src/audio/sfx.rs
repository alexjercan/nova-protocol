//! Fire-and-forget one-shot sound effects.
//!
//! Trigger a [`PlaySfx`] (or call one of the [`SfxCommandsExt`] shorthands) and
//! [`SfxPlugin`] spawns the [`SfxVoice`] entity for it and retires it when the
//! clip ends, so game code never repeats the spawn/despawn boilerplate and
//! never has to remember where the listener is - the engine mixes the cue.
//!
//! Every cue declares its [`AudioRoute`], because there is no sensible default:
//! a menu click, your own gun and someone else's gun belong on three different
//! places in the mix.
//!
//! ```rust
//! # use bevy::prelude::*;
//! # use nova_gameplay::prelude::*;
//! # fn demo(mut commands: Commands, click: Handle<AudioSource>, blast: Handle<AudioSource>) {
//! // UI chrome: no position, no rolloff, no pan.
//! commands.play_sfx(click, AudioRoute::Interface, 0.3);
//!
//! // Something happening out in the world: attenuated and panned by bearing.
//! commands.play_sfx_at(blast, AudioRoute::Exterior, 0.4, Vec3::new(80.0, 0.0, -20.0));
//! # }
//! ```

use bevy::prelude::*;

use super::{
    bus::AudioRoute,
    voice::{SfxSource, SfxVoice},
};

/// Request to play a one-shot sound effect.
///
/// Trigger it with `commands.trigger(PlaySfx::new(handle, route))`; [`SfxPlugin`]
/// observes it and spawns the voice. Prefer [`SfxCommandsExt`] for the common
/// cases.
#[derive(Event, Clone, Debug)]
pub struct PlaySfx {
    /// The sound to play.
    pub handle: Handle<AudioSource>,

    /// Which track scales the cue, and whether it is placed in the world.
    pub route: AudioRoute,

    /// Per-shot linear volume multiplier (1.0 leaves the clip unchanged). The
    /// bus gain, the distance rolloff and the master are applied on top.
    pub volume: f32,

    /// Playback speed, which also shifts pitch (1.0 is normal). Handy for
    /// adding variation, e.g. nudging the pitch up as a combo grows.
    pub speed: f32,

    /// Where the cue is heard from. Read only on
    /// [`AudioRoute::Exterior`]; a hull cue carries its position for the
    /// reader's benefit and is heard in the cockpit either way.
    pub source: SfxSource,
}

impl PlaySfx {
    /// A cue on `route`, at full per-shot volume and normal speed.
    pub fn new(handle: Handle<AudioSource>, route: AudioRoute) -> Self {
        Self {
            handle,
            route,
            volume: 1.0,
            speed: 1.0,
            source: SfxSource::Unplaced,
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

    /// Hear the cue from a world point.
    pub fn at(mut self, position: Vec3) -> Self {
        self.source = SfxSource::At(position);
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

    /// Play `handle` once on `route`, heard from `position`. On
    /// [`AudioRoute::Exterior`] that means distance-attenuated and panned by
    /// bearing; on [`AudioRoute::Hull`] the position says WHERE on your ship it
    /// happened and the cue is still heard flat in the cockpit.
    fn play_sfx_at(
        &mut self,
        handle: Handle<AudioSource>,
        route: AudioRoute,
        volume: f32,
        position: Vec3,
    );
}

impl SfxCommandsExt for Commands<'_, '_> {
    fn play_sfx(&mut self, handle: Handle<AudioSource>, route: AudioRoute, volume: f32) {
        self.trigger(PlaySfx::new(handle, route).with_volume(volume));
    }

    fn play_sfx_at(
        &mut self,
        handle: Handle<AudioSource>,
        route: AudioRoute,
        volume: f32,
        position: Vec3,
    ) {
        self.trigger(PlaySfx::new(handle, route).with_volume(volume).at(position));
    }
}

/// Plugin that enables fire-and-forget one-shots via [`PlaySfx`].
///
/// Playback itself belongs to [`NovaAudioPlugin`](super::NovaAudioPlugin),
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
/// gain, rolloff, pan, and dropping a cue nobody could hear - is the engine's,
/// one system later.
fn on_play_sfx(event: On<PlaySfx>, mut commands: Commands) {
    trace!("on_play_sfx: {:?} at {:?}", event.route, event.source);

    commands.spawn((
        Name::new("Sfx"),
        SfxAudioMarker,
        SfxVoice {
            handle: event.handle.clone(),
            route: event.route,
            volume: event.volume,
            speed: event.speed,
            source: event.source,
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
    fn a_triggered_cue_becomes_a_voice_carrying_its_route_and_place() {
        let mut app = sfx_app();
        let where_it_happened = Vec3::new(12.0, 0.0, -3.0);
        app.world_mut().trigger(
            PlaySfx::new(Handle::default(), AudioRoute::Exterior)
                .with_volume(0.4)
                .at(where_it_happened),
        );
        app.update();

        let mut voices = app.world_mut().query::<&SfxVoice>();
        let voice = voices
            .iter(app.world())
            .next()
            .expect("the trigger spawned a voice");
        assert_eq!(voice.route, AudioRoute::Exterior);
        assert_eq!(voice.volume, 0.4);
        assert_eq!(voice.source, SfxSource::At(where_it_happened));
        assert!(!voice.looping, "a PlaySfx is always a one-shot");
    }

    #[test]
    fn the_commands_shorthands_name_the_route_they_fire_on() {
        let mut app = sfx_app();
        let world = app.world_mut();
        let mut commands = world.commands();
        commands.play_sfx(Handle::default(), AudioRoute::Interface, 0.25);
        commands.play_sfx_at(Handle::default(), AudioRoute::Hull, 0.5, Vec3::X);
        world.flush();
        app.update();

        let mut voices = app.world_mut().query::<&SfxVoice>();
        let routes: Vec<AudioRoute> = voices.iter(app.world()).map(|v| v.route).collect();
        assert_eq!(routes.len(), 2);
        assert!(routes.contains(&AudioRoute::Interface));
        assert!(routes.contains(&AudioRoute::Hull));
    }
}
