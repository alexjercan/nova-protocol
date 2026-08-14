//! The living backdrop behind the menu: pick a `menu_backdrop` scenario,
//! stage its camera into a fixed cinematic shot, and hide the HUD chrome.
//!
//! The menu names no scenario ids. The backdrop comes from the `menu_backdrop`
//! scenario flag (moddable), drawn at random on every menu entry.

use bevy::prelude::*;
use bevy_rand::prelude::*;
use nova_gameplay::prelude::*;
use nova_hud::prelude::HudVisibility;
use nova_scenario::prelude::*;
use nova_ship::prelude::*;
use rand::Rng as _;

/// Marks the menu's OWN interface camera.
#[derive(Component, Clone, Debug)]
pub(crate) struct MenuUiCameraMarker;

/// The menu interface's dedicated render target: a 2D overlay camera OWNED
/// BY THE MENU, not by the loaded scenario. The UI must never resolve its
/// layout against the scenario camera: the self-resetting backdrops reload
/// their scenario mid-menu, teardown despawns that camera, and a UI whose
/// target camera dies mid-frame lays out against a degenerate target
/// (observed live: `BorderRadius::resolve` panicking on a negative node
/// size, on every backdrop reset). `IsDefaultUiCamera` makes every root
/// `Node` adopt this camera without per-node targeting; the high order
/// composites the interface over whatever the scenario camera renders, and
/// `ClearColorConfig::None` keeps the backdrop visible beneath it.
pub(crate) fn spawn_menu_ui_camera(mut commands: Commands) {
    commands.spawn((
        DespawnOnExit(GameStates::MainMenu),
        Name::new("Menu UI Camera"),
        MenuUiCameraMarker,
        Camera2d,
        Camera {
            order: 100,
            // The overlay's OWN view clears to transparent every frame -
            // view textures are pooled, and an uncleared overlay inherits
            // whatever the pool last held (observed live: the boot loading
            // screen's final frame ghosting behind the menu).
            clear_color: ClearColorConfig::Custom(Color::NONE),
            // COMPOSITE over the scenario camera's image. The default
            // output mode has no blend state, and an unblended write
            // "ignores the existing data in the final render target" - the
            // overlay's mostly-empty view replaced the whole 3D frame
            // (observed live: the backdrop black).
            output_mode: bevy::camera::CameraOutputMode::Write {
                blend_state: Some(bevy::render::render_resource::BlendState::ALPHA_BLENDING),
                clear_color: ClearColorConfig::None,
            },
            ..default()
        },
        bevy::ui::IsDefaultUiCamera,
    ));
}

/// The living backdrop: load one of the `menu_backdrop`-flagged scenarios
/// behind the menu, picked at RANDOM so several ambience scenes (base or
/// mod-added) can rotate across menu entries. The loader brings its own
/// camera + skybox and tears down whatever was loaded before; the uniform
/// OnExit(MainMenu) teardown (unload_menu_ambience) tears this down again on
/// the way out, whatever the exit path.
///
/// NOTHING flagged is a warned degradation, not a panic (a mod set that
/// removes every backdrop must not brick the menu): a plain fixed camera
/// spawns instead so the UI still renders, over empty space.
pub(crate) fn load_menu_ambience(
    mut commands: Commands,
    scenarios: Res<GameScenarios>,
    issues: Option<Res<ContentIssues>>,
    mut rng: Single<&mut WyRand, With<GlobalRng>>,
) {
    // Deterministic candidate order before the draw (the registry is
    // HashMap-backed; iteration order must not leak into the pick). A
    // backdrop with Error-level content issues is filtered OUT of the draw:
    // the loader would refuse it (runtime content gate) and a refused menu
    // load means no camera at all - degrade to the other backdrops or the
    // bare-camera path instead.
    let mut backdrops: Vec<&ScenarioConfig> = scenarios
        .values()
        .filter(|s| s.menu_backdrop)
        .filter(|s| {
            let broken = issues
                .as_ref()
                .is_some_and(|issues| !issues.errors(&s.id).is_empty());
            if broken {
                warn!(
                    "load_menu_ambience: backdrop '{}' has content errors;                      skipping it in the draw",
                    s.id
                );
            }
            !broken
        })
        .collect();
    backdrops.sort_by(|a, b| a.id.cmp(&b.id));

    if backdrops.is_empty() {
        warn!(
            "load_menu_ambience: no registered scenario is flagged menu_backdrop; \
             the menu renders without a living backdrop"
        );
        commands.spawn((
            DespawnOnExit(GameStates::MainMenu),
            Name::new("Menu Fallback Camera"),
            Camera3d::default(),
            Transform::IDENTITY,
        ));
        return;
    }

    // Dev/capture override: NOVA_MENU_BACKDROP pins the pick to one id, so a
    // screenshot run (or a scene being authored) can look at a SPECIFIC
    // backdrop instead of re-rolling the menu until the draw cooperates. An
    // unknown id warns and falls back to the draw - a stale script must not
    // brick the menu.
    let forced = std::env::var("NOVA_MENU_BACKDROP")
        .ok()
        .filter(|id| !id.is_empty());
    let pick = match &forced {
        Some(id) => backdrops
            .iter()
            .find(|s| s.id == *id)
            .copied()
            .unwrap_or_else(|| {
                warn!(
                    "load_menu_ambience: NOVA_MENU_BACKDROP='{id}' matches no clean \
                     menu_backdrop scenario; drawing at random instead"
                );
                backdrops[rng.next_u32() as usize % backdrops.len()]
            }),
        None => backdrops[rng.next_u32() as usize % backdrops.len()],
    }
    .clone();
    commands.trigger(LoadScenario(pick));
}

