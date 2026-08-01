//! The behavior state machine: [`AIBehaviorState`] and the transition rules
//! that pick it each frame from target, threat, territorial [`AILeash`] and
//! the passive assignment ([`AIPatrolRoute`] / [`AIOrbitDirective`]).

use avian3d::prelude::*;
use bevy::prelude::*;
use nova_events::prelude::*;

#[cfg(test)]
use super::acquisition::update_ai_target;
#[cfg(test)]
use super::guns::{on_projectile_input, update_turret_target_input};
#[cfg(test)]
use super::maneuver::on_thruster_input;
use super::threat::{AI_THREAT_AIM_COS, AI_THREAT_AIM_RANGE};
use crate::prelude::*;

/// What an AI ship is currently doing - the state skeleton of the AI combat
/// arc. One state per ship root, driven by `update_behavior_state`; every AI
/// system gates its behavior on it.
///
/// `Engage`, `Patrol`, `Idle` and `Evade` have real behavior today. `Retreat`
/// exists so its task slots into a stable enum instead of reshaping it: low-
/// integrity disengage (stubs to `Engage`).
#[derive(Component, Debug, Clone, Copy, Default, PartialEq, Eq, Reflect)]
#[reflect(Component)]
pub enum AIBehaviorState {
    /// Station-keeping: kill drift, hold position loosely, no fire.
    Idle,
    /// Fly the ship's [`AIPatrolRoute`] waypoint loop through the GOTO
    /// autopilot; no fire.
    Patrol,
    /// Circle the [`AIOrbitDirective`]'s gravity well through the ORBIT
    /// autopilot; no fire. Passive like Patrol/Idle: combat pulls the ship
    /// out and calm returns it.
    Orbit,
    /// Chase and shoot the hostile - today's whole AI, and the default so
    /// an AI ship dropped into a fight behaves exactly as before the state
    /// machine existed.
    #[default]
    Engage,
    /// Under-fire jinking: timed maneuvers off the pursuit vector while the
    /// guns keep fighting, decaying back to `Engage`.
    Evade,
    /// Low-integrity disengage; stubs to `Engage` until then.
    Retreat,
}

impl AIBehaviorState {
    /// Whether this state runs the engage-style chase/aim/fire pipeline.
    /// `Evade` fights it too - the jink only swaps the flight direction,
    /// the guns stay on target. `Retreat` deliberately stubs to Engage
    /// behavior until its task lands (see the variant docs).
    pub(crate) fn engages(&self) -> bool {
        matches!(self, Self::Engage | Self::Evade | Self::Retreat)
    }

    /// Whether this state runs a passive routine (no fire, autopilot-flown);
    /// the calm fallback states of [`next_behavior_state`].
    fn is_passive(&self) -> bool {
        matches!(self, Self::Idle | Self::Patrol | Self::Orbit)
    }
}

/// The waypoint loop an AI ship flies while nothing hostile is close enough
/// to fight. Present = the ship has a patrol assignment: the no-hostile
/// fallback state becomes `Patrol` instead of `Idle`
/// (`next_behavior_state`). Spawn-configured (scenario/editor); flown by
/// `update_passive_flight` through the real GOTO autopilot, leg by leg.
#[derive(Component, Debug, Clone, PartialEq, Reflect)]
#[reflect(Component)]
pub struct AIPatrolRoute {
    /// The loop's waypoints, world coordinates. Legs shorter than the
    /// arrival radius (arrival_standoff + `AI_WAYPOINT_SLACK`) are all
    /// "arrived" at once and collapse into station keeping at the cluster.
    pub waypoints: Vec<Vec3>,
    /// Index of the waypoint currently being flown to. Out-of-range values
    /// (both fields are inspector-editable) self-heal by wrapping.
    pub current: usize,
}

impl AIPatrolRoute {
    /// A route starting at its first waypoint.
    pub fn new(waypoints: Vec<Vec3>) -> Self {
        Self {
            waypoints,
            current: 0,
        }
    }

    /// `current` wrapped into range; `None` for an empty route. An edited
    /// route (waypoints shrunk below `current`) must strand the patrol
    /// never, so out-of-range indices wrap instead of failing the lookup.
    fn wrapped_current(&self) -> Option<usize> {
        (!self.waypoints.is_empty()).then(|| self.current % self.waypoints.len())
    }

    /// The waypoint currently being flown to; `None` for an empty route.
    pub(crate) fn current_waypoint(&self) -> Option<Vec3> {
        Some(self.waypoints[self.wrapped_current()?])
    }

