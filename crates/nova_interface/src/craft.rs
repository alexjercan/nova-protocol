//! The Inventory pane's Craft view: the recipes the player ship can run.
//!
//! The Craft button at the right end of the Inventory pane's filter row opens
//! the view; [`manage_craft_view`] then puts a [`CraftViewHost`] node over the
//! Inventory pane body, the way the Ship pane's catalogue covers its pane.
//! The button shows docked and undocked, because ship recipes run anywhere.
//! Back or Esc returns to the Inventory view; a tab switch or the interface
//! closing closes the view. On the left:
//!
//! - The head row: the category chips (All, then each category a recipe's
//!   output is in), and Back at its right end.
//! - The tile grid, whose columns share the width evenly. A tile shows the
//!   output's category icon, its name, the count a batch makes and the site
//!   (`SHIP` or `STATION`). A tile one batch of which would not start is
//!   muted. A station recipe the docked Craft station is below the
//!   advancement of has no tile.
//!
//! On the right, the card:
//!
//! - The Running block, while the player ship runs a craft job: the output,
//!   the batch it is on and the time left, the progress bar and Cancel.
//!   Cancel raises a [`CraftJobCancelRequest`]; the batches finished stay.
//! - The picked tile's recipe for the batch count: the output and the count
//!   the batches make, then each input as what the hold has over what the
//!   batches take, the fee of a station recipe for the batches, and the
//!   seconds the batches run. A count the ship is short of, and a fee over
//!   the balance, is painted in the danger tone.
//! - The Batches block: one amount row (the field, "/ most", the slider and
//!   Max) from 1 to the most the hold and the balance cover, and Craft at its
//!   bottom right, greyed while the start would be refused. The block is
//!   spawned once and written in place, so a rebuild cannot drop the slider
//!   mid-drag.
//!
//! The view writes no result line: the job row, the tiles and the hold show
//! a start, a cancel and a refusal. Craft and a refused start play the
//! outcome cue.
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
    prelude::{
        amount_row, bar, button, chip_bar, control_row, inspector, metrics, panel_paint,
        spawn_block, spawn_kv, text, text_toned, verb_row, Amount, AmountChanged, AmountField,
        AmountSpec, Bar, ButtonCueType, ButtonSpec, RowAlign, Selected, Short, TextRole, VerbGate,
    },
    screen::drive_wheel_scroll,
    theme::UiColorType,
    widget::{TextFieldFocused, ThemedBorder, ThemedFill, ThemedRadius},
};

use crate::{
    icons::{icon_node, InterfaceIcons},
    inventory::{category_chip, category_color, select_chip, FILTER_ORDER},
    pane::{play_outcome_cue, ActionOutcomeType, InterfacePaneBody, InterfacePaneType},
};

/// Glob-import surface: `use nova_interface::craft::prelude::*`.
pub mod prelude {
    pub use super::{
        craft_tile_name, CraftRuntime, CraftStartCommand, CRAFT_BACK, CRAFT_BATCHES, CRAFT_BUTTON,
        CRAFT_CANCEL, CRAFT_JOB_ROW, CRAFT_MAX, CRAFT_SLIDER, CRAFT_TILES, CRAFT_VIEW,
        INVENTORY_CRAFT,
    };
}

