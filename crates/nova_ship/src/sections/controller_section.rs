//! A section of a spaceship that can control its rotation using a PD controller.
//!
//! A hull may carry several controllers; they share one attitude loop rather
//! than each running their own, and the share is what makes stacking a
//! diminishing return (see [`update_controller_stack_tuning`]).

use avian3d::prelude::*;
use bevy::{platform::collections::HashSet, prelude::*};
use nova_gameplay::prelude::{
    AssetRef, ControllerSectionMarker, SectionClass, SectionInactiveMarker,
};

use crate::prelude::{
    PDController, PDControllerInput, PDControllerOutput, PDControllerSystems, PDControllerTarget,
    RenderMeshTransform, SectionRenderMeshTransform, SectionRenderOf,
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
    /// The frequency of the PD controller in Hz.
    pub frequency: f32,
    /// The damping ratio of the PD controller.
    pub damping_ratio: f32,
    /// The maximum torque that can be applied by the PD controller.
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
    /// controllers author all five via gen_content.
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
/// component for the five cues - they share the same consumers). The audio
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
            rcs_loop: config.rcs_loop_sound.clone(),
        }
    }
}

impl Default for ControllerSectionConfig {
    fn default() -> Self {
        Self {
            frequency: 2.0,
            damping_ratio: 2.0,
            max_torque: 1.0,
            render_mesh: None,
            render_mesh_transform: None,
            lock_on_sound: None,
            lock_off_sound: None,
            radar_deny_sound: None,
            radar_retarget_sound: None,
            safety_on_sound: None,
            rcs_loop_sound: None,
        }
    }
}