    /// Turn onto the next leg, wrapping - the route is a loop. Also snaps
    /// an out-of-range `current` back into range.
    pub(crate) fn advance(&mut self) {
        if let Some(current) = self.wrapped_current() {
            self.current = (current + 1) % self.waypoints.len();
        }
    }
}

/// Directs an AI ship to orbit a gravity well while nothing hostile is close
/// enough to fight. Present = the no-hostile fallback state becomes `Orbit`,
/// taking precedence over `Patrol` (`next_behavior_state`). The well is
/// named by its scenario [`EntityId`]; `update_passive_flight` resolves it
/// and keeps the ORBIT autopilot engaged on it, mirroring how Patrol flies
/// its GOTO legs. Spawn-configured (scenario config); an id that resolves to
/// no live well behaves like Idle-without-a-STOP (the ship simply drifts)
/// until the well appears.
#[derive(Component, Debug, Clone, Reflect)]
#[reflect(Component)]
pub struct AIOrbitDirective {
    /// Scenario id of the gravity-well entity to circle.
    pub well: EntityId,
}

/// Fraction of the leash radius a PASSIVE leashed ship must be inside of
/// before it may engage again. The asymmetry is hysteresis: combat breaks off
/// strictly beyond the radius, but re-engagement needs the ship well back
/// inside - without the band, a hostile parked at the boundary makes the ship
/// ping-pong Engage/Patrol every crossing.
const LEASH_REENGAGE_FRACTION: f32 = 0.8;

/// Whether `distance` from the leash center exceeds the state-dependent
/// threshold: the full radius for combat states (break off), the tighter
/// re-engage band for passive ones (hold fire until well inside). Pure
/// for unit testing.
fn leash_exceeded(current: AIBehaviorState, distance: f32, leash: &AILeash) -> bool {
    let threshold = if current.is_passive() {
        leash.radius * LEASH_REENGAGE_FRACTION
    } else {
        leash.radius
    };
    distance > threshold
}

/// Territorial tether (round 3): a leashed ship abandons combat and returns
/// to its passive routine whenever it strays beyond `radius` of `center` -
/// the shakedown scavenger stays at the debris field instead of chasing
/// across the map. Being under fire overrides the leash (a ship dragged out
/// and shot may defend itself); the tether reasserts once the damage memory
/// fades.
#[derive(Component, Debug, Clone, Reflect)]
#[reflect(Component)]
pub struct AILeash {
    /// World-space anchor (the patrol centroid, else the spawn position).
    pub center: Vec3,
    /// Distance from `center` beyond which combat breaks off.
    pub radius: f32,
}

/// Hostile-detection range (m): a passive ship (Idle/Patrol/Orbit) leaves
/// its routine and engages only when the acquired target is inside this range.
/// Acquisition itself scans out to [`AI_TARGET_MAX_RANGE`], so a patrolling
/// ship knows what is out there without aborting the patrol for it; combat
/// states keep holding on any acquired target, as before.
const AI_ENGAGE_RANGE: f32 = 800.0;

/// The skeleton's transitions: combat states need a hostile to fight - with
/// none acquired every state falls back to its passive routine (`Orbit` with
/// an orbit directive, else `Patrol` with a route, else `Idle`) - and a
/// hostile inside [`AI_ENGAGE_RANGE`] pulls the passive states into `Engage`.
/// One merely acquired further out does not abort the routine (detection
/// range) - unless it is shooting: a recent hostile hit interrupts the
/// routine at any acquired distance. Under threat, `Engage` breaks into
/// `Evade` (gated by the refractory cooldown), which decays back to `Engage`
/// when its cycle expires. Pure for unit testing.
fn next_behavior_state(
    current: AIBehaviorState,
    hostile_distance: Option<f32>,
    has_orbit: bool,
    has_route: bool,
    beyond_leash: bool,
    grace_held: bool,
    threat: ThreatSignals,
) -> AIBehaviorState {
    let passive = if has_orbit {
        AIBehaviorState::Orbit
    } else if has_route {
        AIBehaviorState::Patrol
    } else {
        AIBehaviorState::Idle
    };
    // The territorial tether: beyond the leash, combat breaks off and
    // passive states refuse to engage - the routine (patrol home) walks
    // the ship back inside. Recent damage overrides it: a ship dragged
    // out and shot defends itself until the memory fades.
    if beyond_leash && !threat.recently_damaged {
        return passive;
    }
    // The arrival grace: a telegraphed ship holds its routine until the timer
    // runs out. Damage overrides here too - and the ticking system makes that
    // override permanent by pinning the timer (a shot ship never calms back
    // into its entrance). UNCONDITIONAL on the current state on purpose:
    // `AIBehaviorState`'s default is Engage, so every graced scenario spawn
    // takes THIS return on its first behavior tick to land on its routine -
    // restricting it to passive states would silently break every real
    // telegraphed arrival ('s mutation probe).
    if grace_held && !threat.recently_damaged {
        return passive;
    }
    let Some(distance) = hostile_distance else {
        return passive;
    };
    match current {
        state if state.is_passive() && (distance <= AI_ENGAGE_RANGE || threat.recently_damaged) => {
            AIBehaviorState::Engage
        }
        state if state.is_passive() => passive,
        AIBehaviorState::Engage if threat.threatened() && threat.evade_ready => {
            AIBehaviorState::Evade
        }
        AIBehaviorState::Evade if threat.evade_expired => AIBehaviorState::Engage,
        // The remaining combat states hold; their exit triggers are their
        // tasks' scope.
        state => state,
    }
}

