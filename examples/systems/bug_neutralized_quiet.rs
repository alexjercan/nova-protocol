//! bug_neutralized_quiet: a hull is neutralized with a torpedo in flight, and its
//! point defence goes quiet.
//!
//! The claim is behavioural and it is about a WRECK. Neutralization is a
//! capability edge, not a physical one - a neutralized ship keeps its hull, its
//! colliders and its allegiance, and only loses the crew. Every other AI
//! behaviour already read that edge through [`AINonCombatant`]; point defence
//! did not, because it deliberately bypasses the behavior state so that a
//! patrolling or idle ship still swats inbound ordnance. The result was a hulk
//! with nobody aboard shooting down torpedoes.
//!
//! The range is built so nothing else can explain the silence:
//!
//! - The raider carries a HULL, a flight computer and a turret. It is
//!   neutralized by killing the COMPUTER, so its gun is alive and loaded the
//!   whole time - a disarmed raider would go quiet for the boring reason.
//! - Its authored engage range is 1 u, so it never chases the boat and the
//!   whole run happens in a passive state. That is exactly the state point
//!   defence is documented to fire in, so the passive routine cannot be
//!   mistaken for the fix.
//! - The torpedoes are aimed at a point 400 u BEYOND the raider, offset 30 u
//!   off its flank. They fly through its 150 u point-defence envelope and
//!   detonate far away, so the raider is never damaged by them and the stream
//!   never stops.
//!
//! What the run walks: open the tubes -> watch the live hull take an inbound
//! and hold its trigger down -> kill the flight computer -> watch the wreck let
//! the next torpedo fly straight past -> shoot the wreck and watch it still
//! bleed.
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//! ```text
//! NOVA_AUTOPILOT=1 cargo run --example bug_neutralized_quiet --features debug
//! # look for: `nova harness: reached Playing`,
//! #           `neutralized_quiet: live hull defending - mount on torpedo ...`,
//! #           `neutralized_quiet: the wreck is quiet with a torpedo ... u out`,
//! #           `neutralized_quiet: the wreck still bleeds ...`,
//! #           `autopilot: cycle complete, no panic`
//! ```

use bevy::prelude::*;
use clap::Parser;
use nova_authoring::scenario_helpers::number;
use nova_probe::fixtures::{self, prelude::*};
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "bug_neutralized_quiet")]
#[command(version = "1.0.0")]
#[command(about = "A neutralized hull stops defending itself while a torpedo flies past. Autopilot-only correctness range", long_about = None)]
struct Cli;

/// The scenario the range loads under.
const SCENARIO_ID: &str = "neutralized_quiet";

/// The first step's name, so `loop_from` restarts the cycle at the spawn
/// without repeating the string.
#[cfg(feature = "debug")]
const LOAD_STEP: &str = "spawn the range";

/// Scenario object ids.
const RAIDER_ID: &str = "raider";
const BOAT_ID: &str = "boat";

/// The raider's section slot ids. The computer is the one the script kills:
/// losing the last flight computer is what neutralizes an armed ship, and it
/// leaves the gun working, which is the whole point of the range.
const RAIDER_HULL: &str = "raider_hull";
const RAIDER_COMPUTER: &str = "raider_computer";
const RAIDER_GUNS: &str = "raider_guns";

/// The variable the scenario's own `OnNeutralized` handler sets. Nothing in the
/// script writes it, so its value is the SCENARIO's reading of the edge - the
/// count a mission script would branch on, which this change must not move.
const NEUTRALIZED_VAR: &str = "raider_neutralized";

/// Lateral offset (u) of the torpedo lane from the raider's hull: close enough
/// to sit inside the 150 u point-defence envelope for ~290 u of the lane, far
/// enough that a torpedo never collides with the wreck.
const PASS_OFFSET: f32 = 30.0;

/// Where the boat sits on the lane, and where the torpedoes are aimed. The aim
/// point is well beyond the raider, so a torpedo's proximity fuze fires 400 u
/// past it instead of on it.
const LAUNCH_Z: f32 = 300.0;
const FLYBY_AIM_Z: f32 = -400.0;

