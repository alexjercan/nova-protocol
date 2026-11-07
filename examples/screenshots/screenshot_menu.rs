//! screenshot_menu: the main menu over the ambience backdrop
//! (`tutorial-menu.png`), the Settings panel open over it
//! (`wiki-settings.png`), its Controls tab (`wiki-controls.png`) and the New
//! Game world setup window holding a typed seed (`news-0150-world-setup.png`)
//! over its chart preview, the role step with the hull turning
//! (`menu-role-step.png`), then Create, a leave through the pause menu, the
//! menu offering Continue (`menu-continue.png`) and the Load screen listing
//! that save (`menu-load-screen.png`), driven
//! through the shipped app (`editor_app`).
//!
//! Two armed confirms are proven by rect, not by eye, at any window size and
//! interface scale (`NOVA_CAPTURE_SIZE`, `NOVA_CAPTURE_UI_SCALE`): Settings >
//! Controls > Reset Defaults (`menu-reset-armed.png`) and the Mods Uninstall
//! row of an installed fixture mod (`menu-uninstall-armed.png`). Each armed
//! Cancel must cover the idle box, Cancel and Confirm must stay inside their
//! scroll viewport and panel, Cancel must cancel and Confirm must confirm.
//! Every pointer target that can sit below a scroll fold is scrolled into view
//! with the harness wheel first.
//!
//! Two run modes, both under the autopilot (`NOVA_AUTOPILOT`):
//! - `NOVA_AUTOPILOT=1` alone: the smoke path - walk the menu, exit clean,
//!   capturing nothing.
//! - `NOVA_AUTOPILOT=1 NOVA_CAPTURE=1`: also write each PNG (staged under
//!   `NOVA_CAPTURE_DIR`).
//!
//! Capture (windowed, real GPU):
//! ```text
//! NOVA_CAPTURE_DIR=target/shots NOVA_AUTOPILOT=1 NOVA_CAPTURE=1 \
//!   cargo run --example screenshot_menu --features debug
//! ```
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//! ```text
//! NOVA_AUTOPILOT=1 cargo run --example screenshot_menu --features debug
//! # look for: `nova harness: reached Playing`, `autopilot: cycle complete, no panic`
//! ```

#[cfg(feature = "debug")]
use bevy::prelude::*;
use clap::Parser;
use nova_protocol::prelude::*;

// The pointer gestures, shared with the other menu walks. Script-only, so the
// whole module sits behind one gate here.
#[cfg(feature = "debug")]
#[path = "shared/ui_walk.rs"]
mod ui_walk;
#[cfg(feature = "debug")]
use std::sync::Arc;

#[cfg(feature = "debug")]
use nova_protocol::nova_debug::harness::{AutopilotPlugin, Predicate};
#[cfg(feature = "debug")]
use nova_ui::widget::TextFieldValue;
#[cfg(feature = "debug")]
use ui_walk::Gestures;

/// The backdrop this walk shoots against.
const BACKDROP: &str = "menu_gauntlet";

/// The one ship the gauntlet is built around, and the subject of the frame
/// behind the menu.
#[cfg(feature = "debug")]
const BACKDROP_SHIP_ID: &str = "gauntlet_ship";

/// Where the camera stands relative to that warship, in the HULL's own frame:
/// off the starboard bow (`-Z` is ahead of it), raised, about 370 m out.
///
/// The hull frame rather than world axes, because the warship flies a patrol
/// and a world-axis offset photographs whatever end the current leg presents -
/// the first cut of this shot came out stern-on, a drive bell filling the
/// middle of the menu. Riding the hull always gives the same three-quarter
/// bow. A `line_warship` is about 185 m stem to stern, and 370 m of standoff
/// makes a frame 540 m wide: the hull is about a third of it, so the sections and their
/// mounts stay separable, with the rock band still around them.
#[cfg(feature = "debug")]
const BACKDROP_EYE: Meters3 = Meters3::new(285.0, 110.0, -210.0);

/// The seed typed into the world setup window: the one the other v0.15.0
/// world figures fly, so the post shows one world from setup to flight.
#[cfg(feature = "debug")]
const WORLD_SEED: u32 = 115;

/// The world setup window's seed field.
#[cfg(feature = "debug")]
const SEED_FIELD: &str = "World Seed Field";

/// Backspaces that clear any seed the window opens with: it pre-fills a random
/// u32, at most ten digits.
#[cfg(feature = "debug")]
const SEED_DIGITS_MAX: usize = 10;

/// The still of the world setup window.
#[cfg(feature = "debug")]
const WORLD_SETUP_SHOT: &str = "news-0150-world-setup.png";

/// The still of the role step with the selected role's hull turning.
#[cfg(feature = "debug")]
const ROLE_STEP_SHOT: &str = "menu-role-step.png";

/// The still of the main menu offering Continue on the save just left.
#[cfg(feature = "debug")]
const CONTINUE_SHOT: &str = "menu-continue.png";

/// The still of the Load screen listing the world this walk left.
#[cfg(feature = "debug")]
const LOAD_SHOT: &str = "menu-load-screen.png";

/// The role step's hull view.
#[cfg(feature = "debug")]
const HULL_PREVIEW: &str = "Start Role Hull Preview";

/// The pause menu's leave to the main menu.
#[cfg(feature = "debug")]
const BACK_TO_MENU_BUTTON: &str = "Back To Menu Button";

