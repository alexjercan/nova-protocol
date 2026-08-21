//! system_section_severing: a destroyed interior section becomes a real hole and
//! detaches the structure behind it as an independent wreck body.
//!
//! BLUE is the controller-bearing command component. RED is the bridge that is
//! destroyed. ORANGE is the healthy component that severs and drifts free.
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
    exit_delay: u32,
}

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();
    let mut app = AppBuilder::new().with_game_plugins(range_plugin).build();
    app.add_plugins(nova_probe::NovaProbePlugin::default());
    app.run()
}

fn range_plugin(app: &mut App) {
    // NOTE: `ShipIntegrityPlugin` is NOT added here. `NovaShipPlugin` brings it
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
    commands
        .spawn((
            Name::new(format!("{part:?}")),
            ChildOf(root),
            Transform::from_translation(at),
            SectionMarker,
            ConnectedTo::default(),
            Collider::cuboid(1.0, 1.0, 1.0),
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

fn setup_range(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut probe: ResMut<SeverProbe>,
) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(9.0, 8.0, 16.0).looking_at(Vec3::new(1.5, 0.0, 0.0), Vec3::Y),
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
    let cube = meshes.add(Cuboid::new(1.0, 1.0, 1.0));
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
        Vec3::X,
    );
    let front = spawn_part(
        &mut commands,
        root,
        cube.clone(),
        materials.add(Color::from(tailwind::ORANGE_500)),
        ProbePart::DetachedFront,
        Vec3::X * 2.0,
    );
    let rear = spawn_part(
        &mut commands,
        root,
        cube,
        materials.add(Color::from(tailwind::ORANGE_300)),
        ProbePart::DetachedRear,
        Vec3::X * 3.0,
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

    world.resource_mut::<SeverProbe>().verified = true;
    info!("section_severing: every sever invariant held");
}
