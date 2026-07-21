//! Human piloting: turns keyboard/mouse/gamepad input into ship intent. The
//! always-on flight rig drives the flight verbs (burn, the STOP/GOTO/ORBIT
//! autopilot commands, RCS fine-adjust), and per-weapon `input_mapping` bindings
//! (thruster/turret/torpedo) fire the sections. Marks the human's ship with
//! [`PlayerSpaceshipMarker`] and maintains [`FlightVerbHints`] for the verb-hint
//! HUD.
//!
//! The reserved flight-rig sources ([`flight_rig_reserved_sources`]) must not be
//! reused by content weapon bindings or flight silently double-drives; see that
//! function's note. Autopilot verbs land as [`FlightIntent`](crate::flight) /
//! [`Autopilot`](crate::flight) on the ship, consumed by
//! [`flight`](crate::flight).

use avian3d::prelude::*;
use bevy::prelude::*;
use bevy_common_systems::prelude::*;
use bevy_enhanced_input::prelude::*;

use super::targeting::{
    ComponentCycleNextInput, ComponentCyclePrevInput, RadarClearInput, RadarHoldInput,
};
use crate::prelude::*;

pub mod prelude {
    pub use super::{
        binding_label, binding_source, flight_rig_reserved_sources, FlightVerbHints, InputSource,
        PlayerSpaceshipMarker, SpaceshipPlayerInputPlugin, SpaceshipThrusterInputBinding,
        SpaceshipTorpedoInputBinding, SpaceshipTurretInputBinding, VerbHint,
    };
}

/// Wires human input for the player ship: the flight rig, weapon fire bindings,
/// autopilot verbs and RCS. Added by [`SpaceshipInputPlugin`].
pub struct SpaceshipPlayerInputPlugin;

impl Plugin for SpaceshipPlayerInputPlugin {
    fn build(&self, app: &mut App) {
        debug!("SpaceshipPlayerInputPlugin: build");

        app.add_input_context::<FlightInputMarker>();
        app.add_observer(on_player_added_spawn_flight_input);
        app.add_observer(on_player_removed_despawn_flight_input);
        app.add_observer(on_flight_burn_input);
        app.add_observer(on_flight_burn_input_completed);
        app.add_observer(on_autopilot_stop_input);
        app.add_observer(on_autopilot_goto_input);
        app.add_observer(on_autopilot_orbit_input);
        app.add_observer(on_autopilot_off_input);
        app.add_observer(on_rcs_modifier_start);
        app.add_observer(on_rcs_modifier_released);
        app.add_observer(on_rcs_aim);

        app.add_input_context::<ThrusterInputMarker>();
        app.add_observer(on_thruster_input_binding);
        app.add_observer(on_thruster_input);
        app.add_observer(on_thruster_input_completed);

        app.add_input_context::<TurretInputMarker>();
        app.add_observer(on_turret_input_binding);
        app.add_observer(on_turret_input);
        app.add_observer(on_turret_input_completed);

        app.add_input_context::<TorpedoInputMarker>();
        app.add_observer(on_torpedo_input_binding);
        app.add_observer(on_torpedo_input);
        app.add_observer(on_torpedo_input_completed);

        app.init_resource::<FlightVerbHints>();
        app.register_type::<FlightVerbHints>();

        app.add_systems(
            Update,
            (
                update_controller_target_rotation_torque,
                // The turret feed reads the lock, focus and component state,
                // so it runs after the targeting chain, same as the torpedo
                // commit (previously a .chain() when they shared a module).
                update_turret_target_input.after(super::targeting::SpaceshipTargetingSystems),
                update_torpedo_target_input.after(super::targeting::SpaceshipTargetingSystems),
                update_flight_verb_hints.after(super::targeting::SpaceshipTargetingSystems),
            )
                .in_set(super::SpaceshipInputSystems),
        );
    }
}