/// The main menu's Continue, shown once a save stands.
#[cfg(feature = "debug")]
const CONTINUE_BUTTON: &str = "Continue Button";

/// The Load details' chart preview of the selected save.
#[cfg(feature = "debug")]
const LOAD_PREVIEW: &str = "Load World Details Preview";

/// A Load row's open objectives count.
#[cfg(feature = "debug")]
const LOAD_ROW_OPEN: &str = "Load World Row Open";

/// A Load row's play time, the column the open count must read like.
#[cfg(feature = "debug")]
const LOAD_ROW_PLAY_TIME: &str = "Load World Row Play Time";

/// The Load details' FLEET line naming the hull flown.
#[cfg(feature = "debug")]
const LOAD_FLEET_SHIP: &str = "Load World Details Fleet Ship";

/// One open objective in the Load details: the role goal stands at the leave.
#[cfg(feature = "debug")]
const LOAD_OBJECTIVE: &str = "Load World Details Objective";

/// The menu card's scrolling button list.
#[cfg(feature = "debug")]
const MENU_BUTTONS: &str = "Menu Buttons";

/// The Settings panel and its scrolling body.
#[cfg(feature = "debug")]
const SETTINGS_PANEL: &str = "Settings Panel";
#[cfg(feature = "debug")]
const SETTINGS_BODY: &str = "Settings Body";

/// Reset Defaults while idle, and the Confirm it becomes once armed (the armed
/// row keeps the node name), plus the Cancel that takes the idle box.
#[cfg(feature = "debug")]
const RESET_BUTTON: &str = "Reset Bindings";
#[cfg(feature = "debug")]
const RESET_CANCEL: &str = "Reset Bindings Cancel";
#[cfg(feature = "debug")]
const RESET_PROMPT: &str = "Reset Bindings Prompt";

/// The still of the armed Reset Defaults.
#[cfg(feature = "debug")]
const RESET_ARMED_SHOT: &str = "menu-reset-armed.png";

/// The key the walk moves one binding onto; no shipped binding uses it.
#[cfg(feature = "debug")]
const MOVED_KEY: KeyCode = KeyCode::F13;

/// The installed fixture mod the Uninstall walk removes. Written into this
/// run's own mod cache before the app starts, never the player's.
#[cfg(feature = "debug")]
const FIXTURE_MOD_ID: &str = "walk-uninstall-fixture";
#[cfg(feature = "debug")]
const FIXTURE_MOD_ROW: &str = "Mod Row: walk-uninstall-fixture";

/// The Mods screen's panel, its scrolling list and its details pane.
#[cfg(feature = "debug")]
const MODS_PANEL: &str = "Mods Panel";
#[cfg(feature = "debug")]
const MODS_LIST: &str = "Mods List";
#[cfg(feature = "debug")]
const MOD_DETAILS_PANEL: &str = "Mod Details Panel";

/// The Uninstall button while idle, and the Confirm it becomes once armed (the
/// armed row keeps the node name), plus the Cancel that takes the idle place.
#[cfg(feature = "debug")]
const UNINSTALL_BUTTON: &str = "Mod Details Uninstall Button";
#[cfg(feature = "debug")]
const UNINSTALL_CANCEL: &str = "Mod Details Uninstall Cancel";
#[cfg(feature = "debug")]
const UNINSTALL_PROMPT: &str = "Mod Details Uninstall Prompt";

/// The still of the armed Uninstall.
#[cfg(feature = "debug")]
const UNINSTALL_ARMED_SHOT: &str = "menu-uninstall-armed.png";

/// Slack for rect comparisons, in logical pixels: layout rounds each node edge
/// to a device pixel, so a child may end one pixel past its parent's rect.
#[cfg(feature = "debug")]
const RECT_SLACK: f32 = 1.0;

/// Frames between scroll corrections while a target is out of its viewport.
#[cfg(feature = "debug")]
const SCROLL_RETRY_FRAMES: u32 = 6;

#[derive(Parser)]
#[command(name = "screenshot_menu")]
#[command(version = "1.0.0")]
#[command(about = "Capture the main menu, its Settings panel, both New Game steps and the Load screen. Autopilot-only: a scripted pointer walk over the real menu", long_about = None)]
struct Cli;

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();

    // This run's own settings, saved worlds and installed mods, set before the
    // app reads them: Create makes a world, and neither a player's worlds nor
    // an earlier run's "New World" may be in its way. The mod cache moves too,
    // so the Uninstall walk removes a fixture from this run's own cache and
    // never touches the player's installed mods.
    let profile = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(format!(
        "target/example-profiles/{}-{}-{}",
        env!("CARGO_CRATE_NAME"),
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("the clock is after 1970")
            .as_nanos(),
    ));
    std::env::set_var(nova_assets::storage::CONFIG_ROOT_ENV, &profile);
    #[cfg(feature = "debug")]
    {
        std::env::set_var(
            nova_assets::mod_cache::prelude::MOD_CACHE_ROOT_ENV,
            profile.join("mod-cache"),
        );
        install_fixture_mod();
    }

    // Pin the backdrop before the app boots. Menu entry draws one of the four
    // shipped backdrops at RANDOM, so an unpinned capture is a different scene
    // every run and the page's figure note can only describe whichever one it
    // drew.
    //
    // The gauntlet, because it is the backdrop built around ONE named ship:
    // a warship holding a station against torpedo waves. The other three put
    // their traffic on a route or on a dogfight, so the hull in shot is
    // whichever one the act has carried into view.
    //
    // Set here rather than exported by the capture loop: the packager runs
    // every still producer through one generic command, so a producer that
    // needs an environment carries it itself.
    std::env::set_var(MENU_BACKDROP_ENV, BACKDROP);

    // The same app the game/binary runs (main menu over the ambience backdrop).
    let mut app = editor_app(true, None);

    #[cfg(feature = "debug")]
    {
        // Probe wiring (each plugin is inert without its NOVA_PROBE_* env):
        // run timeline + engine-bound invariants, so `probe run` grades this
        // example instead of asserting nothing. No frame-time capture - the
        // walk is a sequence of posed framings with no steady-state window,
        // so a captured fps would measure the script, not the engine.
        app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
        if std::env::var_os("NOVA_AUTOPILOT").is_some() {
            // Turn command errors (despawned-entity targets on the menu
            // teardown) into panics so the run fails loudly on them.
            app.insert_resource(bevy::ecs::error::FallbackErrorHandler(
                bevy::ecs::error::panic,
            ));
        }
        // Clean frames at a known 16:9 size: force the window resolution and drop
        // the dev overlays.
        app.add_systems(Startup, (force_capture_resolution, hide_dev_overlays));
        app.add_plugins(menu_script());
    }

    app.run()
}

