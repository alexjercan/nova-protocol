//! A DOCK approach's corridor: where on the station port's axis the approach
//! may start its final leg, and the one shape cast that says the whole hull
//! can close from there to the berth without touching anything.
//!
//! Engine units throughout, like every avian `Position` the cast reads.

use avian3d::prelude::*;
use bevy::{ecs::system::SystemParam, prelude::*};
use nova_events::prelude::EntityId;

use super::autopilot::DETOUR_HYSTERESIS;
use crate::{prelude::*, sections::docking_section::BERTH_GAP};

/// The separation, engine units (2.5 m), inside which the corridor sweep
/// counts a collider as hit. Below [`BERTH_GAP`], so station plating flush
/// with the host port's face, one berth gap from a hull whose port face is
/// its foremost point, is not a hit. A hull part reaching past its own port
/// into that gap, or plating standing proud of the port, is.
const DOCK_CORRIDOR_SKIN: f32 = 0.25;

/// How far the two port axes may be off opposed while leg B flies, radians:
/// the gate the final leg starts on. The sweep tilts the hull by this much.
pub(super) const DOCK_FINAL_LEG_ANGLE: f32 = 10.0_f32.to_radians();

/// How far the hull's port may sit off the station port's axis while leg B
/// flies, engine units: the gate the final leg starts on. The sweep widens
/// the hull by this much.
pub(super) const DOCK_FINAL_LEG_LATERAL: f32 = 0.3;

/// The berth gap a DOCK approach's final leg rests at: the scripted berth gap,
/// or the pair's capture distance where that is tighter.
pub(super) fn dock_berth_gap(pair: &DockingPair) -> f32 {
    BERTH_GAP.min(pair.envelope.capture_distance)
}

/// A hull measured about one of its own docking ports, in that port's frame:
/// how far the hull reaches off the port axis, and how far in front of and
/// behind the port face it reaches. Roll about the port axis does not change
/// any of it, which is why it can be swept while the approach leaves roll
/// free.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct PortEnvelope {
    /// The furthest any collider corner stands off the port axis.
    pub(super) radius: f32,
    /// The least depth behind the port face of any corner; negative where the
    /// hull reaches past its own port.
    pub(super) front: f32,
    /// The greatest depth behind the port face of any corner.
    pub(super) back: f32,
    /// The furthest any corner stands from the port face's centre: the lever a
    /// tilt of the hull swings it sideways through.
    pub(super) reach: f32,
    /// The centre of mass's distance off the port axis.
    pub(super) com_radius: f32,
    /// The centre of mass's depth behind the port face.
    pub(super) com_depth: f32,
}

impl PortEnvelope {
    /// Measure `corners`, given with the port face centre `face` and the
    /// outward port `axis`, all in one frame.
    pub(super) fn measure(
        face: Vec3,
        axis: Vec3,
        center_of_mass: Vec3,
        corners: impl IntoIterator<Item = Vec3>,
    ) -> Option<Self> {
        let depth_and_radius = |point: Vec3| {
            let offset = point - face;
            let along = offset.dot(axis);
            (-along, (offset - axis * along).length(), offset.length())
        };
        let (com_depth, com_radius, _) = depth_and_radius(center_of_mass);
        let mut envelope: Option<Self> = None;
        for corner in corners {
            let (depth, radius, reach) = depth_and_radius(corner);
            let next = envelope.get_or_insert(Self {
                radius,
                front: depth,
                back: depth,
                reach,
                com_radius,
                com_depth,
            });
            next.radius = next.radius.max(radius);
            next.front = next.front.min(depth);
            next.back = next.back.max(depth);
            next.reach = next.reach.max(reach);
        }
        envelope
    }

    /// The swept volume of the hull at any roll: a cylinder on the port axis,
    /// as its port-offset allowance, its radius, and its front and back depths
    /// behind the port face.
    ///
    /// The allowance, and so the radius, takes the sideways swing of a hull tilted up to
    /// [`DOCK_FINAL_LEG_ANGLE`] and offset up to [`DOCK_FINAL_LEG_LATERAL`],
    /// the gates leg B starts on. The length is the hull's own. A tilt also
    /// moves one rim point of the front forward by up to `radius * sin`, and
    /// that is not swept: the hull holds the axes opposed as it closes, so its
    /// tilt at the berth is far under the gate, and the berth gap less
    /// [`DOCK_CORRIDOR_SKIN`] is the margin left for it.
    fn swept(&self) -> (f32, f32, f32, f32) {
        let lateral = self.reach * DOCK_FINAL_LEG_ANGLE.tan() + DOCK_FINAL_LEG_LATERAL;
        (lateral, self.radius + lateral, self.front, self.back)
    }
}

