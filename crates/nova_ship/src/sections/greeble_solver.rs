//! The GREEBLE SOLVER: greeble rules, their compile step and a seeded solve
//! over the [`SkinSurface`] graph.
//!
//! The rules are solver configuration. A greeble names a link type on each of
//! its four link points, and a link type lists the link types it may face. Empty
//! space is a greeble with no fixture and takes part in the solve like any
//! other. Rules past the direct neighbours are `near` terms: a factor on the
//! greeble's weight when a listed greeble is placed within a graph radius, where
//! a factor of zero is a hard ban.
//!
//! The solve is a pure function of the graph, the compiled rules and a seed.
//! There is no random stream: every draw hashes the seed, the cell key and the
//! attempt, so a draw for one cell does not depend on how many draws came
//! before it.
//!
//! Contradictions have one static source the compile step refuses (a hard ban
//! on a greeble a link type can force) and one runtime source it cannot see (a
//! `max_count` cap or a forced run that no cell can end on this graph). The
//! solve restarts with a derived hash up to [`GREEBLE_RESTARTS`] times, then
//! solves once without the greebles that carry a forcing link type and logs a
//! warning. A player-built hull is a valid runtime condition, so that last
//! solve is a fallback, not a hidden authoring error.

use std::{
    collections::{BTreeMap, VecDeque},
    fmt,
};

use bevy::prelude::*;
use nova_gameplay::prelude::Fnv64;

use crate::sections::skin_surface::{SkinSurface, SURFACE_SLOTS};

/// The prelude: the rule types, their ids, the compiled rules, the solve and
/// the seed helper.
pub mod prelude {
    pub use super::{
        greeble_seed, solve_greebles, CompiledGreebleRules, GreebleConfig, GreebleDesignId,
        GreebleLinkConfig, GreebleLinkDesignId, GreebleLinkPoints, GreebleNearConfig,
        GreeblePlacement, GreebleRotateType, GreebleRules, GreebleRulesError, GreebleSolution,
        GREEBLE_RESTARTS,
    };
}

/// How many times a contradicted solve starts again with a derived hash before
/// the solve without forcing greebles runs.
pub const GREEBLE_RESTARTS: u32 = 8;

/// The most link types one rule set may declare: the solver holds a set of link
/// types in one `u64`.
const MAX_LINK_TYPES: usize = 64;

/// The id a greeble is named by within one rule set.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(transparent))]
pub struct GreebleDesignId(String);

impl GreebleDesignId {
    /// The id as authored.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for GreebleDesignId {
    fn from(id: &str) -> Self {
        Self(id.to_string())
    }
}

impl fmt::Display for GreebleDesignId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// The id a link type is named by within one rule set.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(transparent))]
pub struct GreebleLinkDesignId(String);

impl GreebleLinkDesignId {
    /// The id as authored.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for GreebleLinkDesignId {
    fn from(id: &str) -> Self {
        Self(id.to_string())
    }
}

impl fmt::Display for GreebleLinkDesignId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// One rule set: the link types and the greebles that use them.
#[derive(Clone, Debug, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct GreebleRules {
    /// Every link type a link point may name.
    pub link_types: Vec<GreebleLinkConfig>,
    /// Every greeble, empty ones included.
    pub greebles: Vec<GreebleConfig>,
}

/// A link type and the link types it may face across an edge. `connects` must
/// be symmetric: if `a` connects `b`, `b` connects `a`.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct GreebleLinkConfig {
    /// The id link points name it by.
    pub id: GreebleLinkDesignId,
    /// The link types it may face.
    pub connects: Vec<GreebleLinkDesignId>,
}

/// Which quarter turns of a greeble the solve may place.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum GreebleRotateType {
    /// All four quarter turns.
    All,
    /// No turn and the half turn.
    Half,
    /// No turn: link point `up` stays on the cell's `up` slot.
    None,
}

/// The link type on each link point, in the cell frame of
/// [`crate::sections::skin_surface`].
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct GreebleLinkPoints {
    /// The link type on the `up` side.
    pub up: GreebleLinkDesignId,
    /// The link type on the `right` side.
    pub right: GreebleLinkDesignId,
    /// The link type on the `down` side.
    pub down: GreebleLinkDesignId,
    /// The link type on the `left` side.
    pub left: GreebleLinkDesignId,
}

