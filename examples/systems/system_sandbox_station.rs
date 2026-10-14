//! system_sandbox_station: a hull built in the sandbox, marked as a station,
//! spawned as one and docked with.
//!
//! The walk is a builder's. Sandbox, Add Ship, and the flown ship gets a
//! controller and a docking port on its +Z face. A second ship (an AI ship,
//! because one ship is already flown) gets a controller and a thruster, and
//! its Station box is ticked from the inspector. The Station Check row then
//! names the missing port, and Play refuses on the status line, naming the
//! port and the thruster. The thruster comes off, the port goes on, the check
//! reads ready, the hull is turned to face the flown ship, and Play spawns it
//! from the station design the editor registered.
//!
//! | # | marker | claim |
//! | - | - | - |
//! | 1 | `outcome: a marked hull with no port reads as no station` | the Station box takes the mark and the Station Check row names the missing Docking section |
//! | 2 | `outcome: Play refuses a hull the station lint refuses` | Play leaves the editor up and the status line says the hull cannot be a station, naming the missing port and the thruster |
//! | 3 | `outcome: a marked hull with a port and a controller reads ready` | the Station Check row reads `ready` once the thruster is off and the port is on |
//! | 4 | `outcome: the marked hull spawns as a station` | the root carries the editor's station design, services, a port, a controller and no thruster, on the pose it was authored with |
//! | 5 | `outcome: the sandbox station holds still with no thrust` | no throttle under the root, and it stands at rest where it spawned |
//! | 6 | `outcome: the sandbox station levels` | nudged off its attitude, its own computer turns it back within the station tolerance |
//! | 7 | `outcome: the sandbox station admits a dock` | the flown ship docks at its port and the living station holds the helm |
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//! ```text
//! NOVA_AUTOPILOT=1 NOVA_AUTOPILOT_DEADLINE=1500 \
//!   cargo run --example system_sandbox_station --features debug
//! # look for: `sandbox station: ... spawned as a station`,
//! #           `sandbox station: docked, helm Station(..)`,
//! #           `autopilot: cycle complete, no panic`
//! ```

#[cfg(feature = "debug")]
#[path = "shared/editor_stage.rs"]
mod editor_stage;

#[cfg(feature = "debug")]
use std::sync::Arc;

#[cfg(feature = "debug")]
use avian3d::prelude::{AngularVelocity, LinearVelocity, Position, Rotation};
#[cfg(feature = "debug")]
use bevy::prelude::*;
use clap::Parser;
use nova_protocol::prelude::*;

#[cfg(feature = "debug")]
use crate::editor_stage::{ADD_MENU, EMPTY_SPACE};

#[derive(Parser)]
#[command(name = "system_sandbox_station")]
#[command(version = "1.0.0")]
#[command(about = "A sandbox hull marked as a station, refused while the lint refuses it, then spawned, levelled and docked with. Autopilot-only correctness range", long_about = None)]
struct Cli;

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();

    // The same editor app the game binary runs - not a bespoke copy.
    let mut app = editor_app(true, None);

    #[cfg(feature = "debug")]
    {
        app.add_plugins(nova_probe::NovaProbePlugin::default());
        app.init_resource::<StationEpoch>();
        app.add_plugins(nova_screenshot(station_script()));
    }

    app.run()
}

/// The script type, named once so the step list and its helpers agree.
#[cfg(feature = "debug")]
type Script = nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates>;

/// A wait the script holds on.
#[cfg(feature = "debug")]
type Wait = Arc<nova_protocol::nova_debug::harness::Predicate>;

/// In-step seconds the boot and the hand-off get: each is an asset-gated
/// scene load rather than a gesture, and a software renderer is slow at both.
#[cfg(feature = "debug")]
const LOAD_DEADLINE_SECS: f32 = 120.0;

/// The node id the second Add Ship mints: the ship the walk marks.
#[cfg(feature = "debug")]
const STATION_NODE: &str = "ship_2";

/// The design id Play registers the marked ship under: the node id, under
/// the sandbox's namespace.
#[cfg(feature = "debug")]
const STATION_DESIGN: &str = "editor_sandbox_ship_2";

/// The centre of a unit section's +Z face, in ship space, a hair inside it so
/// the aim lands on the face and not on its edge. The stage camera stands
/// above and behind (+Y, +Z), so this face is the one it sees.
#[cfg(feature = "debug")]
const STERN_FACE: Vec3 = Vec3::new(0.0, 0.0, 0.49);

/// The editor's key that walks which socket of the armed part does the
/// mating.
#[cfg(feature = "debug")]
const PLACEMENT_SOCKET_KEY: KeyCode = KeyCode::KeyF;

/// What the Station Check row reads when the hull can be a station.
#[cfg(feature = "debug")]
const STATION_READY: &str = "ready";

/// The yaw the station is turned by, degrees: half a turn, so its +Z port
/// faces the flown ship's +Z port across the berth.
#[cfg(feature = "debug")]
const STATION_YAW_DEG: f32 = 180.0;

/// The gap between the two port faces at the berth, engine units: half a
/// cell, inside the one-cell capture distance.
#[cfg(feature = "debug")]
const BERTH_GAP: f32 = 0.5;

