//! Reference checks over one start role: its hull resolves, carries a cargo
//! intake clear of its docking ports and advancement-0 parts only, its kit
//! fits, and its goal asks for a count above zero.

use std::collections::HashSet;

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
/// - a hull section's advancement is above 0 (named by prototype id and
///   value): the starting station builds advancement-0 parts only, so a lost
///   one there could never be rebuilt;
/// - the kit's `stock` names an item `items` does not hold, or weighs more
///   than the hull's hold;
/// - the goal asks for a zero count: no hand-in could ever move it.
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
            check_advancement(id, role, &resolved, source, &mut issues);
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
    check_goal(id, &role.goal, source, &mut issues);
    issues
}

/// Refuse a goal whose count is zero: it would read met before any
/// hand-in.
fn check_goal(id: &str, goal: &RoleGoal, source: &str, issues: &mut Vec<LintIssue>) {
    let (count, what) = match &goal.kind {
        RoleGoalType::OreDeliveries { jobs }
        | RoleGoalType::Recoveries { jobs }
        | RoleGoalType::Patrols { jobs } => (*jobs, "jobs"),
        RoleGoalType::StationNetwork { stations } => (*stations, "stations"),
        RoleGoalType::KillList { hulls } | RoleGoalType::Outlaw { hulls } => (*hulls, "hulls"),
    };
    if count == 0 {
        issues.push(LintIssue::error(
            source,
            format!("start role '{id}': goal asks for 0 {what}"),
        ));
    }
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
            working: true,
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

/// Refuse a start hull with a section above advancement 0: the starting
/// station builds the parts every civilization knows, and a section above 0
/// is not one of them, so a lost one could never be rebuilt there.
fn check_advancement(
    id: &str,
    role: &StartRole,
    resolved: &ResolvedShipDesign,
    source: &str,
    issues: &mut Vec<LintIssue>,
) {
    let hull = &role.hull;
    let mut seen = HashSet::new();
    for section in &resolved.sections {
        let a = section.config.base.advancement;
        let prototype = &section.config.base.id;
        if a > 0.0 && seen.insert(prototype.clone()) {
            issues.push(LintIssue::error(
                source,
                format!(
                    "start role '{id}': hull '{hull}' section '{prototype}' needs \
                     advancement {a}; a start hull carries advancement-0 parts only"
                ),
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use nova_gameplay::prelude::{JobMixType, RoleIconType, ShipInventoryStock};

    use super::*;
    use crate::lint::{
        fixtures::{fixture_recipe, sections, ships},
        LintSeverity,
    };

    fn role(hull: &str) -> StartRole {
        StartRole {
            id: "military".into(),
            name: "Military".to_string(),
            icon: RoleIconType::Shield,
            about: String::new(),
            hull: hull.into(),
            kit: StartKit {
                credits: 0,
                stock: ShipInventoryStock::default(),
            },
            jobs: JobMixType::Trade,
            goal: RoleGoal {
                title: "A goal".to_string(),
                kind: RoleGoalType::KillList { hulls: 6 },
            },
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
            icon: RoleIconType::Drill,
            about: String::new(),
            hull: "no_such_hull".into(),
            kit: StartKit {
                credits: 0,
                stock: ShipInventoryStock::default(),
            },
            jobs: JobMixType::Trade,
            goal: RoleGoal {
                title: "A goal".to_string(),
                kind: RoleGoalType::KillList { hulls: 6 },
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

    /// A goal of any kind is refused by a zero count, named by what it
    /// counts; a count above zero passes for every kind.
    #[test]
    fn a_start_role_goal_with_a_zero_count_is_refused() {
        let goal_errors = |kind: RoleGoalType| -> Vec<String> {
            let mut role = role("picket");
            role.goal = RoleGoal {
                title: "A goal".to_string(),
                kind,
            };
            lint_start_role(
                &role,
                &ships(&["picket"]),
                &sections(&["hull"]),
                &GameItems::default(),
                "test",
            )
            .into_iter()
            .filter(|issue| issue.severity == LintSeverity::Error && issue.message.contains("goal"))
            .map(|issue| issue.message)
            .collect()
        };
        let sound = [
            RoleGoalType::OreDeliveries { jobs: 6 },
            RoleGoalType::Recoveries { jobs: 6 },
            RoleGoalType::StationNetwork { stations: 4 },
            RoleGoalType::KillList { hulls: 6 },
            RoleGoalType::Patrols { jobs: 6 },
            RoleGoalType::Outlaw { hulls: 6 },
        ];
        for kind in sound {
            assert!(goal_errors(kind.clone()).is_empty(), "{kind:?}");
        }
        let zero = [
            (RoleGoalType::OreDeliveries { jobs: 0 }, "0 jobs"),
            (RoleGoalType::Recoveries { jobs: 0 }, "0 jobs"),
            (RoleGoalType::StationNetwork { stations: 0 }, "0 stations"),
            (RoleGoalType::KillList { hulls: 0 }, "0 hulls"),
            (RoleGoalType::Patrols { jobs: 0 }, "0 jobs"),
            (RoleGoalType::Outlaw { hulls: 0 }, "0 hulls"),
        ];
        for (kind, text) in zero {
            let errors = goal_errors(kind.clone());
            assert!(
                errors
                    .iter()
                    .any(|m| m.contains("start role 'military': goal asks for") && m.contains(text)),
                "{kind:?}: {errors:?}"
            );
        }
    }

    /// The starting station builds advancement-0 parts only: a hull section
    /// above advancement 0 is refused once per prototype, not once per cube.
    #[test]
    fn a_start_role_whose_hull_carries_a_part_above_advancement_zero_is_refused() {
        use bevy::prelude::{default, Quat, Vec3};
        use nova_ship::prelude::{BaseSectionConfig, HullSectionConfig, SectionConfig};

        let mut configs = sections(&["hull"]).catalog().0.clone();
        configs.push(SectionConfig {
            base: BaseSectionConfig {
                id: "advanced".to_string(),
                build: fixture_recipe(),
                advancement: 0.3,
                draw_weight: 1.0,
                ..default()
            },
            kind: SectionKind::Hull(HullSectionConfig::default()),
        });
        let known_sections = KnownSections::from_configs(&configs);

        let design = ShipDesign {
            sections: vec![
                SpaceshipSectionConfig {
                    id: "a".to_string(),
                    position: Vec3::ZERO,
                    rotation: Quat::IDENTITY,
                    source: SectionSource::prototype("advanced"),
                },
                SpaceshipSectionConfig {
                    id: "b".to_string(),
                    position: Vec3::new(0.0, 0.0, 1.0),
                    rotation: Quat::IDENTITY,
                    source: SectionSource::prototype("advanced"),
                },
            ],
            ..default()
        };
        let designs = KnownShipDesigns::from_configs(&[ShipDesignPrototype {
            id: "flagship".into(),
            name: "Flagship".to_string(),
            design,
        }]);

        let found = lint_start_role(
            &role("flagship"),
            &designs,
            &known_sections,
            &GameItems::default(),
            "test",
        );
        let advancement_errors: Vec<&str> = found
            .iter()
            .filter(|issue| {
                issue.severity == LintSeverity::Error
                    && issue.message.contains("needs advancement 0.3")
            })
            .map(|issue| issue.message.as_str())
            .collect();
        assert_eq!(advancement_errors.len(), 1, "{found:?}");
        assert!(advancement_errors[0].contains("'advanced'"), "{found:?}");
    }
}
