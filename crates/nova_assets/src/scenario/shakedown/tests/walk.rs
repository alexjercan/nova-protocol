//! The beat walk: a headless app running the real event pipeline with the
//! scenario's handlers registered exactly as `on_load_scenario` registers
//! them, driven through all five beats.

use nova_events::prelude::{EventHandler, GameEventsPlugin};
use nova_hud::prelude::HintEmphasis;

use super::*;

/// A headless app running the real event pipeline with the scenario's handlers
/// registered exactly as on_load_scenario registers them - the shared rig for
/// the beat-walk tests.
fn scripted_app() -> App {
    use avian3d::prelude::PhysicsPlugins;
    use bevy_rand::prelude::{EntropyPlugin, WyRand};

    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        TransformPlugin,
        AssetPlugin::default(),
        bevy::mesh::MeshPlugin,
        PhysicsPlugins::default(),
        EntropyPlugin::<WyRand>::default(),
    ));
    app.init_asset::<StandardMaterial>();
    app.add_plugins(GameEventsPlugin::<NovaEventWorld>::default());
    app.init_resource::<NovaEventWorld>();
    app.init_resource::<GameObjectives>();
    // Production inits this in the HUD plugin; the emphasis actions
    // write it through the same drain the walk exercises.
    app.init_resource::<HintEmphasis>();
    // The ships reference their sections by prototype id, so
    // `insert_spaceship_sections` needs the real catalog in `GameSections`
    // to resolve them (production loads it from the sections RON).
    app.insert_resource(GameSections(crate::sections::build_sections(
        &crate::sections::SectionMeshRefs::from_paths(),
    )));
    app.add_plugins(ScenarioObjectsPlugin { render: false });
    app.finish();

    let config = scenario();
    for event in &config.events {
        let mut handler = EventHandler::<NovaEventWorld>::from(event.name);
        for filter in &event.filters {
            handler.add_filter(filter.clone());
        }
        for action in &event.actions {
            handler.add_action(action.clone());
        }
        app.world_mut().spawn(handler);
    }
    app
}

/// Fire the OnStart the loader fires after registration, plus one
/// OnUpdate pulse (the loader's fire_on_update equivalent).
fn boot(app: &mut App) {
    use nova_events::prelude::*;
    app.world_mut()
        .commands()
        .fire::<OnStartEvent>(OnStartEventInfo);
    pulse(app);
}

/// Set the scenario clock the loader normally advances each frame, so the
/// opening conversation's `scenario_elapsed` gates fire in the headless rig.
fn set_clock(app: &mut App, secs: f64) {
    app.world_mut()
        .resource_mut::<NovaEventWorld>()
        .insert_variable(
            SCENARIO_ELAPSED_VAR.to_string(),
            VariableLiteral::Number(secs),
        );
}

/// Advance the scenario clock past the current beat gate and pulse, so the
/// pending `beat_setup` posts its objective. Since the pacing rework an
/// objective posts a beat AFTER its transition's comms line; beats now use
/// different gaps, so the walk jumps past the LONGEST (`REVEAL_GAP`) - which
/// clears every category - and keeps `scenario_elapsed` monotonic across beats.
fn settle_beat(app: &mut App) {
    let now = match app
        .world()
        .resource::<NovaEventWorld>()
        .get_variable(SCENARIO_ELAPSED_VAR)
    {
        Some(VariableLiteral::Number(n)) => *n,
        _ => 0.0,
    };
    set_clock(app, now + REVEAL_GAP + 1.0);
    pulse(app);
}

/// Walk boot -> beat 10 (the combat rehearsal), settling each beat gate so the
/// delayed objectives and lazy spawns post. The fight tests only need to REACH
/// the rehearsal; the end-to-end walk asserts each beat inline instead of using
/// this.
fn walk_to_rehearsal(app: &mut App) {
    boot(app);
    finish_opening(app);
    enter(app, ID_BEACON_1);
    settle_beat(app);
    enter(app, ID_BEACON_2);
    settle_beat(app);
    // The crate markers/objective are up now (setup_last == 3), so the
    // guarded pickups fire; the third pickup advances to beat 4.
    for crate_id in ["crate_1", "crate_2", "crate_3"] {
        enter(app, crate_id);
        pulse(app);
    }
    settle_beat(app);
    travel_lock(app, ID_BEACON_3);
    settle_beat(app);
    enter(app, ID_BEACON_3);
    settle_beat(app);
    enter(app, ID_BEACON_4);
    settle_beat(app);
    enter(app, ID_COAST_RING);
    settle_beat(app);
    orbit(app, ID_PLANETOID);
    settle_beat(app);
    exit(app, ID_COAST_RING);
    settle_beat(app);
}

