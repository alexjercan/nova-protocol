//! What the Command shell says about the world it opened over.
//!
//! One line, read from live state rather than passed in: the shell can be
//! opened from the main menu, the editor sandbox, a running scenario or a
//! pause surface, and each has to describe itself the same way whether the
//! introduction is being revealed or `status` is being printed.

use bevy::prelude::*;
use nova_gameplay::prelude::*;
use nova_scenario::prelude::CurrentScenario;

/// The introduction's `WORLD` row: `<what> / <whether it is running>`.
///
/// The scenario id is printed as authored, so the editor sandbox reads
/// `editor_sandbox` rather than a friendlier name this crate would have to
/// know about `nova_editor` to produce.
pub fn world_line(world: &World) -> String {
    let state = world.get_resource::<State<GameStates>>().map(State::get);
    let scenario = world
        .get_resource::<CurrentScenario>()
        .and_then(|current| current.0.as_ref().map(|config| config.id.clone()));
    let motion = motion_word(world);

    match (state, scenario) {
        (Some(GameStates::Loading), _) => "loading / idle".to_string(),
        (Some(GameStates::MainMenu), _) | (None, None) => "main menu / idle".to_string(),
        (_, Some(id)) => format!("{id} / {motion}"),
        (_, None) => "no scenario / idle".to_string(),
    }
}

/// Whether the world is advancing.
///
/// Two sources, because they disagree for exactly one frame: the freeze is the
/// truth once it is applied, and the pause STATE is what the frame that opened
/// the shell already decided. Reading only the freeze made the introduction
/// announce `running` on the very frame the CRT paused the world.
fn motion_word(world: &World) -> &'static str {
    let held = world
        .get_resource::<ClockFreeze>()
        .is_some_and(ClockFreeze::is_held);
    let pausing = world
        .get_resource::<State<PauseStates>>()
        .is_some_and(|state| *state.get() != PauseStates::Unpaused)
        || world.get_resource::<NextState<PauseStates>>().is_some_and(
            |next| matches!(next, NextState::Pending(state) if *state != PauseStates::Unpaused),
        );
    if held || pausing {
        "paused"
    } else {
        "running"
    }
}

#[cfg(test)]
mod tests {
    use bevy::ecs::system::RunSystemOnce;

    use super::*;

    /// A world with the game state and the freeze the console reads.
    fn world(state: GameStates, held: bool) -> World {
        let mut world = World::new();
        world.insert_resource(State::new(state));
        world.init_resource::<ClockFreeze>();
        world.init_resource::<Time<Virtual>>();
        world.init_resource::<CurrentScenario>();
        if held {
            world
                .run_system_once(|mut clocks: Clocks| clocks.hold(FreezeOwner::Terminal))
                .unwrap();
        }
        world
    }

    /// The frame that opens the CRT has decided to pause but has not applied
    /// the freeze yet. The introduction is staged on exactly that frame.
    #[test]
    fn a_pause_that_is_only_queued_still_reads_as_paused() {
        let mut world = world(GameStates::Playing, false);
        world.resource_mut::<CurrentScenario>().0 = Some(scenario("shakedown_run"));
        assert_eq!(world_line(&world), "shakedown_run / running");

        world.insert_resource(NextState::Pending(PauseStates::NovaOs));
        assert_eq!(world_line(&world), "shakedown_run / paused");
    }

    #[test]
    fn the_world_row_names_the_surface_and_whether_it_runs() {
        assert_eq!(
            world_line(&world(GameStates::MainMenu, true)),
            "main menu / idle"
        );
        assert_eq!(
            world_line(&world(GameStates::Loading, false)),
            "loading / idle"
        );
        // Playing with nothing loaded is a real state: the sandbox before a
        // scenario is chosen.
        assert_eq!(
            world_line(&world(GameStates::Playing, false)),
            "no scenario / idle"
        );
    }

    #[test]
    fn a_loaded_scenario_is_named_and_says_whether_it_is_frozen() {
        let mut frozen = world(GameStates::Playing, true);
        frozen.resource_mut::<CurrentScenario>().0 = Some(scenario("shakedown_run"));
        assert_eq!(world_line(&frozen), "shakedown_run / paused");

        let mut running = world(GameStates::Playing, false);
        running.resource_mut::<CurrentScenario>().0 = Some(scenario("editor_sandbox"));
        assert_eq!(world_line(&running), "editor_sandbox / running");
    }

    fn scenario(id: &str) -> nova_scenario::prelude::ScenarioConfig {
        nova_scenario::prelude::ScenarioConfig::new(
            id.to_string(),
            id.to_string(),
            Handle::default().into(),
        )
    }
}
