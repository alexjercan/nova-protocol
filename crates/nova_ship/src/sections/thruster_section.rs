//! Defines a thruster section for a spaceship, which provides thrust in a specified direction.

use avian3d::prelude::*;
use bevy::{
    pbr::{ExtendedMaterial, MaterialExtension},
    platform::collections::HashMap,
    prelude::*,
    render::render_resource::AsBindGroup,
    shader::ShaderRef,
};
use nova_gameplay::prelude::{
    AssetRef, DamageLevel, SectionClass, SectionInactiveMarker, ThrusterSectionMarker,
    TriangleMeshBuilder,
};

use crate::{
    prelude::{PlaceholderArt, RenderMeshTransform, SectionRenderMeshTransform, SectionRenderOf},
    sections::damage_plume::prelude::{plume_scale, DamagePlume},
};

/// The `thruster_section` spawner, its config, input and magnitude components, the exhaust
/// configuration and `ThrusterSectionPlugin`.
pub mod prelude {
    pub use super::{
        plume_bucket, thruster_section, ExhaustMaterials, ExhaustMeshes, ThrusterExhaust,
        ThrusterExhaustConfig, ThrusterExhaustMaterial, ThrusterExhaustShape,
        ThrusterPlumeMaterial, ThrusterSectionConfig, ThrusterSectionInput,
        ThrusterSectionMagnitude, ThrusterSectionPlugin, ThrusterSectionRenderMarker,
        EXHAUST_PLUME_BUCKETS,
    };
}

const THRUSTER_SECTION_DEFAULT_MAGNITUDE: f32 = 1.0;

/// Configuration for a thruster section of a spaceship.
#[derive(Clone, Debug, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ThrusterSectionConfig {
    /// The magnitude of the force produced by this thruster section.
    pub magnitude: f32,
    /// The render mesh of the section, defaults to prototype mesh if None.
    #[reflect(ignore)]
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub render_mesh: Option<AssetRef<WorldAsset>>,
    /// Optional transform (position + rotation) applied to the thruster's render
    /// mesh only. None = the mesh sits at the section origin (unchanged).
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub render_mesh_transform: Option<RenderMeshTransform>,
    /// The engine-hum loop this thruster contributes to. An authorable
    /// [`AssetRef<AudioSource>`] like the render mesh: thrusters sharing a
    /// sound share one loop entity whose volume tracks the loudest ship burning
    /// it. AUTHORED-OR-SILENT: a thruster with no loop_sound hums nothing; base
    /// thrusters author
    /// `self://sounds/thruster_loop.wav` via gen_content. Snapshotted
    /// (unresolved) as `ThrusterSectionLoopSound`; the audio module resolves
    /// and groups by handle.
    #[reflect(ignore)]
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub loop_sound: Option<AssetRef<AudioSource>>,
    /// Authorable exhaust cone: where it sits, how it is rotated, and its shape.
    /// `None` uses the default placement/shape (same cone the procedural
    /// thruster always had); set it to align the cone with a custom render mesh.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub exhaust: Option<ThrusterExhaust>,
}

/// A thruster's authored hum, snapshotted UNRESOLVED from
/// [`ThrusterSectionConfig::loop_sound`] by [`thruster_section`]; the audio
/// module resolves it and groups thrusters by handle. `pub(crate)` for the
/// audio module.
#[derive(Component, Clone, Debug, Deref, DerefMut, Reflect)]
pub(crate) struct ThrusterSectionLoopSound(#[reflect(ignore)] pub Option<AssetRef<AudioSource>>);

impl Default for ThrusterSectionConfig {
    fn default() -> Self {
        Self {
            magnitude: THRUSTER_SECTION_DEFAULT_MAGNITUDE,
            render_mesh: None,
            render_mesh_transform: None,
            loop_sound: None,
            exhaust: None,
        }
    }
}

/// The section's authored exhaust placement/shape, snapshotted from
/// [`ThrusterSectionConfig::exhaust`] so the render observer can spawn the cone.
#[derive(Component, Clone, Debug, Deref, DerefMut, Reflect)]
struct ThrusterSectionExhaust(Option<ThrusterExhaust>);

/// Helper function to create an thruster section entity bundle.
pub fn thruster_section(config: ThrusterSectionConfig) -> impl Bundle {
    debug!("thruster_section: config {:?}", config);

    (
        ThrusterSectionMarker,
        SectionClass::Thruster,
        ThrusterSectionMagnitude(config.magnitude),
        ThrusterSectionInput(0.0),
        ThrusterSectionLoopSound(config.loop_sound.clone()),
        ThrusterSectionRenderMesh(config.render_mesh),
        SectionRenderMeshTransform(config.render_mesh_transform),
        ThrusterSectionExhaust(config.exhaust),
    )
}

/// Configuration for the thruster exhaust shader (cone shape + glow). Authorable
/// via [`ThrusterExhaust::shape`]; `#[serde(default)]` lets a mod set only the
/// fields it cares about.
#[derive(Component, Clone, Debug, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(default))]
pub struct ThrusterExhaustConfig {
    /// Cross-section of the exhaust flame: round `Cone` (sized by the radius
    /// fields) or axis-aligned `Rect` (sized by `width`/`height`).
    pub geometry: ThrusterExhaustShape,
    /// Rect cross-section FULL width (local X) and height (local Z), used when
    /// `geometry` is `Rect`. Set these to the nozzle hole size; the inner core
    /// scales down by `exhaust_inner_radius / exhaust_radius`. Ignored for `Cone`.
    pub width: f32,
    /// Rect cross-section FULL height (local Z); see `width`. Ignored for `Cone`.
    pub height: f32,
    /// Length of the outer exhaust cone along +Y.
    pub exhaust_height: f32,
    /// Base radius of the outer exhaust cone.
    pub exhaust_radius: f32,
    /// Peak opacity/intensity of the outer exhaust cone at full throttle.
    pub exhaust_max: f32,
    /// Length of the inner (core) exhaust cone along +Y.
    pub exhaust_inner_height: f32,
    /// Base radius of the inner (core) exhaust cone.
    pub exhaust_inner_radius: f32,
    /// Peak opacity/intensity of the inner (core) exhaust cone at full throttle.
    pub exhaust_inner_max: f32,
    /// Emissive glow color of the outer exhaust cone.
    pub emissive_color: LinearRgba,
    /// Emissive glow color of the inner (core) exhaust cone.
    pub emissive_inner_color: LinearRgba,
}

/// The exhaust flame's cross-section. `Rect` builds an axis-aligned rectangular
/// pyramid (sized by `width`/`height`) instead of a round cone; the glow shader
/// is shape-agnostic (it elongates along +Y using the xz radius, which still
/// fades a rectangle).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ThrusterExhaustShape {
    /// Round cone cross-section (sized by the radius fields).
    #[default]
    Cone,
    /// Axis-aligned rectangular cross-section (sized by `width`/`height`).
    Rect,
}

