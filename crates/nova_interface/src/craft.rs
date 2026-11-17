//! The Inventory pane's Craft view: the recipes the player ship can run.
//!
//! The Craft button at the right end of the Inventory pane's filter row opens
//! the view; [`manage_craft_view`] then puts a [`CraftViewHost`] node over the
//! Inventory pane body, the way the Ship pane's catalogue covers its pane.
//! The button shows docked and undocked, because ship recipes run anywhere.
//! Back or Esc returns to the Inventory view; a tab switch or the interface
//! closing closes the view. From the top:
//!
//! - The notice line: the last Craft or Cancel result ([`ActionNotice`]).
//! - The job row, while the player ship runs a craft job: the output, the
//!   batch it is on, the progress bar, the time left and Cancel. Cancel
//!   raises a [`CraftJobCancelRequest`]; the batches finished stay.
//! - The category row: All, then each category a recipe's output is in, and
//!   Back at its right end.
//! - The tile grid beside the focus card. A tile shows the output's category
//!   icon, its name, the count a batch makes and the site (`SHIP` or
//!   `STATION`). A tile one batch of which would not start is muted and
//!   names why: `Dock at a Craft station`, `Short of stone ore: 0 of 1`, `A
//!   craft is running`.
//! - The focus card, for the picked tile: each input as what the hold has
//!   over what a batch takes, the fee of a station recipe, the seconds a
//!   batch runs, the batch count with `-`, `+` and Max (what the hold and the
//!   balance cover), what the batches take and make, and Craft. A refused
//!   start greys Craft and shows the reason under the summary.
//!
//! Craft writes a [`CraftStartCommand`]; [`apply_craft_start_commands`]
//! checks it again by [`plan_craft_start`] and inserts the
//! [`ShipCraftJob`]. The job runs in `nova_ship` on the game clock, which
//! the interface holds, so a job moves only while the player flies. Recipe
//! inputs are counted from the hold less what running repair jobs hold back,
//! as the job's batch end counts them.
//!
//! Touch this module when changing what the Craft view shows, how it opens
//! and closes, or how it starts or cancels a craft.

use std::collections::BTreeSet;

use bevy::{
    input::mouse::MouseWheel,
    picking::hover::Hovered,
    prelude::*,
    ui_widgets::{Activate, Button},
};
use nova_gameplay::prelude::*;
use nova_ship::prelude::*;
use nova_sound::prelude::*;
use nova_ui::{
    screen::drive_wheel_scroll,
    theme::UiColorType,
    widget::{
        button, ButtonCueType, ButtonSpec, ThemedBorder, ThemedFill, ThemedRadius, ThemedText,
    },
};

use crate::{
    icons::{icon_node, InterfaceIcons},
    inventory::{category_color, category_label, chip_paint, FILTER_ORDER},
    pane::{
        compact_button, control_row, divider, notice_line, panel_paint, refusable_button,
        side_panel, themed_label, ActionNotice, ActionOutcomeType, InterfacePaneBody,
        InterfacePaneType,
    },
};

/// Glob-import surface: `use nova_interface::craft::prelude::*`.
pub mod prelude {
    pub use super::{
        craft_tile_name, CraftRuntime, CraftStartCommand, CRAFT_BACK, CRAFT_BUTTON, CRAFT_CANCEL,
        CRAFT_JOB_ROW, CRAFT_MAX, CRAFT_MORE, CRAFT_NOTICE, CRAFT_TILES, CRAFT_VIEW,
        INVENTORY_CRAFT,
    };
}

/// The name of the Inventory pane's Craft button, which opens the view.
pub const INVENTORY_CRAFT: &str = "InventoryCraft";
/// The name of the view's host node over the Inventory pane body.
pub const CRAFT_VIEW: &str = "CraftView";
/// The name of the view's Back button.
pub const CRAFT_BACK: &str = "CraftBack";
/// The name of the notice line.
pub const CRAFT_NOTICE: &str = "CraftNotice";
/// The name of the job row.
pub const CRAFT_JOB_ROW: &str = "CraftJobRow";
/// The name of the job row's Cancel button.
pub const CRAFT_CANCEL: &str = "CraftCancel";
/// The name of the tile grid.
pub const CRAFT_TILES: &str = "CraftTiles";
/// The name of the focus card's Craft button.
pub const CRAFT_BUTTON: &str = "CraftStart";
/// The name of the focus card's `+` button.
pub const CRAFT_MORE: &str = "CraftMore";
/// The name of the focus card's Max button.
pub const CRAFT_MAX: &str = "CraftMax";

/// The name of the tile of `recipe`.
pub fn craft_tile_name(recipe: &RecipeDesignId) -> String {
    format!("CraftTile-{recipe}")
}

