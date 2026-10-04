//! The channel's frame recorder - `--record <DIR>`: every stepped tick is
//! drawn by the real render stack and saved as `<DIR>/frame_%06d.png`.
//!
//! The app arrives in the OFFSCREEN assembly (`AppBuilder::offscreen`): a GPU
//! and the full visual plugin stack, but no winit and no OS window. Nothing in
//! that assembly draws on its own - the cameras target the channel's virtual
//! `PrimaryWindow`, which has no surface, so the render graph skips them. This
//! module closes the loop:
//!
//! - every camera aimed at the primary window is retargeted into one offscreen
//!   image, sized to the virtual window so the UI lays out identically;
//! - the picking pointer's location is retargeted the same way, because the UI
//!   picking backend only hit-tests cameras whose target EQUALS the pointer's
//!   ([`bevy_ui` `picking_backend.rs`], target equality) - without this the
//!   pointer lane would go dead the moment the cameras moved;
//! - the shared loop recorder requests the image in `Last` of each driven tick,
//!   samples SFX at the same frame index, and drains on session exit.
//!
//! Frame zero and sidecar frame zero both describe the state after tick one.

use std::path::PathBuf;

use bevy::{
    camera::{NormalizedRenderTarget, RenderTarget},
    picking::{pointer::PointerLocation, PickingSystems},
    prelude::*,
    ui::IsDefaultUiCamera,
    window::{PrimaryWindow, WindowRef},
};
use nova_gameplay::prelude::new_render_target_image;

/// The offscreen image shared by the channel camera and pointer routes, plus
/// the session destination. Absent unless launched with `--record`.
#[derive(Resource)]
pub struct ChannelRecorder {
    /// The render target every primary-window camera is retargeted into.
    pub image: Handle<Image>,
    /// Directory reserved for this session's frames, sidecar and WebM.
    pub dir: PathBuf,
}

/// Arm the recorder: create the target image at the virtual window's size,
/// and install the two retargeting systems. Called from the plugin's `build`,
/// after the virtual window exists.
///
/// The image is born through the shared [`new_render_target_image`] recipe, so
/// the recorder cannot drift from the format rule the web build depends on, and
/// a window measuring zero yields a 1x1 target instead of a texture wgpu
/// refuses to allocate.
pub(crate) fn setup(app: &mut App, dir: PathBuf) {
    let mut windows = app
        .world_mut()
        .query_filtered::<&Window, With<PrimaryWindow>>();
    let size = windows
        .single(app.world())
        .map(|window| window.physical_size())
        .expect("the channel's build spawned the primary window before arming the recorder");
    let image = app
        .world_mut()
        .resource_mut::<Assets<Image>>()
        .add(new_render_target_image(size));
    app.insert_resource(ChannelRecorder { image, dir });
    app.add_systems(PostUpdate, retarget_cameras);
    // The slot matters: `PointerInput::receive` (ProcessInput) re-applies the
    // frame's window-targeted messages onto `PointerLocation`, so a rewrite
    // any earlier is clobbered on exactly the frames a gesture arrives.
    app.add_systems(
        PreUpdate,
        retarget_pointers
            .after(PickingSystems::ProcessInput)
            .before(PickingSystems::Backend),
    );
}

/// Marks the camera whose [`IsDefaultUiCamera`] is the recorder's doing, so
/// standing down never strips a marker the game itself placed.
#[derive(Component)]
struct RecorderUiCamera;

