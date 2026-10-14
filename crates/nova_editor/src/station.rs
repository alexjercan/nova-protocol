//! A ship node marked as a STATION: the station design the sandbox writes for
//! it, and the faults that keep it from spawning as one.
//!
//! The builder authors only the hull and the mark. Everything else a
//! [`StationDesign`] states is a sandbox default, chosen to match what the
//! generated stations offer: the four dock services, the waystation's storage
//! cap and credits, and the waystation's role goods cut to the hull's hold.
//! The editor has no panel for those; a save rewrites them from these
//! defaults every time.
//!
//! The faults are [`lint_station_design`]'s own errors, so the editor refuses
//! exactly what the content lint refuses: no docking port, a thruster, no
//! controller, or a hull whose sections do not make one connected ship.

use bevy::{ecs::system::SystemParam, prelude::*};
use nova_gameplay::prelude::{GameItems, ItemDesignId, ItemRoleType, ShipInventoryStock};
use nova_scenario::prelude::{
    lint_station_design, resolve_ship_design, GameShipDesigns, KnownSections, LintSeverity,
    ShipDesign, ShipDesignSource, ShipIntegrityConfig, StationDesign, StationDesignId,
    StationServiceType, STATION_COLLAPSE_THRESHOLD,
};
use nova_ship::prelude::GameSections;

use crate::scenario::{ship_design, HullForm, LoweredFleet, LoweredShip, SANDBOX_ID};

/// The services a sandbox station offers: the four every generated station
/// docks a ship for.
const SERVICES: [StationServiceType; 4] = [
    StationServiceType::Rebuild,
    StationServiceType::Refit,
    StationServiceType::Trade,
    StationServiceType::Storage,
];

/// Grams a sandbox station stores for a docked ship: the base waystation's
/// cap, so a docked ship reads the same store here as at a shipped station.
const STORAGE_CAP_G: u32 = 100_000;

/// The credits a sandbox station trades with: the base waystation's.
const CREDITS: u32 = 5_000;

/// The goods a sandbox station opens with, before the hold cuts them: the
/// base waystation's role goods, in the order the hold is filled.
///
/// Roles rather than the waystation's section parts, because a role item is
/// one every item pack must carry, so the line resolves under any mod set.
const STOCK: [(ItemRoleType, u32); 3] = [
    (ItemRoleType::HullPlate, 8),
    (ItemRoleType::Rations, 20),
    (ItemRoleType::SalvagedParts, 10),
];

/// What a sandbox station says about itself in a picker.
const ABOUT: &str = "A station built in the sandbox editor.";

/// The source a station fault is filed under. The editor files nothing; the
/// lint wants a name for the report it would write.
const LINT_SOURCE: &str = "editor";

/// The id the station design of `ship` is spawned by.
///
/// A SAVE writes it under the ship node's own id, as it writes a ship
/// prototype. Play registers it into the live catalog for the session, so it
/// is namespaced under the sandbox's id there: a node id must not replace a
/// shipped station of the same name.
pub(crate) fn station_design_id(ship: &LoweredShip, form: HullForm) -> StationDesignId {
    match form {
        HullForm::Prototype => ship.id.as_str().into(),
        HullForm::Inline => format!("{SANDBOX_ID}_{}", ship.id).as_str().into(),
    }
}

/// The station design a marked ship spawns as, under the id `form` names.
///
/// `sections` resolves the hull to measure its hold; `items` weighs the
/// stock. A stock line whose item the catalog lacks is left out, and a line
/// is cut to the whole items that still fit, so the spawn never overfills the
/// hold the lint checks it against.
pub(crate) fn station_design(
    ship: &LoweredShip,
    form: HullForm,
    sections: &GameSections,
    items: &GameItems,
) -> StationDesign {
    let hull = ShipDesign {
        integrity: ShipIntegrityConfig {
            collapse_threshold: Some(STATION_COLLAPSE_THRESHOLD),
        },
        ..ship_design(ship)
    };
    let (resolved, _) = resolve_ship_design(
        &ShipDesignSource::Inline(hull.clone()),
        &GameShipDesigns::default(),
        sections,
    );
    StationDesign {
        id: station_design_id(ship, form),
        name: ship.station_name(),
        about: ABOUT.to_string(),
        hull,
        services: SERVICES.to_vec(),
        storage_cap_g: STORAGE_CAP_G,
        stock: stock_within(u64::from(resolved.cargo_capacity_g()), items),
        credits: CREDITS,
    }
}

