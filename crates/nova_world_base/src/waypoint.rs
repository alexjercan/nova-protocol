//! The player's chart waypoint: where its mark comes from, the hand-off to a
//! GOTO of the live station, and the rules that clear it.
//!
//! A chart GOTO names a target the window may not hold. Its mark is the pose
//! the station froze at, which is where it thaws ([`FrozenSectors`]), or the
//! centre of the chart cell. The Map engages `GotoPos` at that mark and puts
//! a [`Waypoint`] on the player ship. When the station streams in, the
//! `GotoPos` becomes a `Goto` of the station with the travel lock on it, so
//! the leg parks at the hull and not at a point inside it.
//!
//! A cell centre is a point the generator never keeps clear, so a rock or a
//! planetoid may stand on it. The autopilot pushes every `GotoPos` goal out
//! of a sized body's clearance and parks there, so the leg rests beside the
//! body, off the centre. That is reported as a warning once the body is live.
//!
//! Every fault here is a broken world, not a choice the player made: a
//! charted station with no frozen record has no place to fly to, and a
//! station that is gone once its cell is placed never thaws. Each one logs an
//! error and drops the waypoint. No fault falls back to the cell centre.

use bevy::prelude::*;
use nova_events::prelude::GeneratedBodyId;
use nova_gameplay::prelude::{
    Chart, ChartSectorEdge, PlayerSpaceshipMarker, StationMarker, Waypoint, WaypointTargetType,
};
use nova_ship::prelude::{
    Autopilot, AutopilotAction, BodyRadius, PlayerAutopilotCompleted, TravelLock,
};
use nova_world::prelude::{
    FrozenBodyType, FrozenSectors, NovaWorldSystems, PendingSectorStation, WorldConfig,
};

use crate::layered::NovaLayeredWorld;

/// Why a waypoint has no mark.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WaypointFault {
    /// The station is not in the player's chart.
    UnknownStation(GeneratedBodyId),
    /// The station is charted, but no cell holds a frozen record of it, so
    /// nothing says where it thaws.
    NoFrozenRecord(GeneratedBodyId),
    /// The mark is not a finite point.
    NonFiniteMark(WaypointTargetType),
}

impl std::fmt::Display for WaypointFault {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownStation(id) => write!(f, "station '{id}' is not in the chart"),
            Self::NoFrozenRecord(id) => {
                write!(
                    f,
                    "station '{id}' is charted but no cell holds its frozen record"
                )
            }
            Self::NonFiniteMark(target) => write!(f, "the mark of {target:?} is not finite"),
        }
    }
}

/// The engine-space point a waypoint at `target` flies to.
///
/// A station's mark is the pose its frozen record holds: a frozen body does
/// not move, and it thaws at that pose. A record held back as a pending
/// station carries no pose, so it is not a mark. A cell's mark is its
/// centre. The caller asks for a live station first: a live station has no
/// frozen record, and the Map flies a `Goto` of it at once.
pub fn waypoint_mark(
    target: &WaypointTargetType,
    chart: &Chart,
    frozen: &FrozenSectors,
    edge: ChartSectorEdge,
) -> Result<Vec3, WaypointFault> {
    let mark = match target {
        WaypointTargetType::Station(id) => {
            if chart.station(id).is_none() {
                return Err(WaypointFault::UnknownStation(id.clone()));
            }
            frozen_station_pose(frozen, id)
                .ok_or_else(|| WaypointFault::NoFrozenRecord(id.clone()))?
        }
        WaypointTargetType::Sector(coord) => coord.centre(edge.0).to_engine(),
    };
    if !mark.is_finite() {
        return Err(WaypointFault::NonFiniteMark(target.clone()));
    }
    Ok(mark)
}

