//! The parts gallery: a browse-and-pick surface over the section catalog - 3D
//! preview tiles with labels, a category row, a text filter and a focus
//! turntable - that puts its pick in hand ([`HeldPart`]).
//!
//! Two hosts show it: the editor, full screen over its stage, and the Ship
//! pane, over its view while docked at a refit station. A host spawns a UI
//! node with a [`GalleryHost`] (and, to price its parts, a [`GalleryListing`]),
//! and the gallery lays itself out inside that node and draws its tiles
//! through the host's camera. Everything else here is the gallery's own.
//! Change this module when the browse flow changes; `catalog` owns WHAT is
//! listed, `ui` the layout, `scene` the 3D tiles and `input` the keyboard.

pub mod catalog;
mod input;
mod scene;
mod ui;

use bevy::{
    app::HierarchyPropagatePlugin,
    camera::visibility::RenderLayers,
    gizmos::GizmoPlugin,
    input::mouse::{MouseMotion, MouseWheel},
    prelude::*,
    ui::ComputedNode,
};
use nova_ui::prelude::{owns_or_enters, ClaimKeyboard, InputMode, InputModeSystems};

pub use self::{
    catalog::GalleryCategory,
    input::{gallery_keyboard, GALLERY_FILTER_KEY, GALLERY_PICK_KEY},
    scene::{draw_socket, FocusView, GalleryGizmos, GalleryItem, GalleryTileOf, GalleryTiles},
    ui::{GalleryAction, GalleryCell, GalleryRoot},
};
use crate::prelude::*;

/// Glob-import surface: `use nova_ship::gallery::prelude::*`.
pub mod prelude {
    pub use super::{
        catalog::browsable, draw_socket, gallery_open, GalleryAction, GalleryAlternate,
        GalleryCategory, GalleryCell, GalleryEntry, GalleryGizmos, GalleryHost, GalleryItem,
        GalleryLayout, GalleryListing, GalleryState, HeldPart, PartsGalleryPlugin,
        PartsGallerySystems,
    };
}

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

/// What the gallery's header row and hint line take off the host's height
/// before the grid is given what is left.
///
/// A measured constant rather than a read of the laid-out nodes: the grid's own
/// height is what this decides, so reading it back would be a layout that
/// depended on its own answer.
const GALLERY_CHROME: f32 = 86.0;

/// The part in hand: the catalog id a click places, or `None` to select.
///
/// One resource for every surface that places parts. The gallery writes it,
/// the editor's placement tool and the Ship pane's ghost read it, and each
/// host clears it when its own view goes away.
#[derive(Resource, Default, Debug, PartialEq, Eq, Clone, Reflect)]
#[reflect(Resource)]
pub struct HeldPart(pub Option<String>);

impl HeldPart {
    /// The held catalog id, if a part is in hand.
    pub fn id(&self) -> Option<&str> {
        self.0.as_deref()
    }
}

/// The UI node a host lays the gallery out in, and how its tiles are drawn.
///
/// The overlay spawns as a child filling this node. A cell maps to the
/// camera's viewport by its place in the node, so the camera's viewport must
/// show exactly this node: the window for a full-screen node, or an image the
/// node displays 1:1. One host at a time; the gallery stays idle with none.
#[derive(Component, Clone, Debug)]
pub struct GalleryHost {
    /// The camera that draws the tiles.
    pub camera: Entity,
    /// Where the tiles stand, in world space. Clear of anything else the
    /// camera's layers hold.
    pub stage: Vec3,
    /// The render layers the tiles and the focus view's socket marks draw on.
    pub layers: RenderLayers,
    /// The hint line's close keys, as the host binds them (`Esc` or
    /// `Esc/Tab`).
    pub close_keys: &'static str,
}

/// The parts a host offers, each with its own line, marking what it cannot
/// craft. Without one the gallery lists the whole browsable catalog.
#[derive(Component, Clone, Debug, Default, PartialEq)]
pub struct GalleryListing(pub Vec<GalleryEntry>);

impl GalleryListing {
    /// The entry for catalog id `id`, if it is listed.
    pub fn entry(&self, id: &str) -> Option<&GalleryEntry> {
        self.0.iter().find(|entry| entry.id == id)
    }
}

