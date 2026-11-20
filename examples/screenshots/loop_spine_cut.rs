//! loop_spine_cut: the `spine-cut` webm loop - an interceptor's port wing tip
//! is cut through, and the freed gun mount and drive sever and drift away as
//! independent wrecks.
//!
//! The moving version of the section-severing invariant
//! (`examples/systems/section_severing.rs`), staged on a real catalog hull:
//! killing the interceptor's port wing-tip plate turns it into a genuine hole, and
//! the mount and the drive that were held on through it disconnect, get their
//! own rigid bodies and drift free (`nova_ship::sections::integrity`). The loop opens on the
//! intact hull turning slowly, holds through the cut, and rides the freed
//! structure out to a calm drifting tail.
//!
//! Authored in the one capture idiom: an autopilot script whose steps call
//! `loop_start` / `loop_end` (`nova_autopilot::loops`), same file smoke and
//! capture. The cut is the production damage path ([`HealthApplyDamage`]);
//! the sever is the production integrity pipeline - the script asserts a
//! [`ShipWreckFragmentMarker`] actually appears, so a run that fails to sever
//! aborts by name instead of recording a loop of nothing. The drift is the
//! production sever kick and the hull's own inherited spin, plus one scripted
//! nudge per fragment.
//!
//! Two run modes, both under the autopilot (`NOVA_AUTOPILOT`):
//! - `NOVA_AUTOPILOT=1` alone: the smoke path - the full walk, recording
//!   nothing.
//! - `NOVA_AUTOPILOT=1 NOVA_CAPTURE=1`: record and encode the loop into
//!   `NOVA_CAPTURE_DIR/spine-cut.webm`.
//!
//! Capture:
//! ```text
//! NOVA_CAPTURE_DIR=target/loop-shots NOVA_AUTOPILOT=1 NOVA_CAPTURE=1 \
//!   cargo run --example loop_spine_cut --features debug
//! ```

#[path = "shared/kit.rs"]
mod kit;

use bevy::prelude::*;
use clap::Parser;
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "loop_spine_cut")]
#[command(version = "1.0.0")]
#[command(about = "The spine-cut webm loop: a severed gun mount drifts off an interceptor. Autopilot-only: every actor is scripted or inert", long_about = None)]
struct Cli;

/// The loop this example records - the webm's file stem.
#[cfg(feature = "debug")]
const LOOP_NAME: &str = "spine-cut";

/// Scenario id of the interceptor that loses its port wing tip.
const SUBJECT_ID: &str = "loop_subject";

/// The catalog hull the cut is made in.
const SUBJECT_HULL: &str = "interceptor";

/// The BUILD-GRID CELL the cut goes through: the port wing-tip plate, which is
/// the seat the port mount stands on and the only link of the mount and of
/// the wing-tip drive behind it, so cutting it severs the gun and the drive
/// as independent wrecks. A plate with nothing hanging off it frees nothing.
///
/// Named by cell rather than by id because a block hull's `plate_N` numbering
/// is an artifact of the order its boxes were unioned (see
/// [`kit::cell_section`]); the cell is the coordinate the hull is authored in.
#[cfg(feature = "debug")]
const CUT_CELL: Vec3 = Vec3::new(-3.0, 0.0, 1.0);

/// Where the lens stands off [`CUT_CELL`], in the HULL's own frame, meters.
///
/// High off the starboard quarter, looking across the aft deck at the port
/// wing tip. The freed mount and drive leave to port at about 35 m/s, the
/// sever kick plus the scripted nudge; a lens astern of the wing tip has them
/// out of frame inside two seconds. From here they cross the frame and are
/// still in it when the loop closes.
///
/// Read in the hull's frame because the subject is authored with a yaw; a
/// pose in world axes lands on whichever part of the hull that yaw turns
/// toward it.
#[cfg(feature = "debug")]
const CUT_EYE: Vec3 = Vec3::new(75.0, 75.0, 155.0);

/// What the lens aims at, as an offset from [`CUT_CELL`] in the hull's frame,
/// meters: outboard and aft of the cut, so the hull sits in the right half of
/// the frame and the left half is open for the drift.
#[cfg(feature = "debug")]
const CUT_AIM: Vec3 = Vec3::new(-25.0, 10.0, 20.0);

