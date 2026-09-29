//! lesson_interface_ship: the two TAB interface demonstrations that are about
//! the SHIP pane's inspector - `interface_ship_service` (repair and reload
//! acting on the selected section) and `interface_rebind_section` (arming the
//! rebind and giving the section a new trigger).
//!
//! One producer, two sheets, because they are one session at one keyboard: the
//! interface comes up once, M switches to the Ship pane, and both
//! demonstrations are photographed off the same inspector panel.
//!
//! ## Both are ACTION loops
//!
//! Both lessons were authored as STILLS, and a still cannot make either claim.
//! "Repair restores its integrity" photographed as a still is a picture of a
//! meter at some reading, with nothing saying what moved it; "the next key
//! takes over its trigger" is a picture of an amber line asking for a key that
//! never arrives. So both are loops
//! (`crates/nova_authoring/src/base_content/lessons.rs`) and their alt text
//! describes the footage.
//!
//! The camera never moves, and there is no [`LessonSweep`](lesson::LessonSweep)
//! here at all: the interface freezes the game (`PauseStates::Interface` stops
//! `Time<Virtual>` and `Time<Physics>`) and hands the window to a themed pane
//! that does not care where the world camera stands. What moves in the cell is
//! the PANEL - the meter filling, the ammo count refilling, the amber prompt
//! arriving and the binding line taking a new key.
//!
//! ## Why the turret is the section both sheets sit on
//!
//! `PDC-1` is the only section on the range that can take all three verbs: it
//! carries integrity to restore, a magazine to refill, and a trigger to
//! rebind. Repairing the hull and reloading the turret would have cost the
//! sheet a reselection in the middle of its twenty cells, and the lesson's own
//! sentence puts both verbs on ONE selected section.
//!
//! The selection is reached with two `viewer_next` presses, because
//! `ShipSections::collect()` sorts by code and the pane opens with the first
//! selected - `CTL-1`, `HULL-1`, `PDC-1`. Nothing observable says which section
//! is selected (`ShipRuntime`'s selection is crate-private), so the walk does
//! not assert on the selection: it asserts on the RESULT. The turret is the
//! only damaged section on the range, so a repair that fills its meter is proof
//! the selection landed on it, and a walk that cycled wrong stalls on a named
//! beat instead of shipping a sheet of the wrong panel.
//!
//! ## Why the damage is written in
//!
//! Nothing on the range shoots, so the turret is hurt and its magazine spent
//! by writing `Health` and `SectionAmmo` directly before the sheet opens. That
//! is staging, the same kind as posing a camera: the lesson claims what REPAIR
//! does, not how the damage arrived. The range ship carries no stock, so the
//! same step gives it the open world's 12 hull plates for the repair to spend.
//! The range's turret also carries the trigger a player ship's turret carries
//! (`shared/computer.rs`), because a section with no binding cannot be rebound
//! - the pane disables the button.
//!
//! ## Why the interface is closed and reopened between the two sheets
//!
//! The panel's note line is a transient (`ShipRuntime.note`), and it counts
//! down on real time. An armed capture pins that clock at one cell per frame,
//! so a 2.5 s note outlives a twenty-cell sheet, and the rebind sheet would
//! have opened with `reloaded PDC-1: ammo 8/8` still sitting under the
//! buttons. Leaving the pane clears the note, the selection and any armed
//! rebind (`manage_ship_scene`'s teardown), so the walk closes the interface
//! with TAB, reopens it - TAB reopens the last pane, which is Ship - and
//! re-cycles. The second sheet then opens on a panel that says only what it
//! is about.
//!
//! ## What did not work
//!
//! A key that goes down and never comes up. The rebind capture will not take a
//! key until it has seen a frame with EVERY source up (`rebind_awaiting_release`,
//! so the key that armed the capture is not the key it takes), and a held key is
//! a source that is never up. Every press in this walk is `press_action` or
//! `press_key` with its own `release_*` beat for that reason; the walk sat on
//! the armed prompt for a whole sheet the one time a press was left down, and
//! because an armed capture outranks the note line, the panel showed nothing to
//! say why.
//!
//! Two run modes, both under the autopilot (`NOVA_AUTOPILOT`):
//! - `NOVA_AUTOPILOT=1` alone: the smoke path - drive the whole session, exit
//!   clean, recording nothing.
//! - `NOVA_AUTOPILOT=1 NOVA_CAPTURE=1`: also tile the two sheets (staged under
//!   `NOVA_CAPTURE_DIR`).
//!
//! Capture (windowed, real GPU):
//! ```text
//! NOVA_CAPTURE_DIR=target/lesson-shots NOVA_AUTOPILOT=1 NOVA_CAPTURE=1 \
//!   cargo run --example lesson_interface_ship --features debug
//! ```

