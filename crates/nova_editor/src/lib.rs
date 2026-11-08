//! The spaceship editor: a scene where you build a ship out of sections and then
//! hand it off to a scenario simulation.
//!
//! ONE DOCUMENT, ITS OWN VIEWS: the open asset is a scenario, a ship entered
//! in one, a section or a form (`project::DocumentType`), and each has its own
//! views (`config::EditorViewType`). A scenario's SCENE lays the range out on a
//! stage with the Inspector beside it, and its EVENTS gives the stage's room to
//! the script (`ui::shell::sync_panels`). The top bar's view switch shows only
//! the views of the open document.
//!
//! Structure:
//! - `readout`   - what the hull under construction weighs, pushes and turns like
//! - `node`      - the document: the node tree and the edit context
//! - `history`   - undo and redo: labelled snapshots of the document, and the dirty mark
//! - `inspect`   - the reflected fields of the inspected node, and their write-back
//! - `camera`    - the viewport rig: look, orbit, dolly, pan
//! - `frame`     - putting the camera on a node, on demand
//! - `tool`      - which tool the viewport holds, and the snap steps
//! - `gizmo`     - the handles that move, turn and size the selection
//! - `selection` - a press on nothing, the selection box, select all
//! - `clipboard` - duplicate, copy and paste
//! - `glyph`     - one mark per kind, wherever a kind is drawn
//! - `config`    - the placement state + screen furniture
//! - `preview`   - the one place a section or object config becomes preview entities
//! - `placement` - creating ships and objects + the pointer place/preview/delete observers
//! - `keybind`   - section keybind chips + click-to-rebind
//! - `gallery`   - the full-screen parts browser that arms the placement tool
//! - `snap`      - where the armed prototype would land, and why not
//! - `skin`      - the derived surface, re-derived live while a part is dragged
//! - `stage`     - the ground plane the range is laid out on
//! - `highlight` - the node under the pointer, lit on the rail and on the stage
//! - `generate`  - the seed, and the hull the collapse builds from it
//! - `station`   - the station design a ship marked as a station spawns as
//! - `scenario`  - the default world a document is seeded with, and the sandbox script
//! - `template`  - the worlds File > New Scenario can start from
//! - `event`     - the script as nodes: a handler, its filters and actions, its
//!   beats, and a condition down to its operators
//! - `bundle`    - the document as a saved mod bundle, and the read back out of one
//! - `ui`        - the wiki-style rail + component drawer + tooltip
//! - `probe`     - the one public, read-only snapshot of all of the above
#![warn(missing_docs)]

use bevy::{
    gizmos::GizmoPlugin,
    input::mouse::{AccumulatedMouseMotion, MouseWheel},
    prelude::*,
    transform::TransformSystems,
    window::{CursorGrabMode, CursorOptions, PrimaryWindow},
};
use nova_gameplay::prelude::*;
use nova_scenario::prelude::*;
use nova_ship::prelude::{gallery_open, GalleryState, HeldPart, PartsGallerySystems};
use nova_ui::prelude::{
    close_popups, in_input_mode, in_input_mode_at_most, owns_or_enters, ClaimKeyboard, DoubleClick,
    InputMode, InputModeSystems, PopupRoot, TextFieldSystems,
};

mod asset;
mod asset_index;
mod bundle;
mod camera;
mod clipboard;
mod config;
/// The editor's four cue voices: place, remove, rotate, deny.
mod cues;
mod event;
mod frame;
mod gallery;
mod generate;
mod gizmo;
mod glyph;
mod highlight;
mod history;
mod inspect;
mod keybind;
mod log_feed;
mod node;
mod palette;
mod placement;
mod preview;
mod probe;
mod problems;
mod project;
mod readout;
mod scenario;
mod selection;
mod skin;
mod snap;
mod stage;
mod station;
mod template;
mod tool;
mod ui;

use asset::{
    session::{project_sections_moved, sync_project_sections, ProjectSections},
    view::{draw_bench_sockets, sync_bench_view, sync_socket_views},
};
use bundle::{save_key, FileWindowRequest};
pub use camera::CameraGestureType;
use camera::{drive_editor_camera, sync_viewport_pointer};
use clipboard::{clipboard_keys, EditorClipboard};
use config::{
    editor_gizmo_config, EditorGizmos, EditorOverlays, EditorStatus, EditorViewType, HoveredNode,
    PlacementPose, PlacementPreview, SelectedNode,
};
use frame::{apply_frame_request, frame_key, CameraFraming, FrameRequest};
use generate::{read_seed_field, HullSeed};
use gizmo::sync_gizmo;
use highlight::{paint_hovered_rows, sync_hovered_node};
use history::{history_keys, EditHistory};
use keybind::{
    apply_section_rebind, hide_section_keybind_labels, position_section_keybind_labels,
    sync_section_keybind_labels, EditorRebind,
};
pub use log_feed::{editor_log_layer, EditorLogFeed};
use node::{
    drop_edited_views, ensure_document, ids_or_parents_moved, rebuild_node_views,
    reflow_auto_ships, report_duplicate_ids, sync_camera_focus, sync_object_views, sync_ship_focus,
    sync_stage_frame, teardown_document, EditContext, ObjectBodyStale, StageFrame,
};
use placement::{
    clear_placement_preview, cycle_placement_pose, delete_key, disarm_outside_ship,
    draw_link_points, draw_ship_heading, found_empty_ship, on_click_spaceship_section,
    on_stage_drag, on_stage_drag_end, on_stage_drag_start, on_stage_release,
    pick_section_under_pointer, sync_placement_ghost, update_placement_preview, StageDrag,
};
use probe::{sync_editor_probe, sync_shell_probe};
pub use probe::{EditorPlacement, EditorProbe, EditorSection, EditorTool, ShellProbe};
use problems::{reset_problems, sync_problems, Problems};
use project::{
    apply_pending_mods, conform_view,
    trail::{CameraReturn, DocumentTrail},
    verbs::go_back,
    PendingMods,
};
use readout::sync_ship_readout;
use scenario::setup_scenario;
use selection::{
    empty_space_press, select_all, setup_selection_frame, sync_selection_frame, EmptyPress,
};
use skin::sync_editor_skin;
pub use stage::AxisRoseOrigin;
use stage::{draw_axis_rose, draw_node_marks, draw_object_volumes, draw_world_grid};
use tool::{sync_scale_refusal, tool_keys, StageTool};
pub use tool::{GizmoSpaceType, StageToolType};
use ui::{
    a_ship_is_entered,
    add_menu::add_menu_key,
    asset_files::fit_viewed_images,
    assets::{
        apply_asset_rename, asset_tile_keys, sync_asset_controls, sync_asset_thumbs, sync_assets,
        AssetKindFilter, AssetRenaming,
    },
    browser::{sync_browser, sync_problem_rows, BrowserTabType},
    callout::sync_placement_callout,
    canvas::{BlockPopup, CanvasDrag},
    files::{close_file_window, open_file_window, sync_path_window, sync_save_name},
    hierarchy::{focus_pending_rename, on_tree_caret, PendingRename},
    hub::{hub_systems, spawn_hub, HubForm},
    inspector::{
        apply_inspector_edits, paint_field_reasons, paint_swatch_hover, sync_inspector,
        sync_inspector_head, sync_inspector_tooltip, sync_reference_faults, InspectorFolds,
    },
    list_field::fill_list_choices,
    log::{sync_log, sync_log_controls, StatusLog},
    menu::{
        close_menu_on_item, close_menus, close_open_menu, sync_history_menu, sync_menu_item_paint,
        sync_menus, sync_unsaved_mark, sync_view_menu_marks, OpenMenu,
    },
    menu_context::{sync_menu_verbs, tidy_menu_separators},
    pick::{pick_filter_escape, submit_pick_filter, sync_pick_popups},
    picker::{revert_picker, settle_on_close, sync_colour_windows, ColourWindow},
    plate::sync_nameplates,
    rail::sync_scene_tooltip,
    row_copy::{copy_rows_hovered, row_copy_keys, sync_copy_controls, sync_copy_marks, CopyRows},
    section_kind::{section_kind_keys, sync_section_kind_tiles, SectionKindModal},
    settings::{apply_settings_fields, sync_settings_window},
    setup_editor_scene,
    shell::{
        panel_keys, stage_enter_opens_ship, sync_panels, sync_ship_step, sync_stage_overlay,
        sync_toolbar, EditorPanels,
    },
    stage_menu::{stage_context_menu, StageRightPress},
    sync_context_panels, sync_editor_title, sync_hull_plan, sync_key_legend, sync_part_ticks,
    sync_part_zones, sync_play_button, sync_row_trash, sync_scene_list, sync_skin_toggle,
    sync_status_line, sync_style_list, sync_view_tabs,
    thumbnail::{drive_thumbnails, AssetThumbnails},
    window::{ask_about_stopped_leave, close_confirm_window, on_destructive_item},
};