/// The slow tumble the hull carries into the cut, so the freed structure
/// inherits real motion instead of hanging dead in frame.
#[cfg(feature = "debug")]
const SUBJECT_SPIN: Vec3 = Vec3::new(0.0, 0.10, 0.04);

/// Scripted send-off speed per freed fragment, on top of the sever kick and
/// the inherited spin. Added straight to an avian `LinearVelocity`, so it is
/// an engine world-unit figure (world units per second).
#[cfg(feature = "debug")]
const FRAGMENT_DRIFT_SPEED: f32 = 1.1;

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
                .until(subject_present())
                .deadline(60.0)
                .add()
                // One fixed framing, solved once before the spin: the hull
                // from high off its starboard quarter, with room to port for
                // the freed mount and drive. The HUD drops to cinematic so the
                // fps/version bar stays out of the recording.
                .step("frame the port wing tip")
                .on_enter(|world| {
                    hide_hud(world);
                    frame_the_cut(world);
                })
                .until(elapsed(0.8))
                .add()
                // Motion first, recording second: the spin is established
                // before the loop opens, so frame one is already alive.
                .step("set the hull adrift")
                .on_enter(spin_subject)
                .until(elapsed(0.7))
                .add()
                .step("open the loop")
                .on_enter(|world| loop_start(world, LOOP_NAME))
                // A beat of the intact hull, so damage lands mid-loop rather
                // than on frame one.
                .until(elapsed(0.5))
                .add()
                .step("scar the wing-tip plate")
                .on_enter(scar_plate)
                .until(elapsed(1.0))
                .add()
                // The cut: production damage kills the plate, and the step
                // waits for the production sever - a wreck fragment with its
                // own body - to actually exist.
                .step("cut the wing-tip plate")
                .on_enter(cut_plate)
                .until(any_entity::<With<ShipWreckFragmentMarker>>())
                .deadline(5.0)
                .add()
                .step("send the wreck adrift")
                .on_enter(nudge_fragments)
                .until(elapsed(3.5))
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

fn load_scene(mut commands: Commands, game_assets: Res<GameAssets>, ships: Res<GameShipDesigns>) {
    commands.trigger(LoadScenario(sever_range(&game_assets, &ships)));
}

/// The set: one interceptor, three-quarter to the lens, a near rock field for
/// depth and the photo rig.
fn sever_range(game_assets: &GameAssets, ships: &GameShipDesigns) -> ScenarioConfig {
    let subject = EventActionConfig::SpawnScenarioObject(ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: SUBJECT_ID.to_string(),
            name: "Subject".to_string(),
            position: Meters3::ZERO,
            rotation: Quat::from_rotation_y(-0.5),
        },
        kind: ScenarioObjectKind::Spaceship(SpaceshipConfig {
            controller: SpaceshipController::None,
            allegiance: Some(Allegiance::Enemy),
            // The whole shipped interceptor, turrets included. The kit used to
            // drop them: its own hand-typed copy of the mount centres had
            // drifted from the builders', so the mounts mated nothing and the
            // ship came back `Disconnected` - empty adjacency, under which ANY
            // section death severs the whole hull into loose wrecks. The kit
            // reads the ship catalog now, so the cut severs exactly what hangs
            // off the cut section.
            design: ShipDesignSource::Inline(kit::catalog_ship(ships, SUBJECT_HULL)),
            ..default()
        }),
    });
    let field = kit::NearField {
        id_prefix: "loop_rock_",
        count: 12,
        seed: 20260818,
        center: Meters3::ZERO,
        distance: (Meters(550.0), Meters(1_200.0)),
        radius: (Meters(10.0), Meters(25.0)),
        y_spread: Meters(300.0),
    };

    ScenarioConfig {
        description: "An interceptor losing its port mount and wing-tip drive to one cut."
            .to_string(),
        events: vec![ScenarioEventConfig {
            label: None,
            position: None,
            name: EventConfig::OnStart,
            once: false,
            filters: vec![],
            actions: [
                vec![field.action(game_assets), subject],
                ThreePointRig::around("photo", Meters3::ZERO, 1.0).actions(),
            ]
            .concat(),
        }],
        ..ScenarioConfig::new(
            "loop_spine_cut".to_string(),
            "Spine Cut Loop".to_string(),
            game_assets.cubemap.clone().into(),
        )
    }
}

