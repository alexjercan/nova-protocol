//! com_range: verify mass properties follow section destruction, live.
//!
//! Task 20260709-140620: in play, a ship that loses sections appears to keep
//! spinning around the old full-ship center of mass - a pivot outside the
//! surviving structure. The headless physics tests say avian's
//! `ComputedCenterOfMass` follows destroyed sections, so this range checks the
//! same claim in the real app, visibly.
//!
//! One player ship, five sections in a line along -Z:
//! controller(0), hull(1), hull(2), hull(3), thruster(4).
//!
//! Gizmos, drawn every frame:
//! - RED sphere: avian's live center of mass (root transform * ComputedCenterOfMass).
//! - GREEN sphere: mass-weighted centroid of the sections still attached
//!   (any health state) - where the COM *should* be.
//! - GRAY sphere: where the full ship's COM was at spawn, carried along with
//!   the hull. If the ship seems to pivot around GRAY after losing sections,
//!   the stale-COM bug is real; if RED and GREEN stay glued together and the
//!   spin pivots there, physics is healthy.
//!
//! Controls:
//! - O: give the ship a spin (angular velocity impulse).
//! - K: kill the frontmost surviving section (exact-health damage, so the
//!   real disable -> destroy -> despawn pipeline runs).
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//! ```text
//! NOVA_AUTOPILOT=1 cargo run --example com_range --features debug
//! # scripted beats: wait for the ship to spawn, spin it, kill the controller
//! # and wait for it to despawn, kill hull(1) and wait for the same, settle,
//! # then assert that the live COM sits on the attached-section centroid, has
//! # moved aft from the spawn COM, and that the chase camera anchor tracks it.
//! # Every beat waits on the world, not the clock, so a slow load delays the
//! # walk instead of truncating it. Exits non-zero on a stale COM, a phantom
//! # camera anchor, and on a beat that never resolves inside its deadline -
//! # which the harness reports by NAME.
//! ```

use avian3d::prelude::*;
use bevy::{color::palettes::tailwind, platform::collections::HashMap, prelude::*};
use clap::Parser;
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "com_range")]
#[command(version = "1.0.0")]
#[command(about = "A test range for mass properties under section destruction", long_about = None)]
struct Cli;

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();
    let mut app = AppBuilder::new().with_game_plugins(custom_plugin).build();

    #[cfg(feature = "debug")]
    {
        // Probe wiring (task 20260719-210443; each plugin is inert without
        // its NOVA_PERF_* env): run timeline + engine-bound invariants +
        // frame-time capture, so `probe run` can measure this example.
        app.add_plugins(nova_probe::nova_timeline());
        app.add_plugins(nova_probe::nova_invariants());
        app.add_plugins(nova_probe::nova_frametime());
        // Not the stock nova_autopilot(): the script is its own beat list,
        // and every beat waits on what has to HAPPEN rather than on a guessed
        // duration - the ship spawns, the section despawns - so a slow load
        // delays the walk instead of truncating it. The per-step deadlines are
        // what the 30s runway used to be, and they name the beat that stalled
        // instead of reporting a generic timeout. Keep their sum well under
        // DEFAULT_DEADLINE_SECS (120s) so a named stall wins the race against
        // the generic collector deadline.
        app.add_plugins(
            nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
                .step("load the range")
                .enter(GameStates::Loading)
                .until(player_ship_present())
                .deadline(30.0)
                .add()
                .step("spin the ship")
                .on_enter(apply_spin)
                .until(elapsed(1.0))
                .add()
                .step("kill the controller section")
                .on_enter(kill_frontmost_section)
                .until(section_gone("controller"))
                .deadline(10.0)
                .add()
                .step("kill the first hull section")
                .on_enter(kill_frontmost_section)
                .until(section_gone("hull1"))
                .deadline(10.0)
                .add()
                // The mass properties and the camera anchor both need a beat
                // to follow the despawn before they are worth reading.
                .step("settle after the losses")
                .until(elapsed(1.5))
                .add()
                // Last beat: the driver reports done after it, so the run ends
                // on the assertion rather than idling out a runway.
                .step("assert the com follows the surviving sections")
                .on_enter(assert_com_follows_sections)
                .add(),
        );
        app.add_plugins(nova_screenshot());
    }

    app.run()
}

