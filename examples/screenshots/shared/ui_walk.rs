//! The pointer-gesture kit the menu, picker and editor walks share.
//!
//! Every gesture is a real pointer gesture, the way `examples/systems/menu_picker.rs`
//! does it: the pointer is moved to the widget's own screen position (resolved
//! from its `Name`) and pressed and released there. Nothing is reached by
//! triggering its observer, so a panel that stopped opening on a click fails the
//! run instead of quietly producing the previous shot again. Widgets act on
//! `Activate`, which fires on RELEASE over the same node, so every click here
//! spans two beats.
//!
//! Included by each walk with
//! `#[cfg(feature = "debug")] #[path = "shared/ui_walk.rs"] mod ui_walk;` -
//! everything here is script-only, so the whole module sits behind ONE gate at
//! the `mod` instead of an attribute per item. It lives one level down on
//! purpose - `catalog_matches_disk`
//! (`crates/nova_probe_cli/tests/catalog_drift.rs`) treats every `.rs` DIRECTLY
//! under a category dir as a cataloged example, so a sibling `ui_walk.rs` would
//! fail the catalog check.

// Each walk includes the whole kit and uses the part its script needs; the
// unused half is not dead code, it is another walk's tool.
#![allow(dead_code)]

use bevy::prelude::*;
use nova_protocol::prelude::*;

/// Seconds a step may sit before it is called a stall. Sized with headroom for a
/// slow software-rendered CI GPU (llvmpipe), where every beat costs more
/// wall-clock than on a real GPU. An expiry is an error exit naming the step,
/// which is the point: a walk that never reaches the menu or the editor now
/// fails loudly instead of idling out a fixed window.
pub const STEP_DEADLINE_SECS: f32 = 30.0;

/// Where the editor camera is put before the ship is built.
///
/// The editor's own camera sits at `(0, 5, 10)` looking down the -Z axis, dead
/// on: from there every SIDE face of a section is exactly edge-on, so the
/// picking ray can only ever hit a top or a front face and a ship can only be
/// built as a stack toward the lens. Off the axis, the +X, +Y and +Z faces are
/// all hittable, which is what lets the editor walk lay out a spine.
pub const EDITOR_EYE: Vec3 = Vec3::new(4.44, 3.17, 7.25);

/// What the editor camera looks at.
///
/// NOT the built ship's centre: the rail owns the left edge of the frame, so a
/// ship centred in the WINDOW sits left of centre in the picture. The look point
/// is pushed a little over a unit along the camera's screen-left, which slides
/// the ship the same distance right into the free part of the frame. (The push
/// was sized against the rail PLUS the 280 px component drawer; the drawer is
/// gone, so it now clears more room than it has to.)
pub const EDITOR_LOOK: Vec3 = Vec3::new(0.62, 0.25, 0.5);

/// Frames a pointer gesture on the SHIP gets: the picking backend needs a
/// frame to raycast the new pointer position and the editor's observers a
/// frame to react. Never shorter than the walk's other settles - a click that
/// lands before the raycast has moved places nothing, and this is the one part
/// of the walk that silently produces a thinner ship rather than failing.
pub const GESTURE_FRAMES: u32 = 12;

/// Put the editor's camera on [`EDITOR_EYE`] and PIN it there.
///
/// Pinned with a `ScriptedCameraPose` - the same component the scenario
/// `SetCamera` action and the scene captures use - rather than by writing the
/// Transform once. The editor camera is a free-fly WASD camera, and that
/// controller's state machine rewrites the Transform every frame whether or not
/// any key is down (removing the component does not stop it: its private state
/// survives), so a one-shot set is gone by the next frame. It was, and every
/// section the build placed landed on the face the ORIGINAL pose saw.
pub fn pose_editor_camera(world: &mut World) {
    let camera = world
        .query_filtered::<Entity, With<Camera3d>>()
        .iter(world)
        .next()
        .expect("the editor is up, so it has a 3D camera");
    world.entity_mut(camera).insert(ScriptedCameraPose {
        position: EDITOR_EYE,
        look_at: EDITOR_LOOK,
    });
}

/// Let the gallery park the editor camera on its isolated preview stage.
///
/// The scripted build pose must not survive while the gallery is open. It
/// otherwise overwrites the gallery's parked transform every frame and draws
/// the real ship behind the preview tiles, unlike the ordinary game editor.
pub fn release_editor_camera_pose(world: &mut World) {
    let cameras: Vec<Entity> = world
        .query_filtered::<Entity, With<Camera3d>>()
        .iter(world)
        .collect();
    for camera in cameras {
        world.entity_mut(camera).remove::<ScriptedCameraPose>();
    }
}

/// Fail the capture walk if the gallery did not move the editor camera away
/// from the build area.
pub fn assert_gallery_camera_is_parked(world: &mut World) {
    let camera = world
        .query_filtered::<&Transform, With<Camera3d>>()
        .iter(world)
        .next()
        .expect("the open gallery retains the editor camera");
    assert!(
        camera.translation.y > 1_000.0,
        "the open gallery must park its camera away from the preview ship; got {:?}",
        camera.translation
    );
}

/// Count the preview ship's sections.
pub fn count_sections(world: &mut World) -> usize {
    let mut q = world.query_filtered::<(), With<SectionMarker>>();
    q.iter(world).count()
}

