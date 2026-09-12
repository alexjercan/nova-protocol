//! system_section_severing: a destroyed interior section becomes a real hole and
//! detaches the structure behind it as an independent wreck body.
//!
//! BLUE is the controller-bearing command component. RED is the bridge that is
//! destroyed. ORANGE is the healthy component that severs and drifts free.
//!
//! The components are three build-grid cells on a side, so the wreck the cut
//! makes is large enough that its OWN size, and not the 10 m/s floor, sets the
//! speed it leaves at. That is the fifth claim: a sever opens a gap as wide as
//! the two halves it made in about two seconds, whatever their size.
//!
//! Headless smoke test:
//!
//! ```text
//! NOVA_AUTOPILOT=1 cargo run --example system_section_severing --features debug
//! ```

use avian3d::prelude::*;
use bevy::{color::palettes::tailwind, prelude::*};
use clap::Parser;
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "system_section_severing")]
#[command(version = "1.0.0")]
#[command(about = "Interior section destruction and physical wreck severing. Autopilot-only correctness range", long_about = None)]
struct Cli;

/// The edge of one probe component.
///
/// Three build-grid cells. Big enough that both halves of the cut are past the
/// separation floor, so this range reads the size rule rather than the floor.
const PART_SIZE: Meters = Meters(30.0);

/// The separation speed a fragment gets when its own size does not ask for a
/// faster one. Mirrored from `nova_ship::sections::integrity`.
const SEVER_MIN_SEPARATION_SPEED: MetersPerSecond = MetersPerSecond(10.0);

/// How long a sever is given to open a gap as wide as the halves it made.
/// Mirrored from the same module.
const SEVER_CLEARANCE_SECS: f32 = 2.0;

/// How far the measured fracture speed may sit from the rule, u/s.
///
/// A settling allowance, not a band: the two bodies are re-based on the frame
/// they are made, and the reading is taken one solve later.
const KICK_TOLERANCE: f32 = 0.2;

/// Frames the fracture kick is given to land before the range calls it missing.
const KICK_FRAME_BUDGET: u32 = 120;

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
enum ProbePart {
    Command,
    Bridge,
    DetachedFront,
    DetachedRear,
}

#[derive(Resource, Default)]
struct SeverProbe {
    root: Option<Entity>,
    bridge: Option<Entity>,
    detached: Option<Entity>,
    frames: u32,
    fired: bool,
    verified: bool,
    settling: u32,
    exit_delay: u32,
}

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();
    let mut app = AppBuilder::new().with_game_plugins(range_plugin).build();
    app.add_plugins(nova_probe::NovaProbePlugin::default());
    app.run()
}

fn range_plugin(app: &mut App) {
    // `ShipIntegrityPlugin` is NOT added here. `NovaShipPlugin` brings it
    // (`sections/mod.rs`), and bevy panics on a duplicate. The hand-assembled
    // app this range used to build did not, which is exactly the class of thing
    // `AppBuilder` exists to stop a range from having to know.
    app.init_resource::<SeverProbe>();
    app.add_systems(Startup, setup_range);
    // Gated on `Playing`, because the range EXITS on a frame count once the
    // sever invariants hold. Headless it verifies and shuts down inside 136
    // frames, sooner than the loading screen finishes, so an ungated range
    // failed `reached_playing` while every assertion passed.
    app.add_systems(Update, drive_range.run_if(in_state(GameStates::Playing)));
}

fn spawn_part(
    commands: &mut Commands,
    root: Entity,
    mesh: Handle<Mesh>,
    material: Handle<StandardMaterial>,
    part: ProbePart,
    at: Vec3,
) -> Entity {
    let collider = SectionCollider::Cuboid {
        size: Vec3::splat(PART_SIZE.to_engine()),
    };
    commands
        .spawn((
            Name::new(format!("{part:?}")),
            ChildOf(root),
            Transform::from_translation(at),
            SectionMarker,
            ConnectedTo::default(),
            collider,
            collider.to_collider(),
            ColliderDensity(1.0),
            Health::new(100.0),
            Visibility::default(),
            part,
            children![(
                Name::new(format!("{part:?} render")),
                Mesh3d(mesh),
                MeshMaterial3d(material),
            )],
        ))
        .id()
}

