use avian3d::prelude::*;
use bevy::prelude::*;
use nova_events::prelude::*;

use crate::prelude::*;

pub mod prelude {
    pub use super::{
        AIBehaviorState, AIEngageGrace, AIEvade, AIFireCadence, AILeash, AIOrbitDirective,
        AIPatrolRoute, AIPointDefenseTarget, AISpaceshipMarker, AITarget, AIThreat, AITorpedoBay,
        SpaceshipAIInputPlugin,
    };
}

/// Arrival grace (task 20260717-163042): a telegraphed ship holds its
/// PASSIVE routine (patrol/orbit/idle) and refuses the engage pull until
/// this timer runs out - enemies ARRIVE instead of appearing hot. Being
/// shot ends the grace immediately and PERMANENTLY (the ticking system
/// pins the timer to finished), mirroring the leash's damage override.
/// Point defense is untouched: a graced ship still swats inbound
/// ordnance (the PD path deliberately bypasses behavior states).
/// Authored via `AIControllerConfig::engage_delay`.
#[derive(Component, Debug, Clone, Reflect)]
#[reflect(Component)]
pub struct AIEngageGrace {
    /// Time left before the ship may engage.
    pub timer: Timer,
}

impl AIEngageGrace {
    pub fn new(seconds: f32) -> Self {
        Self {
            timer: Timer::from_seconds(seconds, TimerMode::Once),
        }
    }
}

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

        // Threat sensing is an observer, not a system: HealthApplyDamage is
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
                // Commit-on-launch before the trigger write: the frame
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

/// The entity this AI ship currently fights - what every AI behavior system
/// aims, chases and shoots at. Written by [`update_ai_target`] from the
/// relation model (task 20260709-225727); `None` means nothing hostile in
/// acquisition range, which [`update_behavior_state`] turns into `Idle`.
#[derive(Component, Debug, Clone, Copy, Default, PartialEq, Eq, Deref, DerefMut, Reflect)]
#[reflect(Component)]
pub struct AITarget(pub Option<Entity>);

/// Mirror the AI's engagement onto the shared combat components (deliberate-
/// radar AI parity, task 20260713-082337): `CombatLock` = the point-defense
/// override else the primary target, refreshed every frame (the AI's own
/// acquisition hygiene replaces the player's validity/decay upkeep), and the
/// stance is raised while engaged - so the shared section-side weapons-safety
/// gate never silences a fighting AI, and the AI's guns go SAFE the moment it
/// disengages. Instant acquisition for AI is the accepted spec (the human
/// gesture is the deliberate part, not the machine's).
fn mirror_ai_combat_state(
    mut commands: Commands,
    mut q_ships: Query<
        (
            Entity,
            &AITarget,
            &AIPointDefenseTarget,
            Option<&mut CombatLock>,
            Option<&mut WeaponsRaised>,
        ),
        With<AISpaceshipMarker>,
    >,
) {
    for (ship, target, pd_target, lock, raised) in &mut q_ships {
        let engaged = pd_target.0.or(target.0);
        let managed = lock.is_some();
        match lock {
            Some(mut lock) => {
                if lock.0 != engaged {
                    lock.0 = engaged;
                }
            }
            None => {
                commands.entity(ship).insert(CombatLock(engaged));
            }
        }
        let is_raised = engaged.is_some();
        match raised {
            Some(mut raised) => {
                raised.set_if_neq(WeaponsRaised(is_raised));
            }
            None => {
                commands.entity(ship).insert(WeaponsRaised(is_raised));
            }
        }
        // WeaponsHot itself is derived by the shared safety system; give the
        // ship the component so it becomes a MANAGED ship.
        if !managed {
            commands.entity(ship).insert(WeaponsHot::default());
        }
    }
}

/// What kind of body a target candidate is. Priority TIER, not a score
/// tweak: hostile ships always beat hostile torpedoes (the urgency flip for
/// an incoming torpedo is the point-defense task, 20260709-225733).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum AITargetKind {
    Ship,
    Torpedo,
}

/// Acquisition range (m) of AI target selection. Deliberately shorter than
/// the player's TARGETING_MAX_RANGE (20 km): the player's lock doubles as a
/// long-range designator for GOTO legs and torpedo launches, while AI
/// sensors only need to find things worth fighting.
const AI_TARGET_MAX_RANGE: f32 = 2000.0;
/// Switch hysteresis: the current target's distance is discounted by this
/// factor, so a rival has to be meaningfully closer (not a frame-noise
/// sliver) to steal the pick.
const AI_TARGET_HYSTERESIS_DISCOUNT: f32 = 0.8;

/// Choose the best target from `candidates`: highest priority tier first
/// ([`AITargetKind`] order), nearest within the tier, with the current
/// target's distance discounted by [`AI_TARGET_HYSTERESIS_DISCOUNT`] so the
/// pick does not flip-flop between two comparably distant hostiles, and the
/// ship that recently damaged me discounted by
/// [`AI_THREAT_ATTACKER_DISCOUNT`] so whoever is shooting me steals the
/// pick from comparably distant bystanders (the discounts stack). Out of
/// [`AI_TARGET_MAX_RANGE`] (or with no candidates) the pick is `None`.
/// Pure for unit testing.
fn pick_ai_target(
    own_anchor: Vec3,
    current: Option<Entity>,
    attacker: Option<Entity>,
    candidates: impl Iterator<Item = (Entity, Vec3, AITargetKind)>,
) -> Option<Entity> {
    candidates
        .filter_map(|(entity, position, kind)| {
            let mut distance = own_anchor.distance(position);
            if distance > AI_TARGET_MAX_RANGE || distance <= f32::EPSILON {
                return None;
            }
            if current == Some(entity) {
                distance *= AI_TARGET_HYSTERESIS_DISCOUNT;
            }
            if attacker == Some(entity) {
                distance *= AI_THREAT_ATTACKER_DISCOUNT;
            }
            Some((entity, kind, distance))
        })
        .min_by(|(_, kind_a, dist_a), (_, kind_b, dist_b)| {
            kind_a.cmp(kind_b).then(dist_a.total_cmp(dist_b))
        })
        .map(|(entity, _, _)| entity)
}

/// Acquire each AI ship's [`AITarget`] over the relation model: every
/// hostile ship root or committed hostile torpedo inside acquisition range
/// is a candidate; [`pick_ai_target`] scores them. Runs first in the AI
/// chain - acquisition drives engagement, so a ship in `Idle` still scans.
#[allow(clippy::type_complexity)]
fn update_ai_target(
    q_candidates: Query<(
        Entity,
        &Transform,
        Option<&ComputedCenterOfMass>,
        Option<&Allegiance>,
        Has<SpaceshipRootMarker>,
        Option<&TorpedoProjectileMarker>,
        Option<&TorpedoTargetChosen>,
    )>,
    mut q_spaceship: Query<
        (
            Entity,
            &Transform,
            Option<&ComputedCenterOfMass>,
            &Allegiance,
            &AIThreat,
            &mut AITarget,
        ),
        (With<SpaceshipRootMarker>, With<AISpaceshipMarker>),
    >,
) {
    for (ship, transform, com, own_allegiance, threat, mut target) in &mut q_spaceship {
        let own_anchor = live_structure_anchor(transform, com);
        let candidates = q_candidates.iter().filter_map(
            |(entity, c_transform, c_com, allegiance, is_ship, is_torpedo, committed)| {
                if entity == ship {
                    return None;
                }
                // Hostility comes from the relation model: the player's ship
                // and projectiles are hostile to an Enemy-aligned AI, other
                // AI ships and neutral bodies (asteroids) are not.
                if relation(Some(own_allegiance), allegiance) != Relation::Hostile {
                    return None;
                }
                let kind = if is_ship {
                    AITargetKind::Ship
                } else if is_torpedo.is_some() {
                    // Only committed torpedoes are targets, matching the
                    // player targeting rule: a just-launched torpedo has not
                    // decided what it is yet.
                    committed?;
                    AITargetKind::Torpedo
                } else {
                    return None;
                };
                Some((entity, live_structure_anchor(c_transform, c_com), kind))
            },
        );

        let next = pick_ai_target(own_anchor, **target, threat.recent_attacker(), candidates);
        // Change-detection hygiene: only write on a real change. A dead or
        // out-of-range target clears here (the pick simply no longer finds
        // it), so consumers never chase a stale entity.
        if **target != next {
            **target = next;
        }
    }
}

/// The inbound torpedo this ship's guns are currently defending against.
/// When set, it OVERRIDES the primary [`AITarget`] for turret aim and fire -
/// the PDC role is the turrets' main purpose (user decision, 20260710) -
/// while flight keeps chasing the primary target. Written by
/// [`update_point_defense_target`].
#[derive(Component, Debug, Clone, Copy, Default, PartialEq, Eq, Deref, DerefMut, Reflect)]
#[reflect(Component)]
pub struct AIPointDefenseTarget(pub Option<Entity>);

/// Range (m) inside which an inbound hostile torpedo pulls the guns off the
/// primary target. Kept inside the default turret's effective range
/// (muzzle_speed * lifetime * margin = 450 m) so a defending turret can
/// actually reach what it defends against.
const AI_POINT_DEFENSE_RANGE: f32 = 400.0;

/// Choose the torpedo to defend against: ones hunting THIS ship outrank
/// ones hunting someone else (a tier, like the ship/torpedo target tiers),
/// nearest wins within a tier, nothing outside
/// [`AI_POINT_DEFENSE_RANGE`]. Pure for unit testing.
fn pick_point_defense_target(
    own_anchor: Vec3,
    candidates: impl Iterator<Item = (Entity, Vec3, bool)>,
) -> Option<Entity> {
    candidates
        .filter_map(|(entity, position, targeting_me)| {
            let distance = own_anchor.distance(position);
            if distance > AI_POINT_DEFENSE_RANGE || distance <= f32::EPSILON {
                return None;
            }
            // false < true, so invert: hunting-me sorts first.
            Some((entity, !targeting_me, distance))
        })
        .min_by(|(_, me_a, dist_a), (_, me_b, dist_b)| {
            me_a.cmp(me_b).then(dist_a.total_cmp(dist_b))
        })
        .map(|(entity, _, _)| entity)
}

/// Acquire each AI ship's [`AIPointDefenseTarget`]: hostile committed
/// torpedoes inside point-defense range, preferring ones whose
/// [`TorpedoTargetEntity`] is this ship. Runs right after primary
/// acquisition; the turret systems consume the override the same frame.
#[allow(clippy::type_complexity)]
fn update_point_defense_target(
    q_torpedoes: Query<
        (
            Entity,
            &Transform,
            Option<&Allegiance>,
            Option<&TorpedoTargetEntity>,
        ),
        (With<TorpedoProjectileMarker>, With<TorpedoTargetChosen>),
    >,
    mut q_spaceship: Query<
        (
            Entity,
            &Transform,
            Option<&ComputedCenterOfMass>,
            &Allegiance,
            &mut AIPointDefenseTarget,
        ),
        (With<SpaceshipRootMarker>, With<AISpaceshipMarker>),
    >,
) {
    for (ship, transform, com, own_allegiance, mut pd_target) in &mut q_spaceship {
        let own_anchor = live_structure_anchor(transform, com);
        let candidates =
            q_torpedoes
                .iter()
                .filter_map(|(entity, t_transform, allegiance, torpedo_target)| {
                    if relation(Some(own_allegiance), allegiance) != Relation::Hostile {
                        return None;
                    }
                    let targeting_me = torpedo_target.map(|t| **t) == Some(ship);
                    Some((entity, t_transform.translation, targeting_me))
                });

        let next = pick_point_defense_target(own_anchor, candidates);
        // Change-detection hygiene, and stale-entity safety as with AITarget.
        if **pd_target != next {
            **pd_target = next;
        }
    }
}

/// How long (s) a hostile hit stays "recent" for the threat model: within
/// this window the ship counts as under fire and the attacker biases target
/// selection.
const AI_THREAT_DAMAGE_MEMORY_SECS: f32 = 3.0;
/// Range (m) inside which a hostile holding its nose on me counts as a
/// threat even before a shot lands. Kept near the guns' effective range
/// (450 m): a nose on me further out cannot hurt me yet.
const AI_THREAT_AIM_RANGE: f32 = 500.0;
/// Aim cone (cos) for the aiming-at-me signal: the hostile's hull forward
/// against the bearing to my anchor. A cheap proxy - turrets can aim off
/// the hull axis - accepted per the spike; true incoming-projectile
/// detection is the follow-up if evasion feels blind.
const AI_THREAT_AIM_COS: f32 = 0.95;
/// How long (s) one evade cycle lasts before decaying back to Engage.
/// Three jink legs at [`AI_JINK_INTERVAL_SECS`]. Playtest note: Evade has
/// no speed budget (the jink bypasses the standoff envelope's brake
/// regime), so back-to-back cycles can build speed that Engage re-entry
/// then brakes off; if evasion reads as careening, cap the cycle count or
/// shorten this.
const AI_EVADE_SECS: f32 = 3.6;
/// Refractory period (s) after an evade cycle before a threat can trigger
/// the next one. Without it a hostile that keeps its nose on the ship would
/// re-trigger Evade every frame and the standoff orbit would never be seen;
/// with it a fight reads as jink bursts with engage windows between them.
const AI_EVADE_COOLDOWN_SECS: f32 = 1.5;
/// Length (s) of one jink leg: long enough for the hull to swing onto the
/// leg's heading (torque-budget slew) and burn, short enough to read as
/// jinking. Playtest knob, paired with AI_EVADE_SECS.
const AI_JINK_INTERVAL_SECS: f32 = 1.2;
/// Thrust gate (dot) while evading: looser than [`AI_THRUST_ALIGNMENT`] so
/// lateral bursts fire while the hull is still swinging onto the jink leg -
/// waiting for a tight alignment would spend most of each leg coasting.
const AI_EVADE_THRUST_ALIGNMENT: f32 = 0.75;
/// Distance discount for the ship that recently damaged me: whoever is
/// shooting me steals the pick from comparably distant hostiles
/// (recently-damaged-me threat scoring, deferred from 20260709-225727).
const AI_THREAT_ATTACKER_DISCOUNT: f32 = 0.5;

/// The ship's under-fire memory: how recently a hostile hit landed and who
/// fired it. Written by [`on_damage_track_threat`], ticked by
/// [`update_behavior_state`]; drives the Engage -> Evade transition and the
/// attacker bias in [`pick_ai_target`]. Required by [`AISpaceshipMarker`].
#[derive(Component, Debug, Clone, Reflect)]
#[reflect(Component)]
pub struct AIThreat {
    /// Time left in the recent-damage memory; finished = not under fire.
    damage_memory: Timer,
    /// The ship root behind the remembered damage (a hit's source resolved
    /// through [`ProjectileOwner`]). May be despawned by read time; the
    /// picker simply no longer finds it, while the memory still evades.
    attacker: Option<Entity>,
}

impl Default for AIThreat {
    fn default() -> Self {
        // Starts expired: a freshly spawned ship has not been shot yet.
        let mut damage_memory = Timer::from_seconds(AI_THREAT_DAMAGE_MEMORY_SECS, TimerMode::Once);
        damage_memory.tick(damage_memory.duration());
        Self {
            damage_memory,
            attacker: None,
        }
    }
}

impl AIThreat {
    /// Remember a hostile hit: restart the memory window and note the
    /// attacker. An unattributed hit (no resolvable owner) keeps the
    /// previous attacker - the shooter most likely has not changed.
    fn record(&mut self, attacker: Option<Entity>) {
        self.damage_memory.reset();
        self.attacker = attacker.or(self.attacker);
    }

    /// Whether a hostile hit landed within the memory window.
    fn recently_damaged(&self) -> bool {
        !self.damage_memory.is_finished()
    }

    /// The remembered attacker, while the memory window is open.
    fn recent_attacker(&self) -> Option<Entity> {
        self.recently_damaged().then_some(self.attacker).flatten()
    }
}

/// The evade cycle's clocks: how long the current cycle has left, the
/// refractory period before the next one, and the jink-leg cadence within a
/// cycle. Managed by [`update_behavior_state`]; the rotation and thrust
/// systems read [`Self::leg`] to fly the current jink. Required by
/// [`AISpaceshipMarker`].
#[derive(Component, Debug, Clone, Reflect)]
#[reflect(Component)]
pub struct AIEvade {
    /// Time left in the current evade cycle. Reset on entering Evade;
    /// ticks only while evading; expiry decays the state back to Engage.
    duration: Timer,
    /// Refractory period after an evade cycle. Reset on leaving Evade;
    /// starts elapsed so a fresh ship's first threat evades immediately.
    cooldown: Timer,
    /// Cadence of the jink pattern: each completion turns onto the next leg.
    jink: Timer,
    /// The jink pattern leg currently being flown (see
    /// [`ai_evade_direction`]). Advances monotonically, wrapping.
    leg: u32,
}

impl Default for AIEvade {
    fn default() -> Self {
        let mut cooldown = Timer::from_seconds(AI_EVADE_COOLDOWN_SECS, TimerMode::Once);
        cooldown.tick(cooldown.duration());
        Self {
            duration: Timer::from_seconds(AI_EVADE_SECS, TimerMode::Once),
            cooldown,
            jink: Timer::from_seconds(AI_JINK_INTERVAL_SECS, TimerMode::Repeating),
            leg: 0,
        }
    }
}

/// Resolve a damage source (the hitting collider bcs puts in
/// `HealthApplyDamage.source`) to the attacking ship root and the
/// allegiance governing the hit, walking `ChildOf` ancestors: a turret
/// bullet carries [`ProjectileOwner`] on the collider itself, a torpedo
/// warhead on its projectile root, a detonation blast on the blast entity.
/// With no owner anywhere (a ram), the attacker is the source's own ship
/// root, if it has one.
fn resolve_damage_attacker(
    source: Entity,
    q_owner: &Query<&ProjectileOwner>,
    q_allegiance: &Query<&Allegiance>,
    q_parent: &Query<&ChildOf>,
    q_ship_root: &Query<(), With<SpaceshipRootMarker>>,
) -> (Option<Entity>, Option<Allegiance>) {
    let mut allegiance = None;
    let mut entity = source;
    loop {
        if let Ok(&ProjectileOwner(owner)) = q_owner.get(entity) {
            // The projectile copies the shooter's allegiance at launch, so
            // the hit stays classifiable even if the owner died mid-flight.
            let allegiance = allegiance.or_else(|| q_allegiance.get(entity).ok().copied());
            return (Some(owner), allegiance);
        }
        if allegiance.is_none() {
            allegiance = q_allegiance.get(entity).ok().copied();
        }
        let Ok(&ChildOf(parent)) = q_parent.get(entity) else {
            // Topmost ancestor, no owner anywhere: a direct body-to-body
            // hit. A ship root is its own attacker; anything else
            // (asteroid, debris) has nobody to blame.
            let attacker = q_ship_root.get(entity).is_ok().then_some(entity);
            return (attacker, allegiance);
        };
        entity = parent;
    }
}