impl Default for ThrusterExhaustConfig {
    fn default() -> Self {
        Self {
            geometry: ThrusterExhaustShape::Cone,
            width: 0.8,
            height: 0.8,
            exhaust_height: 0.1,
            exhaust_radius: 0.4,
            exhaust_max: 1.0,
            exhaust_inner_height: 0.05,
            exhaust_inner_radius: 0.1,
            exhaust_inner_max: 0.5,
            emissive_color: LinearRgba::rgb(0.0, 10.0, 10.0),
            emissive_inner_color: LinearRgba::rgb(0.0, 0.0, 10.0),
        }
    }
}

/// A unit square pyramid (base square [-1,1]^2 at y=0, tip at y=1), sides
/// subdivided by height, mirroring `TriangleMeshBuilder::new_cone` so the glow
/// shader (which reads position.y / xz) treats it exactly like a cone. Scaled by
/// `(hx, height, hz)` at the call site, so the half-extents become hx/hz - a
/// non-uniform scale turns the unit square into the authored rectangle.
fn rect_exhaust_builder(height_subdivisions: u32) -> TriangleMeshBuilder {
    let vertical = height_subdivisions.max(1);
    let mut builder = TriangleMeshBuilder::new_empty();
    let tip = Vec3::new(0.0, 1.0, 0.0);
    let corners = [
        Vec3::new(1.0, 0.0, -1.0),
        Vec3::new(1.0, 0.0, 1.0),
        Vec3::new(-1.0, 0.0, 1.0),
        Vec3::new(-1.0, 0.0, -1.0),
    ];
    for i in 0..4 {
        let c0 = corners[i];
        let c1 = corners[(i + 1) % 4];
        for v in 0..vertical {
            let t0 = v as f32 / vertical as f32;
            let t1 = (v + 1) as f32 / vertical as f32;
            let p00 = Vec3::lerp(tip, c0, t0);
            let p01 = Vec3::lerp(tip, c1, t0);
            let p10 = Vec3::lerp(tip, c0, t1);
            let p11 = Vec3::lerp(tip, c1, t1);
            builder.add_triangle(Triangle3d::new(p00, p10, p11));
            builder.add_triangle(Triangle3d::new(p00, p11, p01));
        }
    }
    // Base cap, wound to face -Y (down), matching the cone's base.
    for i in 0..4 {
        let c0 = corners[i];
        let c1 = corners[(i + 1) % 4];
        builder.add_triangle(Triangle3d::new(Vec3::ZERO, c1, c0));
    }
    builder
}

/// Build the exhaust flame mesh for `shape`, with base half-extents `hx`/`hz`
/// (equal for a cone -> circular) and `height` along +Y.
fn exhaust_mesh(shape: ThrusterExhaustShape, hx: f32, hz: f32, height: f32) -> Mesh {
    let builder = match shape {
        ThrusterExhaustShape::Cone => TriangleMeshBuilder::new_cone(32, 4),
        ThrusterExhaustShape::Rect => rect_exhaust_builder(4),
    };
    builder.with_scale(Vec3::new(hx, height, hz)).build()
}

/// Everything [`exhaust_mesh`] is a function of, as a hashable key.
///
/// The floats go in as BIT PATTERNS. Two nozzles off one prototype are built
/// from the same authored numbers, so they hash equal without any tolerance -
/// and a tolerance would be the wrong tool anyway: a near miss should mint its
/// own mesh rather than silently wear a neighbour's.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
struct ExhaustMeshKey {
    geometry: ThrusterExhaustShape,
    hx: u32,
    hz: u32,
    height: u32,
}

/// The exhaust flame meshes every nozzle of one size shares.
///
/// The mesh is a pure function of the four numbers in [`ExhaustMeshKey`] and
/// nothing writes into it. Without this cache each nozzle minted two fresh
/// 32-segment cones, so a hull with a dozen drives introduced two dozen
/// distinct meshes that were all one of two shapes, and a distinct mesh is
/// prepared, bound and written every frame whatever it is a copy of.
#[derive(Resource, Default)]
pub struct ExhaustMeshes(HashMap<ExhaustMeshKey, Handle<Mesh>>);

