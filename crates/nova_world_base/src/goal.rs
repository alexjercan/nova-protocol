//! What derives the start role's goal progress while the open world streams,
//! and stamps the goal met.
//!
//! A role's goal (`RoleGoal`, content on the `StartRole`) is a predicate over
//! facts the world save already carries: the jobs the player handed in
//! ([`Jobs::completed`]), read by kind and by the civilization ring of the
//! posting station's cell ([`ring_of`]); the player's live hull, read as
//! its sections' recipe value at ask ([`hull_price_cr`]); and the stations
//! the player owns with what they store, read live
//! ([`OwnedStationMarker`] + [`StationStorage`]) and off the frozen ledger
//! (`FrozenStation::owned` + `::storage`), so a yard still counts while its
//! cell is off the window; and the escorts in the player's fleet
//! ([`PlayerFleetMarker`]), which fly with the player and are never frozen
//! with a cell; and the player's [`CivilizationStances`], for the
//! civilizations the player's events made Enemy and the hulls taken while
//! they stand so. [`track_role_goal`] derives [`GoalProgress`]
//! from them every frame for the Map's chart view, and stamps
//! [`GoalRecord`] with the world clock the first frame the progress is met.
//! The record is the one saved fact; the progress is never saved. Both
//! clear with the chart (`chart::clear_chart`) and the record comes back on
//! a Load.
//!
//! A met goal is an optional milestone, not an ending: the record is saved
//! and shown only, and the world does not change at thaw.

use std::collections::BTreeSet;

use bevy::prelude::*;
use nova_events::prelude::Meters;
use nova_gameplay::prelude::{
    CivilizationStances, GameItems, GoalLine, GoalProgress, GoalRecord, Job, JobType, Jobs,
    OwnedStationMarker, PlayerFleetMarker, PlayerSpaceshipMarker, StationStorage,
};
use nova_scenario::prelude::{
    resolve_ship_design, GameShipDesigns, GameStartRoles, OpenWorldSession, RoleGoal, RoleGoalType,
    ShipDesignId, ShipDesignSource,
};
use nova_ship::prelude::{hull_price_cr, GameSections, SectionBuildConfig, SectionConfig};
use nova_world::prelude::{FrozenBodyType, FrozenSectors, WorldClock, WorldConfig};

use crate::{civilizations::ring_of, ship_parts::ShipPartFamilyType, NovaLayeredWorld};

/// The saved facts one evaluation of a goal reads.
pub(crate) struct GoalFacts<'a> {
    /// The jobs the player handed in, in hand-in order.
    pub completed: &'a [Job],
    /// The streamed world's cell edge, for the ring of a poster's cell.
    pub sector_edge: Meters,
    /// The player's live sections' recipe value at ask.
    pub hull_value_cr: u64,
    /// The goal hull's recipe value at ask. `None` for a goal that is not a
    /// `Flagship`; a `Flagship` goal with none is an assembly fault.
    pub flagship_value_cr: Option<u64>,
    /// The storage of every station the player owns, live or frozen: one
    /// entry per owned station.
    pub owned_storage: &'a [&'a StationStorage],
    /// Every loaded section, whose recipes a yard stocks by part family.
    pub sections: &'a [SectionConfig],
    /// How many escorts fly in the player's fleet.
    pub escorts: u64,
    /// How many civilizations the player's events made Enemy, and the most
    /// intact hulls taken from one of them while it stands so.
    pub outlaw: (u64, u64),
}

/// How many part families `storage` can build one loaded section of: a
/// family counts when some section of it has a recipe with parts and the
/// store holds every part at its count. A recipe with no parts stocks
/// nothing, so no family comes free.
pub(crate) fn families_stocked(storage: &StationStorage, sections: &[SectionConfig]) -> u64 {
    let stocked: BTreeSet<ShipPartFamilyType> = sections
        .iter()
        .filter(|section| !section.base.build.parts.is_empty())
        .filter(|section| {
            section
                .base
                .build
                .parts
                .iter()
                .all(|(item, count)| storage.count(item) >= *count)
        })
        .filter_map(|section| ShipPartFamilyType::of(&section.kind))
        .collect();
    stocked.len() as u64
}

