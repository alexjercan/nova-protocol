//! screenshot_key_hints: the key hints name the key the player would press
//! now, on the device in hand.
//!
//! Six stills of one scene, a warship beside a hauler that coasts on an
//! orbit, with the hauler as the travel lock:
//! - `key-hints-stop.png`: the dock row at rest. Chip one reads `STOP`.
//! - `key-hints-match-chord.png`: the RCS modifier held. Chip one reads
//!   `MATCH` and the RCS chip is lit.
//! - `key-hints-match-flying-chord.png`: MATCH engaged, the modifier still
//!   held. Chip one reads `MATCH`, hot: the same press ends the match.
//! - `key-hints-match-flying.png`: MATCH engaged, the modifier let go. Chip one
//!   reads `STOP`, lit and not hot: the plain key brakes and replaces the match.
//! - `key-hints-pad.png`: after a pad press. The row draws pad buttons.
//! - `key-hints-interface-pad.png`: the interface card head with the pad's
//!   Close and Next buttons.
//!
//! Real player input only: `rcs_modifier`, `autopilot_stop`, `autopilot_off`
//! and `interface_toggle` through `press_action`, and a button pressed on a
//! spawned `Gamepad`. The script checks the hint state behind each still and
//! panics when it is wrong, so a smoke run is a proof without the PNGs.
//!
//! Two run modes, both under the autopilot (`NOVA_AUTOPILOT`):
//! - `NOVA_AUTOPILOT=1` alone: the smoke path, writing nothing.
//! - `NOVA_AUTOPILOT=1 NOVA_CAPTURE=1`: also write the stills (staged under
//!   `NOVA_CAPTURE_DIR`).
//!
//! ```text
//! NOVA_CAPTURE_DIR=/tmp/key-hints NOVA_AUTOPILOT=1 NOVA_CAPTURE=1 \
//!   cargo run --example screenshot_key_hints --features debug
//! ```

#[path = "shared/ring.rs"]
mod ring;

#[cfg(feature = "debug")]
use std::sync::Arc;

use bevy::prelude::*;
use clap::Parser;
#[cfg(feature = "debug")]
use nova_input::prelude::{InputBindings, InputDeviceType};
#[cfg(feature = "debug")]
use nova_protocol::nova_debug::harness::Predicate;
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "screenshot_key_hints")]
#[command(version = "1.0.0")]
#[command(
    about = "Shoot the dock row and the interface head for STOP, the MATCH chord and a gamepad",
    long_about = None
)]
struct Cli;

const PLAYER_ID: &str = "hints_player";
const TARGET_ID: &str = "hints_hauler";

/// The player's start: outside the hauler's orbit, on the hauler's side of
/// the planetoid.
const START: Meters3 = Meters3::new(-600.0, 0.0, -4_300.0);
/// The hauler's start: 4 km from the planetoid's centre, inside the core of
/// its well, where a circular orbit holds.
const TARGET_START: Meters3 = Meters3::new(0.0, 0.0, -4_000.0);

/// How long the scene settles before the walk presses anything.
#[cfg(feature = "debug")]
const SETTLE_SECS: f32 = 0.5;

/// A pad button no action is bound to, so the press changes the device in
/// hand and nothing else.
#[cfg(feature = "debug")]
const UNBOUND_PAD_BUTTON: GamepadButton = GamepadButton::Mode;

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();
    // NovaMenuPlugin explicitly: the interface's clock hold lives in
    // nova_menu, and `with_game_plugins` turns the menu plugin off.
    let mut app = AppBuilder::new()
        .with_game_plugins((custom_plugin, NovaMenuPlugin))
        .build();

    #[cfg(feature = "debug")]
    {
        app.add_plugins(key_hints_script());
        app.add_systems(Startup, (force_capture_resolution, hide_dev_overlays));
        app.add_systems(Update, ring::drive_leg_camera);
    }

    app.run()
}

fn custom_plugin(app: &mut App) {
    app.add_systems(OnEnter(GameAssetsStates::Loaded), load_scene);
}

