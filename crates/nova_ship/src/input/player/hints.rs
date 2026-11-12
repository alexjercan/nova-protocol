//! The keybind hint surface: which flight verbs are available right now and
//! what key each is bound to, resolved from the live rig for the HUD.

use std::ops::Deref;

use avian3d::prelude::LinearVelocity;
use bevy::{ecs::system::SystemParam, prelude::*};
use bevy_enhanced_input::prelude::*;
use nova_gameplay::prelude::*;
use nova_input::prelude::*;

use super::{
    flight_rig::{AutopilotStopInput, FlightBurnInput, RcsModifierInput},
    weapons::{
        SpaceshipMiningInputBinding, SpaceshipRailgunInputBinding, SpaceshipTorpedoInputBinding,
        SpaceshipTurretInputBinding,
    },
};
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
    /// The label of the button held first when the verb is a two-button chord
    /// on the device pressed last: `Pad Left Thumb` for DOCK on a gamepad,
    /// where [`key`](Self::key) is then the second button. `None` for a verb
    /// one button answers.
    pub modifier: Option<String>,
    /// Whether the verb applies to the situation, so the dock draws its chip:
    /// a GOTO with a lock to fly to, a DOCK with a port in reach, a FIRE with
    /// the weapons hot. A shown verb may still be unavailable (the chip draws
    /// dim); a verb that is not shown leaves the row.
    pub shown: bool,
    /// Whether pressing the key right now would do something.
    pub available: bool,
    /// The world entity the verb would act on (the aim lock for GOTO, the
    /// dominant well for ORBIT), for hints anchored on the object itself.
    pub anchor: Option<Entity>,
}

/// Which of its two verbs the STOP chip stands for.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Reflect)]
pub enum StopVerbType {
    /// Flip retrograde and burn to rest.
    #[default]
    Stop,
    /// Match the travel lock's velocity on the RCS: the RCS modifier is held.
    Match,
}

/// Optional playtest flag (adversarial round NIT): deny the fire PRESS while
/// the radar search is held, so sweeping with the trigger down cannot rake
/// bystanders. Off by default - manual gunnery during a search is a player
/// freedom until playtest says otherwise.
pub(super) const HOLD_FIRE_DURING_RADAR: bool = false;

/// The player's currently available flight verbs, resolved every frame by
/// `update_flight_verb_hints` - computed here, where the verbs and their
/// (private) input actions live; the HUD renders it dumb. Labels follow the
/// device the player pressed last ([`InputDeviceType`]).
#[derive(Resource, Clone, Debug, Default, PartialEq, Reflect)]
#[reflect(Resource)]
pub struct FlightVerbHints {
    /// The INTERFACE verb hint (open the interface), labelled off
    /// `interface_toggle`; shown and available while the flight rig exists.
    pub interface: VerbHint,
    /// The DRIVE verb hint (the main-drive burn), labelled off `main_drive`;
    /// available while a live thruster answers it and the hull flies.
    pub drive: VerbHint,
    /// Whether the main-drive burn is held, so the DRIVE chip paints hot.
    pub drive_held: bool,
    /// The hint for what the STOP key does now (flip retrograde and burn to
    /// rest). While the RCS modifier is held ([`rcs_held`](Self::rcs_held)) it
    /// is the MATCH chord's hint, gated like the chord.
    pub stop: VerbHint,
    /// The word the STOP chip reads, named by the press and nothing else:
    /// MATCH while the RCS modifier is held, else STOP. A flying match does
    /// not change it, because the plain key then brakes.
    pub stop_verb: StopVerbType,
    /// Whether the maneuver the chip's word names is flying now, so the same
    /// press disengages it and the chip paints hot. STOP word: the engaged
    /// action is `Stop`. MATCH word: the engaged action is `MatchTarget`.
    pub stop_flying: bool,
    /// Whether the RCS modifier is held, so the RCS chip paints hot.
    pub rcs_held: bool,
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
    /// station, `<NAME> REFUSES CONTRABAND` for a lawful one while the hold
    /// carries contraband, `<NAME> DOCK OCCUPIED` for one already holding
    /// its connection. `None` while the
    /// lock is clear, on a ship, or on a station that admits; a ship's
    /// refusal keeps the dark chip it always had.
    pub dock_refusal: Option<String>,
    /// The MINE verb hint (hold a mining emitter out), keyed off the first
    /// mining section's own [`SpaceshipMiningInputBinding`]; shown while the
    /// ship carries a bound mining section.
    pub mine: VerbHint,
    /// The AIM verb hint (raise the weapons), labelled off `combat_stance`;
    /// shown while the ship carries a turret, torpedo bay or railgun.
    pub aim: VerbHint,
    /// The FIRE verb hint, keyed off the first turret's own
    /// [`SpaceshipTurretInputBinding`]; shown while the weapons are hot and a
    /// turret holds a binding.
    pub fire: VerbHint,
    /// The TORPEDO verb hint, keyed off the first torpedo bay's own
    /// [`SpaceshipTorpedoInputBinding`]; shown on the same terms as FIRE.
    pub torpedo: VerbHint,
    /// The RAILGUN verb hint, keyed off the first railgun's own
    /// [`SpaceshipRailgunInputBinding`]; shown on the same terms as FIRE.
    pub railgun: VerbHint,
    /// Whether any maneuver is engaged right now - explicit, so consumers
    /// (the GOTO cue hides mid-maneuver) do not have to proxy it through
    /// another verb's availability.
    pub engaged: bool,
}

