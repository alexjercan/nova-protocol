//! 19_broadside: the chapter-two slice, wired to the smoke-test harness.
//!
//! Boots the exact app the binary runs, drives the Scenarios picker to launch
//! Broadside, and plays the whole win/lose frame in ONE run: dies to prove
//! the Defeat overlay + Retry (the scenario reloads clean), then wins the
//! reloaded instance - teleports to the hauler to spring the ambush, kills
//! the corvettes to force the gunship twist, breaks the gunship - and
//! asserts the Victory overlay with nothing queued. Every act transition is
//! staged on scenario STATE (act/outcome/entities), never wall-clock
//! (event-driven-autopilot-beats); wall-clock lives only in the per-stage
//! stall deadline and the autopilot's overall lifetime.
//!
//! The kill stimulus is `HealthApplyDamage` overkill on the ship ROOT - the
//! production damage entry point (propagation, integrity explode, and the
//! OnDestroyed bridge all run for real; only the bullets are skipped, and
//! those are pinned by the weapon-range examples).
//!
//! A completion guard turns the silent-timeout hole into a failure: if the
//! autopilot's lifetime expires before the script's final stage, the run
//! panics naming the stage it stalled in, instead of logging "cycle
//! complete" over an unfinished walk.
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//! ```text
//! BCS_AUTOPILOT=1 cargo run --example 19_broadside --features debug
//! # look for: `probe: defeat overlay up, retrying`,
//! #           `probe: victory overlay up`,
//! #           `probe: script complete, exiting` (the self-ending sentinel
//! #           the smoke suite accepts in place of the autopilot's line)
//! ```

#[cfg(feature = "debug")]
use bevy::prelude::*;
use clap::Parser;
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "19_broadside")]
#[command(version = "1.0.0")]
#[command(about = "The Broadside slice: defeat, retry, victory - under the smoke harness", long_about = None)]
struct Cli;

fn main() {
    let _ = Cli::parse();

    let mut app = editor_app(true);

    #[cfg(feature = "debug")]
    {
        if std::env::var_os("BCS_AUTOPILOT").is_some() {
            app.insert_resource(bevy::ecs::error::FallbackErrorHandler(
                bevy::ecs::error::panic,
            ));
            // Completion guard: an AppExit with the script unfinished is a
            // stalled walk, not a pass.
            app.add_systems(Last, guard_script_completion);
        }
        app.init_resource::<SliceAutopilot>();
        // The full walk needs two scenario loads plus three staged fights;
        // 40s of runway (the script exits itself in ~10s when healthy).
        app.add_plugins(
            nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
                .hold(GameStates::Loading, 40.0)
                .input(slice_autopilot),
        );
        app.add_plugins(nova_screenshot());
    }

    app.run();
}

/// Stall deadline per stage (s): generous under llvmpipe, far below the
/// autopilot lifetime, so a stuck stage fails with its name instead of a
/// generic timeout.
#[cfg(feature = "debug")]
const STAGE_DEADLINE_SECS: f32 = 12.0;

#[cfg(feature = "debug")]
#[derive(Resource, Default)]
struct SliceAutopilot {
    stage: u32,
    stage_started: f32,
    wait: u32,
    done: bool,
}

#[cfg(feature = "debug")]
fn guard_script_completion(mut exits: MessageReader<AppExit>, script: Option<Res<SliceAutopilot>>) {
    let Some(script) = script else { return };
    if exits.read().next().is_some() && !script.done {
        panic!(
            "19_broadside: run ended with the script stalled in stage {}",
            script.stage
        );
    }
}

