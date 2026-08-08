//! The torpedo section: a launch bay ([`TorpedoSectionConfig`] /
//! [`TorpedoSectionMarker`]) and the guided projectiles it fires. The bay reads
//! its fire input and spawns a torpedo entity; the projectile then runs its own
//! lifecycle - [`TorpedoArming`] (time/distance safety), proportional-navigation
//! [`TorpedoGuidance`] / [`TorpedoSteering`] toward the chosen target, and a
//! proximity [`TorpedoBlast`] that detonates and applies area damage.
//!
//! Touch this module to change torpedo behavior; [`TorpedoSectionConfig`] is the
//! authored tuning surface and the `Torpedo` arm of
//! [`SectionKind`] selects it. The private `bay`, `projectile` and `render`
//! submodules hold the launch path, the in-flight systems and the particle/mesh
//! rendering.

use avian3d::prelude::*;
use bevy::prelude::*;
use bevy_hanabi::prelude::*;
use bevy_transform_interpolation::{RotationEasingState, TranslationEasingState};
use nova_gameplay::{lifetime::TempEntity, prelude::*};

use super::local_pose_in_root;
use crate::{physics::prelude::rigid_body_point_velocity, prelude::*};

/// Building the bay, its fire timer, and the launch that spawns a torpedo.
mod bay;
/// In-flight torpedo behavior: target tracking, arming, detonation, and PN
/// guidance (steer / thrust). These systems act on the spawned projectiles, not
/// on the bay that launched them.
mod projectile;
/// Render/particle systems for the bay and the projectile (gated by the plugin's
/// `render` flag).
mod render;

use bay::*;
use projectile::*;
use render::*;

/// The `torpedo_section` spawner, the bay, arming and steering state, the guidance and target
/// components, the blast, and `TorpedoSectionPlugin`.
pub mod prelude {
    pub use super::{
        torpedo_section, TorpedoArming, TorpedoBlast, TorpedoControllerMarker, TorpedoGuidance,
        TorpedoSectionConfig, TorpedoSectionConfigHelper, TorpedoSectionInput,
        TorpedoSectionPartOf, TorpedoSectionPlugin, TorpedoSectionSpawnerFireState,
        TorpedoSectionSpawnerMarker, TorpedoSteering, TorpedoTargetChosen, TorpedoTargetEntity,
        TorpedoTargetPosition,
    };
}

