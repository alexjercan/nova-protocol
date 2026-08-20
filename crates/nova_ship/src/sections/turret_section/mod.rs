//! A turret section is a component that can be added to an entity to give it a turret-like
//! behavior.

mod aim;
mod arc;
mod config;
mod firing;
mod render;
mod setup;
#[cfg(test)]
mod test_support;

#[cfg(test)]
pub(crate) use aim::lead_intercept_point;
pub use aim::{
    muzzle_aim_error, muzzle_on_target, update_turret_aim_point, TurretSectionAimSystems,
    CLOSE_ENGAGEMENT_RANGE, HULL_HIT_RADIUS, TURRET_ON_TARGET_RAD,
};
use aim::{sync_turret_joint_rotation, update_turret_target_joints_system};
pub use arc::TurretSectionArc;
use bevy::prelude::*;
use bevy_hanabi::prelude::EffectAsset;
pub use config::{MuzzleConfig, TurretJoint, TurretSectionConfig};
use firing::{resolve_bullet_hit, shoot_spawn_projectile};
use nova_gameplay::prelude::*;
use render::{
    insert_projectile_render, insert_turret_barrel_muzzle_effect, insert_turret_joint_render,
    on_projectile_marker_effect, DefaultProjectileRender,
};
use setup::{apply_turret_config_to_children, insert_turret_section};

use crate::prelude::*;

/// The `turret_section` spawner, the joint, muzzle and barrel components, the aim and target
/// inputs, the loaded bullet and `TurretSectionPlugin`.
pub mod prelude {
    pub use super::{
        muzzle_aim_error, muzzle_on_target, turret_section, LoadedBullet, MuzzleConfig,
        TurretJoint, TurretSectionAimPoint, TurretSectionAimSystems, TurretSectionArc,
        TurretSectionBarrelMuzzleMarker, TurretSectionConfig, TurretSectionConfigHelper,
        TurretSectionInput, TurretSectionMuzzleEntity, TurretSectionPlugin,
        TurretSectionTargetInput, TurretSectionTargetVelocity, CLOSE_ENGAGEMENT_RANGE,
        HULL_HIT_RADIUS, TURRET_ON_TARGET_RAD,
    };
}

/// Helper function to create a turret section entity bundle.
pub fn turret_section(config: TurretSectionConfig) -> impl Bundle {
    trace!("turret_section: config {:?}", config);

    // The loaded-ammo slot, seeded from the authored config. Read before `config`
    // moves into the helper (both fields are `Copy`).
    let loaded = LoadedBullet {
        kind: config.bullet_kind,
        damage: config.bullet_damage,
    };

    (
        TurretSectionMarker,
        SectionClass::Turret,
        loaded,
        TurretSectionTargetInput(None),
        TurretSectionTargetVelocity::default(),
        TurretSectionAimPoint::default(),
        TurretSectionConfigHelper(config),
        TurretSectionInput(false),
    )
}

/// The turret's loaded-ammo "slot": the round it currently fires. Runtime state
/// (seeded from [`TurretSectionConfig::bullet_kind`]/`bullet_damage`), NOT the
/// authored config - a future ship-management / station / scenario action swaps
/// the loaded round by mutating this one small component, and the fire path and
/// ammo readout both read it. The growth seam toward per-type magazines +
/// reload.
#[derive(Component, Clone, Copy, Debug, Reflect)]
pub struct LoadedBullet {
    /// The damage type of the loaded round (stamps `ProjectileDamage.kind`).
    pub kind: DamageType,
    /// The pre-resistance per-hit damage of the loaded round.
    pub damage: f32,
}

/// Input to request the turret to shoot a projectile.
#[derive(Component, Clone, Debug, Deref, DerefMut, Reflect)]
pub struct TurretSectionInput(pub bool);

/// The muzzle marker of the turret section.
#[derive(Component, Clone, Copy, Debug, Reflect)]
pub struct TurretSectionBarrelMuzzleMarker;

