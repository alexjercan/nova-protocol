//! The keybind hint surface: which flight verbs are available right now and
//! what key each is bound to, resolved from the live rig for the HUD.

use bevy::prelude::*;
use bevy_enhanced_input::prelude::*;
use nova_gameplay::prelude::*;
use nova_input::prelude::*;

use super::flight_rig::AutopilotStopInput;
use crate::{
    flight::{
        ship_capabilities, ship_has_attitude_authority, LiveFlightComputers, ShipCapabilityQuery,
    },
    prelude::*,
};

/// One flight verb's hint state, for the keybind-hint HUD.
#[derive(Clone, Debug, Default, PartialEq, Reflect)]
pub struct VerbHint {
    /// The verb's keycap label ("X", "G"...), read from the LIVE bindings
    /// table; empty until the flight rig exists.
    pub key: String,
    /// Whether pressing the key right now would do something.
    pub available: bool,
    /// The world entity the verb would act on (the aim lock for GOTO, the
    /// dominant well for ORBIT), for hints anchored on the object itself.
    pub anchor: Option<Entity>,
}

/// Optional playtest flag (adversarial round NIT): deny the fire PRESS while
/// the radar search is held, so sweeping with the trigger down cannot rake
/// bystanders. Off by default - manual gunnery during a search is a player
/// freedom until playtest says otherwise.
pub(super) const HOLD_FIRE_DURING_RADAR: bool = false;

/// The player's currently available flight verbs, resolved every frame by
/// `update_flight_verb_hints` - computed here, where the verbs and their
/// (private) input actions live; the HUD renders it dumb. Keyboard labels
/// only in v1 (device awareness is a recorded open question).
#[derive(Resource, Clone, Debug, Default, PartialEq, Reflect)]
#[reflect(Resource)]
pub struct FlightVerbHints {
    /// The STOP verb hint (flip retrograde and burn to rest).
    pub stop: VerbHint,
    /// The GOTO verb hint (fly to the current nav lock).
    pub goto: VerbHint,
    /// The ORBIT verb hint (park into orbit around a gravity well).
    pub orbit: VerbHint,
    /// The CANCEL verb hint (disengage the autopilot, resume manual).
    pub cancel: VerbHint,
    /// The radar gesture (hold = radar, tap = clear), labelled off
    /// `radar_hold`; available while the computer grants Lock.
    pub radar: VerbHint,
    /// The DOCK verb hint (capture the locked ship with a docking port).
    /// Available only while a port pair on the two hulls would actually be
    /// accepted right now, so the chip is the offer and not an invitation to
    /// press a key that refuses.
    pub dock: VerbHint,
    /// The RCS fine-adjust modifier, labelled off `rcs_modifier`; available
    /// while the computer grants the `Rcs` verb, so the row shows only where
    /// RCS is enabled - the same opt-out the mainline campaign uses while RCS
    /// is off pending rework.
    pub rcs: VerbHint,
    /// The HELM verb hint (take or hand back a docked pair's helm), labelled
    /// off `dock_helm`; available only while docked, and only to hand the
    /// helm back while the pair has a [`helm_fault`](Self::helm_fault).
    pub helm: VerbHint,
    /// Whether the player holds its docked pair's helm, so the HELM chip
    /// paints hot: pressing it hands the helm back.
    pub helm_held: bool,
    /// Whether the docked pair cannot be measured
    /// ([`DockingConnection::measurement_fault`]), so HELM refuses to take
    /// it. The HELM chip stays on the dock, dark, under the mode chip's
    /// `HELM FAULT`, unless the player holds the helm and can still hand it
    /// back.
    pub helm_fault: bool,
    /// Whether a living station holds the docked pair's helm
    /// ([`DockedHelmType::Station`]), so HELM is refused outright. The HELM
    /// chip stays on the dock, dark, under the mode chip's `STATION HELM`.
    pub helm_station: bool,
    /// The line the HUD posts once as a notice when the travel lock settles
    /// on a station that refuses a dock ([`DockRefusalType`]), naming the
    /// station: `<NAME> REFUSES DOCKING` for a living Enemy or hostile
    /// station, `<NAME> DOCK OCCUPIED` for one already holding its connection. `None` while the
    /// lock is clear, on a ship, or on a station that admits; a ship's
    /// refusal keeps the dark chip it always had.
    pub dock_refusal: Option<String>,
    /// Whether any maneuver is engaged right now - explicit, so consumers
    /// (the GOTO cue hides mid-maneuver) do not have to proxy it through
    /// another verb's availability.
    pub engaged: bool,
}

