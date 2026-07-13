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
        binding_label, FlightVerbHints, PlayerSpaceshipMarker, SpaceshipPlayerInputPlugin,
        SpaceshipThrusterInputBinding, SpaceshipTorpedoInputBinding, SpaceshipTurretInputBinding,
        VerbHint,
    };
}

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
/// [`update_flight_verb_hints`] - computed here, where the verbs and their
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
/// list, keyboards via [`keyboard_label`] and mouse buttons as `LMB`/`RMB`/`MMB`.
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
        (&ChildOf, Option<&ControllerVerbs>),
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
    // so a controller missing the flags component can never brick the ship - it
    // just falls back to the all-on default (matching `ControllerVerbs::default`
    // and the pre-flags behavior). The `SetControllerVerb` action flips these.
    let verb_granted = |verb: FlightVerb| -> bool {
        ship.is_some_and(|ship| {
            q_computer.iter().any(|(&ChildOf(parent), verbs)| {
                parent == ship && verbs.is_none_or(|verbs| verbs.granted(verb))
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

/// Query over every live controller section and its (optional) granted verbs,
/// shared by the three maneuver observers so they gate execution on the same
/// controller-provided capability the hint pass shows. `ControllerVerbs` is
/// optional for the same reason as in the hint pass: a controller missing the
/// flags falls back to the all-on default rather than becoming ungovernable.
type ControllerVerbQuery<'w, 's> = Query<
    'w,
    's,
    (&'static ChildOf, Option<&'static ControllerVerbs>),
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
    q_verbs
        .iter()
        .any(|(&ChildOf(parent), verbs)| parent == ship && verbs.is_none_or(|v| v.granted(verb)))
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
    /// always carries [`ControllerVerbs`]; the hint pass and the observers both
    /// require it, so a controller without it would match nothing.
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
                ControllerVerbs::default(),
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
        world.entity_mut(controller).insert(ControllerVerbs {
            stop: true,
            goto: false,
            orbit: false,
            lock: true,
        });
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
        // flag, not some other condition, was the gate.
        world
            .entity_mut(controller)
            .insert(ControllerVerbs::default());
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
            .insert(ControllerVerbs {
                stop: true,
                goto: false,
                orbit: true,
                lock: true,
            });
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
            .insert(ControllerVerbs::default());
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

    /// A controller that predates the flags (no `ControllerVerbs` component)
    /// must stay flyable and grant every verb - the flags are decoupled from
    /// `flyable`, so a missing component falls back to the all-on default and
    /// never bricks the ship. Guards the fail-closed hazard (review MINOR 1).
    #[test]
    fn controller_without_verb_flags_is_flyable_and_grants_all_verbs() {
        let mut world = hint_world();
        // A live controller WITHOUT ControllerVerbs, plus a thruster: not the
        // production bundle, but the shape a legacy/hand-built spawn could take.
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
