//! Built-in START ROLE content: the six New Game starts.
//!
//! `content gen` serializes this builder into
//! `assets/base/roles/base.content.ron`. Each kit is worth 3,000 cr at ask:
//! the credits plus the items at their base `ask_cr`.
//!
//! Goals are the roles' finite milestones over saved facts (decision 13 of
//! `tasks/20261012-095526`). The transport and bounty hunter counts rest on
//! the trade and combat job mixes (a trade board shows about two Procure or
//! Escort offers, a combat board about one Neutralize) and on the first ring
//! boundary at 240 km, one trip out of the start ring. The scavenger's yard
//! takes no count: one owned station stocked for every part family. The
//! military bar is the Line Warship's recipe value, 64,300 cr, which
//! generated Armored hulls reach about 2,000 km out. The industrial empire
//! is two station buys and the full fleet (`FLEET_SIDES`), with no stock
//! clause: a bought station keeps its targets and restocks on every thaw,
//! so such a clause would read met from the buy on. The pirate's outlaw
//! count is twice the offence step: the stance graph turns a Neutral
//! civilization Enemy at three neutralized hulls within an hour of play,
//! and six more of its hulls while it stands Enemy is twice that. A charter
//! is a milestone, not an ending: the world plays on past it.

use nova_gameplay::prelude::{ItemRoleType, JobMixType, RoleIconType, ShipInventoryStock};
use nova_scenario::prelude::{RoleGoal, RoleGoalType, StartKit, StartRole};

use super::ships::{
    INTERCEPTOR_SHIP_ID, LIGHT_FREIGHTER_SHIP_ID, LINE_WARSHIP_SHIP_ID, MINING_TUG_SHIP_ID,
    PATROL_PICKET_SHIP_ID, RAIDER_SHIP_ID, SALVAGE_HAULER_SHIP_ID,
};

/// Every shipped start role, in the order a picker lists them.
pub(crate) fn start_role_catalog() -> Vec<StartRole> {
    vec![
        role(
            "industrial",
            "Industrial",
            RoleIconType::Drill,
            "A mining tug with a bow mining beam and a cargo intake, unarmed. Cut ore from \
             rocks and sell it.",
            MINING_TUG_SHIP_ID,
            2_200,
            &[(ItemRoleType::HullPlate, 15), (ItemRoleType::Rations, 25)],
            JobMixType::Trade,
            RoleGoal {
                title: "A mining empire".to_string(),
                kind: RoleGoalType::MiningEmpire {
                    stations: 2,
                    escorts: 2,
                },
            },
            "Mining crews work the rock fields for whoever buys the ore. The tug is old, but \
             it pays.",
        ),
        role(
            "scavenger",
            "Scavenger",
            RoleIconType::Claw,
            "A salvage hauler with the largest start hold, a cargo intake, a bow mining \
             beam and one point-defense mount. Pick wrecks clean and sell what you take.",
            SALVAGE_HAULER_SHIP_ID,
            1_800,
            &[(ItemRoleType::PdcRound, 200), (ItemRoleType::HullPlate, 10)],
            JobMixType::Salvage,
            RoleGoal {
                title: "A yard of your own".to_string(),
                kind: RoleGoalType::OwnYard,
            },
            "Every fight leaves wrecks behind. Scavengers take what still has a price.",
        ),
        role(
            "transport",
            "Transport",
            RoleIconType::Container,
            "A light freighter with freight cubes, a tank pod and no weapons. Buy cargo at \
             one station and sell it at another.",
            LIGHT_FREIGHTER_SHIP_ID,
            2_000,
            &[(ItemRoleType::Rations, 50), (ItemRoleType::WaterIce, 50)],
            JobMixType::Trade,
            RoleGoal {
                title: "A station network".to_string(),
                kind: RoleGoalType::StationNetwork {
                    jobs: 8,
                    stations: 4,
                    rings: 2,
                },
            },
            "Stations need food, water and parts they cannot make. Haulers carry them \
             between stations.",
        ),
        role(
            "bounty_hunter",
            "Bounty Hunter",
            RoleIconType::Star,
            "An interceptor with two point-defense mounts. Hunt armed ships for the price \
             on them.",
            INTERCEPTOR_SHIP_ID,
            1_600,
            &[(ItemRoleType::PdcRound, 250), (ItemRoleType::HullPlate, 10)],
            JobMixType::Combat,
            RoleGoal {
                title: "A kill list".to_string(),
                kind: RoleGoalType::KillList { hulls: 6, rings: 2 },
            },
            "Some ships carry a price on their hull. Hunters collect it.",
        ),
        role(
            "military",
            "Military",
            RoleIconType::Shield,
            "A patrol picket with two point-defense mounts and a torpedo bay on reinforced \
             plate. The Line Warship class is its later goal.",
            PATROL_PICKET_SHIP_ID,
            1_000,
            &[
                (ItemRoleType::Torpedo, 2),
                (ItemRoleType::PdcRound, 200),
                (ItemRoleType::HullPlate, 10),
            ],
            JobMixType::Combat,
            RoleGoal {
                title: "The Line Warship".to_string(),
                kind: RoleGoalType::Flagship {
                    hull: LINE_WARSHIP_SHIP_ID.into(),
                },
            },
            "Patrol crews start on small pickets and earn larger hulls. Few reach a warship.",
        ),
        role(
            "pirate",
            "Pirate",
            RoleIconType::Blades,
            "A raider with two point-defense mounts. Attack cargo ships and take their \
             holds.",
            RAIDER_SHIP_ID,
            1_800,
            &[(ItemRoleType::PdcRound, 200), (ItemRoleType::HullPlate, 10)],
            JobMixType::Combat,
            RoleGoal {
                title: "An outlaw's name".to_string(),
                kind: RoleGoalType::Outlaw { hulls: 6 },
            },
            "Past the patrol routes, a full hold is a target. Raiders live on what they take.",
        ),
    ]
}

/// One role drawn as `icon`: `hull` with a kit of `credits` and `stock`,
/// whose boards lean to `jobs`, with its charter `goal`.
#[expect(
    clippy::too_many_arguments,
    reason = "one row of the role table; every field is authored here"
)]
fn role(
    id: &str,
    name: &str,
    icon: RoleIconType,
    about: &str,
    hull: &str,
    credits: u32,
    stock: &[(ItemRoleType, u32)],
    jobs: JobMixType,
    goal: RoleGoal,
    lore: &str,
) -> StartRole {
    StartRole {
        id: id.into(),
        name: name.to_string(),
        icon,
        about: about.to_string(),
        hull: hull.into(),
        kit: StartKit {
            credits,
            stock: ShipInventoryStock::new(stock.iter().map(|&(item, count)| (item.into(), count))),
        },
        jobs,
        goal,
        lore: lore.to_string(),
    }
}
