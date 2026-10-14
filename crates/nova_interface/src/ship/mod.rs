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
//! it is docked at a living station that offers Refit, the panel lists the
//! parts of the same kind and footprint the selected section can be swapped
//! for, each with a Swap button that raises a `SectionSwapCommand`, and
//! under them the selected section's free sockets and the parts that fit at
//! the chosen one, each with an Add button that raises a `SectionAddCommand`.
//! What the station sells (ships, escorts and the station itself) is in the
//! Station pane (`crate::station`).
//! While the ship is docked with a hull it may loot, a "Strip the docked
//! wreck" list shows each of the partner's sections with the parts a strip
//! gives, its time and a Strip button that raises a `SectionStripCommand`;
//! the strip runs on the game clock and its progress shows under the header.
//! The panel scrolls with the wheel while the pointer is over it.
//!
//! # Module layout
//!
//! | Module | Concern |
//! | --- | --- |
//! | `sections` | Section codes, the live section view and the repair seam. |
//! | `app` | The side panel layout and the repair action handler. |
//! | `rebuild` | The lost-section list and the rebuild action handler. |
//! | `refit` | The swap and add lists and their action handlers. |
//! | `strip` | The "Strip the docked wreck" list, the strip action handler and the strip timer. |
//! | `scene` | The schematic 3D scene, its camera, the projected badges, the button observers and the panel refresh. |

mod app;
mod rebind;
mod rebuild;
mod refit;
mod scene;
mod sections;
mod strip;

#[cfg(test)]
mod tests;

use bevy::prelude::*;
use nova_ui::input_mode::prelude::{in_input_mode, InputMode};

pub(crate) use self::{app::*, rebind::*, rebuild::*, refit::*, scene::*, sections::*, strip::*};
pub use self::{
    refit::{SectionAddCommand, SectionSwapCommand},
    scene::{cuboid_edges, ease_orbit_center, ship_framing, ShipRuntime, SHIP_BLOCK_FILL_SCALE},
    sections::{design_section_code, SectionCode, ShipSections},
    strip::{SectionStripCommand, STRIP_BUTTON_PREFIX, STRIP_LIST, STRIP_PROGRESS},
};

/// Glob-import surface: `use nova_interface::ship::prelude::*`.
pub mod prelude {
    // Named by module path: each item is reachable both `pub` (here) and
    // `pub(crate)` (the module glob above), which rustc rejects as an
    // ambiguous import visibility through `super::`.
    pub use super::{
        refit::{SectionAddCommand, SectionSwapCommand},
        scene::{
            cuboid_edges, ease_orbit_center, ship_framing, ShipRuntime, SHIP_BLOCK_FILL_SCALE,
        },
        sections::{design_section_code, SectionCode, ShipSections},
        strip::{SectionStripCommand, STRIP_BUTTON_PREFIX, STRIP_LIST, STRIP_PROGRESS},
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
        app.add_message::<SectionRepairCommand>();
        app.add_message::<SectionRebuildCommand>();
        app.add_message::<SectionSwapCommand>();
        app.add_message::<SectionAddCommand>();
        app.add_message::<SectionStripCommand>();
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
                    apply_ship_strip_commands,
                    // Runs with the pane closed too: a strip goes on in flight.
                    advance_ship_strip,
                    reselect_swapped_section,
                    manage_ship_scene,
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
                    project_ship_blips,
                    label_the_selected_section,
                    update_ship_panel,
                    update_ship_rebuild_list,
                    update_ship_swap_list,
                    update_ship_add_list,
                    update_ship_strip_list,
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
