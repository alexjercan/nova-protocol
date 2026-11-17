//! The Ship pane's lost-section list and the Rebuild action.
//!
//! A lost section is a section of the player ship's design with no live
//! section of that id. The panel lists each lost section with its design
//! code, what a rebuild takes, a short part or balance as have / need, or
//! the cause of another refusal, and a Rebuild button. A
//! click raises a [`SectionRebuildCommand`]; [`apply_ship_rebuild_commands`]
//! plans it with [`plan_section_rebuild`] at the site the ship is at:
//!
//! - Docked: the station rebuilds at once. The handler takes the parts and
//!   the credits and raises the [`SectionRebuildRequest`] the scenario crate
//!   spawns the section from.
//! - Undocked: a field rebuild. The handler starts a [`RepairJob`] that takes
//!   the recipe's parts from the hold when its time is up and raises the same
//!   request. The parts other jobs hold back are not free for it.
//!
//! Touch this module when changing what the lost list shows or what Rebuild
//! takes.

use std::collections::{BTreeMap, HashSet};

use bevy::{ecs::system::SystemParam, prelude::*, ui_widgets::Activate};
use nova_events::prelude::{CreditChangeCauseType, EntityId};
use nova_gameplay::prelude::*;
use nova_ship::prelude::*;
use nova_sound::prelude::*;
use nova_ui::{
    theme::UiColorType,
    widget::{ButtonCueType, ButtonSpec},
};

use super::{scene::ShipRuntime, sections::design_section_code};
use crate::pane::{compact_button, themed_label};

/// A request to rebuild one lost section of the player ship, raised by a
/// lost-list Rebuild button.
#[derive(Message, Clone, Debug)]
pub struct SectionRebuildCommand {
    /// The design section id.
    pub section: String,
}

/// The lost-section list container, hidden while nothing is lost.
#[derive(Component)]
pub(crate) struct ShipRebuildList;

/// The rows inside [`ShipRebuildList`], and the row texts they were built
/// from, so the rows respawn only when one changes.
#[derive(Component, Default)]
pub(crate) struct ShipRebuildRows(Vec<RebuildRow>);

/// One lost section as the list shows it.
#[derive(Clone, Debug, PartialEq)]
struct RebuildRow {
    section: String,
    code: String,
    /// What a rebuild takes, the short count, or why there is none.
    line: String,
    /// Whether the row offers Rebuild: the plan holds and no field rebuild
    /// runs on the section.
    offered: bool,
    /// The line is a short count, painted in the danger tone.
    short: bool,
}

/// A live section of the player ship, placed in ship-root space.
pub(crate) struct LiveSection {
    pub(crate) entity: Entity,
    pub(crate) id: String,
    pub(crate) position: Vec3,
    pub(crate) rotation: Quat,
    link_points: Vec<LinkPoint>,
    /// Whether the section works: a collapse-disabled section stays live
    /// with `SectionInactiveMarker` and does nothing.
    pub(crate) working: bool,
}

