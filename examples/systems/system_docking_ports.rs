//! system_docking_ports: what `DOCK` builds between two hulls, what the joint
//! then holds, and what takes it away again.
//!
//! The mechanic is one verb wide and is deliberately NOT an attachment: two
//! docked hulls stay two rigid bodies held by one avian `FixedJoint` and the
//! sleeves that reach across are art. It is also MODAL - a docked hull's drive
//! is inert until the verb is pressed again - and that is a claim about
//! FORCES, not about state. Which is why these are staged here rather than in
//! the unit tests beside the code: the unit tests grade the candidate search,
//! this range grades what the SOLVER does with the joint the search produces.
//!
//! | # | marker | claim |
//! | - | - | - |
//! | 1 | `outcome: one command builds one connection and one root joint` | a single `DOCK` yields exactly one connection entity, one fixed joint, and two reserved ports |
//! | 2 | `outcome: both sleeves reach out once the joint holds` | the joint exists BEFORE the art moves, and both ports leave `Retracted` |
//! | 3 | `outcome: the sleeve never changes what the hull collides with` | the port's collider is the same box extended as retracted |
//! | 4 | `outcome: the joint carries the pair without zeroing its drift` | a hull pushed while docked tows the other one, and the pair keeps the velocity it was given |
//! | 5 | `outcome: the joint holds the pose the two hulls met in` | after the tow, the second hull sits where it sat in the first hull's frame at capture |
//! | 6 | `outcome: a docked hull ignores the throttle` | a full burn held on a docked hull moves neither ship: the drive is gated at the force, not at the key |
//! | 7 | `outcome: the dock verb takes the dock away from either hull` | the ship that did NOT issue `DOCK` asks to be released, and the connection, the joint and both reservations go |
//! | 8 | `outcome: the throttle bites the moment the dock lets go` | the SAME burn, still held, accelerates the same hull once it is free - so claim 6 is the dock and not a dead engine |
//! | 9 | `outcome: a destroyed port frees its partner` | destroying one port cleans the connection up and leaves the surviving port free and stowing |
//! | 10 | `outcome: the dock geometry is recorded` | RECORD: the face gap at capture, the tow distance, and the pose error the joint carried |
//! | 11 | `outcome: the scenario hears every dock and every release` | both captures and both releases - the verb's and the destroyed port's - reach authored `OnDocked` / `OnUndocked` handlers, through an `Entity` filter on the acting hull the payload has to fill |
//!
//! Claim 10 asserts nothing. It is the geometry the other claims are read
//! against, kept so a later change to the envelope can be compared rather than
//! argued about.
//!
//! Each hull is a one-cell block with a port on its nose and a drive on its
//! tail, so the throttle claims are made against a ship that can actually
//! burn. They stand nose to nose down Z
//! at half a cell of face gap - inside the authored one-cell capture distance,
//! and far enough out that the two colliders never touch, so nothing here can
//! be explained by a contact.
//!
//! Headless smoke test:
//!
//! ```text
//! NOVA_AUTOPILOT=1 cargo run --example system_docking_ports --features debug
//! ```

use avian3d::prelude::*;
use bevy::{color::palettes::tailwind, prelude::*};
use clap::Parser;
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "system_docking_ports")]
#[command(version = "1.0.0")]
#[command(about = "Docking capture, the fixed joint it builds, and the two ways it ends. Autopilot-only correctness range", long_about = None)]
struct Cli;

/// The face gap the pair is staged at, engine units. Half the authored
/// one-cell capture distance: eligible with room to spare, and half a cell of
/// daylight between the two colliders.
const FACE_GAP: f32 = 0.5;

/// The speed the docked pair is towed at, to prove the joint carries the
/// second hull instead of leaving it behind. Well under the 5 m/s capture
/// ceiling is irrelevant here - the pair is already docked, and a dock has no
/// speed limit of its own.
const TOW_SPEED: MetersPerSecond = MetersPerSecond(120.0);

/// How far the second hull may sit from where it sat at capture, in the first
/// hull's frame, after the tow. A solver allowance, not a band: a fixed joint
/// is a constraint the solver converges on, not a weld.
const POSE_TOLERANCE: f32 = 0.1;

