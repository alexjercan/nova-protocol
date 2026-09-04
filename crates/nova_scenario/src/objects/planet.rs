//! The planet SCENARIO OBJECT: an authored world with a gravity well.
//!
//! `planet_type` owns what a world is made of and `planet_surface` owns the
//! shape and the material. This module is the third piece: the components a
//! spawned planet carries, the well it becomes, and the plugin that draws it.
//!
//! # Why this is not just an asteroid with a nicer material
//!
//! An asteroid's `radius` is a DESIGNATION. Its noise mesh reaches several
//! times past the unit sphere, so the geometric [`BodyRadius`] the sim
//! actually measures from is `radius * unit_extent`, with `unit_extent`
//! somewhere in `[3.5, 6.0]` depending on the seed
//! ([`ASTEROID_GEOMETRIC_FACTOR_MIN`](super::asteroid::ASTEROID_GEOMETRIC_FACTOR_MIN)).
//!
//! A planet is a sphere. Its mesh spans `1 - relief` to `1 + relief` around
//! the unit sphere, so its `unit_extent` is `1 + relief` - call it 1.05. The
//! DERIVATION is identical; the mesh is not. Porting a rock to a planet
//! therefore means porting its derived body radius, not the number in its
//! config, or the well, the sphere of influence and every orbit ring inside it
//! shrink by a factor of five. See `PLANETOID_BODY_RADIUS` in
//! `nova_authoring`'s stage builders for the authored consequence.

use avian3d::prelude::*;
use bevy::prelude::*;
use nova_events::prelude::*;
use nova_gameplay::prelude::*;
use nova_hud::prelude::*;
use nova_ship::prelude::*;

use super::{planet_surface::prelude::*, planet_type::prelude::*};

/// What the crate root re-exports for this module.
pub mod prelude {
    pub use super::{
        planet_scenario_object, PlanetInvulnerable, PlanetMarker, PlanetMass, PlanetPlugin,
        PlanetRadius, PlanetRenderBody,
    };
}

/// On the root of an authored planet.
#[derive(Component, Clone, Copy, Debug, Default, Reflect)]
#[reflect(Component)]
pub struct PlanetMarker;

/// The authored MEAN radius, in world units. The geometric surface is the
/// separately derived [`BodyRadius`], which is this times `1 + relief`.
#[derive(Component, Clone, Copy, Debug, Deref, Reflect)]
#[reflect(Component)]
pub struct PlanetRadius(pub f32);

/// The authored well strength, or `None` to fall back to the global rule.
#[derive(Component, Clone, Copy, Debug, Deref, Reflect)]
#[reflect(Component)]
pub struct PlanetMass(pub Option<f32>);

/// Whether weapons fire leaves this body alone.
#[derive(Component, Clone, Copy, Debug, Deref, Reflect)]
#[reflect(Component)]
pub struct PlanetInvulnerable(pub bool);

/// The built surface, parked on the render child until
/// [`insert_planet_render`] can reach `Assets` and turn it into handles.
///
/// Carries the whole [`PlanetVisual`] rather than a mesh alone: a planet's
/// material is generated beside its mesh from the same seeded draw, so
/// splitting them would mean generating the surface twice.
#[derive(Component, Clone, Debug, Deref)]
pub struct PlanetRenderBody(pub PlanetVisual);

