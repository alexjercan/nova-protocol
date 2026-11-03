//! A DOCK approach's obstacle forecast in its station's frame.
//!
//! Leg A and the corridor fly in the frame that moves with the station: the
//! command is the station's velocity plus a capped closing velocity, and a
//! detour corner moves with the station. A body the station moves past is
//! not where its snapshot says for long in that frame. A station orbiting
//! close to a planet sees the planet circle it once per orbit, at the orbit
//! speed, which is several times the approach's closing cap, so a leg planned
//! against the snapshot puts the hull in the planet's path.
//!
//! The forecast steps the station and every field body for
//! [`DOCK_FORECAST_HORIZON`] under the dominant-well pull the physics applies
//! ([`dominant_well_acceleration`]) and keeps each body's path relative to the
//! station as a row of spheres. That set does not move in the station's frame,
//! so leg A plans around it with a corner that moves with the station, and the
//! corridor is refused where the set comes within the leg's clearance. A
//! planet, which is static, comes out as the arc (or ring) of its orbit about
//! the station.
//!
//! Engine units throughout.

use bevy::prelude::*;
use nova_gameplay::prelude::*;

/// How far ahead the forecast steps the station and the field, seconds: past a
/// generated station's ~73 s orbit, so a planet that station circles closes
/// its ring, and about the time a hull at the 2 u/s closing cap takes to
/// clear a planet-sized body.
pub(super) const DOCK_FORECAST_HORIZON: f32 = 90.0;

/// The forecast's integration step, seconds. Semi-implicit Euler at 1 s turns
/// a 73 s circular orbit through 0.086 rad a step and stays on it.
const DOCK_FORECAST_STEP: f32 = 1.0;

/// How often a DOCK approach steps its forecast again, seconds. The set is
/// held in the station's frame, so a held set lags the field by at most this
/// much and still covers every later position up to its horizon.
pub(super) const DOCK_FORECAST_REFRESH: f32 = 1.0;

/// About the most spheres one body's path keeps: a path longer than this
/// many quarter radii is kept at a wider spacing, with each sphere grown to
/// cover the gap. The last point is kept too, so a path can keep a few more.
const DOCK_FORECAST_SPHERES_MAX: usize = 64;

/// A sized field body as the forecast reads it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct FieldBody {
    /// The body entity.
    pub(super) entity: Entity,
    /// Its centre, world frame.
    pub(super) center: Vec3,
    /// Its geometric radius.
    pub(super) radius: f32,
    /// Its velocity; `None` for a body that does not move (a static planet).
    pub(super) velocity: Option<Vec3>,
    /// Whether the wells pull it.
    pub(super) feels_wells: bool,
}

/// One sphere of a body's forecast path: its body, its centre relative to the
/// station's centre of mass when the forecast was made, and its radius.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct ForecastSphere {
    /// The body whose path this is.
    pub(super) body: Entity,
    /// The centre, relative to the station's centre of mass.
    pub(super) offset: Vec3,
    /// The radius: the body's own, grown to cover the gap to its neighbours.
    pub(super) radius: f32,
}

/// A DOCK approach's held forecast.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct DockForecast {
    /// The station it was made for.
    pub(super) station: Entity,
    /// When it was made, seconds of fixed time.
    pub(super) made_at: f32,
    /// The station's rotation when it was made.
    pub(super) rotation: Quat,
    /// The station's mean angular velocity, radians per second, over the
    /// time since the forecast before it: its rotation change over that
    /// time, zero for the first forecast to a station. Avian's
    /// `AngularVelocity` on a live station changes axis and sign from one
    /// second to the next by up to about 1 deg/s; led with directly, it moves
    /// the frame 0.6 u/s at 40 u and the RCS chases that until the
    /// magazine is empty. A steady spin keeps its full lead.
    pub(super) spin: Vec3,
    /// Every path sphere that can reach the approach.
    pub(super) spheres: Vec<ForecastSphere>,
}

impl DockForecast {
    /// The station's mean spin from `earlier` at `earlier_at` to `rotation`
    /// at `now`, radians per second, taken the short way round.
    pub(super) fn mean_spin(earlier: Quat, earlier_at: f32, rotation: Quat, now: f32) -> Vec3 {
        let elapsed = now - earlier_at;
        if elapsed <= 0.0 {
            return Vec3::ZERO;
        }
        let turn = rotation * earlier.inverse();
        let turn = if turn.w < 0.0 { -turn } else { turn };
        turn.to_scaled_axis() / elapsed
    }

    /// The spheres in the world frame of this tick: each offset from the
    /// station's centre of mass `station_center` now.
    pub(super) fn world(&self, station_center: Vec3) -> Vec<(Entity, Vec3, f32)> {
        self.spheres
            .iter()
            .map(|sphere| (sphere.body, station_center + sphere.offset, sphere.radius))
            .collect()
    }
}

/// One body stepped under the dominant-well pull.
struct Stepper {
    position: Vec3,
    velocity: Vec3,
    dominant: Option<Entity>,
}

