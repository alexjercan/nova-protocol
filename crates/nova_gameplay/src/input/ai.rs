use avian3d::prelude::*;
use bevy::prelude::*;

use crate::prelude::*;

pub mod prelude {
    pub use super::{
        AIBehaviorState, AIFireCadence, AIPointDefenseTarget, AISpaceshipMarker, AITarget,
        SpaceshipAIInputPlugin,
    };
}

pub struct SpaceshipAIInputPlugin;

impl Plugin for SpaceshipAIInputPlugin {
    fn build(&self, app: &mut App) {
        debug!("SpaceshipAIInputPlugin: build");

        app.register_type::<AIBehaviorState>();
        app.register_type::<AITarget>();
        app.register_type::<AIFireCadence>();
        app.register_type::<AIPointDefenseTarget>();

        app.add_systems(
            Update,
            (
                update_ai_target,
                update_point_defense_target,
                update_behavior_state,
                update_fire_cadence,
                update_controller_target_rotation_torque,
                on_thruster_input,
                update_turret_target_input,
                on_projectile_input,
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
    AIFireCadence
)]
pub struct AISpaceshipMarker;

/// The entity this AI ship currently fights - what every AI behavior system
/// aims, chases and shoots at. Written by [`update_ai_target`] from the
/// relation model (task 20260709-225727); `None` means nothing hostile in
/// acquisition range, which [`update_behavior_state`] turns into `Idle`.
#[derive(Component, Debug, Clone, Copy, Default, PartialEq, Eq, Deref, DerefMut, Reflect)]
#[reflect(Component)]
pub struct AITarget(pub Option<Entity>);

/// What kind of body a target candidate is. Priority TIER, not a score
/// tweak: hostile ships always beat hostile torpedoes (the urgency flip for
/// an incoming torpedo is the point-defense task, 20260709-225733).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum AITargetKind {
    Ship,
    Torpedo,
}

/// Acquisition range (m) of AI target selection, matching the player's
/// TARGETING_MAX_RANGE.
const AI_TARGET_MAX_RANGE: f32 = 2000.0;
/// Switch hysteresis: the current target's distance is discounted by this
/// factor, so a rival has to be meaningfully closer (not a frame-noise
/// sliver) to steal the pick.
const AI_TARGET_HYSTERESIS_DISCOUNT: f32 = 0.8;

/// Choose the best target from `candidates`: highest priority tier first
/// ([`AITargetKind`] order), nearest within the tier, with the current
/// target's distance discounted by [`AI_TARGET_HYSTERESIS_DISCOUNT`] so the
/// pick does not flip-flop between two comparably distant hostiles. Out of
/// [`AI_TARGET_MAX_RANGE`] (or with no candidates) the pick is `None`.
/// Pure for unit testing.
fn pick_ai_target(
    own_anchor: Vec3,
    current: Option<Entity>,
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
            &mut AITarget,
        ),
        (With<SpaceshipRootMarker>, With<AISpaceshipMarker>),
    >,
) {
    for (ship, transform, com, own_allegiance, mut target) in &mut q_spaceship {
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

        let next = pick_ai_target(own_anchor, **target, candidates);
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

/// What an AI ship is currently doing - the state skeleton of the AI combat
/// arc (docs/spikes/20260709-225508-ai-combat-behaviors.md). One state per
/// ship root, driven by [`update_behavior_state`]; every AI system gates its
/// behavior on it.
///
/// Only `Engage` and `Idle` have real behavior today. The others exist so
/// their tasks slot into a stable enum instead of reshaping it:
/// - `Patrol`: waypoint flight, task 20260709-225730 (behaves as `Idle`).
/// - `Evade`: under-fire jinking, task 20260709-225731 (stubs to `Engage`).
/// - `Retreat`: low-integrity disengage, task 20260709-225734 (stubs to
///   `Engage`).
#[derive(Component, Debug, Clone, Copy, Default, PartialEq, Eq, Reflect)]
#[reflect(Component)]
pub enum AIBehaviorState {
    /// Station-keeping: no thrust, no fire, frozen helm.
    Idle,
    /// Waypoint flight (20260709-225730); behaves as `Idle` until then.
    Patrol,
    /// Chase and shoot the hostile - today's whole AI, and the default so
    /// an AI ship dropped into a fight behaves exactly as before the state
    /// machine existed.
    #[default]
    Engage,
    /// Under-fire evasion (20260709-225731); stubs to `Engage` until then.
    Evade,
    /// Low-integrity disengage (20260709-225734); stubs to `Engage` until
    /// then.
    Retreat,
}

impl AIBehaviorState {
    /// Whether this state runs the engage-style chase/aim/fire pipeline.
    /// `Evade` and `Retreat` deliberately stub to Engage behavior until
    /// their tasks land (see the variant docs).
    fn engages(&self) -> bool {
        matches!(self, Self::Engage | Self::Evade | Self::Retreat)
    }
}

/// The skeleton's one real transition: combat states need a hostile to
/// fight - with none in the world every state falls back to `Idle`, and a
/// hostile appearing pulls the passive states into `Engage`. Detection
/// RANGE (engage only when close enough) is the patrol task's scope
/// (20260709-225730); presence-based engagement matches today's
/// always-chase behavior. Pure for unit testing.
fn next_behavior_state(current: AIBehaviorState, hostile_present: bool) -> AIBehaviorState {
    if !hostile_present {
        return AIBehaviorState::Idle;
    }
    match current {
        // A hostile appeared: the passive states pick the fight up.
        AIBehaviorState::Idle | AIBehaviorState::Patrol => AIBehaviorState::Engage,
        // Combat states hold; their exit triggers are their tasks' scope.
        state => state,
    }
}

/// Drive each AI ship's [`AIBehaviorState`] from its [`AITarget`]. Runs
/// after acquisition and before the behavior systems in the same frame so a
/// transition takes effect immediately (no one-frame stale-state window).
fn update_behavior_state(
    mut q_spaceship: Query<(&mut AIBehaviorState, &AITarget), With<AISpaceshipMarker>>,
) {
    for (mut state, target) in &mut q_spaceship {
        let next = next_behavior_state(*state, target.is_some());
        // Change-detection hygiene: only write on a real transition.
        if *state != next {
            *state = next;
        }
    }
}

// AI "brain" tuning constants. The AI chases the player at a speed that scales with
// distance (so it slows as it closes in) and brakes when it overshoots.
/// Target chase speed per unit of distance to the player.
const AI_CHASE_SPEED_GAIN: f32 = 0.2;
/// Lower/upper clamp on the distance-scaled chase speed.
const AI_MIN_CHASE_SPEED: f32 = 2.0;
const AI_MAX_CHASE_SPEED: f32 = 20.0;
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

/// The direction an AI ship should face: toward its target while it is slower than its
/// distance-scaled target speed, or opposite its velocity when overshooting (braking).
/// Falls back to facing the target if the computed direction degenerates to zero.
fn ai_desired_direction(to_target: Vec3, velocity: Vec3) -> Vec3 {
    let target_speed =
        (to_target.length() * AI_CHASE_SPEED_GAIN).clamp(AI_MIN_CHASE_SPEED, AI_MAX_CHASE_SPEED);
    let too_fast = velocity.length() > target_speed + AI_BRAKE_SPEED_MARGIN;

    let desired = if too_fast {
        // Brake: point opposite the current velocity.
        -velocity.normalize_or_zero()
    } else {
        // Chase: point toward the target.
        to_target.normalize()
    };

    if desired.length_squared() == 0.0 {
        to_target.normalize_or_zero()
    } else {
        desired
    }
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
        ),
        (With<SpaceshipRootMarker>, With<AISpaceshipMarker>),
    >,
    q_target: Query<(&Transform, Option<&ComputedCenterOfMass>)>,
) {
    for (entity, transform, velocity, inertia, com, state, target) in &q_spaceship {
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
        let desired_direction = ai_desired_direction(to_target, **velocity);

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
        ),
        (With<SpaceshipRootMarker>, With<AISpaceshipMarker>),
    >,
    q_target: Query<(&Transform, Option<&ComputedCenterOfMass>)>,
) {
    for (entity, transform, velocity, com, state, target) in &q_spaceship {
        // A non-engaging state (or no target left to chase) cuts the burn -
        // written as an explicit 0.0, not a skip, so a ship that was
        // thrusting when the state flipped actually stops.
        let thrust_level = match ai_target_anchor(**target, &q_target) {
            Some(target_anchor) if state.engages() => {
                // Same live-structure vector as the rotation system, so the
                // thrust gate and the rotation command agree on where
                // "toward the target" is.
                let to_target = target_anchor - live_structure_anchor(transform, com);
                let desired_direction = ai_desired_direction(to_target, **velocity);

                // Thrust only when the ship is pointing roughly toward the
                // desired direction.
                let forward = transform.forward();
                let alignment = forward.dot(desired_direction);
                if alignment > AI_THRUST_ALIGNMENT {
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
            **input = alignment > AI_FIRE_ALIGNMENT;
        }
    }
}

#[cfg(test)]
mod behavior_state_tests {
    use bevy::ecs::system::RunSystemOnce;

    use super::*;

    #[test]
    fn transitions_need_a_hostile_to_fight() {
        use AIBehaviorState::*;

        // No hostile: every state falls back to Idle.
        for state in [Idle, Patrol, Engage, Evade, Retreat] {
            assert_eq!(next_behavior_state(state, false), Idle, "from {state:?}");
        }
        // Hostile present: passive states engage, combat states hold (their
        // exit triggers belong to their own tasks).
        assert_eq!(next_behavior_state(Idle, true), Engage);
        assert_eq!(next_behavior_state(Patrol, true), Engage);
        assert_eq!(next_behavior_state(Engage, true), Engage);
        assert_eq!(next_behavior_state(Evade, true), Evade);
        assert_eq!(next_behavior_state(Retreat, true), Retreat);
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
mod tests {
    use bevy::ecs::system::RunSystemOnce;

    use super::*;

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

        app.world_mut().spawn((
            SpaceshipRootMarker,
            PlayerSpaceshipMarker,
            Transform::from_translation(Vec3::new(0.0, 0.0, 100.0)),
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

        // Player abeam at +X: a 90-degree swing from the AI's initial -Z.
        app.world_mut().spawn((
            SpaceshipRootMarker,
            PlayerSpaceshipMarker,
            Transform::from_translation(Vec3::new(200.0, 0.0, 0.0)),
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
        // The aim axes are quiet; what residual spin remains is pure ROLL
        // about the nose, which the bcs PD cannot damp (open bug
        // 20260709-125640, amplitude ~0.23 rad/s in this rig). Bound it so a
        // regression in THIS path still trips, and tighten toward ~0 when
        // the bcs fix lands.
        assert!(
            max_spin < 0.5,
            "residual spin must stay within the known roll-damping bound \
             (20260709-125640), got {max_spin} rad/s"
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
                [(entity(1), Vec3::new(0.0, 0.0, -2500.0), AITargetKind::Ship)].into_iter(),
            ),
            None,
            "beyond acquisition range"
        );
        assert_eq!(
            pick_ai_target(Vec3::ZERO, None, std::iter::empty()),
            None,
            "no candidates"
        );
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
    use bevy::ecs::system::RunSystemOnce;

    use super::*;

    /// An AI ship engaged on a hand-set target, with one turret whose
    /// muzzle sits at the origin facing -Z. Returns (world, turret, muzzle).
    fn firing_world(target_position: Vec3, target_velocity: Vec3) -> (World, Entity, Entity) {
        let mut world = World::new();
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
