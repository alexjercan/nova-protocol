//! Camera, screenshot, and skybox actions: the presentation levers a beat pulls.

use bevy::{
    prelude::*,
    render::{
        render_resource::{TextureViewDescriptor, TextureViewDimension},
        view::screenshot::{save_to_disk, Screenshot},
    },
};
use nova_events::prelude::*;
use nova_gameplay::prelude::*;

use crate::prelude::*;

/// Pose the scenario camera (the [`ScenarioCameraMarker`] entity) at `position`
/// looking at `look_at` by pinning a [`ScriptedCameraPose`] on it (and dropping
/// [`WASDCameraController`] so free-fly input stops). The pose is enforced every
/// frame after the WASD sync, so it holds even though the controller's state
/// machine keeps writing the Transform - a one-shot set would be overwritten,
/// and removing the controller does not stop it (its private state components
/// survive). A no-op with a warning when no scenario camera is present (e.g. a
/// headless rig without the loader's camera).
///
/// Part of the in-engine photo-mode surface, paired with
/// [`ScreenshotActionConfig`]: a beat poses the camera, settles, then captures.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SetCameraActionConfig {
    /// World-space camera position.
    pub position: Vec3,
    /// World-space point the camera looks at (up is +Y).
    pub look_at: Vec3,
}

impl EventAction<NovaEventWorld> for SetCameraActionConfig {
    fn action(&self, world: &mut NovaEventWorld, _: &GameEventInfo) {
        let position = self.position;
        let look_at = self.look_at;
        debug!("SetCamera: position {:?} look_at {:?}", position, look_at);

        world.push_command(move |commands| {
            commands.queue(move |world: &mut World| {
                // Resolve the camera before taking a mutable borrow (the query's
                // immutable borrow of `world` ends with this block).
                let camera = {
                    let mut query = world.query_filtered::<Entity, With<ScenarioCameraMarker>>();
                    query.iter(world).next()
                };
                let Some(camera) = camera else {
                    warn!("SetCamera: no scenario camera present; nothing to pose");
                    return;
                };

                if let Ok(mut entity) = world.get_entity_mut(camera) {
                    // Drop free-fly input and pin the scripted pose; the loader's
                    // enforcer applies it after the WASD sync every frame.
                    entity.remove::<WASDCameraController>();
                    entity.insert(ScriptedCameraPose { position, look_at });
                }
            });
        });
    }
}

/// Resolve a screenshot output path. Absolute paths are used as-is; a relative
/// path is joined under the `NOVA_SHOT_DIR` env var when set (so an example or a
/// packaging script can redirect all captures to a staging folder), else it is
/// relative to the process working directory.
fn resolve_capture_path(path: &str) -> std::path::PathBuf {
    let dir = std::env::var("NOVA_SHOT_DIR")
        .ok()
        .filter(|dir| !dir.is_empty());
    resolve_capture_path_in(path, dir.as_deref())
}

/// Pure core of [`resolve_capture_path`], with the capture dir passed in so it
/// is testable without mutating the process environment.
fn resolve_capture_path_in(path: &str, capture_dir: Option<&str>) -> std::path::PathBuf {
    let path = std::path::Path::new(path);
    match capture_dir {
        Some(dir) if !path.is_absolute() => std::path::Path::new(dir).join(path),
        _ => path.to_path_buf(),
    }
}

/// Capture the primary window to a PNG at `path` (photo mode). Relative paths
/// resolve under `NOVA_SHOT_DIR` (see `resolve_capture_path`). Built on Bevy's
/// built-in `Screenshot::primary_window()` + `save_to_disk` observer - the same
/// primitive the screenshot harness uses - so no capture dependency is added.
/// The parent directory is created if missing; a capture on a build without a
/// render backend simply never lands, which is acceptable for a dev/marketing
/// tool.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ScreenshotActionConfig {
    /// Output PNG path (relative paths resolve under `NOVA_SHOT_DIR`).
    pub path: String,
}

impl ScreenshotActionConfig {
    /// Construct from a string slice.
    pub fn new(path: &str) -> Self {
        Self {
            path: path.to_string(),
        }
    }
}

