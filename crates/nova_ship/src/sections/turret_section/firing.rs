//! Turret firing: the fixed-clock muzzle loop that spawns bullets and the
//! bullet's own contact rule - deal the round's bite, then let its type decide
//! whether it travels on.

use std::time::Duration;

use avian3d::prelude::*;
use bevy::prelude::*;
use bevy_transform_interpolation::{RotationEasingState, TranslationEasingState};
use nova_gameplay::lifetime::TempEntity;

use super::*;
use crate::{physics::prelude::rigid_body_point_velocity, sections::local_pose_in_root};

/// A bullet deals its bite to the first TANGIBLE thing it meets, and its type
/// decides whether it travels on.
///
/// Nova OWNS the damage here: the bullet is a near-zero-mass Sensor (see the
/// spawn bundle), so the emergent kinetic term is negligible; instead this
/// deals the bullet's authored [`ProjectileDamage`], scaled only by the CLOSING
/// SPEED curve of its type, and triggers `HealthApplyDamage` itself.
///
/// The closing speed is the round's own arrival energy, so it is computed here
/// rather than at the muzzle: the same relative-velocity term
/// `on_impact_collision_deal_damage` uses for a ram, projected onto the round's
/// line of flight. A charging shooter's rounds arrive faster; a fleeing
/// target's slower.
///
/// [`spend_piercing_damage`] then decides the round's own fate. A Kinetic round
/// that merely dents its target is expended on it, as it always was - that
/// despawn is what stops a sensor round crossing the target and damaging every
/// event-enabled collider along its line - and one that DESTROYS its target
/// flies on with what its budget had left. A Pierce round is not stopped by
/// being alive: it pays that layer's thickness out of its power and rakes on
/// through. `amount` is zeroed when a round stops because the despawn is
/// deferred: a second contact in the same flush must find nothing left.
///
/// The OTHER side must not itself be a pure volume: scenario trigger
/// areas, beacon spheres and blast shells are Sensor colliders with
/// collision events enabled, and expending rounds at a beacon's 70u
/// trigger boundary made the pirate un-hittable while it patrolled near
/// one. A sensor-vs-sensor pair is two intangibles crossing - nothing to
/// expend on.
pub(super) fn resolve_bullet_hit(
    collision: On<CollisionStart>,
    mut commands: Commands,
    mut q_bullets: Query<Option<&mut ProjectileDamage>, With<TurretBulletProjectileMarker>>,
    q_sensors: Query<(), With<Sensor>>,
    q_health: Query<&Health>,
    q_events: Query<(), With<CollisionEventsEnabled>>,
    q_velocity: Query<&LinearVelocity>,
) {
    // Avian raises one CollisionStart per EVENT-ENABLED collider, so a contact
    // with events on both sides - a production round, which carries its own,
    // against any health-bearing collider, which the integrity hook enables -
    // arrives TWICE with the orderings swapped. Only the ordering that names the
    // round first may act, or one contact is charged twice. The mirrored event
    // does not always exist (a round with no events of its own is reported only
    // as the target's `collider2`), so the second ordering stays for that case.
    let mirrored = q_events.contains(collision.collider1) && q_events.contains(collision.collider2);
    let pairs: &[(Option<Entity>, Option<Entity>, Entity)] = if mirrored {
        &[(collision.body1, collision.body2, collision.collider2)]
    } else {
        &[
            (collision.body1, collision.body2, collision.collider2),
            (collision.body2, collision.body1, collision.collider1),
        ]
    };
    for &(body, other_body, other_collider) in pairs {
        let Some(body) = body else {
            continue;
        };
        // Membership gate: is this body a turret bullet? (`damage` is None only
        // for bare test rigs; production bullets always carry it.)
        let Ok(damage) = q_bullets.get_mut(body) else {
            continue;
        };
        if q_sensors.contains(other_collider) {
            // A trigger/blast volume: the round flies on through.
            continue;
        }
        let Some(mut damage) = damage else {
            trace!("resolve_bullet_hit: bullet {:?} expended", body);
            commands.entity(body).try_despawn();
            continue;
        };
        if damage.amount <= 0.0 {
            // Already spent earlier in this flush; its despawn is still queued.
            continue;
        }
        // Velocities live on the BODIES, not the colliders. A target with no
        // body of its own (or no velocity: a Static planetoid) counts as at
        // rest, which is what it is.
        let velocity = |entity: Option<Entity>| {
            entity
                .and_then(|entity| q_velocity.get(entity).ok())
                .map_or(Vec3::ZERO, |velocity| **velocity)
        };
        let closing = closing_speed(velocity(Some(body)), velocity(other_body));
        // The bullet is the source, carrying ProjectileOwner for threat
        // attribution. A collider with no Health is a wall to either type.
        let health = q_health.get(other_collider).ok();
        match spend_piercing_damage(
            &mut commands,
            other_collider,
            Some(body),
            health,
            *damage,
            closing,
        ) {
            Some(remaining) => {
                trace!(
                    "resolve_bullet_hit: bullet {:?} travels on ({:?}, {} power, {} layers)",
                    body,
                    remaining.kind,
                    remaining.power,
                    remaining.layers
                );
                *damage = remaining;
            }
            None => {
                trace!("resolve_bullet_hit: bullet {:?} expended", body);
                damage.amount = 0.0;
                commands.entity(body).try_despawn();
            }
        }
    }
}

/// A runaway-config backstop for the multi-shot loop: at 64 Hz ticks this
/// caps the effective fire rate at 512 rounds/s per barrel, far above any
/// authored turret; without it a zero-ish fire interval would spawn
/// unboundedly inside one tick.
const MAX_SHOTS_PER_TICK: u32 = 8;