/// The threat model's inputs to [`next_behavior_state`], gathered by
/// [`update_behavior_state`]. A struct (not bools in the signature) so the
/// call sites stay readable and the pure tests name what they assert.
#[derive(Debug, Clone, Copy, Default)]
struct ThreatSignals {
    /// A hostile hit landed within the memory window ([`AIThreat`]).
    recently_damaged: bool,
    /// The current target is inside [`AI_THREAT_AIM_RANGE`] holding its
    /// nose on me ([`AI_THREAT_AIM_COS`]).
    aimed_at: bool,
    /// The evade cooldown has elapsed ([`AIEvade`]).
    evade_ready: bool,
    /// The running evade cycle has expired ([`AIEvade`]).
    evade_expired: bool,
}

impl ThreatSignals {
    /// The threat model proper: under fire, or under a hostile's guns.
    fn threatened(&self) -> bool {
        self.recently_damaged || self.aimed_at
    }
}

/// Drive each AI ship's [`AIBehaviorState`] from its [`AITarget`] and the
/// threat model ([`AIThreat`] + the aiming-at-me signal). Runs after
/// acquisition and before the behavior systems in the same frame so a
/// transition takes effect immediately (no one-frame stale-state window).
/// Also owns the threat/evade clocks: the damage memory and evade cooldown
/// tick every frame, the evade cycle and jink cadence only while evading,
/// and the Evade edges arm them (cycle + jink on entry, cooldown on exit).
pub(super) fn update_behavior_state(
    time: Res<Time>,
    mut q_spaceship: Query<
        (
            &Transform,
            Option<&ComputedCenterOfMass>,
            &mut AIBehaviorState,
            &AITarget,
            &mut AIThreat,
            &mut AIEvade,
            Has<AIOrbitDirective>,
            Has<AIPatrolRoute>,
            Option<&AILeash>,
            Option<&mut AIEngageGrace>,
        ),
        With<AISpaceshipMarker>,
    >,
    q_target: Query<(&Transform, Option<&ComputedCenterOfMass>)>,
) {
    for (
        transform,
        com,
        mut state,
        target,
        mut threat,
        mut evade,
        has_orbit,
        has_route,
        leash,
        mut grace,
    ) in &mut q_spaceship
    {
        threat.damage_memory.tick(time.delta());
        let grace_held = match grace.as_deref_mut() {
            Some(grace) => {
                grace.timer.tick(time.delta());
                if threat.recently_damaged() && !grace.timer.is_finished() {
                    // Shot during the entrance: the courtesy is over for good
                    // (a finished timer never holds again). Tick to the end
                    // rather than set_elapsed - only tick updates the
                    // finished flag (Bevy Timer semantics).
                    let remaining = grace.timer.remaining();
                    grace.timer.tick(remaining);
                }
                !grace.timer.is_finished()
            }
            None => false,
        };
        evade.cooldown.tick(time.delta());
        if *state == AIBehaviorState::Evade {
            evade.duration.tick(time.delta());
            if evade.jink.tick(time.delta()).just_finished() {
                evade.leg = evade.leg.wrapping_add(1);
            }
        }

        // The detection distance runs anchor to anchor - the same
        // live-structure vector the behavior systems fly and shoot along.
        let own_anchor = live_structure_anchor(transform, com);
        let target_info = (**target).and_then(|target| q_target.get(target).ok());
        let hostile_distance = target_info.map(|(t_transform, t_com)| {
            live_structure_anchor(t_transform, t_com).distance(own_anchor)
        });
        // Aiming-at-me: the hostile's hull forward held on my anchor inside
        // aim range. The hull axis is a cheap proxy for its guns (see
        // AI_THREAT_AIM_COS). Anchor to anchor, like every other AI vector.
        let aimed_at =
            target_info
                .zip(hostile_distance)
                .is_some_and(|((t_transform, t_com), distance)| {
                    distance <= AI_THREAT_AIM_RANGE
                        && (own_anchor - live_structure_anchor(t_transform, t_com))
                            .try_normalize()
                            .is_some_and(|bearing| {
                                t_transform.forward().dot(bearing) > AI_THREAT_AIM_COS
                            })
                });
        let signals = ThreatSignals {
            recently_damaged: threat.recently_damaged(),
            aimed_at,
            evade_ready: evade.cooldown.is_finished(),
            evade_expired: evade.duration.is_finished(),
        };

        let beyond_leash = leash
            .is_some_and(|leash| leash_exceeded(*state, own_anchor.distance(leash.center), leash));
        let next = next_behavior_state(
            *state,
            hostile_distance,
            has_orbit,
            has_route,
            beyond_leash,
            grace_held,
            signals,
        );
        // Change-detection hygiene: only write on a real transition.
        if *state != next {
            // The Evade edges arm the clocks: a fresh cycle + jink cadence
            // on entry, the refractory cooldown on ANY exit (expiry, target
            // loss, a future retreat).
            if next == AIBehaviorState::Evade {
                evade.duration.reset();
                evade.jink.reset();
            }
            if *state == AIBehaviorState::Evade {
                evade.cooldown.reset();
            }
            *state = next;
        }
    }
}

