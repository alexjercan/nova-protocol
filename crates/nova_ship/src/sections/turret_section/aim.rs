//! Turret aiming: the lead-intercept solve and the per-frame hinge pass that
//! swings each articulated joint toward the aim point.
//!
//! Engine units throughout: a hull span comes off an avian collider and an
//! intercept is solved against a `LinearVelocity`, so every length here is a
//! world unit - one of which is 10 m - and every speed is a world unit per
//! second. Authored fields the cone is graded against are quoted in the meters
//! a creator reads in the content file.

use avian3d::prelude::*;
use bevy::prelude::*;

use super::{firing::MUZZLE_SPREAD_RAD, *};
use crate::physics::prelude::rigid_body_point_velocity;

/// System set for the FixedUpdate aim chain (target track, intercept solve and
/// joint targets).
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct TurretSectionAimSystems;

/// How big the place a crosshair marks is, in world units: 16 m.
///
/// A commanded POINT has no size of its own - the player swung the camera and
/// the mounts follow the ray - so it is graded as a mark of about the beam of
/// a small warship. Purely a firing-discipline figure: it sets how long a
/// manually aimed turret holds fire, and it is the half-beam the turret's
/// TRACK constant is graded against.
pub const POINT_AIM_RADIUS: f32 = 1.6;

/// The CLOSE edge of a gunfight, in world units: 100 u is 1 km, and gunfights
/// are fought at 1-2 km, so this is the range the commanded-point gate below
/// is graded at.
pub const CLOSE_ENGAGEMENT_RANGE: f32 = 100.0;

/// How far off a barrel may point at a commanded POINT and still count as ON
/// it: 0.016 rad, 0.92 degrees.
///
/// The fallback gate, for the one case with no body to measure: a mount
/// following the player's crosshair into empty space. It is the same
/// arithmetic [`on_target_cone`] does, evaluated once on a mark of
/// [`POINT_AIM_RADIUS`] at [`CLOSE_ENGAGEMENT_RANGE`] - the LOOSEST angle that
/// still means "this round lands on the place you pointed at". At this size an
/// angle and its tangent agree to within 2e-6 rad, so the RATIO is the angle
/// and the constant needs no transcendental to state.
///
/// A mount aimed at a BODY is not graded on this. It is graded on how big that
/// body is at the range it is at, which is the whole point of
/// [`on_target_cone`]: one gate for every gun was 0.92 deg of slack against a
/// torpedo it should have been threading and 0.92 deg of held fire against a
/// carrier it could not miss.
pub const POINT_AIM_ON_TARGET_RAD: f32 = POINT_AIM_RADIUS / CLOSE_ENGAGEMENT_RANGE;

/// The widest a barrel may be off its aim point and still put the round on
/// what it was aimed at: `atan(hit_radius / distance)`.
///
/// A round leaving the muzzle `e` off the aim point misses that point
/// laterally by `distance * tan(e)`, so the widest error that still lands
/// inside a target of `hit_radius` is exactly this. It OPENS as the target
/// grows and TIGHTENS as the range does, which is what a gunner does: a
/// carrier at knife range fills the sky and a torpedo across a gunfight is a
/// needle.
///
/// Two bounds. `None` - no body, a commanded point - is
/// [`POINT_AIM_ON_TARGET_RAD`]. And no gate is ever tighter than
/// [`FIRE_GATE_FLOOR_RAD`], the best a settled barrel can point plus its
/// scatter: a gate inside it holds fire on a barrel that has stopped
/// correcting, for good.
pub fn on_target_cone(hit_radius: Option<f32>, distance: f32) -> f32 {
    let Some(hit_radius) = hit_radius else {
        return POINT_AIM_ON_TARGET_RAD;
    };
    (hit_radius.max(0.0) / distance.max(f32::EPSILON))
        .atan()
        .max(FIRE_GATE_FLOOR_RAD)
}

/// The widest a muzzle may sit off its aim point once its hinges have
/// settled: about 0.32 deg.
///
/// Each hinge HOLDS once its own correction is inside [`AIM_DEADBAND_RAD`],
/// so it can rest anywhere inside that band. A turret's yaw and pitch hinges
/// turn the muzzle in square planes, so their residuals add in quadrature.
const AIM_SETTLED_RAD: f32 = AIM_DEADBAND_RAD * std::f32::consts::SQRT_2;

/// The tightest any fire gate gets: [`AIM_SETTLED_RAD`] plus
/// [`MUZZLE_SPREAD_RAD`], about 0.42 deg.
///
/// A small mount at long range is a smaller angle than a settled barrel can
/// hold: a 4 m turret mount at 2 km is 0.002 rad, half the hinge deadband. A
/// gate graded there never opens, and the turret holds fire with the trigger
/// down. At this floor a round still lands within 15 m of the mark at 2 km.
const FIRE_GATE_FLOOR_RAD: f32 = AIM_SETTLED_RAD + MUZZLE_SPREAD_RAD;

/// The angle (radians) between where a muzzle POINTS and where it should point
/// to put a round on `aim`. Zero for a degenerate line (the aim point sitting on
/// the muzzle): there is no bearing to be wrong about.
///
/// `forward` need not be normalized; `muzzle` and `aim` are world-space points.
pub fn muzzle_aim_error(forward: Vec3, muzzle: Vec3, aim: Vec3) -> f32 {
    let (Some(forward), Some(to_aim)) = (forward.try_normalize(), (aim - muzzle).try_normalize())
    else {
        return 0.0;
    };
    forward.dot(to_aim).clamp(-1.0, 1.0).acos()
}

/// Whether a muzzle at `muzzle` pointing along `forward` is on `aim` within
/// the cone [`on_target_cone`] grades a target of `hit_radius` at that range -
/// the single "may this barrel shoot" predicate, used by the section fire path
/// and by the AI trigger.
///
/// This is the whole reachability rule as well as the alignment one: a mount
/// whose hinges cannot swing onto its target never converges, so it is never
/// within the cone, so it never fires. [`TurretSectionArc`] stays the ACQUIRE-
/// time question (never assign a mount a target it cannot reach); this is the
/// fire-time one, and it needs no second reachability test to answer.
pub fn muzzle_on_target(forward: Vec3, muzzle: Vec3, aim: Vec3, hit_radius: Option<f32>) -> bool {
    muzzle_aim_error(forward, muzzle, aim) <= on_target_cone(hit_radius, muzzle.distance(aim))
}

/// The point a turret should aim at to hit a maneuvering target: the intercept
/// point, where a round leaving the muzzle at `projectile_speed` meets a target
/// at `target` moving at `target_vel` and accelerating at `target_accel`.
/// Solves `|(target - shooter) + target_vel*t + target_accel*t^2/2| =
/// projectile_speed*t` for the smallest positive `t` no later than `max_flight`
/// and returns where the target is at that `t`.
///
/// The frames. A round inherits the muzzle velocity on launch and then flies
/// straight, so the solve runs in the frame of the muzzle AT LAUNCH: the caller
/// passes the target velocity RELATIVE to the muzzle, and the target's own WORLD
/// acceleration. The shooter's acceleration after the round leaves moves the
/// shooter, never the round, so it is not part of the lead.
///
/// The quartic has no closed form worth carrying, so the solve brackets its first
/// sign change on [`INTERCEPT_SCAN_STEPS`] samples of the round's flight and
/// bisects it. A round expires at `max_flight`, so an intercept after it is no
/// intercept: with no root inside the flight - a target that out-accelerates the
/// round, or one beyond its reach - the solve falls back to
/// [`velocity_lead_point`], which keeps a sane aim on a target the gun cannot
/// reach yet. A target with no acceleration takes that exact closed form.
///
/// `pub(crate)` because it is the solution the torpedo's terminal weave exists
/// to break: the weave measurement in `torpedo_section` scores itself against
/// THIS solve rather than a copy of it that could drift away from what turrets
/// actually fire on.
pub(crate) fn lead_intercept_point(
    shooter: Vec3,
    target: Vec3,
    target_vel: Vec3,
    target_accel: Vec3,
    projectile_speed: f32,
    max_flight: f32,
) -> Vec3 {
    if target_accel == Vec3::ZERO || !(max_flight > 0.0 && max_flight.is_finite()) {
        return velocity_lead_point(shooter, target, target_vel, projectile_speed);
    }
    let to_target = target - shooter;
    let half_accel = 0.5 * target_accel;
    let at = |t: f32| to_target + target_vel * t + half_accel * t * t;
    // Positive while the target stands outside the sphere a round has covered
    // by `t`, and zero where the two meet.
    let gap = |t: f32| at(t).length_squared() - (projectile_speed * t).powi(2);

    let step = max_flight / INTERCEPT_SCAN_STEPS as f32;
    let bracket = (1..=INTERCEPT_SCAN_STEPS)
        .map(|k| step * k as f32)
        .find(|&t| gap(t) <= 0.0);
    let Some(mut late) = bracket else {
        return velocity_lead_point(shooter, target, target_vel, projectile_speed);
    };
    let mut early = late - step;
    for _ in 0..INTERCEPT_BISECTIONS {
        let mid = 0.5 * (early + late);
        if gap(mid) > 0.0 {
            early = mid;
        } else {
            late = mid;
        }
    }
    shooter + at(late)
}

/// Samples of the round's flight [`lead_intercept_point`] brackets its root on.
/// At the PDC's 2 s flight one sample is 62 ms, 62 m of round travel: two roots
/// inside one sample are a target grazing the round's reach sphere, which a
/// barrel cannot thread anyway.
const INTERCEPT_SCAN_STEPS: u32 = 32;

/// Bisections of the bracketing sample. 24 halvings of 62 ms are finer than an
/// `f32` flight time near one second can hold.
const INTERCEPT_BISECTIONS: u32 = 24;

