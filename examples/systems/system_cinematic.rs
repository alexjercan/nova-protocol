//! system_cinematic: a SCENE end to end - the beat chain that takes the screen,
//! the prompt and the title card it puts on the real HUD, the pause that
//! freezes the way out, the skip the player actually presses, and the two
//! endings the scenario hears, in order.
//!
//! The unit tests around `CinematicActionConfig` drive `NovaEventWorld`
//! directly, so every one of them starts from a scene that is already running
//! and a clock they move by hand. Four things only the live chain has are
//! therefore unmeasured, and this range is each of them:
//!
//! 1. **The gate.** `skip_cinematic_on_request` is chained into the scenario
//!    pulse behind `scenario_is_live && Unpaused && scenario_has_settled`
//!    (`loader/clock.rs`). The range opens the pause overlay, presses the skip
//!    against it, and reads the NEGATIVE: the scene is still playing when the
//!    overlay comes down.
//! 2. **The run condition.** `a_player_can_answer` asks for a real
//!    `ButtonInput` pair and a real `InputBindings`. The skip here is
//!    `press_action(CINEMATIC_SKIP_ACTION)` - the registry decides which key
//!    that is, exactly as it does for a player.
//! 3. **The order.** `OnCinematicSkipped` is fired before
//!    `OnCinematicFinished` so the catch-up handler runs before the handler
//!    that gives the camera back. Through the real `GameEventQueue` that is a
//!    claim about DISPATCH order, not about the two `commands.fire` calls, so
//!    the finish handler proves it: its first action copies the skip handler's
//!    latch, and a finish that ran first would copy a zero.
//! 4. **The HUD.** The prompt and the title card are resources nova_scenario
//!    writes every frame and nova_hud draws. The range reads them off the live
//!    app: the prompt names the skip action while the skippable scene runs and
//!    is quiet under the unskippable one, and the card comes down on its own
//!    hold with no handler taking it.
//!
//! The scenario is built in Rust and loaded with `LoadScenario`, so the
//! compiler catches grammar changes and no shipped story content is reachable
//! from here.
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//! ```text
//! NOVA_AUTOPILOT=1 cargo run --example system_cinematic --features debug
//! # look for: `nova harness: reached Playing`,
//! #           `cinematic probe: the scene owns the screen`,
//! #           `cinematic probe: the paused scene kept its beats`,
//! #           `cinematic probe: the skip was announced before the finish`,
//! #           `autopilot: cycle complete, no panic`
//! ```

use bevy::prelude::*;
use clap::Parser;
use nova_authoring::scenario_helpers::prelude::*;
use nova_probe::fixtures;
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "system_cinematic")]
#[command(version = "1.0.0")]
#[command(about = "Cinematic showcase: a scene takes the screen, a pause freezes the way out, the player skips, and both endings arrive in order. Autopilot-only correctness range", long_about = None)]
struct Cli;

/// The scenario this example builds and loads. Shared with the smoke-test
/// assertion so both agree on what "loaded" means.
const SCENARIO_ID: &str = "cinematic_showcase";

/// The skippable scene: the one the player walks out of.
const OVERTURE: &str = "overture";

/// The unskippable scene the overture's finish handler starts: a scene that
/// runs to its end, offers nothing, and reports a finish and no skip.
const DEBRIEF: &str = "debrief";

/// The card the overture posts, and the string the range looks for on the HUD.
const TITLE_LOCATION: &str = "MERIDIAN, OUTER HOLD";

/// How long that card holds, on the scenario's own pause-frozen clock.
const CARD_SECONDS: f32 = 6.0;

/// When the overture's SECOND beat would run.
///
/// Shorter than [`CARD_SECONDS`], which is what makes the `late_beat` latch a
/// real proof: the range's last wait is the card expiring, so by the time it
/// reads the latch the scenario clock has passed the moment the unplayed beat
/// was due.
const LATE_BEAT_AFTER: f64 = 5.0;

/// The gap between the debrief's two beats.
const DEBRIEF_GAP: f64 = 1.0;

/// How many frames a negative verdict gets to be wrong in. A key that went
/// nowhere raises no event, so the only honest wait is a frame count.
#[cfg(feature = "debug")]
const REFUSAL_FRAMES: u32 = 12;

