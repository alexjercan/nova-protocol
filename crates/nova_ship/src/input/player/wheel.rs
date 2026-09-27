//! The mouse wheel in flight: one gesture, one owner. RCS vertical thrust
//! while RCS is held, the component fine-lock while the weapons are raised,
//! and the chase-camera zoom otherwise.
//!
//! The wheel actions run with `consume_input: false` beside every other flight
//! action, so input consumption cannot pick the owner. Both observers read
//! [`wheel_role`] instead.
//!
//! Both observers run in `PreUpdate` and must route by THIS frame's stance, so
//! that an RMB or SHIFT edge in the same frame as a wheel line counts. They read
//! the combat action's `TriggerState`, not [`WeaponsRaised`]: enhanced input
//! writes every action's state before it triggers any event, but the flag is
//! derived later, in `Update`. [`RcsActive`] is current because the flight rig
//! spawns the RCS modifier before the wheel actions, so its observers insert or
//! remove the marker before the wheel events trigger.

use bevy::prelude::*;
use bevy_enhanced_input::prelude::*;
use nova_gameplay::prelude::*;

use super::control::player_control_is_suspended;
use crate::{
    camera::{action_held, CombatInput},
    input::targeting::{step_component_lock, RCS_SCROLL_STEP},
    prelude::*,
};

/// What the wheel drives.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum WheelRoleType {
    /// RCS vertical thrust, up = +Y.
    Rcs,
    /// The component fine-lock, up = next section.
    ComponentLock,
    /// The chase-camera zoom, up = closer.
    CameraZoom,
}

/// The one wheel precedence rule. RCS wins while it is held, in every camera
/// mode. Raised weapons keep the wheel on the component lock. Normal and
/// FreeLook flight give it to the zoom.
pub(crate) fn wheel_role(rcs_active: bool, weapons_raised: bool) -> WheelRoleType {
    if rcs_active {
        WheelRoleType::Rcs
    } else if weapons_raised {
        WheelRoleType::ComponentLock
    } else {
        WheelRoleType::CameraZoom
    }
}

/// Wheel up, in lines: positive. bevy_enhanced_input reads trackpad pixels as
/// lines of 100 pixels.
#[derive(InputAction)]
#[action_output(f32)]
pub(crate) struct WheelUpInput;

/// Wheel down, in lines: negative.
#[derive(InputAction)]
#[action_output(f32)]
pub(crate) struct WheelDownInput;

/// The wheel's discrete half: one RCS nudge or one component step per scroll
/// stream. `Start` fires once per unbroken stream, so a stream that began as a
/// zoom does not cycle a component when the weapons rise part way through it.
pub(super) fn on_wheel_step<A: InputAction<Output = f32>>(
    start: On<Start<A>>,
    time: Res<Time>,
    q_sections: Query<(Entity, &ChildOf, &Transform), With<SectionMarker>>,
    mut q_ship: Query<
        (
            &CombatLock,
            &LockFocus,
            &mut ComponentLock,
            Has<RcsActive>,
            Option<&mut RcsIntent>,
        ),
        (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>),
    >,
    q_combat: Query<&TriggerState, With<Action<CombatInput>>>,
    pause: Res<State<nova_gameplay::PauseStates>>,
    control: Option<Res<PlayerControlSuspended>>,
) {
    if pause.get().is_frozen() || player_control_is_suspended(control) {
        return;
    }
    let direction: isize = if start.value > 0.0 { 1 } else { -1 };
    let raised = action_held(&q_combat);
    for (lock, focus, mut component, rcs_active, rcs_intent) in &mut q_ship {
        match wheel_role(rcs_active, raised) {
            WheelRoleType::Rcs => {
                if let Some(mut intent) = rcs_intent {
                    intent.y = crate::flight::accumulate_rcs_axis(
                        intent.y,
                        RCS_SCROLL_STEP * direction as f32,
                    );
                }
            }
            WheelRoleType::ComponentLock => {
                step_component_lock(direction, &time, lock, focus, &mut component, &q_sections);
            }
            WheelRoleType::CameraZoom => {}
        }
    }
}

