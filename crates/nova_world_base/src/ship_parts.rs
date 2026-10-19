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
//! Every usable part belongs to one [`ShipPartFamilyType`] and carries the
//! advancement and draw weight its section authors: a civilization builds a
//! part from the part's advancement up, and a generator draws it against the
//! other parts of its family by weight. A civilization knows only a seeded
//! subset of the parts its advancement opens, see [`ShipPartSnapshot::known`].
//! A valid part that a generator cannot place is excluded. A socketed part
//! that fires or docks down a non-cardinal lane, or through a face that
//! carries one of its own sockets, is not excluded: it is an authoring fault,
//! as a bad stat is, because a hull built around it fires or docks into
//! whatever stands in its lane.
//!
//! PURE. The open world pins one to its generator when it arms, beside the
//! digest of the loaded catalog it came from, and lays out every hull a cell
//! owns from it; the content lint and the `world_ships` debug example build
//! their own.

use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
};

use bevy::prelude::{Quat, UVec3, Vec3};
use nova_gameplay::prelude::{CivilizationId, Fnv32, Fnv64, SeedStream, ShipRoleType};
use nova_ship::prelude::{
    cell_grid_fit, oriented_part, CellGridFault, SectionCollider, SectionConfig, SectionKind,
    TurretJoint, GRID_EPSILON,
};

use crate::station_layout::StationArchetypeType;

/// The version of the canonical form [`ShipPartSnapshot::content_hash`]
/// reads. Change it with any change to what the hash covers or how it is
/// written, so an old pin cannot match a new form.
const SHIP_PART_HASH_VERSION: u32 = 1;

/// The chance that a civilization knows one part its advancement opens,
/// drawn once per civilization and part, see [`ShipPartSnapshot::known`].
pub const KNOWN_CHANCE: f32 = 0.5;

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
    /// Structure.
    Hull,
    /// Flight computer.
    Controller,
    /// Drive.
    Thruster,
    /// Turret, torpedo bay or railgun.
    Weapon,
    /// Cargo intake.
    CargoIntake,
    /// Docking port.
    Docking,
    /// Mining beam.
    Mining,
}

impl ShipPartFamilyType {
    /// Every family, in declaration order.
    pub const ALL: [Self; 7] = [
        Self::Hull,
        Self::Controller,
        Self::Thruster,
        Self::Weapon,
        Self::CargoIntake,
        Self::Docking,
        Self::Mining,
    ];

    /// The family of a section kind.
    pub(crate) fn of(kind: &SectionKind) -> Option<Self> {
        Some(match kind {
            SectionKind::Hull(_) => Self::Hull,
            SectionKind::Controller(_) => Self::Controller,
            SectionKind::Thruster(_) => Self::Thruster,
            SectionKind::Turret(_) | SectionKind::Torpedo(_) | SectionKind::Railgun(_) => {
                Self::Weapon
            }
            SectionKind::CargoIntake(_) => Self::CargoIntake,
            SectionKind::Docking(_) => Self::Docking,
            SectionKind::Mining(_) => Self::Mining,
        })
    }

    /// Whether a ship of `role` may carry a part of this family. Civilian
    /// and industrial ships carry no weapon, not even an empty mount, and
    /// only an industrial ship carries a mining beam.
    pub(crate) fn allowed_on(self, role: ShipRoleType) -> bool {
        match self {
            Self::Weapon => matches!(role, ShipRoleType::Scavenger | ShipRoleType::Armored),
            Self::Mining => role == ShipRoleType::Industrial,
            Self::Hull | Self::Controller | Self::Thruster | Self::CargoIntake | Self::Docking => {
                true
            }
        }
    }

