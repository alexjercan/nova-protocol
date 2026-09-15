//! Structural checks over a scenario's ships and their section configs.

use bevy::prelude::{UVec3, Vec3};
use nova_events::units::prelude::*;
use nova_gameplay::prelude::NarrativeChannelConfig;
use nova_ship::prelude::{
    candidate_link_point_mates, derive_link_point_graph, duplicate_muzzle_id, muzzle_ids,
    section_colliders_overlap, AmmoCapacity, ControllerSectionConfig, LinkPointGraphError,
    LinkPointRef, PlacedSectionCollider, PlacedSectionLinkPoints, RailgunSectionConfig,
    ReloadConfig, SectionCollider, SectionConfig, SectionFootprint, SectionKind, ShipGrammarConfig,
    TorpedoSectionConfig, TurretJoint, TurretSectionConfig, MAX_GRAMMAR_CELLS,
};

use super::{KnownSections, KnownShipDesigns, LintIssue, LintSeverity};
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

/// Static well-formedness of one authored ship GRAMMAR: every prototype it
/// names has to resolve, and its grid has to be big enough to hold the seeds
/// it lays down and small enough to be collapsed at all.
///
/// The id checks are the authoring rule in force - an unrecognized id is an
/// error at lint, then again at load - and they matter more here than
/// elsewhere, because a grammar is read by a GENERATOR: a keel role that
/// resolves to nothing is a hull with no spine, and a drawable part that does
/// not exist is a hole in the draw that only shows up as a thinner ship.
///
/// Whether a named prototype can actually be a TILE is not checked here. That
/// is a geometric question about link points, the generator answers it by
/// simply not offering the part, and it needs the collapse's own machinery to
/// ask. A part on the list that cannot tile is a wasted line, not a broken
/// ship.
pub fn lint_grammar_config(
    grammar: &ShipGrammarConfig,
    sections: &KnownSections,
    source: &str,
) -> Vec<LintIssue> {
    let mut issues = Vec::new();
    let mut error = |message: String| {
        issues.push(LintIssue {
            severity: LintSeverity::Error,
            scenario: grammar.id.clone(),
            message,
        });
    };

    for id in grammar.named_sections() {
        if !sections.contains(id) {
            error(format!(
                "grammar '{}' in {source} names section prototype '{id}', which no visible \
                 catalog holds",
                grammar.id
            ));
        }
    }

    if grammar.parts.is_empty() {
        error(format!(
            "grammar '{}' in {source} draws from no parts, so a collapse under it can only \
             ever produce its own seeded keel",
            grammar.id
        ));
    }
    for part in &grammar.parts {
        if !(part.weight.is_finite() && part.weight > 0.0) {
            error(format!(
                "grammar '{}' in {source} gives part '{}' weight {}, which is never drawn - \
                 leave the part out instead",
                grammar.id, part.prototype, part.weight
            ));
        }
    }

    // Three cells is the floor the SKIN sets: a plate is a flat run only where
    // it has neighbours on all four sides, so a surface narrower than this has
    // no interior and the hull comes out all rim.
    let grid = grammar.grid;
    for (axis, cells) in [
        ("half_width", grid.half_width),
        ("height", grid.height),
        ("length", grid.length),
    ] {
        if cells < 3 {
            error(format!(
                "grammar '{}' in {source} is {cells} cell(s) in {axis}; a hull needs at least \
                 3 there or its skin is all rim and no interior",
                grammar.id
            ));
        }
    }
    // A domain per cell is laid down before the collapse can refuse anything,
    // so the size is an allocation the generator is asked to make on trust.
    if grid.cells() > MAX_GRAMMAR_CELLS {
        error(format!(
            "grammar '{}' in {source} asks for a {}x{}x{} grid, which is {} cells; the collapse \
             holds one domain per cell and stops at {MAX_GRAMMAR_CELLS}",
            grammar.id,
            grid.half_width,
            grid.height,
            grid.length,
            grid.cells(),
        ));
    }

    // The seeds' own footprints. The stern seed stands the drive one cell off
    // the centreline with its deck plate in front of it, and the bow gun
    // stands IN the keel column at the other end; both are as big as whatever
    // prototype the grammar named, so a floor on the axes cannot see them.
    // These are the three bounds `nova_wfc::runnable` refuses a grid with,
    // asked here so a grammar is refused where it is AUTHORED rather than when
    // a builder presses Generate.
    let drive = seeded_span(sections, &grammar.keel.stern_drive);
    for (axis, cells, wanted) in [
        ("across its half-width", grid.half_width, drive.x + 1),
        ("tall", grid.height, drive.y),
        ("long", grid.length, drive.z + 1),
    ] {
        if cells < wanted {
            error(format!(
                "grammar '{}' in {source} is {cells} cell(s) {axis} and its seeded stern drive \
                 '{}' needs {wanted}",
                grammar.id, grammar.keel.stern_drive
            ));
        }
    }
    if let Some(prototype) = grammar.keel.bow_gun.as_deref() {
        let bow = seeded_span(sections, prototype);
        if bow.x != 1 || bow.y != 1 {
            error(format!(
                "grammar '{}' in {source} seats '{prototype}' as its bow gun, but a spinal gun \
                 stands on the keel line and that one is {} cell(s) across and {} tall",
                grammar.id, bow.x, bow.y
            ));
        }
        if grid.length < bow.z + drive.z + 2 {
            error(format!(
                "grammar '{}' in {source} is {} cell(s) long, and its bow gun '{prototype}' and \
                 stern drive '{}' leave no keel between them",
                grammar.id, grid.length, grammar.keel.stern_drive
            ));
        }
    }

    for (field, value) in [
        ("base", grammar.vacuum.base),
        ("taper", grammar.vacuum.taper),
        ("stern", grammar.vacuum.stern),
        ("bow_taper", grammar.vacuum.bow_taper),
    ] {
        if !value.is_finite() || value < 0.0 {
            error(format!(
                "grammar '{}' in {source} prices vacuum.{field} at {value}; a vacuum weight is \
                 a non-negative number",
                grammar.id
            ));
        }
    }
    if grammar.vacuum.base <= 0.0 {
        error(format!(
            "grammar '{}' in {source} prices vacuum.base at 0, so the collapse can never leave \
             a cell empty and every hull is a solid block",
            grammar.id
        ));
    }

    issues
}

