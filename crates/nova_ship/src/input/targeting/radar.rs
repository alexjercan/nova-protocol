//! The live radar search: while the gesture is open, pick the best body on
//! the look ray inside the cone and hold it for the dwell that commits it.

use avian3d::prelude::*;
use bevy::prelude::*;
use bevy_enhanced_input::prelude::*;
use nova_gameplay::prelude::*;

use super::{
    contacts::{collect_lockable, Lockable, LockableQuery},
    gesture::RadarHoldInput,
};
use crate::prelude::*;

/// Half-angle (degrees) of the RADAR cone around the look ray. While the
/// radar is held, any lockable body within this angle of where the player is
/// looking is a candidate and the one closest to the ray wins (with
/// [`RADAR_PICK_HYSTERESIS`]); the wide cone means rough pointing suffices,
/// no pixel-perfect ray needed.
const TARGETING_CONE_HALF_ANGLE_DEG: f32 = 18.0;

/// Provisional-candidate hysteresis: while the radar is held,
/// a challenger only steals the candidate when its angular distance to the
/// ray (measured as `1 - cos`) is below this fraction of the incumbent's -
/// otherwise two near-collinear bodies (a torpedo and its launcher) strobe
/// the candidate and the release commits a coin flip.
const RADAR_PICK_HYSTERESIS: f32 = 0.75;

/// The radar search AND the live lock (strand A1): while a [`RadarState`]
/// exists (the hold gesture is active), live-retarget the candidate to the
/// best body on the look ray, with incumbent hysteresis so two
/// near-collinear bodies do not strobe the pick. Once the hold crosses its
/// threshold (the action reports `Fired`), the destination slot is latched
/// from the CURRENT raised stance (Q1a) and written with the candidate every
/// frame it resolves - the lock is LIVE under the sweep; releasing merely
/// stops the retargeting. A `None` candidate never writes (keep-last, Q2a),
/// so the tap window (pre-Fired) and an empty sweep both leave the slots
/// alone. Runs inside the pause-gated input set, so a pause freezes the
/// writes while the release observers still tear the search down.
#[expect(
    clippy::type_complexity,
    reason = "one query term per radar search input"
)]
pub(super) fn update_radar_search(
    look_ray: ActiveLookRay,
    time: Res<Time>,
    settings: Res<TargetingSettings>,
    q_candidates: LockableQuery,
    q_hold: Query<&TriggerState, With<Action<RadarHoldInput>>>,
    mut acquired_cue: MessageWriter<RadarLockAcquired>,
    mut retarget_cue: MessageWriter<RadarRetargeted>,
    mut spaceship: Query<
        (
            &Transform,
            Option<&ComputedCenterOfMass>,
            Entity,
            Option<&Allegiance>,
            Option<&WeaponsRaised>,
            &mut RadarState,
            &mut TravelLock,
            &mut CombatLock,
            &mut CombatDecay,
        ),
        (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>),
    >,
) {
    let hold_fired = q_hold.iter().any(|&state| state == TriggerState::Fired);
    for (
        transform,
        com,
        ship,
        ship_allegiance,
        raised,
        mut radar,
        mut travel,
        mut combat,
        mut decay,
    ) in &mut spaceship
    {
        let Some(aim_rotation) = look_ray.rotation() else {
            continue;
        };
        let origin = live_structure_anchor(transform, com);
        let aim = (aim_rotation * Vec3::NEG_Z).normalize();
        let candidates = collect_lockable(
            &q_candidates,
            &settings,
            origin,
            ship,
            ship_allegiance,
            &[radar.candidate],
        );
        let picked = radar_pick(
            radar.candidate,
            origin,
            aim,
            TARGETING_CONE_HALF_ANGLE_DEG.to_radians().cos(),
            &candidates,
        );
        if radar.candidate != picked {
            radar.candidate = picked;
        }

        // Tap window: nothing latches, nothing writes.
        if !hold_fired {
            continue;
        }
        let slot = *radar.engaged.get_or_insert_with(|| {
            if raised.is_some_and(|raised| raised.0) {
                RadarSlot::Combat
            } else {
                RadarSlot::Travel
            }
        });
        // An engaged combat sweep IS combat activity: hold the decay at zero
        // even across equality-skip frames (F12 - a long sweep must not
        // cross the decay boundary mid-gesture).
        if slot == RadarSlot::Combat && decay.0 != 0.0 {
            decay.0 = 0.0;
        }
        let Some(candidate) = radar.candidate else {
            // Sweeping onto empty space cancels the in-progress acquisition
            // dwell; the committed lock keeps-last (nothing new commits).
            radar.dwell_target = None;
            radar.dwell_secs = 0.0;
            radar.dwell_needed = 0.0;
            continue;
        };

        // Acquisition dwell: charge a timer on the settled candidate; the
        // slot is written only once it completes. Moving to a DIFFERENT
        // candidate (or off, above) restarts the dwell - that is the cancel-
        // by-sweeping-off beat, and it means a re-designation must earn a
        // fresh dwell too, not just the first acquire.
        if radar.dwell_target != Some(candidate) {
            radar.dwell_target = Some(candidate);
            radar.dwell_secs = 0.0;
        }
        radar.dwell_secs += time.delta_secs();

        // Distance to the pending candidate drives the dwell curve (the
        // collected list already carries its world position).
        let distance = candidates
            .iter()
            .find(|(entity, ..)| *entity == candidate)
            .map_or(0.0, |(_, position, ..)| position.distance(origin));
        // 1.0 = the stealth/aspect extension seam (no such mechanic yet).
        let needed = lock_dwell_secs(distance, 1.0, &settings);
        // Cache it for the ring HUD's fill.
        radar.dwell_needed = needed;
        if radar.dwell_secs < needed {
            // Still charging: the ring fills, but nothing commits and the
            // previous lock (if any) holds under the sweep.
            continue;
        }

        // Dwell complete: hard-commit the slot (the write the instant path
        // used to do the moment a candidate settled).
        let changed = match slot {
            RadarSlot::Combat => {
                let changed = combat.0 != Some(candidate);
                if changed {
                    combat.0 = Some(candidate);
                }
                changed
            }
            RadarSlot::Travel => {
                let changed = travel.0 != Some(candidate);
                if changed {
                    travel.0 = Some(candidate);
                }
                changed
            }
        };
        if !radar.acquired {
            radar.acquired = true;
            acquired_cue.write(RadarLockAcquired {
                combat: slot == RadarSlot::Combat,
            });
        } else if changed {
            // A re-designation within the already-acquired gesture (the slot
            // moved to a NEW candidate, after its own dwell): the subtle
            // retarget tick, distinct from the once-per-gesture acquire above.
            retarget_cue.write(RadarRetargeted {
                combat: slot == RadarSlot::Combat,
            });
        }
        // dwell_secs stays >= needed: holding the same candidate re-runs this
        // block harmlessly (equality-skip, no cue); a new candidate resets it
        // above. This keeps the committed lock stable without re-firing.
    }
}

