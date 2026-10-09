# Camera view gate, revision 3 (for approval)

The requirement: save the camera position and rotation relative to the
ship, so a loaded world shows the player's view, not the default framing.

- Revision 1 saved the Normal rig only.
- Revision 2 saved the anchor rotation and the zoom. The owner refused it: the
  rendered position can differ (Turret fixed distance, hull framing,
  smoothing, a blend in flight).

Revision 3 saves the rendered camera pose relative to the rendered ship and
shows it on the first displayed frame after Load. A bounded blend then eases
onto the live chase pose, and the ship is never turned. Nothing below is
edited yet.

## How the rendered pose is made (evidence)

- The chase sync (`camera/chase.rs:226-247`, PostUpdate,
  `ChaseCameraSystems::Sync`):
  - The position is the smoothed `ChaseCameraState.anchor_pos` (private,
    `chase.rs:127-131`), eased toward `anchor_pos + anchor_rot * offset`
    (`chase.rs:202-224`).
  - The rotation is `look_at(anchor_pos + anchor_rot * focus_offset,
    anchor_rot * Y)`.
- The rig fields (offset, focus_offset, smoothing) come from the mode, the
  hull envelope and the zoom (`update_camera_rig`, `camera/framing.rs`). The
  Turret rig ignores the zoom (`rig.rs:118-123`).
- `anchor_rot` follows the active rig, or a `CameraHandbackBlend`
  (`framing.rs:26-80`).
- Camera shake adds an offset and a kick after the solve
  (`CameraAuthoritySystems::Solve -> Additive -> Override`,
  `camera/authority.rs:24-66`). It exposes them in the public
  `CameraShakeOutput { offset, kick }` (`nova_gameplay/src/shake.rs:136-142`),
  and un-applies them at the next frame start (`shake.rs:257-265`).
- During `Update` (where `snapshot_world` runs), the `GlobalTransform` of
  the camera and of the ship are both the last displayed frame's: both are
  propagated in the previous PostUpdate. A `Transform` read in Update is not
  a consistent pair, because physics interpolation moves the ship's
  `Transform` before Update.
- The Normal rig (`rig.rs:133-138`) drives the ship's PD and is never
  re-seeded on a mode return (`mode.rs:82-85`). FreeLook and Turret rigs are
  re-seeded from the outgoing rig (`mode.rs:52-130`). `Autopilot`
  (`flight/state.rs:177`) is not saved; its disengage re-seeds the Normal rig
  from the hull (`handback.rs:41-98`).
- The mode is derived from held input and frozen while paused
  (`mode.rs:16-33`, `camera/mod.rs:131-138`). After a Load nothing is held,
  so it derives to Normal.
- Zoom: `ChaseZoom.manual` (`zoom.rs:29-35`, 1 to `CHASE_ZOOM_MAX` 8,
  `zoom.rs:15`, `:70`).
- Attach: `on_player_spaceship_spawned` (`nova_scenario/src/loader/lifecycle.rs:671-683`)
  inserts the controller. The order between the player marker and the
  resumed `Transform` in `spawn_scenario_spaceship`
  (`nova_scenario/src/actions/spawn.rs:242-285`) is not pinned, so nothing
  reads the ship's pose at attach time.
- Save: `SavedPlayer` (`nova_world_base/src/save/mod.rs:98-110`), built in
  `snapshot_world` (`save/session.rs:304-435`). It is decoded in `open_world`
  (`save/mod.rs:271-280`) and seeded by `resume_world`
  (`save/session.rs:208-234`). `WORLD_SAVE_FORMAT` stays 1 because it is
  unshipped.

## End state

- The save holds, relative to the ship:
  - POSE: the camera pose of the last displayed frame without shake:
    `camera_global` minus `CameraShakeOutput`, in the frame of the ship's
    `GlobalTransform` from that same frame. That is position and rotation.
  - STEER: the Normal rig look relative to the ship's physics `Rotation`, or
    identity while an `Autopilot` flies the ship (a Load is a disengage).
  - ZOOM: `ChaseZoom.manual`.
- After a Load, on the first frame that renders the player camera, the camera
  pose is exactly `ship_now * POSE`. A `CameraResumeBlend` then eases position
  (lerp) and rotation (slerp, smoothstep) onto the live solved chase pose
  over `HANDBACK_BLEND_SECONDS` (0.45 s). While the blend runs, POSE is
  re-composed with the live ship pose every frame, so it stays ship-relative
  while the ship moves.
