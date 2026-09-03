//! Ship helm ORDERS: the durable directive a scenario installs on a ship's
//! helm, the driver that flies it, and the alignment drive that turns a hull
//! without translating it.
//!
//! The division of labour is the same one the scripted torpedo bay uses. This
//! module owns the STATE and the physics; the scenario layer owns the
//! vocabulary that installs it and the events that report its lifecycle.
//! Nothing here knows what a scenario is - it reports outcomes into
//! [`ShipOrderReports`] and lets the layer above turn them into events.
//!
//! An order works the same on a ship nobody drives and on an AI ship. The AI
//! keeps perceiving and shooting either way; what the order takes is the
//! HELM, through [`ShipOrderHelmAuthority`], which the AI's flight writers
//! refuse to run against. A player-driven ship is never accepted - the input
//! layer drops the autopilot on any stick movement, so an order there would
//! be a fight rather than a command.
//!
//! Engine units: every position, radius and tolerance here is compared
//! against an avian `Position` every tick, so the authoring seam is the
//! scenario action that builds the directive, not this module.

use avian3d::prelude::*;
use bevy::prelude::*;
use nova_events::prelude::*;
use nova_gameplay::prelude::*;

use super::{
    ship_turn_rate, slew_rotation, Autopilot, AutopilotAction, AutopilotPhase,
    FlightArrivalStandoff, FlightSettings,
};
use crate::prelude::*;

/// The one live HELM order on a ship root.
///
/// Move, align, stop, patrol and orbit are a mutually exclusive family, and
/// this component IS that exclusion: installing an order retires whatever was
/// here first, so a ship can never be flying two authored marks at once. It
/// outlives its own completion on purpose - an alignment holds its facing
/// afterwards, an orbit keeps station-keeping, and the record of what a ship
/// was last told is what makes a replacement legible in a log.
///
/// The directive is DURABLE and carries everything needed to rebuild the
/// maneuver. That is not redundancy with [`Autopilot`]: an AI interruption
/// tears the autopilot down so the ship can fight, and the order has to know
/// how to pick the leg back up afterwards.
#[derive(Component, Clone, Debug, Reflect)]
#[reflect(Component)]
pub struct ShipHelmOrder {
    /// The authored key this order's lifecycle is reported under.
    pub key: String,
    /// What the ship was told to do.
    pub directive: ShipOrderDirective,
}

impl ShipHelmOrder {
    /// A freshly installed order.
    pub fn new(key: String, directive: ShipOrderDirective) -> Self {
        Self { key, directive }
    }

    /// Which order this is, for a payload or a filter.
    pub fn kind(&self) -> ShipOrderKind {
        self.directive.kind()
    }
}

/// What a [`ShipHelmOrder`] tells the helm to do.
///
/// An enum because the helm executes exactly one directive at a time: this is
/// a command kind, not a behavior role, so the cross-product objection that
/// rules out hard-coded AI roles does not apply.
#[derive(Clone, Debug, PartialEq, Reflect)]
pub enum ShipOrderDirective {
    /// Fly to a fixed mark and come to rest.
    Move {
        /// The mark, world coordinates.
        position: Vec3,
        /// How far short of the mark to stop, replacing the ship's own
        /// arrival standoff for the life of the order. `None` keeps whatever
        /// the ship already flies.
        arrival_standoff: Option<f32>,
    },
    /// Turn the hull onto a bearing and hold it, without translating.
    Align {
        /// World position to put under the bore.
        look_at: Vec3,
        /// How close the aim must come, radians.
        tolerance: f32,
    },
    /// Kill the ship's velocity.
    Stop,
    /// Fly one loop of an authored route.
    ///
    /// `leg` is the progress that makes the loop resumable: it counts flown
    /// legs, not waypoints, so an interruption halfway round comes back to
    /// the mark it was flying rather than to the start.
    Patrol {
        /// The route's marks, world coordinates, in order.
        waypoints: Vec<Vec3>,
        /// How many legs are already flown.
        leg: usize,
    },
    /// Circularize into a station-keeping orbit and hold it.
    Orbit {
        /// Scenario id of the gravity well to orbit.
        ///
        /// The authored id, not an `Entity`: the well is resolved every tick,
        /// so a well that has not spawned yet is simply not there yet, and a
        /// well that is destroyed fails the order rather than orbiting a
        /// dangling handle.
        well: String,
    },
}

