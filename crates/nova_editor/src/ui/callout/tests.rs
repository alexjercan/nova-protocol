//! Which of the callout's two lines a solve calls for, and what they say.

use nova_ship::prelude::{
    BaseSectionConfig, HullSectionConfig, LinkPoint, SectionConfig, SectionKind,
};

use super::*;
use crate::snap::{self, Refusal};

fn part(id: &str, sockets: &[&str]) -> SectionConfig {
    SectionConfig {
        base: BaseSectionConfig {
            id: id.to_string(),
            name: id.to_string(),
            link_points: sockets
                .iter()
                .map(|socket| LinkPoint {
                    id: (*socket).to_string(),
                    position: Vec3::ZERO,
                    normal: Vec3::X,
                })
                .collect(),
            ..default()
        },
        kind: SectionKind::Hull(HullSectionConfig::default()),
    }
}

fn placed(refusal: Option<Refusal>) -> Placement {
    Placement {
        prototype: "thruster".to_string(),
        target_section: Entity::PLACEHOLDER,
        solve: snap::Placement {
            transform: Transform::default(),
            source: 0,
            target: 1,
            refusal,
        },
    }
}

/// A mate names its two sockets on a line of its own. Two raw ids in the slot
/// an error uses make the eye re-read them to find out which is which.
#[test]
fn a_legal_mate_names_its_sockets_on_a_line_of_its_own() {
    let sections = GameSections(vec![part("thruster", &["mount"])]);
    let hull = part("hull", &["fore", "aft"]);

    let (refusal, mate) = callout_lines(&placed(None), &sections, Some(&hull));

    assert_eq!(refusal, None, "a legal pose has no fault to report");
    assert_eq!(mate.as_deref(), Some("mate  aft <- mount"));
}

/// A refusal is about a mate that is not going to happen, so it does not
/// describe one.
#[test]
fn a_refused_pose_reports_the_fault_and_no_mate() {
    let sections = GameSections(vec![part("thruster", &["mount"])]);
    let hull = part("hull", &["fore", "aft"]);

    let (refusal, mate) = callout_lines(&placed(Some(Refusal::Occupied)), &sections, Some(&hull));

    assert_eq!(refusal, Some(Refusal::Occupied.message()));
    assert_eq!(mate, None);
}

/// A socket the catalog cannot name reads empty rather than panicking: an
/// unloaded catalog is a frame, not a fault.
#[test]
fn an_unknown_part_names_no_socket() {
    let (_, mate) = callout_lines(&placed(None), &GameSections(Vec::new()), None);

    assert_eq!(mate.as_deref(), Some("mate   <- "));
}

/// A row is taken away rather than left holding the last thing it said.
#[test]
fn a_line_with_nothing_to_say_is_taken_away() {
    let mut text = Text::new("socket occupied");
    let mut row = Node::default();

    write_row(&mut text, &mut row, None);

    assert_eq!(row.display, Display::None);
}