/// The point-defence envelope the range is built around. Mirrors the engine's
/// `AI_POINT_DEFENSE_RANGE`, which is private; the assertions read the live
/// geometry against THIS number, so a range that stops staging an intercept
/// fails loudly instead of passing on an empty sky.
#[cfg(feature = "debug")]
const PD_ENVELOPE: f32 = 150.0;

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();
    let mut app = AppBuilder::new().with_game_plugins(custom_plugin).build();

    #[cfg(feature = "debug")]
    {
        app.init_resource::<LiveIntercept>();
        app.add_systems(Update, latch_the_live_intercept);
        app.add_plugins(nova_screenshot(
            nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
                // Both hulls up AND the neutralize latch re-seeded: on a looped
                // cycle the old run's variables outlive the load replacing them
                // (ScenarioLoaded fires before OnStart runs).
                .step(LOAD_STEP)
                .enter(GameStates::Loading)
                .until(and(
                    and(ship_present(RAIDER_ID), ship_present(BOAT_ID)),
                    scenario_variable_is(NEUTRALIZED_VAR, 0.0),
                ))
                .deadline(60.0)
                .add()
                // The scene is live again: close the reload interval so a frame
                // capture excludes it. A no-op on the first cycle.
                .step("close the reload interval")
                .on_enter(nova_probe::capture_reload_end)
                .add()
                // The control half. Until this holds, the range has not shown
                // that its point defence works at all, and the silence later
                // would prove nothing.
                .step("open the tubes")
                .on_enter(open_the_tubes)
                .until(the_raider_is_defending())
                .deadline(90.0)
                .add()
                .step("report the live intercept")
                .on_enter(report_live_intercept)
                .add()
                // The stimulus: brain death, not disarmament. The turret, its
                // ammunition and its arc are untouched.
                .step("kill the flight computer")
                .on_enter(kill_the_flight_computer)
                .until(and(
                    the_raider_is_neutralized(),
                    scenario_variable_is(NEUTRALIZED_VAR, 1.0),
                ))
                .deadline(30.0)
                .add()
                // Wait for a torpedo to be back inside the envelope, then give
                // the wreck a full second of it. A latched trigger or a stale
                // assignment has every chance to show.
                .step("let the next torpedo fly past the wreck")
                .until(and(a_torpedo_is_in_the_envelope(), elapsed(1.0)))
                .deadline(60.0)
                .add()
                .step("assert the wreck is quiet")
                .on_enter(assert_the_wreck_is_quiet)
                .add()
                // Quiet must not mean invulnerable: the hull is still solid and
                // still bleeds, and the scenario still counts exactly one
                // neutralization.
                .step("assert the wreck still takes damage")
                .on_enter(assert_the_wreck_still_bleeds)
                .add()
                // Enrolled in capture looping: when a frame capture outlives the
                // script the scene reloads and the arc replays, so the capture
                // measures ACTIVITY rather than an idle tail.
                .loop_from(LOAD_STEP)
                .on_loop(respawn_the_range),
        ));
        app.add_plugins(assert_scenario_loaded(SCENARIO_ID));
        app.add_plugins(nova_probe::NovaProbePlugin::default());
    }

    app.run()
}

fn custom_plugin(app: &mut App) {
    app.add_systems(OnEnter(GameAssetsStates::Loaded), setup_range);
    app.add_systems(Update, commit_fresh_torpedoes);
}

fn setup_range(mut commands: Commands, game_assets: Res<GameAssets>, sections: Res<GameSections>) {
    commands.trigger(LoadScenario(range_scenario(&game_assets, &sections)));
}

/// Commit every fresh torpedo to the fly-by lane.
///
/// A torpedo's target is decided exactly once, right after launch: the player
/// commits from the crosshair lock and the AI from its own `AITarget`. The boat
/// is neither (nobody drives it), so this does that one write, and the guidance,
/// arming and fuze run themselves.
///
/// The commit is a POSITION, not an entity. A position-committed torpedo flies
/// the lane and fuzes at the far end whatever happens to the raider; committing
/// it to the raider would make it a weapon aimed at the subject, and committing
/// it to nothing at all leaves it with no guidance and no thrust, so linear
/// damping stalls it before it ever reaches the envelope.
fn commit_fresh_torpedoes(
    mut commands: Commands,
    q_fresh: Query<
        Entity,
        (
            With<TorpedoProjectileMarker>,
            Without<TorpedoTargetChosen>,
            Without<TorpedoTargetEntity>,
        ),
    >,
) {
    for torpedo in &q_fresh {
        commands.entity(torpedo).insert((
            TorpedoTargetChosen,
            TorpedoTargetPosition(Vec3::new(0.0, PASS_OFFSET, FLYBY_AIM_Z)),
        ));
    }
}

