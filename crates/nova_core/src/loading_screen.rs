//! The boot loading screen: a full-screen phosphor panel shown while the game's
//! assets load, then torn down at the handoff to the menu/gameplay.
//!
//! Before this, native builds showed a blank window during
//! [`GameAssetsStates::Loading`] (the web build has its own HTML spinner). Now
//! the two-state boot chain in `nova_assets` preloads the UI font in
//! [`GameAssetsStates::Boot`], so from the first `Loading` frame this screen can
//! render themed text: a near-black CRT screen with a green-phosphor "NOVA OS"
//! mark and a "LOADING" line carrying an indeterminate CRT animation (marching
//! amber dots + a blinking block cursor). No progress tracking - the animation
//! is purely time-driven, so it needs no per-collection wiring or extra
//! dependency.
//!
//! The screen owns its own 2D UI camera because nothing else renders during
//! loading (the scene/menu cameras spawn later). Both the panel and its camera
//! despawn at `OnEnter(`[`GameAssetsStates::Loaded`]`)`, before the
//! menu/gameplay handoff spawns their own cameras.

use bevy::prelude::*;
use nova_assets::prelude::GameAssetsStates;
use nova_ui::font::UiFont;

/// Near-black CRT screen (PoC `--screen`). The panel background.
const LOADING_BACKDROP: Color = Color::srgb_u8(0, 3, 6);
/// Hot neon phosphor (PoC `--phosphor`) for the mark and cursor.
const LOADING_PHOSPHOR: Color = Color::srgb_u8(54, 255, 121);
/// Pale mint body text (PoC `--text`) for the LOADING label.
const LOADING_TEXT: Color = Color::srgb_u8(185, 255, 201);
/// Amber accent (PoC `--amber`) for the marching dots.
const LOADING_AMBER: Color = Color::srgb_u8(255, 184, 74);

/// The block-cursor glyph (full block, present in the shipped Iosevka face).
const CURSOR_GLYPH: &str = "\u{2588}";
/// Cursor blink half-period, seconds (the PoC `.caret` ~1.06s blink).
const CURSOR_BLINK_SECS: f32 = 0.53;
/// Marching-dots step, seconds: the dot count cycles 0..=3 at this cadence.
const DOTS_MARCH_SECS: f32 = 0.4;

/// The full-screen loading panel root (despawned recursively at `Loaded`).
#[derive(Component)]
struct LoadingScreenMarker;

/// The dedicated 2D UI camera the loading screen renders through.
#[derive(Component)]
struct LoadingScreenCameraMarker;

/// The blinking block cursor text.
#[derive(Component)]
struct LoadingCursorMarker;

/// The marching-dots text after the LOADING label.
#[derive(Component)]
struct LoadingDotsMarker;

/// Spawns/animates/tears down the boot [loading screen](self).
pub struct LoadingScreenPlugin;

impl Plugin for LoadingScreenPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameAssetsStates::Loading), spawn_loading_screen);
        app.add_systems(
            Update,
            animate_loading_screen.run_if(in_state(GameAssetsStates::Loading)),
        );
        app.add_systems(OnEnter(GameAssetsStates::Loaded), despawn_loading_screen);
    }
}

/// Build a phosphor text bundle at `size` in the given `color`.
fn loading_text(text: &str, size: f32, color: Color, font: &Handle<Font>) -> impl Bundle {
    (
        Text::new(text),
        TextColor(color),
        TextFont {
            font: FontSource::Handle(font.clone()),
            font_size: FontSize::Px(size),
            ..default()
        },
    )
}

/// Spawn the loading screen (camera + full-screen panel) at `OnEnter(Loading)`.
///
/// The UI font is preloaded in `Boot` and published as [`UiFont`] at
/// `OnExit(Boot)`, which runs before this, so the handle is normally present; a
/// missing resource (a rig that never ran the boot collection) falls back to
/// the engine default font rather than failing.
fn spawn_loading_screen(mut commands: Commands, ui_font: Option<Res<UiFont>>) {
    let font = ui_font.map(|f| f.handle()).unwrap_or_default();

    commands.spawn((
        Name::new("LoadingScreenCamera"),
        LoadingScreenCameraMarker,
        Camera2d,
    ));

    commands
        .spawn((
            Name::new("LoadingScreen"),
            LoadingScreenMarker,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                row_gap: Val::Px(18.0),
                ..default()
            },
            BackgroundColor(LOADING_BACKDROP),
        ))
        .with_children(|screen| {
            screen.spawn(loading_text("NOVA OS", 44.0, LOADING_PHOSPHOR, &font));
            screen
                .spawn(Node {
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    ..default()
                })
                .with_children(|line| {
                    line.spawn(loading_text("LOADING", 20.0, LOADING_TEXT, &font));
                    line.spawn((
                        loading_text("", 20.0, LOADING_AMBER, &font),
                        LoadingDotsMarker,
                    ));
                    line.spawn((
                        loading_text(CURSOR_GLYPH, 20.0, LOADING_PHOSPHOR, &font),
                        LoadingCursorMarker,
                    ));
                });
        });
}