impl ExhaustMeshes {
    /// The mesh for this flame, building it if this is the first nozzle to ask.
    fn mesh(
        &mut self,
        geometry: ThrusterExhaustShape,
        hx: f32,
        hz: f32,
        height: f32,
        meshes: &mut Assets<Mesh>,
    ) -> Handle<Mesh> {
        self.0
            .entry(ExhaustMeshKey {
                geometry,
                hx: hx.to_bits(),
                hz: hz.to_bits(),
                height: height.to_bits(),
            })
            .or_insert_with(|| meshes.add(exhaust_mesh(geometry, hx, hz, height)))
            .clone()
    }
}

/// How many throttle steps a plume's glow is drawn at, idle and full included.
///
/// This is the number of material bins ONE nozzle shape can produce, so it is
/// what keeps the frame's material work off the number of drives burning. The
/// end steps are exact: bucket 0 is a dead plume and the last bucket is a full
/// one, because `system_thrust_and_plume` asserts the shader uniform reaches
/// both.
///
/// Raising it is cheap and lowering it is free; what it buys is how smoothly a
/// plume stretches as the throttle rolls on, and what it costs is bins.
pub const EXHAUST_PLUME_BUCKETS: usize = 16;

/// The bucket a throttle snaps to: 0 out, `EXHAUST_PLUME_BUCKETS - 1` full.
///
/// Nearest rather than floor, so a plume is never drawn a whole step colder
/// than the drive is burning.
pub fn plume_bucket(input: f32) -> usize {
    // The clamp bounds the product to 0..=top, so the cast cannot wrap.
    let top = EXHAUST_PLUME_BUCKETS - 1;
    (input.clamp(0.0, 1.0) * top as f32).round() as usize
}

/// The throttle bucket `bucket` is drawn at.
fn bucket_input(bucket: usize) -> f32 {
    bucket as f32 / (EXHAUST_PLUME_BUCKETS - 1) as f32
}

/// The material a plume is, apart from its throttle: what it glows and how far
/// the shader stretches it.
///
/// The two cones of one nozzle differ in all three, so an outer flame and an
/// inner core are two specs rather than one.
#[derive(Clone, Copy, Debug)]
struct PlumeSpec {
    /// The cone's glow colour.
    emissive: LinearRgba,
    /// The shader's falloff radius (`thruster_exhaust_radius`).
    radius: f32,
    /// How far a full throttle stretches the cone (`thruster_exhaust_height`).
    height: f32,
}

impl PlumeSpec {
    /// This spec as a hashable key. Floats go in as BIT PATTERNS, the same way
    /// [`ExhaustMeshKey`] takes its extents: two nozzles off one prototype are
    /// built from the same authored numbers and hash equal without a tolerance,
    /// and a near miss should mint its own material rather than silently wear a
    /// neighbour's glow.
    fn key(&self) -> PlumeMaterialKey {
        PlumeMaterialKey {
            emissive: [
                self.emissive.red.to_bits(),
                self.emissive.green.to_bits(),
                self.emissive.blue.to_bits(),
                self.emissive.alpha.to_bits(),
            ],
            radius: self.radius.to_bits(),
            height: self.height.to_bits(),
        }
    }

    /// The material this spec draws at `bucket`.
    fn material(&self, bucket: usize) -> ThrusterPlumeMaterial {
        ExtendedMaterial {
            base: StandardMaterial {
                base_color: Color::srgba(1.0, 1.0, 1.0, 1.0),
                perceptual_roughness: 1.0,
                metallic: 0.0,
                emissive: self.emissive,
                ..default()
            },
            extension: ThrusterExhaustMaterial {
                thruster_input: bucket_input(bucket),
                ..ThrusterExhaustMaterial::default()
                    .with_exhaust_height(self.height)
                    .with_exhaust_radius(self.radius)
            },
        }
    }
}

/// Everything a plume material is a function of, as a hashable key.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
struct PlumeMaterialKey {
    emissive: [u32; 4],
    radius: u32,
    height: u32,
}

/// The exhaust glow materials every nozzle of one shape and throttle shares.
///
/// The throttle is QUANTISED into [`EXHAUST_PLUME_BUCKETS`] steps and a cone
/// SWAPS to the shared material for its `(spec, bucket)` pair, so nothing is
/// ever written into a built material. That is the whole saving:
/// `Assets::get_mut` marks a material modified whether or not the value moves,
/// and a modified material is re-extracted, re-uploaded and has its bind group
/// rebuilt that frame. A read-before-write guard covers a drive holding one
/// throttle, and covers nothing at all on a guided torpedo - whose thrust
/// genuinely changes every frame, and of which there can be a hundred in the
/// air.
///
/// Materials are minted ON DEMAND rather than up front: a plume that never
/// leaves idle should cost one material, not [`EXHAUST_PLUME_BUCKETS`] of them.
/// They are then kept for the process, which bounds the set at nozzle shapes
/// times buckets however many drives are burning.
#[derive(Resource, Default)]
pub struct ExhaustMaterials(
    HashMap<PlumeMaterialKey, [Option<Handle<ThrusterPlumeMaterial>>; EXHAUST_PLUME_BUCKETS]>,
);

