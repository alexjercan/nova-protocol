//! The keybind hint surface: which flight verbs are available right now and
//! what key each is bound to, resolved from the live rig for the HUD.

use bevy::prelude::*;
use bevy_enhanced_input::prelude::*;
use nova_gameplay::prelude::*;

#[cfg(test)]
use super::flight_rig::{flight_input_rig, FlightBurnInput, FlightInputMarker};
use super::flight_rig::{
    AutopilotGotoInput, AutopilotOffInput, AutopilotOrbitInput, AutopilotStopInput,
};
use crate::prelude::*;

/// One flight verb's hint state, for the keybind-hint HUD.
#[derive(Clone, Debug, Default, PartialEq, Reflect)]
pub struct VerbHint {
    /// The verb's keyboard label ("X", "G"...), read from the live bindings
    /// of the flight rig; empty until the rig exists.
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
    /// Component fine-lock cycle (plain scroll). The key label is the fixed
    /// string "SCROLL" - a wheel binding has no keyboard label to read.
    pub component_cycle: VerbHint,
    /// The radar gesture (hold CTRL = radar, tap = clear). Fixed "CTRL" label
    /// like the wheel rows (the binding spans both Control keys plus a pad
    /// button); available while the computer grants Lock (CTRL was missing
    /// from the cluster entirely).
    pub radar: VerbHint,
    /// The RCS fine-adjust modifier (hold SHIFT). Fixed "SHIFT" label like
    /// the wheel/CTRL rows; available while the computer grants the `Rcs`
    /// verb, so the row shows only where RCS is enabled - the same opt-out
    /// the mainline campaign uses while RCS is off pending rework.
    pub rcs: VerbHint,
    /// Whether any maneuver is engaged right now - explicit, so consumers
    /// (the GOTO cue hides mid-maneuver) do not have to proxy it through
    /// another verb's availability.
    pub engaged: bool,
}

/// The fixed label of a wheel-gesture hint, empty while the flight rig is
/// missing so the rows vanish with the other verbs'.
fn cycle_label(label: &str, rig_exists: bool) -> String {
    if rig_exists {
        label.to_string()
    } else {
        String::new()
    }
}

/// A short chip label for a keyboard binding: `KeyX` -> `X`, `Digit1` -> `1`,
/// everything else (Space, Enter...) as spelled. `nova_hud`'s key-glyph
/// coverage test labels the real bindings with THIS function, so it crosses the
/// HUD seam.
pub fn keyboard_label(key: KeyCode) -> String {
    let name = format!("{key:?}");
    name.strip_prefix("Key")
        .or_else(|| name.strip_prefix("Digit"))
        .unwrap_or(&name)
        .to_string()
}

/// A short display chip for a section's input binding (the editor keybind
/// readout): the first keyboard or mouse binding in the list, keyboards via
/// `keyboard_label` and mouse buttons as `LMB`/`RMB`/`MMB`. Empty string when
/// there is no keyboard/mouse binding (e.g. gamepad-only).
pub fn binding_label(bindings: &[Binding]) -> String {
    bindings
        .iter()
        .find_map(|binding| match binding {
            Binding::Keyboard { key, .. } => Some(keyboard_label(*key)),
            Binding::MouseButton { button, .. } => Some(
                match button {
                    MouseButton::Left => "LMB",
                    MouseButton::Right => "RMB",
                    MouseButton::Middle => "MMB",
                    _ => "MB",
                }
                .to_string(),
            ),
            _ => None,
        })
        .unwrap_or_default()
}

/// A physical input source - the discrete button a binding occupies, stripped
/// of modifiers and gesture conditions. Two bindings that name the same source
/// drive the same physical input; that is exactly the silent double-drive a
/// content `input_mapping` must not create against the always-on flight rig.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum InputSource {
    /// A keyboard key.
    Keyboard(KeyCode),
    /// A mouse button.
    Mouse(MouseButton),
    /// A gamepad button.
    Gamepad(GamepadButton),
}

impl InputSource {
    /// A short human label for the source (`W`, `Space`, `LMB`, `RightTrigger`).
    pub fn label(&self) -> String {
        match self {
            InputSource::Keyboard(key) => keyboard_label(*key),
            InputSource::Mouse(MouseButton::Left) => "LMB".to_string(),
            InputSource::Mouse(MouseButton::Right) => "RMB".to_string(),
            InputSource::Mouse(MouseButton::Middle) => "MMB".to_string(),
            InputSource::Mouse(button) => format!("{button:?}"),
            InputSource::Gamepad(button) => format!("{button:?}"),
        }
    }
}

/// The physical source a binding occupies, if it names a discrete button
/// (keyboard / mouse / gamepad). Motion, wheel, stick-axis, `AnyKey`, custom
/// and empty bindings have no single collision key and return `None`.
pub fn binding_source(binding: &Binding) -> Option<InputSource> {
    match binding {
        Binding::Keyboard { key, .. } => Some(InputSource::Keyboard(*key)),
        Binding::MouseButton { button, .. } => Some(InputSource::Mouse(*button)),
        Binding::GamepadButton(button) => Some(InputSource::Gamepad(*button)),
        _ => None,
    }
}

