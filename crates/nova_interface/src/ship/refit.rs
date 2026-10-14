//! The Ship pane's swap list and the Swap action.
//!
//! While the ship is docked at a living station that offers Refit, the panel
//! lists the parts the selected design section can be swapped for: every
//! catalogue part of the same kind and footprint ([`swap_offers`]), each with
//! the price or the reason it cannot be paid, and a Swap button. A click
//! raises a [`SectionSwapCommand`]; [`apply_ship_swap_commands`] plans it with
//! [`plan_section_swap`], checks the ship it leaves with [`swap_build_fault`],
//! takes the parts and the credits, and raises the [`SectionSwapRequest`] the
//! scenario crate replaces the section from.
//!
//! Touch this module when changing what the swap list offers or what Swap
//! takes.

use std::collections::BTreeMap;

use bevy::{prelude::*, ui_widgets::Activate};
use nova_events::prelude::{CreditChangeCauseType, EntityId};
use nova_gameplay::prelude::*;
use nova_ship::prelude::*;
use nova_ui::{theme::UiColor, widget::ButtonSpec};

use super::{
    rebuild::{payment_refusal_text, rebuild_price_text, ResolvedSite, StationSite},
    scene::ShipRuntime,
    sections::design_section_code,
};
use crate::pane::{compact_button, play_menu_select, themed_label};

/// A request to swap one live section of the player ship for another
/// catalogue part, raised by a swap-list Swap button.
#[derive(Message, Clone, Debug)]
pub struct SectionSwapCommand {
    /// The design section id.
    pub section: String,
    /// The catalogue part to put in its place.
    pub prototype: String,
}

/// The swap list container, hidden while the refit gate does not hold or
/// the selection has nothing to swap for.
#[derive(Component)]
pub(crate) struct ShipSwapList;

/// The swap list's header text, naming the selected section.
#[derive(Component)]
pub(crate) struct ShipSwapHeader;

/// The rows inside [`ShipSwapList`], and the row texts they were built from,
/// so the rows respawn only when one changes.
#[derive(Component, Default)]
pub(crate) struct ShipSwapRows(Vec<SwapRow>);

/// One offered part as the list shows it.
#[derive(Clone, Debug, PartialEq)]
struct SwapRow {
    section: String,
    prototype: String,
    name: String,
    line: String,
}

/// The lower-case noun for a section kind, for the refusal lines.
fn class_noun(class: SectionClass) -> &'static str {
    match class {
        SectionClass::Hull => "hull",
        SectionClass::Thruster => "thruster",
        SectionClass::Controller => "controller",
        SectionClass::Turret => "turret",
        SectionClass::Torpedo => "torpedo bay",
        SectionClass::Railgun => "railgun",
        SectionClass::Docking => "docking port",
        SectionClass::CargoIntake => "cargo intake",
        SectionClass::Mining => "mining emitter",
    }
}

/// The note line for a refused swap of section `code` for the part named
/// `name`; the list and the handler share it.
fn swap_refusal_text(
    items: &GameItems,
    code: &str,
    name: &str,
    refusal: &SectionSwapRefusalType,
) -> String {
    match refusal {
        SectionSwapRefusalType::NotDocked => "swap: dock at a station that refits".to_string(),
        SectionSwapRefusalType::NoRefitService => "swap: this dock offers no refit".to_string(),
        SectionSwapRefusalType::Family(SectionFamilyFaultType::OtherKind { have, offered }) => {
            format!(
                "swap: {code} is a {}, {name} is a {}",
                class_noun(*have),
                class_noun(*offered)
            )
        }
        SectionSwapRefusalType::Family(SectionFamilyFaultType::OtherFootprint {
            have,
            offered,
        }) => format!(
            "swap: {code} fills {}x{}x{} cells, {name} fills {}x{}x{}",
            have.x, have.y, have.z, offered.x, offered.y, offered.z
        ),
        SectionSwapRefusalType::Family(SectionFamilyFaultType::SamePart) => {
            format!("swap: {code} already is a {name}")
        }
        SectionSwapRefusalType::Payment(refusal) => {
            format!("swap: {}", payment_refusal_text(items, name, refusal))
        }
        SectionSwapRefusalType::Build(fault) => match fault {
            SwapBuildFaultType::Disconnected => {
                format!("swap: {name} would split the ship at {code}")
            }
            SwapBuildFaultType::InvalidSockets => {
                format!("swap: {name} does not mate with the sections around {code}")
            }
            SwapBuildFaultType::BlockedExit => {
                format!("swap: {name} at {code} would block a firing or exhaust lane")
            }
            SwapBuildFaultType::Underpowered { thrust, needed } => {
                format!("swap: the drives would push {thrust:.1}, the ship needs {needed:.1}")
            }
            SwapBuildFaultType::Undercontrolled {
                controllers,
                needed,
            } => format!(
                "swap: the ship would carry {controllers} flight computers, it needs {needed}"
            ),
        },
    }
}