#[cfg(test)]
mod behavior_state_tests {
    use avian3d::collider_tree::ColliderTrees;
    use bevy::ecs::system::RunSystemOnce;

    use super::*;

    /// No threat, evade ready: the baseline signals of a ship that has
    /// never been shot.
    fn calm() -> ThreatSignals {
        ThreatSignals {
            evade_ready: true,
            ..default()
        }
    }

    /// The arrival grace: a graced passive ship refuses the engage pull with
    /// a hostile in range; damage overrides the grace; grace composes with
    /// the leash (passive either way).
    #[test]
    fn the_grace_holds_passive_and_damage_overrides_it() {
        use AIBehaviorState::*;
        let near = Some(100.0);

        // Graced, hostile in range: the routine holds.
        assert_eq!(
            next_behavior_state(Patrol, near, false, true, false, true, calm()),
            Patrol
        );
        assert_eq!(
            next_behavior_state(Idle, near, false, false, false, true, calm()),
            Idle
        );
        // Shot during the entrance: the grace yields NOW.
        let shot = ThreatSignals {
            recently_damaged: true,
            evade_ready: false,
            ..default()
        };
        assert_eq!(
            next_behavior_state(Patrol, near, false, true, false, true, shot),
            Engage
        );
        // Grace + beyond-leash compose: passive, no double-engage path.
        assert_eq!(
            next_behavior_state(Patrol, near, false, true, true, true, calm()),
            Patrol
        );
        // The LOAD-BEARING row: a graced ship in Engage demotes to its
        // routine - AIBehaviorState defaults to Engage, so every graced
        // scenario spawn's first tick IS this transition.
        assert_eq!(
            next_behavior_state(Engage, near, false, true, false, true, calm()),
            Patrol
        );
        // Delivery guard: the same ungraced shape engages immediately.
        assert_eq!(
            next_behavior_state(Patrol, near, false, true, false, false, calm()),
            Engage
        );
    }

    /// Leash hysteresis: combat breaks off strictly beyond the radius, but a
    /// passive ship only re-engages once well back inside the re-engage band
    /// - between the two thresholds an engaged ship keeps fighting and a
    /// passive one keeps walking home, so a hostile parked at the boundary
    /// cannot ping-pong the state.
    #[test]
    fn leash_hysteresis_uses_a_reengage_band() {
        use AIBehaviorState::*;
        let leash = AILeash {
            center: Vec3::ZERO,
            radius: 100.0,
        };
        // In the band (between 80 and 100): combat holds, passive holds.
        assert!(
            !leash_exceeded(Engage, 90.0, &leash),
            "combat holds in the band"
        );
        assert!(
            leash_exceeded(Patrol, 90.0, &leash),
            "passive holds fire in the band"
        );
        // Beyond the radius: everyone is out.
        assert!(leash_exceeded(Engage, 110.0, &leash));
        // Well inside: everyone is in.
        assert!(!leash_exceeded(Patrol, 70.0, &leash));
    }

