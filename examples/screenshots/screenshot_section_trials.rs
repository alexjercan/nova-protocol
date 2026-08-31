//! screenshot_section_trials: the remodelled sections under LIVE fire, with
//! the walk failing unless every weapon demonstrably works.
//!
//! The acceptance range for the section remodel (task 20260831-083625). The
//! gallery example shows the promoted MODELS; this range is the other half of
//! the proof: the same prototypes doing their jobs on production paths - the
//! turret assemblies standing joint by joint over their new art, the twin
//! draining its magazine through TWO muzzles, the two-cell bay throwing a
//! torpedo out of its open face - against targets built from the new hull and
//! controller models, loaded through the asset server exactly as the game
//! loads them.
//!
//! ## The range
//!
//! Three firing lanes, all unmanaged (no `WeaponsHot`), so the script fires
//! them by writing the section inputs directly:
//!
//! - gatling lane (x -6): the default PDC on a pedestal, hosing its own
//!   target column 30 units downrange.
//! - twin lane (x +6): the twin PDC doing the same at half the cadence per
//!   muzzle - two offset streams, the same total rate.
//! - bay lane (x 0): a lone two-cell bay (the gauntlet battery's shape) under
//!   a scripted order, launching one torpedo at a soft column far downrange.
//!
//! Each PDC target column stacks the remodel's four looks - tank, personnel,
//! cargo, wired controller - toughened so the burst scars them without
//! deleting the scenery. The bay's column is soft on purpose: the blast
//! erasing its middle section is the pass condition.
//!
//! The walk PROVES, not just shows: each gun step holds until its own lane's
//! column has lost health (rounds fired, flew, and landed), and the bay step
//! holds until the target section is gone (torpedo launched, flew, and
//! killed). A gun that never fires or a bay that launches into its own roof
//! fails the walk by deadline.
//!
//! The walk opens with the STOW round (task 20260831-083622): managed cold,
//! both mounts must fold into their housings behind shut lids; weapons hot,
//! they must rise again - with the trigger HELD and the safety passing the
//! whole way up, so a single round fired before full deployment fails the
//! walk. Only then do the firing lanes open.
//!
//! Two run modes, both under the autopilot (`NOVA_AUTOPILOT`):
//! - `NOVA_AUTOPILOT=1` alone: the smoke path - the full live-fire walk,
//!   capturing nothing.
//! - `NOVA_AUTOPILOT=1 NOVA_CAPTURE=1`: also write the stills (staged under
//!   `NOVA_CAPTURE_DIR`).
//!
//! ```text
//! NOVA_CAPTURE_DIR=target/shots NOVA_AUTOPILOT=1 NOVA_CAPTURE=1 \
//!   cargo run --example screenshot_section_trials --features debug
//! ```

#[cfg(feature = "debug")]
use std::sync::Arc;

use bevy::prelude::*;
use clap::Parser;
use nova_probe::fixtures::{self, prelude::*};
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "screenshot_section_trials")]
#[command(version = "1.0.0")]
#[command(about = "Live-fire the remodelled sections: both PDCs, the two-cell bay, the new hull and controller looks", long_about = None)]
struct Cli;

/// The two PDC lanes, one gun each. Far enough apart that neither stream can
/// scar the other's column (the twin's muzzle offsets are a tenth of this).
const GATLING_LANE_X: f32 = -6.0;
const TWIN_LANE_X: f32 = 6.0;

/// How far each PDC target column stands downrange. Inside the PDC's 200-unit
/// reach by a wide margin, so a round crosses in about a third of a second
/// and a scarred column proves the gun within one short burst.
const TRIAL_RANGE: f32 = 30.0;

/// How far the bay's column stands downrange. Far enough that the 30-unit
/// blast at the column cannot reach back across the diagonal to the PDC
/// columns (that diagonal is ~40 units); near enough the flight fits a step.
const BAY_RANGE: f32 = 70.0;

/// Health each PDC target section is authored to: the burst scars the column
/// instead of deleting the scenery, so every still has a subject.
const TOUGH_SECTION_HEALTH: f32 = 20_000.0;

/// The section id the torpedo must erase for the walk to pass.
const BAY_MARK_ID: &str = "bay_mark";

