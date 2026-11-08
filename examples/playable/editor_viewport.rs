//! editor_viewport: the editor's viewport, to navigate and edit by hand.
//!
//! Boots the shipped app and opens a scratch mod past the Mod Hub, on a
//! profile folder of its own.
//!
//! Try: hold RMB and move the mouse to look about. Hold MMB or Alt+LMB to orbit
//! the selection, Alt+RMB or the wheel to dolly, Shift+MMB to pan. `F` frames
//! the selection.
//!
//! `Q` `W` `E` `R` pick select, move, turn and scale, and `X` swaps world and
//! local axes. Hold Ctrl on a handle to snap. Click empty space to
//! deselect, Ctrl+click to add, `Ctrl+A` for everything, and drag empty space
//! with the select tool for a box. `Ctrl+D`, `Ctrl+C` and `Ctrl+V` copy, `Del`
//! deletes, and `Ctrl+Z` takes any of it back.
//!
//! The panel reads the tool, the axes, the gesture and the selection.
//!
//! With `NOVA_AUTOPILOT=1` a smoke walk does these gestures through the real
//! keys and buttons, and asserts the document and the camera after each.
//! `NOVA_CAPTURE=1` shoots `editor-viewport-move.png`, `-turn.png`, `-scale.png`,
//! `-box.png` and `-group.png`. Unset, no walk and no deadline run.

use bevy::prelude::*;
use clap::Parser;
use nova_protocol::{nova_debug::diagnostics, prelude::*};

#[path = "shared/open_world_stage.rs"]
mod open_world_stage;

#[derive(Parser)]
#[command(name = "editor_viewport")]
#[command(version = "1.0.0")]
#[command(
    about = "The editor's viewport, to navigate and edit by hand",
    long_about = None
)]
struct Cli;

const INTRO: &str = "RMB looks, MMB orbits, Q W E R pick a tool, Ctrl snaps.";

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();
    open_world_stage::isolate_the_profile(env!("CARGO_CRATE_NAME"));

    let mut app = editor_app(true, None);
    app.insert_resource(EditorOpensScratch);
    app.add_plugins(diagnostics::DiagnosticsHintPlugin::new(INTRO));
    app.add_systems(OnEnter(GameStates::MainMenu), open_the_sandbox);
    app.add_systems(
        PostUpdate,
        read_the_viewport
            .in_set(diagnostics::DiagnosticsSystems)
            .run_if(in_state(GameStates::Playing)),
    );

    if std::env::var_os("NOVA_AUTOPILOT").is_some() {
        app.insert_resource(bevy::ecs::error::FallbackErrorHandler(
            bevy::ecs::error::panic,
        ));
        app.add_systems(
            Startup,
            nova_protocol::nova_debug::harness::force_capture_resolution,
        );
        app.init_resource::<smoke::StageTrace>();
        app.add_systems(Last, smoke::trace_the_stage);
        app.add_plugins(smoke::script());
    }

    app.run()
}

/// What the Sandbox button does, once: a later Back to Main Menu stays on the
/// menu.
fn open_the_sandbox(
    mut opened: Local<bool>,
    mut mode: ResMut<GameMode>,
    mut state: ResMut<NextState<GameStates>>,
) {
    if !*opened {
        *opened = true;
        *mode = GameMode::Sandbox;
        state.set(GameStates::Playing);
    }
}

/// The diagnostics panel: what the viewport is doing.
fn read_the_viewport(world: &mut World) {
    let Some(probe) = world.get_resource::<EditorProbe>().cloned() else {
        return;
    };
    let mut panel = world.resource_mut::<diagnostics::DiagnosticsPanel>();
    panel.line(format!(
        "Tool: {:?}, {:?} axes",
        probe.stage_tool, probe.gizmo_space
    ));
    panel.line(format!("Camera: {:?}", probe.camera_gesture));
    panel.line(format!("Selected: {}", probe.selected_nodes.len()));
    panel.line(format!("Undo steps: {}", probe.history_depth));
}

