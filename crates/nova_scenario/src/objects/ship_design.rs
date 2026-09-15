//! The SHIP DESIGN content kind: an assembled section list authored once and
//! spawned by id.
//!
//! A [`ShipDesign`] owns what is intrinsic to a build - its placed sections,
//! how far it must be dismantled before it collapses, the look and the voice
//! it wears - and a scenario spawn owns the rest (where it sits, who drives
//! it, which side it is on, what it is PERMITTED to do).
//!
//! Design, not hull: `Hull` is already a section KIND, and a design is made of
//! hull sections among others.
//!
//! Touch this module when changing what a ship IS, apart from any one spawn of
//! it.

use std::collections::BTreeMap;

use bevy::prelude::*;
use nova_gameplay::asset_ref::AssetRef;
use nova_ship::prelude::{SectionConfig, SectionPatchError, DEFAULT_WARN_HULL_FRACTION};

use crate::objects::spaceship::prelude::{
    SectionId, SpaceshipSectionConfig, SpaceshipSectionConfigPatch,
};

/// The ship-design content kind: its id, design, spawn-time source, the
/// resolved form every consumer reads, and the loaded catalog.
pub mod prelude {
    pub use super::{
        resolve_ship_design, GameShipDesigns, ResolvedSection, ResolvedShipDesign, ShipDesign,
        ShipDesignError, ShipDesignId, ShipDesignPrototype, ShipDesignSource, ShipIntegrityConfig,
        ShipPresentationConfig,
    };
}

/// The id a ship design is referenced by, from a scenario spawn or another mod.
pub type ShipDesignId = String;

/// What a ship IS, apart from any one spawn of it: its section list plus the
/// design-wide properties that follow from that structure.
///
/// Everything here is REUSABLE - two scenarios spawning the same design want
/// the same answer. Anything that differs per spawn (position, name, driver,
/// allegiance, capabilities) belongs on the scenario object instead, and
/// anything that differs per spawn but is still about the STRUCTURE is a
/// [`SpaceshipSectionConfigPatch`].
#[derive(Clone, Debug, Default, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ShipDesign {
    /// The design's sections (hull, thrusters, weapons, controller) and their
    /// placement. Empty by default; each is spawned as a child at load.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Vec::is_empty")
    )]
    pub sections: Vec<SpaceshipSectionConfig>,
    /// When what is left of this design stops being a ship.
    #[cfg_attr(feature = "serde", serde(default))]
    pub integrity: ShipIntegrityConfig,
    /// What this design looks like and sounds like.
    #[cfg_attr(feature = "serde", serde(default))]
    pub presentation: ShipPresentationConfig,
}

/// When a design comes apart: the structural half of what a build IS.
#[derive(Clone, Copy, Debug, Default, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(default))]
pub struct ShipIntegrityConfig {
    /// Structural collapse: the fraction of the hull the ship was BUILT with
    /// (its pinned maximum health) below which what is left comes apart and
    /// the whole ship is destroyed. `None` (the authored default - omit the
    /// field) uses `DEFAULT_STRUCTURAL_COLLAPSE_THRESHOLD`. Lower means the
    /// ship must be dismantled further before it goes, which is how a capital
    /// takes more killing than a fighter; `Some(0.0)` is "strip every last
    /// section". In strict RON the `Option` keeps its variant:
    /// `collapse_threshold: Some(0.1)`.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub collapse_threshold: Option<f32>,
}

