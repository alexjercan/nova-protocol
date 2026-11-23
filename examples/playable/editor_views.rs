//! editor_views: each asset kind opens in its own view, with its own menus.
//!
//! Boots the shipped app on a profile and mod cache of its own and opens the Mod
//! Hub, so nothing here reaches your real mods.
//!
//! Try: name a mod and press Create. In the Assets tab double click a tile. A
//! scenario opens on Scene | Events, a ship opens inside its scenario on the
//! Ship view, a section on the Section view, and any other kind on its form.
//! The view switch shows only the open document's views, and the Add menu only
//! its verbs, then New Asset. Press + > UI Theme for a blank theme, then press a palette
//! swatch: drag in the square and the hue strip, type a hex, Esc to take it
//! back. Each picker session is one undo step.
//!
//! The panel reads the project's undo depth, unsaved mark and the status line.
//!
//! With `NOVA_AUTOPILOT=1` a smoke walk opens the scenario, a ship, a blank
//! section and a blank UI theme in turn, reads the view switch and the Add menu
//! in each, picks a theme colour, and undoes and redoes it. `NOVA_CAPTURE=1`
//! shoots each view, each Add menu, the blank section and the picker. Unset, no
//! walk and no deadline run.

use bevy::prelude::*;
use clap::Parser;
use nova_protocol::{nova_debug::diagnostics, prelude::*};

#[path = "shared/open_world_stage.rs"]
mod open_world_stage;

/// Command line for `editor_views`.
#[derive(Parser)]
#[command(name = "editor_views", version, about = "One view per asset kind", long_about = None)]
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