    /// Whether every ship of `role` needs a part of this family. Every ship
    /// flies, docks, and takes cargo in and jettisons it through an intake; a
    /// fighting ship fights. A mining beam is an industrial ship's seeded
    /// option, never a requirement.
    pub(crate) fn required_by(self, role: ShipRoleType) -> bool {
        match self {
            Self::Hull | Self::Controller | Self::Thruster | Self::CargoIntake | Self::Docking => {
                true
            }
            Self::Mining => false,
            Self::Weapon => matches!(role, ShipRoleType::Scavenger | ShipRoleType::Armored),
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
            Self::Mining => "mining beam",
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
    /// The lowest civilization advancement that may build the part, in
    /// `[0, 1]`, as its section authors it.
    pub advancement: f32,
    /// Its draw weight against the other known parts of its family, as its
    /// section authors it: finite and positive.
    pub draw_weight: f32,
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
    /// The section cannot stand on the cell grid. Never
    /// [`CellGridFault::ObliqueExit`] or [`CellGridFault::SocketOnExitFace`]:
    /// those two are [`ShipPartFault::UnlanedExit`] instead, because they are
    /// authoring faults, not merely ungenerated geometry.
    OffGrid(CellGridFault),
}

/// Why a set of packs is not a valid snapshot, or why loaded content cannot arm
/// the open world.
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
    /// No usable part of a family every civilian or industrial ship needs
    /// is open at advancement 0, so the least advanced civilization could
    /// not build one. A hull counts only as a structural cube: a ship spine
    /// and a station frame stand on nothing else.
    MissingFamily(ShipPartFamilyType),
    /// A weapon, thruster or mining beam fires down a lane the grid cannot
    /// keep clear, or one of those, a cargo intake or a docking port fires,
    /// opens or docks through a face that carries one of its own sockets.
    UnlanedExit {
        /// The pack whose definition is effective.
        pack: String,
        /// The section id.
        id: String,
        /// The [`CellGridFault`] the exit check refused it for: always
        /// [`CellGridFault::ObliqueExit`] or [`CellGridFault::SocketOnExitFace`].
        fault: CellGridFault,
    },
    /// No loaded style has the id a role's generated ships wear.
    MissingRoleStyle {
        /// The role.
        role: ShipRoleType,
        /// Its [`role_style_id`].
        style: &'static str,
    },
    /// No loaded style has the id an archetype's generated stations wear.
    MissingStationStyle {
        /// The archetype.
        archetype: StationArchetypeType,
        /// Its [`StationArchetypeType::style_id`].
        style: &'static str,
    },
    /// No part of a family every generated station carries is usable at
    /// advancement 0, so the least advanced civilization could not build
    /// one.
    MissingStationPart {
        /// The family: hull, flight computer, docking port or turret.
        family: ShipPartFamilyType,
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
                "no usable {family} part at advancement 0, a structural cube for a hull: a \
                 minimum-advancement civilian and industrial ship needs one"
            ),
            Self::UnlanedExit { pack, id, fault } => write!(f, "section '{id}' ({pack}) {fault}"),
            Self::MissingRoleStyle { role, style } => write!(
                f,
                "{} ships wear style '{style}', and no loaded content authors it",
                role.label()
            ),
            Self::MissingStationStyle { archetype, style } => write!(
                f,
                "{archetype} stations wear style '{style}', and no loaded content authors it"
            ),
            Self::MissingStationPart { family } => write!(
                f,
                "no {family} part a station may carry at advancement 0: every generated \
                 station needs one"
            ),
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
    /// Validate `packs` as one effective catalog, in any order, and collect
    /// its usable parts. Returns every pack, definition and stat fault, not
    /// only the first; family floors are checked once those are clear.
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

        let mut parts: Vec<ShipPart> = Vec::new();
        let mut excluded = BTreeMap::new();
        for (source, config) in &effective {
            let before = faults.len();
            check_stats(source, config, &mut faults);
            if faults.len() > before {
                continue;
            }
            let Some(family) = ShipPartFamilyType::of(&config.kind) else {
                continue;
            };
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
            parts.push(ShipPart {
                source: source.to_string(),
                family,
                advancement: config.base.advancement,
                draw_weight: config.base.draw_weight,
                config: (*config).clone(),
            });
        }

        // A refused definition would read as a missing family below; name the
        // cause alone.
        if !faults.is_empty() {
            return Err(faults);
        }
        for family in ShipPartFamilyType::ALL {
            let floor_needed = family.required_by(ShipRoleType::Civilian)
                || family.required_by(ShipRoleType::Industrial);
            let open = parts.iter().any(|part| {
                part.family == family
                    && part.advancement <= 0.0
                    && (family != ShipPartFamilyType::Hull || is_structural_cube(part))
            });
            if floor_needed && !open {
                faults.push(ShipPartFault::MissingFamily(family));
            }
        }

        if !faults.is_empty() {
            return Err(faults);
        }
        Ok(Self {
            sources: by_id.keys().map(|id| id.to_string()).collect(),
            parts,
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

    /// The parts of `family` that `advancement` opens, in id order: every
    /// part a civilization at that advancement could know.
    pub(crate) fn open(
        &self,
        family: ShipPartFamilyType,
        advancement: f32,
    ) -> impl Iterator<Item = &ShipPart> {
        self.parts
            .iter()
            .filter(move |part| part.family == family && part.advancement <= advancement)
    }

    /// Every part `civilization` builds at `advancement`, in id order: the
    /// parts its advancement opens that it knows.
    ///
    /// Each open part is known with [`KNOWN_CHANCE`], drawn once from the
    /// civilization's identity and the part id, so the same part is known
    /// or not from every cell and at any advancement that opens it. A
    /// family with an open part always has a known one: when no draw knows
    /// one, the open part with the lowest draw is known. A structural cube
    /// hull and a turret are guaranteed the same way before their families
    /// are, so a ship spine and a station frame always have a cube and a
    /// generated station, which carries turrets alone, always has a gun.
    pub fn known(&self, civilization: CivilizationId, advancement: f32) -> Vec<&ShipPart> {
        let mut known = Vec::new();
        for family in ShipPartFamilyType::ALL {
            let draws: Vec<(&ShipPart, f32)> = self
                .open(family, advancement)
                .map(|part| (part, known_draw(civilization, part.id())))
                .collect();
            let mut chosen: Vec<&ShipPart> = draws
                .iter()
                .filter(|(_, draw)| *draw < KNOWN_CHANCE)
                .map(|(part, _)| *part)
                .collect();
            if family == ShipPartFamilyType::Weapon {
                let turrets: Vec<(&ShipPart, f32)> = draws
                    .iter()
                    .filter(|(part, _)| matches!(part.config.kind, SectionKind::Turret(_)))
                    .copied()
                    .collect();
                guarantee_one(&turrets, &mut chosen);
            }
            if family == ShipPartFamilyType::Hull {
                let cubes: Vec<(&ShipPart, f32)> = draws
                    .iter()
                    .filter(|(part, _)| is_structural_cube(part))
                    .copied()
                    .collect();
                guarantee_one(&cubes, &mut chosen);
            }
            guarantee_one(&draws, &mut chosen);
            chosen.sort_by(|a, b| a.id().cmp(b.id()));
            known.extend(chosen);
        }
        known
    }

    /// The parts of `family` a ship of `role` built by `civilization` at
    /// `advancement` may carry, grouped by source. Each group is one
    /// equal-weight source: a generator draws a source first and a part
    /// within it second, so a pack with many near-identical parts does not
    /// crowd out the others.
    pub fn eligible(
        &self,
        civilization: CivilizationId,
        role: ShipRoleType,
        family: ShipPartFamilyType,
        advancement: f32,
    ) -> BTreeMap<&str, Vec<&ShipPart>> {
        let mut sources: BTreeMap<&str, Vec<&ShipPart>> = BTreeMap::new();
        if !family.allowed_on(role) {
            return sources;
        }
        for part in self.known(civilization, advancement) {
            if part.family == family {
                sources.entry(part.source.as_str()).or_default().push(part);
            }
        }
        sources
    }

    /// The parts of `family` a generated station of `civilization` at
    /// `advancement` may carry, grouped by source as
    /// [`eligible`](Self::eligible) groups them. A station never flies,
    /// mines or takes cargo in through an intake, so those families are
    /// empty. Its only weapon is a turret: a turret traverses, so it covers
    /// the station from any face, and a fixed bore on a hull that never
    /// turns to aim cannot, see [`station_carries`].
    pub fn station_parts(
        &self,
        civilization: CivilizationId,
        family: ShipPartFamilyType,
        advancement: f32,
    ) -> BTreeMap<&str, Vec<&ShipPart>> {
        let mut sources: BTreeMap<&str, Vec<&ShipPart>> = BTreeMap::new();
        for part in self.known(civilization, advancement) {
            if part.family == family && station_carries(part) {
                sources.entry(part.source.as_str()).or_default().push(part);
            }
        }
        sources
    }

    /// The roles whose every required family has an eligible part for
    /// `civilization` at `advancement`. Civilian and industrial are always
    /// among them: a snapshot without their floor parts does not build. A
    /// fighting role is absent until a weapon is open.
    pub fn eligible_roles(
        &self,
        civilization: CivilizationId,
        advancement: f32,
    ) -> Vec<ShipRoleType> {
        ShipRoleType::ALL
            .into_iter()
            .filter(|role| {
                ShipPartFamilyType::ALL.into_iter().all(|family| {
                    !family.required_by(*role)
                        || !self
                            .eligible(civilization, *role, family, advancement)
                            .is_empty()
                })
            })
            .collect()
    }
}

/// Keep one part of `pool` known: when `chosen` holds none of them, the one
/// with the lowest draw joins it. An empty pool needs nothing.
fn guarantee_one<'p>(pool: &[(&'p ShipPart, f32)], chosen: &mut Vec<&'p ShipPart>) {
    if pool.is_empty() || pool.iter().any(|(part, _)| chosen.contains(part)) {
        return;
    }
    let (lowest, _) = pool
        .iter()
        .min_by(|(_, a), (_, b)| a.total_cmp(b))
        .expect("a non-empty pool has a lowest draw");
    chosen.push(lowest);
}

