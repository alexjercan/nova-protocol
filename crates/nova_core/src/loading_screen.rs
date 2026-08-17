//! The loading screens: the boot panel shown while the game's assets load, and
//! the scenario panel shown over a gameplay scenario swap.
//!
//! Both wear the same phosphor CRT face and share one animation, driven by
//! [`Time<Real>`] rather than the virtual clock: a scenario swap can be
//! requested from a PAUSED outcome frame, where the virtual clock is stopped and
//! a virtual-clock animation would sit frozen on the exact screen that exists to
//! prove the game is still alive. Real time also makes a long frame VISIBLE - the
//! sweep jumps the distance the stall cost instead of gliding through it.
//!
//! Change this module when the load presentation changes. The scenario
//! screen's dismissal rule is the interesting part: it is up for exactly as
//! long as the scenario is SETTLING (the engine's own spawn gate,
//! `EventWorld::is_settling`, which holds every handler while queued objects
//! land), with [`SCENARIO_MIN_DWELL`] as a floor and
//! [`SCENARIO_SETTLED_DELTA`] as the "the machine is smooth again" test.

use bevy::prelude::*;
use nova_assets::prelude::GameAssetsStates;
use nova_events::prelude::EventWorld;
use nova_gameplay::prelude::GameStates;
use nova_scenario::prelude::{LoadScenario, NovaEventWorld};
use nova_ui::font::UiFont;

/// Near-black CRT screen (PoC `--screen`). The panel background.
const LOADING_BACKDROP: Color = Color::srgb_u8(0, 3, 6);
/// Hot neon phosphor (PoC `--phosphor`) for the mark and cursor.
const LOADING_PHOSPHOR: Color = Color::srgb_u8(54, 255, 121);
/// Pale mint body text (PoC `--text`) for the LOADING label.
const LOADING_TEXT: Color = Color::srgb_u8(185, 255, 201);
/// Amber accent (PoC `--amber`) for the marching dots and the sweep block.
const LOADING_AMBER: Color = Color::srgb_u8(255, 184, 74);
/// Dim phosphor for the sweep track the block runs along.
const LOADING_TRACK: Color = Color::srgb_u8(13, 110, 53);

/// The block-cursor glyph (full block, present in the shipped Iosevka face).
const CURSOR_GLYPH: &str = "\u{2588}";
/// Cursor blink half-period, seconds (the PoC `.caret` ~1.06s blink).
const CURSOR_BLINK_SECS: f32 = 0.53;
/// Marching-dots step, seconds: the dot count cycles 0..=3 at this cadence.
const DOTS_MARCH_SECS: f32 = 0.4;

/// Sweep track width and height, pixels.
const SWEEP_TRACK: Vec2 = Vec2::new(260.0, 6.0);
/// Sweep block width, pixels.
const SWEEP_BLOCK_W: f32 = 44.0;
/// Seconds the sweep block takes to cross the track and come back.
///
/// Short on purpose. It was sized when a load cost a handful of ~300 ms frames
/// and the block had to move a VISIBLE fraction of the track between two of
/// them; the spawn now chunks across smooth frames, where the same cycle simply
/// reads as motion. Kept short so a long frame still shows as a jump.
const SWEEP_CYCLE_SECS: f32 = 1.1;

/// How long the scenario screen stays up at minimum, seconds.
///
/// The FLOOR, not the rule: the spawn gate is what normally holds the panel,
/// until every object the scenario asked for has landed. This is what stops a
/// scenario that spawns nothing from flashing a panel up and down inside two
/// RENDERED frames, which reads as a flicker rather than as a screen.
const SCENARIO_MIN_DWELL: f32 = 0.6;
/// The frame delta the scenario screen calls "settled", seconds. Past the
/// minimum dwell and the spawn gate, the panel comes down on the first frame at
/// or under this - so a machine still hitching on the newly built scene keeps
/// its cover.
const SCENARIO_SETTLED_DELTA: f32 = 0.05;
/// Hard cap on the scenario screen, seconds: a machine that never gets back
/// under [`SCENARIO_SETTLED_DELTA`] must still be given its game back. It does
/// NOT cap the spawn gate - a big scene taking longer than this to build is
/// working, not stuck, and the panel belongs over it.
const SCENARIO_MAX_DWELL: f32 = 6.0;