/// Authorable config for a torpedo bay section (the guided-torpedo launcher).
#[derive(Clone, Debug, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TorpedoSectionConfig {
    /// The render mesh of the torpedo bay, defaults to a cuboid of size 1x1x1.
    #[reflect(ignore)]
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub render_mesh: Option<AssetRef<WorldAsset>>,
    /// Optional transform (position + rotation) applied to the torpedo bay's
    /// render mesh only (the section body, not the projectile). None = the mesh
    /// sits at the section origin (unchanged).
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub render_mesh_transform: Option<RenderMeshTransform>,
    /// The render mesh of the launched torpedo projectile.
    #[reflect(ignore)]
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub projectile_render_mesh: Option<AssetRef<WorldAsset>>,
    /// The offset of the spawn point of the projectile relative to the torpedo section.
    pub spawn_offset: Vec3,
    /// The rotation of the spawn point of the projectile relative to the torpedo section.
    pub spawn_rotation: Quat,
    /// The fire rate of the turret in rounds per second.
    pub fire_rate: f32,
    /// The muzzle speed of the turret in units per second.
    pub spawner_speed: f32,
    /// The lifetime of the projectile in seconds.
    pub projectile_lifetime: f32,
    /// Arming delay: minimum seconds after firing before the torpedo may
    /// detonate. Prevents a torpedo fired at a nearby target from blowing up on
    /// (or right after) spawn. Armed when this OR `arm_distance` is reached.
    pub arm_time: f32,
    /// Arming distance: minimum distance from the muzzle the torpedo must travel
    /// before it may detonate, so it clears the firing ship first. Armed when
    /// this OR `arm_time` is reached.
    pub arm_distance: f32,
    /// Proportional-navigation constant (`N`). Higher values turn harder to null
    /// the line-of-sight rate, so the torpedo leads a moving target more
    /// aggressively. Typical PN values are 3-5.
    pub nav_constant: f32,
    /// Cruise speed cap in units per second. The thruster tapers off as the
    /// torpedo approaches this speed. Without a cap the torpedo accelerates the
    /// whole flight and arrives so fast that its minimum turning circle
    /// (speed / turn rate) is larger than the proximity fuze - it then orbits the
    /// target instead of hitting it. Keep `max_speed / turn rate` comfortably
    /// under the blast trigger radius.
    pub max_speed: f32,
    /// Linear damping (drag) on the torpedo body. The thrust cap alone gates only
    /// the along-nose speed, so repeated turns against a moving target "pump"
    /// total speed up sideways; drag gives a real terminal velocity regardless of
    /// thrust direction and relaxes the velocity toward wherever the nose points,
    /// so the flight path follows the guidance command.
    pub linear_damping: f32,
    /// Blast radius on detonation, in units. The proximity fuze fires when the
    /// torpedo is within half this radius of the target, and blast damage falls off
    /// linearly to zero at this radius.
    pub blast_radius: f32,
    /// Peak blast damage at the detonation centre, falling off to zero at
    /// `blast_radius`.
    pub blast_damage: f32,
    /// The explosion effect to play when the torpedo detonates.
    #[reflect(ignore)]
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub blast_effect: Option<AssetRef<EffectAsset>>,
    /// The launch particle burst played at the bay spawner each time a torpedo is
    /// fired. Mirrors the turret's `muzzle_effect`; when `None`, a default
    /// spawn-on-command burst is built in `insert_torpedo_spawner_effect`. A
    /// custom effect must be spawn-on-command and declare the `normal` and
    /// `base_velocity` `Vec3` properties, which `on_torpedo_launch_effect` sets
    /// per shot (unknown properties are ignored by hanabi).
    #[reflect(ignore)]
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub launch_effect: Option<AssetRef<EffectAsset>>,
    /// The sound played when a torpedo leaves the bay. An authorable
    /// [`AssetRef<AudioSource>`] like the meshes/effects: base bays author
    /// `self://sounds/torpedo_launch.wav`, a mod bay can ship its own.
    /// Snapshotted (unresolved) onto the spawner as
    /// `TorpedoSectionLaunchSound`; the audio observer resolves and plays it.
    /// `None` means no launch sound (authored-or-silent).
    #[reflect(ignore)]
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub launch_sound: Option<AssetRef<AudioSource>>,
    /// The sound this torpedo's DETONATION plays (the blast destroying the
    /// projectile fires the destroy observer). Snapshotted onto the projectile
    /// as [`ImpactDestroySounds`] (destroy slot). AUTHORED-OR-SILENT.
    #[reflect(ignore)]
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub detonation_sound: Option<AssetRef<AudioSource>>,
    /// Magazine size in torpedoes. `None` launches without limit (the pre-ammo
    /// behavior); `Some(n)` gives the bay a [`SectionAmmo`] of `n` torpedoes
    /// that depletes one per launch and blocks firing once empty.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub ammo_capacity: Option<u32>,
    /// Auto-reload for the bay. `None` = no reload (a spent bay stays empty).
    /// `Some` attaches a [`SectionReload`] alongside the `SectionAmmo`, so it
    /// only applies when `ammo_capacity` is also `Some`; an unlimited bay never
    /// reloads.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub reload: Option<SectionReloadConfig>,
}

impl Default for TorpedoSectionConfig {
    fn default() -> Self {
        Self {
            render_mesh: None,
            render_mesh_transform: None,
            projectile_render_mesh: None,
            spawn_offset: Vec3::Y * 2.0,
            spawn_rotation: Quat::IDENTITY,
            fire_rate: 1.0,
            spawner_speed: 1.0,
            projectile_lifetime: 100.0,
            arm_time: 0.5,
            arm_distance: 5.0,
            nav_constant: 3.0,
            max_speed: 35.0,
            linear_damping: 0.8,
            blast_radius: 30.0,
            blast_damage: 100.0,
            blast_effect: None,
            launch_effect: None,
            launch_sound: None,
            detonation_sound: None,
            ammo_capacity: None,
            reload: None,
        }
    }
}

/// Bundle factory for a torpedo launch bay from its [`TorpedoSectionConfig`],
/// tagged [`TorpedoSectionMarker`]. The bay spawns [`TorpedoProjectileMarker`]
/// entities when fired; those carry the guidance/arming/blast runtime state.
pub fn torpedo_section(config: TorpedoSectionConfig) -> impl Bundle {
    debug!("torpedo_section: config {:?}", config);

    (
        TorpedoSectionMarker,
        SectionDamageClass::Torpedo,
        TorpedoSectionConfigHelper(config),
        TorpedoSectionInput(false),
    )
}

