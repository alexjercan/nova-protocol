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
//! Every beat THIS KIT builds waits on the app rather than on a frame count:
//! the widget laid out, the picking pointer registering the press, the editor
//! solving a placement, the camera reaching its pose. A frame is not a unit of
//! work - the same count was milliseconds on a workstation and most of a second
//! under lavapipe, and said nothing about whether the app had finished
//! reacting. Nothing here counts frames at all; the only frame count a walk
//! built on it should still need is pre-SHOT stillness, which is what
//! `SETTLE_FRAMES` is for.
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
#![allow(
    dead_code,
    reason = "one source, many example targets: what one producer leaves unused another needs, so no single build can fulfil an expectation"
)]

use std::sync::Arc;

use bevy::prelude::*;
use nova_protocol::{nova_debug::harness::Predicate, prelude::*};

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

/// How close the camera has to be to [`EDITOR_EYE`] before a beat calls the
/// build pose reached. Loose enough for float drift through the enforcer,
/// far tighter than the distance to the gallery's parking spot.
const POSE_EPSILON: f32 = 1e-2;

/// Put the editor's camera on [`EDITOR_EYE`] and PIN it there.
///
/// Pinned with a `ScriptedCameraPose` - the same component the scenario
/// `SetCamera` action and the scene captures use - rather than by writing the
/// Transform once. The editor camera is a free-fly WASD camera, and that
/// controller's state machine rewrites the Transform every frame whether or not
/// any key is down (removing the component does not stop it: its private state
/// survives), so a one-shot set is gone by the next frame. It was, and every
/// section the build placed landed on the face the ORIGINAL pose saw.
///
/// The pose is APPLIED by the loader's enforcer a system later, which is why
/// the beat that calls this holds on [`the_build_camera_is_posed`].
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
    let heights: Vec<f32> = world
        .query_filtered::<&Transform, With<Camera3d>>()
        .iter(world)
        .map(|camera| camera.translation.y)
        .collect();
    assert!(
        a_camera_is_parked(world),
        "the open gallery must park its camera away from the preview ship; \
         the cameras stood at {heights:?}"
    );
}

/// How high the gallery parks the editor camera above the build area.
const GALLERY_PARK_HEIGHT: f32 = 1_000.0;

/// Whether a 3D camera stands off the build area.
///
/// One rule, read by the predicate that ADVANCES the beat and by the assert
/// that then states it. Reading the first camera in one and any camera in the
/// other let the walk advance on one camera and fail on a different one.
fn a_camera_is_parked(world: &World) -> bool {
    world
        .try_query_filtered::<&Transform, With<Camera3d>>()
        .is_some_and(|mut cameras| {
            cameras
                .iter(world)
                .any(|camera| camera.translation.y > GALLERY_PARK_HEIGHT)
        })
}

/// Advance once the gallery has parked the editor camera off the build area -
/// what [`assert_gallery_camera_is_parked`] then states as a claim.
pub fn the_gallery_camera_is_parked() -> Arc<Predicate> {
    Arc::new(a_camera_is_parked)
}

/// Advance once the editor camera has REACHED the scripted build pose.
///
/// [`pose_editor_camera`] pins the pose; the loader's enforcer applies it a
/// system later. A beat that aims through the camera before then projects its
/// target through whatever pose the camera still holds - the gallery's parking
/// spot, a thousand units up - and misses the ship entirely.
pub fn the_build_camera_is_posed() -> Arc<Predicate> {
    Arc::new(|world: &World| {
        world
            .try_query_filtered::<&Transform, With<Camera3d>>()
            .is_some_and(|mut cameras| {
                cameras
                    .iter(world)
                    .any(|camera| camera.translation.abs_diff_eq(EDITOR_EYE, POSE_EPSILON))
            })
    })
}

/// Count the sections on the ship being EDITED.
///
/// The probe's own list, scoped to the edit context, NOT a sweep of every
/// `SectionMarker` in the world: a new document opens seeded with the stock
/// range (`node.rs`), so a sweep counts ten other hulls along with the one the
/// walk built. Zero before the editor is up.
///
/// `&World`, so one counter serves both the beats that read it and the
/// predicates that wait on it.
pub fn count_sections(world: &World) -> usize {
    world
        .get_resource::<EditorProbe>()
        .map_or(0, |probe| probe.ship.len())
}

/// Advance once the ship being edited HAS a section - what founding a blank
/// ship leaves behind.
pub fn the_ship_is_up() -> Arc<Predicate> {
    Arc::new(|world: &World| count_sections(world) > 0)
}

