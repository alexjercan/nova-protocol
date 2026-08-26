//! The spaceship editor: a scene where you build a ship out of sections and then
//! hand it off to a scenario simulation.
//!
//! Structure:
//! - `attitude`  - what the hull under construction would turn like
//! - `node`      - the document: the node tree and the edit context
//! - `inspect`   - the reflected fields of the inspected node, and their write-back
//! - `frame`     - putting the camera on a node, on demand
//! - `gizmo`     - the handles that move and turn the selected node
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
//! - `scenario`  - the default world a document is seeded with, and the sandbox script
//! - `bundle`    - the document as a saved mod bundle, and the read back out of one
//! - `ui`        - the wiki-style rail + component drawer + tooltip
//! - `probe`     - the one public, read-only snapshot of all of the above
#![warn(missing_docs)]

use bevy::{
    gizmos::GizmoPlugin,
    input::mouse::MouseWheel,
    prelude::*,
    window::{CursorGrabMode, CursorOptions, PrimaryWindow},
};
use nova_assets::prelude::{GameAssets, GameAssetsStates};
use nova_gameplay::prelude::*;
use nova_scenario::prelude::*;

mod attitude;
mod bundle;
mod config;
mod frame;
mod gallery;
mod gizmo;
mod glyph;
mod highlight;
mod inspect;
mod keybind;
mod node;
mod placement;
mod preview;
mod probe;
mod scenario;
mod skin;
mod snap;
mod stage;
mod ui;

use attitude::sync_attitude_readout;
use bundle::{apply_file_request, save_key, FileRequest};
use config::{
    editor_gizmo_config, EditorGizmos, EditorOverlays, EditorStatus, HoveredNode, LastClick,
    PlacementPose, PlacementPreview, SectionChoice, SelectedNode,
};
use frame::{apply_frame_request, frame_key, sync_frame_item, FrameRequest};
use gizmo::sync_gizmo;
use highlight::{paint_hovered_rows, sync_hovered_node};
use keybind::{
    apply_section_rebind, hide_section_keybind_labels, position_section_keybind_labels,
    sync_section_keybind_labels, EditorRebind,
};
use node::{
    drop_edited_views, ensure_document, rebuild_node_views, report_duplicate_ids,
    sync_camera_focus, sync_object_views, sync_ship_focus, teardown_document, EditContext,
};
use placement::{
    clear_placement_preview, cycle_placement_pose, delete_key, disarm_outside_ship,
    draw_link_points, draw_ship_heading, found_empty_ship, on_click_spaceship_section,
    on_stage_drag, on_stage_drag_end, on_stage_drag_start, pick_section_under_pointer,
    sync_placement_ghost, update_placement_preview, wheel_placement_pose, StageDrag,
};
use probe::sync_editor_probe;
pub use probe::{EditorPlacement, EditorProbe, EditorSection, EditorTool};
use scenario::{register_sandbox_scenario, sandbox_unregistered, setup_scenario};
use skin::sync_editor_skin;
use stage::{draw_node_marks, draw_object_volumes, draw_world_grid};
use ui::{
    callout::sync_placement_callout,
    inspector::{
        apply_inspector_edits, hold_camera_while_typing, paint_field_reasons, sync_inspector,
        typing_into_a_field,
    },
    menu::{
        close_menu_on_item, close_menus, close_open_menu, sync_armed_menu, sync_menu_delete,
        sync_menu_item_paint, sync_menus, sync_scenario_menu, sync_ship_menu, sync_view_menu_marks,
        OpenMenu,
    },
    rail::sync_scene_tooltip,
    setup_editor_scene, sync_breadcrumb, sync_context_panels, sync_key_legend, sync_play_button,
    sync_rebind_button, sync_row_trash, sync_scene_list, sync_skin_toggle, sync_status_line,
    sync_style_list,
    window::{close_confirm_window, on_colour_slider, on_destructive_item, sync_colour_windows},
};

/// Glob-import surface: `use nova_editor::prelude::*` brings [`NovaEditorPlugin`],
/// the sandbox registration ordering handle and the read-only [`EditorProbe`]
/// snapshot into scope.
pub mod prelude {
    pub use super::{
        EditorPlacement, EditorProbe, EditorSandboxSystems, EditorSection, EditorTool,
        NovaEditorPlugin,
    };
}

