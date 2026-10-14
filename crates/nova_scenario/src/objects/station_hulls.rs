//! The hulls a station sells, and the buy that replaces a ship's hull with
//! one of them.
//!
//! A generated living station carries a [`StationHullCatalogue`]: a short
//! list of whole inline designs the world generator laid out for its
//! civilization. The catalogue rides the station's frozen record, so a
//! station keeps it across a retire, a thaw and a save. Whenever it lands,
//! [`insert_hull_offers`] resolves it into the ship crate's [`HullOffers`],
//! which the buy rule and the Ship pane read. A station with no catalogue
//! sells no hull.
//!
//! [`buy_station_hull`] answers a [`HullBuyRequest`] once the buy rule moved
//! the hold and the credits. The ship root stays, so everything that keys on
//! it (the HUD, the world observer, the scenario id) keeps its owner. Its
//! sections go, its design becomes the offer, inline, and the new sections
//! spawn through the per-section spawn every ship takes, posed so the new
//! hull's first docking port lies on the old port's face. The old port's
//! despawn releases the dock, and [`redock_bought_hull`] docks the new port
//! once it is live.

use avian3d::prelude::*;
use bevy::prelude::*;
use nova_gameplay::prelude::*;
use nova_ship::prelude::*;

use crate::objects::{
    ship_design::prelude::{resolve_ship_design, GameShipDesigns, ShipDesign, ShipDesignSource},
    spaceship::{
        hull_components,
        prelude::{SpaceshipController, SpaceshipDesign},
        spawn_design_section,
    },
};

/// The station hull catalogue.
pub mod prelude {
    pub use super::{StationHullCatalogue, StationHullOffer, StationHullsPlugin};
}

/// How many frames a bought hull tries to dock its new port before it gives
/// up and flies free: a pair the capture gates refuse that long will not
/// pass them by waiting.
const REDOCK_FRAMES: u32 = 30;

/// One hull a station sells: the name its list shows and the whole design,
/// inline, every section a prototype of the loaded catalog.
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

/// The hulls a station sells, in list order. Inserted on a station root by
/// the world's spawn and thaw; a station with none sells no hull.
#[derive(Component, Clone, Debug, Default, Reflect)]
pub struct StationHullCatalogue(pub Vec<StationHullOffer>);

/// Marks a ship whose hull was just bought and whose new port has yet to
/// dock with `station`. `released` is the connection the old port held,
/// which the old port's despawn lets go.
#[derive(Component, Clone, Copy, Debug)]
struct HullBuyRedock {
    station: Entity,
    released: Entity,
    frames: u32,
}

/// Registers the catalogue observers, the buy and the redock.
pub struct StationHullsPlugin;

impl Plugin for StationHullsPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(insert_hull_offers);
        app.add_observer(buy_station_hull);
        app.add_systems(Update, redock_bought_hull);
    }
}

/// The resolved sections of `design`.
///
/// # Panics
///
/// On a design that does not resolve: the manifest check and the catalog
/// digest refuse one before a station carries it.
fn resolve_offer(
    design: &ShipDesign,
    designs: &GameShipDesigns,
    sections: &GameSections,
) -> (
    crate::objects::ship_design::prelude::ResolvedShipDesign,
    Vec<DesignSection>,
) {
    let (resolved, errors) =
        resolve_ship_design(&ShipDesignSource::Inline(design.clone()), designs, sections);
    assert!(
        errors.is_empty(),
        "station hull catalogue: a hull does not resolve: {errors:?}"
    );
    let placed = resolved
        .sections
        .iter()
        .map(|section| DesignSection {
            id: section.id.clone(),
            position: section.position,
            rotation: section.rotation,
            config: section.config.clone(),
        })
        .collect();
    (resolved, placed)
}

/// Resolve a station's catalogue into the [`HullOffers`] the buy rule and
/// the Ship pane read, whenever the catalogue lands.
///
/// # Panics
///
/// On a hull that does not resolve, see [`resolve_offer`].
fn insert_hull_offers(
    insert: On<Insert, StationHullCatalogue>,
    mut commands: Commands,
    q_catalogue: Query<&StationHullCatalogue>,
    designs: Res<GameShipDesigns>,
    sections: Res<GameSections>,
) {
    let Ok(catalogue) = q_catalogue.get(insert.entity) else {
        return;
    };
    let offers = catalogue
        .0
        .iter()
        .map(|offer| HullOffer {
            name: offer.name.clone(),
            terms: offer.terms,
            sections: resolve_offer(&offer.design, &designs, &sections).1,
        })
        .collect();
    commands.entity(insert.entity).insert(HullOffers(offers));
}

