//! The live radar search: while the gesture is open, pick the body under the
//! screen centre and hold it for the dwell that commits it.

use avian3d::prelude::*;
use bevy::prelude::*;
use bevy_enhanced_input::prelude::*;
use nova_gameplay::prelude::*;

use super::gesture::RadarHoldInput;
use crate::prelude::*;

/// How far from the screen centre a body may sit and still be under the
/// crosshair, as a share of the viewport half-height: 24 px on a 1080p
/// screen, about one degree at the default 45 degree field of view.
///
/// A ray alone threads past a skiff a few kilometres out, which is a few
/// pixels across, so a body whose live anchor projects inside this circle is
/// picked even when the ray misses its hull. Screen-relative, so the circle is
/// the same size at every resolution and zoom.
pub const RADAR_PICK_TOLERANCE: f32 = 0.045;

/// The wider circle the current candidate keeps, as a share of the viewport
/// half-height: twice the pick's. A hand holding a small far body near the
/// edge of the sight would otherwise drop it for a frame, and dropping the
/// candidate restarts the dwell.
const RADAR_HOLD_TOLERANCE: f32 = 2.0 * RADAR_PICK_TOLERANCE;

/// The radar search AND the live lock: while a [`RadarState`] exists (the hold
/// gesture is active), live-retarget the candidate to the body under the
/// screen centre ([`radar_pick`]). Once the hold crosses its threshold (the
/// action reports `Fired`), the destination slot is latched from the CURRENT
/// raised stance and written with the candidate every frame it resolves - the
/// lock is LIVE under the sweep; releasing merely stops the retargeting. A
/// `None` candidate never writes (keep-last), so the tap window (pre-Fired)
/// and an empty sweep both leave the slots alone. Runs inside the pause-gated
/// input set, so a pause freezes the writes while the release observers still
/// tear the search down.
#[expect(
    clippy::type_complexity,
    reason = "one query term per radar search input"
)]
pub(super) fn update_radar_search(
    centre: ScreenCentreRay,
    spatial: SpatialQuery,
    q_collider_of: Query<&ColliderOf>,
    time: Res<Time>,
    settings: Res<TargetingSettings>,
    q_hold: Query<&TriggerState, With<Action<RadarHoldInput>>>,
    q_canisters: Query<(), With<CargoCanister>>,
    mut acquired_cue: MessageWriter<RadarLockAcquired>,
    mut retarget_cue: MessageWriter<RadarRetargeted>,
    mut spaceship: Query<
        (
            Option<&WeaponsRaised>,
            &SensorContacts,
            &mut RadarState,
            &mut TravelLock,
            &mut CombatLock,
        ),
        (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>),
    >,
) {
    let hold_fired = q_hold.iter().any(|&state| state == TriggerState::Fired);
    for (raised, contacts, mut radar, mut travel, mut combat) in &mut spaceship {
        let Some(ray) = centre.ray() else {
            continue;
        };
        let origin = contacts.origin;
        let combat_pick = radar.engaged == Some(RadarSlot::Combat)
            || (radar.engaged.is_none() && raised.is_some_and(|raised| raised.0));
        let is_canister = |entity| q_canisters.contains(entity);
        // The first hull the ray meets that belongs to a body the radar may
        // pick. Every other collider - the player's own hull, rounds in
        // flight, a body out of range - is transparent to the pick.
        let body_of = |collider| q_collider_of.get(collider).ok().map(|of| of.body);
        let hull_hit = spatial
            .cast_ray_predicate(
                ray.origin,
                ray.direction,
                f32::MAX,
                true,
                &SpatialQueryFilter::default(),
                &|collider| {
                    body_of(collider)
                        .and_then(|body| contacts.get(body))
                        .is_some_and(|contact| pickable(contact, combat_pick, is_canister))
                },
            )
            .and_then(|hit| body_of(hit.entity).map(|body| (body, hit.distance)));
        let picked = radar_pick(
            radar.candidate,
            &ray,
            hull_hit,
            contacts,
            combat_pick,
            is_canister,
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
        // sensor pass already carries its world anchor).
        let distance = contacts
            .get(candidate)
            .map_or(0.0, |contact| contact.anchor.distance(origin));
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

/// Whether the radar may offer `contact` at all: it needs the line (you
/// cannot designate what the radar cannot see), and a combat sweep never
/// offers a cargo canister.
fn pickable(contact: &SensorContact, combat: bool, is_canister: impl Fn(Entity) -> bool) -> bool {
    contact.in_sight && !(combat && is_canister(contact.entity))
}

/// The radar's pick rule: the NEAREST body under the screen centre.
///
/// A body is under it when the centre ray meets its hull (`hull_hit`, the
/// body and the hit distance, already restricted to [`pickable`] bodies), or
/// when its live anchor projects within [`RADAR_PICK_TOLERANCE`] of the
/// centre - [`RADAR_HOLD_TOLERANCE`] for the `current` candidate. Of those,
/// the one nearest the camera along the ray wins, so a ship crossing in front
/// of another takes the pick, as it takes the pixels. Nothing under the centre
/// drops the candidate (a release then commits nothing - the abort gesture).
///
/// Pure so the rule is unit-testable; the hull ray is the caller's.
fn radar_pick(
    current: Option<Entity>,
    ray: &CentreRay,
    hull_hit: Option<(Entity, f32)>,
    contacts: &SensorContacts,
    combat: bool,
    is_canister: impl Fn(Entity) -> bool,
) -> Option<Entity> {
    let beside = contacts
        .iter()
        .filter(|contact| pickable(contact, combat, &is_canister))
        .filter_map(|contact| {
            let (along, offset) = ray.screen_offset(contact.anchor)?;
            let tolerance = if current == Some(contact.entity) {
                RADAR_HOLD_TOLERANCE
            } else {
                RADAR_PICK_TOLERANCE
            };
            (offset <= tolerance).then_some((contact.entity, along))
        });
    hull_hit
        .into_iter()
        .chain(beside)
        .min_by(|(_, a), (_, b)| a.total_cmp(b))
        .map(|(entity, _)| entity)
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
    use nova_gameplay::test_support::{settle, test_items, unfinished_integrity_physics_app};

    use super::*;

    /// A camera at the origin looking down -Z with the default field of view.
    fn bore() -> CentreRay {
        CentreRay {
            origin: Vec3::ZERO,
            direction: Dir3::NEG_Z,
            tan_half_fov: (PerspectiveProjection::default().fov * 0.5).tan(),
        }
    }

    /// Where a point `distance` down the bore sits when it projects `offset`
    /// (a share of the viewport half-height) to the right of the centre.
    fn off_centre(distance: f32, offset: f32) -> Vec3 {
        Vec3::new(distance * bore().tan_half_fov * offset, 0.0, -distance)
    }

    /// A staged sighting set: what the ship's sensor pass would have
    /// published, without standing up a world to publish it.
    fn seen(entries: impl IntoIterator<Item = (Entity, Vec3, bool)>) -> SensorContacts {
        entries
            .into_iter()
            .map(|(entity, anchor, in_sight)| SensorContact {
                entity,
                anchor,
                relation: Relation::Hostile,
                is_ship: true,
                is_torpedo: false,
                neutralized: false,
                in_sight,
            })
            .collect()
    }

    fn pick(
        current: Option<Entity>,
        hull_hit: Option<(Entity, f32)>,
        contacts: &SensorContacts,
    ) -> Option<Entity> {
        radar_pick(current, &bore(), hull_hit, contacts, false, |_| false)
    }

    #[test]
    fn the_nearest_body_under_the_centre_wins() {
        let near = Entity::from_raw_u32(1).unwrap();
        let far = Entity::from_raw_u32(2).unwrap();
        let hull = Entity::from_raw_u32(3).unwrap();
        let contacts = seen([
            (far, off_centre(300.0, 0.0), true),
            (near, off_centre(100.0, 0.03), true),
            (hull, off_centre(200.0, 0.5), true),
        ]);
        assert_eq!(
            pick(None, None, &contacts),
            Some(near),
            "the nearer of two bodies under the sight wins, even a little off \
             the exact centre"
        );
        assert_eq!(
            pick(None, Some((hull, 60.0)), &contacts),
            Some(hull),
            "a hull the ray meets in front of both takes the pick, though its \
             centre is far off the sight"
        );
        assert_eq!(
            pick(None, Some((hull, 150.0)), &contacts),
            Some(near),
            "and loses it to a body in front of the hit"
        );
    }

    #[test]
    fn combat_skips_canisters_but_travel_can_designate_them() {
        let canister = Entity::from_raw_u32(1).unwrap();
        let neutral = Entity::from_raw_u32(2).unwrap();
        let contacts = seen([
            (canister, off_centre(10.0, 0.0), true),
            (neutral, off_centre(12.0, 0.0), true),
        ]);
        let pick = |combat| {
            radar_pick(None, &bore(), None, &contacts, combat, |entity| {
                entity == canister
            })
        };
        assert_eq!(pick(false), Some(canister));
        assert_eq!(pick(true), Some(neutral));
    }

    #[test]
    fn a_body_outside_the_sight_or_behind_the_camera_is_not_picked() {
        let body = Entity::from_raw_u32(1).unwrap();
        let at = |anchor| seen([(body, anchor, true)]);
        assert_eq!(
            pick(
                None,
                None,
                &at(off_centre(100.0, 0.9 * RADAR_PICK_TOLERANCE))
            ),
            Some(body)
        );
        assert_eq!(
            pick(
                None,
                None,
                &at(off_centre(100.0, 1.1 * RADAR_PICK_TOLERANCE))
            ),
            None,
            "a body just outside the sight is not under the crosshair"
        );
        assert_eq!(
            pick(None, None, &at(Vec3::new(0.0, 0.0, 100.0))),
            None,
            "a body behind the camera must not be picked"
        );
        assert_eq!(pick(None, None, &seen([])), None);
    }

    /// Acquisition needs the line. A body dead under the centre is still not
    /// offered while a rock stands in front of it - which is the half of the
    /// occlusion rule a HELD travel designation is exempt from.
    #[test]
    fn radar_pick_never_offers_a_body_it_cannot_see() {
        let hidden = Entity::from_raw_u32(1).unwrap();
        let clear = Entity::from_raw_u32(2).unwrap();
        let behind_cover = seen([(hidden, off_centre(100.0, 0.0), false)]);
        assert_eq!(
            pick(None, None, &behind_cover),
            None,
            "a body under the centre but behind cover must not be offered"
        );
        // And it does not shadow the body that IS visible behind it.
        let mixed = seen([
            (hidden, off_centre(100.0, 0.0), false),
            (clear, off_centre(200.0, 0.0), true),
        ]);
        assert_eq!(
            pick(None, None, &mixed),
            Some(clear),
            "the nearest VISIBLE body wins, not the nearest body"
        );
        // An incumbent that goes behind cover loses the pick outright.
        assert_eq!(
            pick(Some(hidden), None, &behind_cover),
            None,
            "the hold circle must not keep a candidate the radar can no longer see"
        );
    }

    /// The candidate keeps a wider circle than a fresh pick, so a small far
    /// body held near the edge of the sight does not strobe off and restart
    /// the dwell.
    #[test]
    fn the_candidate_keeps_a_wider_circle_than_a_fresh_pick() {
        let body = Entity::from_raw_u32(1).unwrap();
        let between = seen([(body, off_centre(500.0, 1.5 * RADAR_PICK_TOLERANCE), true)]);
        assert_eq!(pick(None, None, &between), None, "too far out to pick");
        assert_eq!(pick(Some(body), None, &between), Some(body), "but held");
        let outside = seen([(body, off_centre(500.0, 1.1 * RADAR_HOLD_TOLERANCE), true)]);
        assert_eq!(pick(Some(body), None, &outside), None, "until it leaves");
    }

    /// Player with an OPEN radar search, and the player camera at the origin
    /// looking down -Z, so a body placed straight down -Z is under the screen
    /// centre. Returns (world, player).
    fn radar_world() -> (World, Entity) {
        let mut world = World::new();
        world.insert_resource(Time::<()>::default());
        world.init_resource::<TargetingSettings>();
        // The lock scanner's line-of-sight ray and the pick's hull ray read
        // avian's collider trees. Empty here, which is the point: these rigs
        // are about the RANGE model and the sight circle, and nothing in them
        // stands between the ship and a candidate.
        world.init_resource::<avian3d::collider_tree::ColliderTrees>();
        world.init_resource::<Messages<RadarLockAcquired>>();
        world.init_resource::<Messages<RadarRetargeted>>();
        world.spawn((
            SpaceshipCameraController,
            Projection::Perspective(PerspectiveProjection::default()),
            Transform::IDENTITY,
            GlobalTransform::IDENTITY,
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

    /// The signature and sensing passes FIRST: a hull publishes what it looks
    /// like, the sensor pass turns that into what this ship can see, and
    /// nothing else in the frame decides either any more.
    fn search(world: &mut World) {
        world
            .run_system_once(crate::sections::signature::publish_ship_signatures)
            .unwrap();
        world
            .run_system_once(super::super::sensing::update_sensor_contacts)
            .unwrap();
        world.run_system_once(update_radar_search).unwrap();
    }

    /// Stand the player camera where the Normal chase rig settles it behind a
    /// small craft at the origin facing -Z: lifted and set back, pitched down
    /// onto a focus point ahead of the hull. Returns its pose.
    fn chase_camera(world: &mut World) -> Transform {
        let pose = chase_camera_opening_pose(Vec3::ZERO, Quat::IDENTITY, 0.0, 1.0);
        let mut q_camera =
            world.query_filtered::<(&mut Transform, &mut GlobalTransform), With<SpaceshipCameraController>>();
        let (mut transform, mut global) = q_camera.single_mut(world).unwrap();
        *transform = pose;
        *global = GlobalTransform::from(pose);
        pose
    }

    /// A signed body the radar sees from across a combat volume.
    fn spawn_mark(world: &mut World, at: Vec3) -> Entity {
        world
            .spawn((
                RigidBody::Dynamic,
                LockSignature(20.0),
                Transform::from_translation(at),
            ))
            .id()
    }

    /// The owner's report: a target held in the middle of the screen does not
    /// lock. The chase camera sits above and behind the hull and looks down
    /// onto a point ahead of it, so the screen centre and the ship's nose are
    /// two different rays; they cross only at that focus point. 300 u out, the
    /// body on the nose shows about 140 px above the centre of a 1080p screen.
    #[test]
    fn the_body_under_the_screen_centre_wins_over_the_body_on_the_nose() {
        let (mut world, player) = radar_world();
        let camera = chase_camera(&mut world);
        let centred = spawn_mark(&mut world, camera.translation + camera.forward() * 300.0);
        spawn_mark(&mut world, Vec3::new(0.0, 0.0, -300.0));
        search(&mut world);
        assert_eq!(
            candidate(&mut world, player),
            Some(centred),
            "the body under the screen centre is the one the player aims at"
        );
    }

    #[test]
    fn a_body_on_the_nose_but_off_the_screen_centre_is_not_picked() {
        let (mut world, player) = radar_world();
        chase_camera(&mut world);
        spawn_mark(&mut world, Vec3::new(0.0, 0.0, -300.0));
        search(&mut world);
        assert_eq!(candidate(&mut world, player), None);
    }

    /// Close in, the screen centre and the look ray part by tens of degrees:
    /// a body 50 m ahead of the hull, held in the middle of the screen, sits
    /// far off the look ray and must still be offered.
    #[test]
    fn a_close_body_under_the_screen_centre_is_picked() {
        let (mut world, player) = radar_world();
        let camera = chase_camera(&mut world);
        // 25 u down the centre ray is 5 u ahead of the hull and 2 u above it.
        let close = spawn_mark(&mut world, camera.translation + camera.forward() * 25.0);
        search(&mut world);
        assert_eq!(candidate(&mut world, player), Some(close));
    }

    /// The pick against a real collider tree: the ray meets a rock's flank
    /// while the rock's centre projects far outside the sight, and a body
    /// dead centre behind the rock loses to it - the nearest hull along the
    /// ray is the one under the crosshair.
    #[test]
    fn the_ray_picks_the_nearest_hull_it_meets() {
        let mut app = unfinished_integrity_physics_app();
        app.init_resource::<TargetingSettings>();
        app.init_resource::<Messages<RadarLockAcquired>>();
        app.init_resource::<Messages<RadarRetargeted>>();
        app.world_mut().spawn((
            SpaceshipCameraController,
            Projection::Perspective(PerspectiveProjection::default()),
            Transform::IDENTITY,
        ));
        let player = app
            .world_mut()
            .spawn((
                SpaceshipRootMarker,
                PlayerSpaceshipMarker,
                Transform::IDENTITY,
                targeting_state(),
                RadarState::default(),
            ))
            .id();
        // 40 u off the ray at 300 u is about 0.32 of the half-height: the
        // centre is nowhere near the sight, the 50 u hull is right across it.
        let rock = app
            .world_mut()
            .spawn((
                RigidBody::Dynamic,
                LockSignature(50.0),
                Transform::from_xyz(40.0, 0.0, -300.0),
            ))
            .id();
        app.world_mut().spawn((
            ChildOf(rock),
            Transform::default(),
            Collider::sphere(50.0),
            ColliderDensity(1.0),
        ));
        let behind = spawn_mark(app.world_mut(), Vec3::new(0.0, 0.0, -500.0));
        app.finish();
        settle(&mut app);

        search(app.world_mut());
        assert_eq!(candidate(app.world_mut(), player), Some(rock));

        // The rock moves off the line: the body behind it is under the sight.
        app.world_mut()
            .get_mut::<Transform>(rock)
            .unwrap()
            .translation = Vec3::new(400.0, 0.0, -300.0);
        settle(&mut app);
        search(app.world_mut());
        assert_eq!(candidate(app.world_mut(), player), Some(behind));
    }

    #[test]
    fn canister_is_travel_lockable_past_unsigned_range_but_excluded_from_combat() {
        let (mut world, player) = radar_world();
        // 90 m dead ahead: past the 50 m unsigned range, inside the canister's
        // own return.
        let at = Transform::from_xyz(0.0, 0.0, -9.0);
        assert!(at.translation.length() > TargetingSettings::default().unsigned_lock_range);
        let canister = world
            .spawn((
                cargo_canister(
                    CargoCanister::new(&test_items(), &ItemRoleType::HullPlate.into(), 1),
                    at,
                    Vec3::ZERO,
                    AssetRef::from("canister.glb#Scene0"),
                ),
                CargoCanisterRuntimeId(0),
            ))
            .id();
        // Behind the canister on the same ray: the travel sweep meets the
        // canister first, the combat sweep looks through it.
        let neutral = world
            .spawn((
                RigidBody::Dynamic,
                LockSignature(1.0),
                Transform::from_xyz(0.0, 0.0, -12.0),
            ))
            .id();
        search(&mut world);
        assert!(world
            .get::<SensorContacts>(player)
            .unwrap()
            .get(canister)
            .is_some());
        assert_eq!(candidate(&mut world, player), Some(canister));
        world.entity_mut(player).insert(WeaponsRaised(true));
        search(&mut world);
        assert_eq!(candidate(&mut world, player), Some(neutral));
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
                Transform::from_translation(Vec3::new(0.0, 0.0, -200.0)),
            ))
            .id();

        search(&mut world);
        assert_eq!(candidate(&mut world, player), None);

        world
            .entity_mut(rock)
            .insert(Transform::from_translation(Vec3::new(0.0, 0.0, -40.0)));
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
                Transform::from_translation(Vec3::new(0.0, 0.0, -8.0)),
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
            .insert(Transform::from_translation(Vec3::new(0.0, 0.0, -4.0)));
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
                Transform::from_translation(Vec3::new(0.0, 0.0, -300.0)),
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
            Transform::from_translation(Vec3::new(0.0, 0.0, -4.0)),
        ));
        search(&mut world);
        assert_eq!(
            candidate(&mut world, player),
            None,
            "an unsigned static body (trigger area) is never pickable"
        );
    }

    /// There is no "full-range class" left: how far a hull is pickable from
    /// comes from the hull. A big ship returns more of the scanner wave than
    /// a small one and is picked from further out.
    #[test]
    fn a_bigger_hull_is_picked_from_further_away() {
        let far = Transform::from_translation(Vec3::new(0.0, 0.0, -5000.0));

        // A 40u structural arm returns 248u, which gates at 7440u. (The arm
        // is staged directly: `publish_hull_radii` derives it from live
        // sections, and this rig is about the picker.)
        let (mut world, player) = radar_world();
        let carrier = world
            .spawn((
                SpaceshipRootMarker,
                RigidBody::Dynamic,
                HullRadius(40.0),
                far,
            ))
            .id();
        search(&mut world);
        assert_eq!(
            candidate(&mut world, player),
            Some(carrier),
            "a hull that size is pickable right across a combat volume"
        );

        // A bare root returns the base signature alone, 28u, which gates at
        // 840u: the same gap is far too wide.
        let (mut world, player) = radar_world();
        world.spawn((SpaceshipRootMarker, RigidBody::Dynamic, far));
        search(&mut world);
        assert_eq!(
            candidate(&mut world, player),
            None,
            "and a skiff-sized return does not cross it"
        );
    }

    /// An invisible gravity anchor - a well with no collider, no mesh and no
    /// authored signature - is collected, because a well body is a thing a
    /// pilot navigates by, but it returns nothing of its own and so is
    /// point-blank. A planet or a rock publishes a signature from its radius
    /// and is not this case.
    #[test]
    fn an_unsigned_well_body_is_point_blank() {
        let (mut world, player) = radar_world();
        let anchor = world
            .spawn((
                RigidBody::Static,
                GravityWell::from_mass(1200.0, 20.0, &GravitySettings::default()),
                Transform::from_translation(Vec3::new(0.0, 0.0, -4.0)),
            ))
            .id();
        search(&mut world);
        assert_eq!(candidate(&mut world, player), Some(anchor));

        world
            .entity_mut(anchor)
            .insert(Transform::from_translation(Vec3::new(0.0, 0.0, -5000.0)));
        world.get_mut::<RadarState>(player).unwrap().candidate = None;
        search(&mut world);
        assert_eq!(
            candidate(&mut world, player),
            None,
            "nothing about an anchor comes back from across the map"
        );
    }

    #[test]
    fn committed_torpedoes_lock_at_combat_range_not_across_the_map() {
        let (mut world, player) = radar_world();
        // The shipped 320 m/s type: 820 m of return, which gates at 2460u.
        let torpedo = world
            .spawn((
                TorpedoProjectileMarker,
                TorpedoTargetChosen,
                LockSignature(82.0),
                RigidBody::Dynamic,
                Transform::from_translation(Vec3::new(0.0, 0.0, -2000.0)),
            ))
            .id();
        search(&mut world);
        assert_eq!(candidate(&mut world, player), Some(torpedo));

        world
            .entity_mut(torpedo)
            .insert(Transform::from_translation(Vec3::new(0.0, 0.0, -5000.0)));
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
                Transform::from_translation(Vec3::new(0.0, 0.0, -4.0)),
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
                Transform::from_translation(Vec3::new(0.0, 0.0, -65.0)),
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
            .insert(Transform::from_translation(Vec3::new(0.0, 0.0, -80.0)));
        search(&mut world);
        assert_eq!(candidate(&mut world, player), None);
    }
}