/// The name of the Inventory pane's Craft button, which opens the view.
pub const INVENTORY_CRAFT: &str = "InventoryCraft";
/// The name of the view's host node over the Inventory pane body.
pub const CRAFT_VIEW: &str = "CraftView";
/// The name of the view's Back button.
pub const CRAFT_BACK: &str = "CraftBack";
/// The name of the card's Running block.
pub const CRAFT_JOB_ROW: &str = "CraftJobRow";
/// The name of the Running block's Cancel button.
pub const CRAFT_CANCEL: &str = "CraftCancel";
/// The name of the tile grid.
pub const CRAFT_TILES: &str = "CraftTiles";
/// The name of the Batches block's Craft button.
pub const CRAFT_BUTTON: &str = "CraftStart";
/// The name of the batch amount row's field.
pub const CRAFT_BATCHES: &str = "CraftField";
/// The name of the batch amount row's slider.
pub const CRAFT_SLIDER: &str = "CraftSlider";
/// The name of the batch amount row's Max button.
pub const CRAFT_MAX: &str = "CraftMax";
/// The name prefix of the batch amount row's parts.
const CRAFT_AMOUNT: &str = "Craft";

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
    /// The cue the last start owes: Done for a job inserted, Refused for a
    /// start [`plan_craft_start`] refused. Taken by [`play_craft_cue`] once
    /// the sound bank lands.
    cue: Option<ActionOutcomeType>,
    /// The picked recipe; the first shown tile while none is.
    selected: Option<RecipeDesignId>,
    /// The batch count the batch form shows, from 1 to `max_batches`.
    batches: u32,
    /// The category the tiles show; `None` shows every one.
    filter: Option<ItemCategoryType>,
    /// The amount row's top: the batches the hold and the balance cover for
    /// the picked recipe, at most [`CRAFT_BATCHES_MAX`] and at least 1.
    /// Written each frame the view draws.
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

    /// The batch count the batch form shows.
    pub fn batches(&self) -> u32 {
        self.batches.max(1)
    }
}

/// Start `batches` batches of `recipe` on the player ship, written by the
/// batch form's Craft and applied by [`apply_craft_start_commands`].
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

/// The card's Running block, shown while the player ship runs a craft job.
#[derive(Component)]
pub(crate) struct CraftJobRowMarker;

/// A live part of the Running block.
#[derive(Component, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CraftJobField {
    /// The output and the batch: `PDC round x100  batch 3/5`.
    Label,
    /// The game seconds left.
    Time,
}

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

/// The focus card's recipe rows, and the card they were built for.
#[derive(Component, Default)]
pub(crate) struct CraftFocusCard(Option<Option<CraftCardView>>);

/// The Batches block under the focus card's recipe rows, hidden while no
/// recipe is picked.
#[derive(Component)]
pub(crate) struct CraftBatchForm;

/// The Batches block's amount row, from 1 to the batches the hold and the
/// balance cover.
#[derive(Component)]
pub(crate) struct CraftBatchAmount;

/// The batch form's Craft button.
#[derive(Component)]
pub(crate) struct CraftStartButton;

/// What the view's checks read off the player ship.
pub(crate) struct CraftSite {
    /// The hold less what running repair jobs hold back.
    hold: ShipInventory,
    /// The ship's balance.
    own_cr: u32,
    /// The advancement of the living station that offers Craft the ship is
    /// docked at; `None` when it is docked at none.
    station: Option<f32>,
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
    /// One batch would not start: the tile is muted.
    refused: bool,
    selected: bool,
}

/// The focus card's recipe rows as drawn, for the batch count.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CraftCardView {
    /// `PDC round x200`: what the batches make.
    title: String,
    /// `SHIP` or `STATION`.
    site: &'static str,
    /// Each input, then `Fee` for a station recipe, then `Time`.
    rows: Vec<CraftCardRow>,
}

/// One row of the focus card: a name and its value, such as `Iron ore` and
/// `12 / 2`, `Fee` and `20 cr`, or `Time` and `16 s`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CraftCardRow {
    name: String,
    value: String,
    /// The ship holds less than the batches take, or its balance is under
    /// the fee: the value is painted in the danger tone.
    short: bool,
}

/// The Batches block as written.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CraftBatchView {
    batches: u32,
    /// The amount row's top.
    most: u32,
    /// The start would be refused: Craft is greyed.
    refused: bool,
}

/// Whether the view shows `recipe` at `site`: every recipe but a station
/// recipe the docked Craft station is below the advancement of, which is
/// hidden. Undocked, a station recipe shows muted.
fn shown_at(recipe: &RecipeDesign, site: &CraftSite) -> bool {
    site.station.is_none() || recipe.site.runs_at(site.station)
}