/// The target input for the turret section. This is a world-space position that the turret will
/// aim at. If None, the turret will not rotate.
#[derive(Component, Clone, Copy, Debug, Deref, DerefMut, Reflect)]
pub struct TurretSectionTargetInput(pub Option<Vec3>);

/// The world-space velocity of the turret's target, used to lead a moving target
/// (aim where it will be when a bullet arrives). Defaults to zero - a stationary
/// aim point (e.g. the player crosshair) needs no lead. Whoever aims the turret at
/// a moving object (auto-targeting, AI) sets this to the object's velocity.
#[derive(Component, Clone, Copy, Debug, Default, Deref, DerefMut, Reflect)]
pub struct TurretSectionTargetVelocity(pub Vec3);

/// The world-space point the turret is actually aiming its barrel at: the lead
/// intercept of `TurretSectionTargetInput` given `TurretSectionTargetVelocity`,
/// the bullet `muzzle_speed`, and the shooter's own muzzle velocity that the
/// bullet inherits on launch (see `update_turret_aim_point` - the solve runs in
/// the shooter's frame). `None` when there is no target. Read by the yaw/pitch
/// systems to steer, and exposed so tooling (aim gizmos, the HUD lead pip) can
/// show where the turret leads.
#[derive(Component, Clone, Copy, Debug, Default, Deref, DerefMut, Reflect)]
pub struct TurretSectionAimPoint(pub Option<Vec3>);

/// The Turret "parent" entity of the turret component.
///
/// `pub(crate)` so the audio module can key each gun's fire SFX by its turret
/// entity (multiple guns each sound). Not re-exported from the public prelude.
#[derive(Component, Clone, Copy, Debug, Deref, DerefMut, Reflect)]
pub(crate) struct TurretSectionPartOf(pub Entity);