/// What a design LOOKS like and SOUNDS like: the derived skin, and the
/// feedback voice the hull speaks to its pilot with.
///
/// The voice used to live on the controller section, on the theory that the
/// computer is what talks to the pilot. A controller is attitude hardware, and
/// a hull that loses one does not stop being able to warn about its own
/// structure - so the set is the DESIGN's, snapshotted onto the root at spawn.
///
/// AUTHORED-OR-SILENT throughout: a sound nobody names plays nothing, rather
/// than falling back to another ship's.
#[derive(Clone, Debug, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(default))]
pub struct ShipPresentationConfig {
    /// Whether the ship wears a DERIVED skin: cladding computed from the
    /// structure at spawn, with nothing authored and nothing saved. See
    /// `ShipSkin`.
    ///
    /// `false` by default, and off for every shipped design: the derivation
    /// reads a hull as unit cells, which the catalog's cube sections are and
    /// the modelled semantic parts are not.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "is_false"))]
    pub skin: bool,
    /// The LOOK the derived skin wears, by style id: the material of each
    /// surface role plus the decoration scattered over it. See
    /// `ShipStyleConfig`.
    ///
    /// `None` (omit the field) is the undressed derivation - built-in plate
    /// colours and no greebles. A style named here but authored by nobody
    /// leaves the ship bare rather than falling back to another look, so a
    /// missing mod is visible instead of silently substituted.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub style: Option<String>,
    /// The sound this design makes when it COLLAPSES - the moment it stops
    /// being a ship and becomes wreckage, which is one event however many
    /// frames the sections then take to peel away.
    ///
    /// Authored on the design rather than derived from its parts because a
    /// hull failing is not a section failing loudly: it is the spine going,
    /// and the only thing that knows how big that is is the ship.
    #[reflect(ignore)]
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub collapse_sound: Option<AssetRef<AudioSource>>,
    /// Lock acquired (once per radar gesture).
    #[reflect(ignore)]
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub lock_on_sound: Option<AssetRef<AudioSource>>,
    /// Lock cleared (tap-clear).
    #[reflect(ignore)]
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub lock_off_sound: Option<AssetRef<AudioSource>>,
    /// Radar hold denied - the ship has no LOCK capability.
    #[reflect(ignore)]
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub radar_deny_sound: Option<AssetRef<AudioSource>>,
    /// Held radar gesture re-designated to a new target.
    #[reflect(ignore)]
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub radar_retarget_sound: Option<AssetRef<AudioSource>>,
    /// Weapons safety re-engaged (hot -> cold edge).
    #[reflect(ignore)]
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub safety_on_sound: Option<AssetRef<AudioSource>>,
    /// A hostile has this ship in its combat lock - the threat alarm.
    #[reflect(ignore)]
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub warn_lock_sound: Option<AssetRef<AudioSource>>,
    /// A magazine ran dry - the cockpit GAUGE, as distinct from the gun's own
    /// dead-trigger click out on the mount.
    #[reflect(ignore)]
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub ammo_dry_sound: Option<AssetRef<AudioSource>>,
    /// The hull is critical - ONE alarm, on the falling edge through
    /// [`warn_hull_fraction`](Self::warn_hull_fraction).
    #[reflect(ignore)]
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub warn_hull_sound: Option<AssetRef<AudioSource>>,
    /// The hull fraction the alarm above fires at, clamped to `0..=1`.
    ///
    /// A SHIP decision, and the reason it is authored at all: a cheap civilian
    /// build may warn late, or (at `0.0`) never warn until there is nothing
    /// left.
    #[cfg_attr(
        feature = "serde",
        serde(skip_serializing_if = "is_default_warn_hull_fraction")
    )]
    pub warn_hull_fraction: f32,
    /// RCS fine-adjust LOOP: sustained while the ship burns the RCS
    /// primitive, unlike the one-shots above.
    #[reflect(ignore)]
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub rcs_loop_sound: Option<AssetRef<AudioSource>>,
}

impl Default for ShipPresentationConfig {
    fn default() -> Self {
        Self {
            skin: false,
            style: None,
            collapse_sound: None,
            lock_on_sound: None,
            lock_off_sound: None,
            radar_deny_sound: None,
            radar_retarget_sound: None,
            safety_on_sound: None,
            warn_lock_sound: None,
            ammo_dry_sound: None,
            warn_hull_sound: None,
            warn_hull_fraction: DEFAULT_WARN_HULL_FRACTION,
            rcs_loop_sound: None,
        }
    }
}

/// `skip_serializing_if` predicate for a `bool` that defaults to false, so an
/// unclad design keeps the field out of its RON entirely.
#[cfg(feature = "serde")]
fn is_false(flag: &bool) -> bool {
    !*flag
}

/// `skip_serializing_if` for the hull-alarm threshold, so only a design that
/// actually chose a different one writes it.
#[cfg(feature = "serde")]
fn is_default_warn_hull_fraction(fraction: &f32) -> bool {
    *fraction == DEFAULT_WARN_HULL_FRACTION
}

