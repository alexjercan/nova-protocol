//! editor_mod_project: the Mod Hub and one mod worked through to a package.
//!
//! Boots the shipped app and presses the menu's Sandbox button on a profile
//! and mod cache of its own, so nothing here reaches your real mods.
//!
//! Try: type a name and press Create. In the Assets tab press + for a second
//! scenario, and open either with a double click. File > Mod Settings edits the
//! manifest and the Dependencies list. Ctrl+S saves, Ctrl+Z walks back through
//! project steps and scenario edits alike. File > Package opens a window with
//! the version, its bumps, the output folder, the check and the files, and
//! writes a zip; the hub's Import reads one back through the path window. The
//! Log tab's rows copy out: click, Shift+click, Ctrl+A, then Copy or Ctrl+C. Play flies the
//! open scenario, and the mod is in the Scenarios list after Back to Main Menu.
//!
//! The panel reads the project's undo depth, unsaved mark and the status line.
//!
//! With `NOVA_AUTOPILOT=1` a smoke walk does all of that through real buttons:
//! new mod, two scenarios, a dependency, save, reopen through the hub, a
//! refused package (a declared file gone from disk), undo, a bump undone in one
//! step, package, Copy Path, copy out of the Log, import over itself, play, and
//! the mod in the Scenarios list. `NOVA_CAPTURE=1` shoots the hub, the New
//! window, the settings window, the Assets tab, the Package window, the package
//! result and the copied Log rows.
//! Unset, no walk and no deadline run.

use bevy::prelude::*;
use clap::Parser;
use nova_protocol::{nova_debug::diagnostics, prelude::*};

#[path = "shared/open_world_stage.rs"]
mod open_world_stage;

/// Command line for `editor_mod_project`.
#[derive(Parser)]
#[command(name = "editor_mod_project", version, about = "The Mod Hub and one mod", long_about = None)]
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

const INTRO: &str = "Name a mod and press Create. File > Package writes a zip.";

fn main() -> bevy::app::AppExit {
    let cli = Cli::parse();
    open_world_stage::isolate_the_profile(env!("CARGO_CRATE_NAME"));

    let mut app = editor_app(true, None);
    // The panel stands over the top bar, which is what the frames are of.
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
    }
    app.add_plugins(smoke::script());
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

