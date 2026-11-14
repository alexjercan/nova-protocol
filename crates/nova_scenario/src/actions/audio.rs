//! `PlaySound`: an authored one-shot, fired from a beat.
//!
//! Sound that BELONGS to something in the world - a crate's pickup ding, a
//! hull's collapse - is authored on that object and played where it happens.
//! This is the other kind: the cue a scene needs and nothing in the world
//! produces. A door tone under a comms line, the hit that lands off-screen,
//! the alarm that starts a chapter.
//!
//! The author names the cue's [`SoundCategoryType`] and where it plays. The
//! category's row in the sound table sets the level, so the action carries no
//! gain. A [`SoundBus::Ui`] category plays at [`SoundAtConfig::Ui`], heard
//! flat; a [`SoundBus::Effect`] category plays at [`SoundAtConfig::Body`], in
//! the world at that body. The lint refuses any other pairing, and a loop
//! category, so the load refuses them too.

use bevy::{audio::AudioSource, prelude::*};
use nova_events::prelude::*;
use nova_gameplay::prelude::*;
use nova_sound::prelude::*;

use crate::prelude::*;

/// Where an authored cue plays.
#[derive(Clone, Debug, PartialEq, Eq, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum SoundAtConfig {
    /// Interface feedback, heard flat on the interface track. Takes a
    /// [`SoundBus::Ui`] category.
    Ui,
    /// In the world at this body: a scenario object id, or `$id` /
    /// `$other_id` bound from the firing event. Takes a [`SoundBus::Effect`]
    /// category. Refused with `unknown_body` when no live body wears the id.
    #[reflect(@Names::Object)]
    Body(String),
}

/// Play one authored sound. RON:
/// `PlaySound((sound: "self://sounds/alarm.wav", category: Alarm, at: Ui))`.
#[derive(Clone, Debug, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PlaySoundActionConfig {
    /// The clip, as any other content sound ref.
    pub sound: AssetRef<AudioSource>,
    /// What kind of sound it is. Its row in the sound table sets the level,
    /// the reach and the track.
    pub category: SoundCategoryType,
    /// Where it plays. Must match the category's track.
    pub at: SoundAtConfig,
}

/// Why an authored `PlaySound` cannot play, found without a world.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SoundActionFault {
    /// A loop category in a one-shot action.
    LoopCategory,
    /// A [`SoundBus::Ui`] category at a body, or a [`SoundBus::Effect`]
    /// category at [`SoundAtConfig::Ui`].
    BusMismatch,
}

impl PlaySoundActionConfig {
    /// Whether the category can play where the action puts it. The lint
    /// reads this, and the load refuses what the lint refuses.
    pub fn fault(&self) -> Option<SoundActionFault> {
        if self.category.is_loop() {
            return Some(SoundActionFault::LoopCategory);
        }
        match (self.category.bus(), &self.at) {
            (SoundBus::Ui, SoundAtConfig::Ui) | (SoundBus::Effect, SoundAtConfig::Body(_)) => None,
            (SoundBus::Ui, SoundAtConfig::Body(_)) | (SoundBus::Effect, SoundAtConfig::Ui) => {
                Some(SoundActionFault::BusMismatch)
            }
        }
    }
}

/// A `PlaySound` place with its binding read off the firing event.
enum BoundPlace {
    Ui,
    Body {
        reference: String,
        bound: Option<String>,
    },
}

impl EventAction<NovaEventWorld> for PlaySoundActionConfig {
    fn action(&self, world: &mut NovaEventWorld, info: &GameEventInfo) {
        let sound = self.sound.clone();
        let category = self.category;
        let place = match &self.at {
            SoundAtConfig::Ui => BoundPlace::Ui,
            SoundAtConfig::Body(reference) => BoundPlace::Body {
                reference: reference.clone(),
                bound: bind_object_reference(reference, info),
            },
        };
        debug!("PlaySound: {:?} as {category:?}", sound.path());

        world.push_command(move |commands| {
            commands.queue(move |world: &mut World| play_sound(world, &sound, category, place));
        });
    }
}

/// Resolve the place and trigger the engine's [`PlaySound`], or refuse a body
/// no live entity wears.
fn play_sound(
    world: &mut World,
    sound: &AssetRef<AudioSource>,
    category: SoundCategoryType,
    place: BoundPlace,
) {
    const ACTION: &str = "PlaySound";

    let at = match place {
        BoundPlace::Ui => SoundAt::Ui,
        BoundPlace::Body { reference, bound } => {
            let Some(id) = bound else {
                refuse_action(
                    world,
                    ACTION,
                    &reference,
                    ActionRefusalType::UnknownBody,
                    "the firing event carries no such field",
                );
                return;
            };
            let body = match id.parse::<GeneratedBodyId>() {
                Ok(generated) => generated_entity(world, &generated),
                Err(_) => scoped_entity(world, &id),
            };
            let Some(body) = body else {
                refuse_action(
                    world,
                    ACTION,
                    &id,
                    ActionRefusalType::UnknownBody,
                    "no live body wears the id",
                );
                return;
            };
            SoundAt::Entity(body)
        }
    };
    let handle = sound.resolve(world.resource::<AssetServer>());
    world.trigger(PlaySound {
        sound: handle,
        category,
        at,
    });
}

