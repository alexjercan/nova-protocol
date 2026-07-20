use avian3d::prelude::{ComputedCenterOfMass, LinearVelocity, Rotation};
use bevy::{prelude::*, transform::TransformSystems};
use bevy_common_systems::prelude::*;
use bevy_enhanced_input::prelude::*;

use crate::prelude::*;

pub mod prelude {
    pub use super::{
        ActiveLookRay, NovaCameraSystems, SpaceshipCameraControlMode, SpaceshipCameraController,
        SpaceshipCameraControllerPlugin, SpaceshipCameraFreeLookInputMarker,
        SpaceshipCameraInputMarker, SpaceshipCameraNormalInputMarker,
        SpaceshipCameraTurretInputMarker, SpaceshipRotationInputActiveMarker, WeaponsRaised,
    };
}

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct NovaCameraSystems;

pub struct SpaceshipCameraControllerPlugin;

impl Plugin for SpaceshipCameraControllerPlugin {
    fn build(&self, app: &mut App) {
        debug!("SpaceshipCameraControllerPlugin: build");

        app.init_resource::<SpaceshipCameraControlMode>();
        app.add_input_context::<PlayerInputMarker>();

        app.add_observer(insert_camera_controller);
        app.add_observer(insert_camera_freelook);
        app.add_observer(insert_camera_turret);
        app.add_observer(insert_player_input);
        app.add_observer(destroy_camera_controller);

        app.add_observer(on_autopilot_disengaged);

        app.add_observer(on_rotation_input);
        app.add_observer(on_rotation_input_completed);

        app.register_type::<WeaponsRaised>();

        app.add_systems(
            Update,
            // Fully chained: the mode (and raised flag) is derived from the
            // held inputs first, then the rig system owns every ChaseCamera
            // field and must run after the mode switch (whose markers decide
            // the rig) AND after the input write, because its velocity lead
            // is expressed in this frame's anchor rotation frame.
            (
                derive_control_mode_and_raised,
                update_chase_camera_input,
                sync_spaceship_control_mode,
                update_camera_rig,
            )
                .chain()
                .in_set(NovaCameraSystems),
        );

        // bcs moves the camera Transform in PostUpdate but leaves its order
        // against Bevy's transform propagation AMBIGUOUS - if propagation
        // wins the race, the frame renders with LAST frame's camera pose (a
        // per-build coin flip; task 20260710-231928). Pin it from nova via
        // the exported set so the rendered camera is always this frame's.
        app.configure_sets(
            PostUpdate,
            ChaseCameraSystems::Sync.before(TransformSystems::Propagate),
        );
    }
}

/// Marker component to identify the camera controller for the player's spaceship.
///
/// This should be added to an entity that also has a `ChaseCamera` component.
#[derive(Component, Debug, Clone, Reflect)]
#[require(ChaseCamera)]
pub struct SpaceshipCameraController;

/// The mode that the camera is currently in for controlling the spaceship.
///
/// Derived each frame from the HELD state of the mode inputs (Turret while
/// RMB/CombatInput is held, else FreeLook while Alt/FreeLookInput is held,
/// else Normal - task 20260713-082324). Memoryless by design: any
/// press/release order in any nesting lands on the right mode, which the old
/// four last-writer-wins observers could not guarantee (Alt-release while RMB
/// was held used to stomp the mode back to Normal). `PartialEq` +
/// `set_if_neq` keep `is_changed()` meaningful for the rig-sync system.
#[derive(Resource, Default, Clone, Debug, PartialEq, Eq)]
pub enum SpaceshipCameraControlMode {
    #[default]
    Normal,
    FreeLook,
    Turret,
}

/// Weapons-raised: the gameplay-facing flag for "the player is holding the
/// combat stance" (RMB/CombatInput held), derived each frame onto the PLAYER
/// ship root alongside the camera mode. Gameplay consumers (the radar slot
/// latch, the weapons safety, manual turret aim - deliberate-radar spike
/// 20260713-082207) read THIS component, never the camera enum: the enum is a
/// camera concern, and routing gameplay off it is the round-3 M2 bug class.
/// Living on the ship root means a respawn starts lowered for free.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Default, Reflect)]
#[reflect(Component)]
pub struct WeaponsRaised(pub bool);

/// The live look ray: the [`PointRotationOutput`] of whichever camera rig
/// currently holds [`SpaceshipRotationInputActiveMarker`] - Normal, FreeLook
/// or Turret. Consumers that need "where is the player looking RIGHT NOW"
/// (the targeting picker, the radar) read this instead of pinning a specific
/// rig, whose output freezes the moment its mode is left (the frozen-ray bug,
/// task 20260713-082324).
///
/// Press-frame property: on the frame a mode transition begins, the active
/// marker still sits on the OUTGOING rig (marker moves are command-flushed
/// after the sync system), so this accessor is the live look at press time.
#[derive(bevy::ecs::system::SystemParam)]
pub struct ActiveLookRay<'w, 's> {
    query: Query<
        'w,
        's,
        &'static PointRotationOutput,
        (
            With<SpaceshipCameraInputMarker>,
            With<SpaceshipRotationInputActiveMarker>,
        ),
    >,
}

impl ActiveLookRay<'_, '_> {
    /// The active rig's rotation, or `None` when no rig exists (menu states,
    /// headless tests without a camera).
    pub fn rotation(&self) -> Option<Quat> {
        self.query.iter().next().map(|output| **output)
    }

    /// The active look direction (unit vector), if a rig exists.
    pub fn direction(&self) -> Option<Vec3> {
        self.rotation()
            .map(|rotation| (rotation * Vec3::NEG_Z).normalize())
    }
}

/// General Marker for the rotation input of the spaceship camera.
#[derive(Component, Debug, Clone)]
pub struct SpaceshipCameraInputMarker;

/// Marker for the rotation input of the spaceship camera in normal mode.
#[derive(Component, Debug, Clone)]
pub struct SpaceshipCameraNormalInputMarker;

/// Marker for the rotation input of the spaceship camera in free look mode.
#[derive(Component, Debug, Clone)]
pub struct SpaceshipCameraFreeLookInputMarker;

/// Marker for the rotation input of the spaceship camera in turret mode.
#[derive(Component, Debug, Clone)]
pub struct SpaceshipCameraTurretInputMarker;

#[derive(Component, Debug, Clone)]
pub struct SpaceshipRotationInputActiveMarker;

fn insert_camera_controller(
    add: On<Add, SpaceshipCameraController>,
    mut commands: Commands,
    q_camera: Query<Entity, With<SpaceshipCameraController>>,
) {
    let entity = add.entity;
    trace!("insert_camera_controller: entity {:?}", entity);

    let Ok(camera) = q_camera.get(entity) else {
        error!(
            "insert_camera_controller: entity {:?} not found in q_camera",
            add.entity
        );
        return;
    };

    commands
        .entity(camera)
        .insert(ChaseCamera::default())
        // A fresh controller starts blend-free: a stale handback blend
        // surviving a death/respawn path that skipped the teardown would
        // play a wrong 0.45s swing on the first frame of the new life.
        .remove::<CameraHandbackBlend>()
        .with_children(|parent| {
            parent.spawn((
                SpaceshipCameraInputMarker,
                SpaceshipCameraNormalInputMarker,
                SpaceshipRotationInputActiveMarker,
                PointRotation::default(),
            ));
        });
}

fn insert_camera_freelook(
    add: On<Add, SpaceshipCameraController>,
    mut commands: Commands,
    q_camera: Query<Entity, (With<ChaseCamera>, With<SpaceshipCameraController>)>,
) {
    let entity = add.entity;
    trace!("insert_camera_controller: entity {:?}", entity);

    let Ok(camera) = q_camera.get(entity) else {
        error!(
            "insert_camera_controller: entity {:?} not found in q_camera",
            entity
        );
        return;
    };

    commands.entity(camera).with_children(|parent| {
        parent.spawn((
            SpaceshipCameraInputMarker,
            SpaceshipCameraFreeLookInputMarker,
            PointRotation::default(),
        ));
    });
}

