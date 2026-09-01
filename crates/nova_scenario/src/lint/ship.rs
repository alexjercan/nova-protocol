//! Structural checks over a scenario's ships and their section configs.

use bevy::prelude::Vec3;
use nova_ship::prelude::{
    derive_link_point_graph, ControllerSectionConfig, LinkPointGraphError, LinkPointRef,
    PlacedSectionLinkPoints, RailgunSectionConfig, SectionCollider, SectionConfig, SectionKind,
    SectionReloadConfig, TurretJoint, TurretSectionConfig,
};

use super::{KnownSections, KnownShips, LintIssue};
use crate::prelude::*;

/// Every reference a spawned (or scatter-template) ship makes must resolve: the
/// hull it names, the section prototypes that hull is built from, and the
/// sections its spawn-time modifications aim at.
///
/// A `Prototype` hull's own geometry is NOT re-checked here - it is linted where
/// the ship catalog is walked ([`lint_ship_config`]), the same rule a
/// `Prototype` section follows.
pub(super) fn check_object_prototypes(
    config: &ScenarioObjectConfig,
    scenario: &str,
    sections: &KnownSections,
    ships: &KnownShips,
    issues: &mut Vec<LintIssue>,
) {
    let ScenarioObjectKind::Spaceship(ship) = &config.kind else {
        return;
    };
    let hull = match &ship.hull {
        ShipSource::Inline(hull) => {
            check_hull_sections(config.base.id.as_str(), hull, scenario, sections, issues);
            hull
        }
        ShipSource::Prototype(id) => match ships.get(id) {
            Some(hull) => hull,
            None => {
                issues.push(LintIssue::error(
                    scenario,
                    format!("ship '{}': unknown ship '{id}'", config.base.id),
                ));
                return;
            }
        },
    };

    // A spawn override aimed at a section the hull does not carry does nothing
    // at all - a silent no-op is exactly what this lint exists to catch.
    for modification in &ship.modifications {
        if !hull
            .sections
            .iter()
            .any(|section| section.id == modification.section)
        {
            issues.push(LintIssue::error(
                scenario,
                format!(
                    "ship '{}': modification names section '{}', which this hull does not carry",
                    config.base.id, modification.section
                ),
            ));
        }
    }
}

/// Static checks over one hull's section list: every prototype resolves, the
/// sections do not interpenetrate, the link-point graph is sound, and every
/// inline section config is well-formed.
///
/// Run on an inline hull where the scenario spawns it, and on a catalog ship
/// where the ship catalog is walked - so a hull is checked exactly once,
/// wherever it is authored.
fn check_hull_sections(
    ship_id: &str,
    hull: &ShipHull,
    source: &str,
    sections: &KnownSections,
    issues: &mut Vec<LintIssue>,
) {
    for section in &hull.sections {
        if let SectionSource::Prototype(proto) = &section.source {
            if !sections.contains(proto) {
                issues.push(LintIssue::error(
                    source,
                    format!(
                        "ship '{ship_id}' section '{}': unknown section prototype '{proto}'",
                        section.id
                    ),
                ));
            }
        }
    }
    check_section_overlaps(ship_id, &hull.sections, source, sections, issues);
    check_link_point_graph(ship_id, &hull.sections, source, sections, issues);
    // Inline section configs authored directly (a Prototype ref resolves to a
    // catalog section, which is linted where the catalog is walked -
    // lint_bundle - so it is not re-linted here).
    for section in &hull.sections {
        if let SectionSource::Inline(inline) = &section.source {
            issues.extend(lint_section_config(inline, source));
        }
    }
}

/// Static well-formedness of one CATALOG ship: the same structural checks a
/// scenario's inline hull gets, run where the ship is authored so a hull
/// referenced by eleven scenarios is checked once. Pure over the config, like
/// [`lint_section_config`] beside it.
pub fn lint_ship_config(
    ship: &ShipConfig,
    sections: &KnownSections,
    source: &str,
) -> Vec<LintIssue> {
    let mut issues = Vec::new();
    check_hull_sections(ship.id.as_str(), &ship.hull, source, sections, &mut issues);
    issues
}

/// Static well-formedness of one section's config that the RON parser cannot
/// catch (a well-typed field can still be nonsense). Checks controller response,
/// weapon reload, and the turret joint tree. Pure over the config, so every
/// consumer - the author CLI's `lint`, the CI gate, the runtime merge - runs the SAME check on base +
/// mod section catalogs, and `lint_scenario` runs it on inline turret sections.
pub fn lint_section_config(config: &SectionConfig, source: &str) -> Vec<LintIssue> {
    let mut issues = Vec::new();
    match &config.kind {
        SectionKind::Controller(controller) => {
            check_controller_config(config.base.id.as_str(), controller, source, &mut issues);
        }
        SectionKind::Turret(turret) => {
            check_reload_config(
                config.base.id.as_str(),
                turret.ammo_capacity,
                turret.reload,
                source,
                &mut issues,
            );
            check_turret_tree(config.base.id.as_str(), turret, source, &mut issues);
        }
        SectionKind::Torpedo(torpedo) => {
            check_reload_config(
                config.base.id.as_str(),
                torpedo.ammo_capacity,
                torpedo.reload,
                source,
                &mut issues,
            );
        }
        SectionKind::Railgun(railgun) => {
            check_reload_config(
                config.base.id.as_str(),
                railgun.ammo_capacity,
                railgun.reload,
                source,
                &mut issues,
            );
            check_railgun_charge(config.base.id.as_str(), railgun, source, &mut issues);
        }
        _ => {}
    }
    check_link_point_config(config, source, &mut issues);
    issues
}