/// One authored design in the catalog: an id, a name, and the design it is.
///
/// Resolved by [`id`](ShipDesignPrototype::id) out of [`GameShipDesigns`],
/// which the mod merge fills exactly as it fills the section and style
/// catalogs - so a mod's design with the id of a base one REPLACES it, and a
/// new id is a new design.
#[derive(Clone, Debug, Default, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ShipDesignPrototype {
    /// The id a scenario spawns this design by.
    pub id: ShipDesignId,
    /// The name a picker would show. Not used at spawn - a spawned ship is
    /// named by the scenario object that placed it.
    pub name: String,
    /// The design itself.
    pub design: ShipDesign,
}

/// Where a spawned ship's [`ShipDesign`] comes from, and what this one spawn
/// changes about it.
///
/// An `Inline` design is COMPLETE and takes no patch: there is no prototype
/// left to inherit from, so anything it would say it can simply say. A
/// `Prototype` is looked up by id and may tune named sections through
/// `section_patches` - which is what stops eleven scenarios carrying eleven
/// copies of the same corvette just to harden one engine.
#[derive(Clone, Debug, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
// Inline carries a whole section list next to Prototype's small id, and boxing
// it cannot compile here because the enum derives Reflect and bevy_reflect 0.19
// has no Reflect impl for Box<T>. Spawn-time config data, not per-frame state -
// the same trade `SectionSource` makes.
pub enum ShipDesignSource {
    /// The design, authored inline. The right shape for a one-off - a scripted
    /// battery that is one torpedo tube, an example's two-section test rig.
    Inline(ShipDesign),
    /// A reference to a catalog design by id, with this spawn's own patches
    /// over named sections of it.
    Prototype {
        /// The catalog id, resolved against [`GameShipDesigns`].
        id: ShipDesignId,
        /// Per-section patches keyed by the design's own section ids. Empty by
        /// default, which authored files omit; an id no section carries is a
        /// content lint error.
        #[cfg_attr(
            feature = "serde",
            serde(default, skip_serializing_if = "BTreeMap::is_empty")
        )]
        section_patches: BTreeMap<SectionId, SpaceshipSectionConfigPatch>,
    },
}

impl Default for ShipDesignSource {
    fn default() -> Self {
        Self::Inline(ShipDesign::default())
    }
}

impl ShipDesignSource {
    /// A prototype reference with nothing patched - the common authored form.
    pub fn prototype(id: impl Into<ShipDesignId>) -> Self {
        Self::Prototype {
            id: id.into(),
            section_patches: BTreeMap::new(),
        }
    }

    /// The catalog id this source names, if it is a prototype. What an error
    /// message says, available without a catalog.
    pub fn prototype_id(&self) -> Option<&str> {
        match self {
            ShipDesignSource::Inline(_) => None,
            ShipDesignSource::Prototype { id, .. } => Some(id),
        }
    }
}

/// The loaded ship-design catalog: every [`ShipDesignPrototype`] the enabled
/// mods registered, in merge order. Filled by the mod merge (`nova_assets`'s
/// `register_bundles`) and read through
/// [`get_design`](GameShipDesigns::get_design).
#[derive(Resource, Clone, Debug, Deref, DerefMut, Default)]
pub struct GameShipDesigns(pub Vec<ShipDesignPrototype>);

impl GameShipDesigns {
    /// The design with this id, or `None` if nothing authored it.
    pub fn get_design(&self, id: &str) -> Option<&ShipDesignPrototype> {
        self.0.iter().find(|design| design.id == id)
    }
}

/// One placed section of a design with every layer already applied: this is
/// what the spawn, the lint, the preload and the editor preview all read.
#[derive(Clone, Debug)]
pub struct ResolvedSection {
    /// The section's design-local id (keys input bindings, scripts, patches).
    pub id: SectionId,
    /// The mount cell relative to the ship root, in build-grid cells.
    pub position: Vec3,
    /// The mount rotation relative to the ship root.
    pub rotation: Quat,
    /// The finished section config: the prototype (or inline config) with the
    /// reference's own patch and then the spawn's patch applied.
    pub config: SectionConfig,
}

/// A whole design with every prototype resolved and every patch applied.
///
/// Resolution happens ONCE, up front, and everything downstream reads this -
/// so the lint, the preview, the balance audit and the live spawn can never
/// disagree about what a ship actually is.
#[derive(Clone, Debug, Default)]
pub struct ResolvedShipDesign {
    /// The placed, finished sections.
    pub sections: Vec<ResolvedSection>,
    /// The design's structural configuration.
    pub integrity: ShipIntegrityConfig,
    /// The design's look and voice.
    pub presentation: ShipPresentationConfig,
}