/// How close a teleported hull must settle to its berth before a dock is
/// asked for, engine units, and how slow it must be.
#[cfg(feature = "debug")]
const BERTH_TOLERANCE: f32 = 0.05;
#[cfg(feature = "debug")]
const BERTH_REST: f32 = 0.01;

/// Sim seconds the station is watched for drift before it is nudged: long
/// enough that a live drive would have moved it well past [`REST_DRIFT`].
/// Sim time rather than frames: a software renderer draws the seeded range's
/// rock field at a few frames a second.
#[cfg(feature = "debug")]
const WATCH_SECS: f32 = 1.5;

/// Sim seconds the docked pair is left before the picture, so the collar
/// has closed.
#[cfg(feature = "debug")]
const COLLAR_SECS: f32 = 1.0;

/// Real seconds a beat in the flown range gets. Generous for the same
/// reason as [`WATCH_SECS`]: the range is slow to draw on a software
/// renderer, and a deadline is a backstop, not a settle.
#[cfg(feature = "debug")]
const RANGE_DEADLINE_SECS: f32 = 240.0;

/// How far the station may stand from its spawn point after the watch,
/// engine units, and how fast it may still be moving.
#[cfg(feature = "debug")]
const REST_DRIFT: f32 = 0.05;
#[cfg(feature = "debug")]
const REST_SPEED: f32 = 0.01;

/// How far the nudge turns the hull, radians, and the spin it leaves it
/// with, radians per second. The displacement is what asks the hold to
/// recover anything; a spin alone dies inside a frame.
#[cfg(feature = "debug")]
const NUDGE_RAD: f32 = 0.5;
#[cfg(feature = "debug")]
const NUDGE_RAD_PER_SEC: f32 = 0.3;

/// How far off its spawn attitude the hull must be before the hold is asked
/// to recover it. Above [`STATION_LEVEL_TOLERANCE_RAD`], so a hull that never
/// moved cannot pass for free.
#[cfg(feature = "debug")]
const TURNED_RAD: f32 = 0.1;

/// The angular speed under which the hull counts as at rest.
#[cfg(feature = "debug")]
const REST_RAD_PER_SEC: f32 = 0.01;

/// What the walk remembers between beats.
#[cfg(feature = "debug")]
#[derive(Resource, Default)]
struct StationEpoch {
    /// The station root, its pose when first read, engine units.
    station: Option<Entity>,
    spawn_position: Vec3,
    spawn_rotation: Quat,
}