impl ShipOrderDirective {
    /// Which [`ShipOrderKind`] this directive is.
    pub fn kind(&self) -> ShipOrderKind {
        match self {
            ShipOrderDirective::Move { .. } => ShipOrderKind::Move,
            ShipOrderDirective::Align { .. } => ShipOrderKind::Align,
            ShipOrderDirective::Stop => ShipOrderKind::Stop,
            ShipOrderDirective::Patrol { .. } => ShipOrderKind::Patrol,
            ShipOrderDirective::Orbit { .. } => ShipOrderKind::Orbit,
        }
    }

    /// Whether reaching this directive's condition leaves something still
    /// running that the helm must keep owning.
    ///
    /// Align and orbit are HOLDS: completion reports that the bearing was
    /// settled or the ring established, and the ship keeps holding it until a
    /// scenario says otherwise. The other three end with the ship at rest and
    /// nothing left to do, so their completion hands the helm back - which on
    /// an AI ship is what lets it return to its own routine.
    fn holds_after_completion(&self) -> bool {
        matches!(
            self,
            ShipOrderDirective::Align { .. } | ShipOrderDirective::Orbit { .. }
        )
    }
}

/// How many legs a patrol route flies before the loop is complete.
///
/// One waypoint is one leg: visiting it IS returning to the start. Two or
/// more fly every waypoint in order and then the first one again, which is
/// what "one loop" means for a route that is not a closed list. Duplicate and
/// coincident points are legal and simply arrive together.
fn patrol_leg_count(waypoints: &[Vec3]) -> usize {
    match waypoints.len() {
        0 => 0,
        1 => 1,
        n => n + 1,
    }
}

/// Present while the order owns the ship's helm.
///
/// Separate from [`ShipHelmOrder`] because authority comes and goes while the
/// order stays: an interruption takes it away and a resume gives it back. It
/// is also the gate the AI's flight writers read - they refuse to run on a
/// ship carrying this - so two systems can never write one hull's thrust in
/// the same tick.
#[derive(Component, Clone, Copy, Debug, Default, Reflect)]
#[reflect(Component)]
pub struct ShipOrderHelmAuthority;

/// Present once an order has produced a TERMINAL outcome (completed or
/// failed).
///
/// The latch that makes "one terminal event per accepted order" fall out
/// rather than be enforced at the reporting end. An alignment keeps its order
/// installed while it holds the facing, so the order's presence cannot be the
/// thing that says "not yet told"; and a canceled order checks this to avoid
/// reporting a cancellation for something that already finished.
#[derive(Component, Clone, Copy, Debug, Default, Reflect)]
#[reflect(Component)]
pub struct ShipOrderReported;

/// Present while the order's TRANSIENT execution is installed.
///
/// The difference between "not started yet" and "finished": both look like a
/// move directive with no [`Autopilot`], and without this marker the driver
/// would re-engage the maneuver it just watched complete, forever.
#[derive(Component, Clone, Copy, Debug, Default, Reflect)]
#[reflect(Component)]
pub struct ShipOrderEngaged;

/// Present while autonomous AI has temporarily taken the helm from an
/// installed order.
///
/// The order and its directive stay whole; only the execution is torn down.
/// Paired with the absence of [`ShipOrderHelmAuthority`], which is what
/// actually lets the AI fly.
#[derive(Component, Clone, Copy, Debug, Default, Reflect)]
#[reflect(Component)]
pub struct AIOrderInterrupted;

/// What happened to an order, for the layer that turns it into an event.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Reflect)]
pub enum ShipOrderOutcome {
    /// The physical completion condition was reached. Terminal.
    Complete,
    /// Autonomous AI took the helm; the order is still installed.
    Interrupted,
    /// An interrupted order got the helm back.
    Resumed,
    /// The order was retired on purpose - cleared, or replaced. Terminal.
    Canceled,
    /// An accepted order became impossible to continue. Terminal.
    Failed,
}

/// One unreported thing that happened to a ship's order.
///
/// The key and kind are COPIED rather than read back off the order, because
/// the two terminal-by-retirement outcomes report an order that is already
/// gone by the time anything drains this.
#[derive(Clone, Debug, Reflect)]
pub struct ShipOrderReport {
    /// The order's authored key.
    pub key: String,
    /// Which helm order it was.
    pub kind: ShipOrderKind,
    /// What happened to it.
    pub outcome: ShipOrderOutcome,
}

/// The queue of order outcomes a ship has produced and nothing has reported.
///
/// A queue rather than a flag because a single tick can legitimately produce
/// two: a replacement order cancels the old one and the new one can fail on
/// its first evaluation. Installed alongside the first order and left in
/// place afterwards - an empty queue costs nothing and keeps the driver's
/// query free of an `Option`.
#[derive(Component, Clone, Debug, Default, Reflect)]
#[reflect(Component)]
pub struct ShipOrderReports(pub Vec<ShipOrderReport>);

