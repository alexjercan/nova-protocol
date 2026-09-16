//! Structural checks over a scenario's ships and their section configs.

use bevy::prelude::Vec3;
use nova_events::units::prelude::*;
use nova_ship::prelude::{
    candidate_link_point_mates, derive_link_point_graph, duplicate_muzzle_id, muzzle_ids,
    section_colliders_overlap, AmmoCapacity, ControllerSectionConfig, DockingSectionConfig,
    LinkPointGraphError, LinkPointRef, PlacedSectionCollider, PlacedSectionLinkPoints,
    RailgunSectionConfig, ReloadConfig, SectionAnimationCue, SectionCollider, SectionConfig,
    SectionKind, TorpedoSectionConfig, TurretJoint, TurretSectionConfig,
};

use super::{KnownSections, KnownShipDesigns, LintIssue};
use crate::prelude::*;

/// Every reference a spawned (or scatter-template) ship makes must resolve:
/// the design it names, the section prototypes that design is built from, and
/// the sections its spawn-time patches aim at.
///
/// A `Prototype` design's own geometry is NOT re-checked here - it is linted
/// where the design catalog is walked ([`lint_ship_design_config`]), the same
/// rule a `Prototype` section follows.
pub(super) fn check_object_prototypes(
    config: &ScenarioObjectConfig,
    scenario: &str,
    sections: &KnownSections,
    designs: &KnownShipDesigns,
    issues: &mut Vec<LintIssue>,
) {
    let ScenarioObjectKind::Spaceship(ship) = &config.kind else {
        return;
    };
    let catalog = GameShipDesigns(
        designs
            .get(ship.design.prototype_id().unwrap_or_default())
            .map(|design| {
                vec![ShipDesignPrototype {
                    id: ship.design.prototype_id().unwrap_or_default().to_string(),
                    name: String::new(),
                    design: design.clone(),
                }]
            })
            .unwrap_or_default(),
    );
    // An INLINE design is authored right here, so it is resolved and its
    // structure linted in one place below. It has no spawn layer to check
    // separately - it IS the design - and resolving it twice would report
    // every broken reference twice.
    if let ShipDesignSource::Inline(design) = &ship.design {
        check_design_sections(config.base.id.as_str(), design, scenario, sections, issues);
        return;
    }

    // The SPAWN's own layer: resolve exactly the way the spawn will, so a
    // patch that names nothing, disagrees with a section's kind or misspells a
    // muzzle is caught here rather than at the first playthrough. The design's
    // OWN section references are dropped: they belong to the design, which the
    // catalog walk lints once however many scenarios spawn it.
    let (_, errors) = resolve_ship_design(&ship.design, &catalog, sections.catalog());
    for error in errors {
        if matches!(error, ShipDesignError::UnknownSectionPrototype { .. }) {
            continue;
        }
        issues.push(LintIssue::error(
            scenario,
            format!("ship '{}': {error}", config.base.id),
        ));
    }
}

/// Static checks over one design's section list: every prototype resolves,
/// every patch applies, the sections do not interpenetrate, the link-point
/// graph is sound, and every inline section config is well-formed.
///
/// Run on an inline design where the scenario spawns it, and on a catalog
/// design where the design catalog is walked - so a design is checked exactly
/// once, wherever it is authored.
fn check_design_sections(
    ship_id: &str,
    design: &ShipDesign,
    source: &str,
    sections: &KnownSections,
    issues: &mut Vec<LintIssue>,
) {
    // The design's OWN layer: each section reference resolved and patched, by
    // the one resolver the spawn uses.
    let (_, errors) = resolve_ship_design(
        &ShipDesignSource::Inline(design.clone()),
        &GameShipDesigns::default(),
        sections.catalog(),
    );
    for error in errors {
        issues.push(LintIssue::error(
            source,
            format!("ship '{ship_id}': {error}"),
        ));
    }

    check_section_overlaps(ship_id, &design.sections, source, sections, issues);
    check_link_point_graph(ship_id, &design.sections, source, sections, issues);
    // Inline section configs authored directly (a Prototype ref resolves to a
    // catalog section, which is linted where the catalog is walked -
    // lint_bundle - so it is not re-linted here).
    for section in &design.sections {
        if let SectionSource::Inline(inline) = &section.source {
            issues.extend(lint_section_config(inline, source));
        }
    }
}

