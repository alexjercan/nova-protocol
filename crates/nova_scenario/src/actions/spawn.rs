//! Spawning and despawning scenario objects: the single-object config, the
//! scatter field, and the trigger area.

use avian3d::prelude::*;
use bevy::prelude::*;
use nova_events::prelude::*;

use crate::prelude::*;

/// Despawn the scenario object whose [`EntityId`] matches `id` (recursive,
/// so the object's whole child hierarchy goes with it). The complement of
/// `SpawnScenarioObject`, e.g. a salvage crate the script removes on pickup.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DespawnScenarioObjectActionConfig {
    /// The `EntityId` of the scoped object to despawn.
    pub id: String,
}

impl DespawnScenarioObjectActionConfig {
    /// Construct from a string slice.
    pub fn new(id: &str) -> Self {
        Self { id: id.to_string() }
    }
}

impl EventAction<NovaEventWorld> for DespawnScenarioObjectActionConfig {
    fn action(&self, world: &mut NovaEventWorld, _: &GameEventInfo) {
        let id = self.id.clone();
        debug!("DespawnScenarioObject: despawning '{}'", id);

        // The id -> Entity lookup needs world access, which push_command's
        // `&mut Commands` does not have - so the command queues a Command
        // closure that resolves and despawns in one step. The lookup is
        // gated on ScenarioScopedMarker: spaceship SECTIONS also carry
        // EntityId (their per-ship section ids like "controller"), and an
        // unscoped match on such an id would rip that section out of every
        // ship in the scene.
        world.push_command(move |commands| {
            commands.queue(move |world: &mut World| {
                let mut query =
                    world.query_filtered::<(Entity, &EntityId), With<ScenarioScopedMarker>>();
                let matches: Vec<Entity> = query
                    .iter(world)
                    .filter(|(_, entity_id)| entity_id.0 == id)
                    .map(|(entity, _)| entity)
                    .collect();
                if matches.is_empty() {
                    warn!(
                        "DespawnScenarioObject: no entity with id '{}'; check the scenario \
                         for a typo or a double despawn",
                        id
                    );
                }
                for entity in matches {
                    // get_entity_mut, not entity_mut: an earlier recursive
                    // despawn in this loop may have taken a matched descendant
                    // with it.
                    if let Ok(entity_mut) = world.get_entity_mut(entity) {
                        entity_mut.despawn();
                    }
                }
            });
        });
    }
}

/// A spawnable scenario object: the shared base (id, name, transform) plus the
/// kind-specific config that picks what to spawn.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ScenarioObjectConfig {
    /// The shared base fields every scenario object carries.
    pub base: BaseScenarioObjectConfig,
    /// Which kind of object to spawn and its per-kind config.
    pub kind: ScenarioObjectKind,
}

/// The fields every scenario object shares, regardless of kind: identity and
/// initial pose.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BaseScenarioObjectConfig {
    /// The object's scenario `EntityId`.
    pub id: String,
    /// The object's display name.
    pub name: String,
    /// The object's initial world position.
    pub position: Vec3,
    /// The object's initial world rotation.
    pub rotation: Quat,
}

/// Build the shared bundle every scenario object spawns with: scoped marker,
/// identity, transform, and visibility.
///
/// Deliberately carries NO body: a body is a per-kind decision, and three of the
/// five kinds are static. Each kind's bundle declares its own `RigidBody`.
pub fn base_scenario_object(config: &BaseScenarioObjectConfig) -> impl Bundle {
    (
        ScenarioScopedMarker,
        Name::new(config.name.clone()),
        EntityId::new(config.id.clone()),
        Transform::from_translation(config.position).with_rotation(config.rotation),
        Visibility::Visible,
    )
}

/// Which kind of scenario object to spawn, carrying that kind's config.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ScenarioObjectKind {
    /// An invisible authored point publishing a deterministic gravity well
    /// (camera framing, orbit targets) with no mesh, collider, or BodyRadius.
    Anchor(AnchorConfig),
    /// A destructible rock with a gravity well.
    Asteroid(AsteroidConfig),
    /// A ship built from sections, with a controller (None/Player/AI).
    Spaceship(SpaceshipConfig),
    /// A nav waypoint with an automatic HUD chip.
    Beacon(BeaconConfig),
    /// A proximity pickup crate that fires `OnEnter` when flown through.
    SalvageCrate(SalvageCrateConfig),
    /// An authored light - the scene's own key, rim, fill or lamp. A scene that
    /// spawns none renders black; the engine no longer supplies one.
    Light(LightConfig),
}