/// The first step's name, so `loop_from` restarts the cycle at the scene
/// reload without repeating the string.
#[cfg(feature = "debug")]
const LOAD_STEP: &str = "load the cinematic showcase";

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();
    // NovaMenuPlugin explicitly: ESC-to-pause lives in nova_menu, and
    // `with_game_plugins` turns the menu plugin off. The pause overlay is what
    // this range presses the skip against, so the plugin has to be here; it is
    // built to stand alone in slim apps, and the boot-into-MainMenu handoff it
    // normally owns is not taken (this run goes Loading -> Playing).
    let mut app = AppBuilder::new()
        .with_game_plugins((custom_plugin, NovaMenuPlugin))
        .build();

    #[cfg(feature = "debug")]
    {
        app.add_plugins(nova_screenshot(
            nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
                // OnStart seeds `act` and clears `skip_seen`, so waiting for
                // both is also the gate a looped scene reload needs: the old
                // cycle's variables outlive the load replacing them
                // (ScenarioLoaded fires BEFORE OnStart runs).
                .step(LOAD_STEP)
                .enter(GameStates::Loading)
                .until(and(
                    scenario_variable_is("act", 1.0),
                    scenario_variable_is("skip_seen", 0.0),
                ))
                .deadline(30.0)
                .add()
                // The scene is live again: close the reload interval so a frame
                // capture excludes it. A no-op on the first cycle.
                .step("close the reload interval")
                .on_enter(nova_probe::capture_reload_end)
                .add()
                // The scene's first beat wrote its latch, the prompt names the
                // skip action and the card is on the HUD - three separate
                // resources, all of them written by the live sync rather than
                // by the test.
                .step("the scene takes the screen")
                .until(and(
                    and(
                        scenario_variable_is("beat_one", 1.0),
                        resource_where::<CinematicPrompt>(|prompt| {
                            prompt.skip_action.as_deref() == Some(CINEMATIC_SKIP_ACTION)
                        }),
                    ),
                    resource_where::<CinematicTitle>(|title| {
                        title
                            .card
                            .as_ref()
                            .is_some_and(|card| card.location == TITLE_LOCATION)
                    }),
                ))
                .deadline(30.0)
                .add()
                .step("report the live scene")
                .on_enter(report_live_scene)
                .add()
                // The gate. ESC is not a registry action - it is the universal
                // back-out - so this one is a raw key.
                .step("open the pause overlay")
                .on_enter(press_key(KeyCode::Escape))
                .until(state_is(PauseStates::Paused))
                .deadline(15.0)
                .add()
                .step("release escape")
                .on_enter(release_key(KeyCode::Escape))
                .add()
                // The press lands on the frame the driver runs, which is the
                // only frame its `just_pressed` edge exists. The gated system
                // does not run on that frame, so the edge is gone for good -
                // exactly what a player mashing the key behind the menu gets.
                .step("press the skip against the pause")
                .on_enter(press_action(CINEMATIC_SKIP_ACTION))
                .until(frames(REFUSAL_FRAMES))
                .add()
                .step("report the frozen skip")
                .on_enter(report_frozen_skip)
                .add()
                .step("release the skip")
                .on_enter(release_action(CINEMATIC_SKIP_ACTION))
                .add()
                .step("close the pause overlay")
                .on_enter(press_key(KeyCode::Escape))
                .until(state_is(PauseStates::Unpaused))
                .deadline(15.0)
                .add()
                .step("release escape again")
                .on_enter(release_key(KeyCode::Escape))
                .add()
                // The same press, on an unpaused scene: this one is answered.
                .step("leave the scene")
                .on_enter(press_action(CINEMATIC_SKIP_ACTION))
                .until(scenario_variable_is("finish_seen", 1.0))
                .deadline(20.0)
                .add()
                .step("release the skip again")
                .on_enter(release_action(CINEMATIC_SKIP_ACTION))
                .add()
                .step("report the skip")
                .on_enter(report_skip)
                .add()
                // The finish handler started the second scene, which is
                // authored unskippable: its beats are running and the prompt
                // has gone quiet.
                //
                // `debrief_beat` is a COUNTER, not one of this scenario's
                // one-way latches, so the wait is "has left zero" and not
                // `scenario_variable_is(.., 1.0)`. Beat 1 holds for
                // `DEBRIEF_GAP` of scenario clock, which is a couple of frames
                // on the software renderer - fewer than the two steps between
                // the finish and this wait can spend, and the equality then
                // waits forever on a beat the scene is already past. Beat 2
                // witnesses the takeover exactly as well.
                .step("the unskippable scene takes over")
                .until(and(
                    resource_where::<NovaEventWorld>(|events| {
                        matches!(
                            events.get_variable("debrief_beat"),
                            Some(VariableLiteral::Number(beat)) if *beat >= 1.0
                        )
                    }),
                    resource_where::<CinematicPrompt>(|prompt| prompt.skip_action.is_none()),
                ))
                .deadline(20.0)
                .add()
                .step("report the unskippable scene")
                .on_enter(report_unskippable_scene)
                .add()
                .step("the scene runs out")
                .until(scenario_variable_is("debrief_finish", 1.0))
                .deadline(20.0)
                .add()
                .step("report the patient ending")
                .on_enter(report_patient_ending)
                .add()
                // Nothing takes the card down: the last wait is its own hold
                // running out on the scenario clock, which is also what puts
                // the run past the moment the skipped beat was due.
                .step("the card comes down on its own")
                .until(resource_where::<CinematicTitle>(|title| {
                    title.card.is_none()
                }))
                .deadline(40.0)
                .add()
                .step("report the whole scene")
                .on_enter(report_scene)
                .add()
                // Enrolled in capture looping: when a frame capture outlives
                // the script, the scene reloads and the scenes replay, so the
                // capture measures ACTIVITY.
                .loop_from(LOAD_STEP)
                .on_loop(reload_the_showcase),
        ));
        app.add_plugins(assert_scenario_loaded(SCENARIO_ID));
        // Every latch this scenario keeps is one-way by design. A reload
        // re-seeds them, which the checker forgets at teardown.
        app.add_plugins(nova_probe::NovaProbePlugin::default().monotonic([
            "beat_one",
            "skip_seen",
            "finish_seen",
            "finish_saw_skip",
            "late_beat",
            "debrief_beat",
            "debrief_finish",
            "debrief_skip",
        ]));
    }

    app.run()
}

