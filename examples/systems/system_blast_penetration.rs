//! system_blast_penetration: explosive falloff, structural shielding and salvo ordering.
//!
//! Three lanes isolate the blast-pressure rules on compound rigid bodies:
//!
//! - RED: an outer section dies, the middle section receives attenuated pressure
//!   and survives, and the rear section is shielded;
//! - BLUE: two simultaneous blasts combine on one blocker but cannot use the
//!   hole they create in that same fixed tick;
//! - GREEN: a non-structural plate takes radial damage without consuming a
//!   structural penetration layer, while the section behind it shields a rear
//!   fixture.
//!
//! The old blast path makes this range fail: it measures every child collider
//! from the shared body root, then deals unattenuated damage to every overlap.
//!
//! Headless smoke test:
//!
//! ```text
//! NOVA_AUTOPILOT=1 cargo run --example system_blast_penetration --features debug
//! ```

use avian3d::prelude::*;
use bevy::{color::palettes::tailwind, prelude::*};
use clap::Parser;
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "system_blast_penetration")]
#[command(version = "1.0.0")]
#[command(about = "A layered-section range for explosive pressure. Autopilot-only correctness range", long_about = None)]
struct Cli;

const BLAST_RADIUS: f32 = 4.0;
const BLAST_DAMAGE: f32 = 300.0;
const SALVO_DAMAGE: f32 = 200.0;
const EPSILON: f32 = 0.1;

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum ProbePart {
    LayerOuter,
    LayerMiddle,
    LayerRear,
    SalvoBlocker,
    SalvoRear,
    Plate,
    PlateRear,
    PlateFar,
}

#[derive(Resource, Default)]
struct DamageSeen {
    amounts: std::collections::HashMap<ProbePart, f32>,
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
    app.init_resource::<DamageSeen>();
    app.add_observer(record_probe_damage);
    app.add_systems(Startup, setup_range);
    // Gated on `Playing`, because the range EXITS on a frame count once its
    // assertions hold. Headless the blasts resolve inside 32 frames, which is
    // sooner than the loading screen finishes, so an ungated range shut the app
    // down before it ever entered `Playing` and failed `reached_playing` while
    // every pressure assertion passed.
    app.add_systems(Update, verify_range.run_if(in_state(GameStates::Playing)));
}

fn spawn_part(
    commands: &mut Commands,
    root: Entity,
    mesh: Handle<Mesh>,
    material: Handle<StandardMaterial>,
    part: ProbePart,
    position: Vec3,
    health: f32,
    structural: bool,
) {
    let mut entity = commands.spawn((
        Name::new(format!("{part:?}")),
        ChildOf(root),
        Transform::from_translation(position),
        Mesh3d(mesh),
        MeshMaterial3d(material),
        Collider::cuboid(1.0, 1.0, 1.0),
        ColliderDensity(1.0),
        Health::new(health),
        part,
    ));
    if structural {
        entity.insert(SectionMarker);
    } else {
        entity.insert(HealthIsolated);
    }
}

fn spawn_blast(commands: &mut Commands, at: Vec3, damage: f32) {
    commands.spawn((
        nova_blast(BLAST_RADIUS, damage, DamageType::Explosive),
        Transform::from_translation(at),
        TempEntity(0.1),
    ));
}

fn setup_range(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(12.0, 12.0, 30.0).looking_at(Vec3::new(1.5, 10.0, 0.0), Vec3::Y),
    ));
    commands.spawn((
        DirectionalLight {
            illuminance: 8_000.0,
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.8, -0.7, 0.0)),
    ));

    let cube = meshes.add(Cuboid::new(1.0, 1.0, 1.0));
    let red = materials.add(Color::from(tailwind::RED_500));
    let blue = materials.add(Color::from(tailwind::BLUE_500));
    let green = materials.add(Color::from(tailwind::GREEN_500));
    let gray = materials.add(Color::from(tailwind::SLATE_400));

    let layered = commands
        .spawn((
            Name::new("Layered structural lane"),
            RigidBody::Static,
            Transform::from_xyz(0.0, 0.0, 0.0),
            Visibility::default(),
        ))
        .id();
    spawn_part(
        &mut commands,
        layered,
        cube.clone(),
        red.clone(),
        ProbePart::LayerOuter,
        Vec3::X * 1.0,
        200.0,
        true,
    );
    spawn_part(
        &mut commands,
        layered,
        cube.clone(),
        red.clone(),
        ProbePart::LayerMiddle,
        Vec3::X * 2.0,
        150.0,
        true,
    );
    spawn_part(
        &mut commands,
        layered,
        cube.clone(),
        red,
        ProbePart::LayerRear,
        Vec3::X * 3.0,
        100.0,
        true,
    );
    spawn_blast(&mut commands, Vec3::ZERO, BLAST_DAMAGE);

    let salvo_origin = Vec3::Y * 10.0;
    let salvo = commands
        .spawn((
            Name::new("Atomic salvo lane"),
            RigidBody::Static,
            Transform::from_translation(salvo_origin),
            Visibility::default(),
        ))
        .id();
    spawn_part(
        &mut commands,
        salvo,
        cube.clone(),
        blue.clone(),
        ProbePart::SalvoBlocker,
        Vec3::X * 1.0,
        300.0,
        true,
    );
    spawn_part(
        &mut commands,
        salvo,
        cube.clone(),
        blue,
        ProbePart::SalvoRear,
        Vec3::X * 2.0,
        100.0,
        true,
    );
    spawn_blast(&mut commands, salvo_origin, SALVO_DAMAGE);
    spawn_blast(&mut commands, salvo_origin, SALVO_DAMAGE);

    let plate_origin = Vec3::Y * 20.0;
    let plated = commands
        .spawn((
            Name::new("Non-structural plate lane"),
            RigidBody::Static,
            Transform::from_translation(plate_origin),
            Visibility::default(),
        ))
        .id();
    spawn_part(
        &mut commands,
        plated,
        cube.clone(),
        gray.clone(),
        ProbePart::Plate,
        Vec3::X * 1.0,
        500.0,
        false,
    );
    spawn_part(
        &mut commands,
        plated,
        cube.clone(),
        green,
        ProbePart::PlateRear,
        Vec3::X * 2.0,
        500.0,
        true,
    );
    spawn_part(
        &mut commands,
        plated,
        cube,
        gray,
        ProbePart::PlateFar,
        Vec3::X * 3.0,
        500.0,
        false,
    );
    spawn_blast(&mut commands, plate_origin, BLAST_DAMAGE);
}