/// The discrete input sources the always-on flight rig (`flight_input_rig`)
/// reserves, each paired with the flight verb it drives. Every action in that
/// rig runs with `consume_input: false`, so a content `input_mapping` section
/// that reuses one of these sources SILENTLY double-drives flight (bug:
/// "guns" on Space burned the ship off its mark and broke the 10_playable CI
/// smoke; lesson `input-mapping-overlays-flight-rig`). The content lint's
/// input-overlap check flags exactly this set; the
/// `flight_rig_reserves_exactly_these_sources` test in this module pins the
/// list against the REAL rig so authoring and lint cannot drift apart. Wheel
/// and motion sources (component cycle, RCS aim) are deliberately absent:
/// they are axes, not discrete buttons a section binding collides on.
pub fn flight_rig_reserved_sources() -> Vec<(InputSource, &'static str)> {
    use InputSource::{Gamepad, Keyboard};
    vec![
        (Keyboard(KeyCode::KeyW), "flight burn"),
        (Keyboard(KeyCode::Space), "flight burn"),
        (Gamepad(GamepadButton::RightTrigger), "flight burn"),
        (Keyboard(KeyCode::KeyX), "autopilot stop"),
        (Gamepad(GamepadButton::East), "autopilot stop"),
        (Keyboard(KeyCode::KeyG), "autopilot goto"),
        (Gamepad(GamepadButton::North), "autopilot goto"),
        (Keyboard(KeyCode::KeyO), "autopilot orbit"),
        (Gamepad(GamepadButton::South), "autopilot orbit"),
        (Keyboard(KeyCode::KeyZ), "autopilot off"),
        (Gamepad(GamepadButton::West), "autopilot off"),
        (
            Keyboard(KeyCode::ControlLeft),
            "radar hold / lock-cycle modifier",
        ),
        (
            Keyboard(KeyCode::ControlRight),
            "radar hold / lock-cycle modifier",
        ),
        (Gamepad(GamepadButton::DPadUp), "radar hold"),
        (Keyboard(KeyCode::BracketRight), "component cycle next"),
        (Gamepad(GamepadButton::DPadRight), "component cycle next"),
        (Keyboard(KeyCode::BracketLeft), "component cycle prev"),
        (Gamepad(GamepadButton::DPadLeft), "component cycle prev"),
        (Keyboard(KeyCode::ShiftLeft), "RCS modifier"),
        (Keyboard(KeyCode::ShiftRight), "RCS modifier"),
        (Gamepad(GamepadButton::LeftTrigger2), "RCS modifier"),
    ]
}

