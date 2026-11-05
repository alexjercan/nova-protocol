//! A torpedo's flight trail: a faint ribbon of exhaust left in the sky behind
//! a lit drive, so a salvo's paths stay readable after the torpedoes have
//! moved on.
//!
//! The ribbon is a chain of short world-space boxes, one per sample, each
//! stretched from the last sample to the torpedo's tail. A segment is
//! unparented and has no physics body, so it stays where it was laid down,
//! outlives the torpedo, and is a visual transient to the save (a top-level
//! [`TempEntity`] with no body is never frozen) and to sector streaming
//! (despawned with the window like an impact spark). It only thins with age:
//! its length stays the ground the torpedo covered.
//!
//! The length of the trail in seconds comes from
//! [`GraphicsBudget::trail_seconds`]: none on the Low tier.

use bevy::light::NotShadowCaster;

use super::*;

/// Seconds between trail samples: each sample lays one segment.
const TRAIL_SAMPLE_SECS: f32 = 0.05;

/// A new segment's width, world units (2 m): about the torpedo's body.
const TRAIL_WIDTH: f32 = 0.2;

/// How far behind the torpedo's centre the trail starts, world units: the
/// nozzle end of a 2-unit body.
const TRAIL_TAIL_OFFSET: f32 = 1.0;

/// The trail's colour, added to what is behind it. Faint and cold: a trail
/// says where a torpedo went, and anything brighter would hide the target
/// the torpedo is going to.
const TRAIL_COLOR: LinearRgba = LinearRgba::new(0.22, 0.28, 0.38, 1.0);

/// A torpedo whose lit drive is laying a trail. Inserted on
/// [`TorpedoIgnited`]; the trail stops when the torpedo is shot down or gone.
#[derive(Component, Clone, Copy, Debug, Default, Reflect)]
#[reflect(Component)]
pub(super) struct TorpedoTrailEmitter {
    /// The tail position the last segment ended at. `None` until the first
    /// sample after ignition.
    last: Option<Vec3>,
    /// Seconds since the last sample.
    since: f32,
}

/// One piece of a torpedo trail, in world space.
#[derive(Component, Clone, Copy, Debug, Reflect)]
#[reflect(Component)]
pub(super) struct TorpedoTrailSegment {
    /// The segment's length, world units. Fixed for its life.
    length: f32,
}

/// The one mesh and material every trail segment is drawn with: a unit cube,
/// scaled per segment, and one unlit additive material.
#[derive(Resource, Debug)]
pub(super) struct TorpedoTrailAssets {
    mesh: Handle<Mesh>,
    material: Handle<StandardMaterial>,
}

impl FromWorld for TorpedoTrailAssets {
    fn from_world(world: &mut World) -> Self {
        let mesh = world
            .resource_mut::<Assets<Mesh>>()
            .add(Cuboid::new(1.0, 1.0, 1.0));
        let material = world
            .resource_mut::<Assets<StandardMaterial>>()
            .add(StandardMaterial {
                base_color: Color::LinearRgba(TRAIL_COLOR),
                alpha_mode: bevy::prelude::AlphaMode::Add,
                unlit: true,
                ..default()
            });
        Self { mesh, material }
    }
}

/// A segment's width at `age` seconds of a `life`-second life: full at birth,
/// thinning linearly to nothing at the end.
pub(super) fn trail_width(age: f32, life: f32) -> f32 {
    if life <= 0.0 {
        return 0.0;
    }
    TRAIL_WIDTH * (1.0 - age / life).clamp(0.0, 1.0)
}

/// Start the trail on a torpedo whose drive just lit.
pub(super) fn on_torpedo_ignited_trail(ignited: On<TorpedoIgnited>, mut commands: Commands) {
    commands
        .entity(ignited.torpedo)
        .try_insert(TorpedoTrailEmitter::default());
}

