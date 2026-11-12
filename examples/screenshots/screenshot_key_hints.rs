//! screenshot_key_hints: the key hints name the key the player would press
//! now, on the device in hand.
//!
//! Eight stills of one scene, a warship beside a hauler that coasts on an
//! orbit, with the hauler as the travel lock. The warship carries a mining
//! emitter under the bow and binds each weapon and emitter from
//! `section_default_bindings`, so the row can show every chip. The chips are
//! found by NAME: the row is contextual, so the STOP chip's place depends on
//! what else is shown.
//! - `key-hints-stop.png`: the dock row at rest. The STOP chip reads `STOP`.
//! - `key-hints-match-chord.png`: the RCS modifier held. The STOP chip reads
//!   `MATCH` and the RCS chip is lit.
//! - `key-hints-match-flying-chord.png`: MATCH engaged, the modifier still
//!   held. The STOP chip reads `MATCH`, hot: the same press ends the match.
//! - `key-hints-match-flying.png`: MATCH engaged, the modifier let go. The
//!   STOP chip reads `STOP`, lit and not hot: the plain key brakes and
//!   replaces the match.
//! - `key-hints-pad.png`: after a pad press. The row draws pad buttons, and
//!   DOCK draws two caps: the RCS modifier, then the GOTO button.
//! - `key-hints-pad-weapons.png`: the stance trigger held. FIRE, TORPEDO,
//!   RAILGUN and MINE draw the pad buttons of their sections.
//! - `key-hints-pad-docked.png`: docked by the DOCK chord. HELM draws two
//!   caps: the RCS modifier, then the ORBIT button.
//! - `key-hints-interface-pad.png`: the interface card head with the pad's
//!   Close and Next buttons.
//!
//! Real player input only: `rcs_modifier`, `autopilot_stop`, `autopilot_off`
//! and `interface_toggle` through `press_action`, and buttons pressed on the
//! synthesized pad through `press_source`. The script checks the hint state
//! behind each still and panics when it is wrong, so a smoke run is a proof
//! without the PNGs. The pad half docks with L3 + Y, takes the helm with
//! L3 + A and undocks with L3 + Y, and checks each.
//!
//! The dock is posed, not flown: the warship is put at a berth off the
//! hauler's port and both clocks are held, so the chord meets a pair inside
//! the envelope. The clocks are also held while the match flies, since a
//! slow renderer otherwise lets the match finish between two stills.
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

use std::collections::BTreeMap;
#[cfg(feature = "debug")]
use std::sync::Arc;

#[cfg(feature = "debug")]
use avian3d::prelude::{AngularVelocity, LinearVelocity, Position, Rotation};
#[cfg(feature = "debug")]
use bevy::ecs::system::RunSystemOnce;
use bevy::prelude::*;
use clap::Parser;
use nova_input::prelude::InputSource;
#[cfg(feature = "debug")]
use nova_input::prelude::{dispatch, InputBindings, InputDeviceType, InputPhase};
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