/// Record hostile hits into the damaged ship's [`AIThreat`].
///
/// `HealthApplyDamage` propagates from the hit section up through `ChildOf`
/// to the ship root (bcs), so this fires once the event reaches an entity
/// carrying `AIThreat` - the AI root. Only hits whose resolved allegiance is
/// hostile count: the ship's own torpedo blast catching it (blast damage
/// deliberately affects the owner) must not spook it into evading itself.
fn on_damage_track_threat(
    damage: On<HealthApplyDamage>,
    mut q_ship: Query<(&Allegiance, &mut AIThreat), With<AISpaceshipMarker>>,
    q_owner: Query<&ProjectileOwner>,
    q_allegiance: Query<&Allegiance>,
    q_parent: Query<&ChildOf>,
    q_ship_root: Query<(), With<SpaceshipRootMarker>>,
) {
    // A zero amount is a hit on a corpse (bcs zeroes absorbed damage), not
    // fire worth reacting to.
    if damage.amount <= 0.0 {
        return;
    }
    let Ok((own_allegiance, mut threat)) = q_ship.get_mut(damage.entity) else {
        return;
    };
    let Some(source) = damage.source else {
        return;
    };
    let (attacker, attacker_allegiance) =
        resolve_damage_attacker(source, &q_owner, &q_allegiance, &q_parent, &q_ship_root);
    if relation(Some(own_allegiance), attacker_allegiance.as_ref()) != Relation::Hostile {
        return;
    }
    threat.record(attacker);
}

/// What an AI ship is currently doing - the state skeleton of the AI combat
/// arc (docs/spikes/20260709-225508-ai-combat-behaviors.md). One state per
/// ship root, driven by [`update_behavior_state`]; every AI system gates its
/// behavior on it.
///
/// `Engage`, `Patrol`, `Idle` and `Evade` have real behavior today.
/// `Retreat` exists so its task slots into a stable enum instead of
/// reshaping it: low-integrity disengage, task 20260709-225734 (stubs to
/// `Engage`).
#[derive(Component, Debug, Clone, Copy, Default, PartialEq, Eq, Reflect)]
#[reflect(Component)]
pub enum AIBehaviorState {
    /// Station-keeping: kill drift, hold position loosely, no fire.
    Idle,
    /// Fly the ship's [`AIPatrolRoute`] waypoint loop through the GOTO
    /// autopilot (20260709-225730); no fire.
    Patrol,
    /// Circle the [`AIOrbitDirective`]'s gravity well through the ORBIT
    /// autopilot (20260711-212521); no fire. Passive like Patrol/Idle:
    /// combat pulls the ship out and calm returns it.
    Orbit,
    /// Chase and shoot the hostile - today's whole AI, and the default so
    /// an AI ship dropped into a fight behaves exactly as before the state
    /// machine existed.
    #[default]
    Engage,
    /// Under-fire jinking (20260709-225731): timed maneuvers off the
    /// pursuit vector while the guns keep fighting, decaying back to
    /// `Engage`.
    Evade,
    /// Low-integrity disengage (20260709-225734); stubs to `Engage` until
    /// then.
    Retreat,
}

