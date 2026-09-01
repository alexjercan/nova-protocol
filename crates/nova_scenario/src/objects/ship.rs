//! The SHIP content kind: a hull authored once and spawned by id.
//!
//! [`ShipConfig`] owns what is intrinsic to a hull - its sections, the look it
//! wears, how far it must be dismantled before it collapses - and a scenario
//! spawn owns the rest (where it sits, who drives it, which side it is on).
//!
//! Touch this module when changing what a ship IS, apart from any one spawn of
//! it.

use bevy::prelude::*;
use nova_gameplay::asset_ref::AssetRef;

use crate::objects::{
    modification::prelude::SectionModification,
    spaceship::prelude::{SectionId, SpaceshipSectionConfig},
};

/// The ship content kind: its id, hull, spawn-time source and delta, and the
/// loaded catalog.
pub mod prelude {
    pub use super::{GameShips, ShipConfig, ShipHull, ShipId, ShipSectionModification, ShipSource};
}

/// The id a ship is referenced by, from a scenario spawn or another mod.
pub type ShipId = String;

/// What a ship IS, apart from any one spawn of it: its section list plus the
/// hull-wide properties derived from that structure.
///
/// Everything here is REUSABLE - two scenarios spawning the same hull want the
/// same answer. Anything that differs per spawn (position, name, driver,
/// allegiance) belongs on the scenario object instead, and anything that
/// differs per spawn but is still about the STRUCTURE is a
/// [`ShipSectionModification`].
#[derive(Clone, Debug, Default, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ShipHull {
    /// The ship's sections (hull, thrusters, weapons, controller) and their
    /// placement. Empty by default; each is spawned as a child at load.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Vec::is_empty")
    )]
    pub sections: Vec<SpaceshipSectionConfig>,
    /// Structural collapse: the fraction of the hull the ship was BUILT with
    /// (its pinned maximum health) below which what is left comes apart and
    /// the whole ship is destroyed. `None` (the authored default - omit the
    /// field) uses `DEFAULT_STRUCTURAL_COLLAPSE_THRESHOLD`. Lower means the
    /// ship must be dismantled further before it goes, which is how a capital
    /// takes more killing than a fighter; `Some(0.0)` is "strip every last
    /// section". In strict RON the `Option` keeps its variant:
    /// `collapse_threshold: Some(0.1)`.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub collapse_threshold: Option<f32>,
    /// Whether the ship wears a DERIVED skin: cladding computed from the
    /// structure above at spawn, with nothing authored and nothing saved. See
    /// `ShipSkin`.
    ///
    /// `false` by default, and off for every shipped ship: the derivation reads
    /// a hull as unit cells, which the catalog's cube sections are and the
    /// modelled semantic parts are not.
    #[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "is_false"))]
    pub skin: bool,
    /// The LOOK the derived skin wears, by style id: the material of each
    /// surface role plus the decoration scattered over it. See
    /// `ShipStyleConfig`.
    ///
    /// `None` (omit the field) is the undressed derivation - built-in plate
    /// colours and no greebles. A style named here but authored by nobody leaves
    /// the ship bare rather than falling back to another look, so a missing mod
    /// is visible instead of silently substituted.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub style: Option<String>,
    /// The sound this hull makes when it COLLAPSES - the moment it stops being
    /// a ship and becomes wreckage, which is one event however many frames the
    /// sections then take to peel away.
    ///
    /// Authored on the hull rather than derived from its parts because a hull
    /// failing is not a section failing loudly: it is the spine going, and the
    /// only thing that knows how big that is is the ship. AUTHORED-OR-SILENT -
    /// a hull that names none comes apart to the sound of its own sections,
    /// which is what every ship did before this field existed.
    #[reflect(ignore)]
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub collapse_sound: Option<AssetRef<AudioSource>>,
}

/// `skip_serializing_if` predicate for a `bool` that defaults to false, so an
/// unclad ship keeps the field out of its RON entirely.
#[cfg(feature = "serde")]
fn is_false(flag: &bool) -> bool {
    !*flag
}

