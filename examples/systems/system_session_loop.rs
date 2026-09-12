//! system_session_loop: one game SESSION after another, out of the same
//! process.
//!
//! Boots the shipped app (via [`editor_app`]) with a `--scenario` launch
//! request, then walks the whole return trip a store player takes: ESC, Back to
//! Main Menu, New Game, and round again - through real pointer clicks and real
//! key presses, never by writing a state resource.
//!
//! ONE SUBJECT: what a session LEAVES BEHIND. Returning to the menu restarts
//! the content (the Wesnoth rule - the menu is where the game catches up with
//! what is on disk), and a restart is the moment every once-only piece of the
//! first boot can quietly double: the status bar's root, the launch scenario
//! that is supposed to be spent, a menu built and thrown away between the
//! gameplay the player left and the loading screen. The range holds the count
//! of each at one.
//!
//! Nothing here asserts what the training range CONTAINS - `system_menu_boot`
//! owns the New Game click and the range owns itself.
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//! ```text
//! NOVA_AUTOPILOT=1 cargo run --example system_session_loop --features debug
//! # look for: `session_loop: the launch request was spent on the first boot`,
//! #           `session_loop: one restart, one menu, one backdrop`,
//! #           `autopilot: cycle complete, no panic`
//! ```

#[cfg(feature = "debug")]
use bevy::prelude::*;
use clap::Parser;
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "system_session_loop")]
#[command(version = "1.0.0")]
#[command(
    about = "Session after session out of one process: the launch request is spent, the content restart leaves one menu, one backdrop and one status bar. Autopilot-only correctness range - play the game to use the menu",
    long_about = None
)]
struct Cli;

/// The scenario the run is LAUNCHED with (the binary's `--scenario <id>`).
///
/// A menu backdrop on purpose: it is the cheapest registered scenario to stand
/// up, and the flag reaches one by design (see `report_unknown_startup_scenario`
/// in nova_core). What it contains is not the subject - that it is spent after
/// one boot is.
#[cfg(feature = "debug")]
const LAUNCH_SCENARIO: &str = "menu_weave";

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();

    #[cfg(feature = "debug")]
    let startup = Some(StartupScenario::Id(LAUNCH_SCENARIO.to_string()));
    #[cfg(not(feature = "debug"))]
    let startup = None;

    // The same app the game/binary runs, launched the way `--scenario` launches
    // it - not a bespoke copy.
    let mut app = editor_app(true, startup);

    #[cfg(feature = "debug")]
    {
        if std::env::var_os("NOVA_AUTOPILOT").is_some() {
            app.insert_resource(bevy::ecs::error::FallbackErrorHandler(
                bevy::ecs::error::panic,
            ));
        }
        // No frame-time claim: the walk PAUSES (ESC is how a player leaves),
        // and a capture window that spans a stopped `Time<Virtual>` is refused
        // by the recorder - correctly. This range measures counts, not
        // milliseconds.
        app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
        app.add_plugins(SessionCensusPlugin);
        app.add_plugins(session_script());
    }

    app.run()
}

/// What the walk counts, over the whole run.
///
/// Counted by the app itself rather than sampled at beat boundaries: a menu
/// that is built and torn down INSIDE one frame pair is exactly the defect this
/// range holds down, and a sampler would never see it.
#[cfg(feature = "debug")]
#[derive(Resource, Default, Debug)]
struct SessionCensus {
    /// `OnEnter(GameAssetsStates::Loading)`: the boot load, plus one per
    /// content restart.
    asset_loads: u32,
    /// `OnEnter(GameStates::MainMenu)`: one per menu the player is shown.
    menu_entries: u32,
    /// Every `LoadScenario` raised while the main menu owns the screen - the
    /// ambience backdrop, and nothing else loads there.
    backdrop_loads: u32,
}

/// The census, wired to the production transitions it counts.
#[cfg(feature = "debug")]
struct SessionCensusPlugin;

