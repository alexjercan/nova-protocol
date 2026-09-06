//! `Cinematic`: a beat chain the player is allowed to walk out of.
//!
//! Mechanically a [`SequenceActionConfig`] with a different contract. A
//! sequence is scenario logic and always runs to its end; a cinematic is a
//! SCENE, it takes the camera and the controls away, and a player who has seen
//! it once must be able to get them back. Everything else - the cursor, the
//! step gates, the deadlines - is the sequence machinery, reused rather than
//! copied.
//!
//! The scene's two ends are EVENTS, not action lists on the action:
//!
//! - [`OnCinematicFinishedEvent`] fires on every path out of the scene - the
//!   last beat, a skip, a `CancelCinematic`, a blown deadline. The post-state
//!   the scene owes the player (camera released, control returned, the
//!   objective it was hiding) is authored there, once.
//! - [`OnCinematicSkippedEvent`] fires first on a skip, and carries only what
//!   the unplayed beats would have left behind: the actors spawned, moved or
//!   destroyed, the variables latched.
//!
//! A skip cancels the CURSOR. It never runs the remaining steps at speed,
//! because a step chain is not a list of world edits - it holds camera moves,
//! comms lines and shots the player has just said they do not want.

use std::sync::Arc;

use bevy::prelude::*;
use nova_events::prelude::*;
use nova_hud::prelude::ScreenCorner;
use nova_input::prelude::*;

use crate::prelude::*;

/// How long a scene refuses the skip binding after it starts.
///
/// The player is very often still holding the button that got them here - the
/// interaction that triggered the scene, the last press of the beat before it.
/// Without the hold-off the scene ends on the frame it opens and reads as a
/// bug rather than a skip.
pub const CINEMATIC_SKIP_ARM_SECONDS: f64 = 0.75;

/// Play an ordered beat chain as a scene, under an authored key.
///
/// The steps are [`SequenceStepConfig`]s and mean exactly what they mean in a
/// `Sequence`. What the wrapper adds is the ending: the scene reports one, on
/// every path out.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CinematicActionConfig {
    /// Scenario-local key the cursor is filed under, and the key both
    /// cinematic events report. A literal, so the lint can pair a scene with
    /// the handlers that end it.
    pub key: String,
    /// Whether the player may leave this scene early. Authored on every
    /// cinematic: a scene that cannot be skipped is a real decision (a scene
    /// three seconds long, a scene the first time only) and is not the kind of
    /// thing that should happen because a field was left out.
    pub skippable: bool,
    /// The beats, in order.
    pub steps: Vec<SequenceStepConfig>,
}

impl EventAction<NovaEventWorld> for CinematicActionConfig {
    fn action(&self, world: &mut NovaEventWorld, _: &GameEventInfo) {
        if self.key.trim().is_empty() {
            error!("CinematicActionConfig: cinematic key must not be empty");
            return;
        }
        if self.steps.is_empty() {
            error!(
                "CinematicActionConfig: cinematic '{}' has no steps",
                self.key
            );
            return;
        }
        world.start_cinematic(
            self.key.clone(),
            Arc::new(self.steps.clone()),
            self.skippable,
        );
    }
}

/// End a running cinematic now, from the scenario rather than from the player.
///
/// The scene still reports [`OnCinematicFinishedEvent`], so whatever the scene
/// took is given back. It does NOT report a skip: nobody asked to leave.
#[derive(Clone, Debug, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CancelCinematicActionConfig {
    /// The scene to end.
    #[reflect(@Names::Cinematic)]
    pub key: String,
}

impl EventAction<NovaEventWorld> for CancelCinematicActionConfig {
    fn action(&self, world: &mut NovaEventWorld, _: &GameEventInfo) {
        world.cancel_cinematic(&self.key);
    }
}

/// Which corner of the screen a title card sits in.
///
/// Mirrors nova_hud's [`ScreenCorner`], the same split
/// `HudReadoutFormatConfig` makes with `HudReadoutFormat`: the HUD cannot
/// depend on this crate. The `Config` suffix is what keeps the two halves
/// distinguishable when nova_core globs both preludes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ScreenCornerConfig {
    /// Under the status bar. The corner most often free.
    #[default]
    TopLeft,
    /// Opposite the status bar's version stamp.
    TopRight,
    /// Where the comms stack lives - free only in a scene with no dialogue.
    BottomLeft,
    /// Free unless a scene has put something there.
    BottomRight,
}

