//! Where the armed prototype would land: the socket under the pointer, the pose
//! that mates onto it, and why a placement is refused.
//!
//! Link points are the sole source of structural adjacency, so this module
//! never invents one from geometry; collider bounds appear only in the overlap
//! refusal, exactly as the ship lint uses them. Pure over its inputs, so the
//! ghost the builder sees and the click that commits share one answer.

use bevy::prelude::*;
use nova_ship::prelude::*;

/// How far two collider boxes must interpenetrate before it counts as overlap.
/// Mated parts touch exactly, and a float error at the seam must not read as a
/// collision.
const OVERLAP_EPSILON: f32 = 1e-3;

/// One section already on the preview ship, in ship-root space.
#[derive(Clone, Debug)]
pub(crate) struct PlacedSection {
    /// Section origin.
    pub(crate) position: Vec3,
    /// Section rotation.
    pub(crate) rotation: Quat,
    /// Section-local sockets.
    pub(crate) link_points: Vec<LinkPoint>,
    /// Authored collider, for the overlap refusal only.
    pub(crate) collider: SectionCollider,
}

/// Why a placement cannot be committed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Refusal {
    /// The section under the pointer has no sockets to mate with.
    NoTargetSockets,
    /// The part being placed has no sockets of its own.
    NoPartSockets,
    /// The chosen socket is already mated to another section.
    Occupied,
    /// The placement would leave a socket with two suitors, which the runtime
    /// graph rejects for the whole ship.
    Ambiguous,
    /// The part would interpenetrate a section it does not mate with.
    Overlap,
}

impl Refusal {
    /// What the builder is told.
    pub(crate) fn message(self) -> &'static str {
        match self {
            Self::NoTargetSockets => "no socket here",
            Self::NoPartSockets => "this part has no sockets",
            Self::Occupied => "socket occupied",
            Self::Ambiguous => "socket is ambiguous",
            Self::Overlap => "parts would overlap",
        }
    }
}

/// What a click would build.
#[derive(Clone, Debug)]
pub(crate) struct Placement {
    /// Pose of the part in ship-root space.
    pub(crate) transform: Transform,
    /// Index of the socket on the placed part that does the mating.
    pub(crate) source: usize,
    /// Index of the socket it mates onto, on the hovered section.
    pub(crate) target: usize,
    /// `None` when the placement is legal.
    pub(crate) refusal: Option<Refusal>,
}

/// Solve the placement of a part with `part_link_points` onto the section
/// `hovered` of `ship`, aiming at the socket nearest the pointer's `hit`.
///
/// `source` and `roll` are the builder's two choices - which of the part's
/// sockets does the mating, and how far it is rolled about the mating axis -
/// and both wrap, so a caller can just count up.
pub(crate) fn solve(
    ship: &[PlacedSection],
    hovered: usize,
    hit: Vec3,
    part_link_points: &[LinkPoint],
    part_collider: SectionCollider,
    source: usize,
    roll: u32,
) -> Placement {
    let target_section = &ship[hovered];
    let Some((target, target_position, target_normal)) = nearest_socket(target_section, hit) else {
        return Placement {
            transform: Transform::from_translation(hit),
            source: 0,
            target: 0,
            refusal: Some(Refusal::NoTargetSockets),
        };
    };
    if part_link_points.is_empty() {
        return Placement {
            transform: Transform::from_translation(target_position),
            source: 0,
            target,
            refusal: Some(Refusal::NoPartSockets),
        };
    }

    let source = source % part_link_points.len();
    let (position, rotation) = snap_placement(
        target_position,
        target_normal,
        &part_link_points[source],
        roll,
    );
    let transform = Transform::from_translation(position).with_rotation(rotation);

    Placement {
        transform,
        source,
        target,
        refusal: refuse(
            ship,
            hovered,
            target,
            part_link_points,
            part_collider,
            transform,
        ),
    }
}

/// The socket of `section` nearest `hit`, with its ship-root position and
/// outward normal.
fn nearest_socket(section: &PlacedSection, hit: Vec3) -> Option<(usize, Vec3, Vec3)> {
    section
        .link_points
        .iter()
        .enumerate()
        .map(|(index, point)| {
            (
                index,
                section.position + section.rotation * point.position,
                (section.rotation * point.normal).normalize_or(Vec3::Z),
            )
        })
        .min_by(|(_, a, _), (_, b, _)| {
            a.distance_squared(hit)
                .partial_cmp(&b.distance_squared(hit))
                .unwrap_or(std::cmp::Ordering::Equal)
        })
}