/// What the Sandbox button does, once: a later Back to Main Menu stays on the
/// menu, where the saved mod is in the Scenarios list.
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

    use nova_assets::mod_cache::prelude::{exports_dir, mod_file_path, read_index};
    use nova_protocol::nova_debug::harness::{shoot, AutopilotPlugin, Predicate};
    use nova_ui::prelude::{Selected, TextFieldValue};

    use super::*;

    /// Seconds a cold boot gets on a software GPU.
    const BOOT_SECS: f32 = 300.0;
    /// Seconds a beat gets.
    const BEAT_SECS: f32 = 120.0;
    /// Seconds a frame capture gets.
    const SHOT_SECS: f32 = 120.0;
    /// Seconds the flight scene and a content restart get.
    const PLAY_SECS: f32 = 300.0;
    const MOD_NAME: &str = "Red Run";
    const MOD_ID: &str = "editor_red-run";
    const MOD_ROW: &str = "Hub Row editor_red-run";
    const FILE_MENU: &str = "File Menu Button";
    const HUB_SHOT: &str = "editor-mod-hub.png";
    const NEW_SHOT: &str = "editor-mod-new.png";
    const SETTINGS_SHOT: &str = "editor-mod-settings.png";
    const DEPS_SHOT: &str = "editor-mod-deps.png";
    const REFUSED_SHOT: &str = "editor-mod-deps-refused.png";
    const ASSETS_SHOT: &str = "editor-mod-assets.png";
    const REFUSED_PACKAGE_SHOT: &str = "s10-package-refused.png";
    const PACKAGE_SHOT: &str = "s10-package-modal.png";
    const DONE_SHOT: &str = "s10-package-done.png";
    const COPY_SHOT: &str = "s5-copy.png";
    const SAVE: &[KeyCode] = &[KeyCode::ControlLeft, KeyCode::KeyS];
    const UNDO: &[KeyCode] = &[KeyCode::ControlLeft, KeyCode::KeyZ];
    /// A file the walk imports and then takes off disk, so the check finds a
    /// declared file missing.
    const IMPORTED: &str = "walk_notes.txt";
    /// A file the walk drops in the mod's folder undeclared: not packaged.
    const LOOSE: &str = "loose_notes.txt";

    /// The status line, for a stalled beat to print.
    fn readout(world: &World) -> String {
        let hub = texts(world)
            .into_iter()
            .filter(|text| text.contains("mod") || text.contains("import"))
            .take(6)
            .collect::<Vec<_>>();
        world.get_resource::<EditorProbe>().map_or_else(
            || format!("no probe, {hub:?}"),
            |probe| format!("status {:?} dirty {}, {hub:?}", probe.status, probe.dirty),
        )
    }

    /// Every piece of text standing.
    fn texts(world: &World) -> Vec<String> {
        world
            .try_query::<&Text>()
            .map_or_else(Vec::new, |mut texts| {
                texts.iter(world).map(|text| text.0.clone()).collect()
            })
    }

    /// Advance once some text holds `needle`.
    fn reads(needle: &'static str) -> Arc<Predicate> {
        Arc::new(move |world: &World| texts(world).iter().any(|text| text.contains(needle)))
    }

    /// Advance once the kind column's `heading` row counts `count` items.
    fn kind_counts(heading: &'static str, count: usize) -> Arc<Predicate> {
        Arc::new(move |world: &World| {
            let wanted = format!("Asset Kind {heading}");
            let count = count.to_string();
            let Some(mut rows) = world.try_query::<(Entity, &Name)>() else {
                return false;
            };
            let Some(row) = rows
                .iter(world)
                .find(|(_, name)| name.as_str() == wanted)
                .map(|(row, _)| row)
            else {
                return false;
            };
            let mut stack = vec![row];
            while let Some(at) = stack.pop() {
                let entity = world.entity(at);
                if entity.get::<Text>().is_some_and(|text| text.0 == count) {
                    return true;
                }
                if let Some(children) = entity.get::<Children>() {
                    stack.extend(children.iter());
                }
            }
            false
        })
    }

    /// Advance once a save reports itself and the unsaved mark is gone.
    fn saved_and_clean() -> Arc<Predicate> {
        Arc::new(|world: &World| {
            world
                .get_resource::<EditorProbe>()
                .is_some_and(|probe| !probe.dirty && probe.status.starts_with("saved"))
        })
    }

    /// Write `value` into the field named `name`, as typing would.
    fn fill(world: &mut World, name: &str, value: &str) {
        let mut fields = world.query::<(&Name, &mut TextFieldValue)>();
        let (_, mut field) = fields
            .iter_mut(world)
            .find(|(named, _)| named.as_str() == name)
            .unwrap_or_else(|| panic!("{name} stands"));
        field.0 = value.to_string();
    }

    /// Advance once the status line says something that starts with one of
    /// `starts`.
    fn status_starts(starts: &'static [&'static str]) -> Arc<Predicate> {
        Arc::new(move |world: &World| {
            world
                .get_resource::<EditorProbe>()
                .is_some_and(|probe| starts.iter().any(|start| probe.status.starts_with(start)))
        })
    }

    /// The status line now.
    fn status(world: &World) -> String {
        world
            .get_resource::<EditorProbe>()
            .map_or_else(String::new, |probe| probe.status.clone())
    }

    /// How many undo steps the history holds.
    fn depth(world: &World) -> usize {
        world
            .get_resource::<EditorProbe>()
            .map_or(0, |probe| probe.history_depth)
    }

    /// The names of the Log rows marked for copying, sorted.
    fn marked_log_rows(world: &World) -> Vec<String> {
        let mut rows: Vec<String> = world
            .try_query_filtered::<&Name, With<Selected>>()
            .map_or_else(Vec::new, |mut rows| {
                rows.iter(world)
                    .filter(|name| name.as_str().starts_with("Log Row "))
                    .map(|name| name.to_string())
                    .collect()
            });
        rows.sort();
        rows
    }

    /// How many Log rows stand.
    fn log_rows(world: &World) -> usize {
        world.try_query::<&Name>().map_or(0, |mut names| {
            names
                .iter(world)
                .filter(|name| name.as_str().starts_with("Log Row "))
                .count()
        })
    }

    /// Hold `keys` down, wait for `landed`, and let them go.
    fn chord(
        script: AutopilotPlugin<GameStates>,
        label: &str,
        keys: &'static [KeyCode],
        landed: Arc<Predicate>,
    ) -> AutopilotPlugin<GameStates> {
        script
            .step(format!("mod project: {label}"))
            .on_enter(move |world: &mut World| {
                for key in keys {
                    press_key(*key)(world);
                }
            })
            .until(landed)
            .diagnose(readout)
            .deadline(BEAT_SECS)
            .add()
            .step(format!("mod project: let go after {label}"))
            .on_enter(move |world: &mut World| {
                for key in keys {
                    release_key(*key)(world);
                }
            })
            .until(frames(2))
            .add()
    }

    /// The zip Package wrote, by the only name it can have here.
    fn package_path() -> String {
        exports_dir()
            .expect("the data dir resolves")
            .join("red-run-0.1.0.zip")
            .to_string_lossy()
            .into_owned()
    }

    /// Shoot `shot`.
    fn shot(
        script: AutopilotPlugin<GameStates>,
        shot: &'static str,
    ) -> AutopilotPlugin<GameStates> {
        script
            .step(format!("mod project: shoot {shot}"))
            .on_enter(move |world: &mut World| shoot(world, shot))
            .until(shot_written(shot))
            .deadline(SHOT_SECS)
            .add()
    }

    /// Drop the File menu and press `row`.
    fn file_row(
        script: AutopilotPlugin<GameStates>,
        label: &str,
        row: &str,
        landed: Arc<Predicate>,
    ) -> AutopilotPlugin<GameStates> {
        script
            .click_named(
                &format!("mod project: {label}: open the menu"),
                FILE_MENU,
                ui_node_present(row),
                BEAT_SECS,
            )
            .click_named(&format!("mod project: {label}"), row, landed, BEAT_SECS)
    }

    pub(super) fn script() -> AutopilotPlugin<GameStates> {
        let script = AutopilotPlugin::<GameStates>::new()
            .step("mod project: the button opens the hub")
            .until(ui_node_present("Hub New Button"))
            .diagnose(readout)
            .deadline(BOOT_SECS)
            .add();
        let script = shot(script, HUB_SHOT)
            .click_named(
                "mod project: press New",
                "Hub New Button",
                ui_node_present("Hub Create Button"),
                BEAT_SECS,
            )
            .step("mod project: name the mod")
            .on_enter(|world: &mut World| fill(world, "Hub Name Field", MOD_NAME))
            .until(reads("saves as editor_red-run"))
            .diagnose(readout)
            .deadline(BEAT_SECS)
            .add();
        let script = shot(script, NEW_SHOT).click_named(
            "mod project: create it",
            "Hub Create Button",
            ui_node_present(FILE_MENU),
            PLAY_SECS,
        );
        let script = script
            .step("mod project: the mod is on disk before any edit")
            .on_enter(|world: &mut World| {
                let index = read_index().unwrap_or_default();
                assert!(
                    index.iter().any(|record| record.id == MOD_ID),
                    "New Mod writes the mod: {index:?}"
                );
                info!("mod project: created {MOD_ID}");
                let _ = world;
            })
            .add()
            .click_named(
                "mod project: show the mod's contents",
                "Browser Tab Assets",
                kind_counts("Scenarios", 1),
                BEAT_SECS,
            )
            .click_named(
                "mod project: open the Add menu",
                "Asset Add Button",
                ui_node_present("Asset Add Scenarios"),
                BEAT_SECS,
            )
            .click_named(
                "mod project: add a blank scenario",
                "Asset Add Scenarios",
                kind_counts("Scenarios", 2),
                BEAT_SECS,
            );
        let script = shot(script, ASSETS_SHOT);
        let script = file_row(
            script,
            "open Mod Settings",
            "Mod Settings Item",
            ui_node_present("Dependency Add"),
        )
        .click_named(
            "mod project: open Add dependency",
            "Dependency Add",
            ui_node_present("List Pick example"),
            BEAT_SECS,
        );
        let script = shot(script, DEPS_SHOT)
            .click_named(
                "mod project: depend on the example mod",
                "List Pick example",
                ui_node_present("Dependency Row example"),
                BEAT_SECS,
            )
            // The template's ships name sections the example mod also
            // defines, so its x is refused and the status line says by what.
            .click_named(
                "mod project: the example mod is in use, so its x is refused",
                "Dependency Remove example",
                Arc::new(|world: &World| {
                    reads("example is in use by")(world)
                        && ui_node_present("Dependency Row example")(world)
                }),
                BEAT_SECS,
            );
        let script = shot(script, REFUSED_SHOT)
            .click_named(
                "mod project: Undo takes the add back in one step",
                "Undo Button",
                Arc::new(|world: &World| {
                    !ui_node_present("Dependency Row example")(world)
                        && ui_node_present("Dependencies Empty")(world)
                }),
                BEAT_SECS,
            )
            .click_named(
                "mod project: open Add dependency again",
                "Dependency Add",
                ui_node_present("List Pick example"),
                BEAT_SECS,
            )
            .click_named(
                "mod project: depend on the example mod again",
                "List Pick example",
                ui_node_present("Dependency Row example"),
                BEAT_SECS,
            )
            .step("mod project: the dependency list holds example")
            .on_enter(|_world: &mut World| {
                info!("mod project: PASS dependency add, refused x, undo, re-add")
            })
            .until(frames(2))
            .add();
        let script = shot(script, SETTINGS_SHOT)
            .step("mod project: Ctrl+S saves the project")
            .on_enter(|world: &mut World| {
                for key in SAVE {
                    press_key(*key)(world);
                }
            })
            .until(saved_and_clean())
            .diagnose(readout)
            .deadline(BEAT_SECS)
            .add()
            .step("mod project: let go of the save")
            .on_enter(|world: &mut World| {
                for key in SAVE {
                    release_key(*key)(world);
                }
            })
            .until(frames(2))
            .add();
        let script = file_row(
            script,
            "back to the hub",
            "Open Mod Item",
            ui_node_present("Hub New Button"),
        )
        .step("mod project: the hub lists it")
        .until(Arc::new(|world: &World| {
            reads(MOD_NAME)(world) && ui_node_present(MOD_ROW)(world)
        }))
        .diagnose(readout)
        .deadline(BEAT_SECS)
        .add();
        let script = open_world_stage::open_hub_row(
            script,
            "mod project: reopen it",
            MOD_ROW,
            ui_node_present(FILE_MENU),
            PLAY_SECS,
        )
        .click_named(
            "mod project: the reopened mod holds both",
            "Browser Tab Assets",
            kind_counts("Scenarios", 2),
            BEAT_SECS,
        );
        let script = package_walk(script);
        let script = copy_walk(script);
        let script = file_row(
            script,
            "back to the hub to import",
            "Open Mod Item",
            ui_node_present("Hub Import Button"),
        )
        .click_named(
            "mod project: Import Package",
            "Hub Import Button",
            ui_node_present("Path Field"),
            BEAT_SECS,
        )
        .step("mod project: type the path")
        .on_enter(|world: &mut World| {
            let path = package_path();
            fill(world, "Path Field", &path);
        })
        .until(frames(3))
        .add()
        .click_named(
            "mod project: read the package",
            "File Save Button",
            ui_node_present("Confirm Discard Button"),
            BEAT_SECS,
        )
        .click_named(
            "mod project: overwrite the local mod",
            "Confirm Discard Button",
            reads("imported editor_red-run"),
            BEAT_SECS,
        );
        let script = open_world_stage::open_hub_row(
            script,
            "mod project: open the import",
            MOD_ROW,
            ui_node_present(FILE_MENU),
            PLAY_SECS,
        )
        .click_named(
            "mod project: fly the open scenario",
            "Play Button",
            player_ship_flies(),
            PLAY_SECS,
        )
        .step("mod project: leave the flight")
        .on_enter(press_key(KeyCode::F1))
        .until(ui_node_present(FILE_MENU))
        .diagnose(readout)
        .deadline(PLAY_SECS)
        .add()
        .step("mod project: let the flight key go")
        .on_enter(release_key(KeyCode::F1))
        .until(frames(2))
        .add();
        file_row(
            script,
            "leave for the menu",
            "Back To Main Menu Item",
            ui_node_present("Scenarios Button"),
        )
        .click_named(
            "mod project: open the Scenarios list",
            "Scenarios Button",
            ui_node_present("Scenario Row: red_run__saved_range"),
            PLAY_SECS,
        )
        .step("mod project: the saved mod is in the list")
        .on_enter(|_world: &mut World| {
            info!("mod project: the Scenarios list holds the mod's range");
        })
        .add()
    }

    /// S10: File > Package opens its window. A declared file gone from disk
    /// blocks it; undo clears that; a bump is one undo step; then it packages
    /// and Copy Path runs.
    fn package_walk(script: AutopilotPlugin<GameStates>) -> AutopilotPlugin<GameStates> {
        let script = script
            .step("mod project: put a file to import beside the profile, one loose in the mod")
            .on_enter(|_world: &mut World| {
                let outside = exports_dir()
                    .expect("the data dir resolves")
                    .with_file_name(IMPORTED);
                std::fs::create_dir_all(outside.parent().expect("a parent"))
                    .expect("the data dir writes");
                std::fs::write(&outside, b"notes").expect("the import source writes");
                let loose = mod_file_path(MOD_ID, LOOSE).expect("the mod's folder resolves");
                std::fs::write(&loose, b"loose").expect("the loose file writes");
            })
            .until(frames(2))
            .add()
            .click_named(
                "mod project: open the + menu to import",
                "Asset Add Button",
                ui_node_present("Asset Add Import File"),
                BEAT_SECS,
            )
            .click_named(
                "mod project: Import File",
                "Asset Add Import File",
                ui_node_present("Path Field"),
                BEAT_SECS,
            )
            .step("mod project: type the file's path")
            .on_enter(|world: &mut World| {
                let outside = exports_dir()
                    .expect("the data dir resolves")
                    .with_file_name(IMPORTED);
                fill(world, "Path Field", &outside.to_string_lossy());
            })
            .until(frames(3))
            .add()
            .click_named(
                "mod project: import it",
                "File Save Button",
                frames(3),
                BEAT_SECS,
            )
            .step("mod project: the file is in the mod's folder")
            .until(Arc::new(|_world: &World| {
                mod_file_path(MOD_ID, IMPORTED).is_some_and(|held| held.exists())
            }))
            .diagnose(|world: &World| {
                let field = world
                    .try_query::<(&Name, &TextFieldValue)>()
                    .and_then(|mut fields| {
                        fields
                            .iter(world)
                            .find(|(name, _)| name.as_str() == "Path Field")
                            .map(|(_, value)| value.0.clone())
                    });
                format!(
                    "{}, path field {field:?}, held at {:?}",
                    readout(world),
                    mod_file_path(MOD_ID, IMPORTED)
                )
            })
            .deadline(BEAT_SECS)
            .add()
            .step("mod project: take the imported file off disk")
            .on_enter(|world: &mut World| {
                info!("mod project: imported, status {:?}", status(world));
                let held = mod_file_path(MOD_ID, IMPORTED).expect("the mod's folder resolves");
                std::fs::remove_file(&held).unwrap_or_else(|error| panic!("{held:?}: {error}"));
            })
            .until(frames(2))
            .add();
        let script = file_row(
            script,
            "open the Package window",
            "Package Item",
            Arc::new(|world: &World| {
                ui_node_present("Package Modal")(world)
                    && reads("blocked: 1 error")(world)
                    && reads("resource walk_notes.txt is missing")(world)
                    && ui_node_present("Package File Row loose_notes.txt")(world)
            }),
        );
        let script = script
            .click_named(
                "mod project: Package refuses the missing file",
                "Package Button",
                Arc::new(|world: &World| {
                    reads("package refused: resource walk_notes.txt is missing")(world)
                        && !ui_node_present("Package Done Path")(world)
                }),
                BEAT_SECS,
            )
            .step("mod project: PASS the refusal")
            .on_enter(|world: &mut World| {
                info!(
                    "mod project: PASS package refused in the window, status {:?}",
                    status(world)
                );
            })
            .until(frames(2))
            .add();
        let script = shot(script, REFUSED_PACKAGE_SHOT);
        let script = chord(
            script,
            "Ctrl+Z takes the import back under the window",
            UNDO,
            Arc::new(|world: &World| reads("passes: 0 errors")(world)),
        )
        .step("mod project: note the depth before a bump")
        .on_enter(|world: &mut World| {
            let held = depth(world);
            world.insert_resource(DepthBeforeBump(held));
        })
        .until(frames(2))
        .add()
        .click_named(
            "mod project: bump the patch",
            "Package Bump Patch",
            Arc::new(|world: &World| {
                reads("red-run-0.1.1.zip")(world)
                    && world
                        .get_resource::<DepthBeforeBump>()
                        .is_some_and(|before| depth(world) == before.0 + 1)
            }),
            BEAT_SECS,
        );
        let script = chord(
            script,
            "Ctrl+Z takes the bump back in one step",
            UNDO,
            Arc::new(|world: &World| {
                reads("red-run-0.1.0.zip")(world)
                    && world
                        .get_resource::<DepthBeforeBump>()
                        .is_some_and(|before| depth(world) == before.0)
            }),
        )
        .step("mod project: PASS the bump")
        .on_enter(|_world: &mut World| {
            info!("mod project: PASS a bump is one undo step");
        })
        .until(frames(2))
        .add();
        let script = shot(script, PACKAGE_SHOT).click_named(
            "mod project: package",
            "Package Button",
            Arc::new(|world: &World| {
                ui_node_present("Package Done Path")(world) && reads("red-run-0.1.0.zip")(world)
            }),
            BEAT_SECS,
        );
        let script = script
            .step("mod project: the archive is on disk at the size the window says")
            .on_enter(|world: &mut World| {
                let path = package_path();
                let size = std::fs::metadata(&path)
                    .unwrap_or_else(|error| panic!("{path}: {error}"))
                    .len();
                assert!(
                    texts(world).iter().any(|text| text == &path),
                    "the window shows {path}"
                );
                info!("mod project: PASS packaged {path}, {size} bytes");
            })
            .until(frames(2))
            .add()
            .click_named(
                "mod project: Copy Path",
                "Package Copy Path",
                status_starts(&["Copied ", "copy refused:"]),
                BEAT_SECS,
            )
            .step("mod project: PASS Copy Path")
            .on_enter(|world: &mut World| {
                info!("mod project: PASS Copy Path said {:?}", status(world));
            })
            .until(frames(2))
            .add();
        let script = shot(script, DONE_SHOT);
        chord(
            script,
            "Esc closes the Package window",
            &[KeyCode::Escape],
            Arc::new(|world: &World| !ui_node_present("Package Modal")(world)),
        )
    }

    /// The history depth before the bump.
    #[derive(Resource)]
    struct DepthBeforeBump(usize);

    /// S5: mark Log rows with a click and a Shift+click, copy them, copy one
    /// line from its menu, then Ctrl+A and Ctrl+C over the list.
    fn copy_walk(script: AutopilotPlugin<GameStates>) -> AutopilotPlugin<GameStates> {
        let script = script
            .click_named(
                "mod project: open the Log",
                "Browser Tab Log",
                ui_node_present("Log Row 3"),
                BEAT_SECS,
            )
            .click_named(
                "mod project: mark a Log row",
                "Log Row 3",
                Arc::new(|world: &World| marked_log_rows(world) == ["Log Row 3"]),
                BEAT_SECS,
            )
            .step("mod project: hold Shift")
            .on_enter(press_key(KeyCode::ShiftLeft))
            .until(frames(2))
            .add()
            .click_named(
                "mod project: Shift+click marks the range",
                "Log Row 1",
                Arc::new(|world: &World| {
                    marked_log_rows(world) == ["Log Row 1", "Log Row 2", "Log Row 3"]
                }),
                BEAT_SECS,
            )
            .step("mod project: let Shift go")
            .on_enter(release_key(KeyCode::ShiftLeft))
            .until(frames(2))
            .add()
            .click_named(
                "mod project: Copy the marked rows",
                "Copy Rows Button",
                status_starts(&["Copied 3 lines", "copy refused:"]),
                BEAT_SECS,
            )
            .step("mod project: PASS copy of the marked rows")
            .on_enter(|world: &mut World| {
                info!("mod project: PASS Copy said {:?}", status(world));
            })
            .until(frames(2))
            .add();
        let script = shot(script, COPY_SHOT)
            .step("mod project: point at a Log row")
            .on_enter(|world: &mut World| {
                let at = ui_node_centre(world, "Log Row 2").expect("the row is on screen");
                move_cursor(at)(world);
            })
            .until(frames(3))
            .add()
            .step("mod project: press right on it")
            .on_enter(press_mouse(MouseButton::Right))
            .until(frames(2))
            .add()
            .step("mod project: release right on it")
            .on_enter(release_mouse(MouseButton::Right))
            .until(ui_node_present("Copy Line Row"))
            .diagnose(readout)
            .deadline(BEAT_SECS)
            .add()
            .click_named(
                "mod project: Copy line",
                "Copy Line Row",
                status_starts(&["Copied 1 line", "copy refused:"]),
                BEAT_SECS,
            )
            .step("mod project: point at the list for the keys")
            .on_enter(|world: &mut World| {
                let at = ui_node_centre(world, "Log Row 0").expect("the row is on screen");
                move_cursor(at)(world);
            })
            .until(frames(3))
            .add();
        let script = chord(
            script,
            "Ctrl+A marks every Log row",
            &[KeyCode::ControlLeft, KeyCode::KeyA],
            Arc::new(|world: &World| {
                let rows = log_rows(world);
                rows > 3 && marked_log_rows(world).len() == rows
            }),
        );
        chord(
            script,
            "Ctrl+C copies them",
            &[KeyCode::ControlLeft, KeyCode::KeyC],
            Arc::new(|world: &World| {
                let status = status(world);
                status.starts_with("copy refused:")
                    || status == format!("Copied {} lines", marked_log_rows(world).len())
            }),
        )
        .step("mod project: PASS Ctrl+A and Ctrl+C")
        .on_enter(|world: &mut World| {
            info!(
                "mod project: PASS Ctrl+C said {:?} with {} rows marked",
                status(world),
                marked_log_rows(world).len()
            );
        })
        .until(frames(2))
        .add()
    }

    /// Advance once a player ship flies.
    fn player_ship_flies() -> Arc<Predicate> {
        Arc::new(|world: &World| {
            world
                .try_query_filtered::<(), With<PlayerSpaceshipMarker>>()
                .is_some_and(|mut ships| ships.iter(world).next().is_some())
        })
    }
}
