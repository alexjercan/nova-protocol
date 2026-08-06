//! Rigid-body control: [`pd_controller`] drives a body's rotation toward a
//! target orientation with a critically-damped PD law, and [`rigid_body`]
//! supplies the point-velocity relation a muzzle needs.
//!
//! Nova owns these because the PD controller is the ship's attitude authority -
//! its frequency and damping ratio are flight-feel decisions, not engine ones -
//! and the point-velocity relation is what gives every torpedo and turret shot
//! the swing of the hull it left. Nova's radial gravity lives in
//! [`crate::gravity`], not here.

pub mod pd_controller;
pub mod rigid_body;

/// Glob-import surface: `use nova_gameplay::physics::prelude::*` re-exports the
/// public API of this module.
pub mod prelude {
    pub use super::{
        pd_controller::{
            PDController, PDControllerInput, PDControllerOutput, PDControllerPlugin,
            PDControllerSystems, PDControllerTarget,
        },
        rigid_body::rigid_body_point_velocity,
    };
}