/// Replace a ship's hull with a station's offer, as a Ship pane buy asks
/// once the buy rule moved the hold and the credits.
///
/// Every section goes and the root's design becomes the offer, inline, so a
/// freeze and a save keep it. The player binding table becomes the
/// request's. The root's pinned health, damage marks, history stamps and
/// restored-skin mark go with the old hull, and the new hold opens empty at
/// the new hull's capacity. The root is posed by [`bought_hull_pose`] at the
/// station's motion, and [`redock_bought_hull`] docks it again.
///
/// # Panics
///
/// On a ship that is not a docked authored root with an empty hold, a
/// station with no such offer, or an offer with no docking port: the pane
/// raises a buy only for the docked player, with an offer of the station it
/// is docked with, after moving the hold, and the manifest check refuses a
/// hull with no port.
#[expect(
    clippy::type_complexity,
    reason = "the root's controller, pose, hold and sections in one query"
)]
fn buy_station_hull(
    request: On<HullBuyRequest>,
    mut commands: Commands,
    game_sections: Res<GameSections>,
    game_designs: Res<GameShipDesigns>,
    items: Res<GameItems>,
    q_station: Query<(&StationHullCatalogue, &LinearVelocity, &AngularVelocity)>,
    q_ship: Query<
        (
            &SpaceshipController,
            &Position,
            &Rotation,
            &ShipInventory,
            &Children,
            &DockedShip,
        ),
        (With<SpaceshipRootMarker>, Without<SectionMarker>),
    >,
    q_sections: Query<
        (
            Option<&Transform>,
            Has<DockingSectionMarker>,
            Has<DockedPort>,
        ),
        With<SectionMarker>,
    >,
) {
    let ship = request.entity;
    let Ok((controller, position, rotation, hold, children, docked)) = q_ship.get(ship) else {
        panic!("buy_station_hull: {ship:?} is not a docked authored ship root");
    };
    assert!(
        hold.is_empty(),
        "buy_station_hull: ship {ship:?} still holds cargo; the buy rule stores it first"
    );
    let Ok((catalogue, velocity, spin)) = q_station.get(request.station) else {
        panic!(
            "buy_station_hull: {:?} is not a station with a hull catalogue",
            request.station
        );
    };
    let Some(offer) = catalogue.0.get(request.offer) else {
        panic!(
            "buy_station_hull: station {:?} has no offer {}",
            request.station, request.offer
        );
    };
    assert_eq!(
        offer.terms,
        HullOfferType::Sale,
        "buy_station_hull: offer {} of station {:?} is for hire, not for sale",
        request.offer,
        request.station
    );
    let old_port = children
        .iter()
        .find_map(|child| match q_sections.get(child) {
            Ok((Some(transform), true, true)) => Some((transform.translation, transform.rotation)),
            _ => None,
        })
        .unwrap_or_else(|| panic!("buy_station_hull: ship {ship:?} has no docked port"));
    let (resolved, sections) = resolve_offer(&offer.design, &game_designs, &game_sections);
    let new_port = sections
        .iter()
        .find(|section| section.config.kind.class() == SectionClass::Docking)
        .map(|section| (section.position, section.rotation))
        .unwrap_or_else(|| {
            panic!(
                "buy_station_hull: hull '{}' has no docking port",
                offer.name
            )
        });
    let (at, turn) = bought_hull_pose((position.0, rotation.0), old_port, new_port);

    for child in children.iter() {
        if q_sections.contains(child) {
            commands.entity(child).despawn();
        }
    }
    // After the despawns: a section leaving writes its binding into the
    // controller table, and the new table replaces what those wrote.
    let mut controller = controller.clone();
    if let SpaceshipController::Player(config) = &mut controller {
        config.input_mapping = request.bindings.clone();
    }
    info!(
        "buy_station_hull: ship {ship:?} buys '{}' ({} sections) from station {:?}",
        offer.name,
        sections.len(),
        request.station
    );
    commands
        .entity(ship)
        .remove::<(
            Health,
            WasArmedCombatant,
            HadFlightComputer,
            HadThruster,
            ShipSkinRestored,
        )>()
        .insert((
            SpaceshipDesign(ShipDesignSource::Inline(offer.design.clone())),
            controller.clone(),
            DamageMarks::default(),
            hull_components(&resolved),
            ShipInventory::new(&items, resolved.cargo_capacity_g(), []),
            Position(at),
            Rotation(turn),
            Transform::from_translation(at).with_rotation(turn),
            *velocity,
            *spin,
            HullBuyRedock {
                station: request.station,
                released: docked.connection,
                frames: 0,
            },
        ));
    let mut mining_sources = Vec::new();
    commands.entity(ship).with_children(|parent| {
        for section in &sections {
            spawn_design_section(
                parent,
                section,
                &controller,
                false,
                &mut mining_sources,
                None,
            );
        }
    });
    commands.entity(ship).insert(ShipDesignSections(sections));
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
