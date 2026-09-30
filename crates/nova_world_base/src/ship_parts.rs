//! The open world's ship parts: one canonical, validated snapshot of the
//! section prototypes that generated world ships may be built from.
//!
//! A snapshot is one effective catalog, base plus the mod packs it merges. Its
//! identity is independent of pack and section order: packs, sources and parts
//! are sorted by id before anything is scored or hashed. A section id defined
//! by several packs must come from a dependency chain, where the most
//! dependent pack wins, as the runtime merge overlays it; two packs that do
//! not depend on each other and define one id are a fault, not a load-order
//! choice.
//!
//! Every usable part belongs to one [`ShipPartFamilyType`] and gets one
//! capability score from its own stats. Scores are normalized log-linearly
//! within a family, so the weakest usable part of a family needs advancement
//! 0 and the strongest needs 1. A valid part that a generator cannot place is
//! excluded before normalization, so it can never define a family's floor.
//! A socketed part that fires or docks down a non-cardinal lane, or through a
//! face that carries one of its own sockets, is not excluded: it is an
//! authoring fault, as a bad stat is, because a hull built around it fires or
//! docks into whatever stands in its lane.
//!
//! PURE. The open world pins one to its generator when it arms, beside the
//! digest of the loaded catalog it came from; the content lint and the
//! `world_ships` debug example build their own. No generator builds ships
//! from it yet. Scoring is provisional until generated ships are reviewed.

use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
};

use nova_gameplay::prelude::Fnv64;
use nova_ship::prelude::{
    cell_grid_fit, CellGridFault, SectionCollider, SectionConfig, SectionKind, TurretJoint,
};

use crate::civilizations::ShipRoleType;

/// The version of the canonical form [`ShipPartSnapshot::content_hash`]
/// reads. Change it with any change to what the hash covers or how it is
/// written, so an old pin cannot match a new form.
const SHIP_PART_HASH_VERSION: u32 = 1;

/// One content pack's sections as a snapshot reads them.
#[derive(Clone, Debug)]
pub struct ShipPartPack {
    /// The pack id, the source every part it defines is credited to.
    pub id: String,
    /// The ids of the packs this one depends on. The caller adds implicit
    /// dependencies, such as every mod's dependency on the base game.
    pub dependencies: Vec<String>,
    /// The section prototypes this pack defines, in authored order.
    pub sections: Vec<SectionConfig>,
}

/// What a usable part does on a generated ship.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ShipPartFamilyType {
    /// Structure, scored by health per volume.
    Hull,
    /// Flight computer, scored by torque.
    Controller,
    /// Drive, scored by thrust.
    Thruster,
    /// Turret, torpedo bay or railgun, scored by damage per second.
    Weapon,
    /// Cargo intake, scored by door aperture.
    CargoIntake,
    /// Docking port, scored by capture distance in engine units.
    Docking,
}

impl ShipPartFamilyType {
    /// Every family, in declaration order.
    pub const ALL: [Self; 6] = [
        Self::Hull,
        Self::Controller,
        Self::Thruster,
        Self::Weapon,
        Self::CargoIntake,
        Self::Docking,
    ];

    /// The family of a section kind.
    fn of(kind: &SectionKind) -> Self {
        match kind {
            SectionKind::Hull(_) => Self::Hull,
            SectionKind::Controller(_) => Self::Controller,
            SectionKind::Thruster(_) => Self::Thruster,
            SectionKind::Turret(_) | SectionKind::Torpedo(_) | SectionKind::Railgun(_) => {
                Self::Weapon
            }
            SectionKind::CargoIntake(_) => Self::CargoIntake,
            SectionKind::Docking(_) => Self::Docking,
            SectionKind::Mining(_) => unreachable!("mining is excluded before family classification"),
        }
    }

    /// Whether a ship of `role` may carry a part of this family. Civilian
    /// and industrial ships carry no weapon, not even an empty mount.
    pub(crate) fn allowed_on(self, role: ShipRoleType) -> bool {
        match self {
            Self::Weapon => matches!(role, ShipRoleType::Scavenger | ShipRoleType::Armored),
            Self::Hull | Self::Controller | Self::Thruster | Self::CargoIntake | Self::Docking => {
                true
            }
        }
    }

    /// Whether every ship of `role` needs a part of this family. Every ship
    /// flies and docks; an industrial ship mines; a fighting ship fights.
    pub(crate) fn required_by(self, role: ShipRoleType) -> bool {
        match self {
            Self::Hull | Self::Controller | Self::Thruster | Self::Docking => true,
            Self::Weapon => matches!(role, ShipRoleType::Scavenger | ShipRoleType::Armored),
            Self::CargoIntake => role == ShipRoleType::Industrial,
        }
    }
}

