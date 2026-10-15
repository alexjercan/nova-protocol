//! A station's hull offers, and the buy that swaps a ship's hull with a hull
//! parked at the station.
//!
//! A generated living station's offers are [`StationHullOffer`]s: the hulls
//! for sale and the escorts for hire the world generator laid out for its
//! civilization, as whole inline designs. Its first spawn hands them to
//! `crate::objects::parked_hulls`, which parks each hull for sale at a port
//! and lets each escort for hire coast beside it.
//!
//! [`buy_station_hull`] answers a [`HullBuyRequest`] once the buy rule moved
//! the credits, by a swap ([`swap_parked_hull`]). The ship root stays, so
//! everything that keys on it (the HUD, the world observer, the fleet, the
//! scenario id) keeps its owner: it takes the parked hull's design, state
//! and place and the start-role bindings for it, and [`redock_bought_hull`]
//! docks it at the parked hull's port. The old hull is parked where it
//! stood, its hold whole, for rebuy at the trade-in.

use avian3d::prelude::*;
use bevy::prelude::*;
use nova_gameplay::{prelude::*, transform::prelude::PointRotationOutput};
use nova_ship::prelude::*;

use crate::objects::{
    parked_hulls::spawn_parked_hull,
    ship_design::prelude::{ResolvedShipDesign, ShipDesign},
    spaceship::{freeze_ship, prelude::SpaceshipController, rehull_ship, thaw_ship},
    start_role::prelude::start_role_bindings,
};

/// The station hull offer and the buy.
pub mod prelude {
    pub use super::{StationHullOffer, StationHullsPlugin};
}

/// How many frames a bought hull tries to dock at its port before it gives
/// up and flies free: a pair the capture gates refuse that long will not
/// pass them by waiting.
const REDOCK_FRAMES: u32 = 30;

/// One hull a generated station offers: the name its list shows, its terms
/// and the whole design, inline, every section a prototype of the loaded
/// catalog. The station's first spawn parks a hull for sale at a port and
/// lets an escort for hire coast beside it.
#[derive(Clone, Debug, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct StationHullOffer {
    /// The name the list shows (`Vel civilian Mk 1`).
    pub name: String,
    /// Whether the player buys it or hires it as an escort.
    pub terms: HullOfferType,
    /// The hull a buy or a hire spawns.
    pub design: ShipDesign,
}

/// Marks a ship whose hull was just swapped and whose new port has yet to
/// dock with `station`. `released` is the connection the old port held,
/// which the old port's despawn lets go.
#[derive(Component, Clone, Copy, Debug)]
struct HullBuyRedock {
    station: Entity,
    released: Entity,
    frames: u32,
}

/// Registers the buy and the redock.
pub struct StationHullsPlugin;

impl Plugin for StationHullsPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(buy_station_hull);
        app.add_systems(Update, redock_bought_hull);
    }
}

/// Swap a ship's hull with a parked hull of the station it is docked at, as
/// a Station pane buy asks once the buy rule moved the credits. The swap
/// needs the whole world, so it runs as a queued command
/// ([`swap_parked_hull`]).
fn buy_station_hull(request: On<HullBuyRequest>, mut commands: Commands) {
    let request = request.event().clone();
    commands.queue(move |world: &mut World| swap_parked_hull(world, request));
}