impl ShipOrderReports {
    /// Queue one outcome.
    fn push(&mut self, key: &str, kind: ShipOrderKind, outcome: ShipOrderOutcome) {
        self.0.push(ShipOrderReport {
            key: key.to_string(),
            kind,
            outcome,
        });
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
/// A ship with no live flight computer cannot turn. `drive_ship_orders` calls
/// that a FAILURE rather than letting the order hang forever, so the beat
/// waiting on the bearing gets told.
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

/// Install, advance, complete and fail every order that owns its ship's helm.
///
/// The order layer's whole engine. It runs BEFORE the autopilot each tick so
/// a maneuver engaged here burns the same tick it was engaged, and it reads
/// the previous tick's autopilot state to decide whether a leg ended.
///
/// The one subtlety is telling "arrived" from "gave up": the autopilot
/// removes itself for both, so a disengage is read against the hull's
/// remaining capability. A ship that still has a flight computer and an
/// engine finished its leg; one that has lost either could not have.
#[expect(
    clippy::too_many_arguments,
    reason = "one system owns the whole order lifecycle; splitting it would split the latch"
)]
pub(super) fn drive_ship_orders(
    mut commands: Commands,
    mut q_ships: Query<
        (
            Entity,
            &mut ShipHelmOrder,
            &mut ShipOrderReports,
            Option<&Autopilot>,
            Has<ShipOrderEngaged>,
            Has<ScriptedAlignSettled>,
            Has<ShipOrderReported>,
        ),
        (With<SpaceshipRootMarker>, With<ShipOrderHelmAuthority>),
    >,
    q_wells: Query<(Entity, &EntityId), With<GravityWell>>,
    q_computer: Query<
        &ChildOf,
        (
            With<ControllerSectionMarker>,
            Without<SectionInactiveMarker>,
        ),
    >,
    q_engine: Query<&ChildOf, (With<ThrusterSectionMarker>, Without<SectionInactiveMarker>)>,
    mut q_thruster_input: Query<(&mut ThrusterSectionInput, &ChildOf), With<ThrusterSectionMarker>>,
    q_standoff: Query<&FlightArrivalStandoff>,
) {
    for (ship, mut order, mut reports, autopilot, engaged, settled, reported) in &mut q_ships {
        let kind = order.kind();
        let can_turn = q_computer.iter().any(|&ChildOf(parent)| parent == ship);
        let can_burn = q_engine.iter().any(|&ChildOf(parent)| parent == ship);

        if !engaged {
            match engage_leg(
                ship,
                &order,
                &mut commands,
                &q_wells,
                &mut q_thruster_input,
                &q_standoff,
            ) {
                Ok(()) => {
                    commands.entity(ship).insert(ShipOrderEngaged);
                }
                Err(reason) => {
                    warn!(
                        "drive_ship_orders: ship {ship:?} order '{}' failed: {reason}",
                        order.key
                    );
                    fail(&mut commands, ship, &order, &mut reports, reported);
                }
            }
            continue;
        }

        // A hold that has already reported keeps running and is done being
        // judged; only an interruption or a cancellation touches it now.
        if reported {
            continue;
        }

        match kind {
            ShipOrderKind::Align => {
                if !can_turn {
                    warn!(
                        "drive_ship_orders: ship {ship:?} lost its flight computer mid-align; \
                         order '{}' can never settle",
                        order.key
                    );
                    fail(&mut commands, ship, &order, &mut reports, reported);
                } else if settled {
                    complete(&mut commands, ship, &order, &mut reports);
                }
            }
            ShipOrderKind::Orbit => {
                // ORBIT never self-completes: the computer holds the ring
                // until something takes the maneuver away. So a missing
                // autopilot here is always the maneuver giving up - the well
                // vanished, or it has no stable band this hull can reach.
                match autopilot {
                    Some(autopilot) if autopilot.phase == AutopilotPhase::Hold => {
                        complete(&mut commands, ship, &order, &mut reports);
                    }
                    Some(_) => {}
                    None => {
                        warn!(
                            "drive_ship_orders: ship {ship:?} lost its ORBIT maneuver before \
                             establishing the ring; order '{}' failed",
                            order.key
                        );
                        fail(&mut commands, ship, &order, &mut reports, reported);
                    }
                }
            }
            ShipOrderKind::Move | ShipOrderKind::Stop => {
                if autopilot.is_some() {
                    continue;
                }
                if can_turn && can_burn {
                    complete(&mut commands, ship, &order, &mut reports);
                } else {
                    warn!(
                        "drive_ship_orders: ship {ship:?} lost the capability its {kind:?} \
                         maneuver runs on; order '{}' failed short of its mark",
                        order.key
                    );
                    fail(&mut commands, ship, &order, &mut reports, reported);
                }
            }
            ShipOrderKind::Patrol => {
                if autopilot.is_some() {
                    continue;
                }
                if !(can_turn && can_burn) {
                    warn!(
                        "drive_ship_orders: ship {ship:?} lost the capability its patrol runs \
                         on; order '{}' failed mid-route",
                        order.key
                    );
                    fail(&mut commands, ship, &order, &mut reports, reported);
                    continue;
                }
                let closed = {
                    let ShipOrderDirective::Patrol { waypoints, leg } = &mut order.directive else {
                        unreachable!("the kind came from the directive");
                    };
                    *leg += 1;
                    *leg >= patrol_leg_count(waypoints)
                };
                if closed {
                    complete(&mut commands, ship, &order, &mut reports);
                } else if let Err(reason) = engage_leg(
                    // Straight into the next leg rather than through a dead
                    // tick: a route that coasted for a frame at every corner
                    // would read as stop-and-go rather than as one sweep.
                    ship,
                    &order,
                    &mut commands,
                    &q_wells,
                    &mut q_thruster_input,
                    &q_standoff,
                ) {
                    warn!(
                        "drive_ship_orders: ship {ship:?} patrol '{}' failed: {reason}",
                        order.key
                    );
                    fail(&mut commands, ship, &order, &mut reports, reported);
                }
            }
        }
    }
}