/// Run the ~40s opening conversation to its hand-off: push the clock past
/// the last line and pulse until objective 1 posts and beacon 1 spawns.
fn finish_opening(app: &mut App) {
    set_clock(app, OPEN_5_AT + 1.0);
    // Each pulse advances the open_step counter by one line; five lines plus
    // the hand-off settle in a handful of pulses (the clock is already past
    // every gate, so they chain).
    for _ in 0..7 {
        pulse(app);
    }
}

/// One OnUpdate pulse + settle, the way the loader's fire_on_update
/// emits it while a scenario is live.
fn pulse(app: &mut App) {
    use nova_events::prelude::*;
    app.world_mut()
        .commands()
        .fire::<OnUpdateEvent>(OnUpdateEventInfo);
    app.update();
    app.update();
}

/// The player ship enters `area` (the physics half of this event is proven by
/// the salvage pipeline test in).
fn enter(app: &mut App, area: &str) {
    use nova_events::prelude::*;
    app.world_mut()
        .commands()
        .fire::<OnEnterEvent>(OnEnterEventInfo {
            id: area.to_string(),
            other_id: ID_PLAYER.to_string(),
            other_type_name: "spaceship".to_string(),
        });
    app.update();
    app.update();
}

/// The player has held an orbit around `well` (the orbit-hold
/// tracker's event; the tracker itself is tested in nova_scenario's
/// loader tests - here the script consumes the event).
fn orbit(app: &mut App, well: &str) {
    use nova_events::prelude::*;
    app.world_mut()
        .commands()
        .fire::<OnOrbitEvent>(OnOrbitEventInfo {
            id: well.to_string(),
            other_id: ID_PLAYER.to_string(),
            other_type_name: "spaceship".to_string(),
        });
    app.update();
    app.update();
}

fn destroy(app: &mut App, id: &str) {
    use nova_events::prelude::*;
    app.world_mut()
        .commands()
        .fire::<OnDestroyedEvent>(OnDestroyedEventInfo {
            id: id.to_string(),
            type_name: "spaceship".to_string(),
        });
    app.update();
    app.update();
}

/// The player left `area` (the area plugin's exit half).
fn exit(app: &mut App, area: &str) {
    use nova_events::prelude::*;
    app.world_mut()
        .commands()
        .fire::<OnExitEvent>(OnExitEventInfo {
            id: area.to_string(),
            other_id: ID_PLAYER.to_string(),
            other_type_name: "spaceship".to_string(),
        });
    app.update();
    app.update();
}

/// The player's TRAVEL lock landed on `id` (the loader's lock bridge -
/// tested in nova_scenario; here the script consumes the event). The
/// bridge ECHOES a held lock every few seconds, so firing this twice
/// for the same id models a stale held lock.
fn travel_lock(app: &mut App, id: &str) {
    use nova_events::prelude::*;
    app.world_mut()
        .commands()
        .fire::<OnTravelLockEvent>(OnTravelLockEventInfo {
            id: id.to_string(),
            other_id: ID_PLAYER.to_string(),
            other_type_name: "spaceship".to_string(),
        });
    app.update();
    app.update();
}

/// The player's COMBAT lock landed on `id`.
fn combat_lock(app: &mut App, id: &str) {
    use nova_events::prelude::*;
    app.world_mut()
        .commands()
        .fire::<OnCombatLockEvent>(OnCombatLockEventInfo {
            id: id.to_string(),
            other_id: ID_PLAYER.to_string(),
            other_type_name: "spaceship".to_string(),
        });
    app.update();
    app.update();
}