    /// The territorial tether (playtest round 3): an engaged leashed ship
    /// beyond its radius breaks off to its passive routine, and a passive
    /// one beyond the leash refuses to engage - but recent damage
    /// overrides the tether (a dragged-out ship defends itself). Inside
    /// the leash everything behaves exactly as unleashed (delivery
    /// guard).
    #[test]
    fn the_leash_breaks_off_combat_beyond_its_radius() {
        use AIBehaviorState::*;
        let near = Some(100.0);

        // Engaged beyond the leash: back to the routine.
        assert_eq!(
            next_behavior_state(Engage, near, false, true, true, false, calm()),
            Patrol
        );
        // Passive beyond the leash: refuses to engage a hostile in range.
        assert_eq!(
            next_behavior_state(Patrol, near, false, true, true, false, calm()),
            Patrol
        );
        // Under fire the tether yields: the ship fights back.
        assert_eq!(
            next_behavior_state(
                Engage,
                near,
                false,
                true,
                true,
                false,
                ThreatSignals {
                    recently_damaged: true,
                    evade_ready: false,
                    ..Default::default()
                }
            ),
            Engage
        );
        // Delivery guard: INSIDE the leash the same engaged ship keeps
        // engaging - the tether only acts beyond the radius.
        assert_eq!(
            next_behavior_state(Engage, near, false, true, false, false, calm()),
            Engage
        );
    }

    #[test]
    fn transitions_need_a_hostile_to_fight() {
        use AIBehaviorState::*;

        // No hostile: every state falls back to the passive routine - Idle
        // without a patrol assignment, Patrol with one.
        for state in [Idle, Patrol, Engage, Evade, Retreat] {
            assert_eq!(
                next_behavior_state(state, None, false, false, false, false, calm()),
                Idle,
                "from {state:?}"
            );
            assert_eq!(
                next_behavior_state(state, None, false, true, false, false, calm()),
                Patrol,
                "from {state:?}"
            );
        }
        // Hostile inside detection range: passive states engage, combat
        // states hold (their exit triggers belong to their own tasks).
        let near = Some(AI_ENGAGE_RANGE * 0.5);
        assert_eq!(
            next_behavior_state(Idle, near, false, false, false, false, calm()),
            Engage
        );
        assert_eq!(
            next_behavior_state(Patrol, near, false, true, false, false, calm()),
            Engage
        );
        assert_eq!(
            next_behavior_state(Engage, near, false, false, false, false, calm()),
            Engage
        );
        assert_eq!(
            next_behavior_state(Evade, near, false, false, false, false, calm()),
            Evade
        );
        assert_eq!(
            next_behavior_state(Retreat, near, false, false, false, false, calm()),
            Retreat
        );
    }

    #[test]
    fn a_hostile_beyond_detection_range_does_not_abort_the_routine() {
        use AIBehaviorState::*;

        // Acquired (inside the 2000 m scan) but outside the 800 m detection
        // range: the passive states keep their routine...
        let far = Some(AI_ENGAGE_RANGE * 1.5);
        assert_eq!(
            next_behavior_state(Patrol, far, false, true, false, false, calm()),
            Patrol
        );
        assert_eq!(
            next_behavior_state(Idle, far, false, false, false, false, calm()),
            Idle
        );
        // ...while a combat state already on that target keeps fighting -
        // the detection range gates entry, not pursuit.
        assert_eq!(
            next_behavior_state(Engage, far, false, false, false, false, calm()),
            Engage
        );
    }

    #[test]
    fn threats_break_engage_into_evade_when_the_cooldown_allows() {
        use AIBehaviorState::*;

        let near = Some(AI_ENGAGE_RANGE * 0.5);
        // Either threat signal breaks Engage into Evade...
        let shot = ThreatSignals {
            recently_damaged: true,
            ..calm()
        };
        assert_eq!(
            next_behavior_state(Engage, near, false, false, false, false, shot),
            Evade
        );
        let aimed = ThreatSignals {
            aimed_at: true,
            ..calm()
        };
        assert_eq!(
            next_behavior_state(Engage, near, false, false, false, false, aimed),
            Evade
        );
        // ...but not during the refractory cooldown: threats between evade
        // cycles are fought through, or the standoff orbit never shows.
        let refractory = ThreatSignals {
            recently_damaged: true,
            aimed_at: true,
            evade_ready: false,
            ..default()
        };
        assert_eq!(
            next_behavior_state(Engage, near, false, false, false, false, refractory),
            Engage
        );
    }

