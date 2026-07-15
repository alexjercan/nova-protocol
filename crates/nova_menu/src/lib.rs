//! The main menu: the game's front door.
//!
//! `NovaMenuPlugin` owns [`GameStates::MainMenu`]: a small panel anchored to the
//! bottom-right of the screen with the game title and New Game / Sandbox /
//! Settings / Exit buttons, drawn over a live ambient scene - the
//! `menu_ambience` scenario (nova_assets), where an AI ship flies a real
//! thruster-driven orbit around a planetoid's gravity well (its
//! AIControllerConfig orbit directive engages the ORBIT autopilot; task
//! 20260711-212504), watched by a
//! fixed cinematic camera with the status bar hidden. The buttons write
//! [`GameMode`] and hand off to [`GameStates::Playing`]; the editor
//! (`nova_editor`) only comes up in `Sandbox` mode, and the menu's own
//! `OnEnter(Playing)` system loads the New Game scenario in `NewGame` mode.
//!
//! `nova_core`'s `AppBuilder` adds this plugin (and routes `Loading -> MainMenu`
//! instead of `Loading -> Playing`) only for the default editor app; examples
//! that supply their own game plugins never see the menu.
//!
//! Design rationale: docs/spikes/20260711-180500-main-menu.md.

use avian3d::prelude::{Physics, PhysicsTime};
use bevy::{
    picking::hover::Hovered,
    prelude::*,
    ui::Pressed,
    ui_widgets::{observe, Activate, Button},
    window::{CursorGrabMode, CursorOptions, PrimaryWindow},
};
use nova_assets::prelude::{EnabledMods, ModCatalog, ModEntry};
use nova_events::prelude::EntityId;
use nova_gameplay::prelude::*;
use nova_scenario::prelude::*;

pub mod prelude {
    pub use super::NovaMenuPlugin;
}

/// The scenario New Game drops the player into: the Shakedown Run starter
/// tutorial (task 20260711-180506, spike 20260712-092926). Registered by
/// `nova_assets` with its own canned player ship, so the menu needs no
/// content of its own.
const NEW_GAME_SCENARIO_ID: &str = "shakedown_run";

/// The backdrop scenario (nova_assets registers it; task 20260711-180455).
const MENU_AMBIENCE_SCENARIO_ID: &str = "menu_ambience";
/// EntityId of the planetoid whose well anchors the camera framing (the
/// orbit itself is the AI orbiter's business, nova_assets). Selected by id
/// (not "any well") so a second big rock in the backdrop cannot silently
/// retarget the camera.
const MENU_PLANETOID_ID: &str = "menu_planetoid";
/// Estimated orbit clearance above the well's GEOMETRIC body radius, used
/// only to frame the camera far enough out to keep the ring in shot. The
/// planetoid's noise mesh reaches several times past its nominal 20u and
/// the well's mu/SOI derive from that real radius at runtime (see
/// insert_asteroid_gravity_well), so the framing math starts from
/// body_radius, not the nominal size. The AI's actual ring radius is the
/// autopilot's own plan (stable band); this constant only shapes the shot.
const ORBIT_CLEARANCE: f32 = 40.0;

// The whole game UI shares one theme (task 20260714-212139): the palette + metrics
// live in `nova_ui::theme`. The menu keeps its own tiny polling colour system
// (`update_button_colors`) rather than nova_ui's observers, to stay self-contained,
// but draws every colour + metric from the shared theme.
use nova_ui::theme;

pub struct NovaMenuPlugin;

impl Plugin for NovaMenuPlugin {
    fn build(&self, app: &mut App) {
        // Owned by NovaHudPlugin in the assembled app; initialized here too so
        // the menu plugin stands alone (tests, future slim apps).
        app.init_resource::<HudVisibility>();
        app.add_systems(
            OnEnter(GameStates::MainMenu),
            (load_menu_ambience, setup_menu_ui, hide_hud_chrome),
        );
        // Uniform backdrop teardown: EVERY exit from the menu unloads the
        // ambience scenario, so future exits (e.g. the pause menu's Back path,
        // task 20260711-185156) cannot leak a simulating backdrop. OnExit runs
        // before OnEnter(Playing), so New Game's LoadScenario still lands after
        // this unload.
        app.add_systems(
            OnExit(GameStates::MainMenu),
            (restore_hud_chrome, unload_menu_ambience),
        );
        app.add_systems(
            Update,
            (stage_menu_camera, update_mod_toggle_labels).run_if(in_state(GameStates::MainMenu)),
        );
        // Wheel-scroll for the mods list: gated on the input message buffer existing
        // (the real app's InputPlugin provides it; minimal headless test apps do not).
        app.add_systems(
            Update,
            scroll_mods_panel
                .run_if(in_state(GameStates::MainMenu))
                .run_if(resource_exists::<Messages<bevy::input::mouse::MouseWheel>>),
        );
        // Button hover/press feedback serves both the main menu panel and the
        // pause overlay; the query only matches MenuButton, so running it
        // unconditionally is a no-op elsewhere.
        app.add_systems(Update, update_button_colors);
        // Menu-button press cue (task 20260714-090006): one global observer
        // clicks for every MenuButton activation, so New Game / Sandbox /
        // Settings / Exit and the pause/mods buttons all sound the same crisp
        // click without touching each handler.
        app.add_observer(on_menu_button_activate);
        app.add_systems(
            OnEnter(GameStates::Playing),
            start_new_game_scenario.run_if(resource_equals(GameMode::NewGame)),
        );

        // The pause overlay (task 20260711-185156): ESC toggles it anywhere
        // in Playing; entering Paused freezes both clocks and frees the
        // cursor, leaving restores them. Update systems keep running while
        // paused (Time<Virtual> pause zeroes deltas, it does not stop
        // schedules), which is exactly what lets the overlay stay
        // interactive.
        app.add_systems(Update, toggle_pause.run_if(in_state(GameStates::Playing)));
        app.add_systems(
            OnEnter(PauseStates::Paused),
            (pause_clocks, release_cursor, setup_pause_ui),
        );
        app.add_systems(
            OnExit(PauseStates::Paused),
            (unpause_clocks, restore_cursor),
        );
        // Leaving Playing while paused (Back to Main Menu) must not leave the
        // game frozen for the next session of play.
        app.add_systems(OnExit(GameStates::Playing), force_unpause);
    }
}