/// Walk ALL FIVE BEATS through the real event pipeline: the actual
/// handlers registered exactly as the loader registers them, real
/// spawn/despawn commands applied to a real World, beat transitions
/// driven by the same OnEnter/OnDestroyed/OnUpdate events production
/// fires. This test owns the SCRIPT: gating, counting, lazy spawns,
/// tally text, the main ending.
#[test]
fn the_five_beats_walk_end_to_end() {
    use nova_events::prelude::*;

    let mut app = scripted_app();

    let beat = |app: &App| -> f64 {
        match app
            .world()
            .resource::<NovaEventWorld>()
            .get_variable(VAR_BEAT)
        {
            Some(VariableLiteral::Number(n)) => *n,
            other => panic!("beat variable missing or non-numeric: {:?}", other),
        }
    };
    let has_objective = |app: &App, id: &str| -> bool {
        app.world()
            .resource::<GameObjectives>()
            .objectives
            .iter()
            .any(|objective| objective.id == id)
    };
    let objective_message = |app: &App, id: &str| -> String {
        app.world()
            .resource::<GameObjectives>()
            .objectives
            .iter()
            .find(|objective| objective.id == id)
            .map(|objective| objective.message.clone())
            .unwrap_or_default()
    };
    let entity_with_id = |app: &mut App, id: &str| -> Option<Entity> {
        let mut query = app.world_mut().query::<(Entity, &EntityId)>();
        query
            .iter(app.world())
            .find(|(_, entity_id)| entity_id.0 == id)
            .map(|(entity, _)| entity)
    };
    let marker_label = |app: &mut App, id: &str| -> Option<String> {
        let entity = entity_with_id(app, id)?;
        app.world()
            .get::<ObjectiveMarkerTarget>(entity)
            .map(|marker| marker.label.clone())
    };
    let goto_emphasized =
        |app: &App| -> bool { app.world().resource::<HintEmphasis>().contains("GOTO") };
    let radar_emphasized =
        |app: &App| -> bool { app.world().resource::<HintEmphasis>().contains("RADAR") };
    // The Lock capability on the player's REAL controller section (the
    // capability beat, - same pin shape as the training governor).
    let verb_granted = |app: &mut App, player: Entity, verb: FlightVerb| -> bool {
        let mut q_controllers = app
            .world_mut()
            .query_filtered::<(&ChildOf, Option<&WithheldVerbs>), With<ControllerSectionMarker>>();
        q_controllers
            .iter(app.world())
            .find(|(ChildOf(parent), _)| *parent == player)
            .map(|(_, withheld)| withheld.is_none_or(|w| w.granted(verb)))
            .expect("the player ship has a controller section")
    };

    // Boot: OnStart is what the loader fires after registration.
    boot(&mut app);

    // The opening conversation runs first: at boot the captain is briefing, so
    // beat 1 is set but objective 1 and beacon 1 are not up yet.
    assert_eq!(beat(&app), 1.0);
    assert!(
        !has_objective(&app, OBJ_B1),
        "objective 1 waits for the opening conversation to finish"
    );
    assert!(
        entity_with_id(&mut app, ID_BEACON_1).is_none(),
        "beacon 1 spawns only after the briefing"
    );
    assert!(
        app.world()
            .resource::<GameObjectives>()
            .objectives
            .is_empty(),
        "the objectives panel stays empty during the opening conversation \
         (owner pacing pass 20260722-092421)"
    );
    assert!(
        entity_with_id(&mut app, ID_PLAYER).is_some(),
        "the player ship spawned"
    );

    // Run the ~40s briefing to its hand-off.
    finish_opening(&mut app);
    assert_eq!(
        marker_label(&mut app, ID_BEACON_1).as_deref(),
        Some("BEACON 1"),
        "the gold marker rides beacon 1 once the briefing ends"
    );
    assert!(has_objective(&app, OBJ_B1), "beat 1 objective is up");
    assert_eq!(
        app.world().resource::<GameObjectives>().objectives.len(),
        1,
        "only the real objective is up after hand-off - no holding line"
    );
    assert!(entity_with_id(&mut app, ID_BEACON_1).is_some());
    assert!(
        entity_with_id(&mut app, ID_BEACON_2).is_none(),
        "beacon 2 spawns lazily with its beat"
    );
    assert!(entity_with_id(&mut app, ID_PLANETOID).is_some());
    assert!(entity_with_id(&mut app, "crate_1").is_some());
    // The training governor is aboard for beat 1 (delivery guard for
    // the release assert below: the cap must exist to be removed).
    let player = entity_with_id(&mut app, ID_PLAYER).unwrap();
    assert!(
        app.world().get::<FlightSpeedCap>(player).is_some(),
        "the training governor caps the fresh ship"
    );
    assert!(
        !verb_granted(&mut app, player, FlightVerb::Lock),
        "the targeting computer starts OFFLINE (lock withheld; CTRL answers with the deny cue)"
    );
    assert!(
        !verb_granted(&mut app, player, FlightVerb::Orbit),
        "the orbit computer starts OFFLINE (a lit [O] during the coast reads as an ask)"
    );

    // Beat 1 -> 2: the transition completes beat 1 and calls the next mark;
    // beacon 2 and its objective post a beat later, once the line lands.
    enter(&mut app, ID_BEACON_1);
    assert_eq!(beat(&app), 2.0);
    assert!(!has_objective(&app, OBJ_B1), "beat 1 objective completed");
    // The governor releases with the transition (playtest round 2 finding 3).
    assert!(
        app.world().get::<FlightSpeedCap>(player).is_none(),
        "reaching beacon 1 releases the training governor"
    );
    assert!(
        !has_objective(&app, OBJ_B2),
        "beat 2 waits for the transition line to finish"
    );
    assert!(
        entity_with_id(&mut app, ID_BEACON_2).is_none(),
        "beacon 2 spawns with its objective, a beat later"
    );
    settle_beat(&mut app);
    assert!(has_objective(&app, OBJ_B2));
    assert!(entity_with_id(&mut app, ID_BEACON_2).is_some());
    // Marker hand-off: beacon 1 yields, the fresh beacon 2 carries it.
    assert_eq!(marker_label(&mut app, ID_BEACON_1), None);
    assert_eq!(
        marker_label(&mut app, ID_BEACON_2).as_deref(),
        Some("BEACON 2")
    );

    // A stray re-entry into beacon 1 must not re-fire the beat.
    enter(&mut app, ID_BEACON_1);
    assert_eq!(beat(&app), 2.0, "finished beats do not re-fire");

    // Beat 2 -> 3: the salvage objective and the crate markers post a beat
    // after the sweep call.
    enter(&mut app, ID_BEACON_2);
    assert_eq!(beat(&app), 3.0);
    assert!(
        !has_objective(&app, OBJ_B3),
        "the salvage objective waits for the sweep line"
    );
    settle_beat(&mut app);
    assert!(has_objective(&app, OBJ_B3));
    // All three crates carry the marker at once.
    assert_eq!(marker_label(&mut app, ID_BEACON_2), None);
    for crate_id in ["crate_1", "crate_2", "crate_3"] {
        assert_eq!(
            marker_label(&mut app, crate_id).as_deref(),
            Some("SALVAGE"),
            "{crate_id} is marked for the sweep"
        );
    }

    // Beat 3: the salvage sweep. Tally text follows the count via the
    // OnUpdate milestones; crates despawn on pickup.
    enter(&mut app, "crate_1");
    pulse(&mut app);
    assert!(
        entity_with_id(&mut app, "crate_1").is_none(),
        "picked-up crate despawns"
    );
    assert!(
        objective_message(&app, OBJ_B3).contains("1/3"),
        "tally shows 1/3, got: {}",
        objective_message(&app, OBJ_B3)
    );

    enter(&mut app, "crate_2");
    pulse(&mut app);
    assert!(objective_message(&app, OBJ_B3).contains("2/3"));

    enter(&mut app, "crate_3");
    pulse(&mut app);
    assert_eq!(beat(&app), 4.0, "all crates aboard advances the beat");
    assert!(
        !has_objective(&app, OBJ_B4),
        "the lock lesson waits for the transition line"
    );
    settle_beat(&mut app);
    assert!(has_objective(&app, OBJ_B4));
    assert!(
        entity_with_id(&mut app, ID_BEACON_3).is_some(),
        "beacon 3 appears with beat 4"
    );
    assert!(
        entity_with_id(&mut app, ID_PIRATE).is_none(),
        "beat 4 is pirate-free (playtest finding 4)"
    );
    // Beat 4 conveyance: the marker rides the lock target, RADAR (and
    // only RADAR) pulses, and the targeting computer is now online.
    assert_eq!(
        marker_label(&mut app, ID_BEACON_3).as_deref(),
        Some("BEACON 3")
    );
    assert!(radar_emphasized(&app), "beat 4 emphasizes RADAR");
    assert!(!goto_emphasized(&app), "GOTO waits for its own beat");
    assert!(
        verb_granted(&mut app, player, FlightVerb::Lock),
        "beat 4 brings the targeting computer ONLINE (delivery guard: withheld at boot)"
    );

    // Beat 4 -> 5: the white lock lands (the OnTravelLock bridge). RADAR
    // retires immediately with the lesson; the GOTO objective posts a beat
    // after the line.
    travel_lock(&mut app, ID_BEACON_3);
    assert_eq!(beat(&app), 5.0, "the lock lesson ticks on the lock");
    assert!(!radar_emphasized(&app), "RADAR retires with its lesson");
    assert!(
        !has_objective(&app, OBJ_B5),
        "the GOTO objective waits for the hand-off line"
    );
    settle_beat(&mut app);
    assert!(has_objective(&app, OBJ_B5));
    assert!(goto_emphasized(&app), "beat 5 emphasizes GOTO");

    // The bridge ECHOES held locks every few seconds: a stale re-fire
    // for beacon 3 during beat 5 must be a no-op (beat guards own
    // ordering; the echo exists so a lock HELD across a beat advance
    // can still complete a lesson, not to skip ones already done).
    travel_lock(&mut app, ID_BEACON_3);
    assert_eq!(
        beat(&app),
        5.0,
        "a stale lock echo does not re-fire the beat"
    );

    // Beat 5 -> 6: arrival at beacon 3; the waypoint run opens a beat after
    // the line.
    enter(&mut app, ID_BEACON_3);
    assert_eq!(beat(&app), 6.0);
    assert!(
        !has_objective(&app, OBJ_B6),
        "the waypoint waits for the line"
    );
    settle_beat(&mut app);
    assert!(has_objective(&app, OBJ_B6));
    assert!(
        entity_with_id(&mut app, ID_BEACON_4).is_some(),
        "beacon 4 spawns lazily with its beat"
    );
    assert_eq!(marker_label(&mut app, ID_BEACON_3), None);
    assert_eq!(
        marker_label(&mut app, ID_BEACON_4).as_deref(),
        Some("BEACON 4")
    );

    // Beat 6 -> 7: arrival at beacon 4; GOTO retires immediately, and the
    // coast ring and objective appear a beat after the line.
    enter(&mut app, ID_BEACON_4);
    assert_eq!(beat(&app), 7.0);
    assert!(!goto_emphasized(&app), "GOTO retires at the coast");
    assert!(!has_objective(&app, OBJ_B7), "the coast objective waits");
    assert!(
        entity_with_id(&mut app, ID_COAST_RING).is_none(),
        "the coast ring spawns with its objective, a beat later (never \
         early - the already-inside trap)"
    );
    settle_beat(&mut app);
    assert!(has_objective(&app, OBJ_B7));
    assert!(
        entity_with_id(&mut app, ID_COAST_RING).is_some(),
        "the coast ring spawns with its beat"
    );
    assert_eq!(
        marker_label(&mut app, ID_PLANETOID).as_deref(),
        Some("PLANETOID")
    );

    // Beat 7 -> 8: the drift crosses the ring; the orbit computer comes
    // online with its lesson, a beat after the line.
    assert!(
        !verb_granted(&mut app, player, FlightVerb::Orbit),
        "ORBIT stays withheld through the coast"
    );
    enter(&mut app, ID_COAST_RING);
    assert_eq!(beat(&app), 8.0);
    assert!(!has_objective(&app, OBJ_B8), "the orbit lesson waits");
    assert!(
        !verb_granted(&mut app, player, FlightVerb::Orbit),
        "ORBIT arrives with its lesson, not the bare transition"
    );
    settle_beat(&mut app);
    assert!(has_objective(&app, OBJ_B8));
    assert!(
        verb_granted(&mut app, player, FlightVerb::Orbit),
        "the ring grants ORBIT (delivery guard: withheld at boot)"
    );

    // Beat 8 -> 9: orbit held; the derelict and its marker appear at the
    // transition (so a fast break-away cannot outrun them), while the
    // break-away objective text posts a beat after the line.
    orbit(&mut app, ID_PLANETOID);
    assert_eq!(beat(&app), 9.0);
    assert!(
        entity_with_id(&mut app, ID_DERELICT).is_some(),
        "the derelict spawns at the transition"
    );
    assert_eq!(
        marker_label(&mut app, ID_DERELICT).as_deref(),
        Some("DERELICT"),
        "the marker hands off to the hulk at the transition"
    );
    assert!(
        !has_objective(&app, OBJ_B9),
        "the break-away objective waits for the line"
    );
    settle_beat(&mut app);
    assert!(has_objective(&app, OBJ_B9));
    assert!(
        entity_with_id(&mut app, ID_PIRATE).is_none(),
        "still no scavenger - the rehearsal comes first"
    );

    // Beat 9 -> 10: breaking away exits the ring; the combat-lock lesson
    // begins a beat after the line.
    exit(&mut app, ID_COAST_RING);
    assert_eq!(beat(&app), 10.0);
    assert!(!has_objective(&app, OBJ_B10), "the paint objective waits");
    assert!(
        !radar_emphasized(&app),
        "RADAR lights with the objective, not the bare transition"
    );
    settle_beat(&mut app);
    assert!(has_objective(&app, OBJ_B10));
    assert!(radar_emphasized(&app), "the rehearsal re-emphasizes RADAR");

    // An early COMBAT lock on the derelict during beat 9 would have
    // been a no-op; the echo covers the held lock once beat 10 arms -
    // modeled here by the beat-10 fire.
    combat_lock(&mut app, ID_DERELICT);
    assert_eq!(beat(&app), 11.0, "the red lock ticks the lesson");
    assert!(has_objective(&app, OBJ_B11));
    assert!(!radar_emphasized(&app), "RADAR retires with the red lock");

    // Beat 11 -> 12: the hulk dies; NOW the scavenger appears - the ship and
    // its marker with the warning line, the objective a beat later (pacing
    // pass).
    destroy(&mut app, ID_DERELICT);
    assert_eq!(beat(&app), 12.0);
    assert!(
        !has_objective(&app, OBJ_B12),
        "the scavenger objective waits a beat past the warning line"
    );
    assert!(
        entity_with_id(&mut app, ID_PIRATE).is_some(),
        "the scavenger spawns with the beat-12 reveal"
    );
    // Advance past the beat's deadline: the objective posts now.
    settle_beat(&mut app);
    assert!(has_objective(&app, OBJ_B12));
    assert_eq!(
        marker_label(&mut app, ID_PIRATE).as_deref(),
        Some("SCAVENGER")
    );

    // Beat 12 -> done: the scavenger driven off.
    destroy(&mut app, ID_PIRATE);
    assert_eq!(beat(&app), 13.0);
    assert!(!has_objective(&app, OBJ_B12));
    assert!(has_objective(&app, OBJ_DONE), "the run completes");
    // Free flight is marker-free: the done handler's defensive detach
    // (the rig's destroy event does not despawn the wreck, so the
    // detach action is what clears it here).
    assert_eq!(marker_label(&mut app, ID_PIRATE), None);
}