impl ExhaustMaterials {
    /// The shared material `spec` draws with at `bucket`, building it the first
    /// time a nozzle of that shape reaches that throttle.
    fn material(
        &mut self,
        spec: &PlumeSpec,
        bucket: usize,
        materials: &mut Assets<ThrusterPlumeMaterial>,
    ) -> Handle<ThrusterPlumeMaterial> {
        let slot = &mut self.0.entry(spec.key()).or_default()[bucket];
        slot.get_or_insert_with(|| materials.add(spec.material(bucket)))
            .clone()
    }

    /// How many distinct nozzle shapes have materials built. Test and
    /// instrument surface.
    pub fn shapes(&self) -> usize {
        self.0.len()
    }
}

/// Base half-extents `(hx, hz)` and the shader falloff radius for the OUTER
/// flame of a given exhaust config.
fn outer_extents(config: &ThrusterExhaustConfig) -> (f32, f32, f32) {
    match config.geometry {
        ThrusterExhaustShape::Cone => (
            config.exhaust_radius,
            config.exhaust_radius,
            config.exhaust_radius,
        ),
        ThrusterExhaustShape::Rect => {
            let (hx, hz) = (config.width * 0.5, config.height * 0.5);
            (hx, hz, hx.max(hz))
        }
    }
}

/// Authorable placement + shape of a thruster's exhaust cone. `offset`/`rotation`
/// position and orient the cone relative to the section (the cone is built along
/// +Y); `shape` is the shader/size config. The default matches the procedural
/// thruster's historical hardcoded placement, so an omitted `exhaust` still
/// yields the same cone.
#[derive(Clone, Debug, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(default))]
pub struct ThrusterExhaust {
    /// Position of the exhaust cone relative to the section.
    pub offset: Vec3,
    /// Orientation of the exhaust cone (built along +Y) relative to the section.
    pub rotation: Quat,
    /// The exhaust shader/size config.
    pub shape: ThrusterExhaustConfig,
}

impl Default for ThrusterExhaust {
    fn default() -> Self {
        Self {
            offset: Vec3::new(0.0, 0.0, 0.3),
            rotation: Quat::from_rotation_x(std::f32::consts::FRAC_PI_2),
            shape: ThrusterExhaustConfig::default(),
        }
    }
}

