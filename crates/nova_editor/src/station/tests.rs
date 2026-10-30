//! The station design a marked ship writes, and what refuses it.

use nova_gameplay::prelude::{ItemCategoryType, ItemDesign};
use nova_scenario::prelude::{SectionSource, SpaceshipSectionConfig};
use nova_ship::prelude::{
    unit_cube_link_points, BaseSectionConfig, ControllerSectionConfig, DockingSectionConfig,
    HullSectionConfig, SectionConfig, SectionKind, ThrusterSectionConfig,
};

use super::*;

const DOCK: &str = "dock";
const BRIDGE: &str = "bridge";
const PLATE: &str = "plate";
const DRIVE: &str = "drive";

/// The four parts a sandbox station is built from here, each one cell.
fn sections() -> GameSections {
    let part = |id: &str, kind: SectionKind| SectionConfig {
        base: BaseSectionConfig {
            id: id.to_string(),
            link_points: unit_cube_link_points(),
            ..default()
        },
        kind,
    };
    GameSections(vec![
        part(DOCK, SectionKind::Docking(DockingSectionConfig::default())),
        part(
            BRIDGE,
            SectionKind::Controller(ControllerSectionConfig::default()),
        ),
        part(PLATE, SectionKind::Hull(HullSectionConfig::default())),
        part(
            DRIVE,
            SectionKind::Thruster(ThrusterSectionConfig::default()),
        ),
    ])
}

/// The three role goods, at 10 kg, 2 kg and 25 kg each.
fn items() -> GameItems {
    let item = |role: ItemRoleType, mass_g: u32| ItemDesign {
        id: ItemDesignId::from(role),
        name: role.id().to_string(),
        about: String::new(),
        category: ItemCategoryType::Parts,
        mass_g,
        price_lot: 1,
        ask_cr: 1,
        bid_cr: 1,
    };
    GameItems::new([
        item(ItemRoleType::HullPlate, 10_000),
        item(ItemRoleType::Rations, 2_000),
        item(ItemRoleType::SalvagedParts, 25_000),
    ])
}

/// A ship marked as a station, built from `parts` one cell apart down +Z.
fn marked(parts: &[&str]) -> LoweredShip {
    LoweredShip {
        id: "ship_2".to_string(),
        name: "Outpost".to_string(),
        sections: parts
            .iter()
            .enumerate()
            .map(|(cell, prototype)| SpaceshipSectionConfig {
                id: format!("{prototype}_{cell}"),
                position: Vec3::new(0.0, 0.0, cell as f32),
                rotation: Quat::IDENTITY,
                source: SectionSource::prototype(*prototype),
            })
            .collect(),
        station: true,
        ..default()
    }
}

/// A port, a computer and two plates make a station with the sandbox
/// defaults: the four dock services, the waystation's cap and credits, the
/// station collapse share, and the role goods cut to the 200 kg hold - eight
/// plates and twenty rations fit whole, three of ten salvage crates fit the
/// 80 kg left.
#[test]
fn a_marked_hull_writes_the_sandbox_station_defaults_cut_to_its_hold() {
    let ship = marked(&[DOCK, BRIDGE, PLATE, PLATE]);

    let design = station_design(&ship, HullForm::Inline, &sections(), &items());

    assert_eq!(design.id.as_str(), "editor_sandbox_ship_2");
    assert_eq!(
        station_design(&ship, HullForm::Prototype, &sections(), &items())
            .id
            .as_str(),
        "ship_2",
        "a save writes the design under the node's own id"
    );
    assert_eq!(design.name, "Outpost");
    assert_eq!(design.design.services, SERVICES.to_vec());
    assert_eq!(design.design.storage_cap_g, STORAGE_CAP_G);
    assert_eq!(design.design.credits, CREDITS);
    assert_eq!(
        design.design.hull.integrity.collapse_threshold,
        Some(STATION_COLLAPSE_THRESHOLD)
    );
    let stock: Vec<(String, u32)> = design
        .design
        .stock
        .stacks()
        .map(|(item, count)| (item.to_string(), count))
        .collect();
    assert_eq!(
        stock,
        [
            (ItemRoleType::HullPlate.id().to_string(), 8),
            (ItemRoleType::Rations.id().to_string(), 20),
            (ItemRoleType::SalvagedParts.id().to_string(), 3),
        ]
    );
    assert!(
        station_faults(&design, &sections(), &items(), &GameStyles::default()).is_empty(),
        "the station lint takes it"
    );
}

/// A hull with no plates has no hold, so it opens with nothing to sell
/// rather than with stock the spawn could not fit.
#[test]
fn a_marked_hull_with_no_hold_opens_with_no_stock() {
    let design = station_design(
        &marked(&[DOCK, BRIDGE]),
        HullForm::Inline,
        &sections(),
        &items(),
    );

    assert_eq!(design.design.stock.stacks().count(), 0);
    assert!(station_faults(&design, &sections(), &items(), &GameStyles::default()).is_empty());
}

/// The three ways a hull fails as a station are the station lint's, and the
/// check row reads the clause that names each.
#[test]
fn a_drive_a_missing_port_or_a_missing_computer_refuses_the_station() {
    let short = |parts: &[&str]| -> Vec<String> {
        let design = station_design(&marked(parts), HullForm::Inline, &sections(), &items());
        station_faults(&design, &sections(), &items(), &GameStyles::default())
            .iter()
            .map(|fault| short_fault(fault).to_string())
            .collect()
    };

    assert_eq!(
        short(&[DOCK, BRIDGE, PLATE, DRIVE]),
        ["section 'drive_3' is a Thruster"]
    );
    assert_eq!(short(&[BRIDGE, PLATE]), ["no Docking section"]);
    assert_eq!(short(&[DOCK, PLATE]), ["no Controller section"]);
}