fn custom_plugin(app: &mut App) {
    app.add_systems(OnEnter(GameAssetsStates::Loaded), setup_range);
    app.add_systems(Update, (com_range_hotkeys, draw_com_gizmos, log_com_status));
}

fn setup_range(mut commands: Commands, game_assets: Res<GameAssets>, sections: Res<GameSections>) {
    commands.trigger(LoadScenario(com_range(&game_assets, &sections)));
}

/// Build the range scenario: a single player ship, five sections in a line.
fn com_range(game_assets: &GameAssets, sections: &GameSections) -> ScenarioConfig {
    let section = |id: &str| {
        sections
            .get_section(id)
            .unwrap_or_else(|| panic!("section '{id}' not found"))
            .clone()
    };
    let at = |id: &str, kind: &str, z: f32| SpaceshipSectionConfig {
        id: id.to_string(),
        position: Vec3::new(0.0, 0.0, z),
        rotation: Quat::IDENTITY,
        source: SectionSource::Inline(section(kind)),
        modifications: vec![],
    };

    let ship = SpaceshipConfig {
        allegiance: None,
        controller: SpaceshipController::Player(PlayerControllerConfig {
            input_mapping: HashMap::new(),
            speed_cap: None,
            // Dev/tuning harness: fire freely.
            infinite_ammo: true,
            lock_refire_secs: None,
        }),
        sections: vec![
            at("controller", "basic_controller_section", 0.0),
            at("hull1", "reinforced_hull_section", 1.0),
            at("hull2", "reinforced_hull_section", 2.0),
            at("hull3", "reinforced_hull_section", 3.0),
            at("thruster", "basic_thruster_section", 4.0),
        ],
    };

    let events = vec![ScenarioEventConfig {
        name: EventConfig::OnStart,
        filters: vec![],
        actions: vec![EventActionConfig::SpawnScenarioObject(
            ScenarioObjectConfig {
                base: BaseScenarioObjectConfig {
                    id: "player_ship".to_string(),
                    name: "COM Test Ship".to_string(),
                    position: Vec3::ZERO,
                    rotation: Quat::IDENTITY,
                },
                kind: ScenarioObjectKind::Spaceship(ship),
            },
        )],
    }];

    ScenarioConfig {
        id: "com_range".to_string(),
        name: "COM Range".to_string(),
        description: "A test range for mass properties under section destruction.".to_string(),
        cubemap: game_assets.cubemap.clone().into(),
        events,
        ..Default::default()
    }
}

/// The player ship root, if it exists.
fn player_root(world: &mut World) -> Option<Entity> {
    world
        .query_filtered::<Entity, (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>)>()
        .iter(world)
        .next()
}

/// Give the ship a healthy tumble so the pivot is visible.
fn apply_spin(world: &mut World) {
    let Some(root) = player_root(world) else {
        return;
    };
    world
        .entity_mut(root)
        .insert(AngularVelocity(Vec3::new(0.4, 1.5, 0.2)));
    info!("com range: spin applied to {root:?}");
}