/// The Craft view's state that outlives its host node.
#[derive(Resource, Debug, Default)]
pub struct CraftRuntime {
    /// The view covers the Inventory pane body. Set by the Craft button;
    /// cleared by Back, Esc, a tab switch and the interface closing.
    open: bool,
    /// The last Craft or Cancel result, shown on the notice line.
    pub notice: ActionNotice,
    /// The picked recipe; the first shown tile while none is.
    selected: Option<RecipeDesignId>,
    /// The batch count the focus card shows, at least 1.
    batches: u32,
    /// The category the tiles show; `None` shows every one.
    filter: Option<ItemCategoryType>,
    /// What Max sets: the batches the hold and the balance cover for the
    /// picked recipe, at least 1. Written each frame the view draws.
    max_batches: u32,
}

impl CraftRuntime {
    /// Whether the view covers the Inventory pane body.
    pub fn is_open(&self) -> bool {
        self.open
    }

    /// Open the view over the Inventory pane body.
    pub(crate) fn open(&mut self) {
        self.open = true;
    }

    /// Close the view. Returns whether it was open.
    pub(crate) fn close(&mut self) -> bool {
        std::mem::take(&mut self.open)
    }

    /// The picked recipe.
    pub fn selected(&self) -> Option<&RecipeDesignId> {
        self.selected.as_ref()
    }

    /// The batch count the focus card shows.
    pub fn batches(&self) -> u32 {
        self.batches.max(1)
    }
}

/// Start `batches` batches of `recipe` on the player ship, written by the
/// focus card's Craft and applied by [`apply_craft_start_commands`].
#[derive(Message, Clone, Debug, PartialEq, Eq)]
pub struct CraftStartCommand {
    /// The recipe.
    pub recipe: RecipeDesignId,
    /// Batches to run.
    pub batches: u32,
}

/// Marks the view's host node over the Inventory pane body.
#[derive(Component)]
pub(crate) struct CraftViewHost;

/// The notice line.
#[derive(Component)]
pub(crate) struct CraftNoticeMarker;

/// The job row, shown while the player ship runs a craft job.
#[derive(Component)]
pub(crate) struct CraftJobRowMarker;

/// A live part of the job row.
#[derive(Component, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CraftJobField {
    /// The output and the batch: `PDC round x100  batch 3/5`.
    Label,
    /// The game seconds left.
    Time,
}

/// The job row's progress bar fill.
#[derive(Component)]
pub(crate) struct CraftJobFill;

/// The category row, and the categories it was built for.
#[derive(Component, Default)]
pub(crate) struct CraftFilterBar(Option<Vec<ItemCategoryType>>);

/// A category chip; `None` is All.
#[derive(Component, Clone, Copy)]
pub(crate) struct CraftFilterChip(Option<ItemCategoryType>);

/// The tile grid, and the tiles it was built for.
#[derive(Component, Default)]
pub(crate) struct CraftTileGrid(Option<Vec<CraftTileView>>);

/// A tile, by its recipe.
#[derive(Component, Clone)]
pub(crate) struct CraftTile(RecipeDesignId);

/// The focus card, and the card it was built for.
#[derive(Component, Default)]
pub(crate) struct CraftFocusCard(Option<Option<CraftCardView>>);

/// A focus card batch button.
#[derive(Component, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CraftBatchStepType {
    /// One batch fewer, down to 1.
    Less,
    /// One batch more, up to [`CRAFT_BATCHES_MAX`].
    More,
    /// What the hold and the balance cover.
    Max,
}

/// What the view's checks read off the player ship.
pub(crate) struct CraftSite {
    /// The hold less what running repair jobs hold back.
    hold: ShipInventory,
    /// The ship's balance.
    own_cr: u32,
    /// Docked at a living station that offers Craft.
    at_station: bool,
    /// A craft job runs.
    running: bool,
}

/// One tile as drawn.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CraftTileView {
    recipe: RecipeDesignId,
    name: String,
    category: ItemCategoryType,
    /// `x100  SHIP`.
    yield_line: String,
    /// Why one batch would not start.
    refusal: Option<String>,
    selected: bool,
}

/// The focus card as drawn.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CraftCardView {
    recipe: RecipeDesignId,
    /// `PDC round x100`.
    title: String,
    /// `SHIP` or `STATION`.
    site: &'static str,
    /// Each input: its name, then `have / need a batch`.
    inputs: Vec<(String, String)>,
    /// `30 cr a batch` for a station recipe.
    fee: Option<String>,
    /// `8 s a batch`.
    duration: String,
    batches: u32,
    /// `Takes 5 iron ore, 5 stone ore` and, for a station recipe, the fee.
    takes: String,
    /// `Makes 500 PDC round in 40 s`.
    makes: String,
    /// Why Craft is refused.
    refusal: Option<String>,
}

/// The categories, in filter order, the outputs of `recipes` are in.
fn craft_categories(recipes: &GameRecipes, items: &GameItems) -> Vec<ItemCategoryType> {
    let present: BTreeSet<usize> = recipes
        .iter()
        .filter_map(|recipe| {
            let category = items.design(recipe.output_item()).category;
            FILTER_ORDER.iter().position(|each| *each == category)
        })
        .collect();
    present
        .into_iter()
        .map(|index| FILTER_ORDER[index])
        .collect()
}

