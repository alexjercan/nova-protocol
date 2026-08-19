//! CRACKS: what damage does to a section's surface.
//!
//! One damage effect among several, and the replacement for the oldest of them.
//! It reads [`DamageLevel`] - the share of a section's own health that is gone -
//! and drives a fracture pattern on that section's material: an untouched
//! section is exactly what the artist painted, a battered one is veined with
//! dark lines, a critical one glows through them, and a dead one reads burnt.
//!
//! # Why the tint had to go
//!
//! SCORCH said "this is damaged" by reddening and darkening the whole body. That
//! is information rather than a picture of anything: a hull at 60% looked like a
//! hull painted red, and it fought every authored paint scheme it was laid over.
//! It also disagreed with the geometry - a section that is visibly bitten into
//! wants a surface that is FAILING, not one that has changed colour.
//!
//! Cracks say the same thing by showing the material coming apart, they say it
//! WHERE it comes apart rather than everywhere at once, and being dark lines
//! rather than a hue they read against any paint. The burnt endpoint the tint
//! ended on is kept - it is what makes a dead section read as wreckage.
//!
//! # Per-section material clones
//!
//! Sections render via gltf `WorldAssetRoot` scenes, and a gltf scene's
//! materials are shared handles across every instance of the same mesh - so
//! writing a damage level into a material in place would crack every section
//! that shares that mesh at once. Each rendered mesh therefore gets a private
//! [`SectionCracksMaterial`] built from its own pristine `StandardMaterial`,
//! which this module owns and writes to.
//!
//! The cracked material is also what a dead section wears as it tumbles away:
//! destruction detaches the art rather than re-drawing it, so the last damage
//! level written here is the one the wreck leaves with.
//!
//! # Timing
//!
//! Capture keys on `Added<MeshMaterial3d<StandardMaterial>>`, which fires the
//! frame a mesh appears - whether it is a synchronous cuboid or an
//! asynchronously instantiated gltf node - so it does not depend on any
//! scene-ready signal. The handle can exist before its asset does, so the clone
//! is retried until the asset arrives rather than dropped.

use bevy::{
    pbr::{ExtendedMaterial, MaterialExtension},
    prelude::*,
    render::render_resource::AsBindGroup,
    shader::ShaderRef,
};
use nova_gameplay::prelude::{DamageLevel, SectionMarker};
#[cfg(test)]
use nova_gameplay::prelude::{DamageLevelPlugin, Health, SectionInactiveMarker};

#[cfg(test)]
use crate::sections::damage_effects::prelude::{DamageEffects, DamageEffectsPlugin};
use crate::sections::fixture::prelude::SectionFixture;

/// `DamageCracks`, `SectionCracks`, the cracks material and its plugin.
pub mod prelude {
    pub use super::{
        DamageCracks, SectionCracks, SectionCracksMaterial, SectionCracksMaterialExt,
        SectionCracksPlugin, SECTION_CRACK_SCALE,
    };
}

/// Fracture lines per unit of a section's own local space.
///
/// A section is one to three units across, so this puts a handful of major
/// fractures on one - enough to read as a broken plate rather than as a texture,
/// and few enough that the finer octaves have room to be seen.
pub const SECTION_CRACK_SCALE: f32 = 2.5;

/// The section material: a standard PBR material with fractures cut into it.
pub type SectionCracksMaterial = ExtendedMaterial<StandardMaterial, SectionCracksMaterialExt>;

/// The cracks extension's own bindings.
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone, Default)]
pub struct SectionCracksMaterialExt {
    /// How far gone the section is: 0 pristine, 1 dead.
    #[uniform(100)]
    pub damage: f32,
    /// Fracture lines per unit of the body's own local space.
    #[uniform(100)]
    pub scale: f32,
    #[cfg(target_arch = "wasm32")]
    #[uniform(100)]
    _webgl2_padding_16b1: u32,
    #[cfg(target_arch = "wasm32")]
    #[uniform(100)]
    _webgl2_padding_16b2: u32,
}

impl SectionCracksMaterialExt {
    /// A pristine section's extension.
    #[cfg_attr(
        not(target_arch = "wasm32"),
        expect(
            clippy::needless_update,
            reason = "the webgl2 padding fields exist only on wasm32, and there this update is what fills them"
        )
    )]
    pub fn new() -> Self {
        Self {
            damage: 0.0,
            scale: SECTION_CRACK_SCALE,
            ..default()
        }
    }
}

