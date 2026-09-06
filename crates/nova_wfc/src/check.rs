//! What a generated hull is checked against before anyone flies it.
//!
//! Two claims, and neither of them is "the tiles fit". The first is the GAME's
//! own: `lint_scenario` is the same function the `content lint` gate and the
//! runtime loader run, so a hull that clears it is a hull the game would
//! accept. The second is stronger than the game asks and belongs to the
//! generator: every contact a collapse leaves has to MATE. A section resting a
//! blind flank on a plate is legal content and a modelling fault, and it is
//! invisible in a screenshot.
//!
//! Findings, not panics. The caller decides: an example fails its run, the
//! editor writes a line and refuses the hull.

use std::collections::HashSet;

use bevy::prelude::*;
use nova_events::prelude::Meters3;
use nova_gameplay::prelude::AssetRef;
use nova_scenario::prelude::{
    lint_scenario, BaseScenarioObjectConfig, EventActionConfig, EventConfig, KnownSections,
    KnownShips, LintSeverity, ScenarioConfig, ScenarioEventConfig, ScenarioObjectConfig,
    ScenarioObjectKind, SectionSource, ShipHull, ShipSource, SpaceshipConfig, SpaceshipController,
};
use nova_ship::prelude::{
    derive_link_point_graph, GameSections, LinkPointRef, PlacedSectionLinkPoints, SectionConfig,
};

use crate::{grid::FACES, tiles::rotated_half_extents};

/// Slack for the flush test. Wider than the grid's own epsilon: two bodies that
/// were placed a quantum apart are still touching.
const CONTACT_EPSILON: f32 = 1e-3;
/// How far past a socket to look for the body it should have mated.
const SOCKET_PROBE: f32 = 0.05;

/// One section as the graph derivation and the contact test both want it.
pub struct Placed<'a> {
    /// The catalog prototype this section is an instance of.
    pub config: &'a SectionConfig,
    /// Its place in ship space.
    pub position: Vec3,
    /// Its orientation in ship space.
    pub rotation: Quat,
    /// Centre and half-extents of the body, in ship space.
    pub body: (Vec3, Vec3),
}

/// Resolve a hull's sections against the catalog, or say which id did not
/// resolve.
pub fn place<'a>(ship: &ShipHull, sections: &'a GameSections) -> Result<Vec<Placed<'a>>, String> {
    ship.sections
        .iter()
        .map(|section| {
            let SectionSource::Prototype(id) = &section.source else {
                return Err(format!(
                    "section '{}' is inline, and a generated hull is prototypes only",
                    section.id
                ));
            };
            let config = sections
                .get_section(id)
                .ok_or_else(|| format!("no section prototype '{id}'"))?;
            let half =
                rotated_half_extents(config.base.collider.unwrap_or_default(), section.rotation);
            Ok(Placed {
                config,
                position: section.position,
                rotation: section.rotation,
                body: (section.position, half),
            })
        })
        .collect()
}

/// The face of `a` that `b` lies flush against, or `None` if the two bodies do
/// not meet face to face.
///
/// Face to face means what it says: flush on ONE axis and overlapping on the
/// other two. Two sections that share only an edge or a corner are not in
/// contact, and neither is a pair that meets on two axes at once.
fn contact_face(a: (Vec3, Vec3), b: (Vec3, Vec3)) -> Option<usize> {
    let delta = (b.0 - a.0).to_array();
    let reach = (a.1 + b.1).to_array();
    let mut face = None;
    for axis in 0..3 {
        let gap = delta[axis].abs() - reach[axis];
        if gap > CONTACT_EPSILON {
            return None;
        }
        if gap >= -CONTACT_EPSILON {
            if face.is_some() {
                return None;
            }
            face = Some(axis * 2 + usize::from(delta[axis] < 0.0));
        }
    }
    face
}

/// Whether a placed section carries a socket on the cell face `face`, in ship
/// space. Read off the placement rather than off the prototype, because which
/// face a socket ends up on is decided by the rotation.
fn offers(section: &Placed, face: usize) -> bool {
    section
        .config
        .base
        .link_points
        .iter()
        .any(|point| (section.rotation * point.normal).dot(FACES[face]) > 0.5)
}

/// Whether a point is strictly inside a body.
fn body_holds((centre, half): (Vec3, Vec3), point: Vec3) -> bool {
    (point - centre)
        .abs()
        .cmplt(half - Vec3::splat(CONTACT_EPSILON))
        .all()
}