/// Ordering handle for the editor sandbox's registration into `GameScenarios`.
///
/// Exists because `nova_core`'s startup-scenario handoff resolves an id against
/// that registry in the same `OnEnter(GameAssetsStates::Loaded)` transition, and
/// the sandbox is the one scenario no content file publishes.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct EditorSandboxSystems;

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
    Editor,
    Scenario,
}

fn editor_plugin(app: &mut App) {
    app.init_state::<ExampleStates>();
    // Everything the editor draws in immediate mode, at one weight. See
    // `EditorGizmos`. Gated like the picking backend below: registering a
    // group adds the render-side system that turns its storage into meshes,
    // and a headless app has neither the plugin nor the assets it wants.
    if app.is_plugin_added::<GizmoPlugin>() {
        app.insert_gizmo_config(EditorGizmos, editor_gizmo_config());
    }
    app.insert_resource(SectionChoice::None);
    app.init_resource::<EditContext>();
    app.init_resource::<SelectedNode>();
    app.init_resource::<HoveredNode>();
    app.init_resource::<EditorRebind>();
    app.init_resource::<EditorOverlays>();
    // The one line the editor speaks through - the placement readout and every
    // verb that has something to say both write it. See `EditorStatus`.
    app.init_resource::<EditorStatus>();
    // Save and open: what the File menu and Ctrl+S ask for, and the one worker
    // that answers. Editor-only, because a document only exists in there.
    app.init_resource::<FileRequest>();
    app.add_systems(
        Update,
        (
            // Ctrl+S is a modifier and a letter, and the letter is one a
            // builder types into an inspector field. See `typing_into_a_field`.
            save_key.run_if(not(typing_into_a_field)),
            apply_file_request,
        )
            .chain()
            .run_if(in_state(ExampleStates::Editor)),
    );
    // Del removes what is marked, at any depth. Same guard as Ctrl+S: a name
    // being typed into an Inspector field takes Delete with it.
    app.add_systems(
        Update,
        delete_key
            .run_if(not(typing_into_a_field))
            .run_if(in_state(ExampleStates::Editor)),
    );
    // The top bar's menus. Closed on entering the editor because the bar they
    // hang off is `DespawnOnExit(Editor)`: an "open" menu on the way back in
    // would be a dropdown with no button under it.
    app.init_resource::<OpenMenu>();
    app.add_observer(close_menu_on_item);
    app.add_systems(OnEnter(ExampleStates::Editor), close_menus);
    // Normally the gameplay plugin's; init'd here too so a menu-less rig (and
    // the tests below) still has one to write.
    app.init_resource::<EscapeOwner>();

    // The sandbox is registered by id at load, like every shipped scenario,
    // and repaired whenever the bundle merge replaces the registry: a live
    // re-merge (an enabled mod changing) rebuilds `GameScenarios` from content
    // alone, and this entry has no content file to be rebuilt from.
    // `PostUpdate` on purpose - the re-merge runs in `Update`, and the next
    // frame's state transitions must not see the id missing.
    app.add_systems(
        OnEnter(GameAssetsStates::Loaded),
        register_sandbox_scenario
            .in_set(EditorSandboxSystems)
            .run_if(resource_exists::<GameAssets>)
            .run_if(resource_exists::<GameScenarios>),
    );
    app.add_systems(
        PostUpdate,
        register_sandbox_scenario
            .in_set(EditorSandboxSystems)
            .run_if(resource_exists::<GameAssets>)
            .run_if(resource_exists::<GameScenarios>)
            .run_if(sandbox_unregistered),
    );

    // Escape is a BACK key in the editor and a pause key only when there is
    // nothing to back out of. `PreUpdate` on purpose: `nova_menu`'s pause
    // toggle reads the answer in `Update`, and the two crates cannot order
    // against each other.
    app.add_systems(PreUpdate, declare_editor_escape_owner);
    app.add_systems(
        Update,
        escape_backs_out
            // Same reason as the placement chain below: the Escape that closes
            // the gallery must not also put down the part it was holding.
            .before(gallery::gallery_keyboard)
            // And the Escape that cancels a rebind must not also leave the ship.
            // Explicit, because this reads the target that system clears: run
            // the other way round and the guard sees a rebind already gone.
            .before(apply_section_rebind)
            // And Escape in a field is the field's: it puts back what was
            // there, which is one rung of its own.
            .run_if(not(typing_into_a_field))
            .run_if(in_state(ExampleStates::Editor).and_then(not(gallery::gallery_open))),
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
            |mode: Res<GameMode>, mut game_state: ResMut<NextState<ExampleStates>>| {
                game_state.set(match *mode {
                    GameMode::Sandbox => ExampleStates::Editor,
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
        ),
    );

    app.add_systems(
        OnEnter(ExampleStates::Scenario),
        (
            setup_grab_cursor_scenario,
            |mut selection: ResMut<SectionChoice>| {
                *selection = SectionChoice::None;
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
            |mut selection: ResMut<SectionChoice>| {
                *selection = SectionChoice::None;
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
            |mut selection: ResMut<SectionChoice>| {
                *selection = SectionChoice::None;
            },
        ),
    );

    // Object nodes get their bodies here rather than beside the spawn, because
    // an object's body is a mesh the editor builds out of the asset stores (see
    // `sync_object_views`). Gated on those stores existing so a headless rig
    // with no pbr plugin is skipped rather than panicked, and NOT gated on the
    // gallery, so an object placed in one frame is on the stage in that frame.
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
    // frame that has just finished whichever system decided it, and ungated so
    // leaving the editor CLEARS it rather than freezing the last build.
    app.init_resource::<EditorProbe>();
    app.add_systems(PostUpdate, sync_editor_probe);

    // Button colours, selection highlight, and the component tooltip.
    ui::register(app);
    // The parts browser: its own state, overlay and 3D stage.
    gallery::register(app);
    // One click selects, two enter - shared by the Scene tree and the stage so
    // both count the same double.
    app.init_resource::<LastClick>();
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
    app.add_systems(
        Update,
        frame_key
            .before(apply_frame_request)
            .run_if(not(typing_into_a_field))
            .run_if(in_state(ExampleStates::Editor).and_then(not(gallery::gallery_open))),
    );
    // A request cannot outlive the visit that raised it: the way in frames the
    // context, and a stale request would move the camera straight back off it.
    app.add_systems(
        OnEnter(ExampleStates::Editor),
        |mut request: ResMut<FrameRequest>| *request = FrameRequest::default(),
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
                (
                    sync_menu_delete,
                    sync_ship_menu,
                    sync_scenario_menu,
                    sync_armed_menu,
                    sync_frame_item,
                )
                    .chain(),
                sync_menu_item_paint,
            )
                .chain(),
            sync_attitude_readout,
            sync_skin_toggle,
            sync_style_list,
            // The tree, the breadcrumb, the panels, the two greyable buttons
            // and the stage focus all report the edit context, so they sit
            // together with the rest of the rail's readouts. An inner group
            // only because a flat tuple would pass Bevy's arity limit.
            (
                sync_scene_list,
                // After the rows: a hint is positioned from the row it
                // describes, and a rebuilt list has no laid-out rows yet. The
                // row's own delete is revealed by the same pass, for the same
                // reason.
                (sync_scene_tooltip, sync_row_trash, paint_hovered_rows),
                // The panel, and then the floating windows: a window shows what
                // the row it was opened from shows, and closes when that row
                // goes away. One element, because the group around it is
                // already at Bevy's tuple arity.
                (sync_inspector, sync_colour_windows, paint_field_reasons).chain(),
                sync_context_panels,
                sync_breadcrumb,
                sync_rebind_button,
                sync_play_button,
                sync_status_line,
                sync_ship_focus,
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
            report_duplicate_ids,
            // Both read single letters, which is also what a builder types
            // into an inspector field. See `typing_into_a_field`.
            pick_section_under_pointer.run_if(not(typing_into_a_field)),
            cycle_placement_pose.run_if(not(typing_into_a_field)),
            update_placement_preview,
            // The founding click reads the same pointer state the solver does:
            // with an empty edited ship there is nothing to solve against, and
            // a click on clear space drops the first part at the ship origin.
            found_empty_ship,
            // The ghost, then the verdict said where the part is: one solve,
            // drawn once as a box and once as words.
            (sync_placement_ghost, sync_placement_callout).chain(),
            // AFTER the ghost: the skin counts the part under the pointer
            // as structure, so it has to be derived from the same solve the
            // ghost on screen is showing.
            sync_editor_skin,
            draw_link_points,
            draw_ship_heading,
            draw_world_grid,
            draw_object_volumes,
            draw_node_marks,
        )
            .chain()
            // BEFORE the gallery's keyboard, which shares two keys with the
            // build view: Q takes a part in both, and Esc backs out of both. The
            // gallery answers first because it is on top, and running the build
            // view ahead of it means these read the gallery-open state the
            // gesture was AIMED at rather than the one it just changed - the
            // frame Q closes the gallery is not also a frame the pipette fires
            // on whatever the overlay was covering.
            .before(gallery::gallery_keyboard)
            .run_if(in_state(ExampleStates::Editor).and_then(not(gallery::gallery_open))),
    );
    // The wheel half of the pose control, split off so a headless rig with no
    // input plugin (and so no wheel message queue) still runs the rest.
    app.add_systems(
        Update,
        wheel_placement_pose
            .before(update_placement_preview)
            .run_if(not(typing_into_a_field))
            .run_if(resource_exists::<Messages<MouseWheel>>)
            .run_if(in_state(ExampleStates::Editor).and_then(not(gallery::gallery_open))),
    );
    // A fresh visit starts with the part's first socket, unrolled.
    app.add_systems(
        OnEnter(ExampleStates::Editor),
        |mut pose: ResMut<PlacementPose>| *pose = PlacementPose::default(),
    );

    // NOTE: a stale rebind must not survive a scene change, so clear it on
    // every state entry (like SectionChoice).
    app.add_systems(
        OnEnter(ExampleStates::Editor),
        |mut rebind: ResMut<EditorRebind>| *rebind = EditorRebind::default(),
    );
    app.add_systems(
        OnEnter(ExampleStates::Scenario),
        |mut rebind: ResMut<EditorRebind>| *rebind = EditorRebind::default(),
    );
    // NOTE: the rebind capture and the right-drag cursor grab are gated on the
    // gallery being CLOSED - while the overlay is up it owns the keyboard (a
    // filter keystroke would otherwise be captured as a section binding) and
    // the pointer.
    app.add_systems(
        Update,
        (
            sync_section_keybind_labels,
            apply_section_rebind
                .run_if(not(gallery::gallery_open))
                .run_if(not(typing_into_a_field)),
            position_section_keybind_labels.run_if(not(gallery::gallery_open)),
            // The gallery covers the ship the chips label, so they go off with
            // the rest of the editor's chrome while it is up.
            hide_section_keybind_labels.run_if(gallery::gallery_open),
        )
            .run_if(in_state(ExampleStates::Editor)),
    );

    // Floating windows. The picker's observer reads the dragged value out of
    // the event rather than off the slider, so it does not care whether
    // nova_ui's `slider_self_update` has committed it yet.
    app.add_observer(on_colour_slider);
    // The question goes up before the verb runs, and comes down whichever way
    // it is answered. See `ui::window::DestructiveVerb`.
    app.add_observer(on_destructive_item);
    app.add_observer(close_confirm_window);

    // The inspector: what a typed field does to the document, and the camera
    // rig it borrows while the field has the keyboard. Ungated on the gallery
    // because opening one drops the focus anyway (the click that opens it
    // submits the field), and a hold that outlived its trigger would leave the
    // camera dead.
    app.add_systems(
        Update,
        (
            // Between the field and the panel's repaint, with a sync point on
            // each edge: Enter drops the focus and reports the text in one
            // frame, and the verdict is a COMMAND. A repaint that landed in
            // the gap saw a field with neither focus nor error and painted the
            // document value over the number the builder has to correct.
            apply_inspector_edits
                .after(nova_ui::prelude::TextFieldSystems)
                .before(sync_inspector),
            hold_camera_while_typing,
        )
            .run_if(in_state(ExampleStates::Editor)),
    );

    app.add_systems(
        Update,
        lock_on_left_click
            .run_if(in_state(ExampleStates::Editor).and_then(in_state(PauseStates::Unpaused)))
            .run_if(not(gallery::gallery_open)),
    );
    app.add_systems(
        Update,
        // NOTE: F1-to-editor is demo/sandbox furniture - campaigns (NewGame)
        // must not offer an editor escape; the pause menu is the sanctioned way
        // out.
        switch_scene_editor
            .run_if(in_state(ExampleStates::Scenario).and_then(resource_equals(GameMode::Sandbox))),
    );

    // NOTE: the spaceship input/section system sets are deliberately NOT gated
    // here - nova_scenario's ScenarioLoaderPlugin gates them on
    // scenario-liveness. The editor's build-mode preview stays inert because the
    // Editor state never has a scenario loaded: initial entry loads nothing and
    // F1 triggers UnloadScenario.
}

/// Say whether the editor answers Escape itself this frame (see
/// [`EscapeOwner`]).
///
/// It does while there is something to back OUT of: an inspector field has the
/// keyboard, a menu is open, the parts gallery is up, a part is armed, a rebind
/// is waiting for a key, or the editor is INSIDE a ship and can step back out to
/// the scenario. With none of those the key falls through to the pause menu,
/// which stays the sanctioned way out of the editor.
/// Written every frame, including the `false`: whoever claims the key also has
/// to release it.
fn declare_editor_escape_owner(
    editor: Res<State<ExampleStates>>,
    gallery: Res<gallery::GalleryState>,
    choice: Res<SectionChoice>,
    rebind: Res<EditorRebind>,
    context: Res<EditContext>,
    menu: Res<OpenMenu>,
    typing: Query<(), With<nova_ui::prelude::TextFieldFocused>>,
    mut owner: ResMut<EscapeOwner>,
) {
    let owned = *editor.get() == ExampleStates::Editor
        && (!typing.is_empty()
            || menu.0.is_some()
            || gallery.open
            || rebind.target.is_some()
            || *choice != SectionChoice::None
            || context.ship().is_some());
    if owner.0 != owned {
        owner.0 = owned;
    }
}

/// Escape backs out one step: it puts the armed part down, and with nothing in
/// hand it leaves the ship you are inside.
///
/// ONE RUNG PER PRESS. The full ladder is: an open top-bar menu, then the
/// gallery (which answers its own Escape while it is up, see `gallery::input`),
/// then a pending rebind (which `keybind::apply_section_rebind` cancels), then
/// the armed part, then the edit context, then the pause menu. The two rungs
/// this system does not own are the two it has to check for itself - the
/// gallery through a run condition, the rebind here - because both of those
/// cancel in the SAME frame this reads the key, not before it.
fn escape_backs_out(
    keys: Res<ButtonInput<KeyCode>>,
    // Read before `apply_section_rebind` consumes it - see the ordering at the
    // registration. A rebind cancelled and a ship left on one press is two rungs
    // of context thrown away for one gesture.
    rebind: Res<EditorRebind>,
    mut menu: ResMut<OpenMenu>,
    mut choice: ResMut<SectionChoice>,
    mut context: ResMut<EditContext>,
) {
    if !keys.just_pressed(KeyCode::Escape) || rebind.target.is_some() {
        return;
    }
    // The menu is drawn over everything else, so it is what the press is aimed
    // at while it is open.
    if close_open_menu(&mut menu) {
        return;
    }
    if *choice != SectionChoice::None {
        *choice = SectionChoice::None;
        return;
    }
    context.exit();
}

fn switch_scene_editor(
    keys: Res<ButtonInput<KeyCode>>,
    gamepad: Option<Res<ButtonInput<GamepadButton>>>,
    mut state: ResMut<NextState<ExampleStates>>,
    mut commands: Commands,
) {
    let pad = gamepad
        .map(|g| g.just_pressed(GamepadButton::LeftThumb))
        .unwrap_or(false);
    if keys.just_pressed(KeyCode::F1) || pad {
        debug!("switch_scene_editor: F1/L3 pressed, switching to Editor state.");
        state.set(ExampleStates::Editor);
        commands.trigger(UnloadScenario);
    }
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

fn lock_on_left_click(
    primary_cursor_options: Single<&mut CursorOptions, With<PrimaryWindow>>,
    mouse: Res<ButtonInput<MouseButton>>,
) {
    if mouse.just_pressed(MouseButton::Right) {
        let mut primary_cursor_options = primary_cursor_options.into_inner();
        primary_cursor_options.grab_mode = CursorGrabMode::Locked;
        primary_cursor_options.visible = false;
    } else if mouse.just_released(MouseButton::Right) {
        let mut primary_cursor_options = primary_cursor_options.into_inner();
        primary_cursor_options.grab_mode = CursorGrabMode::None;
        primary_cursor_options.visible = true;
    }
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
            (
                ExampleStates::Editor,
                false,
                SectionChoice::None,
                false,
                false,
            ),
            (
                ExampleStates::Editor,
                true,
                SectionChoice::None,
                false,
                true,
            ),
            (
                ExampleStates::Editor,
                false,
                SectionChoice::Section("hull".to_string()),
                false,
                true,
            ),
            // A rebind waiting for a key: Escape cancels it (see `keybind`).
            (
                ExampleStates::Editor,
                false,
                SectionChoice::None,
                true,
                true,
            ),
            // Flying: the pause menu owns Escape whatever the editor left behind.
            (
                ExampleStates::Scenario,
                true,
                SectionChoice::Section("hull".to_string()),
                true,
                false,
            ),
        ];

        for (state, gallery_open, choice, rebinding, expected) in cases {
            let mut world = World::new();
            world.insert_resource(State::new(state.clone()));
            world.insert_resource(gallery::GalleryState {
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

    /// An open top-bar menu is a back step of its own, and the topmost one: it
    /// is drawn over everything else, so Escape has to close it rather than
    /// fall through to the pause menu behind it.
    #[test]
    fn an_open_menu_claims_escape() {
        let mut world = World::new();
        world.insert_resource(State::new(ExampleStates::Editor));
        world.init_resource::<gallery::GalleryState>();
        world.insert_resource(SectionChoice::None);
        world.init_resource::<EditorRebind>();
        world.init_resource::<EditContext>();
        world.init_resource::<EscapeOwner>();
        world.init_resource::<OpenMenu>();

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

    /// The back gesture is a LADDER, one rung per press: a pending rebind owns
    /// the press, then a part in hand goes down, and only after that does
    /// Escape leave the ship. One press doing two would throw away two steps of
    /// context for one gesture.
    #[test]
    fn escape_puts_the_part_down_first_and_leaves_the_ship_second() {
        let scenario = Entity::from_raw_u32(1).expect("a test entity id");
        let ship = Entity::from_raw_u32(2).expect("a test entity id");

        let mut world = World::new();
        let mut keys = ButtonInput::<KeyCode>::default();
        keys.press(KeyCode::Escape);
        world.insert_resource(keys);
        world.insert_resource(SectionChoice::Section("hull".to_string()));
        world.init_resource::<EditorRebind>();
        world.insert_resource(OpenMenu(Some(MenuId::File)));
        world.insert_resource(EditContext {
            path: vec![scenario, ship],
        });

        // A pending rebind owns the press outright: `apply_section_rebind`
        // cancels it in this same frame, so backing out here as well would
        // spend two rungs on one gesture.
        world.resource_mut::<EditorRebind>().target = Some(Entity::PLACEHOLDER);
        world
            .run_system_once(escape_backs_out)
            .expect("the back-out system runs");
        assert_eq!(
            *world.resource::<SectionChoice>(),
            SectionChoice::Section("hull".to_string()),
            "the rebind takes the press; the part stays in hand"
        );
        assert_eq!(world.resource::<EditContext>().ship(), Some(ship));
        world.resource_mut::<EditorRebind>().target = None;

        // Then the open menu, which is drawn over the part in hand.
        world
            .run_system_once(escape_backs_out)
            .expect("the back-out system runs");
        assert_eq!(world.resource::<OpenMenu>().0, None);
        assert_eq!(
            *world.resource::<SectionChoice>(),
            SectionChoice::Section("hull".to_string()),
            "closing the menu did not also put the part down"
        );

        world
            .run_system_once(escape_backs_out)
            .expect("the back-out system runs");
        assert_eq!(*world.resource::<SectionChoice>(), SectionChoice::None);
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
            "the second press leaves the ship"
        );
        assert_eq!(
            world.resource::<EditContext>().scenario(),
            Some(scenario),
            "and lands in the scenario context, not outside the document"
        );
    }

    /// Counts LoadScenario triggers so the NewGame test can prove the editor
    /// stayed out of the menu's scenario load (review R1.1).
    #[derive(Resource, Default)]
    struct EditorScenarioLoads(usize);

    fn app() -> App {
        let mut app = App::new();
        app.add_plugins(StatesPlugin);
        app.init_state::<GameStates>();
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
    /// Editor FROM a live scenario - F1 - and asserts the same press
    /// unloads it, with the editor firing no scenario load of its own
    /// anywhere on the route.
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
            1,
            "the same press must unload the scenario"
        );
        assert_eq!(
            app.world().resource::<EditorScenarioLoads>().0,
            0,
            "the editor fired no scenario load of its own on this route"
        );
    }

    /// Sandbox mode heads for the editor scene, exactly as before the menu. The
    /// full editor path (scene setup needs GameAssets) is covered end to end by
    /// the editor smoke run; this pins just the state routing.
    #[test]
    fn sandbox_heads_to_editor_state() {
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
        assert_eq!(queued, Some(ExampleStates::Editor));
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