/// Where the PDC guns aim: each lane's own column centre.
const AIM_HEIGHT: f32 = 0.5;

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();
    let mut app = AppBuilder::new().with_game_plugins(custom_plugin).build();

    #[cfg(feature = "debug")]
    {
        // Probe wiring, the vfx-range pattern: run timeline + engine-bound
        // invariants so `probe run` grades the walk. No frame-time capture -
        // the range proves behaviour, it measures nothing.
        app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
        app.add_systems(
            Startup,
            (force_capture_resolution, hide_dev_overlays, hide_hud),
        );
        app.add_plugins(trials_script());
    }

    app.run()
}

fn custom_plugin(app: &mut App) {
    app.add_systems(OnEnter(GameAssetsStates::Loaded), load_scene);
}

fn load_scene(mut commands: Commands, game_assets: Res<GameAssets>, sections: Res<GameSections>) {
    let objects = vec![
        pdc_stand(
            &sections,
            "gatling_stand",
            "Gatling Stand",
            "pdc_kinetic_turret_section",
            GATLING_LANE_X,
        ),
        pdc_stand(
            &sections,
            "twin_stand",
            "Twin Stand",
            "pdc_twin_kinetic_turret_section",
            TWIN_LANE_X,
        ),
        trial_column(
            &sections,
            "gatling_column",
            "Gatling Column",
            GATLING_LANE_X,
        ),
        trial_column(&sections, "twin_column", "Twin Column", TWIN_LANE_X),
        bay_stand(&sections),
        bay_column(&sections),
    ];
    commands.trigger(LoadScenario(ScenarioConfig {
        description: "Live-fire trials for the remodelled sections.".to_string(),
        events: vec![ScenarioEventConfig {
            label: None,
            name: EventConfig::OnStart,
            once: false,
            filters: vec![],
            actions: [
                objects
                    .into_iter()
                    .map(EventActionConfig::SpawnScenarioObject)
                    .collect::<Vec<_>>(),
                ThreePointRig::around("trials", Vec3::new(0.0, 0.0, -TRIAL_RANGE * 0.5), 3.0)
                    .actions(),
            ]
            .concat(),
        }],
        ..ScenarioConfig::new(
            "section_trials".to_string(),
            "Section Trials".to_string(),
            game_assets.cubemap.clone().into(),
        )
    }));
}

/// One gun on a pedestal: a hull cell with the mount bolted to its roof.
/// Controller-less and unmanaged, so the script owns the trigger.
fn pdc_stand(
    sections: &GameSections,
    id: &str,
    name: &str,
    turret: &str,
    lane_x: f32,
) -> ScenarioObjectConfig {
    let specs = [
        SectionSpec::new("pedestal", REINFORCED_HULL_SECTION_ID, Vec3::ZERO),
        // A mount is half a cell: it mates half its own size above the face.
        SectionSpec::new("gun", turret, Vec3::new(0.0, 0.75, 0.0)),
    ];
    placed(
        id,
        name,
        Vec3::new(lane_x, 0.0, 0.0),
        fixtures::ship(sections, SpaceshipController::None, &specs),
    )
}

/// One target column carrying the remodel's four looks, bottom to top: tank,
/// personnel (the standard hull), cargo, and the wired controller core. All
/// loaded through the asset server - the same path the game loads them by.
fn trial_column(
    sections: &GameSections,
    id: &str,
    name: &str,
    lane_x: f32,
) -> ScenarioObjectConfig {
    let specs = [
        SectionSpec::new("tank", "tank_hull_section", Vec3::new(0.0, -1.0, 0.0)),
        SectionSpec::new("personnel", REINFORCED_HULL_SECTION_ID, Vec3::ZERO),
        SectionSpec::new("cargo", "cargo_hull_section", Vec3::new(0.0, 1.0, 0.0)),
        SectionSpec::new(
            "core",
            BASIC_CONTROLLER_SECTION_ID,
            Vec3::new(0.0, 2.0, 0.0),
        ),
    ];
    toughened(
        id,
        name,
        Vec3::new(lane_x, 0.0, -TRIAL_RANGE),
        fixtures::ship(sections, SpaceshipController::None, &specs),
    )
}

/// The bay lane's gun: a lone two-cell bay, the gauntlet battery's shape.
/// `ScriptedTorpedoOrder` is its whole brain.
fn bay_stand(sections: &GameSections) -> ScenarioObjectConfig {
    let specs = [SectionSpec::new("bay", "torpedo_section", Vec3::ZERO)];
    placed(
        "bay_stand",
        "Bay Stand",
        Vec3::ZERO,
        fixtures::ship(sections, SpaceshipController::None, &specs),
    )
}