fn check_controller_config(
    section_id: &str,
    controller: &ControllerSectionConfig,
    source: &str,
    issues: &mut Vec<LintIssue>,
) {
    if !controller.has_valid_steering_lag() {
        issues.push(LintIssue::error(
            source,
            format!(
                "section '{section_id}': controller steering_lag must be a positive, finite, \
                 computable number of seconds, got {}",
                controller.steering_lag
            ),
        ));
    }
}

fn check_reload_config(
    section_id: &str,
    capacity: Option<u32>,
    reload: Option<SectionReloadConfig>,
    source: &str,
    issues: &mut Vec<LintIssue>,
) {
    let Some(reload) = reload else { return };
    if capacity.is_none_or(|capacity| capacity == 0) {
        issues.push(LintIssue::error(
            source,
            format!("section '{section_id}': reload requires a positive ammo_capacity"),
        ));
    }
    if reload.delay <= 0.0 || !reload.delay.is_finite() {
        issues.push(LintIssue::error(
            source,
            format!(
                "section '{section_id}': reload delay must be positive and finite, got {}",
                reload.delay
            ),
        ));
    }
    if reload.amount == 0 {
        issues.push(LintIssue::error(
            source,
            format!("section '{section_id}': reload amount must be greater than zero"),
        ));
    }
}

/// Flag a lance whose charge clock cannot run. A non-finite or negative
/// `charge_seconds` divides the cue's progress by nonsense; zero is legal and
/// means an instant commit, which is a design choice rather than a mistake.
fn check_railgun_charge(
    section_id: &str,
    config: &RailgunSectionConfig,
    source: &str,
    issues: &mut Vec<LintIssue>,
) {
    if config.charge_seconds < 0.0 || !config.charge_seconds.is_finite() {
        issues.push(LintIssue::error(
            source,
            format!(
                "section '{section_id}': railgun charge_seconds must be a finite, non-negative \
                 number of seconds, got {}",
                config.charge_seconds
            ),
        ));
    }
}

/// Walk a turret's joint tree and flag authoring mistakes the parser accepts but

/// the runtime cannot use: a hinge with a degenerate (zero or non-finite) axis
/// or a non-positive traverse speed can never aim, min > max locks the hinge
/// shut, a non-positive `fire_rate` used to panic the spawn outright (the
/// runtime now clamps it, so this is the early, named report), and a tree with
/// no muzzle can never fire (the spawn observer rejects it at runtime). Cheap:
/// one DFS. `min`/`max`/a non-default `speed` on a FIXED
/// node (no `axis`) is a soft warning - harmless (the runtime ignores them) but
/// usually a forgotten `axis`.
fn check_turret_tree(
    section_id: &str,
    config: &TurretSectionConfig,
    source: &str,
    issues: &mut Vec<LintIssue>,
) {
    fn walk(
        section_id: &str,
        joint: &TurretJoint,
        source: &str,
        issues: &mut Vec<LintIssue>,
    ) -> usize {
        let mut muzzles = usize::from(joint.muzzle.is_some());
        if let Some(muzzle) = &joint.muzzle {
            if !muzzle.fire_rate.is_finite() || muzzle.fire_rate <= 0.0 {
                issues.push(LintIssue::error(
                    source,
                    format!(
                        "section '{section_id}': turret muzzle fire_rate must be a positive, \
                         finite number of shots/s, got {}",
                        muzzle.fire_rate
                    ),
                ));
            }
        }
        match joint.axis {
            Some(axis) => {
                if !axis.is_finite() || axis.length_squared() < 1e-12 {
                    issues.push(LintIssue::error(
                        source,
                        format!(
                            "section '{section_id}': turret joint has a degenerate hinge axis \
                             {axis:?} - a hinge axis must be a non-zero, finite vector"
                        ),
                    ));
                }
                if !joint.speed.is_finite() || joint.speed <= 0.0 {
                    issues.push(LintIssue::error(
                        source,
                        format!(
                            "section '{section_id}': turret hinge speed must be a positive, \
                             finite number of rad/s, got {}",
                            joint.speed
                        ),
                    ));
                }
                if let (Some(min), Some(max)) = (joint.min, joint.max) {
                    if min > max {
                        issues.push(LintIssue::error(
                            source,
                            format!(
                                "section '{section_id}': turret hinge min {min} exceeds max {max} \
                                 - the hinge is locked shut"
                            ),
                        ));
                    }
                }
            }
            None => {
                if joint.min.is_some() || joint.max.is_some() {
                    issues.push(LintIssue::warn(
                        source,
                        format!(
                            "section '{section_id}': turret joint sets rotation limits but has no \
                             `axis`, so it never rotates - did you forget the hinge axis?"
                        ),
                    ));
                }
            }
        }
        for child in &joint.children {
            muzzles += walk(section_id, child, source, issues);
        }
        muzzles
    }

    let muzzles = walk(section_id, &config.root, source, issues);
    if muzzles == 0 {
        issues.push(LintIssue::error(
            source,
            format!(
                "section '{section_id}': turret has no muzzle joint - it can never fire \
                 (add a `muzzle:` to a leaf joint)"
            ),
        ));
    }
}

