//! system_scenario_picker: drive the main menu's Scenarios picker, MEASURE it,
//! and prove it starts the row the pointer chose.
//!
//! Boots the exact app the `nova_protocol` binary runs (via the shared
//! [`editor_app`]), enables the example mod so the picker lists more than the
//! base game's single scenario, clicks Scenarios, then selects every scenario
//! row in turn - including the ones past the fold, which it reaches with the
//! wheel the way a player does. The run FAILS if fewer than two rows were
//! reached, because the split it measures is only testable ACROSS selections.
//! Every gesture is a REAL one: the pointer is moved to the widget's own screen
//! position, resolved from its `Name`, and pressed and released there - nothing
//! is reached by triggering its observer. After each selection settles it logs
//! the laid-out width of the two panes ("Scenarios List" and "Scenario Details
//! Panel") and, at the end, a verdict line saying whether those widths held
//! constant across selections.
//!
//! It then finishes as a player does: it leaves a row that is NOT the one the
//! picker opened on selected, clicks Play, waits for the atomic load gate to
//! release, and asserts the live `CurrentScenario` is exactly that row. A
//! picker that lays out perfectly and starts the wrong scenario is still
//! broken, and nothing else in the tree drives that delivery through real
//! input.
//!
//! The property this rig exists for: the picker's split must NOT depend on
//! which scenario is selected (a long description or a thumbnail must not
//! resize the list). Real fonts, real text measure, real taffy - a headless
//! unit rig measures every text node as zero-width and cannot see this at all.
//!
//! Run (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//! ```text
//! NOVA_AUTOPILOT=1 cargo run --example system_scenario_picker --features debug
//! # look for: `scenarios pane widths:` per row, then
//! #           `scenarios pane widths HELD` / `... CHANGED`,
//! #           then `probe: the picker started <id>`,
//! #           `nova harness: reached Playing` and
//! #           `probe: script complete, exiting`
//! ```
//!
//! With `NOVA_CAPTURE=1` it also shoots `scenarios-picker-<id>.png` per selection
//! (staged under `NOVA_CAPTURE_DIR`), so the campaign indentation and the held
//! split can be EYEBALLED and not just measured.

#[cfg(feature = "debug")]
use bevy::prelude::*;
use clap::Parser;
use nova_protocol::prelude::*;
#[cfg(feature = "debug")]
use nova_ui::widget::Selected;

#[derive(Parser)]
#[command(name = "system_scenario_picker")]
#[command(version = "1.0.0")]
#[command(about = "Drive the Scenarios picker, measure its pane widths and prove it starts the clicked row. Autopilot-only correctness range", long_about = None)]
struct Cli;