/// One flight verb's hint state, for the keybind-hint HUD (spike
/// docs/spikes/20260710-174523-diegetic-instruments-keybind-hints.md).
#[derive(Clone, Debug, Default, PartialEq, Reflect)]
pub struct VerbHint {
    /// The verb's keyboard label ("X", "G", ...), read from the live
    /// bindings of the flight rig; empty until the rig exists.
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
const HOLD_FIRE_DURING_RADAR: bool = false;

/// The player's currently available flight verbs, resolved every frame by
/// `update_flight_verb_hints` - computed here, where the verbs and their
/// (private) input actions live; the HUD renders it dumb. Keyboard labels
/// only in v1 (device awareness is a recorded open question).
#[derive(Resource, Clone, Debug, Default, PartialEq, Reflect)]
#[reflect(Resource)]
pub struct FlightVerbHints {
    pub stop: VerbHint,
    pub goto: VerbHint,
    pub orbit: VerbHint,
    pub cancel: VerbHint,
    /// Component fine-lock cycle (plain scroll). The key label is the fixed
    /// string "SCROLL" - a wheel binding has no keyboard label to read.
    pub component_cycle: VerbHint,
    /// The radar gesture (hold CTRL = radar, tap = clear). Fixed "CTRL"
    /// label like the wheel rows (the binding spans both Control keys plus
    /// a pad button); available while the computer grants Lock (playtest
    /// 2026-07-13: CTRL was missing from the cluster entirely).
    pub radar: VerbHint,
    /// The RCS fine-adjust modifier (hold SHIFT). Fixed "SHIFT" label like the
    /// wheel/CTRL rows; available while the computer grants the `Rcs` verb, so
    /// the row shows only where RCS is enabled - the same opt-out the mainline
    /// campaign uses while RCS is off pending rework (task 20260718-175502).
    pub rcs: VerbHint,
    /// Whether any maneuver is engaged right now - explicit, so consumers
    /// (the GOTO cue hides mid-maneuver) do not have to proxy it through
    /// another verb's availability.
    pub engaged: bool,
}

/// The fixed label of a wheel-gesture hint, empty while the flight rig is
/// missing so the rows vanish with the other verbs' (review R1.1).
fn cycle_label(label: &str, rig_exists: bool) -> String {
    if rig_exists {
        label.to_string()
    } else {
        String::new()
    }
}

/// A short chip label for a keyboard binding: `KeyX` -> `X`,
/// `Digit1` -> `1`, everything else (Space, Enter, ...) as spelled.
fn keyboard_label(key: KeyCode) -> String {
    let name = format!("{key:?}");
    name.strip_prefix("Key")
        .or_else(|| name.strip_prefix("Digit"))
        .unwrap_or(&name)
        .to_string()
}

/// A short display chip for a section's input binding (the editor keybind
/// readout, task 20260712-163912): the first keyboard or mouse binding in the
/// list, keyboards via `keyboard_label` and mouse buttons as `LMB`/`RMB`/`MMB`.
/// Empty string when there is no keyboard/mouse binding (e.g. gamepad-only).
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
    Keyboard(KeyCode),
    Mouse(MouseButton),
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
/// that reuses one of these sources SILENTLY double-drives flight (bug
/// 20260718-235837: "guns" on Space burned the ship off its mark and broke the
/// 10_playable CI smoke; lesson `input-mapping-overlays-flight-rig`). The
/// content lint's input-overlap check flags exactly this set; the
/// `flight_rig_reserves_exactly_these_sources` test in this module pins the
/// list against the REAL rig so authoring and lint cannot drift apart. Wheel
/// and motion sources (component cycle, RCS aim) are deliberately absent: they
/// are axes, not discrete buttons a section binding collides on.
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
fn update_flight_verb_hints(
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
        // invariant (review R1.1). Component cycling needs the COMBAT focus
        // dwell complete and at least two attached sections to step between.
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

/// Marker component to identify the player's spaceship.
///
/// This should be added to the root entity of the player's spaceship.
/// Carries [`Allegiance::Player`] by requirement, so every player-marked
/// root participates in the relation model without extra spawn wiring.
#[derive(Component, Debug, Clone, Reflect)]
#[require(SpaceshipRootMarker, Allegiance = Allegiance::Player)]
pub struct PlayerSpaceshipMarker;

/// System that takes the point rotation output from the chase camera and applies it to the
/// controller of the player's spaceship.
///
/// Gated on `Without<Autopilot>`: while a maneuver is engaged the autopilot
/// owns the rotation command, and the mouse - which keeps driving the camera
/// rig - becomes camera-only free-look for free.
fn update_controller_target_rotation_torque(
    time: Res<Time>,
    settings: Res<FlightSettings>,
    point_rotation: Single<
        &PointRotationOutput,
        (
            With<SpaceshipCameraInputMarker>,
            With<SpaceshipCameraNormalInputMarker>,
        ),
    >,
    mut q_controller: Query<
        (&mut ControllerSectionRotationInput, &ChildOf),
        With<ControllerSectionMarker>,
    >,
    spaceship: Single<
        (Entity, &ComputedAngularInertia),
        (
            With<SpaceshipRootMarker>,
            With<PlayerSpaceshipMarker>,
            Without<Autopilot>,
            // RCS fine-adjust repurposes the mouse to translation and freezes
            // the heading, exactly as an engaged maneuver does (spike Q4).
            Without<RcsActive>,
        ),
    >,
    q_computer: Query<
        (&PDController, &ChildOf),
        (
            With<ControllerSectionMarker>,
            Without<SectionInactiveMarker>,
        ),
    >,
) {
    let point_rotation = point_rotation.into_inner();
    let (spaceship, inertia) = spaceship.into_inner();
    // Slew the command toward the camera instead of jumping: a mouse 180 fed
    // to the PD in one step drives it into torque saturation where its
    // damping is swamped and the hull limit-cycles (the high-speed flip
    // wobble). The camera stays instant; the hull's commanded target ramps
    // at the hull's own torque-budget turn rate - the same one the autopilot
    // plans with (see flight::ship_turn_rate) - so a heavy build swings
    // slower than a stripped one. With no live computer the command FREEZES:
    // nothing consumes it, and slewing a dead helm would drift it so a later
    // re-activation snaps the hull.
    let Some(turn_rate) = crate::flight::ship_turn_rate(
        q_computer
            .iter()
            .filter(|(_, &ChildOf(parent))| parent == spaceship)
            .map(|(pd, _)| pd.max_torque),
        inertia,
        &settings,
    ) else {
        return;
    };
    let max_step = turn_rate * time.delta_secs();

    for (mut controller, _) in q_controller
        .iter_mut()
        .filter(|(_, ChildOf(c_parent))| *c_parent == spaceship)
    {
        **controller = crate::flight::slew_rotation(**controller, **point_rotation, max_step);
    }
}

/// System that takes the point rotation output from the chase camera and applies it to the
/// turret target input of the player's spaceship.
fn update_turret_target_input(
    point_rotation: Single<
        &PointRotationOutput,
        (
            With<SpaceshipCameraInputMarker>,
            With<SpaceshipCameraTurretInputMarker>,
        ),
    >,
    mut q_turret: Query<
        (
            &mut TurretSectionTargetInput,
            &mut TurretSectionTargetVelocity,
            &ChildOf,
        ),
        With<TurretSectionMarker>,
    >,
    spaceship: Single<
        (
            &Transform,
            Option<&ComputedCenterOfMass>,
            Entity,
            Option<&CombatLock>,
            Option<&ComponentLock>,
        ),
        (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>),
    >,
    q_lock_target: Query<(
        &Transform,
        Option<&ComputedCenterOfMass>,
        Option<&LinearVelocity>,
    )>,
    q_section_position: Query<&GlobalTransform, With<SectionMarker>>,
) {
    let point_rotation = point_rotation.into_inner();
    let (transform, com, spaceship, lock, component) = spaceship.into_inner();
    let lock = lock.and_then(|lock| lock.0);
    let component_section = component.and_then(|component| component.section);

    // Base the aim ray on the live structure so the turret aim point matches
    // the COM-anchored camera crosshair after losing sections (task
    // 20260709-150711).
    let position = live_structure_anchor(transform, com);

    // Three-tier auto-fire feed (component-lock spike, task 20260709-173700):
    // the fine-locked section, else the locked ship's live structure, else
    // the camera ray as always. Lock tiers carry the lock root's velocity so
    // lead_intercept_point computes a real intercept; the ray tier aims at a
    // commanded point, not a body, so its velocity is zero. A dead section or
    // lock falls through to the next tier the same frame (the targeting
    // systems clear the stale state on their next run).
    let lock_tier = lock.and_then(|target| {
        q_lock_target
            .get(target)
            .ok()
            .map(|(target_transform, target_com, target_velocity)| {
                (
                    live_structure_anchor(target_transform, target_com),
                    target_velocity
                        .map(|velocity| **velocity)
                        .unwrap_or(Vec3::ZERO),
                )
            })
    });
    let component_tier = component_section.and_then(|section| {
        let section_position = q_section_position.get(section).ok()?;
        let (_, lock_velocity) = lock_tier?;
        Some((section_position.translation(), lock_velocity))
    });
    let ray_tier = {
        let forward = **point_rotation * Vec3::NEG_Z;
        (position + forward * 100.0, Vec3::ZERO)
    };
    // LOCK-WINS routing (playtest verdict 2026-07-13, task 20260713-121605,
    // flipping the manual-wins knob from spike 20260713-082207): a combat
    // lock holds the turrets even while RAISED - moving the cursor must not
    // pull them off the target; tap CTRL (clearing the lock) is the explicit
    // road back to manual. With NO lock, the ray tier IS the raised manual
    // aim, so no stance special-case remains - the pure three-tier feed.
    let (target_point, target_velocity) = component_tier.or(lock_tier).unwrap_or(ray_tier);

    for (mut turret, mut velocity, _) in q_turret
        .iter_mut()
        .filter(|(_, _, ChildOf(t_parent))| *t_parent == spaceship)
    {
        **turret = Some(target_point);
        **velocity = target_velocity;
    }
}

/// Commit each freshly launched torpedo to its launch-time target.
///
/// A torpedo's targeting decision is made exactly once, right after launch:
/// whatever the crosshair has locked at that moment becomes the torpedo's target
/// for life (`TorpedoTargetChosen` marks the decision as made). No lock means a
/// dumb-fire shot that never acquires anything mid-flight - so, e.g., bullets
/// fired past a loitering torpedo are not picked up as targets, and a torpedo
/// whose target died (link dropped by `update_target_position`, position frozen)
/// is not re-assigned to whatever the player locks next.
fn update_torpedo_target_input(
    mut commands: Commands,
    q_torpedo: Query<
        (Entity, &ProjectileOwner),
        (
            With<TorpedoProjectileMarker>,
            Without<TorpedoTargetEntity>,
            Without<TorpedoTargetChosen>,
        ),
    >,
    spaceship: Single<
        (Entity, Option<&CombatLock>),
        (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>),
    >,
) {
    let (spaceship, lock) = spaceship.into_inner();
    let lock = lock.and_then(|lock| lock.0);

    for (torpedo, owner) in &q_torpedo {
        if **owner != spaceship {
            continue;
        }

        debug!(
            "update_torpedo_target_input: committing torpedo {:?} to target {:?}",
            torpedo, lock
        );

        let mut torpedo_commands = commands.entity(torpedo);
        torpedo_commands.insert(TorpedoTargetChosen);
        if let Some(target_entity) = lock {
            torpedo_commands.insert(TorpedoTargetEntity(target_entity));
        }
    }
}

/// Input context for the player's flight controls: analog main-drive burn
/// plus the autopilot engagements. One rig exists while a player ship does;
/// the observers below write the ship's [`FlightIntent`] and insert/remove
/// its [`Autopilot`] (`crate::flight`). Any flight input while an autopilot
/// is engaged disengages it - mouse-look does not, so watching a maneuver
/// never cancels it.
#[derive(Component, Debug, Clone)]
pub(crate) struct FlightInputMarker;

/// Analog main-drive burn (`0..1`).
#[derive(InputAction)]
#[action_output(f32)]
struct FlightBurnInput;

/// Engage the STOP maneuver (kill all velocity); pressing it again while
/// stopping disengages.
#[derive(InputAction)]
#[action_output(bool)]
struct AutopilotStopInput;

/// Engage the GOTO maneuver on the current aim-assist lock; pressing it again
/// while flying there disengages.
#[derive(InputAction)]
#[action_output(bool)]
struct AutopilotGotoInput;

/// Engage the ORBIT maneuver around the ship's dominant gravity well;
/// pressing it again while orbiting disengages. A no-op outside every SOI.
#[derive(InputAction)]
#[action_output(bool)]
struct AutopilotOrbitInput;

/// Plain autopilot off.
#[derive(InputAction)]
#[action_output(bool)]
struct AutopilotOffInput;

/// The RCS fine-adjust modifier: held (SHIFT) to enter the docking translation
/// mode. A plain Down action read as a held modifier (the `action_held` pattern,
/// not a binding Chord - see `modal-input-observer-dispatch`), whose Start/Stop
/// the observers turn into [`RcsActive`] on the player ship.
#[derive(InputAction)]
#[action_output(bool)]
struct RcsModifierInput;

/// The RCS aim: raw mouse motion (a per-frame `Vec2` delta), accumulated into
/// the ship-local `RcsIntent` XZ plane while [`RcsActive`] is held. Bound to the
/// same `mouse_motion` source as the camera rig (`consume_input: false`); the
/// camera's own consumer is frozen during RCS so the view holds.
#[derive(InputAction)]
#[action_output(Vec2)]
struct RcsAimInput;

fn on_player_added_spawn_flight_input(
    add: On<Add, PlayerSpaceshipMarker>,
    mut commands: Commands,
    q_existing: Query<(), With<FlightInputMarker>>,
) {
    trace!(
        "on_player_added_spawn_flight_input: entity {:?}",
        add.entity
    );
    // One player, one flight rig; a respawn reuses the existing one.
    if !q_existing.is_empty() {
        return;
    }

    commands.spawn(flight_input_rig());
}

/// The flight rig bundle: all flight actions and their bindings. A named
/// fn (not inlined in the observer) so the input tests can spawn the REAL
/// rig and drive it with simulated devices.
///
/// The CTRL layer (cycle the SHIP lock instead of components) is NOT
/// expressed as input conditions: a binding-level Chord ignores the
/// binding's own value and fired on the bare modifier (bug
/// 20260711-173237), and pairing it with an explicit Down still yields
/// Ongoing on the unmodified gesture, which triggers Start. Instead the
/// modifier is a plain action whose state the cycle observers READ
/// (input/targeting.rs dispatch): plain wheel/brackets step components,
/// the same gesture with the modifier held steps the ship lock.
pub(crate) fn flight_input_rig() -> impl Bundle {
    (
        Name::new("Input: Flight"),
        FlightInputMarker,
        actions!(
            FlightInputMarker[
                (
                    Name::new("Input: Flight Burn"),
                    Action::<FlightBurnInput>::new(),
                    ActionSettings {
                        consume_input: false,
                        ..default()
                    },
                    bindings![
                        KeyCode::KeyW,
                        KeyCode::Space,
                        GamepadButton::RightTrigger
                    ],
                ),
                (
                    Name::new("Input: Autopilot Stop"),
                    Action::<AutopilotStopInput>::new(),
                    ActionSettings {
                        consume_input: false,
                        ..default()
                    },
                    bindings![KeyCode::KeyX, GamepadButton::East],
                ),
                (
                    Name::new("Input: Autopilot Goto"),
                    Action::<AutopilotGotoInput>::new(),
                    ActionSettings {
                        consume_input: false,
                        ..default()
                    },
                    bindings![KeyCode::KeyG, GamepadButton::North],
                ),
                (
                    Name::new("Input: Autopilot Orbit"),
                    Action::<AutopilotOrbitInput>::new(),
                    ActionSettings {
                        consume_input: false,
                        ..default()
                    },
                    // South: the scenario-advance confirm (loader.rs) was moved
                    // off South to DPadDown so this pad press cannot both skip
                    // the scenario and toggle a parking maneuver.
                    bindings![KeyCode::KeyO, GamepadButton::South],
                ),
                (
                    Name::new("Input: Autopilot Off"),
                    Action::<AutopilotOffInput>::new(),
                    ActionSettings {
                        consume_input: false,
                        ..default()
                    },
                    bindings![KeyCode::KeyZ, GamepadButton::West],
                ),
                (
                    // The radar hold (deliberate-radar spike 20260713-082207):
                    // Start = search opens (slot latched), Fire = active,
                    // Complete = commit-on-release, Cancel = sub-threshold
                    // release (no commit; the Tap below is that gesture).
                    // Pad: DPadUp, freed by the target cycle's retirement -
                    // a provisional binding until the keybind rework
                    // (20260710-231927) picks the pad gesture properly.
                    Name::new("Input: Radar Hold"),
                    Action::<RadarHoldInput>::new(),
                    Hold::new(RADAR_TAP_SECS),
                    ActionSettings {
                        consume_input: false,
                        ..default()
                    },
                    bindings![
                        KeyCode::ControlLeft,
                        KeyCode::ControlRight,
                        GamepadButton::DPadUp
                    ],
                ),
                (
                    // The tap clear, same key + threshold const as the hold
                    // so the boundary frame cannot fall between them.
                    Name::new("Input: Radar Clear"),
                    Action::<RadarClearInput>::new(),
                    Tap::new(RADAR_TAP_SECS),
                    ActionSettings {
                        consume_input: false,
                        ..default()
                    },
                    bindings![
                        KeyCode::ControlLeft,
                        KeyCode::ControlRight,
                        GamepadButton::DPadUp
                    ],
                ),
                (
                    Name::new("Input: Component Cycle Next"),
                    Action::<ComponentCycleNextInput>::new(),
                    ActionSettings {
                        consume_input: false,
                        ..default()
                    },
                    // Scroll up = next: the wheel is an axis (y = vertical),
                    // so swizzle y into the action value and clamp away the
                    // opposite direction so only up-scrolls actuate.
                    bindings![
                        KeyCode::BracketRight,
                        GamepadButton::DPadRight,
                        (Binding::mouse_wheel(), SwizzleAxis::YXZ, Clamp::pos()),
                    ],
                ),
                (
                    Name::new("Input: Component Cycle Prev"),
                    Action::<ComponentCyclePrevInput>::new(),
                    ActionSettings {
                        consume_input: false,
                        ..default()
                    },
                    // Scroll down = prev: negate the (swizzled) wheel axis so
                    // down-scrolls become positive, then clamp like above.
                    bindings![
                        KeyCode::BracketLeft,
                        GamepadButton::DPadLeft,
                        (
                            Binding::mouse_wheel(),
                            SwizzleAxis::YXZ,
                            Negate::all(),
                            Clamp::pos()
                        ),
                    ],
                ),
                (
                    // The RCS fine-adjust modifier (SHIFT). Plain Down: Start on
                    // press, Complete on release; the observers read those into
                    // RcsActive. SHIFT is otherwise free (only CTRL is taken, by
                    // the radar). Pad: LeftTrigger2 (a free analog-as-button).
                    Name::new("Input: RCS Modifier"),
                    Action::<RcsModifierInput>::new(),
                    ActionSettings {
                        consume_input: false,
                        ..default()
                    },
                    bindings![
                        KeyCode::ShiftLeft,
                        KeyCode::ShiftRight,
                        GamepadButton::LeftTrigger2
                    ],
                ),
                (
                    // The RCS aim: raw mouse motion, accumulated into RcsIntent's
                    // XZ plane while RCS is held. Shares mouse_motion with the
                    // camera rig (consume_input: false); the camera's consumer is
                    // frozen during RCS so this is the only reader that acts.
                    Name::new("Input: RCS Aim"),
                    Action::<RcsAimInput>::new(),
                    ActionSettings {
                        consume_input: false,
                        ..default()
                    },
                    Bindings::spawn(Spawn((Binding::mouse_motion(), Scale::splat(1.0)))),
                ),
            ]
        ),
    )
}

fn on_player_removed_despawn_flight_input(
    remove: On<Remove, PlayerSpaceshipMarker>,
    mut commands: Commands,
    q_rig: Query<Entity, With<FlightInputMarker>>,
) {
    trace!(
        "on_player_removed_despawn_flight_input: entity {:?}",
        remove.entity
    );
    for rig in &q_rig {
        commands.entity(rig).try_despawn();
    }
}

fn on_flight_burn_input(
    fire: On<Fire<FlightBurnInput>>,
    mut commands: Commands,
    ship: Single<(Entity, &mut FlightIntent, Has<Autopilot>), With<PlayerSpaceshipMarker>>,
    pause: Res<State<crate::PauseStates>>,
) {
    // Observers bypass system-set gating; freeze intent changes while the
    // pause overlay is up (review R1.1). Releases stay ungated so held keys
    // clear cleanly during a pause.
    if *pause.get() == crate::PauseStates::Paused {
        return;
    }

    let (entity, mut intent, engaged) = ship.into_inner();
    intent.burn = fire.value;
    // Grabbing the throttle is a flight input: it takes the ship back.
    if engaged {
        debug!("on_flight_burn_input: manual burn disengages the autopilot");
        commands.entity(entity).remove::<Autopilot>();
    }
}

fn on_flight_burn_input_completed(
    _: On<Complete<FlightBurnInput>>,
    ship: Single<&mut FlightIntent, With<PlayerSpaceshipMarker>>,
) {
    let mut intent = ship.into_inner();
    intent.burn = 0.0;
}

/// Query over every live controller section and its (optional) withheld verbs,
/// shared by the three maneuver observers so they gate execution on the same
/// controller-provided capability the hint pass shows. `WithheldVerbs` is
/// optional for the same reason as in the hint pass: a controller missing the
/// component falls back to the all-granted default rather than becoming
/// ungovernable.
type ControllerVerbQuery<'w, 's> = Query<
    'w,
    's,
    (&'static ChildOf, Option<&'static WithheldVerbs>),
    (
        With<ControllerSectionMarker>,
        With<PDController>,
        Without<SectionInactiveMarker>,
    ),
>;

/// Whether some live controller section on `ship` grants `verb` (union across
/// controllers). Doubles as the controller-present check: no live controller,
/// no grant. Mirrors the `verb_granted` closure in the hint pass so a lit hint
/// and a firing key never disagree.
fn ship_grants_verb(ship: Entity, verb: FlightVerb, q_verbs: &ControllerVerbQuery) -> bool {
    q_verbs.iter().any(|(&ChildOf(parent), withheld)| {
        parent == ship && withheld.is_none_or(|w| w.granted(verb))
    })
}

fn on_autopilot_stop_input(
    _: On<Start<AutopilotStopInput>>,
    mut commands: Commands,
    ship: Single<(Entity, Option<&Autopilot>), With<PlayerSpaceshipMarker>>,
    q_verbs: ControllerVerbQuery,
    pause: Res<State<crate::PauseStates>>,
) {
    // Observers bypass system-set gating; freeze intent changes while the
    // pause overlay is up (review R1.1). Releases stay ungated so held keys
    // clear cleanly during a pause.
    if *pause.get() == crate::PauseStates::Paused {
        return;
    }

    let (entity, autopilot) = ship.into_inner();
    match autopilot.map(|ap| ap.action) {
        // Toggle off an active STOP... (disengage stays ungated so a verb
        // disabled mid-maneuver can never strand the ship braking).
        Some(AutopilotAction::Stop) => {
            debug!("on_autopilot_stop_input: disengaging STOP");
            commands.entity(entity).remove::<Autopilot>();
        }
        // ...but braking overrides any other maneuver (or engages fresh) -
        // only if a live controller on this ship grants STOP. No controller,
        // or STOP withheld, and the press is a no-op (matches the dark hint).
        _ if ship_grants_verb(entity, FlightVerb::Stop, &q_verbs) => {
            debug!("on_autopilot_stop_input: engaging STOP");
            commands
                .entity(entity)
                .insert(Autopilot::engage(AutopilotAction::Stop));
        }
        _ => {
            debug!("on_autopilot_stop_input: STOP not granted by a controller");
        }
    }
}

fn on_autopilot_goto_input(
    _: On<Start<AutopilotGotoInput>>,
    mut commands: Commands,
    ship: Single<(Entity, Option<&Autopilot>, Option<&TravelLock>), With<PlayerSpaceshipMarker>>,
    q_verbs: ControllerVerbQuery,
    pause: Res<State<crate::PauseStates>>,
) {
    // Observers bypass system-set gating; freeze intent changes while the
    // pause overlay is up (review R1.1). Releases stay ungated so held keys
    // clear cleanly during a pause.
    if *pause.get() == crate::PauseStates::Paused {
        return;
    }

    let (entity, autopilot, travel) = ship.into_inner();

    // Already flying somewhere? G toggles the trip off. Disengage stays
    // ungated so a verb disabled mid-trip can never strand the ship in GOTO.
    if let Some(Autopilot {
        action: AutopilotAction::Goto { .. },
        ..
    }) = autopilot
    {
        debug!("on_autopilot_goto_input: disengaging GOTO");
        commands.entity(entity).remove::<Autopilot>();
        return;
    }

    // GOTO is granted by the controller: no live controller enabling it (the
    // shakedown withholds it until the first objective) and the press is a
    // no-op, matching the dark hint.
    if !ship_grants_verb(entity, FlightVerb::Goto, &q_verbs) {
        debug!("on_autopilot_goto_input: GOTO not granted by a controller");
        return;
    }

    // A destination needs a TRAVEL lock (the deliberate-radar designation);
    // without one this is a no-op (the status line keeps reading MAN). The
    // target is CAPTURED here, at [G] (decision D8): re-designating the
    // travel lock later does not re-route the engaged trip.
    let Some(target) = travel.and_then(|travel| travel.0) else {
        debug!("on_autopilot_goto_input: no travel lock, nothing to fly to");
        return;
    };

    debug!("on_autopilot_goto_input: engaging GOTO {target:?}");
    commands
        .entity(entity)
        .insert(Autopilot::engage(AutopilotAction::Goto { target }));
}

fn on_autopilot_orbit_input(
    _: On<Start<AutopilotOrbitInput>>,
    mut commands: Commands,
    ship: Single<(Entity, Option<&Autopilot>, Option<&DominantWell>), With<PlayerSpaceshipMarker>>,
    q_verbs: ControllerVerbQuery,
    pause: Res<State<crate::PauseStates>>,
) {
    // Observers bypass system-set gating; freeze intent changes while the
    // pause overlay is up (review R1.1). Releases stay ungated so held keys
    // clear cleanly during a pause.
    if *pause.get() == crate::PauseStates::Paused {
        return;
    }

    let (entity, autopilot, dominant) = ship.into_inner();

    // Already orbiting? O toggles the parking off. Disengage stays ungated so
    // a verb disabled mid-orbit can never strand the ship station-keeping.
    if let Some(Autopilot {
        action: AutopilotAction::Orbit { .. },
        ..
    }) = autopilot
    {
        debug!("on_autopilot_orbit_input: disengaging ORBIT");
        commands.entity(entity).remove::<Autopilot>();
        return;
    }

    // ORBIT is granted by the controller: no live controller enabling it and
    // the press is a no-op, matching the dark hint.
    if !ship_grants_verb(entity, FlightVerb::Orbit, &q_verbs) {
        debug!("on_autopilot_orbit_input: ORBIT not granted by a controller");
        return;
    }

    // Parking needs a well; outside every SOI this is a no-op (the status
    // line shows no GRAV state, which is the v1 hint).
    let Some(well) = dominant else {
        debug!("on_autopilot_orbit_input: no dominant well, nothing to orbit");
        return;
    };

    debug!(
        "on_autopilot_orbit_input: engaging ORBIT around {:?}",
        **well
    );
    commands.entity(entity).insert(Autopilot::engage(
        // The plan (ring + plane) is computed by the autopilot on its first
        // engaged tick - the input layer only names the well.
        AutopilotAction::Orbit {
            well: **well,
            plan: None,
        },
    ));
}

fn on_autopilot_off_input(
    _: On<Start<AutopilotOffInput>>,
    mut commands: Commands,
    ship: Single<(Entity, Has<Autopilot>), With<PlayerSpaceshipMarker>>,
    pause: Res<State<crate::PauseStates>>,
) {
    // Observers bypass system-set gating; freeze intent changes while the
    // pause overlay is up (review R1.1). Releases stay ungated so held keys
    // clear cleanly during a pause.
    if *pause.get() == crate::PauseStates::Paused {
        return;
    }

    let (entity, engaged) = ship.into_inner();
    if engaged {
        debug!("on_autopilot_off_input: disengaging");
        commands.entity(entity).remove::<Autopilot>();
    }
}

/// Mouse-motion -> `RcsIntent` gain: how far one frame's mouse delta drives the
/// (delta-driven) intent before the per-tick decay bleeds it off. Small, so a
/// deliberate sweep crosses the range and a twitch barely moves it. Feel-tunable
/// (task 20260718-122912; nudged up 0.02 -> 0.03 in 20260718-192708).
const RCS_AIM_SENSITIVITY: f32 = 0.03;

/// Enter RCS fine-adjust mode: while SHIFT is held on a ship whose controller
/// grants the RCS verb, mark it [`RcsActive`] (the modal gate the helm, camera
/// and scroll all read) and disengage any autopilot - entering RCS is a flight
/// input, exactly like grabbing the throttle (`on_flight_burn_input`).
fn on_rcs_modifier_start(
    _: On<Start<RcsModifierInput>>,
    mut commands: Commands,
    ship: Single<Entity, With<PlayerSpaceshipMarker>>,
    q_verbs: ControllerVerbQuery,
    pause: Res<State<crate::PauseStates>>,
) {
    if *pause.get() == crate::PauseStates::Paused {
        return;
    }
    let entity = *ship;
    if !ship_grants_verb(entity, FlightVerb::Rcs, &q_verbs) {
        debug!("on_rcs_modifier_start: RCS not granted by a controller");
        return;
    }
    debug!("on_rcs_modifier_start: entering RCS fine-adjust");
    commands
        .entity(entity)
        .insert(RcsActive)
        .remove::<Autopilot>();
}

/// Leave RCS mode on SHIFT release: drop [`RcsActive`] and zero the held
/// virtual-joystick offset so the ship stops adding RCS force (its residual
/// velocity persists - Newtonian - per spike Q2). NOT pause-gated: a release
/// must always clean up, like the other input releases.
fn on_rcs_modifier_released(
    _: On<Complete<RcsModifierInput>>,
    mut commands: Commands,
    // `RcsIntent` is optional so a ship that somehow lacks it can still LEAVE
    // RCS: the modal `RcsActive` (which freezes the helm and view) must always
    // clear on release, never get stranded behind a missing component.
    ship: Single<(Entity, Option<&mut RcsIntent>), With<PlayerSpaceshipMarker>>,
) {
    let (entity, intent) = ship.into_inner();
    if let Some(mut intent) = intent {
        intent.0 = Vec3::ZERO;
    }
    commands.entity(entity).remove::<RcsActive>();
}

/// Accumulate mouse motion into the ship-local `RcsIntent` XZ plane while RCS is
/// active: mouse X -> strafe (+X), mouse Y -> forward/back (Z). Held-direction,
/// so the offset persists when the mouse stops; the pilot pulls back to null it.
/// A no-op unless the ship is [`RcsActive`], so the shared mouse_motion binding
/// does nothing outside RCS mode.
fn on_rcs_aim(
    fire: On<Fire<RcsAimInput>>,
    ship: Single<(&mut RcsIntent, Has<RcsActive>), With<PlayerSpaceshipMarker>>,
    pause: Res<State<crate::PauseStates>>,
) {
    if *pause.get() == crate::PauseStates::Paused {
        return;
    }
    let (mut intent, active) = ship.into_inner();
    if !active {
        return;
    }
    // DELTA-driven (playtest 2026-07-18): SET the intent from THIS frame's mouse
    // motion rather than accumulating a persistent offset - the held-direction
    // joystick was too hard to control because it kept pushing after the mouse
    // stopped. `decay_player_rcs_intent` fades this to zero when the mouse stops,
    // so force follows motion.
    let delta = (fire.value * RCS_AIM_SENSITIVITY).clamp(Vec2::splat(-1.0), Vec2::splat(1.0));
    intent.x = delta.x;
    // Bevy mouse-motion Y is +down; pushing the mouse forward (up, -y) drives
    // the ship forward (ship-local -Z), pulling back drives it aft.
    intent.z = delta.y;
}

/// The player input bindings that fire a thruster section, snapshotted from its
/// content `input_mapping` onto the section entity. One section may bind several
/// [`Binding`]s. Must not reuse a [`flight_rig_reserved_sources`] source.
#[derive(Component, Debug, Clone, Deref, DerefMut, Reflect)]
pub struct SpaceshipThrusterInputBinding(pub Vec<Binding>);

#[derive(Component, Debug, Clone)]
struct ThrusterInputMarker;

#[derive(InputAction)]
#[action_output(bool)]
struct ThrusterInput;

fn on_thruster_input_binding(
    add: On<Add, SpaceshipThrusterInputBinding>,
    mut commands: Commands,
    q_binding: Query<&SpaceshipThrusterInputBinding>,
) {
    let entity = add.entity;
    trace!("on_thruster_input_binding: entity {:?}", entity);

    let Ok(binding) = q_binding.get(entity) else {
        error!(
            "on_thruster_input_binding: entity {:?} not found in q_binding",
            entity
        );
        return;
    };

    commands.entity(entity).insert((
        ThrusterInputMarker,
        actions!(
            ThrusterInputMarker[(
                Name::new("Input: Thruster"),
                Action::<ThrusterInput>::new(),
                ActionSettings {
                    consume_input: false,
                    ..default()
                },
                Bindings::spawn(binding.0.clone()),
            )]
        ),
    ));
}

fn on_thruster_input(
    fire: On<Start<ThrusterInput>>,
    mut commands: Commands,
    mut q_input: Query<(&mut ThrusterSectionInput, Option<&ChildOf>), With<ThrusterInputMarker>>,
    pause: Res<State<crate::PauseStates>>,
) {
    // Observers bypass system-set gating; freeze intent changes while the
    // pause overlay is up (review R1.1). Releases stay ungated so held keys
    // clear cleanly during a pause.
    if *pause.get() == crate::PauseStates::Paused {
        return;
    }

    let entity = fire.event().context;
    trace!("on_thruster_input: entity {:?}", entity);

    let Ok((mut input, child_of)) = q_input.get_mut(entity) else {
        error!(
            "on_thruster_input: entity {:?} not found in q_input",
            entity
        );
        return;
    };

    **input = 1.0;
    // Grabbing a bound throttle is a flight input: it takes the ship back
    // from an engaged autopilot (removing an absent component is a no-op).
    if let Some(&ChildOf(ship)) = child_of {
        commands.entity(ship).remove::<Autopilot>();
    }
}

fn on_thruster_input_completed(
    fire: On<Complete<ThrusterInput>>,
    mut q_input: Query<&mut ThrusterSectionInput, With<ThrusterInputMarker>>,
) {
    let entity = fire.event().context;
    trace!("on_thruster_input_completed: entity {:?}", entity);

    let Ok(mut input) = q_input.get_mut(entity) else {
        return;
    };

    **input = 0.0;
}

/// The player input bindings that fire a turret section, snapshotted from its
/// content `input_mapping`. Same rules as [`SpaceshipThrusterInputBinding`].
#[derive(Component, Debug, Clone, Deref, DerefMut, Reflect)]
pub struct SpaceshipTurretInputBinding(pub Vec<Binding>);

#[derive(Component, Debug, Clone)]
struct TurretInputMarker;

#[derive(InputAction)]
#[action_output(bool)]
struct TurretInput;

fn on_turret_input_binding(
    add: On<Add, SpaceshipTurretInputBinding>,
    mut commands: Commands,
    q_binding: Query<&SpaceshipTurretInputBinding>,
) {
    let entity = add.entity;
    trace!("on_turret_input_binding: entity {:?}", entity);

    let Ok(binding) = q_binding.get(entity) else {
        return;
    };

    commands.entity(entity).insert((
        TurretInputMarker,
        actions!(
            TurretInputMarker[(
                Name::new("Input: Turret"),
                Action::<TurretInput>::new(),
                ActionSettings {
                    consume_input: false,
                    ..default()
                },
                Bindings::spawn(binding.0.clone()),
            )]
        ),
    ));
}

fn on_turret_input(
    fire: On<Start<TurretInput>>,
    mut q_input: Query<&mut TurretSectionInput, With<TurretInputMarker>>,
    q_player_safety: Query<
        (&WeaponsHot, Option<&RadarState>),
        (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>),
    >,
    pause: Res<State<crate::PauseStates>>,
) {
    // Observers bypass system-set gating; freeze intent changes while the
    // pause overlay is up (review R1.1). Releases stay ungated so held keys
    // clear cleanly during a pause.
    if *pause.get() == crate::PauseStates::Paused {
        return;
    }

    // The weapons safety denies the PRESS on a managed cold ship (the live
    // section-side gate is the enforcement; this is the immediate feedback
    // path - the input bool never even latches). HOLD_FIRE_DURING_RADAR:
    // optional playtest flag from the adversarial round (sweeping with the
    // trigger down rakes bystanders); off by default.
    let cold = q_player_safety
        .iter()
        .next()
        .is_some_and(|(hot, radar)| !hot.0 || (HOLD_FIRE_DURING_RADAR && radar.is_some()));
    if cold {
        return;
    }

    let entity = fire.event().context;
    trace!("on_turret_input: entity {:?}", entity);

    let Ok(mut input) = q_input.get_mut(entity) else {
        return;
    };

    **input = true;
}

fn on_turret_input_completed(
    fire: On<Complete<TurretInput>>,
    mut q_input: Query<&mut TurretSectionInput, With<TurretInputMarker>>,
) {
    let entity = fire.event().context;
    trace!("on_turret_input_completed: entity {:?}", entity);

    let Ok(mut input) = q_input.get_mut(entity) else {
        return;
    };

    **input = false;
}

/// The player input bindings that fire a torpedo section, snapshotted from its
/// content `input_mapping`. Same rules as [`SpaceshipThrusterInputBinding`].
#[derive(Component, Debug, Clone, Deref, DerefMut, Reflect)]
pub struct SpaceshipTorpedoInputBinding(pub Vec<Binding>);

#[derive(Component, Debug, Clone)]
struct TorpedoInputMarker;

#[derive(InputAction)]
#[action_output(bool)]
struct TorpedoInput;

fn on_torpedo_input_binding(
    add: On<Add, SpaceshipTorpedoInputBinding>,
    mut commands: Commands,
    q_binding: Query<&SpaceshipTorpedoInputBinding>,
) {
    let entity = add.entity;
    trace!("on_torpedo_input_binding: entity {:?}", entity);

    let Ok(binding) = q_binding.get(entity) else {
        return;
    };

    commands.entity(entity).insert((
        TorpedoInputMarker,
        actions!(
            TorpedoInputMarker[(
                Name::new("Input: Torpedo"),
                Action::<TorpedoInput>::new(),
                ActionSettings {
                    consume_input: false,
                    ..default()
                },
                Bindings::spawn(binding.0.clone()),
            )]
        ),
    ));
}

fn on_torpedo_input(
    fire: On<Start<TorpedoInput>>,
    mut q_input: Query<&mut TorpedoSectionInput, With<TorpedoInputMarker>>,
    q_player_safety: Query<
        (&WeaponsHot, Option<&RadarState>),
        (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>),
    >,
    pause: Res<State<crate::PauseStates>>,
) {
    // Observers bypass system-set gating; freeze intent changes while the
    // pause overlay is up (review R1.1). Releases stay ungated so held keys
    // clear cleanly during a pause.
    if *pause.get() == crate::PauseStates::Paused {
        return;
    }

    // The weapons safety denies the PRESS on a managed cold ship (the live
    // section-side gate is the enforcement; this is the immediate feedback
    // path - the input bool never even latches). HOLD_FIRE_DURING_RADAR:
    // optional playtest flag from the adversarial round (sweeping with the
    // trigger down rakes bystanders); off by default.
    let cold = q_player_safety
        .iter()
        .next()
        .is_some_and(|(hot, radar)| !hot.0 || (HOLD_FIRE_DURING_RADAR && radar.is_some()));
    if cold {
        return;
    }

    let entity = fire.event().context;
    trace!("on_torpedo_input: entity {:?}", entity);

    let Ok(mut input) = q_input.get_mut(entity) else {
        return;
    };

    **input = true;
}

fn on_torpedo_input_completed(
    fire: On<Complete<TorpedoInput>>,
    mut q_input: Query<&mut TorpedoSectionInput, With<TorpedoInputMarker>>,
) {
    let entity = fire.event().context;
    trace!("on_torpedo_input_completed: entity {:?}", entity);

    let Ok(mut input) = q_input.get_mut(entity) else {
        return;
    };

    **input = false;
}

#[cfg(test)]
mod command_lag_tests {
    // Kept as its own module for its distinct harness (manual time), but
    // named and placed beside `tests` deliberately.
    use core::time::Duration;

    use bevy::time::TimeUpdateStrategy;

    use super::*;

    /// The drift guard for [`flight_rig_reserved_sources`]: build the REAL
    /// flight rig and confirm the hand-authored reserved list names exactly the
    /// rig's discrete-button sources - no more, no less. If a future edit adds,
    /// removes or rebinds a flight action, this fails until the reserved list
    /// (and the content lint that reads it) is updated, so `input-mapping-
    /// overlays-flight-rig` cannot silently regress.
    #[test]
    fn flight_rig_reserves_exactly_these_sources() {
        use std::collections::HashSet;

        use bevy::input::InputPlugin;

        let mut app = App::new();
        app.add_plugins((MinimalPlugins, InputPlugin, EnhancedInputPlugin));
        app.add_input_context::<FlightInputMarker>();
        // The context registry finalizes in App::finish; run the lifecycle
        // before spawning the rig, exactly as the production app does.
        app.finish();
        app.cleanup();
        app.update();
        app.world_mut().spawn(flight_input_rig());
        app.update();

        let mut rig_sources: HashSet<InputSource> = HashSet::new();
        let mut q = app.world_mut().query::<&Binding>();
        for binding in q.iter(app.world()) {
            if let Some(source) = binding_source(binding) {
                rig_sources.insert(source);
            }
        }

        let reserved: HashSet<InputSource> = flight_rig_reserved_sources()
            .into_iter()
            .map(|(s, _)| s)
            .collect();

        let missing: Vec<_> = rig_sources.difference(&reserved).collect();
        let extra: Vec<_> = reserved.difference(&rig_sources).collect();
        assert!(
            missing.is_empty() && extra.is_empty(),
            "flight_rig_reserved_sources drifted from the real rig.\n  \
             in the rig but not reserved (add them): {missing:?}\n  \
             reserved but not in the rig (remove them): {extra:?}"
        );
    }

    /// A mouse 180 must NOT reach the rotation command in one frame: the
    /// command slews at the hull's torque-budget turn rate, so the PD tracks
    /// a small error instead of saturating (flip-wobble fix) and a heavy
    /// hull audibly lags the camera (flight-feel retune, 20260709-095043).
    #[test]
    fn a_camera_flip_reaches_the_command_over_many_frames() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
            1.0 / 60.0,
        )));
        app.init_resource::<FlightSettings>();
        app.add_systems(Update, update_controller_target_rotation_torque);

        let target = Quat::from_rotation_y(core::f32::consts::PI);
        app.world_mut().spawn((
            SpaceshipCameraInputMarker,
            SpaceshipCameraNormalInputMarker,
            PointRotationOutput(target),
        ));
        // The stock ship's numbers: inertia ~2.3, computer torque 10.
        let ship = app
            .world_mut()
            .spawn((
                SpaceshipRootMarker,
                PlayerSpaceshipMarker,
                Transform::default(),
                ComputedAngularInertia::new(Vec3::splat(2.3)),
            ))
            .id();
        let controller = app
            .world_mut()
            .spawn((
                ChildOf(ship),
                ControllerSectionMarker,
                PDController {
                    frequency: 4.0,
                    damping_ratio: 4.0,
                    max_torque: 10.0,
                },
                ControllerSectionRotationInput::default(),
            ))
            .id();

        // First update has dt = 0; the second advances one real frame.
        app.update();
        app.update();

        let command = **app
            .world()
            .get::<ControllerSectionRotationInput>(controller)
            .unwrap();
        let moved = command.angle_between(Quat::IDENTITY);
        let remaining = command.angle_between(target);
        // One frame advances exactly one slew step of the DERIVED rate - this
        // pins hull_turn_rate's wiring, not just "some" slew.
        let expected = crate::flight::hull_turn_rate(
            10.0,
            2.3,
            &app.world().resource::<FlightSettings>().clone(),
        ) / 60.0;
        assert!(
            (moved - expected).abs() < expected * 0.15,
            "one frame must advance one torque-budget slew step \
             (moved {moved}, expected {expected})"
        );
        assert!(
            remaining > 2.0,
            "a 180 flip must not reach the command in one frame ({remaining} left)"
        );
    }
}