fn check_link_point_config(config: &SectionConfig, source: &str, issues: &mut Vec<LintIssue>) {
    let placed = [PlacedSectionLinkPoints {
        position: Vec3::ZERO,
        rotation: bevy::prelude::Quat::IDENTITY,
        link_points: &config.base.link_points,
    }];
    let Err(errors) = derive_link_point_graph(&placed) else {
        return;
    };
    for error in errors {
        let message = match error {
            LinkPointGraphError::EmptyLinkPointId { link_point } => format!(
                "section '{}': link point {} has an empty id",
                config.base.id, link_point.link_point_index
            ),
            LinkPointGraphError::DuplicateLinkPointId { first, duplicate } => format!(
                "section '{}': link points {} and {} use duplicate id '{}'",
                config.base.id,
                first.link_point_index,
                duplicate.link_point_index,
                config.base.link_points[duplicate.link_point_index].id
            ),
            LinkPointGraphError::NonFiniteLinkPointPosition { link_point } => format!(
                "section '{}': link point '{}' has a non-finite position {:?}",
                config.base.id,
                link_point_name(&config.base.link_points, link_point),
                config.base.link_points[link_point.link_point_index].position
            ),
            LinkPointGraphError::NonFiniteLinkPointNormal { link_point } => format!(
                "section '{}': link point '{}' has a non-finite normal {:?}",
                config.base.id,
                link_point_name(&config.base.link_points, link_point),
                config.base.link_points[link_point.link_point_index].normal
            ),
            LinkPointGraphError::ZeroLinkPointNormal { link_point } => format!(
                "section '{}': link point '{}' has a zero normal",
                config.base.id,
                link_point_name(&config.base.link_points, link_point)
            ),
            LinkPointGraphError::NonUnitLinkPointNormal { link_point } => format!(
                "section '{}': link point '{}' normal {:?} must have unit length",
                config.base.id,
                link_point_name(&config.base.link_points, link_point),
                config.base.link_points[link_point.link_point_index].normal
            ),
            _ => continue,
        };
        issues.push(LintIssue::error(source, message));
    }
}

fn link_point_name(points: &[nova_ship::prelude::LinkPoint], reference: LinkPointRef) -> &str {
    points
        .get(reference.link_point_index)
        .map(|point| point.id.as_str())
        .unwrap_or("<invalid>")
}

fn check_link_point_graph(
    ship_id: &str,
    ship_sections: &[SpaceshipSectionConfig],
    scenario: &str,
    sections: &KnownSections,
    issues: &mut Vec<LintIssue>,
) {
    let resolved: Option<Vec<_>> = ship_sections
        .iter()
        .map(|section| match &section.source {
            SectionSource::Inline(config) => Some(config.base.link_points.as_slice()),
            SectionSource::Prototype(id) => {
                sections.get(id).map(|known| known.link_points.as_slice())
            }
        })
        .collect();
    let Some(resolved) = resolved else {
        return;
    };
    let placed: Vec<_> = ship_sections
        .iter()
        .zip(resolved)
        .map(|(section, link_points)| PlacedSectionLinkPoints {
            position: section.position,
            rotation: section.rotation,
            link_points,
        })
        .collect();
    let Err(errors) = derive_link_point_graph(&placed) else {
        return;
    };

    for error in errors {
        let message = match error {
            LinkPointGraphError::NonFiniteSectionPosition { section_index } => format!(
                "ship '{ship_id}' section '{}': position {:?} must be finite",
                ship_sections[section_index].id, ship_sections[section_index].position
            ),
            LinkPointGraphError::NonFiniteSectionRotation { section_index } => format!(
                "ship '{ship_id}' section '{}': rotation {:?} must be finite",
                ship_sections[section_index].id, ship_sections[section_index].rotation
            ),
            LinkPointGraphError::NonUnitSectionRotation { section_index } => format!(
                "ship '{ship_id}' section '{}': rotation {:?} must have unit length",
                ship_sections[section_index].id, ship_sections[section_index].rotation
            ),
            LinkPointGraphError::AmbiguousMate {
                link_point,
                candidates,
            } => {
                let point = &resolved_link_point(ship_sections, sections, link_point);
                let candidates = candidates
                    .into_iter()
                    .map(|candidate| {
                        let candidate_point =
                            resolved_link_point(ship_sections, sections, candidate);
                        format!(
                            "{}.{}",
                            ship_sections[candidate.section_index].id, candidate_point.id
                        )
                    })
                    .collect::<Vec<_>>()
                    .join(", ");
                format!(
                    "ship '{ship_id}' section '{}' link point '{}': ambiguous mates [{candidates}]",
                    ship_sections[link_point.section_index].id, point.id
                )
            }
            LinkPointGraphError::Disconnected { components } => {
                let components = components
                    .into_iter()
                    .map(|component| {
                        component
                            .into_iter()
                            .map(|index| ship_sections[index].id.as_str())
                            .collect::<Vec<_>>()
                            .join(", ")
                    })
                    .collect::<Vec<_>>()
                    .join(" | ");
                format!(
                    "ship '{ship_id}': link-point graph is disconnected; components: [{components}]"
                )
            }
            // Section configs are linted independently, so do not duplicate local findings here.
            _ => continue,
        };
        issues.push(LintIssue::error(scenario, message));
    }
}

fn resolved_link_point<'a>(
    ship_sections: &'a [SpaceshipSectionConfig],
    sections: &'a KnownSections,
    reference: LinkPointRef,
) -> &'a nova_ship::prelude::LinkPoint {
    let section = &ship_sections[reference.section_index];
    let points = match &section.source {
        SectionSource::Inline(config) => &config.base.link_points,
        SectionSource::Prototype(id) => &sections.get(id).expect("prototype resolved").link_points,
    };
    &points[reference.link_point_index]
}