#[cfg(feature = "debug")]
impl Plugin for SessionCensusPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SessionCensus>();
        app.add_systems(
            OnEnter(GameAssetsStates::Loading),
            |mut census: ResMut<SessionCensus>| census.asset_loads += 1,
        );
        app.add_systems(
            OnEnter(GameStates::MainMenu),
            |mut census: ResMut<SessionCensus>| census.menu_entries += 1,
        );
        app.add_observer(
            |_: On<LoadScenario>,
             state: Res<State<GameStates>>,
             mut census: ResMut<SessionCensus>| {
                if *state.get() == GameStates::MainMenu {
                    census.backdrop_loads += 1;
                }
            },
        );
    }
}

/// Read the census out of the world.
#[cfg(feature = "debug")]
fn census(world: &World) -> (u32, u32, u32) {
    let census = world.resource::<SessionCensus>();
    (
        census.asset_loads,
        census.menu_entries,
        census.backdrop_loads,
    )
}

/// The New Game button, and the node whose absence proves the menu tore down.
#[cfg(feature = "debug")]
const NEW_GAME_BUTTON: &str = "New Game Button";
/// The pause overlay's way out.
#[cfg(feature = "debug")]
const BACK_TO_MENU_BUTTON: &str = "Back To Menu Button";

/// Seconds a scenario load or a content restart is given. Sized to outlast the
/// training range's load on a software-rendered CI GPU, and kept under the
/// harness completion deadline so a stall names THIS beat.
#[cfg(feature = "debug")]
const SESSION_SECS: f32 = 90.0;

/// How many status bar roots, FPS items and version items are in the world.
///
/// By `Name`, like `system_menu_boot` counts its menu: the claim is about what
/// EXISTS, and a second root that happens to lay out on top of the first is
/// still a second root.
#[cfg(feature = "debug")]
fn status_bar_census(world: &World) -> (usize, usize, usize) {
    let named = |world: &World, wanted: &str| {
        world.try_query::<&Name>().map_or(0, |mut names| {
            names
                .iter(world)
                .filter(|name| name.as_str() == wanted)
                .count()
        })
    };
    (
        named(world, "StatusBarUIRoot"),
        named(world, "StatusBarItem: -fps"),
        named(world, "StatusBarItem: v-"),
    )
}

/// Advance once the return trip is OVER: the content restart has happened, the
/// state has settled on the menu, and the menu is laid out.
///
/// All three, because a menu that is about to be thrown away satisfies any one
/// of them on its own - the New Game button exists for the frame before the
/// restart tears it down, and `MainMenu` is entered again afterwards. Waiting
/// for the restart to land first means the census that follows counts a
/// FINISHED return, so a disposable menu shows up as a count rather than as a
/// race the walk might win.
///
/// `restarts` is the number of `OnEnter(GameAssetsStates::Loading)` the run has
/// paid for by then: the boot load plus one per return.
#[cfg(feature = "debug")]
fn the_menu_is_up(restarts: u32) -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    let laid_out = ui_node_present(NEW_GAME_BUTTON);
    let settled = state_is(GameStates::MainMenu);
    std::sync::Arc::new(move |world: &World| {
        world.resource::<SessionCensus>().asset_loads >= restarts
            && settled(world)
            && laid_out(world)
    })
}

