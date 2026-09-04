//! Nova's flight layer: manual Newtonian piloting plus a diegetic autopilot
//! that flies the ship through its real actuators.
//!
//! There are no invisible forces anywhere in this module: the autopilot swings
//! the nose by writing the same [`ControllerSectionRotationInput`] the player's
//! mouse command uses (the controller section's PD torque does the turning),
//! and it burns by writing the live thrusters' [`ThrusterSectionInput`] - so
//! the plume, the engine hum, and the actual impulse are the maneuver.
//!
//! - **Manual** (default): the mouse points the hull, W/Space/right-trigger is
//!   an analog main-drive burn, momentum persists. Pure Newtonian.
//! - **Autopilot** (engaged per action, [`Autopilot`] component present):
//!   - `X` - **STOP**: face retrograde, burn until the ship is at rest.
//!   - `G` - **GOTO** the current aim-assist lock: burn toward the target,
//!     flip at the arrival curve (`v_allowed = sqrt(2 * a * margin * d)`),
//!     decelerate, and come to rest at a standoff outside blast radius.
//!
//!   Both are one rule: compute the desired velocity for the goal, face the
//!   velocity *error*, and burn when aligned - the flip emerges naturally the
//!   moment the error points backward. While engaged, the ship stops
//!   listening to the mouse (the manual rotation copy is gated off), which
//!   makes the mouse camera-only free-look for free; any flight input
//!   disengages, and disengaging re-seeds the mouse rig from the ship's
//!   current facing so nothing lurches (see `camera/`).
//!
//! Capability comes from the live sections: the main drive is the summed
//! magnitude of forward-aligned live thrusters, and the flight computer *is*
//! the controller section - no live controller, no autopilot (it disengages),
//! exactly as rotation authority already dies with it. Thruster inputs are
//! spooled (exponential ramp) so engines light up and cut instead of
//! snapping. Tunables live on the reflected [`FlightSettings`]; the math is
//! pure helpers, unit-tested, shared-shaped so the AI brain (input/ai/,
//! today a cruder version of the same idea) can adopt it later.

use bevy::prelude::*;
use nova_gameplay::prelude::*;

use crate::prelude::*;

mod autopilot;
mod guidance;
mod manual;
mod order;
mod state;
mod thrusters;

#[cfg(test)]
mod tests;

// Only the input layer's turn-rate tests derive the rate independently.
#[cfg(test)]
pub(crate) use self::guidance::hull_turn_rate;
use self::{
    autopilot::{autopilot_system, on_autopilot_removed_cool_engines},
    manual::{decay_player_rcs_intent, manual_burn_system, rcs_burn_system},
    order::{drive_scripted_align, drive_ship_orders},
    state::remove_maneuver_telemetry,
};
pub(crate) use self::{
    guidance::{ship_turn_rate, slew_rotation},
    manual::accumulate_rcs_axis,
};
pub use self::{
    order::{
        cancel_ship_order, interrupt_ship_order, resume_ship_order, retire_ship_order_execution,
        AIOrderInterrupted, ScriptedAlign, ScriptedAlignSettled, ShipHelmOrder, ShipOrderDirective,
        ShipOrderEngaged, ShipOrderHelmAuthority, ShipOrderOutcome, ShipOrderReport,
        ShipOrderReported, ShipOrderReports, SuspendedArrivalStandoff,
    },
    state::{
        resolved_arrival_standoff, Autopilot, AutopilotAction, AutopilotPhase, BodyRadius,
        FlightArrivalStandoff, FlightIntent, FlightSettings, FlightSpeedCap, ManeuverTelemetry,
        OrbitPlan, RcsActive, RcsIntent, RcsReference, RcsSpeedCap,
    },
};

/// The flight intent, settings and speed caps, the autopilot and orbit plan, RCS state, maneuver
/// telemetry, and `NovaFlightPlugin` with `NovaFlightSystems`.
pub mod prelude {
    pub use super::{
        cancel_ship_order, interrupt_ship_order, resolved_arrival_standoff, resume_ship_order,
        retire_ship_order_execution, AIOrderInterrupted, Autopilot, AutopilotAction,
        AutopilotPhase, BodyRadius, FlightArrivalStandoff, FlightIntent, FlightSettings,
        FlightSpeedCap, ManeuverTelemetry, NovaFlightPlugin, NovaFlightSystems, OrbitPlan,
        RcsActive, RcsIntent, RcsSpeedCap, ScriptedAlign, ScriptedAlignSettled, ShipHelmOrder,
        ShipOrderDirective, ShipOrderEngaged, ShipOrderHelmAuthority, ShipOrderOutcome,
        ShipOrderReport, ShipOrderReported, ShipOrderReports, SuspendedArrivalStandoff,
    };
}