/// The read-only facts a station service (a rebuild or a swap) is planned
/// from: the player ship's design and live sections, and the station it is
/// docked with.
#[derive(SystemParam)]
pub(crate) struct StationSite<'w, 's> {
    player: Query<
        'w,
        's,
        (
            Entity,
            Option<&'static ShipDesignSections>,
            Option<&'static DockedShip>,
            Option<&'static Children>,
        ),
        (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>),
    >,
    connections: Query<'w, 's, &'static DockingConnection>,
    stations: Query<
        'w,
        's,
        (
            Option<&'static StationServices>,
            Option<&'static StationBuilds>,
            Has<DerelictShipMarker>,
        ),
        With<StationMarker>,
    >,
    sections: Query<
        'w,
        's,
        (
            &'static EntityId,
            &'static Transform,
            &'static SectionLinkPoints,
            Has<SectionInactiveMarker>,
        ),
        With<SectionMarker>,
    >,
    /// Every section's root, world pose and collider, read for the docked
    /// partner's sections ([`StationSite::partner_colliders`]).
    outside: Query<
        'w,
        's,
        (
            &'static ChildOf,
            &'static GlobalTransform,
            Option<&'static SectionCollider>,
        ),
        With<SectionMarker>,
    >,
    roots: Query<'w, 's, &'static GlobalTransform, With<SpaceshipRootMarker>>,
    /// Every docked ship root, read for the ships docked to the partner,
    /// such as the hulls a station parks for sale.
    docked: Query<'w, 's, (Entity, &'static DockedShip), With<SpaceshipRootMarker>>,
}

/// How far past the player ship's farthest live section origin, in engine
/// units (cells), a docked partner's section still counts as near: it is
/// drawn in the Ship view and checked by the add fit. Four cells is 40 m,
/// past the largest catalogue part.
pub(crate) const PARTNER_REACH_MARGIN: f32 = 4.0;

/// What [`StationSite::resolve`] found: the player ship, its design, its
/// live sections, and the partner it is docked with, whether that partner
/// is a living station offering the service asked about, and the section
/// prototypes it builds.
pub(crate) struct ResolvedSite<'a> {
    pub(crate) player: Entity,
    pub(crate) design: &'a ShipDesignSections,
    pub(crate) live: Vec<LiveSection>,
    pub(crate) partner: Option<(Entity, bool, &'a [String])>,
    /// The player ship's port the dock holds by, while it is docked.
    pub(crate) port: Option<Entity>,
}

impl StationSite<'_, '_> {
    /// The partner the player ship `player` is docked with through `docked`,
    /// whether it is a living station that offers `service`, and the section
    /// prototypes it builds: its [`StationBuilds`], none for a partner that
    /// is no station.
    ///
    /// # Panics
    ///
    /// On a station that carries [`StationServices`] without
    /// [`StationBuilds`]: every station root spawns and thaws with both.
    fn partner(
        &self,
        player: Entity,
        docked: &DockedShip,
        service: StationServiceType,
    ) -> (Entity, bool, &[String]) {
        let connection = self.connections.get(docked.connection).unwrap_or_else(|_| {
            panic!(
                "player ship {player:?} is docked through {:?}, which has no DockingConnection",
                docked.connection
            )
        });
        let other = if connection.first_ship == player {
            connection.second_ship
        } else {
            connection.first_ship
        };
        let (offers, builds) = match self.stations.get(other) {
            Ok((services, builds, derelict)) => {
                let builds = match (services, builds) {
                    (Some(_), None) => {
                        panic!("station {other:?} carries StationServices but no StationBuilds")
                    }
                    (_, Some(builds)) => builds.0.as_slice(),
                    (None, None) => &[],
                };
                (station_offers(services, derelict, service), builds)
            }
            Err(_) => (false, &[][..]),
        };
        (other, offers, builds)
    }

    /// Whether the `service` gate holds: the player ship has a design and is
    /// docked with a living station that offers `service`. Reads no
    /// sections, so a list can ask every frame.
    pub(crate) fn gate(&self, service: StationServiceType) -> bool {
        self.player
            .single()
            .ok()
            .and_then(|(player, design, docked, _)| design.and(docked).map(|d| (player, d)))
            .is_some_and(|(player, docked)| self.partner(player, docked, service).1)
    }

    /// The player ship and its facts for `service`, or `None` when there is
    /// no player ship or it has no design to plan from.
    pub(crate) fn resolve(&self, service: StationServiceType) -> Option<ResolvedSite<'_>> {
        let (player, design, docked, children) = self.player.single().ok()?;
        let design = design?;
        let live = children
            .into_iter()
            .flatten()
            .filter_map(|child| self.sections.get(*child).ok().map(|found| (*child, found)))
            .map(|(entity, (id, transform, points, inactive))| LiveSection {
                entity,
                id: id.0.clone(),
                position: transform.translation,
                rotation: transform.rotation,
                link_points: points.0.clone(),
                working: !inactive,
            })
            .collect();
        let partner = docked.map(|docked| self.partner(player, docked, service));
        let port = docked
            .and_then(|docked| self.connections.get(docked.connection).ok())
            .map(|connection| {
                if connection.first_ship == player {
                    connection.first_section
                } else {
                    connection.second_section
                }
            });
        Some(ResolvedSite {
            player,
            design,
            live,
            partner,
            port,
        })
    }
}