/// The session loop, one beat per gesture.
#[cfg(feature = "debug")]
fn session_script() -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
        .step("session_loop: the launch request opens its scenario")
        .until(state_is(GameStates::Playing))
        .deadline(SESSION_SECS)
        .add()
        .step("session_loop: the launched scenario is the one asked for")
        .until(scenario_is(LAUNCH_SCENARIO))
        .deadline(SESSION_SECS)
        .add()
        // ESC, then the pause overlay's own Back - the store player's way out,
        // not a state write.
        .step("session_loop: press ESC")
        .on_enter(press_key(KeyCode::Escape))
        .until(ui_node_present(BACK_TO_MENU_BUTTON))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("session_loop: let ESC go")
        .on_enter(release_key(KeyCode::Escape))
        .add()
        .step("session_loop: click Back to Main Menu")
        .on_enter(click_named(BACK_TO_MENU_BUTTON))
        .until(pointer_pressed())
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("session_loop: release Back to Main Menu")
        .on_enter(release_mouse(MouseButton::Left))
        .until(the_menu_is_up(2))
        .deadline(SESSION_SECS)
        .add()
        .step("session_loop: the launch request was spent on the first boot")
        .on_enter(|world: &mut World| {
            // THE one-shot claim. A launch request that survived the restart
            // would have put the player straight back into its scenario, and
            // Back to Main Menu would be a door that opens onto itself.
            assert_eq!(
                *world.resource::<State<GameStates>>().get(),
                GameStates::MainMenu,
                "Back to Main Menu after a `--scenario` launch must reach the MENU; \
                 the launch request is spent on the first boot"
            );
            nova_probe::probe_marker(
                world,
                "outcome: the launch scenario opens once and the menu is reachable",
                serde_json::json!({ "launch": LAUNCH_SCENARIO }),
            );
            info!("session_loop: the launch request was spent on the first boot");
        })
        .add()
        .step("session_loop: one restart, one menu, one backdrop")
        .on_enter(|world: &mut World| {
            let (asset_loads, menu_entries, backdrop_loads) = census(world);
            // Boot load + exactly one restart. Two would mean the return
            // bounced through a screen that asked for its own.
            assert_eq!(
                asset_loads, 2,
                "one return to the menu is one content restart (boot load + 1), \
                 saw {asset_loads} asset loads"
            );
            nova_probe::probe_marker(
                world,
                "outcome: leaving gameplay restarts the content exactly once",
                serde_json::json!({ "asset_loads": asset_loads }),
            );
            // The defect this range was written for: `Playing -> MainMenu ->
            // Loading -> MainMenu` built the whole front door, drew a random
            // ambience backdrop, loaded it, and threw all of it away one frame
            // later.
            assert_eq!(
                menu_entries, 1,
                "the return must enter the menu ONCE, not build a disposable one \
                 before the restart; saw {menu_entries} menu entries"
            );
            assert_eq!(
                backdrop_loads, 1,
                "one menu entry draws one backdrop; saw {backdrop_loads} backdrop loads"
            );
            nova_probe::probe_marker(
                world,
                "outcome: the restart builds no throwaway menu or backdrop",
                serde_json::json!({
                    "menu_entries": menu_entries,
                    "backdrop_loads": backdrop_loads,
                }),
            );
            info!("session_loop: one restart, one menu, one backdrop");
        })
        .add()
        .step("session_loop: the restarted menu wears one status bar")
        .on_enter(|world: &mut World| {
            let (roots, fps, version) = status_bar_census(world);
            // `insert_status_bar_item` hangs every item off a `Single` root, so
            // a second root does not add a second bar - it silences the items
            // the restart spawned.
            assert_eq!(
                (roots, fps, version),
                (1, 1, 1),
                "a content restart rebuilds the status bar, it does not stack one: \
                 saw {roots} root(s), {fps} FPS item(s), {version} version item(s)"
            );
            nova_probe::probe_marker(
                world,
                "outcome: the restart leaves one status bar with its FPS and version",
                serde_json::json!({ "roots": roots, "fps": fps, "version": version }),
            );
            info!("session_loop: one status root, one FPS item, one version item");
        })
        .add()
        .step("session_loop: the first session left nothing behind")
        .on_enter(|world: &mut World| {
            // The menu is a cinematic shot over ONE backdrop. Anything the
            // played scenario left - its player hull, a second 3D camera, a
            // status bar that never hid - would be visible in it.
            let players = count::<PlayerSpaceshipMarker>(world);
            assert_eq!(
                players, 0,
                "the menu must carry no player hull out of the session it ended; \
                 {players} survived"
            );
            let hud = *world.resource::<HudVisibility>();
            assert_eq!(
                hud,
                HudVisibility::Cinematic,
                "the menu hides the HUD chrome on every entry, first or fifth"
            );
            nova_probe::probe_marker(
                world,
                "outcome: a returned-to menu is a first-boot menu",
                serde_json::json!({ "players": players }),
            );
            info!("session_loop: the first session left nothing behind");
        })
        .add()
        // Round two: the menu still works, and New Game starts the base
        // bundle's own start rather than the spent launch request.
        .step("session_loop: click New Game")
        .on_enter(click_named(NEW_GAME_BUTTON))
        .until(pointer_pressed())
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("session_loop: release New Game")
        .on_enter(release_mouse(MouseButton::Left))
        .until(state_is(GameStates::Playing))
        .deadline(SESSION_SECS)
        .add()
        .step("session_loop: the second session is a NEW session")
        .on_enter(|world: &mut World| {
            let current = current_scenario_id(world);
            assert!(
                current.as_deref().is_some_and(|id| id != LAUNCH_SCENARIO),
                "New Game must start the bundle's declared start, not the spent \
                 launch request; CurrentScenario is {current:?}"
            );
            nova_probe::probe_marker(
                world,
                "outcome: New Game after a restart starts the bundle's own start",
                serde_json::json!({ "scenario": current }),
            );
            info!("session_loop: the second session started {current:?}");
        })
        .add()
        // ...and round again, which is what makes this a LOOP rather than one
        // return: the second restart must cost exactly what the first did.
        .step("session_loop: press ESC again")
        .on_enter(press_key(KeyCode::Escape))
        .until(ui_node_present(BACK_TO_MENU_BUTTON))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("session_loop: let ESC go again")
        .on_enter(release_key(KeyCode::Escape))
        .add()
        .step("session_loop: click Back to Main Menu again")
        .on_enter(click_named(BACK_TO_MENU_BUTTON))
        .until(pointer_pressed())
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("session_loop: release Back to Main Menu again")
        .on_enter(release_mouse(MouseButton::Left))
        .until(the_menu_is_up(3))
        .deadline(SESSION_SECS)
        .add()
        .step("session_loop: the loop closes on the same counts")
        .on_enter(|world: &mut World| {
            let (asset_loads, menu_entries, backdrop_loads) = census(world);
            let (roots, fps, version) = status_bar_census(world);
            assert_eq!(
                (asset_loads, menu_entries, backdrop_loads),
                (3, 2, 2),
                "the second return must cost exactly what the first did: \
                 one restart, one menu entry, one backdrop"
            );
            assert_eq!(
                (roots, fps, version),
                (1, 1, 1),
                "two restarts still leave one status bar"
            );
            nova_probe::probe_marker(
                world,
                "outcome: the session loop repeats at the same cost",
                serde_json::json!({
                    "asset_loads": asset_loads,
                    "menu_entries": menu_entries,
                    "backdrop_loads": backdrop_loads,
                }),
            );
            info!("session_loop: the loop closed on the same counts");
        })
        .add()
}

/// How many entities carry `M`.
#[cfg(feature = "debug")]
fn count<M: Component>(world: &World) -> usize {
    world
        .try_query_filtered::<(), With<M>>()
        .map_or(0, |mut q| q.iter(world).count())
}

/// The live scenario's id, if one is loaded.
#[cfg(feature = "debug")]
fn current_scenario_id(world: &World) -> Option<String> {
    world
        .get_resource::<CurrentScenario>()
        .and_then(|current| current.0.as_ref().map(|scenario| scenario.id.clone()))
}

/// Advance once the named scenario is the live one.
#[cfg(feature = "debug")]
fn scenario_is(id: &'static str) -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(move |world: &World| {
        current_scenario_id(world).is_some_and(|live| live == id)
    })
}