mod smoke {
    use std::sync::Arc;

    use bevy::input::mouse::MouseMotion;
    use nova_protocol::nova_debug::harness::{shoot, AutopilotPlugin, Predicate};

    use super::*;

    /// Seconds a cold boot gets on a software GPU.
    const BOOT_SECS: f32 = 300.0;
    /// Seconds a beat gets.
    const BEAT_SECS: f32 = 60.0;
    /// Seconds a frame capture gets.
    const SHOT_SECS: f32 = 120.0;
    const ADD_MENU: &str = "Add Menu Button";
    const ADD_ASTEROID: &str = "Add Asteroid";
    const CAMERA: &str = "Editor Camera";
    const DRAG_HANDLE: &str = "Gizmo Move Grab Y";
    const MOVE_SHOT: &str = "editor-viewport-move.png";
    const TURN_SHOT: &str = "editor-viewport-turn.png";
    const SCALE_SHOT: &str = "editor-viewport-scale.png";
    const BOX_SHOT: &str = "editor-viewport-box.png";
    const GROUP_SHOT: &str = "editor-viewport-group.png";
    const UNDO: &[KeyCode] = &[KeyCode::ControlLeft, KeyCode::KeyZ];
    const DUPLICATE: &[KeyCode] = &[KeyCode::ControlLeft, KeyCode::KeyD];
    const COPY: &[KeyCode] = &[KeyCode::ControlLeft, KeyCode::KeyC];
    const PASTE: &[KeyCode] = &[KeyCode::ControlLeft, KeyCode::KeyV];
    const SELECT_ALL: &[KeyCode] = &[KeyCode::ControlLeft, KeyCode::KeyA];

    /// The walk's notes: how many nodes the document opened with, and where the
    /// camera and the rock stood when a beat began.
    #[derive(Resource, Default)]
    struct Walk {
        opened: usize,
        eye: Vec3,
        turn: Quat,
        reach: f32,
        rock: String,
    }

    /// A beat traced frame by frame. Runs in `Last`, after propagation, so it
    /// reads the poses the frame renders with.
    #[derive(Resource, Default)]
    pub(super) struct StageTrace {
        beat: &'static str,
        left: u32,
        frame: u32,
        /// Mouse motion to send each frame.
        sway: Vec2,
        /// Where the pointer goes next, and its step each frame.
        cursor: Option<(Vec2, Vec2)>,
        /// The rose's screen position, frame by frame.
        rose: Vec<Vec2>,
        /// How far the rig stood from the rock on screen, frame by frame.
        gaps: Vec<f32>,
    }

    impl StageTrace {
        fn new(beat: &'static str, frames: u32) -> Self {
            Self {
                beat,
                left: frames,
                ..default()
            }
        }
    }

