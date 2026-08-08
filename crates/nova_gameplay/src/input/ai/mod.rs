//! Enemy piloting: the AI behavior state machine that flies and fights the
//! non-player ships. An [`AISpaceshipMarker`] ship steps through
//! [`AIBehaviorState`] (idle/patrol/orbit/engage/evade/retreat) driven by its
//! [`AITarget`], under-fire memory ([`AIThreat`]), evasion clocks
//! ([`AIEvade`]), fire cadence ([`AIFireCadence`]) and territorial
//! [`AILeash`]. Passive ships follow an [`AIPatrolRoute`] or
//! [`AIOrbitDirective`]; the guns also run point defense against inbound
//! torpedoes ([`AIPointDefenseTarget`]).
//!
//! Touch this module to change how enemies behave. The AI writes the same ship
//! intents the player does (thrust, turret aim, fire), so the section and flight
//! layers treat AI and player ships identically. See the AI/behavior wiki page
//! for the state-machine design.

use bevy::prelude::*;

use crate::prelude::*;

pub mod acquisition;
pub mod behavior;
pub mod guns;
pub mod maneuver;
pub mod passive;
pub mod threat;
pub mod torpedo;

use acquisition::{mirror_ai_combat_state, update_ai_target, update_point_defense_target};
use behavior::update_behavior_state;
use guns::{on_projectile_input, update_fire_cadence, update_turret_target_input};
use maneuver::{on_thruster_input, update_controller_target_rotation_torque};
use passive::update_passive_flight;
use threat::on_damage_track_threat;
use torpedo::{update_torpedo_section_input, update_torpedo_target_input};

pub use self::{
    acquisition::{AIPointDefenseTarget, AITarget},
    behavior::{AIBehaviorState, AILeash, AIOrbitDirective, AIPatrolRoute},
    guns::AIFireCadence,
    threat::{AIEvade, AIThreat},
    torpedo::AITorpedoBay,
};

/// The AI behaviour, threat, patrol and target components and `SpaceshipAIInputPlugin`.
pub mod prelude {
    pub use super::{
        AIBehaviorState, AIEngageGrace, AIEvade, AIFireCadence, AILeash, AINonCombatant,
        AIOrbitDirective, AIPatrolRoute, AIPointDefenseTarget, AISpaceshipMarker, AITarget,
        AIThreat, AITorpedoBay, SpaceshipAIInputPlugin,
    };
}

/// Arrival grace: a telegraphed ship holds its PASSIVE routine
/// (patrol/orbit/idle) and refuses the engage pull until this timer runs
/// out - enemies ARRIVE instead of appearing hot. Being shot ends the grace
/// immediately and PERMANENTLY (the ticking system pins the timer to
/// finished), mirroring the leash's damage override. Point defense is
/// untouched: a graced ship still swats inbound ordnance (the PD path
/// deliberately bypasses behavior states). Authored via
/// `AIControllerConfig::engage_delay`.
#[derive(Component, Debug, Clone, Reflect)]
#[reflect(Component)]
pub struct AIEngageGrace {
    /// Time left before the ship may engage. Starts counting down: the grace
    /// is running from the moment the ship arrives.
    pub timer: Cooldown,
}

impl AIEngageGrace {
    /// Builds an arrival grace with `seconds` before the ship may engage.
    pub fn new(seconds: f32) -> Self {
        Self {
            timer: Cooldown::started(seconds),
        }
    }
}

/// Runs the AI behavior state machine and the systems that turn its decisions
/// into ship intent (steer, thrust, aim, fire). Added by
/// [`SpaceshipInputPlugin`].
pub struct SpaceshipAIInputPlugin;

impl Plugin for SpaceshipAIInputPlugin {
    fn build(&self, app: &mut App) {
        debug!("SpaceshipAIInputPlugin: build");

        app.register_type::<AIBehaviorState>();
        app.register_type::<AITarget>();
        app.add_systems(
            Update,
            mirror_ai_combat_state.in_set(super::SpaceshipInputSystems),
        );
        app.register_type::<AIFireCadence>();
        app.register_type::<AIPointDefenseTarget>();
        app.register_type::<AIPatrolRoute>();
        app.register_type::<AIOrbitDirective>();
        app.register_type::<AIThreat>();
        app.register_type::<AIEvade>();
        app.register_type::<AITorpedoBay>();
        app.register_type::<AIEngageGrace>();

        // NOTE: threat sensing is an observer, not a system: HealthApplyDamage is
        // an entity event that propagates to the ship root, and reacting at
        // trigger time is what lets the source entity (the projectile) be
        // resolved before its despawn command applies.
        app.add_observer(on_damage_track_threat);

        app.add_systems(
            Update,
            (
                update_ai_target,
                update_point_defense_target,
                update_behavior_state,
                update_passive_flight,
                update_fire_cadence,
                update_controller_target_rotation_torque,
                on_thruster_input,
                update_turret_target_input,
                on_projectile_input,
                // NOTE: commit-on-launch runs before the trigger write: the frame
                // after a launch then sees the freshly reset bay cooldown
                // and drops the trigger, instead of holding it one frame
                // on the stale elapsed one.
                update_torpedo_target_input,
                update_torpedo_section_input,
            )
                .chain()
                .in_set(super::SpaceshipInputSystems),
        );
    }
}

/// Marker component to identify the ai's spaceship.
///
/// This should be added to the root entity of the ai's spaceship.
/// Carries [`Allegiance::Enemy`], an [`AIBehaviorState`] and an [`AITarget`]
/// by requirement, so every AI-marked root participates in the relation
/// model, the behavior state machine and target selection without extra
/// spawn wiring.
#[derive(Component, Debug, Clone, Reflect)]
#[require(
    SpaceshipRootMarker,
    Allegiance = Allegiance::Enemy,
    AIBehaviorState,
    AITarget,
    AIPointDefenseTarget,
    AIFireCadence,
    AIThreat,
    AIEvade
)]
pub struct AISpaceshipMarker;

/// A non-combatant AI ship: it flies its passive routine (patrol / orbit /
/// idle) but NEVER acquires a target or engages - it simply cannot fight. An
/// unarmed ship (no turret or torpedo section) gets this at spawn (see
/// nova_scenario's `insert_spaceship_sections`); a Lifeline convoy hauler is
/// the first user.
///
/// It stays TARGET-able by hostiles (its allegiance is unchanged), so a
/// Player-aligned convoy is still something the enemy hunts and the player must
/// defend - it just does not shoot back or chase. `update_ai_target` skips it
/// and keeps its [`AITarget`] clear, so `update_behavior_state` always reads
/// "nothing hostile" and holds the routine.
#[derive(Component, Debug, Clone, Copy, Default, Reflect)]
#[reflect(Component)]
pub struct AINonCombatant;