/// The recipes the tiles show, ship recipes first, each by id: those whose
/// output is in `filter`, or every one.
fn shown_recipes<'a>(
    recipes: &'a GameRecipes,
    items: &GameItems,
    filter: Option<ItemCategoryType>,
) -> Vec<&'a RecipeDesign> {
    let mut shown: Vec<&RecipeDesign> = recipes
        .iter()
        .filter(|recipe| {
            filter.is_none_or(|category| items.design(recipe.output_item()).category == category)
        })
        .collect();
    shown.sort_by_key(|recipe| (recipe.site.fee_cr() > 0, recipe.id.clone()));
    shown
}

/// The tile of `recipe` at `site`.
fn tile_view(
    recipe: &RecipeDesign,
    items: &GameItems,
    site: &CraftSite,
    selected: bool,
) -> CraftTileView {
    let output = items.design(recipe.output_item());
    CraftTileView {
        recipe: recipe.id.clone(),
        name: output.name.clone(),
        category: output.category,
        yield_line: format!("x{}  {}", recipe.output_count(), recipe.site.tag()),
        refusal: plan_craft_start(
            recipe,
            1,
            site.running,
            site.at_station,
            &site.hold,
            site.own_cr,
        )
        .err()
        .map(|refusal| refusal.reason(items)),
        selected,
    }
}

/// The focus card of `batches` batches of `recipe` at `site`.
fn card_view(
    recipe: &RecipeDesign,
    items: &GameItems,
    site: &CraftSite,
    batches: u32,
) -> CraftCardView {
    let output = items.design(recipe.output_item());
    let fee_cr = recipe.site.fee_cr();
    let mut takes: Vec<String> = recipe
        .inputs
        .iter()
        .map(|(item, count)| {
            format!(
                "{} {}",
                u64::from(*count) * u64::from(batches),
                items.design(item).name.to_lowercase()
            )
        })
        .collect();
    if fee_cr > 0 {
        takes.push(cr_text(u64::from(fee_cr) * u64::from(batches)));
    }
    CraftCardView {
        recipe: recipe.id.clone(),
        title: format!("{} x{}", output.name, recipe.output_count()),
        site: recipe.site.tag(),
        inputs: recipe
            .inputs
            .iter()
            .map(|(item, count)| {
                (
                    items.design(item).name.clone(),
                    format!("{} / {count}", site.hold.count(item)),
                )
            })
            .collect(),
        fee: (fee_cr > 0).then(|| format!("{} a batch", cr_text(u64::from(fee_cr)))),
        duration: format!("{} s a batch", recipe.duration_secs),
        batches,
        takes: format!("Takes {}", takes.join(", ")),
        makes: format!(
            "Makes {} {} in {} s",
            u64::from(recipe.output_count()) * u64::from(batches),
            output.name,
            recipe.duration_secs * batches as f32
        ),
        refusal: plan_craft_start(
            recipe,
            batches,
            site.running,
            site.at_station,
            &site.hold,
            site.own_cr,
        )
        .err()
        .map(|refusal| refusal.reason(items)),
    }
}

/// The player ship and what the view reads off it.
type PlayerCraftQuery<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static ShipInventory,
        &'static ShipCredits,
        Option<&'static ShipRepairJobs>,
        Option<&'static ShipCraftJob>,
        Option<&'static DockedShip>,
    ),
    (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>),
>;

/// The player ship's [`CraftSite`], with the ship, or `None` with no player
/// ship or more than one.
fn player_site(
    q_player: &PlayerCraftQuery,
    stations: &CraftStations,
) -> Option<(Entity, CraftSite)> {
    let (ship, hold, credits, repairs, craft, docked) = q_player.single().ok()?;
    Some((
        ship,
        CraftSite {
            hold: repairs.map_or_else(|| hold.clone(), |repairs| repairs.free_hold(hold)),
            own_cr: credits.0,
            at_station: stations.docked_at(ship, docked).is_some(),
            running: craft.is_some(),
        },
    ))
}

/// The Inventory pane's Craft button, at the right end of its filter row and
/// styled as the Ship pane's Catalogue button.
pub(crate) fn spawn_craft_open_button(bar: &mut ChildSpawnerCommands) {
    bar.spawn(Node {
        margin: UiRect::left(Val::Auto),
        flex_shrink: 0.0,
        ..default()
    })
    .with_children(|slot| {
        slot.spawn((
            Name::new(INVENTORY_CRAFT),
            button(ButtonSpec::new("Craft").fit()),
        ))
        .observe(on_craft_open);
    });
}