/// Where the frozen record of station `id` stands, in any cell: a station
/// that drifted while live froze in the cell it drifted into.
fn frozen_station_pose(frozen: &FrozenSectors, id: &GeneratedBodyId) -> Option<Vec3> {
    let id = id.to_string();
    frozen.iter().find_map(|(_, record)| {
        record.bodies().iter().find_map(|body| {
            (matches!(body.body(), FrozenBodyType::Station(..)) && body.id() == Some(id.as_str()))
                .then(|| body.transform().translation)
        })
    })
}

/// The waypoint systems, in `Update` after [`NovaWorldSystems::Retire`], so a
/// station placed this frame is on hand for the hand-off.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct WaypointSystems;

/// Hands a station waypoint to the live station and clears a waypoint the
/// autopilot no longer flies.
pub struct WaypointPlugin;

impl Plugin for WaypointPlugin {
    fn build(&self, app: &mut App) {
        app.configure_sets(
            Update,
            WaypointSystems
                .after(NovaWorldSystems::Retire)
                .run_if(resource_exists::<WorldConfig<NovaLayeredWorld>>),
        );
        app.add_systems(
            Update,
            (
                hand_off_waypoint,
                refuse_lost_waypoint_station,
                clear_stale_waypoint,
                report_buried_sector_mark,
            )
                .chain()
                .in_set(WaypointSystems),
        );
        app.add_observer(refuse_unhanded_arrival);
    }
}

/// Whether `autopilot` still flies `waypoint`: the `GotoPos` at its mark, or,
/// for a station, the `Goto` of that station.
fn flies(
    waypoint: &Waypoint,
    autopilot: Option<&Autopilot>,
    q_ids: &Query<&GeneratedBodyId>,
) -> bool {
    match (
        &waypoint.target,
        autopilot.map(|autopilot| autopilot.action),
    ) {
        (_, Some(AutopilotAction::GotoPos { position })) => position == waypoint.mark,
        (WaypointTargetType::Station(id), Some(AutopilotAction::Goto { target })) => {
            q_ids.get(target).is_ok_and(|found| found == id)
        }
        _ => false,
    }
}

/// A station waypoint whose station is live flies a `Goto` of it, with the
/// travel lock on it, the way a local Map GOTO does.
pub(crate) fn hand_off_waypoint(
    mut commands: Commands,
    q_player: Query<(Entity, &Waypoint, &Autopilot), With<PlayerSpaceshipMarker>>,
    q_station: Query<(Entity, &GeneratedBodyId), With<StationMarker>>,
) {
    for (player, waypoint, autopilot) in &q_player {
        let WaypointTargetType::Station(id) = &waypoint.target else {
            continue;
        };
        if !matches!(autopilot.action, AutopilotAction::GotoPos { position } if position == waypoint.mark)
        {
            continue;
        }
        let Some((station, _)) = q_station.iter().find(|(_, found)| *found == id) else {
            continue;
        };
        commands.entity(player).insert((
            Autopilot::engage(AutopilotAction::Goto { target: station }),
            TravelLock(Some(station)),
        ));
    }
}

/// A station waypoint whose station is neither live, nor held back, nor
/// frozen anywhere is gone: its cell was placed without it. The leg would fly
/// to an empty point, so the waypoint and the autopilot drop and the error
/// says which station.
///
/// Checked when the ledger changes, which is when a cell's placement takes
/// its record, and when a waypoint is set.
pub(crate) fn refuse_lost_waypoint_station(
    mut commands: Commands,
    frozen: Res<FrozenSectors>,
    q_player: Query<(Entity, Ref<Waypoint>, &Autopilot), With<PlayerSpaceshipMarker>>,
    q_station: Query<&GeneratedBodyId, With<StationMarker>>,
    q_pending: Query<&PendingSectorStation>,
) {
    for (player, waypoint, autopilot) in &q_player {
        if !(frozen.is_changed() || waypoint.is_added()) {
            continue;
        }
        let WaypointTargetType::Station(id) = &waypoint.target else {
            continue;
        };
        if !matches!(autopilot.action, AutopilotAction::GotoPos { position } if position == waypoint.mark)
        {
            continue;
        }
        let live = q_station.iter().any(|found| found == id);
        let held = q_pending.iter().any(|pending| &pending.station().id == id);
        if live || held || frozen_station_pose(&frozen, id).is_some() {
            continue;
        }
        error!(
            "nova_world_base: waypoint station '{id}' is gone: no live station, no held \
             station and no frozen record holds it; dropping the waypoint and the autopilot"
        );
        commands
            .entity(player)
            .remove::<(Waypoint, Autopilot, TravelLock)>();
    }
}

