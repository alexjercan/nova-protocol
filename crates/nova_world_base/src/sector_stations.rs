//! The open world's generated stations: which civilization keeps a station
//! at which cluster, how many a civilization keeps, and the design each one
//! is built from.
//!
//! PURE. A civilization's station nodes are a function of the world seed and
//! the civilization alone: the cluster nodes nearest its centroid, in centroid
//! distance order, that grow a cluster able to host a station. So a cluster
//! holds the same station from every cell and in any visit order, and a
//! claim never depends on which cell asked first.
//!
//! A living civilization keeps at least one station, at the cluster nearest
//! its centroid, and more as its advancement rises. An extinct civilization
//! left exactly one, dead: the same design spawned `derelict`, its hull
//! inactive and lootable, with no service offered on it. A cluster holds at
//! most one station; when two civilizations claim one cluster, the one whose
//! centroid is nearer keeps it, and the other's claim is simply fewer. The
//! nearest cluster of a civilization is never another's: the civilization
//! lattice stands centroids at least 160 km apart, and a nearest cluster node
//! is within 44 km of its centroid.
//!
//! A living station's stock and credits scale with its civilization's
//! advancement, from half its design's at advancement 0 to all of it at 1; a
//! dead station keeps the design's stock and credits as loot. See
//! [`generated_station_design`].
//!
//! Every chance here is provisional until generated stations are reviewed in
//! travel.

use std::collections::BTreeMap;

use nova_events::prelude::Meters3;
use nova_gameplay::prelude::ShipInventoryStock;
use nova_scenario::prelude::StationDesign;
use nova_world::prelude::*;

use crate::{
    civilizations::{Civilization, CivilizationField, CivilizationStatusType},
    clusters::CLUSTER_LATTICE,
    BLOCK_WAYSTATION_STATION_ID,
};

/// Every station design id the generator places. Arming pins each from the
/// loaded catalog, and the base lint refuses a base bundle that authors none
/// of them.
pub const GENERATED_STATION_DESIGN_IDS: &[&str] = &[BLOCK_WAYSTATION_STATION_ID];

/// How many stations a living civilization keeps past its first at full
/// advancement: one per tier, a tier being an equal share of `[0, 1]`
/// advancement. Two tiers, so the count is one to three. A civilization has
/// no size trait and every reach is the same, so advancement is the one
/// trait that can scale it.
const STATION_ADVANCEMENT_TIERS: f32 = 2.0;

/// How many lattice steps from the node nearest a centroid the station nodes
/// are looked for: a 5x5x5 block, 125 nodes. Every node of the block is
/// within 125 km of the centroid, inside the 320 km influence reach.
const STATION_NODE_SEARCH: i32 = 2;

/// The station design `civilization` builds.
///
/// TODO(20261012-100005): every civilization builds the base waystation
/// until the per-civilization station catalogue lands; replace this with a
/// seeded draw from the civilization's own designs and delete
/// [`GENERATED_STATION_DESIGN_IDS`] when that task widens the catalogue.
pub fn station_design_id(_civilization: &Civilization) -> &'static str {
    GENERATED_STATION_DESIGN_IDS[0]
}

/// The share of its design's stock and credits a living station is generated
/// with, at advancement 0 and at advancement 1, read linearly between. Never
/// above 1, so the scaled stock always fits the hull hold that content lint
/// checked the authored stock against; at least a half, so a frontier
/// station still trades enough to be worth the dock.
const STATION_STOCK_SHARE: (f32, f32) = (0.5, 1.0);

/// The design a generated station of `civilization` is built from.
///
/// A living civilization's station takes `design` with every stock line and
/// the credits scaled by [`STATION_STOCK_SHARE`] read at its advancement: a
/// stack floors, but never below one, so every line the design stocks is
/// still stocked; the credits round to the nearest. An extinct
/// civilization's dead station keeps the authored stock and credits: it is
/// loot a docked ship takes, as a derelict hull's hold is. Every other field
/// is the design's.
///
/// PURE: the same design and civilization give the same result.
pub fn generated_station_design(
    design: &StationDesign,
    civilization: &Civilization,
) -> StationDesign {
    if civilization.status == CivilizationStatusType::Extinct {
        return design.clone();
    }
    let (least, most) = STATION_STOCK_SHARE;
    let share = least + (most - least) * civilization.advancement.clamp(0.0, 1.0);
    StationDesign {
        stock: ShipInventoryStock::new(
            design.stock.stacks().map(|(item, count)| {
                (item.clone(), ((count as f32 * share).floor() as u32).max(1))
            }),
        ),
        credits: (design.credits as f32 * share).round() as u32,
        ..design.clone()
    }
}

