//! What moves the player's [`CivilizationStances`] while the open world
//! streams, and what the moves reach.
//!
//! An offence is an intact generated hull the player's ship or a fleet
//! escort neutralized ([`record_offence`], on the hull's
//! [`NeutralizedMarker`] with the player as its [`LastAttacker`]); a merit
//! is a job handed in at a civilization's station ([`record_merit`], called
//! by the station pane at the hand-in); a provocation is the player's fire
//! on a Neutral or Ally body crossing the AI's retaliation threshold
//! ([`record_provocation`], on [`ShipProvoked`]). A step an offence makes
//! reaches the civilization's living neighbours once, by the seeded edge
//! ([`CivilizationField::edge`]): an Ally follows one step down, an Enemy
//! warms one step up, and nothing reaches further. A merit, a settlement
//! or the decay moves the one civilization alone.
//!
//! [`expire_stances`] lets the play clock pass over the table every frame,
//! and [`sync_civilization_sides`] writes each civilization's effective
//! stance as the [`Allegiance`] of every live generated body it owns, so
//! admission, the AI and the HUD keep reading the side alone. A station the
//! player owns, an escort in the player's fleet and a neutralized hull keep
//! the side they have: a bought station flies the player's, an escort the
//! player's, and a dead hull is merchandise.
//!
//! The fine a station of a neighbour takes to settle an Enemy
//! ([`fine_cr`]) is flat by that civilization's advancement, between
//! [`FINE_CR`]'s ends: a backwater forgives for about a start kit's third, a
//! core world for several.

use std::collections::BTreeMap;

use bevy::prelude::*;
use nova_gameplay::prelude::{
    Allegiance, CivilizationId, CivilizationStances, NeutralizedMarker, OwnedStationMarker,
    PlayerFleetMarker, PlayerSpaceshipMarker, StanceStep, StanceType, TraderMarket,
    ENEMY_DECAY_SECONDS, OFFENCE_WINDOW_SECONDS, PROVOKED_WINDOW_SECONDS,
};
use nova_ship::prelude::{LastAttacker, ShipProvoked};
use nova_world::prelude::WorldClock;

use crate::civilizations::CivilizationField;

/// The fine to settle with a civilization, in credits, at advancement 0 and
/// at advancement 1. Content: a backwater near 0.01 asks about 1,090 cr,
/// below a 3,000 cr start kit; a core world near 1 asks 10,000 cr.
pub const FINE_CR: (f32, f32) = (1_000.0, 10_000.0);

/// The stance the seed drew for `civilization` toward the player.
pub fn seeded_stance(civilization: &CivilizationId) -> StanceType {
    CivilizationField::new(civilization.world_seed)
        .civilization(civilization.node)
        .stance
}

/// The fine a station takes to settle the player with `civilization`:
/// [`FINE_CR`] read at its advancement.
pub fn fine_cr(civilization: &CivilizationId) -> u64 {
    let advancement = CivilizationField::new(civilization.world_seed)
        .civilization(civilization.node)
        .advancement;
    let (low, high) = FINE_CR;
    (low + (high - low) * advancement).round() as u64
}

/// Apply an offence against `civilization` at `now`: an outlaw hull is
/// counted first, so the kill that flips it is not one taken while Enemy;
/// then the offence is recorded and the step it made, if any, reaches the
/// neighbours. Every step made, the civilization's own first.
pub fn apply_offence(
    stances: &mut CivilizationStances,
    civilization: CivilizationId,
    now: f64,
) -> Vec<StanceStep> {
    stances.count_outlaw_hull(&civilization);
    let Some(step) = stances.offence(civilization, seeded_stance(&civilization), now) else {
        return Vec::new();
    };
    let mut steps = vec![step];
    let field = CivilizationField::new(civilization.world_seed);
    for neighbour in field.neighbours(civilization.node) {
        let by = match field.edge(civilization.node, neighbour.id.node) {
            Some(StanceType::Ally) => -1,
            Some(StanceType::Enemy) => 1,
            Some(StanceType::Neutral) | None => continue,
        };
        if let Some(step) = stances.propagate(neighbour.id, neighbour.stance, by, now) {
            steps.push(step);
        }
    }
    steps
}