impl EventAction<NovaEventWorld> for ScenarioObjectConfig {
    fn action(&self, world: &mut NovaEventWorld, _info: &GameEventInfo) {
        let config = self.clone();
        debug!("SpawnScenarioObject: spawning '{}'", config.base.id);

        world.push_command(move |commands| {
            let mut entity_commands = commands.spawn(base_scenario_object(&config.base));

            match &config.kind {
                ScenarioObjectKind::Anchor(config) => {
                    entity_commands.insert(anchor_scenario_object(config.clone()));
                }
                ScenarioObjectKind::Asteroid(asteroid) => {
                    // The rock builds its own collider node here, in this
                    // batch, so the body never meets a physics tick without
                    // it - see `asteroid_scenario_object`. The seed resolves
                    // at the call site because a command has no RNG to draw
                    // from: authored wins, else it derives from the id.
                    let seed = asteroid
                        .seed
                        .unwrap_or_else(|| asteroid_seed_from_id(&config.base.id));
                    asteroid_scenario_object(&mut entity_commands, asteroid.clone(), seed);
                }
                ScenarioObjectKind::Spaceship(config) => {
                    entity_commands.insert(spaceship_scenario_object(config.clone()));
                    // The authored allegiance override. Ordering is safe
                    // either way: observer-queued commands (the controller
                    // marker whose requirement defaults Player/Enemy) apply
                    // BEFORE this queue's remaining commands (ledger:
                    // verify-engine-guarantees-in-source), and a plain
                    // insert overwrites the requirement default - so the
                    // authored side always wins.
                    if let Some(allegiance) = config.allegiance {
                        entity_commands.insert(allegiance);
                    }
                }
                ScenarioObjectKind::Beacon(config) => {
                    entity_commands.insert(beacon_scenario_object(config.clone()));
                }
                ScenarioObjectKind::SalvageCrate(config) => {
                    entity_commands.insert(salvage_crate_scenario_object(config.clone()));
                }
                ScenarioObjectKind::Light(config) => {
                    entity_commands.insert(light_scenario_object(config.clone()));
                }
            }
        });
    }
}

/// A volume to scatter objects within, for [`ScatterObjectsConfig`].
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ScatterRegion {
    /// An axis-aligned box; each object is placed uniformly per-axis in
    /// `[min, max]`.
    Box {
        /// The box's minimum corner.
        min: Vec3,
        /// The box's maximum corner.
        max: Vec3,
    },
    /// A horizontal annulus centred on `center`: uniform angle, radius in
    /// `[inner, outer]`, height in `[y_min, y_max]`, all relative to that
    /// centre.
    Ring {
        /// The annulus centre in world space. Omitted in RON, it is the origin.
        #[cfg_attr(feature = "serde", serde(default))]
        center: Vec3,
        /// The annulus inner radius.
        inner: f32,
        /// The annulus outer radius.
        outer: f32,
        /// The lower bound of the vertical (y) spread.
        y_min: f32,
        /// The upper bound of the vertical (y) spread.
        y_max: f32,
    },
}

impl ScatterRegion {
    /// Sample a position in the region. `random_in` guards empty ranges
    /// (`a >= b` yields `a`) so a degenerate authored region cannot panic.
    fn sample(&self, rng: &mut impl rand::Rng) -> Vec3 {
        fn random_in(rng: &mut impl rand::Rng, a: f32, b: f32) -> f32 {
            use rand::RngExt;
            if a < b {
                rng.random_range(a..b)
            } else {
                a
            }
        }
        match self {
            ScatterRegion::Box { min, max } => Vec3::new(
                random_in(rng, min.x, max.x),
                random_in(rng, min.y, max.y),
                random_in(rng, min.z, max.z),
            ),
            ScatterRegion::Ring {
                center,
                inner,
                outer,
                y_min,
                y_max,
            } => {
                let angle = random_in(rng, 0.0, std::f32::consts::TAU);
                let dist = random_in(rng, *inner, *outer);
                *center
                    + Vec3::new(
                        angle.cos() * dist,
                        random_in(rng, *y_min, *y_max),
                        angle.sin() * dist,
                    )
            }
        }
    }
}

/// The most objects one `ScatterObjects` action will spawn.
///
/// `count` is an unvalidated authored `u32`, and the spawn loop allocates an
/// entity per iteration - so without this an authored `count: 50000000` OOMs
/// from content that passed both the static lint and the runtime gate. An
/// anti-absurdity cap, not a quota: the densest shipped field is far below it.
pub const MAX_SCATTER_COUNT: u32 = 4096;

/// Spawn `count` copies of a template object scattered through a region, with a
/// deterministic seed so the layout is reproducible across loads. Each copy is a
/// clone of `template` with `base.id = "{id_prefix}{i}"` and a sampled position;
/// when `asteroid_radius` is set and the template is an asteroid, its radius is
/// randomized too. This is the declarative form of a procedural asteroid field.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ScatterObjectsConfig {
    /// The id prefix each copy gets (`"{id_prefix}{i}"`).
    pub id_prefix: String,
    /// How many copies to spawn, capped at [`MAX_SCATTER_COUNT`].
    pub count: u32,
    /// The RNG seed, so the layout is reproducible across loads.
    pub seed: u64,
    /// The region copies are scattered within.
    pub region: ScatterRegion,
    /// The template object each copy clones.
    pub template: ScenarioObjectConfig,
    /// If set and `template.kind` is an asteroid, randomize each rock's radius in
    /// this `[lo, hi]` range.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub asteroid_radius: Option<(f32, f32)>,
    /// Minimum centre-to-centre distance (world units) between a copy of this
    /// scatter and EVERY body already scattered this scenario - this action's
    /// earlier copies and every earlier scatter's. Uniform sampling puts bodies
    /// on top of each other, and
    /// overlapping DYNAMIC bodies (a scattered rock is one) are shoved apart on
    /// the first physics step hard enough to damage or destroy each other - a
    /// field that explodes as it spawns. Author it as the widest two bodies
    /// side by side: for asteroids the collider reaches
    /// `radius * ASTEROID_GEOMETRIC_FACTOR_MAX`, not `radius`.
    ///
    /// A sample that cannot clear the placed copies within
    /// [`Self::SEPARATION_ATTEMPTS`] tries is DROPPED, so a region too small
    /// for `count` bodies yields fewer of them rather than an overlap.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub min_separation: Option<f32>,
}

