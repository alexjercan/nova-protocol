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

pub mod flight_rig;
mod hints;
pub mod intent;
#[cfg(test)]
mod test_support;
mod weapons;

use flight_rig::{
    on_autopilot_goto_input, on_autopilot_off_input, on_autopilot_orbit_input,
    on_autopilot_stop_input, on_flight_burn_input, on_flight_burn_input_completed,
    on_player_added_spawn_flight_input, on_player_removed_despawn_flight_input, on_rcs_aim,
    on_rcs_modifier_released, on_rcs_modifier_start,
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

#[cfg(test)]
pub(crate) use self::flight_rig::flight_input_rig;
pub(crate) use self::flight_rig::FlightInputMarker;
pub use self::{
    hints::{
        binding_label, binding_source, flight_rig_reserved_sources, keyboard_label,
        FlightVerbHints, InputSource, VerbHint,
    },
    weapons::{
        SpaceshipThrusterInputBinding, SpaceshipTorpedoInputBinding, SpaceshipTurretInputBinding,
    },
};

/// The input sources and per-verb bindings, the verb hints and
/// `SpaceshipPlayerInputPlugin`.
pub mod prelude {
    pub use super::{
        binding_label, binding_source, flight_rig_reserved_sources, FlightVerbHints, InputSource,
        SpaceshipPlayerInputPlugin, SpaceshipThrusterInputBinding, SpaceshipTorpedoInputBinding,
        SpaceshipTurretInputBinding, VerbHint,
    };
}

/// Wires human input for the player ship: the flight rig, weapon fire bindings,
/// autopilot verbs and RCS. Added by
/// [`SpaceshipInputPlugin`](super::SpaceshipInputPlugin).
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
                // commit (previously a `.chain()` when they shared a module).
                update_turret_target_input.after(super::targeting::SpaceshipTargetingSystems),
                update_torpedo_target_input.after(super::targeting::SpaceshipTargetingSystems),
                update_flight_verb_hints.after(super::targeting::SpaceshipTargetingSystems),
            )
                .in_set(super::SpaceshipInputSystems),
        );
    }
}
