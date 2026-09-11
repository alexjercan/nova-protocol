//! ONE sensor pass per observing ship per frame, and the policy every
//! consumer of it shares.
//!
//! The player's lock upkeep, the player's radar picker and the AI's target
//! acquisition all ask the same question - what can this ship see from where
//! it is standing - and used to answer it three times with three sets of
//! rules: two range models, two position conventions, and a line-of-sight ray
//! cast once per collection pass. A ship's answers could disagree with each
//! other within one frame, and a modded hull that was lockable to a player was
//! not necessarily visible to an AI picket beside it.
//!
//! So the pass runs once per OBSERVER and publishes [`SensorContacts`] on it.
//! What a consumer then does with a contact is its own business - a travel
//! designation and a combat lock apply different policies to the same
//! sighting - but they can no longer disagree about the sighting itself.
//!
//! ENGINE UNITS throughout, as everywhere under `input/`: every range here is
//! compared against an avian position each frame, so it is world units (one is
//! 10 m). A scenario authors in metres and the loader crosses the seam once.

use avian3d::prelude::*;
use bevy::prelude::*;
use nova_gameplay::prelude::*;

use super::occlusion::RadarScan;
use crate::prelude::*;

/// The player's sensor reach, world units: 200 km.
///
/// The player's lock doubles as a long-range DESIGNATOR - a GOTO leg is
/// plotted on it - so the ceiling is the play area rather than a fighting
/// distance. What is actually visible at that reach is the signature model's
/// business, not this cap's.
pub const PLAYER_SENSOR_RANGE: f32 = 20_000.0;

/// The reach an AI ship gets when its controller authors none, world units:
/// 20 km. A pilot only needs to find things worth fighting.
pub const AI_SENSOR_RANGE: f32 = 2_000.0;

/// How far one observing ship can see, world units.
///
/// The observer's half of the range model: a contact is visible inside
/// `min(this, what the target's own class returns)`. Present on every ship
/// that observes; a ship without one is blind, which is what a hulk with no
/// live controller is.
#[derive(Component, Clone, Copy, Debug, PartialEq, Deref, DerefMut, Reflect)]
#[reflect(Component)]
pub struct SensorRange(pub f32);

/// One body an observing ship can see this frame.
///
/// `in_sight` is REPORTED rather than applied: a contact behind cover stays in
/// the set. Both lock slots then drop it - a designation is the same radio
/// link a weapons lock is - but the upkeep needs the difference between a body
/// it cannot SEE and a body that is no longer there, because those are two
/// different drop branches and the flight log names them apart.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SensorContact {
    /// The body the contact names - the root a lock is written with.
    pub entity: Entity,
    /// Where it is, measured on its live structure rather than its root
    /// origin, so a hull that lost sections is seen where its metal is.
    pub anchor: Vec3,
    /// What the observer is to it.
    pub relation: Relation,
    /// A ship root.
    pub is_ship: bool,
    /// A committed torpedo.
    pub is_torpedo: bool,
    /// Already neutralized - collected, because the HUD still names a
    /// surrendered ship, and skipped by anything that picks a fight.
    pub neutralized: bool,
    /// Nothing that stops radar stands on the line to it.
    pub in_sight: bool,
}

impl SensorContact {
    /// A thing to fight: a ship or a committed torpedo. An asteroid, a planet
    /// and a nav beacon are contacts a pilot navigates by, not targets.
    pub fn is_combat_target(&self) -> bool {
        self.is_ship || self.is_torpedo
    }

    /// Hostile to the observer.
    pub fn is_hostile(&self) -> bool {
        self.relation == Relation::Hostile
    }
}

/// Everything one observing ship can see this frame, published by
/// [`update_sensor_contacts`] before anything reads it.
///
/// Rebuilt in place each frame rather than reallocated: every observer in a
/// busy scene publishes one of these per frame.
#[derive(Component, Clone, Debug, Default, PartialEq)]
pub struct SensorContacts {
    /// The observer's own anchor the contacts were measured from, so a
    /// consumer that needs a distance does not re-derive the origin and get a
    /// different one.
    pub origin: Vec3,
    entries: Vec<SensorContact>,
    /// The range each body this ship is HOLDING returned when the hold
    /// started, world units. A signature shrinks as a hull is shot apart, and
    /// a lock taken on a whole ship must not be let go of because the ship the
    /// player is shooting is now smaller than it was: the held gate is the
    /// larger of the fresh range and this floor. One short list per observer,
    /// pruned every frame to what it is still holding.
    floors: Vec<(Entity, f32)>,
}

impl SensorContacts {
    /// Every contact this frame.
    pub fn iter(&self) -> impl Iterator<Item = &SensorContact> {
        self.entries.iter()
    }

    /// The contact for `target`, if the observer can see it at all.
    pub fn get(&self, target: Entity) -> Option<&SensorContact> {
        self.entries.iter().find(|contact| contact.entity == target)
    }

