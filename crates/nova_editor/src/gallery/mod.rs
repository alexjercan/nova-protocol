//! The parts gallery: a full-screen browse-and-pick surface over the section
//! catalog - 3D preview tiles with labels, a category row, a text filter and a
//! focus turntable - that hands its pick to the editor's placement tool.
//!
//! It is a second picker, not a replacement: the drawer list stays the
//! quick-pick surface for a part you already know. Change this module when the
//! browse flow changes; `catalog` owns WHAT is listed, `ui` the layout, `scene`
//! the 3D tiles and `input` the keyboard.

pub(crate) mod catalog;
mod input;
mod scene;
mod ui;

use bevy::prelude::*;
use nova_ship::prelude::*;
pub(crate) use scene::EditorCamera;
pub(crate) use ui::{EditorChrome, GalleryAction};

use crate::{gallery::catalog::GalleryCategory, ExampleStates};

/// Tile grid of one page.
pub(crate) const COLS: usize = 4;
/// Tile rows of one page.
pub(crate) const ROWS: usize = 3;
/// Prototypes shown per page.
pub(crate) const PAGE: usize = COLS * ROWS;

/// What the gallery is showing. The single source the overlay is rebuilt from,
/// so every control (mouse, keyboard, autopilot) drives the same seam.
#[derive(Resource, Clone, Debug, Default, PartialEq)]
pub(crate) struct GalleryState {
    /// Whether the overlay is up.
    pub(crate) open: bool,
    /// Active category filter.
    pub(crate) category: GalleryCategory,
    /// Case-insensitive name/id filter.
    pub(crate) filter: String,
    /// Selection index into the FILTERED list, not the catalog.
    pub(crate) selected: usize,
    /// Whether the focus card is up for the selection.
    pub(crate) focused: bool,
}

impl GalleryState {
    /// Move the selection by `delta`, clamped to a list of `len` entries.
    /// Clamped rather than wrapped: paging off the end of the last page should
    /// stop there, not jump back to the first tile.
    pub(crate) fn step(&mut self, delta: isize, len: usize) {
        if len == 0 {
            self.selected = 0;
            return;
        }
        let last = len as isize - 1;
        self.selected = (self.selected as isize + delta).clamp(0, last) as usize;
    }

    /// The catalog id of the selected prototype, if the filtered list still has
    /// one there.
    pub(crate) fn selected_id(&self, sections: &GameSections) -> Option<String> {
        let listed = catalog::browsable(sections, self.category, &self.filter);
        let index = *listed.get(self.selected)?;
        Some(sections.get(index)?.base.id.clone())
    }
}

/// True while the gallery is up: the run condition for the editor systems that
/// must not act on input the gallery owns.
pub(crate) fn gallery_open(state: Res<GalleryState>) -> bool {
    state.open
}

/// Wire the gallery into the editor plugin.
pub(crate) fn register(app: &mut App) {
    app.init_resource::<GalleryState>();

    // A stale gallery must not survive a scene change, exactly as the section
    // choice and the pending rebind do not.
    app.add_systems(
        OnEnter(ExampleStates::Editor),
        |mut state: ResMut<GalleryState>| *state = GalleryState::default(),
    );

    app.add_systems(
        Update,
        (
            input::gallery_keyboard,
            ui::rebuild_gallery,
            ui::paint_gallery_cells,
            ui::sync_editor_chrome,
            scene::park_camera_for_gallery,
            scene::place_gallery_items,
            scene::spin_focused_item,
        )
            .chain()
            .run_if(in_state(ExampleStates::Editor)),
    );

    app.add_observer(ui::on_gallery_action);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn catalog_of(ids: &[&str]) -> GameSections {
        GameSections(
            ids.iter()
                .map(|id| SectionConfig {
                    base: BaseSectionConfig {
                        id: (*id).to_string(),
                        name: (*id).to_string(),
                        ..default()
                    },
                    kind: SectionKind::Hull(HullSectionConfig::default()),
                })
                .collect(),
        )
    }

    /// Paging past either end stops at the end. Wrapping here would silently
    /// move the selection a page away from what the player was looking at.
    #[test]
    fn stepping_clamps_at_both_ends() {
        let mut state = GalleryState::default();
        state.step(-1, 3);
        assert_eq!(state.selected, 0);
        state.step(99, 3);
        assert_eq!(state.selected, 2);
        // An empty list has nothing to select.
        state.step(0, 0);
        assert_eq!(state.selected, 0);
    }

    /// The selection indexes the FILTERED list: with a filter up, tile 0 is the
    /// first match, not the first catalog entry.
    #[test]
    fn the_selection_resolves_through_the_active_filter() {
        let sections = catalog_of(&["hull_a", "racer_nose", "racer_tail"]);
        let mut state = GalleryState {
            filter: "racer".to_string(),
            ..default()
        };
        assert_eq!(state.selected_id(&sections).as_deref(), Some("racer_nose"));
        state.selected = 1;
        assert_eq!(state.selected_id(&sections).as_deref(), Some("racer_tail"));
        // Out of range (a filter that just narrowed) resolves to nothing rather
        // than to a neighbour.
        state.selected = 7;
        assert_eq!(state.selected_id(&sections), None);
    }
}
