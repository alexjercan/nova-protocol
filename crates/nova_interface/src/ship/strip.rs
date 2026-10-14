//! The Ship pane's "Strip the docked wreck" list and the timed Strip action.
//!
//! While the player ship is docked with a partner it may loot (neutralized,
//! or a derelict carrying [`LootableShipMarker`]), the panel lists each live
//! section of the partner under its design code, in code order, with the
//! parts a strip would give, its time and a Strip button, or the reason it
//! cannot be stripped. A click raises a [`SectionStripCommand`];
//! [`apply_ship_strip_commands`] plans it with [`plan_section_strip`] and
//! starts a [`SectionStripJob`] on the player ship. [`advance_ship_strip`]
//! runs the job on the game clock, so it holds while the TAB interface
//! pauses the world, and shows its progress on the list's progress line.
//! When the timer ends the strip is planned again: the parts go into the
//! player's hold, one `Stripped` report is queued, and a
//! [`SectionStripRequest`] takes the section off the partner. The job stops
//! with no change when the player undocks, the section is gone, or the plan
//! now refuses.
//!
//! Touch this module when changing what the strip list shows or what Strip
//! does.

use std::collections::BTreeMap;

use bevy::{ecs::system::SystemParam, prelude::*, ui_widgets::Activate};
use nova_events::prelude::EntityId;
use nova_gameplay::prelude::*;
use nova_ship::prelude::*;
use nova_ui::{
    theme::UiColor,
    widget::{ButtonSpec, ThemedFill},
};

use super::{scene::ShipRuntime, sections::design_section_code};
use crate::pane::{compact_button, play_menu_select, themed_label};

/// The name of the strip list's container, which a capture scrolls into
/// view.
pub const STRIP_LIST: &str = "ShipStripList";

/// The name of the strip list's progress line, which a capture scrolls into
/// view.
pub const STRIP_PROGRESS: &str = "ShipStripProgress";

/// The name prefix of a row's Strip button, followed by the section's pane
/// code: `ShipStrip-PDC-1`.
pub const STRIP_BUTTON_PREFIX: &str = "ShipStrip-";

/// A request to strip one section of the docked partner, raised by a strip
/// list Strip button.
#[derive(Message, Clone, Debug)]
pub struct SectionStripCommand {
    /// The partner's design section id.
    pub section: String,
}

/// The strip list container, hidden while the player is not docked with a
/// lootable partner and no strip runs.
#[derive(Component)]
pub(crate) struct ShipStripList;

/// The rows inside [`ShipStripList`], and the rows they were built from, so
/// the rows respawn only when one changes.
#[derive(Component, Default)]
pub(crate) struct ShipStripRows(Vec<StripRow>);

/// The progress line of a running strip.
#[derive(Component)]
pub(crate) struct ShipStripProgress;

/// The progress bar's fill.
#[derive(Component)]
pub(crate) struct ShipStripProgressFill;

/// One live partner section as the list shows it.
#[derive(Clone, Debug, PartialEq)]
struct StripRow {
    section: String,
    code: String,
    line: String,
    /// Whether the strip is allowed: the row shows a Strip button, or the
    /// refusal in the danger tone and no button.
    allowed: bool,
}

/// The docked partner a strip reads.
pub(crate) struct StripPartner<'a> {
    pub(crate) entity: Entity,
    /// The partner's port that holds the dock.
    berth: Entity,
    lootable: bool,
    design: Option<&'a ShipDesignSections>,
    /// Every live section: entity, design section id, health share and
    /// neighbours.
    live: Vec<(Entity, &'a str, f32, &'a [Entity])>,
}

impl StripPartner<'_> {
    /// The live section with design section id `id`.
    fn live(&self, id: &str) -> Option<Entity> {
        self.live
            .iter()
            .find(|(_, live_id, ..)| *live_id == id)
            .map(|(entity, ..)| *entity)
    }

    /// The pane code of design section `id`, or the id when the partner has
    /// no design to number it from.
    fn code(&self, id: &str) -> String {
        self.design
            .and_then(|design| design_section_code(design, id))
            .unwrap_or_else(|| id.to_string())
    }

    /// The facts [`plan_section_strip`] reads for design section `id`:
    /// `None` when it is not live on the partner.
    fn target(&self, id: &str) -> Option<StripTarget<'_>> {
        let (entity, _, share, _) = *self.live.iter().find(|(_, live_id, ..)| *live_id == id)?;
        let recipe = &self.design?.get(id)?.config.base.build;
        Some(StripTarget {
            recipe,
            health_share: share,
            berth: entity == self.berth,
            splits_hull: strip_splits_hull(
                entity,
                self.live
                    .iter()
                    .map(|(section, _, _, neighbours)| (*section, *neighbours)),
            ),
        })
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
    fn player(&self) -> Option<Entity> {
        self.player.single().ok().map(|(player, _)| player)
    }

    /// The partner the player ship is docked with, or `None` while it is not
    /// docked.
    fn partner(&self) -> Option<StripPartner<'_>> {
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
        let (design, children, lootable, neutralized) = self.partners.get(entity).ok()?;
        let live = children
            .into_iter()
            .flatten()
            .filter_map(|child| {
                let (id, health, connected) = self.sections.get(*child).ok()?;
                let share = if health.max > 0.0 {
                    health.current / health.max
                } else {
                    0.0
                };
                let neighbours = connected.map_or(&[][..], |connected| connected.0.as_slice());
                Some((*child, id.0.as_str(), share, neighbours))
            })
            .collect();
        Some(StripPartner {
            entity,
            berth,
            lootable: lootable || neutralized,
            design,
            live,
        })
    }
}