/// The walk.
#[cfg(feature = "debug")]
fn station_script() -> Script {
    Script::new()
        .step("sandbox station: reach the main menu")
        .until(state_is(GameStates::MainMenu))
        .deadline(LOAD_DEADLINE_SECS)
        .add()
        .click("sandbox station: click Sandbox", "Sandbox Button")
        .step("sandbox station: the editor is up")
        .until(and(
            state_is(GameStates::Playing),
            ui_node_present(ADD_MENU),
        ))
        .deadline(LOAD_DEADLINE_SECS)
        .add()
        // The flown ship: a controller and a port on its stern (+Z) face.
        .click("sandbox station: open the Add menu", ADD_MENU)
        .click("sandbox station: add the flown ship", "Add Ship Button")
        .step("sandbox station: inside the blank flown ship")
        .until(inside_with(0))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .arm(
            "sandbox station: arm the flown ship's controller",
            BASIC_CONTROLLER_SECTION_ID,
        )
        .found("sandbox station: found the flown ship")
        .arm(
            "sandbox station: arm the flown ship's port",
            DOCKING_PORT_SECTION_ID,
        )
        // The port has no socket on its hatch face, so none of its sockets
        // faces the hull and the natural mate is a side one, the hatch turned
        // along the face. One F walks the mate to its back socket, hatch out.
        .place_port_on_the_face(
            "sandbox station: put the port on the flown ship",
            STERN_FACE,
            1,
            2,
        )
        .put_down("sandbox station: put the flown ship's tool down")
        .leave("sandbox station: leave the flown ship")
        // The hull to mark: a controller and a thruster, no port yet.
        .click("sandbox station: open the Add menu again", ADD_MENU)
        .click("sandbox station: add the hull to mark", "Add Ship Button")
        .step("sandbox station: inside the blank second ship")
        .until(inside_with(0))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("sandbox station: the second ship is the one the walk marks")
        .on_enter(|world: &mut World| {
            let probe = world.resource::<EditorProbe>();
            assert_eq!(
                probe.inside.as_deref(),
                Some(STATION_NODE),
                "the second Add Ship mints the node the walk marks"
            );
        })
        .add()
        .arm(
            "sandbox station: arm the hull's controller",
            BASIC_CONTROLLER_SECTION_ID,
        )
        .found("sandbox station: found the hull")
        .arm(
            "sandbox station: arm the hull's thruster",
            BASIC_THRUSTER_SECTION_ID,
        )
        .place_on_the_face(
            "sandbox station: put the thruster on the hull",
            STERN_FACE,
            0,
            2,
        )
        .put_down("sandbox station: put the hull's tool down")
        .leave("sandbox station: leave the hull")
        .select_the_hull("sandbox station: select the hull")
        .step("sandbox station: the hull is an AI ship and not yet a station")
        .on_enter(|world: &mut World| {
            assert_eq!(inspector_reading(world, "Driver"), "AI");
            assert_eq!(inspector_reading(world, "Station"), "false");
            assert!(
                !inspector_has(world, "Station Check"),
                "an unmarked ship shows no check row"
            );
        })
        .add()
        .click("sandbox station: tick Station", "Inspector Flag Station")
        .step("sandbox station: the mark took and the check names the fault")
        .until(and(
            editor_inspector_reads("Station", "true"),
            the_check_reads_something(),
        ))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("sandbox station: assert the check refuses a hull with no port")
        .on_enter(|world: &mut World| {
            let check = inspector_reading(world, "Station Check");
            assert!(
                check.contains("no Docking section"),
                "the check names the missing port: {check:?}"
            );
            info!("sandbox station: a hull with no port reads {check:?}");
            nova_probe::probe_marker(
                world,
                "outcome: a marked hull with no port reads as no station",
                serde_json::json!({ "check": check }),
            );
        })
        .add()
        .click(
            "sandbox station: press Play on a hull with no port",
            "Play Button",
        )
        .step("sandbox station: Play is refused on the status line")
        .until(the_status_says("cannot be a station"))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("sandbox station: shoot the refusal")
        .on_enter(|world: &mut World| {
            assert!(
                world
                    .try_query_filtered::<(), With<PlayerSpaceshipMarker>>()
                    .is_none_or(|mut flown| flown.iter(world).next().is_none()),
                "a refused Play must not hand off"
            );
            let status = world.resource::<EditorProbe>().status.clone();
            assert!(
                status.contains("no Docking section") && status.contains("is a Thruster"),
                "the refusal names the missing port and the thruster: {status:?}"
            );
            info!("sandbox station: Play refused: {status:?}");
            nova_probe::probe_marker(
                world,
                "outcome: Play refuses a hull the station lint refuses",
                serde_json::json!({ "status": status }),
            );
            shoot(world, "sandbox-station-refused.png");
        })
        .until(shot_written("sandbox-station-refused.png"))
        .deadline(SHOT_DEADLINE_SECS)
        .add()
        // Back in: the thruster comes off, and the port goes on in its place.
        .double_click_named(
            "sandbox station: enter the hull through the tree",
            &format!("Scene Row {STATION_NODE}"),
            BEAT_DEADLINE_SECS,
        )
        .step("sandbox station: inside the hull again")
        .until(inside_with(2))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("sandbox station: point at the thruster")
        .on_enter(|world: &mut World| {
            let at = thruster_aim(world).expect("the thruster's face is on screen");
            move_cursor(at)(world);
        })
        .add()
        .step("sandbox station: click the thruster")
        .on_enter(press_mouse(MouseButton::Left))
        .add()
        .step("sandbox station: release on the thruster")
        .on_enter(release_mouse(MouseButton::Left))
        .until(the_thruster_is_selected())
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("sandbox station: press Del on the thruster")
        .on_enter(press_key(KeyCode::Delete))
        .until(inside_with(1))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("sandbox station: release Del")
        .on_enter(release_key(KeyCode::Delete))
        .add()
        .arm(
            "sandbox station: arm the hull's port",
            DOCKING_PORT_SECTION_ID,
        )
        // No F: the editor keeps the socket choice from the last placement,
        // so the port arms hatch out already.
        .place_port_on_the_face(
            "sandbox station: put the port on the hull",
            STERN_FACE,
            0,
            2,
        )
        .put_down("sandbox station: put the hull's port tool down")
        .leave("sandbox station: leave the finished hull")
        .select_the_hull("sandbox station: select the finished hull")
        .step("sandbox station: the check reads ready")
        .until(editor_inspector_reads("Station Check", STATION_READY))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        // Turned half a turn, so its stern port faces the flown ship's.
        .click(
            "sandbox station: reach for the hull's yaw",
            "Inspector Field Rotation Y",
        )
        .step("sandbox station: the yaw box has the caret")
        .until(editor_field_focused())
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("sandbox station: type the yaw")
        .on_enter(|world: &mut World| {
            press_edit_key(Key::End)(world);
            for _ in 0..12 {
                press_edit_key(Key::Backspace)(world);
            }
            type_text(format!("{STATION_YAW_DEG}"))(world);
            press_edit_key(Key::Enter)(world);
        })
        .until(the_yaw_reads_half_a_turn())
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("sandbox station: shoot the marked hull")
        .on_enter(|world: &mut World| {
            let check = inspector_reading(world, "Station Check");
            assert_eq!(check, STATION_READY);
            info!("sandbox station: the finished hull reads {check:?}");
            nova_probe::probe_marker(
                world,
                "outcome: a marked hull with a port and a controller reads ready",
                serde_json::json!({ "check": check }),
            );
            shoot(world, "sandbox-station-toggle.png");
        })
        .until(shot_written("sandbox-station-toggle.png"))
        .deadline(SHOT_DEADLINE_SECS)
        .add()
        .click("sandbox station: press Play", "Play Button")
        .step("sandbox station: the range is up with the station in it")
        .until(and(
            scenario_is_built(),
            and(player_ship_present(), station_up()),
        ))
        .deadline(LOAD_DEADLINE_SECS)
        .add()
        .step("sandbox station: assert the hull spawned as a station")
        .on_enter(assert_spawned_as_station)
        .add()
        .step("sandbox station: watch the station stand")
        .until(elapsed(WATCH_SECS))
        .deadline(RANGE_DEADLINE_SECS)
        .add()
        .step("sandbox station: assert it holds still with no thrust")
        .on_enter(assert_no_thrust)
        .add()
        .step("sandbox station: nudge it off its attitude")
        .on_enter(nudge_station)
        .until(station_turned())
        .deadline(RANGE_DEADLINE_SECS)
        .add()
        .step("sandbox station: wait for the spin to die")
        .until(station_settled())
        .deadline(RANGE_DEADLINE_SECS)
        .add()
        .step("sandbox station: assert it levelled")
        .on_enter(assert_levelled)
        .add()
        .step("sandbox station: berth the flown ship at its port")
        .on_enter(berth_player)
        .until(player_berthed())
        .deadline(RANGE_DEADLINE_SECS)
        .diagnose(describe_ports)
        .add()
        .step("sandbox station: dock")
        .each(|world, _, _| request_dock(world))
        .until(player_docked())
        .deadline(RANGE_DEADLINE_SECS)
        .diagnose(describe_ports)
        .add()
        .step("sandbox station: assert the station admitted the dock")
        .on_enter(assert_docked)
        .add()
        .step("sandbox station: let the collar close")
        .until(elapsed(COLLAR_SECS))
        .deadline(RANGE_DEADLINE_SECS)
        .add()
        .step("sandbox station: shoot the docked pair")
        .on_enter(|world: &mut World| shoot(world, "sandbox-station-docked.png"))
        .until(shot_written("sandbox-station-docked.png"))
        .deadline(SHOT_DEADLINE_SECS)
        .add()
}