#[derive(Component, Clone, Debug, Deref, DerefMut, Reflect)]
struct ThrusterSectionRenderMesh(#[reflect(ignore)] Option<AssetRef<WorldAsset>>);

/// The thrust magnitude produced by this thruster section, in IMPULSE PER
/// FIXED TICK, not force. [`thruster_impulse_system`] hands it straight to
/// `apply_linear_impulse_at_point` with no `dt` factor, so a section at full
/// input delivers `magnitude / mass` of delta-v every `FixedUpdate` tick and
/// the ship's acceleration is `magnitude / (mass * dt)`.
///
/// DO NOT change this to a force without rescaling every authored magnitude by
/// `1 / dt`: the unit is a deliberate authoring convention, and every consumer
/// already converts from it. The autopilot divides by `dt` where it needs an
/// acceleration (`flight/autopilot.rs`, brake authority) and keeps the raw
/// value where it needs a delta-v (the tail-burn estimate); the flight
/// balancers use it only for ratios, which any uniform scale leaves alone.
///
/// The cost of the convention is that acceleration is tied to the fixed-tick
/// RATE: at half the ticks per second, every ship accelerates half as hard.
/// Nothing configures `Time<Fixed>` today, so it stays at Bevy's 64 Hz, and
/// the authored magnitudes are tuned against that. Retune them, or move to
/// `apply_force_at_point`, before touching the rate.
#[derive(Component, Clone, Debug, Deref, DerefMut, Reflect)]
pub struct ThrusterSectionMagnitude(pub f32);

/// The thuster input. Will be a value between 0.0 and 1.0, where 0.0 means no thrust and 1.0 means
/// full thrust.
#[derive(Component, Clone, Debug, Deref, DerefMut, Reflect)]
pub struct ThrusterSectionInput(pub f32);

/// A plugin that enables the ThrusterSection component and its related systems.
#[derive(Default)]
pub struct ThrusterSectionPlugin {
    /// Whether to spawn the section's render mesh and exhaust (false on headless servers).
    pub render: bool,
}

impl Plugin for ThrusterSectionPlugin {
    fn build(&self, app: &mut App) {
        debug!("ThrusterSectionPlugin: build");

        app.add_plugins(MaterialPlugin::<ThrusterPlumeMaterial>::default());
        // Outside the render gate: `thruster_shader_update_system` runs on a
        // headless server too, and a system whose resource is missing does not
        // run at all.
        app.init_resource::<ExhaustMaterials>();

        if self.render {
            app.init_resource::<ExhaustMeshes>();
            app.init_resource::<PlaceholderArt>();
            app.add_observer(insert_thruster_section_render);
            app.add_observer(insert_thruster_shader);
        }

        app.add_systems(
            Update,
            thruster_shader_update_system.in_set(super::SpaceshipSectionSystems),
        );
        app.add_systems(
            FixedUpdate,
            thruster_impulse_system.in_set(super::SpaceshipSectionSystems),
        );
    }
}

// `pub(crate)` so the flight-control tests can register the real impulse
// system and cover the whole intent -> thrust -> velocity pipeline.
pub(crate) fn thruster_impulse_system(
    q_thruster: Query<
        (
            &Transform,
            &ChildOf,
            &ThrusterSectionMagnitude,
            &ThrusterSectionInput,
        ),
        (With<ThrusterSectionMarker>, Without<SectionInactiveMarker>),
    >,
    mut q_root: Query<Forces>,
) {
    for (transform, &ChildOf(root), magnitude, input) in &q_thruster {
        let Ok(mut force) = q_root.get_mut(root) else {
            error!(
                "thruster_impulse_system: entity {:?} not found in q_root",
                root
            );
            continue;
        };

        // NOTE: compose the burn from the ROOT's raw physics pose and the
        // engine's local mount (sections are direct children of the root), the
        // exact math of the balancer's lever arms in flight/. In FixedUpdate,
        // `GlobalTransform` and the child collider's avian pose are at least
        // one tick stale - at speed that trails the hull by `v * dt` and a
        // COM-centered engine torques the ship it must not touch.
        let position = force.position().0;
        let rotation = *force.rotation();
        let thrust_direction = rotation
            .mul_vec3(transform.rotation.mul_vec3(Vec3::NEG_Z))
            .normalize();
        // RAW IMPULSE, no `dt`: the magnitude IS a per-tick impulse, see
        // `ThrusterSectionMagnitude`. Multiplying by `dt` here without
        // rescaling every authored magnitude would cut all thrust by 64x.
        let thrust_impulse = thrust_direction * **magnitude * input.clamp(0.0, 1.0);
        let world_point = position + rotation.mul_vec3(transform.translation);

        force.apply_linear_impulse_at_point(thrust_impulse, world_point);
    }
}

/// One cone of a thruster's exhaust: what it is, and which throttle step it is
/// currently drawn at.
///
/// The spec rides the entity rather than being re-derived from the config,
/// because a plume outlives its section - a severed drive keeps drawing its
/// cone while it tumbles away - and the swap must still know what to swap to.
#[derive(Component, Clone, Copy, Debug)]
struct ThrusterExhaustPlume {
    /// The shape and colour half of this cone's material.
    spec: PlumeSpec,
    /// Which of the [`EXHAUST_PLUME_BUCKETS`] steps it is drawn at.
    bucket: usize,
}

#[expect(
    clippy::type_complexity,
    reason = "the plume query carries its own material and parent handles"
)]
fn thruster_shader_update_system(
    time: Res<Time>,
    q_thruster: Query<
        (&ThrusterSectionInput, Has<SectionInactiveMarker>),
        With<ThrusterSectionMarker>,
    >,
    // PLUME is graded HERE rather than in a system of its own, because two
    // systems choosing one cone's material would fight over it every frame. The
    // effect owns the curve; this owns the swap.
    q_plume: Query<&DamageLevel, With<DamagePlume>>,
    mut q_render: Query<(
        &mut ThrusterExhaustPlume,
        &mut MeshMaterial3d<ThrusterPlumeMaterial>,
        &ChildOf,
    )>,
    q_child: Query<&ChildOf>,
    mut registry: ResMut<ExhaustMaterials>,
    mut materials: ResMut<Assets<ThrusterPlumeMaterial>>,
) {
    let seconds = time.elapsed_secs();
    for (mut plume, mut material, &ChildOf(parent)) in &mut q_render {
        let section = find_thruster_section(parent, &q_thruster, &q_child);

        let wanted = match section {
            // No section above it at all: the drive died and its art detached
            // with the wreck, which still draws this cone. A dead drive is not
            // burning, so the plume goes out rather than keeping the last value
            // it was given. An inactive one is the same.
            None => 0.0,
            Some((_, _, true)) => 0.0,
            // A drive that wears no plume effect, or is not hurt enough to show
            // one, scales by exactly 1.0 - so this line is what an undamaged
            // thruster always did.
            Some((section, input, false)) => {
                let damage = q_plume
                    .get(section)
                    .map_or(1.0, |level| plume_scale(level.0, seconds));
                *input * damage
            }
        };

        // SWAP, never write. Nothing is ever stored into a built material, so a
        // drive cannot change a neighbour's plume and - the point - a throttle
        // that moves every frame costs a component write rather than a material
        // re-extract, re-upload and bind-group rebuild. A hundred guided
        // torpedoes all burning at once therefore share
        // [`EXHAUST_PLUME_BUCKETS`] materials rather than owning two hundred.
        let bucket = plume_bucket(wanted);
        if bucket == plume.bucket {
            continue;
        }
        material.0 = registry.material(&plume.spec, bucket, &mut materials);
        plume.bucket = bucket;
    }
}

fn find_thruster_section(
    parent: Entity,
    q_thruster: &Query<
        (&ThrusterSectionInput, Has<SectionInactiveMarker>),
        With<ThrusterSectionMarker>,
    >,
    q_child: &Query<&ChildOf>,
) -> Option<(Entity, ThrusterSectionInput, bool)> {
    let mut parent = parent;
    loop {
        if let Ok((input, inactive)) = q_thruster.get(parent) {
            // The ENTITY comes back too: the plume effect hangs on the section,
            // not on the cone, so a caller that only got the input would have
            // to walk the same chain again to find it.
            return Some((parent, input.clone(), inactive));
        }

        let Ok(ChildOf(grandparent)) = q_child.get(parent) else {
            return None;
        };

        parent = *grandparent;
    }
}

/// Marks the render-mesh child spawned for a thruster section, so the render
/// observer can find and style it (including the exhaust). Present only when
/// rendering is enabled.
#[derive(Component, Clone, Debug, Reflect)]
pub struct ThrusterSectionRenderMarker;

