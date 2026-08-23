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
/// Scripted launches: an authored one-shot order that fires a bay and commits
/// the projectile with no controller involved (scenario-timer emplacements).
mod scripted;

use bay::*;
use projectile::*;
use render::*;
use scripted::*;

/// The `torpedo_section` spawner, the bay, arming and steering state, the guidance and target
/// components, the blast, and `TorpedoSectionPlugin`.
pub mod prelude {
    pub use super::{
        scripted::ScriptedTorpedoOrder, torpedo_section, TorpedoArming, TorpedoBlast,
        TorpedoControllerMarker, TorpedoGuidance, TorpedoSectionConfig, TorpedoSectionConfigHelper,
        TorpedoSectionInput, TorpedoSectionPartOf, TorpedoSectionPlugin,
        TorpedoSectionSpawnerFireState, TorpedoSectionSpawnerMarker, TorpedoShotDownMarker,
        TorpedoSteering, TorpedoTargetChosen, TorpedoTargetEntity, TorpedoTargetPosition,
        TorpedoType, TorpedoTypeConfig, TorpedoWeave,
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
    /// Linear damping (drag) on the torpedo body. The thrust cap alone gates only
    /// the along-nose speed, so repeated turns against a moving target "pump"
    /// total speed up sideways; drag gives a real terminal velocity regardless of
    /// thrust direction and relaxes the velocity toward wherever the nose points,
    /// so the flight path follows the guidance command.
    pub linear_damping: f32,
    /// Blast radius on detonation, in units. The proximity fuze fires near the
    /// target's skin, and blast damage falls off linearly to zero at this radius.
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
    /// Hit points on EACH of the projectile's two collider sections
    /// (controller and thruster); either one reaching zero shoots the
    /// torpedo down (silently - no blast). The default is set so no SINGLE
    /// PDC round can one-shot a warhead (see [`default_projectile_health`]);
    /// an armored siege torpedo authors far more and PDC fire has to chew
    /// through it across the whole closing window.
    #[cfg_attr(
        feature = "serde",
        serde(
            default = "default_projectile_health",
            skip_serializing_if = "is_default_projectile_health"
        )
    )]
    pub projectile_health: f32,
    /// The ordnance this bay launches - see [`TorpedoTypeConfig`]. The bay is
    /// the tube; this is what comes out of it.
    #[cfg_attr(feature = "serde", serde(default))]
    pub torpedo_type: TorpedoTypeConfig,
    /// Magazine size in torpedoes. `None` launches without limit (the pre-ammo
    /// behavior); `Some(n)` gives the bay a [`SectionAmmo`] of `n` torpedoes
    /// that depletes one per launch and blocks firing once empty.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub ammo_capacity: Option<u32>,
    /// Idle batch reload for the bay. Each successful launch resets its delay;
    /// each completed delay restores the authored amount until full. Requires
    /// `ammo_capacity`; an unlimited bay never reloads.
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
            linear_damping: 0.8,
            blast_radius: 30.0,
            // Matches the shipped standard torpedo: 750 over a 30 u linear
            // falloff, so an un-authored bay hits like the catalog's rather
            // than the pre-rework 100.
            blast_damage: 750.0,
            blast_effect: None,
            launch_effect: None,
            launch_sound: None,
            detonation_sound: None,
            projectile_health: default_projectile_health(),
            torpedo_type: TorpedoTypeConfig::default(),
            ammo_capacity: None,
            reload: None,
        }
    }
}