/// Where to point the pointer to hit the `face` face of the section mounted at
/// `section`, in logical pixels.
///
/// Aims just INSIDE the face (sections are one unit apart, so a face sits half a
/// unit out): a ray to a point on the face plane itself grazes it, and the
/// editor places nothing without a hit normal. `world_to_viewport` answers in
/// logical pixels, which is the space [`move_cursor`] takes.
pub fn aim_at_face(world: &mut World, section: Vec3, face: Vec3) -> Vec2 {
    let target = section + face * 0.49;
    let camera_entity = world
        .query_filtered::<Entity, With<Camera3d>>()
        .iter(world)
        .next()
        .expect("the editor is up, so it has a 3D camera");
    let camera = world
        .get::<Camera>(camera_entity)
        .expect("a 3D camera has a Camera");
    let camera_transform = world
        .get::<GlobalTransform>(camera_entity)
        .expect("a camera has a global transform");
    camera
        .world_to_viewport(camera_transform, target)
        .unwrap_or_else(|err| panic!("the point {target} must be on screen for the shot: {err}"))
}

/// The two gesture shapes the walks are written in: a click on a WIDGET, and a
/// click on the SHIP.
///
/// An extension trait rather than free functions so a gesture reads in the
/// script as one line, the way `examples/ui/editor.rs` writes its ship clicks -
/// a press and a release spelled out at every call site buried what the walk
/// actually does.
pub trait Gestures {
    /// Press and release the pointer over the named widget. Widgets act on
    /// `Activate`, which fires on RELEASE over the same node.
    fn click(self, label: &str, name: &str) -> Self;

    /// Place a section on the ship: aim at the `face` face of the section
    /// mounted at `on`, press, release. The editor acts on `Pointer<Press>`, so
    /// the press does the work and the release only lets go.
    fn place(self, label: &str, on: Vec3, face: Vec3) -> Self;

    /// Arm `prototype` through the parts gallery - the editor's only parts
    /// picker. The gallery owns the keyboard while it is up, so the walk types
    /// the catalog id to narrow the grid to one tile, then Enter to focus it
    /// and Enter again to place it (which closes the gallery).
    fn arm(self, label: &str, prototype: &str) -> Self;
}

impl Gestures for nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    fn click(self, label: &str, name: &str) -> Self {
        self.step(format!("{label}: press"))
            .on_enter(click_named(name.to_string()))
            .until(frames(GESTURE_FRAMES))
            .add()
            .step(format!("{label}: release"))
            .on_enter(release_mouse(MouseButton::Left))
            .until(frames(GESTURE_FRAMES))
            .add()
    }

    fn place(self, label: &str, on: Vec3, face: Vec3) -> Self {
        self.step(format!("{label}: aim"))
            .on_enter(move |world: &mut World| {
                let at = aim_at_face(world, on, face);
                move_cursor(at)(world);
            })
            .until(frames(GESTURE_FRAMES))
            .add()
            .step(format!("{label}: press"))
            .on_enter(press_mouse(MouseButton::Left))
            .until(frames(GESTURE_FRAMES))
            .add()
            .step(format!("{label}: release"))
            .on_enter(release_mouse(MouseButton::Left))
            .until(frames(GESTURE_FRAMES))
            .add()
            // Per-gesture, not just the total at the end: a short final count
            // says the build missed, this says WHICH gesture missed.
            .step(format!("{label}: count"))
            .on_enter(|world: &mut World| {
                let sections = count_sections(world);
                info!("editor build: {sections} sections");
            })
            .until(frames(1))
            .add()
    }

    fn arm(self, label: &str, prototype: &str) -> Self {
        let filter = prototype.to_string();
        let mut script = self
            .step(format!("{label}: release the scripted build camera"))
            .on_enter(release_editor_camera_pose)
            .until(frames(1))
            .add()
            .click(
                &format!("{label}: open the gallery"),
                "Parts Gallery Category",
            )
            .step(format!("{label}: verify the gallery camera"))
            .on_enter(assert_gallery_camera_is_parked)
            .until(frames(1))
            .add()
            // The filter takes the keyboard only once it has the caret.
            .step(format!("{label}: put the caret in the filter"))
            .on_enter(press_key(KeyCode::Slash))
            .until(frames(GESTURE_FRAMES))
            .add()
            .step(format!("{label}: release /"))
            .on_enter(release_key(KeyCode::Slash))
            .until(frames(GESTURE_FRAMES))
            .add()
            .step(format!("{label}: filter to `{prototype}`"))
            .on_enter(move |world: &mut World| type_text(filter.clone())(world))
            .until(frames(GESTURE_FRAMES))
            .add();
        for beat in ["focus", "place"] {
            script = script
                .step(format!("{label}: Enter to {beat}"))
                .on_enter(press_key(KeyCode::Enter))
                .until(frames(GESTURE_FRAMES))
                .add()
                .step(format!("{label}: release Enter"))
                .on_enter(release_key(KeyCode::Enter))
                .until(frames(GESTURE_FRAMES))
                .add();
        }
        script
            .step(format!("{label}: the gallery closed"))
            .on_enter(|world: &mut World| {
                assert!(
                    ui_node_rect(world, "Parts Gallery").is_none(),
                    "placing from the gallery must close it - a shot taken with \
                     the overlay still up would be of the overlay"
                );
                pose_editor_camera(world);
            })
            .until(frames(SETTLE_FRAMES))
            .add()
    }
}
