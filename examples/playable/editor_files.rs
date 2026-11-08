//! editor_files: the Events palette, the right-click menus, the Form view and
//! the mod folder's files in Assets.
//!
//! Boots the shipped app on a profile and mod cache of its own and opens the Mod
//! Hub, so nothing here reaches your real mods.
//!
//! Try: name a mod and press Create. Take Events: the palette groups its blocks
//! by role, hats first, and a hover says where a block goes. Drag a block and
//! the slots it fits light up. Right click the canvas for Paste and Clean Up.
//! Add a scenario with + and take Events: the empty canvas says to start with
//! an Events block. Right click the stage for the Add verbs and Paste. In
//! Assets press + > Import File... and pick a png, a glb and a wav: each lands
//! as a tile with its picture or glyph. Double click the image to view it and
//! the sound to play it. Delete the sound: it leaves the package and stays in
//! the folder, with a dotted mark. + > UI Themes opens a form in the main area.
//!
//! The panel reads the project's undo depth, unsaved mark and the status line.
//!
//! With `NOVA_AUTOPILOT=1` a smoke walk does all of that through the real
//! widgets. `NOVA_CAPTURE=1` shoots the palette, a drag, the canvas and stage
//! menus, the empty canvas, the file tiles, the file menu, the image viewer
//! and the form. Unset, no walk and no deadline run.

use bevy::prelude::*;
use clap::Parser;
use nova_protocol::{nova_debug::diagnostics, prelude::*};

#[path = "shared/open_world_stage.rs"]
mod open_world_stage;
#[path = "../screenshots/shared/ui_walk.rs"]
mod ui_walk;

/// Command line for `editor_files`.
#[derive(Parser)]
#[command(name = "editor_files", version, about = "Palette, menus and mod files", long_about = None)]
struct Cli {
    /// Window size as `WIDTHxHEIGHT`, for the narrow-screen check.
    #[arg(long, value_parser = parse_size)]
    size: Option<(f32, f32)>,
}

fn parse_size(text: &str) -> Result<(f32, f32), String> {
    let (width, height) = text
        .split_once('x')
        .ok_or_else(|| format!("{text} is not WIDTHxHEIGHT"))?;
    let number = |part: &str| part.parse::<f32>().map_err(|error| error.to_string());
    Ok((number(width)?, number(height)?))
}

const INTRO: &str = "Name a mod and press Create. Assets > + > Import File... adds a file.";

fn main() -> bevy::app::AppExit {
    let cli = Cli::parse();
    open_world_stage::isolate_the_profile(env!("CARGO_CRATE_NAME"));

    let mut app = editor_app(true, None);
    if !capturing() {
        app.add_plugins(diagnostics::DiagnosticsHintPlugin::new(INTRO));
    }
    app.add_systems(OnEnter(GameStates::MainMenu), open_the_hub);
    app.add_systems(
        PostUpdate,
        read_the_project
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
        app.add_plugins(smoke::script(cli.size.map_or(1920, |size| size.0 as u32)));
    }
    if let Some(size) = cli.size {
        app.add_systems(
            Startup,
            (move |mut windows: Query<&mut Window>| {
                for mut window in &mut windows {
                    window.resolution.set(size.0, size.1);
                }
            })
            .after(nova_protocol::nova_debug::harness::force_capture_resolution),
        );
    }

    app.run()
}

