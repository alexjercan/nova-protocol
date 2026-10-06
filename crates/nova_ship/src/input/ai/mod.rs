//! Enemy piloting: the AI behavior state machine that flies and fights the
//! non-player ships. An [`AISpaceshipMarker`] ship steps through
//! [`AIBehaviorState`] (idle/patrol/orbit/engage/evade/retreat) driven by its
//! [`AITarget`], under-fire memory ([`AIThreat`]), evasion clocks
//! ([`AIEvade`]) and territorial [`AILeash`]. Passive ships follow an
//! [`AIPatrolRoute`] or
//! [`AIOrbitDirective`]; the guns also run point defense against inbound
//! torpedoes ([`AIPointDefenseTarget`]).
//!
//! Touch this module to change how enemies behave. The AI writes the same ship
//! intents the player does (thrust, turret aim, fire), so the section and flight
//! layers treat AI and player ships identically. See the AI/behavior wiki page
//! for the state-machine design.
//!
//! ENGINE UNITS throughout. Every range, speed and margin in this module is
//! compared against an avian `Position` or `LinearVelocity` on every tick, so
//! the numbers here are world units (one is 10 m) and the conversion happens
//! once, at the scenario spawn that authored an override. The docs quote the
//! metric figure beside each one.

use bevy::prelude::*;
use nova_gameplay::prelude::*;

use crate::input::targeting::prelude::{SensorContacts, SensorRange, AI_SENSOR_RANGE};

mod acquisition;
mod behavior;
mod guns;
pub mod maneuver;
mod mission;
pub mod passive;
mod railgun;
mod threat;
mod torpedo;

use acquisition::{mirror_ai_combat_state, update_ai_target, update_point_defense_target};
use behavior::update_behavior_state;
use guns::{on_projectile_input, update_turret_target_input};
use maneuver::update_combat_flight;
use mission::interrupt_ai_ship_orders;
use passive::update_passive_flight;
use railgun::update_railgun_section_input;
use threat::{on_damage_track_threat, release_retaliation_target};
use torpedo::{update_torpedo_section_input, update_torpedo_target_input};

// The allocator that reads the point-defence envelope sits outside the AI
// module; the number itself stays here with the engagement-range chain it
// belongs to.
pub(crate) use self::acquisition::AI_POINT_DEFENSE_RANGE;
pub use self::{
    acquisition::{AIPointDefenseRange, AIPointDefenseTarget, AITarget},
    behavior::{AIBehaviorState, AIEngageRange, AILeash, AIOrbitDirective, AIPatrolRoute},
    guns::AI_FIRE_RANGE_FACTOR,
    maneuver::{AIStandoffClearance, AI_STANDOFF_OUTER_EDGE},
    mission::AIOrderInterruption,
    passive::{AIAvoidMargin, AIAvoidanceDetour, AIWaypointSlack},
    railgun::AIRailgun,
    threat::{AIEvade, AIThreat},
    torpedo::AI_TORPEDO_MAX_RANGE,
};

/// A world for an AI unit test: bare, plus the three things the passes under
/// test read.
///
/// Sensing asks for line of sight, and that question goes through avian's
/// `SpatialQuery`, which refuses to run without `ColliderTrees`. A rig that
/// spawns no colliders wants exactly this: an empty tree, in which nothing
/// stands between anybody. The pass also reads the shipped
/// [`TargetingSettings`], so the defaults stand in for a scenario's, and the
/// combat envelope reads the shipped [`FlightSettings`] for the stopping
/// rule's own margin.
#[cfg(test)]
pub(super) fn ai_test_world() -> World {
    let mut world = World::new();
    world.init_resource::<avian3d::collider_tree::ColliderTrees>();
    world.init_resource::<crate::input::targeting::prelude::TargetingSettings>();
    world.init_resource::<crate::prelude::FlightSettings>();
    world.init_resource::<nova_gameplay::prelude::GravitySettings>();
    world
}

/// One production frame of acquisition in a test world that has no schedule:
/// every hull publishes what it looks like, the single sensor pass publishes
/// what every ship can see, then the picker reads it. Running the picker alone
/// leaves every contact set empty, and running the sensor pass alone leaves
/// every ship as quiet as bare wreckage - both are silent passes rather than
/// failures, so tests go through here.
#[cfg(test)]
pub(super) fn sense_and_pick(world: &mut World) {
    use bevy::ecs::system::RunSystemOnce;

    world
        .run_system_once(crate::sections::signature::publish_ship_signatures)
        .unwrap();
    world
        .run_system_once(crate::input::targeting::update_sensor_contacts)
        .unwrap();
    world.run_system_once(update_ai_target).unwrap();
}

