//! screenshot_ship_repair: the 0.15.0 news still of a hull plate repair
//! drafted on the Ship pane (`news-0150-ship-repair.png`).
//!
//! It boots the interface range from `shared/computer.rs`, knocks the player
//! hull down and stocks hull plates, then opens the interface with the real
//! keys: TAB, the next-pane key onto Ship, and one `viewer_next` from `CTL-1`
//! onto `HULL-1`. It clicks the form's All and shoots the panel BEFORE Repair
//! is pressed, so the frame shows the draft: the condition bar, the plate
//! count, the slider and the integrity the repair will reach.
//!
//! ## Why the damage is written in
//!
//! Nothing on the range shoots, so the hull is hurt by writing `Health`
//! directly, as `lesson_interface_ship` hurts its turret. That is staging: the
//! still claims what the repair form shows, not how the damage arrived. The
//! range ship carries no stock, so the same step fills its hold with plates.
//!
//! ## What the walk checks before the shot
//!
//! The panel title names `HULL-1`, the preview reads full integrity, and the
//! one visible slider holds the All count. All must sit strictly between one
//! plate and the whole stock, so the frame shows a computed count rather than
//! either bound. The hull and the stock are unchanged: nothing was submitted.
//!
//! Two run modes, both under the autopilot (`NOVA_AUTOPILOT`):
//! - `NOVA_AUTOPILOT=1` alone: the smoke path - draft the repair, run the
//!   checks, exit clean, capturing nothing.
//! - `NOVA_AUTOPILOT=1 NOVA_CAPTURE=1`: also write the PNG (staged under
//!   `NOVA_CAPTURE_DIR`).
//!
//! Capture (windowed, real GPU):
//! ```text
//! NOVA_CAPTURE_DIR=target/shots NOVA_AUTOPILOT=1 NOVA_CAPTURE=1 \
//!   cargo run --example screenshot_ship_repair --features debug
//! ```

#[path = "shared/computer.rs"]
mod computer;

use bevy::prelude::*;
#[cfg(feature = "debug")]
use bevy::ui_widgets::{SliderRange, SliderValue};
use clap::Parser;
use computer::interface_range;
#[cfg(feature = "debug")]
use nova_protocol::nova_interface::pane::InterfacePaneType;
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "screenshot_ship_repair")]
#[command(version = "1.0.0")]
#[command(about = "Capture a hull plate repair drafted on the Ship pane", long_about = None)]
struct Cli;

/// The still the walk writes.
#[cfg(feature = "debug")]
const REPAIR_SHOT: &str = "news-0150-ship-repair.png";

/// The fraction of its integrity the hull keeps: `degraded` on the status
/// line, and several plates short of full.
#[cfg(feature = "debug")]
const HURT_INTEGRITY: f32 = 0.35;

/// Hull plates the hold carries: more than the repair needs, so All is a
/// computed count and not the whole stock.
#[cfg(feature = "debug")]
const STOCKED_PLATES: u32 = 10;

/// The panel title's prefix once the selection is on the hull.
#[cfg(feature = "debug")]
const HULL_TITLE: &str = "HULL-1  ";

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();
    let mut app = AppBuilder::new().with_game_plugins(custom_plugin).build();

    #[cfg(feature = "debug")]
    {
        // Run timeline and engine invariants only: a posed still holds no
        // steady-state load worth a frame-time claim.
        app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
        app.add_plugins(repair_script());
        app.add_systems(Startup, (force_capture_resolution, hide_dev_overlays));
    }

    app.run()
}

fn custom_plugin(app: &mut App) {
    app.add_systems(OnEnter(GameAssetsStates::Loaded), setup_range);
}

fn setup_range(mut commands: Commands, game_assets: Res<GameAssets>, sections: Res<GameSections>) {
    commands.trigger(LoadScenario(interface_range(&game_assets, &sections)));
}