/// Whether `part` can stand as a structural cube, the spine of a ship and
/// the block of a station frame: one filled cell with a socket on every face.
pub(crate) fn is_structural_cube(part: &ShipPart) -> bool {
    part.family == ShipPartFamilyType::Hull
        && oriented_part(&part.config, Quat::IDENTITY).is_ok_and(|oriented| {
            oriented.span == UVec3::ONE
                && oriented.origin.abs_diff_eq(Vec3::ZERO, GRID_EPSILON)
                && oriented.cells[0].faces == [true; 6]
        })
}

/// Whether a generated station may carry `part`: a hull, a flight computer,
/// a docking port or a turret.
pub(crate) fn station_carries(part: &ShipPart) -> bool {
    match part.family {
        ShipPartFamilyType::Hull | ShipPartFamilyType::Controller | ShipPartFamilyType::Docking => {
            true
        }
        ShipPartFamilyType::Weapon => matches!(part.config.kind, SectionKind::Turret(_)),
        ShipPartFamilyType::Thruster
        | ShipPartFamilyType::CargoIntake
        | ShipPartFamilyType::Mining => false,
    }
}

/// The unit draw that decides whether `civilization` knows the part `id`:
/// FNV-1a 32 over the civilization's identity and the id, so it is the same
/// from every cell and does not depend on the other parts loaded.
fn known_draw(civilization: CivilizationId, id: &str) -> f32 {
    let mut hash = Fnv32::new().write(&civilization.world_seed.to_le_bytes());
    for axis in civilization.node {
        hash = hash.write(&axis.to_le_bytes());
    }
    SeedStream::new(hash.write(b"known_part").write(id.as_bytes()).finish()).unit()
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

/// Push a fault for every stat a generated part is drawn or built from that
/// is not finite and in range.
fn check_stats(source: &str, config: &SectionConfig, faults: &mut Vec<ShipPartFault>) {
    if !(0.0..=1.0).contains(&config.base.advancement) {
        faults.push(ShipPartFault::InvalidStat {
            pack: source.to_string(),
            id: config.base.id.clone(),
            stat: "advancement",
            value: config.base.advancement,
        });
    }
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
    require("draw_weight", config.base.draw_weight, true);
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
        SectionKind::Mining(mining) => {
            require("reach", mining.reach.get(), true);
            require(
                "pulse_interval_seconds",
                mining.pulse_interval_seconds,
                true,
            );
            require("carve_radius_cells", mining.carve_radius_cells, true);
        }
        SectionKind::Hull(_) => {}
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
    match cell_grid_fit(config) {
        Ok(()) => Ok(None),
        Err(fault @ (CellGridFault::ObliqueExit | CellGridFault::SocketOnExitFace)) => Err(fault),
        Err(fault) => Ok(Some(ShipPartExclusionType::OffGrid(fault))),
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
        HullSectionConfig, LinkPoint, LinkPointSkinType, MuzzleConfig, ThrusterSectionConfig,
        TorpedoSectionConfig, TurretSectionConfig,
    };

    use super::*;

    /// A one-cell section open at advancement 0 with the common draw weight
    /// and a socket on each of `faces`.
    fn section(id: &str, kind: SectionKind, faces: &[Vec3]) -> SectionConfig {
        SectionConfig {
            base: BaseSectionConfig {
                id: id.to_string(),
                health: 100.0,
                advancement: 0.0,
                draw_weight: 1.0,
                link_points: faces
                    .iter()
                    .enumerate()
                    .map(|(index, face)| LinkPoint {
                        id: format!("link_{index}"),
                        position: *face * 0.5,
                        normal: *face,
                        skin: LinkPointSkinType::Clad,
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

    /// A hull open at advancement 0 that is not a structural cube does not
    /// satisfy the hull family: a ship spine and a station frame need a cube.
    #[test]
    fn a_catalog_whose_only_hull_at_advancement_zero_is_not_a_cube_is_refused() {
        let mut sections = floor();
        sections.retain(|section| section.base.id != "hull");
        let mut strut = hull("strut");
        strut.base.link_points.truncate(2);
        sections.push(strut);

        assert_eq!(
            faults(&[pack("base", &[], sections)]),
            [ShipPartFault::MissingFamily(ShipPartFamilyType::Hull)]
        );
    }

    #[test]
    fn a_docking_port_refuses_a_bad_envelope_or_mouth() {
        let reach = |meters| DockingSectionConfig {
            capture_distance: Meters(meters),
            ..default()
        };
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

    /// The stat check walks every muzzle of a turret's joint tree: a twin
    /// with two good muzzles builds, and one bad rate on a nested muzzle is
    /// refused by name.
    #[test]
    fn a_turret_refuses_a_bad_fire_rate_on_any_nested_muzzle() {
        let twin = |right_rate: f32| {
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
            right.muzzle = Some(muzzle("right", right_rate));
            turret.root.muzzle = None;
            turret.root.children = vec![left, right];
            let mut sections = floor();
            sections.push(section("twin", SectionKind::Turret(turret), &[Vec3::NEG_Y]));
            vec![pack("base", &[], sections)]
        };

        let snapshot = ShipPartSnapshot::build(&twin(3.0)).expect("the packs build");
        assert!(snapshot.parts().iter().any(|part| part.id() == "twin"));

        assert_eq!(
            faults(&twin(0.0)),
            [ShipPartFault::InvalidStat {
                pack: "base".to_string(),
                id: "twin".to_string(),
                stat: "muzzle fire_rate",
                value: 0.0,
            }]
        );
    }

    #[test]
    fn a_part_carries_its_authored_tier_and_refuses_one_out_of_range() {
        let tiered = |id: &str, advancement, draw_weight| {
            let mut drive = thruster(id, 2.0);
            drive.base.advancement = advancement;
            drive.base.draw_weight = draw_weight;
            drive
        };
        let mut sections = floor();
        sections.push(tiered("rare_drive", 0.5, 0.1));
        let snapshot =
            ShipPartSnapshot::build(&[pack("base", &[], sections)]).expect("the packs build");
        let rare = snapshot
            .parts()
            .iter()
            .find(|part| part.id() == "rare_drive")
            .expect("the drive is usable");
        assert_eq!((rare.advancement, rare.draw_weight), (0.5, 0.1));

        let mut sections = floor();
        sections.push(tiered("late_drive", 1.5, 1.0));
        sections.push(tiered("nan_drive", f32::NAN, 1.0));
        sections.push(tiered("unweighted_drive", 0.0, 0.0));
        sections.push(tiered("endless_drive", 0.0, f32::INFINITY));
        let refused: Vec<(String, &str)> = faults(&[pack("base", &[], sections)])
            .into_iter()
            .map(|fault| match fault {
                ShipPartFault::InvalidStat { id, stat, .. } => (id, stat),
                other => panic!("{other:?}"),
            })
            .collect();
        let expected = [
            ("endless_drive", "draw_weight"),
            ("late_drive", "advancement"),
            ("nan_drive", "advancement"),
            ("unweighted_drive", "draw_weight"),
        ]
        .map(|(id, stat)| (id.to_string(), stat));
        assert_eq!(refused, expected);

        // The floor is the parts open at advancement 0, not the weakest
        // part: a family whose every part is tiered up is missing.
        let mut sections = floor();
        sections.retain(|section| section.base.id != "thruster");
        sections.push(tiered("late_drive", 0.1, 1.0));
        assert_eq!(
            faults(&[pack("base", &[], sections)]),
            [ShipPartFault::MissingFamily(ShipPartFamilyType::Thruster)]
        );
    }

    /// Across many civilizations, every family with an open part has a known
    /// one, a structural cube hull and a turret are known wherever one is
    /// open, no part is known before its advancement, and the subset is a
    /// real one: some civilization lacks some open part.
    #[test]
    fn every_civilization_knows_an_open_part_of_each_family_a_cube_and_a_turret() {
        let turret = |id: &str| {
            let kind = SectionKind::Turret(TurretSectionConfig {
                bullet_damage: 10.0,
                ..default()
            });
            section(id, kind, &[Vec3::NEG_Y])
        };
        let bay = |id: &str| {
            let kind = SectionKind::Torpedo(TorpedoSectionConfig {
                spawn_offset: Vec3::NEG_Z,
                ..default()
            });
            section(id, kind, &[Vec3::Z])
        };
        let mut sections = floor();
        let mut strut = hull("strut");
        strut.base.link_points.truncate(2);
        sections.extend([
            strut,
            thruster("big_drive", 9.0),
            turret("turret"),
            turret("twin_turret"),
            bay("bay"),
        ]);
        let mut late = thruster("late_drive", 20.0);
        late.base.advancement = 0.5;
        sections.push(late);
        let snapshot =
            ShipPartSnapshot::build(&[pack("base", &[], sections)]).expect("the packs build");

        let mut lacking = 0;
        for world_seed in 0..4 {
            for node in 0..8 {
                let civilization = CivilizationId {
                    world_seed,
                    node: [node, -node, 1],
                };
                let known = snapshot.known(civilization, 0.25);
                let ids: BTreeSet<&str> = known.iter().map(|part| part.id()).collect();
                assert!(
                    !ids.contains("late_drive"),
                    "{civilization:?} knows a part above its advancement"
                );
                for family in ShipPartFamilyType::ALL {
                    let open = snapshot.open(family, 0.25).count();
                    let known = known.iter().filter(|part| part.family == family).count();
                    assert!(
                        open == 0 || known >= 1,
                        "{civilization:?} knows no {family} part"
                    );
                }
                assert!(
                    ids.contains("turret") || ids.contains("twin_turret"),
                    "{civilization:?} knows no turret"
                );
                assert!(ids.contains("hull"), "{civilization:?} knows no cube");
                assert_eq!(
                    snapshot.known(civilization, 0.25).len(),
                    known.len(),
                    "{civilization:?} draws the same subset twice"
                );
                let open: usize = ShipPartFamilyType::ALL
                    .iter()
                    .map(|family| snapshot.open(*family, 0.25).count())
                    .sum();
                lacking += usize::from(ids.len() < open);
            }
        }
        assert!(lacking > 0, "every civilization knows every open part");
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
                advancement: 0.0,
                draw_weight: 1.0,
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