impl ResolvedSite<'_> {
    /// The design section the selected entity `selected` is, when it is a
    /// live section of this ship's design.
    fn selected_section(&self, selected: Option<Entity>) -> Option<&DesignSection> {
        let selected = selected?;
        let live = self.live.iter().find(|live| live.entity == selected)?;
        self.design.get(&live.id)
    }

    /// Plan swapping design section `section` for `offered` against the hold
    /// `own`, the balance `own_cr` and the docked partner's stock and balance.
    fn plan_swap(
        &self,
        items: &GameItems,
        section: &DesignSection,
        offered: &SectionConfig,
        own: &ShipInventory,
        own_cr: u32,
        partner: Option<(&ShipInventory, u32)>,
    ) -> Result<RecipePayment, SectionSwapRefusalType> {
        let partner = self
            .partner
            .zip(partner)
            .map(|((_, offers), (stock, credits_cr))| StationPartner {
                offers,
                stock,
                credits_cr,
            });
        plan_section_swap(items, &section.config, offered, partner, own, own_cr)
    }

    /// The post-build fault swapping design section `section` for `offered`
    /// adds to the ship its live sections make, at their design mounts.
    fn swap_fault(
        &self,
        section: &DesignSection,
        offered: &SectionConfig,
    ) -> Option<SwapBuildFaultType> {
        let placed: Vec<SwapPlacedSection<'_>> = self
            .live
            .iter()
            .map(|live| {
                let design = self.design.get(&live.id).unwrap_or_else(|| {
                    panic!(
                        "swap: live section '{}' is not in the ship's design",
                        live.id
                    )
                });
                SwapPlacedSection {
                    position: design.position,
                    rotation: design.rotation,
                    config: &design.config,
                }
            })
            .collect();
        let index = self
            .live
            .iter()
            .position(|live| live.id == section.id)
            .expect("a swapped section is live");
        swap_build_fault(&placed, index, offered)
    }
}

/// Build the swap list under the lost list: a header and the rows
/// [`update_ship_swap_list`] fills.
pub(crate) fn spawn_ship_swap_list(panel: &mut ChildSpawnerCommands) {
    panel
        .spawn((
            ShipSwapList,
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
            list.spawn((ShipSwapHeader, themed_label("Swap", 13.0, UiColor::Accent)));
            list.spawn((
                ShipSwapRows::default(),
                Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(6.0),
                    ..default()
                },
            ));
        });
}

/// Show the swap list while the refit gate holds and the selected section
/// has a part to swap for, and respawn its rows when an offer, a price or a
/// refusal changes.
#[expect(
    clippy::too_many_arguments,
    reason = "one system reading the selection, the catalogue, both holds and the list"
)]
pub(crate) fn update_ship_swap_list(
    mut commands: Commands,
    runtime: Res<ShipRuntime>,
    items: Option<Res<GameItems>>,
    sections: Option<Res<GameSections>>,
    site: StationSite,
    q_holds: Query<(&ShipInventory, &ShipCredits)>,
    mut q_list: Query<&mut Node, With<ShipSwapList>>,
    mut q_header: Query<&mut Text, With<ShipSwapHeader>>,
    mut q_rows: Query<(Entity, &mut ShipSwapRows)>,
) {
    let Ok(mut list) = q_list.single_mut() else {
        return;
    };
    let Ok((rows_entity, mut shown)) = q_rows.single_mut() else {
        return;
    };
    let resolved = if site.gate(StationServiceType::Refit) {
        site.resolve(StationServiceType::Refit)
    } else {
        None
    };
    let mut header = None;
    let rows = match (resolved, items, sections) {
        (Some(resolved), Some(items), Some(sections)) => {
            match resolved.selected_section(runtime.selected) {
                Some(section) => {
                    let code = design_section_code(resolved.design, &section.id)
                        .expect("a live design section has a code");
                    let (own, own_cr) = q_holds
                        .get(resolved.player)
                        .expect("the player ship carries a ShipInventory and ShipCredits");
                    let partner = resolved
                        .partner
                        .and_then(|(partner, _)| q_holds.get(partner).ok())
                        .map(|(stock, credits)| (stock, credits.0));
                    let rows: Vec<SwapRow> = swap_offers(&sections, &section.config)
                        .map(|offered| {
                            let name = offered.base.name.clone();
                            let line = match resolved
                                .plan_swap(&items, section, offered, own, own_cr.0, partner)
                            {
                                Ok(payment) => rebuild_price_text(&payment),
                                Err(refusal) => swap_refusal_text(&items, &code, &name, &refusal),
                            };
                            SwapRow {
                                section: section.id.clone(),
                                prototype: offered.base.id.clone(),
                                name,
                                line,
                            }
                        })
                        .collect();
                    header = Some(format!("Swap {code}: {}", section.config.base.name));
                    rows
                }
                None => Vec::new(),
            }
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
    if let (Some(header), Ok(mut text)) = (header, q_header.single_mut()) {
        if text.0 != header {
            text.0 = header;
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
                    cell.spawn(themed_label(&row.name, 13.0, UiColor::Primary));
                    cell.spawn(themed_label(&row.line, 12.0, UiColor::Body));
                    cell.spawn((
                        Name::new(format!("ShipSwap-{}", row.prototype)),
                        compact_button(ButtonSpec::new("Swap").fit().primary()),
                    ))
                    .observe(on_ship_swap_button(
                        row.section.clone(),
                        row.prototype.clone(),
                    ));
                });
        }
    });
    shown.0 = rows;
}

