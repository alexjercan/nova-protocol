//! The Inventory pane's whole-hull strip: its block, its command and its
//! timer.
//!
//! While the player ship is docked with a partner it may loot (neutralized,
//! or a derelict carrying [`LootableShipMarker`]) that is not a living
//! station, a compact block above the partner column shows the Strip button
//! and four meters: the whole strip and its time, the running section, the
//! sections taken and the hold with the mass still to come. Under them, one
//! tile per part names what the strip recovered so far over what it plans to
//! give. The plan of the sections still live is an estimate at their current
//! health, so it changes as they take damage. A click raises a
//! [`HullStripCommand`]; [`apply_hull_strip_commands`] plans it with
//! [`plan_strip_press`], takes the partner's whole hold and balance as Take
//! all does, and starts a [`HullStripJob`] on the player ship. When
//! that take would leave anything behind, the strip does not start and
//! nothing moves: the button is disabled by the same rule, and its tooltip
//! names what has no room. [`advance_hull_strip`] runs the job on the game clock, with the pane
//! open or shut, so the player starts it and flies on; the TAB interface's
//! pause holds it like any other game work.
//!
//! The job takes one live section at a time, the one [`next_strip_section`]
//! names: the farthest from the berth, the berth port last. When a section's
//! timer ends, [`plan_section_yield`] plans its parts: they go into the
//! player's hold, one `Stripped` report is queued, and a
//! [`SectionStripRequest`] takes the section off. A hold too full for them
//! holds the job at that section until there is room. Before the berth's
//! parts, the job takes what fits of anything put into the wreck's hold or
//! balance during the strip, and holds there until the rest fits, so the
//! wreck never despawns with loot in it. The berth's parts end the strip:
//! the player lets go of the dock and the wreck despawns. The job
//! stops when the player undocks or docks with another hull; every section
//! already taken stays taken.
//!
//! Touch this module when changing what the strip block shows or what Strip
//! does.

#[cfg(test)]
mod tests;

use std::collections::{BTreeMap, BTreeSet};

use bevy::{ecs::system::SystemParam, prelude::*, ui::InteractionDisabled, ui_widgets::Activate};
use nova_events::prelude::EntityId;
use nova_gameplay::prelude::*;
use nova_ship::prelude::*;
use nova_ui::{
    theme::UiColor,
    widget::{ButtonSpec, ThemedBorder, ThemedFill, ThemedText, Tooltip},
};

use super::{loot_text, take_loot, InventoryRuntime, InventoryShips};
use crate::{
    icons::InterfaceIcons,
    pane::{compact_button, play_menu_select, spawn_part_tile, themed_label},
    ship::design_section_code,
};

/// The name of the strip block, shown only while docked with a wreck.
pub const INVENTORY_STRIP_BLOCK: &str = "InventoryStripBlock";

/// The name of the Strip button.
pub const INVENTORY_STRIP_BUTTON: &str = "InventoryStripButton";

/// The name of the strip block's whole-strip time text.
pub const INVENTORY_STRIP_PROGRESS: &str = "InventoryStripProgress";

/// A request to strip the whole docked partner, raised by the strip block's
/// Strip button.
#[derive(Message, Clone, Copy, Debug, Default)]
pub struct HullStripCommand;

/// The strip block over the partner column, shown only while
/// [`StripPartner::recyclable`] holds.
#[derive(Component)]
pub(crate) struct StripBlock;

/// A text of the strip block that [`update_strip_block`] refreshes.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum StripBlockField {
    /// The time a strip takes, the time left, or `Held`.
    Time,
    /// The running section's code and its time left.
    Step,
    /// Sections taken over the hull's sections.
    Sections,
    /// The hold, used over capacity, with the mass still to come or a held
    /// section's need.
    Hold,
}

/// A meter fill of the strip block.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum StripBlockFill {
    Total,
    Step,
    Sections,
    Hold,
}

/// The running section's row, shown while a strip job exists.
#[derive(Component)]
pub(crate) struct StripStepRow;

/// The part tiles, with what they last drew so they rebuild only on a change.
#[derive(Component, Default)]
pub(crate) struct StripYieldTiles(Vec<StripYieldTile>);