/// The AI behaviour, threat, patrol and target components and `SpaceshipAIInputPlugin`.
pub mod prelude {
    pub use super::{
        AIAvoidMargin, AIAvoidanceDetour, AIBehaviorState, AIEngageGrace, AIEngageRange, AIEvade,
        AILeash, AINonCombatant, AIOrbitDirective, AIOrderInterruption, AIPatrolRoute,
        AIPointDefenseRange, AIPointDefenseTarget, AIRailgun, AISpaceshipMarker,
        AIStandoffClearance, AITarget, AIThreat, AIWaypointSlack, SpaceshipAIInputPlugin,
        AI_FIRE_RANGE_FACTOR, AI_STANDOFF_OUTER_EDGE, AI_TORPEDO_MAX_RANGE,
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
///
/// The plugin runs in `Update` because its systems read eased poses. It sits
/// in [`SpaceshipInputSystems`](super::SpaceshipInputSystems), so the pause and
/// scenario-teardown gates cover it.
pub struct SpaceshipAIInputPlugin;

impl Plugin for SpaceshipAIInputPlugin {
    fn build(&self, app: &mut App) {
        trace!("SpaceshipAIInputPlugin: build");

        app.register_type::<AIBehaviorState>();
        app.register_type::<AITarget>();
        app.add_systems(
            Update,
            mirror_ai_combat_state.in_set(super::SpaceshipInputSystems),
        );
        app.register_type::<AIPointDefenseTarget>();
        app.register_type::<AIPatrolRoute>();
        app.register_type::<AIOrbitDirective>();
        app.register_type::<AIThreat>();
        app.register_type::<AIEvade>();
        app.register_type::<AIEngageGrace>();
        app.register_type::<AIAvoidanceDetour>();
        app.register_type::<AIEngageRange>();
        app.register_type::<AIPointDefenseRange>();
        app.register_type::<AIAvoidMargin>();
        app.register_type::<AIWaypointSlack>();
        app.register_type::<AIStandoffClearance>();
        app.register_type::<AIOrderInterruption>();

        // Threat sensing is an observer, not a system: HealthApplyDamage is
        // an entity event that propagates to the ship root, and reacting at
        // trigger time is what lets the source entity (the projectile) be
        // resolved before its despawn command applies.
        app.add_observer(on_damage_track_threat);
        app.add_observer(on_neutralized_stand_down);
        // The lance cadence is burned on the SHOT, not on the decision - see
        // `railgun::on_railgun_fired_burn_ai_cadence`.
        app.add_observer(railgun::on_railgun_fired_burn_ai_cadence);

        // The chain stays on the render clock because every
        // system in it reads an eased pose - `&Transform` on ship roots, the
        // muzzle and thruster `&GlobalTransform`, and the collider trees
        // `ai_line_of_fire_blocked` raycasts. A FixedUpdate system MUST read
        // raw `Position`/`Rotation` instead (docs/architecture.md),
        // and in FixedUpdate of frame N those eased poses still hold frame
        // N-1's values. These are decision reads, not impulses, and they are
        // correct against the frame they are deciding for.
        app.add_systems(
            Update,
            (
                release_retaliation_target,
                update_ai_target,
                update_point_defense_target,
                update_behavior_state,
                // Between perception and the flight writers: the helm
                // authority it hands back and forth is exactly what those
                // writers gate on, so deciding it first means a ship that
                // breaks off this frame flies this frame.
                interrupt_ai_ship_orders,
                // Before the passive pilot, and both write the same helm.
                // They never write it in the same frame - the behavior states
                // they answer to are disjoint - but the passive pilot READS
                // the maneuver, and this order means a ship breaking off
                // reads a helm combat has already released rather than the
                // velocity it was holding.
                update_combat_flight,
                update_passive_flight,
                update_turret_target_input,
                on_projectile_input,
                // Commit-on-launch runs before the trigger write, so the
                // frame after a launch counts the new torpedo against its
                // target's in-flight limit.
                update_torpedo_target_input,
                update_torpedo_section_input,
                // Last in the chain, and on the same render clock as the rest
                // of it: the commit reads the section's eased `GlobalTransform`
                // for the bore, which is a decision read against the frame it
                // is deciding for (see the note above).
                update_railgun_section_input,
            )
                .chain()
                // The per-turret assignment moved out to the shared point-
                // defence chain (it never depended on the AI), and the gun
                // systems below read what it writes - so the whole AI chain
                // now declares the edge the old in-chain position used to give
                // it for free.
                .after(super::point_defense::SpaceshipPointDefenseSystems)
                // Acquisition reads the ship's own `SensorContacts`, which the
                // one sensor pass publishes. Stated rather than inherited: the
                // two plugins are siblings and nothing else orders them.
                .after(super::targeting::SensorContactSystems)
                .in_set(super::SpaceshipInputSystems),
        );
    }
}

/// Marker component to identify the ai's spaceship.
///
/// This should be added to the root entity of the ai's spaceship.
/// Carries [`Allegiance::Enemy`], an empty [`RetaliationTarget`], an
/// [`AIBehaviorState`], an [`AITarget`] and the sensing pair
/// ([`SensorRange`] + [`SensorContacts`]) by requirement, so every AI-marked
/// root participates in the relation model, the behavior state machine and
/// target selection without extra spawn wiring. The AI sees
/// through the same pass the player does; only the reach differs.
#[derive(Component, Debug, Clone, Reflect)]
#[require(
    SpaceshipRootMarker,
    Allegiance = Allegiance::Enemy,
    RetaliationTarget,
    AIBehaviorState,
    AITarget,
    AIPointDefenseTarget,
    AIThreat,
    AIEvade,
    SensorRange = SensorRange(AI_SENSOR_RANGE),
    SensorContacts
)]
pub struct AISpaceshipMarker;