/// The HUD notice line for a station's refusal, or `None` for a ship's, which
/// stays a dark chip: a ship under fire is a situation the player can read off the
/// world, a station's side is not.
pub fn dock_refusal_line(name: &str, refusal: DockRefusalType) -> Option<String> {
    match refusal {
        DockRefusalType::Combat => None,
        DockRefusalType::EnemyStation | DockRefusalType::HostileStation => {
            Some(format!("{name} REFUSES DOCKING"))
        }
        DockRefusalType::StationOccupied => Some(format!("{name} DOCK OCCUPIED")),
    }
}

/// Resolve the verb hints from the live world: availability from the same
/// state the input observers AND the autopilot gate on (lock, dominant
/// well, engagement, and a flyable ship - a live flight computer plus at
/// least one live engine, else autopilot_system strips the maneuver on its
/// next tick and a lit hint would be a lie), labels from the LIVE bindings
/// table so a rebind cannot desync the hints.
#[expect(clippy::type_complexity, reason = "one query per private action type")]
pub(super) fn update_flight_verb_hints(
    mut hints: ResMut<FlightVerbHints>,
    q_ship: Query<
        (
            Entity,
            Option<&Autopilot>,
            Option<&DominantWell>,
            Option<&TravelLock>,
            Option<&DockedShip>,
        ),
        With<PlayerSpaceshipMarker>,
    >,
    q_connections: Query<&DockingConnection>,
    q_computer: LiveFlightComputers,
    q_capabilities: ShipCapabilityQuery,
    q_thruster: Query<&ChildOf, (With<ThrusterSectionMarker>, Without<SectionInactiveMarker>)>,
    q_rig: Query<(), With<Action<AutopilotStopInput>>>,
    q_names: Query<&Name>,
    ports: DockingPorts,
    admission: DockAdmission,
    bindings: Option<Res<InputBindings>>,
) {
    // The rig is the gate, not the source: a row is drawn only while the rig
    // that answers it exists, so all eight vanish together on a ship with no
    // flight computer.
    let rig_exists = !q_rig.is_empty();
    // The keycap an action draws, off the LIVE table. Reading the rig's own
    // `Bindings` matched KEYBOARD entries only, so a verb moved onto a mouse
    // button still fired and lost its chip with no way back except rebinding
    // to a key.
    let label = |action: &str| -> String {
        if !rig_exists {
            return String::new();
        }
        bindings
            .as_deref()
            .and_then(|table| table.get(action))
            .and_then(|action| action.sources().next())
            .map(|source| source.glyph_label())
            .unwrap_or_default()
    };

    // Exactly one player ship, same rule as the Single-based observers.
    let (ship, autopilot, dominant, travel, docked) = match q_ship.single() {
        Ok((entity, autopilot, dominant, travel, docked)) => {
            (Some(entity), autopilot, dominant, travel, docked)
        }
        Err(_) => (None, None, None, None, None),
    };
    let connection = docked.and_then(|docked| q_connections.get(docked.connection).ok());
    let helm_held = ship.is_some_and(|ship| {
        connection.is_some_and(|connection| connection.helm == DockedHelmType::Held(ship))
    });
    let helm_fault = connection.is_some_and(|connection| connection.measurement_fault);
    let helm_station =
        connection.is_some_and(|connection| matches!(connection.helm, DockedHelmType::Station(_)));
    // Read off `drives`, the flag every flight writer gates on, not off the
    // helm: the helm changes at once, `drives` at the next fixed tick, and a
    // held pair that cannot be measured never drives at all.
    let docked_without_helm = docked.is_some_and(|docked| !docked.drives);
    let travel = travel.and_then(|travel| travel.0);
    // The partner flies with the pair, so GOTO has nothing to close on it.
    let partner_lock =
        travel.is_some_and(|target| connection.is_some_and(|connection| connection.joins(target)));
    // The autopilot needs a live flight computer and at least one live
    // engine or it disengages on its next tick; a hint below that bar
    // would light a key that visibly does nothing.
    let hardware = ship.is_some_and(|ship| {
        ship_has_attitude_authority(ship, &q_computer)
            && q_thruster.iter().any(|&ChildOf(parent)| parent == ship)
    });
    // The individual maneuvers are the SHIP's own capabilities, read from the
    // same root the input observers read, so a lit hint and a firing key can
    // never disagree. Kept SEPARATE from `hardware` above (which only asks "is
    // there a live controller + engine"): the two answer different questions -
    // hardware versus permission - and folding them would let a hulk's dead
    // attitude loop silently erase what the ship was configured to do. The
    // `SetShipCapability*` actions flip these.
    let capabilities = ship
        .map(|ship| ship_capabilities(ship, &q_capabilities))
        .unwrap_or_default();
    let engaged = autopilot.is_some();
    let orbiting = matches!(
        autopilot.map(|ap| ap.action),
        Some(AutopilotAction::Orbit { .. })
    );
    // A docked hull that does not drive its pair flies nothing: the input
    // observers refuse its drive, trim and maneuvers until it drives again.
    let helm_gate = |ready: bool| VerbHint {
        available: ready && !docked_without_helm,
        ..default()
    };
    // The station's answer, read off the same admission the chip and the
    // request read, so the line and the dark chip can never disagree.
    let dock_refusal = match (ship, travel) {
        (Some(ship), Some(target)) if docked.is_none() => {
            admission.refusal(ship, target).and_then(|refusal| {
                let name = q_names.get(target).map_or("", |name| name.as_str());
                dock_refusal_line(name, refusal)
            })
        }
        _ => None,
    };

    let next = FlightVerbHints {
        stop: VerbHint {
            key: label("autopilot_stop"),
            ..helm_gate(hardware && capabilities.stop_enabled)
        },
        goto: VerbHint {
            key: label("autopilot_goto"),
            anchor: travel,
            ..helm_gate(hardware && capabilities.goto_enabled && travel.is_some() && !partner_lock)
        },
        orbit: VerbHint {
            key: label("autopilot_orbit"),
            anchor: dominant.map(|well| **well),
            ..helm_gate(hardware && capabilities.orbit_enabled && dominant.is_some() && !orbiting)
        },
        cancel: VerbHint {
            key: label("autopilot_off"),
            // Z always answers while engaged, even on a crippled ship.
            available: engaged,
            ..default()
        },
        radar: VerbHint {
            key: label("radar_hold"),
            available: capabilities.lock_enabled,
            ..default()
        },
        // The one verb whose availability is the real answer: the search that
        // lights the chip is the search the command runs, so a lit DOCK means
        // a pair exists on these two hulls at this instant. While the hull IS
        // docked the same key undocks, so the chip stays lit - and the HUD
        // draws it inverted, because that is the state the key would leave.
        dock: VerbHint {
            key: label("dock"),
            available: docked.is_some()
                || ship.is_some_and(|ship| {
                    capabilities.dock_enabled
                        && travel.is_some_and(|target| {
                            admission.admits(ship, target)
                                && ports.best_candidate(ship, target).is_some()
                        })
                }),
            anchor: travel,
            ..default()
        },
        rcs: VerbHint {
            // Shown only while the computer grants RCS, and only to the pair's
            // driver while docked.
            key: label("rcs_modifier"),
            ..helm_gate(capabilities.rcs_enabled)
        },
        helm: VerbHint {
            key: label("dock_helm"),
            // A fault refuses taking the helm, never handing it back; a
            // living station refuses both.
            available: docked.is_some() && !helm_station && (helm_held || !helm_fault),
            ..default()
        },
        helm_held,
        helm_fault,
        helm_station,
        dock_refusal,
        engaged,
    };
    // set_if_neq semantics by hand: only dirty the resource on real change.
    if *hints != next {
        *hints = next;
    }
}