- On the blend's first frame the chase smoothing is seeded at the shown
  position, so no smoothing jump is left when the blend ends.
- The Normal rig opens at STEER, so the ship keeps exactly the heading command
  it had and is never turned toward the camera. The zoom opens at ZOOM.
- The mode after Load is Normal (D-C3 a, a release). A Turret or FreeLook
  view shows on the first frame, then eases to the Normal framing in 0.45 s.
- Shake is not saved: a jolt is transient.
- What dies: "the zoom level is never saved" for a saved world (`zoom.rs:21`,
  wiki `flight-autopilot.md:32`).
- Unchanged: a death respawn, a scenario, a session-only world and the web
  build open the default framing.
- Fails loudly, without stopping valid play:
  - A view on disk that is not finite (position), not a unit quaternion
    (rotation, steer), or has a zoom outside 1 to `CHASE_ZOOM_MAX` refuses the
    world in the Load list (Unreadable). Nothing starts.
  - A save on a frame with no camera controller, no Normal rig, or no
    propagated camera (the first arm frames) waits visibly: "Waiting to save:
    the player camera is not ready". It retries each frame, and ends Failed
    (visible, last save kept) at `SAVE_SETTLING_FRAMES_MAX`, the bound and
    path of unsettled bodies (`save/session.rs:33-36`, `:347-361`). It never
    panics.
- A scripted camera pose (`Override`: photo mode, capture scripts) is not
  saved; the base chase pose is.

## Proposed types and functions

In `nova_ship::camera`, exported in its prelude:

```rust
/// The player's camera as a saved world keeps it, relative to the ship.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize), serde(deny_unknown_fields))]
pub struct CameraView {
    /// The rendered camera position, in the ship's frame, engine units.
    pub position_from_ship: Vec3,
    /// The rendered camera rotation, in the ship's frame.
    pub rotation_from_ship: Quat,
    /// The look the ship steers by, in the ship's frame. Identity while an
    /// autopilot maneuver flies the ship.
    pub steer_from_ship: Quat,
    /// The session zoom level, 1 to `CHASE_ZOOM_MAX`.
    pub zoom: f32,
}

impl CameraView {
    /// The player camera as the last displayed frame showed it, without
    /// shake, or `None` while the controller, its Normal rig or a propagated
    /// camera pose does not exist yet.
    pub fn capture(world: &mut World) -> Option<Self>;
    /// # Errors
    /// A non-finite position, a rotation or steer that is not a finite unit
    /// quaternion, or a zoom that is not finite or is outside 1 to
    /// `CHASE_ZOOM_MAX`.
    pub fn validate(&self) -> Result<(), CameraViewFault>;
}

/// Why a saved view cannot open.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CameraViewFault { NonFinitePosition(Vec3), NonUnitRotation(Quat), ZoomOutOfRange(f32) }
// + Display

/// The view the next player camera opens at. The first
/// `insert_camera_controller` takes it; a later respawn opens the default.
#[derive(Resource, Debug, Clone, Copy)]
pub struct ResumedCameraView {
    /// The saved view; its pose stays ship-relative until the blend shows it.
    pub view: CameraView,
    /// STEER in world space, composed from the saved physics rotation.
    pub steer: Quat,
}

/// Eases the camera from a resumed ship-relative pose onto the solved chase
/// pose. On the camera controller; removed when it ends.
#[derive(Component, Debug, Clone, Copy)]
pub struct CameraResumeBlend {
    pub position_from_ship: Vec3,
    pub rotation_from_ship: Quat,
    pub elapsed: f32,
}

/// PostUpdate, `CameraAuthoritySystems::Solve`, after `ChaseCameraSystems::Sync`:
/// on the first frame seeds the chase smoothing at the shown position; each
/// frame writes `blend(ship_now * from, solved, eased(elapsed))`, then
/// advances `elapsed` by `Time` (virtual: a paused game holds the blend).
fn apply_camera_resume_blend(..);  // private system in camera/chase.rs
```

In `nova_world_base::save`: the required field `SavedPlayer.view: CameraView`.

## Call graph

