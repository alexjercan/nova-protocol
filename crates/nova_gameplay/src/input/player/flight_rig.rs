//! The always-on flight rig: the input actions the player ship spawns with
//! and the observers that turn them into burns, autopilot verbs and RCS
//! fine-adjust.

use bevy::prelude::*;
use bevy_enhanced_input::prelude::*;

use crate::{
    input::targeting::{
        ComponentCycleNextInput, ComponentCyclePrevInput, RadarClearInput, RadarHoldInput,
    },
    prelude::*,
};

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
pub(super) struct FlightBurnInput;

/// Engage the STOP maneuver (kill all velocity); pressing it again while
/// stopping disengages.
#[derive(InputAction)]
#[action_output(bool)]
pub(super) struct AutopilotStopInput;

/// Engage the GOTO maneuver on the current aim-assist lock; pressing it again
/// while flying there disengages.
#[derive(InputAction)]
#[action_output(bool)]
pub(super) struct AutopilotGotoInput;

/// Engage the ORBIT maneuver around the ship's dominant gravity well;
/// pressing it again while orbiting disengages. A no-op outside every SOI.
#[derive(InputAction)]
#[action_output(bool)]
pub(super) struct AutopilotOrbitInput;

/// Plain autopilot off.
#[derive(InputAction)]
#[action_output(bool)]
pub(super) struct AutopilotOffInput;

/// The RCS fine-adjust modifier: held (SHIFT) to enter the docking translation
/// mode. A plain Down action read as a held modifier (the `action_held` pattern,
/// not a binding Chord - see `modal-input-observer-dispatch`), whose Start/Stop
/// the observers turn into [`RcsActive`] on the player ship.
#[derive(InputAction)]
#[action_output(bool)]
pub(super) struct RcsModifierInput;

/// The RCS aim: raw mouse motion (a per-frame `Vec2` delta), accumulated into
/// the ship-local `RcsIntent` XZ plane while [`RcsActive`] is held. Bound to the
/// same `mouse_motion` source as the camera rig (`consume_input: false`); the
/// camera's own consumer is frozen during RCS so the view holds.
#[derive(InputAction)]
#[action_output(Vec2)]
pub(super) struct RcsAimInput;