/// The gestures the walk is written in. Each one acts and then holds until
/// the editor did what the act asks for.
#[cfg(feature = "debug")]
trait SandboxGestures {
    /// Click a named widget; the caller's next beat waits on its effect.
    fn click(self, label: &str, name: &str) -> Self;
    /// Arm `prototype` through the gallery by keyboard.
    fn arm(self, label: &str, prototype: &str) -> Self;
    /// Found the blank ship with the armed part, on empty space.
    fn found(self, label: &str) -> Self;
    /// Place the armed part on the face at `socket`, in ship space, after
    /// `turns` presses of the socket key, and wait for the ship to hold
    /// `sections`.
    fn place_on_the_face(self, label: &str, socket: Vec3, turns: usize, sections: usize) -> Self;
    /// [`Self::place_on_the_face`] for the docking port: the placed port's
    /// hatch must face out of that face.
    fn place_port_on_the_face(
        self,
        label: &str,
        socket: Vec3,
        turns: usize,
        sections: usize,
    ) -> Self;
    /// Escape puts the armed part down.
    fn put_down(self, label: &str) -> Self;
    /// Out to the scenario node through the tree's root row.
    fn leave(self, label: &str) -> Self;
    /// One click on the marked hull's tree row: selected, so the inspector
    /// shows its ship rows.
    fn select_the_hull(self, label: &str) -> Self;
}

#[cfg(feature = "debug")]
impl SandboxGestures for Script {
    fn click(self, label: &str, name: &str) -> Self {
        self.click_named(label, name, pointer_released(), BEAT_DEADLINE_SECS)
    }

    fn arm(self, label: &str, prototype: &str) -> Self {
        let filter = prototype.to_string();
        let narrowed = prototype.to_string();
        let armed = prototype.to_string();
        self.step(format!("{label}: open the gallery"))
            .on_enter(press_key(KeyCode::Tab))
            .until(editor_gallery_open())
            .deadline(BEAT_DEADLINE_SECS)
            .add()
            .step(format!("{label}: release Tab"))
            .on_enter(release_key(KeyCode::Tab))
            .add()
            .step(format!("{label}: put the caret in the filter"))
            .on_enter(press_key(KeyCode::Slash))
            .until(editor_filter_focused())
            .deadline(BEAT_DEADLINE_SECS)
            .add()
            .step(format!("{label}: release /"))
            .on_enter(release_key(KeyCode::Slash))
            .add()
            .step(format!("{label}: filter to `{prototype}`"))
            .on_enter(move |world: &mut World| type_text(filter.clone())(world))
            .until(editor_gallery_selected(narrowed))
            .deadline(BEAT_DEADLINE_SECS)
            .add()
            .step(format!("{label}: Enter to focus"))
            .on_enter(press_key(KeyCode::Enter))
            .until(ui_node_present("Gallery Focus Card"))
            .deadline(BEAT_DEADLINE_SECS)
            .add()
            .step(format!("{label}: release Enter"))
            .on_enter(release_key(KeyCode::Enter))
            .add()
            .step(format!("{label}: Enter to take the part"))
            .on_enter(press_key(KeyCode::Enter))
            .until(and(
                editor_gallery_closed(),
                editor_tool_is(EditorTool::Place(armed)),
            ))
            .deadline(BEAT_DEADLINE_SECS)
            .add()
            .step(format!("{label}: release Enter again"))
            .on_enter(release_key(KeyCode::Enter))
            .add()
    }

    fn found(self, label: &str) -> Self {
        self.step(format!("{label}: point at empty space"))
            .on_enter(move_cursor(EMPTY_SPACE))
            .until(and(pointer_at(EMPTY_SPACE), editor_placement_clear()))
            .deadline(BEAT_DEADLINE_SECS)
            .add()
            .step(format!("{label}: press"))
            .on_enter(press_mouse(MouseButton::Left))
            .until(inside_with(1))
            .deadline(BEAT_DEADLINE_SECS)
            .add()
            .step(format!("{label}: release"))
            .on_enter(release_mouse(MouseButton::Left))
            .until(pointer_released())
            .deadline(BEAT_DEADLINE_SECS)
            .add()
    }

