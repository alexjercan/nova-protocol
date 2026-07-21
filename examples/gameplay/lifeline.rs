//! lifeline: the chapter-three convoy defense, wired to the smoke-test
//! harness (task 20260721-160957).
//!
//! Boots the exact app the binary runs, drives the Scenarios picker to
//! launch Lifeline, and plays the win/lose frame in ONE run: dies to prove
//! the Defeat overlay + Retry (the scenario reloads clean), then wins the
//! reloaded instance by clearing all three raider waves for the early-clear
//! Victory. The wave gates ride the SCENARIO CLOCK (`scenario_elapsed`), so
//! the script compresses time the same way the engine advances it - the
//! clock is an accumulated event-world variable (`tick_scenario_clock` adds
//! dt to the stored value), and the script jumps it forward at each stage;
//! ticking continues seamlessly from the jumped value. Every act transition
//! is otherwise staged on scenario STATE (vars/outcome/entities), never
//! wall-clock (event-driven-autopilot-beats).
//!
//! The kill stimulus is `HealthApplyDamage` overkill on the ship ROOT, the
//! production damage entry point, exactly like the broadside example.
//!
//! A completion guard turns the silent-timeout hole into a failure: if the
//! autopilot's lifetime expires before the script's final stage, the run
//! panics naming the stage it stalled in.
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//! ```text
//! BCS_AUTOPILOT=1 cargo run --example lifeline --features debug
//! # look for: `probe: defeat overlay up, retrying`,
//! #           `probe: victory overlay up`,
//! #           `probe: script complete, exiting`
//! ```

#[cfg(feature = "debug")]
use bevy::prelude::*;
use clap::Parser;
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "lifeline")]
#[command(version = "1.0.0")]
#[command(about = "The Lifeline convoy defense: defeat, retry, the three waves, victory", long_about = None)]
struct Cli;

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();

    let mut app = editor_app(true);

    #[cfg(feature = "debug")]
    {
        if std::env::var_os("BCS_AUTOPILOT").is_some() {
            app.insert_resource(bevy::ecs::error::FallbackErrorHandler(
                bevy::ecs::error::panic,
            ));
            app.add_systems(Last, guard_script_completion);
        }
        app.init_resource::<SliceAutopilot>();
        // Probe wiring (inert without the NOVA_PERF_* env): run timeline +
        // engine-bound invariants + frame-time capture.
        app.add_plugins(nova_probe::nova_timeline());
        app.add_plugins(nova_probe::nova_invariants());
        app.add_plugins(nova_probe::nova_frametime());
        // Two scenario loads (launch, retry) plus three staged waves; the
        // healthy walk exits itself in ~15s of compressed time.
        app.add_plugins(
            nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
                .self_completing()
                .hold(GameStates::Loading, 50.0)
                .input(slice_autopilot),
        );
        app.add_plugins(nova_screenshot());
    }

    app.run()
}

/// Stall deadline per stage (s): generous under llvmpipe, far below the
/// autopilot lifetime, so a stuck stage fails with its name.
#[cfg(feature = "debug")]
const STAGE_DEADLINE_SECS: f32 = 12.0;

#[cfg(feature = "debug")]
#[derive(Resource, Default)]
struct SliceAutopilot {
    stage: u32,
    stage_started: f32,
    wait: u32,
    done: bool,
    announce: Option<(u32, String)>,
}

#[cfg(feature = "debug")]
fn guard_script_completion(mut exits: MessageReader<AppExit>, script: Option<Res<SliceAutopilot>>) {
    let Some(script) = script else { return };
    if exits.read().next().is_some() && !script.done {
        panic!(
            "lifeline: run ended with the script stalled in stage {}",
            script.stage
        );
    }
}