impl fmt::Display for ShipPartFamilyType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Hull => "hull",
            Self::Controller => "controller",
            Self::Thruster => "thruster",
            Self::Weapon => "weapon",
            Self::CargoIntake => "cargo intake",
            Self::Docking => "docking port",
        })
    }
}

/// One usable part of a snapshot.
#[derive(Clone, Debug, PartialEq)]
pub struct ShipPart {
    /// The id of the pack whose definition won.
    pub source: String,
    /// What the part does on a ship.
    pub family: ShipPartFamilyType,
    /// The raw score within its family, see [`ShipPartFamilyType`].
    pub capability: f32,
    /// The lowest civilization advancement that may build the part, in
    /// `[0, 1]`: 0 for the weakest part of its family, 1 for the strongest.
    pub advancement: f32,
    /// The effective section prototype.
    pub config: SectionConfig,
}

impl ShipPart {
    /// The section prototype id.
    pub fn id(&self) -> &str {
        &self.config.base.id
    }
}

/// Why a valid section is not a usable part. None of these is an authoring
/// fault: the section still loads and still serves authored ships.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShipPartExclusionType {
    /// The section has no link point, so it cannot join one connected ship.
    MissingLinks,
    /// A railgun with no charge time has no damage per second to score.
    UnchargedRail,
    /// The section cannot stand on the cell grid. Never
    /// [`CellGridFault::ObliqueExit`] or [`CellGridFault::SocketOnExitFace`]:
    /// those two are [`ShipPartFault::UnlanedExit`] instead, because they are
    /// authoring faults, not merely ungenerated geometry.
    OffGrid(CellGridFault),
}

/// Why a set of packs is not a valid snapshot.
#[derive(Clone, Debug, PartialEq)]
pub enum ShipPartFault {
    /// Two packs share one id.
    DuplicatePack {
        /// The repeated pack id.
        pack: String,
    },
    /// A pack depends on a pack the snapshot was not given.
    UnknownDependency {
        /// The dependent pack.
        pack: String,
        /// The missing dependency.
        dependency: String,
    },
    /// One pack defines one section id twice.
    DuplicateInPack {
        /// The pack.
        pack: String,
        /// The repeated section id.
        id: String,
    },
    /// Two packs that do not depend on each other define one section id.
    UnorderedDuplicate {
        /// The section id.
        id: String,
        /// The first pack, by id.
        first: String,
        /// The second pack, by id.
        second: String,
    },
    /// A stat is not a finite number in its valid range.
    InvalidStat {
        /// The pack whose definition is effective.
        pack: String,
        /// The section id.
        id: String,
        /// The stat, as the section content names it.
        stat: &'static str,
        /// The authored value.
        value: f32,
    },
    /// No usable part of a family every minimum-advancement civilian or
    /// industrial ship needs.
    MissingFamily(ShipPartFamilyType),
    /// A weapon or thruster fires down a lane the grid cannot keep clear, or a
    /// weapon, thruster, cargo intake or docking port fires, opens or docks
    /// through a face that carries one of its own sockets.
    UnlanedExit {
        /// The pack whose definition is effective.
        pack: String,
        /// The section id.
        id: String,
        /// The [`CellGridFault`] the exit check refused it for: always
        /// [`CellGridFault::ObliqueExit`] or [`CellGridFault::SocketOnExitFace`].
        fault: CellGridFault,
    },
}

impl fmt::Display for ShipPartFault {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicatePack { pack } => write!(f, "pack '{pack}' is given twice"),
            Self::UnknownDependency { pack, dependency } => {
                write!(
                    f,
                    "pack '{pack}' depends on '{dependency}', which is not loaded"
                )
            }
            Self::DuplicateInPack { pack, id } => {
                write!(f, "pack '{pack}' defines section '{id}' twice")
            }
            Self::UnorderedDuplicate { id, first, second } => write!(
                f,
                "section '{id}' is defined by '{first}' and '{second}', and neither depends on \
                 the other - an overlay needs a dependency chain"
            ),
            Self::InvalidStat {
                pack,
                id,
                stat,
                value,
            } => write!(
                f,
                "section '{id}' ({pack}): {stat} must be a finite number in range for a \
                 generated ship part, got {value}"
            ),
            Self::MissingFamily(family) => write!(
                f,
                "no usable {family} part: a minimum-advancement civilian and industrial ship \
                 needs one"
            ),
            Self::UnlanedExit { pack, id, fault } => write!(f, "section '{id}' ({pack}) {fault}"),
        }
    }
}

