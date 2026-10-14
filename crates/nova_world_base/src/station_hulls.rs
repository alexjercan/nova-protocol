//! The hulls a living generated station sells: a short catalogue laid out for
//! its civilization.
//!
//! PURE. Every draw comes from a stream keyed by the world seed and the
//! station's id, so a station sells the same hulls from every cell, in any
//! visit order, and a thawed or loaded station keeps the list it froze with.
//! The catalogue holds [`STATION_HULLS`] hulls, a seeded count. Their roles
//! cycle, from a seeded start, through the roles the civilization's
//! advancement can build, and each is [`generate_ship`] at that advancement,
//! so the generator's size ceiling tiers the catalogue: a station near the
//! origin sells small hulls only.
//!
//! FAIL LOUD. A layout that fails fails the sector, as a planned hull's does.

use nova_gameplay::prelude::{Fnv32, SeedStream};
use nova_scenario::prelude::StationHullOffer;
use nova_world::prelude::ShipRoleType;

use crate::{
    civilizations::Civilization,
    ship_layout::{generate_ship, ShipLayout, ShipLayoutFailure, ShipLayoutRequest},
    ship_parts::ShipPartSnapshot,
};

/// The fewest and the most hulls one station sells, both included.
pub(crate) const STATION_HULLS: (usize, usize) = (3, 5);

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

/// Lay out the catalogue of the living station `station` (its scenario id)
/// that `civilization` keeps, in the world of `seed`: each hull's name and
/// layout, in list order.
///
/// # Errors
///
/// The first [`ShipLayoutFailure`] a hull's layout gives.
pub(crate) fn station_hull_layouts(
    parts: &ShipPartSnapshot,
    seed: u32,
    station: &str,
    civilization: &Civilization,
) -> Result<Vec<(String, ShipRoleType, ShipLayout)>, ShipLayoutFailure> {
    let mut stream = SeedStream::new(hull_key(seed, station, b"plan", 0));
    // A fixed number of draws, in a fixed order, whatever they decide.
    let count_draw = stream.unit();
    let start_draw = stream.unit();
    let (least, most) = STATION_HULLS;
    let count = (least + (count_draw * (most - least + 1) as f32) as usize).min(most);
    // Civilian and industrial are always buildable, so the list is never
    // empty.
    let roles = parts.eligible_roles(civilization.advancement);
    let start = ((start_draw * roles.len() as f32) as usize).min(roles.len() - 1);
    let civilization_name = civilization.id.name();
    (0..count)
        .map(|slot| {
            let role = roles[(start + slot) % roles.len()];
            let mark = slot / roles.len() + 1;
            let layout = generate_ship(
                parts,
                ShipLayoutRequest {
                    seed: hull_key(seed, station, b"hull", slot),
                    civilization: civilization.id,
                    role,
                    advancement: civilization.advancement,
                },
            )?;
            Ok((
                format!("{civilization_name} {} Mk {mark}", role.label()),
                role,
                layout,
            ))
        })
        .collect()
}

/// The catalogue [`station_hull_layouts`] lays out, as the station carries
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
) -> Result<Vec<StationHullOffer>, ShipLayoutFailure> {
    Ok(station_hull_layouts(parts, seed, station, civilization)?
        .into_iter()
        .map(|(name, _, layout)| StationHullOffer {
            name,
            design: layout.design,
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        civilizations::{CivilizationStatusType, CIVILIZATION_LATTICE},
        ship_layout::{ceiling, fixture::packs},
        CivilizationField,
    };

    /// A station in the first ring around the origin sells only small hulls:
    /// every hull's clearance stays under the medium band's ceiling (the
    /// band `no_medium_hull_spawns_inside_the_second_ring` reads). The same
    /// seed and station give the same catalogue, of three to five hulls.
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
                        let hulls = station_hull_layouts(&parts, seed, &station, &civilization)
                            .unwrap_or_else(|failure| panic!("seed {seed}: {failure}"));
                        assert!(
                            (STATION_HULLS.0..=STATION_HULLS.1).contains(&hulls.len()),
                            "seed {seed}: {} hulls",
                            hulls.len()
                        );
                        for (name, _, layout) in &hulls {
                            assert!(
                                layout.clearance < medium,
                                "seed {seed}: '{name}' at advancement {:.3} reaches {:?}, past \
                                 the medium band's {medium:?}",
                                civilization.advancement,
                                layout.clearance
                            );
                        }
                        let again = station_hulls(&parts, seed, &station, &civilization)
                            .expect("the catalogue lays out again");
                        assert_eq!(
                            format!("{again:?}"),
                            format!(
                                "{:?}",
                                station_hulls(&parts, seed, &station, &civilization)
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
                                .map(|(name, _, _)| name.as_str())
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
}
