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
//! # Shared bucket materials
//!
//! Sections render via gltf `WorldAssetRoot` scenes, and a gltf scene's
//! materials are shared handles across every instance of the same mesh - so
//! writing a damage level into a material in place would crack every section
//! that shares that mesh at once. Damage is therefore QUANTISED into
//! [`SECTION_CRACK_BUCKETS`] steps and a mesh SWAPS to the shared
//! [`SectionCracksMaterial`] for its `(source material, bucket)` pair. Nothing
//! is ever written into a built material, so no section can crack a neighbour.
//!
//! Quantising is what keeps the material count off the fleet size. Draw calls
//! bin on the material, so a continuous value per section put every section
//! mesh in a bin of its own - 2,652 bins of one instance each on an eleven-ship
//! gallery, and roughly half the frame rate. Buckets cap the bins at source
//! materials times [`SECTION_CRACK_BUCKETS`] however many ships are in the
//! scene.
//!
//! # A pristine section is not swapped at all
//!
//! Bucket 0 is the pristine step, and a mesh in it KEEPS its own
//! `MeshMaterial3d<StandardMaterial>`. The bucket-0 material is never built.
//!
//! This is not an optimisation of a working scheme, it is the repair of a wrong
//! one. A pristine section used to be swapped onto a bucket-0
//! [`SectionCracksMaterial`], on the reasoning that one shared bucket is one
//! bin, so an undamaged fleet batches as if the effect were not there. The bin
//! count was right and the conclusion did not follow: a bucket-0 material is an
//! [`ExtendedMaterial`], which is a DIFFERENT PIPELINE, and its draws cannot
//! batch with anything still drawn as a [`StandardMaterial`]. Measured by
//! ablation, an idle scene with nothing damaged anywhere paid 10.4% of mean
//! frame time and 12.7% of p95 for cracks it was not showing.
//!
//! The swap is therefore deferred to the first bucket that draws something, and
//! reversed if a section is ever healed back to pristine. An undamaged fleet now
//! costs what it costs without this module, and a battered one pays in
//! proportion to how battered it is.
//!
//! The shader was always built for this: crack width is `damage * damage`, so
//! bucket 0 renders EXACTLY the source material. Not swapping it is invisible.
//!
//! The cracked material is also what a dead section wears as it tumbles away:
//! destruction detaches the art rather than re-drawing it, so the last bucket
//! swapped in here is the one the wreck leaves with.
//!
//! # Timing
//!
//! Capture keys on `Added<MeshMaterial3d<StandardMaterial>>`, which fires the
//! frame a mesh appears - whether it is a synchronous cuboid or an
//! asynchronously instantiated gltf node - so it does not depend on any
//! scene-ready signal. The handle can exist before its asset does, so the swap
//! is retried until the asset arrives rather than dropped.