fn insert_thruster_section_render(
    add: On<Add, ThrusterSectionMarker>,
    mut commands: Commands,
    placeholder: Res<PlaceholderArt>,
    asset_server: Res<AssetServer>,
    q_thruster: Query<
        (
            &ThrusterSectionRenderMesh,
            &SectionRenderMeshTransform,
            &ThrusterSectionExhaust,
            Has<ThrusterSectionRenderMarker>,
        ),
        With<ThrusterSectionMarker>,
    >,
) {
    let entity = add.entity;
    trace!("insert_thruster_section_render: entity {:?}", entity);

    let Ok((render_mesh, render_mesh_transform, exhaust, has_render)) = q_thruster.get(entity)
    else {
        error!(
            "insert_thruster_section_render: entity {:?} not found in q_thruster",
            entity
        );
        return;
    };

    if has_render {
        trace!(
            "insert_thruster_section_render: entity {:?} already has render, skipping",
            entity
        );
        return;
    }

    commands.entity(entity).insert(ThrusterSectionRenderMarker);
    match &**render_mesh {
        Some(asset_ref) => {
            let scene = asset_ref.resolve(&asset_server);
            // Authored render-mesh transform (identity when unset), on the mesh
            // CHILD so it moves the art only.
            let transform = render_mesh_transform
                .map(RenderMeshTransform::to_transform)
                .unwrap_or_default();
            commands.entity(entity).insert((children![(
                Name::new("Thruster Section Body"),
                transform,
                SectionRenderOf(entity),
                WorldAssetRoot(scene),
            ),],));
        }
        None => {
            commands.entity(entity).insert((children![
                (
                    Name::new("Thruster Section Body (A)"),
                    SectionRenderOf(entity),
                    Mesh3d(placeholder.barrel.clone()),
                    MeshMaterial3d(placeholder.structure_material.clone()),
                    Transform::from_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2))
                        .with_translation(Vec3::new(0.0, 0.0, -0.3)),
                ),
                (
                    Name::new("Thruster Section Body (B)"),
                    SectionRenderOf(entity),
                    Mesh3d(placeholder.nozzle.clone()),
                    MeshMaterial3d(placeholder.nozzle_material.clone()),
                    Transform::from_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
                ),
            ],));
        }
    }

    // Spawn the exhaust cone for EVERY thruster (custom mesh or procedural),
    // using the authored placement/shape or the default. `insert_thruster_shader`
    // turns the ThrusterExhaustConfig into the cone mesh + glow material.
    let exhaust = exhaust.0.clone().unwrap_or_default();
    commands.entity(entity).with_children(|parent| {
        parent.spawn((
            Name::new("Thruster Exhaust"),
            SectionRenderOf(entity),
            exhaust.shape,
            Transform::from_translation(exhaust.offset).with_rotation(exhaust.rotation),
        ));
    });
}

fn insert_thruster_shader(
    add: On<Add, ThrusterExhaustConfig>,
    mut commands: Commands,
    q_config: Query<&ThrusterExhaustConfig>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut flames: ResMut<ExhaustMeshes>,
    mut glows: ResMut<ExhaustMaterials>,
    mut exhaust_materials: ResMut<Assets<ThrusterPlumeMaterial>>,
) {
    let entity = add.entity;
    trace!("insert_thruster_shader: entity {:?}", entity);

    let Ok(config) = q_config.get(entity) else {
        error!(
            "insert_thruster_shader: entity {:?} not found in q_config",
            entity
        );
        return;
    };

    // Outer flame: extents from the shape (radius for cone, width/height for rect).
    let (ohx, ohz, omax_r) = outer_extents(config);
    let mesh = flames.mesh(
        config.geometry,
        ohx,
        ohz,
        config.exhaust_height,
        &mut meshes,
    );
    let spec = PlumeSpec {
        emissive: config.emissive_color,
        radius: omax_r,
        height: config.exhaust_max,
    };

    // Inner core: a cone keeps its own inner radius; a rect scales the outer
    // extents by the inner/outer radius ratio so it stays proportional.
    let (ihx, ihz, imax_r) = match config.geometry {
        ThrusterExhaustShape::Cone => (
            config.exhaust_inner_radius,
            config.exhaust_inner_radius,
            config.exhaust_inner_radius,
        ),
        ThrusterExhaustShape::Rect => {
            let ratio = if config.exhaust_radius > 1e-6 {
                config.exhaust_inner_radius / config.exhaust_radius
            } else {
                0.25
            };
            (ohx * ratio, ohz * ratio, omax_r * ratio)
        }
    };
    let inner_mesh = flames.mesh(
        config.geometry,
        ihx,
        ihz,
        config.exhaust_inner_height,
        &mut meshes,
    );
    let inner_spec = PlumeSpec {
        emissive: config.emissive_inner_color,
        radius: imax_r,
        height: config.exhaust_inner_max,
    };

    // Both cones open at the IDLE bucket, which is what a freshly spawned
    // thruster is: `thruster_shader_update_system` swaps them up on the first
    // frame the drive burns, so a fleet parked at zero shares one material a
    // shape.
    let glow = glows.material(&spec, 0, &mut exhaust_materials);
    let inner_glow = glows.material(&inner_spec, 0, &mut exhaust_materials);
    commands.entity(entity).insert((
        ThrusterExhaustPlume { spec, bucket: 0 },
        Mesh3d(mesh),
        MeshMaterial3d(glow),
        children![(
            ThrusterExhaustPlume {
                spec: inner_spec,
                bucket: 0,
            },
            Transform::from_xyz(0.0, 1e-4, 0.0),
            Mesh3d(inner_mesh),
            MeshMaterial3d(inner_glow),
        )],
    ));
}