/// Play the menu-button click for any [`MenuButton`] activation (task
/// 20260714-090006). One global observer covers every menu and pause-overlay
/// button - the `button()` helper always carries `MenuButton` - so presses that
/// were visual-only (hover/press colours) now also have a voice. A missing
/// [`SoundBank`] (assets not loaded) is a graceful no-op.
fn on_menu_button_activate(
    activate: On<Activate>,
    q_button: Query<(), With<MenuButton>>,
    bank: Option<Res<SoundBank<NovaSfx>>>,
    mut commands: Commands,
) {
    if !q_button.contains(activate.entity) {
        return;
    }
    if let Some(bank) = bank {
        commands.play_sfx_volume(bank.get(NovaSfx::MenuSelect), MENU_SELECT_VOLUME);
    }
}

/// ESC (or the gamepad Start button) toggles the pause overlay. Plain
/// press-to-toggle; no existing Escape binding anywhere in the repo
/// (checked 2026-07-11).
fn toggle_pause(
    keys: Res<ButtonInput<KeyCode>>,
    gamepad: Option<Res<ButtonInput<GamepadButton>>>,
    current: Res<State<PauseStates>>,
    mut next: ResMut<NextState<PauseStates>>,
    bank: Option<Res<SoundBank<NovaSfx>>>,
    mut commands: Commands,
) {
    let pad = gamepad
        .map(|g| g.just_pressed(GamepadButton::Start))
        .unwrap_or(false);
    if keys.just_pressed(KeyCode::Escape) || pad {
        next.set(match current.get() {
            PauseStates::Unpaused => PauseStates::Paused,
            PauseStates::Paused => PauseStates::Unpaused,
        });
        // The overlay open/close toggle (task 20260714-090006): a soft UI blip
        // on both directions. The Resume/Exit buttons close it with their own
        // MenuSelect click, so only the ESC/pad toggle needs this.
        if let Some(bank) = bank {
            commands.play_sfx_volume(bank.get(NovaSfx::UiToggle), UI_TOGGLE_VOLUME);
        }
    }
}

/// Freeze the simulation: virtual time (Update deltas + FixedUpdate
/// accumulation, which physics follows) and avian's own physics clock, so
/// nothing integrates regardless of which clock a system reads.
fn pause_clocks(mut virtual_time: ResMut<Time<Virtual>>, mut physics_time: ResMut<Time<Physics>>) {
    virtual_time.pause();
    physics_time.pause();
}

/// Unconditional: the pause menu is currently the only clock-pauser in the
/// app. A future cutscene/debug freeze that also pauses these clocks will be
/// stomped here and needs a coordination story first (review R1.6).
fn unpause_clocks(
    mut virtual_time: ResMut<Time<Virtual>>,
    mut physics_time: ResMut<Time<Physics>>,
) {
    virtual_time.unpause();
    physics_time.unpause();
}

/// The scenario locks and hides the cursor (nova_editor's grab systems); the
/// overlay needs it back to be clickable.
fn release_cursor(mut cursor: Single<&mut CursorOptions, With<PrimaryWindow>>) {
    cursor.grab_mode = CursorGrabMode::None;
    cursor.visible = true;
}

/// Re-grab on resume, but only during scenario play: a live player ship is
/// what distinguishes it (PlayerSpaceshipMarker is only inserted by the
/// scenario spawn path; the editor's build-mode preview never carries it).
/// Mirrors the cfg carve-out of setup_grab_cursor_scenario: debug builds
/// never grab.
fn restore_cursor(
    mut cursor: Single<&mut CursorOptions, With<PrimaryWindow>>,
    q_player: Query<(), With<PlayerSpaceshipMarker>>,
    game_state: Res<State<GameStates>>,
) {
    // The Back path exits Paused and Playing in the same transition batch
    // (GameStates applies first, it is init'd first): never re-grab when the
    // destination is the menu (review R1.4).
    if *game_state.get() != GameStates::Playing {
        return;
    }
    if cfg!(not(feature = "debug")) && !q_player.is_empty() {
        cursor.grab_mode = CursorGrabMode::Locked;
        cursor.visible = false;
    }
}

/// Safety net for the Back to Main Menu path (and any future exit from
/// Playing while paused): reset the pause state and clocks.
fn force_unpause(
    mut next: ResMut<NextState<PauseStates>>,
    mut virtual_time: ResMut<Time<Virtual>>,
    mut physics_time: ResMut<Time<Physics>>,
) {
    next.set(PauseStates::Unpaused);
    virtual_time.unpause();
    physics_time.unpause();
}

/// The pause overlay: a dim full-screen layer with a centered panel.
fn setup_pause_ui(mut commands: Commands) {
    commands
        .spawn((
            DespawnOnExit(PauseStates::Paused),
            Name::new("Pause Overlay"),
            // A modal blocker, unlike the main menu root: the editor's
            // buttons and section-picking live beneath this overlay and must
            // not receive clicks through it (review R1.2).
            Pickable {
                should_block_lower: true,
                is_hoverable: false,
            },
            Node {
                width: percent(100),
                height: percent(100),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.6)),
            // Above the HUD chrome.
            GlobalZIndex(10),
        ))
        .with_children(|parent| {
            parent
                .spawn((
                    Name::new("Pause Panel"),
                    Node {
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        width: px(280),
                        padding: UiRect::all(px(20)),
                        border: UiRect::all(px(theme::BORDER_W)),
                        border_radius: BorderRadius::all(px(theme::RADIUS)),
                        ..default()
                    },
                    BorderColor::all(theme::BORDER),
                    BackgroundColor(theme::PANEL),
                ))
                .with_children(|parent| {
                    parent.spawn((
                        Name::new("Pause Title"),
                        Text::new("Paused"),
                        TextFont {
                            font_size: FontSize::Px(24.0),
                            ..default()
                        },
                        TextColor(theme::TEXT),
                    ));
                    parent.spawn((
                        Name::new("Resume Button"),
                        button("Resume"),
                        observe(on_resume),
                    ));
                    parent.spawn((
                        Name::new("Back To Menu Button"),
                        button("Back to Main Menu"),
                        observe(on_back_to_menu),
                    ));
                    // No process to quit on wasm; the browser tab owns the
                    // lifecycle (same rule as the main menu's Exit).
                    #[cfg(not(target_arch = "wasm32"))]
                    parent.spawn((
                        Name::new("Pause Exit Button"),
                        button("Exit"),
                        observe(on_exit),
                    ));
                });
        });
}