    /// Log where the rock, the rig and the rose stand on screen this frame.
    pub(super) fn trace_the_stage(world: &mut World) {
        if world.resource::<StageTrace>().left == 0 {
            return;
        }
        let (camera, eye) = {
            let mut cameras = world.query::<(&Name, &Camera, &GlobalTransform)>();
            let (_, camera, eye) = cameras
                .iter(world)
                .find(|(name, ..)| name.as_str() == CAMERA)
                .expect("the editor camera stands");
            (camera.clone(), *eye)
        };
        let rock = world.resource::<Walk>().rock.clone();
        let rock = probe(world)
            .node_positions
            .iter()
            .find(|(id, _)| *id == rock)
            .expect("the rock stands")
            .1;
        let rig = world
            .query::<(&Name, &GlobalTransform)>()
            .iter(world)
            .find(|(name, _)| name.as_str() == "Editor Gizmo")
            .expect("the rig stands")
            .1
            .compute_transform();
        let handles = world
            .query::<(&Name, &InheritedVisibility)>()
            .iter(world)
            .filter(|(name, shown)| name.as_str().starts_with("Gizmo ") && shown.get())
            .count();
        let rose = world
            .resource::<AxisRoseOrigin>()
            .0
            .expect("the rose is drawn");
        let on_screen = |at: Vec3| camera.world_to_viewport(&eye, at).expect("on screen");
        let (rock_at, rig_at, rose_at) =
            (on_screen(rock), on_screen(rig.translation), on_screen(rose));
        let gap = rock_at.distance(rig_at);

        let mut trace = world.resource_mut::<StageTrace>();
        trace.frame += 1;
        trace.left -= 1;
        trace.rose.push(rose_at);
        trace.gaps.push(gap);
        info!(
            "editor viewport: trace {} frame {} eye {:.2} node_px {:.1} rig_px {:.1} gap_px {:.2} rose_px {:.2} handles {} arm {:.2}",
            trace.beat,
            trace.frame,
            eye.translation(),
            rock_at,
            rig_at,
            gap,
            rose_at,
            handles,
            rig.scale.x,
        );
        let sway = trace.sway;
        let next = trace.cursor.map(|(at, step)| at + step);
        trace.cursor = trace.cursor.map(|(at, step)| (at + step, step));
        if sway != Vec2::ZERO {
            world.write_message(MouseMotion { delta: sway });
        }
        if let Some(next) = next {
            move_cursor(next)(world);
        }
    }

    /// Fail if the traced beat moved the rose off its corner, or the rig off
    /// the rock.
    fn the_overlays_held(world: &mut World) {
        let trace = world.resource::<StageTrace>();
        let first = trace.rose[0];
        let drift = trace
            .rose
            .iter()
            .map(|at| at.distance(first))
            .fold(0.0, f32::max);
        let gap = trace.gaps.iter().copied().fold(0.0, f32::max);
        info!(
            "editor viewport: {} rose drift {drift} gap {gap}",
            trace.beat
        );
        assert!(drift < 1.0, "the rose left its corner by {drift} px");
        assert!(gap < 1.0, "the rig left the rock by {gap} px");
    }

    /// Advance once the traced beat has run out.
    fn traced() -> Arc<Predicate> {
        Arc::new(|world: &World| world.resource::<StageTrace>().left == 0)
    }

    /// Where the named handle stands in the window.
    fn handle_aim(world: &mut World, name: &str) -> Option<Vec2> {
        let at = world
            .query::<(&Name, &GlobalTransform)>()
            .iter(world)
            .find(|(held, _)| held.as_str().starts_with(name))
            .map(|(_, pose)| pose.translation())?;
        let (camera, eye) = world
            .query::<(&Name, &Camera, &GlobalTransform)>()
            .iter(world)
            .find(|(held, ..)| held.as_str() == CAMERA)
            .map(|(_, camera, eye)| (camera.clone(), *eye))?;
        camera.world_to_viewport(&eye, at).ok()
    }

    /// Whether the pointer rests on a gizmo handle.
    fn pointer_on_a_handle() -> Arc<Predicate> {
        Arc::new(|world: &World| {
            let Some(hit) = world
                .try_query::<&bevy::picking::pointer::PointerInteraction>()
                .and_then(|mut pointers| {
                    pointers
                        .iter(world)
                        .filter_map(|interaction| interaction.get_nearest_hit())
                        .map(|(entity, _)| *entity)
                        .next()
                })
            else {
                return false;
            };
            world
                .get::<Name>(hit)
                .is_some_and(|name| name.as_str().starts_with("Gizmo "))
        })
    }

    fn probe(world: &World) -> &EditorProbe {
        world.resource::<EditorProbe>()
    }

    /// Advance once the probe passes.
    fn editor(check: impl Fn(&EditorProbe) -> bool + Send + Sync + 'static) -> Arc<Predicate> {
        Arc::new(move |world: &World| world.get_resource::<EditorProbe>().is_some_and(&check))
    }

