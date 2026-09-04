//! Authoritative structural sockets shared by ship runtime, lint, NOVA OS, and editor placement.
//!
//! Socket mates are derived in ship-root space; collider geometry never creates structural edges.

use std::collections::{BTreeSet, HashMap, VecDeque};

use bevy::prelude::*;

/// Link-point authoring, live snapshots, graph derivation, mate candidates,
/// snap placement, and fixed mate tolerances.
pub mod prelude {
    pub use super::{
        box_link_points, candidate_link_point_mates, cardinal_axis, derive_link_point_graph,
        link_point_up, snap_placement, unit_cube_link_points, LinkPoint, LinkPointGraphError,
        LinkPointMate, LinkPointRef, PlacedSectionLinkPoints, SectionLinkPoints,
        LINK_POINT_NORMAL_MIN_DOT, LINK_POINT_POSITION_EPSILON,
    };
}

/// Maximum ship-root-space distance between two socket positions that can mate.
pub const LINK_POINT_POSITION_EPSILON: f32 = 1e-3;

/// Minimum `dot(a.normal, -b.normal)` for two socket normals that can mate.
pub const LINK_POINT_NORMAL_MIN_DOT: f32 = 0.999;

const UNIT_LENGTH_EPSILON: f32 = 1e-4;

/// One authorable structural socket in section-local space.
#[derive(Clone, Debug, PartialEq, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct LinkPoint {
    /// Section-local identifier used by diagnostics and UI, not compatibility.
    pub id: String,
    /// Socket position relative to the section origin.
    pub position: Vec3,
    /// Unit outward direction relative to the section frame.
    pub normal: Vec3,
}

/// The link points snapshotted onto one live section.
#[derive(Component, Clone, Debug, Default, Deref, DerefMut, Reflect)]
#[reflect(Component)]
pub struct SectionLinkPoints(pub Vec<LinkPoint>);

/// One section placement supplied to the pure graph derivation.
#[derive(Clone, Copy, Debug)]
pub struct PlacedSectionLinkPoints<'a> {
    /// Section origin in ship-root space.
    pub position: Vec3,
    /// Section rotation in ship-root space.
    pub rotation: Quat,
    /// Section-local sockets.
    pub link_points: &'a [LinkPoint],
}

/// Temporary reference to one socket in a graph-derivation input.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LinkPointRef {
    /// Index in the input ship's section list.
    pub section_index: usize,
    /// Index in that section's link-point list.
    pub link_point_index: usize,
}

/// Two sockets that form one structural mate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LinkPointMate {
    /// First socket, ordered before `b` by [`LinkPointRef`].
    pub a: LinkPointRef,
    /// Second socket.
    pub b: LinkPointRef,
}

/// A validation or topology error that prevents publishing the whole ship graph.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LinkPointGraphError {
    /// A section origin contains NaN or infinity.
    NonFiniteSectionPosition {
        /// Index in the input section list.
        section_index: usize,
    },
    /// A section quaternion contains NaN or infinity.
    NonFiniteSectionRotation {
        /// Index in the input section list.
        section_index: usize,
    },
    /// A section quaternion is zero or differs from unit length beyond tolerance.
    NonUnitSectionRotation {
        /// Index in the input section list.
        section_index: usize,
    },
    /// A socket has no diagnostic identity.
    EmptyLinkPointId {
        /// Invalid socket.
        link_point: LinkPointRef,
    },
    /// Two sockets in one section use the same ID.
    DuplicateLinkPointId {
        /// First socket with the ID.
        first: LinkPointRef,
        /// Later socket with the same ID.
        duplicate: LinkPointRef,
    },
    /// A socket position contains NaN or infinity.
    NonFiniteLinkPointPosition {
        /// Invalid socket.
        link_point: LinkPointRef,
    },
    /// A socket normal contains NaN or infinity.
    NonFiniteLinkPointNormal {
        /// Invalid socket.
        link_point: LinkPointRef,
    },
    /// A socket normal has no direction.
    ZeroLinkPointNormal {
        /// Invalid socket.
        link_point: LinkPointRef,
    },
    /// A socket normal differs from unit length beyond tolerance.
    NonUnitLinkPointNormal {
        /// Invalid socket.
        link_point: LinkPointRef,
    },
    /// One socket can mate with more than one other socket.
    AmbiguousMate {
        /// Socket with multiple candidates.
        link_point: LinkPointRef,
        /// Candidate sockets in input order.
        candidates: Vec<LinkPointRef>,
    },
    /// The derived section graph has more than one connected component.
    Disconnected {
        /// Components in section-index order; indices inside each component are sorted.
        components: Vec<Vec<usize>>,
    },
}

