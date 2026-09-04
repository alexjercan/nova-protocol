//! Who the scenario camera is pointed at while a script owns it.
//!
//! Two ways to own it, one enforcement path. A [`ScriptedCameraPose`] pins a
//! FIXED world pose - photo mode, the capture scripts, a menu backdrop.  A
//! [`ScriptedCameraAnchor`] pins a pose RELATIVE to a live entity and
//! re-solves it every frame, which is what a cinematic needs: a fixed pose
//! abandons the player's ship the moment it moves, and a shot the player is
//! not in is a shot about nothing.
//!
//! Both write the same [`ScriptedCameraTransform`], and
//! [`enforce_scripted_camera_pose`] copies that onto the camera in
//! [`CameraAuthoritySystems::Override`] - after every base writer, so the
//! script wins the frame. Releasing is spelled `remove`: the chase rig never
//! stopped writing its own solve underneath, so taking the override off hands
//! the camera straight back to it with no restore pose to author.
//!
//! Camera authority is NOT helm authority. Nothing here touches the ship it
//! frames; a cinematic that needed the player parked has to earn that with an
//! objective, not take the controls away.

use bevy::prelude::*;
use nova_events::prelude::*;
use nova_ship::prelude::CameraAuthoritySystems;

/// A scripted camera pose that overrides the free-fly WASD controller, applied
/// every frame by [`enforce_scripted_camera_pose`]. Set by the `SetCamera`
/// scenario action (photo mode) and the capture scripts (`pose_camera`); while
/// present it pins the `ScenarioCameraMarker` camera at `position` looking at
/// `look_at`.
#[derive(Component, Debug, Clone, Copy)]
#[require(ScriptedCameraTransform)]
pub struct ScriptedCameraPose {
    /// World-space camera position.
    pub position: Meters3,
    /// World-space point the camera looks at (up is +Y).
    pub look_at: Meters3,
}

/// Which frame a [`ScriptedCameraAnchor`]'s offset is measured in.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum CameraOffsetFrame {
    /// The anchor's own frame: `+Z` is behind it, `+Y` above it. An
    /// over-the-shoulder shot that rides the hull however it turns.
    #[default]
    Local,
    /// World axes. The composition is then fixed no matter which way the
    /// anchor is pointing - what a staged set piece wants, because the player
    /// is still flying and their heading is not the author's to choose.
    World,
}

/// What a [`ScriptedCameraAnchor`] points at.
#[derive(Clone, Copy, Debug, Default)]
pub enum ScriptedCameraLookAt {
    /// The anchor itself.
    #[default]
    Anchor,
    /// A fixed world point. Survives the death of whatever is standing there,
    /// which is what a shot of something being destroyed needs.
    Point(Meters3),
    /// Another live entity, tracked. Falls back to the anchor if it goes away.
    Entity(Entity),
}

/// A scripted camera pose anchored to a live entity, re-solved every frame by
/// [`track_scripted_camera_anchor`].
///
/// The offset is measured in `frame`, so the same authored numbers give either
/// an over-the-shoulder rig or a fixed staged composition. When `anchor` is
/// gone the whole component is dropped and the camera goes back to its own rig
/// - a cinematic outliving its subject is worse than no cinematic.
#[derive(Component, Debug, Clone, Copy)]
#[require(ScriptedCameraTransform)]
pub struct ScriptedCameraAnchor {
    /// The entity the shot is framed around.
    pub anchor: Entity,
    /// Camera position relative to the anchor, in `frame`.
    pub offset: Meters3,
    /// Which frame `offset` is measured in.
    pub frame: CameraOffsetFrame,
    /// What the camera looks at.
    pub look_at: ScriptedCameraLookAt,
}

/// The Bevy transform a scripted pose means, derived by
/// [`derive_scripted_camera_transform`] only when a fixed pose changes, and
/// every frame by [`track_scripted_camera_anchor`] when the pose follows
/// something alive.
///
/// A fixed pose has to be RE-APPLIED every frame, because the whole point of
/// the override is to win the frame's last write; it does not have to be
/// RE-DERIVED every frame. The identity default is never read: both producers
/// run in `PostUpdate` before the override, and `Changed` fires on the insert,
/// so the transform exists by the first frame the pose does.
#[derive(Component, Debug, Clone, Copy, Default, Deref)]
pub struct ScriptedCameraTransform(pub Transform);

