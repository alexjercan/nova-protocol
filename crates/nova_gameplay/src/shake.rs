//! A Bevy plugin that adds trauma-driven camera shake as a reusable module.
//!
//! ## Overview
//!
//! `CameraShakePlugin` implements the classic "trauma" camera shake: game code
//! adds *trauma* (a `0..1` energy value) on impactful events, the trauma decays
//! back to zero over time, and while it is positive the camera is offset by a
//! random jitter whose magnitude is `amount = trauma^exponent` (squared by
//! default, so small residual energy fades out fast).
//!
//! The one bug this module exists to prevent is the *accumulating* shake: if you
//! write the jitter as `transform.translation += offset` on a camera whose base
//! position is not rewritten every frame, the offsets pile up and the camera
//! drifts off-center. This module never
//! accumulates. It applies the offset in two phases:
//!
//! - [`CameraShakeSystems::Restore`] runs *before* any base-writing driver and
//!   un-applies the previous frame's offset, so the transform is back to the
//!   driver's clean base.
//! - [`CameraShakeSystems::Apply`] runs *after* the driver and re-applies a
//!   fresh offset.
//!
//! The result is always `driver_base + offset`, whether the base comes from a
//! chase camera, a custom framing system, or nothing at all (a static camera).
//! This module names NO driver: `nova_ship`'s `CameraAuthorityPlugin` folds
//! every camera-`Transform` writer, this one included, into one total order.
//! Ordering against a specific rig from here would be an edge pointing up out
//! of the shared layer, and it was always the weaker guarantee - it silently
//! dropped whenever that rig's plugin was absent.
//!
//! The component split follows the crate convention:
//!
//! 1. [`CameraShake`] - config: decay rate, peak offset/kick, trauma exponent.
//! 2. [`CameraShakeInput`] - written by game code each frame to add trauma (or
//!    reset it).
//! 3. [`CameraShakeOutput`] - the offset/kick currently applied, for reading.
//! 4. `CameraShakeState` - private trauma + last-applied bookkeeping.
//!
//! ## Usage
//!
//! ```rust
//! # use bevy::prelude::*;
//! # use nova_gameplay::prelude::*;
//! # fn setup(mut commands: Commands) {
//! commands.spawn((
//!     Camera3d::default(),
//!     Transform::from_xyz(0.0, 0.0, 22.0),
//!     CameraShake {
//!         decay: 1.8,
//!         max_offset: Vec3::splat(0.6),
//!         ..default()
//!     },
//! ));
//! # }
//!
//! // On an impactful event, add trauma (clamped to 1.0):
//! # fn on_hit(mut input: Single<&mut CameraShakeInput>) {
//! input.add_trauma += 0.3;
//! # }
//! ```

use bevy::prelude::*;
use rand::RngExt;

/// Glob-import surface for the trauma camera-shake rig.
pub mod prelude {
    pub use super::{
        CameraShake, CameraShakeInput, CameraShakeOutput, CameraShakePlugin, CameraShakeSuppressed,
        CameraShakeSystems,
    };
}

/// Configuration for trauma-driven camera shake on a camera entity.
///
/// Add trauma through [`CameraShakeInput`]; the plugin decays it and offsets the
/// camera transform while it is positive. Peak jitter at full trauma is
/// `max_offset` (translation) and `max_kick` (rotation, radians per axis).
#[derive(Component, Debug, Clone, Reflect)]
#[require(Transform, CameraShakeInput, CameraShakeOutput, CameraShakeState)]
pub struct CameraShake {
    /// Trauma decay per second. Trauma falls linearly from its current value to
    /// zero at this rate, so `1.0 / decay` seconds is the longest a full shake
    /// can last.
    pub decay: f32,

    /// Peak positional offset at full trauma, in world units, sampled per axis.
    /// Set an axis to zero to keep the shake in a plane (e.g. `Vec3::new(x, y,
    /// 0.0)` for a 2D game).
    pub max_offset: Vec3,

    /// Peak rotational kick at full trauma, in radians, applied as an euler
    /// (pitch, yaw, roll) rotation on top of the base rotation. Defaults to
    /// [`Vec3::ZERO`] (translation-only shake).
    pub max_kick: Vec3,

    /// Exponent mapping trauma to shake amount (`amount = trauma^exponent`).
    /// The classic value is `2.0`, which makes small residual trauma fade out
    /// quickly.
    pub exponent: f32,
}