/// How many stations `civilization` keeps: one plus a tier per
/// [`STATION_ADVANCEMENT_TIERS`] share of advancement for a living one, and
/// exactly one dead station for an extinct one.
pub fn station_count(civilization: &Civilization) -> usize {
    match civilization.status {
        CivilizationStatusType::Living => {
            1 + (civilization.advancement.clamp(0.0, 1.0) * STATION_ADVANCEMENT_TIERS).floor()
                as usize
        }
        CivilizationStatusType::Extinct => 1,
    }
}

/// The lattice nodes around `centroid`, nearest first, that a civilization's
/// stations are looked for at. Ties break by node order, so the order is a
/// fact about the centroid.
fn nodes_by_distance(centroid: Meters3) -> Vec<[i32; 3]> {
    let lattice = CLUSTER_LATTICE.get();
    let home = centroid
        .get()
        .to_array()
        .map(|axis| (axis / lattice).round() as i32);
    let mut nodes = Vec::with_capacity(125);
    for x in home[0] - STATION_NODE_SEARCH..=home[0] + STATION_NODE_SEARCH {
        for y in home[1] - STATION_NODE_SEARCH..=home[1] + STATION_NODE_SEARCH {
            for z in home[2] - STATION_NODE_SEARCH..=home[2] + STATION_NODE_SEARCH {
                nodes.push([x, y, z]);
            }
        }
    }
    nodes.sort_by(|a, b| {
        let distance = |node: &[i32; 3]| {
            let [x, y, z] = node.map(|index| index as f32 * lattice);
            centroid.distance(Meters3::new(x, y, z)).get()
        };
        distance(a).total_cmp(&distance(b)).then_with(|| a.cmp(b))
    });
    nodes
}

/// The station claims one cell's plan resolves, memoized per civilization so
/// the halo's clusters share each civilization's node walk.
pub(crate) struct StationClaims {
    nodes: BTreeMap<CivilizationId, Vec<[i32; 3]>>,
}

impl StationClaims {
    pub(crate) fn new() -> Self {
        Self {
            nodes: BTreeMap::new(),
        }
    }

    /// The nodes `civilization` keeps stations at: the first
    /// [`station_count`] nodes by centroid distance for which `hosts` says
    /// a cluster grows there that can hold a station.
    ///
    /// # Errors
    ///
    /// Whatever `hosts` refuses, and [`SectorFault::Generation`] when fewer
    /// than [`station_count`] nodes of the search block host: a civilization
    /// that cannot place its stations is a bug in the bands, not a world to
    /// quietly generate.
    pub(crate) fn nodes(
        &mut self,
        civilization: &Civilization,
        hosts: &mut dyn FnMut(&Civilization, [i32; 3]) -> Result<bool, SectorFault>,
    ) -> Result<&[[i32; 3]], SectorFault> {
        if !self.nodes.contains_key(&civilization.id) {
            let wanted = station_count(civilization);
            let mut found = Vec::with_capacity(wanted);
            for node in nodes_by_distance(civilization.centroid) {
                if found.len() == wanted {
                    break;
                }
                if hosts(civilization, node)? {
                    found.push(node);
                }
            }
            if found.len() < wanted {
                return Err(SectorFault::Generation {
                    id: civilization.id.to_string(),
                    field: "stations",
                    value: format!(
                        "{} of {wanted} station nodes host a station within {} lattice steps \
                         of the centroid",
                        found.len(),
                        STATION_NODE_SEARCH
                    ),
                });
            }
            self.nodes.insert(civilization.id, found);
        }
        Ok(&self.nodes[&civilization.id])
    }

    /// The civilization whose station the cluster at `node`, anchored at
    /// `anchor`, holds: among the civilizations in reach of the anchor that
    /// claim the node, the one whose centroid is nearest, ties by id. `None`
    /// when no civilization claims it.
    ///
    /// # Errors
    ///
    /// Whatever [`CivilizationField::in_reach`] refuses at the anchor, and
    /// whatever [`Self::nodes`] refuses for a civilization in reach.
    pub(crate) fn claim(
        &mut self,
        civilizations: &CivilizationField,
        anchor: Meters3,
        node: [i32; 3],
        hosts: &mut dyn FnMut(&Civilization, [i32; 3]) -> Result<bool, SectorFault>,
    ) -> Result<Option<Civilization>, SectorFault> {
        let mut reach = civilizations.in_reach(anchor)?;
        reach.sort_by(|a, b| {
            a.distance
                .get()
                .total_cmp(&b.distance.get())
                .then_with(|| a.civilization.id.cmp(&b.civilization.id))
        });
        for entry in reach {
            if self.nodes(&entry.civilization, hosts)?.contains(&node) {
                return Ok(Some(entry.civilization));
            }
        }
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};