impl StationSite<'_, '_> {
    /// The colliders of the docked partner's sections, and of the sections of
    /// every other ship docked to it (a station's parked hulls), near the
    /// player ship, in its ship-root space: those whose box comes within
    /// [`PARTNER_REACH_MARGIN`] of the ship's farthest live section origin.
    /// Empty while the ship is not docked. The add fit refuses a part that
    /// overlaps one, and the Ship view draws them.
    pub(crate) fn partner_colliders(
        &self,
        resolved: &ResolvedSite<'_>,
    ) -> Vec<PlacedSectionCollider> {
        let Some((partner, ..)) = resolved.partner else {
            return Vec::new();
        };
        let Ok(ship) = self.roots.get(resolved.player) else {
            return Vec::new();
        };
        let reach = resolved
            .live
            .iter()
            .map(|section| section.position.length())
            .fold(0.0_f32, f32::max)
            + PARTNER_REACH_MARGIN;
        let berthed: Vec<Entity> = self
            .docked
            .iter()
            .filter(|&(ship, docked)| {
                ship != resolved.player
                    && self
                        .connections
                        .get(docked.connection)
                        .is_ok_and(|connection| connection.joins(partner))
            })
            .map(|(ship, _)| ship)
            .collect();
        self.outside
            .iter()
            .filter(|(root, ..)| root.parent() == partner || berthed.contains(&root.parent()))
            .map(|(_, pose, collider)| {
                let local = pose.reparented_to(ship);
                PlacedSectionCollider {
                    position: local.translation,
                    rotation: local.rotation,
                    collider: collider.copied().unwrap_or_default(),
                }
            })
            .filter(|placed| placed.position.length() - placed.half_extents().length() <= reach)
            .collect()
    }
}

impl ResolvedSite<'_> {
    /// The design sections with no live section of that id, in design order.
    fn lost(&self) -> impl Iterator<Item = &DesignSection> {
        let live: HashSet<&str> = self
            .live
            .iter()
            .map(|section| section.id.as_str())
            .collect();
        self.design
            .0
            .iter()
            .filter(move |section| !live.contains(section.id.as_str()))
    }

    /// Plan rebuilding design section `section` against the hold `own`: at
    /// the docked partner with the balance `own_cr` and the partner's
    /// `stock` and balance, or in the field while the ship is not docked.
    fn plan(
        &self,
        items: &GameItems,
        section: &DesignSection,
        own: &ShipInventory,
        own_cr: u32,
        partner: Option<(&ShipInventory, u32)>,
    ) -> Result<RecipePayment, SectionRebuildRefusalType> {
        let live: Vec<(&str, PlacedSectionLinkPoints<'_>)> = self
            .live
            .iter()
            .map(|live| {
                (
                    live.id.as_str(),
                    PlacedSectionLinkPoints {
                        position: live.position,
                        rotation: live.rotation,
                        link_points: &live.link_points,
                    },
                )
            })
            .collect();
        let facts = lost_section_facts(section, &live);
        let site = match self.partner {
            None => RebuildSiteType::Field,
            Some((_, offers, builds)) => {
                let (stock, credits_cr) =
                    partner.expect("a docked partner carries a ShipInventory and ShipCredits");
                RebuildSiteType::Station {
                    partner: StationPartner {
                        offers,
                        builds,
                        stock,
                        credits_cr,
                    },
                    own_cr,
                }
            }
        };
        plan_section_rebuild(
            items,
            &section.config.base.id,
            &section.config.base.build,
            site,
            facts.lost,
            facts.anchored,
            own,
        )
    }

    /// Whether the ship is in the field: not docked, so a rebuild is a
    /// field rebuild job.
    fn in_field(&self) -> bool {
        self.partner.is_none()
    }
}