/// One validated effective catalog of generated-ship parts.
#[derive(Clone, Debug)]
pub struct ShipPartSnapshot {
    sources: Vec<String>,
    parts: Vec<ShipPart>,
    excluded: BTreeMap<String, ShipPartExclusionType>,
    content_hash: u64,
}

impl ShipPartSnapshot {
    /// Validate `packs` as one effective catalog, in any order, and score its
    /// usable parts. Returns every pack, definition and stat fault, not only
    /// the first; family floors are checked once those are clear.
    pub fn build(packs: &[ShipPartPack]) -> Result<Self, Vec<ShipPartFault>> {
        let mut faults = Vec::new();

        let mut by_id: BTreeMap<&str, &ShipPartPack> = BTreeMap::new();
        for pack in packs {
            if by_id.insert(pack.id.as_str(), pack).is_some() {
                faults.push(ShipPartFault::DuplicatePack {
                    pack: pack.id.clone(),
                });
            }
        }
        for pack in by_id.values() {
            for dependency in &pack.dependencies {
                if !by_id.contains_key(dependency.as_str()) {
                    faults.push(ShipPartFault::UnknownDependency {
                        pack: pack.id.clone(),
                        dependency: dependency.clone(),
                    });
                }
            }
        }
        let ancestors: BTreeMap<&str, BTreeSet<&str>> = by_id
            .keys()
            .map(|id| (*id, ancestors_of(id, &by_id)))
            .collect();
        let depends = |pack: &str, on: &str| ancestors[pack].contains(on);

        // Every definition of every id, by pack in id order.
        let mut definitions: BTreeMap<&str, Vec<(&str, &SectionConfig)>> = BTreeMap::new();
        for pack in by_id.values() {
            let mut seen = BTreeSet::new();
            for section in &pack.sections {
                let id = section.base.id.as_str();
                if !seen.insert(id) {
                    faults.push(ShipPartFault::DuplicateInPack {
                        pack: pack.id.clone(),
                        id: id.to_string(),
                    });
                    continue;
                }
                definitions
                    .entry(id)
                    .or_default()
                    .push((pack.id.as_str(), section));
            }
        }

        // The effective definition of each id: the one pack that depends on
        // every other pack defining it.
        let mut effective: Vec<(&str, &SectionConfig)> = Vec::new();
        for (id, defined) in &definitions {
            let mut ordered = true;
            for (index, (first, _)) in defined.iter().enumerate() {
                for (second, _) in &defined[index + 1..] {
                    if depends(first, second) == depends(second, first) {
                        ordered = false;
                        faults.push(ShipPartFault::UnorderedDuplicate {
                            id: id.to_string(),
                            first: first.to_string(),
                            second: second.to_string(),
                        });
                    }
                }
            }
            if !ordered {
                continue;
            }
            let winner = defined
                .iter()
                .find(|(pack, _)| {
                    defined
                        .iter()
                        .all(|(other, _)| other == pack || depends(pack, other))
                })
                .expect("a pairwise dependency order has a most dependent pack");
            effective.push(*winner);
        }

        let mut scored: Vec<ShipPart> = Vec::new();
        let mut excluded = BTreeMap::new();
        for (source, config) in &effective {
            let before = faults.len();
            check_stats(source, config, &mut faults);
            if faults.len() > before {
                continue;
            }
            let family = ShipPartFamilyType::of(&config.kind);
            match exclusion(config) {
                Ok(Some(excluded_as)) => {
                    excluded.insert(config.base.id.clone(), excluded_as);
                    continue;
                }
                Ok(None) => {}
                Err(fault) => {
                    faults.push(ShipPartFault::UnlanedExit {
                        pack: source.to_string(),
                        id: config.base.id.clone(),
                        fault,
                    });
                    continue;
                }
            }
            let capability = capability(config);
            if !(capability.is_finite() && capability > 0.0) {
                faults.push(ShipPartFault::InvalidStat {
                    pack: source.to_string(),
                    id: config.base.id.clone(),
                    stat: "capability",
                    value: capability,
                });
                continue;
            }
            scored.push(ShipPart {
                source: source.to_string(),
                family,
                capability,
                advancement: 0.0,
                config: (*config).clone(),
            });
        }

        // A refused definition would read as a missing family below; name the
        // cause alone.
        if !faults.is_empty() {
            return Err(faults);
        }
        for family in ShipPartFamilyType::ALL {
            let scores = scored
                .iter()
                .filter(|part| part.family == family)
                .map(|part| part.capability);
            let (weakest, strongest) = scores
                .fold((f32::INFINITY, 0.0_f32), |(low, high), score| {
                    (low.min(score), high.max(score))
                });
            let floor_needed = family.required_by(ShipRoleType::Civilian)
                || family.required_by(ShipRoleType::Industrial);
            if weakest > strongest {
                if floor_needed {
                    faults.push(ShipPartFault::MissingFamily(family));
                }
                continue;
            }
            let span = (strongest / weakest).ln();
            for part in scored.iter_mut().filter(|part| part.family == family) {
                part.advancement = if span > 0.0 {
                    ((part.capability / weakest).ln() / span).clamp(0.0, 1.0)
                } else {
                    0.0
                };
            }
        }

        if !faults.is_empty() {
            return Err(faults);
        }
        Ok(Self {
            sources: by_id.keys().map(|id| id.to_string()).collect(),
            parts: scored,
            excluded,
            content_hash: content_hash(&effective),
        })
    }

