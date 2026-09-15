//! Typed, curated deltas a prototype REFERENCE applies on top of the section
//! config it resolves to.
//!
//! A patch is not a generic merge of serialized values. Every field here is
//! one an author can reach in the scenario editor, and nothing else is
//! reachable at all: art, colliders, sockets, animations, the turret joint
//! tree, the section kind and the section source are the prototype's, and a
//! patch that could move them would make the prototype meaningless.
//!
//! The rules are the same at every layer that applies one:
//!
//! - an omitted field INHERITS the prototype's value;
//! - `Some(value)` REPLACES it;
//! - for a nullable field, `Some(None)` CLEARS it;
//! - a kind patch that disagrees with the resolved kind is a lint error;
//! - an unknown or duplicated muzzle id is a lint error.
//!
//! Resolution happens once, before preload, lint, preview, balance and spawn,
//! so every one of them reads the same finished section.

use std::collections::{BTreeMap, BTreeSet};

use bevy::prelude::*;
use nova_events::units::prelude::*;
use nova_gameplay::prelude::{DamageType, SectionClass};

use crate::prelude::*;

/// The section patch types, the error a bad one reports, and the muzzle ids a
/// patch and an editor row address a turret's barrels by.
pub mod prelude {
    pub use super::{
        duplicate_muzzle_id, muzzle_ids, ControllerSectionConfigPatch, HullSectionConfigPatch,
        MuzzleConfigPatch, RailgunSectionConfigPatch, SectionConfigPatch, SectionKindPatch,
        SectionPatchError, ThrusterSectionConfigPatch, TorpedoSectionConfigPatch,
        TurretSectionConfigPatch,
    };
}

/// What is wrong with a patch, reported by the content lint and again at
/// resolution so a bad patch can never reach a spawned ship.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SectionPatchError {
    /// The patch names one section kind and the resolved prototype is another.
    /// A patch cannot change what a section IS.
    KindMismatch {
        /// What the prototype resolved to.
        resolved: SectionClass,
        /// What the patch tried to speak to.
        patch: SectionClass,
    },
    /// The patch names a muzzle this turret does not carry.
    UnknownMuzzle(String),
    /// The turret carries two muzzles with the same id, so a patch could not
    /// say which one it meant.
    DuplicateMuzzle(String),
}

impl std::fmt::Display for SectionPatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SectionPatchError::KindMismatch { resolved, patch } => write!(
                f,
                "patch is for a {patch:?} section but the prototype resolves to a {resolved:?}"
            ),
            SectionPatchError::UnknownMuzzle(id) => {
                write!(f, "no muzzle with id '{id}' on this turret")
            }
            SectionPatchError::DuplicateMuzzle(id) => {
                write!(f, "two muzzles share the id '{id}' on this turret")
            }
        }
    }
}

/// The delta a prototype reference applies to the section config it resolves
/// to. The all-inherit value changes nothing and is what an omitted patch
/// means, so absent and empty are never distinguishable.
#[derive(Clone, Debug, Default, PartialEq, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(default))]
pub struct SectionConfigPatch {
    /// This section's starting health (current and max).
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub health: Option<f32>,
    /// The kind-specific delta. Must match the resolved kind.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub kind: Option<SectionKindPatch>,
}

impl SectionConfigPatch {
    /// The all-inherit patch, as a constant: what an `Inline` source's patch
    /// is, without allocating one per call.
    pub const EMPTY: Self = Self {
        health: None,
        kind: None,
    };