/// The pirate exists only from the beat-12 reveal on (playtest finding
/// 4 lineage), so an "early kill" is no longer reachable: a stray
/// OnDestroyed(pirate) DURING the rehearsal (e.g. a scenario edit
/// re-introducing an early spawn) must be a no-op, not a skipped
/// fight - the beat-12 guard on the kill handler owns that.
#[test]
fn pirate_destruction_only_counts_during_the_final_beat() {
    let mut app = scripted_app();
    walk_to_rehearsal(&mut app);

    // Beat 10 (the rehearsal): a pirate death event is out-of-script;
    // nothing moves.
    destroy(&mut app, ID_PIRATE);
    let objectives = &app.world().resource::<GameObjectives>().objectives;
    assert!(
        !objectives.iter().any(|objective| objective.id == OBJ_DONE),
        "a stray pirate death during the rehearsal must not complete the run"
    );

    // The real path still works: red lock, hulk down, scavenger down.
    combat_lock(&mut app, ID_DERELICT);
    destroy(&mut app, ID_DERELICT);
    destroy(&mut app, ID_PIRATE);
    let objectives = &app.world().resource::<GameObjectives>().objectives;
    assert!(
        objectives.iter().any(|objective| objective.id == OBJ_DONE),
        "the beat-12 kill completes the run, got: {:?}",
        objectives
    );
}