    /// Every pack id, sorted. Each is one source a generator weighs equally.
    pub fn sources(&self) -> &[String] {
        &self.sources
    }

    /// Every usable part, sorted by id.
    pub fn parts(&self) -> &[ShipPart] {
        &self.parts
    }

    /// Every valid section that is not a usable part, by id, and why.
    pub fn excluded(&self) -> &BTreeMap<String, ShipPartExclusionType> {
        &self.excluded
    }

    /// The stable identity of the effective catalog: FNV-1a 64 over a
    /// versioned canonical form of every effective section and its source.
    /// Pack and section order do not change it.
    pub fn content_hash(&self) -> u64 {
        self.content_hash
    }

    /// The parts of `family` a ship of `role` may carry at `advancement`,
    /// grouped by source. Each group is one equal-weight source: a generator
    /// draws a source first and a part within it second, so a pack with many
    /// near-identical parts does not crowd out the others.
    pub fn eligible(
        &self,
        role: ShipRoleType,
        family: ShipPartFamilyType,
        advancement: f32,
    ) -> BTreeMap<&str, Vec<&ShipPart>> {
        let mut sources: BTreeMap<&str, Vec<&ShipPart>> = BTreeMap::new();
        if !family.allowed_on(role) {
            return sources;
        }
        for part in &self.parts {
            if part.family == family && part.advancement <= advancement {
                sources.entry(part.source.as_str()).or_default().push(part);
            }
        }
        sources
    }

    /// The roles whose every required family has an eligible part at
    /// `advancement`. Civilian and industrial are always among them: a
    /// snapshot without their floor parts does not build. A fighting role is
    /// absent until a weapon is eligible.
    pub fn eligible_roles(&self, advancement: f32) -> Vec<ShipRoleType> {
        ShipRoleType::ALL
            .into_iter()
            .filter(|role| {
                ShipPartFamilyType::ALL.into_iter().all(|family| {
                    !family.required_by(*role)
                        || !self.eligible(*role, family, advancement).is_empty()
                })
            })
            .collect()
    }
}

/// Every pack `id` depends on, directly or through another pack. A missing
/// dependency is reported by the caller and ends that path.
fn ancestors_of<'a>(id: &str, by_id: &BTreeMap<&'a str, &'a ShipPartPack>) -> BTreeSet<&'a str> {
    let mut found = BTreeSet::new();
    let mut pending: Vec<&str> = vec![id];
    while let Some(next) = pending.pop() {
        let Some(pack) = by_id.get(next) else {
            continue;
        };
        for dependency in &pack.dependencies {
            if let Some((known, _)) = by_id.get_key_value(dependency.as_str()) {
                if found.insert(*known) {
                    pending.push(known);
                }
            }
        }
    }
    found
}