/// Drops a waypoint the autopilot no longer flies: the pilot took the helm,
/// another order replaced it, or the leg ended.
pub(crate) fn clear_stale_waypoint(
    mut commands: Commands,
    q_player: Query<(Entity, &Waypoint, Option<&Autopilot>), With<PlayerSpaceshipMarker>>,
    q_ids: Query<&GeneratedBodyId>,
) {
    for (player, waypoint, autopilot) in &q_player {
        if !flies(waypoint, autopilot, &q_ids) {
            commands.entity(player).remove::<Waypoint>();
        }
    }
}

/// The first live sized body whose radius holds `mark`, of `bodies` as
/// `(entity, centre, radius)`.
pub(crate) fn burying_body(
    mark: Vec3,
    bodies: impl IntoIterator<Item = (Entity, Vec3, f32)>,
) -> Option<Entity> {
    bodies
        .into_iter()
        .find(|(_, centre, radius)| centre.distance(mark) < *radius)
        .map(|(entity, _, _)| entity)
}

/// A sector waypoint whose cell centre lies inside a live sized body: the
/// leg parks outside the body, not at the centre, so the warning names the
/// body. Once per waypoint and body.
pub(crate) fn report_buried_sector_mark(
    q_player: Query<(Entity, &Waypoint), With<PlayerSpaceshipMarker>>,
    q_bodies: Query<(Entity, &GlobalTransform, &BodyRadius, Option<&Name>)>,
    mut reported: Local<Option<(Vec3, Entity)>>,
) {
    for (player, waypoint) in &q_player {
        let WaypointTargetType::Sector(coord) = waypoint.target else {
            continue;
        };
        let bodies = q_bodies
            .iter()
            .filter(|(body, ..)| *body != player)
            .map(|(body, at, radius, _)| (body, at.translation(), **radius));
        let Some(body) = burying_body(waypoint.mark, bodies) else {
            continue;
        };
        if *reported == Some((waypoint.mark, body)) {
            continue;
        }
        *reported = Some((waypoint.mark, body));
        let name = q_bodies
            .get(body)
            .ok()
            .and_then(|(.., name)| name)
            .map_or_else(|| body.to_string(), ToString::to_string);
        warn!(
            "nova_world_base: the centre of waypoint cell {coord} lies inside '{name}'; the \
             leg parks outside it, off the centre"
        );
    }
}

