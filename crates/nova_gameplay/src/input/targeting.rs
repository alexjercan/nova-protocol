//! The player's target locks: DELIBERATE radar acquisition onto two sticky
//! ship-root lock slots (deliberate-radar spike 20260713-082207, task
//! 20260713-082330).
//!
//! - [`TravelLock`] (white crosshair): the nav designation GOTO reads.
//! - [`CombatLock`] (red crosshair): what guns/torpedoes/focus/inset read.
//! - Hold CTRL ([`RadarHoldInput`], a `Hold` condition) = radar on: the
//!   picker live-retargets to the best body on the ACTIVE look ray
//!   ([`ActiveLookRay`]). At the hold THRESHOLD the destination slot is
//!   latched from the CURRENT raised stance (combat while [`WeaponsRaised`],
//!   else travel - Q1a, spike 20260713-110039) and written LIVE with the
//!   candidate every held frame (keep-last: sweeping over empty space never
//!   drops the lock, Q2a). Releasing just ends the search - the lock
//!   sticks. A hold that never resolves a candidate leaves the slots
//!   untouched (D1).
//! - Tap CTRL ([`RadarClearInput`], a `Tap` condition) = staged clear: the
//!   combat lock first, then the travel lock (disengaging an engaged GOTO);
//!   while raised, only ever the combat lock.
//! - NOTHING locks passively: the old aim-assist cone auto-pick and the
//!   close-range signature auto-acquire are gone. Locks clear naturally on
//!   death/despawn, out-of-range, a hostile target turning non-hostile, and
//!   the combat lock decays after [`COMBAT_DECAY_SECS`] without combat
//!   activity.
//!
//! The scanner-wave RANGE model (LockSignature) survives as the radar
//! picker's gate, and [`ThreatContacts`] keeps the ranked hostile set alive
//! for the edge-indicator arrows. All state lives on the PLAYER ship root as
//! components (respawn hygiene; AI parity in 20260713-082337).

use avian3d::prelude::*;
use bevy::prelude::*;
#[cfg(test)]
use bevy_common_systems::prelude::PointRotationOutput;
use bevy_enhanced_input::prelude::{Cancel as ActionCancel, *};

use crate::prelude::*;

pub mod prelude {
    pub use super::{
        targeting_state, CombatLock, ComponentLock, ComponentLockMode, LockClearedToast, LockFocus,
        LockSignature, RadarDenied, RadarLockAcquired, RadarSlot, RadarState,
        SpaceshipTargetingPlugin, SpaceshipTargetingSystems, TargetingSettings, ThreatContacts,
        TravelLock, WeaponsHot, RADAR_TAP_SECS,
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
    /// battle debris, loose fragments. Point-blank by design (retuned 15 -> 5
    /// with the deliberate radar, spike 20260713-082207: debris at ~5 m).
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
            unsigned_lock_range: 5.0,
            range_hysteresis: 1.15,
            torpedo_lock_range: 2500.0,
        }
    }
}

/// The travel (nav) lock slot on the player ship root: the designation GOTO
/// reads. White crosshair. `None` = no designation. Sticky: only a radar
/// commit, a staged tap-clear, or a natural clear (death/out-of-range) moves
/// it.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Default, Reflect)]
#[reflect(Component)]
pub struct TravelLock(pub Option<Entity>);

/// The combat lock slot on the player ship root: guns, torpedo commit, focus
/// dwell, component fine-lock and the target inset read it; while it is Some
/// the weapons safety stays off (20260713-082337). Red crosshair. Sticky,
/// plus the [`COMBAT_DECAY_SECS`] idle decay.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Default, Reflect)]
#[reflect(Component)]
pub struct CombatLock(pub Option<Entity>);

/// The radar's destination slot, latched at the hold THRESHOLD from the
/// raised stance current at that moment (Q1a, spike 20260713-110039). The
/// old press-time latch carried a same-frame RMB+CTRL edge - the raised
/// flag derives in Update while the radar Start observer runs PreUpdate -
/// which the threshold latch retires: by then the stance has settled.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Reflect)]
pub enum RadarSlot {
    /// Writing the [`TravelLock`] (white crosshair).
    Travel,
    /// Writing the [`CombatLock`] (red crosshair).
    Combat,
}

/// Live radar search state, present on the player ship root ONLY while the
/// radar gesture is held. Inside the tap window nothing is latched or
/// written (a sub-threshold release is the Tap clear, not a lock); from the
/// threshold on, the engaged slot is written live every frame the candidate
/// resolves.
#[derive(Component, Debug, Clone, Copy, PartialEq, Default, Reflect)]
#[reflect(Component)]
pub struct RadarState {
    /// The latched destination slot; `None` until the hold threshold.
    pub engaged: Option<RadarSlot>,
    /// The current best candidate under the look ray (with hysteresis).
    /// `None` = searching empty space; the engaged slot keeps its last
    /// target (keep-last, Q2a).
    pub candidate: Option<Entity>,
    /// Whether this gesture has acquired yet - first-write bookkeeping for
    /// the once-per-gesture [`RadarLockAcquired`] cue (Q3a).
    pub acquired: bool,
}

/// The weapons safety, derived every frame on any ship carrying a
/// [`CombatLock`]: HOT (can fire) while the stance is raised OR a combat lock
/// exists; SAFE otherwise (deliberate-radar spike 20260713-082207, task
/// 20260713-082337). Ships WITHOUT the component are unmanaged and fire
/// freely (bare example turrets); the player gets it via
/// [`targeting_state`], AI ships via their combat mirror (input/ai.rs).
/// Enforced LIVE in the section fire systems - a held trigger stops the
/// frame the safety engages - plus a trigger-interrupt (the zeroed inputs
/// need a fresh press once hot again).
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Default, Reflect)]
#[reflect(Component)]
pub struct WeaponsHot(pub bool);

/// Idle bookkeeping for the combat-lock decay (decision D4): seconds since
/// the last combat activity while a combat lock exists. Reset by the raised
/// stance (and by firing, once 20260713-082337 lands); at
/// [`COMBAT_DECAY_SECS`] the combat lock clears and the safety follows.
#[derive(Component, Debug, Clone, Copy, PartialEq, Default, Reflect)]
#[reflect(Component)]
pub struct CombatDecay(pub f32);

/// The always-on ranked hostile combat set (top [`TARGET_CANDIDATE_COUNT`]
/// toward the look ray): the edge-indicator threat arrows read it (decision
/// D9 - the on-screen candidate list HUD is retired, the tracker is not).
#[derive(Component, Debug, Clone, PartialEq, Default, Reflect)]
#[reflect(Component)]
pub struct ThreatContacts {
    /// Ranked hostile combat targets, best first.
    pub entries: Vec<Entity>,
}

/// The targeting state bundle a player ship root carries (inserted by the
/// plugin's observer on [`PlayerSpaceshipMarker`]; AI parity gives AI ships
/// the lock/decay components in 20260713-082337).
pub fn targeting_state() -> impl Bundle {
    (
        TravelLock::default(),
        CombatLock::default(),
        CombatDecay::default(),
        LockFocus::default(),
        ComponentLock::default(),
        ThreatContacts::default(),
        WeaponsHot::default(),
    )
}

/// One radar gesture threshold (seconds), shared by the `Hold` (radar/commit)
/// and `Tap` (clear) conditions on CTRL - deriving both from one constant is
/// what keeps the boundary frame from falling in a gap between them.
pub const RADAR_TAP_SECS: f32 = 0.25;

/// Hold CTRL: radar on (live retarget); release past the threshold commits.
#[derive(InputAction)]
#[action_output(bool)]
pub(crate) struct RadarHoldInput;

/// Tap CTRL: staged lock clear.
#[derive(InputAction)]
#[action_output(bool)]
pub(crate) struct RadarClearInput;

/// One radar gesture acquired its first target - fired the first frame the
/// engaged slot RESOLVES a candidate (re-acquiring the target the slot
/// already held is still an acquisition; the slot write itself is an
/// equality-skip then), once per gesture (acquire-only, Q3a of spike
/// 20260713-110039), never on the live retargets that follow. The LockOn
/// cue reads this (consumer lands with task 20260713-110311).
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq)]
pub struct RadarLockAcquired {
    /// True when the combat slot acquired (red), false for travel (white).
    pub combat: bool,
}

/// A tap-clear just cleared a lock. The HUD's unlatch ghost (the crosshair
/// visibly popping off the target - the wordless replacement for the old
/// text toast, Q7a of spike 20260713-110039) and the LockOff cue read this.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq)]
pub struct LockClearedToast {
    /// True: the combat lock was cleared; false: the travel lock (and any
    /// engaged GOTO was disengaged with it).
    pub combat: bool,
    /// The target the lock held, so the ghost can anchor where the
    /// crosshair was (`None` only if the slot was somehow already empty).
    pub target: Option<Entity>,
}

/// A radar hold was denied because the ship's computer grants no Lock
/// capability (F7 - previously a silent no-op). The deny buzz + the radar
/// adornment flash read this (Q8a).
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq)]
pub struct RadarDenied;

/// System set for the lock update, so consumers (torpedo commit, turret
/// feed) can order after it.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct SpaceshipTargetingSystems;

/// Plugin owning the lock components, the radar gesture and the per-frame
/// contact/validity upkeep.
pub struct SpaceshipTargetingPlugin;

impl Plugin for SpaceshipTargetingPlugin {
    fn build(&self, app: &mut App) {
        debug!("SpaceshipTargetingPlugin: build");

        app.init_resource::<TargetingSettings>();
        app.register_type::<TargetingSettings>();
        app.register_type::<LockSignature>();
        app.register_type::<TravelLock>();
        app.register_type::<CombatLock>();
        app.register_type::<RadarState>();
        app.register_type::<RadarSlot>();
        app.register_type::<CombatDecay>();
        app.register_type::<ThreatContacts>();
        app.register_type::<WeaponsHot>();
        app.register_type::<LockFocus>();
        app.register_type::<ComponentLock>();
        app.add_message::<LockClearedToast>();
        app.add_message::<RadarLockAcquired>();
        app.add_message::<RadarDenied>();

        // The state bundle rides the player marker wherever ships spawn
        // (observer-over-spawn-site).
        app.add_observer(insert_targeting_state);

        app.add_systems(
            Update,
            (
                update_contacts_and_locks,
                update_radar_search,
                update_weapons_safety,
                enforce_safety_trigger_interrupt,
                tick_lock_focus,
                update_component_lock,
            )
                .chain()
                .in_set(SpaceshipTargetingSystems)
                .in_set(super::SpaceshipInputSystems),
        );
        app.add_observer(on_radar_start);
        app.add_observer(on_radar_commit);
        app.add_observer(on_radar_cancel);
        app.add_observer(on_lock_clear_tap);
        app.add_observer(on_component_cycle_next);
        app.add_observer(on_component_cycle_prev);
    }
}