/// Push a fault for every stat a generated part is scored or built from that
/// is not finite and in range.
fn check_stats(source: &str, config: &SectionConfig, faults: &mut Vec<ShipPartFault>) {
    let mut require = |stat: &'static str, value: f32, positive: bool| {
        let valid = value.is_finite() && if positive { value > 0.0 } else { value >= 0.0 };
        if !valid {
            faults.push(ShipPartFault::InvalidStat {
                pack: source.to_string(),
                id: config.base.id.clone(),
                stat,
                value,
            });
        }
    };
    require("health", config.base.health, true);
    match config.base.collider.unwrap_or_default() {
        SectionCollider::Cuboid { size } => {
            require("collider size.x", size.x, true);
            require("collider size.y", size.y, true);
            require("collider size.z", size.z, true);
        }
        SectionCollider::Sphere { radius } => require("collider radius", radius, true),
        SectionCollider::Capsule { radius, length } => {
            require("collider radius", radius, true);
            require("collider length", length, true);
        }
        SectionCollider::Cylinder { radius, height } => {
            require("collider radius", radius, true);
            require("collider height", height, true);
        }
    }
    match &config.kind {
        SectionKind::Thruster(thruster) => require("magnitude", thruster.magnitude, true),
        SectionKind::Controller(controller) => require("max_torque", controller.max_torque, true),
        SectionKind::Turret(turret) => {
            require("bullet_damage", turret.bullet_damage, true);
            for_each_muzzle_rate(&turret.root, &mut |rate| {
                require("muzzle fire_rate", rate, true);
            });
        }
        SectionKind::Torpedo(torpedo) => {
            require("blast_damage", torpedo.blast_damage, true);
            require("fire_rate", torpedo.fire_rate, true);
        }
        SectionKind::Railgun(railgun) => {
            require("slug_damage", railgun.slug_damage, true);
            require("charge_seconds", railgun.charge_seconds, false);
        }
        SectionKind::CargoIntake(intake) => {
            require("aperture_width", intake.aperture_width.get(), true);
            require("aperture_height", intake.aperture_height.get(), true);
        }
        SectionKind::Docking(docking) => {
            require("capture_distance", docking.capture_distance.get(), true);
            require(
                "maximum_relative_speed",
                docking.maximum_relative_speed.get(),
                false,
            );
            require(
                "maximum_relative_angular_speed",
                docking.maximum_relative_angular_speed,
                false,
            );
            // The runtime docking lint's range: a cone at or past 90 degrees
            // accepts a partner facing away.
            if !(0.0..90.0).contains(&docking.capture_angle) {
                faults.push(ShipPartFault::InvalidStat {
                    pack: source.to_string(),
                    id: config.base.id.clone(),
                    stat: "capture_angle",
                    value: docking.capture_angle,
                });
            }
        }
        SectionKind::Hull(_) | SectionKind::Mining(_) => {}
    }
}

/// Why a valid section of a generated-ship family is not usable, if it is
/// not: `Ok(Some(_))` for an exclusion, `Ok(None)` for a usable part, and
/// `Err(_)` for the two cell-grid faults that are authoring errors instead of
/// an exclusion ([`ShipPartFault::UnlanedExit`]).
fn exclusion(config: &SectionConfig) -> Result<Option<ShipPartExclusionType>, CellGridFault> {
    if config.base.link_points.is_empty() {
        return Ok(Some(ShipPartExclusionType::MissingLinks));
    }
    if let SectionKind::Railgun(railgun) = &config.kind {
        if railgun.charge_seconds == 0.0 {
            return Ok(Some(ShipPartExclusionType::UnchargedRail));
        }
    }
    match cell_grid_fit(config) {
        Ok(()) => Ok(None),
        Err(fault @ (CellGridFault::ObliqueExit | CellGridFault::SocketOnExitFace)) => Err(fault),
        Err(fault) => Ok(Some(ShipPartExclusionType::OffGrid(fault))),
    }
}

/// The raw family score of a section whose stats passed [`check_stats`].
fn capability(config: &SectionConfig) -> f32 {
    match &config.kind {
        SectionKind::Hull(_) => {
            config.base.health / collider_volume(config.base.collider.unwrap_or_default())
        }
        SectionKind::Thruster(thruster) => thruster.magnitude,
        SectionKind::Controller(controller) => controller.max_torque,
        SectionKind::Turret(turret) => {
            let mut rate = 0.0;
            for_each_muzzle_rate(&turret.root, &mut |muzzle| rate += muzzle);
            rate * turret.bullet_damage
        }
        SectionKind::Torpedo(torpedo) => torpedo.fire_rate * torpedo.blast_damage,
        SectionKind::Railgun(railgun) => railgun.slug_damage / railgun.charge_seconds,
        SectionKind::CargoIntake(intake) => {
            intake.aperture_width.get() * intake.aperture_height.get()
        }
        SectionKind::Docking(docking) => docking.capture_distance.to_engine(),
        SectionKind::Mining(_) => unreachable!("mining is excluded before capability scoring"),
    }
}

/// Call `visit` with the fire rate of every muzzle in a turret's joint tree.
/// A twin mount fires every barrel.
fn for_each_muzzle_rate(joint: &TurretJoint, visit: &mut impl FnMut(f32)) {
    if let Some(muzzle) = &joint.muzzle {
        visit(muzzle.fire_rate);
    }
    for child in &joint.children {
        for_each_muzzle_rate(child, visit);
    }
}