/// How far the pair must actually travel before the pose reading means
/// anything. Twenty times the tolerance, so "it held" cannot be satisfied by a
/// scene that never moved.
const TOW_DISTANCE: f32 = 2.0;

/// The speed each hull must still carry when the tow is read, engine units.
/// A tenth of the push: the pair shares one hull's momentum between two
/// bodies and spins about the joint while it does, so the number to assert is
/// that NEITHER hull was stopped - not that both hold the speed one was given.
const TOWED_SPEED_FLOOR: f32 = 1.0;

/// Frames a full burn is held on the docked pair before the modal claim is
/// read. Long enough that an ungated drive would have moved the pair well past
/// [`HELD_SPEED_CEILING`], which is the only thing that makes "it did not
/// move" mean anything.
const HOLD_FRAMES: u32 = 60;

/// How fast either hull may still be drifting after the held burn, engine
/// units per second. Not zero: the pair was towed and brought back to rest by
/// hand a moment earlier, and a solver settling two jointed bodies is never
/// exactly still.
const HELD_SPEED_CEILING: f32 = 0.05;

/// How fast the released hull must be going once the same burn reaches its
/// drive. Twenty times the ceiling above, so the two readings cannot be
/// confused for each other.
const FREED_SPEED_FLOOR: f32 = 1.0;

/// Frames given to each stage of the range before it calls the world stuck.
const STAGE_BUDGET: u32 = 600;

/// Scenario ids the two hand-built hulls wear, so authored handlers can name
/// them the way they name a spawned ship.
const FIRST_HULL_ID: &str = "first_hull";
/// The hull `DOCK` is issued AGAINST: the subject of both docking events.
const SECOND_HULL_ID: &str = "second_hull";

/// Scenario variables the two docking handlers count into.
const DOCKED_TALLY: &str = "docked_seen";
/// The release counterpart of [`DOCKED_TALLY`].
const UNDOCKED_TALLY: &str = "undocked_seen";

/// Docks the range makes, and therefore releases it makes: the verb's, and
/// the one a destroyed port forces.
const EXPECTED_DOCKS: f64 = 2.0;

/// What the range is doing right now.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Stage {
    /// Letting avian link colliders and settle the two bodies.
    #[default]
    Settling,
    /// Docked; being towed, so the joint can be read under load.
    Towing,
    /// Docked, at rest, with a full burn commanded: the modal claim.
    Holding,
    /// Docked with the burn still held; waiting for the verb to let go.
    Releasing,
    /// Docked a third time; waiting for a destroyed port to break it.
    Destroying,
    /// Both docks and both releases made; waiting for the scenario's own
    /// handlers to report them.
    Hearing,
    /// Every claim made.
    Done,
}

#[derive(Resource, Default)]
struct DockProbe {
    first: Option<Entity>,
    first_port: Option<Entity>,
    second: Option<Entity>,
    second_port: Option<Entity>,
    stage: Stage,
    frames: u32,
    stage_frames: u32,
    /// The second hull's pose in the first hull's frame at capture.
    captured: Option<(Vec3, Quat)>,
    /// Where the pair stood when the tow started.
    tow_origin: Option<Vec3>,
    /// The port collider's half extents, read before the sleeve moved.
    retracted_collider: Option<Vec3>,
    /// The face gap the capture was accepted at.
    gap: f32,
    /// How far the pair travelled under tow.
    towed: f32,
    /// How far the joint let the second hull slip, engine units.
    pose_error: f32,
    /// The frame within the release stage the connection actually went, so
    /// the freed burn is given a window of its own rather than a guess.
    released_at: Option<u32>,
    exit_delay: u32,
}

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();
    let mut app = AppBuilder::new().with_game_plugins(range_plugin).build();
    app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
    app.run()
}

fn range_plugin(app: &mut App) {
    app.init_resource::<DockProbe>();
    app.add_systems(Startup, setup_range);
    app.add_systems(OnEnter(GameAssetsStates::Loaded), open_the_section_gate);
    app.add_systems(Update, drive_range.run_if(in_state(GameStates::Playing)));
}