/// Glob-import surface: `use nova_editor::prelude::*` brings [`NovaEditorPlugin`],
/// the sandbox registration ordering handle and the read-only [`EditorProbe`]
/// snapshot into scope.
pub mod prelude {
    pub use super::{
        cues::EditorCueSystems, AxisRoseOrigin, CameraGestureType, EditorOpensScratch,
        EditorOverlaySystems, EditorPlacement, EditorProbe, EditorSection, EditorTool,
        GizmoSpaceType, NovaEditorPlugin, StageToolType,
    };
}

/// The stage's immediate overlays: the grid, the marks, the link points and the
/// axis rose.
///
/// `PostUpdate`, after [`TransformSystems::Propagate`]. A line drawn from a
/// `GlobalTransform` in `Update` uses last frame's pose and renders through this
/// frame's camera, which threw the rose off the screen on every moving frame.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EditorOverlaySystems;

/// The spaceship editor plugin.
///
/// `nova_core` adds this as its default "game" plugin when no custom game plugins are
/// supplied (see `AppBuilder`). Examples that provide their own scenario opt out of it.
pub struct NovaEditorPlugin;

impl Plugin for NovaEditorPlugin {
    fn build(&self, app: &mut App) {
        editor_plugin(app);
    }
}

#[derive(Clone, Eq, PartialEq, Debug, Hash, Default, States)]
pub(crate) enum ExampleStates {
    #[default]
    Loading,
    /// The Mod Hub: the mods on this machine, before any is open.
    Hub,
    Editor,
    Scenario,
}

/// Enter the Sandbox straight into a mod with no id, past the Mod Hub.
///
/// A walk or a lesson inserts this to reach a document in one press. Nothing in
/// the player path sets it, and with it absent the Sandbox opens the hub.
#[derive(Resource, Debug, Default)]
pub struct EditorOpensScratch;