/// Reject collider AABB interpenetration unless the two sections directly mate.
///
/// Tight primitive colliders conservatively overlap where semantic meshes interlock. An
/// authored mate makes that interface intentional. Unmated overlap still catches accidental
/// duplicate or embedded parts. Rotated AABBs make this a broad-phase authoring check rather
/// than physical narrow-phase geometry.
const OVERLAP_EPSILON: f32 = 1e-3;

fn check_section_overlaps(
    ship_id: &str,
    ship_sections: &[SpaceshipSectionConfig],
    scenario: &str,
    sections: &KnownSections,
    issues: &mut Vec<LintIssue>,
) {
    fn resolved<'a>(
        section: &'a SpaceshipSectionConfig,
        sections: &'a KnownSections,
    ) -> Option<(SectionCollider, &'a [nova_ship::prelude::LinkPoint])> {
        match &section.source {
            SectionSource::Inline(config) => Some((
                config.base.collider.unwrap_or_default(),
                &config.base.link_points,
            )),
            SectionSource::Prototype(id) => sections
                .get(id)
                .map(|known| (known.collider, known.link_points.as_slice())),
        }
    }

    let resolved: Option<Vec<_>> = ship_sections
        .iter()
        .map(|section| resolved(section, sections))
        .collect();
    let Some(resolved) = resolved else {
        return;
    };
    let placed: Vec<_> = ship_sections
        .iter()
        .zip(&resolved)
        .map(|(section, (_, points))| PlacedSectionLinkPoints {
            position: section.position,
            rotation: section.rotation,
            link_points: points,
        })
        .collect();
    let direct_mates = derive_link_point_graph(&placed)
        .unwrap_or_default()
        .into_iter()
        .map(|mate| {
            let a = mate.a.section_index.min(mate.b.section_index);
            let b = mate.a.section_index.max(mate.b.section_index);
            (a, b)
        })
        .collect::<std::collections::BTreeSet<_>>();

    for i in 0..ship_sections.len() {
        for j in (i + 1)..ship_sections.len() {
            let (a, b) = (&ship_sections[i], &ship_sections[j]);
            let d = a.position - b.position;
            let sum = resolved[i].0.rotated_aabb_half_extents(a.rotation)
                + resolved[j].0.rotated_aabb_half_extents(b.rotation);
            if d.x.abs() + OVERLAP_EPSILON < sum.x
                && d.y.abs() + OVERLAP_EPSILON < sum.y
                && d.z.abs() + OVERLAP_EPSILON < sum.z
                && !direct_mates.contains(&(i, j))
            {
                issues.push(LintIssue::error(
                    scenario,
                    format!(
                        "ship '{ship_id}': unmated sections '{}' at {:?} and '{}' at {:?} \
                         overlap (collider boxes interpenetrate: centers must be >= {:?} apart \
                         on some axis, or the sections must directly mate)",
                        a.id, a.position, b.id, b.position, sum
                    ),
                ));
            }
        }
    }
}

#[cfg(test)]
mod tests {

    use bevy::prelude::*;

    use super::*;
    use crate::lint::fixtures::*;

    #[test]
    fn unknown_prototype_is_an_error() {
        let s = scenario(vec![spawn_ship("player", "no_such_proto")], vec![]);
        let issues = lint_scenario(
            &s,
            &sections(&["known_proto"]),
            &ships(&[]),
            &known(&["test_scenario"]),
        );
        let errs = errors(&issues);
        assert_eq!(errs.len(), 1, "{issues:?}");
        assert!(errs[0].message.contains("no_such_proto"));
    }

    /// A spawn naming a ship no bundle authored is an Error, exactly like a
    /// section prototype that resolves to nothing - the reference class this
    /// lint exists for. A known id is clean, and its geometry is NOT re-linted
    /// here (it is checked where the ship catalog is walked).
    #[test]
    fn an_unknown_ship_reference_is_an_error() {
        let by_id = |ship: &str| {
            EventActionConfig::SpawnScenarioObject(ScenarioObjectConfig {
                base: BaseScenarioObjectConfig {
                    id: "raider".to_string(),
                    name: "Raider".to_string(),
                    position: Vec3::ZERO,
                    rotation: Quat::IDENTITY,
                },
                kind: ScenarioObjectKind::Spaceship(SpaceshipConfig {
                    hull: ShipSource::Prototype(ship.to_string()),
                    ..default()
                }),
            })
        };

        let s = scenario(vec![by_id("no_such_ship")], vec![]);
        let issues = lint_scenario(
            &s,
            &sections(&["hull"]),
            &ships(&["cargoa"]),
            &known(&["test_scenario"]),
        );
        let errs = errors(&issues);
        assert_eq!(errs.len(), 1, "{issues:?}");
        assert!(errs[0].message.contains("no_such_ship"));

        let s = scenario(vec![by_id("cargoa")], vec![]);
        let issues = lint_scenario(
            &s,
            &sections(&["hull"]),
            &ships(&["cargoa"]),
            &known(&["test_scenario"]),
        );
        assert!(issues.is_empty(), "a known ship lints clean: {issues:?}");
    }

