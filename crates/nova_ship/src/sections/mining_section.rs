//! The mining section: a retractable emitter whose beam cuts ore out of its
//! ship's travel-locked rock.
//!
//! This module owns the mount: its config, the door and tip sequence, and
//! whether the emitter is fully deployed. The beam itself belongs to
//! `nova_scenario`'s mining plugin, which reads [`MiningEmitter::is_deployed`],
//! aims down the emitter's local -Z face ([`mining_emitter_face`]) and carves.
//!
//! While its ship holds [`MiningHeld`], an emitter parts its
//! [`SectionAnimationCue::StowDoors`] doors and, once they are fully open,
//! extends its [`SectionAnimationCue::StowLift`] tip. On release the tip
//! retracts first and the doors shut after it. A press or a release during
//! travel reverses from where the parts are. The emitter is deployed only
//! while its ship holds the key and both tracks are at progress 0.
//!
//! The art rests deployed, which is what an editor preview shows. A live
//! emitter starts stowed by snap, as a retractable turret does.

use bevy::prelude::*;
use nova_events::units::prelude::*;
use nova_gameplay::{asset_ref::AssetRef, prelude::*};

use crate::prelude::*;

/// The `mining_section` spawners, its face helper, config, fault, markers
/// and `MiningSectionPlugin` with `MiningSectionSystems`.
pub mod prelude {
    pub use super::{
        mining_emitter_face, mining_section, preview_mining_section, MiningEmitter,
        MiningSectionConfig, MiningSectionConfigFault, MiningSectionConfigHelper,
        MiningSectionMarker, MiningSectionPlugin, MiningSectionSystems,
    };
}

/// Authorable config for a mining section. Every field is required.
#[derive(Clone, Debug, PartialEq, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct MiningSectionConfig {
    /// The emitter's scene: the casing, the `stow_lid_` doors the
    /// `StowDoors` track slides and the `beam_tip` the `StowLift` track
    /// retracts.
    #[reflect(ignore)]
    pub render_mesh: AssetRef<WorldAsset>,
    /// Optional transform applied to the render mesh only (never the collider
    /// the emitter face is measured from).
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub render_mesh_transform: Option<RenderMeshTransform>,
    /// How far the beam reaches from the emitter face along its local -Z.
    pub reach: Meters,
    /// Game seconds between pulses of a fully deployed emitter.
    pub pulse_interval_seconds: f32,
    /// The radius of one pulse's carve sphere, in cells of the rock's own
    /// field.
    pub carve_radius_cells: f32,
}

impl MiningSectionConfig {
    /// Refuse a config no emitter can run on.
    ///
    /// The content lint and the live spawn both call this, so a stat the lint
    /// refuses never reaches a pulse.
    ///
    /// # Errors
    ///
    /// The first field at fault: `reach`, `pulse_interval_seconds` or
    /// `carve_radius_cells` that is not finite and positive.
    pub fn validate(&self) -> Result<(), MiningSectionConfigFault> {
        for (field, value) in [
            ("reach", self.reach.get()),
            ("pulse_interval_seconds", self.pulse_interval_seconds),
            ("carve_radius_cells", self.carve_radius_cells),
        ] {
            if !value.is_finite() || value <= 0.0 {
                return Err(MiningSectionConfigFault { field, value });
            }
        }
        Ok(())
    }
}

/// The field a [`MiningSectionConfig`] is refused for, from
/// [`MiningSectionConfig::validate`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MiningSectionConfigFault {
    /// The config field at fault, named as it is authored.
    pub field: &'static str,
    /// What that field held, in its authored units.
    pub value: f32,
}

impl std::fmt::Display for MiningSectionConfigFault {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "mining section {} must be finite and positive, got {}",
            self.field, self.value
        )
    }
}

/// Tags a live or previewed mining section.
#[derive(Component, Clone, Copy, Debug, Default, Reflect)]
#[reflect(Component)]
pub struct MiningSectionMarker;

/// The emitter's full config, kept on the section entity. Read-only via
/// `Deref`.
#[derive(Component, Clone, Debug, Deref, Reflect)]
pub struct MiningSectionConfigHelper(MiningSectionConfig);

/// The deploy state of a live emitter. Editor previews do not carry it, so
/// they never retract and never pulse.
#[derive(Component, Clone, Copy, Debug, Default, Reflect)]
#[reflect(Component)]
pub struct MiningEmitter {
    deployed: bool,
}

impl MiningEmitter {
    /// True while the ship holds the key and the doors and tip are fully out.
    /// The one gate the beam reads.
    pub fn is_deployed(&self) -> bool {
        self.deployed
    }
}

