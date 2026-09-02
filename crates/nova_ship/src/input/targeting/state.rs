//! The lock state itself: the two slots ([`TravelLock`], [`CombatLock`]),
//! the radar/decay/contact components that ride the player ship root, the
//! [`TargetingSettings`] range model, and the messages the HUD reads.

use bevy::prelude::*;

use crate::prelude::*;

/// How strongly the lock scanner "sees" a body, a radius-like magnitude in
/// world units (request: think of the lock as a scanner wave - small objects
/// return no signature at range). A candidate without this component and
/// without an intrinsic class (well body, ship, committed torpedo) is only
/// lockable point-blank ([`TargetingSettings::unsigned_lock_range`]); with
/// it, lock range scales as `signature * signature_range_per_unit`. The
/// scenario layer authors it on asteroids from their radius.
#[derive(Component, Clone, Copy, Debug, Deref, DerefMut, Reflect)]
#[reflect(Component)]
pub struct LockSignature(pub f32);

/// Lock-acquisition tunables, reflected for the inspector.
#[derive(Resource, Clone, Debug, Reflect)]
#[reflect(Resource)]
pub struct TargetingSettings {
    /// Lock range per unit of [`LockSignature`]. A pure ratio, so it reads the
    /// same in either scale: at 30, a 20 m field rock is lockable within
    /// 600 m - close enough to matter, far enough not to steal mid-fight
    /// locks.
    pub signature_range_per_unit: f32,
    /// Lock range for bodies with no signature and no intrinsic class -
    /// battle debris, loose fragments. World units, point-blank by design
    /// (retuned 150 m -> 50 m with the deliberate radar).
    pub unsigned_lock_range: f32,
    /// The incumbent lock stays lockable this factor beyond its gate, so
    /// a body at its boundary cannot strobe the lock (and reset the focus
    /// dwell) as the ship drifts. Fresh acquisition still uses the plain
    /// gate.
    pub range_hysteresis: f32,
    /// Lock range for committed torpedoes, world units. Small object, hot
    /// drive: far more visible than its size but not across the map. 2500 u is
    /// 25 km, which covers the AI's whole launch envelope
    /// (`AI_TORPEDO_MAX_RANGE`, 1000 u / 10 km) with margin; a playtest knob.
    pub torpedo_lock_range: f32,
    /// Acquisition dwell: a candidate must stay steady under the ray for this
    /// many seconds AT POINT-BLANK before it hard-commits to its slot; the
    /// radial ring HUD fills over it and sweeping off before it completes
    /// cancels. Distance scales it up (see the other `lock_dwell_*` knobs),
    /// so far locks are a real skill beat.
    pub lock_dwell_base: f32,
    /// Extra dwell at `lock_dwell_reference_range` as a multiple of
    /// `lock_dwell_base`: at 1.5 a lock at the reference distance costs
    /// `base * (1 + 1.5)` = 2.5x the point-blank dwell. Beyond the reference
    /// range the term saturates.
    pub lock_dwell_range_factor: f32,
    /// The distance (world units) at which the distance term reaches full
    /// strength (`lock_dwell_range_factor`); closer targets scale linearly
    /// between point-blank and here. Covers the torpedo engagement band.
    pub lock_dwell_reference_range: f32,
    /// Floor on the computed dwell, seconds - even a point-blank lock is not
    /// instant (the ring must be visible to be cancelable).
    pub lock_dwell_min: f32,
    /// Ceiling on the computed dwell, seconds - a very distant lock never
    /// takes longer than this.
    pub lock_dwell_max: f32,
}

impl Default for TargetingSettings {
    fn default() -> Self {
        Self {
            signature_range_per_unit: 30.0,
            unsigned_lock_range: 5.0,
            range_hysteresis: 1.15,
            torpedo_lock_range: 2500.0,
            lock_dwell_base: 0.6,
            lock_dwell_range_factor: 1.5,
            lock_dwell_reference_range: 2000.0,
            lock_dwell_min: 0.25,
            lock_dwell_max: 2.5,
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
/// the weapons safety stays off. Red crosshair. Sticky, plus the
/// `COMBAT_DECAY_SECS` idle decay.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Default, Reflect)]
#[reflect(Component)]
pub struct CombatLock(pub Option<Entity>);

/// The radar's destination slot, latched at the hold THRESHOLD from the
/// raised stance current at that moment. The old press-time latch carried a
/// same-frame RMB+CTRL edge - the raised flag derives in Update while the
/// radar Start observer runs PreUpdate - which the threshold latch retires:
/// by then the stance has settled.
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
    /// The candidate the acquisition dwell is currently charging on. The
    /// engaged slot is written only once the dwell completes, so this is the
    /// PENDING target, which can differ from the still-committed lock while a
    /// new candidate charges (keep-last). Resets (canceling the dwell)
    /// whenever the candidate changes or drops to `None`.
    pub dwell_target: Option<Entity>,
    /// Seconds the current [`dwell_target`](Self::dwell_target) has been held
    /// steady under the ray. Reaches the per-target dwell
    /// (`lock_dwell_secs`) before the slot commits.
    pub dwell_secs: f32,
    /// The dwell (seconds) the current [`dwell_target`](Self::dwell_target)
    /// needs to commit - the live `lock_dwell_secs` for its distance, cached
    /// each charging frame so the ring HUD renders the fill without
    /// recomputing the distance curve. `0.0` when not dwelling.
    pub dwell_needed: f32,
}

impl RadarState {
    /// Fill fraction of the acquisition dwell against a `needed` duration,
    /// clamped to `[0, 1]` - what the ring HUD renders. A non-positive
    /// `needed` reads as instantly full.
    pub fn dwell_fraction(&self, needed: f32) -> f32 {
        if needed <= 0.0 {
            return 1.0;
        }
        (self.dwell_secs / needed).clamp(0.0, 1.0)
    }

    /// The ring fill against the cached [`dwell_needed`](Self::dwell_needed).
    pub fn dwell_fill(&self) -> f32 {
        self.dwell_fraction(self.dwell_needed)
    }

    /// Whether an acquisition dwell is actively CHARGING (a pending candidate
    /// whose dwell has not yet completed) - i.e. the ring should be shown.
    /// False once the dwell completes (the commit) or when nothing is pending.
    pub fn is_dwelling(&self) -> bool {
        self.dwell_target.is_some()
            && self.dwell_needed > 0.0
            && self.dwell_secs < self.dwell_needed
    }
}

/// The weapons safety, derived every frame on any ship carrying a
/// [`CombatLock`]: HOT (can fire) while the stance is raised OR a combat lock
/// exists; SAFE otherwise. Ships WITHOUT the component are unmanaged and fire
/// freely (bare example turrets); the player gets it via [`targeting_state`],
/// AI ships via their combat mirror (input/ai/acquisition.rs). Enforced LIVE in the
/// section fire systems - a held trigger stops the frame the safety engages -
/// plus a trigger-interrupt (the zeroed inputs need a fresh press once hot
/// again).
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Default, Reflect)]
#[reflect(Component)]
pub struct WeaponsHot(pub bool);