    /// A spawn override aimed at a section the hull does not carry is a silent
    /// no-op at runtime, so it is an Error here.
    #[test]
    fn a_modification_naming_no_section_of_the_hull_is_an_error() {
        let with_override = |section: &str| {
            EventActionConfig::SpawnScenarioObject(ScenarioObjectConfig {
                base: BaseScenarioObjectConfig {
                    id: "duelist".to_string(),
                    name: "Duelist".to_string(),
                    position: Vec3::ZERO,
                    rotation: Quat::IDENTITY,
                },
                kind: ScenarioObjectKind::Spaceship(SpaceshipConfig {
                    hull: ShipSource::Prototype("cargoa".to_string()),
                    modifications: vec![ShipSectionModification {
                        section: section.to_string(),
                        modifications: vec![SectionModification::SetHealth(500.0)],
                    }],
                    ..default()
                }),
            })
        };

        let s = scenario(vec![with_override("no_such_section")], vec![]);
        let issues = lint_scenario(
            &s,
            &sections(&["hull"]),
            &ships(&["cargoa"]),
            &known(&["test_scenario"]),
        );
        let errs = errors(&issues);
        assert_eq!(errs.len(), 1, "{issues:?}");
        assert!(errs[0].message.contains("no_such_section"));

        // The fixture hull's one section is named `hull`.
        let s = scenario(vec![with_override("hull")], vec![]);
        assert!(lint_scenario(
            &s,
            &sections(&["hull"]),
            &ships(&["cargoa"]),
            &known(&["test_scenario"])
        )
        .is_empty());
    }

    /// A CATALOG ship is linted where it is authored: its own section
    /// prototypes must resolve. This is what keeps a hull checked once eleven
    /// scenarios have stopped inlining it.
    #[test]
    fn a_catalog_ship_is_linted_where_it_is_authored() {
        let ship = |proto: &str| ShipConfig {
            id: "cargoa".to_string(),
            name: "CargoA".to_string(),
            hull: ShipHull {
                sections: vec![SpaceshipSectionConfig {
                    id: "fuselage".to_string(),
                    position: Vec3::ZERO,
                    rotation: Quat::IDENTITY,
                    source: SectionSource::Prototype(proto.to_string()),
                    modifications: vec![],
                }],
                ..default()
            },
        };

        let issues = lint_ship_config(&ship("no_such_proto"), &sections(&["hull"]), "base");
        assert_eq!(issues.len(), 1, "{issues:?}");
        assert!(issues[0].message.contains("no_such_proto"));

        assert!(lint_ship_config(&ship("hull"), &sections(&["hull"]), "base").is_empty());
    }

    #[test]
    fn controller_steering_lag_must_be_positive_finite_and_computable() {
        use nova_ship::prelude::BaseSectionConfig;

        let controller = |steering_lag| SectionConfig {
            base: BaseSectionConfig {
                id: "computer".to_string(),
                ..default()
            },
            kind: SectionKind::Controller(ControllerSectionConfig {
                steering_lag,
                ..default()
            }),
        };

        for invalid in [0.0, -0.5, f32::NAN, f32::INFINITY, f32::MIN_POSITIVE] {
            let issues = lint_section_config(&controller(invalid), "mod");
            assert_eq!(errors(&issues).len(), 1, "{invalid}: {issues:?}");
            assert!(issues[0].message.contains("steering_lag"));
        }
        assert!(lint_section_config(&controller(0.0001), "mod").is_empty());
        assert!(lint_section_config(&controller(0.5), "mod").is_empty());
    }

    #[test]
    fn reload_requires_a_valid_magazine_delay_and_amount() {
        let reload = |delay, amount| SectionReloadConfig { delay, amount };
        let check = |capacity, reload| {
            let mut issues = Vec::new();
            check_reload_config("weapon", capacity, Some(reload), "mod", &mut issues);
            issues
        };

        assert!(check(Some(500), reload(3.0, 200)).is_empty());
        for issues in [
            check(None, reload(3.0, 200)),
            check(Some(0), reload(3.0, 200)),
            check(Some(500), reload(0.0, 200)),
            check(Some(500), reload(f32::NAN, 200)),
            check(Some(500), reload(3.0, 0)),
        ] {
            assert_eq!(errors(&issues).len(), 1, "{issues:?}");
        }
    }

    #[test]
    fn malformed_link_points_and_disconnected_ships_are_errors() {
        use nova_ship::prelude::{BaseSectionConfig, HullSectionConfig, LinkPoint};

        let malformed = SectionConfig {
            base: BaseSectionConfig {
                id: "bad".to_string(),
                link_points: vec![
                    LinkPoint {
                        id: "dup".to_string(),
                        position: Vec3::ZERO,
                        normal: Vec3::X,
                    },
                    LinkPoint {
                        id: "dup".to_string(),
                        position: Vec3::X,
                        normal: Vec3::ZERO,
                    },
                ],
                ..default()
            },
            kind: SectionKind::Hull(HullSectionConfig::default()),
        };
        let issues = lint_section_config(&malformed, "mod");
        assert!(
            issues
                .iter()
                .any(|issue| issue.message.contains("duplicate id")),
            "{issues:?}"
        );
        assert!(
            issues
                .iter()
                .any(|issue| issue.message.contains("zero normal")),
            "{issues:?}"
        );

        let no_points = SectionConfig {
            base: BaseSectionConfig {
                id: "empty".to_string(),
                ..default()
            },
            kind: SectionKind::Hull(HullSectionConfig::default()),
        };
        let catalog = KnownSections::from_configs([&no_points]);
        let mut action = spawn_ship("ship", "empty");
        let EventActionConfig::SpawnScenarioObject(object) = &mut action else {
            unreachable!()
        };
        let ScenarioObjectKind::Spaceship(ship) = &mut object.kind else {
            unreachable!()
        };
        let ShipSource::Inline(hull) = &mut ship.hull else {
            unreachable!()
        };
        hull.sections.push(SpaceshipSectionConfig {
            id: "second".to_string(),
            position: Vec3::X,
            rotation: Quat::IDENTITY,
            source: SectionSource::Prototype("empty".to_string()),
            modifications: Vec::new(),
        });
        let scenario = scenario(vec![action], vec![]);
        let issues = lint_scenario(&scenario, &catalog, &ships(&[]), &known(&["test_scenario"]));
        assert!(
            issues
                .iter()
                .any(|issue| issue.message.contains("disconnected")),
            "{issues:?}"
        );
    }