/// One part a strip gives.
#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) struct StripYieldTile {
    item: ItemDesignId,
    /// Parts recovered so far (the job's `yielded` sum; the player may have
    /// moved them since); `None` before the strip starts.
    yielded: Option<u32>,
    /// `yielded` plus the plan of the sections still live.
    planned: u32,
}

/// The Strip button: hidden while a strip runs or the partner is not a wreck,
/// disabled with a tooltip while the loot it takes first has no room.
#[derive(Component)]
pub(crate) struct StripBlockButton;

/// The recipe of a live section its hull's design does not list: a
/// hand-built hull has no design. It gives no part and takes the shortest
/// strip.
static NO_RECIPE: BuildRecipe = BuildRecipe {
    credits: 0,
    parts: BTreeMap::new(),
    strip_yield: 0.0,
};

/// One live section of the docked partner.
struct LiveSection<'a> {
    entity: Entity,
    id: &'a str,
    health_share: f32,
    neighbours: &'a [Entity],
}

/// The docked partner a strip reads.
pub(crate) struct StripPartner<'a> {
    pub(crate) entity: Entity,
    /// The partner's port that holds the dock.
    berth: Entity,
    living_station: bool,
    lootable: bool,
    design: Option<&'a ShipDesignSections>,
    live: Vec<LiveSection<'a>>,
}

impl StripPartner<'_> {
    /// Whether the strip block serves this partner: one the player may loot
    /// that is not a living station.
    pub(crate) fn recyclable(&self) -> bool {
        self.lootable && !self.living_station
    }

    /// The pane code of design section `id`, or the id when the partner has
    /// no design to number it from.
    fn code(&self, id: &str) -> String {
        self.design
            .and_then(|design| design_section_code(design, id))
            .unwrap_or_else(|| id.to_string())
    }

    fn target(&self, section: &LiveSection<'_>) -> StripTarget<'_> {
        let recipe = self
            .design
            .and_then(|design| design.get(section.id))
            .map_or(&NO_RECIPE, |design| &design.config.base.build);
        StripTarget {
            recipe,
            health_share: section.health_share,
        }
    }

    /// The facts [`plan_hull_strip`] reads, with the Take all a strip runs
    /// first.
    fn hull(&self, loot: LootTake) -> HullStripPartner<'_> {
        HullStripPartner {
            living_station: self.living_station,
            lootable: self.lootable,
            sections: self
                .live
                .iter()
                .map(|section| self.target(section))
                .collect(),
            loot,
        }
    }

    /// The live section a strip takes next, by [`next_strip_section`] with
    /// ties in pane code order.
    fn next(&self) -> Option<&LiveSection<'_>> {
        let mut order: Vec<(usize, (String, u32))> = self
            .live
            .iter()
            .enumerate()
            .map(|(index, section)| (index, code_order(&self.code(section.id))))
            .collect();
        order.sort_by(|a, b| a.1.cmp(&b.1));
        let next = next_strip_section(
            self.berth,
            order.iter().enumerate().map(|(rank, (index, _))| {
                let section = &self.live[*index];
                (section.entity, rank, section.neighbours)
            }),
        )?;
        self.live.iter().find(|section| section.entity == next)
    }

    fn live(&self, entity: Entity) -> Option<&LiveSection<'_>> {
        self.live.iter().find(|section| section.entity == entity)
    }
}

/// The player ship, the partner it is docked with and every fact a strip
/// reads.
#[derive(SystemParam)]
pub(crate) struct StripSite<'w, 's> {
    player: Query<
        'w,
        's,
        (Entity, Option<&'static DockedShip>),
        (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>),
    >,
    connections: Query<'w, 's, &'static DockingConnection>,
    partners: Query<
        'w,
        's,
        (
            Option<&'static ShipDesignSections>,
            Option<&'static Children>,
            Has<LootableShipMarker>,
            Has<NeutralizedMarker>,
            Has<StationMarker>,
            Has<DerelictShipMarker>,
        ),
        With<SpaceshipRootMarker>,
    >,
    sections: Query<
        'w,
        's,
        (
            &'static EntityId,
            &'static Health,
            Option<&'static ConnectedTo>,
        ),
        With<SectionMarker>,
    >,
}

impl StripSite<'_, '_> {
    /// The player ship, or `None` when there is not exactly one.
    pub(crate) fn player(&self) -> Option<Entity> {
        self.player.single().ok().map(|(player, _)| player)
    }