fn insert_camera_turret(
    add: On<Add, SpaceshipCameraController>,
    mut commands: Commands,
    q_camera: Query<Entity, (With<ChaseCamera>, With<SpaceshipCameraController>)>,
) {
    let entity = add.entity;
    trace!("insert_camera_turret: entity {:?}", entity);

    let Ok(camera) = q_camera.get(entity) else {
        error!(
            "insert_camera_turret: entity {:?} not found in q_camera",
            entity
        );
        return;
    };

    commands.entity(camera).with_children(|parent| {
        parent.spawn((
            SpaceshipCameraInputMarker,
            SpaceshipCameraTurretInputMarker,
            PointRotation::default(),
        ));
    });
}

fn insert_player_input(
    add: On<Add, SpaceshipCameraController>,
    mut commands: Commands,
    q_camera: Query<Entity, (With<ChaseCamera>, With<SpaceshipCameraController>)>,
) {
    let entity = add.entity;
    trace!("insert_camera_turret: entity {:?}", entity);

    let Ok(camera) = q_camera.get(entity) else {
        error!(
            "insert_player_input: entity {:?} not found in q_camera",
            entity
        );
        return;
    };

    // Spawn a player input controller entity to hold the input from the player
    commands.entity(camera).with_children(|parent| {
        parent.spawn((
            Name::new("Player Input Controller"),
            PlayerInputMarker,
            actions!(
                PlayerInputMarker[
                    (
                        Name::new("Input: Camera Rotate"),
                        Action::<CameraInputRotate>::new(),
                        Bindings::spawn((
                            // Bevy requires single entities to be wrapped in `Spawn`.
                            // You can attach modifiers to individual bindings as well.
                            Spawn((Binding::mouse_motion(), Scale::splat(0.001), Negate::all())),
                            Axial::right_stick().with((Scale::splat(2.0), Negate::none())),
                        )),
                    ),
                    (
                        Name::new("Input: Free Look Mode"),
                        Action::<FreeLookInput>::new(),
                        bindings![KeyCode::AltLeft, GamepadButton::LeftTrigger],
                    ),
                    (
                        Name::new("Input: Combat Mode"),
                        Action::<CombatInput>::new(),
                        bindings![MouseButton::Right, GamepadButton::LeftTrigger2],
                    ),
                ]
            ),
        ));
    });
}

fn destroy_camera_controller(
    remove: On<Remove, SpaceshipCameraController>,
    mut commands: Commands,
    q_camera: Query<&Children, With<ChaseCamera>>,
) {
    let entity = remove.entity;
    trace!("destroy_camera_controller: entity {:?}", entity);

    let Ok(children) = q_camera.get(entity) else {
        error!(
            "destroy_camera_controller: entity {:?} not found in q_camera",
            entity
        );
        return;
    };

    for child in children.iter() {
        commands.entity(child).try_despawn();
    }

    commands
        .entity(entity)
        .try_remove::<(ChaseCamera, SpaceshipCameraController, CameraHandbackBlend)>();
}

/// How long the camera takes to swing from its autopilot free-look
/// direction onto the re-seeded manual rig after a disengage, seconds.
/// The re-seed itself is instant (the PD's no-lurch contract); only the
/// camera's view of that discontinuity is eased. Playtest knob.
const HANDBACK_BLEND_SECONDS: f32 = 0.45;

/// Blends the camera's anchor rotation across the autopilot-to-manual
/// re-seed discontinuity (task 20260710-222517): from the free-look
/// direction the camera held at disengage toward the live rig output,
/// over `HANDBACK_BLEND_SECONDS`. Lives on the camera controller
/// entity; `update_chase_camera_input` ticks and removes it.
#[derive(Component, Debug, Clone, Reflect)]
pub struct CameraHandbackBlend {
    /// The rig output the camera was following when the autopilot let go.
    pub from: Quat,
    /// Seconds since the handback.
    pub elapsed: f32,
}

/// The blended anchor rotation `elapsed` seconds into a handback:
/// smoothstep-eased slerp from the held direction to the live rig. Pure
/// for unit testing.
fn handback_anchor_rot(from: Quat, to: Quat, elapsed: f32) -> Quat {
    let t = (elapsed / HANDBACK_BLEND_SECONDS.max(1e-3)).clamp(0.0, 1.0);
    let eased = t * t * (3.0 - 2.0 * t);
    from.slerp(to, eased)
}

/// When an autopilot maneuver disengages, re-seed the normal rotation rig
/// from the ship's *current* attitude. While engaged the mouse kept turning
/// the rig (as camera free-look) but the hull followed the maneuver, so the
/// rig's quat is stale; without this re-seed the PD would violently swing the
/// ship back to wherever the rig last pointed. Re-inserting `PointRotation`
/// resets its internal state, exactly like the free-look mode switches do.
///
/// The re-seed is instant for the SHIP (no-lurch contract) but a visible
/// snap for the CAMERA, whose anchor follows the rig quat - so when the
/// normal rig is the active one, the camera gets a [`CameraHandbackBlend`]
/// seeded with the direction it was actually looking, and eases onto the
/// new rig like a mode switch instead of teleporting (the mode switches
/// are smooth precisely because they re-seed from the CURRENT output;
/// this is the one re-seed that cannot, so the camera bridges it).
fn on_autopilot_disengaged(
    remove: On<Remove, Autopilot>,
    mut commands: Commands,
    q_ship: Query<&Rotation, With<PlayerSpaceshipMarker>>,
    q_rig: Query<
        (
            Entity,
            Option<&PointRotationOutput>,
            Has<SpaceshipRotationInputActiveMarker>,
        ),
        With<SpaceshipCameraNormalInputMarker>,
    >,
    q_camera: Query<Entity, With<SpaceshipCameraController>>,
) {
    let Ok(rotation) = q_ship.get(remove.entity) else {
        // Not the player's ship - nothing to re-seed. (A despawning ship
        // still passes this guard - Remove observers see sibling
        // components during the despawn flush - so a dead ship's camera
        // briefly carries a blend; the controller teardown removes it,
        // and a fresh controller clears any stale one defensively.)
        return;
    };

    for (rig, output, active) in &q_rig {
        // Bridge the camera only when this rig is the one it follows (in
        // FreeLook/Turret the normal rig is dormant, so its re-seed only
        // shows on the later switch back to Normal - a pre-existing
        // transition this task does not change). A re-disengage mid-blend restarts from the rig's
        // pre-reseed output, not the mid-blend display value - a small
        // pop in a rare double-handback, accepted for a stateless
        // observer.
        if active {
            if let Some(output) = output {
                for camera in &q_camera {
                    commands.entity(camera).try_insert(CameraHandbackBlend {
                        from: **output,
                        elapsed: 0.0,
                    });
                }
            }
        }
        commands.entity(rig).try_insert(PointRotation {
            initial_rotation: rotation.0,
        });
    }
}

fn update_chase_camera_input(
    mut commands: Commands,
    time: Res<Time>,
    camera: Single<
        (
            Entity,
            &mut ChaseCameraInput,
            Option<&mut CameraHandbackBlend>,
        ),
        (With<ChaseCamera>, With<SpaceshipCameraController>),
    >,
    spaceship: Single<
        (&Transform, Option<&ComputedCenterOfMass>),
        (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>),
    >,
    point_rotation: Single<
        &PointRotationOutput,
        (
            With<SpaceshipCameraInputMarker>,
            With<SpaceshipRotationInputActiveMarker>,
        ),
    >,
) {
    let (camera_entity, mut camera_input, blend) = camera.into_inner();
    let (spaceship_transform, center_of_mass) = spaceship.into_inner();
    let point_rotation = point_rotation.into_inner();

    // Anchor on the live center of mass, not the root origin: a camera
    // anchored at the origin makes a section-stripped wreck appear to orbit
    // an empty point in space (task 20260709-140620). The COM lift lives in
    // the shared helper so aim, lock cones and the camera agree on the
    // anchor (task 20260709-150711). Every real ship root has a `RigidBody`,
    // which requires the component; the None fallback is defensive
    // (marker-only roots in tests).
    camera_input.anchor_pos =
        crate::sections::live_structure_anchor(spaceship_transform, center_of_mass);

    // An in-flight handback eases the anchor from the direction the
    // camera held at disengage onto the live rig; mouse motion during the
    // blend moves the live target, so it converges to wherever the player
    // is looking. Everywhere else the rig drives directly.
    let live = **point_rotation;
    camera_input.anchor_rot = match blend {
        Some(mut blend) => {
            blend.elapsed += time.delta_secs();
            if blend.elapsed >= HANDBACK_BLEND_SECONDS {
                commands
                    .entity(camera_entity)
                    .remove::<CameraHandbackBlend>();
                live
            } else {
                handback_anchor_rot(blend.from, live, blend.elapsed)
            }
        }
        None => live,
    };
}