fn on_resume(_activate: On<Activate>, mut next: ResMut<NextState<PauseStates>>) {
    next.set(PauseStates::Unpaused);
}

/// Back out to the front door. Unpauses in the same transition batch (a
/// force_unpause on OnExit(Playing) alone would apply one frame late,
/// leaving the overlay over the menu for a frame - review R1.4); entering
/// MainMenu loads the ambience backdrop (tearing the gameplay scenario down)
/// and the editor resets its own inner state on OnExit(Playing).
fn on_back_to_menu(
    _activate: On<Activate>,
    mut state: ResMut<NextState<GameStates>>,
    mut pause: ResMut<NextState<PauseStates>>,
) {
    state.set(GameStates::MainMenu);
    pause.set(PauseStates::Unpaused);
}

/// Marker for the menu's buttons, so the color feedback system only touches ours.
#[derive(Component)]
struct MenuButton;

/// Marker for the Settings placeholder panel, toggled by the Settings button.
#[derive(Component)]
struct SettingsPanel;

/// Marker for the Mods panel root, toggled by the Mods button.
#[derive(Component)]
struct ModsPanel;

/// The scrollable container holding the mod rows (wheel-scrolled by
/// `scroll_mods_panel`, so a long installed-mods list stays reachable).
#[derive(Component)]
struct ModsScrollPanel;

/// A mod row's enable/disable toggle button: carries the catalog `id` it toggles
/// and whether it is the locked `base` entry. `on_mod_toggle` reads this on click;
/// `update_mod_toggle_labels` renders the button's state from `EnabledMods`.
#[derive(Component)]
struct ModToggle {
    id: String,
    base: bool,
}

/// The living backdrop: load the ambient scenario behind the menu. The loader
/// brings its own camera + skybox and tears down whatever was loaded before;
/// the uniform OnExit(MainMenu) teardown (unload_menu_ambience) tears this
/// down again on the way out, whatever the exit path.
fn load_menu_ambience(mut commands: Commands, scenarios: Res<GameScenarios>) {
    let scenario = scenarios
        .get(MENU_AMBIENCE_SCENARIO_ID)
        .unwrap_or_else(|| panic!("Scenario '{MENU_AMBIENCE_SCENARIO_ID}' not found"))
        .clone();
    commands.trigger(LoadScenario(scenario));
}

/// Turn the loader's flyable camera into a fixed cinematic viewpoint: strip the
/// WASD controller (the user must not be able to fly the menu backdrop), then
/// hold the framing pose every frame. The pose is written only AFTER the
/// controller is gone: the controller drives Transform from its own state each
/// frame, so a pose written in the same frame the removal is queued gets
/// overwritten before the removal applies (observed: camera stuck at the
/// loader's default inside the planetoid). The camera spawns a frame after
/// LoadScenario, so an OnEnter hook would miss it - this polls instead.
fn stage_menu_camera(
    mut commands: Commands,
    mut controlled: Query<(Entity, &mut Camera), (With<Camera3d>, With<WASDCameraController>)>,
    mut staged: Query<
        (&mut Transform, &mut Camera),
        (With<Camera3d>, Without<WASDCameraController>),
    >,
    wells: Query<(&Transform, &GravityWell, &EntityId), Without<Camera3d>>,
) {
    // Blank the frame while the controller is still attached: the loader
    // spawns the camera inside the planetoid's geometric radius, and staging
    // takes effect one frame later, so an active camera would flash the
    // inside of the rock on every menu entry.
    for (entity, mut camera) in &mut controlled {
        camera.is_active = false;
        commands.entity(entity).remove::<WASDCameraController>();
    }
    // Frame the planetoid + orbit from ITS well's real geometry (the body
    // radius is only known at runtime; see ORBIT_CLEARANCE).
    let Some((well_transform, well, _)) = wells.iter().find(|(_, _, id)| id.0 == MENU_PLANETOID_ID)
    else {
        return;
    };
    let r_orbit = well.body_radius + ORBIT_CLEARANCE;
    let pose = well_transform.translation + Vec3::new(0.0, r_orbit * 0.75, r_orbit * 2.5);
    for (mut transform, mut camera) in &mut staged {
        *transform =
            Transform::from_translation(pose).looking_at(well_transform.translation, Vec3::Y);
        camera.is_active = true;
    }
}

/// The menu is a cinematic shot: drive the HUD level to None while it is up
/// (this is the mechanism from task 20260711-180501; it owns the status bar
/// and every tagged HUD widget). Restoring to All on exit intentionally
/// resets any mid-game cycle the player had going - simple beats sticky.
fn hide_hud_chrome(mut level: ResMut<HudVisibility>) {
    *level = HudVisibility::None;
}

fn restore_hud_chrome(mut level: ResMut<HudVisibility>) {
    *level = HudVisibility::All;
}