/// Give every player ship root its targeting state the moment it is marked.
fn insert_targeting_state(add: On<Add, PlayerSpaceshipMarker>, mut commands: Commands) {
    if let Ok(mut ship) = commands.get_entity(add.entity) {
        ship.insert(targeting_state());
    }
}

/// Maximum distance at which the aim-assist will lock a target - the
/// ceiling for the intrinsic classes (well bodies, ships), which stay
/// designatable from across the play area for GOTO legs (user report
/// 20260710). Everything else is gated far shorter by the signature model
/// (the later 20260710 report: long-range locks should only see large
/// objects), see [`LockSignature`] and [`TargetingSettings`].
const TARGETING_MAX_RANGE: f32 = 20_000.0;

/// Half-angle (degrees) of the RADAR cone around the look ray. While the
/// radar is held, any lockable body within this angle of where the player is
/// looking is a candidate and the one closest to the ray wins (with
/// [`RADAR_PICK_HYSTERESIS`]); the wide cone means rough pointing suffices,
/// no pixel-perfect ray needed.
const TARGETING_CONE_HALF_ANGLE_DEG: f32 = 18.0;

/// Provisional-candidate hysteresis (decision D7): while the radar is held,
/// a challenger only steals the candidate when its angular distance to the
/// ray (measured as `1 - cos`) is below this fraction of the incumbent's -
/// otherwise two near-collinear bodies (a torpedo and its launcher) strobe
/// the candidate and the release commits a coin flip.
const RADAR_PICK_HYSTERESIS: f32 = 0.75;

/// Seconds without combat activity (raised stance; firing joins in
/// 20260713-082337) before a held combat lock decays and the weapons safety
/// re-engages (decision D4, user-tuned). A const knob.
const COMBAT_DECAY_SECS: f32 = 30.0;

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

/// Focus: how long the COMBAT lock has been held on the same target.
/// Component fine-locking unlocks at [`FOCUS_TIME`]; the HUD renders the
/// fill fraction while it accumulates. On the player ship root; the
/// provisional radar candidate never touches it - only a committed lock
/// accrues dwell.
#[derive(Component, Debug, Clone, PartialEq, Default, Reflect)]
#[reflect(Component)]
pub struct LockFocus {
    /// The target the timer is accumulating on (mirrors the combat lock).
    pub target: Option<Entity>,
    /// Continuous seconds the lock has stayed on `target`.
    pub seconds: f32,
}

impl LockFocus {
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
#[derive(Debug, Clone, Copy, PartialEq, Default, Reflect)]
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

/// The fine-locked section of the combat-locked ship, only ever `Some` while
/// the focus dwell is complete. Sections stay lockable while ATTACHED - a
/// disabled-in-place section (`SectionInactiveMarker`) can still be targeted
/// to blow it off the hull; despawn/detach clears the selection (decision
/// from the component-lock spike, lockable-while-attached). On the player
/// ship root.
#[derive(Component, Debug, Clone, PartialEq, Default, Reflect)]
#[reflect(Component)]
pub struct ComponentLock {
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

/// How many hostile contacts the threat tracker keeps for the edge
/// indicators. A feel knob; more would clutter the HUD.
const TARGET_CANDIDATE_COUNT: usize = 5;

/// A collected lockable body: entity, world position, hostile-to-player,
/// combat-target (ship or committed torpedo).
type Lockable = (Entity, Vec3, bool, bool);

/// The scanner query every collection pass walks. Turret bullets are excluded
/// outright: they are dynamic bodies that stream straight down the aim ray.
type LockableQuery<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static GlobalTransform,
        &'static RigidBody,
        Option<&'static GravityWell>,
        Option<&'static LockSignature>,
        Has<SpaceshipRootMarker>,
        Option<&'static TorpedoProjectileMarker>,
        Option<&'static TorpedoTargetChosen>,
        Option<&'static Allegiance>,
    ),
    Without<TurretBulletProjectileMarker>,
>;

/// Collect every body the scanner can currently see from `origin`, applying
/// the LockSignature range model at collection so every consumer (the radar
/// pick, lock validity, the threat set) inherits it:
///
/// - Only physical, movable bodies are lockable. This skips static sensor
///   volumes such as scenario trigger areas (`RigidBody::Static`), which are
///   invisible and must never be locked. Two exceptions sit on rails (Static)
///   yet are visible things the player navigates by: gravity-well sources and
///   bodies with an AUTHORED LockSignature (nav beacons) - trigger areas
///   never carry a signature, so the invisible-statics rule holds.
/// - A freshly launched torpedo that has not committed its target yet is
///   skipped (it spawns right on the aim ray); once committed it is a normal
///   lockable body.
/// - Range: well bodies and ships return a signature at any range; committed
///   torpedoes at combat range; signed bodies at `signature * range/unit`
///   (floored at the debris range); unsigned debris only point-blank.
/// - `incumbents` (current locks / the radar candidate) hold a little beyond
///   their gate ([`TargetingSettings::range_hysteresis`]) so a body at the
///   boundary cannot strobe its lock as the ship drifts.
fn collect_lockable(
    q_candidates: &LockableQuery,
    settings: &TargetingSettings,
    origin: Vec3,
    ship_entity: Entity,
    ship_allegiance: Option<&Allegiance>,
    incumbents: &[Option<Entity>],
) -> Vec<Lockable> {
    q_candidates
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
                if is_torpedo.is_some() && torpedo_committed.is_none() {
                    return None;
                }
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
                if incumbents.contains(&Some(entity)) {
                    max_range *= settings.range_hysteresis.max(1.0);
                }
                let position = transform.translation();
                if position.distance_squared(origin) > max_range * max_range {
                    return None;
                }
                let is_hostile = relation(ship_allegiance, allegiance) == Relation::Hostile;
                let is_combat_target = is_ship || is_torpedo.is_some();
                Some((entity, position, is_hostile, is_combat_target))
            },
        )
        .collect()
}

/// Per-frame lock upkeep, always on: hold the LOCKS only while their targets
/// stay collectible (death/despawn and out-of-range clear them - stickiness
/// never needs a re-pick because NOTHING re-picks), clear the combat lock
/// when a hostile target turns non-hostile, tick the [`CombatDecay`] idle
/// clock (decision D4), and maintain the ranked hostile [`ThreatContacts`]
/// for the edge indicators (decision D9).
#[allow(clippy::type_complexity)]
fn update_contacts_and_locks(
    time: Res<Time>,
    look_ray: ActiveLookRay,
    settings: Res<TargetingSettings>,
    q_candidates: LockableQuery,
    q_flipped: Query<(), Changed<Allegiance>>,
    q_allegiances: Query<&Allegiance>,
    mut spaceship: Query<
        (
            &Transform,
            Option<&ComputedCenterOfMass>,
            Entity,
            Option<&Allegiance>,
            &mut TravelLock,
            &mut CombatLock,
            &mut CombatDecay,
            &mut ThreatContacts,
            Option<&WeaponsRaised>,
        ),
        (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>),
    >,
) {
    for (
        transform,
        com,
        ship,
        ship_allegiance,
        mut travel,
        mut combat,
        mut decay,
        mut threats,
        raised,
    ) in &mut spaceship
    {
        // Cone origin on the live structure, not the root origin, so the
        // scanner agrees with the COM-anchored crosshair after losing
        // sections (task 20260709-150711).
        let origin = live_structure_anchor(transform, com);
        let candidates = collect_lockable(
            &q_candidates,
            &settings,
            origin,
            ship,
            ship_allegiance,
            &[travel.0, combat.0],
        );

        // Validity: a lock holds exactly while its target is collectible.
        let still = |target: Option<Entity>| {
            target.filter(|target| candidates.iter().any(|&(entity, ..)| entity == *target))
        };
        let travel_now = still(travel.0);
        if travel.0 != travel_now {
            travel.0 = travel_now;
        }
        let mut combat_now = still(combat.0);

        // A hostile combat target FLIPPING to non-hostile clears the lock (a
        // scripted surrender must not keep the guns hot); a deliberate lock
        // on an always-neutral body is untouched - only a CHANGE trips this.
        combat_now = combat_now.filter(|target| {
            !(q_flipped.get(*target).is_ok()
                && relation(ship_allegiance, q_allegiances.get(*target).ok()) != Relation::Hostile)
        });

        // The idle decay (D4): combat activity (the raised stance; firing
        // joins in 20260713-082337) resets the clock; at COMBAT_DECAY_SECS
        // the lock lets go and the safety follows.
        if combat_now.is_some() {
            if raised.is_some_and(|raised| raised.0) {
                decay.set_if_neq(CombatDecay(0.0));
            } else {
                decay.0 += time.delta_secs();
                if decay.0 >= COMBAT_DECAY_SECS {
                    combat_now = None;
                    decay.set_if_neq(CombatDecay(0.0));
                }
            }
        } else {
            decay.set_if_neq(CombatDecay(0.0));
        }
        if combat.0 != combat_now {
            combat.0 = combat_now;
        }

        // The threat set: hostile combat targets ranked toward the look ray
        // (ship-forward fallback keeps the arrows meaningful rig-less).
        let aim = look_ray
            .direction()
            .unwrap_or_else(|| (transform.rotation * Vec3::NEG_Z).normalize());
        let ranked = rank_combat_targets(
            origin,
            aim,
            candidates
                .iter()
                .filter(|&&(_, _, is_hostile, is_combat)| is_hostile && is_combat)
                .map(|&(entity, position, ..)| (entity, position)),
        );
        let entries = maintain_contacts(&ranked, combat.0);
        if threats.entries != entries {
            threats.entries = entries;
        }
    }
}