/// The categories, in filter order, the outputs of the `recipes` shown at
/// `site` are in.
fn craft_categories(
    recipes: &GameRecipes,
    items: &GameItems,
    site: &CraftSite,
) -> Vec<ItemCategoryType> {
    let present: BTreeSet<usize> = recipes
        .iter()
        .filter(|recipe| shown_at(recipe, site))
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

/// The recipes the tiles show at `site`, ship recipes first, each by id:
/// those whose output is in `filter`, or every one.
fn shown_recipes<'a>(
    recipes: &'a GameRecipes,
    items: &GameItems,
    site: &CraftSite,
    filter: Option<ItemCategoryType>,
) -> Vec<&'a RecipeDesign> {
    let mut shown: Vec<&RecipeDesign> = recipes
        .iter()
        .filter(|recipe| shown_at(recipe, site))
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
        refused: plan_craft_start(
            recipe,
            1,
            site.running,
            site.station,
            &site.hold,
            site.own_cr,
        )
        .is_err(),
        selected,
    }
}

/// The focus card's recipe rows of `batches` batches of `recipe` at `site`.
fn card_view(
    recipe: &RecipeDesign,
    items: &GameItems,
    site: &CraftSite,
    batches: u32,
) -> CraftCardView {
    let output = items.design(recipe.output_item());
    let times = u64::from(batches);
    let mut rows: Vec<CraftCardRow> = recipe
        .inputs
        .iter()
        .map(|(item, count)| {
            let have = site.hold.count(item);
            let need = u64::from(*count) * times;
            CraftCardRow {
                name: items.design(item).name.clone(),
                value: format!("{have} / {need}"),
                short: u64::from(have) < need,
            }
        })
        .collect();
    let fee_cr = u64::from(recipe.site.fee_cr()) * times;
    if fee_cr > 0 {
        rows.push(CraftCardRow {
            name: "Fee".to_string(),
            value: cr_text(fee_cr),
            short: u64::from(site.own_cr) < fee_cr,
        });
    }
    rows.push(CraftCardRow {
        name: "Time".to_string(),
        value: format!("{} s", recipe.duration_secs * batches as f32),
        short: false,
    });
    CraftCardView {
        title: format!(
            "{} x{}",
            output.name,
            u64::from(recipe.output_count()) * times
        ),
        site: recipe.site.tag(),
        rows,
    }
}

/// The Batches block of `batches` batches of `recipe` at `site`, on an
/// amount row up to `most`.
fn batch_view(recipe: &RecipeDesign, site: &CraftSite, batches: u32, most: u32) -> CraftBatchView {
    CraftBatchView {
        batches,
        most,
        refused: plan_craft_start(
            recipe,
            batches,
            site.running,
            site.station,
            &site.hold,
            site.own_cr,
        )
        .is_err(),
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
            station: stations
                .docked_at(ship, docked)
                .map(|(_, advancement)| advancement),
            running: craft.is_some(),
        },
    ))
}

