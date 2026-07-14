//! Nav beacon scenario object (task 20260712-093044, spike
//! docs/spikes/20260712-092926-starter-scenario.md): a small emissive,
//! blinking marker body the player navigates by. The HUD side (label +
//! distance chip, edge-clamped direction cue) hangs off [`BeaconMarker`]
//! in nova_gameplay; this module owns the world-side body: mesh, blink,
//! sensor collider, and - when `area_radius` is set - the beacon doubling
//! as its own trigger area, firing `OnEnter` under the beacon's scenario
//! id with no separate `CreateScenarioArea` needed.

use avian3d::prelude::*;
use bevy::prelude::*;
use nova_events::prelude::*;
use nova_gameplay::prelude::*;

use crate::prelude::*;

pub mod prelude {
    pub use super::{beacon_scenario_object, BeaconConfig, BeaconPlugin, BEACON_TYPE_NAME};
}

pub const BEACON_TYPE_NAME: &str = "beacon";

/// The lock scanner sees a beacon like a well-sized rock: a nav point is
/// exactly the thing the player locks to GOTO, so it must be acquirable
/// from a full tutorial leg away (signature * range-per-unit, 20 * 30 =
/// 600u at the default settings), not at debris range.
const BEACON_LOCK_SIGNATURE: f32 = 20.0;

/// Blink period (seconds) of the emissive pulse.
const BEACON_BLINK_PERIOD_SECS: f32 = 1.2;

/// Emissive luminance range the blink sweeps (dim floor, not off - the
/// beacon must stay findable mid-blink).
const BEACON_EMISSIVE_MAX: f32 = 60.0;
const BEACON_EMISSIVE_MIN: f32 = 8.0;

#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BeaconConfig {
    /// The short name the HUD chip shows ("BEACON 1").
    pub label: String,
    /// Visual radius of the beacon body (world units).
    pub radius: f32,
    /// Beacon light color (base and emissive).
    pub color: Color,
    /// When set, the beacon is also its own trigger area of this radius:
    /// `OnEnter`/`OnExit` fire with the beacon's scenario id.
    pub area_radius: Option<f32>,
    /// Radar signature override; `None` = the default
    /// [`BEACON_LOCK_SIGNATURE`] (600u lock range). A scenario whose GOTO
    /// leg is longer than that authors the signature the leg needs
    /// (shakedown's waypoint run, task 20260713-140929).
    pub lock_signature: Option<f32>,
}

pub fn beacon_scenario_object(config: BeaconConfig) -> impl Bundle {
    debug!("beacon_scenario_object: config {:?}", config);

    (
        BeaconMarker,
        EntityTypeName::new(BEACON_TYPE_NAME),
        BeaconLabel(config.label),
        BeaconRenderConfig {
            radius: config.radius,
            color: config.color,
        },
        BeaconAreaRadius(config.area_radius),
        // A nav point holds its position: on rails like a well source,
        // overriding the base scenario bundle's Dynamic. Lockability is
        // preserved by the authored LockSignature (the targeting gate
        // admits Static bodies with one; input/targeting.rs).
        RigidBody::Static,
        LockSignature(config.lock_signature.unwrap_or(BEACON_LOCK_SIGNATURE)),
    )
}

/// Render inputs, consumed by `insert_beacon_render`.
#[derive(Component, Clone, Debug, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BeaconRenderConfig {
    pub radius: f32,
    pub color: Color,
}

/// The authored trigger radius (None = plain marker, no area role).
#[derive(Component, Clone, Debug, Deref, DerefMut, Reflect)]
pub struct BeaconAreaRadius(pub Option<f32>);

/// The blink driver on the beacon's render child: the material it pulses
/// and its phase offset (seeded from the entity index so a row of beacons
/// does not strobe in unison).
#[derive(Component, Clone, Debug, Reflect)]
pub struct BeaconBlink {
    pub material: Handle<StandardMaterial>,
    pub phase: f32,
}

pub struct BeaconPlugin {
    pub render: bool,
}

impl Plugin for BeaconPlugin {
    fn build(&self, app: &mut App) {
        debug!("BeaconPlugin: build");

        app.add_observer(insert_beacon_area);
        if self.render {
            app.add_observer(insert_beacon_render);
            app.add_systems(Update, blink_beacons);
        }
    }
}

/// A beacon with an authored area radius becomes its own trigger volume:
/// sensor sphere + [`ScenarioAreaMarker`], so the area plugin wires the
/// collision events and `OnEnter` fires under the beacon's id.
fn insert_beacon_area(
    add: On<Add, BeaconMarker>,
    mut commands: Commands,
    q_beacon: Query<&BeaconAreaRadius, With<BeaconMarker>>,
) {
    let entity = add.entity;
    let Ok(area_radius) = q_beacon.get(entity) else {
        return;
    };
    let Some(radius) = **area_radius else {
        return;
    };
    trace!("insert_beacon_area: entity {:?} radius {}", entity, radius);
    commands
        .entity(entity)
        .insert((ScenarioAreaMarker, Collider::sphere(radius), Sensor));
}