/// Chase smoothing for the gameplay camera modes (bcs
/// `ChaseCamera::smoothing`; 0.0 = bolted on). Gives the camera weight: it
/// trails the hull into and out of maneuvers instead of teleporting with it.
/// Deliberate default from the flight-feel retune (task 20260709-095043).
const CAMERA_SMOOTHING: f32 = 0.15;

/// Seconds of velocity lead that cancel the chase lerp's steady-state lag
/// at the given smoothing and frame delta. bcs `lerp_and_snap` keeps
/// `r = (smoothing^7)^dt` of the remaining error each frame, so a camera
/// tracking an anchor that advances `v * dt` per frame settles
/// `v * dt * r / (1 - r)` BEHIND its rig position - about 20 u at 300 u/s
/// and 60 fps with the shipped 0.15 (task 20260711-121711: the "camera
/// zooms out too much at speed" was never a designed zoom). Leading the
/// camera offset by exactly this cancels the lag; the focus stays on the
/// true anchor, so framing is speed-invariant and the steady camera
/// distance is the RIG distance at any cruise speed - the cap the playtest
/// asked for, by construction. (The discrete form, not the continuous
/// tau = -1/(7 ln s): at 60 fps the difference is a visible 2.4 u
/// overshoot at 300 u/s.)
fn chase_lag_lead_seconds(smoothing: f32, dt: f32) -> f32 {
    if smoothing <= 0.0 || smoothing >= 1.0 || dt <= 0.0 {
        // A rigid camera has no lag; a smoothing of 1.0 never converges and
        // has no finite lead either - both degenerate to no compensation.
        return 0.0;
    }
    let remaining = smoothing.powi(7).powf(dt);
    if remaining >= 1.0 - f32::EPSILON {
        return 0.0;
    }
    dt * remaining / (1.0 - remaining)
}

/// How far the camera is pushed back (anchor-frame -Z, away from the hull) at
/// full main-drive burn, world units. Driven by the spooled thruster input,
/// so the push ramps with the engines - lighting up leans the camera back,
/// spool-down eases it home even after the key is released.
const BURN_PUSH_DISTANCE: f32 = 3.0;

/// Survey dolly while parked in orbit (task 20260710-222518): the camera
/// distance grows to this multiple of the planned ring radius, so the
/// orbited body, the ring and the surrounding area read as a whole
/// instead of the hull filling the screen. Playtest knob.
const SURVEY_RING_FACTOR: f32 = 1.4;

/// Cap on the survey dolly distance, world units, so a giant well cannot
/// push the camera out to where the scene is specks. Playtest knob.
const SURVEY_MAX_DISTANCE: f32 = 250.0;

/// Each control mode's camera rig: `(offset, focus_offset)`. One source of
/// truth for the mode-switch system and the per-frame burn push, so the push
/// composes onto the mode's base instead of fighting it.
fn mode_camera_rig(mode: &SpaceshipCameraControlMode) -> (Vec3, Vec3) {
    match mode {
        SpaceshipCameraControlMode::Normal => {
            (Vec3::new(0.0, 5.0, -20.0), Vec3::new(0.0, 0.0, 20.0))
        }
        SpaceshipCameraControlMode::FreeLook => (Vec3::new(0.0, 10.0, -30.0), Vec3::ZERO),
        SpaceshipCameraControlMode::Turret => {
            (Vec3::new(0.0, 5.0, -10.0), Vec3::new(0.0, 0.0, 50.0))
        }
    }
}

/// The survey dolly scale for the current autopilot state: while parked
/// in a PLANNED orbit the mode offset stretches so the camera distance
/// reaches `plan.radius * SURVEY_RING_FACTOR` (capped, never closer than
/// the mode's own rig) - the ring radius IS the area to visualize, so
/// the dolly adapts to the orbit scale. 1.0 (no dolly) everywhere else,
/// including the plan-less first orbit tick. Pure for unit testing.
fn survey_scale(action: Option<&AutopilotAction>, base_len: f32) -> f32 {
    let Some(AutopilotAction::Orbit {
        plan: Some(plan), ..
    }) = action
    else {
        return 1.0;
    };
    if base_len <= f32::EPSILON {
        return 1.0;
    }
    // min-then-max, not clamp: f32::clamp panics when min > max, and both
    // bounds are playtest knobs - a knob turn (or a future rig longer than
    // the cap) must degrade to "no dolly", not a per-frame panic.
    (plan.radius * SURVEY_RING_FACTOR)
        .min(SURVEY_MAX_DISTANCE)
        .max(base_len)
        / base_len
}

/// Applies the whole camera rig, every frame: `offset = mode rig * survey
/// dolly + spooled main-drive heat * BURN_PUSH_DISTANCE`, the mode's focus
/// offset, and the gameplay smoothing. Per-frame ownership (not on mode
/// change) is load-bearing: player death removes `ChaseCamera` and respawn
/// re-inserts a default (smoothing 0.0), so anything applied only on
/// `mode.is_changed()` is silently lost after the first life. Heat is the
/// hottest live forward-mounted thruster - the flight layer's main-drive
/// definition - so autopilot burns push too, and spool-down eases the
/// camera home. In FreeLook/Turret the offset lives in the mouse-rig
/// frame, so the push is a dolly-out rather than a hull-frame lean;
/// acceptable juice either way. The survey dolly (engaged ORBIT) applies
/// in Normal and FreeLook but NOT Turret - a fight while orbiting should
/// not be fought from survey range - and rides the same per-frame
/// smoothing as everything else, so engage and breakout ease exactly like
/// a mode switch instead of snapping.
fn update_camera_rig(
    time: Res<Time>,
    mode: Res<SpaceshipCameraControlMode>,
    camera: Single<(&mut ChaseCamera, &ChaseCameraInput), With<SpaceshipCameraController>>,
    spaceship: Single<
        (Entity, Option<&LinearVelocity>),
        (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>),
    >,
    q_autopilot: Query<&Autopilot>,
    q_thruster: Query<
        (&ThrusterSectionInput, &Transform, &ChildOf),
        (With<ThrusterSectionMarker>, Without<SectionInactiveMarker>),
    >,
) {
    let (ship, ship_velocity) = spaceship.into_inner();
    let (mut camera, camera_input) = camera.into_inner();

    let mut heat = 0.0f32;
    for (input, transform, &ChildOf(parent)) in &q_thruster {
        if parent != ship {
            continue;
        }
        let local_dir = transform.rotation.mul_vec3(Vec3::NEG_Z).normalize();
        if crate::flight::is_forward_aligned(local_dir, Vec3::NEG_Z) {
            heat = heat.max(**input);
        }
    }

    // Max heat, not a sum: the push reads "engines are lit", and one small
    // engine at full burn is lit; authority-weighted push is a playtest knob.
    let (base_offset, focus_offset) = mode_camera_rig(&mode);
    let scale = if matches!(*mode, SpaceshipCameraControlMode::Turret) {
        1.0
    } else {
        survey_scale(
            q_autopilot.get(ship).ok().map(|a| &a.action),
            base_offset.length(),
        )
    };
    // Velocity lead: cancel the chase lerp's steady-state lag (see
    // chase_lag_tau) so the camera holds the rig distance at any cruise
    // speed. Expressed in the anchor rotation frame because bcs re-rotates
    // the offset by anchor_rot; the bcs offset convention is
    // world = rot * (x, y, -z), hence the z sign flip. The lead moves only
    // the CAMERA - focus_offset stays untouched, so the look-at point (and
    // the ship's framing) is identical at every speed.
    let world_lead = ship_velocity.map(|v| v.0).unwrap_or(Vec3::ZERO)
        * chase_lag_lead_seconds(CAMERA_SMOOTHING, time.delta_secs());
    let local_lead = camera_input.anchor_rot.inverse() * world_lead;
    let offset_lead = Vec3::new(local_lead.x, local_lead.y, -local_lead.z);

    camera.offset = base_offset * scale
        + Vec3::new(0.0, 0.0, -BURN_PUSH_DISTANCE * heat.clamp(0.0, 1.0))
        + offset_lead;
    camera.focus_offset = focus_offset;
    camera.smoothing = CAMERA_SMOOTHING;
}