/// Load the range's scenario: it opens the gate the section systems ride, and
/// it carries the two docking handlers claim 11 reads.
///
/// `configure_scenario_gating` holds `SpaceshipSectionSystems` on
/// `scenario_is_live`, and the drive's impulse is in that set. The two hulls
/// here are hand-built rather than authored, so without a live scenario a
/// docked hull's throttle would be inert because NOTHING RAN - which is the
/// one reading that would make claim 6 worthless. The scenario carries no
/// objects: this range spawns its own camera, light and hulls.
///
/// The handlers are authored the way a mod authors them - a trigger, an
/// `Entity` filter naming the pair, an action - so claim 11 grades the whole
/// path from the joint to a scenario variable, the registration in
/// `NovaScenarioPlugin` included.
fn open_the_section_gate(mut commands: Commands, game_assets: Res<GameAssets>) {
    let mut scenario = ScenarioConfig::new(
        "docking_range",
        "Docking Range",
        game_assets.cubemap.clone().into(),
    );
    scenario.events.push(ScenarioEventConfig {
        label: Some("seed the docking tallies".to_string()),
        name: EventConfig::OnStart,
        once: false,
        filters: vec![],
        actions: vec![set_tally(DOCKED_TALLY, 0.0), set_tally(UNDOCKED_TALLY, 0.0)],
    });
    for (event, tally) in [
        (EventConfig::OnDocked, DOCKED_TALLY),
        (EventConfig::OnUndocked, UNDOCKED_TALLY),
    ] {
        scenario.events.push(ScenarioEventConfig {
            label: Some(format!("count {tally}")),
            name: event,
            once: false,
            // Matched on the ACTING hull's type name rather than the two
            // ids: an `Entity` filter's ids are lint-checked against what the
            // scenario SPAWNS, and these hulls are hand-built. The field is
            // still only filled by the tracker, so a payload that named
            // nobody would fail the filter and leave the tally at zero.
            // Which hull is which is graded in the tracker's own tests.
            filters: vec![EventFilterConfig::Entity(EntityFilterConfig {
                other_type_name: Some(SPACESHIP_TYPE_NAME.to_string()),
                ..default()
            })],
            actions: vec![EventActionConfig::VariableSet(VariableSetActionConfig {
                key: tally.to_string(),
                expression: VariableExpressionNode::new_add(
                    VariableTermNode::new_factor(VariableFactorNode::new_name(tally)),
                    VariableExpressionNode::new_term(VariableTermNode::new_factor(
                        VariableFactorNode::new_literal(VariableLiteral::Number(1.0)),
                    )),
                ),
            })],
        });
    }
    commands.trigger(LoadScenario(scenario));
}

/// A `VariableSet` that parks one tally on a literal.
fn set_tally(key: &str, value: f64) -> EventActionConfig {
    EventActionConfig::VariableSet(VariableSetActionConfig {
        key: key.to_string(),
        expression: VariableExpressionNode::new_term(VariableTermNode::new_factor(
            VariableFactorNode::new_literal(VariableLiteral::Number(value)),
        )),
    })
}