    /// A scatter TEMPLATE ship with a bad prototype must flag like a directly
    /// spawned one.
    #[test]
    fn unknown_prototype_in_a_scatter_template_is_an_error() {
        let template = match spawn_ship("swarm_", "no_such_proto") {
            EventActionConfig::SpawnScenarioObject(config) => config,
            _ => unreachable!(),
        };
        let s = scenario(
            vec![EventActionConfig::ScatterObjects(ScatterObjectsConfig {
                id_prefix: "swarm_".to_string(),
                count: 2,
                seed: 1,
                region: ScatterRegion::Ring {
                    center: Vec3::ZERO,
                    inner: 10.0,
                    outer: 20.0,
                    y_min: -1.0,
                    y_max: 1.0,
                },
                template,
                asteroid_radius: None,
                min_separation: None,
            })],
            vec![],
        );
        let issues = lint_scenario(
            &s,
            &sections(&["known_proto"]),
            &ships(&[]),
            &known(&["test_scenario"]),
        );
        let errs = errors(&issues);
        assert_eq!(errs.len(), 1, "{issues:?}");
        assert!(errs[0].message.contains("no_such_proto"));
    }
    /// Section overlaps: strictly-inside-the-cube errors; flush spine/side
    /// mounts pass (the fail-first is a half-embedded spine tube at z 0.5,
    /// the shape this check was born from).
    #[test]
    fn overlapping_sections_error_and_flush_sections_pass() {
        let ship_with = |tube_pos: Vec3| {
            EventActionConfig::SpawnScenarioObject(ScenarioObjectConfig {
                base: BaseScenarioObjectConfig {
                    id: "ship".to_string(),
                    name: "ship".to_string(),
                    position: Vec3::ZERO,
                    rotation: Quat::IDENTITY,
                },
                kind: ScenarioObjectKind::Spaceship(SpaceshipConfig {
                    hull: ShipSource::Inline(ShipHull {
                        sections: vec![
                            SpaceshipSectionConfig {
                                id: "a".to_string(),
                                position: Vec3::ZERO,
                                rotation: Quat::IDENTITY,
                                source: SectionSource::Prototype("known".to_string()),
                                modifications: vec![],
                            },
                            SpaceshipSectionConfig {
                                id: "b".to_string(),
                                position: tube_pos,
                                rotation: Quat::IDENTITY,
                                source: SectionSource::Prototype("known".to_string()),
                                modifications: vec![],
                            },
                        ],
                        ..default()
                    }),
                    ..default()
                }),
            })
        };

        // The failing shape: half-embedded on the spine.
        let s = scenario(vec![ship_with(Vec3::new(0.0, 0.0, 0.5))], vec![]);
        let issues = lint_scenario(
            &s,
            &sections(&["known"]),
            &ships(&[]),
            &known(&["test_scenario"]),
        );
        assert_eq!(
            issues
                .iter()
                .filter(|issue| issue.message.contains("overlap"))
                .count(),
            1,
            "{issues:?}"
        );

        // Flush side mount: legal.
        let s = scenario(vec![ship_with(Vec3::new(1.0, 0.0, 0.0))], vec![]);
        let issues = lint_scenario(
            &s,
            &sections(&["known"]),
            &ships(&[]),
            &known(&["test_scenario"]),
        );
        assert!(issues.is_empty(), "{issues:?}");
    }