fn sync_spaceship_control_mode(
    mut commands: Commands,
    mode: Res<SpaceshipCameraControlMode>,
    _spaceship: Single<&Transform, (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>)>,
    // The OUTGOING rig: the marker still sits on the rig being left this
    // frame (marker moves below are command-flushed), so its output is the
    // live look at transition time - the seed for the incoming rig. Seeding
    // unconditionally from the NORMAL rig was the round-3 B3 bug: raising out
    // of FreeLook snapped the aim to wherever the normal rig last pointed
    // instead of the flanker being looked at (task 20260713-082324).
    active_output: Query<
        &PointRotationOutput,
        (
            With<SpaceshipCameraInputMarker>,
            With<SpaceshipRotationInputActiveMarker>,
        ),
    >,
    spaceship_input_rotation: Single<Entity, With<SpaceshipCameraNormalInputMarker>>,
    spaceship_input_free_look: Single<Entity, With<SpaceshipCameraFreeLookInputMarker>>,
    spaceship_input_turret: Single<Entity, With<SpaceshipCameraTurretInputMarker>>,
) {
    if !mode.is_changed() {
        return;
    }

    let seed = active_output
        .iter()
        .next()
        .map(|output| **output)
        .unwrap_or_default();
    let spaceship_input_rotation = spaceship_input_rotation.into_inner();
    let spaceship_input_free_look = spaceship_input_free_look.into_inner();
    let spaceship_input_combat = spaceship_input_turret.into_inner();

    match *mode {
        // The NORMAL rig is deliberately never re-seeded on return: it drives
        // the SHIP's PD rotation, and seeding it from a free-look/turret
        // direction would steer the hull to wherever the player was looking.
        SpaceshipCameraControlMode::Normal => {
            commands
                .entity(spaceship_input_rotation)
                .insert(SpaceshipRotationInputActiveMarker);
            commands
                .entity(spaceship_input_free_look)
                .remove::<SpaceshipRotationInputActiveMarker>();
            commands
                .entity(spaceship_input_combat)
                .remove::<SpaceshipRotationInputActiveMarker>();
        }
        SpaceshipCameraControlMode::FreeLook => {
            commands
                .entity(spaceship_input_rotation)
                .remove::<SpaceshipRotationInputActiveMarker>();
            commands
                .entity(spaceship_input_free_look)
                .insert(PointRotation {
                    initial_rotation: seed,
                })
                .insert(SpaceshipRotationInputActiveMarker);
            commands
                .entity(spaceship_input_combat)
                .remove::<SpaceshipRotationInputActiveMarker>();
        }
        SpaceshipCameraControlMode::Turret => {
            commands
                .entity(spaceship_input_rotation)
                .remove::<SpaceshipRotationInputActiveMarker>();
            commands
                .entity(spaceship_input_free_look)
                .remove::<SpaceshipRotationInputActiveMarker>();
            commands
                .entity(spaceship_input_combat)
                .insert(PointRotation {
                    initial_rotation: seed,
                })
                .insert(SpaceshipRotationInputActiveMarker);
        }
    }
    // The ChaseCamera fields themselves (offset/focus/smoothing) are owned by
    // `update_camera_rig`, chained after this system - never re-inserted (an
    // insert would fire bcs's observer and reset the anchor to the origin for
    // a frame, the visible snap this system's history fixed), and never
    // written only-on-change (a respawned camera would lose them, R1.1).
}

#[derive(Component, Debug, Clone)]
struct PlayerInputMarker;

#[derive(InputAction)]
#[action_output(Vec2)]
struct CameraInputRotate;

#[derive(InputAction)]
#[action_output(bool)]
struct FreeLookInput;

#[derive(InputAction)]
#[action_output(bool)]
struct CombatInput;

fn on_rotation_input(
    fire: On<Fire<CameraInputRotate>>,
    mut q_input: Query<
        &mut PointRotationInput,
        (
            With<SpaceshipCameraInputMarker>,
            With<SpaceshipRotationInputActiveMarker>,
        ),
    >,
    q_rcs: Query<(), (With<PlayerSpaceshipMarker>, With<RcsActive>)>,
    pause: Res<State<crate::PauseStates>>,
) {
    // Observers bypass system-set gating; freeze intent changes while the
    // pause overlay is up (review R1.1). Releases stay ungated so held keys
    // clear cleanly during a pause.
    if *pause.get() == crate::PauseStates::Paused {
        return;
    }

    // While RCS fine-adjust is held the mouse is repurposed to translation
    // (spike Q4), so it must not orbit the camera either. ZERO the rig rate
    // rather than merely skipping the write: `point_rotation_update_system`
    // integrates the rate every frame, so a stale nonzero value left over from a
    // mouse that was moving at the moment SHIFT was pressed would keep drifting
    // the view. Held at zero, the rig quat stays at the frozen heading, so the
    // helm resumes on exit without a snap (no re-seed, unlike the autopilot).
    if !q_rcs.is_empty() {
        for mut input in &mut q_input {
            **input = Vec2::ZERO;
        }
        return;
    }

    for mut input in &mut q_input {
        **input = fire.value;
    }
}

fn on_rotation_input_completed(
    _: On<Complete<CameraInputRotate>>,
    mut q_input: Query<&mut PointRotationInput, With<SpaceshipCameraInputMarker>>,
) {
    for mut input in &mut q_input {
        **input = Vec2::ZERO;
    }
}

/// Whether a held bool action currently fires, read from its action entity's
/// state (the `cycle_modifier_held` pattern - a plain Down-conditioned action
/// reports `Fired` while its key is held).
fn action_held<A: InputAction>(q: &Query<&TriggerState, With<Action<A>>>) -> bool {
    q.iter().any(|&state| state == TriggerState::Fired)
}