    fn place_port_on_the_face(
        self,
        label: &str,
        socket: Vec3,
        turns: usize,
        sections: usize,
    ) -> Self {
        let face_label = label.to_string();
        self.place_on_the_face(label, socket, turns, sections)
            .step(format!("{label}: the hatch faces out"))
            .on_enter(move |world: &mut World| {
                let hatch = hatch_axis(world).expect("the ship holds a docking port");
                assert!(
                    hatch.dot(socket.normalize()) > 0.99,
                    "{face_label}: the port's hatch must face out of the face, \
                     it faces {hatch:?} in the ship's frame"
                );
            })
            .add()
    }

    fn place_on_the_face(self, label: &str, socket: Vec3, turns: usize, sections: usize) -> Self {
        let mut script = self
            .step(format!("{label}: aim"))
            .on_enter(move |world: &mut World| {
                let origin = world
                    .resource::<EditorProbe>()
                    .inside_origin
                    .unwrap_or(Vec3::ZERO);
                let at = aim_at_world(world, origin + socket).expect("that face is on screen");
                move_cursor(at)(world);
            })
            .until(editor_placement_solved())
            .deadline(BEAT_DEADLINE_SECS)
            .add();
        for turn in 1..=turns {
            script = script
                .step(format!("{label}: F walks the mating socket ({turn})"))
                .on_enter(press_key(PLACEMENT_SOCKET_KEY))
                .add()
                .step(format!("{label}: release F ({turn})"))
                .on_enter(release_key(PLACEMENT_SOCKET_KEY))
                .until(editor_placement_solved())
                .deadline(BEAT_DEADLINE_SECS)
                .add();
        }
        script
            .step(format!("{label}: press"))
            .on_enter(press_mouse(MouseButton::Left))
            .until(inside_with(sections))
            .deadline(BEAT_DEADLINE_SECS)
            .add()
            .step(format!("{label}: release"))
            .on_enter(release_mouse(MouseButton::Left))
            .until(pointer_released())
            .deadline(BEAT_DEADLINE_SECS)
            .add()
    }

    fn put_down(self, label: &str) -> Self {
        self.step(format!("{label}: press Escape"))
            .on_enter(press_key(KeyCode::Escape))
            .until(editor_tool_is(EditorTool::Select))
            .deadline(BEAT_DEADLINE_SECS)
            .add()
            .step(format!("{label}: release Escape"))
            .on_enter(release_key(KeyCode::Escape))
            .add()
    }

    fn leave(self, label: &str) -> Self {
        self.click(label, "Scene Row scenario")
            .step(format!("{label}: at the scenario node"))
            .until(at_the_scenario_node())
            .deadline(BEAT_DEADLINE_SECS)
            .add()
    }

    fn select_the_hull(self, label: &str) -> Self {
        self.click(label, &format!("Scene Row {STATION_NODE}"))
            .step(format!("{label}: the inspector is on it"))
            .until(the_hull_is_inspected())
            .deadline(BEAT_DEADLINE_SECS)
            .add()
    }
}

/// The viewport aim for a world point, logical pixels.
#[cfg(feature = "debug")]
fn aim_at_world(world: &mut World, point: Vec3) -> Option<Vec2> {
    let camera_entity = world
        .query_filtered::<Entity, With<Camera3d>>()
        .iter(world)
        .next()?;
    let camera = world.get::<Camera>(camera_entity)?.clone();
    let camera_transform = *world.get::<GlobalTransform>(camera_entity)?;
    camera
        .world_to_viewport(&camera_transform, point)
        .ok()
        .filter(|aim: &Vec2| aim.x.is_finite() && aim.y.is_finite())
}

/// The way the edited ship's docking port's hatch points, in the ship's frame:
/// the port docks through its local -Z face.
#[cfg(feature = "debug")]
fn hatch_axis(world: &World) -> Option<Vec3> {
    world
        .resource::<EditorProbe>()
        .ship
        .iter()
        .find(|section| section.prototype == DOCKING_PORT_SECTION_ID)
        .map(|section| section.rotation * Vec3::NEG_Z)
}

/// The edited ship's thruster section, as the probe reports it.
#[cfg(feature = "debug")]
fn thruster_of(world: &World) -> Option<&EditorSection> {
    world
        .resource::<EditorProbe>()
        .ship
        .iter()
        .find(|section| section.prototype == BASIC_THRUSTER_SECTION_ID)
}

/// The viewport aim for the centre of the thruster's +Z face, the face the
/// stage camera sees.
#[cfg(feature = "debug")]
fn thruster_aim(world: &mut World) -> Option<Vec2> {
    let origin = world
        .resource::<EditorProbe>()
        .inside_origin
        .unwrap_or(Vec3::ZERO);
    let centre = thruster_of(world)?.position;
    aim_at_world(world, origin + centre + STERN_FACE)
}

