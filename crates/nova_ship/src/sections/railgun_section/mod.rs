//! The railgun section: a SPINAL kinetic lance the hull aims.
//!
//! Every other gun on a ship answers a bearing. This one answers the hull:
//! it fires straight down its own -Z and cannot traverse, so the pilot points
//! the SHIP and the skill is holding that point through the charge. That is
//! the whole reason the family exists as its own [`SectionKind`] arm rather
//! than a turret with a narrow cone - the turret's aim code is about cones and
//! joints, and a lance has neither.
//!
//! The cycle is three authored numbers and no state the player can back out
//! of: a trigger pull COMMITS the shot, [`RailgunSectionConfig::charge_seconds`]
//! runs (with the capacitor bolt walking the bore as the tell), the slug leaves
//! along the hull axis, and the magazine's reload is the long silence after.
//! There is no abort - committing IS the decision, so the charge window is
//! aiming time the pilot has already paid for.
//!
//! The slug is a Pierce round bounded by POWER ALONE
//! ([`RailgunSectionConfig::slug_power`], `layers: u32::MAX`): one shell crosses
//! the target's whole section stack rather than stopping at the first hull.
//! It rides `nova_gameplay::rounds` like any gun round, so it curves in wells,
//! is charged once per layer, and needs no second damage pipeline.
//!
//! An authored [`RailgunSectionConfig::rake_radius`] turns that budget sideways.
//! No shipped craft is deep enough to spend 1800 power along one line, so a
//! lance that only ever cut a bore-width hole threw most of every shot out
//! through the far side. The rake drags a sphere behind the slug's tip and
//! charges what it sweeps out of the SAME budget, which converts the surplus
//! depth into a corridor you can see. Omitting the field is the old behavior
//! exactly.
//!
//! Recoil is real and is applied AT THE MUZZLE, not at the centre of mass:
//! `apply_linear_impulse_at_point` turns a lance mounted off the ship's axis
//! into a lance that spins the ship every time it fires. Where the builder put
//! it is part of what it costs.

use avian3d::prelude::*;
use bevy::prelude::*;
use nova_events::units::prelude::*;
use nova_gameplay::{asset_ref::AssetRef, lifetime::TempEntity, prelude::*};

use super::local_pose_in_root;
use crate::{physics::prelude::rigid_body_point_velocity, prelude::*};

/// The charge/fire path: the commit, the charge clock, the slug and the recoil.
mod firing;
/// Render/audio for the lance: the slug's body and the muzzle flash. Gated by
/// the plugin's `render` flag.
mod render;
/// The wake the slug leaves and the light that rides it. Gated the same way.
mod wake;

pub use firing::RailgunFired;
use firing::*;
pub(crate) use firing::{
    RailgunSectionChargeSound, RailgunSectionFireSound, RailgunSectionReloadSound,
};
use render::*;
use wake::*;

/// The `railgun_section` spawners, its config, marker, input, charge state,
/// fire report, `RailgunSectionPlugin`, and the slug's wake and light.
pub mod prelude {
    pub use super::{
        preview_railgun_section, railgun_section,
        wake::{
            RailgunSlugLight, RailgunWakeEmitter, RailgunWakeLayer, RailgunWakeTuning,
            RAILGUN_SLUG_LIGHT_LUMENS, RAILGUN_SLUG_LIGHT_RANGE,
        },
        RailgunCharge, RailgunFired, RailgunSectionConfig, RailgunSectionConfigHelper,
        RailgunSectionInput, RailgunSectionPlugin, RailgunSectionSystems,
    };
}

