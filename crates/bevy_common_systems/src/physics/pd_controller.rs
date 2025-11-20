//! PD Controller for 3D rotations in Bevy using Avian3D

use avian3d::prelude::*;
use bevy::prelude::*;

pub mod prelude {
    pub use super::{
        PDController, PDControllerInput, PDControllerOutput, PDControllerPlugin,
        PDControllerSystems, PDControllerTarget,
    };
}

/// Component that defines a PD controller for rotational control.
#[derive(Component, Clone, Copy, Debug, Reflect)]
#[require(PDControllerInput, PDControllerOutput)]
pub struct PDController {
    /// The frequency of the PD controller in Hz.
    pub frequency: f32,
    /// The damping ratio of the PD controller.
    pub damping_ratio: f32,
    /// The maximum torque that can be applied by the PD controller.
    pub max_torque: f32,
}

/// Input rotation for the PD controller.
#[derive(Component, Debug, Clone, Default, Deref, DerefMut, Reflect)]
pub struct PDControllerInput(pub Quat);

/// Target entity for the PD controller to follow.
#[derive(Component, Debug, Clone, Deref, DerefMut, Reflect)]
pub struct PDControllerTarget(pub Entity);

/// Output torque from the PD controller.
#[derive(Component, Debug, Clone, Default, Deref, DerefMut, Reflect)]
pub struct PDControllerOutput(pub Vec3);

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum PDControllerSystems {
    Sync,
}

pub struct PDControllerPlugin;

impl Plugin for PDControllerPlugin {
    fn build(&self, app: &mut App) {
        debug!("PDControllerPlugin: build");

        app.add_observer(setup_pd_controller_system);

        app.add_systems(
            FixedUpdate,
            update_controller_root_torque.in_set(PDControllerSystems::Sync),
        );
    }
}

fn setup_pd_controller_system(add: On<Add, PDController>, mut commands: Commands) {
    let entity = add.entity;
    trace!("setup_pd_controller_system: entity {:?}", entity);

    commands
        .entity(entity)
        .insert(PDControllerInput::default())
        .insert(PDControllerOutput::default());
}

fn update_controller_root_torque(
    q_root: Query<(&ComputedAngularInertia, &Rotation, &AngularVelocity)>,
    mut q_controller: Query<(
        &PDController,
        &PDControllerInput,
        &PDControllerTarget,
        &mut PDControllerOutput,
    )>,
) {
    for (controller, controller_input, controller_target, mut controller_output) in
        &mut q_controller
    {
        let Ok((angular_inertia, rotation, angular_velocity)) = q_root.get(**controller_target)
        else {
            error!(
                "update_controller_root_torque: root entity {:?} not found in q_root",
                **controller_target
            );
            continue;
        };

        let (principal, local_frame) = angular_inertia.principal_angular_inertia_with_local_frame();

        let torque = compute_pd_torque(
            controller.frequency,
            controller.damping_ratio,
            controller.max_torque,
            **rotation,
            **controller_input,
            **angular_velocity,
            principal,
            local_frame,
        );

        **controller_output = torque;
    }
}

fn compute_pd_torque(
    frequency: f32,
    damping_ratio: f32,
    max_torque: f32,
    from_rotation: Quat,
    to_rotation: Quat,
    angular_velocity: Vec3,
    inertia_principal: Vec3,
    inertia_local_frame: Quat,
) -> Vec3 {
    // PD gains
    let kp = (6.0 * frequency).powi(2) * 0.25;
    let kd = 4.5 * frequency * damping_ratio;

    let mut delta = to_rotation * from_rotation.conjugate();
    if delta.w < 0.0 {
        delta = Quat::from_xyzw(-delta.x, -delta.y, -delta.z, -delta.w);
    }

    let (mut axis, mut angle) = delta.to_axis_angle();
    axis = axis.normalize_or_zero();
    if angle > std::f32::consts::PI {
        angle -= 2.0 * std::f32::consts::PI;
    }

    // Normalize axis (avoid NaNs if angle is zero)
    axis = axis.normalize_or_zero();

    // PD control (raw torque)
    let raw = axis * (kp * angle) - angular_velocity * kd;

    let rot_inertia_to_world = inertia_local_frame * from_rotation;
    let torque_local = rot_inertia_to_world.inverse() * raw;
    let torque_scaled = torque_local * inertia_principal;
    let final_torque = rot_inertia_to_world * torque_scaled;

    // Optionally clamp final torque magnitude
    if final_torque.length_squared() > max_torque * max_torque {
        final_torque.normalize() * max_torque
    } else {
        final_torque
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_pd_torque_zero_error() {
        let torque = compute_pd_torque(
            1.0,
            1.0,
            10.0,
            Quat::IDENTITY,
            Quat::IDENTITY,
            Vec3::ZERO,
            Vec3::ONE,
            Quat::IDENTITY,
        );
        assert!(torque.abs_diff_eq(Vec3::ZERO, 1e-6));
    }

    #[test]
    fn test_compute_pd_torque_small_angle() {
        let torque = compute_pd_torque(
            1.0,
            1.0,
            10.0,
            Quat::IDENTITY,
            Quat::from_axis_angle(Vec3::Y, 0.1),
            Vec3::ZERO,
            Vec3::ONE,
            Quat::IDENTITY,
        );
        assert!(torque.length() > 0.0);
    }

    #[test]
    fn test_compute_pd_torque_large_angle() {
        let torque = compute_pd_torque(
            1.0,
            1.0,
            10.0,
            Quat::IDENTITY,
            Quat::from_axis_angle(Vec3::Y, std::f32::consts::PI),
            Vec3::ZERO,
            Vec3::ONE,
            Quat::IDENTITY,
        );
        assert!(torque.length() > 0.0);
    }

    #[test]
    fn test_compute_pd_torque_with_angular_velocity() {
        let torque = compute_pd_torque(
            1.0,
            1.0,
            10.0,
            Quat::IDENTITY,
            Quat::from_axis_angle(Vec3::Y, 0.5),
            Vec3::new(0.0, 2.0, 0.0),
            Vec3::ONE,
            Quat::IDENTITY,
        );
        assert!(torque.length() > 0.0);
    }
}