#[cfg(test)]
mod tests {
    use bevy::asset::AssetPlugin;
    use nova_events::prelude::EventWorld;

    use super::*;

    /// Every cue the app heard.
    #[derive(Resource, Default)]
    struct Heard(Vec<PlaySound>);

    fn record(play: On<PlaySound>, mut heard: ResMut<Heard>) {
        heard.0.push(play.event().clone());
    }

    /// A headless app that can resolve an `AssetRef` and observe the cue,
    /// without an output device: the action's whole job is turning an authored
    /// row into one `PlaySound`, and the mixer is nova_sound's to test.
    fn audio_app() -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.init_asset::<AudioSource>();
        app.init_resource::<NovaEventWorld>();
        app.init_resource::<GameObjectives>();
        app.init_resource::<Heard>();
        app.add_observer(record);
        app
    }

    fn fire(app: &mut App, action: PlaySoundActionConfig, info: &GameEventInfo) {
        let mut world = app.world_mut().resource_mut::<NovaEventWorld>();
        action.action(&mut world, info);
        NovaEventWorld::state_to_world_system(app.world_mut());
        app.update();
    }

    fn cue(category: SoundCategoryType, at: SoundAtConfig) -> PlaySoundActionConfig {
        PlaySoundActionConfig {
            sound: AssetRef::from("sounds/alarm.ogg"),
            category,
            at,
        }
    }

    /// A Ui cue plays flat, as the authored category.
    #[test]
    fn a_ui_cue_plays_flat_as_its_category() {
        let mut app = audio_app();
        fire(
            &mut app,
            cue(SoundCategoryType::Alarm, SoundAtConfig::Ui),
            &GameEventInfo::default(),
        );

        let heard = &app.world().resource::<Heard>().0;
        assert_eq!(heard.len(), 1, "one row, one cue");
        assert_eq!(heard[0].category, SoundCategoryType::Alarm);
        assert_eq!(heard[0].at, SoundAt::Ui);
    }

    /// A body cue plays in the world at the body the firing event binds.
    #[test]
    fn a_body_cue_plays_at_the_bound_body() {
        let mut app = audio_app();
        let freighter = app
            .world_mut()
            .spawn((
                ScenarioScopedMarker,
                ScenarioAddressableMarker,
                EntityId::new("freighter".to_string()),
            ))
            .id();
        let info = GameEventInfo::from_data(DockingEventInfo {
            id: "freighter".to_string(),
            other_id: "player".to_string(),
            ..default()
        });
        fire(
            &mut app,
            cue(
                SoundCategoryType::Machinery,
                SoundAtConfig::Body("$id".to_string()),
            ),
            &info,
        );

        let heard = &app.world().resource::<Heard>().0;
        assert_eq!(heard.len(), 1);
        assert_eq!(heard[0].category, SoundCategoryType::Machinery);
        assert_eq!(heard[0].at, SoundAt::Entity(freighter));
    }

    /// A body no live entity wears plays nothing.
    #[test]
    fn a_body_cue_at_an_unknown_body_plays_nothing() {
        let mut app = audio_app();
        fire(
            &mut app,
            cue(
                SoundCategoryType::Machinery,
                SoundAtConfig::Body("nobody".to_string()),
            ),
            &GameEventInfo::default(),
        );

        assert!(app.world().resource::<Heard>().0.is_empty());
    }

    /// The category decides where a cue may play: a Ui category only at
    /// `Ui`, an Effect category only at a body, and never a loop.
    #[test]
    fn the_category_must_match_where_the_cue_plays() {
        let body = || SoundAtConfig::Body("freighter".to_string());
        assert_eq!(
            cue(SoundCategoryType::Alarm, SoundAtConfig::Ui).fault(),
            None
        );
        assert_eq!(cue(SoundCategoryType::Machinery, body()).fault(), None);
        assert_eq!(
            cue(SoundCategoryType::Alarm, body()).fault(),
            Some(SoundActionFault::BusMismatch)
        );
        assert_eq!(
            cue(SoundCategoryType::Explosion, SoundAtConfig::Ui).fault(),
            Some(SoundActionFault::BusMismatch)
        );
        assert_eq!(
            cue(SoundCategoryType::Drive, body()).fault(),
            Some(SoundActionFault::LoopCategory)
        );
    }

    #[cfg(feature = "serde")]
    #[test]
    fn a_cue_round_trips_through_authored_ron() {
        let authored = r#"PlaySound((sound: "self://sounds/alarm.ogg", category: Explosion, at: Body("$id")))"#;
        let parsed: EventActionConfig = ron::from_str(authored).expect("the cue parses");
        let EventActionConfig::PlaySound(config) = &parsed else {
            panic!("PlaySound variant");
        };
        assert_eq!(config.category, SoundCategoryType::Explosion);
        assert_eq!(config.at, SoundAtConfig::Body("$id".to_string()));

        let flat = r#"PlaySound((sound: "self://sounds/alarm.ogg", category: Alarm, at: Ui))"#;
        let parsed: EventActionConfig = ron::from_str(flat).expect("a Ui cue parses");
        let EventActionConfig::PlaySound(config) = &parsed else {
            panic!("PlaySound variant");
        };
        assert_eq!(config.at, SoundAtConfig::Ui);
    }
}