#[cfg(feature = "debug")]
fn slice_autopilot(world: &mut World, elapsed: f32) {
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
            "lifeline: stage {} stalled for {STAGE_DEADLINE_SECS}s",
            state.stage
        );
    }

    let advance = |state: &mut SliceAutopilot, to: u32, note: &str| {
        info!("probe: stage {} -> {to}: {note}", state.stage);
        state.stage = to;
        state.stage_started = elapsed;
        state.wait = 10;
        state.announce = Some((to, note.to_string()));
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
    let num_var = |world: &World, key: &str| -> Option<f64> {
        match world.resource::<NovaEventWorld>().get_variable(key) {
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
    // Jump the scenario clock forward: the clock is an ACCUMULATED
    // event-world variable (tick_scenario_clock adds dt to the stored
    // value), so writing a larger value is a legitimate fast-forward - the
    // next tick continues from here and every clock gate sees it.
    let jump_clock = |world: &mut World, to: f64| {
        world
            .resource_mut::<NovaEventWorld>()
            .insert_variable("scenario_elapsed".to_string(), VariableLiteral::Number(to));
    };

    match state.stage {
        // Menu walk: Scenarios -> the lifeline row -> Play.
        0 => {
            if let Some(button) = entity_by_name(world, "Scenarios Button") {
                world.trigger(Activate { entity: button });
                advance(&mut state, 1, "opened the Scenarios picker");
            }
        }
        1 => {
            if let Some(row) = entity_by_name(world, "Scenario Row: lifeline") {
                world.trigger(Activate { entity: row });
                advance(&mut state, 2, "selected Lifeline");
            }
        }
        2 => {
            if let Some(play) = entity_by_name(world, "Scenario Play Button") {
                world.trigger(Activate { entity: play });
                advance(&mut state, 3, "clicked Play");
            }
        }
        // LOSE first: die on the live act, prove Defeat + Retry reloads.
        3 => {
            if num_var(world, "act") == Some(1.0) && kill(world, "player_spaceship") {
                advance(&mut state, 4, "killed the player (defeat path)");
            }
        }
        4 => {
            // Overlay presence is part of the GATE (outcome lands a frame
            // before the overlay; asserting the first frame is a race).
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
            // The retry tore down and reloaded: outcome cleared, the act
            // machine live again, a fresh player and BOTH haulers present,
            // and the relief countdown seeded back to full.
            if outcome(world).is_none() && num_var(world, "act") == Some(1.0) {
                let fresh = root_by_id(world, "player_spaceship").is_some()
                    && root_by_id(world, "hauler_queen").is_some()
                    && root_by_id(world, "hauler_meridian").is_some();
                if fresh {
                    advance(&mut state, 6, "retry reloaded the lane clean");
                }
            }
        }
        // WIN on the reloaded instance: fast-forward through the waves.
        6 => {
            jump_clock(world, 30.0);
            advance(&mut state, 7, "clock jumped past the first wave mark");
        }
        7 => {
            // Wave one arrived (the clock gate fired for real) and the
            // countdown readout variable tracks the jumped clock.
            if root_by_id(world, "raider_1a").is_some() {
                let remaining = num_var(world, "relief_remaining").unwrap_or(f64::MAX);
                assert!(
                    remaining < 215.0,
                    "the relief countdown tracks the clock (remaining {remaining})"
                );
                let both = kill(world, "raider_1a") & kill(world, "raider_1b");
                if both {
                    advance(&mut state, 8, "wave one down");
                }
            }
        }
        8 => {
            // Wave two holds until its clock mark: jump, then kill.
            if num_var(world, "r1a_down") == Some(1.0) && num_var(world, "r1b_down") == Some(1.0) {
                jump_clock(world, 100.0);
                advance(&mut state, 9, "clock jumped past the second wave mark");
            }
        }
        9 => {
            if root_by_id(world, "raider_2a").is_some() {
                let all =
                    kill(world, "raider_2a") & kill(world, "raider_2b") & kill(world, "raider_2c");
                if all {
                    advance(&mut state, 10, "wave two down");
                }
            }
        }
        10 => {
            if num_var(world, "r2c_down") == Some(1.0) {
                jump_clock(world, 170.0);
                advance(&mut state, 11, "clock jumped past the last wave mark");
            }
        }
        11 => {
            if root_by_id(world, "raider_3a").is_some() {
                let both = kill(world, "raider_3a") & kill(world, "raider_3b");
                if both {
                    advance(&mut state, 12, "the last wave down");
                }
            }
        }
        12 => {
            // The early-clear win: Victory overlay, the whole-convoy
            // banner, nothing queued (the finale task rewires this).
            if outcome(world) == Some(ScenarioOutcomeKind::Victory)
                && entity_by_name(world, "Outcome Overlay").is_some()
            {
                assert!(
                    world.resource::<NovaEventWorld>().next_scenario.is_none(),
                    "chapter three part one ends the chain until the finale lands"
                );
                info!("probe: victory overlay up");
                if std::env::var_os("NOVA_SHOT_DIR").is_some() {
                    world
                        .spawn(bevy::render::view::screenshot::Screenshot::primary_window())
                        .observe(bevy::render::view::screenshot::save_to_disk(
                            std::path::Path::new(
                                &std::env::var("NOVA_SHOT_DIR").unwrap_or_default(),
                            )
                            .join("lifeline_victory.png"),
                        ));
                }
                advance(&mut state, 13, "victory capture settling");
                state.wait = 45;
            }
        }
        13 => {
            // The suite's completion sentinel for SELF-ENDING examples
            // (tests/examples_smoke.rs accepts this line or the autopilot's).
            info!("probe: script complete, exiting");
            state.done = true;
            // Negotiated (task 20260720-000609): report done; the harness
            // watcher exits when every collector (script, capture) is done.
            world
                .resource_mut::<nova_protocol::nova_gameplay::bevy_common_systems::completion::HarnessCompletion>()
                .done(nova_protocol::nova_gameplay::bevy_common_systems::completion::AUTOPILOT);
        }
        _ => unreachable!(),
    }

    // Flush the stage marker `advance` buffered: stage transitions are the
    // script's design-promised beats (task 20260719-210450).
    let announce = state.announce.take();
    world.insert_resource(state);
    if let Some((stage, note)) = announce {
        nova_probe::probe_marker(
            world,
            &format!("stage {stage}"),
            serde_json::json!({ "note": note, "t": elapsed }),
        );
    }
}