/// The scene: [`ring::planetoid`]'s well, the player at [`START`] facing the
/// hauler, and the hauler at [`TARGET_START`] on a circular orbit, so the
/// lock has a velocity to match.
fn load_scene(mut commands: Commands, game_assets: Res<GameAssets>, ships: Res<GameShipDesigns>) {
    let design = |id: &str| {
        ships
            .get_design(&id.into())
            .unwrap_or_else(|| panic!("screenshot_key_hints: unknown ship '{id}'"))
            .design
            .clone()
    };
    let facing = Quat::from_rotation_arc(
        Vec3::NEG_Z,
        (TARGET_START.get() - START.get())
            .try_normalize()
            .expect("screenshot_key_hints: START and TARGET_START must differ"),
    );
    let player = ring::ship(
        PLAYER_ID,
        "Hints Player",
        START,
        facing,
        SpaceshipController::Player(PlayerControllerConfig {
            // The walk never fires: nothing here needs a weapon binding.
            input_mapping: std::collections::BTreeMap::new(),
        }),
        None,
        design("line_warship"),
    );
    // Circular orbit speed: v = sqrt(mu / r), in engine units because mu is
    // authored in them, then converted at the scenario edge.
    let radius = TARGET_START.to_engine().length();
    let orbit_speed = (ring::PLANETOID_MASS / radius).sqrt();
    let hauler = EventActionConfig::SpawnScenarioObject(ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: TARGET_ID.to_string(),
            name: "Hints Hauler".to_string(),
            position: TARGET_START,
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Spaceship(SpaceshipConfig {
            controller: SpaceshipController::None,
            allegiance: Some(Allegiance::Neutral),
            design: ShipDesignSource::Inline(design("light_freighter")),
            initial_velocity: MetersPerSecond3::from_engine(Vec3::X * orbit_speed),
            ..default()
        }),
    });
    commands.trigger(LoadScenario(ScenarioConfig {
        description: "A warship beside a hauler on an orbit, for the key hints.".to_string(),
        events: vec![ScenarioEventConfig {
            label: None,
            name: EventConfig::OnStart,
            once: false,
            filters: vec![],
            actions: [
                vec![ring::planetoid(), player, hauler],
                ThreePointRig::around("photo", Meters3::ZERO, 1.0).actions(),
            ]
            .concat(),
        }],
        ..ScenarioConfig::new(
            "screenshot_key_hints".to_string(),
            "Key Hints".to_string(),
            game_assets.cubemap.clone().into(),
        )
    }));
}

/// True once the hints satisfy `check`.
#[cfg(feature = "debug")]
fn hints_where(check: impl Fn(&FlightVerbHints) -> bool + Send + Sync + 'static) -> Arc<Predicate> {
    Arc::new(move |world: &World| world.get_resource::<FlightVerbHints>().is_some_and(&check))
}

/// The label the dock draws for `action` on `device`, off the live table.
#[cfg(feature = "debug")]
fn live_label(world: &World, action: &str, device: InputDeviceType) -> String {
    world
        .resource::<InputBindings>()
        .get(action)
        .and_then(|binding| binding.hint_source(device))
        .map(|source| source.glyph_label())
        .unwrap_or_else(|| panic!("screenshot_key_hints: '{action}' holds no source"))
}

/// One still, held until the PNG is on disk. `check` runs first, on the
/// state the still is about to show, and panics when the state is wrong.
#[cfg(feature = "debug")]
fn still(
    script: nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates>,
    name: &'static str,
    check: impl Fn(&mut World) + Send + Sync + 'static,
) -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    script
        .step(format!("shoot {name}"))
        .on_enter(move |world: &mut World| {
            check(world);
            shoot(world, name);
        })
        .until(shot_written(name))
        .deadline(SHOT_DEADLINE_SECS)
        .add()
}

/// The pad is the device in hand.
#[cfg(feature = "debug")]
fn the_pad_is_in_hand() -> Arc<Predicate> {
    Arc::new(|world: &World| {
        world
            .get_resource::<InputDeviceType>()
            .is_some_and(|device| *device == InputDeviceType::Pad)
    })
}

/// Panic unless the dock draws `device`'s source for STOP.
#[cfg(feature = "debug")]
fn assert_stop_label(world: &World, device: InputDeviceType) {
    assert_eq!(
        world.resource::<FlightVerbHints>().stop.key,
        live_label(world, "autopilot_stop", device),
        "screenshot_key_hints: the row draws the {device:?} source for STOP"
    );
}

/// The engaged action is a MATCH, whatever the chip reads.
#[cfg(feature = "debug")]
fn assert_match_flying(world: &mut World) {
    let mut ships = world.query_filtered::<&Autopilot, With<PlayerSpaceshipMarker>>();
    let autopilot = ships
        .single(world)
        .expect("screenshot_key_hints: one player autopilot");
    assert!(
        matches!(autopilot.action, AutopilotAction::MatchTarget { .. }),
        "screenshot_key_hints: the chord engaged {:?}, not MATCH",
        autopilot.action
    );
}

