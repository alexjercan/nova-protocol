//! The autopilot: one rule flies STOP, GOTO, GotoPos and ORBIT - compute
//! the desired velocity for the goal, rotate the cheapest engine group onto
//! the velocity error, and burn. Plus the disengage cleanup that cools the
//! engines and parks the helm.
//!
//! Engine units throughout: every figure here is read from or written to an
//! avian `Position`, `LinearVelocity` or `Forces`, so a distance is a world
//! unit (10 m), a speed a world unit per second and an acceleration a world
//! unit per second squared.

use avian3d::prelude::*;
use bevy::{
    ecs::entity::{EntityHashMap, EntityHashSet},
    platform::collections::HashMap,
    prelude::*,
};
use nova_gameplay::prelude::*;

use super::{
    capability::{ship_capabilities, LiveFlightComputers, ShipCapabilityQuery},
    dock_corridor::{
        dock_berth_gap, DockBlocker, DockCorridors, DOCK_FINAL_LEG_ANGLE, DOCK_FINAL_LEG_LATERAL,
    },
    dock_forecast::{station_frame_sweep, DockForecast, FieldBody, DOCK_FORECAST_REFRESH},
    guidance::{
        arrival_eta, flip_lead, goto_desired_velocity, goto_flip_point, orbit_band_floor,
        orbit_desired_velocity, orbit_plane_normal, orbit_ring_offset, orbit_target_radius,
        ship_turn_rate, slew_rotation, slew_urgency, spool_tail, stop_rest_distance, FlipEstimate,
    },
    navigation::{goal_outside, search_leg},
    state::DOCK_BLOCKED_SECS,
    thrusters::{
        balance_throttles, burn_input, choose_group, cluster_thrusters, hold_for_opposed_wind_down,
        spool, BalanceEngine,
    },
};
use crate::{
    input::ai::AI_EVADE_LEG_SLACK_SECS,
    prelude::*,
    sections::{
        docking_section::GAP_SLACK,
        thruster_section::{engine_direction, engine_direction_local},
    },
};

/// Fraction of `rcs_accel` the local gravity accel must stay under for the
/// autopilot to hand a goal to the RCS - the ORBIT trim or the STOP settle.
/// Below it the RCS push has better than 6x authority over the inward pull,
/// enough headroom to correct a perturbation; above it the main drive (full
/// authority) keeps the goal.
///
/// The number is a fraction, but the threshold it has to reproduce is
/// absolute: ~0.75 u/s^2, the value the "menu ambience ships crashed the
/// asteroid" regression was validated at. It is written as a fraction of
/// `rcs_accel` so a hull with a weaker RCS gets a proportionally stricter
/// gate - which means a retune of `rcs_accel` MOVES this threshold, and the
/// fraction has to move with it. The menu planetoid's ~2.2 u/s^2 pull far
/// exceeds `rcs_accel * 0.15 = 0.74`, so its orbits stay on the main drive.
const RCS_GRAVITY_AUTHORITY: f32 = 0.15;

/// The fill a refilling RCS magazine reaches before the autopilot hands it a
/// goal again. Recovery pays a sliver per tick, and a trim against a standing
/// pull (a tidal one at an orbiting station) spends each sliver the tick it
/// lands, so without this floor the magazine stays near empty and the main
/// drive and the RCS trade the goal every tick.
const RCS_RESUME_FRACTION: f32 = 0.5;

/// The RCS deflection an RCS settle may still be pushing at when a rest leg
/// releases, as a fraction of full deflection - the same wind-down the main
/// drive's throttle is held to before release. The settle is proportional
/// (`residual / crumb band`), so this is a residual of a twentieth of the
/// band: a rest the drive's own epsilon cannot promise.
const RCS_RELEASE_DEFLECTION: f32 = 0.05;

/// The mover the autopilot flies, as one step reads it: the root pose, the
/// flown centre of mass and the mass properties. A docked root that drives its
/// pair carries the assembly's centre of mass, velocity, mass, inertia and
/// reach instead of its own.
pub(super) struct FlightBody {
    /// Root origin, world space; engine lever arms are composed from it.
    pub(super) position: Vec3,
    /// Root attitude.
    pub(super) rotation: Quat,
    /// The point the leg flies, world space.
    pub(super) center_of_mass: Vec3,
    /// [`Self::center_of_mass`] in the root's body frame.
    pub(super) local_center_of_mass: Vec3,
    /// Velocity of [`Self::center_of_mass`].
    pub(super) linear_velocity: Vec3,
    /// Root angular velocity, world space.
    pub(super) angular_velocity: Vec3,
    /// The plan divides by it; a step at zero mass holds the leg.
    pub(super) mass: f32,
    /// Inertia about [`Self::center_of_mass`], in the root's body frame.
    pub(super) angular_inertia: ComputedAngularInertia,
    /// This hull's (or assembly's) reach from the centre of mass to its face.
    pub(super) mover_radius: f32,
}

/// One live thruster section of the mover. The step skips an engine whose
/// mount names no direction, but still spools it.
pub(super) struct HelmEngine {
    /// The thruster section entity the system writes [`Self::input`] back to.
    pub(super) entity: Entity,
    /// The section's fixed mount on the hull.
    pub(super) mount: Transform,
    /// Thrust at full input.
    pub(super) magnitude: f32,
    /// Spooled input, `0..1`; the step advances it one tick.
    pub(super) input: f32,
}

/// One controller section of the mover. Every controller takes the rotation
/// command; only a live one ([`LiveFlightComputers`]) lends its PD authority.
pub(super) struct HelmController {
    /// The controller section entity the system writes [`Self::command`] back
    /// to.
    pub(super) entity: Entity,
    /// The live flight computer's PD, `None` on a disabled or preview section.
    pub(super) live_pd: Option<PDController>,
    /// The rotation command, evolved from its own previous value.
    pub(super) command: Quat,
}

/// The actuator state one step reads and writes.
pub(super) struct FlightHelm {
    /// Every live thruster section, in query order.
    pub(super) engines: Vec<HelmEngine>,
    /// Every controller section, in query order.
    pub(super) controllers: Vec<HelmController>,
    /// The torque-free RCS command, body frame, each axis `-1..1`.
    pub(super) rcs_intent: Vec3,
    /// The RCS magazine the settle and trim hand-offs check.
    pub(super) rcs_budget: RcsBudget,
}

/// A GOTO or GotoPos goal, resolved by the system: where the leg anchors, the
/// size measured from that anchor, and the band floor a well-bearing target
/// parks at.
pub(super) struct ArrivalTarget {
    /// The target entity; `None` for a GotoPos mark.
    pub(super) entity: Option<Entity>,
    /// The anchor point, world space.
    pub(super) goal: Vec3,
    /// The target's size measured from [`Self::goal`].
    pub(super) radius: f32,
    /// The ORBIT band floor for a well-bearing target, else zero.
    pub(super) floor: f32,
    /// Where [`Self::goal`] sits on the target.
    pub(super) anchor: ArrivalAnchorType,
}

/// The point of a target an [`ArrivalTarget::goal`] is measured from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ArrivalAnchorType {
    /// The target's avian origin: a `BodyRadius` or well body radius is the
    /// larger size. A GotoPos mark is its own origin.
    Origin,
    /// The target's world centre of mass: its `HullRadius` is the larger size,
    /// or it publishes no size.
    CenterOfMass,
}

/// One gravity well as the step reads it.
pub(super) struct WellSample {
    /// The well entity, for ORBIT and GOTO lookups.
    pub(super) entity: Entity,
    /// The avian position the well pulls from.
    pub(super) position: Vec3,
    /// The physics well the pull is computed from.
    pub(super) well: GravityWell,
    /// The same well with its body radius raised to the body's geometric
    /// extent, for the ORBIT band math.
    pub(super) band_well: GravityWell,
}

/// What one step reads about the world around the mover.
pub(super) struct FlightEnvironment<'a> {
    /// The fixed tick, seconds.
    pub(super) dt: f32,
    /// Flight tuning.
    pub(super) settings: &'a FlightSettings,
    /// Gravity shaping, for the well pull.
    pub(super) gravity: &'a GravitySettings,
    /// Every gravity well, in query order.
    pub(super) wells: &'a [WellSample],
    /// The mover's resolved arrival margin.
    pub(super) arrival_standoff: f32,
    /// The mover's resolved detour margin: the daylight a GOTO leg keeps
    /// between the mover's skin and a body's.
    pub(super) avoid_margin: f32,
    /// The sized bodies a GOTO leg routes around. See [`ObstacleQuery`].
    pub(super) obstacles: &'a [FieldBody],
    /// The mover's [`FlightCruiseCap`], engine units per second, when it
    /// wears one.
    pub(super) cruise_cap: Option<f32>,
    /// What the mover is permitted to do.
    pub(super) capabilities: ShipCapabilities,
    /// The station a DOCK approach flies to, as this tick reads it; `None`
    /// for every other action and for a station that is gone.
    pub(super) dock: Option<DockTarget<'a>>,
}

/// A DOCK approach's station, as one step reads it.
#[derive(Clone, Copy, Debug)]
pub(super) struct DockTarget<'a> {
    /// The selected pair, graded this tick; `None` while it is held and no
    /// other pair is free, so the approach waits.
    pub(super) pair: Option<DockingPair>,
    /// The station root's velocity, engine units per second.
    pub(super) station_velocity: Vec3,
    /// The station root's angular velocity, radians per second: leg B leads
    /// the port with it.
    pub(super) station_spin: Vec3,
    /// The station's centre of mass and containment radius: the body leg A
    /// routes around.
    pub(super) station_bubble: (Vec3, f32),
    /// Every field body's path relative to the station, (body, centre,
    /// radius) in this tick's world frame: what leg A routes around besides
    /// the station. See [`station_frame_sweep`].
    pub(super) forecast: &'a [(Entity, Vec3, f32)],
}

impl DockTarget<'_> {
    /// The velocity of the station's own point at `point`: its centre of
    /// mass's velocity (avian's `LinearVelocity`) plus its spin about that
    /// centre. A hull that holds it moves as if bolted to the station; one
    /// that holds the centre's velocity alone near a spinning station is
    /// passed by its port at the spin times the lever and never settles onto
    /// its axis.
    fn frame_velocity(&self, point: Vec3) -> Vec3 {
        self.station_velocity + self.station_spin.cross(point - self.station_bubble.0)
    }
}

/// The proportional gain of a DOCK approach's closing velocity, per second.
const DOCK_CLOSING_GAIN: f32 = 0.3;

/// The offset inside which a DOCK approach closes at [`DOCK_NEAR_CAP`],
/// engine units (20 m). Above it the proportional law slows the hull to
/// 0.6 u/s. The crawl is kept short: at an orbiting station the RCS trims the
/// tidal pull through the whole close and refills only when idle, so a long
/// crawl empties the magazine short of the berth and the hull drifts out of
/// the lateral gate.
const DOCK_CLOSING_SWITCH: f32 = 2.0;

/// The fastest a DOCK approach closes outside [`DOCK_CLOSING_SWITCH`],
/// engine units per second (20 m/s).
const DOCK_FAR_CAP: f32 = 2.0;

/// The fastest a DOCK approach closes inside [`DOCK_CLOSING_SWITCH`],
/// engine units per second (2 m/s): inside the 5 m/s capture gate.
const DOCK_NEAR_CAP: f32 = 0.2;

/// The velocity, relative to the station, that closes `offset`: proportional
/// to it, capped at [`DOCK_FAR_CAP`] or, inside [`DOCK_CLOSING_SWITCH`], at
/// [`DOCK_NEAR_CAP`].
fn dock_closing_velocity(offset: Vec3) -> Vec3 {
    let cap = if offset.length() > DOCK_CLOSING_SWITCH {
        DOCK_FAR_CAP
    } else {
        DOCK_NEAR_CAP
    };
    (offset * DOCK_CLOSING_GAIN).clamp_length_max(cap)
}

/// The velocity, relative to the station, that closes `offset` to the berth
/// once the gates pass: [`DOCK_NEAR_CAP`] inside [`DOCK_CLOSING_SWITCH`], so
/// it never decays to zero. A proportional close stalls far from the origin:
/// at 250 km the f32 position quantum is 0.002 u, and once the commanded step
/// per 64 Hz tick falls under half of it, `Position` never moves while
/// `LinearVelocity` reads the command. A 0.2 u/s step is 0.003 u, enough out
/// to 65536 u; at the berth it chatters by one step and capture fires on the
/// way in.
fn dock_final_close_velocity(offset: Vec3) -> Vec3 {
    if offset.length() > DOCK_CLOSING_SWITCH {
        dock_closing_velocity(offset)
    } else {
        offset.normalize_or_zero() * DOCK_NEAR_CAP
    }
}

/// Seconds the capped proportional closing law takes to bring an offset of
/// `distance` down to `settle`, engine units.
fn dock_closing_secs(distance: f32, settle: f32) -> f32 {
    let mut secs = 0.0;
    for (low, high, cap) in [
        (DOCK_CLOSING_SWITCH, f32::INFINITY, DOCK_FAR_CAP),
        (0.0, DOCK_CLOSING_SWITCH, DOCK_NEAR_CAP),
    ] {
        let from = distance.min(high);
        let to = settle.max(low);
        if from <= to {
            continue;
        }
        // Below the knee the law is proportional and decays exponentially.
        let knee = cap / DOCK_CLOSING_GAIN;
        let capped_to = to.max(knee);
        if from > capped_to {
            secs += (from - capped_to) / cap;
        }
        let proportional_from = from.min(knee);
        if proportional_from > to {
            secs += (proportional_from / to).ln() / DOCK_CLOSING_GAIN;
        }
    }
    secs
}

/// The factor a DOCK approach's deadline allows over the closing law's
/// modelled time. The model ignores the turn-in and the lateral settle the
/// hull flies at the same time, and the side-port rig flew within 4% of an
/// unscaled deadline.
const DOCK_CLOSING_ALLOWANCE: f32 = 1.5;