#[cfg(test)]
mod tests {
    use bevy::ecs::system::RunSystemOnce;

    use super::*;
    use crate::input::player::test_support::{hint_world, spawn_flyable_ship};

    #[test]
    fn verb_hints_derive_labels_from_the_live_bindings() {
        let mut world = hint_world();
        spawn_flyable_ship(&mut world);

        world.run_system_once(update_flight_verb_hints).unwrap();

        let hints = world.resource::<FlightVerbHints>();
        // The keyboard binding wins even with a gamepad binding first in
        // line; "Key" prefixes are stripped for chip-sized labels.
        assert_eq!(hints.stop.key, "X");
        assert_eq!(hints.goto.key, "G");
        assert_eq!(hints.orbit.key, "O");
        assert_eq!(hints.cancel.key, "Z");
    }

    /// The dock follows a rebind. It read the rig's own `Bindings` before, so
    /// a row the player had moved kept drawing the old keycap.
    #[test]
    fn a_rebound_verb_redraws_the_dock_on_the_new_key() {
        let mut world = hint_world();
        spawn_flyable_ship(&mut world);
        world.resource_mut::<InputBindings>().rebind(
            "radar_hold",
            BindingSpec {
                keyboard: vec![InputSource::from(KeyCode::KeyK)],
                gamepad: vec![],
            },
        );

        world.run_system_once(update_flight_verb_hints).unwrap();

        assert_eq!(world.resource::<FlightVerbHints>().radar.key, "K");
    }

