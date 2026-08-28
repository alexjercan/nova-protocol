//! The keybind hint surface: which flight verbs are available right now and
//! what key each is bound to, resolved from the live rig for the HUD.

use bevy::prelude::*;
use bevy_enhanced_input::prelude::*;
use nova_gameplay::prelude::*;
use nova_input::prelude::*;

use super::flight_rig::AutopilotStopInput;
use crate::prelude::*;

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
    /// Component fine-lock cycle. The label stays the fixed string "SCROLL":
    /// the wheel half of `component_next` is part of the action rather than
    /// its spec, so no rebind can move it and no table read would tell the
    /// player anything the wheel does not already do.
    pub component_cycle: VerbHint,
    /// The radar gesture (hold = radar, tap = clear), labelled off
    /// `radar_hold`; available while the computer grants Lock.
    pub radar: VerbHint,
    /// The RCS fine-adjust modifier, labelled off `rcs_modifier`; available
    /// while the computer grants the `Rcs` verb, so the row shows only where
    /// RCS is enabled - the same opt-out the mainline campaign uses while RCS
    /// is off pending rework.
    pub rcs: VerbHint,
    /// Whether any maneuver is engaged right now - explicit, so consumers
    /// (the GOTO cue hides mid-maneuver) do not have to proxy it through
    /// another verb's availability.
    pub engaged: bool,
}