/// The walk: lock the hauler, shoot the row at rest, hold the RCS modifier,
/// fly the MATCH chord, cancel it, press a pad button, open the interface.
#[cfg(feature = "debug")]
fn key_hints_script() -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    let script = nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
        .step("load the scene")
        .enter(GameStates::Loading)
        .until(player_ship_present())
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("show the HUD")
        .on_enter(ring::hud_instrument)
        .until(elapsed(SETTLE_SECS))
        .add()
        .step("lock the hauler")
        .on_enter(|world: &mut World| {
            let player =
                ring::player_root(world).expect("screenshot_key_hints: the player is present");
            let target = ring::entity_by_id(world, TARGET_ID)
                .expect("screenshot_key_hints: the hauler is present");
            world.entity_mut(player).insert(TravelLock(Some(target)));
        })
        .until(hints_where(|hints| {
            hints.stop_verb == StopVerbType::Stop && hints.stop.available && hints.goto.available
        }))
        .deadline(BEAT_DEADLINE_SECS)
        .add();
    let script = still(script, "key-hints-stop.png", |world| {
        assert_stop_label(world, InputDeviceType::Desk);
    })
    .step("hold the RCS modifier")
    .on_enter(press_action("rcs_modifier"))
    .until(hints_where(|hints| {
        hints.rcs_held && hints.stop_verb == StopVerbType::Match && hints.stop.available
    }))
    .deadline(BEAT_DEADLINE_SECS)
    .add();
    let script = still(script, "key-hints-match-chord.png", |_| {})
        .step("press the STOP key under the modifier")
        .on_enter(press_action("autopilot_stop"))
        .until(any_entity::<(With<PlayerSpaceshipMarker>, With<Autopilot>)>())
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("let the STOP key up")
        .on_enter(release_action("autopilot_stop"))
        .until(hints_where(|hints| {
            hints.rcs_held
                && hints.stop_verb == StopVerbType::Match
                && hints.stop_flying
                && hints.engaged
        }))
        .deadline(BEAT_DEADLINE_SECS)
        .add();
    let script = still(script, "key-hints-match-flying-chord.png", |world| {
        assert_match_flying(world);
    })
    .step("let the modifier up")
    .on_enter(release_action("rcs_modifier"))
    .until(hints_where(|hints| {
        !hints.rcs_held
            && hints.stop_verb == StopVerbType::Stop
            && !hints.stop_flying
            && hints.engaged
    }))
    .deadline(BEAT_DEADLINE_SECS)
    .add();
    let script = still(script, "key-hints-match-flying.png", |world| {
        assert_match_flying(world);
        let hints = world.resource::<FlightVerbHints>();
        assert!(
            !hints.rcs_held
                && hints.stop_verb == StopVerbType::Stop
                && !hints.stop_flying
                && hints.engaged,
            "screenshot_key_hints: a flying match with the modifier up reads STOP, not hot: {hints:?}"
        );
    })
    .step("cancel the match")
    .on_enter(press_action("autopilot_off"))
    .until(hints_where(|hints| {
        !hints.engaged && hints.stop_verb == StopVerbType::Stop
    }))
    .deadline(BEAT_DEADLINE_SECS)
    .add()
    .step("press a pad button")
    .on_enter(|world: &mut World| {
        release_action("autopilot_off")(world);
        let mut pad = Gamepad::default();
        pad.digital_mut().press(UNBOUND_PAD_BUTTON);
        world.spawn((Name::new("Key Hints Pad"), pad));
    })
    .until(the_pad_is_in_hand())
    .deadline(BEAT_DEADLINE_SECS)
    .add()
    .step("let the row repaint")
    .until(frames(2))
    .add();
    let script = still(script, "key-hints-pad.png", |world| {
        assert_stop_label(world, InputDeviceType::Pad);
    })
    .step("open the interface")
    .on_enter(press_action("interface_toggle"))
    .until(state_is(PauseStates::Interface))
    .deadline(BEAT_DEADLINE_SECS)
    .add()
    .step("let the interface build")
    .on_enter(release_action("interface_toggle"))
    .until(frames(20))
    .add()
    // A harness key press is a desk press, so the hints went back to the
    // desk. Press the pad again, the way a pad player's next press would.
    .step("press the pad again")
    .on_enter(|world: &mut World| {
        assert_eq!(
            *world.resource::<InputDeviceType>(),
            InputDeviceType::Desk,
            "screenshot_key_hints: a key press hands the hints back to the desk"
        );
        let mut pads = world.query::<&mut Gamepad>();
        let mut pad = pads
            .single_mut(world)
            .expect("screenshot_key_hints: the pad spawned above");
        pad.digital_mut().release(UNBOUND_PAD_BUTTON);
        pad.digital_mut().press(UNBOUND_PAD_BUTTON);
    })
    .until(the_pad_is_in_hand())
    .deadline(BEAT_DEADLINE_SECS)
    .add()
    .step("let the head repaint")
    .until(frames(3))
    .add();
    still(script, "key-hints-interface-pad.png", |_| {})
}