/// Plan stripping design section `id` of `partner` into `own`.
fn plan(
    items: &GameItems,
    partner: Option<&StripPartner<'_>>,
    busy: bool,
    id: &str,
    own: &ShipInventory,
) -> Result<SectionStrip, SectionStripRefusalType> {
    plan_section_strip(
        items,
        partner.map(|partner| partner.lootable),
        busy,
        partner.and_then(|partner| partner.target(id)),
        own,
    )
}

/// The note line for a refused strip; the list and the handlers share it.
fn strip_refusal_text(code: &str, refusal: &SectionStripRefusalType) -> String {
    match refusal {
        SectionStripRefusalType::NotDocked => "strip: dock with a wreck first".to_string(),
        SectionStripRefusalType::Busy => "strip: another section is being stripped".to_string(),
        SectionStripRefusalType::NotLootable => {
            "strip: the docked hull is not a wreck you may loot".to_string()
        }
        SectionStripRefusalType::SectionGone => format!("strip: {code} is already gone"),
        SectionStripRefusalType::Berth => format!("strip: {code} holds the dock"),
        SectionStripRefusalType::SplitsHull => {
            format!("strip: taking {code} would split the hull")
        }
        SectionStripRefusalType::NoParts => {
            format!("strip: {code} is too damaged to give a whole part")
        }
        SectionStripRefusalType::HoldFull { need_g, free_g } => format!(
            "strip: {code} needs {} of hold, {} free",
            kg_text(*need_g),
            kg_text(u64::from(*free_g))
        ),
    }
}