fn custom_plugin(app: &mut App) {
    // On assets-Loaded, not on Playing: `assert_scenario_loaded` checks the
    // load happened by OnEnter(Playing), and loading in that same schedule
    // is an unordered race against the check.
    app.add_systems(OnEnter(GameAssetsStates::Loaded), setup_showcase);
}

fn setup_showcase(mut commands: Commands, game_assets: Res<GameAssets>) {
    commands.trigger(LoadScenario(showcase(&game_assets)));
}

/// The showcase scenario: two scenes and the four handlers that hear them end.
fn showcase(game_assets: &GameAssets) -> ScenarioConfig {
    let mut start_actions = vec![
        set_number("act", 1.0),
        set_number("beat_one", 0.0),
        set_number("late_beat", 0.0),
        set_number("skip_seen", 0.0),
        set_number("finish_seen", 0.0),
        set_number("finish_saw_skip", 0.0),
        set_number("debrief_beat", 0.0),
        set_number("debrief_finish", 0.0),
        set_number("debrief_skip", 0.0),
    ];
    // Something for the scene to be ABOUT, so a looped capture of this range is
    // a shot rather than an empty cubemap.
    start_actions.push(spawn_object(fixtures::asteroid(
        game_assets,
        "hold_rock",
        "Outer Hold Rock",
        Meters3::new(0.0, 0.0, -120.0),
        Meters(20.0),
        None,
    )));
    // The showcase lights itself: the engine spawns no light, so a scenario
    // that authors none renders black.
    start_actions.extend(ThreePointRig::around("showcase", Meters3::ZERO, 5.0).actions());
    // The scene itself. Beat one says it ran and posts the card; beat two is
    // the one the skip has to cancel.
    start_actions.push(cinematic(
        OVERTURE,
        true,
        vec![
            step(
                0.0,
                vec![
                    set_number("beat_one", 1.0),
                    title(
                        ScreenCornerConfig::TopLeft,
                        TITLE_LOCATION,
                        "END OF SHIFT",
                        "412 aboard, and due under way.",
                        CARD_SECONDS,
                    ),
                ],
            ),
            step(LATE_BEAT_AFTER, vec![set_number("late_beat", 1.0)]),
        ],
    ));

    let events = vec![
        ScenarioEventConfig {
            label: None,
            name: EventConfig::OnStart,
            once: false,
            filters: vec![],
            actions: start_actions,
        },
        // The catch-up handler: everything the unplayed beats would have left
        // behind. Here that is one latch, which the finish handler then reads.
        ScenarioEventConfig {
            label: None,
            name: EventConfig::OnCinematicSkipped,
            once: true,
            filters: vec![scene(OVERTURE)],
            actions: vec![set_number("skip_seen", 1.0)],
        },
        // The restore handler, and the ORDER proof: its first action copies the
        // skip handler's latch. A finish dispatched ahead of the skip would
        // copy the seeded zero, and `report_skip` would fail naming it.
        ScenarioEventConfig {
            label: None,
            name: EventConfig::OnCinematicFinished,
            once: true,
            filters: vec![scene(OVERTURE)],
            actions: vec![
                set_variable("finish_saw_skip", variable("skip_seen")),
                set_number("finish_seen", 1.0),
                cinematic(
                    DEBRIEF,
                    false,
                    vec![
                        step(0.0, vec![set_number("debrief_beat", 1.0)]),
                        step(DEBRIEF_GAP, vec![set_number("debrief_beat", 2.0)]),
                    ],
                ),
            ],
        },
        // The unskippable scene's two ends. The skip latch must stay at zero:
        // an unskippable scene is not offered and does not answer.
        ScenarioEventConfig {
            label: None,
            name: EventConfig::OnCinematicSkipped,
            once: true,
            filters: vec![scene(DEBRIEF)],
            actions: vec![set_number("debrief_skip", 1.0)],
        },
        ScenarioEventConfig {
            label: None,
            name: EventConfig::OnCinematicFinished,
            once: true,
            filters: vec![scene(DEBRIEF)],
            actions: vec![set_number("debrief_finish", 1.0)],
        },
    ];

    ScenarioConfig {
        description: "Two scenes, a pause, a skip and the endings both report.".to_string(),
        events,
        ..ScenarioConfig::new(
            SCENARIO_ID.to_string(),
            "Cinematic Showcase".to_string(),
            game_assets.cubemap.clone().into(),
        )
    }
}

