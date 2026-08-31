//! system_borrowed_battery: the Flight Computer works the player's idle PDCs, and
//! hands one straight back the moment the player locks something.
//!
//! The claim is an OWNERSHIP one, and it is about a PLAYER hull. Per-turret
//! point defence is a controller CAPABILITY
//! ([`FlightVerb::PointDefense`](nova_protocol::prelude::FlightVerb)), not a
//! behaviour of the AI controller: one allocator claims a player's IDLE mounts
//! the same way it claims an AI ship's, and lets go of one the instant the
//! player wants it.
//!
//! The range is built so nothing else can explain what it shows:
//!
//! - The defender is a real PLAYER ship - the chase camera, the targeting
//!   state, the weapons safety, the whole player path. Nobody touches its
//!   controls, so it is the IDLE case by construction.
//! - It is COLD the whole time it defends. The weapons safety would stop a
//!   player trigger here, so a round leaving the muzzle is the computer's
//!   exemption working, not a stance nobody set.
//! - The torpedoes are committed to a point 400 u BEYOND the defender, offset
//!   30 u off its flank. They fly through its 150 u envelope and fuze far
//!   away, so the defender is never damaged and the stream never stops.
//! - The lock is written onto the player's own `CombatLock` slot - the state
//!   the CTRL radar gesture lands, which `systems/system_player_path` drives with the
//!   real keys. What THIS range is about is what the mounts do about it.
//!
//! What the run walks: open the tubes -> watch an idle mount get claimed ->
//! watch it fire inside the 0.92 deg bearing gate while the ship is cold ->
//! lock a target and watch the mount leave the computer's pool the same beat ->
//! clear the lock and watch the debounce grace hold it back before it returns.
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//! ```text
//! NOVA_AUTOPILOT=1 cargo run --example system_borrowed_battery --features debug
//! # look for: `nova harness: reached Playing`,
//! #           `borrowed_battery: the computer claimed mount ...`,
//! #           `borrowed_battery: cold hull firing - aim error ... deg`,
//! #           `borrowed_battery: the lock took the mount back ...`,
//! #           `borrowed_battery: the mount returned after ... s of grace`,
//! #           `autopilot: cycle complete, no panic`
//! ```

use std::collections::BTreeMap;

use bevy::prelude::*;
use clap::Parser;
use nova_probe::fixtures::{self, prelude::*};
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "system_borrowed_battery")]
#[command(version = "1.0.0")]
#[command(about = "The flight computer works an idle player battery, and gives a mount back on a lock. Autopilot-only correctness range", long_about = None)]
struct Cli;

/// The scenario the range loads under.
const SCENARIO_ID: &str = "borrowed_battery";

/// The first step's name, so `loop_from` restarts the cycle at the spawn
/// without repeating the string.
#[cfg(feature = "debug")]
const LOAD_STEP: &str = "spawn the range";

/// Scenario object ids.
const DEFENDER_ID: &str = "defender";
const BOAT_ID: &str = "boat";

/// The defender's gun slot: bound to LMB like every shipped hull, because
/// binding presence must NOT decide ownership. A mount the player can fire is
/// still a mount the computer borrows while the player is not firing it.
const DEFENDER_GUNS: &str = "defender_guns";

/// Lateral offset (u) of the torpedo lane from the defender's hull: inside the
/// 150 u point-defence envelope for most of the lane, far enough that a torpedo
/// never collides with the ship being defended.
const PASS_OFFSET: f32 = 30.0;

/// Where the boat sits on the lane, and where the torpedoes are aimed. The aim
/// point is well beyond the defender, so a proximity fuze fires 400 u past it.
const LAUNCH_Z: f32 = 300.0;
const FLYBY_AIM_Z: f32 = -400.0;