/// How far out on the station port's axis the corridor starts, engine units
/// from the station port's face to the hull's: the nearest point at which the
/// centre of mass of the hull, with its axes opposed and at any roll, stands
/// outside the bubble a GOTO leg keeps around the station, so the leg planner
/// flies to it unmoved once the hull has turned. Never inside the berth gap.
///
/// `face` and `axis` are the station port's, `bubble` the station's centre of
/// mass and containment radius, and `clearance` the leg's clearance outside
/// it, all in one frame.
pub(super) fn dock_entry(
    face: Vec3,
    axis: Vec3,
    bubble: (Vec3, f32),
    clearance: f32,
    hull: &PortEnvelope,
    berth_gap: f32,
) -> f32 {
    let (center, radius) = bubble;
    // The leg planner pushes a goal out to its clear clearance plus one unit;
    // the entry stands at that push, plus the centre of mass's swing off the
    // axis.
    let keep = radius + clearance + DETOUR_HYSTERESIS + 1.0 + hull.com_radius;
    let from_center = face - center;
    let along = from_center.dot(axis);
    let off_axis_sq = from_center.length_squared() - along * along;
    let reach_sq = keep * keep - off_axis_sq;
    if reach_sq <= 0.0 {
        return berth_gap;
    }
    // The far root of |from_center + axis * t| = keep: past it, every point on
    // the axis is outside.
    let com_along = -along + reach_sq.sqrt();
    (com_along - hull.com_depth).max(berth_gap)
}

/// The collider that refused a corridor: what it is and whose.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct DockBlocker {
    /// The collider entity the sweep met.
    pub(super) collider: Entity,
    /// Its name, where it has one.
    pub(super) name: Option<String>,
    /// Its design-local section id, where the spawn path gave it one.
    pub(super) id: Option<String>,
    /// The body it belongs to, where it is attached to one.
    pub(super) body: Option<Entity>,
    /// How far along the corridor it was met, engine units from the entry.
    pub(super) distance: f32,
}

impl core::fmt::Display for DockBlocker {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "collider {:?} (name {:?}, id {:?}) of body {:?}, {:.2} u along the corridor",
            self.collider, self.name, self.id, self.body, self.distance
        )
    }
}

/// The world reads a DOCK approach's corridor needs, beside the docking ports
/// themselves.
#[derive(SystemParam)]
pub(super) struct DockCorridors<'w, 's> {
    /// Free ports, their poses and the two bodies.
    pub(super) ports: DockingPorts<'w, 's>,
    /// The collider tree the corridor is swept through.
    spatial: SpatialQuery<'w, 's>,
    /// Every collider attached to a body: the hull being measured, and the
    /// colliders the sweep leaves out.
    body_colliders: Query<'w, 's, &'static RigidBodyColliders>,
    /// A collider's shape and its pose in its body's frame.
    colliders: Query<'w, 's, (&'static Collider, &'static ColliderTransform), Without<Sensor>>,
    /// Sensors overlap and never touch, so the sweep passes through them.
    sensors: Query<'w, 's, (), With<Sensor>>,
    /// Which body a collider the sweep met belongs to.
    collider_of: Query<'w, 's, &'static ColliderOf>,
    /// A body's centre of mass in its own frame.
    centers_of_mass: Query<'w, 's, &'static ComputedCenterOfMass>,
    /// A hull's containment radius about its centre of mass.
    envelopes: Query<'w, 's, &'static HullEnvelopeRadius>,
    /// What a blocker is called.
    labels: Query<'w, 's, (Option<&'static Name>, Option<&'static EntityId>)>,
}