/// The player's mining emitter: its id on the hull and its catalog section.
const EMITTER_ID: &str = "hints_emitter";
const EMITTER_SECTION: &str = "mining_beam_section";
/// The emitter's cell: under the bow, ahead of the keel, so its lane runs
/// forward under the hull.
const EMITTER_CELL: Vec3 = Vec3::new(0.0, -1.0, -2.0);

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
    let hull = with_an_emitter(design("line_warship"));
    let player = ring::ship(
        PLAYER_ID,
        "Hints Player",
        START,
        facing,
        SpaceshipController::Player(PlayerControllerConfig {
            input_mapping: default_input_mapping(&hull),
        }),
        None,
        hull,
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
            position: None,
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

/// `hull` with a mining emitter at [`EMITTER_CELL`], so one hull carries a
/// section of every kind the row has a chip for.
fn with_an_emitter(mut hull: ShipDesign) -> ShipDesign {
    assert!(
        hull.sections
            .iter()
            .all(|section| !section.position.abs_diff_eq(EMITTER_CELL, 0.5)),
        "screenshot_key_hints: the emitter cell is taken"
    );
    hull.sections.push(SpaceshipSectionConfig {
        id: EMITTER_ID.to_string(),
        position: EMITTER_CELL,
        rotation: Quat::IDENTITY,
        source: SectionSource::prototype(EMITTER_SECTION),
    });
    hull
}

/// The bindings a new hull gets: each weapon and mining section on its
/// [`section_default_bindings`], as the editor and the start roles bind one.
fn default_input_mapping(hull: &ShipDesign) -> BTreeMap<SectionId, Vec<InputSource>> {
    let mut mining_taken = Vec::new();
    hull.sections
        .iter()
        .filter_map(|section| {
            let class = match section.source.prototype_id() {
                PDC_KINETIC_TURRET_SECTION_ID => SectionClass::Turret,
                TORPEDO_SECTION_ID => SectionClass::Torpedo,
                RAILGUN_LANCE_SECTION_ID => SectionClass::Railgun,
                EMITTER_SECTION => SectionClass::Mining,
                _ => return None,
            };
            let sources = section_default_bindings(class, &mining_taken)
                .expect("screenshot_key_hints: one emitter, so a mining key is free");
            if class == SectionClass::Mining {
                mining_taken.extend(&sources);
            }
            Some((section.id.clone(), sources))
        })
        .collect()
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

/// The first pad button of `action`, as the row labels it.
#[cfg(feature = "debug")]
fn pad_label(world: &World, action: &str) -> String {
    world
        .resource::<InputBindings>()
        .get(action)
        .and_then(|binding| binding.gamepad.first())
        .map(|source| source.glyph_label())
        .unwrap_or_else(|| panic!("screenshot_key_hints: '{action}' holds no pad button"))
}

/// Panic unless `hint` draws the pad chord: the RCS modifier, then the first
/// pad button of `rides`.
#[cfg(feature = "debug")]
fn assert_chord(world: &World, verb: &str, hint: &VerbHint, rides: &str) {
    assert_eq!(
        (hint.modifier.clone(), hint.key.clone()),
        (
            Some(pad_label(world, "rcs_modifier")),
            pad_label(world, rides)
        ),
        "screenshot_key_hints: {verb} draws the modifier cap, then the '{rides}' button"
    );
    assert!(
        hint.shown,
        "screenshot_key_hints: {verb} is on the row: {hint:?}"
    );
}

/// Press or release one button of the synthesized pad.
#[cfg(feature = "debug")]
fn pad(button: GamepadButton, phase: InputPhase) -> impl Fn(&mut World) + Send + Sync + 'static {
    move |world: &mut World| dispatch::press_source(world, InputSource::Gamepad(button), phase)
}

/// Hold or release both clocks. The walk borrows the debug HUD's hold: that
/// HUD is closed for the whole run, so nothing else takes or drops it.
#[cfg(feature = "debug")]
fn hold_the_clocks(held: bool) -> impl Fn(&mut World) + Send + Sync + 'static {
    move |world: &mut World| {
        world
            .run_system_once(move |mut clocks: Clocks| {
                if held {
                    clocks.hold(FreezeOwner::DebugHud);
                } else {
                    clocks.release(FreezeOwner::DebugHud);
                }
            })
            .expect("screenshot_key_hints: the clocks answer");
    }
}

/// Every docking port of `root`: position and rotation in the root's frame.
#[cfg(feature = "debug")]
fn ports_of(world: &mut World, root: Entity) -> Vec<(Vec3, Quat)> {
    let mut ports = world.query_filtered::<(&ChildOf, &Transform), With<DockingSectionMarker>>();
    ports
        .iter(world)
        .filter(|(&ChildOf(parent), _)| parent == root)
        .map(|(_, at)| (at.translation, at.rotation))
        .collect()
}