    /// Whether this patch changes nothing - the value an omitted patch takes,
    /// and what serde omits from authored content.
    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }

    /// Apply this patch to a resolved section config in place.
    ///
    /// The ONE apply. Every layer that patches a section - a design tuning a
    /// catalog section, a spawn tuning a design's section - calls this, so a
    /// new patchable field lands on all of them at once and the two layers can
    /// never drift.
    pub fn apply(&self, config: &mut SectionConfig) -> Result<(), SectionPatchError> {
        if let Some(health) = self.health {
            config.base.health = health;
        }
        match &self.kind {
            None => Ok(()),
            Some(kind) => kind.apply(&mut config.kind),
        }
    }

    /// The patch that turns `prototype` into `edited`, or `None` when the
    /// difference between them is one no patch can say.
    ///
    /// The inverse of [`apply`](Self::apply), and what the editor writes an
    /// edit back through: a builder types into the RESOLVED section, and this
    /// is what turns the finished value into the delta the reference keeps. A
    /// field that matches the prototype again produces no patch field at all,
    /// which is what makes Reset to inherited a plain write of the inherited
    /// value.
    ///
    /// `None` means the edit left the patchable boundary - a different kind, a
    /// moved joint, a renamed muzzle - and the caller has to keep the whole
    /// config instead of a delta. It is not an error: it is the answer to "can
    /// this still be said as a patch".
    ///
    /// The diff is CHECKED rather than trusted: the patch it builds is applied
    /// to a fresh prototype and compared against `edited`, so a field this
    /// function forgets can never be silently dropped from a saved document.
    pub fn between(prototype: &SectionConfig, edited: &SectionConfig) -> Option<Self> {
        let kind = SectionKindPatch::between(&prototype.kind, &edited.kind);
        let patch = Self {
            health: changed(prototype.base.health, edited.base.health),
            kind: kind.filter(|kind| !kind.is_empty()),
        };
        let mut rebuilt = prototype.clone();
        patch.apply(&mut rebuilt).ok()?;
        (rebuilt == *edited).then_some(patch)
    }
}

/// The kind-specific half of a [`SectionConfigPatch`].
#[derive(Clone, Debug, PartialEq, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum SectionKindPatch {
    /// A hull section: structure only, so there is nothing kind-specific to
    /// patch. The variant exists so a hull patch can still be WRITTEN and
    /// kind-checked like any other.
    Hull(HullSectionConfigPatch),
    /// A thruster section.
    Thruster(ThrusterSectionConfigPatch),
    /// A controller section: attitude hardware.
    Controller(ControllerSectionConfigPatch),
    /// A turret section.
    Turret(TurretSectionConfigPatch),
    /// A torpedo bay.
    Torpedo(TorpedoSectionConfigPatch),
    /// A railgun.
    Railgun(RailgunSectionConfigPatch),
}

impl SectionKindPatch {
    /// Which class this patch speaks to, for the kind check.
    pub fn class(&self) -> SectionClass {
        match self {
            SectionKindPatch::Hull(_) => SectionClass::Hull,
            SectionKindPatch::Thruster(_) => SectionClass::Thruster,
            SectionKindPatch::Controller(_) => SectionClass::Controller,
            SectionKindPatch::Turret(_) => SectionClass::Turret,
            SectionKindPatch::Torpedo(_) => SectionClass::Torpedo,
            SectionKindPatch::Railgun(_) => SectionClass::Railgun,
        }
    }

    /// Whether this kind patch changes nothing. An all-inherit kind patch is
    /// dropped rather than written, so an untouched section keeps a reference
    /// with no patch on it at all.
    fn is_empty(&self) -> bool {
        match self {
            SectionKindPatch::Hull(patch) => unchanged(patch),
            SectionKindPatch::Thruster(patch) => unchanged(patch),
            SectionKindPatch::Controller(patch) => unchanged(patch),
            SectionKindPatch::Turret(patch) => unchanged(patch),
            SectionKindPatch::Torpedo(patch) => unchanged(patch),
            SectionKindPatch::Railgun(patch) => unchanged(patch),
        }
    }

