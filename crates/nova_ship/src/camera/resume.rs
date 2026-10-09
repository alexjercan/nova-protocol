//! The player's view as a saved world keeps it, and how a Load shows it again.
//!
//! The save takes the pose the base chase solve wrote on the last displayed
//! frame, relative to the ship as that frame showed it. Shake and a scripted
//! pose come after the solve, so neither is saved. The Load shows that pose on
//! the first frame that renders the player camera, then
//! [`CameraResumeBlend`] eases it onto the live chase pose.

use avian3d::prelude::Rotation;
use bevy::prelude::*;
use nova_gameplay::prelude::*;

use super::{
    chase::ChaseCameraState,
    rig::{SpaceshipCameraController, SpaceshipCameraNormalInputMarker},
    zoom::{ChaseZoom, CHASE_ZOOM_MAX},
};
use crate::prelude::Autopilot;

#[cfg(test)]
mod tests;

/// The player's camera as a saved world keeps it, relative to the ship.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(deny_unknown_fields)
)]
pub struct CameraView {
    /// The rendered camera position, in the ship's frame, engine units.
    pub position_from_ship: Vec3,
    /// The rendered camera rotation, in the ship's frame.
    pub rotation_from_ship: Quat,
    /// The look the ship steers by, in the ship's frame. Identity while an
    /// autopilot maneuver flies the ship.
    pub steer_from_ship: Quat,
    /// The session zoom level, 1 to `CHASE_ZOOM_MAX`.
    pub zoom: f32,
}

impl CameraView {
    /// The player camera as the last displayed frame showed it, without
    /// shake or a scripted pose, or `None` while the controller, its Normal
    /// rig or a solved pose does not exist yet.
    ///
    /// Run it in `Update`: the solved pose and the ship's `GlobalTransform`
    /// are then both the last displayed frame's.
    pub fn capture(world: &mut World) -> Option<Self> {
        let solved = world
            .query_filtered::<&ChaseCameraState, With<SpaceshipCameraController>>()
            .single(world)
            .ok()?
            .solved?;
        let steer = world
            .query_filtered::<&PointRotationOutput, With<SpaceshipCameraNormalInputMarker>>()
            .single(world)
            .ok()?
            .0;
        let (ship, rotation, autopilot) = world
            .query_filtered::<(&GlobalTransform, &Rotation, Has<Autopilot>), (
                With<SpaceshipRootMarker>,
                With<PlayerSpaceshipMarker>,
            )>()
            .single(world)
            .ok()?;
        let ship = ship.compute_transform();
        let to_ship = ship.rotation.inverse();
        Some(Self {
            position_from_ship: to_ship * (solved.translation - ship.translation),
            rotation_from_ship: to_ship * solved.rotation,
            // A Load is a disengage: the hull is the heading the maneuver
            // left it on.
            steer_from_ship: if autopilot {
                Quat::IDENTITY
            } else {
                rotation.0.inverse() * steer
            },
            zoom: world.resource::<ChaseZoom>().manual,
        })
    }

    /// # Errors
    ///
    /// A non-finite position, a rotation or steer that is not a finite unit
    /// quaternion, or a zoom that is not finite or is outside 1 to
    /// `CHASE_ZOOM_MAX`.
    pub fn validate(&self) -> Result<(), CameraViewFault> {
        if !self.position_from_ship.is_finite() {
            return Err(CameraViewFault::NonFinitePosition(self.position_from_ship));
        }
        for rotation in [self.rotation_from_ship, self.steer_from_ship] {
            if !rotation.is_finite() || !rotation.is_normalized() {
                return Err(CameraViewFault::NonUnitRotation(rotation));
            }
        }
        if !(1.0..=CHASE_ZOOM_MAX).contains(&self.zoom) {
            return Err(CameraViewFault::ZoomOutOfRange(self.zoom));
        }
        Ok(())
    }
}

/// Why a saved view cannot open.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CameraViewFault {
    /// The position is NaN or infinite.
    NonFinitePosition(Vec3),
    /// A rotation is not a finite unit quaternion.
    NonUnitRotation(Quat),
    /// The zoom is not finite or is outside 1 to `CHASE_ZOOM_MAX`.
    ZoomOutOfRange(f32),
}

impl std::fmt::Display for CameraViewFault {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NonFinitePosition(position) => {
                write!(f, "the position {position} is not finite")
            }
            Self::NonUnitRotation(rotation) => {
                write!(f, "the rotation {rotation} is not a unit quaternion")
            }
            Self::ZoomOutOfRange(zoom) => {
                write!(f, "the zoom {zoom} is outside 1 to {CHASE_ZOOM_MAX}")
            }
        }
    }
}

/// The view the next player camera opens at. The first
/// `insert_camera_controller` takes it; a later respawn opens the default.
#[derive(Resource, Debug, Clone, Copy)]
pub struct ResumedCameraView {
    /// The saved view; its pose stays ship-relative until the blend shows it.
    pub view: CameraView,
    /// STEER in world space, composed from the saved physics rotation.
    pub steer: Quat,
}

/// Eases the camera from a resumed ship-relative pose onto the solved chase
/// pose. On the camera controller; removed when it ends.
#[derive(Component, Debug, Clone, Copy)]
pub struct CameraResumeBlend {
    /// The shown camera position, in the ship's frame.
    pub position_from_ship: Vec3,
    /// The shown camera rotation, in the ship's frame.
    pub rotation_from_ship: Quat,
    /// Seconds since the first frame that showed it.
    pub elapsed: f32,
}