/// Lay a segment behind every emitting torpedo each [`TRAIL_SAMPLE_SECS`].
///
/// A shot-down torpedo stops laying trail on the frame it is marked: the
/// ribbon ends where it broke up. On a budget with no trail the emitter still
/// tracks the tail, so a trail turned on mid-flight starts from where the
/// torpedo is and not from where it lit.
pub(super) fn emit_torpedo_trail(
    time: Res<Time>,
    budget: Option<Res<GraphicsBudget>>,
    assets: Res<TorpedoTrailAssets>,
    mut q_emitter: Query<
        (&Transform, &mut TorpedoTrailEmitter),
        (
            With<TorpedoProjectileMarker>,
            Without<TorpedoShotDownMarker>,
        ),
    >,
    mut commands: Commands,
) {
    let life = budget
        .as_deref()
        .copied()
        .unwrap_or_default()
        .trail_seconds();
    let dt = time.delta_secs();
    for (transform, mut emitter) in &mut q_emitter {
        let tail = transform.translation - transform.forward() * TRAIL_TAIL_OFFSET;
        let Some(last) = emitter.last else {
            emitter.last = Some(tail);
            emitter.since = 0.0;
            continue;
        };
        emitter.since += dt;
        if emitter.since < TRAIL_SAMPLE_SECS {
            continue;
        }
        emitter.since = 0.0;
        emitter.last = Some(tail);
        let span = tail - last;
        let length = span.length();
        if life <= 0.0 || length <= f32::EPSILON {
            continue;
        }
        commands.spawn((
            Name::new("Torpedo Trail Segment"),
            TorpedoTrailSegment { length },
            Mesh3d(assets.mesh.clone()),
            MeshMaterial3d(assets.material.clone()),
            // Long on Z: `looking_to` points -Z along the span.
            Transform::from_translation(last + span * 0.5)
                .looking_to(span / length, Vec3::Y)
                .with_scale(Vec3::new(TRAIL_WIDTH, TRAIL_WIDTH, length)),
            NotShadowCaster,
            TempEntity(life),
        ));
    }
}

/// Thin every segment by [`trail_width`] over its [`TempEntity`] life. The
/// width only: the length is the ground the torpedo covered, and shrinking it
/// would open gaps in the ribbon.
pub(super) fn fade_torpedo_trail(
    mut q_segment: Query<(&TorpedoTrailSegment, &TempEntityState, &mut Transform)>,
) {
    for (segment, state, mut transform) in &mut q_segment {
        let width = trail_width(state.elapsed_secs(), state.duration().as_secs_f32());
        transform.scale = Vec3::new(width, width, segment.length);
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use bevy::time::TimeUpdateStrategy;

    use super::*;

    /// A rig with the trail observer and systems, the lifetime reaper, and a
    /// fixed 1/60 s frame.
    fn trail_app(quality: GraphicsQuality) -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.init_asset::<Mesh>();
        app.init_asset::<StandardMaterial>();
        app.add_plugins(TempEntityPlugin);
        app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(
            1.0 / 60.0,
        )));
        app.insert_resource(GraphicsBudget::for_quality(quality));
        app.init_resource::<TorpedoTrailAssets>();
        app.add_observer(on_torpedo_ignited_trail);
        app.add_systems(Update, (emit_torpedo_trail, fade_torpedo_trail).chain());
        // The first update of a manual clock advances by zero.
        app.update();
        app
    }

    /// A lit torpedo flying down -Z at 30 units a second.
    fn ignite(app: &mut App) -> Entity {
        let torpedo = app
            .world_mut()
            .spawn((TorpedoProjectileMarker, Transform::default()))
            .id();
        app.world_mut().trigger(TorpedoIgnited {
            torpedo,
            at: Vec3::ZERO,
        });
        app.world_mut().flush();
        torpedo
    }

    fn fly(app: &mut App, torpedo: Entity) {
        app.world_mut()
            .get_mut::<Transform>(torpedo)
            .expect("the torpedo is alive")
            .translation
            .z -= 0.5;
        app.update();
    }

    fn segments(app: &mut App) -> Vec<(f32, f32)> {
        app.world_mut()
            .query_filtered::<(&TempEntity, &TempEntityState), With<TorpedoTrailSegment>>()
            .iter(app.world())
            .map(|(life, state)| (life.0, state.elapsed_secs()))
            .collect()
    }

    /// The Low tier lays no trail at all, however long the torpedo flies.
    #[test]
    fn a_low_tier_torpedo_lays_no_trail() {
        let mut app = trail_app(GraphicsQuality::Low);
        let torpedo = ignite(&mut app);
        for _ in 0..120 {
            fly(&mut app, torpedo);
        }
        assert!(segments(&mut app).is_empty());
    }

    /// On High every segment lives the tier's trail length and no longer: none
    /// is ever older than its life, and the trail outlives the torpedo only by
    /// that long.
    #[test]
    fn a_high_tier_trail_segment_never_outlives_its_life() {
        let mut app = trail_app(GraphicsQuality::High);
        let life = GraphicsBudget::for_quality(GraphicsQuality::High).trail_seconds();
        let torpedo = ignite(&mut app);
        for _ in 0..180 {
            fly(&mut app, torpedo);
            for (total, age) in segments(&mut app) {
                assert_eq!(total, life);
                assert!(age <= total, "a segment aged {age} s of a {total} s life");
            }
        }
        assert!(
            !segments(&mut app).is_empty(),
            "a High torpedo lays a trail"
        );

        app.world_mut().entity_mut(torpedo).despawn();
        app.update();
        assert!(
            !segments(&mut app).is_empty(),
            "the trail outlives the torpedo"
        );
        for _ in 0..=(life * 60.0).ceil() as usize {
            app.update();
        }
        assert!(
            segments(&mut app).is_empty(),
            "every segment is gone after its life"
        );
    }
}