/// Advance once the click marked the thruster.
#[cfg(feature = "debug")]
fn the_thruster_is_selected() -> Wait {
    Arc::new(|world: &World| {
        let marked = world.resource::<EditorProbe>().selected_node.clone();
        thruster_of(world).is_some_and(|thruster| marked.as_deref() == Some(thruster.id.as_str()))
    })
}

/// Advance once the editor is inside a ship holding `sections` sections.
#[cfg(feature = "debug")]
fn inside_with(sections: usize) -> Wait {
    Arc::new(move |world: &World| {
        world
            .get_resource::<EditorProbe>()
            .is_some_and(|probe| probe.inside.is_some() && probe.ship.len() == sections)
    })
}

/// Advance once the editor is out at the scenario node, where Play lives.
#[cfg(feature = "debug")]
fn at_the_scenario_node() -> Wait {
    Arc::new(|world: &World| {
        let probe = world.resource::<EditorProbe>();
        probe.inside.is_none() && probe.can_play
    })
}

/// Advance once the marked hull is selected and its Station row is up.
#[cfg(feature = "debug")]
fn the_hull_is_inspected() -> Wait {
    Arc::new(|world: &World| {
        let probe = world.resource::<EditorProbe>();
        probe.selected_node.as_deref() == Some(STATION_NODE)
            && probe.inspector.iter().any(|(row, _)| row == "Station")
    })
}

/// Advance once the Station Check row is up with a reading.
#[cfg(feature = "debug")]
fn the_check_reads_something() -> Wait {
    Arc::new(|world: &World| !inspector_reading(world, "Station Check").is_empty())
}

/// Advance once the status line carries `wanted`.
#[cfg(feature = "debug")]
fn the_status_says(wanted: &'static str) -> Wait {
    Arc::new(move |world: &World| world.resource::<EditorProbe>().status.contains(wanted))
}

/// Advance once the Rotation row's yaw reads half a turn, either sign.
#[cfg(feature = "debug")]
fn the_yaw_reads_half_a_turn() -> Wait {
    Arc::new(|world: &World| {
        inspector_reading(world, "Rotation")
            .split(", ")
            .next()
            .and_then(|yaw| yaw.parse::<f32>().ok())
            .is_some_and(|yaw| (yaw.abs() - STATION_YAW_DEG).abs() < 0.5)
    })
}

/// The inspector row `label`'s reading, or empty when no such row is up.
#[cfg(feature = "debug")]
fn inspector_reading(world: &World, label: &str) -> String {
    world
        .resource::<EditorProbe>()
        .inspector
        .iter()
        .find(|(row, _)| row == label)
        .map(|(_, reading)| reading.clone())
        .unwrap_or_default()
}

/// Whether the inspector shows a row `label`.
#[cfg(feature = "debug")]
fn inspector_has(world: &World, label: &str) -> bool {
    world
        .resource::<EditorProbe>()
        .inspector
        .iter()
        .any(|(row, _)| row == label)
}

/// The station root the sandbox spawned, or `None` while none is up.
#[cfg(feature = "debug")]
fn station_root(world: &World) -> Option<Entity> {
    let mut stations = world.try_query_filtered::<(Entity, &EntityId), With<StationMarker>>()?;
    stations
        .iter(world)
        .find(|(_, id)| id.0 == STATION_NODE)
        .map(|(entity, _)| entity)
}

/// The flown ship's root, or `None` while none is up.
#[cfg(feature = "debug")]
fn player_root(world: &World) -> Option<Entity> {
    let mut players = world.try_query_filtered::<Entity, With<PlayerSpaceshipMarker>>()?;
    players.iter(world).next()
}

/// The station stands with a physics pose and a port.
#[cfg(feature = "debug")]
fn station_up() -> Wait {
    Arc::new(|world: &World| {
        station_root(world).is_some_and(|station| {
            world.get::<Position>(station).is_some() && port_of(world, station).is_some()
        })
    })
}

/// Which section kinds stand under `root`: a Docking, a Controller and a
/// Thruster section.
#[cfg(feature = "debug")]
fn section_kinds(world: &World, root: Entity) -> (bool, bool, bool) {
    let mut found = (false, false, false);
    for child in world
        .entity(root)
        .get::<Children>()
        .into_iter()
        .flat_map(|children| children.iter())
    {
        if let Some(config) = world.entity(child).get::<SectionBuildConfig>() {
            match config.0.kind {
                SectionKind::Docking(_) => found.0 = true,
                SectionKind::Controller(_) => found.1 = true,
                SectionKind::Thruster(_) => found.2 = true,
                _ => {}
            }
        }
    }
    found
}