/// The out-of-order rehearsal (playtest 2026-07-13: the player shot
/// the hulk before ever locking it and the run soft-locked): killing
/// the derelict during ANY rehearsal beat skips straight to the fight
/// - lessons complete by demonstration, never dead-end.
#[test]
fn an_early_derelict_kill_skips_to_the_fight() {
    let mut app = scripted_app();
    walk_to_rehearsal(&mut app);
    // Beat 10 (the paint lesson is up, RADAR pulsing): the player
    // guns the hulk down WITHOUT locking it.
    assert!(
        app.world().resource::<HintEmphasis>().contains("RADAR"),
        "delivery guard: the rehearsal was mid-lesson"
    );
    destroy(&mut app, ID_DERELICT);

    // Pacing pass: the scavenger objective posts a beat AFTER the warning line,
    // so right after the kill the panel is empty; it fills once the deadline
    // passes.
    assert!(
        !app.world()
            .resource::<GameObjectives>()
            .objectives
            .iter()
            .any(|objective| objective.id == OBJ_B12),
        "the scavenger objective waits a beat past the warning line"
    );
    // Advance past the beat's deadline: the objective posts now.
    settle_beat(&mut app);

    let objectives = &app.world().resource::<GameObjectives>().objectives;
    assert!(
        objectives.iter().any(|objective| objective.id == OBJ_B12),
        "the kill skips to the fight, got: {:?}",
        objectives
    );
    assert!(
        !objectives
            .iter()
            .any(|objective| objective.id == OBJ_B10 || objective.id == OBJ_B11),
        "the skipped lessons are completed, not orphaned"
    );
    assert!(
        !app.world().resource::<HintEmphasis>().contains("RADAR"),
        "the skip retires the RADAR emphasis"
    );
    // The fight still ends the run.
    destroy(&mut app, ID_PIRATE);
    let objectives = &app.world().resource::<GameObjectives>().objectives;
    assert!(objectives.iter().any(|objective| objective.id == OBJ_DONE));
}