/// A drive's exhaust flame: a standard PBR material stretched and lit by the
/// throttle.
///
/// Named because it is the biggest material population a torpedo fight puts in
/// the frame, and the census cannot see it through
/// `MeshMaterial3d<StandardMaterial>` any more than it can see the crack
/// buckets - so it is counted by type, the same way `SectionCracksMaterial` is.
pub type ThrusterPlumeMaterial = ExtendedMaterial<StandardMaterial, ThrusterExhaustMaterial>;

/// Material extension driving the thruster exhaust glow shader.
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone, Default)]
pub struct ThrusterExhaustMaterial {
    /// Current throttle (0.0..1.0), fed to the shader to scale the glow.
    #[uniform(100)]
    pub thruster_input: f32,
    /// Base radius of the exhaust cone, passed to the shader.
    #[uniform(100)]
    pub thruster_exhaust_radius: f32,
    /// Length of the exhaust cone along +Y, passed to the shader.
    #[uniform(100)]
    pub thruster_exhaust_height: f32,
    #[cfg(target_arch = "wasm32")]
    #[uniform(100)]
    _webgl2_padding_16b: u32,
}

impl ThrusterExhaustMaterial {
    /// Builder setter for the exhaust cone base radius.
    pub fn with_exhaust_radius(mut self, radius: f32) -> Self {
        self.thruster_exhaust_radius = radius;
        self
    }

    /// Builder setter for the exhaust cone height.
    pub fn with_exhaust_height(mut self, height: f32) -> Self {
        self.thruster_exhaust_height = height;
        self
    }
}

impl MaterialExtension for ThrusterExhaustMaterial {
    fn vertex_shader() -> ShaderRef {
        "shaders/thruster_exhaust.wgsl".into()
    }