    /// The kind delta between two configs of the SAME kind, or `None` when
    /// they are different kinds - which a patch may never change.
    fn between(prototype: &SectionKind, edited: &SectionKind) -> Option<Self> {
        match (prototype, edited) {
            (SectionKind::Hull(_), SectionKind::Hull(_)) => {
                Some(SectionKindPatch::Hull(HullSectionConfigPatch))
            }
            (SectionKind::Thruster(prototype), SectionKind::Thruster(edited)) => {
                Some(SectionKindPatch::Thruster(ThrusterSectionConfigPatch {
                    magnitude: changed(prototype.magnitude, edited.magnitude),
                }))
            }
            (SectionKind::Controller(prototype), SectionKind::Controller(edited)) => {
                Some(SectionKindPatch::Controller(ControllerSectionConfigPatch {
                    steering_lag: changed(prototype.steering_lag, edited.steering_lag),
                    max_torque: changed(prototype.max_torque, edited.max_torque),
                }))
            }
            (SectionKind::Turret(prototype), SectionKind::Turret(edited)) => Some(
                SectionKindPatch::Turret(TurretSectionConfigPatch::between(prototype, edited)),
            ),
            (SectionKind::Torpedo(prototype), SectionKind::Torpedo(edited)) => Some(
                SectionKindPatch::Torpedo(TorpedoSectionConfigPatch::between(prototype, edited)),
            ),
            (SectionKind::Railgun(prototype), SectionKind::Railgun(edited)) => Some(
                SectionKindPatch::Railgun(RailgunSectionConfigPatch::between(prototype, edited)),
            ),
            _ => None,
        }
    }

    fn apply(&self, kind: &mut SectionKind) -> Result<(), SectionPatchError> {
        match (self, kind) {
            (SectionKindPatch::Hull(_), SectionKind::Hull(_)) => Ok(()),
            (SectionKindPatch::Thruster(patch), SectionKind::Thruster(config)) => {
                if let Some(magnitude) = patch.magnitude {
                    config.magnitude = magnitude;
                }
                Ok(())
            }
            (SectionKindPatch::Controller(patch), SectionKind::Controller(config)) => {
                if let Some(steering_lag) = patch.steering_lag {
                    config.steering_lag = steering_lag;
                }
                if let Some(max_torque) = patch.max_torque {
                    config.max_torque = max_torque;
                }
                Ok(())
            }
            (SectionKindPatch::Turret(patch), SectionKind::Turret(config)) => patch.apply(config),
            (SectionKindPatch::Torpedo(patch), SectionKind::Torpedo(config)) => {
                patch.apply(config);
                Ok(())
            }
            (SectionKindPatch::Railgun(patch), SectionKind::Railgun(config)) => {
                patch.apply(config);
                Ok(())
            }
            (patch, kind) => Err(SectionPatchError::KindMismatch {
                resolved: kind.class(),
                patch: patch.class(),
            }),
        }
    }
}

/// A hull section carries no gameplay knobs of its own; its health is the
/// common [`SectionConfigPatch::health`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct HullSectionConfigPatch;

/// A thruster's tunable output.
#[derive(Clone, Copy, Debug, Default, PartialEq, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(default))]
pub struct ThrusterSectionConfigPatch {
    /// Thrust magnitude.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub magnitude: Option<f32>,
}

/// A controller's attitude tuning. Nothing else: a controller is hardware, and
/// what the ship is PERMITTED to do is [`ShipCapabilities`] on the root.
#[derive(Clone, Copy, Debug, Default, PartialEq, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(default))]
pub struct ControllerSectionConfigPatch {
    /// How long the hull trails a moving steering command, in seconds.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub steering_lag: Option<f32>,
    /// How hard this computer twists the hull.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub max_torque: Option<f32>,
}

/// One muzzle's tunable rate, addressed by [`MuzzleConfig::id`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(default))]
pub struct MuzzleConfigPatch {
    /// Rounds per second for this muzzle.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub fire_rate: Option<f32>,
}

/// A turret's gameplay values. The joint TREE is the prototype's - a patch
/// reaches the muzzles inside it by id and nothing else.
#[derive(Clone, Debug, Default, PartialEq, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(default))]
pub struct TurretSectionConfigPatch {
    /// Muzzle speed.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub muzzle_speed: Option<MetersPerSecond>,
    /// How long a round lives.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub projectile_lifetime: Option<f32>,
    /// Damage per round.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub bullet_damage: Option<f32>,
    /// What kind of damage a round does.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub bullet_kind: Option<DamageType>,
    /// The magazine.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub ammunition: Option<AmmoCapacity>,
    /// The reload cycle.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub reload: Option<ReloadConfig>,
    /// Per-muzzle rates, keyed by muzzle id. An empty map patches none, which
    /// is what an omitted field means.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "BTreeMap::is_empty"))]
    pub muzzles: BTreeMap<String, MuzzleConfigPatch>,
}

