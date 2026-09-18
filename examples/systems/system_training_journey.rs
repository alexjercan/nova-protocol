//! system_training_journey: what a player learns survives the app.
//!
//! The composed path nothing else covers. A fresh profile is offered Basic
//! Training, the corner opens the handbook, opening a lesson marks it READ,
//! its practice range launches through the real hand-off, winning that range
//! marks exactly the lessons that name it PROVEN, and a second app on the same
//! profile comes up with all of it on screen. The focused tests underneath
//! this each own one link - `training_store`'s round trip and its completion
//! writer, `nova_menu`'s live-tree tests for the corner and the rows - and none
//! of them can see the JOIN, because the join only exists across two app
//! lifetimes with a real scenario outcome in the middle.
//!
//! Three apps, in one process, in order:
//!
//!   1. FRESH. An empty profile. The corner offers Basic Training; `Open
//!      lessons` puts the player on the first lesson; a row click reads
//!      `combat_radar`; its Practice button launches `drill_gunnery`; Target 1
//!      dies through the production damage path and the range is WON. The four
//!      lessons that name `drill_gunnery` go to Completed and the file is
//!      written.
//!   2. RELAUNCH. A second app on the same root. The record loads, the rows
//!      draw their badges, and the offer is still standing because nobody
//!      answered it. `Not now` answers it, and the SETTINGS file takes that -
//!      the dismissal is a switch the player threw, not something they learned.
//!   3. AGAIN. A third app on the same root. The corner is gone, the record is
//!      not, the menu card's `Lessons` row still opens the handbook, and the
//!      range a player has already been proven on still launches from it.
//!
//! Which lessons `drill_gunnery` proves is AUTHORED (`proven_by` in
//! `lessons.rs`), so the run reads them out of the live catalog rather than
//! listing them here: the claim is "exactly the lessons that name the range",
//! and a hand-copied list would stop being that the first time an author
//! changes one.
//!
//! The stores are the point, so this range cannot use the ones every other
//! scripted range gets: `from_env` makes both INERT under `NOVA_AUTOPILOT`
//! (`harness_env_active`), which is right everywhere else and is exactly what a
//! persistence range cannot use. Both are pinned to `ReadWrite` on a TEMPORARY
//! root of this process's own, which is what keeps the developer's real profile
//! out of it.
//!
//! Headless, because three `App`s live and die in one process and a winit event
//! loop does not. The handbook is a reconciler - the list is rebuilt whenever
//! the record changes - so every click re-resolves its widget by `Name`.
//!
//! The run recorder is the last app's: a second recorder on one path TRUNCATES
//! the first one's timeline. The two earlier apps write `timeline-fresh.jsonl`
//! and `timeline-relaunch.jsonl` beside it, so every marker this range emits is
//! still on disk under the run.
//!
//! Run (no display needed):
//! ```text
//! NOVA_AUTOPILOT=1 cargo run --example system_training_journey --features debug
//! # look for: `training journey: PASS ...` once per phase.
//! ```

#[cfg(feature = "debug")]
use bevy::{prelude::*, window::PrimaryWindow};
use clap::Parser;
#[cfg(feature = "debug")]
use nova_protocol::prelude::*;
#[cfg(feature = "debug")]
use nova_training::prelude::{LessonStatus, TrainingCatalog, TrainingProgress};

#[derive(Parser)]
#[command(name = "system_training_journey")]
#[command(version = "1.0.0")]
#[command(
    about = "Walk a fresh profile through the handbook, a won practice range, and two restarts. Autopilot-only correctness range",
    long_about = None
)]
struct Cli;

#[cfg(not(feature = "debug"))]
fn main() {
    let _ = Cli::parse();
    eprintln!("system_training_journey drives the app through the debug-only autopilot gestures;");
    eprintln!("run it with --features debug");
}

/// The lesson the run opens and practises. Chosen because it carries BOTH a
/// practice range and a `proven_by` that names it, so one row covers the read
/// path and the proven path.
#[cfg(feature = "debug")]
const LESSON: &str = "combat_radar";

/// The range that lesson's Practice button launches.
#[cfg(feature = "debug")]
const RANGE: &str = "drill_gunnery";

/// The scenario object whose death wins that range. Authored in
/// `base_content/scenarios/tutorial/range.rs` as `target_id(1)`, which is
/// `pub(crate)` there; the drill's win handler filters on this exact id.
#[cfg(feature = "debug")]
const TARGET: &str = "target_1";