/// The raider: a hull to hang everything off, the flight computer the script
/// kills, and the turret that must go quiet when it dies.
///
/// The hull sits at the origin so it is the section the others link to; killing
/// the computer therefore never disconnects the gun, and the ship stays ARMED
/// through the neutralization. `engage_range: Some(1.0)` is what keeps the
/// raider parked and passive: it acquires the boat but never leaves its
/// routine, so nothing it does in this run is engagement behaviour.
fn raider(sections: &GameSections) -> SpaceshipConfig {
    fixtures::ship(
        sections,
        SpaceshipController::AI(AIControllerConfig {
            engage_range: Some(1.0),
            ..default()
        }),
        &[
            SectionSpec::new(RAIDER_HULL, "reinforced_hull_section", Vec3::ZERO),
            SectionSpec::new(
                RAIDER_COMPUTER,
                "basic_controller_section",
                Vec3::new(0.0, 0.0, 1.0),
            ),
            SectionSpec::new(
                RAIDER_GUNS,
                "pdc_kinetic_turret_section",
                Vec3::new(0.0, 0.75, 0.0),
            ),
        ],
    )
}

/// The torpedo boat: nobody drives it, so its bay answers a written input
/// directly instead of a weapons-safety stance, and it is Player-aligned so
/// its ordnance reads HOSTILE to the Enemy-aligned raider.
fn boat(sections: &GameSections) -> SpaceshipConfig {
    SpaceshipConfig {
        allegiance: Some(Allegiance::Player),
        ..fixtures::ship(
            sections,
            SpaceshipController::None,
            &[
                SectionSpec::new("boat_hull", "reinforced_hull_section", Vec3::ZERO),
                SectionSpec::new(
                    "boat_computer",
                    "basic_controller_section",
                    Vec3::new(0.0, 0.0, 1.0),
                ),
                SectionSpec::new("boat_tube", "torpedo_section", Vec3::new(0.0, 0.0, -1.0)),
            ],
        )
    }
}

fn spaceship(
    id: &str,
    name: &str,
    position: Vec3,
    config: SpaceshipConfig,
) -> ScenarioObjectConfig {
    ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: id.to_string(),
            name: name.to_string(),
            position,
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Spaceship(config),
    }
}

/// The range: the raider at the origin, the boat up the lane pointing at it
/// (ships spawn with -Z forward), and the scenario's own reading of the
/// neutralize edge.
fn range_scenario(game_assets: &GameAssets, sections: &GameSections) -> ScenarioConfig {
    let objects = vec![
        spaceship(RAIDER_ID, "Raider", Vec3::ZERO, raider(sections)),
        spaceship(
            BOAT_ID,
            "Torpedo Boat",
            Vec3::new(0.0, PASS_OFFSET, LAUNCH_Z),
            boat(sections),
        ),
    ];

    let mut events = fixtures::spawn_on_start(
        [
            objects,
            ThreePointRig::around("range", Vec3::ZERO, 10.0).objects(),
        ]
        .concat(),
    );
    // The latch, seeded here and nowhere else, so its value after a reload
    // proves OnStart ran again rather than the old event world surviving.
    events[0]
        .actions
        .push(EventActionConfig::VariableSet(VariableSetActionConfig {
            key: NEUTRALIZED_VAR.to_string(),
            expression: number(0.0),
        }));
    // No player ship, so no chase camera: the scenario poses its own, off the
    // raider's quarter looking down the torpedo lane.
    events[0]
        .actions
        .push(EventActionConfig::SetCamera(SetCameraActionConfig {
            position: Vec3::new(220.0, 90.0, 160.0),
            look_at: Vec3::ZERO,
        }));
    // The scenario-facing edge, counted the way a mission script counts it.
    // This example must not change what it reports.
    events.push(ScenarioEventConfig {
        name: EventConfig::OnNeutralized,
        filters: vec![EventFilterConfig::Entity(EntityFilterConfig {
            id: Some(RAIDER_ID.to_string()),
            type_name: None,
            ..default()
        })],
        actions: vec![EventActionConfig::VariableSet(VariableSetActionConfig {
            key: NEUTRALIZED_VAR.to_string(),
            expression: number(1.0),
        })],
    });

    ScenarioConfig {
        description: "A neutralized hull with a live gun, and a torpedo flying past it."
            .to_string(),
        hidden: true,
        events,
        ..ScenarioConfig::new(
            SCENARIO_ID.to_string(),
            "Neutralized Quiet".to_string(),
            game_assets.cubemap.clone().into(),
        )
    }
}