    /// A verb moved onto a mouse button still fires, so its chip must still be
    /// drawn: the old reader matched keyboard entries only and left the player
    /// no way back except rebinding blind.
    #[test]
    fn a_verb_on_a_mouse_button_keeps_its_chip() {
        let mut world = hint_world();
        spawn_flyable_ship(&mut world);
        world.resource_mut::<InputBindings>().rebind(
            "autopilot_goto",
            BindingSpec {
                keyboard: vec![InputSource::from(MouseButton::Middle)],
                gamepad: vec![],
            },
        );

        world.run_system_once(update_flight_verb_hints).unwrap();

        assert_eq!(world.resource::<FlightVerbHints>().goto.key, "MMB");
    }

    /// The RCS hint carries the fixed "SHIFT" label and is available only while
    /// the ROOT enables RCS - so the cluster row shows only when RCS is on (the
    /// mainline campaign, which turns it off, never shows it).
    #[test]
    fn rcs_hint_shows_shift_only_when_the_capability_is_on() {
        let mut world = hint_world();
        let (ship, _controller) = spawn_flyable_ship(&mut world);

        world.run_system_once(update_flight_verb_hints).unwrap();
        let hints = world.resource::<FlightVerbHints>();
        assert_eq!(hints.rcs.key, "ShiftLeft");
        assert!(hints.rcs.available, "enabled RCS lights the SHIFT hint");

        // Turn RCS off (the mainline path): the hint goes unavailable and the
        // renderer drops the row.
        world.entity_mut(ship).insert(ShipCapabilities {
            rcs_enabled: false,
            ..default()
        });
        world.run_system_once(update_flight_verb_hints).unwrap();
        assert!(
            !world.resource::<FlightVerbHints>().rcs.available,
            "RCS off hides the SHIFT hint"
        );
    }

    #[test]
    fn verb_hints_track_lock_well_and_engagement() {
        let mut world = hint_world();
        let (ship, controller) = spawn_flyable_ship(&mut world);

        // Flyable ship in flat space: STOP only.
        world.run_system_once(update_flight_verb_hints).unwrap();
        let hints = world.resource::<FlightVerbHints>().clone();
        assert!(hints.stop.available);
        assert!(!hints.goto.available && !hints.orbit.available && !hints.cancel.available);

        // A lock offers GOTO and anchors it; a dominant well offers ORBIT.
        let lock = world.spawn_empty().id();
        let well = world.spawn_empty().id();
        world
            .entity_mut(ship)
            .insert((TravelLock(Some(lock)), DominantWell(well)));
        world.run_system_once(update_flight_verb_hints).unwrap();
        let hints = world.resource::<FlightVerbHints>().clone();
        assert!(hints.goto.available);
        assert_eq!(hints.goto.anchor, Some(lock));
        assert!(hints.orbit.available);
        assert_eq!(hints.orbit.anchor, Some(well));

        // Orbiting retires the ORBIT offer and arms CANCEL.
        world
            .entity_mut(ship)
            .insert(Autopilot::engage(AutopilotAction::Orbit {
                well,
                plan: None,
            }));
        world.run_system_once(update_flight_verb_hints).unwrap();
        let hints = world.resource::<FlightVerbHints>().clone();
        assert!(!hints.orbit.available, "already orbiting");
        assert!(hints.cancel.available);
        assert!(hints.engaged);

        // A dead flight computer grounds every verb except CANCEL: the
        // autopilot would strip the maneuver on its next tick, so a lit hint
        // would be a lie.
        world.entity_mut(controller).insert(SectionInactiveMarker);
        world.run_system_once(update_flight_verb_hints).unwrap();
        let hints = world.resource::<FlightVerbHints>().clone();
        assert!(!hints.stop.available, "no computer, no STOP");
        assert!(!hints.goto.available && !hints.orbit.available);
        assert!(hints.cancel.available, "Z still answers while engaged");
        world
            .entity_mut(controller)
            .remove::<SectionInactiveMarker>();

        // No player ship at all: nothing is available, labels remain.
        world.entity_mut(ship).despawn();
        world.run_system_once(update_flight_verb_hints).unwrap();
        let hints = world.resource::<FlightVerbHints>().clone();
        assert!(!hints.stop.available && !hints.cancel.available);
        assert_eq!(hints.stop.key, "X", "labels survive the ship");
    }

