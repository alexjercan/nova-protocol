//! Structural checks over a scenario's ships and their section configs.

use bevy::prelude::Vec3;
use nova_ship::prelude::{
    derive_link_point_graph, LinkPointGraphError, LinkPointRef, PlacedSectionLinkPoints,
    SectionCollider, SectionConfig, SectionKind, TurretJoint, TurretSectionConfig,
};

use super::{KnownSections, LintIssue};
use crate::prelude::*;

/// Every section prototype a spawned (or scatter-template) ship references
/// must exist in the caller's known set.
pub(super) fn check_object_prototypes(
    config: &ScenarioObjectConfig,
    scenario: &str,
    sections: &KnownSections,
    issues: &mut Vec<LintIssue>,
) {
    if let ScenarioObjectKind::Spaceship(ship) = &config.kind {
        for section in &ship.sections {
            if let SectionSource::Prototype(proto) = &section.source {
                if !sections.contains(proto) {
                    issues.push(LintIssue::error(
                        scenario,
                        format!(
                            "ship '{}' section '{}': unknown section prototype '{proto}'",
                            config.base.id, section.id
                        ),
                    ));
                }
            }
        }
        check_section_overlaps(config.base.id.as_str(), ship, scenario, issues);
        check_link_point_graph(config.base.id.as_str(), ship, scenario, sections, issues);
        check_mount_adjacency(config.base.id.as_str(), ship, scenario, sections, issues);
        // Inline section configs a scenario writes directly (a Prototype ref
        // resolves to a catalog section, which is linted where the catalog is
        // walked - lint_bundle - so it is not re-linted here).
        for section in &ship.sections {
            if let SectionSource::Inline(inline) = &section.source {
                issues.extend(lint_section_config(inline, scenario));
            }
        }
    }
}

