//! The base game's streamed world: which rocks, worlds and ships a cell holds.
//!
//! Policy, not mechanism. `nova_world` owns the check every manifest passes;
//! this module and the cluster policy beside it decide where the world's
//! places are, what they are FILLED with out of shipped content, and how far
//! apart it stands them. It takes no list from a caller.

use std::sync::Arc;

use nova_assets::prelude::{ContentCatalogDigest, LoadedSectionPacks};
use nova_events::prelude::Meters;
use nova_world::prelude::*;

use crate::{
    clusters::{plan_sector, validate_cluster_geometry},
    environment::EnvironmentFields,
    ship_parts::{ShipPartFault, ShipPartPack, ShipPartSnapshot},
};

/// Extra room this generator keeps between every pair of clearance spheres
/// in one cell.
///
/// A sector whose bodies merely fail to intersect still reads as a pile. The
/// margin is what makes a generated cell look placed. `validate_manifest`
/// refuses only an overlap; this margin is the generator's own. Two bodies in
/// neighbouring cells are kept apart by the faces instead: each clearance
/// sphere stands wholly inside its own cell.
pub const CLEARANCE_MARGIN: Meters = Meters(500.0);

/// The widest cell edge this generator fills.
///
/// Measurement, not geometry: the cluster halo stays finite at any edge, but
/// 128 km is the widest edge the generator has been measured at, and a cell
/// replays more clusters as its volume grows. The limit stays until a
/// measurement moves it. Refusing the edge at the config fails before anything
/// streams.
const SECTOR_EDGE_MAX: Meters = Meters(128_000.0);

/// The base game's sector generator: a cell filled from the cluster policy.
///
/// Each cell replays the clusters whose bodies can reach it, keeps the bodies
/// whose centres it holds, and draws a small background scatter only when it
/// owns none. Every choice is drawn from streams keyed by the seed and a
/// lattice node or the cell, in a fixed order, so a cell is the same cell in
/// any visit order.
///
/// No content list from a caller. The bodies it draws from are the shipped
/// content the policy names - every natural asteroid kind, every `PlanetType`,
/// and the shipped derelict hulls - so there is no list a caller could leave
/// empty or fill with an id the game does not ship. The ship parts it pins are
/// the snapshot of the loaded catalog it was built from, with that catalog's
/// digest; nothing draws a ship from them yet.
#[derive(Clone, Debug)]
pub struct NovaLayeredWorld {
    parts: Arc<ShipPartSnapshot>,
    catalog: ContentCatalogDigest,
}

/// Equal when both pin one catalog and one ship-part snapshot. Compares two
/// digests, never the parts, so the arming check stays cheap every frame.
impl PartialEq for NovaLayeredWorld {
    fn eq(&self, other: &Self) -> bool {
        self.catalog == other.catalog && self.parts.content_hash() == other.parts.content_hash()
    }
}

impl NovaLayeredWorld {
    /// Pin the ship parts of `loaded` and the digest of the catalog they came
    /// from. Builds the snapshot once; a clone shares it.
    ///
    /// # Errors
    ///
    /// Every fault [`ShipPartSnapshot::build`] finds in the loaded sections.
    pub fn from_loaded(loaded: &LoadedSectionPacks) -> Result<Self, Vec<ShipPartFault>> {
        let packs: Vec<ShipPartPack> = loaded
            .packs
            .iter()
            .map(|pack| ShipPartPack {
                id: pack.id.clone(),
                dependencies: pack.dependencies.clone(),
                sections: pack.sections.clone(),
            })
            .collect();
        Ok(Self {
            parts: Arc::new(ShipPartSnapshot::build(&packs)?),
            catalog: loaded.digest,
        })
    }

    /// The ship parts this world was armed with.
    pub fn parts(&self) -> &ShipPartSnapshot {
        &self.parts
    }

    /// The digest of the catalog this world was armed over.
    pub fn catalog(&self) -> ContentCatalogDigest {
        self.catalog
    }
}

impl SectorGenerator for NovaLayeredWorld {
    /// Refuse an edge wider than the 128 km `SECTOR_EDGE_MAX`, one too narrow
    /// to hold the widest core of worlds or background scatter, and the
    /// cluster bands `validate_cluster_geometry` refuses.
    fn validate(&self, geometry: WorldGeometry) -> Result<(), SectorFault> {
        if geometry.sector_edge > SECTOR_EDGE_MAX {
            return Err(SectorFault::Config {
                field: "sector_edge",
                value: format!(
                    "{} m, wider than the {} m edge limit",
                    geometry.sector_edge.get(),
                    SECTOR_EDGE_MAX.get()
                ),
            });
        }
        validate_cluster_geometry(geometry)
    }

    fn generate(&self, input: SectorGenerationInput) -> Result<SectorManifest, SectorFault> {
        Ok(plan_sector(&EnvironmentFields::new(input.seed), input)?.manifest())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn layered(sector_edge: Meters) -> WorldConfig<NovaLayeredWorld> {
        WorldConfig {
            seed: 20_260_922,
            sector_edge,
            active_radius: 2,
            generator: crate::tests::fixture_world(),
        }
    }

    /// The floor is set by the widest core of worlds: four 1,696 m planetoids
    /// at the corners of a tetrahedron at the outer spacing, so the core
    /// reaches 4,330 m from its anchor and a cell has to be twice that wide.
    /// That is wider than the widest background scatter needs, so it is the
    /// floor a narrow edge meets. Naming the core tells a caller what they
    /// would have to shrink.
    #[test]
    fn an_edge_too_narrow_to_hold_a_core_of_worlds_is_refused() {
        for edge in [Meters(1_000.0), Meters(7_500.0), Meters(8_660.0)] {
            let fault = layered(edge)
                .validate()
                .expect_err("a cell narrower than its own core of worlds must refuse");
            assert!(
                matches!(
                    &fault,
                    SectorFault::Config { field: "sector_edge", value }
                        if value.contains("core of planetoids")
                ),
                "a {edge:?} cell must refuse by naming the core of planetoids, got {fault:?}"
            );
        }
        assert!(
            layered(Meters(8_661.0)).validate().is_ok(),
            "a cell just wide enough for a 4,330 m core must arm"
        );
    }

    /// The 128 km limit itself arms, and the next `f32` above it refuses by
    /// naming the edge and the limit.
    #[test]
    fn an_edge_wider_than_the_edge_limit_is_refused() {
        assert!(
            layered(SECTOR_EDGE_MAX).validate().is_ok(),
            "the 128 km edge limit must arm"
        );
        assert_eq!(
            layered(Meters(SECTOR_EDGE_MAX.get().next_up())).validate(),
            Err(SectorFault::Config {
                field: "sector_edge",
                value: "128000.01 m, wider than the 128000 m edge limit".to_string(),
            }),
        );
    }
}