/// Aim every camera that targets the (surfaceless) primary window at the
/// record image instead - and keep the UI routed.
///
/// Retargeting alone kills the UI: with no [`IsDefaultUiCamera`] in the world,
/// `bevy_ui` falls back to the highest-order camera whose target IS the
/// primary window (`DefaultUiCamera::get`), which after retargeting is no
/// camera at all - no HUD in the frames, no UI hit-tests, dead pointer lane.
/// So the recorder marks the camera that fallback would have picked (same
/// `(order, entity)` ordering), and stands down whenever the game marks its
/// own (the menu ambience rig, the render-scale blit) - a second marker would
/// void both.
fn retarget_cameras(
    mut commands: Commands,
    recorder: Res<ChannelRecorder>,
    primary: Query<Entity, With<PrimaryWindow>>,
    mut cameras: Query<(
        Entity,
        &Camera,
        &mut RenderTarget,
        Has<IsDefaultUiCamera>,
        Has<RecorderUiCamera>,
    )>,
) {
    for (.., mut target, _, _) in &mut cameras {
        let windowed = match &*target {
            RenderTarget::Window(WindowRef::Primary) => true,
            RenderTarget::Window(WindowRef::Entity(window)) => primary.contains(*window),
            _ => false,
        };
        if windowed {
            *target = RenderTarget::Image(recorder.image.clone().into());
        }
    }

    let game_marked = cameras.iter().any(|(.., marked, ours)| marked && !ours);
    let fallback = cameras
        .iter()
        .filter(|(_, _, target, ..)| {
            matches!(&**target, RenderTarget::Image(image) if image.handle == recorder.image)
        })
        .max_by_key(|(entity, camera, ..)| (camera.order, *entity))
        .map(|(entity, ..)| entity);
    for (entity, _, _, marked, ours) in &cameras {
        let keep = !game_marked && Some(entity) == fallback;
        if ours && !keep {
            commands
                .entity(entity)
                .remove::<(IsDefaultUiCamera, RecorderUiCamera)>();
        }
        if keep && !marked {
            commands
                .entity(entity)
                .insert((IsDefaultUiCamera, RecorderUiCamera));
        }
    }
}

/// Follow the cameras: a pointer located on the primary window is re-located
/// onto the record image, same position (the image is window-sized at scale
/// 1.0), so target-equality picking keeps resolving. Runs every frame - the
/// pointer writer and the autopilot pin re-assert window locations per
/// gesture.
fn retarget_pointers(
    recorder: Res<ChannelRecorder>,
    primary: Query<Entity, With<PrimaryWindow>>,
    mut pointers: Query<&mut PointerLocation>,
) {
    for mut pointer in &mut pointers {
        let on_window = pointer.location.as_ref().is_some_and(|location| {
            matches!(
                &location.target,
                NormalizedRenderTarget::Window(window) if primary.contains(window.entity())
            )
        });
        if !on_window {
            continue;
        }
        if let Some(location) = pointer.location.as_mut() {
            location.target = NormalizedRenderTarget::Image(recorder.image.clone().into());
        }
    }
}

#[cfg(test)]
mod tests {
    use bevy::asset::AssetPlugin;

    use super::*;

    #[test]
    fn offscreen_camera_and_pointer_share_the_ui_target() {
        use bevy::picking::pointer::Location;
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.init_asset::<Image>();
        let window = app
            .world_mut()
            .spawn((Window::default(), PrimaryWindow))
            .id();
        setup(&mut app, std::env::temp_dir().join("channel-routing-only"));
        let camera = app.world_mut().spawn(Camera::default()).id();
        let pointer = app
            .world_mut()
            .spawn(PointerLocation::new(Location {
                target: RenderTarget::Window(WindowRef::Entity(window))
                    .normalize(Some(window))
                    .unwrap(),
                position: Vec2::new(25.0, 40.0),
            }))
            .id();
        app.update();
        let image = &app.world().resource::<ChannelRecorder>().image;
        assert_eq!(
            app.world().get::<RenderTarget>(camera).unwrap().as_image(),
            Some(image)
        );
        assert!(
            app.world().get::<IsDefaultUiCamera>(camera).is_some(),
            "the UI has an offscreen camera"
        );
        let location = app
            .world()
            .get::<PointerLocation>(pointer)
            .unwrap()
            .location()
            .unwrap();
        assert_eq!(location.position, Vec2::new(25.0, 40.0));
        assert!(
            matches!(&location.target, NormalizedRenderTarget::Image(target) if &target.handle == image),
            "picking uses the same image as the UI camera"
        );
    }

    /// wgpu refuses a zero-area texture, so arming the recorder against a
    /// window that measures zero must still hand it an allocatable target. The
    /// recorder used to pass the window's size straight through.
    #[test]
    fn the_recorder_arms_a_non_zero_target_for_a_zero_sized_window() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.init_asset::<Image>();
        app.world_mut().spawn((
            Window {
                resolution: (0, 0).into(),
                ..default()
            },
            PrimaryWindow,
        ));

        setup(&mut app, std::env::temp_dir().join("channel-target-only"));

        let handle = app.world().resource::<ChannelRecorder>().image.clone();
        let images = app.world().resource::<Assets<Image>>();
        let size = images
            .get(&handle)
            .expect("record target")
            .texture_descriptor
            .size;
        assert_eq!(
            size.width, 1,
            "a zero-wide window still needs a real texture"
        );
        assert_eq!(
            size.height, 1,
            "a zero-tall window still needs a real texture"
        );
    }
}