/// Build the transient execution one directive needs, from the durable order.
///
/// The same call installs a fresh order and resumes an interrupted one -
/// there is no third state to get wrong, which is the point of keeping the
/// directive whole.
fn engage_leg(
    ship: Entity,
    order: &ShipHelmOrder,
    commands: &mut Commands,
    q_wells: &Query<(Entity, &EntityId), With<GravityWell>>,
    q_thruster_input: &mut Query<
        (&mut ThrusterSectionInput, &ChildOf),
        With<ThrusterSectionMarker>,
    >,
    q_standoff: &Query<&FlightArrivalStandoff>,
) -> Result<(), String> {
    match &order.directive {
        ShipOrderDirective::Move {
            position,
            arrival_standoff,
        } => {
            if let Some(standoff) = arrival_standoff {
                // Recorded so retiring the order can put the hull back the
                // way it found it; a cinematic's tight staging tolerance must
                // not outlive the cinematic.
                let previous = q_standoff.get(ship).ok().map(|standoff| **standoff);
                commands.entity(ship).insert((
                    SuspendedArrivalStandoff(previous),
                    FlightArrivalStandoff(*standoff),
                ));
            }
            commands
                .entity(ship)
                .insert(Autopilot::engage(AutopilotAction::GotoPos {
                    position: *position,
                }));
            Ok(())
        }
        ShipOrderDirective::Align { look_at, tolerance } => {
            // An alignment engages no autopilot, so nothing else will cut a
            // burn the ship was already running - an AI hull that was
            // chasing when the order landed would keep its throttle open
            // under a helm that only turns. Cut it here, once.
            for (mut input, &ChildOf(parent)) in q_thruster_input.iter_mut() {
                if parent == ship {
                    **input = 0.0;
                }
            }
            commands.entity(ship).insert(ScriptedAlign {
                look_at: *look_at,
                tolerance: *tolerance,
            });
            Ok(())
        }
        ShipOrderDirective::Stop => {
            commands
                .entity(ship)
                .insert(Autopilot::engage(AutopilotAction::Stop));
            Ok(())
        }
        ShipOrderDirective::Patrol { waypoints, leg } => {
            let count = patrol_leg_count(waypoints);
            if count == 0 {
                return Err("the patrol route has no waypoints".to_string());
            }
            // The last leg of a multi-point loop returns to the start; every
            // other leg is the waypoint at its own index.
            let waypoint = waypoints[*leg % waypoints.len()];
            commands
                .entity(ship)
                .insert(Autopilot::engage(AutopilotAction::GotoPos {
                    position: waypoint,
                }));
            Ok(())
        }
        ShipOrderDirective::Orbit { well } => {
            let Some(entity) = q_wells
                .iter()
                .find(|(_, id)| id.0 == *well)
                .map(|(entity, _)| entity)
            else {
                return Err(format!("gravity well '{well}' is not in the world"));
            };
            commands
                .entity(ship)
                .insert(Autopilot::engage(AutopilotAction::Orbit {
                    well: entity,
                    plan: None,
                }));
            Ok(())
        }
    }
}

