//! Anchor scenario object: an invisible authored point that satisfies
//! position-plus-radius contracts (the menu camera, orbit directives) without
//! spawning a body. It carries a [`GravityWell`] with an AUTHORED radius - so
//! anything reading well geometry sees a deterministic value instead of a
//! noise-mesh derivation - but no mesh, no collider, and no `BodyRadius`:
//! nothing can see it, hit it, or steer around it.

use avian3d::prelude::*;
use bevy::prelude::*;
use nova_events::prelude::*;
use nova_gameplay::prelude::*;

/// The anchor scenario object: `AnchorConfig` and `AnchorPlugin`.
pub mod prelude {
    pub use super::{
        anchor_scenario_object, AnchorBodyRadius, AnchorConfig, AnchorMarker, AnchorMass,
        AnchorPlugin,
    };
}

/// The scenario/modding RON surface for an anchor: the well geometry it
/// publishes and the optional pull. Passed to [`anchor_scenario_object`].
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct AnchorConfig {
    /// The well's published body radius, world units. Authored, so every
    /// consumer (menu camera framing, orbit rings) sees the same value on
    /// every load.
    pub body_radius: f32,
    /// Gravitational parameter (`mu`, u^3/s^2), same authoring unit as
    /// asteroid mass. `None` = an inert anchor: the well exists with zero
    /// strength, so it frames and anchors without pulling anything.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub mass: Option<f32>,
}

/// Marks an anchor scenario object root.
#[derive(Component, Clone, Debug, Default, Reflect)]
#[reflect(Component)]
pub struct AnchorMarker;

/// The authored well radius, consumed by `insert_anchor_well`.
#[derive(Component, Clone, Debug, Deref, Reflect)]
#[reflect(Component)]
pub struct AnchorBodyRadius(pub f32);

/// The authored mass (`None` = zero-strength), consumed by `insert_anchor_well`.
#[derive(Component, Clone, Debug, Deref, Reflect)]
#[reflect(Component)]
pub struct AnchorMass(pub Option<f32>);

/// Build the anchor bundle from an [`AnchorConfig`]: the marker plus the
/// authored well inputs. On rails like every well source - the force system
/// reads well positions through the physics transform, and an anchor that
/// could be shoved would drag its consumers with it.
pub fn anchor_scenario_object(config: AnchorConfig) -> impl Bundle {
    trace!("anchor_scenario_object: config {:?}", config);

    (
        AnchorMarker,
        EntityTypeName::new(ANCHOR_TYPE_NAME),
        AnchorBodyRadius(config.body_radius),
        AnchorMass(config.mass),
        RigidBody::Static,
    )
}

/// The invisible anchor scenario object: publishes a deterministic
/// [`GravityWell`] and nothing else. No render flag - there is nothing to
/// render.
pub struct AnchorPlugin;

impl Plugin for AnchorPlugin {
    fn build(&self, app: &mut App) {
        trace!("AnchorPlugin: build");

        app.register_type::<AnchorMarker>()
            .register_type::<AnchorBodyRadius>()
            .register_type::<AnchorMass>();
        app.add_observer(insert_anchor_well);
    }
}

/// Give each anchor its authored well. [`GravityWell::from_mass`] keeps the
/// same strength cap and SOI derivation as asteroid wells; a mass of `None`
/// authors `mu = 0` - a well that frames (radius, SOI = radius) but never
/// pulls.
fn insert_anchor_well(
    add: On<Add, AnchorMarker>,
    mut commands: Commands,
    settings: Res<GravitySettings>,
    q_anchor: Query<(&AnchorBodyRadius, &AnchorMass), With<AnchorMarker>>,
) {
    let entity = add.entity;
    let Ok((body_radius, mass)) = q_anchor.get(entity) else {
        return;
    };

    commands.entity(entity).insert(GravityWell::from_mass(
        mass.unwrap_or(0.0),
        **body_radius,
        &settings,
    ));
}

#[cfg(test)]
mod tests {
    use nova_ship::prelude::BodyRadius;

    use super::*;

    fn app() -> App {
        let mut app = App::new();
        app.init_resource::<GravitySettings>();
        app.add_observer(insert_anchor_well);
        app
    }

    /// The anchor's whole contract: a deterministic authored-radius well on
    /// rails, with no body around it - no mesh, no collider, no `BodyRadius`
    /// for avoidance to steer around.
    #[test]
    fn an_anchor_is_an_invisible_authored_well() {
        let mut app = app();
        let entity = app
            .world_mut()
            .spawn(anchor_scenario_object(AnchorConfig {
                body_radius: 80.0,
                mass: None,
            }))
            .id();
        app.world_mut().flush();

        let well = app
            .world()
            .get::<GravityWell>(entity)
            .expect("the anchor publishes a gravity well");
        assert_eq!(
            well.body_radius, 80.0,
            "the radius is authored, not derived"
        );
        assert_eq!(well.mu, 0.0, "no mass -> zero strength");
        assert_eq!(well.soi_radius, 80.0, "the SOI floors at the body radius");
        assert!(matches!(
            app.world().get::<RigidBody>(entity),
            Some(RigidBody::Static)
        ));
        assert!(
            app.world().get::<BodyRadius>(entity).is_none(),
            "no BodyRadius: AI avoidance must fly straight through an anchor"
        );
        assert!(
            app.world().get::<Collider>(entity).is_none(),
            "no collider: nothing can hit an anchor"
        );
    }

    /// An authored mass makes a real well through the same derivation as
    /// asteroid mass, so per-scene gravity stays one authoring vocabulary.
    #[test]
    fn an_authored_mass_makes_a_real_well() {
        let mut app = app();
        let entity = app
            .world_mut()
            .spawn(anchor_scenario_object(AnchorConfig {
                body_radius: 20.0,
                mass: Some(1200.0),
            }))
            .id();
        app.world_mut().flush();

        let well = app.world().get::<GravityWell>(entity).expect("well");
        let settings = GravitySettings::default();
        let expected = GravityWell::from_mass(1200.0, 20.0, &settings);
        assert_eq!(well.mu, expected.mu);
        assert_eq!(well.soi_radius, expected.soi_radius);
    }
}
