//! The Ship pane's swap and add lists and the Swap and Add actions.
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
//! Under it, the add list shows the selected section's free sockets
//! ([`free_link_points`]) as point buttons, the first one chosen, and every
//! catalogue part that fits at the chosen point ([`section_add_fit`]), each
//! with the price or the reason it cannot be added, and an Add button. A
//! click raises a [`SectionAddCommand`]; [`apply_ship_add_commands`] plans it
//! with [`plan_section_add`], takes the parts and the credits, and raises the
//! [`SectionAddRequest`] the scenario crate adds the section from.
//!
//! Touch this module when changing what the swap or add list offers or what
//! Swap or Add takes.

use std::{
    collections::BTreeMap,
    hash::{DefaultHasher, Hash, Hasher},
};

use bevy::{prelude::*, ui_widgets::Activate};
use nova_events::prelude::{CreditChangeCauseType, EntityId};
use nova_gameplay::prelude::*;
use nova_input::prelude::InputSource;
use nova_ship::prelude::*;
use nova_ui::{theme::UiColor, widget::ButtonSpec};

use super::{
    rebuild::{payment_refusal_text, rebuild_price_text, ResolvedSite, StationSite},
    scene::ShipRuntime,
    sections::{code_prefix, design_section_code},
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
        SectionSwapRefusalType::Build(fault) => build_fault_text("swap", name, code, fault),
    }
}