/// Per-beat pacing: an INSTRUCTION beat's objective lands MID-READ - after
/// `INSTRUCTION_GAP`, well before the full `REVEAL_GAP` a threat reveal would
/// wait. Beat 1 -> 2 ("swing your look around and find it" -> "Find Beacon 2")
/// is an instruction beat. This pins the split: if the gap were reverted to a
/// uniform `REVEAL_GAP`, advancing only `INSTRUCTION_GAP` past the transition
/// would NOT post the objective and the first assert would fire.
#[test]
fn instruction_objectives_land_mid_read_not_after_the_full_reveal_gap() {
    let has_obj = |app: &App, id: &str| -> bool {
        app.world()
            .resource::<GameObjectives>()
            .objectives
            .iter()
            .any(|o| o.id == id)
    };

    let mut app = scripted_app();
    boot(&mut app);
    finish_opening(&mut app);
    // The opening handoff parks the clock just past the last opening line.
    let t0 = OPEN_5_AT + 1.0;

    // Beat 1 -> 2: reaching beacon 1 completes B1 and plays the beat-2 line;
    // the objective is NOT up yet (it posts a gap later, never same-frame).
    enter(&mut app, ID_BEACON_1);
    assert!(
        !has_obj(&app, OBJ_B2),
        "the instruction objective is not posted in the transition frame"
    );

    // Still short of INSTRUCTION_GAP: nothing posts.
    set_clock(&mut app, t0 + INSTRUCTION_GAP - 1.0);
    pulse(&mut app);
    assert!(
        !has_obj(&app, OBJ_B2),
        "the objective waits at least the instruction gap"
    );

    // Just past INSTRUCTION_GAP but well short of REVEAL_GAP: it posts NOW.
    // A uniform REVEAL_GAP (the pre-split behavior) would still be waiting.
    const _: () = assert!(
        INSTRUCTION_GAP + 1.0 < REVEAL_GAP,
        "the instruction gap must be strictly shorter than the reveal gap for this pin to bite"
    );
    set_clock(&mut app, t0 + INSTRUCTION_GAP + 1.0);
    pulse(&mut app);
    assert!(
        has_obj(&app, OBJ_B2),
        "the instruction objective lands mid-read (after INSTRUCTION_GAP), not after the full REVEAL_GAP"
    );
}

