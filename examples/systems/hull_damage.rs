//! hull_damage: hull sections through the damage pipeline, and the mass
//! properties that follow them.
//!
//! One player ship of five sections - a spine along +Z (controller, hull1,
//! hull2, thruster) with hull3 mounted beside hull2 - takes scripted damage.
//! The side mount is load-bearing, not decoration. This range keeps its focused
//! leaf-removal and COM checks; `section_severing` covers interior destruction
//! and physical graph splits. This run absorbed the former COM range (task 20260709-140620): the per-section
//! damage slice and the ship-wide "does avian's centre of mass follow a lost
//! section?" deep-dive are the same rig, walked in one script.
//!
//! EIGHT named invariants across three rounds:
//!
//! | # | marker | claim |
//! | - | - | - |
//! | 1 | `outcome: partial hit exact` | a partial hit subtracts EXACTLY its amount |
//! | 2 | `outcome: section destroyed` | overkill destroys and despawns the section |
//! | 3 | `outcome: root and controller survive` | the rest of the ship survives the loss |
//! | 4 | `outcome: com follows surviving sections` | the live COM sits on the surviving centroid |
//! | 5 | `outcome: com moved aft` | and has moved AFT of where it spawned |
//! | 6 | `outcome: root interpolates` | the root still carries `TransformInterpolation` |
//! | 7 | `outcome: camera anchor tracks com` | the chase camera orbits the physical pivot |
//! | 8 | `outcome: damage invariants hold after reload` | 1-7 all hold again on a fresh rig |
//!
//! Round order is load-bearing: 1-3 need a LIVE controller, so the side hull
//! dies first; only then does round 2 spin the ship and strip the front.
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
//! NOVA_AUTOPILOT=1 cargo run --example hull_damage --features debug
//! # look for: `hull probe: round 1 - damage lands, the section dies, the ship survives`,
//! #           `hull probe: round 1 - the com follows the surviving sections`,
//! #           `hull probe: every invariant held again after the reload`,
//! #           `autopilot: cycle complete, no panic`
//! ```

#[cfg(feature = "debug")]
use std::sync::Arc;

use avian3d::prelude::*;
use bevy::{color::palettes::tailwind, platform::collections::HashMap, prelude::*};
use clap::Parser;
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "hull_damage")]
#[command(version = "1.0.0")]
#[command(about = "Hull sections: damage, destruction, and the mass properties that follow. Autopilot-only correctness range - the O/K keys are a debug aid, not the subject", long_about = None)]
struct Cli;

/// The partial hit the probe lands first, well under the hull's health.
#[cfg(feature = "debug")]
const PARTIAL_HIT: f32 = 60.0;

/// The scenario object id of the side-mounted hull round 1 destroys. Named
/// rather than positional so the `section_gone` beats and the rig config
/// cannot drift apart.
#[cfg(feature = "debug")]
const SIDE_HULL: &str = "hull3";

/// How far aft (in +Z, the rig's local rear) the COM must have travelled from
/// its spawn position once the front sections are gone. Half a section pitch:
/// well inside the ~0.7u the surviving set implies, and far outside the noise
/// of a settled solve.
#[cfg(feature = "debug")]
const COM_AFT_SHIFT: f32 = 0.5;

/// How far the live COM may sit from the surviving-section centroid before
/// avian's mass properties count as stale.
#[cfg(feature = "debug")]
const COM_DRIFT_TOLERANCE: f32 = 0.3;

/// How far the chase camera's anchor may sit from the live COM before the
/// camera counts as orbiting a phantom point.
#[cfg(feature = "debug")]
const CAMERA_DRIFT_TOLERANCE: f32 = 0.5;

/// How many sections `hull_rig` spawns. Invariant 5's baseline is only the
/// WHOLE rig's COM once every one of them has attached, so [`freeze_spawn_com`]
/// waits for this count rather than for the first frame with a plausible mass.
const RIG_SECTION_COUNT: usize = 5;

