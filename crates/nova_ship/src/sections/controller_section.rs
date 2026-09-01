//! A section of a spaceship that can control its rotation using a PD controller.
//!
//! A hull may carry several controllers; they share one attitude loop rather
//! than each running their own, and the share is what turns their summed torque
//! into one hull-wide ceiling (see [`update_controller_stack_tuning`]).

use avian3d::prelude::*;
use bevy::{ecs::entity::EntityHashMap, platform::collections::HashSet, prelude::*};
use nova_gameplay::prelude::{
    AssetRef, ControllerSectionMarker, SectionClass, SectionInactiveMarker, SectionMarker,
};

use crate::prelude::{
    structural_arm, AttitudeEnvelope, PDController, PDControllerInput, PDControllerOutput,
    PDControllerSystems, PDControllerTarget, PlaceholderArt, RenderMeshTransform, SectionCollider,
    SectionRenderMeshTransform, SectionRenderOf,
};

/// The controller-section spawners, its config, authored tuning and rotation input, and the
/// flight verbs it withholds.
pub mod prelude {
    pub use super::{
        controller_section, preview_controller_section, ControllerSectionConfig,
        ControllerSectionPlugin, ControllerSectionRenderMarker, ControllerSectionRotationInput,
        ControllerSectionSystems, ControllerSectionTuning, FlightVerb, WithheldVerbs,
    };
}

/// Configuration for a controller section.
#[derive(Clone, Debug, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ControllerSectionConfig {
    /// Approximate time the hull trails a continuously moving steering command, in seconds.
    pub steering_lag: f32,
    /// How hard this computer's reaction wheels twist the hull, in torque units.
    ///
    /// Controllers ADD: two computers are twice the torque, with no cap and no
    /// stacking curve, because the hull's own structural ceiling already caps
    /// the result. What the ship gets out of it is
    /// [`AttitudeEnvelope`] - torque over the hull's inertia, or the load limit
    /// over its arm, whichever gives up first.
    pub max_torque: f32,
    /// The render mesh of the hull section, defaults to a cuboid of size 1x1x1.
    #[reflect(ignore)]
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub render_mesh: Option<AssetRef<WorldAsset>>,
    /// Optional transform (position + rotation) applied to the controller's
    /// render mesh only. None = the mesh sits at the section origin (unchanged).
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub render_mesh_transform: Option<RenderMeshTransform>,
    /// The radar/lock and weapons-safety cues this computer plays, as
    /// authorable [`AssetRef<AudioSource>`]s like the render mesh: the
    /// controller IS the ship's computer (it grants the Lock capability), so its
    /// feedback ticks are its own authorable voice. Snapshotted (unresolved)
    /// into
    /// `ControllerSectionSounds`; the audio cues resolve the PLAYER ship's
    /// controller's refs. AUTHORED-OR-SILENT: `None` plays nothing; base
    /// controllers author all of them via gen_content.
    ///
    /// Lock acquired (once per radar gesture).
    #[reflect(ignore)]
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub lock_on_sound: Option<AssetRef<AudioSource>>,
    /// Lock cleared (tap-clear).
    #[reflect(ignore)]
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub lock_off_sound: Option<AssetRef<AudioSource>>,
    /// Radar hold denied (no Lock capability).
    #[reflect(ignore)]
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub radar_deny_sound: Option<AssetRef<AudioSource>>,
    /// Held radar gesture re-designated to a new target.
    #[reflect(ignore)]
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub radar_retarget_sound: Option<AssetRef<AudioSource>>,
    /// Weapons safety re-engaged (hot -> cold edge).
    #[reflect(ignore)]
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub safety_on_sound: Option<AssetRef<AudioSource>>,
    /// A hostile has this ship in its combat lock - the threat alarm, on the
    /// rising edge of "somebody is aiming at me". The computer OWNS it because
    /// knowing you are locked is a sensor capability, not a property of the
    /// hull: a ship built without a controller flies blind and gets no warning.
    #[reflect(ignore)]
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub warn_lock_sound: Option<AssetRef<AudioSource>>,
    /// A magazine ran dry - the GAUGE, inside the cockpit, as distinct from the
    /// gun's own dead-trigger click out on the mount
    /// ([`TurretSectionConfig::dry_fire_sound`]). The two fire on the same edge
    /// and are meant to be heard as one event from two places; the gun's is
    /// per-turret, this one is per-SHIP.
    ///
    /// [`TurretSectionConfig::dry_fire_sound`]: crate::sections::turret_section::TurretSectionConfig::dry_fire_sound
    #[reflect(ignore)]
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub ammo_dry_sound: Option<AssetRef<AudioSource>>,
    /// RCS fine-adjust LOOP: plays continuously while this controller is burning
    /// the RCS primitive - whether the player is holding SHIFT or the autopilot
    /// is trimming an ORBIT / settling a STOP. Unlike the five one-shot cues
    /// above this is a sustained loop, resolved and volume-
    /// tracked by the audio module (one loop per distinct handle), exactly like a
    /// thruster's `loop_sound`. AUTHORED-OR-SILENT: `None` plays nothing.
    #[reflect(ignore)]
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub rcs_loop_sound: Option<AssetRef<AudioSource>>,
}

/// The controller's authored feedback sounds, snapshotted UNRESOLVED from
/// [`ControllerSectionConfig`] by the [`controller_section`] bundle (one
/// component for the whole set - they share the same consumers). The audio
/// module reads the PLAYER ship's controller and resolves per cue.
/// `pub(crate)` for the audio module.
#[derive(Component, Clone, Debug, Default, Reflect)]
pub(crate) struct ControllerSectionSounds {
    #[reflect(ignore)]
    pub lock_on: Option<AssetRef<AudioSource>>,
    #[reflect(ignore)]
    pub lock_off: Option<AssetRef<AudioSource>>,
    #[reflect(ignore)]
    pub radar_deny: Option<AssetRef<AudioSource>>,
    #[reflect(ignore)]
    pub radar_retarget: Option<AssetRef<AudioSource>>,
    #[reflect(ignore)]
    pub safety_on: Option<AssetRef<AudioSource>>,
    #[reflect(ignore)]
    pub warn_lock: Option<AssetRef<AudioSource>>,
    #[reflect(ignore)]
    pub ammo_dry: Option<AssetRef<AudioSource>>,
    #[reflect(ignore)]
    pub rcs_loop: Option<AssetRef<AudioSource>>,
}