/// The beat variable gates every non-setup handler: a stray re-entry
/// into an old area cannot re-fire a finished beat, and the tally
/// milestones advance on OnUpdate (order-independent of the pickup
/// event's handler iteration).
#[test]
fn every_gameplay_handler_is_beat_gated() {
    let config = scenario();

    for event in &config.events {
        if matches!(event.name, EventConfig::OnStart) {
            continue;
        }
        // The death handler is deliberately beat-free (dying is always
        // fatal).
        let is_death_handler = event.filters.iter().any(|filter| {
            matches!(
                filter,
                EventFilterConfig::Entity(entity)
                    if entity.id.as_deref() == Some(ID_PLAYER)
            )
        });
        if is_death_handler {
            continue;
        }
        assert!(
            event
                .filters
                .iter()
                .any(|filter| matches!(filter, EventFilterConfig::Expression(_))),
            "handler {:?} with entity filters {:?} lacks a beat/variable guard",
            event.name,
            event.filters.len()
        );
    }
}

/// The first/New Game scenario runs FINITE ammo now that catalog weapons
/// auto-reload: guard that the player ship is built with `infinite_ammo` OFF,
/// so the flag cannot be silently turned back on and hide the ammo readout /
/// reload cadence. Fails if the flag is flipped - the mechanism test in
/// nova_scenario would still pass, so this is the one that pins the user-facing
/// behavior (was ON before the reload mechanic).
#[test]
fn the_new_game_player_has_finite_reloading_ammo() {
    let player = player_ship();
    let ScenarioObjectKind::Spaceship(config) = player.kind else {
        panic!("the player object must be a spaceship");
    };
    let SpaceshipController::Player(controller) = config.controller else {
        panic!("the player ship must be player-controlled");
    };
    assert!(
        !controller.infinite_ammo,
        "the New Game player must have finite (auto-reloading) ammo"
    );
}

/// The player's controller section carries DisableVerb modifications for
/// GOTO, LOCK and ORBIT (STOP is left granted), so those verbs are off from
/// the instant the section is built - no OnStart-action ordering window. The
/// controller is the racer's inline Controller cube, and the withholding is
/// expressed as modifications on it.
#[test]
fn the_new_game_player_starts_with_goto_withheld() {
    let player = player_ship();
    let ScenarioObjectKind::Spaceship(config) = player.kind else {
        panic!("the player object must be a spaceship");
    };
    let catalog = crate::sections::build_sections(&crate::sections::SectionMeshRefs::from_paths());
    let is_controller = |section: &SpaceshipSectionConfig| match &section.source {
        SectionSource::Inline(c) => matches!(c.kind, SectionKind::Controller(_)),
        SectionSource::Prototype(id) => catalog
            .iter()
            .find(|c| c.base.id == *id)
            .is_some_and(|c| matches!(c.kind, SectionKind::Controller(_))),
    };
    let controller = config
        .sections
        .iter()
        .find(|section| is_controller(section))
        .expect("the player ship has a controller cube");

    let disables_verb = |verb: FlightVerb| {
        controller
            .modifications
            .iter()
            .any(|m| matches!(m, SectionModification::DisableVerb(v) if *v == verb))
    };
    assert!(
        disables_verb(FlightVerb::Goto),
        "GOTO starts withheld on the fresh player controller"
    );
    assert!(
        disables_verb(FlightVerb::Lock) && disables_verb(FlightVerb::Orbit),
        "LOCK and ORBIT start withheld too - each computer comes online with its lesson"
    );
    assert!(
        !disables_verb(FlightVerb::Stop),
        "STOP is granted from the start (the very first lesson needs it)"
    );
}