/// The radar search AND the live lock (spike 20260713-110039, strand A1):
/// while a [`RadarState`] exists (the hold gesture is active), live-retarget
/// the candidate to the best body on the look ray, with incumbent hysteresis
/// (decision D7) so two near-collinear bodies do not strobe the pick. Once
/// the hold crosses its threshold (the action reports `Fired`), the
/// destination slot is latched from the CURRENT raised stance (Q1a) and
/// written with the candidate every frame it resolves - the lock is LIVE
/// under the sweep; releasing merely stops the retargeting. A `None`
/// candidate never writes (keep-last, Q2a), so the tap window (pre-Fired)
/// and an empty sweep both leave the slots alone. Runs inside the
/// pause-gated input set, so a pause freezes the writes while the release
/// observers still tear the search down.
#[allow(clippy::type_complexity)]
fn update_radar_search(
    look_ray: ActiveLookRay,
    settings: Res<TargetingSettings>,
    q_candidates: LockableQuery,
    q_hold: Query<&TriggerState, With<Action<RadarHoldInput>>>,
    mut acquired_cue: MessageWriter<RadarLockAcquired>,
    mut spaceship: Query<
        (
            &Transform,
            Option<&ComputedCenterOfMass>,
            Entity,
            Option<&Allegiance>,
            Option<&WeaponsRaised>,
            &mut RadarState,
            &mut TravelLock,
            &mut CombatLock,
            &mut CombatDecay,
        ),
        (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>),
    >,
) {
    let hold_fired = q_hold.iter().any(|&state| state == TriggerState::Fired);
    for (
        transform,
        com,
        ship,
        ship_allegiance,
        raised,
        mut radar,
        mut travel,
        mut combat,
        mut decay,
    ) in &mut spaceship
    {
        let Some(aim_rotation) = look_ray.rotation() else {
            continue;
        };
        let origin = live_structure_anchor(transform, com);
        let aim = (aim_rotation * Vec3::NEG_Z).normalize();
        let candidates = collect_lockable(
            &q_candidates,
            &settings,
            origin,
            ship,
            ship_allegiance,
            &[radar.candidate],
        );
        let picked = radar_pick(
            radar.candidate,
            origin,
            aim,
            TARGETING_CONE_HALF_ANGLE_DEG.to_radians().cos(),
            &candidates,
        );
        if radar.candidate != picked {
            radar.candidate = picked;
        }

        // Tap window: nothing latches, nothing writes.
        if !hold_fired {
            continue;
        }
        let slot = *radar.engaged.get_or_insert_with(|| {
            if raised.is_some_and(|raised| raised.0) {
                RadarSlot::Combat
            } else {
                RadarSlot::Travel
            }
        });
        // An engaged combat sweep IS combat activity: hold the decay at zero
        // even across equality-skip frames (F12 - a long sweep must not
        // cross the decay boundary mid-gesture).
        if slot == RadarSlot::Combat && decay.0 != 0.0 {
            decay.0 = 0.0;
        }
        let Some(candidate) = radar.candidate else {
            continue;
        };
        match slot {
            RadarSlot::Combat => {
                if combat.0 != Some(candidate) {
                    combat.0 = Some(candidate);
                }
            }
            RadarSlot::Travel => {
                if travel.0 != Some(candidate) {
                    travel.0 = Some(candidate);
                }
            }
        }
        if !radar.acquired {
            radar.acquired = true;
            acquired_cue.write(RadarLockAcquired {
                combat: slot == RadarSlot::Combat,
            });
        }
    }
}

/// The radar's pick rule: the body nearest the look ray inside the cone,
/// except that an incumbent candidate
/// holds unless the challenger is DECISIVELY nearer the ray - its angular
/// distance (as `1 - cos`) under [`RADAR_PICK_HYSTERESIS`] of the
/// incumbent's. Leaving the cone entirely drops the candidate (a release
/// then commits nothing - the abort gesture, decision D1).
///
/// Pure so the hysteresis rule is unit-testable.
fn radar_pick(
    current: Option<Entity>,
    origin: Vec3,
    aim: Vec3,
    min_cos: f32,
    candidates: &[Lockable],
) -> Option<Entity> {
    let scored: Vec<(Entity, f32)> = candidates
        .iter()
        .filter_map(|&(entity, position, ..)| {
            let to_target = position - origin;
            let distance = to_target.length();
            if distance < f32::EPSILON {
                return None;
            }
            let cos_angle = to_target.normalize().dot(aim);
            (cos_angle >= min_cos).then_some((entity, cos_angle))
        })
        .collect();
    let (best, best_cos) = scored
        .iter()
        .copied()
        .max_by(|(_, a), (_, b)| a.total_cmp(b))?;
    if let Some(current) = current {
        if let Some(&(_, current_cos)) = scored.iter().find(|(entity, _)| *entity == current) {
            if best != current && (1.0 - best_cos) >= RADAR_PICK_HYSTERESIS * (1.0 - current_cos) {
                return Some(current);
            }
        }
    }
    Some(best)
}