/// What a bay LAUNCHES, as opposed to the tube that launches it: the torpedo
/// type. Two bays can share every other number - blast, cruise, magazine, the
/// warhead itself - and still load different ordnance.
///
/// A type is defined by **how it flies**, not by how hard it hits (owner
/// direction: the types deal the same blast). What that buys is measured, not
/// assumed - against one stock PDC across the shipped 150 u point-defense
/// envelope, the weaving type costs ~370 rounds to stop and is only killed
/// ~40 u out, where the straight type costs ~120 and dies ~115 u out. Straight
/// ordnance is what point defense is good at, which is exactly why it is the
/// right thing to fire at something that cannot shoot back.
///
/// **What evasion does NOT cost is time.** A corkscrew is a longer path - by
/// ~1.5% as the real body flies it, not the ideal `1 / cos(angle)` - and
/// [`thrust_headroom`](super::thrust_headroom) gates thrust on the ALONG-NOSE
/// speed, so a torpedo flying with its nose tilted off its own velocity keeps
/// its engine lit and settles at a higher terminal speed. It arrives no later
/// and reaches no less far. See
/// `bay::tests::the_weave_is_a_longer_path_that_costs_no_time_to_target`; a
/// type that trades reach for evasion has to author that on
/// [`TorpedoSectionConfig::projectile_lifetime`].
///
/// Data, not an enum: a mod authors its own type by writing one of these into
/// its bay's config, and gets a name and a colour for it on the same terms the
/// base game does.
#[derive(Clone, Debug, PartialEq, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TorpedoTypeConfig {
    /// Player-facing name of the ordnance ("Lance", "Serpent"). Names the
    /// launched projectile entity, so the log, the event timeline and a probe
    /// snapshot all say WHICH torpedo is in the air.
    pub name: String,
    /// Body colour of the warhead in flight: the one difference a player can
    /// read at a glance in the frame before the flight path has drawn itself.
    ///
    /// It costs ONE material per distinct colour for the whole process, not one
    /// per launch, so authoring a type is free and authoring a colour PER SHOT
    /// would not be - a tint drawn from a random or a timer would put the
    /// frame's asset work back on the size of the salvo.
    pub tint: Color,
    /// Cruise speed cap in units per second. The thruster tapers off as the
    /// torpedo approaches this speed. Without a cap the torpedo accelerates the
    /// whole flight and arrives so fast that its minimum turning circle
    /// (speed / turn rate) is larger than the proximity fuze - it then orbits
    /// the target instead of hitting it. Keep `max_speed / turn rate`
    /// comfortably under the blast trigger radius.
    ///
    /// **On the TYPE and not on the bay, because it is the price of the
    /// weave.** The cap is read along the NOSE
    /// ([`thrust_headroom`](super::thrust_headroom)), so a torpedo holding its
    /// nose off its own velocity never reaches the taper band, never throttles
    /// down, and outruns a straight one on the same number. Owner direction:
    /// an evasive torpedo should BE slower rather than fly just as fast behind
    /// a shorter timer, so the evasive type authors a lower cap and the taper
    /// band it never quite reaches comes down with it.
    pub max_speed: f32,
    /// Peak half-angle (radians) of the terminal weave: how far off the
    /// guidance solution the corkscrew tilts the steering command. `0.0`
    /// disables the weave and the torpedo flies the bare intercept.
    ///
    /// **This is the balance knob.** Measured against one stock PDC over a
    /// 400 u run-in (`point_defense_cost_tests`), the rounds an intercept
    /// costs track the ANGLE and are all but blind to `weave_rate`: 308
    /// straight, ~1180 at 0.30, ~1245 at 0.44, ~1440 at 0.70. Retune the
    /// exchange here.
    ///
    /// 0.44 is where the two prices cross. Below ~0.30 the lead solution is
    /// good enough again as the flight time shortens; above ~0.55 the torpedo
    /// is paying real closing speed for an intercept the defender was losing
    /// anyway.
    pub weave_angle: f32,
    /// How fast (rad/s) the weave tilt direction rotates about the guidance
    /// command - the corkscrew's spin rate. Higher spins a tighter, faster
    /// helix: the lateral acceleration a lead solution fails to predict scales
    /// with `speed * sin(weave_angle) * weave_rate`, while the helix RADIUS
    /// shrinks with it.
    ///
    /// **This is the LOOK knob, not the balance knob.** At a fixed
    /// `weave_angle` of 0.44 the intercept costs ~1245 rounds at every rate
    /// from 0.7 to 2.2 rad/s, because breaking a straight-line lead
    /// extrapolation does not need a fast turn - it needs a sustained one.
    /// What the rate DOES decide is how far the torpedo visibly swings off the
    /// direct line: 23.6 u at 0.7, 11.1 u at 1.4, 6.1 u at 2.2. Ignored
    /// entirely when `weave_angle` is zero.
    pub weave_rate: f32,
}

impl Default for TorpedoTypeConfig {
    /// The evasive type, so an un-authored bay is the one the rest of the
    /// combat model is balanced against (see the field docs for the exchange
    /// the angle buys).
    ///
    /// The cruise cap is 32.0 against the straight type's 35.0, and it is the
    /// evasive type's PRICE - the thing that makes "evasion costs something"
    /// true rather than asserted. Swept against the three standing constraints
    /// (`defend` for the first two, the real body for the third):
    ///
    /// | cap | rounds an intercept costs | killed at | arrival vs straight |
    /// |---|---|---|---|
    /// | 35.0 | 369 (3.18x) | 38.8 u | **1.5% SOONER** |
    /// | 34.0 | 375 (3.23x) | 39.0 u | +1.3% |
    /// | **32.0** | **390 (3.36x)** | **39.9 u** | **+7.5%** |
    /// | 30.0 | 409 (3.53x) | 41.8 u | +14.5% |
    /// | 24.0 | 484 (4.17x) | 47.6 u | - |
    ///
    /// Dying CLOSE is the binding constraint and it erodes monotonically with
    /// the cap, so the value is the smallest cut that buys an arrival gap a
    /// player can feel: 0.68 s over a 300 u run-in, for 1.1 u of kill range.
    /// The intercept cost does not fall as expected - a slower torpedo IS
    /// easier to lead, but the extra seconds it spends in the envelope more
    /// than pay for the defender's better hit rate, so the ratio improves.
    ///
    /// The rate is chosen for the PICTURE, not the exchange. At 2.2 rad/s the
    /// swing is inside the torpedo's own drive plume and the flight path reads
    /// as straight - a weave nobody can see is not evasion. At 0.7 a whole
    /// engagement is barely half a turn, which reads as a torpedo wandering.
    /// 1.4 gives a couple of visible turns across a full-envelope run-in at an
    /// amplitude comfortably inside the proximity fuze. The swing is much
    /// smaller than the naive `speed * sin(angle) / rate` because the body's
    /// linear damping is a first-order lag on the velocity: the nose holds the
    /// commanded cone, but the velocity only partly follows it.
    fn default() -> Self {
        Self {
            name: "Serpent".to_string(),
            // Hazard orange, the Explosive hue the ammo readout already speaks:
            // the assault torpedo reads hot.
            tint: Color::srgb(0.95, 0.45, 0.1),
            max_speed: 32.0,
            weave_angle: 0.44,
            weave_rate: 1.4,
        }
    }
}

