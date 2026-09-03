//! Scripted helm authority: the components a scenario installs to fly a
//! controller-less ship on authored marks, and the alignment drive that turns
//! a hull without translating it.
//!
//! The division of labour is the same one the scripted torpedo bay uses. This
//! module owns the STATE and the physics; the scenario layer owns the
//! vocabulary that installs it and the event that reports it finished. Nothing
//! here knows what a scenario is.
//!
//! Engine units: `look_at` is compared against an avian `Position` every tick
//! and the tolerance is radians, so both cross the authoring seam in the
//! scenario action, not here.

use avian3d::prelude::*;
use bevy::prelude::*;
use nova_events::prelude::*;
use nova_gameplay::prelude::*;

use super::{ship_turn_rate, slew_rotation, FlightSettings};
use crate::prelude::*;

/// The one live scripted HELM order on a ship root.
///
/// Move, align and stop are a mutually exclusive family, and this component IS
/// that exclusion: installing an order removes whatever was here first, so a
/// ship can never be flying two authored marks at once. It outlives its own
/// completion on purpose - an alignment holds its facing afterwards, and the
/// record of what a ship was last told is what makes a replacement legible in
/// a log.
///
/// `reported` is bookkeeping for the scenario layer's completion tracker, not
/// for the flight layer: this crate never fires a scenario event, so the
/// tracker that does flips the flag once it has. A replacement installs a
/// fresh component with `reported: false`, which is what lets one ship be
/// given the same order key twice and complete it twice.
#[derive(Component, Clone, Debug, Reflect)]
#[reflect(Component)]
pub struct ScriptedHelmOrder {
    /// The authored key a completion is reported under.
    pub key: String,
    /// Which order this is.
    pub kind: ShipOrderKind,
    /// Whether the completion has already been reported.
    pub reported: bool,
}

impl ScriptedHelmOrder {
    /// A freshly installed, unreported order.
    pub fn new(key: String, kind: ShipOrderKind) -> Self {
        Self {
            key,
            kind,
            reported: false,
        }
    }
}

/// A live [`ShipOrderKind::Align`] order's bearing, on the ship root.
///
/// Rotation only: no [`Autopilot`](super::Autopilot) is engaged, so no engine
/// ever burns for translation. The hull turns under the same PD controller and
/// the same derived turn rate the player and the AI use, which is why an
/// alignment takes exactly as long as the ship's rotation authority says it
/// should.
#[derive(Component, Clone, Copy, Debug, Reflect)]
#[reflect(Component)]
pub struct ScriptedAlign {
    /// World position to put under the bore.
    pub look_at: Vec3,
    /// How close the aim must come, radians.
    pub tolerance: f32,
}

/// Present once a [`ScriptedAlign`] order has reached its tolerance and
/// settled. The alignment keeps holding afterwards, so this latches for the
/// life of the order rather than tracking the aim frame by frame: a hold that
/// is nudged a hair off by a torpedo leaving its bay has not un-completed.
#[derive(Component, Clone, Copy, Debug, Default, Reflect)]
#[reflect(Component)]
pub struct ScriptedAlignSettled;

/// The [`FlightArrivalStandoff`](super::FlightArrivalStandoff) a scripted move
/// displaced, so retiring the order can put the hull back the way it found it.
///
/// A cinematic move authors a tight standoff because 500 m is too coarse to
/// stage a shot with. That tuning belongs to the ORDER, not to the ship: left
/// installed it would silently retune every later GOTO the hull flies.
/// `None` records "there was no override" - the same shape, and the same
/// reason, as [`SuspendedSectionAmmo`](crate::prelude::SuspendedSectionAmmo).
#[derive(Component, Clone, Copy, Debug, Reflect)]
#[reflect(Component)]
pub struct SuspendedArrivalStandoff(pub Option<f32>);