    /// The probe, for a stalled beat to print.
    fn readout(world: &World) -> String {
        world.get_resource::<EditorProbe>().map_or_else(
            || "no probe".to_string(),
            |probe| {
                format!(
                    "tool {:?} {:?} camera {:?} selected {:?} nodes {} depth {} status {:?}",
                    probe.stage_tool,
                    probe.gizmo_space,
                    probe.camera_gesture,
                    probe.selected_nodes,
                    probe.context_nodes.len(),
                    probe.history_depth,
                    probe.status
                )
            },
        )
    }

    fn camera(world: &mut World) -> Transform {
        *world
            .query::<(&Name, &Transform)>()
            .iter(world)
            .find(|(name, _)| name.as_str() == CAMERA)
            .expect("the editor camera stands")
            .1
    }

    /// The camera's distance to the rock the walk added.
    fn reach(world: &mut World) -> f32 {
        let rock = world.resource::<Walk>().rock.clone();
        let at = probe(world)
            .node_positions
            .iter()
            .find(|(id, _)| *id == rock)
            .expect("the rock stands")
            .1;
        camera(world).translation.distance(at)
    }

    /// The middle of the stage: clear of the rail and the inspector.
    fn stage_point(world: &mut World, height: f32) -> Vec2 {
        let window = world
            .query::<&Window>()
            .iter(world)
            .next()
            .expect("the window stands");
        Vec2::new(window.width() * 0.5, window.height() * height)
    }

    /// Hold `keys`, wait for `landed`, let go, and log what the editor reads.
    fn chord(
        script: AutopilotPlugin<GameStates>,
        label: &str,
        keys: &'static [KeyCode],
        landed: Arc<Predicate>,
    ) -> AutopilotPlugin<GameStates> {
        script
            .step(format!("editor viewport: {label}"))
            .on_enter(move |world: &mut World| {
                for key in keys {
                    press_key(*key)(world);
                }
            })
            .until(landed)
            .diagnose(readout)
            .deadline(BEAT_SECS)
            .add()
            .step(format!("editor viewport: let go after {label}"))
            .on_enter(move |world: &mut World| {
                for key in keys {
                    release_key(*key)(world);
                }
                info!("editor viewport: {}", readout(world));
            })
            .until(frames(2))
            .add()
    }

    /// Shoot `name` once the rig has had a frame to change.
    fn shot(
        script: AutopilotPlugin<GameStates>,
        name: &'static str,
    ) -> AutopilotPlugin<GameStates> {
        script
            .step(format!("editor viewport: settle before {name}"))
            .until(frames(3))
            .add()
            .step(format!("editor viewport: shoot {name}"))
            .on_enter(move |world: &mut World| shoot(world, name))
            .until(shot_written(name))
            .deadline(SHOT_SECS)
            .add()
    }