/// The scenario-scoped root carrying `id`.
#[cfg(feature = "debug")]
fn object_by_id(world: &World, id: &str) -> Option<Entity> {
    world
        .try_query_filtered::<(Entity, &EntityId), With<ScenarioScopedMarker>>()
        .and_then(|mut query| {
            query
                .iter(world)
                .find(|(_, live)| live.0 == id)
                .map(|(entity, _)| entity)
        })
}

/// The live SECTION carrying `id`.
#[cfg(feature = "debug")]
fn section_by_id(world: &World, id: &str) -> Option<Entity> {
    world
        .try_query_filtered::<(Entity, &EntityId), With<SectionMarker>>()
        .and_then(|mut query| {
            query
                .iter(world)
                .find(|(_, live)| live.0 == id)
                .map(|(entity, _)| entity)
        })
}

/// Advance once a ship root carries `id`.
#[cfg(feature = "debug")]
fn ship_present(id: &'static str) -> std::sync::Arc<dyn Fn(&World) -> bool + Send + Sync> {
    std::sync::Arc::new(move |world: &World| {
        object_by_id(world, id).is_some_and(|ship| world.get::<SpaceshipRootMarker>(ship).is_some())
    })
}

/// Every live turret section on the raider, with its point-defence assignment
/// and its trigger.
#[cfg(feature = "debug")]
fn raider_mounts(world: &World) -> Vec<(Entity, Option<Entity>, bool)> {
    let Some(raider) = object_by_id(world, RAIDER_ID) else {
        return Vec::new();
    };
    let Some(children) = world.get::<Children>(raider) else {
        return Vec::new();
    };
    children
        .iter()
        .filter(|child| world.get::<TurretSectionMarker>(*child).is_some())
        .map(|turret| {
            (
                turret,
                world
                    .get::<TurretDefenseTarget>(turret)
                    .and_then(|assignment| **assignment),
                world
                    .get::<TurretSectionInput>(turret)
                    .is_some_and(|input| **input),
            )
        })
        .collect()
}

/// Hostile committed torpedoes inside the raider's point-defence envelope, and
/// how far out each one is.
#[cfg(feature = "debug")]
fn torpedoes_in_the_envelope(world: &World) -> Vec<(Entity, f32)> {
    let Some(raider) = object_by_id(world, RAIDER_ID) else {
        return Vec::new();
    };
    let Some(anchor) = world.get::<Transform>(raider).map(|t| t.translation) else {
        return Vec::new();
    };
    let Some(mut query) = world.try_query_filtered::<(Entity, &Transform), (
        With<TorpedoProjectileMarker>,
        With<TorpedoTargetChosen>,
    )>() else {
        return Vec::new();
    };
    query
        .iter(world)
        .map(|(torpedo, transform)| (torpedo, transform.translation.distance(anchor)))
        .filter(|(_, distance)| *distance <= PD_ENVELOPE)
        .collect()
}

/// The control half, LATCHED: the first frame a mount on the LIVE raider was
/// both assigned an inbound and firing at it, and what it was firing at.
///
/// A latch rather than a live reading, because the live reading is a RACE. The
/// AI trigger is a per-frame decision - it drops for the frames the barrel is
/// slewing onto its next bearing - so "assigned and firing" is true on some
/// frames and false on others while the raider is defending perfectly well.
/// A step that advanced on the live reading and a hook that re-read it a frame
/// later disagreed on most runs (the run failed 2/2 on this box, 4/4 on the
/// box that found it, task 20260816-114054): the step advanced, the trigger
/// dropped, the hook found nothing and panicked. Both the gate and the report
/// now read this, which is monotonic: once the range has shown its point
/// defence working, it has shown it.
#[cfg(feature = "debug")]
#[derive(Resource, Default)]
struct LiveIntercept(Option<(Entity, Entity)>);

/// Latch the first defending mount. Runs every frame, so the observation cannot
/// fall between two steps; does nothing once latched, and nothing at all after
/// the neutralization, whose whole point is that the mounts go quiet.
#[cfg(feature = "debug")]
fn latch_the_live_intercept(world: &mut World) {
    if world.resource::<LiveIntercept>().0.is_some() {
        return;
    }
    let defending = raider_mounts(world)
        .into_iter()
        .find_map(|(turret, assignment, firing)| {
            assignment
                .filter(|_| firing)
                .map(|torpedo| (turret, torpedo))
        });
    if let Some(pair) = defending {
        world.resource_mut::<LiveIntercept>().0 = Some(pair);
    }
}