#[cfg(test)]
mod tests {
    use bevy::ecs::system::RunSystemOnce;

    use super::*;

    /// The settings-menu keybind reference ([`crate::input::reference`]) is
    /// authored static data because it renders in the main menu where no rig is
    /// spawned. This pins each reference row's DISPLAYED keyboard string to the
    /// key the live `flight_input_rig` actually binds: the expected label is
    /// derived from the rig's current key (not a hardcoded constant), so a remap
    /// of the rig either flips the derived label (and the row no longer contains
    /// it - assert fails) or hits an unmapped key (`display_label` panics). Both
    /// force the reference in `reference.rs` to be revisited in the same change;
    /// the readout cannot silently drift from the rig (would-it-fail-without-it).
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

    /// A world with the flight rig's four autopilot actions bound as in
    /// the real rig, plus the resources the resolver reads.
    fn hint_world() -> World {
        let mut world = World::new();
        world.init_resource::<FlightVerbHints>();
        world.spawn((
            Action::<AutopilotStopInput>::new(),
            bindings![KeyCode::KeyX, GamepadButton::East],
        ));
        world.spawn((
            Action::<AutopilotGotoInput>::new(),
            bindings![KeyCode::KeyG, GamepadButton::North],
        ));
        world.spawn((
            Action::<AutopilotOrbitInput>::new(),
            bindings![KeyCode::KeyO, GamepadButton::South],
        ));
        world.spawn((
            Action::<AutopilotOffInput>::new(),
            bindings![KeyCode::KeyZ, GamepadButton::West],
        ));
        world
    }

