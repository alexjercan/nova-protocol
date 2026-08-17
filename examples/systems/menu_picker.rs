//! menu_picker: drive the main menu's Scenarios picker and MEASURE it.
//!
//! Boots the exact app the `nova_protocol` binary runs (via the shared
//! [`editor_app`]), clicks Scenarios, then selects every scenario row that lies
//! inside the list's visible box, in turn. The picker does not scroll under the
//! harness, so a row past the fold is warned about and skipped rather than
//! faked - and the run FAILS if fewer than two rows were reached, because the
//! split it measures is only testable ACROSS selections. Every click is a REAL
//! gesture: the pointer is moved to the widget's
//! own screen position, resolved from its `Name`, and pressed and released
//! there (task 20260804-094021) - nothing is reached by triggering its
//! observer. After each selection settles it logs the laid-out width of the two
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
//! NOVA_AUTOPILOT=1 cargo run --example menu_picker --features debug
//! # look for: `scenarios pane widths:` per row, then
//! #           `scenarios pane widths HELD` / `... CHANGED`,
//! #           then `nova harness: reached Playing` and
//! #           `probe: script complete, exiting` (it finishes by clicking Play
//! #           on the last selection, so the smoke suite can run it)
//! ```
//!
//! With `NOVA_CAPTURE=1` it also shoots `scenarios-picker-<id>.png` per selection
//! (staged under `NOVA_SHOT_DIR`), so the campaign indentation and the held
//! split can be EYEBALLED and not just measured.

#[cfg(feature = "debug")]
use bevy::prelude::*;
use clap::Parser;
use nova_protocol::prelude::*;
#[cfg(feature = "debug")]
use nova_ui::widget::Selected;

#[derive(Parser)]
#[command(name = "menu_picker")]
#[command(version = "1.0.0")]
#[command(about = "Drive the Scenarios picker and measure its pane widths", long_about = None)]
struct Cli;

