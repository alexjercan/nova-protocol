//! The player's target lock: angular aim-assist acquisition and the shared
//! lock resource every targeting consumer reads (torpedo launches, the HUD
//! reticle/readout, and - as of the component-lock arc - auto-mode turrets).
//!
//! Extracted from input/player.rs (task 20260709-192503); the acquisition
//! rule lives in pure helpers so it stays unit-testable.

use avian3d::prelude::*;
use bevy::prelude::*;
use bevy_common_systems::prelude::*;
use bevy_enhanced_input::prelude::*;

use crate::prelude::*;

pub mod prelude {
    pub use super::{
        ComponentLockMode, LockSignature, SpaceshipPlayerComponentLock, SpaceshipPlayerLockFocus,
        SpaceshipPlayerTargetCandidates, SpaceshipPlayerTargetLock, SpaceshipTargetingPlugin,
        SpaceshipTargetingSystems, TargetingSettings,
    };
}

/// How strongly the lock scanner "sees" a body, a radius-like magnitude in
/// world units (user request 2026-07-10: think of the lock as a scanner
/// wave - small objects return no signature at range). A candidate without
/// this component and without an intrinsic class (well body, ship,
/// committed torpedo) is only lockable point-blank
/// ([`TargetingSettings::unsigned_lock_range`]); with it, lock range scales
/// as `signature * signature_range_per_unit`. The scenario layer authors it
/// on asteroids from their radius.
#[derive(Component, Clone, Copy, Debug, Deref, DerefMut, Reflect)]
#[reflect(Component)]
pub struct LockSignature(pub f32);

/// Lock-acquisition tunables, reflected for the inspector.
#[derive(Resource, Clone, Debug, Reflect)]
#[reflect(Resource)]
pub struct TargetingSettings {
    /// Lock range per unit of [`LockSignature`], world units. At 30, a 2u
    /// field rock is lockable within 60u - close enough to matter, far
    /// enough not to steal mid-fight locks.
    pub signature_range_per_unit: f32,
    /// Lock range for bodies with no signature and no intrinsic class -
    /// battle debris, loose fragments. Point-blank by design.
    pub unsigned_lock_range: f32,
    /// The incumbent lock stays lockable this factor beyond its gate, so
    /// a body at its boundary cannot strobe the lock (and reset the focus
    /// dwell) as the ship drifts. Fresh acquisition still uses the plain
    /// gate.
    pub range_hysteresis: f32,
    /// Lock range for committed torpedoes. Small object, hot drive: far
    /// more visible than its size but not across the map. Covers every
    /// real point-defense engagement (AI launch range 1000u, heat
    /// fallback 550u) with margin; a playtest knob.
    pub torpedo_lock_range: f32,
}

impl Default for TargetingSettings {
    fn default() -> Self {
        Self {
            signature_range_per_unit: 30.0,
            unsigned_lock_range: 15.0,
            range_hysteresis: 1.15,
            torpedo_lock_range: 2500.0,
        }
    }
}

/// The player's current target lock. `None` means no lock (reticle hidden,
/// torpedoes dumb-fire). Torpedo launches, the HUD and turret auto-fire all
/// consume this one resource.
#[derive(Resource, Debug, Clone, Deref, DerefMut, Default)]
pub struct SpaceshipPlayerTargetLock(pub Option<Entity>);

/// System set for the lock update, so consumers (torpedo commit, turret
/// feed) can order after it.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct SpaceshipTargetingSystems;

/// Plugin owning the lock resource and its per-frame acquisition.
pub struct SpaceshipTargetingPlugin;

impl Plugin for SpaceshipTargetingPlugin {
    fn build(&self, app: &mut App) {
        debug!("SpaceshipTargetingPlugin: build");

        app.init_resource::<TargetingSettings>();
        app.register_type::<TargetingSettings>();
        app.register_type::<LockSignature>();

        app.insert_resource(SpaceshipPlayerTargetLock::default());
        app.insert_resource(SpaceshipPlayerTargetCandidates::default());
        app.insert_resource(SpaceshipPlayerLockFocus::default());
        app.insert_resource(SpaceshipPlayerComponentLock::default());
        app.add_systems(
            Update,
            (
                update_spaceship_target_input,
                tick_lock_focus,
                update_component_lock,
            )
                .chain()
                .in_set(SpaceshipTargetingSystems)
                .in_set(super::SpaceshipInputSystems),
        );
        app.add_observer(on_component_cycle_next);
        app.add_observer(on_component_cycle_prev);
        app.add_observer(on_target_cycle_next);
        app.add_observer(on_target_cycle_prev);
    }
}

/// Maximum distance at which the aim-assist will lock a target - the
/// ceiling for the intrinsic classes (well bodies, ships), which stay
/// designatable from across the play area for GOTO legs (user report
/// 20260710). Everything else is gated far shorter by the signature model
/// (the later 20260710 report: long-range locks should only see large
/// objects), see [`LockSignature`] and [`TargetingSettings`].
const TARGETING_MAX_RANGE: f32 = 20_000.0;

/// Half-angle (degrees) of the lock-on cone around the aim direction. Any lockable
/// body whose bearing from the ship falls within this angle of where the player is
/// aiming is eligible, and the one closest to the aim ray wins. This is the whole
/// point of the aim-assist: a wide cone means the player only has to point roughly
/// at a target instead of landing a pixel-perfect ray on it. Pan the view and the
/// lock snaps to whichever eligible body is now nearest the center, so cycling
/// between targets is just "look at the next one".
const TARGETING_CONE_HALF_ANGLE_DEG: f32 = 18.0;

/// Seconds of continuous lock on the same target before the component layer
/// unlocks (the WoT-style aim-in dwell from the component-lock spike,
/// docs/spikes/20260709-192358-component-lock-vats-lite.md).
const FOCUS_TIME: f32 = 1.5;

/// Seconds a cycle press pins the component selection before aim-snap
/// resumes. A feel knob; tune in playtest.
const COMPONENT_PIN_WINDOW: f32 = 2.0;

/// Snap hysteresis: a challenger section only steals the fine lock when its
/// ray distance is below this fraction of the incumbent's, so the selection
/// does not flicker between adjacent sections. A feel knob; tune in playtest.
const SNAP_HYSTERESIS: f32 = 0.75;

/// Focus: how long the current lock has been held on the same target.
/// Component fine-locking unlocks at [`FOCUS_TIME`]; the HUD renders the
/// fill fraction while it accumulates.
#[derive(Resource, Debug, Clone, PartialEq, Default)]
pub struct SpaceshipPlayerLockFocus {
    /// The target the timer is accumulating on (mirrors the lock).
    pub target: Option<Entity>,
    /// Continuous seconds the lock has stayed on `target`.
    pub seconds: f32,
}

impl SpaceshipPlayerLockFocus {
    /// Focus completion in [0, 1], for the HUD meter.
    pub fn fraction(&self) -> f32 {
        (self.seconds / FOCUS_TIME).clamp(0.0, 1.0)
    }

    /// Whether the component layer is unlocked for `target`.
    pub fn focused_on(&self, target: Entity) -> bool {
        self.target == Some(target) && self.seconds >= FOCUS_TIME
    }
}

/// How the component fine-lock is currently selected.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum ComponentLockMode {
    /// Follow the live section nearest the crosshair ray (with hysteresis).
    #[default]
    Snap,
    /// A cycle press chose deliberately; snap is suppressed until `until`
    /// (Time::elapsed_secs) or until the pinned section dies.
    Pinned {
        /// Elapsed-time deadline after which snap resumes.
        until: f32,
    },
}

/// The fine-locked section of the locked ship, only ever `Some` while the
/// focus dwell is complete. Sections stay lockable while ATTACHED - a
/// disabled-in-place section (`SectionInactiveMarker`) can still be targeted
/// to blow it off the hull; despawn/detach clears the selection (decision
/// from the component-lock spike, lockable-while-attached).
#[derive(Resource, Debug, Clone, PartialEq, Default)]
pub struct SpaceshipPlayerComponentLock {
    /// The fine-locked section entity (a `SectionMarker` child of the lock).
    pub section: Option<Entity>,
    /// Snap or pinned-by-cycle selection.
    pub mode: ComponentLockMode,
}

/// Cycle the component fine-lock to the next section (stable order).
#[derive(InputAction)]
#[action_output(bool)]
pub(crate) struct ComponentCycleNextInput;

/// Cycle the component fine-lock to the previous section (stable order).
#[derive(InputAction)]
#[action_output(bool)]
pub(crate) struct ComponentCyclePrevInput;

/// Cycle the ship lock to the next tracked candidate (ranked order).
#[derive(InputAction)]
#[action_output(bool)]
pub(crate) struct TargetCycleNextInput;

/// Cycle the ship lock to the previous tracked candidate (ranked order).
#[derive(InputAction)]
#[action_output(bool)]
pub(crate) struct TargetCyclePrevInput;

/// Held modifier that layers the cycle gesture one level up: while it fires,
/// the wheel (and brackets) cycle the SHIP lock instead of components (spike
/// docs/spikes/20260711-163800-multi-target-cycle.md).
#[derive(InputAction)]
#[action_output(bool)]
pub(crate) struct TargetCycleModifierInput;

/// How many candidate ships the tracker keeps for the HUD list and the cycle
/// input. A feel knob; more would clutter the HUD.
const TARGET_CANDIDATE_COUNT: usize = 5;

/// Seconds a cycle press pins the ship lock against the aim-driven picker.
/// Longer than [`COMPONENT_PIN_WINDOW`]: re-aiming a whole ship is slower
/// than re-snapping a section. A feel knob; tune in playtest.
const TARGET_PIN_WINDOW: f32 = 4.0;

/// The maintained multi-target set: the top lockable hostile ships, ranked
/// by angle to the aim ray then distance (best first), rebuilt each frame by
/// the acquisition pass. The candidate HUD renders it, the target-cycle
/// input walks it, and the edge-indicator overlay points at its off-screen
/// members (spike docs/spikes/20260711-163800-multi-target-cycle.md).
#[derive(Resource, Debug, Clone, PartialEq, Default)]
pub struct SpaceshipPlayerTargetCandidates {
    /// Ranked candidate ship roots, best first - also the cycle order.
    pub entries: Vec<Entity>,
    /// While `Some`, a cycle press pinned the lock until this elapsed-time
    /// deadline: the aim-driven picker must not overwrite it, and `entries`
    /// keeps a stable order (membership still updates) so repeated presses
    /// walk a list that is not reshuffling under the player.
    pub pinned_until: Option<f32>,
}