/// The constant-velocity intercept: [`lead_intercept_point`] with no
/// acceleration, in closed form. Solves `|(target - shooter) + target_vel*t| =
/// projectile_speed*t` for the smallest positive `t` and returns
/// `target + target_vel*t`.
///
/// Falls back to the target's current position when there is no valid intercept -
/// a target with no relative motion resolves to the target itself, and a target
/// too fast to catch (or receding faster than the bullet) has no positive solution,
/// so the turret simply aims where the target is now.
fn velocity_lead_point(
    shooter: Vec3,
    target: Vec3,
    target_vel: Vec3,
    projectile_speed: f32,
) -> Vec3 {
    let to_target = target - shooter;
    // a*t^2 + b*t + c = 0
    let a = target_vel.length_squared() - projectile_speed * projectile_speed;
    let b = 2.0 * to_target.dot(target_vel);
    let c = to_target.length_squared();

    let time_to_intercept = if a.abs() < 1e-4 {
        // Target speed ~ bullet speed: the equation is linear in t.
        (b.abs() > 1e-6).then(|| -c / b)
    } else {
        let discriminant = b * b - 4.0 * a * c;
        (discriminant >= 0.0)
            .then(|| {
                let sqrt_d = discriminant.sqrt();
                [(-b + sqrt_d) / (2.0 * a), (-b - sqrt_d) / (2.0 * a)]
                    .into_iter()
                    .filter(|&t| t > 0.0)
                    .reduce(f32::min)
            })
            .flatten()
    };

    match time_to_intercept {
        Some(t) if t > 0.0 && t.is_finite() => target + target_vel * t,
        _ => target,
    }
}

/// Time constant (seconds) of the target-velocity track: the exponential
/// low-pass every mount runs over the sample its aim feed leaves in
/// [`TurretSectionTargetVelocity`], before [`lead_intercept_point`]
/// extrapolates it over a round's whole flight.
///
/// DERIVED, not chosen. A one-pole filter following a target under constant
/// lateral acceleration `a` settles exactly `a * tau` short of the true
/// velocity, so the intercept it feeds lands `a * tau * flight` short of the
/// target. Holding that inside a hull - the same [`HULL_HIT_RADIUS`] the
/// on-target cone is drawn against, over the same
/// `CLOSE_ENGAGEMENT_RANGE / muzzle_speed` flight time the cone is graded at -
/// is the bound, and
/// [`the_track_constant_holds_a_serpents_lead_inside_a_hull`] re-derives it
/// from the live numbers rather than from these words.
///
/// The acceleration it is drawn against is the shipped Serpent's, out of its
/// own field docs: the terminal weave holds the nose `weave_angle` off the
/// guidance solution and spins that tilt at `weave_rate`, so the lateral
/// acceleration no straight-line lead can predict is
/// `max_speed * sin(weave_angle) * weave_rate`. The bound is LINEAR in that
/// number, so it is set by the hardest-turning thing point defence is ever
/// pointed at - a small body under continuous thrust - and a ship, which turns
/// its whole hull to change course, is inside it with room to spare.
///
/// The floor under it is the fixed step. At 64 Hz this is five ticks, so one
/// tick's spike enters the track at 18% of its size and is gone within three
/// more; shorter, and the barrel is handed back the per-tick jitter that stops
/// it ever settling inside the fire gate.
///
/// The acceleration lead takes the steady `a * tau` shortfall back out: the
/// solve leads `velocity + acceleration * tau` (see
/// [`TurretSectionTargetTrack::lead`]). The bound above still governs whatever
/// that estimate gets wrong - a weave turning its acceleration faster than
/// [`TARGET_ACCEL_TAU`] follows it.
const TARGET_TRACK_TAU: f32 = 0.08;

/// Time constant (seconds) of the target-acceleration track: the low-pass over
/// the change in the filtered velocity, per second.
///
/// The sample is already smoothed once, by [`TARGET_TRACK_TAU`]; this second
/// pole is for the bang-bang drive. A ship's thrusters fire full or not at all
/// per tick, so the per-tick acceleration jumps between zero and the drive's
/// full figure while the mean is what moves the hull. A quarter second spans
/// sixteen 64 Hz ticks of that chatter, and it costs little on a ship: the
/// raider burns measured in the escort fight hold one figure for one to several
/// seconds. The price is a weave: the Serpent turns its acceleration at 1.4
/// rad/s, and the two poles lag that turn by
/// `atan(1.4 * 0.08) + atan(1.4 * 0.25)`, about 26 deg. The weave model in
/// `point_defense_cost_tests` prices a Serpent within 1% of its velocity-only
/// cost, so the weave still does its job.
const TARGET_ACCEL_TAU: f32 = 0.25;

/// The largest acceleration sample (world units per second squared, 1 u/s^2
/// is 10 m/s^2) the track believes: 400 m/s^2.
///
/// The fastest-maneuvering bodies a gun is pointed at are torpedoes: about
/// 25 u/s^2 of thrust in the guidance model, and the Serpent's weave at
/// `max_speed * sin(weave_angle) * weave_rate`, about 19 u/s^2. The raiders in
/// the escort fight measured at most 8.2 u/s^2. A contact or a ram changes a
/// velocity by tens of u/s inside one 1/64 s tick, which reads as thousands of
/// u/s^2: physics, not a maneuver, and nothing to lead over a round's flight.
/// Clamped here, one such tick enters the track as at most
/// `40 * (1 - exp(-(1/64) / 0.25))`, 2.4 u/s^2.
const TARGET_ACCEL_MAX: f32 = 40.0;

/// One mount's filtered track of what it is shooting at: the mean course and
/// the acceleration the intercept solve leads, and the entity they belong to.
///
/// **Why a track at all.** [`lead_intercept_point`] extrapolates whatever
/// velocity and acceleration it is handed over the whole flight. Handed the raw
/// per-tick sample it chases a Serpent's corkscrew instead of predicting it:
/// the aim point swings with the weave, and a barrel that is always slewing is
/// never inside the fire gate. No fire-control solution has ever been laid off
/// a single sensor return.
///
/// **Why the acceleration is derived here.** The aim feeds publish a velocity,
/// which is what every body carries. The acceleration is that velocity's change
/// per second, taken off the FILTERED velocity so a contact spike does not
/// differentiate into a wild figure, clamped at [`TARGET_ACCEL_MAX`] and
/// smoothed at [`TARGET_ACCEL_TAU`].
///
/// **Why per MOUNT.** A per-TARGET track is the better picture of a real
/// battery - one filtered track per body, shared by every mount working it -
/// but the turret aim contract is a position, a velocity and an entity written
/// onto the section, not a handle into a shared track table, and the player's
/// crosshair tier aims at a commanded POINT with no entity behind it at all. A
/// shared table would owe that tier an answer forever. Every mount assigned the
/// same torpedo folds the same samples with the same fixed `dt` anyway, so the
/// per-mount copies agree; what the shared version would save is arithmetic,
/// not divergence. What it would BUY - a mount inheriting a converged track the
/// moment it is handed a torpedo another mount has been following - is real,
/// and is the reason to revisit this if the aim feeds ever grow an entity-keyed
/// picture.
#[derive(Component, Clone, Copy, Debug, Default, Reflect)]
pub struct TurretSectionTargetTrack {
    /// The entity the filter below belongs to. Any change to it throws the
    /// filter away: see [`TurretSectionTargetTrack::observe`].
    tracking: Option<Entity>,
    /// The filtered velocity, or `None` before the track has taken a sample.
    velocity: Option<Vec3>,
    /// The filtered acceleration. Zero until the track has seen a velocity
    /// change.
    acceleration: Vec3,
}

impl TurretSectionTargetTrack {
    /// The filtered velocity, or `None` when the track holds no sample yet.
    pub fn velocity(&self) -> Option<Vec3> {
        self.velocity
    }

    /// The filtered acceleration, world units per second squared.
    pub fn acceleration(&self) -> Vec3 {
        self.acceleration
    }

    /// The velocity and acceleration to lead on, or `None` when the track
    /// holds no sample yet.
    ///
    /// The velocity is the filtered one plus `acceleration * TARGET_TRACK_TAU`:
    /// a one-pole filter on a target under constant acceleration settles
    /// exactly that far behind it, so this is the target's own velocity again,
    /// not the filter's lag led over the flight.
    pub fn lead(&self) -> Option<(Vec3, Vec3)> {
        self.velocity.map(|velocity| {
            (
                velocity + self.acceleration * TARGET_TRACK_TAU,
                self.acceleration,
            )
        })
    }

    /// Fold one step of `dt` seconds of `sample` into the track.
    pub(crate) fn observe(&mut self, target: Option<Entity>, sample: Vec3, dt: f32) {
        // TARGET SWITCH. A mount that swapped torpedoes and kept the filter
        // would lead the new one along the dead one's course for a whole time
        // constant - and the two are rarely even on the same bearing, so the
        // barrel would spend that time aimed at nothing. Dropping the state is
        // the only honest answer: the new body shares no history with the old.
        if self.tracking != target {
            self.tracking = target;
            self.velocity = None;
            self.acceleration = Vec3::ZERO;
        }
        let Some(filtered) = self.velocity else {
            // TRACK INITIATION, on the first return of a new track. Easing up
            // from zero instead would aim at the target's CURRENT position for
            // the first time constant, which on a 30 u/s torpedo is a bigger
            // error than having no filter at all. One sample holds no change,
            // so the acceleration starts at zero.
            self.velocity = Some(sample);
            return;
        };
        // A zero step (the startup frame) folds nothing and has no rate.
        if dt <= 0.0 {
            return;
        }
        // The same exponential form as `AIM_CORRECTION_RATE` below, for the
        // same reason: it folds the same fraction of the error per unit TIME at
        // any step length, so a track means the same thing on any host.
        let velocity = filtered + (sample - filtered) * (1.0 - (-dt / TARGET_TRACK_TAU).exp());
        let change = ((velocity - filtered) / dt).clamp_length_max(TARGET_ACCEL_MAX);
        self.acceleration += (change - self.acceleration) * (1.0 - (-dt / TARGET_ACCEL_TAU).exp());
        self.velocity = Some(velocity);
    }
}

/// Fold each mount's raw target-velocity sample into its
/// [`TurretSectionTargetTrack`], so the intercept solve leads the target's mean
/// course and acceleration instead of the tick it happened to catch.
///
/// Runs on the FIXED clock at the head of the aim chain, with everything else
/// that decides where a barrel points: a filter on the render clock would
/// average a number of samples per simulated second that follows the frame
/// rate, which is a frame-rate-dependent point defence by another route.
pub(super) fn update_turret_target_track(
    time: Res<Time>,
    mut q_turret: Query<
        (
            &TurretSectionTargetEntity,
            &TurretSectionTargetVelocity,
            &mut TurretSectionTargetTrack,
        ),
        With<TurretSectionMarker>,
    >,
) {
    let dt = time.delta_secs();
    for (target, sample, mut track) in &mut q_turret {
        track.observe(**target, **sample, dt);
    }
}