/// The note line for a refused rebuild; the list and the handler share it.
/// `field` is true for a field rebuild, whose parts come from the hold alone.
fn rebuild_refusal_text(
    items: &GameItems,
    code: &str,
    field: bool,
    refusal: &SectionRebuildRefusalType,
) -> String {
    match refusal {
        SectionRebuildRefusalType::NoRebuildService => {
            "rebuild: this dock offers no rebuild; undock to rebuild in the field".to_string()
        }
        SectionRebuildRefusalType::NotBuilt => {
            format!("rebuild: this station does not build {code}")
        }
        SectionRebuildRefusalType::SectionNotLost => format!("rebuild: {code} is not lost"),
        SectionRebuildRefusalType::Unanchored => {
            format!("rebuild: {code} has no live section to mount on")
        }
        SectionRebuildRefusalType::Payment(RecipePaymentRefusalType::MissingParts {
            item,
            short,
        }) if field => format!(
            "rebuild: {code} needs {short} more {} free in the hold",
            items.design(item).name
        ),
        SectionRebuildRefusalType::Payment(refusal) => {
            format!("rebuild: {}", payment_refusal_text(items, code, refusal))
        }
    }
}

/// The lost row's line for a refused rebuild. A short part or a short
/// balance reads as a have / need count under the row's code; the other
/// refusals name their cause. Returns the line and whether it is a short
/// count.
fn rebuild_row_refusal(
    items: &GameItems,
    code: &str,
    field: bool,
    holds: (&ShipInventory, Option<&ShipInventory>),
    refusal: &SectionRebuildRefusalType,
) -> (String, bool) {
    match refusal {
        SectionRebuildRefusalType::Payment(RecipePaymentRefusalType::MissingParts {
            item,
            short,
        }) => {
            let (own, partner) = holds;
            let have = own.count(item)
                + partner
                    .filter(|_| !field)
                    .map_or(0, |partner| partner.count(item));
            (
                format!(
                    "{} {have} / {}",
                    items.design(item).name,
                    have.saturating_add(*short)
                ),
                true,
            )
        }
        SectionRebuildRefusalType::Payment(RecipePaymentRefusalType::InsufficientCredits {
            need,
            have,
        }) => (
            format!("{} / {}", cr_digits(u64::from(*have)), cr_text(*need)),
            true,
        ),
        refusal => (rebuild_refusal_text(items, code, field, refusal), false),
    }
}

/// Why a recipe cannot be paid, after the action's prefix; a rebuild and a
/// swap share it.
pub(crate) fn payment_refusal_text(
    items: &GameItems,
    code: &str,
    refusal: &RecipePaymentRefusalType,
) -> String {
    match refusal {
        RecipePaymentRefusalType::MissingParts { item, short } => format!(
            "{code} needs {short} more {} than both holds carry",
            items.design(item).name
        ),
        RecipePaymentRefusalType::InsufficientCredits { need, have } => {
            format!(
                "{code} costs {}, the ship holds {}",
                cr_text(*need),
                cr_text(*have)
            )
        }
        RecipePaymentRefusalType::CreditOverflow => {
            format!("the station cannot take the price of {code}")
        }
    }
}

/// The price line for a rebuild or swap the list can offer.
pub(crate) fn rebuild_price_text(rebuild: &RecipePayment) -> String {
    let held: u32 = rebuild.from_hold.values().sum();
    let bought: u32 = rebuild.bought.values().sum();
    format!(
        "{}: {held} parts from the hold, {bought} bought",
        cr_text(rebuild.price_cr)
    )
}

/// The line for a field rebuild the list can offer: its time and the parts
/// it takes from the hold, such as `field: 20 s, 4 parts from the hold`.
fn field_rebuild_text(section: &DesignSection) -> String {
    let recipe = &section.config.base.build;
    format!(
        "field: {:.0} s, {} parts from the hold",
        field_rebuild_duration_secs(recipe),
        recipe.part_count()
    )
}

/// Build the lost list under the repair form: a header and the rows
/// [`update_ship_rebuild_list`] fills.
pub(crate) fn spawn_ship_rebuild_list(panel: &mut ChildSpawnerCommands) {
    panel
        .spawn((
            ShipRebuildList,
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
            list.spawn(themed_label("Lost sections", 13.0, UiColorType::Accent));
            list.spawn((
                ShipRebuildRows::default(),
                Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(6.0),
                    ..default()
                },
            ));
        });
}

