//! Who owns the camera `Transform` this frame, and in what order.
//!
//! The camera Transform has four independent writers - chase sync, WASD sync,
//! camera shake (`Restore`/`Apply`) and `enforce_scripted_camera_pose`. A
//! PARTIAL lattice between them leaves the missing edges to executor readiness,
//! i.e. a per-frame coin flip: the scripted pose in the capture scripts
//! flickers because some frames the chase camera writes last.
//!
//! Shake sits BEFORE the scripted pose on purpose: a posed shot (photo mode,
//! the capture scripts, the cinematic framings) must be steady even when combat
//! next to the camera is feeding trauma.
//!
//! [`CameraAuthorityPlugin`] declares the total order once, using only the
//! `SystemSet`s the rigs already export. No rig changes and no writer is
//! disabled - a loser still runs and still writes, it is simply overwritten in
//! the same order every frame ("order, don't disable").

use bevy::{prelude::*, transform::TransformSystems};
use nova_gameplay::shake::{CameraShakeSuppressed, CameraShakeSystems};

use super::{
    chase::ChaseCameraSystems,
    wasd::{WASDCamera, WASDCameraSystems},
};

/// The camera-`Transform` write order in `PostUpdate`, from the base pose to
/// the pose the frame renders with.
///
/// Every camera writer belongs to exactly one phase. The rigs' own sets are
/// folded into these by [`CameraAuthorityPlugin`], so a system joins the chain
/// by naming a phase and nothing else.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum CameraAuthoritySystems {
    /// Solve the base pose from game state: chase sync and WASD sync.
    Solve,
    /// Add an offset on top of the solved pose: camera shake `Apply`.
    Additive,
    /// Overwrite the pose with a scripted one (photo mode, the capture
    /// scripts). Runs LAST, so the script wins the frame and a posed shot is
    /// shake-free - trauma from nearby combat must not jitter a cinematic
    /// framing or a screenshot.
    Override,
}

/// Declares the [`CameraAuthoritySystems`] chain. Added by
/// [`SpaceshipCameraControllerPlugin`](super::SpaceshipCameraControllerPlugin);
/// add it directly in a test app that drives a camera writer without the
/// gameplay plugin.
pub struct CameraAuthorityPlugin;

impl Plugin for CameraAuthorityPlugin {
    fn build(&self, app: &mut App) {
        trace!("CameraAuthorityPlugin: build");

        // `Restore` un-applies the previous frame's shake offset and so must
        // precede every base writer. `nova_gameplay::shake` pins only
        // `Restore.before(Apply)` and names no driver, so this is the ONLY
        // place the shake is ordered against the rigs that write the base pose.
        //
        // The `.before(Propagate)` on the tail is the edge the rigs never had for
        // shake or for a scripted pose: without it the frame can propagate
        // LAST frame's camera pose (a per-build coin flip). WASD sync carries
        // its own such edge; being inside `Solve` keeps it.
        app.configure_sets(
            PostUpdate,
            (
                CameraShakeSystems::Restore,
                CameraAuthoritySystems::Solve,
                CameraAuthoritySystems::Additive,
                CameraAuthoritySystems::Override,
            )
                .chain()
                .before(TransformSystems::Propagate),
        );

        // Fold the rigs' writers into the phases. Set-in-set rather than a
        // bare ordering edge: a rig writer is then ordered against every phase
        // at once, including phases added later.
        app.configure_sets(
            PostUpdate,
            (
                (ChaseCameraSystems::Sync, WASDCameraSystems::Sync)
                    .in_set(CameraAuthoritySystems::Solve),
                CameraShakeSystems::Apply.in_set(CameraAuthoritySystems::Additive),
            ),
        );

        app.add_observer(suppress_shake_while_wasd_drives);
        app.add_observer(release_shake_when_wasd_stops);
    }
}

/// Free-fly is not a player camera, so combat trauma must never jitter it: the
/// WASD rig is a detached observer's tripod, not a cockpit. The gate is keyed
/// on the RIG component itself - present exactly while WASD drives - because
/// the camera ENTITY persists across the WASD<->chase handoffs (its
/// `CameraShake` rides along), so component insertion alone cannot scope the
/// shake. This module owns the gate for the same reason it owns the write
/// order: `nova_gameplay::shake` names no driver, and only the authority layer
/// knows which rig currently is one.
fn suppress_shake_while_wasd_drives(insert: On<Insert, WASDCamera>, mut commands: Commands) {
    commands.entity(insert.entity).insert(CameraShakeSuppressed);
}