/// The liveness deadline of an approach whose hull flies `to_entry` engine
/// units to the corridor entry and then `corridor` along it, seconds: a half
/// turn at `turn_rate`, [`DOCK_CLOSING_ALLOWANCE`] times the proportional
/// closing law's time for the two legs (an upper bound on the corridor leg,
/// whose final close holds [`DOCK_NEAR_CAP`]), and the AI's leg slack. A hull that cannot fly the
/// approach in that time is not going to by waiting.
fn dock_approach_deadline(to_entry: f32, corridor: f32, turn_rate: f32) -> f32 {
    let closing = dock_closing_secs(to_entry, DOCK_FINAL_LEG_LATERAL)
        + dock_closing_secs(corridor, GAP_SLACK);
    core::f32::consts::PI / turn_rate + DOCK_CLOSING_ALLOWANCE * closing + AI_EVADE_LEG_SLACK_SECS
}

/// The relative speed, u/s, at or under which a DOCK approach counts its hull
/// settled onto the station's velocity: the closing cap plus 30%. A hull that
/// arrives from cruise at an orbiting station arms its deadline 13 u/s off
/// the station's velocity, overshoots while it brakes and plans a longer leg
/// back; settled, it flies the closing law the deadline models.
const DOCK_SETTLE_SPEED: f32 = 2.6;

/// Extra clearance (u, 100 m) a DETOURING leg demands before it calls the
/// direct leg clear again, so a leg grazing the margin cannot flip
/// blocked/clear every tick - each flip moves the goal and resets the
/// maneuver to its align phase.
pub(super) const DETOUR_HYSTERESIS: f32 = 10.0;

/// The bodies a GOTO, GotoPos or DOCK leg routes around: everything that
/// publishes a geometric [`BodyRadius`] - field rocks, planetoids - except a
/// nav beacon (a mark to fly TO, whose volume stops nothing), a ship (hulls are
/// flown AT and fought around, never detoured) and a station (a dock to arrive
/// at; a DOCK approach adds its own station's bubble).
/// Avian `Position` where the body has one, the render pose otherwise; a
/// static body, or one with no `LinearVelocity`, does not move.
pub(super) type ObstacleQuery<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        Option<&'static Position>,
        &'static GlobalTransform,
        &'static BodyRadius,
        (
            Option<&'static RigidBody>,
            Option<&'static LinearVelocity>,
            Has<GravityAffected>,
            Has<GravityWell>,
        ),
    ),
    (
        Without<BeaconMarker>,
        Without<SpaceshipRootMarker>,
        Without<StationMarker>,
    ),
>;

/// Whether `action` plans its leg around sized bodies. GOTO, GotoPos and a
/// DOCK approach's leg A do; ORBIT, STOP and MatchVelocity never read the
/// field.
pub(super) fn routes_around_bodies(action: AutopilotAction) -> bool {
    matches!(
        action,
        AutopilotAction::Goto { .. }
            | AutopilotAction::GotoPos { .. }
            | AutopilotAction::DockApproach { .. }
    )
}

/// Every obstacle as a step reads it, in query order, when at least one of
/// `legs` routes around bodies; empty otherwise, so a tick with no GOTO or
/// GotoPos leg never walks every sized body in the world. Collected once per
/// system run; the field does not change per ship. The live system passes
/// every flying autopilot's action, the prediction seed its one leg, so both
/// read the same bodies or none.
pub(super) fn obstacle_field(
    q_obstacles: &ObstacleQuery,
    legs: impl IntoIterator<Item = AutopilotAction>,
) -> Vec<FieldBody> {
    if !legs.into_iter().any(routes_around_bodies) {
        return Vec::new();
    }
    q_obstacles
        .iter()
        .map(
            |(entity, position, transform, radius, (body, velocity, affected, well))| FieldBody {
                entity,
                center: position.map_or_else(|| transform.translation(), |p| p.0),
                radius: **radius,
                velocity: velocity
                    .filter(|_| !body.is_some_and(RigidBody::is_static))
                    .map(|velocity| velocity.0),
                // The physics pulls an affected body that is no well itself.
                feels_wells: affected && !well,
            },
        )
        .collect()
}

/// Why a step disengaged the autopilot before flying the tick.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) enum DisengageType {
    /// No live flight computer is left.
    NoFlightComputer,
    /// No live engine with a valid mount direction is left.
    NoLiveEngines,
    /// The GOTO target or the DOCK station is gone.
    TargetGone,
    /// The ORBIT well is gone.
    WellGone,
    /// The ORBIT well has no stable band.
    NoStableBand,
    /// The GOTO leg's planned goal or corner is not finite: the obstacle
    /// geometry it was planned against is invalid, and the leg is not flown.
    NoSafeLeg,
    /// The DOCK hull has no free live docking port.
    NoDockingPort,
    /// The DOCK approach to its selected pair overran its liveness deadline.
    DockDeadline,
    /// No free pair of the DOCK approach has a corridor its hull can close
    /// along without touching anything, or the station's hull is unmeasured.
    NoSafeDockPath,
}

/// How a step ended the maneuver.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) enum AutopilotEndType {
    /// The leg arrived and the autopilot lets go.
    Completed,
    /// A GOTO arrived at a well body and the step re-engaged ORBIT.
    ParkedIntoOrbit,
    /// The step disengaged before flying the tick.
    Disengaged(DisengageType),
}

/// The result of one autopilot step. The actuator writes are in the
/// [`FlightHelm`] the step was given.
pub(super) struct AutopilotStep {
    /// The leg's live numbers, `None` where no leg is published.
    pub(super) telemetry: Option<ManeuverTelemetry>,
    /// Whether a forward engine of the primary set fires this tick.
    pub(super) main_drive_commanded: bool,
    /// `None` while the maneuver continues.
    pub(super) end: Option<AutopilotEndType>,
    /// The well pull along the arrival leg, when the leg lost its stopping
    /// plan this tick.
    pub(super) brake_degraded: Option<f32>,
    /// The zero-mass hold: the step flew nothing, and the system writes no
    /// component back.
    pub(super) held: bool,
}

/// ALL live forward engines, including thrusters with manual per-section
/// bindings (the editor binds keys straight to thrusters): when the computer
/// takes the ship it commands every engine - an editor-built ship would
/// otherwise leave the autopilot with zero authority (it rotated but could
/// never burn). Pressing a bound thruster key is a flight input and
/// disengages instead (see input/player/intent.rs).
pub(super) type HelmThrusterQuery<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static mut ThrusterSectionInput,
        &'static ThrusterSectionMagnitude,
        &'static Transform,
        &'static ChildOf,
    ),
    (
        With<ThrusterSectionMarker>,
        Without<SectionInactiveMarker>,
        Without<SpaceshipRootMarker>,
    ),
>;

/// Every controller section, the rotation command the helm evolves.
pub(super) type HelmControllerQuery<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static mut ControllerSectionRotationInput,
        &'static ChildOf,
    ),
    With<ControllerSectionMarker>,
>;

/// GOTO's goal pose: prefer the target's raw avian Position (a physics body
/// chased at closing speed must be read on the clock of the forces chasing
/// it - in FixedUpdate, GlobalTransform is the previous frame's eased render
/// pose); the GlobalTransform fallback keeps static markers without a
/// physics body navigable.
///
/// The size resolve takes the LARGEST size a target publishes - a solid
/// body's BodyRadius, a hull's HullRadius, or the well radius read below -
/// so it never grows a per-target-kind branch. An unsized mark stays a
/// point, which is honest. A hull's radius is measured from its centre of
/// mass, so a hull target is flown to at its centre of mass too.
pub(super) type ArrivalTargetQuery<'w, 's> = Query<
    'w,
    's,
    (
        Option<&'static Position>,
        Option<&'static Rotation>,
        Option<&'static ComputedCenterOfMass>,
        &'static GlobalTransform,
        Option<&'static BodyRadius>,
        Option<&'static HullRadius>,
    ),
>;

/// ORBIT's well lookup: avian Position (the force system's frame), not
/// GlobalTransform, so the ring the computer flies is the ring gravity
/// pulls on. Without<SpaceshipRootMarker> is a design statement, not an
/// aliasing need: a ship is never an orbit target, even if someone bolts
/// a GravityWell onto one - ORBIT would treat it as "well gone" and
/// disengage. The GOTO arm reads it too (arrival gravity budget +
/// target radius), inheriting the same statement: a ship target never
/// contributes a well radius - ships stay center-relative.
pub(super) type FlightWellQuery<'w, 's> =
    Query<'w, 's, (Entity, &'static Position, &'static GravityWell), Without<SpaceshipRootMarker>>;

/// The strongest well pull at `point`, u/s^2: the same shaped `well_accel`
/// the physics applies, so the SOI fade and the surface clamp are in it.
pub(super) fn local_gravity_accel(
    wells: &[WellSample],
    point: Vec3,
    gravity_settings: &GravitySettings,
) -> f32 {
    wells
        .iter()
        .map(|sample| {
            let well = &sample.well;
            well_accel(
                well.mu,
                (sample.position - point).length(),
                well.body_radius,
                well.soi_radius,
                gravity_settings.fade_fraction,
                gravity_settings.surface_margin,
            )
        })
        .fold(0.0f32, f32::max)
}

/// The strongest pull that differs between `point` and `frame`, u/s^2: per
/// well, the length of its shaped pull at `point` less its pull at `frame`.
/// A hull that holds a velocity relative to a body falling in the same well
/// holds against this tidal difference, not against the whole pull. The ship
/// and the body fall together, so their velocity difference does not see the
/// shared pull. Each well is compared with itself, so this reads low for two
/// bodies that inside overlapping SOIs fall toward different dominant wells;
/// a generated orbiting station stands inside one SOI only.
pub(super) fn relative_gravity_accel(
    wells: &[WellSample],
    point: Vec3,
    frame: Vec3,
    gravity_settings: &GravitySettings,
) -> f32 {
    let pull = |sample: &WellSample, at: Vec3| {
        let well = &sample.well;
        let offset = sample.position - at;
        offset.normalize_or_zero()
            * well_accel(
                well.mu,
                offset.length(),
                well.body_radius,
                well.soi_radius,
                gravity_settings.fade_fraction,
                gravity_settings.surface_margin,
            )
    };
    wells
        .iter()
        .map(|sample| (pull(sample, point) - pull(sample, frame)).length())
        .fold(0.0f32, f32::max)
}

/// Whether the autopilot may hand `budget`'s RCS a goal: any nonempty
/// magazine idle for less than `recovery_delay` seconds, and one idle past it,
/// which is refilling, only from [`RCS_RESUME_FRACTION`].
pub(super) fn rcs_magazine_ready(budget: &RcsBudget, recovery_delay: f32) -> bool {
    !budget.is_empty() && (budget.idle < recovery_delay || budget.fraction() >= RCS_RESUME_FRACTION)
}

/// Whether an RCS pushing at `rcs_accel` u/s^2 out-pushes a local well pull
/// of `gravity_accel` u/s^2 by the [`RCS_GRAVITY_AUTHORITY`] margin.
pub(super) fn rcs_has_gravity_authority(gravity_accel: f32, rcs_accel: f32) -> bool {
    gravity_accel < rcs_accel * RCS_GRAVITY_AUTHORITY
}

/// Every gravity well as a step reads it, in query order.
pub(super) fn well_samples(
    q_wells: &FlightWellQuery,
    q_target: &ArrivalTargetQuery,
) -> Vec<WellSample> {
    // The well's GEOMETRIC radius for orbit-band math: the physics
    // body_radius is the nominal designation radius, but a generated
    // body's collider (noise-displaced mesh) can reach well past it -
    // the derived [`BodyRadius`] on the well entity carries that true
    // extent. The band's clearance floor must clear the real rock,
    // not the designation sphere.
    q_wells
        .iter()
        .map(|(entity, position, well)| {
            let mut band_well = well.clone();
            band_well.body_radius = band_well.body_radius.max(
                q_target
                    .get(entity)
                    .ok()
                    .and_then(|(_, _, _, _, r, _)| r.map(|r| **r))
                    .unwrap_or(0.0),
            );
            WellSample {
                entity,
                position: position.0,
                well: well.clone(),
                band_well,
            }
        })
        .collect()
}

/// The mover one step flies: the root's own body, or the docked pair's when
/// `assembly` is the pair the root drives.
pub(super) fn flight_body(
    (position, rotation): (&Position, &Rotation),
    (velocity, angular_velocity): (&LinearVelocity, &AngularVelocity),
    (mass, angular_inertia): (&ComputedMass, &ComputedAngularInertia),
    com: Option<&ComputedCenterOfMass>,
    assembly: Option<&DockedAssembly>,
    hull_radius: Option<&HullRadius>,
) -> FlightBody {
    // The point the leg flies: the centre of mass, whose velocity is the
    // avian LinearVelocity and which coasts straight while the hull turns
    // about it. The body origin swings around it with attitude - on the
    // block warship, 49 m aft of the origin, a flip walked the origin's
    // distance 100 m off its own velocity, the plan un-planned the flip
    // halfway, and the park set the origin at the margin with the face
    // (measured from the COM, see HullRadius) 50 m inside it. Body-local;
    // lifted to world with rotation + translation (never render scale).
    FlightBody {
        position: position.0,
        rotation: rotation.0,
        center_of_mass: match assembly {
            Some(assembly) => assembly.center_of_mass,
            None => com
                .map(|c| rotation.mul_vec3(c.0) + position.0)
                .unwrap_or(position.0),
        },
        local_center_of_mass: match assembly {
            Some(assembly) => rotation
                .inverse()
                .mul_vec3(assembly.center_of_mass - position.0),
            None => com.map_or(Vec3::ZERO, |c| c.0),
        },
        linear_velocity: assembly.map_or(velocity.0, |assembly| assembly.linear_velocity),
        angular_velocity: angular_velocity.0,
        mass: assembly.map_or_else(|| mass.value(), |assembly| assembly.mass),
        angular_inertia: assembly.map_or(*angular_inertia, |assembly| assembly.inertia),
        // The mover's half of the model. Every centre distance is
        // `target radius + this + margin`.
        mover_radius: match assembly {
            Some(assembly) => assembly.reach,
            None => hull_radius.map_or(0.0, |radius| **radius),
        },
    }
}