/// The radar's pick rule: the body nearest the look ray inside the cone,
/// except that an incumbent candidate
/// holds unless the challenger is DECISIVELY nearer the ray - its angular
/// distance (as `1 - cos`) under [`RADAR_PICK_HYSTERESIS`] of the
/// incumbent's. Leaving the cone entirely drops the candidate (a release
/// then commits nothing - the abort gesture, decision D1).
///
/// Pure so the hysteresis rule is unit-testable.
fn radar_pick(
    current: Option<Entity>,
    origin: Vec3,
    aim: Vec3,
    min_cos: f32,
    candidates: &[Lockable],
) -> Option<Entity> {
    let scored: Vec<(Entity, f32)> = candidates
        .iter()
        .filter_map(|&(entity, position, ..)| {
            let to_target = position - origin;
            let distance = to_target.length();
            if distance < f32::EPSILON {
                return None;
            }
            let cos_angle = to_target.normalize().dot(aim);
            (cos_angle >= min_cos).then_some((entity, cos_angle))
        })
        .collect();
    let (best, best_cos) = scored
        .iter()
        .copied()
        .max_by(|(_, a), (_, b)| a.total_cmp(b))?;
    if let Some(current) = current {
        if let Some(&(_, current_cos)) = scored.iter().find(|(entity, _)| *entity == current) {
            if best != current && (1.0 - best_cos) >= RADAR_PICK_HYSTERESIS * (1.0 - current_cos) {
                return Some(current);
            }
        }
    }
    Some(best)
}

