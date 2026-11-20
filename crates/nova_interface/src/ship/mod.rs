//! The TAB interface's Ship pane: a schematic 3D viewer of the player ship.
//!
//! Every section is addressed by a short [`SectionCode`] (`HULL-3`, `PDC-1`,
//! `TRB-1`), assigned stably per session from the section kind + a stable
//! index - the real ships use auto grid-coord `EntityId`s (`engine_port`,
//! `fuselage`) that can be long, so the code is the label handle.
//!
//! The viewer follows the Map pane pattern: a dedicated [`Camera3d`] on its own
//! `RenderLayers` renders proxy BLOCKS (built from each section's authored
//! [`SectionCollider`](nova_ship::prelude::SectionCollider) + local transform)
//! into an offscreen image shown in the pane. Each block is filled in its
//! section family's tint and wrapped in a box outline (a `cuboid_edges`
//! wireframe) with a gap, so adjacent sections read apart; an arrow past the
//! foremost block marks the bow. The interactive SECTIONS ride on top as
//! projected clickable badges: the family icon, a status pip and the code.
//! The section panel beside the view shows the selection's code, name and
//! condition bar over kit blocks (`nova_ui::widget::layout`), each a heading,
//! a body and its verbs at the bottom right: Control with the binding and
//! Rebind, Repair with the plate amount row and Repair, the running Repairs,
//! Distress, the Lost sections and Salvage with Delete. A block whose verb
//! cannot work here hides and takes no space; a verb short of a resource is
//! disabled with the short number in Danger; another cause tags it with one
//! word ([`VerbGate`](nova_ui::prelude::VerbGate)). Orbit the camera as the
//! editor's looks, with the right-button drag
//! (the pad's D-pad) + wheel; `Q` picks the part of the section under the
//! pointer; `[`/`]` cycle the selection; Fit frames the whole ship at the
//! current angles, and Reset also restores the opening angles; `G` toggles
//! structural mates; `P` repairs, `B` replaces the selected bindable
//! section's input, and Delete takes the selected section off while docked
//! at a station that refits, its parts back in the hold. The Salvage block
//! lists what it gives back over the Delete button and hides away from a
//! refit; a delete that would split the ship disables Delete and fills and
//! outlines the sections that would lose the dock in red. While the repair
//! amount field has the caret, these keys type into it instead. The footer
//! names only the keys that apply now.
//!
//! While [`ShipViewSubject`] names a hull (the Station pane's Info), the view
//! draws that hull in place of the player ship, read-only: it turns, zooms
//! and selects, and every edit above is off.
//!
//! Repair applies at once through a single `SectionRepairCommand` seam (pane
//! key or panel button -> message), spending hull plates from the player
//! ship's `ShipInventory`.
//!
//! The panel lists the design sections the ship has lost, each under its
//! design code, with a Rebuild button that raises a `SectionRebuildCommand`:
//! at once at a docked station that offers Rebuild, or as a timed field
//! rebuild from the hold's parts while the ship is undocked. Repair and field
//! rebuild run as jobs on the game clock; the Repairs block shows each with
//! its progress bar and Cancel. While
//! it is docked at a living station that offers Refit, the footer's Catalogue
//! button opens the parts gallery over the pane: the parts the station can
//! build, each with its price, and Swap on the focus card for a part of the
//! selected section's kind and footprint, which raises a
//! `SectionSwapCommand`. Place puts the part in hand (`HeldPart`): a ghost
//! block follows the pointer over the view, snapped to the free socket
//! nearest it, with its price or refusal tag beside it, and a click on a valid
//! ghost raises a `SectionAddCommand`. While the part is in hand the panel
//! shows it in a Placing block in place of the selection. With no part in
//! hand, a click on a block selects its section. What the station sells as a whole (ships,
//! escorts and the station itself) is in the Station pane (`crate::station`);
//! stripping a docked wreck is in the Inventory pane (`crate::inventory`).
//! The panel scrolls with the wheel while the pointer is over it.
//!
//! # Module layout
//!
//! | Module | Concern |
//! | --- | --- |
//! | `sections` | Section codes, the live section view and the repair seam. |
//! | `app` | The side panel layout and the repair action handler. |
//! | `rebuild` | The Lost block and the rebuild action handler. |
//! | `repair_jobs` | The Repairs block: running repair jobs, their bars and Cancel. |
//! | `rescue` | The Distress block: the SOS call a stranded ship places, its bar and Cancel. |
//! | `refit` | The Swap and Add action handlers and their refusal tags. |
//! | `delete` | The Delete action: its rule, handler and gate. |
//! | `catalogue` | The station catalogue: the gallery host, its listing and Swap. |
//! | `ghost` | The placement ghost, its callout, and the view's pointer and click. |
//! | `held` | The panel's Placing block for the part in hand. |
//! | `scene` | The schematic 3D scene, its camera, the projected badges, the button observers and the panel refresh. |

mod app;
mod catalogue;
mod delete;
mod ghost;
mod held;
mod rebind;
mod rebuild;
mod refit;
mod repair_jobs;
// The answer moves the ship through avian's pose, which the web build has
// none of, so the web build offers no call.
#[cfg(not(target_arch = "wasm32"))]
mod rescue;
mod scene;
mod sections;

#[cfg(test)]
mod tests;

use bevy::prelude::*;
use nova_ship::prelude::{PartsGalleryPlugin, PartsGallerySystems};
use nova_ui::input_mode::prelude::{in_input_mode, InputMode};

