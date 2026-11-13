//! news_0180_railgun_impact: the v0.18.0 news railgun loop - a lance slug
//! landing on a hull, slowed to a tenth: the steel-blue flash at the entry
//! face and the spall burst, with the wake haze still on the line of flight.
//!
//! A player gunboat with a lance on its spine stands at the origin and a base
//! Raider stands [`TARGET_RANGE`] down its bore, bow-on. The whole range is
//! yawed [`BORE_YAW`] off world -Z: a flash or a haze that follows the bore
//! here, and not only down -Z, is one whose vectors reach the particle system
//! in the weapon's frame.
//!
//! The walk raises the combat stance, commits the shot with a
//! [`ScriptedRailgunOrder`] and lets the charge run at real speed. At
//! [`LOOP_OPEN_CHARGE`] it opens the loop and slows gameplay time to
//! [`SHOT_TIME_SCALE`]; the slug's whole flight is under two fixed steps, so
//! the slow clock spreads the landing and what follows it, not the crossing.
//! The loop holds [`AFTERMATH_SECS`] of world time after the shot.
//!
//! Two run modes, both under the autopilot (`NOVA_AUTOPILOT`):
//! - `NOVA_AUTOPILOT=1` alone: the smoke path - the walk and its shot,
//!   recording nothing.
//! - `NOVA_AUTOPILOT=1 NOVA_CAPTURE=1`: also record
//!   `news-0180-railgun-impact.webm` (staged under `NOVA_CAPTURE_DIR`).
//!
//! ```text
//! NOVA_CAPTURE_DIR=target/news-0180-shots NOVA_AUTOPILOT=1 NOVA_CAPTURE=1 \
//!   cargo run --example news_0180_railgun_impact --features debug
//! # look for: `railgun impact: PASS the lance fired down a bore off world -Z`
//! ```

use std::sync::Arc;

use bevy::prelude::*;
use clap::Parser;
use nova_protocol::{nova_debug::harness::Predicate, prelude::*};

#[path = "shared/kit.rs"]
mod kit;

#[derive(Parser)]
#[command(name = "news_0180_railgun_impact")]
#[command(version = "1.0.0")]
#[command(
    about = "Record the v0.18.0 railgun loop: a slug landing on a hull, slowed to a tenth",
    long_about = None
)]
struct Cli;

/// The scenario id the range loads under.
const RANGE_ID: &str = "railgun_impact_range";

/// Scenario id of the gunboat that fires, and the id of the lance on its spine.
const BOAT_ID: &str = "lance_boat";
const LANCE_ID: &str = "lance";

/// Scenario id of the hull that is shot at, and its catalog hull: a base
/// Raider, about 70 m from nose to drives.
const TARGET_ID: &str = "target_raider";
const TARGET_HULL: &str = "raider";

/// How far down the bore the target stands.
const TARGET_RANGE: Meters = Meters(220.0);

/// The yaw of the whole range about +Y, in radians: the bore runs about 115
/// degrees off world -Z.
const BORE_YAW: f32 = 2.0;

/// Where the eye stands and what it looks at, in the range's own frame (bore
/// down -Z), in meters: off the target's bow, with the last of the line of
/// flight in the frame.
const EYE: Vec3 = Vec3::new(78.0, 22.0, -138.0);
const LOOK: Vec3 = Vec3::new(0.0, 0.0, -196.0);

/// Charge fraction the recording opens at: the lance charges for 1.5 s, so
/// the loop opens on a whole hull a moment before the shot.
const LOOP_OPEN_CHARGE: f32 = 0.9;

/// How fast gameplay time runs while the loop records.
const SHOT_TIME_SCALE: f32 = 0.1;

/// Seconds of world time the loop holds after the slug is away: five
/// seconds of footage at [`SHOT_TIME_SCALE`].
const AFTERMATH_SECS: f32 = 0.5;

/// The loop this producer records.
const LOOP_NAME: &str = "news-0180-railgun-impact";

/// A point of the range's own frame, in the world.
fn on_range(local: Vec3) -> Meters3 {
    Meters3(range_rotation() * local)
}

/// The rotation that takes the range's own frame to the world.
fn range_rotation() -> Quat {
    Quat::from_rotation_y(BORE_YAW)
}

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();
    let mut app = AppBuilder::new().with_game_plugins(custom_plugin).build();

    app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
    app.add_plugins(nova_protocol::nova_debug::harness::LoopCapturePlugin::default());
    app.init_resource::<RangeProbe>();
    app.add_observer(count_shots);
    app.add_systems(
        Startup,
        (force_capture_resolution, hide_dev_overlays, hide_hud),
    );
    // The recoil is one impulse on a free hull. Holding the gunboat keeps the
    // bore on the target through the charge; the slug, the debris and the
    // target stay free.
    app.add_systems(Update, hold_the_boat);
    app.add_plugins(range_script());
    app.run()
}