impl TurretSectionConfigPatch {
    fn between(prototype: &TurretSectionConfig, edited: &TurretSectionConfig) -> Self {
        // BY ID, never by position in the joint tree: a twin's two barrels
        // are told apart by name, which is the whole reason a muzzle carries
        // one. An id the prototype does not have is left out - a patch cannot
        // add a muzzle, and `between`'s own check turns that into a refusal.
        let mut muzzles = BTreeMap::new();
        for (id, muzzle) in muzzles_of(&edited.root) {
            let Some(before) = find_muzzle(&prototype.root, id) else {
                continue;
            };
            let patch = MuzzleConfigPatch {
                fire_rate: changed(before.fire_rate, muzzle.fire_rate),
            };
            if !unchanged(&patch) {
                muzzles.insert(id.to_string(), patch);
            }
        }
        Self {
            muzzle_speed: changed(prototype.muzzle_speed, edited.muzzle_speed),
            projectile_lifetime: changed(prototype.projectile_lifetime, edited.projectile_lifetime),
            bullet_damage: changed(prototype.bullet_damage, edited.bullet_damage),
            bullet_kind: changed(prototype.bullet_kind, edited.bullet_kind),
            ammunition: changed(prototype.ammunition, edited.ammunition),
            reload: changed(prototype.reload, edited.reload),
            muzzles,
        }
    }

    fn apply(&self, config: &mut TurretSectionConfig) -> Result<(), SectionPatchError> {
        if let Some(muzzle_speed) = self.muzzle_speed {
            config.muzzle_speed = muzzle_speed;
        }
        if let Some(projectile_lifetime) = self.projectile_lifetime {
            config.projectile_lifetime = projectile_lifetime;
        }
        if let Some(bullet_damage) = self.bullet_damage {
            config.bullet_damage = bullet_damage;
        }
        if let Some(bullet_kind) = self.bullet_kind {
            config.bullet_kind = bullet_kind;
        }
        if let Some(ammunition) = self.ammunition {
            config.ammunition = ammunition;
        }
        if let Some(reload) = self.reload {
            config.reload = reload;
        }
        if self.muzzles.is_empty() {
            return Ok(());
        }
        // Duplicates first: a patch aimed at an ambiguous id has no single
        // right answer, so the whole apply fails rather than picking one.
        if let Some(duplicate) = duplicate_muzzle_id(&config.root) {
            return Err(SectionPatchError::DuplicateMuzzle(duplicate));
        }
        for (id, patch) in &self.muzzles {
            let Some(muzzle) = find_muzzle_mut(&mut config.root, id) else {
                return Err(SectionPatchError::UnknownMuzzle(id.clone()));
            };
            if let Some(fire_rate) = patch.fire_rate {
                muzzle.fire_rate = fire_rate;
            }
        }
        Ok(())
    }
}

/// A torpedo bay's gameplay values.
#[derive(Clone, Copy, Debug, Default, PartialEq, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(default))]
pub struct TorpedoSectionConfigPatch {
    /// Launches per second.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub fire_rate: Option<f32>,
    /// The speed a torpedo leaves the tube at.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub spawner_speed: Option<MetersPerSecond>,
    /// How long a torpedo lives.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub projectile_lifetime: Option<f32>,
    /// Seconds before the warhead arms.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub arm_time: Option<f32>,
    /// Distance before the warhead arms.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub arm_distance: Option<Meters>,
    /// The guidance law's navigation constant.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub nav_constant: Option<f32>,
    /// Blast radius.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub blast_radius: Option<Meters>,
    /// Blast damage.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub blast_damage: Option<f32>,
    /// The magazine.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub ammunition: Option<AmmoCapacity>,
    /// The reload cycle.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub reload: Option<ReloadConfig>,
}

