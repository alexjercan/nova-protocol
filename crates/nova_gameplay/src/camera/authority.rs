//! Who owns the camera `Transform` this frame, and in what order.
//!
//! The camera Transform has four independent writers - chase sync, WASD
//! sync, camera shake (`Restore`/`Apply`) and
//! `enforce_scripted_camera_pose` - and until this module existed the lattice
//! between them was PARTIAL. The missing edges were filled in by executor
//! readiness, i.e. a per-frame coin flip, which is what made the scripted pose
//! in the capture scripts flicker: some frames the chase camera wrote last.
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

use super::{chase::ChaseCameraSystems, shake::CameraShakeSystems, wasd::WASDCameraSystems};

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
        debug!("CameraAuthorityPlugin: build");

        // `Restore` un-applies the previous frame's shake offset and so must
        // precede every base writer - `shake` pins it before
        // `ChaseCameraSystems::Sync` only, which is nothing when the chase
        // plugin is absent.
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
    }
}

#[cfg(test)]
mod tests {
    use super::{
        super::{
            chase::{ChaseCamera, ChaseCameraPlugin},
            shake::{CameraShake, CameraShakeInput, CameraShakePlugin},
            wasd::WASDCameraPlugin,
        },
        *,
    };

    /// The chain must be ACYCLIC against the edges the rigs declare for themselves
    /// (`Restore.before(Chase::Sync)`, `Apply.after(Chase::Sync)`,
    /// `WASD::Sync.before(Propagate)`) - a cycle would panic the schedule on
    /// the first `PostUpdate` run, and every camera writer runs there.
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
