//! Mission policy: whether an AI ship's own judgement may take the helm back
//! from a scenario's [`ShipHelmOrder`], and when it gives it up again.
//!
//! The policy is a SEPARATE component from the order on purpose. The action
//! says what the ship must do; this says whether autonomous behavior is
//! allowed to interrupt it. Folding the two together would put a policy field
//! on every helm action and make the same order mean different things
//! depending on who was flying.
//!
//! Absence is the default and the default is never: an order owns the helm
//! until a terminal outcome unless the ship was authored to break off.

use bevy::prelude::*;
use nova_gameplay::prelude::*;

use super::{AITarget, AIThreat};
use crate::prelude::*;

/// When autonomous AI may take the helm back from an installed ship order.
///
/// Present = this ship is allowed to interrupt its mission. Absent = it is
/// not, which is what a cinematic actor and a tug under tow both need.
///
/// The condition is re-read every frame in both directions: the same signal
/// that takes the helm gives it back when it clears, so a cleanup ship that
/// breaks off to chase an intruder returns to its sweep on its own.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, Reflect)]
#[reflect(Component)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum AIOrderInterruption {
    /// Break off while a hostile is acquired, resume when the sky is clear.
    OnHostileContact,
    /// Break off while under fire, resume when the damage memory lapses.
    ///
    /// Later than `OnHostileContact` and narrower: a picket that keeps
    /// sweeping past an intruder it has seen, and only turns to fight once
    /// the intruder actually shoots.
    OnDamage,
}

/// Take the helm from an order whose ship's policy says to, and give it back
/// when the condition clears.
///
/// Runs in the input layer rather than the flight layer because the signals
/// it reads are the AI's - the acquired target and the damage memory. What it
/// does with them is a flight-layer transition, which is why both halves are
/// one call into the order module instead of component edits spelled out
/// here.
pub(super) fn interrupt_ai_ship_orders(
    mut commands: Commands,
    q_ships: Query<
        (
            Entity,
            &AIOrderInterruption,
            &AITarget,
            &AIThreat,
            Has<ShipOrderHelmAuthority>,
            Has<AIOrderInterrupted>,
        ),
        (
            With<SpaceshipRootMarker>,
            With<AISpaceshipMarker>,
            With<ShipHelmOrder>,
        ),
    >,
) {
    for (ship, policy, target, threat, holds_helm, interrupted) in &q_ships {
        let break_off = match policy {
            AIOrderInterruption::OnHostileContact => target.is_some(),
            AIOrderInterruption::OnDamage => threat.recently_damaged(),
        };
        if break_off && holds_helm {
            commands.queue(move |world: &mut World| interrupt_ship_order(world, ship));
        } else if !break_off && interrupted {
            commands.queue(move |world: &mut World| resume_ship_order(world, ship));
        }
    }
}