/// Put the warship at a berth off the hauler's port, at rest against it, with
/// both clocks held. The pose is [`parked_hull_pose`], the one a hull parked
/// at a station stands at, so the pair is inside the capture envelope.
#[cfg(feature = "debug")]
fn berth_at_the_hauler(world: &mut World) {
    let player = ring::player_root(world).expect("screenshot_key_hints: the player is present");
    let hauler =
        ring::entity_by_id(world, TARGET_ID).expect("screenshot_key_hints: the hauler is present");
    let hauler_port = *ports_of(world, hauler)
        .first()
        .expect("screenshot_key_hints: the hauler carries a port");
    let outward = |port: &(Vec3, Quat)| port.1 * Vec3::NEG_Z;
    // The warship collar that already faces the hauler's, so the two hulls
    // stand side by side with no turn.
    let player_port = ports_of(world, player)
        .into_iter()
        .min_by(|a, b| {
            outward(a)
                .dot(outward(&hauler_port))
                .total_cmp(&outward(b).dot(outward(&hauler_port)))
        })
        .expect("screenshot_key_hints: the warship carries a collar");

    let hauler_ref = world.entity(hauler);
    let body = |what: &str| -> ! { panic!("screenshot_key_hints: the hauler has no {what}") };
    let at = hauler_ref
        .get::<Position>()
        .unwrap_or_else(|| body("position"))
        .0;
    let turn = hauler_ref
        .get::<Rotation>()
        .unwrap_or_else(|| body("rotation"))
        .0;
    let velocity = *hauler_ref
        .get::<LinearVelocity>()
        .unwrap_or_else(|| body("velocity"));
    let spin = *hauler_ref
        .get::<AngularVelocity>()
        .unwrap_or_else(|| body("spin"));
    let (position, rotation) = parked_hull_pose((at, turn), hauler_port, player_port);

    hold_the_clocks(true)(world);
    let mut ship = world.entity_mut(player);
    ship.insert((Position(position), Rotation(rotation), velocity, spin));
    let mut transform = ship
        .get_mut::<Transform>()
        .expect("screenshot_key_hints: the player has a transform");
    transform.translation = position;
    transform.rotation = rotation;
}

/// The player holds no dock, or holds one.
#[cfg(feature = "debug")]
fn the_player_is_docked(docked: bool) -> Arc<Predicate> {
    Arc::new(move |world: &World| {
        ring::player_root_ref(world)
            .is_some_and(|player| world.get::<DockedShip>(player).is_some() == docked)
    })
}