/// How many sections are still attached once a round has destroyed `hull3`, the
/// controller and `hull1`. The mass-properties beat waits for exactly this, so
/// "the despawns landed" is a counted fact rather than a guessed settling time.
#[cfg(feature = "debug")]
const SURVIVING_SECTION_COUNT: usize = RIG_SECTION_COUNT - 3;

/// The round label the reloaded rig runs under, so invariant 8 fires on that
/// pass and only there.
#[cfg(feature = "debug")]
const RELOADED_ROUND: &str = "round 2";

/// How many avian solve passes the local COM must repeat before the solve
/// counts as done with the despawns. Three rather than one: a despawn can take
/// more than one pass to propagate through the collider hierarchy, so a single
/// matching pair can straddle a pass that had not seen the change yet.
///
/// Passes, not `Update` frames: [`track_com_settle`] samples in
/// `FixedPostUpdate` after [`PhysicsSystems::Prepare`], so one sample is one
/// recompute.
#[cfg(feature = "debug")]
const COM_SETTLE_PASSES: u32 = 3;

/// The script type, named once so the step list and its helpers agree.
#[cfg(feature = "debug")]
type Script = nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates>;

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();
    let mut app = AppBuilder::new().with_game_plugins(custom_plugin).build();

    #[cfg(feature = "debug")]
    {
        app.init_resource::<HullProbe>();
        // Probe wiring (task 20260719-210443; each plugin is inert without
        // its NOVA_PERF_* env): run timeline + engine-bound invariants +
        // frame-time capture, so `probe run` can measure this example.
        app.add_plugins(nova_probe::NovaProbePlugin::default());
        app.add_plugins(hull_script());
        app.add_plugins(nova_screenshot());
    }

    app.run()
}

/// The scripted run: damage round, mass-properties round, then both again on
/// a reloaded rig.
///
/// Not the stock `nova_autopilot()` preset: every beat waits on the value it
/// depends on - the section's live `Health`, its despawn, the surviving-section
/// count, the COM settling - so a slow load or a slow solve delays the walk
/// instead of truncating it. The per-step deadlines NAME the beat that stalled, and their
/// runtime sum (15 + 30 + 10 + 30 = 85s, counting `damage_and_mass_rounds`'
/// beats once per CALL rather than once per source line) stays under
/// `DEFAULT_DEADLINE_SECS` (120s) so a named stall wins the race against the
/// generic collector deadline.
#[cfg(feature = "debug")]
fn hull_script() -> Script {
    let script = Script::new()
        .step("load the rig")
        .enter(GameStates::Loading)
        .until(player_ship_present())
        .deadline(15.0)
        .add();
    let script = damage_and_mass_rounds(script, "round 1");
    let script = script
        // Wait for a rig that is DEMONSTRABLY the fresh one: round 1 destroyed
        // `SIDE_HULL`, so only a respawned rig has it back.
        //
        // Not "wait for the ship to be gone, then back": there is no such
        // frame. `on_load_scenario` queues the scoped despawns and fires
        // `OnStartEvent` on the SAME `Commands`
        // (nova_scenario/src/loader/lifecycle.rs:162-253), so the teardown and
        // the respawn land in one flush and `not(player_ship_present())` never
        // observes true - it stalls out its deadline instead.
        //
        // `not` is qualified: the name collides with `bevy::prelude::not`,
        // which this file globs through the bevy prelude.
        .step("reload the rig")
        .on_enter(reload_rig)
        .until(nova_protocol::nova_debug::harness::not(section_gone(
            SIDE_HULL,
        )))
        .deadline(10.0)
        .add();
    // Last beats: the driver reports done after the round's closing assertion,
    // so the run ends on an assertion rather than idling out a runway.
    damage_and_mass_rounds(script, RELOADED_ROUND)
}