    /// Whether `target` is visible AND has a clear line - what a live radio
    /// link needs.
    pub fn in_sight(&self, target: Entity) -> bool {
        self.get(target).is_some_and(|contact| contact.in_sight)
    }
}

impl FromIterator<SensorContact> for SensorContacts {
    /// Build a contact set directly, for a rig that stages SIGHTINGS rather
    /// than a world. The origin is left at zero: a consumer that needs one
    /// takes it as an argument.
    fn from_iter<T: IntoIterator<Item = SensorContact>>(iter: T) -> Self {
        Self {
            origin: Vec3::ZERO,
            entries: iter.into_iter().collect(),
            floors: Vec::new(),
        }
    }
}

/// The scanner query one pass walks. Turret bullets are excluded outright:
/// they are dynamic bodies that stream straight down the aim ray.
type CandidateQuery<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static Transform,
        Option<&'static ComputedCenterOfMass>,
        &'static RigidBody,
        Option<&'static GravityWell>,
        Option<&'static LockSignature>,
        Has<SpaceshipRootMarker>,
        Option<&'static TorpedoProjectileMarker>,
        Option<&'static TorpedoTargetChosen>,
        Option<&'static Allegiance>,
        Has<NeutralizedMarker>,
    ),
    Without<TurretBulletProjectileMarker>,
>;

/// The observing ships one pass writes.
type ObserverQuery<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static Transform,
        Option<&'static ComputedCenterOfMass>,
        Option<&'static Allegiance>,
        &'static SensorRange,
        &'static mut SensorContacts,
        Option<&'static TravelLock>,
        Option<&'static CombatLock>,
        Option<&'static RadarState>,
    ),
    (With<SpaceshipRootMarker>, Without<SensorsDark>),
>;

/// How far `observer` can see this body, world units.
///
/// The observer's cap and the target's own RETURN, whichever gives up first.
/// There are no classes left in this rule: a ship, a rock, a planet, a beacon
/// and a committed torpedo each publish a [`LockSignature`] computed from what
/// they are, and this turns that signature into a distance. Only unsigned
/// wreckage has no return of its own, and it stays point-blank.
fn class_range(settings: &TargetingSettings, cap: f32, signature: Option<f32>) -> f32 {
    let returned = signature.map_or(settings.unsigned_lock_range, |signature| {
        (settings.signature_range_per_unit * signature).max(settings.unsigned_lock_range)
    });
    cap.min(returned)
}

/// Collect what every observing ship can see, once per ship per frame.
///
/// - Only physical, movable bodies are contacts. This skips static sensor
///   volumes such as scenario trigger areas, which are invisible and must
///   never be locked. Two exceptions sit on rails yet are visible things a
///   pilot navigates by: gravity-well sources and bodies with an AUTHORED
///   signature (nav beacons) - a trigger area never carries a signature, so
///   the invisible-statics rule holds.
/// - A freshly launched torpedo that has not committed its target yet is
///   skipped: it spawns right on the aim ray.
/// - Range is [`class_range`]. For a body the observer is already HOLDING it
///   is the larger of that and the range the body returned when the hold
///   began, widened by [`TargetingSettings::range_hysteresis`]: a body at the
///   boundary cannot strobe a lock as the ship drifts, and a target that has
///   been shot smaller than it was cannot silently fall out of a lock taken
///   while it was whole. Fresh acquisition always uses the plain gate, so
///   damage does shorten the range a NEW lock can be taken at.
/// - Line of sight is asked LAST, after the cheap component and range
///   rejects, so a frame pays for one ray per body the ship could otherwise
///   see rather than one per body in the world.
pub(crate) fn update_sensor_contacts(
    scan: RadarScan,
    settings: Res<TargetingSettings>,
    q_candidates: CandidateQuery,
    mut q_observers: ObserverQuery,
) {
    for (
        observer,
        transform,
        com,
        observer_allegiance,
        range,
        mut contacts,
        travel,
        combat,
        radar,
    ) in &mut q_observers
    {
        // The anchor on the live structure, not the root origin, so the
        // scanner agrees with the COM-anchored crosshair after losing
        // sections.
        let origin = live_structure_anchor(transform, com);
        let incumbents = [
            travel.and_then(|lock| lock.0),
            combat.and_then(|lock| lock.0),
            radar.and_then(|radar| radar.candidate),
        ];

        let mut entries = std::mem::take(&mut contacts.entries);
        entries.clear();
        // Only what the ship still holds keeps a floor: a released lock
        // re-acquires at whatever the target returns now.
        let mut floors = std::mem::take(&mut contacts.floors);
        floors.retain(|(entity, _)| incumbents.contains(&Some(*entity)));
        entries.extend(q_candidates.iter().filter_map(
            |(
                entity,
                c_transform,
                c_com,
                rigid_body,
                well,
                signature,
                is_ship,
                torpedo,
                committed,
                allegiance,
                neutralized,
            )| {
                if entity == observer {
                    return None;
                }
                if !matches!(rigid_body, RigidBody::Dynamic)
                    && well.is_none()
                    && signature.is_none()
                {
                    return None;
                }
                let is_torpedo = torpedo.is_some();
                if is_torpedo && committed.is_none() {
                    return None;
                }
                let fresh = class_range(&settings, **range, signature.map(|signature| **signature));
                let max_range = if incumbents.contains(&Some(entity)) {
                    let floor = match floors.iter_mut().find(|(held, _)| *held == entity) {
                        Some((_, floor)) => *floor,
                        None => {
                            floors.push((entity, fresh));
                            fresh
                        }
                    };
                    fresh.max(floor) * settings.range_hysteresis.max(1.0)
                } else {
                    fresh
                };
                let anchor = live_structure_anchor(c_transform, c_com);
                if anchor.distance_squared(origin) > max_range * max_range {
                    return None;
                }
                Some(SensorContact {
                    entity,
                    anchor,
                    relation: relation(observer_allegiance, allegiance),
                    is_ship,
                    is_torpedo,
                    neutralized,
                    in_sight: !scan.is_occluded(observer, origin, anchor, entity),
                })
            },
        ));
        contacts.entries = entries;
        contacts.floors = floors;
        contacts.origin = origin;
    }
}