impl GreebleLinkPoints {
    /// The four link points in slot order: up, right, down, left.
    fn slots(&self) -> [&GreebleLinkDesignId; SURFACE_SLOTS] {
        [&self.up, &self.right, &self.down, &self.left]
    }
}

/// A range term: when one of `greebles` is placed within `radius` cells of
/// graph distance, this greeble's weight is scaled by `factor`. A factor of
/// zero bans the pair within that radius in both directions.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct GreebleNearConfig {
    /// The greebles the term looks for.
    pub greebles: Vec<GreebleDesignId>,
    /// The graph distance the term reaches, in cells. At least 1.
    pub radius: u32,
    /// The weight factor; 0 is a hard ban.
    pub factor: f32,
}

/// One greeble.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct GreebleConfig {
    /// The id the rule set names it by.
    pub id: GreebleDesignId,
    /// The style fixture this greeble places, or `None` for empty space.
    pub fixture: Option<String>,
    /// The relative weight of the greeble, shared by all its turns. Above 0.
    pub weight: f32,
    /// Which quarter turns may be placed.
    pub rotate: GreebleRotateType,
    /// The link type on each side.
    pub link_points: GreebleLinkPoints,
    /// Range terms. Empty means no range rules.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Vec::is_empty")
    )]
    pub near: Vec<GreebleNearConfig>,
    /// The most cells this greeble may take on one surface. `None` means no
    /// cap.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub max_count: Option<u32>,
}

/// Why a rule set does not compile.
#[derive(Clone, Debug, PartialEq)]
pub enum GreebleRulesError {
    /// Two link types share an id.
    DuplicateLink(GreebleLinkDesignId),
    /// Two greebles share an id.
    DuplicateGreeble(GreebleDesignId),
    /// More link types than [`MAX_LINK_TYPES`].
    TooManyLinks(usize),
    /// A link type's `connects` names an unknown link type.
    UnknownConnect {
        /// The link type whose `connects` names it.
        link: GreebleLinkDesignId,
        /// The unknown id.
        target: GreebleLinkDesignId,
    },
    /// `link` connects `other`, but `other` does not connect `link`.
    AsymmetricConnects {
        /// The link type that names the other.
        link: GreebleLinkDesignId,
        /// The link type that does not name it back.
        other: GreebleLinkDesignId,
    },
    /// A greeble's link point names an unknown link type.
    UnknownLinkPoint {
        /// The greeble.
        greeble: GreebleDesignId,
        /// The unknown id.
        link: GreebleLinkDesignId,
    },
    /// A `near` term names an unknown greeble.
    UnknownNearGreeble {
        /// The greeble that owns the term.
        greeble: GreebleDesignId,
        /// The unknown id.
        target: GreebleDesignId,
    },
    /// A weight is not a finite number above 0.
    BadWeight(GreebleDesignId),
    /// A `near` factor is not a finite number of at least 0.
    BadFactor(GreebleDesignId),
    /// A `near` radius is 0.
    ZeroRadius(GreebleDesignId),
    /// No greeble has `fixture: None`, so no cell can stay empty.
    NoEmptyGreeble,
    /// A factor-0 term involves a greeble a link type can force. A ban on a
    /// forced cell empties it.
    ForcedBan {
        /// The greeble that owns the term.
        greeble: GreebleDesignId,
        /// The greeble the term bans.
        target: GreebleDesignId,
    },
    /// A factor-0 term involves an empty greeble. Banning empty space forces a
    /// fixture into a cell that may hold none.
    EmptyBan {
        /// The greeble that owns the term.
        greeble: GreebleDesignId,
        /// The greeble the term bans.
        target: GreebleDesignId,
    },
    /// An empty greeble has a `max_count`. Capping empty space forces fixtures
    /// into the rest of the surface.
    EmptyCap(GreebleDesignId),
}