    #[test]
    fn root_capabilities_gate_the_hints_independently_of_lock_and_well() {
        let mut world = hint_world();
        let (ship, _controller) = spawn_flyable_ship(&mut world);

        // A lock and a dominant well are present, so absent the capabilities
        // GOTO and ORBIT would both light (as the neighbor test proves).
        let lock = world.spawn_empty().id();
        let well = world.spawn_empty().id();
        world
            .entity_mut(ship)
            .insert((TravelLock(Some(lock)), DominantWell(well)));

        // Turn GOTO and ORBIT off on the root; STOP stays on.
        world.entity_mut(ship).insert(ShipCapabilities {
            goto_enabled: false,
            orbit_enabled: false,
            ..default()
        });
        world.run_system_once(update_flight_verb_hints).unwrap();
        let hints = world.resource::<FlightVerbHints>().clone();
        assert!(hints.stop.available, "STOP is still on");
        assert!(
            !hints.goto.available,
            "GOTO off on the root despite a live lock"
        );
        assert!(
            !hints.orbit.available,
            "ORBIT off on the root despite a dominant well"
        );

        // Turning them back on lights both (the lock/well are unchanged) -
        // proves the capabilities, not some other condition, were the gate.
        world.entity_mut(ship).insert(ShipCapabilities::default());
        world.run_system_once(update_flight_verb_hints).unwrap();
        let hints = world.resource::<FlightVerbHints>().clone();
        assert!(hints.goto.available, "GOTO lights once enabled");
        assert!(hints.orbit.available, "ORBIT lights once enabled");
    }

    #[test]
    fn a_docked_hull_flies_nothing_until_it_drives_the_pair() {
        let mut world = hint_world();
        let (ship, _controller) = spawn_flyable_ship(&mut world);
        let lock = world.spawn_empty().id();
        let well = world.spawn_empty().id();
        let partner = world.spawn(SpaceshipRootMarker).id();
        let ports = [(); 2].map(|_| world.spawn_empty().id());
        let connection = world
            .spawn(DockingConnection {
                first_ship: ship,
                first_section: ports[0],
                second_ship: partner,
                second_section: ports[1],
                helm: DockedHelmType::Neutral,
                measurement_fault: false,
            })
            .id();
        world.entity_mut(ship).insert((
            TravelLock(Some(lock)),
            DominantWell(well),
            DockedShip {
                connection,
                helm: Quat::IDENTITY,
                drives: false,
            },
        ));
        let flies = |world: &World| {
            let hints = world.resource::<FlightVerbHints>();
            [&hints.stop, &hints.goto, &hints.orbit, &hints.rcs].map(|hint| hint.available)
        };

        world.run_system_once(update_flight_verb_hints).unwrap();
        assert_eq!(flies(&world), [false; 4], "nothing flies off the helm");

        // The helm is taken at once, but the pair drives only from the next
        // fixed tick, and never while it cannot be measured.
        world.get_mut::<DockingConnection>(connection).unwrap().helm = DockedHelmType::Held(ship);
        world.run_system_once(update_flight_verb_hints).unwrap();
        assert_eq!(
            flies(&world),
            [false; 4],
            "a held helm that does not drive yet flies nothing"
        );

        world.get_mut::<DockedShip>(ship).unwrap().drives = true;
        world.run_system_once(update_flight_verb_hints).unwrap();
        assert_eq!(flies(&world), [true; 4], "the helm gives them back");

        // A fault leaves the held helm free to hand back and refuses taking
        // it.
        let helm = |world: &World| {
            let hints = world.resource::<FlightVerbHints>();
            (hints.helm.available, hints.helm_fault)
        };
        world
            .get_mut::<DockingConnection>(connection)
            .unwrap()
            .measurement_fault = true;
        world.get_mut::<DockedShip>(ship).unwrap().drives = false;
        world.run_system_once(update_flight_verb_hints).unwrap();
        assert_eq!(helm(&world), (true, true), "held: release it");
        assert_eq!(flies(&world), [false; 4], "nothing flies through a fault");
        world.get_mut::<DockingConnection>(connection).unwrap().helm = DockedHelmType::Neutral;
        world.run_system_once(update_flight_verb_hints).unwrap();
        assert_eq!(helm(&world), (false, true), "neutral: not taken");

        world
            .get_mut::<DockingConnection>(connection)
            .unwrap()
            .measurement_fault = false;
        world.run_system_once(update_flight_verb_hints).unwrap();
        assert_eq!(helm(&world), (true, false), "measured: take it");
    }
}