/// The player hull's `(current, max)` Health, once exactly one exists.
#[cfg(feature = "debug")]
fn player_hull(world: &World) -> Option<(f32, f32)> {
    let mut hulls = world.try_query_filtered::<(&Health, &ChildOf), With<HullSectionMarker>>()?;
    let mut player = hulls.iter(world).filter(|(_, parent)| {
        world
            .get::<PlayerSpaceshipMarker>(parent.parent())
            .is_some()
    });
    let (health, _) = player.next()?;
    player
        .next()
        .is_none()
        .then_some((health.current, health.max))
}

/// Knock the player hull down and stock the plates the repair drafts against.
#[cfg(feature = "debug")]
fn hurt_the_hull(world: &mut World) {
    let mut ships = world.query_filtered::<Entity, With<PlayerSpaceshipMarker>>();
    let player = ships
        .single(world)
        .expect("screenshot_ship_repair: one player ship");
    let mut hulls = world.query_filtered::<(&mut Health, &ChildOf), With<HullSectionMarker>>();
    let mut hurt = 0;
    for (mut health, parent) in hulls.iter_mut(world) {
        if parent.parent() == player {
            health.current = health.max * HURT_INTEGRITY;
            hurt += 1;
        }
    }
    assert_eq!(
        hurt, 1,
        "screenshot_ship_repair: the player ship has one hull"
    );
    let items = world.resource::<GameItems>().clone();
    let mut inventory = world
        .get_mut::<ShipInventory>(player)
        .expect("screenshot_ship_repair: the player ship has a ShipInventory");
    *inventory = ShipInventory::new(
        &items,
        inventory.capacity_g(),
        [(ITEM_HULL_PLATE.into(), STOCKED_PLATES)],
    );
}

/// The plates All drafts for the hurt hull.
#[cfg(feature = "debug")]
fn all_plates(current: f32, max: f32) -> u32 {
    STOCKED_PLATES.min(((max - current) / HULL_PLATE_HEALTH).ceil() as u32)
}

/// Visible, laid-out UI texts that pass `test`, as a player sees them.
#[cfg(feature = "debug")]
fn visible_texts(world: &World, test: impl Fn(&str) -> bool) -> usize {
    world
        .try_query::<(&Text, &ComputedNode, &InheritedVisibility)>()
        .map_or(0, |mut texts| {
            texts
                .iter(world)
                .filter(|(text, node, visible)| {
                    visible.get() && node.size().x > 0.0 && test(&text.0)
                })
                .count()
        })
}

/// Advance once the panel title names the hull.
#[cfg(feature = "debug")]
fn the_hull_is_selected() -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(|world: &World| {
        visible_texts(world, |text| text.starts_with(HULL_TITLE)) == 1
    })
}

/// The preview line a draft that fills the hurt hull reads.
#[cfg(feature = "debug")]
fn full_preview(max: f32) -> String {
    format!(
        "Predicted integrity: {max:.0}/{max:.0} HP\n1 hull plate restores up to {HULL_PLATE_HEALTH:.0} HP."
    )
}

/// Advance once the form previews a repair to full integrity.
#[cfg(feature = "debug")]
fn the_preview_reads_full() -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(|world: &World| {
        player_hull(world).is_some_and(|(_, max)| {
            let preview = full_preview(max);
            visible_texts(world, |text| text == preview) == 1
        })
    })
}

/// Advance once the interface is open on `pane`.
#[cfg(feature = "debug")]
fn the_interface_shows(
    pane: InterfacePaneType,
) -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(move |world: &World| {
        world
            .get_resource::<State<PauseStates>>()
            .is_some_and(|pause| *pause.get() == PauseStates::Interface)
            && world
                .get_resource::<InterfacePaneType>()
                .is_some_and(|shown| *shown == pane)
    })
}