impl fmt::Display for GreebleRulesError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateLink(id) => write!(formatter, "link type '{id}' is declared twice"),
            Self::DuplicateGreeble(id) => write!(formatter, "greeble '{id}' is declared twice"),
            Self::TooManyLinks(count) => write!(
                formatter,
                "{count} link types are declared; at most {MAX_LINK_TYPES} are allowed"
            ),
            Self::UnknownConnect { link, target } => write!(
                formatter,
                "link type '{link}' connects unknown link type '{target}'"
            ),
            Self::AsymmetricConnects { link, other } => write!(
                formatter,
                "link type '{link}' connects '{other}', but '{other}' does not connect '{link}'"
            ),
            Self::UnknownLinkPoint { greeble, link } => write!(
                formatter,
                "greeble '{greeble}' names unknown link type '{link}'"
            ),
            Self::UnknownNearGreeble { greeble, target } => write!(
                formatter,
                "greeble '{greeble}' has a near term on unknown greeble '{target}'"
            ),
            Self::BadWeight(id) => write!(
                formatter,
                "greeble '{id}' needs a finite weight above 0"
            ),
            Self::BadFactor(id) => write!(
                formatter,
                "greeble '{id}' has a near factor that is not a finite number of at least 0"
            ),
            Self::ZeroRadius(id) => write!(formatter, "greeble '{id}' has a near radius of 0"),
            Self::NoEmptyGreeble => write!(formatter, "no greeble has fixture None"),
            Self::ForcedBan { greeble, target } => write!(
                formatter,
                "greeble '{greeble}' bans '{target}' with factor 0, but a link type can force one of them"
            ),
            Self::EmptyBan { greeble, target } => write!(
                formatter,
                "greeble '{greeble}' bans '{target}' with factor 0, but one of them has fixture None"
            ),
            Self::EmptyCap(id) => write!(
                formatter,
                "greeble '{id}' has fixture None and a max_count"
            ),
        }
    }
}

impl std::error::Error for GreebleRulesError {}

/// A rule set checked and expanded for the solver. Built once per rule set and
/// reused for every surface.
#[derive(Clone, Debug)]
pub struct CompiledGreebleRules {
    greebles: Vec<CompiledGreeble>,
    variants: Vec<Variant>,
    /// `u64` words in one variant set.
    words: usize,
    /// Per link type, the set of link types it connects.
    connects: Vec<u64>,
    /// Per slot and link type, `slot * link_count + link`: the variants that
    /// show that link type on that slot.
    with_link: Vec<Vec<u64>>,
    link_count: usize,
    /// Every variant.
    all: Vec<u64>,
    /// The variants of greebles that carry no forcing link type.
    unforced: Vec<u64>,
    /// The largest radius of any soft term.
    soft_reach: u32,
}

#[derive(Clone, Debug)]
struct CompiledGreeble {
    id: GreebleDesignId,
    fixture: Option<String>,
    /// The variants of this greeble.
    variants: Vec<u64>,
    soft: Vec<SoftTerm>,
    /// Variant sets to ban within a radius when this greeble is placed.
    bans: Vec<(u32, Vec<u64>)>,
    max_count: Option<u32>,
}

#[derive(Clone, Debug)]
struct SoftTerm {
    radius: u32,
    /// Per greeble, whether the term looks for it.
    targets: Vec<bool>,
    factor: f64,
}

#[derive(Clone, Copy, Debug)]
struct Variant {
    greeble: usize,
    turns: u8,
    /// The link type on each slot after the turn.
    sides: [usize; SURFACE_SLOTS],
    /// The greeble's weight split over its variants.
    weight: f64,
}

impl CompiledGreebleRules {
    /// The number of greebles.
    pub fn greeble_count(&self) -> usize {
        self.greebles.len()
    }

    /// The number of placeable variants after rotation and deduplication.
    pub fn variant_count(&self) -> usize {
        self.variants.len()
    }

    /// The id of greeble `index`, as [`GreeblePlacement::greeble`] names it.
    pub fn greeble_id(&self, index: usize) -> &GreebleDesignId {
        &self.greebles[index].id
    }

    /// The fixture greeble `index` places, or `None` for empty space.
    pub fn fixture(&self, index: usize) -> Option<&str> {
        self.greebles[index].fixture.as_deref()
    }

    fn variant_set(&self, cell: usize) -> std::ops::Range<usize> {
        cell * self.words..(cell + 1) * self.words
    }
}