/// The fixed label of the wheel-gesture hint, empty while the flight rig is
/// missing so the row vanishes with the other verbs'.
fn cycle_label(label: &str, rig_exists: bool) -> String {
    if rig_exists {
        label.to_string()
    } else {
        String::new()
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
    q_sections: Query<&ChildOf, With<SectionMarker>>,
    q_ship: Query<
        (
            Entity,
            Option<&Autopilot>,
            Option<&DominantWell>,
            Option<&TravelLock>,
            Option<&CombatLock>,
            Option<&LockFocus>,
        ),
        With<PlayerSpaceshipMarker>,
    >,
    q_computer: Query<
        (&ChildOf, Option<&WithheldVerbs>),
        (
            With<ControllerSectionMarker>,
            With<PDController>,
            Without<SectionInactiveMarker>,
        ),
    >,
    q_thruster: Query<&ChildOf, (With<ThrusterSectionMarker>, Without<SectionInactiveMarker>)>,
    q_rig: Query<(), With<Action<AutopilotStopInput>>>,
    bindings: Option<Res<InputBindings>>,
) {
    // The rig is the gate, not the source: a row is drawn only while the rig
    // that answers it exists, so all seven vanish together on a ship with no
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
    let (ship, autopilot, dominant, travel, combat, focus) = match q_ship.single() {
        Ok((entity, autopilot, dominant, travel, combat, focus)) => {
            (Some(entity), autopilot, dominant, travel, combat, focus)
        }
        Err(_) => (None, None, None, None, None, None),
    };
    let travel = travel.and_then(|travel| travel.0);
    let combat = combat.and_then(|combat| combat.0);
    // The autopilot needs a live flight computer and at least one live
    // engine or it disengages on its next tick; a hint below that bar
    // would light a key that visibly does nothing.
    let flyable = ship.is_some_and(|ship| {
        q_computer
            .iter()
            .any(|(&ChildOf(parent), _)| parent == ship)
            && q_thruster.iter().any(|&ChildOf(parent)| parent == ship)
    });
    // The individual maneuvers are a capability the controller GRANTS: a verb
    // lights only if some live controller on this ship enables it (union across
    // controllers), on top of `flyable`. The verb flags are kept SEPARATE from
    // `flyable` above (which only asks "is there a live controller + engine")
    // so a controller missing the withheld-verbs component can never brick the
    // ship - it just falls back to the all-granted default (an absent component
    // means nothing is withheld). The `SetControllerVerb` action flips these.
    let verb_granted = |verb: FlightVerb| -> bool {
        ship.is_some_and(|ship| {
            q_computer.iter().any(|(&ChildOf(parent), withheld)| {
                parent == ship && withheld.is_none_or(|withheld| withheld.granted(verb))
            })
        })
    };
    let engaged = autopilot.is_some();
    let orbiting = matches!(
        autopilot.map(|ap| ap.action),
        Some(AutopilotAction::Orbit { .. })
    );

    let next = FlightVerbHints {
        stop: VerbHint {
            key: label("autopilot_stop"),
            available: flyable && verb_granted(FlightVerb::Stop),
            anchor: None,
        },
        goto: VerbHint {
            key: label("autopilot_goto"),
            available: flyable && verb_granted(FlightVerb::Goto) && travel.is_some(),
            anchor: travel,
        },
        orbit: VerbHint {
            key: label("autopilot_orbit"),
            available: flyable
                && verb_granted(FlightVerb::Orbit)
                && dominant.is_some()
                && !orbiting,
            anchor: dominant.map(|well| **well),
        },
        cancel: VerbHint {
            key: label("autopilot_off"),
            // Z always answers while engaged, even on a crippled ship.
            available: engaged,
            anchor: None,
        },
        // The one row that stays a literal: the wheel belongs to the ACTION,
        // not its spec, so no rebind can move it. Gated on the rig existing to
        // keep the "no rig, no keys, no hints" invariant. Component cycling
        // needs the COMBAT focus dwell complete and at least two attached
        // sections to step between.
        component_cycle: VerbHint {
            key: cycle_label("SCROLL", rig_exists),
            available: combat.is_some_and(|target| {
                focus.is_some_and(|focus| focus.focused_on(target))
                    && q_sections
                        .iter()
                        .filter(|&&ChildOf(parent)| parent == target)
                        .count()
                        >= 2
            }),
            anchor: None,
        },
        radar: VerbHint {
            key: label("radar_hold"),
            available: verb_granted(FlightVerb::Lock),
            anchor: None,
        },
        rcs: VerbHint {
            // Shown only while the computer grants RCS.
            key: label("rcs_modifier"),
            available: verb_granted(FlightVerb::Rcs),
            anchor: None,
        },
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
    /// the controller grants the `Rcs` verb - so the cluster row shows only when
    /// RCS is enabled (the mainline campaign, which withholds it, never shows it).
    #[test]
    fn rcs_hint_shows_shift_only_when_the_verb_is_granted() {
        let mut world = hint_world();
        let (_, controller) = spawn_flyable_ship(&mut world);

        world.run_system_once(update_flight_verb_hints).unwrap();
        let hints = world.resource::<FlightVerbHints>();
        assert_eq!(hints.rcs.key, "ShiftLeft");
        assert!(hints.rcs.available, "granted RCS lights the SHIFT hint");

        // Withhold RCS (the mainline path): the hint goes unavailable and the
        // renderer drops the row.
        world
            .entity_mut(controller)
            .insert(WithheldVerbs([FlightVerb::Rcs].into_iter().collect()));
        world.run_system_once(update_flight_verb_hints).unwrap();
        assert!(
            !world.resource::<FlightVerbHints>().rcs.available,
            "withheld RCS hides the SHIFT hint"
        );
    }

    #[test]
    fn cycle_hints_track_the_combat_focus() {
        let mut world = hint_world();
        let (ship, _) = spawn_flyable_ship(&mut world);

        // No lock: the cycle row is present (fixed label) but dim.
        world.run_system_once(update_flight_verb_hints).unwrap();
        let hints = world.resource::<FlightVerbHints>().clone();
        assert_eq!(hints.component_cycle.key, "SCROLL");
        assert!(!hints.component_cycle.available);

        // COMPONENT lights once the dwell completes on a combat lock with at
        // least two attached sections.
        let target = world.spawn_empty().id();
        world.spawn((SectionMarker, ChildOf(target)));
        world.spawn((SectionMarker, ChildOf(target)));
        world.get_mut::<CombatLock>(ship).unwrap().0 = Some(target);
        world.run_system_once(update_flight_verb_hints).unwrap();
        assert!(
            !world
                .resource::<FlightVerbHints>()
                .component_cycle
                .available,
            "no focus yet"
        );
        *world.get_mut::<LockFocus>(ship).unwrap() = LockFocus {
            target: Some(target),
            seconds: f32::MAX,
        };
        world.run_system_once(update_flight_verb_hints).unwrap();
        assert!(
            world
                .resource::<FlightVerbHints>()
                .component_cycle
                .available
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
    fn controller_verb_flags_gate_the_hints_independently_of_lock_and_well() {
        let mut world = hint_world();
        let (ship, controller) = spawn_flyable_ship(&mut world);

        // A lock and a dominant well are present, so absent the flags GOTO and
        // ORBIT would both light (as the neighbor test proves).
        let lock = world.spawn_empty().id();
        let well = world.spawn_empty().id();
        world
            .entity_mut(ship)
            .insert((TravelLock(Some(lock)), DominantWell(well)));

        // Withhold GOTO and ORBIT on the controller; STOP stays granted.
        world.entity_mut(controller).insert(WithheldVerbs(
            [FlightVerb::Goto, FlightVerb::Orbit].into_iter().collect(),
        ));
        world.run_system_once(update_flight_verb_hints).unwrap();
        let hints = world.resource::<FlightVerbHints>().clone();
        assert!(hints.stop.available, "STOP is still granted");
        assert!(
            !hints.goto.available,
            "GOTO withheld by the controller despite a live lock"
        );
        assert!(
            !hints.orbit.available,
            "ORBIT withheld by the controller despite a dominant well"
        );

        // Granting them lights both (the lock/well are unchanged) - proves the
        // withheld set, not some other condition, was the gate.
        world
            .entity_mut(controller)
            .insert(WithheldVerbs::default());
        world.run_system_once(update_flight_verb_hints).unwrap();
        let hints = world.resource::<FlightVerbHints>().clone();
        assert!(hints.goto.available, "GOTO lights once granted");
        assert!(hints.orbit.available, "ORBIT lights once granted");
    }
}