impl ControllerSectionSounds {
    fn from_config(config: &ControllerSectionConfig) -> Self {
        Self {
            lock_on: config.lock_on_sound.clone(),
            lock_off: config.lock_off_sound.clone(),
            radar_deny: config.radar_deny_sound.clone(),
            radar_retarget: config.radar_retarget_sound.clone(),
            safety_on: config.safety_on_sound.clone(),
            warn_lock: config.warn_lock_sound.clone(),
            ammo_dry: config.ammo_dry_sound.clone(),
            rcs_loop: config.rcs_loop_sound.clone(),
        }
    }
}

impl ControllerSectionConfig {
    /// Whether the authored lag can produce finite internal controller gains.
    pub fn has_valid_steering_lag(&self) -> bool {
        steering_lag_is_valid(self.steering_lag)
    }
}

impl Default for ControllerSectionConfig {
    fn default() -> Self {
        Self {
            steering_lag: 0.5,
            max_torque: DEFAULT_MAX_TORQUE,
            render_mesh: None,
            render_mesh_transform: None,
            lock_on_sound: None,
            lock_off_sound: None,
            radar_deny_sound: None,
            radar_retarget_sound: None,
            safety_on_sound: None,
            warn_lock_sound: None,
            ammo_dry_sound: None,
            rcs_loop_sound: None,
        }
    }
}

#[derive(Component, Clone, Debug, Deref, DerefMut, Reflect)]
struct ControllerSectionRenderMesh(#[reflect(ignore)] Option<AssetRef<WorldAsset>>);

/// Helper function to create a controller section entity bundle.
pub fn controller_section(config: ControllerSectionConfig) -> impl Bundle {
    trace!("controller_section: config {:?}", config);

    let frequency = frequency_from_steering_lag(config.steering_lag);
    let tuning = ControllerSectionTuning {
        steering_lag: config.steering_lag,
        max_torque: config.max_torque,
    };
    let sounds = ControllerSectionSounds::from_config(&config);
    (
        preview_controller_section(config),
        tuning,
        PDController {
            frequency,
            damping_ratio: INTERNAL_DAMPING_RATIO,
            // Seeded at zero and filled in by `update_controller_stack_tuning`
            // on the first fixed tick: the ceiling is a property of the HULL,
            // and a section bundle cannot know one.
            max_angular_acceleration: 0.0,
        },
        ControllerSectionRotationInput::default(),
        sounds,
    )
}

/// The AUTHORED steering tuning of a controller section, kept apart from the live
/// [`PDController`] that [`update_controller_stack_tuning`] derives from it.
/// The live component is a share of a ship-wide budget and changes whenever a
/// sibling controller is added or dies, so the authored numbers must survive
/// somewhere to re-derive from.
#[derive(Component, Clone, Copy, Debug, PartialEq, Reflect)]
pub struct ControllerSectionTuning {
    /// Approximate time the hull trails a continuously moving steering command, in seconds.
    pub steering_lag: f32,
    /// How hard this computer twists the hull, in torque units. See
    /// [`ControllerSectionConfig::max_torque`].
    pub max_torque: f32,
}

/// The torque one shipped flight computer carries.
///
/// PROVISIONAL, and a floor rather than an estimate: it was pinned by putting
/// the structure/torque crossover at a 10 u hull, and the largest hull in the
/// game is 2.93 u, so the inertia at that radius is an extrapolation and every
/// measured hull comes out heavier than the curve it was read off. Nothing that
/// ships can settle it, and nothing that ships can see it either - all four
/// reference hulls are structure-bound with 12x to 115x of headroom.
const DEFAULT_MAX_TORQUE: f32 = 1501.0;

/// The hidden response profile. At the shipped 0.5 second lag this derives the
/// former frequency 4 / damping 4 gains exactly.
const INTERNAL_DAMPING_RATIO: f32 = 4.0;

fn steering_lag_is_valid(steering_lag: f32) -> bool {
    let frequency = 2.0 / steering_lag;
    let proportional_gain = (6.0 * frequency).powi(2) * 0.25;
    let damping_gain = 4.5 * frequency * INTERNAL_DAMPING_RATIO;
    steering_lag > 0.0
        && steering_lag.is_finite()
        && proportional_gain.is_finite()
        && damping_gain.is_finite()
}

fn frequency_from_steering_lag(steering_lag: f32) -> f32 {
    if steering_lag_is_valid(steering_lag) {
        2.0 / steering_lag
    } else {
        0.0
    }
}

/// Ceiling on a stack's precision gain - how much earlier than a single
/// computer the stack starts arresting the turn. Costs command-tracking lag
/// one for one (the hull trails a moving command by `rate / slope`), so it
/// stays small: a stacked hull should read as deliberate, not as detached from
/// the helm.
///
/// A constant rather than an authored field on purpose: a curve a mod could
/// raise is a curve a mod could flatten. (There is a mechanical reason too: the
/// section layer sits under the flight layer and must not read
/// `FlightSettings`, where the rest of the flight-feel tunables live.)
const STACK_PRECISION_LIMIT: f32 = 1.5;

/// The precision curve: `limit - (limit - 1) / n`, worth 1.0 at `n = 1` and
/// approaching `limit` from below.
///
/// Chosen for the ASYMPTOTE rather than the growth rate, so a hull cannot farm
/// arbitrarily early braking by bolting on computers. AUTHORITY has no curve of
/// its own - torque sums and the structure caps it.
fn stack_curve(n: f32, limit: f32) -> f32 {
    limit - (limit - 1.0) / n.max(1.0)
}

/// The mass properties and spin the attitude ceiling is derived from. Avian
/// keeps the centre of mass in the body's local frame, which is the frame a
/// section's `Transform` is already in.
type HullBody<'w> = (
    &'w ComputedCenterOfMass,
    &'w ComputedAngularInertia,
    &'w AngularVelocity,
);

/// Live sections, as the arm needs them: where the section sits on the hull and
/// how big its authored box is.
type HullSection<'w> = (&'w Transform, Option<&'w SectionCollider>, &'w ChildOf);

/// The structural arm of every hull with live sections, keyed by root.
///
/// ONE pass over every live section rather than one pass per hull: the arm
/// needs each section's offset from its own root's centre of mass, and
/// re-filtering the section query per hull is quadratic in a busy scene.
fn hull_arms(
    arms: &mut EntityHashMap<f32>,
    q_root: &Query<HullBody>,
    q_section: &Query<HullSection, (With<SectionMarker>, Without<SectionInactiveMarker>)>,
) {
    arms.clear();
    for (transform, collider, &ChildOf(root)) in q_section {
        let Ok((center_of_mass, _, _)) = q_root.get(root) else {
            continue;
        };
        let half_extents = collider.copied().unwrap_or_default().aabb_half_extents();
        let arm = structural_arm(
            center_of_mass.0,
            [(transform.translation, transform.rotation, half_extents)],
        );
        let entry = arms.entry(root).or_insert(0.0);
        *entry = entry.max(arm);
    }
}

