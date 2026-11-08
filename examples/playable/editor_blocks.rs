//! editor_blocks: the script as puzzle blocks, on a copy of the tutorial.
//!
//! Boots the shipped app on a profile folder of its own, with a mod holding a
//! copy of the tutorial, and stops at the Mod Hub with that mod listed. Press
//! Open, then the Events tab.
//!
//! Try: drag a block off the palette into a handler. Every slot it fits lights,
//! and the nearest opens a gap. Drag a stack by its top block and the blocks
//! below ride along. Drop a block on the palette to bin it. Click a value to
//! type it, drag a number to scrub it. `Ctrl+Z` takes one drag back.
//! Middle or right drag pans, `Ctrl+wheel` zooms, `F` frames the selection.
//!
//! With `NOVA_AUTOPILOT=1` a smoke walk does these gestures with the harness
//! pointer and asserts the document after each. `NOVA_CAPTURE=1` shoots frames.
//! Unset, no walk and no deadline run.

use bevy::{prelude::*, window::PrimaryWindow};
use clap::Parser;
use nova_protocol::{nova_debug::diagnostics, prelude::*};

#[path = "shared/open_world_stage.rs"]
mod open_world_stage;
#[path = "../screenshots/shared/ui_walk.rs"]
mod ui_walk;

#[derive(Parser)]
#[command(name = "editor_blocks")]
#[command(version = "1.0.0")]
#[command(about = "The script as puzzle blocks, on a copy of the tutorial", long_about = None)]
struct Cli {
    /// The walk's window width, in logical px.
    #[arg(long, default_value_t = 1920.0)]
    width: f32,
    /// The walk's window height, in logical px.
    #[arg(long, default_value_t = 1080.0)]
    height: f32,
}

const INTRO: &str = "Open the mod, take the Events tab, drag blocks. Ctrl+Z undoes a drag.";

/// The mod the walk works on.
const MOD_ID: &str = "editor_blocks_demo";

/// The size the walk asked for.
#[derive(Resource, Clone, Copy)]
struct WalkWindow(f32, f32);

/// Install the copy of the tutorial in this run's profile.
fn install_the_demo_mod() {
    let source = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/assets/base/scenarios/tutorial.content.ron"
    );
    let content = std::fs::read_to_string(source)
        .expect("the tutorial ships in the repository")
        .replace("self://", "base/")
        .replacen("id: \"tutorial\"", "id: \"blocks_tutorial\"", 1);
    let bundle = format!("{MOD_ID}.bundle.ron");
    let file = format!("{MOD_ID}.content.ron");
    let manifest = format!(
        "(\n    content: [\n        \"{file}\",\n    ],\n    meta: (\n        name: \"Blocks Demo\",\n        \
         description: \"A copy of the tutorial.\",\n        author: \"\",\n        version: \"1.0.0\",\n        \
         dependencies: [],\n        icon: None,\n        screenshots: [],\n    ),\n)\n"
    );
    nova_assets::mod_cache::install_local(
        MOD_ID,
        "1.0.0",
        &bundle,
        &[
            (bundle.clone(), manifest.into_bytes()),
            (file, content.into_bytes()),
        ],
    )
    .expect("the demo mod installs in the run's profile");
}

