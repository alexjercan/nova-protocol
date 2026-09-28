//! Camera framing: where the chase rig anchors (the live centre of mass,
//! eased across a handback) and how far back it sits - the per-mode rig, the
//! wheel zoom, the burn push, the orbit survey dolly, and the velocity lead
//! that keeps the framing speed-invariant.
//!
//! Engine units throughout: a rig offset is a Bevy transform and a lead is
//! measured against an avian velocity, so every distance below is a world
//! unit (10 m) and every speed is a world unit per second (10 m/s). Nothing
//! here is authored.

use avian3d::prelude::{ComputedCenterOfMass, LinearVelocity};
use bevy::prelude::*;
use nova_events::units::prelude::*;
use nova_gameplay::prelude::*;

use super::{
    handback::{handback_anchor_rot, CameraHandbackBlend, HANDBACK_BLEND_SECONDS},
    mode::SpaceshipCameraControlMode,
    rig::{
        SpaceshipCameraController, SpaceshipCameraInputMarker, SpaceshipRotationInputActiveMarker,
    },
    zoom::ChaseZoom,
};
use crate::prelude::*;

pub(super) fn update_chase_camera_input(
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

    // Anchor on the live center of mass, not the root origin: a camera anchored
    // at the origin makes a section-stripped wreck appear to orbit an empty
    // point in space. The COM lift lives in the shared helper so aim, lock
    // cones and the camera agree on the anchor. Every real ship root has a
    // `RigidBody`, which requires the component; the None fallback is defensive
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

/// Chase smoothing for the gameplay camera modes (`ChaseCamera::smoothing`;
/// 0.0 = bolted on). Gives the camera weight: it trails the hull into and out
/// of maneuvers instead of teleporting with it. Deliberate default from the
/// flight-feel retune.
pub(super) const CAMERA_SMOOTHING: f32 = 0.15;

/// Seconds of velocity lead that cancel the chase lerp's steady-state lag at
/// the given smoothing and frame delta. `lerp_and_snap` keeps `r =
/// (smoothing^7)^dt` of the remaining error each frame, so a camera tracking an
/// anchor that advances `v * dt` per frame settles `v * dt * r / (1 - r)`
/// BEHIND its rig position - about 20 u at 300 u/s and 60 fps with the shipped
/// 0.15 (the "camera zooms out too much at speed" was never a designed zoom).
/// Leading the camera offset by exactly this cancels the lag; the focus stays
/// on the true anchor, so framing is speed-invariant and the steady camera
/// distance is the RIG distance at any cruise speed - the cap the playtest
/// asked for, by construction. (The discrete form, not the continuous tau =
/// -1/(7 ln s): at 60 fps the difference is a visible 2.4 u overshoot at 300
/// u/s.)
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

/// How far outside a hull's own physical envelope
/// ([`HullEnvelopeRadius`]) the camera stands.
///
/// The envelope is the collider reach; the visible skin, its greebles and its
/// running lights live in the gap, and a camera parked on that surface frames
/// plating instead of a ship. A FIXED metric gap, the same one the HUD shells
/// keep: a clearance that scaled with hull size would put the carrier's camera
/// a kilometre out for the same picture.
pub const CAMERA_HULL_CLEARANCE: Meters = Meters(5.0);

/// The fraction of the live rig distance a fully extended burn push moves the
/// camera back (anchor-frame -Z, away from the hull).
///
/// A fraction and not a distance, because the rig distance is already what
/// tracks hull size: the old fixed 30 m was a lurch behind a 49 m skiff and
/// nothing behind a 195 m carrier. Flat: throttle, heat and acceleration do
/// not scale it, so a modulated brake cannot breathe the zoom.
const BURN_PUSH_RIG_FRACTION: f32 = 0.15;

/// How long the forward main-drive command must hold without a break to engage
/// the burn push, and stay quiet without a break to release it, seconds of
/// fixed (virtual) time. A Space tap and the autopilot's brake pulses and orbit
/// micro-burns end inside it; a sustained burn outlasts it.
const BURN_PUSH_DEBOUNCE_SECONDS: f32 = 0.5;

/// The chase camera's burn push, on the player ship root: whether the camera
/// leans back, how long [`MainDriveCommanded`] has disagreed with that without
/// a break, and how far the lean has eased toward it.
///
/// Stepped in `FixedUpdate` by [`update_burn_push`], on the same clock as the
/// command, so pause freezes it and the render rate cannot change the timing.
/// Required by [`MainDriveCommanded`], so a respawned hull starts disengaged.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq)]
pub(super) struct BurnPush {
    /// The camera eases toward a lean of [`BURN_PUSH_RIG_FRACTION`] of the
    /// rig.
    engaged: bool,
    /// Contiguous seconds the command has disagreed with `engaged`.
    disagreeing: f32,
    /// The share of the lean applied, 0 to 1. Spools toward `engaged` at the
    /// thruster spool rates, so a flip of `engaged` moves the camera target
    /// continuously instead of a whole rig fraction in one tick.
    extension: f32,
}

impl BurnPush {
    /// One fixed tick of `dt` seconds with the forward drive `commanded` or
    /// not, spooling the extension at `up_rate` and `down_rate` (1/s). Any
    /// tick that agrees with the current state restarts the count.
    ///
    /// Flips on the tick nearest the debounce: half a tick of slack, so an
    /// `f32` sum of `dt` that lands a hair short of it (fifteen 1/30 s ticks)
    /// cannot add a tick at any fixed rate. The extension spools toward the
    /// state after the flip, so it starts moving on the flip tick.
    fn step(self, commanded: bool, dt: f32, up_rate: f32, down_rate: f32) -> Self {
        let (engaged, disagreeing) = if commanded == self.engaged {
            (self.engaged, 0.0)
        } else {
            let disagreeing = self.disagreeing + dt;
            if disagreeing + 0.5 * dt >= BURN_PUSH_DEBOUNCE_SECONDS {
                (commanded, 0.0)
            } else {
                (self.engaged, disagreeing)
            }
        };
        let target = if engaged { 1.0 } else { 0.0 };
        Self {
            engaged,
            disagreeing,
            extension: crate::flight::spool(self.extension, target, up_rate, down_rate, dt),
        }
    }
}

/// Debounces the player's forward main-drive command into the camera's
/// [`BurnPush`] and eases its extension at the [`FlightSettings`] spool rates,
/// once per fixed tick after the flight layer writes the command.
///
/// The spool rates only pace the camera: the live thruster inputs, throttle and
/// heat never reach it.
pub(super) fn update_burn_push(
    time: Res<Time>,
    settings: Res<FlightSettings>,
    mut q_ship: Query<(&MainDriveCommanded, &mut BurnPush), With<PlayerSpaceshipMarker>>,
) {
    let dt = time.delta_secs();
    for (commanded, mut push) in &mut q_ship {
        *push = push.step(
            **commanded,
            dt,
            settings.spool_up_rate,
            settings.spool_down_rate,
        );
    }
}

/// Survey dolly while parked in orbit: the camera distance grows to this
/// multiple of the planned ring radius, so the orbited body, the ring and the
/// surrounding area read as a whole instead of the hull filling the screen.
/// Playtest knob.
const SURVEY_RING_FACTOR: f32 = 1.4;

/// Cap on how far BEYOND the hull's cleared rig the survey dolly may go, world
/// units, so a giant well cannot push the camera out to where the scene is
/// specks. Measured from the hull's own envelope plus
/// [`CAMERA_HULL_CLEARANCE`], not from the anchor: a flat cap that ignored hull
/// size would dolly a carrier barely clear of its own stern. Playtest knob.
const SURVEY_MAX_DISTANCE: f32 = 250.0;

/// Each control mode's AUTHORED camera composition: `(offset, focus_offset)`,
/// world units, framed on a small craft. Normal is the standard chase view,
/// Turret is closer and elevated with its focus pushed ahead so the hull leaves
/// the combat area clear, FreeLook is the wider view.
///
/// These are compositions, not distances: a hull too big to fit inside one
/// grows it uniformly ([`hull_clearance_scale`]) rather than reframing it, so
/// every ship is shot the same way at whatever size it needs.
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

/// How much the authored composition has to grow for the camera to stand
/// [`CAMERA_HULL_CLEARANCE`] outside a hull whose envelope is `envelope` world
/// units.
///
/// Never below 1.0: the mode rigs ARE the framing, so a hull that already fits
/// inside one is shot exactly as it is shipped today, and only a hull that
/// would swallow the camera moves it. Uniform, so the composition - the lift,
/// the lead ahead, the angle - survives the growth.
fn hull_clearance_scale(base_offset: Vec3, envelope: f32) -> f32 {
    let base = base_offset.length();
    if base <= f32::EPSILON {
        return 1.0;
    }
    ((envelope + CAMERA_HULL_CLEARANCE.to_engine()) / base).max(1.0)
}

/// The hull-cleared rig for `mode` on a hull of `envelope` world units, dollied
/// out to `zoom` times its distance: the authored composition grown to clear
/// the hull, with the gameplay smoothing. The zoom moves only the camera; the
/// focus stays where the composition puts it.
///
/// [`update_camera_rig`] composes the survey dolly, the burn push and the
/// velocity lead onto this every frame; the controller observer stamps it
/// with the session zoom, so a camera inserted into a big hull opens OUTSIDE it
/// instead of wearing a cutter-sized rig for its first frame.
pub(super) fn spaceship_camera_rig(
    mode: &SpaceshipCameraControlMode,
    envelope: f32,
    zoom: f32,
) -> ChaseCamera {
    let (offset, focus_offset) = mode_camera_rig(mode);
    let scale = hull_clearance_scale(offset, envelope);
    ChaseCamera {
        offset: offset * scale * zoom,
        focus_offset: focus_offset * scale,
        smoothing: CAMERA_SMOOTHING,
    }
}

/// Where the chase camera stands the moment it opens on a hull of `envelope`
/// world units at the session `zoom` level, anchored at `anchor` and facing
/// `facing`.
///
/// The SAME composition [`spaceship_camera_rig`] gives, resolved to a pose:
/// this is the rig's own settled answer, not an approximation of it, so the
/// frame the scenario opens on is the frame the chase camera would have eased
/// to. A scenario spawns its camera before the player hull exists, and a fixed
/// pose there opened a big hull from inside its own plate and a distant player
/// on empty space.
///
/// Engine units, like everything else in this module: `envelope` is a world
/// unit reach and the returned transform is a Bevy transform.
pub fn chase_camera_opening_pose(
    anchor: Vec3,
    facing: Quat,
    envelope: f32,
    zoom: f32,
) -> Transform {
    let rig = spaceship_camera_rig(&SpaceshipCameraControlMode::Normal, envelope, zoom);
    // `chase_camera_update_state_system`'s frame, written out: the rig's own Z
    // counts FORWARD, so a rig standing behind the hull has a negative one.
    let behind = |offset: Vec3| facing * Vec3::new(offset.x, offset.y, -offset.z);
    Transform::from_translation(anchor + behind(rig.offset))
        .looking_at(anchor + behind(rig.focus_offset), facing * Vec3::Y)
}

/// The survey dolly scale for the current autopilot state: while parked
/// in a PLANNED orbit the mode offset stretches so the camera distance
/// reaches `plan.radius * SURVEY_RING_FACTOR` (capped, never closer than
/// the mode's own hull-cleared rig) - the ring radius IS the area to
/// visualize, so the dolly adapts to the orbit scale. 1.0 (no dolly)
/// everywhere else, including the plan-less first orbit tick. Both bounds
/// carry the hull: `base_len` is already the cleared rig, and the cap is
/// measured out from the same cleared distance. Pure for unit testing.
fn survey_scale(action: Option<&AutopilotAction>, base_len: f32, envelope: f32) -> f32 {
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
        .min(envelope + CAMERA_HULL_CLEARANCE.to_engine() + SURVEY_MAX_DISTANCE)
        .max(base_len)
        / base_len
}

/// Applies the whole camera rig, every frame: `offset = mode rig * hull
/// clearance * (wheel zoom or survey dolly) + burn push`, the mode's focus
/// offset, and the gameplay smoothing. Per-frame ownership (not on mode change) is
/// load-bearing: player death removes `ChaseCamera` and respawn re-inserts one,
/// so anything applied only on `mode.is_changed()` is silently lost after the
/// first life. The hull clearance grows the composition until the camera stands
/// outside the live [`HullEnvelopeRadius`] - without it the Turret rig parks
/// 10 u back inside a hull that reaches 19.5 u. The burn push is the debounced
/// [`BurnPush`], so autopilot burns push too, and the smoothing eases the
/// camera out and home. In
/// FreeLook/Turret the offset lives in the mouse-rig frame, so the push is a
/// dolly-out rather than a hull-frame lean; acceptable juice either way. The
/// survey dolly (engaged ORBIT) applies in Normal and FreeLook but NOT Turret -
/// a fight while orbiting should not be fought from survey range - and rides
/// the same per-frame smoothing as everything else, so engage and breakout ease
/// exactly like a mode switch instead of snapping. The wheel zoom rides that
/// smoothing as well; inside a planned orbit the wheel overrides the survey
/// distance ([`ChaseZoom`]).
pub(super) fn update_camera_rig(
    time: Res<Time>,
    mode: Res<SpaceshipCameraControlMode>,
    mut zoom: ResMut<ChaseZoom>,
    camera: Single<(&mut ChaseCamera, &ChaseCameraInput), With<SpaceshipCameraController>>,
    spaceship: Single<
        (
            Entity,
            Option<&LinearVelocity>,
            Option<&HullEnvelopeRadius>,
            &BurnPush,
        ),
        (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>),
    >,
    q_autopilot: Query<&Autopilot>,
) {
    let (ship, ship_velocity, envelope, burn_push) = spaceship.into_inner();
    let (mut camera, camera_input) = camera.into_inner();

    let envelope = envelope.map_or(0.0, |envelope| **envelope);
    let (base_offset, focus_offset) = mode_camera_rig(&mode);
    let hull_scale = hull_clearance_scale(base_offset, envelope);
    let base_offset = base_offset * hull_scale;
    let focus_offset = focus_offset * hull_scale;
    let action = q_autopilot.get(ship).ok().map(|a| &a.action);
    let planned_orbit = matches!(action, Some(AutopilotAction::Orbit { plan: Some(_), .. }));
    let scale = zoom.drain_scale(
        matches!(*mode, SpaceshipCameraControlMode::Turret),
        planned_orbit.then(|| survey_scale(action, base_offset.length(), envelope)),
    );
    // Velocity lead: cancel the chase lerp's steady-state lag (see
    // chase_lag_tau) so the camera holds the rig distance at any cruise speed.
    // Expressed in the anchor rotation frame because the chase rig re-rotates
    // the offset by anchor_rot; the offset convention is world = rot * (x, y, -z),
    // hence the z sign flip. The lead moves only the CAMERA - focus_offset
    // stays untouched, so the look-at point (and the ship's framing) is
    // identical at every speed.
    let world_lead = ship_velocity.map(|v| v.0).unwrap_or(Vec3::ZERO)
        * chase_lag_lead_seconds(CAMERA_SMOOTHING, time.delta_secs());
    let local_lead = camera_input.anchor_rot.inverse() * world_lead;
    let offset_lead = Vec3::new(local_lead.x, local_lead.y, -local_lead.z);

    let rig = base_offset * scale;
    let push = rig.length() * BURN_PUSH_RIG_FRACTION * burn_push.extension;
    camera.offset = rig + Vec3::new(0.0, 0.0, -push) + offset_lead;
    camera.focus_offset = focus_offset;
    camera.smoothing = CAMERA_SMOOTHING;
}

#[cfg(test)]
mod tests {
    use super::{super::CameraAuthorityPlugin, *};

    /// The chase anchor is the ship's live center of mass, not the root
    /// origin: the origin is where the first sections were built and never
    /// moves, so after those sections are destroyed a tumbling ship anchored
    /// there appears to orbit an empty point in space.
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

    /// A fully extended burn push leans the camera back by the flat rig
    /// fraction, a partial one by its share of it, and a retracted one returns
    /// it exactly to the mode's base rig. Also covers the respawn case: the rig
    /// (including smoothing) lands on a factory-fresh `ChaseCamera` with no
    /// mode change ever happening, as after a player death re-insert.
    #[test]
    fn an_engaged_burn_push_leans_back_a_flat_rig_fraction_and_returns_home() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(ChaseCameraPlugin);
        app.init_resource::<SpaceshipCameraControlMode>();
        app.init_resource::<ChaseZoom>();
        app.add_systems(Update, update_camera_rig);

        let ship = app
            .world_mut()
            .spawn((
                SpaceshipRootMarker,
                PlayerSpaceshipMarker,
                Transform::default(),
                BurnPush::default(),
            ))
            .id();
        let camera = app.world_mut().spawn(SpaceshipCameraController).id();

        let (base, focus) = mode_camera_rig(&SpaceshipCameraControlMode::Normal);

        // Disengaged, no mode change ever: the full rig - offset, focus and
        // the weight-giving smoothing - lands on the default ChaseCamera.
        app.update();
        let chase = app.world().get::<ChaseCamera>(camera).unwrap();
        assert_eq!(chase.offset, base);
        assert_eq!(chase.focus_offset, focus);
        assert_eq!(chase.smoothing, CAMERA_SMOOTHING);

        // Fully extended: pushed straight back by the whole rig fraction.
        *app.world_mut().get_mut::<BurnPush>(ship).unwrap() = BurnPush {
            engaged: true,
            disagreeing: 0.0,
            extension: 1.0,
        };
        app.update();
        let pushed = app.world().get::<ChaseCamera>(camera).unwrap().offset;
        assert_eq!(
            pushed,
            base + Vec3::new(0.0, 0.0, -base.length() * BURN_PUSH_RIG_FRACTION)
        );

        // Half extended: half the rig fraction.
        app.world_mut().get_mut::<BurnPush>(ship).unwrap().extension = 0.5;
        app.update();
        let easing = app.world().get::<ChaseCamera>(camera).unwrap().offset;
        assert_eq!(
            easing,
            base + Vec3::new(0.0, 0.0, -base.length() * BURN_PUSH_RIG_FRACTION * 0.5)
        );

        // Released: the camera comes home, not to a drifted base.
        *app.world_mut().get_mut::<BurnPush>(ship).unwrap() = BurnPush::default();
        app.update();
        assert_eq!(app.world().get::<ChaseCamera>(camera).unwrap().offset, base);
    }

    /// Every mode's composition survives a hull it already fits, and grows -
    /// uniformly, direction intact - around one it does not. The shipped
    /// Turret rig sits 11.2 u back; the carrier's collider envelope reaches
    /// 19.5 u, so without this the combat camera is inside the ship.
    #[test]
    fn every_mode_rig_clears_the_live_hull_envelope() {
        const CARRIER_ENVELOPE: f32 = 19.53;
        const SKIFF_ENVELOPE: f32 = 4.89;

        for mode in [
            SpaceshipCameraControlMode::Normal,
            SpaceshipCameraControlMode::FreeLook,
            SpaceshipCameraControlMode::Turret,
        ] {
            let (authored, authored_focus) = mode_camera_rig(&mode);

            // A hull that fits keeps the shipped framing exactly.
            let skiff = spaceship_camera_rig(&mode, SKIFF_ENVELOPE, 1.0);
            assert_eq!(skiff.offset, authored, "{mode:?} reframed a small hull");
            assert_eq!(skiff.focus_offset, authored_focus);
            assert_eq!(skiff.smoothing, CAMERA_SMOOTHING);

            // A hull that does not fit grows the rig past its envelope plus
            // the visual clearance, without turning the camera.
            let carrier = spaceship_camera_rig(&mode, CARRIER_ENVELOPE, 1.0);
            assert!(
                carrier.offset.length()
                    >= CARRIER_ENVELOPE + CAMERA_HULL_CLEARANCE.to_engine() - 1e-4,
                "{mode:?} parks the camera inside the carrier: {}",
                carrier.offset.length()
            );
            assert!(
                carrier.offset.normalize().dot(authored.normalize()) > 0.9999,
                "{mode:?} reframed instead of growing"
            );
            // The focus rides the same scale, so the lead ahead of the hull
            // stays in proportion to the distance behind it.
            let scale = carrier.offset.length() / authored.length();
            assert!(
                (carrier.focus_offset - authored_focus * scale).length() < 1e-3,
                "{mode:?} grew the offset but not the focus"
            );
        }
    }

    /// One shipped fixed tick, 64 Hz.
    const TICK: f32 = 1.0 / 64.0;

    /// Steps `push` through `commands`, one fixed tick each, and reports the
    /// engaged state after every tick.
    fn run_burn_push(mut push: BurnPush, commands: impl IntoIterator<Item = bool>) -> Vec<bool> {
        let settings = FlightSettings::default();
        commands
            .into_iter()
            .map(|commanded| {
                push = push.step(
                    commanded,
                    TICK,
                    settings.spool_up_rate,
                    settings.spool_down_rate,
                );
                push.engaged
            })
            .collect()
    }

    /// A Space tap shorter than the debounce never pushes the camera, during
    /// the tap or after its release.
    #[test]
    fn a_short_main_drive_tap_never_engages_the_burn_push() {
        let tap = std::iter::repeat_n(true, 31).chain(std::iter::repeat_n(false, 128));
        assert!(!run_burn_push(BurnPush::default(), tap).contains(&true));
    }

    /// Autopilot brake pulses and orbit micro-burns break the command before it
    /// has held for the debounce, so the camera never leans for them.
    #[test]
    fn pulsed_brakes_and_orbit_micro_burns_never_engage_the_burn_push() {
        let pulse = |on: usize, off: usize, repeats: usize| {
            (0..repeats)
                .flat_map(move |_| {
                    std::iter::repeat_n(true, on).chain(std::iter::repeat_n(false, off))
                })
                .collect::<Vec<_>>()
        };
        // A modulated brake: mostly lit, one cold tick every 30.
        assert!(!run_burn_push(BurnPush::default(), pulse(29, 1, 20)).contains(&true));
        // Station-keeping: a few lit ticks every second.
        assert!(!run_burn_push(BurnPush::default(), pulse(3, 61, 20)).contains(&true));
    }

    /// A sustained command engages on the tick it has held for the debounce, a
    /// gap shorter than the debounce keeps the push, and a quiet as long as the
    /// debounce releases it.
    #[test]
    fn a_sustained_burn_engages_and_only_a_sustained_quiet_releases_it() {
        let ticks = (BURN_PUSH_DEBOUNCE_SECONDS / TICK) as usize;
        assert_eq!(ticks, 32, "the shipped rate counts 32 ticks");

        let engaged = run_burn_push(BurnPush::default(), std::iter::repeat_n(true, ticks));
        assert!(!engaged[..ticks - 1].contains(&true), "engaged early");
        assert!(engaged[ticks - 1], "a held half second engages");

        let lit = BurnPush {
            engaged: true,
            disagreeing: 0.0,
            extension: 1.0,
        };
        let gap = std::iter::repeat_n(false, ticks - 1)
            .chain(std::iter::once(true))
            .chain(std::iter::repeat_n(false, ticks - 1));
        assert!(
            !run_burn_push(lit, gap).contains(&false),
            "a short gap released the push"
        );

        let released = run_burn_push(lit, std::iter::repeat_n(false, ticks));
        assert!(!released[..ticks - 1].contains(&false), "released early");
        assert!(!released[ticks - 1], "a quiet half second releases");
    }

    /// At every fixed rate the debounce flips on the tick that completes half
    /// a second, and the camera never jumps: from the engage tick the
    /// extension rises as `1 - e^(-up_rate t)`, from the release tick it falls
    /// as `e^(-down_rate t)`, and no tick moves it by more than one spool step.
    /// The ship has no thruster, so throttle and heat cannot pace it.
    #[test]
    fn the_burn_push_flips_on_the_debounce_tick_and_eases_at_any_fixed_rate() {
        use bevy::time::TimeUpdateStrategy;

        let settings = FlightSettings::default();
        for (hz, debounce_ticks) in [(30.0, 15), (64.0, 32), (120.0, 60)] {
            let mut app = App::new();
            app.add_plugins(MinimalPlugins);
            let fixed = Time::<Fixed>::from_hz(hz);
            let step = fixed.timestep();
            let dt = step.as_secs_f32();
            app.insert_resource(fixed);
            app.insert_resource(TimeUpdateStrategy::ManualDuration(step));
            app.insert_resource(settings.clone());
            app.add_systems(FixedUpdate, update_burn_push);
            let ship = app
                .world_mut()
                .spawn((
                    PlayerSpaceshipMarker,
                    MainDriveCommanded(true),
                    BurnPush::default(),
                ))
                .id();
            let max_step = 1.0 - (-settings.spool_up_rate.max(settings.spool_down_rate) * dt).exp();
            // The first update's delta is zero; after it every update runs
            // exactly one fixed tick.
            app.update();

            // Two debounces of sustained burn, then two of quiet.
            let mut previous = 0.0;
            let mut before_release = 0.0;
            for commanded in [true, false] {
                app.world_mut()
                    .get_mut::<MainDriveCommanded>(ship)
                    .unwrap()
                    .0 = commanded;
                for tick in 1..=2 * debounce_ticks {
                    app.update();
                    let push = *app.world().get::<BurnPush>(ship).unwrap();
                    assert!(
                        (push.extension - previous).abs() <= max_step + 1e-6,
                        "{hz} Hz tick {tick}: extension stepped {previous} -> {}",
                        push.extension
                    );
                    if tick < debounce_ticks {
                        assert_eq!(push.engaged, !commanded, "{hz} Hz flipped at {tick}");
                        if commanded {
                            assert_eq!(push.extension, 0.0, "{hz} Hz leaned early at {tick}");
                        } else {
                            assert!(
                                push.extension >= previous,
                                "{hz} Hz retracted before the release at {tick}"
                            );
                        }
                        before_release = push.extension;
                    } else {
                        assert_eq!(push.engaged, commanded, "{hz} Hz no flip at {tick}");
                        let k = (tick - debounce_ticks + 1) as f32 * dt;
                        let expected = if commanded {
                            1.0 - (-settings.spool_up_rate * k).exp()
                        } else {
                            before_release * (-settings.spool_down_rate * k).exp()
                        };
                        assert!(
                            (push.extension - expected).abs() < 1e-4,
                            "{hz} Hz tick {tick}: extension {} != {expected}",
                            push.extension
                        );
                    }
                    previous = push.extension;
                }
            }
            // Half a second after the release the lean is within 1% of home.
            assert!(previous < 0.01, "{hz} Hz still leans: {previous}");
        }
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
        let scale = survey_scale(Some(&orbit(100.0)), base, 0.0);
        assert!((scale * base - 100.0 * SURVEY_RING_FACTOR).abs() < 1e-3);
        // ...capped for giant wells...
        let capped = survey_scale(Some(&orbit(1000.0)), base, 0.0);
        assert!(
            (capped * base - (CAMERA_HULL_CLEARANCE.to_engine() + SURVEY_MAX_DISTANCE)).abs()
                < 1e-3
        );
        // ...with the cap measured out from the HULL, so a carrier surveys
        // from as far beyond its own stern as a cutter does.
        let carrier = survey_scale(Some(&orbit(1000.0)), base, 19.53);
        assert!(
            (carrier * base - (19.53 + CAMERA_HULL_CLEARANCE.to_engine() + SURVEY_MAX_DISTANCE))
                .abs()
                < 1e-3
        );
        // ...and never dollies IN on a tiny ring.
        assert_eq!(survey_scale(Some(&orbit(5.0)), base, 0.0), 1.0);

        // No dolly without a planned orbit: manual flight, other verbs,
        // the plan-less first orbit tick.
        assert_eq!(survey_scale(None, base, 0.0), 1.0);
        assert_eq!(survey_scale(Some(&AutopilotAction::Stop), base, 0.0), 1.0);
        assert_eq!(
            survey_scale(
                Some(&AutopilotAction::Orbit {
                    well: Entity::PLACEHOLDER,
                    plan: None,
                }),
                base,
                0.0,
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
        app.init_resource::<ChaseZoom>();
        app.add_systems(Update, update_camera_rig);

        let ship = app
            .world_mut()
            .spawn((
                SpaceshipRootMarker,
                PlayerSpaceshipMarker,
                BurnPush::default(),
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

    /// Normal and FreeLook share the wheel level, FreeLook stays farther out,
    /// and the zoom moves only the camera, never the focus. Turret keeps its
    /// authored combat rig at any level.
    #[test]
    fn the_wheel_zoom_dollies_normal_and_free_look_but_not_turret() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(ChaseCameraPlugin);
        app.init_resource::<SpaceshipCameraControlMode>();
        app.insert_resource(ChaseZoom {
            manual: 4.0,
            ..default()
        });
        app.add_systems(Update, update_camera_rig);
        app.world_mut().spawn((
            SpaceshipRootMarker,
            PlayerSpaceshipMarker,
            BurnPush::default(),
            Transform::default(),
        ));
        let camera = app.world_mut().spawn(SpaceshipCameraController).id();

        let mut rig_in = |mode: SpaceshipCameraControlMode| {
            *app.world_mut().resource_mut::<SpaceshipCameraControlMode>() = mode;
            app.update();
            let chase = app.world().get::<ChaseCamera>(camera).unwrap();
            (chase.offset, chase.focus_offset)
        };

        let (normal_base, normal_focus) = mode_camera_rig(&SpaceshipCameraControlMode::Normal);
        let (normal, focus) = rig_in(SpaceshipCameraControlMode::Normal);
        assert_eq!(normal, normal_base * 4.0);
        assert_eq!(focus, normal_focus);

        let (free_base, _) = mode_camera_rig(&SpaceshipCameraControlMode::FreeLook);
        let (free, _) = rig_in(SpaceshipCameraControlMode::FreeLook);
        assert_eq!(free, free_base * 4.0);
        assert!(
            free.length() > normal.length(),
            "FreeLook stays farther out"
        );

        let (turret_base, _) = mode_camera_rig(&SpaceshipCameraControlMode::Turret);
        assert_eq!(rig_in(SpaceshipCameraControlMode::Turret).0, turret_base);
    }

    /// The camera must hold its RIG framing at any cruise speed. The chase lerp
    /// settles v * tau behind a moving anchor (22 u at 300 u/s - the playtest's
    /// "camera zooms out too much, pivot too far behind"); the rig's velocity
    /// lead cancels it, so the ship's position in CAMERA space (what the player
    /// sees) is the same at 300 u/s as at walking pace. Uses the real
    /// update_camera_rig; before the lead this differed by ~20 u.
    #[test]
    fn camera_framing_is_speed_invariant() {
        use avian3d::prelude::*;
        use nova_gameplay::test_support::{settle, unfinished_integrity_physics_app};

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
            app.add_plugins((ChaseCameraPlugin, CameraAuthorityPlugin));
            app.init_resource::<SpaceshipCameraControlMode>();
            app.init_resource::<ChaseZoom>();
            app.add_systems(Update, (drive_camera_input, update_camera_rig).chain());
            app.finish();

            let ship = app
                .world_mut()
                .spawn((
                    CruisingShip,
                    PlayerSpaceshipMarker,
                    BurnPush::default(),
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
}