    #[test]
    fn evade_holds_until_its_cycle_expires_then_reengages() {
        use AIBehaviorState::*;

        let near = Some(AI_ENGAGE_RANGE * 0.5);
        // Mid-cycle, even with the threat gone: the jink is timed, not
        // signal-chasing.
        assert_eq!(
            next_behavior_state(Evade, near, false, false, false, false, calm()),
            Evade
        );
        // Expiry decays back to Engage even under an ongoing threat - the
        // cooldown (armed on exit) is what spaces the cycles.
        let expired_under_fire = ThreatSignals {
            recently_damaged: true,
            evade_expired: true,
            ..calm()
        };
        assert_eq!(
            next_behavior_state(Evade, near, false, false, false, false, expired_under_fire),
            Engage
        );
    }

    #[test]
    fn getting_shot_interrupts_the_routine_beyond_detection_range() {
        use AIBehaviorState::*;

        // Acquired but outside detection range: a passive ship normally
        // keeps its routine (test above) - but not while taking fire.
        let far = Some(AI_ENGAGE_RANGE * 1.5);
        let shot = ThreatSignals {
            recently_damaged: true,
            ..calm()
        };
        assert_eq!(
            next_behavior_state(Patrol, far, false, true, false, false, shot),
            Engage
        );
        assert_eq!(
            next_behavior_state(Idle, far, false, false, false, false, shot),
            Engage
        );
        // Merely being aimed at from out there does not: the aim signal is
        // range-gated well inside detection range anyway.
        let aimed = ThreatSignals {
            aimed_at: true,
            ..calm()
        };
        assert_eq!(
            next_behavior_state(Patrol, far, false, true, false, false, aimed),
            Patrol
        );
    }

    #[test]
    fn an_orbit_directive_wins_the_passive_fallback() {
        use AIBehaviorState::*;

        // Precedence orbit > patrol > idle, from every state with no
        // hostile acquired.
        for state in [Idle, Patrol, Orbit, Engage, Evade, Retreat] {
            assert_eq!(
                next_behavior_state(state, None, true, true, false, false, calm()),
                Orbit,
                "orbit beats patrol from {state:?}"
            );
            assert_eq!(
                next_behavior_state(state, None, true, false, false, false, calm()),
                Orbit,
                "orbit without a route from {state:?}"
            );
        }
        // A far-off acquired hostile does not abort the orbit...
        let far = Some(AI_ENGAGE_RANGE * 1.5);
        assert_eq!(
            next_behavior_state(Orbit, far, true, false, false, false, calm()),
            Orbit
        );
        // ...one in detection range pulls it into combat, as does taking a
        // hit from further out.
        let near = Some(AI_ENGAGE_RANGE * 0.5);
        assert_eq!(
            next_behavior_state(Orbit, near, true, false, false, false, calm()),
            Engage
        );
        let shot = ThreatSignals {
            recently_damaged: true,
            ..calm()
        };
        assert_eq!(
            next_behavior_state(Orbit, far, true, false, false, false, shot),
            Engage
        );
        // And calm returns the fight to the ring.
        assert_eq!(
            next_behavior_state(Engage, None, true, false, false, false, calm()),
            Orbit
        );
    }

    #[test]
    fn an_ai_ship_spawns_engaged_by_requirement() {
        // The default state preserves pre-state-machine behavior: an AI
        // ship dropped into a fight chases and shoots immediately.
        let mut world = World::new();
        let ship = world.spawn(AISpaceshipMarker).id();
        assert_eq!(
            *world.entity(ship).get::<AIBehaviorState>().unwrap(),
            AIBehaviorState::Engage
        );
    }

    #[test]
    fn the_state_idles_without_a_target_and_reengages_with_one() {
        // Drive the real acquisition -> transition pipeline: no hostile in
        // range means no target means Idle; a hostile appearing re-engages.
        let mut world = World::new();
        world.init_resource::<Time>();
        let ship = world.spawn((AISpaceshipMarker, Transform::default())).id();

        world.run_system_once(update_ai_target).unwrap();
        world.run_system_once(update_behavior_state).unwrap();
        assert_eq!(
            *world.entity(ship).get::<AIBehaviorState>().unwrap(),
            AIBehaviorState::Idle,
            "no hostile in the world: nothing to engage"
        );

        world.spawn((
            SpaceshipRootMarker,
            PlayerSpaceshipMarker,
            Transform::from_translation(Vec3::new(100.0, 0.0, 0.0)),
        ));
        world.run_system_once(update_ai_target).unwrap();
        world.run_system_once(update_behavior_state).unwrap();
        assert_eq!(
            *world.entity(ship).get::<AIBehaviorState>().unwrap(),
            AIBehaviorState::Engage,
            "a hostile appearing pulls Idle back into the fight"
        );
    }