/// The full-screen BOOT loading panel root (despawned recursively at `Loaded`).
#[derive(Component)]
struct LoadingScreenMarker;

/// The dedicated 2D UI camera the boot loading screen renders through.
#[derive(Component)]
struct LoadingScreenCameraMarker;

/// The full-screen SCENARIO loading panel root, and the hold that decides when
/// it comes down.
#[derive(Component)]
struct ScenarioLoadScreenMarker {
    /// `Time<Real>` elapsed when the load was requested.
    started: f32,
}

/// The blinking block cursor text.
#[derive(Component)]
struct LoadingCursorMarker;

/// The marching-dots text after the LOADING label.
#[derive(Component)]
struct LoadingDotsMarker;

/// The block that sweeps back and forth along its track.
#[derive(Component)]
struct LoadingSweepMarker;

/// Spawns/animates/tears down both [loading screens](self).
pub struct LoadingScreenPlugin;

impl Plugin for LoadingScreenPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameAssetsStates::Loading), spawn_loading_screen);
        app.add_systems(OnEnter(GameAssetsStates::Loaded), despawn_loading_screen);
        app.add_observer(spawn_scenario_load_screen);
        // Deliberately ungated: one animation drives BOTH screens, which live in
        // different states, and the gate that matters is simply whether a panel
        // exists - which is what the (empty) queries already answer. Chained so a
        // panel is never taken down on a frame it was not first animated for.
        app.add_systems(
            Update,
            (animate_loading_screen, dismiss_scenario_load_screen).chain(),
        );
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

/// The CRT panel both screens wear: a full-screen backdrop over a centred
/// column - the `mark`, the LOADING line (label + marching dots + cursor), and
/// the sweep track.
///
/// One builder rather than two, so the boot screen and the scenario screen
/// cannot drift apart; the only difference between them is the mark and who
/// despawns them.
fn loading_panel(mark: &str, font: Handle<Font>) -> impl Bundle {
    (
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
        children![
            loading_text(mark, 44.0, LOADING_PHOSPHOR, &font),
            (
                Node {
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    ..default()
                },
                children![
                    loading_text("LOADING", 20.0, LOADING_TEXT, &font),
                    (
                        loading_text("", 20.0, LOADING_AMBER, &font),
                        LoadingDotsMarker,
                    ),
                    (
                        loading_text(CURSOR_GLYPH, 20.0, LOADING_PHOSPHOR, &font),
                        LoadingCursorMarker,
                    ),
                ],
            ),
            (
                Node {
                    width: Val::Px(SWEEP_TRACK.x),
                    height: Val::Px(SWEEP_TRACK.y),
                    ..default()
                },
                BackgroundColor(LOADING_TRACK),
                children![(
                    LoadingSweepMarker,
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(0.0),
                        width: Val::Px(SWEEP_BLOCK_W),
                        height: Val::Px(SWEEP_TRACK.y),
                        ..default()
                    },
                    BackgroundColor(LOADING_AMBER),
                )],
            ),
        ],
    )
}

/// The UI font, or the engine default when the boot collection never ran (a
/// headless rig): a missing handle must not fail the screen.
fn ui_font_handle(font: Option<Res<UiFont>>) -> Handle<Font> {
    font.map(|font| font.handle()).unwrap_or_default()
}

/// Spawn the boot loading screen (camera + full-screen panel) at
/// `OnEnter(Loading)`.
///
/// It owns a 2D UI camera because nothing else renders during boot - the
/// scene/menu cameras spawn later. The UI font is preloaded in `Boot` and
/// published as [`UiFont`] at `OnExit(Boot)`, which runs before this, so the
/// handle is normally present.
fn spawn_loading_screen(mut commands: Commands, ui_font: Option<Res<UiFont>>) {
    commands.spawn((
        Name::new("LoadingScreenCamera"),
        LoadingScreenCameraMarker,
        Camera2d,
    ));

    commands.spawn((
        Name::new("LoadingScreen"),
        LoadingScreenMarker,
        loading_panel("NOVA OS", ui_font_handle(ui_font)),
    ));
}