/// One full pass of invariants 1-7, appended to `script`. Called twice - once
/// on the booted rig and once on the reloaded one - which is what makes
/// invariant 8 a claim about the whole set rather than a spot check. The
/// reloaded pass emits invariant 8 from its closing assertion.
#[cfg(feature = "debug")]
fn damage_and_mass_rounds(script: Script, round: &'static str) -> Script {
    script
        // R1: the side hull dies first. Invariants 1-3 all need a LIVE
        // controller to be about anything, and round 2 kills it.
        .step("land the partial hit on the side hull")
        .on_enter(damage_side_hull(PARTIAL_HIT))
        .until(side_hull_took_the_partial_hit())
        .deadline(6.0)
        .add()
        .step("assert the partial hit landed exactly")
        .on_enter(move |world: &mut World| assert_partial_hit(world, round))
        .add()
        .step("overkill the side hull")
        .on_enter(destroy_side_hull)
        .until(section_gone(SIDE_HULL))
        .deadline(6.0)
        .add()
        .step("assert the ship survived the loss")
        .on_enter(move |world: &mut World| assert_ship_survives(world, round))
        .add()
        // R2: spin, strip the front, and read the mass properties the losses
        // should have moved.
        .step("spin the ship")
        .on_enter(apply_spin)
        .add()
        .step("kill the controller section")
        .on_enter(kill_frontmost_section)
        .until(section_gone("controller"))
        .deadline(6.0)
        .add()
        .step("kill the first hull section")
        .on_enter(kill_frontmost_section)
        .until(section_gone("hull1"))
        .deadline(6.0)
        .add()
        // The mass properties and the camera anchor both need the solve to
        // follow the despawn; wait for THAT rather than for a settling second.
        .step("settle onto the surviving mass")
        .until(mass_properties_settled())
        .deadline(6.0)
        .add()
        .step("assert the com follows the surviving sections")
        .on_enter(move |world: &mut World| assert_com_follows_sections(world, round))
        .add()
}

fn custom_plugin(app: &mut App) {
    app.init_resource::<SpawnLocalCom>();
    app.add_systems(OnEnter(GameAssetsStates::Loaded), setup_rig);
    app.add_systems(
        Update,
        (hull_range_hotkeys, draw_com_gizmos, log_com_status),
    );
    // Both COM samplers live on the PHYSICS schedule, after avian's mass
    // properties have been recomputed for the tick (`MassPropertyPlugin`
    // defaults to `FixedPostUpdate`, inside `PhysicsSystems::Prepare`). On
    // `Update` a "the COM held still for N frames" reading proves nothing: at
    // any framerate above the fixed rate most `Update` frames carry no solve
    // pass at all, so N unchanged frames can all precede the recompute that
    // the beat is waiting for.
    app.add_systems(
        FixedPostUpdate,
        freeze_spawn_com.after(PhysicsSystems::Prepare),
    );
    #[cfg(feature = "debug")]
    {
        app.init_resource::<ComSettle>();
        app.add_systems(
            FixedPostUpdate,
            track_com_settle.after(PhysicsSystems::Prepare),
        );
    }
}

/// How many consecutive avian solve passes have reproduced the same LOCAL
/// centre of mass under the same attached-section count.
///
/// Local, not world: round 2 spins the rig, so its world-space COM moves every
/// frame whether or not the solve has anything left to do.
///
/// The count is carried alongside so the beat can read BOTH facts from the same
/// post-solve sample. Reading the count live off the world instead would let a
/// despawn that has flushed but not yet been solved pair a fresh count with a
/// stale COM's saturated streak.
#[cfg(feature = "debug")]
#[derive(Resource, Default)]
struct ComSettle {
    last: Option<ComSample>,
    still_passes: u32,
}

/// One post-solve reading: which rig, how many sections were attached to it,
/// and the local COM that came out.
#[cfg(feature = "debug")]
#[derive(Clone, Copy)]
struct ComSample {
    root: Entity,
    attached: usize,
    com: Vec3,
}

