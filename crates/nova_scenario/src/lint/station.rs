//! Structural checks over one station design: a parked hull that can be
//! docked with, offers something, and holds only what fits.

use std::collections::HashSet;

use nova_gameplay::prelude::{kg_text, GameItems};
use nova_ship::prelude::SectionKind;

use super::{ship::check_design_sections, KnownSections, LintIssue};
use crate::prelude::*;

/// Static well-formedness of one CATALOG station design, run where the
/// catalog is walked so a design is checked once however many scenarios spawn
/// it. Pure over the config, like [`lint_ship_design_config`] beside it.
///
/// Errors:
/// - the hull's section list fails the ship design checks: an unknown
///   prototype, a reserved or duplicate section id, overlapping sections, a
///   broken link graph ([`check_design_sections`]);
/// - the hull has no Docking section (nothing could dock), any Thruster
///   section (a station never flies) or no Controller section (nothing holds
///   its attitude);
/// - the hull authors no `integrity.collapse_threshold` in `0..=1`: on a
///   station it is the floor its structure stops taking damage at;
/// - `services` is empty;
/// - `stock` names an item `items` does not hold, or weighs more than the
///   hull's hold;
/// - `storage_cap_g` is above zero without a Storage service, or zero with
///   one.
///
/// Warns on a service listed twice.
pub fn lint_station_design(
    design: &StationDesign,
    sections: &KnownSections,
    items: &GameItems,
    source: &str,
) -> Vec<LintIssue> {
    let mut issues = Vec::new();
    let id = design.id.as_str();
    check_design_sections(
        "station",
        id,
        &design.hull,
        source,
        sections,
        items,
        &mut issues,
    );

    let (resolved, _) = resolve_ship_design(
        &ShipDesignSource::Inline(design.hull.clone()),
        &GameShipDesigns::default(),
        sections.catalog(),
    );
    let has = |wanted: fn(&SectionKind) -> bool| {
        resolved
            .sections
            .iter()
            .any(|section| wanted(&section.config.kind))
    };
    if !has(|kind| matches!(kind, SectionKind::Docking(_))) {
        issues.push(LintIssue::error(
            source,
            format!("station '{id}': no Docking section; nothing could dock with it"),
        ));
    }
    for section in &resolved.sections {
        if matches!(section.config.kind, SectionKind::Thruster(_)) {
            issues.push(LintIssue::error(
                source,
                format!(
                    "station '{id}': section '{}' is a Thruster; a station never flies",
                    section.id
                ),
            ));
        }
    }
    if !has(|kind| matches!(kind, SectionKind::Controller(_))) {
        issues.push(LintIssue::error(
            source,
            format!("station '{id}': no Controller section; nothing would hold its attitude"),
        ));
    }

    match design.hull.integrity.collapse_threshold {
        Some(floor) if (0.0..=1.0).contains(&floor) => {}
        Some(floor) => issues.push(LintIssue::error(
            source,
            format!("station '{id}': integrity collapse_threshold {floor} is outside 0..=1"),
        )),
        None => issues.push(LintIssue::error(
            source,
            format!(
                "station '{id}': the hull authors no integrity collapse_threshold; on a \
                 station it is the damage floor"
            ),
        )),
    }

    if design.services.is_empty() {
        issues.push(LintIssue::error(
            source,
            format!("station '{id}': services is empty; a station offers at least one"),
        ));
    }
    let mut seen = HashSet::new();
    for service in &design.services {
        if !seen.insert(service) {
            issues.push(LintIssue::warn(
                source,
                format!("station '{id}': service {service:?} is listed more than once"),
            ));
        }
    }
    let offers_storage = design.services.contains(&StationServiceType::Storage);
    if design.storage_cap_g > 0 && !offers_storage {
        issues.push(LintIssue::error(
            source,
            format!(
                "station '{id}': storage_cap_g is {} but services lists no Storage",
                design.storage_cap_g
            ),
        ));
    }
    if design.storage_cap_g == 0 && offers_storage {
        issues.push(LintIssue::error(
            source,
            format!("station '{id}': services lists Storage but storage_cap_g is 0"),
        ));
    }

    let mut known_stock = true;
    for (item, _) in design.stock.stacks() {
        if items.get(item).is_none() {
            known_stock = false;
            issues.push(LintIssue::error(
                source,
                format!("station '{id}': unknown item '{item}'"),
            ));
        }
    }
    // The spawn fills the hold from this stock and panics when it does not
    // fit. An unknown item has no mass to weigh.
    if known_stock {
        let capacity_g = u64::from(resolved.cargo_capacity_g());
        let stock_g = design.stock.mass_g(items);
        if stock_g > capacity_g {
            issues.push(LintIssue::error(
                source,
                format!(
                    "station '{id}': stock of {} exceeds its {} hold",
                    kg_text(stock_g),
                    kg_text(capacity_g),
                ),
            ));
        }
    }
    issues
}