/// Widgets this range drives, by `Name`. Every one is resolved fresh on the
/// frame it is clicked - the handbook is rebuilt whenever the record changes.
#[cfg(feature = "debug")]
const CORNER: &str = "Training Prompt";
#[cfg(feature = "debug")]
const CORNER_LESSONS: &str = "Training Prompt Lessons";
#[cfg(feature = "debug")]
const CORNER_DISMISS: &str = "Training Prompt Dismiss";
#[cfg(feature = "debug")]
const PANEL: &str = "Training Panel";
#[cfg(feature = "debug")]
const MENU_LESSONS: &str = "Lessons Button";
#[cfg(feature = "debug")]
const PRACTICE: &str = "Lesson Practice Button";

#[cfg(feature = "debug")]
fn lesson_row() -> String {
    format!("Lesson Row: {LESSON}")
}

#[cfg(feature = "debug")]
fn lesson_badge() -> String {
    format!("Lesson Status: {LESSON}")
}

#[cfg(feature = "debug")]
fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();

    let profile = temp_root();
    let _ = std::fs::remove_dir_all(&profile);

    finished("the fresh profile", fresh_app(&profile).run());
    finished("the relaunch", relaunch_app(&profile).run());
    let exit = again_app(&profile).run();

    let _ = std::fs::remove_dir_all(&profile);
    exit
}

/// A phase that did not exit cleanly is a FAILED range, not a quieter one: the
/// phases after it read the profile the failed one was supposed to write, and
/// would otherwise report on an empty one.
#[cfg(feature = "debug")]
fn finished(phase: &str, exit: bevy::app::AppExit) {
    assert!(
        exit.is_success(),
        "{phase} did not exit cleanly ({exit:?}); the phases after it read what it wrote"
    );
}

/// A profile of this run's own, under the system temp dir.
///
/// Per-process, so two copies of this range on one box do not read each other's
/// files, and never the player's own.
#[cfg(feature = "debug")]
fn temp_root() -> std::path::PathBuf {
    std::env::temp_dir().join(format!("nova_training_journey_{}", std::process::id()))
}

/// The SHIPPED app on an explicit profile, held at the main menu.
///
/// The two `with_*_store` calls are the only difference, and they are the whole
/// point: see the module docs on why `from_env` cannot be used here.
#[cfg(feature = "debug")]
fn shipped_app_on(root: &std::path::Path) -> App {
    let mut app = AppBuilder::headless()
        .with_settings_store(SettingsStorePlugin {
            access: SettingsStoreAccess::ReadWrite,
            root: Some(root.to_path_buf()),
        })
        .with_training_store(TrainingProgressPlugin {
            access: TrainingStoreAccess::ReadWrite,
        })
        .build();
    // Headless has no winit, so the UI needs a window to lay out in. TALL: the
    // handbook lists every lesson it holds, and a row below the pane's fold has
    // no box a pointer can be put in.
    app.world_mut().spawn((
        Window {
            resolution: (1280, 1600).into(),
            ..default()
        },
        PrimaryWindow,
    ));
    app
}

/// A marker sink for a phase that is not the last one.
///
/// [`ProbeTimeline::create`] TRUNCATES, so three apps pointed at the run's one
/// timeline would leave only the last one's markers on disk. Each earlier phase
/// gets a sibling file in the same run directory instead. Off the probe there is
/// no path to derive, and the recorder stays inert.
#[cfg(feature = "debug")]
fn phase_timeline(phase: &str) -> nova_probe::prelude::RunRecorderPlugin {
    let recorder = nova_probe::prelude::nova_timeline();
    match nova_probe::prelude::probe_param(nova_probe::prelude::TIMELINE_PARAM) {
        Some(path) => {
            let path = std::path::PathBuf::from(path);
            recorder.out(path.with_file_name(format!("timeline-{phase}.jsonl")))
        }
        None => recorder,
    }
}

// ---------------------------------------------------------------- phase 1 --