#[path = "shared/computer.rs"]
mod computer;
#[cfg(feature = "debug")]
#[path = "shared/lesson.rs"]
mod lesson;

use bevy::prelude::*;
use clap::Parser;
use computer::interface_range;
#[cfg(feature = "debug")]
use lesson::{lesson_profile, LESSON_GRID};
#[cfg(feature = "debug")]
use nova_input::prelude::InputSource;
#[cfg(feature = "debug")]
use nova_protocol::nova_interface::{pane::InterfacePaneType, prelude::ShipRuntime};
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "lesson_interface_ship")]
#[command(version = "1.0.0")]
#[command(about = "Record the handbook's Ship pane demonstrations", long_about = None)]
struct Cli;

/// The sheet for "Repair and reload a section".
#[cfg(feature = "debug")]
const SERVICE_LESSON: &str = "interface_ship_service";
/// The sheet for "Rebinding a section".
#[cfg(feature = "debug")]
const REBIND_LESSON: &str = "interface_rebind_section";

/// Cells each sheet opens on before the first key goes down.
///
/// A step's entry takes a frame of its own and `frames(n)` is polled from the
/// next one, so this is four cells: long enough to read the panel the action
/// is about to change, short enough to leave the change most of the sheet.
#[cfg(feature = "debug")]
const LEAD_CELLS: u32 = 3;

/// Cells the repaired panel holds before the reload key goes down.
#[cfg(feature = "debug")]
const REPAIR_CELLS: u32 = 4;

/// Cells the armed rebind prompt holds before the new key arrives.
///
/// The longest hold in either sheet, because this is the cell the lesson is
/// about: `PRESS A KEY OR MOUSE BUTTON - ESC CANCELS` in amber across the
/// panel, with nothing else moving.
#[cfg(feature = "debug")]
const ARMED_CELLS: u32 = 5;

/// How far the turret's integrity is knocked down before the sheet opens.
///
/// A third, not a sliver: the meter is ten ASCII cells wide
/// (`ShipSectionView::meter`), so this reads as `[###-------]` against the
/// `[##########]` the repair leaves, and the status line reads `degraded`
/// rather than `critical` - a section worth repairing, not a wreck.
#[cfg(feature = "debug")]
const HURT_INTEGRITY: f32 = 0.35;

/// Rounds left in the turret's magazine before the sheet opens. Not zero: a
/// spent magazine and an empty one read the same in the panel, and one round
/// left is the honest state of a gun that has been firing.
#[cfg(feature = "debug")]
const SPENT_ROUNDS: u32 = 1;

/// The key the rebind sheet gives the turret.
///
/// Free of the flight rig, which is what the panel refuses by name
/// (`reserved_conflict` in `nova_interface/src/ship/rebind.rs`): the helm holds W,
/// Space, X, G, O, Z and D, and the interface holds Tab, M, the brackets, WASD,
/// QERF, T, G, L, P and B. Nothing holds K.
#[cfg(feature = "debug")]
const REBIND_KEY: KeyCode = KeyCode::KeyK;

/// How many times the walk presses `viewer_next` to reach the turret.
#[cfg(feature = "debug")]
const STEPS_TO_THE_TURRET: usize = 2;

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();
    let mut app = AppBuilder::new().with_game_plugins(custom_plugin).build();

    #[cfg(feature = "debug")]
    {
        app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
        app.add_plugins(nova_protocol::nova_debug::harness::LoopCapturePlugin::new(
            lesson_profile(),
        ));
        app.add_systems(Startup, (force_capture_resolution, hide_dev_overlays));
        app.add_plugins(ship_script());
        // The range is parked and the interface is the subject, so nothing in it
        // may drift between the two sheets - but only on a capture run, so a
        // plain `cargo run` is the range as it flies.
        app.add_systems(Update, freeze_bodies.run_if(capturing));
    }

    app.run()
}

fn custom_plugin(app: &mut App) {
    app.add_systems(OnEnter(GameAssetsStates::Loaded), setup_range);
}

fn setup_range(mut commands: Commands, game_assets: Res<GameAssets>, sections: Res<GameSections>) {
    commands.trigger(LoadScenario(interface_range(&game_assets, &sections)));
}