#[cfg(test)]
mod tests {
    use bevy::prelude::*;
    use nova_gameplay::prelude::{
        Allegiance, GameItems, ItemCategoryType, ItemDesign, ItemDesignId, ShipInventoryStock,
    };
    use nova_ship::prelude::{
        BaseSectionConfig, ControllerSectionConfig, DockingSectionConfig, HullSectionConfig,
        SectionConfig, SectionKind, ThrusterSectionConfig,
    };

    use super::*;
    use crate::lint::{KnownSections, LintSeverity};

    const DOCK: &str = "dock";
    const BRIDGE: &str = "bridge";
    const PLATE: &str = "plate";
    const DRIVE: &str = "drive";

    /// The four prototypes a station lint resolves against.
    fn sections() -> KnownSections {
        let section = |id: &str, kind: SectionKind| SectionConfig {
            base: BaseSectionConfig {
                id: id.to_string(),
                link_points: nova_ship::prelude::unit_cube_link_points(),
                ..default()
            },
            kind,
        };
        let configs = vec![
            section(DOCK, SectionKind::Docking(DockingSectionConfig::default())),
            section(
                BRIDGE,
                SectionKind::Controller(ControllerSectionConfig::default()),
            ),
            section(PLATE, SectionKind::Hull(HullSectionConfig::default())),
            section(
                DRIVE,
                SectionKind::Thruster(ThrusterSectionConfig::default()),
            ),
        ];
        KnownSections::from_configs(&configs)
    }

    fn items() -> GameItems {
        GameItems::new([ItemDesign {
            id: ItemDesignId::from("crate"),
            name: "Crate".to_string(),
            about: String::new(),
            category: ItemCategoryType::Parts,
            mass_g: 50_000,
            ask_cr: 1,
            bid_cr: 1,
        }])
    }

    fn placed(id: &str, prototype: &str, z: f32) -> SpaceshipSectionConfig {
        SpaceshipSectionConfig {
            id: id.to_string(),
            position: Vec3::new(0.0, 0.0, z),
            rotation: Quat::IDENTITY,
            source: SectionSource::prototype(prototype),
        }
    }

    /// A station that passes every rule: dock, bridge, two plates, Trade and
    /// Storage with a cap, one crate in a 200 kg hold.
    fn sound() -> StationDesign {
        StationDesign {
            id: "post".into(),
            name: "Post".to_string(),
            about: String::new(),
            hull: ShipDesign {
                sections: vec![
                    placed("port", DOCK, 0.0),
                    placed("bridge", BRIDGE, 1.0),
                    placed("plate_0", PLATE, 2.0),
                    placed("plate_1", PLATE, 3.0),
                ],
                integrity: ShipIntegrityConfig {
                    collapse_threshold: Some(0.5),
                },
                ..default()
            },
            services: vec![StationServiceType::Trade, StationServiceType::Storage],
            storage_cap_g: 100_000,
            stock: ShipInventoryStock::new([(ItemDesignId::from("crate"), 1)]),
            credits: 100,
        }
    }

    fn errors(design: &StationDesign) -> Vec<String> {
        lint_station_design(design, &sections(), &items(), "test")
            .into_iter()
            .filter(|issue| issue.severity == LintSeverity::Error)
            .map(|issue| issue.message)
            .collect()
    }

    #[test]
    fn a_sound_station_design_lints_clean() {
        assert!(
            lint_station_design(&sound(), &sections(), &items(), "test").is_empty(),
            "{:?}",
            errors(&sound())
        );
    }

    #[test]
    fn a_station_naming_an_unknown_section_id_is_refused() {
        let mut design = sound();
        design.hull.sections[2] = placed("plate_0", "no_such_plate", 2.0);
        let found = errors(&design);
        assert!(
            found
                .iter()
                .any(|message| message.contains("station 'post'")
                    && message.contains("no_such_plate")),
            "{found:?}"
        );
    }

    #[test]
    fn a_station_with_no_docking_section_is_refused() {
        let mut design = sound();
        design.hull.sections[0] = placed("port", PLATE, 0.0);
        let found = errors(&design);
        assert!(
            found
                .iter()
                .any(|message| message.contains("no Docking section")),
            "{found:?}"
        );
    }