/// [`STOCK`], cut to `capacity_g` in the order it is listed.
fn stock_within(capacity_g: u64, items: &GameItems) -> ShipInventoryStock {
    let mut free_g = capacity_g;
    let mut lines = Vec::new();
    for (role, wanted) in STOCK {
        let id = ItemDesignId::from(role);
        let Some(item) = items.get(&id) else {
            continue;
        };
        let fits = free_g / u64::from(item.mass_g.max(1));
        let count = u32::try_from(fits.min(u64::from(wanted))).unwrap_or(wanted);
        if count == 0 {
            continue;
        }
        free_g -= item.stack_mass_g(count);
        lines.push((id, count));
    }
    ShipInventoryStock::new(lines)
}

/// Why `design` cannot spawn as a station, one line per fault, or empty when
/// it can. The station lint's errors, with the `station '<id>': ` lead it
/// writes taken off: the reader already knows which station they are on.
pub(crate) fn station_faults(
    design: &StationDesign,
    sections: &GameSections,
    items: &GameItems,
) -> Vec<String> {
    let lead = format!("station '{}': ", design.id);
    lint_station_design(
        design,
        &KnownSections::from_configs(&sections.0),
        items,
        LINT_SOURCE,
    )
    .into_iter()
    .filter(|issue| issue.severity == LintSeverity::Error)
    .map(|issue| {
        issue
            .message
            .strip_prefix(&lead)
            .map_or_else(|| issue.message.clone(), str::to_string)
    })
    .collect()
}

/// A fault as the one-line Station Check row shows it: the clause before the
/// lint's explanation.
pub(crate) fn short_fault(fault: &str) -> &str {
    fault.split_once("; ").map_or(fault, |(clause, _)| clause)
}

/// The two catalogs a station design is built and checked against.
///
/// Optional because a rig that merged no content still runs the editor: with
/// no sections every catalog part is unknown and every station is refused,
/// which is the honest answer for a hull nothing can resolve.
#[derive(SystemParam)]
pub(crate) struct StationCatalogs<'w> {
    sections: Option<Res<'w, GameSections>>,
    items: Option<Res<'w, GameItems>>,
}

impl StationCatalogs<'_> {
    /// Hand `read` the section and item catalogs, or empty ones in a rig that
    /// merged none.
    fn with<R>(&self, read: impl FnOnce(&GameSections, &GameItems) -> R) -> R {
        let (no_sections, no_items) = (GameSections::default(), GameItems::default());
        read(
            self.sections.as_deref().unwrap_or(&no_sections),
            self.items.as_deref().unwrap_or(&no_items),
        )
    }

    /// Every station design `fleet` spawns, in the form asked for, each with
    /// its faults. Empty faults mean the design spawns as a station.
    pub(crate) fn designs(
        &self,
        fleet: &LoweredFleet,
        form: HullForm,
    ) -> Vec<(StationDesign, Vec<String>)> {
        self.with(|sections, items| {
            fleet
                .stations()
                .map(|ship| {
                    let design = station_design(ship, form, sections, items);
                    let faults = station_faults(&design, sections, items);
                    (design, faults)
                })
                .collect()
        })
    }

    /// Every marked ship of `fleet` that Play would spawn and that cannot
    /// spawn as a station, by name, with its faults. A marked ship with no
    /// sections is not here: Play skips a blank ship whatever it is marked.
    pub(crate) fn refused(&self, fleet: &LoweredFleet) -> Vec<(String, Vec<String>)> {
        fleet
            .stations()
            .filter(|ship| !ship.sections.is_empty())
            .filter_map(|ship| {
                let faults = self.faults_of(ship);
                (!faults.is_empty()).then(|| (ship.station_name(), faults))
            })
            .collect()
    }

    /// The faults of ONE lowered ship's station design: what the inspector's
    /// Station Check row reads.
    pub(crate) fn faults_of(&self, ship: &LoweredShip) -> Vec<String> {
        self.with(|sections, items| {
            station_faults(
                &station_design(ship, HullForm::Inline, sections, items),
                sections,
                items,
            )
        })
    }
}

#[cfg(test)]
mod tests;
