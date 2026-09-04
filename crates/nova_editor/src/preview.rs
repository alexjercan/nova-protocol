//! The one place that knows how a [`SectionConfig`] or a
//! [`ScenarioObjectKind`] becomes preview entities, so the build ship, the
//! gallery tiles, the placement ghost and the world's objects cannot drift
//! apart.
//!
//! Preview entities render and pick but never simulate, and they are inert by
//! CONSTRUCTION: every section kind is built from its `preview_*_section` half,
//! which carries the render mesh and the config the render observers read and
//! leaves out the live state - thrust input and magnitude, turret aim and
//! trigger, torpedo fire input, the controller's `PDController`. The simulation
//! systems all demand one of those, so they match a preview against no query at
//! all. Nothing here depends on the preview root being unmarked or on the
//! scenario being dead.
//!
//! An OBJECT preview is inert the same way and by a blunter route: it is a
//! plain mesh the editor builds at the size the flown object draws at, and
//! carries none of that object's markers - no `AsteroidMarker` to be carved, no
//! `ScenarioAreaMarker` to fire `OnEnter`, no `LightMarker` to light anything.
//! A schematic body, not a second spawn path for the real one.

use avian3d::prelude::Collider;
use bevy::{
    ecs::system::{EntityCommands, SystemParam},
    prelude::*,
};
use nova_gameplay::markers::prelude::SectionMarker;
use nova_scenario::prelude::*;
use nova_ship::prelude::*;

use crate::{inspect::PathStep, node::ObjectNode};

/// The smallest a placed object's body is drawn at, so an anchor with a hairline
/// radius and a light (which has no body at all) are still things you can see
/// and click.
const MIN_OBJECT_RADIUS: f32 = 1.5;

/// The asset stores an object preview builds its body out of.
///
/// A [`SystemParam`] because every caller is a system and all three come as a
/// set: an object's body is a mesh, a material and - for a textured rock - a
/// path to resolve.
#[derive(SystemParam)]
pub(crate) struct PreviewArt<'w> {
    meshes: ResMut<'w, Assets<Mesh>>,
    materials: ResMut<'w, Assets<StandardMaterial>>,
    planet_materials: ResMut<'w, Assets<PlanetSurfaceMaterial>>,
    asset_server: Res<'w, AssetServer>,
}

/// What a preview entity IS to the rest of the editor.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum PreviewRole {
    /// A section of the ship being built: picked, counted, placed against and
    /// handed to the scenario.
    Section,
    /// Scenery that only has to render - a gallery tile, the placement ghost.
    /// It must NOT read as a section: a display copy in a section query would
    /// be one more part on a ship nobody built, and one more collider in the
    /// pointer's way.
    Display,
}

/// Turn `entity` into a preview of `section`: the shared preview bundle plus
/// the kind-specific one that renders it.
///
/// No input bindings. A section's binds are DOCUMENT data
/// ([`SectionNode::binds`](crate::node::SectionNode::binds)), and a preview is a
/// picture of a section: a second copy of the binds out here would have to be
/// kept in step across every despawn of the view that held it.
pub(crate) fn insert_preview_section(
    entity: &mut EntityCommands,
    section: &SectionConfig,
    role: PreviewRole,
) {
    entity.insert(preview_section(section.base.clone()));
    match &section.kind {
        SectionKind::Hull(hull) => {
            entity.insert(preview_hull_section(hull.clone()));
        }
        SectionKind::Controller(controller) => {
            entity.insert(preview_controller_section(controller.clone()));
        }
        SectionKind::Thruster(thruster) => {
            entity.insert(preview_thruster_section(thruster.clone()));
        }
        SectionKind::Turret(turret) => {
            entity.insert(preview_turret_section(turret.clone()));
        }
        SectionKind::Torpedo(torpedo) => {
            entity.insert(preview_torpedo_section(torpedo.clone()));
        }
        SectionKind::Railgun(railgun) => {
            entity.insert(preview_railgun_section(railgun.clone()));
        }
    }
    if role == PreviewRole::Display {
        // Dropped rather than never inserted: the preview bundle is one shared
        // recipe, and a display copy is that recipe minus its identity.
        entity.remove::<(SectionMarker, Collider)>();
    }
}