/// FRESH: offer, handbook, read, practice, win, write.
#[cfg(feature = "debug")]
fn fresh_app(root: &std::path::Path) -> App {
    let mut app = shipped_app_on(root);
    app.add_plugins(phase_timeline("fresh"));
    let owned = root.to_path_buf();
    let written = root.to_path_buf();
    let watched = root.to_path_buf();
    app.add_plugins(
        nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
            .step("training journey: reach the main menu on an empty profile")
            .enter(GameStates::Loading)
            .until(state_is(GameStates::MainMenu))
            .deadline(STEP_DEADLINE_SECS)
            .add()
            .step("training journey: the profile is writable and ours")
            .on_enter(move |world: &mut World| assert_the_profile_is_ours(world, &owned))
            .add()
            .step("training journey: a fresh profile is offered Basic Training")
            .until(ui_node_present(CORNER))
            .diagnose(ui_node_diagnosis(CORNER.to_string()))
            .deadline(BEAT_DEADLINE_SECS)
            .add()
            .step("training journey: record the offer")
            .on_enter(assert_the_offer_stands)
            .add()
            .click_named(
                "training journey: open the handbook from the corner",
                CORNER_LESSONS,
                ui_node_present(PANEL),
                BEAT_DEADLINE_SECS,
            )
            .step("training journey: the corner opened it ON a lesson")
            .on_enter(assert_the_corner_opened_the_first_lesson)
            .add()
            .click_named(
                "training journey: open the lesson",
                &lesson_row(),
                ui_node_present(lesson_badge()),
                BEAT_DEADLINE_SECS,
            )
            .step("training journey: opening a lesson reads it and proves nothing")
            .on_enter(assert_the_lesson_is_read_not_proven)
            .add()
            .click_named(
                "training journey: launch the lesson's practice range",
                PRACTICE,
                state_is(GameStates::Playing),
                BEAT_DEADLINE_SECS,
            )
            .step("training journey: the practice range is the one the lesson names")
            .until(scenario_is(RANGE))
            .diagnose(|world: &World| format!("the live scenario is {:?}", live_scenario(world)))
            .deadline(STEP_DEADLINE_SECS)
            .add()
            .step("training journey: record the launch")
            .on_enter(assert_the_practice_range_launched)
            .add()
            // The range's objects are spawned by its own `OnStart` handler,
            // a few frames past the state change the launch waited on.
            .step("training journey: the firing line is on the range")
            .until(the_object_is_up(TARGET))
            .diagnose(|world: &World| format!("the range holds {:?}", scenario_objects(world)))
            .deadline(STEP_DEADLINE_SECS)
            .add()
            // The range is won the way a player wins it - the authored
            // `OnDestroyed` handler on Target 1 - not by writing an outcome.
            .step("training journey: destroy Target 1")
            .on_enter(kill_object(TARGET))
            .until(the_range_is_won())
            .diagnose(|world: &World| format!("the outcome is {:?}", live_outcome(world)))
            .deadline(STEP_DEADLINE_SECS)
            .add()
            .step("training journey: a won range proves only the lessons that name it")
            .on_enter(assert_only_the_named_lessons_are_proven)
            .add()
            .step("training journey: the proven record reaches the file")
            .until(the_file_carries_the_record(watched))
            .diagnose(move |_: &World| format!("the file holds {:?}", stored(&written)))
            .deadline(BEAT_DEADLINE_SECS)
            .add()
            .step("training journey: inspect the written record")
            .on_enter(assert_the_file_was_written)
            .add(),
    );
    app
}

/// The gate itself: this app writes, and it writes THERE.
///
/// Both halves matter. An inert store would leave the later phases reading an
/// empty profile and reporting defaults as if they were saved; a store on the
/// default root would be the developer's own.
#[cfg(feature = "debug")]
fn assert_the_profile_is_ours(world: &mut World, root: &std::path::Path) {
    let access = *world.resource::<TrainingStoreAccess>();
    assert_eq!(
        access,
        TrainingStoreAccess::ReadWrite,
        "this range has to WRITE; `TrainingProgressPlugin::default` would have \
         made the store inert under NOVA_AUTOPILOT"
    );
    let live = world.resource::<SettingsStoreRoot>().clone();
    assert_eq!(
        live,
        SettingsStoreRoot(Some(root.to_path_buf())),
        "the profile must be this run's own temporary root, never the player's"
    );
    assert_eq!(
        load_training(&live),
        None,
        "the phase that WRITES the record must start from an empty profile, or \
         it proves nothing about having written it"
    );
    assert_eq!(
        *world.resource::<TrainingProgress>(),
        TrainingProgress::default(),
        "an empty profile is a player who has learned nothing yet"
    );
    nova_probe::probe_marker(
        world,
        "outcome: the run writes an isolated training profile",
        serde_json::json!({ "access": format!("{access:?}"), "root": root }),
    );
}