fn editor_plugin(app: &mut App) {
    app.init_state::<ExampleStates>();
    // Everything the editor draws in immediate mode, at one weight. See
    // `EditorGizmos`. Gated like the picking backend below: registering a
    // group adds the render-side system that turns its storage into meshes,
    // and a headless app has neither the plugin nor the assets it wants.
    if app.is_plugin_added::<GizmoPlugin>() {
        app.insert_gizmo_config(EditorGizmos, editor_gizmo_config());
    }
    app.insert_resource(HeldPart(None));
    app.init_resource::<EditContext>();
    app.init_resource::<SelectedNode>();
    app.init_resource::<DocumentTrail>();
    app.init_resource::<CameraReturn>();
    app.init_resource::<HoveredNode>();
    app.init_resource::<EditorRebind>();
    app.init_resource::<EditorOverlays>();
    app.init_resource::<AxisRoseOrigin>();
    app.init_resource::<StageFrame>();
    app.init_resource::<EditorViewType>();
    // Random on init, so the first Generate of a session is a ship nobody
    // else has rather than everyone's seed zero.
    app.init_resource::<HullSeed>();
    // The one line the editor speaks through - the placement readout and every
    // verb that has something to say both write it. See `EditorStatus`.
    app.init_resource::<EditorStatus>();
    app.init_resource::<EditHistory>();
    // Outlives the session: it is applied on the way out of Playing.
    app.init_resource::<PendingMods>();
    app.init_resource::<HubForm>();
    app.init_resource::<DoubleClick<String>>();
    app.init_resource::<AssetKindFilter>();
    app.init_resource::<AssetRenaming>();
    app.init_resource::<asset::files::ModFolder>();
    app.init_resource::<AssetThumbnails>();
    app.add_systems(
        OnExit(ExampleStates::Editor),
        |mut thumbnails: ResMut<AssetThumbnails>| thumbnails.stop(),
    );
    app.init_resource::<ProjectSections>();
    // In every state: Play flies the mod's sections, and a merge on the way there
    // writes a new catalog over them.
    app.add_systems(Update, sync_project_sections.run_if(project_sections_moved));
    app.init_resource::<UnsavedWork>();
    app.init_resource::<FileWindowRequest>();
    app.add_systems(
        Update,
        (
            // Ctrl+S answers under the parts gallery as well. Browse is the
            // gallery's claim on the ARROWS and the letters that reach its
            // grid, and a save is neither - while a gallery covering the whole
            // screen also covers the one line the editor could refuse on, so a
            // builder pressing Ctrl+S out of habit there got no file and no
            // word about it. Insert and Bind still take it: `S` is a letter
            // being typed, and the chord is one a rebind may capture.
            save_key.run_if(in_input_mode_at_most(InputMode::Browse)),
            // Under the same gate as the save.
            history_keys.run_if(in_input_mode_at_most(InputMode::Browse)),
        )
            .chain()
            .run_if(in_state(ExampleStates::Editor)),
    );
    // In the hub as well: Import Package asks through the path window.
    app.add_systems(
        Update,
        (open_file_window, sync_save_name, sync_path_window)
            .chain()
            .run_if(in_state(ExampleStates::Editor).or_eager(in_state(ExampleStates::Hub))),
    );
    app.add_systems(
        OnEnter(ExampleStates::Hub),
        (teardown_document, spawn_hub).chain(),
    );
    app.add_systems(Update, hub_systems().run_if(in_state(ExampleStates::Hub)));
    app.add_systems(
        Update,
        (
            sync_editor_title,
            apply_settings_fields,
            sync_settings_window,
            fill_list_choices,
        )
            .chain()
            .run_if(in_state(ExampleStates::Editor)),
    );
    // Del removes what is marked, at any depth - and it is the key that proves
    // why the modes exist: bound while a rebind waited for it, it deleted the
    // very section it was being bound to.
    app.add_systems(
        Update,
        delete_key
            .run_if(in_input_mode(InputMode::Normal))
            .run_if(in_state(ExampleStates::Editor)),
    );
    // The top bar's menus. Closed on entering the editor because the bar they
    // hang off is `DespawnOnExit(Editor)`: an "open" menu on the way back in
    // would be a dropdown with no button under it.
    app.init_resource::<OpenMenu>();
    app.add_observer(close_menu_on_item);
    app.add_systems(OnEnter(ExampleStates::Editor), close_menus);
    // In every state: the ways out of a flown range and of the pause menu must
    // see the unsaved work too, and the ask starts from any of them.
    app.add_systems(Update, (sync_unsaved_mark, ask_about_stopped_leave).chain());
    app.add_observer(settle_on_close);
    // Normally the gameplay plugin's; init'd here too so a menu-less rig (and
    // the tests below) still has one to write.
    app.init_resource::<EscapeOwner>();

    // Escape is a BACK key in the editor and a pause key only when there is
    // nothing to back out of. `PreUpdate` on purpose: `nova_menu`'s pause
    // toggle reads the answer in `Update`, and the two crates cannot order
    // against each other.
    app.add_systems(PreUpdate, declare_editor_escape_owner);
    // The editor's two keyboard owners, declared where the state that defines
    // them lives. `PreUpdate` and before the arbiter, so `Update` reads a mode
    // that already knows about them.
    app.add_systems(
        PreUpdate,
        declare_editor_keyboard_owner.before(InputModeSystems),
    );
    // Both are verbs, so Normal is the whole guard. The rungs they do NOT own -
    // the Escape that closes the gallery, cancels a rebind or puts back what a
    // field held - belong to those modes' owners, and one press cannot reach
    // two owners because only one of them is the keyboard's.
    //
    // Unpaused as well: the CRT and the pause menu draw OVER the editor without
    // claiming an input mode, so an Escape or Backspace typed into the shell
    // used to back the editor out from under it.
    app.add_systems(
        Update,
        (escape_backs_out, backspace_steps_out)
            .run_if(in_input_mode(InputMode::Normal))
            .run_if(in_state(ExampleStates::Editor).and_then(in_state(PauseStates::Unpaused))),
    );
    app.add_systems(
        Update,
        escape_block_popup
            .run_if(in_input_mode(InputMode::Browse))
            .run_if(in_state(ExampleStates::Editor).and_then(in_state(PauseStates::Unpaused))),
    );

    // The editor is the Sandbox game. When the main menu fronts the app it hands
    // off to Playing with GameMode set: Sandbox enters the editor, NewGame goes
    // straight to the Scenario state. The menu owns the NewGame scenario load;
    // setup_scenario below stays Sandbox-only so the two do not both fire.
    // GameMode defaults to Sandbox (NovaGameplayPlugin), so menu-less apps
    // behave as before. (The spaceship input/section sets are gated on
    // scenario-liveness by nova_scenario, not on these states - see the note
    // at the end of this function.)
    app.add_systems(
        OnEnter(GameStates::Playing),
        (
            |mode: Res<GameMode>,
             scratch: Option<Res<EditorOpensScratch>>,
             mut game_state: ResMut<NextState<ExampleStates>>| {
                game_state.set(match *mode {
                    GameMode::Sandbox if scratch.is_some() => ExampleStates::Editor,
                    GameMode::Sandbox => ExampleStates::Hub,
                    GameMode::NewGame => ExampleStates::Scenario,
                });
            },
        ),
    );

    // Leaving Playing (the pause menu's Back to Main Menu) must tear the
    // editor scene down: DespawnOnExit(ExampleStates::...) entities only
    // despawn when the inner state actually changes, and a later Sandbox
    // entry must start fresh in Editor, not resume a stale Scenario. The
    // DOCUMENT goes with it - it survives every inner state change so Play
    // can round-trip, but the session ends here (owner, 2026-08-25).
    app.add_systems(
        OnExit(GameStates::Playing),
        (
            |mut game_state: ResMut<NextState<ExampleStates>>| {
                game_state.set(ExampleStates::Loading);
            },
            teardown_document,
            apply_pending_mods,
        ),
    );

    app.add_systems(
        OnEnter(ExampleStates::Scenario),
        (
            setup_grab_cursor_scenario,
            |mut selection: ResMut<HeldPart>| {
                *selection = HeldPart(None);
            },
        ),
    );
    app.add_systems(
        OnEnter(ExampleStates::Editor),
        (
            // The document first: the rail is built for the ship it opens on,
            // and the views are hung off nodes that have to exist.
            ensure_document,
            setup_editor_scene,
            // Node entities survive the trip out to the scenario; their views do
            // not, so a second visit gives the same document new bodies. Play
            // spawns the ship either way - it reads the document, not the view.
            rebuild_node_views,
            setup_grab_cursor_editor,
            |mut selection: ResMut<HeldPart>| {
                *selection = HeldPart(None);
            },
        )
            .chain(),
    );
    app.add_systems(
        OnEnter(ExampleStates::Scenario),
        (
            // Sandbox-only: in NewGame the menu already loaded its scenario and a
            // second LoadScenario here would tear it straight back down.
            setup_scenario.run_if(resource_equals(GameMode::Sandbox)),
            |mut selection: ResMut<HeldPart>| {
                *selection = HeldPart(None);
            },
        ),
    );

    // Object nodes get their bodies here rather than beside the spawn, because
    // an object's body is a mesh the editor builds out of the asset stores (see
    // `sync_object_views`). Gated on those stores existing so a headless rig
    // with no pbr plugin is skipped rather than panicked, and NOT gated on the
    // gallery, so an object placed in one frame is on the stage in that frame.
    app.add_message::<ObjectBodyStale>();
    app.add_systems(
        Update,
        (drop_edited_views, sync_object_views)
            .chain()
            .before(sync_ship_focus)
            .run_if(in_state(ExampleStates::Editor))
            .run_if(resource_exists::<Assets<Mesh>>)
            .run_if(resource_exists::<Assets<StandardMaterial>>),
    );

    // The outward snapshot of everything below. `PostUpdate` so it reports the
    // frame that has just finished whichever system decided it, and unGATED
    // within the build that has it, so leaving the editor CLEARS it rather
    // than freezing the last build.
    //
    // Present only in a DEBUG build, because that is the only build anything
    // reads it in: the harness predicates live in `nova_debug`, which the same
    // feature links, and the editor ranges that assert off them are built with
    // it. The snapshot walks the whole document and clones what it finds, so a
    // shipped editor was paying a full sweep per frame - worst on the hulls
    // the Generate verb makes, which are one node with every section under it -
    // to fill a resource nothing in that build can see. A run condition is not
    // the tool: the snapshot reads fourteen sources, and a hand-written list of
    // them would go stale silently and freeze the harness's read surface.
    if cfg!(feature = "debug") {
        app.init_resource::<EditorProbe>();
        app.add_systems(PostUpdate, (sync_editor_probe, sync_shell_probe).chain());
    }

    app.init_resource::<EditorPanels>()
        .init_resource::<BrowserTabType>()
        .init_resource::<Problems>()
        .init_resource::<StatusLog>()
        .init_resource::<InspectorFolds>()
        .init_resource::<PendingRename>();
    app.add_observer(on_tree_caret);
    app.add_systems(OnEnter(ExampleStates::Editor), reset_problems);
    app.add_systems(OnExit(ExampleStates::Editor), reset_problems);
    // The shell's readouts. Each writes only on a change, and none of them
    // feeds the tree or the Inspector, so they need no order among themselves.
    app.add_systems(
        Update,
        (
            sync_panels,
            sync_toolbar,
            sync_ship_step,
            stage_enter_opens_ship.run_if(in_input_mode(InputMode::Normal)),
            sync_stage_overlay,
            sync_browser,
            (apply_asset_rename, sync_assets, sync_asset_thumbs).chain(),
            fit_viewed_images,
            sync_asset_controls,
            drive_thumbnails,
            asset_tile_keys.run_if(in_input_mode(InputMode::Normal)),
            (sync_problems, sync_problem_rows).chain(),
            (sync_log, sync_log_controls).chain(),
            (sync_copy_marks, sync_copy_controls).after(sync_log),
            sync_inspector_head,
            focus_pending_rename,
        )
            .run_if(in_state(ExampleStates::Editor)),
    );
    app.add_systems(
        Update,
        (sync_bench_view, sync_socket_views, draw_bench_sockets)
            .run_if(in_state(ExampleStates::Editor)),
    );
    // Ahead of everything that reads the view: the panels, the tabs and the
    // menus all show the view of the document that is open THIS frame.
    app.add_systems(
        Update,
        conform_view
            .before(sync_panels)
            .before(sync_view_tabs)
            .before(sync_menus)
            .run_if(in_state(ExampleStates::Editor)),
    );

    // Button colours, selection highlight, and the component tooltip.
    ui::register(app);
    ui::canvas::register(app);
    // The parts browser: its own state, overlay and 3D stage.
    gallery::register(app);
    // One click selects, two enter - shared by the Scene tree and the stage so
    // both count the same double.
    app.init_resource::<DoubleClick<Entity>>();
    app.add_observer(on_click_spaceship_section);

    // Dragging a ship or an object across the stage - the scenario node's one
    // transform gesture. The grab state is reset on entering the editor for the
    // same reason the rebind is: a drag cannot survive its views being rebuilt.
    app.init_resource::<StageDrag>();
    app.add_observer(on_stage_drag_start);
    app.add_observer(on_stage_drag);
    app.add_observer(on_stage_drag_end);
    app.add_systems(
        OnEnter(ExampleStates::Editor),
        |mut drag: ResMut<StageDrag>| *drag = StageDrag::default(),
    );

    // Framing: a key, a menu row and (below) a click on the tree all raise one
    // request, and one system serves it. The key is gated like the other single
    // letters - an F typed into an inspector field is not a camera gesture.
    app.init_resource::<FrameRequest>();
    app.init_resource::<CameraFraming>();
    app.add_systems(
        Update,
        frame_key
            .before(apply_frame_request)
            .run_if(in_input_mode(InputMode::Normal))
            .run_if(in_state(ExampleStates::Editor)),
    );
    // A request cannot outlive the visit that raised it: the way in frames the
    // context, and a stale request would move the camera straight back off it.
    app.add_systems(
        OnEnter(ExampleStates::Editor),
        |mut request: ResMut<FrameRequest>, mut framing: ResMut<CameraFraming>| {
            *request = FrameRequest::default();
            *framing = CameraFraming::default();
        },
    );

    // The move/turn handles on the selected node, and the mesh picking backend
    // that reaches them.
    gizmo::register(app);

    // The placement ghost: solve once per frame, then show it. Both are gated
    // on the gallery being closed - it covers the build area, so nothing under
    // it is being pointed at.
    app.init_resource::<PlacementPose>();
    app.init_resource::<PlacementPreview>();
    // The answer is REBUILT every frame the editor is up, and this half is
    // ungated: the solver below is skipped while the gallery covers the build
    // area, and the gallery CLOSES later in the same `Update` than the solver
    // would have run - so without an unconditional clear, that frame reports the
    // build view's last answer from before the overlay went up. See
    // `clear_placement_preview`.
    app.add_systems(
        Update,
        clear_placement_preview
            .before(update_placement_preview)
            .run_if(in_state(ExampleStates::Editor)),
    );
    app.add_systems(
        Update,
        (
            // Before the tool-chip reconciler, so a tool put down by leaving
            // the ship repaints in the same frame.
            disarm_outside_ship,
            sync_key_legend,
            // The node under the pointer, ahead of everything that lights it:
            // the rail's rows, and the mark the stage draws at the foot of
            // this chain.
            sync_hovered_node,
            // The menus: which one hangs open, and what its rows report about
            // the state they toggle.
            (
                sync_menus,
                sync_view_menu_marks,
                // The greying passes, then the paint that reads their verdict
                // off the rows.
                (sync_history_menu, sync_menu_verbs, tidy_menu_separators).chain(),
                sync_menu_item_paint,
            )
                .chain(),
            sync_ship_readout,
            sync_skin_toggle,
            // The rail's mark-per-row lists and what the part list adds up
            // to, grouped so the tuple stays under Bevy's arity limit.
            (
                sync_style_list,
                (
                    sync_part_ticks,
                    sync_part_zones,
                    // The only one here that is gated on more than a change: it
                    // formats a string off the whole draw list, and its line
                    // lives in a block that is shown only inside a ship.
                    sync_hull_plan.run_if(a_ship_is_entered),
                ),
            ),
            // After the field has taken the keystroke, so a seed is judged the
            // frame it is typed rather than the frame after - the widget's own
            // instruction to anything that reads what a field holds.
            read_seed_field.after(TextFieldSystems),
            // The tree, the breadcrumb, the panels, the two greyable buttons
            // and the stage focus all report the edit context, so they sit
            // together with the rest of the rail's readouts. An inner group
            // only because a flat tuple would pass Bevy's arity limit.
            (
                sync_view_tabs,
                // Before the tree and the panel: both are laid out inside the
                // widths this decides, and a rail that widens a frame after
                // its rows are built shows one frame of the old crop.
                sync_scene_list,
                // After the rows: a hint is positioned from the row it
                // describes, and a rebuilt list has no laid-out rows yet. The
                // row's own delete is revealed by the same pass, for the same
                // reason.
                (
                    sync_scene_tooltip,
                    sync_inspector_tooltip,
                    sync_row_trash,
                    paint_hovered_rows,
                ),
                // The panel, and then the floating windows: a window shows what
                // the row it was opened from shows, and closes when that row
                // goes away. One element, because the group around it is
                // already at Bevy's tuple arity.
                (
                    sync_inspector,
                    (
                        sync_colour_windows,
                        sync_pick_popups,
                        submit_pick_filter,
                        pick_filter_escape,
                        sync_reference_faults,
                    ),
                    paint_field_reasons,
                )
                    .chain(),
                paint_swatch_hover,
                sync_context_panels,
                sync_play_button,
                sync_status_line,
                sync_ship_focus,
                // BEFORE the framing below: the automatic row decides where a
                // minted ship stands, and a frame taken before it had moved
                // would look at the pose the ship is about to leave.
                reflow_auto_ships,
                sync_stage_frame,
                sync_camera_focus,
                // AFTER the context's own framing: both write the camera, and
                // a gesture that named a node beats the context having
                // changed in the same frame.
                apply_frame_request,
                // The gizmo rides the selection, so it is placed once the tree
                // above has settled what the selection IS.
                sync_gizmo,
            )
                .chain(),
            // Quadratic in the children of one node, and a generated hull
            // puts every section under one. Only a frame that moved an id or
            // a parent can have changed the answer.
            report_duplicate_ids.run_if(ids_or_parents_moved),
            // Both read single letters, which is also what a builder types
            // into an inspector field - so both are verbs, and Normal is where
            // verbs live. The rest of this chain draws, and drawing is not a
            // question about the keyboard.
            pick_section_under_pointer.run_if(in_input_mode(InputMode::Normal)),
            cycle_placement_pose.run_if(in_input_mode(InputMode::Normal)),
            update_placement_preview,
            // The founding click reads the same pointer state the solver does:
            // with an empty edited ship there is nothing to solve against, and
            // a click on clear space drops the first part at the ship origin.
            found_empty_ship,
            // AFTER the framing above: a plate is placed from the camera, so a
            // frame this frame would leave every plate one frame behind.
            sync_nameplates,
            // The ghost, then the verdict said where the part is: one solve,
            // drawn once as a box and once as words.
            (sync_placement_ghost, sync_placement_callout).chain(),
            // AFTER the ghost: the skin counts the part under the pointer
            // as structure, so it has to be derived from the same solve the
            // ghost on screen is showing.
            sync_editor_skin,
        )
            .chain()
            // BEFORE the gallery's keyboard, which shares keys with the build
            // view: Esc backs out of both, and Q takes a tile there and picks a
            // tool here. The gallery answers first because it is on top, and
            // running the build view ahead of it means these read the
            // gallery-open state the gesture was AIMED at rather than the one
            // it just changed - the frame Q closes the gallery is not also a
            // frame a build-view key fires on what the overlay was covering.
            .before(PartsGallerySystems)
            .run_if(in_state(ExampleStates::Editor).and_then(not(gallery_open))),
    );
    app.configure_sets(
        PostUpdate,
        EditorOverlaySystems
            .after(TransformSystems::Propagate)
            .run_if(in_state(ExampleStates::Editor).and_then(not(gallery_open))),
    );
    app.add_systems(
        PostUpdate,
        (
            draw_link_points,
            draw_ship_heading,
            draw_world_grid,
            draw_object_volumes,
            draw_node_marks,
            draw_axis_rose,
        )
            .in_set(EditorOverlaySystems),
    );
    app.init_resource::<EmptyPress>();
    app.init_resource::<StageRightPress>();
    app.init_resource::<EditorClipboard>();
    app.init_resource::<CopyRows>();
    #[cfg(not(target_arch = "wasm32"))]
    app.add_plugins(ui::package::PackageWindowPlugin);
    app.add_observer(on_stage_release);
    app.add_systems(OnEnter(ExampleStates::Editor), setup_selection_frame);
    app.add_systems(
        Update,
        (
            section_kind_keys.run_if(in_input_mode(InputMode::Browse)),
            sync_section_kind_tiles,
        )
            .chain()
            .run_if(in_state(ExampleStates::Editor)),
    );
    app.add_systems(
        OnEnter(ExampleStates::Editor),
        |mut tool: ResMut<StageTool>| *tool = StageTool::default(),
    );
    app.add_systems(
        Update,
        (
            // Over the Log or Problems list, Ctrl+A and Ctrl+C mean its rows.
            (
                tool_keys,
                select_all.run_if(not(copy_rows_hovered)),
                clipboard_keys.run_if(not(copy_rows_hovered)),
                row_copy_keys.run_if(copy_rows_hovered),
                panel_keys,
            )
                .run_if(in_input_mode(InputMode::Normal)),
            sync_scale_refusal,
            empty_space_press
                .after(sync_viewport_pointer)
                .run_if(resource_exists::<ButtonInput<MouseButton>>),
            sync_selection_frame.after(empty_space_press),
            stage_context_menu
                .after(sync_viewport_pointer)
                .run_if(resource_exists::<ButtonInput<MouseButton>>),
            add_menu_key
                .after(sync_viewport_pointer)
                .run_if(in_input_mode(InputMode::Normal)),
        )
            .before(sync_gizmo)
            .before(PartsGallerySystems)
            .run_if(in_state(ExampleStates::Editor).and_then(not(gallery_open))),
    );
    // AFTER every writer of the pose, so one frame that rolls and cycles at
    // once is still one detent. Unguarded by input mode: the two menu rows
    // turn the part from `Browse` too, and the tick belongs to the pose
    // moving, not to which control moved it. Both the ordering and the gate
    // sit on the SET, so a second editor cue inherits them by joining it.
    app.configure_sets(
        Update,
        cues::EditorCueSystems
            .after(cycle_placement_pose)
            .run_if(in_state(ExampleStates::Editor)),
    );
    app.add_systems(
        Update,
        cues::play_placement_pose_cue.in_set(cues::EditorCueSystems),
    );
    // A fresh visit starts with the part's first socket, unrolled - and the
    // detent watcher forgets with it, or the reset itself reads as a turn.
    app.init_resource::<cues::PlacementPoseHeard>();
    app.add_systems(
        OnEnter(ExampleStates::Editor),
        |mut pose: ResMut<PlacementPose>, mut heard: ResMut<cues::PlacementPoseHeard>| {
            *pose = PlacementPose::default();
            heard.0 = None;
        },
    );

    // A stale rebind must not survive a scene change, so clear it on
    // every state entry (like HeldPart).
    app.add_systems(
        OnEnter(ExampleStates::Editor),
        |mut rebind: ResMut<EditorRebind>| *rebind = EditorRebind::default(),
    );
    app.add_systems(
        OnEnter(ExampleStates::Scenario),
        |mut rebind: ResMut<EditorRebind>| *rebind = EditorRebind::default(),
    );
    // The capture is Bind's own system, so it answers under Bind and under
    // Normal - the click that arms it takes the mode in its own frame, and
    // Normal is what the frame before that was. Nothing is more exclusive than
    // Bind, so nothing takes the keyboard off it.
    app.add_systems(
        Update,
        (
            sync_section_keybind_labels,
            apply_section_rebind.run_if(owns_or_enters(InputMode::Bind)),
            position_section_keybind_labels.run_if(not(gallery_open)),
            // The gallery covers the ship the chips label, so they go off with
            // the rest of the editor's chrome while it is up.
            hide_section_keybind_labels.run_if(gallery_open),
        )
            .run_if(in_state(ExampleStates::Editor)),
    );

    // Floating windows. The picker's observer reads the dragged value out of
    // the event rather than off the slider, so it does not care whether
    // nova_ui's `slider_self_update` has committed it yet.
    // The question goes up before the verb runs, and comes down whichever way
    // it is answered. See `ui::window::DestructiveVerb`.
    app.add_observer(on_destructive_item);
    app.add_observer(close_confirm_window);
    // The naming, path and result windows go up on request, and any of their
    // answers takes them down. See `ui::files`.
    app.add_observer(close_file_window);

    // The inspector: what a typed field does to the document. Between the field
    // and the panel's repaint, with a sync point on each edge: Enter drops the
    // focus and reports the text in one frame, and the verdict is a COMMAND. A
    // repaint that landed in the gap saw a field with neither focus nor error
    // and painted the document value over the number the builder has to correct.
    app.add_systems(
        Update,
        apply_inspector_edits
            .after(nova_ui::prelude::TextFieldSystems)
            .before(sync_inspector)
            .run_if(in_state(ExampleStates::Editor)),
    );

    // The viewport camera. After the framing, because a gesture continues from
    // the pose a frame request left; before the gizmo, which sizes itself from
    // the eye. Ungated on the gallery and the pause so a gesture they interrupt
    // can end and hand the cursor back: the rig checks them itself.
    camera::register(app);
    app.add_systems(
        Update,
        (
            sync_viewport_pointer,
            drive_editor_camera
                .run_if(resource_exists::<ButtonInput<KeyCode>>)
                .run_if(resource_exists::<ButtonInput<MouseButton>>)
                .run_if(resource_exists::<AccumulatedMouseMotion>)
                .run_if(resource_exists::<Messages<MouseWheel>>),
        )
            .chain()
            .after(apply_frame_request)
            .before(sync_gizmo)
            .run_if(in_state(ExampleStates::Editor)),
    );
    app.add_systems(
        Update,
        // F1-to-editor is demo/sandbox furniture - campaigns (NewGame)
        // must not offer an editor escape; the pause menu is the sanctioned way
        // out.
        switch_scene_editor
            .run_if(in_state(ExampleStates::Scenario).and_then(resource_equals(GameMode::Sandbox))),
    );
    unload_on_the_way_to_the_editor(app);

    // The spaceship input/section system sets are deliberately NOT gated
    // here - nova_scenario's ScenarioLoaderPlugin gates them on
    // scenario-liveness. The editor's build-mode preview stays inert because the
    // Editor state never has a scenario loaded: initial entry loads nothing and
    // the way back unloads it (`unload_on_the_way_to_the_editor`).
}

