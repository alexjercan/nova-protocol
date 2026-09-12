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
use nova_ui::prelude::{owns_or_enters, InputMode};
pub(crate) use scene::EditorCamera;
pub(crate) use ui::{EditorChrome, GalleryAction};

use crate::ExampleStates;

/// The widest grid the gallery ever draws.
///
/// A cap rather than "as many as fit": past eight across, a tile is a thumbnail
/// the size of its own name strip, and the browse flow this surface exists for
/// (see the module comment) is reading shapes, not scanning a contact sheet.
const COLS_MAX: usize = 8;

/// The tallest grid the gallery ever draws, for the reason [`COLS_MAX`] gives.
const ROWS_MAX: usize = 5;

/// The smallest a tile may be drawn and still read, in logical pixels: enough
/// preview to tell a thruster from a turret, and enough name strip to hold a
/// prototype name without clipping it.
const TILE_MIN: Vec2 = Vec2::new(220.0, 200.0);

/// The grid's own padding on each side, and the gap between two tiles. The
/// numbers `ui::grid_body` lays out with.
const GRID_PAD: f32 = 12.0;
/// See [`GRID_PAD`].
const TILE_GAP: f32 = 10.0;

/// What the gallery's header row and hint line take off the window height
/// before the grid is given what is left.
///
/// A measured constant rather than a read of the laid-out nodes: the grid's own
/// height is what this decides, so reading it back would be a layout that
/// depended on its own answer.
const GALLERY_CHROME: f32 = 86.0;

/// How many tiles a page holds, and in what shape.
///
/// DERIVED from the window rather than a fixed 4x3: 200 prototypes are 17 pages
/// on a 4K display with room for 40, and the same grid on a small window draws
/// tiles too small to tell apart. Paging and the arrow keys both read it, so a
/// resize changes what Page Down means and what Down lands on together.
#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct GalleryLayout {
    cols: usize,
    rows: usize,
}

impl Default for GalleryLayout {
    /// The grid at the 1024x768 size the editor is built for, which is the 4x3
    /// this replaced.
    fn default() -> Self {
        Self::for_viewport(Vec2::new(1024.0, 768.0))
    }
}

impl GalleryLayout {
    /// The grid that fits a window of `viewport` logical pixels.
    pub(crate) fn for_viewport(viewport: Vec2) -> Self {
        Self {
            cols: fits(viewport.x, TILE_MIN.x, COLS_MAX),
            rows: fits(viewport.y - GALLERY_CHROME, TILE_MIN.y, ROWS_MAX),
        }
    }

    /// Tiles across.
    pub(crate) fn cols(self) -> usize {
        self.cols
    }

    /// Tile rows.
    pub(crate) fn rows(self) -> usize {
        self.rows
    }

    /// Prototypes on one page.
    pub(crate) fn page(self) -> usize {
        self.cols * self.rows
    }
}

/// How many tiles of at least `tile` fit along `space`: at least one, at most
/// `cap`. A window too small for even one still gets one, because a gallery
/// with no tiles is a gallery that cannot be used at all.
fn fits(space: f32, tile: f32, cap: usize) -> usize {
    let room = space - 2.0 * GRID_PAD + TILE_GAP;
    let count = (room / (tile + TILE_GAP)).floor();
    if count >= 1.0 {
        (count as usize).min(cap)
    } else {
        1
    }
}