/// The offer a first-time player gets, and the two facts that make it one: the
/// setting has never been answered, and nothing has been learned.
#[cfg(feature = "debug")]
fn assert_the_offer_stands(world: &mut World) {
    assert_eq!(
        *world.resource::<TrainingPromptSetting>(),
        TrainingPromptSetting::Shown,
        "a profile nobody has answered the offer on must still be offered it"
    );
    let (viewed, completed, total) = counts(world);
    assert_eq!(
        (viewed, completed),
        (0, 0),
        "a fresh profile has read nothing and been proven on nothing"
    );
    info!("training journey: PASS a fresh profile is offered Basic Training");
    nova_probe::probe_marker(
        world,
        "outcome: a fresh profile is offered Basic Training",
        serde_json::json!({ "lessons": total }),
    );
}

/// `Open lessons` is the one door that puts a player ON a lesson - the menu
/// card's row deliberately does not, because a default selection nobody chose
/// must not read as something they read.
#[cfg(feature = "debug")]
fn assert_the_corner_opened_the_first_lesson(world: &mut World) {
    let first = world
        .resource::<TrainingCatalog>()
        .first_id()
        .cloned()
        .expect("the shipped catalog has lessons in it");
    let progress = world.resource::<TrainingProgress>();
    assert_eq!(
        progress.status(&first),
        LessonStatus::Viewed,
        "`Open lessons` puts the player on the first lesson, which reads it"
    );
    assert_eq!(
        progress.completed().count(),
        0,
        "opening the handbook proves nothing"
    );
    info!("training journey: PASS the corner opened the handbook on '{first}'");
    nova_probe::probe_marker(
        world,
        "outcome: the corner opens the handbook on the first lesson",
        serde_json::json!({ "lesson": first }),
    );
}

/// The Viewed/Completed line, on the one path that marks Viewed.
#[cfg(feature = "debug")]
fn assert_the_lesson_is_read_not_proven(world: &mut World) {
    let progress = world.resource::<TrainingProgress>();
    assert_eq!(
        progress.status(LESSON),
        LessonStatus::Viewed,
        "clicking a row reads the lesson"
    );
    assert_eq!(
        progress.completed().count(),
        0,
        "reading a lesson never claims mastery of it"
    );
    info!("training journey: PASS '{LESSON}' is read and not proven");
    nova_probe::probe_marker(
        world,
        "outcome: opening a lesson reads it and proves nothing",
        serde_json::json!({ "lesson": LESSON }),
    );
}

/// The Practice button goes through the SAME New Game hand-off the Scenarios
/// picker uses; what this records is that it arrived at the authored range.
#[cfg(feature = "debug")]
fn assert_the_practice_range_launched(world: &mut World) {
    assert_eq!(
        live_scenario(world).as_deref(),
        Some(RANGE),
        "the Practice button must launch the range the lesson names"
    );
    assert_eq!(
        world.resource::<TrainingProgress>().completed().count(),
        0,
        "launching practice is not finishing it"
    );
    info!("training journey: PASS the handbook launched '{RANGE}'");
    nova_probe::probe_marker(
        world,
        "outcome: the handbook launches the lesson's own practice range",
        serde_json::json!({ "lesson": LESSON, "scenario": RANGE }),
    );
}

/// EXACTLY the lessons that name the range, read out of the live catalog.
///
/// Both directions: every lesson whose `proven_by` holds the range is Completed,
/// and no lesson that does not name it is. A one-directional check would pass on
/// a writer that completed the whole catalog.
#[cfg(feature = "debug")]
fn assert_only_the_named_lessons_are_proven(world: &mut World) {
    // Sorted, because the record is a set and the catalog is an authored
    // order: the claim is WHICH lessons, not which order they are in.
    let mut expected: Vec<String> = world
        .resource::<TrainingCatalog>()
        .lessons()
        .iter()
        .filter(|lesson| lesson.proven_by.iter().any(|id| id == RANGE))
        .map(|lesson| lesson.id.clone())
        .collect();
    expected.sort();
    assert!(
        !expected.is_empty(),
        "no shipped lesson names '{RANGE}' in `proven_by`, so this range would \
         prove nothing and pass on an empty claim"
    );
    let progress = world.resource::<TrainingProgress>();
    let mut proven: Vec<String> = progress.completed().cloned().collect();
    proven.sort();
    assert_eq!(
        proven, expected,
        "a won range completes exactly the lessons that name it"
    );
    for lesson in &expected {
        assert_eq!(
            progress.status(lesson),
            LessonStatus::Completed,
            "'{lesson}' names '{RANGE}' and the range was won"
        );
    }
    info!("training journey: PASS '{RANGE}' proved {proven:?}");
    nova_probe::probe_marker(
        world,
        "outcome: a won range proves only the lessons that name it",
        serde_json::json!({ "scenario": RANGE, "proven": proven }),
    );
}

