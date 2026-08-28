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
//! - the runner spawns one [`Screenshot`] of that image per tick, each with
//!   its own numbered [`save_to_disk`] observer, so capture completion order
//!   cannot scramble frame order.
//!
//! Captures complete asynchronously a frame or two after their tick;
//! [`flush_captures`] pumps the app after EOF until the last one lands.
//!
//! The PNGs stitch into a real-time movie regardless of how slowly the driver
//! stepped: `ffmpeg -framerate 60 -i <DIR>/frame_%06d.png -pix_fmt yuv420p
//! out.mp4` (one tick is 1/60 s of simulated time). The poc client
//! (`tasks/20260820-174148/poc/channel.py`) runs that automatically on close.

use std::path::PathBuf;

use bevy::{
    camera::{NormalizedRenderTarget, RenderTarget},
    picking::{pointer::PointerLocation, PickingSystems},
    prelude::*,
    render::view::window::screenshot::{save_to_disk, Screenshot},
    ui::IsDefaultUiCamera,
    window::{PrimaryWindow, WindowRef},
};

/// The armed recorder: the offscreen image the cameras draw into, the
/// directory the PNGs land in, and the counter that names them. Absent unless
/// the run was launched with `--record`.
#[derive(Resource)]
pub struct ChannelRecorder {
    /// The render target every primary-window camera is retargeted into.
    pub image: Handle<Image>,
    /// Where `frame_%06d.png` land.
    pub dir: PathBuf,
    /// Frames captured so far, which is also the next frame's number.
    pub frames: u64,
}

/// Arm the recorder: create the target image at the virtual window's size,
/// and install the two retargeting systems. Called from the plugin's `build`,
/// after the virtual window exists.
pub(crate) fn setup(app: &mut App, dir: PathBuf) {
    std::fs::create_dir_all(&dir).expect("the --record directory can be created");
    let mut windows = app
        .world_mut()
        .query_filtered::<&Window, With<PrimaryWindow>>();
    let (width, height) = windows
        .single(app.world())
        .map(|window| (window.physical_width(), window.physical_height()))
        .expect("the channel's build spawned the primary window before arming the recorder");
    let image = Image::new_target_texture(
        width,
        height,
        bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb,
        None,
    );
    let image = app.world_mut().resource_mut::<Assets<Image>>().add(image);
    app.insert_resource(ChannelRecorder {
        image,
        dir,
        frames: 0,
    });
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

/// Queue this tick's capture: one screenshot of the record image, saved under
/// the frame number the counter hands out. Called by the runner right before
/// the tick's `app.update()`, so the capture is of exactly that frame's
/// render. A no-op when the recorder is not armed.
pub(crate) fn record_frame(app: &mut App) {
    let world = app.world_mut();
    let Some(mut recorder) = world.get_resource_mut::<ChannelRecorder>() else {
        return;
    };
    let frame = recorder.frames;
    recorder.frames += 1;
    let image = recorder.image.clone();
    let path = recorder.dir.join(format!("frame_{frame:06}.png"));
    world
        .spawn(Screenshot::image(image))
        .observe(save_to_disk(path));
}

/// After the session: pump the app until the captures still in flight have
/// landed on disk. A capture needs a frame or two of queue submissions to
/// complete; the bound only exists so a wedged GPU cannot hang the exit.
pub(crate) fn flush_captures(app: &mut App) {
    if app.world().get_resource::<ChannelRecorder>().is_none() {
        return;
    }
    for _ in 0..60 {
        let mut screenshots = app.world_mut().query_filtered::<(), With<Screenshot>>();
        let pending = screenshots.iter(app.world()).count();
        if pending == 0 {
            return;
        }
        app.update();
    }
    let mut screenshots = app.world_mut().query_filtered::<(), With<Screenshot>>();
    let pending = screenshots.iter(app.world()).count();
    warn!("nova channel: {pending} frame captures never completed");
}