/// One hull: a dynamic body carrying a block section and a docking port on
/// its -Z face, the face the ports look out of.
fn spawn_hull(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    name: &str,
    id: &str,
    at: Vec3,
    facing: Quat,
    tint: Srgba,
) -> (Entity, Entity) {
    let root = commands
        .spawn((
            Name::new(name.to_string()),
            // A hand-built hull is still a scenario object as far as an
            // authored handler is concerned: the docking events name a pair by
            // these two components, exactly as they would a spawned ship.
            EntityId::new(id),
            EntityTypeName::new(SPACESHIP_TYPE_NAME),
            SpaceshipRootMarker,
            RigidBody::Dynamic,
            Transform::from_translation(at).with_rotation(facing),
            Visibility::default(),
            TransformInterpolation,
            // A sleeping body ignores an applied force. The modal claim is
            // "the throttle does nothing while docked", and a pair that avian
            // had put to sleep would satisfy it for the wrong reason - as
            // would the claim after it, which reads the same throttle biting.
            SleepingDisabled,
        ))
        .id();
    let block = SectionCollider::Cuboid {
        size: Vec3::splat(1.0),
    };
    let hull = commands
        .spawn((
            ChildOf(root),
            Name::new(format!("{name} block")),
            Transform::default(),
            SectionMarker,
            ConnectedTo::default(),
            block,
            block.to_collider(),
            ColliderDensity(1.0),
            Health::new(200.0),
            Visibility::default(),
            children![(
                Name::new(format!("{name} block render")),
                Mesh3d(meshes.add(Cuboid::from_length(1.0))),
                MeshMaterial3d(materials.add(Color::from(tint))),
            )],
        ))
        .id();
    // The port stands one cell forward, so its own -Z face - the face the
    // capture is measured from - is the outermost thing on the hull.
    let port = commands
        .spawn((
            ChildOf(root),
            Name::new(format!("{name} port")),
            Transform::from_translation(Vec3::NEG_Z),
            SectionMarker,
            ConnectedTo(vec![hull]),
            block,
            block.to_collider(),
            ColliderDensity(1.0),
            Health::new(90.0),
            Visibility::default(),
            docking_section(DockingSectionConfig::default()),
            children![(
                Name::new(format!("{name} port render")),
                Mesh3d(meshes.add(Cuboid::from_length(0.9))),
                MeshMaterial3d(materials.add(Color::from(tailwind::SLATE_300))),
            )],
        ))
        .id();
    // A drive on the tail, so the throttle claims are made against a ship
    // that can actually burn. Its plume points aft with no rotation, the same
    // convention every catalog hull mounts a drive with.
    let drive = commands
        .spawn((
            ChildOf(root),
            Name::new(format!("{name} drive")),
            Transform::from_translation(Vec3::Z),
            SectionMarker,
            ConnectedTo(vec![hull]),
            block,
            block.to_collider(),
            ColliderDensity(1.0),
            Health::new(90.0),
            Visibility::default(),
            thruster_section(ThrusterSectionConfig::default()),
            children![(
                Name::new(format!("{name} drive render")),
                Mesh3d(meshes.add(Cuboid::from_length(0.8))),
                MeshMaterial3d(materials.add(Color::from(tailwind::SLATE_500))),
            )],
        ))
        .id();
    commands.entity(hull).insert(ConnectedTo(vec![port, drive]));
    (root, port)
}

fn setup_range(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut probe: ResMut<DockProbe>,
) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(6.0, 4.0, 6.0).looking_at(Vec3::NEG_Z * 1.5, Vec3::Y),
    ));
    commands.spawn((
        DirectionalLight {
            illuminance: 9_000.0,
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.8, -0.7, 0.0)),
    ));

    // Nose to nose: each port face stands 1.5 cells out from its own hull
    // origin, so the two hulls are three cells plus the gap apart.
    let separation = 3.0 + FACE_GAP;
    let (first, first_port) = spawn_hull(
        &mut commands,
        &mut meshes,
        &mut materials,
        "First",
        FIRST_HULL_ID,
        Vec3::ZERO,
        Quat::IDENTITY,
        tailwind::BLUE_500,
    );
    let (second, second_port) = spawn_hull(
        &mut commands,
        &mut meshes,
        &mut materials,
        "Second",
        SECOND_HULL_ID,
        Vec3::NEG_Z * separation,
        Quat::from_rotation_y(std::f32::consts::PI),
        tailwind::ORANGE_500,
    );

    probe.first = Some(first);
    probe.first_port = Some(first_port);
    probe.second = Some(second);
    probe.second_port = Some(second_port);
}

/// A body's avian pose - never `GlobalTransform`, which lags a fixed step.
fn body_pose(world: &World, body: Entity) -> Option<(Vec3, Quat)> {
    let position = world.get::<Position>(body)?.0;
    let rotation = world.get::<Rotation>(body)?.0;
    Some((position, rotation))
}

/// `second`'s pose expressed in `first`'s frame.
fn relative_pose(world: &World, first: Entity, second: Entity) -> Option<(Vec3, Quat)> {
    let (first_position, first_rotation) = body_pose(world, first)?;
    let (second_position, second_rotation) = body_pose(world, second)?;
    let inverse = first_rotation.inverse();
    Some((
        inverse * (second_position - first_position),
        inverse * second_rotation,
    ))
}

/// The half extents of a section's authored collider box.
fn collider_box(world: &World, section: Entity) -> Option<Vec3> {
    match world.get::<SectionCollider>(section)? {
        SectionCollider::Cuboid { size } => Some(*size),
        _ => None,
    }
}