/// Put the view over the Inventory pane body while it is open, and take it
/// away otherwise. The view closes when the Inventory pane is not shown: a
/// tab switch despawns the host with the pane body, and the interface
/// closing despawns the body.
pub(crate) fn manage_craft_view(
    mut commands: Commands,
    mut runtime: ResMut<CraftRuntime>,
    pane: Res<InterfacePaneType>,
    q_body: Query<Entity, With<InterfacePaneBody>>,
    q_host: Query<Entity, With<CraftViewHost>>,
) {
    let body = q_body
        .single()
        .ok()
        .filter(|_| *pane == InterfacePaneType::Inventory);
    if body.is_none() && runtime.open {
        runtime.open = false;
    }
    let Some(body) = body.filter(|_| runtime.open) else {
        for host in &q_host {
            commands.entity(host).despawn();
        }
        return;
    };
    if !q_host.is_empty() {
        return;
    }
    commands
        .spawn((
            CraftViewHost,
            Name::new(CRAFT_VIEW),
            Node {
                position_type: PositionType::Absolute,
                left: px(0),
                right: px(0),
                top: px(0),
                bottom: px(0),
                flex_direction: FlexDirection::Column,
                row_gap: px(10),
                padding: UiRect::all(px(12)),
                ..default()
            },
            // Opaque, so the Inventory view under it does not show through,
            // and above the Inventory body's children for picking.
            BackgroundColor(Color::NONE),
            ThemedFill::new(UiColorType::Surface),
            ZIndex(10),
            ChildOf(body),
        ))
        .with_children(craft_body);
}

/// Build the Craft view: the notice line, the job row, then the category row
/// with Back and the tiles beside the focus card. The reconcilers fill them.
pub(crate) fn craft_body(body: &mut ChildSpawnerCommands) {
    body.spawn((CraftNoticeMarker, Name::new(CRAFT_NOTICE), notice_line()));
    spawn_job_row(body);
    body.spawn(Node {
        flex_grow: 1.0,
        min_height: px(0),
        flex_direction: FlexDirection::Row,
        column_gap: px(12),
        ..default()
    })
    .with_children(|split| {
        split
            .spawn(Node {
                flex_grow: 1.0,
                flex_basis: px(0),
                min_width: px(0),
                min_height: px(0),
                flex_direction: FlexDirection::Column,
                row_gap: px(10),
                ..default()
            })
            .with_children(|left| {
                left.spawn(Node {
                    column_gap: px(8),
                    ..control_row(JustifyContent::FlexStart)
                })
                .with_children(|row| {
                    row.spawn((
                        CraftFilterBar::default(),
                        Node {
                            flex_grow: 1.0,
                            column_gap: px(8),
                            ..control_row(JustifyContent::FlexStart)
                        },
                    ));
                    row.spawn((Name::new(CRAFT_BACK), button(ButtonSpec::new("Back").fit())))
                        .observe(on_craft_back);
                });
                left.spawn((
                    CraftTileGrid::default(),
                    Name::new(CRAFT_TILES),
                    Node {
                        flex_grow: 1.0,
                        min_height: px(0),
                        flex_direction: FlexDirection::Row,
                        flex_wrap: FlexWrap::Wrap,
                        align_content: AlignContent::FlexStart,
                        column_gap: px(10),
                        row_gap: px(10),
                        padding: UiRect::all(px(12)),
                        border: UiRect::all(px(1)),
                        overflow: Overflow::scroll_y(),
                        ..default()
                    },
                    panel_paint(),
                    ThemedRadius::control(),
                    ScrollPosition::default(),
                    Hovered::default(),
                ));
            });
        split.spawn((
            CraftFocusCard::default(),
            Name::new("CraftCard"),
            side_panel(Overflow::clip()),
        ));
    });
}

/// Open the view over the Inventory pane body.
fn on_craft_open(_activate: On<Activate>, mut runtime: ResMut<CraftRuntime>) {
    runtime.open();
}

/// Return to the Inventory view.
fn on_craft_back(_activate: On<Activate>, mut runtime: ResMut<CraftRuntime>) {
    runtime.close();
}

/// The job row: the label, the time left and Cancel over the progress bar.
fn spawn_job_row(body: &mut ChildSpawnerCommands) {
    body.spawn((
        CraftJobRowMarker,
        Name::new(CRAFT_JOB_ROW),
        Node {
            display: Display::None,
            flex_direction: FlexDirection::Column,
            row_gap: px(3),
            flex_shrink: 0.0,
            ..default()
        },
    ))
    .with_children(|row| {
        row.spawn(control_row(JustifyContent::FlexStart))
            .with_children(|line| {
                line.spawn((
                    CraftJobField::Label,
                    themed_label("", 13.0, UiColorType::Text),
                    TextLayout::new(Justify::Left, LineBreak::NoWrap),
                    Node {
                        flex_grow: 1.0,
                        flex_basis: px(0),
                        min_width: px(0),
                        overflow: Overflow::clip(),
                        ..default()
                    },
                ));
                line.spawn((
                    CraftJobField::Time,
                    themed_label("", 12.0, UiColorType::TextBody),
                    TextLayout::new(Justify::Right, LineBreak::NoWrap),
                    Node {
                        flex_shrink: 0.0,
                        ..default()
                    },
                ));
                line.spawn((
                    Name::new(CRAFT_CANCEL),
                    compact_button(ButtonSpec::new("Cancel").fit().ghost()),
                ))
                .observe(on_craft_cancel);
            });
        row.spawn((
            Node {
                height: px(6),
                flex_shrink: 0.0,
                overflow: Overflow::clip(),
                ..default()
            },
            BackgroundColor(Color::NONE),
            ThemedFill::alpha(UiColorType::TextSecondary, 0.25),
            ThemedRadius::control(),
        ))
        .with_children(|track| {
            track.spawn((
                CraftJobFill,
                Node {
                    width: percent(0),
                    height: percent(100),
                    ..default()
                },
                BackgroundColor(Color::NONE),
                ThemedFill::new(UiColorType::Accent),
            ));
        });
    });
}

