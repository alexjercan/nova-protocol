//! Built-in START ROLE content: the six New Game starts.
//!
//! `content gen` serializes this builder into
//! `assets/base/roles/base.content.ron`. The credits follow what a role
//! must buy before it earns: 2,200 cr on the two unarmed hulls, 1,800 cr on
//! the three hulls that start with PDCs and no working income but a fight,
//! 1,500 cr on the salvage hauler, whose first wreck pays in goods. A
//! PDC-armed hull also carries 3,000 PDC rounds in its shared hold, not per
//! mount: 30 whole lots of 100. One start-band escort can empty a 2-PDC
//! hull's 2,000-round load, so 3,000 in the hold is two such fights or more
//! before the first station Buy.
//!
//! Every kit carries three repair kits, hull plates and six section
//! panels, so each start can patch its hull and rebuild one small lost
//! section before it can craft.
//!
//! Goals are the roles' finite milestones over saved facts. Every role's
//! progress opens with its own count and ends with the same last line, one
//! owned station, a rule in code. Each count is three: three hand-ins of
//! the kinds the role's board mix draws most, or for the pirate three
//! trading hulls it neutralized, which no board posts. A charter is a
//! milestone, not an ending: the world plays on past it.

use nova_gameplay::prelude::{
    ItemDesignId, ItemRoleType, JobMixType, RoleIconType, ShipInventoryStock,
};
use nova_scenario::prelude::{RoleGoal, RoleGoalType, StartKit, StartRole};

use super::{
    items::{REPAIR_KIT, SECTION_PANEL},
    ships::{
        INTERCEPTOR_SHIP_ID, LIGHT_FREIGHTER_SHIP_ID, MINING_TUG_SHIP_ID, PATROL_PICKET_SHIP_ID,
        RAIDER_SHIP_ID, SALVAGE_HAULER_SHIP_ID,
    },
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
            &[
                (ItemRoleType::HullPlate.id(), 15),
                (ItemRoleType::Rations.id(), 25),
                (SECTION_PANEL, 6),
                (REPAIR_KIT, 3),
            ],
            JobMixType::Mining,
            RoleGoal {
                title: "Ore to market".to_string(),
                kind: RoleGoalType::OreDeliveries { jobs: 3 },
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
            1_500,
            &[
                (ItemRoleType::PdcRound.id(), 3_000),
                (ItemRoleType::HullPlate.id(), 10),
                (SECTION_PANEL, 6),
                (REPAIR_KIT, 3),
            ],
            JobMixType::Salvage,
            RoleGoal {
                title: "A salvage run".to_string(),
                kind: RoleGoalType::Recoveries { jobs: 3 },
            },
            "Every fight leaves wrecks behind. Scavengers take what still has a price.",
        ),
        role(
            "transport",
            "Transport",
            RoleIconType::Container,
            "A light freighter with a box hold, two drive pods and no weapons. Buy cargo at \
             one station and sell it at another.",
            LIGHT_FREIGHTER_SHIP_ID,
            2_200,
            &[
                (ItemRoleType::Rations.id(), 50),
                (ItemRoleType::WaterIce.id(), 50),
                (ItemRoleType::HullPlate.id(), 10),
                (SECTION_PANEL, 6),
                (REPAIR_KIT, 3),
            ],
            JobMixType::Trade,
            RoleGoal {
                title: "A freight line".to_string(),
                kind: RoleGoalType::CargoRuns { jobs: 3 },
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
            1_800,
            &[
                (ItemRoleType::PdcRound.id(), 3_000),
                (ItemRoleType::HullPlate.id(), 10),
                (SECTION_PANEL, 6),
                (REPAIR_KIT, 3),
            ],
            JobMixType::Bounty,
            RoleGoal {
                title: "A kill list".to_string(),
                kind: RoleGoalType::KillList { hulls: 3 },
            },
            "Some ships carry a price on their hull. Hunters collect it.",
        ),
        role(
            "military",
            "Military",
            RoleIconType::Shield,
            "A patrol picket with two point-defense mounts and a torpedo bay on light \
             plate. Answer distress calls and escort haulers past the raiders.",
            PATROL_PICKET_SHIP_ID,
            1_800,
            &[
                (ItemRoleType::Torpedo.id(), 2),
                (ItemRoleType::PdcRound.id(), 3_000),
                (ItemRoleType::HullPlate.id(), 10),
                (SECTION_PANEL, 6),
                (REPAIR_KIT, 3),
            ],
            JobMixType::Patrol,
            RoleGoal {
                title: "Patrol record".to_string(),
                kind: RoleGoalType::Patrols { jobs: 3 },
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
            &[
                (ItemRoleType::PdcRound.id(), 3_000),
                (ItemRoleType::HullPlate.id(), 10),
                (SECTION_PANEL, 6),
                (REPAIR_KIT, 3),
            ],
            JobMixType::Raid,
            RoleGoal {
                title: "An outlaw's name".to_string(),
                kind: RoleGoalType::Outlaw { hulls: 3 },
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
    stock: &[(&str, u32)],
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
            stock: ShipInventoryStock::new(
                stock
                    .iter()
                    .map(|&(item, count)| (ItemDesignId::from(item), count)),
            ),
        },
        jobs,
        goal,
        lore: lore.to_string(),
    }
}