/// Claim 4: the root is a station of the editor's design, on the pose the
/// hull was authored with.
#[cfg(feature = "debug")]
fn assert_spawned_as_station(world: &mut World) {
    let station = station_root(world).expect("sandbox station: the station is up");
    let design = world
        .get::<StationMarker>(station)
        .expect("sandbox station: the root carries its marker")
        .0
        .to_string();
    assert_eq!(
        design, STATION_DESIGN,
        "sandbox station: the root spawned from the design the editor registered"
    );
    let services = world
        .get::<StationServices>(station)
        .expect("sandbox station: the station offers services")
        .0
        .clone();
    assert!(
        !services.is_empty(),
        "sandbox station: the station offers no service"
    );
    let (docking, controller, thruster) = section_kinds(world, station);
    assert!(docking, "sandbox station: the station has no port");
    assert!(controller, "sandbox station: the station has no controller");
    assert!(!thruster, "sandbox station: the station has a thruster");
    assert!(
        !world.entity(station).contains::<PlayerSpaceshipMarker>(),
        "sandbox station: a station is nobody's ship"
    );
    let position = world
        .get::<Position>(station)
        .expect("sandbox station: the station has a physics position")
        .0;
    let rotation = world
        .get::<Rotation>(station)
        .expect("sandbox station: the station has a physics rotation")
        .0;
    // The authored half turn: its stern (+Z) points down world -Z.
    let stern = rotation * Vec3::Z;
    assert!(
        stern.dot(Vec3::NEG_Z) > 0.99,
        "sandbox station: the station stands on its authored yaw, stern {stern:?}"
    );
    let elapsed = world.resource::<Time>().elapsed_secs();
    info!(
        "sandbox station: {STATION_NODE} spawned as a station of '{design}' at {position}, services {services:?}"
    );
    nova_probe::probe_marker(
        world,
        "outcome: the marked hull spawns as a station",
        serde_json::json!({ "t": elapsed, "design": design, "services": services.len() }),
    );
    let mut epoch = world.resource_mut::<StationEpoch>();
    epoch.station = Some(station);
    epoch.spawn_position = position;
    epoch.spawn_rotation = rotation;
}

/// Claim 5: no throttle under the root, and it stood at rest through the
/// watch.
#[cfg(feature = "debug")]
fn assert_no_thrust(world: &mut World) {
    let epoch = world.resource::<StationEpoch>();
    let (station, spawn) = (
        epoch
            .station
            .expect("sandbox station: the station was read"),
        epoch.spawn_position,
    );
    let throttles = world
        .try_query::<(&ChildOf, &ThrusterSectionInput)>()
        .map_or(0, |mut throttles| {
            throttles
                .iter(world)
                .filter(|(parent, _)| parent.0 == station)
                .count()
        });
    assert_eq!(
        throttles, 0,
        "sandbox station: the station carries a throttle"
    );
    let drift = world
        .get::<Position>(station)
        .map_or(f32::INFINITY, |position| position.0.distance(spawn));
    let speed = world
        .get::<LinearVelocity>(station)
        .map_or(f32::INFINITY, |velocity| velocity.0.length());
    assert!(
        drift < REST_DRIFT && speed < REST_SPEED,
        "sandbox station: the station moved with no thruster: drift {drift:.3}, speed {speed:.3}"
    );
    let elapsed = world.resource::<Time>().elapsed_secs();
    info!("sandbox station: held still with no thrust (drift {drift:.4}, speed {speed:.4})");
    nova_probe::probe_marker(
        world,
        "outcome: the sandbox station holds still with no thrust",
        serde_json::json!({ "t": elapsed, "drift": drift, "speed": speed }),
    );
}

/// Turn the station about +Y and leave it spinning.
#[cfg(feature = "debug")]
fn nudge_station(world: &mut World) {
    let epoch = world.resource::<StationEpoch>();
    let (station, spawn) = (
        epoch
            .station
            .expect("sandbox station: the station was read"),
        epoch.spawn_rotation,
    );
    world.entity_mut(station).insert((
        Rotation(spawn * Quat::from_rotation_y(NUDGE_RAD)),
        AngularVelocity(Vec3::new(0.0, NUDGE_RAD_PER_SEC, 0.0)),
    ));
    info!("sandbox station: nudged {NUDGE_RAD} rad off spawn at {NUDGE_RAD_PER_SEC} rad/s");
}

/// Angle between the station's attitude and its spawn attitude.
#[cfg(feature = "debug")]
fn attitude_error(world: &World) -> Option<f32> {
    let epoch = world.get_resource::<StationEpoch>()?;
    let rotation = world.get::<Rotation>(epoch.station?)?;
    Some(rotation.0.angle_between(epoch.spawn_rotation))
}

/// The nudge took.
#[cfg(feature = "debug")]
fn station_turned() -> Wait {
    Arc::new(|world: &World| attitude_error(world).is_some_and(|error| error > TURNED_RAD))
}

/// The spin died. Where it came to rest is the assertion's question.
#[cfg(feature = "debug")]
fn station_settled() -> Wait {
    Arc::new(|world: &World| {
        let Some(station) = world
            .get_resource::<StationEpoch>()
            .and_then(|epoch| epoch.station)
        else {
            return false;
        };
        world
            .get::<AngularVelocity>(station)
            .is_some_and(|spin| spin.0.length() < REST_RAD_PER_SEC)
    })
}

/// Claim 6: the hull came to rest back on its spawn attitude.
#[cfg(feature = "debug")]
fn assert_levelled(world: &mut World) {
    let error = attitude_error(world).expect("sandbox station: the station is up");
    assert!(
        error < STATION_LEVEL_TOLERANCE_RAD,
        "sandbox station: the hull came to rest {error:.3} rad off its spawn attitude \
         (tolerance {STATION_LEVEL_TOLERANCE_RAD})"
    );
    let elapsed = world.resource::<Time>().elapsed_secs();
    info!("sandbox station: levelled ({error:.4} rad off spawn)");
    nova_probe::probe_marker(
        world,
        "outcome: the sandbox station levels",
        serde_json::json!({ "t": elapsed, "error_rad": error }),
    );
}