/// Advance once a mount on the LIVE raider has been both assigned an inbound
/// and firing at it. Both halves matter: an assignment alone would not prove
/// the gun ever spoke.
#[cfg(feature = "debug")]
fn the_raider_is_defending() -> std::sync::Arc<dyn Fn(&World) -> bool + Send + Sync> {
    std::sync::Arc::new(|world: &World| {
        world
            .get_resource::<LiveIntercept>()
            .is_some_and(|latched| latched.0.is_some())
    })
}

#[cfg(feature = "debug")]
fn the_raider_is_neutralized() -> std::sync::Arc<dyn Fn(&World) -> bool + Send + Sync> {
    std::sync::Arc::new(|world: &World| {
        object_by_id(world, RAIDER_ID)
            .is_some_and(|raider| world.get::<NeutralizedMarker>(raider).is_some())
    })
}

#[cfg(feature = "debug")]
fn a_torpedo_is_in_the_envelope() -> std::sync::Arc<dyn Fn(&World) -> bool + Send + Sync> {
    std::sync::Arc::new(|world: &World| !torpedoes_in_the_envelope(world).is_empty())
}

/// Open the bay and leave it open: the bay relaunches on its own fire-rate
/// clock, so one write buys a stream of torpedoes down the lane for the whole
/// run rather than a single shot that has to be timed against the stimulus.
#[cfg(feature = "debug")]
fn open_the_tubes(world: &mut World) {
    let bays: Vec<Entity> = world
        .query_filtered::<Entity, With<TorpedoSectionMarker>>()
        .iter(world)
        .collect();
    assert!(
        !bays.is_empty(),
        "neutralized_quiet: the boat must carry a torpedo bay"
    );
    for bay in bays {
        if let Some(mut input) = world.entity_mut(bay).get_mut::<TorpedoSectionInput>() {
            **input = true;
        }
    }
}

/// Brain death through the production damage entry point: an overkill on the
/// flight-computer SECTION. The turret is not touched, so the wreck keeps a
/// working gun - which is what makes the silence afterwards a decision rather
/// than an absence.
#[cfg(feature = "debug")]
fn kill_the_flight_computer(world: &mut World) {
    let computer = section_by_id(world, RAIDER_COMPUTER)
        .expect("neutralized_quiet: the raider must carry a flight computer to lose");
    let t = world.resource::<Time>().elapsed_secs();
    nova_probe::probe_marker(
        world,
        "beat: brain death",
        serde_json::json!({ "t": t, "section": RAIDER_COMPUTER }),
    );
    world.trigger(HealthApplyDamage {
        entity: computer,
        source: None,
        amount: 1e6,
    });
}

/// The control half, recorded where the step's predicate already held it.
#[cfg(feature = "debug")]
fn report_live_intercept(world: &mut World) {
    let (turret, torpedo) = world
        .resource::<LiveIntercept>()
        .0
        .expect("neutralized_quiet: the step advanced on the latched intercept");
    nova_probe::probe_marker(
        world,
        "outcome: the live hull defends itself",
        serde_json::json!({ "mount": format!("{turret:?}"), "torpedo": format!("{torpedo:?}") }),
    );
    info!(
        "neutralized_quiet: live hull defending - mount {turret:?} on torpedo \
         {torpedo:?}, trigger down"
    );
    nova_probe::probe_snapshot(world, "live hull defending");
}