/// State what the file now holds. The waiting was done by the beat before this
/// one; what is left is to say it, on the record.
#[cfg(feature = "debug")]
fn assert_the_file_was_written(world: &mut World) {
    let root = world.resource::<SettingsStoreRoot>().clone();
    let saved = load_training(&root).expect("the beat before this one waited for the file");
    let live = PersistedTraining::from_progress(world.resource::<TrainingProgress>());
    assert_eq!(
        saved, live,
        "the file must carry the record the run actually holds"
    );
    assert!(
        saved.viewed.iter().any(|id| id == LESSON),
        "the lesson the run opened must be in the file"
    );
    assert!(
        !saved.completed.is_empty(),
        "the lessons the won range proved must be in the file"
    );
    info!("training journey: PASS the run wrote {saved:?}");
    nova_probe::probe_marker(
        world,
        "outcome: the proven record reaches the file",
        serde_json::json!({ "viewed": saved.viewed, "completed": saved.completed }),
    );
}

// ---------------------------------------------------------------- phase 2 --

/// RELAUNCH: the record loads, the badges draw, and the standing offer is
/// answered - into the SETTINGS file, not this one.
#[cfg(feature = "debug")]
fn relaunch_app(root: &std::path::Path) -> App {
    let mut app = shipped_app_on(root);
    app.add_plugins(phase_timeline("relaunch"));
    let watched = root.to_path_buf();
    let shown = root.to_path_buf();
    app.add_plugins(
        nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
            .step("training journey: the relaunch reaches the main menu")
            .enter(GameStates::Loading)
            .until(state_is(GameStates::MainMenu))
            .deadline(STEP_DEADLINE_SECS)
            .add()
            .step("training journey: the relaunch loaded what the last run learned")
            .on_enter(assert_the_relaunch_loaded_the_record)
            .add()
            .click_named(
                "training journey: open the handbook from the menu card",
                MENU_LESSONS,
                ui_node_present(PANEL),
                BEAT_DEADLINE_SECS,
            )
            .step("training journey: the loaded record reaches the screen")
            .until(ui_node_present(lesson_badge()))
            .diagnose(ui_node_diagnosis(lesson_badge()))
            .deadline(BEAT_DEADLINE_SECS)
            .add()
            .step("training journey: record the drawn badges")
            .on_enter(assert_the_badges_draw)
            .add()
            // Back out, because the modal covers the corner the next beat
            // clicks - the handbook is the screen in front, by design.
            .click_named(
                "training journey: close the handbook",
                "Training Back Button",
                ui_node_present(CORNER),
                BEAT_DEADLINE_SECS,
            )
            .step("training journey: the offer nobody answered is still standing")
            .on_enter(assert_the_offer_survived_the_restart)
            .add()
            .click_named(
                "training journey: answer the offer with Not now",
                CORNER_DISMISS,
                the_prompt_is_hidden(),
                BEAT_DEADLINE_SECS,
            )
            .step("training journey: the answer reaches the settings file")
            .until(the_settings_file_hides_the_prompt(watched))
            .diagnose(move |_: &World| {
                format!(
                    "the settings file holds {:?}",
                    load_settings(&SettingsStoreRoot(Some(shown.clone())))
                        .map(|saved| saved.training_prompt)
                )
            })
            .deadline(BEAT_DEADLINE_SECS)
            .add()
            .step("training journey: record the answered offer")
            .on_enter(assert_the_answer_went_to_the_settings)
            .add(),
    );
    app
}

/// The load, before anything on screen: the resource the handbook draws from
/// has to be the last run's record, not a default.
#[cfg(feature = "debug")]
fn assert_the_relaunch_loaded_the_record(world: &mut World) {
    let root = world.resource::<SettingsStoreRoot>().clone();
    let saved = load_training(&root).expect("the phase before this one wrote the file");
    let live = PersistedTraining::from_progress(world.resource::<TrainingProgress>());
    assert_eq!(
        live, saved,
        "the relaunch must come up ON the saved record, not on a default one"
    );
    let progress = world.resource::<TrainingProgress>();
    assert_eq!(
        progress.status(LESSON),
        LessonStatus::Completed,
        "'{LESSON}' was proven last run and must still be"
    );
    info!("training journey: PASS the relaunch loaded {saved:?}");
    nova_probe::probe_marker(
        world,
        "outcome: a relaunch loads what the last run proved",
        serde_json::json!({ "viewed": saved.viewed, "completed": saved.completed }),
    );
}