#[derive(Component, Clone, Debug, Deref, DerefMut, Reflect)]
struct BulletProjectileRenderMesh(#[reflect(ignore)] Option<AssetRef<WorldAsset>>);

/// A turret joint entity: the runtime of one [`TurretJoint`] node. Articulated
/// joints (axis Some) also carry a [`SmoothLookRotation`]. Paired with a
/// [`TurretSectionPartOf`] pointing at the turret section root.
#[derive(Component, Clone, Copy, Debug, Reflect)]
struct TurretJointMarker {
    axis: Option<Vec3>,
}

/// The last in-plane heading the aim solver measured for an articulated joint,
/// stored in the joint's own POST-rotation frame so it turns with the joint.
/// Carried only so the solver has a heading left when the muzzle forward lines
/// up with the hinge axis - see `update_turret_target_joints_system`. `None`
/// until the joint's first non-degenerate frame.
#[derive(Component, Clone, Copy, Debug, Default, Reflect)]
struct TurretJointAimHeading(Option<Vec3>);

/// This joint's render mesh (generic; was the per-type `*RenderMesh` zoo).
#[derive(Component, Clone, Debug, Deref, DerefMut, Reflect)]
struct TurretJointRenderMesh(#[reflect(ignore)] Option<AssetRef<WorldAsset>>);

/// The authored transform for this joint's render mesh, snapshotted from
/// [`TurretJoint::render_mesh_transform`] so the render observer can apply it to
/// the mesh child without re-reading the joint tree.
#[derive(Component, Clone, Copy, Debug, Deref, DerefMut, Reflect)]
struct TurretJointRenderMeshTransform(Option<RenderMeshTransform>);

/// The live tuning config carried by a turret section entity. The aim/shoot systems read
/// `muzzle_speed` from it directly every frame; the rotator speeds, pitch limits and fire rate
/// are snapshotted onto child entities when the turret is built, so edits to those are pushed to
/// the children by `apply_turret_config_to_children`. Editing this component (it derefs to
/// [`TurretSectionConfig`]) is the supported way to retune a turret live - see the turret range
/// example's sliders.
#[derive(Component, Clone, Debug, Deref, DerefMut, Reflect)]
pub struct TurretSectionConfigHelper(pub TurretSectionConfig);

/// Per-barrel-muzzle fire cooldown timer, snapshotted from the turret's fire
/// rate when the turret is built. The muzzle system ticks it down and resets
/// it on each shot, gating the barrel's cadence.
///
/// A `Timer` and NOT a `Cooldown` (which the torpedo bay's equivalent uses):
/// the muzzle needs two things `Cooldown` does not have. It reads `elapsed`
/// BEFORE the tick to recover how far past due a shot came within the tick
/// window - that is what makes the bullet stream uniformly spaced at any ship
/// velocity - and `apply_turret_config` calls `set_duration` to retune the fire
/// rate live from the turret range example's sliders.
#[derive(Component, Clone, Debug, Deref, DerefMut, Reflect)]
pub struct TurretSectionBarrelFireState(pub Timer);

#[derive(Component, Clone, Debug, Deref, DerefMut, Reflect)]
struct TurretSectionBarrelMuzzleEffect(#[reflect(ignore)] Option<AssetRef<EffectAsset>>);

/// A turret's authored fire sound, snapshotted from
/// [`TurretSectionConfig::fire_sound`] onto the turret section entity at spawn -
/// the UNRESOLVED [`AssetRef`], exactly like [`TurretSectionBarrelMuzzleEffect`]
/// carries the unresolved muzzle effect. The audio module resolves it (against
/// its own `AssetServer`, only when it actually plays the cue). Authored-or-
/// silent: `None` (the config left `fire_sound` unset) means no fire sound.
///
/// Carrying the `AssetRef` rather than a resolved `Handle` keeps
/// `insert_turret_section` free of an `AssetServer` dependency (it is registered
/// unconditionally, so many headless section rigs spawn turrets through it);
/// resolution lives with the one system that needs it.
///
/// `pub(crate)` so the audio module can read it, keyed by the firing turret via
/// [`TurretSectionPartOf`] - the same seam the fire cue already uses.
#[derive(Component, Clone, Debug, Deref, DerefMut, Reflect)]
pub(crate) struct TurretSectionFireSound(#[reflect(ignore)] pub Option<AssetRef<AudioSource>>);

/// The turret's authored dry-fire click, snapshotted UNRESOLVED from
/// [`TurretSectionConfig::dry_fire_sound`] like [`TurretSectionFireSound`];
/// the audio cue resolves it. `pub(crate)` for the audio module.
#[derive(Component, Clone, Debug, Deref, DerefMut, Reflect)]
pub(crate) struct TurretSectionDryFireSound(#[reflect(ignore)] pub Option<AssetRef<AudioSource>>);

#[derive(Component, Clone, Copy, Debug, Reflect)]
struct TurretSectionBarrelMuzzleEffectMarker;

/// The entity that represents the muzzle of the turret.
#[derive(Component, Clone, Copy, Debug, Deref, DerefMut, Reflect)]
pub struct TurretSectionMuzzleEntity(pub Entity);

/// Every muzzle (fire point) of a turret, in tree DFS order. The section-wide
/// fire/aim path iterates these; [`TurretSectionMuzzleEntity`] stays as the
/// PRIMARY muzzle (the first) for the single-point consumers (lead HUD pip, the
/// aim-point lead solve, AI alignment gate).
#[derive(Component, Clone, Debug, Deref, DerefMut, Reflect)]
pub struct TurretSectionMuzzles(pub Vec<Entity>);

/// A plugin that enables the TurretSection component and its related systems.
#[derive(Default)]
pub struct TurretSectionPlugin {
    /// Whether to spawn the section's render mesh (false on headless servers).
    pub render: bool,
}

impl Plugin for TurretSectionPlugin {
    fn build(&self, app: &mut App) {
        trace!("TurretSectionPlugin: build");

        app.register_type::<TurretSectionArc>();
        app.add_observer(insert_turret_section);
        app.add_observer(resolve_bullet_hit);

        if self.render {
            app.init_resource::<PlaceholderArt>();
            app.add_observer(insert_turret_joint_render);
            app.init_resource::<DefaultProjectileRender>();
            app.add_observer(insert_projectile_render);

            // Hanabi muzzle-flash and projectile-trail effects: run on wasm too
            // now that the web build uses the WebGPU backend.
            app.add_observer(insert_turret_barrel_muzzle_effect);
            app.add_observer(on_projectile_marker_effect);
        }

        app.add_systems(
            Update,
            apply_turret_config_to_children.in_set(super::SpaceshipSectionSystems),
        );

        // NOTE: the WHOLE gun runs on the physics clock - the intercept solve,
        // the hinge demand, the controller that eases onto it, the joint pose
        // it writes, and the round that leaves along that pose. The fire timer
        // accumulates fixed ticks and bullets spawn from the RAW root pose, so
        // shot spacing is exact at any ship velocity; in Update the timer
        // quantized shots to render frames and the muzzle pose was the eased
        // render pose.
        //
        // The aim half joined it because the fire gate is a 0.92 deg cone
        // (`TURRET_ON_TARGET_RAD`) and a barrel whose pose advanced once per
        // FRAME crossed that cone a number of times per SIMULATED second that
        // followed the frame rate - measured on `stress_point_defense` as 0.10
        // of the trigger held at 20 fps against 0.62 at 106 fps, same scene,
        // same seed. Anything that decides whether a barrel BEARS has to sit on
        // the clock the rounds are spent on; the alternative is a battery that
        // is six times stronger on a fast machine.
        //
        // The chain steers `SmoothLookRotation`, so it writes this step's joint
        // targets before the rig eases toward them, and the joint sync writes
        // the pose before the spawner reads it. Declared here rather than in
        // the rig: nova_gameplay owns a generic controller that names no driver,
        // and the driver is this crate's.
        app.configure_sets(
            FixedUpdate,
            TurretSectionAimSystems.before(SmoothLookRotationSystems::Sync),
        );
        app.add_systems(
            FixedUpdate,
            (
                (update_turret_aim_point, update_turret_target_joints_system)
                    .chain()
                    .in_set(TurretSectionAimSystems),
                sync_turret_joint_rotation.after(SmoothLookRotationSystems::Sync),
                shoot_spawn_projectile.after(sync_turret_joint_rotation),
            )
                .in_set(super::SpaceshipSectionSystems),
        );
    }
}

#[cfg(test)]
mod tests {
    use nova_gameplay::transform::prelude::SmoothLookRotationPlugin;

    use super::*;

    /// The aim chain and the rig it steers are both `PostUpdate` writers of the
    /// same components: the chain reads `SmoothLookRotationOutput` and writes
    /// `SmoothLookRotationTarget`, and `Sync` does the reverse. Unordered, the
    /// joints consume the target either this frame or next on a per-build
    /// topological coin flip.
    ///
    /// Asserted through `ambiguity_detection` rather than an observed order,
    /// because bevy's tie-break happens to supply the right order today: a test
    /// that reads the order back passes with and without the edge, which is
    /// exactly how this stayed hidden. Only the two plugins are added, so the
    /// conflicting pairs this can report are theirs.
    #[test]
    fn the_aim_chain_is_ordered_against_the_rig_it_steers() {
        use bevy::ecs::schedule::{LogLevel, ScheduleBuildSettings};

        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            TransformPlugin,
            SmoothLookRotationPlugin,
            TurretSectionPlugin::default(),
        ));
        app.edit_schedule(PostUpdate, |schedule| {
            schedule.set_build_settings(ScheduleBuildSettings {
                ambiguity_detection: LogLevel::Error,
                ..default()
            });
        });

        app.update();
    }
}