/// End-to-end: GOTO is withheld on the live player controller after boot
/// and is granted when the first objective (beat 1) completes. Withheld
/// initially and granted after - deleting either the config off-state or
/// the beat-1 SetControllerVerb would flip one of these asserts.
#[test]
fn goto_unlocks_at_the_first_objective() {
    use nova_events::prelude::*;

    let mut app = scripted_app();

    let controller_goto = |app: &mut App| -> bool {
        let player = {
            let mut q = app.world_mut().query::<(Entity, &EntityId)>();
            q.iter(app.world())
                .find(|(_, id)| id.0 == ID_PLAYER)
                .map(|(e, _)| e)
                .expect("player ship spawned")
        };
        let mut q = app
            .world_mut()
            .query_filtered::<(&ChildOf, Option<&WithheldVerbs>), With<ControllerSectionMarker>>();
        q.iter(app.world())
            .find(|(&ChildOf(parent), _)| parent == player)
            .map(|(_, withheld)| withheld.is_none_or(|w| w.granted(FlightVerb::Goto)))
            .expect("player has a controller section")
    };

    boot(&mut app);
    assert!(
        !controller_goto(&mut app),
        "GOTO is withheld on the fresh ship"
    );

    // Clearing beat 1 (the first objective) grants GOTO.
    enter(&mut app, ID_BEACON_1);
    assert!(
        controller_goto(&mut app),
        "reaching beacon 1 (first objective) unlocks GOTO"
    );
}

/// Pacing pass (owner playtest): the opening holds a real conversation before
/// objective 1, and every navigation beat stamps a beat gate, so the next
/// objective posts a beat after the transition line instead of back to back.
/// Config-level pin so deleting the deferral, the voice, or the beat-gate
/// timing fails here.
#[test]
fn the_opening_converses_before_objective_one_and_beats_breathe() {
    let config = scenario();

    let posts = |event: &ScenarioEventConfig, id: &str| {
        event
            .actions
            .iter()
            .any(|a| matches!(a, EventActionConfig::Objective(o) if o.id == id))
    };

    // OnStart posts NO objective at all - the panel stays empty through the
    // opening conversation (owner pacing pass); objective 1 posts only when the
    // conversation hands off.
    let on_start = config
        .events
        .iter()
        .find(|e| matches!(e.name, EventConfig::OnStart))
        .unwrap();
    assert!(
        !on_start
            .actions
            .iter()
            .any(|a| matches!(a, EventActionConfig::Objective(_))),
        "OnStart posts no objective during the opening conversation"
    );
    assert!(
        !posts(on_start, OBJ_B1),
        "objective 1 is deferred past the opening conversation"
    );
    let obj1_posts = config
        .events
        .iter()
        .filter(|e| !matches!(e.name, EventConfig::OnStart) && posts(e, OBJ_B1))
        .count();
    assert_eq!(obj1_posts, 1, "exactly one deferred objective-1 post");

    // The opening + the per-beat transition lines carry voice, and the
    // player has lines (the campaign's first player voice - the belt
    // register, "You").
    let speakers: Vec<String> = config
        .events
        .iter()
        .flat_map(|e| e.actions.iter())
        .filter_map(|a| match a {
            EventActionConfig::StoryMessage(s) => Some(s.speaker.clone()),
            _ => None,
        })
        .collect();
    let voice_lines = speakers
        .iter()
        .filter(|s| s.as_str() == PLAYER || s.as_str() == CAPTAIN_HALLORAN)
        .count();
    assert!(
        voice_lines >= 5,
        "the opening conversation and beat transition lines carry the captain/player voice, got {voice_lines}"
    );
    assert!(
        speakers.iter().any(|s| s == PLAYER),
        "the player speaks (the opening back-and-forth)"
    );

    // Every navigation beat transition stamps the beat gate (plus the
    // OnStart init), so the next objective posts a fixed delay after the
    // transition line - the "no two beats back to back" guarantee.
    let gate_stamps = config
        .events
        .iter()
        .flat_map(|e| e.actions.iter())
        .filter(|a| matches!(a, EventActionConfig::VariableSet(v) if v.key == VAR_GATE))
        .count();
    assert!(
        gate_stamps >= 9,
        "each navigation beat transition stamps the beat gate, got {gate_stamps}"
    );
}