/// `ship`'s live thrusters and controller sections in query order, with its
/// RCS command and magazine.
pub(super) fn flight_helm(
    ship: Entity,
    rcs_intent: Option<&RcsIntent>,
    rcs_budget: &RcsBudget,
    q_thruster: &HelmThrusterQuery,
    q_rotation_input: &HelmControllerQuery,
    q_computer: &LiveFlightComputers,
) -> FlightHelm {
    FlightHelm {
        engines: q_thruster
            .iter()
            .filter(|(.., &ChildOf(parent))| parent == ship)
            .map(|(entity, input, magnitude, transform, _)| HelmEngine {
                entity,
                mount: *transform,
                magnitude: **magnitude,
                input: **input,
            })
            .collect(),
        controllers: q_rotation_input
            .iter()
            .filter(|(.., &ChildOf(parent))| parent == ship)
            .map(|(entity, command, _)| HelmController {
                entity,
                live_pd: q_computer.get(entity).ok().map(|(pd, _)| *pd),
                command: **command,
            })
            .collect(),
        rcs_intent: rcs_intent.map_or(Vec3::ZERO, |intent| intent.0),
        rcs_budget: *rcs_budget,
    }
}

/// The goal a GOTO or GotoPos `action` flies to; `None` for every other
/// action and for a GOTO whose target is gone.
pub(super) fn arrival_target(
    action: AutopilotAction,
    wells: &[WellSample],
    q_target: &ArrivalTargetQuery,
    gravity_settings: &GravitySettings,
    settings: &FlightSettings,
) -> Option<ArrivalTarget> {
    match action {
        AutopilotAction::Goto { target } => q_target.get(target).ok().map(
            |(
                target_position,
                target_rotation,
                target_com,
                target_transform,
                body_radius,
                target_hull,
            )| {
                // The LARGEST size the target publishes, AND THE POINT IT WAS
                // MEASURED FROM. The two travel together or the leg parks
                // wrong: a radius is a distance from somewhere, and the goal
                // the arrival subtracts it from has to be that same somewhere.
                //
                // `HullRadius` is COM-relative by definition - "the distance
                // from its live centre of mass to the outer FACE"
                // (`hull_radius.rs`) - so a hull target is anchored at its
                // centre of mass. `BodyRadius` is the mesh's outermost vertex
                // from the object ORIGIN
                // (`nova_scenario/src/objects/asteroid.rs`), and a well's
                // `body_radius` is its physics body, which
                // `WellSample::position`, `orbit_band_floor` and the ORBIT
                // park handoff in `autopilot_step` all measure from the
                // origin as well. So those two anchor at the
                // origin.
                //
                // Mixing them is not a rounding error. A field rock is a
                // dynamic convex hull of a noise-displaced, per-seed-stretched
                // mesh, so its centre of mass is nowhere near its origin, and
                // carving moves it further; the rock also TUMBLES, so the COM
                // offset sweeps around the origin once a rotation and would
                // drag the goal, the readout anchor and the park point with
                // it - the ship parking a COM offset inside the authored
                // margin on one side of the spin and outside it on the other.
                //
                // Max is conservative if two ever disagree. A target that
                // publishes no size at all is a point, and there the two
                // anchors are equally arbitrary: it keeps the centre of mass,
                // which is the one of the pair that means something physical.
                // A well-bearing target parks no closer than the ring ORBIT
                // would accept, so the handoff in `autopilot_step` never has
                // to burn the ship back outward to reach a legal ring.
                let well = wells.iter().find(|sample| sample.entity == target);
                let hull_radius = target_hull.map_or(0.0, |r| **r);
                let origin_radius = body_radius
                    .map_or(0.0, |r| **r)
                    .max(well.map_or(0.0, |sample| sample.well.body_radius));
                let floor = well.map_or(0.0, |sample| {
                    orbit_band_floor(&sample.band_well, gravity_settings, settings)
                });
                let target_origin =
                    target_position.map_or_else(|| target_transform.translation(), |p| p.0);
                let (radius, goal, anchor) = if origin_radius > hull_radius {
                    (origin_radius, target_origin, ArrivalAnchorType::Origin)
                } else {
                    let com = target_position.and(target_com).map_or(Vec3::ZERO, |c| {
                        target_rotation.map_or(c.0, |r| r.mul_vec3(c.0))
                    });
                    (
                        hull_radius,
                        target_origin + com,
                        ArrivalAnchorType::CenterOfMass,
                    )
                };
                ArrivalTarget {
                    entity: Some(target),
                    goal,
                    radius,
                    floor,
                    anchor,
                }
            },
        ),
        // A mark has no size, and no well to floor against: the leg
        // rests one margin off this hull's own face.
        AutopilotAction::GotoPos { position } => Some(ArrivalTarget {
            entity: None,
            goal: position,
            radius: 0.0,
            floor: 0.0,
            anchor: ArrivalAnchorType::Origin,
        }),
        _ => None,
    }
}

/// The station a DOCK approach on `autopilot` flies to this tick, `None` for
/// every other action and for a station that is gone.
///
/// Keeps the selected pair while both its ports are free and, on each
/// `refreshed` forecast, its corridor casts clear again. When either port is
/// held, or the new cast meets something, the free pairs are walked in rank
/// order and the first whose corridor casts clear replaces it; its deadline
/// and leg A corner are cleared, so the step arms a new one. With no free pair
/// the selection and its deadline are kept and the approach waits. With free
/// pairs but none clear, the approach holds with no pair and casts again on
/// each refresh, and refuses once it has held that way [`DOCK_BLOCKED_SECS`]
/// in all. `clearance` is the leg's clearance outside the station's bubble,
/// the hull's arm plus its detour margin. `forecast` is the field's paths
/// relative to the station; a corridor they come within `clearance` of is
/// refused like a cast one.
///
/// One approach per station flies at a time ([`dock_turns`]): without the
/// `turn`, the approach waits as it does on held ports, before any selection.
/// Every call walks the approach's engaged time by `dt`.
///
/// # Errors
///
/// [`DisengageType::NoDockingPort`] when the ship has no free live port, and
/// [`DisengageType::NoSafeDockPath`] when no free pair's corridor has been
/// clear for [`DOCK_BLOCKED_SECS`] or the station's hull is unmeasured. The
/// refusal is warned here, with the first-ranked pair's blocker.
fn dock_approach_target<'a>(
    ship: Entity,
    autopilot: &mut Mut<Autopilot>,
    corridors: &DockCorridors,
    (clearance, forecast, refreshed): (f32, &'a [(Entity, Vec3, f32)], bool),
    (turn, dt): (bool, f32),
) -> Result<Option<DockTarget<'a>>, DisengageType> {
    let AutopilotAction::DockApproach {
        station,
        plan: selected,
        blocked_s,
        ..
    } = autopilot.action
    else {
        return Ok(None);
    };
    // Bookkeeping, not a flown tick: none of these writes marks the maneuver
    // changed.
    if let AutopilotAction::DockApproach { engaged_s, .. } =
        &mut autopilot.bypass_change_detection().action
    {
        *engaged_s += dt;
    }
    // Holding for a clear corridor since an earlier tick.
    let holding = selected.is_none() && blocked_s.is_some();
    let describe = |blocker: &Option<DockBlocker>| {
        blocker.as_ref().map_or_else(
            || "an unmeasured hull".to_string(),
            |blocker| format!("{blocker}"),
        )
    };
    let ports = &corridors.ports;
    let (Some(station_velocity), Some(station_spin)) =
        (ports.body_velocity(station), ports.body_spin(station))
    else {
        return Ok(None);
    };
    if !ports.has_free_port(ship) {
        return Err(DisengageType::NoDockingPort);
    }
    let Some(station_bubble) = corridors.bubble(station) else {
        warn!(
            "autopilot_system: ship {ship:?} cannot plan a DOCK corridor to station \
             {station:?}: the station has no measured centre of mass or hull envelope"
        );
        return Err(DisengageType::NoSafeDockPath);
    };
    let target = |pair| DockTarget {
        pair,
        station_velocity,
        station_spin,
        station_bubble,
        forecast,
    };
    if !turn {
        return Ok(Some(target(None)));
    }
    let set = |autopilot: &mut Mut<Autopilot>, plan: Option<DockPlan>, blocked: Option<f32>| {
        let autopilot = autopilot.bypass_change_detection();
        autopilot.detour = None;
        if let AutopilotAction::DockApproach {
            plan: selected,
            deadline,
            settled,
            blocked_s,
            ..
        } = &mut autopilot.action
        {
            *selected = plan;
            *deadline = None;
            *settled = false;
            *blocked_s = blocked;
        }
    };
    let held_ports =
        selected.and_then(|plan| ports.pair(ship, plan.own_port, station, plan.station_port));
    let held = match held_ports {
        Some(pair) if refreshed => match corridors.plan(ship, &pair, clearance, forecast) {
            Ok(_) => Some(pair),
            Err(blocker) => {
                debug!(
                    "autopilot_system: ship {ship:?} drops its DOCK pair {:?} -> {:?} to station \
                     {station:?}: its corridor is now blocked by {}",
                    pair.first_section,
                    pair.second_section,
                    describe(&blocker),
                );
                set(autopilot, None, blocked_s);
                None
            }
        },
        other => other,
    };
    if let Some(pair) = held {
        return Ok(Some(target(Some(pair))));
    }
    // Holding for a clear corridor: cast again only on a refresh.
    if holding && !refreshed {
        if let AutopilotAction::DockApproach {
            blocked_s: Some(blocked),
            ..
        } = &mut autopilot.bypass_change_detection().action
        {
            *blocked += dt;
        }
        return Ok(Some(target(None)));
    }
    let mut first_refusal = None;
    let mut chosen = None;
    for pair in ports.ranked_pairs(ship, station) {
        match corridors.plan(ship, &pair, clearance, forecast) {
            Ok(plan) => {
                chosen = Some((pair, plan));
                break;
            }
            Err(blocker) => {
                first_refusal.get_or_insert((pair, blocker));
            }
        }
    }
    match (chosen, first_refusal) {
        (Some((pair, plan)), _) => {
            if let Some(blocked) = blocked_s.filter(|_| holding) {
                debug!(
                    "autopilot_system: ship {ship:?} found a clear DOCK corridor to station \
                     {station:?} after holding {blocked:.1} s in all"
                );
            }
            set(autopilot, Some(plan), blocked_s);
            Ok(Some(target(Some(pair))))
        }
        (None, Some((pair, blocker))) => {
            let blocked = blocked_s.unwrap_or(0.0) + dt;
            if blocked >= DOCK_BLOCKED_SECS {
                warn!(
                    "autopilot_system: ship {ship:?} has no safe DOCK corridor to station \
                     {station:?} after holding {blocked:.1} s; the first-ranked pair {:?} -> {:?} \
                     is blocked by {}",
                    pair.first_section,
                    pair.second_section,
                    describe(&blocker),
                );
                return Err(DisengageType::NoSafeDockPath);
            }
            if holding {
                // Still holding: keep the corner a waiting hull flies clear
                // of the field by.
                if let AutopilotAction::DockApproach { blocked_s, .. } =
                    &mut autopilot.bypass_change_detection().action
                {
                    *blocked_s = Some(blocked);
                }
            } else {
                debug!(
                    "autopilot_system: ship {ship:?} holds for a clear DOCK corridor to station \
                     {station:?} ({blocked:.1} s held so far); the first-ranked pair {:?} -> {:?} \
                     is blocked by {}",
                    pair.first_section,
                    pair.second_section,
                    describe(&blocker),
                );
                set(autopilot, None, Some(blocked));
            }
            Ok(Some(target(None)))
        }
        // Every station port is held: wait on the kept selection.
        (None, None) => Ok(Some(target(None))),
    }
}

/// The one DOCK approach that flies to each station this tick, from every
/// driving hull's action. Two hulls flown into one station's approach space at
/// once foul each other's corridor and attitude: both overran in a live run.
/// An approach whose deadline is armed is already flying and keeps its turn;
/// otherwise the longest engaged takes it, then the lowest entity bits, so a
/// hull ordered later never takes the turn from one already waiting. The rest
/// wait at the station's velocity with their deadlines held, as on held
/// ports, and take a turn when it ends.
pub(super) fn dock_turns(
    approaches: impl Iterator<Item = (Entity, AutopilotAction)>,
) -> EntityHashMap<Entity> {
    let mut turns: EntityHashMap<(bool, f32, Entity)> = EntityHashMap::default();
    for (ship, action) in approaches {
        let AutopilotAction::DockApproach {
            station,
            deadline,
            engaged_s,
            ..
        } = action
        else {
            continue;
        };
        // Armed sorts first, then the longest engaged, then the lower bits.
        let rank = (deadline.is_none(), engaged_s, ship);
        let before = |a: &(bool, f32, Entity), b: &(bool, f32, Entity)| {
            a.0.cmp(&b.0)
                .then(b.1.total_cmp(&a.1))
                .then(a.2.to_bits().cmp(&b.2.to_bits()))
                .is_lt()
        };
        turns
            .entry(station)
            .and_modify(|best| {
                if before(&rank, best) {
                    *best = rank;
                }
            })
            .or_insert(rank);
    }
    turns
        .into_iter()
        .map(|(station, (_, _, ship))| (station, ship))
        .collect()
}

