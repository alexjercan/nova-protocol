//! Production-faithful scenario tests for the NEUTRALIZED (combat-dead) signal
//! over the shipped training range.
//!
//! A ship that was armed and has lost all working weapons OR its flight
//! computer fires `OnNeutralizedEvent` instead of being destroyed. The physical
//! predicate is pinned in `nova_gameplay::integrity::neutralize`; what this
//! file owns is the SCENARIO DATA's consumption of it.
//!
//! The contract is the PLAYER's side of it: losing the helm is a Defeat, it
//! queues the retry, and it is gated below the epilogue so a hull coming apart
//! during an earned win cannot overwrite it. That last guard is the one worth
//! a test: it is invisible in the script and catastrophic when it is missing.
//! The range's drones go quiet under the same signal and must declare
//! nothing.

use bevy::{ecs::system::RunSystemOnce, prelude::*};
use nova_events::prelude::{
    CommandsGameEventExt, GameEventsPlugin, OnDefeatedEvent, OnDefeatedEventInfo,
    OnNeutralizedEvent, OnNeutralizedEventInfo, OnUpdateEvent, OnUpdateEventInfo,
};
use nova_gameplay::prelude::GameObjectives;
use nova_scenario::prelude::*;

const TUTORIAL_RON: &str = include_str!("../../../assets/base/scenarios/tutorial.content.ron");

/// The scenario the range queues when the player loses the helm: itself.
const TUTORIAL_ID: &str = "tutorial";

/// The beat the range's defeat gate sits below: the outro beat in the authored
/// script. Spelled out here rather than imported, because the point of the
/// test is that the RON carries the gate - importing the constant would let a
/// script that dropped the guard still pass.
const TUTORIAL_OUTRO: f64 = 12.0;

/// A beat while the lesson is still running.
const TUTORIAL_LIVE: f64 = 2.0;

/// The id the range's own defeat gates name. Also spelled out rather than
/// imported: a script that renamed the player's hull and left the gates
/// pointing at the old id would still pass against a constant.
const TUTORIAL_PLAYER: &str = "trainer";

fn scenario_from(ron: &str) -> ScenarioConfig {
    let items: Vec<nova_modding::prelude::Content> =
        ron::de::from_str(ron).expect("content parses");
    items
        .into_iter()
        .find_map(|c| match c {
            nova_modding::prelude::Content::Scenario(s) => Some(s),
            _ => None,
        })
        .expect("content contains a Scenario")
}

fn slice_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(GameEventsPlugin::<NovaEventWorld>::default());
    app.init_resource::<NovaEventWorld>();
    app.init_resource::<GameObjectives>();
    app.init_resource::<CurrentOutcome>();
    app.add_systems(Update, |mut commands: Commands| {
        commands.fire::<OnUpdateEvent>(OnUpdateEventInfo);
    });
    app
}

fn seed_var(app: &mut App, key: &str, value: f64) {
    app.world_mut()
        .resource_mut::<NovaEventWorld>()
        .insert_variable(key.to_string(), VariableLiteral::Number(value));
}

/// Register everything but `OnStart`: the rig seeds the variables that handler
/// would have written, and spawning a whole scenario's objects is not what is
/// under test.
fn register_non_start_handlers(app: &mut App, scenario: &ScenarioConfig) {
    for event in scenario
        .events
        .iter()
        .filter(|e| !matches!(e.name, EventConfig::OnStart))
    {
        app.world_mut().spawn(event.build_handler());
        for gate in event.gate_handlers() {
            app.world_mut().spawn(gate);
        }
    }
}

/// Fire the outcome pair for a neutralized ship in production order: the
/// unified defeat first, then the specific neutralization edge.
fn neutralize(app: &mut App, id: &str) {
    let defeated = OnDefeatedEventInfo {
        id: id.to_string(),
        type_name: "spaceship".to_string(),
    };
    app.world_mut()
        .run_system_once(move |mut commands: Commands| {
            commands.fire::<OnDefeatedEvent>(defeated.clone());
            commands.fire::<OnNeutralizedEvent>(OnNeutralizedEventInfo {
                id: defeated.id.clone(),
                type_name: defeated.type_name.clone(),
            });
        })
        .expect("fire neutralized outcome pair");
    app.update();
    app.update();
}

fn outcome_kind(app: &App) -> Option<ScenarioOutcomeKind> {
    app.world()
        .resource::<CurrentOutcome>()
        .0
        .as_ref()
        .map(|outcome| outcome.outcome)
}

fn queued_scenario(app: &App) -> Option<String> {
    app.world()
        .resource::<NovaEventWorld>()
        .next_scenario
        .as_ref()
        .map(|next| next.scenario_id.clone())
}

/// A live beat, the epilogue beat, and what a neutralize means in each.
#[test]
fn a_tutorial_player_neutralize_is_a_gated_terminal_defeat() {
    let (id, ship) = (TUTORIAL_ID, TUTORIAL_PLAYER);
    let scenario = scenario_from(TUTORIAL_RON);

    // On a live beat: an immediate Defeat with the run queued for retry.
    let mut app = slice_app();
    register_non_start_handlers(&mut app, &scenario);
    seed_var(&mut app, "beat", TUTORIAL_LIVE);
    app.update();
    assert_eq!(
        outcome_kind(&app),
        None,
        "{id}: nothing declares on its own"
    );

    neutralize(&mut app, ship);
    assert_eq!(
        outcome_kind(&app),
        Some(ScenarioOutcomeKind::Defeat),
        "{id}: losing the helm is a player Defeat"
    );
    assert_eq!(
        queued_scenario(&app).as_deref(),
        Some(id),
        "{id}: the retry is the run itself, offered rather than forced"
    );

    // On the epilogue beat: the win is already locked, and the same event
    // declares nothing. Without the gate this would overwrite an earned
    // Victory with a Defeat while the banner was on screen.
    let mut app = slice_app();
    register_non_start_handlers(&mut app, &scenario);
    seed_var(&mut app, "beat", TUTORIAL_OUTRO);
    neutralize(&mut app, ship);
    assert_eq!(
        outcome_kind(&app),
        None,
        "{id}: a neutralize during the epilogue must not overwrite the win"
    );
}

/// A drone going quiet is the gunnery lesson being PASSED, not a loss
/// condition. Only the player's id is wired to an outcome: a scenario that read
/// a disarmed drone as the player's defeat would end the run on the beat it
/// exists for.
#[test]
fn nothing_but_the_player_can_end_the_range() {
    let scenario = scenario_from(TUTORIAL_RON);
    let mut app = slice_app();
    register_non_start_handlers(&mut app, &scenario);
    seed_var(&mut app, "beat", 11.0);

    for bystander in ["drone_1", "drone_2"] {
        neutralize(&mut app, bystander);
        assert_eq!(
            outcome_kind(&app),
            None,
            "'{bystander}' going quiet must not declare an outcome"
        );
    }
}