Before:
```
snapshot_world -> SavedPlayer{id, transform, motion, ship}
resume_world   -> ResumedSpaceship
player spawn   -> insert_camera_controller -> Normal rig default, zoom.manual
PostUpdate     -> chase sync -> shake -> scripted -> propagate
```
After:
```
snapshot_world -> CameraView::capture(world)
                    None => Waiting("the player camera is not ready"),
                            settling_frames += advanced; > MAX => Failed
                    Some(view) => SavedPlayer{.., view}
open_world     -> decode -> state.player.view.validate()
                    Err => WorldRefusal::Unreadable("camera view: ...")
resume_world   -> ResumedSpaceship
               -> ResumedCameraView{ view, steer: rot * view.steer_from_ship }
player spawn   -> insert_camera_controller
                    take ResumedCameraView
                    Some => Normal rig PointRotation{initial_rotation: steer},
                            zoom.manual = view.zoom,
                            insert CameraResumeBlend{pose from view, elapsed 0}
                    None => as today
PostUpdate     -> chase sync -> apply_camera_resume_blend (while present)
               -> shake -> scripted -> propagate
```

## Callers to update

- `snapshot_world` (`save/session.rs:304-435`): capture first, so a missing
  camera waits before the sector snapshot.
- `nova_world_base` `test_support::arm_save_fixture`: spawn the camera
  controller with its rigs and a propagated `GlobalTransform`, and
  `ChaseZoom`.
- `chase.rs`: register the blend system in `ChaseCameraPlugin` (Solve, after
  Sync).
- Every `SavedPlayer {` literal in tests (search).
- Docs: the `zoom.rs:21` doc, wiki `flight-autopilot.md:32`, and the save docs
  in slice 5.

## Proof

1. ECS, `nova_ship` camera tests (beside `a_respawned_camera_opens_at_the_session_zoom`,
   `rig.rs:325`):
   `a_resumed_camera_renders_its_saved_pose_first_then_eases_onto_the_chase_pose`.
   The ship is rotated and translated. POSE is placed far from the Normal
   framing (a Turret distance and a FreeLook angle). On the first frame,
   the camera `GlobalTransform` relative to the ship equals POSE (1e-4). At
   0.45 s it equals the solved chase pose. The Normal rig equals STEER, the
   zoom is ZOOM, and the resource is gone. A second controller (respawn)
   opens the default.
2. ECS, `nova_ship` camera tests:
   `a_captured_view_is_the_rendered_pose_without_shake_relative_to_a_rotated_ship`.
   The ship is rotated, with propagated poses, looped over:
   - Normal;
   - FreeLook looking away (a distinct rotation);
   - Turret (a distinct position: the fixed combat distance);
   - a live shake offset and kick (removed);
   - an engaged autopilot (STEER is identity);
   - no controller or rig (`None`).
3. ECS, `nova_world_base` save tests:
   `a_save_waits_visibly_until_the_player_camera_is_ready`. With no camera,
   the status is Waiting with the camera reason and nothing is written. Once
   the camera exists, the save lands, and the view on disk equals the fixture
   camera relative to the ship.
4. Unit, `nova_ship`: `a_saved_view_refuses_a_non_finite_or_non_unit_view_or_an_out_of_range_zoom`.
5. P6 two-process probe:
   - Phase 1 turns the Normal rig, zooms, and holds FreeLook through a
     crossing save. At the leave save it records the camera
     `GlobalTransform` relative to the ship.
   - Phase 2 asserts the first frame that renders the player camera against
     that record (position and rotation, tolerance), then the Normal rig and
     the zoom.
   - P8 frames: the last frame before the leave and the first frame after
     the Load, compared by eye and by a pixel-difference figure. Lavapipe
     frames are deterministic for one scene.

Mutation checks:
- Skip the first-frame pose: proof 1 fails.
- Save the camera `Transform` instead of `GlobalTransform` minus shake:
  proof 2 fails on shake.
- Save the anchor rotation and zoom only, as in rev 2: proof 2 fails on
  Turret.
- Panic or skip on a missing camera: proof 3 fails.

## Decisions

- D-C1 Zoom saved: approved (yes).
- D-C2 No camera: approved as a visible wait and retry, Failed at the bound.
- D-C3 After Load the mode is Normal and the view eases from the saved pose
  in 0.45 s: accepted as reversible, pending this proof of first-frame
  fidelity.
- D-C4 Shake is not saved. Recommend: not saved (a jolt is transient).