impl DockCorridors<'_, '_> {
    /// The station's bubble: its centre of mass in the world and its
    /// containment radius. `None` while the station has no measured hull.
    pub(super) fn bubble(&self, station: Entity) -> Option<(Vec3, f32)> {
        let (position, rotation) = self.ports.body_pose(station)?;
        let center_of_mass = self.centers_of_mass.get(station).ok()?;
        let radius = self.envelopes.get(station).ok()?;
        Some((position + rotation * center_of_mass.0, **radius))
    }

    /// `ship` measured about its port `pair.first`, in the world frame of this
    /// tick's pose. `None` when the hull has no solid collider.
    fn port_envelope(&self, ship: Entity, pair: &DockingPair) -> Option<PortEnvelope> {
        let (position, rotation) = self.ports.body_pose(ship)?;
        let center_of_mass = self.centers_of_mass.get(ship).ok()?;
        let colliders = self.body_colliders.get(ship).ok()?;
        let corners = colliders
            .iter()
            .filter_map(|collider| self.colliders.get(collider).ok())
            .flat_map(|(collider, pose)| {
                // The shape's own box, so a cuboid section is measured
                // exactly; avian has already scaled the shape.
                let local = collider.aabb(Vec3::ZERO, Quat::IDENTITY);
                let (min, max) = (local.min, local.max);
                (0..8).map(move |corner: u8| {
                    let point = Vec3::new(
                        if corner & 1 == 0 { min.x } else { max.x },
                        if corner & 2 == 0 { min.y } else { max.y },
                        if corner & 4 == 0 { min.z } else { max.z },
                    );
                    position + rotation * (pose.translation + pose.rotation.0 * point)
                })
            });
        PortEnvelope::measure(
            pair.first.face,
            pair.first.axis,
            position + rotation * center_of_mass.0,
            corners,
        )
    }

    /// Plan `pair`'s corridor: the entry on the station port's axis, and one
    /// shape cast of the swept hull from there to the berth gap.
    ///
    /// The cast leaves out only the station's port section and every collider
    /// of `ship`. Anything else it meets refuses the pair: a station section
    /// beside the port, a turret, a docked hull, a rock.
    ///
    /// # Errors
    ///
    /// The first collider the sweep met, or `None` when the hull or the
    /// station cannot be measured.
    pub(super) fn plan(
        &self,
        ship: Entity,
        pair: &DockingPair,
        clearance: f32,
    ) -> Result<DockPlan, Option<DockBlocker>> {
        let bubble = self.bubble(pair.second_ship).ok_or(None)?;
        let hull = self.port_envelope(ship, pair).ok_or(None)?;
        let berth_gap = dock_berth_gap(pair);
        let entry = dock_entry(
            pair.second.face,
            pair.second.axis,
            bubble,
            clearance,
            &hull,
            berth_gap,
        );
        let (lateral, radius, front, back) = hull.swept();
        let axis = pair.second.axis;
        let start = pair.second.face + axis * (entry + (front + back) * 0.5);
        let shape = Collider::cylinder(radius, back - front);
        let excluded = self
            .body_colliders
            .get(ship)
            .into_iter()
            .flat_map(|colliders| colliders.iter())
            .chain([pair.second_section]);
        let filter = SpatialQueryFilter::from_excluded_entities(excluded);
        let config = ShapeCastConfig {
            max_distance: entry - berth_gap,
            target_distance: DOCK_CORRIDOR_SKIN,
            compute_contact_on_penetration: true,
            ignore_origin_penetration: false,
        };
        let hit = self.spatial.cast_shape_predicate(
            &shape,
            start,
            Quat::from_rotation_arc(Vec3::Y, axis),
            Dir3::new(-axis).map_err(|_| None)?,
            &config,
            &filter,
            &|collider| !self.sensors.contains(collider),
        );
        match hit {
            None => Ok(DockPlan {
                own_port: pair.first_section,
                station_port: pair.second_section,
                entry,
                lateral,
            }),
            Some(hit) => {
                let (name, id) = self.labels.get(hit.entity).unwrap_or((None, None));
                Err(Some(DockBlocker {
                    collider: hit.entity,
                    name: name.map(|name| name.to_string()),
                    id: id.map(|id| id.0.clone()),
                    body: self.collider_of.get(hit.entity).ok().map(|of| of.body),
                    distance: hit.distance,
                }))
            }
        }
    }
}