/// Build one authored planet on `entity`.
///
/// Takes an [`EntityCommands`] rather than returning a bundle, for the same
/// reason [`asteroid_scenario_object`](super::asteroid::asteroid_scenario_object)
/// does: the collider child has to land in the same command batch as the
/// root's `RigidBody`, or avian computes the mass twice.
pub fn planet_scenario_object(entity: &mut EntityCommands, config: PlanetConfig) {
    let visual = PlanetVisual::build(&config, PLANET_SUBDIVISIONS);

    let radius = config.radius.to_engine();
    let body_radius = config.body_radius().to_engine();

    entity.insert((
        PlanetMarker,
        EntityTypeName::new(PLANET_TYPE_NAME),
        PlanetRadius(radius),
        PlanetMass(config.mass),
        PlanetInvulnerable(config.invulnerable),
        // A planet is stone, so it answers the impact table as stone. Nothing
        // here is authorable yet: an ice or metal world is a palette question
        // first, and this follows whatever that decides.
        SurfaceMaterial::new(MATERIAL_ROCK.to_string()),
        // The lock scanner sees a body in proportion to its size, same rule as
        // a rock. A planet's radius is its real size, so this needs no factor.
        LockSignature(config.lock_signature.map_or(radius, Meters::to_engine)),
        InsetZoomable,
        RigidBody::Dynamic,
        TransformInterpolation,
        // The DERIVED surface. Its `Add` is what sequences the well after the
        // body is built, exactly as it does for an asteroid.
        BodyRadius(body_radius),
    ));

    entity.with_children(|parent| {
        parent.spawn((
            Transform::from_scale(Vec3::splat(radius)),
            // A SPHERE, not a convex hull off the mesh.
            //
            // The asteroid takes a hull because a rock is genuinely lumpy and
            // a trimesh cost it 21.9 ms a step against 0.10 ms as a hull. A
            // planet is smoother still: it is a sphere to within its relief,
            // a few percent. A primitive sphere needs no vertex data and no
            // hull build at all, and the error it carries is smaller than the
            // relief the mesh already exaggerates for the silhouette.
            Collider::sphere(1.0 + config.relief_fraction()),
            PlanetRenderBody(visual),
            ConnectedTo::default(),
            ColliderDensity(1.0),
            Visibility::Inherited,
        ));
    });
}

/// Turn a planet into a gravity well once its [`BodyRadius`] lands.
///
/// The asteroid's rule, applied to a planet: strength comes from the authored
/// mass, the SOI is measured from the GEOMETRIC body radius, and the source
/// goes on rails (`RigidBody::Static`, overriding the bundle's Dynamic) so a
/// hit cannot shove a well and drag every orbit in it along.
///
/// Qualification differs in one way and it matters. A rock qualifies on its
/// nominal radius against `min_well_radius`, because that number is the
/// designation intent. A planet's radius is its real size, so an unmassed
/// planet qualifies on the same threshold but means it literally.
fn insert_planet_gravity_well(
    add: On<Add, BodyRadius>,
    mut commands: Commands,
    settings: Res<GravitySettings>,
    q_planet: Query<(&PlanetRadius, &BodyRadius, &PlanetMass), With<PlanetMarker>>,
) {
    let entity = add.entity;
    let Ok((radius, body_radius, authored)) = q_planet.get(entity) else {
        return;
    };

    let mu = match **authored {
        Some(mass) => mass,
        None if **radius >= settings.min_well_radius => settings.default_mass,
        None => return,
    };

    commands.entity(entity).insert((
        GravityWell::from_mass(mu, **body_radius, &settings),
        RigidBody::Static,
    ));
}

/// Turn the parked [`PlanetRenderBody`] into live mesh and material handles.
fn insert_planet_render(
    add: On<Add, PlanetRenderBody>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<PlanetSurfaceMaterial>>,
    q_body: Query<&PlanetRenderBody>,
) {
    let entity = add.entity;
    let Ok(body) = q_body.get(entity) else {
        error!("insert_planet_render: entity {entity:?} carries no PlanetRenderBody");
        return;
    };

    commands.entity(entity).insert((
        Mesh3d(meshes.add(body.mesh.clone())),
        MeshMaterial3d(materials.add(body.material.clone())),
    ));
}

/// The authored planet: its well, and (when rendering) its surface.
///
/// Named for the OBJECT, not the look -
/// [`PlanetSurfacePlugin`](super::planet_surface::PlanetSurfacePlugin) is the
/// material pipeline alone and stays separately addable, so an example can
/// draw a planet surface without spawning scenario objects.
#[derive(Default)]
pub struct PlanetPlugin {
    /// Whether to register the render path. False in a headless app.
    pub render: bool,
}