/// Advance once the editor is inside a ship - what Add Ship does.
pub fn the_editor_is_inside_a_ship() -> Arc<Predicate> {
    Arc::new(|world: &World| {
        world
            .get_resource::<EditorProbe>()
            .is_some_and(|probe| probe.inside.is_some())
    })
}

/// Advance once Play would hand off - the editor is out at the scenario node,
/// which is the only place Play compiles the document from.
pub fn the_editor_can_play() -> Arc<Predicate> {
    Arc::new(|world: &World| {
        world
            .get_resource::<EditorProbe>()
            .is_some_and(|probe| probe.can_play)
    })
}

/// Where a founding click lands, in logical pixels on the 1920x1080 capture
/// window: clear of the ship, the rail, the top bar AND the inspector down the
/// right edge, so nothing is under the pointer - which is the editor's own test
/// for "found here".
pub const FOUND_CLICK: Vec2 = Vec2::new(1300.0, 900.0);

/// The derived skin, on the ship the walk is building.
pub fn the_skin_is_on() -> Arc<Predicate> {
    Arc::new(|world: &World| {
        world
            .try_query_filtered::<(), With<ShipSkinMarker>>()
            .is_some_and(|mut plates| plates.iter(world).next().is_some())
    })
}

/// The section count a [`Gestures::place`] beat measured before its gesture, so
/// the beat after it can wait for exactly one more.
///
/// Inserted on first use rather than registered by each walk: this kit is
/// `#[path]`-included, and a resource three examples had to remember to add
/// would be three chances to forget one.
#[derive(Resource, Default)]
pub struct BuildTally(pub usize);

/// Advance once the ship carries one more section than the last [`BuildTally`].
pub fn the_section_landed() -> Arc<Predicate> {
    Arc::new(|world: &World| {
        world
            .get_resource::<BuildTally>()
            .is_some_and(|tally| count_sections(world) == tally.0 + 1)
    })
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
    /// Press and release the pointer over the named widget, once it is on
    /// screen. Widgets act on `Activate`, which fires on RELEASE over the same
    /// node.
    ///
    /// The layout wait is the one a frame count hid: `click_named` warns and
    /// CONTINUES when the name resolves to nothing, so a press fired at a panel
    /// that has not laid out yet is a beat silently lost and the walk fails
    /// somewhere else.
    fn click(self, label: &str, name: &str) -> Self;

    /// Place a section on the ship: aim at the `face` face of the section
    /// mounted at `on`, press, release. The editor acts on `Pointer<Press>`, so
    /// the press does the work and the release only lets go.
    ///
    /// The aim holds until the EDITOR has solved a placement there, and the
    /// last beat until the section has landed - so a gesture that missed its
    /// face fails at that gesture rather than three shots later on a thin ship.
    fn place(self, label: &str, on: Vec3, face: Vec3) -> Self;

    /// Arm `prototype` through the parts gallery - the editor's only parts
    /// picker. The gallery owns the keyboard while it is up, so the walk types
    /// the catalog id to narrow the grid to one tile, then Enter to focus it
    /// and Enter again to place it (which closes the gallery).
    fn arm(self, label: &str, prototype: &str) -> Self;

    /// FOUND a blank ship: with a part armed, click empty space. The editor
    /// drops the first section at the ship's own origin, because a blank ship
    /// has no view for a mate ray to hit.
    fn found(self, label: &str) -> Self;
}