/// The note line for a refit that would leave the ship with the post-build
/// `fault`, after the action's prefix: part `name` at section `code`.
fn build_fault_text(action: &str, name: &str, code: &str, fault: &RefitBuildFaultType) -> String {
    match fault {
        RefitBuildFaultType::Disconnected => {
            format!("{action}: {name} would split the ship at {code}")
        }
        RefitBuildFaultType::InvalidSockets => {
            format!("{action}: {name} does not mate with the sections around {code}")
        }
        RefitBuildFaultType::BlockedExit => {
            format!("{action}: {name} at {code} would block a firing or exhaust lane")
        }
        RefitBuildFaultType::Underpowered { thrust, needed } => {
            format!("{action}: the drives would push {thrust:.1}, the ship needs {needed:.1}")
        }
        RefitBuildFaultType::Undercontrolled {
            controllers,
            needed,
        } => format!(
            "{action}: the ship would carry {controllers} flight computers, it needs {needed}"
        ),
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
    ) -> Option<RefitBuildFaultType> {
        let placed: Vec<RefitPlacedSection<'_>> = self
            .live
            .iter()
            .map(|live| {
                let design = self.design.get(&live.id).unwrap_or_else(|| {
                    panic!(
                        "swap: live section '{}' is not in the ship's design",
                        live.id
                    )
                });
                RefitPlacedSection {
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

/// Select the section a swap replaced or an add made once its new entity is
/// live, so the panel shows it.
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

/// A request to add a catalogue part at a free socket of a live section of
/// the player ship, raised by an add-list Add button.
#[derive(Message, Clone, Debug)]
pub struct SectionAddCommand {
    /// The design section id the part mounts on.
    pub section: String,
    /// The socket id of that section the part mates.
    pub point: String,
    /// The catalogue part to add.
    pub prototype: String,
}

/// The add list container, hidden while the refit gate does not hold or the
/// selection has no free socket.
#[derive(Component)]
pub(crate) struct ShipAddList;

/// The add list's header text, naming the selected section.
#[derive(Component)]
pub(crate) struct ShipAddHeader;

/// The point buttons inside [`ShipAddList`], and the points they were built
/// from, so the buttons respawn only when one changes.
#[derive(Component, Default)]
pub(crate) struct ShipAddPoints(Vec<AddPoint>);

/// The part rows inside [`ShipAddList`], and the row texts they were built
/// from, so the rows respawn only when one changes.
#[derive(Component, Default)]
pub(crate) struct ShipAddRows(Vec<AddRow>);

/// One free socket as a point button shows it.
#[derive(Clone, Debug, PartialEq)]
struct AddPoint {
    entity: Entity,
    socket: String,
    chosen: bool,
}

/// One fitting part as the add list shows it, or the line saying nothing
/// fits when `prototype` is empty.
#[derive(Clone, Debug, PartialEq)]
struct AddRow {
    section: String,
    point: String,
    prototype: String,
    name: String,
    line: String,
}

/// The fits of every catalogue part at one point, kept while the ship and
/// the point stay the same: a fit reads the socket graph and the exit lanes
/// of the whole ship, too much to redo for every part every frame.
#[derive(Default)]
pub(crate) struct AddFitCache {
    key: Option<u64>,
    fits: Vec<(String, Result<(Vec3, Quat), SectionAddFaultType>)>,
}

/// The player binding components a live section may carry, beside its id
/// and class.
pub(crate) type SectionBindingRow = (
    &'static EntityId,
    &'static SectionClass,
    Option<&'static SpaceshipThrusterInputBinding>,
    Option<&'static SpaceshipTurretInputBinding>,
    Option<&'static SpaceshipTorpedoInputBinding>,
    Option<&'static SpaceshipRailgunInputBinding>,
    Option<&'static SpaceshipMiningInputBinding>,
);

/// The bindings the live sections of `site` carry, as
/// [`added_section_binds`] reads them.
pub(crate) fn live_bindings(
    site: &ResolvedSite<'_>,
    q_bindings: &Query<SectionBindingRow, With<SectionMarker>>,
) -> Vec<(String, SectionClass, Vec<InputSource>)> {
    site.live
        .iter()
        .filter_map(|live| q_bindings.get(live.entity).ok())
        .filter_map(|(id, class, thruster, turret, torpedo, railgun, mining)| {
            let sources = thruster
                .map(|binding| &binding.0)
                .or_else(|| turret.map(|binding| &binding.0))
                .or_else(|| torpedo.map(|binding| &binding.0))
                .or_else(|| railgun.map(|binding| &binding.0))
                .or_else(|| mining.map(|binding| &binding.0))?;
            Some((id.0.clone(), *class, sources.clone()))
        })
        .collect()
}

/// The pane code a section of `class` added to `design` takes: its id sorts
/// after every id of its class ([`added_section_id`]), so it is the next one.
fn added_section_code(design: &ShipDesignSections, class: SectionClass) -> String {
    let before = design
        .0
        .iter()
        .filter(|section| section.config.kind.class() == class)
        .count();
    format!("{}-{}", code_prefix(class), before + 1)
}

/// The note line for a refused add of the part named `name` at socket
/// `point` of section `code`; the list and the handler share it.
fn add_refusal_text(
    items: &GameItems,
    code: &str,
    point: &str,
    name: &str,
    refusal: &SectionAddRefusalType,
) -> String {
    match refusal {
        SectionAddRefusalType::NotDocked => "add: dock at a station that refits".to_string(),
        SectionAddRefusalType::NoRefitService => "add: this dock offers no refit".to_string(),
        SectionAddRefusalType::Fit(SectionAddFaultType::Occupied) => {
            format!("add: {code} {point} is taken")
        }
        SectionAddRefusalType::Fit(SectionAddFaultType::NoSockets) => {
            format!("add: {name} has no socket to mount by")
        }
        SectionAddRefusalType::Fit(SectionAddFaultType::Overlap) => {
            format!("add: {name} at {code} {point} would overlap the hull")
        }
        SectionAddRefusalType::Fit(SectionAddFaultType::Build(fault)) => {
            build_fault_text("add", name, code, fault)
        }
        SectionAddRefusalType::NoFreeKey => {
            format!("add: every mining key is taken, {name} would have none")
        }
        SectionAddRefusalType::Payment(refusal) => {
            format!("add: {}", payment_refusal_text(items, name, refusal))
        }
    }
}

impl ResolvedSite<'_> {
    /// Every section of the design placed at its mount, and the indices of
    /// the live ones, as the add rule reads them.
    fn refit_design(&self) -> (Vec<RefitPlacedSection<'_>>, Vec<usize>) {
        let placed = self
            .design
            .0
            .iter()
            .map(|section| RefitPlacedSection {
                position: section.position,
                rotation: section.rotation,
                config: &section.config,
            })
            .collect();
        let live = self
            .design
            .0
            .iter()
            .enumerate()
            .filter(|(_, section)| self.live.iter().any(|live| live.id == section.id))
            .map(|(index, _)| index)
            .collect();
        (placed, live)
    }

    /// The design index of design section `section`.
    fn design_index(&self, section: &DesignSection) -> usize {
        self.design
            .0
            .iter()
            .position(|other| other.id == section.id)
            .expect("a design section is in its design")
    }

    /// A key for the ship's design, its live sections and one point, so a
    /// cache of fits knows when to redo them.
    fn fit_key(&self, section: &DesignSection, point: usize) -> u64 {
        let mut hasher = DefaultHasher::new();
        section.id.hash(&mut hasher);
        point.hash(&mut hasher);
        for design in &self.design.0 {
            design.id.hash(&mut hasher);
            design.config.base.id.hash(&mut hasher);
        }
        for live in &self.live {
            live.id.hash(&mut hasher);
        }
        hasher.finish()
    }

    /// Plan adding `offered` with the fit `fit` against the hold `own`, the
    /// balance `own_cr`, the live bindings `bound` and the docked partner's
    /// stock and balance.
    #[expect(
        clippy::too_many_arguments,
        reason = "the part, its fit, the bindings, both holds and both balances"
    )]
    fn plan_add(
        &self,
        items: &GameItems,
        offered: &SectionConfig,
        fit: Result<(Vec3, Quat), SectionAddFaultType>,
        bound: &[(String, SectionClass, Vec<InputSource>)],
        own: &ShipInventory,
        own_cr: u32,
        partner: Option<(&ShipInventory, u32)>,
    ) -> Result<SectionAddPlan, SectionAddRefusalType> {
        let partner = self
            .partner
            .zip(partner)
            .map(|((_, offers), (stock, credits_cr))| StationPartner {
                offers,
                stock,
                credits_cr,
            });
        plan_section_add(
            items,
            offered,
            fit,
            bound
                .iter()
                .map(|(id, class, sources)| (id.as_str(), *class, sources.as_slice())),
            partner,
            own,
            own_cr,
        )
    }
}

/// Build the add list under the swap list: a header, the point buttons and
/// the rows [`update_ship_add_list`] fills.
pub(crate) fn spawn_ship_add_list(panel: &mut ChildSpawnerCommands) {
    panel
        .spawn((
            ShipAddList,
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
            list.spawn((ShipAddHeader, themed_label("Add", 13.0, UiColor::Accent)));
            list.spawn((
                ShipAddPoints::default(),
                Node {
                    flex_direction: FlexDirection::Row,
                    flex_wrap: FlexWrap::Wrap,
                    column_gap: Val::Px(6.0),
                    row_gap: Val::Px(6.0),
                    ..default()
                },
            ));
            list.spawn((
                ShipAddRows::default(),
                Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(6.0),
                    ..default()
                },
            ));
        });
}

/// Show the add list while the refit gate holds and the selected section has
/// a free socket, and respawn its point buttons and rows when a point, an
/// offer, a price or a refusal changes.
#[expect(
    clippy::too_many_arguments,
    reason = "one system reading the selection, the catalogue, both holds, the bindings and the list"
)]
#[expect(
    clippy::type_complexity,
    reason = "the list's three parts are each one filtered query"
)]
pub(crate) fn update_ship_add_list(
    mut commands: Commands,
    mut cache: Local<AddFitCache>,
    runtime: Res<ShipRuntime>,
    items: Option<Res<GameItems>>,
    sections: Option<Res<GameSections>>,
    site: StationSite,
    q_holds: Query<(&ShipInventory, &ShipCredits)>,
    q_bindings: Query<SectionBindingRow, With<SectionMarker>>,
    mut q_list: Query<&mut Node, With<ShipAddList>>,
    mut q_header: Query<&mut Text, With<ShipAddHeader>>,
    mut q_parts: ParamSet<(
        Query<(Entity, &mut ShipAddPoints)>,
        Query<(Entity, &mut ShipAddRows)>,
    )>,
) {
    let Ok(mut list) = q_list.single_mut() else {
        return;
    };
    let resolved = if site.gate(StationServiceType::Refit) {
        site.resolve(StationServiceType::Refit)
    } else {
        None
    };
    let mut header = None;
    let mut points = Vec::new();
    let mut rows = Vec::new();
    if let (Some(resolved), Some(items), Some(sections), Some(selected)) =
        (resolved, items, sections, runtime.selected)
    {
        if let Some(section) = resolved.selected_section(Some(selected)) {
            let (placed, live) = resolved.refit_design();
            let index = resolved.design_index(section);
            let free = free_link_points(&placed, index);
            let sockets = &section.config.base.link_points;
            let chosen = runtime
                .add_point
                .as_ref()
                .filter(|(entity, _)| *entity == selected)
                .and_then(|(_, socket)| {
                    free.iter()
                        .copied()
                        .find(|&point| sockets[point].id == *socket)
                })
                .or_else(|| free.first().copied());
            if let Some(chosen) = chosen {
                let code = design_section_code(resolved.design, &section.id)
                    .expect("a live design section has a code");
                points = free
                    .iter()
                    .map(|&point| AddPoint {
                        entity: selected,
                        socket: sockets[point].id.clone(),
                        chosen: point == chosen,
                    })
                    .collect();
                let key = resolved.fit_key(section, chosen);
                if cache.key != Some(key) {
                    cache.fits = sections
                        .iter()
                        .filter(|offered| !offered.base.hide_in_editor)
                        .map(|offered| {
                            let fit = section_add_fit(&placed, &live, index, chosen, offered);
                            (offered.base.id.clone(), fit)
                        })
                        .collect();
                    cache.key = Some(key);
                }
                let (own, own_cr) = q_holds
                    .get(resolved.player)
                    .expect("the player ship carries a ShipInventory and ShipCredits");
                let partner = resolved
                    .partner
                    .and_then(|(partner, _)| q_holds.get(partner).ok())
                    .map(|(stock, credits)| (stock, credits.0));
                let bound = live_bindings(&resolved, &q_bindings);
                let point = sockets[chosen].id.clone();
                rows = cache
                    .fits
                    .iter()
                    .filter(|(_, fit)| fit.is_ok())
                    .filter_map(|(prototype, fit)| {
                        let offered = sections.get_section(prototype)?;
                        let name = offered.base.name.clone();
                        let line = match resolved.plan_add(
                            &items,
                            offered,
                            fit.clone(),
                            &bound,
                            own,
                            own_cr.0,
                            partner,
                        ) {
                            Ok(plan) => rebuild_price_text(&plan.payment),
                            Err(refusal) => {
                                add_refusal_text(&items, &code, &point, &name, &refusal)
                            }
                        };
                        Some(AddRow {
                            section: section.id.clone(),
                            point: point.clone(),
                            prototype: prototype.clone(),
                            name,
                            line,
                        })
                    })
                    .collect();
                if rows.is_empty() {
                    rows.push(AddRow {
                        section: section.id.clone(),
                        point: point.clone(),
                        prototype: String::new(),
                        name: String::new(),
                        line: format!("No part fits at {code} {point}"),
                    });
                }
                header = Some(format!("Add at {code}: {}", section.config.base.name));
            }
        }
    }
    let display = if points.is_empty() {
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
    if let Ok((points_entity, mut shown)) = q_parts.p0().single_mut() {
        if shown.0 != points {
            commands.entity(points_entity).despawn_children();
            commands.entity(points_entity).with_children(|parent| {
                for point in &points {
                    let spec = ButtonSpec::new(&point.socket).fit();
                    let spec = if point.chosen { spec.primary() } else { spec };
                    parent
                        .spawn((
                            Name::new(format!("ShipAddPoint-{}", point.socket)),
                            compact_button(spec),
                        ))
                        .observe(on_ship_add_point_button(point.entity, point.socket.clone()));
                }
            });
            shown.0 = points;
        }
    }
    if let Ok((rows_entity, mut shown)) = q_parts.p1().single_mut() {
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
                        if row.prototype.is_empty() {
                            cell.spawn(themed_label(&row.line, 12.0, UiColor::Body));
                            return;
                        }
                        cell.spawn(themed_label(&row.name, 13.0, UiColor::Primary));
                        cell.spawn(themed_label(&row.line, 12.0, UiColor::Body));
                        cell.spawn((
                            Name::new(format!("ShipAdd-{}", row.prototype)),
                            compact_button(ButtonSpec::new("Add").fit().primary()),
                        ))
                        .observe(on_ship_add_button(
                            row.section.clone(),
                            row.point.clone(),
                            row.prototype.clone(),
                        ));
                    });
            }
        });
        shown.0 = rows;
    }
}