/// The stage's stand-in for a kind's surface: the kind's palette and its
/// specular, over the authored texture.
///
/// Not the shipped shader. A flown rock wears a triplanar
/// [`AsteroidSurfaceMaterial`] over a noise-carved mesh, and the stage draws a
/// smooth sphere - but the PALETTE is what tells an ice rock from a carbon one
/// at a glance, and that is the question the inspector's kind picker asks. A
/// creator who picks `ice` has to see the rock go pale, or the control is a
/// text field with extra steps.
///
/// [`AsteroidKindLook::kind_mix`] is how far the palette replaces the texture's
/// own colour, so it is the blend here too: `plain` mixes nothing in and stays
/// the texture, exactly as it does in the shader.
///
/// A kind nobody ships paints MAGENTA, unlit. The lint refuses such a document
/// and the loader refuses to render it, so a rock that reaches here with an
/// unknown kind is a file hand-edited past both - and the wrong colour is the
/// report.
fn asteroid_preview_material(kind: &str, texture: Handle<Image>) -> StandardMaterial {
    let Some(look) = asteroid_kind_look(kind) else {
        return StandardMaterial {
            base_color: Color::srgb(1.0, 0.0, 1.0),
            unlit: true,
            ..default()
        };
    };
    StandardMaterial {
        base_color_texture: Some(texture),
        base_color: Color::from(
            LinearRgba::WHITE.mix(&look.shade.mix(&look.tint, 0.5), look.kind_mix),
        ),
        perceptual_roughness: (look.roughness_low + look.roughness_high) * 0.5,
        metallic: look.metallic,
        ..default()
    }
}

/// Turn `entity` into a preview of `object`: a schematic body at the size the
/// flown object draws at, and a collider so the pointer can reach it.
///
/// SCHEMATIC on purpose. The real bodies are built by the scenario's own spawn
/// path - a noise-meshed rock, a beacon that is its own trigger area, a light
/// that lights - and none of that belongs on a stage that is not running a
/// scenario. What the editor needs is a thing at the right place, at the right
/// SIZE, that answers a click; what it must not have is an object half-alive.
pub(crate) fn insert_preview_object(
    entity: &mut EntityCommands,
    object: &ObjectNode,
    art: &mut PreviewArt,
    sections: Option<&GameSections>,
    ships: Option<&GameShips>,
) {
    match &object.kind {
        // Translucent, because an anchor has no body at all: what is drawn is
        // the radius it publishes, and a solid ball there would read as a rock.
        ScenarioObjectKind::Anchor(anchor) => {
            let radius = anchor.body_radius.to_engine().max(MIN_OBJECT_RADIUS);
            let material = art.materials.add(StandardMaterial {
                base_color: Color::srgba(0.45, 0.60, 0.75, 0.20),
                alpha_mode: AlphaMode::Blend,
                unlit: true,
                ..default()
            });
            sphere_body(entity, art, radius, material);
        }
        // The DRAWN radius, not the nominal one: a rock's shape function is
        // based several times out from the unit sphere, so a planetoid authored
        // at 24 is a ball a hundred units across and an editor that drew 24
        // would put the whole layout in the wrong place by eye.
        ScenarioObjectKind::Asteroid(rock) => {
            let radius = (rock.radius * ASTEROID_GEOMETRIC_FACTOR_MIN)
                .to_engine()
                .max(MIN_OBJECT_RADIUS);
            let texture = rock.texture.resolve(&art.asset_server);
            let material = art
                .materials
                .add(asteroid_preview_material(&rock.material, texture));
            sphere_body(entity, art, radius, material);
        }
        // The one preview that is NOT schematic, because for a planet the
        // look IS the authored value. A creator picks a type and a seed and
        // has no other way to see what they picked; a flat swatch would show
        // the type and hide the seed completely, and two seeds of one type are
        // different worlds. So the stage builds the real surface, just coarser
        // (see PLANET_EDITOR_SUBDIVISIONS).
        //
        // The BODY radius, like the rock above and for the same reason - but
        // the factor is honest here. A planet's mesh stands `1 + relief` off
        // its authored radius, a few percent, so the stage draws very nearly
        // the number the inspector shows.
        ScenarioObjectKind::Planet(planet) => {
            let radius = planet.body_radius().to_engine().max(MIN_OBJECT_RADIUS);
            let visual = PlanetVisual::build(planet, PLANET_EDITOR_SUBDIVISIONS);
            entity.insert((
                Transform::from_scale(Vec3::splat(radius)),
                Mesh3d(art.meshes.add(visual.mesh)),
                MeshMaterial3d(art.planet_materials.add(visual.material)),
                Collider::sphere(radius),
            ));
        }
        ScenarioObjectKind::Beacon(beacon) => {
            let radius = beacon.radius.to_engine().max(MIN_OBJECT_RADIUS);
            let material = art.materials.add(StandardMaterial {
                base_color: beacon.color,
                emissive: beacon.color.to_linear() * 4.0,
                ..default()
            });
            sphere_body(entity, art, radius, material);
        }
        ScenarioObjectKind::SalvageCrate(salvage) => {
            let size = salvage.size.to_engine().max(1.0);
            let material = art.materials.add(StandardMaterial {
                base_color: Color::srgb(0.85, 0.65, 0.25),
                perceptual_roughness: 0.7,
                ..default()
            });
            entity.insert((
                Mesh3d(art.meshes.add(Cuboid::from_length(size))),
                MeshMaterial3d(material),
                Collider::cuboid(size, size, size),
            ));
        }
        // A light is invisible where it stands, so the editor gives it a bulb:
        // a small glowing marker in the light's own colour, big enough to grab.
        ScenarioObjectKind::Light(light) => {
            let colour = light_colour(light);
            let material = art.materials.add(StandardMaterial {
                base_color: colour,
                emissive: colour.to_linear() * 6.0,
                ..default()
            });
            sphere_body(entity, art, MIN_OBJECT_RADIUS, material);
        }
        // A hull the editor did not design: drawn out of the same preview
        // sections a built ship is, so a picket on the range and a picket on the
        // build deck are the same picture. ONE collider over the whole hull,
        // not one per section: the object is edited as a unit, and a hit has to
        // land on a view whose parent is the node.
        ScenarioObjectKind::Spaceship(spaceship) => {
            // An empty catalog to resolve against, so a rig with no `GameShips`
            // still draws the hulls that carry their own sections inline.
            let empty = GameShips::default();
            let placed = spaceship
                .hull
                .resolve(ships.unwrap_or(&empty))
                .map(|hull| hull.sections.as_slice())
                .unwrap_or_default();
            let extents = hull_extents(placed);
            entity.insert(Collider::cuboid(extents.x, extents.y, extents.z));
            let placed: Vec<SpaceshipSectionConfig> = placed.to_vec();
            entity.with_children(|parent| {
                for section in &placed {
                    let Some(config) = resolve_section(&section.source, sections) else {
                        continue;
                    };
                    let mut child = parent.spawn((
                        Transform::from_translation(section.position)
                            .with_rotation(section.rotation),
                        Visibility::Inherited,
                    ));
                    insert_preview_section(&mut child, config, PreviewRole::Display);
                }
            });
        }
    }
}