/// The raw speed one severed fragment leaves at, from its own containment
/// radius. Mirrored from `nova_ship::sections::integrity`.
fn separation_speed(envelope: f32) -> f32 {
    (envelope / SEVER_CLEARANCE_SECS).max(SEVER_MIN_SEPARATION_SPEED.to_engine())
}

/// The containment radius of a group of probe components: the distance from the
/// group's centre of mass to the furthest corner of its furthest cube.
///
/// The components are one size and one density, so their centre of mass is the
/// mean of their offsets and this range can state the figure the sever rule
/// reads without borrowing the code that computes it.
fn containment_radius(offsets: &[f32]) -> f32 {
    let half = PART_SIZE.to_engine() * 0.5;
    let center = offsets.iter().sum::<f32>() / offsets.len() as f32;
    offsets
        .iter()
        .map(|offset| Vec3::new((center - offset).abs() + half, half, half).length())
        .fold(0.0_f32, f32::max)
}

/// One body's world-space centre of mass and linear velocity.
fn body_motion(world: &World, body: Entity) -> Option<(Vec3, Vec3)> {
    let position = world.get::<Position>(body)?.0;
    let rotation = world.get::<Rotation>(body)?.0;
    let center = world.get::<ComputedCenterOfMass>(body)?.0;
    let velocity = world.get::<LinearVelocity>(body)?.0;
    Some((position + rotation * center, velocity))
}

fn setup_range(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut probe: ResMut<SeverProbe>,
) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(27.0, 24.0, 48.0).looking_at(Vec3::new(4.5, 0.0, 0.0), Vec3::Y),
    ));
    commands.spawn((
        DirectionalLight {
            illuminance: 9_000.0,
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.8, -0.7, 0.0)),
    ));

    let root = commands
        .spawn((
            Name::new("Severing subject"),
            SpaceshipRootMarker,
            RigidBody::Dynamic,
            Transform::default(),
            Visibility::default(),
            AngularVelocity(Vec3::new(0.0, 0.15, 0.0)),
            TransformInterpolation,
        ))
        .id();
    let step = PART_SIZE.to_engine();
    let cube = meshes.add(Cuboid::from_length(step));
    let command = spawn_part(
        &mut commands,
        root,
        cube.clone(),
        materials.add(Color::from(tailwind::BLUE_500)),
        ProbePart::Command,
        Vec3::ZERO,
    );
    commands.entity(command).insert(ControllerSectionMarker);
    let bridge = spawn_part(
        &mut commands,
        root,
        cube.clone(),
        materials.add(Color::from(tailwind::RED_500)),
        ProbePart::Bridge,
        Vec3::X * step,
    );
    let front = spawn_part(
        &mut commands,
        root,
        cube.clone(),
        materials.add(Color::from(tailwind::ORANGE_500)),
        ProbePart::DetachedFront,
        Vec3::X * 2.0 * step,
    );
    let rear = spawn_part(
        &mut commands,
        root,
        cube,
        materials.add(Color::from(tailwind::ORANGE_300)),
        ProbePart::DetachedRear,
        Vec3::X * 3.0 * step,
    );

    commands.entity(command).insert(ConnectedTo(vec![bridge]));
    commands
        .entity(bridge)
        .insert(ConnectedTo(vec![command, front]));
    commands
        .entity(front)
        .insert(ConnectedTo(vec![bridge, rear]));
    commands.entity(rear).insert(ConnectedTo(vec![front]));

    probe.root = Some(root);
    probe.bridge = Some(bridge);
    probe.detached = Some(front);
}