impl From<ScreenCornerConfig> for ScreenCorner {
    fn from(value: ScreenCornerConfig) -> Self {
        match value {
            ScreenCornerConfig::TopLeft => ScreenCorner::TopLeft,
            ScreenCornerConfig::TopRight => ScreenCorner::TopRight,
            ScreenCornerConfig::BottomLeft => ScreenCorner::BottomLeft,
            ScreenCornerConfig::BottomRight => ScreenCorner::BottomRight,
        }
    }
}

/// Post the shot's title card: where this is, when it is, and one line that
/// makes it matter.
///
/// The card fades in, holds for `seconds` (fades included), and goes. Nothing
/// takes it down, which is the point: a scene that is skipped, cancelled,
/// deadlined or torn down leaves a card that finishes its own hold, so the
/// card needs no cleanup on any handler.
///
/// Posting a second card REPLACES the first, at its own age. Two cards on
/// screen at once would be two answers to "where am I".
///
/// RON: `CinematicTitle((corner: TopLeft, location: "Meridian", date: "2481.114",
/// note: "412 aboard.", seconds: 8.0))`.
///
/// The corner is authored because only the shot knows which corner is free:
/// `BottomLeft` is the comms stack's, and the top strip carries the status bar.
/// An empty `note` draws no third line.
#[derive(Clone, Debug, PartialEq, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CinematicTitleActionConfig {
    /// Where on screen the card sits.
    pub corner: ScreenCornerConfig,
    /// The place, drawn largest.
    pub location: String,
    /// The stamp under it - a date, a time, a bearing, whatever the shot needs.
    pub date: String,
    /// One line of context. Empty draws no line.
    pub note: String,
    /// How long the card holds, fades included.
    pub seconds: f32,
}

impl EventAction<NovaEventWorld> for CinematicTitleActionConfig {
    fn action(&self, world: &mut NovaEventWorld, _: &GameEventInfo) {
        debug!(
            "CinematicTitle: '{}' in {:?} for {}s",
            self.location, self.corner, self.seconds
        );
        world.post_cinematic_title(self.clone());
    }
}

/// The action name the skip is bound to.
///
/// FOLLOWS `scenario_advance`: advancing past a beat and leaving a scene are
/// one gesture read two ways, the same pair `radar_clear` makes with
/// `radar_hold`. As a follower it shares the key on purpose, is never a
/// conflict, and moves with Advance when a player rebinds it.
pub const CINEMATIC_SKIP_ACTION: &str = "cinematic_skip";

/// Whether there is an input backend to poll at all.
///
/// A scenario also runs in apps that have no player: the capture scripts, the
/// headless test rigs, the content tools. Those have no `ButtonInput` and no
/// binding registry, and a system that read them would fail parameter
/// validation and take the whole scenario clock down with it. Nothing is lost
/// by not offering a skip to nobody.
pub fn a_player_can_answer(
    keys: Option<Res<ButtonInput<KeyCode>>>,
    mouse: Option<Res<ButtonInput<MouseButton>>>,
    bindings: Option<Res<InputBindings>>,
) -> bool {
    keys.is_some() && mouse.is_some() && bindings.is_some()
}