/// A non-combatant AI ship: it flies its passive routine (patrol / orbit /
/// idle) but NEVER acquires a target or engages - it simply cannot fight. An
/// unarmed ship (no turret or torpedo section) gets this at spawn (see
/// nova_scenario's `insert_spaceship_sections`); a Lifeline convoy hauler is
/// the first user, and a neutralized AI ship gains it from
/// [`on_neutralized_stand_down`].
///
/// It stays TARGET-able by hostiles (its allegiance is unchanged), so a
/// Player-aligned convoy is still something the enemy hunts and the player must
/// defend - it just does not shoot back or chase.
///
/// # The one gate that means "this hull does not fight"
///
/// Only the two ACQUISITION systems read it, and everything downstream falls
/// out of the empty picks they leave behind:
///
/// - `update_ai_target` keeps [`AITarget`] clear, so `update_behavior_state`
///   reads "nothing hostile", holds the passive routine, and `engages()` is
///   false for the gun, torpedo and maneuver systems.
/// - `update_point_defense_target` keeps [`AIPointDefenseTarget`] clear and
///   `update_turret_point_defense` keeps every [`TurretDefenseTarget`](super::point_defense::TurretDefenseTarget)
///   clear, which is the arm that closed the neutralized-wreck-still-swats-
///   torpedoes hole: point defense deliberately bypasses the behavior state,
///   so the passive routine alone never silenced it.
///
/// Adding the check HERE rather than to each consumer is deliberate. The gun
/// and torpedo systems write an explicit "hold fire" when they find no target,
/// so gating them too would only trade a released trigger for a skipped ship -
/// and a skipped ship LATCHES whatever its mounts were last told.
/// `mirror_ai_combat_state` is left running for the same reason: it is what
/// publishes the cleared `CombatLock` and lowers the stance.
#[derive(Component, Debug, Clone, Copy, Default, Reflect)]
#[reflect(Component)]
pub struct AINonCombatant;

/// Take a neutralized AI ship out of the fight: the crew is gone, so the hull
/// stops choosing targets, stops shooting, and stops defending itself.
///
/// This is the AI HALF of neutralization, and it lives here rather than in
/// `integrity::neutralize` on purpose: the integrity layer decides only that a
/// ship is out of the fight, and the AI decides what that means for AI ships.
/// Keyed on [`AISpaceshipMarker`], so a neutralized PLAYER ship never gains
/// this - a player is their own crew and keeps their own trigger.
///
/// The observer is the ONE place that says "the crew is gone", but it cannot
/// carry the rule alone: the picks that drive the guns ([`AITarget`],
/// [`AIPointDefenseTarget`], every mount's [`TurretDefenseTarget`](super::point_defense::TurretDefenseTarget)) are
/// recomputed from the world every frame, so a one-shot clear here is
/// overwritten on the next tick. What the observer CAN do is name the state
/// once, and it does - [`AINonCombatant`] is that name, and the two
/// acquisition systems are the only readers.
///
/// Neutralization is a CAPABILITY edge, not a physical one: the wreck keeps
/// its allegiance, its colliders and its health, so it still counts for
/// `OnNeutralized`, still shows on the HUD, and can still be shot to pieces.
fn on_neutralized_stand_down(
    add: On<Add, NeutralizedMarker>,
    mut commands: Commands,
    q_ai: Query<(), With<AISpaceshipMarker>>,
) {
    if q_ai.contains(add.entity) {
        commands.entity(add.entity).try_insert(AINonCombatant);
    }
}

#[cfg(test)]
mod tests {
    use nova_gameplay::test_support::unfinished_integrity_physics_app;

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

    /// [`AISpaceshipMarker`] requires [`SpaceshipRootMarker`], so `gravity`'s
    /// `On<Add, SpaceshipRootMarker>` observer covers AI ships too. This
    /// proves that required-component chain delivers the opt-in through the
    /// real plugin.
    #[test]
    fn an_ai_ship_opts_into_gravity_through_the_shared_ship_root_observer() {
        let mut app = unfinished_integrity_physics_app();
        app.add_plugins(NovaGravityPlugin);
        app.finish();

        let ai = app.world_mut().spawn(AISpaceshipMarker).id();
        app.update();

        assert!(app.world().get::<GravityAffected>(ai).is_some());
    }
}