/// Idle bookkeeping for the combat-lock decay: seconds since
/// the last combat activity while a combat lock exists. Reset by the raised
/// stance and by a held weapon trigger; at `COMBAT_DECAY_SECS` the combat
/// lock clears and the safety follows.
#[derive(Component, Debug, Clone, Copy, PartialEq, Default, Reflect)]
#[reflect(Component)]
pub struct CombatDecay(pub f32);

/// The always-on ranked hostile combat set (top `TARGET_CANDIDATE_COUNT`
/// toward the look ray): the edge-indicator threat arrows read it (decision
/// D9 - the on-screen candidate list HUD is retired, the tracker is not).
#[derive(Component, Debug, Clone, PartialEq, Default, Reflect)]
#[reflect(Component)]
pub struct ThreatContacts {
    /// Ranked hostile combat targets, best first.
    pub entries: Vec<Entity>,
}

/// The targeting state bundle a player ship root carries (inserted by the
/// plugin's observer on [`PlayerSpaceshipMarker`](nova_gameplay::prelude::PlayerSpaceshipMarker); AI parity gives AI ships
/// the lock/decay components in).
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

/// One radar gesture acquired its first target - fired the first frame the
/// engaged slot RESOLVES a candidate (re-acquiring the target the slot
/// already held is still an acquisition; the slot write itself is an
/// equality-skip then), once per gesture (acquire-only, Q3a of), never on the
/// live retargets that follow. The LockOn cue reads this (consumer lands
/// with).
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq)]
pub struct RadarLockAcquired {
    /// True when the combat slot acquired (red), false for travel (white).
    pub combat: bool,
}

/// A held radar gesture RE-DESIGNATED to a new target - fired each frame the
/// engaged slot changes to a different candidate AFTER the initial acquire.
/// The subtle retarget tick reads this; the once-per-gesture acquire is
/// [`RadarLockAcquired`] instead, so the two never overlap (acquire on the
/// first resolve, retarget on every change after).
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq)]
pub struct RadarRetargeted {
    /// True when the combat slot retargeted (red), false for travel (white).
    pub combat: bool,
}

/// A tap-clear just cleared a lock. The HUD's unlatch ghost (the crosshair
/// visibly popping off the target - the wordless replacement for the old text
/// toast, Q7a of) and the LockOff cue read this.
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

/// Why the per-frame upkeep let go of a combat lock. The owner's report -
/// "sometimes the ship loses radar focus on locked enemies" - could not be
/// answered by INFERRING the branch from the world state after the fact, so
/// the upkeep names the branch it took. The staged tap-clear is NOT here:
/// that is a deliberate player gesture on its own path
/// ([`LockClearedToast`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Reflect)]
pub enum CombatLockDrop {
    /// The target no longer exists, or stopped being a lockable body at all
    /// (death, despawn, losing its dynamic body).
    TargetGone,
    /// The target is still a lockable body but no longer passes the
    /// candidate gate - in practice its range gate, widened for the incumbent
    /// by [`TargetingSettings::range_hysteresis`]. (The gate's other
    /// rejects - the ship itself, an uncommitted torpedo - cannot hold a lock
    /// in the first place, so they never reach this branch.)
    OutOfRange,
    /// A hostile target turned non-hostile - a scripted surrender must not
    /// keep the guns hot.
    AllegianceFlip,
    /// The idle decay (D4): `COMBAT_DECAY_SECS` without combat activity.
    IdleDecay,
}

/// The combat lock was dropped by the upkeep, with the branch that dropped
/// it named ([`CombatLockDrop`]) and the idle clock as it stood.
#[derive(Message, Debug, Clone, Copy, PartialEq)]
pub struct CombatLockDropped {
    /// The target the lock held.
    pub target: Entity,
    /// Which upkeep branch let go.
    pub reason: CombatLockDrop,
    /// Seconds on the [`CombatDecay`] clock at the drop.
    pub idle_secs: f32,
}
