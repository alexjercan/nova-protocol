//! loop_torpedo_blast: the `torpedo-blast` webm loop - a Serpent torpedo
//! weaves in and kills a stationary gunship.
//!
//! The docs site's first MOVING figure, authored in the same idiom as every
//! still: an autopilot script whose steps call `loop_start` / `loop_end`
//! around the beats worth watching (`nova_autopilot::loops`). The recorded
//! window opens with the round already inside 700 m - drive plume and the
//! terminal weave on camera - and runs through the detonation, the fireball
//! and the hull coming apart inside it.
//!
//! ONE camera, posed before the loop opens and never touched again. What was
//! here was a hard cut to a tracking shot for the aftermath, and it read as a
//! jump: a fixed lens is what makes the frame a place the hit happened IN,
//! and the framing below is wide enough that the wreck spreads inside it
//! instead of leaving it.
//!
//! The set is `screenshot_combat`'s ordnance chapter boiled down to its two
//! ships: a target gunship parked at the origin and the torpedo boat high
//! off its quarter, firing DOWN through open sky. Same rock shell, same seed,
//! same reasoning (a level camera in a rock field frames rock soup; tipping
//! the lens up puts sky behind the subject). Every actor is scripted or
//! inert, so a re-capture reproduces the same frames.
//!
//! NOTHING about the destruction is authored. The target wears the catalog
//! hull at catalog health and the warhead does what a warhead does: 750 blast
//! damage across a 300 m sphere depletes the hull, the aggregate falls through
//! the structural-collapse floor, and every section it has left dies in one
//! frame - which is the chain of fireballs `nova_gameplay::integrity::pyre`
//! exists to draw. The earlier version of this loop toughened the plating and
//! carved two named cells off with a scripted `HealthApplyDamage`, and it
//! showed: a warhead went off and a gunship shrugged.
//!
//! Two run modes, both under the autopilot (`NOVA_AUTOPILOT`):
//! - `NOVA_AUTOPILOT=1` alone: the smoke path - the full walk, recording
//!   nothing.
//! - `NOVA_AUTOPILOT=1 NOVA_CAPTURE=1`: record and encode the loop into
//!   `NOVA_CAPTURE_DIR/torpedo-blast.webm` (the armed run is frame-clocked to
//!   the loop cadence).
//!
//! Capture:
//! ```text
//! NOVA_CAPTURE_DIR=target/loop-shots NOVA_AUTOPILOT=1 NOVA_CAPTURE=1 \
//!   cargo run --example loop_torpedo_blast --features debug
//! ```

#[path = "shared/kit.rs"]
mod kit;

use bevy::prelude::*;
use clap::Parser;
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "loop_torpedo_blast")]
#[command(version = "1.0.0")]
#[command(about = "The torpedo-blast webm loop: a salvo carves a gunship's outer hull. Autopilot-only: every actor is scripted or inert", long_about = None)]
struct Cli;

/// The loop this example records - the webm's file stem.
#[cfg(feature = "debug")]
const LOOP_NAME: &str = "torpedo-blast";

/// Scenario id of the stationary gunship the salvo carves.
const TARGET_ID: &str = "loop_target";
/// The catalog hull the salvo is aimed at.
const TARGET_HULL: &str = "block_gunship";
/// Scenario id of the torpedo boat.
const LANCE_ID: &str = "loop_lance";

/// Where the boat sits: high and off the target's quarter, so the salvo comes
/// DOWN onto the hull through open sky and clear of the rock shell - the
/// bearing `screenshot_combat` proved out for its ordnance chapter.
const LANCE_POSITION: Meters3 = Meters3::new(-380.0, 300.0, -560.0);

/// The proximity fuze fires 150 m short of the target (half the standard
/// assault bay's 300 m blast radius); the camera frames the midpoint of hull
/// and fuze point so the detonation opens inside the frame, not at its edge.
#[cfg(feature = "debug")]
const TORPEDO_FUZE_RANGE: Meters = Meters(150.0);

