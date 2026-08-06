//! Who owns the camera `Transform` this frame, and in what order.
//!
//! The camera Transform has four independent writers - bcs chase sync, bcs WASD
//! sync, bcs camera shake (`Restore`/`Apply`) and nova's
//! `enforce_scripted_camera_pose` - and until this module existed the lattice
//! between them was PARTIAL. The missing edges were filled in by executor
//! readiness, i.e. a per-frame coin flip, which is what made the scripted pose
//! in the capture scripts flicker: some frames the chase camera wrote last.
//!
//! [`CameraAuthorityPlugin`] declares the total order once, in nova, using only
//! the `SystemSet`s bcs already exports. Nothing in bcs changes and no writer is
//! disabled - a loser still runs and still writes, it is simply overwritten in
//! the same order every frame ("order, don't disable").

use bevy::{prelude::*, transform::TransformSystems};
use bevy_common_systems::prelude::{CameraShakeSystems, ChaseCameraSystems, WASDCameraSystems};

/// The camera-`Transform` write order in `PostUpdate`, from the base pose to
/// the pose the frame renders with.
///
/// Every camera writer belongs to exactly one phase. bcs's own sets are folded
/// into these by [`CameraAuthorityPlugin`], so a nova system joins the chain by
/// naming a phase and nothing else.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum CameraAuthority {
    /// Solve the base pose from game state: bcs chase sync and WASD sync.
    Solve,
    /// Overwrite the solved pose with a scripted one (photo mode, the capture
    /// scripts). Runs after every base writer, so the script wins the frame.
    Override,
    /// Add an offset on top of whatever pose won: bcs camera shake `Apply`.
    /// Additive, so it composes with either of the two above.
    Additive,
}

/// Declares the [`CameraAuthority`] chain. Added by
/// [`SpaceshipCameraControllerPlugin`](super::SpaceshipCameraControllerPlugin);
/// add it directly in a test app that drives a camera writer without the
/// gameplay plugin.
pub struct CameraAuthorityPlugin;

impl Plugin for CameraAuthorityPlugin {
    fn build(&self, app: &mut App) {
        debug!("CameraAuthorityPlugin: build");

        // `Restore` un-applies the previous frame's shake offset and so must
        // precede every base writer - bcs pins it before `ChaseCameraSystems::Sync`
        // only, which is nothing when the chase plugin is absent.
        //
        // The `.before(Propagate)` on the tail is the edge bcs never had for
        // shake or for a scripted pose: without it the frame can propagate
        // LAST frame's camera pose (a per-build coin flip). WASD sync carries
        // its own such edge; being inside `Solve` keeps it.
        app.configure_sets(
            PostUpdate,
            (
                CameraShakeSystems::Restore,
                CameraAuthority::Solve,
                CameraAuthority::Override,
                CameraAuthority::Additive,
            )
                .chain()
                .before(TransformSystems::Propagate),
        );

        // Fold bcs's writers into the phases. Set-in-set rather than a bare
        // ordering edge: a bcs writer is then ordered against every phase at
        // once, including phases added later.
        app.configure_sets(
            PostUpdate,
            (
                (ChaseCameraSystems::Sync, WASDCameraSystems::Sync).in_set(CameraAuthority::Solve),
                CameraShakeSystems::Apply.in_set(CameraAuthority::Additive),
            ),
        );
    }
}

#[cfg(test)]
mod tests {
    use bevy_common_systems::prelude::{
        CameraShake, CameraShakePlugin, ChaseCamera, ChaseCameraPlugin, WASDCameraPlugin,
    };

    use super::*;

    /// The chain must be ACYCLIC against the edges bcs declares for itself
    /// (`Restore.before(Chase::Sync)`, `Apply.after(Chase::Sync)`,
    /// `WASD::Sync.before(Propagate)`) - a cycle would panic the schedule on
    /// the first `PostUpdate` run, and every camera writer runs there.
    #[test]
    fn the_chain_composes_with_every_bcs_camera_plugin() {
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
            pin_scripted_pose.in_set(CameraAuthority::Override),
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
}
