//! system_interface: the TAB interface and the `:` command modal over it, driven
//! the way a player drives them - keys only, one walk.
//!
//! ONE SUBJECT: the round trip pane -> modal -> pane. TAB opens the themed
//! interface on its Map pane, M switches to Ship, `:` opens NOVA COMMANDS over
//! the pane, and Escape closes the modal back to the SAME pane, with the world
//! held still the whole way. The parts are unit-tested
//! (`crates/nova_interface/src/pane.rs` and `terminal/tests/`); what no test
//! asserts is the chain AT ONCE in a real app, across the frames where one
//! surface hands the screen to the other.
//!
//! That handoff is where it can fail invisibly. A modal that returns to
//! `Unpaused` instead of the pane, a pane root respawned on the way back, or a
//! freeze released for one frame all leave a green run and a screen that looks
//! right: the player just loses their pane selection, or the world ticks once
//! under a computer they believe is holding it still. And the keys the modal
//! is typed with are the keys the pane and flight read - a key the pane acts on
//! during the transition frame moves its camera or fires a weapon with nobody
//! at the controls, and nothing on screen says so.
//!
//! So each claim reads state, not a picture: the pause axis and the recorded
//! `return_to`, the pane resource, the interface root and Ship camera entities
//! (the same ones before and after), `Time<Virtual>` (paused, and its elapsed
//! time identical before and after), and, for "nothing fired", the Ship camera
//! pose and the player ship's thruster, weapon and turn inputs and velocities
//! (unchanged). No timing is asserted.
//!
//! It is `systems/`, not `screenshots/`: the product is a verdict, not a frame.
//! `screenshot_interface` and `screenshot_command_shell` capture the surfaces
//! and assert nothing beyond not panicking.
//!
//! Smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//! ```text
//! NOVA_AUTOPILOT=1 cargo run --example system_interface --features debug
//! # named beats, each waiting on the world rather than on a dwell: load the
//! # range; TAB and wait for the interface and the freeze; M and wait for the
//! # Ship scene; type `:` and assert the modal covers a KEPT pane; Escape and
//! # assert the same pane is back with the clocks never having run; Escape
//! # again and assert flight is released. A beat that never resolves inside its
//! # deadline is an error exit naming it.
//! ```

#[cfg(feature = "debug")]
use avian3d::prelude::{AngularVelocity, LinearVelocity};
use bevy::prelude::*;
#[cfg(feature = "debug")]
use bevy::{
    ecs::entity::Entity,
    time::{Time, Virtual},
};
use clap::Parser;
#[cfg(feature = "debug")]
use nova_protocol::nova_interface::prelude::{InterfacePaneType, NovaOsCloseTransition};
use nova_protocol::prelude::*;

#[path = "../screenshots/shared/computer.rs"]
mod computer;

/// The name of the interface's root node.
#[cfg(feature = "debug")]
const INTERFACE_ROOT: &str = "InterfaceRoot";

/// The Ship pane's orbit camera. It is spawned with the pane's scene and
/// despawned with it, so the same entity before and after the modal is the proof
/// the scene was not rebuilt.
#[cfg(feature = "debug")]
const SHIP_CAMERA: &str = "NovaOsShipCamera";

/// Frames a "nothing happened" beat waits before it believes nothing happened.
/// The claims are equalities on frozen state, so this is a count of chances for
/// a wrong system to act, not a duration.
#[cfg(feature = "debug")]
const SETTLE_FRAMES: u32 = 30;

#[derive(Parser)]
#[command(name = "system_interface")]
#[command(version = "1.0.0")]
#[command(about = "The TAB interface and the `:` command modal over it. Autopilot-only correctness range", long_about = None)]
struct Cli;

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();
    // NovaMenuPlugin explicitly: the interface's clock hold and the `:` gesture
    // live in nova_menu, and `with_game_plugins` turns the menu plugin off. They
    // are the subject here, so the plugin has to be present; it stands alone in
    // a slim app, and the run goes Loading -> Playing without the menu handoff.
    let mut app = AppBuilder::new()
        .with_game_plugins((custom_plugin, NovaMenuPlugin))
        .build();

    #[cfg(feature = "debug")]
    {
        // Probe wiring (each plugin is inert without its NOVA_PROBE_* env): run
        // timeline + engine-bound invariants, so `probe run` grades this range.
        // No frame-time capture - the walk is a sequence of gestures with no
        // steady-state window, so a captured fps would measure the script.
        app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
        app.add_plugins(interface_script());
    }

    app.run()
}