/// Sample the live local COM and count the solve passes it has held still.
#[cfg(feature = "debug")]
fn track_com_settle(
    mut settle: ResMut<ComSettle>,
    q_root: Query<
        (Entity, &ComputedCenterOfMass),
        (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>),
    >,
    q_sections: Query<&ChildOf, With<SectionMarker>>,
) {
    let Ok((root, com)) = q_root.single() else {
        settle.last = None;
        settle.still_passes = 0;
        return;
    };
    let sample = ComSample {
        root,
        attached: attached_to(&q_sections, root),
        com: com.0,
    };
    let held = settle.last.is_some_and(|last| {
        last.root == sample.root
            && last.attached == sample.attached
            && last.com.distance(sample.com) < 1e-4
    });
    if held {
        settle.still_passes += 1;
    } else {
        settle.still_passes = 0;
    }
    settle.last = Some(sample);
}

/// How many sections are children of `root`.
fn attached_to(q_sections: &Query<&ChildOf, With<SectionMarker>>, root: Entity) -> usize {
    q_sections
        .iter()
        .filter(|&&ChildOf(parent)| parent == root)
        .count()
}

fn setup_rig(mut commands: Commands, game_assets: Res<GameAssets>, sections: Res<GameSections>) {
    commands.trigger(LoadScenario(hull_rig(&game_assets, &sections)));
}

/// The rig scenario: a single PLAYER ship, five sections in a line.
///
/// Player-controlled with an EMPTY input mapping, not decoration: the chase
/// camera (invariant 7) only follows a player ship, and an empty mapping means
/// no input is synthesized, so the rig still only moves when the script
/// damages or spins it.
fn hull_rig(game_assets: &GameAssets, sections: &GameSections) -> ScenarioConfig {
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
        modifications: vec![],
    };

    let ship = SpaceshipConfig {
        allegiance: None,
        controller: SpaceshipController::Player(PlayerControllerConfig {
            input_mapping: HashMap::new(),
            speed_cap: None,
            // Dev/tuning harness: fire freely.
            infinite_ammo: true,
        }),
        // A four-section spine plus one side-mounted hull. This established
        // range keeps a simple leaf removal for its COM assertions; the
        // `section_severing` range owns interior cuts and wreck-body motion.
        hull: ShipSource::Inline(ShipHull {
            sections: vec![
                at(
                    "controller",
                    "basic_controller_section",
                    Vec3::new(0.0, 0.0, 0.0),
                ),
                at("hull1", "reinforced_hull_section", Vec3::new(0.0, 0.0, 1.0)),
                at("hull2", "reinforced_hull_section", Vec3::new(0.0, 0.0, 2.0)),
                at(
                    "thruster",
                    "basic_thruster_section",
                    Vec3::new(0.0, 0.0, 3.0),
                ),
                at("hull3", "reinforced_hull_section", Vec3::new(1.0, 0.0, 2.0)),
            ],
            ..default()
        }),
        ..default()
    };

    ScenarioConfig {
        description: "A sectioned ship taking scripted damage.".to_string(),
        events: vec![ScenarioEventConfig {
            name: EventConfig::OnStart,
            filters: vec![],
            // The rig lights itself: the engine spawns no light, so a
            // scenario that authors none renders black.
            actions: [
                vec![EventActionConfig::SpawnScenarioObject(
                    ScenarioObjectConfig {
                        base: BaseScenarioObjectConfig {
                            id: "player_ship".to_string(),
                            name: "Rig Ship".to_string(),
                            position: Vec3::ZERO,
                            rotation: Quat::IDENTITY,
                        },
                        kind: ScenarioObjectKind::Spaceship(ship),
                    },
                )],
                ThreePointRig::around("rig", Vec3::ZERO, 1.0).actions(),
            ]
            .concat(),
        }],
        ..ScenarioConfig::new(
            "hull_rig".to_string(),
            "Hull Section Rig".to_string(),
            game_assets.cubemap.clone().into(),
        )
    }
}

/// The player ship root, if it exists.
fn player_root(world: &World) -> Option<Entity> {
    world
        .try_query_filtered::<Entity, (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>)>()?
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
    info!("hull rig: spin applied to {root:?}");
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
        info!("hull rig: no surviving section left to kill");
        return;
    };
    info!("hull rig: killing section {section:?} at z={z} ({health} hp)");
    world.trigger(HealthApplyDamage {
        entity: section,
        source: None,
        amount: health,
    });
}