fn connection_count(world: &mut World) -> usize {
    world.query::<&DockingConnection>().iter(world).count()
}

fn joint_count(world: &mut World) -> usize {
    world.query::<&FixedJoint>().iter(world).count()
}

fn request_dock(world: &mut World, first: Entity, second: Entity) {
    world.trigger(DockingConnectionRequest {
        entity: first,
        target: second,
    });
}

fn drive_range(world: &mut World) {
    let (first, first_port, second, second_port, stage, frames, stage_frames) = {
        let mut probe = world.resource_mut::<DockProbe>();
        probe.frames += 1;
        probe.stage_frames += 1;
        (
            probe.first.unwrap(),
            probe.first_port.unwrap(),
            probe.second.unwrap(),
            probe.second_port.unwrap(),
            probe.stage,
            probe.frames,
            probe.stage_frames,
        )
    };
    assert!(
        stage == Stage::Done || stage_frames < STAGE_BUDGET,
        "docking_ports: stuck in {stage:?} for {stage_frames} frames"
    );

    match stage {
        Stage::Settling => {
            settle_and_capture(world, first, first_port, second, second_port, frames)
        }
        Stage::Towing => tow(world, first, second),
        Stage::Holding => hold_against_the_throttle(world, first, second, stage_frames),
        Stage::Releasing => {
            release_on_request(world, first, first_port, second, second_port, stage_frames)
        }
        Stage::Destroying => release_on_destruction(world, first_port, second_port),
        Stage::Hearing => hear_the_scenario(world),
        Stage::Done => finish(world),
    }
}

/// Stage 1: let the bodies settle, dock them, and read what one command built.
fn settle_and_capture(
    world: &mut World,
    first: Entity,
    first_port: Entity,
    second: Entity,
    second_port: Entity,
    frames: u32,
) {
    // Avian needs a few steps to link the colliders and finish the masses.
    if frames < 30 {
        return;
    }
    if connection_count(world) == 0 {
        // Read the geometry BEFORE the capture, so the collider comparison
        // below is against the retracted port and not against itself.
        let retracted = collider_box(world, first_port);
        let mut probe = world.resource_mut::<DockProbe>();
        if probe.retracted_collider.is_none() {
            probe.retracted_collider = retracted;
        }
        request_dock(world, first, second);
        return;
    }

    let connections = connection_count(world);
    let joints = joint_count(world);
    assert_eq!(connections, 1, "docking_ports: one command, one connection");
    assert_eq!(joints, 1, "docking_ports: one connection, one root joint");
    for port in [first_port, second_port] {
        assert!(
            world.get::<DockedPort>(port).is_some(),
            "docking_ports: both ports must be reserved by the connection"
        );
    }
    for ship in [first, second] {
        assert!(
            world.get::<DockedShip>(ship).is_some(),
            "docking_ports: both hulls must know they are held"
        );
    }
    nova_probe::probe_marker(
        world,
        "outcome: one command builds one connection and one root joint",
        serde_json::json!({ "connections": connections, "joints": joints }),
    );

    let sleeves: Vec<DockingSectionState> = [first_port, second_port]
        .into_iter()
        .map(|port| {
            *world
                .get::<DockingSectionState>(port)
                .expect("a live port carries its sleeve state")
        })
        .collect();
    assert!(
        sleeves.iter().all(|state| matches!(
            state,
            DockingSectionState::Extending | DockingSectionState::Extended
        )),
        "docking_ports: both sleeves must reach out once the joint holds: {sleeves:?}"
    );
    nova_probe::probe_marker(
        world,
        "outcome: both sleeves reach out once the joint holds",
        serde_json::json!({ "sleeves": format!("{sleeves:?}") }),
    );

    let retracted = world.resource::<DockProbe>().retracted_collider;
    let extended = collider_box(world, first_port);
    assert_eq!(
        retracted, extended,
        "docking_ports: the sleeve is ART - extending it must not change the \
         box the hull collides with"
    );
    nova_probe::probe_marker(
        world,
        "outcome: the sleeve never changes what the hull collides with",
        serde_json::json!({ "collider": format!("{extended:?}") }),
    );

    let captured = relative_pose(world, first, second).expect("both hulls have a pose");
    let origin = body_pose(world, first)
        .expect("the first hull has a pose")
        .0;
    let gap = (captured.0.length() - 3.0).abs();
    let mut probe = world.resource_mut::<DockProbe>();
    probe.captured = Some(captured);
    probe.tow_origin = Some(origin);
    probe.gap = gap;
    probe.stage = Stage::Towing;
    probe.stage_frames = 0;
    info!("docking_ports: captured at a {gap:.2} unit face gap");
}