#[derive(Component, Clone, Debug, Deref, DerefMut, Reflect)]
struct ControllerSectionRenderMesh(#[reflect(ignore)] Option<AssetRef<WorldAsset>>);

/// Helper function to create a controller section entity bundle.
pub fn controller_section(config: ControllerSectionConfig) -> impl Bundle {
    debug!("controller_section: config {:?}", config);

    (
        ControllerSectionMarker,
        SectionClass::Controller,
        ControllerSectionTuning {
            frequency: config.frequency,
            damping_ratio: config.damping_ratio,
            max_torque: config.max_torque,
        },
        PDController {
            frequency: config.frequency,
            damping_ratio: config.damping_ratio,
            max_torque: config.max_torque,
        },
        ControllerSectionRotationInput::default(),
        ControllerSectionSounds::from_config(&config),
        ControllerSectionRenderMesh(config.render_mesh),
        SectionRenderMeshTransform(config.render_mesh_transform),
    )
}

/// The AUTHORED PD tuning of a controller section, kept apart from the live
/// [`PDController`] that [`update_controller_stack_tuning`] derives from it.
/// The live component is a share of a ship-wide budget and changes whenever a
/// sibling controller is added or dies, so the authored numbers must survive
/// somewhere to re-derive from.
#[derive(Component, Clone, Copy, Debug, PartialEq, Reflect)]
pub struct ControllerSectionTuning {
    /// The frequency of the PD controller in Hz.
    pub frequency: f32,
    /// The damping ratio of the PD controller.
    pub damping_ratio: f32,
    /// The maximum torque that can be applied by the PD controller.
    pub max_torque: f32,
}

/// Ceiling on a stack's torque budget, as a multiple of its strongest
/// controller's own budget. Deliberately small: rotation authority is what a
/// hull's g-forces are made of, so a barge with ten computers may pull twice
/// what one computer pulls and never more, whatever it bolts on.
///
/// Both limits are constants rather than authored fields on purpose. A curve
/// a mod could raise is a curve a mod could flatten back into the linear
/// stacking the ceiling exists to prevent, and per-hull handling is already
/// authorable through the knob that should carry it - a controller's own
/// `max_torque`. (There is a mechanical reason too: the section layer sits
/// under the flight layer and must not read `FlightSettings`, where the rest
/// of the flight-feel tunables live.)
const STACK_AUTHORITY_LIMIT: f32 = 2.0;

/// Ceiling on a stack's precision gain - how much earlier than a single
/// computer the stack starts arresting the turn. Costs command-tracking lag
/// one for one (the hull trails a moving command by `rate / slope`), so it
/// stays well under the authority limit: a stacked hull should read as
/// deliberate, not as detached from the helm.
const STACK_PRECISION_LIMIT: f32 = 1.5;

/// The stacking curve: `limit - (limit - 1) / n`, worth 1.0 at `n = 1` and
/// approaching `limit` from below.
///
/// Chosen for the ASYMPTOTE rather than the growth rate. The n-th unit is
/// worth `(limit - 1) / (n * (n - 1))` - half the total gain arrives with the
/// second controller, three quarters by the fourth, and the tenth is worth
/// 0.6% of one controller. A sum-like curve (linear, or the harmonic series)
/// has no ceiling, so a hull could always buy more authority by bolting on
/// more computers; this one cannot be farmed.
fn stack_curve(n: f32, limit: f32) -> f32 {
    limit - (limit - 1.0) / n.max(1.0)
}

/// What the `rank`-th strongest controller (rank 0 = strongest) adds to the
/// torque budget, as a fraction of its own authored `max_torque`. These are
/// the marginal steps of [`stack_curve`], so `n` identical controllers sum to
/// `stack_curve(n, STACK_AUTHORITY_LIMIT)` exactly.
fn authority_weight(rank: usize) -> f32 {
    match rank {
        0 => 1.0,
        rank => (STACK_AUTHORITY_LIMIT - 1.0) / ((rank + 1) * rank) as f32,
    }
}

/// Fold every live controller on a hull into ONE attitude loop, split back
/// across the sections that provide it.
///
/// Each controller runs its own PD and adds its own torque, so a naive stack
/// multiplies gains AND torque by the section count: ten computers would turn
/// a barge ten times harder, and - worse - ten times the damping gain is
/// numerically unstable at the fixed timestep (`kd * dt` passes 2 at two
/// computers on the shipped tuning, which is the bang-bang limit cycle that
/// used to corkscrew released hulls). So the pass derives ship-level totals
/// and hands each controller a SHARE of them; because every controller sees
/// the same hull state, the shares re-sum to exactly the totals:
///
/// - **Authority** (`max_torque`, the saturated turn) grows on
///   [`stack_curve`] toward [`STACK_AUTHORITY_LIMIT`]. Bounded against the
///   strongest computer, never against the hull, so a bigger ship is always a
///   slower ship: peak angular acceleration is `budget / inertia` and only the
///   numerator is capped.
/// - **Precision** (the P gain) is DIVIDED by [`stack_curve`] toward
///   [`STACK_PRECISION_LIMIT`], which lowers the ratio `kp / kd` the hull
///   coasts down to the command on. A shallower ratio means the stack starts
///   braking the turn earlier, so it lands on the commanded attitude instead
///   of sailing past and wobbling back.
/// - **Damping** (the D gain) is held at exactly one computer's worth, which
///   is what keeps the stack numerically stable at any size.
///
/// The split leaves onset untouched: at rest the D term is zero, so the first
/// tick of a command still spends the whole (larger) torque budget. Stacking
/// makes a hull heavier-handed, never slower to answer.
///
/// A single controller is the identity case - `stack_curve(1) = 1` - so
/// small craft fly exactly as they did before stacking existed.
pub(crate) fn update_controller_stack_tuning(
    // One buffer, reused: this runs on every fixed tick for every hull in the
    // scene, so it must not allocate per ship per tick.
    mut stacks: Local<Vec<(Entity, Entity, ControllerSectionTuning)>>,
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
    stacks.clear();
    for (entity, tuning, _, &ChildOf(root)) in &q_controller {
        stacks.push((root, entity, *tuning));
    }
    // Hull, then strongest computer first: the rank weights are a diminishing
    // series, so the budget must spend the biggest weight on the biggest
    // computer. Sorting by root also groups each hull into one contiguous run.
    stacks.sort_unstable_by(|(left_root, _, left), (right_root, _, right)| {
        left_root
            .cmp(right_root)
            .then(right.max_torque.total_cmp(&left.max_torque))
    });

    let mut start = 0;
    while start < stacks.len() {
        let root = stacks[start].0;
        let end = stacks[start..]
            .iter()
            .position(|(other, _, _)| *other != root)
            .map_or(stacks.len(), |offset| start + offset);
        let stack = &stacks[start..end];
        start = end;

        let base = stack[0].2;
        let budget: f32 = stack
            .iter()
            .enumerate()
            .map(|(rank, (_, _, tuning))| tuning.max_torque * authority_weight(rank))
            .sum();
        let precision = stack_curve(stack.len() as f32, STACK_PRECISION_LIMIT);

        for (rank, (_, entity, tuning)) in stack.iter().enumerate() {
            // Share of the ship-level loop this section carries. Weighting it
            // by the section's own contribution keeps a live `max_torque`
            // readable as "what this computer is worth to this hull".
            let share = if budget > 0.0 {
                tuning.max_torque * authority_weight(rank) / budget
            } else {
                1.0 / stack.len() as f32
            };
            // kp scales with frequency^2 and kd with frequency * damping
            // ratio, so a share of (kp / precision, kd) is this pair.
            let next = PDController {
                frequency: base.frequency * (share / precision).sqrt(),
                damping_ratio: base.damping_ratio * (share * precision).sqrt(),
                max_torque: budget * share,
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

/// A render-only controller section for the editor preview: it shows the controller mesh (and is
/// pickable) but carries no [`PDController`], so it never tries to torque a root. The editor
/// preview ship is a visual config preview with no `RigidBody`; a live controller there just
/// floods the log with "root not found" every frame. Because it has no
/// `PDController`, the bcs PD systems and `insert_controller_section_target` both skip it, so the
/// preview controller is inert.
pub fn preview_controller_section(config: ControllerSectionConfig) -> impl Bundle {
    debug!("preview_controller_section: config {:?}", config);

    (
        ControllerSectionMarker,
        ControllerSectionRenderMesh(config.render_mesh),
        // The shared render observer (`insert_controller_section_render`) queries
        // for this too, so without it the preview controller matches no query and
        // renders nothing - a meshless controller in the editor even though the
        // live one shows the default cuboid. Preview carries it like the live
        // `controller_section` bundle does.
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
        debug!("ControllerSectionPlugin: build");

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

        // NOTE: the command copy into the bcs PDControllerInput runs on the
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
) {
    let entity = add.entity;
    trace!("insert_controller_section_target: entity {:?}", entity);
    let Ok(ChildOf(root)) = q_controller.get(entity) else {
        // No `PDController` (a preview controller) - nothing to target. Not an error.
        return;
    };

    commands.entity(entity).insert(PDControllerTarget(*root));
}

/// Marks the render-mesh child spawned for a controller section, so the render
/// observer can find and style it. Present only when rendering is enabled.
#[derive(Component, Clone, Debug, Reflect)]
pub struct ControllerSectionRenderMarker;

fn insert_controller_section_render(
    add: On<Add, ControllerSectionMarker>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
    q_controller: Query<
        (
            &ControllerSectionRenderMesh,
            &SectionRenderMeshTransform,
            Has<ControllerSectionRenderMarker>,
        ),
        With<ControllerSectionMarker>,
    >,
) {
    let entity = add.entity;
    trace!("insert_controller_section_render: entity {:?}", entity);

    let Ok((render_mesh, render_mesh_transform, has_render)) = q_controller.get(entity) else {
        error!(
            "insert_controller_section_render: entity {:?} not found in q_controller",
            entity
        );
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
                    Mesh3d(meshes.add(Cuboid::new(1.0, 1.0, 1.0))),
                    MeshMaterial3d(materials.add(Color::srgb(0.2, 0.7, 0.9))),
                ),
                (
                    Name::new("Controller Section Window (B)"),
                    SectionRenderOf(entity),
                    Mesh3d(meshes.add(Cylinder::new(0.2, 0.1))),
                    MeshMaterial3d(materials.add(Color::srgb(0.9, 0.9, 1.0))),
                    Transform::from_xyz(0.0, 0.5, 0.0),
                )
            ],));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The stacking curve's shape is the feature: a real ceiling, most of the
    /// gain on the second unit, and a tenth unit that is not worth mounting.
    #[test]
    fn the_stack_curve_starts_at_one_and_converges_on_its_limit() {
        // Authority: 1.00, 1.50, 1.75, 1.90 -> 2.00.
        let authority = |n: f32| stack_curve(n, STACK_AUTHORITY_LIMIT);
        assert!(
            (authority(1.0) - 1.0).abs() < 1e-6,
            "one computer is the identity"
        );
        assert!((authority(2.0) - 1.5).abs() < 1e-6);
        assert!((authority(4.0) - 1.75).abs() < 1e-6);
        assert!((authority(10.0) - 1.9).abs() < 1e-6);
        assert!(
            authority(1000.0) < STACK_AUTHORITY_LIMIT,
            "the limit is an asymptote, never reached"
        );
        // The second computer is worth 50 times the tenth.
        let second = authority(2.0) - authority(1.0);
        let tenth = authority(10.0) - authority(9.0);
        assert!(
            (second / tenth - 45.0).abs() < 1.0,
            "second {second}, tenth {tenth}"
        );
        // Precision: 1.00, 1.25, 1.375, 1.45 -> 1.50.
        let precision = |n: f32| stack_curve(n, STACK_PRECISION_LIMIT);
        assert!((precision(1.0) - 1.0).abs() < 1e-6);
        assert!((precision(2.0) - 1.25).abs() < 1e-6);
        assert!((precision(10.0) - 1.45).abs() < 1e-6);
        // Degenerate counts stay on the identity rather than exploding.
        assert!((authority(0.0) - 1.0).abs() < 1e-6);
    }

    /// The per-rank weights ARE the curve, taken one step at a time, so a
    /// stack of identical computers spends exactly the curve's budget.
    #[test]
    fn the_authority_weights_sum_to_the_stack_curve() {
        for n in 1..=10usize {
            let summed: f32 = (0..n).map(authority_weight).sum();
            let curve = stack_curve(n as f32, STACK_AUTHORITY_LIMIT);
            assert!(
                (summed - curve).abs() < 1e-5,
                "n = {n}: weights sum to {summed}, curve says {curve}"
            );
        }
    }

    fn stack_app() -> App {
        let mut app = App::new();
        app.add_systems(FixedUpdate, update_controller_stack_tuning);
        app
    }

    fn tuning() -> ControllerSectionTuning {
        ControllerSectionTuning {
            frequency: 4.0,
            damping_ratio: 4.0,
            max_torque: 40.0,
        }
    }

    fn spawn_stack(app: &mut App, count: usize) -> (Entity, Vec<Entity>) {
        let root = app.world_mut().spawn_empty().id();
        let controllers = (0..count)
            .map(|_| {
                app.world_mut()
                    .spawn((
                        ChildOf(root),
                        ControllerSectionMarker,
                        tuning(),
                        PDController {
                            frequency: 4.0,
                            damping_ratio: 4.0,
                            max_torque: 40.0,
                        },
                    ))
                    .id()
            })
            .collect();
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
            .fold((0.0, 0.0, 0.0), |(kp, kd, torque), pd| {
                (
                    kp + pd.frequency * pd.frequency,
                    kd + pd.frequency * pd.damping_ratio,
                    torque + pd.max_torque,
                )
            })
    }

    #[test]
    fn a_lone_controller_keeps_its_authored_tuning() {
        // Every shipped hull carries exactly one computer, so the identity
        // case is the whole fleet's handling: it must not move at all.
        let mut app = stack_app();
        let (_, controllers) = spawn_stack(&mut app, 1);
        app.world_mut().run_schedule(FixedUpdate);

        let live = *app.world().get::<PDController>(controllers[0]).unwrap();
        assert_eq!(live.frequency, 4.0);
        assert_eq!(live.damping_ratio, 4.0);
        assert_eq!(live.max_torque, 40.0);
    }

    /// Stacking splits ONE loop rather than running several: the torque
    /// budget grows on the curve, the P gain drops by the precision curve,
    /// and the D gain - the numerically dangerous one - does not move.
    #[test]
    fn a_stack_shares_one_attitude_loop() {
        let mut app = stack_app();
        let (_, one) = spawn_stack(&mut app, 1);
        let (_, four) = spawn_stack(&mut app, 4);
        app.world_mut().run_schedule(FixedUpdate);

        let (kp_one, kd_one, torque_one) = stack_loop(&app, &one);
        let (kp_four, kd_four, torque_four) = stack_loop(&app, &four);

        assert!(
            (torque_four / torque_one - 1.75).abs() < 1e-3,
            "four computers must carry the curve's budget, got {}",
            torque_four / torque_one
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

    /// Redundancy: a stack that loses a computer re-derives itself into a
    /// smaller stack - handling degrades, the hull keeps flying.
    #[test]
    fn losing_one_controller_degrades_the_stack_instead_of_killing_it() {
        let mut app = stack_app();
        let (_, controllers) = spawn_stack(&mut app, 3);
        app.world_mut().run_schedule(FixedUpdate);
        let (_, _, three) = stack_loop(&app, &controllers);

        app.world_mut()
            .entity_mut(controllers[1])
            .insert(SectionInactiveMarker);
        app.world_mut().run_schedule(FixedUpdate);
        let survivors = [controllers[0], controllers[2]];
        let (_, _, two) = stack_loop(&app, &survivors);

        assert!(
            (three / 40.0 - 5.0 / 3.0).abs() < 1e-3,
            "three computers carry 1.667 budgets, got {}",
            three / 40.0
        );
        assert!(
            (two / 40.0 - 1.5).abs() < 1e-3,
            "the survivors must re-derive to the two-computer budget, got {}",
            two / 40.0
        );
        assert!(
            two > 40.0,
            "two survivors still out-steer a single computer"
        );
    }

    /// A weaker computer bolted onto a strong one is worth its own weight at
    /// the second rank, not the strong one's - the budget is built from each
    /// section's authored torque, in strength order.
    #[test]
    fn a_mixed_stack_ranks_by_authored_torque() {
        let mut app = stack_app();
        let root = app.world_mut().spawn_empty().id();
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
                        max_torque,
                    },
                ))
                .id()
        };
        // Weak one first, so a pass that trusted spawn order would rank wrong.
        let weak = spawn(&mut app, 10.0);
        let strong = spawn(&mut app, 40.0);
        app.world_mut().run_schedule(FixedUpdate);

        let (_, _, budget) = stack_loop(&app, &[weak, strong]);
        assert!(
            (budget - 45.0).abs() < 1e-3,
            "40 at full weight plus 10 at half, got {budget}"
        );
        assert!(
            app.world().get::<PDController>(strong).unwrap().max_torque
                > app.world().get::<PDController>(weak).unwrap().max_torque,
            "the strong computer must carry the larger share"
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
        // Arrange
        let mut app = App::new();
        let id = app
            .world_mut()
            .spawn(controller_section(ControllerSectionConfig::default()))
            .id();

        // Act
        app.update();

        // Assert
        assert!(app.world().get::<ControllerSectionMarker>(id).is_some());
    }

    #[test]
    fn spawns_controller_with_custom_scene() {
        // Arrange
        let mut app = App::new();
        let custom_scene = Handle::<WorldAsset>::default();
        let config = ControllerSectionConfig {
            render_mesh: Some(custom_scene.clone().into()),
            ..Default::default()
        };
        let id = app.world_mut().spawn(controller_section(config)).id();

        // Act
        app.update();

        // Assert
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
