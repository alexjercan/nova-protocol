//! `PlayMusic` and `StopMusic`: a scenario's own song, in place of the
//! context rule's.
//!
//! The rule picks songs by situation (cruise, threat, combat, docked). A
//! scene that needs its own song - a boss theme, the music under a reveal -
//! plays it with `PlayMusic`, and the director hands it the deck until it
//! ends, `StopMusic` ends it, or the scenario ends. `then` says what its end
//! does, and `in_combat` whether a fight takes over.

use std::time::Duration;

use bevy::{audio::AudioSource, prelude::*};
use nova_events::prelude::*;
use nova_gameplay::prelude::*;

use crate::prelude::*;

/// Play the scenario's own song. RON:
/// `PlayMusic((sound: "self://music/boss.ogg", fade_in: 2.0, then: Loop, in_combat: Keep))`.
#[derive(Clone, Debug, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct PlayMusicActionConfig {
    /// The song, an `.ogg` file.
    pub sound: AssetRef<AudioSource>,
    /// Its fade in, in seconds: the crossfade from the song that plays.
    pub fade_in: f32,
    /// What its end does: `Resume` hands back to the situation's songs,
    /// `Loop` plays it again.
    pub then: MusicThenType,
    /// What a fight does: `Yield` lets the combat songs take over, `Keep`
    /// plays it through the fight.
    pub in_combat: MusicInCombatType,
}

/// End the scenario's song. RON: `StopMusic((fade_out: 3.0))`.
#[derive(Clone, Debug, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct StopMusicActionConfig {
    /// Its fade out, in seconds. The situation's songs come back after its
    /// normal gap.
    pub fade_out: f32,
}

/// Whether `seconds` is a fade a music action may carry: finite and not
/// negative. The lint reads it, and the action plays only what it allows.
pub fn music_fade_is_valid(seconds: f32) -> bool {
    seconds.is_finite() && seconds >= 0.0
}

impl EventAction<NovaEventWorld> for PlayMusicActionConfig {
    fn action(&self, world: &mut NovaEventWorld, _: &GameEventInfo) {
        let sound = self.sound.clone();
        let fade_in = Duration::from_secs_f32(self.fade_in.max(0.0));
        let (then, in_combat) = (self.then, self.in_combat);
        debug!(
            "PlayMusic: {:?} then {then:?}, {in_combat:?} in combat",
            sound.path()
        );

        world.push_command(move |commands| {
            commands.queue(move |world: &mut World| {
                let sound = sound.resolve(world.resource::<AssetServer>());
                world.resource_mut::<ScenarioMusic>().play(ScenarioSong {
                    sound,
                    fade_in,
                    then,
                    in_combat,
                });
            });
        });
    }
}

impl EventAction<NovaEventWorld> for StopMusicActionConfig {
    fn action(&self, world: &mut NovaEventWorld, _: &GameEventInfo) {
        let fade_out = Duration::from_secs_f32(self.fade_out.max(0.0));
        debug!("StopMusic: fade out {:.1} s", fade_out.as_secs_f32());

        world.push_command(move |commands| {
            commands.queue(move |world: &mut World| {
                world.resource_mut::<ScenarioMusic>().stop(fade_out);
            });
        });
    }
}

#[cfg(test)]
mod tests {
    use bevy::asset::AssetPlugin;
    use nova_events::prelude::EventWorld;

    use super::*;

    fn music_app() -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.init_asset::<AudioSource>();
        app.init_resource::<NovaEventWorld>();
        app.init_resource::<GameObjectives>();
        app.init_resource::<ScenarioMusic>();
        app
    }

    fn fire(app: &mut App, action: &impl EventAction<NovaEventWorld>) {
        let mut world = app.world_mut().resource_mut::<NovaEventWorld>();
        action.action(&mut world, &GameEventInfo::default());
        NovaEventWorld::state_to_world_system(app.world_mut());
        app.update();
    }

    #[test]
    fn play_music_hands_the_song_and_its_rules_to_the_director() {
        let mut app = music_app();
        fire(
            &mut app,
            &PlayMusicActionConfig {
                sound: AssetRef::from("music/boss.ogg"),
                fade_in: 2.0,
                then: MusicThenType::Loop,
                in_combat: MusicInCombatType::Keep,
            },
        );

        let mut music = app.world_mut().resource_mut::<ScenarioMusic>();
        let mut director = MusicDirector::default();
        let tracks = GameMusicTracks::default();
        let tick = crate::music::tests::flight_tick(&tracks);
        let cues = director.tick(tick, &mut music, &mut crate::music::tests::seeded());
        let song = music.song().expect("the scenario song plays");
        assert_eq!(song.fade_in, Duration::from_secs(2));
        assert_eq!(song.then, MusicThenType::Loop);
        assert_eq!(song.in_combat, MusicInCombatType::Keep);
        assert!(
            matches!(&cues[..], [MusicCue::Play { song: MusicSong::Scenario(_), fade }] if *fade == Duration::from_secs(2)),
            "{cues:?}"
        );
    }

    #[cfg(feature = "serde")]
    #[test]
    fn music_actions_round_trip_through_authored_ron() {
        let authored = r#"PlayMusic((sound: "self://music/boss.ogg", fade_in: 2.0, then: Resume, in_combat: Yield))"#;
        let parsed: EventActionConfig = ron::from_str(authored).expect("PlayMusic parses");
        let EventActionConfig::PlayMusic(config) = &parsed else {
            panic!("PlayMusic variant");
        };
        assert_eq!(config.then, MusicThenType::Resume);
        assert_eq!(config.in_combat, MusicInCombatType::Yield);

        let missing = r#"PlayMusic((sound: "self://music/boss.ogg", fade_in: 2.0, then: Resume))"#;
        assert!(
            ron::from_str::<EventActionConfig>(missing).is_err(),
            "every field is required"
        );

        let stop = r#"StopMusic((fade_out: 3.0))"#;
        let parsed: EventActionConfig = ron::from_str(stop).expect("StopMusic parses");
        assert!(matches!(parsed, EventActionConfig::StopMusic(_)));
    }
}