/// The bay's column: three SOFT hull cells. The warhead erasing the marked
/// middle one is the walk's pass condition, so nothing here is toughened.
fn bay_column(sections: &GameSections) -> ScenarioObjectConfig {
    let specs = [
        SectionSpec::new(
            "bay_lower",
            LIGHT_HULL_SECTION_ID,
            Vec3::new(0.0, -1.0, 0.0),
        ),
        SectionSpec::new(BAY_MARK_ID, LIGHT_HULL_SECTION_ID, Vec3::ZERO),
        SectionSpec::new("bay_upper", LIGHT_HULL_SECTION_ID, Vec3::new(0.0, 1.0, 0.0)),
    ];
    placed(
        "bay_column",
        "Bay Column",
        Vec3::new(0.0, 0.0, -BAY_RANGE),
        fixtures::ship(sections, SpaceshipController::None, &specs),
    )
}

/// One ship at its park, unmodified.
fn placed(id: &str, name: &str, position: Vec3, ship: SpaceshipConfig) -> ScenarioObjectConfig {
    ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: id.to_string(),
            name: name.to_string(),
            position,
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Spaceship(ship),
    }
}

/// One ship whose sections survive the trial: authored health and no
/// structural collapse, the vfx-range idiom.
fn toughened(
    id: &str,
    name: &str,
    position: Vec3,
    mut ship: SpaceshipConfig,
) -> ScenarioObjectConfig {
    if let ShipSource::Inline(hull) = &mut ship.hull {
        hull.collapse_threshold = Some(0.0);
        for section in &mut hull.sections {
            section.modifications = vec![SectionModification::SetHealth(TOUGH_SECTION_HEALTH)];
        }
    }
    placed(id, name, position, ship)
}

/// The driven walk: stand the range up, fire both guns until both columns are
/// scarred, then order the torpedo and hold until its mark is gone.
#[cfg(feature = "debug")]
fn trials_script() -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
        .step("stand the trials up")
        .enter(GameStates::Loading)
        .until(and(range_standing(), scenario_camera_present()))
        .deadline(30.0)
        .add()
        // THE STOW ROUND (task 20260831-083622). Managing the stands and
        // going weapons-cold makes both mounts fold up, sink and shut their
        // lids; weapons-hot brings them back. The trigger is HELD through
        // the whole deploy with the safety passing (hot) and no aim point
        // (an unaimed turret fires freely), so the ONLY thing that can hold
        // fire is the stow gate - a bullet before full deployment fails the
        // walk.
        .step("weapons cold - both mounts must park in their housings")
        .on_enter(|world| {
            set_weapons_hot(world, false);
            pose_camera(
                world,
                Vec3::new(GATLING_LANE_X + 1.7, 2.3, 2.0),
                Vec3::new(GATLING_LANE_X, 0.85, 0.0),
            );
        })
        .until(mounts_parked())
        .deadline(30.0)
        .add()
        .step("shoot the stowed mount")
        .on_enter(|world| shoot(world, "section-trials-stowed.png"))
        .until(shot_written("section-trials-stowed.png"))
        .deadline(30.0)
        .add()
        .step("weapons hot, trigger held - rising with not one round fired")
        .on_enter(|world| {
            set_weapons_hot(world, true);
            set_triggers(world, true);
        })
        .until(and(mount_rising(), no_bullets()))
        .deadline(15.0)
        .add()
        // The shot asserts its own subject: a frame that lands after the
        // rise finished stalls this beat instead of writing a mislabelled
        // still of the deployed pose.
        .step("shoot the deploy mid-rise")
        .on_enter(|world| shoot(world, "section-trials-deploying.png"))
        .until(and(
            mount_rising(),
            shot_written("section-trials-deploying.png"),
        ))
        .deadline(30.0)
        .add()
        .step("both mounts fully up, trigger released")
        .on_enter(|world| set_triggers(world, false))
        .until(and(mounts_deployed(), no_bullets()))
        .deadline(15.0)
        .add()
        .step("frame the range and lay both guns")
        .on_enter(|world| {
            frame_range(world);
            lay_guns(world);
        })
        .until(frames(12))
        .add()
        .step("guns free - both columns must scar")
        .on_enter(|world| set_triggers(world, true))
        .until(and(lane_scarred(GATLING_LANE_X), lane_scarred(TWIN_LANE_X)))
        .deadline(30.0)
        .add()
        .step("shoot the range mid-burst")
        .on_enter(|world| shoot(world, "section-trials-range.png"))
        .until(shot_written("section-trials-range.png"))
        .deadline(30.0)
        .add()
        .step("close on the twin's two streams")
        .on_enter(|world| {
            pose_camera(
                world,
                Vec3::new(TWIN_LANE_X + 1.9, 1.9, 2.6),
                Vec3::new(TWIN_LANE_X, 1.1, -1.0),
            );
        })
        .until(frames(8))
        .add()
        .step("shoot the twin")
        .on_enter(|world| shoot(world, "section-trials-twin.png"))
        .until(shot_written("section-trials-twin.png"))
        .deadline(30.0)
        .add()
        .step("cease fire, frame the bay")
        .on_enter(|world| {
            set_triggers(world, false);
            pose_camera(world, Vec3::new(2.8, 1.2, 2.2), Vec3::new(0.0, 0.0, -1.5));
        })
        .until(frames(8))
        .add()
        .step("order the torpedo away")
        .on_enter(order_torpedo)
        .until(torpedo_in_flight())
        .deadline(15.0)
        .add()
        .step("shoot the launch")
        .on_enter(|world| shoot(world, "section-trials-launch.png"))
        .until(shot_written("section-trials-launch.png"))
        .deadline(30.0)
        .add()
        .step("the warhead must erase its mark")
        .until(section_gone(BAY_MARK_ID))
        .deadline(45.0)
        .add()
}