    /// A non-combatant (the convoy hauler) never acquires a target even with
    /// a hostile point-blank, so it holds its passive routine instead of
    /// chasing. An armed ship in the same spot engages - the control.
    #[test]
    fn a_non_combatant_never_targets_or_engages() {
        let mut world = World::new();
        world.init_resource::<Time>();

        // A hostile (player-marked) ship well inside acquisition range.
        world.spawn((
            SpaceshipRootMarker,
            PlayerSpaceshipMarker,
            Transform::from_translation(Vec3::new(100.0, 0.0, 0.0)),
        ));

        let hauler = world
            .spawn((AISpaceshipMarker, AINonCombatant, Transform::default()))
            .id();
        world.run_system_once(update_ai_target).unwrap();
        world.run_system_once(update_behavior_state).unwrap();
        assert_eq!(
            *world.entity(hauler).get::<AITarget>().unwrap(),
            AITarget(None),
            "a non-combatant never acquires a target"
        );
        assert!(
            world
                .entity(hauler)
                .get::<AIBehaviorState>()
                .unwrap()
                .is_passive(),
            "so it holds its passive routine instead of engaging"
        );

        // Control: an ARMED ship (no tag) acquires the same hostile.
        let fighter = world.spawn((AISpaceshipMarker, Transform::default())).id();
        world.run_system_once(update_ai_target).unwrap();
        assert!(
            world.entity(fighter).get::<AITarget>().unwrap().is_some(),
            "an ordinary AI ship acquires the hostile the non-combatant ignored"
        );
    }

    #[test]
    fn idle_cuts_thrust_fire_and_aim() {
        // Flip a fully lit ship to Idle with its target still present: every
        // actuator must be explicitly zeroed, not left at its last value.
        let mut world = World::new();
        // Empty collider trees for the fire gate's SpatialQuery: no
        // colliders means no occluders, which is this rig's intent.
        world.init_resource::<ColliderTrees>();
        let player = world
            .spawn((
                SpaceshipRootMarker,
                PlayerSpaceshipMarker,
                Transform::from_translation(Vec3::new(100.0, 0.0, 0.0)),
            ))
            .id();
        let ship = world
            .spawn((
                AISpaceshipMarker,
                AIBehaviorState::Idle,
                AITarget(Some(player)),
                Transform::default(),
                LinearVelocity(Vec3::ZERO),
            ))
            .id();
        let thruster = world
            .spawn((
                ThrusterSectionMarker,
                ThrusterSectionInput(1.0),
                GlobalTransform::IDENTITY,
                ChildOf(ship),
            ))
            .id();
        let turret = world
            .spawn((
                TurretSectionMarker,
                TurretSectionTargetInput(Some(Vec3::X)),
                TurretSectionTargetVelocity(Vec3::ZERO),
                TurretSectionAimPoint(None),
                TurretSectionConfigHelper(TurretSectionConfig::default()),
                TurretSectionInput(true),
                TurretSectionMuzzleEntity(Entity::PLACEHOLDER),
                ChildOf(ship),
            ))
            .id();

        world.run_system_once(on_thruster_input).unwrap();
        world.run_system_once(update_turret_target_input).unwrap();
        world.run_system_once(on_projectile_input).unwrap();

        assert_eq!(
            **world
                .entity(thruster)
                .get::<ThrusterSectionInput>()
                .unwrap(),
            0.0,
            "Idle cuts the burn"
        );
        assert_eq!(
            **world
                .entity(turret)
                .get::<TurretSectionTargetInput>()
                .unwrap(),
            None,
            "Idle clears the turret aim"
        );
        assert!(
            !**world.entity(turret).get::<TurretSectionInput>().unwrap(),
            "Idle holds fire"
        );
    }
}

#[cfg(test)]
mod engage_grace_tests {
    use core::time::Duration;

    use bevy::time::TimeUpdateStrategy;

    use super::*;