/// Static well-formedness of one section's config that the RON parser cannot
/// catch (a well-typed field can still be nonsense). Currently the turret joint
/// tree; other kinds pass. Pure over the config, so every consumer - the author
/// CLI's `lint`, the CI gate, the runtime merge - runs the SAME check on base +
/// mod section catalogs, and `lint_scenario` runs it on inline turret sections.
pub fn lint_section_config(config: &SectionConfig, source: &str) -> Vec<LintIssue> {
    let mut issues = Vec::new();
    if let SectionKind::Turret(turret) = &config.kind {
        check_turret_tree(config.base.id.as_str(), turret, source, &mut issues);
    }
    check_link_point_config(config, source, &mut issues);
    issues
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
    ship: &SpaceshipConfig,
    scenario: &str,
    sections: &KnownSections,
    issues: &mut Vec<LintIssue>,
) {
    let resolved: Option<Vec<_>> = ship
        .sections
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
    let placed: Vec<_> = ship
        .sections
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
                ship.sections[section_index].id, ship.sections[section_index].position
            ),
            LinkPointGraphError::NonFiniteSectionRotation { section_index } => format!(
                "ship '{ship_id}' section '{}': rotation {:?} must be finite",
                ship.sections[section_index].id, ship.sections[section_index].rotation
            ),
            LinkPointGraphError::NonUnitSectionRotation { section_index } => format!(
                "ship '{ship_id}' section '{}': rotation {:?} must have unit length",
                ship.sections[section_index].id, ship.sections[section_index].rotation
            ),
            LinkPointGraphError::AmbiguousMate {
                link_point,
                candidates,
            } => {
                let point = &resolved_link_point(ship, sections, link_point);
                let candidates = candidates
                    .into_iter()
                    .map(|candidate| {
                        let candidate_point = resolved_link_point(ship, sections, candidate);
                        format!(
                            "{}.{}",
                            ship.sections[candidate.section_index].id, candidate_point.id
                        )
                    })
                    .collect::<Vec<_>>()
                    .join(", ");
                format!(
                    "ship '{ship_id}' section '{}' link point '{}': ambiguous mates [{candidates}]",
                    ship.sections[link_point.section_index].id, point.id
                )
            }
            LinkPointGraphError::Disconnected { components } => {
                let components = components
                    .into_iter()
                    .map(|component| {
                        component
                            .into_iter()
                            .map(|index| ship.sections[index].id.as_str())
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
    ship: &'a SpaceshipConfig,
    sections: &'a KnownSections,
    reference: LinkPointRef,
) -> &'a nova_ship::prelude::LinkPoint {
    let section = &ship.sections[reference.section_index];
    let points = match &section.source {
        SectionSource::Inline(config) => &config.base.link_points,
        SectionSource::Prototype(id) => &sections.get(id).expect("prototype resolved").link_points,
    };
    &points[reference.link_point_index]
}

/// Two sections of one ship OVERLAP - clip visually and double up their
/// colliders in the same space - iff their axis-aligned collider boxes
/// interpenetrate: centers strictly closer than the sum of their half-extents
/// on EVERY axis. For the default unit-cube sections (half-extent 0.5 each)
/// that is the classic "closer than 1.0 on every axis"; authorable colliders
/// ([`SectionCollider`]) widen or narrow the threshold per section. Flush
/// contact (distance exactly the half-extent sum on some axis) is the normal
/// spine/side-mount layout and passes. The check ignores section ROTATION:
/// exact for the quarter-turn rotations all shipped content uses (a unit cube
/// is symmetric under them; a non-cube box's AABB is a conservative
/// over-approximation), conservative-only for exotic angles. Only INLINE
/// colliders are resolved; a `Prototype` section falls back to the unit cube
/// (the catalog is not in scope here), matching pre-config behavior. Caught in
/// the wild by the Auditor's torpedo bay authored at z 0.5, embedded between
/// two spine sections.
fn check_section_overlaps(
    ship_id: &str,
    ship: &SpaceshipConfig,
    scenario: &str,
    issues: &mut Vec<LintIssue>,
) {
    /// AABB half-extents of a section's collider, ignoring rotation. Inline
    /// sources use their authored collider (unit cube when unset); Prototype
    /// sources fall back to the unit cube since the catalog is not resolvable
    /// here.
    fn half_extents(section: &SpaceshipSectionConfig) -> Vec3 {
        match &section.source {
            SectionSource::Inline(config) => {
                config.base.collider.unwrap_or_default().aabb_half_extents()
            }
            SectionSource::Prototype(_) => SectionCollider::default().aabb_half_extents(),
        }
    }

    for i in 0..ship.sections.len() {
        for j in (i + 1)..ship.sections.len() {
            let (a, b) = (&ship.sections[i], &ship.sections[j]);
            let d = a.position - b.position;
            let sum = half_extents(a) + half_extents(b);
            if d.x.abs() < sum.x && d.y.abs() < sum.y && d.z.abs() < sum.z {
                issues.push(LintIssue::error(
                    scenario,
                    format!(
                        "ship '{ship_id}': sections '{}' at {:?} and '{}' at {:?} overlap (collider boxes interpenetrate: centers must be >= {:?} apart on some axis)",
                        a.id, a.position, b.id, b.position, sum
                    ),
                ));
            }
        }
    }
}

/// Mount-base adjacency: a mount section's base face (local -Y) must sit flush
/// against an occupied neighbor cell - `position + rotation * -Y` lands on a
/// sibling section's cell. ANY sibling counts, not just hulls (most shipped
/// ships seat the aft turret's base against the controller cell). All shipped
/// content rotates sections by quarter-turns, so the base direction is
/// axis-aligned; a mount with a non-quarter rotation gets a Warn note and is
/// otherwise skipped (conservative, like the overlap check's rotation caveat).
/// This check would have caught both shipped wrong-roll bugs at authoring time:
/// the Auditor bay bottom-down at a flank cell and all four gunship side mounts
/// with spine-end rolls.
fn check_mount_adjacency(
    ship_id: &str,
    ship: &SpaceshipConfig,
    scenario: &str,
    sections: &KnownSections,
    issues: &mut Vec<LintIssue>,
) {
    // f32 quat error on authored quarter-turns is ~1e-7 and authored
    // positions sit on the unit grid; the smallest shipped slip (the
    // Auditor bay's 0.5-cell offset) is orders above both epsilons.
    const AXIS_EPS: f32 = 1e-4;
    const CELL_EPS: f32 = 1e-3;
    for section in &ship.sections {
        let is_mount = match &section.source {
            SectionSource::Inline(config) => KnownSections::kind_mounts(&config.kind),
            SectionSource::Prototype(proto) => {
                sections.get(proto).is_some_and(|section| section.mounts)
            }
        };
        if !is_mount {
            continue;
        }
        let base_dir = section.rotation * Vec3::NEG_Y;
        let snapped = base_dir.round();
        // A quarter-turn of a UNIT quat sends -Y to a unit axis vector.
        // Anything else - a free angle, or a non-unit hand-typed quat, for
        // which `q * v` is not a rotation at all (a sqrt(2)-scaled quarter-turn
        // yields an INTEGER base direction like (-2, 1, 0) that would pass the
        // deviation test alone) - is statically uncheckable: note and skip. `snapped` components are exact integers, so the length
        // comparison is exact.
        if (base_dir - snapped).abs().max_element() > AXIS_EPS || snapped.length_squared() != 1.0 {
            issues.push(LintIssue::warn(
                scenario,
                format!(
                    "ship '{ship_id}' section '{}': non-quarter-turn (or non-unit) rotation, \
                     mount-base adjacency unchecked (base direction {base_dir:?})",
                    section.id
                ),
            ));
            continue;
        }
        // The section can never satisfy itself: the target cell is a full
        // unit away from its own position.
        let target = section.position + snapped;
        let occupied = ship
            .sections
            .iter()
            .any(|other| (other.position - target).abs().max_element() < CELL_EPS);
        if !occupied {
            issues.push(LintIssue::error(
                scenario,
                format!(
                    "ship '{ship_id}' section '{}' at {:?}: mount base (rotation * -Y = \
                     {snapped:?}) points at empty cell {target:?} - a turret/torpedo bay \
                     must sit base-against an occupied neighbor cell",
                    section.id, section.position
                ),
            ));
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
        let issues = lint_scenario(&s, &sections(&["known_proto"]), &known(&["test_scenario"]));
        let errs = errors(&issues);
        assert_eq!(errs.len(), 1, "{issues:?}");
        assert!(errs[0].message.contains("no_such_proto"));
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
        ship.sections.push(SpaceshipSectionConfig {
            id: "second".to_string(),
            position: Vec3::X,
            rotation: Quat::IDENTITY,
            source: SectionSource::Prototype("empty".to_string()),
            modifications: Vec::new(),
        });
        let scenario = scenario(vec![action], vec![]);
        let issues = lint_scenario(&scenario, &catalog, &known(&["test_scenario"]));
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
        let issues = lint_scenario(&s, &sections(&["known_proto"]), &known(&["test_scenario"]));
        let errs = errors(&issues);
        assert_eq!(errs.len(), 1, "{issues:?}");
        assert!(errs[0].message.contains("no_such_proto"));
    }
    /// Section overlaps: strictly-inside-the-cube errors; flush spine/side
    /// mounts pass (the fail-first is the shipped Auditor tube at z 0.5 this
    /// check was born from).
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
                    controller: SpaceshipController::None,
                    allegiance: None,
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
                }),
            })
        };

        // The Auditor shape: half-embedded on the spine.
        let s = scenario(vec![ship_with(Vec3::new(0.0, 0.0, 0.5))], vec![]);
        let issues = lint_scenario(&s, &sections(&["known"]), &known(&["test_scenario"]));
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
        let issues = lint_scenario(&s, &sections(&["known"]), &known(&["test_scenario"]));
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
                    controller: SpaceshipController::None,
                    allegiance: None,
                    sections: vec![a, b],
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
        let issues = lint_scenario(&s, &sections(&[]), &known(&["test_scenario"]));
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
        let issues = lint_scenario(&s, &sections(&[]), &known(&["test_scenario"]));
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
        let issues = lint_scenario(&s, &sections(&[]), &known(&["test_scenario"]));
        assert_eq!(
            issues
                .iter()
                .filter(|issue| issue.message.contains("overlap"))
                .count(),
            1,
            "oversized cubes clip: {issues:?}"
        );
    }

    /// The mount-fixture ship: a hull cell at the origin plus one MOUNT
    /// prototype at `pos`/`rotation`. Catalog for these tests:
    /// `sections_with_mounts(&["hull_proto"], &["mount_proto"])`.
    fn ship_with_mount(pos: Vec3, rotation: Quat) -> EventActionConfig {
        EventActionConfig::SpawnScenarioObject(ScenarioObjectConfig {
            base: BaseScenarioObjectConfig {
                id: "ship".to_string(),
                name: "ship".to_string(),
                position: Vec3::ZERO,
                rotation: Quat::IDENTITY,
            },
            kind: ScenarioObjectKind::Spaceship(SpaceshipConfig {
                controller: SpaceshipController::None,
                allegiance: None,
                sections: vec![
                    SpaceshipSectionConfig {
                        id: "hull".to_string(),
                        position: Vec3::ZERO,
                        rotation: Quat::IDENTITY,
                        source: SectionSource::Prototype("hull_proto".to_string()),
                        modifications: vec![],
                    },
                    SpaceshipSectionConfig {
                        id: "mount".to_string(),
                        position: pos,
                        rotation,
                        source: SectionSource::Prototype("mount_proto".to_string()),
                        modifications: vec![],
                    },
                ],
            }),
        })
    }

    /// every shipped mount-roll shape lints clean - flank mounts with inboard
    /// Rz rolls, a top mount's identity, and the bow mount's Rx(-90) (base
    /// against the cell astern of it).
    #[test]
    fn mount_bases_against_occupied_cells_are_clean() {
        use std::f32::consts::FRAC_PI_2;
        let catalog = sections_with_mounts(&["hull_proto"], &["mount_proto"]);
        for (pos, rotation) in [
            // Starboard flank, base rolled inboard (-Y -> -X).
            (Vec3::new(1.0, 0.0, 0.0), Quat::from_rotation_z(-FRAC_PI_2)),
            // Port flank, the mirror roll (-Y -> +X).
            (Vec3::new(-1.0, 0.0, 0.0), Quat::from_rotation_z(FRAC_PI_2)),
            // Top mount, identity: base straight down at the hull.
            (Vec3::new(0.0, 1.0, 0.0), Quat::IDENTITY),
            // Bow mount, the player-ship roll: base astern (-Y -> +Z).
            (Vec3::new(0.0, 0.0, -1.0), Quat::from_rotation_x(-FRAC_PI_2)),
        ] {
            let s = scenario(vec![ship_with_mount(pos, rotation)], vec![]);
            let issues = lint_scenario(&s, &catalog, &known(&["test_scenario"]));
            assert!(issues.is_empty(), "mount at {pos:?} flagged: {issues:?}");
        }
    }

    /// The two shipped wrong-roll shapes are errors: the Auditor bay
    /// bottom-down on a flank cell and the gunship side mounts with the
    /// spine-end Rx(-90) roll.
    #[test]
    fn mount_base_at_an_empty_cell_is_an_error() {
        let catalog = sections_with_mounts(&["hull_proto"], &["mount_proto"]);
        for (pos, rotation) in [
            (Vec3::new(1.0, 0.0, 0.0), Quat::IDENTITY),
            (
                Vec3::new(1.0, 0.0, 0.0),
                Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2),
            ),
        ] {
            let s = scenario(vec![ship_with_mount(pos, rotation)], vec![]);
            let issues = lint_scenario(&s, &catalog, &known(&["test_scenario"]));
            let errs = errors(&issues);
            assert_eq!(errs.len(), 1, "mount rot {rotation:?}: {issues:?}");
            assert!(errs[0].message.contains("mount base"), "{issues:?}");
        }
    }

    /// A non-quarter-turn - or non-unit (`q * v` is not a rotation for a
    /// non-unit hand-typed quat, and a sqrt(2)-scaled quarter-turn snaps to an
    /// integer NON-UNIT direction that the deviation test alone would accept) -
    /// mount rotation is skipped with a Warn note, never errored: the
    /// static check cannot reason about either (the same conservative caveat as
    /// the overlap check's).
    #[test]
    fn mount_with_non_quarter_rotation_warns_and_skips() {
        let catalog = sections_with_mounts(&["hull_proto"], &["mount_proto"]);
        for rotation in [
            Quat::from_rotation_z(0.7),
            // Rz(-90) scaled by sqrt(2): base_dir snaps to (-2, 1, 0).
            Quat::from_xyzw(0.0, 0.0, -1.0, 1.0),
        ] {
            let s = scenario(
                vec![ship_with_mount(Vec3::new(1.0, 0.0, 0.0), rotation)],
                vec![],
            );
            let issues = lint_scenario(&s, &catalog, &known(&["test_scenario"]));
            assert!(
                issues.iter().any(|issue| {
                    issue.message.contains("disconnected")
                        || issue.message.contains("must have unit length")
                }),
                "the link graph rejects the placement: {issues:?}"
            );
            assert!(
                issues
                    .iter()
                    .any(|issue| issue.message.contains("non-quarter")),
                "{issues:?}"
            );
        }
    }

    /// Occupancy is kind-blind: a mount seated base-against ANOTHER MOUNT's
    /// cell passes - any sibling section counts, matching the shipped ships
    /// that seat turrets against the controller cell.
    #[test]
    fn mount_seated_against_another_mount_is_clean() {
        use std::f32::consts::FRAC_PI_2;
        let inboard = Quat::from_rotation_z(-FRAC_PI_2);
        let mount = |id: &str, pos: Vec3| SpaceshipSectionConfig {
            id: id.to_string(),
            position: pos,
            rotation: inboard,
            source: SectionSource::Prototype("mount_proto".to_string()),
            modifications: vec![],
        };
        let s = scenario(
            vec![EventActionConfig::SpawnScenarioObject(
                ScenarioObjectConfig {
                    base: BaseScenarioObjectConfig {
                        id: "ship".to_string(),
                        name: "ship".to_string(),
                        position: Vec3::ZERO,
                        rotation: Quat::IDENTITY,
                    },
                    kind: ScenarioObjectKind::Spaceship(SpaceshipConfig {
                        controller: SpaceshipController::None,
                        allegiance: None,
                        sections: vec![
                            SpaceshipSectionConfig {
                                id: "hull".to_string(),
                                position: Vec3::ZERO,
                                rotation: Quat::IDENTITY,
                                source: SectionSource::Prototype("hull_proto".to_string()),
                                modifications: vec![],
                            },
                            // Inner mount seats against the hull; the outer one
                            // seats against the INNER MOUNT.
                            mount("mount_inner", Vec3::new(1.0, 0.0, 0.0)),
                            mount("mount_outer", Vec3::new(2.0, 0.0, 0.0)),
                        ],
                    }),
                },
            )],
            vec![],
        );
        let catalog = sections_with_mounts(&["hull_proto"], &["mount_proto"]);
        let issues = lint_scenario(&s, &catalog, &known(&["test_scenario"]));
        assert!(issues.is_empty(), "{issues:?}");
    }

    /// An INLINE mount section is checked from its own kind - no catalog
    /// membership involved.
    #[test]
    fn inline_mount_sections_are_checked() {
        use nova_ship::prelude::{BaseSectionConfig, TurretSectionConfig};

        let mut action = ship_with_mount(Vec3::new(1.0, 0.0, 0.0), Quat::IDENTITY);
        let EventActionConfig::SpawnScenarioObject(config) = &mut action else {
            unreachable!()
        };
        let ScenarioObjectKind::Spaceship(ship) = &mut config.kind else {
            unreachable!()
        };
        ship.sections[1].source = SectionSource::Inline(SectionConfig {
            base: BaseSectionConfig {
                link_points: nova_ship::prelude::unit_cube_link_points(),
                ..default()
            },
            kind: SectionKind::Turret(TurretSectionConfig::default()),
        });
        let s = scenario(vec![action], vec![]);
        let issues = lint_scenario(&s, &sections(&["hull_proto"]), &known(&["test_scenario"]));
        let errs = errors(&issues);
        assert_eq!(errs.len(), 1, "{issues:?}");
        assert!(errs[0].message.contains("mount base"), "{issues:?}");
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

    /// `KnownSections::from_configs` classifies the resolved last-wins definition.
    #[test]
    fn section_catalog_classifies_mount_kinds() {
        use nova_ship::prelude::{
            BaseSectionConfig, ControllerSectionConfig, HullSectionConfig, ThrusterSectionConfig,
            TorpedoSectionConfig, TurretSectionConfig,
        };

        let section = |id: &str, kind: SectionKind| SectionConfig {
            base: BaseSectionConfig {
                id: id.to_string(),
                ..Default::default()
            },
            kind,
        };
        let configs = vec![
            section("hull", SectionKind::Hull(HullSectionConfig::default())),
            section(
                "thruster",
                SectionKind::Thruster(ThrusterSectionConfig::default()),
            ),
            section(
                "controller",
                SectionKind::Controller(ControllerSectionConfig::default()),
            ),
            section(
                "turret",
                SectionKind::Turret(TurretSectionConfig::default()),
            ),
            section(
                "torpedo",
                SectionKind::Torpedo(TorpedoSectionConfig::default()),
            ),
            // The same id defined as a mount in one bundle, a hull in another.
            section(
                "contested",
                SectionKind::Turret(TurretSectionConfig::default()),
            ),
            section("contested", SectionKind::Hull(HullSectionConfig::default())),
        ];
        let catalog = KnownSections::from_configs(&configs);
        for id in [
            "hull",
            "thruster",
            "controller",
            "turret",
            "torpedo",
            "contested",
        ] {
            assert!(catalog.contains(id), "missing {id}: {catalog:?}");
        }
        assert!(catalog.get("turret").unwrap().mounts);
        assert!(catalog.get("torpedo").unwrap().mounts);
        assert!(
            !catalog.get("contested").unwrap().mounts,
            "the later hull override wins: {catalog:?}"
        );
    }
}