    /// Authorable colliders move the overlap threshold: two sections 0.8 apart
    /// clip as default unit cubes but sit flush once both tighten to a 0.8
    /// cube, and oversized colliders clip where unit cubes would not. Only
    /// INLINE colliders are resolved; prototypes fall back to the unit cube.
    #[test]
    fn overlap_uses_authored_collider_half_extents() {
        use nova_ship::prelude::{BaseSectionConfig, HullSectionConfig};

        // An inline hull section at `pos` with the given collider.
        let inline =
            |id: &str, pos: Vec3, collider: Option<SectionCollider>| SpaceshipSectionConfig {
                id: id.to_string(),
                position: pos,
                rotation: Quat::IDENTITY,
                source: SectionSource::Inline(SectionConfig {
                    base: BaseSectionConfig {
                        collider,
                        link_points: nova_ship::prelude::unit_cube_link_points(),
                        ..Default::default()
                    },
                    kind: SectionKind::Hull(HullSectionConfig {
                        render_mesh: None,
                        render_mesh_transform: None,
                    }),
                }),
                modifications: vec![],
            };

        let ship = |a: SpaceshipSectionConfig, b: SpaceshipSectionConfig| {
            EventActionConfig::SpawnScenarioObject(ScenarioObjectConfig {
                base: BaseScenarioObjectConfig {
                    id: "ship".to_string(),
                    name: "ship".to_string(),
                    position: Vec3::ZERO,
                    rotation: Quat::IDENTITY,
                },
                kind: ScenarioObjectKind::Spaceship(SpaceshipConfig {
                    hull: ShipSource::Inline(ShipHull {
                        sections: vec![a, b],
                        ..default()
                    }),
                    ..default()
                }),
            })
        };

        let cube = |n: f32| {
            Some(SectionCollider::Cuboid {
                size: Vec3::splat(n),
            })
        };
        let x = |n: f32| Vec3::new(n, 0.0, 0.0);

        // 0.8 apart, default unit cubes: half-extents sum to 1.0 > 0.8 -> overlap.
        let s = scenario(
            vec![ship(
                inline("a", Vec3::ZERO, None),
                inline("b", x(0.8), None),
            )],
            vec![],
        );
        let issues = lint_scenario(&s, &sections(&[]), &ships(&[]), &known(&["test_scenario"]));
        assert_eq!(
            issues
                .iter()
                .filter(|issue| issue.message.contains("overlap"))
                .count(),
            1,
            "unit cubes should clip: {issues:?}"
        );

        // Same spacing, both tightened to 0.8 cubes: sum 0.8 == distance -> flush.
        let s = scenario(
            vec![ship(
                inline("a", Vec3::ZERO, cube(0.8)),
                inline("b", x(0.8), cube(0.8)),
            )],
            vec![],
        );
        let issues = lint_scenario(&s, &sections(&[]), &ships(&[]), &known(&["test_scenario"]));
        assert!(
            issues
                .iter()
                .all(|issue| !issue.message.contains("overlap")),
            "tightened cubes are flush: {issues:?}"
        );

        // 1.5 apart, oversized 2.0 cubes: sum 2.0 > 1.5 -> overlap where unit
        // cubes (sum 1.0) would pass.
        let s = scenario(
            vec![ship(
                inline("a", Vec3::ZERO, cube(2.0)),
                inline("b", x(1.5), cube(2.0)),
            )],
            vec![],
        );
        let issues = lint_scenario(&s, &sections(&[]), &ships(&[]), &known(&["test_scenario"]));
        assert_eq!(
            issues
                .iter()
                .filter(|issue| issue.message.contains("overlap"))
                .count(),
            1,
            "oversized cubes clip: {issues:?}"
        );

        // Capital-like 5x5x3 boxes mounted along Y are only three units deep
        // on that axis. Four units between their centres leaves one hull cell
        // between them; using unrotated extents falsely treated them as five
        // units deep and rejected this sandwich.
        let capital = Some(SectionCollider::Cuboid {
            size: Vec3::new(5.0, 5.0, 3.0),
        });
        let mut a = inline("a", Vec3::Y * 2.0, capital);
        let mut b = inline("b", Vec3::NEG_Y * 2.0, capital);
        a.rotation = Quat::from_rotation_x(std::f32::consts::FRAC_PI_2);
        b.rotation = Quat::from_rotation_x(std::f32::consts::FRAC_PI_2);
        let s = scenario(vec![ship(a, b)], vec![]);
        let issues = lint_scenario(&s, &sections(&[]), &ships(&[]), &known(&["test_scenario"]));
        assert!(
            issues
                .iter()
                .all(|issue| !issue.message.contains("overlap")),
            "rotated capital boxes leave room for the middle hull: {issues:?}"
        );

        // Quarter-turned unit cubes can produce extents just over 0.5 due to
        // quaternion arithmetic. Grid neighbours that meet at an edge remain
        // flush rather than becoming a microscopic overlap.
        let mut a = inline("a", Vec3::ZERO, cube(1.0));
        let mut b = inline("b", Vec3::new(1.0, 1.0, 0.0), cube(1.0));
        a.rotation = Quat::from_rotation_x(std::f32::consts::FRAC_PI_2);
        b.rotation = Quat::from_rotation_y(std::f32::consts::FRAC_PI_2);
        let s = scenario(vec![ship(a, b)], vec![]);
        let issues = lint_scenario(&s, &sections(&[]), &ships(&[]), &known(&["test_scenario"]));
        assert!(
            issues
                .iter()
                .all(|issue| !issue.message.contains("overlap")),
            "rotated grid neighbours are flush within epsilon: {issues:?}"
        );
    }

    /// A two-section fixture used to prove that a direct mate authorizes an
    /// intentional collider overlap.
    fn ship_with_mated_overlap() -> (EventActionConfig, KnownSections) {
        use nova_ship::prelude::{BaseSectionConfig, HullSectionConfig, LinkPoint};

        let section = |id: &str, normal: Vec3| SectionConfig {
            base: BaseSectionConfig {
                id: id.to_string(),
                collider: Some(SectionCollider::Cuboid {
                    size: Vec3::splat(2.0),
                }),
                link_points: vec![LinkPoint {
                    id: "mate".to_string(),
                    position: normal * 0.25,
                    normal,
                }],
                ..default()
            },
            kind: SectionKind::Hull(HullSectionConfig::default()),
        };
        let configs = [section("left", Vec3::X), section("right", Vec3::NEG_X)];
        let action = EventActionConfig::SpawnScenarioObject(ScenarioObjectConfig {
            base: BaseScenarioObjectConfig {
                id: "ship".to_string(),
                name: "ship".to_string(),
                position: Vec3::ZERO,
                rotation: Quat::IDENTITY,
            },
            kind: ScenarioObjectKind::Spaceship(SpaceshipConfig {
                hull: ShipSource::Inline(ShipHull {
                    sections: vec![
                        SpaceshipSectionConfig {
                            id: "left".to_string(),
                            position: Vec3::ZERO,
                            rotation: Quat::IDENTITY,
                            source: SectionSource::Prototype("left".to_string()),
                            modifications: vec![],
                        },
                        SpaceshipSectionConfig {
                            id: "right".to_string(),
                            position: Vec3::X * 0.5,
                            rotation: Quat::IDENTITY,
                            source: SectionSource::Prototype("right".to_string()),
                            modifications: vec![],
                        },
                    ],
                    ..default()
                }),
                ..default()
            }),
        });
        (action, KnownSections::from_configs(&configs))
    }