/// The progress of `goal` over `facts`: one `have / need` line per count
/// the goal asks for, labelled as the Map prints them.
///
/// A trade job is a Procure or an Escort; a kill is a Neutralize. A job
/// counts toward the stations and rings of its kind only, by its poster's id
/// and its poster's cell. A yard's families are the best single owned
/// station's ([`families_stocked`]): two half-stocked stations do not add up.
///
/// # Panics
///
/// A `Flagship` goal with no `flagship_value_cr`: the caller resolves the
/// goal hull before it evaluates.
pub(crate) fn goal_progress(goal: &RoleGoal, facts: &GoalFacts<'_>) -> GoalProgress {
    let counted = |kept: fn(&JobType) -> bool| {
        let jobs: Vec<&Job> = facts
            .completed
            .iter()
            .filter(|job| kept(&job.kind))
            .collect();
        let stations: BTreeSet<_> = jobs.iter().map(|job| job.posted_by()).collect();
        let rings: BTreeSet<u32> = jobs
            .iter()
            .map(|job| ring_of(job.posted_by().origin, facts.sector_edge))
            .collect();
        (jobs.len() as u64, stations.len() as u64, rings.len() as u64)
    };
    let lines = match &goal.kind {
        RoleGoalType::StationNetwork {
            jobs,
            stations,
            rings,
        } => {
            let (have_jobs, have_stations, have_rings) =
                counted(|kind| matches!(kind, JobType::Procure { .. } | JobType::Escort { .. }));
            vec![
                GoalLine::new("Trade jobs handed in", have_jobs, u64::from(*jobs)),
                GoalLine::new("Stations served", have_stations, u64::from(*stations)),
                GoalLine::new("Rings worked", have_rings, u64::from(*rings)),
            ]
        }
        RoleGoalType::KillList { hulls, rings } => {
            let (have_hulls, _, have_rings) =
                counted(|kind| matches!(kind, JobType::Neutralize { .. }));
            vec![
                GoalLine::new("Hulls neutralized", have_hulls, u64::from(*hulls)),
                GoalLine::new("Rings hunted", have_rings, u64::from(*rings)),
            ]
        }
        RoleGoalType::Flagship { hull } => {
            let need = facts.flagship_value_cr.unwrap_or_else(|| {
                panic!("goal_progress: the flagship hull '{hull}' was not valued")
            });
            vec![GoalLine::new("Hull value, cr", facts.hull_value_cr, need)]
        }
        RoleGoalType::OwnYard => {
            let families = facts
                .owned_storage
                .iter()
                .map(|storage| families_stocked(storage, facts.sections))
                .max()
                .unwrap_or(0);
            vec![
                GoalLine::new("Stations owned", facts.owned_storage.len() as u64, 1),
                GoalLine::new(
                    "Part families stocked",
                    families,
                    ShipPartFamilyType::ALL.len() as u64,
                ),
            ]
        }
        RoleGoalType::MiningEmpire { stations, escorts } => vec![
            GoalLine::new(
                "Stations owned",
                facts.owned_storage.len() as u64,
                u64::from(*stations),
            ),
            GoalLine::new("Escorts in fleet", facts.escorts, u64::from(*escorts)),
        ],
        RoleGoalType::Outlaw { hulls } => vec![
            GoalLine::new("Civilizations turned Enemy", facts.outlaw.0, 1),
            GoalLine::new("Hulls taken while Enemy", facts.outlaw.1, u64::from(*hulls)),
        ],
    };
    GoalProgress {
        title: goal.title.clone(),
        lines,
    }
}

