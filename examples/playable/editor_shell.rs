//! editor_shell: the editor's fixed panels, to read and press by hand.
//!
//! Boots the shipped app and opens a scratch mod past the Mod Hub, on a
//! profile folder of its own.
//!
//! Try: the top bar's tool buttons against `Q` `W` `E` `R`, Axes against `X`,
//! Snap against a held Ctrl. In the Hierarchy, press a caret, type in the filter,
//! Ctrl+click and Shift+click rows, and right-click a row for its menu. In the
//! strip under the stage, read Assets, and press a row
//! of Problems to reach the node it names. `Ctrl+1` `Ctrl+2` `Ctrl+3` hide the
//! Hierarchy, the Inspector and the strip's body. A group heading in the
//! Inspector folds.
//!
//! The panel reads the panels, the open tab, the problem counts and the row
//! counts.
//!
//! With `NOVA_AUTOPILOT=1` a smoke walk does these gestures through the real
//! buttons and asserts the probe after each. `--width` and `--height` set the
//! walk's window. `NOVA_CAPTURE=1` shoots `editor-shell-scene.png`, `-filter.png`,
//! `-menu.png`, `-assets.png`, `-tile-hint.png`, `-tile-clash.png`,
//! `-problems.png`, `-ship.png`, `-events.png` and `-popup.png`.
//! Unset, no walk and no deadline run.

use bevy::{prelude::*, window::PrimaryWindow};
use clap::Parser;
use nova_protocol::{
    nova_debug::{diagnostics, harness::capturing},
    prelude::*,
};

#[path = "shared/open_world_stage.rs"]
mod open_world_stage;
#[path = "../screenshots/shared/ui_walk.rs"]
mod ui_walk;

#[derive(Parser)]
#[command(name = "editor_shell")]
#[command(version = "1.0.0")]
#[command(
    about = "The editor's fixed panels, to read and press by hand",
    long_about = None
)]
struct Cli {
    /// The walk's window width, in logical px.
    #[arg(long, default_value_t = 1920.0)]
    width: f32,
    /// The walk's window height, in logical px.
    #[arg(long, default_value_t = 1080.0)]
    height: f32,
}

const INTRO: &str = "Press the top bar, right-click a tree row, read the strip's tabs.";

/// The size the walk asked for.
#[derive(Resource, Clone, Copy)]
struct WalkWindow(f32, f32);

fn main() -> bevy::app::AppExit {
    let cli = Cli::parse();
    open_world_stage::isolate_the_profile(env!("CARGO_CRATE_NAME"));

    let mut app = editor_app(true, None);
    app.insert_resource(EditorOpensScratch);
    app.add_systems(OnEnter(GameStates::MainMenu), open_the_sandbox);
    // The panel stands over the top bar, which is what the frames are of.
    if !capturing() {
        app.add_plugins(diagnostics::DiagnosticsHintPlugin::new(INTRO));
        app.add_systems(
            PostUpdate,
            read_the_shell
                .in_set(diagnostics::DiagnosticsSystems)
                .run_if(in_state(GameStates::Playing)),
        );
    }

    if std::env::var_os("NOVA_AUTOPILOT").is_some() {
        app.insert_resource(bevy::ecs::error::FallbackErrorHandler(
            bevy::ecs::error::panic,
        ));
        app.insert_resource(WalkWindow(cli.width, cli.height));
        app.add_systems(Startup, size_the_window);
        app.add_plugins(smoke::script());
    }

    app.run()
}