impl GreebleRules {
    /// Checks the rule set and expands it for the solver.
    ///
    /// Every error is returned, so a lint can report them all at once. A link
    /// type FORCES when its `connects` holds no link type of a `fixture: None`
    /// greeble: a cell beside it cannot stay empty. A greeble that can face a
    /// forcing link type can be forced, and a factor-0 term may involve no such
    /// greeble on either end, because the ban would empty a forced cell.
    ///
    /// Each greeble expands into one variant per allowed quarter turn; turns
    /// that show the same link types on every slot are dropped, and the
    /// greeble's weight is split evenly over the variants left.
    pub fn compile(&self) -> Result<CompiledGreebleRules, Vec<GreebleRulesError>> {
        let mut errors = Vec::new();

        let mut links: BTreeMap<&GreebleLinkDesignId, usize> = BTreeMap::new();
        for (index, link) in self.link_types.iter().enumerate() {
            if links.insert(&link.id, index).is_some() {
                errors.push(GreebleRulesError::DuplicateLink(link.id.clone()));
            }
        }
        let link_count = self.link_types.len();
        if link_count > MAX_LINK_TYPES {
            errors.push(GreebleRulesError::TooManyLinks(link_count));
            return Err(errors);
        }
        let mut connects = vec![0u64; link_count];
        for (index, link) in self.link_types.iter().enumerate() {
            for target in &link.connects {
                match links.get(target) {
                    Some(other) => connects[index] |= 1 << other,
                    None => errors.push(GreebleRulesError::UnknownConnect {
                        link: link.id.clone(),
                        target: target.clone(),
                    }),
                }
            }
        }
        for (index, link) in self.link_types.iter().enumerate() {
            for other in bits(connects[index]) {
                if connects[other] & (1 << index) == 0 {
                    errors.push(GreebleRulesError::AsymmetricConnects {
                        link: link.id.clone(),
                        other: self.link_types[other].id.clone(),
                    });
                }
            }
        }

        let mut greebles: BTreeMap<&GreebleDesignId, usize> = BTreeMap::new();
        for (index, greeble) in self.greebles.iter().enumerate() {
            if greebles.insert(&greeble.id, index).is_some() {
                errors.push(GreebleRulesError::DuplicateGreeble(greeble.id.clone()));
            }
        }
        let mut sides = Vec::with_capacity(self.greebles.len());
        for greeble in &self.greebles {
            let mut resolved = [0; SURFACE_SLOTS];
            for (slot, link) in greeble.link_points.slots().into_iter().enumerate() {
                match links.get(link) {
                    Some(index) => resolved[slot] = *index,
                    None => errors.push(GreebleRulesError::UnknownLinkPoint {
                        greeble: greeble.id.clone(),
                        link: link.clone(),
                    }),
                }
            }
            sides.push(resolved);
            if !(greeble.weight.is_finite() && greeble.weight > 0.0) {
                errors.push(GreebleRulesError::BadWeight(greeble.id.clone()));
            }
            for term in &greeble.near {
                if term.radius == 0 {
                    errors.push(GreebleRulesError::ZeroRadius(greeble.id.clone()));
                }
                if !(term.factor.is_finite() && term.factor >= 0.0) {
                    errors.push(GreebleRulesError::BadFactor(greeble.id.clone()));
                }
                for target in &term.greebles {
                    if !greebles.contains_key(target) {
                        errors.push(GreebleRulesError::UnknownNearGreeble {
                            greeble: greeble.id.clone(),
                            target: target.clone(),
                        });
                    }
                }
            }
        }
        if !errors.is_empty() {
            return Err(errors);
        }

        let empty: Vec<bool> = self
            .greebles
            .iter()
            .map(|greeble| greeble.fixture.is_none())
            .collect();
        if !empty.iter().any(|empty| *empty) {
            return Err(vec![GreebleRulesError::NoEmptyGreeble]);
        }
        let empty_links = sides
            .iter()
            .zip(&empty)
            .filter(|(_, empty)| **empty)
            .flat_map(|(sides, _)| sides.iter())
            .fold(0u64, |set, link| set | 1 << link);
        let forcing = (0..link_count)
            .filter(|link| connects[*link] & empty_links == 0)
            .fold(0u64, |set, link| set | 1 << link);
        let forced_faces = bits(forcing).fold(0u64, |set, link| set | connects[link]);
        let forceable: Vec<bool> = sides
            .iter()
            .map(|sides| sides.iter().any(|link| forced_faces & (1 << link) != 0))
            .collect();
        let carries_forcing: Vec<bool> = sides
            .iter()
            .map(|sides| sides.iter().any(|link| forcing & (1 << link) != 0))
            .collect();

        for (index, greeble) in self.greebles.iter().enumerate() {
            for term in greeble.near.iter().filter(|term| term.factor == 0.0) {
                for target in &term.greebles {
                    let other = greebles[target];
                    let error = if empty[index] || empty[other] {
                        GreebleRulesError::EmptyBan {
                            greeble: greeble.id.clone(),
                            target: target.clone(),
                        }
                    } else if forceable[index] || forceable[other] {
                        GreebleRulesError::ForcedBan {
                            greeble: greeble.id.clone(),
                            target: target.clone(),
                        }
                    } else {
                        continue;
                    };
                    errors.push(error);
                }
            }
            if empty[index] && greeble.max_count.is_some() {
                errors.push(GreebleRulesError::EmptyCap(greeble.id.clone()));
            }
        }
        if !errors.is_empty() {
            return Err(errors);
        }

        let mut variants = Vec::new();
        for (index, greeble) in self.greebles.iter().enumerate() {
            let turns: &[u8] = match greeble.rotate {
                GreebleRotateType::All => &[0, 1, 2, 3],
                GreebleRotateType::Half => &[0, 2],
                GreebleRotateType::None => &[0],
            };
            let mut kept: Vec<(u8, [usize; SURFACE_SLOTS])> = Vec::new();
            for turn in turns {
                let turned: [usize; SURFACE_SLOTS] = std::array::from_fn(|slot| {
                    sides[index][(slot + SURFACE_SLOTS - usize::from(*turn)) % SURFACE_SLOTS]
                });
                if !kept.iter().any(|(_, sides)| *sides == turned) {
                    kept.push((*turn, turned));
                }
            }
            let weight = f64::from(greeble.weight) / kept.len() as f64;
            variants.extend(kept.into_iter().map(|(turns, sides)| Variant {
                greeble: index,
                turns,
                sides,
                weight,
            }));
        }

        let words = variants.len().div_ceil(64).max(1);
        let set_of = |keep: &dyn Fn(&Variant) -> bool| {
            let mut set = vec![0u64; words];
            for (index, variant) in variants.iter().enumerate() {
                if keep(variant) {
                    set[index / 64] |= 1 << (index % 64);
                }
            }
            set
        };
        let with_link = (0..SURFACE_SLOTS * link_count)
            .map(|key| set_of(&|variant| variant.sides[key / link_count] == key % link_count))
            .collect();
        let greeble_sets: Vec<Vec<u64>> = (0..self.greebles.len())
            .map(|index| set_of(&|variant| variant.greeble == index))
            .collect();

        let mut compiled: Vec<CompiledGreeble> = self
            .greebles
            .iter()
            .enumerate()
            .map(|(index, greeble)| CompiledGreeble {
                id: greeble.id.clone(),
                fixture: greeble.fixture.clone(),
                variants: greeble_sets[index].clone(),
                soft: Vec::new(),
                bans: Vec::new(),
                max_count: greeble.max_count,
            })
            .collect();
        let mut soft_reach = 0;
        for (index, greeble) in self.greebles.iter().enumerate() {
            for term in &greeble.near {
                let targets: Vec<usize> = term.greebles.iter().map(|id| greebles[id]).collect();
                if term.factor == 0.0 {
                    for target in &targets {
                        compiled[*target]
                            .bans
                            .push((term.radius, greeble_sets[index].clone()));
                        compiled[index]
                            .bans
                            .push((term.radius, greeble_sets[*target].clone()));
                    }
                } else {
                    let mut looks_for = vec![false; self.greebles.len()];
                    for target in targets {
                        looks_for[target] = true;
                    }
                    soft_reach = soft_reach.max(term.radius);
                    compiled[index].soft.push(SoftTerm {
                        radius: term.radius,
                        targets: looks_for,
                        factor: f64::from(term.factor),
                    });
                }
            }
        }

        Ok(CompiledGreebleRules {
            all: set_of(&|_| true),
            unforced: set_of(&|variant| !carries_forcing[variant.greeble]),
            greebles: compiled,
            variants,
            words,
            connects,
            with_link,
            link_count,
            soft_reach,
        })
    }
}

