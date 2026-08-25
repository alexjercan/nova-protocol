//! The alphabet's one rule: no two kinds wear the same mark.

use nova_scenario::prelude::{ScenarioObjectKind, SectionSource, SpaceshipConfig};
use nova_ship::prelude::{
    BaseSectionConfig, ControllerSectionConfig, HullSectionConfig, SectionConfig, SectionKind,
    ThrusterSectionConfig, TorpedoSectionConfig, TurretSectionConfig,
};

use super::*;

fn section(kind: SectionKind) -> SectionNode {
    SectionNode {
        source: SectionSource::Inline(SectionConfig {
            base: BaseSectionConfig::default(),
            kind,
        }),
        modifications: Vec::new(),
        binds: Vec::new(),
    }
}

/// `o` used to be a controller AND an asteroid, `!` a torpedo AND a beacon.
/// A mark two kinds share marks neither.
#[test]
fn every_kind_the_editor_draws_wears_its_own_mark() {
    let mut marks = vec![
        SCENARIO,
        SHIP_PLAYER,
        SHIP_AI,
        section_mark(
            &section(SectionKind::Hull(HullSectionConfig::default())),
            None,
        )
        .0,
        section_mark(
            &section(SectionKind::Controller(ControllerSectionConfig::default())),
            None,
        )
        .0,
        section_mark(
            &section(SectionKind::Thruster(ThrusterSectionConfig::default())),
            None,
        )
        .0,
        section_mark(
            &section(SectionKind::Turret(TurretSectionConfig::default())),
            None,
        )
        .0,
        section_mark(
            &section(SectionKind::Torpedo(TorpedoSectionConfig::default())),
            None,
        )
        .0,
        // The one object kind the palette does not offer: a scenario can hold a
        // ship the editor does not design, and the tree still has to draw it.
        object_mark(&ObjectNode {
            name: String::new(),
            kind: ScenarioObjectKind::Spaceship(SpaceshipConfig::default()),
        })
        .0,
    ];
    marks.extend(ObjectChoice::ALL.map(choice_mark));

    let drawn = marks.len();
    marks.sort_unstable();
    marks.dedup();

    assert_eq!(marks.len(), drawn, "two kinds share a mark: {marks:?}");
}

/// The Add row that creates a rock wears what the rock will wear, which is how
/// the menu teaches the tree's alphabet without a legend.
#[test]
fn an_add_row_wears_the_mark_the_node_it_creates_will() {
    for choice in ObjectChoice::ALL {
        assert_eq!(
            choice_mark(choice),
            object_mark(&choice.stock()).0,
            "{choice:?} is drawn twice and has to read the same both times"
        );
    }
}