/// The swap a [`HullBuyRequest`] asks for.
///
/// Both hulls freeze. The parked hull despawns, and the ship root is
/// rebuilt from its record where it lay ([`rehull_ship`]), with the
/// start-role bindings for its design ([`bought_hull_controller`]), at the
/// station's motion, to dock at its port ([`redock_bought_hull`]). The old
/// hull's record thaws as a parked hull of the station where the ship root
/// stood ([`FrozenShip::parked`](crate::objects::spaceship::prelude::FrozenShip)),
/// its hold whole, listed for rebuy at `trade_in_cr`, and docks at the port
/// the ship held.
///
/// # Panics
///
/// On a ship that is not docked with the station, on a hull that is not
/// parked there, and on either hull mid-process: the pane raises a buy only
/// for the docked player, with a hull parked at the station it is docked
/// with.
fn swap_parked_hull(world: &mut World, request: HullBuyRequest) {
    let HullBuyRequest {
        entity: ship,
        station,
        hull,
        trade_in_cr,
    } = request;
    assert_eq!(
        world.get::<ParkedHull>(hull).map(|parked| parked.0),
        Some(station),
        "swap_parked_hull: {hull:?} is not parked at station {station:?}"
    );
    assert!(
        !matches!(
            world.get::<ParkedHullType>(hull),
            Some(ParkedHullType::Hire { .. })
        ),
        "swap_parked_hull: {hull:?} is for hire, not for sale"
    );
    let Some(docked) = world.get::<DockedShip>(ship).copied() else {
        panic!("swap_parked_hull: ship {ship:?} is not docked");
    };
    assert!(
        world
            .get::<DockingConnection>(docked.connection)
            .is_some_and(|connection| connection.joins(station)),
        "swap_parked_hull: ship {ship:?} is not docked with station {station:?}"
    );
    let pose = |world: &World, entity: Entity| {
        (
            world
                .get::<Position>(entity)
                .map_or(Vec3::ZERO, |position| position.0),
            world
                .get::<Rotation>(entity)
                .map_or(Quat::IDENTITY, |rotation| rotation.0),
        )
    };
    let (old_at, old_turn) = pose(world, ship);
    let (new_at, new_turn) = pose(world, hull);
    let velocity = world
        .get::<LinearVelocity>(station)
        .copied()
        .unwrap_or_default();
    let spin = world
        .get::<AngularVelocity>(station)
        .copied()
        .unwrap_or_default();
    let buyer = world
        .get::<SpaceshipController>(ship)
        .cloned()
        .unwrap_or_else(|| panic!("swap_parked_hull: ship {ship:?} has no controller"));
    let old_name = world
        .get::<Name>(ship)
        .map_or_else(|| "Traded-in hull".to_string(), ToString::to_string);
    let new_name = world
        .get::<Name>(hull)
        .map_or_else(String::new, ToString::to_string);
    let unsettled = |what: &str, unsettled: UnsettledBody| -> ! {
        panic!("swap_parked_hull: the {what} cannot freeze: {unsettled}")
    };
    let bought = freeze_ship(world, hull).unwrap_or_else(|reason| unsettled("parked hull", reason));
    let old = freeze_ship(world, ship).unwrap_or_else(|reason| unsettled("ship", reason));
    info!(
        "swap_parked_hull: ship {ship:?} takes '{new_name}' at station {station:?}, its old hull \
         parks for rebuy at {trade_in_cr} cr"
    );

    world.entity_mut(hull).despawn();
    // Posed before the rebuild: each new flight computer seeds its attitude
    // command from the root's Transform as it spawns
    // (`insert_controller_section_target`), and a seed from the old pose
    // turns the bought hull off its port before it redocks.
    let pose = Transform::from_translation(new_at).with_rotation(new_turn);
    world
        .entity_mut(ship)
        .insert((Position(new_at), Rotation(new_turn), pose));
    rehull_ship(world, ship, bought, |resolved| {
        bought_hull_controller(&buyer, resolved)
    });
    // The root takes the bought hull's name, so a later rebuy parks this
    // hull under it.
    world.entity_mut(ship).insert((
        Name::new(new_name),
        velocity,
        spin,
        HullBuyRedock {
            station,
            released: docked.connection,
            frames: 0,
        },
    ));
    // Undocked, the player's hull slews toward the chase camera's aim; aim it
    // along the new hull, or the hull turns toward where the old one looked
    // before it redocks.
    for mut aim in world.query::<&mut PointRotationOutput>().iter_mut(world) {
        **aim = new_turn;
    }

    let mut commands = world.commands();
    let mut parked = spawn_parked_hull(
        &mut commands,
        station,
        &old_name,
        ParkedHullType::Rebuy {
            price_cr: trade_in_cr,
        },
        Transform::from_translation(old_at).with_rotation(old_turn),
    );
    thaw_ship(&mut parked, old.parked());
    parked.insert((velocity, spin));
    world.flush();
}

/// The driver a bought hull flies with: the buyer's, its binding table
/// replaced by the start-role scheme for `hull` ([`start_role_bindings`]).
/// No old binding carries over.
///
/// # Panics
///
/// On a buyer that is not player-driven: only the player's pane raises a
/// buy.
fn bought_hull_controller(
    buyer: &SpaceshipController,
    hull: &ResolvedShipDesign,
) -> SpaceshipController {
    let SpaceshipController::Player(config) = buyer else {
        panic!("buy_station_hull: the buyer is not player-driven: {buyer:?}");
    };
    let mut config = config.clone();
    config.input_mapping = start_role_bindings(hull);
    SpaceshipController::Player(config)
}