/// Restart the cycle on an autopilot loop: mark the reload interval and
/// re-trigger the SAME showcase scenario. The step after the loop point closes
/// the interval once the fixture has re-seeded itself.
///
/// Silently does nothing before the loader has inserted the asset resources -
/// a loop cannot happen that early, but reading them unguarded would panic.
#[cfg(feature = "debug")]
fn reload_the_showcase(world: &mut World) {
    let Some(game_assets) = world.get_resource::<GameAssets>().cloned() else {
        return;
    };
    nova_probe::capture_reload_begin(world);
    world.trigger(LoadScenario(showcase(&game_assets)));
}

/// Read a Number variable from the live event world, or panic with context.
#[cfg(feature = "debug")]
fn number_variable(world: &World, key: &str) -> f64 {
    match world.resource::<NovaEventWorld>().get_variable(key) {
        Some(VariableLiteral::Number(value)) => *value,
        other => panic!("cinematic probe: variable {key} should be a number, got {other:?}"),
    }
}

/// The scene is up, and it reached BOTH HUD surfaces it owes the player: the
/// way out, and the card that says where this is.
#[cfg(feature = "debug")]
fn report_live_scene(world: &mut World) {
    assert_eq!(
        world.resource::<CinematicPrompt>().skip_action.as_deref(),
        Some(CINEMATIC_SKIP_ACTION),
        "a live skippable scene must offer the skip action on the HUD"
    );
    let card = world
        .resource::<CinematicTitle>()
        .card
        .clone()
        .expect("cinematic probe: the scene's title card must reach the HUD");
    assert_eq!(card.location, TITLE_LOCATION, "the card is the scene's own");
    assert!(
        (card.seconds - CARD_SECONDS).abs() < f32::EPSILON,
        "the card carries its authored hold, not a HUD default: {}",
        card.seconds
    );
    nova_probe::probe_marker(
        world,
        "outcome: a live scene offers the skip on the real hud",
        serde_json::json!({}),
    );
    info!("cinematic probe: the scene owns the screen");
}