/// What the solve placed on one cell: a greeble and its turn.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GreeblePlacement {
    /// The greeble, as an index for [`CompiledGreebleRules::greeble_id`].
    pub greeble: usize,
    /// Quarter turns clockwise seen from outside, in the cell frame: at one
    /// turn, link point `up` lies on slot `right`.
    pub turns: u8,
}

/// The result of one solve.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GreebleSolution {
    /// One entry per surface cell. `None` only where the solve without forcing
    /// greebles still contradicted; that cell gets no greeble.
    pub cells: Vec<Option<GreeblePlacement>>,
    /// How many times the solve started again after a contradiction.
    pub restarts: u32,
    /// Whether every attempt contradicted and the solve without forcing
    /// greebles produced this result.
    pub fallback: bool,
}

/// The seed a surface is solved with: [`Fnv64`] over the style id alone.
///
/// The structure is NOT in the seed. Each draw also hashes the plate's own
/// key (its cell and out face, [`crate::sections::skin_surface::SurfaceCell`]),
/// so plates still differ from each other, and an edit to the structure
/// changes the draws only where it changes the plates and their links.
pub fn greeble_seed(style: &str) -> u64 {
    Fnv64::new()
        .write(&(style.len() as u64).to_le_bytes())
        .write(style.as_bytes())
        .finish()
}

