//! Greebles on the real base hulls: every piece the greeble solve places on a
//! base ship or station stands on a plate whose top is one plane under the
//! piece's whole turned footprint, so no piece overhangs a chamfer.

use nova_authoring::generation::{
    build_section_catalog, build_ships, build_stations, build_styles,
};
use nova_gameplay::prelude::SectionClass;
use nova_scenario::prelude::{resolve_ship_design, GameShipDesigns, ShipDesign, ShipDesignSource};
use nova_ship::{
    prelude::{GameSections, PlacedPart, SectionFootprint},
    sections::{
        clearance::exit_normal,
        shell_skin::{derive_skin, read_structure},
        skin_decor::{decor_footprint, solve_decor},
    },
};

/// Every base hull that wears a style, by id, with its design.
fn styled_hulls() -> Vec<(String, ShipDesign)> {
    let ships = build_ships()
        .into_iter()
        .map(|ship| (ship.id.to_string(), ship.design));
    let stations = build_stations()
        .into_iter()
        .map(|station| (station.id.to_string(), station.design.hull));
    ships
        .chain(stations)
        .filter(|(_, design)| design.presentation.skin && design.presentation.style.is_some())
        .collect()
}

/// Every piece placed on every styled base hull stands on a plateau that
/// covers its turned collider footprint.
#[test]
fn every_base_greeble_stands_on_a_plateau_that_covers_its_footprint() {
    let sections = GameSections(build_section_catalog());
    let styles = build_styles();
    let hulls = styled_hulls();
    assert!(!hulls.is_empty(), "the base content has styled hulls");
    let mut placed = 0;
    let mut overhangs: Vec<String> = Vec::new();
    for (id, design) in &hulls {
        let style_id = design.presentation.style.as_deref().expect("styled");
        let style = styles
            .iter()
            .find(|style| style.id == style_id)
            .unwrap_or_else(|| panic!("{id}: style '{style_id}' is in the base catalog"));
        let (resolved, errors) = resolve_ship_design(
            &ShipDesignSource::Inline(design.clone()),
            &GameShipDesigns::default(),
            &sections,
        );
        assert!(errors.is_empty(), "{id}: {errors:?}");
        let parts: Vec<PlacedPart> = resolved
            .sections
            .iter()
            .map(|section| PlacedPart {
                position: section.position,
                rotation: section.rotation,
                link_points: section.config.base.link_points.as_slice(),
                footprint: *SectionFootprint::from_collider(
                    section.config.base.collider.unwrap_or_default(),
                ),
                exit: exit_normal(&section.config.kind),
                port: section.config.kind.class() == SectionClass::Docking,
            })
            .collect();
        let (structure, _, _) = read_structure(&parts);
        let mut plates = derive_skin(&structure);
        for piece in solve_decor(&structure, &mut plates, style) {
            placed += 1;
            let fixture = &style.fixtures[piece.fixture];
            let half = decor_footprint(fixture, piece.turns);
            let plate = &plates[piece.plate];
            if !plate.shape.seats(half) {
                overhangs.push(format!(
                    "{id}: {} ({} x {} cells) on `{}` at {}",
                    fixture.id,
                    2.0 * half.x,
                    2.0 * half.y,
                    plate.shape.id(),
                    plate.cell,
                ));
            }
        }
    }
    assert!(placed > 0, "the base hulls carry greebles");
    assert!(
        overhangs.is_empty(),
        "{} of {placed} pieces overhang their plate:\n{}",
        overhangs.len(),
        overhangs.join("\n"),
    );
}
