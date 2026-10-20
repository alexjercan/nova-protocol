//! The Ship pane's lost-section list and the Rebuild action.
//!
//! A lost section is a section of the player ship's design with no live
//! section of that id. While the ship is docked at a living station that
//! offers Rebuild, the panel lists each lost section with its design code, the
//! price or the reason it cannot be rebuilt, and a Rebuild button. A click
//! raises a [`SectionRebuildCommand`]; [`apply_ship_rebuild_commands`] plans
//! it with [`plan_section_rebuild`], takes the parts and the credits, and
//! raises the [`SectionRebuildRequest`] the scenario crate spawns the section
//! from.
//!
//! Touch this module when changing what the lost list shows or what Rebuild
//! takes.

use std::collections::{BTreeMap, HashSet};

use bevy::{ecs::system::SystemParam, prelude::*, ui_widgets::Activate};
use nova_events::prelude::{CreditChangeCauseType, EntityId};
use nova_gameplay::prelude::*;
use nova_ship::prelude::*;
use nova_ui::{theme::UiColor, widget::ButtonSpec};

use super::{scene::ShipRuntime, sections::design_section_code};
use crate::pane::{compact_button, play_menu_select, themed_label};

/// A request to rebuild one lost section of the player ship, raised by a
/// lost-list Rebuild button.
#[derive(Message, Clone, Debug)]
pub struct SectionRebuildCommand {
    /// The design section id.
    pub section: String,
}

/// The lost-section list container, hidden while the rebuild gate does not
/// hold or nothing is lost.
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
    line: String,
}