/// Resolve the verb hints from the live world: availability from the same
/// state the input observers AND the autopilot gate on (lock, dominant
/// well, engagement, and a flyable ship - a live flight computer plus at
/// least one live engine, else autopilot_system strips the maneuver on its
/// next tick and a lit hint would be a lie), labels from the flight rig's
/// actual `Bindings` so a future remap screen cannot desync the hints.
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
    q_stop: Query<&Bindings, With<Action<AutopilotStopInput>>>,
    q_goto: Query<&Bindings, With<Action<AutopilotGotoInput>>>,
    q_orbit: Query<&Bindings, With<Action<AutopilotOrbitInput>>>,
    q_off: Query<&Bindings, With<Action<AutopilotOffInput>>>,
    q_binding: Query<&Binding>,
) {
    let label = |bindings: Option<&Bindings>| -> String {
        bindings
            .into_iter()
            .flatten()
            .find_map(|entity| match q_binding.get(entity) {
                Ok(Binding::Keyboard { key, .. }) => Some(keyboard_label(*key)),
                _ => None,
            })
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
            key: label(q_stop.single().ok()),
            available: flyable && verb_granted(FlightVerb::Stop),
            anchor: None,
        },
        goto: VerbHint {
            key: label(q_goto.single().ok()),
            available: flyable && verb_granted(FlightVerb::Goto) && travel.is_some(),
            anchor: travel,
        },
        orbit: VerbHint {
            key: label(q_orbit.single().ok()),
            available: flyable
                && verb_granted(FlightVerb::Orbit)
                && dominant.is_some()
                && !orbiting,
            anchor: dominant.map(|well| **well),
        },
        cancel: VerbHint {
            key: label(q_off.single().ok()),
            // Z always answers while engaged, even on a crippled ship.
            available: engaged,
            anchor: None,
        },
        // The wheel gesture carries a fixed label (no keyboard key to read),
        // gated on the rig existing to keep the "no rig, no keys, no hints"
        // invariant. Component cycling needs the COMBAT focus dwell complete
        // and at least two attached sections to step between.
        component_cycle: VerbHint {
            key: cycle_label("SCROLL", q_stop.single().is_ok()),
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
            key: cycle_label("CTRL", q_stop.single().is_ok()),
            available: verb_granted(FlightVerb::Lock),
            anchor: None,
        },
        rcs: VerbHint {
            // Fixed "SHIFT" label (a modifier binding, no keyboard key to read),
            // gated on the rig existing like the wheel/CTRL rows; shown only
            // while the computer grants RCS.
            key: cycle_label("SHIFT", q_stop.single().is_ok()),
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
    fn reference_rows_track_the_flight_rig() {
        use bevy::input::InputPlugin;

        use crate::input::reference::KEYBINDS;

        let mut app = App::new();
        app.add_plugins((MinimalPlugins, InputPlugin, EnhancedInputPlugin));
        app.add_input_context::<FlightInputMarker>();
        app.finish();
        app.cleanup();
        app.update();
        app.world_mut().spawn(flight_input_rig());
        app.update();

        // The first keyboard key the rig binds to an action (its primary key,
        // the one the reference row leads with).
        fn primary_key<A: bevy_enhanced_input::prelude::InputAction>(app: &mut App) -> KeyCode {
            let mut q = app
                .world_mut()
                .query_filtered::<&Bindings, With<Action<A>>>();
            let world = app.world();
            for bindings in q.iter(world) {
                for binding_entity in bindings.iter() {
                    if let Some(Binding::Keyboard { key, .. }) =
                        world.get::<Binding>(binding_entity)
                    {
                        return *key;
                    }
                }
            }
            panic!("the flight rig binds no keyboard key to this action");
        }

        // The friendly label the reference uses for a rig key. An unmapped key
        // panics on purpose: a remap to a new key must add its label here AND in
        // the reference, so neither can drift silently.
        fn display_label(key: KeyCode) -> &'static str {
            match key {
                KeyCode::KeyW => "W",
                KeyCode::KeyX => "X",
                KeyCode::KeyG => "G",
                KeyCode::KeyO => "O",
                KeyCode::KeyZ => "Z",
                KeyCode::ControlLeft | KeyCode::ControlRight => "Ctrl",
                KeyCode::BracketRight => "]",
                KeyCode::BracketLeft => "[",
                other => panic!(
                    "flight rig binds {other:?}, which has no reference display \
                     mapping - update reference.rs KEYBINDS and this test"
                ),
            }
        }

        // (reference action name, the rig key that row must display). The key is
        // read LIVE from the rig, so it tracks a remap.
        let rows: [(&str, KeyCode); 8] = [
            ("Main Drive", primary_key::<FlightBurnInput>(&mut app)),
            (
                "Autopilot: Stop",
                primary_key::<AutopilotStopInput>(&mut app),
            ),
            (
                "Autopilot: Go To",
                primary_key::<AutopilotGotoInput>(&mut app),
            ),
            (
                "Autopilot: Orbit",
                primary_key::<AutopilotOrbitInput>(&mut app),
            ),
            ("Autopilot: Off", primary_key::<AutopilotOffInput>(&mut app)),
            (
                "Radar (hold search / tap clear)",
                primary_key::<crate::input::targeting::RadarHoldInput>(&mut app),
            ),
            (
                "Lock / Component Next",
                primary_key::<crate::input::targeting::ComponentCycleNextInput>(&mut app),
            ),
            (
                "Lock / Component Prev",
                primary_key::<crate::input::targeting::ComponentCyclePrevInput>(&mut app),
            ),
        ];

        for (action, key) in rows {
            let row = KEYBINDS
                .iter()
                .find(|e| e.action == action)
                .unwrap_or_else(|| panic!("missing keybind reference row for {action}"));
            let label = display_label(key);
            assert!(
                row.keyboard.contains(label),
                "reference row {action:?} shows keyboard {:?}, but the rig binds \
                 {key:?} (displayed as {label:?}) - the readout has drifted",
                row.keyboard
            );
        }
    }

    #[test]
    fn binding_label_shows_the_first_keyboard_or_mouse_input() {
        assert_eq!(
            binding_label(&[Binding::from(KeyCode::KeyW)]),
            "W",
            "keyboard keys drop the Key/Digit prefix"
        );
        assert_eq!(binding_label(&[Binding::from(MouseButton::Left)]), "LMB");
        // First bindable input wins; a keyboard key ahead of a gamepad button.
        assert_eq!(
            binding_label(&[
                Binding::from(KeyCode::Space),
                Binding::from(GamepadButton::South),
            ]),
            "Space"
        );
        // Gamepad-only / empty -> no chip.
        assert_eq!(binding_label(&[Binding::from(GamepadButton::South)]), "");
        assert_eq!(binding_label(&[]), "");
    }

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

    /// The RCS hint carries the fixed "SHIFT" label and is available only while
    /// the controller grants the `Rcs` verb - so the cluster row shows only when
    /// RCS is enabled (the mainline campaign, which withholds it, never shows it).
    #[test]
    fn rcs_hint_shows_shift_only_when_the_verb_is_granted() {
        let mut world = hint_world();
        let (_, controller) = spawn_flyable_ship(&mut world);

        world.run_system_once(update_flight_verb_hints).unwrap();
        let hints = world.resource::<FlightVerbHints>();
        assert_eq!(hints.rcs.key, "SHIFT");
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