/// Follow the window: the grid is derived, so a resize has to reach the paging
/// and the keyboard before either answers again.
pub(crate) fn sync_gallery_layout(
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
    mut layout: ResMut<GalleryLayout>,
) {
    let Ok(window) = windows.single() else {
        return;
    };
    let wanted = GalleryLayout::for_viewport(Vec2::new(window.width(), window.height()));
    if *layout != wanted {
        *layout = wanted;
    }
}

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
    app.init_resource::<GalleryLayout>();
    app.init_resource::<scene::GalleryStage>();
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
            //
            // Both are Browse's own systems, so both answer under Browse and
            // under Normal, and both go quiet under a mode above it: browsing
            // parts is not what a builder naming a beacon asked for, and a
            // rebind waiting for a key gets the key.
            input::toggle_gallery
                .run_if(gallery_reachable)
                .run_if(owns_or_enters(InputMode::Browse)),
            input::gallery_keyboard.run_if(owns_or_enters(InputMode::Browse)),
            // Both BEFORE the rebuild: the grid is laid out from one and the
            // tiles are spawned on the other, and either read a frame late
            // would draw the page the window used to hold.
            sync_gallery_layout,
            scene::sync_gallery_stage,
            ui::rebuild_gallery,
            ui::paint_gallery_cells,
            ui::sync_editor_chrome,
            scene::park_camera_for_gallery,
            scene::measure_gallery_items,
            scene::place_gallery_items,
            scene::pose_focused_item,
            scene::draw_focus_sockets,
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
    use bevy::{core_pipeline::Skybox, ui_widgets::Activate};

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

    /// The grid a window gets is the grid its space pays for, between one tile
    /// and the readable caps. The 1024x768 editor keeps the 4x3 this replaced,
    /// so the change is a 4K display gaining tiles, not every window changing
    /// shape.
    #[test]
    fn the_grid_grows_with_the_window_and_stops_at_the_caps() {
        assert_eq!(
            GalleryLayout::for_viewport(Vec2::new(1024.0, 768.0)),
            GalleryLayout { cols: 4, rows: 3 },
            "the size the editor is built for keeps the grid it had"
        );

        let wall = GalleryLayout::for_viewport(Vec2::new(3840.0, 2160.0));
        assert_eq!(
            (wall.cols(), wall.rows()),
            (COLS_MAX, ROWS_MAX),
            "a 4K display fills to the caps, not past them"
        );
        assert!(wall.page() > GalleryLayout::default().page());

        let slot = GalleryLayout::for_viewport(Vec2::new(200.0, 120.0));
        assert_eq!(
            (slot.cols(), slot.rows()),
            (1, 1),
            "a window too small for one tile still draws one"
        );
    }

    /// A resize has to reach the layout: the paging and the arrow keys both
    /// read it, so a stale grid means Page Down moves by a page the player is
    /// no longer looking at.
    #[test]
    fn a_resize_moves_the_grid_the_paging_reads() {
        let mut app = App::new();
        app.init_resource::<GalleryLayout>();
        let window = app
            .world_mut()
            .spawn((
                Window {
                    resolution: bevy::window::WindowResolution::new(1024, 768),
                    ..default()
                },
                bevy::window::PrimaryWindow,
            ))
            .id();
        app.add_systems(Update, sync_gallery_layout);

        app.update();
        let small = *app.world().resource::<GalleryLayout>();
        assert_eq!((small.cols(), small.rows()), (4, 3));

        app.world_mut()
            .entity_mut(window)
            .get_mut::<Window>()
            .expect("the window is still there")
            .resolution
            .set(2560.0, 1440.0);
        app.update();

        let wide = *app.world().resource::<GalleryLayout>();
        assert!(
            wide.cols() > small.cols() && wide.rows() > small.rows(),
            "the wider window draws a wider grid (got {wide:?})"
        );
        assert_eq!(wide.page(), wide.cols() * wide.rows());
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
        let sections = catalog_of(&["hull_a", "hauler_nose", "hauler_tail"]);
        let mut state = GalleryState {
            filter: "hauler".to_string(),
            ..default()
        };
        assert_eq!(state.selected_id(&sections).as_deref(), Some("hauler_nose"));
        state.selected = 1;
        assert_eq!(state.selected_id(&sections).as_deref(), Some("hauler_tail"));
        // Out of range (a filter that just narrowed) resolves to nothing rather
        // than to a neighbour.
        state.selected = 7;
        assert_eq!(state.selected_id(&sections), None);
    }

    /// The stage is empty by contract: the scenario's skybox comes off the
    /// camera while the gallery is parked on it, and goes back on at the close.
    /// A cubemap is not clipped by anything, so leaving it on put a star in
    /// among the tiles.
    #[test]
    fn parking_the_camera_takes_the_skybox_off_and_gives_it_back() {
        let mut app = App::new();
        app.insert_resource(GalleryState {
            open: true,
            ..default()
        });
        app.init_resource::<scene::GalleryStage>();
        app.add_systems(Update, scene::park_camera_for_gallery);
        let flown_to = Vec3::new(1.0, 2.0, 3.0);
        let camera = app
            .world_mut()
            .spawn((
                scene::EditorCamera,
                Transform::from_translation(flown_to),
                Skybox {
                    brightness: 42.0,
                    ..default()
                },
            ))
            .id();

        app.update();
        assert!(
            app.world().get::<Skybox>(camera).is_none(),
            "the parked camera must not draw the world's sky"
        );

        app.world_mut().resource_mut::<GalleryState>().open = false;
        app.update();
        let sky = app
            .world()
            .get::<Skybox>(camera)
            .expect("closing the gallery gives the sky back");
        assert_eq!(sky.brightness, 42.0, "and gives back the SAME sky");
        assert_eq!(
            app.world()
                .get::<Transform>(camera)
                .map(|pose| pose.translation),
            Some(flown_to),
            "along with the pose the builder had flown to"
        );
    }

    /// Clearing the filter widens the grid and leaves the category alone: the
    /// x beside the field answers "why is this grid short", and the chip row
    /// answers a different question.
    #[test]
    fn clearing_the_filter_keeps_the_category() {
        let mut world = World::new();
        world.insert_resource(catalog_of(&["hull_a"]));
        world.init_resource::<GalleryLayout>();
        world.insert_resource(SectionChoice::default());
        world.insert_resource(GalleryState {
            open: true,
            category: GalleryCategory::Weapons,
            filter: "hauler".to_string(),
            selected: 4,
            focused: true,
            ..default()
        });
        world.add_observer(ui::on_gallery_action);
        let row = world.spawn(GalleryAction::ClearFilter).id();

        world.trigger(Activate { entity: row });
        world.flush();

        let state = world.resource::<GalleryState>();
        assert!(state.filter.is_empty());
        assert_eq!(state.category, GalleryCategory::Weapons);
        assert_eq!(state.selected, 0, "a wider list renumbers the tiles");
        assert!(state.open, "clearing a filter is not leaving the gallery");
    }

    /// An Add row that NAMES a kind opens the gallery on that kind, and on a
    /// clean filter. Plain Open keeps the last browse's category, which would
    /// make "Add > Weapons" show hulls.
    #[test]
    fn browsing_a_kind_opens_the_gallery_on_that_kind() {
        let mut world = World::new();
        world.insert_resource(catalog_of(&["hull_a"]));
        world.init_resource::<GalleryLayout>();
        world.insert_resource(SectionChoice::default());
        world.insert_resource(GalleryState {
            category: GalleryCategory::Structure,
            filter: "hauler".to_string(),
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