fn main() -> bevy::app::AppExit {
    let cli = Cli::parse();
    open_world_stage::isolate_the_profile(env!("CARGO_CRATE_NAME"));
    install_the_demo_mod();

    let mut app = editor_app(true, None);
    app.add_systems(OnEnter(GameStates::MainMenu), open_the_sandbox);
    if !capturing() {
        app.add_plugins(diagnostics::DiagnosticsHintPlugin::new(INTRO));
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

fn capturing() -> bool {
    std::env::var_os("NOVA_CAPTURE").is_some()
}

/// Fix the window at the walk's size.
fn size_the_window(size: Res<WalkWindow>, mut windows: Query<&mut Window, With<PrimaryWindow>>) {
    if let Ok(mut window) = windows.single_mut() {
        window.resolution.set(size.0, size.1);
        window.resizable = false;
    }
}

/// What the Sandbox button does, once.
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

mod smoke {
    use std::sync::Arc;

    use bevy::input::keyboard::Key;
    use nova_protocol::nova_debug::harness::{shoot, AutopilotPlugin, Predicate};

    use super::{
        ui_walk::{block_centre, block_named, BlockName, BlockPoint, Gestures},
        *,
    };

    const BOOT_SECS: f32 = 300.0;
    const BEAT_SECS: f32 = 60.0;
    const SHOT_SECS: f32 = 120.0;
    const OPEN: &str = "Hub Row editor_blocks_demo";
    const CANVAS: &str = "Block Canvas";
    const FIRED: &str = "blocks_fired";
    const FIRED_VALUE: f64 = 7.0;
    const SAVE: &[KeyCode] = &[KeyCode::ControlLeft, KeyCode::KeyS];
    const UNDO: &[KeyCode] = &[KeyCode::ControlLeft, KeyCode::KeyZ];
    const REDO: &[KeyCode] = &[KeyCode::ControlLeft, KeyCode::KeyY];

    /// The ids the walk's drops minted, and what it measured before them.
    #[derive(Resource, Default, Clone)]
    struct Built {
        hat: String,
        set: String,
        sequence: String,
        step: String,
        debug: String,
        or: String,
        blocks: usize,
        depth: usize,
        hat_at: Vec2,
        palette_all: usize,
        palette_events: usize,
    }

    fn probe(world: &World) -> &EditorProbe {
        world.resource::<EditorProbe>()
    }

    fn built(world: &World) -> Built {
        world.resource::<Built>().clone()
    }

    fn editor(check: impl Fn(&EditorProbe) -> bool + Send + Sync + 'static) -> Arc<Predicate> {
        Arc::new(move |world: &World| world.get_resource::<EditorProbe>().is_some_and(&check))
    }

    /// Advance once one more undo step stands than the last note took.
    fn stepped() -> Arc<Predicate> {
        Arc::new(|world: &World| probe(world).history_depth == built(world).depth + 1)
    }

    fn readout(world: &World) -> String {
        world.get_resource::<EditorProbe>().map_or_else(
            || "no probe".to_string(),
            |probe| {
                format!(
                    "selected {:?} depth {} undo {:?} shell {:?} status {:?}",
                    probe.selected_nodes,
                    probe.history_depth,
                    probe.next_undo,
                    probe.shell,
                    probe.status
                )
            },
        )
    }

    /// Where `name` stands now, for a name the walk knows at compile time.
    fn at(name: &'static str) -> BlockPoint {
        block_centre(move |_| name.to_string())
    }

    /// Where a name worked out from the walk's notes stands now.
    fn at_built(name: impl Fn(&Built) -> String + Send + Sync + 'static) -> BlockPoint {
        block_centre(move |world| name(&built(world)))
    }

    /// A name worked out from the walk's notes, as the start of a drag.
    fn named_built(name: impl Fn(&Built) -> String + Send + Sync + 'static) -> BlockName {
        Arc::new(move |world: &World| name(&built(world)))
    }

    /// Mark where the history stands, before a gesture is judged against it.
    fn note(script: AutopilotPlugin<GameStates>, what: &str) -> AutopilotPlugin<GameStates> {
        script
            .step(format!("editor blocks: note the document before {what}"))
            .on_enter(|world: &mut World| {
                let (depth, blocks) = {
                    let probe = probe(world);
                    (probe.history_depth, probe.shell.blocks)
                };
                let mut notes = world.resource_mut::<Built>();
                notes.depth = depth;
                notes.blocks = blocks;
            })
            .add()
    }

    /// One drag with the harness pointer, with an optional frame while it is
    /// held over the target.
    fn drag(
        script: AutopilotPlugin<GameStates>,
        label: &str,
        from: BlockName,
        to: BlockPoint,
        held_shot: Option<&'static str>,
        landed: Arc<Predicate>,
    ) -> AutopilotPlugin<GameStates> {
        script
            .drag_block(
                &format!("editor blocks: {label}"),
                from,
                to,
                |held| match held_shot.filter(|_| capturing()) {
                    Some(name) => shot(held, name),
                    None => held,
                },
                landed,
            )
            .step(format!("editor blocks: settle after {label}"))
            .until(frames(4))
            .add()
    }

    fn chord(
        script: AutopilotPlugin<GameStates>,
        label: &str,
        keys: &'static [KeyCode],
        landed: Arc<Predicate>,
    ) -> AutopilotPlugin<GameStates> {
        script
            .step(format!("editor blocks: {label}"))
            .on_enter(move |world: &mut World| {
                for key in keys {
                    press_key(*key)(world);
                }
            })
            .until(landed)
            .diagnose(readout)
            .deadline(BEAT_SECS)
            .add()
            .step(format!("editor blocks: let go after {label}"))
            .on_enter(move |world: &mut World| {
                for key in keys {
                    release_key(*key)(world);
                }
            })
            .until(frames(3))
            .add()
    }

    /// Click a chip, replace its text in `field`, press Enter. A plain chip
    /// opens `Chip Input Field`; a variable chip opens its list on `Pick Filter`.
    fn type_in(
        script: AutopilotPlugin<GameStates>,
        label: &str,
        chip: impl Fn(&Built) -> String + Send + Sync + 'static,
        field: &'static str,
        erase: usize,
        text: &'static str,
    ) -> AutopilotPlugin<GameStates> {
        let script = script
            .step(format!("editor blocks: point at {label}"))
            .on_enter(move |world: &mut World| {
                let name = chip(&built(world));
                hover_named(name)(world);
            })
            .until(frames(3))
            .add()
            .step(format!("editor blocks: press {label}"))
            .on_enter(press_mouse(MouseButton::Left))
            .until(frames(3))
            .add()
            .step(format!("editor blocks: release on {label}"))
            .on_enter(release_mouse(MouseButton::Left))
            .until(ui_node_present(field))
            .diagnose(readout)
            .deadline(BEAT_SECS)
            .add();
        let script = if capturing() && field == "Pick Filter" {
            shot(script, "editor-blocks-popup.png")
        } else {
            script
        };
        script
            .step(format!("editor blocks: type {label}"))
            .on_enter(move |world: &mut World| {
                for _ in 0..erase {
                    press_edit_key(Key::Backspace)(world);
                }
                type_text(text)(world);
            })
            .until(frames(3))
            .add()
            .step(format!("editor blocks: enter {label}"))
            .on_enter(press_edit_key(Key::Enter))
            .until(Arc::new(move |world: &World| {
                !ui_node_present(field)(world)
            }))
            .diagnose(readout)
            .deadline(BEAT_SECS)
            .add()
    }

    /// Open a chip's list and press outside it: the list goes, and its filter
    /// field goes with it.
    fn dismiss_list(
        script: AutopilotPlugin<GameStates>,
        chip: impl Fn(&Built) -> String + Send + Sync + 'static,
    ) -> AutopilotPlugin<GameStates> {
        script
            .step("editor blocks: point at the chip again")
            .on_enter(move |world: &mut World| {
                let name = chip(&built(world));
                hover_named(name)(world);
            })
            .until(frames(3))
            .add()
            .step("editor blocks: press the chip again")
            .on_enter(press_mouse(MouseButton::Left))
            .until(frames(3))
            .add()
            .step("editor blocks: its list opens")
            .on_enter(release_mouse(MouseButton::Left))
            .until(ui_node_present("Pick Filter"))
            .diagnose(readout)
            .deadline(BEAT_SECS)
            .add()
            .step("editor blocks: point away from the list")
            .on_enter(|world: &mut World| {
                let window = world
                    .query::<&Window>()
                    .iter(world)
                    .next()
                    .expect("the window stands");
                let away = Vec2::new(window.width() - 30.0, window.height() - 60.0);
                move_cursor(away)(world);
            })
            .until(frames(3))
            .add()
            .step("editor blocks: a press outside closes the list")
            .on_enter(press_mouse(MouseButton::Left))
            .until(Arc::new(|world: &World| {
                !ui_node_present("Pick Filter")(world)
            }))
            .diagnose(readout)
            .deadline(BEAT_SECS)
            .add()
            .step("editor blocks: let the press go")
            .on_enter(release_mouse(MouseButton::Left))
            .until(frames(3))
            .add()
            .step("editor blocks: the press touched nothing stale")
            .on_enter(|world: &mut World| {
                let lines = &probe(world).shell.log_lines;
                assert!(
                    !lines.iter().any(|line| line.contains("despawned")),
                    "{lines:?}"
                );
            })
            .add()
    }

    /// What the last drop left selected.
    fn selected(world: &World) -> String {
        probe(world)
            .shell
            .selected_block
            .clone()
            .expect("a drop selects the block it made")
    }

    fn shot(
        script: AutopilotPlugin<GameStates>,
        name: &'static str,
    ) -> AutopilotPlugin<GameStates> {
        script
            .step(format!("editor blocks: settle before {name}"))
            .until(frames(6))
            .add()
            .step(format!("editor blocks: shoot {name}"))
            .on_enter(move |world: &mut World| shoot(world, name))
            .until(shot_written(name))
            .deadline(SHOT_SECS)
            .add()
    }

    /// A drop onto the palette or a slot, keeping the id it minted.
    fn keep(
        script: AutopilotPlugin<GameStates>,
        write: impl Fn(&mut Built, String) + Send + Sync + 'static,
    ) -> AutopilotPlugin<GameStates> {
        script
            .step("editor blocks: keep the id of the new block")
            .on_enter(move |world: &mut World| {
                let id = selected(world);
                write(&mut world.resource_mut::<Built>(), id);
            })
            .add()
    }

    /// The canvas near its top left, clear of every handler once it has rolled.
    fn clear_ground() -> BlockPoint {
        Arc::new(|world: &World| {
            ui_node_rect(world, CANVAS).map(|rect| rect.min + Vec2::new(60.0, 40.0))
        })
    }

    pub(super) fn script() -> AutopilotPlugin<GameStates> {
        let script = AutopilotPlugin::<GameStates>::new()
            .step("editor blocks: the hub lists the mod")
            .on_enter(|world: &mut World| {
                world.init_resource::<Built>();
            })
            .until(ui_node_present(OPEN))
            .deadline(BOOT_SECS)
            .add();
        let script = if capturing() {
            shot(script, "editor-blocks-hub.png")
        } else {
            script
        };
        let script = open_world_stage::open_hub_row(
            script,
            "editor blocks: open the mod",
            OPEN,
            ui_node_present("View Tab Events"),
            BOOT_SECS,
        );
        let mut script = script.click_named(
            "editor blocks: take the Events tab",
            "View Tab Events",
            editor(|probe| probe.shell.rail_tab == "Events" && probe.shell.blocks > 0),
            BEAT_SECS,
        );
        if capturing() {
            script = shot(script, "editor-blocks-canvas.png");
        }
        script = script
            .step("editor blocks: the tutorial opens with no block at fault")
            .on_enter(|world: &mut World| {
                let shell = &probe(world).shell;
                assert_eq!(shell.errors, 0, "{shell:?}");
                assert_eq!(shell.block_faults, 0, "{shell:?}");
                info!("editor blocks: the tutorial draws {} blocks", shell.blocks);
            })
            .add();

        // The rail filters the palette by group, together with the filter box.
        script = script.filter_palette("");
        script = script
            .step("editor blocks: count the whole palette")
            .until(editor(|probe| {
                probe.shell.palette_group == "All" && !probe.shell.palette_entries.is_empty()
            }))
            .diagnose(readout)
            .deadline(BEAT_SECS)
            .add()
            .step("editor blocks: note the whole palette")
            .on_enter(|world: &mut World| {
                let all = probe(world).shell.palette_entries.len();
                world.resource_mut::<Built>().palette_all = all;
            })
            .add()
            .click_named(
                "editor blocks: show only Events",
                "Palette Rail Events",
                editor(|probe| probe.shell.palette_group == "Events"),
                BEAT_SECS,
            )
            .step("editor blocks: the palette lists only Events blocks")
            .on_enter(|world: &mut World| {
                let all = built(world).palette_all;
                let entries = probe(world).shell.palette_entries.clone();
                assert!(!entries.is_empty(), "no Events block shown");
                assert!(entries.len() < all, "{} of {all}", entries.len());
                assert!(
                    entries.iter().all(|entry| entry.starts_with("Events / ")),
                    "{entries:?}"
                );
                info!(
                    "editor blocks: the Events group shows {} of {all} blocks",
                    entries.len()
                );
                world.resource_mut::<Built>().palette_events = entries.len();
            })
            .add();
        if capturing() {
            script = shot(script, "s4-rail-filter.png");
        }
        script = script.filter_palette("start");
        script = script
            .step("editor blocks: the group and the filter both apply")
            .on_enter(|world: &mut World| {
                let events = built(world).palette_events;
                let entries = probe(world).shell.palette_entries.clone();
                assert!(!entries.is_empty(), "On Start is an Events block");
                assert!(entries.len() < events, "{} of {events}", entries.len());
                assert!(
                    entries.iter().all(|entry| {
                        entry.starts_with("Events / ") && entry.to_lowercase().contains("start")
                    }),
                    "{entries:?}"
                );
                info!("editor blocks: Events and \"start\" show {entries:?}");
            })
            .add();
        script = script.filter_palette("");
        script = script
            .click_named(
                "editor blocks: a second click on Events shows every group",
                "Palette Rail Events",
                Arc::new(|world: &World| {
                    let all = built(world).palette_all;
                    let shell = &probe(world).shell;
                    shell.palette_group == "All" && shell.palette_entries.len() == all
                }),
                BEAT_SECS,
            )
            .click_named(
                "editor blocks: show only Conditions",
                "Palette Rail Conditions",
                Arc::new(|world: &World| {
                    let shell = &probe(world).shell;
                    shell.palette_group == "Conditions"
                        && !shell.palette_entries.is_empty()
                        && shell
                            .palette_entries
                            .iter()
                            .all(|entry| entry.starts_with("Conditions / "))
                }),
                BEAT_SECS,
            )
            .click_named(
                "editor blocks: All shows every group",
                "Palette Rail All",
                Arc::new(|world: &World| {
                    let all = built(world).palette_all;
                    let shell = &probe(world).shell;
                    shell.palette_group == "All" && shell.palette_entries.len() == all
                }),
                BEAT_SECS,
            );
        script = script
            .step("editor blocks: point at the canvas")
            .on_enter(|world: &mut World| {
                let centre = ui_node_centre(world, CANVAS).expect("the canvas is up");
                move_cursor(centre)(world);
            })
            .until(frames(3))
            .add()
            .step("editor blocks: roll past the handlers")
            .on_enter(scroll_lines(-80.0))
            .until(frames(4))
            .add();

        // A hat, on the bare canvas.
        script = note(script, "the hat");
        script = script.filter_palette("On Start");
        script = drag(
            script,
            "a hat",
            block_named("Palette On Start"),
            clear_ground(),
            None,
            stepped(),
        );
        script = keep(script, |notes, id| notes.hat = id);

        // Move the placed hat across the bare canvas: the ghost follows the
        // pointer from its first frame.
        script = note(script, "the hat move");
        script = drag(
            script,
            "the placed hat",
            named_built(|notes| format!("Block {}", notes.hat)),
            Arc::new(|world: &World| {
                ui_node_rect(world, CANVAS).map(|rect| rect.min + Vec2::new(360.0, 60.0))
            }),
            None,
            stepped(),
        );

        // A stack in the hat.
        script = note(script, "the stack block");
        script = script.filter_palette("Variable Set");
        script = drag(
            script,
            "a stack block",
            block_named("Palette Variable Set"),
            at_built(|notes| format!("Slot {} 0", notes.hat)),
            None,
            stepped(),
        );
        script = keep(script, |notes, id| notes.set = id);

        // A C-block under it, and the gate in its step.
        script = note(script, "the C-block");
        script = script.filter_palette("Sequence");
        script = drag(
            script,
            "a C-block",
            block_named("Palette Sequence"),
            at_built(|notes| format!("Slot {} 1", notes.hat)),
            None,
            stepped(),
        );
        script = keep(script, |notes, id| notes.sequence = id);

        // The values, typed on the block.
        script = type_in(
            script,
            "the key",
            |notes| format!("Chip {}/{} 1", notes.hat, notes.set),
            "Pick Filter",
            0,
            FIRED,
        );
        script = dismiss_list(script, |notes| {
            format!("Chip {}/{} 1", notes.hat, notes.set)
        });
        script = type_in(
            script,
            "the value",
            |notes| format!("Chip {}/{} 2", notes.hat, notes.set),
            "Chip Input Field",
            1,
            "7",
        );
        script = type_in(
            script,
            "the sequence key",
            |notes| format!("Chip {}/{} 1", notes.hat, notes.sequence),
            "Chip Input Field",
            0,
            "blocks_beat",
        );

        script = script
            .step("editor blocks: find the step the sequence came with")
            .on_enter(|world: &mut World| {
                let prefix = {
                    let notes = world.resource::<Built>();
                    format!("Block {}/{}/", notes.hat, notes.sequence)
                };
                let mut names = world.query::<&Name>();
                let step = names
                    .iter(world)
                    .find_map(|name| {
                        name.as_str()
                            .strip_prefix(prefix.as_str())
                            .map(str::to_string)
                    })
                    .expect("a new sequence holds a step");
                world.resource_mut::<Built>().step = step;
            })
            .add();
        script = note(script, "the gate");
        script = script.filter_palette("Wait For");
        script = drag(
            script,
            "a gate",
            block_named("Palette Wait For"),
            at_built(|notes| format!("Slot {}/{}/{} gate", notes.hat, notes.sequence, notes.step)),
            None,
            stepped(),
        );

        // An action inside the step, shot while it is held over the gap.
        script = note(script, "the block in the step");
        script = script.filter_palette("Debug Message");
        script = drag(
            script,
            "a block into the step",
            block_named("Palette Debug Message"),
            at_built(|notes| format!("Slot {}/{}/{} 0", notes.hat, notes.sequence, notes.step)),
            Some("editor-blocks-drag.png"),
            stepped(),
        );
        script = keep(script, |notes, id| notes.debug = id);

        // A boolean: an Or arrives with two holes, which the Problems tab
        // reports until they are filled.
        script = note(script, "the boolean");
        script = script.filter_palette("Logic");
        script = drag(
            script,
            "a boolean",
            block_named("Palette Or"),
            at_built(|notes| format!("Slot {} filter 0", notes.hat)),
            None,
            stepped(),
        );
        script = keep(script, |notes, id| notes.or = id);
        script = script
            .step("editor blocks: a half-built Or is a problem on its block")
            .until(editor(|probe| {
                probe.shell.errors > 0 && probe.shell.block_faults > 0
            }))
            .diagnose(readout)
            .deadline(BEAT_SECS)
            .add();
        script = note(script, "the first comparison");
        script = drag(
            script,
            "a comparison into a socket",
            block_named("Palette =="),
            at_built(|notes| format!("Socket {}/{} condition 0", notes.hat, notes.or)),
            None,
            stepped(),
        );

        // One drag is one undo step, both ways.
        script = chord(
            script,
            "undo the drag",
            UNDO,
            Arc::new(|world: &World| {
                let notes = built(world);
                let probe = probe(world);
                probe.history_depth == notes.depth && probe.shell.blocks == notes.blocks
            }),
        );
        script = chord(
            script,
            "redo the drag",
            REDO,
            Arc::new(|world: &World| probe(world).shell.blocks > built(world).blocks),
        );
        script = note(script, "the second comparison");
        script = drag(
            script,
            "a second comparison",
            block_named("Palette =="),
            at_built(|notes| format!("Socket {}/{} condition 2", notes.hat, notes.or)),
            None,
            stepped(),
        );
        script = script
            .step("editor blocks: a whole Or clears the problem")
            .until(editor(|probe| {
                probe.shell.errors == 0 && probe.shell.block_faults == 0
            }))
            .diagnose(readout)
            .deadline(BEAT_SECS)
            .add();

        // A refusal: only a hat stands alone, and nothing moves.
        script = note(script, "the refused drop");
        script = drag(
            script,
            "a comparison onto bare canvas",
            block_named("Palette =="),
            Arc::new(|world: &World| {
                ui_node_rect(world, CANVAS).map(|rect| rect.max - Vec2::splat(60.0))
            }),
            None,
            Arc::new(|world: &World| probe(world).status.contains("only")),
        );
        script = script
            .step("editor blocks: the refused drop changed nothing")
            .on_enter(|world: &mut World| {
                let notes = built(world);
                let probe = probe(world);
                assert_eq!(probe.history_depth, notes.depth, "{}", probe.status);
                assert_eq!(probe.shell.blocks, notes.blocks, "{}", probe.status);
                info!("editor blocks: refused with: {}", probe.status);
            })
            .add();

        // Moving a block out of the step, and binning it on the palette.
        script = note(script, "the move");
        script = drag(
            script,
            "a block out of the step",
            named_built(|notes| {
                format!(
                    "Block {}/{}/{}/{}",
                    notes.hat, notes.sequence, notes.step, notes.debug
                )
            }),
            at_built(|notes| format!("Slot {} 2", notes.hat)),
            None,
            stepped(),
        );
        script = keep(script, |notes, id| notes.debug = id);
        script = script
            .step("editor blocks: the block landed at the foot of the hat")
            .until(Arc::new(|world: &World| {
                let notes = built(world);
                ui_node_present(format!("Block {}/{}", notes.hat, notes.debug))(world)
            }))
            .diagnose(readout)
            .deadline(BEAT_SECS)
            .add();
        // Zoom in on the hat and carry the block back under the scale: the
        // spike for whether a drag still lands on what it was aimed at.
        script = script
            .step("editor blocks: aim the zoom at the hat")
            .on_enter(|world: &mut World| {
                let hat = format!("Block {}", built(world).hat);
                let centre = ui_node_centre(world, &hat).expect("the hat is on screen");
                move_cursor(centre)(world);
                press_key(KeyCode::ControlLeft)(world);
            })
            .until(frames(3))
            .add()
            .step("editor blocks: zoom in")
            .on_enter(scroll_lines(3.0))
            .until(editor(|probe| probe.shell.canvas_zoom > 100))
            .diagnose(readout)
            .deadline(BEAT_SECS)
            .add()
            .step("editor blocks: let go of Ctrl")
            .on_enter(release_key(KeyCode::ControlLeft))
            .until(frames(3))
            .add();
        script = note(script, "the zoomed move");
        script = drag(
            script,
            "a block back under zoom",
            named_built(|notes| format!("Block {}/{}", notes.hat, notes.debug)),
            at_built(|notes| format!("Slot {} 1", notes.hat)),
            Some("editor-blocks-zoom.png"),
            stepped(),
        );
        script = keep(script, |notes, id| notes.debug = id);

        // Pan by the middle button: the hat follows the pointer.
        script = script
            .step("editor blocks: take hold of the canvas")
            .on_enter(|world: &mut World| {
                let hat = format!("Block {}", built(world).hat);
                let centre = ui_node_centre(world, &hat).expect("the hat is on screen");
                world.resource_mut::<Built>().hat_at = centre;
                let ground = ui_node_rect(world, CANVAS).expect("the canvas is up").max;
                move_cursor(ground - Vec2::splat(80.0))(world);
            })
            .until(frames(3))
            .add()
            .step("editor blocks: press the middle button")
            .on_enter(press_mouse(MouseButton::Middle))
            .until(frames(3))
            .add()
            .step("editor blocks: pull the canvas")
            .on_enter(|world: &mut World| {
                let ground = ui_node_rect(world, CANVAS).expect("the canvas is up").max;
                move_cursor(ground - Vec2::new(40.0, 60.0))(world);
            })
            .until(frames(3))
            .add()
            .step("editor blocks: let the canvas go")
            .on_enter(release_mouse(MouseButton::Middle))
            .until(Arc::new(|world: &World| {
                let notes = built(world);
                ui_node_centre(world, &format!("Block {}", notes.hat))
                    .is_some_and(|at| at.distance(notes.hat_at) > 20.0)
            }))
            .diagnose(readout)
            .deadline(BEAT_SECS)
            .add();
        script = note(script, "the bin");
        script = drag(
            script,
            "a block onto the palette",
            named_built(|notes| format!("Block {}/{}", notes.hat, notes.debug)),
            at("Block Palette"),
            None,
            Arc::new(|world: &World| {
                let notes = built(world);
                let probe = probe(world);
                probe.history_depth == notes.depth + 1 && probe.shell.blocks < notes.blocks
            }),
        );

        // Save, then read the position out of the file.
        script = chord(
            script,
            "save",
            SAVE,
            Arc::new(|world: &World| !probe(world).dirty),
        );
        script = script
            .step("editor blocks: the hat's position is in the saved file")
            .on_enter(|_world: &mut World| {
                let root = std::env::var(nova_assets::mod_cache::MOD_CACHE_ROOT_ENV)
                    .expect("the run sets its own mod root");
                let file = format!("{root}/mods/{MOD_ID}/{MOD_ID}.content.ron");
                let saved = std::fs::read_to_string(&file).expect("the save wrote the mod");
                assert!(saved.contains("position:"), "no position in {file}");
                assert!(saved.contains(FIRED), "the authored key is not in {file}");
            })
            .add();

        // And play it: the event the blocks authored fires in the world.
        script
            .click_named(
                "editor blocks: play",
                "Play Button",
                scenario_variable_is(FIRED, FIRED_VALUE),
                BOOT_SECS,
            )
            .step("editor blocks: the authored event fired in the world")
            .on_enter(|_world: &mut World| {
                info!("editor blocks: {FIRED} reads {FIRED_VALUE} in the flown scenario");
            })
            .add()
    }
}