/// The field forecast a DOCK approach by `ship` to `station` reads at `now`,
/// as (body, centre, radius) in this tick's world frame: the held one while it
/// is for the same station and younger than [`DOCK_FORECAST_REFRESH`], a new
/// one stepped from `field` and `wells` otherwise. A body whose path stays
/// `margin` past the station's bubble, or past the hull where the hull stands
/// farther out, is left out. Empty
/// while the station is gone or unmeasured; the selection then refuses or
/// waits on its own. The flag says the forecast was stepped this tick: the
/// approach casts its corridors again on it.
fn dock_forecast(
    held: &mut EntityHashMap<DockForecast>,
    (ship, station): (Entity, Entity),
    now: f32,
    corridors: &DockCorridors,
    (field, wells, gravity): (&[FieldBody], &[WellSample], &GravitySettings),
    center_of_mass: Vec3,
    margin: f32,
) -> (Vec<(Entity, Vec3, f32)>, bool) {
    let (Some((station_center, bubble_radius)), Some(station_velocity)) = (
        corridors.bubble(station),
        corridors.ports.body_velocity(station),
    ) else {
        held.remove(&ship);
        return (Vec::new(), false);
    };
    let fresh = held.get(&ship).is_some_and(|forecast| {
        forecast.station == station && now - forecast.made_at < DOCK_FORECAST_REFRESH
    });
    if !fresh {
        let wells: Vec<(Entity, Vec3, GravityWell)> = wells
            .iter()
            .map(|sample| (sample.entity, sample.position, sample.well.clone()))
            .collect();
        let reach = center_of_mass.distance(station_center).max(bubble_radius) + margin;
        held.insert(
            ship,
            DockForecast {
                station,
                made_at: now,
                spheres: station_frame_sweep(
                    station_center,
                    station_velocity,
                    field,
                    &wells,
                    gravity,
                    reach,
                ),
            },
        );
    }
    (held[&ship].world(station_center), !fresh)
}

/// Flies every engaged autopilot one fixed tick: gathers each driving ship's
/// [`FlightBody`], [`FlightHelm`], [`ArrivalTarget`] and [`FlightEnvironment`],
/// runs [`autopilot_step`], then writes the helm, telemetry, completion and
/// disengagement back in that order.
/// How many times each live hull's maneuver has ended each way.
pub(super) type EndingTally = HashMap<(Entity, AutopilotEndType), u32>;

/// Count one more `end` of `ship`'s maneuver and give the tag its debug line
/// carries when this one is said: the first ending of that kind, then every
/// tenfold (` (x10)`, ` (x100)`, ...). A hull that is re-engaged and ends the
/// same way every few frames - a brain-dead wreck whose AI keeps handing it a
/// maneuver, a patrol closing leg after leg - then logs a handful of lines
/// over a session instead of one per engagement.
pub(super) fn logged_ending(
    endings: &mut EndingTally,
    ship: Entity,
    end: AutopilotEndType,
) -> Option<String> {
    let count = endings.entry((ship, end)).or_insert(0);
    *count += 1;
    let mut mantissa = *count;
    while mantissa % 10 == 0 {
        mantissa /= 10;
    }
    match (*count, mantissa) {
        (1, _) => Some(String::new()),
        (count, 1) => Some(format!(" (x{count})")),
        _ => None,
    }
}

pub(super) fn autopilot_system(
    time: Res<Time>,
    settings: Res<FlightSettings>,
    gravity_settings: Res<GravitySettings>,
    mut commands: Commands,
    mut q_ship: Query<
        (
            Entity,
            &mut Autopilot,
            &Position,
            &Rotation,
            (&LinearVelocity, &AngularVelocity),
            (&ComputedMass, &ComputedAngularInertia),
            Option<&ComputedCenterOfMass>,
            Option<&ManeuverTelemetry>,
            // The per-ship translation-arrival override (scenario-authored;
            // ships without it fly the global standoff), the per-ship detour
            // margin, and the per-ship cruise cap a convoy hull wears (ships
            // without it fly the arrival limit alone).
            (
                Option<&FlightArrivalStandoff>,
                Option<&FlightAvoidMargin>,
                Option<&FlightCruiseCap>,
            ),
            // This hull's own outer reach, so the arrival parks the FACE at the
            // margin rather than the origin. Absent (a hull with no live
            // sections measured yet) reads as a point.
            Option<&HullRadius>,
            // RCS terminal settle: the magazine the hand-off checks and the
            // intent the autopilot writes to hand the last-meters brake to the
            // torque-free RCS primitive.
            &RcsBudget,
            Option<&mut RcsIntent>,
            // A docked root flies the pair only while it drives it, and then
            // plans on the pair's mass, centre of mass, velocity and reach.
            // A suppressed partner's maneuver is frozen, not released.
            (Option<&DockedShip>, Option<&DockedAssembly>),
            // The camera's burn command: cleared every tick, set from this
            // tick's forward allocation.
            (Has<PlayerSpaceshipMarker>, Option<&mut MainDriveCommanded>),
        ),
        With<SpaceshipRootMarker>,
    >,
    mut q_thruster: HelmThrusterQuery,
    // The live flight computers ([`LiveFlightComputers`] says what makes one
    // live). Their acceleration limit is the hull's rotation authority - the
    // PHYSICAL half, kept apart from what the ship is permitted to do.
    q_computer: LiveFlightComputers,
    q_capabilities: ShipCapabilityQuery,
    mut q_rotation_input: HelmControllerQuery,
    q_target: ArrivalTargetQuery,
    q_wells: FlightWellQuery,
    q_obstacles: ObstacleQuery,
    // Docked drivers whose missing assembly is already logged, so the error
    // is said once per loss, not at the fixed rate.
    mut missing_assembly: Local<EntityHashSet>,
    // How many maneuvers each live hull has ended each way, so the ending's
    // debug line is said at the first and every tenfold, not once per
    // engagement.
    //
    // Each DOCK approach's held field forecast, stepped again every
    // DOCK_FORECAST_REFRESH seconds rather than every tick.
    (mut endings, mut forecasts): (Local<EndingTally>, Local<EntityHashMap<DockForecast>>),
    q_roots: Query<(), With<SpaceshipRootMarker>>,
    dock_corridors: DockCorridors,
) {
    let dt = time.delta_secs();

    let wells = well_samples(&q_wells, &q_target);
    // Only a driving hull's leg counts: a suppressed docked partner is frozen
    // below and never plans one.
    let obstacles = obstacle_field(
        &q_obstacles,
        q_ship
            .iter()
            .filter(|(.., (docked, _), _)| docked.is_none_or(|docked| docked.drives))
            .map(|(_, autopilot, ..)| autopilot.action),
    );

    missing_assembly.retain(|ship| {
        q_ship
            .get(*ship)
            .is_ok_and(|(.., (docked, assembly), _)| docked.is_some() && assembly.is_none())
    });
    endings.retain(|(ship, _), _| q_roots.contains(*ship));
    let dock_turns = dock_turns(
        q_ship
            .iter()
            .filter(|(.., (docked, _), _)| docked.is_none_or(|docked| docked.drives))
            .map(|(ship, autopilot, ..)| (ship, autopilot.action)),
    );
    forecasts.retain(|ship, _| {
        q_ship.get(*ship).is_ok_and(|(_, autopilot, ..)| {
            matches!(autopilot.action, AutopilotAction::DockApproach { .. })
        })
    });
    for (
        ship,
        mut autopilot,
        position,
        rotation,
        (velocity, angular_velocity),
        (mass, angular_inertia),
        com,
        prev_telemetry,
        (standoff_override, avoid_margin, cruise_cap),
        hull_radius,
        rcs_budget,
        mut rcs_intent,
        (docked, assembly),
        (is_player, mut commanded),
    ) in &mut q_ship
    {
        if let Some(commanded) = commanded.as_deref_mut() {
            commanded.0 = false;
        }
        match (docked, assembly) {
            (Some(docked), _) if !docked.drives => continue,
            (Some(_), None) => {
                if missing_assembly.insert(ship) {
                    error!(
                        "autopilot_system: docked driver {ship:?} has no DockedAssembly; \
                         flying nothing rather than on its root alone"
                    );
                }
                continue;
            }
            _ => {}
        }
        let body = flight_body(
            (position, rotation),
            (velocity, angular_velocity),
            (mass, angular_inertia),
            com,
            assembly,
            hull_radius,
        );
        let mut helm = flight_helm(
            ship,
            rcs_intent.as_deref(),
            rcs_budget,
            &q_thruster,
            &q_rotation_input,
            &q_computer,
        );
        let target = arrival_target(
            autopilot.action,
            &wells,
            &q_target,
            &gravity_settings,
            &settings,
        );
        let avoid_margin = resolved_avoid_margin(avoid_margin, &settings);
        let arrival_standoff = resolved_arrival_standoff(standoff_override, &settings);
        let (forecast, refreshed) = match autopilot.action {
            AutopilotAction::DockApproach { station, .. } => dock_forecast(
                &mut forecasts,
                (ship, station),
                time.elapsed_secs(),
                &dock_corridors,
                (&obstacles, &wells, &gravity_settings),
                body.center_of_mass,
                // The corridor entry stands within the bubble plus the
                // clearance, the hysteresis and the hull's arm, and the
                // refusal reads the clearance and the arm again; a corner
                // rounds a body by its clear clearance plus its arrival
                // window. A path that stays farther out meets neither.
                2.0 * (2.0 * body.mover_radius + avoid_margin + DETOUR_HYSTERESIS)
                    + arrival_standoff,
            ),
            _ => (Vec::new(), false),
        };
        let dock = match dock_approach_target(
            ship,
            &mut autopilot,
            &dock_corridors,
            (body.mover_radius + avoid_margin, &forecast, refreshed),
            (dock_turns.values().any(|&turn| turn == ship), dt),
        ) {
            Ok(dock) => dock,
            Err(reason) => {
                // A refused corridor fails its DOCK order and is warned with
                // its blocker where it was found.
                if let Some(times) =
                    logged_ending(&mut endings, ship, AutopilotEndType::Disengaged(reason))
                {
                    debug!("autopilot_system: ship {ship:?} DOCK approach ends: {reason:?}{times}");
                }
                commands.entity(ship).remove::<Autopilot>();
                continue;
            }
        };
        let env = FlightEnvironment {
            dt,
            settings: &settings,
            gravity: &gravity_settings,
            wells: &wells,
            arrival_standoff,
            avoid_margin,
            obstacles: &obstacles,
            cruise_cap: cruise_cap.map(|cap| **cap),
            capabilities: ship_capabilities(ship, &q_capabilities),
            dock,
        };
        let action = autopilot.action;
        let step = autopilot_step(
            autopilot.bypass_change_detection(),
            &body,
            &mut helm,
            target.as_ref(),
            prev_telemetry,
            &env,
        );
        // Only a flown tick (the phase) and a park (the new ORBIT) write
        // Autopilot; a hold, a completion and a disengagement leave its change
        // tick alone.
        if matches!(step.end, None | Some(AutopilotEndType::ParkedIntoOrbit)) && !step.held {
            autopilot.set_changed();
        }

        if let (
            AutopilotAction::DockApproach {
                station,
                deadline: before,
                settled: was_settled,
                ..
            },
            AutopilotAction::DockApproach {
                deadline: Some(deadline),
                settled,
                ..
            },
        ) = (action, autopilot.action)
        {
            let event = match (before, was_settled, settled) {
                (None, _, _) => Some("armed"),
                (Some(_), false, true) => Some("settled"),
                _ => None,
            };
            if let Some(event) = event {
                debug!(
                    "autopilot_system: ship {ship:?} {event} its DOCK approach to station \
                     {station:?}: {deadline:.1} s left, {:.1} u from it, {:.2} u/s off its velocity",
                    dock.map_or(f32::NAN, |dock| dock.station_bubble.0.distance(body.center_of_mass)),
                    dock.map_or(f32::NAN, |dock| (body.linear_velocity - dock.station_velocity)
                        .length()),
                );
            }
        }
        if let Some(gravity) = step.brake_degraded {
            debug!(
                "autopilot_system: well pull {gravity} exceeds brake authority on the arrival \
                 leg of {ship:?}; no stopping plan"
            );
        }
        if let Some(AutopilotEndType::Disengaged(reason)) = step.end {
            // Every overrun fails its DOCK order, so each one is said with its
            // metrics: the tally would hide a retried ship's later overruns.
            if let (DisengageType::DockDeadline, AutopilotAction::DockApproach { station, .. }) =
                (reason, action)
            {
                let pair = dock.and_then(|dock| dock.pair);
                warn!(
                    "autopilot_system: ship {ship:?} overran its DOCK approach to station \
                     {station:?}: gap {:?} u, axes {:?} deg off opposed, relative speed {:?} \
                     u/s, relative spin {:?} deg/s, station spin {:?} deg/s; disengaging",
                    pair.map(|pair| pair.gap),
                    pair.map(|pair| (-pair.opposition).clamp(-1.0, 1.0).acos().to_degrees()),
                    pair.map(|pair| pair.relative_speed),
                    pair.map(|pair| pair.relative_spin.to_degrees()),
                    dock.map(|dock| dock.station_spin.length().to_degrees()),
                );
            } else if let Some(times) =
                logged_ending(&mut endings, ship, AutopilotEndType::Disengaged(reason))
            {
                match (reason, action) {
                    (DisengageType::NoFlightComputer, _) => {
                        debug!(
                            "autopilot_system: ship {ship:?} lost its flight computer, \
                             disengaging{times}"
                        );
                    }
                    (DisengageType::NoLiveEngines, _) => {
                        debug!(
                            "autopilot_system: ship {ship:?} has no live engines, \
                             disengaging{times}"
                        );
                    }
                    (DisengageType::TargetGone, AutopilotAction::Goto { target }) => {
                        debug!(
                            "autopilot_system: GOTO target {target:?} is gone, disengaging{times}"
                        );
                    }
                    (DisengageType::WellGone, AutopilotAction::Orbit { well, .. }) => {
                        debug!("autopilot_system: ORBIT well {well:?} is gone, disengaging{times}");
                    }
                    (DisengageType::NoStableBand, AutopilotAction::Orbit { well, .. }) => {
                        debug!(
                            "autopilot_system: well {well:?} has no stable band, disengaging \
                             ORBIT{times}"
                        );
                    }
                    (DisengageType::NoSafeLeg, action) => {
                        error!(
                            "autopilot_system: ship {ship:?} {action:?} planned a non-finite \
                             goal against the sized bodies on its leg; the leg is not flown \
                             and the autopilot disengages{times}"
                        );
                    }
                    (reason, action) => {
                        debug!(
                            "autopilot_system: ship {ship:?} {action:?} disengaged: \
                             {reason:?}{times}"
                        );
                    }
                }
            }
            commands.entity(ship).remove::<Autopilot>();
            continue;
        }
        if step.held {
            continue;
        }

        // Keep the published telemetry in step with the engaged verb: GOTO and
        // moving STOP legs update it every tick; ORBIT and a settled STOP clear
        // a stale one (disengage clears via remove_maneuver_telemetry).
        match step.telemetry {
            Some(numbers) => {
                commands.entity(ship).try_insert(numbers);
            }
            None if prev_telemetry.is_some() => {
                commands.entity(ship).remove::<ManeuverTelemetry>();
            }
            None => {}
        }
        // The step writes a zero intent whenever it does not use the RCS, so
        // a ship without the component only needs one once it does.
        if let Some(intent) = rcs_intent.as_deref_mut() {
            intent.0 = helm.rcs_intent;
        } else if helm.rcs_intent != Vec3::ZERO {
            commands.entity(ship).insert(RcsIntent(helm.rcs_intent));
        }
        match step.end {
            Some(AutopilotEndType::ParkedIntoOrbit) => {
                if let AutopilotAction::Orbit {
                    plan: Some(plan), ..
                } = autopilot.action
                {
                    debug!(
                        "autopilot_system: ship {ship:?} arrived, parking into ORBIT at ring {}",
                        plan.radius
                    );
                }
                if is_player {
                    commands
                        .entity(ship)
                        .insert(PlayerAutopilotCompleted { action });
                }
                continue;
            }
            Some(AutopilotEndType::Completed) => {
                if let Some(times) = logged_ending(&mut endings, ship, AutopilotEndType::Completed)
                {
                    debug!("autopilot_system: ship {ship:?} maneuver complete, disengaging{times}");
                }
                if is_player {
                    commands
                        .entity(ship)
                        .insert(PlayerAutopilotCompleted { action });
                }
                commands.entity(ship).remove::<Autopilot>();
                continue;
            }
            Some(AutopilotEndType::Disengaged(_)) | None => {}
        }

        for controller in &helm.controllers {
            if let Ok((_, mut input, _)) = q_rotation_input.get_mut(controller.entity) {
                **input = controller.command;
            }
        }
        if let Some(commanded) = commanded.as_deref_mut() {
            commanded.0 = step.main_drive_commanded;
        }
        for engine in &helm.engines {
            if let Ok((_, mut input, ..)) = q_thruster.get_mut(engine.entity) {
                **input = engine.input;
            }
        }
    }
}