/// The line for a strip the list can offer: the parts, the health and the
/// time.
fn strip_offer_text(strip: &SectionStrip, share: f32) -> String {
    let parts: u32 = strip.parts.values().sum();
    format!(
        "{parts} part{} at {:.0}% health, {:.0} s",
        if parts == 1 { "" } else { "s" },
        share * 100.0,
        strip.duration_secs
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

/// The parts a finished strip gave, by item name.
fn strip_parts_text(items: &GameItems, parts: &BTreeMap<ItemDesignId, u32>) -> String {
    parts
        .iter()
        .map(|(item, count)| format!("{count} {}", items.design(item).name))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Build the strip list under the station lists: a header, the progress
/// line and bar, and the rows [`update_ship_strip_list`] fills.
pub(crate) fn spawn_ship_strip_list(panel: &mut ChildSpawnerCommands) {
    panel
        .spawn((
            ShipStripList,
            Name::new(STRIP_LIST),
            Node {
                display: Display::None,
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(8.0),
                flex_shrink: 0.0,
                ..default()
            },
        ))
        .with_children(|list| {
            list.spawn(crate::pane::divider());
            list.spawn(themed_label(
                "Strip the docked wreck",
                13.0,
                UiColor::Accent,
            ));
            list.spawn((
                ShipStripProgress,
                Name::new(STRIP_PROGRESS),
                themed_label("", 12.0, UiColor::Primary),
            ));
            list.spawn((
                Node {
                    height: Val::Px(6.0),
                    flex_shrink: 0.0,
                    border_radius: BorderRadius::all(Val::Px(3.0)),
                    overflow: Overflow::clip(),
                    ..default()
                },
                BackgroundColor(Color::NONE),
                ThemedFill::alpha(UiColor::Secondary, 0.25),
            ))
            .with_children(|track| {
                track.spawn((
                    ShipStripProgressFill,
                    Node {
                        width: Val::Percent(0.0),
                        height: Val::Percent(100.0),
                        ..default()
                    },
                    BackgroundColor(Color::NONE),
                    ThemedFill::new(UiColor::Accent),
                ));
            });
            list.spawn((
                ShipStripRows::default(),
                Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(6.0),
                    ..default()
                },
            ));
        });
}

/// Show the strip list while the player is docked with a lootable partner
/// or a strip runs, keep the progress line on the running strip, and respawn
/// the rows when a code, a yield or a refusal changes. While a strip runs the
/// rows are empty and the progress line stands alone.
#[expect(
    clippy::type_complexity,
    reason = "the list, its progress line and bar, and its rows"
)]
pub(crate) fn update_ship_strip_list(
    mut commands: Commands,
    items: Option<Res<GameItems>>,
    site: StripSite,
    q_player: Query<(&ShipInventory, Option<&SectionStripJob>), With<PlayerSpaceshipMarker>>,
    mut q_list: Query<&mut Node, (With<ShipStripList>, Without<ShipStripProgressFill>)>,
    mut q_progress: Query<&mut Text, With<ShipStripProgress>>,
    mut q_fill: Query<&mut Node, (With<ShipStripProgressFill>, Without<ShipStripList>)>,
    mut q_rows: Query<(Entity, &mut ShipStripRows)>,
) {
    let Ok(mut list) = q_list.single_mut() else {
        return;
    };
    let Ok((rows_entity, mut shown)) = q_rows.single_mut() else {
        return;
    };
    let player = site.player().and_then(|player| q_player.get(player).ok());
    let job = player.and_then(|(_, job)| job);
    let partner = site.partner().filter(|partner| partner.lootable);
    // While a strip runs the list shows its progress alone: every row would
    // read the same Busy refusal.
    let mut rows: Vec<StripRow> = match (&partner, player, items.as_deref(), job) {
        (Some(partner), Some((own, _)), Some(items), None) => partner
            .live
            .iter()
            .map(|&(_, id, share, _)| {
                let code = partner.code(id);
                let (line, allowed) = match plan(items, Some(partner), false, id, own) {
                    Ok(strip) => (strip_offer_text(&strip, share), true),
                    Err(refusal) => (strip_refusal_text(&code, &refusal), false),
                };
                StripRow {
                    section: id.to_string(),
                    code,
                    line,
                    allowed,
                }
            })
            .collect(),
        _ => Vec::new(),
    };
    rows.sort_by_cached_key(|row| code_order(&row.code));
    let display = if rows.is_empty() && job.is_none() {
        Display::None
    } else {
        Display::Flex
    };
    if list.display != display {
        list.display = display;
    }
    let (progress, share) = match job {
        Some(job) => (
            format!(
                "Stripping {}: {:.0}%, {:.0} s left",
                job.code,
                job.progress() * 100.0,
                job.remaining_secs().ceil()
            ),
            job.progress(),
        ),
        None => ("No strip running".to_string(), 0.0),
    };
    if let Ok(mut text) = q_progress.single_mut() {
        if text.0 != progress {
            text.0 = progress;
        }
    }
    if let Ok(mut fill) = q_fill.single_mut() {
        let width = Val::Percent(share * 100.0);
        if fill.width != width {
            fill.width = width;
        }
    }
    if shown.0 == rows {
        return;
    }
    commands.entity(rows_entity).despawn_children();
    commands.entity(rows_entity).with_children(|parent| {
        for row in &rows {
            parent
                .spawn(Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(4.0),
                    ..default()
                })
                .with_children(|cell| {
                    cell.spawn(themed_label(&row.code, 13.0, UiColor::Primary));
                    if !row.allowed {
                        cell.spawn(themed_label(&row.line, 12.0, UiColor::Danger));
                        return;
                    }
                    cell.spawn(themed_label(&row.line, 12.0, UiColor::Body));
                    cell.spawn((
                        Name::new(format!("{STRIP_BUTTON_PREFIX}{}", row.code)),
                        compact_button(ButtonSpec::new("Strip").fit().primary()),
                    ))
                    .observe(on_ship_strip_button(row.section.clone()));
                });
        }
    });
    shown.0 = rows;
}

/// Raise a [`SectionStripCommand`] for design section `section` when its
/// Strip button is clicked.
fn on_ship_strip_button(
    section: String,
) -> impl FnMut(On<Activate>, MessageWriter<SectionStripCommand>, Option<Res<SoundBank<UiSfx>>>, Commands)
{
    move |_activate, mut strips, bank, mut commands| {
        strips.write(SectionStripCommand {
            section: section.clone(),
        });
        play_menu_select(&mut commands, bank.as_deref());
    }
}