/// Show the job row while the player ship runs a craft job, and fill its
/// label, time and bar.
#[expect(clippy::type_complexity, reason = "the row, its texts and its fill")]
pub(crate) fn update_craft_job_row(
    items: Option<Res<GameItems>>,
    recipes: Option<Res<GameRecipes>>,
    q_player: Query<
        Option<&ShipCraftJob>,
        (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>),
    >,
    mut q_row: Query<&mut Node, (With<CraftJobRowMarker>, Without<CraftJobFill>)>,
    mut q_text: Query<(&CraftJobField, &mut Text)>,
    mut q_fill: Query<&mut Node, (With<CraftJobFill>, Without<CraftJobRowMarker>)>,
) {
    let job = q_player.single().ok().flatten();
    let shown = job.zip(items.as_deref()).zip(recipes.as_deref());
    let display = if shown.is_some() {
        Display::Flex
    } else {
        Display::None
    };
    for mut row in &mut q_row {
        if row.display != display {
            row.display = display;
        }
    }
    let Some(((job, items), recipes)) = shown else {
        return;
    };
    let recipe = recipes.design(&job.recipe);
    let output = items.design(recipe.output_item());
    for (field, mut text) in &mut q_text {
        let value = match field {
            CraftJobField::Label => format!(
                "{} x{}  batch {}/{}",
                output.name,
                recipe.output_count(),
                (job.done + 1).min(job.queued),
                job.queued
            ),
            CraftJobField::Time => format!("{:.0} s left", job.remaining_secs(recipe).ceil()),
        };
        if text.0 != value {
            text.0 = value;
        }
    }
    let width = percent(job.progress(recipe) * 100.0);
    for mut fill in &mut q_fill {
        if fill.width != width {
            fill.width = width;
        }
    }
}

/// Fill the category row, the tiles and the focus card from the recipes and
/// the player ship, rebuilding each only when what it shows changes.
/// A container a new view spawned starts with nothing drawn, so it fills
/// on its first frame.
#[expect(
    clippy::too_many_arguments,
    reason = "the catalogs, the ship, the stations and the three containers"
)]
pub(crate) fn update_craft_view(
    mut commands: Commands,
    items: Option<Res<GameItems>>,
    recipes: Option<Res<GameRecipes>>,
    icons: Res<InterfaceIcons>,
    mut runtime: ResMut<CraftRuntime>,
    q_player: PlayerCraftQuery,
    stations: CraftStations,
    mut q_bar: Query<(Entity, &mut CraftFilterBar)>,
    mut q_grid: Query<(Entity, &mut CraftTileGrid)>,
    mut q_card: Query<(Entity, &mut CraftFocusCard)>,
) {
    let (Some(items), Some(recipes)) = (items.as_deref(), recipes.as_deref()) else {
        return;
    };
    let Some((_, site)) = player_site(&q_player, &stations) else {
        return;
    };
    let categories = craft_categories(recipes, items);
    if runtime
        .filter
        .is_some_and(|filter| !categories.contains(&filter))
    {
        runtime.filter = None;
    }
    let shown = shown_recipes(recipes, items, runtime.filter);
    if !runtime
        .selected
        .as_ref()
        .is_some_and(|selected| shown.iter().any(|recipe| recipe.id == *selected))
    {
        runtime.selected = shown.first().map(|recipe| recipe.id.clone());
        runtime.batches = 1;
    }
    let picked = runtime
        .selected
        .as_ref()
        .map(|selected| recipes.design(selected));
    runtime.batches = runtime.batches.clamp(1, CRAFT_BATCHES_MAX);
    runtime.max_batches = picked.map_or(1, |recipe| {
        craft_batches_max(recipe, &site.hold, site.own_cr).max(1)
    });

    for (bar, mut drawn) in &mut q_bar {
        let key = categories.clone();
        if drawn.0.as_ref() != Some(&key) {
            commands.entity(bar).despawn_children();
            commands.entity(bar).with_children(|bar| {
                filter_chip(bar, None, &icons);
                for category in &key {
                    filter_chip(bar, Some(*category), &icons);
                }
            });
            drawn.0 = Some(key);
        }
    }
    let tiles: Vec<CraftTileView> = shown
        .iter()
        .map(|recipe| {
            tile_view(
                recipe,
                items,
                &site,
                runtime.selected.as_ref() == Some(&recipe.id),
            )
        })
        .collect();
    for (grid, mut drawn) in &mut q_grid {
        if drawn.0.as_ref() != Some(&tiles) {
            commands.entity(grid).despawn_children();
            commands.entity(grid).with_children(|grid| {
                for tile in &tiles {
                    spawn_tile(grid, tile, &icons);
                }
            });
            drawn.0 = Some(tiles.clone());
        }
    }
    let card = picked.map(|recipe| card_view(recipe, items, &site, runtime.batches));
    for (panel, mut drawn) in &mut q_card {
        if drawn.0.as_ref() != Some(&card) {
            commands.entity(panel).despawn_children();
            commands.entity(panel).with_children(|panel| match &card {
                Some(card) => spawn_card(panel, card),
                None => {
                    panel.spawn(themed_label(
                        "No recipe is loaded.",
                        13.0,
                        UiColorType::TextMuted,
                    ));
                }
            });
            drawn.0 = Some(card.clone());
        }
    }
}