/// The step DEADLINE, not the pacing budget: the walk is frame-counted (roughly
/// 21 frames per listed scenario) and ENDS ITSELF once the launched scenario is
/// up, so this only has to outlast the slowest plausible walk. Sized for a
/// software-rendered CI GPU (llvmpipe) with room for the scenario set to keep
/// growing, and kept UNDER the harness completion deadline
/// (`NOVA_AUTOPILOT_DEADLINE`, default 120 s) so the step deadline is what
/// expires first and the stall is named rather than reported as a generic
/// deadline.
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
        // Probe wiring (inert without its NOVA_PROBE_* env).
        app.add_plugins(nova_probe::NovaProbePlugin::default());
        // No `nova_screenshot` beat here: the walk shoots its own per-selection
        // pictures, and it OWNS its completion - a beat appended after the step
        // that reports done would never be reached.
        app.add_plugins(
            nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
                .step("reach the main menu")
                .enter(GameStates::Loading)
                .until(state_is(GameStates::MainMenu))
                .deadline(STEP_DEADLINE_SECS)
                .add()
                // The base game lists ONE scenario, and the split this rig
                // measures is only testable across selections - so the second
                // row is authored here rather than waited for. The example mod
                // ships `example_arena`, the same row
                // `screenshot_scenario_picker` enables it for, and its
                // description and thumbnail differ from the training range's,
                // which is exactly the input the pane split must be immune to.
                .step("enable the example mod")
                .on_enter(|world: &mut World| {
                    world
                        .resource_mut::<EnabledMods>()
                        .0
                        .insert("example".to_string());
                })
                .until(frames(SETTLE_FRAMES * 2))
                .add()
                // SCRIPT-OWNED completion: the step
                // ends where the walk reports done, and a deadline that
                // expires first means the script STALLED - which is an error
                // exit naming this step, not an ordinary "cycle complete" over
                // an unfinished walk.
                .step("walk the scenarios picker")
                .each(scenarios_autopilot)
                .until(nova_protocol::nova_debug::harness::script_reports_done())
                .deadline(SCENARIOS_AUTOPILOT_SECS)
                .add(),
        );
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
    /// The row the picker had selected ITSELF when it opened, read before the
    /// walk clicked anything. The delivery claim is only worth something
    /// against it: a run that plays the default row proves nothing about
    /// whether the click reached the loader.
    default_row: Option<String>,
    /// The non-default row this run leaves selected and plays.
    target_row: Option<String>,
    /// The pane verdict has been stated. The walk stays in the closing beat
    /// for several frames while it re-selects its target, and the verdict is a
    /// summary of the walk, not of those frames.
    reported: bool,
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

/// Attempts a row gets to become clickable - to lay out, or to be scrolled in -
/// before the walk gives up on it.
///
/// Every selection REBUILDS the list, so on the frames right after a rebuild
/// the list - or the row - has no rect at all. A single-frame look read that as
/// "below the fold" and skipped the row for good, which is why the measured
/// count varied run to run. A row is only skipped once it has failed for this
/// many consecutive attempts, which is also what bounds the scroll: a row the
/// wheel cannot bring in is dropped instead of chased.
#[cfg(feature = "debug")]
const ROW_SETTLE_FRAMES: u32 = 10;

/// Where a row is, as far as the walk can tell THIS frame.
///
/// A row scrolled past the fold still LAYS OUT, so `ui_node_centre` happily
/// returns a coordinate for it - one that points at whatever is actually on
/// screen there. Clicking it would select nothing and the walk would measure the
/// previous selection's panes. So a row below the fold is SCROLLED to instead,
/// with the wheel, the way a player reaches it.
///
/// The cases are kept APART because they mean different things: one is
/// clickable now, one after a scroll, and one only after the list settles.
/// Collapsing them into one `false` is what made a settling list
/// indistinguishable from a fold.
#[cfg(feature = "debug")]
enum RowPlacement {
    /// Inside the list's box, at this logical-pixel centre. Click it.
    OnScreen(Vec2),
    /// Laid out but past the fold: turn the wheel by this many logical pixels
    /// to bring it in, then look again.
    PastTheFold(f32),
    /// Not clickable this frame, for the reason named (for the warn).
    Unreached(&'static str),
}

/// How far INSIDE the list a scroll aims to put the row's centre.
///
/// A scroll that lands the centre exactly on the fold edge is a rounding error
/// away from still reading as outside, and the walk would scroll again and
/// oscillate. One row height clears the boundary and puts the whole row on
/// screen, since the placement test only looks at the centre.
#[cfg(feature = "debug")]
const SCROLL_MARGIN_ROWS: f32 = 1.0;

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
        return RowPlacement::OnScreen(row.center());
    }
    // Only the VERTICAL fold is scrollable: the list scrolls on y alone, so a
    // row outside on x is a layout the wheel cannot fix and must be reported as
    // such rather than scrolled at forever.
    let centre = row.center().y;
    let margin = row.height() * SCROLL_MARGIN_ROWS;
    // `scroll_viewports` SUBTRACTS the wheel delta from the scroll offset, and
    // the offset moves the content the other way again - so a positive delta
    // moves the content DOWN the screen, which is what a row above the box
    // needs.
    if centre > list.max.y {
        RowPlacement::PastTheFold(list.max.y - centre - margin)
    } else if centre < list.min.y {
        RowPlacement::PastTheFold(list.min.y - centre + margin)
    } else {
        RowPlacement::Unreached("the row's centre is outside the list's box on x")
    }
}