/// Fold every live controller on a hull into ONE attitude loop, split back
/// across the sections that provide it.
///
/// Each controller runs its own PD and adds its own torque, so a naive stack
/// multiplies gains AND the applied torque by the section count. Ten times the
/// damping gain is numerically unstable at the fixed timestep (`kd * dt` passes
/// 2 at two computers on the shipped tuning, which is the bang-bang limit cycle
/// that used to corkscrew released hulls). So the pass derives ship-level
/// totals and hands each controller a SHARE of them; because every controller
/// sees the same hull state, the shares re-sum to exactly the totals:
///
/// - **Authority** is the [`AttitudeEnvelope`]: the computers' summed
///   `max_torque` over the hull's largest principal inertia, or the global load
///   limit over the hull's structural arm, whichever gives up first. Torque
///   stacks linearly and needs no curve of its own, because the structure
///   already caps the result - fit enough computers and the hull turns at its
///   own limit and no harder.
/// - **Response** starts from the smallest authored `steering_lag`, independent
///   of which controller supplies the most torque.
/// - **Precision** (the P gain) is DIVIDED by [`stack_curve`] toward
///   [`STACK_PRECISION_LIMIT`], which lowers the ratio `kp / kd` the hull
///   coasts down to the command on. A shallower ratio means the stack starts
///   braking the turn earlier, so it lands on the commanded attitude instead
///   of sailing past and wobbling back.
/// - **Damping** (the D gain) stays at the fastest computer's single-controller
///   value, which keeps the stack numerically stable at any size.
///
/// The ceiling is re-derived every tick on purpose: the arm, the inertia and
/// the spin all move while the ship is flying and dying, so a hull that lost
/// its nose turns sharper and a hull already in a hard turn has less left.
pub(crate) fn update_controller_stack_tuning(
    // Two buffers, reused: this runs on every fixed tick for every hull in the
    // scene, so it must not allocate per ship per tick.
    mut stacks: Local<Vec<(Entity, Entity, ControllerSectionTuning)>>,
    mut arms: Local<EntityHashMap<f32>>,
    q_root: Query<HullBody>,
    q_section: Query<HullSection, (With<SectionMarker>, Without<SectionInactiveMarker>)>,
    mut q_controller: Query<
        (
            Entity,
            &ControllerSectionTuning,
            &mut PDController,
            &ChildOf,
        ),
        (
            With<ControllerSectionMarker>,
            Without<SectionInactiveMarker>,
        ),
    >,
) {
    hull_arms(&mut arms, &q_root, &q_section);

    stacks.clear();
    for (entity, tuning, _, &ChildOf(root)) in &q_controller {
        stacks.push((root, entity, *tuning));
    }
    // Sorted by root so each hull is one contiguous run. Rank inside a run no
    // longer means anything: torque sums, so the order it sums in does not.
    stacks.sort_unstable_by_key(|(root, _, _)| *root);

    let mut start = 0;
    while start < stacks.len() {
        let root = stacks[start].0;
        let end = stacks[start..]
            .iter()
            .position(|(other, _, _)| *other != root)
            .map_or(stacks.len(), |offset| start + offset);
        let stack = &stacks[start..end];
        start = end;

        let steering_lag = stack
            .iter()
            .map(|(_, _, tuning)| tuning.steering_lag)
            .min_by(f32::total_cmp)
            .unwrap_or(0.0);
        let base_frequency = frequency_from_steering_lag(steering_lag);
        let total_torque: f32 = stack
            .iter()
            .map(|(_, _, tuning)| tuning.max_torque.max(0.0))
            .sum();
        // A root avian has not measured yet reads as a point mass: both
        // ceilings come out infinite, which is the honest answer while the
        // colliders are still being linked and neither can be asked.
        let (inertia, arm, spin) = match q_root.get(root) {
            Ok((_, angular_inertia, angular_velocity)) => (
                angular_inertia
                    .principal_angular_inertia_with_local_frame()
                    .0
                    .max_element(),
                arms.get(&root).copied().unwrap_or(0.0),
                angular_velocity.length(),
            ),
            Err(_) => (0.0, 0.0, 0.0),
        };
        let budget = AttitudeEnvelope::new(total_torque, inertia, arm).available(spin);
        let precision = stack_curve(stack.len() as f32, STACK_PRECISION_LIMIT);

        for (_, entity, tuning) in stack {
            // Share of the ship-level loop this section carries. Weighting it
            // by the section's own torque keeps the live value readable as what
            // this computer is worth to this hull.
            let share = if total_torque > 0.0 {
                tuning.max_torque.max(0.0) / total_torque
            } else {
                1.0 / stack.len() as f32
            };
            // kp scales with frequency^2 and kd with frequency * damping
            // ratio, so a share of (kp / precision, kd) is this pair.
            let next = PDController {
                frequency: base_frequency * (share / precision).sqrt(),
                damping_ratio: INTERNAL_DAMPING_RATIO * (share * precision).sqrt(),
                max_angular_acceleration: budget * share,
            };
            let Ok((_, _, mut controller, _)) = q_controller.get_mut(*entity) else {
                continue;
            };
            // Change detection drives nothing here, but a stack that rewrites
            // itself every tick would wake every reader of the component.
            if *controller != next {
                *controller = next;
            }
        }
    }
}

/// The render-only half of a controller section: it shows the controller mesh
/// (and is pickable) but carries no [`PDController`], so it never tries to
/// torque a root.
///
/// The editor's view ship is a visual preview with no `RigidBody`; a live
/// controller there just floods the log with "root not found" every frame.
/// Without a `PDController` the bcs PD systems and
/// `insert_controller_section_target` both skip it, so the view is inert.
///
/// [`controller_section`] builds on this rather than repeating it, so the live
/// and view halves cannot drift: the shared render observer
/// (`insert_controller_section_render`) demands
/// [`SectionRenderMeshTransform`], and a view that dropped it would render
/// nothing where the live section shows the default cuboid.
pub fn preview_controller_section(config: ControllerSectionConfig) -> impl Bundle {
    trace!("preview_controller_section: config {:?}", config);

    (
        ControllerSectionMarker,
        SectionClass::Controller,
        ControllerSectionRenderMesh(config.render_mesh),
        SectionRenderMeshTransform(config.render_mesh_transform),
    )
}

