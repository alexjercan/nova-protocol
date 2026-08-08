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
use nova_gameplay::prelude::*;

mod acquisition;
mod behavior;
mod guns;
pub mod maneuver;
pub mod passive;
mod threat;
mod torpedo;

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
/// [`SpaceshipInputPlugin`](super::SpaceshipInputPlugin).
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
        app.add_observer(on_neutralized_stand_down);
        app.add_observer(insert_gravity_affected_on_ai_ship);

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

/// Take a neutralized AI ship out of the fight so it stops being engaged and
/// chased. Its own weapons and thrusters are already gone, so it cannot act.
///
/// This is the AI HALF of neutralization, and it lives here rather than in
/// `integrity::neutralize` on purpose: the integrity layer decides only that a
/// ship is out of the fight, and the AI decides what that means for AI ships.
/// Keyed on [`AISpaceshipMarker`], so a neutralized PLAYER ship never gains
/// this.
fn on_neutralized_stand_down(
    add: On<Add, NeutralizedMarker>,
    mut commands: Commands,
    q_ai: Query<(), With<AISpaceshipMarker>>,
) {
    if q_ai.contains(add.entity) {
        commands.entity(add.entity).try_insert(AINonCombatant);
    }
}

/// PILOTED ships opt into gravity, and this is the AI half of that opt-in (the
/// player half is `gravity`'s own observer). It sits here because
/// [`AISpaceshipMarker`] requires the AI behavior state and so cannot move down
/// to the gravity layer. A hauler that GAINS an AI pilot mid-scenario (the
/// Lifeline loiter) opts in the moment its marker lands; both observers
/// `try_insert` the same idempotent marker.
fn insert_gravity_affected_on_ai_ship(add: On<Add, AISpaceshipMarker>, mut commands: Commands) {
    commands.entity(add.entity).try_insert(GravityAffected);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Both halves of the neutralize inversion in one rig: `integrity` inserts
    /// the generic marker, and only an AI ship stands down for it. Without the
    /// `q_ai` guard the player arm fails, which is the exact spurious
    /// `AINonCombatant` the old `is_ai` read prevented.
    #[test]
    fn only_a_neutralized_ai_ship_stands_down() {
        let mut app = App::new();
        app.add_observer(on_neutralized_stand_down);

        let ai = app.world_mut().spawn(AISpaceshipMarker).id();
        let player = app.world_mut().spawn(PlayerSpaceshipMarker).id();
        app.update();

        for ship in [ai, player] {
            app.world_mut().entity_mut(ship).insert(NeutralizedMarker);
        }
        app.update();

        assert!(
            app.world().entity(ai).contains::<AINonCombatant>(),
            "a neutralized AI ship is switched to non-combatant"
        );
        assert!(
            !app.world().entity(player).contains::<AINonCombatant>(),
            "a neutralized PLAYER ship is not - it carries no AI marker"
        );
    }

    /// The AI arm of the gravity opt-in; the player, torpedo and turret-round
    /// arms are tested beside their observers in `gravity`.
    #[test]
    fn an_ai_ship_opts_into_gravity() {
        let mut app = App::new();
        app.add_observer(insert_gravity_affected_on_ai_ship);

        let ai = app.world_mut().spawn(AISpaceshipMarker).id();
        app.update();

        assert!(app.world().get::<GravityAffected>(ai).is_some());
    }
}
