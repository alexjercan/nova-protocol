//! Authoritative structural sockets shared by ship runtime, lint, NOVA OS, and editor placement.
//!
//! Socket mates are derived in ship-root space; collider geometry never creates structural edges.

use std::collections::{BTreeSet, HashMap, VecDeque};

use bevy::prelude::*;

/// Link-point authoring, live snapshots, graph derivation, and fixed mate tolerances.
pub mod prelude {
    pub use super::{
        derive_link_point_graph, unit_cube_link_points, LinkPoint, LinkPointGraphError,
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

/// Six face-center structural sockets for the existing one-unit cube grid.
pub fn unit_cube_link_points() -> Vec<LinkPoint> {
    [
        ("positive_x", Vec3::X * 0.5, Vec3::X),
        ("negative_x", Vec3::NEG_X * 0.5, Vec3::NEG_X),
        ("positive_y", Vec3::Y * 0.5, Vec3::Y),
        ("negative_y", Vec3::NEG_Y * 0.5, Vec3::NEG_Y),
        ("positive_z", Vec3::Z * 0.5, Vec3::Z),
        ("negative_z", Vec3::NEG_Z * 0.5, Vec3::NEG_Z),
    ]
    .into_iter()
    .map(|(id, position, normal)| LinkPoint {
        id: id.to_string(),
        position,
        normal,
    })
    .collect()
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

    #[test]
    fn zero_and_one_section_inputs_are_connected() {
        assert_eq!(derive_link_point_graph(&[]), Ok(Vec::new()));
        let sections = [placed(Vec3::ZERO, Quat::IDENTITY, &[])];
        assert_eq!(derive_link_point_graph(&sections), Ok(Vec::new()));
    }
}
