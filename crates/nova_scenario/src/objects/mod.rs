//! The things a scenario can place in the world - asteroids, beacons, lights,
//! spaceships, trigger areas - one module each.
//!
//! Each submodule owns its authored config, its spawn bundle and its plugin;
//! [`ScenarioObjectsPlugin`] adds them all and is the only registration point.
//!
//! Touch this module when adding a new kind of authored world object.

/// Anchor scenario object: an invisible authored point publishing a
/// deterministic gravity well (camera framing, orbit targets) with no body.
pub mod anchor;
/// Scenario trigger areas: sensor volumes that fire `OnEnter`/`OnExit` events.
pub mod area;
/// Asteroid scenario object: mobile noise-generated rocks that feel gravity wells.
pub mod asteroid;
/// The signed field behind a carvable rock, and the remesh that follows a hit.
pub mod asteroid_carve;
/// What a rock is made of, and the palette and noise knobs that makes it wear.
pub mod asteroid_kind;
/// How a rock is textured (triplanar, no UVs) and how its silhouette is shaped.
pub mod asteroid_surface;
pub mod beacon;
/// The player's fleet: hired escorts that follow the player ship.
pub mod fleet;
/// Light scenario object: the authored directional and point lights a scene
/// lights itself with.
pub mod light;
/// The hulls a station parks at its docking ports, for sale or the player's own.
pub mod parked_hulls;
/// The authored planet OBJECT: the components a spawned world carries, the
/// gravity well it becomes, and the plugin that draws it.
pub mod planet;
/// The SHAPE of a planetoid and the material that shades it: a signed height
/// field, the displaced icosphere carrying it, and the banded planet material.
pub mod planet_surface;
/// What KIND of world a planetoid is: the authored types, the biomes each may
/// draw from, and the seeded draw that turns one into the other.
pub mod planet_type;
/// The SHIP DESIGN content kind: an assembled section list authored once and
/// spawned by id.
pub mod ship_design;
/// Spaceship scenario object: player/AI ships built from a section list.
pub mod spaceship;
/// The START ROLE content kind: one New Game start, a hull and a kit.
pub mod start_role;
/// The STATION content kind and object: a parked hull with a docking port,
/// authored once and spawned by id.
pub mod station;
/// The escorts a station hires out and the buy that swaps a ship's hull with
/// a parked one.
pub mod station_hulls;

/// Every scenario object submodule's prelude plus `ScenarioObjectsPlugin`.
pub mod prelude {
    pub use super::{
        anchor::prelude::*, area::prelude::*, asteroid::prelude::*, asteroid_carve::prelude::*,
        asteroid_kind::prelude::*, asteroid_surface::prelude::*, beacon::prelude::*,
        fleet::prelude::*, light::prelude::*, parked_hulls::prelude::*, planet::prelude::*,
        planet_surface::prelude::*, planet_type::prelude::*, ship_design::prelude::*,
        spaceship::prelude::*, start_role::prelude::*, station::prelude::*,
        station_hulls::prelude::*, ScenarioObjectsPlugin,
    };
}

use bevy::prelude::*;

/// Aggregates the scenario-object plugins (asteroid, spaceship, area, beacon,
/// light) into one group. `render` is threaded to the
/// render-bearing members so headless tools can spawn objects without their
/// visuals.
/// Adds each object type's own plugin (see [`asteroid::AsteroidPlugin`],
/// [`spaceship::SpaceshipPlugin`], [`area::ScenarioAreaPlugin`],
/// [`beacon::BeaconPlugin`], [`light::LightPlugin`]) at build time.
pub struct ScenarioObjectsPlugin {
    /// Whether the render-bearing object plugins spawn their visuals (false for headless tools).
    pub render: bool,
}

impl Plugin for ScenarioObjectsPlugin {
    fn build(&self, app: &mut App) {
        trace!("ScenarioObjectsPlugin: build");

        // The merge replaces it with the loaded roles, and `SpawnStartRole`
        // reads it. It has no systems, so it has no plugin of its own.
        app.init_resource::<start_role::GameStartRoles>();
        app.add_plugins(anchor::AnchorPlugin);
        app.add_plugins(asteroid::AsteroidPlugin {
            render: self.render,
        });
        app.add_plugins(asteroid_carve::AsteroidCarvePlugin {
            render: self.render,
        });
        app.add_plugins(planet::PlanetPlugin {
            render: self.render,
        });
        app.add_plugins(spaceship::SpaceshipPlugin);
        app.add_plugins(station_hulls::StationHullsPlugin);
        app.add_plugins(parked_hulls::ParkedHullsPlugin);
        app.add_plugins(fleet::FleetPlugin);
        app.add_plugins(area::ScenarioAreaPlugin);
        app.add_plugins(beacon::BeaconPlugin {
            render: self.render,
        });
        app.add_plugins(light::LightPlugin {
            render: self.render,
        });
    }
}