/// The loaded record on SCREEN. The beat before this waited for the badge to
/// lay out; this says what it is - a `done` badge is what `Completed` draws,
/// and a record that loaded into a resource nothing renders would still have
/// passed the load check above.
#[cfg(feature = "debug")]
fn assert_the_badges_draw(world: &mut World) {
    let drawn: Vec<String> = world
        .resource::<TrainingProgress>()
        .completed()
        .filter(|id| ui_node_rect(world, &format!("Lesson Status: {id}")).is_some())
        .cloned()
        .collect();
    let proven: Vec<String> = world
        .resource::<TrainingProgress>()
        .completed()
        .cloned()
        .collect();
    assert_eq!(
        drawn, proven,
        "every lesson the loaded record proved must carry its badge on the list"
    );
    info!("training journey: PASS the loaded record draws {drawn:?}");
    nova_probe::probe_marker(
        world,
        "outcome: the loaded record reaches the screen",
        serde_json::json!({ "badges": drawn }),
    );
}

/// Reading is not answering. Two runs, a won range and a handbook full of
/// badges later, the corner is still there because nobody pressed either of the
/// buttons that answers it.
#[cfg(feature = "debug")]
fn assert_the_offer_survived_the_restart(world: &mut World) {
    assert_eq!(
        *world.resource::<TrainingPromptSetting>(),
        TrainingPromptSetting::Shown,
        "`Open lessons` and a practice range do not answer the offer"
    );
    assert!(
        ui_node_rect(world, CORNER).is_some(),
        "the unanswered offer must be back on screen with the handbook closed"
    );
    nova_probe::probe_marker(
        world,
        "outcome: reading the handbook does not answer the offer",
        serde_json::json!({ "prompt": "Shown" }),
    );
}

/// The split the whole design rests on: the dismissal is a SWITCH and lives in
/// `settings.ron`; what the player learned is a RECORD and lives beside it. A
/// player who deletes their settings to fix their audio must not lose their
/// progress.
#[cfg(feature = "debug")]
fn assert_the_answer_went_to_the_settings(world: &mut World) {
    let root = world.resource::<SettingsStoreRoot>().clone();
    let settings = load_settings(&root).expect("the beat before this one waited for the file");
    assert_eq!(
        settings.training_prompt,
        TrainingPromptSetting::Hidden,
        "`Not now` is a setting, and the settings file is where it goes"
    );
    let record = load_training(&root).expect("the record is a file of its own");
    assert!(
        !record.completed.is_empty(),
        "answering the offer must not touch what the player was proven on"
    );
    info!("training journey: PASS the dismissal went to the settings, the record stayed");
    nova_probe::probe_marker(
        world,
        "outcome: the dismissal is a setting and the record is not",
        serde_json::json!({
            "training_prompt": format!("{:?}", settings.training_prompt),
            "completed": record.completed,
        }),
    );
}

// ---------------------------------------------------------------- phase 3 --