/// Derive the session role's goal progress from the completed jobs, the
/// player's hull, the player's owned stations, the player's fleet and the
/// player's stances every frame, and stamp the record once when it is met.
///
/// No session, a role the catalog does not hold or a role with no goal: the
/// progress is empty and the record is left as it is. A `Flagship` goal
/// values its hull once per hull id, off the loaded catalog. An `OwnYard` or
/// `MiningEmpire` goal reads every owned station, live and frozen; the yard
/// reads them against the loaded sections' recipes.
///
/// # Panics
///
/// A `Flagship` goal with no [`GameShipDesigns`] or [`GameSections`] on
/// hand, or naming a design the catalog does not hold, or an `OwnYard` goal
/// with no [`GameSections`]: the lint refuses the id, so this is an assembly
/// fault.
#[expect(
    clippy::too_many_arguments,
    reason = "one system reading the session, the roles, the jobs, the world's edge, the catalog, the clock, the player's hull, the owned stations, the fleet and the stances"
)]
pub(crate) fn track_role_goal(
    session: Option<Res<OpenWorldSession>>,
    roles: Res<GameStartRoles>,
    jobs: Res<Jobs>,
    config: Res<WorldConfig<NovaLayeredWorld>>,
    items: Res<GameItems>,
    designs: Option<Res<GameShipDesigns>>,
    sections: Option<Res<GameSections>>,
    clock: Res<WorldClock>,
    player: Query<&Children, With<PlayerSpaceshipMarker>>,
    q_sections: Query<&SectionBuildConfig>,
    owned: Query<&StationStorage, With<OwnedStationMarker>>,
    frozen: Res<FrozenSectors>,
    (fleet, stances): (Query<(), With<PlayerFleetMarker>>, Res<CivilizationStances>),
    mut progress: ResMut<GoalProgress>,
    mut record: ResMut<GoalRecord>,
    mut flagship: Local<Option<(ShipDesignId, u64)>>,
) {
    let goal = session
        .as_deref()
        .and_then(|session| roles.get_role(&session.role))
        .map(|role| &role.goal);
    let Some(goal) = goal else {
        if !progress.is_empty() {
            *progress = GoalProgress::default();
        }
        return;
    };
    let hull_value_cr = player.single().map_or(0, |children| {
        hull_price_cr(
            &items,
            q_sections
                .iter_many(children)
                .map(|config| &config.0.base.build),
        )
    });
    let flagship_value_cr = match &goal.kind {
        RoleGoalType::Flagship { hull } => {
            let (Some(designs), Some(sections)) = (designs.as_ref(), sections.as_ref()) else {
                panic!("track_role_goal: a Flagship goal needs the loaded ship catalog");
            };
            // A mod install re-publishes the catalog without a restart, so
            // the cached value dies with the hull id or the catalog it came
            // from.
            let catalog_changed = designs.is_changed() || sections.is_changed();
            if catalog_changed || flagship.as_ref().is_none_or(|(valued, _)| valued != hull) {
                assert!(
                    designs.get_design(hull).is_some(),
                    "track_role_goal: the flagship hull '{hull}' is no loaded ship design"
                );
                let (resolved, _) = resolve_ship_design(
                    &ShipDesignSource::prototype(hull.clone()),
                    designs,
                    sections,
                );
                let value = hull_price_cr(
                    &items,
                    resolved
                        .sections
                        .iter()
                        .map(|section| &section.config.base.build),
                );
                *flagship = Some((hull.clone(), value));
            }
            flagship.as_ref().map(|(_, value)| *value)
        }
        RoleGoalType::StationNetwork { .. }
        | RoleGoalType::KillList { .. }
        | RoleGoalType::OwnYard
        | RoleGoalType::MiningEmpire { .. }
        | RoleGoalType::Outlaw { .. } => None,
    };
    let owned_storage: Vec<&StationStorage> = match &goal.kind {
        RoleGoalType::OwnYard | RoleGoalType::MiningEmpire { .. } => {
            let frozen_owned = frozen
                .iter()
                .flat_map(|(_, sector)| sector.bodies())
                .filter_map(|body| match body.body() {
                    FrozenBodyType::Station(station) if station.owned() => Some(station.storage()),
                    _ => None,
                });
            owned.iter().chain(frozen_owned).collect()
        }
        RoleGoalType::StationNetwork { .. }
        | RoleGoalType::KillList { .. }
        | RoleGoalType::Flagship { .. }
        | RoleGoalType::Outlaw { .. } => Vec::new(),
    };
    let catalog: &[SectionConfig] = match &goal.kind {
        RoleGoalType::OwnYard => {
            let Some(sections) = sections.as_deref() else {
                panic!("track_role_goal: an OwnYard goal needs the loaded section catalog");
            };
            &sections.0
        }
        RoleGoalType::StationNetwork { .. }
        | RoleGoalType::KillList { .. }
        | RoleGoalType::Flagship { .. }
        | RoleGoalType::MiningEmpire { .. }
        | RoleGoalType::Outlaw { .. } => &[],
    };
    let next = goal_progress(
        goal,
        &GoalFacts {
            completed: jobs.completed(),
            sector_edge: config.sector_edge,
            hull_value_cr,
            flagship_value_cr,
            owned_storage: &owned_storage,
            sections: catalog,
            escorts: fleet.iter().count() as u64,
            outlaw: stances.outlaw_progress(),
        },
    );
    if next.is_met() && !record.is_met() {
        record.met_at = Some(clock.play_seconds);
        info!(
            "nova_world_base: the charter '{}' is met at {} s of play",
            next.title, clock.play_seconds
        );
    }
    if *progress != next {
        *progress = next;
    }
}