/// What `spawn_scenario_row` (`crates/nova_menu/src/scenarios.rs:350`) names a
/// row: the prefix, then the scenario id the row launches. Stripping it is how
/// the walk turns a widget it clicked into the id it expects to be playing.
#[cfg(feature = "debug")]
const ROW_PREFIX: &str = "Scenario Row: ";

/// Every scenario row currently in the list, by name, in list order.
#[cfg(feature = "debug")]
fn scenario_row_names(world: &mut World) -> Vec<String> {
    let mut q = world.query::<&Name>();
    let mut names: Vec<String> = q
        .iter(world)
        .map(|n| n.as_str().to_string())
        .filter(|n| n.starts_with(ROW_PREFIX))
        .collect();
    names.sort();
    names
}

/// The scenario row the picker currently has selected, if any.
///
/// [`Selected`] is the picker's own record - `select_scenario_row` inserts it
/// on the clicked row and removes it from every other - so this reads the
/// widget tree rather than the walk's intent. Other widgets carry `Selected`
/// too, hence the row filter.
#[cfg(feature = "debug")]
fn selected_row_name(world: &mut World) -> Option<String> {
    world
        .query_filtered::<&Name, With<Selected>>()
        .iter(world)
        .map(|name| name.as_str().to_string())
        .find(|name| name.starts_with(ROW_PREFIX))
}

/// Open the picker, click each row once, measure the panes after each, report
/// whether the split held, and launch the last selection.
#[cfg(feature = "debug")]
fn scenarios_autopilot(world: &mut World, _elapsed: f32, _frame: u32) {
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
        // Loading is ATOMIC: `GameStates::Playing` is entered while the world
        // is still HELD behind the loading panel, and `CurrentScenario` is only
        // the delivered scenario once the gate lets go. So the delivery claim
        // waits on the GATE - a condition the loader owns - and never on a
        // frame count, which would only be a guess at how long a software
        // renderer takes to warm the scene's art.
        match world.get_resource::<ScenarioLoadGate>().copied() {
            Some(ScenarioLoadGate::Failed) => panic!(
                "the picker's Play never delivered `{}`: the scenario load FAILED, and a                  failed load holds the world for good",
                state.target_row.as_deref().unwrap_or("<no target row>")
            ),
            Some(gate) if gate.is_held() => {
                world.insert_resource(state);
                return;
            }
            _ => {}
        }
        if state.launched && !state.finished {
            assert_the_clicked_row_started(world, &state);
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
        // resolve the same node a second time.
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
        // across selections that never happened.
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
            // is the one thing that string exists to prevent.
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
    // The picker's OWN opening selection, read on the first frame the rows
    // exist and before the walk has touched any of them.
    if state.default_row.is_none() && state.visited.is_empty() && !rows.is_empty() {
        state.default_row = selected_row_name(world);
        if let Some(default_row) = &state.default_row {
            info!("probe: the picker opened on {default_row}");
        }
    }
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
            RowPlacement::PastTheFold(delta) => {
                // Aim the wheel first: it goes to the pane under the pointer,
                // and after a selection the pointer is still sitting on the row
                // that was clicked - which may itself have scrolled away.
                if let Some(list) = ui_node_rect(world, "Scenarios List") {
                    move_cursor(list.center())(world);
                }
                scroll_pixels(delta)(world);
                info!("probe: scrolled {delta:.1}px to reach {name}");
                // One driven frame for the wheel to land: the scroll is applied
                // in `Update` and layout moves the rows in `PostUpdate`, so
                // measuring the gap again on the very next frame would read the
                // PRE-scroll rects and scroll a second time, overshooting.
                state.wait = 1;
                settle_or_skip(&mut state, name, "it stayed past the fold after scrolling");
            }
            RowPlacement::Unreached(reason) => settle_or_skip(&mut state, name, reason),
        },
        None => {
            if !state.reported {
                report(world, &state);
                state.reported = true;
            }
            play_a_non_default_row(world, &mut state);
        }
    }

    world.insert_resource(state);
}