/// Where the lens stands, as an offset from what it frames.
///
/// From BELOW the target, looking up the salvo's bearing, sky behind the hull.
/// 320 m out, against a 110 m hull: the lens spans 1.47 times its distance, so
/// the frame is some 470 m wide. That holds the fireball (110 m at its peak),
/// the sections thrown out of it, and the drift they have picked up by the end
/// of the tail - the whole reason this framing does not need a camera move.
#[cfg(feature = "debug")]
const LENS_OFFSET: Meters3 = Meters3::new(210.0, -185.0, 160.0);

/// ONE round, not the boat's full salvo. One is already lethal - the 300 m
/// blast covers the whole gunship and the aggregate falls through the
/// structural-collapse floor - and a second warhead arriving into a hull that
/// is already coming apart adds a fireball nobody can attribute to anything.
#[cfg(feature = "debug")]
const EXPECTED_TORPEDO_COUNT: usize = 1;

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();
    let mut app = AppBuilder::new().with_game_plugins(custom_plugin).build();

    #[cfg(feature = "debug")]
    {
        app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
        app.add_plugins(nova_protocol::nova_debug::harness::LoopCapturePlugin::default());
        app.add_plugins(
            nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
                .step("load the range")
                .enter(GameStates::Loading)
                .until(cast_present())
                .deadline(60.0)
                .add()
                // The ONE framing, posed here and never touched again. The HUD
                // drops to cinematic so the fps/version bar stays out of the
                // recording.
                .step("frame the run")
                .on_enter(|world| {
                    hide_hud(world);
                    let subject = blast_subject(world);
                    pose_camera(world, subject + LENS_OFFSET, subject);
                })
                .until(elapsed(1.0))
                .add()
                .step("loose the torpedoes")
                .on_enter(loose_torpedoes)
                .until(torpedo_salvo_in_flight(EXPECTED_TORPEDO_COUNT))
                .deadline(10.0)
                .add()
                .step("commit the salvo")
                .on_enter(commit_torpedoes)
                .until(elapsed(0.1))
                .add()
                // Recording opens once the weave is inside 400 m: at a
                // Serpent's 320 m/s that is a second and a bit of lit drive
                // closing in - anticipation, not the two and a half seconds of
                // distant dot a wider gate spent.
                .step("wait for the terminal run")
                .until(torpedo_within(Meters(400.0)))
                .deadline(15.0)
                .add()
                .step("open the loop")
                .on_enter(|world| loop_start(world, LOOP_NAME))
                .add()
                .step("ride the salvo in")
                .until(no_torpedo_in_flight())
                .deadline(20.0)
                .add()
                // The tail, on the SAME lens: the fireball burns down, the
                // sections it threw tumble out of it, and the last frame is a
                // wreck spreading. Long enough for the hulk fireball's own
                // burn (under a second) plus the drift that makes it read as
                // wreckage rather than as a freeze, and no longer - past two
                // seconds the frame is debris holding station.
                .step("let the blast clear")
                .until(elapsed(2.0))
                .add()
                .step("close the loop")
                .on_enter(|world| loop_end(world, LOOP_NAME))
                .until(loop_written(LOOP_NAME))
                .deadline(60.0)
                .add(),
        );
        app.add_systems(Startup, (force_capture_resolution, hide_dev_overlays));
    }

    app.run()
}

fn custom_plugin(app: &mut App) {
    app.add_systems(OnEnter(GameAssetsStates::Loaded), load_scene);
}

fn load_scene(mut commands: Commands, game_assets: Res<GameAssets>, ships: Res<GameShips>) {
    commands.trigger(LoadScenario(blast_range(&game_assets, &ships)));
}