/// Fix the window at the walk's size.
fn size_the_window(size: Res<WalkWindow>, mut windows: Query<&mut Window, With<PrimaryWindow>>) {
    if let Ok(mut window) = windows.single_mut() {
        window.resolution.set(size.0, size.1);
        window.resizable = false;
    }
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

/// The diagnostics panel: what the shell is showing.
fn read_the_shell(world: &mut World) {
    let Some(probe) = world.get_resource::<EditorProbe>().cloned() else {
        return;
    };
    let shell = probe.shell;
    let mut panel = world.resource_mut::<diagnostics::DiagnosticsPanel>();
    panel.line(format!(
        "Panels: hierarchy {}, inspector {}, strip {}",
        shell.hierarchy, shell.inspector, shell.browser
    ));
    panel.line(format!("Tabs: {}, {}", shell.rail_tab, shell.browser_tab));
    panel.line(format!(
        "Problems: {} errors, {} warnings",
        shell.errors, shell.warnings
    ));
    panel.line(format!(
        "Rows: tree {}, assets {}",
        shell.tree_rows, shell.asset_rows
    ));
}

mod smoke {
    use std::sync::{Arc, OnceLock};

    use nova_protocol::nova_debug::harness::{shoot, AutopilotPlugin, Predicate};

    use super::{
        ui_walk::{block_centre, block_named, canvas_middle, Gestures},
        *,
    };

    /// Seconds a cold boot gets on a software GPU.
    const BOOT_SECS: f32 = 300.0;
    /// Seconds a beat gets.
    const BEAT_SECS: f32 = 60.0;
    /// Seconds a frame capture gets.
    const SHOT_SECS: f32 = 120.0;
    const ADD_MENU: &str = "Add Menu Button";
    const ADD_ASTEROID: &str = "Add Asteroid";
    const FILTER: &str = "Hierarchy Filter";
    const GRIP: &str = "Browser Resize Handle";
    const UNDO: &[KeyCode] = &[KeyCode::ControlLeft, KeyCode::KeyZ];
    const FOLD_HIERARCHY: &[KeyCode] = &[KeyCode::ControlLeft, KeyCode::Digit1];
    const FOLD_INSPECTOR: &[KeyCode] = &[KeyCode::ControlLeft, KeyCode::Digit2];
    const FOLD_STRIP: &[KeyCode] = &[KeyCode::ControlLeft, KeyCode::Digit3];

    /// The walk's notes: the rock's id, and the tree's size before the filter.
    #[derive(Resource, Default)]
    struct Walk {
        rock: String,
        rows: usize,
        log_lines: usize,
        strip: f32,
        depth: usize,
        play_at: Vec2,
    }

    /// Advance once the probe passes.
    fn editor(check: impl Fn(&EditorProbe) -> bool + Send + Sync + 'static) -> Arc<Predicate> {
        Arc::new(move |world: &World| world.get_resource::<EditorProbe>().is_some_and(&check))
    }

    fn probe(world: &World) -> &EditorProbe {
        world.resource::<EditorProbe>()
    }

    /// The probe, for a stalled beat to print.
    fn readout(world: &World) -> String {
        world.get_resource::<EditorProbe>().map_or_else(
            || "no probe".to_string(),
            |probe| {
                format!(
                    "tool {:?} {:?} selected {:?} depth {} undo {:?} shell {:?} status {:?}",
                    probe.stage_tool,
                    probe.gizmo_space,
                    probe.selected_nodes,
                    probe.history_depth,
                    probe.next_undo,
                    probe.shell,
                    probe.status
                )
            },
        )
    }

    /// Hold `keys`, wait for `landed`, and let go.
    fn chord(
        script: AutopilotPlugin<GameStates>,
        label: &str,
        keys: &'static [KeyCode],
        landed: Arc<Predicate>,
    ) -> AutopilotPlugin<GameStates> {
        script
            .step(format!("editor shell: {label}"))
            .on_enter(move |world: &mut World| {
                for key in keys {
                    press_key(*key)(world);
                }
            })
            .until(landed)
            .diagnose(readout)
            .deadline(BEAT_SECS)
            .add()
            .step(format!("editor shell: let go after {label}"))
            .on_enter(move |world: &mut World| {
                for key in keys {
                    release_key(*key)(world);
                }
            })
            .until(frames(2))
            .add()
    }

    /// Shoot `name` once the panels have had frames to settle.
    fn shot(
        script: AutopilotPlugin<GameStates>,
        name: &'static str,
    ) -> AutopilotPlugin<GameStates> {
        script
            .step(format!("editor shell: settle before {name}"))
            .until(frames(4))
            .add()
            .step(format!("editor shell: shoot {name}"))
            .on_enter(move |world: &mut World| shoot(world, name))
            .until(shot_written(name))
            .deadline(SHOT_SECS)
            .add()
    }

    /// The Log tab: what reaches it, and what its controls do.
    fn log_tab(script: AutopilotPlugin<GameStates>) -> AutopilotPlugin<GameStates> {
        let script = script
            .click_named(
                "editor shell: open Log",
                "Browser Tab Log",
                ui_node_present("Log Clear"),
                BEAT_SECS,
            )
            .step("editor shell: the log warns and errs")
            .on_enter(|world: &mut World| {
                let held = probe(world).shell.log_lines.len();
                world.resource_mut::<Walk>().log_lines = held;
                warn!("a warning for the Log tab");
                error!("an error for the Log tab");
            })
            .until(Arc::new(|world: &World| {
                probe(world).shell.log_lines.len() == world.resource::<Walk>().log_lines + 2
            }))
            .diagnose(readout)
            .deadline(BEAT_SECS)
            .add();
        let script = shot(script, "editor-shell-log.png");
        let script = script
            .click_named(
                "editor shell: show errors alone",
                "Log Level Errors",
                editor(|probe| {
                    probe.shell.log_shown >= 1
                        && probe.shell.log_shown < probe.shell.log_lines.len()
                }),
                BEAT_SECS,
            )
            .click_named(
                "editor shell: show every level",
                "Log Level All",
                editor(|probe| probe.shell.log_shown == probe.shell.log_lines.len()),
                BEAT_SECS,
            )
            .click_named(
                "editor shell: focus the log filter",
                "Log Filter",
                frames(3),
                BEAT_SECS,
            )
            .step("editor shell: type a log filter")
            .on_enter(type_text("a warning"))
            .until(editor(|probe| probe.shell.log_shown == 1))
            .diagnose(readout)
            .deadline(BEAT_SECS)
            .add();
        let script = shot(script, "editor-shell-log-filter.png");
        script
            .step("editor shell: wipe the log filter")
            .on_enter(|world: &mut World| {
                for _ in 0.."a warning".len() {
                    press_edit_key(Key::Backspace)(world);
                }
            })
            .until(editor(|probe| {
                probe.shell.log_shown == probe.shell.log_lines.len()
            }))
            .diagnose(readout)
            .deadline(BEAT_SECS)
            .add()
            .click_named(
                "editor shell: clear the log",
                "Log Clear",
                editor(|probe| probe.shell.log_lines.is_empty()),
                BEAT_SECS,
            )
            .step("editor shell: a line after Clear still arrives")
            .on_enter(|_: &mut World| warn!("a warning after Clear"))
            .until(editor(|probe| probe.shell.log_lines.len() == 1))
            .diagnose(readout)
            .deadline(BEAT_SECS)
            .add()
    }

    /// How tall the strip stands.
    fn strip_height(world: &mut World) -> f32 {
        world
            .query::<(&Name, &ComputedNode)>()
            .iter(world)
            .find(|(name, _)| name.as_str() == "Editor Browser")
            .map(|(_, node)| node.size().y * node.inverse_scale_factor())
            .expect("the strip stands")
    }

    /// Pull the strip's top edge up, then back down.
    fn resize_strip(script: AutopilotPlugin<GameStates>) -> AutopilotPlugin<GameStates> {
        let script = script
            .step("editor shell: point at the strip's top edge")
            .on_enter(|world: &mut World| {
                world.resource_mut::<Walk>().strip = strip_height(world);
                hover_named(GRIP)(world);
            })
            .until(frames(3))
            .add()
            .step("editor shell: take the edge")
            .on_enter(press_mouse(MouseButton::Left))
            .until(frames(3))
            .add()
            .step("editor shell: pull the edge up")
            .on_enter(|world: &mut World| {
                let at = ui_node_centre(world, GRIP).expect("the grip stands");
                move_cursor(at - Vec2::new(0.0, 80.0))(world);
            })
            .until(frames(4))
            .add()
            .step("editor shell: let the edge go")
            .on_enter(release_mouse(MouseButton::Left))
            .until(frames(3))
            .add()
            .step("editor shell: the strip grew by the pull")
            .on_enter(|world: &mut World| {
                let (was, now) = (world.resource::<Walk>().strip, strip_height(world));
                info!("editor shell: the strip stood {was} and stands {now}");
                assert!((now - was - 80.0).abs() < 2.0, "{was} -> {now}");
            })
            .add();
        let script = shot(script, "editor-shell-strip-tall.png");
        script
            .step("editor shell: point at the edge again")
            .on_enter(hover_named(GRIP))
            .until(frames(3))
            .add()
            .step("editor shell: take the edge again")
            .on_enter(press_mouse(MouseButton::Left))
            .until(frames(3))
            .add()
            .step("editor shell: push the edge down")
            .on_enter(|world: &mut World| {
                let at = ui_node_centre(world, GRIP).expect("the grip stands");
                move_cursor(at + Vec2::new(0.0, 80.0))(world);
            })
            .until(frames(4))
            .add()
            .step("editor shell: let it go")
            .on_enter(release_mouse(MouseButton::Left))
            .until(frames(3))
            .add()
    }

    pub(super) fn script() -> AutopilotPlugin<GameStates> {
        let script = AutopilotPlugin::<GameStates>::new()
            .step("editor shell: the editor is up")
            .until(Arc::new(|world: &World| {
                world
                    .get_resource::<EditorProbe>()
                    .is_some_and(|probe| !probe.context_nodes.is_empty())
                    && ui_node_present(ADD_MENU)(world)
            }))
            .deadline(BOOT_SECS)
            .add()
            .step("editor shell: every panel opens shown, on Assets")
            .on_enter(|world: &mut World| {
                let shell = &probe(world).shell;
                assert!(shell.hierarchy && shell.inspector && shell.browser);
                assert_eq!(shell.browser_tab, "Assets");
                assert_eq!(shell.rail_tab, "Scene");
                world.init_resource::<Walk>();
            })
            .add()
            .click_named(
                "editor shell: open the Add menu",
                ADD_MENU,
                ui_node_present(ADD_ASTEROID),
                BEAT_SECS,
            )
            .click_named(
                "editor shell: add a rock",
                ADD_ASTEROID,
                editor(|probe| probe.gizmo_node.is_some()),
                BEAT_SECS,
            )
            .step("editor shell: note the rock")
            .on_enter(|world: &mut World| {
                let rock = probe(world)
                    .selected_node
                    .clone()
                    .expect("the rock is selected");
                let rows = probe(world).shell.tree_rows;
                let mut walk = world.resource_mut::<Walk>();
                walk.rock = rock;
                walk.rows = rows;
            })
            .add();
        let script = shot(script, "editor-shell-scene.png");

        let script = script
            .click_named(
                "editor shell: the Turn button is the E key",
                "Tool Button Turn",
                editor(|probe| probe.stage_tool == StageToolType::Turn),
                BEAT_SECS,
            )
            .click_named(
                "editor shell: the Move button is the W key",
                "Tool Button Move",
                editor(|probe| probe.stage_tool == StageToolType::Move),
                BEAT_SECS,
            )
            .click_named(
                "editor shell: the Axes button is the X key",
                "Axes Button",
                editor(|probe| probe.gizmo_space == GizmoSpaceType::Local),
                BEAT_SECS,
            )
            .click_named(
                "editor shell: and back",
                "Axes Button",
                editor(|probe| probe.gizmo_space == GizmoSpaceType::World),
                BEAT_SECS,
            )
            .click_named(
                "editor shell: focus the tree filter",
                FILTER,
                frames(3),
                BEAT_SECS,
            )
            .step("editor shell: a filter leaves the rock and its parent")
            .on_enter(type_text("aster"))
            .until(Arc::new(|world: &World| {
                let shell = &world.resource::<EditorProbe>().shell;
                shell.tree_rows > 0 && shell.tree_rows < world.resource::<Walk>().rows
            }))
            .diagnose(readout)
            .deadline(BEAT_SECS)
            .add();
        let script = shot(script, "editor-shell-filter.png");
        let script = script
            .step("editor shell: clear the filter")
            .on_enter(|world: &mut World| {
                for _ in 0..5 {
                    press_edit_key(Key::Backspace)(world);
                }
            })
            .until(Arc::new(|world: &World| {
                world.resource::<EditorProbe>().shell.tree_rows == world.resource::<Walk>().rows
            }))
            .diagnose(readout)
            .deadline(BEAT_SECS)
            .add()
            .step("editor shell: let the field go")
            .on_enter(press_edit_key(Key::Escape))
            .until(frames(3))
            .add()
            .step("editor shell: point at the rock's row")
            .on_enter(|world: &mut World| {
                let row = format!("Scene Row {}", world.resource::<Walk>().rock);
                let at = ui_node_centre(world, &row).expect("the rock's row stands");
                let depth = probe(world).history_depth;
                world.resource_mut::<Walk>().depth = depth;
                move_cursor(at)(world);
            })
            .until(frames(4))
            .add()
            .step("editor shell: press the right button")
            .on_enter(press_mouse(MouseButton::Right))
            .until(frames(3))
            .add()
            .step("editor shell: release it for the row's menu")
            .on_enter(release_mouse(MouseButton::Right))
            .until(ui_node_present("Row Duplicate"))
            .diagnose(readout)
            .deadline(BEAT_SECS)
            .add();
        let script = shot(script, "editor-shell-menu.png");
        let script = script
            .step("editor shell: aim at Duplicate")
            .on_enter(hover_named("Row Duplicate"))
            .until(pointer_over_node("Row Duplicate"))
            .diagnose(readout)
            .deadline(BEAT_SECS)
            .add()
            .step("editor shell: press Duplicate")
            .on_enter(press_mouse(MouseButton::Left))
            .until(frames(3))
            .add()
            .step("editor shell: release on Duplicate")
            .on_enter(release_mouse(MouseButton::Left))
            .until(editor(|probe| {
                probe.next_undo.as_deref() == Some("Duplicate")
            }))
            .diagnose(readout)
            .deadline(20.0)
            .add()
            .step("editor shell: the verb is one step, and the menu went")
            .on_enter(|world: &mut World| {
                let depth = world.resource::<Walk>().depth;
                assert_eq!(probe(world).history_depth, depth + 1);
            })
            .until(Arc::new(|world: &World| {
                !ui_node_present("Row Duplicate")(world)
            }))
            .deadline(BEAT_SECS)
            .add();
        let script = chord(
            script,
            "Ctrl+Z takes it back",
            UNDO,
            Arc::new(|world: &World| {
                world.resource::<EditorProbe>().history_depth == world.resource::<Walk>().depth
            }),
        );

        let script = script.click_named(
            "editor shell: open Assets",
            "Browser Tab Assets",
            editor(|probe| probe.shell.browser_tab == "Assets" && probe.shell.asset_rows > 0),
            BEAT_SECS,
        );
        let script = shot(script, "editor-shell-assets.png");
        let script = script
            .step("editor shell: aim at a ship tile")
            .on_enter(hover_named("Asset Tile hulk_1"))
            .until(pointer_over_node("Asset Tile hulk_1"))
            .diagnose(readout)
            .deadline(BEAT_SECS)
            .add();
        let script = shot(script, "editor-shell-tile-hint.png");
        // A copy of the scenario holds the same nodes: the short names repeat,
        // and the tiles have to say which scenario each one is from.
        let script = script
            .step("editor shell: aim at the scenario tile")
            .on_enter(hover_named("Asset Tile Saved Range"))
            .until(pointer_over_node("Asset Tile Saved Range"))
            .diagnose(readout)
            .deadline(BEAT_SECS)
            .add()
            .step("editor shell: right press the tile")
            .on_enter(press_mouse(MouseButton::Right))
            .until(frames(3))
            .add()
            .step("editor shell: let go for the tile's menu")
            .on_enter(release_mouse(MouseButton::Right))
            .until(ui_node_present("Asset Menu Duplicate"))
            .diagnose(readout)
            .deadline(BEAT_SECS)
            .add()
            .click_named(
                "editor shell: duplicate the scenario",
                "Asset Menu Duplicate",
                editor(|probe| probe.next_undo.as_deref() == Some("Duplicate Scenario")),
                BEAT_SECS,
            );
        let script = script
            .step("editor shell: take the pointer off the tiles")
            .on_enter(hover_named("Asset Filter"))
            .until(pointer_over_node("Asset Filter"))
            .diagnose(readout)
            .deadline(BEAT_SECS)
            .add();
        let script = shot(script, "editor-shell-tile-clash.png");
        let script = chord(
            script,
            "Ctrl+Z takes the copy back",
            UNDO,
            editor(|probe| probe.next_undo.as_deref() != Some("Duplicate Scenario")),
        );
        let script = script.click_named(
            "editor shell: open Problems",
            "Browser Tab Problems",
            editor(|probe| probe.shell.browser_tab == "Problems"),
            BEAT_SECS,
        );
        let script = shot(script, "editor-shell-problems.png");
        let script = log_tab(script);
        let script = resize_strip(script);

        let script = chord(
            script,
            "Ctrl+3 folds the strip",
            FOLD_STRIP,
            editor(|probe| !probe.shell.browser),
        );
        let script = chord(
            script,
            "Ctrl+3 opens it",
            FOLD_STRIP,
            editor(|probe| probe.shell.browser),
        );
        let script = chord(
            script,
            "Ctrl+1 hides the Hierarchy",
            FOLD_HIERARCHY,
            editor(|probe| !probe.shell.hierarchy),
        );
        let script = chord(
            script,
            "Ctrl+1 shows it",
            FOLD_HIERARCHY,
            editor(|probe| probe.shell.hierarchy),
        );
        let script = chord(
            script,
            "Ctrl+2 hides the Inspector",
            FOLD_INSPECTOR,
            editor(|probe| !probe.shell.inspector),
        );
        let script = chord(
            script,
            "Ctrl+2 shows it",
            FOLD_INSPECTOR,
            editor(|probe| probe.shell.inspector),
        );

        let script = script
            .click_named(
                "editor shell: open the Add menu for a ship",
                ADD_MENU,
                ui_node_present("Add Ship Button"),
                BEAT_SECS,
            )
            .click_named(
                "editor shell: add a ship, which enters it",
                "Add Ship Button",
                ui_node_present("Browser Tab Ship"),
                BEAT_SECS,
            )
            .click_named(
                "editor shell: open the Ship tab",
                "Browser Tab Ship",
                editor(|probe| probe.shell.browser_tab == "Ship"),
                BEAT_SECS,
            );
        let script = shot(script, "editor-shell-ship.png");
        let script = script
            .step("editor shell: note where Play stands")
            .on_enter(|world: &mut World| {
                let at = ui_node_centre(world, "Play Button").expect("Play stands");
                world.resource_mut::<Walk>().play_at = at;
            })
            .until(frames(1))
            .add()
            // Events is a view of the SCENARIO: inside a ship the view
            // switch offers the ship's own view only.
            .step("editor shell: the ship offers no Events view")
            .until(Arc::new(|world: &World| {
                ui_node_present("View Tab Ship")(world)
                    && !ui_node_present("View Tab Events")(world)
            }))
            .diagnose(readout)
            .deadline(BEAT_SECS)
            .add()
            .click_named(
                "editor shell: back out to the scenario",
                "Scene Row scenario",
                editor(|probe| probe.inside.is_none()),
                BEAT_SECS,
            )
            .click_named(
                "editor shell: take the Events tab",
                "View Tab Events",
                editor(|probe| probe.shell.rail_tab == "Events"),
                BEAT_SECS,
            );
        let script = shot(script, "editor-shell-events.png");
        let script = script
            .step("editor shell: Play held its place")
            .on_enter(|world: &mut World| {
                let at = ui_node_centre(world, "Play Button").expect("Play stands");
                assert_eq!(at, world.resource::<Walk>().play_at);
            })
            .until(frames(1))
            .add();
        // An Or with nothing in it is the shortest way to a Problem: the save
        // would lose it, so the Problems tab has to say so at once.
        let script = script
            .filter_palette("On Start")
            .drag_block(
                "editor shell: a new handler",
                block_named("Palette On Start"),
                canvas_middle(),
                |held| held,
                editor(move |probe| probe.next_undo.as_deref() == Some("Add On Start")),
            )
            .filter_palette("Logic")
            .drag_block(
                "editor shell: an empty Or",
                block_named("Palette Or"),
                block_centre(|world| {
                    let hat = world.resource::<EditorProbe>().shell.selected_block.clone();
                    format!(
                        "Slot {} filter 0",
                        hat.expect("the new handler is selected")
                    )
                }),
                |held| held,
                editor(move |probe| probe.next_undo.as_deref() == Some("Add Or")),
            );
        // The Or's pop-up holds its Kind, and the keyboard: Del would take the
        // block out from under it. The fault mark rebuilds the handler, so wait
        // for it: `release_mouse` clears the button a frame before picking sees
        // the release, and a lint held back by the button would take the chip
        // away on that frame.
        let chip_name = Arc::new(OnceLock::new());
        let aimed = chip_name.clone();
        let script = script
            .step("editor shell: the lint marks the empty Or")
            .until(editor(|probe| probe.shell.block_faults > 0))
            .diagnose(readout)
            .deadline(BEAT_SECS)
            .add()
            .step("editor shell: point at the Or's more chip")
            .on_enter(move |world: &mut World| {
                let block = world
                    .resource::<EditorProbe>()
                    .shell
                    .selected_block
                    .clone()
                    .expect("the Or is selected");
                let wanted = format!("/{block}");
                let mut names = world.query::<&Name>();
                let chip = names
                    .iter(world)
                    .map(|name| name.as_str().to_string())
                    .find(|name| name.starts_with("More ") && name.ends_with(&wanted))
                    .expect("an Or block reaches its Kind row");
                hover_named(chip.clone())(world);
                aimed.set(chip).expect("the step enters once");
            })
            .until(Arc::new(move |world: &World| {
                chip_name
                    .get()
                    .is_some_and(|chip: &String| pointer_over_node(chip.clone())(world))
            }))
            .diagnose(readout)
            .deadline(BEAT_SECS)
            .add()
            .step("editor shell: press the more chip")
            .on_enter(press_mouse(MouseButton::Left))
            .until(pointer_pressed())
            .diagnose(readout)
            .deadline(BEAT_SECS)
            .add()
            .step("editor shell: release on the more chip")
            .on_enter(release_mouse(MouseButton::Left))
            .until(ui_node_present("Block Popup Catcher"))
            .diagnose(readout)
            .deadline(BEAT_SECS)
            .add()
            .step("editor shell: the pop-up lists the kind row")
            .until(ui_node_present("Inspector Row Filter"))
            .diagnose(readout)
            .deadline(BEAT_SECS)
            .add();
        let script = shot(script, "editor-shell-popup.png");
        let script = script
            .step("editor shell: Del under the pop-up")
            .on_enter(press_key(KeyCode::Delete))
            .until(frames(4))
            .add()
            .step("editor shell: let go of Del")
            .on_enter(release_key(KeyCode::Delete))
            .until(frames(3))
            .add()
            .step("editor shell: Del stopped at the pop-up")
            .on_enter(|world: &mut World| {
                let probe = world.resource::<EditorProbe>();
                assert_eq!(probe.next_undo.as_deref(), Some("Add Or"));
            })
            .until(ui_node_present("Block Popup Catcher"))
            .diagnose(readout)
            .deadline(BEAT_SECS)
            .add()
            .step("editor shell: Esc closes the pop-up")
            .on_enter(press_key(KeyCode::Escape))
            .until(Arc::new(|world: &World| {
                !ui_node_present("Block Popup Catcher")(world)
            }))
            .diagnose(readout)
            .deadline(BEAT_SECS)
            .add()
            .step("editor shell: let go of Esc")
            .on_enter(release_key(KeyCode::Escape))
            .until(frames(3))
            .add();
        let script = script
            .click_named(
                "editor shell: back to Problems",
                "Browser Tab Problems",
                editor(|probe| probe.shell.browser_tab == "Problems"),
                BEAT_SECS,
            )
            .step("editor shell: the gate shows up without a save")
            .until(editor(|probe| probe.shell.errors > 0))
            .diagnose(readout)
            .deadline(BEAT_SECS)
            .add();
        let script = shot(script, "editor-shell-lint.png");
        script
            .step("editor shell: done")
            .on_enter(|world: &mut World| info!("editor shell: {}", readout(world)))
            .add()
    }
}
