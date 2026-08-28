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
    /// The direction the section fires, launches or exhausts, in its own frame,
    /// for the clearance refusal only. `None` for a part that does none of
    /// those. See `nova_ship`'s `exit_normal`.
    pub(crate) exit: Option<Vec3>,
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
    /// The part would stand in the space something has to fire, launch or
    /// exhaust through - its own, or one already on the ship.
    BlockedExit,
}

impl Refusal {
    /// What the builder is told: the fault, then the gesture that clears it.
    ///
    /// A VERDICT alone - "socket is ambiguous" - is a compiler error, and the
    /// editor already knows which key resolves each case: F cycles the socket,
    /// R rolls the part, Q picks another, Esc puts it down. A refusal that
    /// names one is a refusal a builder can act on without leaving the ghost.
    pub(crate) fn message(self) -> &'static str {
        match self {
            Self::NoTargetSockets => "nothing here to mate onto - aim at a part",
            Self::NoPartSockets => "this part has no sockets - Q picks another",
            Self::Occupied => "socket occupied - F cycles to a free one",
            Self::Ambiguous => "two sockets are equally close - F picks one",
            Self::Overlap => "parts would overlap - R rolls it clear",
            Self::BlockedExit => "this blocks an exit - R rolls it clear",
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
    part_exit: Option<Vec3>,
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

    // `source` is an OFFSET from the socket the part would naturally use, not
    // an absolute index: zero mounts the part AS DRAWN, and stepping the key
    // walks away from that. An absolute index made the default arbitrary - a
    // thruster's first socket is its +X face, so bolting one to a tail turned
    // it broadside until the builder found the right number of presses.
    let source =
        (natural_source(part_link_points, target_normal) + source) % part_link_points.len();
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
            part_exit,
            transform,
        ),
    }
}

/// The socket a part would naturally mate with on a face pointing
/// `target_normal`: the one facing most nearly the opposite way.
///
/// That socket is the one whose mate leaves the part in the orientation it was
/// AUTHORED in - an engine keeps its nozzle aft, a bay keeps its doors forward
/// - because opposing two already-opposed normals asks for no rotation at all.
///
/// It is the answer a builder means nine times in ten, so it is where the
/// socket cycle starts rather than somewhere the cycle has to reach.
fn natural_source(part_link_points: &[LinkPoint], target_normal: Vec3) -> usize {
    part_link_points
        .iter()
        .enumerate()
        .max_by(|(_, a), (_, b)| {
            let facing = |point: &LinkPoint| point.normal.dot(-target_normal);
            let radial = |point: &LinkPoint| point.position.cross(point.normal).length_squared();
            facing(a)
                .total_cmp(&facing(b))
                .then_with(|| radial(b).total_cmp(&radial(a)))
        })
        .map_or(0, |(index, _)| index)
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
    part_exit: Option<Vec3>,
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
        exit: part_exit,
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
    if overlapping {
        return Some(Refusal::Overlap);
    }

    // And the rule the GENERATOR is held to, from the other side. A muzzle, a
    // nozzle and a bay's mouth all need the space in front of them, and a part
    // takes that space two ways: by standing in the lane, or by standing beside
    // it and demanding the skin that would close it over. The rule itself is
    // `nova_ship`'s, so a hull the collapse may not draw is one a builder may
    // not build either.
    let firing: Vec<PlacedPart<'_>> = ship.iter().map(clearing).collect();
    placement_blocks_an_exit(&firing, &clearing(&ghost)).then_some(Refusal::BlockedExit)
}

/// One placed section as the shared clearance rule reads it.
fn clearing(section: &PlacedSection) -> PlacedPart<'_> {
    PlacedPart {
        position: section.position,
        rotation: section.rotation,
        link_points: &section.link_points,
        footprint: *SectionFootprint::from_collider(section.collider),
        exit: section.exit,
    }
}

fn placed(section: &PlacedSection) -> PlacedSectionLinkPoints<'_> {
    PlacedSectionLinkPoints {
        position: section.position,
        rotation: section.rotation,
        link_points: &section.link_points,
    }
}

