//! Scripted railgun shots: an authored order on one named gun that commits a
//! shot with no controller involved.
//!
//! The bay's [`ScriptedTorpedoOrder`](crate::prelude::ScriptedTorpedoOrder)
//! pattern, for the gun the HULL aims. The scenario layer inserts a
//! [`ScriptedRailgunOrder`]; the trigger hold keeps the gun's input down until
//! the shot actually leaves, and the `RailgunFired` observer consumes the
//! order. Every gate the gun has stays in force: the charge runs its authored
//! seconds, an empty magazine refuses the commit, and the twelve-second reload
//! is the cadence.
//!
//! There is no target here, and that is the point. A railgun does not steer;
//! the shot leaves down whatever line the hull holds when the charge
//! completes. Putting the bore on something is `ForceAlign`'s job, and keeping
//! the two separate is what lets a scenario charge several guns while one
//! alignment holds.

use bevy::prelude::*;

use super::{firing::RailgunFired, RailgunSectionInput, RailgunSectionMarker};

/// A pending scripted shot on one railgun section. One-shot - the shot
/// consumes it.
#[derive(Component, Clone, Copy, Debug, Default, Reflect)]
#[reflect(Component)]
pub struct ScriptedRailgunOrder;

/// Hold the trigger of every gun carrying an order.
///
/// The gun's own gates decide when the shot happens; the order only keeps the
/// trigger down until it does. A held trigger on this gun is a gun cycling at
/// its own cadence, so a scripted order that outlived its shot would fire
/// again forever - which is why the observer below retires it.
pub(super) fn hold_scripted_railgun_trigger(
    mut q_railgun: Query<
        &mut RailgunSectionInput,
        (With<ScriptedRailgunOrder>, With<RailgunSectionMarker>),
    >,
) {
    for mut input in &mut q_railgun {
        // Change-detection hygiene, like the bay and the AI trigger side.
        if !**input {
            **input = true;
        }
    }
}

/// Retire the order when the shell actually LEAVES, and release the trigger
/// with it.
///
/// On the shot, not on the commit: a gun whose charge was dumped by the safety
/// keeps its shell and its order, and fires when it can. This is the bay's
/// rule - `a bay ignored never burns the cooldown` - for the gun.
pub(super) fn on_railgun_fired_retire_scripted_order(
    fired: On<RailgunFired>,
    mut commands: Commands,
    mut q_railgun: Query<&mut RailgunSectionInput, With<ScriptedRailgunOrder>>,
) {
    if let Ok(mut input) = q_railgun.get_mut(fired.entity) {
        **input = false;
        commands
            .entity(fired.entity)
            .remove::<ScriptedRailgunOrder>();
    }
}
