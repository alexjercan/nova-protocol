//! menu_scenarios: drive the main menu's Scenarios picker and MEASURE it.
//!
//! Boots the exact app the `nova_protocol` binary runs (via the shared
//! [`editor_app`]), clicks Scenarios, then selects every listed scenario row in
//! turn. After each selection settles it logs the laid-out width of the two
//! panes ("Scenarios List" and "Scenario Details Panel") and, at the end, a
//! verdict line saying whether those widths held constant across selections.
//!
//! This is the rig for task 20260729-211150: the picker's split must NOT depend
//! on which scenario is selected (a long description or a thumbnail must not
//! resize the list). Real fonts, real text measure, real taffy - a headless
//! unit rig measures every text node as zero-width and cannot see this at all.
//!
//! Run (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//! ```text
//! BCS_AUTOPILOT=1 cargo run --example menu_scenarios --features debug
//! # look for: `scenarios pane widths:` per row, then
//! #           `scenarios pane widths HELD` / `... CHANGED`,
//! #           then `nova harness: reached Playing` and
//! #           `probe: script complete, exiting` (it finishes by clicking Play
//! #           on the last selection, so the smoke suite can run it)
//! ```
//!
//! With `BCS_REEL=1` it also captures `scenarios-picker-<id>.png` per selection
//! (staged under `NOVA_SHOT_DIR`), so the campaign indentation and the held
//! split can be EYEBALLED and not just measured.

#[cfg(feature = "debug")]
use bevy::prelude::*;
use clap::Parser;
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "menu_scenarios")]
#[command(version = "1.0.0")]
#[command(about = "Drive the Scenarios picker and measure its pane widths", long_about = None)]
struct Cli;

/// The RUNWAY, not the pacing budget: the walk is frame-counted (roughly 21
/// frames per listed scenario) and ENDS ITSELF once the launched scenario is up
/// (`self_completing`), so this only has to outlast the slowest plausible walk.
/// Sized for a software-rendered CI GPU (llvmpipe) with room for the scenario
/// set to keep growing, and kept UNDER the harness completion deadline
/// (`BCS_HARNESS_DEADLINE`, default 120 s) so the runway is what expires first
/// and the stall is named rather than reported as a deadline (review R2.1/R2.3).
#[cfg(feature = "debug")]
const SCENARIOS_AUTOPILOT_SECS: f32 = 100.0;

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();

    // The same app the game/binary runs - not a bespoke copy.
    let mut app = editor_app(true);

    #[cfg(feature = "debug")]
    {
        if std::env::var_os("BCS_AUTOPILOT").is_some() {
            app.insert_resource(bevy::ecs::error::FallbackErrorHandler(
                bevy::ecs::error::panic,
            ));
        }
        app.init_resource::<ScenariosAutopilot>();
        // Probe wiring (inert without its NOVA_PERF_* env).
        app.add_plugins(nova_probe::nova_timeline());
        app.add_plugins(nova_probe::nova_invariants());
        app.add_plugins(nova_probe::nova_frametime());
        app.add_plugins(
            AutopilotPlugin::<GameStates>::new()
                .hold(GameStates::Loading, SCENARIOS_AUTOPILOT_SECS)
                // SCRIPT-OWNED completion (the broadside pattern): the walk
                // reports done when it lands, and a timeline that expires first
                // means the script STALLED - the autopilot then writes
                // `AppExit::error` from `PreUpdate`, which is both an abort and
                // the only exit a `Last` reader can observe. Without this the
                // expiry would report an ordinary "cycle complete" over an
                // unfinished walk AND `guard_run_completion` would never see the
                // exit at all (review R2.1 - it did not).
                .self_completing()
                .input(scenarios_autopilot),
        );
        // Harness-only: an interactive run never finishes a walk, so the guard
        // would panic on an ordinary window close (review R2.2).
        if std::env::var_os("BCS_AUTOPILOT").is_some() {
            app.add_systems(Last, guard_run_completion);
        }
        app.add_plugins(nova_screenshot());
    }

    app.run()
}