/// What the Sandbox button does, once.
fn open_the_hub(
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

/// The diagnostics panel: where the project stands.
fn read_the_project(world: &mut World) {
    let Some(probe) = world.get_resource::<EditorProbe>().cloned() else {
        return;
    };
    let mut panel = world.resource_mut::<diagnostics::DiagnosticsPanel>();
    panel.line(format!(
        "Undo steps: {}{}",
        probe.history_depth,
        if probe.dirty { ", unsaved" } else { ", saved" }
    ));
    panel.line(format!("Status: {}", probe.status));
}

mod smoke {
    use std::sync::Arc;

    use nova_protocol::nova_debug::harness::{shoot, AutopilotPlugin, Predicate};

    use super::{
        ui_walk::{block_named, canvas_middle, Gestures},
        *,
    };

    /// Seconds a cold boot gets on a software GPU.
    const BOOT_SECS: f32 = 300.0;
    /// Seconds a beat gets.
    const BEAT_SECS: f32 = 120.0;
    /// Seconds a frame capture gets.
    const SHOT_SECS: f32 = 120.0;
    const MOD_NAME: &str = "File Works";
    const FILE_MENU: &str = "File Menu Button";
    const SCENARIO_TILE: &str = "Asset Tile Saved Range";
    const CANVAS: &str = "Block Canvas";
    /// The files the walk imports, from the game's own assets, and the tiles
    /// they land as.
    const FILES: [(&str, &str); 3] = [
        (
            "assets/icons/nova_crt_mark.png",
            "Asset Tile nova_crt_mark.png",
        ),
        ("assets/base/gltf/bay_tube.glb", "Asset Tile bay_tube.glb"),
        (
            "assets/sounds/editor_place.wav",
            "Asset Tile editor_place.wav",
        ),
    ];

    /// The status line, the undo label and the tab, for a stalled beat.
    fn readout(world: &World) -> String {
        world.get_resource::<EditorProbe>().map_or_else(
            || "no probe".to_string(),
            |probe| {
                format!(
                    "status {:?} depth {} next {:?} tab {:?}",
                    probe.status, probe.history_depth, probe.next_undo, probe.shell.rail_tab,
                )
            },
        )
    }

    /// Advance once `check` holds of the probe.
    fn editor(check: impl Fn(&EditorProbe) -> bool + Send + Sync + 'static) -> Arc<Predicate> {
        Arc::new(move |world: &World| world.get_resource::<EditorProbe>().is_some_and(&check))
    }

    /// Advance once the status line holds `text`.
    fn said(text: &'static str) -> Arc<Predicate> {
        editor(move |probe| probe.status.contains(text))
    }

    /// Shoot `name` at this run's width.
    fn shot(
        script: AutopilotPlugin<GameStates>,
        width: u32,
        name: &'static str,
    ) -> AutopilotPlugin<GameStates> {
        let file: &'static str = Box::leak(format!("files-{width}-{name}.png").into_boxed_str());
        script
            .step(format!("files: settle before {file}"))
            .until(frames(4))
            .add()
            .step(format!("files: shoot {file}"))
            .on_enter(move |world: &mut World| shoot(world, file))
            .until(shot_written(file))
            .deadline(SHOT_SECS)
            .add()
    }

    /// Say a PASS line.
    fn pass(
        script: AutopilotPlugin<GameStates>,
        line: &'static str,
    ) -> AutopilotPlugin<GameStates> {
        script
            .step(format!("files: {line}"))
            .on_enter(move |_world: &mut World| info!("files: PASS {line}"))
            .add()
    }

    /// Advance once the heading `name` stands inside the palette list's box.
    fn heading_in_list(name: &'static str) -> Arc<Predicate> {
        Arc::new(move |world: &World| {
            let (Some(heading), Some(list)) = (
                ui_node_rect(world, name),
                ui_node_rect(world, "Palette List"),
            ) else {
                return false;
            };
            heading.min.y >= list.min.y - 1.0 && heading.max.y <= list.max.y + 1.0
        })
    }

    /// Advance once the canvas hint shows a line holding `text`.
    fn hint_says(text: &'static str) -> Arc<Predicate> {
        Arc::new(move |world: &World| {
            ui_node_present("Canvas Hint")(world)
                && world
                    .try_query::<(&Text, &InheritedVisibility)>()
                    .is_some_and(|mut texts| {
                        texts
                            .iter(world)
                            .any(|(said, shown)| shown.get() && said.0.contains(text))
                    })
        })
    }

    /// Write `text` into the text field `name`.
    fn type_into(name: &'static str, text: String) -> impl Fn(&mut World) + Send + Sync + 'static {
        move |world: &mut World| {
            let mut fields = world.query::<(&Name, &mut nova_ui::prelude::TextFieldValue)>();
            let (_, mut field) = fields
                .iter_mut(world)
                .find(|(named, _)| named.as_str() == name)
                .unwrap_or_else(|| panic!("{name} stands"));
            field.0.clone_from(&text);
        }
    }

    /// A right click with the harness pointer at the point `at` gives, opening
    /// the menu whose row `row` names.
    fn right_click(
        script: AutopilotPlugin<GameStates>,
        label: &str,
        at: impl Fn(&World) -> Option<Vec2> + Send + Sync + 'static,
        row: &'static str,
    ) -> AutopilotPlugin<GameStates> {
        script
            .step(format!("files: point for {label}"))
            .on_enter(move |world: &mut World| {
                let point = at(world).expect("the target is on screen");
                move_cursor(point)(world);
            })
            .until(frames(3))
            .add()
            .step(format!("files: press right for {label}"))
            .on_enter(press_mouse(MouseButton::Right))
            .until(frames(2))
            .add()
            .step(format!("files: release right for {label}"))
            .on_enter(release_mouse(MouseButton::Right))
            .until(ui_node_present(row))
            .diagnose(readout)
            .deadline(BEAT_SECS)
            .add()
    }

    /// Put a menu away with Esc.
    fn escape(
        script: AutopilotPlugin<GameStates>,
        gone: &'static str,
    ) -> AutopilotPlugin<GameStates> {
        script
            .step(format!("files: Esc puts {gone} away"))
            .on_enter(press_key(KeyCode::Escape))
            .until(Arc::new(move |world: &World| !ui_node_present(gone)(world)))
            .deadline(BEAT_SECS)
            .add()
            .step(format!("files: let Esc go after {gone}"))
            .on_enter(release_key(KeyCode::Escape))
            .until(frames(2))
            .add()
    }

    /// Hub, a new mod, and the Assets tab.
    fn make_the_mod(script: AutopilotPlugin<GameStates>) -> AutopilotPlugin<GameStates> {
        script
            .step("files: the hub is up")
            .until(ui_node_present("Hub New Button"))
            .diagnose(readout)
            .deadline(BOOT_SECS)
            .add()
            .click_named(
                "files: open New",
                "Hub New Button",
                ui_node_present("Hub Create Button"),
                BEAT_SECS,
            )
            .step("files: name the mod")
            .on_enter(type_into("Hub Name Field", MOD_NAME.to_string()))
            .until(frames(3))
            .add()
            .click_named(
                "files: create it",
                "Hub Create Button",
                ui_node_present(FILE_MENU),
                BOOT_SECS,
            )
            .click_named(
                "files: show the mod's contents",
                "Browser Tab Assets",
                ui_node_present(SCENARIO_TILE),
                BEAT_SECS,
            )
    }

    /// The palette, a drag with its slots lit, and the canvas menu.
    fn the_canvas(script: AutopilotPlugin<GameStates>, width: u32) -> AutopilotPlugin<GameStates> {
        let script = script
            .click_named(
                "files: take Events",
                "View Tab Events",
                editor(|probe| probe.shell.rail_tab == "Events" && probe.shell.blocks > 0),
                BEAT_SECS,
            )
            .step("files: the palette groups by role, hats first")
            .until(Arc::new(|world: &World| {
                let (Some(events), Some(actions)) = (
                    ui_node_rect(world, "Palette Heading Events"),
                    ui_node_rect(world, "Palette Heading Actions"),
                ) else {
                    return false;
                };
                events.min.y < actions.min.y
            }))
            .deadline(BEAT_SECS)
            .add();
        let script = pass(script, "the palette lists Events above Actions");
        let script = shot(script, width, "palette");
        let script = script.click_named(
            "files: the rail takes the list to Values",
            "Palette Rail Values",
            heading_in_list("Palette Heading Values and logic"),
            BEAT_SECS,
        );
        let script = pass(script, "a rail click brings the Values group into view");
        let script = shot(script, width, "rail");
        let script = script.click_named(
            "files: the rail takes the list back to Events",
            "Palette Rail Events",
            heading_in_list("Palette Heading Events"),
            BEAT_SECS,
        );
        let script = script.filter_palette("Debug Message").drag_block(
            "files: carry an action",
            block_named("Palette Debug Message"),
            canvas_middle(),
            |held| {
                if capturing() {
                    shot(held, width, "drag")
                } else {
                    held
                }
            },
            pointer_released(),
        );
        let script = script
            .step("files: settle after the drag")
            .until(frames(4))
            .add();
        let script = script.filter_palette("");
        let script = right_click(
            script,
            "the canvas menu",
            |world| ui_node_rect(world, CANVAS).map(|rect| rect.max - Vec2::splat(40.0)),
            "Canvas Clean Up",
        );
        let script = pass(script, "a right click on the canvas offers Clean Up");
        let script = shot(script, width, "canvas-menu");
        let script = script
            .click_named(
                "files: Clean Up",
                "Canvas Clean Up",
                editor(|probe| probe.next_undo.as_deref() == Some("Clean Up")),
                BEAT_SECS,
            )
            .step("files: settle after Clean Up")
            .until(frames(4))
            .add();
        let script = pass(script, "Clean Up is one undo step");
        shot(script, width, "cleaned-up")
    }

    /// A blank scenario: its empty canvas says how to start, and its stage
    /// menu offers the scenario's Add verbs.
    fn the_blank_scenario(
        script: AutopilotPlugin<GameStates>,
        width: u32,
    ) -> AutopilotPlugin<GameStates> {
        let script = script
            .click_named(
                "files: open the + menu for a scenario",
                "Asset Add Button",
                ui_node_present("Asset Add Scenarios"),
                BEAT_SECS,
            )
            .click_named(
                "files: a blank scenario",
                "Asset Add Scenarios",
                editor(|probe| {
                    probe
                        .next_undo
                        .as_deref()
                        .is_some_and(|step| step.contains("Scenario"))
                }),
                BEAT_SECS,
            )
            .click_named(
                "files: take Events on the blank",
                "View Tab Events",
                ui_node_present("Canvas Hint"),
                BEAT_SECS,
            );
        let script = pass(
            script,
            "the empty canvas says to start with an Events block",
        );
        let script = shot(script, width, "empty-canvas");
        let script = script.filter_palette("Debug Message").drag_block(
            "files: hold an action over the bare canvas",
            block_named("Palette Debug Message"),
            canvas_middle(),
            |held| {
                let held = held
                    .step("files: the canvas says why a loose block will not stand")
                    .until(hint_says("stands alone"))
                    .diagnose(readout)
                    .deadline(BEAT_SECS)
                    .add();
                let held = pass(
                    held,
                    "a loose block over bare canvas says why it is refused",
                );
                if capturing() {
                    shot(held, width, "loose")
                } else {
                    held
                }
            },
            pointer_released(),
        );
        let script = script
            .step("files: the loose drop added nothing")
            .until(editor(|probe| {
                probe.shell.blocks == 0 && probe.next_undo.as_deref() == Some("Add Scenario")
            }))
            .diagnose(readout)
            .deadline(BEAT_SECS)
            .add()
            .filter_palette("");
        let script = pass(script, "a loose drop is refused and makes no step");
        let script = script.click_named(
            "files: take Scene",
            "View Tab Scene",
            editor(|probe| probe.shell.rail_tab == "Scene"),
            BEAT_SECS,
        );
        let script = right_click(
            script,
            "the stage menu",
            |world| {
                let window = world
                    .iter_entities()
                    .find_map(|entity| entity.get::<Window>().map(|window| window.size()))?;
                Some(Vec2::new(window.x * 0.5, window.y * 0.35))
            },
            "Stage Add Ship",
        );
        let script = pass(
            script,
            "a right click on the stage offers the scenario's Add verbs",
        );
        let script = shot(script, width, "stage-menu");
        escape(script, "Stage Add Ship")
    }

    /// Import the three files through + > Import File...
    fn import(
        script: AutopilotPlugin<GameStates>,
        (path, tile): (&'static str, &'static str),
    ) -> AutopilotPlugin<GameStates> {
        let full = std::env::current_dir()
            .expect("a working directory")
            .join(path)
            .display()
            .to_string();
        script
            .click_named(
                &format!("files: + for {path}"),
                "Asset Add Button",
                ui_node_present("Asset Add Import File"),
                BEAT_SECS,
            )
            .click_named(
                &format!("files: Import File for {path}"),
                "Asset Add Import File",
                ui_node_present("Path Field"),
                BEAT_SECS,
            )
            .step(format!("files: type {path}"))
            .on_enter(type_into("Path Field", full))
            .until(frames(3))
            .add()
            .click_named(
                &format!("files: pick {path}"),
                "File Save Button",
                ui_node_present(tile),
                BEAT_SECS,
            )
    }

    /// The file tiles, the file menu, the image viewer and the sound.
    fn the_files(script: AutopilotPlugin<GameStates>, width: u32) -> AutopilotPlugin<GameStates> {
        let mut script = script
            .click_named(
                "files: back to Assets",
                "Browser Tab Assets",
                ui_node_present("Asset Add Button"),
                BEAT_SECS,
            )
            .step("files: show only the files")
            .on_enter(type_into("Asset Filter", ".".to_string()))
            .until(frames(3))
            .add();
        for file in FILES {
            script = import(script, file);
        }
        let script = script
            .step("files: each import is one undo step")
            .until(editor(|probe| {
                probe.next_undo.as_deref() == Some("Import File")
            }))
            .diagnose(readout)
            .deadline(BEAT_SECS)
            .add();
        let script = pass(script, "three files land as tiles, each one undo step");
        let (_, sound) = FILES[2];
        let script = right_click(
            script,
            "the sound's menu",
            move |world| ui_node_centre(world, sound),
            "Asset Menu Delete",
        );
        let script = shot(script, width, "file-menu");
        let script = script
            .click_named(
                "files: Delete the sound",
                "Asset Menu Delete",
                editor(|probe| probe.next_undo.as_deref() == Some("Remove File")),
                BEAT_SECS,
            )
            .step("files: the sound wears the out-of-package mark")
            .until(ui_node_present("Asset Tile Unpackaged"))
            .diagnose(readout)
            .deadline(BEAT_SECS)
            .add();
        let script = pass(
            script,
            "a deleted declared file stays in the folder, marked",
        );
        // The mesh picture is rendered off screen and takes its time on a
        // software GPU.
        let script = script
            .step("files: let the pictures come in")
            .until(frames(90))
            .add();
        let script = shot(script, width, "assets");
        let (_, image) = FILES[0];
        let script = script
            .double_click_named("files: view the image", image, BEAT_SECS)
            .step("files: the viewer stands")
            .until(ui_node_present("File Viewer Picture"))
            .diagnose(readout)
            .deadline(BEAT_SECS)
            .add();
        let script = pass(script, "a double click views the image large");
        let script = shot(script, width, "image-viewer");
        let script = escape(script, "File Viewer Picture");
        let script = script
            .double_click_named("files: play the sound", sound, BEAT_SECS)
            .step("files: the sound plays")
            .until(said("playing editor_place.wav"))
            .diagnose(readout)
            .deadline(BEAT_SECS)
            .add();
        pass(script, "a double click plays the sound")
    }

    /// A blank UI theme: its form takes the main area.
    fn the_form(script: AutopilotPlugin<GameStates>, width: u32) -> AutopilotPlugin<GameStates> {
        let script = script
            .step("files: show every tile again")
            .on_enter(type_into("Asset Filter", String::new()))
            .until(frames(3))
            .add()
            .click_named(
                "files: open the + menu for a theme",
                "Asset Add Button",
                ui_node_present("Asset Add UI Themes"),
                BEAT_SECS,
            )
            .click_named(
                "files: a blank theme",
                "Asset Add UI Themes",
                ui_node_present("View Tab Form"),
                BEAT_SECS,
            )
            .step("files: the stage tools hide in Form")
            .until(Arc::new(|world: &World| {
                !ui_node_present("Tool Button Move")(world)
            }))
            .diagnose(readout)
            .deadline(BEAT_SECS)
            .add();
        let script = pass(
            script,
            "the form takes the main area and the stage tools hide",
        );
        shot(script, width, "form")
    }

    pub(super) fn script(width: u32) -> AutopilotPlugin<GameStates> {
        let script = make_the_mod(AutopilotPlugin::<GameStates>::new())
            .double_click_named("files: open the scenario", SCENARIO_TILE, BEAT_SECS)
            .step("files: the scenario is the document")
            .until(editor(|probe| probe.inside.is_none()))
            .diagnose(readout)
            .deadline(BEAT_SECS)
            .add();
        let script = the_canvas(script, width);
        let script = the_blank_scenario(script, width);
        let script = the_files(script, width);
        the_form(script, width)
    }
}