/// Raise the scenario loading screen when a gameplay scenario is (re)loaded.
///
/// PLAYING ONLY. The menu loads a backdrop scenario of its own on every entry,
/// and covering the front door with a LOADING panel would both hide the menu the
/// player just asked for and swallow the frames a click needs to land.
///
/// No camera of its own: a scenario load happens inside gameplay, which always
/// has one. Re-triggering while the panel is already up (a chapter chain that
/// switches twice) restarts the hold rather than stacking a second panel.
fn spawn_scenario_load_screen(
    _: On<LoadScenario>,
    mut commands: Commands,
    state: Option<Res<State<GameStates>>>,
    time: Res<Time<Real>>,
    ui_font: Option<Res<UiFont>>,
    mut q_existing: Query<&mut ScenarioLoadScreenMarker>,
) {
    if !state.is_some_and(|state| *state.get() == GameStates::Playing) {
        return;
    }
    let started = time.elapsed_secs();
    if let Some(mut existing) = q_existing.iter_mut().next() {
        existing.started = started;
        return;
    }

    commands.spawn((
        Name::new("Scenario Loading Screen"),
        ScenarioLoadScreenMarker { started },
        // Dies with gameplay: backing out to the menu mid-load must not leave
        // the panel over the front door.
        DespawnOnExit(GameStates::Playing),
        // Above the pause overlay (10) and its settings modal (11): a load
        // requested from a paused outcome frame draws over both.
        GlobalZIndex(100),
        // Deliberately NOT a modal blocker: the panel is up for a handful of
        // frames and swallowing a click in that window is worse than letting
        // one through to a screen the player cannot see anyway.
        Pickable::IGNORE,
        loading_panel("LOADING SCENARIO", ui_font_handle(ui_font)),
    ));
}

/// Take the scenario screen down once the swap is done: never while the
/// scenario is still spawning, and then after the minimum dwell on the first
/// frame back under [`SCENARIO_SETTLED_DELTA`], or at the hard cap.
///
/// The spawn gate ([`EventWorld::is_settling`]) makes the panel and the script
/// gate ONE fact: the scenario engine holds every handler while its queued
/// objects are still landing, and the panel says so on screen for exactly that
/// long. It is checked BEFORE the cap on purpose - the cap exists for a machine
/// that never gets smooth again, not for a scene that is legitimately big, and
/// the spawn queue is finite and always makes progress (at least one object per
/// frame), so this cannot hold forever. Optional, so a rig without the scenario
/// engine (the boot screen's own tests) dismisses on the frame rule alone.
///
/// Reads `Time<Real>` for both the dwell and the settle test - a load can be
/// requested from a paused outcome frame, where the virtual clock is stopped and
/// the panel would never come down.
fn dismiss_scenario_load_screen(
    mut commands: Commands,
    time: Res<Time<Real>>,
    event_world: Option<Res<NovaEventWorld>>,
    q_screen: Query<(Entity, &ScenarioLoadScreenMarker)>,
) {
    if event_world.is_some_and(|world| world.is_settling()) {
        return;
    }
    let now = time.elapsed_secs();
    let delta = time.delta_secs();
    for (entity, screen) in &q_screen {
        let held = now - screen.started;
        let settled = held >= SCENARIO_MIN_DWELL && delta <= SCENARIO_SETTLED_DELTA;
        if settled || held >= SCENARIO_MAX_DWELL {
            commands.entity(entity).despawn();
        }
    }
}