/// Range (m) of the close-in "signature" auto-acquisition: with nothing in
/// the aim cone, the nearest hostile inside this range locks by itself, as if
/// the ship's sensors picked up its heat signature. Deliberately well inside
/// [`TARGETING_MAX_RANGE`], so long-range designation stays aim-driven
/// (decided in docs/spikes/20260709-192358-component-lock-vats-lite.md).
const TARGETING_SIGNATURE_RANGE: f32 = 550.0;

/// Choose the closest hostile within `max_range` of `origin` - the signature
/// fallback used when the aim cone is empty. Candidates carry an
/// `is_hostile` flag (resolved by the relation model: [`relation`] vs the
/// player is [`Relation::Hostile`]); non-hostiles are never auto-acquired,
/// so asteroids, neutrals and stray torpedoes do not steal the lock.
///
/// Pure and camera/physics-free so the selection rule can be unit-tested
/// directly.
fn pick_signature_target(
    origin: Vec3,
    max_range: f32,
    candidates: impl Iterator<Item = (Entity, Vec3, bool)>,
) -> Option<Entity> {
    candidates
        .filter_map(|(entity, position, is_hostile)| {
            if !is_hostile {
                return None;
            }
            let distance = origin.distance(position);
            (distance <= max_range && distance > f32::EPSILON).then_some((entity, distance))
        })
        .min_by(|(_, a), (_, b)| a.total_cmp(b))
        .map(|(entity, _)| entity)
}

/// Choose the best lock-on target from `candidates` (each an entity and its world
/// position): the one whose bearing from `origin` is closest to the `aim`
/// direction, as long as it is within `max_range` and inside the cone (bearing
/// dot aim `>= min_cos`, i.e. `min_cos = cos(half_angle)`). Returns `None` when
/// nothing qualifies - e.g. the player is looking at empty space - which drops the
/// lock and hides the reticle.
///
/// Pure and camera/physics-free so the selection rule can be unit-tested directly.
fn pick_target(
    origin: Vec3,
    aim: Vec3,
    max_range: f32,
    min_cos: f32,
    candidates: impl Iterator<Item = (Entity, Vec3)>,
) -> Option<Entity> {
    candidates
        .filter_map(|(entity, position)| {
            let to_target = position - origin;
            let distance = to_target.length();
            if distance > max_range || distance < f32::EPSILON {
                return None;
            }
            let cos_angle = to_target.normalize().dot(aim);
            (cos_angle >= min_cos).then_some((entity, cos_angle))
        })
        // Largest cosine == smallest angle from the aim ray == closest to center.
        .max_by(|(_, a), (_, b)| a.total_cmp(b))
        .map(|(entity, _)| entity)
}

/// Update the player's target lock, hybrid-style: angular aim-assist first
/// (enumerate the physical bodies in front of the ship and lock the one
/// nearest the aim direction, see [`pick_target`]), and with an empty cone
/// the signature fallback (nearest hostile inside
/// [`TARGETING_SIGNATURE_RANGE`], see [`pick_signature_target`]).
fn update_spaceship_target_input(
    point_rotation: Single<
        &PointRotationOutput,
        (
            With<SpaceshipCameraInputMarker>,
            With<SpaceshipCameraTurretInputMarker>,
        ),
    >,
    // Turret bullets are excluded outright: they are dynamic bodies that stream
    // straight down the aim ray, so without this the lock would constantly snap
    // onto the player's own gunfire instead of the enemy behind it.
    q_candidates: Query<
        (
            Entity,
            &GlobalTransform,
            &RigidBody,
            Option<&GravityWell>,
            Option<&LockSignature>,
            Has<SpaceshipRootMarker>,
            Option<&TorpedoProjectileMarker>,
            Option<&TorpedoTargetChosen>,
            Option<&Allegiance>,
        ),
        Without<TurretBulletProjectileMarker>,
    >,
    settings: Res<TargetingSettings>,
    spaceship: Single<
        (
            &Transform,
            Option<&ComputedCenterOfMass>,
            Entity,
            Option<&Allegiance>,
        ),
        (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>),
    >,
    time: Res<Time>,
    mut res_target: ResMut<SpaceshipPlayerTargetLock>,
    mut res_candidates: ResMut<SpaceshipPlayerTargetCandidates>,
) {
    let point_rotation = point_rotation.into_inner();
    let (ship_transform, ship_com, ship_entity, ship_allegiance) = spaceship.into_inner();

    // Cone origin on the live structure, not the root origin, so the lock
    // cone agrees with the COM-anchored camera crosshair after losing
    // sections (task 20260709-150711).
    let origin = live_structure_anchor(ship_transform, ship_com);
    let aim = (**point_rotation * Vec3::NEG_Z).normalize();
    let min_cos = TARGETING_CONE_HALF_ANGLE_DEG.to_radians().cos();

    // Collected once because both pickers walk it (the cone pick first, then
    // the signature fallback), and the multi-target tracker filters it.
    let candidates: Vec<(Entity, Vec3, bool, bool)> = q_candidates
        .iter()
        .filter_map(
            |(
                entity,
                transform,
                rigid_body,
                well,
                signature,
                is_ship,
                is_torpedo,
                torpedo_committed,
                allegiance,
            )| {
                // Only physical, movable bodies are lockable. This skips static sensor
                // volumes such as scenario trigger areas (`RigidBody::Static`), which
                // are invisible and must never be locked. Two exceptions sit on rails
                // (Static) yet are visible things the player navigates by, so they stay
                // lockable: gravity-well sources (big rocks the player GOTOs and orbits,
                // Static since the gravity task) and bodies with an AUTHORED
                // LockSignature (nav beacons, task 20260712-093044) - trigger areas
                // never carry a signature, so the invisible-statics rule holds.
                if !matches!(rigid_body, RigidBody::Dynamic)
                    && well.is_none()
                    && signature.is_none()
                {
                    return None;
                }
                // Never lock the player's own ship.
                if entity == ship_entity {
                    return None;
                }
                // Skip a freshly launched torpedo that has not committed its
                // launch-time target yet: it spawns right on the aim ray and would
                // otherwise be picked as its own target. Once committed
                // (`TorpedoTargetChosen`) a torpedo is a normal lockable body - e.g.
                // you can lock and shoot down your own dumb-fired torpedo.
                if is_torpedo.is_some() && torpedo_committed.is_none() {
                    return None;
                }
                // The scanner-wave gate: how far away this body can be
                // locked from. Well bodies and ships (ships deferred to
                // the sensor task 20260710-195953) return a signature at
                // any range; committed torpedoes are small but hot
                // (point defense needs them at combat range, not across
                // the map); everything else is gated by its authored
                // LockSignature - floored at the debris range, so an
                // authored signature can never make a body stealthier
                // than none - and unsigned bodies, debris, only
                // point-blank. Gated here, at collection, so the cone
                // pick and the heat-signature fallback both inherit it.
                let mut max_range = if well.is_some() || is_ship {
                    TARGETING_MAX_RANGE
                } else if is_torpedo.is_some() {
                    settings.torpedo_lock_range
                } else {
                    signature.map_or(settings.unsigned_lock_range, |signature| {
                        (settings.signature_range_per_unit * **signature)
                            .max(settings.unsigned_lock_range)
                    })
                };
                // The incumbent lock holds a little beyond its gate, so a
                // body at the boundary cannot strobe the lock and reset
                // the focus dwell as the ship drifts.
                if **res_target == Some(entity) {
                    max_range *= settings.range_hysteresis.max(1.0);
                }
                let position = transform.translation();
                if position.distance_squared(origin) > max_range * max_range {
                    return None;
                }
                // Hostility comes from the relation model, so an enemy's
                // torpedo is auto-acquirable while the player's own is not.
                let is_hostile = relation(ship_allegiance, allegiance) == Relation::Hostile;
                Some((entity, position, is_hostile, is_ship))
            },
        )
        .collect();

    // A cycle press pinned the lock: the aim-driven picker stands down while
    // the window is open and the pinned target is still collectible (range
    // gates above, with the incumbent hysteresis). Expiry or target loss
    // hands the lock back to the picker.
    let pinned = match res_candidates.pinned_until {
        Some(until) => {
            let holds = time.elapsed_secs() < until
                && res_target
                    .is_some_and(|target| candidates.iter().any(|&(entity, ..)| entity == target));
            if !holds {
                res_candidates.pinned_until = None;
            }
            holds
        }
        None => false,
    };

    // Sticky-from-acquisition, SHIP LOCKS ONLY (task 20260712-203353, review
    // R1.1): once a SHIP is locked, the aim picker stands down while it is still
    // a collectible candidate (in range, with the incumbent hysteresis applied
    // above), so a body crossing the aim ray (a passing torpedo, another ship)
    // no longer steals the combat lock or resets the focus dwell. Deliberate
    // switches go through the CTRL+scroll cycle (`step_target_lock`); torpedoes
    // stay lockable for point defense (spike 20260712-203235, option B5).
    //
    // NON-ship locks (asteroids, beacons) are NOT sticky: the lock doubles as
    // the GOTO/torpedo nav designator, and you re-designate a nav target by
    // aiming at it - CTRL+scroll only cycles hostile ships, so a sticky nav lock
    // could not be switched off by aiming (review R1.1). Only the `is_ship` flag
    // in the candidate tuple makes a lock sticky.
    let held = res_target.is_some_and(|target| {
        candidates
            .iter()
            .any(|&(entity, _, _, is_ship)| entity == target && is_ship)
    });

    if !pinned && !held {
        // Aiming designates as always; with an empty cone the nearest hostile
        // inside the signature range auto-acquires (the close-in
        // heat-signature lock from the component-lock spike).
        let cone_pick = pick_target(
            origin,
            aim,
            TARGETING_MAX_RANGE,
            min_cos,
            candidates
                .iter()
                .map(|&(entity, position, ..)| (entity, position)),
        );
        **res_target = cone_pick.or_else(|| {
            pick_signature_target(
                origin,
                TARGETING_SIGNATURE_RANGE,
                candidates
                    .iter()
                    .map(|&(entity, position, is_hostile, _)| (entity, position, is_hostile)),
            )
        });
    }

    // Maintain the multi-target set: hostile ships from the same range-gated
    // collection, ranked toward the aim ray.
    let ranked = rank_ship_candidates(
        origin,
        aim,
        candidates
            .iter()
            .filter(|&&(_, _, is_hostile, is_ship)| is_hostile && is_ship)
            .map(|&(entity, position, ..)| (entity, position)),
    );
    let entries = maintain_candidates(&res_candidates.entries, &ranked, **res_target, pinned);
    if res_candidates.entries != entries {
        res_candidates.entries = entries;
    }
}