/// Record a merit for `civilization` at `now`: the player handed in a job
/// one of its stations posted. The step it made, if any.
pub fn record_merit(
    stances: &mut CivilizationStances,
    civilization: CivilizationId,
    now: f64,
) -> Option<StanceStep> {
    stances.merit(civilization, seeded_stance(&civilization), now)
}

/// Settle the player with `civilization` at `now`: the fine is paid. The
/// step it made, or `None` when it was not Enemy.
pub fn settle(
    stances: &mut CivilizationStances,
    civilization: CivilizationId,
    now: f64,
) -> Option<StanceStep> {
    stances.settle(civilization, seeded_stance(&civilization), now)
}

/// Where one civilization stands with the player at a play time, and what
/// moves it next, as the Map's stance block shows it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StanceStanding {
    /// The table stance: the seed's, moved by the player's events.
    pub table: StanceType,
    /// The stance its bodies act on: Enemy while a provocation holds.
    pub effective: StanceType,
    /// Offences inside [`OFFENCE_WINDOW_SECONDS`] toward the next step down.
    pub offences: u32,
    /// Jobs handed in toward the next step up.
    pub merits: u32,
    /// Play seconds until a provocation stops reading Enemy, while one holds.
    pub provoked_left: Option<f64>,
    /// Play seconds with no new offence until an Enemy by events returns to
    /// Neutral. `None` while not Enemy, and for an Enemy by the seed alone,
    /// which time never clears: only the fine does.
    pub decay_left: Option<f64>,
}

/// How `civilization` stands with the player at `now`.
pub fn standing(
    stances: &CivilizationStances,
    civilization: &CivilizationId,
    now: f64,
) -> StanceStanding {
    let seeded = seeded_stance(civilization);
    let table = stances.stance(civilization, seeded);
    let record = stances.record(civilization);
    StanceStanding {
        table,
        effective: stances.effective(civilization, seeded, now),
        offences: record.map_or(0, |record| {
            record
                .offences
                .iter()
                .filter(|at| now - **at < OFFENCE_WINDOW_SECONDS)
                .count() as u32
        }),
        merits: record.map_or(0, |record| record.merits),
        provoked_left: record
            .and_then(|record| record.provoked_at)
            .map(|at| PROVOKED_WINDOW_SECONDS - (now - at))
            .filter(|left| *left > 0.0),
        decay_left: record
            .filter(|record| record.stance == StanceType::Enemy)
            .and_then(|record| record.enemy_since)
            .map(|since| (ENEMY_DECAY_SECONDS - (now - since)).max(0.0)),
    }
}

/// The living neighbours of `civilization` whose table stance is Enemy,
/// each with its fine ([`fine_cr`]) in the balance's unit: the fines a
/// station of `civilization` takes. The table stance, not the effective
/// one: a provocation reads Enemy for its window but [`settle`] steps only a
/// true Enemy, and a fine must never move without a step.
pub fn enemy_neighbours(
    civilization: CivilizationId,
    stances: &CivilizationStances,
) -> Vec<(CivilizationId, u32)> {
    CivilizationField::new(civilization.world_seed)
        .neighbours(civilization.node)
        .into_iter()
        .filter(|neighbour| stances.stance(&neighbour.id, neighbour.stance) == StanceType::Enemy)
        .map(|neighbour| {
            let fine = u32::try_from(fine_cr(&neighbour.id)).expect("FINE_CR's ends fit a balance");
            (neighbour.id, fine)
        })
        .collect()
}