/// The point-defence envelope the range is built around. Mirrors the engine's
/// `AI_POINT_DEFENSE_RANGE`, which is crate-private; the assertions read the
/// live geometry against THIS number, so a range that stops staging an
/// intercept fails loudly instead of passing on an empty sky.
#[cfg(feature = "debug")]
const PD_ENVELOPE: f32 = 150.0;

/// When the script cleared the combat lock, so the next beat can price the
/// debounce grace against the clock the grace itself runs on.
#[cfg(feature = "debug")]
#[derive(Resource)]
struct LockReleasedAt(f32);

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();
    let mut app = AppBuilder::new().with_game_plugins(custom_plugin).build();

    #[cfg(feature = "debug")]
    {
        app.add_plugins(nova_screenshot(
            nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
                .step(LOAD_STEP)
                .enter(GameStates::Loading)
                .until(and(player_ship_present(), ship_present(BOAT_ID)))
                .deadline(60.0)
                .add()
                // The scene is live again: close the reload interval so a frame
                // capture excludes it. A no-op on the first cycle.
                .step("close the reload interval")
                .on_enter(nova_probe::capture_reload_end)
                .add()
                // The claim. An idle battery, a torpedo in the envelope, and a
                // mount the player never touched holding it.
                .step("open the tubes")
                .on_enter(open_the_tubes)
                .until(a_mount_is_claimed())
                .deadline(90.0)
                .add()
                .step("report the claim")
                .on_enter(report_the_claim)
                .add()
                // The exemption. The hull is COLD - a player trigger would be
                // refused here - and the round still leaves, inside the gate.
                .step("let the computer take its shot")
                .until(a_mount_is_firing())
                .deadline(60.0)
                .add()
                .step("report the cold shot")
                .on_enter(report_the_cold_shot)
                .add()
                // The precedence. A lock is the top tier, and it takes the
                // mount in the same beat - no grace on the way IN.
                .step("lock a target")
                .on_enter(lock_the_boat)
                .until(the_player_holds_every_mount())
                .deadline(20.0)
                .add()
                .step("report the steal")
                .on_enter(report_the_steal)
                .add()
                // The debounce. Releasing does not hand the battery straight
                // back: the grace is what stops the mounts swinging away and
                // back inside one gesture.
                .step("clear the lock")
                .on_enter(release_the_lock)
                .until(the_computer_holds_every_mount())
                .deadline(20.0)
                .add()
                .step("report the return")
                .on_enter(report_the_return)
                .add()
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
/// Nobody drives the boat, so this does the one write a launch-time commit
/// would, and the guidance, arming and fuze run themselves. A POSITION, not an
/// entity: a torpedo committed to the defender would be a weapon aimed at the
/// subject of the run, and one committed to nothing has no guidance at all.
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

/// The defender: a real player hull with a flight computer and one PDC.
///
/// `infinite_ammo` keeps the magazine out of the story - this range is about
/// who holds the mount, not what an intercept costs.
fn defender(sections: &GameSections) -> SpaceshipConfig {
    fixtures::ship(
        sections,
        SpaceshipController::Player(PlayerControllerConfig {
            input_mapping: BTreeMap::from([(
                DEFENDER_GUNS.to_string(),
                vec![
                    MouseButton::Left.into(),
                    GamepadButton::RightTrigger2.into(),
                ],
            )]),
            speed_cap: None,
            infinite_ammo: true,
        }),
        &[
            SectionSpec::new("defender_hull", "reinforced_hull_section", Vec3::ZERO),
            SectionSpec::new(
                "defender_computer",
                "basic_controller_section",
                Vec3::new(0.0, 0.0, 1.0),
            ),
            SectionSpec::new(
                DEFENDER_GUNS,
                "pdc_kinetic_turret_section",
                Vec3::new(0.0, 0.75, 0.0),
            ),
        ],
    )
}

/// The torpedo boat: nobody drives it, so its bay answers a written input
/// directly instead of a weapons-safety stance, and it is Enemy-aligned so its
/// ordnance reads HOSTILE to the Player-aligned defender.
fn boat(sections: &GameSections) -> SpaceshipConfig {
    SpaceshipConfig {
        allegiance: Some(Allegiance::Enemy),
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
                // The 1x1x2 tube seats on the half cell so both of its cells
                // land on the grid and its back plate mates the hull.
                SectionSpec::new("boat_tube", "torpedo_section", Vec3::new(0.0, 0.0, -1.5)),
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

/// The range: the defender at the origin, the boat up the lane pointing past it
/// (ships spawn with -Z forward).
fn range_scenario(game_assets: &GameAssets, sections: &GameSections) -> ScenarioConfig {
    let objects = vec![
        spaceship(DEFENDER_ID, "Defender", Vec3::ZERO, defender(sections)),
        spaceship(
            BOAT_ID,
            "Torpedo Boat",
            Vec3::new(0.0, PASS_OFFSET, LAUNCH_Z),
            boat(sections),
        ),
    ];

    let events = fixtures::spawn_on_start(
        [
            objects,
            ThreePointRig::around("range", Vec3::ZERO, 10.0).objects(),
        ]
        .concat(),
    );

    ScenarioConfig {
        description: "An idle player battery answering a torpedo stream on its own.".to_string(),
        hidden: true,
        events,
        ..ScenarioConfig::new(
            SCENARIO_ID.to_string(),
            "Borrowed Battery".to_string(),
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

/// Advance once a ship root carries `id`.
#[cfg(feature = "debug")]
fn ship_present(id: &'static str) -> std::sync::Arc<dyn Fn(&World) -> bool + Send + Sync> {
    std::sync::Arc::new(move |world: &World| {
        object_by_id(world, id).is_some_and(|ship| world.get::<SpaceshipRootMarker>(ship).is_some())
    })
}

/// One reading of one of the defender's mounts.
#[cfg(feature = "debug")]
struct MountReading {
    turret: Entity,
    authority: MountAuthority,
    assignment: Option<Entity>,
    firing: bool,
}

/// Every live turret section on the defender, with who holds it, what it was
/// assigned and whether its trigger is down.
#[cfg(feature = "debug")]
fn defender_mounts(world: &World) -> Vec<MountReading> {
    let Some(defender) = object_by_id(world, DEFENDER_ID) else {
        return Vec::new();
    };
    let Some(children) = world.get::<Children>(defender) else {
        return Vec::new();
    };
    children
        .iter()
        .filter(|child| world.get::<TurretSectionMarker>(*child).is_some())
        .map(|turret| MountReading {
            turret,
            authority: world
                .get::<PointDefenseMount>(turret)
                .map(|mount| mount.authority)
                .unwrap_or(MountAuthority::Cold),
            assignment: world
                .get::<TurretDefenseTarget>(turret)
                .and_then(|assignment| **assignment),
            firing: world
                .get::<TurretSectionInput>(turret)
                .is_some_and(|input| **input),
        })
        .collect()
}

/// Hostile committed torpedoes inside the defender's point-defence envelope,
/// and how far out each one is.
#[cfg(feature = "debug")]
fn torpedoes_in_the_envelope(world: &World) -> Vec<(Entity, f32)> {
    let Some(defender) = object_by_id(world, DEFENDER_ID) else {
        return Vec::new();
    };
    let Some(anchor) = world.get::<Transform>(defender).map(|t| t.translation) else {
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

/// Advance once a mount the player never touched is the computer's AND has an
/// inbound on it. Both halves matter: the tier alone would not prove the
/// allocator ever reached a player hull.
#[cfg(feature = "debug")]
fn a_mount_is_claimed() -> std::sync::Arc<dyn Fn(&World) -> bool + Send + Sync> {
    std::sync::Arc::new(|world: &World| {
        defender_mounts(world).iter().any(|mount| {
            mount.authority == MountAuthority::FlightComputer && mount.assignment.is_some()
        })
    })
}

/// Advance once a claimed mount has its trigger down.
#[cfg(feature = "debug")]
fn a_mount_is_firing() -> std::sync::Arc<dyn Fn(&World) -> bool + Send + Sync> {
    std::sync::Arc::new(|world: &World| {
        defender_mounts(world)
            .iter()
            .any(|mount| mount.authority == MountAuthority::FlightComputer && mount.firing)
    })
}

#[cfg(feature = "debug")]
fn the_player_holds_every_mount() -> std::sync::Arc<dyn Fn(&World) -> bool + Send + Sync> {
    std::sync::Arc::new(|world: &World| {
        let mounts = defender_mounts(world);
        !mounts.is_empty() && mounts.iter().all(|mount| mount.authority.is_player())
    })
}

#[cfg(feature = "debug")]
fn the_computer_holds_every_mount() -> std::sync::Arc<dyn Fn(&World) -> bool + Send + Sync> {
    std::sync::Arc::new(|world: &World| {
        let mounts = defender_mounts(world);
        !mounts.is_empty()
            && mounts
                .iter()
                .all(|mount| mount.authority == MountAuthority::FlightComputer)
    })
}

/// Open the bay and leave it open: it relaunches on its own fire-rate clock, so
/// one write buys a stream down the lane for the whole run rather than a single
/// shot that has to be timed against the beats.
#[cfg(feature = "debug")]
fn open_the_tubes(world: &mut World) {
    let bays: Vec<Entity> = world
        .query_filtered::<Entity, With<TorpedoSectionMarker>>()
        .iter(world)
        .collect();
    assert!(
        !bays.is_empty(),
        "borrowed_battery: the boat must carry a torpedo bay"
    );
    for bay in bays {
        if let Some(mut input) = world.entity_mut(bay).get_mut::<TorpedoSectionInput>() {
            **input = true;
        }
    }
    let t = world.resource::<Time>().elapsed_secs();
    nova_probe::probe_marker(world, "beat: tubes open", serde_json::json!({ "t": t }));
}

/// The claim, recorded where the step's predicate already held it: a mount
/// nobody aimed, holding a torpedo, on a hull whose weapons are SAFE.
#[cfg(feature = "debug")]
fn report_the_claim(world: &mut World) {
    let inbound = torpedoes_in_the_envelope(world);
    assert!(
        !inbound.is_empty(),
        "borrowed_battery: the range must stage a torpedo inside the {PD_ENVELOPE} u \
         envelope, or the claim proves nothing"
    );
    let nearest = inbound
        .iter()
        .map(|(_, distance)| *distance)
        .fold(f32::MAX, f32::min);

    let mounts = defender_mounts(world);
    let claimed = mounts
        .iter()
        .find(|mount| {
            mount.authority == MountAuthority::FlightComputer && mount.assignment.is_some()
        })
        .expect("borrowed_battery: the step advanced on a claimed mount");
    assert!(
        !hull_is_hot(world),
        "borrowed_battery: nobody touched the controls, so the battery must be COLD - \
         a hot hull would explain the shot without the computer"
    );
    nova_probe::probe_marker(
        world,
        "outcome: the computer claims an idle mount",
        serde_json::json!({}),
    );

    let t = world.resource::<Time>().elapsed_secs();
    let turret = claimed.turret;
    let torpedo = claimed.assignment;
    info!(
        "borrowed_battery: the computer claimed mount {turret:?} onto torpedo {torpedo:?} \
         ({nearest:.0} u out, {} in the envelope)",
        inbound.len()
    );
    nova_probe::probe_marker(
        world,
        "beat: idle mount claimed",
        serde_json::json!({
            "t": t,
            "authority": "FlightComputer",
            "nearest_inbound_u": nearest,
            "in_envelope": inbound.len(),
            "weapons_hot": false,
        }),
    );
    nova_probe::probe_snapshot(world, "idle battery claimed by the flight computer");
}

/// The exemption and the discipline in one reading: the hull is cold, the
/// trigger is down, and the barrel is inside the 0.92 deg gate it shares with
/// the AI's own point defence.
#[cfg(feature = "debug")]
fn report_the_cold_shot(world: &mut World) {
    assert!(
        !hull_is_hot(world),
        "borrowed_battery: the whole claim is that a COLD hull fires - a raise or a \
         lock here would make the round the player's"
    );
    let mounts = defender_mounts(world);
    let firing = mounts
        .iter()
        .find(|mount| mount.authority == MountAuthority::FlightComputer && mount.firing)
        .expect("borrowed_battery: the step advanced on a firing mount");
    let error = mount_aim_error_deg(world, firing.turret)
        .expect("borrowed_battery: a firing mount has a muzzle to measure");
    let gate = TURRET_ON_TARGET_RAD.to_degrees();
    assert!(
        error <= gate,
        "borrowed_battery: the computer fired {error:.3} deg off, outside the {gate:.3} deg \
         bearing gate - fire discipline must be the SAME rule the AI trigger uses"
    );
    nova_probe::probe_marker(
        world,
        "outcome: the cold hull fires inside the bearing gate",
        serde_json::json!({}),
    );

    let t = world.resource::<Time>().elapsed_secs();
    info!("borrowed_battery: cold hull firing - aim error {error:.3} deg, gate {gate:.3} deg");
    nova_probe::probe_marker(
        world,
        "beat: cold mount fired inside the gate",
        serde_json::json!({
            "t": t,
            "aim_error_deg": error,
            "bearing_gate_deg": gate,
            "weapons_hot": false,
        }),
    );
    nova_probe::probe_snapshot(world, "cold battery firing on an inbound torpedo");
}

/// Lock a target. The write lands on the player's own `CombatLock` slot - the
/// state the CTRL radar gesture commits, which `systems/system_player_path` drives
/// with the real keys; what THIS range is about is what the mounts do about it.
#[cfg(feature = "debug")]
fn lock_the_boat(world: &mut World) {
    let boat = object_by_id(world, BOAT_ID).expect("borrowed_battery: the boat must be lockable");
    let player = world
        .try_query_filtered::<Entity, With<PlayerSpaceshipMarker>>()
        .and_then(|mut query| query.iter(world).next())
        .expect("borrowed_battery: the defender is a player ship");
    let t = world.resource::<Time>().elapsed_secs();
    nova_probe::probe_marker(
        world,
        "beat: combat lock written",
        serde_json::json!({ "t": t }),
    );
    world
        .get_mut::<CombatLock>(player)
        .expect("borrowed_battery: a player ship carries the targeting state")
        .0 = Some(boat);
}

/// The steal: the top tier took every mount, the claims went with it, and the
/// trigger the computer was holding is released rather than left latched in the
/// player's hands.
#[cfg(feature = "debug")]
fn report_the_steal(world: &mut World) {
    let mounts = defender_mounts(world);
    assert!(
        !mounts.is_empty(),
        "borrowed_battery: the defender must still carry its mount"
    );
    for mount in &mounts {
        assert_eq!(
            mount.authority,
            MountAuthority::PlayerLock,
            "borrowed_battery: mount {:?} is not the lock's",
            mount.turret
        );
        assert_eq!(
            mount.assignment, None,
            "borrowed_battery: mount {:?} still holds the computer's claim",
            mount.turret
        );
        assert!(
            !mount.firing,
            "borrowed_battery: mount {:?} kept a trigger nobody pressed",
            mount.turret
        );
    }
    let inbound = torpedoes_in_the_envelope(world);
    assert!(
        !inbound.is_empty(),
        "borrowed_battery: the steal must happen MID-ENGAGEMENT, with a torpedo still \
         inside the envelope, or it is only a quiet sky"
    );
    nova_probe::probe_marker(
        world,
        "outcome: the player lock steals every mount",
        serde_json::json!({}),
    );

    let t = world.resource::<Time>().elapsed_secs();
    info!(
        "borrowed_battery: the lock took the mount back with {} torpedo(es) still inbound",
        inbound.len()
    );
    nova_probe::probe_marker(
        world,
        "beat: player lock stole the mount",
        serde_json::json!({
            "t": t,
            "authority": "PlayerLock",
            "in_envelope": inbound.len(),
            "assignment": Option::<u64>::None,
        }),
    );
    nova_probe::probe_snapshot(world, "player lock holding the whole battery");
}

/// Clear the lock and start the debounce, recording the clock the grace runs
/// on so the next beat can price it.
#[cfg(feature = "debug")]
fn release_the_lock(world: &mut World) {
    let player = world
        .try_query_filtered::<Entity, With<PlayerSpaceshipMarker>>()
        .and_then(|mut query| query.iter(world).next())
        .expect("borrowed_battery: the defender is a player ship");
    world
        .get_mut::<CombatLock>(player)
        .expect("borrowed_battery: a player ship carries the targeting state")
        .0 = None;
    let t = world.resource::<Time>().elapsed_secs();
    world.insert_resource(LockReleasedAt(t));
    nova_probe::probe_marker(world, "beat: lock cleared", serde_json::json!({ "t": t }));
}

/// The debounce, priced: the battery came back, and it did not come back on the
/// release frame.
#[cfg(feature = "debug")]
fn report_the_return(world: &mut World) {
    let released = world
        .get_resource::<LockReleasedAt>()
        .expect("borrowed_battery: the release beat records its own clock")
        .0;
    let t = world.resource::<Time>().elapsed_secs();
    let waited = t - released;
    assert!(
        waited >= POINT_DEFENSE_REGRASP_SECS,
        "borrowed_battery: the mount came back after {waited:.3} s, inside the \
         {POINT_DEFENSE_REGRASP_SECS} s grace - a release must not hand the battery \
         straight back"
    );
    nova_probe::probe_marker(
        world,
        "outcome: the mount returns only after the regrasp grace",
        serde_json::json!({}),
    );

    info!("borrowed_battery: the mount returned after {waited:.3} s of grace");
    nova_probe::probe_marker(
        world,
        "beat: mount returned after the grace",
        serde_json::json!({
            "t": t,
            "waited_secs": waited,
            "grace_secs": POINT_DEFENSE_REGRASP_SECS,
            "authority": "FlightComputer",
        }),
    );
    nova_probe::probe_snapshot(world, "battery back with the flight computer");
}

/// Whether the defender's weapons safety is OFF - i.e. whether a PLAYER trigger
/// would be allowed. Every claim in this run is that it is not.
#[cfg(feature = "debug")]
fn hull_is_hot(world: &World) -> bool {
    world
        .try_query_filtered::<&WeaponsHot, With<PlayerSpaceshipMarker>>()
        .and_then(|mut query| query.iter(world).next().map(|hot| hot.0))
        .unwrap_or(false)
}

/// How far off its aim point `turret`'s muzzle points, in degrees.
#[cfg(feature = "debug")]
fn mount_aim_error_deg(world: &World, turret: Entity) -> Option<f32> {
    let muzzle = **world.get::<TurretSectionMuzzleEntity>(turret)?;
    let pose = world.get::<GlobalTransform>(muzzle)?;
    let aim = (**world.get::<TurretSectionAimPoint>(turret)?)?;
    Some(muzzle_aim_error(pose.forward().into(), pose.translation(), aim).to_degrees())
}

/// Reload the range for a looped capture cycle.
#[cfg(feature = "debug")]
fn respawn_the_range(world: &mut World) {
    nova_probe::capture_reload_begin(world);
    let scenario = {
        let game_assets = world.resource::<GameAssets>().clone();
        let sections = world.resource::<GameSections>().clone();
        range_scenario(&game_assets, &sections)
    };
    world.trigger(LoadScenario(scenario));
}
