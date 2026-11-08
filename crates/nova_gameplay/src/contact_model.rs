//! The avian contact model that nova's ram damage, blasts and pickups are tuned
//! against.

use avian3d::prelude::*;
use bevy::prelude::*;

/// The contact model plugin.
pub mod prelude {
    pub use super::ContactModelPlugin;
}

/// Sets avian's collision model for every rigid body. Add it after
/// `PhysicsPlugins`, because it replaces their [`NarrowPhaseConfig`].
///
/// - Every [`RigidBody`] requires [`SpeculativeCcd::MAX`]. A contact then
///   starts while the shapes are still apart, so the solver stops a ram at its
///   approach speed. Without it, the first contact is an overlap, and the
///   push-out impulse adds to the ram damage. The cargo intake also refuses a
///   new canister on a separated contact before the shapes touch.
/// - Every [`RigidBody`] requires [`SweptCcd`] with [`CcdFilter::NONE`].
///   Speculative contacts already stop tunnelling. A swept body moves back to
///   its time of impact, which changes when a ram lands.
/// - Contact recycling is off. A recycled contact keeps the `normal_speed` of
///   the step that computed it (avian3d 0.8 `narrow_phase/system_param.rs`,
///   the `'recycle` block). A pair that the solver pushes apart then still
///   reads as closing at the ram speed, and `deal_contact_impact_damage`
///   charges each push-out impulse as a new ram.
pub struct ContactModelPlugin;

impl Plugin for ContactModelPlugin {
    fn build(&self, app: &mut App) {
        app.register_required_components_with::<RigidBody, SpeculativeCcd>(|| SpeculativeCcd::MAX);
        app.register_required_components_with::<RigidBody, SweptCcd>(|| SweptCcd {
            filter: CcdFilter::NONE,
            ..default()
        });
        app.insert_resource(NarrowPhaseConfig {
            recycle_distance: 0.0,
            ..default()
        });
    }
}