    /// The partner the player ship is docked with, or `None` while it is not
    /// docked.
    ///
    /// # Panics
    ///
    /// On a docked player whose connection is gone: `DockedShip` leaves with
    /// its connection.
    pub(crate) fn partner(&self) -> Option<StripPartner<'_>> {
        let (player, docked) = self.player.single().ok()?;
        let docked = docked?;
        let connection = self.connections.get(docked.connection).unwrap_or_else(|_| {
            panic!(
                "player ship {player:?} is docked through {:?}, which has no DockingConnection",
                docked.connection
            )
        });
        let (entity, berth) = if connection.first_ship == player {
            (connection.second_ship, connection.second_section)
        } else {
            (connection.first_ship, connection.first_section)
        };
        let (design, children, lootable, neutralized, station, derelict) =
            self.partners.get(entity).ok()?;
        let live = children
            .into_iter()
            .flatten()
            .filter_map(|child| {
                let (id, health, connected) = self.sections.get(*child).ok()?;
                let health_share = if health.max > 0.0 {
                    health.current / health.max
                } else {
                    0.0
                };
                Some(LiveSection {
                    entity: *child,
                    id: id.0.as_str(),
                    health_share,
                    neighbours: connected.map_or(&[][..], |connected| connected.0.as_slice()),
                })
            })
            .collect();
        Some(StripPartner {
            entity,
            berth,
            living_station: station && !derelict,
            lootable: lootable || neutralized,
            design,
            live,
        })
    }
}

/// The Strip rule the strip block's button and the press share:
/// [`plan_hull_strip`] of the docked `partner`, with the Take all of its hold,
/// a dead station's storage and its balance into the player's
/// ([`InventoryShips::loot`]) that a strip runs first. Returns the partner
/// as the rule read it beside the verdict: its loot is what a start takes,
/// or what a [`HullStripRefusalType::NoRoom`] leaves behind.
///
/// # Panics
///
/// On a partner with no player ship to take its loot: a partner is only
/// found from the player's dock.
pub(crate) fn plan_strip_press<'a>(
    items: &GameItems,
    ships: &InventoryShips,
    partner: Option<&'a StripPartner<'_>>,
    busy: bool,
) -> (
    Option<HullStripPartner<'a>>,
    Result<HullStrip, HullStripRefusalType>,
) {
    let hull = partner.map(|partner| {
        let own = ships
            .pair()
            .expect("a docked partner has a player ship")
            .own;
        partner.hull(ships.loot(items, own, partner.entity))
    });
    let verdict = plan_hull_strip(hull.as_ref(), busy);
    (hull, verdict)
}

/// The note line for a strip that does not start. `hull` names what has no
/// room on a [`HullStripRefusalType::NoRoom`].
fn strip_refusal_text(
    items: &GameItems,
    refusal: HullStripRefusalType,
    hull: Option<&HullStripPartner<'_>>,
) -> String {
    match refusal {
        HullStripRefusalType::NotDocked => "strip: dock with a wreck first".to_string(),
        HullStripRefusalType::Busy => "strip: a strip is already running".to_string(),
        HullStripRefusalType::LivingStation => "strip: a living station is not salvage".to_string(),
        HullStripRefusalType::NotLootable => {
            "strip: the docked hull is not a wreck you may loot".to_string()
        }
        HullStripRefusalType::NoRoom => format!(
            "strip: {}",
            hull.map_or_else(String::new, |hull| no_room_text(items, &hull.loot))
        ),
    }
}

/// What a Take all leaves behind: `no room for 6 Hull plate`.
fn no_room_text(items: &GameItems, loot: &LootTake) -> String {
    format!(
        "no room for {}",
        loot_text(items, &loot.stayed, loot.stayed_cr)
    )
}

/// The list order of a pane code: its prefix, then its number, so `HULL-2`
/// sorts before `HULL-10`. A code with no number sorts by its text alone.
fn code_order(code: &str) -> (String, u32) {
    match code.rsplit_once('-') {
        Some((prefix, index)) => match index.parse() {
            Ok(index) => (prefix.to_string(), index),
            Err(_) => (code.to_string(), 0),
        },
        None => (code.to_string(), 0),
    }
}