/// Interactive controls: O = spin, K = kill the frontmost surviving section.
fn hull_range_hotkeys(world: &mut World) {
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

/// The ship's LOCAL centre of mass at spawn, frozen once avian has settled
/// real mass properties. Invariant 5 measures the aft shift against THIS
/// rather than a hard-coded z: the surviving set differs between the merged
/// run and the range it came from, so the baseline has to come from the run.
#[derive(Resource, Default)]
struct SpawnLocalCom(Option<Vec3>);

/// Freeze the spawn-time local COM once, per rig. Cleared on reload, so the
/// second rig measures against its own spawn.
///
/// Gated on all [`RIG_SECTION_COUNT`] sections having attached AND on avian
/// having reproduced the same local COM on two consecutive SOLVE PASSES, not
/// merely on a finite mass: the sections arrive over several frames, a single
/// attached one already carries a plausible mass, and the solve lands behind
/// the last child. A capture taken mid-assembly would silently move invariant
/// 5's baseline toward the nose - the aft shift it then measures would be
/// partly the rest of the rig arriving.
///
/// Runs in `FixedPostUpdate` after [`PhysicsSystems::Prepare`] for that reason:
/// on `Update` the two "consecutive" samples can both precede the first
/// recompute after the fifth section attached, and freeze exactly the
/// mid-assembly baseline this guard exists to reject.
fn freeze_spawn_com(
    mut spawn_com: ResMut<SpawnLocalCom>,
    // Keyed by root entity, so a reload cannot match the previous rig's reading.
    mut previous: Local<Option<(Entity, Vec3)>>,
    q_root: Query<
        (Entity, &ComputedCenterOfMass, &ComputedMass),
        (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>),
    >,
    q_sections: Query<&ChildOf, With<SectionMarker>>,
) {
    if spawn_com.0.is_some() {
        return;
    }
    let Ok((root, com, mass)) = q_root.single() else {
        *previous = None;
        return;
    };
    let attached = attached_to(&q_sections, root);
    if !(com.0.is_finite() && mass.value() > 0.5 && attached == RIG_SECTION_COUNT) {
        *previous = None;
        return;
    }
    if previous.is_some_and(|(last_root, last)| last_root == root && last.distance(com.0) < 1e-4) {
        spawn_com.0 = Some(com.0);
    }
    *previous = Some((root, com.0));
}

#[cfg(feature = "debug")]
/// Live COM (world), attached-section centroid (world), and the ship root's
/// GlobalTransform.
fn com_snapshot(world: &World) -> Option<(Vec3, Vec3, GlobalTransform)> {
    let root = player_root(world)?;
    let entity = world.get_entity(root).ok()?;
    let root_gt = *entity.get::<GlobalTransform>()?;
    let com = entity.get::<ComputedCenterOfMass>()?.0;
    let world_com = root_gt.transform_point(com);
    let (acc, total) = attached_section_moment(world, root)?;
    (total > 0.0).then(|| (world_com, acc / total, root_gt))
}

#[cfg(feature = "debug")]
/// Mass-weighted position sum and total density of the sections still attached
/// to `root`, in world space.
fn attached_section_moment(world: &World, root: Entity) -> Option<(Vec3, f32)> {
    let mut query = world
        .try_query_filtered::<(&ChildOf, &GlobalTransform, &ColliderDensity), With<SectionMarker>>(
        )?;
    let mut acc = Vec3::ZERO;
    let mut total = 0.0;
    for (&ChildOf(parent), gt, density) in query.iter(world) {
        if parent == root {
            acc += gt.translation() * density.0;
            total += density.0;
        }
    }
    Some((acc, total))
}

#[cfg(feature = "debug")]
/// The chase camera's orbit anchor, if a chase camera is up.
fn camera_anchor(world: &World) -> Option<Vec3> {
    world
        .try_query_filtered::<&ChaseCameraInput, With<ChaseCamera>>()?
        .iter(world)
        .next()
        .map(|input| input.anchor_pos)
}

/// RED = avian's live COM, GREEN = attached-section centroid, GRAY = the
/// spawn-time COM carried along with the hull.
#[expect(
    clippy::type_complexity,
    reason = "one query per center-of-mass gizmo input"
)]
fn draw_com_gizmos(
    mut gizmos: Gizmos,
    spawn_com: Res<SpawnLocalCom>,
    q_root: Query<
        (Entity, &GlobalTransform, &ComputedCenterOfMass),
        (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>),
    >,
    q_sections: Query<(&ChildOf, &GlobalTransform, &ColliderDensity), With<SectionMarker>>,
) {
    let Ok((root, root_gt, com)) = q_root.single() else {
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
    if let Some(frozen_local) = spawn_com.0 {
        gizmos.sphere(
            Isometry3d::from_translation(root_gt.transform_point(frozen_local)),
            0.25,
            tailwind::GRAY_500,
        );
    }
}

/// One status line per second: live mass and the COM-vs-centroid distance.
#[expect(
    clippy::type_complexity,
    reason = "one query term per logged mass quantity"
)]
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
        "hull rig: mass={:.2} com={world_com:.2?} centroid={centroid:.2?} drift={:.3}",
        mass.value(),
        world_com.distance(centroid)
    );
}