impl EventAction<NovaEventWorld> for ScreenshotActionConfig {
    fn action(&self, world: &mut NovaEventWorld, _: &GameEventInfo) {
        let path = self.path.clone();
        debug!("Screenshot: capturing to '{}'", path);

        world.push_command(move |commands| {
            commands.queue(move |world: &mut World| {
                let resolved = resolve_capture_path(&path);
                if let Some(parent) = resolved.parent() {
                    if !parent.as_os_str().is_empty() {
                        if let Err(error) = std::fs::create_dir_all(parent) {
                            warn!(
                                "Screenshot: could not create capture dir {:?}: {error}",
                                parent
                            );
                        }
                    }
                }
                world
                    .spawn(Screenshot::primary_window())
                    .observe(save_to_disk(resolved));
            });
        });
    }
}

/// Fallback skybox brightness, matching the value the loader spawns the scenario
/// camera with (`loader.rs`). Only used if a swap targets a camera that somehow
/// has no current `SkyboxConfig` to inherit brightness from.
const DEFAULT_SKYBOX_BRIGHTNESS: f32 = 1000.0;

/// Swap the scenario's skybox cubemap mid-scenario. A modding hook: a beat can
/// change the sky by authoring a new cubemap path, resolved through the same
/// [`AssetRef`] path-or-handle layer the RON format uses for the initial
/// `cubemap`.
///
/// The cubemap cannot be applied synchronously: the skybox setup observer in
/// `bevy_common_systems` reads the image out of `Assets<Image>` the instant a
/// `SkyboxConfig` is inserted and panics if it is not loaded yet - and a
/// freshly-referenced modder path is not. So the action only *tags* the
/// scenario camera with a [`PendingSkyboxSwap`]; [`apply_pending_skybox_swaps`]
/// inserts the real `SkyboxConfig` once the image has finished loading.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SetSkyboxActionConfig {
    /// The new cubemap image, authored as an asset path (e.g.
    /// `"scenarios/space.cube.png"`) or a live handle in code-built configs.
    pub cubemap: AssetRef<Image>,
    /// Optional brightness multiplier. `None` keeps the current skybox brightness.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub brightness: Option<f32>,
}

impl SetSkyboxActionConfig {
    /// Construct a swap to `cubemap`, keeping the current brightness.
    pub fn new(cubemap: impl Into<AssetRef<Image>>) -> Self {
        Self {
            cubemap: cubemap.into(),
            brightness: None,
        }
    }
}

impl EventAction<NovaEventWorld> for SetSkyboxActionConfig {
    fn action(&self, world: &mut NovaEventWorld, _: &GameEventInfo) {
        let cubemap = self.cubemap.clone();
        let brightness = self.brightness;
        debug!("SetSkybox: cubemap {:?}", cubemap.path());

        world.push_command(move |commands| {
            commands.queue(move |world: &mut World| {
                // Start the load (idempotent for an already-resolved handle).
                let handle = {
                    let asset_server = world.resource::<AssetServer>();
                    cubemap.resolve(asset_server)
                };

                // Resolve the camera before taking a mutable borrow.
                let camera = {
                    let mut query = world.query_filtered::<Entity, With<ScenarioCameraMarker>>();
                    query.iter(world).next()
                };
                let Some(camera) = camera else {
                    warn!("SetSkybox: no scenario camera present; nothing to swap");
                    return;
                };

                if let Ok(mut entity) = world.get_entity_mut(camera) {
                    // NOTE: do NOT insert SkyboxConfig here - the setup observer
                    // would read the not-yet-loaded image and panic. Tag for the
                    // deferred applier instead.
                    entity.insert(PendingSkyboxSwap {
                        cubemap: handle,
                        brightness,
                    });
                }
            });
        });
    }
}

/// A requested skybox swap waiting on its cubemap image to finish loading. Set by
/// [`SetSkyboxActionConfig`], consumed by [`apply_pending_skybox_swaps`].
#[derive(Component, Clone, Debug, Reflect)]
pub struct PendingSkyboxSwap {
    /// The (loading) cubemap to install once it is present in `Assets<Image>`.
    pub cubemap: Handle<Image>,
    /// Brightness override, or `None` to keep the camera's current brightness.
    pub brightness: Option<f32>,
}