/// System set for the flight layer; ordered before the section systems in
/// `FixedUpdate` so the thruster impulse system consumes the inputs written
/// this tick.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct NovaFlightSystems;

/// Plugin wiring the flight layer.
#[derive(Default)]
pub struct NovaFlightPlugin;

impl Plugin for NovaFlightPlugin {
    fn build(&self, app: &mut App) {
        trace!("NovaFlightPlugin: build");

        // The ORBIT verb reads the gravity tunables; init here too so the
        // flight layer stands alone (the AI physics tests build it without
        // NovaGravityPlugin). init_resource is idempotent, so the gravity
        // plugin owning the same resource is fine.
        app.init_resource::<GravitySettings>();

        app.init_resource::<FlightSettings>()
            // Register the whole reflected tree, not just the resource root.
            .register_type::<FlightSettings>()
            .register_type::<FlightIntent>()
            .register_type::<Autopilot>()
            .register_type::<AutopilotAction>()
            .register_type::<AutopilotPhase>()
            .register_type::<OrbitPlan>()
            .register_type::<ManeuverTelemetry>()
            .register_type::<BodyRadius>()
            .register_type::<FlightSpeedCap>()
            .register_type::<FlightArrivalStandoff>()
            .register_type::<RcsIntent>()
            .register_type::<RcsSpeedCap>()
            .register_type::<RcsReference>()
            .register_type::<RcsActive>()
            .register_type::<ShipHelmOrder>()
            .register_type::<ShipOrderDirective>()
            .register_type::<ShipOrderHelmAuthority>()
            .register_type::<ShipOrderReported>()
            .register_type::<ShipOrderEngaged>()
            .register_type::<ShipOrderReports>()
            .register_type::<AIOrderInterrupted>()
            .register_type::<ScriptedAlign>()
            .register_type::<ScriptedAlignSettled>()
            .register_type::<SuspendedArrivalStandoff>();

        app.add_observer(insert_flight_control);
        app.add_observer(on_autopilot_removed_cool_engines);
        app.add_observer(remove_maneuver_telemetry);

        // The autopilot is a rotation-authority writer, so it must land before
        // the controller section copies the held command into the PD input.
        // Declared here, from the writer's side: `sections` sits under `flight`
        // and cannot name `NovaFlightSystems` without a back-edge.
        app.configure_sets(
            FixedUpdate,
            NovaFlightSystems
                // ...and a rotation-authority READER of the controller stack:
                // the turn-rate budget it plans with is the sum of the shares
                // the stack pass writes, so it must not run on last tick's
                // split after a controller dies.
                .after(ControllerSectionSystems::SyncStack)
                .before(ControllerSectionSystems::SyncRotationInput)
                .before(SpaceshipSectionSystems),
        );
        app.add_systems(
            FixedUpdate,
            (
                // Before the autopilot, so a maneuver this tick's order
                // engages burns this tick rather than next: an order layer
                // that lagged the physics by a frame would put every beat
                // sequenced off it a frame behind too.
                drive_ship_orders,
                autopilot_system,
                // A rotation-authority writer like the autopilot, and after
                // it: the two never coexist (one mutually exclusive helm
                // family), and ordering them says which would win if they
                // ever did.
                drive_scripted_align,
                manual_burn_system,
                rcs_burn_system,
                decay_player_rcs_intent,
            )
                .chain()
                .in_set(NovaFlightSystems),
        );
    }
}

/// Give the player's ship its manual flight input. Only intent-carrying ships
/// are driven by this layer; AI ships keep writing the raw seams directly.
pub(super) fn insert_flight_control(add: On<Add, PlayerSpaceshipMarker>, mut commands: Commands) {
    let entity = add.entity;
    trace!("insert_flight_control: entity {:?}", entity);

    commands
        .entity(entity)
        .insert((FlightIntent::default(), RcsIntent::default()));
}