/// Drive the indeterminate CRT animation on whichever screen is up: blink the
/// block cursor, march the amber dots (0..=3), and sweep the block along its
/// track.
///
/// Time-driven, no progress tracking, so it needs no per-collection wiring. The
/// clock is [`Time<Real>`]: see the module docs for why the virtual one cannot
/// be used, and the sweep is derived from ABSOLUTE elapsed time rather than
/// accumulated deltas so a stalled frame moves it by what the stall cost.
fn animate_loading_screen(
    time: Res<Time<Real>>,
    mut q_cursor: Query<&mut Visibility, With<LoadingCursorMarker>>,
    mut q_dots: Query<&mut Text, With<LoadingDotsMarker>>,
    mut q_sweep: Query<&mut Node, With<LoadingSweepMarker>>,
) {
    let elapsed = time.elapsed_secs();

    let cursor_on = ((elapsed / CURSOR_BLINK_SECS) as u32).is_multiple_of(2);
    let vis = if cursor_on {
        Visibility::Visible
    } else {
        Visibility::Hidden
    };
    for mut visibility in &mut q_cursor {
        visibility.set_if_neq(vis);
    }

    let dots = ".".repeat((elapsed / DOTS_MARCH_SECS) as usize % 4);
    for mut text in &mut q_dots {
        if text.0 != dots {
            text.0 = dots.clone();
        }
    }

    // Ping-pong across the free travel: 0 -> 1 -> 0 over one cycle.
    let phase = (elapsed / SWEEP_CYCLE_SECS).fract();
    let travel = (phase * 2.0 - 1.0).abs();
    let left = Val::Px((SWEEP_TRACK.x - SWEEP_BLOCK_W) * (1.0 - travel));
    for mut node in &mut q_sweep {
        node.left = left;
    }
}