/// Rank the lockable hostile ships for the multi-target set: nearest the aim
/// ray first (largest cosine), distance as the tie-breaker. Not cone-gated -
/// a hostile behind the player is still tracked (the edge-indicator overlay
/// points at it); the cone only decides the aim PICK, not the SET.
///
/// Pure and camera/physics-free so the ranking rule can be unit-tested.
fn rank_ship_candidates(
    origin: Vec3,
    aim: Vec3,
    ships: impl Iterator<Item = (Entity, Vec3)>,
) -> Vec<Entity> {
    let mut scored: Vec<(Entity, f32, f32)> = ships
        .filter_map(|(entity, position)| {
            let to_ship = position - origin;
            let distance = to_ship.length();
            (distance > f32::EPSILON).then(|| (entity, to_ship.normalize().dot(aim), distance))
        })
        .collect();
    scored.sort_by(|(_, cos_a, dist_a), (_, cos_b, dist_b)| {
        cos_b.total_cmp(cos_a).then(dist_a.total_cmp(dist_b))
    });
    scored.into_iter().map(|(entity, ..)| entity).collect()
}

/// Compose this frame's candidate entries from the ranked pool.
///
/// Unpinned: the top [`TARGET_CANDIDATE_COUNT`] by rank. Pinned: membership
/// still updates (dead or out-of-range ships drop, newcomers append by rank)
/// but survivors keep their relative order, so a cycle in progress walks a
/// stable list (the spike's frozen-snapshot rule). Either way the current
/// lock stays a member while it is still a ranked ship, even when it falls
/// out of the top N - the reticle target must never vanish from its own list.
fn maintain_candidates(
    prev: &[Entity],
    ranked: &[Entity],
    lock: Option<Entity>,
    pinned: bool,
) -> Vec<Entity> {
    let mut entries: Vec<Entity> = if pinned {
        let mut kept: Vec<Entity> = prev
            .iter()
            .copied()
            .filter(|entity| ranked.contains(entity))
            .collect();
        for &entity in ranked {
            if kept.len() >= TARGET_CANDIDATE_COUNT {
                break;
            }
            if !kept.contains(&entity) {
                kept.push(entity);
            }
        }
        kept
    } else {
        ranked
            .iter()
            .copied()
            .take(TARGET_CANDIDATE_COUNT)
            .collect()
    };
    if let Some(lock) = lock {
        if ranked.contains(&lock) && !entries.contains(&lock) {
            entries.pop();
            entries.push(lock);
        }
    }
    entries
}

/// Accumulate focus while the lock stays on one target; any change (new
/// target or lock lost) restarts the dwell from zero.
fn tick_lock_focus(
    time: Res<Time>,
    lock: Res<SpaceshipPlayerTargetLock>,
    mut focus: ResMut<SpaceshipPlayerLockFocus>,
) {
    if focus.target != **lock {
        focus.target = **lock;
        focus.seconds = 0.0;
        return;
    }
    if focus.target.is_some() {
        focus.seconds += time.delta_secs();
    }
}

/// Distance from `point` to the ray `(origin, dir)`, with the projection
/// clamped behind the origin (a point behind the ship measures to the origin
/// rather than to the ray's backward extension).
fn ray_distance(origin: Vec3, dir: Vec3, point: Vec3) -> f32 {
    let to_point = point - origin;
    let along = to_point.dot(dir).max(0.0);
    (to_point - dir * along).length()
}

/// Snap selection with hysteresis: the nearest candidate wins, unless an
/// incumbent is still selected and the challenger is not decisively closer
/// (below [`SNAP_HYSTERESIS`] of the incumbent's distance).
fn snap_pick(current: Option<Entity>, candidates: &[(Entity, f32)]) -> Option<Entity> {
    let (best, best_distance) = candidates
        .iter()
        .min_by(|(_, a), (_, b)| a.total_cmp(b))
        .copied()?;
    if let Some(current) = current {
        if let Some((_, current_distance)) =
            candidates.iter().find(|(entity, _)| *entity == current)
        {
            if best_distance >= SNAP_HYSTERESIS * current_distance {
                return Some(current);
            }
        }
    }
    Some(best)
}

/// Stable cycle order for a ship's sections: nose-to-tail by local build
/// position (z, then x, then y), so repeated presses walk the hull the same
/// way every time regardless of query iteration order.
fn cycle_order(sections: &mut [(Entity, Vec3)]) {
    sections.sort_by(|(_, a), (_, b)| {
        a.z.total_cmp(&b.z)
            .then(a.x.total_cmp(&b.x))
            .then(a.y.total_cmp(&b.y))
    });
}

/// Maintain the component fine-lock: valid only while focused on the locked
/// ship and while the section stays attached; a pin expires by deadline or
/// with its section; snap follows the crosshair ray otherwise.
#[allow(clippy::type_complexity)]
fn update_component_lock(
    time: Res<Time>,
    lock: Res<SpaceshipPlayerTargetLock>,
    focus: Res<SpaceshipPlayerLockFocus>,
    mut component: ResMut<SpaceshipPlayerComponentLock>,
    q_sections: Query<(Entity, &ChildOf, &GlobalTransform), With<SectionMarker>>,
    point_rotation: Option<
        Single<
            &PointRotationOutput,
            (
                With<SpaceshipCameraInputMarker>,
                With<SpaceshipCameraTurretInputMarker>,
            ),
        >,
    >,
    spaceship: Option<
        Single<
            (&Transform, Option<&ComputedCenterOfMass>),
            (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>),
        >,
    >,
) {
    // The component layer only exists while focused on the current lock.
    let target = match **lock {
        Some(target) if focus.focused_on(target) => target,
        _ => {
            component.set_if_neq(SpaceshipPlayerComponentLock::default());
            return;
        }
    };

    let sections: Vec<(Entity, Vec3)> = q_sections
        .iter()
        .filter(|(_, ChildOf(parent), _)| *parent == target)
        .map(|(entity, _, transform)| (entity, transform.translation()))
        .collect();
    if sections.is_empty() {
        component.set_if_neq(SpaceshipPlayerComponentLock::default());
        return;
    }

    // Detach/despawn invalidates the selection (inactive sections stay
    // lockable - see SpaceshipPlayerComponentLock).
    let current = component
        .section
        .filter(|section| sections.iter().any(|(entity, _)| entity == section));
    if component.section != current {
        component.section = current;
    }

    // A pin outlives neither its deadline nor its section.
    if let ComponentLockMode::Pinned { until } = component.mode {
        if component.section.is_none() || time.elapsed_secs() >= until {
            component.mode = ComponentLockMode::Snap;
        }
    }

    if component.mode != ComponentLockMode::Snap {
        return;
    }
    let (Some(point_rotation), Some(spaceship)) = (point_rotation, spaceship) else {
        // No aim rig (menu states, headless tests): hold the current
        // selection rather than guessing.
        return;
    };
    let (ship_transform, ship_com) = spaceship.into_inner();
    let origin = live_structure_anchor(ship_transform, ship_com);
    let dir = (***point_rotation * Vec3::NEG_Z).normalize();
    let candidates: Vec<(Entity, f32)> = sections
        .iter()
        .map(|&(entity, position)| (entity, ray_distance(origin, dir, position)))
        .collect();
    let picked = snap_pick(component.section, &candidates);
    if component.section != picked {
        component.section = picked;
    }
}

/// Shared body of the cycle observers: step the fine lock through the locked
/// ship's attached sections in [`cycle_order`] and pin the choice for
/// [`COMPONENT_PIN_WINDOW`] seconds.
fn step_component_lock(
    direction: isize,
    time: &Time,
    lock: &SpaceshipPlayerTargetLock,
    focus: &SpaceshipPlayerLockFocus,
    component: &mut SpaceshipPlayerComponentLock,
    q_sections: &Query<(Entity, &ChildOf, &Transform), With<SectionMarker>>,
) {
    let target = match **lock {
        Some(target) if focus.focused_on(target) => target,
        _ => return,
    };

    let mut order: Vec<(Entity, Vec3)> = q_sections
        .iter()
        .filter(|(_, ChildOf(parent), _)| *parent == target)
        .map(|(entity, _, transform)| (entity, transform.translation))
        .collect();
    if order.is_empty() {
        return;
    }
    cycle_order(&mut order);

    let len = order.len() as isize;
    let index = component
        .section
        .and_then(|section| order.iter().position(|(entity, _)| *entity == section));
    let next = match index {
        Some(index) => (index as isize + direction).rem_euclid(len) as usize,
        // First press with no selection: next starts at the nose, prev at
        // the tail.
        None if direction >= 0 => 0,
        None => (len - 1) as usize,
    };

    component.section = Some(order[next].0);
    component.mode = ComponentLockMode::Pinned {
        until: time.elapsed_secs() + COMPONENT_PIN_WINDOW,
    };
}

/// Whether the CTRL cycle-layer modifier currently fires, read from its
/// action entity's state. The wheel/bracket gestures are routed HERE, in
/// the observers, rather than via input conditions: a binding-level Chord
/// ignores the binding's own value (pressing CTRL alone cycled the lock,
/// bug 20260711-173237), and pairing it with an explicit Down still leaves
/// the unmodified gesture Ongoing, which triggers Start.
fn cycle_modifier_held(
    q_modifier: &Query<&TriggerState, With<Action<TargetCycleModifierInput>>>,
) -> bool {
    q_modifier.iter().any(|&state| state == TriggerState::Fired)
}

#[allow(clippy::too_many_arguments)]
fn on_component_cycle_next(
    _: On<Start<ComponentCycleNextInput>>,
    time: Res<Time>,
    q_modifier: Query<&TriggerState, With<Action<TargetCycleModifierInput>>>,
    mut lock: ResMut<SpaceshipPlayerTargetLock>,
    mut candidates: ResMut<SpaceshipPlayerTargetCandidates>,
    focus: Res<SpaceshipPlayerLockFocus>,
    mut component: ResMut<SpaceshipPlayerComponentLock>,
    q_sections: Query<(Entity, &ChildOf, &Transform), With<SectionMarker>>,
    pause: Res<State<crate::PauseStates>>,
) {
    // Observers bypass system-set gating; freeze intent changes while the
    // pause overlay is up (review R1.1). Releases stay ungated so held keys
    // clear cleanly during a pause.
    if *pause.get() == crate::PauseStates::Paused {
        return;
    }

    if cycle_modifier_held(&q_modifier) {
        step_target_lock(1, &time, &mut lock, &mut candidates);
    } else {
        step_component_lock(1, &time, &lock, &focus, &mut component, &q_sections);
    }
}