/// A station waypoint that reached its mark as a `GotoPos` was never handed
/// to the station: the station's cell is live within two cells of the
/// player, long before the mark. That is a broken world, so the error says
/// which station.
fn refuse_unhanded_arrival(
    trigger: On<Insert<PlayerAutopilotCompleted>>,
    mut commands: Commands,
    q_player: Query<(&PlayerAutopilotCompleted, &Waypoint)>,
) {
    let player = trigger.event_target();
    let Ok((completed, waypoint)) = q_player.get(player) else {
        return;
    };
    let WaypointTargetType::Station(id) = &waypoint.target else {
        return;
    };
    if !matches!(completed.action, AutopilotAction::GotoPos { .. }) {
        return;
    }
    error!(
        "nova_world_base: waypoint station '{id}' was not live when the leg reached its mark; \
         the hand-off never ran; dropping the waypoint"
    );
    commands.entity(player).remove::<Waypoint>();
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use bevy::ecs::system::RunSystemOnce;
    use nova_events::prelude::{Meters, SectorCoord};
    use nova_gameplay::prelude::{
        DamageMarks, ShipInventory, StationAdvancement, StationBuilds, StationServices,
        StationStorage,
    };
    use nova_scenario::prelude::{freeze_station, spaceship_scenario_object, SpaceshipConfig};
    use nova_ship::prelude::{RcsBudget, StationPdcReserve, StationPost};

    use super::*;

    const EDGE: ChartSectorEdge = ChartSectorEdge(Meters(32_000.0));

    fn station_id() -> GeneratedBodyId {
        GeneratedBodyId::new(SectorCoord::new(4, 0, -1), "cluster_4_0_n1_station", 0)
    }

    /// Another body of the station's cell.
    fn other_id(index: u32) -> GeneratedBodyId {
        GeneratedBodyId::new(station_id().origin, "cluster_4_0_n1_station", index)
    }

    fn charted() -> Chart {
        let mut chart = Chart::default();
        chart.find_station(&station_id(), false, false, None, 10.0);
        chart
    }

    /// A ledger whose cell `coord` holds `body` under `id` at `at`, built
    /// through the record's RON form: no ledger body has a public
    /// constructor outside `nova_world`.
    fn ledger(coord: SectorCoord, id: &str, at: Vec3, body: &FrozenBodyType) -> FrozenSectors {
        ron::from_str(&format!(
            "{{{}: Visited([(id: Some({}), name: None, transform: {}, visibility: None, \
             motion: None, body: {}, frozen_at: 0.0)])}}",
            ron::to_string(&coord).unwrap(),
            ron::to_string(id).unwrap(),
            ron::to_string(&Transform::from_translation(at)).unwrap(),
            ron::to_string(body).unwrap(),
        ))
        .unwrap()
    }

    /// A bare station, frozen by the production path.
    fn frozen_station() -> FrozenBodyType {
        let mut world = World::new();
        let station = world
            .spawn((
                spaceship_scenario_object(SpaceshipConfig::default()),
                ShipInventory::new(&nova_gameplay::test_support::test_items(), 10_000, []),
                DamageMarks::default(),
                RcsBudget::default(),
                StationMarker,
                StationPost::Fixed {
                    position: Vec3::ZERO,
                },
                StationServices(Vec::new()),
                StationBuilds(Vec::new()),
                StationAdvancement(0.0),
                StationPdcReserve::full(),
                StationStorage::new(0),
            ))
            .id();
        FrozenBodyType::Station(
            Box::new(freeze_station(&world, station).expect("a bare station is settled")),
            Vec::new(),
        )
    }

    /// The station froze in a cell it drifted into, not the one that
    /// generated it: the mark is the frozen pose, wherever the record is.
    #[test]
    fn a_station_mark_is_the_pose_its_frozen_record_holds() {
        let at = Vec3::new(13_000.0, -40.0, 2_500.0);
        let frozen = ledger(
            SectorCoord::new(5, 0, -1),
            &station_id().to_string(),
            at,
            &frozen_station(),
        );
        assert_eq!(
            waypoint_mark(
                &WaypointTargetType::Station(station_id()),
                &charted(),
                &frozen,
                EDGE
            ),
            Ok(at)
        );
    }

    #[test]
    fn a_station_the_chart_does_not_hold_has_no_mark() {
        let frozen = ledger(
            station_id().origin,
            &station_id().to_string(),
            Vec3::ONE,
            &frozen_station(),
        );
        assert_eq!(
            waypoint_mark(
                &WaypointTargetType::Station(station_id()),
                &Chart::default(),
                &frozen,
                EDGE
            ),
            Err(WaypointFault::UnknownStation(station_id()))
        );
    }

    /// No record, or only a held-back one, which stands at no pose: no
    /// mark, and no fallback to the cell centre.
    #[test]
    fn a_charted_station_with_no_frozen_pose_has_no_mark() {
        let target = WaypointTargetType::Station(station_id());
        let fault = Err(WaypointFault::NoFrozenRecord(station_id()));
        assert_eq!(
            waypoint_mark(&target, &charted(), &FrozenSectors::default(), EDGE),
            fault
        );
        let held = FrozenBodyType::PendingStation {
            id: station_id().to_string(),
        };
        let frozen = ledger(
            station_id().origin,
            &station_id().to_string(),
            Vec3::ZERO,
            &held,
        );
        assert_eq!(waypoint_mark(&target, &charted(), &frozen, EDGE), fault);
    }

    /// A body whose radius holds the cell centre buries it; one that only
    /// comes near does not.
    #[test]
    fn a_cell_centre_inside_a_body_names_the_body() {
        let (near, over) = (
            Entity::from_raw_u32(1).unwrap(),
            Entity::from_raw_u32(2).unwrap(),
        );
        let mark = Vec3::new(100.0, 0.0, 0.0);
        assert_eq!(
            burying_body(
                mark,
                [
                    (near, Vec3::new(130.0, 0.0, 0.0), 29.0),
                    (over, Vec3::new(100.0, 40.0, 0.0), 41.0),
                ]
            ),
            Some(over)
        );
        assert_eq!(
            burying_body(mark, [(near, Vec3::new(130.0, 0.0, 0.0), 29.0)]),
            None
        );
    }

    #[test]
    fn a_sector_mark_is_the_cell_centre() {
        let coord = SectorCoord::new(-3, 1, 7);
        assert_eq!(
            waypoint_mark(
                &WaypointTargetType::Sector(coord),
                &Chart::default(),
                &FrozenSectors::default(),
                EDGE
            ),
            Ok(coord.centre(EDGE.0).to_engine())
        );
    }

    #[test]
    fn a_mark_that_is_not_finite_is_refused() {
        let target = WaypointTargetType::Sector(SectorCoord::new(1, 0, 0));
        assert_eq!(
            waypoint_mark(
                &target,
                &Chart::default(),
                &FrozenSectors::default(),
                ChartSectorEdge(Meters(f32::INFINITY))
            ),
            Err(WaypointFault::NonFiniteMark(target))
        );
    }

    const MARK: Vec3 = Vec3::new(0.0, 0.0, -12_800.0);

    /// A player flying a station waypoint's mark.
    fn flying_world() -> (World, Entity) {
        let mut world = World::new();
        world.init_resource::<FrozenSectors>();
        let player = world
            .spawn((
                PlayerSpaceshipMarker,
                Waypoint {
                    target: WaypointTargetType::Station(station_id()),
                    mark: MARK,
                },
                Autopilot::engage(AutopilotAction::GotoPos { position: MARK }),
            ))
            .id();
        (world, player)
    }

    fn action(world: &World, player: Entity) -> Option<AutopilotAction> {
        world
            .get::<Autopilot>(player)
            .map(|autopilot| autopilot.action)
    }

    /// The `GotoPos` flies on while the station is frozen, and becomes a
    /// `Goto` of the station with the travel lock on it once it is live.
    #[test]
    fn a_live_station_takes_over_its_waypoint_leg() {
        let (mut world, player) = flying_world();
        world.run_system_once(hand_off_waypoint).unwrap();
        assert_eq!(
            action(&world, player),
            Some(AutopilotAction::GotoPos { position: MARK })
        );

        world.spawn((StationMarker, other_id(1)));
        let station = world.spawn((StationMarker, station_id())).id();
        world.run_system_once(hand_off_waypoint).unwrap();
        assert_eq!(
            action(&world, player),
            Some(AutopilotAction::Goto { target: station })
        );
        assert_eq!(
            world.get::<TravelLock>(player),
            Some(&TravelLock(Some(station)))
        );
        world.run_system_once(clear_stale_waypoint).unwrap();
        assert!(
            world.get::<Waypoint>(player).is_some(),
            "the handed-off leg is still the waypoint's"
        );
    }

    /// The waypoint stays only while the autopilot flies it.
    #[test]
    fn a_waypoint_the_autopilot_no_longer_flies_is_cleared() {
        let other = |world: &mut World| world.spawn(other_id(7)).id();
        let cases: [(&str, fn(&mut World, Entity, Entity) -> Option<Autopilot>); 4] = [
            ("the pilot took the helm", |_, _, _| None),
            ("a move order flies another point", |_, _, _| {
                Some(Autopilot::engage(AutopilotAction::GotoPos {
                    position: MARK + Vec3::X,
                }))
            }),
            ("a GOTO of another body", |_, _, other| {
                Some(Autopilot::engage(AutopilotAction::Goto { target: other }))
            }),
            ("a STOP", |_, _, _| {
                Some(Autopilot::engage(AutopilotAction::Stop))
            }),
        ];
        for (case, autopilot) in cases {
            let (mut world, player) = flying_world();
            let other = other(&mut world);
            world.run_system_once(clear_stale_waypoint).unwrap();
            assert!(
                world.get::<Waypoint>(player).is_some(),
                "{case}: kept first"
            );
            match autopilot(&mut world, player, other) {
                Some(autopilot) => world.entity_mut(player).insert(autopilot),
                None => world.entity_mut(player).remove::<Autopilot>(),
            };
            world.run_system_once(clear_stale_waypoint).unwrap();
            assert!(world.get::<Waypoint>(player).is_none(), "{case}: cleared");
        }

        // A sector waypoint has no station to hand off to: a GOTO of a body
        // is never its leg.
        let (mut world, player) = flying_world();
        let station = world.spawn(station_id()).id();
        world.entity_mut(player).insert((
            Waypoint {
                target: WaypointTargetType::Sector(SectorCoord::new(4, 0, -1)),
                mark: MARK,
            },
            Autopilot::engage(AutopilotAction::Goto { target: station }),
        ));
        world.run_system_once(clear_stale_waypoint).unwrap();
        assert!(world.get::<Waypoint>(player).is_none());
    }

    /// The station's cell was placed and took its record, but no station
    /// stands, live or held: the waypoint and its leg drop. While the record
    /// is still frozen, or the station is held back, the leg flies on.
    #[test]
    fn a_waypoint_station_gone_after_its_cell_is_placed_drops_the_leg() {
        let (mut world, player) = flying_world();
        world.insert_resource(ledger(
            station_id().origin,
            &station_id().to_string(),
            MARK,
            &frozen_station(),
        ));
        world.run_system_once(refuse_lost_waypoint_station).unwrap();
        assert!(world.get::<Waypoint>(player).is_some(), "still frozen");

        world.insert_resource(FrozenSectors::default());
        world.run_system_once(refuse_lost_waypoint_station).unwrap();
        assert!(world.get::<Waypoint>(player).is_none());
        assert!(world.get::<Autopilot>(player).is_none());
    }

    /// Reaching the mark as a `GotoPos` means the station never streamed in
    /// on the way: the waypoint drops. A sector waypoint's arrival is its
    /// end, and the clear rule takes it.
    #[test]
    fn a_station_waypoint_that_reaches_its_mark_unhanded_is_dropped() {
        let (mut world, player) = flying_world();
        world.add_observer(refuse_unhanded_arrival);
        world.entity_mut(player).insert(PlayerAutopilotCompleted {
            action: AutopilotAction::GotoPos { position: MARK },
        });
        world.flush();
        assert!(world.get::<Waypoint>(player).is_none());

        let (mut world, player) = flying_world();
        world.add_observer(refuse_unhanded_arrival);
        world.entity_mut(player).insert(Waypoint {
            target: WaypointTargetType::Sector(SectorCoord::new(0, 0, 0)),
            mark: MARK,
        });
        world.entity_mut(player).insert(PlayerAutopilotCompleted {
            action: AutopilotAction::GotoPos { position: MARK },
        });
        world.flush();
        assert!(world.get::<Waypoint>(player).is_some());
    }
}