/// Frame-paced walk over the scenario rows.
#[cfg(feature = "debug")]
#[derive(Resource, Default)]
struct ScenariosAutopilot {
    opened: bool,
    /// Rows already clicked, by their `Name` (the list is rebuilt on every
    /// selection, so entity ids do not survive a step).
    visited: Vec<String>,
    /// `(list width, details width)` measured after each selection settled.
    measured: Vec<(String, f32, f32)>,
    /// The walk is over and Play has been clicked.
    launched: bool,
    /// The launched scenario came up and the completion was reported.
    finished: bool,
    wait: u32,
}

/// A run that exits with the walk unfinished is a STALL, not a pass: without
/// this the safety window expiring mid-walk would look like an ordinary
/// "cycle complete" (or a confusing never-reached-Playing) instead of naming
/// what did not happen (the broadside completion-guard pattern).
#[cfg(feature = "debug")]
fn guard_run_completion(mut exits: MessageReader<AppExit>, state: Option<Res<ScenariosAutopilot>>) {
    let Some(state) = state else { return };
    if exits.read().next().is_some() && !state.finished {
        panic!(
            "menu_scenarios: run ended with the walk unfinished ({} of the \
             picker's rows measured, launched={})",
            state.measured.len(),
            state.launched
        );
    }
}

#[cfg(feature = "debug")]
fn button_by_name(world: &mut World, name: &str) -> Option<Entity> {
    let mut names = world.query::<(Entity, &Name)>();
    names
        .iter(world)
        .find(|(_, n)| n.as_str() == name)
        .map(|(entity, _)| entity)
}

/// The laid-out width (logical px) of the uniquely-named node, if it has been
/// through layout yet. `ComputedNode::size` is PHYSICAL; multiplying by the
/// inverse scale factor is what gives logical px (review R1.2 - dividing
/// reported `physical * scale_factor`, which only reads right at scale 1).
#[cfg(feature = "debug")]
fn width_by_name(world: &mut World, name: &str) -> Option<f32> {
    let mut q = world.query::<(&Name, &ComputedNode)>();
    q.iter(world)
        .find(|(n, _)| n.as_str() == name)
        .map(|(_, computed)| computed.size().x * computed.inverse_scale_factor())
}

/// Every scenario row currently in the list, by name, in list order.
#[cfg(feature = "debug")]
fn scenario_row_names(world: &mut World) -> Vec<String> {
    let mut q = world.query::<&Name>();
    let mut names: Vec<String> = q
        .iter(world)
        .map(|n| n.as_str().to_string())
        .filter(|n| n.starts_with("Scenario Row: "))
        .collect();
    names.sort();
    names
}