/// Raise a [`SectionSwapCommand`] swapping design section `section` for
/// `prototype` when its Swap button is clicked.
fn on_ship_swap_button(
    section: String,
    prototype: String,
) -> impl FnMut(On<Activate>, MessageWriter<SectionSwapCommand>, Option<Res<SoundBank<UiSfx>>>, Commands)
{
    move |_activate, mut swaps, bank, mut commands| {
        swaps.write(SectionSwapCommand {
            section: section.clone(),
            prototype: prototype.clone(),
        });
        play_menu_select(&mut commands, bank.as_deref());
    }
}

/// Apply [`SectionSwapCommand`]s by the [`plan_section_swap`] rule and the
/// [`swap_build_fault`] check: take the hold's parts, buy the rest from the
/// station at ask, pay the price to the station, queue one `Swapped` report,
/// and raise the [`SectionSwapRequest`] that replaces the section at full
/// health. The old part is not refunded. A refusal changes nothing and shows
/// its reason on the note line.
#[expect(
    clippy::too_many_arguments,
    reason = "one system reading the catalogue, the site and both holds"
)]
pub(crate) fn apply_ship_swap_commands(
    mut messages: MessageReader<SectionSwapCommand>,
    mut runtime: ResMut<ShipRuntime>,
    mut commands: Commands,
    items: Option<Res<GameItems>>,
    sections: Option<Res<GameSections>>,
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
    let (Some(resolved), Some(items), Some(sections)) =
        (site.resolve(StationServiceType::Refit), items, sections)
    else {
        messages.clear();
        return;
    };
    for command in messages.read() {
        let section = resolved.design.get(&command.section).unwrap_or_else(|| {
            panic!(
                "swap: the player ship's design has no section '{}'",
                command.section
            )
        });
        let offered = sections.get_section(&command.prototype).unwrap_or_else(|| {
            panic!(
                "swap: the section catalogue has no part '{}'",
                command.prototype
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
                .and_then(|(partner, _)| q_holds.get(partner).ok())
                .map(|(stock, credits, _)| (stock, credits.0));
            resolved
                .plan_swap(&items, section, offered, own, own_cr.0, partner)
                .and_then(|payment| match resolved.swap_fault(section, offered) {
                    Some(fault) => Err(SectionSwapRefusalType::Build(fault)),
                    None => Ok(payment),
                })
        };
        let payment = match planned {
            Ok(payment) => payment,
            Err(refusal) => {
                runtime.note = Some((
                    swap_refusal_text(&items, &code, &offered.base.name, &refusal),
                    2.5,
                ));
                continue;
            }
        };
        let (station, _) = resolved
            .partner
            .expect("a planned swap has a docked partner");
        let Ok(
            [(mut own, mut own_cr, mut own_reports), (mut stock, mut station_cr, mut station_reports)],
        ) = q_holds.get_many_mut([resolved.player, station])
        else {
            panic!("swap: the player ship and station {station:?} each carry a hold");
        };
        for (item, &count) in &payment.from_hold {
            own.remove(item, count);
        }
        for (item, &count) in &payment.bought {
            stock.remove(item, count);
        }
        let paid_cr = own_cr.0 - payment.price_cr;
        let earned_cr = station_cr.0 + payment.price_cr;
        own_reports.change_credits(&mut own_cr, paid_cr, CreditChangeCauseType::Swap);
        station_reports.change_credits(&mut station_cr, earned_cr, CreditChangeCauseType::Swap);
        let parts: BTreeMap<ItemDesignId, u32> = offered.base.build.parts.clone();
        own_reports.0.push(ShipInventoryReport::Swapped {
            section: code.clone(),
            from: section.config.base.id.clone(),
            to: offered.base.id.clone(),
            parts,
            price_cr: payment.price_cr,
        });
        commands.trigger(SectionSwapRequest {
            entity: resolved.player,
            section: section.id.clone(),
            prototype: offered.base.id.clone(),
        });
        runtime.reselect = Some(section.id.clone());
        runtime.note = Some((
            format!(
                "swapped {code} for {}: {}",
                offered.base.name,
                rebuild_price_text(&payment)
            ),
            2.5,
        ));
    }
}

/// Select the section a swap replaced once its new entity is live, so the
/// panel stays on it.
pub(crate) fn reselect_swapped_section(
    mut runtime: ResMut<ShipRuntime>,
    q_player: Query<&Children, (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>)>,
    q_sections: Query<&EntityId, With<SectionMarker>>,
) {
    let Some(id) = runtime.reselect.clone() else {
        return;
    };
    let found = q_player.single().ok().and_then(|children| {
        children.iter().find(|child| {
            q_sections
                .get(*child)
                .is_ok_and(|entity_id| entity_id.0 == id)
        })
    });
    if let Some(entity) = found {
        runtime.selected = Some(entity);
        runtime.reselect = None;
    }
}