/// Say whether the editor answers Escape itself this frame (see
/// [`EscapeOwner`]).
///
/// It does while there is something to back OUT of: an inspector field has the
/// keyboard, a block is in hand, a menu is open, the parts gallery is up, a part
/// is armed, a rebind is waiting for a key, or the editor is INSIDE a ship and
/// can step back out to the scenario. With none of those the key falls through
/// to the pause menu, which stays the sanctioned way out of the editor.
/// Written every frame, including the `false`: whoever claims the key also has
/// to release it.
fn declare_editor_escape_owner(
    editor: Res<State<ExampleStates>>,
    gallery: Res<GalleryState>,
    choice: Res<HeldPart>,
    rebind: Res<EditorRebind>,
    trail: Option<Res<DocumentTrail>>,
    menu: Res<OpenMenu>,
    drag: Res<CanvasDrag>,
    typing: Query<(), With<nova_ui::prelude::TextFieldFocused>>,
    popups: Query<(), With<PopupRoot>>,
    block_popup: Res<BlockPopup>,
    mut owner: ResMut<EscapeOwner>,
) {
    let owned = *editor.get() == ExampleStates::Editor
        && (!typing.is_empty()
            || !popups.is_empty()
            || block_popup.is_open()
            || menu.0.is_some()
            || drag.busy()
            || gallery.open
            || rebind.target.is_some()
            || *choice != HeldPart(None)
            || trail.is_some_and(|trail| trail.last().is_some()));
    if owner.0 != owned {
        owner.0 = owned;
    }
}