/// What the script has captured about the round in flight.
#[cfg(feature = "debug")]
#[derive(Resource, Default)]
struct HullProbe {
    /// The side hull's entity and its health BEFORE the partial hit.
    target: Option<(Entity, f32)>,
}

/// The named side hull, its entity and live health.
#[cfg(feature = "debug")]
fn side_hull(world: &World) -> Option<(Entity, f32)> {
    let mut query =
        world.try_query_filtered::<(Entity, &EntityId, &Health), With<HullSectionMarker>>()?;
    query
        .iter(world)
        .find(|(_, id, _)| id.0 == SIDE_HULL)
        .map(|(entity, _, health)| (entity, health.current))
}

/// Land `amount` on the side hull, remembering the health it had first.
#[cfg(feature = "debug")]
fn damage_side_hull(amount: f32) -> impl Fn(&mut World) + Send + Sync + 'static {
    move |world: &mut World| {
        let (entity, health) =
            side_hull(world).unwrap_or_else(|| panic!("hull probe: no live section `{SIDE_HULL}`"));
        world.resource_mut::<HullProbe>().target = Some((entity, health));
        world.trigger(HealthApplyDamage {
            entity,
            source: None,
            amount,
        });
    }
}

/// The partial-hit beat's condition: the section's live health has come down
/// by the hit. Waiting on the VALUE rather than on a settling beat is what
/// makes the exactness assertion a frame later meaningful.
#[cfg(feature = "debug")]
fn side_hull_took_the_partial_hit() -> Arc<nova_protocol::nova_debug::harness::Predicate> {
    Arc::new(|world: &World| {
        let Some((entity, before)) = world.get_resource::<HullProbe>().and_then(|p| p.target)
        else {
            return false;
        };
        world
            .get::<Health>(entity)
            .is_some_and(|health| health.current <= before - PARTIAL_HIT)
    })
}

/// Overkill the same section, so the destroy -> despawn pipeline runs.
#[cfg(feature = "debug")]
fn destroy_side_hull(world: &mut World) {
    let (entity, _) = side_hull(world)
        .unwrap_or_else(|| panic!("hull probe: `{SIDE_HULL}` vanished before the overkill"));
    let health = world
        .get::<Health>(entity)
        .expect("hull probe: the side hull has health")
        .current;
    world.trigger(HealthApplyDamage {
        entity,
        source: None,
        amount: health * 10.0,
    });
}