/// AGAIN: the answered offer is gone, the record is not, and the handbook is
/// still reachable without the corner.
#[cfg(feature = "debug")]
fn again_app(root: &std::path::Path) -> App {
    let mut app = shipped_app_on(root);
    // The LAST phase owns the run's timeline and the invariant stream - see the
    // module docs.
    app.add_plugins(nova_probe::NovaProbePlugin::default());
    app.add_plugins(
        nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
            .step("training journey: the third launch reaches the main menu")
            .enter(GameStates::Loading)
            .until(state_is(GameStates::MainMenu))
            .deadline(STEP_DEADLINE_SECS)
            .add()
            // The corner is spawned or not on the frame the menu builds, so
            // this waits for the MENU CARD, then reads the corner's absence.
            .step("training journey: the menu card is up")
            .until(ui_node_present(MENU_LESSONS))
            .diagnose(ui_node_diagnosis(MENU_LESSONS.to_string()))
            .deadline(BEAT_DEADLINE_SECS)
            .add()
            .step("training journey: the answered offer is gone and the record is not")
            .on_enter(assert_the_answered_offer_stays_answered)
            .add()
            .click_named(
                "training journey: the handbook is still reachable",
                MENU_LESSONS,
                ui_node_present(PANEL),
                BEAT_DEADLINE_SECS,
            )
            .step("training journey: the handbook still draws the record")
            .until(ui_node_present(lesson_badge()))
            .diagnose(ui_node_diagnosis(lesson_badge()))
            .deadline(BEAT_DEADLINE_SECS)
            .add()
            .step("training journey: record the standing handbook")
            .on_enter(assert_the_handbook_is_still_the_door)
            .add()
            // A proven lesson is not a spent one. The range stays open, and
            // flying it again must not take the proof away.
            .click_named(
                "training journey: reopen the proven lesson",
                &lesson_row(),
                ui_node_present(PRACTICE),
                BEAT_DEADLINE_SECS,
            )
            .click_named(
                "training journey: practise it again",
                PRACTICE,
                state_is(GameStates::Playing),
                BEAT_DEADLINE_SECS,
            )
            .step("training journey: the range reopens on a lesson already proven")
            .until(scenario_is(RANGE))
            .diagnose(|world: &World| format!("the live scenario is {:?}", live_scenario(world)))
            .deadline(STEP_DEADLINE_SECS)
            .add()
            .step("training journey: record the reopened range")
            .on_enter(assert_a_proven_lesson_can_be_practised_again)
            .add(),
    );
    app
}

/// Practice is a BUTTON, not a reward: the range a player has already been
/// proven on is still there, and flying it again leaves the proof alone.
#[cfg(feature = "debug")]
fn assert_a_proven_lesson_can_be_practised_again(world: &mut World) {
    assert_eq!(
        live_scenario(world).as_deref(),
        Some(RANGE),
        "a proven lesson keeps its practice range"
    );
    let progress = world.resource::<TrainingProgress>();
    assert_eq!(
        progress.status(LESSON),
        LessonStatus::Completed,
        "relaunching a range must not take back what it proved"
    );
    info!("training journey: PASS '{LESSON}' is proven and still practisable");
    nova_probe::probe_marker(
        world,
        "outcome: a proven lesson can be practised again",
        serde_json::json!({ "lesson": LESSON, "scenario": RANGE }),
    );
}

/// Both halves of the restart: the switch the player threw stayed thrown, and
/// the record beside it is untouched by that.
#[cfg(feature = "debug")]
fn assert_the_answered_offer_stays_answered(world: &mut World) {
    assert_eq!(
        *world.resource::<TrainingPromptSetting>(),
        TrainingPromptSetting::Hidden,
        "the dismissal survives the restart"
    );
    assert!(
        ui_node_rect(world, CORNER).is_none(),
        "a dismissed offer must not be drawn again"
    );
    let progress = world.resource::<TrainingProgress>();
    assert_eq!(
        progress.status(LESSON),
        LessonStatus::Completed,
        "two restarts on, '{LESSON}' is still proven"
    );
    info!("training journey: PASS the answered offer stayed answered");
    nova_probe::probe_marker(
        world,
        "outcome: the answered offer is gone on the next launch",
        serde_json::json!({ "completed": progress.completed().collect::<Vec<_>>() }),
    );
}

/// The product contract's other half: the corner can be switched off, and the
/// handbook must stay reachable when it is.
#[cfg(feature = "debug")]
fn assert_the_handbook_is_still_the_door(world: &mut World) {
    assert!(
        ui_node_rect(world, PANEL).is_some(),
        "the menu card's `Lessons` row opens the handbook with no corner in sight"
    );
    let (viewed, completed, total) = counts(world);
    info!("training journey: PASS the handbook is reachable without the corner");
    nova_probe::probe_marker(
        world,
        "outcome: the handbook is reachable without the corner",
        serde_json::json!({ "viewed": viewed, "completed": completed, "lessons": total }),
    );
}

// ------------------------------------------------------------------ parts --

/// `(viewed, completed, total)` counted against the live catalog.
#[cfg(feature = "debug")]
fn counts(world: &World) -> (usize, usize, usize) {
    world
        .resource::<TrainingProgress>()
        .counts(world.resource::<TrainingCatalog>())
}

/// The scenario the game is playing right now.
#[cfg(feature = "debug")]
fn live_scenario(world: &World) -> Option<String> {
    world
        .get_resource::<CurrentScenario>()
        .and_then(|current| current.0.as_ref())
        .map(|config| config.id.clone())
}

