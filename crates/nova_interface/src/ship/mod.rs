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
//! The section panel beside the view shows the selection's condition bar,
//! description, labelled facts, the Rebind action and, while the selection has
//! repairable damage, the repair form. Orbit the camera with the `viewer_orbit_*` actions +
//! drag + wheel; `[`/`]` or Prev/Next cycle the selection; Fit frames the whole
//! ship at the current angles, and Reset also restores the opening angles;
//! `G` toggles structural mates; `P` repairs, and `B` replaces the selected
//! bindable section's input. While the form's quantity field has the caret,
//! these keys type into it instead.
//!
//! Repair applies at once through a single `SectionRepairCommand` seam (pane
//! key or panel button -> message), spending hull plates from the player
//! ship's `ShipInventory`.
//!
//! While the ship is docked at a living station that offers Rebuild, the
//! panel lists the design sections the ship has lost, each under its design
//! code, with a Rebuild button that raises a `SectionRebuildCommand`. While
//! it is docked at a living station that offers Refit, the panel shows the
//! station catalogue: the parts the station can build, each with its price,
//! a Place button and, for a part of the selected section's kind and
//! footprint, a Swap button that raises a `SectionSwapCommand`. Place puts
//! the part in hand: a ghost block follows the pointer over the view,
//! snapped to the free socket nearest it, with its price or refusal beside
//! it, and a click on a valid ghost raises a `SectionAddCommand`. With no
//! part in hand, a click on a block selects its section. What the station
//! sells as a whole (ships, escorts and the station itself) is in the
//! Station pane (`crate::station`); stripping a docked wreck is in the
//! Inventory pane (`crate::inventory`).
//! The panel scrolls with the wheel while the pointer is over it.
//!
//! # Module layout
//!
//! | Module | Concern |
//! | --- | --- |
//! | `sections` | Section codes, the live section view and the repair seam. |
//! | `app` | The side panel layout and the repair action handler. |
//! | `rebuild` | The lost-section list and the rebuild action handler. |
//! | `refit` | The Swap and Add action handlers and their refusal lines. |
//! | `catalogue` | The station catalogue: listed parts, prices, Place and Swap. |
//! | `ghost` | The placement ghost, its callout, and the view's pointer and click. |
//! | `scene` | The schematic 3D scene, its camera, the projected badges, the button observers and the panel refresh. |

mod app;
mod catalogue;
mod ghost;
mod rebind;
mod rebuild;
mod refit;
mod scene;
mod sections;

#[cfg(test)]
mod tests;

use bevy::prelude::*;
use nova_ui::input_mode::prelude::{in_input_mode, InputMode};

pub(crate) use self::{
    app::*, catalogue::*, ghost::*, rebind::*, rebuild::*, refit::*, scene::*, sections::*,
};
pub use self::{
    ghost::{
        ship_ghost_aim, ship_placement, ship_view_window_px, socket_lane_open, ShipPlacement,
        SHIP_GHOST_CALLOUT,
    },
    refit::{SectionAddCommand, SectionSwapCommand},
    scene::{cuboid_edges, ease_orbit_center, ship_framing, ShipRuntime, SHIP_BLOCK_FILL_SCALE},
    sections::{design_section_code, SectionCode, ShipSectionView, ShipSections},
};

/// Glob-import surface: `use nova_interface::ship::prelude::*`.
pub mod prelude {
    // Named by module path: each item is reachable both `pub` (here) and
    // `pub(crate)` (the module glob above), which rustc rejects as an
    // ambiguous import visibility through `super::`.
    pub use super::{
        ghost::{
            ship_ghost_aim, ship_placement, ship_view_window_px, socket_lane_open, ShipPlacement,
            SHIP_GHOST_CALLOUT,
        },
        refit::{SectionAddCommand, SectionSwapCommand},
        scene::{
            cuboid_edges, ease_orbit_center, ship_framing, ShipRuntime, SHIP_BLOCK_FILL_SCALE,
        },
        sections::{design_section_code, SectionCode, ShipSectionView, ShipSections},
        SHIP_RADIUS_MAX, SHIP_RADIUS_MIN,
    };
}

/// Render layer the ship schematic scene lives on (isolated from the world on 0
/// and the map on 21).
const SHIP_LAYER: usize = 22;
/// The ship camera renders before the command modal RTT (-20); distinct from
/// the map camera (-30) so the two never share an order even mid-teardown.
const SHIP_CAMERA_ORDER: isize = -31;

/// Orbit-radius zoom clamp floor (world units from the center): close enough to
/// read one section.
pub const SHIP_RADIUS_MIN: f32 = 3.0;
/// Orbit-radius zoom clamp ceiling: the fixed reach of a hull.
pub const SHIP_RADIUS_MAX: f32 = 400.0;
const SHIP_THETA_DEFAULT: f32 = 0.7;
const SHIP_PHI_DEFAULT: f32 = 0.5;
/// Exponential ease rate (1/s) for the orbit center chasing the selected
/// section. Frame-rate independent via `1 - exp(-k*dt)`; higher = snappier.
const SHIP_CENTER_EASE: f32 = 9.0;

/// Drives the Ship pane's schematic scene, blips and section actions.
pub(crate) struct ShipPanePlugin;

impl Plugin for ShipPanePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ShipRuntime>();
        app.init_resource::<ShipPlacementPreview>();
        app.add_message::<SectionRepairCommand>();
        app.add_message::<SectionRebuildCommand>();
        app.add_message::<SectionSwapCommand>();
        app.add_message::<SectionAddCommand>();
        app.add_systems(
            Update,
            (
                // Split in two: one tuple holds at most twenty systems.
                (
                    assign_section_codes,
                    apply_ship_section_commands,
                    apply_ship_rebuild_commands,
                    apply_ship_swap_commands,
                    apply_ship_add_commands,
                    reselect_swapped_section,
                    manage_ship_scene,
                    sync_ship_station_blocks,
                    reconcile_ship_target,
                    apply_ship_rebind,
                )
                    .chain(),
                (
                    // A focused repair field types P, B and the selection keys.
                    ship_input.run_if(in_input_mode(InputMode::Normal)),
                    scroll_ship_panel,
                    drive_ship_camera,
                    update_ship_blocks,
                    update_ship_placement,
                    sync_ship_ghost,
                    project_ship_blips,
                    label_the_selected_section,
                    update_ship_panel,
                    update_ship_rebuild_list,
                    update_ship_catalogue,
                )
                    .chain(),
            )
                .chain()
                .in_set(ShipPaneSystems),
        );
    }
}

/// System set for the Ship pane's per-frame work.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct ShipPaneSystems;