#[derive(Clone, Copy)]
struct TransformedLinkPoint {
    reference: LinkPointRef,
    position: Vec3,
    normal: Vec3,
}

/// Derive every unambiguous socket mate and require one connected section graph.
///
/// Zero-section and one-section inputs are connected by definition. Any error
/// rejects the whole graph; no partial mates are returned.
pub fn derive_link_point_graph(
    sections: &[PlacedSectionLinkPoints<'_>],
) -> Result<Vec<LinkPointMate>, Vec<LinkPointGraphError>> {
    let errors = validate_sections(sections);
    if !errors.is_empty() {
        return Err(errors);
    }

    let transformed = transform_link_points(sections);
    let candidates = candidate_mates(&transformed);
    let ambiguity_errors = ambiguity_errors(&transformed, &candidates);
    if !ambiguity_errors.is_empty() {
        return Err(ambiguity_errors);
    }

    let mates = unique_mates(&transformed, &candidates);
    let components = connected_components(sections.len(), &mates);
    if components.len() > 1 {
        return Err(vec![LinkPointGraphError::Disconnected { components }]);
    }

    Ok(mates)
}

fn validate_sections(sections: &[PlacedSectionLinkPoints<'_>]) -> Vec<LinkPointGraphError> {
    let mut errors = Vec::new();

    for (section_index, section) in sections.iter().enumerate() {
        if !section.position.is_finite() {
            errors.push(LinkPointGraphError::NonFiniteSectionPosition { section_index });
        }
        if !section.rotation.is_finite() {
            errors.push(LinkPointGraphError::NonFiniteSectionRotation { section_index });
        } else if (section.rotation.length() - 1.0).abs() > UNIT_LENGTH_EPSILON {
            errors.push(LinkPointGraphError::NonUnitSectionRotation { section_index });
        }

        let mut ids = HashMap::<&str, usize>::new();
        for (link_point_index, link_point) in section.link_points.iter().enumerate() {
            let reference = LinkPointRef {
                section_index,
                link_point_index,
            };
            if link_point.id.is_empty() {
                errors.push(LinkPointGraphError::EmptyLinkPointId {
                    link_point: reference,
                });
            } else if let Some(&first_index) = ids.get(link_point.id.as_str()) {
                errors.push(LinkPointGraphError::DuplicateLinkPointId {
                    first: LinkPointRef {
                        section_index,
                        link_point_index: first_index,
                    },
                    duplicate: reference,
                });
            } else {
                ids.insert(&link_point.id, link_point_index);
            }
            if !link_point.position.is_finite() {
                errors.push(LinkPointGraphError::NonFiniteLinkPointPosition {
                    link_point: reference,
                });
            }
            if !link_point.normal.is_finite() {
                errors.push(LinkPointGraphError::NonFiniteLinkPointNormal {
                    link_point: reference,
                });
            } else {
                let length = link_point.normal.length();
                if length <= f32::EPSILON {
                    errors.push(LinkPointGraphError::ZeroLinkPointNormal {
                        link_point: reference,
                    });
                } else if (length - 1.0).abs() > UNIT_LENGTH_EPSILON {
                    errors.push(LinkPointGraphError::NonUnitLinkPointNormal {
                        link_point: reference,
                    });
                }
            }
        }
    }

    errors
}

fn transform_link_points(sections: &[PlacedSectionLinkPoints<'_>]) -> Vec<TransformedLinkPoint> {
    sections
        .iter()
        .enumerate()
        .flat_map(|(section_index, section)| {
            let rotation = section.rotation.normalize();
            section
                .link_points
                .iter()
                .enumerate()
                .map(move |(link_point_index, link_point)| TransformedLinkPoint {
                    reference: LinkPointRef {
                        section_index,
                        link_point_index,
                    },
                    position: section.position + rotation * link_point.position,
                    normal: (rotation * link_point.normal).normalize(),
                })
        })
        .collect()
}

fn candidate_mates(points: &[TransformedLinkPoint]) -> Vec<Vec<usize>> {
    let mut candidates = vec![Vec::new(); points.len()];
    let max_distance_squared = LINK_POINT_POSITION_EPSILON * LINK_POINT_POSITION_EPSILON;

    for a in 0..points.len() {
        for b in (a + 1)..points.len() {
            if points[a].reference.section_index == points[b].reference.section_index {
                continue;
            }
            let coincident =
                points[a].position.distance_squared(points[b].position) <= max_distance_squared;
            let opposed = points[a].normal.dot(-points[b].normal) >= LINK_POINT_NORMAL_MIN_DOT;
            if coincident && opposed {
                candidates[a].push(b);
                candidates[b].push(a);
            }
        }
    }

    candidates
}

fn ambiguity_errors(
    points: &[TransformedLinkPoint],
    candidates: &[Vec<usize>],
) -> Vec<LinkPointGraphError> {
    candidates
        .iter()
        .enumerate()
        .filter(|(_, candidates)| candidates.len() > 1)
        .map(|(point, candidates)| LinkPointGraphError::AmbiguousMate {
            link_point: points[point].reference,
            candidates: candidates
                .iter()
                .map(|candidate| points[*candidate].reference)
                .collect(),
        })
        .collect()
}

fn unique_mates(points: &[TransformedLinkPoint], candidates: &[Vec<usize>]) -> Vec<LinkPointMate> {
    candidates
        .iter()
        .enumerate()
        .filter_map(|(point, candidates)| {
            let candidate = *candidates.first()?;
            (point < candidate).then_some(LinkPointMate {
                a: points[point].reference,
                b: points[candidate].reference,
            })
        })
        .collect()
}

fn connected_components(section_count: usize, mates: &[LinkPointMate]) -> Vec<Vec<usize>> {
    if section_count == 0 {
        return Vec::new();
    }

    let mut neighbors = vec![BTreeSet::new(); section_count];
    for mate in mates {
        let a = mate.a.section_index;
        let b = mate.b.section_index;
        neighbors[a].insert(b);
        neighbors[b].insert(a);
    }

    let mut visited = vec![false; section_count];
    let mut components = Vec::new();
    for start in 0..section_count {
        if visited[start] {
            continue;
        }
        visited[start] = true;
        let mut pending = VecDeque::from([start]);
        let mut component = Vec::new();
        while let Some(section) = pending.pop_front() {
            component.push(section);
            for &neighbor in &neighbors[section] {
                if !visited[neighbor] {
                    visited[neighbor] = true;
                    pending.push_back(neighbor);
                }
            }
        }
        component.sort_unstable();
        components.push(component);
    }
    components
}

/// Every coincident-and-opposed socket pair, WITHOUT the ambiguity and
/// connectivity gates [`derive_link_point_graph`] applies.
///
/// The graph derivation answers "is this ship valid", which a ship under
/// assembly is legitimately not: a part hovering under the pointer is a
/// component of its own, and a socket the builder is about to fill has no mate
/// yet. Placement needs the raw candidates instead - which sockets ARE taken,
/// and whether adding one more part would leave a socket with two suitors.
pub fn candidate_link_point_mates(sections: &[PlacedSectionLinkPoints<'_>]) -> Vec<LinkPointMate> {
    let transformed = transform_link_points(sections);
    let candidates = candidate_mates(&transformed);
    let mut mates = Vec::new();
    for (point, others) in candidates.iter().enumerate() {
        for &other in others {
            if point < other {
                mates.push(LinkPointMate {
                    a: transformed[point].reference,
                    b: transformed[other].reference,
                });
            }
        }
    }
    mates
}

/// The roll reference of a socket: the unit vector orthogonal to `normal` that
/// a part mated onto that socket stands "up" along.
///
/// DERIVED from the normal rather than authored, and that is the whole point: a
/// socket's up is a pure function of where it faces, so two parts drawn from
/// different craft - or a hand-authored mod part and a shipped one - agree on
/// the mate without anyone having written a second vector. Same normal, same
/// up, every time.
///
/// The reference is ship up (+Y) everywhere except on the sockets that face it,
/// where +Y projects to nothing and forward (+Z) takes over. The formula is
/// even in `normal`, so a socket and the socket it mates always resolve to the
/// SAME up.
pub fn link_point_up(normal: Vec3) -> Vec3 {
    let normal = normal.normalize_or(Vec3::Z);
    let reference = if normal.dot(Vec3::Y).abs() > 0.95 {
        Vec3::Z
    } else {
        Vec3::Y
    };
    (reference - normal * reference.dot(normal)).normalize_or(Vec3::Z)
}

/// The cardinal axis nearest `direction`, in whatever space it was given in.
///
/// Authoring tools that derive a socket from part GEOMETRY (a centre-to-centre
/// direction, a cut plane) get whatever angle the neighbour happened to sit at,
/// and a part mated onto such a socket arrives tilted by exactly that angle.
/// Snapping the authored normal to an axis is what lets one part mate cleanly
/// onto a part it was never drawn beside.
///
/// Antisymmetric by construction (`cardinal_axis(-d) == -cardinal_axis(d)`), so
/// snapping BOTH ends of an authored edge keeps their normals exactly opposed
/// and no existing mate is lost.
pub fn cardinal_axis(direction: Vec3) -> Vec3 {
    let magnitude = direction.abs();
    let (axis, component) = if magnitude.x >= magnitude.y && magnitude.x >= magnitude.z {
        (Vec3::X, direction.x)
    } else if magnitude.y >= magnitude.z {
        (Vec3::Y, direction.y)
    } else {
        (Vec3::Z, direction.z)
    };
    axis * component.signum()
}

/// The orthonormal frame of a socket: its normal is local +Z and its
/// [`link_point_up`] is local +Y.
fn socket_frame(normal: Vec3, up: Vec3) -> Quat {
    Quat::from_mat3(&Mat3::from_cols(up.cross(normal), up, normal))
}

/// The pose that mates `source` - a socket in the placed part's own frame -
/// onto a socket already at `target_position` facing `target_normal`, rolled
/// by `quarter_turns` about the mating axis.
///
/// Mating is the whole definition of the pose: the two sockets become
/// coincident and their normals opposed, which leaves exactly one free degree
/// of freedom - the roll about that shared axis, which is the builder's to
/// choose. Returns the part's `(position, rotation)` in the same space the
/// target socket was given in.
///
/// The two socket FRAMES are what is mated, not just the two normals. Aligning
/// normals alone (a shortest-arc rotation) leaves the roll to whatever axis the
/// arc happened to sweep about, which is why the same part used to arrive at a
/// different, unpredictable tilt on every socket it was offered.
pub fn snap_placement(
    target_position: Vec3,
    target_normal: Vec3,
    source: &LinkPoint,
    quarter_turns: u32,
) -> (Vec3, Quat) {
    let target_normal = target_normal.normalize_or(Vec3::Z);
    let source_normal = source.normal.normalize_or(Vec3::Z);
    // The part's socket ends up facing INTO the target's, so the frame it is
    // rotated onto is the target's flipped about its own up.
    let facing = -target_normal;
    let align = socket_frame(facing, link_point_up(target_normal))
        * socket_frame(source_normal, link_point_up(source_normal)).inverse();
    let roll = Quat::from_axis_angle(facing, quarter_turns as f32 * std::f32::consts::FRAC_PI_2);
    let rotation = (roll * align).normalize();
    (target_position - rotation * source.position, rotation)
}

/// Six face-centre structural sockets for a box of `size` (FULL side lengths,
/// the same units [`SectionCollider::Cuboid`] takes).
///
/// Axis-aligned normals and centred positions, which is what makes a boxy part
/// mate with any other boxy part whatever their sizes: the sockets meet face to
/// face and [`snap_placement`] resolves the roll from the axis alone.
pub fn box_link_points(size: Vec3) -> Vec<LinkPoint> {
    let half = size * 0.5;
    [
        ("positive_x", Vec3::X * half.x, Vec3::X),
        ("negative_x", Vec3::NEG_X * half.x, Vec3::NEG_X),
        ("positive_y", Vec3::Y * half.y, Vec3::Y),
        ("negative_y", Vec3::NEG_Y * half.y, Vec3::NEG_Y),
        ("positive_z", Vec3::Z * half.z, Vec3::Z),
        ("negative_z", Vec3::NEG_Z * half.z, Vec3::NEG_Z),
    ]
    .into_iter()
    .map(|(id, position, normal)| LinkPoint {
        id: id.to_string(),
        position,
        normal,
    })
    .collect()
}

/// Six face-center structural sockets for the existing one-unit cube grid.
pub fn unit_cube_link_points() -> Vec<LinkPoint> {
    box_link_points(Vec3::ONE)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn point(id: &str, position: Vec3, normal: Vec3) -> LinkPoint {
        LinkPoint {
            id: id.to_string(),
            position,
            normal,
        }
    }

    fn placed(
        position: Vec3,
        rotation: Quat,
        link_points: &[LinkPoint],
    ) -> PlacedSectionLinkPoints<'_> {
        PlacedSectionLinkPoints {
            position,
            rotation,
            link_points,
        }
    }

    #[test]
    fn unit_cube_points_are_explicit_face_centers() {
        let points = unit_cube_link_points();
        assert_eq!(points.len(), 6);
        assert_eq!(points[0], point("positive_x", Vec3::X * 0.5, Vec3::X));
        assert_eq!(
            points[5],
            point("negative_z", Vec3::NEG_Z * 0.5, Vec3::NEG_Z)
        );
    }

    #[test]
    fn adjacent_cube_sockets_form_one_mate() {
        let points = unit_cube_link_points();
        let sections = [
            placed(Vec3::ZERO, Quat::IDENTITY, &points),
            placed(Vec3::X, Quat::IDENTITY, &points),
        ];

        let mates = derive_link_point_graph(&sections).unwrap();

        assert_eq!(
            mates,
            vec![LinkPointMate {
                a: LinkPointRef {
                    section_index: 0,
                    link_point_index: 0,
                },
                b: LinkPointRef {
                    section_index: 1,
                    link_point_index: 1,
                },
            }]
        );
    }

    #[test]
    fn arbitrary_section_rotation_transforms_positions_and_normals() {
        let a = [point("out", Vec3::X, Vec3::X)];
        let b = [point("in", Vec3::NEG_X, Vec3::NEG_X)];
        let rotation = Quat::from_rotation_z(std::f32::consts::FRAC_PI_4);
        let offset = rotation * Vec3::X * 2.0;
        let sections = [
            placed(Vec3::ZERO, rotation, &a),
            placed(offset, rotation, &b),
        ];

        assert_eq!(derive_link_point_graph(&sections).unwrap().len(), 1);
    }

    #[test]
    fn position_and_normal_tolerances_are_pinned() {
        let a = [point("a", Vec3::ZERO, Vec3::X)];
        let within_angle = LINK_POINT_NORMAL_MIN_DOT.acos() * 0.5;
        let within_normal = Quat::from_rotation_y(within_angle) * Vec3::NEG_X;
        let b = [point(
            "b",
            Vec3::X * (LINK_POINT_POSITION_EPSILON * 0.5),
            within_normal,
        )];
        let sections = [
            placed(Vec3::ZERO, Quat::IDENTITY, &a),
            placed(Vec3::ZERO, Quat::IDENTITY, &b),
        ];
        assert_eq!(derive_link_point_graph(&sections).unwrap().len(), 1);

        let outside_position = [point(
            "b",
            Vec3::X * (LINK_POINT_POSITION_EPSILON * 2.0),
            Vec3::NEG_X,
        )];
        let sections = [
            placed(Vec3::ZERO, Quat::IDENTITY, &a),
            placed(Vec3::ZERO, Quat::IDENTITY, &outside_position),
        ];
        assert!(matches!(
            derive_link_point_graph(&sections),
            Err(errors) if matches!(errors.as_slice(), [LinkPointGraphError::Disconnected { .. }])
        ));

        let outside_angle = LINK_POINT_NORMAL_MIN_DOT.acos() * 2.0;
        let outside_normal = [point(
            "b",
            Vec3::ZERO,
            Quat::from_rotation_y(outside_angle) * Vec3::NEG_X,
        )];
        let sections = [
            placed(Vec3::ZERO, Quat::IDENTITY, &a),
            placed(Vec3::ZERO, Quat::IDENTITY, &outside_normal),
        ];
        assert!(matches!(
            derive_link_point_graph(&sections),
            Err(errors) if matches!(errors.as_slice(), [LinkPointGraphError::Disconnected { .. }])
        ));
    }

    #[test]
    fn local_validation_collects_independent_errors() {
        let points = [
            point("", Vec3::splat(f32::NAN), Vec3::ZERO),
            point("dup", Vec3::ZERO, Vec3::X * 2.0),
            point("dup", Vec3::ZERO, Vec3::splat(f32::INFINITY)),
        ];
        let sections = [placed(
            Vec3::splat(f32::NAN),
            Quat::from_xyzw(0.0, 0.0, 0.0, 2.0),
            &points,
        )];

        let errors = derive_link_point_graph(&sections).unwrap_err();

        assert!(
            errors.contains(&LinkPointGraphError::NonFiniteSectionPosition { section_index: 0 })
        );
        assert!(errors.contains(&LinkPointGraphError::NonUnitSectionRotation { section_index: 0 }));
        assert!(errors
            .iter()
            .any(|error| matches!(error, LinkPointGraphError::EmptyLinkPointId { .. })));
        assert!(errors
            .iter()
            .any(|error| matches!(error, LinkPointGraphError::DuplicateLinkPointId { .. })));
        assert!(errors.iter().any(|error| matches!(
            error,
            LinkPointGraphError::NonFiniteLinkPointPosition { .. }
        )));
        assert!(errors
            .iter()
            .any(|error| matches!(error, LinkPointGraphError::ZeroLinkPointNormal { .. })));
        assert!(errors
            .iter()
            .any(|error| matches!(error, LinkPointGraphError::NonUnitLinkPointNormal { .. })));
        assert!(errors
            .iter()
            .any(|error| matches!(error, LinkPointGraphError::NonFiniteLinkPointNormal { .. })));
    }

    #[test]
    fn one_socket_with_multiple_candidates_rejects_the_whole_graph() {
        let a = [point("a", Vec3::ZERO, Vec3::X)];
        let b = [point("b", Vec3::ZERO, Vec3::NEG_X)];
        let c = [point("c", Vec3::ZERO, Vec3::NEG_X)];
        let sections = [
            placed(Vec3::ZERO, Quat::IDENTITY, &a),
            placed(Vec3::ZERO, Quat::IDENTITY, &b),
            placed(Vec3::ZERO, Quat::IDENTITY, &c),
        ];

        let errors = derive_link_point_graph(&sections).unwrap_err();

        assert!(errors.iter().any(|error| matches!(
            error,
            LinkPointGraphError::AmbiguousMate {
                link_point: LinkPointRef { section_index: 0, .. },
                candidates,
            } if candidates.len() == 2
        )));
    }

    #[test]
    fn multiple_socket_mates_between_sections_still_form_one_connected_edge() {
        let a = [
            point("a0", Vec3::ZERO, Vec3::X),
            point("a1", Vec3::Y, Vec3::X),
        ];
        let b = [
            point("b0", Vec3::ZERO, Vec3::NEG_X),
            point("b1", Vec3::Y, Vec3::NEG_X),
        ];
        let sections = [
            placed(Vec3::ZERO, Quat::IDENTITY, &a),
            placed(Vec3::ZERO, Quat::IDENTITY, &b),
        ];

        assert_eq!(derive_link_point_graph(&sections).unwrap().len(), 2);
    }

    #[test]
    fn disconnected_error_reports_every_component() {
        let points = unit_cube_link_points();
        let sections = [
            placed(Vec3::ZERO, Quat::IDENTITY, &points),
            placed(Vec3::X, Quat::IDENTITY, &points),
            placed(Vec3::X * 10.0, Quat::IDENTITY, &points),
        ];

        assert_eq!(
            derive_link_point_graph(&sections),
            Err(vec![LinkPointGraphError::Disconnected {
                components: vec![vec![0, 1], vec![2]],
            }])
        );
    }

    /// Candidates are what the graph derivation REJECTS on: a ship mid-assembly
    /// is disconnected, and an over-subscribed socket is exactly what placement
    /// has to see in order to refuse.
    #[test]
    fn candidates_survive_disconnection_and_ambiguity() {
        let points = unit_cube_link_points();
        let sections = [
            placed(Vec3::ZERO, Quat::IDENTITY, &points),
            placed(Vec3::X, Quat::IDENTITY, &points),
            // A third cube far away: the ship is not connected yet.
            placed(Vec3::X * 10.0, Quat::IDENTITY, &points),
        ];
        assert!(derive_link_point_graph(&sections).is_err());
        assert_eq!(candidate_link_point_mates(&sections).len(), 1);

        // Two parts claiming one socket: two candidates on the same point.
        let a = [point("a", Vec3::ZERO, Vec3::X)];
        let b = [point("b", Vec3::ZERO, Vec3::NEG_X)];
        let c = [point("c", Vec3::ZERO, Vec3::NEG_X)];
        let sections = [
            placed(Vec3::ZERO, Quat::IDENTITY, &a),
            placed(Vec3::ZERO, Quat::IDENTITY, &b),
            placed(Vec3::ZERO, Quat::IDENTITY, &c),
        ];
        let mates = candidate_link_point_mates(&sections);
        assert_eq!(mates.len(), 2, "both suitors are reported: {mates:?}");
    }

    /// A snapped part mates: the derivation the runtime uses finds the socket
    /// pair the placement aimed at, which is the only claim placement makes.
    #[test]
    fn a_snapped_part_mates_with_the_socket_it_was_placed_on() {
        let points = unit_cube_link_points();
        // Target: the +X face of a cube standing at the origin.
        let target = &points[0];
        let source = &points[1]; // its -X face, the socket that meets it

        let (position, rotation) = snap_placement(target.position, target.normal, source, 0);
        let sections = [
            placed(Vec3::ZERO, Quat::IDENTITY, &points),
            placed(position, rotation, &points),
        ];

        let mates = derive_link_point_graph(&sections).expect("the placement mates");
        assert!(mates.contains(&LinkPointMate {
            a: LinkPointRef {
                section_index: 0,
                link_point_index: 0,
            },
            b: LinkPointRef {
                section_index: 1,
                link_point_index: 1,
            },
        }));
        // And it lands where the grid always put it: one unit along +X.
        assert!(position.abs_diff_eq(Vec3::X, 1e-5), "{position:?}");
    }

    /// The roll is the one free degree of freedom a mate leaves. A quarter turn
    /// must keep the mate (same position, sockets still coincident and opposed)
    /// while actually turning the part about the mating axis.
    #[test]
    fn a_rolled_snap_keeps_the_mate_and_turns_the_part() {
        let points = unit_cube_link_points();
        let target = &points[0];
        let source = &points[1];

        let (upright, upright_rotation) = snap_placement(target.position, target.normal, source, 0);
        let (rolled, rolled_rotation) = snap_placement(target.position, target.normal, source, 1);
        assert!(rolled.abs_diff_eq(upright, 1e-5), "the pose is unmoved");
        assert!(
            (upright_rotation.angle_between(rolled_rotation) - std::f32::consts::FRAC_PI_2).abs()
                < 1e-4,
            "a quarter turn about the mating axis"
        );

        let sections = [
            placed(Vec3::ZERO, Quat::IDENTITY, &points),
            placed(rolled, rolled_rotation, &points),
        ];
        assert_eq!(
            derive_link_point_graph(&sections)
                .expect("a rolled placement still mates")
                .len(),
            1
        );
    }

    /// The claim the whole frame-mate exists for: ONE part mates the same way
    /// up on EVERY socket. Aligning normals alone left the roll to the arc's
    /// axis, and for a socket facing exactly opposite the part's own that arc
    /// has no axis at all - it fell back to an arbitrary perpendicular, so the
    /// same turret mounted under a hull faced somewhere else than the one
    /// mounted on top of it.
    #[test]
    fn one_part_mates_the_same_way_up_on_every_socket() {
        // A part that stands on its -Y face.
        let foot = point("foot", Vec3::NEG_Y * 0.5, Vec3::NEG_Y);

        for target_normal in [
            Vec3::X,
            Vec3::NEG_X,
            Vec3::Y,
            Vec3::NEG_Y,
            Vec3::Z,
            Vec3::NEG_Z,
        ] {
            let (_, rotation) = snap_placement(target_normal * 0.5, target_normal, &foot, 0);

            // It STANDS on the socket: the part's own up - the axis opposite
            // the foot it mates with - points back out along the target normal.
            assert!(
                (rotation * Vec3::Y).abs_diff_eq(target_normal, 1e-5),
                "a part standing on {target_normal:?} must point its up back out along it"
            );
            // And it arrives untilted: every part axis lands on an axis.
            for axis in [Vec3::X, Vec3::Y, Vec3::Z] {
                let placed = rotation * axis;
                assert!(
                    placed.abs_diff_eq(cardinal_axis(placed), 1e-5),
                    "{axis:?} arrived tilted ({placed:?}) on the {target_normal:?} socket"
                );
            }
        }
    }

    /// The two ends of one authored edge point exactly opposite ways, so
    /// snapping both to an axis has to keep them opposed - including on a tie,
    /// where each end picks from the same two candidates.
    #[test]
    fn the_cardinal_axis_is_antisymmetric() {
        // A real modelled pod -> fuselage direction: 36 degrees off -X, which is
        // exactly how far a part mated onto it used to arrive tilted.
        let oblique = Vec3::new(-0.8055, 0.1527, 0.5726).normalize();
        assert_eq!(cardinal_axis(oblique), Vec3::NEG_X);
        assert_eq!(cardinal_axis(-oblique), Vec3::X);

        for direction in [
            Vec3::new(0.5, 0.5, 0.0),
            Vec3::new(0.0, 0.5, 0.5),
            Vec3::new(0.5, 0.5, 0.5),
        ] {
            assert_eq!(
                cardinal_axis(direction),
                -cardinal_axis(-direction),
                "a tie must resolve to the same axis from both ends"
            );
        }
    }

    /// A socket and the socket it mates face opposite ways, so the up they roll
    /// against must come out the same for both - otherwise the mate would have
    /// two different zeroes.
    #[test]
    fn the_socket_up_is_the_same_from_either_end() {
        for normal in [
            Vec3::X,
            Vec3::Y,
            Vec3::Z,
            Vec3::new(0.6, 0.0, 0.8),
            Vec3::new(0.0, 0.99, 0.14).normalize(),
        ] {
            let up = link_point_up(normal);
            assert!(up.is_normalized(), "{normal:?} -> {up:?}");
            assert!(up.dot(normal).abs() < 1e-5, "{normal:?} -> {up:?}");
            assert!(up.abs_diff_eq(link_point_up(-normal), 1e-6), "{normal:?}");
        }
    }

    /// Snapping is not cube-only: an off-centre socket on a rotated target is
    /// the semantic-part case, and the placed part must still land exactly on
    /// it.
    #[test]
    fn snapping_holds_for_an_off_centre_socket_on_a_rotated_target() {
        let target_rotation = Quat::from_rotation_z(std::f32::consts::FRAC_PI_3);
        let target_origin = Vec3::new(2.0, -1.0, 0.5);
        let target_local = point("to_nose", Vec3::new(0.0, 0.0, -1.25), Vec3::NEG_Z);
        let source = point("to_fuselage", Vec3::new(0.3, 0.0, 0.75), Vec3::Z);

        let target_position = target_origin + target_rotation * target_local.position;
        let target_normal = target_rotation * target_local.normal;
        let (position, rotation) = snap_placement(target_position, target_normal, &source, 0);

        let target_points = [target_local];
        let source_points = [source];
        let sections = [
            placed(target_origin, target_rotation, &target_points),
            placed(position, rotation, &source_points),
        ];
        assert_eq!(
            derive_link_point_graph(&sections)
                .expect("the off-centre socket mates")
                .len(),
            1
        );
    }

    #[test]
    fn zero_and_one_section_inputs_are_connected() {
        assert_eq!(derive_link_point_graph(&[]), Ok(Vec::new()));
        let sections = [placed(Vec3::ZERO, Quat::IDENTITY, &[])];
        assert_eq!(derive_link_point_graph(&sections), Ok(Vec::new()));
    }
}