/// Report a completion and hand the helm back unless the directive is a hold.
fn complete(
    commands: &mut Commands,
    ship: Entity,
    order: &ShipHelmOrder,
    reports: &mut ShipOrderReports,
) {
    debug!(
        "drive_ship_orders: ship {ship:?} completed {:?} order '{}'",
        order.kind(),
        order.key
    );
    reports.push(&order.key, order.kind(), ShipOrderOutcome::Complete);
    commands.entity(ship).insert(ShipOrderReported);
    if !order.directive.holds_after_completion() {
        commands.entity(ship).remove::<ShipOrderHelmAuthority>();
    }
}

/// Report a failure and hand the helm back.
///
/// A failure always releases the helm: whatever the order was going to do, it
/// is not going to do it, and holding an AI ship's helm for a maneuver that
/// cannot run would strand it. `reported` guards the one case a terminal
/// outcome can be reached twice - an order that failed on its engage tick and
/// is evaluated again before anything drains the queue.
fn fail(
    commands: &mut Commands,
    ship: Entity,
    order: &ShipHelmOrder,
    reports: &mut ShipOrderReports,
    reported: bool,
) {
    if !reported {
        reports.push(&order.key, order.kind(), ShipOrderOutcome::Failed);
    }
    commands.entity(ship).insert(ShipOrderReported);
    commands
        .entity(ship)
        .remove::<(ShipOrderHelmAuthority, ShipOrderEngaged)>();
}

/// Tear down an order's TRANSIENT execution, leaving the durable order alone.
///
/// Everything an engage installed comes back off, including the arrival
/// standoff a move displaced. What survives is the directive - which is the
/// whole reason an interruption can be resumed rather than restarted.
pub fn retire_ship_order_execution(world: &mut World, ship: Entity) {
    let Ok(mut entity) = world.get_entity_mut(ship) else {
        return;
    };
    if let Some(SuspendedArrivalStandoff(previous)) =
        entity.get::<SuspendedArrivalStandoff>().copied()
    {
        match previous {
            Some(standoff) => {
                entity.insert(FlightArrivalStandoff(standoff));
            }
            None => {
                entity.remove::<FlightArrivalStandoff>();
            }
        }
        entity.remove::<SuspendedArrivalStandoff>();
    }
    entity.remove::<(
        ShipOrderEngaged,
        ScriptedAlign,
        ScriptedAlignSettled,
        Autopilot,
    )>();
}

/// Retire a ship's order for good, reporting the cancellation.
///
/// Every install runs this first, so replacement and `ClearShipOrder` are one
/// path: an order that is being taken away did not finish, and a beat waiting
/// on its completion must not run. An order that ALREADY reached a terminal
/// outcome reports nothing - a cleared alignment that settled ten seconds ago
/// completed, and saying otherwise would fire two terminal events for one
/// order.
pub fn cancel_ship_order(world: &mut World, ship: Entity) {
    retire_ship_order_execution(world, ship);
    let Ok(mut entity) = world.get_entity_mut(ship) else {
        return;
    };
    let retired = entity
        .get::<ShipHelmOrder>()
        .map(|order| (order.key.clone(), order.kind()));
    let reported = entity.contains::<ShipOrderReported>();
    if let Some((key, kind)) = retired {
        if !reported {
            debug!("cancel_ship_order: ship {ship:?} lost {kind:?} order '{key}'");
            if let Some(mut reports) = entity.get_mut::<ShipOrderReports>() {
                reports.push(&key, kind, ShipOrderOutcome::Canceled);
            }
        }
    }
    entity.remove::<(
        ShipHelmOrder,
        ShipOrderHelmAuthority,
        ShipOrderReported,
        AIOrderInterrupted,
    )>();
}

/// Hand a ship's helm from its order to autonomous AI, keeping the order.
///
/// The execution goes; the directive stays. Nothing here decides WHETHER to
/// interrupt - that is the AI's mission policy, one layer up.
pub fn interrupt_ship_order(world: &mut World, ship: Entity) {
    retire_ship_order_execution(world, ship);
    let Ok(mut entity) = world.get_entity_mut(ship) else {
        return;
    };
    let Some((key, kind)) = entity
        .get::<ShipHelmOrder>()
        .map(|order| (order.key.clone(), order.kind()))
    else {
        return;
    };
    debug!("interrupt_ship_order: ship {ship:?} yields {kind:?} order '{key}' to its AI");
    if let Some(mut reports) = entity.get_mut::<ShipOrderReports>() {
        reports.push(&key, kind, ShipOrderOutcome::Interrupted);
    }
    entity.remove::<ShipOrderHelmAuthority>();
    entity.insert(AIOrderInterrupted);
}