/// Record an offence for a generated hull the player just neutralized: the
/// hull carries a [`TraderMarket`] (every intact generated ship and living
/// station does) and its [`LastAttacker`] is the player's ship or a fleet
/// escort. A hull thawed already neutralized carries no attacker and
/// records nothing.
pub(crate) fn record_offence(
    add: On<Add, NeutralizedMarker>,
    hulls: Query<(Option<&Name>, &TraderMarket, Option<&LastAttacker>)>,
    player: Query<(), Or<(With<PlayerSpaceshipMarker>, With<PlayerFleetMarker>)>>,
    clock: Res<WorldClock>,
    mut stances: ResMut<CivilizationStances>,
) {
    let Ok((name, market, attacker)) = hulls.get(add.entity) else {
        return;
    };
    let Some(LastAttacker(attacker)) = attacker else {
        return;
    };
    if !player.contains(*attacker) {
        return;
    }
    let label = name.map_or_else(|| format!("{:?}", add.entity), |name| name.to_string());
    info!(
        "stance: the player neutralized '{label}' of {} at {:.1} s of play",
        market.civilization.name(),
        clock.play_seconds
    );
    for step in apply_offence(&mut stances, market.civilization, clock.play_seconds) {
        log_step(&step, clock.play_seconds);
    }
}

/// Stamp a provocation for the civilization of a generated body the
/// player's ship just provoked. A body the player owns is the player's; a
/// shooter other than the player's ship is the AI's business.
pub(crate) fn record_provocation(
    provoked: On<ShipProvoked>,
    bodies: Query<(Option<&Name>, &TraderMarket), Without<OwnedStationMarker>>,
    player: Query<(), With<PlayerSpaceshipMarker>>,
    clock: Res<WorldClock>,
    mut stances: ResMut<CivilizationStances>,
) {
    if !player.contains(provoked.shooter) {
        return;
    }
    let Ok((name, market)) = bodies.get(provoked.ship) else {
        return;
    };
    let label = name.map_or_else(|| format!("{:?}", provoked.ship), |name| name.to_string());
    info!(
        "stance: the player provoked '{label}' of {} at {:.1} s of play; it reads Enemy for the window",
        market.civilization.name(),
        clock.play_seconds
    );
    stances.provoke(
        market.civilization,
        seeded_stance(&market.civilization),
        clock.play_seconds,
    );
}

/// Let the play clock pass over the table: windows close and an Enemy by
/// events decays.
pub(crate) fn expire_stances(clock: Res<WorldClock>, mut stances: ResMut<CivilizationStances>) {
    if stances.is_empty() {
        return;
    }
    for step in stances.expire(clock.play_seconds, seeded_stance) {
        log_step(&step, clock.play_seconds);
    }
}

/// Write each civilization's effective stance as the side of every live
/// generated body it owns. The seed stance of each civilization met is
/// cached: the draw is pure in the id.
pub(crate) fn sync_civilization_sides(
    clock: Res<WorldClock>,
    stances: Res<CivilizationStances>,
    mut bodies: Query<
        (&TraderMarket, &mut Allegiance),
        (
            Without<OwnedStationMarker>,
            Without<PlayerFleetMarker>,
            Without<NeutralizedMarker>,
        ),
    >,
    mut seeded: Local<BTreeMap<CivilizationId, StanceType>>,
) {
    for (market, mut allegiance) in &mut bodies {
        let seed = *seeded
            .entry(market.civilization)
            .or_insert_with(|| seeded_stance(&market.civilization));
        let side = stances
            .effective(&market.civilization, seed, clock.play_seconds)
            .side();
        allegiance.set_if_neq(side);
    }
}