/// Shared body of the target-cycle observers: step the ship lock through the
/// tracked candidates in ranked order and pin it for [`TARGET_PIN_WINDOW`]
/// seconds against the aim-driven picker. No focus gate - switching ships is
/// the fast loop, unlike the component cycle. A lock outside the list (an
/// asteroid, a torpedo) is simply not in the order: next starts at the best
/// candidate, prev at the worst.
fn step_target_lock(
    direction: isize,
    time: &Time,
    lock: &mut SpaceshipPlayerTargetLock,
    candidates: &mut SpaceshipPlayerTargetCandidates,
) {
    let len = candidates.entries.len() as isize;
    if len == 0 {
        return;
    }
    let index = lock.and_then(|target| {
        candidates
            .entries
            .iter()
            .position(|&entity| entity == target)
    });
    let next = match index {
        Some(index) => (index as isize + direction).rem_euclid(len) as usize,
        None if direction >= 0 => 0,
        None => (len - 1) as usize,
    };
    **lock = Some(candidates.entries[next]);
    candidates.pinned_until = Some(time.elapsed_secs() + TARGET_PIN_WINDOW);
}

fn on_target_cycle_next(
    _: On<Start<TargetCycleNextInput>>,
    time: Res<Time>,
    mut lock: ResMut<SpaceshipPlayerTargetLock>,
    mut candidates: ResMut<SpaceshipPlayerTargetCandidates>,
    pause: Res<State<crate::PauseStates>>,
) {
    // Observers bypass system-set gating; freeze intent changes while the
    // pause overlay is up (review R1.1). Releases stay ungated so held keys
    // clear cleanly during a pause.
    if *pause.get() == crate::PauseStates::Paused {
        return;
    }

    step_target_lock(1, &time, &mut lock, &mut candidates);
}

fn on_target_cycle_prev(
    _: On<Start<TargetCyclePrevInput>>,
    time: Res<Time>,
    mut lock: ResMut<SpaceshipPlayerTargetLock>,
    mut candidates: ResMut<SpaceshipPlayerTargetCandidates>,
    pause: Res<State<crate::PauseStates>>,
) {
    // Observers bypass system-set gating; freeze intent changes while the
    // pause overlay is up (review R1.1). Releases stay ungated so held keys
    // clear cleanly during a pause.
    if *pause.get() == crate::PauseStates::Paused {
        return;
    }

    step_target_lock(-1, &time, &mut lock, &mut candidates);
}

#[allow(clippy::too_many_arguments)]
fn on_component_cycle_prev(
    _: On<Start<ComponentCyclePrevInput>>,
    time: Res<Time>,
    q_modifier: Query<&TriggerState, With<Action<TargetCycleModifierInput>>>,
    mut lock: ResMut<SpaceshipPlayerTargetLock>,
    mut candidates: ResMut<SpaceshipPlayerTargetCandidates>,
    focus: Res<SpaceshipPlayerLockFocus>,
    mut component: ResMut<SpaceshipPlayerComponentLock>,
    q_sections: Query<(Entity, &ChildOf, &Transform), With<SectionMarker>>,
    pause: Res<State<crate::PauseStates>>,
) {
    // Observers bypass system-set gating; freeze intent changes while the
    // pause overlay is up (review R1.1). Releases stay ungated so held keys
    // clear cleanly during a pause.
    if *pause.get() == crate::PauseStates::Paused {
        return;
    }

    if cycle_modifier_held(&q_modifier) {
        step_target_lock(-1, &time, &mut lock, &mut candidates);
    } else {
        step_component_lock(-1, &time, &lock, &focus, &mut component, &q_sections);
    }
}

#[cfg(test)]
mod tests {
    use bevy::ecs::system::RunSystemOnce;

    use super::*;

    fn cone_cos(half_angle_deg: f32) -> f32 {
        half_angle_deg.to_radians().cos()
    }

    #[test]
    fn pick_target_locks_the_body_nearest_the_aim_ray() {
        // Two candidates in front: one slightly off-axis, one further off-axis.
        // The nearer-to-center one wins even though it is further away.
        let origin = Vec3::ZERO;
        let aim = Vec3::NEG_Z;
        let near_center = Entity::from_raw_u32(1).unwrap();
        let off_center = Entity::from_raw_u32(2).unwrap();
        let candidates = [
            (near_center, Vec3::new(2.0, 0.0, -100.0)), // ~1.1 deg off axis, far
            (off_center, Vec3::new(3.0, 0.0, -20.0)),   // ~8.5 deg off axis, near
        ];

        let picked = pick_target(
            origin,
            aim,
            TARGETING_MAX_RANGE,
            cone_cos(18.0),
            candidates.into_iter(),
        );
        assert_eq!(picked, Some(near_center));
    }

    #[test]
    fn pick_target_ignores_bodies_outside_the_cone() {
        // A body 90 deg off the aim direction (straight to the side) is not lockable.
        let picked = pick_target(
            Vec3::ZERO,
            Vec3::NEG_Z,
            TARGETING_MAX_RANGE,
            cone_cos(18.0),
            [(Entity::from_raw_u32(1).unwrap(), Vec3::new(50.0, 0.0, 0.0))].into_iter(),
        );
        assert_eq!(picked, None, "a body outside the cone must not be locked");
    }

    #[test]
    fn pick_target_ignores_bodies_behind_the_ship() {
        // A body directly behind (dot with aim is negative) is never locked.
        let picked = pick_target(
            Vec3::ZERO,
            Vec3::NEG_Z,
            TARGETING_MAX_RANGE,
            cone_cos(18.0),
            [(Entity::from_raw_u32(1).unwrap(), Vec3::new(0.0, 0.0, 100.0))].into_iter(),
        );
        assert_eq!(picked, None, "a body behind the ship must not be locked");
    }

    #[test]
    fn pick_target_ignores_bodies_beyond_max_range() {
        // Dead ahead but past the range limit: not lockable.
        let picked = pick_target(
            Vec3::ZERO,
            Vec3::NEG_Z,
            100.0,
            cone_cos(18.0),
            [(
                Entity::from_raw_u32(1).unwrap(),
                Vec3::new(0.0, 0.0, -500.0),
            )]
            .into_iter(),
        );
        assert_eq!(picked, None, "a body beyond max range must not be locked");
    }

    #[test]
    fn pick_target_locks_distant_bodies_for_designation() {
        // The lock doubles as the GOTO/torpedo designator, so a body far
        // down-range (well past the old 2 km limit) must still lock when
        // aimed at (user report 20260710).
        let asteroid = Entity::from_raw_u32(1).unwrap();
        let picked = pick_target(
            Vec3::ZERO,
            Vec3::NEG_Z,
            TARGETING_MAX_RANGE,
            cone_cos(18.0),
            [(asteroid, Vec3::new(0.0, 0.0, -15_000.0))].into_iter(),
        );
        assert_eq!(picked, Some(asteroid), "distant designation must lock");
    }

    #[test]
    fn pick_target_returns_none_with_no_candidates() {
        let picked = pick_target(
            Vec3::ZERO,
            Vec3::NEG_Z,
            TARGETING_MAX_RANGE,
            cone_cos(18.0),
            std::iter::empty(),
        );
        assert_eq!(picked, None);
    }

    #[test]
    fn lock_cone_originates_at_the_live_structure_anchor() {
        // A candidate dead ahead of the ANCHOR but 33 degrees off the ROOT
        // ORIGIN bearing: it locks only if the cone originates at the anchor
        // (18 degree half-angle).
        let mut world = World::new();
        world.insert_resource(Time::<()>::default());
        world.insert_resource(SpaceshipPlayerTargetLock(None));
        world.insert_resource(SpaceshipPlayerTargetCandidates::default());
        world.init_resource::<TargetingSettings>();
        world.spawn((
            SpaceshipCameraInputMarker,
            SpaceshipCameraTurretInputMarker,
            PointRotationOutput(Quat::IDENTITY),
        ));
        world.spawn((
            SpaceshipRootMarker,
            PlayerSpaceshipMarker,
            Transform::from_translation(Vec3::new(10.0, 0.0, 0.0)),
            ComputedCenterOfMass(Vec3::new(2.0, 0.0, 0.0)),
        ));
        let candidate = world
            .spawn((
                RigidBody::Dynamic,
                GlobalTransform::from_translation(Vec3::new(12.0, 0.0, -3.0)),
            ))
            .id();

        world
            .run_system_once(update_spaceship_target_input)
            .unwrap();

        assert_eq!(
            **world.resource::<SpaceshipPlayerTargetLock>(),
            Some(candidate),
            "the cone must originate at the anchor, not the root origin"
        );
    }

    #[test]
    fn signature_picks_the_nearest_hostile_in_range() {
        let near = Entity::from_raw_u32(1).unwrap();
        let far = Entity::from_raw_u32(2).unwrap();
        let candidates = [
            (far, Vec3::new(0.0, 0.0, 400.0), true),
            (near, Vec3::new(0.0, 0.0, -200.0), true),
        ];

        let picked = pick_signature_target(Vec3::ZERO, 550.0, candidates.into_iter());

        assert_eq!(picked, Some(near), "nearest hostile wins, direction-blind");
    }

    #[test]
    fn signature_never_acquires_non_hostiles() {
        let rock = Entity::from_raw_u32(1).unwrap();
        let candidates = [(rock, Vec3::new(0.0, 0.0, -50.0), false)];

        let picked = pick_signature_target(Vec3::ZERO, 550.0, candidates.into_iter());

        assert_eq!(picked, None, "asteroids and neutral bodies never auto-lock");
    }

    #[test]
    fn signature_respects_the_range() {
        let hostile = Entity::from_raw_u32(1).unwrap();
        let candidates = [(hostile, Vec3::new(0.0, 0.0, -600.0), true)];

        let picked = pick_signature_target(Vec3::ZERO, 550.0, candidates.into_iter());

        assert_eq!(picked, None, "beyond signature range needs deliberate aim");
    }

    /// Spawn the camera-input rig + player the acquisition system needs.
    fn spawn_acquisition_rig(world: &mut World) {
        world.insert_resource(Time::<()>::default());
        world.insert_resource(SpaceshipPlayerTargetLock(None));
        world.insert_resource(SpaceshipPlayerTargetCandidates::default());
        world.init_resource::<TargetingSettings>();
        world.spawn((
            SpaceshipCameraInputMarker,
            SpaceshipCameraTurretInputMarker,
            PointRotationOutput(Quat::IDENTITY),
        ));
        world.spawn((
            SpaceshipRootMarker,
            PlayerSpaceshipMarker,
            Transform::IDENTITY,
        ));
    }