    #[test]
    fn directly_mated_sections_may_overlap() {
        let (action, catalog) = ship_with_mated_overlap();
        let s = scenario(vec![action], vec![]);
        let issues = lint_scenario(&s, &catalog, &ships(&[]), &known(&["test_scenario"]));
        assert!(issues.is_empty(), "{issues:?}");
    }

    #[test]
    fn turret_joint_tree_wellformedness_is_linted() {
        use nova_ship::prelude::{
            BaseSectionConfig, MuzzleConfig, TurretJoint, TurretSectionConfig,
        };

        fn joint(
            axis: Option<Vec3>,
            min: Option<f32>,
            max: Option<f32>,
            muzzle: bool,
            children: Vec<TurretJoint>,
        ) -> TurretJoint {
            TurretJoint {
                name: None,
                offset: Vec3::ZERO,
                axis,
                speed: std::f32::consts::PI,
                min,
                max,
                render_mesh: None,
                render_mesh_transform: None,
                muzzle: muzzle.then(|| MuzzleConfig {
                    fire_rate: 10.0,
                    muzzle_effect: None,
                }),
                children,
            }
        }
        let turret = |root: TurretJoint| SectionConfig {
            base: BaseSectionConfig {
                id: "t".to_string(),
                ..Default::default()
            },
            kind: SectionKind::Turret(TurretSectionConfig {
                root,
                ..Default::default()
            }),
        };

        // Valid: a hinge over a muzzle leaf, and the shipped default, pass clean.
        let ok = joint(
            Some(Vec3::Y),
            None,
            None,
            false,
            vec![joint(None, None, None, true, vec![])],
        );
        assert!(lint_section_config(&turret(ok), "s").is_empty());
        assert!(
            lint_section_config(&turret(TurretSectionConfig::default().root), "s").is_empty(),
            "the shipped default turret must lint clean"
        );

        // No muzzle anywhere -> error (can never fire).
        let none = joint(Some(Vec3::Y), None, None, false, vec![]);
        assert!(errors(&lint_section_config(&turret(none), "s"))
            .iter()
            .any(|i| i.message.contains("no muzzle")));

        // Degenerate hinge axis -> error.
        let zero = joint(
            Some(Vec3::ZERO),
            None,
            None,
            false,
            vec![joint(None, None, None, true, vec![])],
        );
        assert!(errors(&lint_section_config(&turret(zero), "s"))
            .iter()
            .any(|i| i.message.contains("degenerate hinge axis")));

        // min > max -> error (locked shut).
        let inverted = joint(
            Some(Vec3::X),
            Some(1.0),
            Some(-1.0),
            false,
            vec![joint(None, None, None, true, vec![])],
        );
        assert!(errors(&lint_section_config(&turret(inverted), "s"))
            .iter()
            .any(|i| i.message.contains("exceeds max")));

        // Rotation limits on a FIXED node -> warning, not error.
        let limits_no_axis = joint(None, Some(-1.0), Some(1.0), true, vec![]);
        let issues = lint_section_config(&turret(limits_no_axis), "s");
        assert!(errors(&issues).is_empty(), "{issues:?}");
        assert!(
            issues.iter().any(|i| i.message.contains("no `axis`")),
            "{issues:?}"
        );

        // F10: a non-positive fire_rate used to panic the ship spawn outright
        // (`1.0 / 0.0` into `Duration::from_secs_f32`). The runtime clamps it
        // now, so authoring it must fail HERE, with the field named, rather
        // than silently firing at the clamp's absurd rate.
        for rate in [0.0, -1.0, f32::NAN] {
            let mut leaf = joint(None, None, None, true, vec![]);
            leaf.muzzle.as_mut().unwrap().fire_rate = rate;
            let bad = joint(Some(Vec3::Y), None, None, false, vec![leaf]);
            let issues = lint_section_config(&turret(bad), "s");
            assert!(
                errors(&issues)
                    .iter()
                    .any(|i| i.message.contains("fire_rate")),
                "fire_rate {rate} must be a lint error: {issues:?}"
            );
        }
    }

    #[test]
    fn section_catalog_keeps_last_wins_collider_data() {
        use nova_ship::prelude::{BaseSectionConfig, HullSectionConfig};

        let section = |size: f32| SectionConfig {
            base: BaseSectionConfig {
                id: "contested".to_string(),
                collider: Some(SectionCollider::Cuboid {
                    size: Vec3::splat(size),
                }),
                ..default()
            },
            kind: SectionKind::Hull(HullSectionConfig::default()),
        };
        let configs = [section(1.0), section(2.0)];
        let catalog = KnownSections::from_configs(&configs);
        assert_eq!(
            catalog.get("contested").unwrap().collider,
            SectionCollider::Cuboid {
                size: Vec3::splat(2.0)
            }
        );
    }
}