fn log_step(step: &StanceStep, now: f64) {
    info!(
        "stance: {} moved from {:?} to {:?} at {now:.1} s of play",
        step.civilization.name(),
        step.from,
        step.to
    );
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use nova_gameplay::prelude::{GameItems, OFFENCE_STEP};
    use nova_scenario::prelude::{RoleGoal, RoleGoalType};

    use super::*;
    use crate::{
        civilizations::{
            sweep::{sweep_nodes, SWEEP_SEEDS},
            CivilizationStatusType,
        },
        goal::{goal_progress, GoalFacts},
    };

    /// A living civilization of `seed` with at least one living neighbour.
    fn a_civilization_with_neighbours(seed: u32) -> Option<CivilizationId> {
        let field = CivilizationField::new(seed);
        sweep_nodes().into_iter().find_map(|node| {
            let civilization = field.civilization(node);
            (civilization.status == CivilizationStatusType::Living
                && !field.neighbours(node).is_empty())
            .then_some(civilization.id)
        })
    }

    /// Three offences inside the window step a civilization down one and
    /// reach its living neighbours alone: an Ally edge follows the step, an
    /// Enemy edge warms, a Neutral edge moves nothing, and no civilization
    /// outside the neighbourhood gets a record. Over every sweep seed.
    #[test]
    fn a_flip_moves_only_the_civilization_and_its_neighbours() {
        let mut swept = 0;
        for seed in SWEEP_SEEDS {
            let Some(id) = a_civilization_with_neighbours(seed) else {
                continue;
            };
            swept += 1;
            let field = CivilizationField::new(seed);
            let before = seeded_stance(&id);
            let mut stances = CivilizationStances::default();
            let mut steps = Vec::new();
            for offence in 0..OFFENCE_STEP {
                steps = apply_offence(&mut stances, id, 10.0 * f64::from(offence));
            }
            assert_eq!(
                stances.stance(&id, before),
                before.stepped(-1),
                "seed {seed}: {} steps down once at the third offence",
                id.name()
            );
            let neighbours = field.neighbours(id.node);
            let allowed: BTreeSet<CivilizationId> = neighbours
                .iter()
                .map(|neighbour| neighbour.id)
                .chain([id])
                .collect();
            let recorded: BTreeSet<CivilizationId> = stances.entries().map(|(id, _)| *id).collect();
            assert!(
                recorded.is_subset(&allowed),
                "seed {seed}: a flip reached outside the neighbourhood: {recorded:?} vs {allowed:?}"
            );
            if before == StanceType::Enemy {
                // Nothing lies below Enemy: no step, no propagation.
                assert!(steps.is_empty(), "seed {seed}: an Enemy has no step below");
                continue;
            }
            assert_eq!(steps[0].civilization, id, "seed {seed}: the own step leads");
            for neighbour in &neighbours {
                let expected = match field.edge(id.node, neighbour.id.node) {
                    Some(StanceType::Ally) => neighbour.stance.stepped(-1),
                    Some(StanceType::Enemy) => neighbour.stance.stepped(1),
                    Some(StanceType::Neutral) => neighbour.stance,
                    None => panic!("seed {seed}: a neighbour has an edge"),
                };
                assert_eq!(
                    stances.stance(&neighbour.id, neighbour.stance),
                    expected,
                    "seed {seed}: {} follows its edge to {}",
                    neighbour.id.name(),
                    id.name()
                );
            }
        }
        assert!(
            swept >= 8,
            "the sweep found {swept} civilizations with neighbours"
        );
    }

    /// A civilization the player's offences made Enemy reads the time left
    /// on its decay, counted from the last offence; one the seed drew Enemy
    /// reads none, since only the fine clears it.
    #[test]
    fn an_enemy_by_events_reads_time_left_and_a_seeded_enemy_reads_none() {
        let living = |wanted: StanceType| {
            SWEEP_SEEDS
                .into_iter()
                .find_map(|seed| {
                    let field = CivilizationField::new(seed);
                    sweep_nodes().into_iter().find_map(|node| {
                        let civilization = field.civilization(node);
                        (civilization.status == CivilizationStatusType::Living
                            && civilization.stance == wanted)
                            .then_some(civilization.id)
                    })
                })
                .unwrap_or_else(|| panic!("the sweep holds a living {wanted:?} civilization"))
        };
        let mut stances = CivilizationStances::default();

        let seeded_enemy = living(StanceType::Enemy);
        let read = standing(&stances, &seeded_enemy, 100.0);
        assert_eq!(read.table, StanceType::Enemy);
        assert_eq!(read.decay_left, None, "time never clears a seeded Enemy");

        let neutral = living(StanceType::Neutral);
        for offence in 0..OFFENCE_STEP {
            apply_offence(&mut stances, neutral, 10.0 * f64::from(offence));
        }
        let flipped_at = 10.0 * f64::from(OFFENCE_STEP - 1);
        let read = standing(&stances, &neutral, flipped_at + 600.0);
        assert_eq!(read.table, StanceType::Enemy);
        assert_eq!(read.offences, 0, "the step clears the window's tally");
        assert_eq!(
            read.decay_left,
            Some(ENEMY_DECAY_SECONDS - 600.0),
            "the decay counts from the flipping offence"
        );
    }

    /// The offences that turn a Neutral civilization Enemy take no outlaw
    /// hull, the flipping kill included; six more while it stands Enemy
    /// meet the pirate's charter.
    #[test]
    fn the_flipping_kill_counts_no_outlaw_hull_and_six_after_it_meet_the_outlaw_charter() {
        let id = SWEEP_SEEDS
            .into_iter()
            .find_map(|seed| {
                let field = CivilizationField::new(seed);
                sweep_nodes().into_iter().find_map(|node| {
                    let civilization = field.civilization(node);
                    (civilization.status == CivilizationStatusType::Living
                        && civilization.stance == StanceType::Neutral)
                        .then_some(civilization.id)
                })
            })
            .expect("the sweep holds a living Neutral civilization");
        let goal = RoleGoal {
            title: "An outlaw's name".to_string(),
            kind: RoleGoalType::Outlaw { hulls: 6 },
        };
        let charter = |stances: &CivilizationStances| {
            goal_progress(
                &goal,
                &GoalFacts {
                    completed: &[],
                    owned_stations: 1,
                    items: &GameItems::default(),
                    outlaw: stances.outlaw_progress(),
                },
            )
        };
        let mut stances = CivilizationStances::default();
        for offence in 0..OFFENCE_STEP {
            apply_offence(&mut stances, id, f64::from(offence));
        }
        assert_eq!(stances.stance(&id, StanceType::Neutral), StanceType::Enemy);
        assert_eq!(
            stances.record(&id).map(|record| record.outlaw_hulls),
            Some(0),
            "the flipping kill is no outlaw hull"
        );
        assert!(!charter(&stances).is_met());
        for kill in 0..6 {
            apply_offence(&mut stances, id, 10.0 + f64::from(kill));
        }
        assert_eq!(
            stances.record(&id).map(|record| record.outlaw_hulls),
            Some(6)
        );
        assert!(charter(&stances).is_met());
    }

    /// The fine reads the low end at advancement 0 and the high end at 1,
    /// and a backwater's fine stays under a start kit.
    #[test]
    fn the_fine_runs_between_its_ends_by_advancement() {
        let mut fines: Vec<(f32, u64)> = SWEEP_SEEDS
            .iter()
            .flat_map(|&seed| {
                let field = CivilizationField::new(seed);
                sweep_nodes().map(move |node| {
                    let civilization = field.civilization(node);
                    (civilization.advancement, fine_cr(&civilization.id))
                })
            })
            .collect();
        fines.sort_by(|a, b| a.0.total_cmp(&b.0));
        for pair in fines.windows(2) {
            assert!(
                pair[0].1 <= pair[1].1,
                "the fine rises with advancement: {pair:?}"
            );
        }
        for (advancement, fine) in &fines {
            assert!((1_000..=10_000).contains(fine), "{advancement}: {fine}");
            if *advancement < 0.05 {
                assert!(
                    *fine < 3_000,
                    "a backwater's fine stays under the start kit: {fine}"
                );
            }
        }
    }
}