/// Give an interrupted order its helm back; the driver rebuilds the maneuver
/// from the directive on the next tick.
pub fn resume_ship_order(world: &mut World, ship: Entity) {
    let Ok(mut entity) = world.get_entity_mut(ship) else {
        return;
    };
    let Some((key, kind)) = entity
        .get::<ShipHelmOrder>()
        .map(|order| (order.key.clone(), order.kind()))
    else {
        return;
    };
    debug!("resume_ship_order: ship {ship:?} takes {kind:?} order '{key}' back from its AI");
    if let Some(mut reports) = entity.get_mut::<ShipOrderReports>() {
        reports.push(&key, kind, ShipOrderOutcome::Resumed);
    }
    entity.remove::<AIOrderInterrupted>();
    entity.insert(ShipOrderHelmAuthority);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One loop means every waypoint and then back to the start - except for
    /// a one-point route, where visiting the point IS returning to it. Pinned
    /// because the empty and single-point cases are exactly where an
    /// off-by-one would either never complete or complete instantly.
    #[test]
    fn a_patrol_loop_ends_where_it_started() {
        assert_eq!(
            patrol_leg_count(&[]),
            0,
            "an empty route has no loop to fly"
        );
        assert_eq!(
            patrol_leg_count(&[Vec3::X]),
            1,
            "a single mark is visited once, not twice"
        );
        assert_eq!(
            patrol_leg_count(&[Vec3::X, Vec3::Y]),
            3,
            "two marks and the return leg"
        );
        assert_eq!(
            patrol_leg_count(&[Vec3::X, Vec3::X, Vec3::X]),
            4,
            "duplicate marks are legal and still count as legs"
        );
    }

    /// Align and orbit REPORT a condition and keep holding it, so they keep
    /// the helm after completing; the other three end at rest with nothing
    /// left to run and hand it back, which is what lets an AI ship return to
    /// its own routine after a move.
    #[test]
    fn only_the_holding_directives_keep_the_helm_after_completing() {
        assert!(ShipOrderDirective::Align {
            look_at: Vec3::ZERO,
            tolerance: 0.1,
        }
        .holds_after_completion());
        assert!(ShipOrderDirective::Orbit {
            well: "planetoid".to_string(),
        }
        .holds_after_completion());

        assert!(!ShipOrderDirective::Stop.holds_after_completion());
        assert!(!ShipOrderDirective::Move {
            position: Vec3::ZERO,
            arrival_standoff: None,
        }
        .holds_after_completion());
        assert!(!ShipOrderDirective::Patrol {
            waypoints: vec![Vec3::X],
            leg: 0,
        }
        .holds_after_completion());
    }

    /// A rig that runs only the order driver: everything it reads - the
    /// autopilot's presence, the settle marker, the live sections - is
    /// component state a test can set outright, so the lifecycle is provable
    /// without flying a ship across a kilometre of physics.
    fn order_app() -> App {
        let mut app = App::new();
        app.add_systems(Update, drive_ship_orders);
        app
    }

    /// A ship with a live flight computer and a live engine, under one order.
    fn ordered_ship(app: &mut App, directive: ShipOrderDirective) -> Entity {
        let ship = app
            .world_mut()
            .spawn((
                SpaceshipRootMarker,
                ShipHelmOrder::new("job".to_string(), directive),
                ShipOrderHelmAuthority,
                ShipOrderReports::default(),
            ))
            .id();
        app.world_mut()
            .spawn((ChildOf(ship), ControllerSectionMarker));
        app.world_mut().spawn((
            ChildOf(ship),
            ThrusterSectionMarker,
            ThrusterSectionInput(0.0),
        ));
        ship
    }

    /// The outcomes a ship has queued and nothing has drained.
    fn outcomes(app: &App, ship: Entity) -> Vec<ShipOrderOutcome> {
        app.world()
            .get::<ShipOrderReports>(ship)
            .expect("the ship carries a report queue")
            .0
            .iter()
            .map(|report| report.outcome)
            .collect()
    }

    /// The autopilot removing itself IS the arrival, and the driver says so
    /// once: it engages the maneuver from the durable directive, installs the
    /// order's staging standoff, and on release reports a completion and hands
    /// the helm back - which is what lets an AI ship return to its own routine.
    #[test]
    fn a_move_engages_its_maneuver_and_reports_the_arrival_once() {
        let mut app = order_app();
        let ship = ordered_ship(
            &mut app,
            ShipOrderDirective::Move {
                position: Vec3::new(0.0, 0.0, -100.0),
                arrival_standoff: Some(4.0),
            },
        );

        app.update();
        assert!(
            matches!(
                app.world().get::<Autopilot>(ship).map(|a| a.action),
                Some(AutopilotAction::GotoPos { position }) if position == Vec3::new(0.0, 0.0, -100.0)
            ),
            "the directive builds the maneuver"
        );
        assert_eq!(
            app.world().get::<FlightArrivalStandoff>(ship).map(|s| **s),
            Some(4.0),
            "and the order's staging standoff with it"
        );
        assert!(outcomes(&app, ship).is_empty(), "nothing has happened yet");

        app.update();
        assert!(
            app.world().get::<Autopilot>(ship).is_some(),
            "an engaged maneuver is not re-engaged and not read as finished"
        );

        app.world_mut().entity_mut(ship).remove::<Autopilot>();
        app.update();
        assert_eq!(outcomes(&app, ship), vec![ShipOrderOutcome::Complete]);
        assert!(
            !app.world()
                .entity(ship)
                .contains::<ShipOrderHelmAuthority>(),
            "a finished move hands the helm back"
        );

        app.update();
        assert_eq!(
            outcomes(&app, ship),
            vec![ShipOrderOutcome::Complete],
            "and does not report a second time"
        );
    }

    /// The autopilot disengages for TWO reasons - the maneuver finished, or
    /// the hull can no longer fly it - and reading the second as an arrival
    /// would tell a waiting beat the ship reached a mark it died short of.
    #[test]
    fn a_move_that_loses_its_engines_fails_instead_of_reporting_an_arrival() {
        let mut app = order_app();
        let ship = ordered_ship(
            &mut app,
            ShipOrderDirective::Move {
                position: Vec3::new(0.0, 0.0, -100.0),
                arrival_standoff: None,
            },
        );
        app.update();

        let engine = app
            .world_mut()
            .query_filtered::<Entity, With<ThrusterSectionMarker>>()
            .iter(app.world())
            .next()
            .expect("the fixture spawned one engine");
        app.world_mut().entity_mut(engine).despawn();
        app.world_mut().entity_mut(ship).remove::<Autopilot>();
        app.update();

        assert_eq!(outcomes(&app, ship), vec![ShipOrderOutcome::Failed]);
        assert!(
            !app.world()
                .entity(ship)
                .contains::<ShipOrderHelmAuthority>(),
            "a failed order does not strand the hull holding a helm it cannot use"
        );
    }

    /// One loop means every waypoint and then the first again, and only that
    /// last arrival completes: a beat chained off a patrol fires once a lap,
    /// not once a leg.
    #[test]
    fn a_patrol_flies_every_leg_before_it_closes_the_loop() {
        let mut app = order_app();
        let route = vec![Vec3::new(100.0, 0.0, 0.0), Vec3::new(0.0, 0.0, 100.0)];
        let ship = ordered_ship(
            &mut app,
            ShipOrderDirective::Patrol {
                waypoints: route.clone(),
                leg: 0,
            },
        );

        let goal = |app: &App| match app.world().get::<Autopilot>(ship).map(|a| a.action) {
            Some(AutopilotAction::GotoPos { position }) => Some(position),
            _ => None,
        };
        let arrive = |app: &mut App| {
            app.world_mut().entity_mut(ship).remove::<Autopilot>();
            app.update();
        };

        app.update();
        assert_eq!(goal(&app), Some(route[0]));
        arrive(&mut app);
        assert_eq!(goal(&app), Some(route[1]), "straight onto the next leg");
        assert!(outcomes(&app, ship).is_empty(), "mid-route, nothing to say");
        arrive(&mut app);
        assert_eq!(goal(&app), Some(route[0]), "and home to where it started");
        assert!(outcomes(&app, ship).is_empty());
        arrive(&mut app);
        assert_eq!(outcomes(&app, ship), vec![ShipOrderOutcome::Complete]);
    }

    /// An interruption keeps the DIRECTIVE and throws away only the
    /// execution, so a resume flies the same mark rather than restarting
    /// somewhere else - and the order's staging standoff comes off while the
    /// AI has the helm, so the ship fights on its own tuning.
    #[test]
    fn an_interrupted_order_resumes_from_its_own_directive() {
        let mut app = order_app();
        let ship = ordered_ship(
            &mut app,
            ShipOrderDirective::Move {
                position: Vec3::new(0.0, 0.0, -100.0),
                arrival_standoff: Some(4.0),
            },
        );
        app.update();

        interrupt_ship_order(app.world_mut(), ship);
        let entity = app.world().entity(ship);
        assert!(entity.contains::<ShipHelmOrder>(), "the order survives");
        assert!(entity.contains::<AIOrderInterrupted>());
        assert!(!entity.contains::<ShipOrderHelmAuthority>());
        assert!(!entity.contains::<Autopilot>(), "the execution does not");
        assert!(
            !entity.contains::<FlightArrivalStandoff>(),
            "and neither does the order's staging standoff"
        );

        // With no helm authority the driver leaves the ship alone entirely.
        app.update();
        assert!(app.world().get::<Autopilot>(ship).is_none());

        resume_ship_order(app.world_mut(), ship);
        app.update();
        assert!(
            matches!(
                app.world().get::<Autopilot>(ship).map(|a| a.action),
                Some(AutopilotAction::GotoPos { position }) if position == Vec3::new(0.0, 0.0, -100.0)
            ),
            "the same mark, rebuilt from the directive"
        );
        assert_eq!(
            outcomes(&app, ship),
            vec![ShipOrderOutcome::Interrupted, ShipOrderOutcome::Resumed],
            "and both edges are reported, in the order they happened"
        );
    }

    /// Cancelling an unfinished order reports it; cancelling one that already
    /// reached a terminal outcome reports nothing, because one order fires one
    /// terminal event and this one already fired it.
    #[test]
    fn cancelling_reports_only_an_order_that_had_not_finished() {
        let mut app = order_app();
        let ship = ordered_ship(&mut app, ShipOrderDirective::Stop);
        app.update();

        cancel_ship_order(app.world_mut(), ship);
        assert_eq!(outcomes(&app, ship), vec![ShipOrderOutcome::Canceled]);
        let entity = app.world().entity(ship);
        assert!(!entity.contains::<ShipHelmOrder>());
        assert!(!entity.contains::<Autopilot>());

        let ship = ordered_ship(&mut app, ShipOrderDirective::Stop);
        app.update();
        app.world_mut().entity_mut(ship).remove::<Autopilot>();
        app.update();
        assert_eq!(outcomes(&app, ship), vec![ShipOrderOutcome::Complete]);

        cancel_ship_order(app.world_mut(), ship);
        assert_eq!(
            outcomes(&app, ship),
            vec![ShipOrderOutcome::Complete],
            "a finished order is not also canceled"
        );
    }

    /// An orbit reports the ring ESTABLISHED and keeps holding it - the same
    /// contract an alignment has. Losing the maneuver before that is a
    /// failure, not a completion: ORBIT never self-completes.
    #[test]
    fn an_orbit_reports_the_established_ring_and_keeps_the_helm() {
        let mut app = order_app();
        let well = app
            .world_mut()
            .spawn((
                GravityWell::from_mass(1_000.0, 50.0, &GravitySettings::default()),
                EntityId::new("planetoid"),
            ))
            .id();
        let ship = ordered_ship(
            &mut app,
            ShipOrderDirective::Orbit {
                well: "planetoid".to_string(),
            },
        );

        app.update();
        assert!(
            matches!(
                app.world().get::<Autopilot>(ship).map(|a| a.action),
                Some(AutopilotAction::Orbit { well: engaged, .. }) if engaged == well
            ),
            "the authored id resolves to the live well"
        );
        assert!(outcomes(&app, ship).is_empty(), "inserting is not holding");

        app.world_mut()
            .get_mut::<Autopilot>(ship)
            .expect("the maneuver is engaged")
            .phase = AutopilotPhase::Hold;
        app.update();
        assert_eq!(outcomes(&app, ship), vec![ShipOrderOutcome::Complete]);
        assert!(
            app.world()
                .entity(ship)
                .contains::<ShipOrderHelmAuthority>(),
            "a hold keeps the helm: the ship is still station-keeping"
        );
    }

    /// An orbit whose well is gone fails rather than hanging: the ship cannot
    /// circularize around something that stopped existing, and a beat waiting
    /// on the insertion has to be told.
    #[test]
    fn an_orbit_around_a_missing_well_fails() {
        let mut app = order_app();
        let ship = ordered_ship(
            &mut app,
            ShipOrderDirective::Orbit {
                well: "planetoid".to_string(),
            },
        );

        app.update();
        assert_eq!(outcomes(&app, ship), vec![ShipOrderOutcome::Failed]);
        assert!(app.world().get::<Autopilot>(ship).is_none());
    }

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