/// One category chip: the category icon and word, or `All`.
fn filter_chip(
    bar: &mut ChildSpawnerCommands,
    chip: Option<ItemCategoryType>,
    icons: &InterfaceIcons,
) {
    let label = chip.map_or("All", category_label);
    let (color, fill, border) = chip_paint(false);
    bar.spawn((
        Name::new(format!("CraftFilter{label}")),
        Button,
        ButtonCueType::Owned,
        CraftFilterChip(chip),
        Node {
            height: px(26),
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            column_gap: px(4),
            padding: UiRect::axes(px(8), px(0)),
            border: UiRect::all(px(1)),
            ..default()
        },
        BackgroundColor(Color::NONE),
        ThemedFill::alpha(color, fill),
        BorderColor::all(Color::NONE),
        ThemedBorder::alpha(color, border),
        ThemedRadius::control(),
    ))
    .observe(on_craft_filter_chip)
    .with_children(|chip_node| {
        if let Some(category) = chip {
            chip_node.spawn(icon_node(
                icons.category(category),
                category_color(category),
                16.0,
            ));
        }
        chip_node.spawn((
            themed_label(label, 12.0, UiColorType::TextBody),
            Pickable::IGNORE,
        ));
    });
}

/// Width of one tile, in logical px.
const TILE_PX: f32 = 150.0;

/// One tile: the category icon, the name, the yield line and, refused, the
/// reason. A refused tile is muted but still picks its recipe.
fn spawn_tile(grid: &mut ChildSpawnerCommands, tile: &CraftTileView, icons: &InterfaceIcons) {
    let muted = tile.refusal.is_some();
    let (color, fill, border) = match (tile.selected, muted) {
        (true, _) => (UiColorType::Accent, 0.25, 1.0),
        (false, false) => (UiColorType::TextSecondary, 0.1, 0.6),
        (false, true) => (UiColorType::TextSecondary, 0.04, 0.25),
    };
    let text = |color| if muted { UiColorType::TextMuted } else { color };
    grid.spawn((
        Name::new(craft_tile_name(&tile.recipe)),
        Button,
        ButtonCueType::Owned,
        CraftTile(tile.recipe.clone()),
        Node {
            width: px(TILE_PX),
            min_height: px(96),
            flex_direction: FlexDirection::Column,
            row_gap: px(4),
            padding: UiRect::all(px(8)),
            border: UiRect::all(px(1)),
            ..default()
        },
        BackgroundColor(Color::NONE),
        ThemedFill::alpha(color, fill),
        BorderColor::all(Color::NONE),
        ThemedBorder::alpha(color, border),
        ThemedRadius::control(),
    ))
    .observe(on_craft_tile)
    .with_children(|cell| {
        let tint = if muted {
            UiColorType::TextMuted
        } else {
            category_color(tile.category)
        };
        cell.spawn(icon_node(icons.category(tile.category), tint, 22.0));
        cell.spawn((
            themed_label(&tile.name, 13.0, text(UiColorType::Text)),
            Pickable::IGNORE,
        ));
        cell.spawn((
            themed_label(&tile.yield_line, 11.0, text(UiColorType::TextBody)),
            TextLayout::new(Justify::Left, LineBreak::NoWrap),
            Pickable::IGNORE,
        ));
        if let Some(why) = &tile.refusal {
            cell.spawn((
                themed_label(why, 11.0, UiColorType::TextMuted),
                Pickable::IGNORE,
            ));
        }
    });
}