impl Default for CameraShake {
    fn default() -> Self {
        Self {
            decay: 1.8,
            max_offset: Vec3::splat(0.6),
            max_kick: Vec3::ZERO,
            exponent: 2.0,
        }
    }
}

/// Input component written by game code to drive [`CameraShake`].
///
/// Add impulse trauma by increasing `add_trauma`; the plugin consumes it each
/// frame (clamping the running trauma to `1.0`) and resets the field to zero.
/// Set `reset` to snap trauma and the current offset back to zero immediately,
/// for example when restarting a run so a lingering shake does not bleed into
/// the next scene.
#[derive(Component, Default, Debug, Reflect)]
pub struct CameraShakeInput {
    /// Trauma to add this frame. Consumed and reset to zero after it is applied.
    pub add_trauma: f32,

    /// When true, trauma and the applied offset are cleared this frame and the
    /// flag is reset to false.
    pub reset: bool,
}

/// The offset the plugin is currently applying to the camera, for reading.
///
/// Game code does not need to apply this itself; the plugin writes it to the
/// camera transform. It is exposed so systems can react to the shake (for
/// example to nudge a HUD in sympathy).
#[derive(Component, Default, Debug, Reflect)]
pub struct CameraShakeOutput {
    /// Positional offset currently added to the camera transform.
    pub offset: Vec3,

    /// Rotational kick currently composed onto the camera rotation.
    pub kick: Quat,
}

/// While present on a shaken camera, the applied offset and kick are held at
/// zero: the driver's base pose is the frame's final pose.
///
/// A mode gate, not a kill switch. The camera ENTITY persists while its
/// controller flips between rigs (Nova toggles one camera between free-fly
/// WASD and chase), so stripping [`CameraShake`] on every flip would shed and
/// rebuild shake state each time. Trauma still accrues and decays underneath:
/// combat near a suppressed camera spends its energy silently instead of
/// freezing it and replaying a stale shake the moment the marker is removed.
/// Maintained by whoever owns the camera mode (`nova_ship`'s camera authority
/// suppresses while the WASD rig drives); this module names no driver.
#[derive(Component, Default, Debug, Clone, Copy, Reflect)]
pub struct CameraShakeSuppressed;

/// Private per-camera state: trauma energy and the last-applied shake, kept so
/// [`CameraShakeSystems::Restore`] can un-apply it before drivers run.
#[derive(Component, Default, Debug, Reflect)]
struct CameraShakeState {
    /// Current trauma energy, `0..1`.
    trauma: f32,

    /// Offset applied last frame, subtracted during restore.
    last_offset: Vec3,

    /// Rotational kick applied last frame, inverted during restore.
    last_kick: Quat,
}

/// System ordering hooks for the camera shake plugin.
///
/// A base-writing driver (a chase camera, a framing system) must run *between*
/// these two sets: `Restore` un-applies the previous offset before the driver
/// rewrites the base, and `Apply` re-applies a fresh offset afterwards. The
/// Order a driver `.after(Restore).before(Apply)`, or fold it into
/// `nova_ship`'s `CameraAuthoritySystems::Solve`, which already sits between
/// them.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum CameraShakeSystems {
    /// Un-applies the previous frame's offset. Ordered before base drivers.
    Restore,
    /// Decays trauma and applies a fresh offset. Ordered after base drivers.
    Apply,
}

/// Plugin that manages trauma-driven camera shake.
///
/// Registers the [`CameraShakeSystems::Restore`] and
/// [`CameraShakeSystems::Apply`] systems in `PostUpdate`, pinned only relative
/// to each other. Placing the base-pose drivers between them is
/// `CameraAuthorityPlugin`'s job.
pub struct CameraShakePlugin;

impl Plugin for CameraShakePlugin {
    fn build(&self, app: &mut App) {
        trace!("CameraShakePlugin: build");

        app.register_type::<CameraShake>()
            .register_type::<CameraShakeInput>()
            .register_type::<CameraShakeOutput>()
            .register_type::<CameraShakeSuppressed>()
            .register_type::<CameraShakeState>();

        // NOTE: pin Restore before Apply explicitly. This is the ONE edge this
        // module owns; everything else - where the base writers sit between them -
        // is `CameraAuthorityPlugin`'s. Without this pin the two `Transform`
        // writers are unordered and the drift this module exists to prevent comes
        // back in any app that does not add the authority plugin.
        app.configure_sets(
            PostUpdate,
            CameraShakeSystems::Apply.after(CameraShakeSystems::Restore),
        );

        app.add_systems(
            PostUpdate,
            (
                camera_shake_restore_system.in_set(CameraShakeSystems::Restore),
                camera_shake_apply_system.in_set(CameraShakeSystems::Apply),
            ),
        );
    }
}