impl ScatterObjectsConfig {
    /// Rejection-sampling budget per copy when [`Self::min_separation`] is set.
    /// Bounded so a hopeless region cannot hang the spawn; the layout stays
    /// deterministic because a rejected sample still advances the seeded RNG.
    pub const SEPARATION_ATTEMPTS: u32 = 64;

    /// One position at least `min_separation` from every `placed` one, or
    /// `None` when the budget runs out. Without a separation the first sample
    /// is always taken.
    fn sample_clear_of(&self, placed: &[Vec3], rng: &mut impl rand::Rng) -> Option<Vec3> {
        let Some(separation) = self.min_separation.filter(|s| *s > 0.0) else {
            return Some(self.region.sample(rng));
        };
        let min_sq = separation * separation;
        (0..Self::SEPARATION_ATTEMPTS)
            .map(|_| self.region.sample(rng))
            .find(|candidate| {
                placed
                    .iter()
                    .all(|p| p.distance_squared(*candidate) >= min_sq)
            })
    }
}

impl EventAction<NovaEventWorld> for ScatterObjectsConfig {
    fn action(&self, world: &mut NovaEventWorld, info: &GameEventInfo) {
        use rand::{Rng, RngExt, SeedableRng};
        let mut rng = rand::rngs::StdRng::seed_from_u64(self.seed);
        // Per-rock silhouette seeds come from a SEPARATE stream: drawing them
        // from the position RNG would shift every layout authored before
        // silhouettes became deterministic. The salt only decorrelates the two
        // streams; any fixed value works.
        const SILHOUETTE_SALT: u64 = 0x51E0_0E77_E5EE_D000;
        let mut silhouette_rng = rand::rngs::StdRng::seed_from_u64(self.seed ^ SILHOUETTE_SALT);
        // NOTE: always the authored count, never thinned by a graphics-quality
        // tier - scatter is gameplay content (asteroid / debris fields).
        // Bounded, though: `count` is an unvalidated authored u32 driving a
        // spawn loop, so an absurd one OOMs from data that passed every gate.
        let count = self.count.min(MAX_SCATTER_COUNT);
        if count != self.count {
            warn!(
                "ScatterObjects: '{}' asks for {} objects; clamped to {MAX_SCATTER_COUNT}",
                self.id_prefix, self.count
            );
        }
        debug!(
            "ScatterObjects: scattering {} '{}' objects (seed {})",
            count, self.id_prefix, self.seed
        );

        // Seeded with what earlier scatters placed, so abutting sibling fields
        // (a belt's knots) cannot drop rocks into each other.
        let mut placed: Vec<Vec3> = world.scatter_placements().to_vec();
        for i in 0..count {
            let mut object = self.template.clone();
            object.base.id = format!("{}{}", self.id_prefix, i);
            object.base.name = format!("{} {}", self.template.base.name, i);
            // Drawn per index, before the drop check, so copy N keeps its
            // silhouette even when an earlier copy is dropped by separation.
            let silhouette_seed = silhouette_rng.next_u32();
            if let ScenarioObjectKind::Asteroid(asteroid) = &mut object.kind {
                // An authored template seed means "every copy identical" and
                // is kept; the default is a stable per-rock silhouette.
                asteroid.seed = asteroid.seed.or(Some(silhouette_seed));
            }
            let Some(position) = self.sample_clear_of(&placed, &mut rng) else {
                debug!(
                    "ScatterObjects: dropped '{}{}' - no position clearing the \
                     {}u separation in {} attempts",
                    self.id_prefix,
                    i,
                    self.min_separation.unwrap_or_default(),
                    Self::SEPARATION_ATTEMPTS
                );
                continue;
            };
            placed.push(position);
            world.push_scatter_placement(position);
            object.base.position = position;

            if let (Some((lo, hi)), ScenarioObjectKind::Asteroid(asteroid)) =
                (self.asteroid_radius, &mut object.kind)
            {
                asteroid.radius = if lo < hi {
                    rng.random_range(lo..hi)
                } else {
                    lo
                };
            }

            // Reuse the ordinary spawn path so scatter and SpawnScenarioObject
            // stay identical in how they build an object.
            object.action(world, info);
        }
    }
}

/// A spherical sensor zone that drives `OnEnter`/`OnExit` when a body crosses
/// its boundary.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ScenarioAreaConfig {
    /// The area's scenario `EntityId` (the `id` reported by `OnEnter`/`OnExit`).
    pub id: String,
    /// The area's display name.
    pub name: String,
    /// The area's world position (sphere centre).
    pub position: Vec3,
    /// The area's world rotation.
    pub rotation: Quat,
    /// The sphere radius.
    pub radius: f32,
}

impl EventAction<NovaEventWorld> for ScenarioAreaConfig {
    fn action(&self, world: &mut NovaEventWorld, _info: &GameEventInfo) {
        let config = self.clone();
        debug!(
            "CreateScenarioArea: creating area '{}' (radius: {})",
            config.id, config.radius
        );

        world.push_command(move |commands| {
            commands.spawn((
                ScenarioScopedMarker,
                ScenarioAreaMarker,
                Name::new(config.name.clone()),
                EntityId::new(config.id.clone()),
                Transform::from_translation(config.position).with_rotation(config.rotation),
                RigidBody::Static,
                Collider::sphere(config.radius),
                Sensor,
                Visibility::Visible,
            ));
        });
    }
}