/// The wheel's continuous half: every frame's wheel lines go to the zoom while
/// the zoom owns the wheel. `Fire` carries each frame's full value, so a fast
/// scroll counts every line instead of one per stream.
pub(super) fn on_wheel_zoom<A: InputAction<Output = f32>>(
    fire: On<Fire<A>>,
    q_ship: Query<Has<RcsActive>, (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>)>,
    q_combat: Query<&TriggerState, With<Action<CombatInput>>>,
    mut zoom: ResMut<ChaseZoom>,
    pause: Res<State<nova_gameplay::PauseStates>>,
    control: Option<Res<PlayerControlSuspended>>,
) {
    if pause.get().is_frozen() || player_control_is_suspended(control) {
        return;
    }
    let Ok(rcs_active) = q_ship.single() else {
        return;
    };
    if wheel_role(rcs_active, action_held(&q_combat)) == WheelRoleType::CameraZoom {
        zoom.pending_lines += fire.value;
    }
}

#[cfg(test)]
mod tests {
    use bevy::input::{
        mouse::{MouseScrollUnit, MouseWheel},
        touch::TouchPhase,
        InputPlugin,
    };
    use nova_gameplay::PauseStates;

    use super::*;
    use crate::input::{
        player::{
            flight_rig::{on_rcs_modifier_released, on_rcs_modifier_start},
            test_support::{spawn_flight_rig, spawn_flyable_ship},
            FlightInputMarker,
        },
        targeting::{on_component_cycle_next, on_component_cycle_prev},
    };

    #[test]
    fn rcs_wins_the_wheel_then_raised_weapons_then_the_zoom() {
        assert_eq!(wheel_role(false, false), WheelRoleType::CameraZoom);
        assert_eq!(wheel_role(false, true), WheelRoleType::ComponentLock);
        assert_eq!(wheel_role(true, false), WheelRoleType::Rcs);
        assert_eq!(wheel_role(true, true), WheelRoleType::Rcs);
    }

    /// The camera rig's stand-in: in the game the combat stance is an action on
    /// its own context entity, beside the flight rig.
    #[derive(Component)]
    struct CombatContextMarker;