/// How many cells one seeded prototype spans, upright - the same reading the
/// generator takes.
///
/// `UVec3::ONE` for an id the catalog does not hold, because that id is
/// already an Error above and a bound derived from a guess would bury it under
/// a second finding about a size nobody authored.
fn seeded_span(sections: &KnownSections, id: &str) -> UVec3 {
    sections.get(id).map_or(UVec3::ONE, |section| {
        *SectionFootprint::from_collider(section.base.collider.unwrap_or_default())
    })
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

/// Well-formedness for one authored narrative channel.
///
/// A channel is pure presentation, so there is nothing here about what a line
/// MEANS - only the two ways a card can come out unreadable: a strength that
/// draws it at nothing, and a tag that takes the space beside the speaker
/// without saying anything.
///
/// Whether two channels are visibly different is deliberately NOT checked. Two
/// bands on one tone is a legitimate authoring choice (a mod may want four
/// channels and has four tones), and the tag is there to separate them.
pub fn lint_channel_config(channel: &NarrativeChannelConfig, source: &str) -> Vec<LintIssue> {
    let mut issues = Vec::new();
    let mut error = |message: String| {
        issues.push(LintIssue {
            severity: LintSeverity::Error,
            scenario: channel.id.clone(),
            message,
        });
    };

    if channel.id.trim().is_empty() {
        error(format!("a channel in {source} has an empty id"));
    }

    // Zero is excluded, not clamped: a channel drawn at nothing is a channel
    // whose lines never reach the player, which is a mistake rather than a
    // style. Above one is a card brighter than the HUD it sits in.
    if !(channel.signal_strength.is_finite()
        && channel.signal_strength > 0.0
        && channel.signal_strength <= 1.0)
    {
        error(format!(
            "channel '{}' in {source} has signal strength {}, which is outside (0, 1]",
            channel.id, channel.signal_strength
        ));
    }

    if channel
        .tag
        .as_ref()
        .is_some_and(|tag| tag.trim().is_empty())
    {
        error(format!(
            "channel '{}' in {source} authors an empty tag; omit the field for a channel \
             that needs no saying",
            channel.id
        ));
    }

    issues
}

#[cfg(test)]
mod tests {

    use bevy::prelude::*;
    use nova_ship::prelude::{GrammarGrid, GrammarVacuum, SectionConfigPatch, SectionReloadConfig};

    use super::*;
    use crate::lint::fixtures::*;

    /// A channel is pure presentation, so the only thing to check is that the
    /// presentation is authorable: a card drawn at nothing never reaches the
    /// player, and a tag that says nothing is a label on every line.
    #[test]
    fn a_channel_drawn_at_nothing_or_labelled_with_nothing_is_an_error() {
        use nova_gameplay::prelude::ChipTone;

        let good = NarrativeChannelConfig::new("work", ChipTone::Comms);
        assert!(
            lint_channel_config(&good, "test").is_empty(),
            "a plain channel must lint clean"
        );

        for bad in [
            NarrativeChannelConfig::new("", ChipTone::Comms),
            NarrativeChannelConfig::new("dark", ChipTone::Comms).with_signal_strength(0.0),
            NarrativeChannelConfig::new("loud", ChipTone::Comms).with_signal_strength(1.5),
            NarrativeChannelConfig::new("nameless", ChipTone::Comms).with_tag("  "),
        ] {
            let issues = lint_channel_config(&bad, "test");
            assert_eq!(issues.len(), 1, "{bad:?} -> {issues:?}");
            assert_eq!(issues[0].severity, LintSeverity::Error);
        }
    }

    /// The shape everything below bends: a grammar that names only prototypes
    /// its catalog holds, on a grid that fits its seeds, drawing one part at a
    /// real weight. If this ever fires, every assertion under it is measuring
    /// the fixture instead of the gate.
    #[test]
    fn a_well_formed_grammar_draws_nothing() {
        let issues = lint_grammar_config(&grammar(), &grammar_sections(), "test");
        assert!(issues.is_empty(), "{issues:?}");
    }

    /// The authoring rule, on every role a grammar seeds and on the draw: an id
    /// the catalog does not hold is an Error, named so the author can find it.
    /// A keel role that resolves to nothing is a hull with no spine.
    #[test]
    fn a_grammar_naming_a_prototype_no_catalog_holds_is_an_error() {
        let bend: [(&str, fn(&mut ShipGrammarConfig)); 6] = [
            ("keel.hull", |g| g.keel.hull = "ghost".to_string()),
            ("keel.bridge", |g| g.keel.bridge = "ghost".to_string()),
            ("keel.stern_deck", |g| {
                g.keel.stern_deck = "ghost".to_string()
            }),
            ("keel.stern_drive", |g| {
                g.keel.stern_drive = "ghost".to_string();
            }),
            ("keel.bow_gun", |g| {
                g.keel.bow_gun = Some("ghost".to_string());
            }),
            ("parts", |g| g.parts[0].prototype = "ghost".to_string()),
        ];
        for (role, bend) in bend {
            let mut config = grammar();
            bend(&mut config);
            let issues = lint_grammar_config(&config, &grammar_sections(), "test");
            let named: Vec<&LintIssue> = issues
                .iter()
                .filter(|issue| issue.message.contains("ghost"))
                .collect();
            assert_eq!(named.len(), 1, "{role} -> {issues:?}");
            assert_eq!(named[0].severity, LintSeverity::Error, "{role}");
        }
    }

    /// A draw is what the collapse rolls on. An empty one leaves it nothing to
    /// place but the seeded keel, and a part priced at a weight that is never
    /// drawn is a line that reads as a decision and is not one.
    #[test]
    fn a_grammar_that_draws_from_nothing_or_prices_a_part_at_nothing_is_an_error() {
        let mut empty = grammar();
        empty.parts.clear();
        let issues = lint_grammar_config(&empty, &grammar_sections(), "test");
        assert_eq!(issues.len(), 1, "{issues:?}");
        assert_eq!(issues[0].severity, LintSeverity::Error);

        for weight in [0.0, -1.0, f32::NAN, f32::INFINITY] {
            let mut config = grammar();
            config.parts[0].weight = weight;
            let issues = lint_grammar_config(&config, &grammar_sections(), "test");
            assert_eq!(issues.len(), 1, "weight {weight} -> {issues:?}");
            assert_eq!(issues[0].severity, LintSeverity::Error, "weight {weight}");
        }
    }

    /// The floor the SKIN sets. A surface needs neighbours on all four sides
    /// before it is a flat run, so a hull under three cells on any axis comes
    /// out all rim.
    #[test]
    fn a_grid_too_thin_to_have_an_interior_is_an_error() {
        let bend: [(&str, fn(&mut GrammarGrid)); 3] = [
            ("half_width", |grid| grid.half_width = 2),
            ("height", |grid| grid.height = 2),
            ("length", |grid| grid.length = 2),
        ];
        for (axis, bend) in bend {
            let mut config = grammar();
            bend(&mut config.grid);
            let issues = lint_grammar_config(&config, &grammar_sections(), "test");
            assert!(
                issues
                    .iter()
                    .any(|issue| issue.message.contains(axis)
                        && issue.severity == LintSeverity::Error),
                "{axis} -> {issues:?}"
            );
        }
    }

    /// The bounds the GENERATOR refuses a grid with, asked where the grammar is
    /// authored. A grid can clear the axis floor and still have nowhere to
    /// stand the drive the same grammar seeds - that is a fact about the named
    /// prototype's footprint, which a floor on the axes cannot see.
    #[test]
    fn a_grid_too_small_for_the_seeds_it_lays_is_an_error() {
        // A 5x5x3 capital drive wants 6 across, 5 tall and 4 long; the shipped
        // 4x5x11 grid is a cell short across and clears the other two.
        let mut capital = grammar();
        capital.keel.stern_drive = "capital_drive".to_string();
        let issues = lint_grammar_config(&capital, &grammar_sections(), "test");
        assert_eq!(issues.len(), 1, "{issues:?}");
        assert!(
            issues[0].message.contains("half-width") && issues[0].message.contains("capital_drive"),
            "{issues:?}"
        );

        // A spinal gun stands IN the keel column, so one wider or taller than a
        // single cell is refused by name rather than seeded crooked.
        let mut broad = grammar();
        broad.keel.bow_gun = Some("broad_lance".to_string());
        let issues = lint_grammar_config(&broad, &grammar_sections(), "test");
        assert_eq!(issues.len(), 1, "{issues:?}");
        assert!(issues[0].message.contains("broad_lance"), "{issues:?}");

        // Both seeds eat the same column, and what is left has to hold a keel.
        // A 4-long lance and a 1-long drive need 7; this grid is 6.
        let mut crowded = grammar();
        crowded.keel.bow_gun = Some("lance".to_string());
        crowded.grid.length = 6;
        let issues = lint_grammar_config(&crowded, &grammar_sections(), "test");
        assert_eq!(issues.len(), 1, "{issues:?}");
        assert!(issues[0].message.contains("no keel between"), "{issues:?}");

        // One more cell and the same pair fits.
        crowded.grid.length = 7;
        assert!(
            lint_grammar_config(&crowded, &grammar_sections(), "test").is_empty(),
            "7 cells is exactly enough for a 4-long gun, a 1-long drive and a keel"
        );
    }

    /// The bound the grid never had. The collapse lays down a domain per cell
    /// before it can refuse anything, so an authored size is an allocation
    /// taken on trust - and the product of three `u32` axes wraps.
    #[test]
    fn a_grid_too_big_to_be_collapsed_in_is_an_error() {
        // Exactly 2^32 cells: the size that used to multiply to zero.
        let mut wrapping = grammar();
        wrapping.grid = GrammarGrid {
            half_width: 2048,
            height: 2048,
            length: 1024,
        };
        assert_eq!(
            wrapping.grid.cells(),
            1 << 32,
            "the fixture has to be the size that wraps a u32, or it proves nothing"
        );
        let issues = lint_grammar_config(&wrapping, &grammar_sections(), "test");
        assert!(
            issues
                .iter()
                .any(|issue| issue.message.contains("holds one domain per cell")),
            "{issues:?}"
        );

        // The ceiling itself is clean, and one cell over it is not.
        let mut at_ceiling = grammar();
        at_ceiling.grid = GrammarGrid {
            half_width: 16,
            height: 16,
            length: 256,
        };
        assert_eq!(at_ceiling.grid.cells(), MAX_GRAMMAR_CELLS);
        assert!(
            lint_grammar_config(&at_ceiling, &grammar_sections(), "test").is_empty(),
            "the ceiling is a size that runs, not the first that does not"
        );
        at_ceiling.grid.length += 1;
        assert_eq!(
            lint_grammar_config(&at_ceiling, &grammar_sections(), "test").len(),
            1
        );
    }

    /// Vacuum is how emptiness is priced against the parts. A price that is not
    /// a number is not a price, and a base of zero means the collapse can never
    /// leave a cell empty - every hull comes out a solid block.
    #[test]
    fn a_vacuum_price_that_is_not_a_weight_is_an_error() {
        let bend: [(&str, fn(&mut GrammarVacuum, f32)); 4] = [
            ("base", |vacuum, value| vacuum.base = value),
            ("taper", |vacuum, value| vacuum.taper = value),
            ("stern", |vacuum, value| vacuum.stern = value),
            ("bow_taper", |vacuum, value| vacuum.bow_taper = value),
        ];
        for (field, bend) in bend {
            for value in [-1.0, f32::NAN] {
                let mut config = grammar();
                bend(&mut config.vacuum, value);
                let issues = lint_grammar_config(&config, &grammar_sections(), "test");
                assert!(
                    issues.iter().any(|issue| issue.message.contains(field)
                        && issue.severity == LintSeverity::Error),
                    "vacuum.{field} = {value} -> {issues:?}"
                );
            }
        }

        // Zero is a NUMBER, so only `base` refuses it: a taper of zero is a
        // hull that is evenly sparse, which is taste.
        let mut solid = grammar();
        solid.vacuum.base = 0.0;
        let issues = lint_grammar_config(&solid, &grammar_sections(), "test");
        assert_eq!(issues.len(), 1, "{issues:?}");
        assert!(issues[0].message.contains("solid block"), "{issues:?}");

        for zeroed in [
            |vacuum: &mut GrammarVacuum| vacuum.taper = 0.0,
            |vacuum: &mut GrammarVacuum| vacuum.stern = 0.0,
            |vacuum: &mut GrammarVacuum| vacuum.bow_taper = 0.0,
        ] {
            let mut config = grammar();
            zeroed(&mut config.vacuum);
            assert!(
                lint_grammar_config(&config, &grammar_sections(), "test").is_empty(),
                "an unpriced taper is a flat hull, not a broken one"
            );
        }
    }

    #[test]
    fn unknown_prototype_is_an_error() {
        let s = scenario(vec![spawn_ship("player", "no_such_proto")], vec![]);
        let issues = lint_scenario(
            &s,
            &sections(&["known_proto"]),
            &ships(&[]),
            &known(&["test_scenario"]),
            &base_channels(),
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
            &base_channels(),
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
            &base_channels(),
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
            &base_channels(),
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
            &base_channels(),
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
        let issues = lint_scenario(
            &scenario,
            &catalog,
            &ships(&[]),
            &known(&["test_scenario"]),
            &base_channels(),
        );
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
            &base_channels(),
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
            &base_channels(),
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
            &base_channels(),
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
        let issues = lint_scenario(
            &s,
            &sections(&[]),
            &ships(&[]),
            &known(&["test_scenario"]),
            &base_channels(),
        );
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
        let issues = lint_scenario(
            &s,
            &sections(&[]),
            &ships(&[]),
            &known(&["test_scenario"]),
            &base_channels(),
        );
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
        let issues = lint_scenario(
            &s,
            &sections(&[]),
            &ships(&[]),
            &known(&["test_scenario"]),
            &base_channels(),
        );
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
        let issues = lint_scenario(
            &s,
            &sections(&[]),
            &ships(&[]),
            &known(&["test_scenario"]),
            &base_channels(),
        );
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
        let issues = lint_scenario(
            &s,
            &sections(&[]),
            &ships(&[]),
            &known(&["test_scenario"]),
            &base_channels(),
        );
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
        let issues = lint_scenario(
            &s,
            &catalog,
            &ships(&[]),
            &known(&["test_scenario"]),
            &base_channels(),
        );
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