/// Open the picker, click each row once, measure the panes after each, report
/// whether the split held, and launch the last selection.
#[cfg(feature = "debug")]
fn scenarios_autopilot(world: &mut World, _elapsed: f32) {
    use bevy::ui_widgets::Activate;

    let playing = matches!(
        *world.resource::<State<GameStates>>().get(),
        GameStates::Playing
    );

    let mut state = world.remove_resource::<ScenariosAutopilot>().unwrap();

    // SELF-ENDING (the broadside pattern): the launched scenario is up, so the
    // walk is finished - say so and report the collector done instead of idling
    // out the safety window. `guard_run_completion` turns an exit BEFORE this
    // point into a panic, so a walk that outran the window fails loudly as a
    // stall rather than silently as "never reached Playing".
    if playing {
        if state.launched && !state.finished {
            state.finished = true;
            info!("probe: script complete, exiting");
            world
                .resource_mut::<nova_protocol::nova_gameplay::bevy_common_systems::completion::HarnessCompletion>()
                .done(nova_protocol::nova_gameplay::bevy_common_systems::completion::AUTOPILOT);
        }
        world.insert_resource(state);
        return;
    }

    if state.wait > 0 {
        state.wait -= 1;
        world.insert_resource(state);
        return;
    }
    if state.launched {
        world.insert_resource(state);
        return;
    }

    if !state.opened {
        if let Some(button) = button_by_name(world, "Scenarios Button") {
            world.trigger(Activate { entity: button });
            state.opened = true;
            state.wait = 10;
            info!("probe: opened the Scenarios picker");
        }
        world.insert_resource(state);
        return;
    }

    // Measure the selection made on the previous step (it has settled by now).
    if let Some(last) = state.visited.last().cloned() {
        if state.measured.len() < state.visited.len() {
            let list = width_by_name(world, "Scenarios List");
            let details = width_by_name(world, "Scenario Details Panel");
            if let (Some(list), Some(details)) = (list, details) {
                info!("scenarios pane widths: list={list:.1} details={details:.1} after {last}");
                if std::env::var_os("BCS_REEL").is_some() {
                    let id = last.trim_start_matches("Scenario Row: ");
                    capture_window(world, &format!("scenarios-picker-{id}.png"));
                    // Let the PNG land BEFORE the next row is clicked: the
                    // capture resolves at the end of the frame, so clicking on
                    // through in this same frame renders the NEXT selection into
                    // the shot (it did - the first cut of this rig produced
                    // shots one selection ahead of their filename).
                    state.measured.push((last, list, details));
                    state.wait = 4;
                    world.insert_resource(state);
                    return;
                }
                state.measured.push((last, list, details));
            }
        }
    }

    // Click the next unvisited row.
    let rows = scenario_row_names(world);
    let next = rows.iter().find(|n| !state.visited.contains(n)).cloned();
    match next {
        Some(name) => {
            if let Some(row) = button_by_name(world, &name) {
                world.trigger(Activate { entity: row });
                state.visited.push(name.clone());
                // A selection rebuilds BOTH panes (and may load a thumbnail);
                // give layout and the image load room before measuring.
                state.wait = 20;
            } else {
                // Vanished between listing and clicking - do not spin on it.
                state.visited.push(name);
            }
        }
        None => {
            report(&state);
            // Finish the way a player does: launch the scenario the picker has
            // selected. That also gives the smoke suite its reach-Playing
            // contract, and puts the details pane's Play button - the picker's
            // whole point - under the harness.
            if let Some(play) = button_by_name(world, "Scenario Play Button") {
                world.trigger(Activate { entity: play });
                info!("probe: clicked Play on the selected scenario");
            } else {
                warn!("probe: no Play button in the details pane to finish on");
            }
            state.launched = true;
        }
    }

    world.insert_resource(state);
}

/// The verdict line the run is read by: every measured selection must have the
/// same pane widths.
///
/// Under `BCS_AUTOPILOT` this is an in-example ASSERTION, not just a log line:
/// the smoke suite (`tests/examples_smoke.rs`) only greps for reach-Playing and
/// a clean exit, so a rig that merely logged `CHANGED` would let the exact
/// regression this example exists to catch pass CI green (review R1.3). A run
/// that measured NOTHING fails the same way - "no rows found" is not a pass.
#[cfg(feature = "debug")]
fn report(state: &ScenariosAutopilot) {
    let harnessed = std::env::var_os("BCS_AUTOPILOT").is_some();
    let Some((_, first_list, first_details)) = state.measured.first() else {
        if harnessed {
            panic!(
                "scenarios pane widths: NO measurements - the picker listed no \
                 scenario rows, so nothing was proven"
            );
        }
        warn!("scenarios pane widths: NO measurements (no scenario rows found)");
        return;
    };
    let spread = |pick: fn(&(String, f32, f32)) -> f32| -> f32 {
        let values: Vec<f32> = state.measured.iter().map(pick).collect();
        let min = values.iter().cloned().fold(f32::INFINITY, f32::min);
        let max = values.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
        max - min
    };
    let list_spread = spread(|m| m.1);
    let details_spread = spread(|m| m.2);
    // Sub-pixel spread is rounding, not a layout dependency.
    if list_spread <= 0.5 && details_spread <= 0.5 {
        info!(
            "scenarios pane widths HELD across {} selections (list={first_list:.1} \
             details={first_details:.1})",
            state.measured.len()
        );
    } else {
        error!(
            "scenarios pane widths CHANGED across {} selections: list spread {list_spread:.1}px, \
             details spread {details_spread:.1}px",
            state.measured.len()
        );
        for (name, list, details) in &state.measured {
            error!("  {name}: list={list:.1} details={details:.1}");
        }
        if harnessed {
            panic!(
                "scenarios pane widths CHANGED across {} selections (list spread \
                 {list_spread:.1}px, details spread {details_spread:.1}px) - the \
                 picker's split must not depend on the selection",
                state.measured.len()
            );
        }
    }
}