/// Resolve each turret's lead intercept point into [`TurretSectionAimPoint`] from
/// its target position, its tracked velocity and acceleration, and the bullet
/// `muzzle_speed`. Runs before the yaw/pitch systems, which steer toward this
/// point.
///
/// The intercept is solved in the SHOOTER's frame: bullets inherit the full
/// muzzle point velocity at fire time (`shoot_spawn_projectile` adds ship
/// linear velocity plus the angular swing at the muzzle), so the solve uses
/// the target velocity RELATIVE to that same muzzle point velocity. Aiming
/// the barrel at the shooter-frame intercept and adding the inherited
/// velocity on launch lands the bullet on the true world-frame intercept:
/// dir*s*t = (target - muzzle) + (v_target - v_muzzle)*t + a_target*t^2/2.
/// Solving in the world frame instead makes every shot drift off by the
/// shooter's own motion. The acceleration is the target's own: once the round
/// leaves, the shooter's burn moves the shooter and not the round.
pub(crate) fn update_turret_aim_point(
    mut q_turret: Query<
        (
            &TurretSectionTargetInput,
            &TurretSectionTargetVelocity,
            &TurretSectionTargetTrack,
            &TurretEngineFigures,
            &TurretSectionMuzzleEntity,
            &ChildOf,
            &mut TurretSectionAimPoint,
        ),
        With<TurretSectionMarker>,
    >,
    // Fresh render-clock poses: this runs BEFORE this frame's transform
    // propagation (see the registration comment), so GlobalTransform still
    // holds last frame - TransformHelper composes the eased poses the frame
    // will render with. The pip therefore marks the intercept as the player
    // SEES it; the physical bullet spawns from the raw pose (sub-tick
    // apart, see shoot_spawn_projectile).
    transform_helper: TransformHelper,
    q_spaceship: Query<
        (&LinearVelocity, &AngularVelocity, &ComputedCenterOfMass),
        With<SpaceshipRootMarker>,
    >,
    time: Res<Time>,
) {
    // A ROUND DOES NOT LEAVE WHEN THIS SOLVE RUNS. `shoot_spawn_projectile`
    // fires it `lead` seconds into the tick - uniform over (0, dt] for any fire
    // interval longer than a tick - and compensates its POSITION for that so
    // the stream stays evenly spaced. The aim it flies along is not
    // compensated, so the round crosses the intercept `lead` late and a
    // CROSSING target has moved on: always behind, never ahead, 0 to 0.47 u at
    // a 30 u/s torpedo. Half a step is the expected delay, and correcting for
    // it leaves only the +/- half-step jitter.
    //
    // Invisible until rounds stopped being physics bodies: avian accepted a
    // contact at positive separation up to `dt * |v_relative|`, about 1.63 u at
    // point-defence closing speeds, which swallowed the trail whole.
    let launch_delay = 0.5 * time.delta_secs();
    for (
        target_input,
        target_sample,
        track,
        figures,
        TurretSectionMuzzleEntity(muzzle),
        ChildOf(spaceship),
        mut aim_point,
    ) in &mut q_turret
    {
        let Some(target_pos) = **target_input else {
            **aim_point = None;
            continue;
        };
        // The FILTERED course, never this step's sample: see
        // [`TurretSectionTargetTrack`]. A rig that runs this solve without the
        // track pass holds no track and falls back to the sample with no
        // acceleration, which is the right answer for one - it has no history
        // to average or differentiate.
        let (target_velocity, target_accel) = track.lead().unwrap_or((**target_sample, Vec3::ZERO));
        let Ok(muzzle_transform) = transform_helper.compute_global_transform(*muzzle) else {
            continue;
        };
        let muzzle_pos = muzzle_transform.translation();

        // The same muzzle point velocity the bullet will inherit on launch
        // (same COM lift frame as the muzzle pose above). A shooter without
        // physics components (test rigs) inherits nothing.
        let shooter_velocity = q_spaceship
            .get(*spaceship)
            .ok()
            .zip(transform_helper.compute_global_transform(*spaceship).ok())
            .map(|((lin_vel, ang_vel, center), ship_transform)| {
                rigid_body_point_velocity(
                    **lin_vel,
                    **ang_vel,
                    ship_transform.transform_point(**center),
                    muzzle_pos,
                )
            })
            .unwrap_or(Vec3::ZERO);

        // Solved in the shooter's frame, so the target drifts at the RELATIVE
        // velocity over the launch delay too, and keeps accelerating through
        // it.
        let target_velocity = target_velocity - shooter_velocity;
        **aim_point = Some(lead_intercept_point(
            muzzle_pos,
            target_pos
                + target_velocity * launch_delay
                + 0.5 * target_accel * launch_delay * launch_delay,
            target_velocity + target_accel * launch_delay,
            target_accel,
            figures.muzzle_speed,
            figures.reach / figures.muzzle_speed,
        ));
    }
}

/// The signed angle (radians) that rotates `from` onto `to` about `axis`,
/// measured in the plane perpendicular to `axis`. `axis` is assumed normalized.
///
/// Near-antiparallel `from`/`to` (target dead behind the barrel) is a rotation
/// singularity: either half-turn is valid and the cross product carries no
/// reliable sign, so a naive `atan2` dithers frame to frame and the hinge freezes
/// in place ("stuck, won't turn around"). When the vectors are within ~1.6 deg of
/// opposite, commit to a deterministic +pi half-turn so the joint swings around;
/// once it moves off the singularity the normal solve resumes and finishes the
/// turn the short way.
fn signed_angle_about(from: Vec3, to: Vec3, axis: Vec3) -> f32 {
    let from = from.normalize();
    let to = to.normalize();
    let c = from.cross(to).dot(axis);
    let d = from.dot(to);
    if d < -0.9996 {
        return std::f32::consts::PI;
    }
    c.atan2(d)
}

/// Per-second decay rate of each hinge's CCD aim error. The step target is
/// `output + gain * delta` with `gain = 1 - exp(-AIM_CORRECTION_RATE * dt)`,
/// NOT the full `output + delta`, for two reasons:
///
/// - DAMPING. A hinge solves its `delta` assuming every OTHER joint holds
///   still, but a turret's joints are coupled (yaw and pitch both swing the
///   offset muzzle), so applying each full correction at once overshoots the
///   joint solution and settles into a visible limit cycle - the barrel shakes
///   a few degrees around the aim. Under-correcting makes the coupled system
///   converge monotonically instead of ringing. Large errors are still
///   rate-limited by `SmoothLookRotation` (not this), so a slew stays
///   responsive; the decay only shapes the settle.
/// - FRAMERATE INVARIANCE. A flat per-frame gain decays the error per FRAME,
///   so residual tracking lag scaled with frame time (~0.43 deg at 60 fps vs
///   ~1.8 deg at 14 fps on a crossing target) and sat above the then-fixed
///   0.92 deg fire gate exactly when the machine struggled. The exponential
///   form decays the same error fraction per unit TIME at any frame rate.
///
/// DERIVED from the shipped per-frame gain, not retuned: the damp shipped as
/// a flat 0.35 of the error per frame, tuned at 60 fps (playtest-lowered from
/// 0.5 for shake margin). `-ln(1 - 0.35) * 60 = 25.847` reproduces exactly
/// that fraction at dt = 1/60, so the 60 fps feel is unchanged. Lower the
/// rate if a deep chain rings.
const AIM_CORRECTION_RATE: f32 = 25.847;

/// Below this per-frame correction (radians, ~0.23 deg) a hinge is treated as
/// ON target and HOLDS (target = current output) instead of chasing sub-degree
/// residuals - stops the micro-jitter that the lead-point feedback (the aim
/// point moves with the muzzle it is aiming) would otherwise sustain forever.
const AIM_DEADBAND_RAD: f32 = 0.004;

/// How close the muzzle forward may come to a hinge's own axis before that
/// hinge is treated as being AT ITS POLE. The value is the sine of the angle
/// between them (~0.06 deg), which is what the in-plane muzzle component
/// measures. Below it the component carries no usable heading - it is pure
/// rounding noise - so the solver falls back to the joint's last measured
/// heading instead of reading a direction out of the noise.
const AIM_HINGE_POLE_SIN: f32 = 1e-3;