/// Hand the camera back to the shake when the WASD rig stops driving. Trauma
/// kept accruing and decaying while suppressed, so the handoff resumes from
/// live state instead of replaying a freeze-frame.
fn release_shake_when_wasd_stops(remove: On<Remove, WASDCamera>, mut commands: Commands) {
    // try_remove: the WASDCamera removal may be part of the camera's own
    // despawn, and a plain remove would panic on the dead entity.
    commands
        .entity(remove.entity)
        .try_remove::<CameraShakeSuppressed>();
}

#[cfg(test)]
mod tests {
    use nova_gameplay::shake::{CameraShake, CameraShakeInput, CameraShakePlugin};

    use super::{
        super::{
            chase::{ChaseCamera, ChaseCameraPlugin},
            wasd::WASDCameraPlugin,
        },
        *,
    };

    /// The shake is a generic rig that names no driver, so THIS plugin is the
    /// only thing holding `Restore -> chase base -> Apply`. Delete either
    /// `configure_sets` call above and this fails; `cargo check` cannot see it,
    /// and bevy's topological tie-break can silently supply the right order
    /// anyway, which is the accidental-correctness trap the module exists to
    /// close.
    #[test]
    fn the_shake_brackets_the_chase_base_writer() {
        #[derive(Resource, Default)]
        struct Order(Vec<&'static str>);

        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            TransformPlugin,
            ChaseCameraPlugin,
            CameraShakePlugin,
            CameraAuthorityPlugin,
        ));
        app.init_resource::<Order>();
        app.add_systems(
            PostUpdate,
            (
                (|mut o: ResMut<Order>| o.0.push("restore")).in_set(CameraShakeSystems::Restore),
                (|mut o: ResMut<Order>| o.0.push("chase")).in_set(ChaseCameraSystems::Sync),
                (|mut o: ResMut<Order>| o.0.push("apply")).in_set(CameraShakeSystems::Apply),
            ),
        );

        app.update();

