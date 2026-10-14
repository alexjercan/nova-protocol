//! The hulls a living generated station sells: a short catalogue laid out for
//! its civilization.
//!
//! PURE. Every draw comes from a stream keyed by the world seed and the
//! station's id, so a station sells the same hulls from every cell, in any
//! visit order, and a thawed or loaded station keeps the list it froze with.
//! The station sells one hull per docking port but one, each parked at its
//! port, the last port kept free for the player. Their roles
//! cycle, from a seeded start, through the roles the civilization's
//! advancement can build, and each is [`generate_ship`] at that advancement,
//! so the generator's size ceiling tiers the catalogue: a station near the
//! origin sells small hulls only.
//!
//! After the hulls for sale come [`STATION_ESCORTS`] escorts for hire, a
//! seeded count: each is a fighting role the advancement can build
//! (scavenger, armored), cycled from a seeded start. A civilization that
//! builds no fighting role hires none. Each escort keeps its roster slot, so
//! its id does not move when an earlier one is hired.
//!
//! FAIL LOUD. A layout that fails fails the sector, as a planned hull's does.

use nova_gameplay::prelude::{Fnv32, SeedStream};
use nova_scenario::prelude::StationHullOffer;
use nova_ship::prelude::HullOfferType;
use nova_world::prelude::ShipRoleType;

use crate::{
    civilizations::Civilization,
    ship_layout::{generate_ship, ShipLayout, ShipLayoutFailure, ShipLayoutRequest},
    ship_parts::ShipPartSnapshot,
};

/// The fewest and the most escorts one station hires out, both included,
/// when its civilization builds a fighting role.
pub(crate) const STATION_ESCORTS: (usize, usize) = (1, 3);

/// The seed of one `aspect` of slot `slot` of the station `station`.
fn hull_key(seed: u32, station: &str, aspect: &[u8], slot: usize) -> u32 {
    Fnv32::new()
        .write(&seed.to_le_bytes())
        .write(b"station_hulls")
        .write(aspect)
        .write(station.as_bytes())
        .write(&(slot as u64).to_le_bytes())
        .finish()
}

/// One catalogue entry as laid out: its name, terms, role and layout.
pub(crate) type StationHullLayout = (String, HullOfferType, ShipRoleType, ShipLayout);

/// Lay out the catalogue of the living station `station` (its scenario id)
/// that `civilization` keeps, in the world of `seed`, `sales` hulls for
/// sale: each hull's name, terms and layout, in list order, the hulls for
/// sale first.
///
/// # Errors
///
/// The first [`ShipLayoutFailure`] a hull's layout gives.
pub(crate) fn station_hull_layouts(
    parts: &ShipPartSnapshot,
    seed: u32,
    station: &str,
    civilization: &Civilization,
    sales: usize,
) -> Result<Vec<StationHullLayout>, ShipLayoutFailure> {
    let civilization_name = civilization.id.name();
    let layout = |aspect: &[u8], slot: usize, role: ShipRoleType| {
        generate_ship(
            parts,
            ShipLayoutRequest {
                seed: hull_key(seed, station, aspect, slot),
                civilization: civilization.id,
                role,
                advancement: civilization.advancement,
            },
        )
    };
    // Civilian and industrial are always buildable, so the list is never
    // empty.
    let roles = parts.eligible_roles(civilization.advancement);
    let sales = seeded_roster(hull_key(seed, station, b"plan", 0), (sales, sales), &roles).map(
        |(slot, role)| {
            let mark = slot / roles.len() + 1;
            Ok((
                format!("{civilization_name} {} Mk {mark}", role.label()),
                HullOfferType::Sale,
                role,
                layout(b"hull", slot, role)?,
            ))
        },
    );
    let armed: Vec<ShipRoleType> = roles
        .iter()
        .copied()
        .filter(|role| matches!(role, ShipRoleType::Scavenger | ShipRoleType::Armored))
        .collect();
    let escorts = seeded_roster(
        hull_key(seed, station, b"escort_plan", 0),
        STATION_ESCORTS,
        &armed,
    )
    .map(|(slot, role)| {
        Ok((
            format!("{civilization_name} {} escort {}", role.label(), slot + 1),
            HullOfferType::Hire { slot: slot as u32 },
            role,
            layout(b"escort", slot, role)?,
        ))
    });
    sales.chain(escorts).collect()
}

/// The slots of a roster drawn from `key`: a count in `bounds` and a start in
/// `roles`, two draws whatever they decide, then the roles cycled from the
/// start. Empty when `roles` is.
fn seeded_roster(
    key: u32,
    (least, most): (usize, usize),
    roles: &[ShipRoleType],
) -> impl Iterator<Item = (usize, ShipRoleType)> + '_ {
    let mut stream = SeedStream::new(key);
    let count_draw = stream.unit();
    let start_draw = stream.unit();
    let count = if roles.is_empty() {
        0
    } else {
        (least + (count_draw * (most - least + 1) as f32) as usize).min(most)
    };
    let start = ((start_draw * roles.len() as f32) as usize).min(roles.len().saturating_sub(1));
    (0..count).map(move |slot| (slot, roles[(start + slot) % roles.len()]))
}