/// Choose socket `socket` of section `entity` as the add point when its
/// point button is clicked.
fn on_ship_add_point_button(
    entity: Entity,
    socket: String,
) -> impl FnMut(On<Activate>, ResMut<ShipRuntime>, Option<Res<SoundBank<UiSfx>>>, Commands) {
    move |_activate, mut runtime, bank, mut commands| {
        runtime.add_point = Some((entity, socket.clone()));
        play_menu_select(&mut commands, bank.as_deref());
    }
}

/// Raise a [`SectionAddCommand`] adding `prototype` at socket `point` of
/// design section `section` when its Add button is clicked.
fn on_ship_add_button(
    section: String,
    point: String,
    prototype: String,
) -> impl FnMut(On<Activate>, MessageWriter<SectionAddCommand>, Option<Res<SoundBank<UiSfx>>>, Commands)
{
    move |_activate, mut adds, bank, mut commands| {
        adds.write(SectionAddCommand {
            section: section.clone(),
            point: point.clone(),
            prototype: prototype.clone(),
        });
        play_menu_select(&mut commands, bank.as_deref());
    }
}

/// Apply [`SectionAddCommand`]s by the [`section_add_fit`] and
/// [`plan_section_add`] rules: take the hold's parts, buy the rest from the
/// station at ask, pay the price to the station, queue one `Added` report,
/// and raise the [`SectionAddRequest`] that mounts the part at full health
/// under [`added_section_id`]. A refusal changes nothing and shows its reason
/// on the note line.
#[expect(
    clippy::too_many_arguments,
    reason = "one system reading the catalogue, the site, the bindings and both holds"
)]
pub(crate) fn apply_ship_add_commands(
    mut messages: MessageReader<SectionAddCommand>,
    mut runtime: ResMut<ShipRuntime>,
    mut commands: Commands,
    items: Option<Res<GameItems>>,
    sections: Option<Res<GameSections>>,
    site: StationSite,
    q_bindings: Query<SectionBindingRow, With<SectionMarker>>,
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
                "add: the player ship's design has no section '{}'",
                command.section
            )
        });
        let point = section
            .config
            .base
            .link_points
            .iter()
            .position(|socket| socket.id == command.point)
            .unwrap_or_else(|| {
                panic!(
                    "add: section '{}' has no socket '{}'",
                    command.section, command.point
                )
            });
        let offered = sections.get_section(&command.prototype).unwrap_or_else(|| {
            panic!(
                "add: the section catalogue has no part '{}'",
                command.prototype
            )
        });
        let code =
            design_section_code(resolved.design, &section.id).expect("a design section has a code");
        let planned = {
            let (placed, live) = resolved.refit_design();
            let fit = section_add_fit(
                &placed,
                &live,
                resolved.design_index(section),
                point,
                offered,
            );
            let bound = live_bindings(&resolved, &q_bindings);
            let (own, own_cr, _) = q_holds
                .get(resolved.player)
                .expect("the player ship carries a ShipInventory and ShipCredits");
            let partner = resolved
                .partner
                .and_then(|(partner, _)| q_holds.get(partner).ok())
                .map(|(stock, credits, _)| (stock, credits.0));
            resolved.plan_add(&items, offered, fit, &bound, own, own_cr.0, partner)
        };
        let plan = match planned {
            Ok(plan) => plan,
            Err(refusal) => {
                runtime.note = Some((
                    add_refusal_text(&items, &code, &command.point, &offered.base.name, &refusal),
                    2.5,
                ));
                continue;
            }
        };
        let (station, _) = resolved
            .partner
            .expect("a planned add has a docked partner");
        let Ok(
            [(mut own, mut own_cr, mut own_reports), (mut stock, mut station_cr, mut station_reports)],
        ) = q_holds.get_many_mut([resolved.player, station])
        else {
            panic!("add: the player ship and station {station:?} each carry a hold");
        };
        let payment = &plan.payment;
        for (item, &count) in &payment.from_hold {
            own.remove(item, count);
        }
        for (item, &count) in &payment.bought {
            stock.remove(item, count);
        }
        let paid_cr = own_cr.0 - payment.price_cr;
        let earned_cr = station_cr.0 + payment.price_cr;
        own_reports.change_credits(&mut own_cr, paid_cr, CreditChangeCauseType::Add);
        station_reports.change_credits(&mut station_cr, earned_cr, CreditChangeCauseType::Add);
        let class = offered.kind.class();
        let id = added_section_id(resolved.design, class);
        let new_code = added_section_code(resolved.design, class);
        own_reports.0.push(ShipInventoryReport::Added {
            section: new_code.clone(),
            part: offered.base.id.clone(),
            at: code.clone(),
            point: command.point.clone(),
            parts: offered.base.build.parts.clone(),
            price_cr: payment.price_cr,
        });
        commands.trigger(SectionAddRequest {
            entity: resolved.player,
            section: id.clone(),
            prototype: offered.base.id.clone(),
            position: plan.position,
            rotation: plan.rotation,
            bindings: plan.bindings.clone(),
        });
        runtime.reselect = Some(id);
        runtime.note = Some((
            format!(
                "added {} as {new_code} at {code} {}: {}",
                offered.base.name,
                command.point,
                rebuild_price_text(payment)
            ),
            2.5,
        ));
    }
}