        assert_eq!(
            app.world().resource::<Order>().0,
            vec!["restore", "chase", "apply"],
            "the shake must un-apply before the chase camera writes the base and \
             re-apply after it, or the offset either accumulates or is wiped"
        );
    }

    /// The chain must be ACYCLIC against the edges the rigs declare for
    /// themselves (`Restore.before(Apply)` from the shake, `WASD::Sync
    /// .before(Propagate)`) - a cycle would panic the schedule on the first
    /// `PostUpdate` run, and every camera writer runs there.
    #[test]
    fn the_chain_composes_with_every_camera_plugin() {
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            TransformPlugin,
            ChaseCameraPlugin,
            WASDCameraPlugin,
            CameraShakePlugin,
            CameraAuthorityPlugin,
        ));

        app.update();
    }

    /// A scripted `Override` writer beats the chase camera EVERY frame, which
    /// is the flicker fix: unordered, the two writers raced and the pose held
    /// only on the frames the executor happened to run it last.
    #[test]
    fn override_wins_the_frame_against_the_chase_camera() {
        const SCRIPTED: Vec3 = Vec3::new(11.0, 22.0, 33.0);

        fn pin_scripted_pose(mut cameras: Query<&mut Transform, With<ChaseCamera>>) {
            for mut transform in &mut cameras {
                transform.translation = SCRIPTED;
            }
        }

        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            TransformPlugin,
            ChaseCameraPlugin,
            CameraShakePlugin,
            CameraAuthorityPlugin,
        ));
        app.add_systems(
            PostUpdate,
            pin_scripted_pose.in_set(CameraAuthoritySystems::Override),
        );

        let camera = app
            .world_mut()
            .spawn((
                Transform::default(),
                ChaseCamera::default(),
                CameraShake::default(),
            ))
            .id();

        // Several frames: a race that survives one frame is not a fix.
        for _ in 0..8 {
            app.update();
            let pose = app.world().get::<Transform>(camera).expect("camera pose");
            assert_eq!(
                pose.translation, SCRIPTED,
                "the chase camera overwrote the scripted pose"
            );
        }
    }

    /// The mode contract on ONE persistent camera entity: while the WASD rig
    /// drives (free-fly), pumped trauma must not move the camera off the rig
    /// pose at all; hand the same entity to the chase rig (the player-camera
    /// path, mirroring the scenario lifecycle's controller swap) and the
    /// accrued trauma shakes it again. Keyed on the current MODE, not on
    /// component insertion - the `CameraShake` never leaves the entity.
    #[test]
    fn wasd_mode_holds_the_rig_pose_and_chase_mode_shakes_again() {
        fn feed_trauma(mut inputs: Query<&mut CameraShakeInput>) {
            for mut input in &mut inputs {
                input.add_trauma += 1.0;
            }
        }

        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            TransformPlugin,
            ChaseCameraPlugin,
            WASDCameraPlugin,
            CameraShakePlugin,
            CameraAuthorityPlugin,
        ));
        app.add_systems(Update, feed_trauma);

        let camera = app
            .world_mut()
            .spawn((
                Transform::default(),
                WASDCamera::default(),
                CameraShake::default(),
            ))
            .id();

        // Warm-up: the rig observers attach their state and the first frame's
        // dt settles before anything is asserted.
        app.update();

        // Free-fly: the WASD rig writes the identity pose every frame; with
        // trauma pumped every frame the camera must sit EXACTLY on it.
        for _ in 0..8 {
            app.update();
            let pose = app.world().get::<Transform>(camera).expect("camera pose");
            assert_eq!(
                pose.translation,
                Vec3::ZERO,
                "trauma moved the free-fly camera off its rig pose"
            );
            assert_eq!(
                pose.rotation,
                Quat::IDENTITY,
                "trauma kicked the free-fly camera off its rig rotation"
            );
        }

        // The player-camera handoff: WASD rig off, chase rig on, same entity.
        app.world_mut()
            .entity_mut(camera)
            .remove::<WASDCamera>()
            .insert(ChaseCamera::default());

        // Back in chase mode the accrued trauma applies again: the reported
        // offset goes non-zero on some frame (the sample is random per frame,
        // so any single frame could land near zero on an axis).
        let mut shook = false;
        for _ in 0..8 {
            app.update();
            let output = app
                .world()
                .get::<nova_gameplay::shake::CameraShakeOutput>(camera)
                .expect("shake output");
            if output.offset != Vec3::ZERO {
                shook = true;
            }
        }
        assert!(
            shook,
            "the chase-mode camera never shook again after the WASD handback"
        );
    }

    /// Trauma must not jitter a scripted pose: shake is `Additive`, which runs
    /// BEFORE `Override`, so a posed screenshot or cinematic shot is steady
    /// while combat next to the camera keeps feeding the shake.
    #[test]
    fn shake_does_not_jitter_a_scripted_pose() {
        const SCRIPTED: Vec3 = Vec3::new(11.0, 22.0, 33.0);

        fn pin_scripted_pose(mut cameras: Query<&mut Transform, With<CameraShake>>) {
            for mut transform in &mut cameras {
                transform.translation = SCRIPTED;
            }
        }

        fn feed_trauma(mut inputs: Query<&mut CameraShakeInput>) {
            for mut input in &mut inputs {
                input.add_trauma += 1.0;
            }
        }

        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            TransformPlugin,
            CameraShakePlugin,
            CameraAuthorityPlugin,
        ));
        app.add_systems(Update, feed_trauma);
        app.add_systems(
            PostUpdate,
            pin_scripted_pose.in_set(CameraAuthoritySystems::Override),
        );

        let camera = app
            .world_mut()
            .spawn((
                Transform::default(),
                CameraShake::default(),
                CameraShakeInput::default(),
            ))
            .id();

        for _ in 0..8 {
            app.update();
            let pose = app.world().get::<Transform>(camera).expect("camera pose");
            assert_eq!(
                pose.translation, SCRIPTED,
                "camera shake moved the scripted pose"
            );
        }
    }
}