/// Let the player leave the live skippable scene.
///
/// Polls the registry rather than riding a `bevy_enhanced_input` rig: the rig
/// spawns with the player ship, and a scene that has taken the ship's controls
/// away is exactly when the skip has to still answer.
pub fn skip_cinematic_on_request(
    sources: InputSources,
    bindings: Res<InputBindings>,
    mut world: ResMut<NovaEventWorld>,
) {
    let Some(action) = bindings.get(CINEMATIC_SKIP_ACTION) else {
        return;
    };
    if sources.just_pressed(action) {
        world.skip_cinematic();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A beat that says it ran, so a test can tell a played scene from a
    /// skipped one.
    fn beat(ordinal: f64) -> EventActionConfig {
        EventActionConfig::VariableSet(VariableSetActionConfig {
            key: "at".to_string(),
            expression: VariableExpressionNode::new_term(VariableTermNode::new_factor(
                VariableFactorNode::new_literal(VariableLiteral::Number(ordinal)),
            )),
        })
    }

    fn timed(after: f64, ordinal: f64) -> SequenceStepConfig {
        SequenceStepConfig {
            after: Some(after),
            actions: vec![beat(ordinal)],
            ..default()
        }
    }

    fn start(
        world: &mut NovaEventWorld,
        key: &str,
        skippable: bool,
        steps: Vec<SequenceStepConfig>,
    ) {
        CinematicActionConfig {
            key: key.to_string(),
            skippable,
            steps,
        }
        .action(world, &default());
    }

    /// Run whatever the cursor is ready to deliver at `now`, as the driver does.
    fn pump(world: &mut NovaEventWorld, now: f64) {
        advance(world, now);
        while let Some(actions) = world.take_ready_sequence_step(now) {
            for action in &actions {
                action.action(world, &default());
            }
        }
    }

    fn at(world: &NovaEventWorld) -> Option<f64> {
        match world.get_variable("at") {
            Some(VariableLiteral::Number(n)) => Some(*n),
            _ => None,
        }
    }

    /// A card that holds for `seconds`.
    fn card(seconds: f32) -> CinematicTitleActionConfig {
        CinematicTitleActionConfig {
            corner: ScreenCornerConfig::TopLeft,
            location: "MERIDIAN, OUTER HOLD".to_string(),
            date: "END OF SHIFT".to_string(),
            note: "Unarmed, and due under way.".to_string(),
            seconds,
        }
    }

    /// Move the scenario clock, which is what the hold-off is measured on.
    fn advance(world: &mut NovaEventWorld, to: f64) {
        let delta = to - world.scenario_elapsed();
        world.advance_scenario_elapsed(delta);
    }

    /// A card holds for its authored seconds on the SCENARIO clock and then
    /// goes, with no handler taking it down. That is what lets a scene end any
    /// way it likes without stranding a card on screen.
    #[test]
    fn a_title_card_expires_on_its_own() {
        let mut world = NovaEventWorld::default();
        world.post_cinematic_title(card(4.0));
        let (config, age) = world.cinematic_title().expect("the card is up");
        assert_eq!(config.location, "MERIDIAN, OUTER HOLD");
        assert!((age - 0.0).abs() < f32::EPSILON, "posted this instant");

        advance(&mut world, 3.9);
        let (_, age) = world.cinematic_title().expect("still inside its hold");
        assert!((age - 3.9).abs() < 1e-4, "the age is scenario time: {age}");

        advance(&mut world, 4.0);
        assert!(world.cinematic_title().is_none(), "the hold ran out");
        advance(&mut world, 100.0);
        assert!(world.cinematic_title().is_none(), "and it stays down");
    }

    /// Two cards are two answers to "where am I". The second replaces the
    /// first, and starts its own hold rather than inheriting the age.
    #[test]
    fn a_second_card_replaces_the_first() {
        let mut world = NovaEventWorld::default();
        world.post_cinematic_title(card(4.0));
        advance(&mut world, 3.0);

        let mut second = card(4.0);
        second.location = "PLATE SEVEN".to_string();
        world.post_cinematic_title(second);

        let (config, age) = world.cinematic_title().expect("the second card is up");
        assert_eq!(config.location, "PLATE SEVEN");
        assert!((age - 0.0).abs() < f32::EPSILON, "the hold restarted");

        advance(&mut world, 6.0);
        assert!(
            world.cinematic_title().is_some(),
            "the replacement outlives the card it replaced"
        );
    }

    /// The card is scenario state like everything else: it cannot survive into
    /// the next scenario or the menu.
    #[test]
    fn teardown_takes_the_card_with_it() {
        let mut world = NovaEventWorld::default();
        world.post_cinematic_title(card(30.0));
        world.clear();
        assert!(world.cinematic_title().is_none());
    }

    /// The patient path: the scene plays out and reports ONE finish, not a
    /// skip.
    #[test]
    fn a_scene_that_runs_out_reports_a_finish() {
        let mut world = NovaEventWorld::default();
        start(
            &mut world,
            "strike",
            true,
            vec![timed(0.0, 1.0), timed(1.0, 2.0)],
        );
        pump(&mut world, 0.0);
        assert!(
            world.drain_cinematic_endings().is_empty(),
            "a scene mid-run owes nothing yet"
        );

        pump(&mut world, 1.0);
        assert_eq!(at(&world), Some(2.0), "every beat played");
        let endings = world.drain_cinematic_endings();
        assert_eq!(endings.len(), 1);
        assert_eq!(endings[0].key, "strike");
        assert!(!endings[0].skipped, "nobody asked to leave");
        assert!(
            world.drain_cinematic_endings().is_empty(),
            "the ending is reported once"
        );
    }

    /// The whole point of the wrapper: leaving early CANCELS the cursor, and
    /// reports the skip before the finish the restore hangs on.
    #[test]
    fn a_skip_stops_the_cursor_and_reports_both_endings() {
        let mut world = NovaEventWorld::default();
        start(
            &mut world,
            "strike",
            true,
            vec![timed(0.0, 1.0), timed(30.0, 2.0)],
        );
        pump(&mut world, 0.0);

        advance(&mut world, CINEMATIC_SKIP_ARM_SECONDS);
        assert!(world.skip_cinematic(), "the scene took the skip");

        let endings = world.drain_cinematic_endings();
        assert_eq!(endings.len(), 1);
        assert!(endings[0].skipped);

        pump(&mut world, 60.0);
        assert_eq!(
            at(&world),
            Some(1.0),
            "the unplayed beats must NOT run at speed on the way out"
        );
    }

    /// The hold-off: the player is usually still holding the key that got them
    /// here, and a scene that ends on its first frame reads as a bug.
    #[test]
    fn a_scene_refuses_the_skip_until_it_has_been_up_a_moment() {
        let mut world = NovaEventWorld::default();
        start(&mut world, "strike", true, vec![timed(30.0, 1.0)]);

        assert!(!world.skip_cinematic(), "too soon");
        assert!(world.skippable_cinematic().is_none(), "and no prompt yet");

        advance(&mut world, CINEMATIC_SKIP_ARM_SECONDS);
        assert_eq!(world.skippable_cinematic(), Some("strike"));
        assert!(world.skip_cinematic());
    }

    /// A scene authored unskippable is not offered and does not answer.
    #[test]
    fn an_unskippable_scene_neither_prompts_nor_skips() {
        let mut world = NovaEventWorld::default();
        start(&mut world, "strike", false, vec![timed(30.0, 1.0)]);
        advance(&mut world, 10.0);

        assert!(world.skippable_cinematic().is_none());
        assert!(!world.skip_cinematic());
        assert!(world.drain_cinematic_endings().is_empty());
    }

    /// `CancelCinematic` ends the scene from the scenario. It reports a
    /// finish, because nobody asked to leave - the catch-up handler must not
    /// run for a scene the author ended on purpose.
    #[test]
    fn a_cancelled_scene_reports_a_finish_and_not_a_skip() {
        let mut world = NovaEventWorld::default();
        start(&mut world, "strike", true, vec![timed(30.0, 1.0)]);

        CancelCinematicActionConfig {
            key: "strike".to_string(),
        }
        .action(&mut world, &default());

        let endings = world.drain_cinematic_endings();
        assert_eq!(endings.len(), 1);
        assert!(!endings[0].skipped);

        pump(&mut world, 60.0);
        assert_eq!(at(&world), None, "the cancelled beats never ran");
    }

    /// A gate that never opens is the author's bug; the camera and the
    /// controls the scene took are the player's. The deadline still reports.
    #[test]
    fn a_scene_stopped_by_a_deadline_still_reports_its_ending() {
        let mut world = NovaEventWorld::default();
        start(
            &mut world,
            "strike",
            true,
            vec![SequenceStepConfig {
                until: Some(SequenceGateConfig {
                    name: EventConfig::OnEnter,
                    filters: vec![],
                }),
                deadline: Some(5.0),
                actions: vec![beat(1.0)],
                ..default()
            }],
        );

        pump(&mut world, 6.0);
        let endings = world.drain_cinematic_endings();
        assert_eq!(endings.len(), 1, "a stuck scene hands the camera back");
        assert!(!endings[0].skipped);
    }

    /// Teardown drops both the cursor and the ending it owed: a scene cannot
    /// announce itself into the next scenario.
    #[test]
    fn teardown_drops_a_scene_and_its_pending_ending() {
        let mut world = NovaEventWorld::default();
        start(&mut world, "strike", true, vec![timed(30.0, 1.0)]);
        advance(&mut world, 10.0);
        world.skip_cinematic();

        world.clear();
        assert!(world.drain_cinematic_endings().is_empty());
        assert!(world.skippable_cinematic().is_none());
    }

    /// Two scenes, one skip: the player leaves the one that is asking, and the
    /// other keeps its cursor.
    #[test]
    fn a_skip_leaves_only_the_scene_offering_it() {
        let mut world = NovaEventWorld::default();
        start(&mut world, "quiet", false, vec![timed(30.0, 1.0)]);
        start(&mut world, "loud", true, vec![timed(30.0, 2.0)]);
        advance(&mut world, 10.0);

        assert_eq!(world.skippable_cinematic(), Some("loud"));
        assert!(world.skip_cinematic());
        let endings = world.drain_cinematic_endings();
        assert_eq!(endings.len(), 1);
        assert_eq!(endings[0].key, "loud");
        assert_eq!(world.sequence_step("quiet"), Some(0), "the other plays on");
    }

    /// An empty chain is refused rather than started as a scene that ends on
    /// its own first frame with the camera already taken.
    #[test]
    fn a_scene_with_no_beats_is_refused() {
        let mut world = NovaEventWorld::default();
        start(&mut world, "strike", true, vec![]);
        assert!(world.skippable_cinematic().is_none());
        assert!(world.drain_cinematic_endings().is_empty());
    }

    #[cfg(feature = "serde")]
    #[test]
    fn a_cinematic_round_trips_through_authored_ron() {
        let authored = r#"Cinematic((key: "strike", skippable: true, steps: [(after: Some(1.0), actions: [DebugMessage((message: "boom"))])]))"#;
        let parsed: EventActionConfig = ron::from_str(authored).expect("parse the scene");
        let EventActionConfig::Cinematic(config) = &parsed else {
            panic!("parsed the Cinematic variant");
        };
        assert_eq!(config.key, "strike");
        assert!(config.skippable);
        assert_eq!(config.steps.len(), 1);

        let ron = ron::to_string(&parsed).expect("serialize the scene");
        let back: EventActionConfig = ron::from_str(&ron).expect("re-parse the scene");
        assert!(matches!(back, EventActionConfig::Cinematic(_)));
    }

    /// The skip answers the KEY, read through the registry every frame. This
    /// is the affordance the prompt is promising, and it has to work while the
    /// scene holds the ship's controls - which is why it polls rather than
    /// riding the player's input rig.
    #[test]
    fn pressing_the_bound_key_leaves_the_scene() {
        use bevy::input::ButtonInput;
        use nova_input::prelude::{ActionBinding, InputSource};

        let mut app = App::new();
        app.init_resource::<ButtonInput<KeyCode>>();
        app.init_resource::<ButtonInput<MouseButton>>();
        app.init_resource::<NovaEventWorld>();
        app.insert_resource(InputBindings::from_actions([ActionBinding::new(
            CINEMATIC_SKIP_ACTION,
            "SCENARIO",
            "Skip Scene",
        )
        .keyboard([InputSource::Keyboard(KeyCode::Enter)])]));
        app.add_systems(Update, skip_cinematic_on_request);

        {
            let mut world = app.world_mut().resource_mut::<NovaEventWorld>();
            start(&mut world, "strike", true, vec![timed(30.0, 1.0)]);
            world.advance_scenario_elapsed(CINEMATIC_SKIP_ARM_SECONDS);
        }
        app.update();
        assert!(
            app.world_mut()
                .resource_mut::<NovaEventWorld>()
                .drain_cinematic_endings()
                .is_empty(),
            "nothing was pressed"
        );

        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Enter);
        app.update();

        let endings = app
            .world_mut()
            .resource_mut::<NovaEventWorld>()
            .drain_cinematic_endings();
        assert_eq!(endings.len(), 1);
        assert!(endings[0].skipped);
    }

    /// A scenario also runs where there is no player: the capture scripts and
    /// the headless rigs. The skip is not offered there, and must not take the
    /// scenario clock down with it by reading input that does not exist.
    #[test]
    fn an_app_with_no_input_backend_is_not_asked_for_a_skip() {
        use bevy::{ecs::system::RunSystemOnce, input::ButtonInput};

        let mut app = App::new();
        assert!(
            !app.world_mut()
                .run_system_once(a_player_can_answer)
                .expect("the condition runs"),
            "an app with no input resources cannot be asked"
        );

        app.init_resource::<ButtonInput<KeyCode>>();
        app.init_resource::<ButtonInput<MouseButton>>();
        app.insert_resource(InputBindings::default());
        assert!(app
            .world_mut()
            .run_system_once(a_player_can_answer)
            .expect("the condition runs"));
    }
}