/// Stage 2: shove one hull and check the other comes with it, unchanged in
/// the first one's frame.
fn tow(world: &mut World, first: Entity, second: Entity) {
    let (origin, captured, stage_frames) = {
        let probe = world.resource::<DockProbe>();
        (
            probe.tow_origin.expect("a tow origin"),
            probe.captured.expect("a captured pose"),
            probe.stage_frames,
        )
    };

    // One push, on the FIRST hull only: everything the second hull does from
    // here is the joint's doing.
    if stage_frames == 1 {
        world
            .entity_mut(first)
            .insert(LinearVelocity(Vec3::X * TOW_SPEED.to_engine()));
        return;
    }

    let travelled = body_pose(world, first)
        .expect("the first hull has a pose")
        .0
        .distance(origin);
    if travelled < TOW_DISTANCE {
        return;
    }

    let (position, rotation) = relative_pose(world, first, second).expect("both hulls have a pose");
    let slip = position.distance(captured.0);
    let twist = rotation.angle_between(captured.1);
    assert!(
        slip < POSE_TOLERANCE,
        "docking_ports: the joint must hold the pose the hulls met in: slipped \
         {slip} units over {travelled} units of tow"
    );
    assert!(
        twist < POSE_TOLERANCE,
        "docking_ports: the joint must hold the CLOCKING the hulls met at: \
         twisted {twist} radians"
    );

    // Both hulls, because the claim is about the PAIR: the push went into one
    // of them and the joint shares it, so a docked pair that carried on
    // sailing is two bodies still moving, not one towing a dead weight. The
    // solver is never asked to hold a speed - the connection path is forbidden
    // from writing a velocity at all, and a hull that had been zeroed on
    // capture would read zero here.
    let speeds: Vec<f32> = [first, second]
        .into_iter()
        .map(|ship| {
            world
                .get::<LinearVelocity>(ship)
                .expect("a docked hull keeps its velocity")
                .0
                .length()
        })
        .collect();
    assert!(
        speeds.iter().all(|speed| *speed > TOWED_SPEED_FLOOR),
        "docking_ports: docking must never stop a hull: {speeds:?}"
    );
    nova_probe::probe_marker(
        world,
        "outcome: the joint carries the pair without zeroing its drift",
        serde_json::json!({
            "tow_mps": TOW_SPEED.get(),
            "pushed_hull_mps": MetersPerSecond::from_engine(speeds[0]).get(),
            "towed_hull_mps": MetersPerSecond::from_engine(speeds[1]).get(),
        }),
    );
    nova_probe::probe_marker(
        world,
        "outcome: the joint holds the pose the two hulls met in",
        serde_json::json!({
            "slip_m": Meters::from_engine(slip).get(),
            "twist_deg": twist.to_degrees(),
        }),
    );

    for ship in [first, second] {
        world.entity_mut(ship).insert(LinearVelocity(Vec3::ZERO));
        world.entity_mut(ship).insert(AngularVelocity(Vec3::ZERO));
    }
    let mut probe = world.resource_mut::<DockProbe>();
    probe.towed = travelled;
    probe.pose_error = slip;
    // The tow left the pair moving. Bring it back to rest by hand so the
    // modal claim is read against a pair that is standing still.
    probe.stage = Stage::Holding;
    probe.stage_frames = 0;
}

