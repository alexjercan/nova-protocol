//! Reference checks over one start role: its hull resolves and its kit fits.

use nova_gameplay::prelude::GameItems;

use super::{ship::check_stock, KnownSections, KnownShipDesigns, LintIssue};
use crate::prelude::*;

/// Static well-formedness of one start role, run where the role catalog is
/// walked. Pure over the config, like [`lint_station_design`] beside it.
///
/// Errors:
/// - `hull` names a ship design `designs` does not hold;
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

#[cfg(test)]
mod tests {
    use nova_gameplay::prelude::ShipInventoryStock;

    use super::*;
    use crate::lint::LintSeverity;

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
