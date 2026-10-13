//! The sections a ship's design places, kept on the ship root so a section
//! the ship has lost can still be named, coded and rebuilt.
//!
//! `nova_scenario`'s section spawn resolves the design and inserts
//! [`ShipDesignSections`] beside the sections it spawns. A lost section is a
//! design section with no live child of that id. A Ship pane rebuild raises a
//! [`SectionRebuildRequest`], and the scenario crate spawns that design
//! section again through the same per-section spawn the ship was built with.

use bevy::prelude::*;
use nova_gameplay::prelude::SectionClass;

use super::prelude::*;

/// The design sections list, the rebuild request and the anchor rule.
pub mod prelude {
    pub use super::{rebuild_anchored, DesignSection, SectionRebuildRequest, ShipDesignSections};
}

/// One placed section of a ship's resolved design.
#[derive(Clone, Debug)]
pub struct DesignSection {
    /// The section's design-local id: the `EntityId` its live entity wears.
    pub id: String,
    /// The mount cell relative to the ship root, in build-grid cells.
    pub position: Vec3,
    /// The mount rotation relative to the ship root.
    pub rotation: Quat,
    /// The finished config the section spawns from, with every patch applied.
    pub config: SectionConfig,
}

/// Every section a ship's design places, in design order, whether or not it
/// is live. Inserted on the root by the section spawn of an authored ship; a
/// hand-built root has none.
#[derive(Component, Clone, Debug, Default)]
pub struct ShipDesignSections(pub Vec<DesignSection>);

impl ShipDesignSections {
    /// The design section with id `id`, or `None` when the design has none.
    pub fn get(&self, id: &str) -> Option<&DesignSection> {
        self.0.iter().find(|section| section.id == id)
    }

    /// Design section `id`'s class and its 1-based place among the design
    /// sections of that class, in id order: the index a section code shows.
    /// It reads the design and not the live sections, so it holds while a
    /// sibling is lost and when the section is rebuilt. `None` when the
    /// design has no section `id`.
    pub fn class_index(&self, id: &str) -> Option<(SectionClass, u32)> {
        let class = self.get(id)?.config.kind.class();
        let before = self
            .0
            .iter()
            .filter(|section| section.config.kind.class() == class && section.id.as_str() < id)
            .count();
        Some((class, before as u32 + 1))
    }
}

/// Asks to spawn design section `section` of ship `entity` again, at full
/// health with the ship's binding for it. Raised by a Ship pane rebuild after
/// the rebuild rule took the recipe; the scenario crate observes it.
#[derive(EntityEvent, Clone, Debug)]
pub struct SectionRebuildRequest {
    /// The ship root.
    pub entity: Entity,
    /// The design section id to spawn.
    pub section: String,
}

/// Whether `lost`, placed where its design puts it, mates with at least one
/// of the `live` sections: a rebuilt section with no mate would join no
/// structure.
///
/// Only the live sections whose sockets can reach the lost one's are read,
/// so a two-thousand-section hull costs a handful of comparisons.
pub fn rebuild_anchored(
    live: &[PlacedSectionLinkPoints<'_>],
    lost: PlacedSectionLinkPoints<'_>,
) -> bool {
    let reach = |section: &PlacedSectionLinkPoints<'_>| {
        section
            .link_points
            .iter()
            .map(|point| point.position.length())
            .fold(0.0_f32, f32::max)
    };
    let lost_reach = reach(&lost);
    let mut placed = vec![lost];
    placed.extend(live.iter().copied().filter(|section| {
        section.position.distance(lost.position) <= reach(section) + lost_reach + 0.01
    }));
    candidate_link_point_mates(&placed)
        .iter()
        .any(|mate| (mate.a.section_index == 0) != (mate.b.section_index == 0))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cube_points() -> Vec<LinkPoint> {
        [
            ("px", Vec3::X),
            ("nx", Vec3::NEG_X),
            ("py", Vec3::Y),
            ("ny", Vec3::NEG_Y),
            ("pz", Vec3::Z),
            ("nz", Vec3::NEG_Z),
        ]
        .into_iter()
        .map(|(id, normal)| LinkPoint {
            id: id.to_string(),
            position: normal * 0.5,
            normal,
            skin: LinkPointSkinType::Clad,
        })
        .collect()
    }

    /// Codes count the design, not the live hull: a turret's index is its
    /// place among the design's turrets by id, whatever else is lost.
    #[test]
    fn a_section_index_counts_its_class_in_the_design_by_id() {
        let section = |id: &str, kind: SectionKind| DesignSection {
            id: id.to_string(),
            position: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            config: SectionConfig {
                base: BaseSectionConfig::default(),
                kind,
            },
        };
        let turret = || SectionKind::Turret(TurretSectionConfig::default());
        let hull = || SectionKind::Hull(HullSectionConfig::default());
        let design = ShipDesignSections(vec![
            section("pdc_starboard", turret()),
            section("bow", hull()),
            section("pdc_port", turret()),
            section("aft", hull()),
        ]);
        assert_eq!(
            design.class_index("pdc_port"),
            Some((SectionClass::Turret, 1))
        );
        assert_eq!(
            design.class_index("pdc_starboard"),
            Some((SectionClass::Turret, 2))
        );
        assert_eq!(design.class_index("bow"), Some((SectionClass::Hull, 2)));
        assert_eq!(design.class_index("missing"), None);
    }

    /// A cube beside a live cube mates with it; the same cube two cells away
    /// mates with nothing and would float free.
    #[test]
    fn a_lost_section_is_anchored_only_beside_a_live_one() {
        let points = cube_points();
        let place = |x: f32| PlacedSectionLinkPoints {
            position: Vec3::new(x, 0.0, 0.0),
            rotation: Quat::IDENTITY,
            link_points: &points,
        };
        let live = [place(0.0)];
        assert!(rebuild_anchored(&live, place(1.0)));
        assert!(!rebuild_anchored(&live, place(2.0)));
    }
}