/// The driven walk: menu -> Settings -> Controls -> world setup -> role step
/// -> first flight -> menu -> Load, one shot per screen.
#[cfg(feature = "debug")]
fn menu_script() -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    // The HUD chrome is dropped right before every shot rather than once at
    // `Startup`, because other states re-raise it. `shoot` itself is the capture
    // gate: unarmed, this whole walk runs and writes nothing.
    let shot = |path: &'static str| {
        move |world: &mut World| {
            hide_hud(world);
            shoot(world, path);
        }
    };

    nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
        .step("reach the main menu")
        .enter(GameStates::Loading)
        .until(state_is(GameStates::MainMenu))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("settle the menu and its ambience backdrop")
        .until(frames(SETTLE_FRAMES))
        .add()
        // The backdrop's own SetCamera is an ESTABLISHING shot - 2.6 km out,
        // where the warship is a speck over a rock band. The menu is the
        // reader's first sight of the fleet, so move in on the hull.
        .step("frame the backdrop's warship")
        .on_enter(frame_backdrop_ship)
        .until(frames(SETTLE_FRAMES))
        .add()
        // Hide the HUD first, and let the PNG land BEFORE navigating away:
        // clicking on in the same frame captured a black mid-teardown frame.
        .step("capture the main menu")
        .on_enter(shot("tutorial-menu.png"))
        .until(shot_written("tutorial-menu.png"))
        .deadline(SHOT_DEADLINE_SECS)
        .add()
        // The Settings panel: an overlay over the menu, not its own state. The
        // card's button list scrolls on a short or scaled window, so the button
        // is scrolled into view before the pointer aims at it.
        .click_scrolled("open Settings", "Settings Button", MENU_BUTTONS)
        .step("settle the settings panel")
        .until(frames(SETTLE_FRAMES))
        .add()
        // The panel is toggled by Visibility alone, so this has to assert
        // VISIBLE, not merely laid out: the shot is otherwise the main menu
        // again under a different name.
        .step("the settings panel is up")
        .on_enter(assert_named_visible("Settings Panel"))
        .add()
        .step("capture the settings panel")
        .on_enter(shot("wiki-settings.png"))
        .until(shot_written("wiki-settings.png"))
        .deadline(SHOT_DEADLINE_SECS)
        .add()
        // The panel opens on Audio, so the tab the range exists for - the
        // rebindable keycap rows - is one click further in and needs its own
        // shot. The wiki's Controls section reads on that picture.
        .click("open the Controls tab", "Settings Tab: Controls")
        .step("settle the controls tab")
        .until(frames(SETTLE_FRAMES))
        .add()
        .step("capture the controls tab")
        .on_enter(shot("wiki-controls.png"))
        .until(shot_written("wiki-controls.png"))
        .deadline(SHOT_DEADLINE_SECS)
        .add()
        .reset_defaults_walk()
        .click("close Settings", "Settings Back Button")
        .uninstall_walk()
        .click_scrolled("start a new game", "New Game Button", MENU_BUTTONS)
        // The window opens on a random seed, so the seed is replaced the way a
        // player replaces it: focus, caret to the end, clear, type. The keys
        // land in one frame.
        .click("focus the seed field", SEED_FIELD)
        .step("the seed field takes the keyboard")
        .until(frames(2))
        .add()
        .step("type the seed")
        .on_enter(|world: &mut World| {
            press_edit_key(Key::End)(world);
            for _ in 0..SEED_DIGITS_MAX {
                press_edit_key(Key::Backspace)(world);
            }
            type_text(WORLD_SEED.to_string())(world);
        })
        .until(seed_field_reads(WORLD_SEED))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("settle the world setup window")
        .until(frames(SETTLE_FRAMES))
        .add()
        // A station's chart code is its place in the found order, which no
        // seed knows before play, so the seed preview labels the start alone.
        .step("the seed preview labels only the start")
        .on_enter(|world: &mut World| {
            assert_eq!(
                shown_chart_labels(world),
                ["YOU"],
                "screenshot_menu: the seed preview must label the start alone"
            );
        })
        .add()
        .step("capture the world setup window")
        // The news figure ships with the post, so the footer's build id stays
        // out of it. The wiki shots above keep the footer: they are reshot with
        // the page.
        .on_enter(move |world: &mut World| {
            ui_walk::hide_menu_version(world);
            shot(WORLD_SETUP_SHOT)(world);
        })
        .until(shot_written(WORLD_SETUP_SHOT))
        .deadline(SHOT_DEADLINE_SECS)
        .add()
        // The name field keeps the name it opens on: Create on it is the
        // player's untouched path.
        .click("go on to the start role", "World Next Button")
        // The role step opens on the first role, so its hull view stands.
        .step("the role step shows the selected hull")
        .until(ui_node_present(HULL_PREVIEW))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("settle the role step")
        .until(frames(SETTLE_FRAMES))
        .add()
        .step("capture the role step")
        .on_enter(shot(ROLE_STEP_SHOT))
        .until(shot_written(ROLE_STEP_SHOT))
        .deadline(SHOT_DEADLINE_SECS)
        .add()
        .click("create the world", "Create World Button")
        .step("reach the first flight")
        .until(state_is(GameStates::Playing))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        // The shot is only the figure if Create starts the seed it shows.
        .step("Create started the typed seed")
        .on_enter(|world: &mut World| {
            assert_eq!(
                world.resource::<OpenWorldSession>().seed,
                WORLD_SEED,
                "screenshot_menu: Create must start the typed seed"
            );
        })
        .add()
        // Leave through the pause menu: the leave writes the save the Load
        // screen then lists, and the menu offers Continue on it.
        .step("open the pause menu")
        .on_enter(press_key(KeyCode::Escape))
        .until(ui_node_present(BACK_TO_MENU_BUTTON))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("let escape go")
        .on_enter(release_key(KeyCode::Escape))
        .add()
        .click_named(
            "back to the main menu",
            BACK_TO_MENU_BUTTON,
            state_is(GameStates::MainMenu),
            STEP_DEADLINE_SECS,
        )
        .step("the menu offers Continue on the save")
        .until(ui_node_present(CONTINUE_BUTTON))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("settle the menu with Continue")
        .until(frames(SETTLE_FRAMES))
        .add()
        .step("capture the menu with Continue")
        .on_enter(shot(CONTINUE_SHOT))
        .until(shot_written(CONTINUE_SHOT))
        .deadline(SHOT_DEADLINE_SECS)
        .add()
        .scroll_into_view("scroll to Load", "Load Button", MENU_BUTTONS)
        .click_named(
            "open Load",
            "Load Button",
            ui_node_present(LOAD_PREVIEW),
            BEAT_DEADLINE_SECS,
        )
        .step("the Load row and details show the open objectives and the fleet")
        .on_enter(|world: &mut World| {
            for name in [LOAD_ROW_OPEN, LOAD_FLEET_SHIP, LOAD_OBJECTIVE] {
                assert!(
                    named_node(world, name),
                    "screenshot_menu: the Load details lack '{name}'"
                );
            }
            let objectives = named_nodes(world, LOAD_OBJECTIVE);
            let open = node_text(world, LOAD_ROW_OPEN);
            assert_eq!(
                open,
                format!("{objectives} open"),
                "screenshot_menu: the Load row's open count does not match the {objectives} open objectives in its details"
            );
        })
        .add()
        .step("settle the Load screen")
        .until(frames(SETTLE_FRAMES))
        .add()
        // The walk left the world docked at its starting station, the one
        // station the save's chart has found.
        .step("the Load preview labels the found station and the player")
        .on_enter(|world: &mut World| {
            let labels = shown_chart_labels(world);
            for label in ["STN-1", "YOU"] {
                assert!(
                    labels.iter().any(|shown| shown == label),
                    "screenshot_menu: the Load preview does not show '{label}', it shows {labels:?}"
                );
            }
        })
        .add()
        .step("the Load row's open count reads in the other columns' ink")
        .on_enter(|world: &mut World| {
            // The selected row's fill is the label ink, so an open count in
            // label colour drew but did not read.
            let open = node_text_color(world, LOAD_ROW_OPEN);
            let play_time = node_text_color(world, LOAD_ROW_PLAY_TIME);
            assert_eq!(
                open, play_time,
                "screenshot_menu: the Load row's open count is not in the play time's colour"
            );
        })
        .add()
        .step("capture the Load screen")
        .on_enter(shot(LOAD_SHOT))
        .until(shot_written(LOAD_SHOT))
        .deadline(SHOT_DEADLINE_SECS)
        .add()
}