/// What went wrong while resolving a design. Reported by the content lint and
/// logged by the spawn, which carries on with whatever DID resolve.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ShipDesignError {
    /// The source names a design no mod authored.
    UnknownDesign(ShipDesignId),
    /// A section references a prototype no mod authored. That section is
    /// skipped; the rest of the ship still flies.
    UnknownSectionPrototype {
        /// The design-local section id.
        section: SectionId,
        /// The catalog id it asked for.
        prototype: String,
    },
    /// A spawn patch names a section this design does not carry - the typo
    /// case, which would otherwise silently change nothing.
    UnknownPatchTarget(SectionId),
    /// A patch is not applicable to the section it resolved to.
    Patch {
        /// The design-local section id.
        section: SectionId,
        /// Why the patch was refused.
        error: SectionPatchError,
    },
}

impl std::fmt::Display for ShipDesignError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ShipDesignError::UnknownDesign(id) => {
                write!(f, "unknown ship design '{id}'")
            }
            ShipDesignError::UnknownSectionPrototype { section, prototype } => write!(
                f,
                "section '{section}' references unknown section prototype '{prototype}'"
            ),
            ShipDesignError::UnknownPatchTarget(section) => {
                write!(f, "no section '{section}' to patch in this design")
            }
            ShipDesignError::Patch { section, error } => {
                write!(f, "section '{section}': {error}")
            }
        }
    }
}

/// Resolve a design source into finished sections: look the design up, resolve
/// each section's own source, apply that reference's patch, then apply this
/// spawn's patch for the section.
///
/// The ONE resolver. The spawn, the content lint, the preload walk, the
/// balance audit and the editor's preview all call it, so a new layer or a new
/// lookup rule lands on all of them at once.
///
/// Returns what resolved AND what did not: a bad section is skipped rather
/// than fatal, because a hull missing one engine is still a ship worth
/// spawning and a scenario should not die at load over a mod overlay that
/// dropped a part. The lint turns the same list into authoring errors.
pub fn resolve_ship_design(
    source: &ShipDesignSource,
    designs: &GameShipDesigns,
    sections: &nova_ship::prelude::GameSections,
) -> (ResolvedShipDesign, Vec<ShipDesignError>) {
    let mut errors = Vec::new();
    let empty = BTreeMap::new();
    let (design, patches) = match source {
        ShipDesignSource::Inline(design) => (Some(design), &empty),
        ShipDesignSource::Prototype {
            id,
            section_patches,
        } => match designs.get_design(id) {
            Some(prototype) => (Some(&prototype.design), section_patches),
            None => {
                errors.push(ShipDesignError::UnknownDesign(id.clone()));
                (None, section_patches)
            }
        },
    };
    let Some(design) = design else {
        return (ResolvedShipDesign::default(), errors);
    };

    let mut resolved = ResolvedShipDesign {
        sections: Vec::with_capacity(design.sections.len()),
        integrity: design.integrity,
        presentation: design.presentation.clone(),
    };

    for section in &design.sections {
        let Some(mut config) = section.source.resolve(Some(sections)).cloned() else {
            errors.push(ShipDesignError::UnknownSectionPrototype {
                section: section.id.clone(),
                prototype: section.source.prototype_id().to_string(),
            });
            continue;
        };
        // The reference's own patch first, then the spawn's: the outer layer
        // runs last and wins field by field.
        if let Err(error) = section.source.patch().apply(&mut config) {
            errors.push(ShipDesignError::Patch {
                section: section.id.clone(),
                error,
            });
        }
        let mut position = section.position;
        let mut rotation = section.rotation;
        if let Some(patch) = patches.get(&section.id) {
            if let Some(patched) = patch.position {
                position = patched;
            }
            if let Some(patched) = patch.rotation {
                rotation = patched;
            }
            if let Err(error) = patch.config.apply(&mut config) {
                errors.push(ShipDesignError::Patch {
                    section: section.id.clone(),
                    error,
                });
            }
        }
        resolved.sections.push(ResolvedSection {
            id: section.id.clone(),
            position,
            rotation,
            config,
        });
    }

    // A patch aimed at a section that is not there changes nothing, silently,
    // which is exactly the typo this reports.
    for id in patches.keys() {
        if !design.sections.iter().any(|section| &section.id == id) {
            errors.push(ShipDesignError::UnknownPatchTarget(id.clone()));
        }
    }

    (resolved, errors)
}

#[cfg(test)]
mod tests;