impl MaterialExtension for SectionCracksMaterialExt {
    fn fragment_shader() -> ShaderRef {
        "shaders/section_cracks.wgsl".into()
    }
}

/// Makes a section's surface crack, glow through the cracks and finally burn out
/// as its damage level rises.
///
/// Carried by the SECTION rather than by the meshes drawing it: one section owns
/// one health pool and its whole rendered body cracks together. Fitted from the
/// authored [`DamageEffects`](super::damage_effects::DamageEffects), whose
/// default carries it.
#[derive(Component, Clone, Copy, Debug, Default, Reflect)]
#[reflect(Component)]
pub struct DamageCracks;

/// Records a rendered mesh whose surface is cracked by a section's health.
#[derive(Component, Clone, Debug)]
pub struct SectionCracks {
    /// The section entity whose [`DamageLevel`] drives this mesh.
    pub section: Entity,
    /// The private material this component owns and writes to each frame.
    pub material: Handle<SectionCracksMaterial>,
}

/// A section mesh awaiting material capture.
///
/// Its `StandardMaterial` handle may exist before the asset itself resolves
/// (async gltf load), so marking is decoupled from capture: [`mark_section_meshes`]
/// tags the mesh once (doing the `ChildOf` walk), and [`resolve_pending_cracks`]
/// retries the clone every frame until the asset is available - self-re-arming,
/// so a not-yet-loaded material can never silently drop the mesh out of grading.
#[derive(Component, Clone, Copy, Debug)]
struct PendingSectionCracks {
    section: Entity,
}

/// Cracks section surfaces by their own integrity. Registered by the section
/// plugin only when rendering is enabled.
#[derive(Default, Clone, Debug)]
pub struct SectionCracksPlugin;

impl Plugin for SectionCracksPlugin {
    fn build(&self, app: &mut App) {
        debug!("SectionCracksPlugin: build");

        app.register_type::<DamageCracks>();
        app.add_plugins(MaterialPlugin::<SectionCracksMaterial>::default());
        app.add_systems(
            Update,
            (
                mark_section_meshes,
                resolve_pending_cracks,
                grade_section_cracks,
            )
                .chain(),
        );
    }
}

/// Walk up the `ChildOf` chain from `entity` to the nearest ancestor that is a
/// section WEARING CRACKS, returning that section entity. `None` if the walk
/// reaches a [`SectionFixture`], or leaves the tree without passing through such
/// a section.
///
/// A section that does not wear cracks ends the walk the same way a fixture
/// does. It must not fall through to the section ABOVE it: cracking a turret's
/// meshes by the hull it is bolted to would report the wrong pool, which is
/// worse than reporting nothing.
fn owning_section(
    entity: Entity,
    q_child_of: &Query<&ChildOf>,
    q_is_section: &Query<(), With<SectionMarker>>,
    q_is_fixture: &Query<(), With<SectionFixture>>,
    q_cracks: &Query<(), With<DamageCracks>>,
) -> Option<Entity> {
    let mut current = entity;
    loop {
        // A fixture ENDS the walk, and is not graded by what it hangs on. Its
        // damage read is that it comes off, so it must not crack on the way
        // there - and grading it by the section behind it would be worse than
        // nothing: fresh cladding over a dying hull would read shattered.
        if q_is_fixture.get(current).is_ok() {
            return None;
        }
        if q_is_section.get(current).is_ok() {
            return q_cracks.get(current).is_ok().then_some(current);
        }
        current = q_child_of.get(current).ok()?.0;
    }
}

/// Tag every freshly-spawned section mesh for capture. The `ChildOf` walk
/// happens here, once per mesh; the material clone is deferred to
/// [`resolve_pending_cracks`] so a not-yet-loaded asset does not drop the mesh.
#[expect(
    clippy::type_complexity,
    reason = "the Added filter carries its own Without guards"
)]
fn mark_section_meshes(
    mut commands: Commands,
    q_new: Query<
        Entity,
        (
            Added<MeshMaterial3d<StandardMaterial>>,
            Without<SectionCracks>,
            Without<PendingSectionCracks>,
        ),
    >,
    q_child_of: Query<&ChildOf>,
    q_is_section: Query<(), With<SectionMarker>>,
    q_is_fixture: Query<(), With<SectionFixture>>,
    q_cracks: Query<(), With<DamageCracks>>,
) {
    for entity in &q_new {
        let Some(section) =
            owning_section(entity, &q_child_of, &q_is_section, &q_is_fixture, &q_cracks)
        else {
            continue;
        };

        // `try_insert`, not `insert`: a section mesh can be chain-destroyed the
        // same frame it gains its material (a ship exploding), despawning this
        // entity before the buffer applies - the insert must be a no-op there,
        // not a panic.
        commands
            .entity(entity)
            .try_insert(PendingSectionCracks { section });
    }
}

