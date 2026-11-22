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
//! Goals are the roles' finite chains of steps over saved facts. Every
//! role's chain opens with its own count and closes with the same step,
//! one owned station. Each count is three: three hand-ins of the kinds the
//! role's board mix draws most, or for the pirate three trading hulls it
//! neutralized, which no board posts. Each step's hand-in pays the same on
//! every role ([`FIRST_STEP_REWARD_CR`], [`STATION_STEP_REWARD_CR`]), so
//! the roles stay level with each other. A charter is a milestone, not an
//! ending: the world plays on past it.

use nova_gameplay::prelude::{
    ItemDesignId, ItemRoleType, JobMixType, RoleIconType, ShipInventoryStock,
};
use nova_scenario::prelude::{RoleGoal, RoleGoalStep, RoleGoalType, StartKit, StartRole};

use super::{
    items::{REPAIR_KIT, SECTION_PANEL},
    ships::{
        INTERCEPTOR_SHIP_ID, LIGHT_FREIGHTER_SHIP_ID, MINING_TUG_SHIP_ID, PATROL_PICKET_SHIP_ID,
        RAIDER_SHIP_ID, SALVAGE_HAULER_SHIP_ID,
    },
};

/// What the hand-in of a role's own step pays: about three median job
/// rewards, under the first 15 minutes' median gain and above every start
/// kit, so it reads as a step up without skipping the early game.
const FIRST_STEP_REWARD_CR: u32 = 3_000;

/// What the hand-in of the owned-station step pays: about a fifth of a
/// median station at advancement 0, a refit after the big spend.
const STATION_STEP_REWARD_CR: u32 = 10_000;

/// A charter titled `title`: the role's own step of `kind`, then one owned
/// station.
fn charter(title: &str, kind: RoleGoalType) -> RoleGoal {
    RoleGoal {
        title: title.to_string(),
        steps: vec![
            RoleGoalStep {
                kind,
                reward_cr: FIRST_STEP_REWARD_CR,
            },
            RoleGoalStep {
                kind: RoleGoalType::StationsOwned { stations: 1 },
                reward_cr: STATION_STEP_REWARD_CR,
            },
        ],
    }
}

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
            charter("Ore to market", RoleGoalType::OreDeliveries { jobs: 3 }),
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
            charter("A salvage run", RoleGoalType::Recoveries { jobs: 3 }),
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
            charter("A freight line", RoleGoalType::CargoRuns { jobs: 3 }),
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
            charter("A kill list", RoleGoalType::KillList { hulls: 3 }),
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
            charter("Patrol record", RoleGoalType::Patrols { jobs: 3 }),
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
            charter("An outlaw's name", RoleGoalType::Outlaw { hulls: 3 }),
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