/// Dock a bought hull's new port with the station it was bought at, once
/// the port is live and the old dock has let go. A pair the capture gates
/// refuse for [`REDOCK_FRAMES`] is left to fly free, with a warning.
fn redock_bought_hull(
    mut commands: Commands,
    mut q_ships: Query<(Entity, &mut HullBuyRedock, &Children, Option<&DockedShip>)>,
    q_ports: Query<
        (),
        (
            With<DockingSectionMarker>,
            Without<DockedPort>,
            Without<SectionInactiveMarker>,
        ),
    >,
) {
    for (ship, mut redock, children, docked) in &mut q_ships {
        match docked {
            Some(docked) if docked.connection != redock.released => {
                debug!("redock_bought_hull: ship {ship:?} docked its new hull");
                commands.entity(ship).remove::<HullBuyRedock>();
                continue;
            }
            // The old connection has yet to let go.
            Some(_) => continue,
            None => {}
        }
        if !children.iter().any(|child| q_ports.contains(child)) {
            continue;
        }
        if redock.frames >= REDOCK_FRAMES {
            warn!(
                "redock_bought_hull: ship {ship:?} could not dock its new hull with {:?}",
                redock.station
            );
            commands.entity(ship).remove::<HullBuyRedock>();
            continue;
        }
        redock.frames += 1;
        commands.trigger(DockingConnectionRequest {
            entity: ship,
            target: redock.station,
        });
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use nova_events::prelude::Meters;
    use nova_input::prelude::InputSource;

    use super::*;
    use crate::objects::{
        ship_design::prelude::ResolvedSection, spaceship::prelude::PlayerControllerConfig,
    };

    fn section(id: &str, kind: SectionKind) -> ResolvedSection {
        ResolvedSection {
            id: id.to_string(),
            position: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            config: SectionConfig {
                base: BaseSectionConfig {
                    id: id.to_string(),
                    ..default()
                },
                kind,
            },
        }
    }

    /// A bought hull binds its turrets to the trigger, railguns to R,
    /// torpedoes to F and its mining emitters to V then B, and keeps none of
    /// the old hull's bindings, a rebind included.
    #[test]
    fn a_bought_hull_takes_the_start_role_bindings_and_drops_the_old_ones() {
        let beam = SectionKind::Mining(MiningSectionConfig {
            render_mesh: default(),
            render_mesh_transform: None,
            pulse_sound: default(),
            refusal_sound: default(),
            door_open_sound: default(),
            door_close_sound: default(),
            reach: Meters(100.0),
            pulse_interval_seconds: 1.0,
            carve_radius_cells: 1.5,
        });
        let hull = ResolvedShipDesign {
            sections: vec![
                section("plate", SectionKind::Hull(HullSectionConfig::default())),
                section(
                    "drive",
                    SectionKind::Thruster(ThrusterSectionConfig::default()),
                ),
                section("pdc", SectionKind::Turret(TurretSectionConfig::default())),
                section(
                    "spinal",
                    SectionKind::Railgun(RailgunSectionConfig::default()),
                ),
                section("bay", SectionKind::Torpedo(TorpedoSectionConfig::default())),
                section("beam_a", beam.clone()),
                section("beam_b", beam),
            ],
            ..default()
        };
        let old = SpaceshipController::Player(PlayerControllerConfig {
            input_mapping: BTreeMap::from([
                (
                    "pdc".to_string(),
                    vec![InputSource::Keyboard(KeyCode::KeyQ)],
                ),
                (
                    "old_beam".to_string(),
                    vec![InputSource::Keyboard(KeyCode::KeyB)],
                ),
            ]),
        });

        let SpaceshipController::Player(config) = bought_hull_controller(&old, &hull) else {
            panic!("a bought hull stays player-driven");
        };

        let trigger = vec![
            InputSource::Mouse(MouseButton::Left),
            InputSource::Gamepad(GamepadButton::RightTrigger2),
        ];
        let key = |code| vec![InputSource::Keyboard(code)];
        assert_eq!(
            config.input_mapping,
            BTreeMap::from([
                ("pdc".to_string(), trigger),
                ("spinal".to_string(), key(KeyCode::KeyR)),
                ("bay".to_string(), key(KeyCode::KeyF)),
                ("beam_a".to_string(), key(KeyCode::KeyV)),
                ("beam_b".to_string(), key(KeyCode::KeyB)),
            ])
        );
    }
}