/// One of the autopilot flight verbs the controller section grants. These are
/// the maneuvers the flight computer can fly (STOP/GOTO/ORBIT); CANCEL is not
/// listed because it only ever disengages an already-running maneuver and stays
/// available so a disabled verb can never strand an engaged autopilot. The enum
/// is the addressable handle used by [`WithheldVerbs`] and the
/// `SetControllerVerb` scenario action.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum FlightVerb {
    /// STOP: kill all velocity.
    Stop,
    /// GOTO: fly to the locked target and come to rest.
    Goto,
    /// ORBIT: circularize and station-keep in a gravity well.
    Orbit,
    /// LOCK: the targeting radar - deliberate hold-to-search locking. Not a
    /// maneuver, but the same computer-provided capability model: a ship
    /// without it cannot lock.
    Lock,
    /// RCS: reaction-control fine translation - the hold-to-nudge docking mode
    /// that pushes the hull along its local axes without exceeding a small
    /// speed cap. Not a planned maneuver but the same capability model: a ship
    /// without it cannot fine-adjust. Drives the
    /// shared `RcsIntent` / `rcs_burn_system` primitive the flight layer owns.
    Rcs,
    /// POINT DEFENSE: the computer works the IDLE turrets against inbound
    /// ordnance on its own - the autonomous half of the battery. Not a
    /// maneuver and not a key: it has no gesture at all, because it is the
    /// fallback behaviour of a battery the player is not using. The same
    /// capability model as the rest - a ship whose computer withholds it
    /// answers a salvo only by hand - which is what makes it the teaching
    /// lever (`DisableVerb` at spawn, `SetControllerVerb` mid-scenario).
    PointDefense,
}

/// The set of flight verbs WITHHELD on a controller section: computer-provided
/// capabilities (autopilot maneuvers plus the targeting radar) that this
/// controller does NOT grant, while the controller is otherwise alive. A verb
/// is available only if the ship has a live controller section that does NOT
/// withhold it (layered on top of the existing physical `flyable` gate - a live
/// controller plus a live thruster). An empty set (or an absent component) means
/// every verb is granted. Populated at spawn by the `DisableVerb` section
/// modification and flipped at runtime by the `SetControllerVerb` scenario
/// action.
#[derive(Component, Clone, Debug, Default, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct WithheldVerbs(pub HashSet<FlightVerb>);

impl WithheldVerbs {
    /// Whether the given verb is currently granted (i.e. NOT withheld).
    pub fn granted(&self, verb: FlightVerb) -> bool {
        !self.0.contains(&verb)
    }

    /// Withhold the given verb (remove the grant).
    pub fn withhold(&mut self, verb: FlightVerb) {
        self.0.insert(verb);
    }

    /// Grant the given verb (remove it from the withheld set).
    pub fn grant(&mut self, verb: FlightVerb) {
        self.0.remove(&verb);
    }
}

/// The desired rotation of the controller section, in world space. Written by
/// the player's mouse command, the AI brain, or the autopilot
/// (the flight layer) - whoever currently holds rotation authority.
#[derive(Component, Debug, Clone, Default, Deref, DerefMut, Reflect)]
pub struct ControllerSectionRotationInput(pub Quat);

/// Ordering handle for the controller section's own FixedUpdate work.
///
/// [`SyncRotationInput`](ControllerSectionSystems::SyncRotationInput) is the
/// seam every rotation-authority writer schedules against: the section layer
/// pins it before `PDControllerSystems::Sync`, and each writer above it (the
/// autopilot's `NovaFlightSystems`) declares itself `.before` this set. The
/// edge is declared from the writer's side on purpose - the section layer sits
/// under flight and must not name it.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum ControllerSectionSystems {
    /// Splits the hull's attitude loop across its live controllers. Runs
    /// first: everything downstream - the flight layer's turn-rate budget and
    /// the PD itself - reads the shares it writes.
    SyncStack,
    /// Copies the held rotation command into the PD controller's input.
    SyncRotationInput,
}

/// A plugin that will enable the ControllerSection.
#[derive(Default)]
pub struct ControllerSectionPlugin {
    /// Whether to spawn the section's render mesh (false on headless servers).
    pub render: bool,
}

impl Plugin for ControllerSectionPlugin {
    fn build(&self, app: &mut App) {
        trace!("ControllerSectionPlugin: build");

        // Register the section's reflected components so the debug inspector
        // (and the flight-feel retune) can see and edit them.
        app.register_type::<ControllerSectionMarker>()
            .register_type::<ControllerSectionRotationInput>()
            .register_type::<ControllerSectionTuning>()
            .register_type::<WithheldVerbs>()
            .register_type::<FlightVerb>();

        app.add_observer(insert_controller_section_target);

        app.add_systems(
            FixedUpdate,
            update_controller_stack_tuning.in_set(ControllerSectionSystems::SyncStack),
        );

        // The command copy into the bcs PDControllerInput runs on the
        // FIXED clock, between the command writers and the PD: its producer
        // (the autopilot) and consumer (PDControllerSystems::Sync) both tick in
        // FixedUpdate, and the old Update-schedule copy handed the PD a command
        // 1-2 ticks stale, varying with the 64 Hz-vs-render beat - up to
        // 0.22 rad of phantom command error and ~20% wasted torque during
        // autopilot slews. The chain below plus `NovaFlightPlugin`'s
        // `.before(SyncRotationInput)` transitively pins the autopilot ahead of
        // the PD sync, which the two sets' individual
        // `.before(SpaceshipSectionSystems)` constraints never guaranteed.
        // Update-schedule writers (player mouse, AI brain, torpedo guidance)
        // are unaffected: their command changes once per frame and is picked up
        // by the next tick exactly as before.
        app.add_systems(
            FixedUpdate,
            update_controller_section_rotation_input
                .in_set(ControllerSectionSystems::SyncRotationInput),
        );

        app.add_systems(
            FixedUpdate,
            sync_controller_section_forces.in_set(super::SpaceshipSectionSystems),
        );

        app.configure_sets(
            FixedUpdate,
            (
                ControllerSectionSystems::SyncStack,
                ControllerSectionSystems::SyncRotationInput,
                PDControllerSystems::Sync,
                super::SpaceshipSectionSystems,
            )
                .chain(),
        );

        if self.render {
            app.init_resource::<PlaceholderArt>();
            app.add_observer(insert_controller_section_render);
        }
    }
}

// `pub(crate)` so the flight tests can register the real rotation pipeline
// and cover autopilot -> PD -> hull swing end to end.
pub(crate) fn update_controller_section_rotation_input(
    mut q_controller: Query<
        (&mut PDControllerInput, &ControllerSectionRotationInput),
        (
            With<ControllerSectionMarker>,
            Without<SectionInactiveMarker>,
        ),
    >,
) {
    for (mut input, desired_rotation) in &mut q_controller {
        **input = **desired_rotation;
    }
}