    #[test]
    fn cone_pick_beats_the_signature_fallback() {
        // A hostile BEHIND the player inside signature range, and a body dead
        // ahead in the cone: aiming designates, so the cone target wins.
        let mut world = World::new();
        spawn_acquisition_rig(&mut world);
        world.spawn((
            AISpaceshipMarker,
            RigidBody::Dynamic,
            GlobalTransform::from_translation(Vec3::new(0.0, 0.0, 100.0)),
        ));
        let aimed = world
            .spawn((
                RigidBody::Dynamic,
                // Signed so the scanner sees it at 300u (bare bodies are
                // debris and gate out - see unsigned_debris test).
                LockSignature(20.0),
                GlobalTransform::from_translation(Vec3::new(0.0, 0.0, -300.0)),
            ))
            .id();

        world
            .run_system_once(update_spaceship_target_input)
            .unwrap();

        assert_eq!(**world.resource::<SpaceshipPlayerTargetLock>(), Some(aimed));
    }

    #[test]
    fn small_signatures_only_lock_up_close() {
        // A signed 2u rock (lock range 60u at the default 30/unit) dead
        // ahead: invisible to the scanner at 200u, lockable at 40u.
        let mut world = World::new();
        spawn_acquisition_rig(&mut world);
        let rock = world
            .spawn((
                RigidBody::Dynamic,
                LockSignature(2.0),
                GlobalTransform::from_translation(Vec3::new(0.0, 0.0, -200.0)),
            ))
            .id();

        world
            .run_system_once(update_spaceship_target_input)
            .unwrap();
        assert_eq!(
            **world.resource::<SpaceshipPlayerTargetLock>(),
            None,
            "a small rock returns no signature at range"
        );

        world
            .entity_mut(rock)
            .insert(GlobalTransform::from_translation(Vec3::new(
                0.0, 0.0, -40.0,
            )));
        world
            .run_system_once(update_spaceship_target_input)
            .unwrap();
        assert_eq!(
            **world.resource::<SpaceshipPlayerTargetLock>(),
            Some(rock),
            "close enough, the scanner sees it"
        );
    }

    #[test]
    fn unsigned_debris_is_point_blank_only() {
        // A bare dynamic body (battle debris) dead ahead: never lockable
        // at 50u, only inside the unsigned point-blank range.
        let mut world = World::new();
        spawn_acquisition_rig(&mut world);
        let debris = world
            .spawn((
                RigidBody::Dynamic,
                GlobalTransform::from_translation(Vec3::new(0.0, 0.0, -50.0)),
            ))
            .id();

        world
            .run_system_once(update_spaceship_target_input)
            .unwrap();
        assert_eq!(
            **world.resource::<SpaceshipPlayerTargetLock>(),
            None,
            "debris must not steal mid-fight locks"
        );

        world
            .entity_mut(debris)
            .insert(GlobalTransform::from_translation(Vec3::new(
                0.0, 0.0, -10.0,
            )));
        world
            .run_system_once(update_spaceship_target_input)
            .unwrap();
        assert_eq!(
            **world.resource::<SpaceshipPlayerTargetLock>(),
            Some(debris),
            "point-blank debris is still designatable"
        );
    }

    /// A Static body with an AUTHORED LockSignature is lockable (a nav
    /// beacon on rails, task 20260712-093044) at its signature range, while
    /// a bare Static body (a scenario trigger area) stays invisible to the
    /// scanner at any distance - the gate must tell them apart.
    #[test]
    fn static_beacons_lock_but_static_areas_never_do() {
        let mut world = World::new();
        spawn_acquisition_rig(&mut world);
        let beacon = world
            .spawn((
                RigidBody::Static,
                LockSignature(20.0),
                GlobalTransform::from_translation(Vec3::new(0.0, 0.0, -300.0)),
            ))
            .id();

        world
            .run_system_once(update_spaceship_target_input)
            .unwrap();
        assert_eq!(
            **world.resource::<SpaceshipPlayerTargetLock>(),
            Some(beacon),
            "a signed static body (nav beacon) is aim-lockable"
        );

        // Same rig, but the static body carries no signature: a trigger
        // area. Point-blank dead ahead and still never locked.
        let mut world = World::new();
        spawn_acquisition_rig(&mut world);
        world.spawn((
            RigidBody::Static,
            GlobalTransform::from_translation(Vec3::new(0.0, 0.0, -10.0)),
        ));

        world
            .run_system_once(update_spaceship_target_input)
            .unwrap();
        assert_eq!(
            **world.resource::<SpaceshipPlayerTargetLock>(),
            None,
            "an unsigned static body (trigger area) is never lockable"
        );
    }

    #[test]
    fn ships_and_well_bodies_keep_their_long_range_lock() {
        // The full-range classes: a well body across the field and a ship.
        for components in 0..2 {
            let mut world = World::new();
            spawn_acquisition_rig(&mut world);
            let far = GlobalTransform::from_translation(Vec3::new(0.0, 0.0, -5000.0));
            let target = match components {
                0 => world
                    .spawn((
                        RigidBody::Static,
                        GravityWell::from_surface_gravity(3.0, 20.0, &GravitySettings::default()),
                        far,
                    ))
                    .id(),
                _ => world
                    .spawn((SpaceshipRootMarker, RigidBody::Dynamic, far))
                    .id(),
            };

            world
                .run_system_once(update_spaceship_target_input)
                .unwrap();
            assert_eq!(
                **world.resource::<SpaceshipPlayerTargetLock>(),
                Some(target),
                "full-range class {components} must lock at range"
            );
        }
    }

    #[test]
    fn committed_torpedoes_lock_at_combat_range_not_across_the_map() {
        // Small but hot: well beyond every real point-defense engagement,
        // but not the full designator range - the scanner fiction holds.
        let mut world = World::new();
        spawn_acquisition_rig(&mut world);
        let torpedo = world
            .spawn((
                TorpedoProjectileMarker,
                TorpedoTargetChosen,
                RigidBody::Dynamic,
                GlobalTransform::from_translation(Vec3::new(0.0, 0.0, -2000.0)),
            ))
            .id();

        world
            .run_system_once(update_spaceship_target_input)
            .unwrap();
        assert_eq!(
            **world.resource::<SpaceshipPlayerTargetLock>(),
            Some(torpedo),
            "point defense locks torpedoes at combat range"
        );

        world
            .entity_mut(torpedo)
            .insert(GlobalTransform::from_translation(Vec3::new(
                0.0, 0.0, -5000.0,
            )));
        world.insert_resource(SpaceshipPlayerTargetLock(None));
        world
            .run_system_once(update_spaceship_target_input)
            .unwrap();
        assert_eq!(
            **world.resource::<SpaceshipPlayerTargetLock>(),
            None,
            "a torpedo is not visible across the map"
        );
    }

    #[test]
    fn the_lock_holds_a_little_past_its_gate_but_fresh_locks_do_not() {
        // A signed 2u rock gates at 60u. Fresh acquisition at 65u: refused.
        let mut world = World::new();
        spawn_acquisition_rig(&mut world);
        let rock = world
            .spawn((
                RigidBody::Dynamic,
                LockSignature(2.0),
                GlobalTransform::from_translation(Vec3::new(0.0, 0.0, -65.0)),
            ))
            .id();
        world
            .run_system_once(update_spaceship_target_input)
            .unwrap();
        assert_eq!(**world.resource::<SpaceshipPlayerTargetLock>(), None);

        // Locked inside the gate, then drifting to 65u (inside 1.15x):
        // the incumbent holds - no strobing, the focus dwell survives.
        world.insert_resource(SpaceshipPlayerTargetLock(Some(rock)));
        world
            .run_system_once(update_spaceship_target_input)
            .unwrap();
        assert_eq!(
            **world.resource::<SpaceshipPlayerTargetLock>(),
            Some(rock),
            "the incumbent holds inside the hysteresis band"
        );

        // Truly out (past 1.15x): released.
        world
            .entity_mut(rock)
            .insert(GlobalTransform::from_translation(Vec3::new(
                0.0, 0.0, -80.0,
            )));
        world
            .run_system_once(update_spaceship_target_input)
            .unwrap();
        assert_eq!(**world.resource::<SpaceshipPlayerTargetLock>(), None);
    }

    #[test]
    fn a_held_lock_is_not_stolen_by_a_closer_body() {
        // Sticky-from-acquisition (task 20260712-203353): once locked, a body
        // closer to the aim ray must not steal the lock.
        let mut world = World::new();
        spawn_acquisition_rig(&mut world);
        // A sits slightly off the aim ray (-Z); it is the only candidate, so
        // aim acquires it.
        let a = world
            .spawn((
                SpaceshipRootMarker,
                RigidBody::Dynamic,
                GlobalTransform::from_translation(Vec3::new(5.0, 0.0, -100.0)),
            ))
            .id();
        world
            .run_system_once(update_spaceship_target_input)
            .unwrap();
        assert_eq!(
            **world.resource::<SpaceshipPlayerTargetLock>(),
            Some(a),
            "aim acquires the first ship"
        );

        // A challenger dead ahead (closer to the aim ray) appears: a
        // non-sticky picker would switch to it. The held lock must stay on A.
        let b = world
            .spawn((
                SpaceshipRootMarker,
                RigidBody::Dynamic,
                GlobalTransform::from_translation(Vec3::new(0.0, 0.0, -100.0)),
            ))
            .id();
        world
            .run_system_once(update_spaceship_target_input)
            .unwrap();
        assert_eq!(
            **world.resource::<SpaceshipPlayerTargetLock>(),
            Some(a),
            "a held lock is not stolen by a closer body"
        );

        // Delivery guard: with A gone the picker re-acquires B, proving it CAN
        // still move - the hold above was stickiness, not a wedged picker.
        world.despawn(a);
        world
            .run_system_once(update_spaceship_target_input)
            .unwrap();
        assert_eq!(
            **world.resource::<SpaceshipPlayerTargetLock>(),
            Some(b),
            "the picker re-acquires after the held target leaves"
        );
    }