/// One listed part.
#[derive(Clone, Debug, PartialEq)]
pub struct GalleryEntry {
    /// The catalog id.
    pub id: String,
    /// The short line on the part's tile, under its name: a price or a
    /// refusal in a few words. It wraps inside the tile. `None` draws no
    /// line, as for a part that cannot be placed, which its muted tile says.
    pub tile: Option<String>,
    /// The whole price or refusal. On a placeable part the focus card draws
    /// it as a line above Place; on one that cannot be placed it is only the
    /// disabled Place's tooltip, why the part cannot be placed.
    pub note: String,
    /// The recipe, one line per item in id order. The focus card draws it at
    /// the top of the clipping details column, so a short host clips it
    /// before it moves the buttons.
    pub recipe: Vec<String>,
    /// A second verb on the focus card beside Place, as its button label;
    /// pressing it writes [`GalleryAlternate`].
    pub alternate: Option<String>,
    /// Whether Place, Enter and Q put this part in hand. A host lists a part
    /// it cannot craft with `false` and says why in `note`; the grid and the
    /// focus card draw it muted, and the card disables Place.
    pub placeable: bool,
}

/// The focus card's second verb was pressed for catalog id `id`. The host
/// that listed the verb acts on it; the gallery closes.
#[derive(Message, Clone, Debug, PartialEq, Eq)]
pub struct GalleryAlternate {
    /// The catalog id the verb was offered for.
    pub id: String,
}

/// How many tiles a page holds, and in what shape.
///
/// DERIVED from the host node rather than a fixed 4x3: 200 prototypes are 17
/// pages on a 4K display with room for 40, and the same grid on a small window
/// draws tiles too small to tell apart. Paging and the arrow keys both read it,
/// so a resize changes what Page Down means and what Down lands on together.
#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
pub struct GalleryLayout {
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
    /// The grid that fits a host node of `viewport` logical pixels.
    pub fn for_viewport(viewport: Vec2) -> Self {
        Self {
            cols: fits(viewport.x, TILE_MIN.x, COLS_MAX),
            rows: fits(viewport.y - GALLERY_CHROME, TILE_MIN.y, ROWS_MAX),
        }
    }

    /// Tiles across.
    pub fn cols(self) -> usize {
        self.cols
    }

    /// Tile rows.
    pub fn rows(self) -> usize {
        self.rows
    }

    /// Prototypes on one page.
    pub fn page(self) -> usize {
        self.cols * self.rows
    }
}

/// How many tiles of at least `tile` fit along `space`: at least one, at most
/// `cap`. A host too small for even one still gets one, because a gallery
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

/// Follow the host node: the grid is derived, so a resize has to reach the
/// paging and the keyboard before either answers again. A node not laid out
/// yet keeps the grid it has.
pub(crate) fn sync_gallery_layout(
    host: Query<&ComputedNode, With<GalleryHost>>,
    mut layout: ResMut<GalleryLayout>,
) {
    let Ok(node) = host.single() else {
        return;
    };
    let size = node.size() * node.inverse_scale_factor();
    if size.cmple(Vec2::ZERO).any() {
        return;
    }
    let wanted = GalleryLayout::for_viewport(size);
    if *layout != wanted {
        *layout = wanted;
    }
}

/// What the gallery is showing. The single source the overlay is rebuilt from,
/// so every control (mouse, keyboard, autopilot) drives the same seam.
#[derive(Resource, Clone, Debug, Default, PartialEq)]
pub struct GalleryState {
    /// Whether the overlay is up.
    pub open: bool,
    /// Active category filter.
    pub category: GalleryCategory,
    /// Case-insensitive name/id filter.
    pub filter: String,
    /// Whether the filter field has the caret. Typing reaches the filter only
    /// while it does, which is what leaves the letters free to be shortcuts.
    pub filter_focused: bool,
    /// Selection index into the FILTERED list, not the catalog.
    pub selected: usize,
    /// Whether the focus card is up for the selection.
    pub focused: bool,
}

impl GalleryState {
    /// The state a fresh browse opens on: unfiltered, on the grid, keeping
    /// the category the last browse left.
    pub fn opened(&self) -> Self {
        Self {
            open: true,
            category: self.category,
            ..default()
        }
    }