/// Input to request the turret to shoot a projectile.
#[derive(Component, Clone, Debug, Deref, DerefMut, Reflect)]
pub struct TorpedoSectionInput(pub bool);

/// Marker for the bay's spawner child - the launch point torpedoes fire from,
/// carrying the launch cooldown, effect and sound. Built with the section.
#[derive(Component, Clone, Debug, Reflect)]
pub struct TorpedoSectionSpawnerMarker;

/// Marker for the bay's physical body child (the render/mesh anchor of the
/// launcher on the ship). Built with the section.
#[derive(Component, Clone, Debug, Reflect)]
pub struct TorpedoSectionBodyMarker;

/// Marker for a torpedo projectile's controller child (the steering control
/// section), inserted on the spawned projectile.
#[derive(Component, Clone, Debug, Reflect)]
pub struct TorpedoControllerMarker;

/// Marker for a torpedo projectile's thruster child (the propulsion section),
/// inserted on the spawned projectile.
#[derive(Component, Clone, Debug, Reflect)]
pub struct TorpedoThrusterMarker;

/// Marker for the particle-effect entity spawned at a torpedo's detonation
/// blast, so the render systems can drive/reset it.
#[derive(Component, Clone, Debug, Reflect)]
pub struct TorpedoBlastEffectMarker;

/// The bay's full config, kept on the section entity. Pub (read-only via
/// `Deref`) so the AI input side can derive its launch envelope from the
/// same numbers the bay actually fires with (blast radius), like it reads
/// the turret's config helper for the fire-range gate.
#[derive(Component, Clone, Debug, Deref, Reflect)]
pub struct TorpedoSectionConfigHelper(TorpedoSectionConfig);

/// The bay's launch cooldown, carried on the spawner and seeded from the config
/// fire rate (`1/fire_rate` seconds). The fire system ticks it and triggers it
/// on each launch to gate the bay's cadence.
///
/// A [`Cooldown`], not a `Timer`: a fresh bay must be READY to launch, which is
/// a `Once` timer's exact opposite and used to need a `finish()` at every
/// construction site.
#[derive(Component, Clone, Debug, Deref, DerefMut, Reflect)]
pub struct TorpedoSectionSpawnerFireState(pub Cooldown);

/// Back-pointer from a bay's spawned entities (spawner, body, projectile,
/// blast) to the torpedo SECTION they belong to. Pub so the AI input side
/// can attribute a fresh projectile to the bay that launched it and reset
/// that bay's launch cooldown.
#[derive(Component, Clone, Debug, Deref, Reflect)]
pub struct TorpedoSectionPartOf(pub Entity);

#[derive(Component, Clone, Debug, Deref, DerefMut, Reflect)]
pub(crate) struct TorpedoSectionSpawnerEntity(pub(crate) Entity);