/// The set: the parked target, the boat above its quarter, the proven rock
/// shell around them and the photo rig. Both ships are `Controller::None` -
/// nothing here flies itself, so the capture is deterministic.
fn blast_range(game_assets: &GameAssets, ships: &GameShips) -> ScenarioConfig {
    // The whole shipped gunship at catalog health, turrets and cladding
    // included. Nothing here is tuned for the shot: what the warhead does to
    // this hull is what it does to the same hull in a fight.
    let target = ship(
        TARGET_ID,
        "Target",
        Meters3::ZERO,
        // Nosed toward the camera side, turned off square, so the carved
        // sections face the lens.
        Quat::from_rotation_y(std::f32::consts::PI - 0.4),
        Some(Allegiance::Enemy),
        kit::catalog_ship(ships, TARGET_HULL),
    );
    let lance = ship(
        LANCE_ID,
        "Lance",
        LANCE_POSITION,
        Transform::from_translation(LANCE_POSITION.to_engine())
            .looking_at(Vec3::ZERO, Vec3::Y)
            .rotation,
        Some(Allegiance::Player),
        kit::catalog_ship(ships, "block_cleanup_leader"),
    );
    // The same shell, radii and seed as screenshot_combat's hollow: proven to
    // keep the pocket clear of the subject and the salvo's bearing.
    let shell = kit::NearField {
        id_prefix: "loop_rock_",
        count: 48,
        seed: 40507,
        distance: (Meters(480.0), Meters(1_300.0)),
        radius: (Meters(12.0), Meters(32.0)),
        y_spread: Meters(460.0),
    };

    ScenarioConfig {
        description: "A torpedo salvo carving a parked gunship.".to_string(),
        events: vec![ScenarioEventConfig {
            label: None,
            name: EventConfig::OnStart,
            once: false,
            filters: vec![],
            actions: [
                vec![shell.action(game_assets), target, lance],
                ThreePointRig::around("photo", Meters3::ZERO, 1.0).actions(),
            ]
            .concat(),
        }],
        ..ScenarioConfig::new(
            "loop_torpedo_blast".to_string(),
            "Torpedo Blast Loop".to_string(),
            game_assets.cubemap.clone().into(),
        )
    }
}

/// One posed, inert ship in the set.
fn ship(
    id: &str,
    name: &str,
    position: Meters3,
    rotation: Quat,
    allegiance: Option<Allegiance>,
    hull: ShipHull,
) -> EventActionConfig {
    EventActionConfig::SpawnScenarioObject(ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: id.to_string(),
            name: name.to_string(),
            position,
            rotation,
        },
        kind: ScenarioObjectKind::Spaceship(SpaceshipConfig {
            controller: SpaceshipController::None,
            allegiance,
            hull: ShipSource::Inline(hull),
            ..default()
        }),
    })
}

/// Advance once both ships are in the world.
#[cfg(feature = "debug")]
fn cast_present() -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(|world: &World| {
        world
            .try_query_filtered::<&EntityId, With<SpaceshipRootMarker>>()
            .is_some_and(|mut query| {
                let ids: Vec<&str> = query.iter(world).map(|id| id.0.as_str()).collect();
                [TARGET_ID, LANCE_ID].iter().all(|id| ids.contains(id))
            })
    })
}

/// What the loop frames: the midpoint of the hull and the point the fuze will
/// fire at, [`TORPEDO_FUZE_RANGE`] short of it along the boat's bearing.
#[cfg(feature = "debug")]
fn blast_subject(world: &mut World) -> Meters3 {
    let target = target_position(world);
    let bearing = (LANCE_POSITION - target).get().normalize_or_zero();
    target + Meters3(bearing * (TORPEDO_FUZE_RANGE.get() * 0.5))
}

/// Where the target is (its spawn point if it is somehow gone).
#[cfg(feature = "debug")]
fn target_position(world: &mut World) -> Meters3 {
    ship_by_id(world, TARGET_ID)
        .and_then(|target| world.get::<GlobalTransform>(target))
        .map(|transform| Meters3::from_engine(transform.translation()))
        .unwrap_or(Meters3::ZERO)
}

/// Pull ONE bay trigger on the boat - the same [`TorpedoSectionInput`] write
/// the player's trigger observer and the AI's envelope make. The pick is the
/// port bay (lowest world X), so a re-capture launches from the same rail.
/// See [`EXPECTED_TORPEDO_COUNT`] on why the loop is one round.
#[cfg(feature = "debug")]
fn loose_torpedoes(world: &mut World) {
    let Some(lance) = ship_by_id(world, LANCE_ID) else {
        warn!("torpedo loop: no boat to fire");
        return;
    };
    let bay = world
        .query_filtered::<(Entity, &ChildOf, &GlobalTransform), With<TorpedoSectionMarker>>()
        .iter(world)
        .filter(|(_, parent, _)| parent.parent() == lance)
        .map(|(bay, _, transform)| (bay, transform.translation().x))
        .min_by(|(_, a), (_, b)| a.total_cmp(b))
        .map(|(bay, _)| bay);
    let Some(bay) = bay else {
        warn!("torpedo loop: the boat has no bays");
        return;
    };
    if let Some(mut input) = world.entity_mut(bay).get_mut::<TorpedoSectionInput>() {
        **input = true;
    }
    info!("torpedo loop: port bay firing");
}