/// Stage 3: hold a full burn on the docked pair and read that nothing moves.
///
/// The claim is about FORCES. `manual_burn_system` never writes the input on a
/// docked root and `thruster_impulse_system` never applies the impulse on one,
/// so a throttle held here reaches the drive through neither path - and this
/// hull HAS a drive, which the next stage proves by using it.
fn hold_against_the_throttle(world: &mut World, first: Entity, second: Entity, stage_frames: u32) {
    if world.get::<FlightIntent>(second).is_none() {
        world.entity_mut(second).insert(FlightIntent { burn: 1.0 });
        return;
    }
    if stage_frames < HOLD_FRAMES {
        return;
    }

    assert_eq!(
        connection_count(world),
        1,
        "docking_ports: a throttle is not a release - only the verb is"
    );
    let speeds: Vec<f32> = [first, second]
        .into_iter()
        .map(|ship| {
            world
                .get::<LinearVelocity>(ship)
                .expect("a docked hull keeps its velocity")
                .0
                .length()
        })
        .collect();
    assert!(
        speeds.iter().all(|speed| *speed < HELD_SPEED_CEILING),
        "docking_ports: a docked hull's drive must be inert: {speeds:?}"
    );
    nova_probe::probe_marker(
        world,
        "outcome: a docked hull ignores the throttle",
        serde_json::json!({
            "held_frames": HOLD_FRAMES,
            "commanded_by": "the ship that did not issue DOCK",
            "pushed_hull_mps": MetersPerSecond::from_engine(speeds[1]).get(),
            "partner_hull_mps": MetersPerSecond::from_engine(speeds[0]).get(),
        }),
    );

    let mut probe = world.resource_mut::<DockProbe>();
    probe.stage = Stage::Releasing;
    probe.stage_frames = 0;
}

/// Stage 4: the hull that did NOT issue the command asks to be let go, and
/// the burn it was already holding takes it away.
///
/// The burn is deliberately left ON across the release. Stage 3's claim is
/// only worth something if the same command, on the same drive, moves the same
/// hull the moment the joint stops holding it.
fn release_on_request(
    world: &mut World,
    first: Entity,
    first_port: Entity,
    second: Entity,
    second_port: Entity,
    stage_frames: u32,
) {
    if connection_count(world) > 0 {
        if stage_frames == 1 {
            world.trigger(DockingReleaseRequest { entity: second });
        }
        return;
    }

    if world.resource::<DockProbe>().released_at.is_none() {
        assert_eq!(
            joint_count(world),
            0,
            "docking_ports: the joint goes with the connection"
        );
        for port in [first_port, second_port] {
            assert!(
                world.get::<DockedPort>(port).is_none(),
                "docking_ports: a released port is free again"
            );
            let state = *world
                .get::<DockingSectionState>(port)
                .expect("a live port carries its sleeve state");
            assert!(
                matches!(
                    state,
                    DockingSectionState::Retracting | DockingSectionState::Retracted
                ),
                "docking_ports: a released sleeve stows: {state:?}"
            );
        }
        for ship in [first, second] {
            assert!(
                world.get::<DockedShip>(ship).is_none(),
                "docking_ports: a released hull flies free"
            );
        }
        nova_probe::probe_marker(
            world,
            "outcome: the dock verb takes the dock away from either hull",
            serde_json::json!({ "asked_by": "the ship that did not issue DOCK" }),
        );
        world.resource_mut::<DockProbe>().released_at = Some(stage_frames);
        return;
    }

    // The burn is still held. Give it the same window the docked hull was
    // given, then read the drive it was reaching for all along.
    let released_at = world
        .resource::<DockProbe>()
        .released_at
        .expect("the release was recorded");
    if stage_frames < released_at + HOLD_FRAMES {
        return;
    }
    let freed = world
        .get::<LinearVelocity>(second)
        .expect("the freed hull keeps its velocity")
        .0
        .length();
    assert!(
        freed > FREED_SPEED_FLOOR,
        "docking_ports: the same burn must bite once the hull is free: {freed}"
    );
    nova_probe::probe_marker(
        world,
        "outcome: the throttle bites the moment the dock lets go",
        serde_json::json!({
            "freed_hull_mps": MetersPerSecond::from_engine(freed).get(),
            "held_frames": HOLD_FRAMES,
        }),
    );

    // Stop the burn and bring the pair back to rest for the last claim: a
    // hull that is still commanding a burn can never hold a second dock.
    world.entity_mut(second).remove::<FlightIntent>();
    for ship in [first, second] {
        world.entity_mut(ship).insert(LinearVelocity(Vec3::ZERO));
        world.entity_mut(ship).insert(AngularVelocity(Vec3::ZERO));
    }
    let mut probe = world.resource_mut::<DockProbe>();
    probe.stage = Stage::Destroying;
    probe.stage_frames = 0;
}