#[derive(Component, Clone, Debug, Deref, Reflect)]
struct MiningSectionRenderMesh(#[reflect(ignore)] AssetRef<WorldAsset>);

/// A live mining section.
///
/// # Panics
///
/// On a config [`MiningSectionConfig::validate`] refuses. The content lint
/// reports the same fault first.
pub fn mining_section(config: MiningSectionConfig) -> impl Bundle {
    trace!("mining_section: config {:?}", config);

    if let Err(fault) = config.validate() {
        panic!("mining_section: {fault}");
    }
    (preview_mining_section(config), MiningEmitter::default())
}

/// The render-only half of an emitter, for editor views.
pub fn preview_mining_section(config: MiningSectionConfig) -> impl Bundle {
    trace!("preview_mining_section: config {:?}", config);

    (
        MiningSectionMarker,
        SectionClass::Mining,
        SectionRenderMeshTransform(config.render_mesh_transform),
        MiningSectionRenderMesh(config.render_mesh.clone()),
        MiningSectionConfigHelper(config),
    )
}

/// The centre and outward normal of the emitter face of a section at
/// `position` with `rotation`: the collider's local -Z face. The same
/// unscaled-pose rule as the cargo intake face applies.
pub fn mining_emitter_face(
    position: Vec3,
    rotation: Quat,
    collider: SectionCollider,
) -> (Vec3, Vec3) {
    let normal = rotation * Vec3::NEG_Z;
    (position + normal * collider.aabb_half_extents().z, normal)
}

/// Land every new live emitter stowed, before its scene resolves.
fn arm_mining_emitters(mut q_emitters: Query<&mut SectionAnimations, Added<MiningEmitter>>) {
    for mut animations in &mut q_emitters {
        animations.snap_cue(SectionAnimationCue::StowLift, 1.0);
        animations.snap_cue(SectionAnimationCue::StowDoors, 1.0);
    }
}

/// Steer `cue` toward `target` only when it is headed elsewhere, so a settled
/// rig stays out of change detection.
fn steer(animations: &mut Mut<SectionAnimations>, cue: SectionAnimationCue, target: f32) {
    if animations.cue_target(cue).is_some_and(|was| was != target) {
        animations.set_cue(cue, target);
    }
}

/// Sequence every live emitter from its ship's key: doors then tip out while
/// held, tip then doors in once released. An absent track reads as already
/// at its end, as the turret stow machine reads one.
fn drive_mining_emitters(
    mut q_emitters: Query<
        (&ChildOf, &mut MiningEmitter, &mut SectionAnimations),
        (With<MiningSectionMarker>, Without<SectionInactiveMarker>),
    >,
    q_held: Query<(), With<MiningHeld>>,
) {
    for (&ChildOf(ship), mut emitter, mut animations) in &mut q_emitters {
        let held = q_held.contains(ship);
        let doors = animations.cue_progress(SectionAnimationCue::StowDoors);
        let tip = animations.cue_progress(SectionAnimationCue::StowLift);
        let doors_open = doors.is_none_or(|doors| doors == 0.0);
        let tip_out = tip.is_none_or(|tip| tip == 0.0);
        if held {
            steer(&mut animations, SectionAnimationCue::StowDoors, 0.0);
            if doors_open {
                steer(&mut animations, SectionAnimationCue::StowLift, 0.0);
            }
        } else {
            steer(&mut animations, SectionAnimationCue::StowLift, 1.0);
            if tip.is_none_or(|tip| tip == 1.0) {
                steer(&mut animations, SectionAnimationCue::StowDoors, 1.0);
            }
        }
        let deployed = held && doors_open && tip_out;
        if emitter.deployed != deployed {
            emitter.deployed = deployed;
        }
    }
}

/// Spawn an emitter's scene.
fn insert_mining_section_render(
    add: On<Add, MiningSectionMarker>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    q_emitters: Query<(&MiningSectionRenderMesh, &SectionRenderMeshTransform)>,
) {
    let entity = add.entity;
    let Ok((render_mesh, render_mesh_transform)) = q_emitters.get(entity) else {
        error!("insert_mining_section_render: entity {entity:?} not found in q_emitters");
        return;
    };
    let transform = render_mesh_transform
        .map(RenderMeshTransform::to_transform)
        .unwrap_or_default();
    commands.entity(entity).insert(children![(
        Name::new("Mining Section Body"),
        transform,
        SectionRenderOf(entity),
        WorldAssetRoot(render_mesh.resolve(&asset_server)),
    )]);
}

/// System set for the emitter sequence, on `Update` before the section
/// animation driver, so a cue steered here moves the same frame. The beam in
/// `nova_scenario` runs after this set and reads the deploy state it wrote.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct MiningSectionSystems;

/// Adds mining emitters: the stowed arm, the door and tip sequence, and (when
/// `render`) the emitter scene.
#[derive(Default)]
pub struct MiningSectionPlugin {
    /// Whether the render-side half is added (false on headless servers).
    pub render: bool,
}

impl Plugin for MiningSectionPlugin {
    fn build(&self, app: &mut App) {
        trace!("MiningSectionPlugin: build");

        app.register_type::<MiningSectionMarker>();
        app.register_type::<MiningEmitter>();

        app.configure_sets(Update, MiningSectionSystems.before(SectionAnimationSystems));
        app.add_systems(
            Update,
            (arm_mining_emitters, drive_mining_emitters)
                .chain()
                .in_set(MiningSectionSystems),
        );

        if self.render {
            app.add_observer(insert_mining_section_render);
        }
    }
}

#[cfg(test)]
mod tests;