/// Everything spawned: three stands and three columns.
#[cfg(feature = "debug")]
fn range_standing() -> Arc<nova_debug::harness::Predicate> {
    Arc::new(|world: &World| {
        world
            .try_query_filtered::<Entity, With<SpaceshipRootMarker>>()
            .is_some_and(|mut query| query.iter(world).count() == 6)
    })
}

/// The wide pose: both firing lanes and their columns in one frame.
#[cfg(feature = "debug")]
fn frame_range(world: &mut World) {
    pose_camera(
        world,
        Vec3::new(11.0, 6.5, 12.0),
        Vec3::new(0.0, 0.5, -TRIAL_RANGE * 0.5),
    );
}

/// Lay each gun on its own lane's column: a commanded POINT, not a target
/// entity - the range wants the barrel where the script says.
#[cfg(feature = "debug")]
fn lay_guns(world: &mut World) {
    let mut query = world.query::<(&mut TurretSectionTargetInput, &GlobalTransform)>();
    for (mut aim, transform) in query.iter_mut(world) {
        let lane = if transform.translation().x < 0.0 {
            GATLING_LANE_X
        } else {
            TWIN_LANE_X
        };
        **aim = Some(Vec3::new(lane, AIM_HEIGHT, -TRIAL_RANGE));
    }
}

/// Hold or release every trigger on the range.
#[cfg(feature = "debug")]
fn set_triggers(world: &mut World, firing: bool) {
    let mut query = world.query::<&mut TurretSectionInput>();
    for mut trigger in query.iter_mut(world) {
        **trigger = firing;
    }
}

/// Manage both PDC stands' weapons state. Inserting `WeaponsHot` is what
/// MAKES them managed: the stow machine (and the fire safety) read an
/// unmanaged ship as hot, so the cold state must be written, not defaulted.
#[cfg(feature = "debug")]
fn set_weapons_hot(world: &mut World, hot: bool) {
    for id in ["gatling_stand", "twin_stand"] {
        if let Some(ship) = ship_by_id(world, id) {
            world.entity_mut(ship).insert(WeaponsHot(hot));
        }
    }
}

/// Every mount on the range is parked: assembly sunk (lift 1) behind shut
/// lids (doors 1). The `seen &&` guard keeps an empty query from passing.
#[cfg(feature = "debug")]
fn mounts_parked() -> Arc<nova_debug::harness::Predicate> {
    Arc::new(|world: &World| {
        world
            .try_query_filtered::<&SectionAnimations, With<TurretSectionMarker>>()
            .is_some_and(|mut query| {
                let mut seen = false;
                let parked = query.iter(world).all(|animations| {
                    seen = true;
                    animations.cue_progress(SectionAnimationCue::StowLift) == Some(1.0)
                        && animations.cue_progress(SectionAnimationCue::StowDoors) == Some(1.0)
                });
                seen && parked
            })
    })
}