/// Why this pose cannot be committed, if it cannot.
fn refuse(
    ship: &[PlacedSection],
    hovered: usize,
    target: usize,
    part_link_points: &[LinkPoint],
    part_collider: SectionCollider,
    transform: Transform,
) -> Option<Refusal> {
    // A socket already mated to a neighbour cannot take a second part.
    let existing: Vec<PlacedSectionLinkPoints<'_>> = ship.iter().map(placed).collect();
    let taken = candidate_link_point_mates(&existing)
        .into_iter()
        .any(|mate| {
            [mate.a, mate.b].iter().any(|reference| {
                reference.section_index == hovered && reference.link_point_index == target
            })
        });
    if taken {
        return Some(Refusal::Occupied);
    }

    // With the part in place, no socket may have more than one suitor: the
    // runtime derivation rejects the WHOLE ship over one ambiguous socket, so
    // the editor must never build one.
    let ghost = PlacedSection {
        position: transform.translation,
        rotation: transform.rotation,
        link_points: part_link_points.to_vec(),
        collider: part_collider,
    };
    let mut assembled = existing;
    assembled.push(placed(&ghost));
    let mates = candidate_link_point_mates(&assembled);
    let ambiguous = mates
        .iter()
        .flat_map(|mate| [mate.a, mate.b])
        .fold(
            std::collections::BTreeMap::new(),
            |mut counts, reference| {
                *counts.entry(reference).or_insert(0usize) += 1;
                counts
            },
        )
        .into_values()
        .any(|count| count > 1);
    if ambiguous {
        return Some(Refusal::Ambiguous);
    }

    // Bounds are allowed to interpenetrate exactly where a mate says the
    // interface is intentional - the same rule the ship lint applies.
    let ghost_index = assembled.len() - 1;
    let mated: Vec<usize> = mates
        .iter()
        .filter_map(|mate| match (mate.a.section_index, mate.b.section_index) {
            (a, b) if a == ghost_index => Some(b),
            (a, b) if b == ghost_index => Some(a),
            _ => None,
        })
        .collect();
    let overlapping = ship
        .iter()
        .enumerate()
        .any(|(index, section)| !mated.contains(&index) && overlaps(section, &ghost));
    overlapping.then_some(Refusal::Overlap)
}

fn placed(section: &PlacedSection) -> PlacedSectionLinkPoints<'_> {
    PlacedSectionLinkPoints {
        position: section.position,
        rotation: section.rotation,
        link_points: &section.link_points,
    }
}