/// Parts by item name: `3 Section Panel, 1 Frame Spar`, or `nothing`.
pub(crate) fn strip_parts_text(items: &GameItems, parts: &BTreeMap<ItemDesignId, u32>) -> String {
    if parts.is_empty() {
        return "nothing".to_string();
    }
    parts
        .iter()
        .map(|(item, count)| format!("{count} {}", items.design(item).name))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Game seconds as `2 min 05 s`, or `45 s` under a minute.
fn duration_text(secs: f32) -> String {
    let secs = secs.ceil() as u32;
    if secs < 60 {
        return format!("{secs} s");
    }
    format!("{} min {:02} s", secs / 60, secs % 60)
}

/// The tiles of `plan` (the live sections) and a running `job`'s yield,
/// in item id order.
fn yield_tiles(
    plan: &BTreeMap<ItemDesignId, u32>,
    job: Option<&HullStripJob>,
) -> Vec<StripYieldTile> {
    let yielded = job.map(|job| &job.yielded);
    let ids: BTreeSet<&ItemDesignId> = plan
        .keys()
        .chain(yielded.into_iter().flat_map(BTreeMap::keys))
        .collect();
    ids.into_iter()
        .map(|item| {
            let got = yielded.map(|yielded| yielded.get(item).copied().unwrap_or(0));
            StripYieldTile {
                item: item.clone(),
                yielded: got,
                planned: got.unwrap_or(0) + plan.get(item).copied().unwrap_or(0),
            }
        })
        .collect()
}

/// Build the strip block, hidden until [`update_strip_block`] finds a wreck:
/// the Strip button beside the Total meter, the Step meter shown while a
/// strip runs, the Sections and Hold meters, and the part tiles.
pub(crate) fn spawn_strip_block(panel: &mut ChildSpawnerCommands) {
    panel
        .spawn((
            StripBlock,
            Name::new(INVENTORY_STRIP_BLOCK),
            Node {
                display: Display::None,
                flex_direction: FlexDirection::Column,
                flex_shrink: 0.0,
                row_gap: Val::Px(4.0),
                padding: UiRect::all(Val::Px(6.0)),
                border: UiRect::all(Val::Px(1.0)),
                border_radius: BorderRadius::all(Val::Px(4.0)),
                ..default()
            },
            BorderColor::all(Color::NONE),
            ThemedBorder::alpha(UiColor::Accent, 0.45),
        ))
        .with_children(|block| {
            block
                .spawn(Node {
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    column_gap: Val::Px(8.0),
                    ..default()
                })
                .with_children(|row| {
                    row.spawn((
                        StripBlockButton,
                        Name::new(INVENTORY_STRIP_BUTTON),
                        compact_button(ButtonSpec::new("Strip").fit().primary()),
                        Tooltip::default(),
                    ))
                    .observe(on_strip_button);
                    spawn_meter_row(row, "Total", StripBlockFill::Total, StripBlockField::Time);
                });
            block
                .spawn((
                    StripStepRow,
                    Node {
                        display: Display::None,
                        flex_direction: FlexDirection::Column,
                        ..default()
                    },
                ))
                .with_children(|row| {
                    spawn_meter_row(row, "Step", StripBlockFill::Step, StripBlockField::Step);
                });
            spawn_meter_row(
                block,
                "Sections",
                StripBlockFill::Sections,
                StripBlockField::Sections,
            );
            spawn_meter_row(block, "Hold", StripBlockFill::Hold, StripBlockField::Hold);
            block.spawn((
                StripYieldTiles::default(),
                Node {
                    flex_direction: FlexDirection::Row,
                    flex_wrap: FlexWrap::Wrap,
                    column_gap: Val::Px(12.0),
                    row_gap: Val::Px(4.0),
                    ..default()
                },
            ));
        });
}

/// A labelled meter row: `label`, a track holding `fill`, and `field`.
fn spawn_meter_row(
    block: &mut ChildSpawnerCommands,
    label: &str,
    fill: StripBlockFill,
    field: StripBlockField,
) {
    block
        .spawn(Node {
            flex_grow: 1.0,
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            column_gap: Val::Px(8.0),
            ..default()
        })
        .with_children(|row| {
            row.spawn((
                themed_label(label, 11.0, UiColor::Label),
                TextLayout::new(Justify::Left, LineBreak::NoWrap),
                Node {
                    width: Val::Px(64.0),
                    flex_shrink: 0.0,
                    ..default()
                },
            ));
            row.spawn((
                Node {
                    flex_grow: 1.0,
                    min_width: Val::Px(24.0),
                    height: Val::Px(8.0),
                    border_radius: BorderRadius::all(Val::Px(4.0)),
                    overflow: Overflow::clip(),
                    ..default()
                },
                BackgroundColor(Color::NONE),
                ThemedFill::alpha(UiColor::Secondary, 0.25),
            ))
            .with_children(|track| {
                track.spawn((
                    fill,
                    Node {
                        width: Val::Percent(0.0),
                        height: Val::Percent(100.0),
                        ..default()
                    },
                    BackgroundColor(Color::NONE),
                    ThemedFill::new(UiColor::Accent),
                ));
            });
            let mut value = row.spawn((
                field,
                themed_label("", 12.0, UiColor::Primary),
                TextLayout::new(Justify::Right, LineBreak::NoWrap),
            ));
            if field == StripBlockField::Time {
                value.insert(Name::new(INVENTORY_STRIP_PROGRESS));
            }
        });
}

/// Keep the strip block on the docked partner and the running job: shown only
/// at a wreck, the Strip button while no strip runs, disabled with its
/// tooltip while [`plan_strip_press`] refuses for room, the four meters, and
/// the part tiles. A held strip reads `Held` on the Total row and names what
/// its section needs on the Hold row, both in Danger.
#[expect(
    clippy::type_complexity,
    reason = "the block, its texts, its button, its fills and its tiles"
)]
#[expect(
    clippy::too_many_arguments,
    reason = "the strip rule's reads beside the block's nodes and the button state"
)]
pub(crate) fn update_strip_block(
    items: Option<Res<GameItems>>,
    icons: Res<InterfaceIcons>,
    site: StripSite,
    ships: InventoryShips,
    mut commands: Commands,
    mut q_button: Query<(Entity, &mut Tooltip, Has<InteractionDisabled>), With<StripBlockButton>>,
    q_job: Query<&HullStripJob, With<PlayerSpaceshipMarker>>,
    mut q_field: Query<(&StripBlockField, &mut Text, &mut ThemedText)>,
    mut q_fill: Query<(&StripBlockFill, &mut Node, &mut ThemedFill)>,
    mut q_shown: Query<
        (&mut Node, Has<StripBlock>, Has<StripBlockButton>),
        (
            Or<(With<StripBlock>, With<StripBlockButton>, With<StripStepRow>)>,
            Without<StripBlockFill>,
        ),
    >,
    mut q_tiles: Query<(Entity, &mut StripYieldTiles)>,
) {
    let Some(items) = items else {
        return;
    };
    let player = site.player();
    let job = player.and_then(|player| q_job.get(player).ok());
    let partner = site.partner().filter(StripPartner::recyclable);
    let (hull, verdict) = plan_strip_press(&items, &ships, partner.as_ref(), job.is_some());
    let no_room = verdict == Err(HullStripRefusalType::NoRoom);
    let button_shown = verdict.is_ok() || no_room;
    let reason = match (&hull, no_room) {
        (Some(hull), true) => no_room_text(&items, &hull.loot),
        _ => String::new(),
    };
    for (button, mut tooltip, disabled) in &mut q_button {
        if tooltip.0 != reason {
            tooltip.0.clone_from(&reason);
        }
        if no_room && !disabled {
            commands.entity(button).insert(InteractionDisabled);
        } else if !no_room && disabled {
            commands.entity(button).remove::<InteractionDisabled>();
        }
    }
    // Each meter as (fill share, fill colour, text, text colour).
    let empty = (0.0, UiColor::Accent, String::new(), UiColor::Primary);
    let (mut total, mut step, mut sections, mut hold) =
        (empty.clone(), empty.clone(), empty.clone(), empty);
    let mut tiles = Vec::new();
    if let (Some(partner), Some(hull), Some(player)) = (&partner, &hull, player) {
        let strip = hull.plan();
        let held = job.and_then(|job| job.paused);
        total = match job {
            None => (
                0.0,
                UiColor::Accent,
                format!("{} to strip", duration_text(strip.duration_secs)),
                UiColor::Primary,
            ),
            Some(job) if held.is_some() => (
                job.progress(),
                UiColor::Accent,
                "Held".to_string(),
                UiColor::Danger,
            ),
            Some(job) => (
                job.progress(),
                UiColor::Accent,
                format!("{} left", duration_text(job.remaining_secs())),
                UiColor::Primary,
            ),
        };
        if let Some(running) = job.and_then(|job| job.step.as_ref()) {
            let (share, text) = if held.is_some() {
                (1.0, running.code.clone())
            } else if running.duration_secs > 0.0 {
                (
                    running.elapsed_secs / running.duration_secs,
                    format!(
                        "{}, {} left",
                        running.code,
                        duration_text(running.duration_secs - running.elapsed_secs)
                    ),
                )
            } else {
                (1.0, running.code.clone())
            };
            step = (share, UiColor::Accent, text, UiColor::Primary);
        }
        let taken = job.map_or(0, |job| job.taken);
        let all = taken + partner.live.len() as u32;
        sections = (
            if all > 0 {
                taken as f32 / all as f32
            } else {
                0.0
            },
            UiColor::Accent,
            format!("{taken} / {all}"),
            UiColor::Primary,
        );
        let (_, own, _, _) = ships.ship(player);
        let used_g = own.used_g(&items);
        let capacity_g = own.capacity_g();
        let share = if capacity_g > 0 {
            (used_g as f32 / capacity_g as f32).min(1.0)
        } else {
            1.0
        };
        let used = format!(
            "{} / {}",
            kg_text(u64::from(used_g)),
            kg_text(u64::from(capacity_g))
        );
        hold = match held {
            Some(HullStripPauseType::HoldFull { need_g, .. }) => (
                share,
                UiColor::Danger,
                format!("{used}  needs {}", kg_text(need_g)),
                UiColor::Danger,
            ),
            None => {
                let mass_g: u64 = strip
                    .parts
                    .iter()
                    .map(|(item, &count)| items.design(item).stack_mass_g(count))
                    .sum();
                (
                    share,
                    UiColor::Accent,
                    format!("{used}  +{}", kg_text(mass_g)),
                    UiColor::Primary,
                )
            }
        };
        tiles = yield_tiles(&strip.parts, job);
    }
    for (field, mut text, mut themed) in &mut q_field {
        let (_, _, line, color) = match field {
            StripBlockField::Time => &total,
            StripBlockField::Step => &step,
            StripBlockField::Sections => &sections,
            StripBlockField::Hold => &hold,
        };
        if text.0 != *line {
            text.0.clone_from(line);
        }
        if themed.color != *color {
            themed.color = *color;
        }
    }
    for (fill, mut node, mut themed) in &mut q_fill {
        let (share, color, ..) = match fill {
            StripBlockFill::Total => &total,
            StripBlockFill::Step => &step,
            StripBlockFill::Sections => &sections,
            StripBlockFill::Hold => &hold,
        };
        let width = Val::Percent(share.clamp(0.0, 1.0) * 100.0);
        if node.width != width {
            node.width = width;
        }
        if themed.color != *color {
            themed.color = *color;
        }
    }
    let flex = |shown: bool| if shown { Display::Flex } else { Display::None };
    for (mut node, block, button) in &mut q_shown {
        let display = if block {
            flex(partner.is_some())
        } else if button {
            flex(button_shown)
        } else {
            flex(partner.is_some() && job.is_some())
        };
        if node.display != display {
            node.display = display;
        }
    }
    for (entity, mut drawn) in &mut q_tiles {
        if drawn.0 == tiles {
            continue;
        }
        commands
            .entity(entity)
            .despawn_related::<Children>()
            .with_children(|row| {
                for tile in &tiles {
                    let count = match tile.yielded {
                        Some(yielded) => format!("{yielded}/{}", tile.planned),
                        None => tile.planned.to_string(),
                    };
                    spawn_part_tile(row, &icons, &items, &tile.item, &count);
                }
            });
        drawn.0.clone_from(&tiles);
    }
}