    #[test]
    fn a_non_ship_lock_is_not_sticky_so_nav_re_designates() {
        // Review R1.1: stickiness is ship-only. A nav body (signed rock /
        // beacon-like, NOT a ship) must stay aim-driven, so the GOTO designator
        // can be re-pointed by aiming.
        let mut world = World::new();
        spawn_acquisition_rig(&mut world);
        // Two signed rocks (nav bodies): A dead ahead, B just off-axis.
        let a = world
            .spawn((
                RigidBody::Dynamic,
                LockSignature(50.0),
                GlobalTransform::from_translation(Vec3::new(0.0, 0.0, -100.0)),
            ))
            .id();
        let b = world
            .spawn((
                RigidBody::Dynamic,
                LockSignature(50.0),
                GlobalTransform::from_translation(Vec3::new(3.0, 0.0, -100.0)),
            ))
            .id();
        world
            .run_system_once(update_spaceship_target_input)
            .unwrap();
        assert_eq!(
            **world.resource::<SpaceshipPlayerTargetLock>(),
            Some(a),
            "aim locks the dead-ahead rock"
        );

        // Move A off the aim ray so B is now nearest it. A non-ship lock is not
        // sticky, so the picker re-designates to B - nav switching by aiming
        // still works (unlike a ship lock, which would hold; see
        // a_held_lock_is_not_stolen_by_a_closer_body).
        world
            .entity_mut(a)
            .insert(GlobalTransform::from_translation(Vec3::new(
                10.0, 0.0, -100.0,
            )));
        world
            .run_system_once(update_spaceship_target_input)
            .unwrap();
        assert_eq!(
            **world.resource::<SpaceshipPlayerTargetLock>(),
            Some(b),
            "a non-ship lock is not sticky; aiming re-designates the nav target"
        );
    }

    #[test]
    fn an_authored_signature_never_gates_below_the_debris_floor() {
        // LockSignature(0.0) would gate at zero range; the floor keeps it
        // at least as visible as unsigned debris.
        let mut world = World::new();
        spawn_acquisition_rig(&mut world);
        let speck = world
            .spawn((
                RigidBody::Dynamic,
                LockSignature(0.0),
                GlobalTransform::from_translation(Vec3::new(0.0, 0.0, -10.0)),
            ))
            .id();
        world
            .run_system_once(update_spaceship_target_input)
            .unwrap();
        assert_eq!(
            **world.resource::<SpaceshipPlayerTargetLock>(),
            Some(speck),
            "a zero signature still locks at the debris floor"
        );
    }

    #[test]
    fn a_static_gravity_well_source_is_lockable_but_a_static_sensor_is_not() {
        // Well sources went on rails (RigidBody::Static) in the gravity
        // task, which silently dropped them from the lockable set - the
        // 2026-07-10 playtest could not GOTO the Gravity Rock. A static
        // body with a GravityWell is a big visible rock, lockable; a bare
        // static body (scenario trigger volume) stays unlockable.
        let mut world = World::new();
        spawn_acquisition_rig(&mut world);
        world.spawn((
            RigidBody::Static,
            GlobalTransform::from_translation(Vec3::new(0.0, 0.0, -200.0)),
        ));
        let rock = world
            .spawn((
                RigidBody::Static,
                GravityWell::from_surface_gravity(3.0, 20.0, &GravitySettings::default()),
                GlobalTransform::from_translation(Vec3::new(0.0, 0.0, -300.0)),
            ))
            .id();

        world
            .run_system_once(update_spaceship_target_input)
            .unwrap();

        assert_eq!(**world.resource::<SpaceshipPlayerTargetLock>(), Some(rock));
    }

    #[test]
    fn empty_cone_auto_acquires_the_close_hostile() {
        // Nothing ahead; a hostile behind the player inside signature range
        // locks by itself - the heat-signature acquisition.
        let mut world = World::new();
        spawn_acquisition_rig(&mut world);
        let hostile = world
            .spawn((
                AISpaceshipMarker,
                RigidBody::Dynamic,
                GlobalTransform::from_translation(Vec3::new(0.0, 0.0, 100.0)),
            ))
            .id();

        world
            .run_system_once(update_spaceship_target_input)
            .unwrap();

        assert_eq!(
            **world.resource::<SpaceshipPlayerTargetLock>(),
            Some(hostile)
        );
    }

    #[test]
    fn empty_cone_ignores_non_hostiles_and_far_hostiles() {
        // A controller-less ship nearby and a hostile beyond signature range:
        // neither auto-acquires, the lock stays empty.
        let mut world = World::new();
        spawn_acquisition_rig(&mut world);
        world.spawn((
            SpaceshipRootMarker,
            RigidBody::Dynamic,
            GlobalTransform::from_translation(Vec3::new(0.0, 0.0, 100.0)),
        ));
        world.spawn((
            AISpaceshipMarker,
            RigidBody::Dynamic,
            GlobalTransform::from_translation(Vec3::new(0.0, 0.0, 900.0)),
        ));

        world
            .run_system_once(update_spaceship_target_input)
            .unwrap();

        assert_eq!(**world.resource::<SpaceshipPlayerTargetLock>(), None);
    }

    #[test]
    fn empty_cone_never_auto_acquires_a_neutral_ship() {
        // An explicitly Neutral ship close by is a bystander, not a threat:
        // the signature fallback must leave it alone (task 20260708-203708).
        let mut world = World::new();
        spawn_acquisition_rig(&mut world);
        world.spawn((
            SpaceshipRootMarker,
            Allegiance::Neutral,
            RigidBody::Dynamic,
            GlobalTransform::from_translation(Vec3::new(0.0, 0.0, 100.0)),
        ));

        world
            .run_system_once(update_spaceship_target_input)
            .unwrap();

        assert_eq!(**world.resource::<SpaceshipPlayerTargetLock>(), None);
    }

    // -- focus dwell + component fine-lock --

    use std::time::Duration;

    /// A focused world: aim rig on -Z, player at the origin, a locked target
    /// ship with three sections - one dead on the aim ray, two off to the
    /// side - and the focus dwell already complete.
    fn focused_world() -> (World, Entity, [Entity; 3]) {
        let mut world = World::new();
        world.insert_resource(Time::<()>::default());
        world.spawn((
            SpaceshipCameraInputMarker,
            SpaceshipCameraTurretInputMarker,
            PointRotationOutput(Quat::IDENTITY),
        ));
        world.spawn((
            SpaceshipRootMarker,
            PlayerSpaceshipMarker,
            Transform::IDENTITY,
        ));
        let target = world.spawn(SpaceshipRootMarker).id();
        // Local build order (z) deliberately different from spawn order, so
        // the cycle-order sort is actually exercised.
        let on_ray = world
            .spawn((
                SectionMarker,
                Transform::from_translation(Vec3::new(0.0, 0.0, 1.0)),
                GlobalTransform::from_translation(Vec3::new(0.0, 0.0, -100.0)),
                ChildOf(target),
            ))
            .id();
        let near_ray = world
            .spawn((
                SectionMarker,
                Transform::from_translation(Vec3::new(0.0, 0.0, 0.0)),
                GlobalTransform::from_translation(Vec3::new(5.0, 0.0, -100.0)),
                ChildOf(target),
            ))
            .id();
        let far_ray = world
            .spawn((
                SectionMarker,
                Transform::from_translation(Vec3::new(0.0, 0.0, 2.0)),
                GlobalTransform::from_translation(Vec3::new(10.0, 0.0, -100.0)),
                ChildOf(target),
            ))
            .id();
        world.insert_resource(SpaceshipPlayerTargetLock(Some(target)));
        world.insert_resource(SpaceshipPlayerLockFocus {
            target: Some(target),
            seconds: FOCUS_TIME,
        });
        world.insert_resource(SpaceshipPlayerComponentLock::default());
        (world, target, [on_ray, near_ray, far_ray])
    }

    fn cycle(world: &mut World, direction: isize) {
        world
            .run_system_once(
                move |time: Res<Time>,
                      lock: Res<SpaceshipPlayerTargetLock>,
                      focus: Res<SpaceshipPlayerLockFocus>,
                      mut component: ResMut<SpaceshipPlayerComponentLock>,
                      q_sections: Query<(Entity, &ChildOf, &Transform), With<SectionMarker>>| {
                    step_component_lock(
                        direction,
                        &time,
                        &lock,
                        &focus,
                        &mut component,
                        &q_sections,
                    );
                },
            )
            .unwrap();
    }

    #[test]
    fn focus_accumulates_and_resets_on_lock_change() {
        let mut world = World::new();
        world.insert_resource(Time::<()>::default());
        let a = world.spawn_empty().id();
        let b = world.spawn_empty().id();
        world.insert_resource(SpaceshipPlayerTargetLock(Some(a)));
        world.insert_resource(SpaceshipPlayerLockFocus::default());

        // First tick registers the new target (reset), then time accrues.
        world.run_system_once(tick_lock_focus).unwrap();
        world
            .resource_mut::<Time>()
            .advance_by(Duration::from_secs_f32(1.0));
        world.run_system_once(tick_lock_focus).unwrap();
        let focus = world.resource::<SpaceshipPlayerLockFocus>();
        assert_eq!(focus.target, Some(a));
        assert!((focus.seconds - 1.0).abs() < 1e-6);
        assert!(!focus.focused_on(a));

        world
            .resource_mut::<Time>()
            .advance_by(Duration::from_secs_f32(0.6));
        world.run_system_once(tick_lock_focus).unwrap();
        assert!(world.resource::<SpaceshipPlayerLockFocus>().focused_on(a));

        // Switching targets restarts the dwell.
        world.insert_resource(SpaceshipPlayerTargetLock(Some(b)));
        world.run_system_once(tick_lock_focus).unwrap();
        let focus = world.resource::<SpaceshipPlayerLockFocus>();
        assert_eq!(focus.target, Some(b));
        assert_eq!(focus.seconds, 0.0);
    }

    #[test]
    fn ray_distance_measures_perpendicular_and_clamps_behind() {
        let origin = Vec3::ZERO;
        let dir = Vec3::NEG_Z;
        assert!((ray_distance(origin, dir, Vec3::new(3.0, 4.0, -10.0)) - 5.0).abs() < 1e-6);
        // Behind the origin: distance to the origin itself, not the
        // backward extension.
        assert!((ray_distance(origin, dir, Vec3::new(0.0, 0.0, 7.0)) - 7.0).abs() < 1e-6);
    }

    #[test]
    fn snap_pick_applies_hysteresis() {
        let a = Entity::from_raw_u32(1).unwrap();
        let b = Entity::from_raw_u32(2).unwrap();
        assert_eq!(snap_pick(None, &[]), None);
        // No incumbent: nearest wins.
        assert_eq!(snap_pick(None, &[(a, 5.0), (b, 3.0)]), Some(b));
        // Challenger at 0.8x of the incumbent: not decisive, keep a.
        assert_eq!(snap_pick(Some(a), &[(a, 5.0), (b, 4.0)]), Some(a));
        // Challenger well under the hysteresis fraction: switch.
        assert_eq!(snap_pick(Some(a), &[(a, 5.0), (b, 2.0)]), Some(b));
        // Dead incumbent (not in candidates): nearest wins.
        assert_eq!(snap_pick(Some(a), &[(b, 9.0)]), Some(b));
    }

