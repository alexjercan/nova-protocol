//! Freeze/thaw of an AI ship's behavior-state progress, for a streamed
//! sector that despawns a live ship and later rebuilds it.
//!
//! A fresh spawn wires an AI controller from the ship's authored config the
//! same way it always has; [`thaw_ai`] is applied AFTER that wiring, in the
//! SAME command batch, so its inserts override the fresh ones.

use bevy::prelude::*;

use super::{
    behavior::{AIBehaviorState, AILeash, AIPatrolRoute},
    threat::{AIEvade, AIThreat},
    AIEngageGrace, AISpaceshipMarker,
};

/// An AI-controlled ship's behavior-state progress when its body froze: not
/// the config it was built from, which the fresh spawn already reapplies.
///
/// [`AITarget`](super::AITarget) and [`AIPointDefenseTarget`](super::AIPointDefenseTarget)
/// are deliberately absent: both are recomputed every frame from sensor
/// contacts, so a thawed ship re-acquires them on its own within one frame
/// rather than carrying an `Entity` that cannot survive the freeze's
/// despawn.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct FrozenAI {
    behavior: AIBehaviorState,
    patrol: Option<AIPatrolRoute>,
    engage_grace: Option<AIEngageGrace>,
    threat: AIThreat,
    evade: AIEvade,
    leash: Option<AILeash>,
}

/// Capture `ship`'s AI runtime state, or `None` when it carries no
/// [`AISpaceshipMarker`] (not an AI-flown ship).
///
/// The captured [`AIThreat`]'s attacker is cleared to `None`: it names the
/// ship root behind a recent hit, and an `Entity` cannot survive the
/// despawn a freeze follows this capture with. A thawed ship simply forgets
/// who hit it last; its damage-memory timer (whether it counts as
/// "recently under fire" at all) is still carried.
pub fn freeze_ai(world: &World, ship: Entity) -> Option<FrozenAI> {
    world.get::<AISpaceshipMarker>(ship)?;

    let mut threat = world
        .get::<AIThreat>(ship)
        .expect("freeze_ai: AISpaceshipMarker without its required AIThreat")
        .clone();
    threat.attacker = None;

    Some(FrozenAI {
        behavior: *world
            .get::<AIBehaviorState>(ship)
            .expect("freeze_ai: AISpaceshipMarker without its required AIBehaviorState"),
        patrol: world.get::<AIPatrolRoute>(ship).cloned(),
        engage_grace: world.get::<AIEngageGrace>(ship).cloned(),
        threat,
        evade: world
            .get::<AIEvade>(ship)
            .expect("freeze_ai: AISpaceshipMarker without its required AIEvade")
            .clone(),
        leash: world.get::<AILeash>(ship).cloned(),
    })
}

/// Apply `frozen` onto `ship`, a ship root spawned in the SAME command batch
/// its own spawner built it in, AFTER that spawner's own AI wiring: the
/// inserts here win.
pub fn thaw_ai(ship: &mut EntityCommands, frozen: FrozenAI) {
    ship.insert(frozen.behavior);
    ship.insert(frozen.threat);
    ship.insert(frozen.evade);
    if let Some(patrol) = frozen.patrol {
        ship.insert(patrol);
    }
    if let Some(grace) = frozen.engage_grace {
        ship.insert(grace);
    }
    if let Some(leash) = frozen.leash {
        ship.insert(leash);
    }
}