    /// Move the selection by `delta`, clamped to a list of `len` entries.
    /// Clamped rather than wrapped: paging off the end of the last page should
    /// stop there, not jump back to the first tile.
    pub fn step(&mut self, delta: isize, len: usize) {
        if len == 0 {
            self.selected = 0;
            return;
        }
        let last = len as isize - 1;
        self.selected = (self.selected as isize + delta).clamp(0, last) as usize;
    }

    /// The catalog id of the selected prototype, if the filtered list still has
    /// one there.
    pub fn selected_id(
        &self,
        sections: &GameSections,
        listing: Option<&GalleryListing>,
    ) -> Option<String> {
        let listed = catalog::browsable(sections, listing, self.category, &self.filter);
        let index = *listed.get(self.selected)?;
        Some(sections.get(index)?.base.id.clone())
    }

    /// The selected part's id when its entry may be placed: an unlisted host
    /// places everything, a listed part places only when `placeable`.
    pub fn placeable_id(
        &self,
        sections: &GameSections,
        listing: Option<&GalleryListing>,
    ) -> Option<String> {
        let id = self.selected_id(sections, listing)?;
        listing
            .is_none_or(|listing| listing.entry(&id).is_some_and(|entry| entry.placeable))
            .then_some(id)
    }
}

/// True while the gallery is up: the run condition for host systems that
/// must not act on input the gallery owns.
pub fn gallery_open(state: Res<GalleryState>) -> bool {
    state.open
}

/// The open gallery claims [`InputMode::Browse`]: its arrows and letters reach
/// the grid, and a host's Normal-only keys stay quiet under it.
fn claim_gallery_keyboard(
    state: Res<GalleryState>,
    host: Query<(), With<GalleryHost>>,
    mut claims: MessageWriter<ClaimKeyboard>,
) {
    if state.open && !host.is_empty() {
        claims.write(ClaimKeyboard(InputMode::Browse));
    }
}

/// The gallery's per-frame work, idle while no [`GalleryHost`] exists. A
/// host orders its own keys against this set; the gallery answers first
/// because it is on top.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct PartsGallerySystems;

/// Wires the gallery: its state, the part in hand, the keyboard claim, the
/// overlay, the tiles and the focus view. Each host adds it; a second add is
/// skipped, so two hosts in one app share one gallery.
pub struct PartsGalleryPlugin;

impl Plugin for PartsGalleryPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GalleryState>();
        app.init_resource::<GalleryLayout>();
        app.init_resource::<FocusView>();
        app.init_resource::<HeldPart>();
        app.add_message::<GalleryAlternate>();
        app.add_message::<ClaimKeyboard>();
        // A tile is a preview section whose meshes arrive as children; the
        // host's layers have to reach them.
        if !app.is_plugin_added::<HierarchyPropagatePlugin<RenderLayers>>() {
            app.add_plugins(HierarchyPropagatePlugin::<RenderLayers>::new(PostUpdate));
        }

        app.add_systems(PreUpdate, claim_gallery_keyboard.before(InputModeSystems));
        app.add_systems(
            Update,
            (
                // Browse is the gallery's own mode, so its keyboard answers
                // under Browse and under Normal and goes quiet under a mode
                // above it: a rebind waiting for a key gets the key.
                input::gallery_keyboard.run_if(owns_or_enters(InputMode::Browse)),
                // Both BEFORE the rebuild: the grid is laid out from one and
                // the tiles are spawned on the other, and either read a frame
                // late would draw the page the host used to hold.
                sync_gallery_layout,
                ui::rebuild_gallery,
                ui::paint_gallery_cells,
                scene::measure_gallery_items,
                scene::place_gallery_items,
                scene::pose_focused_item,
            )
                .chain()
                .in_set(PartsGallerySystems),
        );
        // The focus card's socket rings. Gated like the editor's own gizmo
        // group: registering a group adds the render-side system that turns
        // its storage into meshes, and a headless app has neither the plugin
        // nor the assets it wants.
        if app.is_plugin_added::<GizmoPlugin>() {
            app.init_gizmo_group::<GalleryGizmos>();
            app.add_systems(
                Update,
                (scene::sync_gallery_gizmo_layers, scene::draw_focus_sockets)
                    .chain()
                    .after(scene::pose_focused_item)
                    .in_set(PartsGallerySystems),
            );
        }
        // The focus view's zoom and orbit, split off so a headless rig with
        // no input plugin (and so no wheel or motion queue) still runs the
        // rest.
        app.add_systems(
            Update,
            scene::drive_focus_view
                .before(scene::place_gallery_items)
                .in_set(PartsGallerySystems)
                .run_if(resource_exists::<Messages<MouseMotion>>)
                .run_if(resource_exists::<Messages<MouseWheel>>),
        );
        app.configure_sets(
            Update,
            PartsGallerySystems.run_if(any_with_component::<GalleryHost>),
        );

        app.add_observer(ui::on_gallery_action);
    }
}