/// Advance once the subject is in the world.
#[cfg(feature = "debug")]
fn subject_present() -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(|world: &World| {
        world
            .try_query_filtered::<&EntityId, With<SpaceshipRootMarker>>()
            .is_some_and(|mut query| query.iter(world).any(|id| id.0 == SUBJECT_ID))
    })
}

/// Stand the lens [`CUT_EYE`] off [`CUT_CELL`] on the live hull, aimed
/// [`CUT_AIM`] off it.
#[cfg(feature = "debug")]
fn frame_the_cut(world: &mut World) {
    let hull = kit::ship_root(world, SUBJECT_ID)
        .and_then(|subject| world.get::<GlobalTransform>(subject).copied())
        .expect("spine loop: the subject is in the world before it is framed");
    // A build cell is one engine unit, so the cell is the hull-local point.
    let cut = Meters3::from_engine(hull.transform_point(CUT_CELL)).get();
    let eye = cut + hull.rotation() * CUT_EYE;
    let aim = cut + hull.rotation() * CUT_AIM;
    let meters = |v: Vec3| Meters3::new(v.x, v.y, v.z);
    pose_camera(world, meters(eye), meters(aim));
}

/// Give the hull its slow tumble.
#[cfg(feature = "debug")]
fn spin_subject(world: &mut World) {
    let Some(subject) = kit::ship_root(world, SUBJECT_ID) else {
        warn!("spine loop: no subject to spin");
        return;
    };
    if let Some(mut angular) = world
        .entity_mut(subject)
        .get_mut::<avian3d::prelude::AngularVelocity>()
    {
        angular.0 = SUBJECT_SPIN;
    }
}

/// The id the cut section carries on the spawned hull.
#[cfg(feature = "debug")]
fn cut_section(world: &mut World) -> String {
    let ships = world.resource::<GameShipDesigns>().clone();
    kit::cell_section(&ships, SUBJECT_HULL, CUT_CELL)
}

/// Leave the cut section visibly cracked before the final severing hit.
#[cfg(feature = "debug")]
fn scar_plate(world: &mut World) {
    let section = cut_section(world);
    let Some(node) = kit::section_health(world, SUBJECT_ID, &section) else {
        warn!("spine loop: no health node under section '{section}'");
        return;
    };
    let amount = world
        .get::<Health>(node)
        .map_or(0.0, |health| health.max * 0.72);
    world.trigger(HealthApplyDamage {
        entity: node,
        source: None,
        amount,
    });
    info!("spine loop: scarred '{section}' for {amount:.1}");
}

/// Kill the cut section through the production damage path.
#[cfg(feature = "debug")]
fn cut_plate(world: &mut World) {
    let section = cut_section(world);
    let Some(node) = kit::section_health(world, SUBJECT_ID, &section) else {
        warn!("spine loop: no health node under section '{section}'");
        return;
    };
    world.trigger(HealthApplyDamage {
        entity: node,
        source: None,
        amount: 1.0e6,
    });
    info!("spine loop: cut through '{section}'");
}

/// Send each freed fragment gently away from the hull, on top of whatever
/// motion the sever handed it.
#[cfg(feature = "debug")]
fn nudge_fragments(world: &mut World) {
    let origin = kit::ship_root(world, SUBJECT_ID)
        .and_then(|subject| world.get::<GlobalTransform>(subject))
        .map(|transform| transform.translation())
        .unwrap_or(Vec3::ZERO);
    let fragments: Vec<(Entity, Vec3)> = world
        .query_filtered::<(Entity, &GlobalTransform), With<ShipWreckFragmentMarker>>()
        .iter(world)
        .map(|(entity, transform)| (entity, transform.translation()))
        .collect();
    if fragments.is_empty() {
        warn!("spine loop: no fragments to send adrift");
        return;
    }
    for (fragment, position) in fragments {
        let outward = (position - origin).normalize_or(Vec3::X);
        if let Some(mut velocity) = world
            .entity_mut(fragment)
            .get_mut::<avian3d::prelude::LinearVelocity>()
        {
            velocity.0 += outward * FRAGMENT_DRIFT_SPEED + Vec3::Y * 0.15;
        }
    }
    info!("spine loop: wreck adrift");
}