#[cfg(test)]
mod tests {
    use nova_events::prelude::{GeneratedBodyId, MetersPerSecond, SectorCoord};
    use nova_gameplay::prelude::{BuildRecipe, ItemCategoryType, ItemDesign, ItemDesignId, JobId};
    use nova_ship::prelude::{
        BaseSectionConfig, CargoIntakeSectionConfig, MiningSectionConfig, SectionKind,
    };

    use super::*;

    const EDGE: Meters = Meters(32_000.0);

    /// A job of `kind` posted by the station in cell `(x, 0, 0)`, slot
    /// `slot`.
    fn handed_in(x: i32, slot: u32, kind: JobType) -> Job {
        let station = GeneratedBodyId::new(SectorCoord::new(x, 0, 0), "cluster_station", 0);
        Job {
            id: JobId {
                station: station.clone(),
                slot,
                posting: 0,
            },
            kind,
            target: station,
            reward_cr: 100,
        }
    }

    fn procure() -> JobType {
        JobType::Procure {
            item: "Rations".into(),
            count: 4,
        }
    }

    fn kill(x: i32, index: u32) -> JobType {
        JobType::Neutralize {
            hull: GeneratedBodyId::new(SectorCoord::new(x, 0, 0), "cluster_hull", index),
            hull_name: format!("Harrow {index}"),
        }
    }

