//! Compare the known-collider test with Avian's spatial-query contract.

use bevy::ecs::system::SystemState;

use super::*;

/// The spatial path is the reference, including its strict distance bound.
fn spatial_impact(
    world: &SweepWorld,
    shape: &Collider,
    target: Entity,
    origin: Vec3,
    velocity: Vec3,
    target_velocity: Vec3,
    remaining: f32,
    dt: f32,
) -> Option<f32> {
    let relative = velocity - target_velocity;
    let direction = Dir3::new(relative).ok()?;
    let closing = relative.length();
    world
        .spatial
        .cast_shape_predicate(
            shape,
            origin - target_velocity * (dt - remaining),
            Quat::IDENTITY,
            direction,
            &ShapeCastConfig::from_max_distance(closing * remaining),
            &SpatialQueryFilter::default(),
            &|collider| collider == target,
        )
        .map(|hit| hit.distance / closing)
}

#[test]
fn a_known_target_needs_no_spatial_tree_search() {
    let mut app = round_app();
    // Candidate selection owns tree membership; the exact test needs only
    // the selected collider. Empty the trees to pin that separation.
    let target = app
        .world_mut()
        .spawn((
            Position(Vec3::ZERO),
            Rotation::default(),
            Collider::sphere(1.0),
        ))
        .id();
    app.world_mut()
        .insert_resource(avian3d::collider_tree::ColliderTrees::default());
    let mut state = SystemState::<SweepWorld>::new(app.world_mut());
    let world = state.get(app.world()).expect("sweep resources");
    let velocity = Vec3::NEG_Z * 100.0;
    assert_eq!(
        spatial_impact(
            &world,
            &Collider::sphere(ROUND_RADIUS),
            target,
            Vec3::Z * 5.0,
            velocity,
            Vec3::ZERO,
            0.1,
            0.1
        ),
        None,
        "the control must have no spatial proxy"
    );
    assert!(
        rest_frame_impact(
            &world,
            &Collider::sphere(ROUND_RADIUS),
            target,
            Vec3::Z * 5.0,
            velocity,
            Vec3::ZERO,
            0.1,
            0.1,
        )
        .is_some(),
        "an already selected collider must not need another spatial search"
    );
}

#[test]
fn exact_hits_match_spatial_hits_for_scaled_rotated_and_moving_targets() {
    let mut app = round_app();
    let mut targets = Vec::new();
    for (index, shape) in [
        Collider::sphere(1.0),
        Collider::cuboid(2.0, 1.0, 3.0),
        Collider::capsule(0.5, 2.0),
    ]
    .into_iter()
    .enumerate()
    {
        let centre = Vec3::X * index as f32 * 20.0;
        let body = app
            .world_mut()
            .spawn((RigidBody::Static, Transform::from_translation(centre)))
            .id();
        let target = app
            .world_mut()
            .spawn((
                ChildOf(body),
                Transform::from_rotation(Quat::from_rotation_y(0.37))
                    .with_scale(Vec3::new(1.5, 0.5, 2.0)),
                shape,
            ))
            .id();
        targets.push((target, centre));
    }
    settle(&mut app);
    let mut state = SystemState::<SweepWorld>::new(app.world_mut());
    let world = state.get(app.world()).expect("sweep resources");
    let shape = Collider::sphere(ROUND_RADIUS);
    let mut hits = 0;
    let mut misses = 0;
    for (target, centre) in targets {
        for offset in [Vec3::ZERO, Vec3::Z * 5.0, Vec3::new(3.0, 0.0, 5.0)] {
            for velocity in [Vec3::NEG_Z * 100.0, Vec3::Z * 100.0, Vec3::ZERO] {
                for target_velocity in [Vec3::ZERO, Vec3::Z * 20.0, Vec3::X * 40.0, velocity] {
                    for remaining in [0.0, 0.0625, 0.125] {
                        let origin = centre + offset;
                        let expected = spatial_impact(
                            &world,
                            &shape,
                            target,
                            origin,
                            velocity,
                            target_velocity,
                            remaining,
                            0.125,
                        );
                        let actual = rest_frame_impact(
                            &world,
                            &shape,
                            target,
                            origin,
                            velocity,
                            target_velocity,
                            remaining,
                            0.125,
                        );
                        assert_eq!(
                            actual, expected,
                            "target {target:?}, origin {origin:?}, velocity {velocity:?}, \
                             target velocity {target_velocity:?}, remaining {remaining}"
                        );
                        if actual.is_some() {
                            hits += 1;
                        } else {
                            misses += 1;
                        }
                    }
                }
            }
        }
    }
    assert!(
        hits > 0 && misses > 0,
        "the matrix must exercise both outcomes"
    );
}

