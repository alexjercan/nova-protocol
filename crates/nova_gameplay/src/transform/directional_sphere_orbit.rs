//! Directional Sphere Orbit Component and Systems
//!
//! This module provides a component and associated systems to enable entities to orbit
//! around a point on the surface of a sphere based on a directional input. We pass in the input
//! direction vector, and the system will compute the corresponding position on the sphere's
//! surface, that the vector intersects.
//!
//! The orbiting entity can smoothly transition to new directions based on the input.

use bevy::prelude::*;

use crate::math::prelude::*;

/// Glob-import surface for the directional sphere-orbit rig.
pub mod prelude {
    pub use super::{
        DirectionalSphereOrbit, DirectionalSphereOrbitInput, DirectionalSphereOrbitOutput,
        DirectionalSphereOrbitPlugin,
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
            (sphere_update_state, sphere_update_output).chain(),
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
        // `theta` comes back from `direction_to_spherical` folded into
        // `[-PI, PI)`, so a target that crosses the seam reads as a near-TAU
        // jump. Ease towards the UNWRAPPED target instead, or the orbit takes
        // the long way round the sphere every time the direction crosses -Z.
        let unwrapped_theta = state.theta + normalize_angle(new_theta - state.theta);
        let new_theta = state.theta.lerp_and_snap(unwrapped_theta, smoothing, dt);
        let new_phi = state.phi.lerp_and_snap(new_phi, smoothing, dt);

        state.theta = normalize_angle(new_theta);
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

#[cfg(test)]
mod tests {
    use std::{f32::consts::PI, time::Duration};

    use bevy::time::{TimePlugin, TimeUpdateStrategy};

    use super::*;

    /// A direction on the equator at horizontal angle `theta`.
    fn equatorial(theta: f32) -> Vec3 {
        spherical_to_cartesian(1.0, theta, 0.0)
    }

    fn theta_of(app: &App, entity: Entity) -> f32 {
        app.world()
            .entity(entity)
            .get::<DirectionalSphereOrbitState>()
            .expect("the insert observer must seed the orbit state")
            .theta
    }

    /// Easing across the -Z seam takes the SHORT way round. `theta` comes back
    /// from `direction_to_spherical` folded into `[-PI, PI)`, so a step of 0.1
    /// rad that crosses -Z reads as a near-TAU jump; eased towards the folded
    /// target the orbit sweeps the long way back through 0 instead.
    #[test]
    fn easing_across_the_seam_takes_the_short_way() {
        let start = PI - 0.05;
        let target = -PI + 0.05;

        let mut app = App::new();
        app.add_plugins((TimePlugin, DirectionalSphereOrbitPlugin));
        app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
            1.0 / 60.0,
        )));

        let entity = app
            .world_mut()
            .spawn(DirectionalSphereOrbit {
                radius: 1.0,
                center: Vec3::ZERO,
                direction: equatorial(start),
                smoothing: 0.5,
            })
            .id();
        // The observer seeds the state; the first update lands a zero delta.
        app.update();
        assert!(
            (theta_of(&app, entity) - start).abs() < 1e-4,
            "the rig must start at the seam, not somewhere else"
        );

        app.world_mut()
            .entity_mut(entity)
            .insert(DirectionalSphereOrbitInput(equatorial(target)));
        app.update();

        // Folded back into [-PI, PI), a short-way step from PI - 0.05 either
        // stays just under PI or has just crossed to just above -PI. Measuring
        // the SIGNED short-way delta covers both without a seam special case.
        let moved = normalize_angle(theta_of(&app, entity) - start);
        assert!(
            moved > 0.0,
            "the orbit must ease towards the seam, not away from it (moved {moved})"
        );
        assert!(
            moved <= 0.1 + 1e-4,
            "the orbit must not overshoot the 0.1 rad target (moved {moved})"
        );
    }
}