    #[test]
    fn component_lock_requires_focus() {
        let (mut world, _, [on_ray, _, _]) = focused_world();
        world.resource_mut::<SpaceshipPlayerComponentLock>().section = Some(on_ray);
        world.resource_mut::<SpaceshipPlayerLockFocus>().seconds = 0.5;

        world.run_system_once(update_component_lock).unwrap();

        assert_eq!(
            *world.resource::<SpaceshipPlayerComponentLock>(),
            SpaceshipPlayerComponentLock::default(),
            "an incomplete dwell clears the fine lock"
        );
    }

    #[test]
    fn snap_selects_the_section_nearest_the_aim_ray() {
        let (mut world, _, [on_ray, _, _]) = focused_world();

        world.run_system_once(update_component_lock).unwrap();

        assert_eq!(
            world.resource::<SpaceshipPlayerComponentLock>().section,
            Some(on_ray)
        );
    }

    #[test]
    fn cycle_steps_the_stable_order_and_pins() {
        let (mut world, _, [on_ray, near_ray, far_ray]) = focused_world();

        // Stable order is by local z: near_ray (0), on_ray (1), far_ray (2).
        cycle(&mut world, 1);
        let component = world.resource::<SpaceshipPlayerComponentLock>();
        assert_eq!(component.section, Some(near_ray));
        assert!(matches!(component.mode, ComponentLockMode::Pinned { .. }));

        cycle(&mut world, 1);
        assert_eq!(
            world.resource::<SpaceshipPlayerComponentLock>().section,
            Some(on_ray)
        );
        cycle(&mut world, 1);
        assert_eq!(
            world.resource::<SpaceshipPlayerComponentLock>().section,
            Some(far_ray)
        );
        // Wraps.
        cycle(&mut world, 1);
        assert_eq!(
            world.resource::<SpaceshipPlayerComponentLock>().section,
            Some(near_ray)
        );
        // Prev from a fresh (unselected) state starts at the tail.
        world.insert_resource(SpaceshipPlayerComponentLock::default());
        cycle(&mut world, -1);
        assert_eq!(
            world.resource::<SpaceshipPlayerComponentLock>().section,
            Some(far_ray)
        );
    }

    #[test]
    fn cycle_is_a_no_op_before_the_dwell_completes() {
        let (mut world, _, _) = focused_world();
        world.resource_mut::<SpaceshipPlayerLockFocus>().seconds = 0.5;

        cycle(&mut world, 1);

        assert_eq!(
            *world.resource::<SpaceshipPlayerComponentLock>(),
            SpaceshipPlayerComponentLock::default(),
            "cycling before focus completes must not select anything"
        );
    }

    #[test]
    fn pin_expires_back_to_snap() {
        let (mut world, _, [on_ray, near_ray, _]) = focused_world();
        cycle(&mut world, 1);
        assert_eq!(
            world.resource::<SpaceshipPlayerComponentLock>().section,
            Some(near_ray),
            "pinned to the first section in cycle order"
        );

        // The pin holds against snap while its window is open...
        world.run_system_once(update_component_lock).unwrap();
        assert_eq!(
            world.resource::<SpaceshipPlayerComponentLock>().section,
            Some(near_ray)
        );

        // ...and expires after COMPONENT_PIN_WINDOW, letting snap retake.
        world
            .resource_mut::<Time>()
            .advance_by(Duration::from_secs_f32(COMPONENT_PIN_WINDOW + 0.5));
        world.run_system_once(update_component_lock).unwrap();
        let component = world.resource::<SpaceshipPlayerComponentLock>();
        assert_eq!(component.mode, ComponentLockMode::Snap);
        assert_eq!(component.section, Some(on_ray));
    }

    #[test]
    fn pinned_section_death_reverts_to_snap() {
        let (mut world, _, [on_ray, near_ray, _]) = focused_world();
        cycle(&mut world, 1);
        assert_eq!(
            world.resource::<SpaceshipPlayerComponentLock>().section,
            Some(near_ray)
        );

        world.despawn(near_ray);
        world.run_system_once(update_component_lock).unwrap();

        let component = world.resource::<SpaceshipPlayerComponentLock>();
        assert_eq!(component.mode, ComponentLockMode::Snap);
        assert_eq!(
            component.section,
            Some(on_ray),
            "the dead pin falls back to the ray-nearest section"
        );
    }

    // -- multi-target candidate set + target cycle --

    fn entity(raw: u32) -> Entity {
        Entity::from_raw_u32(raw).unwrap()
    }

    #[test]
    fn rank_orders_by_aim_angle_then_distance() {
        let origin = Vec3::ZERO;
        let aim = Vec3::NEG_Z;
        let on_ray_far = entity(1);
        let off_ray_near = entity(2);
        let behind = entity(3);
        let ranked = rank_ship_candidates(
            origin,
            aim,
            [
                (behind, Vec3::new(0.0, 0.0, 100.0)),
                (off_ray_near, Vec3::new(30.0, 0.0, -50.0)),
                (on_ray_far, Vec3::new(1.0, 0.0, -500.0)),
            ]
            .into_iter(),
        );
        assert_eq!(
            ranked,
            vec![on_ray_far, off_ray_near, behind],
            "closest to the aim ray first; behind the ship ranks last but is still tracked"
        );
    }

    #[test]
    fn rank_breaks_angle_ties_by_distance() {
        let near = entity(1);
        let far = entity(2);
        let ranked = rank_ship_candidates(
            Vec3::ZERO,
            Vec3::NEG_Z,
            [
                (far, Vec3::new(0.0, 0.0, -800.0)),
                (near, Vec3::new(0.0, 0.0, -200.0)),
            ]
            .into_iter(),
        );
        assert_eq!(ranked, vec![near, far]);
    }

    #[test]
    fn maintain_keeps_the_top_n_and_the_locked_ship() {
        let ranked: Vec<Entity> = (1..=7).map(entity).collect();
        // Unpinned, no lock: plain top N.
        assert_eq!(
            maintain_candidates(&[], &ranked, None, false),
            ranked[..TARGET_CANDIDATE_COUNT].to_vec()
        );
        // The locked ship fell to rank 7: it replaces the worst entry.
        let entries = maintain_candidates(&[], &ranked, Some(entity(7)), false);
        assert_eq!(entries.len(), TARGET_CANDIDATE_COUNT);
        assert_eq!(entries[..4], ranked[..4]);
        assert_eq!(*entries.last().unwrap(), entity(7));
        // A lock that is not a ranked ship (asteroid, torpedo) is not forced in.
        assert_eq!(
            maintain_candidates(&[], &ranked, Some(entity(99)), false),
            ranked[..TARGET_CANDIDATE_COUNT].to_vec()
        );
    }

    #[test]
    fn maintain_is_order_stable_while_pinned() {
        let a = entity(1);
        let b = entity(2);
        let c = entity(3);
        let d = entity(4);
        // The rank reshuffled (b overtook a) and c died; pinned keeps the
        // survivors' order and appends the newcomer d at the tail.
        let prev = [a, b, c];
        let ranked = [b, a, d];
        assert_eq!(
            maintain_candidates(&prev, &ranked, Some(a), true),
            vec![a, b, d]
        );
        // Unpinned, the same inputs re-rank freely.
        assert_eq!(
            maintain_candidates(&prev, &ranked, Some(a), false),
            vec![b, a, d]
        );
    }

    /// Player at the origin aiming down -Z with two hostile ships: one dead
    /// ahead, one behind.
    fn multi_target_world() -> (World, Entity, Entity) {
        let mut world = World::new();
        spawn_acquisition_rig(&mut world);
        let ahead = world
            .spawn((
                SpaceshipRootMarker,
                AISpaceshipMarker,
                RigidBody::Dynamic,
                GlobalTransform::from_translation(Vec3::new(0.0, 0.0, -400.0)),
            ))
            .id();
        let behind = world
            .spawn((
                SpaceshipRootMarker,
                AISpaceshipMarker,
                RigidBody::Dynamic,
                GlobalTransform::from_translation(Vec3::new(0.0, 0.0, 400.0)),
            ))
            .id();
        (world, ahead, behind)
    }

    #[test]
    fn candidates_track_hostile_ships_only() {
        let (mut world, ahead, behind) = multi_target_world();
        // A neutral ship and a hostile committed torpedo: lockable, but not
        // multi-target candidates.
        world.spawn((
            SpaceshipRootMarker,
            Allegiance::Neutral,
            RigidBody::Dynamic,
            GlobalTransform::from_translation(Vec3::new(50.0, 0.0, -100.0)),
        ));
        world.spawn((
            TorpedoProjectileMarker,
            TorpedoTargetChosen,
            Allegiance::Enemy,
            RigidBody::Dynamic,
            GlobalTransform::from_translation(Vec3::new(0.0, 10.0, -200.0)),
        ));

        world
            .run_system_once(update_spaceship_target_input)
            .unwrap();

        assert_eq!(
            world.resource::<SpaceshipPlayerTargetCandidates>().entries,
            vec![ahead, behind],
            "hostile ships ranked aim-first; neutrals and torpedoes stay out"
        );
    }

    #[test]
    fn cycle_steps_the_candidates_wraps_and_pins() {
        let (mut world, ahead, behind) = multi_target_world();
        world
            .run_system_once(update_spaceship_target_input)
            .unwrap();
        assert_eq!(**world.resource::<SpaceshipPlayerTargetLock>(), Some(ahead));

        let cycle = |world: &mut World, direction: isize| {
            world
                .run_system_once(
                    move |time: Res<Time>,
                          mut lock: ResMut<SpaceshipPlayerTargetLock>,
                          mut candidates: ResMut<SpaceshipPlayerTargetCandidates>| {
                        step_target_lock(direction, &time, &mut lock, &mut candidates);
                    },
                )
                .unwrap();
        };

        cycle(&mut world, 1);
        assert_eq!(
            **world.resource::<SpaceshipPlayerTargetLock>(),
            Some(behind),
            "next steps off the aim pick to the next candidate"
        );
        assert!(
            world
                .resource::<SpaceshipPlayerTargetCandidates>()
                .pinned_until
                .is_some(),
            "a cycle press pins the lock"
        );
        cycle(&mut world, 1);
        assert_eq!(
            **world.resource::<SpaceshipPlayerTargetLock>(),
            Some(ahead),
            "wraps around"
        );
        cycle(&mut world, -1);
        assert_eq!(
            **world.resource::<SpaceshipPlayerTargetLock>(),
            Some(behind)
        );
    }