fn custom_plugin(app: &mut App) {
    app.add_systems(OnEnter(GameAssetsStates::Loaded), setup_range);
}

/// Everything the walk compares across the handoff, taken while the Ship pane
/// is open and settled.
#[cfg(feature = "debug")]
#[derive(Resource, Clone)]
struct Baseline {
    /// The interface root and the Ship camera: the surfaces that must survive
    /// the modal.
    root: Entity,
    camera: Entity,
    /// `Time<Virtual>::elapsed` when TAB first held the clocks.
    virtual_elapsed: std::time::Duration,
    camera_pose: Transform,
    flight: FlightInputs,
}

/// What a flight action would have changed on the player ship.
#[cfg(feature = "debug")]
#[derive(Debug, Clone, PartialEq)]
struct FlightInputs {
    thrust: Vec<f32>,
    fire: Vec<bool>,
    heading: Vec<Quat>,
    linear: Vec3,
    angular: Vec3,
}

/// The walk, one beat per gesture or claim.
#[cfg(feature = "debug")]
fn interface_script() -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    use nova_protocol::nova_debug::harness::{BEAT_DEADLINE_SECS, STEP_DEADLINE_SECS};

    nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
        // Wait for the ship to EXIST: the interface keys off the player ship
        // root, so a TAB pressed before it spawned would open nothing.
        .step("interface: load the range")
        .enter(GameStates::Loading)
        .until(player_ship_present())
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("interface: press TAB")
        .on_enter(press_key(KeyCode::Tab))
        .until(state_is(PauseStates::Interface))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        // The pause axis flips a frame before the freeze lands, so the clock is
        // waited on rather than read on the frame the state changed.
        .step("interface: wait for the freeze")
        .on_enter(release_key(KeyCode::Tab))
        .until(virtual_clock_paused())
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("interface: TAB opens the Map pane")
        .on_enter(assert_opens_on_map)
        .add()
        .step("interface: let frames pass")
        .until(frames(SETTLE_FRAMES))
        .add()
        .step("interface: the clocks did not run")
        .on_enter(assert_clocks_held)
        .add()
        .step("interface: press M")
        .on_enter(press_key(KeyCode::KeyM))
        .until(resource_where::<InterfacePaneType>(|pane| {
            *pane == InterfacePaneType::Ship
        }))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("interface: wait for the Ship scene")
        .on_enter(release_key(KeyCode::KeyM))
        .until(named_entity_present(SHIP_CAMERA))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        // The scene is built and the camera has had frames to finish framing
        // the hull; the baseline is taken after that, so a later pose change is
        // the modal's doing and not the pane still settling.
        .step("interface: let the Ship scene settle")
        .until(frames(SETTLE_FRAMES))
        .add()
        .step("interface: M switches to the Ship pane")
        .on_enter(assert_m_switched_and_record)
        .add()
        .step("interface: type `:` over the pane")
        .on_enter(type_text(":"))
        .until(state_is(PauseStates::Commands))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("interface: let frames pass under the modal")
        .until(frames(SETTLE_FRAMES))
        .add()
        .step("interface: the modal covers a kept pane")
        .on_enter(assert_modal_keeps_pane)
        .add()
        // Closing is animated in real time: the state is back a few frames
        // after the key, so the state is what is waited on.
        .step("interface: press Escape over the modal")
        .on_enter(press_key(KeyCode::Escape))
        .until(state_is(PauseStates::Interface))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("interface: let frames pass after the return")
        .on_enter(release_key(KeyCode::Escape))
        .until(frames(SETTLE_FRAMES))
        .add()
        .step("interface: Escape returns to the same pane")
        .on_enter(assert_escape_returns_to_pane)
        .add()
        .step("interface: nothing fired on the transition frames")
        .on_enter(assert_nothing_fired)
        .add()
        .step("interface: press Escape over the pane")
        .on_enter(press_key(KeyCode::Escape))
        .until(state_is(PauseStates::Unpaused))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("interface: wait for flight to be released")
        .on_enter(release_key(KeyCode::Escape))
        .until(resource_where::<ClockFreeze>(|freeze| !freeze.is_held()))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("interface: Escape leaves the pane")
        .on_enter(assert_escape_leaves_the_pane)
        .add()
}

/// Advance once the virtual clock is paused.
#[cfg(feature = "debug")]
fn virtual_clock_paused() -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    resource_where::<Time<Virtual>>(|clock| clock.is_paused())
}

/// Advance once something named `name` exists.
#[cfg(feature = "debug")]
fn named_entity_present(
    name: &'static str,
) -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(move |world: &World| {
        let Some(mut names) = world.try_query::<&Name>() else {
            return false;
        };
        names.iter(world).any(|found| found.as_str() == name)
    })
}