/// Rank the lockable hostile COMBAT targets (ships + committed torpedoes) for
/// the threat set: nearest the look ray first (largest cosine), distance as
/// the tie-breaker. Not cone-gated - a hostile behind the player is still
/// tracked (the edge-indicator overlay points at it).
///
/// Pure and camera/physics-free so the ranking rule can be unit-tested.
fn rank_combat_targets(
    origin: Vec3,
    aim: Vec3,
    targets: impl Iterator<Item = (Entity, Vec3)>,
) -> Vec<Entity> {
    let mut scored: Vec<(Entity, f32, f32)> = targets
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

/// Compose the threat entries: the top [`TARGET_CANDIDATE_COUNT`] by rank,
/// with the combat lock kept a member while it is still ranked (the arrow to
/// your own target must never vanish).
fn maintain_contacts(ranked: &[Entity], combat_lock: Option<Entity>) -> Vec<Entity> {
    let mut entries: Vec<Entity> = ranked
        .iter()
        .copied()
        .take(TARGET_CANDIDATE_COUNT)
        .collect();
    if let Some(lock) = combat_lock {
        if ranked.contains(&lock) && !entries.contains(&lock) {
            entries.pop();
            entries.push(lock);
        }
    }
    entries
}

/// Whether `ship` has a live controller section granting `verb` - the
/// computer-capability gate (mirrors player.rs's `ship_grants_verb`; the
/// radar needs it here for the Lock capability).
fn ship_grants_lock(
    ship: Entity,
    q_controllers: &Query<
        (&ChildOf, Option<&WithheldVerbs>),
        (
            With<ControllerSectionMarker>,
            Without<SectionInactiveMarker>,
        ),
    >,
) -> bool {
    q_controllers.iter().any(|(ChildOf(parent), withheld)| {
        *parent == ship && withheld.is_none_or(|w| w.granted(FlightVerb::Lock))
    })
}

/// Start of the radar hold: open the search. The destination slot is NOT
/// decided here - it latches at the hold threshold, in the live search
/// (Q1a). Gated on the computer's Lock capability and, like every intent
/// observer, on the pause overlay.
#[allow(clippy::type_complexity)]
fn on_radar_start(
    _: On<Start<RadarHoldInput>>,
    mut commands: Commands,
    pause: Res<State<crate::PauseStates>>,
    mut denied: MessageWriter<RadarDenied>,
    q_controllers: Query<
        (&ChildOf, Option<&WithheldVerbs>),
        (
            With<ControllerSectionMarker>,
            Without<SectionInactiveMarker>,
        ),
    >,
    q_ship: Query<Entity, (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>)>,
) {
    // Observers bypass system-set gating; freeze intent changes while the
    // pause overlay is up. Releases stay ungated so held keys clear cleanly
    // during a pause.
    if *pause.get() == crate::PauseStates::Paused {
        return;
    }
    for ship in &q_ship {
        if !ship_grants_lock(ship, &q_controllers) {
            // No Lock capability on this computer: the radar does not come
            // on - and says so (deny buzz + adornment flash, F7/Q8a).
            denied.write(RadarDenied);
            continue;
        }
        commands.entity(ship).insert(RadarState::default());
    }
}

/// Radar teardown, shared by both release paths: with the lock written live
/// under the sweep (strand A1), a release has nothing to commit - it only
/// closes the search. Deliberately not pause-gated: state cleanup must
/// always run, like the release observers elsewhere.
fn close_radar_search(
    commands: &mut Commands,
    q_ship: &Query<
        Entity,
        (
            With<SpaceshipRootMarker>,
            With<PlayerSpaceshipMarker>,
            With<RadarState>,
        ),
    >,
) {
    for ship in q_ship {
        commands.entity(ship).remove::<RadarState>();
    }
}

/// Radar release past the hold threshold (`Complete`): the lock already
/// holds whatever the sweep last resolved - it sticks; just close the
/// search.
fn on_radar_commit(
    _: On<Complete<RadarHoldInput>>,
    mut commands: Commands,
    q_ship: Query<
        Entity,
        (
            With<SpaceshipRootMarker>,
            With<PlayerSpaceshipMarker>,
            With<RadarState>,
        ),
    >,
) {
    close_radar_search(&mut commands, &q_ship);
}

/// Radar release BEFORE the hold threshold (`Cancel`): nothing was latched
/// or written yet; close the search - the same physical release fires the
/// Tap clear separately.
fn on_radar_cancel(
    _: On<ActionCancel<RadarHoldInput>>,
    mut commands: Commands,
    q_ship: Query<
        Entity,
        (
            With<SpaceshipRootMarker>,
            With<PlayerSpaceshipMarker>,
            With<RadarState>,
        ),
    >,
) {
    close_radar_search(&mut commands, &q_ship);
}

/// The tap clear (decision D3a, staged): lowered - the combat lock first,
/// else the travel lock (disengaging an engaged GOTO with it); raised - only
/// ever the combat lock. Emits [`LockClearedToast`] so the mode-scoped
/// gesture is legible on the HUD.
#[allow(clippy::type_complexity)]
fn on_lock_clear_tap(
    _: On<Fire<RadarClearInput>>,
    mut commands: Commands,
    pause: Res<State<crate::PauseStates>>,
    mut toasts: MessageWriter<LockClearedToast>,
    mut q_ship: Query<
        (
            Entity,
            Option<&WeaponsRaised>,
            &mut TravelLock,
            &mut CombatLock,
            Option<&Autopilot>,
        ),
        (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>),
    >,
) {
    if *pause.get() == crate::PauseStates::Paused {
        return;
    }
    for (ship, raised, mut travel, mut combat, autopilot) in &mut q_ship {
        let raised = raised.is_some_and(|raised| raised.0);
        if combat.0.is_some() {
            let target = combat.0.take();
            toasts.write(LockClearedToast {
                combat: true,
                target,
            });
        } else if !raised && travel.0.is_some() {
            let target = travel.0.take();
            toasts.write(LockClearedToast {
                combat: false,
                target,
            });
            // Clearing the designation disengages an engaged GOTO (decision
            // from the recap Q&A); other maneuvers (STOP/ORBIT) are not
            // lock-bound and keep flying.
            if autopilot
                .is_some_and(|autopilot| matches!(autopilot.action, AutopilotAction::Goto { .. }))
            {
                commands.entity(ship).remove::<Autopilot>();
            }
        }
    }
}

/// Derive the weapons safety on every managed ship: HOT while raised or
/// combat-locked, SAFE otherwise. Generic over player and AI (the AI mirror
/// maintains its CombatLock/WeaponsRaised; a ship without WeaponsHot is
/// unmanaged and unaffected).
fn update_weapons_safety(
    mut q_ships: Query<(Option<&WeaponsRaised>, &CombatLock, &mut WeaponsHot)>,
) {
    update_weapons_safety_impl(&mut q_ships);
}

/// Test-visible alias (the AI mirror test drives the REAL derivation).
#[cfg(test)]
pub(crate) fn update_weapons_safety_for_tests(
    mut q_ships: Query<(Option<&WeaponsRaised>, &CombatLock, &mut WeaponsHot)>,
) {
    update_weapons_safety_impl(&mut q_ships);
}

fn update_weapons_safety_impl(
    q_ships: &mut Query<(Option<&WeaponsRaised>, &CombatLock, &mut WeaponsHot)>,
) {
    for (raised, lock, mut hot) in q_ships.iter_mut() {
        let next = raised.is_some_and(|raised| raised.0) || lock.0.is_some();
        hot.set_if_neq(WeaponsHot(next));
    }
}

/// The trigger-interrupt (adversarial finding: the fire inputs are LATCHED
/// bools, so a press-time gate alone cannot stop a held burst): the frame the
/// safety engages, zero every weapon input on that ship - resuming fire once
/// hot again requires a fresh press. The section fire systems ALSO check
/// [`WeaponsHot`] live, so even a same-frame race cannot leak a shot.
fn enforce_safety_trigger_interrupt(
    q_ships: Query<(Entity, &WeaponsHot), Changed<WeaponsHot>>,
    mut q_turrets: Query<(&mut TurretSectionInput, &ChildOf)>,
    mut q_torpedoes: Query<(&mut TorpedoSectionInput, &ChildOf), Without<TurretSectionInput>>,
) {
    for (ship, hot) in &q_ships {
        if hot.0 {
            continue;
        }
        for (mut input, ChildOf(parent)) in &mut q_turrets {
            if *parent == ship && **input {
                **input = false;
            }
        }
        for (mut input, ChildOf(parent)) in &mut q_torpedoes {
            if *parent == ship && **input {
                **input = false;
            }
        }
    }
}

/// Accumulate focus while the COMBAT lock stays on one target; any change
/// (new target or lock lost) restarts the dwell from zero. Generic over any
/// ship carrying the components (AI parity, 20260713-082337).
fn tick_lock_focus(time: Res<Time>, mut q_ships: Query<(&CombatLock, &mut LockFocus)>) {
    for (lock, mut focus) in &mut q_ships {
        if focus.target != lock.0 {
            focus.target = lock.0;
            focus.seconds = 0.0;
            continue;
        }
        if focus.target.is_some() {
            focus.seconds += time.delta_secs();
        }
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
    q_sections: Query<(Entity, &ChildOf, &GlobalTransform), With<SectionMarker>>,
    // The LIVE look ray (active rig), so the snap follows the crosshair in
    // every view instead of the turret rig's frozen output (task
    // 20260713-082324).
    look_ray: ActiveLookRay,
    mut q_ship: Query<
        (
            &Transform,
            Option<&ComputedCenterOfMass>,
            &CombatLock,
            &LockFocus,
            &mut ComponentLock,
        ),
        (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>),
    >,
) {
    for (ship_transform, ship_com, lock, focus, mut component) in &mut q_ship {
        // The component layer only exists while focused on the combat lock.
        let target = match lock.0 {
            Some(target) if focus.focused_on(target) => target,
            _ => {
                component.set_if_neq(ComponentLock::default());
                continue;
            }
        };

        let sections: Vec<(Entity, Vec3)> = q_sections
            .iter()
            .filter(|(_, ChildOf(parent), _)| *parent == target)
            .map(|(entity, _, transform)| (entity, transform.translation()))
            .collect();
        if sections.is_empty() {
            component.set_if_neq(ComponentLock::default());
            continue;
        }

        // Detach/despawn invalidates the selection (inactive sections stay
        // lockable - see ComponentLock).
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
            continue;
        }
        let Some(aim_rotation) = look_ray.rotation() else {
            // No aim rig (menu states, headless tests): hold the current
            // selection rather than guessing.
            continue;
        };
        let origin = live_structure_anchor(ship_transform, ship_com);
        let dir = (aim_rotation * Vec3::NEG_Z).normalize();
        let candidates: Vec<(Entity, f32)> = sections
            .iter()
            .map(|&(entity, position)| (entity, ray_distance(origin, dir, position)))
            .collect();
        let picked = snap_pick(component.section, &candidates);
        if component.section != picked {
            component.section = picked;
        }
    }
}

/// Shared body of the cycle observers: step the fine lock through the locked
/// ship's attached sections in [`cycle_order`] and pin the choice for
/// [`COMPONENT_PIN_WINDOW`] seconds.
fn step_component_lock(
    direction: isize,
    time: &Time,
    lock: &CombatLock,
    focus: &LockFocus,
    component: &mut ComponentLock,
    q_sections: &Query<(Entity, &ChildOf, &Transform), With<SectionMarker>>,
) {
    let target = match lock.0 {
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

fn on_component_cycle_next(
    _: On<Start<ComponentCycleNextInput>>,
    time: Res<Time>,
    q_sections: Query<(Entity, &ChildOf, &Transform), With<SectionMarker>>,
    mut q_ship: Query<
        (&CombatLock, &LockFocus, &mut ComponentLock),
        (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>),
    >,
    pause: Res<State<crate::PauseStates>>,
) {
    // Observers bypass system-set gating; freeze intent changes while the
    // pause overlay is up (review R1.1). Releases stay ungated so held keys
    // clear cleanly during a pause.
    if *pause.get() == crate::PauseStates::Paused {
        return;
    }
    for (lock, focus, mut component) in &mut q_ship {
        step_component_lock(1, &time, lock, focus, &mut component, &q_sections);
    }
}

fn on_component_cycle_prev(
    _: On<Start<ComponentCyclePrevInput>>,
    time: Res<Time>,
    q_sections: Query<(Entity, &ChildOf, &Transform), With<SectionMarker>>,
    mut q_ship: Query<
        (&CombatLock, &LockFocus, &mut ComponentLock),
        (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>),
    >,
    pause: Res<State<crate::PauseStates>>,
) {
    // Observers bypass system-set gating; freeze intent changes while the
    // pause overlay is up (review R1.1). Releases stay ungated so held keys
    // clear cleanly during a pause.
    if *pause.get() == crate::PauseStates::Paused {
        return;
    }
    for (lock, focus, mut component) in &mut q_ship {
        step_component_lock(-1, &time, lock, focus, &mut component, &q_sections);
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use bevy::ecs::system::RunSystemOnce;

    use super::*;

    fn cone_cos(half_angle_deg: f32) -> f32 {
        half_angle_deg.to_radians().cos()
    }

    // -- pure pick rules --

    #[test]
    fn radar_picks_the_body_nearest_the_aim_ray() {
        let origin = Vec3::ZERO;
        let aim = Vec3::NEG_Z;
        let near_center = Entity::from_raw_u32(1).unwrap();
        let off_center = Entity::from_raw_u32(2).unwrap();
        let candidates = [
            // ~1.1 deg off axis, far vs ~8.5 deg off axis, near: the
            // nearer-to-center one wins even though it is further away.
            (near_center, Vec3::new(2.0, 0.0, -100.0), false, true),
            (off_center, Vec3::new(3.0, 0.0, -20.0), false, true),
        ];
        let picked = radar_pick(None, origin, aim, cone_cos(18.0), &candidates);
        assert_eq!(picked, Some(near_center));
    }

    #[test]
    fn radar_ignores_bodies_outside_the_cone_or_behind() {
        let side = [(
            Entity::from_raw_u32(1).unwrap(),
            Vec3::new(50.0, 0.0, 0.0),
            false,
            true,
        )];
        assert_eq!(
            radar_pick(None, Vec3::ZERO, Vec3::NEG_Z, cone_cos(18.0), &side),
            None,
            "a body outside the cone must not be picked"
        );
        let behind = [(
            Entity::from_raw_u32(1).unwrap(),
            Vec3::new(0.0, 0.0, 100.0),
            false,
            true,
        )];
        assert_eq!(
            radar_pick(None, Vec3::ZERO, Vec3::NEG_Z, cone_cos(18.0), &behind),
            None,
            "a body behind the ship must not be picked"
        );
        assert_eq!(
            radar_pick(None, Vec3::ZERO, Vec3::NEG_Z, cone_cos(18.0), &[]),
            None
        );
    }

    /// D7: the provisional candidate holds against a marginally-nearer
    /// challenger and yields to a decisively-nearer one; leaving the cone
    /// drops it entirely (the release-abort).
    #[test]
    fn radar_pick_applies_angular_hysteresis() {
        let a = Entity::from_raw_u32(1).unwrap();
        let b = Entity::from_raw_u32(2).unwrap();
        let origin = Vec3::ZERO;
        let aim = Vec3::NEG_Z;
        let min_cos = cone_cos(18.0);
        // a at ~8 deg off-ray, b at ~7.4 deg: nearer, but NOT decisively
        // ((1-cos7.4) ~ 0.0084 vs 0.75 * (1-cos8) ~ 0.0073).
        let marginal = [
            (a, Vec3::new(14.0, 0.0, -100.0), false, true),
            (b, Vec3::new(13.0, 0.0, -100.0), false, true),
        ];
        assert_eq!(
            radar_pick(Some(a), origin, aim, min_cos, &marginal),
            Some(a),
            "a marginally-nearer challenger must not steal the candidate"
        );
        // b dead on the ray: decisive.
        let decisive = [
            (a, Vec3::new(14.0, 0.0, -100.0), false, true),
            (b, Vec3::new(0.1, 0.0, -100.0), false, true),
        ];
        assert_eq!(
            radar_pick(Some(a), origin, aim, min_cos, &decisive),
            Some(b),
            "a decisively-nearer challenger takes the candidate"
        );
        // No incumbent: plain nearest wins.
        assert_eq!(radar_pick(None, origin, aim, min_cos, &marginal), Some(b));
        // Cone empty: candidate drops (the abort).
        let outside = [(a, Vec3::new(100.0, 0.0, 0.0), false, true)];
        assert_eq!(radar_pick(Some(a), origin, aim, min_cos, &outside), None);
    }

    #[test]
    fn rank_orders_by_aim_angle_then_distance() {
        let origin = Vec3::ZERO;
        let aim = Vec3::NEG_Z;
        let on_ray_far = Entity::from_raw_u32(1).unwrap();
        let off_ray_near = Entity::from_raw_u32(2).unwrap();
        let behind = Entity::from_raw_u32(3).unwrap();
        let ranked = rank_combat_targets(
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
        let near = Entity::from_raw_u32(1).unwrap();
        let far = Entity::from_raw_u32(2).unwrap();
        let ranked = rank_combat_targets(
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
    fn maintain_contacts_keeps_the_top_n_and_the_locked_target() {
        let entities: Vec<Entity> = (1..=7)
            .map(|raw| Entity::from_raw_u32(raw).unwrap())
            .collect();
        // Top 5 of 7 by rank.
        let entries = maintain_contacts(&entities, None);
        assert_eq!(entries, entities[..5].to_vec());
        // The combat lock ranked 7th stays a member (replaces the 5th).
        let entries = maintain_contacts(&entities, Some(entities[6]));
        assert_eq!(entries.len(), 5);
        assert!(entries.contains(&entities[6]));
        // An unranked lock (not collectible) is not forced in.
        let stranger = Entity::from_raw_u32(99).unwrap();
        let entries = maintain_contacts(&entities, Some(stranger));
        assert!(!entries.contains(&stranger));
    }

    #[test]
    fn ray_distance_measures_perpendicular_and_clamps_behind() {
        let origin = Vec3::ZERO;
        let dir = Vec3::NEG_Z;
        assert!((ray_distance(origin, dir, Vec3::new(3.0, 4.0, -10.0)) - 5.0).abs() < 1e-6);
        assert!((ray_distance(origin, dir, Vec3::new(0.0, 0.0, 7.0)) - 7.0).abs() < 1e-6);
    }

    #[test]
    fn snap_pick_applies_hysteresis() {
        let a = Entity::from_raw_u32(1).unwrap();
        let b = Entity::from_raw_u32(2).unwrap();
        assert_eq!(snap_pick(None, &[]), None);
        assert_eq!(snap_pick(None, &[(a, 5.0), (b, 3.0)]), Some(b));
        assert_eq!(snap_pick(Some(a), &[(a, 5.0), (b, 4.0)]), Some(a));
        assert_eq!(snap_pick(Some(a), &[(a, 5.0), (b, 1.0)]), Some(b));
    }

    // -- the radar search against the scanner range model --

    /// Player + faithful split camera rigs (ACTIVE normal rig on -Z, dormant
    /// turret decoy 90 degrees off - reading the wrong rig fails loudly) with
    /// an OPEN radar search. Returns (world, player).
    fn radar_world() -> (World, Entity) {
        let mut world = World::new();
        world.insert_resource(Time::<()>::default());
        world.init_resource::<TargetingSettings>();
        world.init_resource::<Messages<RadarLockAcquired>>();
        world.spawn((
            SpaceshipCameraInputMarker,
            SpaceshipCameraNormalInputMarker,
            SpaceshipRotationInputActiveMarker,
            PointRotationOutput(Quat::IDENTITY),
        ));
        world.spawn((
            SpaceshipCameraInputMarker,
            SpaceshipCameraTurretInputMarker,
            PointRotationOutput(Quat::from_rotation_y(std::f32::consts::FRAC_PI_2)),
        ));
        let player = world
            .spawn((
                SpaceshipRootMarker,
                PlayerSpaceshipMarker,
                Transform::IDENTITY,
                targeting_state(),
                // An open search still inside the tap window (nothing
                // engaged): these tests exercise the PICKER; the live-write
                // paths have their own rig below.
                RadarState::default(),
            ))
            .id();
        (world, player)
    }

    fn candidate(world: &mut World, player: Entity) -> Option<Entity> {
        world.get::<RadarState>(player).unwrap().candidate
    }

    fn search(world: &mut World) {
        world.run_system_once(update_radar_search).unwrap();
    }

    #[test]
    fn radar_cone_originates_at_the_live_structure_anchor() {
        // A candidate dead ahead of the ANCHOR but 33 degrees off the ROOT
        // ORIGIN bearing: it is picked only if the cone originates at the
        // anchor (18 degree half-angle).
        let (mut world, player) = radar_world();
        world.entity_mut(player).insert((
            Transform::from_translation(Vec3::new(10.0, 0.0, 0.0)),
            ComputedCenterOfMass(Vec3::new(2.0, 0.0, 0.0)),
        ));
        let body = world
            .spawn((
                RigidBody::Dynamic,
                LockSignature(20.0),
                GlobalTransform::from_translation(Vec3::new(12.0, 0.0, -3.0)),
            ))
            .id();

        search(&mut world);
        assert_eq!(
            candidate(&mut world, player),
            Some(body),
            "the cone must originate at the anchor, not the root origin"
        );
    }

    /// Ray-liveness (task 20260713-082324): the radar follows the ACTIVE
    /// rig's live output; the decoy turret rig points at the side body from
    /// the start, so reading it would fail the delivery guard.
    #[test]
    fn radar_follows_the_live_active_ray() {
        let (mut world, player) = radar_world();
        let side = world
            .spawn((
                SpaceshipRootMarker,
                Allegiance::Neutral,
                RigidBody::Dynamic,
                GlobalTransform::from_translation(Vec3::new(-100.0, 0.0, 0.0)),
            ))
            .id();

        search(&mut world);
        assert_eq!(
            candidate(&mut world, player),
            None,
            "delivery guard: the side body is outside the cone while looking -Z"
        );

        let active = world
            .query_filtered::<Entity, With<SpaceshipRotationInputActiveMarker>>()
            .iter(&world)
            .next()
            .expect("an active rig exists");
        world
            .entity_mut(active)
            .insert(PointRotationOutput(Quat::from_rotation_y(
                std::f32::consts::FRAC_PI_2,
            )));
        search(&mut world);
        assert_eq!(
            candidate(&mut world, player),
            Some(side),
            "the radar must follow the live active ray"
        );
    }

    #[test]
    fn small_signatures_only_lock_up_close() {
        // A signed 2u rock (range 60u at the default 30/unit) dead ahead:
        // invisible to the radar at 200u, pickable at 40u.
        let (mut world, player) = radar_world();
        let rock = world
            .spawn((
                RigidBody::Dynamic,
                LockSignature(2.0),
                GlobalTransform::from_translation(Vec3::new(0.0, 0.0, -200.0)),
            ))
            .id();

        search(&mut world);
        assert_eq!(candidate(&mut world, player), None);

        world
            .entity_mut(rock)
            .insert(GlobalTransform::from_translation(Vec3::new(
                0.0, 0.0, -40.0,
            )));
        search(&mut world);
        assert_eq!(candidate(&mut world, player), Some(rock));
    }

    #[test]
    fn unsigned_debris_is_point_blank_only() {
        // A bare dynamic body (battle debris): never pickable at ~8u, only
        // inside the (retuned, ~5u) unsigned point-blank range.
        let (mut world, player) = radar_world();
        let debris = world
            .spawn((
                RigidBody::Dynamic,
                GlobalTransform::from_translation(Vec3::new(0.0, 0.0, -8.0)),
            ))
            .id();

        search(&mut world);
        assert_eq!(
            candidate(&mut world, player),
            None,
            "debris at 8u must be invisible to the radar (range ~5u)"
        );

        world
            .entity_mut(debris)
            .insert(GlobalTransform::from_translation(Vec3::new(0.0, 0.0, -4.0)));
        search(&mut world);
        assert_eq!(candidate(&mut world, player), Some(debris));
    }

    #[test]
    fn static_beacons_lock_but_static_areas_never_do() {
        let (mut world, player) = radar_world();
        let beacon = world
            .spawn((
                RigidBody::Static,
                LockSignature(20.0),
                GlobalTransform::from_translation(Vec3::new(0.0, 0.0, -300.0)),
            ))
            .id();
        search(&mut world);
        assert_eq!(
            candidate(&mut world, player),
            Some(beacon),
            "a signed static body (nav beacon) is radar-pickable"
        );

        let (mut world, player) = radar_world();
        world.spawn((
            RigidBody::Static,
            GlobalTransform::from_translation(Vec3::new(0.0, 0.0, -4.0)),
        ));
        search(&mut world);
        assert_eq!(
            candidate(&mut world, player),
            None,
            "an unsigned static body (trigger area) is never pickable"
        );
    }

    #[test]
    fn ships_and_well_bodies_keep_their_long_range_lock() {
        for components in 0..2 {
            let (mut world, player) = radar_world();
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

            search(&mut world);
            assert_eq!(
                candidate(&mut world, player),
                Some(target),
                "full-range class {components} must be pickable at range"
            );
        }
    }

    #[test]
    fn committed_torpedoes_lock_at_combat_range_not_across_the_map() {
        let (mut world, player) = radar_world();
        let torpedo = world
            .spawn((
                TorpedoProjectileMarker,
                TorpedoTargetChosen,
                RigidBody::Dynamic,
                GlobalTransform::from_translation(Vec3::new(0.0, 0.0, -2000.0)),
            ))
            .id();
        search(&mut world);
        assert_eq!(candidate(&mut world, player), Some(torpedo));

        world
            .entity_mut(torpedo)
            .insert(GlobalTransform::from_translation(Vec3::new(
                0.0, 0.0, -5000.0,
            )));
        world.get_mut::<RadarState>(player).unwrap().candidate = None;
        search(&mut world);
        assert_eq!(
            candidate(&mut world, player),
            None,
            "a torpedo is not visible across the map"
        );
    }

    #[test]
    fn an_authored_signature_never_gates_below_the_debris_floor() {
        let (mut world, player) = radar_world();
        let speck = world
            .spawn((
                RigidBody::Dynamic,
                LockSignature(0.0),
                GlobalTransform::from_translation(Vec3::new(0.0, 0.0, -4.0)),
            ))
            .id();
        search(&mut world);
        assert_eq!(candidate(&mut world, player), Some(speck));
    }

    #[test]
    fn the_candidate_holds_a_little_past_its_gate_but_fresh_picks_do_not() {
        // A signed 2u rock gates at 60u. Fresh pick at 65u: refused.
        let (mut world, player) = radar_world();
        let rock = world
            .spawn((
                RigidBody::Dynamic,
                LockSignature(2.0),
                GlobalTransform::from_translation(Vec3::new(0.0, 0.0, -65.0)),
            ))
            .id();
        search(&mut world);
        assert_eq!(candidate(&mut world, player), None);

        // Held inside the gate, then drifting to 65u (inside 1.15x): holds.
        world.get_mut::<RadarState>(player).unwrap().candidate = Some(rock);
        search(&mut world);
        assert_eq!(candidate(&mut world, player), Some(rock));

        // Truly out (past 1.15x): dropped.
        world
            .entity_mut(rock)
            .insert(GlobalTransform::from_translation(Vec3::new(
                0.0, 0.0, -80.0,
            )));
        search(&mut world);
        assert_eq!(candidate(&mut world, player), None);
    }

    // -- lock upkeep: validity, decay, allegiance flip, threat set --

    /// Player with the state bundle and both locks set; no camera rig (the
    /// upkeep falls back to ship-forward for the threat ranking). Returns
    /// (world, player, travel_target, combat_target).
    fn locked_world() -> (World, Entity, Entity, Entity) {
        let mut world = World::new();
        world.insert_resource(Time::<()>::default());
        world.init_resource::<TargetingSettings>();
        let travel_target = world
            .spawn((
                RigidBody::Static,
                LockSignature(20.0),
                GlobalTransform::from_translation(Vec3::new(0.0, 0.0, -300.0)),
            ))
            .id();
        let combat_target = world
            .spawn((
                SpaceshipRootMarker,
                AISpaceshipMarker,
                RigidBody::Dynamic,
                GlobalTransform::from_translation(Vec3::new(0.0, 0.0, -400.0)),
            ))
            .id();
        let player = world
            .spawn((
                SpaceshipRootMarker,
                PlayerSpaceshipMarker,
                Transform::IDENTITY,
                targeting_state(),
            ))
            .id();
        // Register the upkeep ONCE so change detection (Changed<Allegiance>)
        // is real across runs - run_system_once builds a fresh system each
        // call, which would see EVERYTHING as changed, exactly the
        // false-positive this rig must not have. Settle the spawn-frame
        // Changed ticks before locking, as a live app would.
        let upkeep_id = world.register_system(update_contacts_and_locks);
        world.insert_resource(UpkeepSystem(upkeep_id));
        world.run_system(upkeep_id).unwrap();
        world.get_mut::<TravelLock>(player).unwrap().0 = Some(travel_target);
        world.get_mut::<CombatLock>(player).unwrap().0 = Some(combat_target);
        (world, player, travel_target, combat_target)
    }

    #[derive(Resource)]
    struct UpkeepSystem(bevy::ecs::system::SystemId);

    fn upkeep(world: &mut World) {
        let id = world.resource::<UpkeepSystem>().0;
        world.run_system(id).unwrap();
    }

    #[test]
    fn locks_hold_while_collectible_and_clear_on_death_or_range() {
        let (mut world, player, travel_target, combat_target) = locked_world();

        upkeep(&mut world);
        assert_eq!(
            world.get::<TravelLock>(player).unwrap().0,
            Some(travel_target),
            "delivery guard: the travel lock holds while collectible"
        );
        assert_eq!(
            world.get::<CombatLock>(player).unwrap().0,
            Some(combat_target)
        );

        // The travel target leaves its signature range: cleared; the combat
        // ship (full-range class) survives.
        world
            .entity_mut(travel_target)
            .insert(GlobalTransform::from_translation(Vec3::new(
                0.0, 0.0, -900.0,
            )));
        upkeep(&mut world);
        assert_eq!(world.get::<TravelLock>(player).unwrap().0, None);
        assert_eq!(
            world.get::<CombatLock>(player).unwrap().0,
            Some(combat_target)
        );

        // The combat target dies: cleared.
        world.despawn(combat_target);
        upkeep(&mut world);
        assert_eq!(world.get::<CombatLock>(player).unwrap().0, None);
    }

    /// D4: the combat lock decays after COMBAT_DECAY_SECS idle; the raised
    /// stance resets the clock (the delivery guard - the same span with
    /// activity does NOT decay).
    #[test]
    fn combat_lock_decays_after_idle_and_raised_resets_the_clock() {
        let (mut world, player, _travel, combat_target) = locked_world();

        // 29 idle seconds: still locked.
        world
            .resource_mut::<Time>()
            .advance_by(Duration::from_secs_f32(29.0));
        upkeep(&mut world);
        assert_eq!(
            world.get::<CombatLock>(player).unwrap().0,
            Some(combat_target)
        );

        // Raised at the brink: the clock resets, another 29 s stays locked.
        world.entity_mut(player).insert(WeaponsRaised(true));
        world
            .resource_mut::<Time>()
            .advance_by(Duration::from_secs_f32(2.0));
        upkeep(&mut world);
        world.entity_mut(player).insert(WeaponsRaised(false));
        world
            .resource_mut::<Time>()
            .advance_by(Duration::from_secs_f32(29.0));
        upkeep(&mut world);
        assert_eq!(
            world.get::<CombatLock>(player).unwrap().0,
            Some(combat_target),
            "activity resets the decay clock"
        );

        // Two more idle seconds cross the threshold: cleared.
        world
            .resource_mut::<Time>()
            .advance_by(Duration::from_secs_f32(2.0));
        upkeep(&mut world);
        assert_eq!(
            world.get::<CombatLock>(player).unwrap().0,
            None,
            "an idle combat lock decays at {COMBAT_DECAY_SECS} s"
        );
    }

    /// A hostile combat target FLIPPING non-hostile clears the lock; a
    /// deliberate lock on an always-neutral body is untouched.
    #[test]
    fn allegiance_flip_clears_the_combat_lock_but_deliberate_neutrals_hold() {
        let (mut world, player, _travel, combat_target) = locked_world();
        upkeep(&mut world);
        assert_eq!(
            world.get::<CombatLock>(player).unwrap().0,
            Some(combat_target),
            "delivery guard: locked while hostile"
        );

        // The scripted surrender: Enemy -> Neutral.
        world.entity_mut(combat_target).insert(Allegiance::Neutral);
        upkeep(&mut world);
        assert_eq!(
            world.get::<CombatLock>(player).unwrap().0,
            None,
            "a surrender must not keep the guns hot"
        );

        // A deliberate combat lock on an (unchanged) neutral holds.
        world.get_mut::<CombatLock>(player).unwrap().0 = Some(combat_target);
        upkeep(&mut world);
        assert_eq!(
            world.get::<CombatLock>(player).unwrap().0,
            Some(combat_target),
            "combat mode is combat mode - deliberate neutral locks are legal"
        );
    }

    #[test]
    fn threat_contacts_track_hostile_combat_targets() {
        let (mut world, player, _travel, combat_target) = locked_world();
        // A neutral ship and a hostile committed torpedo alongside.
        world.spawn((
            SpaceshipRootMarker,
            Allegiance::Neutral,
            RigidBody::Dynamic,
            GlobalTransform::from_translation(Vec3::new(50.0, 0.0, -100.0)),
        ));
        let torpedo = world
            .spawn((
                TorpedoProjectileMarker,
                TorpedoTargetChosen,
                Allegiance::Enemy,
                RigidBody::Dynamic,
                GlobalTransform::from_translation(Vec3::new(0.0, 10.0, -200.0)),
            ))
            .id();

        upkeep(&mut world);
        let entries = world.get::<ThreatContacts>(player).unwrap().entries.clone();
        assert!(entries.contains(&combat_target));
        assert!(
            entries.contains(&torpedo),
            "a committed hostile torpedo is a threat"
        );
        assert_eq!(entries.len(), 2, "neutrals and beacons stay out");
    }

    // -- focus dwell + component fine-lock --

    #[test]
    fn focus_accumulates_and_resets_on_lock_change() {
        let mut world = World::new();
        world.insert_resource(Time::<()>::default());
        let a = world.spawn_empty().id();
        let b = world.spawn_empty().id();
        let ship = world
            .spawn((CombatLock(Some(a)), LockFocus::default()))
            .id();

        world.run_system_once(tick_lock_focus).unwrap();
        world
            .resource_mut::<Time>()
            .advance_by(Duration::from_secs_f32(1.0));
        world.run_system_once(tick_lock_focus).unwrap();
        let focus = world.get::<LockFocus>(ship).unwrap();
        assert_eq!(focus.target, Some(a));
        assert!((focus.seconds - 1.0).abs() < 1e-6);
        assert!(!focus.focused_on(a));

        world
            .resource_mut::<Time>()
            .advance_by(Duration::from_secs_f32(0.6));
        world.run_system_once(tick_lock_focus).unwrap();
        assert!(world.get::<LockFocus>(ship).unwrap().focused_on(a));

        // Switching targets restarts the dwell - a radar re-commit of the
        // SAME entity is equality and never lands here.
        world.get_mut::<CombatLock>(ship).unwrap().0 = Some(b);
        world.run_system_once(tick_lock_focus).unwrap();
        let focus = world.get::<LockFocus>(ship).unwrap();
        assert_eq!(focus.target, Some(b));
        assert_eq!(focus.seconds, 0.0);
    }

    /// A player combat-locked and focused on a target ship with three
    /// sections (one dead on the -Z aim ray, two off to the side), faithful
    /// split rigs. Returns (world, player, [on_ray, near_ray, far_ray]).
    fn focused_world() -> (World, Entity, [Entity; 3]) {
        let mut world = World::new();
        world.insert_resource(Time::<()>::default());
        world.spawn((
            SpaceshipCameraInputMarker,
            SpaceshipCameraNormalInputMarker,
            SpaceshipRotationInputActiveMarker,
            PointRotationOutput(Quat::IDENTITY),
        ));
        world.spawn((
            SpaceshipCameraInputMarker,
            SpaceshipCameraTurretInputMarker,
            PointRotationOutput(Quat::from_rotation_y(std::f32::consts::FRAC_PI_2)),
        ));
        let target = world.spawn(SpaceshipRootMarker).id();
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
        let player = world
            .spawn((
                SpaceshipRootMarker,
                PlayerSpaceshipMarker,
                Transform::IDENTITY,
                CombatLock(Some(target)),
                LockFocus {
                    target: Some(target),
                    seconds: FOCUS_TIME,
                },
                ComponentLock::default(),
            ))
            .id();
        (world, player, [on_ray, near_ray, far_ray])
    }

    fn cycle(world: &mut World, direction: isize) {
        world
            .run_system_once(
                move |time: Res<Time>,
                      q_sections: Query<(Entity, &ChildOf, &Transform), With<SectionMarker>>,
                      mut q_ship: Query<(&CombatLock, &LockFocus, &mut ComponentLock)>| {
                    for (lock, focus, mut component) in &mut q_ship {
                        step_component_lock(
                            direction,
                            &time,
                            lock,
                            focus,
                            &mut component,
                            &q_sections,
                        );
                    }
                },
            )
            .unwrap();
    }

    fn selected(world: &mut World, player: Entity) -> Option<Entity> {
        world.get::<ComponentLock>(player).unwrap().section
    }

    #[test]
    fn snap_selects_the_section_nearest_the_aim_ray() {
        let (mut world, player, [on_ray, ..]) = focused_world();
        world.run_system_once(update_component_lock).unwrap();
        assert_eq!(selected(&mut world, player), Some(on_ray));
    }

    #[test]
    fn component_lock_requires_focus() {
        let (mut world, player, _) = focused_world();
        world.get_mut::<LockFocus>(player).unwrap().seconds = 0.0;
        world.run_system_once(update_component_lock).unwrap();
        assert_eq!(selected(&mut world, player), None);
    }

    #[test]
    fn lock_loss_clears_the_component_lock() {
        let (mut world, player, [on_ray, ..]) = focused_world();
        world.run_system_once(update_component_lock).unwrap();
        assert_eq!(selected(&mut world, player), Some(on_ray));

        world.get_mut::<CombatLock>(player).unwrap().0 = None;
        world.run_system_once(update_component_lock).unwrap();
        assert_eq!(selected(&mut world, player), None);
    }

    #[test]
    fn cycle_steps_the_stable_order_and_pins() {
        let (mut world, player, [on_ray, near_ray, far_ray]) = focused_world();

        // Local build order by z: near_ray (0), on_ray (1), far_ray (2).
        cycle(&mut world, 1);
        assert_eq!(selected(&mut world, player), Some(near_ray));
        cycle(&mut world, 1);
        assert_eq!(selected(&mut world, player), Some(on_ray));
        cycle(&mut world, 1);
        assert_eq!(selected(&mut world, player), Some(far_ray));
        cycle(&mut world, 1);
        assert_eq!(selected(&mut world, player), Some(near_ray), "wraps");
        assert!(matches!(
            world.get::<ComponentLock>(player).unwrap().mode,
            ComponentLockMode::Pinned { .. }
        ));

        // Pinned: the snap must NOT move the selection off near_ray even
        // though on_ray sits on the aim ray.
        world.run_system_once(update_component_lock).unwrap();
        assert_eq!(selected(&mut world, player), Some(near_ray));
    }

    #[test]
    fn cycle_is_a_no_op_before_the_dwell_completes() {
        let (mut world, player, _) = focused_world();
        world.get_mut::<LockFocus>(player).unwrap().seconds = 0.0;
        cycle(&mut world, 1);
        assert_eq!(selected(&mut world, player), None);
    }

    #[test]
    fn pin_expires_back_to_snap() {
        let (mut world, player, [on_ray, near_ray, _]) = focused_world();
        cycle(&mut world, 1);
        assert_eq!(selected(&mut world, player), Some(near_ray));

        // Past the pin window the snap resumes and picks the on-ray section.
        world
            .resource_mut::<Time>()
            .advance_by(Duration::from_secs_f32(COMPONENT_PIN_WINDOW + 0.1));
        world.run_system_once(update_component_lock).unwrap();
        assert_eq!(selected(&mut world, player), Some(on_ray));
    }

    #[test]
    fn pinned_section_death_reverts_to_snap() {
        let (mut world, player, [on_ray, near_ray, _]) = focused_world();
        cycle(&mut world, 1);
        assert_eq!(selected(&mut world, player), Some(near_ray));

        world.despawn(near_ray);
        world.run_system_once(update_component_lock).unwrap();
        assert_eq!(selected(&mut world, player), Some(on_ray));
        assert!(matches!(
            world.get::<ComponentLock>(player).unwrap().mode,
            ComponentLockMode::Snap
        ));
    }

    // -- weapons safety --

    #[test]
    fn weapons_safety_derives_from_stance_and_lock() {
        let mut world = World::new();
        let target = world.spawn_empty().id();
        let ship = world.spawn((CombatLock(None), WeaponsHot::default())).id();

        world.run_system_once(update_weapons_safety).unwrap();
        assert!(
            !world.get::<WeaponsHot>(ship).unwrap().0,
            "lowered + no lock = safe"
        );

        // Raised alone: hot.
        world.entity_mut(ship).insert(WeaponsRaised(true));
        world.run_system_once(update_weapons_safety).unwrap();
        assert!(world.get::<WeaponsHot>(ship).unwrap().0, "raised = hot");

        // Lowered but combat-locked: still hot.
        world.entity_mut(ship).insert(WeaponsRaised(false));
        world.get_mut::<CombatLock>(ship).unwrap().0 = Some(target);
        world.run_system_once(update_weapons_safety).unwrap();
        assert!(
            world.get::<WeaponsHot>(ship).unwrap().0,
            "a combat lock keeps the guns hot"
        );

        // Neither: safe again.
        world.get_mut::<CombatLock>(ship).unwrap().0 = None;
        world.run_system_once(update_weapons_safety).unwrap();
        assert!(!world.get::<WeaponsHot>(ship).unwrap().0);
    }

    /// The trigger-interrupt: the frame the safety engages, held weapon
    /// inputs are zeroed (a latched bool would otherwise keep firing).
    /// Registered system: `Changed<WeaponsHot>` needs real tick history.
    #[test]
    fn safety_engage_zeroes_held_triggers() {
        let mut world = World::new();
        let ship = world.spawn(WeaponsHot(true)).id();
        let turret = world.spawn((TurretSectionInput(true), ChildOf(ship))).id();
        let torpedo = world.spawn((TorpedoSectionInput(true), ChildOf(ship))).id();
        let enforce = world.register_system(enforce_safety_trigger_interrupt);

        // Delivery guard: while hot, held triggers survive the pass.
        world.run_system(enforce).unwrap();
        assert!(**world.get::<TurretSectionInput>(turret).unwrap());
        assert!(**world.get::<TorpedoSectionInput>(torpedo).unwrap());

        // The safety engages: both inputs zero the same frame.
        world.get_mut::<WeaponsHot>(ship).unwrap().0 = false;
        world.run_system(enforce).unwrap();
        assert!(
            !**world.get::<TurretSectionInput>(turret).unwrap(),
            "a held turret trigger is interrupted when the safety engages"
        );
        assert!(
            !**world.get::<TorpedoSectionInput>(torpedo).unwrap(),
            "a held torpedo trigger is interrupted too"
        );
    }

    // -- the radar gesture end to end --

    /// Count of [`RadarLockAcquired`] cues seen, drained by a test-local
    /// reader system so the once-per-gesture contract is observable across
    /// frames (Messages double-buffer per update).
    #[derive(Resource, Default)]
    struct AcquiredCueCount(usize);

    /// Real input -> real `Hold`/`Tap` conditions -> the REAL radar search:
    /// the full gesture e2e on the production flight rig, with
    /// `TimeUpdateStrategy::ManualDuration` driving the clock the conditions
    /// tick on (50 ms steps: 5 updates = the exact 250 ms threshold) and the
    /// production split camera rig (ACTIVE normal rig, dormant turret decoy
    /// 90 degrees off - reading the wrong rig fails loudly) feeding
    /// `update_radar_search`, which under the live-lock model (spike
    /// 20260713-110039) owns the threshold latch and the slot writes.
    /// Nothing is stuffed: bodies are picked off the real look ray.
    fn gesture_app() -> (App, Entity) {
        use bevy::input::InputPlugin;

        use crate::input::player::{flight_input_rig, FlightInputMarker};

        let mut app = App::new();
        app.add_plugins((MinimalPlugins, InputPlugin, EnhancedInputPlugin));
        app.add_plugins(bevy::state::app::StatesPlugin);
        app.init_state::<crate::PauseStates>();
        app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
            Duration::from_millis(50),
        ));
        app.init_resource::<TargetingSettings>();
        app.init_resource::<AcquiredCueCount>();
        app.add_input_context::<FlightInputMarker>();
        app.add_observer(on_radar_start);
        app.add_observer(on_radar_commit);
        app.add_observer(on_radar_cancel);
        app.add_observer(on_lock_clear_tap);
        app.add_message::<LockClearedToast>();
        app.add_message::<RadarLockAcquired>();
        app.add_message::<RadarDenied>();
        // The live search runs inside the pause-gated input set, exactly as
        // the production plugin wires it.
        app.add_systems(
            Update,
            update_radar_search.in_set(crate::input::SpaceshipInputSystems),
        );
        crate::plugin::configure_pause_gating(&mut app);
        app.add_systems(
            Update,
            |mut cues: MessageReader<RadarLockAcquired>, mut count: ResMut<AcquiredCueCount>| {
                count.0 += cues.read().count();
            },
        );

        // The production split camera rig: the ACTIVE normal rig looks down
        // -Z; the dormant turret decoy points 90 degrees off so a system
        // reading the wrong rig picks the wrong bodies and fails loudly.
        app.world_mut().spawn((
            SpaceshipCameraInputMarker,
            SpaceshipCameraNormalInputMarker,
            SpaceshipRotationInputActiveMarker,
            PointRotationOutput(Quat::IDENTITY),
        ));
        app.world_mut().spawn((
            SpaceshipCameraInputMarker,
            SpaceshipCameraTurretInputMarker,
            PointRotationOutput(Quat::from_rotation_y(std::f32::consts::FRAC_PI_2)),
        ));

        // A player ship whose computer grants Lock (the default loadout).
        let ship = app
            .world_mut()
            .spawn((
                SpaceshipRootMarker,
                PlayerSpaceshipMarker,
                Transform::IDENTITY,
                targeting_state(),
            ))
            .id();
        // No WithheldVerbs: an absent component grants every verb (Lock).
        app.world_mut()
            .spawn((ControllerSectionMarker, ChildOf(ship)));

        // The context registry finalizes in App::finish; run the lifecycle
        // before spawning the rig, like the production app does.
        app.finish();
        app.cleanup();
        app.update();
        app.world_mut().spawn(flight_input_rig());
        app.update();
        (app, ship)
    }

    /// A radar-pickable ship body at `position` (ships are intrinsic-class:
    /// full scanner range).
    fn spawn_ship(app: &mut App, position: Vec3) -> Entity {
        app.world_mut()
            .spawn((
                SpaceshipRootMarker,
                RigidBody::Dynamic,
                GlobalTransform::from_translation(position),
            ))
            .id()
    }

    /// Point the ACTIVE normal rig's look ray (identity = -Z).
    fn aim_look(app: &mut App, rotation: Quat) {
        let rig = app
            .world_mut()
            .query_filtered::<Entity, With<SpaceshipRotationInputActiveMarker>>()
            .iter(app.world())
            .next()
            .expect("active rig");
        app.world_mut()
            .entity_mut(rig)
            .insert(PointRotationOutput(rotation));
    }

    fn press_ctrl(app: &mut App) {
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::ControlLeft);
    }
    fn release_ctrl(app: &mut App) {
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .release(KeyCode::ControlLeft);
    }
    fn travel_of(app: &App, ship: Entity) -> Option<Entity> {
        app.world().get::<TravelLock>(ship).unwrap().0
    }
    fn combat_of(app: &App, ship: Entity) -> Option<Entity> {
        app.world().get::<CombatLock>(ship).unwrap().0
    }

    #[test]
    fn radar_locks_at_the_threshold_and_retargets_while_held() {
        let (mut app, ship) = gesture_app();
        let ahead = spawn_ship(&mut app, Vec3::new(0.0, 0.0, -100.0));
        // A second body 90 degrees left (identity-look rotated +PI/2 about Y
        // maps -Z onto -X).
        let left = spawn_ship(&mut app, Vec3::new(-100.0, 0.0, 0.0));

        // Tap window: the search is open and the picker may already see the
        // body, but NOTHING latches or writes before the hold threshold.
        press_ctrl(&mut app);
        for _ in 0..3 {
            app.update();
        }
        let radar = *app.world().get::<RadarState>(ship).expect("search open");
        assert_eq!(radar.engaged, None, "nothing latches inside the tap window");
        assert_eq!(
            radar.candidate,
            Some(ahead),
            "the picker sees the body dead ahead (guard: the write test below is live)"
        );
        assert_eq!(
            travel_of(&app, ship),
            None,
            "no write inside the tap window"
        );

        // Cross the threshold: the slot latches (lowered = travel) and the
        // lock is written WHILE STILL HELD - no release needed.
        for _ in 0..3 {
            app.update();
        }
        assert_eq!(
            app.world().get::<RadarState>(ship).unwrap().engaged,
            Some(RadarSlot::Travel),
            "the threshold latches the slot"
        );
        assert_eq!(
            travel_of(&app, ship),
            Some(ahead),
            "the lock is live under the sweep"
        );

        // Sweep to the second body: the lock retargets instantly, still held.
        aim_look(&mut app, Quat::from_rotation_y(std::f32::consts::FRAC_PI_2));
        app.update();
        assert_eq!(
            travel_of(&app, ship),
            Some(left),
            "the live lock follows the sweep"
        );

        // Release: the lock sticks, the search closes.
        release_ctrl(&mut app);
        app.update();
        assert_eq!(travel_of(&app, ship), Some(left), "release = it sticks");
        assert!(
            app.world().get::<RadarState>(ship).is_none(),
            "the search closes on release"
        );
        assert_eq!(combat_of(&app, ship), None, "the engaged slot only");
    }

    #[test]
    fn raising_inside_the_tap_window_latches_the_combat_slot() {
        // Q1a (threshold latch): CTRL pressed lowered, RMB raised 100 ms
        // later - by the threshold the stance is combat, so the COMBAT slot
        // engages. Under the retired press-time latch this gesture wrote the
        // TRAVEL lock (the recorded same-frame RMB+CTRL sharp edge); this
        // test fails against that model by construction.
        let (mut app, ship) = gesture_app();
        let enemy = spawn_ship(&mut app, Vec3::new(0.0, 0.0, -100.0));

        press_ctrl(&mut app);
        app.update();
        app.update();
        app.world_mut().entity_mut(ship).insert(WeaponsRaised(true));
        for _ in 0..4 {
            app.update();
        }
        assert_eq!(
            app.world().get::<RadarState>(ship).unwrap().engaged,
            Some(RadarSlot::Combat),
            "the threshold latches the CURRENT stance (Q1a)"
        );
        assert_eq!(combat_of(&app, ship), Some(enemy));
        assert_eq!(
            travel_of(&app, ship),
            None,
            "press-time latching would have routed this to travel"
        );
        release_ctrl(&mut app);
        app.update();
        assert_eq!(combat_of(&app, ship), Some(enemy), "sticks");
    }

    #[test]
    fn an_empty_sweep_keeps_the_last_target() {
        // Q2a (keep-last): once acquired, sweeping over empty space never
        // drops the lock - tap is the only clear.
        let (mut app, ship) = gesture_app();
        let ahead = spawn_ship(&mut app, Vec3::new(0.0, 0.0, -100.0));

        press_ctrl(&mut app);
        for _ in 0..6 {
            app.update();
        }
        assert_eq!(travel_of(&app, ship), Some(ahead), "acquired (guard)");

        // Sweep to empty space (90 degrees right: nothing there).
        aim_look(
            &mut app,
            Quat::from_rotation_y(-std::f32::consts::FRAC_PI_2),
        );
        for _ in 0..3 {
            app.update();
        }
        assert_eq!(
            app.world().get::<RadarState>(ship).unwrap().candidate,
            None,
            "the picker sees empty space (guard: keep-last is exercised)"
        );
        assert_eq!(
            travel_of(&app, ship),
            Some(ahead),
            "keep-last: the empty sweep does not drop the lock"
        );
        release_ctrl(&mut app);
        app.update();
        assert_eq!(travel_of(&app, ship), Some(ahead));
    }

    #[test]
    fn a_hold_that_never_resolves_leaves_the_locks_alone() {
        // D1 rephrased for the live model: a hold over empty space latches a
        // slot but never writes it - pre-existing locks survive.
        let (mut app, ship) = gesture_app();
        let target = spawn_ship(&mut app, Vec3::new(0.0, 0.0, 100.0));
        let enemy = spawn_ship(&mut app, Vec3::new(0.0, 100.0, 0.0));
        app.world_mut()
            .entity_mut(ship)
            .insert((TravelLock(Some(target)), CombatLock(Some(enemy))));

        press_ctrl(&mut app);
        for _ in 0..7 {
            app.update();
        }
        release_ctrl(&mut app);
        app.update();
        assert_eq!(travel_of(&app, ship), Some(target), "travel survives");
        assert_eq!(combat_of(&app, ship), Some(enemy), "combat survives");
    }

    #[test]
    fn quick_taps_clear_staged_and_disengage_goto() {
        let (mut app, ship) = gesture_app();
        let target = spawn_ship(&mut app, Vec3::new(0.0, 0.0, 100.0));
        let enemy = spawn_ship(&mut app, Vec3::new(0.0, 100.0, 0.0));
        app.world_mut()
            .entity_mut(ship)
            .insert((TravelLock(Some(target)), CombatLock(Some(enemy))));

        // A quick tap (2 frames = 100 ms < threshold): staged clear - the
        // combat lock first, travel survives...
        press_ctrl(&mut app);
        app.update();
        release_ctrl(&mut app);
        app.update();
        assert_eq!(
            combat_of(&app, ship),
            None,
            "first tap clears the combat lock"
        );
        assert_eq!(travel_of(&app, ship), Some(target));

        // ...the second tap clears the travel lock and disengages a GOTO.
        app.world_mut()
            .entity_mut(ship)
            .insert(Autopilot::engage(AutopilotAction::Goto { target }));
        press_ctrl(&mut app);
        app.update();
        release_ctrl(&mut app);
        app.update();
        assert_eq!(
            travel_of(&app, ship),
            None,
            "second tap clears the travel lock"
        );
        assert!(
            app.world().get::<Autopilot>(ship).is_none(),
            "clearing the designation disengages the GOTO"
        );
    }

    #[test]
    fn raised_tap_clears_combat_only_and_the_boundary_release_sticks() {
        let (mut app, ship) = gesture_app();
        let target = spawn_ship(&mut app, Vec3::new(0.0, 0.0, 100.0));
        let enemy = spawn_ship(&mut app, Vec3::new(0.0, 100.0, 0.0));
        app.world_mut().entity_mut(ship).insert((
            TravelLock(Some(target)),
            CombatLock(Some(enemy)),
            WeaponsRaised(true),
        ));

        // Raised tap: combat only, never travel.
        press_ctrl(&mut app);
        app.update();
        release_ctrl(&mut app);
        app.update();
        assert_eq!(combat_of(&app, ship), None);
        assert_eq!(
            travel_of(&app, ship),
            Some(target),
            "a raised tap never touches the travel lock"
        );

        // Boundary frame: at EXACTLY the shared threshold (5 x 50 ms) the
        // Hold has fired - the lock is ALREADY live - and the Tap has
        // expired, so the release sticks the lock and clears nothing.
        let enemy2 = spawn_ship(&mut app, Vec3::new(0.0, 0.0, -100.0));
        press_ctrl(&mut app);
        for _ in 0..5 {
            app.update();
        }
        assert_eq!(
            combat_of(&app, ship),
            Some(enemy2),
            "the lock is live at the boundary frame"
        );
        release_ctrl(&mut app);
        app.update();
        assert_eq!(
            combat_of(&app, ship),
            Some(enemy2),
            "the boundary release sticks (one shared threshold, no gap)"
        );
        assert_eq!(travel_of(&app, ship), Some(target), "and does not clear");
    }

    #[test]
    fn an_engaged_combat_sweep_holds_the_decay_at_zero() {
        // F12: an engaged combat sweep IS combat activity - a long sweep
        // must not cross the 30 s decay boundary mid-gesture.
        let (mut app, ship) = gesture_app();
        spawn_ship(&mut app, Vec3::new(0.0, 0.0, -100.0));
        app.world_mut().entity_mut(ship).insert(WeaponsRaised(true));

        press_ctrl(&mut app);
        for _ in 0..6 {
            app.update();
        }
        app.world_mut()
            .entity_mut(ship)
            .insert(CombatDecay(COMBAT_DECAY_SECS - 0.01));
        app.update();
        assert_eq!(
            app.world().get::<CombatDecay>(ship).unwrap().0,
            0.0,
            "the engaged combat sweep resets the decay every frame"
        );
    }

    #[test]
    fn the_acquired_cue_fires_once_per_gesture() {
        // Q3a (acquire-only): one cue at the first write, silence across the
        // live retargets; a new gesture earns a new cue.
        let (mut app, ship) = gesture_app();
        spawn_ship(&mut app, Vec3::new(0.0, 0.0, -100.0));
        let left = spawn_ship(&mut app, Vec3::new(-100.0, 0.0, 0.0));

        press_ctrl(&mut app);
        for _ in 0..6 {
            app.update();
        }
        assert_eq!(app.world().resource::<AcquiredCueCount>().0, 1, "acquired");

        // Retarget: no new cue.
        aim_look(&mut app, Quat::from_rotation_y(std::f32::consts::FRAC_PI_2));
        for _ in 0..3 {
            app.update();
        }
        assert_eq!(
            travel_of(&app, ship),
            Some(left),
            "guard: the retarget happened"
        );
        assert_eq!(
            app.world().resource::<AcquiredCueCount>().0,
            1,
            "retargets are silent (Q3a)"
        );

        // Release, re-hold: the next gesture cues again.
        release_ctrl(&mut app);
        app.update();
        press_ctrl(&mut app);
        for _ in 0..6 {
            app.update();
        }
        assert_eq!(app.world().resource::<AcquiredCueCount>().0, 2);
    }

    #[test]
    fn a_lock_less_computer_cannot_radar() {
        let (mut app, ship) = gesture_app();
        // Withhold the Lock capability.
        let controller = app
            .world_mut()
            .query_filtered::<Entity, With<ControllerSectionMarker>>()
            .iter(app.world())
            .next()
            .unwrap();
        app.world_mut()
            .entity_mut(controller)
            .insert(WithheldVerbs([FlightVerb::Lock].into_iter().collect()));

        press_ctrl(&mut app);
        app.update();
        assert!(
            app.world().get::<RadarState>(ship).is_none(),
            "no Lock capability: the radar does not come on"
        );

        // Delivery guard: granting it back opens the search on the next press.
        release_ctrl(&mut app);
        app.update();
        app.world_mut()
            .entity_mut(controller)
            .insert(WithheldVerbs::default());
        press_ctrl(&mut app);
        app.update();
        assert!(app.world().get::<RadarState>(ship).is_some());
    }

    #[test]
    fn a_pause_freezes_the_live_lock_but_the_search_still_closes() {
        // Live-lock pause semantics: what was acquired BEFORE the pause was
        // a completed acquisition and sticks (there is no pending commit to
        // drop any more); while paused the gated search neither latches nor
        // retargets; a release during the pause still tears the search down.
        let (mut app, ship) = gesture_app();
        let ahead = spawn_ship(&mut app, Vec3::new(0.0, 0.0, -100.0));
        let left = spawn_ship(&mut app, Vec3::new(-100.0, 0.0, 0.0));

        press_ctrl(&mut app);
        for _ in 0..6 {
            app.update();
        }
        assert_eq!(travel_of(&app, ship), Some(ahead), "acquired pre-pause");

        app.world_mut()
            .resource_mut::<NextState<crate::PauseStates>>()
            .set(crate::PauseStates::Paused);
        app.update();

        // A sweep during the pause retargets nothing (delivery guard: the
        // same sweep DID retarget in the live test above).
        aim_look(&mut app, Quat::from_rotation_y(std::f32::consts::FRAC_PI_2));
        for _ in 0..3 {
            app.update();
        }
        assert_eq!(
            travel_of(&app, ship),
            Some(ahead),
            "the paused radar does not retarget"
        );
        let _ = left;

        release_ctrl(&mut app);
        app.update();
        assert!(
            app.world().get::<RadarState>(ship).is_none(),
            "the search still closes during a pause"
        );
        assert_eq!(
            travel_of(&app, ship),
            Some(ahead),
            "the pre-pause acquisition sticks"
        );
    }
}