/// The gate, read as the negative it is: the skip was pressed against the pause
/// overlay and the scene is still playing.
#[cfg(feature = "debug")]
fn report_frozen_skip(world: &mut World) {
    assert_eq!(
        *world.resource::<State<PauseStates>>().get(),
        PauseStates::Paused,
        "the verdict only means anything while the overlay is up"
    );
    assert_eq!(
        number_variable(world, "skip_seen"),
        0.0,
        "a paused scenario must not take the skip: the driver chain is gated on \
         Unpaused, so the press has nothing to reach"
    );
    assert_eq!(
        number_variable(world, "finish_seen"),
        0.0,
        "and no ending may be reported behind the menu"
    );
    assert_eq!(
        world.resource::<CinematicPrompt>().skip_action.as_deref(),
        Some(CINEMATIC_SKIP_ACTION),
        "the scene is still the live one, so the prompt still names its way out"
    );
    nova_probe::probe_marker(
        world,
        "outcome: a paused scene refuses the skip",
        serde_json::json!({}),
    );
    info!("cinematic probe: the paused scene kept its beats");
}

/// The skip that was answered: both endings arrived, and the catch-up handler
/// ran before the one that gives the camera back.
#[cfg(feature = "debug")]
fn report_skip(world: &mut World) {
    assert_eq!(
        number_variable(world, "skip_seen"),
        1.0,
        "the bound key must leave the scene through the real bindings registry"
    );
    assert_eq!(
        number_variable(world, "finish_saw_skip"),
        1.0,
        "OnCinematicSkipped must be DISPATCHED before OnCinematicFinished: the \
         finish handler copies the skip handler's latch, so a zero here means \
         the restore ran before the catch-up"
    );
    nova_probe::probe_marker(
        world,
        "outcome: the skip is announced before the finish",
        serde_json::json!({}),
    );
    info!("cinematic probe: the skip was announced before the finish");
}

/// A scene authored unskippable offers nothing and answers nothing.
#[cfg(feature = "debug")]
fn report_unskippable_scene(world: &mut World) {
    assert!(
        world.resource::<CinematicPrompt>().skip_action.is_none(),
        "an unskippable scene must not put a way out on the HUD"
    );
    assert_eq!(
        number_variable(world, "debrief_skip"),
        0.0,
        "and it must not report a skip"
    );
    nova_probe::probe_marker(
        world,
        "outcome: an unskippable scene offers no way out",
        serde_json::json!({}),
    );
    info!("cinematic probe: the unskippable scene offers nothing");
}

/// The patient path through the live chain: every beat played, one finish, no
/// skip.
#[cfg(feature = "debug")]
fn report_patient_ending(world: &mut World) {
    assert_eq!(
        number_variable(world, "debrief_beat"),
        2.0,
        "a scene nobody left plays every beat"
    );
    assert_eq!(
        number_variable(world, "debrief_finish"),
        1.0,
        "and reports its finish"
    );
    assert_eq!(
        number_variable(world, "debrief_skip"),
        0.0,
        "nobody asked to leave, so no skip is reported"
    );
    nova_probe::probe_marker(
        world,
        "outcome: a scene that runs out reports its finish",
        serde_json::json!({}),
    );
    info!("cinematic probe: the unskippable scene ran out and reported once");
}

/// The script's last beat. The card's hold has run out, which means the
/// scenario clock is past the moment the skipped beat was due - so the latch it
/// would have written is a real reading, not a run that simply ended early.
#[cfg(feature = "debug")]
fn report_scene(world: &mut World) {
    assert_eq!(
        number_variable(world, "late_beat"),
        0.0,
        "a skip CANCELS the cursor: the unplayed beats must not run at speed on \
         the way out, and this one was due {LATE_BEAT_AFTER}s into a scene the \
         clock has now outlived"
    );
    nova_probe::probe_marker(
        world,
        "outcome: the skip cancels the beats it did not play",
        serde_json::json!({}),
    );
    assert!(
        world.resource::<CinematicTitle>().card.is_none(),
        "nothing takes the card down, so a scene that was skipped leaves a card \
         that finishes its own hold and goes"
    );
    nova_probe::probe_marker(
        world,
        "outcome: the title card comes down on its own",
        serde_json::json!({}),
    );
    info!("cinematic probe: the scene, its skip and its card all closed out");
}
