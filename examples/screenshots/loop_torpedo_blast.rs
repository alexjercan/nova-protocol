//! loop_torpedo_blast: the `torpedo-blast` webm loop - a Serpent torpedo
//! weaves in and its blast carves the outer hull layers off a stationary
//! corvette.
//!
//! The docs site's first MOVING figure, authored in the same idiom as every
//! still: an autopilot script whose steps call `loop_start` / `loop_end`
//! around the beats worth watching (`nova_autopilot::loops`). The recorded
//! window opens with the round already inside 70 units - drive plume and the
//! terminal weave on camera - runs through the detonation and the section
//! deaths, and CUTS to a tracking shot of the carved survivor for the calm
//! tail (the blast impulse throws the hull out of any fixed framing within a
//! second).
//!
//! The set is `screenshot_combat`'s ordnance chapter boiled down to its two
//! ships: a target corvette parked at the origin and the torpedo boat high
//! off its quarter, firing DOWN through open sky. Same rock shell, same seed,
//! same reasoning (a level camera in a rock field frames rock soup; tipping
//! the lens up puts sky behind the subject). Every actor is scripted or
//! inert, so a re-capture reproduces the same frames.
//!
//! The destruction is authored as a difference in section health, not in the
//! weapon: the warhead's 750 blast damage covers the whole hull, so the
//! non-carved sections are set tough enough to take it as wounds
//! ([`TOUGH_SECTION_HEALTH`]) while the carve targets stay at prototype
//! health and die to the real detonation - with a scripted carve as the
//! deterministic backstop, timed to the same beat.
//!
//! Two run modes, both under the autopilot (`NOVA_AUTOPILOT`):
//! - `NOVA_AUTOPILOT=1` alone: the smoke path - the full walk, recording
//!   nothing.
//! - `NOVA_AUTOPILOT=1 NOVA_CAPTURE=1`: record and encode the loop into
//!   `NOVA_SHOT_DIR/torpedo-blast.webm` (the armed run is frame-clocked to
//!   the loop cadence).
//!
//! Capture:
//! ```text
//! NOVA_SHOT_DIR=target/loop-shots NOVA_AUTOPILOT=1 NOVA_CAPTURE=1 \
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
#[command(about = "The torpedo-blast webm loop: a salvo carves a corvette's outer hull", long_about = None)]
struct Cli;

/// The loop this example records - the webm's file stem.
#[cfg(feature = "debug")]
const LOOP_NAME: &str = "torpedo-blast";

/// Scenario id of the stationary corvette the salvo carves.
const TARGET_ID: &str = "loop_target";
/// Scenario id of the torpedo boat.
const LANCE_ID: &str = "loop_lance";

/// Where the boat sits: high and off the target's quarter, so the salvo comes
/// DOWN onto the hull through open sky and clear of the rock shell - the
/// bearing `screenshot_combat` proved out for its ordnance chapter.
const LANCE_POSITION: Vec3 = Vec3::new(-38.0, 30.0, -56.0);

/// The proximity fuze fires 15 units short of the target (half the cargo-B
/// bay's 30-unit blast radius); the camera frames the midpoint of hull and
/// fuze point so the detonation opens inside the frame, not at its edge.
#[cfg(feature = "debug")]
const TORPEDO_FUZE_RANGE: f32 = 15.0;

/// ONE round, not the boat's full salvo: the 30-unit blast covers the whole
/// corvette, and two same-tick detonations (shared health snapshot,
/// `blast_penetration`'s BLUE lane) gut every outer section at once - the
/// aggregate falls through the structural-collapse floor and the WHOLE ship
/// despawns. One warhead leaves a wounded hull for the scripted carve.
#[cfg(feature = "debug")]
const EXPECTED_TORPEDO_COUNT: usize = 1;

/// The outer hull layers the detonation takes off: the sections on the blast
/// side of the parked target. They spawn at prototype health while the rest
/// of the hull is authored tougher (see [`blast_range`]), so the blast beat
/// destroys exactly these and the ship SURVIVES into the aftermath frames.
const CARVED_SECTIONS: [&str; 2] = ["nose", "pod_port"];