/// Commit the fresh salvo to the target and drop the triggers, exactly the
/// way both production commit systems do it (see screenshot_combat).
#[cfg(feature = "debug")]
fn commit_torpedoes(world: &mut World) {
    let Some(target) = ship_by_id(world, TARGET_ID) else {
        warn!("torpedo loop: no target to commit the salvo to");
        return;
    };
    let bays: Vec<Entity> = world
        .query_filtered::<Entity, With<TorpedoSectionMarker>>()
        .iter(world)
        .collect();
    for bay in bays {
        if let Some(mut input) = world.entity_mut(bay).get_mut::<TorpedoSectionInput>() {
            **input = false;
        }
    }
    let torpedoes: Vec<Entity> = world
        .query_filtered::<Entity, (With<TorpedoProjectileMarker>, Without<TorpedoTargetChosen>)>()
        .iter(world)
        .collect();
    assert_eq!(
        torpedoes.len(),
        EXPECTED_TORPEDO_COUNT,
        "torpedo loop: the complete salvo must commit"
    );
    for torpedo in &torpedoes {
        world
            .entity_mut(*torpedo)
            .insert((TorpedoTargetChosen, TorpedoTargetEntity(target)));
    }
    info!("torpedo loop: {} torpedo(es) committed", torpedoes.len());
}

/// Advance once the whole salvo is in the world.
#[cfg(feature = "debug")]
fn torpedo_salvo_in_flight(
    expected: usize,
) -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(move |world: &World| {
        world
            .try_query_filtered::<Entity, With<TorpedoProjectileMarker>>()
            .is_some_and(|mut torpedoes| torpedoes.iter(world).count() == expected)
    })
}

/// Advance once the last torpedo is gone - the fuze despawns it and spawns
/// the blast in the same frame, so this IS the detonation.
#[cfg(feature = "debug")]
fn no_torpedo_in_flight() -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(|world: &World| torpedo_range(world).is_none())
}

/// Advance once the leading torpedo is within `distance` of the target.
#[cfg(feature = "debug")]
fn torpedo_within(
    distance: Meters,
) -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(move |world: &World| {
        torpedo_range(world).is_some_and(|range| range < distance)
    })
}

/// How far the closest live torpedo is from the target, if there is one of
/// each.
#[cfg(feature = "debug")]
fn torpedo_range(world: &World) -> Option<Meters> {
    let target = ship_by_id_ref(world, TARGET_ID)?;
    let position = world.get::<GlobalTransform>(target)?.translation();
    world
        .try_query_filtered::<&GlobalTransform, With<TorpedoProjectileMarker>>()?
        .iter(world)
        .map(|transform| Meters::from_engine(transform.translation().distance(position)))
        .min_by(|a, b| f32::total_cmp(&a.get(), &b.get()))
}

/// The ship root carrying scenario id `id`.
#[cfg(feature = "debug")]
fn ship_by_id(world: &mut World, id: &str) -> Option<Entity> {
    let mut query = world.query_filtered::<(Entity, &EntityId), With<SpaceshipRootMarker>>();
    query
        .iter(world)
        .find(|(_, live)| live.0 == id)
        .map(|(entity, _)| entity)
}

/// The same lookup from a read-only world (what a predicate gets).
#[cfg(feature = "debug")]
fn ship_by_id_ref(world: &World, id: &str) -> Option<Entity> {
    world
        .try_query_filtered::<(Entity, &EntityId), With<SpaceshipRootMarker>>()?
        .iter(world)
        .find(|(_, live)| live.0 == id)
        .map(|(entity, _)| entity)
}
