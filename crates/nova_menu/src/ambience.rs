//! The living backdrop behind the menu: pick a `role: Backdrop` scenario,
//! stage its camera into a fixed cinematic shot, and hide the HUD chrome.
//!
//! The menu names no scenario ids. The backdrops come from the `role: Backdrop`
//! scenario flag (moddable), and the menu owns their rotation: a backdrop
//! reports its act finished with `BackdropDone`, and the menu draws the next
//! one from a shuffled cycle.

use bevy::prelude::*;
use bevy_rand::prelude::*;
use nova_gameplay::prelude::*;
use nova_hud::prelude::HudVisibility;
use nova_scenario::prelude::*;
use nova_ship::prelude::*;
use rand::seq::SliceRandom as _;

/// Environment variable that pins the first backdrop of each menu entry to
/// one scenario id, so a capture run (or a backdrop being authored) opens on a
/// SPECIFIC scene instead of the rotation's draw. The rotation continues from
/// there. An unknown id warns and falls back to the draw.
pub const MENU_BACKDROP_ENV: &str = "NOVA_MENU_BACKDROP";

/// Marks the menu's OWN interface camera.
#[derive(Component, Clone, Debug)]
pub(crate) struct MenuUiCameraMarker;

/// The menu UI camera's render layer: deliberately EMPTY of world entities
/// (nova_interface owns 20-22), so the overlay's 2D world pass draws nothing.
/// Without it the overlay re-rendered every world-space effect the 2D
/// pipeline knows (the hanabi particle bursts) a second time - untonemapped,
/// alpha-blended over the finished frame (observed live: blast VFX ghosting
/// across the menu). UI is unaffected: bevy_ui routes root nodes by TARGET
/// CAMERA (`IsDefaultUiCamera`), not by render layers.
const MENU_UI_LAYER: usize = 23;

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
        bevy::camera::visibility::RenderLayers::layer(MENU_UI_LAYER),
    ));
}

/// How long the menu holds a finished backdrop before it cuts to the next
/// one, on virtual (pause-frozen) time: the beat that lets a finished act's
/// aftermath stay in shot.
pub(crate) const BACKDROP_CUT_SECS: f32 = 1.0;

/// The menu's backdrop rotation: which backdrop plays, in which order, and
/// the cut a finished backdrop has asked for.
///
/// A SHUFFLED CYCLE: every eligible backdrop plays once before any plays
/// again, and a fresh cycle never opens with the backdrop on screen when two
/// or more are eligible. The cycle outlives a menu visit, so re-entering the
/// menu continues it; the pending cut does not (see [`unload_menu_ambience`]).
#[derive(Resource, Debug, Default)]
pub(crate) struct MenuBackdropRotation {
    /// The rest of this cycle; the next draw is the LAST element.
    bag: Vec<ScenarioId>,
    /// The backdrop on screen, so a reshuffled cycle cannot open with it.
    current: Option<ScenarioId>,
    /// The cut a finished backdrop asked for, ticking on virtual time.
    pending_cut: Option<Timer>,
}

impl MenuBackdropRotation {
    /// Draw the next backdrop from `eligible`, the sorted ids that may play.
    /// `None` only when nothing is eligible.
    ///
    /// A bagged id that is no longer eligible (its mod disabled, its content
    /// now erroring) is dropped rather than drawn.
    pub(crate) fn draw<R: rand::Rng>(
        &mut self,
        eligible: &[ScenarioId],
        rng: &mut R,
    ) -> Option<ScenarioId> {
        self.bag.retain(|id| eligible.contains(id));
        if self.bag.is_empty() {
            self.bag = eligible.to_vec();
            self.bag.shuffle(rng);
            // The cycle boundary: the first draw of the new cycle must not
            // repeat the backdrop that just played.
            if self.bag.len() >= 2 && self.bag.last() == self.current.as_ref() {
                let last = self.bag.len() - 1;
                self.bag.swap(0, last);
            }
        }
        self.current = self.bag.pop();
        self.current.clone()
    }