/// Show the lost list while a section is lost, and respawn its rows when a
/// code, a price, a refusal or a running field rebuild changes. A section a
/// field rebuild job works on shows no button: the job row has its progress.
pub(crate) fn update_ship_rebuild_list(
    mut commands: Commands,
    items: Option<Res<GameItems>>,
    site: StationSite,
    q_holds: Query<(&ShipInventory, &ShipCredits, Option<&ShipRepairJobs>)>,
    mut q_list: Query<&mut Node, With<ShipRebuildList>>,
    mut q_rows: Query<(Entity, &mut ShipRebuildRows)>,
) {
    let Ok(mut list) = q_list.single_mut() else {
        return;
    };
    let Ok((rows_entity, mut shown)) = q_rows.single_mut() else {
        return;
    };
    let rows = match (site.resolve(StationServiceType::Rebuild), items) {
        (Some(resolved), Some(items)) => {
            let (own, own_cr, jobs) = q_holds
                .get(resolved.player)
                .expect("the player ship carries a ShipInventory and ShipCredits");
            let jobs = jobs.cloned().unwrap_or_default();
            let free = jobs.free_hold(own);
            let partner = resolved
                .partner
                .and_then(|(partner, ..)| q_holds.get(partner).ok())
                .map(|(stock, credits, _)| (stock, credits.0));
            resolved
                .lost()
                .map(|section| {
                    let code = design_section_code(resolved.design, &section.id)
                        .expect("a lost section is a design section");
                    let running = jobs.get(&RepairJobKey::Lost(section.id.clone())).is_some();
                    // A running field rebuild shows on its Repairs row; its
                    // lost row keeps only the code.
                    let line = if running {
                        Err((String::new(), false))
                    } else {
                        match resolved.plan(&items, section, &free, own_cr.0, partner) {
                            Ok(_) if resolved.in_field() => Ok(field_rebuild_text(section)),
                            Ok(rebuild) => Ok(rebuild_price_text(&rebuild)),
                            Err(refusal) => Err(rebuild_row_refusal(
                                &items,
                                &code,
                                resolved.in_field(),
                                (&free, partner.map(|(stock, _)| stock)),
                                &refusal,
                            )),
                        }
                    };
                    let (line, short) = match line {
                        Ok(line) => (Ok(line), false),
                        Err((line, short)) => (Err(line), short),
                    };
                    RebuildRow {
                        section: section.id.clone(),
                        code,
                        offered: line.is_ok(),
                        line: line.unwrap_or_else(|line| line),
                        short,
                    }
                })
                .collect()
        }
        _ => Vec::new(),
    };
    let display = if rows.is_empty() {
        Display::None
    } else {
        Display::Flex
    };
    if list.display != display {
        list.display = display;
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
                    cell.spawn(themed_label(&row.code, 13.0, UiColorType::Text));
                    if !row.line.is_empty() {
                        let tone = if row.short {
                            UiColorType::Danger
                        } else {
                            UiColorType::TextBody
                        };
                        cell.spawn(themed_label(&row.line, 12.0, tone));
                    }
                    if row.offered {
                        cell.spawn((
                            Name::new(format!("ShipRebuild-{}", row.code)),
                            compact_button(ButtonSpec::new("Rebuild").fit().primary()),
                            ButtonCueType::Owned,
                        ))
                        .observe(on_ship_rebuild_button(row.section.clone()));
                    }
                });
        }
    });
    shown.0 = rows;
}

/// Raise a [`SectionRebuildCommand`] for design section `section` when its
/// Rebuild button is clicked.
fn on_ship_rebuild_button(
    section: String,
) -> impl FnMut(On<Activate>, MessageWriter<SectionRebuildCommand>) {
    move |_activate, mut rebuilds| {
        rebuilds.write(SectionRebuildCommand {
            section: section.clone(),
        });
    }
}