    #[test]
    fn an_expired_pin_leaves_the_lock_sticky_not_re_aimed() {
        // Under sticky-from-acquisition (task 20260712-203353), an expired pin
        // does NOT hand the lock back to the aim pick: the pinned target is
        // still a valid candidate, so it stays HELD. Only losing the target
        // (death / out of range) returns the lock to the picker.
        let (mut world, ahead, behind) = multi_target_world();
        // Pin the lock on the ship BEHIND while aiming at the one ahead.
        world.insert_resource(SpaceshipPlayerTargetLock(Some(behind)));
        world
            .resource_mut::<SpaceshipPlayerTargetCandidates>()
            .pinned_until = Some(TARGET_PIN_WINDOW);

        world
            .run_system_once(update_spaceship_target_input)
            .unwrap();
        assert_eq!(
            **world.resource::<SpaceshipPlayerTargetLock>(),
            Some(behind),
            "the pin holds against the cone pick"
        );

        // Past the deadline: the pin window clears, but the lock stays sticky
        // on `behind` (still in range) rather than re-aiming to `ahead`.
        world
            .resource_mut::<Time>()
            .advance_by(Duration::from_secs_f32(TARGET_PIN_WINDOW + 0.5));
        world
            .run_system_once(update_spaceship_target_input)
            .unwrap();
        assert_eq!(
            **world.resource::<SpaceshipPlayerTargetLock>(),
            Some(behind),
            "an expired pin leaves the lock sticky, not re-aimed"
        );
        assert_eq!(
            world
                .resource::<SpaceshipPlayerTargetCandidates>()
                .pinned_until,
            None,
            "the pin window still clears at its deadline"
        );

        // Delivery guard: once the held target leaves, the aim pick re-acquires
        // `ahead` - proving the stickiness above was a hold, not a wedged picker.
        world.despawn(behind);
        world
            .run_system_once(update_spaceship_target_input)
            .unwrap();
        assert_eq!(
            **world.resource::<SpaceshipPlayerTargetLock>(),
            Some(ahead),
            "with the held target gone, the aim pick re-acquires"
        );
    }

    #[test]
    fn pin_dies_with_its_target() {
        let (mut world, ahead, behind) = multi_target_world();
        world.insert_resource(SpaceshipPlayerTargetLock(Some(behind)));
        world
            .resource_mut::<SpaceshipPlayerTargetCandidates>()
            .pinned_until = Some(TARGET_PIN_WINDOW);

        world.despawn(behind);
        world
            .run_system_once(update_spaceship_target_input)
            .unwrap();

        assert_eq!(
            **world.resource::<SpaceshipPlayerTargetLock>(),
            Some(ahead),
            "a dead pinned target releases the lock to the picker"
        );
        assert_eq!(
            world
                .resource::<SpaceshipPlayerTargetCandidates>()
                .pinned_until,
            None
        );
    }

    #[test]
    fn cycle_with_no_candidates_is_a_no_op() {
        let mut world = World::new();
        world.insert_resource(Time::<()>::default());
        world.insert_resource(SpaceshipPlayerTargetLock(None));
        world.insert_resource(SpaceshipPlayerTargetCandidates::default());

        world
            .run_system_once(
                |time: Res<Time>,
                 mut lock: ResMut<SpaceshipPlayerTargetLock>,
                 mut candidates: ResMut<SpaceshipPlayerTargetCandidates>| {
                    step_target_lock(1, &time, &mut lock, &mut candidates);
                },
            )
            .unwrap();

        assert_eq!(**world.resource::<SpaceshipPlayerTargetLock>(), None);
        assert_eq!(
            world
                .resource::<SpaceshipPlayerTargetCandidates>()
                .pinned_until,
            None,
            "an empty cycle must not pin"
        );
    }

    /// End-to-end through the REAL flight rig and EnhancedInputPlugin with
    /// simulated devices: the wheel gesture is routed by the CTRL modifier -
    /// plain scroll steps the component fine-lock, CTRL+scroll steps the
    /// ship lock, and holding CTRL alone does NOTHING (bug 20260711-173237:
    /// a binding-level Chord ignores the binding's value, so the bare
    /// modifier press cycled the lock the moment CTRL went down).
    #[test]
    fn ctrl_routes_the_wheel_between_component_and_target_cycle() {
        use bevy::input::{
            gamepad::{
                GamepadConnection, GamepadConnectionEvent, RawGamepadButtonChangedEvent,
                RawGamepadEvent,
            },
            mouse::{MouseScrollUnit, MouseWheel},
            touch::TouchPhase,
            InputPlugin,
        };

        use crate::input::player::{flight_input_rig, FlightInputMarker};

        let mut app = App::new();
        app.add_plugins((MinimalPlugins, InputPlugin, EnhancedInputPlugin));
        // The cycle observers are pause-gated (task 20260711-185156).
        app.add_plugins(bevy::state::app::StatesPlugin);
        app.init_state::<crate::PauseStates>();
        app.add_input_context::<FlightInputMarker>();
        app.add_observer(on_component_cycle_next);
        app.add_observer(on_component_cycle_prev);
        app.add_observer(on_target_cycle_next);
        app.add_observer(on_target_cycle_prev);

        // A focused lock with two sections, plus a second tracked candidate.
        let locked = app.world_mut().spawn(SpaceshipRootMarker).id();
        let section_a = app
            .world_mut()
            .spawn((
                SectionMarker,
                Transform::from_translation(Vec3::new(0.0, 0.0, 0.0)),
                ChildOf(locked),
            ))
            .id();
        let section_b = app
            .world_mut()
            .spawn((
                SectionMarker,
                Transform::from_translation(Vec3::new(0.0, 0.0, 1.0)),
                ChildOf(locked),
            ))
            .id();
        let other = app.world_mut().spawn(SpaceshipRootMarker).id();
        app.insert_resource(SpaceshipPlayerTargetLock(Some(locked)));
        app.insert_resource(SpaceshipPlayerLockFocus {
            target: Some(locked),
            seconds: FOCUS_TIME,
        });
        app.insert_resource(SpaceshipPlayerComponentLock::default());
        app.insert_resource(SpaceshipPlayerTargetCandidates {
            entries: vec![locked, other],
            pinned_until: None,
        });

        // The context registry finalizes in App::finish, so run the plugin
        // lifecycle before spawning the rig, like the production app does.
        app.finish();
        app.cleanup();
        app.update();
        app.world_mut().spawn(flight_input_rig());
        app.update();

        let scroll_up = |app: &mut App| {
            app.world_mut().write_message(MouseWheel {
                unit: MouseScrollUnit::Line,
                x: 0.0,
                y: 1.0,
                window: Entity::PLACEHOLDER,
                // A mouse wheel always reports Moved (see bevy_input).
                phase: TouchPhase::Moved,
            });
            app.update();
            // Settle the impulse so the next scroll re-triggers Start.
            app.update();
        };

        // Plain scroll: component cycle only (cycle order is by local z:
        // section_a first), lock untouched.
        scroll_up(&mut app);
        assert_eq!(
            app.world()
                .resource::<SpaceshipPlayerComponentLock>()
                .section,
            Some(section_a),
            "plain scroll steps the component fine-lock"
        );
        assert_eq!(
            **app.world().resource::<SpaceshipPlayerTargetLock>(),
            Some(locked),
            "plain scroll must not touch the ship lock"
        );

        // Holding CTRL alone must change nothing (the reported bug). The
        // delivery guard: the modifier action itself must be firing.
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::ControlLeft);
        app.update();
        app.update();
        let modifier_fired = app
            .world_mut()
            .query_filtered::<&TriggerState, With<Action<TargetCycleModifierInput>>>()
            .iter(app.world())
            .any(|&state| state == TriggerState::Fired);
        assert!(modifier_fired, "the CTRL modifier action must be firing");
        assert_eq!(
            **app.world().resource::<SpaceshipPlayerTargetLock>(),
            Some(locked),
            "CTRL alone must not cycle the ship lock"
        );
        assert_eq!(
            app.world()
                .resource::<SpaceshipPlayerTargetCandidates>()
                .pinned_until,
            None,
            "CTRL alone must not pin"
        );
        assert_eq!(
            app.world()
                .resource::<SpaceshipPlayerComponentLock>()
                .section,
            Some(section_a),
            "CTRL alone must not cycle components either"
        );

        // CTRL+scroll: the same gesture one level up - ship lock cycles and
        // pins, the component selection stays put.
        scroll_up(&mut app);
        assert_eq!(
            **app.world().resource::<SpaceshipPlayerTargetLock>(),
            Some(other),
            "CTRL+scroll cycles the ship lock"
        );
        assert!(
            app.world()
                .resource::<SpaceshipPlayerTargetCandidates>()
                .pinned_until
                .is_some(),
            "the cycled lock is pinned"
        );
        assert_eq!(
            app.world()
                .resource::<SpaceshipPlayerComponentLock>()
                .section,
            Some(section_a),
            "CTRL+scroll must not also cycle components"
        );

        // Releasing CTRL hands the wheel back to the component cycle - the
        // lock stays where the cycle left it (the observers do not run the
        // acquisition system here, so no re-pick interferes). The component
        // step is a no-op because focus is on the OLD lock, which is the
        // gate's job - the wheel routing itself must not move the ship lock.
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .release(KeyCode::ControlLeft);
        app.update();
        scroll_up(&mut app);
        assert_eq!(
            **app.world().resource::<SpaceshipPlayerTargetLock>(),
            Some(other),
            "plain scroll after release must not cycle targets"
        );
        let _ = section_b;

        // The pad DPadUp binding cycles targets directly, no modifier held.
        let pad = app.world_mut().spawn_empty().id();
        app.world_mut().write_message(GamepadConnectionEvent::new(
            pad,
            GamepadConnection::Connected {
                name: "test pad".into(),
                vendor_id: None,
                product_id: None,
            },
        ));
        app.update();
        app.world_mut()
            .write_message(RawGamepadEvent::Button(RawGamepadButtonChangedEvent {
                gamepad: pad,
                button: GamepadButton::DPadUp,
                value: 1.0,
            }));
        app.update();
        assert_eq!(
            **app.world().resource::<SpaceshipPlayerTargetLock>(),
            Some(locked),
            "DPadUp cycles targets with no modifier (wraps back)"
        );
    }

    #[test]
    fn lock_loss_clears_the_component_lock() {
        let (mut world, _, [on_ray, _, _]) = focused_world();
        world.resource_mut::<SpaceshipPlayerComponentLock>().section = Some(on_ray);

        world.insert_resource(SpaceshipPlayerTargetLock(None));
        world.run_system_once(update_component_lock).unwrap();

        assert_eq!(
            *world.resource::<SpaceshipPlayerComponentLock>(),
            SpaceshipPlayerComponentLock::default()
        );
    }
}