/// The player ship's sections the situational and weapon chips read: which
/// kinds the hull carries, the key each section binds, the docking ports a
/// travel lock may carry, the weapons safety and the held main drive.
#[derive(SystemParam)]
pub(super) struct SectionHintQuery<'w, 's> {
    turrets: Query<
        'w,
        's,
        (
            &'static ChildOf,
            Option<&'static SpaceshipTurretInputBinding>,
        ),
        With<TurretSectionMarker>,
    >,
    torpedoes: Query<
        'w,
        's,
        (
            &'static ChildOf,
            Option<&'static SpaceshipTorpedoInputBinding>,
        ),
        With<TorpedoSectionMarker>,
    >,
    railguns: Query<
        'w,
        's,
        (
            &'static ChildOf,
            Option<&'static SpaceshipRailgunInputBinding>,
        ),
        With<RailgunSectionMarker>,
    >,
    miners: Query<
        'w,
        's,
        (
            &'static ChildOf,
            Option<&'static SpaceshipMiningInputBinding>,
        ),
        With<MiningSectionMarker>,
    >,
    /// Live docking ports, held or free: a lock on a hull that carries one
    /// is a DOCK the gates may still refuse, which is the dim chip.
    docking: Query<
        'w,
        's,
        &'static ChildOf,
        (With<DockingSectionMarker>, Without<SectionInactiveMarker>),
    >,
    hot: Query<'w, 's, &'static WeaponsHot>,
    burn: Query<'w, 's, &'static TriggerState, With<Action<FlightBurnInput>>>,
}

/// The keycap a section binding draws: the first source on the device the
/// player pressed last, else the first source at all, so a mining key with no
/// pad button keeps its key the way a rig verb does.
fn section_key(sources: &[InputSource], device: InputDeviceType) -> Option<String> {
    let on_device = |source: &&InputSource| {
        matches!(source, InputSource::Gamepad(_)) == (device == InputDeviceType::Pad)
    };
    sources
        .iter()
        .find(on_device)
        .or(sources.first())
        .map(|source| source.glyph_label())
}