/// Build each pending mesh's private cracks material from its pristine standard
/// one, once that asset is available. Retries until it loads; this query is
/// normally empty.
fn resolve_pending_cracks(
    mut commands: Commands,
    standard: Res<Assets<StandardMaterial>>,
    mut cracked: ResMut<Assets<SectionCracksMaterial>>,
    q_pending: Query<(
        Entity,
        &MeshMaterial3d<StandardMaterial>,
        &PendingSectionCracks,
    )>,
) {
    for (entity, material, pending) in &q_pending {
        let Some(pristine) = standard.get(&material.0).cloned() else {
            // Asset not loaded yet; keep the pending marker and retry next frame.
            continue;
        };

        let handle = cracked.add(SectionCracksMaterial {
            base: pristine,
            extension: SectionCracksMaterialExt::new(),
        });

        // NOTE: same despawn race as `mark_section_meshes`.
        commands
            .entity(entity)
            .try_insert((
                MeshMaterial3d(handle.clone()),
                SectionCracks {
                    section: pending.section,
                    material: handle,
                },
            ))
            .try_remove::<MeshMaterial3d<StandardMaterial>>()
            .try_remove::<PendingSectionCracks>();
    }
}

/// Write every captured mesh's section damage into its material.
///
/// Written only when it differs from what the material already holds, so an
/// idle ship does not re-flag its materials as changed every frame.
fn grade_section_cracks(
    mut materials: ResMut<Assets<SectionCracksMaterial>>,
    q_cracks: Query<&SectionCracks>,
    q_level: Query<&DamageLevel, With<SectionMarker>>,
) {
    for cracks in &q_cracks {
        // The SAME level the carve reads, so a section's surface and its shape
        // cannot disagree about how far gone it is.
        let damage = q_level.get(cracks.section).map_or(0.0, |level| level.0);

        // Read first; only take a mutable (change-flagging) borrow on a real
        // change.
        let Some(current) = materials.get(&cracks.material) else {
            continue;
        };
        if current.extension.damage == damage {
            continue;
        }
        let Some(mut material) = materials.get_mut(&cracks.material) else {
            continue;
        };
        material.extension.damage = damage;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A headless app with the cracks systems wired.
    ///
    /// Carries the level plugin too: grading reads `DamageLevel` rather than
    /// health directly, so without it every section would read pristine and
    /// every one of these tests would pass for the wrong reason. The material
    /// plugin is NOT added - it wants a render app - so the systems are added
    /// by hand and the asset store is initialised on its own.
    fn cracks_app() -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.add_plugins(DamageLevelPlugin);
        // The real fitting path, not a hand-inserted marker: cracks are an
        // AUTHORED effect, and these tests are only honest if a section gets
        // one the way a spawned one does.
        app.add_plugins(DamageEffectsPlugin { render: true });
        app.init_asset::<StandardMaterial>();
        app.init_asset::<SectionCracksMaterial>();
        app.add_systems(
            Update,
            (
                mark_section_meshes,
                resolve_pending_cracks,
                grade_section_cracks,
            )
                .chain(),
        );
        app
    }

    /// Spawn a section with `health` left of `max`, and one mesh under it
    /// wearing `shared`.
    fn section_with_mesh(app: &mut App, shared: &Handle<StandardMaterial>) -> (Entity, Entity) {
        let section = app
            .world_mut()
            .spawn((
                SectionMarker,
                Health {
                    current: 100.0,
                    max: 100.0,
                },
            ))
            .id();
        let mesh = app
            .world_mut()
            .spawn((MeshMaterial3d(shared.clone()), ChildOf(section)))
            .id();
        (section, mesh)
    }

    /// How cracked a captured mesh currently is.
    fn damage_of(app: &App, mesh: Entity) -> f32 {
        let cracks = app
            .world()
            .get::<SectionCracks>(mesh)
            .expect("the mesh was captured");
        app.world()
            .resource::<Assets<SectionCracksMaterial>>()
            .get(&cracks.material)
            .expect("its material exists")
            .extension
            .damage
    }

    /// THE claim: a section's own health drives its own surface, and a shared
    /// gltf material is never written to.
    #[test]
    fn a_section_cracks_by_its_own_health_and_not_its_neighbours() {
        let mut app = cracks_app();
        let pristine = Color::srgb(0.8, 0.8, 0.8);
        let shared = app
            .world_mut()
            .resource_mut::<Assets<StandardMaterial>>()
            .add(StandardMaterial {
                base_color: pristine,
                ..default()
            });

        let (hurt, hurt_mesh) = section_with_mesh(&mut app, &shared);
        let (_, whole_mesh) = section_with_mesh(&mut app, &shared);

        app.update();
        app.update();

        assert_eq!(damage_of(&app, hurt_mesh), 0.0, "a whole section is whole");

        app.world_mut().get_mut::<Health>(hurt).unwrap().current = 25.0;
        app.update();

        assert!(
            (damage_of(&app, hurt_mesh) - 0.75).abs() < 1e-5,
            "a section cracks by the share of its health that is gone"
        );
        assert_eq!(
            damage_of(&app, whole_mesh),
            0.0,
            "and its neighbour sharing the same gltf material does not"
        );
        assert_eq!(
            app.world()
                .resource::<Assets<StandardMaterial>>()
                .get(&shared)
                .unwrap()
                .base_color,
            pristine,
            "the shared source material is never mutated"
        );
    }

    /// Inactivity is capability state, not damage: a healthy severed fragment
    /// reads by its HEALTH rather than cracking apart.
    #[test]
    fn an_intact_inactive_section_is_not_cracked() {
        let mut app = cracks_app();
        let shared = app
            .world_mut()
            .resource_mut::<Assets<StandardMaterial>>()
            .add(StandardMaterial::default());
        let (section, mesh) = section_with_mesh(&mut app, &shared);

        app.world_mut()
            .entity_mut(section)
            .insert(SectionInactiveMarker);
        app.update();
        app.update();

        assert_eq!(damage_of(&app, mesh), 0.0);
    }

    /// A fixture's meshes are left out entirely, while a real section mesh in
    /// the same world is still captured.
    ///
    /// The cost of getting this wrong is paid per MESH: a clad ship carries
    /// hundreds of plates of one to three meshes each, and every one of them
    /// would build a material to grade off the health of the hull behind it.
    #[test]
    fn a_fixture_mesh_is_never_captured() {
        let mut app = cracks_app();
        let shared = app
            .world_mut()
            .resource_mut::<Assets<StandardMaterial>>()
            .add(StandardMaterial::default());
        let (section, section_mesh) = section_with_mesh(&mut app, &shared);
        let plate = app
            .world_mut()
            .spawn((SectionFixture, ChildOf(section)))
            .id();
        // What the scene loader hangs under a greeble: a mesh two fixtures deep.
        let greeble = app.world_mut().spawn((SectionFixture, ChildOf(plate))).id();
        let greeble_mesh = app
            .world_mut()
            .spawn((MeshMaterial3d(shared.clone()), ChildOf(greeble)))
            .id();

        app.update();
        app.update();

        assert!(
            app.world().get::<SectionCracks>(section_mesh).is_some(),
            "a real section mesh is captured"
        );
        assert!(
            app.world().get::<SectionCracks>(greeble_mesh).is_none(),
            "a plate's decoration must not crack - the plate comes OFF instead"
        );
        assert!(
            app.world()
                .get::<PendingSectionCracks>(greeble_mesh)
                .is_none(),
            "and must not sit pending, retried every frame"
        );
        assert_eq!(
            app.world()
                .get::<MeshMaterial3d<StandardMaterial>>(greeble_mesh)
                .expect("it keeps its material")
                .0
                .id(),
            shared.id(),
            "it keeps the shared material rather than building one"
        );
    }

    /// Cracks are AUTHORED. A section that does not author them must stay the
    /// surface it was painted, and must not pay for a material either.
    #[test]
    fn a_section_that_authors_no_cracks_is_never_captured() {
        let mut app = cracks_app();
        let shared = app
            .world_mut()
            .resource_mut::<Assets<StandardMaterial>>()
            .add(StandardMaterial::default());

        let plain = app
            .world_mut()
            .spawn((
                SectionMarker,
                DamageEffects::none(),
                Health {
                    current: 10.0,
                    max: 100.0,
                },
            ))
            .id();
        let mesh = app
            .world_mut()
            .spawn((MeshMaterial3d(shared.clone()), ChildOf(plain)))
            .id();

        app.update();
        app.update();

        assert!(app.world().get::<SectionCracks>(mesh).is_none());
        assert!(app.world().get::<PendingSectionCracks>(mesh).is_none());
        assert_eq!(
            app.world()
                .get::<MeshMaterial3d<StandardMaterial>>(mesh)
                .expect("the mesh keeps its material")
                .0
                .id(),
            shared.id(),
        );
    }

    /// A section that does not crack must not fall through to the section ABOVE
    /// it: a turret authoring no cracks would otherwise be graded by the hull it
    /// is bolted to, reporting a pool that is not its own.
    #[test]
    fn an_uncracked_section_does_not_borrow_its_parents_health() {
        let mut app = cracks_app();
        let shared = app
            .world_mut()
            .resource_mut::<Assets<StandardMaterial>>()
            .add(StandardMaterial::default());

        let hull = app
            .world_mut()
            .spawn((
                SectionMarker,
                Health {
                    current: 10.0,
                    max: 100.0,
                },
            ))
            .id();
        let turret = app
            .world_mut()
            .spawn((
                SectionMarker,
                DamageEffects::none(),
                Health {
                    current: 100.0,
                    max: 100.0,
                },
                ChildOf(hull),
            ))
            .id();
        let mesh = app
            .world_mut()
            .spawn((MeshMaterial3d(shared.clone()), ChildOf(turret)))
            .id();

        app.update();
        app.update();

        assert!(
            app.world().get::<SectionCracks>(mesh).is_none(),
            "the walk stops at the uncracked section, it does not carry on up"
        );
    }

    /// A section mesh whose material asset is not yet loaded (async gltf) must
    /// stay pending and be captured once it arrives, not dropped.
    #[test]
    fn capture_rearms_until_the_material_asset_loads() {
        let mut app = cracks_app();
        let handle = app
            .world()
            .resource::<Assets<StandardMaterial>>()
            .reserve_handle();
        let (_, mesh) = section_with_mesh(&mut app, &handle);

        app.update();
        app.update();
        assert!(
            app.world().get::<SectionCracks>(mesh).is_none(),
            "must not capture before the material asset exists"
        );
        assert!(
            app.world().get::<PendingSectionCracks>(mesh).is_some(),
            "must stay pending, re-arming, until the asset loads"
        );

        app.world_mut()
            .resource_mut::<Assets<StandardMaterial>>()
            .insert(&handle, StandardMaterial::default())
            .expect("insert material asset");
        app.update();

        assert!(
            app.world().get::<SectionCracks>(mesh).is_some(),
            "must capture once the asset loads"
        );
        assert!(app.world().get::<PendingSectionCracks>(mesh).is_none());
    }

    /// Regression: a ship exploding chain-destroys its section leaves, and the
    /// marker's deferred insert then landed on a despawned entity and panicked
    /// (the 2026-08-13 "The Raid" crash shape).
    #[test]
    fn marking_a_mesh_chain_destroyed_the_same_frame_does_not_panic() {
        use bevy::ecs::error::{panic, FallbackErrorHandler};

        #[derive(Resource)]
        struct Doomed(Entity);
        fn despawn_doomed(mut commands: Commands, doomed: Res<Doomed>) {
            commands.entity(doomed.0).despawn();
        }

        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.init_asset::<StandardMaterial>();
        app.init_asset::<SectionCracksMaterial>();
        // Match the game/binary: a command error on a despawned entity is a
        // hard panic, not a silent warn.
        app.insert_resource(FallbackErrorHandler(panic));
        let shared = app
            .world_mut()
            .resource_mut::<Assets<StandardMaterial>>()
            .add(StandardMaterial::default());
        let section = app.world_mut().spawn((SectionMarker, DamageCracks)).id();
        let mesh = app
            .world_mut()
            .spawn((MeshMaterial3d(shared.clone()), ChildOf(section)))
            .id();
        app.insert_resource(Doomed(mesh));
        // The crash order: `despawn_doomed` queues the despawn, then
        // `mark_section_meshes` (its query still sees the live mesh) queues the
        // insert; with no sync point between, the despawn applies first.
        app.add_systems(
            Update,
            (despawn_doomed, mark_section_meshes).chain_ignore_deferred(),
        );

        app.update();

        assert!(
            app.world().get_entity(mesh).is_err(),
            "the section mesh was chain-destroyed this frame"
        );
    }
}