impl Plugin for PlanetPlugin {
    fn build(&self, app: &mut App) {
        trace!("PlanetPlugin: build");

        // Same reason the asteroid does it: the observer has to work in a
        // scenario-only app that never added the gravity layer.
        app.init_resource::<GravitySettings>();

        app.register_type::<PlanetMarker>()
            .register_type::<PlanetRadius>()
            .register_type::<PlanetMass>()
            .register_type::<PlanetInvulnerable>();

        app.add_observer(insert_planet_gravity_well);
        if self.render {
            app.add_plugins(PlanetSurfacePlugin);
            app.add_observer(insert_planet_render);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn planet(config: PlanetConfig) -> (App, Entity) {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        let entity = app.world_mut().spawn_empty().id();
        {
            let mut commands = app.world_mut().commands();
            let mut entity_commands = commands.entity(entity);
            planet_scenario_object(&mut entity_commands, config);
        }
        app.world_mut().flush();
        (app, entity)
    }

    /// The number the sim measures from is the OUTER surface, not the mean
    /// radius - the well clamp, the SOI and an orbit ring all read it.
    #[test]
    fn a_planet_publishes_its_outer_surface_as_the_body_radius() {
        let config = PlanetConfig::new(PlanetType::DustWorld, Meters(1_000.0), 7);
        let expected = config.body_radius().to_engine();
        let (app, entity) = planet(config);

        let body = app.world().get::<BodyRadius>(entity).expect("a BodyRadius");
        assert!(
            (**body - expected).abs() < 1e-3,
            "body radius {} should be the mean radius times 1 + relief ({expected})",
            **body
        );
        assert!(
            **body > app.world().get::<PlanetRadius>(entity).expect("a radius").0,
            "the outer surface must stand above the mean radius"
        );
    }

    /// The same config draws the same world on every load. Nothing on the
    /// spawn path reaches for an RNG or a clock: the authored seed is the only
    /// thing that decides which world of its type this is.
    #[test]
    fn the_same_config_draws_the_same_world_every_load() {
        let config = PlanetConfig::new(PlanetType::IceWorld, Meters(900.0), 7);
        let (first, entity) = planet(config.clone());
        let (second, other) = planet(config);

        let a = first
            .world()
            .get::<PlanetRenderBody>(child_of(&first, entity));
        let b = second
            .world()
            .get::<PlanetRenderBody>(child_of(&second, other));
        let (a, b) = (a.expect("a render body"), b.expect("a render body"));
        assert_eq!(
            a.surface.summary(),
            b.surface.summary(),
            "the same config must draw the same planet"
        );
    }

    fn child_of(app: &App, parent: Entity) -> Entity {
        *app.world()
            .get::<Children>(parent)
            .expect("a render child")
            .first()
            .expect("a render child")
    }

    /// An authored mass makes a well; the body is put on rails so a hit
    /// cannot drag its sphere of influence around.
    #[test]
    fn an_authored_mass_makes_a_well_on_rails() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_plugins(PlanetPlugin { render: false });
        let entity = app.world_mut().spawn_empty().id();
        {
            let mut commands = app.world_mut().commands();
            let mut entity_commands = commands.entity(entity);
            planet_scenario_object(
                &mut entity_commands,
                PlanetConfig::new(PlanetType::BarrenRock, Meters(1_000.0), 7).anchored(27_000.0),
            );
        }
        app.world_mut().flush();

        assert!(
            app.world().get::<GravityWell>(entity).is_some(),
            "an authored mass must raise a well"
        );
        assert!(
            matches!(
                app.world().get::<RigidBody>(entity),
                Some(RigidBody::Static)
            ),
            "a well source goes on rails"
        );
    }
}