/// The one entity called `name`. A second one is a leak, so it panics.
#[cfg(feature = "debug")]
fn the_named(world: &mut World, name: &str) -> Entity {
    let mut query = world.query::<(Entity, &Name)>();
    let found: Vec<Entity> = query
        .iter(world)
        .filter(|(_, found)| found.as_str() == name)
        .map(|(entity, _)| entity)
        .collect();
    let [entity] = found[..] else {
        panic!("expected exactly one `{name}`; found {}", found.len());
    };
    entity
}

#[cfg(feature = "debug")]
fn pause_state(world: &World) -> PauseStates {
    *world.resource::<State<PauseStates>>().get()
}

/// What a flight action would have changed, read off the player ship.
#[cfg(feature = "debug")]
fn flight_inputs(world: &mut World) -> FlightInputs {
    let mut thrust = world.query::<&ThrusterSectionInput>();
    let mut turrets = world.query::<&TurretSectionInput>();
    let mut torpedoes = world.query::<&TorpedoSectionInput>();
    let mut heading = world.query::<&PDControllerInput>();
    let mut ship =
        world.query_filtered::<(&LinearVelocity, &AngularVelocity), With<PlayerSpaceshipMarker>>();
    let (linear, angular) = ship
        .single(world)
        .map(|(linear, angular)| (linear.0, angular.0))
        .expect("the range has exactly one player ship");
    let fire = turrets
        .iter(world)
        .map(|input| input.0)
        .chain(torpedoes.iter(world).map(|input| input.0))
        .collect();
    FlightInputs {
        thrust: thrust.iter(world).map(|input| input.0).collect(),
        fire,
        heading: heading.iter(world).map(|input| input.0).collect(),
        linear,
        angular,
    }
}

/// TAB opened the pane axis on Map, held the shared freeze, and put exactly one
/// root on screen.
#[cfg(feature = "debug")]
fn assert_opens_on_map(world: &mut World) {
    assert_eq!(
        pause_state(world),
        PauseStates::Interface,
        "TAB must open the interface by driving the shared pause axis"
    );
    assert_eq!(
        *world.resource::<InterfacePaneType>(),
        InterfacePaneType::Map,
        "the first open must show the Map pane"
    );
    the_named(world, INTERFACE_ROOT);
    assert!(
        world
            .resource::<ClockFreeze>()
            .is_held_by(FreezeOwner::Interface),
        "the interface must hold the clocks under its own freeze owner"
    );
    let elapsed = world.resource::<Time<Virtual>>().elapsed();
    world.insert_resource(FrozenAt(elapsed));
    nova_probe::probe_marker(
        world,
        "outcome: tab opens the interface",
        serde_json::json!({ "pane": "Map" }),
    );
}

/// `Time<Virtual>::elapsed` when TAB first held the clocks, kept until the pane
/// closes so the whole walk is one comparison.
#[cfg(feature = "debug")]
#[derive(Resource)]
struct FrozenAt(std::time::Duration);

/// The frozen clock never advanced while the pane was open.
#[cfg(feature = "debug")]
fn assert_clocks_held(world: &mut World) {
    let frozen = world.resource::<FrozenAt>().0;
    let now = world.resource::<Time<Virtual>>().elapsed();
    assert_eq!(
        now, frozen,
        "Time<Virtual> advanced under the open interface"
    );
    assert!(world.resource::<Time<Virtual>>().is_paused());
}

/// M switched the pane, then the baseline every later beat compares against.
#[cfg(feature = "debug")]
fn assert_m_switched_and_record(world: &mut World) {
    assert_eq!(
        *world.resource::<InterfacePaneType>(),
        InterfacePaneType::Ship,
        "M must switch the interface to the Ship pane"
    );
    assert_eq!(pause_state(world), PauseStates::Interface);
    let root = the_named(world, INTERFACE_ROOT);
    let camera = the_named(world, SHIP_CAMERA);
    let camera_pose = *world
        .get::<Transform>(camera)
        .expect("the Ship camera has a pose");
    let baseline = Baseline {
        root,
        camera,
        virtual_elapsed: world.resource::<FrozenAt>().0,
        camera_pose,
        flight: flight_inputs(world),
    };
    world.insert_resource(baseline);
    nova_probe::probe_marker(
        world,
        "outcome: m switches the pane",
        serde_json::json!({ "pane": "Ship" }),
    );
}