fn custom_plugin(app: &mut App) {
    app.add_systems(OnEnter(GameAssetsStates::Loaded), setup_range);
}

fn setup_range(
    mut commands: Commands,
    game_assets: Res<GameAssets>,
    sections: Res<GameSections>,
    ships: Res<GameShipDesigns>,
) {
    commands.trigger(LoadScenario(lance_range(&game_assets, &sections, &ships)));
}

/// The gunboat of `screenshot_railgun`: a lance on the spine of a thirteen
/// section hull.
fn boat_hull(sections: &GameSections) -> Vec<SpaceshipSectionConfig> {
    let section = |id: &str| {
        sections
            .get_section(id)
            .unwrap_or_else(|| panic!("section '{id}' not found"))
            .clone()
    };
    let at = |id: &str, kind: &str, position: Vec3| SpaceshipSectionConfig {
        id: id.to_string(),
        position,
        rotation: Quat::IDENTITY,
        source: SectionSource::Inline(section(kind)),
    };
    let hull = |id: &str, position: Vec3| at(id, REINFORCED_HULL_SECTION_ID, position);

    vec![
        at(
            LANCE_ID,
            RAILGUN_LANCE_SECTION_ID,
            Vec3::new(0.0, 0.0, -1.0),
        ),
        hull("mount_port", Vec3::new(-1.0, 0.0, 0.0)),
        hull("mount_starboard", Vec3::new(1.0, 0.0, 0.0)),
        hull("spine_fore", Vec3::new(0.0, 0.0, 1.0)),
        hull("waist_port", Vec3::new(-1.0, 0.0, 1.0)),
        hull("waist_starboard", Vec3::new(1.0, 0.0, 1.0)),
        at(
            "bridge",
            BASIC_CONTROLLER_SECTION_ID,
            Vec3::new(0.0, 0.0, 2.0),
        ),
        hull("flank_port", Vec3::new(-1.0, 0.0, 2.0)),
        hull("flank_starboard", Vec3::new(1.0, 0.0, 2.0)),
        hull("dorsal", Vec3::new(0.0, 1.0, 2.0)),
        hull("spine_aft", Vec3::new(0.0, 0.0, 3.0)),
        at(
            "drive_port",
            BASIC_THRUSTER_SECTION_ID,
            Vec3::new(-1.0, 0.0, 3.0),
        ),
        at(
            "drive_starboard",
            BASIC_THRUSTER_SECTION_ID,
            Vec3::new(1.0, 0.0, 3.0),
        ),
    ]
}

fn lance_range(
    game_assets: &GameAssets,
    sections: &GameSections,
    ships: &GameShipDesigns,
) -> ScenarioConfig {
    let boat = ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: BOAT_ID.to_string(),
            name: "Lance Gunboat".to_string(),
            position: Meters3::ZERO,
            rotation: range_rotation(),
        },
        kind: ScenarioObjectKind::Spaceship(SpaceshipConfig {
            allegiance: Some(Allegiance::Player),
            controller: SpaceshipController::Player(PlayerControllerConfig {
                input_mapping: std::collections::BTreeMap::new(),
            }),
            design: ShipDesignSource::Inline(ShipDesign {
                sections: boat_hull(sections),
                ..default()
            }),
            ..default()
        }),
    };

    let target = ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: TARGET_ID.to_string(),
            name: "Target Raider".to_string(),
            position: on_range(Vec3::new(0.0, 0.0, -TARGET_RANGE.get())),
            // Bow-on: the slug rakes the long axis.
            rotation: range_rotation() * Quat::from_rotation_y(std::f32::consts::PI),
        },
        kind: ScenarioObjectKind::Spaceship(SpaceshipConfig {
            allegiance: Some(Allegiance::Enemy),
            controller: SpaceshipController::None,
            design: ShipDesignSource::Inline(kit::catalog_ship(ships, TARGET_HULL)),
            ..default()
        }),
    };

    ScenarioConfig {
        description: "A gunboat with a spinal lance and a hull down its bore, off world -Z."
            .to_string(),
        events: vec![ScenarioEventConfig {
            label: None,
            position: None,
            name: EventConfig::OnStart,
            once: false,
            filters: vec![],
            actions: [
                vec![
                    EventActionConfig::SpawnScenarioObject(boat),
                    EventActionConfig::SpawnScenarioObject(target),
                ],
                ThreePointRig::around("range", Meters3::ZERO, 6.0).actions(),
            ]
            .concat(),
        }],
        ..ScenarioConfig::new(
            RANGE_ID.to_string(),
            "Railgun Impact Range".to_string(),
            game_assets.cubemap.clone().into(),
        )
    }
}