impl TorpedoSectionConfigPatch {
    fn between(prototype: &TorpedoSectionConfig, edited: &TorpedoSectionConfig) -> Self {
        Self {
            fire_rate: changed(prototype.fire_rate, edited.fire_rate),
            spawner_speed: changed(prototype.spawner_speed, edited.spawner_speed),
            projectile_lifetime: changed(prototype.projectile_lifetime, edited.projectile_lifetime),
            arm_time: changed(prototype.arm_time, edited.arm_time),
            arm_distance: changed(prototype.arm_distance, edited.arm_distance),
            nav_constant: changed(prototype.nav_constant, edited.nav_constant),
            blast_radius: changed(prototype.blast_radius, edited.blast_radius),
            blast_damage: changed(prototype.blast_damage, edited.blast_damage),
            ammunition: changed(prototype.ammunition, edited.ammunition),
            reload: changed(prototype.reload, edited.reload),
        }
    }

    fn apply(&self, config: &mut TorpedoSectionConfig) {
        if let Some(fire_rate) = self.fire_rate {
            config.fire_rate = fire_rate;
        }
        if let Some(spawner_speed) = self.spawner_speed {
            config.spawner_speed = spawner_speed;
        }
        if let Some(projectile_lifetime) = self.projectile_lifetime {
            config.projectile_lifetime = projectile_lifetime;
        }
        if let Some(arm_time) = self.arm_time {
            config.arm_time = arm_time;
        }
        if let Some(arm_distance) = self.arm_distance {
            config.arm_distance = arm_distance;
        }
        if let Some(nav_constant) = self.nav_constant {
            config.nav_constant = nav_constant;
        }
        if let Some(blast_radius) = self.blast_radius {
            config.blast_radius = blast_radius;
        }
        if let Some(blast_damage) = self.blast_damage {
            config.blast_damage = blast_damage;
        }
        if let Some(ammunition) = self.ammunition {
            config.ammunition = ammunition;
        }
        if let Some(reload) = self.reload {
            config.reload = reload;
        }
    }
}

/// A railgun's gameplay values.
#[derive(Clone, Copy, Debug, Default, PartialEq, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(default))]
pub struct RailgunSectionConfigPatch {
    /// Seconds to charge a shot.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub charge_seconds: Option<f32>,
    /// The speed a slug leaves at.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub slug_speed: Option<MetersPerSecond>,
    /// Damage per slug.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub slug_damage: Option<f32>,
    /// How much structure a slug punches through.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub slug_power: Option<f32>,
    /// The rake radius. `Some(None)` clears it back to a clean bore.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub rake_radius: Option<Option<Meters>>,
    /// How long a slug lives.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub slug_lifetime: Option<f32>,
    /// The impulse the shot puts back into the hull.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub recoil_impulse: Option<f32>,
    /// The magazine.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub ammunition: Option<AmmoCapacity>,
    /// The reload cycle.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub reload: Option<ReloadConfig>,
}

impl RailgunSectionConfigPatch {
    fn between(prototype: &RailgunSectionConfig, edited: &RailgunSectionConfig) -> Self {
        Self {
            charge_seconds: changed(prototype.charge_seconds, edited.charge_seconds),
            slug_speed: changed(prototype.slug_speed, edited.slug_speed),
            slug_damage: changed(prototype.slug_damage, edited.slug_damage),
            slug_power: changed(prototype.slug_power, edited.slug_power),
            rake_radius: changed(prototype.rake_radius, edited.rake_radius),
            slug_lifetime: changed(prototype.slug_lifetime, edited.slug_lifetime),
            recoil_impulse: changed(prototype.recoil_impulse, edited.recoil_impulse),
            ammunition: changed(prototype.ammunition, edited.ammunition),
            reload: changed(prototype.reload, edited.reload),
        }
    }

