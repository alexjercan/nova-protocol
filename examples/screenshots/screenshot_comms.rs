//! screenshot_comms: the comms stack with one card per channel.
//!
//! Ships `news-0130-comms-channels.png`: the HUD over the ship's shoulder in
//! the Rock hollow with three cards up at once - a Range Control line on
//! `comms` (transmission blue, with its portrait), an engineering line on
//! `crew` (phosphor) and a guard-channel catch on `guard` (faint amber, tagged
//! GUARD). The lines go into the live scenario's story log the way a
//! `NarrativeCue` action would put them there, so the panel draws them through
//! its own queue and nothing here paints a card.
//!
//! Two run modes, both under the autopilot (`NOVA_AUTOPILOT`):
//! - `NOVA_AUTOPILOT=1` alone: the smoke path - reach Playing, drive the whole
//!   script, exit clean, capturing nothing.
//! - `NOVA_AUTOPILOT=1 NOVA_CAPTURE=1`: also capture the shot (staged under
//!   `NOVA_CAPTURE_DIR`).
//!
//! Capture (windowed, real GPU):
//! ```text
//! NOVA_CAPTURE_DIR=target/shots NOVA_AUTOPILOT=1 NOVA_CAPTURE=1 \
//!   cargo run --example screenshot_comms --features debug
//! ```
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//! ```text
//! NOVA_AUTOPILOT=1 cargo run --example screenshot_comms --features debug
//! # look for: `nova harness: reached Playing`, `autopilot: cycle complete, no panic`
//! ```

#[path = "shared/hollow.rs"]
mod hollow;

use bevy::prelude::*;
use clap::Parser;
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "screenshot_comms")]
#[command(version = "1.0.0")]
#[command(about = "Capture the comms stack with one card per channel over the ship's shoulder. Autopilot-only: the framing is a scripted camera pose", long_about = None)]
struct Cli;

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();
    let mut app = AppBuilder::new().with_game_plugins(custom_plugin).build();

    #[cfg(feature = "debug")]
    {
        // Probe wiring (inert without its NOVA_PROBE_* env): run timeline and
        // engine-bound invariants, no frame-time capture - one posed framing
        // has no steady-state window to measure.
        app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
        app.add_systems(Startup, (force_capture_resolution, hide_dev_overlays));
        app.add_plugins(comms_script());
        // Only under the script: a plain run is the owner flying this set, and
        // a pinned ship cannot be flown.
        if std::env::var_os("NOVA_AUTOPILOT").is_some() {
            app.add_systems(
                Update,
                hollow::pin_player.run_if(resource_exists::<hollow::HoldStation>),
            );
        }
    }

    app.run()
}

fn custom_plugin(app: &mut App) {
    app.add_systems(OnEnter(GameAssetsStates::Loaded), load_scene);
}

fn load_scene(
    mut commands: Commands,
    game_assets: Res<GameAssets>,
    sections: Res<GameSections>,
    ships: Res<GameShips>,
) {
    commands.trigger(LoadScenario(hollow::ambush_hollow(
        &game_assets,
        &sections,
        &ships,
    )));
}

/// The still the capture run writes.
#[cfg(feature = "debug")]
const COMMS_SHOT: &str = "news-0130-comms-channels.png";

/// The panel's root, by the name the HUD gives it; its children are the cards.
#[cfg(feature = "debug")]
const COMMS_PANEL_NAME: &str = "CommsPanelHUD";

/// Range Control's face, as the base bundle ships it: the merged path its
/// `self://portraits/range-control.png` rewrites to, because a cue pushed from
/// here never passes the merge that does the rewriting.
#[cfg(feature = "debug")]
const RANGE_CONTROL_PORTRAIT: &str = "base/portraits/range-control.png";

/// One line on each shipped channel, in the order the stack shows them: the
/// range's traffic first, the crew answering it, and what the guard receiver
/// picked up off the ridge underneath.
#[cfg(feature = "debug")]
const HAIL: [(&str, &str, &str, Option<&str>); 3] = [
    (
        "comms",
        "Range Control",
        "Cutter, Range Control. You are cleared into the hollow. Keep the guns cold until I call the lane.",
        Some(RANGE_CONTROL_PORTRAIT),
    ),
    (
        "crew",
        "Engineering",
        "Reactor is warm and the drive is yours. Say the word and we burn.",
        None,
    ),
    (
        "guard",
        "Ridge Two",
        "...two hulls on the shelf, hold until they commit to the lane...",
        None,
    ),
];

/// Put the three lines in the scenario's story log with the longest hold the
/// panel allows, so a slow software frame cannot age a card out before the
/// shot lands.
#[cfg(feature = "debug")]
fn hail_on_three_channels(world: &mut World) {
    let mut events = world.resource_mut::<NovaEventWorld>();
    for (channel, speaker, text, icon) in HAIL {
        events.push_narrative_cue(NarrativeCueActionConfig {
            channel: channel.to_string(),
            speaker: speaker.to_string(),
            text: text.to_string(),
            dwell: Some(COMMS_DWELL_MAX_SECS),
            icon: icon.map(AssetRef::from),
        });
    }
}

/// True once the panel has at least `count` cards spawned under it.
#[cfg(feature = "debug")]
fn cards_up(count: usize) -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(move |world: &World| {
        world
            .try_query::<(&Name, &Children)>()
            .is_some_and(|mut query| {
                query.iter(world).any(|(name, children)| {
                    name.as_str() == COMMS_PANEL_NAME && children.len() >= count
                })
            })
    })
}

/// Load the hollow, hold the ship on its station, hail on every channel, and
/// shoot the stack once the three cards have faded in.
#[cfg(feature = "debug")]
fn comms_script() -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
        .step("load the hollow")
        .enter(GameStates::Loading)
        .until(player_ship_present())
        .deadline(30.0)
        .add()
        // The raider stays parked: its flight gets no nudge, and the shot is
        // taken inside its engage grace, so the frame behind the cards is a
        // quiet range and not a fight.
        .step("settle the hollow")
        .on_enter(hollow::hold_station)
        .until(elapsed(0.5))
        .add()
        .step("frame the shoulder")
        .on_enter(|world| {
            hollow::hud_instrument(world);
            hollow::pose(
                world,
                Meters3::new(50.0, 16.0, 70.0),
                Meters3::new(0.0, 4.0, -140.0),
            );
        })
        .until(elapsed(0.4))
        .add()
        // The three cards promote together (the stack holds three) and fade
        // in over a quarter second; the hold past that is the fade.
        .step("hail on three channels")
        .on_enter(hail_on_three_channels)
        .until(and(cards_up(HAIL.len()), elapsed(0.6)))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("capture the comms stack")
        .on_enter(|world| shoot(world, COMMS_SHOT))
        .until(shot_written(COMMS_SHOT))
        .deadline(SHOT_DEADLINE_SECS)
        .add()
}