/// Apply [`SectionStripCommand`]s by the [`plan_section_strip`] rule: start
/// a [`SectionStripJob`] on the player ship. Nothing moves until the job
/// ends. A refusal starts nothing and shows its reason on the note line.
pub(crate) fn apply_ship_strip_commands(
    mut messages: MessageReader<SectionStripCommand>,
    mut runtime: ResMut<ShipRuntime>,
    mut commands: Commands,
    items: Option<Res<GameItems>>,
    site: StripSite,
    q_player: Query<(&ShipInventory, Has<SectionStripJob>), With<PlayerSpaceshipMarker>>,
) {
    if messages.is_empty() {
        return;
    }
    // One strip per frame: the job lands with the commands, so a second
    // command this frame would plan as not busy.
    let Some(command) = messages.read().last().cloned() else {
        return;
    };
    messages.clear();
    let (Some(player), Some(items)) = (site.player(), items) else {
        return;
    };
    let (own, busy) = q_player
        .get(player)
        .expect("the player ship carries a ShipInventory");
    let partner = site.partner();
    let code = partner.as_ref().map_or_else(
        || command.section.clone(),
        |partner| partner.code(&command.section),
    );
    let strip = match plan(&items, partner.as_ref(), busy, &command.section, own) {
        Ok(strip) => strip,
        Err(refusal) => {
            info!("strip: refused '{}': {refusal:?}", command.section);
            runtime.note = Some((strip_refusal_text(&code, &refusal), 2.5));
            return;
        }
    };
    let partner = partner.expect("a planned strip has a docked partner");
    let section = partner
        .live(&command.section)
        .expect("a planned strip has a live section");
    info!(
        "strip: {code} of {:?} starts, {:.0} s",
        partner.entity, strip.duration_secs
    );
    runtime.note = Some((
        format!("stripping {code}: {:.0} s", strip.duration_secs),
        2.5,
    ));
    commands.entity(player).insert(SectionStripJob {
        partner: partner.entity,
        section,
        section_id: command.section,
        code,
        elapsed_secs: 0.0,
        duration_secs: strip.duration_secs,
    });
}

/// Run the player ship's [`SectionStripJob`] on the game clock. It stops
/// with no change when the player is no longer docked with the partner or
/// the section is gone. When the timer ends the strip is planned again; on
/// a pass the parts go into the hold, one `Stripped` report is queued and a
/// [`SectionStripRequest`] takes the section off; on a refusal nothing moves
/// and the reason shows on the note line.
///
/// # Panics
///
/// On a finished strip whose partner has no scenario id: every scenario ship
/// and station wears one.
pub(crate) fn advance_ship_strip(
    time: Res<Time>,
    mut runtime: ResMut<ShipRuntime>,
    mut commands: Commands,
    items: Option<Res<GameItems>>,
    site: StripSite,
    q_ids: Query<&EntityId>,
    mut q_player: Query<
        (
            &mut SectionStripJob,
            &mut ShipInventory,
            &mut ShipInventoryReports,
        ),
        With<PlayerSpaceshipMarker>,
    >,
) {
    let Some(player) = site.player() else {
        return;
    };
    let Ok((mut job, mut own, mut reports)) = q_player.get_mut(player) else {
        return;
    };
    let partner = site
        .partner()
        .filter(|partner| partner.entity == job.partner);
    let stop = match &partner {
        None => Some("you undocked"),
        Some(partner) if partner.live(&job.section_id) != Some(job.section) => {
            Some("the section is gone")
        }
        Some(_) => None,
    };
    if let Some(reason) = stop {
        info!("strip: {} stopped: {reason}", job.code);
        runtime.note = Some((format!("strip of {} stopped: {reason}", job.code), 2.5));
        commands.entity(player).remove::<SectionStripJob>();
        return;
    }
    job.elapsed_secs += time.delta_secs();
    if !job.is_done() {
        return;
    }
    let Some(items) = items else {
        return;
    };
    let partner = partner.expect("a running strip has its partner");
    commands.entity(player).remove::<SectionStripJob>();
    let strip = match plan(&items, Some(&partner), false, &job.section_id, &own) {
        Ok(strip) => strip,
        Err(refusal) => {
            info!("strip: {} stopped at the end: {refusal:?}", job.code);
            runtime.note = Some((strip_refusal_text(&job.code, &refusal), 2.5));
            return;
        }
    };
    let hull = q_ids
        .get(partner.entity)
        .unwrap_or_else(|_| panic!("strip: partner {:?} has no scenario id", partner.entity));
    for (item, &count) in &strip.parts {
        own.add(&items, item, count);
    }
    let text = strip_parts_text(&items, &strip.parts);
    info!("strip: {} of '{}' gave {text}", job.code, hull.0);
    reports.0.push(ShipInventoryReport::Stripped {
        hull: hull.0.clone(),
        section: job.code.clone(),
        parts: strip.parts,
    });
    commands.trigger(SectionStripRequest {
        entity: job.section,
    });
    runtime.note = Some((format!("stripped {}: {text}", job.code), 2.5));
}