/// Whether `ship` carries a section of the kind, and the keycap of the first
/// one that holds a binding.
fn section_hint<'a, B: Deref<Target = Vec<InputSource>> + 'a>(
    sections: impl Iterator<Item = (&'a ChildOf, Option<&'a B>)>,
    ship: Option<Entity>,
    device: InputDeviceType,
) -> (bool, Option<String>) {
    let mut carried = false;
    let mut key = None;
    for (&ChildOf(parent), binding) in sections {
        if Some(parent) != ship {
            continue;
        }
        carried = true;
        if key.is_none() {
            key = binding.and_then(|binding| section_key(binding, device));
        }
    }
    (carried, key)
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
        DockRefusalType::Contraband => Some(format!("{name} REFUSES CONTRABAND")),
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
    q_modifier: Query<&TriggerState, With<Action<RcsModifierInput>>>,
    q_budget: Query<&RcsBudget>,
    q_motion: Query<(), With<LinearVelocity>>,
    q_names: Query<&Name>,
    ports: DockingPorts,
    admission: DockAdmission,
    sections: SectionHintQuery,
    bindings: Option<Res<InputBindings>>,
    device: Res<InputDeviceType>,
) {
    // The rig is the gate, not the source: a row is drawn only while the rig
    // that answers it exists, so every chip vanishes together on a ship with
    // no flight computer.
    let rig_exists = !q_rig.is_empty();
    // The keycap an action draws, off the LIVE table: the source on the device
    // the player pressed last, else the other device's. Reading the rig's own
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
            .and_then(|action| action.hint_source(*device))
            .map(|source| source.glyph_label())
            .unwrap_or_default()
    };

    // DOCK and HELM hold no pad button of their own: on a pad each is the RCS
    // modifier plus the button of the verb it rides (`on_autopilot_goto_input`,
    // `on_autopilot_orbit_input`). A verb that ships a pad button, or a table
    // that lacks either half of the chord, draws the one-button label.
    let chord = |action: &str, rides: &str| -> (String, Option<String>) {
        let pad = |name: &str| {
            bindings
                .as_deref()
                .and_then(|table| table.get(name))
                .and_then(|action| action.gamepad.first())
                .map(|source| source.glyph_label())
        };
        if rig_exists && *device == InputDeviceType::Pad && pad(action).is_none() {
            if let (Some(modifier), Some(button)) = (pad("rcs_modifier"), pad(rides)) {
                return (button, Some(modifier));
            }
        }
        (label(action), None)
    };
    let (dock_key, dock_modifier) = chord("dock", "autopilot_goto");
    let (helm_key, helm_modifier) = chord("dock_helm", "autopilot_orbit");

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
    // would light a key that visibly does nothing. The burn needs only the
    // engine.
    let thrusters =
        ship.is_some_and(|ship| q_thruster.iter().any(|&ChildOf(parent)| parent == ship));
    let hardware =
        thrusters && ship.is_some_and(|ship| ship_has_attitude_authority(ship, &q_computer));
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
    let matching = matches!(
        autopilot.map(|ap| ap.action),
        Some(AutopilotAction::MatchTarget { .. })
    );
    let rcs_held = crate::camera::action_held(&q_modifier);
    let drive_held = crate::camera::action_held(&sections.burn);
    let stopping = matches!(autopilot.map(|ap| ap.action), Some(AutopilotAction::Stop));
    let (stop_verb, stop_flying) = if rcs_held {
        (StopVerbType::Match, matching)
    } else {
        (StopVerbType::Stop, stopping)
    };
    // A ship with no RCS budget counts as empty: the quads have nothing to
    // push with, so the RCS chip and the MATCH chord both go dim.
    let magazine_filled = ship
        .and_then(|ship| q_budget.get(ship).ok())
        .is_some_and(|budget| !budget.is_empty());
    // The MATCH branch of `on_autopilot_stop_input`, in its order, so a lit
    // chip and a firing chord cannot disagree. It does not ask for `hardware`.
    let match_available = !docked_without_helm
        && (matching
            || (capabilities.stop_enabled
                && travel.is_some_and(|target| q_motion.contains(target))
                && !partner_lock
                && capabilities.rcs_enabled
                && magazine_filled));
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
    // The weapon chips key off the sections' own bindings, so a section
    // rebound in the Ship pane redraws its chip the way a rig rebind does.
    let section_label = |key: Option<String>| {
        if rig_exists {
            key.unwrap_or_default()
        } else {
            String::new()
        }
    };
    let (turret, turret_key) = section_hint(sections.turrets.iter(), ship, *device);
    let (torpedo_bay, torpedo_key) = section_hint(sections.torpedoes.iter(), ship, *device);
    let (railgun, railgun_key) = section_hint(sections.railguns.iter(), ship, *device);
    let (_, mine_key) = section_hint(sections.miners.iter(), ship, *device);
    let weapons_hot = ship
        .and_then(|ship| sections.hot.get(ship).ok())
        .is_some_and(|hot| hot.0);
    // A lock on a hull with a port is a DOCK worth showing before the gates
    // pass: the chip is dim through the approach and lights when they do.
    let lock_has_port = travel.is_some_and(|target| {
        sections
            .docking
            .iter()
            .any(|&ChildOf(parent)| parent == target)
    });
    // A weapon chip is on the row while the weapons are hot and its section
    // holds a key; the press then fires, so shown and available agree.
    let weapon = |key: String| {
        let shown = weapons_hot && !key.is_empty();
        VerbHint {
            key,
            shown,
            available: shown,
            ..default()
        }
    };

    let next = FlightVerbHints {
        interface: VerbHint {
            key: label("interface_toggle"),
            shown: rig_exists,
            available: rig_exists,
            ..default()
        },
        drive: VerbHint {
            key: label("main_drive"),
            shown: rig_exists,
            ..helm_gate(thrusters)
        },
        drive_held,
        stop: VerbHint {
            key: label("autopilot_stop"),
            shown: rig_exists,
            // The gate of what this press would do. The chord needs the
            // modifier down; with it up the key is plain STOP, also over a
            // flying match, which it replaces with a STOP burn.
            available: if rcs_held {
                match_available
            } else {
                helm_gate(hardware && capabilities.stop_enabled).available
            },
            ..default()
        },
        stop_verb,
        stop_flying,
        rcs_held,
        goto: VerbHint {
            key: label("autopilot_goto"),
            // The partner flies with the pair: no lock to fly to, no chip.
            shown: travel.is_some() && !partner_lock,
            anchor: travel,
            // With the modifier down the press is the DOCK chord, never GOTO.
            ..helm_gate(
                !rcs_held
                    && hardware
                    && capabilities.goto_enabled
                    && travel.is_some()
                    && !partner_lock,
            )
        },
        orbit: VerbHint {
            key: label("autopilot_orbit"),
            shown: dominant.is_some() || orbiting,
            anchor: dominant.map(|well| **well),
            // With the modifier down the press is the HELM chord, never ORBIT.
            ..helm_gate(
                !rcs_held
                    && hardware
                    && capabilities.orbit_enabled
                    && dominant.is_some()
                    && !orbiting,
            )
        },
        cancel: VerbHint {
            key: label("autopilot_off"),
            // Z always answers while engaged, even on a crippled ship.
            shown: engaged,
            available: engaged,
            ..default()
        },
        radar: VerbHint {
            key: label("radar_hold"),
            shown: rig_exists,
            available: capabilities.lock_enabled,
            ..default()
        },
        // The one verb whose availability is the real answer: the search that
        // lights the chip is the search the command runs, so a lit DOCK means
        // a pair exists on these two hulls at this instant. While the hull IS
        // docked the same key undocks, so the chip stays lit - and the HUD
        // draws it inverted, because that is the state the key would leave.
        dock: VerbHint {
            key: dock_key,
            modifier: dock_modifier,
            shown: docked.is_some() || lock_has_port,
            available: docked.is_some()
                || ship.is_some_and(|ship| {
                    capabilities.dock_enabled
                        && travel.is_some_and(|target| {
                            admission.admits(ship, target)
                                && ports.best_candidate(ship, target).is_some()
                        })
                }),
            anchor: travel,
        },
        rcs: VerbHint {
            // Lit only while the computer grants RCS, the magazine has
            // something in it, and only to the pair's driver while docked.
            key: label("rcs_modifier"),
            shown: rig_exists,
            ..helm_gate(capabilities.rcs_enabled && magazine_filled)
        },
        helm: VerbHint {
            key: helm_key,
            modifier: helm_modifier,
            shown: docked.is_some(),
            // A fault refuses taking the helm, never handing it back; a
            // living station refuses both.
            available: docked.is_some() && !helm_station && (helm_held || !helm_fault),
            anchor: None,
        },
        helm_held,
        helm_fault,
        helm_station,
        dock_refusal,
        mine: {
            let key = section_label(mine_key);
            let shown = !key.is_empty();
            VerbHint {
                key,
                shown,
                available: shown,
                ..default()
            }
        },
        aim: VerbHint {
            key: label("combat_stance"),
            shown: turret || torpedo_bay || railgun,
            available: turret || torpedo_bay || railgun,
            ..default()
        },
        fire: weapon(section_label(turret_key)),
        torpedo: weapon(section_label(torpedo_key)),
        railgun: weapon(section_label(railgun_key)),
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
    use crate::input::{
        bindings::flight_bindings,
        player::test_support::{hint_world, spawn_flyable_ship, spawn_rcs_modifier},
    };

    #[test]
    fn verb_hints_derive_labels_from_the_live_bindings() {
        let mut world = hint_world();
        spawn_flyable_ship(&mut world);

        world.run_system_once(update_flight_verb_hints).unwrap();

        let hints = world.resource::<FlightVerbHints>();
        // On the desk the keyboard binding wins even with a gamepad binding
        // first in line; "Key" prefixes are stripped for chip-sized labels.
        assert_eq!(hints.stop.key, "X");
        assert_eq!(hints.goto.key, "G");
        assert_eq!(hints.orbit.key, "O");
        assert_eq!(hints.cancel.key, "Z");
    }

    /// DOCK and HELM hold no pad button, so on a pad each hint is a chord: the
    /// RCS modifier, then the button of the verb the chord rides.
    #[test]
    fn a_pad_press_moves_the_verb_labels_to_the_pad_buttons() {
        let mut world = hint_world();
        spawn_flyable_ship(&mut world);
        let table = InputBindings::from_actions(flight_bindings());
        let pad = |name: &str| {
            table
                .get(name)
                .and_then(|action| action.gamepad.first())
                .map(|source| source.glyph_label())
        };
        assert_eq!(pad("dock"), None, "DOCK holds no pad button of its own");
        assert_eq!(pad("dock_helm"), None, "nor does HELM");
        let modifier = pad("rcs_modifier");
        assert_eq!(modifier.as_deref(), Some("Pad Left Thumb"));

        world.run_system_once(update_flight_verb_hints).unwrap();
        let hints = world.resource::<FlightVerbHints>().clone();
        assert_eq!(
            (hints.dock.key.as_str(), &hints.dock.modifier),
            ("D", &None)
        );
        assert_eq!(
            (hints.helm.key.as_str(), &hints.helm.modifier),
            ("H", &None)
        );

        world.insert_resource(InputDeviceType::Pad);
        world.run_system_once(update_flight_verb_hints).unwrap();
        let hints = world.resource::<FlightVerbHints>().clone();
        assert_eq!(Some(hints.stop.key.clone()), pad("autopilot_stop"));
        assert_ne!(hints.stop.key, "X");
        assert_eq!(hints.stop.modifier, None, "MATCH keeps its one cap");
        assert_eq!(
            (hints.dock.key.as_str(), &hints.dock.modifier),
            ("Pad Y", &modifier),
            "DOCK is the modifier and the GOTO button"
        );
        assert_eq!(
            (hints.helm.key.as_str(), &hints.helm.modifier),
            ("Pad A", &modifier),
            "HELM is the modifier and the ORBIT button"
        );
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
        world.entity_mut(ship).insert(RcsBudget::default());

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

    /// The RCS chip is the magazine's offer: with nothing left to push, the
    /// modifier enters a mode that moves nothing, so the chip goes dim the
    /// way the MATCH chord does. It stayed lit on an empty magazine before.
    #[test]
    fn an_empty_rcs_magazine_dims_the_rcs_hint() {
        let mut world = hint_world();
        let (ship, _controller) = spawn_flyable_ship(&mut world);
        let mut budget = RcsBudget::default();
        world.entity_mut(ship).insert(budget);

        world.run_system_once(update_flight_verb_hints).unwrap();
        let hints = world.resource::<FlightVerbHints>();
        assert!(
            hints.rcs.shown && hints.rcs.available,
            "a full magazine lights RCS"
        );

        budget.spent = budget.capacity;
        world.entity_mut(ship).insert(budget);
        world.run_system_once(update_flight_verb_hints).unwrap();
        let hints = world.resource::<FlightVerbHints>();
        assert!(hints.rcs.shown, "the core chip stays on the row");
        assert!(!hints.rcs.available, "an empty magazine dims RCS");
    }

    /// Which verbs are on the row, from the situation: the core group while
    /// the rig exists, the others only while there is something to act on.
    #[test]
    fn verbs_are_shown_by_the_situation_they_apply_to() {
        let mut world = hint_world();
        let (ship, _controller) = spawn_flyable_ship(&mut world);

        // Flat space, no lock: the core group alone.
        world.run_system_once(update_flight_verb_hints).unwrap();
        let hints = world.resource::<FlightVerbHints>().clone();
        for (verb, hint) in [
            ("DRIVE", &hints.drive),
            ("STOP", &hints.stop),
            ("RCS", &hints.rcs),
            ("RADAR", &hints.radar),
        ] {
            assert!(hint.shown, "{verb} is a core verb");
        }
        assert_eq!(hints.drive.key, "W");
        assert!(hints.drive.available, "a live thruster answers the drive");
        for (verb, hint) in [
            ("GOTO", &hints.goto),
            ("ORBIT", &hints.orbit),
            ("CANCEL", &hints.cancel),
            ("DOCK", &hints.dock),
            ("HELM", &hints.helm),
            ("MINE", &hints.mine),
            ("AIM", &hints.aim),
            ("FIRE", &hints.fire),
        ] {
            assert!(!hint.shown, "{verb} has nothing to act on");
        }

        // A lock shows GOTO, a well shows ORBIT, a maneuver shows CANCEL.
        let lock = world.spawn_empty().id();
        let well = world.spawn_empty().id();
        world.entity_mut(ship).insert((
            TravelLock(Some(lock)),
            DominantWell(well),
            Autopilot::engage(AutopilotAction::Orbit { well, plan: None }),
        ));
        world.run_system_once(update_flight_verb_hints).unwrap();
        let hints = world.resource::<FlightVerbHints>().clone();
        assert!(hints.goto.shown && hints.orbit.shown && hints.cancel.shown);
        assert!(!hints.dock.shown, "a lock with no port is no DOCK");

        // No rig: nothing is shown, nothing has a key.
        let rigs: Vec<Entity> = world
            .query_filtered::<Entity, With<Action<AutopilotStopInput>>>()
            .iter(&world)
            .collect();
        for rig in rigs {
            world.entity_mut(rig).despawn();
        }
        world.run_system_once(update_flight_verb_hints).unwrap();
        let hints = world.resource::<FlightVerbHints>().clone();
        assert!(!hints.stop.shown && !hints.drive.shown && !hints.rcs.shown);
        assert!(hints.stop.key.is_empty() && hints.fire.key.is_empty());
    }

    /// DOCK joins the row the moment the travel lock is a hull with a port,
    /// dim: the pilot sees the key through the approach, and it lights only
    /// when the port pair passes the gates.
    #[test]
    fn a_lock_with_a_docking_port_shows_dock_before_its_gates_pass() {
        let mut world = hint_world();
        let (ship, _controller) = spawn_flyable_ship(&mut world);
        let station = world.spawn(SpaceshipRootMarker).id();
        world.spawn((ChildOf(station), DockingSectionMarker));
        world.entity_mut(ship).insert(TravelLock(Some(station)));

        world.run_system_once(update_flight_verb_hints).unwrap();
        let hints = world.resource::<FlightVerbHints>().clone();
        assert!(hints.dock.shown, "a port in the lock puts DOCK on the row");
        assert!(
            !hints.dock.available,
            "no port pair passes the gates, so the chip is dim"
        );
        assert_eq!(hints.dock.anchor, Some(station));

        // A dead port is no offer at all.
        let ports: Vec<Entity> = world
            .query_filtered::<Entity, With<DockingSectionMarker>>()
            .iter(&world)
            .collect();
        for port in ports {
            world.entity_mut(port).insert(SectionInactiveMarker);
        }
        world.run_system_once(update_flight_verb_hints).unwrap();
        assert!(!world.resource::<FlightVerbHints>().dock.shown);
    }

    /// The weapon chips read the sections' own bindings and follow the
    /// device in hand; FIRE, TORPEDO and RAILGUN wait for the weapons to be
    /// hot, AIM and MINE do not.
    #[test]
    fn weapon_hints_take_their_keys_from_the_section_bindings() {
        use InputSource::Keyboard;

        use crate::input::bindings::section_default_bindings;

        let mut world = hint_world();
        let (ship, _controller) = spawn_flyable_ship(&mut world);
        world.entity_mut(ship).insert(WeaponsHot(false));

        world.run_system_once(update_flight_verb_hints).unwrap();
        let hints = world.resource::<FlightVerbHints>().clone();
        assert!(!hints.aim.shown, "an unarmed hull has no AIM");
        assert!(!hints.mine.shown, "no mining section, no MINE");

        // Bound off the table the editor and the start roles bind new hulls
        // from.
        let defaults = |class| section_default_bindings(class, &[]).unwrap();
        world.spawn((
            ChildOf(ship),
            TurretSectionMarker,
            SpaceshipTurretInputBinding(defaults(SectionClass::Turret)),
        ));
        world.spawn((
            ChildOf(ship),
            TorpedoSectionMarker,
            SpaceshipTorpedoInputBinding(defaults(SectionClass::Torpedo)),
        ));
        let railgun = world
            .spawn((
                ChildOf(ship),
                RailgunSectionMarker,
                SpaceshipRailgunInputBinding(defaults(SectionClass::Railgun)),
            ))
            .id();
        world.spawn((
            ChildOf(ship),
            MiningSectionMarker,
            SpaceshipMiningInputBinding(defaults(SectionClass::Mining)),
        ));
        world.run_system_once(update_flight_verb_hints).unwrap();
        let hints = world.resource::<FlightVerbHints>().clone();
        assert!(
            hints.aim.shown && hints.aim.available,
            "an armed hull has AIM"
        );
        assert_eq!(hints.aim.key, "RMB");
        assert!(hints.mine.shown && hints.mine.available);
        assert_eq!(hints.mine.key, "V");
        assert_eq!(hints.fire.key, "LMB", "the key is read while cold");
        assert!(
            !hints.fire.shown && !hints.torpedo.shown && !hints.railgun.shown,
            "cold weapons keep the trigger chips off the row"
        );

        world.entity_mut(ship).insert(WeaponsHot(true));
        world.run_system_once(update_flight_verb_hints).unwrap();
        let hints = world.resource::<FlightVerbHints>().clone();
        assert!(hints.fire.shown && hints.fire.available);
        assert!(hints.torpedo.shown && hints.railgun.shown);
        assert_eq!(hints.torpedo.key, "F");
        assert_eq!(hints.railgun.key, "R");

        // The pad in hand: every default carries a pad button.
        world.insert_resource(InputDeviceType::Pad);
        world.run_system_once(update_flight_verb_hints).unwrap();
        let hints = world.resource::<FlightVerbHints>().clone();
        assert_eq!(hints.fire.key, "Pad Right Trigger 2");
        assert_eq!(hints.torpedo.key, "Pad Left Trigger");
        assert_eq!(hints.railgun.key, "Pad Select");
        assert_eq!(hints.mine.key, "Pad Right Trigger 2");
        assert_eq!(hints.aim.key, "Pad Left Trigger 2");

        // A hull saved before the pad defaults keeps the binding it was saved
        // with, so its chip keeps the key.
        world
            .entity_mut(railgun)
            .insert(SpaceshipRailgunInputBinding(vec![Keyboard(KeyCode::KeyR)]));
        world.run_system_once(update_flight_verb_hints).unwrap();
        assert_eq!(world.resource::<FlightVerbHints>().railgun.key, "R");
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
            RcsBudget::default(),
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

    fn match_world() -> (World, Entity, Entity) {
        let mut world = hint_world();
        let (ship, _controller) = spawn_flyable_ship(&mut world);
        let modifier = spawn_rcs_modifier(&mut world);
        world.entity_mut(ship).insert(RcsBudget::default());
        (world, ship, modifier)
    }

    #[test]
    fn holding_the_rcs_modifier_turns_the_stop_hint_into_match() {
        let (mut world, ship, modifier) = match_world();
        world.run_system_once(update_flight_verb_hints).unwrap();
        let hints = world.resource::<FlightVerbHints>();
        assert_eq!(hints.stop_verb, StopVerbType::Stop);
        assert!(!hints.rcs_held);
        assert!(hints.stop.available, "plain STOP is lit");

        world.entity_mut(modifier).insert(TriggerState::Fired);
        world.run_system_once(update_flight_verb_hints).unwrap();
        let hints = world.resource::<FlightVerbHints>();
        assert_eq!(hints.stop_verb, StopVerbType::Match);
        assert!(hints.rcs_held);
        assert!(!hints.stop.available, "no travel lock, nothing to match");

        let target = world.spawn(LinearVelocity(Vec3::X)).id();
        world.entity_mut(ship).insert(TravelLock(Some(target)));
        world.run_system_once(update_flight_verb_hints).unwrap();
        assert!(
            world.resource::<FlightVerbHints>().stop.available,
            "a moving travel lock and a filled magazine light MATCH"
        );
        assert!(
            !world.resource::<FlightVerbHints>().goto.available,
            "with the modifier down the GOTO button is the DOCK chord"
        );

        let mut budget = *world.get::<RcsBudget>(ship).unwrap();
        budget.spent = budget.capacity;
        world.entity_mut(ship).insert(budget);
        world.run_system_once(update_flight_verb_hints).unwrap();
        assert!(
            !world.resource::<FlightVerbHints>().stop.available,
            "an empty magazine darkens MATCH"
        );

        world.entity_mut(modifier).insert(TriggerState::None);
        world.run_system_once(update_flight_verb_hints).unwrap();
        let hints = world.resource::<FlightVerbHints>();
        assert_eq!(hints.stop_verb, StopVerbType::Stop);
        assert!(hints.goto.available, "the modifier up gives GOTO back");
    }

    #[test]
    fn a_flying_match_leaves_the_plain_key_reading_stop() {
        let (mut world, ship, modifier) = match_world();
        let target = world.spawn(LinearVelocity(Vec3::X)).id();
        let mut budget = *world.get::<RcsBudget>(ship).unwrap();
        budget.spent = budget.capacity;
        world.entity_mut(ship).insert((
            TravelLock(Some(target)),
            Autopilot::engage(AutopilotAction::MatchTarget { target }),
            budget,
        ));

        world.run_system_once(update_flight_verb_hints).unwrap();
        let hints = world.resource::<FlightVerbHints>();
        assert_eq!(hints.stop_verb, StopVerbType::Stop, "the plain key brakes");
        assert!(!hints.rcs_held);
        assert!(hints.stop.available, "the plain key is an offer");
        assert!(!hints.stop_flying, "a match is not the STOP burn");
        assert!(hints.engaged);

        world.entity_mut(modifier).insert(TriggerState::Fired);
        world.run_system_once(update_flight_verb_hints).unwrap();
        let hints = world.resource::<FlightVerbHints>();
        assert_eq!(hints.stop_verb, StopVerbType::Match);
        assert!(
            hints.stop.available,
            "the chord disengages a flying match, an empty magazine included"
        );
        assert!(hints.stop_flying, "the chord press ends the flying match");
    }

    /// The rig the hint tests share with the real input path: the real flight
    /// rig, the real STOP observer, a full magazine and a moving body on the
    /// travel lock. The hints resolve from the same world the key press lands
    /// in.
    fn press_rig() -> (App, Entity, Entity) {
        use bevy::input::InputPlugin;

        use crate::input::player::{
            flight_rig::{on_autopilot_stop_input, FlightInputMarker},
            test_support::spawn_flight_rig,
        };

        let mut app = App::new();
        app.add_plugins((MinimalPlugins, InputPlugin, EnhancedInputPlugin));
        app.add_plugins(bevy::state::app::StatesPlugin);
        app.init_state::<nova_gameplay::PauseStates>();
        app.init_resource::<Messages<MatchRefusal>>();
        app.init_resource::<FlightVerbHints>();
        app.insert_resource(InputDeviceType::default());
        app.insert_resource(InputBindings::from_actions(flight_bindings()));
        app.add_input_context::<FlightInputMarker>();
        app.add_observer(on_autopilot_stop_input);

        let (ship, _) = spawn_flyable_ship(app.world_mut());
        let target = app.world_mut().spawn(LinearVelocity(Vec3::X)).id();
        app.world_mut()
            .entity_mut(ship)
            .insert((RcsBudget::default(), TravelLock(Some(target))));
        app.finish();
        app.cleanup();
        app.update();
        spawn_flight_rig(&mut app);
        app.update();
        (app, ship, target)
    }

    fn press_key(app: &mut App, key: KeyCode) {
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.press(key);
        app.update();
        app.update();
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .release(key);
        app.update();
    }

    /// One row of the proof: the chip as shown, then the real X press, then the
    /// outcome the label promised.
    fn assert_press_matches_label(
        app: &mut App,
        ship: Entity,
        target: Entity,
        state: &str,
        expect_word: StopVerbType,
        expect_lit: bool,
        expect_flying: bool,
    ) {
        app.world_mut()
            .run_system_once(update_flight_verb_hints)
            .unwrap();
        let hints = app.world().resource::<FlightVerbHints>().clone();
        let before = app.world().get::<Autopilot>(ship).map(|ap| ap.action);
        assert_eq!(hints.stop_verb, expect_word, "{state}: the word");
        assert_eq!(hints.stop.available, expect_lit, "{state}: lit");
        assert_eq!(hints.stop_flying, expect_flying, "{state}: hot");

        press_key(app, KeyCode::KeyX);

        let after = app.world().get::<Autopilot>(ship).map(|ap| ap.action);
        let expected = match (expect_lit, expect_flying, hints.stop_verb) {
            (false, _, _) => before,
            (true, true, _) => None,
            (true, false, StopVerbType::Stop) => Some(AutopilotAction::Stop),
            (true, false, StopVerbType::Match) => Some(AutopilotAction::MatchTarget { target }),
        };
        assert_eq!(
            after, expected,
            "{state}: the press does what the chip says"
        );
    }

    #[test]
    fn the_stop_chip_names_what_its_press_does() {
        let hold = |app: &mut App| {
            app.world_mut()
                .resource_mut::<ButtonInput<KeyCode>>()
                .press(KeyCode::ShiftLeft);
            app.update();
            app.update();
        };
        let matching = |target| Autopilot::engage(AutopilotAction::MatchTarget { target });

        // Idle: plain STOP is an offer.
        let (mut app, ship, target) = press_rig();
        assert_press_matches_label(
            &mut app,
            ship,
            target,
            "idle",
            StopVerbType::Stop,
            true,
            false,
        );

        // STOP flying: the same key ends it, hot.
        let (mut app, ship, target) = press_rig();
        app.world_mut()
            .entity_mut(ship)
            .insert(Autopilot::engage(AutopilotAction::Stop));
        assert_press_matches_label(
            &mut app,
            ship,
            target,
            "stop flying",
            StopVerbType::Stop,
            true,
            true,
        );

        // A flying match with the modifier up: the key is STOP, not hot, and
        // the press replaces the match with the STOP burn.
        let (mut app, ship, target) = press_rig();
        app.world_mut().entity_mut(ship).insert(matching(target));
        assert_press_matches_label(
            &mut app,
            ship,
            target,
            "match flying, modifier up",
            StopVerbType::Stop,
            true,
            false,
        );

        // A flying match with the modifier held: MATCH, hot, the press ends it.
        // The match is engaged after the Shift press because a fresh Shift
        // drops it.
        let (mut app, ship, target) = press_rig();
        hold(&mut app);
        app.world_mut().entity_mut(ship).insert(matching(target));
        assert_press_matches_label(
            &mut app,
            ship,
            target,
            "match flying, modifier held",
            StopVerbType::Match,
            true,
            true,
        );

        // Modifier held, moving travel lock, full magazine: MATCH is an offer.
        let (mut app, ship, target) = press_rig();
        hold(&mut app);
        assert_press_matches_label(
            &mut app,
            ship,
            target,
            "modifier held with a lock",
            StopVerbType::Match,
            true,
            false,
        );

        // Modifier held with no travel lock: the chip is dim and the press
        // changes nothing.
        let (mut app, ship, target) = press_rig();
        app.world_mut().entity_mut(ship).insert(TravelLock(None));
        hold(&mut app);
        assert_press_matches_label(
            &mut app,
            ship,
            target,
            "modifier held without a lock",
            StopVerbType::Match,
            false,
            false,
        );
    }
}
