//! The viewport camera: look, orbit, dolly, pan.
//!
//! Editor-only, because the shared `WASDCamera` in `nova_ship` flies on keys,
//! and the editor gives its letter keys to the tools.
//!
//! THE POSE IS THE CAMERA'S `Transform`, and nothing else. [`EditorRig`] holds
//! the gesture, never a private copy of the pose, so framing, a view preset and
//! the gallery's park are plain `Transform` writes. The rig writes only while a
//! gesture moves the camera.
//!
//! - RMB held: look about the eye.
//! - MMB or Alt+LMB held: orbit the pivot.
//! - Shift+MMB held: pan.
//! - Alt+RMB held, or the wheel with no gesture: dolly toward the pivot.
//!
//! A gesture starts only over the viewport, so a press on a panel is the
//! panel's. It ends with its button, or as soon as the keyboard, a pause or the
//! gallery takes the viewport away.

use avian3d::prelude::{ColliderAabb, Sensor};
use bevy::{
    ecs::system::SystemParam,
    input::mouse::{AccumulatedMouseMotion, MouseWheel},
    picking::pointer::{PointerId, PointerInteraction, PointerLocation},
    prelude::*,
    window::{CursorGrabMode, CursorOptions, PrimaryWindow},
};
use nova_events::units::prelude::*;
use nova_gameplay::prelude::{subtree_bounding_sphere, PauseStates};
use nova_input::prelude::{MousePath, MouseSensitivity};
use nova_ship::prelude::GalleryState;
use nova_ui::prelude::InputMode;

use crate::{
    config::{EditorCamera, SelectedNode},
    frame::CameraFraming,
};

/// Radians of turn per raw look unit. The gain the free camera slider in the
/// settings is measured against, so the slider means the same here as in flight.
const LOOK_GAIN: f32 = 0.1;

/// How close to straight up or down the camera may look, in radians. A pitch of
/// exactly a quarter turn has no yaw left to read back off the pose.
const PITCH_LIMIT: f32 = std::f32::consts::FRAC_PI_2 - 0.001;

/// What one wheel notch multiplies the distance to the pivot by. A ratio, so a
/// notch means the same on a crate and across a range.
const DOLLY_STEP: f32 = 0.85;

/// Wheel notches one pixel of Alt+RMB drag counts as.
const DOLLY_PER_PIXEL: f32 = 0.02;

/// How near a dolly may bring the eye to the pivot. A step past the pivot would
/// turn the next notch into a retreat.
const DOLLY_NEAREST: Meters = Meters(5.0);

/// The share of the distance to the pivot one pixel of pan covers, so a pan
/// keeps pace with the pointer near and far.
const PAN_PER_PIXEL: f32 = 0.0012;

/// What the camera is doing right now.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CameraGestureType {
    /// Nothing is held.
    #[default]
    Idle,
    /// RMB: turning the view about the eye.
    Look,
    /// MMB or Alt+LMB: turning about the pivot.
    Orbit,
    /// Alt+RMB: closing on the pivot.
    Dolly,
    /// Shift+MMB: sliding across the view.
    Pan,
}

/// The rig's own state. The pose is on the camera's `Transform`.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct EditorRig {
    /// The gesture in progress.
    pub(crate) gesture: CameraGestureType,
    /// What the gesture in progress turns about. Fixed for the gesture, so the
    /// camera does not swing when a pan moves the point it was aimed at.
    pivot: Vec3,
}

/// What the pointer is over, read once a frame from picking.
///
/// A resource rather than a query at each reader, because the rig, the wheel and
/// the empty-space click all ask the same two questions of the same hit.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct ViewportPointer {
    /// A UI node is the nearest thing under the pointer.
    pub(crate) over_ui: bool,
    /// Nothing is under the pointer: only the window answered.
    pub(crate) over_nothing: bool,
    /// Where the pointer is, in logical pixels.
    pub(crate) at: Option<Vec2>,
}

/// Read what is under the pointer.
pub(crate) fn sync_viewport_pointer(
    pointers: Query<(&PointerId, &PointerInteraction, &PointerLocation)>,
    ui: Query<(), With<Node>>,
    windows: Query<(), With<Window>>,
    mut viewport: ResMut<ViewportPointer>,
) {
    let mut wanted = ViewportPointer {
        over_nothing: true,
        ..default()
    };
    // The mouse alone: the terminal's CRT holds a pointer of its own off screen.
    for (_, interaction, location) in pointers.iter().filter(|(id, ..)| id.is_mouse()) {
        if let Some(location) = &location.location {
            wanted.at = Some(location.position);
        }
        if let Some((nearest, _)) = interaction.get_nearest_hit() {
            wanted.over_ui |= ui.contains(*nearest);
            wanted.over_nothing &= windows.contains(*nearest);
        }
    }
    if *viewport != wanted {
        *viewport = wanted;
    }
}