    fn apply(&self, config: &mut RailgunSectionConfig) {
        if let Some(charge_seconds) = self.charge_seconds {
            config.charge_seconds = charge_seconds;
        }
        if let Some(slug_speed) = self.slug_speed {
            config.slug_speed = slug_speed;
        }
        if let Some(slug_damage) = self.slug_damage {
            config.slug_damage = slug_damage;
        }
        if let Some(slug_power) = self.slug_power {
            config.slug_power = slug_power;
        }
        if let Some(rake_radius) = self.rake_radius {
            config.rake_radius = rake_radius;
        }
        if let Some(slug_lifetime) = self.slug_lifetime {
            config.slug_lifetime = slug_lifetime;
        }
        if let Some(recoil_impulse) = self.recoil_impulse {
            config.recoil_impulse = recoil_impulse;
        }
        if let Some(ammunition) = self.ammunition {
            config.ammunition = ammunition;
        }
        if let Some(reload) = self.reload {
            config.reload = reload;
        }
    }
}

/// The value a patch field takes to turn `prototype` into `edited`: the new
/// value where the two differ, and nothing - inherit - where they do not.
fn changed<T: Copy + PartialEq>(prototype: T, edited: T) -> Option<T> {
    (prototype != edited).then_some(edited)
}

/// Whether a patch piece changes nothing, which is what an omitted one means.
fn unchanged<T: Default + PartialEq>(patch: &T) -> bool {
    *patch == T::default()
}

/// Every muzzle in a turret's joint tree with the id it answers to, in tree
/// order. What a diff walks: the tree shape is the prototype's, and the ids
/// are the only handles a patch is allowed to hold.
fn muzzles_of(root: &TurretJoint) -> Vec<(&str, &MuzzleConfig)> {
    let mut found = Vec::new();
    collect_muzzles(root, &mut found);
    found
}

fn collect_muzzles<'a>(joint: &'a TurretJoint, found: &mut Vec<(&'a str, &'a MuzzleConfig)>) {
    if let Some(muzzle) = &joint.muzzle {
        found.push((muzzle.id.as_str(), muzzle));
    }
    for child in &joint.children {
        collect_muzzles(child, found);
    }
}

/// The muzzle with this id, for reading. The first one where a tree carries
/// the id twice - which `apply` refuses before any of it is written.
fn find_muzzle<'a>(joint: &'a TurretJoint, id: &str) -> Option<&'a MuzzleConfig> {
    muzzles_of(joint)
        .into_iter()
        .find(|(found, _)| *found == id)
        .map(|(_, muzzle)| muzzle)
}

/// Every muzzle id in a turret's joint tree, in tree order. The editor labels
/// its rows from this and the content lint checks it for duplicates.
pub fn muzzle_ids(root: &TurretJoint) -> Vec<&str> {
    let mut ids = Vec::new();
    collect_muzzle_ids(root, &mut ids);
    ids
}

fn collect_muzzle_ids<'a>(joint: &'a TurretJoint, ids: &mut Vec<&'a str>) {
    if let Some(muzzle) = &joint.muzzle {
        ids.push(muzzle.id.as_str());
    }
    for child in &joint.children {
        collect_muzzle_ids(child, ids);
    }
}

/// The first muzzle id this turret carries twice, if any.
pub fn duplicate_muzzle_id(root: &TurretJoint) -> Option<String> {
    let mut seen = BTreeSet::new();
    muzzle_ids(root)
        .into_iter()
        .find(|id| !seen.insert(id.to_string()))
        .map(str::to_string)
}

/// The muzzle with this id, for writing. Walks the tree rather than indexing
/// it: re-parenting a barrel must not move an authored rate onto another gun.
fn find_muzzle_mut<'a>(joint: &'a mut TurretJoint, id: &str) -> Option<&'a mut MuzzleConfig> {
    if joint.muzzle.as_ref().is_some_and(|muzzle| muzzle.id == id) {
        return joint.muzzle.as_mut();
    }
    joint
        .children
        .iter_mut()
        .find_map(|child| find_muzzle_mut(child, id))
}

#[cfg(test)]
mod tests;
