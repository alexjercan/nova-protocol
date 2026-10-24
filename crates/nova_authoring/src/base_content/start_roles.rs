//! Built-in START ROLE content: the six New Game starts.
//!
//! `content gen` serializes this builder into
//! `assets/base/roles/base.content.ron`. Each kit is worth 3,000 cr at ask,
//! not counting PDC rounds: the credits plus the other items at their base
//! `ask_cr`. A PDC-armed hull also carries 1,000 PDC rounds (4,000 cr at ask)
//! in its shared hold, not per mount.
//!
//! Goals are the roles' finite milestones over saved facts. Every role's
//! progress opens with the same first line, one owned station, a rule in
//! code; the goal here is the flavour line after it. Each count rests on
//! the role's board mix: the trade mix draws Procure 4 of 11 and Procure or
//! Escort 8 of 11, the salvage mix Recover 5 of 10, the combat mix
//! Neutralize 4 of 10 and Distress or Escort 4 of 10, so six hand-ins of a
//! role's dominant kind is about one board cycle, and four distinct posting
//! stations is two trips out. The pirate's outlaw count is twice the
//! offence step: the stance graph turns a Neutral civilization Enemy at
//! three neutralized hulls within an hour of play, and six more of its hulls
//! while it stands Enemy is twice that. A charter is a milestone, not an
//! ending: the world plays on past it.

use nova_gameplay::prelude::{ItemRoleType, JobMixType, RoleIconType, ShipInventoryStock};
use nova_scenario::prelude::{RoleGoal, RoleGoalType, StartKit, StartRole};

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
            RoleIconType::Drill,
            "A mining tug with a bow mining beam and a cargo intake, unarmed. Cut ore from \
             rocks and sell it.",
            MINING_TUG_SHIP_ID,
            2_200,
            &[(ItemRoleType::HullPlate, 15), (ItemRoleType::Rations, 25)],
            JobMixType::Trade,
            RoleGoal {
                title: "Ore to market".to_string(),
                kind: RoleGoalType::OreDeliveries { jobs: 6 },
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
            &[
                (ItemRoleType::PdcRound, 1_000),
                (ItemRoleType::HullPlate, 10),
            ],
            JobMixType::Salvage,
            RoleGoal {
                title: "A salvage run".to_string(),
                kind: RoleGoalType::Recoveries { jobs: 6 },
            },
            "Every fight leaves wrecks behind. Scavengers take what still has a price.",
        ),
        role(
            "transport",
            "Transport",
            RoleIconType::Container,
            "A light freighter with a deep stack, a flank pod and no weapons. Buy cargo at \
             one station and sell it at another.",
            LIGHT_FREIGHTER_SHIP_ID,
            2_000,
            &[(ItemRoleType::Rations, 50), (ItemRoleType::WaterIce, 50)],
            JobMixType::Trade,
            RoleGoal {
                title: "A station network".to_string(),
                kind: RoleGoalType::StationNetwork { stations: 4 },
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
            &[
                (ItemRoleType::PdcRound, 1_000),
                (ItemRoleType::HullPlate, 10),
            ],
            JobMixType::Combat,
            RoleGoal {
                title: "A kill list".to_string(),
                kind: RoleGoalType::KillList { hulls: 6 },
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
            1_000,
            &[
                (ItemRoleType::Torpedo, 2),
                (ItemRoleType::PdcRound, 1_000),
                (ItemRoleType::HullPlate, 10),
            ],
            JobMixType::Combat,
            RoleGoal {
                title: "Patrol record".to_string(),
                kind: RoleGoalType::Patrols { jobs: 6 },
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
                (ItemRoleType::PdcRound, 1_000),
                (ItemRoleType::HullPlate, 10),
            ],
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