/// What the rig reads from the mouse and the keyboard.
#[derive(SystemParam)]
pub(crate) struct RigControls<'w> {
    keys: Res<'w, ButtonInput<KeyCode>>,
    mouse: Res<'w, ButtonInput<MouseButton>>,
    motion: Res<'w, AccumulatedMouseMotion>,
    sensitivity: Option<Res<'w, MouseSensitivity>>,
}

impl RigControls<'_> {
    fn alt(&self) -> bool {
        self.keys.any_pressed([KeyCode::AltLeft, KeyCode::AltRight])
    }

    fn shift(&self) -> bool {
        self.keys
            .any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight])
    }

    /// The gesture a button pressed this frame starts, if any.
    fn started(&self) -> Option<CameraGestureType> {
        if self.mouse.just_pressed(MouseButton::Right) {
            return Some(if self.alt() {
                CameraGestureType::Dolly
            } else {
                CameraGestureType::Look
            });
        }
        if self.mouse.just_pressed(MouseButton::Middle) {
            return Some(if self.shift() {
                CameraGestureType::Pan
            } else {
                CameraGestureType::Orbit
            });
        }
        (self.mouse.just_pressed(MouseButton::Left) && self.alt())
            .then_some(CameraGestureType::Orbit)
    }

    /// Whether the button that holds `gesture` is still down.
    fn holds(&self, gesture: CameraGestureType) -> bool {
        match gesture {
            CameraGestureType::Idle => false,
            CameraGestureType::Look | CameraGestureType::Dolly => {
                self.mouse.pressed(MouseButton::Right)
            }
            CameraGestureType::Orbit => self
                .mouse
                .any_pressed([MouseButton::Left, MouseButton::Middle]),
            CameraGestureType::Pan => self.mouse.pressed(MouseButton::Middle),
        }
    }

    /// The frame's mouse travel, in pixels.
    fn travel(&self) -> Vec2 {
        self.motion.delta
    }

    /// The frame's mouse travel as a look step.
    fn look(&self) -> Vec2 {
        let raw = self.sensitivity.as_deref().map_or_else(
            || MousePath::FreeCamera.default_raw(),
            |sensitivity| sensitivity.raw(MousePath::FreeCamera),
        );
        self.travel() * raw * LOOK_GAIN
    }
}

/// Whether the viewport is the builder's to move right now.
#[derive(SystemParam)]
pub(crate) struct RigGate<'w> {
    mode: Option<Res<'w, InputMode>>,
    pause: Option<Res<'w, State<PauseStates>>>,
    gallery: Res<'w, GalleryState>,
}

impl RigGate<'_> {
    fn open(&self) -> bool {
        self.mode
            .as_deref()
            .is_none_or(|mode| *mode == InputMode::Normal)
            && self
                .pause
                .as_deref()
                .is_none_or(|pause| *pause.get() == PauseStates::Unpaused)
            && !self.gallery.open
    }
}

/// What the rig turns about.
#[derive(SystemParam)]
pub(crate) struct RigAim<'w, 's> {
    selected: Res<'w, SelectedNode>,
    framing: ResMut<'w, CameraFraming>,
    q_children: Query<'w, 's, &'static Children>,
    q_bounds: Query<'w, 's, &'static ColliderAabb, Without<Sensor>>,
    q_poses: Query<'w, 's, &'static GlobalTransform, Without<EditorCamera>>,
}

impl RigAim<'_, '_> {
    /// The pivot: the middle of the selection, or the framed point ahead of the
    /// camera with nothing selected.
    fn pivot(&self, pose: &Transform) -> Vec3 {
        let mut sum = Vec3::ZERO;
        let mut count = 0.0;
        for node in self.selected.iter() {
            let centre = subtree_bounding_sphere(node, &self.q_children, &self.q_bounds)
                .map(|(centre, _)| centre)
                .or_else(|| {
                    self.q_poses
                        .get(node)
                        .ok()
                        .map(GlobalTransform::translation)
                });
            if let Some(centre) = centre {
                sum += centre;
                count += 1.0;
            }
        }
        if count > 0.0 {
            return sum / count;
        }
        pose.translation + pose.forward() * self.framing.distance.to_engine()
    }

    /// Remember where a gesture left the eye, so the next one and the distance
    /// gestures of the inspector read the new reach.
    fn record(&mut self, pivot: Vec3, eye: Vec3) {
        let wanted = CameraFraming::on(pivot, eye);
        if *self.framing != wanted {
            *self.framing = wanted;
        }
    }
}

/// `rotation` turned by a look step, with no roll and the pitch held short of
/// straight up or down.
fn turned(rotation: Quat, look: Vec2) -> Quat {
    let (yaw, pitch, _) = rotation.to_euler(EulerRot::YXZ);
    Quat::from_euler(
        EulerRot::YXZ,
        yaw - look.x,
        (pitch - look.y).clamp(-PITCH_LIMIT, PITCH_LIMIT),
        0.0,
    )
}

