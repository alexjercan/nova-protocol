//! Camera framing: where the chase rig anchors (the live centre of mass,
//! eased across a handback) and how far back it sits - the per-mode rig, the
//! burn push, the orbit survey dolly, and the velocity lead that keeps the
//! framing speed-invariant.

use avian3d::prelude::{ComputedCenterOfMass, LinearVelocity};
use bevy::prelude::*;

use super::{
    handback::{handback_anchor_rot, CameraHandbackBlend, HANDBACK_BLEND_SECONDS},
    mode::SpaceshipCameraControlMode,
    rig::{
        SpaceshipCameraController, SpaceshipCameraInputMarker, SpaceshipRotationInputActiveMarker,
    },
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

/// How far the camera is pushed back (anchor-frame -Z, away from the hull) at
/// full main-drive burn, world units. Driven by the spooled thruster input,
/// so the push ramps with the engines - lighting up leans the camera back,
/// spool-down eases it home even after the key is released.
const BURN_PUSH_DISTANCE: f32 = 3.0;

/// Survey dolly while parked in orbit: the camera distance grows to this
/// multiple of the planned ring radius, so the orbited body, the ring and the
/// surrounding area read as a whole instead of the hull filling the screen.
/// Playtest knob.
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
pub(super) fn update_camera_rig(
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

    camera.offset = base_offset * scale
        + Vec3::new(0.0, 0.0, -BURN_PUSH_DISTANCE * heat.clamp(0.0, 1.0))
        + offset_lead;
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

    /// The burn push leans the camera back with the spooled engines and eases
    /// it home when they cool - offset returns exactly to the mode's base rig
    /// (the flight-feel retune). Also covers the respawn case:
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

    /// The camera must hold its RIG framing at any cruise speed. The chase lerp
    /// settles v * tau behind a moving anchor (22 u at 300 u/s - the playtest's
    /// "camera zooms out too much, pivot too far behind"); the rig's velocity
    /// lead cancels it, so the ship's position in CAMERA space (what the player
    /// sees) is the same at 300 u/s as at walking pace. Uses the real
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
            app.add_plugins((ChaseCameraPlugin, CameraAuthorityPlugin));
            app.init_resource::<SpaceshipCameraControlMode>();
            app.add_systems(Update, (drive_camera_input, update_camera_rig).chain());
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
}