/// The Inventory pane's Craft button, at the right end of its filter row and
/// styled as the Ship pane's Catalogue button.
pub(crate) fn spawn_craft_open_button(bar: &mut ChildSpawnerCommands) {
    bar.spawn((
        Name::new(INVENTORY_CRAFT),
        button(ButtonSpec::control("Craft")),
    ))
    .observe(on_craft_open)
    .entry::<Node>()
    .and_modify(|mut node| node.margin = UiRect::left(Val::Auto));
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

/// Build the Craft view: the head row (the category chips and Back) over
/// the tiles, beside the card (the Running block, the recipe rows and the
/// Batches block). The reconcilers fill them.
pub(crate) fn craft_body(body: &mut ChildSpawnerCommands) {
    body.spawn(Node {
        flex_grow: 1.0,
        min_height: px(0),
        flex_direction: FlexDirection::Row,
        column_gap: px(metrics::GAP_L),
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
                row_gap: px(metrics::GAP_L),
                ..default()
            })
            .with_children(|left| {
                left.spawn(Node {
                    align_items: AlignItems::FlexStart,
                    ..control_row(JustifyContent::FlexStart)
                })
                .with_children(|row| {
                    row.spawn((CraftFilterBar::default(), chip_bar()))
                        .entry::<Node>()
                        .and_modify(|mut node| {
                            node.flex_grow = 1.0;
                            node.flex_shrink = 1.0;
                            node.min_width = px(0);
                        });
                    row.spawn((Name::new(CRAFT_BACK), button(ButtonSpec::control("Back"))))
                        .observe(on_craft_back);
                });
                left.spawn((
                    CraftTileGrid::default(),
                    Name::new(CRAFT_TILES),
                    Node {
                        display: Display::Grid,
                        // As many columns as fit at the tile width, sharing
                        // the rest of the width evenly.
                        grid_template_columns: vec![RepeatedGridTrack::minmax(
                            GridTrackRepetition::AutoFill,
                            MinTrackSizingFunction::Px(TILE_PX),
                            MaxTrackSizingFunction::Fraction(1.0),
                        )],
                        grid_auto_rows: vec![GridTrack::auto()],
                        flex_grow: 1.0,
                        min_height: px(0),
                        align_content: AlignContent::Start,
                        column_gap: px(metrics::GAP),
                        row_gap: px(metrics::GAP),
                        padding: UiRect::all(px(metrics::GAP_L)),
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
        split
            .spawn((Name::new("CraftCard"), inspector(Overflow::clip())))
            .with_children(|card| {
                spawn_job_row(card);
                card.spawn((
                    CraftFocusCard::default(),
                    Node {
                        flex_direction: FlexDirection::Column,
                        row_gap: px(metrics::GAP),
                        flex_shrink: 0.0,
                        ..default()
                    },
                ));
                spawn_batch_form(card);
            });
    });
}

/// The Batches block: the amount row and Craft. Spawned once and written in
/// place by [`update_craft_view`], so a drag keeps its slider.
fn spawn_batch_form(card: &mut ChildSpawnerCommands) {
    let mut form = spawn_block(card, "Batches", CraftBatchForm);
    form.entry::<Node>()
        .and_modify(|mut node| node.display = Display::None);
    form.with_children(|form| {
        form.spawn((
            CraftBatchAmount,
            amount_row(AmountSpec { name: CRAFT_AMOUNT }),
        ))
        .observe(on_craft_amount);
        form.spawn(verb_row(RowAlign::End)).with_children(|row| {
            row.spawn((
                Name::new(CRAFT_BUTTON),
                CraftStartButton,
                button(ButtonSpec::control("Craft").primary()),
                // The start can be refused, so its result plays the cue.
                ButtonCueType::Owned,
                VerbGate::Disabled,
            ))
            .observe(on_craft_start);
        });
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

/// The card's Running block: the label and the time left, the progress bar,
/// and Cancel at its bottom right.
fn spawn_job_row(card: &mut ChildSpawnerCommands) {
    let mut block = spawn_block(
        card,
        "Running",
        (CraftJobRowMarker, Name::new(CRAFT_JOB_ROW)),
    );
    // The block opens the card, so its rule goes under it, between the job
    // and the recipe card, instead of over it against the panel edge.
    block.entry::<Node>().and_modify(|mut node| {
        node.display = Display::None;
        node.border = UiRect::bottom(node.border.top);
        node.padding = UiRect::bottom(node.padding.top);
    });
    block.with_children(|block| {
        block
            .spawn(control_row(JustifyContent::SpaceBetween))
            .with_children(|line| {
                line.spawn((
                    CraftJobField::Label,
                    text("", TextRole::Value),
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
                    text("", TextRole::Value),
                    TextLayout::new(Justify::Right, LineBreak::NoWrap),
                    Node {
                        flex_shrink: 0.0,
                        ..default()
                    },
                ));
            });
        block.spawn(bar(0.0, UiColorType::Accent));
        block.spawn(verb_row(RowAlign::End)).with_children(|row| {
            row.spawn((
                Name::new(CRAFT_CANCEL),
                button(ButtonSpec::control("Cancel").ghost()),
            ))
            .observe(on_craft_cancel);
        });
    });
}

/// Show the Running block while the player ship runs a craft job, and fill
/// its label, time and bar.
pub(crate) fn update_craft_job_row(
    items: Option<Res<GameItems>>,
    recipes: Option<Res<GameRecipes>>,
    q_player: Query<
        Option<&ShipCraftJob>,
        (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>),
    >,
    mut q_row: Query<&mut Node, With<CraftJobRowMarker>>,
    mut q_text: Query<(&CraftJobField, &mut Text)>,
    mut q_bar: Query<&mut Bar>,
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
            CraftJobField::Time => format!("{:.0} s", job.remaining_secs(recipe).ceil()),
        };
        if text.0 != value {
            text.0 = value;
        }
    }
    let progress = Bar(job.progress(recipe).clamp(0.0, 1.0));
    for mut bar in &mut q_bar {
        bar.set_if_neq(progress);
    }
}

/// Fill the category row, the tiles and the focus card's recipe rows from
/// the recipes and the player ship, rebuilding each only when what it shows
/// changes, and write the batch form in place.
/// A container a new view spawned starts with nothing drawn, so it fills
/// on its first frame.
#[expect(
    clippy::too_many_arguments,
    reason = "the catalogs, the ship, the stations, the three containers and the batch form"
)]
pub(crate) fn update_craft_view(
    mut commands: Commands,
    items: Option<Res<GameItems>>,
    recipes: Option<Res<GameRecipes>>,
    icons: Res<InterfaceIcons>,
    mut runtime: ResMut<CraftRuntime>,
    q_player: PlayerCraftQuery,
    stations: CraftStations,
    (mut q_bar, q_chip): (
        Query<(Entity, &mut CraftFilterBar)>,
        Query<(Entity, &CraftFilterChip, Has<Selected>)>,
    ),
    mut q_grid: Query<(Entity, &mut CraftTileGrid)>,
    mut q_card: Query<(Entity, &mut CraftFocusCard)>,
    form: CraftBatchFormParts,
) {
    let (Some(items), Some(recipes)) = (items.as_deref(), recipes.as_deref()) else {
        return;
    };
    let Some((_, site)) = player_site(&q_player, &stations) else {
        return;
    };
    let categories = craft_categories(recipes, items, &site);
    if runtime
        .filter
        .is_some_and(|filter| !categories.contains(&filter))
    {
        runtime.filter = None;
    }
    let shown = shown_recipes(recipes, items, &site, runtime.filter);
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
    runtime.max_batches = picked.map_or(1, |recipe| {
        craft_batches_max(recipe, &site.hold, site.own_cr).max(1)
    });
    runtime.batches = runtime.batches.clamp(1, runtime.max_batches);

    for (bar, mut drawn) in &mut q_bar {
        let key = categories.clone();
        if drawn.0.as_ref() != Some(&key) {
            commands.entity(bar).despawn_children();
            commands.entity(bar).with_children(|bar| {
                for category in std::iter::once(None).chain(key.iter().copied().map(Some)) {
                    bar.spawn((
                        category_chip("CraftFilter", category, &icons),
                        CraftFilterChip(category),
                    ))
                    .observe(on_craft_filter_chip);
                }
            });
            drawn.0 = Some(key);
        }
    }
    for (chip, category, chosen) in &q_chip {
        select_chip(&mut commands, chip, chosen, category.0 == runtime.filter);
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
                    panel.spawn(text("No recipes", TextRole::Label));
                }
            });
            drawn.0 = Some(card.clone());
        }
    }
    let batch =
        picked.map(|recipe| batch_view(recipe, &site, runtime.batches, runtime.max_batches));
    write_batch_form(&mut commands, form, batch.as_ref());
}