/// Holds the configured launch-effect handle on the spawner entity so
/// `insert_torpedo_spawner_effect` can read it when the spawner is added. `None`
/// means "build the default burst".
#[derive(Component, Clone, Debug, Deref, DerefMut, Reflect)]
struct TorpedoSectionSpawnerEffect(#[reflect(ignore)] Option<AssetRef<EffectAsset>>);

/// The bay's authored launch sound, snapshotted UNRESOLVED from
/// [`TorpedoSectionConfig::launch_sound`] onto the spawner entity (exactly like
/// [`TorpedoSectionSpawnerEffect`] carries the launch effect). The audio module
/// reaches it from a fired projectile via [`TorpedoSectionSpawnerEntity`] and
/// resolves it there. `pub(crate)` for the audio module.
#[derive(Component, Clone, Debug, Deref, DerefMut, Reflect)]
pub(crate) struct TorpedoSectionLaunchSound(#[reflect(ignore)] pub Option<AssetRef<AudioSource>>);

/// Marks the child `ParticleEffect` entity of the spawner, so the launch trigger
/// (`on_torpedo_launch_effect`) can find its `EffectSpawner` and `reset()` it.
#[derive(Component, Clone, Copy, Debug, Reflect)]
struct TorpedoSectionSpawnerEffectMarker;

/// The locked target entity a fired torpedo homes on, set at launch alongside
/// [`TorpedoTargetChosen`] when a lock existed. Guidance tracks this entity's
/// live position; a dumb-fired torpedo carries no such component.
#[derive(Component, Debug, Clone, Deref, DerefMut, Reflect)]
pub struct TorpedoTargetEntity(pub Entity);

/// The torpedo's launch-time targeting decision has been made. Inserted by the
/// input targeting system (player crosshair today, spaceship AI later) the first
/// time it processes a torpedo - together with a [`TorpedoTargetEntity`] when a
/// lock exists, or alone for a dumb-fire shot. Once present, no targeting system
/// assigns this torpedo a (new) target: a torpedo keeps its first target for
/// life (freezing on the last known position if it dies), and a dumb-fired one
/// never acquires anything mid-flight - e.g. bullets fired past it.
#[derive(Component, Debug, Clone, Reflect)]
pub struct TorpedoTargetChosen;

#[derive(Component, Clone, Debug, Deref, DerefMut, Reflect)]
struct TorpedoProjectileRenderMesh(#[reflect(ignore)] Option<AssetRef<WorldAsset>>);

/// The world position a torpedo steers toward: the tracked target's last known
/// location, frozen here once the [`TorpedoTargetEntity`] dies so the torpedo
/// keeps flying to where the target was rather than losing guidance.
#[derive(Component, Debug, Clone, Deref, DerefMut, Reflect)]
pub struct TorpedoTargetPosition(pub Vec3);

/// Guidance/propulsion tuning carried by a torpedo projectile (copied from its
/// `TorpedoSectionConfig` at spawn), so each bay's torpedoes can be tuned.
#[derive(Component, Debug, Clone, Reflect)]
pub struct TorpedoGuidance {
    /// Proportional-navigation gain: how hard the torpedo turns onto the intercept.
    pub nav_constant: f32,
    /// Speed cap for the torpedo, in units per second.
    pub max_speed: f32,
}

/// The unit direction the torpedo currently wants its nose pointed, produced by
/// `torpedo_pn_guidance` and consumed by the sync (orientation) and thrust
/// systems. Kept as one source of truth so both read the same command.
#[derive(Component, Debug, Clone, Deref, DerefMut, Reflect)]
pub struct TorpedoSteering(pub Vec3);

/// Blast parameters carried by a torpedo projectile (copied from its
/// `TorpedoSectionConfig` at spawn): the proximity-fuze / damage `radius` and the
/// peak `damage` at the detonation centre.
#[derive(Component, Debug, Clone, Reflect)]
pub struct TorpedoBlast {
    /// Proximity-fuze and area-of-effect radius, in world units.
    pub radius: f32,
    /// Peak damage dealt at the detonation centre.
    pub damage: f32,
}

/// Arming state of a torpedo projectile. A torpedo cannot detonate until it is
/// armed; it arms once it has either lived for `min_time` seconds or traveled
/// `min_distance` from its `origin` (the muzzle). This stops a torpedo fired at
/// a nearby target from self-detonating on spawn. Once armed it stays armed.
#[derive(Component, Debug, Clone, Reflect)]
pub struct TorpedoArming {
    min_time: f32,
    min_distance: f32,
    origin: Vec3,
    elapsed: f32,
    armed: bool,
}

impl TorpedoArming {
    /// Create arming state for a torpedo spawned at `origin`.
    pub fn new(min_time: f32, min_distance: f32, origin: Vec3) -> Self {
        Self {
            min_time,
            min_distance,
            origin,
            elapsed: 0.0,
            armed: false,
        }
    }

    /// Whether the torpedo is armed and allowed to detonate.
    pub fn is_armed(&self) -> bool {
        self.armed
    }

    /// Advance the arming state by `dt` seconds given the torpedo's current
    /// position, latching `armed` once the time or distance threshold is met.
    /// Returns the (possibly updated) armed state.
    fn tick(&mut self, dt: f32, position: Vec3) -> bool {
        if self.armed {
            return true;
        }
        self.elapsed += dt;
        let traveled = position.distance(self.origin);
        if self.elapsed >= self.min_time || traveled >= self.min_distance {
            self.armed = true;
        }
        self.armed
    }
}

/// Registers the torpedo bay and projectile systems (spawn, arm, guide,
/// detonate). Added by [`SpaceshipSectionPlugin`].
#[derive(Default)]
pub struct TorpedoSectionPlugin {
    /// Whether the render/particle systems for the bay and projectile are added.
    pub render: bool,
}

impl Plugin for TorpedoSectionPlugin {
    fn build(&self, app: &mut App) {
        debug!("TorpedoSectionPlugin: build");

        app.add_observer(insert_torpedo_section);

        if self.render {
            app.add_observer(insert_torpedo_section_render);

            app.add_observer(insert_torpedo_render);
            app.add_observer(insert_torpedo_controller_render);

            // Expanding-sphere blast-radius visual: a plain mesh + material, so unlike
            // the hanabi particle burst below it also renders on wasm.
            app.add_observer(insert_blast_radius_visual);
            app.add_systems(Update, animate_blast_radius_visual);

            // Hanabi detonation burst: runs on wasm too now that the web build
            // uses the WebGPU backend.
            app.add_observer(insert_particle_effect);

            // Launch burst at the bay: build the effect on the spawner, fire it
            // whenever a torpedo projectile is spawned.
            app.add_observer(insert_torpedo_spawner_effect);
            app.add_observer(on_torpedo_launch_effect);
        }

        // NOTE: a torpedo whose body is shot dead must die as a whole -
        // without this the collider-less root keeps flying, armed, and still
        // detonates.
        app.register_type::<TorpedoShotDownMarker>();
        app.add_observer(on_torpedo_body_destroyed);

        // NOTE: the launch chain runs on the FIXED clock - the spawn writes
        // physics state (a new body with position + velocity), so its pose
        // sampling and its fire timing must tick with physics. Everything below
        // it stays on the render clock deliberately - guidance, steering
        // sync and thrust levels are control INPUTS (consumed by the
        // FixedUpdate PD/impulse systems on their own clock), and the
        // fuze/arming reads are gameplay thresholds, not force writers.
        app.add_systems(
            FixedUpdate,
            (update_spawner_fire_state, shoot_spawn_projectile)
                .chain()
                .in_set(super::SpaceshipSectionSystems),
        );
        app.add_systems(
            Update,
            (
                despawn_shot_down_torpedoes,
                update_target_position,
                update_torpedo_arming,
                torpedo_detonate_system,
                torpedo_pn_guidance,
                torpedo_sync_system,
                torpedo_thrust_system,
            )
                .chain()
                .in_set(super::SpaceshipSectionSystems),
        );
    }
}

/// A torpedo whose body was shot dead, awaiting removal.
///
/// The kill is deliberately TWO-STEP: the observer that detects the dead
/// body section only inserts this marker (inserting on a live entity is
/// always safe), and `despawn_shot_down_torpedoes` removes the torpedo on
/// the next schedule pass. Despawning directly inside the observer raced
/// the integrity pipeline: its already-queued commands for the dying
/// section (e.g. `IntegrityDisabledMarker`) then hit a despawned entity and
/// panicked mid collision-event flush (live-game crash, 20260710).
/// `torpedo_detonate_system` excludes marked roots, so the warhead cannot
/// fire in the gap.
#[derive(Component, Debug, Clone, Reflect)]
pub struct TorpedoShotDownMarker;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn torpedo_is_unarmed_on_spawn() {
        // A freshly spawned torpedo (no time elapsed, no distance travelled) must
        // not be armed, so it cannot detonate on the muzzle.
        let arming = TorpedoArming::new(0.5, 5.0, Vec3::ZERO);
        assert!(!arming.is_armed());
    }

    #[test]
    fn torpedo_arms_after_min_time_even_without_moving() {
        // Point-blank shot: the target sits on the muzzle so the torpedo never
        // travels far, but the time threshold must still arm it eventually.
        let mut arming = TorpedoArming::new(0.5, 5.0, Vec3::ZERO);
        assert!(!arming.tick(0.4, Vec3::ZERO)); // below min_time, still at origin
        assert!(arming.tick(0.2, Vec3::ZERO)); // 0.6s total >= min_time
        assert!(arming.is_armed());
    }

    #[test]
    fn torpedo_arms_after_min_distance_before_min_time() {
        // A fast torpedo clears the muzzle before the time threshold; distance
        // arms it first.
        let mut arming = TorpedoArming::new(10.0, 5.0, Vec3::ZERO);
        assert!(!arming.tick(0.1, Vec3::new(4.0, 0.0, 0.0))); // under both
        assert!(arming.tick(0.1, Vec3::new(6.0, 0.0, 0.0))); // travelled >= 5.0
        assert!(arming.is_armed());
    }

    #[test]
    fn torpedo_stays_armed_once_armed() {
        // Arming latches: coming back inside the arm distance does not disarm it.
        let mut arming = TorpedoArming::new(10.0, 5.0, Vec3::ZERO);
        assert!(arming.tick(0.0, Vec3::new(6.0, 0.0, 0.0))); // armed via distance
        assert!(arming.tick(0.0, Vec3::ZERO)); // back at origin, still armed
        assert!(arming.is_armed());
    }
}