/// The claim. Every reading the AI publishes is empty, the trigger is released,
/// and a torpedo is demonstrably there to be shot at.
#[cfg(feature = "debug")]
fn assert_the_wreck_is_quiet(world: &mut World) {
    let raider = object_by_id(world, RAIDER_ID)
        .expect("neutralized_quiet: the wreck must still be in the world");
    assert!(
        world.get::<NeutralizedMarker>(raider).is_some(),
        "neutralized_quiet: the raider must still be neutralized"
    );
    assert!(
        world.get::<AINonCombatant>(raider).is_some(),
        "neutralized_quiet: the stand-down observer is what says the crew is gone"
    );
    assert_eq!(
        world
            .get::<AIPointDefenseTarget>(raider)
            .and_then(|target| **target),
        None,
        "neutralized_quiet: a wreck defends against nothing"
    );
    nova_probe::probe_marker(
        world,
        "outcome: the wreck holds no defence target",
        serde_json::json!({}),
    );

    // The gun is still THERE - a silence explained by a dead turret would be
    // no proof at all.
    let mounts = raider_mounts(world);
    assert!(
        !mounts.is_empty(),
        "neutralized_quiet: the wreck must still carry its turret"
    );
    let guns = section_by_id(world, RAIDER_GUNS)
        .expect("neutralized_quiet: the turret section must have survived the brain death");
    assert!(
        world.get::<SectionInactiveMarker>(guns).is_none(),
        "neutralized_quiet: the turret must still be WORKING, or the silence is \
         disarmament rather than a stand-down"
    );
    nova_probe::probe_marker(
        world,
        "outcome: the wreck still carries a working gun",
        serde_json::json!({}),
    );

    for (turret, assignment, firing) in &mounts {
        assert_eq!(
            *assignment, None,
            "neutralized_quiet: mount {turret:?} still holds a point-defence target"
        );
        assert!(
            !firing,
            "neutralized_quiet: mount {turret:?} is still firing on a wreck"
        );
    }
    nova_probe::probe_marker(
        world,
        "outcome: no mount is firing on a wreck",
        serde_json::json!({}),
    );

    let inbound = torpedoes_in_the_envelope(world);
    let nearest = inbound
        .iter()
        .map(|(_, distance)| *distance)
        .fold(f32::MAX, f32::min);
    assert!(
        !inbound.is_empty(),
        "neutralized_quiet: the range must stage a torpedo inside the {PD_ENVELOPE} u \
         envelope, or the silence proves nothing"
    );
    nova_probe::probe_marker(
        world,
        "outcome: a torpedo is inside the envelope",
        serde_json::json!({}),
    );
    info!(
        "neutralized_quiet: the wreck is quiet with a torpedo {nearest:.0} u out \
         ({} in the envelope, {} live mount(s), gun intact)",
        inbound.len(),
        mounts.len()
    );
    nova_probe::probe_snapshot(world, "wreck quiet with a torpedo in the envelope");
}

/// Quiet is not intangible and not invulnerable: the wreck still bleeds through
/// the production damage path, and the scenario still counts exactly the one
/// neutralization it counted before.
#[cfg(feature = "debug")]
fn assert_the_wreck_still_bleeds(world: &mut World) {
    let hull = section_by_id(world, RAIDER_HULL)
        .expect("neutralized_quiet: the wreck must still carry its hull section");
    let before = world
        .get::<Health>(hull)
        .expect("neutralized_quiet: a hull section carries Health")
        .current;
    world.trigger(HealthApplyDamage {
        entity: hull,
        source: None,
        amount: 5.0,
    });
    let after = world
        .get::<Health>(hull)
        .expect("neutralized_quiet: the hull section survives a scratch")
        .current;
    assert!(
        after < before,
        "neutralized_quiet: a neutralized hull is still solid and must still take \
         damage ({before} -> {after})"
    );
    nova_probe::probe_marker(
        world,
        "outcome: the wreck still takes damage",
        serde_json::json!({}),
    );

    let raider = object_by_id(world, RAIDER_ID)
        .expect("neutralized_quiet: a scratch must not despawn the wreck");
    assert!(
        world.get::<DefeatedMarker>(raider).is_some(),
        "neutralized_quiet: the wreck must still carry the unified defeat edge"
    );
    nova_probe::probe_marker(
        world,
        "outcome: the wreck stays defeated once",
        serde_json::json!({}),
    );
    info!("neutralized_quiet: the wreck still bleeds ({before} -> {after}), still defeated once");
    let t = world.resource::<Time>().elapsed_secs();
    nova_probe::probe_marker(world, "beat: done", serde_json::json!({ "t": t }));
}

/// Reload the range for a looped capture cycle.
#[cfg(feature = "debug")]
fn respawn_the_range(world: &mut World) {
    // The control half is per-cycle: a later cycle whose point defence never
    // spoke must not pass on the first cycle's latch.
    world.resource_mut::<LiveIntercept>().0 = None;
    nova_probe::capture_reload_begin(world);
    let scenario = {
        let game_assets = world.resource::<GameAssets>().clone();
        let sections = world.resource::<GameSections>().clone();
        range_scenario(&game_assets, &sections)
    };
    world.trigger(LoadScenario(scenario));
}
