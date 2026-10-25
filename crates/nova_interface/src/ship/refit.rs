//! The Ship pane's Swap and Add actions and the rules they share.
//!
//! The station catalogue (`super::catalogue`) raises a [`SectionSwapCommand`]
//! from a row's Swap button; [`apply_ship_swap_commands`] plans it with
//! [`plan_section_swap`], checks the ship it leaves with [`swap_build_fault`],
//! takes the parts and the credits, and raises the [`SectionSwapRequest`] the
//! scenario crate replaces the section from.
//!
//! The placement ghost (`super::ghost`) raises a [`SectionAddCommand`] when a
//! click lands on a valid ghost; [`apply_ship_add_commands`] fits it with
//! [`section_add_fit`], plans it with [`plan_section_add`], takes the parts
//! and the credits, and raises the [`SectionAddRequest`] the scenario crate
//! adds the section from.
//!
//! Touch this module when changing what Swap or Add takes or how a refusal
//! reads.

use std::{
    collections::BTreeMap,
    hash::{DefaultHasher, Hash, Hasher},
};

use bevy::prelude::*;
use nova_events::prelude::{CreditChangeCauseType, EntityId};
use nova_gameplay::prelude::*;
use nova_input::prelude::InputSource;
use nova_ship::prelude::*;

use super::{
    rebuild::{payment_refusal_text, rebuild_price_text, ResolvedSite, StationSite},
    scene::ShipRuntime,
    sections::{code_prefix, design_section_code},
};

