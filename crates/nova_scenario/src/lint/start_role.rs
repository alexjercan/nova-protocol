//! Reference checks over one start role: its hull resolves, carries a cargo
//! intake clear of its docking ports, and its kit fits.

use nova_gameplay::prelude::GameItems;
use nova_ship::prelude::{intake_in_dock_approach, RefitPlacedSection, SectionKind};

use super::{ship::check_stock, KnownSections, KnownShipDesigns, LintIssue};
use crate::prelude::*;

/// Static well-formedness of one start role, run where the role catalog is
/// walked. Pure over the config, like [`lint_station_design`] beside it.
///
/// Errors:
/// - `hull` names a ship design `designs` does not hold;
/// - the hull carries no cargo intake section: an open-world ship jettisons
///   and takes cargo in through one;
/// - an intake reaches a docking port's face, where a berthed hull parks
///   (see [`intake_in_dock_approach`]);
/// - the kit's `stock` names an item `items` does not hold, or weighs more
///   than the hull's hold.
pub fn lint_start_role(
    role: &StartRole,
    designs: &KnownShipDesigns,
    sections: &KnownSections,
    items: &GameItems,
    source: &str,
) -> Vec<LintIssue> {
    let mut issues = Vec::new();
    let id = role.id.as_str();
    let capacity_g = match designs.get(&role.hull) {
        Some(design) => {
            let (resolved, _) = resolve_ship_design(
                &ShipDesignSource::Inline(design.clone()),
                &GameShipDesigns::default(),
                sections.catalog(),
            );
            check_intake(id, role, &resolved, source, &mut issues);
            Some(resolved.cargo_capacity_g())
        }
        None => {
            issues.push(LintIssue::error(
                source,
                format!(
                    "start role '{id}': hull '{}' is no known ship design",
                    role.hull
                ),
            ));
            None
        }
    };
    check_stock(
        "start role",
        id,
        &role.kit.stock,
        capacity_g,
        items,
        source,
        &mut issues,
    );
    issues
}

/// Refuse a start hull with no cargo intake, or with an intake where a
/// berthed hull parks.
fn check_intake(
    id: &str,
    role: &StartRole,
    resolved: &ResolvedShipDesign,
    source: &str,
    issues: &mut Vec<LintIssue>,
) {
    let hull = &role.hull;
    let has_intake = resolved
        .sections
        .iter()
        .any(|section| matches!(section.config.kind, SectionKind::CargoIntake(_)));
    if !has_intake {
        issues.push(LintIssue::error(
            source,
            format!(
                "start role '{id}': hull '{hull}' has no cargo intake section; an open-world \
                 ship jettisons and takes cargo in through one"
            ),
        ));
        return;
    }
    let placed: Vec<RefitPlacedSection<'_>> = resolved
        .sections
        .iter()
        .map(|section| RefitPlacedSection {
            position: section.position,
            rotation: section.rotation,
            config: &section.config,
        })
        .collect();
    if let Some((intake, port)) = intake_in_dock_approach(&placed) {
        issues.push(LintIssue::error(
            source,
            format!(
                "start role '{id}': hull '{hull}' intake '{}' reaches docking port '{}''s \
                 face, where a berthed hull parks",
                resolved.sections[intake].id, resolved.sections[port].id
            ),
        ));
    }
}

#[cfg(test)]
mod tests {
    use nova_gameplay::prelude::{JobMixType, ShipInventoryStock};

    use super::*;
    use crate::lint::{
        fixtures::{sections, ships},
        LintSeverity,
    };

    fn role(hull: &str) -> StartRole {
        StartRole {
            id: "military".into(),
            name: "Military".to_string(),
            about: String::new(),
            hull: hull.into(),
            kit: StartKit {
                credits: 0,
                stock: ShipInventoryStock::default(),
            },
            jobs: JobMixType::Trade,
            lore: String::new(),
        }
    }

    /// Every open-world ship takes cargo in: a role hull of one plain hull
    /// section, with no intake, is refused by name.
    #[test]
    fn a_start_role_whose_hull_has_no_cargo_intake_is_refused() {
        let found = lint_start_role(
            &role("picket"),
            &ships(&["picket"]),
            &sections(&["hull"]),
            &GameItems::default(),
            "test",
        );
        assert!(
            found
                .iter()
                .any(|issue| issue.severity == LintSeverity::Error
                    && issue.message.contains("start role 'military'")
                    && issue.message.contains("'picket' has no cargo intake")),
            "{found:?}"
        );
    }

    #[test]
    fn a_start_role_naming_an_unknown_hull_is_refused() {
        let role = StartRole {
            id: "industrial".into(),
            name: "Industrial".to_string(),
            about: String::new(),
            hull: "no_such_hull".into(),
            kit: StartKit {
                credits: 0,
                stock: ShipInventoryStock::default(),
            },
            jobs: JobMixType::Trade,
            lore: String::new(),
        };
        let found = lint_start_role(
            &role,
            &KnownShipDesigns::default(),
            &KnownSections::default(),
            &GameItems::default(),
            "test",
        );
        assert!(
            found
                .iter()
                .any(|issue| issue.severity == LintSeverity::Error
                    && issue.message.contains("start role 'industrial'")
                    && issue.message.contains("'no_such_hull'")),
            "{found:?}"
        );
    }
}