    use nova_events::prelude::Meters;
    use nova_gameplay::prelude::Allegiance;
    use nova_scenario::prelude::ASTEROID_GEOMETRIC_FACTOR_MAX;

    use super::*;
    use crate::{
        civilizations::INFLUENCE_REACH,
        clusters::{cluster_at, plan_sector, station_pose, Cluster},
        environment::EnvironmentFields,
        layered::NovaLayeredWorld,
        SHIP_ADVANCEMENT_CURVE,
    };

    /// The examples' cell on `seed`, filled by the fixture generator.
    fn config(seed: u32) -> WorldConfig<NovaLayeredWorld> {
        WorldConfig {
            seed,
            sector_edge: Meters(32_000.0),
            active_radius: 2,
            generator: crate::tests::fixture_world(),
        }
    }

    /// A living station is generated with half its design's stock and
    /// credits at advancement 0 and all of it at 1, every stocked line kept
    /// at one at least; an extinct civilization's dead station keeps the
    /// design's stock and credits as loot.
    #[test]
    fn a_living_station_scales_its_stock_by_advancement_and_a_dead_one_keeps_it() {
        use nova_gameplay::prelude::{ItemDesignId, ItemRoleType};

        let plate = ItemDesignId::from(ItemRoleType::HullPlate);
        let rations = ItemDesignId::from(ItemRoleType::Rations);
        let core = ItemDesignId::from(ItemRoleType::SalvagedParts);
        let design = StationDesign {
            stock: ShipInventoryStock::new([
                (plate.clone(), 8),
                (rations.clone(), 21),
                (core.clone(), 1),
            ]),
            credits: 5_001,
            ..crate::test_support::fixture_stations().0.remove(0)
        };
        let base =
            CivilizationField::new(20_260_922, SHIP_ADVANCEMENT_CURVE).civilization([0, 0, 0]);
        let at = |status, advancement| {
            let generated = generated_station_design(
                &design,
                &Civilization {
                    status,
                    advancement,
                    ..base
                },
            );
            assert_eq!(
                (&generated.id, &generated.services, generated.storage_cap_g),
                (&design.id, &design.services, design.storage_cap_g),
                "only the stock and credits scale"
            );
            let stock: Vec<(ItemDesignId, u32)> = generated
                .stock
                .stacks()
                .map(|(item, count)| (item.clone(), count))
                .collect();
            (stock, generated.credits)
        };
        let living = CivilizationStatusType::Living;
        assert_eq!(
            at(living, 0.0),
            (
                vec![(plate.clone(), 4), (rations.clone(), 10), (core.clone(), 1)],
                2_501
            )
        );
        assert_eq!(
            at(living, 0.5),
            (
                vec![(plate.clone(), 6), (rations.clone(), 15), (core.clone(), 1)],
                3_751
            )
        );
        let authored = (
            vec![(plate.clone(), 8), (rations.clone(), 21), (core.clone(), 1)],
            5_001,
        );
        assert_eq!(at(living, 1.0), authored);
        assert_eq!(at(CivilizationStatusType::Extinct, 0.0), authored);
    }