pub(crate) fn sync_controller_section_forces(
    mut q_root: Query<Forces>,
    // A disabled-in-place controller (zero-health, non-leaf, still attached ->
    // `SectionInactiveMarker`) must stop stabilizing the hull: with no live
    // computer the flight layer's semantics are "adrift" (the autopilot
    // disengages and the player command freezes). Its `PDControllerOutput` is
    // still computed by the bcs PD system, but this is the only seam that
    // applies it, so gating here is what actually stops the torque. Mirrors the
    // filter already on `update_controller_section_rotation_input` and the
    // flight systems.
    q_controller: Query<(&PDControllerOutput, &PDControllerTarget), Without<SectionInactiveMarker>>,
) {
    for (output, target) in &q_controller {
        if let Ok(mut forces) = q_root.get_mut(**target) {
            forces.apply_torque(**output);
        }
    }
}

fn insert_controller_section_target(
    add: On<Add, ControllerSectionMarker>,
    mut commands: Commands,
    // Only real (live) controllers carry a `PDController`; a render-only preview controller
    // (`preview_controller_section`) does not, so it gets no target and stays inert.
    q_controller: Query<&ChildOf, (With<ControllerSectionMarker>, With<PDController>)>,
    q_root: Query<&Transform>,
) {
    let entity = add.entity;
    trace!("insert_controller_section_target: entity {:?}", entity);
    let Ok(ChildOf(root)) = q_controller.get(entity) else {
        // No `PDController` (a preview controller) - nothing to target. Not an error.
        return;
    };

    commands.entity(entity).insert(PDControllerTarget(*root));

    // Park the helm on the hull's spawn attitude. The bundle default is an
    // identity command, so a hull spawned aimed anywhere else fought its own
    // PD toward identity while the first writer (player, AI, autopilot) slewed
    // the command from identity - freshly spawned ships visibly re-aimed
    // before flying. Desired == current at t=0; every writer evolves the
    // command from its own previous state, so this seed is the whole opening.
    // Mirrors `on_autopilot_removed_cool_engines`, which parks the helm the
    // same way on disengage.
    if let Ok(transform) = q_root.get(*root) {
        commands
            .entity(entity)
            .insert(ControllerSectionRotationInput(transform.rotation));
    }
}

/// Marks the render-mesh child spawned for a controller section, so the render
/// observer can find and style it. Present only when rendering is enabled.
#[derive(Component, Clone, Debug, Reflect)]
pub struct ControllerSectionRenderMarker;