/// Give a row that is not clickable THIS frame another driven frame, or give up
/// on it once the budget is spent.
///
/// Shared by both not-yet-clickable cases, because a scroll needs the same
/// bounded patience a rebuild does: a row the wheel cannot actually reach - a
/// list already at its end, a fold on the axis that does not scroll - would
/// otherwise be scrolled at for the rest of the run.
#[cfg(feature = "debug")]
fn settle_or_skip(state: &mut ScenariosAutopilot, name: String, reason: &'static str) {
    let waited = match &state.settling {
        Some((settling, waited)) if *settling == name => waited + 1,
        _ => 1,
    };
    if waited >= ROW_SETTLE_FRAMES {
        warn!(
            "scenarios: row `{name}` unreachable after {ROW_SETTLE_FRAMES} driven attempts \
             ({reason}); skipping it"
        );
        state.visited.push(name.clone());
        state.skipped.push((name, reason));
        state.settling = None;
    } else {
        state.settling = Some((name, waited));
    }
}

/// Leave a NON-DEFAULT row selected and play it.
///
/// The picker default-selects its first row
/// (`refresh_scenarios_list`, `crates/nova_menu/src/scenarios.rs:219`), so a
/// run that simply pressed Play would start that row whether the walk's clicks
/// reached the loader or not - it would prove the button, not the picker. The
/// delivery claim is read against the row the picker opened on: the walk plays
/// a DIFFERENT one, and the scenario that comes up has to be that one.
///
/// Preference goes to the LAST row measured, which is already selected and
/// whose selecting click `assert_selection_landed` has proven landed. When that
/// one happens to be the default, an earlier row is re-selected with a fresh
/// pointer click.
#[cfg(feature = "debug")]
fn play_a_non_default_row(world: &mut World, state: &mut ScenariosAutopilot) {
    let harnessed = std::env::var_os("NOVA_AUTOPILOT").is_some();

    if state.target_row.is_none() {
        let default_row = state.default_row.clone();
        let target = state
            .measured
            .iter()
            .rev()
            .map(|(name, _, _)| name.clone())
            .find(|name| Some(name.as_str()) != default_row.as_deref());
        let Some(target) = target else {
            let detail = format!(
                "every row the walk measured is the one the picker opened on \
                 ({default_row:?}), so playing one would prove nothing about the \
                 click reaching the loader"
            );
            if harnessed {
                panic!("scenarios delivery: NO non-default row to play - {detail}");
            }
            warn!("scenarios delivery: no non-default row to play - {detail}");
            click_play(world, state, "the current selection");
            return;
        };
        info!("probe: playing {target}, which the picker did not open on");
        state.target_row = Some(target);
        state.settling = None;
    }
    let target = state
        .target_row
        .clone()
        .expect("the target row was just chosen");

    // Select it with a REAL click, unless the walk already left it selected.
    if selected_row_name(world).as_deref() != Some(target.as_str()) {
        match row_placement(world, &target) {
            RowPlacement::OnScreen(centre) => {
                click_at(centre, MouseButton::Left)(world);
                state.pending_release = true;
                state.settling = None;
            }
            RowPlacement::PastTheFold(delta) => {
                if let Some(list) = ui_node_rect(world, "Scenarios List") {
                    move_cursor(list.center())(world);
                }
                scroll_pixels(delta)(world);
                info!("probe: scrolled {delta:.1}px to reach {target} for launch");
                state.wait = 1;
                insist(
                    world,
                    state,
                    &target,
                    "it stayed past the fold after scrolling",
                );
            }
            RowPlacement::Unreached(reason) => insist(world, state, &target, reason),
        }
        return;
    }

    nova_probe::probe_marker(
        world,
        "outcome: the played row is not the picker's default",
        serde_json::json!({ "row": target, "opened_on": state.default_row }),
    );
    click_play(world, state, &target);
}