/// `:` over the pane opened the modal, recorded the pane as its return, and
/// left the pane and its scene standing under it.
#[cfg(feature = "debug")]
fn assert_modal_keeps_pane(world: &mut World) {
    let baseline = world.resource::<Baseline>().clone();
    assert_eq!(pause_state(world), PauseStates::Commands);
    assert_eq!(
        world.resource::<NovaOsCloseTransition>().return_to,
        PauseStates::Interface,
        "`:` over the interface must record the interface as the state to return to"
    );
    assert_eq!(
        the_named(world, INTERFACE_ROOT),
        baseline.root,
        "the interface root must outlive the modal, not be respawned"
    );
    assert_eq!(
        the_named(world, SHIP_CAMERA),
        baseline.camera,
        "the Ship scene must outlive the modal, not be rebuilt"
    );
    assert_eq!(
        *world.resource::<InterfacePaneType>(),
        InterfacePaneType::Ship
    );
    assert!(
        world
            .resource::<ClockFreeze>()
            .is_held_by(FreezeOwner::Interface),
        "the clocks must stay held under the modal"
    );
    assert_eq!(
        world.resource::<Time<Virtual>>().elapsed(),
        baseline.virtual_elapsed,
        "Time<Virtual> advanced under the modal"
    );
    nova_probe::probe_marker(
        world,
        "outcome: the modal covers a kept pane",
        serde_json::json!({ "return_to": "Interface" }),
    );
}

/// Escape closed the modal back to the SAME pane, with the same entities, and
/// the clocks never ran across the round trip.
#[cfg(feature = "debug")]
fn assert_escape_returns_to_pane(world: &mut World) {
    let baseline = world.resource::<Baseline>().clone();
    assert_eq!(
        pause_state(world),
        PauseStates::Interface,
        "Escape over the modal must return to the interface, not to flight"
    );
    assert_eq!(
        *world.resource::<InterfacePaneType>(),
        InterfacePaneType::Ship,
        "Escape must return to the pane the modal covered"
    );
    assert_eq!(the_named(world, INTERFACE_ROOT), baseline.root);
    assert_eq!(the_named(world, SHIP_CAMERA), baseline.camera);
    assert!(
        world
            .resource::<ClockFreeze>()
            .is_held_by(FreezeOwner::Interface),
        "the clocks must be held again on the frame the pane returns"
    );
    let clock = world.resource::<Time<Virtual>>();
    assert!(clock.is_paused());
    assert_eq!(
        clock.elapsed(),
        baseline.virtual_elapsed,
        "Time<Virtual> ran during the pane -> modal -> pane round trip"
    );
    nova_probe::probe_marker(
        world,
        "outcome: escape returns to the same pane",
        serde_json::json!({ "pane": "Ship" }),
    );
}

/// Nothing the pane or flight reads fired on the transition frames: the pane
/// camera pose and the ship's inputs and velocities are what they were.
#[cfg(feature = "debug")]
fn assert_nothing_fired(world: &mut World) {
    let baseline = world.resource::<Baseline>().clone();
    let pose = *world
        .get::<Transform>(baseline.camera)
        .expect("the Ship camera survived the round trip");
    assert_eq!(
        pose, baseline.camera_pose,
        "a pane action fired across the modal: the Ship camera moved"
    );
    let flight = flight_inputs(world);
    assert_eq!(
        flight, baseline.flight,
        "a flight action fired across the modal: the ship's inputs or velocity changed"
    );
    nova_probe::probe_marker(
        world,
        "outcome: nothing fires on the transition frames",
        serde_json::json!({}),
    );
}

/// The second Escape closed the pane to flight and released the clocks.
#[cfg(feature = "debug")]
fn assert_escape_leaves_the_pane(world: &mut World) {
    assert_eq!(
        pause_state(world),
        PauseStates::Unpaused,
        "Escape over the pane must return to flight"
    );
    let mut roots = world.query::<&Name>();
    let left = roots
        .iter(world)
        .filter(|name| name.as_str() == INTERFACE_ROOT)
        .count();
    assert_eq!(left, 0, "the closed interface must take its root with it");
    assert!(
        !world.resource::<Time<Virtual>>().is_paused(),
        "flight must run again once the interface is closed"
    );
    nova_probe::probe_marker(
        world,
        "outcome: escape closes the interface",
        serde_json::json!({}),
    );
}

fn setup_range(mut commands: Commands, game_assets: Res<GameAssets>, sections: Res<GameSections>) {
    commands.trigger(LoadScenario(computer::interface_range(
        &game_assets,
        &sections,
    )));
}