/// Every contact on a hull that does NOT mate, spelled by section id.
///
/// Two bodies flush without a socket between them are EXEMPT. Two hull cubes
/// standing shoulder to shoulder are the ordinary case, and demanding a mate
/// there would be demanding a socket on nothing. What is NOT exempt is a socket
/// on one side and a bare face on the other: that is a plug pressed into a
/// blank face, and both halves of this function refuse it wherever it appears.
///
/// `exempt` names the pairs the caller has already accounted for - a
/// post-collapse stamp whose wider face rests flush against tiles that expose
/// no matching socket, which the game accepts and this stricter check would
/// otherwise flag.
pub fn unmated_contacts(
    placed: &[Placed],
    ship: &ShipHull,
    exempt: &dyn Fn(usize, usize) -> bool,
) -> Result<Vec<String>, String> {
    let points: Vec<PlacedSectionLinkPoints> = placed
        .iter()
        .map(|section| PlacedSectionLinkPoints {
            position: section.position,
            rotation: section.rotation,
            link_points: &section.config.base.link_points,
        })
        .collect();
    let mates = derive_link_point_graph(&points)
        .map_err(|error| format!("the collapse built no graph: {error:?}"))?;
    let mated: HashSet<(usize, usize)> = mates
        .iter()
        .map(|mate| (mate.a.section_index, mate.b.section_index))
        .collect();

    let mut unmated = Vec::new();
    for (a, first) in placed.iter().enumerate() {
        for (b, second) in placed.iter().enumerate().skip(a + 1) {
            let Some(face) = contact_face(first.body, second.body) else {
                continue;
            };
            if exempt(a, b) {
                continue;
            }
            if !offers(first, face) && !offers(second, face ^ 1) {
                continue;
            }
            if !mated.contains(&(a, b)) && !mated.contains(&(b, a)) {
                unmated.push(format!(
                    "'{}' and '{}' are flush and do not mate",
                    ship.sections[a].id, ship.sections[b].id
                ));
            }
        }
    }

    // And the same claim from the other end: a socket may point into vacuum,
    // but it may NOT point into another section's body without mating it. That
    // is what a connection which does not match looks like from the link
    // points' side - a plug pressed into a blank face - and the pair test above
    // cannot see it, because two bodies can be flush over a whole face while
    // the sockets that were supposed to answer each other sit somewhere else on
    // it.
    let answered: HashSet<LinkPointRef> = mates.iter().flat_map(|mate| [mate.a, mate.b]).collect();
    for (index, section) in placed.iter().enumerate() {
        for (socket, point) in section.config.base.link_points.iter().enumerate() {
            let reference = LinkPointRef {
                section_index: index,
                link_point_index: socket,
            };
            if answered.contains(&reference) {
                continue;
            }
            let probe = section.position
                + section.rotation * point.position
                + section.rotation * point.normal * SOCKET_PROBE;
            let Some(other) = placed
                .iter()
                .position(|body| body_holds(body.body, probe) && !std::ptr::eq(body, section))
            else {
                continue;
            };
            if exempt(index, other) {
                continue;
            }
            unmated.push(format!(
                "'{}' socket '{}' presses into '{}' with nothing to mate",
                ship.sections[index].id, point.id, ship.sections[other].id
            ));
        }
    }

    Ok(unmated)
}

/// Run a scenario carrying generated hulls through the game's OWN content gate.
///
/// Nothing here re-implements a check: `lint_scenario` is the same function the
/// `content lint` gate and the runtime loader run, so what comes back is
/// exactly what the game would refuse.
pub fn lint_errors(scenario: &ScenarioConfig, sections: &GameSections) -> Vec<String> {
    let known = KnownSections::from_configs(sections.iter());
    lint_scenario(
        scenario,
        &known,
        &KnownShips::default(),
        &HashSet::from([scenario.id.clone()]),
        // A generated hull carries geometry, never dialogue: the throwaway
        // scenario this gate wraps it in authors no cue, so no channel has to
        // resolve.
        &HashSet::new(),
    )
    .into_iter()
    .filter(|issue| issue.severity == LintSeverity::Error)
    .map(|issue| issue.message)
    .collect()
}

/// The id and display name the throwaway lint scenario wears.
///
/// A hull is not a scenario, and the gate only reads scenarios - so one is
/// built around the hull, run, and dropped. It never reaches content.
const LINT_SCENARIO_ID: &str = "generated_hull";

/// Run ONE hull through the game's own content gate.
///
/// The caller that has a hull and no scenario - the editor's Generate verb -
/// asks this. It wraps the hull in the smallest scenario that can carry it and
/// hands back what [`lint_errors`] found, so a hull is refused by exactly the
/// rules that would refuse it in a mod file.
pub fn hull_errors(hull: &ShipHull, sections: &GameSections) -> Vec<String> {
    let spawn = EventActionConfig::SpawnScenarioObject(ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: LINT_SCENARIO_ID.to_string(),
            name: "Generated Hull".to_string(),
            position: Meters3::ZERO,
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Spaceship(SpaceshipConfig {
            allegiance: None,
            controller: SpaceshipController::None,
            hull: ShipSource::Inline(hull.clone()),
            ..default()
        }),
    });
    let scenario = ScenarioConfig {
        events: vec![ScenarioEventConfig {
            label: None,
            name: EventConfig::OnStart,
            once: false,
            filters: vec![],
            actions: vec![spawn],
        }],
        ..ScenarioConfig::new(
            LINT_SCENARIO_ID.to_string(),
            "Generated Hull".to_string(),
            AssetRef::from(LINT_CUBEMAP),
        )
    };
    lint_errors(&scenario, sections)
}

/// The skybox the throwaway scenario names. The gate does not read it; a
/// scenario cannot be built without one.
const LINT_CUBEMAP: &str = "base/textures/cubemap.png";