/// The acquisition dwell (seconds) a candidate at `distance` from the ship
/// needs before it hard-commits to its lock slot: the point-blank
/// [`TargetingSettings::lock_dwell_base`] plus a distance term that scales
/// linearly to [`TargetingSettings::lock_dwell_range_factor`] at
/// [`TargetingSettings::lock_dwell_reference_range`] and saturates beyond it,
/// all times `modifier` and clamped to `[min, max]`.
///
/// `modifier` is the stealth / aspect EXTENSION SEAM (a future "harder to lock
/// at a bad aspect" mechanic multiplies here); today the caller passes 1.0.
/// Pure so the curve is unit-testable.
pub(super) fn lock_dwell_secs(distance: f32, modifier: f32, settings: &TargetingSettings) -> f32 {
    let reach = if settings.lock_dwell_reference_range > 0.0 {
        (distance / settings.lock_dwell_reference_range).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let raw = settings.lock_dwell_base
        * (1.0 + settings.lock_dwell_range_factor * reach)
        * modifier.max(0.0);
    // `min.max(max)` guards the inspector case where the two knobs are set
    // out of order - `f32::clamp` panics if min > max.
    raw.clamp(
        settings.lock_dwell_min,
        settings.lock_dwell_min.max(settings.lock_dwell_max),
    )
}

#[cfg(test)]
mod tests {
    use bevy::ecs::system::RunSystemOnce;

    use super::*;

    fn cone_cos(half_angle_deg: f32) -> f32 {
        half_angle_deg.to_radians().cos()
    }

    #[test]
    fn radar_picks_the_body_nearest_the_aim_ray() {
        let origin = Vec3::ZERO;
        let aim = Vec3::NEG_Z;
        let near_center = Entity::from_raw_u32(1).unwrap();
        let off_center = Entity::from_raw_u32(2).unwrap();
        let candidates = [
            // ~1.1 deg off axis, far vs ~8.5 deg off axis, near: the
            // nearer-to-center one wins even though it is further away.
            (near_center, Vec3::new(2.0, 0.0, -100.0), false, true),
            (off_center, Vec3::new(3.0, 0.0, -20.0), false, true),
        ];
        let picked = radar_pick(None, origin, aim, cone_cos(18.0), &candidates);
        assert_eq!(picked, Some(near_center));
    }

    #[test]
    fn radar_ignores_bodies_outside_the_cone_or_behind() {
        let side = [(
            Entity::from_raw_u32(1).unwrap(),
            Vec3::new(50.0, 0.0, 0.0),
            false,
            true,
        )];
        assert_eq!(
            radar_pick(None, Vec3::ZERO, Vec3::NEG_Z, cone_cos(18.0), &side),
            None,
            "a body outside the cone must not be picked"
        );
        let behind = [(
            Entity::from_raw_u32(1).unwrap(),
            Vec3::new(0.0, 0.0, 100.0),
            false,
            true,
        )];
        assert_eq!(
            radar_pick(None, Vec3::ZERO, Vec3::NEG_Z, cone_cos(18.0), &behind),
            None,
            "a body behind the ship must not be picked"
        );
        assert_eq!(
            radar_pick(None, Vec3::ZERO, Vec3::NEG_Z, cone_cos(18.0), &[]),
            None
        );
    }

    /// D7: the provisional candidate holds against a marginally-nearer
    /// challenger and yields to a decisively-nearer one; leaving the cone
    /// drops it entirely (the release-abort).
    #[test]
    fn radar_pick_applies_angular_hysteresis() {
        let a = Entity::from_raw_u32(1).unwrap();
        let b = Entity::from_raw_u32(2).unwrap();
        let origin = Vec3::ZERO;
        let aim = Vec3::NEG_Z;
        let min_cos = cone_cos(18.0);
        // a at ~8 deg off-ray, b at ~7.4 deg: nearer, but NOT decisively
        // ((1-cos7.4) ~ 0.0084 vs 0.75 * (1-cos8) ~ 0.0073).
        let marginal = [
            (a, Vec3::new(14.0, 0.0, -100.0), false, true),
            (b, Vec3::new(13.0, 0.0, -100.0), false, true),
        ];
        assert_eq!(
            radar_pick(Some(a), origin, aim, min_cos, &marginal),
            Some(a),
            "a marginally-nearer challenger must not steal the candidate"
        );
        // b dead on the ray: decisive.
        let decisive = [
            (a, Vec3::new(14.0, 0.0, -100.0), false, true),
            (b, Vec3::new(0.1, 0.0, -100.0), false, true),
        ];
        assert_eq!(
            radar_pick(Some(a), origin, aim, min_cos, &decisive),
            Some(b),
            "a decisively-nearer challenger takes the candidate"
        );
        // No incumbent: plain nearest wins.
        assert_eq!(radar_pick(None, origin, aim, min_cos, &marginal), Some(b));
        // Cone empty: candidate drops (the abort).
        let outside = [(a, Vec3::new(100.0, 0.0, 0.0), false, true)];
        assert_eq!(radar_pick(Some(a), origin, aim, min_cos, &outside), None);
    }

    /// Player + faithful split camera rigs (ACTIVE normal rig on -Z, dormant
    /// turret decoy 90 degrees off - reading the wrong rig fails loudly) with
    /// an OPEN radar search. Returns (world, player).
    fn radar_world() -> (World, Entity) {
        let mut world = World::new();
        world.insert_resource(Time::<()>::default());
        world.init_resource::<TargetingSettings>();
        world.init_resource::<Messages<RadarLockAcquired>>();
        world.init_resource::<Messages<RadarRetargeted>>();
        world.spawn((
            SpaceshipCameraInputMarker,
            SpaceshipCameraNormalInputMarker,
            SpaceshipRotationInputActiveMarker,
            PointRotationOutput(Quat::IDENTITY),
        ));
        world.spawn((
            SpaceshipCameraInputMarker,
            SpaceshipCameraTurretInputMarker,
            PointRotationOutput(Quat::from_rotation_y(std::f32::consts::FRAC_PI_2)),
        ));
        let player = world
            .spawn((
                SpaceshipRootMarker,
                PlayerSpaceshipMarker,
                Transform::IDENTITY,
                targeting_state(),
                // An open search still inside the tap window (nothing
                // engaged): these tests exercise the PICKER; the live-write
                // paths have their own rig below.
                RadarState::default(),
            ))
            .id();
        (world, player)
    }

    fn candidate(world: &mut World, player: Entity) -> Option<Entity> {
        world.get::<RadarState>(player).unwrap().candidate
    }

    fn search(world: &mut World) {
        world.run_system_once(update_radar_search).unwrap();
    }

    #[test]
    fn radar_cone_originates_at_the_live_structure_anchor() {
        // A candidate dead ahead of the ANCHOR but 33 degrees off the ROOT
        // ORIGIN bearing: it is picked only if the cone originates at the
        // anchor (18 degree half-angle).
        let (mut world, player) = radar_world();
        world.entity_mut(player).insert((
            Transform::from_translation(Vec3::new(10.0, 0.0, 0.0)),
            ComputedCenterOfMass(Vec3::new(2.0, 0.0, 0.0)),
        ));
        let body = world
            .spawn((
                RigidBody::Dynamic,
                LockSignature(20.0),
                GlobalTransform::from_translation(Vec3::new(12.0, 0.0, -3.0)),
            ))
            .id();

        search(&mut world);
        assert_eq!(
            candidate(&mut world, player),
            Some(body),
            "the cone must originate at the anchor, not the root origin"
        );
    }

    /// Ray-liveness: the radar follows the ACTIVE rig's live output; the
    /// decoy turret rig points at the side body from the start, so reading it
    /// would fail the delivery guard.
    #[test]
    fn radar_follows_the_live_active_ray() {
        let (mut world, player) = radar_world();
        let side = world
            .spawn((
                SpaceshipRootMarker,
                Allegiance::Neutral,
                RigidBody::Dynamic,
                GlobalTransform::from_translation(Vec3::new(-100.0, 0.0, 0.0)),
            ))
            .id();

        search(&mut world);
        assert_eq!(
            candidate(&mut world, player),
            None,
            "delivery guard: the side body is outside the cone while looking -Z"
        );

        let active = world
            .query_filtered::<Entity, With<SpaceshipRotationInputActiveMarker>>()
            .iter(&world)
            .next()
            .expect("an active rig exists");
        world
            .entity_mut(active)
            .insert(PointRotationOutput(Quat::from_rotation_y(
                std::f32::consts::FRAC_PI_2,
            )));
        search(&mut world);
        assert_eq!(
            candidate(&mut world, player),
            Some(side),
            "the radar must follow the live active ray"
        );
    }

    #[test]
    fn small_signatures_only_lock_up_close() {
        // A signed 2u rock (range 60u at the default 30/unit) dead ahead:
        // invisible to the radar at 200u, pickable at 40u.
        let (mut world, player) = radar_world();
        let rock = world
            .spawn((
                RigidBody::Dynamic,
                LockSignature(2.0),
                GlobalTransform::from_translation(Vec3::new(0.0, 0.0, -200.0)),
            ))
            .id();

        search(&mut world);
        assert_eq!(candidate(&mut world, player), None);

        world
            .entity_mut(rock)
            .insert(GlobalTransform::from_translation(Vec3::new(
                0.0, 0.0, -40.0,
            )));
        search(&mut world);
        assert_eq!(candidate(&mut world, player), Some(rock));
    }

    #[test]
    fn unsigned_debris_is_point_blank_only() {
        // A bare dynamic body (battle debris): never pickable at ~8u, only
        // inside the (retuned, ~5u) unsigned point-blank range.
        let (mut world, player) = radar_world();
        let debris = world
            .spawn((
                RigidBody::Dynamic,
                GlobalTransform::from_translation(Vec3::new(0.0, 0.0, -8.0)),
            ))
            .id();

        search(&mut world);
        assert_eq!(
            candidate(&mut world, player),
            None,
            "debris at 8u must be invisible to the radar (range ~5u)"
        );

        world
            .entity_mut(debris)
            .insert(GlobalTransform::from_translation(Vec3::new(0.0, 0.0, -4.0)));
        search(&mut world);
        assert_eq!(candidate(&mut world, player), Some(debris));
    }

    #[test]
    fn static_beacons_lock_but_static_areas_never_do() {
        let (mut world, player) = radar_world();
        let beacon = world
            .spawn((
                RigidBody::Static,
                LockSignature(20.0),
                GlobalTransform::from_translation(Vec3::new(0.0, 0.0, -300.0)),
            ))
            .id();
        search(&mut world);
        assert_eq!(
            candidate(&mut world, player),
            Some(beacon),
            "a signed static body (nav beacon) is radar-pickable"
        );

        let (mut world, player) = radar_world();
        world.spawn((
            RigidBody::Static,
            GlobalTransform::from_translation(Vec3::new(0.0, 0.0, -4.0)),
        ));
        search(&mut world);
        assert_eq!(
            candidate(&mut world, player),
            None,
            "an unsigned static body (trigger area) is never pickable"
        );
    }

    #[test]
    fn ships_and_well_bodies_keep_their_long_range_lock() {
        for components in 0..2 {
            let (mut world, player) = radar_world();
            let far = GlobalTransform::from_translation(Vec3::new(0.0, 0.0, -5000.0));
            let target = match components {
                0 => world
                    .spawn((
                        RigidBody::Static,
                        GravityWell::from_mass(1200.0, 20.0, &GravitySettings::default()),
                        far,
                    ))
                    .id(),
                _ => world
                    .spawn((SpaceshipRootMarker, RigidBody::Dynamic, far))
                    .id(),
            };

            search(&mut world);
            assert_eq!(
                candidate(&mut world, player),
                Some(target),
                "full-range class {components} must be pickable at range"
            );
        }
    }

    #[test]
    fn committed_torpedoes_lock_at_combat_range_not_across_the_map() {
        let (mut world, player) = radar_world();
        let torpedo = world
            .spawn((
                TorpedoProjectileMarker,
                TorpedoTargetChosen,
                RigidBody::Dynamic,
                GlobalTransform::from_translation(Vec3::new(0.0, 0.0, -2000.0)),
            ))
            .id();
        search(&mut world);
        assert_eq!(candidate(&mut world, player), Some(torpedo));

        world
            .entity_mut(torpedo)
            .insert(GlobalTransform::from_translation(Vec3::new(
                0.0, 0.0, -5000.0,
            )));
        world.get_mut::<RadarState>(player).unwrap().candidate = None;
        search(&mut world);
        assert_eq!(
            candidate(&mut world, player),
            None,
            "a torpedo is not visible across the map"
        );
    }

    #[test]
    fn an_authored_signature_never_gates_below_the_debris_floor() {
        let (mut world, player) = radar_world();
        let speck = world
            .spawn((
                RigidBody::Dynamic,
                LockSignature(0.0),
                GlobalTransform::from_translation(Vec3::new(0.0, 0.0, -4.0)),
            ))
            .id();
        search(&mut world);
        assert_eq!(candidate(&mut world, player), Some(speck));
    }

    #[test]
    fn the_candidate_holds_a_little_past_its_gate_but_fresh_picks_do_not() {
        // A signed 2u rock gates at 60u. Fresh pick at 65u: refused.
        let (mut world, player) = radar_world();
        let rock = world
            .spawn((
                RigidBody::Dynamic,
                LockSignature(2.0),
                GlobalTransform::from_translation(Vec3::new(0.0, 0.0, -65.0)),
            ))
            .id();
        search(&mut world);
        assert_eq!(candidate(&mut world, player), None);

        // Held inside the gate, then drifting to 65u (inside 1.15x): holds.
        world.get_mut::<RadarState>(player).unwrap().candidate = Some(rock);
        search(&mut world);
        assert_eq!(candidate(&mut world, player), Some(rock));

        // Truly out (past 1.15x): dropped.
        world
            .entity_mut(rock)
            .insert(GlobalTransform::from_translation(Vec3::new(
                0.0, 0.0, -80.0,
            )));
        search(&mut world);
        assert_eq!(candidate(&mut world, player), None);
    }
}