/// The mesh, material and collider of a round object body.
fn sphere_body(
    entity: &mut EntityCommands,
    art: &mut PreviewArt,
    radius: f32,
    material: Handle<StandardMaterial>,
) {
    entity.insert((
        Mesh3d(art.meshes.add(Sphere::new(radius))),
        MeshMaterial3d(material),
        Collider::sphere(radius),
    ));
}

/// The colour a light marker glows in.
fn light_colour(light: &LightConfig) -> Color {
    match light {
        LightConfig::Directional { color, .. } | LightConfig::Point { color, .. } => *color,
    }
}

/// Whether `path` names something the object's BODY is drawn from.
///
/// An object's config holds plenty the body does not draw - a rock's mass and
/// seed, a beacon's dwell, a ship's allegiance - and dropping the body is a
/// fresh mesh, a fresh material and a fresh collider. A held scrub asks once a
/// frame, so the answer has to be no wherever it can be.
pub(crate) fn body_is_drawn_from(kind: &ScenarioObjectKind, path: &[PathStep]) -> bool {
    match path.first() {
        // The whole config was written: a light that became a point light.
        None => true,
        Some(PathStep::Field(name)) => drawn_fields(kind).contains(&name.as_str()),
        Some(PathStep::Slot(_) | PathStep::Item(_)) => true,
    }
}