/// Invariant 1: the partial hit subtracted EXACTLY its amount - the damage
/// path is doing arithmetic, not vibes.
#[cfg(feature = "debug")]
fn assert_partial_hit(world: &mut World, round: &str) {
    let (entity, before) = world
        .resource::<HullProbe>()
        .target
        .expect("hull probe: the partial hit must have recorded its target");
    let current = world
        .get::<Health>(entity)
        .expect("hull probe: the hull must survive a partial hit")
        .current;
    assert!(
        (current - (before - PARTIAL_HIT)).abs() < 1e-3,
        "hull probe ({round}): expected {before} - {PARTIAL_HIT} = {}, got {current}",
        before - PARTIAL_HIT
    );
    info!("hull probe: {round} - the partial hit landed exactly ({current} hp left)");
    let elapsed = world.resource::<Time>().elapsed_secs();
    nova_probe::probe_marker(
        world,
        "outcome: partial hit exact",
        serde_json::json!({ "t": elapsed, "round": round, "health_after": current }),
    );
}

/// Invariants 2 and 3: the overkilled section is destroyed and GONE, and the
/// ship root plus its controller live on - the integrity graph detached a dead
/// leaf rather than the ship.
#[cfg(feature = "debug")]
fn assert_ship_survives(world: &mut World, round: &str) {
    let (entity, _) = world
        .resource::<HullProbe>()
        .target
        .expect("hull probe: the damage round must have recorded its target");
    assert!(
        world.get_entity(entity).is_err(),
        "hull probe ({round}): an overkilled hull section must be destroyed and despawned"
    );
    let elapsed = world.resource::<Time>().elapsed_secs();
    nova_probe::probe_marker(
        world,
        "outcome: section destroyed",
        serde_json::json!({ "t": elapsed, "round": round }),
    );

    let (roots, controllers) = {
        let mut q_root = world.query_filtered::<(), With<SpaceshipRootMarker>>();
        let roots = q_root.iter(world).count();
        let mut q_controller = world.query_filtered::<(), With<ControllerSectionMarker>>();
        (roots, q_controller.iter(world).count())
    };
    assert_eq!(
        roots, 1,
        "hull probe ({round}): the ship root must survive losing a leaf hull"
    );
    assert_eq!(
        controllers, 1,
        "hull probe ({round}): the controller must survive losing a leaf hull"
    );
    info!("hull probe: {round} - damage lands, the section dies, the ship survives");
    nova_probe::probe_marker(
        world,
        "outcome: root and controller survive",
        serde_json::json!({ "t": elapsed, "round": round }),
    );
}

/// The mass-properties beat's condition: avian's last [`COM_SETTLE_PASSES`]
/// solve passes all reproduced the same local COM, and all of them saw the
/// round's despawns landed (`SURVIVING_SECTION_COUNT` sections on the root).
///
/// Both clauses come from [`ComSettle`], i.e. from the same post-solve samples.
/// Reading the count live off the world here instead would reopen the hole the
/// streak is meant to close: a despawn flushes on `Update`, so the world can
/// report the surviving count while the newest solve pass still carries the
/// pre-despawn COM and a streak saturated during the wait before it.
///
/// Deliberately NOT the drift comparisons invariants 4 and 7 then make. Gating
/// on the asserts' own tolerances made those asserts unfailable: a stale COM or
/// a phantom camera anchor could only ever surface as a deadline stall on this
/// beat's name, never as the invariant message that explains it. This gate is
/// about the world having stopped changing; where the COM ENDED UP is the
/// assertions' question.
#[cfg(feature = "debug")]
fn mass_properties_settled() -> Arc<nova_protocol::nova_debug::harness::Predicate> {
    Arc::new(|world: &World| {
        world.get_resource::<ComSettle>().is_some_and(|settle| {
            settle.still_passes >= COM_SETTLE_PASSES
                && settle
                    .last
                    .is_some_and(|sample| sample.attached == SURVIVING_SECTION_COUNT)
        })
    })
}

