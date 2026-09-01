//! The impact table: what a round SOUNDS like against what it hit.
//!
//! A hit has two halves, and until this module only one of them could be
//! authored. The TARGET half was a per-object `impact_sound` field - a rock
//! sounded like a rock because the rock named its own file - but the ROUND half
//! had nowhere to go, so a penetrator and a slug hitting the same plate made
//! the same noise. The table closes that: content authors
//! `(damage type, material) -> sound`, and the target only has to say what it
//! is MADE of.
//!
//! Materials are open strings, not an enum, for the same reason style ids are:
//! a mod that adds ice, ceramic or flesh adds a material by naming one, and the
//! table it authors alongside is what teaches the game to hear it.
//!
//! The table is sparse and falls back once. An entry with no material is the
//! DEFAULT for its damage type - what that round sounds like against anything
//! the table does not name - so the base game authors four entries, not one per
//! pair. Nothing else falls back: a damage type with neither an exact entry nor
//! a default is SILENT, like every other authored voice in the game.

use bevy::prelude::*;

use crate::{asset_ref::prelude::AssetRef, damage::prelude::DamageType};

/// The material tag, the authored table entry, the registry and the two
/// material ids the base game ships.
pub mod prelude {
    pub use super::{
        GameImpacts, ImpactSoundConfig, SurfaceMaterial, MATERIAL_HULL, MATERIAL_ROCK,
    };
}

/// Ship plate: every section in the catalog.
pub const MATERIAL_HULL: &str = "hull";

/// Asteroid stone.
pub const MATERIAL_ROCK: &str = "rock";

/// What a damage target is MADE of, snapshotted unresolved from its config
/// (`BaseSectionConfig::material`, `AsteroidConfig::material`).
///
/// One open string, matched against [`ImpactSoundConfig::material`]. A target
/// without this component is not an error - it takes the damage type's default
/// entry, which is what makes the table sparse.
///
/// The audio observer walks UP `ChildOf` to find it, because an asteroid keeps
/// its health on a child node while the tag sits on the rock's root bundle.
#[derive(Component, Clone, Debug, Reflect)]
#[reflect(Component)]
pub struct SurfaceMaterial(pub String);

impl SurfaceMaterial {
    /// The tag for a named material.
    pub fn new(material: impl Into<String>) -> Self {
        Self(material.into())
    }
}

/// One authored row of the impact table: this round, against this material,
/// sounds like this.
///
/// One content item per row rather than one nested table, so a mod can override
/// a single pair by re-declaring its `id` and nothing else - the same last-wins
/// overlay every other content kind gets.
#[derive(Clone, Debug, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ImpactSoundConfig {
    /// Stable content id. The overlay key: a mod re-declaring this id replaces
    /// the row in place.
    pub id: String,
    /// The round side of the pair.
    pub damage: DamageType,
    /// The material struck, matched against [`SurfaceMaterial`]. `None` is the
    /// DEFAULT row for this damage type: what the round sounds like against
    /// anything the table does not name.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub material: Option<String>,
    /// The voice. Authored as a path like every other sound.
    #[reflect(ignore)]
    pub sound: AssetRef<AudioSource>,
}

/// The merged impact table, in registration order (base then mods), overlaid
/// last-wins by [`ImpactSoundConfig::id`].
///
/// Inserted by the content merge. Init'd empty by `NovaDamagePlugin`, so a rig
/// that loads no content resolves nothing and plays nothing rather than
/// panicking on a missing resource.
#[derive(Resource, Clone, Debug, Default)]
pub struct GameImpacts(pub Vec<ImpactSoundConfig>);

impl GameImpacts {
    /// The voice for a hit: the exact `(kind, material)` row if one is
    /// authored, else the `kind`'s default row, else nothing.
    ///
    /// Order is the registration order, so where content authors the same pair
    /// twice under two ids the FIRST row wins - a duplicate is an authoring
    /// mistake, not a precedence rule.
    pub fn sound(
        &self,
        kind: DamageType,
        material: Option<&str>,
    ) -> Option<&AssetRef<AudioSource>> {
        let exact = material.and_then(|material| {
            self.0
                .iter()
                .find(|row| row.damage == kind && row.material.as_deref() == Some(material))
        });
        exact
            .or_else(|| {
                self.0
                    .iter()
                    .find(|row| row.damage == kind && row.material.is_none())
            })
            .map(|row| &row.sound)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(id: &str, damage: DamageType, material: Option<&str>, sound: &str) -> ImpactSoundConfig {
        ImpactSoundConfig {
            id: id.to_string(),
            damage,
            material: material.map(str::to_string),
            sound: AssetRef::from(sound),
        }
    }

    fn table() -> GameImpacts {
        GameImpacts(vec![
            row("kinetic", DamageType::Kinetic, None, "impact.wav"),
            row(
                "kinetic_rock",
                DamageType::Kinetic,
                Some("rock"),
                "rock.wav",
            ),
            row("pierce", DamageType::Pierce, None, "pierce.wav"),
        ])
    }

    fn path(found: Option<&AssetRef<AudioSource>>) -> Option<String> {
        found.map(|r| match r {
            AssetRef::Path(path) => path.clone(),
            AssetRef::Handle(_) => unreachable!("the test table authors paths"),
        })
    }

    #[test]
    fn a_named_material_takes_its_own_row_over_the_types_default() {
        let table = table();
        assert_eq!(
            path(table.sound(DamageType::Kinetic, Some("rock"))).as_deref(),
            Some("rock.wav")
        );
        assert_eq!(
            path(table.sound(DamageType::Kinetic, Some("hull"))).as_deref(),
            Some("impact.wav")
        );
        assert_eq!(
            path(table.sound(DamageType::Kinetic, None)).as_deref(),
            Some("impact.wav")
        );
    }

    #[test]
    fn a_material_falls_back_only_within_its_own_damage_type() {
        let table = table();
        // The rock has a Kinetic row and no Pierce row, so a penetrator into
        // stone takes the PIERCE default - never the kinetic rock row.
        assert_eq!(
            path(table.sound(DamageType::Pierce, Some("rock"))).as_deref(),
            Some("pierce.wav")
        );
    }

    #[test]
    fn a_damage_type_the_table_never_names_is_silent() {
        assert!(table().sound(DamageType::Explosive, Some("hull")).is_none());
        assert!(GameImpacts::default()
            .sound(DamageType::Kinetic, None)
            .is_none());
    }
}