/// The step DEADLINE, not the pacing budget: the walk is frame-counted (roughly
/// 21 frames per listed scenario) and ENDS ITSELF once the launched scenario is
/// up, so this only has to outlast the slowest plausible walk. Sized for a
/// software-rendered CI GPU (llvmpipe) with room for the scenario set to keep
/// growing, and kept UNDER the harness completion deadline
/// (`NOVA_AUTOPILOT_DEADLINE`, default 120 s) so the step deadline is what
/// expires first and the stall is named rather than reported as a generic
/// deadline (review R2.1/R2.3).
#[cfg(feature = "debug")]
const SCENARIOS_AUTOPILOT_SECS: f32 = 100.0;

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();

    // The same app the game/binary runs - not a bespoke copy.
    let mut app = editor_app(true, None);

    #[cfg(feature = "debug")]
    {
        if std::env::var_os("NOVA_AUTOPILOT").is_some() {
            app.insert_resource(bevy::ecs::error::FallbackErrorHandler(
                bevy::ecs::error::panic,
            ));
        }
        app.init_resource::<ScenariosAutopilot>();
        // Probe wiring (inert without its NOVA_PERF_* env).
        app.add_plugins(nova_probe::NovaProbePlugin::default());
        app.add_plugins(
            nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
                // SCRIPT-OWNED completion: the step
                // ends where the walk reports done, and a deadline that
                // expires first means the script STALLED - which is an error
                // exit naming this step, not an ordinary "cycle complete" over
                // an unfinished walk (review R2.1).
                .step("walk the scenarios picker")
                .enter(GameStates::Loading)
                .each(scenarios_autopilot)
                .until(nova_protocol::nova_debug::harness::script_reports_done())
                .deadline(SCENARIOS_AUTOPILOT_SECS)
                .add(),
        );
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
    /// Rows the walk gave up on, with the reason. Named in the verdict, so a
    /// thin run says WHICH rows it could not reach rather than only how many.
    skipped: Vec<(String, &'static str)>,
    /// The row currently being waited on, and for how many driven frames. Reset
    /// whenever the candidate changes, so the budget is per row.
    settling: Option<(String, u32)>,
    /// The row a click was just issued on, awaiting its measurement. Set only
    /// when the gesture actually went out, so a row that never laid out is
    /// skipped rather than measured against the PREVIOUS selection's panes.
    pending_measure: Option<String>,
    /// The walk is over and Play has been clicked.
    launched: bool,
    /// The launched scenario came up and the completion was reported.
    finished: bool,
    /// A press is outstanding: the next driven frame releases it. Widgets act
    /// on `Activate`, which fires on RELEASE over the same node, so every click
    /// this walk makes spans two frames.
    pending_release: bool,
    /// A shot whose PNG is still being written: the walk holds here until
    /// `CaptureLog` acks it, so the next row is never clicked into a pending
    /// capture.
    pending_shot: Option<String>,
    wait: u32,
}

/// Driven frames a row gets to lay out inside the list before the walk gives up
/// on it.
///
/// Every selection REBUILDS the list, so on the frames right after a rebuild
/// the list - or the row - has no rect at all. A single-frame look read that as
/// "below the fold" and skipped the row for good, which is why the measured
/// count varied run to run (review R2.1). A row is only skipped once it has
/// failed to settle for this many consecutive driven frames, so only a row that
/// is genuinely past the fold is dropped.
#[cfg(feature = "debug")]
const ROW_SETTLE_FRAMES: u32 = 10;

/// Where a row is, as far as the walk can tell THIS frame.
///
/// A row scrolled past the fold still LAYS OUT, so `ui_node_centre` happily
/// returns a coordinate for it - one that points at whatever is actually on
/// screen there. Clicking it would select nothing and the walk would measure the
/// previous selection's panes. The picker does not scroll under the harness, so
/// a row below the fold is skipped rather than faked.
///
/// The three not-yet-clickable cases are kept APART because they mean different
/// things: two are transient and one is permanent, and collapsing them into one
/// `false` is what made a settling list indistinguishable from a fold.
#[cfg(feature = "debug")]
enum RowPlacement {
    /// Inside the list's box, at this logical-pixel centre. Click it.
    OnScreen(Vec2),
    /// Not clickable this frame, for the reason named (for the warn).
    Unreached(&'static str),
}

/// Locate `name` against the `Scenarios List` box.
#[cfg(feature = "debug")]
fn row_placement(world: &mut World, name: &str) -> RowPlacement {
    let Some(list) = ui_node_rect(world, "Scenarios List") else {
        return RowPlacement::Unreached("the Scenarios List has not laid out yet");
    };
    let Some(row) = ui_node_rect(world, name) else {
        return RowPlacement::Unreached("the row has not laid out yet");
    };
    if list.contains(row.center()) {
        RowPlacement::OnScreen(row.center())
    } else {
        RowPlacement::Unreached("the row's centre is outside the list's box")
    }
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
    let playing = matches!(
        *world.resource::<State<GameStates>>().get(),
        GameStates::Playing
    );

    let mut state = world.remove_resource::<ScenariosAutopilot>().unwrap();

    // SELF-ENDING: the launched scenario is up, so the
    // walk is finished - say so and report the collector done instead of idling
    // out the safety window. `guard_run_completion` turns an exit BEFORE this
    // point into a panic, so a walk that outran the window fails loudly as a
    // stall rather than silently as "never reached Playing".
    if playing {
        if state.launched && !state.finished {
            state.finished = true;
            info!("probe: script complete, exiting");
            world
                .resource_mut::<nova_protocol::nova_debug::harness::HarnessCompletion>()
                .done(nova_protocol::nova_debug::harness::AUTOPILOT);
        }
        world.insert_resource(state);
        return;
    }

    if state.wait > 0 {
        state.wait -= 1;
        world.insert_resource(state);
        return;
    }

    // Hold for the ack rather than for a guessed number of frames: the capture
    // resolves at the end of a later frame, and clicking on through before it
    // does renders the NEXT selection into the shot (it did - the first cut of
    // this rig produced shots one selection ahead of their filename).
    if let Some(shot) = state.pending_shot.clone() {
        if !world.resource::<CaptureLog>().wrote(&shot) {
            world.insert_resource(state);
            return;
        }
        state.pending_shot = None;
    }

    // Let go of the press made on a previous frame; THIS is what fires
    // `Activate` on the widget under the pointer.
    if state.pending_release {
        release_mouse(MouseButton::Left)(world);
        state.pending_release = false;
        // A selection rebuilds BOTH panes (and may load a thumbnail); give
        // layout and the image load room before measuring.
        state.wait = 20;
        world.insert_resource(state);
        return;
    }

    if state.launched {
        world.insert_resource(state);
        return;
    }

    if !state.opened {
        // Resolved once and clicked at the bound centre: `click_named` would
        // resolve the same node a second time (review R1.7).
        if let Some(centre) = ui_node_centre(world, "Scenarios Button") {
            click_at(centre, MouseButton::Left)(world);
            state.opened = true;
            state.pending_release = true;
            info!("probe: opened the Scenarios picker");
        }
        world.insert_resource(state);
        return;
    }

    // Measure the selection made on the previous step (it has settled by now).
    if let Some(last) = state.pending_measure.clone() {
        // The click LANDED, or the measurement proves nothing. A pointer
        // gesture can miss where triggering the observer directly could not (a
        // row scrolled out, occluded, or hit-tested to another node), and a
        // missed click leaves both panes untouched - so the widths would "hold"
        // across selections that never happened (review R1.1).
        assert_selection_landed(world, &last);
        let list = ui_node_rect(world, "Scenarios List").map(|rect| rect.width());
        let details = ui_node_rect(world, "Scenario Details Panel").map(|rect| rect.width());
        if let (Some(list), Some(details)) = (list, details) {
            info!("scenarios pane widths: list={list:.1} details={details:.1} after {last}");
            state.pending_measure = None;
            let id = last.trim_start_matches("Scenario Row: ").to_string();
            let shot = format!("scenarios-picker-{id}.png");
            shoot(world, &shot);
            state.pending_shot = capturing().then_some(shot);
            state.measured.push((last, list, details));
        } else {
            // The click LANDED (asserted above) but a pane has not laid out, so
            // this selection cannot be measured. It is recorded as SKIPPED, not
            // merely warned about: a row in neither `measured` nor `skipped`
            // would make `report()` state a coverage the run never had, which
            // is the one thing that string exists to prevent (review R3.2).
            // `pending_measure` is cleared with it, so the row is accounted
            // ONCE rather than once per frame until the next click overwrites
            // it.
            let laid_out = |width: Option<f32>| if width.is_some() { "yes" } else { "no" };
            warn!(
                "scenarios: no pane widths after {last} (list laid out: {}, details laid out: \
                 {}); dropping that measurement",
                laid_out(list),
                laid_out(details)
            );
            state.skipped.push((last, "its pane widths never laid out"));
            state.pending_measure = None;
        }
    }

    // Click the next unvisited row.
    let rows = scenario_row_names(world);
    let next = rows.iter().find(|n| !state.visited.contains(n)).cloned();
    match next {
        Some(name) => match row_placement(world, &name) {
            RowPlacement::OnScreen(centre) => {
                click_at(centre, MouseButton::Left)(world);
                state.pending_measure = Some(name.clone());
                state.visited.push(name);
                state.settling = None;
                state.pending_release = true;
            }
            RowPlacement::Unreached(reason) => {
                // Give the rebuilt list room to settle before believing this.
                // Only a row that stays unreachable for the whole budget is
                // skipped, and the warn names WHICH of the three cases it was.
                let waited = match &state.settling {
                    Some((settling, waited)) if *settling == name => waited + 1,
                    _ => 1,
                };
                if waited >= ROW_SETTLE_FRAMES {
                    warn!(
                        "scenarios: row `{name}` unreachable after {ROW_SETTLE_FRAMES} driven \
                         frames ({reason}); skipping it"
                    );
                    state.visited.push(name.clone());
                    state.skipped.push((name, reason));
                    state.settling = None;
                } else {
                    state.settling = Some((name, waited));
                }
            }
        },
        None => {
            report(world, &state);
            // Finish the way a player does: launch the scenario the picker has
            // selected. That also gives the smoke suite its reach-Playing
            // contract, and puts the details pane's Play button - the picker's
            // whole point - under the harness.
            if let Some(centre) = ui_node_centre(world, "Scenario Play Button") {
                click_at(centre, MouseButton::Left)(world);
                state.pending_release = true;
                info!("probe: clicked Play on the selected scenario");
            } else {
                warn!("probe: no Play button in the details pane to finish on");
            }
            state.launched = true;
        }
    }

    world.insert_resource(state);
}

/// The row named `name` must be the selected one before its measurement counts.
///
/// `select_scenario_row` (`crates/nova_menu/src/scenarios.rs:596`) inserts
/// [`Selected`] on the clicked row and removes it from every other, so this is
/// the picker's own record of which click landed - not a restatement of what
/// the beat intended. Under `NOVA_AUTOPILOT` a miss is fatal, the same way
/// `report()` treats zero measurements; interactively it only warns.
#[cfg(feature = "debug")]
fn assert_selection_landed(world: &mut World, name: &str) {
    let selected = world
        .query_filtered::<&Name, With<Selected>>()
        .iter(world)
        .any(|selected| selected.as_str() == name);
    if selected {
        nova_probe::probe_marker(
            world,
            "outcome: the row click selects the row",
            serde_json::json!({ "row": name }),
        );
        return;
    }
    if std::env::var_os("NOVA_AUTOPILOT").is_some() {
        panic!(
            "the click on `{name}` never landed: the picker has not marked that \
             row Selected, so its pane widths measure the PREVIOUS selection"
        );
    }
    warn!("scenarios: the click on `{name}` did not select it");
}

/// The verdict line the run is read by: every measured selection must have the
/// same pane widths.
///
/// Under `NOVA_AUTOPILOT` this is an in-example ASSERTION, not just a log line:
/// the probe sweep only grades reach-Playing, a clean exit and the invariant
/// set, so a rig that merely logged `CHANGED` would let the exact
/// regression this example exists to catch pass CI green (review R1.3).
///
/// A thin run fails the same way. The property is a COMPARISON across
/// selections, so one measurement makes both spreads `max - min` over a single
/// value - zero - and prints `HELD` having proven nothing (review R2.3). Two is
/// the floor, and falling under it names the rows that were skipped, since a
/// skip is the only way to get there.
#[cfg(feature = "debug")]
fn report(world: &mut World, state: &ScenariosAutopilot) {
    let harnessed = std::env::var_os("NOVA_AUTOPILOT").is_some();
    // COVERAGE, stated on every verdict and not only when something goes wrong.
    // The rows the walk could not reach are the difference between "the split
    // held" and "the split held over whatever happened to be on screen", and a
    // verdict that does not say which it is cannot be read later.
    let coverage = if state.skipped.is_empty() {
        format!("{} rows, none skipped", state.measured.len())
    } else {
        let skipped: Vec<String> = state
            .skipped
            .iter()
            .map(|(name, reason)| format!("{name} ({reason})"))
            .collect();
        format!(
            "{} rows measured, {} skipped: {}",
            state.measured.len(),
            state.skipped.len(),
            skipped.join(", ")
        )
    };

    if state.measured.len() < 2 {
        let detail = format!(
            "only {} of the picker's rows were measured, and the split is only \
             testable ACROSS selections - {coverage}",
            state.measured.len()
        );
        if harnessed {
            panic!("scenarios pane widths: TOO FEW measurements - {detail}");
        }
        warn!("scenarios pane widths: TOO FEW measurements - {detail}");
        return;
    }
    nova_probe::probe_marker(
        world,
        "outcome: two or more rows measured",
        serde_json::json!({ "measured": state.measured.len() }),
    );
    // At least two, per the floor above.
    let (_, first_list, first_details) = &state.measured[0];
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
        nova_probe::probe_marker(
            world,
            "outcome: the pane split holds across selections",
            serde_json::json!({ "list_spread": list_spread, "details_spread": details_spread }),
        );
        info!(
            "scenarios pane widths HELD across {} selections (list={first_list:.1} \
             details={first_details:.1}) - coverage: {coverage}",
            state.measured.len()
        );
    } else {
        error!(
            "scenarios pane widths CHANGED across {} selections: list spread {list_spread:.1}px, \
             details spread {details_spread:.1}px - coverage: {coverage}",
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