/// The autopilot. One rule flies every maneuver: compute the desired velocity
/// for the goal, rotate the *cheapest engine group* onto the velocity error
/// (rotation time * bias + burn time; the nose is nothing special), and fire
/// every engine currently inside the alignment cone. The flip-and-burn
/// emerges when the main drive is worth turning for; a retro or lateral
/// group handles what it already points at. Ends the maneuver
/// ([`AutopilotEndType`]) when the goal is reached, the target is gone, the
/// ship has no engines, or the flight computer (live controller section) is
/// lost.
/// Off-center engine torque is balanced at the source by the wrench allocation
/// ([`balance_throttles`], using each engine's lever arm about the live COM):
/// differential throttle within the firing set when it has headroom,
/// recruiting off-axis engines (laterals, retros), never one opposing the burn,
/// for pure counter-torque when it does not - at the price of a bounded
/// sideways drift the arrival control corrects. The PD holds whatever residual
/// the allocation cannot null.
///
/// One fixed tick for one mover: reads `body`, `target`, `previous` and `env`,
/// advances `autopilot` and the actuator state in `helm`, and reports the
/// published numbers and how the maneuver ended. A zero-mass `body` holds the
/// leg: `helm` and `autopilot` are untouched, `previous` is returned and
/// `held` is set.
pub(super) fn autopilot_step(
    autopilot: &mut Autopilot,
    body: &FlightBody,
    helm: &mut FlightHelm,
    target: Option<&ArrivalTarget>,
    previous: Option<&ManeuverTelemetry>,
    env: &FlightEnvironment,
) -> AutopilotStep {
    let settings = env.settings;
    let gravity_settings = env.gravity;
    let dt = env.dt;
    let arrival_standoff = env.arrival_standoff;
    let cruise_cap = env.cruise_cap;
    let rotation = Rotation(body.rotation);
    let com_world = body.center_of_mass;
    let velocity = body.linear_velocity;
    let mass = body.mass;
    let mover_radius = body.mover_radius;
    let has_telemetry = previous.is_some();
    let disengaged = |reason| AutopilotStep {
        telemetry: None,
        main_drive_commanded: false,
        end: Some(AutopilotEndType::Disengaged(reason)),
        brake_degraded: None,
        held: false,
    };

    // No flight computer, no autopilot - the ship is adrift on manual.
    let live_pds = || helm.controllers.iter().filter_map(|c| c.live_pd.as_ref());
    let Some(turn_rate) =
        ship_turn_rate(live_pds().map(|pd| pd.max_angular_acceleration), settings)
    else {
        return disengaged(DisengageType::NoFlightComputer);
    };
    // How far behind a turning command the hull trails, for the arrival
    // plan: the slowest loop's, when several disagree.
    let tracking_lag = live_pds()
        .map(|pd| pd.tracking_lag())
        .fold(0.0f32, f32::max);

    // The velocity error the leg treats as a crumb. Legs that END AT REST
    // (STOP, GOTO, GotoPos) use the wider settle band: the endgame of a
    // translation leg lives in sub-u/s errors - the brake tail, the
    // boundary creep, the doorstep residual - and chasing those with
    // attitude swings was the "wobbles on GOTO" playtest. Scoping by LEG,
    // not by desired == 0, is deliberate: the hunt's onset is in the brake
    // tail where desired is still nonzero - the desired-zero scoping was
    // tried and left the terminal spin bit-for-bit unchanged. Only ORBIT
    // keeps the tight band: station-keeping is the one regime whose job is
    // chasing small errors forever. The band is also the RCS settle's
    // full deflection, so it is chosen before the settle.
    let crumb_band = match autopilot.action {
        AutopilotAction::Orbit { .. }
        | AutopilotAction::MatchVelocity { .. }
        | AutopilotAction::DockApproach { .. } => settings.attitude_deadband,
        _ => settings.settle_deadband.max(settings.attitude_deadband),
    };

    // Every live engine as (world thrust direction, magnitude), plus how
    // hot the hottest one runs (for the settle check). A section's local
    // Transform is its fixed attitude on the hull; engines do not gimbal.
    // Off-center engine torque is balanced below by the wrench allocation
    // (per-engine lever arms about the live COM, off-axis engines
    // recruited for counter-torque when the firing set cannot balance
    // itself); whatever the allocation cannot null the PD still holds
    // within its cap.
    let mut engines: Vec<(Vec3, f32)> = Vec::new();
    let mut hottest_input = 0.0f32;
    for engine in &helm.engines {
        // An engine whose AUTHORED rotation names no direction is not an
        // engine: see `engine_direction`. Skipped rather than pushed as a
        // NaN, which would poison the group scores, the allocation and the
        // spool tail so the burn never completed. A hull with no valid
        // engine LEFT takes the no-live-engines disengagement below.
        let Some(dir) = engine_direction(&rotation, &engine.mount) else {
            continue;
        };
        engines.push((dir, engine.magnitude));
        hottest_input = hottest_input.max(engine.input);
    }
    if engines.is_empty() {
        return disengaged(DisengageType::NoLiveEngines);
    }
    // Avian publishes a root's mass a tick after its colliders land, and
    // every plan below divides by it: a massless hull reads zero brake
    // authority, which collapses the arrival leg's desired velocity to
    // zero, which is the same shape as "the goal wants rest here and the
    // ship is at rest" - so the release test completes a leg that never
    // flew. The hull is still being assembled, not arrived: hold the leg
    // and wait for the mass rather than commanding or releasing on it.
    if mass <= 0.0 {
        return AutopilotStep {
            telemetry: previous.copied(),
            main_drive_commanded: false,
            end: None,
            brake_degraded: None,
            held: true,
        };
    }
    let groups = cluster_thrusters(&engines, FORWARD_ALIGNMENT_COS);

    // The arrival curve is planned with the group the computer would
    // actually brake with: its authority sets the deceleration, its
    // rotation distance sets the lead (a retro-equipped ship brakes late
    // and flat; a main-drive-only ship budgets its 180) at the full turn
    // rate, which is the rate the flip below is flown at, plus the hull's
    // settle onto the brake attitude behind that command (unbudgeted, it
    // ate the spool pad and the block gunship crossed its standoff at 45
    // m/s) and the spool pad. Shared by GOTO and ORBIT's ring correction.
    let braking_plan = |brake_dir: Vec3, brake_speed: f32| -> (f32, f32) {
        let brake = choose_group(
            &groups,
            brake_dir,
            brake_speed,
            mass,
            dt,
            turn_rate,
            settings.rotation_bias,
        );
        let (brake_authority, brake_angle) = brake
            .map(|g| (g.authority, g.world_dir.angle_between(brake_dir)))
            .unwrap_or((0.0, 0.0));
        let accel = if dt > 0.0 && mass > 0.0 {
            (brake_authority / mass) / dt
        } else {
            0.0
        };
        let lead = flip_lead(
            brake_angle,
            turn_rate,
            tracking_lag,
            settings.align_cos,
            settings.arrival_spool_pad,
        );
        (accel, lead)
    };

    // ORBIT plans once, on its first engaged tick: target ring from the
    // current radius clamped into the stable band, plane from r x v with
    // the ship-up fallback. The plan then stays sticky - replanning
    // every tick would chase the drift the plan exists to correct.
    if let AutopilotAction::Orbit { well, plan: None } = autopilot.action {
        let Some(sample) = env.wells.iter().find(|sample| sample.entity == well) else {
            return disengaged(DisengageType::WellGone);
        };
        let r_vec = com_world - sample.position;
        let Some(radius) = orbit_target_radius(
            r_vec.length(),
            &sample.band_well,
            gravity_settings,
            settings,
        ) else {
            return disengaged(DisengageType::NoStableBand);
        };
        let plan = OrbitPlan {
            radius,
            normal: orbit_plane_normal(r_vec, velocity, rotation.mul_vec3(Vec3::Y)),
        };
        autopilot.action = AutopilotAction::Orbit {
            well,
            plan: Some(plan),
        };
    }

    // The total well pull fighting a leg that rests at `rest_point` while
    // closing along `closing_dir`, in u/s^2: the sum of every well's
    // positive along-track component (overlapping SOIs add up; a pull that
    // helps braking is ignored, never banked). Evaluated at the rest point
    // - the worst point of a monotonic inward leg. Scanning every well
    // (they are few) instead of the ship's DominantWell matters: the flip
    // is usually planned from OUTSIDE the SOI, where the ship has no
    // DominantWell yet but the goal is already deep in one.
    let gravity_along = |rest_point: Vec3, closing_dir: Vec3| -> f32 {
        env.wells
            .iter()
            .map(|sample| {
                let well = &sample.well;
                let offset = sample.position - rest_point;
                let pull = well_accel(
                    well.mu,
                    offset.length(),
                    well.body_radius,
                    well.soi_radius,
                    gravity_settings.fade_fraction,
                    gravity_settings.surface_margin,
                );
                (offset.normalize_or_zero() * pull)
                    .dot(closing_dir)
                    .max(0.0)
            })
            .sum()
    };

    // The arrival leg shared by GOTO and GotoPos. ONE model:
    //
    //     centre distance = target radius + mover radius + margin
    //
    // so a big body is given its size, this hull is given its own, and the
    // margin is the GAP between the two surfaces - which is what makes an
    // authored zero mean "face on the mark" rather than "origin on the
    // mark". `floor` raises that centre distance for a well-bearing target
    // to the ORBIT band's own floor, so the ring the handoff plans is the
    // ring the leg already parked on. Published distances are the gap.
    let arrival_desired =
        |goal: Vec3, target_radius: f32, floor: f32| -> (Vec3, ManeuverTelemetry, Option<f32>) {
            let radii = target_radius.max(0.0) + mover_radius;
            let standoff = (radii + arrival_standoff).max(floor);
            let to_target = goal - com_world;
            let distance = to_target.length();
            // Zero only if the ship sits exactly on the goal center; the
            // else branch below has distance > standoff > 0, so there the
            // fallback never engages.
            let closing_dir = to_target.normalize_or_zero();
            let closing_speed = velocity.dot(closing_dir);
            // Where the leg rests: the standoff boundary on the closing
            // line. Capped at the ship's own distance so at or inside the
            // envelope it degenerates to the ship position - the computer
            // stops there, it never flies back out to the boundary.
            let park_point = goal - closing_dir * standoff.min(distance);
            if distance <= standoff {
                (
                    Vec3::ZERO,
                    ManeuverTelemetry {
                        goal,
                        goal_entity: None,
                        park_point,
                        distance: (distance - radii).max(0.0),
                        closing_speed,
                        braking: false,
                        brake_accel: 0.0,
                        flip_point: None,
                        seconds_to_flip: None,
                        eta: None,
                    },
                    None,
                )
            } else {
                let brake_dir = -closing_dir;
                let brake_speed = velocity.length().max(settings.min_approach_speed);
                let (accel, lead) = braking_plan(brake_dir, brake_speed);
                let gravity = gravity_along(goal - closing_dir * standoff, closing_dir);
                // The published deceleration is the effective one, so any
                // instrument reading it sees the plan the computer actually
                // flies (the field is currently write-only in the HUD).
                // Zero means the pull exceeds the brake authority: no
                // stopping plan (flip/eta are None and the desired velocity
                // is zero - brake flat out).
                let brake_accel = (accel * settings.decel_margin - gravity).max(0.0);
                // Once per degradation entry, not per tick: the previous
                // published plan still had brake authority.
                let degraded = (brake_accel <= 0.0 && previous.is_none_or(|t| t.brake_accel > 0.0))
                    .then_some(gravity);
                let flip = goto_flip_point(
                    distance,
                    closing_speed,
                    accel * settings.decel_margin,
                    lead,
                    standoff,
                    gravity,
                );
                let eta = arrival_eta(
                    distance,
                    closing_speed,
                    accel * settings.decel_margin,
                    lead,
                    standoff,
                    gravity,
                );
                (
                    goto_desired_velocity(
                        to_target,
                        standoff,
                        accel,
                        settings.decel_margin,
                        lead,
                        settings.min_approach_speed,
                        gravity,
                        cruise_cap,
                    ),
                    ManeuverTelemetry {
                        goal,
                        goal_entity: None,
                        park_point,
                        distance: (distance - radii).max(0.0),
                        closing_speed,
                        braking: flip == FlipEstimate::Braking,
                        brake_accel,
                        flip_point: match flip {
                            FlipEstimate::Ahead { from_goal, .. } => {
                                Some(goal - closing_dir * from_goal)
                            }
                            FlipEstimate::Braking | FlipEstimate::Unknown => None,
                        },
                        seconds_to_flip: match flip {
                            FlipEstimate::Ahead { seconds, .. } => Some(seconds),
                            FlipEstimate::Braking | FlipEstimate::Unknown => None,
                        },
                        eta,
                    },
                    degraded,
                )
            }
        };

    // The goal, as a desired velocity right now. GOTO and STOP legs
    // also publish their live numbers as [`ManeuverTelemetry`] for the
    // HUD instruments; ORBIT (and a settled STOP) clears it.
    let mut telemetry: Option<ManeuverTelemetry> = None;
    // Set by the Goto arm when the ship is inside the park envelope;
    // gates the ORBIT handoff in the done branch.
    let mut goto_arrived = false;
    // Set by the Orbit arm: gates the error-relative RCS trim, which only
    // applies while station-keeping - the desired is a fast orbital
    // velocity, not a rest goal.
    let mut is_orbit = false;
    // Set by the MatchVelocity and DockApproach arms: the hull axis to
    // hold on a world direction while the velocity is held (the nose on
    // the caller's facing, the docking port against the station's), and
    // the flag that says this action's desired velocity is a STANDING one
    // rather than a goal to arrive at.
    let mut hold_axis: Option<(Vec3, Vec3)> = None;
    let mut is_velocity_hold = false;
    // Cleared by a MatchVelocity that leaves every push to the drive.
    let mut rcs_permitted = true;
    // Set by the arrival leg when it loses its stopping plan this tick; the
    // system logs the edge.
    let mut brake_degraded = None;
    // The strongest pull the ship feels right now - the same shaped
    // `well_accel` the physics applies, not a raw `mu/r^2`, so the SOI fade
    // and the surface clamp are already in it. Both RCS branches are gated
    // on it: an RCS that cannot out-push the local well must not be handed
    // a goal it will lose. Scanning every well (they are few) rather than
    // reading `DominantWell` keeps this correct for a ship that has not
    // been assigned one yet.
    //
    // A DOCK approach flies in its station's frame, and every station root is
    // a dynamic ship root the same wells pull. Its velocity error does not see
    // the pull both bodies share, only the difference across the gap; gated
    // on the whole pull, a hull could never trim on its RCS at an orbiting
    // station.
    let local_gravity_accel = match (autopilot.action, env.dock) {
        (AutopilotAction::DockApproach { .. }, Some(dock)) => relative_gravity_accel(
            env.wells,
            com_world,
            dock.station_bubble.0,
            gravity_settings,
        ),
        _ => local_gravity_accel(env.wells, com_world, gravity_settings),
    };
    let desired = match autopilot.action {
        AutopilotAction::Stop => {
            // STOP has a spatial goal too: the predicted rest point.
            // Publish it so the instruments (readout chip, trajectory
            // ribbon) cover the braking leg; near rest there is no leg
            // left and the telemetry clears. Hysteresis on the gate:
            // a ship hovering at the threshold (gravity re-accelerating
            // it, engines still winding down) must not strobe the
            // instruments, so a leg starts at twice the epsilon and
            // holds until the epsilon itself.
            let speed = velocity.length();
            let publish = if has_telemetry {
                speed > settings.stop_speed_epsilon
            } else {
                speed > 2.0 * settings.stop_speed_epsilon
            };
            if publish {
                let brake_dir = -velocity.normalize();
                // The plan's group choice floors the speed at
                // min_approach_speed (a lead planned for a crawling
                // ship is meaningless); the rest distance itself uses
                // the raw speed - slight overestimate at low speed,
                // documented asymmetry.
                let (accel, lead) = braking_plan(brake_dir, speed.max(settings.min_approach_speed));
                // STOP's pull budget is evaluated at the ship, not the
                // (yet-unknown) rest point - honest enough for a
                // telemetry-only prediction, and the leg replans every
                // tick anyway.
                let gravity = gravity_along(com_world, velocity.normalize());
                let effective = (accel * settings.decel_margin - gravity).max(0.0);
                if let Some(rest) =
                    stop_rest_distance(speed, accel * settings.decel_margin, lead, gravity)
                {
                    let goal = com_world + velocity.normalize() * rest;
                    telemetry = Some(ManeuverTelemetry {
                        goal,
                        goal_entity: None,
                        // A STOP has no standoff: the predicted rest
                        // point IS the park point.
                        park_point: goal,
                        distance: rest,
                        closing_speed: speed,
                        // A STOP is the brake; there is no coast phase to
                        // be ahead of.
                        braking: true,
                        brake_accel: effective,
                        flip_point: None,
                        seconds_to_flip: None,
                        eta: Some(lead + (speed + gravity * lead) / effective.max(1e-3)),
                    });
                }
            }
            Vec3::ZERO
        }
        AutopilotAction::Goto { .. } | AutopilotAction::GotoPos { .. } => {
            let Some(target) = target else {
                return disengaged(DisengageType::TargetGone);
            };
            // The leg is planned around every sized body but its own target
            // before it is flown. The clearance is this hull's arm plus the
            // daylight it wants past its skin, and a corner is let go only
            // inside the window the leg would otherwise park in, so the
            // computer never comes to rest on a corner.
            let policy = DetourPolicy {
                clearance: mover_radius + env.avoid_margin,
                hull: mover_radius,
                hysteresis: DETOUR_HYSTERESIS,
                arrive_radius: mover_radius + arrival_standoff + DETOUR_HYSTERESIS,
            };
            let obstacles: Vec<(Vec3, f32)> = env
                .obstacles
                .iter()
                .filter(|body| Some(body.entity) != target.entity)
                .map(|body| (body.center, body.radius))
                .collect();
            let plan = plan_leg(com_world, target.goal, autopilot.detour, policy, &obstacles);
            // A plan that is not finite was planned against a body that is
            // not finite. Flying it would fly nothing sane, so the leg ends
            // here and the system says so.
            if !plan.goal.is_finite() {
                return disengaged(DisengageType::NoSafeLeg);
            }
            autopilot.detour = plan.corner;
            // A corner is a mark of its own: no size, no well floor, no
            // entity for the readout to follow.
            let (radius, floor) = match plan.corner {
                Some(_) => (0.0, 0.0),
                None => (target.radius, target.floor),
            };
            let (desired, mut numbers, degraded) = arrival_desired(plan.goal, radius, floor);
            brake_degraded = degraded;
            // Arrived means INSIDE the park envelope, not merely
            // "wants zero velocity": the degraded no-stopping-plan
            // state also zeroes the desired velocity arbitrarily far
            // out, and a done-at-apex there must release (as it
            // always did), never park into an orbit whose ring
            // correction assumes it starts near the ring. The
            // published distance is the hull-to-surface gap, so the
            // envelope is that gap at rest: the margin, or more where
            // the band floor pushed the leg out. A leg still flying a
            // corner has not arrived anywhere.
            let park_gap =
                (radius + mover_radius + arrival_standoff).max(floor) - radius - mover_radius;
            goto_arrived = plan.corner.is_none() && numbers.distance <= park_gap;
            // Braking toward a corner is not the leg's flip: the flip the
            // instruments mark is the final leg's, and a corner must not
            // spend it.
            autopilot.flip_marker_consumed |= numbers.braking && plan.corner.is_none();
            numbers.goal_entity = if plan.corner.is_none() {
                target.entity
            } else {
                None
            };
            telemetry = Some(numbers);
            desired
        }
        AutopilotAction::MatchVelocity {
            velocity,
            facing,
            rcs,
        } => {
            // Nothing to plan and nothing to arrive at: the goal IS the
            // desired velocity, and the whole of the leg is the error
            // below. No telemetry either - the instruments read a leg
            // with a destination, and this has none.
            is_velocity_hold = true;
            hold_axis = facing.map(|facing| (Vec3::NEG_Z, Vec3::from(facing)));
            rcs_permitted = rcs;
            velocity
        }
        AutopilotAction::DockApproach { .. } => {
            let Some(dock) = env.dock else {
                return disengaged(DisengageType::TargetGone);
            };
            is_velocity_hold = true;
            let policy = DetourPolicy {
                clearance: mover_radius + env.avoid_margin,
                hull: mover_radius,
                hysteresis: DETOUR_HYSTERESIS,
                arrive_radius: mover_radius + arrival_standoff + DETOUR_HYSTERESIS,
            };
            // Leg A flies in the station's frame: the field enters as its
            // forecast paths relative to the station, which stand still in
            // that frame, beside the station's own bubble.
            let forecast: Vec<(Vec3, f32)> = dock
                .forecast
                .iter()
                .map(|&(_, center, radius)| (center, radius))
                .collect();
            let obstacles: Vec<(Vec3, f32)> = forecast
                .iter()
                .copied()
                .chain([dock.station_bubble])
                .collect();
            match dock.pair {
                // No pair to fly: move with the station and wait, clear of
                // every forecast path. A point that stands still in the
                // station's frame is crossed by a planet the station orbits
                // closely once per orbit, so a hull waiting on one flies out
                // of it round the station. Inside the station's bubble the
                // hull holds the station's point under it, so a spinning
                // station's plating does not sweep into it.
                None => {
                    let clear = goal_outside(com_world, policy.clear_clearance(), &forecast);
                    if clear == com_world {
                        autopilot.detour = None;
                        let (center, radius) = dock.station_bubble;
                        if com_world.distance(center) <= radius + policy.clearance {
                            dock.frame_velocity(com_world)
                        } else {
                            dock.station_velocity
                        }
                    } else {
                        let held = autopilot
                            .detour
                            .map(|corner| corner + dock.station_velocity * dt);
                        let leg = search_leg(com_world, clear, held, policy, &obstacles);
                        if !leg.plan.goal.is_finite() {
                            return disengaged(DisengageType::NoSafeLeg);
                        }
                        autopilot.detour = leg.plan.corner;
                        dock.station_velocity + dock_closing_velocity(leg.plan.goal - com_world)
                    }
                }
                Some(pair) => {
                    let AutopilotAction::DockApproach {
                        plan: Some(plan), ..
                    } = autopilot.action
                    else {
                        unreachable!("a graded DOCK pair is a planned one");
                    };
                    let axis = pair.second.axis;
                    let berth_gap = dock_berth_gap(&pair);
                    let entry_face = pair.second.face + axis * plan.entry;
                    let from_face = pair.first.face - pair.second.face;
                    let along = from_face.dot(axis);
                    let lateral = (from_face - axis * along).length();
                    // Leg B: inside the swept tube, the port closes along the
                    // axis to the berth while the axes oppose and it sits on
                    // the axis, and backs out along it to the entry while
                    // either gate fails. The leg planner is never asked from
                    // inside: from within the station's bubble it corners
                    // radially out of the centre, across whatever plating
                    // lies that way. Outside the tube, leg A routes the
                    // centre of mass around the station and every sized body
                    // to the entry.
                    let in_tube = (0.0..=plan.entry + DOCK_FINAL_LEG_LATERAL).contains(&along)
                        && lateral < plan.lateral;
                    let (offset, to_entry, closing, frame) = if in_tube {
                        autopilot.detour = None;
                        let aligned =
                            (-pair.opposition).clamp(-1.0, 1.0).acos() < DOCK_FINAL_LEG_ANGLE;
                        let closing = aligned && lateral < DOCK_FINAL_LEG_LATERAL;
                        let gap = if closing { berth_gap } else { plan.entry };
                        (
                            pair.second.face + axis * gap - pair.first.face,
                            entry_face.distance(pair.first.face),
                            closing,
                            // The tube turns with the station.
                            dock.frame_velocity(com_world),
                        )
                    } else {
                        let goal = com_world + (entry_face - pair.first.face);
                        // The command below is station velocity plus closing, and
                        // every body above stands still in the station's frame, so
                        // a corner moves with the station.
                        let held = autopilot
                            .detour
                            .map(|corner| corner + dock.station_velocity * dt);
                        // The entry is fixed by the port and kept clear of every
                        // path by the selection; it is flown as given.
                        let leg = search_leg(com_world, goal, held, policy, &obstacles);
                        if !leg.plan.goal.is_finite() {
                            return disengaged(DisengageType::NoSafeLeg);
                        }
                        autopilot.detour = leg.plan.corner;
                        // A held corner's length runs straight on past it; the
                        // settle re-arm below reads the whole route a fresh
                        // search plans, which may take a second corner.
                        let settling = matches!(
                            autopilot.action,
                            AutopilotAction::DockApproach { settled: false, .. }
                        ) && (velocity - dock.station_velocity).length()
                            <= DOCK_SETTLE_SPEED;
                        let routed = if settling && held.is_some() {
                            search_leg(com_world, goal, None, policy, &obstacles)
                                .length
                                .max(leg.length)
                        } else {
                            leg.length
                        };
                        // A corner stands in the station's translating
                        // frame, where the forecast stands still; the entry
                        // turns with the station, so a leg flown straight to
                        // it leads it.
                        let frame = match leg.plan.corner {
                            Some(_) => dock.station_velocity,
                            None => dock.frame_velocity(goal),
                        };
                        (leg.plan.goal - com_world, routed, false, frame)
                    };
                    let AutopilotAction::DockApproach {
                        deadline, settled, ..
                    } = &mut autopilot.action
                    else {
                        unreachable!("the arm matched a DOCK approach");
                    };
                    let corridor = plan.entry - berth_gap;
                    let left = deadline.get_or_insert_with(|| {
                        dock_approach_deadline(to_entry, corridor, turn_rate)
                    });
                    // Once, on the first tick the hull flies near the station's
                    // velocity: the leg it plans from there is the one the
                    // closing law flies. A hull that never settles keeps the
                    // deadline it was armed with.
                    if !*settled && (velocity - dock.station_velocity).length() <= DOCK_SETTLE_SPEED
                    {
                        *settled = true;
                        *left = left.max(dock_approach_deadline(to_entry, corridor, turn_rate));
                    }
                    *left -= dt;
                    if *left <= 0.0 {
                        return disengaged(DisengageType::DockDeadline);
                    }
                    hold_axis =
                        Some((body.rotation.inverse() * pair.first.axis, -pair.second.axis));
                    frame
                        + if closing {
                            dock_final_close_velocity(offset)
                        } else {
                            dock_closing_velocity(offset)
                        }
                }
            }
        }
        AutopilotAction::Orbit { well, plan } => {
            let Some(sample) = env.wells.iter().find(|sample| sample.entity == well) else {
                return disengaged(DisengageType::WellGone);
            };
            let Some(plan) = plan else {
                unreachable!("the ORBIT plan block above fills the plan or disengages");
            };
            is_orbit = true;
            let r_vec = com_world - sample.position;
            let to_ring = orbit_ring_offset(r_vec, &plan);
            let brake_dir = -to_ring
                .try_normalize()
                .unwrap_or_else(|| -r_vec.normalize_or(Vec3::X));
            let brake_speed = velocity.length().max(settings.min_approach_speed);
            let (accel, lead) = braking_plan(brake_dir, brake_speed);
            orbit_desired_velocity(
                r_vec,
                &plan,
                sample.well.mu,
                accel,
                settings.decel_margin,
                lead,
            )
        }
    };

    let error = desired - velocity;
    let error_speed = error.length();
    let error_dir = (error_speed > 1e-3).then(|| error / error_speed);

    // Two RCS branches hand the burn to the torque-free RCS COM push and
    // spool the main drive down. They share one command formula
    // (proportional toward `desired`) and differ in what must be below
    // `rcs_handoff_speed`:
    //
    // - SETTLE: the maneuver's GOAL is rest (STOP, GOTO/GotoPos inside
    //   the standoff - `desired ~= 0`), so the hull's own speed must be
    //   below the hand-off and RCS brakes the last meters to rest.
    // - TRIM: ORBIT station-keeping or a held velocity, where `desired` is
    //   a standing velocity the ship holds. The RESIDUAL
    //   `error = desired - v` must be below the hand-off, whatever the
    //   absolute speed; while it is above (spinning up, a big ring
    //   correction), the main drive does the work.
    //
    // Both need the ship's `rcs_enabled` capability, so a hull without it
    // keeps the exact main-drive behavior, and delta-v left in the
    // magazine: an empty [`RcsBudget`] hands the goal back to the main
    // drive until it refills to [`RCS_RESUME_FRACTION`]. A held velocity
    // that refuses the RCS
    // (`MatchVelocity::rcs`) flies as a hull without it.
    let capabilities = env.capabilities;
    let rcs_granted = capabilities.rcs_enabled && rcs_permitted;
    let rcs_capable = rcs_granted
        && rcs_magazine_ready(&helm.rcs_budget, settings.rcs_recovery_delay)
        && error_speed > 1e-3;
    // The RCS takes a goal only where it has CLEAR authority over the local
    // gravity: its `rcs_accel` push must comfortably exceed the inward
    // pull, or a perturbed ship falls faster than RCS can correct - the
    // menu ambience ships crashing the asteroid. Where it does not, the
    // main drive (full authority) keeps the goal, exactly as it did before
    // the RCS branches existed.
    //
    // The gate is the SAME on both branches, and for the same reason. A
    // STOP inside a well is the worse case of the two: `desired` is zero
    // for the whole descent, so an ungated settle latches the moment the
    // ship is under the hand-off speed and then parks at the equilibrium
    // where the proportional push equals the pull - a steady fall, with the
    // drive cooled and `done` never firing.
    let rcs_has_gravity_authority =
        rcs_has_gravity_authority(local_gravity_accel, settings.rcs_accel);
    let use_rcs_settle = rcs_capable
        && rcs_has_gravity_authority
        && desired.length() <= settings.stop_speed_epsilon
        && velocity.length() < settings.rcs_handoff_speed;
    // The error-relative trim: the desired velocity is one the ship HOLDS,
    // so what must be below the hand-off is the residual, not the absolute
    // speed.
    // Shared by ORBIT's ring trim and a held velocity, which are the same
    // problem - a standing goal the RCS corrects around.
    let use_rcs_trim = rcs_capable
        && (is_orbit || is_velocity_hold)
        && rcs_has_gravity_authority
        && error_speed < settings.rcs_handoff_speed;
    let use_rcs = use_rcs_settle || use_rcs_trim;
    // The residual that counts as full deflection - the band the branch
    // must hold to, never the hand-off speed. A proportional law brakes
    // with a time constant of `scale / rcs_accel`, and coasts that long
    // again in distance: scaled to 100 m/s the settle took two seconds to
    // kill each m/s it was handed, and a GOTO crossing its standoff at
    // approach speed parked 30 m inside it (an entry at 40 m/s, 80 m).
    // Scaled to the crumb band the settle brakes at full deflection down
    // to the crumbs the drive left it and fades inside them. An ORBIT trim
    // holds a band against the well's STANDING inward pull, and parks at
    // an offset proportional to its scale - referenced to 100 m/s that
    // offset was wider than the hold band itself, so the trim never
    // reported Hold. The band the orbit must hold to is its scale.
    let rcs_scale = if use_rcs_trim && is_orbit {
        settings.orbit_hold_enter
    } else {
        crumb_band
    };
    // Proportional command toward `desired`, scaled so a scale-sized
    // residual is full deflection; fades to zero as the residual does (no
    // overshoot). Clear to zero when not using RCS so a stale nudge never
    // lingers.
    let rcs_command = if use_rcs {
        (rotation.inverse() * error / rcs_scale).clamp(Vec3::splat(-1.0), Vec3::splat(1.0))
    } else {
        Vec3::ZERO
    };
    helm.rcs_intent = rcs_command;

    // The allocation set: EVERY live engine, with the coefficients the
    // balancer needs per unit input - signed thrust along the burn, force
    // perpendicular to it, and lever-arm torque about the live COM. The
    // engines inside the alignment cone of the needed burn are the
    // *primary* set (lit engines keep a slightly looser gate - hysteresis
    // via their own spooled input - so the plume does not flicker at the
    // boundary): they define the deliverable authority and receive the
    // demand. Everything else - laterals, retros - except an engine
    // opposing the burn is a counter-torque candidate the balancer may
    // recruit when the primary set cannot balance itself (the single
    // damage-shifted main drive).
    let mut firing_authority = 0.0f32;
    let mut allocation: Vec<(usize, BalanceEngine)> = Vec::new();
    // Per allocation entry: world thrust direction, primary flag and
    // spooled input, for hold_for_opposed_wind_down.
    let mut engine_dirs: Vec<(Vec3, bool)> = Vec::new();
    let mut spooled: Vec<f32> = Vec::new();
    if let Some(error_dir) = error_dir {
        for (index, engine) in helm.engines.iter().enumerate() {
            let Some(dir) = engine_direction(&rotation, &engine.mount) else {
                continue;
            };
            let gate = if engine.input > 0.1 {
                settings.align_cos - settings.align_hysteresis
            } else {
                settings.align_cos
            };
            let aligned = dir.dot(error_dir);
            let primary = aligned >= gate;
            if primary {
                firing_authority += engine.magnitude;
            }
            // World point of the engine (direct child of the root): raw
            // root pose composed with the local mount, and
            // thruster_impulse_system pushes from this SAME composition -
            // the lever arm about com_world matches the torque physics
            // applies by construction, never through a render-clock
            // GlobalTransform.
            let pos_world = body.position + rotation.mul_vec3(engine.mount.translation);
            // An engine opposing the burn is never a recruit: its
            // counter-torque would cancel the thrust the burn pays for.
            // It stays in the allocation, dark, so it still spools down
            // and holds the burn through its tail.
            let opposed = !primary && aligned <= -settings.align_cos;
            let torque = if opposed {
                Vec3::ZERO
            } else {
                (pos_world - com_world).cross(dir * engine.magnitude)
            };
            // A recruit's whole thrust vector is off-plan force (see
            // BalanceEngine); a primary engine contributes its aligned
            // share to the demand and only the perpendicular rest to the
            // penalty.
            let (forward, lateral) = if primary {
                (
                    engine.magnitude * aligned,
                    (dir - aligned * error_dir) * engine.magnitude,
                )
            } else if opposed {
                (0.0, Vec3::ZERO)
            } else {
                (0.0, dir * engine.magnitude)
            };
            allocation.push((
                index,
                BalanceEngine {
                    forward,
                    lateral,
                    torque,
                    primary,
                },
            ));
            engine_dirs.push((dir, primary));
            spooled.push(engine.input);
        }
    }

    // Within the crumb band (chosen with the plan above) the leftover is a
    // crumb: never re-aim the hull for it - any engine already on the
    // error finishes it, and a residual only a rotation could remove is
    // accepted. This is what stops the ship twitching after perfection.
    //
    // A brake that still OWES more than a crumb is never a crumb itself. A
    // leg that ends at rest and is past its flip point is committed to the
    // brake, and the tick's error there starts at zero (the ship is on the
    // curve) and grows only as fast as the curve falls - a band's worth of
    // it is half a second at speed, and half a second of coast at 82 m/s
    // is 40 m of park point. So the plan says when the brake is due (past
    // the flip point, still closing, a stopping plan in hand) and the band
    // does not judge the TICK's error there. It does judge what the brake
    // owes: below the band that leftover is the residual `settle_deadband`
    // accepts, and a STOP publishes a brake for the whole of its life, so
    // without the floor the last band of every stop is chased to
    // `stop_speed_epsilon` with the attitude swings the deadband exists to
    // prevent - worst where the RCS verb is withheld and the settle falls
    // back to the main drive. The brake leg holds one attitude: the group
    // the plan chose, against the velocity the leg owes - the whole of it,
    // lateral included, the way STOP brakes. Not the tick's error: at the
    // flip point that is a crumb pointing anywhere, and down the burn it
    // is the lateral crumb the drive itself leaves while the hull
    // settles - aimed at that, a single drive swings off the brake to
    // chase it, leaves the retro error to grow, swings back, and saws down
    // the whole burn at 0.4 rad/s. Not the closing line either: a burn
    // along the line leaves the lateral crumb alone, and the line turns
    // under a ship passing its mark, so the crumb grows into a sideways
    // entry. The drive fires only when the error is in its cone (below),
    // so a ship under the curve coasts instead of flipping prograde, and
    // the doorstep settle kills what is left, torque-free. The flip (the
    // brake group not yet facing the burn) turns at the full rate the plan
    // budgeted the lead with.
    let brake = telemetry.and_then(|numbers| {
        // PAST the flip point, not merely "no flip point published". The
        // leg publishes none for three reasons and only this one is a
        // brake; see `FlipEstimate`.
        let owed = velocity.length();
        let due = numbers.braking
            && numbers.brake_accel > 0.0
            && numbers.closing_speed > settings.stop_speed_epsilon
            && owed > crumb_band;
        let brake_dir = -velocity.normalize_or_zero();
        (due && brake_dir != Vec3::ZERO).then(|| (brake_dir, owed.max(settings.min_approach_speed)))
    });
    let flip_pending = brake.is_some_and(|(brake_dir, brake_speed)| {
        choose_group(
            &groups,
            brake_dir,
            brake_speed,
            mass,
            dt,
            turn_rate,
            settings.rotation_bias,
        )
        .is_some_and(|group| group.world_dir.dot(brake_dir) < settings.align_cos)
    });
    let fine = error_speed <= crumb_band && brake.is_none();

    // Done: the goal wants rest here and the ship is at rest - exactly,
    // or within the deadband with no engine on the residual. ORBIT never
    // completes: an orbit is not a destination, the computer
    // station-keeps until breakout, Z, or a capability loss.
    let done = !matches!(
        autopilot.action,
        AutopilotAction::Orbit { .. }
            | AutopilotAction::MatchVelocity { .. }
            | AutopilotAction::DockApproach { .. }
    ) && autopilot.detour.is_none()
        && desired == Vec3::ZERO
        && (error_speed <= settings.stop_speed_epsilon || (fine && firing_authority <= 0.0));
    // Release only once every actuator has wound down. A still-hot,
    // spooling-down drive would push the ship off again. An RCS settle
    // still pushing at more than a crumb of deflection has not rested
    // yet: it has no spool tail and fades with the residual, so the rest
    // it hands back in free space is a fraction of the drive's epsilon,
    // not the whole of it (released at the epsilon, the block gunship
    // drifted half a hull length while its escort was still arriving).
    // Inside a well the settle converges on the deflection that holds
    // the standing pull and never below it, so that hold - with room for
    // the approach - is the release there: the ship is handed back
    // falling as slowly as the RCS could make it, never held forever.
    let rcs_rested = !use_rcs_settle
        || rcs_command.length()
            <= RCS_RELEASE_DEFLECTION.max(2.0 * local_gravity_accel / settings.rcs_accel);
    // A drive holding the ship against a well the RCS cannot is not a
    // spool tail: released, its wind-down hands the ship to the pull it
    // was holding and delivers nothing. The rest at such a well is a
    // hover, and the throttle that hover takes is allowed on release.
    let hover_input = error_dir
        .filter(|_| firing_authority > 0.0)
        .map_or(0.0, |error_dir| {
            gravity_along(com_world, -error_dir) * mass * dt / firing_authority
        });
    if done && hottest_input <= 0.05 + hover_input && rcs_rested {
        // A GOTO that arrived at a well body parks into orbit instead of
        // handing back a ship that immediately starts falling: the one-key
        // parking flow becomes zero-key when the computer was already told
        // where to go. engage() resets the phase. The ring is planned HERE,
        // from the leg's intent - the SAME centre distance the arrival flew,
        // resolved margin and this hull's own radius included - never from
        // wherever terminal creep dragged the ship: a plan-from-current-
        // radius could ring at the band bottom, and the insertion from a
        // crept position has been seen to graze the rock. max with the
        // current radius so a ship that settled slightly outside the park
        // point is not corrected inward. Because the arrival already floors
        // itself at the band floor, this cannot burn the ship outward to a
        // ring it was never told to fly.
        // The park is ORBIT, so it is the ORBIT VERB that decides whether
        // the computer may fly it: a controller withholding ORBIT (the
        // training range, until its lesson) hands the ship back at the
        // standoff instead of flying a maneuver the pilot has not been
        // given. Either way the LEG completed - it arrived - so the
        // completion is reported before the park, and a scenario waiting on
        // the arrival hears it whether or not the computer parks.
        // Breakout semantics (any flight input, Z) are ORBIT's own,
        // unchanged. Everything else - GotoPos, well-less targets, STOP, a
        // withheld verb, a bandless well - releases as before.
        if let AutopilotAction::Goto { target } = autopilot.action {
            if goto_arrived && capabilities.orbit_enabled {
                if let Some(sample) = env.wells.iter().find(|sample| sample.entity == target) {
                    let well = &sample.band_well;
                    let r_vec = com_world - sample.position;
                    let park = well.body_radius + mover_radius + arrival_standoff;
                    if let Some(radius) = orbit_target_radius(
                        park.max(r_vec.length()),
                        well,
                        gravity_settings,
                        settings,
                    ) {
                        *autopilot = Autopilot::engage(AutopilotAction::Orbit {
                            well: target,
                            plan: Some(OrbitPlan {
                                radius,
                                normal: orbit_plane_normal(
                                    r_vec,
                                    velocity,
                                    rotation.mul_vec3(Vec3::Y),
                                ),
                            }),
                        });
                        return AutopilotStep {
                            telemetry,
                            main_drive_commanded: false,
                            end: Some(AutopilotEndType::ParkedIntoOrbit),
                            brake_degraded,
                            held: false,
                        };
                    }
                }
            }
        }
        return AutopilotStep {
            telemetry,
            main_drive_commanded: false,
            end: Some(AutopilotEndType::Completed),
            brake_degraded,
            held: false,
        };
    }

    // Rotate the cheapest group onto the error (only for corrections worth
    // turning for), then allocate the shared burn demand across the whole
    // live engine set as a torque-nulling throttle vector. While settling
    // (done, engines still winding down) command zero to every engine.
    let mut throttles: Vec<f32> = vec![0.0; allocation.len()];
    let mut burning = false;
    // The axis the caller asked to hold, held whenever the burn is NOT
    // asking for the hull: an RCS trim pushes through the COM in any
    // direction, a crumb is not worth turning for, and an error too small
    // to have a direction is not a burn at all. The moment the error
    // outgrows all three the rotation below takes the hull for the burn,
    // and this gets it back when the burn is done.
    //
    // OUTSIDE the burn block on purpose: a ship exactly on its commanded
    // velocity has no error to steer by, and that is precisely when the
    // nose should be sitting where it was asked to sit.
    let holding_axis = hold_axis.filter(|_| fine || use_rcs || error_dir.is_none());
    if let Some((axis, facing)) = holding_axis {
        // Never the full rate: the facing is a request the hull settles
        // onto, not a flip the arrival plan budgeted a lead for. The floor
        // keeps a hull that is already on its velocity from creeping onto
        // the facing at a rate that reads as drift.
        let max_step = turn_rate * dt * slew_urgency(error_speed, crumb_band).max(0.25);
        for controller in &mut helm.controllers {
            let command = controller.command;
            let command_dir = command.mul_vec3(axis);
            let goal = Quat::from_rotation_arc(command_dir, facing) * command;
            controller.command = slew_rotation(command, goal, max_step);
        }
    }
    if let (Some(error_dir), false) = (error_dir, done) {
        // RCS pushes through the center of mass in any direction, so it
        // never needs an attitude change. Keep the current helm bearing
        // while RCS handles a low-speed STOP, terminal settle, or orbit
        // trim; only a main-drive correction chooses and faces an engine.
        // The brake leg aims the plan's brake group at the plan's brake
        // direction for the speed the plan chose it with, so execution
        // and plan agree on which engine flies the arrival; every other
        // rotation aims at this tick's error.
        let (aim_dir, burn_ahead) = brake.unwrap_or((error_dir, error_speed));
        if holding_axis.is_none() && !fine && !use_rcs {
            if let Some(chosen) = choose_group(
                &groups,
                aim_dir,
                burn_ahead,
                mass,
                dt,
                turn_rate,
                settings.rotation_bias,
            ) {
                // The command evolves from ITS OWN previous state, never
                // from the hull: rotate the command so it carries the
                // chosen group onto the burn, slewed at the estimated turn
                // rate (see slew_rotation - a 180 step would drive the PD
                // into undamped saturation). Anchoring to the command
                // instead of the hull also regulates roll: a command
                // rebuilt from the hull each tick inherits the hull's roll,
                // the PD then sees zero roll error, and roll picked up
                // during a flip spins the ship like a drill forever.
                let local_dir = rotation.inverse().mul_vec3(chosen.world_dir);
                // Turn gently when little burn remains: the ending turn is
                // what the hull is still spinning with at release, and a
                // slow final swing keeps that residual under
                // RELEASE_SPIN_EPSILON. Scaled by the same regime-scoped
                // crumb band as `fine`: on a rest leg the brake tail's
                // few-u/s corrections must swing the hull GENTLY or each
                // re-aim overshoots and seeds the next (the arrival hunt
                // cascade). NOTE: the deadband A/B moved this
                // denominator together with the band - keying only the
                // band left the terminal spin unchanged. The planned
                // flip is not a correction: it turns at the full rate
                // the plan budgeted its lead with (keyed to the tick's
                // error, which is a crumb at the flip point, it started
                // at a quarter rate and the block gunship parked 165 m
                // inside its point from 82 m/s).
                let urgency = if flip_pending {
                    1.0
                } else {
                    slew_urgency(error_speed, crumb_band)
                };
                let max_step = turn_rate * dt * urgency;
                for controller in &mut helm.controllers {
                    let command = controller.command;
                    let command_dir = command.mul_vec3(local_dir);
                    let goal = Quat::from_rotation_arc(command_dir, aim_dir) * command;
                    controller.command = slew_rotation(command, goal, max_step);
                }
            }
        }
        // The shared demand this tick: the impulse the maneuver wants,
        // capped by the firing set's authority (burn_input * authority =
        // min(impulse, authority)). balance_throttles delivers it through
        // the firing set and nulls the net torque about the COM,
        // recruiting off-axis engines when the firing set cannot.
        //
        // Spool-tail cutoff for legs ending at rest: a throttle commanded
        // to zero winds down exponentially (see `spool`) and still
        // delivers magnitude * input / (spool_down_rate * dt) of impulse
        // on the way, so a burn that keeps demanding until the error
        // reads zero integrates THROUGH zero. At the finish the ship
        // exits its own standoff backwards, the re-entry error re-aims
        // the hull, and the arrival bounces on the boundary in a limit
        // cycle (previously masked by the accidental dither of the
        // cross-clock command handoff). On the way out, the accelerating
        // burn carries the ship over the arrival curve by the same tail -
        // several m/s on a hot drive - and every one of them is brake
        // distance the plan never budgeted. Once the wind-down tail alone
        // covers the remaining error, the correct demand is zero: cut and
        // coast onto the curve, or to rest. Only those two burns land on
        // a velocity the ship then keeps. The brake burn rides a curve
        // that keeps falling: cut there, the drive chatters on and off
        // around the tail (every relight re-aimed the hull, a 0.35 rad/s
        // sawtooth down the whole brake) - it modulates instead. ORBIT's
        // desired velocity is a ring speed to hold, never a curve to
        // coast onto. Inside a well the tail is net of the pull against
        // the burn: the part of the throttle that is holding the ship up
        // delivers no velocity when cut, it hands the ship to the well.
        // Counted whole, a hover's tail read above the rest epsilon and
        // the cutoff chopped the hover into a fall-and-relight cycle that
        // never rested.
        let lands = desired == Vec3::ZERO || error.dot(velocity) > 0.0;
        let mut tail_dv = 0.0;
        if lands && !is_orbit && dt > 0.0 && mass > 0.0 {
            let mut push = 0.0;
            for engine in &helm.engines {
                let Some(dir) = engine_direction(&rotation, &engine.mount) else {
                    continue;
                };
                push += dir.dot(error_dir).max(0.0) * engine.magnitude * engine.input / dt / mass;
            }
            tail_dv = spool_tail(
                push,
                gravity_along(com_world, -error_dir),
                settings.spool_down_rate,
            );
        }
        let demand = if use_rcs {
            // The RCS COM push is braking the last meters; the main drive
            // spools down so the two never double-push.
            0.0
        } else if lands && !is_orbit && error_speed <= tail_dv {
            0.0
        } else {
            firing_authority * burn_input(error_speed * mass, firing_authority)
        };
        let coeffs: Vec<BalanceEngine> = allocation.iter().map(|(_, e)| *e).collect();
        throttles = balance_throttles(&coeffs, demand);
        // A retro raised while the main still winds down (or the reverse)
        // burns against the tail; hold every engine until it is dark.
        if hold_for_opposed_wind_down(&engine_dirs, &spooled, &throttles, settings.align_cos) {
            throttles.fill(0.0);
        }
        burning = throttles.iter().any(|&u| u > 0.0);
    }

    autopilot.phase = match autopilot.action {
        // ORBIT reports Hold once the velocity error is inside the hold
        // tolerance, with hysteresis so the label does not flicker at
        // the boundary. Micro-burns still fire inside Hold (the
        // attitude deadband, not the hold gate, decides burning) - that
        // IS station-keeping.
        AutopilotAction::Orbit { .. } => {
            let holding = if autopilot.phase == AutopilotPhase::Hold {
                error_speed <= settings.orbit_hold_exit
            } else {
                error_speed <= settings.orbit_hold_enter
            };
            if holding {
                AutopilotPhase::Hold
            } else if burning {
                AutopilotPhase::Burn
            } else {
                AutopilotPhase::Align
            }
        }
        // A held velocity inside the band IS the maneuver working, the
        // same way a trimmed orbit is. Reporting Align there would say the
        // computer is still getting ready for something it is already doing.
        AutopilotAction::MatchVelocity { .. } | AutopilotAction::DockApproach { .. }
            if error_speed <= crumb_band =>
        {
            AutopilotPhase::Hold
        }
        _ if burning => AutopilotPhase::Burn,
        _ => AutopilotPhase::Align,
    };

    // A main-drive burn needs a firing forward engine: `primary` alone
    // also marks a retro brake's set, and forward alone also marks a main
    // drive recruited for counter-torque.
    let main_drive_commanded =
        allocation
            .iter()
            .zip(&throttles)
            .any(|(&(index, engine), &throttle)| {
                engine.primary
                    && throttle > 0.0
                    && engine_direction_local(&helm.engines[index].mount)
                        .is_some_and(|dir| is_forward_aligned(dir, Vec3::NEG_Z))
            });

    // Spool every engine toward its allocated throttle (zero for engines
    // the allocation left dark, and for everything while settling).
    let mut targets = vec![0.0; helm.engines.len()];
    for (&(index, _), &throttle) in allocation.iter().zip(&throttles) {
        targets[index] = throttle;
    }
    for (engine, target) in helm.engines.iter_mut().zip(targets) {
        engine.input = spool(
            engine.input,
            target,
            settings.spool_up_rate,
            settings.spool_down_rate,
            dt,
        );
    }

    AutopilotStep {
        telemetry,
        main_drive_commanded,
        end: None,
        brake_degraded,
        held: false,
    }
}