/// Applies a [`PendingSkyboxSwap`] once its cubemap image is available.
///
/// Readiness is "present in `Assets<Image>`" rather than the asset server's
/// load state, because that is exactly what the skybox setup observer needs to
/// read - and it also lets code-built swaps (a handle added straight to
/// `Assets`) apply without a server round-trip. A load the *server* reports as
/// failed is dropped with a warning so a bad modder path leaves the sky
/// unchanged instead of waiting forever; the action always resolves through a
/// server load, so that covers every real swap (a bare code-built handle that
/// is never added would wait indefinitely, but nothing constructs one).
///
/// A cubemap that arrives ALREADY multi-layer (its `.meta` `array_layout`
/// applied at load time - now every cubemap with a sidecar, base or mod, since
/// `assets_plugin` reads metas with `AssetMetaCheck::Always`) skips the bcs
/// setup observer's single-layer fallback branch, which is also where the Cube
/// texture view was set. Without the view, bevy's skybox sanity check
/// (`sanity_check_skybox_image_and_warn` in bevy_core_pipeline's skybox module)
/// refuses the non-Cube view with a `warn_once` and withholds the skybox bind
/// group - the sky silently disappears. So the applier sets the view itself
/// before installing the config. The write happens only when the view is
/// actually missing: writing through the `AssetMut` guard queues
/// `AssetEvent::Modified` (a full re-upload of the hundreds-of-MB cubemap
/// texture), so the no-change path must provably not write.
pub fn apply_pending_skybox_swaps(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut images: ResMut<Assets<Image>>,
    q_pending: Query<(Entity, &PendingSkyboxSwap, Option<&SkyboxConfig>)>,
) {
    for (entity, pending, current) in &q_pending {
        if images.contains(&pending.cubemap) {
            let needs_cube_view = images.get(&pending.cubemap).is_some_and(|image| {
                image.texture_descriptor.array_layer_count() > 1
                    && image.texture_view_descriptor.is_none()
            });
            if needs_cube_view {
                if let Some(mut image) = images.get_mut(&pending.cubemap) {
                    image.texture_view_descriptor = Some(TextureViewDescriptor {
                        dimension: Some(TextureViewDimension::Cube),
                        ..default()
                    });
                }
            }
            let brightness = pending
                .brightness
                .or_else(|| current.map(|config| config.brightness))
                .unwrap_or(DEFAULT_SKYBOX_BRIGHTNESS);
            debug!("SetSkybox: cubemap loaded, installing (brightness {brightness})");
            commands
                .entity(entity)
                .remove::<PendingSkyboxSwap>()
                .insert(SkyboxConfig {
                    cubemap: pending.cubemap.clone(),
                    brightness,
                });
        } else if asset_server.load_state(&pending.cubemap).is_failed() {
            warn!("SetSkybox: cubemap failed to load; leaving the skybox unchanged");
            commands.entity(entity).remove::<PendingSkyboxSwap>();
        }
        // else: still loading - keep the tag and check again next frame.
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The skybox swap is two-step on purpose: the bcs skybox setup observer
    /// reads the cubemap out of `Assets<Image>` the instant a `SkyboxConfig` is
    /// inserted and panics on an unloaded handle, so
    /// `apply_pending_skybox_swaps` holds the `PendingSkyboxSwap` until the
    /// image is present, then installs the config - inheriting the camera's
    /// current brightness unless the swap overrides it.
    #[test]
    fn skybox_swap_waits_for_load_then_installs() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, bevy::asset::AssetPlugin::default()));
        app.init_asset::<Image>();
        app.add_systems(Update, apply_pending_skybox_swaps);
        app.finish();

        // A scenario camera already showing a skybox at brightness 500.
        let initial = app
            .world_mut()
            .resource_mut::<Assets<Image>>()
            .add(Image::default());
        let camera = app
            .world_mut()
            .spawn((
                ScenarioCameraMarker,
                SkyboxConfig {
                    cubemap: initial.clone(),
                    brightness: 500.0,
                },
            ))
            .id();

        // Swap to a cubemap that has NOT loaded yet: reserve an id with no asset.
        let loading = app
            .world_mut()
            .resource_mut::<Assets<Image>>()
            .reserve_handle();
        app.world_mut()
            .entity_mut(camera)
            .insert(PendingSkyboxSwap {
                cubemap: loading.clone(),
                brightness: None,
            });

        // While the image is absent, the swap stays pending and the sky is unchanged.
        app.update();
        assert!(
            app.world().get::<PendingSkyboxSwap>(camera).is_some(),
            "swap must stay pending until the cubemap loads"
        );
        assert_eq!(
            app.world().get::<SkyboxConfig>(camera).unwrap().cubemap,
            initial,
            "skybox must not change while the new cubemap is still loading"
        );

        // The image arrives (load finishes) -> the applier installs it and clears
        // the tag, inheriting brightness 500 because the swap did not override it.
        app.world_mut()
            .resource_mut::<Assets<Image>>()
            .insert(loading.id(), Image::default())
            .expect("inserting the loaded cubemap asset");
        app.update();
        assert!(
            app.world().get::<PendingSkyboxSwap>(camera).is_none(),
            "swap must be consumed once the cubemap is present"
        );
        let config = app.world().get::<SkyboxConfig>(camera).unwrap();
        assert_eq!(
            config.cubemap, loading,
            "cubemap must swap to the new handle"
        );
        assert_eq!(
            config.brightness, 500.0,
            "brightness must be inherited when the swap does not set it"
        );

        // An explicit brightness overrides the inherited one.
        let bright = app
            .world_mut()
            .resource_mut::<Assets<Image>>()
            .add(Image::default());
        app.world_mut()
            .entity_mut(camera)
            .insert(PendingSkyboxSwap {
                cubemap: bright.clone(),
                brightness: Some(250.0),
            });
        app.update();
        let config = app.world().get::<SkyboxConfig>(camera).unwrap();
        assert_eq!(config.cubemap, bright);
        assert_eq!(
            config.brightness, 250.0,
            "an explicit brightness must override the inherited one"
        );
    }

    /// Builds the applier's minimal rig: assets + the applier, no bcs observer
    /// (its behavior is pinned by the skybox_swap_e2e integration test).
    fn skybox_applier_app() -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, bevy::asset::AssetPlugin::default()));
        app.init_asset::<Image>();
        app.add_systems(Update, apply_pending_skybox_swaps);
        app.finish();
        app
    }

    /// A 6 layer array image the way a meta'd cubemap comes out of the loader:
    /// stacked, then reinterpreted - `texture_view_descriptor` still `None`.
    fn six_layer_image() -> Image {
        use bevy::{
            asset::RenderAssetUsages,
            render::render_resource::{Extent3d, TextureDimension, TextureFormat},
        };
        let mut image = Image::new_fill(
            Extent3d {
                width: 1,
                height: 6,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            &[0, 0, 0, 255],
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::all(),
        );
        let _ = image.reinterpret_stacked_2d_as_array(6);
        assert_eq!(
            image.texture_descriptor.array_layer_count(),
            6,
            "rig sanity: the stacked reinterpret produced the 6 layer array"
        );
        image
    }

    /// A cubemap that arrives ALREADY 6-layer (its `.meta` `array_layout`
    /// applied at load time, e.g. `base/textures/cubemap_alt.png` through
    /// `assets_plugin`) skips the bcs observer's single-layer fallback - the
    /// branch that also set the Cube texture view. The applier must set the
    /// view itself, or bevy's skybox sanity check refuses the non-Cube view
    /// (`warn_once`) and skips rendering - the sky silently disappears.
    #[test]
    fn skybox_swap_sets_cube_view_on_a_preinterpreted_cubemap() {
        let mut app = skybox_applier_app();

        let cubemap = app
            .world_mut()
            .resource_mut::<Assets<Image>>()
            .add(six_layer_image());
        let camera = app
            .world_mut()
            .spawn((
                ScenarioCameraMarker,
                PendingSkyboxSwap {
                    cubemap: cubemap.clone(),
                    brightness: Some(700.0),
                },
            ))
            .id();

        app.update();

        // The swap landed...
        assert_eq!(
            app.world()
                .get::<SkyboxConfig>(camera)
                .expect("the applier installs the SkyboxConfig")
                .cubemap,
            cubemap
        );
        // ...and the applier readied the image for bevy's Cube skybox binding.
        let images = app.world().resource::<Assets<Image>>();
        let image = images.get(&cubemap).expect("cubemap is in Assets");
        assert_eq!(
            image
                .texture_view_descriptor
                .as_ref()
                .and_then(|descriptor| descriptor.dimension),
            Some(TextureViewDimension::Cube),
            "an already-arrayed cubemap must get its Cube view from the applier"
        );
    }

    /// The applier must not WRITE to an image whose Cube view is already set
    /// (the preloaded `GameAssets` cubemap after `prepare_cubemap_view`): a
    /// write through the `AssetMut` guard queues `AssetEvent::Modified`, which
    /// re-uploads the hundreds-of-MB cubemap texture for nothing.
    #[test]
    fn skybox_swap_does_not_remodify_an_already_cubed_image() {
        let mut app = skybox_applier_app();

        let mut cubed = six_layer_image();
        cubed.texture_view_descriptor = Some(TextureViewDescriptor {
            dimension: Some(TextureViewDimension::Cube),
            ..default()
        });
        let cubemap = app.world_mut().resource_mut::<Assets<Image>>().add(cubed);
        let camera = app
            .world_mut()
            .spawn((
                ScenarioCameraMarker,
                PendingSkyboxSwap {
                    cubemap: cubemap.clone(),
                    brightness: None,
                },
            ))
            .id();

        app.update();

        assert!(
            app.world().get::<SkyboxConfig>(camera).is_some(),
            "rig sanity: the applier consumed the swap"
        );
        let events: Vec<_> = app
            .world_mut()
            .resource_mut::<Messages<AssetEvent<Image>>>()
            .drain()
            .collect();
        // Delivery guard: the `.add()` above must have produced an Added event
        // in the drained buffer, or the no-Modified assertion below would be
        // vacuously green whenever asset events stop reaching this resource.
        assert!(
            events
                .iter()
                .any(|e| matches!(e, AssetEvent::Added { id } if *id == cubemap.id())),
            "rig sanity: the add's Added event reaches the drained messages: {events:?}"
        );
        assert!(
            !events
                .iter()
                .any(|e| matches!(e, AssetEvent::Modified { id } if *id == cubemap.id())),
            "consuming a swap for an already-cubed image must not emit Modified \
             (a Modified re-uploads the whole cubemap texture): {events:?}"
        );
    }

    /// SetCamera pins a `ScriptedCameraPose` on the scenario camera and drops
    /// WASD control (the loader's enforcer then applies the pose every frame, so
    /// it holds against the free-fly state machine). Mirrors the despawn harness:
    /// fire into a `NovaEventWorld`, drain, assert on the world.
    #[test]
    fn set_camera_pins_a_scripted_pose_and_drops_wasd() {
        use bevy_common_systems::prelude::WASDCameraController;
        use nova_events::prelude::EventWorld;

        use crate::prelude::{ScenarioCameraMarker, ScriptedCameraPose};

        let mut world = World::new();
        world.init_resource::<NovaEventWorld>();
        world.init_resource::<GameObjectives>();

        let camera = world
            .spawn((
                ScenarioCameraMarker,
                WASDCameraController,
                Transform::from_xyz(0.0, 10.0, 20.0),
            ))
            .id();

        let action = SetCameraActionConfig {
            position: Vec3::new(5.0, 6.0, 7.0),
            look_at: Vec3::ZERO,
        };
        let mut event_world = world.resource_mut::<NovaEventWorld>();
        action.action(&mut event_world, &GameEventInfo::default());
        NovaEventWorld::state_to_world_system(&mut world);

        let pose = world
            .get::<ScriptedCameraPose>(camera)
            .expect("the camera is pinned to a scripted pose");
        assert_eq!(pose.position, Vec3::new(5.0, 6.0, 7.0));
        assert_eq!(pose.look_at, Vec3::ZERO);
        assert!(
            world.get::<WASDCameraController>(camera).is_none(),
            "WASD control is dropped so free-fly input stops"
        );
    }

    /// SetCamera against a world with no scenario camera is a warn-and-continue
    /// no-op, not a panic (a headless rig without the loader's camera).
    #[test]
    fn set_camera_without_a_camera_is_harmless() {
        use nova_events::prelude::EventWorld;

        let mut world = World::new();
        world.init_resource::<NovaEventWorld>();
        world.init_resource::<GameObjectives>();
        let bystander = world.spawn(Transform::default()).id();

        let action = SetCameraActionConfig {
            position: Vec3::ONE,
            look_at: Vec3::ZERO,
        };
        let mut event_world = world.resource_mut::<NovaEventWorld>();
        action.action(&mut event_world, &GameEventInfo::default());
        NovaEventWorld::state_to_world_system(&mut world);

        assert!(world.get_entity(bystander).is_ok());
    }

    /// The Screenshot action queues a capture without panicking on a world with
    /// no render backend (the `save_to_disk` observer simply never fires): the
    /// drain must complete and a `Screenshot` request entity must exist. A bare
    /// filename has no parent dir, so the action writes nothing to disk here.
    #[test]
    fn screenshot_action_queues_a_capture_without_render() {
        use nova_events::prelude::EventWorld;

        let mut world = World::new();
        world.init_resource::<NovaEventWorld>();
        world.init_resource::<GameObjectives>();

        let action = ScreenshotActionConfig::new("nova_test_shot.png");
        let mut event_world = world.resource_mut::<NovaEventWorld>();
        action.action(&mut event_world, &GameEventInfo::default());
        NovaEventWorld::state_to_world_system(&mut world);

        let requests = world.query::<&Screenshot>().iter(&world).count();
        assert_eq!(requests, 1, "exactly one capture request is spawned");
    }

    /// `resolve_capture_path_in` joins relative paths under the capture dir,
    /// leaves absolute paths alone, and is a no-op without a dir. Tests the pure
    /// core so no process-wide env mutation is needed.
    #[test]
    fn resolve_capture_path_honors_the_capture_dir() {
        use std::path::Path;

        // A relative path is joined under the capture dir.
        assert_eq!(
            resolve_capture_path_in("feature-gravity.png", Some("/tmp/nova-shots")),
            Path::new("/tmp/nova-shots/feature-gravity.png")
        );
        // No capture dir: the relative path is used as-is.
        assert_eq!(
            resolve_capture_path_in("feature-gravity.png", None),
            Path::new("feature-gravity.png")
        );
        // An absolute path passes through even with a capture dir set.
        assert_eq!(
            resolve_capture_path_in("/shots/a.png", Some("/tmp/nova-shots")),
            Path::new("/shots/a.png")
        );
    }

    #[cfg(feature = "serde")]
    #[test]
    fn set_camera_config_round_trips_through_ron() {
        let config = SetCameraActionConfig {
            position: Vec3::new(1.0, 2.0, 3.0),
            look_at: Vec3::new(-1.0, 0.0, 5.0),
        };
        let ron = ron::to_string(&config).expect("serialize");
        let back: SetCameraActionConfig = ron::from_str(&ron).expect("deserialize");
        assert_eq!(back.position, config.position);
        assert_eq!(back.look_at, config.look_at);
    }

    #[cfg(feature = "serde")]
    #[test]
    fn screenshot_config_round_trips_through_ron() {
        let config = ScreenshotActionConfig::new("shots/feature-gravity.png");
        let ron = ron::to_string(&config).expect("serialize");
        let back: ScreenshotActionConfig = ron::from_str(&ron).expect("deserialize");
        assert_eq!(back.path, config.path);
    }

    /// A modder authors `SetSkybox` in RON as a bare cubemap path (the `AssetRef`
    /// shape), so the whole action must round-trip through serde. Confirms the new
    /// hook is reachable from a data file, not just from code.
    #[cfg(feature = "serde")]
    #[test]
    fn set_skybox_action_round_trips_through_ron() {
        let action =
            EventActionConfig::SetSkybox(SetSkyboxActionConfig::new("scenarios/nebula.cube.png"));
        let ron = ron::to_string(&action).expect("serialize");
        let back: EventActionConfig = ron::from_str(&ron).expect("deserialize");
        match back {
            EventActionConfig::SetSkybox(config) => {
                assert_eq!(config.cubemap.path(), Some("scenarios/nebula.cube.png"));
                assert_eq!(config.brightness, None);
            }
            other => panic!("expected SetSkybox, got {other:?}"),
        }
    }
}
