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
//! Every station of one civilization is built from one generated design:
//! [`station_archetype`] picks its archetype off the civilization's stream,
//! [`generate_station`] lays the hull out at the civilization's advancement
//! from a seed off that stream too, and [`generated_station`] stocks it. A
//! dead station is that design spawned derelict, its stock and credits left
//! as loot.
//!
//! Every chance here is provisional until generated stations are reviewed in
//! travel.

use std::collections::BTreeMap;

use nova_events::prelude::{Meters, Meters3};
use nova_gameplay::prelude::{CivilizationId, GameItems};
use nova_scenario::prelude::StationDesign;
use nova_world::prelude::*;

use crate::{
    civilizations::{Civilization, CivilizationField, CivilizationStatusType},
    clusters::CLUSTER_LATTICE,
    ship_parts::ShipPartSnapshot,
    station_layout::{
        generate_station, generated_station, station_archetype, StationLayoutRequest,
    },
};

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

/// The station every station of one civilization is built from.
pub(crate) struct CivilizationStation {
    /// The design, stocked for the civilization's advancement.
    pub(crate) design: StationDesign,
    /// The radius around the root that holds the hull in any orientation.
    pub(crate) clearance: Meters,
    /// How many docking ports the hull carries: the station parks one hull
    /// for sale at each but one.
    pub(crate) ports: usize,
}

/// Lay out and stock the station `civilization` builds from `parts`: its
/// [`station_archetype`], at its advancement, from a seed off its own
/// stream.
///
/// # Errors
///
/// [`SectorFault::Generation`] naming the seed, the civilization, the
/// archetype, the advancement and the constraint the last layout failed:
/// the parts cannot build this civilization a station, and that is a bug
/// in the generator or the catalog, not a world to quietly generate.
fn civilization_station(
    parts: &ShipPartSnapshot,
    items: &GameItems,
    civilization: &Civilization,
) -> Result<CivilizationStation, SectorFault> {
    let request = StationLayoutRequest {
        seed: civilization.id.stream(b"station_layout").next_u32(),
        civilization: civilization.id,
        archetype: station_archetype(civilization),
        advancement: civilization.advancement,
    };
    let layout = generate_station(parts, request).map_err(|failure| SectorFault::Generation {
        id: civilization.id.to_string(),
        field: "station",
        value: failure.to_string(),
    })?;
    Ok(CivilizationStation {
        design: generated_station(parts, items, &layout, civilization),
        clearance: layout.clearance,
        ports: layout.ports,
    })
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

/// The station claims one cell's plan resolves, and the station each
/// claiming civilization builds, memoized per civilization so the halo's
/// clusters share each civilization's layout and node walk.
pub(crate) struct StationClaims<'a> {
    parts: &'a ShipPartSnapshot,
    items: &'a GameItems,
    stations: BTreeMap<CivilizationId, CivilizationStation>,
    nodes: BTreeMap<CivilizationId, Vec<[i32; 3]>>,
}

impl<'a> StationClaims<'a> {
    /// Claims whose stations are laid out from `parts` and stocked from
    /// `items`.
    pub(crate) fn new(parts: &'a ShipPartSnapshot, items: &'a GameItems) -> Self {
        Self {
            parts,
            items,
            stations: BTreeMap::new(),
            nodes: BTreeMap::new(),
        }
    }

    /// The station `civilization` builds, laid out once.
    ///
    /// # Errors
    ///
    /// Whatever [`civilization_station`] refuses.
    pub(crate) fn station(
        &mut self,
        civilization: &Civilization,
    ) -> Result<&CivilizationStation, SectorFault> {
        if !self.stations.contains_key(&civilization.id) {
            let station = civilization_station(self.parts, self.items, civilization)?;
            self.stations.insert(civilization.id, station);
        }
        Ok(&self.stations[&civilization.id])
    }