impl ScriptedCameraTransform {
    /// The transform `pose` means.
    ///
    /// Engine boundary: the one place a scripted pose becomes a Bevy transform,
    /// so every poser upstream of it stays in meters.
    pub fn of(pose: &ScriptedCameraPose) -> Self {
        Self(
            Transform::from_translation(pose.position.to_engine())
                .looking_at(pose.look_at.to_engine(), Vec3::Y),
        )
    }
}

/// Below this the eye and its target are the same point and `looking_at` has
/// no direction to build a rotation from, so the solve is skipped and last
/// frame's rotation stands. Only a degenerate authored offset reaches it.
const MIN_LOOK_DISTANCE: f32 = 1e-3;

/// Derive each changed [`ScriptedCameraPose`] into the transform the override
/// then writes.
pub(super) fn derive_scripted_camera_transform(
    mut cameras: Query<
        (&mut ScriptedCameraTransform, &ScriptedCameraPose),
        Changed<ScriptedCameraPose>,
    >,
) {
    for (mut derived, pose) in &mut cameras {
        *derived = ScriptedCameraTransform::of(pose);
    }
}

/// Re-solve every [`ScriptedCameraAnchor`] against where its subject is NOW,
/// and release the camera when that subject is gone.
pub(super) fn track_scripted_camera_anchor(
    mut commands: Commands,
    mut cameras: Query<(Entity, &mut ScriptedCameraTransform, &ScriptedCameraAnchor)>,
    q_transform: Query<&GlobalTransform>,
) {
    for (camera, mut derived, anchor) in &mut cameras {
        let Ok(subject) = q_transform.get(anchor.anchor) else {
            // The shot lost its subject: hand the camera back rather than
            // hold a frame of empty space at the last known pose.
            commands.entity(camera).remove::<ScriptedCameraAnchor>();
            continue;
        };
        let subject = subject.compute_transform();
        let offset = anchor.offset.to_engine();
        let position = subject.translation
            + match anchor.frame {
                CameraOffsetFrame::Local => subject.rotation * offset,
                CameraOffsetFrame::World => offset,
            };
        let target = match anchor.look_at {
            ScriptedCameraLookAt::Anchor => subject.translation,
            ScriptedCameraLookAt::Point(point) => point.to_engine(),
            ScriptedCameraLookAt::Entity(entity) => q_transform
                .get(entity)
                .map(|transform| transform.translation())
                .unwrap_or(subject.translation),
        };
        if position.distance(target) < MIN_LOOK_DISTANCE {
            derived.0.translation = position;
            continue;
        }
        *derived = ScriptedCameraTransform(
            Transform::from_translation(position).looking_at(target, Vec3::Y),
        );
    }
}

/// Pin every camera carrying a scripted pose to that pose. Runs in
/// [`CameraAuthoritySystems::Override`] so it wins the frame's last write to the
/// camera Transform, shake offset included.
pub(super) fn enforce_scripted_camera_pose(
    mut cameras: Query<(&mut Transform, &ScriptedCameraTransform)>,
) {
    for (mut transform, derived) in &mut cameras {
        *transform = **derived;
    }
}

/// Release the camera when a fixed pose is taken off it.
///
/// A required component outlives the one that required it, so dropping the
/// pose alone would leave the derived transform behind and the override would
/// keep winning every frame - a camera nothing could aim again. Releasing is
/// spelled `remove::<ScriptedCameraPose>()` wherever a script hands the camera
/// back, so the pair is kept honest here rather than at each of those.
pub(super) fn drop_scripted_camera_transform(
    remove: On<Remove, ScriptedCameraPose>,
    mut commands: Commands,
    q_anchor: Query<(), With<ScriptedCameraAnchor>>,
) {
    release_scripted_camera(
        remove.entity,
        &mut commands,
        q_anchor.contains(remove.entity),
    );
}

/// The same release for an anchored pose.
pub(super) fn drop_scripted_camera_anchor(
    remove: On<Remove, ScriptedCameraAnchor>,
    mut commands: Commands,
    q_pose: Query<(), With<ScriptedCameraPose>>,
) {
    release_scripted_camera(remove.entity, &mut commands, q_pose.contains(remove.entity));
}