/// The first port under `root`: its collar face and the way it points, in
/// world space.
#[cfg(feature = "debug")]
fn port_of(world: &World, root: Entity) -> Option<(Vec3, Vec3)> {
    let mut ports =
        world.try_query_filtered::<(&ChildOf, &GlobalTransform), With<DockingSectionMarker>>()?;
    ports
        .iter(world)
        .find(|(parent, _)| parent.0 == root)
        .map(|(_, global)| {
            let (_, rotation, translation) = global.to_scale_rotation_translation();
            (
                translation + rotation * Vec3::new(0.0, 0.0, -0.5),
                rotation * Vec3::NEG_Z,
            )
        })
}

/// Where the flown ship's root parks: its port face [`BERTH_GAP`] out from
/// the station's, on the attitude it holds now.
#[cfg(feature = "debug")]
fn berth(world: &World) -> Option<Vec3> {
    let station = station_root(world)?;
    let player = player_root(world)?;
    let (station_face, station_axis) = port_of(world, station)?;
    let (player_face, _) = port_of(world, player)?;
    let root = world.get::<Position>(player)?.0;
    Some(station_face + station_axis * BERTH_GAP - (player_face - root))
}

/// Park the flown ship on the berth, at rest.
#[cfg(feature = "debug")]
fn berth_player(world: &mut World) {
    let station = station_root(world).expect("sandbox station: the station is up");
    let player = player_root(world).expect("sandbox station: the flown ship is up");
    let (_, station_axis) = port_of(world, station).expect("sandbox station: the station's port");
    let (_, player_axis) = port_of(world, player).expect("sandbox station: the flown ship's port");
    assert!(
        player_axis.dot(station_axis) < -0.99,
        "sandbox station: the two ports must face each other: {player_axis:?} vs {station_axis:?}"
    );
    let at = berth(world).expect("sandbox station: a berth");
    let rotation = world
        .get::<Rotation>(player)
        .expect("sandbox station: the flown ship has a rotation")
        .0;
    world.entity_mut(player).insert((
        Position(at),
        Rotation(rotation),
        LinearVelocity(Vec3::ZERO),
        AngularVelocity(Vec3::ZERO),
    ));
    info!("sandbox station: parked the flown ship on the berth at {at}");
}

/// The flown ship stands on the berth, at rest.
#[cfg(feature = "debug")]
fn player_berthed() -> Wait {
    Arc::new(|world: &World| {
        let Some(player) = player_root(world) else {
            return false;
        };
        let (Some(position), Some(velocity), Some(at)) = (
            world.get::<Position>(player),
            world.get::<LinearVelocity>(player),
            berth(world),
        ) else {
            return false;
        };
        position.0.distance(at) < BERTH_TOLERANCE && velocity.0.length() < BERTH_REST
    })
}

/// Every port's face and axis, and the flown ship's motion: what a stall on
/// the berth or the dock needs to say.
#[cfg(feature = "debug")]
fn describe_ports(world: &World) -> String {
    let mut lines = Vec::new();
    for (name, root) in [
        ("player", player_root(world)),
        ("station", station_root(world)),
    ] {
        let Some(root) = root else {
            lines.push(format!("{name}: not up"));
            continue;
        };
        lines.push(format!(
            "{name} {root:?}: pos {:?} vel {:?} port {:?} docked {}",
            world.get::<Position>(root).map(|p| p.0),
            world.get::<LinearVelocity>(root).map(|v| v.0),
            port_of(world, root),
            world.entity(root).contains::<DockedShip>()
        ));
    }
    lines.push(format!("berth {:?}", berth(world)));
    lines.join("\n")
}

/// Ask for a dock, as `DOCK` does, until one lands.
#[cfg(feature = "debug")]
fn request_dock(world: &mut World) {
    let player = player_root(world).expect("sandbox station: the flown ship is up");
    if world.get::<DockedShip>(player).is_some() {
        return;
    }
    let station = station_root(world).expect("sandbox station: the station is up");
    world.trigger(DockingConnectionRequest {
        entity: player,
        target: station,
    });
}

/// The flown ship holds a connection.
#[cfg(feature = "debug")]
fn player_docked() -> Wait {
    Arc::new(|world: &World| {
        player_root(world).is_some_and(|player| world.get::<DockedShip>(player).is_some())
    })
}

/// Claim 7: the dock opened, and the living station holds the helm.
#[cfg(feature = "debug")]
fn assert_docked(world: &mut World) {
    let player = player_root(world).expect("sandbox station: the flown ship is up");
    let station = station_root(world).expect("sandbox station: the station is up");
    let docked = *world
        .get::<DockedShip>(player)
        .expect("sandbox station: the flown ship is docked");
    let connection = *world
        .get::<DockingConnection>(docked.connection)
        .expect("sandbox station: the connection is live");
    assert!(
        connection.first_ship == station || connection.second_ship == station,
        "sandbox station: the dock is with the station"
    );
    assert_eq!(
        connection.helm,
        DockedHelmType::Station(station),
        "sandbox station: a living station holds the helm"
    );
    let elapsed = world.resource::<Time>().elapsed_secs();
    info!("sandbox station: docked, helm {:?}", connection.helm);
    nova_probe::probe_marker(
        world,
        "outcome: the sandbox station admits a dock",
        serde_json::json!({ "t": elapsed }),
    );
}