    /// Across three seeds, every civilization of the 27 nodes around the
    /// origin keeps the stations its status and advancement promise: a
    /// living one at least one, an extinct one exactly one and dead, each at
    /// a cluster whose anchor is in its reach, the first at the nearest
    /// hosting cluster; every one of them is in the manifest of the cell it
    /// stands in, once per cluster, keeping the civilization that claimed it,
    /// standing clear of every other body of its cell; and the sweep sees
    /// both a living and a dead station, so it is not vacuous.
    #[test]
    fn every_civilization_keeps_its_stations_at_distinct_clear_clusters() {
        let (mut living, mut dead) = (0, 0);
        for seed in [20_260_922, 12_345, 4_000_000_000] {
            let config = config(seed);
            let edge = config.sector_edge;
            let fields = EnvironmentFields::new(seed);
            let civilizations = CivilizationField::new(seed, SHIP_ADVANCEMENT_CURVE);
            let pinned = config.generator.stations();
            let mut hosts = |civilization: &Civilization, node: [i32; 3]| {
                let clearance = pinned
                    .iter()
                    .find(|station| station.design.id == station_design_id(civilization).into())
                    .expect("the fixture pins the design")
                    .clearance;
                Ok(cluster_at(&fields, seed, edge, node)?
                    .is_some_and(|cluster| station_pose(&cluster, edge, clearance, seed).is_some()))
            };
            let mut claims = StationClaims::new();
            let mut manifests: BTreeMap<SectorCoord, SectorManifest> = BTreeMap::new();
            let mut stems: BTreeSet<String> = BTreeSet::new();
            for x in -1..=1 {
                for y in -1..=1 {
                    for z in -1..=1 {
                        let civilization = civilizations.civilization([x, y, z]);
                        let nodes = claims
                            .nodes(&civilization, &mut hosts)
                            .unwrap_or_else(|fault| panic!("{seed} {}: {fault}", civilization.id))
                            .to_vec();
                        let wanted = station_count(&civilization);
                        assert_eq!(nodes.len(), wanted, "{seed} {}", civilization.id);
                        match civilization.status {
                            CivilizationStatusType::Living => assert!(wanted >= 1),
                            CivilizationStatusType::Extinct => assert_eq!(wanted, 1),
                        }
                        let mut last_distance = 0.0_f32;
                        for node in nodes {
                            let cluster = cluster_at(&fields, seed, edge, node)
                                .expect("the claim built it")
                                .expect("a claimed node hosts a cluster");
                            let distance = civilization.centroid.distance(cluster.anchor).get();
                            assert!(
                                distance >= last_distance,
                                "{seed} {}: stations are claimed nearest first",
                                civilization.id
                            );
                            last_distance = distance;
                            assert!(
                                distance < INFLUENCE_REACH.get(),
                                "{seed} {}: a station {distance} m from its centroid is out of \
                                 reach",
                                civilization.id
                            );
                            let stem = format!("{}_station", Cluster::slug(node));
                            assert!(
                                stems.insert(stem.clone()),
                                "{seed}: cluster {stem} is claimed by two civilizations"
                            );
                            let clearance = pinned[0].clearance;
                            let pose = station_pose(&cluster, edge, clearance, seed)
                                .expect("a claimed node hosts a station");
                            let coord = SectorCoord::containing(pose.position, edge);
                            let manifest = manifests.entry(coord).or_insert_with(|| {
                                plan_sector(
                                    &fields,
                                    config.generator.parts(),
                                    config.generator.items(),
                                    pinned,
                                    config.input(coord),
                                )
                                .and_then(|plan| plan.manifest())
                                .unwrap_or_else(|fault| panic!("{seed} {coord}: {fault}"))
                            });
                            let placed: Vec<&SectorStation> = manifest
                                .stations
                                .iter()
                                .filter(|station| station.id.stem == stem)
                                .collect();
                            let [station] = placed[..] else {
                                panic!(
                                    "{seed} {coord}: cluster {stem} places {} stations, not one",
                                    placed.len()
                                );
                            };
                            assert_eq!(station.civilization, civilization.id, "{seed} {stem}");
                            assert_eq!(station.position, pose.position, "{seed} {stem}");
                            match civilization.status {
                                CivilizationStatusType::Living => {
                                    assert!(!station.is_dead(), "{seed} {stem} is dead");
                                    living += 1;
                                }
                                CivilizationStatusType::Extinct => {
                                    assert!(station.is_dead(), "{seed} {stem} is living");
                                    assert_eq!(station.spawn.allegiance, Some(Allegiance::Neutral));
                                    dead += 1;
                                }
                            }
                            assert_eq!(station.spawn.design, station.design.id);
                            let clear = |other: Meters3, reach: f32, what: &str| {
                                let gap = station.position.distance(other).get();
                                assert!(
                                    gap > reach + station.clearance.get(),
                                    "{seed} {stem} stands {gap} m from {what} of reach {reach} m"
                                );
                            };
                            for rock in &manifest.asteroids {
                                clear(
                                    rock.position,
                                    rock.config.radius.get() * ASTEROID_GEOMETRIC_FACTOR_MAX,
                                    &rock.id.to_string(),
                                );
                            }
                            for planet in &manifest.planets {
                                clear(
                                    planet.position,
                                    planet.config.body_radius().get(),
                                    &planet.id.to_string(),
                                );
                            }
                            for ship in &manifest.ships {
                                clear(ship.position, ship.clearance.get(), &ship.id.to_string());
                            }
                            for other in manifest
                                .stations
                                .iter()
                                .filter(|other| other.id != station.id)
                            {
                                clear(other.position, other.clearance.get(), &other.id.to_string());
                            }
                        }
                    }
                }
            }
        }
        assert!(
            living > 0 && dead > 0,
            "{living} living and {dead} dead stations seen"
        );
    }
}