/// Turn every aligning hull onto its authored bearing and hold it there.
///
/// Writes the same absolute world rotation command the AI brain writes, slewed
/// at the hull's acceleration-derived turn rate, and for the same reason: a
/// setpoint slammed to the goal drives the PD into saturation where its
/// damping is swamped and the hull limit-cycles. The command evolves from its
/// own previous value, never from the hull, so roll picked up during a swing
/// is not fed back as zero roll error.
///
/// A ship with no live flight computer cannot turn. The order stays installed
/// and simply never completes, which is the honest outcome: the scenario's
/// deadline is what reports it.
pub(super) fn drive_scripted_align(
    mut commands: Commands,
    time: Res<Time>,
    settings: Res<FlightSettings>,
    mut q_rotation_input: Query<
        (&mut ControllerSectionRotationInput, &ChildOf),
        With<ControllerSectionMarker>,
    >,
    q_computer: Query<
        (&PDController, &ChildOf),
        (
            With<ControllerSectionMarker>,
            Without<SectionInactiveMarker>,
        ),
    >,
    q_ship: Query<
        (
            Entity,
            &ScriptedAlign,
            &Position,
            &Rotation,
            &AngularVelocity,
            Option<&ComputedCenterOfMass>,
            Has<ScriptedAlignSettled>,
        ),
        With<SpaceshipRootMarker>,
    >,
) {
    for (ship, align, position, rotation, angular_velocity, com, settled) in &q_ship {
        // The avian pose, like the autopilot beside it: in `FixedUpdate` a
        // `Transform` is the previous frame's eased render pose, and a bearing
        // held off a stale attitude chases its own lag.
        //
        // The bearing runs from live STRUCTURE, exactly as the AI's chase
        // vector does: a root origin is the build spot of the first sections
        // and floats in empty space once they are destroyed. The COM is
        // body-local, so it lifts to world with rotation + translation.
        let own_anchor = com
            .map(|com| rotation.mul_vec3(com.0) + position.0)
            .unwrap_or(position.0);
        let to_mark = align.look_at - own_anchor;
        let Ok(desired_direction) = Dir3::new(to_mark) else {
            // The mark is exactly under the ship's own anchor: no bearing to
            // hold. Nothing to command, and nothing to complete.
            continue;
        };

        let Some(turn_rate) = ship_turn_rate(
            q_computer
                .iter()
                .filter(|(_, &ChildOf(parent))| parent == ship)
                .map(|(pd, _)| pd.max_angular_acceleration),
            &settings,
        ) else {
            continue;
        };
        let max_step = turn_rate * time.delta_secs();

        for (mut rotation_input, _) in q_rotation_input
            .iter_mut()
            .filter(|(_, ChildOf(parent))| *parent == ship)
        {
            let command = **rotation_input;
            let command_forward = command * Vec3::NEG_Z;
            let goal = Quat::from_rotation_arc(command_forward, *desired_direction) * command;
            **rotation_input = slew_rotation(command, goal, max_step);
        }

        if settled {
            continue;
        }
        // Completion is the HULL's aim, not the command's: the command reaches
        // the goal a swing before the ship does.
        let error = rotation
            .mul_vec3(Vec3::NEG_Z)
            .angle_between(*desired_direction);
        if error <= align.tolerance && aim_has_settled(angular_velocity.0, align.tolerance) {
            debug!(
                "drive_scripted_align: ship {ship:?} settled on its bearing \
                 (error {error} rad, tolerance {} rad)",
                align.tolerance
            );
            commands.entity(ship).insert(ScriptedAlignSettled);
        }
    }
}

/// Whether a hull inside its tolerance is actually SETTLED rather than
/// swinging through.
///
/// Derived from the authored tolerance instead of a global threshold: the
/// tolerance is the only statement anyone made about how precise this shot has
/// to be, and a fixed angular-velocity floor would be too strict for a coarse
/// stage move and too loose for a spinal bore. The rule is "it would still be
/// inside tolerance a second from now", which is what a scenario about to
/// spend a charge on the bearing actually needs.
fn aim_has_settled(angular_velocity: Vec3, tolerance: f32) -> bool {
    angular_velocity.length() <= tolerance
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The settle rule is the authored tolerance read as a rate: a hull
    /// drifting slowly enough to stay inside a coarse tolerance for another
    /// second is settled, and the same drift under a tight spinal tolerance is
    /// not. A fixed global floor could not tell those two apart.
    #[test]
    fn the_settle_rule_scales_with_the_authored_tolerance() {
        let drift = Vec3::Y * 0.05;

        assert!(
            aim_has_settled(drift, 0.2),
            "0.05 rad/s stays inside a 0.2 rad tolerance for another second"
        );
        assert!(
            !aim_has_settled(drift, 0.01),
            "the same drift leaves a 0.01 rad spinal tolerance well inside a second"
        );
        assert!(
            aim_has_settled(Vec3::ZERO, 0.0),
            "a dead-still hull is settled at any tolerance"
        );
    }
}