/// Generic aim: a Jacobi per-frame hinge-CCD pass over each turret's muzzle
/// chain (one step per articulated joint, from the muzzle up to the root). Each
/// articulated joint nudges its [`SmoothLookRotationTarget`] toward the angle
/// that swings the muzzle forward (-Z) at the aim point, decomposed in that
/// joint's own hinge plane and DAMPED by [`AIM_CORRECTION_RATE`] so coupled
/// joints do not overshoot into a shake; the [`SmoothLookRotation`] controller
/// rate-limits and clamps the visible motion. Basis-independent, so it reduces
/// to the old yaw/pitch behavior for the Y/X chain and solves arbitrary trees.
///
/// Same fresh-pose composition as [`update_turret_aim_point`]: runs BEFORE this
/// frame's transform propagation, so poses are composed via [`TransformHelper`].
///
/// A hinge whose muzzle forward lines up with its OWN axis is at a pole: the
/// joint can no longer see where it points, because rotating it does not move
/// the muzzle direction at all. The elevation limit of a hull-mounted turret
/// sits exactly on such a pole (barrel straight up, yaw axis vertical), and a
/// pinned pitch hinge holds the chain there, so the pole is a trap rather than a
/// point the solve passes through: yaw is the only joint that can fix a heading
/// error, and it went blind. Each articulated joint therefore remembers the last
/// heading it could measure ([`TurretJointAimHeading`]) and steers by that while
/// at the pole, which turns it onto the target's bearing until the pitch demand
/// drops back under the limit and the normal solve resumes.
pub(super) fn update_turret_target_joints_system(
    time: Res<Time>,
    q_turret: Query<
        (
            &TurretSectionAimPoint,
            &TurretSectionMuzzles,
            Option<&TurretStow>,
        ),
        (With<TurretSectionMarker>, Without<SectionInactiveMarker>),
    >,
    q_child_of: Query<&ChildOf>,
    q_joint: Query<(&TurretJointMarker, &Transform)>,
    mut q_target_mut: Query<&mut SmoothLookRotationTarget>,
    mut q_heading: Query<&mut TurretJointAimHeading>,
    q_output: Query<&SmoothLookRotationOutput>,
    transform_helper: TransformHelper,
) {
    // The error fraction each hinge corrects this frame; dt 0 (startup frame)
    // holds every joint. See AIM_CORRECTION_RATE for the derivation.
    let gain = 1.0 - (-AIM_CORRECTION_RATE * time.delta_secs()).exp();
    for (aim_point, muzzles, stow) in &q_turret {
        // A mount that is not fully deployed does not track: the stow
        // machine owns its hinges (the barrel-up attitude) until the phase
        // returns to Deployed. A turret with no stow machine always tracks.
        if stow.is_some_and(|stow| !stow.is_deployed()) {
            continue;
        }
        let Some(target) = **aim_point else {
            continue;
        };

        // Steer EVERY muzzle of the turret at the shared aim point. Shared
        // joints (a twin barrel's common yaw/pitch) are
        // written by each muzzle's pass and agree for symmetric barrels; a
        // muzzle's private joints refine only its own chain.
        for &muzzle in &muzzles.0 {
            let Ok(muzzle_transform) = transform_helper.compute_global_transform(muzzle) else {
                error!(
                    "update_turret_target_joints_system: muzzle {:?} has no computable pose",
                    muzzle
                );
                continue;
            };
            let m = muzzle_transform.translation();
            let d: Vec3 = muzzle_transform.forward().into();
            if target == m {
                continue;
            }

            // Walk from the muzzle up to the turret root via ChildOf, collecting
            // the articulated joints along the chain. A single Jacobi pass: every
            // joint steps from the CURRENT pose this frame.
            let mut chain = muzzle;
            while let Ok(&ChildOf(parent)) = q_child_of.get(chain) {
                // Only walk while the current node is a turret joint; the parent
                // of the root joint is the turret section entity (not a joint),
                // which stops the walk.
                let Ok((marker, joint_transform)) = q_joint.get(chain) else {
                    break;
                };

                if let Some(a_local) = marker.axis {
                    // Parent global pose, then the joint's pre-rotation frame F:
                    // parent orientation + this joint's origin (offset applied).
                    if let Ok(parent_global) = transform_helper.compute_global_transform(parent) {
                        let f = parent_global
                            * Transform::from_translation(joint_transform.translation);
                        let w2j = f.to_matrix().inverse();

                        let ml = w2j.transform_point3(m);
                        let dl = w2j.transform_vector3(d);
                        let tl = w2j.transform_point3(target);
                        let a = a_local.normalize();

                        let des = tl - ml;
                        let d_perp = dl - a * dl.dot(a);
                        let t_perp = des - a * des.dot(a);
                        let out = q_output.get(chain).map(|o| **o).unwrap_or(0.0);

                        // The joint's heading in its own hinge plane. Off the
                        // pole the muzzle forward gives it, and the joint keeps
                        // a copy in its post-rotation frame (so the copy turns
                        // WITH the joint and stays valid at any angle). At the
                        // pole that copy is all there is.
                        let mut stored = q_heading.get_mut(chain).ok();
                        let heading = if d_perp.length() > AIM_HINGE_POLE_SIN {
                            let heading = d_perp.normalize();
                            if let Some(stored) = stored.as_mut() {
                                stored.0 = Some(Quat::from_axis_angle(a, -out) * heading);
                            }
                            Some(heading)
                        } else {
                            // The forward sits ON the hinge axis, but the
                            // muzzle's up vector still turns with the joint,
                            // and its projection is exactly where the forward
                            // lands once the barrel comes off the pole (sign
                            // per which pole). Measuring it here is what lets
                            // a mount that WAKES UP parked on a pole - the
                            // stow's barrel-up snap - find its bearing at
                            // all; the stored copy alone only covers a pole
                            // crossed while already tracking.
                            let ul = w2j.transform_vector3(muzzle_transform.up().into());
                            let u_perp = ul - a * ul.dot(a);
                            if u_perp.length() > AIM_HINGE_POLE_SIN {
                                let heading = (u_perp * -dl.dot(a).signum()).normalize();
                                if let Some(stored) = stored.as_mut() {
                                    stored.0 = Some(Quat::from_axis_angle(a, -out) * heading);
                                }
                                Some(heading)
                            } else {
                                stored
                                    .as_ref()
                                    .and_then(|stored| stored.0)
                                    .map(|heading| Quat::from_axis_angle(a, out) * heading)
                            }
                        };

                        // A target ON the hinge axis has no bearing to steer to.
                        if let Some(heading) = heading.filter(|_| t_perp.length() > 1e-6) {
                            let delta = signed_angle_about(heading, t_perp, a);
                            if let Ok(mut target_angle) = q_target_mut.get_mut(chain) {
                                // Damp the correction so coupled joints converge
                                // instead of ringing; hold once aimed (deadband).
                                **target_angle = if delta.abs() <= AIM_DEADBAND_RAD {
                                    out
                                } else {
                                    out + gain * delta
                                };
                            }
                        }
                    }
                }

                chain = parent;
            }
        }
    }
}