/// Hide the HUD chrome and write `path`: the capture step every shot here takes.
#[cfg(feature = "debug")]
fn capture_shot(path: &'static str) -> impl Fn(&mut World) + Send + Sync + 'static {
    move |world: &mut World| {
        hide_hud(world);
        shoot(world, path);
    }
}

/// Write the installed fixture mod into this run's own mod cache.
///
/// The Mods screen lists a cache record with no loaded bundle too, so the
/// manifest only has to be readable; it carries no content. Files first, index
/// last, as every install does.
#[cfg(feature = "debug")]
fn install_fixture_mod() {
    let manifest = format!(
        "(\n    content: [\"{FIXTURE_MOD_ID}.content.ron\"],\n    meta: (\n        name: \"Uninstall Fixture\",\n        description: \"A mod the screenshot walk installs and removes.\",\n        author: \"Nova Protocol\",\n        version: \"1.0.0\",\n        dependencies: [],\n    ),\n)\n"
    );
    let bundle = format!("{FIXTURE_MOD_ID}.bundle.ron");
    nova_assets::mod_cache::prelude::install_local(
        FIXTURE_MOD_ID,
        "1.0.0",
        &bundle,
        &[
            (bundle.clone(), manifest.into_bytes()),
            (format!("{FIXTURE_MOD_ID}.content.ron"), b"[]\n".to_vec()),
        ],
    )
    .expect("screenshot_menu: the fixture mod installs into this run's own cache");
}

/// The box a button had while idle, kept for the click on the same spot after
/// it armed.
#[cfg(feature = "debug")]
#[derive(Resource, Clone, Copy)]
struct IdleBox(Rect);

/// The action whose binding the walk moved off its default.
#[cfg(feature = "debug")]
#[derive(Resource, Clone)]
struct MovedBinding(String);

/// The laid-out rect of `name`, or a panic naming it.
#[cfg(feature = "debug")]
fn rect_of(world: &World, name: &str) -> Rect {
    ui_node_rect(world, name)
        .unwrap_or_else(|| panic!("screenshot_menu: no laid-out, visible node '{name}'"))
}

/// Whether `inner` lies inside `outer`, within [`RECT_SLACK`].
#[cfg(feature = "debug")]
fn rect_within(outer: Rect, inner: Rect) -> bool {
    inner.min.x >= outer.min.x - RECT_SLACK
        && inner.min.y >= outer.min.y - RECT_SLACK
        && inner.max.x <= outer.max.x + RECT_SLACK
        && inner.max.y <= outer.max.y + RECT_SLACK
}

/// Whether two rects share more than [`RECT_SLACK`] of area on both axes.
#[cfg(feature = "debug")]
fn rects_overlap(a: Rect, b: Rect) -> bool {
    a.max.x.min(b.max.x) - a.min.x.max(b.min.x) > RECT_SLACK
        && a.max.y.min(b.max.y) - a.min.y.max(b.min.y) > RECT_SLACK
}

/// Advance once `target` lies wholly inside `viewport`.
#[cfg(feature = "debug")]
fn in_view(target: &'static str, viewport: &'static str) -> Arc<Predicate> {
    Arc::new(move |world: &World| {
        ui_node_rect(world, target)
            .zip(ui_node_rect(world, viewport))
            .is_some_and(|(target, viewport)| rect_within(viewport, target))
    })
}

/// Advance once no visible node is called `name`.
#[cfg(feature = "debug")]
fn node_gone(name: &'static str) -> Arc<Predicate> {
    nova_autopilot::predicate::not(ui_node_present(name))
}

/// Scroll `viewport` by the gap that keeps `target` out of view, in the wheel's
/// own unit. Positive moves the content down.
///
/// The wheel reads logical pixels of the node tree and the rects are window
/// pixels, so the gap is divided by the interface scale. A target that is
/// already inside costs nothing.
#[cfg(feature = "debug")]
fn scroll_gap(world: &mut World, target: &str, viewport: &str) {
    let (Some(target), Some(viewport)) =
        (ui_node_rect(world, target), ui_node_rect(world, viewport))
    else {
        return;
    };
    let gap = if target.max.y > viewport.max.y {
        viewport.max.y - target.max.y
    } else if target.min.y < viewport.min.y {
        viewport.min.y - target.min.y
    } else {
        return;
    };
    let ui_scale = world.get_resource::<UiScale>().map_or(1.0, |scale| scale.0);
    scroll_pixels(gap / ui_scale)(world);
}

/// The pointer gestures and checks this walk adds to the shared kit.
#[cfg(feature = "debug")]
trait MenuWalk: Sized {
    /// Bring `target` wholly into `viewport` with the harness wheel, then hold
    /// until it is there. The pointer rests on the viewport first, because the
    /// wheel goes to the hovered one.
    fn scroll_into_view(self, label: &str, target: &'static str, viewport: &'static str) -> Self;

    /// [`scroll_into_view`](Self::scroll_into_view), then
    /// [`Gestures::click`] on `target`.
    fn click_scrolled(self, label: &str, target: &'static str, viewport: &'static str) -> Self;

    /// Press and release the pointer on the recorded [`IdleBox`] centre, which
    /// must pick `under`, then hold until `landed`.
    fn click_idle_spot(self, label: &str, under: &'static str, landed: Arc<Predicate>) -> Self;

    /// Settings > Controls: Reset Defaults, armed, cancelled, armed, confirmed.
    fn reset_defaults_walk(self) -> Self;

    /// Mods: Uninstall on the fixture mod, armed, cancelled, armed, confirmed.
    fn uninstall_walk(self) -> Self;
}

#[cfg(feature = "debug")]
impl MenuWalk for AutopilotPlugin<GameStates> {
    fn scroll_into_view(self, label: &str, target: &'static str, viewport: &'static str) -> Self {
        self.step(format!("{label}: the target is up"))
            .until(ui_node_present(target))
            .diagnose(ui_node_diagnosis(target))
            .deadline(BEAT_DEADLINE_SECS)
            .add()
            .step(format!("{label}: rest the pointer on the viewport"))
            .on_enter(hover_named(viewport))
            .until(pointer_over_node(viewport))
            .diagnose(pointer_hover_diagnosis(viewport))
            .deadline(BEAT_DEADLINE_SECS)
            .add()
            .step(format!("{label}: scroll"))
            .on_enter(move |world: &mut World| scroll_gap(world, target, viewport))
            // The first turn can fall short (a clamped end, a unit this guess
            // missed), so the gap is measured again every few frames.
            .each(move |world: &mut World, _, frame| {
                if frame % SCROLL_RETRY_FRAMES == SCROLL_RETRY_FRAMES - 1 {
                    scroll_gap(world, target, viewport);
                }
            })
            .until(in_view(target, viewport))
            .deadline(BEAT_DEADLINE_SECS)
            .add()
            .step(format!("{label}: let the scroll settle"))
            .until(frames(2))
            .add()
    }

    fn click_scrolled(self, label: &str, target: &'static str, viewport: &'static str) -> Self {
        self.scroll_into_view(label, target, viewport)
            .click(label, target)
    }

    fn click_idle_spot(self, label: &str, under: &'static str, landed: Arc<Predicate>) -> Self {
        self.step(format!("{label}: aim at the idle spot"))
            .on_enter(|world: &mut World| {
                let at = world.resource::<IdleBox>().0.center();
                move_cursor(at)(world);
            })
            .until(pointer_over_node(under))
            .diagnose(pointer_hover_diagnosis(under))
            .deadline(BEAT_DEADLINE_SECS)
            .add()
            .step(format!("{label}: press"))
            .on_enter(press_mouse(MouseButton::Left))
            .until(pointer_pressed())
            .deadline(BEAT_DEADLINE_SECS)
            .add()
            .step(format!("{label}: release"))
            .on_enter(release_mouse(MouseButton::Left))
            .until(landed)
            .deadline(BEAT_DEADLINE_SECS)
            .add()
    }

    fn reset_defaults_walk(self) -> Self {
        self
            // The binding differs from its default only through the world, so a
            // reset that does nothing is visible.
            .step("move one binding off its default")
            .on_enter(|world: &mut World| {
                let mut bindings = world.resource_mut::<nova_input::prelude::InputBindings>();
                let action = bindings
                    .rows()
                    .next()
                    .expect("screenshot_menu: the Controls tab lists no action");
                let name = action.name;
                let mut spec = action.spec();
                spec.keyboard = vec![nova_input::prelude::InputSource::from(MOVED_KEY)];
                assert!(
                    bindings.rebind(name, spec),
                    "screenshot_menu: the table refused the moved binding for '{name}'"
                );
                assert!(
                    bindings.overrides().contains_key(name),
                    "screenshot_menu: '{name}' still reads its default after the move"
                );
                world.insert_resource(MovedBinding(name.to_string()));
            })
            .until(frames(SETTLE_FRAMES))
            .add()
            .scroll_into_view("scroll to Reset Defaults", RESET_BUTTON, SETTINGS_BODY)
            .step("record the idle Reset Defaults box")
            .on_enter(|world: &mut World| {
                let idle = rect_of(world, RESET_BUTTON);
                info!("screenshot_menu: idle Reset Defaults {idle:?}");
                world.insert_resource(IdleBox(idle));
            })
            .add()
            .click_named(
                "arm Reset Defaults",
                RESET_BUTTON,
                ui_node_present(RESET_CANCEL),
                BEAT_DEADLINE_SECS,
            )
            .step("settle the armed Reset Defaults")
            .until(frames(SETTLE_FRAMES))
            .add()
            .step("capture the armed Reset Defaults")
            .on_enter(capture_shot(RESET_ARMED_SHOT))
            .until(shot_written(RESET_ARMED_SHOT))
            .deadline(SHOT_DEADLINE_SECS)
            .add()
            .step("the armed Reset Defaults keeps its box and stays in view")
            .on_enter(|world: &mut World| {
                let idle = world.resource::<IdleBox>().0;
                assert_armed_row(
                    world,
                    "Reset Defaults",
                    idle,
                    RESET_CANCEL,
                    RESET_BUTTON,
                    RESET_PROMPT,
                    &[SETTINGS_BODY, SETTINGS_PANEL],
                    0.0,
                );
            })
            .add()
            .click_idle_spot("cancel Reset Defaults", RESET_CANCEL, node_gone(RESET_CANCEL))
            .step("Cancel disarmed it and the moved binding stayed moved")
            .on_enter(|world: &mut World| {
                let name = world.resource::<MovedBinding>().0.clone();
                assert!(
                    world
                        .resource::<nova_input::prelude::InputBindings>()
                        .overrides()
                        .contains_key(name.as_str()),
                    "screenshot_menu: Cancel reset '{name}'"
                );
                let idle = world.resource::<IdleBox>().0;
                let now = rect_of(world, RESET_BUTTON);
                assert!(
                    rect_within(now, idle) && rect_within(idle, now),
                    "screenshot_menu: Reset Defaults is not back on its idle box: {now:?} vs {idle:?}"
                );
            })
            .add()
            .click_named(
                "arm Reset Defaults again",
                RESET_BUTTON,
                ui_node_present(RESET_CANCEL),
                BEAT_DEADLINE_SECS,
            )
            .click_named(
                "confirm Reset Defaults",
                RESET_BUTTON,
                node_gone(RESET_CANCEL),
                BEAT_DEADLINE_SECS,
            )
            .step("Confirm put the moved binding back on its default")
            .on_enter(|world: &mut World| {
                let name = world.resource::<MovedBinding>().0.clone();
                assert!(
                    !world
                        .resource::<nova_input::prelude::InputBindings>()
                        .overrides()
                        .contains_key(name.as_str()),
                    "screenshot_menu: Confirm left '{name}' off its default"
                );
            })
            .add()
    }

    fn uninstall_walk(self) -> Self {
        self.click_scrolled("open Mods", "Mods Button", MENU_BUTTONS)
            .step("the Mods list shows the fixture mod")
            .until(ui_node_present(FIXTURE_MOD_ROW))
            .diagnose(ui_node_diagnosis(FIXTURE_MOD_ROW))
            .deadline(STEP_DEADLINE_SECS)
            .add()
            .scroll_into_view("scroll to the fixture mod", FIXTURE_MOD_ROW, MODS_LIST)
            .click_named(
                "select the fixture mod",
                FIXTURE_MOD_ROW,
                ui_node_present(UNINSTALL_BUTTON),
                BEAT_DEADLINE_SECS,
            )
            .step("settle the mod details")
            .until(frames(SETTLE_FRAMES))
            .add()
            .step("record the idle Uninstall box")
            .on_enter(|world: &mut World| {
                let idle = rect_of(world, UNINSTALL_BUTTON);
                info!("screenshot_menu: idle Uninstall {idle:?}");
                world.insert_resource(IdleBox(idle));
            })
            .add()
            .click_named(
                "arm Uninstall",
                UNINSTALL_BUTTON,
                ui_node_present(UNINSTALL_CANCEL),
                BEAT_DEADLINE_SECS,
            )
            .step("settle the armed Uninstall")
            .until(frames(SETTLE_FRAMES))
            .add()
            .step("capture the armed Uninstall")
            .on_enter(capture_shot(UNINSTALL_ARMED_SHOT))
            .until(shot_written(UNINSTALL_ARMED_SHOT))
            .deadline(SHOT_DEADLINE_SECS)
            .add()
            .step("the armed Uninstall keeps its box and stays in view")
            .on_enter(|world: &mut World| {
                let idle = world.resource::<IdleBox>().0;
                assert_armed_row(
                    world,
                    "Uninstall",
                    idle,
                    UNINSTALL_CANCEL,
                    UNINSTALL_BUTTON,
                    UNINSTALL_PROMPT,
                    &[MOD_DETAILS_PANEL, MODS_PANEL],
                    idle.width(),
                );
            })
            .add()
            .click_idle_spot(
                "cancel Uninstall",
                UNINSTALL_CANCEL,
                node_gone(UNINSTALL_CANCEL),
            )
            .step("Cancel kept the mod installed")
            .on_enter(|world: &mut World| {
                assert!(
                    ui_node_rect(world, FIXTURE_MOD_ROW).is_some(),
                    "screenshot_menu: Cancel removed the fixture mod's row"
                );
                assert!(
                    fixture_mod_installed(),
                    "screenshot_menu: Cancel removed the fixture mod from the cache"
                );
            })
            .add()
            .click_named(
                "arm Uninstall again",
                UNINSTALL_BUTTON,
                ui_node_present(UNINSTALL_CANCEL),
                BEAT_DEADLINE_SECS,
            )
            .click_named(
                "confirm Uninstall",
                UNINSTALL_BUTTON,
                node_gone(FIXTURE_MOD_ROW),
                BEAT_DEADLINE_SECS,
            )
            .step("Confirm removed the mod from the list and the cache")
            .on_enter(|_world: &mut World| {
                assert!(
                    !fixture_mod_installed(),
                    "screenshot_menu: Confirm left the fixture mod in the cache"
                );
            })
            .add()
            .click("close Mods", "Mods Back Button")
    }
}

/// Whether the fixture mod still has a record in this run's mod cache.
#[cfg(feature = "debug")]
fn fixture_mod_installed() -> bool {
    nova_assets::mod_cache::prelude::read_index()
        .unwrap_or_default()
        .iter()
        .any(|record| record.id == FIXTURE_MOD_ID)
}

/// Check an armed idle/confirm row at the current layout.
///
/// `cancel` must cover the whole `idle` box, so a second click on the same spot
/// cancels. `confirm` must not sit on that box, and neither may be narrower
/// than the idle box (`confirm` against `confirm_min_width`). The `prompt`
/// sentence must be laid out, and none of the three may be clipped by any of
/// `clips`. Every rect is logged first, so a failed run still carries the
/// numbers.
#[cfg(feature = "debug")]
#[expect(
    clippy::too_many_arguments,
    reason = "one check naming the idle box, the three armed nodes and their clips"
)]
fn assert_armed_row(
    world: &World,
    what: &str,
    idle: Rect,
    cancel: &str,
    confirm: &str,
    prompt: &str,
    clips: &[&str],
    confirm_min_width: f32,
) {
    let cancel_rect = rect_of(world, cancel);
    let confirm_rect = rect_of(world, confirm);
    let prompt_rect = rect_of(world, prompt);
    let clip_rects: Vec<(&str, Rect)> = clips
        .iter()
        .map(|name| (*name, rect_of(world, name)))
        .collect();
    info!(
        "screenshot_menu: armed {what}: idle {idle:?}, Cancel {cancel_rect:?}, Confirm {confirm_rect:?}, prompt {prompt_rect:?}, clips {clip_rects:?}"
    );
    assert!(
        rect_within(cancel_rect, idle),
        "screenshot_menu: armed {what}: Cancel {cancel_rect:?} does not cover the idle box {idle:?}"
    );
    assert!(
        cancel_rect.width() >= idle.width() - RECT_SLACK
            && cancel_rect.height() >= idle.height() - RECT_SLACK,
        "screenshot_menu: armed {what}: Cancel {cancel_rect:?} is smaller than the idle box {idle:?}"
    );
    assert!(
        !rects_overlap(confirm_rect, idle),
        "screenshot_menu: armed {what}: Confirm {confirm_rect:?} sits on the idle box {idle:?}"
    );
    assert!(
        confirm_rect.width() >= confirm_min_width - RECT_SLACK && confirm_rect.height() > 0.0,
        "screenshot_menu: armed {what}: Confirm {confirm_rect:?} is narrower than {confirm_min_width}"
    );
    for (name, clip) in clip_rects {
        for (button, rect) in [
            ("Cancel", cancel_rect),
            ("Confirm", confirm_rect),
            ("the prompt", prompt_rect),
        ] {
            assert!(
                rect_within(clip, rect),
                "screenshot_menu: armed {what}: {button} {rect:?} is clipped by '{name}' {clip:?}"
            );
        }
    }
}

