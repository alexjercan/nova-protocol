//! The stand-in bodies a section wears when its prototype authors no render
//! mesh: four meshes and four materials, built once and shared by every
//! section that falls back to them.
//!
//! Change this module when a section kind grows a placeholder, or when one of
//! them gains authored art and its entry here goes unused.

use bevy::prelude::*;

/// The shared placeholder mesh and material set.
pub mod prelude {
    pub use super::PlaceholderArt;
}

/// The unit cube a hull, a torpedo bay and a controller body stand in as.
const BODY_CUBE: f32 = 1.0;

/// The controller porthole's radius and depth.
const WINDOW_CYLINDER: (f32, f32) = (0.2, 0.1);

/// The thruster barrel's radius and length.
const THRUSTER_BARREL: (f32, f32) = (0.4, 0.4);

/// The thruster nozzle's base radius and length.
const THRUSTER_NOZZLE: (f32, f32) = (0.5, 0.5);

/// The glow of the placeholder nozzle. The stand-in already called itself a
/// hot red and then gave the cone albedo only, so the side of it facing away
/// from the key light sat in shadow reading as painted plastic.
///
/// Held to the base colour's own ratio and only just over 1.0. Brighter (2.5
/// was shot beside this) tips the cone to a washed salmon under the
/// tonemapper - hotter, but no longer RED-hot, and loud beside the exhaust
/// that is the actual light source when the drive burns.
const NOZZLE_EMISSIVE: LinearRgba = LinearRgba::new(1.2, 0.28, 0.1, 1.0);

/// The turret base plate's radius and thickness. A wide flat disc, which is
/// what an unmeshed mount stood on before the joints were split out.
const TURRET_PLATE: (f32, f32) = (0.5, 0.1);

/// The meshes and materials of every un-authored section body in the game.
///
/// ONE set for the whole app. Each fallback branch used to call `meshes.add`
/// and `materials.add` per SECTION ENTITY, so a hull of sixteen procedural
/// drives introduced thirty-two meshes and thirty-two materials that were all
/// one of four values. A distinct asset is extracted, prepared, bound and
/// written every frame however many entities share its value, which is what
/// made a placeholder cost more than the authored art it stands in for.
///
/// Built from the world rather than assembled by a plugin, the same way
/// `turret_section`'s projectile art is: the values are constants, so there is
/// nothing for a caller to configure and nothing to get out of step.
#[derive(Resource)]
pub struct PlaceholderArt {
    /// A hull, a torpedo bay and a controller body.
    pub body: Handle<Mesh>,
    /// The controller's porthole.
    pub window: Handle<Mesh>,
    /// The thruster's barrel.
    pub barrel: Handle<Mesh>,
    /// The thruster's nozzle.
    pub nozzle: Handle<Mesh>,
    /// The base plate under an unmeshed turret joint.
    pub turret_plate: Handle<Mesh>,
    /// Bare structure, which is what a hull and a torpedo bay wear.
    pub structure_material: Handle<StandardMaterial>,
    /// The controller body's blue.
    pub controller_material: Handle<StandardMaterial>,
    /// The porthole's white.
    pub window_material: Handle<StandardMaterial>,
    /// The nozzle's hot red, emissive so it reads hot unlit.
    pub nozzle_material: Handle<StandardMaterial>,
    /// The turret plate's dark grey.
    pub turret_plate_material: Handle<StandardMaterial>,
}

impl FromWorld for PlaceholderArt {
    fn from_world(world: &mut World) -> Self {
        let (body, window, barrel, nozzle, turret_plate) = {
            let mut meshes = world.resource_mut::<Assets<Mesh>>();
            (
                meshes.add(Cuboid::new(BODY_CUBE, BODY_CUBE, BODY_CUBE)),
                meshes.add(Cylinder::new(WINDOW_CYLINDER.0, WINDOW_CYLINDER.1)),
                meshes.add(Cylinder::new(THRUSTER_BARREL.0, THRUSTER_BARREL.1)),
                meshes.add(Cone::new(THRUSTER_NOZZLE.0, THRUSTER_NOZZLE.1)),
                meshes.add(Cylinder::new(TURRET_PLATE.0, TURRET_PLATE.1)),
            )
        };
        let mut materials = world.resource_mut::<Assets<StandardMaterial>>();
        Self {
            body,
            window,
            barrel,
            nozzle,
            turret_plate,
            structure_material: materials.add(Color::srgb(0.8, 0.8, 0.8)),
            controller_material: materials.add(Color::srgb(0.2, 0.7, 0.9)),
            window_material: materials.add(Color::srgb(0.9, 0.9, 1.0)),
            nozzle_material: materials.add(StandardMaterial {
                base_color: Color::srgb(0.9, 0.3, 0.2),
                emissive: NOZZLE_EMISSIVE,
                ..default()
            }),
            turret_plate_material: materials.add(Color::srgb(0.25, 0.25, 0.25)),
        }
    }
}

#[cfg(test)]
mod tests {
    use bevy::asset::AssetPlugin;

    use super::*;

    /// The whole point: the set is five meshes and five materials however many
    /// sections wear it, and asking twice does not build it twice.
    #[test]
    fn the_placeholder_set_is_built_once() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.init_asset::<Mesh>();
        app.init_asset::<StandardMaterial>();

        app.init_resource::<PlaceholderArt>();
        app.init_resource::<PlaceholderArt>();

        assert_eq!(app.world().resource::<Assets<Mesh>>().len(), 5);
        assert_eq!(app.world().resource::<Assets<StandardMaterial>>().len(), 5);
    }
}