/// Claim the keyboard for the editor's rebind (see [`InputMode`]).
///
/// A rebind waiting for a key is entitled to every one of them. Written from
/// the state that DEFINES the mode rather than latched on entry, so a mode
/// that ends by having its state cleared - on a scene change, say - ends here
/// too. The gallery claims its own mode (`nova_ship::gallery`).
///
/// A block's pop-up holds Browse, so every verb gated on Normal stops under it
/// through this one claim: each acts on the document the pop-up is editing.
/// The New Section modal holds Browse for its arrows and Enter.
fn declare_editor_keyboard_owner(
    editor: Res<State<ExampleStates>>,
    rebind: Res<EditorRebind>,
    block_popup: Res<BlockPopup>,
    section_kind: Query<(), With<SectionKindModal>>,
    mut claims: MessageWriter<ClaimKeyboard>,
) {
    if *editor.get() != ExampleStates::Editor {
        return;
    }
    if rebind.target.is_some() {
        claims.write(ClaimKeyboard(InputMode::Bind));
    }
    if block_popup.is_open() || !section_kind.is_empty() {
        claims.write(ClaimKeyboard(InputMode::Browse));
    }
}

/// Backspace goes Back to the document this one was opened over, from
/// anywhere.
///
/// Escape ends at the same rung, but it is also the key that closes a menu,
/// drops the part in hand and cancels a rebind - so "take me back" is a press
/// whose effect depends on what else is open. This one does the one thing,
/// always.
///
/// Not while a field has the keyboard: there, Backspace is a character.
fn backspace_steps_out(mut commands: Commands, keys: Res<ButtonInput<KeyCode>>) {
    if keys.just_pressed(KeyCode::Backspace) {
        commands.queue(go_back);
    }
}