/// Whether a node called `name` exists.
#[cfg(feature = "debug")]
fn named_node(world: &mut World, name: &str) -> bool {
    let mut query = world.query::<&Name>();
    query.iter(world).any(|node| node.as_str() == name)
}

/// How many nodes are called `name`.
#[cfg(feature = "debug")]
fn named_nodes(world: &mut World, name: &str) -> usize {
    let mut query = world.query::<&Name>();
    query
        .iter(world)
        .filter(|node| node.as_str() == name)
        .count()
}

/// The text of every chart preview label shown, sorted.
#[cfg(feature = "debug")]
fn shown_chart_labels(world: &mut World) -> Vec<String> {
    let mut query = world.query::<(&Name, &InheritedVisibility)>();
    let mut labels: Vec<String> = query
        .iter(world)
        .filter(|(_, shown)| shown.get())
        .filter_map(|(name, _)| name.as_str().strip_prefix("Menu Chart Label "))
        .map(str::to_string)
        .collect();
    labels.sort();
    labels
}

/// The text of the one node called `name`.
#[cfg(feature = "debug")]
fn node_text(world: &mut World, name: &str) -> String {
    let mut query = world.query::<(&Name, &Text)>();
    query
        .iter(world)
        .find(|(node, _)| node.as_str() == name)
        .map(|(_, text)| text.0.clone())
        .unwrap_or_else(|| panic!("screenshot_menu: no text node '{name}'"))
}