/// The visible beacon: an emissive orb child scaled to the config radius,
/// carrying the blink driver. A child (not the root) so the trigger
/// collider on the root keeps its own radius.
fn insert_beacon_render(
    add: On<Add, BeaconMarker>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    q_beacon: Query<&BeaconRenderConfig, With<BeaconMarker>>,
) {
    let entity = add.entity;
    let Ok(config) = q_beacon.get(entity) else {
        error!(
            "insert_beacon_render: entity {:?} not found in q_beacon",
            entity
        );
        return;
    };

    let material = materials.add(StandardMaterial {
        base_color: config.color,
        emissive: config.color.to_linear() * BEACON_EMISSIVE_MAX,
        unlit: false,
        ..default()
    });

    commands.entity(entity).insert(children![(
        Name::new("BeaconOrb"),
        Transform::from_scale(Vec3::splat(config.radius)),
        Mesh3d(meshes.add(Sphere::new(1.0))),
        MeshMaterial3d(material.clone()),
        BeaconBlink {
            material,
            phase: entity.index_u32() as f32 * 0.7,
        },
    )]);
}

/// Pulse each beacon's emissive between the dim floor and full glow.
fn blink_beacons(
    time: Res<Time>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    q_blink: Query<(&BeaconBlink, &MeshMaterial3d<StandardMaterial>)>,
) {
    for (blink, material_handle) in &q_blink {
        let Some(mut material) = materials.get_mut(&material_handle.0) else {
            continue;
        };
        let t = time.elapsed_secs() * std::f32::consts::TAU / BEACON_BLINK_PERIOD_SECS;
        let wave = 0.5 + 0.5 * (t + blink.phase).sin();
        let luminance = BEACON_EMISSIVE_MIN + (BEACON_EMISSIVE_MAX - BEACON_EMISSIVE_MIN) * wave;
        material.emissive = material.base_color.to_linear() * luminance;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config(area_radius: Option<f32>) -> BeaconConfig {
        BeaconConfig {
            label: "BEACON 1".to_string(),
            radius: 2.0,
            color: Color::srgb(0.3, 0.9, 1.0),
            area_radius,
            lock_signature: None,
        }
    }

    #[test]
    fn the_signature_override_beats_the_default() {
        let mut world = World::new();
        let default_beacon = world.spawn(beacon_scenario_object(config(None))).id();
        let far_beacon = world
            .spawn(beacon_scenario_object(BeaconConfig {
                lock_signature: Some(30.0),
                ..config(None)
            }))
            .id();
        assert_eq!(
            world.get::<LockSignature>(default_beacon).map(|s| **s),
            Some(BEACON_LOCK_SIGNATURE)
        );
        assert_eq!(
            world.get::<LockSignature>(far_beacon).map(|s| **s),
            Some(30.0),
            "an authored signature overrides the default"
        );
    }

    /// The beacon's contract with the rest of the stack: on rails (a nav
    /// point cannot drift) yet lockable via the authored signature, and
    /// carrying the marker + label the HUD chip observer hangs off.
    #[test]
    fn beacon_is_a_static_lockable_labeled_marker() {
        let mut world = World::new();
        world.add_observer(insert_beacon_area);
        let entity = world.spawn(beacon_scenario_object(config(None))).id();
        world.flush();

        assert!(world.get::<BeaconMarker>(entity).is_some());
        assert_eq!(
            world.get::<BeaconLabel>(entity).map(|l| l.0.as_str()),
            Some("BEACON 1")
        );
        assert!(matches!(
            world.get::<RigidBody>(entity),
            Some(RigidBody::Static)
        ));
        assert!(
            world.get::<LockSignature>(entity).is_some(),
            "the authored signature is what keeps a Static beacon lockable"
        );
        // No authored area -> no trigger role.
        assert!(world.get::<ScenarioAreaMarker>(entity).is_none());
        assert!(world.get::<Sensor>(entity).is_none());
    }

    /// With an authored area radius the beacon doubles as its own trigger
    /// volume: area marker + sensor sphere, so OnEnter fires under the
    /// beacon's scenario id.
    #[test]
    fn beacon_with_area_radius_is_its_own_trigger() {
        let mut world = World::new();
        world.add_observer(insert_beacon_area);
        let entity = world.spawn(beacon_scenario_object(config(Some(40.0)))).id();
        world.flush();

        assert!(world.get::<ScenarioAreaMarker>(entity).is_some());
        assert!(world.get::<Sensor>(entity).is_some());
        assert!(world.get::<Collider>(entity).is_some());
    }
}