#[cfg(test)]
mod tests {
    use nova_gameplay::prelude::*;

    use super::*;
    // Apply EVERY queued command: the production sync drains under a per-frame
    // time budget, so a rig asserting on a whole multi-object batch runs it to
    // settled. Shared with nova_authoring's beat walks, which need the same
    // thing through an `App`.
    use crate::test_support::drain_spawns as drain;

    /// The authored `SpaceshipConfig.allegiance` override, through the
    /// production spawn path: a NEUTRAL AI ship ends NEUTRAL even though
    /// `AISpaceshipMarker` requires `Allegiance = Enemy` - the spawn action's
    /// explicit insert wins over the requirement default regardless of command
    /// ordering (observer commands apply before the queue's remaining commands,
    /// and a plain insert overwrites). Companion delivery guard: the same spawn
    /// WITHOUT the override ends Enemy, so the Neutral assert cannot pass
    /// vacuously.
    #[test]
    fn authored_allegiance_overrides_the_controller_default() {
        fn spawn_ship(allegiance: Option<Allegiance>) -> Option<Allegiance> {
            let mut app = App::new();
            app.add_plugins(MinimalPlugins);
            app.add_plugins(crate::objects::spaceship::SpaceshipPlugin);
            app.init_resource::<NovaEventWorld>();
            app.init_resource::<GameObjectives>();

            let config = ScenarioObjectConfig {
                base: BaseScenarioObjectConfig {
                    id: "ship".to_string(),
                    name: "Ship".to_string(),
                    position: Vec3::ZERO,
                    rotation: Quat::IDENTITY,
                },
                kind: ScenarioObjectKind::Spaceship(SpaceshipConfig {
                    controller: SpaceshipController::AI(AIControllerConfig::default()),
                    allegiance,
                    ..default()
                }),
            };
            {
                let mut world = app.world_mut().resource_mut::<NovaEventWorld>();
                EventActionConfig::SpawnScenarioObject(config)
                    .action(&mut world, &GameEventInfo { data: None });
            }
            NovaEventWorld::state_to_world_system(app.world_mut());
            app.update();

            let mut q = app
                .world_mut()
                .query_filtered::<&Allegiance, With<SpaceshipRootMarker>>();
            q.iter(app.world()).next().copied()
        }

        assert_eq!(
            spawn_ship(Some(Allegiance::Neutral)),
            Some(Allegiance::Neutral),
            "the authored override survives the AI marker's Enemy default"
        );
        assert_eq!(
            spawn_ship(None),
            Some(Allegiance::Enemy),
            "delivery guard: without the override the AI default applies"
        );
        // The ALLY variant: an AI-flown ship on the player's side - Lifeline's
        // convoy - rides the same path; the relation-model consequences are
        // pinned in nova_gameplay's ally_relation_tests.
        assert_eq!(
            spawn_ship(Some(Allegiance::Player)),
            Some(Allegiance::Player),
            "an authored Player allegiance survives the AI marker's Enemy default"
        );
    }