/// The Batches block's parts [`write_batch_form`] writes.
#[derive(bevy::ecs::system::SystemParam)]
pub(crate) struct CraftBatchFormParts<'w, 's> {
    forms: Query<'w, 's, &'static mut Node, With<CraftBatchForm>>,
    amounts: Query<'w, 's, (Entity, &'static mut Amount), With<CraftBatchAmount>>,
    children: Query<'w, 's, &'static Children>,
    focused: Query<'w, 's, (), (With<AmountField>, With<TextFieldFocused>)>,
    buttons: Query<'w, 's, &'static mut VerbGate, With<CraftStartButton>>,
}

/// Show the Batches block for `batch`, or hide it with no recipe picked; a
/// hidden block's field lets go of the keyboard. The amount is written only
/// on a difference, so it does not fight a drag.
fn write_batch_form(
    commands: &mut Commands,
    mut form: CraftBatchFormParts,
    batch: Option<&CraftBatchView>,
) {
    let display = if batch.is_some() {
        Display::Flex
    } else {
        Display::None
    };
    for mut node in &mut form.forms {
        if node.display != display {
            node.display = display;
        }
    }
    for (row, mut amount) in &mut form.amounts {
        match batch {
            Some(batch) => {
                let wanted = Amount {
                    value: batch.batches,
                    min: 1,
                    max: batch.most,
                };
                if *amount != wanted {
                    *amount = wanted;
                }
            }
            None => {
                for part in form.children.iter_descendants(row) {
                    if form.focused.contains(part) {
                        commands.entity(part).remove::<TextFieldFocused>();
                    }
                }
            }
        }
    }
    let Some(batch) = batch else {
        return;
    };
    let gate = if batch.refused {
        VerbGate::Disabled
    } else {
        VerbGate::Ready
    };
    for mut button in &mut form.buttons {
        button.set_if_neq(gate.clone());
    }
}

/// The narrowest a tile gets, in logical px; the grid shares the rest of
/// its width evenly between the tiles of a line.
const TILE_PX: f32 = 150.0;

/// One tile: the category icon, the name and the yield line. A refused tile
/// is muted but still picks its recipe.
fn spawn_tile(grid: &mut ChildSpawnerCommands, tile: &CraftTileView, icons: &InterfaceIcons) {
    let muted = tile.refused;
    let (color, fill, border) = match (tile.selected, muted) {
        (true, _) => (UiColorType::Accent, 0.25, 1.0),
        (false, false) => (UiColorType::TextSecondary, 0.1, 0.6),
        (false, true) => (UiColorType::TextSecondary, 0.04, 0.25),
    };
    let tone = |color| if muted { UiColorType::TextMuted } else { color };
    grid.spawn((
        Name::new(craft_tile_name(&tile.recipe)),
        Button,
        ButtonCueType::Owned,
        CraftTile(tile.recipe.clone()),
        Node {
            min_width: px(0),
            flex_direction: FlexDirection::Column,
            row_gap: px(metrics::GAP_S),
            padding: UiRect::all(px(metrics::GAP)),
            border: UiRect::all(px(1)),
            overflow: Overflow::clip(),
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
            text_toned(&tile.name, TextRole::Value, tone(UiColorType::Text)),
            TextLayout::new(Justify::Left, LineBreak::NoWrap),
            Pickable::IGNORE,
        ));
        cell.spawn((
            text_toned(
                &tile.yield_line,
                TextRole::Label,
                tone(UiColorType::TextBody),
            ),
            TextLayout::new(Justify::Left, LineBreak::NoWrap),
            Pickable::IGNORE,
        ));
    });
}