/// The volume of a collider, in cubic cells.
fn collider_volume(collider: SectionCollider) -> f32 {
    use std::f32::consts::PI;
    match collider {
        SectionCollider::Cuboid { size } => size.x * size.y * size.z,
        SectionCollider::Sphere { radius } => 4.0 / 3.0 * PI * radius.powi(3),
        SectionCollider::Capsule { radius, length } => {
            PI * radius * radius * (4.0 / 3.0 * radius + length)
        }
        SectionCollider::Cylinder { radius, height } => PI * radius * radius * height,
    }
}

/// FNV-1a 64 over the version, then every effective section in id order as
/// its id, its source and its RON form, each closed by a zero byte.
fn content_hash(effective: &[(&str, &SectionConfig)]) -> u64 {
    let mut hash = Fnv64::new().write(&SHIP_PART_HASH_VERSION.to_le_bytes());
    for (source, config) in effective {
        let canonical = ron::to_string(config).expect("a section config serializes to RON");
        for field in [config.base.id.as_str(), source, canonical.as_str()] {
            hash = hash.write(field.as_bytes()).write(&[0]);
        }
    }
    hash.finish()
}

#[cfg(test)]
mod tests {
    use bevy::prelude::*;
    use nova_events::prelude::{Meters, MetersPerSecond};
    use nova_ship::prelude::{
        BaseSectionConfig, CargoIntakeSectionConfig, ControllerSectionConfig, DockingSectionConfig,
        HullSectionConfig, LinkPoint, MuzzleConfig, ThrusterSectionConfig, TorpedoSectionConfig,
        TurretSectionConfig,
    };

    use super::*;

    /// A one-cell section with a socket on each of `faces`.
    fn section(id: &str, kind: SectionKind, faces: &[Vec3]) -> SectionConfig {
        SectionConfig {
            base: BaseSectionConfig {
                id: id.to_string(),
                health: 100.0,
                link_points: faces
                    .iter()
                    .enumerate()
                    .map(|(index, face)| LinkPoint {
                        id: format!("link_{index}"),
                        position: *face * 0.5,
                        normal: *face,
                    })
                    .collect(),
                ..default()
            },
            kind,
        }
    }

    fn hull(id: &str) -> SectionConfig {
        let faces = [
            Vec3::X,
            Vec3::NEG_X,
            Vec3::Y,
            Vec3::NEG_Y,
            Vec3::Z,
            Vec3::NEG_Z,
        ];
        section(id, SectionKind::Hull(HullSectionConfig::default()), &faces)
    }

    fn controller(id: &str) -> SectionConfig {
        let kind = SectionKind::Controller(ControllerSectionConfig::default());
        section(id, kind, &[Vec3::NEG_Z])
    }

    fn thruster(id: &str, magnitude: f32) -> SectionConfig {
        let kind = SectionKind::Thruster(ThrusterSectionConfig {
            magnitude,
            ..default()
        });
        section(id, kind, &[Vec3::NEG_Z])
    }

    fn intake(id: &str) -> SectionConfig {
        let kind = SectionKind::CargoIntake(CargoIntakeSectionConfig {
            render_mesh: "intake.glb#Scene0".into(),
            render_mesh_transform: None,
            canister_mesh: "canister.glb#Scene0".into(),
            door_sound: "door.wav".into(),
            eject_sound: "eject.wav".into(),
            take_sound: "take.wav".into(),
            detection_range: Meters(40.0),
            capture_gap: Meters(1.0),
            aperture_width: Meters(8.0),
            aperture_height: Meters(8.0),
            eject_speed: MetersPerSecond(3.0),
        });
        section(id, kind, &[Vec3::Z])
    }

    /// A port with a socket on every face but its mouth, -Z.
    fn dock(id: &str, docking: DockingSectionConfig) -> SectionConfig {
        let faces = [Vec3::X, Vec3::NEG_X, Vec3::Y, Vec3::NEG_Y, Vec3::Z];
        section(id, SectionKind::Docking(docking), &faces)
    }

    fn pack(id: &str, dependencies: &[&str], sections: Vec<SectionConfig>) -> ShipPartPack {
        ShipPartPack {
            id: id.to_string(),
            dependencies: dependencies.iter().map(|id| id.to_string()).collect(),
            sections,
        }
    }

    /// The minimum flyable civilian and industrial set.
    fn floor() -> Vec<SectionConfig> {
        vec![
            hull("hull"),
            controller("controller"),
            thruster("thruster", 1.0),
            intake("intake"),
            dock("dock", DockingSectionConfig::default()),
        ]
    }

    fn faults(packs: &[ShipPartPack]) -> Vec<ShipPartFault> {
        ShipPartSnapshot::build(packs).expect_err("the packs are refused")
    }