/// Rotation-agnostic AABB interpenetration, the conservative broad-phase the
/// ship lint uses.
fn overlaps(a: &PlacedSection, b: &PlacedSection) -> bool {
    let distance = (a.position - b.position).abs();
    let sum = a.collider.aabb_half_extents() + b.collider.aabb_half_extents();
    distance.x + OVERLAP_EPSILON < sum.x
        && distance.y + OVERLAP_EPSILON < sum.y
        && distance.z + OVERLAP_EPSILON < sum.z
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cube(position: Vec3) -> PlacedSection {
        PlacedSection {
            position,
            rotation: Quat::IDENTITY,
            link_points: unit_cube_link_points(),
            collider: SectionCollider::default(),
        }
    }

    fn solve_on(ship: &[PlacedSection], hovered: usize, hit: Vec3, roll: u32) -> Placement {
        solve(
            ship,
            hovered,
            hit,
            &unit_cube_link_points(),
            SectionCollider::default(),
            1,
            roll,
        )
    }

    /// The socket nearest the pointer is the one that mates, so a click builds
    /// against the FACE the builder aimed at.
    #[test]
    fn the_socket_under_the_pointer_is_the_one_that_mates() {
        let ship = [cube(Vec3::ZERO)];

        let placement = solve_on(&ship, 0, Vec3::new(0.5, 0.1, 0.0), 0);
        assert_eq!(placement.refusal, None);
        assert!(placement.transform.translation.abs_diff_eq(Vec3::X, 1e-5));

        // The opposite face of the same cube sends the part the other way.
        let placement = solve_on(&ship, 0, Vec3::new(0.0, 0.0, -0.5), 0);
        assert_eq!(placement.refusal, None);
        assert!(placement
            .transform
            .translation
            .abs_diff_eq(Vec3::NEG_Z, 1e-5));
    }

    /// A socket that already carries a neighbour is refused: the ship it would
    /// build is one the runtime derivation rejects outright.
    #[test]
    fn an_occupied_socket_is_refused() {
        let ship = [cube(Vec3::ZERO), cube(Vec3::X)];

        let placement = solve_on(&ship, 0, Vec3::new(0.5, 0.0, 0.0), 0);
        assert_eq!(placement.refusal, Some(Refusal::Occupied));

        // A free face of the same section still takes the part - the refusal is
        // about the socket, not the section.
        let placement = solve_on(&ship, 0, Vec3::new(0.0, 0.5, 0.0), 0);
        assert_eq!(placement.refusal, None);
    }

    /// Two sections whose free sockets meet at one point: filling it would
    /// leave a socket with two suitors, which invalidates the whole graph.
    #[test]
    fn a_placement_that_would_be_ambiguous_is_refused() {
        // Two cubes two units apart on X, with a one-unit gap between them: a
        // part in the gap mates BOTH, and each of its side sockets is then
        // claimed once - that is legal. Stack a second gap-filler claim by
        // hovering the same gap from the other side and the socket doubles up.
        let ship = [cube(Vec3::ZERO), cube(Vec3::X * 2.0), cube(Vec3::X)];
        // The middle cube already fills the gap, so aiming at either inner face
        // is occupied rather than ambiguous.
        let placement = solve_on(&ship, 0, Vec3::new(0.5, 0.0, 0.0), 0);
        assert_eq!(placement.refusal, Some(Refusal::Occupied));

        // Ambiguity proper: a part whose OWN sockets both land on free sockets
        // of two different sections at once. A cube placed between two cubes
        // that are two apart mates two sockets - legal - so the ambiguous case
        // needs a part with two coincident sockets on one side.
        let doubled = vec![
            LinkPoint {
                id: "a".to_string(),
                position: Vec3::NEG_X * 0.5,
                normal: Vec3::NEG_X,
            },
            LinkPoint {
                id: "b".to_string(),
                position: Vec3::NEG_X * 0.5,
                normal: Vec3::NEG_X,
            },
        ];
        let ship = [cube(Vec3::ZERO)];
        let placement = solve(
            &ship,
            0,
            Vec3::new(0.5, 0.0, 0.0),
            &doubled,
            SectionCollider::default(),
            0,
            0,
        );
        assert_eq!(placement.refusal, Some(Refusal::Ambiguous));
    }

    /// Bounds refuse a part that would sit inside another section it does not
    /// mate with; a mated interface is intentional and stays allowed.
    #[test]
    fn overlap_is_refused_but_a_mated_interface_is_not() {
        // A long part whose socket is on its face: mating it onto +X puts its
        // far end through the cube standing at +2X, which it does not mate.
        let long = SectionCollider::Cuboid {
            size: Vec3::new(4.0, 1.0, 1.0),
        };
        let socket = vec![LinkPoint {
            id: "in".to_string(),
            position: Vec3::NEG_X * 2.0,
            normal: Vec3::NEG_X,
        }];
        let ship = [cube(Vec3::ZERO), cube(Vec3::X * 3.0)];
        let placement = solve(&ship, 0, Vec3::new(0.5, 0.0, 0.0), &socket, long, 0, 0);
        assert_eq!(placement.refusal, Some(Refusal::Overlap));

        // The ordinary neighbour case - boxes meeting exactly at the seam - is
        // not overlap.
        let ship = [cube(Vec3::ZERO)];
        assert_eq!(
            solve_on(&ship, 0, Vec3::new(0.5, 0.0, 0.0), 0).refusal,
            None
        );
    }

    /// A part with no sockets cannot be placed at all, and neither can one
    /// aimed at a section that has none: link points are the only adjacency.
    #[test]
    fn a_part_or_target_without_sockets_is_refused() {
        let ship = [cube(Vec3::ZERO)];
        let placement = solve(
            &ship,
            0,
            Vec3::new(0.5, 0.0, 0.0),
            &[],
            SectionCollider::default(),
            0,
            0,
        );
        assert_eq!(placement.refusal, Some(Refusal::NoPartSockets));

        let socketless = [PlacedSection {
            link_points: vec![],
            ..cube(Vec3::ZERO)
        }];
        let placement = solve_on(&socketless, 0, Vec3::new(0.5, 0.0, 0.0), 0);
        assert_eq!(placement.refusal, Some(Refusal::NoTargetSockets));
    }

    /// The roll is a quarter turn about the mating axis: the pose does not
    /// move, and the mate survives.
    #[test]
    fn rolling_turns_the_part_without_moving_it() {
        let ship = [cube(Vec3::ZERO)];
        let upright = solve_on(&ship, 0, Vec3::new(0.5, 0.0, 0.0), 0);
        let rolled = solve_on(&ship, 0, Vec3::new(0.5, 0.0, 0.0), 1);

        assert_eq!(rolled.refusal, None);
        assert!(rolled
            .transform
            .translation
            .abs_diff_eq(upright.transform.translation, 1e-5));
        assert!(
            (upright
                .transform
                .rotation
                .angle_between(rolled.transform.rotation)
                - std::f32::consts::FRAC_PI_2)
                .abs()
                < 1e-4
        );
    }

    /// The source socket wraps, so a caller can count up forever without
    /// knowing how many sockets the part carries.
    #[test]
    fn the_source_socket_wraps() {
        let ship = [cube(Vec3::ZERO)];
        let points = unit_cube_link_points();
        let first = solve(
            &ship,
            0,
            Vec3::new(0.5, 0.0, 0.0),
            &points,
            SectionCollider::default(),
            1,
            0,
        );
        let wrapped = solve(
            &ship,
            0,
            Vec3::new(0.5, 0.0, 0.0),
            &points,
            SectionCollider::default(),
            1 + points.len(),
            0,
        );
        assert_eq!(first.source, wrapped.source);
        assert!(wrapped
            .transform
            .translation
            .abs_diff_eq(first.transform.translation, 1e-5));
    }
}