/// Panic unless the row draws the pad button of each weapon and mining
/// section, read off the same table the hull was bound from.
#[cfg(feature = "debug")]
fn assert_section_pad_labels(world: &World) {
    let pad_of = |class| {
        section_default_bindings(class, &[])
            .into_iter()
            .flatten()
            .find(|source| matches!(source, InputSource::Gamepad(_)))
            .map(|source| source.glyph_label())
            .unwrap_or_else(|| panic!("screenshot_key_hints: {class:?} has no pad default"))
    };
    let hints = world.resource::<FlightVerbHints>();
    for (verb, hint, class) in [
        ("FIRE", &hints.fire, SectionClass::Turret),
        ("TORPEDO", &hints.torpedo, SectionClass::Torpedo),
        ("RAILGUN", &hints.railgun, SectionClass::Railgun),
        ("MINE", &hints.mine, SectionClass::Mining),
    ] {
        assert_eq!(
            (hint.key.clone(), hint.shown, hint.available),
            (pad_of(class), true, true),
            "screenshot_key_hints: {verb} draws its section's pad button"
        );
    }
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
/// fly the MATCH chord, cancel it, press a pad button, raise the weapons on
/// the pad, dock and undock by the pad chord, open the interface.
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
        // The match is held still from here to its cancel: under a slow
        // renderer it otherwise completes between two stills.
        .step("let the STOP key up")
        .on_enter(|world: &mut World| {
            hold_the_clocks(true)(world);
            release_action("autopilot_stop")(world);
        })
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
    .on_enter(|world: &mut World| {
        hold_the_clocks(false)(world);
        press_action("autopilot_off")(world);
    })
    .until(hints_where(|hints| {
        !hints.engaged && hints.stop_verb == StopVerbType::Stop
    }))
    .deadline(BEAT_DEADLINE_SECS)
    .add()
    .step("press a pad button")
    .on_enter(|world: &mut World| {
        release_action("autopilot_off")(world);
        pad(UNBOUND_PAD_BUTTON, InputPhase::Press)(world);
    })
    .until(the_pad_is_in_hand())
    .deadline(BEAT_DEADLINE_SECS)
    .add()
    .step("let the row repaint")
    .on_enter(pad(UNBOUND_PAD_BUTTON, InputPhase::Release))
    .until(frames(2))
    .add();
    let script = still(script, "key-hints-pad.png", |world| {
        assert_stop_label(world, InputDeviceType::Pad);
        let hints = world.resource::<FlightVerbHints>();
        assert_chord(world, "DOCK", &hints.dock, "autopilot_goto");
        assert!(
            !hints.dock.available,
            "screenshot_key_hints: the hauler is out of reach, so DOCK is dim"
        );
    })
    .step("hold the stance trigger")
    .on_enter(pad(GamepadButton::LeftTrigger2, InputPhase::Press))
    .until(hints_where(|hints| {
        hints.fire.shown && hints.torpedo.shown && hints.railgun.shown && hints.mine.shown
    }))
    .deadline(BEAT_DEADLINE_SECS)
    .add();
    let script = still(script, "key-hints-pad-weapons.png", |world| {
        assert_section_pad_labels(world);
    })
    .step("let the stance trigger up")
    .on_enter(pad(GamepadButton::LeftTrigger2, InputPhase::Release))
    .until(hints_where(|hints| !hints.fire.shown))
    .deadline(BEAT_DEADLINE_SECS)
    .add()
    .step("berth at the hauler")
    .on_enter(berth_at_the_hauler)
    .until(hints_where(|hints| hints.dock.available))
    .deadline(BEAT_DEADLINE_SECS)
    .add()
    .step("hold L3")
    .on_enter(pad(GamepadButton::LeftThumb, InputPhase::Press))
    .until(hints_where(|hints| hints.rcs_held))
    .deadline(BEAT_DEADLINE_SECS)
    .add()
    .step("press Y under L3")
    .on_enter(pad(GamepadButton::North, InputPhase::Press))
    .until(the_player_is_docked(true))
    .deadline(BEAT_DEADLINE_SECS)
    .add()
    .step("let Y up")
    .on_enter(pad(GamepadButton::North, InputPhase::Release))
    .until(hints_where(|hints| hints.helm.shown))
    .deadline(BEAT_DEADLINE_SECS)
    .add();
    let script = still(script, "key-hints-pad-docked.png", |world| {
        assert!(
            !any_entity::<(With<PlayerSpaceshipMarker>, With<Autopilot>)>()(world),
            "screenshot_key_hints: Y under L3 docked and flew no GOTO"
        );
        let hints = world.resource::<FlightVerbHints>();
        assert_chord(world, "DOCK", &hints.dock, "autopilot_goto");
        assert_chord(world, "HELM", &hints.helm, "autopilot_orbit");
        assert!(
            hints.helm.available && !hints.helm_held,
            "screenshot_key_hints: the helm is there to take: {hints:?}"
        );
    })
    .step("press A under L3")
    .on_enter(pad(GamepadButton::South, InputPhase::Press))
    .until(hints_where(|hints| hints.helm_held))
    .deadline(BEAT_DEADLINE_SECS)
    .add()
    .step("let A up")
    .on_enter(pad(GamepadButton::South, InputPhase::Release))
    .until(frames(2))
    .add()
    .step("press Y under L3 again")
    .on_enter(|world: &mut World| {
        assert!(
            the_player_is_docked(true)(world),
            "screenshot_key_hints: taking the helm kept the dock"
        );
        pad(GamepadButton::North, InputPhase::Press)(world);
    })
    .until(the_player_is_docked(false))
    .deadline(BEAT_DEADLINE_SECS)
    .add()
    .step("let the pad up")
    .on_enter(|world: &mut World| {
        pad(GamepadButton::North, InputPhase::Release)(world);
        pad(GamepadButton::LeftThumb, InputPhase::Release)(world);
        hold_the_clocks(false)(world);
    })
    .until(hints_where(|hints| !hints.rcs_held && !hints.helm.shown))
    .deadline(BEAT_DEADLINE_SECS)
    .add()
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
        pad(UNBOUND_PAD_BUTTON, InputPhase::Press)(world);
    })
    .until(the_pad_is_in_hand())
    .deadline(BEAT_DEADLINE_SECS)
    .add()
    .step("let the head repaint")
    .until(frames(3))
    .add();
    still(script, "key-hints-interface-pad.png", |_| {})
}