/// Raise a [`HullStripCommand`] when the Strip button is clicked.
fn on_strip_button(
    _activate: On<Activate>,
    mut strips: MessageWriter<HullStripCommand>,
    bank: Option<Res<SoundBank<UiSfx>>>,
    mut commands: Commands,
) {
    strips.write(HullStripCommand);
    play_menu_select(&mut commands, bank.as_deref());
}

/// Apply [`HullStripCommand`]s by the [`plan_strip_press`] rule: take the
/// partner's whole hold, a dead station's whole storage and its balance into
/// the player's, as Take all does, then start a [`HullStripJob`] on the
/// player ship. No section moves until
/// its timer ends. A refusal moves nothing, the loot included, and shows its
/// reason on the note line.
pub(crate) fn apply_hull_strip_commands(
    mut messages: MessageReader<HullStripCommand>,
    mut runtime: ResMut<InventoryRuntime>,
    mut commands: Commands,
    items: Option<Res<GameItems>>,
    site: StripSite,
    mut ships: InventoryShips,
    q_player: Query<Has<HullStripJob>, With<PlayerSpaceshipMarker>>,
) {
    if messages.is_empty() {
        return;
    }
    // One strip per frame: the job lands with the commands, so a second
    // command this frame would plan as not busy.
    messages.clear();
    let (Some(player), Some(items)) = (site.player(), items) else {
        return;
    };
    let busy = q_player.get(player).expect("the player ship has a root");
    let partner = site.partner();
    let (hull, verdict) = plan_strip_press(&items, &ships, partner.as_ref(), busy);
    let strip = match verdict {
        Ok(strip) => strip,
        Err(refusal) => {
            info!("strip: refused: {refusal:?}");
            runtime.say(strip_refusal_text(&items, refusal, hull.as_ref()));
            return;
        }
    };
    let loot = hull.expect("a planned strip has a docked partner").loot;
    let partner = partner.expect("a planned strip has a docked partner");
    let pair = ships.pair().expect("a docked partner has a player ship");
    take_loot(&items, &mut ships, pair, partner.entity, &loot);
    let looted = loot_text(&items, &loot.taken, loot.taken_cr);
    info!(
        "strip: {:?} starts, took {}, {} sections, {:.0} s, expected {}",
        partner.entity,
        if loot.is_empty() { "nothing" } else { &looted },
        partner.live.len(),
        strip.duration_secs,
        strip_parts_text(&items, &strip.parts)
    );
    let took = if loot.is_empty() {
        String::new()
    } else {
        format!("took {looted}; ")
    };
    runtime.say(format!(
        "{took}stripping the wreck: {}",
        duration_text(strip.duration_secs)
    ));
    commands
        .entity(player)
        .insert(HullStripJob::new(partner.entity, strip.duration_secs, loot));
}