    #[test]
    fn a_station_offering_no_service_is_refused() {
        let mut design = sound();
        design.services.clear();
        design.storage_cap_g = 0;
        let found = errors(&design);
        assert!(
            found
                .iter()
                .any(|message| message.contains("services is empty")),
            "{found:?}"
        );
    }

    #[test]
    fn a_station_with_a_thruster_or_no_controller_is_refused() {
        let mut design = sound();
        design.hull.sections[1] = placed("bridge", DRIVE, 1.0);
        let found = errors(&design);
        assert!(
            found
                .iter()
                .any(|message| message.contains("'bridge' is a Thruster")),
            "{found:?}"
        );
        assert!(
            found
                .iter()
                .any(|message| message.contains("no Controller section")),
            "{found:?}"
        );
    }

    #[test]
    fn a_station_with_no_damage_floor_is_refused() {
        let mut design = sound();
        design.hull.integrity.collapse_threshold = None;
        let found = errors(&design);
        assert!(
            found
                .iter()
                .any(|message| message.contains("no integrity collapse_threshold")),
            "{found:?}"
        );
        design.hull.integrity.collapse_threshold = Some(1.5);
        let found = errors(&design);
        assert!(
            found
                .iter()
                .any(|message| message.contains("outside 0..=1")),
            "{found:?}"
        );
    }

    #[test]
    fn a_storage_cap_disagreeing_with_the_storage_service_is_refused() {
        let mut capped = sound();
        capped.services = vec![StationServiceType::Trade];
        assert!(errors(&capped)
            .iter()
            .any(|message| message.contains("lists no Storage")),);
        let mut uncapped = sound();
        uncapped.storage_cap_g = 0;
        assert!(errors(&uncapped)
            .iter()
            .any(|message| message.contains("storage_cap_g is 0")),);
    }

    #[test]
    fn stock_the_hull_cannot_hold_or_nobody_defines_is_refused() {
        let mut heavy = sound();
        heavy.stock = ShipInventoryStock::new([(ItemDesignId::from("crate"), 5)]);
        assert!(
            errors(&heavy)
                .iter()
                .any(|message| message.contains("exceeds its 200 kg hold")),
            "{:?}",
            errors(&heavy)
        );
        let mut unknown = sound();
        unknown.stock = ShipInventoryStock::new([(ItemDesignId::from("ghost"), 1)]);
        assert!(errors(&unknown)
            .iter()
            .any(|message| message.contains("unknown item 'ghost'")),);
    }

    /// A spawn naming a station no bundle authored is an Error, the same
    /// reference class an unknown ship design is. A known id is clean; its
    /// hull is linted where the station catalog is walked, not here.
    #[test]
    fn a_station_spawn_naming_an_unknown_design_is_refused() {
        use crate::lint::fixtures::{errors, known, scenario, sections, ships, stations};
        let by_id = |design: &str| {
            EventActionConfig::SpawnScenarioObject(ScenarioObjectConfig {
                base: BaseScenarioObjectConfig {
                    id: "post".to_string(),
                    name: "Post".to_string(),
                    position: nova_events::prelude::Meters3::ZERO,
                    rotation: Quat::IDENTITY,
                },
                kind: ScenarioObjectKind::Station(StationConfig {
                    design: design.into(),
                    allegiance: Allegiance::Neutral,
                    derelict: false,
                    initial_velocity: nova_events::prelude::MetersPerSecond3::ZERO,
                }),
            })
        };
        let lint = |design: &str| {
            lint_scenario(
                &scenario(vec![by_id(design)], vec![]),
                &sections(&["hull"]),
                &ships(&[]),
                &stations(&["waystation"]),
                &known(&["test_scenario"]),
                &GameItems::default(),
            )
        };
        let unknown = lint("no_such_station");
        let found = errors(&unknown);
        assert_eq!(found.len(), 1, "{unknown:?}");
        assert!(found[0]
            .message
            .contains("unknown station design 'no_such_station'"));
        let clean = lint("waystation");
        assert!(errors(&clean).is_empty(), "{clean:?}");
    }

    #[test]
    fn a_service_listed_twice_warns_without_refusing() {
        let mut design = sound();
        design.services.push(StationServiceType::Trade);
        let issues = lint_station_design(&design, &sections(), &items(), "test");
        assert!(issues
            .iter()
            .all(|issue| issue.severity == LintSeverity::Warn));
        assert_eq!(issues.len(), 1, "{issues:?}");
    }
}