    #[test]
    fn pack_and_section_order_do_not_change_the_snapshot() {
        let mut reversed = floor();
        reversed.reverse();
        let forward = [
            pack("base", &[], floor()),
            pack("drives", &["base"], vec![thruster("big_drive", 9.0)]),
        ];
        let backward = [
            pack("drives", &["base"], vec![thruster("big_drive", 9.0)]),
            pack("base", &[], reversed),
        ];
        let forward = ShipPartSnapshot::build(&forward).expect("the forward packs build");
        let backward = ShipPartSnapshot::build(&backward).expect("the backward packs build");

        assert_eq!(forward.content_hash(), backward.content_hash());
        assert_eq!(forward.parts(), backward.parts());
        assert_eq!(forward.sources(), ["base", "drives"]);
    }

    #[test]
    fn only_a_dependent_pack_may_overlay_a_section() {
        let overlay = || vec![thruster("thruster", 4.0)];

        let unordered = faults(&[
            pack("base", &[], floor()),
            pack("left", &["base"], overlay()),
            pack("right", &["base"], overlay()),
        ]);
        assert_eq!(
            unordered,
            [ShipPartFault::UnorderedDuplicate {
                id: "thruster".to_string(),
                first: "left".to_string(),
                second: "right".to_string(),
            }]
        );

        let chained = ShipPartSnapshot::build(&[
            pack("right", &["left"], overlay()),
            pack("left", &["base"], overlay()),
            pack("base", &[], floor()),
        ])
        .expect("a dependency chain overlays");
        let drive = chained
            .parts()
            .iter()
            .find(|part| part.id() == "thruster")
            .expect("the thruster is usable");
        assert_eq!(drive.source, "right");
    }

    #[test]
    fn a_catalog_without_a_cargo_intake_or_a_docking_port_is_refused() {
        for (missing, family) in [
            ("intake", ShipPartFamilyType::CargoIntake),
            ("dock", ShipPartFamilyType::Docking),
        ] {
            let mut sections = floor();
            sections.retain(|section| section.base.id != missing);

            assert_eq!(
                faults(&[pack("base", &[], sections)]),
                [ShipPartFault::MissingFamily(family)]
            );
        }
    }

    #[test]
    fn a_docking_port_scores_its_capture_distance_and_refuses_a_bad_envelope_or_mouth() {
        let reach = |meters| DockingSectionConfig {
            capture_distance: Meters(meters),
            ..default()
        };
        let mut sections = floor();
        sections.push(dock("long_dock", reach(40.0)));
        let snapshot =
            ShipPartSnapshot::build(&[pack("base", &[], sections)]).expect("the packs build");
        let scores: Vec<(&str, f32, f32)> = snapshot
            .parts()
            .iter()
            .filter(|part| part.family == ShipPartFamilyType::Docking)
            .map(|part| (part.id(), part.capability, part.advancement))
            .collect();
        assert_eq!(scores, [("dock", 1.0, 0.0), ("long_dock", 4.0, 1.0)]);

        let mut sections = floor();
        sections.push(dock("short_dock", reach(0.0)));
        sections.push(dock(
            "wide_dock",
            DockingSectionConfig {
                capture_angle: 90.0,
                ..default()
            },
        ));
        sections.push(dock(
            "restless_dock",
            DockingSectionConfig {
                maximum_relative_speed: MetersPerSecond(f32::INFINITY),
                maximum_relative_angular_speed: -1.0,
                ..default()
            },
        ));
        // A socket on the mouth invites a plate over the hatch.
        sections.push(section(
            "sealed_dock",
            SectionKind::Docking(DockingSectionConfig::default()),
            &[Vec3::Z, Vec3::NEG_Z],
        ));
        let refused: Vec<(String, String)> = faults(&[pack("base", &[], sections)])
            .into_iter()
            .map(|fault| match fault {
                ShipPartFault::InvalidStat { id, stat, .. } => (id, stat.to_string()),
                ShipPartFault::UnlanedExit { id, fault, .. } => (id, format!("{fault:?}")),
                other => panic!("{other:?}"),
            })
            .collect();
        let expected = [
            ("restless_dock", "maximum_relative_speed"),
            ("restless_dock", "maximum_relative_angular_speed"),
            ("sealed_dock", "SocketOnExitFace"),
            ("short_dock", "capture_distance"),
            ("wide_dock", "capture_angle"),
        ]
        .map(|(id, stat)| (id.to_string(), stat.to_string()));
        assert_eq!(refused, expected);
    }