#[cfg(test)]
mod tests {
    use bevy::ecs::system::RunSystemOnce;

    use super::*;

    fn settings() -> TargetingSettings {
        TargetingSettings::default()
    }

    #[test]
    fn a_loud_target_is_still_capped_by_the_observers_own_reach() {
        let settings = settings();
        assert_eq!(
            class_range(&settings, 2_000.0, Some(1_000.0)),
            2_000.0,
            "a picket sees a planet as far as its own sensor reaches and no further"
        );
    }

    #[test]
    fn a_body_with_no_return_of_its_own_stays_point_blank() {
        let settings = settings();
        assert_eq!(
            class_range(&settings, 20_000.0, None),
            settings.unsigned_lock_range,
            "unsigned wreckage is lockable point-blank whatever is looking at it"
        );
    }

    #[test]
    fn a_short_sensor_caps_every_return() {
        let settings = settings();
        let cap = 10.0;
        for signature in [None, Some(1.0), Some(1_000.0)] {
            assert!(
                class_range(&settings, cap, signature) <= cap,
                "nothing returns past the observer's own reach"
            );
        }
    }

    /// The whole range model in one line: sensitivity times the target's own
    /// return, under the observer's cap.
    #[test]
    fn a_bigger_signature_is_seen_from_further_away() {
        let settings = settings();
        let near = class_range(&settings, 20_000.0, Some(72.0));
        let far = class_range(&settings, 20_000.0, Some(195.0));
        assert!(
            far > near,
            "a carrier has to be visible from further than a skiff: {far} against {near}"
        );
        assert!(
            (near - settings.signature_range_per_unit * 72.0).abs() < 1e-3,
            "and the range is the sensitivity times the signature, got {near}"
        );
    }

    /// The whole point of the pass: one collection, one set of rules, and a
    /// hostile the observer cannot reach is simply not in it.
    #[test]
    fn the_pass_publishes_what_one_ship_can_reach() {
        let mut world = crate::input::ai::ai_test_world();
        world.insert_resource(settings());
        // Both hostiles are LOUD: a 20u structural arm returns 138u, which
        // gates at 4140u, well past the picket's own 2000u reach. So the only
        // thing that can separate these two is the OBSERVER's cap.
        let loud = HullRadius(20.0);
        let near = world
            .spawn((
                Transform::from_xyz(0.0, 0.0, -100.0),
                RigidBody::Dynamic,
                SpaceshipRootMarker,
                Allegiance::Enemy,
                loud,
            ))
            .id();
        let far = world
            .spawn((
                Transform::from_xyz(0.0, 0.0, -5_000.0),
                RigidBody::Dynamic,
                SpaceshipRootMarker,
                Allegiance::Enemy,
                loud,
            ))
            .id();
        let observer = world
            .spawn((
                Transform::default(),
                RigidBody::Dynamic,
                SpaceshipRootMarker,
                Allegiance::Player,
                SensorRange(AI_SENSOR_RANGE),
                SensorContacts::default(),
            ))
            .id();

        world
            .run_system_once(crate::sections::signature::publish_ship_signatures)
            .expect("the signature pass runs");
        world
            .run_system_once(update_sensor_contacts)
            .expect("the sensing pass runs");

        let contacts = world.get::<SensorContacts>(observer).expect("published");
        assert!(
            contacts.get(near).is_some(),
            "a hostile inside the sensor reach is a contact"
        );
        assert!(
            contacts.get(far).is_none(),
            "one past it is not, and no consumer has to gate it again"
        );
        assert!(contacts.get(observer).is_none(), "a ship never sees itself");
        assert_eq!(
            contacts.get(near).map(|contact| contact.relation),
            Some(Relation::Hostile),
            "and the relation is resolved once, on the contact"
        );
    }
}