fn record_probe_damage(
    damage: On<HealthApplyDamage>,
    q_probe: Query<&ProbePart>,
    mut seen: ResMut<DamageSeen>,
) {
    if damage.entity != damage.original_event_target() {
        return;
    }
    let Ok(part) = q_probe.get(damage.entity) else {
        return;
    };
    *seen.amounts.entry(*part).or_default() += damage.amount;
}

fn near(actual: f32, expected: f32) -> bool {
    (actual - expected).abs() <= EPSILON
}

fn verify_range(world: &mut World) {
    if world.resource::<DamageSeen>().verified {
        if std::env::var_os("NOVA_AUTOPILOT").is_some() {
            let exit = {
                let mut seen = world.resource_mut::<DamageSeen>();
                seen.exit_delay += 1;
                seen.exit_delay >= 30
            };
            if exit {
                world.write_message(AppExit::Success);
            }
        }
        return;
    }

    let ready = {
        let seen = world.resource::<DamageSeen>();
        seen.amounts.contains_key(&ProbePart::LayerOuter)
            && seen.amounts.contains_key(&ProbePart::LayerMiddle)
            && seen.amounts.contains_key(&ProbePart::SalvoBlocker)
            && seen.amounts.contains_key(&ProbePart::Plate)
            && seen.amounts.contains_key(&ProbePart::PlateRear)
    };
    if !ready {
        return;
    }

    let amounts = world.resource::<DamageSeen>().amounts.clone();
    let amount = |part| amounts.get(&part).copied().unwrap_or(0.0);

    assert!(
        near(amount(ProbePart::LayerOuter), 200.0) && near(amount(ProbePart::LayerMiddle), 97.5),
        "blast_penetration: section-centre falloff and 65% transmission drifted: {amounts:?}"
    );
    nova_probe::probe_marker(
        world,
        "outcome: destroyed section attenuates pressure",
        serde_json::json!({
            "outer": amount(ProbePart::LayerOuter),
            "middle": amount(ProbePart::LayerMiddle),
        }),
    );

    assert!(
        near(amount(ProbePart::LayerRear), 0.0),
        "blast_penetration: the surviving middle section did not shield the rear: {amounts:?}"
    );
    nova_probe::probe_marker(
        world,
        "outcome: surviving section stops pressure",
        serde_json::json!({ "rear": amount(ProbePart::LayerRear) }),
    );

    assert!(
        near(amount(ProbePart::SalvoBlocker), 300.0) && near(amount(ProbePart::SalvoRear), 0.0),
        "blast_penetration: same-tick blasts tunneled through their shared blocker: {amounts:?}"
    );
    nova_probe::probe_marker(
        world,
        "outcome: simultaneous blasts share one health snapshot",
        serde_json::json!({
            "blocker": amount(ProbePart::SalvoBlocker),
            "rear": amount(ProbePart::SalvoRear),
        }),
    );

    assert!(
        near(amount(ProbePart::Plate), 225.0) && near(amount(ProbePart::PlateRear), 150.0),
        "blast_penetration: a non-structural plate consumed penetration: {amounts:?}"
    );
    nova_probe::probe_marker(
        world,
        "outcome: fixtures do not consume penetration",
        serde_json::json!({
            "plate": amount(ProbePart::Plate),
            "rear": amount(ProbePart::PlateRear),
        }),
    );

    assert!(
        near(amount(ProbePart::PlateFar), 0.0),
        "blast_penetration: pressure travelled around the section into a rear fixture: {amounts:?}"
    );
    nova_probe::probe_marker(
        world,
        "outcome: sections shield fixtures behind them",
        serde_json::json!({ "far_fixture": amount(ProbePart::PlateFar) }),
    );

    world.resource_mut::<DamageSeen>().verified = true;
    info!("blast_penetration: every pressure invariant held");
}