impl Stepper {
    fn step(
        &mut self,
        wells: &[(Entity, Vec3, GravityWell)],
        gravity: &GravitySettings,
        scratch: &mut (Vec<(Entity, f32, Vec3)>, Vec<(Entity, f32)>),
    ) {
        let pull = dominant_well_acceleration(
            self.position,
            self.dominant,
            wells,
            gravity,
            &mut scratch.0,
            &mut scratch.1,
        );
        self.dominant = pull.map(|(well, _)| well);
        if let Some((_, acceleration)) = pull {
            self.velocity += acceleration * DOCK_FORECAST_STEP;
        }
        self.position += self.velocity * DOCK_FORECAST_STEP;
    }
}

/// Every field body's path relative to the station over
/// [`DOCK_FORECAST_HORIZON`], as spheres offset from the station's centre of
/// mass `station` now, which moves at `station_velocity` and falls in
/// `wells`. A body whose path never comes within `reach` of the station,
/// plus its sphere radius, is left out.
///
/// Kept spheres stand a quarter of the body's radius apart, or wider where
/// the path would need more than [`DOCK_FORECAST_SPHERES_MAX`]. Each takes the
/// radius `sqrt(r^2 + (gap / 2)^2)` of the widest gap, so two neighbours
/// cover the tube of the body's own radius `r` between them.
pub(super) fn station_frame_sweep(
    station: Vec3,
    station_velocity: Vec3,
    bodies: &[FieldBody],
    wells: &[(Entity, Vec3, GravityWell)],
    gravity: &GravitySettings,
    reach: f32,
) -> Vec<ForecastSphere> {
    let steps = (DOCK_FORECAST_HORIZON / DOCK_FORECAST_STEP).round() as usize;
    let mut scratch = (Vec::new(), Vec::new());
    let mut hull = Stepper {
        position: station,
        velocity: station_velocity,
        dominant: None,
    };
    // How far the station has moved at each step.
    let mut moved = Vec::with_capacity(steps + 1);
    moved.push(Vec3::ZERO);
    for _ in 0..steps {
        hull.step(wells, gravity, &mut scratch);
        moved.push(hull.position - station);
    }

    let mut spheres = Vec::new();
    let mut path = Vec::with_capacity(steps + 1);
    for body in bodies {
        path.clear();
        let mut stepper = Stepper {
            position: body.center,
            velocity: body.velocity.unwrap_or(Vec3::ZERO),
            dominant: None,
        };
        let moves = body.velocity.is_some();
        for shift in &moved {
            path.push(stepper.position - station - *shift);
            if moves {
                if body.feels_wells {
                    stepper.step(wells, gravity, &mut scratch);
                } else {
                    stepper.position += stepper.velocity * DOCK_FORECAST_STEP;
                }
            }
        }
        sample_path(body, &path, reach, &mut spheres);
    }
    spheres
}

/// Keep `path`'s spheres for `body` in `spheres`, when any of them comes
/// within `reach` of the station.
fn sample_path(body: &FieldBody, path: &[Vec3], reach: f32, spheres: &mut Vec<ForecastSphere>) {
    let length: f32 = path.windows(2).map(|pair| pair[0].distance(pair[1])).sum();
    let spacing = (body.radius * 0.25).max(length / DOCK_FORECAST_SPHERES_MAX as f32);
    let mut kept = Vec::new();
    let mut widest = 0.0f32;
    let mut last = path[0];
    kept.push(last);
    for (index, point) in path.iter().enumerate().skip(1) {
        let gap = point.distance(last);
        if gap >= spacing || index == path.len() - 1 {
            widest = widest.max(gap);
            kept.push(*point);
            last = *point;
        }
    }
    let radius = (body.radius * body.radius + widest * widest * 0.25).sqrt();
    if kept.iter().all(|offset| offset.length() > reach + radius) {
        return;
    }
    spheres.extend(kept.into_iter().map(|offset| ForecastSphere {
        body: body.entity,
        offset,
        radius,
    }));
}

/// How many steps [`swept_exit`] takes at most.
const SWEPT_EXIT_STEPS: usize = 16;

/// The nearest point clear of every sphere in `spheres` by `clearance`, from
/// `from` inside their union.
///
/// A hull inside a forecast path is where that body will stand within the
/// horizon, and every leg from it is blocked. Pushing out of one sphere at a
/// time lands inside the next one along a path. A step instead follows the
/// depth-weighted sum of the pushes out of every sphere that holds the point,
/// as long as the deepest push. Along a path of overlapping spheres (a
/// planet's ring) the pushes along the path cancel, so the steps cross it.
/// Bounded: a set with no way out returns the last point.
pub(super) fn swept_exit(from: Vec3, clearance: f32, spheres: &[(Vec3, f32)]) -> Vec3 {
    let mut point = from;
    for _ in 0..SWEPT_EXIT_STEPS {
        let mut push = Vec3::ZERO;
        let mut deepest = 0.0f32;
        for &(center, radius) in spheres {
            let offset = point - center;
            let depth = radius + clearance - offset.length();
            if depth > 0.0 {
                push += offset.normalize_or(Vec3::Y) * depth;
                deepest = deepest.max(depth);
            }
        }
        if deepest <= 0.0 {
            return point;
        }
        point += push.normalize_or(Vec3::Y) * (deepest + 1.0);
    }
    point
}