#[cfg(test)]
mod tests {
    use bevy::ui_widgets::Activate;

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
        let host = app
            .world_mut()
            .spawn((
                GalleryHost {
                    camera: Entity::PLACEHOLDER,
                    stage: Vec3::ZERO,
                    layers: RenderLayers::default(),
                    close_keys: "Esc",
                },
                ComputedNode {
                    size: Vec2::new(1024.0, 768.0),
                    inverse_scale_factor: 1.0,
                    ..default()
                },
            ))
            .id();
        app.add_systems(Update, sync_gallery_layout);

        app.update();
        let small = *app.world().resource::<GalleryLayout>();
        assert_eq!((small.cols(), small.rows()), (4, 3));

        app.world_mut()
            .entity_mut(host)
            .get_mut::<ComputedNode>()
            .expect("the host is still there")
            .size = Vec2::new(2560.0, 1440.0);
        app.update();

        let wide = *app.world().resource::<GalleryLayout>();
        assert!(
            wide.cols() > small.cols() && wide.rows() > small.rows(),
            "the wider host draws a wider grid (got {wide:?})"
        );
        assert_eq!(wide.page(), wide.cols() * wide.rows());
    }

    /// The open gallery claims Browse, so a host's Normal-only keys (the
    /// interface's close, the Ship pane's verbs, the editor's build keys) go
    /// quiet under it. With no host there is nothing on screen to claim for.
    #[test]
    fn an_open_gallery_with_a_host_claims_browse() {
        for (open, hosted, expected) in [
            (false, true, vec![]),
            (true, false, vec![]),
            (true, true, vec![InputMode::Browse]),
        ] {
            let mut world = World::new();
            world.insert_resource(GalleryState { open, ..default() });
            world.init_resource::<Messages<ClaimKeyboard>>();
            if hosted {
                world.spawn(GalleryHost {
                    camera: Entity::PLACEHOLDER,
                    stage: Vec3::ZERO,
                    layers: RenderLayers::default(),
                    close_keys: "Esc",
                });
            }

            bevy::ecs::system::RunSystemOnce::run_system_once(&mut world, claim_gallery_keyboard)
                .expect("the claim runs");

            let claimed: Vec<InputMode> = world
                .resource_mut::<Messages<ClaimKeyboard>>()
                .drain()
                .map(|claim| claim.0)
                .collect();
            assert_eq!(claimed, expected, "open {open} / hosted {hosted}");
        }
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
        assert_eq!(
            state.selected_id(&sections, None).as_deref(),
            Some("hauler_nose")
        );
        state.selected = 1;
        assert_eq!(
            state.selected_id(&sections, None).as_deref(),
            Some("hauler_tail")
        );
        // Out of range (a filter that just narrowed) resolves to nothing rather
        // than to a neighbour.
        state.selected = 7;
        assert_eq!(state.selected_id(&sections, None), None);
    }

    /// Clearing the filter widens the grid and leaves the category alone: the
    /// x beside the field answers "why is this grid short", and the chip row
    /// answers a different question.
    #[test]
    fn clearing_the_filter_keeps_the_category() {
        let mut world = World::new();
        world.insert_resource(catalog_of(&["hull_a"]));
        world.init_resource::<GalleryLayout>();
        world.init_resource::<HeldPart>();
        world.init_resource::<Messages<GalleryAlternate>>();
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
        world.init_resource::<HeldPart>();
        world.init_resource::<Messages<GalleryAlternate>>();
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
