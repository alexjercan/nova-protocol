//! The parts gallery: a full-screen browse-and-pick surface over the section
//! catalog - 3D preview tiles with labels, a category row, a text filter and a
//! focus turntable - that hands its pick to the editor's placement tool.
//!
//! It is the editor's ONLY parts picker (the component drawer it replaced could
//! not show what a part looks like), so it carries both a browse flow and a
//! fast one: Tab up, point, Q, back to building. Change this module when the
//! browse flow changes; `catalog` owns WHAT is listed, `ui` the layout, `scene`
//! the 3D tiles and `input` the keyboard.

pub(crate) mod catalog;
mod input;
mod scene;
mod ui;

use bevy::{
    input::mouse::{MouseMotion, MouseWheel},
    prelude::*,
};
pub(crate) use catalog::GalleryCategory;
pub(crate) use input::gallery_keyboard;
use nova_ship::prelude::*;
pub(crate) use scene::EditorCamera;
pub(crate) use ui::{EditorChrome, GalleryAction};

use crate::ExampleStates;

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
    /// Whether the filter field has the caret. Typing reaches the filter only
    /// while it does, which is what leaves the letters free to be shortcuts.
    pub(crate) filter_focused: bool,
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

/// Whether Tab has a gallery to talk to: parts are ship-context verbs (the
/// Parts button lives in the ship's action group, and a part armed at the
/// scenario node is put straight back down), so the toggle only OPENS inside
/// a ship - while staying answerable wherever it is already open, so Tab can
/// always close what Tab opened.
fn gallery_reachable(context: Res<crate::node::EditContext>, state: Res<GalleryState>) -> bool {
    state.open || context.ship().is_some()
}

/// Wire the gallery into the editor plugin.
pub(crate) fn register(app: &mut App) {
    app.init_resource::<GalleryState>();
    app.init_resource::<scene::FocusView>();

    // A stale gallery must not survive a scene change, exactly as the section
    // choice and the pending rebind do not.
    app.add_systems(
        OnEnter(ExampleStates::Editor),
        |mut state: ResMut<GalleryState>| *state = GalleryState::default(),
    );

    app.add_systems(
        Update,
        (
            // Ahead of the rest: Tab is the only gallery key that acts while
            // the gallery is CLOSED, and the frame it opens on is a frame the
            // browse keys must already see as open.
            // Tab is a key like any other while an inspector field has the
            // keyboard: browsing parts is not what a builder naming a beacon
            // asked for.
            input::toggle_gallery
                .run_if(gallery_reachable)
                .run_if(not(crate::ui::inspector::typing_into_a_field)),
            input::gallery_keyboard,
            ui::rebuild_gallery,
            ui::paint_gallery_cells,
            ui::sync_editor_chrome,
            scene::park_camera_for_gallery,
            scene::measure_gallery_items,
            scene::place_gallery_items,
            scene::pose_focused_item,
        )
            .chain()
            .run_if(in_state(ExampleStates::Editor)),
    );
    // The focus view's zoom and orbit, split off so a headless rig with no
    // input plugin (and so no wheel or motion queue) still runs the rest.
    app.add_systems(
        Update,
        scene::drive_focus_view
            .before(scene::place_gallery_items)
            .run_if(resource_exists::<Messages<MouseMotion>>)
            .run_if(resource_exists::<Messages<MouseWheel>>)
            .run_if(in_state(ExampleStates::Editor)),
    );

    app.add_observer(ui::on_gallery_action);
}

#[cfg(test)]
mod tests {
    use bevy::ui_widgets::Activate;

    use super::*;
    use crate::config::SectionChoice;

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

    /// An Add row that NAMES a kind opens the gallery on that kind, and on a
    /// clean filter. Plain Open keeps the last browse's category, which would
    /// make "Add > Weapons" show hulls.
    #[test]
    fn browsing_a_kind_opens_the_gallery_on_that_kind() {
        let mut world = World::new();
        world.insert_resource(catalog_of(&["hull_a"]));
        world.insert_resource(SectionChoice::default());
        world.insert_resource(GalleryState {
            category: GalleryCategory::Structure,
            filter: "racer".to_string(),
            selected: 4,
            focused: true,
            ..default()
        });
        world.add_observer(ui::on_gallery_action);
        let row = world
            .spawn(GalleryAction::Browse(GalleryCategory::Weapons))
            .id();

        world.trigger(Activate { entity: row });
        world.flush();

        let state = world.resource::<GalleryState>();
        assert!(state.open);
        assert_eq!(state.category, GalleryCategory::Weapons);
        assert!(state.filter.is_empty(), "a named row opens unfiltered");
        assert_eq!(state.selected, 0);
        assert!(!state.focused, "on the grid, not on a leftover focus card");
    }
}