/// Places one greeble variant on every cell of `surface`.
///
/// Arc-consistent propagation over the slot links keeps every pair of facing
/// link types connected; a slot with no neighbour accepts any link type. The
/// next cell is the one with the fewest variants left, ties broken by a hash
/// of the seed, the cell key and the attempt, then by index. The variant is
/// drawn by weight from the same hash family, each weight scaled by the soft
/// `near` terms that match greebles already placed within their radius.
/// Placing a greeble applies its factor-0 bans within their radius, and
/// reaching a greeble's `max_count` bans it everywhere else.
///
/// On a contradiction the solve starts again with the next attempt, up to
/// [`GREEBLE_RESTARTS`] times. If every attempt fails it solves once without
/// the greebles that carry a forcing link type, logs a warning, and leaves any
/// cell that still contradicts without a greeble.
pub fn solve_greebles(
    surface: &SkinSurface,
    rules: &CompiledGreebleRules,
    seed: u64,
) -> GreebleSolution {
    for attempt in 0..=GREEBLE_RESTARTS {
        if let Ok(cells) = Collapse::new(surface, rules, seed, attempt, &rules.all, false).run() {
            return GreebleSolution {
                cells,
                restarts: attempt,
                fallback: false,
            };
        }
    }
    warn!(
        "greeble solve of {} cells contradicted on {} attempts with seed {seed:#x}; solved without forcing greebles",
        surface.len(),
        GREEBLE_RESTARTS + 1,
    );
    let cells = Collapse::new(
        surface,
        rules,
        seed,
        GREEBLE_RESTARTS + 1,
        &rules.unforced,
        true,
    )
    .run()
    .expect("a lenient solve leaves contradicted cells empty instead of failing");
    GreebleSolution {
        cells,
        restarts: GREEBLE_RESTARTS,
        fallback: true,
    }
}

/// A cell lost its last variant.
#[derive(Debug)]
struct Contradiction;

/// What a draw hash is for, so two draws on one cell stay apart.
#[derive(Clone, Copy)]
enum Purpose {
    Pick = 1,
    Draw = 2,
}

/// One attempt at a solve.
struct Collapse<'a> {
    surface: &'a SkinSurface,
    rules: &'a CompiledGreebleRules,
    seed: u64,
    attempt: u32,
    /// A contradicted cell is left empty rather than failing the attempt.
    lenient: bool,
    domains: Vec<u64>,
    decided: Vec<bool>,
    cells: Vec<Option<GreeblePlacement>>,
    counts: Vec<u32>,
    queue: VecDeque<usize>,
    queued: Vec<bool>,
}

impl<'a> Collapse<'a> {
    fn new(
        surface: &'a SkinSurface,
        rules: &'a CompiledGreebleRules,
        seed: u64,
        attempt: u32,
        start: &[u64],
        lenient: bool,
    ) -> Self {
        let count = surface.len();
        Self {
            surface,
            rules,
            seed,
            attempt,
            lenient,
            domains: start.repeat(count),
            decided: vec![false; count],
            cells: vec![None; count],
            counts: vec![0; rules.greebles.len()],
            queue: (0..count).collect(),
            queued: vec![true; count],
        }
    }

