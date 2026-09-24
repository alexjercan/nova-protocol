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
        SectionKind::Docking(docking) => {
            entity.insert(preview_docking_section(docking.clone()));
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
    id: &str,
    object: &ObjectNode,
    art: &mut PreviewArt,
    sections: Option<&GameSections>,
    ships: Option<&GameShipDesigns>,
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
        //
        // THIS rock's own reach, resolved through the same seed the spawn
        // resolves - authored, or hashed from the object's id. The 3.5 floor
        // this replaced is the smallest any rock can be: a flown body reaches
        // up to 6.0 times its radius, so a belt laid flush by eye came apart on
        // the first physics step.
        ScenarioObjectKind::Asteroid(rock) => {
            let seed = rock.seed.unwrap_or_else(|| asteroid_seed_from_id(id));
            let radius = (rock.radius * rock_geometric_factor(seed))
                .to_engine()
                .max(MIN_OBJECT_RADIUS);
            let texture = rock.texture.resolve(&art.asset_server);
            let material = art
                .materials
                .add(asteroid_preview_material(&rock.kind, texture));
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
            // Empty catalogs to resolve against, so a rig with no
            // `GameShipDesigns` still draws the designs that carry their own
            // sections inline. Resolved exactly as the spawn will build it,
            // patches included: a patch can move a section, and the box has to
            // reach where the section will actually stand.
            let (no_designs, no_sections) = (GameShipDesigns::default(), GameSections::default());
            let (design, _) = resolve_ship_design(
                &spaceship.design,
                ships.unwrap_or(&no_designs),
                sections.unwrap_or(&no_sections),
            );
            let (centre, extents) = hull_bounds(&design.sections);
            // A COMPOUND of one box, because the merged bounds of a hull are
            // not centred on its node: a ship whose drive hangs off the stern
            // has more behind the origin than in front of it, and a bare
            // `Collider::cuboid` can only be centred.
            entity.insert(Collider::compound(vec![(
                centre,
                Quat::IDENTITY,
                Collider::cuboid(extents.x, extents.y, extents.z),
            )]));
            entity.with_children(|parent| {
                for section in &design.sections {
                    let mut child = parent.spawn((
                        Transform::from_translation(section.position)
                            .with_rotation(section.rotation),
                        Visibility::Inherited,
                    ));
                    insert_preview_section(&mut child, &section.config, PreviewRole::Display);
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
        // The SEED is drawn from too: it picks the silhouette, and with it how
        // far the body reaches past its authored radius.
        ScenarioObjectKind::Asteroid(_) => &["radius", "seed", "texture", "kind"],
        // Every field the surface is generated from, because the editor draws
        // the real surface: a seed change IS a different world.
        ScenarioObjectKind::Planet(_) => &["radius", "planet_type", "seed", "relief", "sea_level"],
        ScenarioObjectKind::Beacon(_) => &["radius", "color"],
        ScenarioObjectKind::SalvageCrate(_) => &["size"],
        ScenarioObjectKind::Light(_) => &["color"],
        ScenarioObjectKind::Spaceship(_) => &["hull"],
    }
}

/// The box that covers a placed hull: its centre, and its full extents.
///
/// Every section's OWN authored collider, rotated the way the hull places it
/// and merged about its position. The half-cell pad this replaced assumed a
/// 1x1x1 collider, so a 3x3x2 vector thruster or a 5x5x3 capital drive at the
/// stern was unclickable past its first cell and under-reported to
/// [`nova_gameplay::prelude::subtree_collider_aabb`] - which is what the stage
/// frames and lays out from.
///
/// An empty hull, or one whose every prototype a mod overlay dropped, falls
/// back to the unit cell centred on the node, so it is still something a click
/// can reach. The extents never go below one cell for the same reason.
fn hull_bounds(sections: &[ResolvedSection]) -> (Vec3, Vec3) {
    let mut merged: Option<(Vec3, Vec3)> = None;
    for section in sections {
        let half = section
            .config
            .base
            .collider
            .unwrap_or_default()
            .rotated_aabb_half_extents(section.rotation);
        let (low, high) = (section.position - half, section.position + half);
        merged = Some(match merged {
            Some((min, max)) => (min.min(low), max.max(high)),
            None => (low, high),
        });
    }
    let (min, max) = merged.unwrap_or((Vec3::splat(-0.5), Vec3::splat(0.5)));
    ((min + max) * 0.5, (max - min).max(Vec3::ONE))
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
            kind: KIND_ROCK.to_string(),
            destroy_sound: None,
            mass: None,
            seed: None,
            lock_signature: None,
        });

        assert!(body_is_drawn_from(
            &rock,
            &[PathStep::Field("kind".to_string())]
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

    /// A section entry standing where it is placed, turned how it is turned.
    fn placed(config: SectionConfig, position: Vec3, rotation: Quat) -> ResolvedSection {
        ResolvedSection {
            id: "section".to_string(),
            position,
            rotation,
            config,
        }
    }

    /// A hull section with no authored collider: the unit cell.
    fn unit_hull() -> SectionConfig {
        SectionConfig {
            base: BaseSectionConfig {
                id: "cell".to_string(),
                name: "cell".to_string(),
                ..default()
            },
            kind: SectionKind::Hull(HullSectionConfig::default()),
        }
    }

    /// A long section turned off axis has to EXPAND the hull box it is in. The
    /// half-cell pad this replaced assumed every section was a unit cube, so a
    /// 3x3x2 vector thruster laid across the stern was unclickable past its
    /// first cell.
    #[test]
    fn a_rotated_multi_cell_section_expands_the_hull_box() {
        let drive = SectionConfig {
            base: BaseSectionConfig {
                id: "drive".to_string(),
                name: "drive".to_string(),
                collider: Some(SectionCollider::Cuboid {
                    size: Vec3::new(3.0, 3.0, 2.0),
                }),
                ..default()
            },
            kind: SectionKind::Hull(HullSectionConfig::default()),
        };
        let stern = |rotation| vec![placed(drive.clone(), Vec3::new(0.0, 0.0, 4.0), rotation)];

        let (centre, square) = hull_bounds(&stern(Quat::IDENTITY));
        assert_eq!(
            centre,
            Vec3::new(0.0, 0.0, 4.0),
            "one section is its own box"
        );
        assert_eq!(square, Vec3::new(3.0, 3.0, 2.0), "at its authored size");

        // A quarter turn about Y swaps the section's X and Z reach.
        let (_, turned) = hull_bounds(&stern(Quat::from_rotation_y(std::f32::consts::FRAC_PI_2)));
        assert!(
            (turned.x - 2.0).abs() < 1e-4 && (turned.z - 3.0).abs() < 1e-4,
            "the turn has to reach the box (got {turned:?})"
        );
    }

    /// A hull whose box is not centred on its node still gets a collider over
    /// the whole of it: the box is placed, not assumed to be about the origin.
    #[test]
    fn an_off_centre_hull_is_bounded_where_it_actually_stands() {
        let hull = vec![
            placed(unit_hull(), Vec3::ZERO, Quat::IDENTITY),
            placed(unit_hull(), Vec3::new(0.0, 0.0, -10.0), Quat::IDENTITY),
        ];

        let (centre, extents) = hull_bounds(&hull);

        assert_eq!(centre, Vec3::new(0.0, 0.0, -5.0));
        assert_eq!(extents, Vec3::new(1.0, 1.0, 11.0));
    }

    /// An empty or unresolved hull is still something a click can reach.
    #[test]
    fn a_hull_with_nothing_resolved_keeps_a_unit_cell() {
        let (centre, extents) = hull_bounds(&[]);

        assert_eq!(centre, Vec3::ZERO);
        assert_eq!(extents, Vec3::ONE);
    }

    /// The stage draws a rock at the size the SPAWN will build it, through the
    /// seed the spawn will resolve. Drawn at the 3.5 floor, a belt laid flush
    /// by eye came apart on the first physics step.
    #[test]
    fn a_rock_without_an_authored_seed_is_drawn_at_the_reach_its_id_gives_it() {
        let id = "belt_rock_7";
        let seed = asteroid_seed_from_id(id);
        let factor = rock_geometric_factor(seed);

        assert!(
            (ASTEROID_GEOMETRIC_FACTOR_MIN..=ASTEROID_GEOMETRIC_FACTOR_MAX).contains(&factor),
            "the reach is inside the bounds the spawn promises (got {factor})"
        );
        assert!(
            factor > ASTEROID_GEOMETRIC_FACTOR_MIN,
            "and this id's rock is bigger than the floor, which is the bug (got {factor})"
        );
        // An AUTHORED seed is the one that is used, exactly as the spawn does.
        let pinned = 99;
        assert_ne!(
            rock_geometric_factor(pinned),
            factor,
            "a pinned seed is a different silhouette"
        );
    }
}