/// Serde default for [`TorpedoSectionConfig::projectile_health`]: enough that
/// no ONE PDC round can swat a warhead out of the air.
///
/// Derived, not picked: the shipped PDC authors 4.0 per round, and the Kinetic
/// closing-speed curve tops out at 2x, so the hardest single round any stock
/// gun can land is 8.0. 10 sits above that ceiling, which makes an intercept
/// cost two rounds head-on and three in a station-keeping engagement. That is
/// still ~30 ms of fire from a 100 rounds/s mount, so point defense loses
/// nothing against a salvo - what it loses is the free one-tap that made
/// ordnance feel like paper.
fn default_projectile_health() -> f32 {
    10.0
}

/// Serde skip for [`TorpedoSectionConfig::projectile_health`], so content
/// authored before the field round-trips byte-identical.
#[cfg_attr(not(feature = "serde"), allow(dead_code))]
fn is_default_projectile_health(health: &f32) -> bool {
    *health == default_projectile_health()
}

/// Bundle factory for a torpedo launch bay from its [`TorpedoSectionConfig`],
/// tagged [`TorpedoSectionMarker`]. The bay spawns [`TorpedoProjectileMarker`]
/// entities when fired; those carry the guidance/arming/blast runtime state.
pub fn torpedo_section(config: TorpedoSectionConfig) -> impl Bundle {
    trace!("torpedo_section: config {:?}", config);

    (
        TorpedoSectionMarker,
        SectionClass::Torpedo,
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
/// `torpedo_pn_guidance`, perturbed by `torpedo_terminal_weave`, and consumed by
/// the sync (orientation) and thrust systems. Kept as one source of truth so
/// both read the same command.
#[derive(Component, Debug, Clone, Deref, DerefMut, Reflect)]
pub struct TorpedoSteering(pub Vec3);

/// What a launched torpedo IS: the identity half of its
/// [`TorpedoTypeConfig`], copied onto the projectile at launch. The flight half
/// lives on [`TorpedoWeave`], which needs per-torpedo runtime state anyway.
///
/// Carried by the projectile rather than looked up through its bay: a torpedo
/// outlives the tube that fired it (and the ship the tube was bolted to), and
/// whoever reads it - the body's material, a probe snapshot, a log line - wants
/// the answer without a second hop.
#[derive(Component, Debug, Clone, Reflect)]
pub struct TorpedoType {
    /// The type's player-facing name, e.g. `Lance`.
    pub name: String,
    /// The warhead body's colour in flight.
    pub tint: Color,
}

/// Terminal-weave state on a torpedo projectile: the open-loop corkscrew laid
/// over the guidance command so a point-defense lead solution fires at where
/// the torpedo was GOING to be.
///
/// No awareness of individual incoming rounds is needed, and none is used - the
/// perturbation is open loop. It works because the defender's solution is real:
/// `lead_intercept_point` extrapolates the torpedo's current velocity over the
/// round's flight time, and a torpedo that is turning breaks that extrapolation.
///
/// It also prices itself with no tuning. A corkscrew of half-angle `angle`
/// lengthens the flown path by `1 / cos(angle)`, so an evading torpedo spends
/// proportionally MORE seconds inside the point-defense envelope than a straight
/// one: evade harder, survive each burst better, eat more bursts. The longer
/// path IS the price, so straightening it out is not an optimisation - it is
/// deleting the balance.
#[derive(Component, Debug, Clone, Reflect)]
pub struct TorpedoWeave {
    /// Peak half-angle (radians) the guidance command is tilted by. Zero flies
    /// the bare intercept.
    pub angle: f32,
    /// Rate (rad/s) the tilt direction spins about the guidance command.
    pub rate: f32,
    /// The current tilt direction: a unit vector held perpendicular to the
    /// guidance command. Runtime state, seeded per torpedo.
    pub offset: Vec3,
}

/// Phase stride between consecutive launches from one bay (radians), the golden
/// angle. A salvo whose torpedoes all start their corkscrew at the same roll
/// flies in lockstep and presents ONE weaving target that a defender's turrets
/// can all lead the same way; strided, each torpedo of a salvo is somewhere else
/// on its helix.
const WEAVE_PHASE_STRIDE: f32 = 2.399_963_2;

impl TorpedoWeave {
    /// Weave state for a torpedo launched pointing `forward` (a unit vector),
    /// with the tilt direction seeded `phase` radians around it.
    pub fn new(angle: f32, rate: f32, forward: Vec3, phase: f32) -> Self {
        let seed = forward.any_orthonormal_vector();
        Self {
            angle,
            rate,
            offset: Quat::from_axis_angle(forward, phase) * seed,
        }
    }

    /// The seed phase for a torpedo, strided by [`WEAVE_PHASE_STRIDE`]. Keyed
    /// off the projectile's own entity at launch, so it is deterministic for a
    /// given run rather than random.
    pub fn phase_for(torpedo: Entity) -> f32 {
        // Low bits only: the generation is irrelevant, and the phase wraps
        // anyway - all this has to do is differ between siblings of a salvo.
        (torpedo.to_bits() & 0xffff) as f32 * WEAVE_PHASE_STRIDE
    }
}

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
        trace!("TorpedoSectionPlugin: build");

        app.add_observer(insert_torpedo_section);

        if self.render {
            app.init_resource::<PlaceholderArt>();
            app.add_observer(insert_torpedo_section_render);

            app.add_observer(insert_torpedo_render);
            // FromWorld, not a `Startup` system: the observer takes the shared
            // body as a plain `Res`, and a scripted emplacement can launch
            // before a startup command flush.
            app.init_resource::<DefaultTorpedoRender>();
            app.add_observer(insert_torpedo_controller_render);

            // Hanabi detonation burst: runs on wasm too now that the web build
            // uses the WebGPU backend. The fallback burst lives in a resource so
            // a salvo shares one effect asset instead of minting one per
            // detonation.
            app.init_resource::<DefaultBlastEffect>();
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
        app.register_type::<TorpedoType>();
        app.register_type::<TorpedoWeave>();
        app.add_observer(on_torpedo_body_destroyed);

        // NOTE: the launch chain runs on the FIXED clock - the spawn writes
        // physics state (a new body with position + velocity), so its pose
        // sampling and its fire timing must tick with physics. Guidance,
        // steering sync and thrust levels stay on the render clock deliberately:
        // they are control INPUTS, consumed by the FixedUpdate PD/impulse
        // systems on their own clock.
        app.add_systems(
            FixedUpdate,
            (update_spawner_fire_state, shoot_spawn_projectile)
                .chain()
                .in_set(super::SpaceshipSectionSystems),
        );
        // The FUZE is not a control input, and that is why it is not up there
        // with them. It is a spatial test against a body physics is moving, so
        // it has to be SAMPLED on the clock that moves it: FixedUpdate runs
        // before Update in a frame, so a render-clock fuze looks once per
        // rendered frame at a torpedo that took several physics steps since the
        // last look. It can cross the whole standoff inside that gap - contact
        // first, fuze afterwards, which is a contact dud and is what the
        // crossing range caught on a software rasterizer. Widening the window
        // cannot fix a window nothing samples.
        //
        // AFTER `PhysicsSystems::Last`, like `advance_rounds`, so the test
        // resolves against a world that has finished moving. Reading the fixed
        // clock also makes the swept term in `contact_reach` a constant instead
        // of whatever the frame rate happened to be.
        app.add_systems(
            FixedPostUpdate,
            torpedo_detonate_system
                .after(PhysicsSystems::Last)
                .in_set(super::SpaceshipSectionSystems),
        );
        // Scripted launches: the trigger hold feeds the FixedUpdate spawn
        // above; the commit runs before target tracking so a scripted
        // torpedo starts guiding the same frame it is committed.
        app.register_type::<ScriptedTorpedoOrder>();
        app.add_systems(
            Update,
            (
                hold_scripted_torpedo_trigger,
                commit_scripted_torpedo,
                despawn_shot_down_torpedoes,
                update_target_position,
                update_torpedo_arming,
                torpedo_pn_guidance,
                // Strictly AFTER guidance and BEFORE the sync: the weave is a
                // perturbation of the guidance solution, so it needs the
                // solution to exist, and the nose must be pointed at the
                // perturbed command, not the bare one.
                torpedo_terminal_weave,
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