impl AIBehaviorState {
    /// Whether this state runs the engage-style chase/aim/fire pipeline.
    /// `Evade` fights it too - the jink only swaps the flight direction,
    /// the guns stay on target. `Retreat` deliberately stubs to Engage
    /// behavior until its task lands (see the variant docs).
    fn engages(&self) -> bool {
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
/// ([`next_behavior_state`]). Spawn-configured (scenario/editor); flown by
/// [`update_passive_flight`] through the real GOTO autopilot, leg by leg.
#[derive(Component, Debug, Clone, PartialEq, Reflect)]
#[reflect(Component)]
pub struct AIPatrolRoute {
    /// The loop's waypoints, world coordinates. Legs shorter than the
    /// arrival radius (arrival_standoff + [`AI_WAYPOINT_SLACK`]) are all
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
    fn current_waypoint(&self) -> Option<Vec3> {
        Some(self.waypoints[self.wrapped_current()?])
    }

    /// Turn onto the next leg, wrapping - the route is a loop. Also snaps
    /// an out-of-range `current` back into range.
    fn advance(&mut self) {
        if let Some(current) = self.wrapped_current() {
            self.current = (current + 1) % self.waypoints.len();
        }
    }
}

/// Directs an AI ship to orbit a gravity well while nothing hostile is close
/// enough to fight. Present = the no-hostile fallback state becomes `Orbit`,
/// taking precedence over `Patrol` ([`next_behavior_state`]). The well is
/// named by its scenario [`EntityId`]; [`update_passive_flight`] resolves it
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
/// before it may engage again. The asymmetry is hysteresis: combat breaks
/// off strictly beyond the radius, but re-engagement needs the ship well
/// back inside - without the band, a hostile parked at the boundary makes
/// the ship ping-pong Engage/Patrol every crossing (review R1.2).
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

/// Territorial tether (task 20260712-125342, playtest round 3): a leashed
/// ship abandons combat and returns to its passive routine whenever it
/// strays beyond `radius` of `center` - the shakedown scavenger stays at
/// the debris field instead of chasing across the map. Being under fire
/// overrides the leash (a ship dragged out and shot may defend itself);
/// the tether reasserts once the damage memory fades.
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
/// none acquired every state falls back to its passive routine (`Orbit`
/// with an orbit directive, else `Patrol` with a route, else `Idle`) - and
/// a hostile inside [`AI_ENGAGE_RANGE`] pulls the passive states into
/// `Engage`. One merely acquired further out
/// does not abort the routine (detection range, task 20260709-225730) -
/// unless it is shooting: a recent hostile hit interrupts the routine at
/// any acquired distance. Under threat, `Engage` breaks into `Evade` (gated
/// by the refractory cooldown), which decays back to `Engage` when its
/// cycle expires. Pure for unit testing.
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
    // The arrival grace: a telegraphed ship holds its routine until the
    // timer runs out. Damage overrides here too - and the ticking system
    // makes that override permanent by pinning the timer (a shot ship
    // never calms back into its entrance). UNCONDITIONAL on the current
    // state on purpose: `AIBehaviorState`'s default is Engage, so every
    // graced scenario spawn takes THIS return on its first behavior tick
    // to land on its routine - restricting it to passive states would
    // silently break every real telegraphed arrival (review R1.1's
    // mutation probe).
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
fn update_behavior_state(
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
                    // Shot during the entrance: the courtesy is over for
                    // good (a finished timer never holds again). Tick to
                    // the end rather than set_elapsed - only tick()
                    // updates the finished flag (Bevy Timer semantics).
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

/// Arrival slack (m) on top of the autopilot's arrival standoff for calling
/// a patrol waypoint reached and turning onto the next leg. Turning early,
/// while the arrival curve is still braking, keeps the loop flowing instead
/// of stop-and-go at every corner.
const AI_WAYPOINT_SLACK: f32 = 25.0;
/// Drift speed (u/s) above which a station-keeping ship burns to rest.
/// Holding position "loosely" means arresting drift, not chasing crumbs:
/// kept well above the autopilot's stop_speed_epsilon so a completed STOP
/// actually satisfies it and the helm rests between corrections.
const AI_IDLE_DRIFT_SPEED: f32 = 1.0;

/// Fly the passive states through the real autopilot (flight.rs) instead of
/// a parallel steering path: `Patrol` keeps a GOTO engaged toward the
/// current [`AIPatrolRoute`] waypoint and turns onto the next leg on
/// arrival; `Orbit` keeps an ORBIT engaged on its directive's well (the
/// autopilot plans its own insertion on the first tick and never
/// self-completes, so one engage holds the ring); `Idle` engages a STOP
/// burn while drifting faster than
/// [`AI_IDLE_DRIFT_SPEED`] (station keeping - the drift is arrested, not
/// rewound). The engaging states drop the autopilot: the AI's own actuator
/// systems own the helm and engines in combat, and a leftover passive
/// maneuver would fight them. Runs right after the state transition so a
/// flip takes effect the same frame.
///
/// Idle and Orbit let an already-engaged maneuver finish before taking
/// over, so a ship whose route is removed mid-leg (not a supported flow
/// today) flies out its stale GOTO once before settling into its routine.
fn update_passive_flight(
    settings: Res<FlightSettings>,
    mut commands: Commands,
    mut q_spaceship: Query<
        (
            Entity,
            &Transform,
            &LinearVelocity,
            &AIBehaviorState,
            Option<&mut AIPatrolRoute>,
            Option<&AIOrbitDirective>,
            Option<&Autopilot>,
        ),
        (With<SpaceshipRootMarker>, With<AISpaceshipMarker>),
    >,
    q_wells: Query<(Entity, &EntityId), With<GravityWell>>,
) {
    for (ship, transform, velocity, state, route, orbit, autopilot) in &mut q_spaceship {
        let has_autopilot = autopilot.is_some();
        match *state {
            AIBehaviorState::Patrol => {
                // Patrol without a route cannot happen through the
                // transition function (the fallback picks Patrol only with
                // one), but a hand-set state should idle, not panic.
                let Some(mut route) = route else {
                    continue;
                };
                let Some(waypoint) = route.current_waypoint() else {
                    continue;
                };
                // Arrived: turn onto the next leg. The check runs on the
                // ship's position, not on autopilot completion, so a ship
                // shoved onto its waypoint (or re-entering Patrol on top of
                // one) advances too.
                let arrive_radius = settings.arrival_standoff + AI_WAYPOINT_SLACK;
                if transform.translation.distance(waypoint) <= arrive_radius {
                    route.advance();
                }
                let Some(goal) = route.current_waypoint() else {
                    continue;
                };
                // On station (a single-waypoint route, parked at it with the
                // drift killed) there is nothing left to fly; re-engaging
                // would churn engage/complete every frame.
                let on_station = transform.translation.distance(goal) <= arrive_radius
                    && velocity.length() <= AI_IDLE_DRIFT_SPEED;
                // (Re)engage when the leg changed or nothing is engaged; a
                // maneuver already flying the current leg is left alone.
                let leg_changed = goal != waypoint;
                if (leg_changed || !has_autopilot) && !on_station {
                    commands
                        .entity(ship)
                        .insert(Autopilot::engage(AutopilotAction::GotoPos {
                            position: goal,
                        }));
                }
            }
            AIBehaviorState::Orbit => {
                // Orbit without a directive cannot happen through the
                // transition function; a hand-set state drifts, not panics.
                let Some(directive) = orbit else {
                    continue;
                };
                // A non-ORBIT maneuver (e.g. a stale patrol GOTO after a
                // hot-inserted directive) flies out first, same as Idle's
                // let-it-finish policy - and skips the well scan entirely.
                let engaged_well = match autopilot.map(|autopilot| autopilot.action) {
                    Some(AutopilotAction::Orbit { well, .. }) => Some(well),
                    Some(_) => continue,
                    None => None,
                };
                // The ORBIT autopilot self-plans on its first engaged tick
                // and disengages itself if the well dies, so a bare engage
                // is enough; re-resolve and retry every calm frame (also
                // covers a well that spawns later than the ship).
                let Some(well) = q_wells
                    .iter()
                    .find(|(_, id)| ***id == *directive.well)
                    .map(|(entity, _)| entity)
                else {
                    debug_once!(
                        "update_passive_flight: orbit directive well '{}' matches no live \
                         GravityWell entity; ship {ship:?} drifts until it appears",
                        *directive.well
                    );
                    continue;
                };
                // (Re)engage when nothing is engaged or the directive was
                // retargeted to another well - the ORBIT analogue of the
                // patrol arm's leg_changed (review R1.2); an autopilot
                // already circling the right well is left alone.
                if engaged_well != Some(well) {
                    commands
                        .entity(ship)
                        .insert(Autopilot::engage(AutopilotAction::Orbit {
                            well,
                            plan: None,
                        }));
                }
            }
            AIBehaviorState::Idle => {
                if !has_autopilot && velocity.length() > AI_IDLE_DRIFT_SPEED {
                    commands
                        .entity(ship)
                        .insert(Autopilot::engage(AutopilotAction::Stop));
                }
            }
            // Combat: the AI actuator systems own the ship.
            _ => {
                if has_autopilot {
                    commands.entity(ship).remove::<Autopilot>();
                }
            }
        }
    }
}

// AI "brain" tuning constants. The AI flies a standoff envelope around its
// target: approach when far, orbit at the preferred range, extend when too
// close, and brake when it overshoots.
/// Target speed per unit of RANGE ERROR (distance outside the standoff
/// band), so the ship slows as it nears the band instead of the target.
const AI_CHASE_SPEED_GAIN: f32 = 0.2;
/// Orbit speed floor: inside the band the ship keeps circling at least this
/// fast, so it stays a moving target instead of a parked one.
const AI_ORBIT_SPEED: f32 = 8.0;
const AI_MAX_CHASE_SPEED: f32 = 20.0;
/// Preferred engagement range (m): inside the turrets' effective range
/// (default 450 m, see AI_FIRE_RANGE_FACTOR) with room to spare.
const AI_STANDOFF_RANGE: f32 = 250.0;
/// Half-width (m) of the band around the preferred range where the orbit
/// term dominates the radial term.
const AI_STANDOFF_BAND: f32 = 60.0;
/// The ship brakes once its speed exceeds the target chase speed by this margin.
const AI_BRAKE_SPEED_MARGIN: f32 = 1.0;
/// Only thrust when the ship's forward vector aligns with the desired direction at least
/// this much (dot product, 1.0 == perfectly aligned).
const AI_THRUST_ALIGNMENT: f32 = 0.95;
/// Only fire when the muzzle aligns with the aim point at least this much.
const AI_FIRE_ALIGNMENT: f32 = 0.95;
/// Fraction of a turret's maximum bullet travel (muzzle_speed * lifetime)
/// inside which the AI considers a shot worth taking: a margin below 1.0 so
/// bullets arrive with the target still catchable, not at their despawn
/// range.
const AI_FIRE_RANGE_FACTOR: f32 = 0.9;
/// Burst cadence (s): guns fire for the window, then hold, cyclically.
const AI_BURST_FIRE_SECS: f32 = 1.5;
const AI_BURST_HOLD_SECS: f32 = 0.8;

/// The direction an AI ship should face: the standoff envelope around its
/// target. Far outside the band it approaches; inside the band it orbits
/// (tangential to the line of sight, stable handedness); too close it
/// extends away - pure pursuit is what parked the old AI at zero range in
/// a turret duel, or rammed. Overshooting its speed budget it brakes
/// (opposite its velocity), as before. Falls back to facing the target if
/// the computed direction degenerates to zero. Pure for unit testing.
fn ai_desired_direction(to_target: Vec3, velocity: Vec3) -> Vec3 {
    let distance = to_target.length();
    if distance <= f32::EPSILON {
        return Vec3::ZERO;
    }
    let los = to_target / distance;

    // Positive = too far (approach), negative = too close (extend).
    let range_error = distance - AI_STANDOFF_RANGE;
    // Orbit tangent with a stable handedness; the X fallback covers a
    // dead-polar line of sight. Global handedness (every ship circles the
    // same way) is fine for one archetype - see task Notes.
    let tangent = los
        .cross(Vec3::Y)
        .try_normalize()
        .unwrap_or_else(|| los.cross(Vec3::X).normalize());
    // Radial weight ramps with how far outside the band the ship is; inside
    // the band the orbit term dominates.
    let radial_weight = (range_error.abs() / AI_STANDOFF_BAND).clamp(0.0, 1.0);
    let radial = los * range_error.signum();
    let desired = radial * radial_weight + tangent * (1.0 - radial_weight);

    // Speed budget scales with the range error, never below orbit speed;
    // overshooting it brakes, exactly as the old chase regime did.
    let target_speed =
        (range_error.abs() * AI_CHASE_SPEED_GAIN).clamp(AI_ORBIT_SPEED, AI_MAX_CHASE_SPEED);
    let too_fast = velocity.length() > target_speed + AI_BRAKE_SPEED_MARGIN;

    let desired = if too_fast {
        // Brake: point opposite the current velocity.
        -velocity.normalize_or_zero()
    } else {
        desired.normalize_or_zero()
    };

    if desired.length_squared() == 0.0 {
        to_target.normalize_or_zero()
    } else {
        desired
    }
}

/// The direction an evading ship flies on jink pattern leg `leg`: a box
/// weave off the pursuit vector. Each leg is mostly lateral (the four
/// tangent quadrants around the line of sight in turn) with a small
/// alternating along-LOS bias, so consecutive legs swing the heading hard
/// off the pursuit vector AND vary the closure rate - the "timed jink"
/// the task asks for. Deterministic by design: unit-testable, and one
/// archetype does not need unpredictability yet (playtest knob). Falls
/// back to zero on a degenerate line of sight. Pure for unit testing.
fn ai_evade_direction(to_target: Vec3, leg: u32) -> Vec3 {
    let Some(los) = to_target.try_normalize() else {
        return Vec3::ZERO;
    };
    // The same stable tangent basis as the standoff orbit, with the X
    // fallback covering a dead-polar line of sight.
    let tangent = los
        .cross(Vec3::Y)
        .try_normalize()
        .unwrap_or_else(|| los.cross(Vec3::X).normalize());
    // Perpendicular to both, unit length (los and tangent are orthonormal).
    let bitangent = los.cross(tangent);
    let lateral = match leg % 4 {
        0 => tangent,
        1 => bitangent,
        2 => -tangent,
        _ => -bitangent,
    };
    let along = if leg.is_multiple_of(2) { 0.25 } else { -0.25 };
    (lateral + los * along).normalize()
}

/// The live-structure anchor of a target entity, or `None` without one (or
/// when it despawned this frame). The shared aim/chase point of every AI
/// behavior system, for both the primary and the point-defense target.
fn ai_target_anchor(
    target: Option<Entity>,
    q_target: &Query<(&Transform, Option<&ComputedCenterOfMass>)>,
) -> Option<Vec3> {
    let (transform, com) = q_target.get(target?).ok()?;
    Some(live_structure_anchor(transform, com))
}

fn update_controller_target_rotation_torque(
    time: Res<Time>,
    settings: Res<FlightSettings>,
    mut q_controller: Query<
        (&mut ControllerSectionRotationInput, &ChildOf),
        With<ControllerSectionMarker>,
    >,
    q_computer: Query<
        (&PDController, &ChildOf),
        (
            With<ControllerSectionMarker>,
            Without<SectionInactiveMarker>,
        ),
    >,
    q_spaceship: Query<
        (
            Entity,
            &Transform,
            &LinearVelocity,
            &ComputedAngularInertia,
            Option<&ComputedCenterOfMass>,
            &AIBehaviorState,
            &AITarget,
            &AIEvade,
        ),
        (With<SpaceshipRootMarker>, With<AISpaceshipMarker>),
    >,
    q_target: Query<(&Transform, Option<&ComputedCenterOfMass>)>,
) {
    for (entity, transform, velocity, inertia, com, state, target, evade) in &q_spaceship {
        // A non-engaging state (Idle/Patrol) holds its helm: the command
        // freezes exactly like a dead helm, so re-engaging resumes from
        // where the hull actually points. No target freezes it the same way.
        if !state.engages() {
            continue;
        }
        // Chase the target's live structure, not its root origin: the origin
        // is the build spot of the first sections and floats in empty space
        // once they are destroyed (task 20260709-150711).
        let Some(target_anchor) = ai_target_anchor(**target, &q_target) else {
            continue;
        };
        // Both ends of the chase vector track live structure: the AI's own
        // root origin goes as stale as the target's once sections die.
        let own_anchor = live_structure_anchor(transform, com);
        let to_target = target_anchor - own_anchor;
        // Evade swaps the standoff envelope for the jink weave; the guns
        // stay on target regardless (turret aim is hull-independent).
        let desired_direction = if *state == AIBehaviorState::Evade {
            ai_evade_direction(to_target, evade.leg)
        } else {
            ai_desired_direction(to_target, **velocity)
        };

        // Slew the command at the hull's torque-budget turn rate instead of
        // rewriting it every frame: a distant setpoint drives the PD into
        // torque saturation where its damping is swamped and the hull
        // limit-cycles - the regime the player path was fixed for in the
        // flight-feel retune (20260709-095043). Same derivation as the
        // player path and the autopilot (flight::ship_turn_rate). With no
        // live computer the command FREEZES, matching the player path:
        // nothing consumes it, and slewing a dead helm would drift it so a
        // later re-activation snaps the hull.
        let Some(turn_rate) = crate::flight::ship_turn_rate(
            q_computer
                .iter()
                .filter(|(_, &ChildOf(parent))| parent == entity)
                .map(|(pd, _)| pd.max_torque),
            inertia,
            &settings,
        ) else {
            continue;
        };
        let max_step = turn_rate * time.delta_secs();

        for (mut controller, _) in q_controller
            .iter_mut()
            .filter(|(_, ChildOf(parent))| *parent == entity)
        {
            // The input is an ABSOLUTE world rotation - every other writer
            // treats it that way; the old code wrote a delta arc (the bug
            // this task fixes). The goal carries the command's own forward
            // onto the desired direction, and the command evolves from ITS
            // OWN previous state, never from the hull: a command rebuilt
            // from the hull each tick inherits the hull's roll, the PD then
            // sees zero roll error, and roll picked up during a swing spins
            // the ship forever (see the autopilot's rotation step).
            let command = **controller;
            let command_forward = command * Vec3::NEG_Z;
            let goal = Quat::from_rotation_arc(command_forward, desired_direction) * command;
            **controller = crate::flight::slew_rotation(command, goal, max_step);
        }
    }
}

fn on_thruster_input(
    mut q_thruster: Query<
        (&mut ThrusterSectionInput, &GlobalTransform, &ChildOf),
        With<ThrusterSectionMarker>,
    >,
    q_spaceship: Query<
        (
            Entity,
            &Transform,
            &LinearVelocity,
            Option<&ComputedCenterOfMass>,
            &AIBehaviorState,
            &AITarget,
            &AIEvade,
            Has<Autopilot>,
        ),
        (With<SpaceshipRootMarker>, With<AISpaceshipMarker>),
    >,
    q_target: Query<(&Transform, Option<&ComputedCenterOfMass>)>,
) {
    for (entity, transform, velocity, com, state, target, evade, has_autopilot) in &q_spaceship {
        // While a passive-state maneuver is engaged the flight computer
        // owns the engines: writing here - even an explicit 0.0 - would
        // fight the autopilot's spooled inputs every frame.
        if has_autopilot {
            continue;
        }
        // A non-engaging state (or no target left to chase) cuts the burn -
        // written as an explicit 0.0, not a skip, so a ship that was
        // thrusting when the state flipped actually stops.
        let thrust_level = match ai_target_anchor(**target, &q_target) {
            Some(target_anchor) if state.engages() => {
                // Same live-structure vector as the rotation system, so the
                // thrust gate and the rotation command agree on where
                // "toward the target" is - including the jink swap, and with
                // a looser gate while evading so the lateral burst fires
                // mid-swing instead of waiting out the slew.
                let to_target = target_anchor - live_structure_anchor(transform, com);
                let (desired_direction, gate) = if *state == AIBehaviorState::Evade {
                    (
                        ai_evade_direction(to_target, evade.leg),
                        AI_EVADE_THRUST_ALIGNMENT,
                    )
                } else {
                    (
                        ai_desired_direction(to_target, **velocity),
                        AI_THRUST_ALIGNMENT,
                    )
                };

                // Thrust only when the ship is pointing roughly toward the
                // desired direction.
                let forward = transform.forward();
                let alignment = forward.dot(desired_direction);
                if alignment > gate {
                    1.0
                } else {
                    0.0
                }
            }
            _ => 0.0,
        };

        for (mut thruster_input, _, _) in q_thruster
            .iter_mut()
            .filter(|(_, _, ChildOf(parent))| *parent == entity)
        {
            **thruster_input = thrust_level;
        }
    }
}

fn update_turret_target_input(
    mut q_turret: Query<
        (
            &mut TurretSectionTargetInput,
            &mut TurretSectionTargetVelocity,
            &ChildOf,
        ),
        With<TurretSectionMarker>,
    >,
    q_spaceship: Query<
        (Entity, &AIBehaviorState, &AITarget, &AIPointDefenseTarget),
        (With<SpaceshipRootMarker>, With<AISpaceshipMarker>),
    >,
    q_target: Query<(&Transform, Option<&ComputedCenterOfMass>)>,
    q_target_velocity: Query<&LinearVelocity>,
) {
    for (entity, state, target, pd_target) in &q_spaceship {
        // The PDC override first: an inbound torpedo pulls the guns off the
        // primary target (the turrets' main purpose is torpedo defense -
        // user decision 20260710), and it applies in EVERY behavior state:
        // a patrolling or idle ship still defends itself. Otherwise the
        // engaging states track the primary target, and non-engaging states
        // clear the aim so turrets slew back to rest.
        let gun_target = (**pd_target).or_else(|| if state.engages() { **target } else { None });
        // Aim at the live structure: fire converging on the root origin
        // lands in empty space once the front sections die (task
        // 20260709-150711).
        let aim = ai_target_anchor(gun_target, &q_target);
        // Feed the target root's velocity alongside the position so
        // lead_intercept_point computes a real lead for AI turrets - the
        // AI-side sibling of the player lock feed (20260709-173700). The
        // solve is shooter-frame-correct on its own (20260709-211701).
        let velocity = aim
            .and_then(|_| gun_target.and_then(|entity| q_target_velocity.get(entity).ok()))
            .map(|velocity| **velocity)
            .unwrap_or(Vec3::ZERO);
        for (mut turret_input, mut turret_velocity, _) in q_turret
            .iter_mut()
            .filter(|(_, _, ChildOf(c_parent))| *c_parent == entity)
        {
            **turret_input = aim;
            **turret_velocity = velocity;
        }
    }
}

/// The free-running burst cycle of an AI ship's guns: fire for
/// [`AI_BURST_FIRE_SECS`], hold for [`AI_BURST_HOLD_SECS`], repeat. A ship
/// fires only while the window is open (and every other gate passes), so AI
/// fire reads as deliberate bursts instead of a continuous hose. Required by
/// [`AISpaceshipMarker`]; ticked by [`update_fire_cadence`].
#[derive(Component, Debug, Clone, Reflect)]
#[reflect(Component)]
pub struct AIFireCadence {
    /// Time left in the current phase.
    timer: Timer,
    /// Whether the current phase is a fire window (else a hold).
    firing: bool,
}

impl Default for AIFireCadence {
    fn default() -> Self {
        // Starts in a fire window so an AI ship dropped into a fight shoots
        // immediately, matching pre-cadence behavior at spawn.
        Self {
            timer: Timer::from_seconds(AI_BURST_FIRE_SECS, TimerMode::Once),
            firing: true,
        }
    }
}

impl AIFireCadence {
    /// Advance the cycle, flipping between fire and hold phases.
    fn tick(&mut self, delta: core::time::Duration) {
        self.timer.tick(delta);
        if self.timer.is_finished() {
            self.firing = !self.firing;
            let phase = if self.firing {
                AI_BURST_FIRE_SECS
            } else {
                AI_BURST_HOLD_SECS
            };
            self.timer = Timer::from_seconds(phase, TimerMode::Once);
        }
    }
}

/// Tick every AI ship's burst cycle. Free-running (it does not reset on
/// state or target changes): the phase offset between ships also staggers
/// their volleys for free.
fn update_fire_cadence(
    time: Res<Time>,
    mut q_spaceship: Query<&mut AIFireCadence, With<AISpaceshipMarker>>,
) {
    for mut cadence in &mut q_spaceship {
        cadence.tick(time.delta());
    }
}

/// Whether the straight line from `origin` to `aim` is blocked by a tangible
/// collider belonging to neither the shooter nor the target.
///
/// The ray IS the bullet's path: turrets steer to the LEADED aim point, so
/// occlusion is judged against that line, not the target's current position.
/// A hit that resolves to the TARGET's own body (a slow or near target whose
/// collider straddles the line before the lead point) is not occlusion.
/// Sensor colliders are transparent to the ray for the same reason they are
/// transparent to rounds (`despawn_bullet_on_hit` skips sensors): a beacon's
/// trigger sphere or a blast shell must not read as cover. The shooter's own
/// colliders are transparent too - the muzzle sits on its hull.
///
/// An unattributable tangible hit (no [`ColliderOf`]) counts as blocked:
/// failing closed holds one burst, failing open shoots through a wall.
///
/// The collider trees are maintained by the physics schedule, so this
/// Update-schedule reader sees poses at most one physics tick stale; for a
/// fire gate the cost is a one-frame-late hold or release at a cover edge,
/// never a wrong sustained decision.
fn ai_line_of_fire_blocked(
    spatial: &SpatialQuery,
    q_sensor: &Query<(), With<Sensor>>,
    q_collider_of: &Query<&ColliderOf>,
    shooter: Entity,
    target: Entity,
    origin: Vec3,
    aim: Vec3,
) -> bool {
    let to_aim = aim - origin;
    let Ok(direction) = Dir3::new(to_aim) else {
        // Degenerate line (aim on the muzzle): nothing to occlude.
        return false;
    };
    let body_of = |collider: Entity| q_collider_of.get(collider).map(|of| of.body);
    spatial
        .cast_ray_predicate(
            origin,
            direction,
            to_aim.length(),
            true,
            &SpatialQueryFilter::default(),
            // Predicate false = the collider is transparent to the ray.
            &|collider| !q_sensor.contains(collider) && body_of(collider) != Ok(shooter),
        )
        .is_some_and(|hit| body_of(hit.entity) != Ok(target))
}

fn on_projectile_input(
    mut q_turret: Query<
        (
            &TurretSectionMuzzleEntity,
            &TurretSectionAimPoint,
            &TurretSectionConfigHelper,
            &mut TurretSectionInput,
            &ChildOf,
        ),
        With<TurretSectionMarker>,
    >,
    q_muzzle: Query<&GlobalTransform, With<TurretSectionBarrelMuzzleMarker>>,
    q_spaceship: Query<
        (
            Entity,
            &AIBehaviorState,
            &AITarget,
            &AIPointDefenseTarget,
            &AIFireCadence,
        ),
        (With<SpaceshipRootMarker>, With<AISpaceshipMarker>),
    >,
    q_target: Query<(&Transform, Option<&ComputedCenterOfMass>)>,
    spatial: SpatialQuery,
    q_sensor: Query<(), With<Sensor>>,
    q_collider_of: Query<&ColliderOf>,
) {
    for (entity, state, target, pd_target, cadence) in &q_spaceship {
        // Same gun-target resolution as the aim system: PDC override first,
        // in every behavior state. While defending, the burst cadence is
        // BYPASSED - point defense fires continuously; bursts are a
        // discipline for shooting at ships, not at inbound ordnance.
        let defending = pd_target.is_some();
        let gun_target = (**pd_target).or_else(|| if state.engages() { **target } else { None });
        let target_anchor = ai_target_anchor(gun_target, &q_target);
        let firing_allowed = defending || (state.engages() && cadence.firing);
        for (muzzle, aim_point, config, mut input, _) in q_turret
            .iter_mut()
            .filter(|(_, _, _, _, ChildOf(c_parent))| *c_parent == entity)
        {
            // Hold fire with no gun target or outside the burst window -
            // written as an explicit false so a firing turret stops.
            let (Some(target_anchor), true) = (target_anchor, firing_allowed) else {
                **input = false;
                continue;
            };

            let Ok(muzzle_transform) = q_muzzle.get(**muzzle) else {
                error!(
                    "on_projectile_input: muzzle entity {:?} not found in q_muzzle",
                    **muzzle
                );
                continue;
            };

            // Range gate per turret: past the distance its bullets can
            // actually live (muzzle_speed * lifetime, with a margin), a shot
            // is noise, not pressure.
            let effective_range =
                config.muzzle_speed * config.projectile_lifetime * AI_FIRE_RANGE_FACTOR;
            let muzzle_position = muzzle_transform.translation();
            if target_anchor.distance(muzzle_position) > effective_range {
                **input = false;
                continue;
            }

            // Align against the LEADED aim point the turret actually steers
            // to (falling back to the anchor before the lead resolves): a
            // turret correctly leading a crossing target never aligns with
            // the raw anchor, and would otherwise hold fire forever.
            let aim = aim_point.unwrap_or(target_anchor);
            let direction_to_aim = (aim - muzzle_position).normalize();
            let forward = muzzle_transform.forward();

            let alignment = forward.dot(direction_to_aim);
            if alignment <= AI_FIRE_ALIGNMENT {
                **input = false;
                continue;
            }

            // Line-of-fire gate, last so only a shot that would otherwise
            // fire pays for the ray. Point defense is exempt: inbound
            // ordnance is hunting THIS ship, so its line is short, closing,
            // and the one case where a wasted round beats a held trigger.
            // target_anchor came through ai_target_anchor, so gun_target is
            // Some here; the else arm is unreachable belt-and-braces.
            let Some(gun_target) = gun_target else {
                **input = false;
                continue;
            };
            **input = defending
                || !ai_line_of_fire_blocked(
                    &spatial,
                    &q_sensor,
                    &q_collider_of,
                    entity,
                    gun_target,
                    muzzle_position,
                    aim,
                );
        }
    }
}

// AI torpedo launch tuning. The bay's own fire-rate cooldown
// (TorpedoSectionSpawnerFireState) still applies underneath; these knobs
// shape WHEN the AI pulls the trigger at all.
/// Per-bay launch cadence (s): the AI takes deliberate, spaced torpedo
/// shots instead of holding the trigger and dumping one every
/// 1/fire_rate seconds. Playtest knob.
const AI_TORPEDO_COOLDOWN_SECS: f32 = 10.0;
/// Outer edge (m) of the launch envelope. Beyond detection range
/// (AI_ENGAGE_RANGE) but well inside acquisition range
/// (AI_TARGET_MAX_RANGE), so a launch can open the approach on a fight the
/// ship is already committing to. Playtest knob.
const AI_TORPEDO_MAX_RANGE: f32 = 1000.0;
/// Inner edge of the envelope, as a multiple of the bay's configured blast
/// radius: a point-blank launch detonates inside the shooter's own blast
/// (blast damage deliberately affects the owner, 20260709-140559). The
/// factor keeps the detonation point - the target - clear of the shooter,
/// with margin for the closure that happens while the torpedo flies.
/// Playtest knob.
const AI_TORPEDO_MIN_RANGE_BLAST_FACTOR: f32 = 3.0;
/// Rough hull-alignment gate (cos) on a launch. Deliberately loose - PN
/// guidance does the turning - it exists so launches read as aimed attack
/// runs rather than popping off an orbit tangent pointed away, and so the
/// torpedo does not open its flight turning back through the shooter.
const AI_TORPEDO_ALIGNMENT_COS: f32 = 0.5;

/// Per-bay AI launch state: the launch cadence on top of the bay's own
/// fire-rate timer. Lazily inserted by [`update_torpedo_section_input`] on
/// torpedo sections whose ship is AI-controlled - an Add-observer would
/// race the root's `AISpaceshipMarker`, which lands after the child
/// sections spawn. Reset by [`update_torpedo_target_input`] when a launch
/// ACTUALLY happens (a projectile spawned), so a trigger pull a disabled
/// bay ignored never burns the cooldown.
#[derive(Component, Debug, Clone, Reflect)]
#[reflect(Component)]
pub struct AITorpedoBay {
    /// Time until this bay may launch again. Starts elapsed: the first
    /// launch of a fight comes as soon as the envelope opens.
    cooldown: Timer,
}

impl Default for AITorpedoBay {
    fn default() -> Self {
        let mut cooldown = Timer::from_seconds(AI_TORPEDO_COOLDOWN_SECS, TimerMode::Once);
        cooldown.tick(cooldown.duration());
        Self { cooldown }
    }
}

/// The geometric half of the launch decision: the target inside the range
/// band [blast_radius * [`AI_TORPEDO_MIN_RANGE_BLAST_FACTOR`],
/// [`AI_TORPEDO_MAX_RANGE`]] with the hull roughly on the bearing
/// ([`AI_TORPEDO_ALIGNMENT_COS`]). The per-ship gates (behavior state,
/// ship-kind target, bay cooldown) live in the calling system. Pure for
/// unit testing.
fn ai_torpedo_envelope(to_target: Vec3, forward: Vec3, blast_radius: f32) -> bool {
    let distance = to_target.length();
    if distance <= f32::EPSILON
        || distance < blast_radius * AI_TORPEDO_MIN_RANGE_BLAST_FACTOR
        || distance > AI_TORPEDO_MAX_RANGE
    {
        return false;
    }
    forward.dot(to_target / distance) > AI_TORPEDO_ALIGNMENT_COS
}

/// Pull each AI ship's torpedo triggers: write [`TorpedoSectionInput`] on
/// its bays when the launch envelope is open. The per-ship gates: an
/// Engage-like state - Evade excluded, a jinking hull is no launch
/// platform (Retreat inherits, per its stub) - and a SHIP target: hostile
/// torpedoes are the guns' job (point defense), not worth a bay's ordnance.
/// Per ship: the line of fire clear ([`ai_line_of_fire_blocked`]) - no
/// torpedo spent on the cover between the bay and the target. Per bay: the
/// launch cadence elapsed and the envelope ([`ai_torpedo_envelope`]) open
/// from the ship's anchor.
#[allow(clippy::type_complexity)]
fn update_torpedo_section_input(
    time: Res<Time>,
    mut commands: Commands,
    q_missing: Query<(Entity, &ChildOf), (With<TorpedoSectionMarker>, Without<AITorpedoBay>)>,
    mut q_section: Query<
        (
            &mut TorpedoSectionInput,
            &mut AITorpedoBay,
            &TorpedoSectionConfigHelper,
            &ChildOf,
        ),
        With<TorpedoSectionMarker>,
    >,
    q_spaceship: Query<
        (
            Entity,
            &Transform,
            Option<&ComputedCenterOfMass>,
            &AIBehaviorState,
            &AITarget,
        ),
        (With<SpaceshipRootMarker>, With<AISpaceshipMarker>),
    >,
    q_target: Query<(&Transform, Option<&ComputedCenterOfMass>)>,
    q_ship_root: Query<(), With<SpaceshipRootMarker>>,
    spatial: SpatialQuery,
    q_sensor: Query<(), With<Sensor>>,
    q_collider_of: Query<&ColliderOf>,
) {
    // Arm AI bays with their launch state, lazily: at section-spawn time
    // "is this an AI ship" is not answerable yet (see [`AITorpedoBay`]),
    // so a bare bay picks its state up here, one frame before first use.
    for (section, ChildOf(parent)) in &q_missing {
        if q_spaceship.contains(*parent) {
            commands.entity(section).insert(AITorpedoBay::default());
        }
    }

    for (entity, transform, com, state, target) in &q_spaceship {
        let engaged = state.engages() && *state != AIBehaviorState::Evade;
        // The launch bearing runs anchor to anchor, like every AI vector.
        let own_anchor = live_structure_anchor(transform, com);
        let target_ship = (**target).filter(|&target| q_ship_root.contains(target));
        let target_anchor =
            target_ship.and_then(|target| ai_target_anchor(Some(target), &q_target));
        // Line-of-fire gate, memoized so the ship casts AT MOST one ray per
        // frame (every bay launches down the same anchor-to-anchor bearing)
        // and none at all while the cheap per-bay gates hold the trigger
        // anyway (R1.1). A torpedo PN-navigates, so the straight ray is
        // conservative - it can hold a launch that would have curved around
        // a rock's edge; the cost is a delayed launch, never a torpedo
        // spent on cover.
        let mut line_clear: Option<bool> = None;
        let mut line_clear = |own_anchor: Vec3| {
            *line_clear.get_or_insert_with(|| match (target_ship, target_anchor) {
                (Some(target_ship), Some(anchor)) => !ai_line_of_fire_blocked(
                    &spatial,
                    &q_sensor,
                    &q_collider_of,
                    entity,
                    target_ship,
                    own_anchor,
                    anchor,
                ),
                _ => false,
            })
        };

        for (mut input, mut bay, config, _) in q_section
            .iter_mut()
            .filter(|(_, _, _, ChildOf(parent))| *parent == entity)
        {
            // The cadence elapses unconditionally - maneuvering outside
            // the envelope between launches is part of the cadence, not a
            // pause of it.
            bay.cooldown.tick(time.delta());
            let launch = engaged
                && bay.cooldown.is_finished()
                && target_anchor.is_some_and(|anchor| {
                    ai_torpedo_envelope(
                        anchor - own_anchor,
                        *transform.forward(),
                        config.blast_radius,
                    )
                })
                && line_clear(own_anchor);
            // Change-detection hygiene, and an explicit release (not a
            // skip) so a bay holding the trigger drops it the moment any
            // gate closes.
            if **input != launch {
                **input = launch;
            }
        }
    }
}

/// Commit each freshly launched AI torpedo to its owner's launch-time
/// [`AITarget`] - the AI-side sibling of the player's commit-on-launch
/// (input/player.rs): the targeting decision is made exactly once, right
/// after launch, and an owner with no target by commit time makes it a
/// dumb-fire shot for life. Also resets the sourcing bay's launch cadence
/// (attributed through the projectile's [`TorpedoSectionPartOf`]): only an
/// actual launch burns the cooldown. Torpedoes owned by non-AI ships are
/// left to the player's commit system, and vice versa.
///
/// Only a SHIP target commits, matching the trigger side's gate: the
/// launch and the commit are one frame apart, and an [`AITarget`] that
/// flipped to a hostile torpedo in that frame (ship target died, ordnance
/// in range) must not send this torpedo chasing ordnance - it dumb-fires
/// instead.
fn update_torpedo_target_input(
    mut commands: Commands,
    q_torpedo: Query<
        (Entity, &ProjectileOwner, &TorpedoSectionPartOf),
        (
            With<TorpedoProjectileMarker>,
            Without<TorpedoTargetEntity>,
            Without<TorpedoTargetChosen>,
        ),
    >,
    q_spaceship: Query<&AITarget, With<AISpaceshipMarker>>,
    q_ship_root: Query<(), With<SpaceshipRootMarker>>,
    mut q_bay: Query<&mut AITorpedoBay>,
) {
    for (torpedo, owner, part_of) in &q_torpedo {
        let Ok(target) = q_spaceship.get(**owner) else {
            continue;
        };
        let target = (**target).filter(|&target| q_ship_root.contains(target));

        debug!(
            "update_torpedo_target_input: committing AI torpedo {:?} to target {:?}",
            torpedo, target
        );

        let mut torpedo_commands = commands.entity(torpedo);
        torpedo_commands.insert(TorpedoTargetChosen);
        if let Some(target_entity) = target {
            torpedo_commands.insert(TorpedoTargetEntity(target_entity));
        }
        if let Ok(mut bay) = q_bay.get_mut(**part_of) {
            bay.cooldown.reset();
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

    /// The arrival grace (task 20260717-163042): a graced passive ship
    /// refuses the engage pull with a hostile in range; damage overrides
    /// the grace; grace composes with the leash (passive either way).
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
        // The LOAD-BEARING row (review R1.1): a graced ship in Engage
        // demotes to its routine - AIBehaviorState defaults to Engage, so
        // every graced scenario spawn's first tick IS this transition.
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

    /// Leash hysteresis: combat breaks off strictly beyond the radius,
    /// but a passive ship only re-engages once well back inside the
    /// re-engage band - between the two thresholds an engaged ship keeps
    /// fighting and a passive one keeps walking home, so a hostile parked
    /// at the boundary cannot ping-pong the state (review R1.2).
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
mod patrol_idle_tests {
    use bevy::ecs::system::RunSystemOnce;

    use super::*;

    const W1: Vec3 = Vec3::new(0.0, 0.0, -400.0);
    const W2: Vec3 = Vec3::new(400.0, 0.0, -400.0);

    /// Run the acquisition -> transition -> passive-flight pipeline once.
    fn run_pipeline(world: &mut World) {
        world.run_system_once(update_ai_target).unwrap();
        world.run_system_once(update_behavior_state).unwrap();
        world.run_system_once(update_passive_flight).unwrap();
    }

    fn patrol_world() -> (World, Entity) {
        let mut world = World::new();
        world.init_resource::<FlightSettings>();
        world.init_resource::<Time>();
        let ship = world
            .spawn((
                AISpaceshipMarker,
                AIPatrolRoute::new(vec![W1, W2]),
                Transform::default(),
                LinearVelocity(Vec3::ZERO),
            ))
            .id();
        (world, ship)
    }

    #[test]
    fn a_patrol_ship_engages_a_goto_toward_its_waypoint() {
        // No hostile in the world: the route makes the fallback Patrol, and
        // Patrol flies the first leg through the real autopilot.
        let (mut world, ship) = patrol_world();

        run_pipeline(&mut world);

        assert_eq!(
            *world.entity(ship).get::<AIBehaviorState>().unwrap(),
            AIBehaviorState::Patrol,
            "a routed ship without a hostile patrols instead of idling"
        );
        assert_eq!(
            world.entity(ship).get::<Autopilot>().map(|ap| ap.action),
            Some(AutopilotAction::GotoPos { position: W1 }),
            "Patrol engages the GOTO autopilot toward the current waypoint"
        );
    }

    #[test]
    fn arrival_turns_onto_the_next_leg() {
        let (mut world, ship) = patrol_world();
        // Parked just short of W1, inside standoff + slack: arrived.
        world
            .entity_mut(ship)
            .get_mut::<Transform>()
            .unwrap()
            .translation = W1 + Vec3::new(0.0, 0.0, 60.0);

        run_pipeline(&mut world);

        assert_eq!(
            world.entity(ship).get::<AIPatrolRoute>().unwrap().current,
            1,
            "reaching a waypoint advances the loop"
        );
        assert_eq!(
            world.entity(ship).get::<Autopilot>().map(|ap| ap.action),
            Some(AutopilotAction::GotoPos { position: W2 }),
            "the new leg is engaged immediately"
        );
    }

    #[test]
    fn the_loop_wraps_back_to_the_first_waypoint() {
        let (mut world, ship) = patrol_world();
        world
            .entity_mut(ship)
            .get_mut::<AIPatrolRoute>()
            .unwrap()
            .current = 1;
        world
            .entity_mut(ship)
            .get_mut::<Transform>()
            .unwrap()
            .translation = W2;

        run_pipeline(&mut world);

        assert_eq!(
            world.entity(ship).get::<AIPatrolRoute>().unwrap().current,
            0,
            "the route is a loop, not a one-way trip"
        );
    }

    #[test]
    fn an_out_of_range_index_self_heals() {
        // Both route fields are inspector-editable: shrinking the waypoint
        // list below `current` must wrap, not strand the patrol (R1.1).
        let (mut world, ship) = patrol_world();
        world
            .entity_mut(ship)
            .get_mut::<AIPatrolRoute>()
            .unwrap()
            .current = 7; // 7 % 2 waypoints = W2

        run_pipeline(&mut world);

        assert_eq!(
            world.entity(ship).get::<Autopilot>().map(|ap| ap.action),
            Some(AutopilotAction::GotoPos { position: W2 }),
            "an out-of-range index wraps instead of stranding the route"
        );
    }

    #[test]
    fn a_mid_leg_maneuver_is_left_alone() {
        // Re-running the pipeline mid-leg must not re-engage (churning the
        // autopilot would reset its phase every frame). A re-engage is
        // bit-identical to the first engage (autopilot_system never runs
        // here), so plant a sentinel phase: churn resets it to Align, a
        // left-alone maneuver keeps burning (hardened alongside review
        // R1.1 of task 20260711-212521).
        let (mut world, ship) = patrol_world();

        run_pipeline(&mut world);
        world.entity_mut(ship).get_mut::<Autopilot>().unwrap().phase = AutopilotPhase::Burn;
        run_pipeline(&mut world);

        assert_eq!(
            world.entity(ship).get::<Autopilot>().map(|ap| ap.phase),
            Some(AutopilotPhase::Burn),
            "an autopilot already flying the current leg is untouched"
        );
    }

    #[test]
    fn a_single_waypoint_station_holds_without_churn() {
        // Parked on a one-waypoint route with the drift killed: nothing to
        // fly, so nothing is engaged (re-engaging would churn
        // engage/complete every frame).
        let mut world = World::new();
        world.init_resource::<FlightSettings>();
        world.init_resource::<Time>();
        let ship = world
            .spawn((
                AISpaceshipMarker,
                AIPatrolRoute::new(vec![W1]),
                Transform::from_translation(W1 + Vec3::new(0.0, 0.0, 60.0)),
                LinearVelocity(Vec3::ZERO),
            ))
            .id();

        run_pipeline(&mut world);

        assert!(
            world.entity(ship).get::<Autopilot>().is_none(),
            "on station at rest: no maneuver to fly"
        );
    }

    #[test]
    fn an_idle_drifter_burns_to_rest_and_then_rests() {
        // No route, no hostile: Idle. Drifting engages a STOP burn...
        let mut world = World::new();
        world.init_resource::<FlightSettings>();
        world.init_resource::<Time>();
        let ship = world
            .spawn((
                AISpaceshipMarker,
                Transform::default(),
                LinearVelocity(Vec3::new(5.0, 0.0, 0.0)),
            ))
            .id();

        run_pipeline(&mut world);

        assert_eq!(
            *world.entity(ship).get::<AIBehaviorState>().unwrap(),
            AIBehaviorState::Idle
        );
        assert_eq!(
            world.entity(ship).get::<Autopilot>().map(|ap| ap.action),
            Some(AutopilotAction::Stop),
            "station keeping kills the drift through the real autopilot"
        );

        // ...while a ship already at rest is left alone (the autopilot
        // disengaged itself; below the drift threshold nothing re-engages).
        world.entity_mut(ship).remove::<Autopilot>();
        **world.entity_mut(ship).get_mut::<LinearVelocity>().unwrap() = Vec3::new(0.1, 0.0, 0.0);
        run_pipeline(&mut world);
        assert!(
            world.entity(ship).get::<Autopilot>().is_none(),
            "sub-threshold drift is accepted, not chased"
        );
    }

    #[test]
    fn a_hostile_in_detection_range_interrupts_the_patrol() {
        let (mut world, ship) = patrol_world();
        run_pipeline(&mut world);
        assert!(world.entity(ship).get::<Autopilot>().is_some());

        // A hostile pops inside detection range: Engage, and the passive
        // maneuver is dropped so the combat actuators own the ship.
        world.spawn((
            SpaceshipRootMarker,
            PlayerSpaceshipMarker,
            Transform::from_translation(Vec3::new(300.0, 0.0, 0.0)),
        ));
        run_pipeline(&mut world);

        assert_eq!(
            *world.entity(ship).get::<AIBehaviorState>().unwrap(),
            AIBehaviorState::Engage
        );
        assert!(
            world.entity(ship).get::<Autopilot>().is_none(),
            "engaging drops the passive-state autopilot"
        );
    }

    #[test]
    fn a_hostile_beyond_detection_range_leaves_the_patrol_flying() {
        // Settle onto the patrol first: ships spawn in the default Engage
        // state, and a combat state holds on ANY acquired target - only the
        // passive states gate on detection range.
        let (mut world, ship) = patrol_world();
        run_pipeline(&mut world);

        // Acquired (inside the 2000 m scan) but outside detection range.
        world.spawn((
            SpaceshipRootMarker,
            PlayerSpaceshipMarker,
            Transform::from_translation(Vec3::new(1500.0, 0.0, 0.0)),
        ));
        run_pipeline(&mut world);

        assert!(
            world.entity(ship).get::<AITarget>().unwrap().is_some(),
            "the hostile is acquired"
        );
        assert_eq!(
            *world.entity(ship).get::<AIBehaviorState>().unwrap(),
            AIBehaviorState::Patrol,
            "but too far to abort the patrol for"
        );
    }

    #[test]
    fn an_engaged_autopilot_owns_the_engines() {
        // While the flight computer flies a passive maneuver the AI thrust
        // system must not touch the throttles - not even to zero them.
        let (mut world, ship) = patrol_world();
        world
            .entity_mut(ship)
            .insert(Autopilot::engage(AutopilotAction::Stop));
        let thruster = world
            .spawn((
                ThrusterSectionMarker,
                ThrusterSectionInput(0.7),
                GlobalTransform::IDENTITY,
                ChildOf(ship),
            ))
            .id();

        world.run_system_once(on_thruster_input).unwrap();
        assert_eq!(
            **world
                .entity(thruster)
                .get::<ThrusterSectionInput>()
                .unwrap(),
            0.7,
            "an engaged autopilot owns the throttles"
        );

        // Without a maneuver the passive state cuts the burn, as before.
        world.entity_mut(ship).remove::<Autopilot>();
        world.entity_mut(ship).insert(AIBehaviorState::Patrol);
        world.run_system_once(on_thruster_input).unwrap();
        assert_eq!(
            **world
                .entity(thruster)
                .get::<ThrusterSectionInput>()
                .unwrap(),
            0.0,
            "no autopilot: the passive state zeroes the throttles"
        );
    }
}

#[cfg(test)]
mod orbit_directive_tests {
    use bevy::ecs::system::RunSystemOnce;

    use super::*;

    const WELL_ID: &str = "planetoid";

    /// Run the acquisition -> transition -> passive-flight pipeline once.
    fn run_pipeline(world: &mut World) {
        world.run_system_once(update_ai_target).unwrap();
        world.run_system_once(update_behavior_state).unwrap();
        world.run_system_once(update_passive_flight).unwrap();
    }

    /// A calm world with one orbit-directed AI ship; the well is spawned
    /// separately so tests can omit or delay it.
    fn orbit_world() -> (World, Entity) {
        let mut world = World::new();
        world.init_resource::<FlightSettings>();
        world.init_resource::<Time>();
        let ship = world
            .spawn((
                AISpaceshipMarker,
                AIOrbitDirective {
                    well: EntityId::new(WELL_ID),
                },
                Transform::default(),
                LinearVelocity(Vec3::ZERO),
            ))
            .id();
        (world, ship)
    }

    fn spawn_well(world: &mut World) -> Entity {
        world
            .spawn((
                GravityWell {
                    mu: 2400.0,
                    body_radius: 20.0,
                    soi_radius: 400.0,
                },
                EntityId::new(WELL_ID),
                Transform::from_translation(Vec3::new(0.0, 0.0, -200.0)),
            ))
            .id()
    }

    #[test]
    fn an_orbit_ship_engages_the_orbit_autopilot_on_its_well() {
        let (mut world, ship) = orbit_world();
        let well = spawn_well(&mut world);

        run_pipeline(&mut world);

        assert_eq!(
            *world.entity(ship).get::<AIBehaviorState>().unwrap(),
            AIBehaviorState::Orbit,
            "a directed ship without a hostile orbits instead of idling"
        );
        assert_eq!(
            world.entity(ship).get::<Autopilot>().map(|ap| ap.action),
            Some(AutopilotAction::Orbit { well, plan: None }),
            "Orbit engages the ORBIT autopilot on the resolved well (the \
             autopilot plans the ring itself on its first tick)"
        );
    }

    #[test]
    fn the_directive_beats_a_patrol_route() {
        let (mut world, ship) = orbit_world();
        spawn_well(&mut world);
        world
            .entity_mut(ship)
            .insert(AIPatrolRoute::new(vec![Vec3::new(0.0, 0.0, -400.0)]));

        run_pipeline(&mut world);

        assert_eq!(
            *world.entity(ship).get::<AIBehaviorState>().unwrap(),
            AIBehaviorState::Orbit,
            "passive precedence: orbit > patrol"
        );
        assert!(
            matches!(
                world.entity(ship).get::<Autopilot>().map(|ap| ap.action),
                Some(AutopilotAction::Orbit { .. })
            ),
            "the ORBIT maneuver is engaged, not the patrol GOTO"
        );
    }

    #[test]
    fn an_unresolvable_well_id_drifts_without_panicking() {
        // No well entity in the world: the state still becomes Orbit (the
        // directive is present), but nothing is engaged - the ship drifts
        // until the well appears (spawn-order tolerance)...
        let (mut world, ship) = orbit_world();

        run_pipeline(&mut world);
        assert_eq!(
            *world.entity(ship).get::<AIBehaviorState>().unwrap(),
            AIBehaviorState::Orbit
        );
        assert!(
            world.entity(ship).get::<Autopilot>().is_none(),
            "no live well matches the id: nothing to engage"
        );

        // ...and the same pipeline engages once it does (delivery guard for
        // the nothing-happens half above).
        let well = spawn_well(&mut world);
        run_pipeline(&mut world);
        assert_eq!(
            world.entity(ship).get::<Autopilot>().map(|ap| ap.action),
            Some(AutopilotAction::Orbit { well, plan: None }),
            "a late-spawned well is picked up by the retry"
        );
    }

    #[test]
    fn a_mid_flight_orbit_is_left_alone() {
        // Re-running the pipeline must not re-engage (churn would reset the
        // autopilot's plan every frame). A re-engage produces a component
        // bit-identical to the first (autopilot_system never runs here), so
        // plant a sentinel plan the real autopilot would have computed: a
        // churn resets it to None, a left-alone maneuver keeps it (R1.1).
        let (mut world, ship) = orbit_world();
        let well = spawn_well(&mut world);

        run_pipeline(&mut world);
        let sentinel = Some(OrbitPlan {
            radius: 123.0,
            normal: Vec3::Y,
        });
        world
            .entity_mut(ship)
            .get_mut::<Autopilot>()
            .unwrap()
            .action = AutopilotAction::Orbit {
            well,
            plan: sentinel,
        };
        run_pipeline(&mut world);

        assert_eq!(
            world.entity(ship).get::<Autopilot>().map(|ap| ap.action),
            Some(AutopilotAction::Orbit {
                well,
                plan: sentinel
            }),
            "an autopilot already circling the right well is untouched"
        );
    }

    #[test]
    fn a_retargeted_directive_re_engages_on_the_new_well() {
        // Editing the directive's well while an ORBIT is engaged must take
        // effect (review R1.2): ORBIT never self-completes, so waiting for
        // the autopilot to clear would ignore the retarget forever.
        let (mut world, ship) = orbit_world();
        spawn_well(&mut world);
        let other = world
            .spawn((
                GravityWell {
                    mu: 2400.0,
                    body_radius: 20.0,
                    soi_radius: 400.0,
                },
                EntityId::new("moon"),
                Transform::from_translation(Vec3::new(0.0, 0.0, 300.0)),
            ))
            .id();

        run_pipeline(&mut world);
        world
            .entity_mut(ship)
            .get_mut::<AIOrbitDirective>()
            .unwrap()
            .well = EntityId::new("moon");
        run_pipeline(&mut world);

        assert_eq!(
            world.entity(ship).get::<Autopilot>().map(|ap| ap.action),
            Some(AutopilotAction::Orbit {
                well: other,
                plan: None
            }),
            "the retargeted directive re-engages on the new well"
        );
    }

    #[test]
    fn combat_interrupts_the_orbit_and_calm_resumes_it() {
        let (mut world, ship) = orbit_world();
        let well = spawn_well(&mut world);
        run_pipeline(&mut world);
        assert!(world.entity(ship).get::<Autopilot>().is_some());

        // A hostile inside detection range: Engage, and the passive
        // maneuver is dropped so the combat actuators own the ship.
        let hostile = world
            .spawn((
                SpaceshipRootMarker,
                PlayerSpaceshipMarker,
                Transform::from_translation(Vec3::new(300.0, 0.0, 0.0)),
            ))
            .id();
        run_pipeline(&mut world);
        assert_eq!(
            *world.entity(ship).get::<AIBehaviorState>().unwrap(),
            AIBehaviorState::Engage
        );
        assert!(
            world.entity(ship).get::<Autopilot>().is_none(),
            "engaging drops the passive-state autopilot"
        );

        // The hostile gone, the ship returns to its ring.
        world.despawn(hostile);
        run_pipeline(&mut world);
        assert_eq!(
            *world.entity(ship).get::<AIBehaviorState>().unwrap(),
            AIBehaviorState::Orbit
        );
        assert_eq!(
            world.entity(ship).get::<Autopilot>().map(|ap| ap.action),
            Some(AutopilotAction::Orbit { well, plan: None }),
            "calm re-engages the orbit"
        );
    }
}

#[cfg(test)]
mod tests {
    use bevy::ecs::system::RunSystemOnce;

    use super::*;

    /// AI parity mirror (task 20260713-082337): engagement -> CombatLock +
    /// raised stance + a managed WeaponsHot, so the shared section-side
    /// safety gate never silences a fighting AI; the point-defense override
    /// wins; disengaging safes the guns.
    #[test]
    fn ai_engagement_mirrors_onto_the_combat_components() {
        use bevy::ecs::system::RunSystemOnce;

        let mut world = World::new();
        let enemy = world.spawn_empty().id();
        let torpedo = world.spawn_empty().id();
        let ship = world
            .spawn((
                AISpaceshipMarker,
                AITarget(None),
                AIPointDefenseTarget(None),
            ))
            .id();

        // Disengaged: managed but safe once the safety derivation runs.
        world.run_system_once(mirror_ai_combat_state).unwrap();
        world
            .run_system_once(crate::input::targeting::update_weapons_safety_for_tests)
            .unwrap();
        assert_eq!(world.get::<CombatLock>(ship).unwrap().0, None);
        assert!(
            !world.get::<WeaponsHot>(ship).unwrap().0,
            "disengaged AI is safe"
        );

        // Engaged on the primary target: locked, raised, hot.
        world.get_mut::<AITarget>(ship).unwrap().0 = Some(enemy);
        world.run_system_once(mirror_ai_combat_state).unwrap();
        world
            .run_system_once(crate::input::targeting::update_weapons_safety_for_tests)
            .unwrap();
        assert_eq!(world.get::<CombatLock>(ship).unwrap().0, Some(enemy));
        assert!(world.get::<WeaponsRaised>(ship).unwrap().0);
        assert!(world.get::<WeaponsHot>(ship).unwrap().0, "engaged AI fires");

        // Point defense overrides the primary.
        world.get_mut::<AIPointDefenseTarget>(ship).unwrap().0 = Some(torpedo);
        world.run_system_once(mirror_ai_combat_state).unwrap();
        assert_eq!(world.get::<CombatLock>(ship).unwrap().0, Some(torpedo));

        // Disengage everything: lock drops, stance lowers.
        world.get_mut::<AITarget>(ship).unwrap().0 = None;
        world.get_mut::<AIPointDefenseTarget>(ship).unwrap().0 = None;
        world.run_system_once(mirror_ai_combat_state).unwrap();
        world
            .run_system_once(crate::input::targeting::update_weapons_safety_for_tests)
            .unwrap();
        assert_eq!(world.get::<CombatLock>(ship).unwrap().0, None);
        assert!(!world.get::<WeaponsHot>(ship).unwrap().0);
    }

    #[test]
    fn ai_turrets_target_the_live_structure_anchor() {
        // AI fire must converge on the target's surviving structure, not the
        // root origin build-spot (task 20260709-150711). Driven through the
        // real acquisition system, not a hand-set target.
        let mut world = World::new();
        world.spawn((
            SpaceshipRootMarker,
            PlayerSpaceshipMarker,
            Transform::from_translation(Vec3::new(10.0, 0.0, 0.0)),
            ComputedCenterOfMass(Vec3::new(0.0, 0.0, 3.0)),
        ));
        let ai_ship = world.spawn((AISpaceshipMarker, Transform::default())).id();
        let turret = world
            .spawn((
                TurretSectionMarker,
                TurretSectionTargetInput(None),
                TurretSectionTargetVelocity(Vec3::ZERO),
                ChildOf(ai_ship),
            ))
            .id();

        world.run_system_once(update_ai_target).unwrap();
        world.run_system_once(update_turret_target_input).unwrap();

        assert_eq!(
            **world
                .entity(turret)
                .get::<TurretSectionTargetInput>()
                .unwrap(),
            Some(Vec3::new(10.0, 0.0, 3.0)),
            "AI turret input = the player's live-structure anchor"
        );
    }

    #[test]
    fn ai_turrets_fall_back_to_the_origin_without_a_com() {
        let mut world = World::new();
        world.spawn((
            SpaceshipRootMarker,
            PlayerSpaceshipMarker,
            Transform::from_translation(Vec3::new(1.0, 2.0, 3.0)),
        ));
        let ai_ship = world.spawn((AISpaceshipMarker, Transform::default())).id();
        let turret = world
            .spawn((
                TurretSectionMarker,
                TurretSectionTargetInput(None),
                TurretSectionTargetVelocity(Vec3::ZERO),
                ChildOf(ai_ship),
            ))
            .id();

        world.run_system_once(update_ai_target).unwrap();
        world.run_system_once(update_turret_target_input).unwrap();

        assert_eq!(
            **world
                .entity(turret)
                .get::<TurretSectionTargetInput>()
                .unwrap(),
            Some(Vec3::new(1.0, 2.0, 3.0))
        );
    }
}

#[cfg(test)]
mod rotation_tests {
    // Command-level harness with manual time, mirroring the player path's
    // command_lag_tests: the AI rotation command must be an ABSOLUTE world
    // rotation slewed at the hull's derived turn rate (task 20260709-155921).
    use core::time::Duration;

    use bevy::time::TimeUpdateStrategy;

    use super::*;

    /// An AI ship + controller facing -Z with the player dead astern (+Z),
    /// so the desired direction is a 180 flip from the initial command.
    fn flip_world() -> (App, Entity) {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
            1.0 / 60.0,
        )));
        app.init_resource::<FlightSettings>();
        // The real acquisition system feeds the rotation system, so the
        // harness drives the same pipeline the plugin chains.
        app.add_systems(
            Update,
            (update_ai_target, update_controller_target_rotation_torque).chain(),
        );

        // Dead astern, well OUTSIDE the standoff band so the approach
        // regime points straight at the player and the flip semantics hold.
        app.world_mut().spawn((
            SpaceshipRootMarker,
            PlayerSpaceshipMarker,
            Transform::from_translation(Vec3::new(0.0, 0.0, 800.0)),
        ));
        // The stock ship's numbers: inertia ~2.3, computer torque 10.
        let ship = app
            .world_mut()
            .spawn((
                AISpaceshipMarker,
                Transform::default(),
                LinearVelocity(Vec3::ZERO),
                ComputedAngularInertia::new(Vec3::splat(2.3)),
            ))
            .id();
        let controller = app
            .world_mut()
            .spawn((
                ChildOf(ship),
                ControllerSectionMarker,
                PDController {
                    frequency: 4.0,
                    damping_ratio: 4.0,
                    max_torque: 10.0,
                },
                ControllerSectionRotationInput::default(),
            ))
            .id();
        (app, controller)
    }

    #[test]
    fn an_ai_flip_reaches_the_command_over_many_frames() {
        // The old code rewrote the command every frame with no slew - the
        // exact PD-saturation regime the player path was fixed for.
        let (mut app, controller) = flip_world();

        // First update has dt = 0; the second advances one real frame.
        app.update();
        app.update();

        let command = **app
            .world()
            .get::<ControllerSectionRotationInput>(controller)
            .unwrap();
        let moved = command.angle_between(Quat::IDENTITY);
        let expected = crate::flight::hull_turn_rate(
            10.0,
            2.3,
            &app.world().resource::<FlightSettings>().clone(),
        ) / 60.0;
        // One frame advances exactly one slew step of the DERIVED rate -
        // this pins hull_turn_rate's wiring, not just "some" slew.
        assert!(
            (moved - expected).abs() < expected * 0.15,
            "one frame must advance one torque-budget slew step \
             (moved {moved}, expected {expected})"
        );
        let flip = Quat::from_rotation_arc(Vec3::NEG_Z, Vec3::Z);
        assert!(
            command.angle_between(flip) > 2.0,
            "a 180 flip must not reach the command in one frame"
        );
    }

    #[test]
    fn the_command_converges_to_the_absolute_look_at_rotation() {
        // The input is an absolute world rotation; the old code wrote a
        // DELTA (`from_rotation_arc(forward, desired)`), which for a
        // constant bearing never points the commanded forward at the
        // player. Slewed long enough, the command's forward must land on
        // the player bearing exactly.
        let (mut app, controller) = flip_world();

        for _ in 0..600 {
            app.update();
        }

        let command = **app
            .world()
            .get::<ControllerSectionRotationInput>(controller)
            .unwrap();
        let commanded_forward = command * Vec3::NEG_Z;
        let to_player = Vec3::Z; // player at +Z, ship at the origin
        assert!(
            commanded_forward.dot(to_player) > 0.999,
            "the commanded forward must converge on the player bearing, \
             got {commanded_forward:?}"
        );
    }

    #[test]
    fn a_dead_helm_freezes_the_command() {
        // With no live computer the command must not drift (matches the
        // player path): slewing a dead helm would snap the hull on a later
        // re-activation.
        let (mut app, controller) = flip_world();
        app.world_mut()
            .entity_mut(controller)
            .insert(SectionInactiveMarker);

        app.update();
        app.update();

        let command = **app
            .world()
            .get::<ControllerSectionRotationInput>(controller)
            .unwrap();
        assert_eq!(command, Quat::IDENTITY, "dead helm: the command freezes");
    }
}

#[cfg(test)]
mod physics_tests {
    // A real avian world with the real PD, mirroring flight.rs's
    // physics-level harness: AI rotation command -> PD torque -> hull
    // swings. Covers the task's acceptance: the AI swings to the target
    // attitude and settles without limit-cycling (task 20260709-155921).
    use super::*;
    use crate::{
        integrity::test_support::{settle, unfinished_integrity_physics_app},
        sections::controller_section::{
            sync_controller_section_forces, update_controller_section_rotation_input,
        },
    };

    #[test]
    fn the_ai_swings_onto_the_player_and_settles() {
        let mut app = unfinished_integrity_physics_app();
        app.init_resource::<FlightSettings>();
        app.add_plugins(PDControllerPlugin);
        app.configure_sets(
            FixedUpdate,
            (
                super::super::SpaceshipInputSystems,
                PDControllerSystems::Sync,
                SpaceshipSectionSystems,
            )
                .chain(),
        );
        app.add_systems(
            FixedUpdate,
            (
                update_ai_target,
                update_point_defense_target,
                update_behavior_state,
                update_controller_target_rotation_torque,
                update_controller_section_rotation_input,
            )
                .chain()
                .in_set(super::super::SpaceshipInputSystems),
        );
        app.add_systems(
            FixedUpdate,
            sync_controller_section_forces.in_set(SpaceshipSectionSystems),
        );
        app.finish();

        // Player abeam at +X, far outside the standoff band (approach
        // regime): a 90-degree swing from the AI's initial -Z onto +X.
        app.world_mut().spawn((
            SpaceshipRootMarker,
            PlayerSpaceshipMarker,
            Transform::from_translation(Vec3::new(1000.0, 0.0, 0.0)),
        ));
        let ship = app
            .world_mut()
            .spawn((RigidBody::Dynamic, Transform::default(), AISpaceshipMarker))
            .id();
        app.world_mut().spawn((
            ChildOf(ship),
            Name::new("hull"),
            Transform::from_xyz(0.0, 0.0, -1.0),
            Collider::cuboid(1.0, 1.0, 1.0),
            ColliderDensity(1.0),
        ));
        app.world_mut().spawn((
            ChildOf(ship),
            Name::new("controller"),
            ControllerSectionMarker,
            ControllerSectionRotationInput::default(),
            PDController {
                frequency: 4.0,
                damping_ratio: 4.0,
                max_torque: 10.0,
            },
            PDControllerTarget(ship),
            Transform::from_xyz(0.0, 0.0, 0.0),
            Collider::cuboid(1.0, 1.0, 1.0),
            ColliderDensity(1.0),
        ));

        settle(&mut app);
        // 10 simulated seconds: ample for the swing plus settling.
        for _ in 0..600 {
            app.update();
        }

        // No limit cycle on the aim: the nose must be ON the player and STAY
        // there for a further simulated second. The old delta-command code
        // fails this two ways: the delta setpoint never points the hull at
        // the player at all, and the unslewed rewrite saturates the PD into
        // an attitude limit cycle.
        let mut min_aim = f32::INFINITY;
        let mut max_spin = 0.0f32;
        for _ in 0..60 {
            app.update();
            let forward: Vec3 = app.world().get::<Transform>(ship).unwrap().forward().into();
            min_aim = min_aim.min(forward.dot(Vec3::X));
            let spin = app.world().get::<AngularVelocity>(ship).unwrap().length();
            max_spin = max_spin.max(spin);
        }
        assert!(
            min_aim > 0.996,
            "the hull must hold its nose on the player (within ~5 degrees) \
             for a full second, worst aim cos {min_aim}"
        );
        // The aim axes are quiet and, since the bcs inertia-frame fix
        // (20260709-125640), so is the roll: the residual spin in this rig
        // measures ~5e-6 rad/s. The bound leaves ~4 orders of margin for
        // solver noise while still tripping on any real roll-damping
        // regression (the pre-fix amplitude was ~0.23 rad/s).
        assert!(
            max_spin < 0.05,
            "residual spin must stay damped (20260709-125640), \
             got {max_spin} rad/s"
        );
    }
}

#[cfg(test)]
mod target_selection_tests {
    use bevy::ecs::system::RunSystemOnce;

    use super::*;

    fn entity(raw: u32) -> Entity {
        Entity::from_raw_u32(raw).unwrap()
    }

    // -- the pure picker --

    #[test]
    fn nearest_wins_within_a_tier() {
        let near = entity(1);
        let far = entity(2);
        let picked = pick_ai_target(
            Vec3::ZERO,
            None,
            None,
            [
                (far, Vec3::new(0.0, 0.0, -500.0), AITargetKind::Ship),
                (near, Vec3::new(0.0, 0.0, -100.0), AITargetKind::Ship),
            ]
            .into_iter(),
        );
        assert_eq!(picked, Some(near));
    }

    #[test]
    fn a_ship_beats_a_nearer_torpedo() {
        // Tiered priority, not a distance tweak: the urgency flip for
        // incoming torpedoes is the point-defense task (20260709-225733).
        let ship = entity(1);
        let torpedo = entity(2);
        let picked = pick_ai_target(
            Vec3::ZERO,
            None,
            None,
            [
                (torpedo, Vec3::new(0.0, 0.0, -50.0), AITargetKind::Torpedo),
                (ship, Vec3::new(0.0, 0.0, -1500.0), AITargetKind::Ship),
            ]
            .into_iter(),
        );
        assert_eq!(picked, Some(ship));
    }

    #[test]
    fn hysteresis_holds_the_current_pick_against_slivers() {
        let current = entity(1);
        let rival = entity(2);
        // The rival is 10% closer: inside the 20% hysteresis discount, the
        // current target holds.
        let held = pick_ai_target(
            Vec3::ZERO,
            Some(current),
            None,
            [
                (current, Vec3::new(0.0, 0.0, -1000.0), AITargetKind::Ship),
                (rival, Vec3::new(0.0, 0.0, -900.0), AITargetKind::Ship),
            ]
            .into_iter(),
        );
        assert_eq!(held, Some(current), "a sliver does not steal the pick");

        // At 2x closer the rival wins even against the discount.
        let stolen = pick_ai_target(
            Vec3::ZERO,
            Some(current),
            None,
            [
                (current, Vec3::new(0.0, 0.0, -1000.0), AITargetKind::Ship),
                (rival, Vec3::new(0.0, 0.0, -500.0), AITargetKind::Ship),
            ]
            .into_iter(),
        );
        assert_eq!(stolen, Some(rival), "a real gap does steal the pick");
    }

    #[test]
    fn out_of_range_or_empty_picks_nothing() {
        assert_eq!(
            pick_ai_target(
                Vec3::ZERO,
                None,
                None,
                [(entity(1), Vec3::new(0.0, 0.0, -2500.0), AITargetKind::Ship)].into_iter(),
            ),
            None,
            "beyond acquisition range"
        );
        assert_eq!(
            pick_ai_target(Vec3::ZERO, None, None, std::iter::empty()),
            None,
            "no candidates"
        );
    }

    #[test]
    fn the_recent_attacker_steals_the_pick_from_a_comparable_bystander() {
        // Recently-damaged-me threat scoring (deferred from 225727): the
        // ship shooting me outranks a somewhat closer bystander...
        let attacker = entity(1);
        let bystander = entity(2);
        let picked = pick_ai_target(
            Vec3::ZERO,
            None,
            Some(attacker),
            [
                (attacker, Vec3::new(0.0, 0.0, -1000.0), AITargetKind::Ship),
                (bystander, Vec3::new(0.0, 0.0, -700.0), AITargetKind::Ship),
            ]
            .into_iter(),
        );
        assert_eq!(picked, Some(attacker), "the shooter draws the aggro");

        // ...but the bias is a discount, not a tier: a bystander well
        // inside the discounted distance still wins.
        let picked = pick_ai_target(
            Vec3::ZERO,
            None,
            Some(attacker),
            [
                (attacker, Vec3::new(0.0, 0.0, -1000.0), AITargetKind::Ship),
                (bystander, Vec3::new(0.0, 0.0, -300.0), AITargetKind::Ship),
            ]
            .into_iter(),
        );
        assert_eq!(picked, Some(bystander), "a far closer threat still wins");
    }

    // -- the acquisition system over the relation model --

    #[test]
    fn acquisition_prefers_the_hostile_ship_and_ignores_non_hostiles() {
        let mut world = World::new();
        let ai_ship = world.spawn((AISpaceshipMarker, Transform::default())).id();
        // A fellow AI ship (Own), a neutral asteroid-like body (no
        // allegiance), and an uncommitted hostile torpedo: all ignored.
        world.spawn((
            AISpaceshipMarker,
            Transform::from_translation(Vec3::new(20.0, 0.0, 0.0)),
        ));
        world.spawn(Transform::from_translation(Vec3::new(30.0, 0.0, 0.0)));
        world.spawn((
            TorpedoProjectileMarker,
            Allegiance::Player,
            Transform::from_translation(Vec3::new(40.0, 0.0, 0.0)),
        ));
        // A committed hostile torpedo nearer than the hostile ship: the
        // ship still wins the tier.
        world.spawn((
            TorpedoProjectileMarker,
            TorpedoTargetChosen,
            Allegiance::Player,
            Transform::from_translation(Vec3::new(50.0, 0.0, 0.0)),
        ));
        let player = world
            .spawn((
                SpaceshipRootMarker,
                PlayerSpaceshipMarker,
                Transform::from_translation(Vec3::new(500.0, 0.0, 0.0)),
            ))
            .id();

        world.run_system_once(update_ai_target).unwrap();

        assert_eq!(
            **world.entity(ai_ship).get::<AITarget>().unwrap(),
            Some(player),
            "the hostile SHIP wins over the nearer hostile torpedo; \
             own/neutral/uncommitted bodies are never candidates"
        );
    }

    #[test]
    fn a_committed_hostile_torpedo_is_acquired_when_no_ship_remains() {
        let mut world = World::new();
        let ai_ship = world.spawn((AISpaceshipMarker, Transform::default())).id();
        let torpedo = world
            .spawn((
                TorpedoProjectileMarker,
                TorpedoTargetChosen,
                Allegiance::Player,
                Transform::from_translation(Vec3::new(50.0, 0.0, 0.0)),
            ))
            .id();

        world.run_system_once(update_ai_target).unwrap();

        assert_eq!(
            **world.entity(ai_ship).get::<AITarget>().unwrap(),
            Some(torpedo)
        );
    }

    #[test]
    fn a_dead_target_clears_on_the_next_pick() {
        let mut world = World::new();
        let ai_ship = world.spawn((AISpaceshipMarker, Transform::default())).id();
        let player = world
            .spawn((
                SpaceshipRootMarker,
                PlayerSpaceshipMarker,
                Transform::from_translation(Vec3::new(100.0, 0.0, 0.0)),
            ))
            .id();

        world.run_system_once(update_ai_target).unwrap();
        assert_eq!(
            **world.entity(ai_ship).get::<AITarget>().unwrap(),
            Some(player)
        );

        world.despawn(player);
        world.run_system_once(update_ai_target).unwrap();
        assert_eq!(
            **world.entity(ai_ship).get::<AITarget>().unwrap(),
            None,
            "consumers must never chase a stale entity"
        );
    }
}

#[cfg(test)]
mod fire_discipline_tests {
    use avian3d::collider_tree::ColliderTrees;
    use bevy::ecs::system::RunSystemOnce;

    use super::*;

    /// An AI ship engaged on a hand-set target, with one turret whose
    /// muzzle sits at the origin facing -Z. Returns (world, turret, muzzle).
    fn firing_world(target_position: Vec3, target_velocity: Vec3) -> (World, Entity, Entity) {
        let mut world = World::new();
        // Empty collider trees for the fire gate's SpatialQuery: no
        // colliders means no occluders. Occlusion itself is covered by the
        // physics-app tests in `line_of_fire_tests`.
        world.init_resource::<ColliderTrees>();
        let target = world
            .spawn((
                SpaceshipRootMarker,
                PlayerSpaceshipMarker,
                Transform::from_translation(target_position),
                LinearVelocity(target_velocity),
            ))
            .id();
        let ship = world
            .spawn((
                AISpaceshipMarker,
                AITarget(Some(target)),
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
        (world, turret, muzzle)
    }

    #[test]
    fn the_turret_feed_carries_the_target_velocity() {
        // The AI-side sibling of the player lock feed (20260709-173700):
        // without it, lead_intercept_point degenerates to aim-at-the-target
        // and AI turrets shoot behind every mover.
        let velocity = Vec3::new(30.0, 0.0, 0.0);
        let (mut world, turret, _) = firing_world(Vec3::new(0.0, 0.0, -100.0), velocity);

        world.run_system_once(update_turret_target_input).unwrap();

        assert_eq!(
            **world
                .entity(turret)
                .get::<TurretSectionTargetVelocity>()
                .unwrap(),
            velocity
        );
    }

    #[test]
    fn fire_aligns_with_the_leaded_aim_point_not_the_anchor() {
        // A crossing target: the aim point leads it well off the raw
        // anchor bearing. The muzzle (facing -Z) is ON the lead point and
        // ~22 degrees OFF the anchor (cos ~0.93, outside the 0.95 cone) -
        // discipline must fire anyway, because the turret is exactly where
        // the lead solution wants it.
        let (mut world, turret, _) = firing_world(Vec3::new(40.0, 0.0, -100.0), Vec3::ZERO);
        **world
            .entity_mut(turret)
            .get_mut::<TurretSectionAimPoint>()
            .unwrap() = Some(Vec3::new(0.0, 0.0, -100.0));

        world.run_system_once(on_projectile_input).unwrap();

        assert!(
            **world.entity(turret).get::<TurretSectionInput>().unwrap(),
            "aligned with the LEADED point: fire, even though the raw \
             anchor is off-axis"
        );
    }

    #[test]
    fn no_fire_beyond_the_effective_range() {
        // Dead ahead and perfectly aligned, but past muzzle_speed *
        // lifetime * margin (default 100 * 5 * 0.9 = 450 m): the bullet
        // dies in flight, so discipline holds.
        let (mut world, turret, _) = firing_world(Vec3::new(0.0, 0.0, -500.0), Vec3::ZERO);

        world.run_system_once(on_projectile_input).unwrap();

        assert!(
            !**world.entity(turret).get::<TurretSectionInput>().unwrap(),
            "beyond effective range: hold fire"
        );

        // The same shot inside the envelope fires.
        let (mut world, turret, _) = firing_world(Vec3::new(0.0, 0.0, -400.0), Vec3::ZERO);
        world.run_system_once(on_projectile_input).unwrap();
        assert!(
            **world.entity(turret).get::<TurretSectionInput>().unwrap(),
            "inside effective range and aligned: fire"
        );
    }

    #[test]
    fn the_burst_cadence_alternates_fire_and_hold() {
        let mut cadence = AIFireCadence::default();
        assert!(cadence.firing, "spawns in a fire window");

        // Tick past the fire window: the hold begins.
        cadence.tick(core::time::Duration::from_secs_f32(
            AI_BURST_FIRE_SECS + 0.01,
        ));
        assert!(!cadence.firing, "fire window over: hold");

        // Tick past the hold: firing resumes.
        cadence.tick(core::time::Duration::from_secs_f32(
            AI_BURST_HOLD_SECS + 0.01,
        ));
        assert!(cadence.firing, "hold over: next burst");
    }

    #[test]
    fn a_closed_burst_window_holds_fire_even_when_aligned() {
        let (mut world, turret, _) = firing_world(Vec3::new(0.0, 0.0, -100.0), Vec3::ZERO);
        // Force the ship's cadence into a hold phase.
        let ship = world
            .query_filtered::<Entity, With<AISpaceshipMarker>>()
            .iter(&world)
            .next()
            .unwrap();
        let mut cadence = world.entity_mut(ship);
        let mut cadence = cadence.get_mut::<AIFireCadence>().unwrap();
        cadence.tick(core::time::Duration::from_secs_f32(
            AI_BURST_FIRE_SECS + 0.01,
        ));
        assert!(!cadence.firing);

        world.run_system_once(on_projectile_input).unwrap();

        assert!(
            !**world.entity(turret).get::<TurretSectionInput>().unwrap(),
            "hold phase: no fire, alignment notwithstanding"
        );
    }
}

#[cfg(test)]
mod point_defense_tests {
    use avian3d::collider_tree::ColliderTrees;
    use bevy::ecs::system::RunSystemOnce;

    use super::*;

    fn entity(raw: u32) -> Entity {
        Entity::from_raw_u32(raw).unwrap()
    }

    // -- the pure picker --

    #[test]
    fn a_torpedo_hunting_me_outranks_a_nearer_one_hunting_someone_else() {
        let mine = entity(1);
        let other = entity(2);
        let picked = pick_point_defense_target(
            Vec3::ZERO,
            [
                (other, Vec3::new(0.0, 0.0, -50.0), false),
                (mine, Vec3::new(0.0, 0.0, -300.0), true),
            ]
            .into_iter(),
        );
        assert_eq!(picked, Some(mine));
    }

    #[test]
    fn nearest_wins_within_a_threat_tier_and_range_gates() {
        let near = entity(1);
        let far = entity(2);
        assert_eq!(
            pick_point_defense_target(
                Vec3::ZERO,
                [
                    (far, Vec3::new(0.0, 0.0, -350.0), true),
                    (near, Vec3::new(0.0, 0.0, -100.0), true),
                ]
                .into_iter(),
            ),
            Some(near)
        );
        assert_eq!(
            pick_point_defense_target(
                Vec3::ZERO,
                [(near, Vec3::new(0.0, 0.0, -500.0), true)].into_iter(),
            ),
            None,
            "outside point-defense range: the primary target keeps the guns"
        );
    }

    // -- the acquisition + turret override --

    /// An AI ship engaged on the player, with a hostile committed torpedo
    /// hunting the AI ship inside point-defense range. Returns
    /// (world, ai_ship, player, torpedo, turret).
    fn defended_world() -> (World, Entity, Entity, Entity, Entity) {
        let mut world = World::new();
        // Empty collider trees for the fire gate's SpatialQuery (no
        // occluders in this rig; PD bypasses the gate anyway).
        world.init_resource::<ColliderTrees>();
        let player = world
            .spawn((
                SpaceshipRootMarker,
                PlayerSpaceshipMarker,
                Transform::from_translation(Vec3::new(300.0, 0.0, 0.0)),
                LinearVelocity(Vec3::ZERO),
            ))
            .id();
        let ai_ship = world.spawn((AISpaceshipMarker, Transform::default())).id();
        let torpedo = world
            .spawn((
                TorpedoProjectileMarker,
                TorpedoTargetChosen,
                TorpedoTargetEntity(ai_ship),
                Allegiance::Player,
                Transform::from_translation(Vec3::new(0.0, 0.0, -150.0)),
                LinearVelocity(Vec3::new(0.0, 0.0, 30.0)),
            ))
            .id();
        let turret = world
            .spawn((
                TurretSectionMarker,
                TurretSectionTargetInput(None),
                TurretSectionTargetVelocity(Vec3::ZERO),
                TurretSectionAimPoint(None),
                TurretSectionConfigHelper(TurretSectionConfig::default()),
                TurretSectionInput(false),
                TurretSectionMuzzleEntity(Entity::PLACEHOLDER),
                ChildOf(ai_ship),
            ))
            .id();
        (world, ai_ship, player, torpedo, turret)
    }

    #[test]
    fn the_guns_defend_while_the_hull_keeps_chasing_the_ship() {
        let (mut world, ai_ship, player, torpedo, turret) = defended_world();

        world.run_system_once(update_ai_target).unwrap();
        world.run_system_once(update_point_defense_target).unwrap();
        world.run_system_once(update_turret_target_input).unwrap();

        // Flight target: still the hostile SHIP (ship-first tiers).
        assert_eq!(
            **world.entity(ai_ship).get::<AITarget>().unwrap(),
            Some(player),
            "the hull keeps chasing the ship"
        );
        // Gun target: the inbound torpedo, position and velocity.
        assert_eq!(
            **world.entity(ai_ship).get::<AIPointDefenseTarget>().unwrap(),
            Some(torpedo)
        );
        assert_eq!(
            **world
                .entity(turret)
                .get::<TurretSectionTargetInput>()
                .unwrap(),
            Some(Vec3::new(0.0, 0.0, -150.0)),
            "the guns aim at the torpedo, not the ship"
        );
        assert_eq!(
            **world
                .entity(turret)
                .get::<TurretSectionTargetVelocity>()
                .unwrap(),
            Vec3::new(0.0, 0.0, 30.0),
            "the lead feed follows the gun target"
        );
    }

    #[test]
    fn point_defense_bypasses_the_burst_hold() {
        let (mut world, ai_ship, _, _, turret) = defended_world();
        // Muzzle at the origin facing -Z: dead on the torpedo at -150.
        let muzzle = world
            .spawn((TurretSectionBarrelMuzzleMarker, GlobalTransform::IDENTITY))
            .id();
        world
            .entity_mut(turret)
            .insert(TurretSectionMuzzleEntity(muzzle));
        // Force the cadence into a hold phase: bursts must not delay defense.
        {
            let mut entity = world.entity_mut(ai_ship);
            let mut cadence = entity.get_mut::<AIFireCadence>().unwrap();
            cadence.tick(core::time::Duration::from_secs_f32(
                AI_BURST_FIRE_SECS + 0.01,
            ));
            assert!(!cadence.firing);
        }

        world.run_system_once(update_ai_target).unwrap();
        world.run_system_once(update_point_defense_target).unwrap();
        world.run_system_once(on_projectile_input).unwrap();

        assert!(
            **world.entity(turret).get::<TurretSectionInput>().unwrap(),
            "PDC fires through the burst hold"
        );
    }

    #[test]
    fn an_idle_ship_still_defends_itself() {
        let (mut world, ai_ship, _, torpedo, turret) = defended_world();
        world.entity_mut(ai_ship).insert(AIBehaviorState::Idle);

        world.run_system_once(update_point_defense_target).unwrap();
        world.run_system_once(update_turret_target_input).unwrap();

        let _ = torpedo;
        assert_eq!(
            **world
                .entity(turret)
                .get::<TurretSectionTargetInput>()
                .unwrap(),
            Some(Vec3::new(0.0, 0.0, -150.0)),
            "point defense applies in every behavior state"
        );
    }
}

#[cfg(test)]
mod threat_tests {
    // Damage attribution into the threat memory: bcs populates
    // HealthApplyDamage.source with the hitting collider, and the observer
    // resolves it to the firing ship root through ProjectileOwner
    // (task 20260709-225731, wiring deferred from 225727).
    use super::*;

    fn threat_world() -> (World, Entity) {
        let mut world = World::new();
        world.add_observer(on_damage_track_threat);
        let ship = world.spawn((AISpaceshipMarker, Transform::default())).id();
        (world, ship)
    }

    fn hit(world: &mut World, ship: Entity, source: Entity) {
        world.trigger(HealthApplyDamage {
            entity: ship,
            source: Some(source),
            amount: 10.0,
        });
    }

    fn recent_attacker(world: &World, ship: Entity) -> Option<Entity> {
        world
            .entity(ship)
            .get::<AIThreat>()
            .unwrap()
            .recent_attacker()
    }

    #[test]
    fn a_bullet_hit_records_its_owner_as_the_attacker() {
        // Turret bullet: root and collider are one entity, the owner sits
        // right on the source.
        let (mut world, ship) = threat_world();
        let player = world
            .spawn((SpaceshipRootMarker, PlayerSpaceshipMarker))
            .id();
        let bullet = world
            .spawn((ProjectileOwner(player), Allegiance::Player))
            .id();

        hit(&mut world, ship, bullet);

        assert_eq!(recent_attacker(&world, ship), Some(player));
    }

    #[test]
    fn a_warhead_section_resolves_through_its_projectile_root() {
        // Torpedo contact damage: the source is the warhead child section;
        // the owner (and copied allegiance) live on the projectile root.
        let (mut world, ship) = threat_world();
        let player = world
            .spawn((SpaceshipRootMarker, PlayerSpaceshipMarker))
            .id();
        let torpedo = world
            .spawn((
                TorpedoProjectileMarker,
                ProjectileOwner(player),
                Allegiance::Player,
            ))
            .id();
        let warhead = world.spawn(ChildOf(torpedo)).id();

        hit(&mut world, ship, warhead);

        assert_eq!(recent_attacker(&world, ship), Some(player));
    }

    #[test]
    fn a_hit_on_a_section_propagates_to_the_root_threat() {
        // The production path: bcs triggers the event on the HIT SECTION
        // and it propagates through ChildOf to the root. The observer must
        // catch it at the root hop (R1.1).
        let (mut world, ship) = threat_world();
        let section = world.spawn(ChildOf(ship)).id();
        let player = world
            .spawn((SpaceshipRootMarker, PlayerSpaceshipMarker))
            .id();
        let bullet = world
            .spawn((ProjectileOwner(player), Allegiance::Player))
            .id();

        hit(&mut world, section, bullet);

        assert_eq!(
            recent_attacker(&world, ship),
            Some(player),
            "a section hit must reach the root's threat memory"
        );
    }

    #[test]
    fn a_self_blast_is_not_a_threat() {
        // Blast damage deliberately reaches the owner (see ProjectileHooks):
        // a ship caught in its own torpedo's blast must not spook itself.
        let (mut world, ship) = threat_world();
        let blast = world.spawn((ProjectileOwner(ship), Allegiance::Enemy)).id();

        hit(&mut world, ship, blast);

        assert_eq!(recent_attacker(&world, ship), None);
        assert!(
            !world
                .entity(ship)
                .get::<AIThreat>()
                .unwrap()
                .recently_damaged(),
            "an own-relation hit must not open the threat window"
        );
    }

    #[test]
    fn a_hostile_ram_blames_the_rammer() {
        // No ProjectileOwner anywhere up the chain: a body-to-body hit. The
        // source's own ship root is the attacker.
        let (mut world, ship) = threat_world();
        let rammer = world
            .spawn((SpaceshipRootMarker, PlayerSpaceshipMarker))
            .id();
        let rammer_section = world.spawn(ChildOf(rammer)).id();

        hit(&mut world, ship, rammer_section);

        assert_eq!(recent_attacker(&world, ship), Some(rammer));
    }

    #[test]
    fn the_memory_decays_and_unattributed_hits_keep_the_shooter() {
        let attacker = Entity::from_raw_u32(7).unwrap();
        let mut threat = AIThreat::default();
        assert!(!threat.recently_damaged(), "spawns with no memory");

        threat.record(Some(attacker));
        assert_eq!(threat.recent_attacker(), Some(attacker));

        // A later hit that could not be attributed keeps the known shooter:
        // the most likely source has not changed.
        threat.record(None);
        assert_eq!(threat.recent_attacker(), Some(attacker));

        threat
            .damage_memory
            .tick(core::time::Duration::from_secs_f32(
                AI_THREAT_DAMAGE_MEMORY_SECS + 0.01,
            ));
        assert_eq!(threat.recent_attacker(), None, "the window closed");
    }
}

#[cfg(test)]
mod evade_tests {
    // The evade cycle through the real acquisition -> transition pipeline,
    // with real time (the manual-duration harness the rotation tests use).
    use core::time::Duration;

    use bevy::{ecs::system::RunSystemOnce, time::TimeUpdateStrategy};

    use super::*;

    fn evade_app() -> (App, Entity, Entity) {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
            1.0 / 60.0,
        )));
        app.add_observer(on_damage_track_threat);
        app.add_systems(Update, (update_ai_target, update_behavior_state).chain());

        // Inside engage range, NOT aiming at the ship (default forward -Z,
        // the ship is at -X of the player): only the damage signal fires.
        let player = app
            .world_mut()
            .spawn((
                SpaceshipRootMarker,
                PlayerSpaceshipMarker,
                Transform::from_translation(Vec3::new(300.0, 0.0, 0.0)),
            ))
            .id();
        let ship = app
            .world_mut()
            .spawn((AISpaceshipMarker, Transform::default()))
            .id();
        (app, ship, player)
    }

    fn state_of(app: &App, ship: Entity) -> AIBehaviorState {
        *app.world().get::<AIBehaviorState>(ship).unwrap()
    }

    #[test]
    fn a_hit_breaks_engage_into_evade_and_the_cycle_decays_back() {
        let (mut app, ship, player) = evade_app();

        // Settle into Engage on the acquired hostile.
        app.update();
        app.update();
        assert_eq!(state_of(&app, ship), AIBehaviorState::Engage);

        // A hostile bullet lands.
        let bullet = app
            .world_mut()
            .spawn((ProjectileOwner(player), Allegiance::Player))
            .id();
        app.world_mut().trigger(HealthApplyDamage {
            entity: ship,
            source: Some(bullet),
            amount: 10.0,
        });

        app.update();
        assert_eq!(
            state_of(&app, ship),
            AIBehaviorState::Evade,
            "getting shot breaks Engage into Evade"
        );

        // Mid-cycle the state holds...
        for _ in 0..60 {
            app.update();
        }
        assert_eq!(state_of(&app, ship), AIBehaviorState::Evade);

        // ...and past AI_EVADE_SECS the cycle decays back to Engage (the
        // damage memory is shorter than the cycle, and the player is not
        // aiming, so nothing re-triggers).
        for _ in 0..((AI_EVADE_SECS * 60.0) as usize + 30) {
            app.update();
        }
        assert_eq!(
            state_of(&app, ship),
            AIBehaviorState::Engage,
            "the jink is timed: it decays back to Engage"
        );
    }

    #[test]
    fn a_hostile_holding_its_nose_on_me_triggers_evade_without_a_hit() {
        // The second cheap signal: inside aim range with the hostile's hull
        // forward on my anchor. Driven through the real systems with a
        // zero-delta Time - entry does not need elapsed time.
        let mut world = World::new();
        world.init_resource::<Time>();
        let ship = world.spawn((AISpaceshipMarker, Transform::default())).id();
        world.spawn((
            SpaceshipRootMarker,
            PlayerSpaceshipMarker,
            Transform::from_translation(Vec3::new(300.0, 0.0, 0.0)).looking_at(Vec3::ZERO, Vec3::Y),
        ));

        world.run_system_once(update_ai_target).unwrap();
        world.run_system_once(update_behavior_state).unwrap();

        assert_eq!(
            *world.entity(ship).get::<AIBehaviorState>().unwrap(),
            AIBehaviorState::Evade,
            "a hostile's guns on me inside aim range is a threat"
        );
    }

    #[test]
    fn beyond_aim_range_a_pointed_nose_is_not_a_threat() {
        // Same geometry outside AI_THREAT_AIM_RANGE (still inside engage
        // range): the nose cannot hurt me yet, so the ship keeps engaging.
        let mut world = World::new();
        world.init_resource::<Time>();
        let ship = world.spawn((AISpaceshipMarker, Transform::default())).id();
        world.spawn((
            SpaceshipRootMarker,
            PlayerSpaceshipMarker,
            Transform::from_translation(Vec3::new(AI_THREAT_AIM_RANGE + 100.0, 0.0, 0.0))
                .looking_at(Vec3::ZERO, Vec3::Y),
        ));

        world.run_system_once(update_ai_target).unwrap();
        world.run_system_once(update_behavior_state).unwrap();

        assert_eq!(
            *world.entity(ship).get::<AIBehaviorState>().unwrap(),
            AIBehaviorState::Engage
        );
    }

    #[test]
    fn an_evading_ship_burns_along_the_jink_not_the_pursuit_vector() {
        // Target dead ahead at -Z, far outside the standoff band: Engage
        // would burn straight at it. Evade must not - the jink leg points
        // well off the line of sight.
        let mut world = World::new();
        let target = world
            .spawn((
                SpaceshipRootMarker,
                PlayerSpaceshipMarker,
                Transform::from_translation(Vec3::new(0.0, 0.0, -1000.0)),
            ))
            .id();
        let ship = world
            .spawn((
                AISpaceshipMarker,
                AIBehaviorState::Evade,
                AITarget(Some(target)),
                Transform::default(),
                LinearVelocity(Vec3::ZERO),
            ))
            .id();
        let thruster = world
            .spawn((
                ThrusterSectionMarker,
                ThrusterSectionInput(0.0),
                GlobalTransform::IDENTITY,
                ChildOf(ship),
            ))
            .id();

        // Facing the target (the pursuit vector): an engaging ship would
        // burn, an evading one must hold - the jink points elsewhere.
        world.run_system_once(on_thruster_input).unwrap();
        assert_eq!(
            **world
                .entity(thruster)
                .get::<ThrusterSectionInput>()
                .unwrap(),
            0.0,
            "facing the target, the jink gate must not open"
        );

        // Swing the hull onto the jink leg: the lateral burst fires.
        let jink = ai_evade_direction(Vec3::new(0.0, 0.0, -1000.0), 0);
        world
            .entity_mut(ship)
            .get_mut::<Transform>()
            .unwrap()
            .look_to(jink, Vec3::Y);
        world.run_system_once(on_thruster_input).unwrap();
        assert_eq!(
            **world
                .entity(thruster)
                .get::<ThrusterSectionInput>()
                .unwrap(),
            1.0,
            "aligned with the jink leg, the burst fires"
        );
    }
}

#[cfg(test)]
mod jink_tests {
    use super::*;

    const LOS_TARGET: Vec3 = Vec3::new(0.0, 0.0, -400.0);

    #[test]
    fn every_leg_stays_off_the_pursuit_vector() {
        let los = LOS_TARGET.normalize();
        for leg in 0..8 {
            let direction = ai_evade_direction(LOS_TARGET, leg);
            assert!(
                direction.dot(los).abs() < 0.5,
                "leg {leg} hugs the pursuit vector: {direction:?}"
            );
            assert!(
                (direction.length() - 1.0).abs() < 1e-3,
                "leg {leg} is not a unit direction"
            );
        }
    }

    #[test]
    fn consecutive_legs_swing_the_heading_hard() {
        for leg in 0..8 {
            let a = ai_evade_direction(LOS_TARGET, leg);
            let b = ai_evade_direction(LOS_TARGET, leg + 1);
            assert!(
                a.dot(b) < 0.5,
                "legs {leg} and {} barely differ: {a:?} vs {b:?}",
                leg + 1
            );
        }
    }

    #[test]
    fn the_pattern_wraps_and_survives_degenerate_geometry() {
        assert_eq!(
            ai_evade_direction(LOS_TARGET, 0),
            ai_evade_direction(LOS_TARGET, 4),
            "the box weave is a 4-leg loop"
        );
        // A polar line of sight uses the X-fallback tangent basis.
        let polar = ai_evade_direction(Vec3::new(0.0, 300.0, 0.0), 1);
        assert!(polar.is_finite() && polar.length() > 0.9);
        // A degenerate (zero) line of sight yields no direction at all.
        assert_eq!(ai_evade_direction(Vec3::ZERO, 0), Vec3::ZERO);
    }
}

#[cfg(test)]
mod standoff_tests {
    use super::*;

    #[test]
    fn far_outside_the_band_the_ship_approaches() {
        let to_target = Vec3::new(0.0, 0.0, -1000.0);
        let desired = ai_desired_direction(to_target, Vec3::ZERO);
        assert!(
            desired.dot(to_target.normalize()) > 0.999,
            "far away: point straight at the target, got {desired:?}"
        );
    }

    #[test]
    fn inside_the_band_the_ship_orbits() {
        // Dead on the preferred range: the radial term vanishes and the
        // desired direction is tangential to the line of sight.
        let to_target = Vec3::new(0.0, 0.0, -AI_STANDOFF_RANGE);
        let desired = ai_desired_direction(to_target, Vec3::ZERO);
        assert!(
            desired.dot(to_target.normalize()).abs() < 0.05,
            "in band: orbit, not chase (los dot {})",
            desired.dot(to_target.normalize())
        );
        assert!(
            (desired.length() - 1.0).abs() < 1e-3,
            "the desired direction stays a unit vector"
        );
    }

    #[test]
    fn too_close_the_ship_extends_away() {
        let to_target = Vec3::new(0.0, 0.0, -50.0);
        let desired = ai_desired_direction(to_target, Vec3::ZERO);
        assert!(
            desired.dot(to_target.normalize()) < -0.9,
            "well inside the envelope: extend AWAY from the target, got {desired:?}"
        );
    }

    #[test]
    fn the_overshoot_brake_regime_survives_the_envelope() {
        // Screaming toward the target far faster than the speed budget:
        // the ship points opposite its velocity, exactly as pre-envelope.
        let to_target = Vec3::new(0.0, 0.0, -1000.0);
        let velocity = Vec3::new(0.0, 0.0, -100.0);
        let desired = ai_desired_direction(to_target, velocity);
        assert!(
            desired.dot(velocity.normalize()) < -0.999,
            "overshooting: brake against the velocity, got {desired:?}"
        );
    }

    #[test]
    fn a_polar_line_of_sight_still_orbits() {
        // Line of sight straight up Y: the Y-cross tangent degenerates and
        // the X fallback must keep the orbit term finite.
        let to_target = Vec3::new(0.0, AI_STANDOFF_RANGE, 0.0);
        let desired = ai_desired_direction(to_target, Vec3::ZERO);
        assert!(
            desired.is_finite() && desired.length() > 0.9,
            "polar approach must not degenerate, got {desired:?}"
        );
    }
}

#[cfg(test)]
mod standoff_physics_tests {
    // The full diegetic loop on the physics harness: acquisition ->
    // behavior -> rotation command -> PD torque -> hull swing -> aligned
    // thrust -> impulses. Pins the task's acceptance: the ship settles
    // into the standoff band instead of closing to zero (ramming/parking).
    use super::*;
    use crate::{
        integrity::test_support::{settle, unfinished_integrity_physics_app},
        sections::{
            controller_section::{
                sync_controller_section_forces, update_controller_section_rotation_input,
            },
            thruster_section::thruster_impulse_system,
        },
    };

    #[test]
    fn the_ship_settles_into_the_standoff_band() {
        let mut app = unfinished_integrity_physics_app();
        app.init_resource::<FlightSettings>();
        app.add_plugins(PDControllerPlugin);
        app.configure_sets(
            FixedUpdate,
            (
                super::super::SpaceshipInputSystems,
                PDControllerSystems::Sync,
                SpaceshipSectionSystems,
            )
                .chain(),
        );
        app.add_systems(
            FixedUpdate,
            (
                update_ai_target,
                update_behavior_state,
                update_controller_target_rotation_torque,
                on_thruster_input,
                update_controller_section_rotation_input,
            )
                .chain()
                .in_set(super::super::SpaceshipInputSystems),
        );
        app.add_systems(
            FixedUpdate,
            (sync_controller_section_forces, thruster_impulse_system)
                .in_set(SpaceshipSectionSystems),
        );
        app.finish();

        // The target dead ahead (-Z), outside the band.
        let player_position = Vec3::new(0.0, 0.0, -600.0);
        app.world_mut().spawn((
            SpaceshipRootMarker,
            PlayerSpaceshipMarker,
            Transform::from_translation(player_position),
        ));
        let ship = app
            .world_mut()
            .spawn((RigidBody::Dynamic, Transform::default(), AISpaceshipMarker))
            .id();
        app.world_mut().spawn((
            ChildOf(ship),
            Name::new("hull"),
            Transform::from_xyz(0.0, 0.0, -1.0),
            Collider::cuboid(1.0, 1.0, 1.0),
            ColliderDensity(1.0),
        ));
        app.world_mut().spawn((
            ChildOf(ship),
            Name::new("thruster"),
            ThrusterSectionMarker,
            ThrusterSectionMagnitude(1.0),
            ThrusterSectionInput(0.0),
            Transform::from_xyz(0.0, 0.0, 1.0),
            Collider::cuboid(1.0, 1.0, 1.0),
            ColliderDensity(1.0),
        ));
        app.world_mut().spawn((
            ChildOf(ship),
            Name::new("controller"),
            ControllerSectionMarker,
            ControllerSectionRotationInput::default(),
            PDController {
                frequency: 4.0,
                damping_ratio: 4.0,
                max_torque: 40.0,
            },
            PDControllerTarget(ship),
            Transform::from_xyz(0.0, 0.0, 0.0),
            Collider::cuboid(1.0, 1.0, 1.0),
            ColliderDensity(1.0),
        ));

        settle(&mut app);
        // Fly for 45 simulated seconds: approach (~350 m at up to ~20 u/s)
        // plus braking and orbit capture.
        let mut min_distance = f32::INFINITY;
        for _ in 0..2700 {
            app.update();
            let position = app.world().get::<Transform>(ship).unwrap().translation;
            min_distance = min_distance.min(position.distance(player_position));
        }

        // The last simulated second must stay inside a generous band around
        // the standoff range - the old pure pursuit closes to ~zero.
        let mut worst_error = 0.0f32;
        for _ in 0..60 {
            app.update();
            let position = app.world().get::<Transform>(ship).unwrap().translation;
            let error = (position.distance(player_position) - AI_STANDOFF_RANGE).abs();
            worst_error = worst_error.max(error);
        }
        assert!(
            worst_error < AI_STANDOFF_BAND * 2.0,
            "the ship must hold the standoff band (worst error {worst_error} m)"
        );
        assert!(
            min_distance > 100.0,
            "the ship must never dive far inside the envelope \
             (closest approach {min_distance} m)"
        );
    }
}

#[cfg(test)]
mod patrol_physics_tests {
    // The full patrol loop on the physics harness: no hostile -> Patrol ->
    // GotoPos engaged -> the real autopilot swings the hull and burns the
    // real thruster -> the ship physically reaches its first waypoint and
    // turns onto the next leg. Pins the task's acceptance: an AI ship
    // placed in a scenario flies its route before combat starts
    // (task 20260709-225730).
    use super::*;
    use crate::{
        integrity::test_support::{settle, unfinished_integrity_physics_app},
        sections::{
            controller_section::{
                sync_controller_section_forces, update_controller_section_rotation_input,
            },
            thruster_section::thruster_impulse_system,
        },
    };

    #[test]
    fn a_patrol_ship_flies_its_first_leg_and_turns_onto_the_next() {
        let mut app = unfinished_integrity_physics_app();
        app.add_plugins(PDControllerPlugin);
        // The real flight layer: autopilot_system flies what
        // update_passive_flight engages.
        app.add_plugins(NovaFlightPlugin);
        app.configure_sets(
            FixedUpdate,
            (
                super::super::SpaceshipInputSystems,
                NovaFlightSystems,
                PDControllerSystems::Sync,
                SpaceshipSectionSystems,
            )
                .chain(),
        );
        app.add_systems(
            FixedUpdate,
            (
                update_ai_target,
                update_behavior_state,
                update_passive_flight,
            )
                .chain()
                .in_set(super::super::SpaceshipInputSystems),
        );
        app.add_systems(
            FixedUpdate,
            update_controller_section_rotation_input
                .after(NovaFlightSystems)
                .before(PDControllerSystems::Sync),
        );
        app.add_systems(
            FixedUpdate,
            (sync_controller_section_forces, thruster_impulse_system)
                .in_set(SpaceshipSectionSystems),
        );
        app.finish();

        let first = Vec3::new(0.0, 0.0, -300.0);
        let second = Vec3::new(0.0, 0.0, 300.0);
        let ship = app
            .world_mut()
            .spawn((
                RigidBody::Dynamic,
                Transform::default(),
                AISpaceshipMarker,
                AIPatrolRoute::new(vec![first, second]),
            ))
            .id();
        app.world_mut().spawn((
            ChildOf(ship),
            Name::new("hull"),
            Transform::from_xyz(0.0, 0.0, -1.0),
            Collider::cuboid(1.0, 1.0, 1.0),
            ColliderDensity(1.0),
        ));
        app.world_mut().spawn((
            ChildOf(ship),
            Name::new("thruster"),
            ThrusterSectionMarker,
            ThrusterSectionMagnitude(1.0),
            ThrusterSectionInput(0.0),
            Transform::from_xyz(0.0, 0.0, 1.0),
            Collider::cuboid(1.0, 1.0, 1.0),
            ColliderDensity(1.0),
        ));
        app.world_mut().spawn((
            ChildOf(ship),
            Name::new("controller"),
            ControllerSectionMarker,
            ControllerSectionRotationInput::default(),
            PDController {
                frequency: 4.0,
                damping_ratio: 4.0,
                max_torque: 40.0,
            },
            PDControllerTarget(ship),
            Transform::from_xyz(0.0, 0.0, 0.0),
            Collider::cuboid(1.0, 1.0, 1.0),
            ColliderDensity(1.0),
        ));

        settle(&mut app);

        // No hostile anywhere: the routed ship must patrol, not idle.
        app.update();
        assert_eq!(
            *app.world().get::<AIBehaviorState>(ship).unwrap(),
            AIBehaviorState::Patrol
        );

        // Fly until the route turns onto the second leg (arrival at the
        // first waypoint), with a generous budget for align + burn + brake.
        let mut turned_at = None;
        for tick in 0..4800 {
            app.update();
            if app.world().get::<AIPatrolRoute>(ship).unwrap().current == 1 {
                turned_at = Some(tick);
                break;
            }
        }
        assert!(
            turned_at.is_some(),
            "the ship must physically reach its first waypoint and turn \
             onto the next leg within the budget"
        );

        // Still on the routine, and already flying the second leg.
        assert_eq!(
            *app.world().get::<AIBehaviorState>(ship).unwrap(),
            AIBehaviorState::Patrol
        );
        assert_eq!(
            app.world().get::<Autopilot>(ship).map(|ap| ap.action),
            Some(AutopilotAction::GotoPos { position: second }),
        );
    }
}

#[cfg(test)]
mod torpedo_tests {
    use avian3d::collider_tree::ColliderTrees;
    use bevy::ecs::system::RunSystemOnce;

    use super::*;

    #[test]
    fn the_envelope_is_a_range_band_with_rough_alignment() {
        let blast_radius = 30.0; // min range = 30 * 3 = 90 m
        let forward = Vec3::NEG_Z;

        assert!(
            ai_torpedo_envelope(Vec3::NEG_Z * 300.0, forward, blast_radius),
            "in band, dead ahead: launch"
        );
        assert!(
            ai_torpedo_envelope(Vec3::new(100.0, 0.0, -300.0), forward, blast_radius),
            "in band, ~18 degrees off: rough alignment accepts it"
        );
        assert!(
            !ai_torpedo_envelope(Vec3::NEG_Z * 80.0, forward, blast_radius),
            "below the blast-derived minimum: a launch here self-hits"
        );
        assert!(
            !ai_torpedo_envelope(
                Vec3::NEG_Z * (AI_TORPEDO_MAX_RANGE + 1.0),
                forward,
                blast_radius
            ),
            "beyond the outer edge"
        );
        assert!(
            !ai_torpedo_envelope(Vec3::X * 300.0, forward, blast_radius),
            "perpendicular bearing: misaligned"
        );
        assert!(
            !ai_torpedo_envelope(Vec3::ZERO, forward, 0.0),
            "degenerate zero bearing"
        );
    }

    /// An AI ship (at the origin, facing -Z) engaged on a player ship, with
    /// one default-config torpedo bay. Returns (world, ship, target, bay).
    /// The bay's `AITorpedoBay` is NOT armed yet - run
    /// `update_torpedo_section_input` once (the lazy insert) before
    /// asserting on trigger state.
    fn torpedo_world(target_position: Vec3) -> (World, Entity, Entity, Entity) {
        let mut world = World::new();
        world.init_resource::<Time>();
        // Empty collider trees for the launch gate's SpatialQuery: no
        // colliders means a clear line of fire, which is this rig's intent.
        world.init_resource::<ColliderTrees>();
        let target = world
            .spawn((
                SpaceshipRootMarker,
                PlayerSpaceshipMarker,
                Transform::from_translation(target_position),
            ))
            .id();
        let ship = world
            .spawn((
                AISpaceshipMarker,
                AITarget(Some(target)),
                Transform::default(),
            ))
            .id();
        let bay = world
            .spawn((
                torpedo_section(TorpedoSectionConfig::default()),
                ChildOf(ship),
            ))
            .id();
        (world, ship, target, bay)
    }

    /// Arm the bay (lazy-insert pass) and run the trigger write once.
    fn run_trigger(world: &mut World) {
        world.run_system_once(update_torpedo_section_input).unwrap();
        world.run_system_once(update_torpedo_section_input).unwrap();
    }

    #[test]
    fn an_engaged_ship_pulls_the_trigger_inside_the_envelope() {
        // Default blast radius 30 -> min range 90; 300 m dead ahead is in
        // band and aligned, the default state is Engage, the cadence
        // starts elapsed: everything open.
        let (mut world, _, _, bay) = torpedo_world(Vec3::new(0.0, 0.0, -300.0));

        world.run_system_once(update_torpedo_section_input).unwrap();
        assert!(
            world.entity(bay).get::<AITorpedoBay>().is_some(),
            "the bare bay picks its launch state up lazily"
        );
        assert!(
            !**world.entity(bay).get::<TorpedoSectionInput>().unwrap(),
            "the arming pass itself does not fire yet"
        );

        world.run_system_once(update_torpedo_section_input).unwrap();
        assert!(
            **world.entity(bay).get::<TorpedoSectionInput>().unwrap(),
            "envelope open: trigger pulled"
        );
    }

    #[test]
    fn out_of_envelope_releases_a_held_trigger() {
        // Inside the blast-derived minimum: the gate is closed, and a
        // trigger that was held (input true) must be explicitly released.
        let (mut world, _, _, bay) = torpedo_world(Vec3::new(0.0, 0.0, -50.0));
        run_trigger(&mut world);
        **world
            .entity_mut(bay)
            .get_mut::<TorpedoSectionInput>()
            .unwrap() = true;

        world.run_system_once(update_torpedo_section_input).unwrap();

        assert!(
            !**world.entity(bay).get::<TorpedoSectionInput>().unwrap(),
            "below minimum range: released, not skipped"
        );
    }

    #[test]
    fn evade_and_passive_states_hold_torpedoes() {
        for state in [
            AIBehaviorState::Evade,
            AIBehaviorState::Idle,
            AIBehaviorState::Patrol,
        ] {
            let (mut world, ship, _, bay) = torpedo_world(Vec3::new(0.0, 0.0, -300.0));
            *world.entity_mut(ship).get_mut::<AIBehaviorState>().unwrap() = state;

            run_trigger(&mut world);

            assert!(
                !**world.entity(bay).get::<TorpedoSectionInput>().unwrap(),
                "no launch from {state:?}"
            );
        }
    }

    #[test]
    fn torpedo_targets_are_not_worth_a_bay() {
        // Retarget the ship onto a hostile torpedo (a valid AITarget pick
        // when no ships are around): the guns' job, not the bay's.
        let (mut world, ship, _, bay) = torpedo_world(Vec3::new(0.0, 0.0, -300.0));
        let torpedo = world
            .spawn((
                TorpedoProjectileMarker,
                TorpedoTargetChosen,
                Transform::from_translation(Vec3::new(0.0, 0.0, -300.0)),
            ))
            .id();
        **world.entity_mut(ship).get_mut::<AITarget>().unwrap() = Some(torpedo);

        run_trigger(&mut world);

        assert!(
            !**world.entity(bay).get::<TorpedoSectionInput>().unwrap(),
            "a torpedo target does not open the launch envelope"
        );
    }

    #[test]
    fn the_cadence_gates_the_next_launch() {
        let (mut world, _, _, bay) = torpedo_world(Vec3::new(0.0, 0.0, -300.0));
        run_trigger(&mut world);
        assert!(**world.entity(bay).get::<TorpedoSectionInput>().unwrap());

        // A launch happened: the commit system resets the bay's cadence.
        world
            .entity_mut(bay)
            .get_mut::<AITorpedoBay>()
            .unwrap()
            .cooldown
            .reset();

        world.run_system_once(update_torpedo_section_input).unwrap();
        assert!(
            !**world.entity(bay).get::<TorpedoSectionInput>().unwrap(),
            "cadence running: trigger released despite the open envelope"
        );
    }

    #[test]
    fn a_fresh_ai_torpedo_commits_to_the_owner_target_and_burns_the_cadence() {
        let (mut world, ship, target, bay) = torpedo_world(Vec3::new(0.0, 0.0, -300.0));
        run_trigger(&mut world);
        let torpedo = world
            .spawn((
                TorpedoProjectileMarker,
                ProjectileOwner(ship),
                TorpedoSectionPartOf(bay),
            ))
            .id();

        world.run_system_once(update_torpedo_target_input).unwrap();

        assert!(
            world.entity(torpedo).get::<TorpedoTargetChosen>().is_some(),
            "the launch-time decision is made exactly once"
        );
        assert_eq!(
            world
                .entity(torpedo)
                .get::<TorpedoTargetEntity>()
                .map(|t| **t),
            Some(target),
            "committed to the owner's AITarget"
        );
        assert!(
            !world
                .entity(bay)
                .get::<AITorpedoBay>()
                .unwrap()
                .cooldown
                .is_finished(),
            "an actual launch burns the bay's cadence"
        );
    }

    #[test]
    fn an_ai_torpedo_without_a_target_dumb_fires() {
        // The owner lost its target between launch and commit: the torpedo
        // is committed target-less for life, like a player dumb-fire shot.
        let (mut world, ship, _, bay) = torpedo_world(Vec3::new(0.0, 0.0, -300.0));
        run_trigger(&mut world);
        **world.entity_mut(ship).get_mut::<AITarget>().unwrap() = None;
        let torpedo = world
            .spawn((
                TorpedoProjectileMarker,
                ProjectileOwner(ship),
                TorpedoSectionPartOf(bay),
            ))
            .id();

        world.run_system_once(update_torpedo_target_input).unwrap();

        assert!(
            world.entity(torpedo).get::<TorpedoTargetChosen>().is_some(),
            "the decision is still made"
        );
        assert!(
            world.entity(torpedo).get::<TorpedoTargetEntity>().is_none(),
            "no target to commit to: dumb-fire"
        );
    }

    #[test]
    fn player_torpedoes_are_left_to_the_player_commit_system() {
        let (mut world, _, target, bay) = torpedo_world(Vec3::new(0.0, 0.0, -300.0));
        let torpedo = world
            .spawn((
                TorpedoProjectileMarker,
                ProjectileOwner(target),
                TorpedoSectionPartOf(bay),
            ))
            .id();

        world.run_system_once(update_torpedo_target_input).unwrap();

        assert!(
            world.entity(torpedo).get::<TorpedoTargetChosen>().is_none(),
            "a non-AI owner's torpedo is not this system's to commit"
        );
    }

    #[test]
    fn a_torpedo_target_at_commit_time_dumb_fires() {
        // The launch gate requires a ship, but the commit is a frame
        // later: an AITarget that flipped to a hostile torpedo in that
        // frame (ship target died, ordnance in range) must not send this
        // torpedo chasing ordnance.
        let (mut world, ship, _, bay) = torpedo_world(Vec3::new(0.0, 0.0, -300.0));
        run_trigger(&mut world);
        let hostile_torpedo = world
            .spawn((
                TorpedoProjectileMarker,
                TorpedoTargetChosen,
                Transform::from_translation(Vec3::new(0.0, 0.0, -300.0)),
            ))
            .id();
        **world.entity_mut(ship).get_mut::<AITarget>().unwrap() = Some(hostile_torpedo);
        let torpedo = world
            .spawn((
                TorpedoProjectileMarker,
                ProjectileOwner(ship),
                TorpedoSectionPartOf(bay),
            ))
            .id();

        world.run_system_once(update_torpedo_target_input).unwrap();

        assert!(
            world.entity(torpedo).get::<TorpedoTargetChosen>().is_some(),
            "the decision is still made"
        );
        assert!(
            world.entity(torpedo).get::<TorpedoTargetEntity>().is_none(),
            "a non-ship target commits as a dumb-fire shot"
        );
    }

    #[test]
    fn the_trigger_is_per_ship() {
        // Ship A engaged in envelope, ship B (its own bay) with nothing to
        // fight: A's decision must not leak onto B's bay through the
        // shared section query.
        let (mut world, _, _, bay_a) = torpedo_world(Vec3::new(0.0, 0.0, -300.0));
        let ship_b = world
            .spawn((
                AISpaceshipMarker,
                Transform::from_translation(Vec3::new(500.0, 0.0, 0.0)),
            ))
            .id();
        let bay_b = world
            .spawn((
                torpedo_section(TorpedoSectionConfig::default()),
                ChildOf(ship_b),
            ))
            .id();

        run_trigger(&mut world);

        assert!(
            **world.entity(bay_a).get::<TorpedoSectionInput>().unwrap(),
            "ship A: envelope open, trigger pulled"
        );
        assert!(
            !**world.entity(bay_b).get::<TorpedoSectionInput>().unwrap(),
            "ship B has no target: its bay stays released"
        );
    }
}

#[cfg(test)]
mod line_of_fire_tests {
    use bevy::ecs::system::RunSystemOnce;

    use super::*;
    use crate::integrity::test_support::{integrity_physics_app, settle};

    /// A physics app (real avian collider trees, zero gravity, manual
    /// clock) with an engaged AI ship at the origin whose muzzle faces -Z,
    /// and a hostile target body dead ahead. The gate reads the SAME
    /// spatial state production reads; the bare-World rigs in
    /// `fire_discipline_tests` cover the other gates with empty trees.
    fn los_app(target_position: Vec3, target_radius: f32) -> (App, Entity, Entity, Entity) {
        let mut app = integrity_physics_app();
        let target = app
            .world_mut()
            .spawn((
                SpaceshipRootMarker,
                PlayerSpaceshipMarker,
                RigidBody::Dynamic,
                Collider::sphere(target_radius),
                Transform::from_translation(target_position),
                LinearVelocity(Vec3::ZERO),
            ))
            .id();
        let ship = app
            .world_mut()
            .spawn((
                AISpaceshipMarker,
                AITarget(Some(target)),
                RigidBody::Dynamic,
                Collider::sphere(1.0),
                Transform::default(),
                LinearVelocity(Vec3::ZERO),
            ))
            .id();
        let muzzle = app
            .world_mut()
            .spawn((TurretSectionBarrelMuzzleMarker, GlobalTransform::IDENTITY))
            .id();
        let turret = app
            .world_mut()
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
        settle(&mut app);
        (app, ship, turret, target)
    }

    /// A tangible static rock: the production shape of authored cover
    /// (an invulnerable asteroid is a Static body with a plain collider).
    fn spawn_rock(app: &mut App, position: Vec3, radius: f32) -> Entity {
        app.world_mut()
            .spawn((
                RigidBody::Static,
                Collider::sphere(radius),
                Transform::from_translation(position),
            ))
            .id()
    }

    fn fire_decision(app: &mut App, turret: Entity) -> bool {
        app.world_mut()
            .run_system_once(on_projectile_input)
            .unwrap();
        **app
            .world()
            .entity(turret)
            .get::<TurretSectionInput>()
            .unwrap()
    }

    #[test]
    fn cover_between_muzzle_and_target_holds_fire() {
        let (mut app, _, turret, _) = los_app(Vec3::new(0.0, 0.0, -100.0), 1.0);
        let rock = spawn_rock(&mut app, Vec3::new(0.0, 0.0, -50.0), 5.0);
        settle(&mut app);

        assert!(
            !fire_decision(&mut app, turret),
            "a tangible rock on the line of fire must hold the trigger"
        );

        // Delivery guard, same test: with the rock gone the identical shot
        // fires - so the hold above was the rock, not a broken rig.
        app.world_mut().despawn(rock);
        settle(&mut app);
        assert!(
            fire_decision(&mut app, turret),
            "same geometry without the rock must fire"
        );
    }

    #[test]
    fn sensor_volumes_are_not_cover() {
        // A scenario trigger area / blast shell: rounds fly through
        // sensors (despawn_bullet_on_hit skips them), so the gate must
        // not read one as cover - the R1.1 beacon lesson, ray edition.
        let (mut app, _, turret, _) = los_app(Vec3::new(0.0, 0.0, -100.0), 1.0);
        app.world_mut().spawn((
            RigidBody::Static,
            Collider::sphere(20.0),
            Sensor,
            Transform::from_translation(Vec3::new(0.0, 0.0, -50.0)),
        ));
        settle(&mut app);

        assert!(
            fire_decision(&mut app, turret),
            "a sensor volume on the line is transparent: fire"
        );
    }

    #[test]
    fn the_targets_own_hull_is_not_cover() {
        // A big, close target: the ray meets the target's own collider
        // well before the aim anchor. That is the shot LANDING, not
        // occlusion.
        let (mut app, _, turret, _) = los_app(Vec3::new(0.0, 0.0, -100.0), 30.0);

        assert!(
            fire_decision(&mut app, turret),
            "hitting the target's own collider first is a hit, not cover"
        );
    }

    #[test]
    fn point_defense_fires_through_cover() {
        // Inbound ordnance behind a rock: PD is exempt from the gate -
        // ordnance hunting THIS ship closes the line itself, and a held
        // trigger is worse than a wasted round.
        let (mut app, ship, turret, _) = los_app(Vec3::new(0.0, 0.0, -100.0), 1.0);
        let torpedo = app
            .world_mut()
            .spawn((Transform::from_translation(Vec3::new(0.0, 0.0, -100.0)),))
            .id();
        app.world_mut()
            .entity_mut(ship)
            .insert(AIPointDefenseTarget(Some(torpedo)));
        spawn_rock(&mut app, Vec3::new(0.0, 0.0, -50.0), 5.0);
        settle(&mut app);

        assert!(
            fire_decision(&mut app, turret),
            "point defense bypasses the line-of-fire gate"
        );
    }

    #[test]
    fn cover_holds_the_torpedo_launch() {
        // Same geometry as the trigger tests, torpedo side: in-envelope
        // target (default blast 30 -> min range 90; 300 m dead ahead),
        // rock on the bearing - the bay must not spend ordnance on cover.
        let (mut app, ship, _, _) = los_app(Vec3::new(0.0, 0.0, -300.0), 1.0);
        let bay = app
            .world_mut()
            .spawn((
                torpedo_section(TorpedoSectionConfig::default()),
                AITorpedoBay::default(),
                ChildOf(ship),
            ))
            .id();
        let rock = spawn_rock(&mut app, Vec3::new(0.0, 0.0, -150.0), 10.0);
        settle(&mut app);

        app.world_mut()
            .run_system_once(update_torpedo_section_input)
            .unwrap();
        assert!(
            !**app
                .world()
                .entity(bay)
                .get::<TorpedoSectionInput>()
                .unwrap(),
            "rock on the launch bearing: hold the torpedo"
        );

        // Delivery guard, same test: the identical launch with the rock
        // gone goes out (the cadence only resets on an actual launch, so
        // it is still elapsed here).
        app.world_mut().despawn(rock);
        settle(&mut app);
        app.world_mut()
            .run_system_once(update_torpedo_section_input)
            .unwrap();
        assert!(
            **app
                .world()
                .entity(bay)
                .get::<TorpedoSectionInput>()
                .unwrap(),
            "same envelope without the rock: launch"
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
            // grace's first job is demoting it onto the routine (R1.1).
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
