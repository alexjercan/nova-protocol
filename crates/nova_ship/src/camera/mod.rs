//! The player's chase camera and where it points the ship. Derives the camera
//! mode ([`SpaceshipCameraControlMode`]: normal / free-look / turret) and the
//! weapons-raised stance ([`WeaponsRaised`]) each frame from held inputs, and
//! exposes the live look ray ([`ActiveLookRay`]) that targeting and the radar
//! read to know where the player is aiming right now.
//!
//! Touch this module for camera framing and look-input routing. Gameplay
//! consumers should read [`WeaponsRaised`] / [`ActiveLookRay`], never the raw
//! camera enum.
//!
//! Nova owns the rigs the controller is built on, too: [`chase`], [`skybox`],
//! [`post`], [`wasd`] and [`wasd_controller`]. The trauma shake rig they
//! compose with is [`nova_gameplay::shake`] - it is fed by combat juice, not by
//! the camera, so it sits outside this module. The private `authority` submodule
//! orders it against these, and holding the rigs here keeps that ordering
//! contract module-local rather than a cross-crate promise.

use bevy::prelude::*;
use bevy_enhanced_input::prelude::*;
use nova_input::prelude::*;

use crate::input::bindings::camera_bindings;

mod authority;
pub mod chase;
mod framing;
mod handback;
mod mode;
pub mod post;
mod rig;
pub mod skybox;
pub mod wasd;
pub mod wasd_controller;

pub use self::{
    authority::{CameraAuthorityPlugin, CameraAuthoritySystems},
    handback::CameraHandbackBlend,
    mode::{SpaceshipCameraControlMode, WeaponsRaised},
    rig::{
        ActiveLookRay, SpaceshipCameraController, SpaceshipCameraFreeLookInputMarker,
        SpaceshipCameraInputMarker, SpaceshipCameraNormalInputMarker,
        SpaceshipCameraTurretInputMarker, SpaceshipRotationInputActiveMarker,
    },
};
use self::{
    framing::{update_camera_rig, update_chase_camera_input},
    handback::on_autopilot_disengaged,
    mode::{
        derive_control_mode_and_raised, on_rotation_input, on_rotation_input_completed,
        sync_spaceship_control_mode,
    },
    rig::{
        destroy_camera_controller, insert_camera_controller, insert_camera_freelook,
        insert_camera_turret, insert_player_input, rebuild_player_input_on_rebind,
        PlayerInputMarker,
    },
};

/// Glob-import surface: `use nova_gameplay::camera::prelude::*` re-exports the
/// public API of this module and of the six rigs it is built on.
pub mod prelude {
    pub use super::{
        chase::prelude::*, post::prelude::*, skybox::prelude::*, wasd::prelude::*,
        wasd_controller::prelude::*, ActiveLookRay, CameraAuthorityPlugin, CameraAuthoritySystems,
        NovaCameraSystems, SpaceshipCameraControlMode, SpaceshipCameraController,
        SpaceshipCameraControllerPlugin, SpaceshipCameraFreeLookInputMarker,
        SpaceshipCameraInputMarker, SpaceshipCameraNormalInputMarker,
        SpaceshipCameraTurretInputMarker, SpaceshipRotationInputActiveMarker, WeaponsRaised,
    };
}

/// System set holding the camera-rig sync and look-input systems, ordered after
/// the hud set by [`SpaceshipSystems`](nova_gameplay::plugin::SpaceshipSystems).
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct NovaCameraSystems;

/// Wires the player chase camera, its mode/stance derivation and the look ray.
/// Added by [`NovaGameplayPlugin`](nova_gameplay::plugin::NovaGameplayPlugin).
pub struct SpaceshipCameraControllerPlugin;

impl Plugin for SpaceshipCameraControllerPlugin {
    fn build(&self, app: &mut App) {
        trace!("SpaceshipCameraControllerPlugin: build");

        app.register_input_actions(camera_bindings());

        app.init_resource::<SpaceshipCameraControlMode>();
        app.add_input_context::<PlayerInputMarker>();

        app.add_observer(insert_camera_controller);
        app.add_observer(insert_camera_freelook);
        app.add_observer(insert_camera_turret);
        app.add_observer(insert_player_input);
        app.add_systems(
            Update,
            rebuild_player_input_on_rebind.run_if(resource_changed::<InputBindings>),
        );
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
                derive_control_mode_and_raised
                    .run_if(in_state(nova_gameplay::PauseStates::Unpaused)),
                update_chase_camera_input,
                sync_spaceship_control_mode,
                update_camera_rig,
            )
                .chain()
                .in_set(NovaCameraSystems),
        );

        // Every camera-Transform writer in the app - nova's three rigs and
        // nova's scripted pose - is ordered by this one chain. Guarded because
        // nova_scenario adds it too when it is the only camera consumer, and
        // plugin add order between the two crates is the app's business.
        if !app.is_plugin_added::<CameraAuthorityPlugin>() {
            app.add_plugins(CameraAuthorityPlugin);
        }
    }
}