    /// Put `id` on screen without a draw (the [`MENU_BACKDROP_ENV`] pin). It
    /// counts as played in the current cycle.
    pub(crate) fn pin(&mut self, id: &str) {
        self.bag.retain(|bagged| bagged != id);
        self.current = Some(id.to_string());
    }
}

/// The backdrops the menu may play, sorted by id.
///
/// Sorted because the registry is HashMap-backed, and its iteration order must
/// not leak into a seeded draw. A backdrop with Error-level content issues is
/// left out: the loader would refuse it, and a refused menu load means no
/// camera at all.
fn eligible_backdrops(
    scenarios: &GameScenarios,
    issues: Option<&ContentIssues>,
) -> Vec<ScenarioId> {
    let mut backdrops: Vec<ScenarioId> = scenarios
        .values()
        .filter(|s| s.role.is_backdrop())
        .filter(|s| {
            let broken = issues.is_some_and(|issues| !issues.errors(&s.id).is_empty());
            if broken {
                warn!(
                    "menu backdrop '{}' has content errors; skipping it in the draw",
                    s.id
                );
            }
            !broken
        })
        .map(|s| s.id.clone())
        .collect();
    backdrops.sort();
    backdrops
}

/// Load the drawn backdrop, or, with nothing to draw, a plain fixed camera so
/// the UI still renders over empty space. Nothing eligible is a warned
/// degradation, not a panic: a mod set that removes every backdrop must not
/// brick the menu.
fn start_backdrop(commands: &mut Commands, scenarios: &GameScenarios, pick: Option<ScenarioId>) {
    match pick.and_then(|id| scenarios.get(&id)) {
        Some(config) => commands.trigger(LoadScenario(config.clone())),
        None => {
            warn!(
                "no registered scenario is a clean `role: Backdrop`; the menu renders without \
                 a living backdrop"
            );
            commands.spawn((
                DespawnOnExit(GameStates::MainMenu),
                Name::new("Menu Fallback Camera"),
                Camera3d::default(),
                Transform::IDENTITY,
            ));
        }
    }
}

/// The living backdrop: on menu entry, load the next `role: Backdrop`
/// scenario from the menu's [`MenuBackdropRotation`]. The loader brings its
/// own camera + skybox and tears down whatever was loaded before; the uniform
/// OnExit(MainMenu) teardown (unload_menu_ambience) tears this down again on
/// the way out, whatever the exit path.
///
/// The unload is UNCONDITIONAL and comes first, before the draw and before
/// either camera. Menu entry ends whatever was running, and the branch that
/// happens to load a backdrop must not be the only one that says so: a mod set
/// with no clean backdrop took the early return, and whatever the player left
/// went on simulating behind the front door.
pub(crate) fn load_menu_ambience(
    mut commands: Commands,
    scenarios: Res<GameScenarios>,
    issues: Option<Res<ContentIssues>>,
    mut rotation: ResMut<MenuBackdropRotation>,
    mut rng: Single<&mut WyRand, With<GlobalRng>>,
) {
    commands.trigger(UnloadScenario);

    let eligible = eligible_backdrops(&scenarios, issues.as_deref());
    // Dev/capture override: NOVA_MENU_BACKDROP pins the FIRST backdrop of
    // each menu entry to one id, so a screenshot run (or a scene being
    // authored) opens on a SPECIFIC backdrop; the rotation draws every one
    // after it. An unknown id warns and falls back to the draw - a stale
    // script must not brick the menu.
    let pinned = std::env::var(MENU_BACKDROP_ENV)
        .ok()
        .filter(|id| !id.is_empty())
        .filter(|id| {
            let known = eligible.contains(id);
            if !known && !eligible.is_empty() {
                warn!(
                    "load_menu_ambience: NOVA_MENU_BACKDROP='{id}' matches no clean \
                     menu backdrop scenario; drawing from the rotation instead"
                );
            }
            known
        });
    let pick = match pinned {
        Some(id) => {
            rotation.pin(&id);
            Some(id)
        }
        None => rotation.draw(&eligible, &mut *rng),
    };
    start_backdrop(&mut commands, &scenarios, pick);
}

