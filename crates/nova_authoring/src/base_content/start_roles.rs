//! Built-in START ROLE content: the six New Game starts.
//!
//! `content gen` serializes this builder into
//! `assets/base/roles/base.content.ron`. Each kit is worth 3,000 cr at ask:
//! the credits plus the items at their base `ask_cr`.

use nova_gameplay::prelude::{ItemRoleType, ShipInventoryStock};
use nova_scenario::prelude::{StartKit, StartRole};

use super::ships::{
    INTERCEPTOR_SHIP_ID, LIGHT_FREIGHTER_SHIP_ID, MINING_TUG_SHIP_ID, PATROL_PICKET_SHIP_ID,
    RAIDER_SHIP_ID, SALVAGE_HAULER_SHIP_ID,
};

/// Every shipped start role, in the order a picker lists them.
pub(crate) fn start_role_catalog() -> Vec<StartRole> {
    vec![
        role(
            "industrial",
            "Industrial",
            "A mining tug with a bow mining beam and a cargo intake, unarmed. Cut ore from \
             rocks and sell it.",
            MINING_TUG_SHIP_ID,
            2_200,
            &[(ItemRoleType::HullPlate, 15), (ItemRoleType::Rations, 25)],
            "Mining crews work the rock fields for whoever buys the ore. The tug is old, but \
             it pays.",
        ),
        role(
            "scavenger",
            "Scavenger",
            "A salvage hauler with the largest start hold, a cargo intake and one \
             point-defense mount. Pick wrecks clean and sell what you take.",
            SALVAGE_HAULER_SHIP_ID,
            1_800,
            &[(ItemRoleType::PdcRound, 200), (ItemRoleType::HullPlate, 10)],
            "Every fight leaves wrecks behind. Scavengers take what still has a price.",
        ),
        role(
            "transport",
            "Transport",
            "A light freighter with freight cubes, a tank pod and no weapons. Buy cargo at \
             one station and sell it at another.",
            LIGHT_FREIGHTER_SHIP_ID,
            2_000,
            &[(ItemRoleType::Rations, 50), (ItemRoleType::WaterIce, 50)],
            "Stations need food, water and parts they cannot make. Haulers carry them \
             between stations.",
        ),
        role(
            "bounty_hunter",
            "Bounty Hunter",
            "An interceptor with two point-defense mounts and a bow torpedo bay. Hunt armed \
             ships for the price on them.",
            INTERCEPTOR_SHIP_ID,
            1_000,
            &[
                (ItemRoleType::Torpedo, 2),
                (ItemRoleType::PdcRound, 200),
                (ItemRoleType::HullPlate, 10),
            ],
            "Some ships carry a price on their hull. Hunters collect it.",
        ),
        role(
            "military",
            "Military",
            "A patrol picket with one nose point-defense mount on light plate: the weakest \
             start. The Line Warship class is its later goal.",
            PATROL_PICKET_SHIP_ID,
            1_600,
            &[(ItemRoleType::PdcRound, 250), (ItemRoleType::HullPlate, 10)],
            "Patrol crews start on small pickets and earn larger hulls. Few reach a warship.",
        ),
        role(
            "pirate",
            "Pirate",
            "A raider with a nose railgun and two point-defense mounts. Attack cargo ships \
             and take their holds.",
            RAIDER_SHIP_ID,
            1_200,
            &[
                (ItemRoleType::RailSlug, 15),
                (ItemRoleType::PdcRound, 200),
                (ItemRoleType::HullPlate, 10),
            ],
            "Past the patrol routes, a full hold is a target. Raiders live on what they take.",
        ),
    ]
}

/// One role: `hull` with a kit of `credits` and `stock`.
fn role(
    id: &str,
    name: &str,
    about: &str,
    hull: &str,
    credits: u32,
    stock: &[(ItemRoleType, u32)],
    lore: &str,
) -> StartRole {
    StartRole {
        id: id.into(),
        name: name.to_string(),
        about: about.to_string(),
        hull: hull.into(),
        kit: StartKit {
            credits,
            stock: ShipInventoryStock::new(stock.iter().map(|&(item, count)| (item.into(), count))),
        },
        lore: lore.to_string(),
    }
}