/// The catalogue [`station_hull_layouts`] lays out for a station of `ports`
/// docking ports, one hull for sale at each but one, as the station carries
/// it.
///
/// # Errors
///
/// The first [`ShipLayoutFailure`] a hull's layout gives.
pub(crate) fn station_hulls(
    parts: &ShipPartSnapshot,
    seed: u32,
    station: &str,
    civilization: &Civilization,
    ports: usize,
) -> Result<Vec<StationHullOffer>, ShipLayoutFailure> {
    Ok(
        station_hull_layouts(parts, seed, station, civilization, ports.saturating_sub(1))?
            .into_iter()
            .map(|(name, terms, _, layout)| StationHullOffer {
                name,
                terms,
                design: layout.design,
            })
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        civilizations::{CivilizationStatusType, CIVILIZATION_LATTICE},
        ship_layout::{ceiling, fixture::packs},
        CivilizationField,
    };

    /// The ports of a shipped ring station.
    const PORTS: usize = 4;

    /// A station in the first ring around the origin sells only small hulls:
    /// every hull's clearance stays under the medium band's ceiling (the
    /// band `no_medium_hull_spawns_inside_the_second_ring` reads). The same
    /// seed and station give the same catalogue, one hull for sale per port
    /// but one.
    #[test]
    fn a_first_ring_station_sells_small_hulls_and_the_same_catalogue_for_a_seed() {
        let parts = ShipPartSnapshot::build(&packs()).expect("the packs build");
        let medium = ceiling(1.0 / 3.0);
        let mut stations = 0;
        for seed in 0..4 {
            let field = CivilizationField::new(seed);
            for x in -1..=1 {
                for y in -1..=1 {
                    for z in -1..=1 {
                        let civilization = field.civilization([x, y, z]);
                        if civilization.status != CivilizationStatusType::Living
                            || civilization.centroid.length().0 > CIVILIZATION_LATTICE.0
                        {
                            continue;
                        }
                        stations += 1;
                        let station = format!("ring_one_{x}_{y}_{z}_station");
                        let hulls =
                            station_hull_layouts(&parts, seed, &station, &civilization, PORTS - 1)
                                .unwrap_or_else(|failure| panic!("seed {seed}: {failure}"));
                        let sales = hulls
                            .iter()
                            .filter(|(_, terms, _, _)| *terms == HullOfferType::Sale)
                            .count();
                        assert_eq!(sales, PORTS - 1, "seed {seed}: {sales} hulls for sale");
                        for (name, _, _, layout) in &hulls {
                            assert!(
                                layout.clearance < medium,
                                "seed {seed}: '{name}' at advancement {:.3} reaches {:?}, past \
                                 the medium band's {medium:?}",
                                civilization.advancement,
                                layout.clearance
                            );
                        }
                        let again = station_hulls(&parts, seed, &station, &civilization, PORTS)
                            .expect("the catalogue lays out again");
                        assert_eq!(
                            format!("{again:?}"),
                            format!(
                                "{:?}",
                                station_hulls(&parts, seed, &station, &civilization, PORTS)
                                    .expect("the catalogue lays out")
                            ),
                            "seed {seed}: the catalogue of '{station}' moved"
                        );
                        assert_eq!(
                            again
                                .iter()
                                .map(|offer| offer.name.as_str())
                                .collect::<Vec<_>>(),
                            hulls
                                .iter()
                                .map(|(name, _, _, _)| name.as_str())
                                .collect::<Vec<_>>()
                        );
                    }
                }
            }
        }
        assert!(
            stations >= 4,
            "the sweep saw {stations} first-ring stations"
        );
    }

    /// A living station's escorts for hire follow its hulls for sale: one to
    /// three fighting roles in roster-slot order, none where the civilization
    /// builds no fighting role, and the same roster for the same seed and
    /// station.
    #[test]
    fn a_station_hires_out_armed_escorts_after_its_sales_and_the_same_roster_for_a_seed() {
        let parts = ShipPartSnapshot::build(&packs()).expect("the packs build");
        let mut hiring = 0;
        for seed in 0..4 {
            let field = CivilizationField::new(seed);
            for x in -2..=2 {
                for y in -1..=1 {
                    for z in -1..=1 {
                        let civilization = field.civilization([x, y, z]);
                        if civilization.status != CivilizationStatusType::Living {
                            continue;
                        }
                        let station = format!("sweep_{x}_{y}_{z}_station");
                        let hulls =
                            station_hull_layouts(&parts, seed, &station, &civilization, PORTS - 1)
                                .unwrap_or_else(|failure| panic!("seed {seed}: {failure}"));
                        let first_hire = hulls
                            .iter()
                            .position(|(_, terms, _, _)| *terms != HullOfferType::Sale)
                            .unwrap_or(hulls.len());
                        let escorts = &hulls[first_hire..];
                        let armed = parts
                            .eligible_roles(civilization.advancement)
                            .into_iter()
                            .any(|role| {
                                matches!(role, ShipRoleType::Scavenger | ShipRoleType::Armored)
                            });
                        if armed {
                            hiring += 1;
                            assert!(
                                (STATION_ESCORTS.0..=STATION_ESCORTS.1).contains(&escorts.len()),
                                "seed {seed}: '{station}' hires out {} escorts",
                                escorts.len()
                            );
                        } else {
                            assert!(escorts.is_empty(), "seed {seed}: '{station}' unarmed");
                        }
                        for (slot, (name, terms, role, _)) in escorts.iter().enumerate() {
                            assert_eq!(
                                *terms,
                                HullOfferType::Hire { slot: slot as u32 },
                                "seed {seed}: '{name}' is out of slot order"
                            );
                            assert!(
                                matches!(role, ShipRoleType::Scavenger | ShipRoleType::Armored),
                                "seed {seed}: escort '{name}' is a {role:?}"
                            );
                        }
                        assert_eq!(
                            format!(
                                "{:?}",
                                station_hulls(&parts, seed, &station, &civilization, PORTS)
                            ),
                            format!(
                                "{:?}",
                                station_hulls(&parts, seed, &station, &civilization, PORTS)
                            ),
                            "seed {seed}: the roster of '{station}' moved"
                        );
                    }
                }
            }
        }
        assert!(
            hiring >= 4,
            "the sweep saw {hiring} stations hiring escorts"
        );
    }
}