fn drive_range(world: &mut World) {
    {
        let mut probe = world.resource_mut::<SeverProbe>();
        probe.frames += 1;
    }

    let (root, bridge, detached, frames, fired, verified) = {
        let probe = world.resource::<SeverProbe>();
        (
            probe.root.unwrap(),
            probe.bridge.unwrap(),
            probe.detached.unwrap(),
            probe.frames,
            probe.fired,
            probe.verified,
        )
    };

    if !fired && frames >= 45 {
        world.trigger(HealthApplyDamage {
            entity: bridge,
            source: None,
            amount: 100.0,
        });
        world.resource_mut::<SeverProbe>().fired = true;
        return;
    }

    if verified {
        if std::env::var_os("NOVA_AUTOPILOT").is_some() {
            let exit = {
                let mut probe = world.resource_mut::<SeverProbe>();
                probe.exit_delay += 1;
                probe.exit_delay >= 90
            };
            if exit {
                world.write_message(AppExit::Success);
            }
        }
        return;
    }
    if !fired || world.get_entity(bridge).is_ok() {
        return;
    }

    let fragments: Vec<_> = world
        .query_filtered::<Entity, With<ShipWreckFragmentMarker>>()
        .iter(world)
        .collect();
    if fragments.len() != 1 {
        return;
    }
    let fragment = fragments[0];
    let command = world
        .query_filtered::<Entity, With<ControllerSectionMarker>>()
        .single(world)
        .expect("one command section");
    let Some(command_body) = world.get::<ColliderOf>(command) else {
        return;
    };
    let Some(detached_body) = world.get::<ColliderOf>(detached) else {
        return;
    };
    if command_body.body != root || detached_body.body != fragment {
        return;
    }

    let detached_health = world
        .get::<Health>(detached)
        .expect("detached health")
        .current;
    assert!(
        detached_health > 0.0 && world.get::<SectionInactiveMarker>(detached).is_some(),
        "section_severing: detached structure must be intact, inert, and damageable"
    );
    let step = PART_SIZE.to_engine();
    let hull_envelope = containment_radius(&[0.0]);
    let fragment_envelope = containment_radius(&[2.0 * step, 3.0 * step]);
    let expected_fracture = separation_speed(hull_envelope) + separation_speed(fragment_envelope);
    let (Some((hull_com, hull_velocity)), Some((fragment_com, fragment_velocity))) =
        (body_motion(world, root), body_motion(world, fragment))
    else {
        return;
    };
    let spin = world
        .get::<AngularVelocity>(root)
        .map_or(Vec3::ZERO, |spin| spin.0);
    // The cut alone is the subject: the pre-cut spin carries the two centres of
    // mass apart by itself, so take that off before reading the kick.
    let fracture = fragment_velocity - hull_velocity - spin.cross(fragment_com - hull_com);
    if fracture.length() < 0.5 * expected_fracture {
        let settling = {
            let mut probe = world.resource_mut::<SeverProbe>();
            probe.settling += 1;
            probe.settling
        };
        assert!(
            settling < KICK_FRAME_BUDGET,
            "section_severing: the fracture kick never landed: {fracture:?}"
        );
        return;
    }
    assert!(
        fragment_envelope / SEVER_CLEARANCE_SECS > SEVER_MIN_SEPARATION_SPEED.to_engine(),
        "section_severing: the wreck must be past the separation floor, or this reads the floor"
    );
    assert!(
        (fracture.length() - expected_fracture).abs() < KICK_TOLERANCE,
        "section_severing: the halves must part at the speed their own sizes ask for: {} u/s against {expected_fracture} u/s",
        fracture.length()
    );

    nova_probe::probe_marker(
        world,
        "outcome: interior section becomes a hole",
        serde_json::json!({ "bridge_gone": true }),
    );
    nova_probe::probe_marker(
        world,
        "outcome: detached component gets a rigid body",
        serde_json::json!({ "fragment": format!("{fragment:?}") }),
    );
    nova_probe::probe_marker(
        world,
        "outcome: command component keeps ship identity",
        serde_json::json!({ "ship": format!("{root:?}") }),
    );
    nova_probe::probe_marker(
        world,
        "outcome: wreck remains inert and damageable",
        serde_json::json!({ "health": detached_health }),
    );
    nova_probe::probe_marker(
        world,
        "outcome: a fragment leaves at the speed its own size asks for",
        serde_json::json!({
            "hull_envelope_m": Meters::from_engine(hull_envelope).get(),
            "hull_speed_mps": MetersPerSecond::from_engine(separation_speed(hull_envelope)).get(),
            "fragment_envelope_m": Meters::from_engine(fragment_envelope).get(),
            "fragment_speed_mps":
                MetersPerSecond::from_engine(separation_speed(fragment_envelope)).get(),
            "fracture_mps": MetersPerSecond::from_engine(fracture.length()).get(),
            "floor_mps": SEVER_MIN_SEPARATION_SPEED.get(),
        }),
    );

    world.resource_mut::<SeverProbe>().verified = true;
    info!("section_severing: every sever invariant held");
}