/// Escape backs out one step: it closes an open menu, then takes back an open
/// colour picker, then puts the armed part down, and with nothing in hand it
/// goes Back to the document this one was opened over.
///
/// ONE RUNG PER PRESS. The full ladder is: a block in hand, a popup, a block's
/// pop-up, an open top-bar menu, then the gallery, then a pending rebind, then
/// a colour picker, then the armed part, then Back, then the pause menu. This
/// system owns the middle of that ladder and nothing else: the rungs above it
/// belong to modes, and a mode holding the keyboard is a mode this never runs
/// under. A block's pop-up is such a mode: see [`escape_block_popup`].
fn escape_backs_out(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    popups: Query<Entity, With<PopupRoot>>,
    mut menu: ResMut<OpenMenu>,
    mut choice: ResMut<HeldPart>,
    mut drag: ResMut<CanvasDrag>,
    pickers: Query<Entity, With<ColourWindow>>,
) {
    if !keys.just_pressed(KeyCode::Escape) || drag.cancel() {
        return;
    }
    // A menu is drawn over everything else, so it is what the press is aimed
    // at while it is open. A popup stands over the panels the same way.
    if close_popups(&mut commands, &popups) || close_open_menu(&mut menu) {
        return;
    }
    // An open colour picker takes its colour back and closes.
    if let Some(picker) = pickers.iter().next() {
        commands.queue(move |world: &mut World| revert_picker(world, picker));
        return;
    }
    if *choice != HeldPart(None) {
        *choice = HeldPart(None);
        return;
    }
    commands.queue(go_back);
}

/// Esc under a block's pop-up: a list open on one of its rows goes first, then
/// the pop-up takes back what it did.
fn escape_block_popup(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    popups: Query<Entity, With<PopupRoot>>,
    mut block_popup: ResMut<BlockPopup>,
) {
    if keys.just_pressed(KeyCode::Escape)
        && block_popup.is_open()
        && !close_popups(&mut commands, &popups)
    {
        block_popup.cancel();
    }
}

fn switch_scene_editor(
    keys: Res<ButtonInput<KeyCode>>,
    gamepads: Query<&Gamepad>,
    mut state: ResMut<NextState<ExampleStates>>,
) {
    // Off the `Gamepad` COMPONENT: bevy 0.19 registers no
    // `ButtonInput<GamepadButton>` resource, so the `Option<Res<..>>` this used
    // to ask for was `None` on every real run and L3 never returned anybody to
    // the editor.
    let pad = gamepads
        .iter()
        .any(|pad| pad.digital().just_pressed(GamepadButton::LeftThumb));
    if keys.just_pressed(KeyCode::F1) || pad {
        debug!("switch_scene_editor: F1/L3 pressed, switching to Editor state.");
        state.set(ExampleStates::Editor);
    }
}

/// Unload the flown scenario on the way back to the editor.
///
/// On the transition and never from `Update`: an unload there applies between
/// the commands of that frame, and any insert a gameplay system queued behind
/// it panics on a despawned entity. F1 just after Play did exactly this.
fn unload_on_the_way_to_the_editor(app: &mut App) {
    app.add_systems(
        OnTransition {
            exited: ExampleStates::Scenario,
            entered: ExampleStates::Editor,
        },
        |mut commands: Commands| commands.trigger(UnloadScenario),
    );
}

/// Hide and lock the cursor for flight (owner decision): unconditional,
/// debug builds included. The F11 debug inspector is an egui
/// panel that needs a pointer, so it hands the cursor back while it is up via
/// nova_debug's `sync_inspector_cursor`; menus/pause/outcome free it through
/// their own transitions.
fn setup_grab_cursor_scenario(
    primary_cursor_options: Single<&mut CursorOptions, With<PrimaryWindow>>,
) {
    let mut primary_cursor_options = primary_cursor_options.into_inner();
    primary_cursor_options.grab_mode = CursorGrabMode::Locked;
    primary_cursor_options.visible = false;
}

fn setup_grab_cursor_editor(
    primary_cursor_options: Single<&mut CursorOptions, With<PrimaryWindow>>,
) {
    let mut primary_cursor_options = primary_cursor_options.into_inner();
    primary_cursor_options.grab_mode = CursorGrabMode::None;
    primary_cursor_options.visible = true;
}

#[cfg(test)]
mod tests {
    use bevy::{ecs::system::RunSystemOnce, state::app::StatesPlugin};

    use super::*;
    use crate::ui::menu::MenuId;

    /// Escape is the editor's only while it HAS a back step. With nothing armed
    /// and no gallery up the key falls through to the pause menu, which is the
    /// sanctioned way out of the editor - and in flight the pause menu owns it
    /// outright.
    #[test]
    fn the_editor_claims_escape_only_when_it_has_a_back_step() {
        let cases = [
            (ExampleStates::Editor, false, HeldPart(None), false, false),
            (ExampleStates::Editor, true, HeldPart(None), false, true),
            (
                ExampleStates::Editor,
                false,
                HeldPart(Some("hull".to_string())),
                false,
                true,
            ),
            // A rebind waiting for a key: Escape cancels it (see `keybind`).
            (ExampleStates::Editor, false, HeldPart(None), true, true),
            // Flying: the pause menu owns Escape whatever the editor left behind.
            (
                ExampleStates::Scenario,
                true,
                HeldPart(Some("hull".to_string())),
                true,
                false,
            ),
        ];

        for (state, gallery_open, choice, rebinding, expected) in cases {
            let mut world = World::new();
            world.insert_resource(State::new(state.clone()));
            world.insert_resource(GalleryState {
                open: gallery_open,
                ..default()
            });
            world.insert_resource(choice.clone());
            world.insert_resource(EditorRebind {
                target: rebinding.then_some(Entity::PLACEHOLDER),
                awaiting_release: false,
            });
            // Out in the scenario context, so the claim is decided by the three
            // things this case varies rather than by having a ship to leave.
            world.init_resource::<EditContext>();
            world.init_resource::<EscapeOwner>();
            world.init_resource::<OpenMenu>();
            world.init_resource::<CanvasDrag>();
            world.init_resource::<BlockPopup>();

            world
                .run_system_once(declare_editor_escape_owner)
                .expect("the claim system runs");

            assert_eq!(
                world.resource::<EscapeOwner>().0,
                expected,
                "{state:?} / gallery {gallery_open} / {choice:?} / rebinding {rebinding}"
            );
        }
    }