/// Insist on the delivery target.
///
/// Unlike [`settle_or_skip`], giving up is not an option here: WHICH row is
/// played is the claim, so a target that never becomes clickable is a failed
/// run rather than thinner coverage. Interactively it falls back to playing
/// whatever is selected, so an eyeballing run still ends.
#[cfg(feature = "debug")]
fn insist(world: &mut World, state: &mut ScenariosAutopilot, target: &str, reason: &'static str) {
    let waited = match &state.settling {
        Some((settling, waited)) if settling == target => waited + 1,
        _ => 1,
    };
    if waited < ROW_SETTLE_FRAMES {
        state.settling = Some((target.to_string(), waited));
        return;
    }
    state.settling = None;
    let detail = format!(
        "`{target}` never became selectable after {ROW_SETTLE_FRAMES} driven attempts \
         ({reason}), so the run cannot say which row it played"
    );
    if std::env::var_os("NOVA_AUTOPILOT").is_some() {
        panic!("scenarios delivery: {detail}");
    }
    warn!("scenarios delivery: {detail}");
    state.target_row = None;
    click_play(world, state, "the current selection");
}

/// Press the details pane's Play button - the picker's whole point - with the
/// pointer, at its own screen position.
#[cfg(feature = "debug")]
fn click_play(world: &mut World, state: &mut ScenariosAutopilot, what: &str) {
    if let Some(centre) = ui_node_centre(world, "Scenario Play Button") {
        click_at(centre, MouseButton::Left)(world);
        state.pending_release = true;
        info!("probe: clicked Play on {what}");
    } else {
        warn!("probe: no Play button in the details pane to finish on");
    }
    state.launched = true;
}

/// The scenario that came up is EXACTLY the row the pointer selected and
/// played.
///
/// [`CurrentScenario`] is the LOADER's record of what is live - written by the
/// scenario loader and read by the HUD, the outcome chain and the pause menu
/// alike - so this compares the picker's promise against the loader's fact
/// rather than against the menu's own selection state. It is read on the frame
/// the atomic load gate releases, which is the first complete frame of the
/// scene. Fatal under `NOVA_AUTOPILOT` and a warning interactively, exactly as
/// [`assert_selection_landed`] is: a human at the controls of an unharnessed
/// run can select something else.
#[cfg(feature = "debug")]
fn assert_the_clicked_row_started(world: &mut World, state: &ScenariosAutopilot) {
    let Some(row) = state.target_row.clone() else {
        warn!("scenarios delivery: no target row was recorded, so nothing to check");
        return;
    };
    let expected = row.trim_start_matches(ROW_PREFIX).to_string();
    let live = world
        .resource::<CurrentScenario>()
        .0
        .as_ref()
        .map(|scenario| scenario.id.clone());
    if live.as_deref() == Some(expected.as_str()) {
        info!("probe: the picker started {expected}");
        nova_probe::probe_marker(
            world,
            "outcome: the picker starts the row the player clicked",
            serde_json::json!({ "row": row, "scenario": expected }),
        );
        return;
    }
    let detail = format!(
        "the pointer selected and played `{row}`, and `{live:?}` is what started - \
         the picker delivered a different scenario than the selected row"
    );
    if std::env::var_os("NOVA_AUTOPILOT").is_some() {
        panic!("scenarios delivery: {detail}");
    }
    warn!("scenarios delivery: {detail}");
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
/// regression this example exists to catch pass CI green.
///
/// A thin run fails the same way. The property is a COMPARISON across
/// selections, so one measurement makes both spreads `max - min` over a single
/// value - zero - and prints `HELD` having proven nothing. Two is the floor,
/// and falling under it names the rows that were skipped, since a skip is the
/// only way to get there.
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