/// A fixed known-collider workload, not an arena or rendering benchmark.
/// Both paths read the same 1,024 child colliders and the same hit/miss inputs.
/// Run with `--release -- --ignored --nocapture --test-threads=1`.
#[test]
#[ignore = "wall-clock comparison; run manually, never a timing assertion"]
fn known_collider_cost_against_the_spatial_reference() {
    use std::{hint::black_box, time::Instant};

    const TARGETS: usize = 1024;
    const PASSES: usize = 64;
    let mut app = round_app();
    let mut cases = Vec::new();
    for index in 0..TARGETS {
        let centre = Vec3::new(
            (index % 16) as f32 * 2.0,
            (index / 16 % 8) as f32 * 2.0,
            (index / 128) as f32 * 2.0,
        );
        let body = app
            .world_mut()
            .spawn((RigidBody::Static, Transform::from_translation(centre)))
            .id();
        let target = app
            .world_mut()
            .spawn((
                ChildOf(body),
                Transform::from_rotation(Quat::from_rotation_y(0.37))
                    .with_scale(Vec3::new(1.2, 0.75, 1.0)),
                Collider::cuboid(1.0, 1.0, 1.0),
            ))
            .id();
        let velocity = if index % 2 == 0 {
            Vec3::ZERO
        } else {
            Vec3::X * 80.0
        };
        cases.push((target, centre + Vec3::Z * 5.0, velocity));
    }
    settle(&mut app);
    let mut state = SystemState::<SweepWorld>::new(app.world_mut());
    let world = state.get(app.world()).expect("sweep resources");
    let shape = Collider::sphere(ROUND_RADIUS);
    let measure = |direct: bool, passes: usize| {
        let start = Instant::now();
        let mut hits = 0;
        let mut sum = 0.0f64;
        for _ in 0..passes {
            for index in 0..TARGETS {
                let (target, origin, target_velocity) = cases[index * 127 % TARGETS];
                let result = if direct {
                    rest_frame_impact(
                        &world,
                        &shape,
                        target,
                        origin,
                        Vec3::NEG_Z * 100.0,
                        target_velocity,
                        0.1,
                        0.1,
                    )
                } else {
                    spatial_impact(
                        &world,
                        &shape,
                        target,
                        origin,
                        Vec3::NEG_Z * 100.0,
                        target_velocity,
                        0.1,
                        0.1,
                    )
                };
                if let Some(at) = black_box(result) {
                    hits += 1;
                    sum += f64::from(at);
                }
            }
        }
        (start.elapsed().as_secs_f64() * 1000.0, hits, sum)
    };
    // Warm both paths before the repeat set; do not report warm-up timings.
    measure(false, 1);
    measure(true, 1);
    for repeat in 0..5 {
        let mut results = [(0.0, 0, 0.0); 2];
        for index in [repeat % 2, 1 - repeat % 2] {
            results[index] = measure(index == 1, PASSES);
        }
        assert_eq!(results[0].1, TARGETS * PASSES / 2, "matched hit/miss mix");
        assert_eq!(results[0].1, results[1].1, "same hits");
        assert_eq!(results[0].2, results[1].2, "same impact times");
        eprintln!(
            "known collider repeat={} calls={} hits={} spatial_ms={:.6} direct_ms={:.6}",
            repeat + 1,
            TARGETS * PASSES,
            results[0].1,
            results[0].0,
            results[1].0
        );
    }
}

#[test]
fn a_contact_exactly_at_the_segment_end_is_not_an_impact() {
    let mut app = round_app();
    let target = app
        .world_mut()
        .spawn((
            RigidBody::Static,
            Transform::default(),
            Collider::sphere(1.0),
        ))
        .id();
    settle(&mut app);
    let mut state = SystemState::<SweepWorld>::new(app.world_mut());
    let world = state.get(app.world()).expect("sweep resources");
    let shape = Collider::sphere(1.0);
    for (remaining, expected) in [(0.5, None), (0.75, Some(0.5))] {
        let actual = rest_frame_impact(
            &world,
            &shape,
            target,
            Vec3::Z * 3.0,
            Vec3::NEG_Z * 2.0,
            Vec3::ZERO,
            remaining,
            remaining,
        );
        assert_eq!(actual, expected);
    }
}