    /// The editor declares its rebind from the state that defines it, and
    /// declares nothing at all once it is out flying.
    #[test]
    fn the_editor_claims_the_keyboard_for_a_rebind() {
        let cases = [
            (ExampleStates::Editor, false, vec![]),
            (ExampleStates::Editor, true, vec![InputMode::Bind]),
            (ExampleStates::Scenario, true, vec![]),
        ];

        for (state, rebinding, expected) in cases {
            let mut world = World::new();
            world.insert_resource(State::new(state.clone()));
            world.insert_resource(EditorRebind {
                target: rebinding.then_some(Entity::PLACEHOLDER),
                awaiting_release: false,
            });
            world.init_resource::<BlockPopup>();
            world.init_resource::<Messages<ClaimKeyboard>>();

            world
                .run_system_once(declare_editor_keyboard_owner)
                .expect("the claim system runs");

            let claimed: Vec<InputMode> = world
                .resource_mut::<Messages<ClaimKeyboard>>()
                .drain()
                .map(|claim| claim.0)
                .collect();
            assert_eq!(claimed, expected, "{state:?} / rebinding {rebinding}");
        }
    }

    /// An open top-bar menu is a back step of its own, and the topmost one: it
    /// is drawn over everything else, so Escape has to close it rather than
    /// fall through to the pause menu behind it.
    #[test]
    fn an_open_menu_claims_escape() {
        let mut world = World::new();
        world.insert_resource(State::new(ExampleStates::Editor));
        world.init_resource::<GalleryState>();
        world.insert_resource(HeldPart(None));
        world.init_resource::<EditorRebind>();
        world.init_resource::<EditContext>();
        world.init_resource::<EscapeOwner>();
        world.init_resource::<OpenMenu>();
        world.init_resource::<CanvasDrag>();
        world.init_resource::<BlockPopup>();

        world
            .run_system_once(declare_editor_escape_owner)
            .expect("the claim system runs");
        assert!(
            !world.resource::<EscapeOwner>().0,
            "an idle editor lets the key through"
        );

        world.resource_mut::<OpenMenu>().0 = Some(MenuId::File);
        world
            .run_system_once(declare_editor_escape_owner)
            .expect("the claim system runs");
        assert!(world.resource::<EscapeOwner>().0);
    }

    /// The back gesture is a LADDER, one rung per press: an open menu closes,
    /// then a part in hand goes down, and only after that does Escape leave the
    /// ship. One press doing two would throw away two steps of context for one
    /// gesture.
    ///
    /// The rungs ABOVE these are not here because they are not this system's:
    /// a gallery or a rebind holding the keyboard means Escape never reaches
    /// it at all (`the_editor_claims_the_keyboard_for_a_rebind`, the gallery's own claim
    /// and `nova_ui::input_mode`).
    #[test]
    fn escape_puts_the_part_down_first_and_goes_back_second() {
        let mut app = crate::project::tests::project_app();
        let ship = crate::history::tests::first_with::<crate::node::ShipNode>(&mut app);
        app.world_mut().commands().queue(move |world: &mut World| {
            crate::project::ship::open_ship_node(world, ship, None)
        });
        app.update();
        let world = app.world_mut();
        let ship = world
            .resource::<EditContext>()
            .ship()
            .expect("the ship is open");
        let mut keys = ButtonInput::<KeyCode>::default();
        keys.press(KeyCode::Escape);
        world.insert_resource(keys);
        world.init_resource::<CanvasDrag>();
        world.insert_resource(HeldPart(Some("hull".to_string())));
        world.insert_resource(OpenMenu(Some(MenuId::File)));

        // The open menu first, drawn over the part in hand.
        world
            .run_system_once(escape_backs_out)
            .expect("the back-out system runs");
        assert_eq!(world.resource::<OpenMenu>().0, None);
        assert_eq!(
            *world.resource::<HeldPart>(),
            HeldPart(Some("hull".to_string())),
            "closing the menu did not also put the part down"
        );

        world
            .run_system_once(escape_backs_out)
            .expect("the back-out system runs");
        assert_eq!(*world.resource::<HeldPart>(), HeldPart(None));
        assert_eq!(
            world.resource::<EditContext>().ship(),
            Some(ship),
            "the first press only put the part down"
        );

        world
            .run_system_once(escape_backs_out)
            .expect("the back-out system runs");
        assert_eq!(
            world.resource::<EditContext>().ship(),
            None,
            "the second press goes back"
        );
        assert!(
            world
                .resource::<crate::project::ModProject>()
                .data
                .open
                .as_ref()
                .is_some_and(crate::project::OpenAsset::is_scenario),
            "to the scenario the ship was opened from"
        );
    }

    /// Counts LoadScenario triggers so the NewGame test can prove the editor
    /// stayed out of the menu's scenario load.
    #[derive(Resource, Default)]
    struct EditorScenarioLoads(usize);

    fn app() -> App {
        let mut app = App::new();
        app.add_plugins(StatesPlugin);
        app.init_state::<GameStates>();
        app.init_state::<PauseStates>();
        app.init_resource::<GameMode>();
        // switch_scene_editor polls the keyboard while in the Scenario state.
        app.init_resource::<ButtonInput<KeyCode>>();
        editor_plugin(&mut app);
        app.init_resource::<EditorScenarioLoads>();
        app.add_observer(
            |_: On<LoadScenario>, mut loads: ResMut<EditorScenarioLoads>| {
                loads.0 += 1;
            },
        );
        app
    }

    /// In NewGame mode the editor must still enter its Scenario state (cursor
    /// grab and the F1/despawn furniture key on it), while leaving the scenario
    /// load itself to the menu. (Flyability itself is not tied to this state:
    /// the spaceship sets are gated on scenario-liveness by nova_scenario.)
    #[test]
    fn new_game_enters_scenario_state_without_loading_the_editor_scenario() {
        let mut app = app();
        app.insert_resource(GameMode::NewGame);
        app.world_mut()
            .resource_mut::<NextState<GameStates>>()
            .set(GameStates::Playing);
        app.update();
        app.update();

        // Delivery guard: the handoff actually reached the Scenario state.
        assert_eq!(
            *app.world().resource::<State<ExampleStates>>().get(),
            ExampleStates::Scenario
        );
        // The editor did not fire its own sandbox scenario on top of the menu's.
        assert_eq!(app.world().resource::<EditorScenarioLoads>().0, 0);
    }

    /// Leaving Playing (the pause menu's Back to Main Menu) resets the
    /// editor's inner state so DespawnOnExit scene entities are torn down
    /// and the next Sandbox entry starts fresh.
    #[test]
    fn leaving_playing_resets_the_inner_state() {
        let mut app = app();
        // NewGame routes to Scenario, which applies safely headless (the
        // editor's own scenario load is Sandbox-gated).
        app.insert_resource(GameMode::NewGame);
        app.world_mut()
            .resource_mut::<NextState<GameStates>>()
            .set(GameStates::Playing);
        app.update();
        app.update();
        assert_eq!(
            *app.world().resource::<State<ExampleStates>>().get(),
            ExampleStates::Scenario
        );

        app.world_mut()
            .resource_mut::<NextState<GameStates>>()
            .set(GameStates::MainMenu);
        app.update();
        app.update();
        assert_eq!(
            *app.world().resource::<State<ExampleStates>>().get(),
            ExampleStates::Loading,
            "inner state must reset when Playing is left"
        );
    }