/// Authorable config for a spinal railgun section.
#[derive(Clone, Debug, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct RailgunSectionConfig {
    /// The render mesh of the lance, defaults to a cuboid of size 1x1x1.
    #[reflect(ignore)]
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub render_mesh: Option<AssetRef<WorldAsset>>,
    /// Optional transform applied to the lance's render mesh only (never the
    /// collider, never the bore).
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub render_mesh_transform: Option<RenderMeshTransform>,
    /// The bore exit in the section's own frame: where the slug is born, where
    /// the flash plays, and the point the recoil impulse is applied at.
    ///
    /// It must sit ON the muzzle face, because the recoil lever arm is
    /// measured from it - a muzzle authored at the section origin would push a
    /// three-cell gun as if it were a one-cell one.
    pub muzzle_offset: Vec3,
    /// Seconds from the trigger pull to the shot. The charge cannot be
    /// aborted, so this is aiming time, not a hold.
    pub charge_seconds: f32,
    /// Muzzle speed of the slug.
    pub slug_speed: MetersPerSecond,
    /// Damage the slug deals to EVERY layer it crosses. Flat: Pierce is not
    /// scaled by closing speed and does not decay with depth.
    pub slug_damage: f32,
    /// The slug's pierce budget, priced in the MAX health of each layer it
    /// crosses (divided by the closing-speed multiplier). This is the ONLY
    /// bound on how deep one shell goes - the layer count is deliberately
    /// unlimited, so a lance stops when it runs out of thickness to spend and
    /// never because it met an arbitrary layer.
    pub slug_power: f32,
    /// How wide a corridor the slug opens. Omitted is a bore-width
    /// round and exactly the behavior every lance had before the field
    /// existed.
    ///
    /// A rake spends the SAME [`slug_power`](Self::slug_power) budget on width
    /// that it would otherwise have spent on depth it never finds. Only a body
    /// the narrow slug hits DIRECTLY is raked, so this widens what an aligned
    /// shot destroys and never what it hits: a near miss stays a miss, and a
    /// fighter on the centreline still presents almost no material to spend
    /// the budget on.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub rake_radius: Option<Meters>,
    /// Seconds the slug lives before it expires unspent. With no layer cap
    /// this is also what stops a miss travelling forever.
    pub slug_lifetime: f32,
    /// Recoil impulse, applied backwards along the bore AT THE MUZZLE on the
    /// tick the slug leaves. Raw impulse, no `dt` - the same units the
    /// thruster's magnitude carries.
    pub recoil_impulse: f32,
    /// Sound played on the shot, at the muzzle.
    #[reflect(ignore)]
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub fire_sound: Option<AssetRef<AudioSource>>,
    /// The capacitor bank filling: a LOOP held for the whole charge, played at
    /// a rate that rises with [`RailgunCharge::progress`] so the gun sounds
    /// like it is arriving at something.
    ///
    /// A loop and not a one-shot because the charge is authored per hull -
    /// [`charge_seconds`](Self::charge_seconds) is a number a mod may set to
    /// anything - and a fixed-length file would either end early or be cut off.
    #[reflect(ignore)]
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub charge_sound: Option<AssetRef<AudioSource>>,
    /// A shell going back into the breech, played when the magazine returns to
    /// capacity. For a one-shell lance that is the whole of its cadence: the
    /// reload is the silence, and this is the silence ending.
    #[reflect(ignore)]
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub reload_sound: Option<AssetRef<AudioSource>>,
    /// Shells the gun carries. `None` is unlimited - the bare-rig default every
    /// weapon section shares.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub ammo_capacity: Option<u32>,
    /// The reload, which for a one-shell magazine IS the gun's cadence: the
    /// authored delay is the silence between shots.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub reload: Option<SectionReloadConfig>,
}

impl Default for RailgunSectionConfig {
    /// A bare test-rig lance: unlimited shells, an instant charge and no
    /// recoil, so a headless rig that only wants "does it fire" gets one shot
    /// per trigger pull and nothing else to arrange.
    fn default() -> Self {
        Self {
            render_mesh: None,
            render_mesh_transform: None,
            muzzle_offset: Vec3::NEG_Z * 0.5,
            charge_seconds: 0.0,
            slug_speed: MetersPerSecond(10_000.0),
            slug_damage: 50.0,
            slug_power: PIERCE_BASE_POWER,
            rake_radius: None,
            slug_lifetime: 5.0,
            recoil_impulse: 0.0,
            fire_sound: None,
            charge_sound: None,
            reload_sound: None,
            ammo_capacity: None,
            reload: None,
        }
    }
}

/// The lance's full config, kept on the section entity. Read-only via `Deref`,
/// so the AI's commit rule derives its envelope from the same numbers the gun
/// actually fires with, exactly as it does for the turret and the bay.
#[derive(Component, Clone, Debug, Deref, Reflect)]
pub struct RailgunSectionConfigHelper(RailgunSectionConfig);

/// Input to commit a shot. A rise starts the charge; the charge then runs to
/// completion whatever the trigger does after it, because the commit is the
/// decision. A HELD trigger therefore re-commits the moment the gun is ready
/// again, which is the gun cycling at its own cadence rather than a second
/// mechanic.
#[derive(Component, Clone, Debug, Default, Deref, DerefMut, Reflect)]
pub struct RailgunSectionInput(pub bool);