use bevy::{
    pbr::{ExtendedMaterial, MaterialExtension},
    platform::collections::HashMap,
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

/// `DamageCracks`, `SectionCracks`, the cracks materials and their plugin.
pub mod prelude {
    pub use super::{
        crack_bucket, DamageCracks, SectionCracks, SectionCracksMaterial, SectionCracksMaterialExt,
        SectionCracksMaterials, SectionCracksPlugin, SECTION_CRACK_BUCKETS, SECTION_CRACK_SCALE,
    };
}

/// Fracture lines per unit of a section's own local space.
///
/// A section is one to three units across, so this puts a handful of major
/// fractures on one - enough to read as a broken plate rather than as a texture,
/// and few enough that the finer octaves have room to be seen.
pub const SECTION_CRACK_SCALE: f32 = 2.5;

/// How many steps a section's surface degrades through, pristine included.
///
/// This is the number of material bins a single source material can produce, so
/// it is the whole reason the effect does not cost the frame: bins are capped at
/// source materials times this, whatever the fleet size. Raising it is cheap and
/// lowering it is free; what it buys is how smoothly a surface goes from painted
/// to burnt, and what it costs is bins.
pub const SECTION_CRACK_BUCKETS: usize = 8;

/// The bucket a damage level snaps to: 0 pristine, `SECTION_CRACK_BUCKETS - 1`
/// burnt out.
///
/// Nearest rather than floor, so a section is never drawn a whole step less
/// damaged than it is, and so the pristine bucket is a narrow band around zero
/// rather than a whole step of real damage that draws as untouched.
pub fn crack_bucket(damage: f32) -> usize {
    // The clamp bounds the product to 0..=top, so the cast cannot wrap.
    let top = SECTION_CRACK_BUCKETS - 1;
    (damage.clamp(0.0, 1.0) * top as f32).round() as usize
}

/// The damage value bucket `bucket` is drawn at.
fn bucket_damage(bucket: usize) -> f32 {
    bucket as f32 / (SECTION_CRACK_BUCKETS - 1) as f32
}

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
    /// The extension for a section drawn `damage` far gone.
    #[cfg_attr(
        not(target_arch = "wasm32"),
        expect(
            clippy::needless_update,
            reason = "the webgl2 padding fields exist only on wasm32, and there this update is what fills them"
        )
    )]
    pub fn new(damage: f32) -> Self {
        Self {
            damage,
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
    /// The pristine material this mesh was painted with, and the key its bucket
    /// materials are built under.
    ///
    /// Held STRONG: it is the only reference left once the mesh's
    /// `MeshMaterial3d<StandardMaterial>` is swapped away, and a later bucket
    /// has to be built from it. It is also what makes the key sound - a source
    /// that cannot be dropped cannot have its [`AssetId`] handed to something
    /// else.
    pub source: Handle<StandardMaterial>,
    /// Which of the [`SECTION_CRACK_BUCKETS`] steps this mesh is drawn at.
    pub bucket: usize,
    /// The shared material for that bucket, which this mesh draws with, or
    /// `None` at bucket 0 - where the mesh keeps its own [`Self::source`] and
    /// no cracked material exists at all. `None` and `bucket == 0` always agree.
    pub material: Option<Handle<SectionCracksMaterial>>,
}

/// Every cracked material built so far, keyed by the source material it was
/// built from and the bucket it draws.
///
/// The one place a [`SectionCracksMaterial`] is ever created, so it is also the
/// bound on how many exist: source materials times [`SECTION_CRACK_BUCKETS`].
/// Buckets are built ON DEMAND rather than up front, because most sources never
/// reach most buckets - a pristine fleet is one bucket - and a source a mod
/// mints per instance would otherwise cost eight materials it never draws.
#[derive(Resource, Default, Debug)]
pub struct SectionCracksMaterials {
    by_source: HashMap<
        AssetId<StandardMaterial>,
        [Option<Handle<SectionCracksMaterial>>; SECTION_CRACK_BUCKETS],
    >,
}

impl SectionCracksMaterials {
    /// The shared material `source` draws with at `bucket`, building it the
    /// first time. `None` while the source asset itself has not loaded.
    fn material(
        &mut self,
        source: &Handle<StandardMaterial>,
        bucket: usize,
        standard: &Assets<StandardMaterial>,
        cracked: &mut Assets<SectionCracksMaterial>,
    ) -> Option<Handle<SectionCracksMaterial>> {
        if let Some(handle) = self
            .by_source
            .get(&source.id())
            .and_then(|buckets| buckets[bucket].clone())
        {
            return Some(handle);
        }
        // Read the source BEFORE taking an entry: a handle whose asset has not
        // loaded gets no entry, so an unresolved mesh cannot leave one behind.
        let pristine = standard.get(source)?.clone();
        let handle = cracked.add(SectionCracksMaterial {
            base: pristine,
            extension: SectionCracksMaterialExt::new(bucket_damage(bucket)),
        });
        self.by_source.entry(source.id()).or_default()[bucket] = Some(handle.clone());
        Some(handle)
    }

    /// How many source materials have bucket materials built. Test and
    /// instrument surface.
    pub fn sources(&self) -> usize {
        self.by_source.len()
    }
}

/// A section mesh awaiting material capture.
///
/// Its `StandardMaterial` handle may exist before the asset itself resolves
/// (async gltf load), so marking is decoupled from capture: [`mark_section_meshes`]
/// tags the mesh once (doing the `ChildOf` walk), and [`resolve_pending_cracks`]
/// retries the swap every frame until the asset is available - self-re-arming,
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
        trace!("SectionCracksPlugin: build");

        app.register_type::<DamageCracks>();
        app.init_resource::<SectionCracksMaterials>();
        app.add_plugins(MaterialPlugin::<SectionCracksMaterial>::default());
        app.add_systems(
            Update,
            (
                forget_dead_sources,
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

/// Drop the bucket materials of any source material that is gone.
///
/// Without this the registry is a leak rather than a cache: every source a
/// scene ever drew would keep its bucket materials for the process, so a
/// campaign that loads and drops content would accumulate them. Base sources
/// are all long-lived, so the entries this frees today are a mod's;
/// [`SectionCracks`] holds the only strong reference to a source, so the event
/// fires exactly when the last mesh drawn from it is gone.
fn forget_dead_sources(
    mut registry: ResMut<SectionCracksMaterials>,
    mut events: MessageReader<AssetEvent<StandardMaterial>>,
) {
    for event in events.read() {
        match event {
            AssetEvent::Unused { id } | AssetEvent::Removed { id } => {
                registry.by_source.remove(id);
            }
            _ => {}
        }
    }
}

/// Tag every freshly-spawned section mesh for capture. The `ChildOf` walk
/// happens here, once per mesh; the material swap is deferred to
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

/// Swap each pending mesh onto the shared material for its section's bucket,
/// once its pristine material is available. Retries until it loads; this query
/// is normally empty.
///
/// The bucket is read here rather than left to [`grade_section_cracks`] so a
/// mesh that appears on an already-battered section - a gltf node that finished
/// loading mid-fight - is drawn right on its first frame.
fn resolve_pending_cracks(
    mut commands: Commands,
    standard: Res<Assets<StandardMaterial>>,
    mut cracked: ResMut<Assets<SectionCracksMaterial>>,
    mut registry: ResMut<SectionCracksMaterials>,
    q_pending: Query<(
        Entity,
        &MeshMaterial3d<StandardMaterial>,
        &PendingSectionCracks,
    )>,
    q_level: Query<&DamageLevel, With<SectionMarker>>,
) {
    for (entity, material, pending) in &q_pending {
        let bucket = crack_bucket(q_level.get(pending.section).map_or(0.0, |level| level.0));

        // Pristine: capture it, but leave it on its own material. Nothing is
        // built and nothing is swapped, so this does not wait on the source
        // asset either - a pristine mesh is captured the frame it is marked
        // whether or not its gltf material has resolved.
        if bucket == 0 {
            commands
                .entity(entity)
                .try_insert(SectionCracks {
                    section: pending.section,
                    source: material.0.clone(),
                    bucket,
                    material: None,
                })
                .try_remove::<PendingSectionCracks>();
            continue;
        }

        let Some(handle) = registry.material(&material.0, bucket, &standard, &mut cracked) else {
            // Asset not loaded yet; keep the pending marker and retry next frame.
            continue;
        };

        // NOTE: same despawn race as `mark_section_meshes`.
        commands
            .entity(entity)
            .try_insert((
                MeshMaterial3d(handle.clone()),
                SectionCracks {
                    section: pending.section,
                    source: material.0.clone(),
                    bucket,
                    material: Some(handle),
                },
            ))
            .try_remove::<MeshMaterial3d<StandardMaterial>>()
            .try_remove::<PendingSectionCracks>();
    }
}

/// Swap every captured mesh onto the shared material for its section's current
/// bucket.
///
/// Only on a bucket CHANGE, which is at most [`SECTION_CRACK_BUCKETS`] - 1 times
/// in a mesh's life: an idle ship touches nothing, and a fight writes no
/// material at all.
fn grade_section_cracks(
    mut commands: Commands,
    standard: Res<Assets<StandardMaterial>>,
    mut cracked: ResMut<Assets<SectionCracksMaterial>>,
    mut registry: ResMut<SectionCracksMaterials>,
    mut q_cracks: Query<(Entity, &mut SectionCracks)>,
    q_level: Query<&DamageLevel, With<SectionMarker>>,
) {
    for (entity, mut cracks) in &mut q_cracks {
        // The SAME level the carve reads, so a section's surface and its shape
        // cannot disagree about how far gone it is.
        let damage = q_level.get(cracks.section).map_or(0.0, |level| level.0);
        let bucket = crack_bucket(damage);
        if bucket == cracks.bucket {
            continue;
        }

        // Healed back to pristine: hand the mesh its own material back and drop
        // the cracked one, so a repaired section stops paying the extended
        // pipeline the same way an untouched one never starts.
        if bucket == 0 {
            // NOTE: same despawn race as `mark_section_meshes`.
            commands
                .entity(entity)
                .try_insert(MeshMaterial3d(cracks.source.clone()))
                .try_remove::<MeshMaterial3d<SectionCracksMaterial>>();
            cracks.bucket = bucket;
            cracks.material = None;
            continue;
        }

        let Some(handle) = registry.material(&cracks.source, bucket, &standard, &mut cracked)
        else {
            continue;
        };

        // NOTE: same despawn race as `mark_section_meshes`.
        let mut entity = commands.entity(entity);
        entity.try_insert(MeshMaterial3d(handle.clone()));
        // Leaving pristine for the first time: the source material is still on
        // the mesh and has to come off, or it draws through both pipelines.
        if cracks.bucket == 0 {
            entity.try_remove::<MeshMaterial3d<StandardMaterial>>();
        }
        cracks.bucket = bucket;
        cracks.material = Some(handle);
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
        app.init_resource::<SectionCracksMaterials>();
        app.add_systems(
            Update,
            (
                forget_dead_sources,
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
        // Pristine reads 0.0 with no material to read it from: bucket 0 draws
        // the source material, which is what damage 0.0 renders as anyway.
        let Some(material) = cracks.material.as_ref() else {
            return 0.0;
        };
        app.world()
            .resource::<Assets<SectionCracksMaterial>>()
            .get(material)
            .expect("its material exists")
            .extension
            .damage
    }

    /// How many distinct cracked materials exist.
    fn crack_materials(app: &App) -> usize {
        app.world()
            .resource::<Assets<SectionCracksMaterial>>()
            .len()
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
            (damage_of(&app, hurt_mesh) - bucket_damage(crack_bucket(0.75))).abs() < 1e-5,
            "a section cracks by the share of its health that is gone, to the nearest bucket"
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

    /// The quantiser: pristine is a bucket of its own, dead is the last one,
    /// and a level in between snaps to the NEAREST step rather than down to it.
    #[test]
    fn a_damage_level_snaps_to_the_nearest_bucket() {
        let top = SECTION_CRACK_BUCKETS - 1;
        let step = 1.0 / top as f32;

        assert_eq!(crack_bucket(0.0), 0);
        assert_eq!(crack_bucket(1.0), top);
        assert_eq!(crack_bucket(2.0), top, "an out-of-range level clamps");
        assert_eq!(
            crack_bucket(step * 0.49),
            0,
            "just under half a step still reads pristine"
        );
        assert_eq!(
            crack_bucket(step * 0.51),
            1,
            "just over half a step wears the first crack"
        );
        assert_eq!(bucket_damage(0), 0.0);
        assert_eq!(bucket_damage(top), 1.0);
    }

    /// THE claim the frame rate rests on: what bounds the material count is the
    /// bucket count, not how many sections are on screen. One material per
    /// section mesh made every section its own draw bin and cost roughly half
    /// the frame rate on an eleven-ship gallery.
    #[test]
    fn sections_draw_through_at_most_one_material_per_bucket() {
        let mut app = cracks_app();
        let shared = app
            .world_mut()
            .resource_mut::<Assets<StandardMaterial>>()
            .add(StandardMaterial::default());

        let sections: Vec<_> = (0..40)
            .map(|_| section_with_mesh(&mut app, &shared))
            .collect();

        app.update();
        app.update();

        assert_eq!(
            crack_materials(&app),
            0,
            "an undamaged fleet builds no cracked material at all"
        );

        // A different damage level for every section, so one material per
        // section would be forty of them.
        for (index, (section, _)) in sections.iter().enumerate() {
            app.world_mut().get_mut::<Health>(*section).unwrap().current =
                100.0 - index as f32 * 2.5;
        }
        app.update();

        assert_eq!(
            crack_materials(&app),
            SECTION_CRACK_BUCKETS - 1,
            "forty damage levels draw through the buckets past pristine, and nothing more"
        );
        // Sections 3 and 4 are both in bucket 1; 0 and 1 are still pristine and
        // hold no material to compare.
        assert_eq!(
            app.world()
                .get::<SectionCracks>(sections[3].1)
                .unwrap()
                .material
                .as_ref()
                .map(Handle::id),
            app.world()
                .get::<SectionCracks>(sections[4].1)
                .unwrap()
                .material
                .as_ref()
                .map(Handle::id),
            "two sections in the same bucket share one material"
        );
    }

    /// THE claim the pristine skip rests on: an undamaged section is not moved
    /// onto a second material pipeline, because a bucket-0 material draws
    /// identically to the source and cannot batch with it.
    #[test]
    fn a_pristine_section_keeps_its_own_material() {
        let mut app = cracks_app();
        let shared = app
            .world_mut()
            .resource_mut::<Assets<StandardMaterial>>()
            .add(StandardMaterial::default());
        let (_, mesh) = section_with_mesh(&mut app, &shared);

        app.update();
        app.update();

        let cracks = app
            .world()
            .get::<SectionCracks>(mesh)
            .expect("a pristine mesh is still captured");
        assert_eq!(cracks.bucket, 0);
        assert!(cracks.material.is_none(), "and holds no cracked material");
        assert!(
            app.world()
                .get::<MeshMaterial3d<StandardMaterial>>(mesh)
                .is_some(),
            "it still draws through its own source material"
        );
        assert!(
            app.world()
                .get::<MeshMaterial3d<SectionCracksMaterial>>(mesh)
                .is_none(),
            "and not through the cracked pipeline"
        );
        assert_eq!(crack_materials(&app), 0, "nothing was built for it");
    }

    /// Damage swaps the pipeline on, healing swaps it back off, and the mesh is
    /// never drawn through both at once.
    #[test]
    fn a_section_swaps_onto_cracks_when_hurt_and_back_when_healed() {
        let mut app = cracks_app();
        let shared = app
            .world_mut()
            .resource_mut::<Assets<StandardMaterial>>()
            .add(StandardMaterial::default());
        let (section, mesh) = section_with_mesh(&mut app, &shared);

        app.update();
        app.update();

        app.world_mut().get_mut::<Health>(section).unwrap().current = 20.0;
        app.update();

        assert!(app.world().get::<SectionCracks>(mesh).unwrap().bucket > 0);
        assert!(
            app.world()
                .get::<MeshMaterial3d<SectionCracksMaterial>>(mesh)
                .is_some(),
            "a hurt section draws cracked"
        );
        assert!(
            app.world()
                .get::<MeshMaterial3d<StandardMaterial>>(mesh)
                .is_none(),
            "and its source material came off, so it draws through one pipeline"
        );

        app.world_mut().get_mut::<Health>(section).unwrap().current = 100.0;
        app.update();

        let cracks = app.world().get::<SectionCracks>(mesh).unwrap();
        assert_eq!(cracks.bucket, 0);
        assert!(cracks.material.is_none());
        assert!(
            app.world()
                .get::<MeshMaterial3d<StandardMaterial>>(mesh)
                .is_some(),
            "a healed section gets its own material back"
        );
        assert!(
            app.world()
                .get::<MeshMaterial3d<SectionCracksMaterial>>(mesh)
                .is_none(),
            "and stops paying the extended pipeline"
        );
    }

    /// A source material that dies takes its bucket materials with it.
    ///
    /// The registry is a cache, not a ledger: a torpedo warhead is tinted per
    /// LAUNCH, so remembering every source ever seen would grow the material
    /// store by a bucket set per shot fired.
    #[test]
    fn bucket_materials_die_with_the_source_they_were_built_from() {
        let mut app = cracks_app();
        let shared = app
            .world_mut()
            .resource_mut::<Assets<StandardMaterial>>()
            .add(StandardMaterial::default());
        let (section, mesh) = section_with_mesh(&mut app, &shared);
        // The captured mesh must hold the LAST strong handle to the source.
        drop(shared);
        // HURT, so a bucket material is built at all - a pristine section draws
        // its source and the registry stays empty, which would prove nothing
        // about forgetting.
        app.world_mut().get_mut::<Health>(section).unwrap().current = 25.0;

        app.update();
        app.update();
        assert_eq!(crack_materials(&app), 1);

        app.world_mut().entity_mut(mesh).despawn();
        for _ in 0..4 {
            app.update();
        }

        assert_eq!(
            app.world().resource::<SectionCracksMaterials>().sources(),
            0,
            "the registry forgets a source nothing draws from"
        );
        assert_eq!(
            crack_materials(&app),
            0,
            "and its bucket materials go with it"
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
        let (section, mesh) = section_with_mesh(&mut app, &handle);
        // HURT, because only a mesh past the pristine bucket has to build a
        // material and therefore has anything to wait for. A pristine one is
        // captured on its own source handle whether or not the asset resolved,
        // which the assertion at the end of this test pins.
        app.world_mut().get_mut::<Health>(section).unwrap().current = 25.0;

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

        // And the pristine case does not wait at all: same unresolved handle,
        // full health, captured on the first pass.
        let pending = app
            .world()
            .resource::<Assets<StandardMaterial>>()
            .reserve_handle();
        let (_, whole_mesh) = section_with_mesh(&mut app, &pending);
        app.update();
        app.update();

        assert!(
            app.world().get::<SectionCracks>(whole_mesh).is_some(),
            "a pristine mesh needs no material, so it never waits for one"
        );
        assert!(app
            .world()
            .get::<PendingSectionCracks>(whole_mesh)
            .is_none());
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
