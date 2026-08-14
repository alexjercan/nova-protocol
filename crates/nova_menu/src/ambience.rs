//! The living backdrop behind the menu: pick a `menu_backdrop` scenario,
//! stage its camera into a fixed cinematic shot, and hide the HUD chrome.
//!
//! The menu names no scenario ids. The backdrop comes from the `menu_backdrop`
//! scenario flag (moddable), drawn at random on every menu entry.

use bevy::prelude::*;
use bevy_rand::prelude::*;
use nova_events::prelude::EntityId;
use nova_gameplay::prelude::*;
use nova_hud::prelude::HudVisibility;
use nova_scenario::prelude::*;
use nova_ship::prelude::*;
use rand::Rng as _;

/// EntityId of the planetoid whose well anchors the camera framing (the
/// orbit itself is the AI orbiter's business, nova_assets). Selected by id
/// (not "any well") so a second big rock in the backdrop cannot silently
/// retarget the camera.
pub(crate) const MENU_PLANETOID_ID: &str = "menu_planetoid";
/// Estimated orbit clearance above the well's GEOMETRIC body radius, used
/// only to frame the camera far enough out to keep the ring in shot. The
/// planetoid's noise mesh reaches several times past its nominal 20u and
/// the well's mu/SOI derive from that real radius at runtime (see
/// insert_asteroid_gravity_well), so the framing math starts from
/// body_radius, not the nominal size. The AI's actual ring radius is the
/// autopilot's own plan (stable band); this constant only shapes the shot.
pub(crate) const ORBIT_CLEARANCE: f32 = 40.0;

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

/// How many frames `stage_menu_camera` waits for the backdrop's
/// `menu_planetoid` well before giving up on cinematic framing and activating
/// the camera at the scenario's own pose. Long enough for a scenario's
/// OnStart spawns to settle; short enough that a well-less mod backdrop shows
/// within a second instead of leaving the menu on a blank camera forever.
pub(crate) const MENU_CAMERA_GRACE_FRAMES: u32 = 60;

/// Turn the loader's flyable camera into a fixed cinematic viewpoint: strip the
/// WASD controller (the user must not be able to fly the menu backdrop), then
/// hold the framing pose every frame. The pose is written only AFTER the
/// controller is gone: the controller drives Transform from its own state each
/// frame, so a pose written in the same frame the removal is queued gets
/// overwritten before the removal applies (observed: camera stuck at the
/// loader's default inside the planetoid). The camera spawns a frame after
/// LoadScenario, so an OnEnter hook would miss it - this polls instead.
///
/// A backdrop WITHOUT a `menu_planetoid` well (possible once mods can flag
/// backdrops) must not leave the camera deactivated forever - that would
/// render the menu unusable, since the UI draws through this camera. After
/// [`MENU_CAMERA_GRACE_FRAMES`] without the well, the camera activates at the
/// scenario's own pose: the mod author's framing, unstaged.
pub(crate) fn stage_menu_camera(
    mut commands: Commands,
    mut controlled: Query<(Entity, &mut Camera), (With<Camera3d>, With<WASDCameraController>)>,
    mut staged: Query<
        (&mut Transform, &mut Camera),
        (With<Camera3d>, Without<WASDCameraController>),
    >,
    wells: Query<(&Transform, &GravityWell, &EntityId), Without<Camera3d>>,
    mut frames_without_well: Local<u32>,
) {
    // Blank the frame while the controller is still attached: the loader
    // spawns the camera inside the planetoid's geometric radius, and staging
    // takes effect one frame later, so an active camera would flash the
    // inside of the rock on every menu entry.
    for (entity, mut camera) in &mut controlled {
        camera.is_active = false;
        commands.entity(entity).remove::<WASDCameraController>();
        // A fresh backdrop camera restarts the well grace period.
        *frames_without_well = 0;
    }
    // Frame the planetoid + orbit from ITS well's real geometry (the body
    // radius is only known at runtime; see ORBIT_CLEARANCE).
    let Some((well_transform, well, _)) = wells.iter().find(|(_, _, id)| id.0 == MENU_PLANETOID_ID)
    else {
        *frames_without_well += 1;
        if *frames_without_well == MENU_CAMERA_GRACE_FRAMES {
            warn!(
                "stage_menu_camera: the backdrop has no '{MENU_PLANETOID_ID}' gravity well; \
                 activating the camera at the scenario's own pose (no cinematic framing)"
            );
        }
        if *frames_without_well >= MENU_CAMERA_GRACE_FRAMES {
            for (_, mut camera) in &mut staged {
                camera.is_active = true;
            }
        }
        return;
    };
    *frames_without_well = 0;
    let r_orbit = well.body_radius + ORBIT_CLEARANCE;
    let pose = well_transform.translation + Vec3::new(0.0, r_orbit * 0.75, r_orbit * 2.5);
    for (mut transform, mut camera) in &mut staged {
        *transform =
            Transform::from_translation(pose).looking_at(well_transform.translation, Vec3::Y);
        camera.is_active = true;
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
pub(crate) fn unload_menu_ambience(mut commands: Commands) {
    commands.trigger(UnloadScenario);
}