/// Run the player ship's [`HullStripJob`] on the game clock.
///
/// The job stops when the player is no longer docked with its partner. It
/// picks the next section by [`next_strip_section`] when it has none, or
/// when its section is gone some other way. When the section's timer ends,
/// [`plan_section_yield`] plans its parts: on a pause the job holds there;
/// otherwise the parts go into the hold, one `Stripped` report is queued, and
/// the section comes off by a [`SectionStripRequest`], or, for the berth, by
/// letting go of the dock and despawning the wreck. Before the berth's parts
/// the job takes what fits of the wreck's hold and balance, as Take all does,
/// and holds while any of it stays.
///
/// # Panics
///
/// On a finished section whose partner has no scenario id: every scenario
/// ship and station wears one.
pub(crate) fn advance_hull_strip(
    time: Res<Time>,
    mut runtime: ResMut<InventoryRuntime>,
    mut commands: Commands,
    items: Option<Res<GameItems>>,
    site: StripSite,
    mut ships: InventoryShips,
    q_ids: Query<&EntityId>,
    mut q_job: Query<&mut HullStripJob, With<PlayerSpaceshipMarker>>,
) {
    let Some(player) = site.player() else {
        return;
    };
    let Ok(mut job) = q_job.get_mut(player) else {
        return;
    };
    let Some(partner) = site
        .partner()
        .filter(|partner| partner.entity == job.partner)
    else {
        info!("strip: stopped by the undock after {} sections", job.taken);
        runtime.say(format!(
            "strip stopped: you undocked; {} section{} taken",
            job.taken,
            if job.taken == 1 { "" } else { "s" }
        ));
        commands.entity(player).remove::<HullStripJob>();
        return;
    };
    let Some(items) = items else {
        return;
    };
    // A section taken some other way (shot off) leaves the job for the next.
    if job
        .step
        .as_ref()
        .is_some_and(|step| partner.live(step.section).is_none())
    {
        job.step = None;
        job.paused = None;
    }
    if job.step.is_none() {
        let Some(next) = partner.next() else {
            // The berth is always live while the dock holds, so a docked
            // partner always has a next section.
            return;
        };
        let duration_secs = strip_duration_secs(partner.target(next).recipe);
        job.step = Some(HullStripStep {
            section: next.entity,
            section_id: next.id.to_string(),
            code: partner.code(next.id),
            elapsed_secs: 0.0,
            duration_secs,
        });
    }
    let step = job.step.clone().expect("the job has a step");
    if !step.is_done() {
        let step = job.step.as_mut().expect("the job has a step");
        step.elapsed_secs = (step.elapsed_secs + time.delta_secs()).min(step.duration_secs);
        if !step.is_done() {
            return;
        }
    }
    let section = partner.live(step.section).expect("the step is live");
    let hull = q_ids
        .get(partner.entity)
        .unwrap_or_else(|_| panic!("strip: partner {:?} has no scenario id", partner.entity));
    // Cargo and credits put into the wreck during the strip come off before
    // its berth, so the wreck never leaves the world with loot in it.
    if step.section == partner.berth {
        let sweep = ships.loot(&items, player, partner.entity);
        if !sweep.is_empty() {
            let pair = ships.pair().expect("a docked partner has a player ship");
            take_loot(&items, &mut ships, pair, partner.entity, &sweep);
            info!(
                "strip: swept {} from '{}'",
                loot_text(&items, &sweep.taken, sweep.taken_cr),
                hull.0
            );
            for (item, &count) in &sweep.taken {
                *job.looted.taken.entry(item.clone()).or_insert(0) += count;
            }
            for (item, &count) in &sweep.taken_stored {
                *job.looted.taken_stored.entry(item.clone()).or_insert(0) += count;
            }
            job.looted.taken_cr += sweep.taken_cr;
        }
        if !sweep.is_whole() {
            // Credits stay only when the player's balance would overflow;
            // the hold then reads full with only the items' mass to come.
            let need_g = sweep
                .stayed
                .iter()
                .map(|(item, &count)| items.design(item).stack_mass_g(count))
                .sum();
            let free_g = ships.ship(player).1.free_g(&items);
            let pause = HullStripPauseType::HoldFull { need_g, free_g };
            if job.paused != Some(pause) {
                info!("strip: held at {}: {pause:?}", step.code);
                job.paused = Some(pause);
            }
            return;
        }
    }
    let (mut own, mut reports) = ships.hold_mut(player);
    let parts = match plan_section_yield(&items, partner.target(section), &own) {
        Ok(parts) => parts,
        Err(pause) => {
            if job.paused != Some(pause) {
                info!("strip: held at {}: {pause:?}", step.code);
                job.paused = Some(pause);
            }
            return;
        }
    };
    for (item, &count) in &parts {
        own.add(&items, item, count);
        *job.yielded.entry(item.clone()).or_insert(0) += count;
    }
    let text = strip_parts_text(&items, &parts);
    info!("strip: {} of '{}' gave {text}", step.code, hull.0);
    reports.0.push(ShipInventoryReport::Stripped {
        hull: hull.0.clone(),
        section: step.code.clone(),
        parts,
    });
    job.paused = None;
    job.step = None;
    job.done_secs += step.duration_secs;
    job.taken += 1;
    if step.section != partner.berth {
        commands.trigger(SectionStripRequest {
            entity: step.section,
        });
        return;
    }
    // The berth comes off last: let go of the dock first, so the release is
    // the ordinary one, then the wreck leaves the world.
    info!(
        "strip: '{}' is stripped whole after {} sections",
        hull.0, job.taken
    );
    commands.trigger(DockingReleaseRequest { entity: player });
    commands.entity(partner.entity).despawn();
    commands.entity(player).remove::<HullStripJob>();
    runtime.say(strip_done_text(&items, &job));
}

/// The note that ends a strip: the loot it took, first and at the berth, and
/// every part its sections gave, such as `the wreck is stripped and gone:
/// took 6 Hull plate and 50 cr, stripped 3 Section panel`.
fn strip_done_text(items: &GameItems, job: &HullStripJob) -> String {
    let took = if job.looted.is_empty() {
        String::new()
    } else {
        format!(
            "took {}, ",
            loot_text(items, &job.looted.taken, job.looted.taken_cr)
        )
    };
    format!(
        "the wreck is stripped and gone: {took}stripped {}",
        strip_parts_text(items, &job.yielded)
    )
}
