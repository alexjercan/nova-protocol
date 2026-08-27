//! Human piloting: turns keyboard/mouse/gamepad input into ship intent. The
//! always-on flight rig drives the flight verbs (burn, the STOP/GOTO/ORBIT
//! autopilot commands, RCS fine-adjust), and per-weapon `input_mapping` bindings
//! (thruster/turret/torpedo) fire the sections. Keys off the human's ship marker
//! ([`PlayerSpaceshipMarker`](nova_gameplay::markers::PlayerSpaceshipMarker)) and
//! maintains [`FlightVerbHints`] for the verb-hint HUD.
//!
//! The reserved flight-rig sources ([`flight_rig_reserved_sources`]) must not be
//! reused by content weapon bindings or flight silently double-drives; see that
//! function's note. Autopilot verbs land as [`FlightIntent`](crate::flight) /
//! [`Autopilot`](crate::flight) on the ship, consumed by
//! [`flight`](crate::flight).

use bevy::prelude::*;
use bevy_enhanced_input::prelude::*;
use nova_input::prelude::*;

pub mod flight_rig;
mod hints;
pub mod intent;
#[cfg(test)]
pub(crate) mod test_support;
mod weapons;

use flight_rig::{
    on_autopilot_goto_input, on_autopilot_off_input, on_autopilot_orbit_input,
    on_autopilot_stop_input, on_flight_burn_input, on_flight_burn_input_completed,
    on_player_added_spawn_flight_input, on_player_removed_despawn_flight_input, on_rcs_aim,
    on_rcs_modifier_released, on_rcs_modifier_start, rebuild_flight_input_on_rebind,
};
use hints::update_flight_verb_hints;
use intent::{
    update_controller_target_rotation_torque, update_torpedo_target_input,
    update_turret_target_input,
};
use weapons::{
    on_thruster_input, on_thruster_input_binding, on_thruster_input_completed, on_torpedo_input,
    on_torpedo_input_binding, on_torpedo_input_completed, on_turret_input, on_turret_input_binding,
    on_turret_input_completed, ThrusterInputMarker, TorpedoInputMarker, TurretInputMarker,
};

pub(crate) use self::flight_rig::FlightInputMarker;
pub use self::{
    hints::{FlightVerbHints, VerbHint},
    weapons::{
        SectionInputBindingChanged, SpaceshipThrusterInputBinding, SpaceshipTorpedoInputBinding,
        SpaceshipTurretInputBinding,
    },
};
use crate::input::bindings::flight_bindings;

/// The input sources and per-verb bindings, the verb hints and
/// `SpaceshipPlayerInputPlugin`.
pub mod prelude {
    pub use super::{
        FlightVerbHints, SectionInputBindingChanged, SpaceshipPlayerInputPlugin,
        SpaceshipThrusterInputBinding, SpaceshipTorpedoInputBinding, SpaceshipTurretInputBinding,
        VerbHint,
    };
}

/// Raise the `Flight` context while a flight action can actually be heard.
///
/// Two halves, because flight goes quiet two ways. The rig entity is spawned
/// and despawned with `PlayerSpaceshipMarker`, so with no player ship there is
/// nothing listening at all; and every frozen variant gates the input sets, so
/// while the NOVA OS or the pause overlay holds the screen a flight key is
/// read by whatever is up instead - `W` pans the viewer rather than burning.
///
/// `PauseStates` belongs to `AppBuilder`, and a test rig that adds this plugin
/// alone has no pause state to read. Absent means nothing can freeze the ship.
fn sync_flight_context(
    player: Query<(), With<nova_gameplay::markers::PlayerSpaceshipMarker>>,
    pause: Option<Res<State<nova_gameplay::PauseStates>>>,
    mut active: ResMut<ActiveContexts>,
) {
    let frozen = pause.is_some_and(|state| state.get().is_frozen());
    ActiveContexts::sync(
        &mut active,
        ActionContext::Flight,
        !player.is_empty() && !frozen,
    );
}

/// Wires human input for the player ship: the flight rig, weapon fire bindings,
/// autopilot verbs and RCS. Added by
/// [`SpaceshipInputPlugin`](super::SpaceshipInputPlugin).
pub struct SpaceshipPlayerInputPlugin;

impl Plugin for SpaceshipPlayerInputPlugin {
    fn build(&self, app: &mut App) {
        trace!("SpaceshipPlayerInputPlugin: build");

        app.register_input_actions(flight_bindings());
        app.add_systems(PreUpdate, sync_flight_context);
        app.add_message::<SectionInputBindingChanged>();
        app.add_input_context::<FlightInputMarker>();
        app.add_observer(on_player_added_spawn_flight_input);
        app.add_systems(
            Update,
            rebuild_flight_input_on_rebind.run_if(resource_changed::<InputBindings>),
        );
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
                // commit (previously a `.chain()` when they shared a module).
                update_turret_target_input.after(super::targeting::SpaceshipTargetingSystems),
                update_torpedo_target_input.after(super::targeting::SpaceshipTargetingSystems),
                update_flight_verb_hints.after(super::targeting::SpaceshipTargetingSystems),
            )
                .in_set(super::SpaceshipInputSystems),
        );
    }
}

#[cfg(test)]
mod context_tests {
    use bevy::state::app::StatesPlugin;
    use nova_gameplay::{markers::PlayerSpaceshipMarker, PauseStates};

    use super::*;

    fn rig() -> App {
        let mut app = App::new();
        app.add_plugins(StatesPlugin);
        app.init_state::<PauseStates>();
        app.init_resource::<ActiveContexts>();
        app.add_systems(Update, sync_flight_context);
        app
    }

    fn flight_is_live(app: &App) -> bool {
        app.world()
            .resource::<ActiveContexts>()
            .is_live(ActionContext::Flight)
    }

    fn freeze(app: &mut App, state: PauseStates) {
        app.world_mut()
            .resource_mut::<NextState<PauseStates>>()
            .set(state);
        app.update();
    }

    /// Flight goes quiet two ways, and both have to lower the context: with no
    /// player ship there is no rig to hear a key, and behind a frozen overlay
    /// the input sets do not run, so the key reaches whatever is up instead.
    #[test]
    fn flight_is_live_only_with_a_player_ship_and_the_clocks_running() {
        let mut app = rig();
        app.update();
        assert!(!flight_is_live(&app), "no player ship, nothing listening");

        let player = app.world_mut().spawn(PlayerSpaceshipMarker).id();
        app.update();
        assert!(flight_is_live(&app));

        freeze(&mut app, PauseStates::NovaOs);
        assert!(
            !flight_is_live(&app),
            "the monitor has the screen, so `W` pans the viewer rather than burning"
        );

        freeze(&mut app, PauseStates::Unpaused);
        assert!(flight_is_live(&app));

        freeze(&mut app, PauseStates::Paused);
        assert!(!flight_is_live(&app));

        freeze(&mut app, PauseStates::Unpaused);
        app.world_mut().entity_mut(player).despawn();
        app.update();
        assert!(!flight_is_live(&app), "the rig despawns with the marker");
    }
}