/// The config fields [`insert_preview_object`] reads, per kind.
///
/// Here rather than anywhere else because the two have to agree: a field the
/// builder starts reading and this does not name is a body that stops following
/// its config.
fn drawn_fields(kind: &ScenarioObjectKind) -> &'static [&'static str] {
    match kind {
        ScenarioObjectKind::Anchor(_) => &["body_radius"],
        ScenarioObjectKind::Asteroid(_) => &["radius", "texture", "material"],
        // Every field the surface is generated from, because the editor draws
        // the real surface: a seed change IS a different world.
        ScenarioObjectKind::Planet(_) => &["radius", "planet_type", "seed", "relief", "sea_level"],
        ScenarioObjectKind::Beacon(_) => &["radius", "color"],
        ScenarioObjectKind::SalvageCrate(_) => &["size"],
        ScenarioObjectKind::Light(_) => &["color"],
        ScenarioObjectKind::Spaceship(_) => &["hull"],
    }
}

/// The section config a spawned hull's section names: inline, or a catalog
/// prototype. `None` when a mod overlay dropped the prototype.
fn resolve_section<'a>(
    source: &'a SectionSource,
    sections: Option<&'a GameSections>,
) -> Option<&'a SectionConfig> {
    match source {
        SectionSource::Inline(config) => Some(config),
        SectionSource::Prototype(id) => sections?.get_section(id),
    }
}

/// Full extents of the box that covers a placed hull, from the section poses
/// alone: every catalog section is a unit cell, so half a cell past the
/// outermost one covers it. Never smaller than one cell, so an empty hull is
/// still something a click can reach.
fn hull_extents(sections: &[SpaceshipSectionConfig]) -> Vec3 {
    let reach = sections.iter().fold(Vec3::ZERO, |reach, section| {
        reach.max(section.position.abs())
    });
    (reach + Vec3::splat(0.5)) * 2.0
}

#[cfg(test)]
mod tests {
    use bevy::ecs::system::RunSystemOnce;
    use nova_events::units::prelude::Meters;
    use nova_gameplay::prelude::{
        ControllerSectionMarker, SectionClass, ThrusterSectionMarker, TorpedoSectionMarker,
        TurretSectionMarker,
    };

    use super::*;

    /// The point of the kind picker: pick `ice` and the rock on the stage goes
    /// pale. Two kinds that painted the same sphere would make the control a
    /// text field with extra steps.
    #[test]
    fn every_kind_paints_the_stage_a_different_body() {
        let painted: Vec<StandardMaterial> = ASTEROID_KINDS
            .iter()
            .map(|kind| asteroid_preview_material(kind, Handle::default()))
            .collect();

        for (index, one) in painted.iter().enumerate() {
            for other in &painted[index + 1..] {
                assert!(
                    one.base_color != other.base_color
                        || one.perceptual_roughness != other.perceptual_roughness
                        || one.metallic != other.metallic,
                    "two kinds paint the same rock"
                );
            }
        }
    }

    /// `plain` is the absence of a look, in the editor exactly as in the
    /// shader: the texture, untouched.
    #[test]
    fn the_plain_kind_leaves_the_texture_alone() {
        let plain = asteroid_preview_material(KIND_PLAIN, Handle::default());

        assert_eq!(plain.base_color, Color::WHITE);
    }

    /// The lint refuses such a document and the loader refuses to render it, so
    /// a rock that reaches the stage with an unknown kind was hand-edited past
    /// both. It has to LOOK wrong.
    #[test]
    fn a_kind_the_game_does_not_ship_paints_a_refusal() {
        let unknown = asteroid_preview_material("obsidian", Handle::default());

        assert_eq!(unknown.base_color, Color::srgb(1.0, 0.0, 1.0));
        assert!(unknown.base_color_texture.is_none());
    }

    /// The body is rebuilt from the fields the builder READS, and the kind is
    /// now one of them: a kind changed in the inspector that left the old rock
    /// standing would read as the picker doing nothing.
    #[test]
    fn a_changed_kind_makes_the_rock_stale() {
        let rock = ScenarioObjectKind::Asteroid(AsteroidConfig {
            radius: Meters(30.0),
            texture: default(),
            material: KIND_ROCK.to_string(),
            destroy_sound: None,
            mass: None,
            invulnerable: false,
            seed: None,
            lock_signature: None,
        });

        assert!(body_is_drawn_from(
            &rock,
            &[PathStep::Field("material".to_string())]
        ));
    }

    fn spawn_preview(world: &mut World, kind: SectionKind) -> Entity {
        let section = SectionConfig {
            base: BaseSectionConfig {
                id: "part".to_string(),
                name: "part".to_string(),
                ..default()
            },
            kind,
        };
        world
            .run_system_once(move |mut commands: Commands| {
                let mut entity = commands.spawn_empty();
                insert_preview_section(&mut entity, &section, PreviewRole::Section);
                entity.id()
            })
            .expect("the preview spawner runs")
    }