/// When the autopilot lets go - completion or any breakout - it cools the
/// engines it was driving and parks the helm on the hull's current attitude.
/// Nothing else writes a *bound* thruster's input between key events (the
/// manual burn system deliberately leaves bound thrusters to their own
/// keys), so a residual autopilot burn would otherwise ghost on forever; and
/// a rotation command abandoned mid-maneuver can sit ~180 degrees from the
/// hull, which parks the saturated PD in its degenerate zone where it
/// sustains a perpetual roll instead of damping the leftover spin.
pub(super) fn on_autopilot_removed_cool_engines(
    remove: On<Remove, Autopilot>,
    q_ship: Query<&Rotation, With<SpaceshipRootMarker>>,
    mut q_thruster: Query<(&mut ThrusterSectionInput, &ChildOf), With<ThrusterSectionMarker>>,
    mut q_rotation_input: Query<
        (&mut ControllerSectionRotationInput, &ChildOf),
        With<ControllerSectionMarker>,
    >,
    mut q_rcs: Query<&mut RcsIntent>,
) {
    for (mut input, &ChildOf(parent)) in &mut q_thruster {
        if parent == remove.entity {
            **input = 0.0;
        }
    }
    // Clear the RCS command: the autopilot writes it while settling or
    // trimming, and rcs_burn_system acts on ANY non-zero intent regardless of
    // autopilot state. A residual intent would push the ship off its rest and
    // drain the magazine.
    if let Ok(mut intent) = q_rcs.get_mut(remove.entity) {
        intent.0 = Vec3::ZERO;
    }
    if let Ok(rotation) = q_ship.get(remove.entity) {
        for (mut input, &ChildOf(parent)) in &mut q_rotation_input {
            if parent == remove.entity {
                **input = rotation.0;
            }
        }
    }
}
