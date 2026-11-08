//! editor_section_builder: one custom section, built, placed, packaged and flown.
//!
//! Boots the shipped app on a profile and mod cache of its own and opens the Mod
//! Hub, so nothing here reaches your real mods.
//!
//! Try: name a mod and press Create. In the Assets tab press + > Sections and
//! pick a kind. The new section is blank, and Problems names the fields it
//! still needs. The Viewport shows the section alone, the Hierarchy its parts
//! and sockets, and the Inspector their fields. Add > Socket adds a socket.
//! Click it, press W and drag a handle: each move is one undo step. Del removes
//! it. Add > Mesh... imports a .glb. Then open a scenario, enter a ship and arm
//! the section from the gallery's Mod category. Save, Package and Play refuse
//! while Problems shows an Error.
//!
//! The panel reads the project's undo depth, unsaved mark and the status line.
//!
//! With `NOVA_AUTOPILOT=1` a smoke walk does the whole path through real buttons:
//! new mod, two scenarios, a blank section filled in, a socket added and moved
//! and a real glb imported,
//! placed on a ship, saved, reopened through the hub, packaged and flown, with the
//! section read off the flown ship. The section's kind is picked in its modal by
//! the arrows and Enter. `NOVA_CAPTURE=1` shoots the bench, the + menu, the kind
//! modal, the form, the imported mesh, the placed part and the flight. Unset, no walk and no
//! deadline run.

use bevy::prelude::*;
use clap::Parser;
use nova_protocol::{nova_debug::diagnostics, prelude::*};

#[path = "shared/open_world_stage.rs"]
mod open_world_stage;

/// Command line for `editor_section_builder`.
#[derive(Parser)]
#[command(name = "editor_section_builder", version, about = "One custom section, end to end", long_about = None)]
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