    pub(super) fn script() -> AutopilotPlugin<GameStates> {
        let script = AutopilotPlugin::<GameStates>::new()
            .step("editor viewport: the editor is up")
            .until(Arc::new(|world: &World| {
                world
                    .get_resource::<EditorProbe>()
                    .is_some_and(|probe| !probe.context_nodes.is_empty())
                    && ui_node_present(ADD_MENU)(world)
            }))
            .deadline(BOOT_SECS)
            .add()
            .step("editor viewport: the tool starts on move")
            .on_enter(|world: &mut World| {
                let opened = probe(world).context_nodes.len();
                assert_eq!(probe(world).stage_tool, StageToolType::Move);
                world.insert_resource(Walk {
                    opened,
                    ..default()
                });
            })
            .add()
            .click_named(
                "editor viewport: open the Add menu",
                ADD_MENU,
                ui_node_present(ADD_ASTEROID),
                BEAT_SECS,
            )
            .click_named(
                "editor viewport: add a rock",
                ADD_ASTEROID,
                editor(|probe| probe.gizmo_node.is_some()),
                BEAT_SECS,
            )
            .step("editor viewport: note the rock")
            .on_enter(|world: &mut World| {
                let rock = probe(world)
                    .selected_node
                    .clone()
                    .expect("the rock is selected");
                world.resource_mut::<Walk>().rock = rock;
            })
            .add();
        let script = shot(script, MOVE_SHOT);
        let script = chord(
            script,
            "E takes the turn tool",
            &[KeyCode::KeyE],
            editor(|probe| probe.stage_tool == StageToolType::Turn),
        );
        let script = shot(script, TURN_SHOT);
        let script = chord(
            script,
            "R takes the scale tool",
            &[KeyCode::KeyR],
            editor(|probe| probe.stage_tool == StageToolType::Scale && probe.gizmo_node.is_some()),
        );
        let script = shot(script, SCALE_SHOT);
        let script = chord(
            script,
            "Q takes the select tool and the handles go",
            &[KeyCode::KeyQ],
            editor(|probe| probe.stage_tool == StageToolType::Select && probe.gizmo_node.is_none()),
        );
        let script = chord(
            script,
            "W takes the move tool back",
            &[KeyCode::KeyW],
            editor(|probe| probe.stage_tool == StageToolType::Move),
        );
        let script = chord(
            script,
            "X swaps the handles to local axes",
            &[KeyCode::KeyX],
            editor(|probe| probe.gizmo_space == GizmoSpaceType::Local),
        );
        let script = chord(
            script,
            "X swaps them back",
            &[KeyCode::KeyX],
            editor(|probe| probe.gizmo_space == GizmoSpaceType::World),
        );

        let script = script
            .step("editor viewport: point at the stage")
            .on_enter(|world: &mut World| {
                let at = stage_point(world, 0.5);
                move_cursor(at)(world);
            })
            .until(frames(4))
            .add()
            .step("editor viewport: hold RMB")
            .on_enter(|world: &mut World| {
                world.resource_mut::<Walk>().eye = camera(world).translation;
                press_mouse(MouseButton::Right)(world);
            })
            .until(editor(|probe| {
                probe.camera_gesture == CameraGestureType::Look
            }))
            .diagnose(readout)
            .deadline(BEAT_SECS)
            .add()
            .step("editor viewport: E and a swing with RMB held")
            .on_enter(|world: &mut World| {
                world.resource_mut::<Walk>().turn = camera(world).rotation;
                press_key(KeyCode::KeyE)(world);
                world.write_message(MouseMotion {
                    delta: Vec2::new(40.0, 10.0),
                });
            })
            .until(frames(6))
            .add()
            .step("editor viewport: the view turned in place and E took the turn tool")
            .on_enter(|world: &mut World| {
                release_key(KeyCode::KeyE)(world);
                let walk = world.resource::<Walk>();
                let (eye, turn) = (walk.eye, walk.turn);
                let now = camera(world);
                assert_eq!(now.translation, eye, "nothing flies the eye");
                assert_ne!(now.rotation, turn, "the view turned");
                assert_eq!(probe(world).stage_tool, StageToolType::Turn);
            })
            .add()
            .step("editor viewport: W takes the move tool back under RMB")
            .on_enter(press_key(KeyCode::KeyW))
            .until(editor(|probe| probe.stage_tool == StageToolType::Move))
            .diagnose(readout)
            .deadline(BEAT_SECS)
            .add()
            .step("editor viewport: let RMB go")
            .on_enter(|world: &mut World| {
                release_key(KeyCode::KeyW)(world);
                release_mouse(MouseButton::Right)(world);
            })
            .until(editor(|probe| {
                probe.camera_gesture == CameraGestureType::Idle
            }))
            .diagnose(readout)
            .deadline(BEAT_SECS)
            .add()
            .step("editor viewport: hold MMB")
            .on_enter(|world: &mut World| {
                let held = reach(world);
                world.resource_mut::<Walk>().reach = held;
                press_mouse(MouseButton::Middle)(world);
            })
            .until(editor(|probe| {
                probe.camera_gesture == CameraGestureType::Orbit
            }))
            .diagnose(readout)
            .deadline(BEAT_SECS)
            .add()
            .step("editor viewport: swing the mouse")
            .on_enter(|world: &mut World| {
                world.resource_mut::<Walk>().eye = camera(world).translation;
                world.write_message(MouseMotion {
                    delta: Vec2::new(40.0, 10.0),
                });
            })
            .until(frames(4))
            .add()
            .step("editor viewport: the orbit moved the eye and kept the reach")
            .on_enter(|world: &mut World| {
                let walk = world.resource::<Walk>();
                let (before, held) = (walk.eye, walk.reach);
                let (now, kept) = (camera(world).translation, reach(world));
                assert!(now.distance(before) > 0.01, "the eye swung: {now:?}");
                assert!((kept - held).abs() < held * 0.01, "{held} -> {kept}");
                info!("editor viewport: orbit held {held} as {kept}");
                release_mouse(MouseButton::Middle)(world);
            })
            .until(editor(|probe| {
                probe.camera_gesture == CameraGestureType::Idle
            }))
            .deadline(BEAT_SECS)
            .add()
            .step("editor viewport: hold MMB for the trace")
            .on_enter(press_mouse(MouseButton::Middle))
            .until(editor(|probe| {
                probe.camera_gesture == CameraGestureType::Orbit
            }))
            .diagnose(readout)
            .deadline(BEAT_SECS)
            .add()
            .step("editor viewport: trace an orbit")
            .on_enter(|world: &mut World| {
                world.insert_resource(StageTrace {
                    sway: Vec2::new(14.0, 5.0),
                    ..StageTrace::new("orbit", 12)
                });
            })
            .until(traced())
            .deadline(BEAT_SECS)
            .add()
            .step("editor viewport: the rose held its corner and the rig its rock")
            .on_enter(the_overlays_held)
            .add()
            .step("editor viewport: let the orbit go")
            .on_enter(release_mouse(MouseButton::Middle))
            .until(editor(|probe| {
                probe.camera_gesture == CameraGestureType::Idle
            }))
            .deadline(BEAT_SECS)
            .add()
            .step("editor viewport: point at the up handle")
            .on_enter(|world: &mut World| {
                let at = handle_aim(world, DRAG_HANDLE).expect("the handle is on screen");
                move_cursor(at)(world);
            })
            .until(pointer_on_a_handle())
            .diagnose(readout)
            .deadline(BEAT_SECS)
            .add()
            .step("editor viewport: grab the handle")
            .on_enter(press_mouse(MouseButton::Left))
            .until(frames(3))
            .add()
            .step("editor viewport: trace a handle drag")
            .on_enter(|world: &mut World| {
                let at = handle_aim(world, DRAG_HANDLE).expect("the handle is on screen");
                world.insert_resource(StageTrace {
                    cursor: Some((at, Vec2::new(1.5, -7.0))),
                    ..StageTrace::new("drag", 12)
                });
            })
            .until(traced())
            .deadline(BEAT_SECS)
            .add()
            .step("editor viewport: the rose and the rig held through the drag")
            .on_enter(the_overlays_held)
            .add()
            .step("editor viewport: let the handle go")
            .on_enter(release_mouse(MouseButton::Left))
            .until(editor(|probe| probe.history_depth == 2))
            .diagnose(readout)
            .deadline(BEAT_SECS)
            .add();
        let script = chord(
            script,
            "Ctrl+Z takes the drag back",
            UNDO,
            editor(|probe| probe.history_depth == 1),
        );

        let script = chord(script, "Ctrl+C copies the rock", COPY, frames(2));
        let script = chord(
            script,
            "Ctrl+V pastes as one step",
            PASTE,
            editor(|probe| probe.next_undo.as_deref() == Some("Paste")),
        );
        let script = chord(
            script,
            "Ctrl+D duplicates the paste as one step",
            DUPLICATE,
            editor(|probe| probe.next_undo.as_deref() == Some("Duplicate")),
        );
        let script = chord(
            script,
            "Ctrl+Z takes the duplicate back",
            UNDO,
            editor(|probe| probe.next_undo.as_deref() == Some("Paste")),
        );
        let script = chord(
            script,
            "Ctrl+Z takes the paste back",
            UNDO,
            editor(|probe| probe.history_depth == 1),
        );
        let script = chord(
            script,
            "Ctrl+A takes the whole level",
            SELECT_ALL,
            editor(|probe| probe.selected_nodes.len() > 1),
        );
        let script = script
            .step("editor viewport: scale refuses a group and shows no handles")
            .on_enter(press_key(KeyCode::KeyR))
            .until(editor(|probe| {
                probe.stage_tool == StageToolType::Scale
                    && probe.gizmo_node.is_none()
                    && probe.status.contains("one node")
            }))
            .diagnose(readout)
            .deadline(BEAT_SECS)
            .add()
            .step("editor viewport: back to the move tool")
            .on_enter(|world: &mut World| {
                release_key(KeyCode::KeyR)(world);
                press_key(KeyCode::KeyW)(world);
            })
            .until(frames(2))
            .add()
            .step("editor viewport: point at open space")
            .on_enter(|world: &mut World| {
                release_key(KeyCode::KeyW)(world);
                let at = stage_point(world, 0.12);
                move_cursor(at)(world);
            })
            .until(frames(4))
            .add()
            .step("editor viewport: a click on nothing deselects")
            .on_enter(press_mouse(MouseButton::Left))
            .until(frames(2))
            .add()
            .step("editor viewport: let the click go")
            .on_enter(release_mouse(MouseButton::Left))
            .until(editor(|probe| probe.selected_nodes.is_empty()))
            .diagnose(readout)
            .deadline(BEAT_SECS)
            .add()
            .step("editor viewport: take the select tool")
            .on_enter(press_key(KeyCode::KeyQ))
            .until(editor(|probe| probe.stage_tool == StageToolType::Select))
            .deadline(BEAT_SECS)
            .add()
            .step("editor viewport: press on open space")
            .on_enter(|world: &mut World| {
                release_key(KeyCode::KeyQ)(world);
                let at = stage_point(world, 0.12) - Vec2::new(500.0, 0.0);
                move_cursor(at)(world);
            })
            .until(frames(4))
            .add()
            .step("editor viewport: start the box")
            .on_enter(press_mouse(MouseButton::Left))
            .until(frames(2))
            .add()
            .step("editor viewport: drag the box over the range")
            .on_enter(|world: &mut World| {
                let at = stage_point(world, 0.9) + Vec2::new(500.0, 0.0);
                move_cursor(at)(world);
            })
            .until(frames(4))
            .add();
        let script = shot(script, BOX_SHOT);
        let script = script
            .step("editor viewport: let the box go")
            .on_enter(release_mouse(MouseButton::Left))
            .until(editor(|probe| probe.selected_nodes.len() > 1))
            .diagnose(readout)
            .deadline(BEAT_SECS)
            .add()
            .step("editor viewport: back to the move tool for the group rig")
            .on_enter(press_key(KeyCode::KeyW))
            .until(editor(|probe| {
                probe.stage_tool == StageToolType::Move && probe.gizmo_node.is_some()
            }))
            .diagnose(readout)
            .deadline(BEAT_SECS)
            .add()
            .step("editor viewport: let W go")
            .on_enter(release_key(KeyCode::KeyW))
            .until(frames(2))
            .add();
        let script = shot(script, GROUP_SHOT);
        script
            .step("editor viewport: done")
            .on_enter(|world: &mut World| {
                let opened = world.resource::<Walk>().opened;
                info!(
                    "editor viewport: walked from {opened} nodes: {}",
                    readout(world)
                );
            })
            .add()
    }
}