/// Drive the indeterminate CRT animation while in `Loading`: blink the block
/// cursor and march the amber dots (0..=3), both purely time-driven.
fn animate_loading_screen(
    time: Res<Time>,
    mut cursor_elapsed: Local<f32>,
    mut cursor_on: Local<bool>,
    mut dots_elapsed: Local<f32>,
    mut dots_count: Local<usize>,
    mut q_cursor: Query<&mut Visibility, With<LoadingCursorMarker>>,
    mut q_dots: Query<&mut Text, With<LoadingDotsMarker>>,
) {
    *cursor_elapsed += time.delta_secs();
    if *cursor_elapsed >= CURSOR_BLINK_SECS {
        *cursor_elapsed = 0.0;
        *cursor_on = !*cursor_on;
        let vis = if *cursor_on {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        for mut visibility in &mut q_cursor {
            *visibility = vis;
        }
    }

    *dots_elapsed += time.delta_secs();
    if *dots_elapsed >= DOTS_MARCH_SECS {
        *dots_elapsed = 0.0;
        *dots_count = (*dots_count + 1) % 4;
        let dots = ".".repeat(*dots_count);
        for mut text in &mut q_dots {
            text.0 = dots.clone();
        }
    }
}

/// Despawn the loading screen and its camera at `OnEnter(Loaded)`, before the
/// menu/gameplay handoff spawns its own camera.
fn despawn_loading_screen(
    mut commands: Commands,
    q_screen: Query<Entity, With<LoadingScreenMarker>>,
    q_camera: Query<Entity, With<LoadingScreenCameraMarker>>,
) {
    for entity in &q_screen {
        commands.entity(entity).despawn();
    }
    for entity in &q_camera {
        commands.entity(entity).despawn();
    }
}

#[cfg(test)]
mod tests {
    use bevy::state::app::StatesPlugin;

    use super::*;

    /// Live-tree test: walking the asset states must SPAWN the loading screen +
    /// its camera when entering `Loading`, and DESPAWN both when entering
    /// `Loaded`. Runs the real plugin systems (would fail if spawn or despawn
    /// were a no-op).
    #[test]
    fn loading_screen_spawns_in_loading_and_despawns_on_loaded() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, StatesPlugin));
        app.init_state::<GameAssetsStates>();
        app.add_plugins(LoadingScreenPlugin);

        app.update();
        assert_eq!(
            count::<LoadingScreenMarker>(&mut app),
            0,
            "loading screen must not exist before Loading"
        );

        app.world_mut()
            .resource_mut::<NextState<GameAssetsStates>>()
            .set(GameAssetsStates::Loading);
        app.update();
        assert_eq!(
            count::<LoadingScreenMarker>(&mut app),
            1,
            "loading screen must spawn on OnEnter(Loading)"
        );
        assert_eq!(
            count::<LoadingScreenCameraMarker>(&mut app),
            1,
            "loading screen camera must spawn on OnEnter(Loading)"
        );

        app.world_mut()
            .resource_mut::<NextState<GameAssetsStates>>()
            .set(GameAssetsStates::Loaded);
        app.update();
        assert_eq!(
            count::<LoadingScreenMarker>(&mut app),
            0,
            "loading screen must despawn on OnEnter(Loaded)"
        );
        assert_eq!(
            count::<LoadingScreenCameraMarker>(&mut app),
            0,
            "loading screen camera must despawn on OnEnter(Loaded)"
        );
    }

    /// The marching-dots text cycles through the four dot counts as time
    /// advances (would fail if the animation system were a no-op).
    #[test]
    fn loading_dots_march_over_time() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, StatesPlugin));
        app.init_state::<GameAssetsStates>();
        app.add_plugins(LoadingScreenPlugin);
        app.world_mut()
            .resource_mut::<NextState<GameAssetsStates>>()
            .set(GameAssetsStates::Loading);
        app.update();

        let mut seen = std::collections::HashSet::new();
        for _ in 0..40 {
            std::thread::sleep(std::time::Duration::from_millis(60));
            app.update();
            if let Some(text) = dots_text(&mut app) {
                seen.insert(text.len());
            }
        }
        assert!(
            seen.len() >= 3,
            "marching dots should cycle through multiple lengths, saw {seen:?}"
        );
    }

    fn count<M: Component>(app: &mut App) -> usize {
        app.world_mut()
            .query_filtered::<(), With<M>>()
            .iter(app.world())
            .count()
    }

    fn dots_text(app: &mut App) -> Option<String> {
        app.world_mut()
            .query_filtered::<&Text, With<LoadingDotsMarker>>()
            .iter(app.world())
            .next()
            .map(|t| t.0.clone())
    }
}