impl Gestures for nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    fn click(self, label: &str, name: &str) -> Self {
        let target = name.to_string();
        self.step(format!("{label}: the widget is up"))
            .until(ui_node_present(name.to_string()))
            .deadline(STEP_DEADLINE_SECS)
            .add()
            .step(format!("{label}: press"))
            .on_enter(click_named(target))
            .until(pointer_pressed())
            .deadline(STEP_DEADLINE_SECS)
            .add()
            .step(format!("{label}: release"))
            .on_enter(release_mouse(MouseButton::Left))
            .until(pointer_released())
            .deadline(STEP_DEADLINE_SECS)
            .add()
    }

    fn place(self, label: &str, on: Vec3, face: Vec3) -> Self {
        self.step(format!("{label}: aim"))
            .on_enter(move |world: &mut World| {
                let at = aim_at_face(world, on, face);
                move_cursor(at)(world);
                let count = count_sections(world);
                world.insert_resource(BuildTally(count));
            })
            .until(editor_placement_solved())
            .deadline(STEP_DEADLINE_SECS)
            .add()
            .step(format!("{label}: press"))
            .on_enter(press_mouse(MouseButton::Left))
            .until(pointer_pressed())
            .deadline(STEP_DEADLINE_SECS)
            .add()
            .step(format!("{label}: release"))
            .on_enter(release_mouse(MouseButton::Left))
            .until(pointer_released())
            .deadline(STEP_DEADLINE_SECS)
            .add()
            // Per-gesture, not just the total at the end: a short final count
            // says the build missed, this says WHICH gesture missed.
            .step(format!("{label}: it built"))
            .until(the_section_landed())
            .deadline(STEP_DEADLINE_SECS)
            .add()
            .step(format!("{label}: count"))
            .on_enter(|world: &mut World| {
                let sections = count_sections(world);
                info!("editor build: {sections} sections");
            })
            .add()
    }

    fn found(self, label: &str) -> Self {
        self.step(format!("{label}: point at empty space"))
            .on_enter(|world: &mut World| {
                move_cursor(FOUND_CLICK)(world);
                let count = count_sections(world);
                world.insert_resource(BuildTally(count));
            })
            // Nothing under the pointer is the editor's own founding test, and
            // "no placement to solve" is how it says so.
            .until(editor_placement_clear())
            .deadline(STEP_DEADLINE_SECS)
            .add()
            .step(format!("{label}: press"))
            .on_enter(press_mouse(MouseButton::Left))
            .until(pointer_pressed())
            .deadline(STEP_DEADLINE_SECS)
            .add()
            .step(format!("{label}: release"))
            .on_enter(release_mouse(MouseButton::Left))
            .until(pointer_released())
            .deadline(STEP_DEADLINE_SECS)
            .add()
            .step(format!("{label}: it founded"))
            .until(the_section_landed())
            .deadline(STEP_DEADLINE_SECS)
            .add()
    }

    fn arm(self, label: &str, prototype: &str) -> Self {
        let filter = prototype.to_string();
        let narrowed = prototype.to_string();
        let armed = prototype.to_string();
        self.step(format!("{label}: release the scripted build camera"))
            .on_enter(release_editor_camera_pose)
            .add()
            .click(&format!("{label}: open the Ship menu"), "Ship Menu Button")
            .click(&format!("{label}: open the gallery"), "Parts Item")
            .step(format!("{label}: the gallery parked the camera"))
            .until(and(editor_gallery_open(), the_gallery_camera_is_parked()))
            .deadline(STEP_DEADLINE_SECS)
            .add()
            .step(format!("{label}: verify the gallery camera"))
            .on_enter(assert_gallery_camera_is_parked)
            .add()
            // The filter takes the keyboard only once it has the caret.
            .step(format!("{label}: put the caret in the filter"))
            .on_enter(press_key(KeyCode::Slash))
            .until(editor_filter_focused())
            .deadline(STEP_DEADLINE_SECS)
            .add()
            .step(format!("{label}: release /"))
            .on_enter(release_key(KeyCode::Slash))
            .add()
            // Typed, then WAITED on: the gallery's selection resolving to this
            // id through the live filter is the honest end of "type enough to
            // leave one tile".
            .step(format!("{label}: filter to `{prototype}`"))
            .on_enter(move |world: &mut World| type_text(filter.clone())(world))
            .until(editor_gallery_selected(narrowed))
            .deadline(STEP_DEADLINE_SECS)
            .add()
            .step(format!("{label}: Enter to focus"))
            .on_enter(press_key(KeyCode::Enter))
            .until(ui_node_present("Gallery Focus Card"))
            .deadline(STEP_DEADLINE_SECS)
            .add()
            .step(format!("{label}: release Enter"))
            .on_enter(release_key(KeyCode::Enter))
            .add()
            .step(format!("{label}: Enter to place"))
            .on_enter(press_key(KeyCode::Enter))
            .until(and(
                editor_gallery_closed(),
                editor_tool_is(EditorTool::Place(armed)),
            ))
            .deadline(STEP_DEADLINE_SECS)
            .add()
            .step(format!("{label}: release Enter"))
            .on_enter(release_key(KeyCode::Enter))
            .add()
            .step(format!("{label}: the gallery closed"))
            .on_enter(|world: &mut World| {
                assert!(
                    ui_node_rect(world, "Parts Gallery").is_none(),
                    "placing from the gallery must close it - a shot taken with \
                     the overlay still up would be of the overlay"
                );
                pose_editor_camera(world);
            })
            // The camera has to REACH the build pose before the next beat aims
            // through it; the enforcer applies the pin a system later.
            .until(the_build_camera_is_posed())
            .deadline(STEP_DEADLINE_SECS)
            .add()
    }
}
