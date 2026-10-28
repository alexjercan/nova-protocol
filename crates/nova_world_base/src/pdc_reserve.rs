//! Station PDC reserve catch-up on a thaw: the reserve's consumer of the
//! play-time clock.
//!
//! Nothing frozen simulates, so a station's [`StationPdcReserve`] stands still
//! while its cell is off the window and catches up the moment the cell thaws:
//! [`recharge_thawed_station_pdc_reserves`] reads the [`ThawedAfter`] stamp the
//! thaw put on the body and recharges the reserve by that play time, at the
//! rate `nova_ship`'s `recharge_station_pdc_reserve` applies while the station
//! is live. A dead or neutralized station recharges in neither place.

use bevy::prelude::*;
use nova_gameplay::prelude::{DerelictShipMarker, NeutralizedMarker};
use nova_ship::prelude::StationPdcReserve;
use nova_world::prelude::ThawedAfter;

/// Recharge the [`StationPdcReserve`] of every living station that just
/// thawed by the play time it stood frozen. `thaw_station` inserts the
/// reserve as it froze, so the catch-up keys on the reserve landing, and
/// runs in the thaw's frame, before the next fixed tick recharges or reloads
/// from it.
pub(crate) fn recharge_thawed_station_pdc_reserves(
    mut stations: Query<
        (Entity, Option<&Name>, &ThawedAfter, &mut StationPdcReserve),
        (
            Added<StationPdcReserve>,
            Without<DerelictShipMarker>,
            Without<NeutralizedMarker>,
        ),
    >,
) {
    for (entity, name, thawed, mut reserve) in &mut stations {
        let before = reserve.rounds;
        reserve.recharge(thawed.play_seconds);
        let label = name.map_or_else(|| format!("{entity:?}"), |name| name.to_string());
        info!(
            "pdc reserve: '{label}' stood frozen {:.1} s, {before} -> {} rounds",
            thawed.play_seconds, reserve.rounds
        );
    }
}

#[cfg(test)]
mod tests {
    use bevy::ecs::system::RunSystemOnce;
    use nova_ship::prelude::{STATION_PDC_RESERVE_CAPACITY, STATION_PDC_RESERVE_RECHARGE_SECONDS};

    use super::*;

    /// A living station catches up the play time it stood frozen, up to a
    /// full reserve; a dead or neutralized one keeps what it froze with.
    #[test]
    fn a_thawed_living_station_catches_up_its_reserve_and_a_dead_one_does_not() {
        let mut world = World::new();
        let empty = StationPdcReserve {
            rounds: 0,
            progress: 0.0,
        };
        let stood = |play_seconds| ThawedAfter { play_seconds };
        let half = STATION_PDC_RESERVE_RECHARGE_SECONDS / 2.0;
        let living = world.spawn((stood(half), empty)).id();
        let long_frozen = world
            .spawn((stood(STATION_PDC_RESERVE_RECHARGE_SECONDS * 3.0), empty))
            .id();
        let derelict = world.spawn((stood(half), empty, DerelictShipMarker)).id();
        let neutralized = world.spawn((stood(half), empty, NeutralizedMarker)).id();

        world
            .run_system_once(recharge_thawed_station_pdc_reserves)
            .unwrap();
        let reserve = |world: &World, station| *world.get::<StationPdcReserve>(station).unwrap();
        assert_eq!(
            reserve(&world, living).rounds,
            STATION_PDC_RESERVE_CAPACITY / 2
        );
        assert_eq!(reserve(&world, long_frozen), StationPdcReserve::full());
        assert_eq!(reserve(&world, derelict), empty);
        assert_eq!(reserve(&world, neutralized), empty);
    }
}