/// Despawn the boot loading screen and its camera at `OnEnter(Loaded)`, before
/// the menu/gameplay handoff spawns its own camera.
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
    use nova_scenario::prelude::ScenarioConfig;

    use super::*;

    /// Live-tree test: walking the asset states must SPAWN the boot loading
    /// screen + its camera when entering `Loading`, and DESPAWN both when
    /// entering `Loaded`. Runs the real plugin systems (would fail if spawn or
    /// despawn were a no-op).
    #[test]
    fn loading_screen_spawns_in_loading_and_despawns_on_loaded() {
        let mut app = screen_app();

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

    /// The marching dots cycle and the sweep block MOVES as real time advances.
    /// The sweep is the load-bearing half: the dots step four times a cycle, so
    /// two frames of a stuttering load can easily land on the same dot count,
    /// while the sweep is at a different offset on every frame.
    #[test]
    fn the_indicator_moves_between_frames() {
        let mut app = screen_app();
        app.world_mut()
            .resource_mut::<NextState<GameAssetsStates>>()
            .set(GameAssetsStates::Loading);
        app.update();

        let mut dot_counts = std::collections::HashSet::new();
        let mut sweeps: Vec<f32> = Vec::new();
        for _ in 0..24 {
            std::thread::sleep(std::time::Duration::from_millis(60));
            app.update();
            dot_counts.insert(dots_text(&mut app).map(|text| text.len()));
            sweeps.push(sweep_left(&mut app).expect("the sweep block exists"));
        }

        assert!(
            dot_counts.len() >= 3,
            "marching dots should cycle through multiple lengths, saw {dot_counts:?}"
        );
        let moved = sweeps.windows(2).filter(|w| w[0] != w[1]).count();
        assert!(
            moved >= sweeps.len() - 2,
            "the sweep must be at a new offset on essentially every frame, \
             moved on {moved} of {} steps: {sweeps:?}",
            sweeps.len() - 1
        );
    }

    /// The scenario screen is a PLAYING-only cover: loading the menu's backdrop
    /// scenario must not curtain the front door.
    #[test]
    fn a_scenario_load_raises_the_screen_only_in_playing() {
        let mut app = screen_app();
        app.update();

        app.world_mut().trigger(LoadScenario(scenario()));
        app.update();
        assert_eq!(
            count::<ScenarioLoadScreenMarker>(&mut app),
            0,
            "the menu's backdrop load must not raise the scenario screen"
        );

        app.world_mut()
            .resource_mut::<NextState<GameStates>>()
            .set(GameStates::Playing);
        app.update();
        app.world_mut().trigger(LoadScenario(scenario()));
        app.update();
        assert_eq!(
            count::<ScenarioLoadScreenMarker>(&mut app),
            1,
            "a gameplay scenario load must raise the scenario screen"
        );

        // A second load under the same screen restarts the hold instead of
        // stacking a second panel.
        app.world_mut().trigger(LoadScenario(scenario()));
        app.update();
        assert_eq!(
            count::<ScenarioLoadScreenMarker>(&mut app),
            1,
            "a re-load must not stack a second panel"
        );
    }

    /// The screen holds for the minimum dwell and then comes down on a settled
    /// frame. Driven on the real clock, which is what the production rule reads.
    #[test]
    fn the_scenario_screen_holds_then_comes_down() {
        let mut app = screen_app();
        app.world_mut()
            .resource_mut::<NextState<GameStates>>()
            .set(GameStates::Playing);
        app.update();

        app.world_mut().trigger(LoadScenario(scenario()));
        app.update();
        assert_eq!(
            count::<ScenarioLoadScreenMarker>(&mut app),
            1,
            "delivery guard: the screen went up"
        );

        // Inside the minimum dwell the screen stays, settled frames or not.
        app.update();
        assert_eq!(
            count::<ScenarioLoadScreenMarker>(&mut app),
            1,
            "the screen must not flash away inside the minimum dwell"
        );

        std::thread::sleep(std::time::Duration::from_secs_f32(SCENARIO_MIN_DWELL));
        // One update to pay the sleep as a (long, unsettled) frame delta, then
        // a short one that settles.
        app.update();
        app.update();
        assert_eq!(
            count::<ScenarioLoadScreenMarker>(&mut app),
            0,
            "past the dwell, a settled frame takes the screen down"
        );
    }

    /// The panel and the scenario's spawn gate are ONE fact. The screen HOLDS
    /// while the scenario's queued objects are landing - past the minimum
    /// dwell, on smooth frames, and past the hard cap, which exists for a
    /// machine that never gets smooth and not for a scene that is legitimately
    /// big. It comes down on the first settled frame after the queue empties.
    /// Without the gate the panel would come down at the dwell and the player
    /// would watch the rest of the scene pop in.
    #[test]
    fn the_scenario_screen_holds_while_the_scenario_is_still_spawning() {
        let mut app = screen_app();
        app.init_resource::<NovaEventWorld>();
        app.world_mut()
            .resource_mut::<NextState<GameStates>>()
            .set(GameStates::Playing);
        app.update();

        // A scenario mid-spawn: one command still queued.
        app.world_mut()
            .resource_mut::<NovaEventWorld>()
            .push_command(|_: &mut Commands| {});
        app.world_mut().trigger(LoadScenario(scenario()));
        app.update();
        assert_eq!(
            count::<ScenarioLoadScreenMarker>(&mut app),
            1,
            "delivery guard: the screen went up"
        );

        // Backdate the hold well past the hard cap rather than sleeping it out:
        // both the dwell rule and the cap now say "come down", and only the
        // spawn gate is holding the panel.
        let world = app.world_mut();
        let mut q = world.query::<&mut ScenarioLoadScreenMarker>();
        for mut screen in q.iter_mut(world) {
            screen.started -= SCENARIO_MAX_DWELL * 2.0;
        }
        for _ in 0..3 {
            app.update();
        }
        assert_eq!(
            count::<ScenarioLoadScreenMarker>(&mut app),
            1,
            "the screen holds while the scenario is still spawning, cap included"
        );

        // The queue empties (production drains it; here the teardown reset does).
        app.world_mut().resource_mut::<NovaEventWorld>().clear();
        app.update();
        assert_eq!(
            count::<ScenarioLoadScreenMarker>(&mut app),
            0,
            "a spawned-out scenario lets the screen come down"
        );
    }

    fn screen_app() -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, StatesPlugin));
        app.init_state::<GameAssetsStates>();
        app.init_state::<GameStates>();
        app.add_plugins(LoadingScreenPlugin);
        app
    }

    fn scenario() -> ScenarioConfig {
        ScenarioConfig::new("probe", "Probe", "sky.png".into())
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

    fn sweep_left(app: &mut App) -> Option<f32> {
        app.world_mut()
            .query_filtered::<&Node, With<LoadingSweepMarker>>()
            .iter(app.world())
            .next()
            .and_then(|node| match node.left {
                Val::Px(px) => Some(px),
                _ => None,
            })
    }
}