/// Static well-formedness of one CATALOG design: the same structural checks a
/// scenario's inline design gets, run where the design is authored so a build
/// referenced by eleven scenarios is checked once. Pure over the config, like
/// [`lint_section_config`] beside it.
pub fn lint_ship_design_config(
    design: &ShipDesignPrototype,
    sections: &KnownSections,
    source: &str,
) -> Vec<LintIssue> {
    let mut issues = Vec::new();
    check_design_sections(
        design.id.as_str(),
        &design.design,
        source,
        sections,
        &mut issues,
    );
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
                turret.ammunition,
                turret.reload,
                source,
                &mut issues,
            );
            check_turret_tree(config.base.id.as_str(), turret, source, &mut issues);
        }
        SectionKind::Torpedo(torpedo) => {
            check_reload_config(
                config.base.id.as_str(),
                torpedo.ammunition,
                torpedo.reload,
                source,
                &mut issues,
            );
            check_torpedo_numbers(config.base.id.as_str(), torpedo, source, &mut issues);
        }
        SectionKind::Railgun(railgun) => {
            check_reload_config(
                config.base.id.as_str(),
                railgun.ammunition,
                railgun.reload,
                source,
                &mut issues,
            );
            check_railgun_numbers(config.base.id.as_str(), railgun, source, &mut issues);
        }
        SectionKind::Docking(docking) => {
            check_docking_config(config, docking, source, &mut issues);
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
    ammunition: AmmoCapacity,
    reload: ReloadConfig,
    source: &str,
    issues: &mut Vec<LintIssue>,
) {
    let ReloadConfig::Batch(reload) = reload else {
        return;
    };
    if ammunition.rounds().is_none_or(|rounds| rounds == 0) {
        issues.push(LintIssue::error(
            source,
            format!("section '{section_id}': reload requires a Limited ammunition of at least one round"),
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

/// Flag a bay whose cadence is not one.
///
/// `1.0 / fire_rate` is the launch interval, and the value has no usable answer
/// off the positive finite line: `0.0` gives an infinite cooldown that fires
/// once and never again, a negative or non-finite one gives a cooldown that is
/// ready every tick. The runtime withholds the bay's launcher outright rather
/// than substituting a cadence nobody authored, so this is the early, named
/// report of a bay that will not shoot.
fn check_torpedo_numbers(
    section_id: &str,
    config: &TorpedoSectionConfig,
    source: &str,
    issues: &mut Vec<LintIssue>,
) {
    if !config.fire_rate.is_finite() || config.fire_rate <= 0.0 {
        issues.push(LintIssue::error(
            source,
            format!(
                "section '{section_id}': torpedo fire_rate must be a positive, finite number \
                 of launches/s, got {}",
                config.fire_rate
            ),
        ));
    }
}

/// Flag a lance whose clock or whose corridor cannot be built.
///
/// A non-finite or negative `charge_seconds` divides the cue's progress by
/// nonsense; zero is legal and means an instant commit, which is a design
/// choice rather than a mistake. A non-finite or negative `rake_radius` is a
/// sphere with no size to sweep; zero is legal there too and means the narrow
/// gun, which is also what omitting the field means.
fn check_railgun_numbers(
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
    let bad_rake = config
        .rake_radius
        .filter(|radius| *radius < Meters::ZERO || !radius.is_finite());
    if let Some(radius) = bad_rake {
        issues.push(LintIssue::error(
            source,
            format!(
                "section '{section_id}': railgun rake_radius must be a finite, non-negative \
                 number of meters, got {}",
                radius.get()
            ),
        ));
    }
}

/// A docking port's capture envelope, and the two shape facts the mechanic
/// assumes about the section it is authored on.
///
/// The port face is HALF A CELL along local -Z, so a port authored bigger
/// than one cell measures its gap from a face that is not where its mouth is
/// - the capture would fire early on one side of the hull and late on the
/// other. That is a content error, not a tuning choice, which is why it is
/// graded here rather than clamped at spawn.
///
/// The missing sleeve track is a WARNING: a port with no authored animation
/// docks correctly and simply never moves, which is exactly what a headless
/// fixture wants and almost never what a shipped section does.
fn check_docking_config(
    config: &SectionConfig,
    docking: &DockingSectionConfig,
    source: &str,
    issues: &mut Vec<LintIssue>,
) {
    let section_id = config.base.id.as_str();
    if docking.capture_distance <= Meters::ZERO || !docking.capture_distance.is_finite() {
        issues.push(LintIssue::error(
            source,
            format!(
                "section '{section_id}': docking capture_distance must be a finite, positive \
                 number of meters, got {}",
                docking.capture_distance.get()
            ),
        ));
    }
    if !(0.0..90.0).contains(&docking.capture_angle) || !docking.capture_angle.is_finite() {
        issues.push(LintIssue::error(
            source,
            format!(
                "section '{section_id}': docking capture_angle must be at least 0 and under 90 \
                 degrees, got {}",
                docking.capture_angle
            ),
        ));
    }
    if docking.maximum_relative_speed < MetersPerSecond::ZERO
        || !docking.maximum_relative_speed.is_finite()
    {
        issues.push(LintIssue::error(
            source,
            format!(
                "section '{section_id}': docking maximum_relative_speed must be a finite, \
                 non-negative speed, got {}",
                docking.maximum_relative_speed.get()
            ),
        ));
    }
    if docking.maximum_relative_angular_speed < 0.0
        || !docking.maximum_relative_angular_speed.is_finite()
    {
        issues.push(LintIssue::error(
            source,
            format!(
                "section '{section_id}': docking maximum_relative_angular_speed must be a \
                 finite, non-negative rate in degrees per second, got {}",
                docking.maximum_relative_angular_speed
            ),
        ));
    }
    if let Some(SectionCollider::Cuboid { size }) = config.base.collider {
        if size.x > 1.0 || size.y > 1.0 || size.z > 1.0 {
            issues.push(LintIssue::error(
                source,
                format!(
                    "section '{section_id}': a docking port must fit one cell - its face is half \
                     a cell along -Z - got a collider of {size:?}"
                ),
            ));
        }
    }
    if !config
        .base
        .animations
        .iter()
        .any(|track| track.cue == SectionAnimationCue::DockTube)
    {
        issues.push(LintIssue::warn(
            source,
            format!(
                "section '{section_id}': docking port authors no DockTube animation track, so \
                 its sleeve never moves"
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
    // An id is the handle a patch and an editor row hold a barrel by, so two
    // barrels that answer to one name is a turret no patch can address. Caught
    // HERE rather than where a patch names it: the fault is the section's, and
    // it is a fault whether or not anything has patched it yet.
    if let Some(duplicate) = duplicate_muzzle_id(&config.root) {
        issues.push(LintIssue::error(
            source,
            format!(
                "section '{section_id}': two turret muzzles share the id '{duplicate}' - a patch \
                 and an editor row address a barrel by id, so every one of them must be unique \
                 within the turret"
            ),
        ));
    }
    for (index, id) in muzzle_ids(&config.root).into_iter().enumerate() {
        if id.trim().is_empty() {
            issues.push(LintIssue::error(
                source,
                format!(
                    "section '{section_id}': turret muzzle {index} has an empty id - name it \
                     'main' on a single barrel, 'left' and 'right' on a twin"
                ),
            ));
        }
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
            SectionSource::Prototype { id, .. } => sections
                .get(id)
                .map(|known| known.base.link_points.as_slice()),
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
        SectionSource::Prototype { id, .. } => {
            &sections
                .get(id)
                .expect("prototype resolved")
                .base
                .link_points
        }
    };
    &points[reference.link_point_index]
}

/// Reject collider AABB interpenetration unless the two sections directly mate.
///
/// The arithmetic and its tolerance are `nova_ship`'s
/// [`section_colliders_overlap`], shared with the editor's placement refusal so
/// that a hull a builder is allowed to assemble is a hull this lint accepts.
///
/// What is EXEMPT is the pairs that offer each other a socket
/// ([`candidate_link_point_mates`]), not the pairs the derived ship graph
/// publishes. An authored mate is what makes an interface intentional, and it
/// is authored whether or not the hull around it derives: an ambiguous socket,
/// a disconnected hull or a malformed link point is already reported - by
/// `check_link_point_graph` and by [`lint_section_config`] - and reading such a
/// hull's mate set as EMPTY would bury those findings under an overlap error on
/// every seam the author meant. Interpenetration nothing mates is still
/// reported, which is what catches accidental duplicate or embedded parts.
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
            SectionSource::Prototype { id, .. } => sections.get(id).map(|known| {
                (
                    known.base.collider.unwrap_or_default(),
                    known.base.link_points.as_slice(),
                )
            }),
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
    let direct_mates = candidate_link_point_mates(&placed)
        .into_iter()
        .map(|mate| {
            let a = mate.a.section_index.min(mate.b.section_index);
            let b = mate.a.section_index.max(mate.b.section_index);
            (a, b)
        })
        .collect::<std::collections::BTreeSet<_>>();
    let bounds = |index: usize| PlacedSectionCollider {
        position: ship_sections[index].position,
        rotation: ship_sections[index].rotation,
        collider: resolved[index].0,
    };

    for i in 0..ship_sections.len() {
        for j in (i + 1)..ship_sections.len() {
            let (a, b) = (&ship_sections[i], &ship_sections[j]);
            if section_colliders_overlap(bounds(i), bounds(j)) && !direct_mates.contains(&(i, j)) {
                let separation = bounds(i).half_extents() + bounds(j).half_extents();
                issues.push(LintIssue::error(
                    scenario,
                    format!(
                        "ship '{ship_id}': unmated sections '{}' at {:?} and '{}' at {:?} \
                         overlap (collider boxes interpenetrate: centers must be >= {:?} apart \
                         on some axis, or the sections must directly mate)",
                        a.id, a.position, b.id, b.position, separation
                    ),
                ));
            }
        }
    }
}

#[cfg(test)]
mod tests {

    use bevy::prelude::*;
    use nova_ship::prelude::{SectionConfigPatch, SectionReloadConfig};

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
                    position: Meters3::ZERO,
                    rotation: Quat::IDENTITY,
                },
                kind: ScenarioObjectKind::Spaceship(SpaceshipConfig {
                    design: ShipDesignSource::prototype(ship),
                    ..default()
                }),
            })
        };

        let s = scenario(vec![by_id("no_such_ship")], vec![]);
        let issues = lint_scenario(
            &s,
            &sections(&["hull"]),
            &ships(&["block_gunship"]),
            &known(&["test_scenario"]),
        );
        let errs = errors(&issues);
        assert_eq!(errs.len(), 1, "{issues:?}");
        assert!(errs[0].message.contains("no_such_ship"));

        let s = scenario(vec![by_id("block_gunship")], vec![]);
        let issues = lint_scenario(
            &s,
            &sections(&["hull"]),
            &ships(&["block_gunship"]),
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
                    position: Meters3::ZERO,
                    rotation: Quat::IDENTITY,
                },
                kind: ScenarioObjectKind::Spaceship(SpaceshipConfig {
                    design: ShipDesignSource::Prototype {
                        id: "block_gunship".to_string(),
                        section_patches: [(
                            section.to_string(),
                            SpaceshipSectionConfigPatch {
                                config: SectionConfigPatch {
                                    health: Some(500.0),
                                    ..default()
                                },
                                ..default()
                            },
                        )]
                        .into_iter()
                        .collect(),
                    },
                    ..default()
                }),
            })
        };

        let s = scenario(vec![with_override("no_such_section")], vec![]);
        let issues = lint_scenario(
            &s,
            &sections(&["hull"]),
            &ships(&["block_gunship"]),
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
            &ships(&["block_gunship"]),
            &known(&["test_scenario"]),
        )
        .is_empty());
    }

    /// A mount rotation is a DIRECTION before it is a pose.
    ///
    /// A thruster pushes along its own local -Z turned by this quaternion, so
    /// a zero or non-finite one names no direction at all and every runtime
    /// reader of an engine's line of thrust skips the section (see
    /// `nova_ship`'s `engine_direction`). The link-point graph already rejects
    /// both, for its own reasons; this pins that rule in the terms the flight
    /// layer depends on it, so nobody relaxes it into a warning.
    #[test]
    fn a_section_mounted_on_a_degenerate_rotation_is_an_error() {
        let ship = |rotation: Quat| ShipDesignPrototype {
            id: "block_gunship".to_string(),
            name: "Gunship".to_string(),
            design: ShipDesign {
                sections: vec![SpaceshipSectionConfig {
                    id: "fuselage".to_string(),
                    position: Vec3::ZERO,
                    rotation,
                    source: SectionSource::prototype("hull"),
                }],
                ..default()
            },
        };

        assert!(
            lint_ship_design_config(&ship(Quat::IDENTITY), &sections(&["hull"]), "base").is_empty(),
            "fixture guard: an ordinary mount lints clean"
        );

        for rotation in [
            Quat::from_xyzw(0.0, 0.0, 0.0, 0.0),
            Quat::from_xyzw(f32::NAN, 0.0, 0.0, 1.0),
        ] {
            let issues = lint_ship_design_config(&ship(rotation), &sections(&["hull"]), "base");
            assert!(
                errors(&issues)
                    .iter()
                    .any(|issue| issue.message.contains("rotation")),
                "rotation {rotation:?} must be a lint error: {issues:?}"
            );
        }
    }

    /// A bay's cadence is the one number its whole launcher is built from.
    ///
    /// `1.0 / fire_rate` has no usable answer off the positive finite line, and
    /// the runtime withholds the launcher outright rather than inventing one -
    /// so a bay authored this way never fires, and the author hears it here.
    #[test]
    fn a_torpedo_bay_without_a_usable_fire_rate_is_an_error() {
        use nova_ship::prelude::BaseSectionConfig;

        let bay = |fire_rate: f32| SectionConfig {
            base: BaseSectionConfig {
                id: "bay".to_string(),
                ..default()
            },
            kind: SectionKind::Torpedo(TorpedoSectionConfig {
                fire_rate,
                ..default()
            }),
        };

        assert!(
            errors(&lint_section_config(&bay(1.0), "s")).is_empty(),
            "fixture guard: a real cadence lints clean"
        );

        for rate in [0.0, -1.0, f32::NAN, f32::INFINITY] {
            let issues = lint_section_config(&bay(rate), "s");
            assert!(
                errors(&issues)
                    .iter()
                    .any(|issue| issue.message.contains("fire_rate")),
                "fire_rate {rate} must be a lint error: {issues:?}"
            );
        }
    }

    /// A CATALOG ship is linted where it is authored: its own section
    /// prototypes must resolve. This is what keeps a hull checked once eleven
    /// scenarios have stopped inlining it.
    #[test]
    fn a_catalog_ship_is_linted_where_it_is_authored() {
        let ship = |proto: &str| ShipDesignPrototype {
            id: "block_gunship".to_string(),
            name: "Gunship".to_string(),
            design: ShipDesign {
                sections: vec![SpaceshipSectionConfig {
                    id: "fuselage".to_string(),
                    position: Vec3::ZERO,
                    rotation: Quat::IDENTITY,
                    source: SectionSource::prototype(proto),
                }],
                ..default()
            },
        };

        let issues = lint_ship_design_config(&ship("no_such_proto"), &sections(&["hull"]), "base");
        assert_eq!(issues.len(), 1, "{issues:?}");
        assert!(issues[0].message.contains("no_such_proto"));

        assert!(lint_ship_design_config(&ship("hull"), &sections(&["hull"]), "base").is_empty());
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
        let reload = |delay, amount| ReloadConfig::Batch(SectionReloadConfig { delay, amount });
        let check = |capacity, reload| {
            let mut issues = Vec::new();
            check_reload_config("weapon", capacity, reload, "mod", &mut issues);
            issues
        };
        let rounds = AmmoCapacity::Limited;

        assert!(check(rounds(500), reload(3.0, 200)).is_empty());
        for issues in [
            check(AmmoCapacity::Unlimited, reload(3.0, 200)),
            check(rounds(0), reload(3.0, 200)),
            check(rounds(500), reload(0.0, 200)),
            check(rounds(500), reload(f32::NAN, 200)),
            check(rounds(500), reload(3.0, 0)),
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
        let ShipDesignSource::Inline(design) = &mut ship.design else {
            unreachable!()
        };
        design.sections.push(SpaceshipSectionConfig {
            id: "second".to_string(),
            position: Vec3::X,
            rotation: Quat::IDENTITY,
            source: SectionSource::prototype("empty"),
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
                    center: Meters3::ZERO,
                    inner: Meters(100.0),
                    outer: Meters(200.0),
                    y_min: Meters(-10.0),
                    y_max: Meters(10.0),
                },
                template,
                asteroid_radius: None,
                asteroid_kinds: vec![],
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
                    position: Meters3::ZERO,
                    rotation: Quat::IDENTITY,
                },
                kind: ScenarioObjectKind::Spaceship(SpaceshipConfig {
                    design: ShipDesignSource::Inline(ShipDesign {
                        sections: vec![
                            SpaceshipSectionConfig {
                                id: "a".to_string(),
                                position: Vec3::ZERO,
                                rotation: Quat::IDENTITY,
                                source: SectionSource::prototype("known"),
                            },
                            SpaceshipSectionConfig {
                                id: "b".to_string(),
                                position: tube_pos,
                                rotation: Quat::IDENTITY,
                                source: SectionSource::prototype("known"),
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
            };

        let ship = |a: SpaceshipSectionConfig, b: SpaceshipSectionConfig| {
            EventActionConfig::SpawnScenarioObject(ScenarioObjectConfig {
                base: BaseScenarioObjectConfig {
                    id: "ship".to_string(),
                    name: "ship".to_string(),
                    position: Meters3::ZERO,
                    rotation: Quat::IDENTITY,
                },
                kind: ScenarioObjectKind::Spaceship(SpaceshipConfig {
                    design: ShipDesignSource::Inline(ShipDesign {
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
                position: Meters3::ZERO,
                rotation: Quat::IDENTITY,
            },
            kind: ScenarioObjectKind::Spaceship(SpaceshipConfig {
                design: ShipDesignSource::Inline(ShipDesign {
                    sections: vec![
                        SpaceshipSectionConfig {
                            id: "left".to_string(),
                            position: Vec3::ZERO,
                            rotation: Quat::IDENTITY,
                            source: SectionSource::prototype("left"),
                        },
                        SpaceshipSectionConfig {
                            id: "right".to_string(),
                            position: Vec3::X * 0.5,
                            rotation: Quat::IDENTITY,
                            source: SectionSource::prototype("right"),
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

    /// A hull whose link-point graph does not derive still has AUTHORED mates,
    /// and a seam the author mated is not an overlap.
    ///
    /// The fault such a hull HAS - here a disconnected component - is reported
    /// once, in the words that name it. Reading the mate set of a hull that
    /// fails to derive as EMPTY instead reported every intentional interface as
    /// interpenetration, which is both wrong and the loudest thing the author
    /// would have seen. What genuinely mates with nothing and is buried anyway
    /// is still reported.
    #[test]
    fn a_hull_that_fails_the_graph_still_exempts_its_authored_mates() {
        use nova_ship::prelude::{BaseSectionConfig, HullSectionConfig, LinkPoint};

        let box_collider = Some(SectionCollider::Cuboid {
            size: Vec3::splat(2.0),
        });
        let mating = |id: &str, normal: Vec3| SectionConfig {
            base: BaseSectionConfig {
                id: id.to_string(),
                collider: box_collider,
                link_points: vec![LinkPoint {
                    id: "mate".to_string(),
                    position: normal * 0.25,
                    normal,
                }],
                ..default()
            },
            kind: SectionKind::Hull(HullSectionConfig::default()),
        };
        // No sockets at all: it can mate with nothing, so it stands in a
        // component of its own and the graph never derives.
        let loose = SectionConfig {
            base: BaseSectionConfig {
                id: "loose".to_string(),
                collider: box_collider,
                ..default()
            },
            kind: SectionKind::Hull(HullSectionConfig::default()),
        };
        let configs = [mating("left", Vec3::X), mating("right", Vec3::NEG_X), loose];

        let section = |id: &str, prototype: &str, position: Vec3| SpaceshipSectionConfig {
            id: id.to_string(),
            position,
            rotation: Quat::IDENTITY,
            source: SectionSource::prototype(prototype),
        };
        let ship = ShipDesignPrototype {
            id: "drifter".to_string(),
            name: "Drifter".to_string(),
            design: ShipDesign {
                sections: vec![
                    // Mated, and deliberately interlocking: half a cell apart
                    // with two-cell boxes.
                    section("left", "left", Vec3::ZERO),
                    section("right", "right", Vec3::X * 0.5),
                    // Mated with nothing, and buried in each other.
                    section("loose_a", "loose", Vec3::X * 50.0),
                    section("loose_b", "loose", Vec3::X * 50.25),
                ],
                ..default()
            },
        };

        let issues = lint_ship_design_config(&ship, &KnownSections::from_configs(&configs), "base");
        let overlaps: Vec<_> = issues
            .iter()
            .filter(|issue| issue.message.contains("overlap"))
            .collect();
        assert_eq!(overlaps.len(), 1, "{issues:?}");
        assert!(
            overlaps[0].message.contains("'loose_a'") && overlaps[0].message.contains("'loose_b'"),
            "the mated seam is exempt, the buried pair is not: {issues:?}"
        );
        assert!(
            issues
                .iter()
                .any(|issue| issue.message.contains("disconnected")),
            "the hull still hears the fault it has: {issues:?}"
        );
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
                    id: "main".to_string(),
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

        // A muzzle id is the handle a patch and an editor row hold a barrel
        // by. Two barrels answering to one name is a turret no patch can
        // address, and an unnamed one is a barrel nothing can reach.
        let mut left = joint(None, None, None, true, vec![]);
        left.muzzle.as_mut().unwrap().id = "left".to_string();
        let mut right = joint(None, None, None, true, vec![]);
        right.muzzle.as_mut().unwrap().id = "left".to_string();
        let twin = joint(Some(Vec3::Y), None, None, false, vec![left, right.clone()]);
        assert!(
            errors(&lint_section_config(&turret(twin), "s"))
                .iter()
                .any(|i| i.message.contains("share the id 'left'")),
            "a duplicated muzzle id is an error"
        );

        let mut nameless = joint(None, None, None, true, vec![]);
        nameless.muzzle.as_mut().unwrap().id = "  ".to_string();
        let bad = joint(Some(Vec3::Y), None, None, false, vec![nameless]);
        assert!(
            errors(&lint_section_config(&turret(bad), "s"))
                .iter()
                .any(|i| i.message.contains("empty id")),
            "and so is a barrel with no name at all"
        );

        // The twin that IS authorable: two barrels, two ids.
        let mut port = joint(None, None, None, true, vec![]);
        port.muzzle.as_mut().unwrap().id = "left".to_string();
        let named = joint(Some(Vec3::Y), None, None, false, vec![port, right]);
        let mut named = turret(named);
        let SectionKind::Turret(config) = &mut named.kind else {
            panic!("a turret");
        };
        config.root.children[1].muzzle.as_mut().unwrap().id = "right".to_string();
        assert!(lint_section_config(&named, "s").is_empty());
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
            catalog.get("contested").unwrap().base.collider,
            Some(SectionCollider::Cuboid {
                size: Vec3::splat(2.0)
            })
        );
    }
}