/// Decays trauma by `decay * dt`, clamped to zero so the shake settles.
fn decay_trauma(trauma: f32, decay: f32, dt: f32) -> f32 {
    (trauma - decay * dt).max(0.0)
}

/// Adds impulse trauma, clamped to the `0..1` range so the shake has a ceiling.
fn add_trauma(trauma: f32, amount: f32) -> f32 {
    (trauma + amount).clamp(0.0, 1.0)
}

/// Maps trauma to a shake amount via the configured exponent. Squaring (the
/// default `2.0`) makes small residual trauma fade out quickly.
fn shake_amount(trauma: f32, exponent: f32) -> f32 {
    trauma.max(0.0).powf(exponent)
}

/// Builds the positional offset from a shake amount, per-axis peak offset, and a
/// unit-ish random sample in `[-1, 1]` per axis.
fn shake_offset(amount: f32, max_offset: Vec3, sample: Vec3) -> Vec3 {
    max_offset * sample * amount
}

/// Builds the rotational kick from a shake amount, per-axis peak kick (radians),
/// and a random sample in `[-1, 1]` per axis.
fn shake_kick(amount: f32, max_kick: Vec3, sample: Vec3) -> Quat {
    let angles = max_kick * sample * amount;
    Quat::from_euler(EulerRot::XYZ, angles.x, angles.y, angles.z)
}

/// Un-applies the previous frame's offset and kick so the transform is back to
/// the driver's clean base before any base-writing system runs.
fn camera_shake_restore_system(
    mut q_camera: Query<(&mut Transform, &CameraShakeState), With<CameraShake>>,
) {
    for (mut transform, state) in q_camera.iter_mut() {
        trace!("camera_shake_restore_system: un-applying last offset");
        transform.translation -= state.last_offset;
        transform.rotation = state.last_kick.inverse() * transform.rotation;
    }
}