/// The focus card's rows for `card`.
fn spawn_card(panel: &mut ChildSpawnerCommands, card: &CraftCardView) {
    panel
        .spawn(control_row(JustifyContent::SpaceBetween))
        .with_children(|head| {
            head.spawn(themed_label(&card.title, 14.0, UiColorType::Text));
            head.spawn(themed_label(card.site, 11.0, UiColorType::TextSecondary));
        });
    panel.spawn(divider());
    for (name, count) in &card.inputs {
        panel
            .spawn(control_row(JustifyContent::SpaceBetween))
            .with_children(|row| {
                row.spawn(themed_label(name, 12.0, UiColorType::TextBody));
                row.spawn(themed_label(count, 12.0, UiColorType::Text));
            });
    }
    if let Some(fee) = &card.fee {
        panel.spawn(themed_label(fee, 12.0, UiColorType::TextBody));
    }
    panel.spawn(themed_label(&card.duration, 12.0, UiColorType::TextBody));
    panel.spawn(divider());
    panel
        .spawn(control_row(JustifyContent::FlexStart))
        .with_children(|row| {
            row.spawn(themed_label("Batches", 12.0, UiColorType::TextBody));
            for (step, label, name) in [
                (CraftBatchStepType::Less, "-", "CraftLess"),
                (CraftBatchStepType::More, "+", CRAFT_MORE),
                (CraftBatchStepType::Max, "Max", CRAFT_MAX),
            ] {
                if step == CraftBatchStepType::More {
                    row.spawn((
                        themed_label(&card.batches.to_string(), 13.0, UiColorType::Text),
                        TextLayout::new(Justify::Center, LineBreak::NoWrap),
                        Node {
                            min_width: px(28),
                            justify_content: JustifyContent::Center,
                            ..default()
                        },
                    ));
                }
                row.spawn((
                    Name::new(name),
                    step,
                    compact_button(ButtonSpec::new(label).fit().ghost()),
                ))
                .observe(on_craft_batch_step);
            }
        });
    panel.spawn(themed_label(&card.takes, 12.0, UiColorType::TextBody));
    panel.spawn(themed_label(&card.makes, 12.0, UiColorType::Text));
    panel.spawn(themed_label(
        card.refusal.as_deref().unwrap_or(""),
        12.0,
        UiColorType::Danger,
    ));
    let recipe = card.recipe.clone();
    let batches = card.batches;
    refusable_button(
        panel,
        CRAFT_BUTTON.to_string(),
        "Craft",
        card.refusal.as_deref(),
    )
    .observe(
        move |_: On<Activate>, mut starts: MessageWriter<CraftStartCommand>| {
            starts.write(CraftStartCommand {
                recipe: recipe.clone(),
                batches,
            });
        },
    );
}

/// Show the category a chip picks.
fn on_craft_filter_chip(
    activate: On<Activate>,
    q_chip: Query<&CraftFilterChip>,
    mut runtime: ResMut<CraftRuntime>,
    bank: Option<Res<SoundBank<UiSfx>>>,
    mut commands: Commands,
) {
    let Ok(chip) = q_chip.get(activate.entity) else {
        return;
    };
    if runtime.filter != chip.0 {
        runtime.filter = chip.0;
        play_menu_select(&mut commands, bank.as_deref());
    }
}

/// Pick a tile's recipe at one batch.
fn on_craft_tile(
    activate: On<Activate>,
    q_tile: Query<&CraftTile>,
    mut runtime: ResMut<CraftRuntime>,
    bank: Option<Res<SoundBank<UiSfx>>>,
    mut commands: Commands,
) {
    let Ok(tile) = q_tile.get(activate.entity) else {
        return;
    };
    if runtime.selected.as_ref() != Some(&tile.0) {
        runtime.selected = Some(tile.0.clone());
        runtime.batches = 1;
        play_menu_select(&mut commands, bank.as_deref());
    }
}

/// Step the batch count: `-` down to 1, `+` up to [`CRAFT_BATCHES_MAX`],
/// Max to what the hold and the balance cover.
fn on_craft_batch_step(
    activate: On<Activate>,
    q_step: Query<&CraftBatchStepType>,
    mut runtime: ResMut<CraftRuntime>,
) {
    let Ok(step) = q_step.get(activate.entity) else {
        return;
    };
    let batches = runtime.batches();
    runtime.batches = match step {
        CraftBatchStepType::Less => batches.saturating_sub(1).max(1),
        CraftBatchStepType::More => (batches + 1).min(CRAFT_BATCHES_MAX),
        CraftBatchStepType::Max => runtime.max_batches.max(1),
    };
}