    /// The behaviour the physics pair buys: a moving scenario body's Transform
    /// advances on EVERY render frame, not just on fixed physics ticks. 4 ms
    /// frames against the 15.6 ms tick mean at most one tick lands inside any
    /// 3-frame span - without easing at least two consecutive frames would show
    /// identical translations.
    ///
    /// Spawned as an ASTEROID, not as the bare base bundle: the body and the
    /// interpolation are a per-kind decision now, and the base carries neither.
    #[test]
    fn dynamic_scenario_bodies_move_between_fixed_ticks() {
        use core::time::Duration;

        use bevy::time::TimeUpdateStrategy;

        let mut app = App::new();
        // Mirrors the integrity physics harness: MeshPlugin because avian's
        // collider-from-mesh backend reads AssetEvent<Mesh> at startup.
        app.add_plugins((
            MinimalPlugins,
            TransformPlugin,
            bevy::asset::AssetPlugin::default(),
            bevy::mesh::MeshPlugin,
            PhysicsPlugins::default(),
        ));
        app.insert_resource(Gravity(Vec3::ZERO));
        app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
            0.004,
        )));
        app.finish();

        let world = app.world_mut();
        let body = world
            .spawn((
                base_scenario_object(&BaseScenarioObjectConfig {
                    id: "mover".to_string(),
                    name: "Mover".to_string(),
                    position: Vec3::ZERO,
                    rotation: Quat::IDENTITY,
                }),
                Collider::cuboid(1.0, 1.0, 1.0),
                ColliderDensity(1.0),
                LinearVelocity(Vec3::X * 10.0),
            ))
            .id();
        {
            let mut commands = world.commands();
            let mut entity_commands = commands.entity(body);
            asteroid_scenario_object(
                &mut entity_commands,
                AsteroidConfig {
                    impact_sound: None,
                    destroy_sound: None,
                    radius: 1.0,
                    texture: AssetRef::default(),
                    health: 100.0,
                    mass: None,
                    invulnerable: false,
                    seed: None,
                    lock_signature: None,
                },
                5,
            );
        }
        world.flush();

        // Warm up past two fixed ticks so the easing has start+end states.
        for _ in 0..10 {
            app.update();
        }

        // Four consecutive 4 ms frames: with easing every frame advances the
        // translation; stair-stepping would repeat a value.
        let mut positions = Vec::new();
        for _ in 0..4 {
            app.update();
            positions.push(app.world().get::<Transform>(body).unwrap().translation.x);
        }
        for pair in positions.windows(2) {
            assert!(
                pair[1] > pair[0],
                "translation must advance every render frame, got {positions:?}"
            );
        }
    }

    /// The despawn action removes exactly the scenario object whose id
    /// matches - and ONLY scenario-scoped entities: spaceship sections
    /// carry EntityId too (per-ship ids like "controller"), and an
    /// unscoped match would rip that section out of every ship.
    #[test]
    fn despawn_action_removes_the_scoped_object_by_id() {
        use nova_events::prelude::EventWorld;

        let mut world = World::new();
        world.init_resource::<NovaEventWorld>();
        world.init_resource::<GameObjectives>();

        let crate_1 = world
            .spawn((ScenarioScopedMarker, EntityId::new("crate_1".to_string())))
            .id();
        let crate_2 = world
            .spawn((ScenarioScopedMarker, EntityId::new("crate_2".to_string())))
            .id();
        // An unscoped entity with a colliding id - a stand-in for a ship
        // section - must survive.
        let section = world.spawn(EntityId::new("crate_1".to_string())).id();

        let action = DespawnScenarioObjectActionConfig::new("crate_1");
        let mut event_world = world.resource_mut::<NovaEventWorld>();
        action.action(&mut event_world, &GameEventInfo::default());

        // The action only queues; the drain in state_to_world applies it.
        NovaEventWorld::state_to_world_system(&mut world);

        assert!(
            world.get_entity(crate_1).is_err(),
            "the matching scoped object despawns"
        );
        assert!(
            world.get_entity(crate_2).is_ok(),
            "other scoped objects survive"
        );
        assert!(
            world.get_entity(section).is_ok(),
            "an unscoped entity with the same id (a ship section) survives"
        );
    }

    /// A missing id is a warning, not a crash: the drain must complete and
    /// unrelated entities survive (double-despawn / typo path).
    #[test]
    fn despawn_action_with_missing_id_is_harmless() {
        use nova_events::prelude::EventWorld;

        let mut world = World::new();
        world.init_resource::<NovaEventWorld>();
        world.init_resource::<GameObjectives>();

        let bystander = world
            .spawn((ScenarioScopedMarker, EntityId::new("beacon_1".to_string())))
            .id();

        let action = DespawnScenarioObjectActionConfig::new("no_such_id");
        let mut event_world = world.resource_mut::<NovaEventWorld>();
        action.action(&mut event_world, &GameEventInfo::default());
        NovaEventWorld::state_to_world_system(&mut world);

        assert!(world.get_entity(bystander).is_ok());
    }

    /// Scatter is deterministic: the same seed yields the same layout every
    /// load (a data file must be reproducible), and samples stay in bounds.
    #[test]
    fn scatter_region_sampling_is_deterministic_and_bounded() {
        use rand::SeedableRng;

        let region = ScatterRegion::Box {
            min: Vec3::new(-10.0, -2.0, -10.0),
            max: Vec3::new(10.0, 2.0, 10.0),
        };

        let sample_10 = || {
            let mut rng = rand::rngs::StdRng::seed_from_u64(42);
            (0..10).map(|_| region.sample(&mut rng)).collect::<Vec<_>>()
        };
        let a = sample_10();
        let b = sample_10();
        assert_eq!(a, b, "same seed must produce the same positions");

        for p in &a {
            assert!(p.x >= -10.0 && p.x <= 10.0, "x in box: {p:?}");
            assert!(p.y >= -2.0 && p.y <= 2.0, "y in box: {p:?}");
            assert!(p.z >= -10.0 && p.z <= 10.0, "z in box: {p:?}");
        }
    }

    /// A degenerate region (min == max on an axis) does not panic; it pins that
    /// axis to the value.
    #[test]
    fn scatter_region_degenerate_axis_does_not_panic() {
        use rand::SeedableRng;

        let region = ScatterRegion::Box {
            min: Vec3::new(5.0, 0.0, 5.0),
            max: Vec3::new(5.0, 0.0, 5.0),
        };
        let mut rng = rand::rngs::StdRng::seed_from_u64(1);
        let p = region.sample(&mut rng);
        assert_eq!(p, Vec3::new(5.0, 0.0, 5.0));
    }

    /// A ring with a non-zero centre samples the annulus AROUND that centre, not
    /// around the origin - what an authored belt around a distant body needs.
    #[test]
    fn scatter_region_ring_samples_around_its_center() {
        use rand::SeedableRng;

        let center = Vec3::new(500.0, -40.0, -560.0);
        let region = ScatterRegion::Ring {
            center,
            inner: 620.0,
            outer: 900.0,
            y_min: -160.0,
            y_max: 160.0,
        };

        let mut rng = rand::rngs::StdRng::seed_from_u64(7);
        for _ in 0..64 {
            let p = region.sample(&mut rng);
            let offset = p - center;
            let planar = Vec2::new(offset.x, offset.z).length();
            assert!(
                (620.0..=900.0).contains(&planar),
                "planar distance from centre out of the annulus: {planar} ({p:?})"
            );
            assert!(offset.y >= -160.0 && offset.y <= 160.0, "y spread: {p:?}");
        }
    }

    /// `min_separation` is what keeps a scattered field from spawning inside
    /// itself: uniform sampling WILL put two bodies on top of each other, and
    /// two overlapping dynamic rocks are shoved apart hard enough to destroy
    /// each other on the first physics step. Sampled positions must respect it,
    /// and a region with no room drops copies rather than overlapping them.
    #[test]
    fn scatter_min_separation_is_respected_and_never_hangs() {
        use rand::SeedableRng;

        let scatter = |count: u32, min_separation: Option<f32>| {
            let config = ScatterObjectsConfig {
                id_prefix: "rock_".to_string(),
                count,
                seed: 11,
                region: ScatterRegion::Box {
                    min: Vec3::new(-100.0, -20.0, -100.0),
                    max: Vec3::new(100.0, 20.0, 100.0),
                },
                template: ScenarioObjectConfig {
                    base: BaseScenarioObjectConfig {
                        id: "rock".to_string(),
                        name: "Rock".to_string(),
                        position: Vec3::ZERO,
                        rotation: Quat::IDENTITY,
                    },
                    kind: ScenarioObjectKind::Light(LightConfig::Directional {
                        illuminance: 1000.0,
                        color: Color::WHITE,
                        shadows: false,
                        aim: None,
                    }),
                },
                asteroid_radius: None,
                min_separation,
            };
            let mut rng = rand::rngs::StdRng::seed_from_u64(config.seed);
            let mut placed: Vec<Vec3> = Vec::new();
            for _ in 0..config.count {
                if let Some(p) = config.sample_clear_of(&placed, &mut rng) {
                    placed.push(p);
                }
            }
            placed
        };

        let placed = scatter(12, Some(40.0));
        assert_eq!(placed.len(), 12, "the box has room for 12 at 40u apart");
        for (i, a) in placed.iter().enumerate() {
            for b in placed.iter().skip(i + 1) {
                assert!(
                    a.distance(*b) >= 40.0,
                    "two copies landed {:.1}u apart, under the 40u separation",
                    a.distance(*b)
                );
            }
        }

        // A separation the region cannot satisfy drops the copies it cannot
        // place - it does not loop forever and does not overlap them.
        let crowded = scatter(40, Some(150.0));
        assert!(
            crowded.len() < 40,
            "an impossible separation must drop copies, got all {} placed",
            crowded.len()
        );
        for (i, a) in crowded.iter().enumerate() {
            for b in crowded.iter().skip(i + 1) {
                assert!(a.distance(*b) >= 150.0);
            }
        }

        // No separation authored: every copy is placed, as before the field.
        assert_eq!(scatter(40, None).len(), 40);
    }

    /// Separation holds ACROSS scatters, not just within one. A belt is
    /// authored as sibling knots whose boxes abut - if each scatter only
    /// checked its own copies, the seam between two knots would spawn rocks
    /// inside each other, which is the whole failure `min_separation` exists to
    /// prevent. Two scatters over the SAME box is the worst case.
    #[test]
    fn separation_holds_across_sibling_scatters() {
        let separation = 40.0;
        let scatter = |id_prefix: &str, seed: u64| ScatterObjectsConfig {
            id_prefix: id_prefix.to_string(),
            count: 8,
            seed,
            region: ScatterRegion::Box {
                min: Vec3::new(-100.0, -20.0, -100.0),
                max: Vec3::new(100.0, 20.0, 100.0),
            },
            template: ScenarioObjectConfig {
                base: BaseScenarioObjectConfig {
                    id: "rock".to_string(),
                    name: "Rock".to_string(),
                    position: Vec3::ZERO,
                    rotation: Quat::IDENTITY,
                },
                kind: ScenarioObjectKind::Light(LightConfig::Directional {
                    illuminance: 1000.0,
                    color: Color::WHITE,
                    shadows: false,
                    aim: None,
                }),
            },
            asteroid_radius: None,
            min_separation: Some(separation),
        };

        let mut event_world = NovaEventWorld::default();
        scatter("knot_a_", 11).action(&mut event_world, &GameEventInfo::default());
        let after_first = event_world.scatter_placements().len();
        scatter("knot_b_", 22).action(&mut event_world, &GameEventInfo::default());

        let placed = event_world.scatter_placements();
        assert_eq!(after_first, 8, "the first scatter placed all 8");
        assert_eq!(
            placed.len(),
            16,
            "both scatters placed all 8, got {placed:?}"
        );
        for (i, a) in placed.iter().enumerate() {
            for b in placed.iter().skip(i + 1) {
                assert!(
                    a.distance(*b) >= separation,
                    "two copies landed {:.1}u apart, under the {separation}u separation",
                    a.distance(*b)
                );
            }
        }

        // Teardown drops them, or the next load of the same scenario would
        // scatter around a field that no longer exists.
        event_world.clear();
        assert!(event_world.scatter_placements().is_empty());
    }

    /// `center` is `serde(default)`, so mod RON written before the field
    /// deserializes unchanged - as an origin-centred ring.
    #[cfg(feature = "serde")]
    #[test]
    fn scatter_region_ring_center_defaults_to_zero_in_ron() {
        let region: ScatterRegion =
            ron::from_str("Ring(inner: 10.0, outer: 20.0, y_min: -1.0, y_max: 1.0)")
                .expect("deserialize a ring without a centre");
        match region {
            ScatterRegion::Ring { center, .. } => assert_eq!(center, Vec3::ZERO),
            other => panic!("expected a ring: {other:?}"),
        }
    }

    #[cfg(feature = "serde")]
    #[test]
    fn scatter_objects_config_round_trips_through_ron() {
        let config = ScatterObjectsConfig {
            id_prefix: "rock_".to_string(),
            count: 12,
            seed: 7,
            region: ScatterRegion::Ring {
                center: Vec3::new(10.0, 0.0, -20.0),
                inner: 100.0,
                outer: 150.0,
                y_min: -20.0,
                y_max: 20.0,
            },
            template: ScenarioObjectConfig {
                base: BaseScenarioObjectConfig {
                    id: "rock".to_string(),
                    name: "Rock".to_string(),
                    position: Vec3::ZERO,
                    rotation: Quat::IDENTITY,
                },
                kind: ScenarioObjectKind::Asteroid(AsteroidConfig {
                    impact_sound: None,
                    destroy_sound: None,
                    radius: 2.0,
                    texture: nova_gameplay::prelude::AssetRef::from("textures/asteroid.png"),
                    health: 100.0,
                    mass: None,
                    invulnerable: false,
                    seed: None,
                    lock_signature: None,
                }),
            },
            asteroid_radius: Some((1.0, 3.0)),
            min_separation: None,
        };

        let ron = ron::to_string(&config).expect("serialize");
        let back: ScatterObjectsConfig = ron::from_str(&ron).expect("deserialize");
        assert_eq!(back.id_prefix, "rock_");
        assert_eq!(back.count, 12);
        assert_eq!(back.seed, 7);
        assert_eq!(back.asteroid_radius, Some((1.0, 3.0)));
        // The nested enum fields most likely to regress in a serde change: the
        // region variant and the template's asset ref must survive intact.
        match back.region {
            ScatterRegion::Ring {
                center,
                inner,
                outer,
                y_min,
                y_max,
            } => {
                assert_eq!(center, Vec3::new(10.0, 0.0, -20.0));
                assert_eq!((inner, outer, y_min, y_max), (100.0, 150.0, -20.0, 20.0));
            }
            other => panic!("region variant changed on round-trip: {other:?}"),
        }
        match &back.template.kind {
            ScenarioObjectKind::Asteroid(asteroid) => {
                assert_eq!(asteroid.texture.path(), Some("textures/asteroid.png"))
            }
            other => panic!("template kind changed on round-trip: {other:?}"),
        }
    }

    /// The scatter ACTION spawns exactly `count` scoped objects, each with an id
    /// under the prefix, a position inside the region, and a radius in range.
    /// Mirrors the despawn harness: fire into a `NovaEventWorld`, drain, assert on
    /// the world. Guards the spawn loop that only the windowed example exercised.
    #[test]
    fn scatter_action_spawns_count_objects_in_region() {
        let region_min = Vec3::new(-10.0, -5.0, -10.0);
        let region_max = Vec3::new(10.0, 5.0, 10.0);
        let config = ScatterObjectsConfig {
            id_prefix: "rock_".to_string(),
            count: 8,
            seed: 123,
            region: ScatterRegion::Box {
                min: region_min,
                max: region_max,
            },
            template: ScenarioObjectConfig {
                base: BaseScenarioObjectConfig {
                    id: "rock".to_string(),
                    name: "Rock".to_string(),
                    position: Vec3::ZERO,
                    rotation: Quat::IDENTITY,
                },
                kind: ScenarioObjectKind::Asteroid(AsteroidConfig {
                    impact_sound: None,
                    destroy_sound: None,
                    radius: 2.0,
                    texture: nova_gameplay::prelude::AssetRef::default(),
                    health: 100.0,
                    mass: None,
                    invulnerable: false,
                    seed: None,
                    lock_signature: None,
                }),
            },
            asteroid_radius: Some((1.0, 3.0)),
            min_separation: None,
        };

        let mut world = World::new();
        world.init_resource::<NovaEventWorld>();
        world.init_resource::<GameObjectives>();
        {
            let mut event_world = world.resource_mut::<NovaEventWorld>();
            config.action(&mut event_world, &GameEventInfo::default());
        }
        // The action only queues; the drain in state_to_world applies the spawns.
        drain(&mut world);

        let mut query = world
            .query_filtered::<(&EntityId, &Transform, &AsteroidRadius), With<AsteroidMarker>>();
        let mut ids: Vec<String> = Vec::new();
        for (id, transform, radius) in query.iter(&world) {
            let p = transform.translation;
            assert!(
                p.x >= region_min.x && p.x <= region_max.x,
                "x in region: {p:?}"
            );
            assert!(
                p.y >= region_min.y && p.y <= region_max.y,
                "y in region: {p:?}"
            );
            assert!(
                p.z >= region_min.z && p.z <= region_max.z,
                "z in region: {p:?}"
            );
            assert!(
                radius.0 >= 1.0 && radius.0 <= 3.0,
                "radius in range: {}",
                radius.0
            );
            assert!(id.0.starts_with("rock_"), "id has the prefix: {}", id.0);
            ids.push(id.0.clone());
        }

        assert_eq!(ids.len(), 8, "scatter spawns exactly `count` objects");
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), 8, "scattered ids are unique (no collision)");
    }

    /// Scatter fills each rock's silhouette seed deterministically from its
    /// own authored seed: the same action produces the same id -> seed map on
    /// every load, and an authored template seed (every copy identical) is
    /// kept. The seeds come from a stream SEPARATE from position sampling, so
    /// enabling them cannot shift layouts authored before the field existed.
    #[test]
    fn scatter_assigns_deterministic_silhouette_seeds() {
        let config = |template_seed: Option<u32>| ScatterObjectsConfig {
            id_prefix: "rock_".to_string(),
            count: 6,
            seed: 123,
            region: ScatterRegion::Box {
                min: Vec3::new(-10.0, -5.0, -10.0),
                max: Vec3::new(10.0, 5.0, 10.0),
            },
            template: ScenarioObjectConfig {
                base: BaseScenarioObjectConfig {
                    id: "rock".to_string(),
                    name: "Rock".to_string(),
                    position: Vec3::ZERO,
                    rotation: Quat::IDENTITY,
                },
                kind: ScenarioObjectKind::Asteroid(AsteroidConfig {
                    impact_sound: None,
                    destroy_sound: None,
                    radius: 2.0,
                    texture: nova_gameplay::prelude::AssetRef::default(),
                    health: 100.0,
                    mass: None,
                    invulnerable: false,
                    seed: template_seed,
                    lock_signature: None,
                }),
            },
            asteroid_radius: None,
            min_separation: None,
        };

        let run = |config: &ScatterObjectsConfig| -> Vec<(String, u32)> {
            let mut world = World::new();
            world.init_resource::<NovaEventWorld>();
            world.init_resource::<GameObjectives>();
            {
                let mut event_world = world.resource_mut::<NovaEventWorld>();
                config.action(&mut event_world, &GameEventInfo::default());
            }
            drain(&mut world);
            let mut query =
                world.query_filtered::<(&EntityId, &AsteroidSeed), With<AsteroidMarker>>();
            let mut seeds: Vec<(String, u32)> = query
                .iter(&world)
                .map(|(id, seed)| (id.0.clone(), **seed))
                .collect();
            seeds.sort();
            seeds
        };

        let derived = config(None);
        let first = run(&derived);
        assert_eq!(first.len(), 6);
        let distinct: std::collections::HashSet<_> = first.iter().map(|(_, seed)| *seed).collect();
        assert!(
            distinct.len() > 1,
            "per-rock seeds differ; identical copies are the AUTHORED case"
        );
        assert_eq!(first, run(&derived), "the id -> seed map is reproducible");

        let authored = run(&config(Some(42)));
        assert!(
            authored.iter().all(|(_, seed)| *seed == 42),
            "an authored template seed is kept on every copy: {authored:?}"
        );
    }

    /// Scatter is gameplay content, so it spawns the full authored count on
    /// EVERY graphics tier. Regression: even with the cheapest (Low)
    /// [`GraphicsBudget`] inserted and carried into the event world, the field
    /// is not thinned. Mirrors the full-count harness above with a Low budget
    /// inserted first, to prove the budget has no effect on scatter counts.
    #[test]
    fn scatter_action_ignores_graphics_budget() {
        use nova_gameplay::prelude::{GraphicsBudget, GraphicsQuality};

        let authored_count = 20u32;
        let config = ScatterObjectsConfig {
            id_prefix: "rock_".to_string(),
            count: authored_count,
            seed: 123,
            region: ScatterRegion::Box {
                min: Vec3::new(-10.0, -5.0, -10.0),
                max: Vec3::new(10.0, 5.0, 10.0),
            },
            template: ScenarioObjectConfig {
                base: BaseScenarioObjectConfig {
                    id: "rock".to_string(),
                    name: "Rock".to_string(),
                    position: Vec3::ZERO,
                    rotation: Quat::IDENTITY,
                },
                kind: ScenarioObjectKind::Asteroid(AsteroidConfig {
                    impact_sound: None,
                    destroy_sound: None,
                    radius: 2.0,
                    texture: nova_gameplay::prelude::AssetRef::default(),
                    health: 100.0,
                    mass: None,
                    invulnerable: false,
                    seed: None,
                    lock_signature: None,
                }),
            },
            asteroid_radius: Some((1.0, 3.0)),
            min_separation: None,
        };

        // The cheapest tier: if any preset were going to thin scatter, this is the
        // one that would. It must not.
        let low_budget = GraphicsBudget::for_quality(GraphicsQuality::Low);

        let mut world = World::new();
        world.init_resource::<NovaEventWorld>();
        world.init_resource::<GameObjectives>();
        world.insert_resource(low_budget);
        // Pulls the budget into the event world, exactly as the PostUpdate chain
        // does before the queue processes. This is a no-op now that scatter
        // ignores the budget - kept to prove that even a Low budget present in the
        // world does not thin the field.
        NovaEventWorld::world_to_state_system(&mut world);
        {
            let mut event_world = world.resource_mut::<NovaEventWorld>();
            config.action(&mut event_world, &GameEventInfo::default());
        }
        drain(&mut world);

        let mut query = world.query_filtered::<&EntityId, With<AsteroidMarker>>();
        let spawned = query.iter(&world).count();
        assert_eq!(
            spawned as u32, authored_count,
            "scatter spawns the full authored count ({authored_count}) even on Low - it is never thinned"
        );
    }
}