pub(super) fn shoot_spawn_projectile(
    mut commands: Commands,
    time: Res<Time>,
    q_spaceship: Query<
        (
            &Position,
            &Rotation,
            &LinearVelocity,
            &AngularVelocity,
            &ComputedCenterOfMass,
            Option<&Allegiance>,
        ),
        With<SpaceshipRootMarker>,
    >,
    mut q_turret: Query<
        (
            Entity,
            &TurretSectionMuzzles,
            &ChildOf,
            &TurretSectionConfigHelper,
            Option<&LoadedBullet>,
            &TurretSectionInput,
            Option<&TurretSectionAimPoint>,
            Option<&mut SectionAmmo>,
            Option<&mut SectionReload>,
        ),
        (With<TurretSectionMarker>, Without<SectionInactiveMarker>),
    >,
    mut q_muzzle: Query<&mut TurretSectionBarrelFireState, With<TurretSectionBarrelMuzzleMarker>>,
    q_chain: Query<(&Transform, &ChildOf)>,
    q_hot: Query<&WeaponsHot>,
    q_defense: Query<(&PointDefenseMount, &TurretDefenseTarget)>,
) {
    let dt = time.delta_secs();
    for (
        turret,
        muzzles,
        ChildOf(spaceship),
        config,
        loaded,
        input,
        aim_point,
        mut ammo,
        mut reload,
    ) in &mut q_turret
    {
        // The weapons safety is a LIVE predicate: a managed ship (player,
        // mirrored AI) cannot fire
        // while SAFE even mid-held-trigger - the input bool is latched, so a
        // press-time gate alone would leak. Unmanaged ships (no WeaponsHot -
        // bare example turrets) fire freely.
        //
        // ONE exemption, and it is what autonomous point defence IS: the
        // safety is the PLAYER's trigger discipline, and a COLD battery is
        // exactly the state the Flight Computer borrows a mount in. Narrow by
        // construction - it needs the computer's ownership tier AND a live
        // assignment, and an assignment can only ever be an inbound hostile
        // torpedo.
        let (mount, assignment) = match q_defense.get(turret) {
            Ok((mount, assignment)) => (Some(mount), Some(assignment)),
            Err(_) => (None, None),
        };
        if !flight_computer_works(mount, assignment)
            && q_hot.get(*spaceship).is_ok_and(|hot| !hot.0)
        {
            continue;
        }
        // The fired round: the runtime LoadedBullet slot if present (production
        // turrets carry one), else the authored config default (bare test rigs
        // and any turret not built via `turret_section`).
        let (bullet_kind, bullet_damage) = loaded
            .map(|loaded| (loaded.kind, loaded.damage))
            .unwrap_or((config.bullet_kind, config.bullet_damage));

        // The spaceship pose is a per-TURRET quantity: every muzzle spawns
        // relative to the same root avian pose, so read it once before the
        // muzzle loop.
        let Ok((position, rotation, lin_vel, ang_vel, center, allegiance)) =
            q_spaceship.get(*spaceship)
        else {
            error!(
                "shoot_spawn_projectile: entity {:?} not found in q_spaceship",
                spaceship
            );
            continue;
        };

        // Copy the muzzle Entity list out of the component BEFORE the inner
        // loop, so `q_muzzle` (fire timers) and `commands` are free to borrow
        // while we iterate. Every muzzle draws from the ONE `SectionAmmo` below:
        // a shared magazine, not a per-barrel one.
        let muzzle_entities: Vec<Entity> = muzzles.0.clone();
        for muzzle in muzzle_entities {
            let Ok(mut fire_state) = q_muzzle.get_mut(muzzle) else {
                error!(
                    "shoot_spawn_projectile: entity {:?} not found in q_muzzle",
                    muzzle
                );
                continue;
            };

            // The cooldown elapses on the fixed clock whether or not the trigger
            // is held (absorbed from the old update_barrel_fire_state, which also
            // removed an unordered-tick-vs-shoot ambiguity in the Update set).
            // `elapsed` is sampled BEFORE the tick because a Once timer clamps at
            // its duration: `before + dt - interval` is the only way to recover
            // how far past due the shot came within this tick window.
            let before = fire_state.elapsed_secs();
            fire_state.tick(Duration::from_secs_f32(dt));

            if !**input || !fire_state.is_finished() {
                continue;
            }

            // Out of ammo: the ONE shared magazine gates every muzzle. A mag that
            // empties on an earlier muzzle's burst this tick stops the later ones
            // too. A turret with no `SectionAmmo` (unlimited) is never gated here,
            // so the pre-ammo behavior is untouched.
            if ammo.as_deref().is_some_and(SectionAmmo::is_empty) {
                continue;
            }

            // Muzzle pose on the RAW physics clock: the root's avian pose
            // composed with the local mount chain (turret -> rotators ->
            // muzzle). This system runs in FixedUpdate, where `GlobalTransform`
            // still holds the previous frame's EASED render pose - sampling it
            // scattered spawn points by up to a tick of ship motion per shot.
            // The rotator locals are written by the
            // Update-schedule aim systems; reading them here means the aim is at
            // most one frame old, which is control input staleness, not a
            // velocity-proportional error.
            let Some((muzzle_local_pos, muzzle_local_rot)) =
                local_pose_in_root(muzzle, *spaceship, &q_chain)
            else {
                error!(
                    "shoot_spawn_projectile: muzzle {:?} is not a descendant of ship {:?}",
                    muzzle, spaceship
                );
                continue;
            };
            let projectile_rotation = rotation.0 * muzzle_local_rot;
            let muzzle_position = position.0 + rotation.mul_vec3(muzzle_local_pos);
            let muzzle_direction = projectile_rotation * Vec3::NEG_Z;

            // FIRE ONLY WHERE THE BARREL POINTS. The trigger is intent; this is
            // the barrel's own answer, per MUZZLE, so a mount that cannot bear
            // holds while its siblings keep shooting. It subsumes reachability:
            // hinges that cannot swing onto the target never converge, so the
            // muzzle is never inside the cone (see `muzzle_on_target`).
            //
            // Graded against the LEAD point the turret actually steers to, not
            // the target's current position: a turret correctly leading a
            // crossing target is by construction off the raw bearing.
            //
            // A turret with no aim point has been told nothing to hit and fires
            // freely - the same fail-open as an unmanaged ship above, so bare
            // rigs and example turrets are untouched.
            if let Some(aim) = aim_point.and_then(|aim| **aim) {
                if !muzzle_on_target(muzzle_direction, muzzle_position, aim) {
                    continue;
                }
            }

            // Inherit the full motion of the muzzle, not just the ship's linear
            // velocity: a muzzle offset from the center of mass of a rotating
            // ship also swings tangentially. avian's `ComputedCenterOfMass` is
            // body-local; lift it with the same raw pose as everything else.
            let center_of_mass = position.0 + rotation.mul_vec3(**center);
            let inertia_vel =
                rigid_body_point_velocity(**lin_vel, **ang_vel, center_of_mass, muzzle_position);
            let muzzle_exit_velocity = muzzle_direction * config.muzzle_speed;
            let linear_velocity = muzzle_exit_velocity + inertia_vel;

            let interval = fire_state.duration().as_secs_f32();
            // How far past due the shot came within this tick window. A timer
            // that finished in an earlier tick (idle barrel, trigger just
            // pulled) reads `before == interval`, so the clamp lands the first
            // shot on this tick's start - fire NOW, exactly the old semantics.
            let mut excess = (before + dt - interval).clamp(0.0, dt);

            for _ in 0..MAX_SHOTS_PER_TICK {
                // Spend one round per bullet. A magazine that runs dry mid-burst
                // stops the stream exactly at zero (a high fire rate can queue
                // several shots per tick, so the gate above is not enough on its
                // own). Unlimited turrets carry no `SectionAmmo` and never break.
                if let Some(ammo) = ammo.as_deref_mut() {
                    if !ammo.try_consume() {
                        break;
                    }
                    if let Some(reload) = reload.as_deref_mut() {
                        reload.on_shot();
                    }
                }

                // Sub-tick exactness: a shot due `lead` seconds into this tick
                // starts one lead-time of muzzle-exit travel BEHIND the muzzle,
                // so after this tick's integration it sits exactly where a
                // bullet fired at the due moment would - the stream stays
                // uniformly spaced at any ship velocity. (The ship-motion terms
                // cancel: spawn = muzzle + (v_muzzle - v_bullet) * lead, and
                // v_bullet - v_muzzle is the muzzle exit velocity.)
                let lead = dt - excess;
                let projectile_transform = Transform {
                    translation: muzzle_position - muzzle_exit_velocity * lead,
                    rotation: projectile_rotation,
                    ..default()
                };

                let mut projectile = commands.spawn((
                    Name::new("Turret Projectile"),
                    TurretBulletProjectileMarker,
                    ProjectileOwner(*spaceship),
                    projectile_transform,
                    RigidBody::Dynamic,
                    LinearVelocity(linear_velocity),
                    // Sensor: the impact-damage observer computes damage from
                    // masses and velocities, never from the solver contact -
                    // so a bullet needs NO physical contact response, and a
                    // solid one was the knockback bug (mass 0.1 at 100 u/s
                    // plus restitution shoved a ~4-mass ship ~3 u/s per hit;
                    // playtest round 2 finding 2). resolve_bullet_hit
                    // keeps a sensor round from crossing on through every
                    // collider behind the first - it flies on only through
                    // what it destroyed, and only while its damage budget
                    // lasts. CollisionEventsEnabled is
                    // carried by the BULLET because the other side may not
                    // have it: an invulnerable planetoid's collider has no
                    // Health, so collision events are never enabled on it, and an
                    // event-less sensor pair raises nothing - rounds tunneled
                    // straight through solid cover (review R1.2 MAJOR).
                    // Nested tuple: bundle arity.
                    (Collider::sphere(0.05), Sensor, CollisionEventsEnabled),
                    ActiveCollisionHooks::FILTER_PAIRS,
                    // Near-zero mass so the emergent kinetic term (mass x velocity)
                    // vanishes; nova's authored ProjectileDamage is the only weapon
                    // damage. Gravity is mass-independent, so flight is unaffected.
                    // Nested tuple: bundle arity.
                    (
                        Mass(NEUTRALIZED_BULLET_MASS),
                        // A Dynamic body needs finite, non-zero ANGULAR INERTIA too, or
                        // avian warns "no mass or inertia" once per fired round and
                        // risks NaN. The Sensor collider above
                        // contributes no mass properties, and the neutralized `Mass`
                        // carries no inertia of its own, so derive a matching sphere
                        // inertia from the same shape + mass. The bullet never takes
                        // torque (sensor, authored damage, no angular velocity), so the
                        // value only has to be VALID, not tuned - flight is unaffected.
                        AngularInertia::from_shape(
                            &Collider::sphere(0.05),
                            NEUTRALIZED_BULLET_MASS,
                        ),
                        // The fired round comes from the turret's loaded-ammo slot,
                        // not a hardcoded type, so a future ammo switch changes
                        // what this stamps. The closing-speed scaling is applied
                        // at the HIT, not here - the target is not known yet.
                        ProjectileDamage::new(bullet_damage, bullet_kind),
                    ),
                    TurretSectionPartOf(turret),
                    TurretSectionMuzzleEntity(muzzle),
                    BulletProjectileRenderMesh(config.projectile_render_mesh.clone()),
                    TempEntity(config.projectile_lifetime),
                    Visibility::Visible,
                    // Interpolation plus its render-clock seed: a body
                    // spawned mid-tick misses
                    // FixedFirst, so its easing `start` stays None and the first
                    // rendered frame would show the RAW spawn pose (sub-tick
                    // lead offset and all) while the rest of the world renders
                    // EASED - one visible frame of muzzle pop, cross-stream
                    // error up to a tick of ship motion. Seeding `start` with
                    // the tick-start muzzle pose (no lead offset) puts the first
                    // frame at lerp(muzzle, raw_end, alpha): attached to the
                    // rendered barrel, and only ever ahead of it along the
                    // stream. FixedLast fills `end` with this tick's integrated
                    // raw pose as usual, and the teleport-reset guard keeps the
                    // seed because the written Transform equals `end` exactly.
                    (
                        TransformInterpolation,
                        TranslationEasingState {
                            start: Some(muzzle_position),
                            end: None,
                        },
                        RotationEasingState {
                            start: Some(projectile_rotation),
                            end: None,
                        },
                    ),
                ));
                // The projectile COPIES the shooter's allegiance instead of
                // resolving through ProjectileOwner at read time: it stays
                // attributable even if the owner dies mid-flight, and consumers
                // stay single-query.
                if let Some(&allegiance) = allegiance {
                    projectile.insert(allegiance);
                }

                // Re-arm and immediately advance by the leftover: if the excess
                // spans another full interval the barrel fires again this tick
                // (fire rates above the tick rate keep their true cadence).
                fire_state.reset();
                fire_state.tick(Duration::from_secs_f32(excess));
                if !fire_state.is_finished() {
                    break;
                }
                excess -= interval;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use bevy::time::TimeUpdateStrategy;

    use super::{super::config::default_joint_speed, *};

    /// A minimal app that runs ONLY `shoot_spawn_projectile` on a manual clock,
    /// so ammo behavior is observed by counting spawned bullets without the full
    /// physics/render stack. `dt` far larger than the fire interval keeps the
    /// barrel timer finished every tick, so firing is gated by ammo alone.
    fn firing_app(dt: f32) -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.insert_resource(TimeUpdateStrategy::ManualDuration(
            std::time::Duration::from_secs_f32(dt),
        ));
        app.add_systems(Update, shoot_spawn_projectile);
        app
    }

    /// Spawn a ship + one turret holding its trigger, optionally with a finite
    /// magazine. The muzzle is parented directly under the ship so
    /// `local_pose_in_root` resolves in one hop; the fire timer starts finished
    /// so the first tick can fire. `q_spaceship` reads avian `Position`/
    /// `Rotation`, so those are inserted directly (no physics stepping).
    fn spawn_firing_turret(app: &mut App, ammo: Option<u32>) -> Entity {
        // The default turret's single muzzle fires at 100 rounds/s.
        let interval = 1.0 / 100.0;
        let mut timer = Timer::from_seconds(interval, TimerMode::Once);
        timer.finish();

        let world = app.world_mut();
        let ship = world
            .spawn((
                SpaceshipRootMarker,
                Position(Vec3::ZERO),
                Rotation::default(),
                LinearVelocity(Vec3::ZERO),
                AngularVelocity(Vec3::ZERO),
                ComputedCenterOfMass(Vec3::ZERO),
            ))
            .id();
        let turret = world
            .spawn((
                TurretSectionMarker,
                TurretSectionConfigHelper(TurretSectionConfig::default()),
                TurretSectionInput(true),
                Transform::default(),
                ChildOf(ship),
            ))
            .id();
        let muzzle = world
            .spawn((
                TurretSectionBarrelMuzzleMarker,
                TurretSectionBarrelFireState(timer),
                Transform::default(),
                ChildOf(turret),
            ))
            .id();
        world.entity_mut(turret).insert((
            TurretSectionMuzzleEntity(muzzle),
            TurretSectionMuzzles(vec![muzzle]),
        ));
        if let Some(capacity) = ammo {
            world.entity_mut(turret).insert(SectionAmmo::new(capacity));
        }
        turret
    }

    fn bullet_count(app: &mut App) -> usize {
        app.world_mut()
            .query_filtered::<Entity, With<TurretBulletProjectileMarker>>()
            .iter(app.world())
            .count()
    }

    /// The world-space point every aim-gate test below fires at: dead ahead of
    /// the ship at a plain gunfight range.
    const GATE_TEST_AIM: Vec3 = Vec3::new(0.0, 0.0, -100.0);

    /// A ship at the origin with `bearings.len()` turrets, each holding its
    /// trigger and each aimed at [`GATE_TEST_AIM`], whose muzzle is swung
    /// `bearing` degrees off that line - which is what a mount pinned at a hinge
    /// stop looks like from the fire path. Returns (ship, turrets, muzzles).
    fn spawn_aimed_turrets(app: &mut App, bearings: &[f32]) -> (Entity, Vec<Entity>, Vec<Entity>) {
        let world = app.world_mut();
        let ship = world
            .spawn((
                SpaceshipRootMarker,
                Position(Vec3::ZERO),
                Rotation::default(),
                LinearVelocity(Vec3::ZERO),
                AngularVelocity(Vec3::ZERO),
                ComputedCenterOfMass(Vec3::ZERO),
            ))
            .id();
        let (mut turrets, mut muzzles) = (Vec::new(), Vec::new());
        for &bearing in bearings {
            let turret = world
                .spawn((
                    TurretSectionMarker,
                    TurretSectionConfigHelper(TurretSectionConfig::default()),
                    TurretSectionInput(true),
                    TurretSectionAimPoint(Some(GATE_TEST_AIM)),
                    Transform::default(),
                    ChildOf(ship),
                ))
                .id();
            let muzzle = world
                .spawn((
                    TurretSectionBarrelMuzzleMarker,
                    TurretSectionBarrelFireState({
                        let mut timer = Timer::from_seconds(1.0 / 100.0, TimerMode::Once);
                        timer.finish();
                        timer
                    }),
                    Transform::from_rotation(Quat::from_rotation_y(bearing.to_radians())),
                    ChildOf(turret),
                ))
                .id();
            world.entity_mut(turret).insert((
                TurretSectionMuzzleEntity(muzzle),
                TurretSectionMuzzles(vec![muzzle]),
            ));
            turrets.push(turret);
            muzzles.push(muzzle);
        }
        (ship, turrets, muzzles)
    }

    /// Swing one turret's muzzle onto a new bearing, as the aim chain would.
    fn slew_muzzle(app: &mut App, muzzle: Entity, bearing: f32) {
        app.world_mut()
            .get_mut::<Transform>(muzzle)
            .unwrap()
            .rotation = Quat::from_rotation_y(bearing.to_radians());
    }

    #[test]
    fn a_mount_that_cannot_bear_holds_fire_while_its_siblings_shoot() {
        // THE BUG. A turret fired the instant the trigger was held, wherever the
        // barrel pointed, so a mount whose hinges cannot swing onto the target -
        // the owner's port gun ordered onto something off the starboard side -
        // emptied its magazine into its own hull.
        //
        // The other half is the one that matters more: the gate is PER MUZZLE,
        // so the mount that CAN bear keeps shooting. A hard gate that silenced
        // the whole ship would be a worse bug than the one being fixed.
        let mut app = firing_app(1.0);
        let (_, _, muzzles) = spawn_aimed_turrets(&mut app, &[0.0, 40.0]);

        for _ in 0..10 {
            app.update();
        }

        let per = bullets_per_muzzle(&mut app, &muzzles);
        assert!(
            per[0] > 0,
            "the mount that bears on the target must keep shooting, got {per:?}"
        );
        assert_eq!(
            per[1], 0,
            "the mount pinned 40 deg off it must not fire a round, got {per:?}"
        );
    }

    #[test]
    fn a_turret_mid_slew_holds_fire_until_it_is_on_the_aim_point() {
        // "Sometimes I shoot while turning and I would say let's not do that."
        // Walk one barrel down onto the aim point and record where the stream
        // starts: nothing leaves the muzzle until the error is inside the
        // on-target cone, and then it fires at once (the fire timer is not
        // reset by the hold, so a ready gun shoots the moment it bears).
        let mut app = firing_app(1.0);
        let (_, _, muzzles) = spawn_aimed_turrets(&mut app, &[30.0]);

        for bearing in [30.0, 20.0, 10.0, 5.0, 2.0] {
            slew_muzzle(&mut app, muzzles[0], bearing);
            app.update();
            assert_eq!(
                bullet_count(&mut app),
                0,
                "a barrel {bearing} deg off the aim point must not fire"
            );
        }

        // Settled: half a degree, inside the cone and inside what a converged
        // turret actually holds.
        slew_muzzle(&mut app, muzzles[0], 0.5);
        app.update();
        assert!(
            bullet_count(&mut app) > 0,
            "an on-target barrel must fire immediately once it bears"
        );
    }

    #[test]
    fn a_low_framerate_turret_tracking_a_crosser_reaches_the_gate_and_fires() {
        // THE BUG (task 20260816-184718), end to end: the per-frame aim damp
        // left ~1.8 deg of tracking lag at 14 fps - above the 0.92 deg bearing
        // gate - so a PDC on a struggling machine refused to fire at a crosser
        // it was tracking fine at 60 fps. With the dt-based decay the lag at
        // 1/14 stays inside the gate and the turret shoots. Full production
        // chain on a 1/14 clock: lead solve + joint CCD + controller sync +
        // the gated fire path.
        use nova_gameplay::transform::prelude::SmoothLookRotationPlugin;

        let dt = 1.0 / 14.0;
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, TransformPlugin, SmoothLookRotationPlugin));
        app.add_observer(insert_turret_section);
        app.insert_resource(TimeUpdateStrategy::ManualDuration(
            std::time::Duration::from_secs_f32(dt),
        ));
        // Production runs the fire path on the fixed clock BEFORE the Update
        // joint sync, so the gate reads a muzzle pose one frame staler than
        // the aim point; keep that staleness or the test understates the lag.
        app.add_systems(
            Update,
            (shoot_spawn_projectile, sync_turret_joint_rotation).chain(),
        );
        // The same aim-before-controller edge TurretSectionPlugin declares.
        app.add_systems(
            PostUpdate,
            (update_turret_aim_point, update_turret_target_joints_system)
                .chain()
                .before(SmoothLookRotationSystems::Sync),
        );

        // The ship carries every component both the aim inherit query and the
        // raw-clock fire path read.
        let ship = app
            .world_mut()
            .spawn((
                SpaceshipRootMarker,
                Transform::IDENTITY,
                Position(Vec3::ZERO),
                Rotation::default(),
                LinearVelocity(Vec3::ZERO),
                AngularVelocity(Vec3::ZERO),
                ComputedCenterOfMass(Vec3::ZERO),
            ))
            .id();
        // A crossing target at gunfight range: 100 u out, 12 u/s across the
        // bow (~7 deg/s of bearing rate at closest approach - the defect's
        // measured regime, where the old lag read ~1.4 deg and held fire).
        let velocity = Vec3::new(12.0, 0.0, 0.0);
        let target_at = |t: f32| Vec3::new(-30.0, 0.0, -100.0) + velocity * t;

        // Trigger COLD until the aim point exists: a turret with no aim point
        // fires freely (fail-open), which would pass this test vacuously.
        let turret = app
            .world_mut()
            .spawn(turret_section(TurretSectionConfig::default()))
            .id();
        app.world_mut().entity_mut(turret).insert((
            ChildOf(ship),
            Transform::IDENTITY,
            TurretSectionTargetInput(Some(target_at(0.0))),
            TurretSectionTargetVelocity(velocity),
        ));
        app.world_mut().flush();

        // The first manual-clock update has dt 0; burn it before tracking (it
        // also resolves the first aim point).
        app.update();

        // Track trigger-COLD for 2.5 s first: the acquisition slew sweeps the
        // barrel THROUGH the aim cone once while it catches the crosser, and
        // even the laggy pre-fix turret fired a burst on that crossing. The
        // claim under test is STEADY tracking, so only arm once settled.
        let steps = (2.5 / dt).round() as u32;
        for k in 1..=steps {
            app.world_mut()
                .entity_mut(turret)
                .insert(TurretSectionTargetInput(Some(target_at(k as f32 * dt))));
            app.update();
        }
        assert_eq!(bullet_count(&mut app), 0, "trigger is cold while settling");

        app.world_mut()
            .entity_mut(turret)
            .insert(TurretSectionInput(true));
        for k in steps + 1..=2 * steps {
            app.world_mut()
                .entity_mut(turret)
                .insert(TurretSectionTargetInput(Some(target_at(k as f32 * dt))));
            app.update();
        }

        assert!(
            bullet_count(&mut app) > 0,
            "a turret in steady pursuit of a crosser at 14 fps must sit inside \
             the bearing gate and fire (pre-fix it lagged ~1.4 deg, outside \
             the 0.92 deg gate, and held fire)"
        );
    }

    #[test]
    fn a_turret_with_no_aim_point_still_fires_freely() {
        // FAIL-OPEN, the same rule the weapons safety follows for an unmanaged
        // ship: a turret nobody has aimed (a bare rig, an example range, a mod
        // path that drives the trigger directly) has no bearing to be wrong
        // about, so the gate must not silence it.
        let mut app = firing_app(1.0);
        let (_, turrets, muzzles) = spawn_aimed_turrets(&mut app, &[40.0]);
        app.world_mut()
            .entity_mut(turrets[0])
            .insert(TurretSectionAimPoint(None));

        for _ in 0..3 {
            app.update();
        }

        assert!(
            bullets_per_muzzle(&mut app, &muzzles)[0] > 0,
            "a turret with nothing to aim at must keep its pre-gate behavior"
        );
    }

    #[test]
    fn a_turret_with_ammo_fires_exactly_its_magazine_then_stops() {
        // The core ammo claim: `try_consume` hard-caps total bullets at the
        // magazine size regardless of sub-tick fire timing, so an exact count is
        // a robust assertion. Ten wide ticks would fire far more than three
        // bullets unlimited (see the A/B below).
        let mut app = firing_app(1.0);
        let turret = spawn_firing_turret(&mut app, Some(3));

        for _ in 0..10 {
            app.update();
        }

        assert_eq!(
            bullet_count(&mut app),
            3,
            "a 3-round magazine must fire exactly three bullets, ever"
        );
        let ammo = app
            .world()
            .entity(turret)
            .get::<SectionAmmo>()
            .expect("the turret keeps its magazine");
        assert_eq!(
            ammo.rounds, 0,
            "the magazine must read empty after firing out"
        );
    }

    /// The number of fired bullets stamped with each given muzzle entity.
    fn bullets_per_muzzle(app: &mut App, muzzles: &[Entity]) -> Vec<usize> {
        let stamped: Vec<Entity> = app
            .world_mut()
            .query_filtered::<&TurretSectionMuzzleEntity, With<TurretBulletProjectileMarker>>()
            .iter(app.world())
            .map(|m| **m)
            .collect();
        muzzles
            .iter()
            .map(|&muzzle| stamped.iter().filter(|&&s| s == muzzle).count())
            .collect()
    }

    #[test]
    fn a_twin_barrel_fires_both_muzzles_over_one_shared_magazine() {
        // MULTI-MUZZLE + SHARED MAG: a turret whose barrel
        // joint carries TWO muzzles fires BOTH, and both draw from the ONE section
        // magazine. The key claim is the SHARED magazine: N muzzles do NOT each get
        // their own ammo pool - a 3-round mag yields 3 bullets TOTAL across both
        // barrels, not 3 per barrel (6). Built via the spawn observer so both
        // muzzle entities, their fire timers and `TurretSectionMuzzles` all exist.
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.insert_resource(TimeUpdateStrategy::ManualDuration(
            std::time::Duration::from_secs_f32(1.0),
        ));
        app.add_observer(insert_turret_section);
        app.add_systems(Update, shoot_spawn_projectile);

        // base(fixed) -> yaw(Y) -> pitch(X) -> barrel(fixed) with TWO muzzle
        // children at symmetric lateral offsets. fire_rate 10, shared mag of 3.
        let muzzle = |x: f32| TurretJoint {
            offset: Vec3::new(x, 0.0, -0.5),
            axis: None,
            speed: default_joint_speed(),
            min: None,
            max: None,
            render_mesh: None,
            render_mesh_transform: None,
            muzzle: Some(MuzzleConfig {
                fire_rate: 10.0,
                muzzle_effect: None,
            }),
            children: vec![],
        };
        let barrel = TurretJoint {
            offset: Vec3::new(0.0, 0.0, 0.0),
            axis: None,
            speed: default_joint_speed(),
            min: None,
            max: None,
            render_mesh: None,
            render_mesh_transform: None,
            muzzle: None,
            children: vec![muzzle(0.1), muzzle(-0.1)],
        };
        let config = TurretSectionConfig {
            root: barrel,
            ammo_capacity: Some(3),
            ..default()
        };

        let ship = app
            .world_mut()
            .spawn((
                SpaceshipRootMarker,
                Position(Vec3::ZERO),
                Rotation::default(),
                LinearVelocity(Vec3::ZERO),
                AngularVelocity(Vec3::ZERO),
                ComputedCenterOfMass(Vec3::ZERO),
            ))
            .id();
        let turret = app.world_mut().spawn(turret_section(config)).id();
        app.world_mut().entity_mut(turret).insert((
            ChildOf(ship),
            Transform::default(),
            TurretSectionInput(true),
        ));
        app.world_mut().flush();

        // The two muzzle entities the observer recorded, in DFS order.
        let muzzles = app
            .world()
            .entity(turret)
            .get::<TurretSectionMuzzles>()
            .expect("the turret records its muzzles")
            .0
            .clone();
        assert_eq!(muzzles.len(), 2, "the twin barrel must record two muzzles");

        // Hold the trigger for far more ticks than the magazine can supply.
        for _ in 0..10 {
            app.update();
        }

        let per = bullets_per_muzzle(&mut app, &muzzles);
        assert!(
            per[0] > 0 && per[1] > 0,
            "both muzzles must produce bullets, got {per:?}"
        );
        assert_eq!(
            per[0] + per[1],
            3,
            "the magazine is SHARED: 3 rounds total across both barrels, not per \
             barrel, got {per:?}"
        );
        assert_eq!(
            bullet_count(&mut app),
            3,
            "exactly the shared magazine's worth of bullets ever spawn"
        );
        let ammo = app
            .world()
            .entity(turret)
            .get::<SectionAmmo>()
            .expect("the turret keeps its magazine");
        assert_eq!(
            ammo.rounds, 0,
            "the shared magazine reads empty after firing out"
        );
    }

    /// Every fired round is a Dynamic body, so avian needs it to have finite,
    /// non-zero mass AND angular inertia - otherwise it logs "no mass or inertia"
    /// once per shot and warns of NaN. The Sensor collider
    /// contributes no mass properties and the neutralized `Mass` carries no
    /// inertia of its own, so the spawn adds an explicit sphere `AngularInertia`.
    /// Fire a real round through the production path under physics and read what
    /// avian actually COMPUTED (not just that a component is present).
    #[test]
    fn a_fired_bullet_has_finite_nonzero_mass_and_inertia() {
        use nova_gameplay::test_support::{settle, unfinished_integrity_physics_app_with};

        // A physics app so avian's mass-property systems actually run; the helper
        // sets a 1/60 s manual step, and `settle` steps a few times (the first
        // fires the round; the rest let avian finalize the new body's masses).
        let mut app = unfinished_integrity_physics_app_with(PhysicsPlugins::default());
        app.add_systems(Update, shoot_spawn_projectile);
        app.finish();

        spawn_firing_turret(&mut app, Some(1));
        settle(&mut app);

        let world = app.world_mut();
        let (mass, inertia) = world
            .query_filtered::<(&ComputedMass, &ComputedAngularInertia), With<TurretBulletProjectileMarker>>()
            .single(world)
            .expect("exactly one fired bullet exists");

        let m = mass.value();
        assert!(
            m.is_finite() && m > 0.0,
            "a fired bullet needs finite non-zero mass, got {m}"
        );
        let (principal, _frame) = inertia.principal_angular_inertia_with_local_frame();
        assert!(
            principal.is_finite() && principal.min_element() > 0.0,
            "a fired bullet needs finite non-zero angular inertia on every axis \
             (else avian logs 'no mass or inertia' per shot and risks NaN), got {principal:?}"
        );
    }

    #[test]
    fn a_turret_without_ammo_keeps_firing_past_a_magazine() {
        // A/B control for the gate: the identical rig with no `SectionAmmo`
        // fires every tick, well past three bullets - proof that ammo, not some
        // other limit, stopped the stream above and that unlimited is the opt-in
        // default.
        let mut app = firing_app(1.0);
        spawn_firing_turret(&mut app, None);

        for _ in 0..10 {
            app.update();
        }

        assert!(
            bullet_count(&mut app) > 3,
            "an unlimited turret must not be capped at a magazine size, got {}",
            bullet_count(&mut app)
        );
    }

    #[test]
    fn an_auto_reloading_turret_fires_again_after_running_dry() {
        // End-to-end recovery: a finite turret fires out its 3-round magazine,
        // then the reload cycle refills it and it fires MORE than one magazine
        // over time - the whole point of auto-reload.
        // Contrast with `a_turret_with_ammo_fires_exactly_its_magazine_then_stops`,
        // the same rig with no reload, which caps at 3 forever.
        let mut app = firing_app(1.0);
        app.add_systems(Update, crate::sections::ammo::tick_section_reload);
        let turret = spawn_firing_turret(&mut app, Some(3));
        // Idle batch reload; 0.2s is under the clock's 0.25s per-tick clamp so
        // a spent magazine refills shortly after the final shot.
        app.world_mut()
            .entity_mut(turret)
            .insert(SectionReload::from_config(SectionReloadConfig {
                delay: 0.2,
                amount: 3,
            }));

        for _ in 0..20 {
            app.update();
        }

        assert!(
            bullet_count(&mut app) > 3,
            "an auto-reloading turret must fire past a single magazine, got {}",
            bullet_count(&mut app)
        );
    }

    #[test]
    fn turret_section_seeds_the_loaded_bullet_slot_from_config() {
        // The ammo slot is authored from config: bullet_kind/bullet_damage seed
        // LoadedBullet, and a default turret loads Kinetic.
        let mut world = World::new();
        let pierce = world
            .spawn(turret_section(TurretSectionConfig {
                bullet_kind: DamageType::Pierce,
                bullet_damage: 7.0,
                ..default()
            }))
            .id();
        let loaded = world
            .entity(pierce)
            .get::<LoadedBullet>()
            .expect("turret_section inserts a LoadedBullet slot");
        assert_eq!(loaded.kind, DamageType::Pierce);
        assert_eq!(loaded.damage, 7.0);

        let default_turret = world
            .spawn(turret_section(TurretSectionConfig::default()))
            .id();
        assert_eq!(
            world
                .entity(default_turret)
                .get::<LoadedBullet>()
                .unwrap()
                .kind,
            DamageType::Kinetic,
            "catalog default loadout is Kinetic (feel-preserving)"
        );
    }

    #[test]
    fn insert_turret_section_snapshots_the_configs_fire_sound_onto_the_turret() {
        // The declaration half of the section-authored audio seam: a turret
        // whose CONFIG declares `fire_sound` must carry
        // that UNRESOLVED ref as a `TurretSectionFireSound` after the build
        // observer runs, so the audio module can resolve + play it. Pairs with the
        // audio-module test that resolves the component and plays its handle - the
        // two halves marry declaration -> component -> resolved playback. No
        // `AssetServer` needed here: the snapshot carries the ref, not a handle.
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_observer(insert_turret_section);

        let with_sound = app
            .world_mut()
            .spawn(turret_section(TurretSectionConfig {
                fire_sound: Some(AssetRef::from("base/sounds/turret_fire.wav")),
                dry_fire_sound: Some(AssetRef::from("base/sounds/dry_fire.wav")),
                ..default()
            }))
            .id();
        let without_sound = app
            .world_mut()
            .spawn(turret_section(TurretSectionConfig::default()))
            .id();
        app.world_mut().flush();

        assert_eq!(
            app.world()
                .entity(with_sound)
                .get::<TurretSectionFireSound>()
                .and_then(|s| s.0.as_ref())
                .and_then(|r| r.path()),
            Some("base/sounds/turret_fire.wav"),
            "the declared fire_sound must be snapshotted onto the turret"
        );
        assert_eq!(
            app.world()
                .entity(with_sound)
                .get::<TurretSectionDryFireSound>()
                .and_then(|s| s.0.as_ref())
                .and_then(|r| r.path()),
            Some("base/sounds/dry_fire.wav"),
            "the declared dry_fire_sound must be snapshotted onto the turret"
        );
        // The snapshot is unconditional (None passes through), so the audio side
        // reads one component shape whether or not a sound was authored.
        assert_eq!(
            app.world()
                .entity(without_sound)
                .get::<TurretSectionFireSound>()
                .map(|s| s.0.is_none()),
            Some(true),
            "a turret without a fire_sound still carries the component as None"
        );
    }

    #[test]
    fn fired_bullet_takes_the_loaded_slots_type_not_a_hardcoded_kind() {
        // Load a non-Kinetic round into the slot and confirm the fired bullet
        // carries it. Would fail if the fire path still stamped a hardcoded
        // Kinetic (the pre-slot behavior).
        let mut app = firing_app(1.0);
        let turret = spawn_firing_turret(&mut app, None);
        app.world_mut().entity_mut(turret).insert(LoadedBullet {
            kind: DamageType::Pierce,
            damage: 5.0,
        });

        for _ in 0..3 {
            app.update();
        }

        let dmg = *app
            .world_mut()
            .query_filtered::<&ProjectileDamage, With<TurretBulletProjectileMarker>>()
            .iter(app.world())
            .next()
            .expect("the turret fired at least one bullet");
        assert_eq!(
            dmg.kind,
            DamageType::Pierce,
            "the fired round must take the loaded slot's type, not a hardcoded Kinetic"
        );
        assert_eq!(dmg.amount, 5.0, "and the slot's authored damage");
        assert_eq!(
            (dmg.power, dmg.layers),
            (PIERCE_BASE_POWER, MAX_PIERCE_LAYERS),
            "a fresh round leaves the muzzle with full pierce power and layers"
        );
    }

    /// A ready-to-fire ship + turret + muzzle rig for `shoot_spawn_projectile`,
    /// with the shooter's allegiance as given (`None` = unaligned shooter).
    /// The ship carries the raw avian pose and the muzzle hangs in its
    /// `ChildOf` tree, matching what the raw-clock spawn path reads.
    fn spawn_firing_rig(world: &mut World, allegiance: Option<Allegiance>) {
        let mut ship = world.spawn((
            SpaceshipRootMarker,
            Transform::default(),
            Position(Vec3::ZERO),
            Rotation::default(),
            LinearVelocity(Vec3::ZERO),
            AngularVelocity(Vec3::ZERO),
            ComputedCenterOfMass(Vec3::ZERO),
        ));
        if let Some(allegiance) = allegiance {
            ship.insert(allegiance);
        }
        let ship = ship.id();
        let muzzle = world
            .spawn((
                TurretSectionBarrelMuzzleMarker,
                Transform::default(),
                ChildOf(ship),
                {
                    // Pre-expired so the very first run fires.
                    let mut timer = Timer::from_seconds(0.1, TimerMode::Once);
                    timer.tick(std::time::Duration::from_secs(1));
                    TurretSectionBarrelFireState(timer)
                },
            ))
            .id();
        world.spawn((
            TurretSectionMarker,
            ChildOf(ship),
            TurretSectionMuzzleEntity(muzzle),
            TurretSectionMuzzles(vec![muzzle]),
            TurretSectionConfigHelper(TurretSectionConfig::default()),
            TurretSectionInput(true),
        ));
    }

    fn spawned_projectile_allegiance(world: &mut World) -> Option<Allegiance> {
        use bevy::ecs::system::RunSystemOnce;
        world.init_resource::<Time>();
        world.run_system_once(shoot_spawn_projectile).unwrap();
        world
            .query_filtered::<Option<&Allegiance>, With<TurretBulletProjectileMarker>>()
            .iter(world)
            .next()
            .expect("a projectile spawned")
            .copied()
    }

    #[test]
    fn spawned_projectile_copies_the_shooter_allegiance() {
        // The bullet must read as the shooter's side (relation model): copied
        // at spawn so it stays attributable even if the shooter dies.
        let mut world = World::new();
        spawn_firing_rig(&mut world, Some(Allegiance::Enemy));
        assert_eq!(
            spawned_projectile_allegiance(&mut world),
            Some(Allegiance::Enemy)
        );
    }

    #[test]
    fn spawned_projectile_of_an_unaligned_shooter_carries_no_allegiance() {
        let mut world = World::new();
        spawn_firing_rig(&mut world, None);
        assert_eq!(spawned_projectile_allegiance(&mut world), None);
    }

    /// A live-physics rig for the raw-clock spawn tests: a fast-capable ship
    /// root with a turret child and muzzle grandchild (non-identity local
    /// offsets AND a slewed rotator angle, so the local-chain composition is
    /// exercised, not just translations). Uses the projectile collision
    /// hooks so bullets ignore their own ship like production.
    fn spawn_stream_rig(app: &mut App, fire_rate: f32) -> (Entity, Entity) {
        let ship = app
            .world_mut()
            .spawn((
                SpaceshipRootMarker,
                RigidBody::Dynamic,
                Transform::default(),
                // Production ships interpolate; a raw-clock regression on a
                // non-faithful rig would understate the old bug.
                TransformInterpolation,
                Collider::cuboid(1.0, 1.0, 1.0),
                ColliderDensity(1.0),
            ))
            .id();
        let turret = app
            .world_mut()
            .spawn((
                TurretSectionMarker,
                ChildOf(ship),
                Transform::from_xyz(0.0, 1.0, 0.0),
                // Trigger stays cold through settle(); tests arm it once the
                // rig's velocity is in place, so every bullet belongs to the
                // same stream.
                TurretSectionInput(false),
                // The muzzle child below carries the fire timer directly; the
                // config's per-muzzle rate is unused by this rig.
                TurretSectionConfigHelper(TurretSectionConfig {
                    muzzle_speed: 200.0,
                    ..default()
                }),
            ))
            .id();
        let muzzle = app
            .world_mut()
            .spawn((
                TurretSectionBarrelMuzzleMarker,
                ChildOf(turret),
                Transform::from_xyz(0.0, 0.0, -0.5).with_rotation(Quat::from_rotation_y(0.3)),
                TurretSectionBarrelFireState({
                    // Pre-expired: the first shot leaves on the first tick.
                    let mut timer = Timer::from_seconds(1.0 / fire_rate, TimerMode::Once);
                    timer.finish();
                    timer
                }),
            ))
            .id();
        app.world_mut().entity_mut(turret).insert((
            TurretSectionMuzzleEntity(muzzle),
            TurretSectionMuzzles(vec![muzzle]),
        ));
        (ship, turret)
    }

    fn arm_turret(app: &mut App, turret: Entity) {
        app.world_mut()
            .get_mut::<TurretSectionInput>(turret)
            .unwrap()
            .0 = true;
    }

    /// Bullets from a fast ship must form a uniformly spaced, collinear
    /// stream. The old Update-schedule spawn sampled
    /// the EASED muzzle pose at render-frame shot times with a static 0.01 s
    /// nudge, so each shot picked up a different fraction of a tick of ship
    /// motion - at 150 u/s the stream scattered by whole units ("bullets
    /// spew out"). On the raw clock with sub-tick lead compensation the
    /// inter-bullet spacing is exact: every consecutive delta equals
    /// Sensor bullets deal damage without knockback and die on the first
    /// hit (playtest round 2 finding 2). Before the Sensor change, a
    /// solid 0.1-mass round at 100 u/s shoved a unit-cube target ~2.5+
    /// u/s per hit (momentum 10 into the target mass, amplified by
    /// restitution 0.5) - "1 bullet sends you off like crazy". The emergent
    /// damage observer computes from masses and velocities, not the
    /// solver contact, so removing the contact response leaves damage
    /// intact. Delivery guards: the health drop proves the hit landed
    /// (a missed bullet would also read zero knockback), and the despawn
    /// proves a sensor round cannot sail on through everything behind
    /// the target.
    #[test]
    fn sensor_bullets_damage_without_knockback() {
        use nova_gameplay::{
            projectile_hooks::ProjectileHooks,
            test_support::{settle, unfinished_integrity_physics_app_with},
        };
        let mut app = unfinished_integrity_physics_app_with(
            PhysicsPlugins::default().with_collision_hooks::<ProjectileHooks>(),
        );
        app.add_observer(resolve_bullet_hit);
        app.finish();

        // A free-floating target with health: one body, one collider.
        let target = app
            .world_mut()
            .spawn((
                Name::new("target"),
                RigidBody::Dynamic,
                Transform::default(),
                Collider::cuboid(2.0, 2.0, 2.0),
                ColliderDensity(1.0),
                Health::new(100.0),
            ))
            .id();
        settle(&mut app);

        // A bullet with the OLD emergent-kinetic shape (Mass 0.1, no
        // ProjectileDamage) on purpose: this test isolates the physics-contact
        // behavior - knockback and no-tunnel-through - so it drives the
        // emergent damage rather than the typed path. The production bullet now
        // spawns near-zero mass + ProjectileDamage; its typed damage is covered
        // by `typed_bullet_applies_resistance_scaled_damage`.
        let bullet = app
            .world_mut()
            .spawn((
                Name::new("bullet"),
                TurretBulletProjectileMarker,
                RigidBody::Dynamic,
                Transform::from_translation(Vec3::Z * 5.0),
                Sensor,
                Collider::sphere(0.05),
                Mass(0.1),
                LinearVelocity(Vec3::NEG_Z * 100.0),
            ))
            .id();

        // 5u at 100 u/s: contact within ~0.05s; run a quarter second.
        for _ in 0..15 {
            app.update();
        }

        let health = app
            .world()
            .get::<Health>(target)
            .expect("target still exists")
            .current;
        assert!(
            health < 100.0,
            "delivery guard: the bullet must actually hit and damage, health {health}"
        );
        let speed = app
            .world()
            .get::<LinearVelocity>(target)
            .expect("target body")
            .length();
        assert!(
            speed < 0.05,
            "a sensor bullet imparts no knockback (pre-fix: ~2.5+ u/s), got {speed}"
        );
        assert!(
            app.world().get_entity(bullet).is_err(),
            "the round is expended on its first hit"
        );
    }

    /// Production-faithful typed damage: a bullet as the turret now spawns it -
    /// near-zero mass (so the emergent kinetic is negligible) plus an authored
    /// [`ProjectileDamage`] - hits a section and deals the number the weapon
    /// authored. Damage is ONE number now: what a round deals does not depend on
    /// which section it lands on, for either type. The drop is the nova-authored
    /// amount, NOT the old mass x velocity emergent (which the neutralized mass
    /// reduces to ~0), and it lands exactly once.
    ///
    /// The round flies at exactly [`REFERENCE_CLOSING_SPEED`] into a target at
    /// rest, so the Kinetic curve reads 1.0 and the authored number is the
    /// expected one.
    #[test]
    fn a_bullet_deals_its_authored_damage_whatever_section_it_hits() {
        use nova_gameplay::{
            projectile_hooks::ProjectileHooks,
            test_support::{settle, unfinished_integrity_physics_app_with},
        };
        fn hit_drop(class: SectionClass, damage: ProjectileDamage) -> f32 {
            let mut app = unfinished_integrity_physics_app_with(
                PhysicsPlugins::default().with_collision_hooks::<ProjectileHooks>(),
            );
            app.add_observer(resolve_bullet_hit);
            app.finish();

            let start_hp = 1000.0;
            let target = app
                .world_mut()
                .spawn((
                    Name::new("target"),
                    RigidBody::Dynamic,
                    Transform::default(),
                    Collider::cuboid(2.0, 2.0, 2.0),
                    ColliderDensity(1.0),
                    Health::new(start_hp),
                    class,
                ))
                .id();
            settle(&mut app);

            app.world_mut().spawn((
                Name::new("bullet"),
                TurretBulletProjectileMarker,
                RigidBody::Dynamic,
                Transform::from_translation(Vec3::Z * 5.0),
                Sensor,
                Collider::sphere(0.05),
                Mass(NEUTRALIZED_BULLET_MASS),
                damage,
                LinearVelocity(Vec3::NEG_Z * REFERENCE_CLOSING_SPEED),
            ));
            for _ in 0..15 {
                app.update();
            }
            start_hp
                - app
                    .world()
                    .get::<Health>(target)
                    .expect("target still exists")
                    .current
        }

        let amount = 20.0;
        // Every section class, both bullet types, one expected number. Tolerance
        // covers the ~2e-4 emergent residual from the neutralized mass. This
        // fails the moment anything reintroduces a per-section multiplier.
        for class in [
            SectionClass::Hull,
            SectionClass::Thruster,
            SectionClass::Controller,
            SectionClass::Turret,
            SectionClass::Torpedo,
        ] {
            for kind in [DamageType::Kinetic, DamageType::Pierce] {
                let dealt = hit_drop(class, ProjectileDamage::new(amount, kind));
                assert!(
                    (dealt - amount).abs() < 0.05,
                    "{kind:?} into {class:?} must deal its authored {amount}, dealt {dealt}"
                );
            }
        }
    }

    /// The two collision-event blind spots review R1.1/R1.2 caught in the
    /// sensor-bullet change: a round crossing a pure trigger volume (a
    /// beacon sphere - Sensor + events, no solidity) must SURVIVE, or the
    /// pirate goes un-hittable while patrolling near a beacon; and a round
    /// into an event-less solid (an invulnerable planetoid's collider has
    /// no Health, so collision events are never enabled on it) must still expend
    /// instead of tunneling through cover - the bullet carries its own
    /// CollisionEventsEnabled for exactly that pair.
    #[test]
    fn bullets_ignore_trigger_volumes_and_stop_at_event_less_solids() {
        use nova_gameplay::{
            projectile_hooks::ProjectileHooks,
            test_support::{settle, unfinished_integrity_physics_app_with},
        };
        let mut app = unfinished_integrity_physics_app_with(
            PhysicsPlugins::default().with_collision_hooks::<ProjectileHooks>(),
        );
        app.add_observer(resolve_bullet_hit);
        app.finish();

        // A beacon-style trigger volume in the flight path...
        app.world_mut().spawn((
            Name::new("trigger"),
            RigidBody::Static,
            Transform::from_translation(Vec3::Z * 6.0),
            Collider::sphere(2.0),
            Sensor,
            CollisionEventsEnabled,
        ));
        // ...and an invulnerable-planetoid stand-in behind it: solid,
        // no Health, no CollisionEventsEnabled of its own.
        app.world_mut().spawn((
            Name::new("event-less solid"),
            RigidBody::Static,
            Transform::default(),
            Collider::cuboid(3.0, 3.0, 1.0),
        ));
        settle(&mut app);

        let bullet = app
            .world_mut()
            .spawn((
                Name::new("bullet"),
                TurretBulletProjectileMarker,
                RigidBody::Dynamic,
                Transform::from_translation(Vec3::Z * 10.0),
                (Collider::sphere(0.05), Sensor, CollisionEventsEnabled),
                Mass(0.1),
                LinearVelocity(Vec3::NEG_Z * 100.0),
            ))
            .id();

        // Run to just past the trigger (4u of travel = 0.04s) but short of
        // the solid: the round must still be alive after crossing the
        // volume.
        for _ in 0..4 {
            app.update();
        }
        assert!(
            app.world().get_entity(bullet).is_ok(),
            "a round crossing a trigger volume must fly on (review R1.1)"
        );

        // Run into the solid: the round expends even though the solid has
        // no events of its own.
        for _ in 0..12 {
            app.update();
        }
        assert!(
            app.world().get_entity(bullet).is_err(),
            "a round must stop at an event-less solid instead of tunneling \
             (review R1.2)"
        );
    }

    /// The pierce harness flies at the anchor, where both speed curves read 1.0,
    /// so every budget assertion is the pure pierce arithmetic. Tests that are
    /// ABOUT speed pass their own.
    const PIERCE_TEST_SPEED: f32 = REFERENCE_CLOSING_SPEED;

    /// Plate thickness and spacing along the line of flight. A round at the
    /// anchor speed covers 1.67 u per 1/60 s step, so a 4-unit plate is sampled
    /// twice over and a 6-unit pitch leaves a real gap between plates: nothing
    /// under test can tunnel, and nothing lands two plates in one step.
    const PIERCE_PLATE_THICKNESS: f32 = 4.0;
    const PIERCE_PLATE_PITCH: f32 = 6.0;

    /// The pierce harness: a real avian world plus the hit observer, wired as
    /// the neighbouring typed-damage tests wire it.
    fn pierce_app() -> App {
        use nova_gameplay::{
            projectile_hooks::ProjectileHooks, test_support::unfinished_integrity_physics_app_with,
        };
        let mut app = unfinished_integrity_physics_app_with(
            PhysicsPlugins::default().with_collision_hooks::<ProjectileHooks>(),
        );
        app.add_observer(resolve_bullet_hit);
        app.finish();
        app
    }

    /// A free-floating slab of `hp` centred at `z`, [`PIERCE_PLATE_THICKNESS`]
    /// deep along the round's line of flight.
    ///
    /// Deliberately NO `ConnectedTo`: a slab that reaches zero health is
    /// disabled rather than destroyed, which keeps the render-facing explode
    /// observers (they cannot run headless) out of these tests. The pierce rule
    /// keys on the health pool, not on the destroy marker, so what is under test
    /// is unaffected.
    fn spawn_plate(app: &mut App, z: f32, hp: f32) -> Entity {
        app.world_mut()
            .spawn((
                Name::new("plate"),
                RigidBody::Dynamic,
                Transform::from_translation(Vec3::Z * z),
                Collider::cuboid(8.0, 8.0, PIERCE_PLATE_THICKNESS),
                ColliderDensity(1.0),
                Health::new(hp),
            ))
            .id()
    }

    /// A production-shaped round flying -Z from `z` at `speed`: neutralized
    /// mass, sensor collider carrying its own collision events, and `amount` of
    /// authored budget. The plates carry no `SectionClass`, so resistance
    /// is 1.0 everywhere and the arithmetic reads directly.
    fn spawn_round_at(app: &mut App, z: f32, amount: f32, kind: DamageType, speed: f32) -> Entity {
        app.world_mut()
            .spawn((
                Name::new("bullet"),
                TurretBulletProjectileMarker,
                RigidBody::Dynamic,
                Transform::from_translation(Vec3::Z * z),
                (Collider::sphere(0.05), Sensor, CollisionEventsEnabled),
                Mass(NEUTRALIZED_BULLET_MASS),
                ProjectileDamage::new(amount, kind),
                LinearVelocity(Vec3::NEG_Z * speed),
            ))
            .id()
    }

    /// A Kinetic round at the anchor closing speed: the pre-speed round.
    fn spawn_round(app: &mut App, z: f32, amount: f32) -> Entity {
        spawn_round_at(app, z, amount, DamageType::Kinetic, PIERCE_TEST_SPEED)
    }

    fn plate_health(app: &App, plate: Entity) -> f32 {
        app.world()
            .get::<Health>(plate)
            .expect("plate still exists")
            .current
    }

    /// A Kinetic round that only dents its target is expended on it whatever
    /// budget arithmetic says - the pre-pierce behaviour, and the reason armour
    /// that HOLDS is still a wall. The Pierce contrast is
    /// `a_fast_pierce_round_crosses_a_living_section_and_hits_what_is_behind`.
    #[test]
    fn a_kinetic_round_stops_on_a_target_it_fails_to_destroy() {
        use nova_gameplay::test_support::settle;
        let mut app = pierce_app();
        let plate = spawn_plate(&mut app, 0.0, 100.0);
        settle(&mut app);
        let round = spawn_round(&mut app, 8.0, 20.0);

        for _ in 0..20 {
            app.update();
        }

        assert!(
            (plate_health(&app, plate) - 80.0).abs() < 0.05,
            "the whole 20-damage budget lands on the plate, got {}",
            plate_health(&app, plate)
        );
        assert!(
            app.world().get_entity(round).is_err(),
            "a round that fails to destroy its target must be expended on it"
        );
    }

    /// One contact is charged ONCE. Avian raises a `CollisionStart` per
    /// event-enabled collider, and a production round (its own
    /// `CollisionEventsEnabled`) hitting a health-bearing collider (events
    /// enabled by the integrity hook) is event-enabled on both sides - so the
    /// hit arrives twice with the orderings swapped and the round used to pay
    /// its authored damage out twice (20 authored, 40 dealt). The budget makes
    /// that fatal rather than merely wrong: a mirrored contact would also debit
    /// the budget twice.
    #[test]
    fn a_round_deals_its_authored_damage_once_per_contact() {
        use nova_gameplay::test_support::settle;
        let mut app = pierce_app();
        let plate = spawn_plate(&mut app, 0.0, 1000.0);
        settle(&mut app);
        spawn_round(&mut app, 8.0, 20.0);

        for _ in 0..20 {
            app.update();
        }

        let dealt = 1000.0 - plate_health(&app, plate);
        assert!(
            (dealt - 20.0).abs() < 0.05,
            "a round authored at 20 must deal 20, not the mirrored 40: dealt {dealt}"
        );
    }

    /// The point of the pierce rule: thin destructible cover costs a round part
    /// of its budget instead of stopping it, so the round reaches what the cover
    /// was protecting.
    #[test]
    fn a_round_that_destroys_a_thin_plate_damages_the_hull_behind_it() {
        use nova_gameplay::test_support::settle;
        let mut app = pierce_app();
        let plate = spawn_plate(&mut app, PIERCE_PLATE_PITCH, 20.0);
        let hull = spawn_plate(&mut app, 0.0, 500.0);
        settle(&mut app);
        let round = spawn_round(&mut app, 2.0 * PIERCE_PLATE_PITCH, 100.0);

        for _ in 0..40 {
            app.update();
        }

        assert_eq!(plate_health(&app, plate), 0.0, "the plate is destroyed");
        assert!(
            (plate_health(&app, hull) - 420.0).abs() < 0.05,
            "the surviving 80 of the budget must land on the hull behind, got a \
             drop of {}",
            500.0 - plate_health(&app, hull)
        );
        assert!(
            app.world().get_entity(round).is_err(),
            "the round is expended on the hull it could not destroy"
        );
    }

    /// The KINETIC budget is a hard cap: a slug crossing a stack of destructible
    /// plates deals its authored damage in total and no more, then stops - it
    /// does not re-deal its full amount to every plate on the line. That
    /// conservation belongs to Kinetic alone now; the Pierce contrast is
    /// `a_pierce_round_deals_more_in_total_than_it_was_fired_with`.
    #[test]
    fn a_kinetic_round_never_deals_more_than_the_budget_it_carries() {
        use nova_gameplay::test_support::settle;
        let mut app = pierce_app();
        let budget = 20.0;
        let plate_hp = 5.0;
        // Six 5 hp plates in a row: the 20-point budget is worth exactly four of
        // them.
        let plates: Vec<Entity> = (0..6)
            .map(|index| spawn_plate(&mut app, index as f32 * PIERCE_PLATE_PITCH, plate_hp))
            .collect();
        settle(&mut app);
        let round = spawn_round(&mut app, 6.0 * PIERCE_PLATE_PITCH, budget);

        for _ in 0..60 {
            app.update();
        }

        let total_dealt: f32 = plates
            .iter()
            .map(|&plate| plate_hp - plate_health(&app, plate))
            .sum();
        assert!(
            (total_dealt - budget).abs() < 0.05,
            "a round may deal at most the {budget} it carries, dealt {total_dealt}"
        );
        // Direction guard: the damage really was spent nearest-first, on the
        // four plates the round reached, and the last two never saw it.
        for &plate in &plates[2..] {
            assert_eq!(
                plate_health(&app, plate),
                0.0,
                "the four plates nearest the muzzle are destroyed"
            );
        }
        for &plate in &plates[..2] {
            assert_eq!(
                plate_health(&app, plate),
                plate_hp,
                "a spent round must not reach the plates behind it"
            );
        }
        assert!(
            app.world().get_entity(round).is_err(),
            "the round dies when its budget runs out"
        );
    }

    /// Kinetic's identity under real physics: closing speed is DAMAGE. The same
    /// authored 20-point round hits for 30 on a charge, 20 at the anchor and 10
    /// in a stern chase, and the stern chase is driven by the TARGET's velocity
    /// - the half of the relative-velocity term a muzzle-speed-only reading
    /// would miss.
    #[test]
    fn a_kinetic_round_closing_faster_deals_more_damage_per_hit() {
        use nova_gameplay::test_support::settle;
        /// One hit on a plate far too tough to destroy, so the whole drop is the
        /// round's bite. `plate_speed` runs the plate away down the same line.
        fn hit_drop(round_speed: f32, plate_speed: f32) -> f32 {
            let mut app = pierce_app();
            let start_hp = 10_000.0;
            let plate = spawn_plate(&mut app, 0.0, start_hp);
            settle(&mut app);
            app.world_mut()
                .entity_mut(plate)
                .insert(LinearVelocity(Vec3::NEG_Z * plate_speed));
            spawn_round_at(&mut app, 40.0, 20.0, DamageType::Kinetic, round_speed);
            for _ in 0..80 {
                app.update();
            }
            start_hp - plate_health(&app, plate)
        }

        let anchored = hit_drop(REFERENCE_CLOSING_SPEED, 0.0);
        let charging = hit_drop(1.5 * REFERENCE_CLOSING_SPEED, 0.0);
        let fleeing = hit_drop(REFERENCE_CLOSING_SPEED, 0.5 * REFERENCE_CLOSING_SPEED);

        assert!(
            (anchored - 20.0).abs() < 0.05,
            "at the reference closing speed the round deals exactly its authored \
             20, got {anchored}"
        );
        assert!(
            (charging - 30.0).abs() < 0.05,
            "closing 1.5x faster must deal 1.5x, got {charging}"
        );
        assert!(
            (fleeing - 10.0).abs() < 0.05,
            "a target running away at half the round's speed halves the hit, got \
             {fleeing}"
        );
    }

    /// The rake, end to end: a Pierce round is not stopped by a section being
    /// ALIVE. It pays that section's thickness out of its power and carries on
    /// into what the section was shielding, dealing its full bite there too.
    /// The A/B is a Kinetic round on the same rig, which stops dead on the front
    /// plate - the difference is the TYPE, not the speed.
    #[test]
    fn a_pierce_round_rakes_through_a_living_section_and_hits_what_is_behind() {
        use nova_gameplay::test_support::settle;
        /// `(front drop, back drop)` for a round of `kind` into two 100 hp
        /// plates, neither of which a 20-point bite can destroy.
        fn run(kind: DamageType) -> (f32, f32) {
            let mut app = pierce_app();
            let armour = 100.0;
            let front = spawn_plate(&mut app, 0.0, armour);
            let back = spawn_plate(&mut app, -2.0 * PIERCE_PLATE_PITCH, armour);
            settle(&mut app);
            spawn_round_at(
                &mut app,
                2.0 * PIERCE_PLATE_PITCH,
                20.0,
                kind,
                PIERCE_TEST_SPEED,
            );
            for _ in 0..40 {
                app.update();
            }
            (
                armour - plate_health(&app, front),
                armour - plate_health(&app, back),
            )
        }

        let (front, back) = run(DamageType::Pierce);
        assert!(
            (front - 20.0).abs() < 0.05,
            "the front section takes the round's authored bite, got {front}"
        );
        assert!(
            (back - 20.0).abs() < 0.05,
            "and so does what was behind it, UNDIMINISHED - the rake does not \
             decay with depth, got {back}"
        );

        let (front, back) = run(DamageType::Kinetic);
        assert!(
            (front - 20.0).abs() < 0.05,
            "the slug bites the same 20 on arrival, got {front}"
        );
        assert_eq!(
            back, 0.0,
            "but it is spent there: a slug travels only through what it destroys"
        );
    }

    /// The invariant the rake deliberately breaks: a Pierce round's TOTAL damage
    /// exceeds what it was fired with, because it pays for travel out of power
    /// and its damage never depletes. Six 30 hp plates take 20 each from a round
    /// authored at 20 - 120 in total - and `MAX_PIERCE_LAYERS` is what ends it.
    #[test]
    fn a_pierce_round_deals_more_in_total_than_it_was_fired_with() {
        use nova_gameplay::test_support::settle;
        let mut app = pierce_app();
        let amount = 20.0;
        // 30 hp each: every plate SURVIVES its 20-point bite, so nothing here is
        // the old kill-to-continue rule wearing a different hat.
        let plate_hp = 30.0;
        let count = 8;
        let plates: Vec<Entity> = (0..count)
            .map(|index| spawn_plate(&mut app, index as f32 * PIERCE_PLATE_PITCH, plate_hp))
            .collect();
        settle(&mut app);
        spawn_round_at(
            &mut app,
            count as f32 * PIERCE_PLATE_PITCH,
            amount,
            DamageType::Pierce,
            PIERCE_TEST_SPEED,
        );

        for _ in 0..80 {
            app.update();
        }

        let raked = plates
            .iter()
            .filter(|&&plate| plate_health(&app, plate) < plate_hp)
            .count();
        let dealt: f32 = plates
            .iter()
            .map(|&plate| plate_hp - plate_health(&app, plate))
            .sum();
        assert_eq!(
            raked, MAX_PIERCE_LAYERS as usize,
            "the layer cap is what ends a rake through cheap plates"
        );
        assert!(
            dealt > amount,
            "a rake's total must EXCEED the amount it was fired with, got {dealt}"
        );
        assert!(
            (dealt - amount * MAX_PIERCE_LAYERS as f32).abs() < 0.05,
            "six layers x the authored 20, undiminished by depth, got {dealt}"
        );
    }

    /// Speed is POWER for a penetrator: the same round closing at 2x pays half
    /// as much per layer and rakes deeper, while its per-hit damage does not
    /// move at all. 100 hp plates cost 100 power each at the anchor (three of a
    /// 300 power budget) and 50 each at 2x (the layer cap binds first).
    #[test]
    fn a_fast_pierce_round_rakes_deeper_without_biting_harder() {
        use nova_gameplay::test_support::settle;
        /// `(layers raked, damage on the first layer)` for a Pierce round at
        /// `speed` down a stack of 100 hp plates.
        fn run(speed: f32) -> (usize, f32) {
            let mut app = pierce_app();
            let plate_hp = 100.0;
            let count = 8;
            let plates: Vec<Entity> = (0..count)
                .map(|index| spawn_plate(&mut app, index as f32 * PIERCE_PLATE_PITCH, plate_hp))
                .collect();
            settle(&mut app);
            spawn_round_at(
                &mut app,
                count as f32 * PIERCE_PLATE_PITCH,
                20.0,
                DamageType::Pierce,
                speed,
            );
            for _ in 0..120 {
                app.update();
            }
            let raked = plates
                .iter()
                .filter(|&&plate| plate_health(&app, plate) < plate_hp)
                .count();
            // The round enters from +Z, so the LAST plate is the first one hit.
            let first_hit = plate_hp - plate_health(&app, plates[count - 1]);
            (raked, first_hit)
        }

        let (anchored, anchored_bite) = run(PIERCE_TEST_SPEED);
        let (fast, fast_bite) = run(2.0 * PIERCE_TEST_SPEED);
        assert_eq!(
            anchored, 3,
            "300 power buys three 100 hp layers at the anchor"
        );
        assert!(
            fast > anchored,
            "closing at 2x must rake deeper: {fast} vs {anchored}"
        );
        assert!(
            (fast_bite - anchored_bite).abs() < 0.05,
            "and must NOT bite harder for it: {fast_bite} vs {anchored_bite}"
        );
    }

    /// muzzle_speed * fire_interval along the exit direction, regardless of
    /// ship velocity. The 24 rounds/s rate beats against the 64 Hz tick so
    /// shots sample every phase of the tick window.
    #[test]
    fn bullet_stream_stays_linear_at_high_ship_velocity() {
        use nova_gameplay::{
            projectile_hooks::ProjectileHooks,
            test_support::{settle, unfinished_integrity_physics_app_with},
        };
        let mut app = unfinished_integrity_physics_app_with(
            PhysicsPlugins::default().with_collision_hooks::<ProjectileHooks>(),
        );
        app.add_systems(FixedUpdate, shoot_spawn_projectile);
        app.finish();

        let (ship, turret) = spawn_stream_rig(&mut app, 24.0);
        settle(&mut app);
        app.world_mut()
            .entity_mut(ship)
            .insert(LinearVelocity(Vec3::X * 150.0));
        arm_turret(&mut app, turret);

        for _ in 0..40 {
            app.update();
        }

        let mut positions: Vec<Vec3> = app
            .world_mut()
            .query_filtered::<&Position, With<TurretBulletProjectileMarker>>()
            .iter(app.world())
            .map(|p| p.0)
            .collect();
        assert!(
            positions.len() >= 10,
            "expected a stream, got {} bullets",
            positions.len()
        );

        // Sort along the exit direction (the muzzle's yaw-slewed -Z), then
        // every consecutive delta must be the SAME vector: equal spacing and
        // collinearity in one check.
        let exit_direction = Quat::from_rotation_y(0.3) * Vec3::NEG_Z;
        positions.sort_by(|a, b| a.dot(exit_direction).total_cmp(&b.dot(exit_direction)));
        let expected_spacing = 200.0 / 24.0;
        let first_delta = positions[1] - positions[0];
        // Delivery guard: uniform spacing alone is also satisfied by every
        // bullet sitting on one point; the spacing must be the real
        // muzzle_speed * interval stride.
        assert!(
            (first_delta.length() - expected_spacing).abs() < 0.1,
            "stream stride should be ~{expected_spacing}, got {}",
            first_delta.length()
        );
        for window in positions.windows(2) {
            let delta = window[1] - window[0];
            assert!(
                (delta - first_delta).length() < 0.05,
                "stream must stay uniform and collinear at speed: delta {delta} vs {first_delta}"
            );
        }
    }

    /// A bullet's FIRST rendered frame must sit on the world's render clock.
    /// The spawn writes the RAW physics pose
    /// (tick-start muzzle minus the sub-tick lead), and a body spawned
    /// mid-tick misses FixedFirst, so its easing `start` is None and the
    /// first frame used to render that raw pose while the ship rendered
    /// EASED - one frame of muzzle pop, cross-stream error up to a full
    /// tick of ship motion (~2.3 u at 150 u/s). With the easing seed the
    /// first render is exactly `lerp(muzzle_tick_start, raw_end, alpha)`:
    /// zero cross-stream offset from the rendered barrel, and along-stream
    /// only ever FORWARD by at most one tick of muzzle-exit travel (a
    /// mid-tick shot has already flown; it must never render BEHIND the
    /// barrel, inside the turret). The raw physics stream is pinned
    /// separately by `bullet_stream_stays_linear_at_high_ship_velocity`;
    /// this test asserts the render clock, checking every bullet of a
    /// 24 rounds/s stream so the 64 Hz-vs-60 fps beat sweeps the easing
    /// alpha across its range.
    #[test]
    fn first_rendered_frame_attaches_the_bullet_to_the_eased_muzzle() {
        use std::collections::HashSet;

        use nova_gameplay::{
            projectile_hooks::ProjectileHooks,
            test_support::{settle, unfinished_integrity_physics_app_with},
        };
        let mut app = unfinished_integrity_physics_app_with(
            PhysicsPlugins::default().with_collision_hooks::<ProjectileHooks>(),
        );
        app.add_systems(FixedUpdate, shoot_spawn_projectile);
        app.finish();

        let (ship, turret) = spawn_stream_rig(&mut app, 24.0);
        settle(&mut app);
        app.world_mut()
            .entity_mut(ship)
            .insert(LinearVelocity(Vec3::X * 150.0));
        arm_turret(&mut app, turret);

        // Rig locals (spawn_stream_rig): turret at (0, 1, 0), muzzle at
        // (0, 0, -0.5) yawed 0.3. The ship never spins here, so the exit
        // direction is constant in world space.
        let muzzle_local_rot = Quat::from_rotation_y(0.3);
        // Chain composition (local_pose_in_root): the muzzle's own rotation
        // aims its frame (the exit direction), it does not displace its
        // mount point.
        let muzzle_local_pos = Vec3::new(0.0, 1.0, 0.0) + Vec3::new(0.0, 0.0, -0.5);
        let exit_direction = muzzle_local_rot * Vec3::NEG_Z;
        // One tick of muzzle-exit travel: the most a mid-tick shot may lead
        // the barrel by on its first rendered frame.
        let max_lead = 200.0 * 1.0 / 64.0 + 0.05;

        let mut seen: HashSet<Entity> = HashSet::new();
        let mut sampled = 0usize;
        let mut max_cross = 0.0f32;
        let mut min_alpha = f32::MAX;
        for _ in 0..40 {
            app.update();
            // The ship's Transform is its EASED render pose this frame
            // (TransformInterpolation); compose the rendered muzzle from it.
            let ship_tf = *app.world().get::<Transform>(ship).unwrap();
            let rendered_muzzle = ship_tf.translation + ship_tf.rotation * muzzle_local_pos;
            let alpha = app.world().resource::<Time<Fixed>>().overstep_fraction();

            let bullets: Vec<(Entity, Vec3)> = app
                .world_mut()
                .query_filtered::<(Entity, &Transform), With<TurretBulletProjectileMarker>>()
                .iter(app.world())
                .map(|(e, t)| (e, t.translation))
                .collect();
            for (bullet, rendered) in bullets {
                if !seen.insert(bullet) {
                    continue;
                }
                // This bullet's FIRST rendered frame.
                sampled += 1;
                min_alpha = min_alpha.min(alpha);
                let offset = rendered - rendered_muzzle;
                let along = offset.dot(exit_direction);
                let cross = (offset - along * exit_direction).length();
                max_cross = max_cross.max(cross);
                assert!(
                    along > -0.05,
                    "a bullet must never first-render BEHIND the barrel: along {along}"
                );
                assert!(
                    along < max_lead,
                    "a bullet's first render may lead the barrel by at most one \
                     tick of muzzle travel: along {along} vs {max_lead}"
                );
            }
        }

        // Delivery guards: a real stream was sampled, and the beat actually
        // exercised frames where raw and eased poses diverge (small alpha is
        // where the pre-fix pop is largest).
        assert!(
            sampled >= 10,
            "expected a stream, sampled {sampled} first frames"
        );
        assert!(
            min_alpha < 0.5,
            "the beat must sample misaligned frames for the assertion to bite \
             (min alpha {min_alpha})"
        );
        assert!(
            max_cross < 0.02,
            "first rendered frame must sit ON the rendered stream line: \
             max cross-stream offset {max_cross}"
        );
    }

    /// The shipped default fire rate (100 rounds/s) is faster than the 64 Hz
    /// physics tick: the multi-shot loop must deliver the TRUE cadence via
    /// several spawns per tick. The old render-schedule path silently capped
    /// fire rates at one bullet per frame.
    #[test]
    fn fire_rate_above_the_tick_rate_keeps_its_true_cadence() {
        use nova_gameplay::{
            projectile_hooks::ProjectileHooks,
            test_support::{settle, unfinished_integrity_physics_app_with},
        };
        let mut app = unfinished_integrity_physics_app_with(
            PhysicsPlugins::default().with_collision_hooks::<ProjectileHooks>(),
        );
        app.add_systems(FixedUpdate, shoot_spawn_projectile);
        app.finish();

        let (_ship, turret) = spawn_stream_rig(&mut app, 100.0);
        settle(&mut app);
        arm_turret(&mut app, turret);

        // 60 render frames = 1.0 s of manual time.
        for _ in 0..60 {
            app.update();
        }

        let count = app
            .world_mut()
            .query_filtered::<(), With<TurretBulletProjectileMarker>>()
            .iter(app.world())
            .count();
        assert!(
            (95..=105).contains(&count),
            "one second at 100 rounds/s must yield ~100 bullets, got {count}"
        );
    }
}