/// A live section of the player ship, placed in ship-root space.
pub(crate) struct LiveSection {
    pub(crate) entity: Entity,
    pub(crate) id: String,
    pub(crate) position: Vec3,
    pub(crate) rotation: Quat,
    link_points: Vec<LinkPoint>,
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
            .map(|(entity, (id, transform, points))| LiveSection {
                entity,
                id: id.0.clone(),
                position: transform.translation,
                rotation: transform.rotation,
                link_points: points.0.clone(),
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

    /// Plan rebuilding design section `section` against the hold `own`, the
    /// balance `own_cr` and the docked partner's `stock` and balance.
    fn plan(
        &self,
        items: &GameItems,
        section: &DesignSection,
        own: &ShipInventory,
        own_cr: u32,
        partner: Option<(&ShipInventory, u32)>,
    ) -> Result<RecipePayment, SectionRebuildRefusalType> {
        let lost = !self.live.iter().any(|live| live.id == section.id);
        let placed: Vec<PlacedSectionLinkPoints<'_>> = self
            .live
            .iter()
            .map(|live| PlacedSectionLinkPoints {
                position: live.position,
                rotation: live.rotation,
                link_points: &live.link_points,
            })
            .collect();
        let anchored = rebuild_anchored(
            &placed,
            PlacedSectionLinkPoints {
                position: section.position,
                rotation: section.rotation,
                link_points: &section.config.base.link_points,
            },
        );
        let partner =
            self.partner
                .zip(partner)
                .map(
                    |((_, offers, builds), (stock, credits_cr))| StationPartner {
                        offers,
                        builds,
                        stock,
                        credits_cr,
                    },
                );
        plan_section_rebuild(
            items,
            &section.config.base.id,
            &section.config.base.build,
            partner,
            lost,
            anchored,
            own,
            own_cr,
        )
    }
}

/// The note line for a refused rebuild; the list and the handler share it.
fn rebuild_refusal_text(
    items: &GameItems,
    code: &str,
    refusal: &SectionRebuildRefusalType,
) -> String {
    match refusal {
        SectionRebuildRefusalType::NotDocked => {
            "rebuild: dock at a station that rebuilds".to_string()
        }
        SectionRebuildRefusalType::NoRebuildService => {
            "rebuild: this dock offers no rebuild".to_string()
        }
        SectionRebuildRefusalType::NotBuilt => {
            format!("rebuild: this station does not build {code}")
        }
        SectionRebuildRefusalType::SectionNotLost => format!("rebuild: {code} is not lost"),
        SectionRebuildRefusalType::Unanchored => {
            format!("rebuild: {code} has no live section to mount on")
        }
        SectionRebuildRefusalType::Payment(refusal) => {
            format!("rebuild: {}", payment_refusal_text(items, code, refusal))
        }
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
            format!("{code} costs {need} cr, the ship holds {have} cr")
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
        "{} cr: {held} parts from the hold, {bought} bought",
        rebuild.price_cr
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
            list.spawn(themed_label("Lost sections", 13.0, UiColor::Accent));
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

/// Show the lost list while the rebuild gate holds and a section is lost,
/// and respawn its rows when a code, a price or a refusal changes.
pub(crate) fn update_ship_rebuild_list(
    mut commands: Commands,
    items: Option<Res<GameItems>>,
    site: StationSite,
    q_holds: Query<(&ShipInventory, &ShipCredits)>,
    mut q_list: Query<&mut Node, With<ShipRebuildList>>,
    mut q_rows: Query<(Entity, &mut ShipRebuildRows)>,
) {
    let Ok(mut list) = q_list.single_mut() else {
        return;
    };
    let Ok((rows_entity, mut shown)) = q_rows.single_mut() else {
        return;
    };
    let resolved = if site.gate(StationServiceType::Rebuild) {
        site.resolve(StationServiceType::Rebuild)
    } else {
        None
    };
    let rows = match (resolved, items) {
        (Some(resolved), Some(items)) => {
            let (own, own_cr) = q_holds
                .get(resolved.player)
                .expect("the player ship carries a ShipInventory and ShipCredits");
            let partner = resolved
                .partner
                .and_then(|(partner, ..)| q_holds.get(partner).ok())
                .map(|(stock, credits)| (stock, credits.0));
            resolved
                .lost()
                .map(|section| {
                    let code = design_section_code(resolved.design, &section.id)
                        .expect("a lost section is a design section");
                    let line = match resolved.plan(&items, section, own, own_cr.0, partner) {
                        Ok(rebuild) => rebuild_price_text(&rebuild),
                        Err(refusal) => rebuild_refusal_text(&items, &code, &refusal),
                    };
                    RebuildRow {
                        section: section.id.clone(),
                        code,
                        line,
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
                    cell.spawn(themed_label(&row.code, 13.0, UiColor::Primary));
                    cell.spawn(themed_label(&row.line, 12.0, UiColor::Body));
                    cell.spawn((
                        Name::new(format!("ShipRebuild-{}", row.code)),
                        compact_button(ButtonSpec::new("Rebuild").fit().primary()),
                    ))
                    .observe(on_ship_rebuild_button(row.section.clone()));
                });
        }
    });
    shown.0 = rows;
}

/// Raise a [`SectionRebuildCommand`] for design section `section` when its
/// Rebuild button is clicked.
fn on_ship_rebuild_button(
    section: String,
) -> impl FnMut(
    On<Activate>,
    MessageWriter<SectionRebuildCommand>,
    Option<Res<SoundBank<UiSfx>>>,
    Commands,
) {
    move |_activate, mut rebuilds, bank, mut commands| {
        rebuilds.write(SectionRebuildCommand {
            section: section.clone(),
        });
        play_menu_select(&mut commands, bank.as_deref());
    }
}

/// Apply [`SectionRebuildCommand`]s by the [`plan_section_rebuild`] rule:
/// take the hold's parts, buy the rest from the station at ask, pay the price
/// to the station, queue one `Rebuilt` report, and raise the
/// [`SectionRebuildRequest`] that spawns the section at full health. A
/// refusal changes nothing and shows its reason on the note line.
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
        let planned = {
            let (own, own_cr, _) = q_holds
                .get(resolved.player)
                .expect("the player ship carries a ShipInventory and ShipCredits");
            let partner = resolved
                .partner
                .and_then(|(partner, ..)| q_holds.get(partner).ok())
                .map(|(stock, credits, _)| (stock, credits.0));
            resolved.plan(&items, section, own, own_cr.0, partner)
        };
        let rebuild = match planned {
            Ok(rebuild) => rebuild,
            Err(refusal) => {
                runtime.note = Some((rebuild_refusal_text(&items, &code, &refusal), 2.5));
                continue;
            }
        };
        let (station, ..) = resolved
            .partner
            .expect("a planned rebuild has a docked partner");
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
        rebuilt = true;
    }
}
