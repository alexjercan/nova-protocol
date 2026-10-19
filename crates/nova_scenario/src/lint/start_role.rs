//! Reference checks over one start role: its hull resolves, carries a cargo
//! intake clear of its docking ports, its kit fits, and its goal is one a
//! save can meet and tell apart.

use nova_gameplay::prelude::GameItems;
use nova_ship::prelude::{intake_in_dock_approach, RefitPlacedSection, SectionKind, FLEET_SIDES};

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
///   than the hull's hold;
/// - the goal asks for a zero count, for more stations than jobs (a hand-in
///   serves one station), for more rings than posting stations or hulls (a
///   station stands in one ring, so the save cannot show more), for more
///   escorts than the fleet has slots, or names a flagship hull `designs`
///   does not hold.
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
    check_goal(id, &role.goal, designs, source, &mut issues);
    issues
}

/// Refuse a goal no save can meet or tell apart: a zero count, more stations
/// than jobs, more rings than posting stations or hulls, more escorts than
/// fleet slots, a flagship hull no visible design carries.
fn check_goal(
    id: &str,
    goal: &RoleGoal,
    designs: &KnownShipDesigns,
    source: &str,
    issues: &mut Vec<LintIssue>,
) {
    let mut refuse = |message: String| {
        issues.push(LintIssue::error(
            source,
            format!("start role '{id}': goal {message}"),
        ));
    };
    let mut above_zero = |count: u32, what: &str| {
        if count == 0 {
            refuse(format!("asks for 0 {what}"));
        }
    };
    match &goal.kind {
        RoleGoalType::StationNetwork {
            jobs,
            stations,
            rings,
        } => {
            above_zero(*jobs, "jobs");
            above_zero(*stations, "stations");
            above_zero(*rings, "rings");
            if stations > jobs {
                refuse(format!(
                    "asks for {stations} stations over {jobs} jobs; a hand-in serves one station"
                ));
            }
            if rings > stations {
                refuse(format!(
                    "asks for {rings} rings over {stations} stations; a station stands in one ring"
                ));
            }
        }
        RoleGoalType::KillList { hulls, rings } => {
            above_zero(*hulls, "hulls");
            above_zero(*rings, "rings");
            if rings > hulls {
                refuse(format!(
                    "asks for {rings} rings over {hulls} hulls; a hull is posted from one ring"
                ));
            }
        }
        RoleGoalType::Flagship { hull } => {
            if designs.get(hull).is_none() {
                refuse(format!("hull '{hull}' is no known ship design"));
            }
        }
        // No count to refuse: the base catalog, which every mod loads,
        // holds a recipe-bearing section of each part family.
        RoleGoalType::OwnYard => {}
        RoleGoalType::MiningEmpire { stations, escorts } => {
            above_zero(*stations, "stations");
            above_zero(*escorts, "escorts");
            let slots = FLEET_SIDES.len();
            if *escorts as usize > slots {
                refuse(format!(
                    "asks for {escorts} escorts over the fleet's {slots} slots"
                ));
            }
        }
        RoleGoalType::Outlaw { hulls } => above_zero(*hulls, "hulls"),
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
    use nova_gameplay::prelude::{JobMixType, RoleIconType, ShipInventoryStock};

    use super::*;
    use crate::lint::{
        fixtures::{sections, ships},
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
                kind: RoleGoalType::OwnYard,
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
                kind: RoleGoalType::OwnYard,
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

    /// A goal is refused by what the save cannot show: a zero count, more
    /// stations than the jobs that serve them, more rings than the posters
    /// that stand in them, more escorts than the fleet's slots, or a
    /// flagship hull no design carries. A sound goal of each kind passes.
    #[test]
    fn a_start_role_goal_with_a_zero_count_more_rings_than_posters_or_an_unknown_hull_is_refused() {
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
        assert!(goal_errors(RoleGoalType::StationNetwork {
            jobs: 8,
            stations: 4,
            rings: 2,
        })
        .is_empty());
        assert!(goal_errors(RoleGoalType::KillList { hulls: 6, rings: 2 }).is_empty());
        assert!(goal_errors(RoleGoalType::Flagship {
            hull: "picket".into(),
        })
        .is_empty());
        assert!(goal_errors(RoleGoalType::OwnYard).is_empty());
        assert!(goal_errors(RoleGoalType::Outlaw { hulls: 6 }).is_empty());
        let outlaw = goal_errors(RoleGoalType::Outlaw { hulls: 0 });
        assert!(
            outlaw.iter().any(|m| m.contains("asks for 0 hulls")),
            "{outlaw:?}"
        );
        assert!(goal_errors(RoleGoalType::MiningEmpire {
            stations: 2,
            escorts: 2,
        })
        .is_empty());

        let zero = goal_errors(RoleGoalType::StationNetwork {
            jobs: 0,
            stations: 4,
            rings: 2,
        });
        assert!(
            zero.iter().any(|m| m.contains("asks for 0 jobs")),
            "{zero:?}"
        );
        let stations = goal_errors(RoleGoalType::StationNetwork {
            jobs: 2,
            stations: 4,
            rings: 1,
        });
        assert!(
            stations
                .iter()
                .any(|m| m.contains("4 stations over 2 jobs")),
            "{stations:?}"
        );
        let rings = goal_errors(RoleGoalType::StationNetwork {
            jobs: 8,
            stations: 2,
            rings: 3,
        });
        assert!(
            rings.iter().any(|m| m.contains("3 rings over 2 stations")),
            "{rings:?}"
        );
        let hulls = goal_errors(RoleGoalType::KillList { hulls: 1, rings: 2 });
        assert!(
            hulls.iter().any(|m| m.contains("2 rings over 1 hulls")),
            "{hulls:?}"
        );
        let escorts = goal_errors(RoleGoalType::MiningEmpire {
            stations: 2,
            escorts: 3,
        });
        assert!(
            escorts
                .iter()
                .any(|m| m.contains("3 escorts over the fleet's 2 slots")),
            "{escorts:?}"
        );
        let unknown = goal_errors(RoleGoalType::Flagship {
            hull: "no_such_hull".into(),
        });
        assert!(
            unknown
                .iter()
                .any(|m| m.contains("hull 'no_such_hull' is no known ship design")),
            "{unknown:?}"
        );
    }
}