/// One authored ship in the catalog: an id, a name, and the hull it is.
///
/// Resolved by [`id`](ShipConfig::id) out of [`GameShips`], which the mod merge
/// fills exactly as it fills the section and style catalogs - so a mod's ship
/// with the id of a base one REPLACES it, and a new id is a new ship.
#[derive(Clone, Debug, Default, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ShipConfig {
    /// The id a scenario spawns this ship by.
    pub id: ShipId,
    /// The name a picker would show. Not used at spawn - a spawned ship is
    /// named by the scenario object that placed it.
    pub name: String,
    /// The hull itself.
    pub hull: ShipHull,
}

/// Where a spawned ship's [`ShipHull`] comes from. Resolved at spawn in
/// `insert_spaceship_sections` (mirrors [`SectionSource`](crate::objects::spaceship::prelude::SectionSource)
/// one level up): an `Inline` hull is used as-is; a `Prototype` is looked up by
/// id in the ship catalog ([`GameShips`]). Keeping the compact authored form
/// (the id) in the scenario data is what stops eleven scenarios carrying eleven
/// copies of the same corvette.
#[derive(Clone, Debug, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
// Inline carries a whole section list next to Prototype's small id, and boxing
// it cannot compile here because the enum derives Reflect and bevy_reflect 0.19
// has no Reflect impl for Box<T>. Spawn-time config data, not per-frame state -
// the same trade `SectionSource` makes.
pub enum ShipSource {
    /// The hull, authored inline. The right shape for a one-off - a scripted
    /// battery that is one torpedo tube, an example's two-section test rig.
    Inline(ShipHull),
    /// A reference to a catalog ship by id, resolved against [`GameShips`] at
    /// spawn.
    Prototype(ShipId),
}

impl Default for ShipSource {
    fn default() -> Self {
        Self::Inline(ShipHull::default())
    }
}

impl ShipSource {
    /// The hull this source names: the inline one, or the catalog entry's.
    /// `None` when a prototype id resolves to nothing - the caller decides what
    /// a miss means (the spawn logs it and flies an empty root).
    pub fn resolve<'a>(&'a self, ships: &'a GameShips) -> Option<&'a ShipHull> {
        match self {
            ShipSource::Inline(hull) => Some(hull),
            ShipSource::Prototype(id) => ships.get_ship(id).map(|ship| &ship.hull),
        }
    }
}

/// A spawn-time delta applied to ONE section of the resolved hull.
///
/// The same data-only deltas an authored section carries
/// ([`SectionModification`]), aimed by section id one level up - so a scenario
/// can harden a shared corvette's flight computer or hand its turrets a fixed
/// magazine without forking the ship. Applied AFTER the section's own list, so
/// a spawn override wins.
#[derive(Clone, Debug, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ShipSectionModification {
    /// The hull-local section id this delta applies to. A id no section
    /// carries is an authoring error the content lint reports.
    pub section: SectionId,
    /// The deltas themselves.
    pub modifications: Vec<SectionModification>,
}

/// The loaded ship catalog: every [`ShipConfig`] the enabled mods registered,
/// in merge order. Filled by the mod merge (`nova_assets`'s `register_bundles`)
/// and read at spawn through [`get_ship`](GameShips::get_ship).
#[derive(Resource, Clone, Debug, Deref, DerefMut, Default)]
pub struct GameShips(pub Vec<ShipConfig>);

impl GameShips {
    /// The ship with this id, or `None` if nothing authored it.
    pub fn get_ship(&self, id: &str) -> Option<&ShipConfig> {
        self.0.iter().find(|ship| ship.id == id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A prototype resolves out of the catalog and an unknown id resolves to
    /// nothing - the miss the spawn reports rather than panics on.
    #[test]
    fn a_ship_source_resolves_by_id_or_reports_nothing() {
        let ships = GameShips(vec![ShipConfig {
            id: "cargoa".to_string(),
            name: "Corvette".to_string(),
            hull: ShipHull {
                collapse_threshold: Some(0.25),
                ..default()
            },
        }]);

        let worn = ShipSource::Prototype("cargoa".to_string());
        assert_eq!(
            worn.resolve(&ships)
                .and_then(|hull| hull.collapse_threshold),
            Some(0.25)
        );
        assert!(ShipSource::Prototype("nothing".to_string())
            .resolve(&ships)
            .is_none());

        // An inline hull needs no catalog at all.
        let inline = ShipSource::Inline(ShipHull {
            skin: true,
            ..default()
        });
        assert!(inline
            .resolve(&GameShips::default())
            .is_some_and(|hull| hull.skin));
    }
}
