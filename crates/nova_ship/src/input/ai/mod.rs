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

// The point-defence envelope moved OUT of the AI module with the allocator that
// reads it, but the number itself stays here with the engagement-range chain it
// belongs to.
pub(crate) use self::acquisition::AI_POINT_DEFENSE_RANGE;
pub use self::{
    acquisition::{AIPointDefenseRange, AIPointDefenseTarget, AITarget},
    behavior::{AIBehaviorState, AIEngageRange, AILeash, AIOrbitDirective, AIPatrolRoute},
    guns::{AIFireCadence, AI_FIRE_RANGE_FACTOR},
    maneuver::AI_STANDOFF_OUTER_EDGE,
    passive::{AIAvoidanceDetour, AIWaypointSlack},
    threat::{AIEvade, AIThreat},
    torpedo::AITorpedoBay,
};

/// The AI behaviour, threat, patrol and target components and `SpaceshipAIInputPlugin`.
pub mod prelude {
    pub use super::{
        AIAvoidanceDetour, AIBehaviorState, AIEngageGrace, AIEngageRange, AIEvade, AIFireCadence,
        AILeash, AINonCombatant, AIOrbitDirective, AIPatrolRoute, AIPointDefenseRange,
        AIPointDefenseTarget, AISpaceshipMarker, AITarget, AIThreat, AITorpedoBay, AIWaypointSlack,
        SpaceshipAIInputPlugin, AI_FIRE_RANGE_FACTOR, AI_STANDOFF_OUTER_EDGE,
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
/// The plugin straddles two schedules: `update_fire_cadence` alone runs in
/// `FixedUpdate` so the burst clock does not move with the framerate, and
/// everything else runs in `Update` because it reads eased poses. Both halves
/// sit in [`SpaceshipInputSystems`](super::SpaceshipInputSystems), so the pause
/// and scenario-teardown gates cover them in either schedule.
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
        app.register_type::<AIAvoidanceDetour>();
        app.register_type::<AIEngageRange>();
        app.register_type::<AIPointDefenseRange>();
        app.register_type::<AIWaypointSlack>();

        // NOTE: threat sensing is an observer, not a system: HealthApplyDamage is
        // an entity event that propagates to the ship root, and reacting at
        // trigger time is what lets the source entity (the projectile) be
        // resolved before its despawn command applies.
        app.add_observer(on_damage_track_threat);
        app.add_observer(on_neutralized_stand_down);
        app.add_observer(insert_gravity_affected_on_ai_ship);

        // NOTE: the burst cadence is the one AI clock that must not move with
        // the framerate: its expiry writes the trigger that
        // `shoot_spawn_projectile` consumes in FixedUpdate, so ticked in Update
        // a burst window closes on a render-frame boundary. It reads no pose,
        // so it is the only part of the chain that can move to the physics
        // clock without breaking the rule below.
        app.add_systems(
            FixedUpdate,
            update_fire_cadence.in_set(super::SpaceshipInputSystems),
        );

        // NOTE: the rest of the chain stays on the render clock because every
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
                update_ai_target,
                update_point_defense_target,
                update_behavior_state,
                update_passive_flight,
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
                // The per-turret assignment moved out to the shared point-
                // defence chain (it never depended on the AI), and the gun
                // systems below read what it writes - so the whole AI chain
                // now declares the edge the old in-chain position used to give
                // it for free.
                .after(super::point_defense::SpaceshipPointDefenseSystems)
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
    use std::time::Duration;

    use bevy::time::TimeUpdateStrategy;
    use nova_gameplay::test_support::unfinished_integrity_physics_app;

    use super::*;
    use crate::prelude::{FlightSettings, SpaceshipInputSystems};

    /// Every `firing` value the cadence held at a fixed step, in order.
    #[derive(Resource, Default)]
    struct FiringSamples(Vec<bool>);

    /// The reason `update_fire_cadence` runs in `FixedUpdate`: the trigger it
    /// writes is consumed by `shoot_spawn_projectile` in `FixedUpdate`, so the
    /// burst window must open and close ON a fixed step. Its RATE is the same
    /// in either schedule - a frame's `Time` delta is the sum of that frame's
    /// fixed deltas - so rate proves nothing; the granularity does.
    ///
    /// One long frame of 2s covers 128 fixed steps and spans the whole 1.5s
    /// fire window. Ticked in `Update`, the cadence would advance once for the
    /// entire frame and every fixed step in it would read the SAME `firing`,
    /// holding the trigger a full frame past its expiry.
    #[test]
    fn the_burst_window_closes_on_a_fixed_step_not_a_frame_boundary() {
        const FRAME_SECS: f32 = 2.0;
        let mut app = unfinished_integrity_physics_app();
        app.add_plugins(SpaceshipAIInputPlugin);
        // Normally supplied by SpaceshipFlightPlugin, which this rig omits.
        app.init_resource::<FlightSettings>();
        app.init_resource::<FiringSamples>();
        app.finish();
        app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
            FRAME_SECS,
        )));
        // Time<Virtual> clamps a frame to max_delta (250ms by default), which
        // would cut the long frame this test is built on.
        app.world_mut()
            .resource_mut::<Time<Virtual>>()
            .set_max_delta(Duration::from_secs_f32(FRAME_SECS * 2.0));

        let ship = app.world_mut().spawn(AISpaceshipMarker).id();
        app.add_systems(
            FixedUpdate,
            (move |q: Query<&AIFireCadence>, mut samples: ResMut<FiringSamples>| {
                samples.0.push(q.get(ship).unwrap().firing);
            })
            .after(SpaceshipInputSystems),
        );

        // The first update lands a zero delta, so it runs no fixed step.
        app.update();
        app.world_mut().resource_mut::<FiringSamples>().0.clear();
        app.update();

        let samples = &app.world().resource::<FiringSamples>().0;
        let steps = app.world().resource::<Time<Fixed>>().timestep();
        let expected = (guns::AI_BURST_FIRE_SECS / steps.as_secs_f32()).round() as usize;
        assert!(
            samples.len() > expected,
            "the frame must cover the whole fire window: {} steps, need > {expected}",
            samples.len()
        );

        let closed_at = samples
            .iter()
            .position(|firing| !firing)
            .expect("the burst window must close inside the frame, not at its boundary");
        assert!(
            closed_at.abs_diff(expected) <= 1,
            "the window must close on the fixed step at {expected}, closed at {closed_at}"
        );
        assert!(
            samples[..closed_at].iter().all(|firing| *firing),
            "the window must stay open until it expires"
        );
    }

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