/// Consumes input, decays trauma, and applies a fresh offset and kick on top of
/// whatever base the driver wrote this frame. A [`CameraShakeSuppressed`]
/// camera goes through the same input/decay bookkeeping but applies zero, so
/// the trauma model never forks per mode.
fn camera_shake_apply_system(
    time: Res<Time>,
    mut q_camera: Query<(
        &CameraShake,
        &mut CameraShakeInput,
        &mut CameraShakeOutput,
        &mut CameraShakeState,
        &mut Transform,
        Has<CameraShakeSuppressed>,
    )>,
) {
    let dt = time.delta_secs();
    let mut rng = rand::rng();

    for (shake, mut input, mut output, mut state, mut transform, suppressed) in q_camera.iter_mut()
    {
        trace!("camera_shake_apply_system: trauma {}", state.trauma);

        // NOTE: reset is a floor, not a veto -- it clears trauma first, so a
        // same-frame `add_trauma` still lands and a game can reset and re-kick at once.
        if input.reset {
            state.trauma = 0.0;
            input.reset = false;
        }
        if input.add_trauma != 0.0 {
            state.trauma = add_trauma(state.trauma, input.add_trauma);
            input.add_trauma = 0.0;
        }

        state.trauma = decay_trauma(state.trauma, shake.decay, dt);
        let amount = shake_amount(state.trauma, shake.exponent);

        let (offset, kick) = if suppressed || amount <= 0.0 {
            (Vec3::ZERO, Quat::IDENTITY)
        } else {
            // Two INDEPENDENT samples: one sample for both would lock the
            // rotational kick to the translational offset, so the camera would
            // always roll the same way it slid.
            let mut sample = || {
                Vec3::new(
                    rng.random_range(-1.0..1.0),
                    rng.random_range(-1.0..1.0),
                    rng.random_range(-1.0..1.0),
                )
            };
            (
                shake_offset(amount, shake.max_offset, sample()),
                shake_kick(amount, shake.max_kick, sample()),
            )
        };

        transform.translation += offset;
        transform.rotation = kick * transform.rotation;

        state.last_offset = offset;
        state.last_kick = kick;
        output.offset = offset;
        output.kick = kick;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decay_reduces_trauma_and_clamps_at_zero() {
        assert!((decay_trauma(1.0, 2.0, 0.1) - 0.8).abs() < 1e-6);
        assert_eq!(decay_trauma(0.1, 2.0, 1.0), 0.0);
    }

    #[test]
    fn add_trauma_clamps_to_unit_range() {
        assert!((add_trauma(0.5, 0.3) - 0.8).abs() < 1e-6);
        assert_eq!(add_trauma(0.9, 0.5), 1.0);
        assert_eq!(add_trauma(0.2, -1.0), 0.0);
    }

    #[test]
    fn shake_amount_is_trauma_to_the_exponent() {
        assert!((shake_amount(0.5, 2.0) - 0.25).abs() < 1e-6);
        assert!((shake_amount(0.5, 1.0) - 0.5).abs() < 1e-6);
        assert_eq!(shake_amount(0.0, 2.0), 0.0);
    }

    #[test]
    fn zero_amount_yields_zero_offset() {
        assert_eq!(
            shake_offset(0.0, Vec3::splat(1.0), Vec3::splat(1.0)),
            Vec3::ZERO
        );
    }

    /// The offset scales linearly with both the amount and the per-axis peak,
    /// while a zeroed axis stays zero regardless of the sample.
    #[test]
    fn offset_scales_with_amount_and_max_offset() {
        // NOTE: 0.6 is the peak `shake_app` configures below, so the pure-math
        // and through-the-plugin tests state the same bound; 0.5 amount halves
        // it to 0.3, which is what makes the scaling readable off the numbers.
        let offset = shake_offset(0.5, Vec3::new(0.6, 0.6, 0.0), Vec3::splat(1.0));
        assert!((offset.x - 0.3).abs() < 1e-6);
        assert!((offset.y - 0.3).abs() < 1e-6);
        assert_eq!(offset.z, 0.0);
    }

    #[test]
    fn zero_kick_config_yields_identity_rotation() {
        let kick = shake_kick(1.0, Vec3::ZERO, Vec3::splat(1.0));
        assert!(kick.abs_diff_eq(Quat::IDENTITY, 1e-6));
    }

    /// Builds a minimal app with the plugin and a single shaken camera, driving
    /// `Time` by hand so the Restore/Apply pair runs deterministically.
    fn shake_app(base: Vec3) -> (App, Entity) {
        let mut app = App::new();
        app.init_resource::<Time>();
        app.add_plugins(CameraShakePlugin);
        let cam = app
            .world_mut()
            .spawn((
                Transform::from_translation(base),
                CameraShake {
                    decay: 1.8,
                    max_offset: Vec3::splat(0.6),
                    ..default()
                },
            ))
            .id();
        (app, cam)
    }

    fn step(app: &mut App, dt_ms: u64) {
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_millis(dt_ms));
        app.update();
    }

    /// The camera can never sit further from base than the diagonal of its
    /// configured per-axis peak, whatever the random sample was.
    #[test]
    fn shake_offset_stays_within_the_configured_bound() {
        let base = Vec3::new(0.0, 0.0, 22.0);
        let (mut app, cam) = shake_app(base);
        app.world_mut()
            .get_mut::<CameraShakeInput>(cam)
            .unwrap()
            .add_trauma = 1.0;

        // NOTE: 0.6 must stay in step with `shake_app`'s `max_offset`, or this
        // bound stops describing the camera under test; 1e-4 only absorbs the
        // f32 error in the diagonal.
        let bound = Vec3::splat(0.6).length() + 1e-4;
        for _ in 0..10 {
            step(&mut app, 16);
            let pos = app.world().get::<Transform>(cam).unwrap().translation;
            assert!(
                (pos - base).length() <= bound,
                "offset {} exceeded bound {}",
                (pos - base).length(),
                bound
            );
        }
    }

    /// The offset and the kick are drawn from two INDEPENDENT samples. Sharing
    /// one sample locked the rotational kick to the translational offset, so
    /// the camera always rolled the way it slid - the kick axis was parallel
    /// to the offset on every frame. Neither the bound nor the recenter test
    /// can see that, because both are true of a correlated shake too.
    #[test]
    fn the_kick_axis_is_not_locked_to_the_offset() {
        let mut app = App::new();
        app.init_resource::<Time>();
        app.add_plugins(CameraShakePlugin);
        let cam = app
            .world_mut()
            .spawn((
                Transform::default(),
                CameraShake {
                    // A uniform peak on both, so a SHARED sample would give a
                    // kick axis exactly parallel to the offset.
                    max_offset: Vec3::splat(0.6),
                    max_kick: Vec3::splat(0.3),
                    ..default()
                },
            ))
            .id();

        let mut decorrelated = 0;
        for _ in 0..50 {
            app.world_mut()
                .get_mut::<CameraShakeInput>(cam)
                .unwrap()
                .add_trauma = 1.0;
            step(&mut app, 16);

            let output = app.world().get::<CameraShakeOutput>(cam).unwrap();
            let (axis, angle) = output.kick.to_axis_angle();
            if angle.abs() < 1e-6 || output.offset.length() < 1e-6 {
                continue;
            }
            let alignment = axis.normalize().dot(output.offset.normalize()).abs();
            if alignment < 0.99 {
                decorrelated += 1;
            }
        }

        assert!(
            decorrelated > 40,
            "the kick axis tracked the offset on all but {decorrelated} of 50 frames"
        );
    }

    /// Regression for the bug this module exists to prevent: an accumulating shake
    /// that leaves the camera off-center. Kick trauma repeatedly, let it decay
    /// between kicks, and confirm the camera settles back exactly on base.
    #[test]
    fn camera_recenters_and_does_not_drift() {
        let base = Vec3::new(0.0, 0.0, 22.0);
        let (mut app, cam) = shake_app(base);

        for _ in 0..5 {
            app.world_mut()
                .get_mut::<CameraShakeInput>(cam)
                .unwrap()
                .add_trauma = 1.0;
            // NOTE: 60 frames at 16 ms is well past the full decay time
            // (1.0 / 1.8 ~= 0.56 s), so trauma is guaranteed to have settled.
            for _ in 0..60 {
                step(&mut app, 16);
            }
        }

        let pos = app.world().get::<Transform>(cam).unwrap().translation;
        assert!(
            (pos - base).length() < 1e-3,
            "camera drifted to {} from base {}",
            pos,
            base
        );
    }

    /// Marker + resource for the moving-base driver test below.
    #[derive(Resource, Default)]
    struct DriverClock(u32);

    /// A base driver (standing in for the chase camera or a framing system) rewrites
    /// the camera translation every frame *between* Restore and Apply. The shake must
    /// ride on that moving base: within the offset bound of it throughout, and exactly
    /// on it once trauma decays, with no accumulation.
    #[test]
    fn composes_with_a_moving_base_driver() {
        let mut app = App::new();
        app.init_resource::<Time>();
        app.init_resource::<DriverClock>();
        app.add_plugins(CameraShakePlugin);

        fn base_at(frame: u32) -> Vec3 {
            Vec3::new(frame as f32, 0.0, 22.0)
        }
        fn drive_base(mut clock: ResMut<DriverClock>, mut q: Query<&mut Transform>) {
            clock.0 += 1;
            for mut t in q.iter_mut() {
                t.translation = base_at(clock.0);
            }
        }
        app.add_systems(
            PostUpdate,
            drive_base
                .after(CameraShakeSystems::Restore)
                .before(CameraShakeSystems::Apply),
        );

        let cam = app
            .world_mut()
            .spawn((
                Transform::from_translation(base_at(0)),
                CameraShake {
                    decay: 1.8,
                    max_offset: Vec3::splat(0.6),
                    ..default()
                },
            ))
            .id();

        app.world_mut()
            .get_mut::<CameraShakeInput>(cam)
            .unwrap()
            .add_trauma = 1.0;
        let bound = Vec3::splat(0.6).length() + 1e-4;
        let mut last_frame = 0u32;
        for _ in 0..60 {
            step(&mut app, 16);
            last_frame = app.world().resource::<DriverClock>().0;
            let pos = app.world().get::<Transform>(cam).unwrap().translation;
            let base = base_at(last_frame);
            assert!(
                (pos - base).length() <= bound,
                "camera {} strayed from moving base {} by {}",
                pos,
                base,
                (pos - base).length()
            );
        }

        let pos = app.world().get::<Transform>(cam).unwrap().translation;
        assert!(
            (pos - base_at(last_frame)).length() < 1e-3,
            "camera {} did not settle on moving base {}",
            pos,
            base_at(last_frame)
        );
    }

    /// With a non-zero max_kick the rotation is perturbed while trauma is positive
    /// and must return to the base rotation once it decays. Guards the
    /// `last_kick.inverse()` restore order.
    #[test]
    fn kick_recenters_rotation_after_decay() {
        let base_rot = Quat::from_rotation_y(0.3);
        let mut app = App::new();
        app.init_resource::<Time>();
        app.add_plugins(CameraShakePlugin);
        let cam = app
            .world_mut()
            .spawn((
                Transform::from_rotation(base_rot),
                CameraShake {
                    decay: 1.8,
                    max_offset: Vec3::ZERO,
                    max_kick: Vec3::splat(0.2),
                    ..default()
                },
            ))
            .id();

        app.world_mut()
            .get_mut::<CameraShakeInput>(cam)
            .unwrap()
            .add_trauma = 1.0;
        for _ in 0..60 {
            step(&mut app, 16);
        }

        let rot = app.world().get::<Transform>(cam).unwrap().rotation;
        assert!(
            rot.abs_diff_eq(base_rot, 1e-3),
            "rotation {:?} did not recenter to base {:?}",
            rot,
            base_rot
        );
    }

    /// A suppressed camera must sit EXACTLY on the driver's base while trauma
    /// is live, and the trauma must keep decaying underneath: a suppression
    /// that outlives the decay window leaves nothing to replay on release.
    /// The final fresh-trauma phase is the delivery guard - the same camera
    /// demonstrably still shakes, so the stillness above is the gate working,
    /// not a dead rig.
    #[test]
    fn suppression_pins_the_camera_to_base_while_trauma_decays() {
        let base = Vec3::new(0.0, 0.0, 22.0);
        let (mut app, cam) = shake_app(base);
        app.world_mut()
            .entity_mut(cam)
            .insert(CameraShakeSuppressed);
        app.world_mut()
            .get_mut::<CameraShakeInput>(cam)
            .unwrap()
            .add_trauma = 1.0;

        // 10 frames x 16 ms = 0.16 s: trauma (decay 1.8/s) is still positive
        // on every asserted frame, so the stillness is not vacuous.
        for _ in 0..10 {
            step(&mut app, 16);
            let pos = app.world().get::<Transform>(cam).unwrap().translation;
            assert_eq!(pos, base, "suppressed shake moved the camera off base");
            assert_eq!(
                app.world().get::<CameraShakeOutput>(cam).unwrap().offset,
                Vec3::ZERO,
                "suppressed shake reported a non-zero applied offset"
            );
        }

        // Hold the suppression past the full decay window (1.0 / 1.8 ~= 0.56 s),
        // then release: the trauma drained silently, so nothing replays.
        for _ in 0..60 {
            step(&mut app, 16);
        }
        app.world_mut()
            .entity_mut(cam)
            .remove::<CameraShakeSuppressed>();
        step(&mut app, 16);
        assert_eq!(
            app.world().get::<Transform>(cam).unwrap().translation,
            base,
            "stale trauma replayed after the suppression was released"
        );

        // Delivery guard: fresh trauma on the released camera shakes again.
        let mut moved = false;
        for _ in 0..10 {
            app.world_mut()
                .get_mut::<CameraShakeInput>(cam)
                .unwrap()
                .add_trauma = 1.0;
            step(&mut app, 16);
            if app.world().get::<Transform>(cam).unwrap().translation != base {
                moved = true;
            }
        }
        assert!(moved, "the released camera never shook again");
    }

    #[test]
    fn reset_clears_trauma_immediately() {
        let base = Vec3::new(0.0, 0.0, 22.0);
        let (mut app, cam) = shake_app(base);
        app.world_mut()
            .get_mut::<CameraShakeInput>(cam)
            .unwrap()
            .add_trauma = 1.0;
        step(&mut app, 16);

        app.world_mut()
            .get_mut::<CameraShakeInput>(cam)
            .unwrap()
            .reset = true;
        step(&mut app, 16);

        let pos = app.world().get::<Transform>(cam).unwrap().translation;
        assert!(
            (pos - base).length() < 1e-3,
            "reset left camera at {} off base {}",
            pos,
            base
        );
    }
}