    /// A preview section is inert because of WHAT IT IS, not because of where it
    /// is parented or because no scenario is live. Every kind gets the render
    /// half of its bundle and none of the live state the simulation keys on, so
    /// the thrust, aim, fire and steering paths match a preview against no query
    /// at all.
    ///
    /// Before the split the editor inserted the full live bundle and stayed
    /// quiet only because the preview root is not a `SpaceshipRootMarker` and
    /// the ship system sets are gated on scenario-liveness. Either gate moving
    /// would have woken a build-screen ship up.
    #[test]
    fn preview_sections_carry_the_render_half_and_no_live_state() {
        let mut world = World::new();

        let hull = spawn_preview(&mut world, SectionKind::Hull(HullSectionConfig::default()));
        assert!(world.get::<HullSectionMarker>(hull).is_some());
        assert_eq!(world.get::<SectionClass>(hull), Some(&SectionClass::Hull));

        let controller = spawn_preview(
            &mut world,
            SectionKind::Controller(ControllerSectionConfig::default()),
        );
        assert!(world.get::<ControllerSectionMarker>(controller).is_some());
        assert_eq!(
            world.get::<SectionClass>(controller),
            Some(&SectionClass::Controller)
        );
        assert!(
            world.get::<PDController>(controller).is_none(),
            "a preview controller must never try to torque a root"
        );

        let thruster = spawn_preview(
            &mut world,
            SectionKind::Thruster(ThrusterSectionConfig::default()),
        );
        assert!(world.get::<ThrusterSectionMarker>(thruster).is_some());
        assert_eq!(
            world.get::<SectionClass>(thruster),
            Some(&SectionClass::Thruster)
        );
        assert!(
            world.get::<ThrusterSectionInput>(thruster).is_none(),
            "a preview thruster must not be drivable"
        );
        assert!(
            world.get::<ThrusterSectionMagnitude>(thruster).is_none(),
            "a preview thruster must not be able to push a hull"
        );

        let turret = spawn_preview(
            &mut world,
            SectionKind::Turret(TurretSectionConfig::default()),
        );
        assert!(world.get::<TurretSectionMarker>(turret).is_some());
        assert_eq!(
            world.get::<SectionClass>(turret),
            Some(&SectionClass::Turret)
        );
        assert!(
            world.get::<TurretSectionInput>(turret).is_none(),
            "a preview turret must not have a trigger"
        );
        assert!(
            world.get::<TurretSectionAimPoint>(turret).is_none(),
            "a preview turret must not aim"
        );
        assert!(
            world.get::<LoadedBullet>(turret).is_none(),
            "a preview turret must not be loaded"
        );

        let torpedo = spawn_preview(
            &mut world,
            SectionKind::Torpedo(TorpedoSectionConfig::default()),
        );
        assert!(world.get::<TorpedoSectionMarker>(torpedo).is_some());
        assert_eq!(
            world.get::<SectionClass>(torpedo),
            Some(&SectionClass::Torpedo)
        );
        assert!(
            world.get::<TorpedoSectionInput>(torpedo).is_none(),
            "a preview torpedo bay must not be able to fire"
        );
    }

    /// Delivery guard for the test above: the LIVE bundles still carry the state
    /// the split moved out of the preview half, so those assertions prove the
    /// split rather than a component nothing carries.
    #[test]
    fn live_sections_still_carry_the_state_the_preview_half_drops() {
        let mut world = World::new();

        let thruster = world
            .spawn(thruster_section(ThrusterSectionConfig::default()))
            .id();
        assert!(world.get::<ThrusterSectionInput>(thruster).is_some());
        assert!(world.get::<ThrusterSectionMagnitude>(thruster).is_some());

        let turret = world
            .spawn(turret_section(TurretSectionConfig::default()))
            .id();
        assert!(world.get::<TurretSectionInput>(turret).is_some());
        assert!(world.get::<TurretSectionAimPoint>(turret).is_some());
        assert!(world.get::<LoadedBullet>(turret).is_some());

        let torpedo = world
            .spawn(torpedo_section(TorpedoSectionConfig::default()))
            .id();
        assert!(world.get::<TorpedoSectionInput>(torpedo).is_some());

        let controller = world
            .spawn(controller_section(ControllerSectionConfig::default()))
            .id();
        assert!(world.get::<PDController>(controller).is_some());
    }
}