/// Some mount is mid-rise: lids fully parted, assembly strictly between
/// sunk and up - the deploying money-shot window.
#[cfg(feature = "debug")]
fn mount_rising() -> Arc<nova_debug::harness::Predicate> {
    Arc::new(|world: &World| {
        world
            .try_query_filtered::<&SectionAnimations, With<TurretSectionMarker>>()
            .is_some_and(|mut query| {
                query.iter(world).any(|animations| {
                    animations.cue_progress(SectionAnimationCue::StowDoors) == Some(0.0)
                        && animations
                            .cue_progress(SectionAnimationCue::StowLift)
                            .is_some_and(|progress| progress > 0.0 && progress < 1.0)
                })
            })
    })
}

/// Every mount is fully up: both stow cues back at rest.
#[cfg(feature = "debug")]
fn mounts_deployed() -> Arc<nova_debug::harness::Predicate> {
    Arc::new(|world: &World| {
        world
            .try_query_filtered::<&SectionAnimations, With<TurretSectionMarker>>()
            .is_some_and(|mut query| {
                let mut seen = false;
                let deployed = query.iter(world).all(|animations| {
                    seen = true;
                    animations.cue_progress(SectionAnimationCue::StowLift) == Some(0.0)
                        && animations.cue_progress(SectionAnimationCue::StowDoors) == Some(0.0)
                });
                seen && deployed
            })
    })
}

/// Not one turret round exists in the world. Rounds live 2 s, so this also
/// rules out a bullet that leaked EARLIER in the held-trigger window.
#[cfg(feature = "debug")]
fn no_bullets() -> Arc<nova_debug::harness::Predicate> {
    Arc::new(|world: &World| {
        world
            .try_query_filtered::<Entity, With<TurretBulletProjectileMarker>>()
            .is_none_or(|mut query| query.iter(world).next().is_none())
    })
}

/// Order one torpedo out of the bay, committed to the bay column.
#[cfg(feature = "debug")]
fn order_torpedo(world: &mut World) {
    let Some(target) = ship_by_id(world, "bay_column") else {
        return;
    };
    let bays: Vec<Entity> = world
        .query_filtered::<Entity, With<TorpedoSectionConfigHelper>>()
        .iter(world)
        .collect();
    for bay in bays {
        world
            .entity_mut(bay)
            .insert(ScriptedTorpedoOrder { target });
    }
}

/// A ship root, by the scenario id it was spawned under.
#[cfg(feature = "debug")]
fn ship_by_id(world: &mut World, id: &str) -> Option<Entity> {
    world
        .query_filtered::<(Entity, &EntityId), With<SpaceshipRootMarker>>()
        .iter(world)
        .find(|(_, entity_id)| ***entity_id == *id)
        .map(|(entity, _)| entity)
}

/// Some section in `lane_x`'s target zone has lost health: rounds were fired,
/// crossed the range and LANDED. The z gate keeps a scarred stand (there is
/// no way to scar one, but the claim should not depend on that) from passing
/// the wrong lane.
#[cfg(feature = "debug")]
fn lane_scarred(lane_x: f32) -> Arc<nova_debug::harness::Predicate> {
    Arc::new(move |world: &World| {
        world
            .try_query_filtered::<(&Health, &GlobalTransform), With<SectionMarker>>()
            .is_some_and(|mut query| {
                query.iter(world).any(|(health, transform)| {
                    let at = transform.translation();
                    health.current < health.max
                        && (at.x - lane_x).abs() < 3.0
                        && at.z < -TRIAL_RANGE * 0.5
                })
            })
    })
}

/// A torpedo is in the world: the bay's order left the tube.
#[cfg(feature = "debug")]
fn torpedo_in_flight() -> Arc<nova_debug::harness::Predicate> {
    Arc::new(|world: &World| {
        world
            .try_query_filtered::<Entity, With<TorpedoControllerMarker>>()
            .is_some_and(|mut query| query.iter(world).next().is_some())
    })
}