/// The menu panel: title on top, buttons below, anchored bottom-right per the
/// spike's layout call (the center of the screen stays free for the background
/// scene).
fn setup_menu_ui(
    mut commands: Commands,
    mod_catalog: Option<Res<ModCatalog>>,
    enabled: Option<Res<EnabledMods>>,
) {
    commands
        .spawn((
            DespawnOnExit(GameStates::MainMenu),
            Name::new("Menu Panel"),
            Node {
                position_type: PositionType::Absolute,
                right: px(40),
                bottom: px(40),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::FlexStart,
                width: px(280),
                padding: UiRect::all(px(20)),
                border: UiRect::all(px(theme::BORDER_W)),
                border_radius: BorderRadius::all(px(theme::RADIUS)),
                ..default()
            },
            BorderColor::all(theme::BORDER),
            BackgroundColor(theme::PANEL),
        ))
        .with_children(|parent| {
            parent.spawn((
                Name::new("Title"),
                Text::new("Nova Protocol"),
                TextFont {
                    font_size: FontSize::Px(28.0),
                    ..default()
                },
                TextColor(theme::TEXT),
            ));
            parent.spawn((
                Name::new("Title Separator"),
                Node {
                    width: percent(80),
                    height: px(2),
                    margin: UiRect::all(px(10)),
                    ..default()
                },
                BackgroundColor(theme::BORDER),
            ));
            parent.spawn((
                Name::new("New Game Button"),
                button("New Game"),
                observe(on_new_game),
            ));
            parent.spawn((
                Name::new("Sandbox Button"),
                button("Sandbox"),
                observe(on_sandbox),
            ));
            parent.spawn((Name::new("Mods Button"), button("Mods"), observe(on_mods)));
            parent.spawn((
                Name::new("Settings Button"),
                button("Settings"),
                observe(on_settings),
            ));
            // No process to quit on wasm; the browser tab owns the lifecycle.
            #[cfg(not(target_arch = "wasm32"))]
            parent.spawn((Name::new("Exit Button"), button("Exit"), observe(on_exit)));
        });

    // Settings placeholder: hidden until the Settings button toggles it. Real
    // content is task 20260711-180511 (v0.6.0).
    commands
        .spawn((
            DespawnOnExit(GameStates::MainMenu),
            Name::new("Settings Panel Root"),
            SettingsPanel,
            Visibility::Hidden,
            Pickable {
                should_block_lower: false,
                is_hoverable: false,
            },
            Node {
                width: percent(100),
                height: percent(100),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
        ))
        .with_children(|parent| {
            parent
                .spawn((
                    Name::new("Settings Panel"),
                    Node {
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        width: px(360),
                        padding: UiRect::all(px(20)),
                        border: UiRect::all(px(theme::BORDER_W)),
                        border_radius: BorderRadius::all(px(theme::RADIUS)),
                        ..default()
                    },
                    BorderColor::all(theme::BORDER),
                    BackgroundColor(theme::PANEL),
                ))
                .with_children(|parent| {
                    parent.spawn((
                        Name::new("Settings Title"),
                        Text::new("Settings"),
                        TextFont {
                            font_size: FontSize::Px(24.0),
                            ..default()
                        },
                        TextColor(theme::TEXT),
                    ));
                    parent.spawn((
                        Name::new("Settings Placeholder"),
                        Text::new("Nothing to configure yet."),
                        TextFont {
                            font_size: FontSize::Px(16.0),
                            ..default()
                        },
                        TextColor(theme::TEXT),
                    ));
                    parent.spawn((
                        Name::new("Settings Back Button"),
                        button("Back"),
                        observe(on_settings_back),
                    ));
                });
        });

    // Mods panel: hidden until the Mods button toggles it. Lists the installed
    // mods (from the catalog) with per-mod enable/disable toggles; base is shown
    // locked-on. `Explore online` is a coming-soon placeholder.
    let mods: Vec<ModEntry> = mod_catalog.map(|c| c.0.clone()).unwrap_or_default();
    let is_enabled = |id: &str| enabled.as_ref().is_some_and(|e| e.0.contains(id));

    commands
        .spawn((
            DespawnOnExit(GameStates::MainMenu),
            Name::new("Mods Panel Root"),
            ModsPanel,
            Visibility::Hidden,
            Pickable {
                should_block_lower: false,
                is_hoverable: false,
            },
            Node {
                width: percent(100),
                height: percent(100),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
        ))
        .with_children(|parent| {
            parent
                .spawn((
                    Name::new("Mods Panel"),
                    Node {
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        width: px(460),
                        max_height: percent(80),
                        padding: UiRect::all(px(20)),
                        border: UiRect::all(px(theme::BORDER_W)),
                        border_radius: BorderRadius::all(px(theme::RADIUS)),
                        ..default()
                    },
                    BorderColor::all(theme::BORDER),
                    BackgroundColor(theme::PANEL),
                ))
                .with_children(|parent| {
                    parent.spawn((
                        Name::new("Mods Title"),
                        Text::new("Mods"),
                        TextFont {
                            font_size: FontSize::Px(24.0),
                            ..default()
                        },
                        TextColor(theme::TEXT),
                    ));
                    parent.spawn((
                        Name::new("Mods Subtitle"),
                        Text::new("Enable installed mods. Base is always on."),
                        TextFont {
                            font_size: FontSize::Px(13.0),
                            ..default()
                        },
                        TextColor(theme::TEXT_MUTED),
                    ));

                    // The scrollable list of installed mods.
                    parent
                        .spawn((
                            Name::new("Mods List"),
                            ModsScrollPanel,
                            Node {
                                flex_direction: FlexDirection::Column,
                                align_self: AlignSelf::Stretch,
                                overflow: Overflow::scroll_y(),
                                margin: UiRect::vertical(px(10)),
                                ..default()
                            },
                            ScrollPosition::default(),
                        ))
                        .with_children(|list| {
                            for m in &mods {
                                spawn_mod_row(list, m, is_enabled(&m.id));
                            }
                        });

                    // Explore online: a disabled coming-soon placeholder (not a
                    // MenuButton, so it takes no hover/press feedback and has no
                    // observer - it is inert on purpose).
                    parent.spawn((
                        Name::new("Explore Online Button"),
                        Node {
                            width: percent(100),
                            min_height: px(40),
                            margin: UiRect::all(px(8)),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            border: UiRect::all(px(theme::BORDER_W)),
                            border_radius: BorderRadius::all(px(theme::RADIUS)),
                            ..default()
                        },
                        BorderColor::all(theme::BORDER),
                        BackgroundColor(theme::BG),
                        children![(
                            Text::new("Explore online (coming soon)"),
                            TextFont {
                                font_size: FontSize::Px(14.0),
                                ..default()
                            },
                            TextColor(theme::TEXT_MUTED),
                        )],
                    ));

                    parent.spawn((
                        Name::new("Mods Back Button"),
                        button("Back"),
                        observe(on_mods_back),
                    ));
                });
        });
}

/// Spawn one mod row: name + description, then either a toggle button (its label
/// set by `update_mod_toggle_labels`) or, for the locked `base` mod, a static
/// "Enabled (base)" tag.
fn spawn_mod_row(list: &mut ChildSpawnerCommands, m: &ModEntry, enabled: bool) {
    list.spawn((
        Name::new(format!("Mod Row: {}", m.id)),
        Node {
            flex_direction: FlexDirection::Column,
            align_self: AlignSelf::Stretch,
            padding: UiRect::all(px(8)),
            margin: UiRect::bottom(px(4)),
            border: UiRect::all(px(theme::BORDER_W)),
            border_radius: BorderRadius::all(px(theme::RADIUS)),
            ..default()
        },
        BorderColor::all(theme::BORDER),
        BackgroundColor(theme::PANEL_RAISED),
        children![
            (
                Name::new("Mod Name"),
                Text::new(m.name.clone()),
                TextFont {
                    font_size: FontSize::Px(16.0),
                    ..default()
                },
                TextColor(theme::TEXT),
            ),
            (
                Name::new("Mod Description"),
                Text::new(m.description.clone()),
                TextFont {
                    font_size: FontSize::Px(12.0),
                    ..default()
                },
                TextColor(theme::TEXT_MUTED),
            ),
        ],
    ))
    .with_children(|row| {
        if m.base {
            row.spawn((
                Name::new("Mod Locked Tag"),
                Text::new("Enabled (base)"),
                TextFont {
                    font_size: FontSize::Px(13.0),
                    ..default()
                },
                TextColor(theme::CYAN),
            ));
        } else {
            row.spawn((
                Name::new("Mod Toggle Button"),
                button(if enabled { "Enabled" } else { "Disabled" }),
                ModToggle {
                    id: m.id.clone(),
                    base: m.base,
                },
                observe(on_mod_toggle),
            ));
        }
    });
}

fn on_new_game(
    _activate: On<Activate>,
    mut mode: ResMut<GameMode>,
    mut state: ResMut<NextState<GameStates>>,
) {
    *mode = GameMode::NewGame;
    state.set(GameStates::Playing);
}

fn on_sandbox(
    _activate: On<Activate>,
    mut mode: ResMut<GameMode>,
    mut state: ResMut<NextState<GameStates>>,
) {
    *mode = GameMode::Sandbox;
    state.set(GameStates::Playing);
}

/// Tear the backdrop down whenever the menu is left, no matter through which
/// button or future path. The editor does not unload scenarios on entry, and
/// a forgotten unload would leave the ambience simulating behind the game.
fn unload_menu_ambience(mut commands: Commands) {
    commands.trigger(UnloadScenario);
}

fn on_settings(_activate: On<Activate>, mut panel: Single<&mut Visibility, With<SettingsPanel>>) {
    **panel = match **panel {
        Visibility::Hidden => Visibility::Visible,
        _ => Visibility::Hidden,
    };
}

fn on_settings_back(
    _activate: On<Activate>,
    mut panel: Single<&mut Visibility, With<SettingsPanel>>,
) {
    **panel = Visibility::Hidden;
}

fn on_mods(_activate: On<Activate>, mut panel: Single<&mut Visibility, With<ModsPanel>>) {
    **panel = match **panel {
        Visibility::Hidden => Visibility::Visible,
        _ => Visibility::Hidden,
    };
}

fn on_mods_back(_activate: On<Activate>, mut panel: Single<&mut Visibility, With<ModsPanel>>) {
    **panel = Visibility::Hidden;
}

/// Toggle a mod's enabled state on click. Reads the clicked button's [`ModToggle`]
/// and flips its id in [`EnabledMods`] - which nova_assets' `resource_changed`
/// re-merge then applies live. The `base` mod is locked on (its row has no toggle
/// button, but guard here too).
fn on_mod_toggle(
    activate: On<Activate>,
    toggles: Query<&ModToggle>,
    mut enabled: ResMut<EnabledMods>,
) {
    let Ok(toggle) = toggles.get(activate.entity) else {
        return;
    };
    if toggle.base {
        return;
    }
    if enabled.0.contains(&toggle.id) {
        enabled.0.remove(&toggle.id);
    } else {
        enabled.0.insert(toggle.id.clone());
    }
}

/// Keep each mod toggle button's label + text colour in sync with [`EnabledMods`]
/// (e.g. after a click, or a future persisted set). The button background is left to
/// `update_button_colors` (hover/press); the label text carries the on/off state.
fn update_mod_toggle_labels(
    enabled: Option<Res<EnabledMods>>,
    toggles: Query<(&ModToggle, &Children)>,
    mut texts: Query<(&mut Text, &mut TextColor)>,
) {
    let Some(enabled) = enabled else {
        return;
    };
    for (toggle, children) in &toggles {
        let on = enabled.0.contains(&toggle.id);
        let label = if on { "Enabled" } else { "Disabled" };
        let color = if on { theme::CYAN_BRIGHT } else { theme::TEXT };
        for child in children.iter() {
            if let Ok((mut text, mut text_color)) = texts.get_mut(child) {
                if text.0 != label {
                    text.0 = label.to_string();
                }
                if text_color.0 != color {
                    text_color.0 = color;
                }
            }
        }
    }
}

/// Mouse-wheel scroll for the mods list (the editor's scroll pattern), so a long
/// installed-mods list stays reachable.
fn scroll_mods_panel(
    mut wheel: MessageReader<bevy::input::mouse::MouseWheel>,
    mut panels: Query<&mut ScrollPosition, With<ModsScrollPanel>>,
) {
    use bevy::input::mouse::MouseScrollUnit;
    let dy: f32 = wheel
        .read()
        .map(|ev| match ev.unit {
            MouseScrollUnit::Line => ev.y * 20.0,
            MouseScrollUnit::Pixel => ev.y,
        })
        .sum();
    if dy == 0.0 {
        return;
    }
    for mut scroll in &mut panels {
        scroll.0.y = (scroll.0.y - dy).max(0.0);
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn on_exit(_activate: On<Activate>, mut exit: MessageWriter<AppExit>) {
    exit.write(AppExit::Success);
}

/// In `NewGame` mode the menu itself provides the game: load the canned scenario
/// (player ship included) the moment gameplay starts. `Sandbox` mode does nothing
/// here - the editor owns that path.
fn start_new_game_scenario(mut commands: Commands, scenarios: Res<GameScenarios>) {
    let scenario = scenarios
        .get(NEW_GAME_SCENARIO_ID)
        .unwrap_or_else(|| panic!("Scenario '{NEW_GAME_SCENARIO_ID}' not found"))
        .clone();
    commands.trigger(LoadScenario(scenario));
}

/// Hover/press feedback for the menu buttons. The editor drives the same feedback
/// through per-event observers; a single polling system is enough for four buttons
/// and keeps the menu self-contained.
fn update_button_colors(
    mut buttons: Query<
        (
            &Hovered,
            Has<Pressed>,
            &mut BackgroundColor,
            &mut BorderColor,
        ),
        (With<MenuButton>, With<Button>),
    >,
) {
    for (hovered, pressed, mut color, mut border) in &mut buttons {
        // Crisp instrument hover, matching the editor/web app: fill brightens and
        // the border shifts to cyan on press, bright on hover.
        let (fill, edge) = if pressed {
            (theme::SELECTED_FILL, theme::CYAN)
        } else if hovered.get() {
            (theme::PANEL_RAISED, theme::BORDER_BRIGHT)
        } else {
            (theme::PANEL, theme::BORDER)
        };
        if color.0 != fill {
            color.0 = fill;
        }
        border.set_all(edge);
    }
}

fn button(text: &str) -> impl Bundle {
    (
        Node {
            width: percent(100),
            min_height: px(40),
            margin: UiRect::all(px(8)),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            border: UiRect::all(px(theme::BORDER_W)),
            border_radius: BorderRadius::all(px(theme::RADIUS)),
            ..default()
        },
        MenuButton,
        Button,
        Hovered::default(),
        BorderColor::all(theme::BORDER),
        BackgroundColor(theme::PANEL),
        children![(
            Text::new(text),
            TextFont {
                font_size: FontSize::Px(16.0),
                ..default()
            },
            TextColor(theme::TEXT),
            TextShadow::default(),
        )],
    )
}

#[cfg(test)]
mod tests {
    use bevy::state::app::StatesPlugin;

    use super::*;

    /// A headless app with just enough for the menu's non-UI wiring: states, the
    /// mode resource, and the plugin itself. Tests that enter MainMenu also run
    /// the OnEnter systems (setup_menu_ui spawns plain components; the HUD
    /// level is a plain resource write), so insert `dummy_scenarios()` first -
    /// load_menu_ambience reads GameScenarios.
    fn app() -> App {
        let mut app = App::new();
        app.add_plugins(StatesPlugin);
        app.init_state::<GameStates>();
        app.init_state::<PauseStates>();
        app.init_resource::<GameMode>();
        app.init_resource::<ButtonInput<KeyCode>>();
        // Headless: no TimePlugin, so provide the clocks the pause systems
        // touch.
        app.insert_resource(Time::<Virtual>::default());
        app.insert_resource(Time::<Physics>::default());
        app.add_plugins(NovaMenuPlugin);
        app
    }

    fn enter_playing(app: &mut App) {
        app.world_mut()
            .resource_mut::<NextState<GameStates>>()
            .set(GameStates::Playing);
        app.update();
    }

    fn press_escape(app: &mut App) {
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Escape);
        app.update();
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.release(KeyCode::Escape);
        keys.clear();
        app.update();
    }

    fn pause_state(app: &App) -> PauseStates {
        app.world().resource::<State<PauseStates>>().get().clone()
    }

    fn clocks_paused(app: &App) -> (bool, bool) {
        (
            app.world().resource::<Time<Virtual>>().is_paused(),
            app.world().resource::<Time<Physics>>().is_paused(),
        )
    }

    #[derive(Resource, Default)]
    struct LoadedScenario(Option<String>);

    fn observe_load_scenario(app: &mut App) {
        app.init_resource::<LoadedScenario>();
        app.add_observer(
            |load: On<LoadScenario>, mut loaded: ResMut<LoadedScenario>| {
                loaded.0 = Some(load.0.id.clone());
            },
        );
    }

    fn dummy_scenario(id: &str) -> (String, ScenarioConfig) {
        (
            id.to_string(),
            ScenarioConfig {
                id: id.to_string(),
                name: "Test".to_string(),
                description: "Test".to_string(),
                cubemap: AssetRef::default(),
                events: vec![],
            },
        )
    }

    fn dummy_scenarios() -> GameScenarios {
        GameScenarios(bevy::platform::collections::HashMap::from([
            dummy_scenario(NEW_GAME_SCENARIO_ID),
            dummy_scenario(MENU_AMBIENCE_SCENARIO_ID),
        ]))
    }

    #[derive(Resource, Default)]
    struct Unloaded(bool);

    fn observe_unload_scenario(app: &mut App) {
        app.init_resource::<Unloaded>();
        app.add_observer(|_: On<UnloadScenario>, mut unloaded: ResMut<Unloaded>| {
            unloaded.0 = true;
        });
    }

    /// Count of UI cues played, standing in for "sounds heard".
    #[derive(Resource, Default)]
    struct PlayedCues(usize);

    /// A headless app with a loaded [`SoundBank`] and a `PlaySfx` counter, on
    /// MinimalPlugins so the AssetPlugin task pools exist (task 20260714-090006).
    fn cue_app() -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.init_asset::<AudioSource>();
        app.insert_resource(SoundBank::load(
            app.world().resource::<AssetServer>(),
            NOVA_SFX_FILES,
        ));
        app.init_resource::<PlayedCues>();
        app.add_observer(|_: On<PlaySfx>, mut cues: ResMut<PlayedCues>| cues.0 += 1);
        app
    }

    #[test]
    fn a_menu_button_activation_clicks_and_a_bare_activation_does_not() {
        let mut app = cue_app();
        app.add_observer(on_menu_button_activate);
        // A real menu button (carries MenuButton via `button()`) and a bare
        // entity that merely gets Activate'd.
        let menu_button = app.world_mut().spawn(button("New Game")).id();
        let bare = app.world_mut().spawn_empty().id();
        app.update();

        app.world_mut().trigger(Activate { entity: bare });
        app.update();
        assert_eq!(
            app.world().resource::<PlayedCues>().0,
            0,
            "a non-MenuButton activation is silent"
        );

        app.world_mut().trigger(Activate {
            entity: menu_button,
        });
        app.update();
        assert_eq!(
            app.world().resource::<PlayedCues>().0,
            1,
            "pressing a menu button clicks once"
        );
    }

    #[test]
    fn the_escape_pause_toggle_blips_on_both_directions() {
        let mut app = cue_app();
        app.add_plugins(StatesPlugin);
        app.init_state::<PauseStates>();
        app.init_resource::<ButtonInput<KeyCode>>();
        // Bare (no in_state run condition): drive the toggle directly.
        app.add_systems(Update, toggle_pause);

        let tap_escape = |app: &mut App| {
            app.world_mut()
                .resource_mut::<ButtonInput<KeyCode>>()
                .press(KeyCode::Escape);
            app.update();
            let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            keys.release(KeyCode::Escape);
            keys.clear();
            app.update();
        };

        tap_escape(&mut app); // open
        assert_eq!(pause_state(&app), PauseStates::Paused);
        tap_escape(&mut app); // close
        assert_eq!(pause_state(&app), PauseStates::Unpaused);

        assert_eq!(
            app.world().resource::<PlayedCues>().0,
            2,
            "ESC open and ESC close each blip once"
        );
    }

    #[test]
    fn new_game_button_sets_mode_and_hands_off_to_playing() {
        let mut app = app();
        app.insert_resource(dummy_scenarios());
        observe_load_scenario(&mut app);
        let button = app.world_mut().spawn(observe(on_new_game)).id();
        app.update();

        app.world_mut().trigger(Activate { entity: button });
        app.update();

        assert_eq!(*app.world().resource::<GameMode>(), GameMode::NewGame);
        assert_eq!(
            *app.world().resource::<State<GameStates>>().get(),
            GameStates::Playing
        );
        // Delivery guard: the handoff must actually load the scenario, not just
        // flip states.
        assert_eq!(
            app.world().resource::<LoadedScenario>().0.as_deref(),
            Some(NEW_GAME_SCENARIO_ID)
        );
    }

    #[test]
    fn sandbox_button_sets_mode_and_loads_no_scenario() {
        let mut app = app();
        app.insert_resource(dummy_scenarios());
        let button = app.world_mut().spawn(observe(on_sandbox)).id();
        // Enter the menu first (the real path), so leaving it exercises the
        // uniform OnExit teardown.
        app.world_mut()
            .resource_mut::<NextState<GameStates>>()
            .set(GameStates::MainMenu);
        app.update();
        // Observers registered after entry, so the menu's own ambience load
        // does not count against the "loads nothing" assertion below.
        observe_load_scenario(&mut app);
        observe_unload_scenario(&mut app);

        app.world_mut().trigger(Activate { entity: button });
        app.update();

        assert_eq!(*app.world().resource::<GameMode>(), GameMode::Sandbox);
        assert_eq!(
            *app.world().resource::<State<GameStates>>().get(),
            GameStates::Playing
        );
        // The editor owns the Sandbox path; the menu must not load anything,
        // and it must tear the ambience backdrop down (the editor does not).
        assert_eq!(app.world().resource::<LoadedScenario>().0, None);
        assert!(app.world().resource::<Unloaded>().0);
        // Leaving the menu restores the HUD level.
        assert_eq!(*app.world().resource::<HudVisibility>(), HudVisibility::All);
    }

    /// Entering MainMenu loads the ambience backdrop through the real
    /// OnEnter systems (task 20260711-180455).
    #[test]
    fn entering_main_menu_loads_the_ambience_scenario() {
        let mut app = app();
        app.insert_resource(dummy_scenarios());
        observe_load_scenario(&mut app);
        app.update();

        app.world_mut()
            .resource_mut::<NextState<GameStates>>()
            .set(GameStates::MainMenu);
        app.update();
        app.update();

        assert_eq!(
            app.world().resource::<LoadedScenario>().0.as_deref(),
            Some(MENU_AMBIENCE_SCENARIO_ID)
        );
        // The menu is a cinematic shot: entering drives the HUD level to None
        // (the absorbed status-bar hide, task 20260711-180501).
        assert_eq!(
            *app.world().resource::<HudVisibility>(),
            HudVisibility::None
        );
    }

    /// Delivery-guarded per press: ESC pauses, freezes both clocks, and a
    /// second press resumes and unfreezes.
    #[test]
    fn escape_toggles_pause_and_both_clocks() {
        let mut app = app();
        app.insert_resource(dummy_scenarios());
        enter_playing(&mut app);
        assert_eq!(pause_state(&app), PauseStates::Unpaused);
        assert_eq!(clocks_paused(&app), (false, false));

        press_escape(&mut app);
        assert_eq!(pause_state(&app), PauseStates::Paused);
        assert_eq!(clocks_paused(&app), (true, true), "both clocks freeze");

        press_escape(&mut app);
        assert_eq!(pause_state(&app), PauseStates::Unpaused);
        assert_eq!(clocks_paused(&app), (false, false), "both clocks resume");
    }

    /// The pause overlay spawns with its buttons and despawns on resume.
    #[test]
    fn pause_overlay_spawns_and_despawns() {
        let mut app = app();
        app.insert_resource(dummy_scenarios());
        enter_playing(&mut app);
        press_escape(&mut app);

        let find = |app: &mut App, name: &str| {
            let mut q = app.world_mut().query::<(Entity, &Name)>();
            q.iter(app.world())
                .find(|(_, n)| n.as_str() == name)
                .map(|(e, _)| e)
        };
        assert!(find(&mut app, "Resume Button").is_some());
        let back = find(&mut app, "Back To Menu Button").expect("back button exists");

        // Resume via the real button, then the overlay must be gone.
        let resume = find(&mut app, "Resume Button").unwrap();
        app.world_mut().trigger(Activate { entity: resume });
        app.update();
        app.update();
        assert_eq!(pause_state(&app), PauseStates::Unpaused);
        assert!(
            find(&mut app, "Pause Overlay").is_none(),
            "overlay despawns"
        );
        // The back button entity died with the overlay.
        assert!(app.world().get_entity(back).is_err());
    }

    /// Back to Main Menu from a paused game: lands in MainMenu, unpaused,
    /// clocks running, and the ambience backdrop load fired (which is what
    /// tears the gameplay scenario down).
    #[test]
    fn back_to_menu_unpauses_and_reloads_the_ambience() {
        let mut app = app();
        app.insert_resource(dummy_scenarios());
        observe_load_scenario(&mut app);
        enter_playing(&mut app);
        press_escape(&mut app);
        assert_eq!(
            clocks_paused(&app),
            (true, true),
            "paused before backing out"
        );

        let back = {
            let mut q = app.world_mut().query::<(Entity, &Name)>();
            q.iter(app.world())
                .find(|(_, n)| n.as_str() == "Back To Menu Button")
                .map(|(e, _)| e)
                .expect("back button exists")
        };
        app.world_mut().trigger(Activate { entity: back });
        app.update();
        app.update();

        assert_eq!(
            *app.world().resource::<State<GameStates>>().get(),
            GameStates::MainMenu
        );
        assert_eq!(pause_state(&app), PauseStates::Unpaused);
        assert_eq!(clocks_paused(&app), (false, false));
        assert_eq!(
            app.world().resource::<LoadedScenario>().0.as_deref(),
            Some(MENU_AMBIENCE_SCENARIO_ID)
        );
    }

    /// Review R1.3: exercise the REAL New Game button, not just the handler fn.
    /// Builds the actual menu UI headless, finds the button by Name, and clicks
    /// it - so dropping the observe(on_new_game) wiring from setup_menu_ui fails
    /// this test.
    #[test]
    fn real_new_game_button_is_wired() {
        use bevy::ecs::system::RunSystemOnce;

        let mut app = app();
        app.insert_resource(dummy_scenarios());
        observe_load_scenario(&mut app);
        app.world_mut()
            .run_system_once(setup_menu_ui)
            .expect("setup_menu_ui runs headless");
        app.update();

        let button = {
            let mut names = app.world_mut().query::<(Entity, &Name)>();
            names
                .iter(app.world())
                .find(|(_, name)| name.as_str() == "New Game Button")
                .map(|(entity, _)| entity)
                .expect("the menu spawns a 'New Game Button'")
        };
        app.world_mut().trigger(Activate { entity: button });
        app.update();

        assert_eq!(*app.world().resource::<GameMode>(), GameMode::NewGame);
        assert_eq!(
            *app.world().resource::<State<GameStates>>().get(),
            GameStates::Playing
        );
        assert_eq!(
            app.world().resource::<LoadedScenario>().0.as_deref(),
            Some(NEW_GAME_SCENARIO_ID)
        );
    }

    fn spawn_planetoid_well(app: &mut App) {
        app.world_mut().spawn((
            Transform::from_xyz(0.0, 0.0, 0.0),
            GravityWell {
                mu: 2400.0,
                body_radius: 80.0,
                soi_radius: 600.0,
            },
            EntityId::new(MENU_PLANETOID_ID),
        ));
    }

    /// Review R1.1: the camera staging that carried dev bug 1 (pose written in
    /// the same frame as the controller removal gets overwritten by the
    /// controller). Frame 1 must deactivate + strip the controller; frame 2
    /// must stage the runtime-derived pose and reactivate.
    #[test]
    fn menu_camera_is_staged_from_the_wells_geometry() {
        let mut app = app();
        app.insert_resource(dummy_scenarios());
        app.world_mut()
            .resource_mut::<NextState<GameStates>>()
            .set(GameStates::MainMenu);
        app.update();

        let cam = app
            .world_mut()
            .spawn((
                Camera3d::default(),
                WASDCameraController,
                Transform::from_xyz(0.0, 10.0, 20.0),
            ))
            .id();
        spawn_planetoid_well(&mut app);

        app.update();
        assert!(
            !app.world().get::<Camera>(cam).unwrap().is_active,
            "camera must be blanked while the controller is still attached"
        );
        app.update();
        assert!(
            app.world().get::<WASDCameraController>(cam).is_none(),
            "controller must be stripped"
        );
        // r_orbit = body_radius 80 + clearance 40 = 120 -> pose (0, 90, 300).
        let staged = app.world().get::<Transform>(cam).unwrap().translation;
        assert!(
            (staged - Vec3::new(0.0, 90.0, 300.0)).length() < 1e-3,
            "pose must derive from the well's runtime geometry, got {staged:?}"
        );
        assert!(app.world().get::<Camera>(cam).unwrap().is_active);
    }

    /// Clicking a non-base mod's toggle flips its id in `EnabledMods` (the set the
    /// nova_assets re-merge watches). Driven via `trigger(Activate)` like the other
    /// button tests.
    #[test]
    fn mod_toggle_flips_enabled_state() {
        let mut app = app();
        app.insert_resource(EnabledMods::default());
        let toggle = app
            .world_mut()
            .spawn((
                ModToggle {
                    id: "demo".to_string(),
                    base: false,
                },
                observe(on_mod_toggle),
            ))
            .id();
        app.update();

        // Enable.
        app.world_mut().trigger(Activate { entity: toggle });
        app.update();
        assert!(
            app.world().resource::<EnabledMods>().0.contains("demo"),
            "clicking an off toggle enables the mod"
        );

        // Disable.
        app.world_mut().trigger(Activate { entity: toggle });
        app.update();
        assert!(
            !app.world().resource::<EnabledMods>().0.contains("demo"),
            "clicking an on toggle disables the mod"
        );
    }

    /// Entering the menu with a populated `ModCatalog` builds the mods list: the
    /// demo mod gets a toggle button (it is IN the list and enableable - the flow's
    /// goal), and the locked base mod gets NO toggle.
    #[test]
    fn mods_panel_lists_catalog_demo_toggle_base_locked() {
        let mut app = app();
        app.insert_resource(dummy_scenarios());
        app.insert_resource(ModCatalog(vec![
            ModEntry {
                id: "base".to_string(),
                name: "Base Game".to_string(),
                description: "base".to_string(),
                bundle: "base/base.bundle.ron".to_string(),
                base: true,
            },
            ModEntry {
                id: "demo".to_string(),
                name: "Demo Mod".to_string(),
                description: "demo".to_string(),
                bundle: "mods/demo/demo.bundle.ron".to_string(),
                base: false,
            },
        ]));
        app.insert_resource(EnabledMods(["base".to_string()].into_iter().collect()));
        app.world_mut()
            .resource_mut::<NextState<GameStates>>()
            .set(GameStates::MainMenu);
        app.update();

        let toggles: Vec<String> = app
            .world_mut()
            .query::<&ModToggle>()
            .iter(app.world())
            .map(|t| t.id.clone())
            .collect();
        assert!(
            toggles.contains(&"demo".to_string()),
            "the demo mod is listed with an enable/disable toggle"
        );
        assert!(
            !toggles.contains(&"base".to_string()),
            "base is locked - it has no toggle button"
        );
    }

    /// The base mod is locked on: even if a `ModToggle { base: true }` were clicked,
    /// `on_mod_toggle` is a no-op, so base stays enabled.
    #[test]
    fn base_mod_toggle_is_locked_on() {
        let mut app = app();
        app.insert_resource(EnabledMods(["base".to_string()].into_iter().collect()));
        let toggle = app
            .world_mut()
            .spawn((
                ModToggle {
                    id: "base".to_string(),
                    base: true,
                },
                observe(on_mod_toggle),
            ))
            .id();
        app.update();

        app.world_mut().trigger(Activate { entity: toggle });
        app.update();
        assert!(
            app.world().resource::<EnabledMods>().0.contains("base"),
            "base is locked - toggling it must not disable it"
        );
    }
}