    /// A flyable player ship: live controller (with PD, all verbs granted) +
    /// live thruster. Mirrors the production `controller_section` bundle, which
    /// carries NO [`WithheldVerbs`] by default (an absent component grants every
    /// verb); tests that withhold a verb insert a `WithheldVerbs` on the
    /// returned controller.
    fn spawn_flyable_ship(world: &mut World) -> (Entity, Entity) {
        let ship = world.spawn((PlayerSpaceshipMarker, targeting_state())).id();
        let controller = world
            .spawn((
                ChildOf(ship),
                ControllerSectionMarker,
                PDController {
                    frequency: 4.0,
                    damping_ratio: 4.0,
                    max_torque: 40.0,
                },
            ))
            .id();
        world.spawn((ChildOf(ship), ThrusterSectionMarker));
        (ship, controller)
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
        // autopilot would strip the maneuver on its next tick, so a lit
        // hint would be a lie (review R1.1).
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

    /// End-to-end through the REAL flight rig and EnhancedInputPlugin: a GOTO
    /// keypress engages the autopilot only when a live controller grants GOTO.
    /// With the verb withheld the press is a no-op even with a valid lock; the
    /// gate deleted, the first press would engage and this test would fail.
    #[test]
    fn goto_keypress_is_gated_by_the_controller_verb_flag() {
        use bevy::input::InputPlugin;

        let mut app = App::new();
        app.add_plugins((MinimalPlugins, InputPlugin, EnhancedInputPlugin));
        // The autopilot observers are pause-gated (task 20260711-185156).
        app.add_plugins(bevy::state::app::StatesPlugin);
        app.init_state::<crate::PauseStates>();
        app.add_input_context::<FlightInputMarker>();
        app.add_observer(on_autopilot_goto_input);

        // A player ship whose controller withholds GOTO, plus a valid lock.
        let (ship, controller) = spawn_flyable_ship(app.world_mut());
        app.world_mut()
            .entity_mut(controller)
            .insert(WithheldVerbs([FlightVerb::Goto].into_iter().collect()));
        let target = app.world_mut().spawn_empty().id();
        app.world_mut()
            .entity_mut(ship)
            .insert(TravelLock(Some(target)));

        // The context registry finalizes in App::finish; run the lifecycle
        // before spawning the rig, like the production app does.
        app.finish();
        app.cleanup();
        app.update();
        app.world_mut().spawn(flight_input_rig());
        app.update();

        // Press G with GOTO withheld: nothing engages.
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::KeyG);
        app.update();
        app.update();
        assert!(
            app.world().get::<Autopilot>(ship).is_none(),
            "GOTO withheld: the keypress must not engage the autopilot"
        );

        // Release, grant GOTO, press again: now it engages on the lock.
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .release(KeyCode::KeyG);
        app.update();
        app.world_mut()
            .entity_mut(controller)
            .insert(WithheldVerbs::default());
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::KeyG);
        app.update();
        app.update();
        assert!(
            matches!(
                app.world().get::<Autopilot>(ship).map(|ap| ap.action),
                Some(AutopilotAction::Goto { target: t }) if t == target
            ),
            "GOTO granted: the keypress engages GOTO on the lock"
        );
    }

    /// The full SHIFT gesture through the real rig: press enters RCS (marks the
    /// ship `RcsActive`, which is what freezes the helm, and disengages any
    /// autopilot); release exits and zeroes the held offset. Asserts after each
    /// step (`assert-each-gesture-step`).
    #[test]
    fn rcs_shift_gesture_enters_exits_and_disengages_autopilot() {
        use bevy::input::InputPlugin;

        let mut app = App::new();
        app.add_plugins((MinimalPlugins, InputPlugin, EnhancedInputPlugin));
        app.add_plugins(bevy::state::app::StatesPlugin);
        app.init_state::<crate::PauseStates>();
        app.add_input_context::<FlightInputMarker>();
        app.add_observer(on_rcs_modifier_start);
        app.add_observer(on_rcs_modifier_released);

        let (ship, _controller) = spawn_flyable_ship(app.world_mut());
        // Production inserts a default RcsIntent on player ships; add one plus an
        // engaged autopilot to prove entering RCS both zeroes on exit and
        // disengages the maneuver.
        app.world_mut().entity_mut(ship).insert((
            RcsIntent(Vec3::new(0.2, 0.1, -0.3)),
            Autopilot::engage(AutopilotAction::Stop),
        ));

        app.finish();
        app.cleanup();
        app.update();
        app.world_mut().spawn(flight_input_rig());
        app.update();

        // Press SHIFT: RCS entered, autopilot gone.
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::ShiftLeft);
        app.update();
        app.update();
        assert!(
            app.world().get::<RcsActive>(ship).is_some(),
            "SHIFT on an RCS-granting ship enters fine-adjust"
        );
        assert!(
            app.world().get::<Autopilot>(ship).is_none(),
            "entering RCS disengages the autopilot (a flight input)"
        );
        // The helm's authority query is `Without<RcsActive>`; prove the ship is
        // now excluded from it, i.e. the heading is frozen.
        let mut helm_q = app
            .world_mut()
            .query_filtered::<Entity, (With<PlayerSpaceshipMarker>, Without<RcsActive>)>();
        assert_eq!(
            helm_q.iter(app.world()).count(),
            0,
            "RcsActive excludes the ship from manual rotation authority"
        );

        // Release SHIFT: RCS exited, held offset zeroed.
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .release(KeyCode::ShiftLeft);
        app.update();
        app.update();
        assert!(
            app.world().get::<RcsActive>(ship).is_none(),
            "releasing SHIFT exits RCS"
        );
        assert_eq!(
            app.world().get::<RcsIntent>(ship).unwrap().0,
            Vec3::ZERO,
            "releasing SHIFT zeroes the held virtual-joystick offset"
        );
    }

    /// RCS is a controller verb: SHIFT on a ship whose controller withholds
    /// `Rcs` does not enter the mode. Deleting the `ship_grants_verb` gate would
    /// engage it here and fail the test.
    #[test]
    fn rcs_shift_is_gated_by_the_controller_verb() {
        use bevy::input::InputPlugin;

        let mut app = App::new();
        app.add_plugins((MinimalPlugins, InputPlugin, EnhancedInputPlugin));
        app.add_plugins(bevy::state::app::StatesPlugin);
        app.init_state::<crate::PauseStates>();
        app.add_input_context::<FlightInputMarker>();
        app.add_observer(on_rcs_modifier_start);

        let (ship, controller) = spawn_flyable_ship(app.world_mut());
        app.world_mut()
            .entity_mut(controller)
            .insert(WithheldVerbs([FlightVerb::Rcs].into_iter().collect()));

        app.finish();
        app.cleanup();
        app.update();
        app.world_mut().spawn(flight_input_rig());
        app.update();

        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::ShiftLeft);
        app.update();
        app.update();
        assert!(
            app.world().get::<RcsActive>(ship).is_none(),
            "RCS withheld: SHIFT must not enter fine-adjust"
        );
    }

    /// While RCS is active, mouse motion drives the ship-local `RcsIntent` XZ
    /// plane (strafe + forward/back) from THIS frame's delta - SET, not a running
    /// accumulate (playtest 2026-07-18: delta-driven, not a joystick). Outside RCS
    /// the same motion is ignored.
    #[test]
    fn rcs_mouse_motion_sets_intent_from_the_delta_only_while_active() {
        use bevy::input::{mouse::MouseMotion, InputPlugin};

        let mut app = App::new();
        app.add_plugins((MinimalPlugins, InputPlugin, EnhancedInputPlugin));
        app.add_plugins(bevy::state::app::StatesPlugin);
        app.init_state::<crate::PauseStates>();
        app.add_input_context::<FlightInputMarker>();
        app.add_observer(on_rcs_modifier_start);
        app.add_observer(on_rcs_modifier_released);
        app.add_observer(on_rcs_aim);

        let (ship, _controller) = spawn_flyable_ship(app.world_mut());
        app.world_mut()
            .entity_mut(ship)
            .insert(RcsIntent::default());

        app.finish();
        app.cleanup();
        app.update();
        app.world_mut().spawn(flight_input_rig());
        app.update();

        // Not in RCS yet: mouse motion must not move the intent.
        app.world_mut().write_message(MouseMotion {
            delta: Vec2::new(20.0, 0.0),
        });
        app.update();
        assert_eq!(
            app.world().get::<RcsIntent>(ship).unwrap().0,
            Vec3::ZERO,
            "mouse motion is ignored outside RCS mode"
        );

        // Enter RCS, then sweep the mouse right + forward (up = -y).
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::ShiftLeft);
        app.update();
        app.world_mut().write_message(MouseMotion {
            delta: Vec2::new(20.0, -20.0),
        });
        app.update();
        let intent = app.world().get::<RcsIntent>(ship).unwrap().0;
        assert!(intent.x > 0.0, "mouse-right strafes +X (got {intent:?})");
        assert!(
            intent.z < 0.0,
            "mouse-forward (up) drives the ship forward, -Z (got {intent:?})"
        );
        assert_eq!(intent.y, 0.0, "mouse does not touch the vertical axis");

        // A SECOND, smaller motion REPLACES the intent (delta-driven) - it does
        // NOT accumulate on top of the first. (No decay runs in this harness, so
        // the only reason x shrinks is the SET.)
        app.world_mut().write_message(MouseMotion {
            delta: Vec2::new(10.0, 0.0),
        });
        app.update();
        let intent = app.world().get::<RcsIntent>(ship).unwrap().0;
        assert!(
            (intent.x - 10.0 * RCS_AIM_SENSITIVITY).abs() < 1e-4,
            "x is the LAST delta (0.2), not the sum of both motions (got {})",
            intent.x
        );
        assert_eq!(
            intent.z, 0.0,
            "the second motion had no forward component, so z is set back to 0 (got {})",
            intent.z
        );
    }

    /// While RCS is active a scroll notch nudges the ship-local Y (up/down) axis
    /// of `RcsIntent` instead of stepping the component lock; the same scroll
    /// outside RCS leaves `RcsIntent` untouched (it cycles a component as
    /// before). Reverting the `RcsActive` branch in `on_component_cycle_next`
    /// leaves Y at zero in RCS and fails this.
    #[test]
    fn rcs_scroll_drives_the_vertical_axis_only_while_active() {
        use bevy::input::{
            mouse::{MouseScrollUnit, MouseWheel},
            InputPlugin,
        };

        use crate::input::targeting::on_component_cycle_next;

        let mut app = App::new();
        app.add_plugins((MinimalPlugins, InputPlugin, EnhancedInputPlugin));
        app.add_plugins(bevy::state::app::StatesPlugin);
        app.init_state::<crate::PauseStates>();
        app.add_input_context::<FlightInputMarker>();
        app.add_observer(on_component_cycle_next);

        let (ship, _controller) = spawn_flyable_ship(app.world_mut());
        app.world_mut()
            .entity_mut(ship)
            .insert(RcsIntent::default());

        app.finish();
        app.cleanup();
        app.update();
        app.world_mut().spawn(flight_input_rig());
        app.update();

        let scroll_up = |app: &mut App| {
            app.world_mut().write_message(MouseWheel {
                unit: MouseScrollUnit::Line,
                x: 0.0,
                y: 1.0,
                window: Entity::PLACEHOLDER,
                phase: bevy::input::touch::TouchPhase::Moved,
            });
            app.update();
            app.update();
        };

        // Scroll outside RCS: the vertical axis stays zero (it cycles instead).
        scroll_up(&mut app);
        assert_eq!(
            app.world().get::<RcsIntent>(ship).unwrap().0.y,
            0.0,
            "scroll outside RCS must not touch the vertical axis"
        );

        // Enter RCS, scroll up: the vertical axis rises.
        app.world_mut().entity_mut(ship).insert(RcsActive);
        scroll_up(&mut app);
        assert!(
            app.world().get::<RcsIntent>(ship).unwrap().0.y > 0.0,
            "scroll up in RCS raises the vertical axis (got {})",
            app.world().get::<RcsIntent>(ship).unwrap().0.y
        );
    }

    /// A controller with no `WithheldVerbs` component must stay flyable and
    /// grant every verb - the withheld set is decoupled from `flyable`, so a
    /// missing component falls back to the all-granted default and never bricks
    /// the ship. This is the production default (a controller carries
    /// `WithheldVerbs` only once a `DisableVerb`/`SetControllerVerb` touches it).
    /// Guards the fail-closed hazard (review MINOR 1).
    #[test]
    fn controller_without_verb_flags_is_flyable_and_grants_all_verbs() {
        let mut world = hint_world();
        // A live controller WITHOUT WithheldVerbs, plus a thruster: the
        // production default, matching a controller no modification has touched.
        let ship = world.spawn(PlayerSpaceshipMarker).id();
        world.spawn((
            ChildOf(ship),
            ControllerSectionMarker,
            PDController {
                frequency: 4.0,
                damping_ratio: 4.0,
                max_torque: 40.0,
            },
        ));
        world.spawn((ChildOf(ship), ThrusterSectionMarker));
        let lock = world.spawn_empty().id();
        let well = world.spawn_empty().id();
        world
            .entity_mut(ship)
            .insert((TravelLock(Some(lock)), DominantWell(well)));

        world.run_system_once(update_flight_verb_hints).unwrap();
        let hints = world.resource::<FlightVerbHints>().clone();
        assert!(hints.stop.available, "flyable despite no flags component");
        assert!(hints.goto.available, "GOTO defaults on without flags");
        assert!(hints.orbit.available, "ORBIT defaults on without flags");
    }

    #[test]
    fn no_lock_does_not_despawn_untargeted_torpedo() {
        // Regression: with no current lock, an un-targeted torpedo (e.g. one whose
        // target just died and had its link dropped) must keep flying, not vanish.
        let mut app = App::new();
        app.add_systems(Update, update_torpedo_target_input);

        let ship = app
            .world_mut()
            .spawn((SpaceshipRootMarker, PlayerSpaceshipMarker, CombatLock(None)))
            .id();
        let torpedo = app
            .world_mut()
            .spawn((TorpedoProjectileMarker, ProjectileOwner(ship)))
            .id();

        app.update();

        assert!(
            app.world().entities().contains(torpedo),
            "un-targeted torpedo must survive when there is no lock"
        );
        assert!(
            app.world().get::<TorpedoTargetEntity>(torpedo).is_none(),
            "no target should be assigned when there is no lock"
        );
        assert!(
            app.world().get::<TorpedoTargetChosen>(torpedo).is_some(),
            "the torpedo should be committed to dumb-fire"
        );
    }

    #[test]
    fn lock_assigns_target_to_owned_torpedo() {
        // With a lock, an owned un-targeted torpedo gets the target assigned and
        // is committed to it.
        let mut app = App::new();
        let target = app.world_mut().spawn_empty().id();
        app.add_systems(Update, update_torpedo_target_input);

        let ship = app
            .world_mut()
            .spawn((
                SpaceshipRootMarker,
                PlayerSpaceshipMarker,
                CombatLock(Some(target)),
            ))
            .id();
        let torpedo = app
            .world_mut()
            .spawn((TorpedoProjectileMarker, ProjectileOwner(ship)))
            .id();

        app.update();

        assert_eq!(
            app.world().get::<TorpedoTargetEntity>(torpedo).map(|t| **t),
            Some(target),
            "an owned torpedo should be assigned the locked target"
        );
        assert!(
            app.world().get::<TorpedoTargetChosen>(torpedo).is_some(),
            "the assignment should also commit the torpedo"
        );
    }

    #[test]
    fn dumbfire_torpedo_ignores_later_locks() {
        // THE bullet regression: a torpedo fired with no lock is committed to
        // dumb-fire; a lock appearing later (e.g. the aim cast hitting a bullet
        // fired down the crosshair ray) must not be assigned to it.
        let mut app = App::new();
        app.add_systems(Update, update_torpedo_target_input);

        let ship = app
            .world_mut()
            .spawn((SpaceshipRootMarker, PlayerSpaceshipMarker, CombatLock(None)))
            .id();
        let torpedo = app
            .world_mut()
            .spawn((TorpedoProjectileMarker, ProjectileOwner(ship)))
            .id();

        // Frame 1: no lock -> committed dumb-fire.
        app.update();
        assert!(app.world().get::<TorpedoTargetChosen>(torpedo).is_some());

        // A "bullet" gets combat-locked (deliberately) afterwards.
        let bullet = app.world_mut().spawn_empty().id();
        app.world_mut().get_mut::<CombatLock>(ship).unwrap().0 = Some(bullet);

        // Frame 2: the committed torpedo must NOT pick it up.
        app.update();
        assert!(
            app.world().get::<TorpedoTargetEntity>(torpedo).is_none(),
            "a dumb-fired torpedo must never acquire a target mid-flight"
        );
    }

    #[test]
    fn committed_torpedo_does_not_retarget_after_target_loss() {
        // A torpedo whose target died (link removed by update_target_position,
        // position frozen) keeps its commitment: a fresh lock must not re-target it.
        let mut app = App::new();
        let new_target = app.world_mut().spawn_empty().id();
        app.add_systems(Update, update_torpedo_target_input);

        let ship = app
            .world_mut()
            .spawn((
                SpaceshipRootMarker,
                PlayerSpaceshipMarker,
                CombatLock(Some(new_target)),
            ))
            .id();
        // Committed, un-targeted: the post-target-death state.
        let torpedo = app
            .world_mut()
            .spawn((
                TorpedoProjectileMarker,
                ProjectileOwner(ship),
                TorpedoTargetChosen,
            ))
            .id();

        app.update();

        assert!(
            app.world().get::<TorpedoTargetEntity>(torpedo).is_none(),
            "a torpedo keeps its first target for life - no re-targeting after loss"
        );
    }

    #[test]
    fn turret_aim_ray_bases_on_the_live_structure_anchor() {
        // COM offset perpendicular to the aim: the ray base must shift with
        // it (task 20260709-150711), or the turret aim point keeps a
        // parallax against the COM-anchored crosshair.
        let mut world = World::new();
        world.spawn((
            SpaceshipCameraInputMarker,
            SpaceshipCameraTurretInputMarker,
            PointRotationOutput(Quat::IDENTITY),
        ));
        let ship = world
            .spawn((
                SpaceshipRootMarker,
                PlayerSpaceshipMarker,
                Transform::from_translation(Vec3::new(10.0, 0.0, 0.0)),
                ComputedCenterOfMass(Vec3::new(2.0, 0.0, 0.0)),
                CombatLock(None),
                ComponentLock::default(),
            ))
            .id();
        let turret = world
            .spawn((
                TurretSectionMarker,
                TurretSectionTargetInput(None),
                TurretSectionTargetVelocity(Vec3::ZERO),
                ChildOf(ship),
            ))
            .id();

        world.run_system_once(update_turret_target_input).unwrap();

        assert_eq!(
            **world
                .entity(turret)
                .get::<TurretSectionTargetInput>()
                .unwrap(),
            Some(Vec3::new(12.0, 0.0, -100.0)),
            "aim ray base = anchor (12,0,0), not the root origin (10,0,0)"
        );
    }

    // -- three-tier turret auto-fire feed --

    /// Player + aim rig + one turret, a locked target ship (moving, with a
    /// shifted COM) and one of its sections. Returns (turret, target,
    /// section).
    fn turret_feed_world() -> (World, Entity, Entity, Entity, Entity) {
        let mut world = World::new();
        world.spawn((
            SpaceshipCameraInputMarker,
            SpaceshipCameraTurretInputMarker,
            PointRotationOutput(Quat::IDENTITY),
        ));
        let ship = world
            .spawn((
                SpaceshipRootMarker,
                PlayerSpaceshipMarker,
                Transform::IDENTITY,
                CombatLock(None),
                ComponentLock::default(),
            ))
            .id();
        let turret = world
            .spawn((
                TurretSectionMarker,
                TurretSectionTargetInput(None),
                TurretSectionTargetVelocity(Vec3::ZERO),
                ChildOf(ship),
            ))
            .id();
        let target = world
            .spawn((
                SpaceshipRootMarker,
                Transform::from_translation(Vec3::new(0.0, 0.0, -200.0)),
                ComputedCenterOfMass(Vec3::new(0.0, 0.0, 2.0)),
                LinearVelocity(Vec3::new(7.0, 0.0, 0.0)),
            ))
            .id();
        let section = world
            .spawn((
                SectionMarker,
                GlobalTransform::from_translation(Vec3::new(1.0, 0.5, -199.0)),
                ChildOf(target),
            ))
            .id();
        world.get_mut::<CombatLock>(ship).unwrap().0 = Some(target);
        (world, ship, turret, target, section)
    }

    fn turret_feed(world: &mut World, turret: Entity) -> (Option<Vec3>, Vec3) {
        world.run_system_once(update_turret_target_input).unwrap();
        let entity = world.entity(turret);
        (
            **entity.get::<TurretSectionTargetInput>().unwrap(),
            **entity.get::<TurretSectionTargetVelocity>().unwrap(),
        )
    }

    #[test]
    fn component_lock_feeds_the_section_position() {
        let (mut world, ship, turret, _, section) = turret_feed_world();
        world.get_mut::<ComponentLock>(ship).unwrap().section = Some(section);

        let (point, velocity) = turret_feed(&mut world, turret);

        assert_eq!(point, Some(Vec3::new(1.0, 0.5, -199.0)));
        assert_eq!(velocity, Vec3::new(7.0, 0.0, 0.0), "lock root velocity");
    }

    #[test]
    fn ship_lock_feeds_the_live_structure_anchor() {
        let (mut world, _ship, turret, _, _) = turret_feed_world();

        let (point, velocity) = turret_feed(&mut world, turret);

        // Anchor = target translation + COM offset (identity rotation).
        assert_eq!(point, Some(Vec3::new(0.0, 0.0, -198.0)));
        assert_eq!(velocity, Vec3::new(7.0, 0.0, 0.0));
    }

    #[test]
    fn no_lock_feeds_the_camera_ray_with_zero_velocity() {
        let (mut world, ship, turret, _, _) = turret_feed_world();
        world.get_mut::<CombatLock>(ship).unwrap().0 = None;

        let (point, velocity) = turret_feed(&mut world, turret);

        assert_eq!(point, Some(Vec3::new(0.0, 0.0, -100.0)));
        assert_eq!(velocity, Vec3::ZERO, "a commanded point has no velocity");
    }

    #[test]
    fn dead_section_falls_through_to_the_ship_lock() {
        let (mut world, ship, turret, _, section) = turret_feed_world();
        world.get_mut::<ComponentLock>(ship).unwrap().section = Some(section);
        world.despawn(section);

        let (point, velocity) = turret_feed(&mut world, turret);

        assert_eq!(point, Some(Vec3::new(0.0, 0.0, -198.0)));
        assert_eq!(velocity, Vec3::new(7.0, 0.0, 0.0));
    }

    #[test]
    fn dead_lock_falls_through_to_the_camera_ray() {
        let (mut world, _ship, turret, target, _) = turret_feed_world();
        world.despawn(target);

        let (point, velocity) = turret_feed(&mut world, turret);

        assert_eq!(point, Some(Vec3::new(0.0, 0.0, -100.0)));
        assert_eq!(velocity, Vec3::ZERO);
    }

    /// D8 capture semantics: the engaged GOTO holds the target captured at
    /// [G]; re-designating the travel lock must NOT re-route the trip.
    #[test]
    fn goto_keeps_the_captured_target_across_re_designation() {
        let mut world = World::new();
        let a = world.spawn_empty().id();
        let b = world.spawn_empty().id();
        let ship = world
            .spawn((
                PlayerSpaceshipMarker,
                TravelLock(Some(a)),
                Autopilot::engage(AutopilotAction::Goto { target: a }),
            ))
            .id();

        // Radar re-designates the travel lock to B mid-flight.
        world.get_mut::<TravelLock>(ship).unwrap().0 = Some(b);

        let autopilot = world.get::<Autopilot>(ship).unwrap();
        assert!(
            matches!(autopilot.action, AutopilotAction::Goto { target } if target == a),
            "the engaged GOTO keeps the target captured at [G]"
        );
    }

    #[test]
    fn the_combat_lock_holds_the_turrets_even_while_raised() {
        // LOCK-WINS routing (playtest verdict 2026-07-13, task
        // 20260713-121605, flipping the manual-wins knob): while RAISED with
        // a combat lock, moving the cursor must NOT pull the turrets off the
        // target - the lock tiers win. This test fails against the retired
        // manual-wins feed by construction (it asserted the look ray here).
        let (mut world, ship, turret, _, section) = turret_feed_world();
        world.get_mut::<ComponentLock>(ship).unwrap().section = Some(section);
        world.entity_mut(ship).insert(WeaponsRaised(true));

        let (point, velocity) = turret_feed(&mut world, turret);
        assert_eq!(
            point,
            Some(Vec3::new(1.0, 0.5, -199.0)),
            "raised with a lock: the turrets stay on the locked section"
        );
        assert_eq!(
            velocity,
            Vec3::new(7.0, 0.0, 0.0),
            "the lock tier carries the target's lead velocity"
        );

        // Tap-clear (the lock and its fine-lock go away): still raised, the
        // turrets hand over to the cursor - manual gunnery is the NO-LOCK
        // stance now.
        world.get_mut::<CombatLock>(ship).unwrap().0 = None;
        world.get_mut::<ComponentLock>(ship).unwrap().section = None;
        let (point, velocity) = turret_feed(&mut world, turret);
        assert_eq!(
            point,
            Some(Vec3::new(0.0, 0.0, -100.0)),
            "clearing the lock hands the turrets to the look ray"
        );
        assert_eq!(
            velocity,
            Vec3::ZERO,
            "manual aim commands a point, no lead velocity"
        );
    }
}