/// The resolved text colour of the one node called `name`.
#[cfg(feature = "debug")]
fn node_text_color(world: &mut World, name: &str) -> Color {
    let mut query = world.query::<(&Name, &TextColor)>();
    query
        .iter(world)
        .find(|(node, _)| node.as_str() == name)
        .map(|(_, color)| color.0)
        .unwrap_or_else(|| panic!("screenshot_menu: no text node '{name}'"))
}

/// Advance once the seed field reads `seed`.
#[cfg(feature = "debug")]
fn seed_field_reads(seed: u32) -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    let seed = seed.to_string();
    std::sync::Arc::new(move |world: &World| {
        world
            .try_query::<(&Name, &TextFieldValue)>()
            .is_some_and(|mut fields| {
                fields
                    .iter(world)
                    .any(|(name, value)| name.as_str() == SEED_FIELD && value.0 == seed)
            })
    })
}

/// Bolt the backdrop camera to the gauntlet's warship.
///
/// An anchor rather than a fixed pose: the warship flies a patrol six
/// waypoints wide, so a pose solved at this instant is a picture of where the
/// ship USED to be by the time the PNG lands.
#[cfg(feature = "debug")]
fn frame_backdrop_ship(world: &mut World) {
    let ship = named_ship(world, BACKDROP_SHIP_ID).unwrap_or_else(|| {
        panic!("screenshot_menu: backdrop '{BACKDROP}' spawned no ship '{BACKDROP_SHIP_ID}'")
    });
    let camera = {
        let mut query = world.query_filtered::<Entity, With<ScenarioCameraMarker>>();
        query.iter(world).next()
    };
    let camera = camera.expect("screenshot_menu: the backdrop staged no scenario camera");
    world
        .entity_mut(camera)
        .remove::<WASDCameraController>()
        // The two overrides derive the same transform, so the backdrop's own
        // fixed pose has to go or it fights the anchor every frame.
        .remove::<ScriptedCameraPose>()
        .insert(ScriptedCameraAnchor {
            anchor: ship,
            offset: BACKDROP_EYE,
            frame: CameraOffsetFrame::Local,
            look_at: ScriptedCameraLookAt::Anchor,
        });
}

/// The spawned ship root carrying `id`.
#[cfg(feature = "debug")]
fn named_ship(world: &mut World, id: &str) -> Option<Entity> {
    let mut query = world.query_filtered::<(Entity, &EntityId), With<SpaceshipRootMarker>>();
    query
        .iter(world)
        .find(|(_, entity_id)| entity_id.0 == id)
        .map(|(entity, _)| entity)
}