    fn run(mut self) -> Result<Vec<Option<GreeblePlacement>>, Contradiction> {
        for cell in 0..self.surface.len() {
            if self.domain(cell).iter().all(|word| *word == 0) {
                self.contradict(cell)?;
            }
        }
        self.propagate()?;
        while let Some(cell) = self.pick() {
            let variant = self.draw(cell);
            self.place(cell, variant)?;
            self.propagate()?;
        }
        Ok(self.cells)
    }

    fn domain(&self, cell: usize) -> &[u64] {
        &self.domains[self.rules.variant_set(cell)]
    }

    fn hash(&self, cell: usize, purpose: Purpose) -> u64 {
        let key = self.surface.cells()[cell].key;
        let hash = Fnv64::new()
            .write(&self.seed.to_le_bytes())
            .write(&key.to_le_bytes())
            .write(&self.attempt.to_le_bytes())
            .write(&[purpose as u8])
            .finish();
        avalanche(hash)
    }

    /// The undecided cell with the fewest variants, ties to the lowest hash
    /// then the lowest index.
    fn pick(&self) -> Option<usize> {
        (0..self.surface.len())
            .filter(|cell| !self.decided[*cell])
            .min_by_key(|cell| {
                let size: u32 = self
                    .domain(*cell)
                    .iter()
                    .map(|word| word.count_ones())
                    .sum();
                (size, self.hash(*cell, Purpose::Pick), *cell)
            })
    }

    /// A variant of `cell`'s domain drawn by weight.
    fn draw(&self, cell: usize) -> usize {
        let factors = self.soft_factors(cell);
        let domain = self.domain(cell);
        let candidates: Vec<(usize, f64)> = (0..self.rules.variants.len())
            .filter(|index| domain[index / 64] & (1 << (index % 64)) != 0)
            .map(|index| {
                let variant = &self.rules.variants[index];
                (index, variant.weight * factors[variant.greeble])
            })
            .collect();
        let total: f64 = candidates.iter().map(|(_, weight)| weight).sum();
        let unit = (self.hash(cell, Purpose::Draw) >> 11) as f64 / (1u64 << 53) as f64;
        let mut target = unit * total;
        for (index, weight) in &candidates {
            if target < *weight {
                return *index;
            }
            target -= weight;
        }
        candidates
            .last()
            .expect("an undecided cell keeps at least one variant")
            .0
    }

    /// Per greeble, the product of its soft terms that match a greeble placed
    /// within their radius of `cell`.
    fn soft_factors(&self, cell: usize) -> Vec<f64> {
        let mut factors = vec![1.0; self.rules.greebles.len()];
        if self.rules.soft_reach == 0 {
            return factors;
        }
        let near: Vec<(usize, u32)> = ball(self.surface, cell, self.rules.soft_reach)
            .into_iter()
            .filter_map(|(at, distance)| {
                self.cells[at].map(|placement| (placement.greeble, distance))
            })
            .collect();
        for (index, greeble) in self.rules.greebles.iter().enumerate() {
            for term in &greeble.soft {
                let matched = near
                    .iter()
                    .any(|(placed, distance)| *distance <= term.radius && term.targets[*placed]);
                if matched {
                    factors[index] *= term.factor;
                }
            }
        }
        factors
    }

    fn place(&mut self, cell: usize, variant: usize) -> Result<(), Contradiction> {
        let rules = self.rules;
        let greeble = rules.variants[variant].greeble;
        let range = rules.variant_set(cell);
        self.domains[range.clone()].fill(0);
        self.domains[range.start + variant / 64] = 1 << (variant % 64);
        self.decided[cell] = true;
        self.cells[cell] = Some(GreeblePlacement {
            greeble,
            turns: rules.variants[variant].turns,
        });
        self.counts[greeble] += 1;
        self.enqueue(cell);

        for (radius, banned) in &rules.greebles[greeble].bans {
            for (at, distance) in ball(self.surface, cell, *radius) {
                if distance > 0 && !self.decided[at] {
                    self.restrict(at, |index, word| word & !banned[index])?;
                }
            }
        }
        if rules.greebles[greeble].max_count == Some(self.counts[greeble]) {
            let banned = &rules.greebles[greeble].variants;
            for at in 0..self.surface.len() {
                if !self.decided[at] {
                    self.restrict(at, |index, word| word & !banned[index])?;
                }
            }
        }
        Ok(())
    }