    fn facts(completed: &[Job]) -> GoalFacts<'_> {
        GoalFacts {
            completed,
            sector_edge: EDGE,
            hull_value_cr: 0,
            flagship_value_cr: None,
            owned_storage: &[],
            sections: &[],
            escorts: 0,
            outlaw: (0, 0),
        }
    }

    fn network() -> RoleGoal {
        RoleGoal {
            title: "A station network".to_string(),
            kind: RoleGoalType::StationNetwork {
                jobs: 8,
                stations: 4,
                rings: 2,
            },
        }
    }

    fn haves(progress: &GoalProgress) -> Vec<u64> {
        progress.lines.iter().map(|line| line.have).collect()
    }

    /// Eight procurements at one ring-0 station read 8 / 8 jobs, 1 / 4
    /// stations and 1 / 2 rings; four posters in ring 0 read one ring; a
    /// neutralize adds nothing; two more posters in ring 1 meet the goal.
    #[test]
    fn a_station_network_counts_trade_hand_ins_distinct_posters_and_distinct_rings_and_ignores_kills(
    ) {
        let goal = network();
        let one_station: Vec<Job> = (0..8).map(|slot| handed_in(1, slot, procure())).collect();
        let progress = goal_progress(&goal, &facts(&one_station));
        assert_eq!(progress.title, "A station network");
        assert_eq!(haves(&progress), [8, 1, 1]);
        assert_eq!(
            progress
                .lines
                .iter()
                .map(|line| line.need)
                .collect::<Vec<_>>(),
            [8, 4, 2]
        );
        assert!(!progress.is_met());

        let one_ring: Vec<Job> = (0..8)
            .map(|slot: u32| handed_in(i32::try_from(slot % 4).unwrap(), slot, procure()))
            .chain([handed_in(2, 9, kill(2, 0))])
            .collect();
        let progress = goal_progress(&goal, &facts(&one_ring));
        assert_eq!(haves(&progress), [8, 4, 1]);
        assert!(!progress.is_met());

        let two_rings: Vec<Job> = (0..6)
            .map(|slot: u32| handed_in(i32::try_from(slot % 2).unwrap(), slot, procure()))
            .chain([
                handed_in(
                    9,
                    0,
                    JobType::Escort {
                        hauler: GeneratedBodyId::new(SectorCoord::new(9, 0, 0), "h", 0),
                        cargo_cr: 500,
                    },
                ),
                handed_in(10, 0, procure()),
            ])
            .collect();
        let progress = goal_progress(&goal, &facts(&two_rings));
        assert_eq!(haves(&progress), [8, 4, 2]);
        assert!(progress.is_met());
    }

    /// Six neutralizes posted from two rings meet the list; five, or six in
    /// one ring, do not; a distress call adds nothing.
    #[test]
    fn a_kill_list_counts_neutralize_hand_ins_and_their_rings_and_ignores_distress() {
        let goal = RoleGoal {
            title: "A kill list".to_string(),
            kind: RoleGoalType::KillList { hulls: 6, rings: 2 },
        };
        let one_ring: Vec<Job> = (0..6)
            .map(|index| handed_in(1, index, kill(1, index)))
            .chain([handed_in(
                9,
                7,
                JobType::Distress {
                    caller: GeneratedBodyId::new(SectorCoord::new(9, 0, 0), "c", 0),
                    genuine: true,
                },
            )])
            .collect();
        let progress = goal_progress(&goal, &facts(&one_ring));
        assert_eq!(haves(&progress), [6, 1]);
        assert!(!progress.is_met());

        let five: Vec<Job> = (0..5)
            .map(|index| handed_in(if index < 3 { 1 } else { 9 }, index, kill(1, index)))
            .collect();
        assert_eq!(haves(&goal_progress(&goal, &facts(&five))), [5, 2]);

        let six: Vec<Job> = (0..6)
            .map(|index| handed_in(if index < 3 { 1 } else { 9 }, index, kill(1, index)))
            .collect();
        let progress = goal_progress(&goal, &facts(&six));
        assert_eq!(haves(&progress), [6, 2]);
        assert!(progress.is_met());
    }

    /// A section of `kind` whose recipe takes `parts`.
    fn section(kind: SectionKind, parts: &[(&str, u32)]) -> SectionConfig {
        SectionConfig {
            base: BaseSectionConfig {
                id: format!("{kind:?}"),
                build: BuildRecipe {
                    credits: 10,
                    parts: parts
                        .iter()
                        .map(|&(item, count)| (ItemDesignId::from(item), count))
                        .collect(),
                    strip_yield: 0.5,
                },
                ..default()
            },
            kind,
        }
    }

    /// One section per family: six take parts, the mining beam takes none.
    fn catalog() -> Vec<SectionConfig> {
        vec![
            section(SectionKind::Hull(default()), &[("SectionPanel", 2)]),
            section(SectionKind::Controller(default()), &[("FlightCore", 1)]),
            section(SectionKind::Thruster(default()), &[("DriveBell", 1)]),
            section(
                SectionKind::Turret(default()),
                &[("TurretBody", 1), ("TurretBarrel", 1)],
            ),
            section(
                SectionKind::CargoIntake(CargoIntakeSectionConfig {
                    render_mesh: default(),
                    render_mesh_transform: None,
                    canister_mesh: default(),
                    door_sound: default(),
                    eject_sound: default(),
                    take_sound: default(),
                    detection_range: Meters(50.0),
                    capture_gap: Meters(5.0),
                    aperture_width: Meters(4.0),
                    aperture_height: Meters(4.0),
                    eject_speed: MetersPerSecond(5.0),
                }),
                &[("FrameSpar", 1)],
            ),
            section(SectionKind::Docking(default()), &[("DockingCollar", 1)]),
            section(
                SectionKind::Mining(MiningSectionConfig {
                    render_mesh: default(),
                    render_mesh_transform: None,
                    pulse_sound: default(),
                    refusal_sound: default(),
                    door_open_sound: default(),
                    door_close_sound: default(),
                    reach: Meters(100.0),
                    pulse_interval_seconds: 1.0,
                    carve_radius_cells: 1.5,
                }),
                &[],
            ),
        ]
    }

    fn items() -> GameItems {
        GameItems::new(
            [
                "SectionPanel",
                "FlightCore",
                "DriveBell",
                "TurretBody",
                "TurretBarrel",
                "FrameSpar",
                "DockingCollar",
                "CapacitorBank",
            ]
            .into_iter()
            .map(|id| ItemDesign {
                id: id.into(),
                name: id.to_string(),
                about: String::new(),
                category: ItemCategoryType::Parts,
                mass_g: 1_000,
                ask_cr: 10,
                bid_cr: 5,
            }),
        )
    }

    fn storage(stock: &[(&str, u32)]) -> StationStorage {
        let items = items();
        let mut storage = StationStorage::new(1_000_000);
        for &(item, count) in stock {
            storage.add(&items, &item.into(), count);
        }
        storage
    }

    fn yard() -> RoleGoal {
        RoleGoal {
            title: "A yard of your own".to_string(),
            kind: RoleGoalType::OwnYard,
        }
    }

    fn yard_facts<'a>(
        owned_storage: &'a [&'a StationStorage],
        sections: &'a [SectionConfig],
    ) -> GoalFacts<'a> {
        GoalFacts {
            completed: &[],
            sector_edge: EDGE,
            hull_value_cr: 0,
            flagship_value_cr: None,
            owned_storage,
            sections,
            escorts: 0,
            outlaw: (0, 0),
        }
    }

    /// No station and no escort reads 0 / 2 twice; two stations with one
    /// escort stand; two of each is met; a third station over the count
    /// still reads met.
    #[test]
    fn a_mining_empire_counts_owned_stations_and_fleet_escorts_and_meets_at_both_counts() {
        let goal = RoleGoal {
            title: "A mining empire".to_string(),
            kind: RoleGoalType::MiningEmpire {
                stations: 2,
                escorts: 2,
            },
        };
        let empire = |owned: &[&StationStorage], escorts: u64| {
            goal_progress(
                &goal,
                &GoalFacts {
                    escorts,
                    ..yard_facts(owned, &[])
                },
            )
        };
        let progress = empire(&[], 0);
        assert_eq!(progress.title, "A mining empire");
        assert_eq!(haves(&progress), [0, 0]);
        assert_eq!(
            progress
                .lines
                .iter()
                .map(|line| line.need)
                .collect::<Vec<_>>(),
            [2, 2]
        );
        assert!(!progress.is_met());

        let a = storage(&[]);
        let b = storage(&[("SectionPanel", 1)]);
        let progress = empire(&[&a, &b], 1);
        assert_eq!(haves(&progress), [2, 1]);
        assert!(!progress.is_met());

        assert!(empire(&[&a, &b], 2).is_met());
        let progress = empire(&[&a, &b, &a], 2);
        assert_eq!(haves(&progress), [3, 2]);
        assert!(progress.is_met());
    }

    /// No owned station reads 0 / 1 and 0 / 7; a station one part short of
    /// the turret recipe reads 1 / 1 and 5 / 7 (the mining beam's empty
    /// recipe stocks nothing); two stations each stocking part of the
    /// catalog do not add up; one station stocking a part-bearing section
    /// of every family, with a mining recipe to stock, is met.
    #[test]
    fn a_yard_counts_the_best_owned_station_s_stocked_families_and_meets_at_all_seven() {
        let goal = yard();
        let catalog = catalog();
        let progress = goal_progress(&goal, &yard_facts(&[], &catalog));
        assert_eq!(progress.title, "A yard of your own");
        assert_eq!(haves(&progress), [0, 0]);
        assert_eq!(
            progress
                .lines
                .iter()
                .map(|line| line.need)
                .collect::<Vec<_>>(),
            [1, 7]
        );
        assert!(!progress.is_met());

        let short = storage(&[
            ("SectionPanel", 2),
            ("FlightCore", 1),
            ("DriveBell", 1),
            ("TurretBody", 1),
            ("FrameSpar", 1),
            ("DockingCollar", 1),
        ]);
        let progress = goal_progress(&goal, &yard_facts(&[&short], &catalog));
        assert_eq!(haves(&progress), [1, 5]);
        assert!(!progress.is_met());

        let barrel = storage(&[("TurretBarrel", 1), ("TurretBody", 1)]);
        let progress = goal_progress(&goal, &yard_facts(&[&short, &barrel], &catalog));
        assert_eq!(haves(&progress), [2, 5], "two stations do not add up");
        assert!(!progress.is_met());

        let mut stocked = catalog.clone();
        stocked[6].base.build.parts = [(ItemDesignId::from("CapacitorBank"), 2)].into();
        let full = storage(&[
            ("SectionPanel", 2),
            ("FlightCore", 1),
            ("DriveBell", 1),
            ("TurretBody", 1),
            ("TurretBarrel", 1),
            ("FrameSpar", 1),
            ("DockingCollar", 1),
            ("CapacitorBank", 2),
        ]);
        let progress = goal_progress(&goal, &yard_facts(&[&short, &full], &stocked));
        assert_eq!(haves(&progress), [2, 7]);
        assert!(progress.is_met());
    }

    /// An outlaw's charter reads the civilizations the player's events made
    /// Enemy against one and the hulls taken while Enemy against its count:
    /// a seeded Enemy and the flipping kill count for nothing, six hulls
    /// while Enemy meet it.
    #[test]
    fn an_outlaw_counts_civilizations_turned_enemy_and_hulls_taken_while_enemy() {
        let goal = RoleGoal {
            title: "An outlaw's name".to_string(),
            kind: RoleGoalType::Outlaw { hulls: 6 },
        };
        let clean = goal_progress(&goal, &facts(&[]));
        assert_eq!(haves(&clean), [0, 0]);
        assert!(!clean.is_met());
        let outlawed = GoalFacts {
            outlaw: (1, 5),
            ..facts(&[])
        };
        let progress = goal_progress(&goal, &outlawed);
        assert_eq!(haves(&progress), [1, 5]);
        assert!(!progress.is_met());
        let met = GoalFacts {
            outlaw: (1, 6),
            ..facts(&[])
        };
        assert!(goal_progress(&goal, &met).is_met());
    }

    /// The flagship line reads the hull's value against the design's: one
    /// credit under stands, equal is met.
    #[test]
    fn a_flagship_reads_met_at_the_design_value_and_open_one_credit_under_it() {
        let goal = RoleGoal {
            title: "The Line Warship".to_string(),
            kind: RoleGoalType::Flagship {
                hull: "line_warship".into(),
            },
        };
        let short = GoalFacts {
            completed: &[],
            sector_edge: EDGE,
            hull_value_cr: 9_999,
            flagship_value_cr: Some(10_000),
            owned_storage: &[],
            sections: &[],
            escorts: 0,
            outlaw: (0, 0),
        };
        let progress = goal_progress(&goal, &short);
        assert_eq!(haves(&progress), [9_999]);
        assert!(!progress.is_met());
        let equal = GoalFacts {
            hull_value_cr: 10_000,
            ..short
        };
        assert!(goal_progress(&goal, &equal).is_met());
    }
}