fn insert_controller_section_render(
    add: On<Add, ControllerSectionMarker>,
    mut commands: Commands,
    placeholder: Res<PlaceholderArt>,
    asset_server: Res<AssetServer>,
    q_controller: Query<
        (
            &ControllerSectionRenderMesh,
            &SectionRenderMeshTransform,
            Has<ControllerSectionRenderMarker>,
        ),
        With<ControllerSectionMarker>,
    >,
    q_authored: Query<
        (),
        (
            With<ControllerSectionMarker>,
            With<ControllerSectionRenderMesh>,
        ),
    >,
) {
    let entity = add.entity;
    trace!("insert_controller_section_render: entity {:?}", entity);

    let Ok((render_mesh, render_mesh_transform, has_render)) = q_controller.get(entity) else {
        // Carrying the render mesh but not its transform IS the bug this guard
        // is for: the section renders, silently at identity, losing whatever
        // the author posed. See `preview_controller_section`.
        if q_authored.contains(entity) {
            error!(
                "insert_controller_section_render: entity {:?} has a render mesh but no \
                 SectionRenderMeshTransform, so its authored pose would be dropped",
                entity
            );
        } else {
            // No authored render data at all. `ControllerSectionRenderMesh` is
            // crate-private, so an outside caller - an example marking a part
            // it already gave its own art - can only add the marker. Nothing to
            // build, and nothing wrong.
            trace!(
                "insert_controller_section_render: entity {:?} has no authored render data, \
                 leaving its art alone",
                entity
            );
        }
        return;
    };

    if has_render {
        trace!(
            "insert_controller_section_render: entity {:?} already has render, skipping",
            entity
        );
        return;
    }

    commands
        .entity(entity)
        .insert(ControllerSectionRenderMarker);
    match &**render_mesh {
        Some(asset_ref) => {
            let scene = asset_ref.resolve(&asset_server);
            // Authored render-mesh transform (identity when unset), on the mesh
            // CHILD so it moves the art only.
            let transform = render_mesh_transform
                .map(RenderMeshTransform::to_transform)
                .unwrap_or_default();
            commands.entity(entity).insert((children![(
                Name::new("Controller Section Body"),
                transform,
                SectionRenderOf(entity),
                WorldAssetRoot(scene),
            ),],));
        }
        None => {
            commands.entity(entity).insert((children![
                (
                    Name::new("Controller Section Body (A)"),
                    SectionRenderOf(entity),
                    Mesh3d(placeholder.body.clone()),
                    MeshMaterial3d(placeholder.controller_material.clone()),
                ),
                (
                    Name::new("Controller Section Window (B)"),
                    SectionRenderOf(entity),
                    Mesh3d(placeholder.window.clone()),
                    MeshMaterial3d(placeholder.window_material.clone()),
                    Transform::from_xyz(0.0, 0.5, 0.0),
                )
            ],));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The precision curve's shape is the feature: a real ceiling, most of the
    /// gain on the second unit, and a tenth unit that is not worth mounting.
    #[test]
    fn the_precision_curve_starts_at_one_and_converges_on_its_limit() {
        // 1.00, 1.25, 1.375, 1.45 -> 1.50.
        let precision = |n: f32| stack_curve(n, STACK_PRECISION_LIMIT);
        assert!(
            (precision(1.0) - 1.0).abs() < 1e-6,
            "one computer is the identity"
        );
        assert!((precision(2.0) - 1.25).abs() < 1e-6);
        assert!((precision(4.0) - 1.375).abs() < 1e-6);
        assert!((precision(10.0) - 1.45).abs() < 1e-6);
        assert!(
            precision(1000.0) < STACK_PRECISION_LIMIT,
            "the limit is an asymptote, never reached"
        );
        // Degenerate counts stay on the identity rather than exploding.
        assert!((precision(0.0) - 1.0).abs() < 1e-6);
    }

    /// The arm reaches the outer face of the FURTHEST live section, measured
    /// from the hull's centre of mass - and a section the hull lost stops
    /// counting, which is what makes a damaged hull turn sharper.
    #[test]
    fn the_hull_arm_reaches_the_furthest_live_section() {
        let mut app = stack_app();
        let (root, _) = spawn_hull(&mut app, 5, 1.0);
        spawn_computers(&mut app, root, 1);
        // A nose out past the rest of the hull, so exactly one section owns
        // the arm and losing it has somewhere to fall to.
        let nose = app
            .world_mut()
            .spawn((
                ChildOf(root),
                SectionMarker,
                Transform::from_xyz(0.0, 0.0, 3.0),
                SectionCollider::Cuboid { size: Vec3::ONE },
            ))
            .id();
        app.world_mut().run_schedule(FixedUpdate);
        assert!(
            (hull_arm(&app, root) - 3.5).abs() < 1e-4,
            "the nose owns the arm at 3.5 u, got {}",
            hull_arm(&app, root)
        );

        // Blow the nose off: the arm shrinks even though the section is still
        // parented, because the pass only counts LIVE structure - and a
        // shorter arm is a higher ceiling, so the wreck turns sharper.
        app.world_mut()
            .entity_mut(nose)
            .insert(SectionInactiveMarker);
        app.world_mut().run_schedule(FixedUpdate);
        assert!(
            (hull_arm(&app, root) - 2.5).abs() < 1e-4,
            "a lost nose shortens the arm, got {}",
            hull_arm(&app, root)
        );
    }

    fn stack_app() -> App {
        let mut app = App::new();
        app.add_systems(FixedUpdate, update_controller_stack_tuning);
        app
    }

    fn tuning() -> ControllerSectionTuning {
        ControllerSectionTuning {
            steering_lag: 0.5,
            max_torque: DEFAULT_MAX_TORQUE,
        }
    }

    /// The mass properties avian would compute for a line of `count` unit
    /// cubes at `density`, written straight onto the root so the stack pass can
    /// be exercised without a physics step. `density` buys inertia without
    /// buying arm, which is how a short rig reaches the torque-bound regime.
    fn spawn_hull(app: &mut App, count: usize, density: f32) -> (Entity, Vec<Entity>) {
        let first = -((count as f32) - 1.0) * 0.5;
        let transverse = ((0..count)
            .map(|index| (first + index as f32).powi(2))
            .sum::<f32>()
            + count as f32 / 6.0)
            * density;
        let root = app
            .world_mut()
            .spawn((
                ComputedCenterOfMass(Vec3::ZERO),
                ComputedAngularInertia::new(Vec3::new(
                    transverse,
                    transverse,
                    count as f32 * density / 6.0,
                )),
                AngularVelocity(Vec3::ZERO),
            ))
            .id();
        let sections = (0..count)
            .map(|index| {
                app.world_mut()
                    .spawn((
                        ChildOf(root),
                        SectionMarker,
                        Transform::from_xyz(0.0, 0.0, first + index as f32),
                        SectionCollider::Cuboid { size: Vec3::ONE },
                    ))
                    .id()
            })
            .collect();
        (root, sections)
    }

    /// The arm the pass derived, read back out of the ceiling it wrote: the
    /// shipped hulls are all structure-bound, so `LOAD_LIMIT / ceiling` in
    /// metres is exactly the arm that produced it.
    fn hull_arm(app: &App, root: Entity) -> f32 {
        let budget: f32 = app
            .world()
            .iter_entities()
            .filter(|entity| entity.get::<ChildOf>().map(|c| c.0) == Some(root))
            .filter_map(|entity| entity.get::<PDController>())
            .map(|pd| pd.max_angular_acceleration)
            .sum();
        nova_events::prelude::LOAD_LIMIT / budget / nova_events::prelude::METERS_PER_UNIT
    }

    /// `count` shipped computers on `root`, massless so the rig isolates the
    /// stack from what mounting one would cost in inertia.
    fn spawn_computers(app: &mut App, root: Entity, count: usize) -> Vec<Entity> {
        (0..count)
            .map(|_| {
                app.world_mut()
                    .spawn((
                        ChildOf(root),
                        ControllerSectionMarker,
                        tuning(),
                        PDController {
                            frequency: 4.0,
                            damping_ratio: 4.0,
                            max_angular_acceleration: 0.0,
                        },
                    ))
                    .id()
            })
            .collect()
    }

    fn spawn_stack(app: &mut App, count: usize) -> (Entity, Vec<Entity>) {
        let (root, _) = spawn_hull(app, 3, 1.0);
        let controllers = spawn_computers(app, root, count);
        (root, controllers)
    }

    /// The live loop of a stack, in the terms the PD builds its gains from:
    /// `kp` scales with frequency squared and `kd` with frequency times
    /// damping ratio, so summing those products compares stacks of any size
    /// without restating the PD's own constants.
    fn stack_loop(app: &App, controllers: &[Entity]) -> (f32, f32, f32) {
        controllers
            .iter()
            .filter_map(|entity| app.world().get::<PDController>(*entity))
            .fold((0.0, 0.0, 0.0), |(kp, kd, authority), pd| {
                (
                    kp + pd.frequency * pd.frequency,
                    kd + pd.frequency * pd.damping_ratio,
                    authority + pd.max_angular_acceleration,
                )
            })
    }

    /// The reference hull, end to end: one computer on three unit cubes lands
    /// on the structural ceiling the whole model was calibrated against, and
    /// nothing about it is authored.
    #[test]
    fn a_lone_controller_gets_the_hull_structural_ceiling() {
        let mut app = stack_app();
        let (_, controllers) = spawn_stack(&mut app, 1);
        app.world_mut().run_schedule(FixedUpdate);

        let live = *app.world().get::<PDController>(controllers[0]).unwrap();
        assert_eq!(live.frequency, 4.0);
        assert_eq!(live.damping_ratio, 4.0);
        assert!(
            (live.max_angular_acceleration - 5.232).abs() < 1e-2,
            "8 G over a 15 m arm is 5.23 rad/s2, got {}",
            live.max_angular_acceleration
        );
    }

    /// Stacking splits ONE loop rather than running several. On a
    /// structure-bound hull the extra torque buys no authority at all - the
    /// hull would tear first - while the P gain still drops by the precision
    /// curve and the D gain, the numerically dangerous one, does not move.
    #[test]
    fn a_stack_shares_one_attitude_loop() {
        let mut app = stack_app();
        let (_, one) = spawn_stack(&mut app, 1);
        let (_, four) = spawn_stack(&mut app, 4);
        app.world_mut().run_schedule(FixedUpdate);

        let (kp_one, kd_one, authority_one) = stack_loop(&app, &one);
        let (kp_four, kd_four, authority_four) = stack_loop(&app, &four);

        assert!(
            (authority_four / authority_one - 1.0).abs() < 1e-3,
            "a structure-bound hull gains nothing from computers, got {}",
            authority_four / authority_one
        );
        assert!(
            (kp_four / kp_one - 1.0 / 1.375).abs() < 1e-3,
            "the P gain must fall by the precision curve, got {}",
            kp_four / kp_one
        );
        assert!(
            (kd_four / kd_one - 1.0).abs() < 1e-3,
            "the D gain must stay at one computer's worth (kd * dt stability), \
             got {}",
            kd_four / kd_one
        );
    }

    /// Torque stacks LINEARLY where it is the limit that binds, with no curve
    /// and no cap of its own - and stops the moment the structure catches it.
    #[test]
    fn a_torque_bound_hull_buys_its_physics_back_and_stops_there() {
        let mut app = stack_app();
        // Fifteen sections at twenty times the density: four times the inertia
        // the same arm can carry, so this hull is torque-bound where the
        // reference 1-1-1 is structure-bound.
        let (root, _) = spawn_hull(&mut app, 15, 20.0);
        let controllers = spawn_computers(&mut app, root, 8);

        let budget = |app: &mut App, live: usize| {
            for (index, controller) in controllers.iter().enumerate() {
                let mut entity = app.world_mut().entity_mut(*controller);
                if index < live {
                    entity.remove::<SectionInactiveMarker>();
                } else {
                    entity.insert(SectionInactiveMarker);
                }
            }
            app.world_mut().run_schedule(FixedUpdate);
            stack_loop(app, &controllers[..live]).2
        };

        let one = budget(&mut app, 1);
        let two = budget(&mut app, 2);
        assert!(
            (two / one - 2.0).abs() < 1e-2,
            "torque stacks linearly on a torque-bound hull, got {}",
            two / one
        );
        // Eight computers overshoot the structural ceiling, and it catches them.
        let eight = budget(&mut app, 8);
        assert!(
            eight < one * 8.0 * 0.9,
            "the structure must catch the stack, got {eight} from {one}"
        );
        assert!(
            (eight - 1.0464).abs() < 1e-2,
            "8 G over a 75 m arm is 1.046 rad/s2, got {eight}"
        );
    }

    /// Redundancy: a stack that loses a computer re-derives itself into a
    /// smaller stack - the loop degrades, the hull keeps flying.
    #[test]
    fn losing_one_controller_degrades_the_stack_instead_of_killing_it() {
        let mut app = stack_app();
        let (_, controllers) = spawn_stack(&mut app, 3);
        app.world_mut().run_schedule(FixedUpdate);
        let (kp_three, _, three) = stack_loop(&app, &controllers);

        app.world_mut()
            .entity_mut(controllers[1])
            .insert(SectionInactiveMarker);
        app.world_mut().run_schedule(FixedUpdate);
        let survivors = [controllers[0], controllers[2]];
        let (kp_two, _, two) = stack_loop(&app, &survivors);

        assert!(
            (three - two).abs() < 1e-3,
            "a structure-bound hull keeps its ceiling, {three} -> {two}"
        );
        assert!(
            kp_two > kp_three,
            "the survivors must re-derive two-computer precision, {kp_three} -> {kp_two}"
        );
    }

    #[test]
    fn a_mixed_stack_uses_the_smallest_steering_lag() {
        let mut app = stack_app();
        let (root, _) = spawn_hull(&mut app, 3, 1.0);
        for (steering_lag, max_torque) in [(0.8, 1501.0), (0.25, 375.0)] {
            app.world_mut().spawn((
                ChildOf(root),
                ControllerSectionMarker,
                ControllerSectionTuning {
                    steering_lag,
                    max_torque,
                },
                PDController {
                    frequency: 1.0,
                    damping_ratio: 1.0,
                    max_angular_acceleration: 0.0,
                },
            ));
        }
        app.world_mut().run_schedule(FixedUpdate);

        let controllers = app
            .world()
            .iter_entities()
            .filter(|entity| entity.contains::<ControllerSectionMarker>())
            .map(|entity| entity.id())
            .collect::<Vec<_>>();
        let (kp, kd, _) = stack_loop(&app, &controllers);
        let effective_lag = 0.5 * kd / kp;
        assert!(
            (effective_lag - 0.25 * 1.25).abs() < 1e-4,
            "the 0.25 s computer plus stack braking should set the loop lag, got {effective_lag}"
        );
    }

    /// A bigger computer carries a bigger share of the one loop, because the
    /// share is its fraction of the hull's torque.
    #[test]
    fn a_mixed_stack_shares_the_loop_by_torque() {
        let mut app = stack_app();
        let (root, _) = spawn_hull(&mut app, 3, 1.0);
        let spawn = |app: &mut App, max_torque: f32| {
            app.world_mut()
                .spawn((
                    ChildOf(root),
                    ControllerSectionMarker,
                    ControllerSectionTuning {
                        max_torque,
                        ..tuning()
                    },
                    PDController {
                        frequency: 4.0,
                        damping_ratio: 4.0,
                        max_angular_acceleration: 0.0,
                    },
                ))
                .id()
        };
        // Weak one first, so a pass that trusted spawn order would share wrong.
        let weak = spawn(&mut app, 500.0);
        let strong = spawn(&mut app, 1500.0);
        app.world_mut().run_schedule(FixedUpdate);

        let strong_share = app
            .world()
            .get::<PDController>(strong)
            .unwrap()
            .max_angular_acceleration;
        let weak_share = app
            .world()
            .get::<PDController>(weak)
            .unwrap()
            .max_angular_acceleration;
        assert!(
            (strong_share / weak_share - 3.0).abs() < 1e-3,
            "three times the torque carries three times the share, got {}",
            strong_share / weak_share
        );
    }

    /// The rotation-command pipeline must stay pinned in the order
    /// `NovaFlightSystems -> SyncRotationInput -> PDControllerSystems::Sync`
    /// now that the edge is declared from the flight side instead of by
    /// `sections` naming `crate::flight`. Probes stand in each set, so this
    /// exercises the production `configure_sets` calls of both plugins.
    #[test]
    fn rotation_command_pipeline_runs_flight_then_sync_then_pd() {
        #[derive(Resource, Default)]
        struct Order(Vec<&'static str>);

        let mut app = App::new();
        app.init_resource::<Order>();
        app.add_plugins((
            bevy::time::TimePlugin,
            crate::physics::prelude::PDControllerPlugin,
            ControllerSectionPlugin::default(),
            crate::flight::NovaFlightPlugin,
        ));
        app.add_systems(
            FixedUpdate,
            (
                (|mut order: ResMut<Order>| order.0.push("stack"))
                    .in_set(ControllerSectionSystems::SyncStack),
                (|mut order: ResMut<Order>| order.0.push("flight"))
                    .in_set(crate::flight::NovaFlightSystems),
                (|mut order: ResMut<Order>| order.0.push("sync"))
                    .in_set(ControllerSectionSystems::SyncRotationInput),
                (|mut order: ResMut<Order>| order.0.push("pd")).in_set(PDControllerSystems::Sync),
            ),
        );

        app.world_mut().run_schedule(FixedUpdate);

        assert_eq!(
            app.world().resource::<Order>().0,
            ["stack", "flight", "sync", "pd"],
            "the stack split must land before the turn-rate budget reads it"
        );
    }

    #[test]
    fn spawns_controller_with_default_config() {
        let mut app = App::new();
        let id = app
            .world_mut()
            .spawn(controller_section(ControllerSectionConfig::default()))
            .id();

        app.update();

        assert!(app.world().get::<ControllerSectionMarker>(id).is_some());
    }

    #[test]
    fn spawns_controller_with_custom_scene() {
        let mut app = App::new();
        let custom_scene = Handle::<WorldAsset>::default();
        let config = ControllerSectionConfig {
            render_mesh: Some(custom_scene.clone().into()),
            ..Default::default()
        };
        let id = app.world_mut().spawn(controller_section(config)).id();

        app.update();

        assert!(app.world().get::<ControllerSectionMarker>(id).is_some());
        let render_mesh = app.world().get::<ControllerSectionRenderMesh>(id).unwrap();
        assert!(render_mesh.0.is_some());
        assert_eq!(
            render_mesh.0.as_ref().unwrap(),
            &AssetRef::from(custom_scene)
        );
    }

    #[test]
    fn preview_controller_carries_no_live_pd_controller() {
        // The editor preview controller renders but must not carry a live PDController - that is
        // what spammed "root not found" against the non-physics preview root.
        let mut app = App::new();
        let id = app
            .world_mut()
            .spawn(preview_controller_section(
                ControllerSectionConfig::default(),
            ))
            .id();
        app.update();

        assert!(app.world().get::<ControllerSectionMarker>(id).is_some());
        assert!(
            app.world().get::<PDController>(id).is_none(),
            "a preview controller must not carry a live PDController"
        );
    }

    #[test]
    fn preview_controller_carries_the_render_mesh_transform() {
        // Regression: the shared render observer queries `SectionRenderMeshTransform`,
        // so the preview bundle must carry it or the observer skips the preview
        // controller entirely - a meshless controller in the editor while the live
        // one shows the default cuboid.
        let mut app = App::new();
        let id = app
            .world_mut()
            .spawn(preview_controller_section(
                ControllerSectionConfig::default(),
            ))
            .id();
        app.update();

        assert!(
            app.world().get::<SectionRenderMeshTransform>(id).is_some(),
            "the preview controller must carry SectionRenderMeshTransform so the \
             render observer renders it"
        );
    }

    /// A hull spawned aimed anywhere must hold that attitude: the target
    /// observer parks the helm on the spawn rotation, so desired == current at
    /// t=0 and the PD never issues a correction torque nothing asked for.
    /// Regression: the identity-command default made freshly spawned AI ships
    /// swing toward identity before flying their first leg.
    #[test]
    fn a_spawned_hull_holds_its_spawn_attitude() {
        use std::time::Duration;

        use bevy::time::TimeUpdateStrategy;

        let mut app = App::new();
        // AssetPlugin + MeshPlugin are required even for primitive
        // colliders - avian's collider cache reads `AssetEvent<Mesh>`.
        app.add_plugins((
            MinimalPlugins,
            TransformPlugin,
            bevy::asset::AssetPlugin::default(),
            bevy::mesh::MeshPlugin,
        ));
        app.add_plugins(PhysicsPlugins::default());
        app.add_plugins((
            crate::physics::prelude::PDControllerPlugin,
            ControllerSectionPlugin::default(),
        ));
        app.insert_resource(Gravity(Vec3::ZERO));
        app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
            1.0 / 60.0,
        )));
        app.finish();

        // An arbitrary off-identity spawn attitude, like an arena slot aimed
        // at its first patrol leg.
        let spawn_rotation = Quat::from_axis_angle(Vec3::new(1.0, 2.0, -1.0).normalize(), 1.3);
        let hull = app
            .world_mut()
            .spawn((RigidBody::Dynamic, Transform::from_rotation(spawn_rotation)))
            .id();
        for z in [-1.0, 0.0, 1.0] {
            app.world_mut().spawn((
                ChildOf(hull),
                Transform::from_xyz(0.0, 0.0, z),
                Collider::cuboid(1.0, 1.0, 1.0),
                ColliderDensity(1.0),
            ));
        }
        let controller = app
            .world_mut()
            .spawn((
                ChildOf(hull),
                Transform::default(),
                controller_section(ControllerSectionConfig {
                    steering_lag: 0.5,
                    ..Default::default()
                }),
            ))
            .id();

        // First updated frame: the helm command must already equal the spawn
        // attitude - no synthetic identity command anywhere in the pipeline.
        app.update();
        let command = **app
            .world()
            .get::<ControllerSectionRotationInput>(controller)
            .unwrap();
        assert!(
            command.angle_between(spawn_rotation) < 1e-5,
            "the helm must park on the spawn attitude, got {command:?} vs {spawn_rotation:?}"
        );

        // With nothing writing the command, the hull sits still: no swing.
        for _ in 0..120 {
            app.update();
        }
        let rotation = app.world().get::<Rotation>(hull).unwrap().0;
        let spin = app.world().get::<AngularVelocity>(hull).unwrap().length();
        assert!(
            rotation.angle_between(spawn_rotation) < 1e-2,
            "an unattended hull must hold its spawn attitude, drifted {} rad",
            rotation.angle_between(spawn_rotation)
        );
        assert!(
            spin < 1e-2,
            "an unattended hull must not pick up spin, at {spin} rad/s"
        );
    }

    #[test]
    fn only_a_live_controller_gets_a_pd_target() {
        // `insert_controller_section_target` gives a target only to controllers that carry a
        // PDController. The bcs PD system iterates `(PDController, ..., PDControllerTarget, ...)`,
        // so a preview controller with neither is never processed and never logs "root not found".
        let mut app = App::new();
        app.add_observer(insert_controller_section_target);

        let root = app.world_mut().spawn_empty().id();
        let live = app
            .world_mut()
            .spawn((
                controller_section(ControllerSectionConfig::default()),
                ChildOf(root),
            ))
            .id();
        let preview = app
            .world_mut()
            .spawn((
                preview_controller_section(ControllerSectionConfig::default()),
                ChildOf(root),
            ))
            .id();
        app.update();

        assert!(
            app.world().get::<PDControllerTarget>(live).is_some(),
            "a live controller targets its root"
        );
        assert!(
            app.world().get::<PDControllerTarget>(preview).is_none(),
            "a preview controller must not target a root - that is the PD-spam fix"
        );
    }
}