/// AABB interpenetration after each local collider is turned into ship space.
/// Editor placement uses quarter turns, so the absolute rotation basis exactly
/// permutes cuboid extents instead of conservatively widening them on the wrong
/// axes.
fn overlaps(a: &PlacedSection, b: &PlacedSection) -> bool {
    let half_extents =
        |section: &PlacedSection| section.collider.rotated_aabb_half_extents(section.rotation);
    let distance = (a.position - b.position).abs();
    let sum = half_extents(a) + half_extents(b);
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
            exit: None,
        }
    }

    /// The one socket a bay or a gun mount carries: the base it bolts down
    /// through, opposite the face it fires out of.
    fn base_socket() -> Vec<LinkPoint> {
        vec![LinkPoint {
            id: "base".to_string(),
            position: Vec3::NEG_Y * 0.5,
            normal: Vec3::NEG_Y,
        }]
    }

    /// A part that fires along its own `+Y`, like a bay or a gun mount.
    fn firing(position: Vec3) -> PlacedSection {
        PlacedSection {
            position,
            rotation: Quat::IDENTITY,
            link_points: base_socket(),
            collider: SectionCollider::default(),
            exit: Some(Vec3::Y),
        }
    }

    /// Solve for a part that FIRES, which the ordinary helper's cube does not.
    fn solve_firing(ship: &[PlacedSection], hovered: usize, hit: Vec3) -> Placement {
        solve(
            ship,
            hovered,
            hit,
            &base_socket(),
            SectionCollider::default(),
            Some(Vec3::Y),
            0,
            0,
        )
    }

    fn solve_on(ship: &[PlacedSection], hovered: usize, hit: Vec3, roll: u32) -> Placement {
        solve(
            ship,
            hovered,
            hit,
            &unit_cube_link_points(),
            SectionCollider::default(),
            None,
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

    #[test]
    fn a_multi_cell_drive_mounts_by_its_central_forward_socket() {
        let ship = [cube(Vec3::ZERO)];
        let mut mounts = vec![LinkPoint {
            id: "base_1_1".to_string(),
            position: Vec3::new(0.0, 0.0, -1.0),
            normal: Vec3::NEG_Z,
        }];
        for x in [-1.0, 0.0, 1.0] {
            for y in [-1.0, 0.0, 1.0] {
                if x == 0.0 && y == 0.0 {
                    continue;
                }
                mounts.push(LinkPoint {
                    id: format!("base_{x}_{y}"),
                    position: Vec3::new(x, y, -1.0),
                    normal: Vec3::NEG_Z,
                });
            }
        }
        let placement = solve(
            &ship,
            0,
            Vec3::Z * 0.5,
            &mounts,
            SectionCollider::Cuboid {
                size: Vec3::new(3.0, 3.0, 2.0),
            },
            Some(Vec3::Z),
            0,
            0,
        );
        assert_eq!(placement.refusal, None);
        assert!(placement
            .transform
            .translation
            .abs_diff_eq(Vec3::Z * 1.5, 1e-5));
    }

    #[test]
    fn one_hull_separates_two_capital_drives_mounted_along_x() {
        let collider = SectionCollider::Cuboid {
            size: Vec3::new(5.0, 5.0, 3.0),
        };
        let mount = vec![LinkPoint {
            id: "base".to_string(),
            position: Vec3::NEG_Z * 1.5,
            normal: Vec3::NEG_Z,
        }];
        let ship = [
            cube(Vec3::ZERO),
            PlacedSection {
                position: Vec3::X * 2.0,
                rotation: Quat::from_rotation_y(std::f32::consts::FRAC_PI_2),
                link_points: mount.clone(),
                collider,
                exit: Some(Vec3::Z),
            },
        ];
        let placement = solve(
            &ship,
            0,
            Vec3::NEG_X * 0.5,
            &mount,
            collider,
            Some(Vec3::Z),
            0,
            0,
        );
        assert_eq!(placement.refusal, None);
        assert!(placement
            .transform
            .translation
            .abs_diff_eq(Vec3::NEG_X * 2.0, 1e-5));
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

    /// A part may not be built where something has to fire.
    ///
    /// The rule the generator collapses under, held from the other side: the
    /// editor may not build a hull the collapse would refuse to draw. Both
    /// clauses are checked, and the second is the subtle half - a part
    /// DIAGONALLY off a muzzle is not in the lane at all, it demands the
    /// skin that would close the lane over.
    #[test]
    fn a_part_in_front_of_a_muzzle_is_refused() {
        // A cube with a bay bolted to its roof, firing +Y, and a tower of hull
        // up the next column.
        let ship = [
            cube(Vec3::ZERO),
            firing(Vec3::Y),
            cube(Vec3::X),
            cube(Vec3::X + Vec3::Y),
        ];

        // A slab one cell OFF the lane: nothing stands in front of the muzzle,
        // and the skin would still close the lane over, because that slab's own
        // face would otherwise look at vacuum.
        let placement = solve_on(&ship, 3, Vec3::new(1.0, 1.5, 0.0), 0);
        assert_eq!(placement.refusal, Some(Refusal::BlockedExit));

        // The same slab on the far flank of the same tower is fine, so the
        // refusal is about the lane rather than about the ship carrying a bay.
        let placement = solve_on(&ship, 3, Vec3::new(1.5, 1.0, 0.0), 0);
        assert_eq!(placement.refusal, None);
    }

    /// A part may not be ARMED where it cannot fire either.
    ///
    /// The same rule from the part's own side: the ghost is the thing with the
    /// lane, and the hull it would fire into is already standing.
    #[test]
    fn a_muzzle_pointed_into_the_ship_is_refused() {
        // A well: hull up the +X column and a lid across the top of it, so the
        // roof of the first cube looks straight at that lid two cells up.
        let ship = [
            cube(Vec3::ZERO),
            cube(Vec3::X),
            cube(Vec3::X + Vec3::Y),
            cube(Vec3::X + Vec3::Y * 2.0),
            cube(Vec3::Y * 2.0),
        ];

        let placement = solve_firing(&ship, 0, Vec3::new(0.0, 0.5, 0.0));
        assert_eq!(placement.refusal, Some(Refusal::BlockedExit));

        // Bolted to the OUTSIDE of the same well, the identical part fires into
        // vacuum and is allowed.
        let placement = solve_firing(&ship, 4, Vec3::new(0.0, 2.5, 0.0));
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
            None,
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
        let placement = solve(
            &ship,
            0,
            Vec3::new(0.5, 0.0, 0.0),
            &socket,
            long,
            None,
            0,
            0,
        );
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
            None,
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

    /// A part arrives AS DRAWN: with no cycling, the socket that mates is the
    /// one already facing the other way, so the mate asks for no rotation.
    /// Before this, the socket cycle started at the part's first authored
    /// socket - its +X face for anything on the cube grid - so a thruster
    /// bolted to a tail arrived broadside until the builder found the right
    /// number of presses.
    #[test]
    fn an_uncycled_part_arrives_in_the_orientation_it_was_authored_in() {
        let points = unit_cube_link_points();
        let ship = [cube(Vec3::ZERO)];

        for (hit, expected) in [
            (Vec3::new(0.5, 0.0, 0.0), Vec3::X),
            (Vec3::new(-0.5, 0.0, 0.0), Vec3::NEG_X),
            (Vec3::new(0.0, 0.5, 0.0), Vec3::Y),
            (Vec3::new(0.0, 0.0, -0.5), Vec3::NEG_Z),
        ] {
            let placement = solve(
                &ship,
                0,
                hit,
                &points,
                SectionCollider::default(),
                None,
                0,
                0,
            );
            assert_eq!(placement.refusal, None);
            assert!(
                placement
                    .transform
                    .rotation
                    .abs_diff_eq(Quat::IDENTITY, 1e-5),
                "an uncycled part must not be turned ({:?} on the {expected:?} face)",
                placement.transform.rotation
            );
            assert!(
                placement.transform.translation.abs_diff_eq(expected, 1e-5),
                "and it still lands on the face it was aimed at"
            );
        }

        // Delivery guard: cycling still walks AWAY from that natural socket.
        let cycled = solve(
            &ship,
            0,
            Vec3::new(0.5, 0.0, 0.0),
            &points,
            SectionCollider::default(),
            None,
            1,
            0,
        );
        assert!(
            !cycled.transform.rotation.abs_diff_eq(Quat::IDENTITY, 1e-5),
            "one press of the socket key must change the answer"
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
            None,
            1,
            0,
        );
        let wrapped = solve(
            &ship,
            0,
            Vec3::new(0.5, 0.0, 0.0),
            &points,
            SectionCollider::default(),
            None,
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