/// Where the gun is in its cycle.
///
/// The reload is NOT here: it is [`SectionAmmo`] plus [`SectionReload`] on the
/// same entity, the machinery every weapon section shares, so a one-shell
/// magazine with a long delay gives the cadence and the diegetic gauge at once.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Reflect)]
pub enum RailgunCharge {
    /// Loaded, uncommitted, waiting for a trigger.
    #[default]
    Ready,
    /// Committed. `elapsed` seconds of the authored charge have run; at
    /// `charge_seconds` the slug leaves whether or not the trigger is still
    /// held and whether or not the hull is still pointed anywhere useful.
    Charging {
        /// Seconds of charge run so far.
        elapsed: f32,
    },
}

impl RailgunCharge {
    /// Charge progress in `0.0..=1.0`, which is also what the [`Charge`] cue
    /// is driven with.
    ///
    /// [`Charge`]: SectionAnimationCue::Charge
    pub fn progress(self, charge_seconds: f32) -> f32 {
        match self {
            Self::Ready => 0.0,
            Self::Charging { elapsed } => {
                if charge_seconds > 0.0 {
                    (elapsed / charge_seconds).clamp(0.0, 1.0)
                } else {
                    1.0
                }
            }
        }
    }
}

/// A live railgun section: the lance, plus the trigger and the charge clock
/// that make it a weapon.
pub fn railgun_section(config: RailgunSectionConfig) -> impl Bundle {
    trace!("railgun_section: config {:?}", config);

    (
        preview_railgun_section(config),
        RailgunSectionInput(false),
        RailgunCharge::Ready,
    )
}

/// The render-only half of a lance: what an editor view needs to LOOK like a
/// railgun, with no trigger and no charge clock, so a preview can never fire.
pub fn preview_railgun_section(config: RailgunSectionConfig) -> impl Bundle {
    trace!("preview_railgun_section: config {:?}", config);

    (
        RailgunSectionMarker,
        SectionClass::Railgun,
        SectionRenderMeshTransform(config.render_mesh_transform),
        RailgunSectionRenderMesh(config.render_mesh.clone()),
        // Snapshotted onto the section, like the turret's: the audio layer
        // resolves it from the gun that fired without reading the config.
        RailgunSectionFireSound(config.fire_sound.clone()),
        RailgunSectionChargeSound(config.charge_sound.clone()),
        RailgunSectionReloadSound(config.reload_sound.clone()),
        RailgunSectionConfigHelper(config),
    )
}

#[derive(Component, Clone, Debug, Deref, DerefMut, Reflect)]
struct RailgunSectionRenderMesh(#[reflect(ignore)] Option<AssetRef<WorldAsset>>);

/// The lance's own systems: the charge clock and the shot. Ordered inside
/// [`SpaceshipSectionSystems`] so the recoil impulse lands in the same fixed
/// tick the slug is born in.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct RailgunSectionSystems;

/// Adds the spinal railgun: its charge/fire path, and (when `render`) the slug
/// body and muzzle flash.
#[derive(Default)]
pub struct RailgunSectionPlugin {
    /// Whether the render-side half is added (false on headless servers).
    pub render: bool,
}

impl Plugin for RailgunSectionPlugin {
    fn build(&self, app: &mut App) {
        trace!("RailgunSectionPlugin: build");

        app.register_type::<RailgunSectionInput>();
        app.register_type::<RailgunCharge>();
        app.add_observer(insert_railgun_section);
        app.add_systems(
            FixedUpdate,
            charge_and_fire_railgun
                .in_set(RailgunSectionSystems)
                .in_set(SpaceshipSectionSystems),
        );

        if self.render {
            app.init_resource::<PlaceholderArt>();
            app.init_resource::<RailgunSlugArt>();
            app.init_resource::<RailgunWakeArt>();
            app.init_resource::<SoftDot>();
            app.add_observer(insert_railgun_section_render);
            app.add_observer(insert_railgun_slug_render);
            app.add_observer(on_railgun_fired_flash);
            app.add_observer(on_railgun_fired_kick);
            app.register_type::<RailgunChargeGlowMarker>();
            app.register_type::<RailgunSlugLight>();
            app.add_systems(Update, (drive_railgun_charge_glow, follow_railgun_wakes));
            // After hanabi's own tick, which is what lets the wake say how
            // many particles this frame spawns instead of the asset's rate.
            app.add_systems(
                PostUpdate,
                count_railgun_wake_spawns.after(bevy_hanabi::EffectSystems::TickSpawners),
            );
        }
    }
}

#[cfg(test)]
mod tests;