/// How many shots the lance has fired.
#[derive(Resource, Default)]
struct RangeProbe {
    shots: u32,
}

fn count_shots(_: On<RailgunFired>, mut probe: ResMut<RangeProbe>) {
    probe.shots += 1;
}

/// Hold the gunboat at the origin on the range's bore.
fn hold_the_boat(
    mut boat: Query<
        (
            &mut Transform,
            &mut avian3d::prelude::LinearVelocity,
            &mut avian3d::prelude::AngularVelocity,
        ),
        (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>),
    >,
) {
    for (mut transform, mut linear, mut angular) in &mut boat {
        transform.translation = Vec3::ZERO;
        transform.rotation = range_rotation();
        linear.0 = Vec3::ZERO;
        angular.0 = Vec3::ZERO;
    }
}

/// The range's lance.
fn range_lance(world: &mut World) -> Option<Entity> {
    world
        .try_query_filtered::<(Entity, &EntityId), With<RailgunSectionMarker>>()?
        .iter(world)
        .find(|(_, id)| id.0 == LANCE_ID)
        .map(|(entity, _)| entity)
}

fn lance_present() -> Arc<Predicate> {
    Arc::new(|world: &World| {
        world
            .try_query_filtered::<&EntityId, With<RailgunSectionMarker>>()
            .is_some_and(|mut query| query.iter(world).any(|id| id.0 == LANCE_ID))
    })
}

/// The lance's charge has run to `fraction` of its length.
fn charge_at_least(fraction: f32) -> Arc<Predicate> {
    Arc::new(move |world: &World| {
        let Some(mut query) = world.try_query_filtered::<(
            &EntityId,
            &RailgunCharge,
            &RailgunSectionConfigHelper,
        ), With<RailgunSectionMarker>>() else {
            return false;
        };
        query.iter(world).any(|(id, charge, config)| {
            id.0 == LANCE_ID && charge.progress(config.charge_seconds) >= fraction
        })
    })
}

fn shot_away() -> Arc<Predicate> {
    Arc::new(|world: &World| world.resource::<RangeProbe>().shots >= 1)
}

fn set_time_scale(world: &mut World, scale: f32) {
    world
        .resource_mut::<Time<Virtual>>()
        .set_relative_speed(scale);
}

/// Panic unless the bore the lance stands on is off world -Z.
fn assert_the_bore(world: &mut World) {
    let lance = range_lance(world).expect("railgun impact: the range has its lance");
    let bore = world
        .get::<GlobalTransform>(lance)
        .expect("railgun impact: the lance has a transform")
        .rotation()
        * Vec3::NEG_Z;
    let off = bore.angle_between(Vec3::NEG_Z).to_degrees();
    assert!(
        off > 45.0,
        "railgun impact: the bore runs {off:.0} degrees off world -Z; the range must not face it"
    );
    info!("railgun impact: PASS the lance fired down a bore {off:.0} degrees off world -Z");
}

fn range_script() -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
        .step("wait for the range")
        .enter(GameStates::Loading)
        .until(and(
            state_is(GameStates::Playing),
            and(scenario_camera_present(), lance_present()),
        ))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("raise the weapons and frame the target's bow")
        .on_enter(|world: &mut World| {
            press_action("combat_stance")(world);
            pose_camera(world, on_range(EYE), on_range(LOOK));
        })
        .until(frames(SETTLE_FRAMES))
        .add()
        .step("commit the shot")
        .on_enter(|world: &mut World| {
            let lance = range_lance(world).expect("railgun impact: the range has its lance");
            world.entity_mut(lance).insert(ScriptedRailgunOrder);
        })
        .until(charge_at_least(LOOP_OPEN_CHARGE))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("open the loop and slow the clock")
        .on_enter(|world: &mut World| {
            loop_start(world, LOOP_NAME);
            set_time_scale(world, SHOT_TIME_SCALE);
        })
        .until(shot_away())
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("watch the slug land")
        .on_enter(assert_the_bore)
        .until(elapsed(AFTERMATH_SECS))
        .add()
        .step("close the loop")
        .on_enter(|world: &mut World| {
            loop_end(world, LOOP_NAME);
            set_time_scale(world, 1.0);
        })
        .until(loop_written(LOOP_NAME))
        .deadline(120.0)
        .add()
}