/// `pose` swung about `pivot` by a look step.
///
/// The eye goes round by the same rotation the view does, so the distance to the
/// pivot and the pivot's place on screen both hold.
fn orbited(pose: &Transform, pivot: Vec3, look: Vec2) -> Transform {
    let rotation = turned(pose.rotation, look);
    let swing = rotation * pose.rotation.inverse();
    Transform {
        translation: pivot + swing * (pose.translation - pivot),
        rotation,
        ..*pose
    }
}

/// `eye` moved `notches` wheel notches toward `pivot`. Negative backs away.
fn dollied(eye: Vec3, pivot: Vec3, notches: f32) -> Vec3 {
    let arm = eye - pivot;
    let reach = arm.length();
    let nearest = DOLLY_NEAREST.to_engine().min(reach);
    let wanted = (reach * DOLLY_STEP.powf(notches)).max(nearest);
    pivot + arm.normalize_or(Vec3::Z) * wanted
}

/// How far the view slides for `travel` pixels, at `reach` from the pivot. The
/// scene follows the pointer, so the eye goes the other way.
fn pan_step(pose: &Transform, reach: f32, travel: Vec2) -> Vec3 {
    (pose.up() * travel.y - pose.right() * travel.x) * reach * PAN_PER_PIXEL
}

/// The signed wheel notches this frame. Line and pixel units both arrive, so
/// only the sign of a message is a gesture.
fn notches(wheel: &mut MessageReader<MouseWheel>) -> i32 {
    wheel
        .read()
        .map(|event| match event.y {
            y if y > 0.0 => 1,
            y if y < 0.0 => -1,
            _ => 0,
        })
        .sum()
}

/// Show or free the cursor for a gesture change. Look owns the mouse, so the
/// cursor is locked for it; the other gestures leave it where the pointer is.
fn sync_cursor(cursor: Option<Single<&mut CursorOptions, With<PrimaryWindow>>>, looking: bool) {
    let Some(cursor) = cursor else {
        return;
    };
    let mut cursor = cursor.into_inner();
    cursor.grab_mode = if looking {
        CursorGrabMode::Locked
    } else {
        CursorGrabMode::None
    };
    cursor.visible = !looking;
}

/// Move the camera for the frame's input.
pub(crate) fn drive_editor_camera(
    controls: RigControls,
    gate: RigGate,
    pointer: Res<ViewportPointer>,
    mut aim: RigAim,
    mut wheel: MessageReader<MouseWheel>,
    camera: Option<Single<(&mut Transform, &mut EditorRig), With<EditorCamera>>>,
    cursor: Option<Single<&mut CursorOptions, With<PrimaryWindow>>>,
) {
    let notches = notches(&mut wheel);
    let Some(camera) = camera else {
        return;
    };
    let (mut pose, mut rig) = camera.into_inner();
    let was = rig.gesture;

    let open = gate.open();
    if was != CameraGestureType::Idle && !(open && controls.holds(was)) {
        if was != CameraGestureType::Look {
            aim.record(rig.pivot, pose.translation);
        }
        rig.gesture = CameraGestureType::Idle;
    }
    let free = open && !pointer.over_ui;
    if rig.gesture == CameraGestureType::Idle && free {
        if let Some(gesture) = controls.started() {
            rig.gesture = gesture;
            rig.pivot = aim.pivot(&pose);
        }
    }

    let mut wanted = *pose;
    match rig.gesture {
        CameraGestureType::Look => wanted.rotation = turned(wanted.rotation, controls.look()),
        CameraGestureType::Orbit => wanted = orbited(&wanted, rig.pivot, controls.look()),
        CameraGestureType::Dolly => {
            let pulled = -controls.travel().y * DOLLY_PER_PIXEL;
            wanted.translation = dollied(wanted.translation, rig.pivot, pulled);
        }
        CameraGestureType::Pan => {
            let reach = wanted.translation.distance(rig.pivot);
            let step = pan_step(&wanted, reach, controls.travel());
            wanted.translation += step;
            rig.pivot += step;
        }
        // The wheel always dollies: R and F turn a part in hand.
        CameraGestureType::Idle if free && notches != 0 => {
            let pivot = aim.pivot(&pose);
            wanted.translation = dollied(wanted.translation, pivot, notches as f32);
            aim.record(pivot, wanted.translation);
        }
        CameraGestureType::Idle => {}
    }
    if *pose != wanted {
        *pose = wanted;
    }

    let looking = rig.gesture == CameraGestureType::Look;
    if looking != (was == CameraGestureType::Look) {
        sync_cursor(cursor, looking);
    }
}

/// Register the rig. Headless rigs have no mouse, so it is gated on the inputs
/// it reads.
pub(crate) fn register(app: &mut App) {
    app.init_resource::<ViewportPointer>();
}

#[cfg(test)]
mod tests;