/// Assert the drafted repair the still shows: All between one plate and the
/// whole stock, the slider and stock line on it, and nothing submitted.
#[cfg(feature = "debug")]
fn check_the_draft(world: &mut World) {
    let (current, max) = player_hull(world).expect("screenshot_ship_repair: one player hull");
    assert_eq!(
        current,
        max * HURT_INTEGRITY,
        "screenshot_ship_repair: the hull changed before Repair"
    );
    let all = all_plates(current, max);
    assert!(
        1 < all && all < STOCKED_PLATES,
        "screenshot_ship_repair: All drafts {all} plates, not strictly between 1 and {STOCKED_PLATES}"
    );
    let mut ships = world.query_filtered::<&ShipInventory, With<PlayerSpaceshipMarker>>();
    let stock = ships
        .single(world)
        .expect("screenshot_ship_repair: one player inventory")
        .count(&ITEM_HULL_PLATE.into());
    assert_eq!(
        stock, STOCKED_PLATES,
        "screenshot_ship_repair: plates spent before Repair"
    );
    let stock_line = format!("{STOCKED_PLATES} plates in stock");
    assert_eq!(
        visible_texts(world, |text| text == stock_line),
        1,
        "screenshot_ship_repair: the form does not show `{stock_line}`"
    );
    let mut sliders = world.query::<(
        &SliderValue,
        &SliderRange,
        &ComputedNode,
        &InheritedVisibility,
    )>();
    let shown: Vec<_> = sliders
        .iter(world)
        .filter(|(_, _, node, visible)| visible.get() && node.size().x > 0.0)
        .map(|(value, range, _, _)| (value.0, range.end()))
        .collect();
    assert_eq!(
        shown,
        [(all as f32, all as f32)],
        "screenshot_ship_repair: the repair slider is not at All"
    );
}

/// The walk: hurt the hull, open the Ship pane on it, draft All, check the
/// draft and shoot it.
#[cfg(feature = "debug")]
fn repair_script() -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
        .step("load the range")
        .enter(GameStates::Loading)
        .until(player_ship_present())
        .deadline(30.0)
        .add()
        // The HUD spawns after the ship, and dropping the status bar before it
        // exists drops nothing.
        .step("settle the range")
        .until(elapsed(2.0))
        .add()
        // The status bar carries the build's commit and the frame rate, and it
        // draws OVER the interface.
        .step("drop the status bar and hurt the hull")
        .on_enter(|world: &mut World| {
            hide_status_bar(world);
            hurt_the_hull(world);
        })
        .until(std::sync::Arc::new(|world: &World| {
            player_hull(world).is_some_and(|(current, max)| current < max)
        }))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("press the interface key")
        .on_enter(press_action("interface_toggle"))
        .until(frames(1))
        .add()
        .step("let the key up and wait for the interface")
        .on_enter(release_action("interface_toggle"))
        .until(the_interface_shows(InterfacePaneType::Map))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("press the next-pane key")
        .on_enter(press_action("interface_next_tab"))
        .until(frames(1))
        .add()
        .step("let the key up and wait for the Ship pane")
        .on_enter(release_action("interface_next_tab"))
        .until(the_interface_shows(InterfacePaneType::Ship))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        // Showing the pane is not being STILL: its offscreen schematic builds
        // and settles over frames, which is render work.
        .step("settle the schematic")
        .until(frames(SETTLE_FRAMES))
        .add()
        // `ShipSections::collect()` sorts by code and the pane opens on the
        // first, `CTL-1`; one step lands on `HULL-1`.
        .step("step the selection onto the hull")
        .on_enter(press_action("viewer_next"))
        .until(frames(1))
        .add()
        .step("let the select key up and wait for the hull")
        .on_enter(release_action("viewer_next"))
        .until(the_hull_is_selected())
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .click_named(
            "draft all",
            "ShipRepairAll",
            the_preview_reads_full(),
            STEP_DEADLINE_SECS,
        )
        .step("check the draft")
        .on_enter(check_the_draft)
        .until(frames(1))
        .add()
        .step("settle the panel for the shot")
        .until(frames(SETTLE_FRAMES))
        .add()
        .step("capture the drafted repair")
        .on_enter(|world: &mut World| shoot(world, REPAIR_SHOT))
        .until(shot_written(REPAIR_SHOT))
        .deadline(SHOT_DEADLINE_SECS)
        .add()
}