    /// Back to Main Menu ends the SESSION, and the document dies with it: the
    /// nodes survive every inner state change (Play round-trips them), so the
    /// one exit that must delete them is leaving Playing itself.
    #[test]
    fn leaving_playing_deletes_the_document() {
        let mut app = app();
        // NewGame routes to Scenario, which applies safely headless; the
        // document is fabricated directly, as an editor session leaves it.
        app.insert_resource(GameMode::NewGame);
        app.world_mut()
            .resource_mut::<NextState<GameStates>>()
            .set(GameStates::Playing);
        app.update();
        app.update();
        app.world_mut()
            .run_system_once(ensure_document)
            .expect("the document is created");
        assert!(app.world().resource::<EditContext>().scenario().is_some());

        app.world_mut()
            .resource_mut::<NextState<GameStates>>()
            .set(GameStates::MainMenu);
        app.update();
        app.update();

        assert_eq!(
            app.world_mut()
                .query::<&node::EditorNode>()
                .iter(app.world())
                .count(),
            0,
            "the document must not outlive the session"
        );
        assert!(
            app.world().resource::<EditContext>().path.is_empty(),
            "and nothing may keep pointing into it"
        );
    }

    /// F1 back-to-editor is Sandbox-only: in NewGame
    /// the same press must do nothing. Delivery guard: the identical press in
    /// Sandbox mode queues the Editor state and unloads the scenario, proving
    /// the stimulus path works.
    #[test]
    fn f1_returns_to_editor_only_in_sandbox_mode() {
        let make_app = app;
        // NewGame: F1 must be inert.
        let mut app = make_app();
        app.insert_resource(GameMode::NewGame);
        app.world_mut()
            .resource_mut::<NextState<GameStates>>()
            .set(GameStates::Playing);
        app.update();
        app.update();
        assert_eq!(
            *app.world().resource::<State<ExampleStates>>().get(),
            ExampleStates::Scenario
        );
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::F1);
        app.update();
        app.update();
        assert_eq!(
            *app.world().resource::<State<ExampleStates>>().get(),
            ExampleStates::Scenario,
            "F1 must not leave the scenario in NewGame"
        );
        assert_eq!(
            app.world().resource::<EditorScenarioLoads>().0,
            0,
            "no editor scenario churn in NewGame"
        );

        // Sandbox: the same press flips to Editor. Enter Playing via NewGame
        // (going through Editor would run setup_editor_scene, which needs
        // GameAssets headless), then flip the mode - the gate reads the
        // resource at press time. Assert the queued target without applying
        // it, for the same reason.
        let mut app = make_app();
        app.insert_resource(GameMode::NewGame);
        app.world_mut()
            .resource_mut::<NextState<GameStates>>()
            .set(GameStates::Playing);
        app.update();
        app.update();
        assert_eq!(
            *app.world().resource::<State<ExampleStates>>().get(),
            ExampleStates::Scenario
        );
        app.insert_resource(GameMode::Sandbox);
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::F1);
        app.update();
        let queued = match app.world().resource::<NextState<ExampleStates>>() {
            NextState::Pending(s) => Some(s.clone()),
            _ => None,
        };
        assert_eq!(
            queued,
            Some(ExampleStates::Editor),
            "the same press must work in Sandbox (delivery guard)"
        );
    }

    /// The scenario-liveness gate (nova_scenario)
    /// keeps the editor's build-mode preview inert only if the Editor state
    /// never has a live scenario. This exercises the one route that enters
    /// Editor FROM a live scenario - F1 - and asserts the press queues the way
    /// back and leaves the unload to the transition, with the editor firing no
    /// scenario load of its own anywhere on the route.
    #[test]
    fn editor_state_never_keeps_a_scenario_live() {
        #[derive(Resource, Default)]
        struct Unloads(usize);

        let mut app = app();
        app.init_resource::<Unloads>();
        app.add_observer(|_: On<UnloadScenario>, mut unloads: ResMut<Unloads>| {
            unloads.0 += 1;
        });

        // Enter Playing via NewGame (Editor's OnEnter scene setup needs
        // GameAssets headless), then flip to Sandbox so F1 is armed - the
        // gate reads the resource at press time.
        app.insert_resource(GameMode::NewGame);
        app.world_mut()
            .resource_mut::<NextState<GameStates>>()
            .set(GameStates::Playing);
        app.update();
        app.update();
        assert_eq!(
            *app.world().resource::<State<ExampleStates>>().get(),
            ExampleStates::Scenario
        );
        assert_eq!(app.world().resource::<Unloads>().0, 0);

        app.insert_resource(GameMode::Sandbox);
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::F1);
        app.update();
        let queued = match app.world().resource::<NextState<ExampleStates>>() {
            NextState::Pending(s) => Some(s.clone()),
            _ => None,
        };
        assert_eq!(
            queued,
            Some(ExampleStates::Editor),
            "delivery guard: the press was seen and Editor is queued"
        );
        assert_eq!(
            app.world().resource::<Unloads>().0,
            0,
            "an unload inside Update races the commands of that frame"
        );
        assert_eq!(
            app.world().resource::<EditorScenarioLoads>().0,
            0,
            "the editor fired no scenario load of its own on this route"
        );
    }

    /// The transition is what ends the flown scenario, and only the way back
    /// to the editor is that transition.
    #[test]
    fn the_way_back_to_the_editor_unloads_the_scenario() {
        #[derive(Resource, Default)]
        struct Unloads(usize);

        let mut app = App::new();
        app.add_plugins(StatesPlugin);
        app.init_state::<ExampleStates>();
        unload_on_the_way_to_the_editor(&mut app);
        app.init_resource::<Unloads>();
        app.add_observer(|_: On<UnloadScenario>, mut unloads: ResMut<Unloads>| {
            unloads.0 += 1;
        });
        app.world_mut()
            .resource_mut::<NextState<ExampleStates>>()
            .set(ExampleStates::Scenario);
        app.update();
        assert_eq!(app.world().resource::<Unloads>().0, 0);

        app.world_mut()
            .resource_mut::<NextState<ExampleStates>>()
            .set(ExampleStates::Editor);
        app.update();

        assert_eq!(app.world().resource::<Unloads>().0, 1);
    }

    /// Sandbox mode heads for the editor scene, exactly as before the menu. The
    /// full editor path (scene setup needs GameAssets) is covered end to end by
    /// the editor smoke run; this pins just the state routing.
    #[test]
    fn sandbox_heads_to_the_hub() {
        let mut app = app();
        app.insert_resource(GameMode::Sandbox);
        app.world_mut()
            .resource_mut::<NextState<GameStates>>()
            .set(GameStates::Playing);
        // A single transition step: entering Editor would run setup_editor_scene,
        // which needs GameAssets, so only assert the queued target.
        let queued = match app.world().resource::<NextState<ExampleStates>>() {
            NextState::Pending(s) => Some(s.clone()),
            _ => None,
        };
        assert_eq!(queued, None, "nothing queued before Playing is applied");
        app.world_mut()
            .run_schedule(bevy::state::state::StateTransition);
        let queued = match app.world().resource::<NextState<ExampleStates>>() {
            NextState::Pending(s) => Some(s.clone()),
            _ => None,
        };
        assert_eq!(queued, Some(ExampleStates::Hub));
    }

    /// Flight hides and locks the cursor. Before the fix
    /// `setup_grab_cursor_scenario` was wrapped in `cfg!(not(feature =
    /// "debug"))`, so a `--features dev` build (the standard playtest build)
    /// left the cursor visible the whole flight; the grab is unconditional now.
    #[test]
    fn scenario_grab_hides_and_locks_the_cursor() {
        let mut app = App::new();
        app.world_mut().spawn((
            PrimaryWindow,
            CursorOptions {
                visible: true,
                grab_mode: CursorGrabMode::None,
                ..default()
            },
        ));
        app.add_systems(Update, setup_grab_cursor_scenario);
        app.update();

        let cursor = app
            .world_mut()
            .query_filtered::<&CursorOptions, With<PrimaryWindow>>()
            .single(app.world())
            .unwrap();
        assert!(!cursor.visible, "cursor must be hidden while flying");
        assert_eq!(
            cursor.grab_mode,
            CursorGrabMode::Locked,
            "cursor must be locked (captured) while flying"
        );
    }
}