/// Stage 4: dock again, then destroy one port and read what the survivor is
/// left holding.
fn release_on_destruction(world: &mut World, first_port: Entity, second_port: Entity) {
    let (first, second) = {
        let probe = world.resource::<DockProbe>();
        (probe.first.unwrap(), probe.second.unwrap())
    };

    if connection_count(world) == 0 && world.get_entity(first_port).is_ok() {
        request_dock(world, first, second);
        return;
    }
    if world.get_entity(first_port).is_ok() {
        // Docked again - now take the port off the ship.
        world.trigger(HealthApplyDamage {
            entity: first_port,
            source: None,
            amount: 1_000.0,
        });
        return;
    }
    if connection_count(world) > 0 {
        return;
    }

    assert_eq!(
        joint_count(world),
        0,
        "docking_ports: a destroyed endpoint takes the joint with it"
    );
    assert!(
        world.get::<DockedPort>(second_port).is_none(),
        "docking_ports: the surviving port is free"
    );
    assert!(
        world.get::<DockedShip>(second).is_none(),
        "docking_ports: the surviving hull flies free"
    );
    nova_probe::probe_marker(
        world,
        "outcome: a destroyed port frees its partner",
        serde_json::json!({ "surviving_port": format!("{second_port:?}") }),
    );

    let (gap, towed, pose_error) = {
        let probe = world.resource::<DockProbe>();
        (probe.gap, probe.towed, probe.pose_error)
    };
    nova_probe::probe_marker(
        world,
        "outcome: the dock geometry is recorded",
        serde_json::json!({
            "face_gap_m": Meters::from_engine(gap).get(),
            "tow_distance_m": Meters::from_engine(towed).get(),
            "pose_error_m": Meters::from_engine(pose_error).get(),
        }),
    );

    let mut probe = world.resource_mut::<DockProbe>();
    probe.stage = Stage::Hearing;
    probe.stage_frames = 0;
}

/// Stage 6: read what the SCENARIO heard.
///
/// The physical claims above are made against the connection entity; this one
/// is made against the authored vocabulary on the other side of the tracker.
/// It polls rather than asserting straight away because the release edge is
/// two hops behind the despawn - the tracker reads the connections on the
/// fixed clock, and the handler it wakes runs at the end of the frame - and
/// the stage budget is what calls a chain that never arrives stuck.
fn hear_the_scenario(world: &mut World) {
    let tally = |world: &World, key: &str| {
        match world.resource::<NovaEventWorld>().get_variable(key) {
            Some(VariableLiteral::Number(value)) => *value,
            // The seeding handler runs on `OnStart`, so a missing key means
            // the scenario has not started yet, not that the count is zero.
            _ => f64::NAN,
        }
    };
    let (docked, undocked) = (tally(world, DOCKED_TALLY), tally(world, UNDOCKED_TALLY));
    if docked < EXPECTED_DOCKS || undocked < EXPECTED_DOCKS {
        return;
    }
    assert_eq!(
        (docked, undocked),
        (EXPECTED_DOCKS, EXPECTED_DOCKS),
        "docking_ports: each capture and each release must wake its handler \
         exactly once"
    );
    nova_probe::probe_marker(
        world,
        "outcome: the scenario hears every dock and every release",
        serde_json::json!({
            "docked": docked,
            "undocked": undocked,
            "endings": ["the verb, asked for by the hull that did not dock", "a destroyed port"],
        }),
    );

    let mut probe = world.resource_mut::<DockProbe>();
    probe.stage = Stage::Done;
    probe.stage_frames = 0;
    info!("docking_ports: every docking invariant held");
}

/// Every claim made: idle a moment so a hand-run has something to look at,
/// then leave.
fn finish(world: &mut World) {
    if std::env::var_os("NOVA_AUTOPILOT").is_none() {
        return;
    }
    let exit = {
        let mut probe = world.resource_mut::<DockProbe>();
        probe.exit_delay += 1;
        probe.exit_delay >= 90
    };
    if exit {
        world.write_message(AppExit::Success);
    }
}