/// Cut to the next backdrop once the one on screen reports `BackdropDone`.
///
/// The backdrop's flag lands in `PostUpdate` (the event drain), so this
/// `Update` system sees it the next frame. It arms one cut of
/// [`BACKDROP_CUT_SECS`] on virtual time: a pause holds it, and a repeated
/// `BackdropDone` neither re-arms nor restarts it. At expiry it draws the
/// successor and loads it; that load's teardown clears the flag at this
/// system's command flush, before the next frame, so one report makes one
/// cut. A cut whose backdrop was torn down before expiry (the flag gone) is
/// dropped: it would cut a scene that never asked.
pub(crate) fn advance_menu_backdrop(
    mut commands: Commands,
    time: Res<Time<Virtual>>,
    event_world: Res<NovaEventWorld>,
    scenarios: Res<GameScenarios>,
    issues: Option<Res<ContentIssues>>,
    mut rotation: ResMut<MenuBackdropRotation>,
    mut camera_memory: ResMut<MenuCameraMemory>,
    mut rng: Single<&mut WyRand, With<GlobalRng>>,
) {
    if !event_world.backdrop_done() {
        rotation.pending_cut = None;
        return;
    }
    let cut = rotation
        .pending_cut
        .get_or_insert_with(|| Timer::from_seconds(BACKDROP_CUT_SECS, TimerMode::Once));
    if !cut.tick(time.delta()).is_finished() {
        return;
    }
    rotation.pending_cut = None;

    let eligible = eligible_backdrops(&scenarios, issues.as_deref());
    let pick = rotation.draw(&eligible, &mut *rng);
    if pick.is_none() {
        // The finished backdrop ends with nothing to follow it. The loader
        // only tears down on a load, so end it here; and forget its pose,
        // which belongs to no camera the fallback path spawns.
        commands.trigger(UnloadScenario);
        camera_memory.0 = None;
    }
    start_backdrop(&mut commands, &scenarios, pick);
}

/// The last scripted pose the backdrop staged, held across a MID-MENU
/// backdrop cut (see [`advance_menu_backdrop`]). A cut tears down the
/// scenario camera - and the pose pinned on it - and the fresh camera's own
/// `SetCamera` only lands a frame later; the memory bridges that gap so the
/// backdrop view never blinks to the loader's default pose. Cleared on menu
/// exit so the next entry keeps the classic blank-then-stage sequence (the
/// fresh backdrop's pose is not known yet).
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
/// - across a MID-MENU cut, hold the remembered pose ACTIVE through the
///   one-frame gap before the fresh scenario's SetCamera lands (see
///   [`MenuCameraMemory`]). The interface itself is safe either way - it
///   renders through the menu's own UI camera, never through this one.
pub(crate) fn stage_menu_camera(
    mut commands: Commands,
    mut controlled: Query<(Entity, &mut Camera), (With<Camera3d>, With<WASDCameraController>)>,
    mut staged: Query<
        (
            &mut Transform,
            &mut Camera,
            Option<&ScriptedCameraTransform>,
        ),
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
            // override enforces it); remember it for the next reload gap. The
            // scripted meters crossed to a transform where the loader derived
            // them, so this reads that one number rather than crossing again.
            memory.0 = Some(**pose);
            camera.is_active = true;
        } else if let Some(pose) = memory.0 {
            // Cut gap: the fresh camera exists but the next backdrop's
            // SetCamera has not landed yet.
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
    mut rotation: ResMut<MenuBackdropRotation>,
) {
    // Forget the staged pose: the NEXT menu entry draws a fresh backdrop
    // whose framing is not known yet, and entry must keep its classic
    // blank-then-stage sequence instead of one frame of the old scene's pose.
    camera_memory.0 = None;
    // A cut is a menu-only event: it must never load a backdrop into the
    // game the player is leaving for.
    rotation.pending_cut = None;
    commands.trigger(UnloadScenario);
}