#[cfg(not(target_arch = "wasm32"))]
pub(crate) use self::rescue::*;
pub(crate) use self::{
    app::*, catalogue::*, delete::*, ghost::*, held::*, rebind::*, rebuild::*, refit::*,
    repair_jobs::*, scene::*, sections::*,
};
pub use self::{
    delete::{delete_yield_lines, SectionDeleteCommand},
    ghost::{
        ship_ghost_aim, ship_hovered_section, ship_placement, ship_view_window_px,
        socket_lane_open, ShipPlacement, SHIP_GHOST_CALLOUT,
    },
    held::SHIP_HELD_PANEL,
    refit::{SectionAddCommand, SectionSwapCommand},
    scene::{cuboid_edges, ease_orbit_center, ship_framing, ShipRuntime, SHIP_BLOCK_FILL_SCALE},
    sections::{design_section_code, SectionCode, ShipSectionView, ShipSections, ShipViewSubject},
};

/// Glob-import surface: `use nova_interface::ship::prelude::*`.
pub mod prelude {
    // Named by module path: each item is reachable both `pub` (here) and
    // `pub(crate)` (the module glob above), which rustc rejects as an
    // ambiguous import visibility through `super::`.
    pub use super::{
        delete::{delete_yield_lines, SectionDeleteCommand},
        ghost::{
            ship_ghost_aim, ship_hovered_section, ship_placement, ship_view_window_px,
            socket_lane_open, ShipPlacement, SHIP_GHOST_CALLOUT,
        },
        held::SHIP_HELD_PANEL,
        refit::{SectionAddCommand, SectionSwapCommand},
        scene::{
            cuboid_edges, ease_orbit_center, ship_framing, ShipRuntime, SHIP_BLOCK_FILL_SCALE,
        },
        sections::{
            design_section_code, SectionCode, ShipSectionView, ShipSections, ShipViewSubject,
        },
        SHIP_RADIUS_MAX, SHIP_RADIUS_MIN,
    };
}

/// Render layer the ship schematic scene lives on (isolated from the world on 0
/// and the map on 21).
const SHIP_LAYER: usize = 22;
/// The ship camera renders before the command modal RTT (-20); distinct from
/// the map camera (-30) so the two never share an order even mid-teardown.
const SHIP_CAMERA_ORDER: isize = -31;

/// The orbit-radius zoom clamp, shared with the hull views: one section
/// close up to the fixed reach of a hull.
pub use nova_hud::prelude::{SHIP_RADIUS_MAX, SHIP_RADIUS_MIN};
const SHIP_THETA_DEFAULT: f32 = 0.7;
const SHIP_PHI_DEFAULT: f32 = 0.5;
/// Exponential ease rate (1/s) for the orbit center chasing the selected
/// section. Frame-rate independent via `1 - exp(-k*dt)`; higher = snappier.
const SHIP_CENTER_EASE: f32 = 9.0;

/// Drives the Ship pane's schematic scene, blips and section actions.
pub(crate) struct ShipPanePlugin;

impl Plugin for ShipPanePlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<PartsGalleryPlugin>() {
            app.add_plugins(PartsGalleryPlugin);
        }
        app.init_resource::<ShipRuntime>();
        app.init_resource::<ShipViewSubject>();
        app.init_resource::<ShipPlacementPreview>();
        app.add_message::<SectionRepairCommand>();
        app.add_message::<SectionRebuildCommand>();
        app.add_message::<SectionSwapCommand>();
        app.add_message::<SectionAddCommand>();
        app.add_message::<SectionDeleteCommand>();
        app.add_systems(
            Update,
            (
                // Split in two: one tuple holds at most twenty systems.
                (
                    assign_section_codes,
                    apply_ship_gallery_alternates,
                    apply_ship_section_commands,
                    apply_ship_rebuild_commands,
                    apply_ship_swap_commands,
                    apply_ship_add_commands,
                    apply_ship_delete_commands,
                    reselect_swapped_section,
                    manage_ship_scene,
                    sync_ship_station_blocks.run_if(ship_view_editable),
                    reconcile_ship_target,
                    apply_ship_rebind,
                )
                    .chain(),
                (
                    reset_mount_for_new_part,
                    // Ungated: it drains the pointer under every input mode
                    // and acts only under Normal (see `ship_input`).
                    ship_input,
                    scroll_ship_panel,
                    drive_ship_camera,
                    update_ship_blocks,
                    update_ship_placement,
                    update_ship_delete_verdict,
                    pick_ship_part
                        .run_if(in_input_mode(InputMode::Normal))
                        .run_if(ship_view_editable),
                    sync_ship_ghost,
                    sync_ship_target_face,
                    project_ship_blips,
                    label_the_selected_section,
                    update_ship_panel,
                    update_ship_panel_head,
                    update_ship_rebind_prompt,
                    sync_ship_rebind_click_guard,
                    update_ship_held_block,
                    update_ship_rebuild_list.run_if(ship_view_editable),
                    update_ship_repair_jobs.run_if(ship_view_editable),
                    #[cfg(not(target_arch = "wasm32"))]
                    update_ship_rescue.run_if(ship_view_editable),
                )
                    .chain(),
            )
                .chain()
                .in_set(ShipPaneSystems),
        );
        // The host and its listing are ready before the gallery lays out
        // and draws its tiles in the same frame.
        app.add_systems(
            Update,
            (manage_ship_gallery, update_ship_gallery_listing)
                .chain()
                .after(update_ship_panel)
                .in_set(ShipPaneSystems)
                .before(PartsGallerySystems),
        );
    }
}

/// System set for the Ship pane's per-frame work.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct ShipPaneSystems;