const INTRO: &str = "Name a mod and press Create. Assets > + > Sections adds a blank section.";

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
        app.init_resource::<smoke::Walk>();
        app.init_resource::<smoke::SeekClock>();
        app.add_systems(
            Startup,
            nova_protocol::nova_debug::harness::force_capture_resolution,
        );
        app.add_systems(Update, smoke::seek_a_face);
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
    use std::{path::PathBuf, sync::Arc};

    use bevy::input::keyboard::Key;
    use nova_assets::{mod_cache::prelude::exports_dir, storage::CONFIG_ROOT_ENV};
    use nova_protocol::nova_debug::{
        harness::{
            editor_field_focused, editor_filter_focused, editor_gallery_open,
            editor_gallery_selected, editor_placement_solved, pose_camera, scenario_camera_present,
            scenario_is_built, shoot, AutopilotPlugin, Predicate,
        },
        prelude::type_text,
    };
    use nova_ui::prelude::TextFieldValue;

    use super::*;

    /// Seconds a cold boot gets on a software GPU.
    const BOOT_SECS: f32 = 300.0;
    /// Seconds a beat gets.
    const BEAT_SECS: f32 = 120.0;
    /// Seconds a frame capture gets.
    const SHOT_SECS: f32 = 120.0;
    /// Seconds the flight scene and a content restart get.
    const PLAY_SECS: f32 = 300.0;
    const MOD_NAME: &str = "Spar Works";
    const SECTION: &str = "spar_works__new_hull";
    const SOCKET: &str = "socket_1";
    /// What the blank needs before Save takes it: the fields whose least value
    /// the lint refuses, and a health so the part survives its flight.
    const FILLED: &[(&str, &str)] = &[
        ("Health", "100"),
        ("Draw Weight", "1"),
        ("Credits", "40"),
        ("Strip Yield", "0.5"),
    ];
    const ADD_MENU: &str = "Add Menu Button";
    const SHIP: &str = "picket_warden";
    const FILE_MENU: &str = "File Menu Button";
    const SAVE: &[KeyCode] = &[KeyCode::ControlLeft, KeyCode::KeyS];
    const MODEL: &str = "assets/base/gltf/dock_flush.glb";
    const MODEL_REF: &str = "self://walk_hull.glb#Scene0";
    /// Long enough to wrap in the narrow Inspector.
    const DESCRIPTION: &str = "A long spar for the outer ring, with one socket set off its face \
                               so the next hull can stand clear of the docking collar behind it.";
    const BENCH_SHOT: &str = "section-bench.png";
    const GALLERY_SHOT: &str = "section-gallery.png";
    const FORM_SHOT: &str = "section-form.png";
    const BLANK_SHOT: &str = "section-blank.png";
    const PLUS_SHOT: &str = "section-plus.png";
    const KIND_SHOT: &str = "section-kind.png";
    const MESH_SHOT: &str = "section-mesh.png";
    const PLACED_SHOT: &str = "section-placed.png";
    const FLOWN_SHOT: &str = "section-flown.png";

    /// What the walk remembers between beats.
    #[derive(Resource, Default)]
    pub(super) struct Walk {
        /// The undo depth and socket rows before the drag.
        before: Option<(usize, String)>,
        /// Whether [`seek_a_face`] is walking the pointer over the ship.
        seeking: bool,
    }

    /// The status line, for a stalled beat to print.
    fn readout(world: &World) -> String {
        world.get_resource::<EditorProbe>().map_or_else(
            || "no probe".to_string(),
            |probe| {
                format!(
                    "status {:?} dirty {} depth {} selected {:?} rows {:?} nodes {:?} ship {:?}",
                    probe.status,
                    probe.dirty,
                    probe.history_depth,
                    probe.selected_node,
                    probe
                        .inspector
                        .iter()
                        .map(|(label, _)| label.as_str())
                        .collect::<Vec<_>>(),
                    probe.context_nodes,
                    probe
                        .ship
                        .iter()
                        .map(|section| section.prototype.clone())
                        .collect::<Vec<_>>()
                )
            },
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

    /// Advance once `check` holds of the probe.
    fn editor(check: impl Fn(&EditorProbe) -> bool + Send + Sync + 'static) -> Arc<Predicate> {
        Arc::new(move |world: &World| world.get_resource::<EditorProbe>().is_some_and(&check))
    }

    /// Advance once a save reports itself and the unsaved mark is gone.
    fn saved_and_clean() -> Arc<Predicate> {
        editor(|probe| !probe.dirty && probe.status.starts_with("saved"))
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

    /// The real model, copied beside this run's profile.
    fn scratch_model() -> PathBuf {
        let profile = std::env::var_os(CONFIG_ROOT_ENV).expect("the profile is isolated");
        let target = PathBuf::from(profile).join("walk_hull.glb");
        std::fs::create_dir_all(target.parent().expect("the profile is a folder"))
            .expect("the profile folder writes");
        std::fs::copy(MODEL, &target).expect("the model copies");
        target
    }

    /// The zip Package wrote, by the only name it can have here.
    fn package_path() -> String {
        exports_dir()
            .expect("the data dir resolves")
            .join("spar-works-0.1.0.zip")
            .to_string_lossy()
            .into_owned()
    }

    /// Where a world point falls in the window.
    fn aim_at(world: &mut World, point: Vec3) -> Option<Vec2> {
        // The active one: an asset's thumbnail camera is a Camera3d too, and
        // it stands idle at a 112 px viewport between renders.
        let eye = world
            .query_filtered::<(Entity, &Camera), With<Camera3d>>()
            .iter(world)
            .find(|(_, camera)| camera.is_active)
            .map(|(eye, _)| eye)?;
        let camera = world.get::<Camera>(eye)?.clone();
        let pose = *world.get::<GlobalTransform>(eye)?;
        camera
            .world_to_viewport(&pose, point)
            .ok()
            .filter(|aim: &Vec2| aim.is_finite())
    }

    /// Where the named gizmo handle stands in the window.
    fn handle_aim(world: &mut World, prefix: &str) -> Option<Vec2> {
        let at = world
            .query::<(&Name, &GlobalTransform)>()
            .iter(world)
            .find(|(name, _)| name.as_str().starts_with(prefix))
            .map(|(_, pose)| pose.translation())?;
        aim_at(world, at)
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

    /// While the walk seeks, step the pointer over the faces of the visible
    /// sections, so the beat waits on the editor solving one and not on a guess
    /// at which face of a template ship is free.
    pub(super) fn seek_a_face(world: &mut World) {
        if !world.resource::<Walk>().seeking {
            return;
        }
        let tick = {
            let mut frames = world.resource_mut::<SeekClock>();
            frames.0 += 1;
            frames.0
        };
        if tick % 8 != 0 {
            return;
        }
        let mut faces = Vec::new();
        let mut sections =
            world.query_filtered::<(&GlobalTransform, &InheritedVisibility), With<SectionMarker>>();
        for (pose, visible) in sections.iter(world) {
            if !visible.get() {
                continue;
            }
            let (_, turn, centre) = pose.to_scale_rotation_translation();
            for axis in [
                Vec3::X,
                Vec3::NEG_X,
                Vec3::Y,
                Vec3::NEG_Y,
                Vec3::Z,
                Vec3::NEG_Z,
            ] {
                faces.push(centre + turn * axis * 0.5);
            }
        }
        faces.sort_by(|a, b| a.to_array().partial_cmp(&b.to_array()).expect("finite"));
        if faces.is_empty() {
            return;
        }
        let face = faces[(tick / 8) as usize % faces.len()];
        if let Some(at) = aim_at(world, face) {
            move_cursor(at)(world);
        }
    }

    /// The frames [`seek_a_face`] has seen.
    #[derive(Resource, Default)]
    pub(super) struct SeekClock(u32);

    /// Whether the row or tile `name` carries the highlight.
    fn marked(name: &'static str) -> Arc<Predicate> {
        Arc::new(move |world: &World| {
            world
                .try_query_filtered::<&Name, With<nova_ui::prelude::Selected>>()
                .is_some_and(|mut rows| rows.iter(world).any(|row| row.as_str() == name))
        })
    }

    /// Shoot `shot`.
    fn shot(
        script: AutopilotPlugin<GameStates>,
        shot: &'static str,
    ) -> AutopilotPlugin<GameStates> {
        script
            .step(format!("section: shoot {shot}"))
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
                &format!("section: {label}: open the menu"),
                FILE_MENU,
                ui_node_present(row),
                BEAT_SECS,
            )
            .click_named(&format!("section: {label}"), row, landed, BEAT_SECS)
    }

    /// Ctrl+S, held until the save says so, and let go.
    fn save(script: AutopilotPlugin<GameStates>, label: &str) -> AutopilotPlugin<GameStates> {
        script
            .step(format!("section: {label}"))
            .on_enter(|world: &mut World| {
                for key in SAVE {
                    press_key(*key)(world);
                }
            })
            .until(saved_and_clean())
            .diagnose(readout)
            .deadline(BEAT_SECS)
            .add()
            .step(format!("section: {label}: let go"))
            .on_enter(|world: &mut World| {
                for key in SAVE {
                    release_key(*key)(world);
                }
            })
            .until(frames(2))
            .add()
    }

    /// The mod, two scenarios and the section on its bench.
    fn build_the_mod(script: AutopilotPlugin<GameStates>) -> AutopilotPlugin<GameStates> {
        let script = script
            .step("section: the hub is up")
            .until(ui_node_present("Hub New Button"))
            .diagnose(readout)
            .deadline(BOOT_SECS)
            .add()
            .click_named(
                "section: open New",
                "Hub New Button",
                ui_node_present("Hub Create Button"),
                BEAT_SECS,
            )
            .step("section: name the mod")
            .on_enter(|world: &mut World| fill(world, "Hub Name Field", MOD_NAME))
            .until(reads("saves as editor_spar-works"))
            .diagnose(readout)
            .deadline(BEAT_SECS)
            .add()
            .click_named(
                "section: create it",
                "Hub Create Button",
                ui_node_present(FILE_MENU),
                PLAY_SECS,
            )
            .click_named(
                "section: show the mod's contents",
                "Browser Tab Assets",
                reads("Scenarios 1"),
                BEAT_SECS,
            )
            .click_named(
                "section: open the Add menu",
                "Asset Add Button",
                ui_node_present("Asset Add Scenarios"),
                BEAT_SECS,
            )
            .click_named(
                "section: add a scenario",
                "Asset Add Scenarios",
                reads("Scenarios 2"),
                BEAT_SECS,
            )
            .click_named(
                "section: open the Add menu again",
                "Asset Add Button",
                ui_node_present("Asset Add Sections"),
                BEAT_SECS,
            );
        let script = shot(script, PLUS_SHOT)
            .click_named(
                "section: add a section",
                "Asset Add Sections",
                ui_node_present("Section Kind Hull"),
                BEAT_SECS,
            )
            .step("section: the modal opens on Hull")
            .until(marked("Section Kind Hull"))
            .deadline(BEAT_SECS)
            .add()
            .step("section: Right marks Thruster")
            .on_enter(press_key(KeyCode::ArrowRight))
            .until(marked("Section Kind Thruster"))
            .deadline(BEAT_SECS)
            .add()
            .step("section: let Right go")
            .on_enter(release_key(KeyCode::ArrowRight))
            .until(frames(2))
            .add();
        let script = shot(script, KIND_SHOT)
            .step("section: Left marks Hull again")
            .on_enter(press_key(KeyCode::ArrowLeft))
            .until(marked("Section Kind Hull"))
            .deadline(BEAT_SECS)
            .add()
            .step("section: let Left go")
            .on_enter(release_key(KeyCode::ArrowLeft))
            .until(frames(2))
            .add()
            .step("section: Enter makes a hull")
            .on_enter(press_key(KeyCode::Enter))
            .until(ui_node_present(format!("Scene Row {SECTION}")))
            .diagnose(readout)
            .deadline(BEAT_SECS)
            .add()
            .step("section: let Enter go")
            .on_enter(release_key(KeyCode::Enter))
            .until(frames(2))
            .add()
            .step("section: the blank names what it still needs")
            .until(editor(|probe| probe.shell.errors > 0))
            .diagnose(readout)
            .deadline(BEAT_SECS)
            .add()
            .step("section: read the blank")
            .on_enter(|world: &mut World| {
                let probe = world.resource::<EditorProbe>();
                info!(
                    "section: PASS a blank hull opens with {} errors, sockets {:?}",
                    probe.shell.errors, probe.context_nodes
                );
            })
            .add();
        let script = shot(script, BLANK_SHOT);
        script
            .click_named(
                "section: open the Add menu",
                ADD_MENU,
                ui_node_present("Add Socket Item"),
                BEAT_SECS,
            )
            .click_named(
                "section: add a socket",
                "Add Socket Item",
                ui_node_present(format!("Scene Row {SOCKET}")),
                BEAT_SECS,
            )
    }

    /// Move a socket with the gizmo and read that it was one step.
    fn move_a_socket(script: AutopilotPlugin<GameStates>) -> AutopilotPlugin<GameStates> {
        let script = script
            .click_named(
                "section: select a socket",
                &format!("Scene Row {SOCKET}"),
                editor(|probe| probe.selected_node.as_deref() == Some(SOCKET)),
                BEAT_SECS,
            )
            .step("section: take the move tool")
            .on_enter(press_key(KeyCode::KeyW))
            .until(editor(|probe| probe.gizmo_node.as_deref() == Some(SOCKET)))
            .diagnose(readout)
            .deadline(BEAT_SECS)
            .add()
            .step("section: let W go")
            .on_enter(release_key(KeyCode::KeyW))
            .until(frames(4))
            .add()
            .step("section: point at the up handle")
            .on_enter(|world: &mut World| {
                let probe = world.resource::<EditorProbe>().clone();
                world.resource_mut::<Walk>().before =
                    Some((probe.history_depth, format!("{:?}", probe.inspector)));
                let at = handle_aim(world, "Gizmo Move Grab Y").expect("the handle is on screen");
                move_cursor(at)(world);
            })
            .until(pointer_on_a_handle())
            .diagnose(readout)
            .deadline(BEAT_SECS)
            .add();
        let script = shot(script, BENCH_SHOT);
        script
            .step("section: grab the handle")
            .on_enter(press_mouse(MouseButton::Left))
            .until(frames(4))
            .add()
            .step("section: drag it up")
            .on_enter(|world: &mut World| {
                let at = handle_aim(world, "Gizmo Move Grab Y").expect("the handle is on screen");
                move_cursor(at - Vec2::new(0.0, 90.0))(world);
            })
            .until(frames(6))
            .add()
            .step("section: let the handle go")
            .on_enter(release_mouse(MouseButton::Left))
            .until(Arc::new(|world: &World| {
                let probe = world.resource::<EditorProbe>();
                let (depth, rows) = world
                    .resource::<Walk>()
                    .before
                    .clone()
                    .expect("the walk read the socket");
                probe.history_depth == depth + 1 && format!("{:?}", probe.inspector) != rows
            }))
            .diagnose(readout)
            .deadline(BEAT_SECS)
            .add()
            .step("section: the socket moved on the grid, as one step")
            .on_enter(|world: &mut World| {
                let probe = world.resource::<EditorProbe>();
                let (_, position) = probe
                    .inspector
                    .iter()
                    .find(|(label, _)| label == "Position")
                    .expect("a socket has a Position row");
                for meters in position.split(", ") {
                    let meters: f32 = meters.parse().expect("the row reads as meters");
                    assert_eq!(meters % 5.0, 0.0, "{position} is off the 5 m grid");
                }
                info!(
                    "section: socket {SOCKET} moved to {position}, undo {:?}",
                    probe.next_undo
                );
            })
            .add()
    }

    /// Type `value` into the Inspector row `label`, as a builder would.
    fn type_into(
        script: AutopilotPlugin<GameStates>,
        label: &'static str,
        value: &'static str,
    ) -> AutopilotPlugin<GameStates> {
        script
            .click_named(
                &format!("section: reach for {label}"),
                &format!("Inspector Field {label}"),
                editor_field_focused(),
                BEAT_SECS,
            )
            .step(format!("section: type {label}"))
            .on_enter(move |world: &mut World| {
                press_edit_key(Key::End)(world);
                for _ in 0..12 {
                    press_edit_key(Key::Backspace)(world);
                }
                type_text(value)(world);
                press_edit_key(Key::Enter)(world);
            })
            .until(editor(move |probe| {
                probe.inspector.iter().any(|(row, held)| {
                    row == label && held.parse::<f32>().ok() == value.parse::<f32>().ok()
                })
            }))
            .diagnose(readout)
            .deadline(BEAT_SECS)
            .add()
    }

    /// Type a long Description and the fields the blank needs, then import a
    /// real glb through Add > Mesh.
    fn fill_the_form(script: AutopilotPlugin<GameStates>) -> AutopilotPlugin<GameStates> {
        let script = script
            .step("section: put the tool down")
            .on_enter(press_key(KeyCode::KeyQ))
            .until(frames(2))
            .add()
            .step("section: let Q go")
            .on_enter(release_key(KeyCode::KeyQ))
            .until(frames(2))
            .add()
            .click_named(
                "section: select the section",
                &format!("Scene Row {SECTION}"),
                frames(4),
                BEAT_SECS,
            )
            .step("section: the Inspector shows the section")
            .until(ui_node_present("Inspector Field Description"))
            .diagnose(readout)
            .deadline(BEAT_SECS)
            .add()
            .click_named(
                "section: reach for the description",
                "Inspector Field Description",
                editor_field_focused(),
                BEAT_SECS,
            )
            .step("section: type the description")
            .on_enter(|world: &mut World| {
                press_edit_key(Key::End)(world);
                for _ in 0..40 {
                    press_edit_key(Key::Backspace)(world);
                }
                type_text(DESCRIPTION)(world);
                press_edit_key(Key::Enter)(world);
            })
            .until(editor(|probe| {
                probe
                    .inspector
                    .iter()
                    .any(|(label, value)| label == "Description" && value == DESCRIPTION)
            }))
            .diagnose(readout)
            .deadline(BEAT_SECS)
            .add();
        let mut script = script;
        for &(label, value) in FILLED {
            script = type_into(script, label, value);
        }
        let script = script
            .step("section: the filled section has no error left")
            .until(editor(|probe| probe.shell.errors == 0))
            .diagnose(readout)
            .deadline(BEAT_SECS)
            .add();
        let script = shot(script, FORM_SHOT)
            .click_named(
                "section: open the Add menu for a mesh",
                ADD_MENU,
                ui_node_present("Add Mesh Item"),
                BEAT_SECS,
            )
            .click_named(
                "section: Add > Mesh...",
                "Add Mesh Item",
                ui_node_present("Path Field"),
                BEAT_SECS,
            )
            .step("section: type the path")
            .on_enter(|world: &mut World| {
                let path = scratch_model();
                fill(world, "Path Field", &path.to_string_lossy());
            })
            .until(frames(3))
            .add()
            .click_named(
                "section: read the model",
                "File Save Button",
                Arc::new(|world: &World| !ui_node_present("File Window")(world)),
                BEAT_SECS,
            )
            .click_named(
                "section: select the mesh",
                "Scene Row mesh",
                editor(|probe| {
                    probe
                        .inspector
                        .iter()
                        .any(|(_, value)| value.contains(MODEL_REF))
                }),
                BEAT_SECS,
            );
        shot(script, MESH_SHOT)
    }

    /// Open the first scenario, enter its ship and place the section.
    fn place_it(script: AutopilotPlugin<GameStates>) -> AutopilotPlugin<GameStates> {
        let script = save(script, "save the section");
        let script = open_world_stage::open_hub_row(
            script,
            "section: open the range",
            "Asset Tile Saved Range",
            ui_node_present(format!("Scene Row {SHIP}")),
            PLAY_SECS,
        )
        .double_click_named(
            "section: enter the ship",
            &format!("Scene Row {SHIP}"),
            BEAT_SECS,
        )
        .step("section: inside the ship")
        .until(editor(|probe| probe.inside.is_some()))
        .diagnose(readout)
        .deadline(BEAT_SECS)
        .add()
        // The Ship editor has no stage tools: its Ship menu opens the parts.
        .click_named(
            "section: open the Ship menu",
            "Ship Menu Button",
            ui_node_present("Parts Item"),
            BEAT_SECS,
        )
        .click_named(
            "section: open the gallery",
            "Parts Item",
            editor_gallery_open(),
            BEAT_SECS,
        )
        .step(format!("section: shoot {GALLERY_SHOT}"))
        .on_enter(move |world: &mut World| shoot(world, GALLERY_SHOT))
        .until(shot_written(GALLERY_SHOT))
        .deadline(SHOT_SECS)
        .add()
        .step("section: put the caret in the gallery filter")
        .on_enter(press_key(KeyCode::Slash))
        .until(editor_filter_focused())
        .deadline(BEAT_SECS)
        .add()
        .step("section: let the slash go")
        .on_enter(release_key(KeyCode::Slash))
        .add()
        .step("section: narrow the gallery to the mod's part")
        .on_enter(type_text(SECTION))
        .until(editor_gallery_selected(SECTION))
        .diagnose(readout)
        .deadline(BEAT_SECS)
        .add()
        .step("section: Enter to focus")
        .on_enter(press_key(KeyCode::Enter))
        .until(ui_node_present("Gallery Focus Card"))
        .deadline(BEAT_SECS)
        .add()
        .step("section: let Enter go")
        .on_enter(release_key(KeyCode::Enter))
        .add()
        .step("section: Enter to arm it")
        .on_enter(press_key(KeyCode::Enter))
        .until(nova_protocol::nova_debug::harness::editor_part_armed())
        .deadline(BEAT_SECS)
        .add()
        .step("section: let Enter go again")
        .on_enter(release_key(KeyCode::Enter))
        .add()
        .step("section: find a free face")
        .on_enter(|world: &mut World| world.resource_mut::<Walk>().seeking = true)
        .until(editor_placement_solved())
        .diagnose(readout)
        .deadline(BEAT_SECS)
        .add()
        .step("section: hold the aim")
        .on_enter(|world: &mut World| world.resource_mut::<Walk>().seeking = false)
        .until(frames(2))
        .add()
        .step("section: press on the face")
        .on_enter(press_mouse(MouseButton::Left))
        .until(frames(3))
        .add()
        .step("section: let the press go")
        .on_enter(release_mouse(MouseButton::Left))
        .until(editor(|probe| {
            probe.ship.iter().any(|part| part.prototype == SECTION)
        }))
        .diagnose(readout)
        .deadline(BEAT_SECS)
        .add();
        let script = shot(script, PLACED_SHOT)
            .step("section: put the part down")
            .on_enter(press_key(KeyCode::Escape))
            .until(frames(3))
            .add()
            .step("section: let Escape go")
            .on_enter(release_key(KeyCode::Escape))
            .until(frames(2))
            .add();
        save(script, "save the ship")
    }

    /// Advance once the flown world holds the section by id.
    fn section_flies() -> Arc<Predicate> {
        Arc::new(|world: &World| {
            world
                .try_query::<&SectionBuildConfig>()
                .is_some_and(|mut built| {
                    built.iter(world).any(|config| config.0.base.id == SECTION)
                })
        })
    }

    pub(super) fn script() -> AutopilotPlugin<GameStates> {
        let script = build_the_mod(AutopilotPlugin::<GameStates>::new());
        let script = move_a_socket(script);
        let script = fill_the_form(script);
        let script = place_it(script);
        let script = file_row(
            script,
            "back to the hub",
            "Open Mod Item",
            ui_node_present("Hub Row editor_spar-works"),
        );
        let script = open_world_stage::open_hub_row(
            script,
            "section: reopen it",
            "Hub Row editor_spar-works",
            ui_node_present(FILE_MENU),
            PLAY_SECS,
        )
        .click_named(
            "section: the reopened mod holds the section",
            "Browser Tab Assets",
            reads("Sections 1"),
            BEAT_SECS,
        );
        let script = file_row(
            script,
            "package",
            "Package Item",
            reads("spar-works-0.1.0.zip"),
        )
        .step("section: the archive is on disk")
        .on_enter(|_world: &mut World| {
            let path = package_path();
            let size = std::fs::metadata(&path)
                .unwrap_or_else(|error| panic!("{path}: {error}"))
                .len();
            info!("section: packaged {path}, {size} bytes");
        })
        .add()
        .click_named(
            "section: close the result",
            "File Cancel Button",
            Arc::new(|world: &World| !ui_node_present("File Window")(world)),
            BEAT_SECS,
        )
        .click_named(
            "section: fly the open scenario",
            "Play Button",
            section_flies(),
            PLAY_SECS,
        )
        .step("section: the scene is built")
        .until(Arc::new(|world: &World| {
            scenario_camera_present()(world)
                && scenario_is_built()(world)
                && !ui_node_present("Scenario Loading Screen")(world)
        }))
        .diagnose(readout)
        .deadline(PLAY_SECS)
        .add()
        .step("section: the flown ship carries the section")
        .on_enter(|world: &mut World| {
            let mut built = world.query::<(&SectionBuildConfig, &GlobalTransform)>();
            let flown: Vec<_> = built
                .iter(world)
                .filter(|(config, _)| config.0.base.id == SECTION)
                .map(|(config, pose)| {
                    let socket = config
                        .0
                        .base
                        .link_points
                        .iter()
                        .find(|point| point.id == SOCKET)
                        .map(|point| point.position);
                    (pose.translation(), socket)
                })
                .collect();
            info!("section: PASS flown {SECTION}: {flown:?}");
            let (at, _) = *flown.first().expect("the section flies");
            pose_camera(
                world,
                Meters3::from_engine(at + Vec3::new(5.0, 4.0, 7.0)),
                Meters3::from_engine(at),
            );
        })
        .until(frames(30))
        .add();
        shot(script, FLOWN_SHOT)
            .step("section: leave the flight")
            .on_enter(press_key(KeyCode::F1))
            .until(ui_node_present(FILE_MENU))
            .diagnose(readout)
            .deadline(PLAY_SECS)
            .add()
            .step("section: let the flight key go")
            .on_enter(release_key(KeyCode::F1))
            .until(frames(2))
            .add()
    }
}
