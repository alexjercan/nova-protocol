//! `PlaySound`: an authored one-shot, fired from a beat.
//!
//! Sound that BELONGS to something in the world - a crate's pickup ding, a
//! hull's collapse - is authored on that object and played where it happens.
//! This is the other kind: the cue a scene needs and nothing in the world
//! produces. A door tone under a comms line, the hit that lands off-screen,
//! the alarm that starts a chapter.
//!
//! An Interface cue is heard flat. A Hull cue plays in the world at the
//! player's ship, as a [`SoundCategoryType::Machinery`] sound: the category
//! sets its level, so `volume` applies to Interface cues only.

use bevy::{audio::AudioSource, prelude::*};
use nova_events::prelude::*;
use nova_gameplay::prelude::*;
use nova_sound::prelude::*;

use crate::prelude::*;

/// Volume a cue plays at when the author does not say.
const DEFAULT_SOUND_VOLUME: f32 = 1.0;

/// Where an authored cue is heard.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum SoundRouteConfig {
    /// UI chrome: a prompt, a stinger, a chapter tone. Scaled by the interface
    /// track.
    #[default]
    Interface,
    /// On the player's own ship, in the world. Scaled by the effects track.
    /// Dropped with a warning when there is no player ship.
    Hull,
}

/// Play one authored sound. RON:
/// `PlaySound((sound: "self://sounds/alarm.wav", route: Hull))`.
#[derive(Clone, Debug, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PlaySoundActionConfig {
    /// The clip, as any other content sound ref.
    pub sound: AssetRef<AudioSource>,
    /// Where it is heard. Authored, because "in the cockpit" and "through the
    /// hull" are different cues and neither is the safe guess.
    pub route: SoundRouteConfig,
    /// Optional gain for an Interface cue. Omit for full volume; `Some(0.4)`
    /// for a cue that sits under a comms line rather than over it.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub volume: Option<f32>,
}

impl EventAction<NovaEventWorld> for PlaySoundActionConfig {
    fn action(&self, world: &mut NovaEventWorld, _: &GameEventInfo) {
        let sound = self.sound.clone();
        let route = self.route;
        let volume = self.volume.unwrap_or(DEFAULT_SOUND_VOLUME);
        debug!("PlaySound: {:?} on {route:?}", sound.path());

        world.push_command(move |commands| {
            commands.queue(move |world: &mut World| {
                let handle = {
                    let asset_server = world.resource::<AssetServer>();
                    sound.resolve(asset_server)
                };
                match route {
                    SoundRouteConfig::Interface => {
                        world
                            .commands()
                            .play_sfx(handle, AudioRoute::Interface, volume);
                    }
                    SoundRouteConfig::Hull => {
                        let ship = world
                            .query_filtered::<Entity, With<PlayerSpaceshipMarker>>()
                            .iter(world)
                            .next();
                        let Some(ship) = ship else {
                            warn!("PlaySound: dropped a Hull cue: there is no player ship");
                            return;
                        };
                        world.trigger(PlaySound {
                            sound: handle,
                            category: SoundCategoryType::Machinery,
                            at: SoundAt::Entity(ship),
                        });
                    }
                }
            });
        });
    }
}

#[cfg(test)]
mod tests {
    use bevy::asset::AssetPlugin;
    use nova_events::prelude::EventWorld;

    use super::*;

    /// Every cue the app heard this frame.
    #[derive(Resource, Default)]
    struct Heard(Vec<PlaySfx>);

    fn record(play: On<PlaySfx>, mut heard: ResMut<Heard>) {
        heard.0.push(play.event().clone());
    }

    /// Every world cue the app heard.
    #[derive(Resource, Default)]
    struct HeardInWorld(Vec<PlaySound>);

    fn record_world(play: On<PlaySound>, mut heard: ResMut<HeardInWorld>) {
        heard.0.push(play.event().clone());
    }

    /// A headless app that can resolve an `AssetRef` and observe the cue,
    /// without an output device: the action's whole job is turning an authored
    /// row into one `PlaySfx`, and the mixer is nova_gameplay's to test.
    fn audio_app() -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.init_asset::<AudioSource>();
        app.init_resource::<NovaEventWorld>();
        app.init_resource::<GameObjectives>();
        app.init_resource::<Heard>();
        app.init_resource::<HeardInWorld>();
        app.add_observer(record);
        app.add_observer(record_world);
        app
    }

    fn fire(app: &mut App, action: PlaySoundActionConfig) {
        let mut world = app.world_mut().resource_mut::<NovaEventWorld>();
        action.action(&mut world, &GameEventInfo::default());
        NovaEventWorld::state_to_world_system(app.world_mut());
        app.update();
    }

    /// A Hull cue plays in the world at the player's ship.
    #[test]
    fn a_hull_cue_plays_at_the_player_ship() {
        let mut app = audio_app();
        let ship = app.world_mut().spawn(PlayerSpaceshipMarker).id();
        fire(
            &mut app,
            PlaySoundActionConfig {
                sound: AssetRef::from("sounds/alarm.ogg"),
                route: SoundRouteConfig::Hull,
                volume: None,
            },
        );

        let heard = &app.world().resource::<HeardInWorld>().0;
        assert_eq!(heard.len(), 1, "one row, one cue");
        assert_eq!(heard[0].category, SoundCategoryType::Machinery);
        assert_eq!(heard[0].at, SoundAt::Entity(ship));
        assert!(app.world().resource::<Heard>().0.is_empty());
    }

    /// An omitted volume is full volume, not silence.
    #[test]
    fn a_cue_with_no_authored_gain_plays_at_full_volume() {
        let mut app = audio_app();
        fire(
            &mut app,
            PlaySoundActionConfig {
                sound: AssetRef::from("sounds/alarm.ogg"),
                route: SoundRouteConfig::Interface,
                volume: None,
            },
        );

        let heard = &app.world().resource::<Heard>().0;
        assert_eq!(heard[0].volume, DEFAULT_SOUND_VOLUME);
        assert_eq!(heard[0].route, AudioRoute::Interface);
    }

    #[cfg(feature = "serde")]
    #[test]
    fn a_cue_round_trips_through_authored_ron() {
        let authored =
            r#"PlaySound((sound: "self://sounds/alarm.ogg", route: Hull, volume: Some(0.4)))"#;
        let parsed: EventActionConfig = ron::from_str(authored).expect("the cue parses");
        let EventActionConfig::PlaySound(config) = &parsed else {
            panic!("PlaySound variant");
        };
        assert_eq!(config.route, SoundRouteConfig::Hull);
        assert_eq!(config.volume, Some(0.4));

        let bare = r#"PlaySound((sound: "self://sounds/alarm.ogg", route: Interface))"#;
        let parsed: EventActionConfig = ron::from_str(bare).expect("an omitted gain parses");
        let EventActionConfig::PlaySound(config) = &parsed else {
            panic!("PlaySound variant");
        };
        assert!(config.volume.is_none());
    }
}