/// The last scripted pose the backdrop staged, held across a MID-MENU
/// backdrop reload (the self-resetting backdrops fire `NextScenario` at
/// their own id). A reload tears down the scenario camera - and the pose
/// pinned on it - and the fresh camera's own `SetCamera` only lands a frame
/// later; the memory bridges that gap so the backdrop view never blinks to
/// the loader's default pose. Cleared on menu exit so the next entry keeps
/// the classic blank-then-stage sequence (the fresh backdrop's pose is not
/// known yet).
#[derive(Resource, Clone, Debug, Default)]
pub(crate) struct MenuCameraMemory(pub(crate) Option<Transform>);

/// Chaperone the backdrop's scenario camera: every backdrop POSES ITS OWN
/// CAMERA with a `SetCamera` action in its OnStart (the authored contract -
/// lint makes a poseless backdrop a content Error, and erroring backdrops
/// never enter the draw), so the menu's only jobs are hygiene:
///
/// - strip the loader's WASD controller the frame the camera appears (the
///   user must never fly the menu backdrop; a well-behaved backdrop's
///   SetCamera strips it too, but not until its OnStart drains);
/// - keep the camera BLANK until the backdrop's scripted pose lands, so
///   menu entry never flashes the loader's default pose from inside the
///   scene;
/// - across a MID-MENU reload, hold the remembered pose ACTIVE through the
///   one-frame gap before the fresh scenario's SetCamera lands (see
///   [`MenuCameraMemory`]). The interface itself is safe either way - it
///   renders through the menu's own UI camera, never through this one.
pub(crate) fn stage_menu_camera(
    mut commands: Commands,
    mut controlled: Query<(Entity, &mut Camera), (With<Camera3d>, With<WASDCameraController>)>,
    mut staged: Query<
        (&mut Transform, &mut Camera, Option<&ScriptedCameraPose>),
        (With<Camera3d>, Without<WASDCameraController>),
    >,
    mut memory: ResMut<MenuCameraMemory>,
) {
    // The Transform is NOT written while the controller is attached - the
    // controller drives it from its own state every frame, so a pose written
    // in the same frame the removal is queued gets overwritten (dev bug 1).
    for (entity, mut camera) in &mut controlled {
        camera.is_active = memory.0.is_some();
        commands.entity(entity).remove::<WASDCameraController>();
    }
    for (mut transform, mut camera, scripted) in &mut staged {
        if let Some(pose) = scripted {
            // The backdrop owns the pose (the loader's PostUpdate authority
            // override enforces it); remember it for the next reload gap.
            memory.0 =
                Some(Transform::from_translation(pose.position).looking_at(pose.look_at, Vec3::Y));
            camera.is_active = true;
        } else if let Some(pose) = memory.0 {
            // Reload gap: the fresh camera exists but the reloading
            // backdrop's SetCamera has not landed yet.
            *transform = pose;
            camera.is_active = true;
        }
        // No scripted pose and no memory: a first entry, one frame before
        // the backdrop's OnStart drains - stay blank.
    }
}

/// The menu is a cinematic shot: drive the HUD level to `Cinematic` while it is up.
/// `HudVisibility` owns the status bar and every tagged HUD widget. Restoring to `On`
/// on exit intentionally resets any mid-game cycle the player had going - simple beats
/// sticky.
pub(crate) fn hide_hud_chrome(mut level: ResMut<HudVisibility>) {
    *level = HudVisibility::Cinematic;
}

pub(crate) fn restore_hud_chrome(mut level: ResMut<HudVisibility>) {
    *level = HudVisibility::On;
}

/// Tear the backdrop down whenever the menu is left, no matter through which
/// button or future path. The editor does not unload scenarios on entry, and
/// a forgotten unload would leave the ambience simulating behind the game.
pub(crate) fn unload_menu_ambience(
    mut commands: Commands,
    mut camera_memory: ResMut<MenuCameraMemory>,
) {
    // Forget the staged pose: the NEXT menu entry draws a fresh backdrop
    // whose framing is not known yet, and entry must keep its classic
    // blank-then-stage sequence instead of one frame of the old scene's pose.
    camera_memory.0 = None;
    commands.trigger(UnloadScenario);
}