/// Apply each articulated joint's controller output onto its own transform's
/// rotation (fixed joints keep identity). One generic sync, replacing the
/// per-yaw/per-pitch pair - the joint entity carries both the offset transform
/// and the controller, so the hinge rotation is written back onto the SAME
/// transform.
pub(super) fn sync_turret_joint_rotation(
    mut q_joint: Query<(
        &TurretJointMarker,
        &SmoothLookRotationOutput,
        &mut Transform,
    )>,
) {
    for (marker, output, mut transform) in &mut q_joint {
        if let Some(axis) = marker.axis {
            transform.rotation = Quat::from_axis_angle(axis.normalize(), **output);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use nova_events::units::prelude::*;

    use super::{
        super::{config::default_joint_speed, test_support::*},
        *,
    };

    /// The cone is arithmetic, not a taste call, so the arithmetic is pinned:
    /// a round fired at the FULL tolerance lands exactly on the edge of what
    /// was aimed at, at any range and any size. Re-derive the commanded-point
    /// fallback in the SAME commit as any change to engagement ranges.
    #[test]
    fn the_on_target_cone_is_what_still_lands_on_what_was_aimed_at() {
        let derived = (POINT_AIM_RADIUS / CLOSE_ENGAGEMENT_RANGE).atan();
        assert!(
            (POINT_AIM_ON_TARGET_RAD - derived).abs() < 2e-6,
            "the fallback must be atan({POINT_AIM_RADIUS} / {CLOSE_ENGAGEMENT_RANGE}) \
             = {derived} rad, got {POINT_AIM_ON_TARGET_RAD}"
        );
        assert_eq!(
            on_target_cone(None, 1.0),
            POINT_AIM_ON_TARGET_RAD,
            "a commanded point has no size, so range cannot move its gate"
        );

        // The far edge of a gunfight: the PDC's own fire gate, muzzle_speed *
        // projectile_lifetime * AI_FIRE_RANGE_FACTOR - the authored 1 000 m/s
        // is 100 u/s, so 100 * 2.0 * 0.9 = 180 u.
        let config = TurretSectionConfig::default();
        let far = config
            .muzzle_speed
            .over(config.projectile_lifetime)
            .to_engine()
            * 0.9;
        // The carrier's own arm and a Serpent's own envelope, the two ends of
        // what a gun is ever pointed at.
        for (hit_radius, distance) in [
            (19.4, far),
            (19.4, CLOSE_ENGAGEMENT_RANGE),
            (1.2, CLOSE_ENGAGEMENT_RANGE),
        ] {
            let miss = distance * on_target_cone(Some(hit_radius), distance).tan();
            assert!(
                (miss - hit_radius).abs() < 1e-3,
                "a {hit_radius} u target at {distance} u must permit exactly \
                 {hit_radius} u of miss, permits {miss} u"
            );
        }

        assert_eq!(
            on_target_cone(Some(0.0), far),
            FIRE_GATE_FLOOR_RAD,
            "a barrel is never graded inside what it can settle to plus its scatter"
        );
    }

    #[test]
    fn a_turret_settled_inside_its_deadband_on_a_small_target_at_long_range_may_fire() {
        // The escort fight: the player pins a raider's turret mount, 0.43 u
        // across, at 2 km. Its own size is 0.002 rad, half a hinge deadband.
        let muzzle = Vec3::ZERO;
        let aim = Vec3::new(0.0, 0.0, -200.0);
        let mount_radius = 0.43;
        assert!((mount_radius / 200.0_f32).atan() < AIM_DEADBAND_RAD);

        // Both hinges rest at the edge of their deadband: the worst a settled
        // barrel holds.
        let settled = Quat::from_rotation_y(AIM_DEADBAND_RAD)
            * Quat::from_rotation_x(AIM_DEADBAND_RAD)
            * Vec3::NEG_Z;
        assert!(
            muzzle_on_target(settled, muzzle, aim, Some(mount_radius)),
            "a barrel that stopped correcting must open its gate"
        );

        // The gate still means something: half a degree off is a slew, not
        // a settle, and holds.
        let slewing = Quat::from_rotation_y(0.5_f32.to_radians()) * Vec3::NEG_Z;
        assert!(
            !muzzle_on_target(slewing, muzzle, aim, Some(mount_radius)),
            "a barrel half a degree off a small target must hold"
        );
    }

    #[test]
    fn a_muzzle_pointed_across_its_own_ship_is_never_on_target() {
        // The owner's complaint, as the predicate sees it: a barrel that cannot
        // swing round to the far side reads tens of degrees off, nowhere near
        // the cone, whatever else the fire path thinks.
        let muzzle = Vec3::ZERO;
        let aim = Vec3::new(0.0, 0.0, -100.0);
        assert!(muzzle_on_target(Vec3::NEG_Z, muzzle, aim, None), "dead on");
        assert!(
            !muzzle_on_target(Vec3::X, muzzle, aim, None),
            "a mount pinned across the hull is 90 deg off"
        );
        // The commanded-point boundary: half a degree in is on, two out is off.
        let off = |degrees: f32| Quat::from_rotation_y(degrees.to_radians()) * Vec3::NEG_Z;
        assert!(
            muzzle_on_target(off(0.5), muzzle, aim, None),
            "settled tracking"
        );
        assert!(!muzzle_on_target(off(2.0), muzzle, aim, None), "mid-slew");
        // A BODY moves that boundary BOTH ways, which is the whole reason the
        // gate is derived: 2 deg still lands on a carrier at 1 km, and 1 deg
        // already flies past a Serpent at the same range.
        assert!(
            muzzle_on_target(off(2.0), muzzle, aim, Some(19.4)),
            "a carrier at 1 km is 11 deg wide"
        );
        assert!(
            !muzzle_on_target(off(1.0), muzzle, aim, Some(1.2)),
            "a torpedo at 1 km is 0.7 deg wide"
        );
    }

    #[test]
    fn lead_of_a_stationary_target_is_the_target() {
        // No motion, no lead: aim straight at the target.
        let target = Vec3::new(0.0, 0.0, -100.0);
        let aim = lead_intercept_point(Vec3::ZERO, target, Vec3::ZERO, Vec3::ZERO, 100.0, 2.0);
        assert!(
            (aim - target).length() < 1e-3,
            "expected the target itself, got {aim:?}"
        );
    }

    #[test]
    fn lead_intercepts_a_crossing_target() {
        // Shooter at origin; target 100 ahead crossing at +X. The intercept point
        // must be one a bullet at muzzle_speed and the target reach at the SAME
        // time - that is what "leading" means. It must also sit ahead of the target
        // in its direction of travel (+X).
        let shooter = Vec3::ZERO;
        let target = Vec3::new(0.0, 0.0, -100.0);
        let target_vel = Vec3::new(30.0, 0.0, 0.0);
        let speed = 100.0;

        let aim = lead_intercept_point(shooter, target, target_vel, Vec3::ZERO, speed, 2.0);

        assert!(
            aim.x > 0.1,
            "intercept should lead a +X crosser, got {aim:?}"
        );
        // Consistency: at the bullet's flight time, the target is exactly there.
        let flight_time = (aim - shooter).length() / speed;
        let target_future = target + target_vel * flight_time;
        assert!(
            (target_future - aim).length() < 1e-2,
            "bullet and target should meet: aim {aim:?}, target at t {target_future:?}"
        );
    }

    #[test]
    fn a_round_fired_on_the_lead_meets_a_target_under_constant_acceleration() {
        // The escort fight's shape: a raider ~750 m off, crossing and burning
        // hard across the line of fire, shot by a PDC round (100 u/s, 2 s).
        let shooter = Vec3::ZERO;
        let speed = 100.0;
        let cases = [
            // Crossing at 17 u/s, burning 5 u/s^2 across and away.
            (
                Vec3::new(0.0, 0.0, -75.0),
                Vec3::new(17.0, 0.0, 0.0),
                Vec3::new(3.0, 2.0, -3.0),
            ),
            // Closing while braking and side-slipping off the line.
            (
                Vec3::new(10.0, 5.0, -120.0),
                Vec3::new(0.0, 0.0, 20.0),
                Vec3::new(0.0, 5.0, -3.0),
            ),
            // A Serpent-hard turn at point-defence range.
            (
                Vec3::new(-40.0, 0.0, -60.0),
                Vec3::new(5.0, 0.0, 30.0),
                Vec3::new(19.0, 0.0, 0.0),
            ),
        ];
        for (target, velocity, accel) in cases {
            let aim = lead_intercept_point(shooter, target, velocity, accel, speed, 2.0);
            // Fly the round along the barrel the solve chose and the target
            // along its parabola, and find their closest approach.
            let round = (aim - shooter).normalize() * speed;
            let miss = (0..=20_000)
                .map(|k| {
                    let t = k as f32 * 1e-4;
                    (target + velocity * t + 0.5 * accel * t * t - (shooter + round * t)).length()
                })
                .fold(f32::INFINITY, f32::min);
            assert!(
                miss < 0.05,
                "a round on the lead must meet the target: {target:?} at {velocity:?} \
                 accelerating {accel:?} is missed by {miss} u"
            );
            // The control: the velocity-only lead misses the same target by
            // about half its acceleration over the flight squared.
            let naive = velocity_lead_point(shooter, target, velocity, speed);
            let naive_round = (naive - shooter).normalize() * speed;
            let naive_miss = (0..=20_000)
                .map(|k| {
                    let t = k as f32 * 1e-4;
                    (target + velocity * t + 0.5 * accel * t * t - (shooter + naive_round * t))
                        .length()
                })
                .fold(f32::INFINITY, f32::min);
            assert!(
                naive_miss > 0.5,
                "the case must be one the velocity-only lead misses: {naive_miss} u"
            );
        }
    }

    #[test]
    fn lead_falls_back_when_the_target_cannot_be_caught() {
        // Target receding faster than the bullet: no positive intercept, so aim at
        // where it is now rather than returning a garbage/NaN point.
        let target = Vec3::new(0.0, 0.0, -50.0);
        let target_vel = Vec3::new(0.0, 0.0, -200.0); // fleeing at 200, bullet only 100
        let aim = lead_intercept_point(Vec3::ZERO, target, target_vel, Vec3::ZERO, 100.0, 2.0);
        assert!(aim.is_finite());
        assert!(
            (aim - target).length() < 1e-3,
            "expected fallback to the target, got {aim:?}"
        );
    }

    /// The track constant is arithmetic, not a taste call, and both of its
    /// inputs are live: the Serpent's own authored numbers and the turret's own
    /// muzzle speed. Retune either and this test says the constant needs
    /// re-deriving rather than letting the lead quietly slip off a hull.
    #[test]
    fn the_track_constant_holds_a_serpents_lead_inside_a_hull() {
        // The lateral acceleration a straight-line lead cannot predict, from
        // TorpedoTypeConfig::weave_rate's own field doc: the weave holds the
        // nose `weave_angle` off the guidance solution and spins that tilt at
        // `weave_rate`.
        let serpent = TorpedoTypeConfig::default();
        let lateral =
            serpent.max_speed.to_engine() * serpent.weave_angle.sin() * serpent.weave_rate;
        // The flight time the on-target cone is graded at: a round's time to
        // the CLOSE edge of a gunfight.
        let flight =
            CLOSE_ENGAGEMENT_RANGE / TurretSectionConfig::default().muzzle_speed.to_engine();
        // A one-pole filter following a ramp settles exactly `tau` behind it.
        let shortfall = lateral * TARGET_TRACK_TAU * flight;
        assert!(
            shortfall <= POINT_AIM_RADIUS,
            "a track constant of {TARGET_TRACK_TAU} s leads a Serpent \
             ({lateral} u/s^2 of weave) {shortfall} u short over a {flight} s \
             flight, outside the {POINT_AIM_RADIUS} u half-beam it has to land \
             inside"
        );
    }

    /// One mount whose track the filter pass can be stepped by hand, on a clock
    /// with a fixed `dt` (the production step, unless a test says otherwise).
    fn track_world(dt: f32) -> (World, Entity) {
        let mut world = World::new();
        let mut time = Time::<()>::default();
        time.advance_by(Duration::from_secs_f32(dt));
        world.insert_resource(time);
        let turret = world
            .spawn((TurretSectionMarker, TurretSectionTargetVelocity(Vec3::ZERO)))
            .id();
        (world, turret)
    }

    /// Hand the mount one sample of `target`'s velocity and return the track.
    fn step_track(world: &mut World, turret: Entity, target: Option<Entity>, sample: Vec3) -> Vec3 {
        use bevy::ecs::system::RunSystemOnce;
        {
            let mut entity = world.entity_mut(turret);
            **entity.get_mut::<TurretSectionTargetEntity>().unwrap() = target;
            **entity.get_mut::<TurretSectionTargetVelocity>().unwrap() = sample;
        }
        world.run_system_once(update_turret_target_track).unwrap();
        world
            .entity(turret)
            .get::<TurretSectionTargetTrack>()
            .expect("the velocity component requires a track")
            .velocity()
            .expect("a stepped track holds a sample")
    }

    /// The production fixed step: 64 Hz, which nothing configures away.
    const FIXED_STEP: f32 = 1.0 / 64.0;

    /// Steps of `dt` in `seconds`, for a test that wants a duration.
    fn steps(seconds: f32, dt: f32) -> usize {
        (seconds / dt).ceil() as usize
    }

    #[test]
    fn a_track_converges_on_a_steady_course() {
        // The filter must LAG a course change, not refuse it: a target that
        // settles on a new heading has to be led along the new one.
        let (mut world, turret) = track_world(FIXED_STEP);
        let torpedo = world.spawn_empty().id();
        let old = Vec3::new(0.0, 0.0, 30.0);
        let new = Vec3::new(20.0, 0.0, 30.0);

        step_track(&mut world, turret, Some(torpedo), old);
        let first = step_track(&mut world, turret, Some(torpedo), new);
        assert!(
            (first - old).length() < (first - new).length(),
            "one step must move a fraction of the way, not jump: {first:?}"
        );

        let mut track = first;
        for _ in 0..steps(5.0 * TARGET_TRACK_TAU, FIXED_STEP) {
            track = step_track(&mut world, turret, Some(torpedo), new);
        }
        assert!(
            (track - new).length() < 0.2,
            "five time constants on a steady course must converge onto it: \
             {track:?} against {new:?}"
        );
    }

    #[test]
    fn a_track_rejects_a_single_tick_spike() {
        // THE POINT of the track. One step's velocity - a body shoved by a
        // contact, or a weave sample caught at its extreme - is extrapolated
        // over a whole second of bullet flight by the intercept solve, so a
        // spike the filter passes through is a spike the barrel chases.
        let (mut world, turret) = track_world(FIXED_STEP);
        let torpedo = world.spawn_empty().id();
        let course = Vec3::new(0.0, 0.0, 30.0);
        let spike = Vec3::new(60.0, 0.0, 0.0);

        for _ in 0..steps(3.0 * TARGET_TRACK_TAU, FIXED_STEP) {
            step_track(&mut world, turret, Some(torpedo), course);
        }
        let spiked = step_track(&mut world, turret, Some(torpedo), course + spike);
        assert!(
            (spiked - course).length() < 0.25 * spike.length(),
            "one step's spike must enter the track as a fraction of itself, \
             not whole: {spiked:?} against a course of {course:?}"
        );

        for _ in 0..steps(3.0 * TARGET_TRACK_TAU, FIXED_STEP) {
            step_track(&mut world, turret, Some(torpedo), course);
        }
        let settled = step_track(&mut world, turret, Some(torpedo), course);
        assert!(
            (settled - course).length() < 0.05 * spike.length(),
            "and it must leave again: {settled:?} against a course of \
             {course:?}"
        );
    }

    #[test]
    fn switching_targets_resets_the_track() {
        // THE BUG this component exists to prevent: a mount handed a second
        // torpedo while its filter still holds the first one's course leads the
        // new one along the dead one's heading, and the two are rarely on the
        // same bearing at all. The track has to start over.
        let (mut world, turret) = track_world(FIXED_STEP);
        let first = world.spawn_empty().id();
        let second = world.spawn_empty().id();
        let inbound = Vec3::new(0.0, 0.0, 30.0);
        let crossing = Vec3::new(30.0, 0.0, 0.0);

        for _ in 0..steps(5.0 * TARGET_TRACK_TAU, FIXED_STEP) {
            step_track(&mut world, turret, Some(first), inbound);
        }

        // The control: the SAME target changing course only eases over.
        let eased = step_track(&mut world, turret, Some(first), crossing);
        assert!(
            (eased - inbound).length() < 0.5 * (crossing - inbound).length(),
            "a course change on one target must be filtered: {eased:?}"
        );

        // The switch: a different entity, and the track is the new body's own
        // velocity outright.
        let switched = step_track(&mut world, turret, Some(second), crossing);
        assert!(
            (switched - crossing).length() < 1e-4,
            "a mount handed a different torpedo must drop the old track and \
             take the new one's velocity: {switched:?} against {crossing:?}"
        );
    }

    #[test]
    fn a_track_means_the_same_thing_at_any_step_length() {
        // Framerate invariance, the defect this project already fixed once on
        // the aim chain: the same SIM time of the same samples must land the
        // same track whatever the step is, or point defence leads differently
        // on a fast host.
        //
        // A whole number of BOTH steps, so the two arms cover the same simulated
        // window rather than the same rounded-up count of different ones.
        const WINDOW_SECS: f32 = 0.25;
        let converged = |dt: f32| {
            let (mut world, turret) = track_world(dt);
            let torpedo = world.spawn_empty().id();
            let old = Vec3::new(0.0, 0.0, 30.0);
            let new = Vec3::new(20.0, 0.0, 30.0);
            step_track(&mut world, turret, Some(torpedo), old);
            let mut track = Vec3::ZERO;
            for _ in 0..(WINDOW_SECS / dt).round() as usize {
                track = step_track(&mut world, turret, Some(torpedo), new);
            }
            track
        };

        let fast = converged(FIXED_STEP);
        let slow = converged(4.0 * FIXED_STEP);
        assert!(
            (fast - slow).length() < 0.05,
            "{WINDOW_SECS} s of the same samples must converge the same amount \
             at any step: {fast:?} at {FIXED_STEP} s, {slow:?} at {} s",
            4.0 * FIXED_STEP
        );
    }

    /// World for aim-point tests: a ship with physics state, one turret with
    /// its muzzle 100 m ahead of a target-bearing setup. Returns (turret,
    /// muzzle position).
    fn aim_point_world(ship_velocity: Vec3, muzzle_pos: Vec3) -> (bevy::ecs::world::World, Entity) {
        let mut world = World::new();
        let ship = world
            .spawn((
                SpaceshipRootMarker,
                Transform::IDENTITY,
                LinearVelocity(ship_velocity),
                AngularVelocity(Vec3::ZERO),
                ComputedCenterOfMass(Vec3::ZERO),
            ))
            .id();
        let muzzle = world
            .spawn((
                TurretSectionBarrelMuzzleMarker,
                Transform::from_translation(muzzle_pos),
            ))
            .id();
        let turret = world
            .spawn((
                TurretSectionMarker,
                TurretSectionTargetInput(None),
                TurretSectionTargetVelocity(Vec3::ZERO),
                TurretSectionConfigHelper(TurretSectionConfig::default()),
                TurretSectionMuzzleEntity(muzzle),
                TurretSectionAimPoint(None),
                ChildOf(ship),
            ))
            .id();
        (world, turret)
    }

    fn aim_point(world: &mut World, turret: Entity) -> Option<Vec3> {
        use bevy::ecs::system::RunSystemOnce;
        // The solve reads the fixed clock for the launch delay. A default
        // `Time` has a zero delta, so these rigs measure the intercept with no
        // delay correction - which is what they are about.
        world.init_resource::<Time>();
        world.run_system_once(update_turret_aim_point).unwrap();
        **world.entity(turret).get::<TurretSectionAimPoint>().unwrap()
    }

    #[test]
    fn formation_flight_needs_no_lead() {
        // Shooter and target flying in formation (equal velocities): the
        // bullet inherits the shared motion on launch, so the correct aim is
        // the target itself. Solving in the world frame instead would lead a
        // target that, relatively, is not moving - the shots that never hit
        // from a moving ship.
        let velocity = Vec3::new(40.0, 0.0, -10.0);
        let target = Vec3::new(0.0, 0.0, -100.0);
        let (mut world, turret) = aim_point_world(velocity, Vec3::ZERO);
        {
            let mut entity = world.entity_mut(turret);
            **entity.get_mut::<TurretSectionTargetInput>().unwrap() = Some(target);
            **entity.get_mut::<TurretSectionTargetVelocity>().unwrap() = velocity;
        }

        let aim = aim_point(&mut world, turret).expect("aim point computed");

        assert!(
            (aim - target).length() < 1e-2,
            "formation flight must aim at the target itself, got {aim:?}"
        );
    }

    #[test]
    fn moving_shooter_aims_retrograde_of_its_own_motion() {
        // Static target, shooter strafing +X: the bullet inherits +X on
        // launch, so the barrel must point BEHIND the shooter's motion for
        // the drift to carry the bullet onto the target.
        let target = Vec3::new(0.0, 0.0, -100.0);
        let (mut world, turret) = aim_point_world(Vec3::new(30.0, 0.0, 0.0), Vec3::ZERO);
        {
            let mut entity = world.entity_mut(turret);
            **entity.get_mut::<TurretSectionTargetInput>().unwrap() = Some(target);
        }

        let aim = aim_point(&mut world, turret).expect("aim point computed");

        assert!(
            aim.x < -0.1,
            "a +X shooter must aim -X of the target, got {aim:?}"
        );
        // Consistency: bullet (barrel direction * speed + inherited velocity)
        // and target meet at the flight time the solve implies.
        let speed = TurretSectionConfig::default().muzzle_speed.to_engine();
        let flight_time = aim.length() / speed;
        let barrel_dir = aim.normalize();
        let bullet_at_t = (barrel_dir * speed + Vec3::new(30.0, 0.0, 0.0)) * flight_time;
        assert!(
            (bullet_at_t - target).length() < 0.5,
            "inherited drift must carry the bullet onto the target: bullet \
             {bullet_at_t:?}, target {target:?}"
        );
    }

    #[test]
    fn shooter_without_physics_inherits_nothing() {
        // Test rigs and previews have marker-only ships: the aim point must
        // fall back to the plain world-frame solve instead of skipping.
        let target = Vec3::new(0.0, 0.0, -100.0);
        let mut world = World::new();
        let ship = world.spawn(SpaceshipRootMarker).id();
        let muzzle = world
            .spawn((TurretSectionBarrelMuzzleMarker, Transform::IDENTITY))
            .id();
        let turret = world
            .spawn((
                TurretSectionMarker,
                TurretSectionTargetInput(Some(target)),
                TurretSectionTargetVelocity(Vec3::ZERO),
                TurretSectionConfigHelper(TurretSectionConfig::default()),
                TurretSectionMuzzleEntity(muzzle),
                TurretSectionAimPoint(None),
                ChildOf(ship),
            ))
            .id();

        let aim = aim_point(&mut world, turret).expect("aim point computed");
        assert!((aim - target).length() < 1e-3);
    }

    /// An app that runs the full aim + controller + sync + propagation loop on a
    /// manual clock, so a stepped turret converges its muzzle onto a target.
    fn aim_convergence_app() -> App {
        use bevy::time::TimeUpdateStrategy;
        use nova_gameplay::transform::prelude::SmoothLookRotationPlugin;

        let mut app = App::new();
        app.add_plugins((MinimalPlugins, TransformPlugin, SmoothLookRotationPlugin));
        app.add_observer(insert_turret_section);
        app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
            1.0 / 60.0,
        )));
        // The aim chain runs in FixedUpdate BEFORE the controller's Sync and
        // the joint sync after it, the same edges `TurretSectionPlugin`
        // declares; this app wires the systems directly rather than through the
        // plugin, so it repeats them.
        app.add_systems(
            FixedUpdate,
            (
                (
                    update_turret_target_track,
                    update_turret_aim_point,
                    update_turret_target_joints_system,
                )
                    .chain()
                    .before(SmoothLookRotationSystems::Sync),
                sync_turret_joint_rotation.after(SmoothLookRotationSystems::Sync),
            ),
        );
        app
    }

    /// The world-space muzzle forward (-Z) after propagation.
    fn muzzle_aim_error_deg(app: &mut App, muzzle: Entity, target: Vec3) -> f32 {
        let gt = *app.world().get::<GlobalTransform>(muzzle).unwrap();
        let pos = gt.translation();
        let forward: Vec3 = gt.forward().into();
        let to_target = (target - pos).normalize();
        forward.dot(to_target).clamp(-1.0, 1.0).acos().to_degrees()
    }

    #[test]
    fn a_default_turret_converges_its_muzzle_onto_a_target() {
        // AIM CONVERGENCE (behavioral CCD parity): a default turret steered at a
        // reachable target must point its muzzle forward (-Z) at the target
        // within a few degrees after stepping frames. This is the parity
        // guarantee for the CCD swap - proven behaviorally, not by theta match.
        for target in [
            Vec3::new(10.0, 5.0, -30.0),
            Vec3::new(-15.0, 8.0, -25.0),
            Vec3::new(0.0, 3.0, -40.0),
        ] {
            let mut app = aim_convergence_app();
            let ship = app
                .world_mut()
                .spawn((SpaceshipRootMarker, Transform::IDENTITY))
                .id();
            let turret = app
                .world_mut()
                .spawn(turret_section(TurretSectionConfig {
                    muzzle_speed: MetersPerSecond(10_000.0), // near-straight lead so aim ~= target dir
                    ..Default::default()
                }))
                .id();
            app.world_mut()
                .entity_mut(turret)
                .insert((ChildOf(ship), Transform::IDENTITY));
            app.world_mut().flush();
            app.world_mut()
                .entity_mut(turret)
                .insert(TurretSectionTargetInput(Some(target)));
            let muzzle = muzzle_entity(&app, turret);

            for _ in 0..240 {
                app.update();
            }

            let error = muzzle_aim_error_deg(&mut app, muzzle, target);
            assert!(
                error < 5.0,
                "muzzle should converge onto {target:?}, aim error {error} deg"
            );
        }
    }

    #[test]
    fn a_turret_not_fully_deployed_does_not_track_its_target() {
        // The stow gate on the aim solver: the same rig that converges above
        // must NOT steer while the stow machine holds any phase short of
        // Deployed - its hinges belong to the stow attitude, not the target.
        let target = Vec3::new(30.0, 0.0, -10.0); // ~72 deg off boresight
        let mut app = aim_convergence_app();
        let ship = app
            .world_mut()
            .spawn((SpaceshipRootMarker, Transform::IDENTITY))
            .id();
        let turret = app
            .world_mut()
            .spawn(turret_section(TurretSectionConfig {
                muzzle_speed: MetersPerSecond(10_000.0),
                ..Default::default()
            }))
            .id();
        app.world_mut()
            .entity_mut(turret)
            .insert((ChildOf(ship), Transform::IDENTITY));
        app.world_mut().flush();
        app.world_mut().entity_mut(turret).insert((
            TurretSectionTargetInput(Some(target)),
            TurretStow::new(TurretStowPhase::Stowing),
        ));
        let muzzle = muzzle_entity(&app, turret);

        for _ in 0..240 {
            app.update();
        }

        let error = muzzle_aim_error_deg(&mut app, muzzle, target);
        assert!(
            error > 45.0,
            "a stowing turret must hold its pose, not converge: {error} deg"
        );
    }

    #[test]
    fn a_turret_swings_around_to_a_target_directly_behind() {
        // STUCK-BEHIND REGRESSION: a target dead behind the barrel is a rotation
        // singularity (either way is a valid 180); the naive solve dithered and
        // the turret froze facing forward. It must commit and swing around.
        let mut app = aim_convergence_app();
        let ship = app
            .world_mut()
            .spawn((SpaceshipRootMarker, Transform::IDENTITY))
            .id();
        let turret = app
            .world_mut()
            .spawn(turret_section(TurretSectionConfig {
                muzzle_speed: MetersPerSecond(10_000.0),
                ..Default::default()
            }))
            .id();
        app.world_mut()
            .entity_mut(turret)
            .insert((ChildOf(ship), Transform::IDENTITY));
        app.world_mut().flush();
        // Barrel rest points -Z; put the target dead behind at +Z (a hair off the
        // axis so it is a true 180, level so pitch can hold it).
        let target = Vec3::new(0.0, 0.0, 30.0);
        app.world_mut()
            .entity_mut(turret)
            .insert(TurretSectionTargetInput(Some(target)));
        let muzzle = muzzle_entity(&app, turret);

        for _ in 0..240 {
            app.update();
        }

        let error = muzzle_aim_error_deg(&mut app, muzzle, target);
        assert!(
            error < 5.0,
            "turret must swing around to a target behind, not freeze forward: \
             aim error {error} deg"
        );
    }

    #[test]
    fn a_turret_woken_parked_on_the_zenith_finds_a_target_behind() {
        // COLD-POLE REGRESSION: the stow snaps a cold mount's pitch onto its
        // +90 stop, so the first solve ever runs with the muzzle exactly on
        // the yaw pole and NO stored heading. The yaw pass must measure its
        // bearing off the muzzle's up vector and swing; before it did, the
        // yaw froze at rest and the pitch pinned against its stop demanding
        // the over-the-top path - a whole battery permanently blind.
        let mut app = aim_convergence_app();
        let ship = app
            .world_mut()
            .spawn((SpaceshipRootMarker, Transform::IDENTITY))
            .id();
        let turret = app
            .world_mut()
            .spawn(turret_section(TurretSectionConfig {
                muzzle_speed: MetersPerSecond(10_000.0),
                ..Default::default()
            }))
            .id();
        app.world_mut()
            .entity_mut(turret)
            .insert((ChildOf(ship), Transform::IDENTITY));
        app.world_mut().flush();

        // Park the barrel straight up before the first solve, the way the
        // stow's snap does: every hinge output and target lands on its top
        // stop (the free yaw stays home).
        let joints: Vec<Entity> = app
            .world_mut()
            .query_filtered::<Entity, (With<TurretSectionPartOf>, With<SmoothLookRotation>)>()
            .iter(app.world())
            .collect();
        for joint in joints {
            let look = *app.world().get::<SmoothLookRotation>(joint).unwrap();
            let parked = look.max.unwrap_or(look.initial);
            **app
                .world_mut()
                .get_mut::<SmoothLookRotationTarget>(joint)
                .unwrap() = parked;
            **app
                .world_mut()
                .get_mut::<SmoothLookRotationOutput>(joint)
                .unwrap() = parked;
        }

        // Dead behind and level: reachable only by a 180 yaw swing, which is
        // exactly the bearing the pole solve used to lose.
        let target = Vec3::new(0.0, 0.0, 30.0);
        app.world_mut()
            .entity_mut(turret)
            .insert(TurretSectionTargetInput(Some(target)));
        let muzzle = muzzle_entity(&app, turret);

        for _ in 0..240 {
            app.update();
        }

        let error = muzzle_aim_error_deg(&mut app, muzzle, target);
        assert!(
            error < 5.0,
            "a mount woken on the zenith must find its bearing, not pin \
             against its stop: aim error {error} deg"
        );
    }

    /// The kinematic chain the SHIPPED turret prototypes are built from
    /// (`nova_authoring`'s `turret_joint_tree`), rebuilt here because that crate
    /// depends on this one. The dimensions matter to the pole: every offset is
    /// on the yaw axis' own plane (x = 0), and the barrel hangs BEHIND the pivot,
    /// so at full elevation the muzzle sits astern of the axis it points along -
    /// the case where the obvious lever-arm fallback steers the wrong way.
    fn shipped_turret_chain() -> TurretJoint {
        let fixed = |offset: Vec3, muzzle, children| TurretJoint {
            name: None,
            offset,
            axis: None,
            speed: std::f32::consts::PI,
            min: None,
            max: None,
            render_mesh: None,
            render_mesh_transform: None,
            muzzle,
            children,
        };
        fixed(
            Vec3::new(0.0, -0.5, 0.0),
            None,
            vec![TurretJoint {
                name: None,
                offset: Vec3::new(0.0, 0.1, 0.0),
                axis: Some(Vec3::Y),
                speed: std::f32::consts::PI,
                min: None,
                max: None,
                render_mesh: None,
                render_mesh_transform: None,
                muzzle: None,
                children: vec![TurretJoint {
                    name: None,
                    offset: Vec3::new(0.0, 0.332706, 0.303954),
                    axis: Some(Vec3::X),
                    speed: std::f32::consts::PI,
                    min: Some(-std::f32::consts::PI / 18.0),
                    max: Some(std::f32::consts::FRAC_PI_2),
                    render_mesh: None,
                    render_mesh_transform: None,
                    muzzle: None,
                    children: vec![fixed(
                        Vec3::new(0.0, 0.128437, -0.110729),
                        None,
                        vec![fixed(
                            Vec3::new(0.0, 0.0, -1.2),
                            Some(MuzzleConfig {
                                id: "main".to_string(),
                                fire_rate: 10.0,
                                muzzle_effect: None,
                            }),
                            vec![],
                        )],
                    )],
                }],
            }],
        )
    }

    #[test]
    fn a_turret_tracks_a_target_across_the_zenith() {
        // POLE REGRESSION: a target that crosses overhead drives the pitch hinge
        // into its +90 deg elevation limit, where the muzzle forward is PARALLEL
        // to the yaw hinge axis. The yaw joint's in-plane muzzle heading collapses
        // there, so it took no correction, and pitch - the only other joint - was
        // already pinned at the limit. The barrel parked pointing straight up and
        // never came back down. It must follow the target onto the new bearing.
        //
        // Run on the default chain AND on the shipped one, because the two put
        // the muzzle on opposite sides of the yaw axis at full elevation.
        for root in [TurretSectionConfig::default().root, shipped_turret_chain()] {
            let mut app = aim_convergence_app();
            let ship = app
                .world_mut()
                .spawn((SpaceshipRootMarker, Transform::IDENTITY))
                .id();
            let turret = app
                .world_mut()
                .spawn(turret_section(TurretSectionConfig {
                    root,
                    muzzle_speed: MetersPerSecond(10_000.0),
                    ..Default::default()
                }))
                .id();
            app.world_mut()
                .entity_mut(turret)
                .insert((ChildOf(ship), Transform::IDENTITY));
            app.world_mut().flush();

            // Nearly overhead off the bow: the barrel sits just short of the
            // limit, so the next target change pins it before yaw can slew
            // anywhere.
            let ahead = Vec3::new(0.0, 100.0, -3.0);
            app.world_mut()
                .entity_mut(turret)
                .insert(TurretSectionTargetInput(Some(ahead)));
            let muzzle = muzzle_entity(&app, turret);
            for _ in 0..240 {
                app.update();
            }
            let error = muzzle_aim_error_deg(&mut app, muzzle, ahead);
            assert!(error < 5.0, "setup: aim error {error} deg off the bow");

            // The target has crossed the zenith: opposite bearing, well down off
            // the vertical. Pitch is driven straight into its limit while yaw
            // still has a half-turn to go, so the hinge sits pinned at the pole.
            let behind = Vec3::new(0.0, 30.0, 30.0);
            app.world_mut()
                .entity_mut(turret)
                .insert(TurretSectionTargetInput(Some(behind)));
            for _ in 0..480 {
                app.update();
            }

            let error = muzzle_aim_error_deg(&mut app, muzzle, behind);
            assert!(
                error < 5.0,
                "turret must come back down onto a target that crossed the \
                 zenith, not park at full elevation: aim error {error} deg"
            );
        }
    }

    #[test]
    fn an_aimed_turret_holds_steady_without_shaking() {
        // NO-SHAKE REGRESSION: an undamped CCD
        // step (`target = output + delta`) made the coupled yaw+pitch overshoot
        // and settle into a ~4-5 deg limit cycle - the barrel visibly shook
        // around the aim. With the damping gain + deadband the muzzle must HOLD:
        // after it converges, its aim error must barely move frame to frame.
        let target = Vec3::new(8.0, 6.0, -28.0);
        let mut app = aim_convergence_app();
        let ship = app
            .world_mut()
            .spawn((SpaceshipRootMarker, Transform::IDENTITY))
            .id();
        let turret = app
            .world_mut()
            .spawn(turret_section(TurretSectionConfig {
                muzzle_speed: MetersPerSecond(10_000.0),
                ..Default::default()
            }))
            .id();
        app.world_mut()
            .entity_mut(turret)
            .insert((ChildOf(ship), Transform::IDENTITY));
        app.world_mut().flush();
        app.world_mut()
            .entity_mut(turret)
            .insert(TurretSectionTargetInput(Some(target)));
        let muzzle = muzzle_entity(&app, turret);

        // Converge.
        for _ in 0..240 {
            app.update();
        }

        // Then watch the aim error for 120 more frames: its peak-to-peak swing is
        // the shake amplitude. A limit cycle would read several degrees; a held
        // turret reads a fraction of one.
        let mut min = f32::INFINITY;
        let mut max = f32::NEG_INFINITY;
        for _ in 0..120 {
            app.update();
            let e = muzzle_aim_error_deg(&mut app, muzzle, target);
            min = min.min(e);
            max = max.max(e);
        }
        let peak_to_peak = max - min;
        assert!(
            peak_to_peak < 0.5,
            "an aimed turret must hold steady, not shake: aim error swings \
             {peak_to_peak} deg (min {min}, max {max})"
        );
    }

    /// Track a target crossing the bow (range 100 u, 15 u/s on +X) on a fixed
    /// `dt` clock for `sim_time` seconds, then return the residual aim error
    /// (deg) against the target's final position. One extra settle update
    /// precedes the measurement: the Update-schedule joint sync applies the
    /// PREVIOUS frame's controller output, so without it the measured pose
    /// would lag the last correction by one frame of the rate under test.
    fn crossing_residual_error_deg(dt: f32, sim_time: f32) -> f32 {
        use bevy::time::TimeUpdateStrategy;

        let target_at = |t: f32| Vec3::new(-30.0 + 15.0 * t, 0.0, -100.0);

        let mut app = aim_convergence_app();
        app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
            dt,
        )));
        let ship = app
            .world_mut()
            .spawn((SpaceshipRootMarker, Transform::IDENTITY))
            .id();
        let turret = app
            .world_mut()
            .spawn(turret_section(TurretSectionConfig::default()))
            .id();
        app.world_mut()
            .entity_mut(turret)
            .insert((ChildOf(ship), Transform::IDENTITY));
        app.world_mut().flush();
        let muzzle = muzzle_entity(&app, turret);

        // The first manual-clock update has dt 0; burn it before tracking.
        app.update();

        let steps = (sim_time / dt).round() as u32;
        for k in 1..=steps {
            let t = k as f32 * dt;
            app.world_mut()
                .entity_mut(turret)
                .insert(TurretSectionTargetInput(Some(target_at(t))));
            app.update();
        }
        app.update();

        muzzle_aim_error_deg(&mut app, muzzle, target_at(sim_time))
    }

    #[test]
    fn aim_tracking_lag_is_framerate_invariant() {
        // THE BUG: the correction gain damped the aim error per FRAME, so
        // residual tracking lag scaled with frame time - ~0.43 deg at 60 fps
        // vs ~1.8 deg at 14 fps on a crossing target, above the 0.92 deg fire
        // gate exactly when the machine struggled. With the dt-based decay,
        // the same SIM time against the same crosser must land the same
        // residual at any frame rate.
        //
        // Tolerance: the decay itself is exactly framerate-invariant; what
        // remains is sampling - the target moves w * dt between corrections,
        // so the residual measured right after a correction reads
        // (1 - gain) * w * dt / gain (~0.24 deg at 1/60, ~0.11 deg at 1/14
        // here), both bracketing the continuous-time lag w / rate. 0.5 deg
        // covers that bias; the pre-fix per-frame gain read ~1.0 deg at 1/14
        // in this harness and fails this assert.
        let at_60 = crossing_residual_error_deg(1.0 / 60.0, 4.0);
        let at_14 = crossing_residual_error_deg(1.0 / 14.0, 4.0);
        assert!(
            at_60 < 1.0 && at_14 < 1.0,
            "both rates must actually track the crosser: {at_60} deg at 1/60, \
             {at_14} deg at 1/14"
        );
        assert!(
            (at_60 - at_14).abs() < 0.5,
            "residual tracking lag must not depend on frame rate: {at_60} deg \
             at 1/60 vs {at_14} deg at 1/14"
        );
    }

    #[test]
    fn a_three_hinge_tree_reaches_a_target_a_two_dof_turret_cannot() {
        // MULTI-HINGE: a hand-built 3-hinge arm (Y at base, then X, then Y two
        // down) converges onto a target, sanity that arbitrary chains solve.
        let mut app = aim_convergence_app();
        let ship = app
            .world_mut()
            .spawn((SpaceshipRootMarker, Transform::IDENTITY))
            .id();

        // Y -> X -> Y -> muzzle, each a link one unit long, generous limits.
        let hinge = |axis: Vec3, offset: Vec3, children: Vec<TurretJoint>| TurretJoint {
            name: None,
            offset,
            axis: Some(axis),
            speed: std::f32::consts::TAU,
            min: Some(-std::f32::consts::PI),
            max: Some(std::f32::consts::PI),
            render_mesh: None,
            render_mesh_transform: None,
            muzzle: None,
            children,
        };
        let root = hinge(
            Vec3::Y,
            Vec3::ZERO,
            vec![hinge(
                Vec3::X,
                Vec3::new(0.0, 1.0, 0.0),
                vec![hinge(
                    Vec3::Y,
                    Vec3::new(0.0, 1.0, 0.0),
                    vec![TurretJoint {
                        name: None,
                        offset: Vec3::new(0.0, 0.0, -1.0),
                        axis: None,
                        speed: std::f32::consts::PI,
                        min: None,
                        max: None,
                        render_mesh: None,
                        render_mesh_transform: None,
                        muzzle: Some(MuzzleConfig {
                            id: "main".to_string(),
                            fire_rate: 10.0,
                            muzzle_effect: None,
                        }),
                        children: vec![],
                    }],
                )],
            )],
        );

        let config = TurretSectionConfig {
            root,
            muzzle_speed: MetersPerSecond(10_000.0),
            ..default()
        };
        let turret = app.world_mut().spawn(turret_section(config)).id();
        app.world_mut()
            .entity_mut(turret)
            .insert((ChildOf(ship), Transform::IDENTITY));
        app.world_mut().flush();

        let target = Vec3::new(6.0, -4.0, 8.0); // behind + below: needs the extra DOF
        app.world_mut()
            .entity_mut(turret)
            .insert(TurretSectionTargetInput(Some(target)));
        let muzzle = muzzle_entity(&app, turret);

        for _ in 0..600 {
            app.update();
        }

        let error = muzzle_aim_error_deg(&mut app, muzzle, target);
        assert!(
            error < 8.0,
            "a 3-hinge arm must converge onto {target:?}, aim error {error} deg"
        );
    }

    #[test]
    fn a_shared_chain_twin_barrel_points_both_muzzles_at_the_target() {
        // MULTI-MUZZLE AIM: a twin barrel that shares its
        // yaw+pitch chain (two muzzles hanging off one common barrel) steers BOTH
        // muzzles onto the target - the shared joints are written by each muzzle's
        // CCD pass and agree for the symmetric barrels.
        let mut app = aim_convergence_app();
        let ship = app
            .world_mut()
            .spawn((SpaceshipRootMarker, Transform::IDENTITY))
            .id();

        // base(fixed) -> yaw(Y) -> pitch(X) -> barrel(fixed) with TWO muzzles.
        let muzzle = |x: f32| TurretJoint {
            name: None,
            offset: Vec3::new(x, 0.0, -0.5),
            axis: None,
            speed: default_joint_speed(),
            min: None,
            max: None,
            render_mesh: None,
            render_mesh_transform: None,
            muzzle: Some(MuzzleConfig {
                id: "main".to_string(),
                fire_rate: 10.0,
                muzzle_effect: None,
            }),
            children: vec![],
        };
        let yaw = TurretJoint {
            name: None,
            offset: Vec3::ZERO,
            axis: Some(Vec3::Y),
            speed: std::f32::consts::TAU,
            min: Some(-std::f32::consts::PI),
            max: Some(std::f32::consts::PI),
            render_mesh: None,
            render_mesh_transform: None,
            muzzle: None,
            children: vec![TurretJoint {
                name: None,
                offset: Vec3::ZERO,
                axis: Some(Vec3::X),
                speed: std::f32::consts::TAU,
                min: Some(-std::f32::consts::PI),
                max: Some(std::f32::consts::PI),
                render_mesh: None,
                render_mesh_transform: None,
                muzzle: None,
                children: vec![muzzle(0.1), muzzle(-0.1)],
            }],
        };

        let config = TurretSectionConfig {
            root: yaw,
            muzzle_speed: MetersPerSecond(10_000.0), // near-straight lead so aim ~= target dir
            ..default()
        };
        let turret = app.world_mut().spawn(turret_section(config)).id();
        app.world_mut()
            .entity_mut(turret)
            .insert((ChildOf(ship), Transform::IDENTITY));
        app.world_mut().flush();

        let target = Vec3::new(10.0, 5.0, -30.0);
        app.world_mut()
            .entity_mut(turret)
            .insert(TurretSectionTargetInput(Some(target)));
        let muzzles = app
            .world()
            .entity(turret)
            .get::<TurretSectionMuzzles>()
            .expect("the turret records its muzzles")
            .0
            .clone();
        assert_eq!(muzzles.len(), 2, "the twin barrel must record two muzzles");

        for _ in 0..240 {
            app.update();
        }

        for muzzle in muzzles {
            let error = muzzle_aim_error_deg(&mut app, muzzle, target);
            assert!(
                error < 5.0,
                "both muzzles must converge onto {target:?}, muzzle {muzzle:?} \
                 aim error {error} deg"
            );
        }
    }
}