/// Derive the camera control mode AND the weapons-raised flag from the HELD
/// state of the mode inputs, each frame: Turret while CombatInput is held
/// (priority), else FreeLook while FreeLookInput is held, else Normal.
/// Replaces the four last-writer-wins observers (task 20260713-082324):
/// memoryless, so nested holds (Alt during RMB, either release order) always
/// land on the right mode, and a press+release entirely inside a pause leaves
/// no trace - the state after unpause is a function of what is held NOW.
/// Deliberately not pause-gated, like the camera chain it heads: the mode is
/// a camera concern, and every gameplay consumer of [`WeaponsRaised`] is
/// pause-gated itself.
fn derive_control_mode_and_raised(
    mut commands: Commands,
    mut mode: ResMut<SpaceshipCameraControlMode>,
    q_combat: Query<&TriggerState, With<Action<CombatInput>>>,
    q_free_look: Query<&TriggerState, With<Action<FreeLookInput>>>,
    mut q_ship: Query<
        (Entity, Option<&mut WeaponsRaised>),
        (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>),
    >,
) {
    let combat_held = action_held(&q_combat);
    let next = if combat_held {
        SpaceshipCameraControlMode::Turret
    } else if action_held(&q_free_look) {
        SpaceshipCameraControlMode::FreeLook
    } else {
        SpaceshipCameraControlMode::Normal
    };
    mode.set_if_neq(next);

    // The raised flag mirrors the combat hold onto the player ship root
    // (self-healing insert: a fresh ship starts lowered and gains the flag on
    // its first frame).
    for (ship, raised) in &mut q_ship {
        match raised {
            Some(mut raised) => {
                raised.set_if_neq(WeaponsRaised(combat_held));
            }
            None => {
                commands.entity(ship).insert(WeaponsRaised(combat_held));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn handback_anchor_rot_eases_from_held_to_live() {
        let from = Quat::IDENTITY;
        let to = Quat::from_rotation_y(std::f32::consts::FRAC_PI_2);

        // Endpoints: holds the old view at t=0, lands on the live rig at
        // the duration (and stays there past it).
        // Epsilon note: slerp endpoint noise passes through acos in
        // angle_between and reads as ~7e-4 rad even for "equal" quats;
        // 2e-3 is still far below anything visible.
        assert!(handback_anchor_rot(from, to, 0.0).angle_between(from) < 2e-3);
        assert!(handback_anchor_rot(from, to, HANDBACK_BLEND_SECONDS).angle_between(to) < 2e-3);
        assert!(
            handback_anchor_rot(from, to, HANDBACK_BLEND_SECONDS * 2.0).angle_between(to) < 2e-3
        );

        // Monotonic ease: progress toward the target never reverses.
        let mut last = 0.0f32;
        for i in 0..=10 {
            let elapsed = HANDBACK_BLEND_SECONDS * (i as f32 / 10.0);
            let progress =
                to.angle_between(from) - handback_anchor_rot(from, to, elapsed).angle_between(to);
            assert!(
                progress >= last - 1e-4,
                "ease reversed at step {i}: {progress} < {last}"
            );
            last = progress;
        }
    }

    /// The autopilot handback keeps the camera continuous: at the
    /// disengage frame the anchor still points where the camera was
    /// looking (NOT the hull attitude the rig was re-seeded to), and the
    /// blend converges onto the live rig and removes itself.
    #[test]
    fn handback_blends_the_anchor_instead_of_snapping() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(ChaseCameraPlugin);
        app.add_observer(on_autopilot_disengaged);
        app.add_systems(Update, update_chase_camera_input);

        let held = Quat::IDENTITY;
        let hull = Quat::from_rotation_y(std::f32::consts::FRAC_PI_2);

        let ship = app
            .world_mut()
            .spawn((
                SpaceshipRootMarker,
                PlayerSpaceshipMarker,
                Transform::default(),
                Rotation(hull),
                Autopilot::engage(AutopilotAction::Stop),
            ))
            .id();
        let rig = app
            .world_mut()
            .spawn((
                SpaceshipCameraInputMarker,
                SpaceshipCameraNormalInputMarker,
                SpaceshipRotationInputActiveMarker,
                PointRotationOutput::default(),
            ))
            .id();
        let camera = app.world_mut().spawn(SpaceshipCameraController).id();
        app.update();

        // Disengage: the observer re-seeds the rig to the hull attitude
        // (the ship-side no-lurch contract, asserted below) and bridges
        // the camera with a blend from the held view.
        app.world_mut().entity_mut(ship).remove::<Autopilot>();
        let reseeded = app
            .world()
            .get::<PointRotation>(rig)
            .expect("the rig is re-seeded instantly");
        assert_eq!(reseeded.initial_rotation, hull);

        // Simulate bcs snapping the rig output to the re-seed (the real
        // PointRotation plugin is not in this harness).
        **app.world_mut().get_mut::<PointRotationOutput>(rig).unwrap() = hull;

        // First frame after the handback: the anchor stays on the held
        // view - the whole point - while the rig already reads the hull.
        app.update();
        let anchor = app
            .world()
            .get::<ChaseCameraInput>(camera)
            .unwrap()
            .anchor_rot;
        assert!(
            anchor.angle_between(held) < 0.05,
            "the camera must not snap: anchor is {:?} from the held view",
            anchor.angle_between(held)
        );
        assert!(anchor.angle_between(hull) > 1.0);

        // Force the blend to its end: the anchor lands on the live rig
        // and the blend removes itself.
        app.world_mut()
            .get_mut::<CameraHandbackBlend>(camera)
            .expect("blend inserted on the camera")
            .elapsed = HANDBACK_BLEND_SECONDS;
        app.update();
        let anchor = app
            .world()
            .get::<ChaseCameraInput>(camera)
            .unwrap()
            .anchor_rot;
        assert!(anchor.angle_between(hull) < 2e-3);
        assert!(
            app.world().get::<CameraHandbackBlend>(camera).is_none(),
            "a finished blend cleans itself up"
        );
    }

    /// A dormant normal rig (FreeLook/Turret active) is re-seeded without
    /// bridging the camera: its quat is invisible while another rig
    /// drives (the later switch back to Normal is a separate,
    /// pre-existing transition).
    #[test]
    fn handback_blend_only_bridges_the_active_rig() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_observer(on_autopilot_disengaged);

        let ship = app
            .world_mut()
            .spawn((
                SpaceshipRootMarker,
                PlayerSpaceshipMarker,
                Rotation(Quat::from_rotation_y(1.0)),
                Autopilot::engage(AutopilotAction::Stop),
            ))
            .id();
        // The normal rig exists but is NOT the active one.
        app.world_mut().spawn((
            SpaceshipCameraInputMarker,
            SpaceshipCameraNormalInputMarker,
            PointRotationOutput::default(),
        ));
        let camera = app.world_mut().spawn(SpaceshipCameraController).id();
        app.update();

        app.world_mut().entity_mut(ship).remove::<Autopilot>();
        assert!(
            app.world().get::<CameraHandbackBlend>(camera).is_none(),
            "no bridge for a rig the camera is not following"
        );
    }

    /// The chase anchor is the ship's live center of mass, not the root
    /// origin: the origin is where the first sections were built and never
    /// moves, so after those sections are destroyed a tumbling ship anchored
    /// there appears to orbit an empty point in space (task 20260709-140620).
    #[test]
    fn chase_anchor_tracks_the_center_of_mass() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(ChaseCameraPlugin);
        app.add_systems(Update, update_chase_camera_input);

        let position = Vec3::new(10.0, 0.0, 5.0);
        let local_com = Vec3::new(0.0, 0.0, 3.0);
        app.world_mut().spawn((
            SpaceshipRootMarker,
            PlayerSpaceshipMarker,
            Transform::from_translation(position),
            ComputedCenterOfMass(local_com),
        ));
        app.world_mut().spawn((
            SpaceshipCameraInputMarker,
            SpaceshipRotationInputActiveMarker,
            PointRotationOutput::default(),
        ));
        let camera = app.world_mut().spawn(SpaceshipCameraController).id();

        // First update initializes `ChaseCameraInput`; the second runs the
        // input system against it.
        app.update();
        app.update();

        let input = app
            .world()
            .get::<ChaseCameraInput>(camera)
            .expect("ChaseCameraInput should be initialized by the chase plugin");
        assert_eq!(input.anchor_pos, position + local_com);
    }

    /// The burn push leans the camera back with the spooled engines and eases
    /// it home when they cool - offset returns exactly to the mode's base rig
    /// (flight-feel retune, 20260709-095043). Also covers the respawn case:
    /// the rig (including smoothing) lands on a factory-fresh `ChaseCamera`
    /// with no mode change ever happening, as after a player death re-insert.
    #[test]
    fn burn_push_leans_back_and_returns_to_baseline() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(ChaseCameraPlugin);
        app.init_resource::<SpaceshipCameraControlMode>();
        app.add_systems(Update, update_camera_rig);

        let ship = app
            .world_mut()
            .spawn((
                SpaceshipRootMarker,
                PlayerSpaceshipMarker,
                Transform::default(),
            ))
            .id();
        // A main-drive thruster: section-local -Z, i.e. forward-mounted.
        let thruster = app
            .world_mut()
            .spawn((
                ChildOf(ship),
                ThrusterSectionMarker,
                ThrusterSectionInput(0.0),
                Transform::default(),
            ))
            .id();
        let camera = app.world_mut().spawn(SpaceshipCameraController).id();

        let (base, focus) = mode_camera_rig(&SpaceshipCameraControlMode::Normal);

        // Cold engines, no mode change ever: the full rig - offset, focus and
        // the weight-giving smoothing - lands on the default ChaseCamera.
        app.update();
        let chase = app.world().get::<ChaseCamera>(camera).unwrap();
        assert_eq!(chase.offset, base);
        assert_eq!(chase.focus_offset, focus);
        assert_eq!(chase.smoothing, CAMERA_SMOOTHING);

        // Full spool: pushed straight back by the full distance.
        app.world_mut()
            .get_mut::<ThrusterSectionInput>(thruster)
            .unwrap()
            .0 = 1.0;
        app.update();
        let pushed = app.world().get::<ChaseCamera>(camera).unwrap().offset;
        assert_eq!(pushed, base + Vec3::new(0.0, 0.0, -BURN_PUSH_DISTANCE));

        // Engines cold again: the camera comes home, not to a drifted base.
        app.world_mut()
            .get_mut::<ThrusterSectionInput>(thruster)
            .unwrap()
            .0 = 0.0;
        app.update();
        assert_eq!(app.world().get::<ChaseCamera>(camera).unwrap().offset, base);
    }

    #[test]
    fn survey_scale_stretches_to_the_ring_and_stays_home_otherwise() {
        let orbit = |radius: f32| AutopilotAction::Orbit {
            well: Entity::PLACEHOLDER,
            plan: Some(OrbitPlan {
                radius,
                normal: Vec3::Y,
            }),
        };
        let base = 20.0f32;

        // The dolly reaches ring * factor...
        let scale = survey_scale(Some(&orbit(100.0)), base);
        assert!((scale * base - 100.0 * SURVEY_RING_FACTOR).abs() < 1e-3);
        // ...capped for giant wells...
        let capped = survey_scale(Some(&orbit(1000.0)), base);
        assert!((capped * base - SURVEY_MAX_DISTANCE).abs() < 1e-3);
        // ...and never dollies IN on a tiny ring.
        assert_eq!(survey_scale(Some(&orbit(5.0)), base), 1.0);

        // No dolly without a planned orbit: manual flight, other verbs,
        // the plan-less first orbit tick.
        assert_eq!(survey_scale(None, base), 1.0);
        assert_eq!(survey_scale(Some(&AutopilotAction::Stop), base), 1.0);
        assert_eq!(
            survey_scale(
                Some(&AutopilotAction::Orbit {
                    well: Entity::PLACEHOLDER,
                    plan: None,
                }),
                base,
            ),
            1.0
        );
    }

    /// The survey dolly stretches the rig while parked in a planned orbit
    /// and comes home on breakout, riding the same per-frame rig path as
    /// the burn push; Turret keeps its combat rig even while orbiting.
    #[test]
    fn orbit_survey_dolly_applies_and_releases_with_the_autopilot() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(ChaseCameraPlugin);
        app.init_resource::<SpaceshipCameraControlMode>();
        app.add_systems(Update, update_camera_rig);

        let ship = app
            .world_mut()
            .spawn((
                SpaceshipRootMarker,
                PlayerSpaceshipMarker,
                Transform::default(),
            ))
            .id();
        let camera = app.world_mut().spawn(SpaceshipCameraController).id();
        let (base, _) = mode_camera_rig(&SpaceshipCameraControlMode::Normal);

        // Parked in a 100u orbit: the offset stretches along its own
        // direction to ring * factor.
        app.world_mut()
            .entity_mut(ship)
            .insert(Autopilot::engage(AutopilotAction::Orbit {
                well: Entity::PLACEHOLDER,
                plan: Some(OrbitPlan {
                    radius: 100.0,
                    normal: Vec3::Y,
                }),
            }));
        app.update();
        let offset = app.world().get::<ChaseCamera>(camera).unwrap().offset;
        assert!(
            (offset.length() - 100.0 * SURVEY_RING_FACTOR).abs() < 1e-3,
            "survey distance, got {}",
            offset.length()
        );
        assert!(
            offset.normalize().dot(base.normalize()) > 0.999,
            "the dolly stretches the rig, it does not reframe it"
        );

        // Combat while orbiting: Turret keeps its own rig.
        *app.world_mut().resource_mut::<SpaceshipCameraControlMode>() =
            SpaceshipCameraControlMode::Turret;
        app.update();
        let (turret_base, _) = mode_camera_rig(&SpaceshipCameraControlMode::Turret);
        assert_eq!(
            app.world().get::<ChaseCamera>(camera).unwrap().offset,
            turret_base
        );
        *app.world_mut().resource_mut::<SpaceshipCameraControlMode>() =
            SpaceshipCameraControlMode::Normal;

        // Breakout: the rig comes home through the same per-frame path.
        app.world_mut().entity_mut(ship).remove::<Autopilot>();
        app.update();
        assert_eq!(app.world().get::<ChaseCamera>(camera).unwrap().offset, base);
    }

    /// Switching camera mode must retune the chase offsets without resetting the anchor to the
    /// origin. Re-inserting `ChaseCamera` (the previous approach) fired bcs's insert observer,
    /// which reset `ChaseCameraInput` to the origin for a frame - the visible one-frame snap.
    #[test]
    fn switching_camera_mode_keeps_the_anchor_off_origin() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(ChaseCameraPlugin);
        app.init_resource::<SpaceshipCameraControlMode>();
        app.add_systems(
            Update,
            (sync_spaceship_control_mode, update_camera_rig).chain(),
        );

        // A player ship far from the origin, plus the input rig `sync_spaceship_control_mode`
        // drives (one active-marked normal input, a free-look input, a turret input).
        let anchor = Vec3::new(100.0, 20.0, -50.0);
        app.world_mut().spawn((
            SpaceshipRootMarker,
            PlayerSpaceshipMarker,
            Transform::from_translation(anchor),
        ));
        app.world_mut().spawn((
            SpaceshipCameraInputMarker,
            SpaceshipCameraNormalInputMarker,
            SpaceshipRotationInputActiveMarker,
            PointRotationOutput::default(),
        ));
        app.world_mut().spawn(SpaceshipCameraFreeLookInputMarker);
        app.world_mut().spawn(SpaceshipCameraTurretInputMarker);
        let camera = app.world_mut().spawn(SpaceshipCameraController).id();

        // First frame initializes `ChaseCameraInput`; set the anchor as the per-frame input
        // system (`update_chase_camera_input`) would.
        app.update();
        app.world_mut()
            .get_mut::<ChaseCameraInput>(camera)
            .expect("ChaseCameraInput should be initialized by the chase plugin")
            .anchor_pos = anchor;

        // Switch to FreeLook.
        *app.world_mut().resource_mut::<SpaceshipCameraControlMode>() =
            SpaceshipCameraControlMode::FreeLook;
        app.update();

        // The switch applied the mode rig's weight-giving smoothing.
        assert_eq!(
            app.world().get::<ChaseCamera>(camera).unwrap().smoothing,
            CAMERA_SMOOTHING,
            "mode switches must (re)apply the gameplay camera smoothing"
        );

        // The anchor survives the switch (the bug reset it to the origin for a frame)...
        assert_eq!(
            app.world()
                .get::<ChaseCameraInput>(camera)
                .unwrap()
                .anchor_pos,
            anchor,
            "switching camera mode must not reset the chase anchor to the origin"
        );
        // ...and the offsets now reflect FreeLook.
        assert_eq!(
            app.world().get::<ChaseCamera>(camera).unwrap().offset,
            Vec3::new(0.0, 10.0, -30.0)
        );
    }

    /// Disengaging the autopilot must hand the mouse a rig seeded from the
    /// hull's *current* attitude - otherwise the PD would violently swing the
    /// ship back to the rig's stale pre-maneuver command.
    #[test]
    fn disengaging_autopilot_reseeds_the_normal_rig_from_the_hull() {
        let mut app = App::new();
        app.add_observer(on_autopilot_disengaged);

        let rig = app
            .world_mut()
            .spawn((SpaceshipCameraNormalInputMarker, PointRotation::default()))
            .id();
        let attitude = Quat::from_rotation_y(1.2);
        let ship = app
            .world_mut()
            .spawn((
                SpaceshipRootMarker,
                PlayerSpaceshipMarker,
                Rotation(attitude),
                Autopilot::engage(AutopilotAction::Stop),
            ))
            .id();
        app.update();

        app.world_mut().entity_mut(ship).remove::<Autopilot>();
        app.update();

        let seeded = app.world().get::<PointRotation>(rig).unwrap();
        assert_eq!(
            seeded.initial_rotation, attitude,
            "the rig must be re-seeded from the hull attitude on disengage"
        );
    }

    /// The camera must hold its RIG framing at any cruise speed (task
    /// 20260711-121711). The chase lerp settles v * tau behind a moving
    /// anchor (22 u at 300 u/s - the playtest's "camera zooms out too
    /// much, pivot too far behind"); the rig's velocity lead cancels it,
    /// so the ship's position in CAMERA space (what the player sees) is
    /// the same at 300 u/s as at walking pace. Uses the real
    /// update_camera_rig; before the lead this differed by ~20 u.
    #[test]
    fn camera_framing_is_speed_invariant() {
        use avian3d::prelude::*;

        use crate::integrity::test_support::{settle, unfinished_integrity_physics_app};

        #[derive(Component)]
        struct CruisingShip;

        fn drive_camera_input(
            q_ship: Query<&Transform, With<CruisingShip>>,
            mut q_input: Query<&mut ChaseCameraInput>,
        ) {
            let Ok(ship) = q_ship.single() else {
                return;
            };
            for mut input in &mut q_input {
                input.anchor_pos = ship.translation;
                input.anchor_rot = Quat::IDENTITY;
            }
        }

        let converged_ship_in_camera_space = |speed: f32| -> Vec3 {
            let mut app = unfinished_integrity_physics_app();
            app.add_plugins(ChaseCameraPlugin);
            app.init_resource::<SpaceshipCameraControlMode>();
            app.add_systems(Update, (drive_camera_input, update_camera_rig).chain());
            app.configure_sets(
                PostUpdate,
                ChaseCameraSystems::Sync.before(TransformSystems::Propagate),
            );
            app.finish();

            let ship = app
                .world_mut()
                .spawn((
                    CruisingShip,
                    PlayerSpaceshipMarker,
                    RigidBody::Dynamic,
                    Transform::default(),
                    TransformInterpolation,
                    Collider::cuboid(1.0, 1.0, 1.0),
                    ColliderDensity(1.0),
                ))
                .id();
            let camera = app
                .world_mut()
                .spawn((Transform::default(), SpaceshipCameraController))
                .id();
            settle(&mut app);
            app.world_mut()
                .entity_mut(ship)
                .insert(LinearVelocity(Vec3::NEG_Z * speed));

            // Long enough for the lerp to converge at either speed.
            for _ in 0..600 {
                app.update();
            }

            let world = app.world();
            // Delivery guard: the cruise actually happened.
            let travelled = world
                .entity(ship)
                .get::<GlobalTransform>()
                .unwrap()
                .translation()
                .length();
            assert!(
                travelled > speed * 5.0,
                "the ship must actually cruise, got {travelled} at {speed} u/s"
            );
            let cam = *world.entity(camera).get::<GlobalTransform>().unwrap();
            let ship_pos = world
                .entity(ship)
                .get::<GlobalTransform>()
                .unwrap()
                .translation();
            cam.affine().inverse().transform_point3(ship_pos)
        };

        let slow = converged_ship_in_camera_space(5.0);
        let fast = converged_ship_in_camera_space(300.0);
        assert!(
            (fast - slow).length() < 0.5,
            "framing must not depend on cruise speed: slow {slow}, fast {fast}"
        );
    }

    // -- mode derivation + transition seeding (task 20260713-082324) --

    use bevy::input::InputPlugin;

    /// Distinct per-rig rotations so a seed from the WRONG rig fails loudly.
    fn rot(deg: f32) -> Quat {
        Quat::from_rotation_y(deg.to_radians())
    }

    /// A mode-derivation app with the REAL input stack (InputPlugin +
    /// EnhancedInput + the production action bindings) and FAITHFUL SPLIT
    /// RIGS - one entity per mode, only one holding the active marker, each
    /// with its own distinct PointRotationOutput (a single both-marker rig
    /// masks exactly the frozen-ray/seeding bug class this task fixes).
    /// Returns (app, normal, freelook, turret, ship).
    fn mode_app() -> (App, Entity, Entity, Entity, Entity) {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, InputPlugin, EnhancedInputPlugin));
        app.add_plugins(bevy::state::app::StatesPlugin);
        app.init_state::<crate::PauseStates>();
        app.init_resource::<SpaceshipCameraControlMode>();
        app.add_input_context::<PlayerInputMarker>();
        app.add_systems(
            Update,
            (derive_control_mode_and_raised, sync_spaceship_control_mode).chain(),
        );

        let normal = app
            .world_mut()
            .spawn((
                SpaceshipCameraInputMarker,
                SpaceshipCameraNormalInputMarker,
                SpaceshipRotationInputActiveMarker,
                PointRotation::default(),
                PointRotationOutput(rot(0.0)),
            ))
            .id();
        let freelook = app
            .world_mut()
            .spawn((
                SpaceshipCameraInputMarker,
                SpaceshipCameraFreeLookInputMarker,
                PointRotationOutput(rot(45.0)),
            ))
            .id();
        let turret = app
            .world_mut()
            .spawn((
                SpaceshipCameraInputMarker,
                SpaceshipCameraTurretInputMarker,
                PointRotationOutput(rot(90.0)),
            ))
            .id();
        let ship = app
            .world_mut()
            .spawn((
                SpaceshipRootMarker,
                PlayerSpaceshipMarker,
                Transform::IDENTITY,
            ))
            .id();
        // The context registry finalizes in App::finish, so run the plugin
        // lifecycle BEFORE spawning the action rig, like the production app
        // does (same sequencing as the wheel-routing e2e test).
        app.finish();
        app.cleanup();
        app.update();
        // The production action rig (insert_player_input's shape), so the
        // derivation reads REAL TriggerStates driven by device input.
        app.world_mut().spawn((
            PlayerInputMarker,
            actions!(
                PlayerInputMarker[
                    (
                        Action::<FreeLookInput>::new(),
                        bindings![KeyCode::AltLeft, GamepadButton::LeftTrigger]
                    ),
                    (
                        Action::<CombatInput>::new(),
                        bindings![MouseButton::Right, GamepadButton::LeftTrigger2]
                    ),
                ]
            ),
        ));
        app.update();
        (app, normal, freelook, turret, ship)
    }

    fn mode_of(app: &App) -> SpaceshipCameraControlMode {
        app.world().resource::<SpaceshipCameraControlMode>().clone()
    }

    fn active_rig(app: &mut App) -> Entity {
        let mut rigs: Vec<Entity> = app
            .world_mut()
            .query_filtered::<Entity, With<SpaceshipRotationInputActiveMarker>>()
            .iter(app.world())
            .collect();
        assert_eq!(rigs.len(), 1, "exactly one rig holds the active marker");
        rigs.pop().unwrap()
    }

    fn raised(app: &App, ship: Entity) -> bool {
        app.world()
            .get::<WeaponsRaised>(ship)
            .map(|raised| raised.0)
            .unwrap_or(false)
    }

    fn seed_of(app: &App, rig: Entity) -> Quat {
        app.world()
            .get::<PointRotation>(rig)
            .expect("rig has a PointRotation")
            .initial_rotation
    }

    fn press_rmb(app: &mut App) {
        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .press(MouseButton::Right);
    }
    fn release_rmb(app: &mut App) {
        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .release(MouseButton::Right);
    }
    fn press_alt(app: &mut App) {
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::AltLeft);
    }
    fn release_alt(app: &mut App) {
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .release(KeyCode::AltLeft);
    }

    /// The full nested-hold transition matrix: every press/release order
    /// lands on the derived mode, the marker follows, and the raised flag
    /// mirrors the combat hold. The old four last-writer-wins observers
    /// failed the "release Alt while RMB held" step (mode stomped to Normal
    /// while raised - the manual-aim-on-a-frozen-ray bug).
    #[test]
    fn nested_holds_always_land_on_the_derived_mode() {
        let (mut app, normal, freelook, turret, ship) = mode_app();
        assert!(matches!(mode_of(&app), SpaceshipCameraControlMode::Normal));
        assert_eq!(active_rig(&mut app), normal);
        assert!(!raised(&app, ship), "spawn state is lowered");

        // RMB -> Turret, raised.
        press_rmb(&mut app);
        app.update();
        assert!(matches!(mode_of(&app), SpaceshipCameraControlMode::Turret));
        assert_eq!(active_rig(&mut app), turret);
        assert!(raised(&app, ship));

        // Alt pressed WHILE RMB held: Turret has priority; nothing moves.
        press_alt(&mut app);
        app.update();
        assert!(matches!(mode_of(&app), SpaceshipCameraControlMode::Turret));
        assert_eq!(active_rig(&mut app), turret);
        assert!(raised(&app, ship));

        // RMB released while Alt held: FreeLook (NOT Normal - the old bug),
        // and lowered.
        release_rmb(&mut app);
        app.update();
        assert!(matches!(
            mode_of(&app),
            SpaceshipCameraControlMode::FreeLook
        ));
        assert_eq!(active_rig(&mut app), freelook);
        assert!(!raised(&app, ship));

        // Alt released: back to Normal.
        release_alt(&mut app);
        app.update();
        assert!(matches!(mode_of(&app), SpaceshipCameraControlMode::Normal));
        assert_eq!(active_rig(&mut app), normal);

        // The other release order: Alt first, then RMB joins, then Alt
        // releases - Turret must SURVIVE the Alt release (old bug: Normal).
        press_alt(&mut app);
        app.update();
        assert!(matches!(
            mode_of(&app),
            SpaceshipCameraControlMode::FreeLook
        ));
        press_rmb(&mut app);
        app.update();
        assert!(matches!(mode_of(&app), SpaceshipCameraControlMode::Turret));
        release_alt(&mut app);
        app.update();
        assert!(
            matches!(mode_of(&app), SpaceshipCameraControlMode::Turret),
            "releasing Alt while RMB is held must keep Turret"
        );
        assert_eq!(active_rig(&mut app), turret);
        assert!(raised(&app, ship));
        release_rmb(&mut app);
        app.update();
        assert!(matches!(mode_of(&app), SpaceshipCameraControlMode::Normal));
    }

    /// Transition seeding takes the OUTGOING rig's live output: raising out
    /// of FreeLook aims where the free look pointed (45 deg here), NOT where
    /// the normal rig last pointed (0 deg - the pre-fix source; distinct
    /// rotations make the wrong source fail). Returning to Normal never
    /// re-seeds the normal rig (it steers the SHIP).
    #[test]
    fn transitions_seed_from_the_outgoing_rig() {
        let (mut app, normal, freelook, turret, _ship) = mode_app();
        let normal_seed_before = seed_of(&app, normal);

        // Normal -> FreeLook: seeded from the normal rig's output (0 deg).
        press_alt(&mut app);
        app.update();
        assert!(seed_of(&app, freelook).angle_between(rot(0.0)) < 1e-4);

        // Simulate free-looking at a flanker: the freelook rig's LIVE output
        // moves to 45 deg (already its spawn value; make it explicit).
        app.world_mut()
            .entity_mut(freelook)
            .insert(PointRotationOutput(rot(45.0)));

        // FreeLook -> Turret (raise while free-looking): the turret rig must
        // seed from the FREELOOK output (45 deg), not the normal rig (0 deg).
        press_rmb(&mut app);
        app.update();
        assert!(
            seed_of(&app, turret).angle_between(rot(45.0)) < 1e-4,
            "raising out of FreeLook must aim at the flanker being looked at"
        );

        // Back to Normal: the normal rig is deliberately NOT re-seeded.
        release_rmb(&mut app);
        release_alt(&mut app);
        app.update();
        assert_eq!(
            seed_of(&app, normal),
            normal_seed_before,
            "the ship-steering rig must never be seeded from a look direction"
        );
    }

    /// A press+release entirely inside a pause leaves NO trace after
    /// unpause (memoryless derivation - the state is a function of what is
    /// held NOW), and a press HELD through the unpause is honored. The
    /// delivery guard is the held case: the same gesture demonstrably CAN
    /// raise, so the no-trace assertion is not vacuous.
    #[test]
    fn pause_gestures_leave_no_trace_after_unpause() {
        let (mut app, _normal, _freelook, _turret, ship) = mode_app();
        app.world_mut()
            .resource_mut::<NextState<crate::PauseStates>>()
            .set(crate::PauseStates::Paused);
        app.update();

        // Press AND release inside the pause.
        press_rmb(&mut app);
        app.update();
        release_rmb(&mut app);
        app.update();
        app.world_mut()
            .resource_mut::<NextState<crate::PauseStates>>()
            .set(crate::PauseStates::Unpaused);
        app.update();
        assert!(matches!(mode_of(&app), SpaceshipCameraControlMode::Normal));
        assert!(
            !raised(&app, ship),
            "a paused press+release leaves no trace"
        );

        // Press inside the pause, HELD through unpause: honored.
        app.world_mut()
            .resource_mut::<NextState<crate::PauseStates>>()
            .set(crate::PauseStates::Paused);
        app.update();
        press_rmb(&mut app);
        app.update();
        app.world_mut()
            .resource_mut::<NextState<crate::PauseStates>>()
            .set(crate::PauseStates::Unpaused);
        app.update();
        assert!(
            raised(&app, ship),
            "a hold surviving the pause reflects real current intent"
        );
    }

    /// While RCS fine-adjust is held, the mouse is repurposed to translation, so
    /// `on_rotation_input` must ZERO the rig rate - not merely skip - because the
    /// bcs integrator applies the rate every frame and a stale value (mouse
    /// moving at the instant SHIFT was pressed) would drift the view. Revert the
    /// fix (write `fire.value`, or early-return leaving the stale rate) and the
    /// rate stays non-zero and this fails.
    #[test]
    fn rcs_zeroes_the_rig_rate_so_the_view_does_not_drift() {
        use bevy::input::mouse::MouseMotion;

        let mut app = App::new();
        app.add_plugins((MinimalPlugins, InputPlugin, EnhancedInputPlugin));
        app.add_plugins(bevy::state::app::StatesPlugin);
        app.init_state::<crate::PauseStates>();
        app.add_input_context::<PlayerInputMarker>();
        app.add_observer(on_rotation_input);

        // The active normal rig, seeded with a NON-ZERO rate - as if the mouse
        // were moving at the moment RCS was entered.
        let rig = app
            .world_mut()
            .spawn((
                SpaceshipCameraInputMarker,
                SpaceshipCameraNormalInputMarker,
                SpaceshipRotationInputActiveMarker,
                PointRotation::default(),
                PointRotationInput(Vec2::new(0.3, -0.2)),
                PointRotationOutput(rot(0.0)),
            ))
            .id();
        // A player ship already holding RCS.
        app.world_mut()
            .spawn((SpaceshipRootMarker, PlayerSpaceshipMarker, RcsActive));

        app.finish();
        app.cleanup();
        app.update();
        // The camera rotate action, bound to mouse motion like production.
        app.world_mut().spawn((
            PlayerInputMarker,
            actions!(PlayerInputMarker[
                (
                    Action::<CameraInputRotate>::new(),
                    Bindings::spawn(Spawn((Binding::mouse_motion(), Scale::splat(1.0)))),
                ),
            ]),
        ));
        app.update();

        // Mouse moves while RCS is held.
        app.world_mut().write_message(MouseMotion {
            delta: Vec2::new(15.0, 8.0),
        });
        app.update();

        assert_eq!(
            app.world().get::<PointRotationInput>(rig).unwrap().0,
            Vec2::ZERO,
            "RCS holds the rig rate at zero so the view does not drift"
        );
    }
}