    /// The nodes `civilization` keeps stations at: the first
    /// [`station_count`] nodes by centroid distance for which `hosts` says
    /// a cluster grows there that can hold a station of the clearance it is
    /// given, the clearance of the civilization's own station.
    ///
    /// # Errors
    ///
    /// Whatever [`Self::station`] or `hosts` refuses, and
    /// [`SectorFault::Generation`] when fewer than [`station_count`] nodes of
    /// the search block host: a civilization that cannot place its stations
    /// is a bug in the bands, not a world to quietly generate.
    pub(crate) fn nodes(
        &mut self,
        civilization: &Civilization,
        hosts: &mut dyn FnMut(Meters, [i32; 3]) -> Result<bool, SectorFault>,
    ) -> Result<&[[i32; 3]], SectorFault> {
        if !self.nodes.contains_key(&civilization.id) {
            let clearance = self.station(civilization)?.clearance;
            let wanted = station_count(civilization);
            let mut found = Vec::with_capacity(wanted);
            for node in nodes_by_distance(civilization.centroid) {
                if found.len() == wanted {
                    break;
                }
                if hosts(clearance, node)? {
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
        hosts: &mut dyn FnMut(Meters, [i32; 3]) -> Result<bool, SectorFault>,
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

    use nova_gameplay::prelude::Allegiance;
    use nova_scenario::prelude::ASTEROID_GEOMETRIC_FACTOR_MAX;

    use super::*;
    use crate::{
        civilizations::INFLUENCE_REACH,
        clusters::{cluster_at, plan_sector, station_pose, Cluster},
        environment::EnvironmentFields,
        layered::NovaLayeredWorld,
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

    /// Across three seeds, every civilization of the 27 nodes around the
    /// origin keeps the stations its status and advancement promise: a
    /// living one at least one, an extinct one exactly one and dead, each at
    /// a cluster whose anchor is in its reach, the first at the nearest
    /// hosting cluster; every one of them is in the manifest of the cell it
    /// stands in, once per cluster, keeping the civilization that claimed it,
    /// standing clear of every other body of its cell, its hull the one its
    /// civilization's station was laid out to; and the sweep sees both a
    /// living and a dead station and more than one archetype, so it is not
    /// vacuous and the per-civilization draw varies.
    #[test]
    fn every_civilization_keeps_its_stations_at_distinct_clear_clusters() {
        let (mut living, mut dead) = (0, 0);
        let mut archetypes = BTreeSet::new();
        for seed in [20_260_922, 12_345, 4_000_000_000, 6, 10] {
            let config = config(seed);
            let edge = config.sector_edge;
            let fields = EnvironmentFields::new(seed);
            let civilizations = CivilizationField::new(seed);
            let parts = config.generator.parts();
            let items = config.generator.items();
            let mut hosts = |clearance: Meters, node: [i32; 3]| {
                Ok(cluster_at(&fields, seed, edge, node)?
                    .is_some_and(|cluster| station_pose(&cluster, edge, clearance, seed).is_some()))
            };
            let mut claims = StationClaims::new(parts, items);
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
                            let built = claims
                                .station(&civilization)
                                .expect("the claim laid it out");
                            let (clearance, hull) =
                                (built.clearance, format!("{:?}", built.design.hull));
                            let pose = station_pose(&cluster, edge, clearance, seed)
                                .expect("a claimed node hosts a station");
                            let coord = SectorCoord::containing(pose.position, edge);
                            let manifest = manifests.entry(coord).or_insert_with(|| {
                                plan_sector(&fields, parts, items, config.input(coord))
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
                                    assert_eq!(station.spawn.allegiance, Allegiance::Neutral);
                                    dead += 1;
                                }
                            }
                            assert_eq!(station.clearance, clearance, "{seed} {stem}");
                            assert_eq!(
                                format!("{:?}", station.design().hull),
                                hull,
                                "{seed} {stem}: the station is its civilization's"
                            );
                            archetypes.insert(station_archetype(&civilization));
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
        assert!(
            archetypes.len() > 1,
            "every station drew one archetype: {archetypes:?}"
        );
    }
}