/// The outcome the live scenario has posted, if any.
#[cfg(feature = "debug")]
fn live_outcome(world: &World) -> Option<ScenarioOutcomeKind> {
    world
        .get_resource::<CurrentOutcome>()
        .and_then(|current| current.0.as_ref())
        .map(|config| config.outcome)
}

/// Advance once the loaded scenario is `id`.
#[cfg(feature = "debug")]
fn scenario_is(id: &'static str) -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(move |world: &World| live_scenario(world).as_deref() == Some(id))
}

/// Every scenario object on the range right now, by authored id.
#[cfg(feature = "debug")]
fn scenario_objects(world: &World) -> Vec<String> {
    let Some(mut query) = world.try_query_filtered::<&EntityId, With<ScenarioScopedMarker>>()
    else {
        return Vec::new();
    };
    query.iter(world).map(|live| live.0.clone()).collect()
}

/// Advance once the scenario object `id` has spawned.
#[cfg(feature = "debug")]
fn the_object_is_up(
    id: &'static str,
) -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(move |world: &World| scenario_objects(world).iter().any(|live| live == id))
}

/// Advance once the range has posted a Victory.
#[cfg(feature = "debug")]
fn the_range_is_won() -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(|world: &World| live_outcome(world) == Some(ScenarioOutcomeKind::Victory))
}

/// Advance once the live prompt setting reads Hidden.
#[cfg(feature = "debug")]
fn the_prompt_is_hidden() -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(|world: &World| {
        world
            .get_resource::<TrainingPromptSetting>()
            .is_some_and(|prompt| *prompt == TrainingPromptSetting::Hidden)
    })
}

/// What is in the training file right now, or `None` when nothing is.
#[cfg(feature = "debug")]
fn stored(root: &std::path::Path) -> Option<PersistedTraining> {
    load_training(&SettingsStoreRoot(Some(root.to_path_buf())))
}

/// Advance once the file on disk carries what the run holds.
///
/// A CONDITION on the artifact, not a frame count: the write is a
/// change-detection save, and which frame it lands on is the store's business.
#[cfg(feature = "debug")]
fn the_file_carries_the_record(
    root: std::path::PathBuf,
) -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(move |world: &World| {
        stored(&root).is_some_and(|saved| {
            saved == PersistedTraining::from_progress(world.resource::<TrainingProgress>())
                && !saved.completed.is_empty()
        })
    })
}

/// Advance once the SETTINGS file carries the answered offer. Debounced, like
/// every other settings write.
#[cfg(feature = "debug")]
fn the_settings_file_hides_the_prompt(
    root: std::path::PathBuf,
) -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(move |_: &World| {
        load_settings(&SettingsStoreRoot(Some(root.clone())))
            .is_some_and(|saved| saved.training_prompt == TrainingPromptSetting::Hidden)
    })
}

/// Kill the scenario object `id` with an overkill through the production damage
/// entry point, aimed at the node that actually carries the `Health`.
///
/// The drill's targets are bare hulls whose health sits below the root, so the
/// helper walks to the nearest bearer rather than assuming the root carries it.
#[cfg(feature = "debug")]
fn kill_object(id: &'static str) -> impl Fn(&mut World) + Send + Sync + 'static {
    move |world: &mut World| {
        let root = {
            let mut query =
                world.query_filtered::<(Entity, &EntityId), With<ScenarioScopedMarker>>();
            query
                .iter(world)
                .find(|(_, live)| live.0 == id)
                .map(|(entity, _)| entity)
        };
        let root =
            root.unwrap_or_else(|| panic!("training journey: no scenario object '{id}' to kill"));
        let target = health_bearer(world, root).unwrap_or_else(|| {
            panic!("training journey: scenario object '{id}' carries no Health")
        });
        world.trigger(HealthApplyDamage {
            entity: target,
            source: None,
            amount: 1e6,
        });
    }
}

/// The nearest `Health`-carrying node at or beneath `entity`, depth first.
#[cfg(feature = "debug")]
fn health_bearer(world: &World, entity: Entity) -> Option<Entity> {
    if world.get::<Health>(entity).is_some() {
        return Some(entity);
    }
    let children: Vec<Entity> = world
        .get::<Children>(entity)
        .map(|children| children.iter().collect())
        .unwrap_or_default();
    children
        .into_iter()
        .find_map(|child| health_bearer(world, child))
}
