//! The stacking order every full-screen surface in the game is placed in.
//!
//! A modal is a full-screen blocker, and two of them at the same
//! `GlobalZIndex` are ordered by whatever the UI stack's traversal happens to
//! produce that frame - spawn order, hierarchy shape, which despawn landed
//! first. That is not a decision, so the layers are numbered ONCE, here, where
//! `nova_menu`, `nova_os_ui`, `nova_hud` and `nova_core` all read the same
//! table instead of each carrying a number that agrees with the others by
//! habit.
//!
//! Read bottom to top, the order is: the HUD and the screens, the menu's own
//! panels, the editor's ladder, the report frames, the pause menu, its Settings
//! panel, an open NOVA OS, the diagnostics exempt from the NOVA OS dim, the
//! scenario loading screen, and the fatal asset report. Add a surface by naming
//! it here, not by picking a number at the spawn site.
//!
//! One band is not numbered here. The editor stacks six modules of its own
//! chrome - stage labels, rail, foot, gallery, dropdowns, windows, tooltips -
//! and that ladder is `nova_editor::ui::layer`'s to order. This table reserves
//! [`EDITOR_CEILING_Z`] for it and starts the game's modal band above it: an
//! ESC pause, an outcome and an open NOVA OS all cover the editor, which is
//! what a modal means, and the editor's own rungs stay the editor's business.

/// The flight HUD and every ordinary screen: the ground floor, and the value a
/// node with no `GlobalZIndex` already has.
pub const HUD_Z: i32 = 0;

/// The menu's own full-screen panels: Settings, Mods, Scenarios. Over the menu
/// card they are opened from and under everything that covers the menu itself.
pub const MENU_PANEL_Z: i32 = 1;

/// The highest rung the editor's ladder may claim.
///
/// Not a layer - a CEILING, pinned by a test in `nova_editor`. The editor's
/// chrome is the window you look at the scene through, and the game's modals
/// are what take the screen from it, so every editor rung stays below this and
/// every layer below stays above it.
pub const EDITOR_CEILING_Z: i32 = 49;

/// The report frames a run can end on: the outcome overlay, the FAILED TO
/// START report, and the menu's MODS DISABLED acknowledgement. They are one
/// tier because no two of them can be up at once - the pause panel is
/// reconciled away when one arrives, and the menu's report belongs to a state
/// the other two cannot reach.
pub const REPORT_Z: i32 = 50;

/// The pause menu, over the HUD and over a report it was not allowed to cover.
pub const PAUSE_Z: i32 = 60;

/// The pause menu's Settings panel, over the pause buttons it blocks.
pub const PAUSE_SETTINGS_Z: i32 = 70;

/// The dim field behind an open NOVA OS. Above the pause Settings panel
/// because `:` opens the shell OVER the pause menu, and the covering surface
/// is the one on top. The pause overlay is `DespawnOnExit(Paused)`, so today
/// it is gone before the computer is visible and the old shared number never
/// showed - a traversal order nobody chose is not the same as a decision, and
/// the next surface to outlive its transition would have inherited it.
pub const NOVA_OS_BACKDROP_Z: i32 = 80;

/// The monitor itself, over its own backdrop.
pub const NOVA_OS_Z: i32 = 90;

/// Diagnostic and status chrome that stays readable while the NOVA OS is open
/// (`nova_hud`'s `HudNovaOsExempt`): above the dim field, or the gray would
/// take the one readout the player opened the computer to check.
pub const NOVA_OS_EXEMPT_Z: i32 = 95;

/// The scenario loading screen: over every modal a run can raise, because a
/// load requested from a paused outcome frame draws over both of them.
pub const LOADING_Z: i32 = 100;

/// The fatal asset-failure report. Terminal, and over everything.
pub const FATAL_Z: i32 = 200;

/// Glob-import surface for the layer table.
pub mod prelude {
    pub use super::{
        EDITOR_CEILING_Z, FATAL_Z, HUD_Z, LOADING_Z, MENU_PANEL_Z, NOVA_OS_BACKDROP_Z,
        NOVA_OS_EXEMPT_Z, NOVA_OS_Z, PAUSE_SETTINGS_Z, PAUSE_Z, REPORT_Z,
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The table's whole point: every layer is DISTINCT and in the documented
    /// order. A new surface that reuses a number lands here rather than in a
    /// traversal order nobody chose.
    #[test]
    fn every_modal_layer_is_distinct_and_in_order() {
        let table = [
            ("HUD", HUD_Z),
            ("menu panel", MENU_PANEL_Z),
            ("editor ceiling", EDITOR_CEILING_Z),
            ("report", REPORT_Z),
            ("pause", PAUSE_Z),
            ("pause settings", PAUSE_SETTINGS_Z),
            ("nova os backdrop", NOVA_OS_BACKDROP_Z),
            ("nova os", NOVA_OS_Z),
            ("nova os exempt", NOVA_OS_EXEMPT_Z),
            ("loading", LOADING_Z),
            ("fatal", FATAL_Z),
        ];
        for pair in table.windows(2) {
            let [(below, below_z), (above, above_z)] = pair else {
                unreachable!("windows(2)")
            };
            assert!(
                below_z < above_z,
                "'{below}' ({below_z}) must sit strictly below '{above}' ({above_z})"
            );
        }
    }
}
