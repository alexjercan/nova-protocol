//! The player's chase camera and where it points the ship. Derives the camera
//! mode ([`SpaceshipCameraControlMode`]: normal / free-look / turret) and the
//! weapons-raised stance ([`WeaponsRaised`]) each frame from held inputs, and
//! exposes the live look ray ([`ActiveLookRay`]) that targeting and the radar
//! read to know where the player is aiming right now.
//!
//! Touch this module for camera framing and look-input routing. Gameplay
//! consumers should read [`WeaponsRaised`] / [`ActiveLookRay`], never the raw
//! camera enum. Built on `bevy_common_systems`' `ChaseCamera` /
//! `PointRotation` rigs.

use bevy::{prelude::*, transform::TransformSystems};
use bevy_common_systems::prelude::*;
use bevy_enhanced_input::prelude::*;

mod framing;
mod handback;
mod mode;
mod rig;

use self::{
    framing::{update_camera_rig, update_chase_camera_input},
    handback::on_autopilot_disengaged,
    mode::{
        derive_control_mode_and_raised, on_rotation_input, on_rotation_input_completed,
        sync_spaceship_control_mode,
    },
    rig::{
        destroy_camera_controller, insert_camera_controller, insert_camera_freelook,
        insert_camera_turret, insert_player_input, PlayerInputMarker,
    },
};
pub use self::{
    handback::CameraHandbackBlend,
    mode::{SpaceshipCameraControlMode, WeaponsRaised},
    rig::{
        ActiveLookRay, SpaceshipCameraController, SpaceshipCameraFreeLookInputMarker,
        SpaceshipCameraInputMarker, SpaceshipCameraNormalInputMarker,
        SpaceshipCameraTurretInputMarker, SpaceshipRotationInputActiveMarker,
    },
};

/// Glob-import surface: `use nova_gameplay::camera_controller::prelude::*`
/// re-exports the public API of this module.
pub mod prelude {
    pub use super::{
        ActiveLookRay, NovaCameraSystems, SpaceshipCameraControlMode, SpaceshipCameraController,
        SpaceshipCameraControllerPlugin, SpaceshipCameraFreeLookInputMarker,
        SpaceshipCameraInputMarker, SpaceshipCameraNormalInputMarker,
        SpaceshipCameraTurretInputMarker, SpaceshipRotationInputActiveMarker, WeaponsRaised,
    };
}

/// System set holding the camera-rig sync and look-input systems, ordered after
/// the hud set by [`SpaceshipSystems`].
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct NovaCameraSystems;

/// Wires the player chase camera, its mode/stance derivation and the look ray.
/// Added by [`NovaGameplayPlugin`].
pub struct SpaceshipCameraControllerPlugin;

impl Plugin for SpaceshipCameraControllerPlugin {
    fn build(&self, app: &mut App) {
        debug!("SpaceshipCameraControllerPlugin: build");

        app.init_resource::<SpaceshipCameraControlMode>();
        app.add_input_context::<PlayerInputMarker>();

        app.add_observer(insert_camera_controller);
        app.add_observer(insert_camera_freelook);
        app.add_observer(insert_camera_turret);
        app.add_observer(insert_player_input);
        app.add_observer(destroy_camera_controller);

        app.add_observer(on_autopilot_disengaged);

        app.add_observer(on_rotation_input);
        app.add_observer(on_rotation_input_completed);

        app.register_type::<WeaponsRaised>();

        app.add_systems(
            Update,
            // Fully chained: the mode (and raised flag) is derived from the
            // held inputs first, then the rig system owns every ChaseCamera
            // field and must run after the mode switch (whose markers decide
            // the rig) AND after the input write, because its velocity lead
            // is expressed in this frame's anchor rotation frame.
            (
                // Only derive the combat/free-look mode while gameplay is live:
                // frozen behind the NOVA OS (or pause menu), RMB must NOT flip
                // the ship into Turret/combat stance - it belongs to whatever
                // app owns the screen (e.g. the map's orbit drag). Freezing the
                // derivation holds the last mode; it re-reads held inputs on
                // unpause.
                derive_control_mode_and_raised.run_if(in_state(crate::PauseStates::Unpaused)),
                update_chase_camera_input,
                sync_spaceship_control_mode,
                update_camera_rig,
            )
                .chain()
                .in_set(NovaCameraSystems),
        );

        // bcs moves the camera Transform in PostUpdate but leaves its order
        // against Bevy's transform propagation AMBIGUOUS - if propagation wins
        // the race, the frame renders with LAST frame's camera pose (a
        // per-build coin flip). Pin it from nova via the exported set so the
        // rendered camera is always this frame's.
        app.configure_sets(
            PostUpdate,
            ChaseCameraSystems::Sync.before(TransformSystems::Propagate),
        );
    }
}