/// The focus card's recipe rows for `card`: the head, then one key-value row
/// for each input, the fee and the time.
fn spawn_card(panel: &mut ChildSpawnerCommands, card: &CraftCardView) {
    panel
        .spawn(control_row(JustifyContent::SpaceBetween))
        .with_children(|head| {
            head.spawn(text(&card.title, TextRole::Title));
            head.spawn(text(card.site, TextRole::Label));
        });
    for row in &card.rows {
        spawn_kv(
            panel,
            &row.name,
            (text(&row.value, TextRole::Value), Short(row.short)),
        );
    }
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

/// Take a move of the amount row as the batch count, with one tick per new
/// count. The row clamps it from 1 to the most the hold and the balance
/// cover.
fn on_craft_amount(
    changed: On<AmountChanged>,
    mut runtime: ResMut<CraftRuntime>,
    bank: Option<Res<SoundBank<UiSfx>>>,
    mut commands: Commands,
) {
    let batches = changed.value.clamp(1, runtime.max_batches.max(1));
    if runtime.batches != batches {
        runtime.batches = batches;
        if let Some(bank) = bank.as_deref() {
            commands.trigger(PlaySound::ui(
                bank.get(UiSfx::UiTick),
                SoundCategoryType::UiTick,
            ));
        }
    }
}

/// Start the Batches block's count of the picked recipe. A disabled Craft
/// starts nothing; the button widget drops a press on it, and this guard
/// reads the gate, so it also drops an `Activate` triggered directly.
fn on_craft_start(
    activate: On<Activate>,
    q_gate: Query<&VerbGate>,
    runtime: Res<CraftRuntime>,
    mut starts: MessageWriter<CraftStartCommand>,
) {
    if q_gate
        .get(activate.entity)
        .is_ok_and(|gate| *gate != VerbGate::Ready)
    {
        return;
    }
    if let Some(recipe) = runtime.selected.clone() {
        starts.write(CraftStartCommand {
            recipe,
            batches: runtime.batches(),
        });
    }
}

/// Cancel the player ship's craft job. The job row goes; the batches
/// finished stay in the hold.
fn on_craft_cancel(
    _activate: On<Activate>,
    q_player: Query<(Entity, Has<ShipCraftJob>), With<PlayerSpaceshipMarker>>,
    mut commands: Commands,
) {
    if let Ok((ship, true)) = q_player.single() {
        commands.trigger(CraftJobCancelRequest { entity: ship });
    }
}

/// Apply each [`CraftStartCommand`]: check it again against the live ship
/// by [`plan_craft_start`] and insert the [`ShipCraftJob`], or refuse it with
/// the deny cue.
/// The Craft button is greyed on the same check, so a refusal here means the
/// ship changed between the draw and the press.
pub(crate) fn apply_craft_start_commands(
    mut starts: MessageReader<CraftStartCommand>,
    recipes: Option<Res<GameRecipes>>,
    q_player: PlayerCraftQuery,
    stations: CraftStations,
    mut runtime: ResMut<CraftRuntime>,
    mut commands: Commands,
) {
    let Some(recipes) = recipes.as_deref() else {
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
            site.station,
            &site.hold,
            site.own_cr,
        ) {
            Ok(()) => {
                commands
                    .entity(ship)
                    .insert(ShipCraftJob::new(recipe.id.clone(), start.batches));
                runtime.batches = 1;
                runtime.cue = Some(ActionOutcomeType::Done);
                // One start a pass: the query does not see the job inserted
                // above, so a second start would pass the Running check.
                starts.clear();
                return;
            }
            Err(refusal) => {
                info!(
                    "apply_craft_start_commands: {} refused: {refusal:?}",
                    recipe.id
                );
                runtime.cue = Some(ActionOutcomeType::Refused);
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

/// Play the cue the last start owes once the sound bank lands.
pub(crate) fn play_craft_cue(
    mut runtime: ResMut<CraftRuntime>,
    bank: Option<Res<SoundBank<UiSfx>>>,
    mut commands: Commands,
) {
    if bank.is_none() {
        return;
    }
    if let Some(outcome) = runtime.cue.take() {
        play_outcome_cue(&mut commands, bank.as_deref(), outcome);
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
                play_craft_cue,
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