/// Apply [`SectionRebuildCommand`]s by the [`plan_section_rebuild`] rule.
/// Docked: take the hold's parts, buy the rest from the station at ask, pay
/// the price to the station, queue one `Rebuilt` report, and raise the
/// [`SectionRebuildRequest`] that spawns the section at full health.
/// Undocked: start a field rebuild [`RepairJob`] on the parts free in the
/// hold. A refusal changes nothing and shows its reason on the note line.
#[expect(
    clippy::too_many_arguments,
    reason = "the site, both holds, the jobs and the cue bank"
)]
pub(crate) fn apply_ship_rebuild_commands(
    mut messages: MessageReader<SectionRebuildCommand>,
    mut runtime: ResMut<ShipRuntime>,
    mut commands: Commands,
    items: Option<Res<GameItems>>,
    site: StationSite,
    mut q_holds: Query<(
        &mut ShipInventory,
        &mut ShipCredits,
        &mut ShipInventoryReports,
    )>,
    mut q_jobs: Query<&mut ShipRepairJobs>,
    bank: Option<Res<SoundBank<UiSfx>>>,
) {
    if messages.is_empty() {
        return;
    }
    let (Some(resolved), Some(items)) = (site.resolve(StationServiceType::Rebuild), items) else {
        messages.clear();
        return;
    };
    // One rebuild a frame: the site was read before it, and the section it
    // restores stays lost until the commands run.
    let mut rebuilt = false;
    for command in messages.read() {
        if rebuilt {
            continue;
        }
        let section = resolved.design.get(&command.section).unwrap_or_else(|| {
            panic!(
                "rebuild: the player ship's design has no section '{}'",
                command.section
            )
        });
        let code =
            design_section_code(resolved.design, &section.id).expect("a design section has a code");
        let jobs = q_jobs.get(resolved.player).ok();
        if resolved.in_field()
            && jobs.is_some_and(|jobs| jobs.get(&RepairJobKey::Lost(section.id.clone())).is_some())
        {
            runtime.note = Some((format!("rebuild: {code} is already rebuilding"), 2.5));
            play_menu_deny(&mut commands, bank.as_deref());
            continue;
        }
        let planned = {
            let (own, own_cr, _) = q_holds
                .get(resolved.player)
                .expect("the player ship carries a ShipInventory and ShipCredits");
            let free = jobs.cloned().unwrap_or_default().free_hold(&own);
            let partner = resolved
                .partner
                .and_then(|(partner, ..)| q_holds.get(partner).ok())
                .map(|(stock, credits, _)| (stock, credits.0));
            resolved.plan(&items, section, &free, own_cr.0, partner)
        };
        let rebuild = match planned {
            Ok(rebuild) => rebuild,
            Err(refusal) => {
                runtime.note = Some((
                    rebuild_refusal_text(&items, &code, resolved.in_field(), &refusal),
                    2.5,
                ));
                play_menu_deny(&mut commands, bank.as_deref());
                continue;
            }
        };
        rebuilt = true;
        play_menu_select(&mut commands, bank.as_deref());
        let Some((station, ..)) = resolved.partner else {
            let job = RepairJob::field_rebuild(
                section.id.clone(),
                code.clone(),
                &section.config.base.build,
            );
            // The job row in the Repairs block shows the job; an earlier
            // refusal's note goes.
            runtime.note = None;
            match q_jobs.get_mut(resolved.player) {
                Ok(mut jobs) => jobs.push(job),
                Err(_) => {
                    commands
                        .entity(resolved.player)
                        .insert(ShipRepairJobs(vec![job]));
                }
            }
            continue;
        };
        let Ok(
            [(mut own, mut own_cr, mut own_reports), (mut stock, mut station_cr, mut station_reports)],
        ) = q_holds.get_many_mut([resolved.player, station])
        else {
            panic!("rebuild: the player ship and station {station:?} each carry a hold");
        };
        for (item, &count) in &rebuild.from_hold {
            own.remove(item, count);
        }
        for (item, &count) in &rebuild.bought {
            stock.remove(item, count);
        }
        let paid_cr = own_cr.0 - rebuild.price_cr;
        let earned_cr = station_cr.0 + rebuild.price_cr;
        own_reports.change_credits(&mut own_cr, paid_cr, CreditChangeCauseType::Rebuild);
        station_reports.change_credits(&mut station_cr, earned_cr, CreditChangeCauseType::Rebuild);
        let parts: BTreeMap<ItemDesignId, u32> = section.config.base.build.parts.clone();
        own_reports.0.push(ShipInventoryReport::Rebuilt {
            section: code.clone(),
            parts,
            price_cr: rebuild.price_cr,
        });
        commands.trigger(SectionRebuildRequest {
            entity: resolved.player,
            section: section.id.clone(),
        });
        runtime.note = Some((
            format!("rebuilt {code}: {}", rebuild_price_text(&rebuild)),
            2.5,
        ));
    }
}