    /// An app ticking the real behavior-state system on a manual clock
    /// (0.25s/update measured - the virtual-time clamp), with a hostile
    /// player well inside engage range.
    fn grace_app(grace: Option<f32>) -> (App, Entity) {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
            0.5,
        )));
        app.add_systems(Update, update_behavior_state);
        let player = app
            .world_mut()
            .spawn((
                SpaceshipRootMarker,
                PlayerSpaceshipMarker,
                Transform::from_translation(Vec3::new(0.0, 0.0, -300.0)),
                LinearVelocity(Vec3::ZERO),
            ))
            .id();
        let mut ship = app.world_mut().spawn((
            AISpaceshipMarker,
            // The DEFAULT state (Engage) - the production spawn shape: the
            // grace's first job is demoting it onto the routine.
            AIBehaviorState::default(),
            AIPatrolRoute::new(vec![Vec3::ZERO, Vec3::X * 100.0]),
            AITarget(Some(player)),
            Transform::default(),
            LinearVelocity(Vec3::ZERO),
        ));
        if let Some(seconds) = grace {
            ship.insert(AIEngageGrace::new(seconds));
        }
        let ship = ship.id();
        (app, ship)
    }

    fn state(app: &mut App, ship: Entity) -> AIBehaviorState {
        *app.world().entity(ship).get::<AIBehaviorState>().unwrap()
    }

    /// A graced arrival holds its patrol with the player in plain range,
    /// then engages when the timer runs out. Delivery guard in the same
    /// test family: the ungraced twin engages on the first tick.
    #[test]
    fn a_graced_arrival_holds_its_routine_then_engages() {
        let (mut app, ship) = grace_app(Some(2.0));
        for _ in 0..4 {
            app.update();
        }
        assert_eq!(
            state(&mut app, ship),
            AIBehaviorState::Patrol,
            "inside the grace the entrance holds (player at 300u, engage \
             range 800u - only the grace explains the restraint)"
        );
        for _ in 0..12 {
            app.update();
        }
        assert_eq!(
            state(&mut app, ship),
            AIBehaviorState::Engage,
            "the grace ran out: the arrival goes hot"
        );

        let (mut app, ship) = grace_app(None);
        app.update();
        app.update();
        assert_eq!(
            state(&mut app, ship),
            AIBehaviorState::Engage,
            "delivery guard: without a grace the same shape engages at once"
        );
    }

    /// Shot during the entrance: the ship engages NOW and the grace never
    /// holds again (the timer is pinned to finished).
    #[test]
    fn damage_ends_the_grace_immediately_and_permanently() {
        let (mut app, ship) = grace_app(Some(30.0));
        app.update();
        assert_eq!(state(&mut app, ship), AIBehaviorState::Patrol);

        app.world_mut()
            .entity_mut(ship)
            .get_mut::<AIThreat>()
            .unwrap()
            .record(None);
        app.update();
        assert_eq!(
            state(&mut app, ship),
            AIBehaviorState::Engage,
            "a shot telegraphed ship goes hot immediately"
        );
        assert!(
            app.world()
                .entity(ship)
                .get::<AIEngageGrace>()
                .unwrap()
                .timer
                .is_finished(),
            "the grace is pinned finished - it can never re-hold"
        );
    }

    /// Point defense ignores the grace: a graced ship still swats inbound
    /// ordnance (the PD path bypasses behavior states by design).
    #[test]
    fn point_defense_fires_through_the_grace() {
        use avian3d::collider_tree::ColliderTrees;
        use bevy::ecs::system::RunSystemOnce;

        let mut world = World::new();
        world.init_resource::<ColliderTrees>();
        let torpedo = world
            .spawn((Transform::from_translation(Vec3::new(0.0, 0.0, -100.0)),))
            .id();
        let ship = world
            .spawn((
                AISpaceshipMarker,
                AIBehaviorState::Patrol,
                AIEngageGrace::new(30.0),
                AITarget(None),
                AIPointDefenseTarget(Some(torpedo)),
                Transform::default(),
                LinearVelocity(Vec3::ZERO),
            ))
            .id();
        let muzzle = world
            .spawn((TurretSectionBarrelMuzzleMarker, GlobalTransform::IDENTITY))
            .id();
        let turret = world
            .spawn((
                TurretSectionMarker,
                TurretSectionTargetInput(None),
                TurretSectionTargetVelocity(Vec3::ZERO),
                TurretSectionAimPoint(None),
                TurretSectionConfigHelper(TurretSectionConfig::default()),
                TurretSectionInput(false),
                TurretSectionMuzzleEntity(muzzle),
                ChildOf(ship),
            ))
            .id();

        world.run_system_once(on_projectile_input).unwrap();
        assert!(
            **world.entity(turret).get::<TurretSectionInput>().unwrap(),
            "a graced ship still point-defends (defending bypasses state)"
        );
    }
}