/// Knock the turret's integrity down, spend its magazine, and stock the hull
/// plates the repair spends.
#[cfg(feature = "debug")]
fn hurt_the_turret(world: &mut World) {
    let mut turrets =
        world.query_filtered::<(&mut Health, &mut SectionAmmo), With<TurretSectionMarker>>();
    for (mut health, mut ammo) in turrets.iter_mut(world) {
        health.current = health.max * HURT_INTEGRITY;
        ammo.rounds = SPENT_ROUNDS.min(ammo.capacity);
    }
    let mut ships = world.query_filtered::<&mut ShipInventory, With<PlayerSpaceshipMarker>>();
    for mut inventory in ships.iter_mut(world) {
        *inventory = ShipInventory::new(inventory.capacity_kg(), [(ItemType::HullPlate, 12)]);
    }
}

/// A predicate over the range's one turret section.
#[cfg(feature = "debug")]
fn the_turret_reads(
    test: fn(&Health, &SectionAmmo) -> bool,
) -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(move |world: &World| {
        world
            .try_query_filtered::<(&Health, &SectionAmmo), With<TurretSectionMarker>>()
            .is_some_and(|mut turrets| turrets.iter(world).any(|(health, ammo)| test(health, ammo)))
    })
}

/// Advance once the range's turret carries a keyboard or pad trigger at all.
///
/// The precondition the rebind sheet stands on: the Ship pane reads the binding
/// component to decide whether its rebind button is live, so a turret that
/// spawned without one would arm nothing and the walk would fail four beats
/// later on a capture that never happened.
#[cfg(feature = "debug")]
fn the_turret_carries_a_trigger() -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(|world: &World| {
        world
            .try_query_filtered::<&SpaceshipTurretInputBinding, With<TurretSectionMarker>>()
            .is_some_and(|mut turrets| turrets.iter(world).any(|binding| !binding.0.is_empty()))
    })
}