/// Cancel the player ship's craft job and say what it made.
fn on_craft_cancel(
    _activate: On<Activate>,
    items: Option<Res<GameItems>>,
    recipes: Option<Res<GameRecipes>>,
    q_player: Query<(Entity, Option<&ShipCraftJob>), With<PlayerSpaceshipMarker>>,
    mut runtime: ResMut<CraftRuntime>,
    mut commands: Commands,
) {
    let Ok((ship, Some(job))) = q_player.single() else {
        return;
    };
    let (Some(items), Some(recipes)) = (items.as_deref(), recipes.as_deref()) else {
        return;
    };
    let recipe = recipes.design(&job.recipe);
    let made = u64::from(job.done) * u64::from(recipe.output_count());
    commands.trigger(CraftJobCancelRequest { entity: ship });
    runtime.notice.say(
        ActionOutcomeType::Done,
        format!(
            "Cancelled the craft: {made} {} made",
            items.design(recipe.output_item()).name
        ),
    );
}

/// Apply each [`CraftStartCommand`]: check it again against the live ship
/// by [`plan_craft_start`] and insert the [`ShipCraftJob`], or say why not.
/// The Craft button is greyed on the same check, so a refusal here means the
/// ship changed between the draw and the press.
pub(crate) fn apply_craft_start_commands(
    mut starts: MessageReader<CraftStartCommand>,
    items: Option<Res<GameItems>>,
    recipes: Option<Res<GameRecipes>>,
    q_player: PlayerCraftQuery,
    stations: CraftStations,
    mut runtime: ResMut<CraftRuntime>,
    mut commands: Commands,
) {
    let (Some(items), Some(recipes)) = (items.as_deref(), recipes.as_deref()) else {
        starts.clear();
        return;
    };
    for start in starts.read() {
        let Some((ship, site)) = player_site(&q_player, &stations) else {
            continue;
        };
        let Some(recipe) = recipes.get(&start.recipe) else {
            continue;
        };
        match plan_craft_start(
            recipe,
            start.batches,
            site.running,
            site.at_station,
            &site.hold,
            site.own_cr,
        ) {
            Ok(()) => {
                commands
                    .entity(ship)
                    .insert(ShipCraftJob::new(recipe.id.clone(), start.batches));
                runtime.batches = 1;
                runtime.notice.say(
                    ActionOutcomeType::Done,
                    format!(
                        "Crafting {} {}: {} s on the flight clock",
                        u64::from(recipe.output_count()) * u64::from(start.batches),
                        items.design(recipe.output_item()).name,
                        recipe.duration_secs * start.batches as f32
                    ),
                );
                // One start a pass: the query does not see the job inserted
                // above, so a second start would pass the Running check.
                starts.clear();
                return;
            }
            Err(refusal) => {
                runtime
                    .notice
                    .say(ActionOutcomeType::Refused, refusal.reason(items));
            }
        }
    }
}

/// Wheel-scroll the tiles while the pointer is over them.
pub(crate) fn scroll_craft_tiles(
    mut wheel: MessageReader<MouseWheel>,
    q_grid: Query<
        (&mut ScrollPosition, Option<&ComputedNode>, Option<&Hovered>),
        With<CraftTileGrid>,
    >,
) {
    if !q_grid
        .iter()
        .any(|(_, _, hovered)| hovered.is_some_and(Hovered::get))
    {
        wheel.clear();
        return;
    }
    drive_wheel_scroll(wheel, q_grid);
}

/// Count the notice down on the real clock, paint it in its outcome's tone
/// and play the cue a result queued.
pub(crate) fn update_craft_notice(
    time: Res<Time<Real>>,
    mut runtime: ResMut<CraftRuntime>,
    bank: Option<Res<SoundBank<UiSfx>>>,
    mut commands: Commands,
    mut q_notice: Query<(&mut Text, &mut ThemedText, &mut Node), With<CraftNoticeMarker>>,
) {
    runtime
        .notice
        .tick(time.delta_secs(), bank.as_deref(), &mut commands);
    let (note, tone) = runtime.notice.line();
    // An empty line takes no room, so no blank band sits over the filter bar.
    let display = if note.is_empty() {
        Display::None
    } else {
        Display::Flex
    };
    for (mut text, mut themed, mut node) in &mut q_notice {
        if text.0 != note {
            text.0 = note.to_string();
        }
        if themed.color != tone {
            themed.color = tone;
        }
        if node.display != display {
            node.display = display;
        }
    }
}

/// Drives the Craft view.
pub(crate) struct CraftViewPlugin;

impl Plugin for CraftViewPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CraftRuntime>();
        app.add_message::<CraftStartCommand>();
        // Every view system reads the item and recipe catalogs, which land
        // only at the mod merge.
        app.configure_sets(
            Update,
            CraftViewSystems
                .run_if(resource_exists::<GameItems>)
                .run_if(resource_exists::<GameRecipes>),
        );
        app.add_systems(
            Update,
            (
                manage_craft_view,
                apply_craft_start_commands,
                scroll_craft_tiles,
                update_craft_job_row,
                update_craft_view,
                update_craft_notice,
            )
                .chain()
                .in_set(CraftViewSystems),
        );
    }
}

/// System set for the Craft view's per-frame work. Runs after the interface
/// builds its pane body (`InterfaceSystems`).
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct CraftViewSystems;

#[cfg(test)]
mod tests;