/// Kill the frontmost (lowest local z) surviving section with exact-health
/// damage, so the real disable -> destroy -> despawn pipeline runs. Exact, not
/// overkill: HealthApplyDamage propagates through ChildOf, and overkill would
/// also zero the ship root's aggregate health.
fn kill_frontmost_section(world: &mut World) {
    let Some(root) = player_root(world) else {
        return;
    };
    let mut sections: Vec<(Entity, f32, f32)> = world
        .query_filtered::<(Entity, &ChildOf, &Transform, &Health), With<SectionMarker>>()
        .iter(world)
        .filter(|(_, ChildOf(parent), _, health)| *parent == root && health.current > 0.0)
        .map(|(entity, _, transform, health)| (entity, transform.translation.z, health.current))
        .collect();
    sections.sort_by(|a, b| a.1.total_cmp(&b.1));
    let Some(&(section, z, health)) = sections.first() else {
        info!("com range: no surviving section left to kill");
        return;
    };
    info!("com range: killing section {section:?} at z={z} ({health} hp)");
    world.trigger(HealthApplyDamage {
        entity: section,
        source: None,
        amount: health,
    });
}

/// Live COM (world), attached-section centroid (world), and the ship root's
/// GlobalTransform, for the smoke assertion.
#[cfg(feature = "debug")]
fn com_snapshot(world: &mut World) -> Option<(Vec3, Vec3, GlobalTransform)> {
    let root = player_root(world)?;
    let (root_gt, com) = {
        let entity = world.entity(root);
        (
            *entity.get::<GlobalTransform>()?,
            entity.get::<ComputedCenterOfMass>()?.0,
        )
    };
    let world_com = root_gt.transform_point(com);
    let mut acc = Vec3::ZERO;
    let mut total = 0.0;
    for (ChildOf(parent), gt, density) in world
        .query_filtered::<(&ChildOf, &GlobalTransform, &ColliderDensity), With<SectionMarker>>()
        .iter(world)
    {
        if *parent == root {
            acc += gt.translation() * density.0;
            total += density.0;
        }
    }
    (total > 0.0).then(|| (world_com, acc / total, root_gt))
}

/// Interactive controls: O = spin, K = kill the frontmost surviving section.
fn com_range_hotkeys(world: &mut World) {
    let input = world.resource::<ButtonInput<KeyCode>>();
    let spin = input.just_pressed(KeyCode::KeyO);
    let kill = input.just_pressed(KeyCode::KeyK);
    if spin {
        apply_spin(world);
    }
    if kill {
        kill_frontmost_section(world);
    }
}

/// RED = avian's live COM, GREEN = attached-section centroid, GRAY = the
/// spawn-time COM carried along with the hull.
#[allow(clippy::type_complexity)]
fn draw_com_gizmos(
    mut gizmos: Gizmos,
    q_root: Query<
        (
            Entity,
            &GlobalTransform,
            &ComputedCenterOfMass,
            &ComputedMass,
        ),
        (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>),
    >,
    q_sections: Query<(&ChildOf, &GlobalTransform, &ColliderDensity), With<SectionMarker>>,
    mut frozen: Local<Option<Vec3>>,
) {
    let Ok((root, root_gt, com, mass)) = q_root.single() else {
        return;
    };
    let world_com = root_gt.transform_point(com.0);
    let mut acc = Vec3::ZERO;
    let mut total = 0.0;
    for (&ChildOf(parent), gt, density) in &q_sections {
        if parent == root {
            acc += gt.translation() * density.0;
            total += density.0;
        }
    }
    if total <= 0.0 {
        return;
    }
    let centroid = acc / total;

    gizmos.sphere(
        Isometry3d::from_translation(world_com),
        0.35,
        tailwind::RED_500,
    );
    gizmos.sphere(
        Isometry3d::from_translation(centroid),
        0.30,
        tailwind::GREEN_500,
    );

    // Freeze the spawn-time LOCAL com once avian has settled real mass
    // properties; freezing the first matched frame could pin the marker to the
    // pre-settle default and defeat its purpose.
    if frozen.is_none() && com.0.is_finite() && mass.value() > 0.5 {
        *frozen = Some(com.0);
    }
    if let Some(frozen_local) = *frozen {
        gizmos.sphere(
            Isometry3d::from_translation(root_gt.transform_point(frozen_local)),
            0.25,
            tailwind::GRAY_500,
        );
    }
}