    /// Narrows `cell`'s domain word by word; queues it when it changed.
    fn restrict(
        &mut self,
        cell: usize,
        keep: impl Fn(usize, u64) -> u64,
    ) -> Result<(), Contradiction> {
        let range = self.rules.variant_set(cell);
        let mut changed = false;
        let mut left = 0;
        for (index, word) in self.domains[range].iter_mut().enumerate() {
            let narrowed = keep(index, *word);
            changed |= narrowed != *word;
            left |= narrowed;
            *word = narrowed;
        }
        if !changed {
            return Ok(());
        }
        if left == 0 {
            return self.contradict(cell);
        }
        self.enqueue(cell);
        Ok(())
    }

    /// A cell with no variant left: the attempt fails, or in a lenient solve
    /// the cell is decided with no greeble and stops constraining its
    /// neighbours.
    fn contradict(&mut self, cell: usize) -> Result<(), Contradiction> {
        if !self.lenient {
            return Err(Contradiction);
        }
        self.decided[cell] = true;
        self.cells[cell] = None;
        Ok(())
    }

    fn enqueue(&mut self, cell: usize) {
        if !self.queued[cell] {
            self.queued[cell] = true;
            self.queue.push_back(cell);
        }
    }

    /// AC-3 over the slot links: every neighbour of a changed cell keeps only
    /// the variants whose link type on the shared slot connects one the
    /// changed cell can still show there.
    fn propagate(&mut self) -> Result<(), Contradiction> {
        while let Some(changed) = self.queue.pop_front() {
            self.queued[changed] = false;
            let from = self.domain(changed).to_vec();
            if from.iter().all(|word| *word == 0) {
                continue;
            }
            for (slot, link) in self.surface.cells()[changed].links.iter().enumerate() {
                let Some(link) = link else { continue };
                let mask = self.supported(&from, slot, usize::from(link.slot));
                self.restrict(link.cell, |index, word| word & mask[index])?;
            }
        }
        Ok(())
    }

    /// The variants a neighbour may hold on its slot `far` when this cell's
    /// domain is `from` and its own slot is `near`.
    fn supported(&self, from: &[u64], near: usize, far: usize) -> Vec<u64> {
        let rules = self.rules;
        let shown = (0..rules.link_count)
            .filter(|link| {
                rules.with_link[near * rules.link_count + link]
                    .iter()
                    .zip(from)
                    .any(|(with, domain)| with & domain != 0)
            })
            .fold(0u64, |set, link| set | rules.connects[link]);
        let mut mask = vec![0u64; rules.words];
        for link in bits(shown) {
            for (word, with) in mask
                .iter_mut()
                .zip(&rules.with_link[far * rules.link_count + link])
            {
                *word |= with;
            }
        }
        mask
    }
}

/// Every cell within `radius` links of `from`, with its distance, `from`
/// included at distance 0. Breadth-first in slot order.
fn ball(surface: &SkinSurface, from: usize, radius: u32) -> Vec<(usize, u32)> {
    let mut distance = vec![u32::MAX; surface.len()];
    distance[from] = 0;
    let mut found = vec![(from, 0)];
    let mut next = 0;
    while next < found.len() {
        let (at, steps) = found[next];
        next += 1;
        if steps == radius {
            continue;
        }
        for link in surface.cells()[at].links.iter().flatten() {
            if distance[link.cell] == u32::MAX {
                distance[link.cell] = steps + 1;
                found.push((link.cell, steps + 1));
            }
        }
    }
    found
}

/// The indices of the set bits of `set`, lowest first.
fn bits(set: u64) -> impl Iterator<Item = usize> {
    (0..64).filter(move |bit| set & (1 << bit) != 0)
}

/// The splitmix64 finaliser. FNV-1a mixes its last input bytes into the high
/// bits poorly; this spreads every input bit over the whole word.
fn avalanche(mut hash: u64) -> u64 {
    hash ^= hash >> 30;
    hash = hash.wrapping_mul(0xbf58_476d_1ce4_e5b9);
    hash ^= hash >> 27;
    hash = hash.wrapping_mul(0x94d0_49bb_1331_11eb);
    hash ^ (hash >> 31)
}

#[cfg(test)]
mod tests;