    /// The real flight rig with the wheel, RCS modifier and cycle-key
    /// observers, RMB bound to the combat stance on a second context, and a
    /// player locked and focused on a two-section target. Returns the app and
    /// the player ship.
    fn wheel_app() -> (App, Entity) {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, InputPlugin, EnhancedInputPlugin));
        app.add_plugins(bevy::state::app::StatesPlugin);
        app.init_state::<PauseStates>();
        app.init_resource::<ChaseZoom>();
        app.add_input_context::<FlightInputMarker>();
        app.add_input_context::<CombatContextMarker>();
        app.add_observer(on_wheel_step::<WheelUpInput>);
        app.add_observer(on_wheel_step::<WheelDownInput>);
        app.add_observer(on_wheel_zoom::<WheelUpInput>);
        app.add_observer(on_wheel_zoom::<WheelDownInput>);
        app.add_observer(on_component_cycle_next);
        app.add_observer(on_component_cycle_prev);
        app.add_observer(on_rcs_modifier_start);
        app.add_observer(on_rcs_modifier_released);

        let (ship, _controller) = spawn_flyable_ship(app.world_mut());
        let target = app.world_mut().spawn(SpaceshipRootMarker).id();
        for z in [0.0, 1.0] {
            app.world_mut().spawn((
                SectionMarker,
                ChildOf(target),
                Transform::from_xyz(0.0, 0.0, z),
            ));
        }
        app.world_mut().entity_mut(ship).insert((
            RcsIntent::default(),
            CombatLock(Some(target)),
            LockFocus {
                target: Some(target),
                seconds: f32::MAX,
            },
        ));

        app.finish();
        app.cleanup();
        app.update();
        spawn_flight_rig(&mut app);
        app.world_mut().spawn((
            CombatContextMarker,
            actions!(
                CombatContextMarker[(Action::<CombatInput>::new(), bindings![MouseButton::Right])]
            ),
        ));
        app.update();
        (app, ship)
    }

    /// One wheel event, then the frame that reads it and the frame that ends
    /// the stream, so the next event starts a new one.
    fn scroll(app: &mut App, unit: MouseScrollUnit, y: f32) {
        app.world_mut().write_message(MouseWheel {
            unit,
            x: 0.0,
            y,
            window: Entity::PLACEHOLDER,
            phase: TouchPhase::Moved,
        });
        app.update();
        app.update();
    }

    fn pending_lines(app: &App) -> f32 {
        app.world().resource::<ChaseZoom>().pending_lines
    }

    fn set_rmb(app: &mut App, held: bool) {
        let mut buttons = app.world_mut().resource_mut::<ButtonInput<MouseButton>>();
        if held {
            buttons.press(MouseButton::Right);
        } else {
            buttons.release(MouseButton::Right);
        }
    }

    fn set_shift(app: &mut App, held: bool) {
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        if held {
            keys.press(KeyCode::ShiftLeft);
        } else {
            keys.release(KeyCode::ShiftLeft);
        }
    }

    /// Zero the zoom, the lock and RCS vertical, so the next scroll shows only
    /// its own owner.
    fn clear_wheel_owners(app: &mut App, ship: Entity) {
        app.world_mut().resource_mut::<ChaseZoom>().pending_lines = 0.0;
        app.world_mut()
            .entity_mut(ship)
            .insert((RcsIntent::default(), ComponentLock::default()));
    }

    /// Exactly `owner` moved, by a scroll of `sign` lines.
    fn assert_sole_owner(app: &App, ship: Entity, owner: WheelRoleType, sign: f32, case: &str) {
        let zoom = pending_lines(app);
        let vertical = app.world().get::<RcsIntent>(ship).unwrap().0.y;
        let lock = app.world().get::<ComponentLock>(ship).unwrap().section;
        match owner {
            WheelRoleType::CameraZoom => {
                assert!((zoom - sign).abs() < 1e-5, "{case}: zoom {zoom}");
            }
            WheelRoleType::ComponentLock => assert!(lock.is_some(), "{case}"),
            WheelRoleType::Rcs => {
                assert!(vertical * sign > 0.0, "{case}: vertical {vertical}");
            }
        }
        if owner != WheelRoleType::CameraZoom {
            assert_eq!(zoom, 0.0, "{case}: the zoom moved too");
        }
        if owner != WheelRoleType::ComponentLock {
            assert_eq!(lock, None, "{case}: the lock stepped too");
        }
        if owner != WheelRoleType::Rcs {
            assert_eq!(vertical, 0.0, "{case}: RCS moved too");
        }
    }

    /// One wheel gesture has exactly one owner in every stance, for mouse
    /// lines and trackpad pixels in both directions: the zoom in Normal and
    /// FreeLook flight, the component lock with the weapons raised, and RCS
    /// vertical while RCS is held, raised or not.
    #[test]
    fn one_wheel_gesture_has_exactly_one_owner() {
        let (mut app, ship) = wheel_app();
        let stances = [
            (false, false, WheelRoleType::CameraZoom),
            (false, true, WheelRoleType::ComponentLock),
            (true, false, WheelRoleType::Rcs),
            (true, true, WheelRoleType::Rcs),
        ];
        for (rcs, raised, owner) in stances {
            for (unit, one_line) in [
                (MouseScrollUnit::Line, 1.0),
                (MouseScrollUnit::Pixel, 100.0),
            ] {
                for sign in [1.0f32, -1.0] {
                    clear_wheel_owners(&mut app, ship);
                    set_rmb(&mut app, raised);
                    if rcs {
                        app.world_mut().entity_mut(ship).insert(RcsActive);
                    } else {
                        app.world_mut().entity_mut(ship).remove::<RcsActive>();
                    }

                    scroll(&mut app, unit, sign * one_line);

                    let case = format!("rcs {rcs}, raised {raised}, {unit:?} {sign}");
                    assert_sole_owner(&app, ship, owner, sign, &case);
                }
            }
        }
    }

    /// An RMB or SHIFT edge in the same frame as a wheel line routes that line
    /// by the new stance. The stance is read at the input edge, not from the
    /// flag `Update` derives a step later, so RMB down cycles the lock, RMB up
    /// zooms, and SHIFT down or up hands the wheel to or from RCS.
    #[test]
    fn a_stance_edge_in_the_wheel_frame_routes_that_line_by_the_new_stance() {
        let (mut app, ship) = wheel_app();
        let edges: [(&str, fn(&mut App), WheelRoleType); 5] = [
            (
                "RMB down",
                |app| set_rmb(app, true),
                WheelRoleType::ComponentLock,
            ),
            (
                "RMB up",
                |app| set_rmb(app, false),
                WheelRoleType::CameraZoom,
            ),
            ("SHIFT down", |app| set_shift(app, true), WheelRoleType::Rcs),
            (
                "RMB down under SHIFT",
                |app| set_rmb(app, true),
                WheelRoleType::Rcs,
            ),
            (
                "SHIFT up under RMB",
                |app| set_shift(app, false),
                WheelRoleType::ComponentLock,
            ),
        ];
        for (case, edge, owner) in edges {
            clear_wheel_owners(&mut app, ship);
            edge(&mut app);
            app.world_mut().write_message(MouseWheel {
                unit: MouseScrollUnit::Line,
                x: 0.0,
                y: 1.0,
                window: Entity::PLACEHOLDER,
                phase: TouchPhase::Moved,
            });
            app.update();
            assert_sole_owner(&app, ship, owner, 1.0, case);
            // End the scroll stream so the next edge starts a new one.
            app.update();
        }
    }

    /// With the weapons lowered the wheel zooms, and `]` and D-pad right still
    /// step the component lock without zooming.
    #[test]
    fn the_cycle_keys_still_step_the_lock_while_the_wheel_zooms() {
        let (mut app, ship) = wheel_app();

        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::BracketRight);
        app.update();
        app.update();
        assert!(
            app.world()
                .get::<ComponentLock>(ship)
                .unwrap()
                .section
                .is_some(),
            "] steps the lock"
        );
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .release(KeyCode::BracketRight);
        app.update();

        app.world_mut()
            .entity_mut(ship)
            .insert(ComponentLock::default());
        let pad = app.world_mut().spawn(Gamepad::default()).id();
        app.update();
        app.world_mut()
            .get_mut::<Gamepad>(pad)
            .unwrap()
            .analog_mut()
            .set(GamepadButton::DPadRight, 1.0);
        app.update();
        app.update();
        assert!(
            app.world()
                .get::<ComponentLock>(ship)
                .unwrap()
                .section
                .is_some(),
            "D-pad right steps the lock"
        );
        assert_eq!(pending_lines(&app), 0.0, "the cycle keys never zoom");

        scroll(&mut app, MouseScrollUnit::Line, 1.0);
        assert_eq!(pending_lines(&app), 1.0, "and the wheel still zooms");
    }

    /// Flight behind the pause overlay, behind the NOVA OS, or under a
    /// cinematic hears no wheel, so nothing zooms.
    #[test]
    fn a_frozen_or_suspended_flight_takes_no_zoom() {
        let (mut app, _ship) = wheel_app();

        for state in [PauseStates::Paused, PauseStates::NovaOs] {
            app.world_mut()
                .resource_mut::<NextState<PauseStates>>()
                .set(state);
            app.update();
            scroll(&mut app, MouseScrollUnit::Line, 1.0);
            assert_eq!(pending_lines(&app), 0.0, "{state:?} zoomed");
        }

        app.world_mut()
            .resource_mut::<NextState<PauseStates>>()
            .set(PauseStates::Unpaused);
        app.world_mut()
            .insert_resource(PlayerControlSuspended(true));
        app.update();
        scroll(&mut app, MouseScrollUnit::Line, 1.0);
        assert_eq!(pending_lines(&app), 0.0, "a suspended flight zoomed");

        // Delivery guard: the same scroll zooms once control is back.
        app.world_mut()
            .insert_resource(PlayerControlSuspended(false));
        scroll(&mut app, MouseScrollUnit::Line, 1.0);
        assert_eq!(pending_lines(&app), 1.0);
    }
}