/// One status line per second: live mass and the COM-vs-centroid distance.
#[allow(clippy::type_complexity)]
fn log_com_status(
    time: Res<Time>,
    mut last: Local<f32>,
    q_root: Query<
        (
            Entity,
            &GlobalTransform,
            &ComputedCenterOfMass,
            &ComputedMass,
        ),
        (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>),
    >,
    q_sections: Query<(&ChildOf, &GlobalTransform, &ColliderDensity), With<SectionMarker>>,
) {
    if time.elapsed_secs() - *last < 1.0 {
        return;
    }
    *last = time.elapsed_secs();
    let Ok((root, root_gt, com, mass)) = q_root.single() else {
        return;
    };
    let world_com = root_gt.transform_point(com.0);
    let mut acc = Vec3::ZERO;
    let mut total = 0.0;
    for (&ChildOf(parent), gt, density) in &q_sections {
        if parent == root {
            acc += gt.translation() * density.0;
            total += density.0;
        }
    }
    if total <= 0.0 {
        return;
    }
    let centroid = acc / total;
    info!(
        "com range: mass={:.2} com={world_com:.2?} centroid={centroid:.2?} drift={:.3}",
        mass.value(),
        world_com.distance(centroid)
    );
}

/// The script's last beat: assert the live COM sits on the attached-section
/// centroid, has moved aft, and that the chase camera anchor tracks it.
///
/// A world action rather than a timed branch: the driver runs it on entry to
/// the final step, which the beats before it have already established the
/// preconditions for (the ship exists, two sections are gone, the physics has
/// settled).
#[cfg(feature = "debug")]
fn assert_com_follows_sections(world: &mut World) {
    let Some((world_com, centroid, root_gt)) = com_snapshot(world) else {
        panic!("com range: ship vanished before the assertion");
    };
    let drift = world_com.distance(centroid);
    let local_com = root_gt.affine().inverse().transform_point3(world_com);
    info!("com range: assert drift={drift:.3} local_com={local_com:.2?} (spawn com z=2.0)");
    assert!(
        drift < 0.3,
        "com range: live COM is {drift:.2} away from the attached-section \
         centroid - avian's COM is stale in the real app"
    );
    assert!(
        local_com.z > 2.4,
        "com range: local COM {local_com:.2?} did not move aft after losing \
         the two front sections (expected z near 2.75)"
    );
    // The ship must interpolate its Transform between fixed ticks, or it
    // stair-steps under the smoothed camera (task 20260709-160753).
    let root = player_root(world).expect("player root exists at assert time");
    assert!(
        world.entity(root).contains::<TransformInterpolation>(),
        "com range: the ship root lost TransformInterpolation - camera twitch returns"
    );
    // The chase camera must orbit the physical pivot too, or a tumbling ship
    // appears to spin around the empty space at the root origin. The camera is
    // mandatory here: a broken query must fail, not skip.
    let anchor = world
        .query_filtered::<&ChaseCameraInput, With<ChaseCamera>>()
        .iter(world)
        .next()
        .map(|input| input.anchor_pos)
        .expect("com range: no chase camera input found at assert time");
    let cam_drift = anchor.distance(world_com);
    assert!(
        cam_drift < 0.5,
        "com range: chase camera anchor {anchor:.2?} is {cam_drift:.2} \
         away from the live COM - the camera orbits a phantom point"
    );
    info!("com range: camera anchor tracks the COM (drift {cam_drift:.3})");
    // Timeline beat (task 20260719-210450): COM and camera both tracked the
    // section loss; drifts on the record.
    let elapsed = world.resource::<Time>().elapsed_secs();
    nova_probe::probe_marker(
        world,
        "outcome: com + camera track section loss",
        serde_json::json!({ "t": elapsed, "com_drift": drift, "camera_drift": cam_drift }),
    );
    info!("com range: PASS - COM follows the surviving sections");
}