    fn fragment_shader() -> ShaderRef {
        "shaders/thruster_exhaust.wgsl".into()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn rect_exhaust_builder_is_a_unit_square_pyramid() {
        let mesh = rect_exhaust_builder(4).build();
        let Some(bevy::render::mesh::VertexAttributeValues::Float32x3(positions)) =
            mesh.attribute(Mesh::ATTRIBUTE_POSITION)
        else {
            panic!("rect exhaust mesh has no Float32x3 positions");
        };
        assert!(!positions.is_empty());
        for &[x, y, z] in positions {
            assert!((-1.001..=1.001).contains(&x), "x {x} out of range");
            assert!((-0.001..=1.001).contains(&y), "y {y} out of range");
            assert!((-1.001..=1.001).contains(&z), "z {z} out of range");
        }
        // A square base reaches the corners (|x|~1 AND |z|~1) - a round cone's
        // xz traces the unit circle and never has both near 1, so this pins the
        // shape as square, not cone.
        assert!(
            positions
                .iter()
                .any(|&[x, _, z]| x.abs() > 0.99 && z.abs() > 0.99),
            "expected a square corner near (+-1, _, +-1)"
        );
    }

    #[test]
    fn spawns_thruster_with_default_config() {
        // Arrange
        let mut app = App::new();
        let id = app
            .world_mut()
            .spawn(thruster_section(ThrusterSectionConfig::default()))
            .id();

        // Act
        app.update();

        // Assert
        assert!(app.world().get::<ThrusterSectionMarker>(id).is_some());
    }

    #[test]
    fn spawns_thruster_with_custom_scene() {
        // Arrange
        let mut app = App::new();
        let custom_scene = Handle::<WorldAsset>::default();
        let config = ThrusterSectionConfig {
            render_mesh: Some(custom_scene.clone().into()),
            ..default()
        };
        let id = app.world_mut().spawn(thruster_section(config)).id();

        // Act
        app.update();

        // Assert
        assert!(app.world().get::<ThrusterSectionMarker>(id).is_some());
        let render_mesh = app.world().get::<ThrusterSectionRenderMesh>(id).unwrap();
        assert!(render_mesh.0.is_some());
        assert_eq!(
            render_mesh.0.as_ref().unwrap(),
            &AssetRef::from(custom_scene)
        );
    }

    /// Two nozzles of one size share one flame mesh, and a third of another
    /// size does not. The FRAME is what this buys: a distinct mesh is prepared
    /// and bound every frame however many nozzles wear it, so a fleet whose
    /// drives all came off one prototype must introduce two meshes and not two
    /// per drive.
    #[test]
    fn nozzles_of_one_size_share_one_flame_mesh() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, bevy::asset::AssetPlugin::default()));
        app.init_asset::<Mesh>();
        app.init_resource::<ExhaustMeshes>();

        let mut built = |height: f32| {
            let world = app.world_mut();
            let mut flames = world.remove_resource::<ExhaustMeshes>().expect("cache");
            let mut meshes = world.resource_mut::<Assets<Mesh>>();
            let handle = flames.mesh(ThrusterExhaustShape::Cone, 0.4, 0.4, height, &mut meshes);
            let total = meshes.len();
            world.insert_resource(flames);
            (handle, total)
        };

        let (first, after_first) = built(0.1);
        let (second, after_second) = built(0.1);
        let (third, after_third) = built(0.2);

        assert_eq!(first, second, "one size must mint one mesh");
        assert_eq!(after_first, 1, "the first nozzle builds its mesh");
        assert_eq!(after_second, 1, "the second nozzle builds nothing");
        assert_ne!(first, third, "a different size is a different mesh");
        assert_eq!(after_third, 2, "and it is built once");
    }

    /// The two ends must be EXACT: `system_thrust_and_plume` asserts the shader
    /// uniform reaches 1.0 on a held full burn and returns to 0.0 on release,
    /// both to 1e-6, and it reads whatever material the swap left behind.
    #[test]
    fn the_throttle_buckets_end_at_a_dead_plume_and_a_full_one() {
        assert_eq!(plume_bucket(0.0), 0);
        assert_eq!(plume_bucket(1.0), EXHAUST_PLUME_BUCKETS - 1);
        assert_eq!(bucket_input(0), 0.0);
        assert_eq!(bucket_input(EXHAUST_PLUME_BUCKETS - 1), 1.0);
        // Nearest, not floor: a plume is never drawn a whole step colder than
        // the drive is burning.
        assert_eq!(plume_bucket(0.999), EXHAUST_PLUME_BUCKETS - 1);
        assert_eq!(plume_bucket(2.0), EXHAUST_PLUME_BUCKETS - 1);
        assert_eq!(plume_bucket(-1.0), 0);
    }

    type PlumeMaterials = Assets<ThrusterPlumeMaterial>;

    /// A hundred drives burning together must cost the frame BUCKETS, not
    /// drives. A material written every frame is re-extracted, re-uploaded and
    /// has its bind group rebuilt that frame, and a guided torpedo's throttle
    /// genuinely moves every frame - so the read-before-write guard that covers
    /// a parked fleet covers a salvo not at all.
    #[test]
    fn drives_burning_at_one_throttle_share_one_plume_material() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, bevy::asset::AssetPlugin::default()));
        app.init_asset::<ThrusterPlumeMaterial>();
        app.init_resource::<ExhaustMaterials>();
        app.add_systems(Update, thruster_shader_update_system);

        let spec = PlumeSpec {
            emissive: LinearRgba::rgb(0.0, 10.0, 10.0),
            radius: 0.4,
            height: 1.0,
        };
        let idle = app
            .world_mut()
            .resource_scope(|world, mut glows: Mut<ExhaustMaterials>| {
                let mut materials = world.resource_mut::<PlumeMaterials>();
                glows.material(&spec, 0, &mut materials)
            });
        assert_eq!(app.world().resource::<PlumeMaterials>().len(), 1);

        let mut cones = Vec::new();
        for _ in 0..100 {
            let world = app.world_mut();
            let mut section = world.spawn(thruster_section(ThrusterSectionConfig::default()));
            section.insert(ThrusterSectionInput(1.0));
            let section = section.id();
            cones.push(
                world
                    .spawn((
                        ThrusterExhaustPlume { spec, bucket: 0 },
                        MeshMaterial3d(idle.clone()),
                        ChildOf(section),
                    ))
                    .id(),
            );
        }

        app.update();

        assert_eq!(
            app.world().resource::<PlumeMaterials>().len(),
            2,
            "a hundred drives at full throttle add ONE material to the idle one"
        );
        let handles: Vec<_> = cones
            .iter()
            .map(|&e| {
                app.world()
                    .get::<MeshMaterial3d<ThrusterPlumeMaterial>>(e)
                    .expect("plume material")
                    .0
                    .clone()
            })
            .collect();
        assert!(
            handles.iter().all(|h| *h == handles[0]),
            "every drive at one throttle draws through one handle"
        );
        assert_ne!(
            handles[0], idle,
            "and it is not the idle one they opened at"
        );
        let full = app
            .world()
            .resource::<PlumeMaterials>()
            .get(&handles[0])
            .expect("material");
        assert_eq!(full.extension.thruster_input, 1.0);
        assert_eq!(full.base.emissive, spec.emissive);
        assert_eq!(full.extension.thruster_exhaust_radius, spec.radius);
        assert_eq!(full.extension.thruster_exhaust_height, spec.height);
    }

    /// The registry is bounded by SHAPES times buckets, not by drives: a
    /// throttle sweep over the whole range mints at most one material a step,
    /// and a second nozzle shape is its own set.
    #[test]
    fn a_throttle_sweep_mints_at_most_one_material_a_bucket() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, bevy::asset::AssetPlugin::default()));
        app.init_asset::<ThrusterPlumeMaterial>();
        app.init_resource::<ExhaustMaterials>();

        let cone = PlumeSpec {
            emissive: LinearRgba::rgb(0.0, 10.0, 10.0),
            radius: 0.4,
            height: 1.0,
        };
        let core = PlumeSpec {
            emissive: LinearRgba::rgb(0.0, 0.0, 10.0),
            radius: 0.1,
            height: 0.5,
        };

        // Two hundred samples across the range, twice over, through both shapes.
        for spec in [&cone, &core, &cone] {
            for step in 0..200 {
                let input = step as f32 / 199.0;
                app.world_mut()
                    .resource_scope(|world, mut glows: Mut<ExhaustMaterials>| {
                        let mut materials = world.resource_mut::<PlumeMaterials>();
                        glows.material(spec, plume_bucket(input), &mut materials)
                    });
            }
        }

        assert_eq!(
            app.world().resource::<ExhaustMaterials>().shapes(),
            2,
            "two nozzle shapes, however many samples"
        );
        assert_eq!(
            app.world().resource::<PlumeMaterials>().len(),
            2 * EXHAUST_PLUME_BUCKETS,
            "and each shape tops out at one material a bucket"
        );
    }
}
