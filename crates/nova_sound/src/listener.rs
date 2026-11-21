//! Where the game is heard from.
//!
//! One listener, split in two. The POSITION is the player's ship, marked with
//! [`SoundListenerAnchor`]; with no ship it is the camera. The ORIENTATION is
//! always the camera, marked with [`SfxListenerMarker`], so a sound to the
//! left of the screen is heard in the left ear whichever way the ship points.
//! Every [`SoundBus::Effect`](super::SoundBus::Effect) sound fades by its
//! distance from the position and pans by its bearing in the camera's frame,
//! the player's own ship included.

use bevy::prelude::*;

#[cfg(doc)]
use super::mixing::SfxListenerMarker;
use super::spatial::local_bearing;

/// Marks the entity the listener stands on: the player's ship.
///
/// `nova_gameplay` keeps it on whichever ship carries the player marker. With
/// no anchor the listener stands at the camera.
#[derive(Component, Clone, Copy, Debug, Default, Reflect)]
#[reflect(Component)]
pub struct SoundListenerAnchor;

/// The listener this frame: the camera it faces with and the point it hears
/// from.
#[derive(Clone, Copy, Debug)]
pub struct Hearing {
    /// The camera's pose: bearings are measured in its frame.
    pub camera: GlobalTransform,
    /// The world point distance is measured from.
    pub at: Vec3,
}

impl Hearing {
    /// The listener for a camera at `camera`, standing at `anchor` when there is
    /// one and at the camera when there is not.
    pub fn new(camera: GlobalTransform, anchor: Option<Vec3>) -> Self {
        Self {
            at: anchor.unwrap_or_else(|| camera.translation()),
            camera,
        }
    }

    /// The distance from the listener to `source`, in world units.
    pub fn distance(&self, source: Vec3) -> f32 {
        self.at.distance(source)
    }

    /// The direction to `source` in the camera's local frame, measured from
    /// the listener position.
    pub fn bearing(&self, source: Vec3) -> Vec3 {
        let frame = GlobalTransform::from(Transform {
            translation: self.at,
            rotation: self.camera.rotation(),
            scale: Vec3::ONE,
        });
        local_bearing(&frame, source)
    }
}

/// The listener from the camera and anchor poses found this frame. `None`
/// with no marked camera: there is no frame to pan in.
pub fn resolve_hearing(
    camera: Option<&GlobalTransform>,
    anchor: Option<&GlobalTransform>,
) -> Option<Hearing> {
    camera.map(|camera| Hearing::new(*camera, anchor.map(GlobalTransform::translation)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_listener_stands_on_the_anchor_and_faces_with_the_camera() {
        let camera = GlobalTransform::from(
            Transform::from_xyz(0.0, 0.0, 100.0).looking_at(Vec3::ZERO, Vec3::Y),
        );
        let hearing = Hearing::new(camera, Some(Vec3::ZERO));
        assert_eq!(
            hearing.distance(Vec3::X * 10.0),
            10.0,
            "measured from the ship"
        );
        let bearing = hearing.bearing(Vec3::X * 10.0);
        assert!(
            bearing.x > 0.99,
            "a sound to the ship's screen-right is heard on the right: {bearing}"
        );
    }

    #[test]
    fn without_an_anchor_the_listener_stands_at_the_camera() {
        let camera = GlobalTransform::from_translation(Vec3::new(5.0, 0.0, 0.0));
        let hearing = Hearing::new(camera, None);
        assert_eq!(hearing.at, Vec3::new(5.0, 0.0, 0.0));
    }
}
