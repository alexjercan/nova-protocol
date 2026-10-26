//! news_0160_settings_presets: the v0.16.0 news Settings still - Settings >
//! Graphics with the preset row lit on Custom after one lever moved and the
//! four levers under it.
//!
//! The walk drives the real menu under the harness pointer: Settings, the
//! Graphics tab, then a click on [`MOVED_LEVER`]. A fresh profile opens on
//! the High preset, so the click is what takes the budget off every tier and
//! lights Custom; nothing writes the budget or the selection directly. UI
//! scale and Invert Y live on the Interface and Controls tabs, so this frame
//! does not show them. The Window, VSync and Focus rows stand below the
//! fixed-height panel's fold at 1080p and are reached by scrolling, so the
//! still shows the top of the tab only.
//!
//! Two run modes, both under the autopilot (`NOVA_AUTOPILOT`):
//! - `NOVA_AUTOPILOT=1` alone: the smoke path - the whole walk with its
//!   asserts, capturing nothing.
//! - `NOVA_AUTOPILOT=1 NOVA_CAPTURE=1`: also write
//!   `news-0160-settings-presets.png` (staged under `NOVA_CAPTURE_DIR`).
//!
//! ```text
//! NOVA_CAPTURE_DIR=target/news-0160-shots NOVA_AUTOPILOT=1 NOVA_CAPTURE=1 \
//!   cargo run --example news_0160_settings_presets --features debug
//! ```

use bevy::prelude::*;
use clap::Parser;
use nova_protocol::prelude::*;
use nova_ui::widget::Selected;

// The pointer gestures, shared with the other menu walks.
#[path = "shared/ui_walk.rs"]
mod ui_walk;
use ui_walk::{hide_menu_version, Gestures};

#[derive(Parser)]
#[command(name = "news_0160_settings_presets")]
#[command(version = "1.0.0")]
#[command(
    about = "Capture the v0.16.0 Settings > Graphics still: Custom lit after one lever moved",
    long_about = None
)]
struct Cli;

/// The backdrop behind the Settings panel, pinned: menu entry draws one of
/// the shipped backdrops at random.
const BACKDROP: &str = "menu_gauntlet";

/// The still this producer writes.
const SHOT: &str = "news-0160-settings-presets.png";

/// The preset a fresh profile opens on, and the Custom option the moved
/// lever must light instead.
const OPENING_PRESET: &str = "Graphics High";
const CUSTOM_PRESET: &str = "Graphics Custom";

/// The one lever the walk moves. Shake, because it is off every tier only
/// with the other three on High's values, and it changes nothing in the
/// still's own pixels.
const MOVED_LEVER: &str = "Camera Shake Off";

/// The scrolling body of the Settings panel: every row the still shows must
/// stand inside it, not scrolled under its edge.
const SETTINGS_BODY: &str = "Settings Body";

/// The rows the still shows: the preset row and its four levers.
const SHOWN_ROWS: [&str; 5] = [
    "Graphics Row",
    "Particles Row",
    "Render Scale Row",
    "Flash Lights Row",
    "Camera Shake Row",
];

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();

    // This run's own settings: a player's saved budget would open the tab
    // on another preset.
    std::env::set_var(
        nova_assets::storage::CONFIG_ROOT_ENV,
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(format!(
            "target/example-profiles/{}-{}-{}",
            env!("CARGO_CRATE_NAME"),
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("the clock is after 1970")
                .as_nanos(),
        )),
    );
    std::env::set_var(MENU_BACKDROP_ENV, BACKDROP);

    let mut app = editor_app(true, None);
    app.insert_resource(bevy::ecs::error::FallbackErrorHandler(
        bevy::ecs::error::panic,
    ));
    app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
    app.add_systems(Startup, (force_capture_resolution, hide_dev_overlays));
    app.add_plugins(settings_script());
    app.run()
}

fn when(
    check: impl Fn(&World) -> bool + Send + Sync + 'static,
) -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(check)
}

/// Whether the node named `name` carries [`Selected`].
///
/// # Panics
///
/// When no node is named `name`: the tab lost the option.
fn selected(world: &World, name: &str) -> bool {
    world
        .try_query::<(&Name, Has<Selected>)>()
        .and_then(|mut nodes| {
            nodes
                .iter(world)
                .find(|(node, _)| node.as_str() == name)
                .map(|(_, selected)| selected)
        })
        .unwrap_or_else(|| panic!("settings presets: the Graphics tab has no '{name}'"))
}

/// Assert every row of [`SHOWN_ROWS`] stands whole inside the Settings body.
fn assert_rows_in_view(world: &mut World) {
    let body = ui_node_rect(world, SETTINGS_BODY)
        .unwrap_or_else(|| panic!("settings presets: no laid-out '{SETTINGS_BODY}'"));
    for row in SHOWN_ROWS {
        let rect = ui_node_rect(world, row)
            .unwrap_or_else(|| panic!("settings presets: no laid-out, visible '{row}'"));
        assert!(
            body.contains(rect.min) && body.contains(rect.max),
            "settings presets: '{row}' at {rect:?} is not inside the Settings body {body:?}"
        );
    }
}

/// The walk: Settings, Graphics, one lever, the still.
fn settings_script() -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
        .step("reach the main menu")
        .enter(GameStates::Loading)
        .until(state_is(GameStates::MainMenu))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("settle the menu and its backdrop")
        .until(frames(SETTLE_FRAMES))
        .add()
        .click("open Settings", "Settings Button")
        .step("the Settings panel is up")
        .until(ui_node_present(SETTINGS_BODY))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .click("open the Graphics tab", "Settings Tab: Graphics")
        .step("the Graphics tab stands")
        .until(ui_node_present(MOVED_LEVER))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("a fresh profile opens on High")
        .on_enter(|world: &mut World| {
            assert!(
                selected(world, OPENING_PRESET),
                "settings presets: a fresh profile must open on '{OPENING_PRESET}'"
            );
            assert!(
                !selected(world, CUSTOM_PRESET),
                "settings presets: '{CUSTOM_PRESET}' is lit before any lever moved"
            );
        })
        .add()
        .click("move the camera shake lever", MOVED_LEVER)
        .step("the preset row turns to Custom")
        .until(when(|world| selected(world, CUSTOM_PRESET)))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("check the budget, the rows and the view")
        .on_enter(|world: &mut World| {
            let budget = *world.resource::<GraphicsBudget>();
            assert!(!budget.shake, "settings presets: the lever left shake on");
            assert_eq!(
                budget.preset(),
                None,
                "settings presets: the moved budget still matches a tier"
            );
            for tier in GraphicsQuality::ALL {
                let name = format!("Graphics {}", tier.label());
                assert!(
                    !selected(world, &name),
                    "settings presets: '{name}' is lit beside Custom"
                );
            }
            assert!(
                selected(world, MOVED_LEVER),
                "settings presets: the moved lever does not read '{MOVED_LEVER}'"
            );
            assert_rows_in_view(world);
            hide_dev_overlays(world);
        })
        .add()
        // Off the controls, so no hover tint sits on a button in the still.
        .step("rest the pointer on the panel title")
        .on_enter(hover_named("Settings Title"))
        .until(frames(SETTLE_FRAMES))
        .add()
        // The news figure ships with the post, so the footer's build id stays
        // out of it.
        .step("capture the Graphics tab")
        .on_enter(|world: &mut World| {
            hide_hud(world);
            hide_menu_version(world);
            shoot(world, SHOT);
        })
        .until(shot_written(SHOT))
        .deadline(SHOT_DEADLINE_SECS)
        .add()
}
