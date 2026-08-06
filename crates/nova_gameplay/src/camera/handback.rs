//! The autopilot handback: when a maneuver disengages the mouse rig is
//! re-seeded from the hull instantly (the ship's no-lurch contract), and the
//! camera eases across that discontinuity instead of teleporting.

use avian3d::prelude::Rotation;
use bevy::prelude::*;

use super::rig::{
    SpaceshipCameraController, SpaceshipCameraNormalInputMarker, SpaceshipRotationInputActiveMarker,
};
use crate::prelude::*;

/// How long the camera takes to swing from its autopilot free-look
/// direction onto the re-seeded manual rig after a disengage, seconds.
/// The re-seed itself is instant (the PD's no-lurch contract); only the
/// camera's view of that discontinuity is eased. Playtest knob.
pub(super) const HANDBACK_BLEND_SECONDS: f32 = 0.45;

/// Blends the camera's anchor rotation across the autopilot-to-manual re-seed
/// discontinuity: from the free-look direction the camera held at disengage
/// toward the live rig output, over `HANDBACK_BLEND_SECONDS`. Lives on the
/// camera controller entity; `update_chase_camera_input` ticks and removes it.
#[derive(Component, Debug, Clone, Reflect)]
pub struct CameraHandbackBlend {
    /// The rig output the camera was following when the autopilot let go.
    pub from: Quat,
    /// Seconds since the handback.
    pub elapsed: f32,
}

/// The blended anchor rotation `elapsed` seconds into a handback:
/// smoothstep-eased slerp from the held direction to the live rig. Pure
/// for unit testing.
pub(super) fn handback_anchor_rot(from: Quat, to: Quat, elapsed: f32) -> Quat {
    let t = (elapsed / HANDBACK_BLEND_SECONDS.max(1e-3)).clamp(0.0, 1.0);
    let eased = t * t * (3.0 - 2.0 * t);
    from.slerp(to, eased)
}