#[cfg(feature = "debug")]
fn slice_autopilot(world: &mut World, elapsed: f32) {
    use avian3d::prelude::{LinearVelocity, Position};
    use bevy::ui_widgets::Activate;
    use nova_protocol::nova_scenario::prelude::*;

    let mut state = world.remove_resource::<SliceAutopilot>().unwrap();
    if state.done {
        world.insert_resource(state);
        return;
    }
    if state.wait > 0 {
        state.wait -= 1;
        world.insert_resource(state);
        return;
    }
    if elapsed - state.stage_started > STAGE_DEADLINE_SECS {
        panic!(
            "19_broadside: stage {} stalled for {STAGE_DEADLINE_SECS}s",
            state.stage
        );
    }

    let mut advance = |state: &mut SliceAutopilot, to: u32, note: &str| {
        info!("probe: stage {} -> {to}: {note}", state.stage);
        state.stage = to;
        state.stage_started = elapsed;
        state.wait = 10;
    };

    let entity_by_name = |world: &mut World, name: &str| -> Option<Entity> {
        let mut q = world.query::<(Entity, &Name)>();
        q.iter(world)
            .find(|(_, n)| n.as_str() == name)
            .map(|(e, _)| e)
    };
    let root_by_id = |world: &mut World, id: &str| -> Option<Entity> {
        let mut q = world.query_filtered::<(Entity, &nova_protocol::nova_events::prelude::EntityId), With<SpaceshipRootMarker>>();
        q.iter(world).find(|(_, eid)| ***eid == *id).map(|(e, _)| e)
    };
    let act = |world: &World| -> Option<f64> {
        match world.resource::<NovaEventWorld>().get_variable("act") {
            Some(VariableLiteral::Number(n)) => Some(*n),
            _ => None,
        }
    };
    let outcome = |world: &World| -> Option<ScenarioOutcomeKind> {
        world
            .resource::<CurrentOutcome>()
            .0
            .as_ref()
            .map(|o| o.outcome)
    };
    let kill = |world: &mut World, id: &str| -> bool {
        if let Some(root) = root_by_id(world, id) {
            world.trigger(HealthApplyDamage {
                entity: root,
                source: None,
                amount: 100_000.0,
            });
            true
        } else {
            false
        }
    };
    // Springing the ambush means physically arriving: put the ship at the
    // area's edge with avian's own Position (Transform alone snaps back).
    let teleport_player = |world: &mut World| -> bool {
        let mut q = world.query_filtered::<Entity, With<PlayerSpaceshipMarker>>();
        let Some(player) = q.iter(world).next() else {
            return false;
        };
        let target = Vec3::new(0.0, 10.0, -450.0);
        if let Some(mut pos) = world.get_mut::<Position>(player) {
            pos.0 = target;
        }
        if let Some(mut vel) = world.get_mut::<LinearVelocity>(player) {
            vel.0 = Vec3::ZERO;
        }
        if let Some(mut transform) = world.get_mut::<Transform>(player) {
            transform.translation = target;
        }
        true
    };

    match state.stage {
        // Menu walk: Scenarios -> the broadside row -> Play.
        0 => {
            if let Some(button) = entity_by_name(world, "Scenarios Button") {
                world.trigger(Activate { entity: button });
                advance(&mut state, 1, "opened the Scenarios picker");
            }
        }
        1 => {
            if let Some(row) = entity_by_name(world, "Scenario Row: broadside") {
                world.trigger(Activate { entity: row });
                advance(&mut state, 2, "selected Broadside");
            }
        }
        2 => {
            if let Some(play) = entity_by_name(world, "Scenario Play Button") {
                world.trigger(Activate { entity: play });
                advance(&mut state, 3, "clicked Play");
            }
        }
        // LOSE first: die in act 0, prove Defeat + Retry reloads clean.
        3 => {
            if act(world) == Some(0.0) && kill(world, "player_spaceship") {
                advance(&mut state, 4, "killed the player (defeat path)");
            }
        }
        4 => {
            // Overlay presence is part of the GATE, not an assert: the
            // outcome resource lands a frame before the overlay spawns
            // (PostUpdate write -> next Update sync), so asserting on the
            // first outcome frame is a race (review R1.2). A genuinely
            // missing overlay parks this stage and the stall deadline
            // names it.
            if outcome(world) == Some(ScenarioOutcomeKind::Defeat)
                && entity_by_name(world, "Outcome Overlay").is_some()
            {
                let retry = entity_by_name(world, "Outcome Primary Button").expect("Retry button");
                info!("probe: defeat overlay up, retrying");
                world.trigger(Activate { entity: retry });
                advance(&mut state, 5, "clicked Retry");
            }
        }
        5 => {
            // The retry tore down and reloaded: outcome cleared, act back
            // to 0, and a fresh player ship exists.
            if outcome(world).is_none() && act(world) == Some(0.0) {
                let mut q = world.query_filtered::<Entity, With<PlayerSpaceshipMarker>>();
                if q.iter(world).next().is_some() {
                    advance(&mut state, 6, "retry reloaded the slice clean");
                }
            }
        }
        // WIN on the reloaded instance.
        6 => {
            if teleport_player(world) {
                advance(&mut state, 7, "burned to the hauler (teleport)");
            }
        }
        7 => {
            if act(world) == Some(1.0) {
                // The ambush sprang (the avian area bridge fired for real).
                let both = kill(world, "corvette_a") & kill(world, "corvette_b");
                if both {
                    advance(&mut state, 8, "corvettes down");
                }
            }
        }
        8 => {
            if act(world) == Some(2.0) && kill(world, "gunship") {
                advance(&mut state, 9, "gunship broken");
            }
        }
        9 => {
            // Same gate shape as stage 4 (review R1.2): wait for outcome
            // AND overlay together.
            if outcome(world) == Some(ScenarioOutcomeKind::Victory)
                && entity_by_name(world, "Outcome Overlay").is_some()
            {
                assert!(
                    world.resource::<NovaEventWorld>().next_scenario.is_none(),
                    "the slice win queues nothing (Main Menu is the road out)"
                );
                info!("probe: victory overlay up");
                // Eyeball artifact: the win-variant capture (the Defeat
                // variant was cycle 1's probe; probe-the-adversarial-variant).
                if std::env::var_os("NOVA_SHOT_DIR").is_some() {
                    world
                        .spawn(bevy::render::view::screenshot::Screenshot::primary_window())
                        .observe(bevy::render::view::screenshot::save_to_disk(
                            std::path::Path::new(
                                &std::env::var("NOVA_SHOT_DIR").unwrap_or_default(),
                            )
                            .join("broadside_victory.png"),
                        ));
                }
                advance(&mut state, 10, "victory capture settling");
                state.wait = 45;
            }
        }
        10 => {
            // The suite's completion sentinel for SELF-ENDING examples: the
            // autopilot's own "cycle complete" only prints when its lifetime
            // expires, and idling out the remaining ~30s per CI run buys
            // nothing (tests/examples_smoke.rs accepts either line).
            info!("probe: script complete, exiting");
            state.done = true;
            world.write_message(AppExit::Success);
        }
        _ => unreachable!(),
    }

    world.insert_resource(state);
}
