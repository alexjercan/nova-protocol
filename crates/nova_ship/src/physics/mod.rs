//! Rigid-body control: [`PDControllerPlugin`](prelude::PDControllerPlugin) drives a body's rotation toward a
//! target orientation with a critically-damped PD law, and
//! [`rigid_body_point_velocity`](prelude::rigid_body_point_velocity) supplies the point-velocity relation a muzzle
//! needs.
//!
//! Nova owns these because the PD controller is the ship's attitude authority -
//! its frequency and damping ratio are flight-feel decisions, not engine ones -
//! and the point-velocity relation is what gives every torpedo and turret shot
//! the swing of the hull it left. Nova's radial gravity lives in
//! [`nova_gameplay::gravity`], not here.

mod pd_controller;
mod rigid_body;

/// The PD controller types and plugin, and the rigid-body point-velocity helper.
pub mod prelude {
    pub use super::{
        pd_controller::{
            PDController, PDControllerInput, PDControllerOutput, PDControllerPlugin,
            PDControllerSystems, PDControllerTarget,
        },
        rigid_body::rigid_body_point_velocity,
    };
}