    #[test]
    fn a_non_finite_thrust_is_a_fault() {
        let mut sections = floor();
        sections.push(thruster("broken_drive", f32::NAN));

        let faults = faults(&[pack("base", &[], sections)]);
        assert!(
            matches!(
                faults.as_slice(),
                [ShipPartFault::InvalidStat { id, stat: "magnitude", value, .. }]
                    if id == "broken_drive" && value.is_nan()
            ),
            "{faults:?}"
        );
    }

    #[test]
    fn a_turret_scores_the_fire_rate_of_every_muzzle() {
        let mut turret = TurretSectionConfig {
            bullet_damage: 10.0,
            ..default()
        };
        let muzzle = |id: &str, fire_rate| MuzzleConfig {
            id: id.to_string(),
            fire_rate,
            muzzle_effect: None,
        };
        let mut left = turret.root.clone();
        left.children.clear();
        left.muzzle = Some(muzzle("left", 2.0));
        let mut right = left.clone();
        right.muzzle = Some(muzzle("right", 3.0));
        turret.root.muzzle = None;
        turret.root.children = vec![left, right];
        let mut sections = floor();
        sections.push(section("twin", SectionKind::Turret(turret), &[Vec3::NEG_Y]));

        let snapshot =
            ShipPartSnapshot::build(&[pack("base", &[], sections)]).expect("the packs build");
        let twin = snapshot
            .parts()
            .iter()
            .find(|part| part.id() == "twin")
            .expect("the twin is usable");
        assert_eq!(twin.family, ShipPartFamilyType::Weapon);
        assert_eq!(twin.capability, 50.0);
    }

    #[test]
    fn a_part_that_cannot_fire_down_a_clear_lane_is_a_fault_not_an_exclusion() {
        // Only `Torpedo::spawn_offset` is an authored exit direction (every
        // other kind's exit is a fixed cardinal), so it is the only kind that
        // can carry an oblique exit.
        let oblique_exit = || {
            SectionKind::Torpedo(TorpedoSectionConfig {
                spawn_offset: Vec3::new(1.0, 1.0, 0.0),
                ..default()
            })
        };
        let oblique_bay = section("oblique_bay", oblique_exit(), &[Vec3::NEG_Z]);
        // A turret's exit is the fixed +Y, so a socket on +Y sits on its own
        // muzzle face.
        let blocked_turret = section(
            "blocked_turret",
            SectionKind::Turret(TurretSectionConfig::default()),
            &[Vec3::Y],
        );
        let socketless_bay = SectionConfig {
            base: BaseSectionConfig {
                id: "socketless_bay".to_string(),
                health: 100.0,
                ..default()
            },
            kind: oblique_exit(),
        };
        // Only the effective definition of an overlaid id counts: a mod
        // that breaks a base bay owns the fault, and one that repairs a
        // broken base bay clears it.
        let clear_bay = |id: &str| {
            let kind = SectionKind::Torpedo(TorpedoSectionConfig {
                spawn_offset: Vec3::NEG_Z,
                ..default()
            });
            section(id, kind, &[Vec3::Z])
        };
        let mut base_sections = floor();
        base_sections.push(clear_bay("broken_bay"));
        base_sections.push(section("repaired_bay", oblique_exit(), &[Vec3::NEG_Z]));

        let mod_pack = pack(
            "mod",
            &["base"],
            vec![
                oblique_bay.clone(),
                blocked_turret.clone(),
                socketless_bay.clone(),
                section("broken_bay", oblique_exit(), &[Vec3::NEG_Z]),
                clear_bay("repaired_bay"),
            ],
        );
        assert_eq!(
            faults(&[pack("base", &[], base_sections), mod_pack]),
            [
                ShipPartFault::UnlanedExit {
                    pack: "mod".to_string(),
                    id: "blocked_turret".to_string(),
                    fault: CellGridFault::SocketOnExitFace,
                },
                ShipPartFault::UnlanedExit {
                    pack: "mod".to_string(),
                    id: "broken_bay".to_string(),
                    fault: CellGridFault::ObliqueExit,
                },
                ShipPartFault::UnlanedExit {
                    pack: "mod".to_string(),
                    id: "oblique_bay".to_string(),
                    fault: CellGridFault::ObliqueExit,
                },
            ]
        );

        let mut only_socketless = floor();
        only_socketless.push(socketless_bay);
        let snapshot = ShipPartSnapshot::build(&[pack("base", &[], only_socketless)])
            .expect("a socketless part is excluded, not faulted, so the catalog still builds");
        assert_eq!(
            snapshot.excluded().get("socketless_bay"),
            Some(&ShipPartExclusionType::MissingLinks)
        );
    }
}