/// When an autopilot maneuver disengages, re-seed the normal rotation rig
/// from the ship's *current* attitude. While engaged the mouse kept turning
/// the rig (as camera free-look) but the hull followed the maneuver, so the
/// rig's quat is stale; without this re-seed the PD would violently swing the
/// ship back to wherever the rig last pointed. Re-inserting `PointRotation`
/// resets its internal state, exactly like the free-look mode switches do.
///
/// The re-seed is instant for the SHIP (no-lurch contract) but a visible
/// snap for the CAMERA, whose anchor follows the rig quat - so when the
/// normal rig is the active one, the camera gets a [`CameraHandbackBlend`]
/// seeded with the direction it was actually looking, and eases onto the
/// new rig like a mode switch instead of teleporting (the mode switches
/// are smooth precisely because they re-seed from the CURRENT output;
/// this is the one re-seed that cannot, so the camera bridges it).
pub(super) fn on_autopilot_disengaged(
    remove: On<Remove, Autopilot>,
    mut commands: Commands,
    q_ship: Query<&Rotation, With<PlayerSpaceshipMarker>>,
    q_rig: Query<
        (
            Entity,
            Option<&PointRotationOutput>,
            Has<SpaceshipRotationInputActiveMarker>,
        ),
        With<SpaceshipCameraNormalInputMarker>,
    >,
    q_camera: Query<Entity, With<SpaceshipCameraController>>,
) {
    let Ok(rotation) = q_ship.get(remove.entity) else {
        // Not the player's ship - nothing to re-seed. (A despawning ship
        // still passes this guard - Remove observers see sibling
        // components during the despawn flush - so a dead ship's camera
        // briefly carries a blend; the controller teardown removes it,
        // and a fresh controller clears any stale one defensively.)
        return;
    };

    for (rig, output, active) in &q_rig {
        // Bridge the camera only when this rig is the one it follows (in
        // FreeLook/Turret the normal rig is dormant, so its re-seed only shows
        // on the later switch back to Normal - a pre-existing transition this
        // task does not change). A re-disengage mid-blend restarts from the
        // rig's pre-reseed output, not the mid-blend display value - a small
        // pop in a rare double-handback, accepted for a stateless observer.
        if active {
            if let Some(output) = output {
                for camera in &q_camera {
                    commands.entity(camera).try_insert(CameraHandbackBlend {
                        from: **output,
                        elapsed: 0.0,
                    });
                }
            }
        }
        commands.entity(rig).try_insert(PointRotation {
            initial_rotation: rotation.0,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::camera::{framing::update_chase_camera_input, SpaceshipCameraInputMarker};

    #[test]
    fn handback_anchor_rot_eases_from_held_to_live() {
        let from = Quat::IDENTITY;
        let to = Quat::from_rotation_y(std::f32::consts::FRAC_PI_2);

        // Endpoints: holds the old view at t=0, lands on the live rig at the
        // duration (and stays there past it). Epsilon note: slerp endpoint
        // noise passes through acos in angle_between and reads as ~7e-4 rad
        // even for "equal" quats; 2e-3 is still far below anything visible.
        assert!(handback_anchor_rot(from, to, 0.0).angle_between(from) < 2e-3);
        assert!(handback_anchor_rot(from, to, HANDBACK_BLEND_SECONDS).angle_between(to) < 2e-3);
        assert!(
            handback_anchor_rot(from, to, HANDBACK_BLEND_SECONDS * 2.0).angle_between(to) < 2e-3
        );

        // Monotonic ease: progress toward the target never reverses.
        let mut last = 0.0f32;
        for i in 0..=10 {
            let elapsed = HANDBACK_BLEND_SECONDS * (i as f32 / 10.0);
            let progress =
                to.angle_between(from) - handback_anchor_rot(from, to, elapsed).angle_between(to);
            assert!(
                progress >= last - 1e-4,
                "ease reversed at step {i}: {progress} < {last}"
            );
            last = progress;
        }
    }

    /// The autopilot handback keeps the camera continuous: at the
    /// disengage frame the anchor still points where the camera was
    /// looking (NOT the hull attitude the rig was re-seeded to), and the
    /// blend converges onto the live rig and removes itself.
    #[test]
    fn handback_blends_the_anchor_instead_of_snapping() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(ChaseCameraPlugin);
        app.add_observer(on_autopilot_disengaged);
        app.add_systems(Update, update_chase_camera_input);

        let held = Quat::IDENTITY;
        let hull = Quat::from_rotation_y(std::f32::consts::FRAC_PI_2);

        let ship = app
            .world_mut()
            .spawn((
                SpaceshipRootMarker,
                PlayerSpaceshipMarker,
                Transform::default(),
                Rotation(hull),
                Autopilot::engage(AutopilotAction::Stop),
            ))
            .id();
        let rig = app
            .world_mut()
            .spawn((
                SpaceshipCameraInputMarker,
                SpaceshipCameraNormalInputMarker,
                SpaceshipRotationInputActiveMarker,
                PointRotationOutput::default(),
            ))
            .id();
        let camera = app.world_mut().spawn(SpaceshipCameraController).id();
        app.update();

        // Disengage: the observer re-seeds the rig to the hull attitude
        // (the ship-side no-lurch contract, asserted below) and bridges
        // the camera with a blend from the held view.
        app.world_mut().entity_mut(ship).remove::<Autopilot>();
        let reseeded = app
            .world()
            .get::<PointRotation>(rig)
            .expect("the rig is re-seeded instantly");
        assert_eq!(reseeded.initial_rotation, hull);

        // Simulate the rig snapping its output to the re-seed (the real
        // PointRotation plugin is not in this harness).
        **app.world_mut().get_mut::<PointRotationOutput>(rig).unwrap() = hull;

        // First frame after the handback: the anchor stays on the held
        // view - the whole point - while the rig already reads the hull.
        app.update();
        let anchor = app
            .world()
            .get::<ChaseCameraInput>(camera)
            .unwrap()
            .anchor_rot;
        assert!(
            anchor.angle_between(held) < 0.05,
            "the camera must not snap: anchor is {:?} from the held view",
            anchor.angle_between(held)
        );
        assert!(anchor.angle_between(hull) > 1.0);

        // Force the blend to its end: the anchor lands on the live rig
        // and the blend removes itself.
        app.world_mut()
            .get_mut::<CameraHandbackBlend>(camera)
            .expect("blend inserted on the camera")
            .elapsed = HANDBACK_BLEND_SECONDS;
        app.update();
        let anchor = app
            .world()
            .get::<ChaseCameraInput>(camera)
            .unwrap()
            .anchor_rot;
        assert!(anchor.angle_between(hull) < 2e-3);
        assert!(
            app.world().get::<CameraHandbackBlend>(camera).is_none(),
            "a finished blend cleans itself up"
        );
    }

    /// A dormant normal rig (FreeLook/Turret active) is re-seeded without
    /// bridging the camera: its quat is invisible while another rig
    /// drives (the later switch back to Normal is a separate,
    /// pre-existing transition).
    #[test]
    fn handback_blend_only_bridges_the_active_rig() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_observer(on_autopilot_disengaged);

        let ship = app
            .world_mut()
            .spawn((
                SpaceshipRootMarker,
                PlayerSpaceshipMarker,
                Rotation(Quat::from_rotation_y(1.0)),
                Autopilot::engage(AutopilotAction::Stop),
            ))
            .id();
        // The normal rig exists but is NOT the active one.
        app.world_mut().spawn((
            SpaceshipCameraInputMarker,
            SpaceshipCameraNormalInputMarker,
            PointRotationOutput::default(),
        ));
        let camera = app.world_mut().spawn(SpaceshipCameraController).id();
        app.update();

        app.world_mut().entity_mut(ship).remove::<Autopilot>();
        assert!(
            app.world().get::<CameraHandbackBlend>(camera).is_none(),
            "no bridge for a rig the camera is not following"
        );
    }

    /// Disengaging the autopilot must hand the mouse a rig seeded from the
    /// hull's *current* attitude - otherwise the PD would violently swing the
    /// ship back to the rig's stale pre-maneuver command.
    #[test]
    fn disengaging_autopilot_reseeds_the_normal_rig_from_the_hull() {
        let mut app = App::new();
        app.add_observer(on_autopilot_disengaged);

        let rig = app
            .world_mut()
            .spawn((SpaceshipCameraNormalInputMarker, PointRotation::default()))
            .id();
        let attitude = Quat::from_rotation_y(1.2);
        let ship = app
            .world_mut()
            .spawn((
                SpaceshipRootMarker,
                PlayerSpaceshipMarker,
                Rotation(attitude),
                Autopilot::engage(AutopilotAction::Stop),
            ))
            .id();
        app.update();

        app.world_mut().entity_mut(ship).remove::<Autopilot>();
        app.update();

        let seeded = app.world().get::<PointRotation>(rig).unwrap();
        assert_eq!(
            seeded.initial_rotation, attitude,
            "the rig must be re-seeded from the hull attitude on disengage"
        );
    }
}