pub(super) fn on_player_added_spawn_flight_input(
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
/// expressed as input conditions: a binding-level Chord ignores the binding's
/// own value and fired on the bare modifier, and pairing it with an explicit
/// Down still yields Ongoing on the unmodified gesture, which triggers Start.
/// Instead the modifier is a plain action whose state the cycle observers
/// READ (input/targeting/component_lock.rs dispatch): plain wheel/brackets step components,
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
                    // The radar hold: Start = search opens (slot latched),
                    // Fire = active, Complete = commit-on-release, Cancel =
                    // sub-threshold release (no commit; the Tap below is that
                    // gesture). Pad: DPadUp, freed by the target cycle's
                    // retirement - a provisional binding until the keybind
                    // rework picks the pad gesture properly.
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

pub(super) fn on_player_removed_despawn_flight_input(
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

pub(super) fn on_flight_burn_input(
    fire: On<Fire<FlightBurnInput>>,
    mut commands: Commands,
    ship: Single<(Entity, &mut FlightIntent, Has<Autopilot>), With<PlayerSpaceshipMarker>>,
    pause: Res<State<crate::PauseStates>>,
) {
    // Observers bypass system-set gating; freeze intent changes while the
    // pause overlay is up. Releases stay ungated so held keys clear cleanly
    // during a pause.
    if pause.get().is_frozen() {
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

pub(super) fn on_flight_burn_input_completed(
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

pub(super) fn on_autopilot_stop_input(
    _: On<Start<AutopilotStopInput>>,
    mut commands: Commands,
    ship: Single<(Entity, Option<&Autopilot>), With<PlayerSpaceshipMarker>>,
    q_verbs: ControllerVerbQuery,
    pause: Res<State<crate::PauseStates>>,
) {
    // Observers bypass system-set gating; freeze intent changes while the
    // pause overlay is up. Releases stay ungated so held keys clear cleanly
    // during a pause.
    if pause.get().is_frozen() {
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
        //...but braking overrides any other maneuver (or engages fresh) -
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

pub(super) fn on_autopilot_goto_input(
    _: On<Start<AutopilotGotoInput>>,
    mut commands: Commands,
    ship: Single<(Entity, Option<&Autopilot>, Option<&TravelLock>), With<PlayerSpaceshipMarker>>,
    q_verbs: ControllerVerbQuery,
    pause: Res<State<crate::PauseStates>>,
) {
    // Observers bypass system-set gating; freeze intent changes while the
    // pause overlay is up. Releases stay ungated so held keys clear cleanly
    // during a pause.
    if pause.get().is_frozen() {
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
    // target is CAPTURED here, at [G]: re-designating the
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

pub(super) fn on_autopilot_orbit_input(
    _: On<Start<AutopilotOrbitInput>>,
    mut commands: Commands,
    ship: Single<(Entity, Option<&Autopilot>, Option<&DominantWell>), With<PlayerSpaceshipMarker>>,
    q_verbs: ControllerVerbQuery,
    pause: Res<State<crate::PauseStates>>,
) {
    // Observers bypass system-set gating; freeze intent changes while the
    // pause overlay is up. Releases stay ungated so held keys clear cleanly
    // during a pause.
    if pause.get().is_frozen() {
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

pub(super) fn on_autopilot_off_input(
    _: On<Start<AutopilotOffInput>>,
    mut commands: Commands,
    ship: Single<(Entity, Has<Autopilot>), With<PlayerSpaceshipMarker>>,
    pause: Res<State<crate::PauseStates>>,
) {
    // Observers bypass system-set gating; freeze intent changes while the
    // pause overlay is up. Releases stay ungated so held keys clear cleanly
    // during a pause.
    if pause.get().is_frozen() {
        return;
    }

    let (entity, engaged) = ship.into_inner();
    if engaged {
        debug!("on_autopilot_off_input: disengaging");
        commands.entity(entity).remove::<Autopilot>();
    }
}

/// Mouse-motion -> `RcsIntent` gain: how far one frame's mouse delta drives
/// the (delta-driven) intent before the per-tick decay bleeds it off. Small,
/// so a deliberate sweep crosses the range and a twitch barely moves it.
/// Feel-tunable (nudged up 0.02 -> 0.03 in).
const RCS_AIM_SENSITIVITY: f32 = 0.03;

/// Enter RCS fine-adjust mode: while SHIFT is held on a ship whose controller
/// grants the RCS verb, mark it [`RcsActive`] (the modal gate the helm, camera
/// and scroll all read) and disengage any autopilot - entering RCS is a flight
/// input, exactly like grabbing the throttle (`on_flight_burn_input`).
pub(super) fn on_rcs_modifier_start(
    _: On<Start<RcsModifierInput>>,
    mut commands: Commands,
    ship: Single<Entity, With<PlayerSpaceshipMarker>>,
    q_verbs: ControllerVerbQuery,
    pause: Res<State<crate::PauseStates>>,
) {
    if pause.get().is_frozen() {
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
pub(super) fn on_rcs_modifier_released(
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
pub(super) fn on_rcs_aim(
    fire: On<Fire<RcsAimInput>>,
    ship: Single<(&mut RcsIntent, Has<RcsActive>), With<PlayerSpaceshipMarker>>,
    pause: Res<State<crate::PauseStates>>,
) {
    if pause.get().is_frozen() {
        return;
    }
    let (mut intent, active) = ship.into_inner();
    if !active {
        return;
    }
    // DELTA-driven: SET the intent from THIS frame's mouse motion rather than
    // accumulating a persistent offset - the held-direction joystick was too
    // hard to control because it kept pushing after the mouse stopped.
    // `decay_player_rcs_intent` fades this to zero when the mouse stops, so
    // force follows motion.
    let delta = (fire.value * RCS_AIM_SENSITIVITY).clamp(Vec2::splat(-1.0), Vec2::splat(1.0));
    intent.x = delta.x;
    // Bevy mouse-motion Y is +down; pushing the mouse forward (up, -y) drives
    // the ship forward (ship-local -Z), pulling back drives it aft.
    intent.z = delta.y;
}

#[cfg(test)]
mod tests {
    use bevy::ecs::system::RunSystemOnce;

    use super::*;
    use crate::input::player::{
        hints::update_flight_verb_hints,
        test_support::{hint_world, spawn_flyable_ship},
    };

    /// End-to-end through the REAL flight rig and EnhancedInputPlugin: a GOTO
    /// keypress engages the autopilot only when a live controller grants GOTO.
    /// With the verb withheld the press is a no-op even with a valid lock; the
    /// gate deleted, the first press would engage and this test would fail.
    #[test]
    fn goto_keypress_is_gated_by_the_controller_verb_flag() {
        use bevy::input::InputPlugin;

        let mut app = App::new();
        app.add_plugins((MinimalPlugins, InputPlugin, EnhancedInputPlugin));
        // The autopilot observers are pause-gated.
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

    /// The Tab NOVA OS freezes flight input exactly like the pause menu: the
    /// burn observer self-guards on `PauseStates::is_frozen`, so a throttle
    /// press in `NovaOs` must NOT move `FlightIntent`. This pins the guard-
    /// widen from `== Paused` to `!= Unpaused`; narrowing it back to `==
    /// Paused` fails this test.
    #[test]
    fn flight_input_inert_while_nova_os_open() {
        use bevy::input::InputPlugin;

        let mut app = App::new();
        app.add_plugins((MinimalPlugins, InputPlugin, EnhancedInputPlugin));
        app.add_plugins(bevy::state::app::StatesPlugin);
        app.init_state::<crate::PauseStates>();
        app.add_input_context::<FlightInputMarker>();
        app.add_observer(on_flight_burn_input);

        let (ship, _controller) = spawn_flyable_ship(app.world_mut());
        // The burn observer writes an existing FlightIntent; give the ship one.
        app.world_mut()
            .entity_mut(ship)
            .insert(FlightIntent::default());
        app.finish();
        app.cleanup();
        app.update();
        app.world_mut().spawn(flight_input_rig());
        app.update();

        // Open the NOVA OS, then press the throttle: intent stays put.
        app.world_mut()
            .resource_mut::<NextState<crate::PauseStates>>()
            .set(crate::PauseStates::NovaOs);
        app.update();
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::KeyW);
        app.update();
        app.update();
        assert_eq!(
            app.world().get::<FlightIntent>(ship).map(|i| i.burn),
            Some(0.0),
            "a throttle press while the NOVA OS is open must not move FlightIntent"
        );

        // Close the NOVA OS (back to Unpaused) and press again: now it burns,
        // proving the press itself is live and the freeze is what suppressed it.
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .release(KeyCode::KeyW);
        app.update();
        app.world_mut()
            .resource_mut::<NextState<crate::PauseStates>>()
            .set(crate::PauseStates::Unpaused);
        app.update();
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::KeyW);
        app.update();
        app.update();
        assert!(
            app.world()
                .get::<FlightIntent>(ship)
                .is_some_and(|i| i.burn > 0.0),
            "the same press burns once the NOVA OS is closed"
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
    /// plane (strafe + forward/back) from THIS frame's delta - SET, not a
    /// running accumulate (delta-driven, not a joystick). Outside RCS the
    /// same motion is ignored.
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
    /// Guards the fail-closed hazard.
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
}