/// Invariants 4-7: the live COM sits on the surviving centroid, has moved aft
/// of where it spawned, the root still interpolates, and the chase camera
/// orbits the physical pivot rather than the root origin. On the reloaded round
/// this is also invariant 8 - the whole set held again - since reaching the end
/// of it means every earlier beat and assertion of that pass resolved.
#[cfg(feature = "debug")]
fn assert_com_follows_sections(world: &mut World, round: &str) {
    let Some((world_com, centroid, root_gt)) = com_snapshot(world) else {
        panic!("hull probe ({round}): ship vanished before the assertion");
    };
    let elapsed = world.resource::<Time>().elapsed_secs();

    let drift = world_com.distance(centroid);
    assert!(
        drift < COM_DRIFT_TOLERANCE,
        "hull probe ({round}): live COM is {drift:.2} away from the attached-section \
         centroid - avian's COM is stale in the real app"
    );
    info!("hull probe: {round} - the com follows the surviving sections (drift {drift:.3})");
    nova_probe::probe_marker(
        world,
        "outcome: com follows surviving sections",
        serde_json::json!({ "t": elapsed, "round": round, "com_drift": drift }),
    );

    // Aft of where THIS rig's COM spawned, not of a number copied from a run
    // with a different surviving set.
    let spawn_local = world
        .resource::<SpawnLocalCom>()
        .0
        .expect("hull probe: the spawn COM must have been frozen before any damage");
    let local_com = root_gt.affine().inverse().transform_point3(world_com);
    let shift = local_com.z - spawn_local.z;
    assert!(
        shift > COM_AFT_SHIFT,
        "hull probe ({round}): local COM {local_com:.2?} moved {shift:.2} aft of the \
         spawn COM {spawn_local:.2?}, expected more than {COM_AFT_SHIFT} after losing \
         the front sections"
    );
    nova_probe::probe_marker(
        world,
        "outcome: com moved aft",
        serde_json::json!({ "t": elapsed, "round": round, "aft_shift": shift }),
    );

    // The ship must interpolate its Transform between fixed ticks, or it
    // stair-steps under the smoothed camera (task 20260709-160753).
    let root = player_root(world).expect("hull probe: player root exists at assert time");
    assert!(
        world.entity(root).contains::<TransformInterpolation>(),
        "hull probe ({round}): the ship root lost TransformInterpolation - camera twitch returns"
    );
    nova_probe::probe_marker(
        world,
        "outcome: root interpolates",
        serde_json::json!({ "t": elapsed, "round": round }),
    );

    // The chase camera must orbit the physical pivot too, or a tumbling ship
    // appears to spin around the empty space at the root origin. The camera is
    // mandatory here: a broken query must fail, not skip.
    let anchor =
        camera_anchor(world).expect("hull probe: no chase camera input found at assert time");
    let cam_drift = anchor.distance(world_com);
    assert!(
        cam_drift < CAMERA_DRIFT_TOLERANCE,
        "hull probe ({round}): chase camera anchor {anchor:.2?} is {cam_drift:.2} \
         away from the live COM - the camera orbits a phantom point"
    );
    info!("hull probe: {round} - the camera anchor tracks the COM (drift {cam_drift:.3})");
    nova_probe::probe_marker(
        world,
        "outcome: camera anchor tracks com",
        serde_json::json!({ "t": elapsed, "round": round, "camera_drift": cam_drift }),
    );

    // Invariant 8, on the reloaded pass: the whole set held again on a fresh
    // rig. Emitted from the round's LAST assertion rather than from a step of
    // its own, so the slug rides real asserts - reaching this line means every
    // earlier beat of the pass resolved and every one of 1-7 just passed.
    if round == RELOADED_ROUND {
        info!("hull probe: every invariant held again after the reload");
        nova_probe::probe_marker(
            world,
            "outcome: damage invariants hold after reload",
            serde_json::json!({ "t": elapsed }),
        );
    }
}

/// Re-trigger the scenario load, and clear the per-rig state the fresh rig has
/// to re-establish: the recorded damage target and the frozen spawn COM.
#[cfg(feature = "debug")]
fn reload_rig(world: &mut World) {
    let config = {
        let game_assets = world.resource::<GameAssets>();
        let sections = world.resource::<GameSections>();
        hull_rig(game_assets, sections)
    };
    world.resource_mut::<HullProbe>().target = None;
    world.resource_mut::<SpawnLocalCom>().0 = None;
    info!("hull probe: reloading the rig");
    world.trigger(LoadScenario(config));
}