/// A request to swap one live section of the player ship for another
/// catalogue part, raised by the catalogue's Swap verb.
#[derive(Message, Clone, Debug)]
pub struct SectionSwapCommand {
    /// The design section id.
    pub section: String,
    /// The catalogue part to put in its place.
    pub prototype: String,
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
pub(crate) fn swap_refusal_text(
    items: &GameItems,
    code: &str,
    name: &str,
    refusal: &SectionSwapRefusalType,
) -> String {
    match refusal {
        SectionSwapRefusalType::NotDocked => "swap: dock at a station that refits".to_string(),
        SectionSwapRefusalType::NoRefitService => "swap: this dock offers no refit".to_string(),
        SectionSwapRefusalType::NotBuilt => format!("swap: this station does not build {name}"),
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
pub(crate) fn build_fault_text(
    action: &str,
    name: &str,
    code: &str,
    fault: &RefitBuildFaultType,
) -> String {
    match fault {
        RefitBuildFaultType::Disconnected { .. } => {
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
        RefitBuildFaultType::NoController => {
            format!("{action}: {name} would leave the ship without a flight computer")
        }
        RefitBuildFaultType::LastWeapon => {
            format!("{action}: {name} would leave the ship without a weapon")
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
    pub(crate) fn selected_section(&self, selected: Option<Entity>) -> Option<&DesignSection> {
        let selected = selected?;
        let live = self.live.iter().find(|live| live.entity == selected)?;
        self.design.get(&live.id)
    }

    /// Plan swapping design section `section` for `offered` against the hold
    /// `own`, the balance `own_cr` and the docked partner's stock and balance.
    pub(crate) fn plan_swap(
        &self,
        items: &GameItems,
        section: &DesignSection,
        offered: &SectionConfig,
        own: &ShipInventory,
        own_cr: u32,
        partner: Option<(&ShipInventory, u32)>,
    ) -> Result<RecipePayment, SectionSwapRefusalType> {
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
        plan_section_swap(items, &section.config, offered, partner, own, own_cr)
    }

    /// The ship the live sections make, at their design mounts, in
    /// [`ResolvedSite::live`] order: what the refit checks read.
    ///
    /// # Panics
    ///
    /// On a live section the design lacks: every live section of an
    /// authored ship spawned from a design section.
    pub(crate) fn placed(&self) -> Vec<RefitPlacedSection<'_>> {
        self.live
            .iter()
            .map(|live| {
                let design = self.design.get(&live.id).unwrap_or_else(|| {
                    panic!(
                        "refit: live section '{}' is not in the ship's design",
                        live.id
                    )
                });
                RefitPlacedSection {
                    position: design.position,
                    rotation: design.rotation,
                    config: &design.config,
                    working: live.working,
                }
            })
            .collect()
    }

    /// The post-build fault swapping design section `section` for `offered`
    /// adds to the ship its live sections make, at their design mounts.
    pub(crate) fn swap_fault(
        &self,
        section: &DesignSection,
        offered: &SectionConfig,
    ) -> Option<RefitBuildFaultType> {
        let placed = self.placed();
        let index = self
            .live
            .iter()
            .position(|live| live.id == section.id)
            .expect("a swapped section is live");
        swap_build_fault(&placed, index, offered)
    }
}

/// Apply [`SectionSwapCommand`]s by the [`plan_section_swap`] rule and the
/// [`swap_build_fault`] check: take the hold's parts, buy the rest from the
/// station at ask, pay the price to the station, queue one `Swapped` report,
/// and raise the [`SectionSwapRequest`] that replaces the section at full
/// health. The old part is not refunded. A refusal changes nothing and shows
/// its reason on the note line.
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
    // One swap a frame: the site was read before it, and the section it
    // replaces keeps its old part until the commands run.
    let mut swapped = false;
    for command in messages.read() {
        if swapped {
            continue;
        }
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
                .and_then(|(partner, ..)| q_holds.get(partner).ok())
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
        let (station, ..) = resolved
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
        swapped = true;
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
/// the player ship, raised by a click on a valid placement ghost.
#[derive(Message, Clone, Debug)]
pub struct SectionAddCommand {
    /// The design section id the part mounts on.
    pub section: String,
    /// The socket id of that section the part mates.
    pub point: String,
    /// The catalogue part to add.
    pub prototype: String,
    /// How the part mates the socket.
    pub mount: MountPose,
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

/// The line for a refused add of the part named `name` on section `code`;
/// the ghost callout and the handler's note share it.
pub(crate) fn add_refusal_text(
    items: &GameItems,
    code: &str,
    name: &str,
    refusal: &SectionAddRefusalType,
) -> String {
    match refusal {
        SectionAddRefusalType::NotDocked => "add: dock at a station that refits".to_string(),
        SectionAddRefusalType::NoRefitService => "add: this dock offers no refit".to_string(),
        SectionAddRefusalType::NotBuilt => format!("add: this station does not build {name}"),
        SectionAddRefusalType::Fit(SectionAddFaultType::Occupied) => {
            format!("add: that point on {code} is taken")
        }
        SectionAddRefusalType::Fit(SectionAddFaultType::NoSockets) => {
            format!("add: {name} has no socket to mount by")
        }
        SectionAddRefusalType::Fit(SectionAddFaultType::Overlap) => {
            format!("add: {name} on {code} would overlap the hull")
        }
        SectionAddRefusalType::Fit(SectionAddFaultType::Obstructed) => {
            format!("add: {name} on {code} would hit the station or a ship docked there")
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
    /// the live ones, as the add rule reads them. A lost section is not
    /// working; only the live ones reach the post-build checks.
    pub(crate) fn refit_design(&self) -> (Vec<RefitPlacedSection<'_>>, Vec<usize>) {
        let placed = self
            .design
            .0
            .iter()
            .map(|section| RefitPlacedSection {
                position: section.position,
                rotation: section.rotation,
                config: &section.config,
                working: self
                    .live
                    .iter()
                    .any(|live| live.id == section.id && live.working),
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
    pub(crate) fn design_index(&self, section: &DesignSection) -> usize {
        self.design
            .0
            .iter()
            .position(|other| other.id == section.id)
            .expect("a design section is in its design")
    }

    /// A key for the ship's design, its live sections and one point, so a
    /// cache of fits knows when to redo them.
    pub(crate) fn fit_key(&self, section: &DesignSection, point: usize) -> u64 {
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
    pub(crate) fn plan_add(
        &self,
        items: &GameItems,
        offered: &SectionConfig,
        fit: Result<(Vec3, Quat), SectionAddFaultType>,
        bound: &[(String, SectionClass, Vec<InputSource>)],
        own: &ShipInventory,
        own_cr: u32,
        partner: Option<(&ShipInventory, u32)>,
    ) -> Result<SectionAddPlan, SectionAddRefusalType> {
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
    // One add a frame: the site was read before it, and the section it mounts
    // is not in the design until the commands run.
    let mut added = false;
    for command in messages.read() {
        if added {
            continue;
        }
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
                command.mount,
                &site.partner_colliders(&resolved),
            );
            let bound = live_bindings(&resolved, &q_bindings);
            let (own, own_cr, _) = q_holds
                .get(resolved.player)
                .expect("the player ship carries a ShipInventory and ShipCredits");
            let partner = resolved
                .partner
                .and_then(|(partner, ..)| q_holds.get(partner).ok())
                .map(|(stock, credits, _)| (stock, credits.0));
            resolved.plan_add(&items, offered, fit, &bound, own, own_cr.0, partner)
        };
        let plan = match planned {
            Ok(plan) => plan,
            Err(refusal) => {
                runtime.note = Some((
                    add_refusal_text(&items, &code, &offered.base.name, &refusal),
                    2.5,
                ));
                continue;
            }
        };
        let (station, ..) = resolved
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
                "added {} as {new_code} on {code}: {}",
                offered.base.name,
                rebuild_price_text(payment)
            ),
            2.5,
        ));
        added = true;
    }
}