/// Advance once the turret carries the key the rebind sheet pressed.
///
/// The component, not the panel text: the capture writes
/// `SpaceshipTurretInputBinding` through the shared rebind seam, and a line on
/// a panel is what that looks like rather than what it IS. The pad half of the
/// trigger is left alone by a desk capture, so this asks whether the new key
/// is IN the binding rather than whether it is the whole of it.
#[cfg(feature = "debug")]
fn the_turret_answers_to_the_new_key(
) -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(|world: &World| {
        world
            .try_query_filtered::<&SpaceshipTurretInputBinding, With<TurretSectionMarker>>()
            .is_some_and(|mut turrets| {
                turrets
                    .iter(world)
                    .any(|binding| binding.0.contains(&InputSource::Keyboard(REBIND_KEY)))
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

/// Advance once the Ship pane is waiting for a key to bind.
///
/// The pane's own record that the rebind is armed, not a frame count: the
/// arming key has to be seen UP before the capture takes anything, so the
/// amber prompt is the one thing the walk may not press the next key over.
#[cfg(feature = "debug")]
fn the_rebind_is_armed() -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    resource_where::<ShipRuntime>(ShipRuntime::rebind_armed)
}

/// Bring the interface up on the Ship pane and cycle the selection onto the
/// turret.
///
/// Both sheets need the same opening, and the second one needs it twice - the
/// interface is closed between them to clear the panel (see the module docs).
/// The first open lands on Map and M switches over; TAB reopens the last pane,
/// so every later open lands on Ship already.
#[cfg(feature = "debug")]
fn open_the_ship_pane(
    script: nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates>,
    first_open: bool,
) -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    let landing = if first_open {
        InterfacePaneType::Map
    } else {
        InterfacePaneType::Ship
    };
    let mut script = script
        .step("press the interface key")
        .on_enter(press_action("interface_toggle"))
        .until(frames(1))
        .add()
        .step("let the key up and wait for the interface")
        .on_enter(release_action("interface_toggle"))
        .until(the_interface_shows(landing))
        .deadline(STEP_DEADLINE_SECS)
        .add();
    if first_open {
        script = script
            .step("press the next-pane key")
            .on_enter(press_action("interface_next_tab"))
            .until(frames(1))
            .add()
            .step("let the key up and wait for the Ship pane")
            .on_enter(release_action("interface_next_tab"))
            .until(the_interface_shows(InterfacePaneType::Ship))
            .deadline(STEP_DEADLINE_SECS)
            .add();
    }
    // Showing the pane is not being STILL: its offscreen schematic builds and
    // settles over frames, which is render work.
    script = script
        .step("settle the schematic")
        .until(frames(SETTLE_FRAMES))
        .add();
    for _ in 0..STEPS_TO_THE_TURRET {
        script = script
            .step("step the selection on")
            .on_enter(press_action("viewer_next"))
            .until(frames(1))
            .add()
            .step("let the select key up")
            .on_enter(release_action("viewer_next"))
            .until(frames(1))
            .add();
    }
    script
}

/// Bring the interface up on the Ship pane, service the turret, then give it a
/// new trigger.
#[cfg(feature = "debug")]
fn ship_script() -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    let script = nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
        .step("load the range")
        .enter(GameStates::Loading)
        .until(player_ship_present())
        .deadline(30.0)
        .add()
        .step("settle the range")
        .until(elapsed(2.0))
        .add()
        .step("the range's turret carries a trigger to replace")
        .until(the_turret_carries_a_trigger())
        .deadline(STEP_DEADLINE_SECS)
        .add()
        // The status bar carries the build's own commit and the capture rig's
        // frame rate, and it draws OVER the interface. Dropped before the key,
        // not after, so no recorded cell ever holds it.
        .step("drop the status bar")
        .on_enter(hide_status_bar)
        .until(frames(1))
        .add();

    let script = open_the_ship_pane(script, true);

    let script = script
        // REPAIR AND RELOAD. The damage goes in with the pane already up, so
        // the panel shows it arrive rather than opening on it - and so nothing
        // in the scenario's own two seconds of flight can have refilled the
        // magazine before the sheet starts.
        .step("spend the turret's magazine and knock its integrity down")
        .on_enter(hurt_the_turret)
        .until(the_turret_reads(|health, ammo| {
            health.current < health.max && ammo.rounds < ammo.capacity
        }))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("open the service sheet on the damaged panel")
        .on_enter(|world: &mut World| sheet_start(world, SERVICE_LESSON, LESSON_GRID))
        .until(frames(LEAD_CELLS))
        .add()
        .step("press repair")
        .on_enter(press_action("ship_repair"))
        .until(frames(1))
        .add()
        // The honest end of "repair restores its integrity": the section's own
        // health, not a frame count that would have passed either way.
        .step("let the key up and watch the meter fill")
        .on_enter(release_action("ship_repair"))
        .until(and(
            the_turret_reads(|health, _| health.current >= health.max),
            frames(REPAIR_CELLS),
        ))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("press reload")
        .on_enter(press_action("ship_reload"))
        .until(frames(1))
        .add()
        .step("let the key up and hold the refilled magazine")
        .on_enter(release_action("ship_reload"))
        .until(sheet_written(SERVICE_LESSON))
        .deadline(60.0)
        .add()
        .step("the magazine came back full")
        .until(the_turret_reads(|_, ammo| ammo.rounds == ammo.capacity))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        // REBINDING. Out of the interface and back into it, which is what
        // clears the note line the service sheet left under the buttons.
        .step("close the interface")
        .on_enter(press_action("interface_toggle"))
        .until(frames(1))
        .add()
        .step("let the key up and wait for the flight")
        .on_enter(release_action("interface_toggle"))
        .until(resource_where::<State<PauseStates>>(|pause| {
            *pause.get() == PauseStates::Unpaused
        }))
        .deadline(STEP_DEADLINE_SECS)
        .add();

    let script = open_the_ship_pane(script, false);

    script
        .step("open the rebind sheet on the section's current trigger")
        .on_enter(|world: &mut World| sheet_start(world, REBIND_LESSON, LESSON_GRID))
        .until(frames(LEAD_CELLS))
        .add()
        .step("arm the rebind")
        .on_enter(press_action("ship_rebind"))
        .until(frames(1))
        .add()
        // The arming key has to be UP before the capture will take anything,
        // so this beat is both the release and the hold on the amber prompt.
        .step("let the key up and hold the armed prompt")
        .on_enter(release_action("ship_rebind"))
        .until(and(the_rebind_is_armed(), frames(ARMED_CELLS)))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("press the key the section is to answer to")
        .on_enter(press_key(REBIND_KEY))
        .until(frames(1))
        .add()
        .step("let it up and hold the trigger it took")
        .on_enter(release_key(REBIND_KEY))
        .until(sheet_written(REBIND_LESSON))
        .deadline(60.0)
        .add()
        // A sheet that closed over an armed prompt would be twenty cells of a
        // capture that never captured, so the run ends on the binding itself.
        .step("the turret took the key")
        .until(the_turret_answers_to_the_new_key())
        .deadline(STEP_DEADLINE_SECS)
        .add()
}