const INTRO: &str = "Name a mod and press Create. Double click a tile to open it in its view.";

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
        app.init_resource::<smoke::Walk>();
        app.add_systems(
            Startup,
            nova_protocol::nova_debug::harness::force_capture_resolution,
        );
    }
    app.add_plugins(smoke::script(cli.size.map_or(1920, |size| size.0 as u32)));
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

    use super::*;

    /// Seconds a cold boot gets on a software GPU.
    const BOOT_SECS: f32 = 300.0;
    /// Seconds a beat gets.
    const BEAT_SECS: f32 = 120.0;
    /// Seconds a frame capture gets.
    const SHOT_SECS: f32 = 120.0;
    const MOD_NAME: &str = "View Works";
    const FILE_MENU: &str = "File Menu Button";
    const EDIT_MENU: &str = "Edit Menu Button";
    const ADD_MENU: &str = "Add Menu Button";
    const SHIP_MENU: &str = "Ship Menu Button";
    const SCENARIO_TILE: &str = "Asset Tile Saved Range";
    const SHIP: &str = "picket_warden";
    const SECTION: &str = "view_works__new_hull";
    const SECTION_TILE: &str = "Asset Tile new_hull";
    const THEME_TILE: &str = "Asset Tile new_ui_theme";
    /// A colour the blank theme takes from the theme it inherits.
    const COLOUR: &str = "text";
    /// Every view tab, by the name its document gives it.
    const VIEWS: [&str; 5] = ["Scene", "Events", "Ship", "Section", "Form"];

    /// What the walk remembers between beats.
    #[derive(Resource, Default)]
    pub(super) struct Walk {
        /// The palette as the theme read before the pick.
        palette: String,
        /// The palette as the pick left it.
        picked: String,
        /// The undo depth before the pick.
        depth: usize,
    }

    /// The status line, for a stalled beat to print.
    fn readout(world: &World) -> String {
        world.get_resource::<EditorProbe>().map_or_else(
            || "no probe".to_string(),
            |probe| {
                format!(
                    "status {:?} depth {} next {:?} inside {:?} tab {:?} rows {:?}",
                    probe.status,
                    probe.history_depth,
                    probe.next_undo,
                    probe.inside,
                    probe.shell.rail_tab,
                    probe
                        .inspector
                        .iter()
                        .map(|(label, _)| label.as_str())
                        .collect::<Vec<_>>(),
                )
            },
        )
    }

    /// Advance once `check` holds of the probe.
    fn editor(check: impl Fn(&EditorProbe) -> bool + Send + Sync + 'static) -> Arc<Predicate> {
        Arc::new(move |world: &World| world.get_resource::<EditorProbe>().is_some_and(&check))
    }

    /// What the Inspector reads for `label`, or empty.
    fn reading(world: &World, label: &str) -> String {
        world
            .get_resource::<EditorProbe>()
            .and_then(|probe| {
                probe
                    .inspector
                    .iter()
                    .find(|(row, _)| row == label)
                    .map(|(_, value)| value.clone())
            })
            .unwrap_or_default()
    }

    /// The view tabs on screen, by the view they name.
    fn views_shown(world: &World) -> Vec<&'static str> {
        VIEWS
            .into_iter()
            .filter(|view| ui_node_present(format!("View Tab {view}"))(world))
            .collect()
    }

    /// Advance once the view switch offers exactly `wanted`.
    fn views_are(wanted: &'static [&'static str]) -> Arc<Predicate> {
        Arc::new(move |world: &World| views_shown(world) == wanted)
    }

    /// Shoot `name` at this run's width.
    fn shot(
        script: AutopilotPlugin<GameStates>,
        width: u32,
        name: &'static str,
    ) -> AutopilotPlugin<GameStates> {
        let file: &'static str = Box::leak(format!("views-{width}-{name}.png").into_boxed_str());
        script
            .step(format!("views: settle before {file}"))
            .until(frames(4))
            .add()
            .step(format!("views: shoot {file}"))
            .on_enter(move |world: &mut World| shoot(world, file))
            .until(shot_written(file))
            .deadline(SHOT_SECS)
            .add()
    }

    /// Open the Add menu and read it: every `shown` row stands and no `hidden`
    /// row does. Shoot it, and put it away.
    fn read_the_add_menu(
        script: AutopilotPlugin<GameStates>,
        width: u32,
        document: &'static str,
        shown: &'static [&'static str],
        hidden: &'static [&'static str],
    ) -> AutopilotPlugin<GameStates> {
        let script = script
            .click_named(
                &format!("views: open Add on the {document}"),
                ADD_MENU,
                ui_node_present(shown[0]),
                BEAT_SECS,
            )
            .step(format!(
                "views: the {document}'s Add menu holds its own verbs"
            ))
            .until(Arc::new(move |world: &World| {
                shown.iter().all(|row| ui_node_present(*row)(world))
                    && hidden.iter().all(|row| !ui_node_present(*row)(world))
            }))
            .diagnose(move |world: &World| {
                let standing: Vec<&str> = shown
                    .iter()
                    .chain(hidden)
                    .copied()
                    .filter(|row| ui_node_present(*row)(world))
                    .collect();
                format!("standing {standing:?}")
            })
            .deadline(BEAT_SECS)
            .add()
            .step(format!("views: read the {document}'s Add menu"))
            .on_enter(move |_world: &mut World| {
                info!("views: PASS the {document}'s Add menu shows {shown:?} and hides {hidden:?}");
            })
            .add();
        let script = shot(
            script,
            width,
            Box::leak(format!("{document}-add").into_boxed_str()),
        );
        script
            .step(format!("views: put the {document}'s Add menu away"))
            .on_enter(press_key(KeyCode::Escape))
            .until(Arc::new(move |world: &World| {
                !ui_node_present(shown[0])(world)
            }))
            .deadline(BEAT_SECS)
            .add()
            .step(format!("views: let Escape go on the {document}"))
            .on_enter(release_key(KeyCode::Escape))
            .until(frames(2))
            .add()
    }

    /// The New Asset rows no tree may hold: kinds the editor does not make.
    const NOT_MADE: &[&str] = &["Asset Add Lessons", "Asset Add Stations"];

    /// Whether the New Asset tree stands, with no row for a kind the editor
    /// does not make.
    fn only_made_kinds() -> Arc<Predicate> {
        Arc::new(|world: &World| {
            ui_node_present("Asset Add Scenarios")(world)
                && NOT_MADE.iter().all(|row| !ui_node_present(*row)(world))
        })
    }

    /// Open Add > Asset and read it: every row makes something, so no Lesson
    /// and no Station. Shoot it, and put the menu away.
    fn read_the_add_asset_tree(
        script: AutopilotPlugin<GameStates>,
        width: u32,
    ) -> AutopilotPlugin<GameStates> {
        let script = script
            .click_named(
                "views: open Add for its Asset group",
                ADD_MENU,
                ui_node_present("Add Menu Asset"),
                BEAT_SECS,
            )
            .step("views: point at Add > Asset")
            .on_enter(hover_named("Add Menu Asset"))
            .until(only_made_kinds())
            .diagnose(|world: &World| {
                let standing: Vec<&str> = NOT_MADE
                    .iter()
                    .copied()
                    .filter(|row| ui_node_present(*row)(world))
                    .collect();
                format!("standing {standing:?}")
            })
            .deadline(BEAT_SECS)
            .add()
            .step("views: read Add > Asset")
            .on_enter(|_world: &mut World| {
                info!("views: PASS Add > Asset offers no Lesson and no Station");
            })
            .add();
        let script = shot(script, width, "add-asset");
        script
            .step("views: put Add > Asset away")
            .on_enter(press_key(KeyCode::Escape))
            .until(Arc::new(|world: &World| {
                !ui_node_present("Add Menu Asset")(world)
            }))
            .deadline(BEAT_SECS)
            .add()
            .step("views: let Escape go on Add > Asset")
            .on_enter(release_key(KeyCode::Escape))
            .until(frames(2))
            .add()
    }

    /// Wait for a view switch of exactly `wanted`, say so, and shoot it.
    fn read_the_views(
        script: AutopilotPlugin<GameStates>,
        width: u32,
        document: &'static str,
        wanted: &'static [&'static str],
    ) -> AutopilotPlugin<GameStates> {
        let script = script
            .step(format!("views: the {document} offers {wanted:?}"))
            .until(views_are(wanted))
            .diagnose(|world: &World| format!("views {:?}; {}", views_shown(world), readout(world)))
            .deadline(BEAT_SECS)
            .add()
            .step(format!("views: read the {document}'s views"))
            .on_enter(move |_world: &mut World| {
                info!("views: PASS the {document} opens on {wanted:?}");
            })
            .add();
        shot(script, width, document)
    }

    /// Hub, a new mod, and the Assets tab.
    fn make_the_mod(script: AutopilotPlugin<GameStates>) -> AutopilotPlugin<GameStates> {
        script
            .step("views: the hub is up")
            .until(ui_node_present("Hub New Button"))
            .diagnose(readout)
            .deadline(BOOT_SECS)
            .add()
            .click_named(
                "views: open New",
                "Hub New Button",
                ui_node_present("Hub Create Button"),
                BEAT_SECS,
            )
            .step("views: name the mod")
            .on_enter(|world: &mut World| {
                let mut fields = world.query::<(&Name, &mut nova_ui::prelude::TextFieldValue)>();
                let (_, mut field) = fields
                    .iter_mut(world)
                    .find(|(named, _)| named.as_str() == "Hub Name Field")
                    .expect("the hub has a name field");
                field.0 = MOD_NAME.to_string();
            })
            .until(frames(3))
            .add()
            .click_named(
                "views: create it",
                "Hub Create Button",
                ui_node_present(FILE_MENU),
                BOOT_SECS,
            )
            .click_named(
                "views: show the mod's contents",
                "Browser Tab Assets",
                ui_node_present(SCENARIO_TILE),
                BEAT_SECS,
            )
    }

    pub(super) fn script(width: u32) -> AutopilotPlugin<GameStates> {
        let script = make_the_mod(AutopilotPlugin::<GameStates>::new())
            .double_click_named("views: open the scenario", SCENARIO_TILE, BEAT_SECS)
            .step("views: the scenario is the document")
            .until(editor(|probe| probe.inside.is_none()))
            .diagnose(readout)
            .deadline(BEAT_SECS)
            .add();
        let script = read_the_views(script, width, "scenario", &["Scene", "Events"]);
        let script = read_the_add_menu(
            script,
            width,
            "scenario",
            &[
                "Add Menu Ship",
                "Add Menu Objects",
                "Add Menu Lights",
                "Add Menu Markers",
                "Add Menu Asset",
            ],
            &["Add Menu Parts", "Add Socket Item", "Add Mesh Item"],
        );
        let script = read_the_add_asset_tree(script, width);
        let script = script
            .step("views: the Ship menu is the ship's")
            .until(Arc::new(|world: &World| !ui_node_present(SHIP_MENU)(world)))
            .deadline(BEAT_SECS)
            .add()
            .double_click_named(
                "views: open a ship",
                &format!("Asset Tile {SHIP}"),
                BEAT_SECS,
            )
            .step("views: inside the ship, in its scenario")
            .until(editor(|probe| probe.inside.as_deref() == Some(SHIP)))
            .diagnose(readout)
            .deadline(BEAT_SECS)
            .add();
        let script = read_the_views(script, width, "ship", &["Ship"]);
        let script = read_the_add_menu(
            script,
            width,
            "ship",
            &["Add Menu Parts", "Add Menu Asset"],
            &["Add Menu Objects", "Add Menu Ship", "Add Socket Item"],
        );
        let script = script
            .step("views: the Ship menu stands inside a ship")
            .until(ui_node_present(SHIP_MENU))
            .deadline(BEAT_SECS)
            .add()
            .click_named(
                "views: open the + menu",
                "Asset Add Button",
                ui_node_present("Asset Add Sections"),
                BEAT_SECS,
            )
            .step("views: + offers only kinds it makes")
            .until(only_made_kinds())
            .deadline(BEAT_SECS)
            .add()
            .step("views: read the + menu")
            .on_enter(|_world: &mut World| {
                info!("views: PASS + offers no Lesson and no Station");
            })
            .add();
        let script = shot(script, width, "plus");
        let script = script
            .click_named(
                "views: add a section",
                "Asset Add Sections",
                ui_node_present("Section Kind Hull"),
                BEAT_SECS,
            )
            .click_named(
                "views: pick Hull",
                "Section Kind Hull",
                pointer_released(),
                BEAT_SECS,
            )
            .click_named(
                "views: a blank hull",
                "Section Kind Create",
                ui_node_present(format!("Scene Row {SECTION}")),
                BEAT_SECS,
            )
            .step("views: the blank names what it still needs")
            .until(editor(|probe| probe.shell.errors > 0))
            .diagnose(readout)
            .deadline(BEAT_SECS)
            .add()
            .step("views: read the blank")
            .on_enter(|world: &mut World| {
                let probe = world.resource::<EditorProbe>();
                info!(
                    "views: PASS a blank hull opens with {} errors and {} undo step(s)",
                    probe.shell.errors, probe.history_depth
                );
            })
            .add();
        let script = shot(script, width, "blank");
        // Away and back by the tile, so the section's view is the tile's.
        let script = script
            .double_click_named("views: back to the scenario", SCENARIO_TILE, BEAT_SECS)
            .step("views: the scenario again")
            .until(views_are(&["Scene", "Events"]))
            .deadline(BEAT_SECS)
            .add()
            .double_click_named("views: open the section", SECTION_TILE, BEAT_SECS);
        let script = read_the_views(script, width, "section", &["Section"]);
        let script = read_the_add_menu(
            script,
            width,
            "section",
            &[
                "Add Socket Item",
                "Add Mesh Item",
                "Add Menu Collider",
                "Add Menu Asset",
            ],
            &["Add Menu Objects", "Add Menu Ship", "Add Menu Parts"],
        );
        let script = script
            .click_named(
                "views: open the + menu for a theme",
                "Asset Add Button",
                ui_node_present("Asset Add UI Themes"),
                BEAT_SECS,
            )
            .click_named(
                "views: a blank theme",
                "Asset Add UI Themes",
                ui_node_present(format!("Inspector Swatch {COLOUR}")),
                BEAT_SECS,
            )
            .double_click_named(
                "views: back to the scenario once more",
                SCENARIO_TILE,
                BEAT_SECS,
            )
            .step("views: the scenario once more")
            .until(views_are(&["Scene", "Events"]))
            .deadline(BEAT_SECS)
            .add()
            .double_click_named("views: open the theme", THEME_TILE, BEAT_SECS);
        let script = read_the_views(script, width, "theme", &["Form"]);
        pick_a_colour(script, width)
    }

    /// The theme's Add menu holds only New Asset; its palette is a grid, and a
    /// swatch opens a picker whose session is one undo step that Undo and Redo
    /// walk.
    fn pick_a_colour(
        script: AutopilotPlugin<GameStates>,
        width: u32,
    ) -> AutopilotPlugin<GameStates> {
        let script = script
            .step("views: a form has Add for a new asset, and no Ship menu")
            .until(Arc::new(|world: &World| {
                ui_node_present(ADD_MENU)(world) && !ui_node_present(SHIP_MENU)(world)
            }))
            .deadline(BEAT_SECS)
            .add()
            .step("views: stamp the palette")
            .until(Arc::new(|world: &World| {
                !reading(world, "Palette").is_empty()
            }))
            .diagnose(readout)
            .deadline(BEAT_SECS)
            .add()
            .step("views: hold the palette and the depth")
            .on_enter(|world: &mut World| {
                let palette = reading(world, "Palette");
                let depth = world.resource::<EditorProbe>().history_depth;
                assert!(
                    palette.contains(&format!("{COLOUR}=#")) && palette.contains('^'),
                    "a blank derived theme shows the colours it inherits: {palette}"
                );
                let mut walk = world.resource_mut::<Walk>();
                walk.palette = palette;
                walk.depth = depth;
            })
            .add()
            .click_named(
                "views: press the swatch",
                &format!("Inspector Swatch {COLOUR}"),
                ui_node_present("Colour Window Square"),
                BEAT_SECS,
            )
            .step("views: the picker has no alpha strip for a palette")
            .until(Arc::new(|world: &World| {
                ui_node_present("Colour Window Hue")(world)
                    && ui_node_present("Colour Window Hex")(world)
                    && !ui_node_present("Colour Window Alpha")(world)
            }))
            .deadline(BEAT_SECS)
            .add();
        let script = shot(script, width, "picker");
        let script = script
            .click_named(
                "views: pick in the square",
                "Colour Window Square",
                Arc::new(|world: &World| {
                    reading(world, "Palette") != world.resource::<Walk>().palette
                }),
                BEAT_SECS,
            )
            .click_named(
                "views: close the picker",
                "Colour Window Close",
                Arc::new(|world: &World| !ui_node_present("Colour Window")(world)),
                BEAT_SECS,
            )
            .step("views: the pick is one step")
            .until(editor(|probe| {
                probe.next_undo.as_deref() == Some(&format!("Edit Colour {COLOUR}") as &str)
            }))
            .diagnose(readout)
            .deadline(BEAT_SECS)
            .add()
            .step("views: read the pick")
            .on_enter(|world: &mut World| {
                let picked = reading(world, "Palette");
                let depth = world.resource::<EditorProbe>().history_depth;
                let walk = world.resource::<Walk>();
                assert_eq!(depth, walk.depth + 1, "a picker session is one step");
                let was = walk.palette.clone();
                info!("views: PASS the pick is one step: {was} -> {picked}");
                world.resource_mut::<Walk>().picked = picked;
            })
            .add();
        let script = shot(script, width, "picked");
        let script = script.click_named(
            "views: open Edit for Undo",
            EDIT_MENU,
            ui_node_present("Undo Item"),
            BEAT_SECS,
        );
        shot(script, width, "edit-menu")
            .step("views: the Edit menu ends at its last shown row")
            .until(Arc::new(|world: &World| {
                let (Some(menu), Some(redo)) = (
                    ui_node_rect(world, "Edit Menu"),
                    ui_node_rect(world, "Redo Item"),
                ) else {
                    return false;
                };
                // Padding, border and the row's margin: under 10 px.
                menu.max.y - redo.max.y < 10.0
            }))
            .diagnose(|world: &World| {
                format!(
                    "menu {:?} redo {:?}",
                    ui_node_rect(world, "Edit Menu"),
                    ui_node_rect(world, "Redo Item")
                )
            })
            .deadline(BEAT_SECS)
            .add()
            .click_named(
                "views: Undo",
                "Undo Item",
                Arc::new(|world: &World| {
                    reading(world, "Palette") == world.resource::<Walk>().palette
                }),
                BEAT_SECS,
            )
            .click_named(
                "views: open Edit for Redo",
                EDIT_MENU,
                ui_node_present("Redo Item"),
                BEAT_SECS,
            )
            .click_named(
                "views: Redo",
                "Redo Item",
                Arc::new(|world: &World| {
                    reading(world, "Palette") == world.resource::<Walk>().picked
                }),
                BEAT_SECS,
            )
            .step("views: undo and redo walked the colour")
            .on_enter(|_world: &mut World| {
                info!("views: PASS Undo returned the colour and Redo restored it");
            })
            .add()
    }
}
