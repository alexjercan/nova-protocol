//! Directional Sphere Orbit Component and Systems
//!
//! This module provides a component and associated systems to enable entities to orbit
//! around a point on the surface of a sphere based on a directional input. We pass in the input
//! direction vector, and the system will compute the corresponding position on the sphere's
//! surface, that the vector intersects.
//!
//! The orbiting entity can smoothly transition to new directions based on the input.

use bevy::prelude::*;

use crate::math::{direction_to_spherical, spherical_to_cartesian, LerpSnap};

/// Glob-import surface for the directional sphere-orbit rig.
pub mod prelude {
    pub use super::{
        DirectionalSphereOrbit, DirectionalSphereOrbitInput, DirectionalSphereOrbitOutput,
        DirectionalSphereOrbitPlugin, DirectionalSphereOrbitSystems,
    };
}

/// Component to define a spherical orbit around a center point.
#[derive(Component, Clone, Debug, Default, Reflect)]
pub struct DirectionalSphereOrbit {
    /// Radius of the sphere (distance from origin or from a center)
    pub radius: f32,
    /// (Optional) center of the sphere (in world space)
    pub center: Vec3,
    /// Initial pointing direction
    pub direction: Vec3,
    /// Smoothing factor (between 0 and 1) for the orbit movement
    /// 0 = no smoothing, 1 = full smoothing
    pub smoothing: f32,
}

/// The output position of the orbiting entity on the sphere surface.
#[derive(Component, Clone, Debug, Default, Deref, DerefMut, Reflect)]
pub struct DirectionalSphereOrbitOutput(pub Vec3);

/// The input direction for the orbiting entity on the sphere surface.
#[derive(Component, Default, Clone, Copy, Debug, Deref, DerefMut, Reflect)]
pub struct DirectionalSphereOrbitInput(pub Vec3);

#[derive(Component, Clone, Debug, Default, Reflect)]
struct DirectionalSphereOrbitState {
    theta: f32,
    phi: f32,
}

/// The system set used by the directional sphere orbit plugin.
///
/// Only contains the orbit sync systems.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum DirectionalSphereOrbitSystems {
    /// Advances the orbit and writes the resulting pose.
    Sync,
}

/// Plugin to manage entities with `DirectionalSphereOrbit` component.
///
/// DirectionalSphereOrbit allows an entity to orbit around a point on the surface of a sphere.
pub struct DirectionalSphereOrbitPlugin;

impl Plugin for DirectionalSphereOrbitPlugin {
    fn build(&self, app: &mut App) {
        debug!("DirectionalSphereOrbitPlugin: build");

        app.add_observer(initialize_sphere_orbit_system);

        app.add_systems(
            PostUpdate,
            (sphere_update_state, sphere_update_output)
                .chain()
                .in_set(DirectionalSphereOrbitSystems::Sync),
        );
    }
}

/// Initialize orbit state and next target angles
fn initialize_sphere_orbit_system(
    insert: On<Insert, DirectionalSphereOrbit>,
    mut commands: Commands,
    q_orbit: Query<&DirectionalSphereOrbit>,
) {
    let entity = insert.entity;
    trace!("initialize_sphere_orbit_system: entity {:?}", entity);

    let Ok(orbit) = q_orbit.get(entity) else {
        error!(
            "initialize_sphere_orbit_system: entity {:?} not found in q_orbit",
            entity
        );
        return;
    };

    let (theta, phi) = direction_to_spherical(orbit.direction);

    commands.entity(entity).insert((
        DirectionalSphereOrbitState { theta, phi },
        DirectionalSphereOrbitInput(orbit.direction),
        DirectionalSphereOrbitOutput(
            spherical_to_cartesian(orbit.radius, theta, phi) + orbit.center,
        ),
    ));
}

fn sphere_update_state(
    time: Res<Time>,
    mut query: Query<(
        &DirectionalSphereOrbit,
        &mut DirectionalSphereOrbitState,
        &DirectionalSphereOrbitInput,
    )>,
) {
    let dt = time.delta_secs();

    for (orbit, mut state, next) in query.iter_mut() {
        let (new_theta, new_phi) = direction_to_spherical(**next);

        let smoothing = orbit.smoothing.clamp(0.0, 1.0);
        let new_theta = state.theta.lerp_and_snap(new_theta, smoothing, dt);
        let new_phi = state.phi.lerp_and_snap(new_phi, smoothing, dt);

        state.theta = new_theta;
        state.phi = new_phi;
    }
}

fn sphere_update_output(
    mut query: Query<(
        &DirectionalSphereOrbit,
        &DirectionalSphereOrbitState,
        &mut DirectionalSphereOrbitOutput,
    )>,
) {
    for (orbit, state, mut output) in query.iter_mut() {
        let pos = spherical_to_cartesian(orbit.radius, state.theta, state.phi) + orbit.center;
        **output = pos;
    }
}