/// Drop the derived transform unless the camera is still held by the OTHER
/// kind of scripted pose - a script that swaps a fixed pose for an anchored one
/// must not lose the override in the frame between them.
fn release_scripted_camera(camera: Entity, commands: &mut Commands, still_held: bool) {
    if still_held {
        return;
    }
    // try_remove: the release may be part of the camera's own despawn.
    commands
        .entity(camera)
        .try_remove::<ScriptedCameraTransform>();
}

/// Register the scripted-camera layer's systems and observers.
///
/// The `SetCamera`/`SetCameraAnchor` actions pin their pose on the scenario
/// camera; the enforcement runs in [`CameraAuthoritySystems::Override`], the
/// phase that runs after every base writer - WASD sync, chase sync AND camera
/// shake - and before propagation. Both controllers keep writing the camera
/// Transform every frame (and removing a controller does not stop it - the
/// private state components survive), so a one-shot Transform set would be
/// immediately overwritten; running in the override phase is what makes the
/// pose stick, on every frame rather than on the frames the executor happened
/// to schedule it last.
pub(super) fn register_scripted_camera(app: &mut App) {
    app.add_systems(
        PostUpdate,
        (
            (
                derive_scripted_camera_transform,
                track_scripted_camera_anchor,
            )
                .before(CameraAuthoritySystems::Override),
            enforce_scripted_camera_pose.in_set(CameraAuthoritySystems::Override),
        ),
    );
    app.add_observer(drop_scripted_camera_transform);
    app.add_observer(drop_scripted_camera_anchor);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The rig every test below drives: both producers, then the override,
    /// plus the two release observers. No cameras, no rendering - the whole
    /// layer is transform arithmetic.
    fn camera_app() -> App {
        let mut app = App::new();
        app.add_systems(
            PostUpdate,
            (
                (
                    derive_scripted_camera_transform,
                    track_scripted_camera_anchor,
                ),
                enforce_scripted_camera_pose,
            )
                .chain(),
        );
        app.add_observer(drop_scripted_camera_transform);
        app.add_observer(drop_scripted_camera_anchor);
        app
    }

    /// A subject the camera can frame, at `position` facing `-Z`.
    fn subject(app: &mut App, position: Vec3) -> Entity {
        app.world_mut()
            .spawn((
                Transform::from_translation(position),
                GlobalTransform::from_translation(position),
            ))
            .id()
    }

    /// A script that hands the camera back gets a camera it can aim again.
    ///
    /// [`ScriptedCameraPose`] REQUIRES [`ScriptedCameraTransform`], and a
    /// required component outlives its requirer: with the derived half left
    /// behind, the override would keep writing the released pose every frame.
    #[test]
    fn releasing_the_scripted_pose_releases_the_camera() {
        let mut app = camera_app();

        let eye = Meters3::new(0.0, 100.0, 0.0);
        let camera = app
            .world_mut()
            .spawn((
                Transform::default(),
                ScriptedCameraPose {
                    position: eye,
                    look_at: Meters3::ZERO,
                },
            ))
            .id();
        app.update();
        assert_eq!(
            app.world()
                .get::<Transform>(camera)
                .expect("the camera has a transform")
                .translation,
            eye.to_engine(),
            "the pose is enforced while it is on the camera"
        );

        app.world_mut()
            .entity_mut(camera)
            .remove::<ScriptedCameraPose>();
        let free = Transform::from_xyz(1.0, 2.0, 3.0);
        app.world_mut().entity_mut(camera).insert(free);
        app.update();

        assert!(
            app.world().get::<ScriptedCameraTransform>(camera).is_none(),
            "the derived transform goes with the pose it came from"
        );
        assert_eq!(
            app.world()
                .get::<Transform>(camera)
                .expect("the camera has a transform")
                .translation,
            free.translation,
            "a released camera keeps what its own controller wrote"
        );
    }

    /// An anchored shot follows its subject. A fixed pose would frame the
    /// place the ship USED to be, which is the whole reason this exists.
    #[test]
    fn an_anchored_pose_follows_the_ship_it_frames() {
        let mut app = camera_app();
        let ship = subject(&mut app, Vec3::ZERO);
        let camera = app
            .world_mut()
            .spawn((
                Transform::default(),
                ScriptedCameraAnchor {
                    anchor: ship,
                    offset: Meters3::new(0.0, 0.0, 100.0),
                    frame: CameraOffsetFrame::World,
                    look_at: ScriptedCameraLookAt::Anchor,
                },
            ))
            .id();

        app.update();
        assert_eq!(
            app.world().get::<Transform>(camera).unwrap().translation,
            Meters3::new(0.0, 0.0, 100.0).to_engine()
        );

        // The ship flies on; the camera goes with it.
        let moved = Meters3::new(500.0, 0.0, -2_000.0).to_engine();
        app.world_mut().entity_mut(ship).insert((
            Transform::from_translation(moved),
            GlobalTransform::from_translation(moved),
        ));
        app.update();
        assert_eq!(
            app.world().get::<Transform>(camera).unwrap().translation,
            moved + Meters3::new(0.0, 0.0, 100.0).to_engine(),
            "the anchored pose must track the subject, not the world"
        );
    }

    /// A LOCAL offset rides the hull's own frame, so the same authored numbers
    /// stay over the same shoulder however the ship is pointing. A WORLD offset
    /// does not move when the ship turns - which is what a staged composition
    /// needs, because the player's heading is not the author's to choose.
    #[test]
    fn a_local_offset_turns_with_the_hull_and_a_world_offset_does_not() {
        for (frame, expected) in [
            // Yawed 180 degrees: "100 m behind" becomes 100 m the other way.
            (CameraOffsetFrame::Local, Vec3::new(0.0, 0.0, -10.0)),
            (CameraOffsetFrame::World, Vec3::new(0.0, 0.0, 10.0)),
        ] {
            let mut app = camera_app();
            let turned = Transform::from_rotation(Quat::from_rotation_y(std::f32::consts::PI));
            let ship = app
                .world_mut()
                .spawn((turned, GlobalTransform::from(turned)))
                .id();
            let camera = app
                .world_mut()
                .spawn((
                    Transform::default(),
                    ScriptedCameraAnchor {
                        anchor: ship,
                        offset: Meters3::new(0.0, 0.0, 100.0),
                        frame,
                        look_at: ScriptedCameraLookAt::Anchor,
                    },
                ))
                .id();
            app.update();
            let got = app.world().get::<Transform>(camera).unwrap().translation;
            assert!(
                got.distance(expected) < 1e-3,
                "{frame:?} offset put the camera at {got:?}, expected {expected:?}"
            );
        }
    }

    /// A cut list is not one shot. The mainline chapter anchors the camera,
    /// hands it back across a long approach, and anchors it again on a
    /// DIFFERENT subject for the kill - so a released camera has to be
    /// re-poseable, and the second shot must not inherit the first one's
    /// subject.
    #[test]
    fn a_released_camera_can_be_anchored_again_somewhere_else() {
        let mut app = camera_app();
        let first = subject(&mut app, Vec3::ZERO);
        let second = subject(&mut app, Meters3::new(0.0, 0.0, -3_000.0).to_engine());
        let offset = Meters3::new(0.0, 0.0, 100.0);
        let anchor = |anchor| ScriptedCameraAnchor {
            anchor,
            offset,
            frame: CameraOffsetFrame::World,
            look_at: ScriptedCameraLookAt::Anchor,
        };

        let camera = app
            .world_mut()
            .spawn((Transform::default(), anchor(first)))
            .id();
        app.update();

        app.world_mut()
            .entity_mut(camera)
            .remove::<ScriptedCameraAnchor>();
        app.update();
        assert!(
            app.world().get::<ScriptedCameraTransform>(camera).is_none(),
            "the derived transform outlived the anchor it came from"
        );

        app.world_mut().entity_mut(camera).insert(anchor(second));
        app.update();
        assert_eq!(
            app.world().get::<Transform>(camera).unwrap().translation,
            (Meters3::new(0.0, 0.0, -3_000.0) + offset).to_engine(),
            "the second shot is still framing the first shot's subject"
        );
    }

    /// A shot aimed at a POINT keeps aiming there after whatever was standing
    /// there is gone. The mainline cinematic frames a carrier being destroyed,
    /// so this is the difference between a held shot and a camera that snaps
    /// away at the moment of the kill.
    #[test]
    fn a_point_target_outlives_what_was_standing_on_it() {
        let mut app = camera_app();
        let ship = subject(&mut app, Vec3::ZERO);
        // Off the camera's own axis, so falling back to the anchor is a
        // visibly different aim rather than the same one by luck.
        let doomed = subject(&mut app, Meters3::new(1_000.0, 0.0, -1_000.0).to_engine());
        let camera = app
            .world_mut()
            .spawn((
                Transform::default(),
                ScriptedCameraAnchor {
                    anchor: ship,
                    offset: Meters3::new(0.0, 0.0, 100.0),
                    frame: CameraOffsetFrame::World,
                    look_at: ScriptedCameraLookAt::Entity(doomed),
                },
            ))
            .id();
        app.update();
        let aimed = app.world().get::<Transform>(camera).unwrap().rotation;

        app.world_mut().entity_mut(doomed).despawn();
        app.update();
        let after = app.world().get::<Transform>(camera).unwrap().rotation;
        assert!(
            after.angle_between(aimed) > 0.1,
            "rig sanity: losing an ENTITY target must fall back to the anchor"
        );

        // The same shot authored as a point does not move at all.
        let mut app = camera_app();
        let ship = subject(&mut app, Vec3::ZERO);
        let camera = app
            .world_mut()
            .spawn((
                Transform::default(),
                ScriptedCameraAnchor {
                    anchor: ship,
                    offset: Meters3::new(0.0, 0.0, 100.0),
                    frame: CameraOffsetFrame::World,
                    look_at: ScriptedCameraLookAt::Point(Meters3::new(1_000.0, 0.0, -1_000.0)),
                },
            ))
            .id();
        app.update();
        let aimed = app.world().get::<Transform>(camera).unwrap().rotation;
        app.update();
        assert_eq!(
            app.world().get::<Transform>(camera).unwrap().rotation,
            aimed,
            "a point target is fixed and nothing can take it away"
        );
    }

    /// Losing the SUBJECT ends the shot. A cinematic held on empty space after
    /// the player's ship came apart is worse than no cinematic: the death
    /// handoff (`on_player_spaceship_destroyed`) has to be able to aim the
    /// camera again.
    #[test]
    fn losing_the_anchor_hands_the_camera_back() {
        let mut app = camera_app();
        let ship = subject(&mut app, Vec3::ZERO);
        let camera = app
            .world_mut()
            .spawn((
                Transform::default(),
                ScriptedCameraAnchor {
                    anchor: ship,
                    offset: Meters3::new(0.0, 0.0, 100.0),
                    frame: CameraOffsetFrame::World,
                    look_at: ScriptedCameraLookAt::Anchor,
                },
            ))
            .id();
        app.update();
        assert!(app.world().get::<ScriptedCameraTransform>(camera).is_some());

        app.world_mut().entity_mut(ship).despawn();
        app.update();

        assert!(
            app.world().get::<ScriptedCameraAnchor>(camera).is_none(),
            "an anchor with no subject must drop itself"
        );
        assert!(
            app.world().get::<ScriptedCameraTransform>(camera).is_none(),
            "and take the override with it, or the camera stays frozen"
        );

        let free = Transform::from_xyz(1.0, 2.0, 3.0);
        app.world_mut().entity_mut(camera).insert(free);
        app.update();
        assert_eq!(
            app.world().get::<Transform>(camera).unwrap().translation,
            free.translation,
            "the camera's own rig owns it again"
        );
    }

    /// Swapping a fixed pose for an anchored one must not blink: the release
    /// observer keeps the derived transform while the OTHER kind of pose still
    /// holds the camera, so the cinematic never drops a frame back to chase.
    #[test]
    fn swapping_one_scripted_pose_for_the_other_never_releases_the_camera() {
        let mut app = camera_app();
        let ship = subject(&mut app, Vec3::ZERO);
        let camera = app
            .world_mut()
            .spawn((
                Transform::default(),
                ScriptedCameraPose {
                    position: Meters3::new(0.0, 500.0, 0.0),
                    look_at: Meters3::ZERO,
                },
            ))
            .id();
        app.update();

        app.world_mut()
            .entity_mut(camera)
            .insert(ScriptedCameraAnchor {
                anchor: ship,
                offset: Meters3::new(0.0, 0.0, 100.0),
                frame: CameraOffsetFrame::World,
                look_at: ScriptedCameraLookAt::Anchor,
            });
        app.world_mut()
            .entity_mut(camera)
            .remove::<ScriptedCameraPose>();
        app.update();

        assert!(
            app.world().get::<ScriptedCameraTransform>(camera).is_some(),
            "the anchored pose still holds the camera"
        );
        assert_eq!(
            app.world().get::<Transform>(camera).unwrap().translation,
            Meters3::new(0.0, 0.0, 100.0).to_engine()
        );
    }
}