/// Authored health for every non-carved section. A Serpent's warhead is 750
/// blast damage across a 30-unit sphere - the whole 4-unit hull is inside it,
/// and every prototype-health section is depleted outright (proved live: the
/// root structurally collapsed mid-loop at 42 of 2030 aggregate). At this
/// figure the same warhead wounds the hull for a few hundred per section and
/// the ship SURVIVES into the aftermath, short exactly the carved layers.
const TOUGH_SECTION_HEALTH: f32 = 2500.0;

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();
    let mut app = AppBuilder::new().with_game_plugins(custom_plugin).build();

    #[cfg(feature = "debug")]
    {
        app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
        app.add_plugins(nova_protocol::nova_debug::harness::LoopCapturePlugin);
        app.add_plugins(
            nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
                .step("load the range")
                .enter(GameStates::Loading)
                .until(cast_present())
                .deadline(60.0)
                .add()
                // One fixed framing for the whole loop: from below the target,
                // looking up the salvo's bearing, sky behind the hull. The HUD
                // drops to cinematic so the fps/version bar stays out of the
                // recording.
                .step("frame the run")
                .on_enter(|world| {
                    hide_hud(world);
                    let subject = blast_subject(world);
                    pose_camera(world, subject + Vec3::new(16.0, -14.0, 12.0), subject);
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
                // Recording opens once the weave is inside 70 units: the loop
                // spends its opening seconds on a lit drive closing in rather
                // than on a distant dot.
                .step("wait for the terminal run")
                .until(torpedo_within(70.0))
                .deadline(15.0)
                .add()
                .step("open the loop")
                .on_enter(|world| loop_start(world, LOOP_NAME))
                .until(frames(1))
                .add()
                .step("ride the salvo in")
                .until(no_torpedo_in_flight())
                .deadline(20.0)
                .add()
                // The deterministic backstop for the carve: the real blast
                // usually kills the prototype-health sections itself (the
                // lookup then warns and moves on), and this beat guarantees
                // it through the production damage path either way.
                .step("carve the outer hull")
                .on_enter(carve_target_sections)
                .until(carved_sections_gone())
                .deadline(5.0)
                .add()
                // The calm tail: a hard CUT to a TRACKING shot on the live
                // hull. The blast impulse throws the ship fast enough that
                // any fixed aftermath framing is empty space within a second
                // (proved live twice), so the camera re-poses off the hull
                // every frame - the carved ship rides steady in frame while
                // debris and rocks stream past, and the loop's last frame is
                // a readable aftermath.
                .step("let the blast clear")
                .each(|world, _| {
                    let target = target_position(world);
                    pose_camera(world, target + Vec3::new(14.0, -10.0, 11.0), target);
                })
                .until(elapsed(2.5))
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
    // The whole shipped corvette, turrets included. The kit used to drop them:
    // its own hand-typed copy of the mount centres had drifted from the
    // builders', so the mounts mated nothing and the ship came back
    // `Disconnected` - empty adjacency, under which the scripted carve would
    // burst the WHOLE hull instead of taking its outer layers off. The kit
    // reads the ship catalog now. The non-carved sections are authored tough -
    // see [`TOUGH_SECTION_HEALTH`].
    let target = ship(
        TARGET_ID,
        "Target",
        Vec3::ZERO,
        // Nosed toward the camera side, turned off square, so the carved
        // sections face the lens.
        Quat::from_rotation_y(std::f32::consts::PI - 0.4),
        Some(Allegiance::Enemy),
        kit::kenney_hull(ships, "cargoa")
            .into_iter()
            .map(|mut section| {
                if !CARVED_SECTIONS.contains(&section.id.as_str()) {
                    section.modifications =
                        vec![SectionModification::SetHealth(TOUGH_SECTION_HEALTH)];
                }
                section
            })
            .collect(),
    );
    let lance = ship(
        LANCE_ID,
        "Lance",
        LANCE_POSITION,
        Transform::from_translation(LANCE_POSITION)
            .looking_at(Vec3::ZERO, Vec3::Y)
            .rotation,
        Some(Allegiance::Player),
        kit::kenney_hull(ships, "cargob"),
    );
    // The same shell, radii and seed as screenshot_combat's hollow: proven to
    // keep the pocket clear of the subject and the salvo's bearing.
    let shell = kit::NearField {
        id_prefix: "loop_rock_",
        count: 48,
        seed: 40507,
        distance: (48.0, 130.0),
        radius: (1.2, 3.2),
        y_spread: 46.0,
    };

    ScenarioConfig {
        description: "A torpedo salvo carving a parked corvette.".to_string(),
        events: vec![ScenarioEventConfig {
            name: EventConfig::OnStart,
            filters: vec![],
            actions: [
                vec![shell.action(game_assets), target, lance],
                ThreePointRig::around("photo", Vec3::ZERO, 1.0).actions(),
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
    position: Vec3,
    rotation: Quat,
    allegiance: Option<Allegiance>,
    sections: Vec<SpaceshipSectionConfig>,
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
            hull: ShipSource::Inline(ShipHull {
                sections,
                ..default()
            }),
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
fn blast_subject(world: &mut World) -> Vec3 {
    let target = target_position(world);
    let bearing = (LANCE_POSITION - target).normalize_or_zero();
    target + bearing * (TORPEDO_FUZE_RANGE * 0.5)
}

/// Where the target is (its spawn point if it is somehow gone).
#[cfg(feature = "debug")]
fn target_position(world: &mut World) -> Vec3 {
    ship_by_id(world, TARGET_ID)
        .and_then(|target| world.get::<GlobalTransform>(target))
        .map(|transform| transform.translation())
        .unwrap_or(Vec3::ZERO)
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

/// Take the carved sections off through the production damage path.
#[cfg(feature = "debug")]
fn carve_target_sections(world: &mut World) {
    for section in CARVED_SECTIONS {
        let Some(node) = target_section_health(world, section) else {
            warn!("torpedo loop: no health node under section '{section}'");
            continue;
        };
        world.trigger(HealthApplyDamage {
            entity: node,
            source: None,
            amount: 1.0e6,
        });
        info!("torpedo loop: carved '{section}' off the target");
    }
}

/// Advance once every carved section is really gone from the TARGET - scoped
/// by ship, because the boat's hull shares section ids with the target's.
#[cfg(feature = "debug")]
fn carved_sections_gone() -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(|world: &World| {
        let Some(target) = ship_by_id_ref(world, TARGET_ID) else {
            return false;
        };
        let Some(mut sections) =
            world.try_query_filtered::<(&EntityId, &ChildOf), With<SectionMarker>>()
        else {
            return true;
        };
        !sections.iter(world).any(|(id, parent)| {
            parent.parent() == target && CARVED_SECTIONS.contains(&id.0.as_str())
        })
    })
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
fn torpedo_within(distance: f32) -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(move |world: &World| {
        torpedo_range(world).is_some_and(|range| range < distance)
    })
}

/// How far the closest live torpedo is from the target, if there is one of
/// each.
#[cfg(feature = "debug")]
fn torpedo_range(world: &World) -> Option<f32> {
    let target = ship_by_id_ref(world, TARGET_ID)?;
    let position = world.get::<GlobalTransform>(target)?.translation();
    world
        .try_query_filtered::<&GlobalTransform, With<TorpedoProjectileMarker>>()?
        .iter(world)
        .map(|transform| transform.translation().distance(position))
        .min_by(f32::total_cmp)
}

/// The `Health` node of one of the target's sections (on the section entity
/// or one of its children).
#[cfg(feature = "debug")]
fn target_section_health(world: &mut World, section: &str) -> Option<Entity> {
    let target = ship_by_id(world, TARGET_ID)?;
    let candidates: Vec<Entity> = world
        .query_filtered::<(Entity, &EntityId, &ChildOf), With<SectionMarker>>()
        .iter(world)
        .filter(|(_, id, parent)| id.0 == section && parent.parent() == target)
        .map(|(entity, _, _)| entity)
        .collect();
    let section = candidates.into_iter().next()?;
    if world.get::<Health>(section).is_some() {
        return Some(section);
    }
    let children: Vec<Entity> = world
        .get::<Children>(section)
        .map(|children| children.iter().collect())
        .unwrap_or_default();
    children
        .into_iter()
        .find(|&child| world.get::<Health>(child).is_some())
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
