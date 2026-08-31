//! Turret firing: the fixed-clock muzzle loop that spawns bullets and the
//! bullet's own contact rule - deal the round's bite, then let its type decide
//! whether it travels on.

use std::time::Duration;

use avian3d::prelude::*;
use bevy::prelude::*;
use bevy_rand::prelude::{GlobalRng, WyRand};
use bevy_transform_interpolation::{RotationEasingState, TranslationEasingState};
use nova_gameplay::lifetime::TempEntity;
use rand::RngExt;

use super::*;
use crate::{physics::prelude::rigid_body_point_velocity, sections::local_pose_in_root};

/// A runaway-config backstop for the multi-shot loop: at 64 Hz ticks this
/// caps the effective fire rate at 512 rounds/s per barrel, far above any
/// authored turret; without it a zero-ish fire interval would spawn
/// unboundedly inside one tick.
const MAX_SHOTS_PER_TICK: u32 = 8;

/// Half-angle of the cone a round leaves the muzzle in: 0.1 degrees.
///
/// A LOOK number, and it cannot be read as a balance lever in either
/// direction. The fire gate already lets a barrel shoot from anywhere inside
/// [`TURRET_ON_TARGET_RAD`] - 0.92 deg, 2.4 u of lateral miss at the 150 u
/// point-defense envelope - so 0.26 u of extra scatter at that range makes
/// each round marginally WORSE than the aim it was fired on, never better.
/// Spread cannot raise a hit rate; it is here so a stream reads as a gun
/// throwing rounds instead of one laser-straight line of them.
///
/// A FEEL number: tuned by eye, wide enough to see the stream fray at gunfight
/// range and far too narrow to argue with the gate above it.
const MUZZLE_SPREAD_RAD: f32 = 0.1 * std::f32::consts::PI / 180.0;

/// A tilt of up to [`MUZZLE_SPREAD_RAD`] about a random axis square to the
/// barrel, drawn off the SEEDED stream so a replayed run scatters identically.
///
/// The barrel points -Z in its own frame, so any axis in the local XY plane
/// swings a round off it; `sqrt` on the tilt keeps the draw uniform over the
/// cone's disc instead of piling rounds onto the axis.
fn muzzle_spread(rng: &mut impl RngExt) -> Quat {
    let azimuth: f32 = rng.random_range(0.0..std::f32::consts::TAU);
    let tilt = MUZZLE_SPREAD_RAD * rng.random_range(0.0..1.0f32).sqrt();
    Quat::from_axis_angle(Vec3::new(azimuth.cos(), azimuth.sin(), 0.0), tilt)
}

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
            Option<&TurretStow>,
        ),
        (With<TurretSectionMarker>, Without<SectionInactiveMarker>),
    >,
    mut q_muzzle: Query<&mut TurretSectionBarrelFireState, With<TurretSectionBarrelMuzzleMarker>>,
    q_chain: Query<(&Transform, &ChildOf)>,
    q_hot: Query<&WeaponsHot>,
    q_defense: Query<(&PointDefenseMount, &TurretDefenseTarget)>,
    // OPTIONAL on purpose. Spread is cosmetic, so it must never be able to
    // gate the fire path: a rig with no `EntropyPlugin` (a bare unit-test app)
    // fires perfectly straight rounds rather than silently firing none, which
    // is what a plain `Single` would do.
    mut rng: Option<Single<&mut WyRand, With<GlobalRng>>>,
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
        stow,
    ) in &mut q_turret
    {
        // BEFORE the safety and its point-defence exemption: a mount that is
        // not fully deployed cannot fire at all - a gun inside its housing
        // has no line of fire, and the deploy travel is the design's cost.
        // The Flight Computer deploys a mount by assigning it (the stow
        // machine reads `TurretDefenseTarget`); it never shoots through it.
        if stow.is_some_and(|stow| !stow.is_deployed()) {
            continue;
        }
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

                // Drawn PER ROUND, not per muzzle: a barrel firing several
                // times inside one tick must not send that whole burst down the
                // same perturbed line, which is exactly the laser this exists
                // to break up. The NOMINAL bearing is what the fire gate above
                // judged - spread is applied after it, so scatter can never
                // talk a barrel into shooting.
                let exit_rotation =
                    projectile_rotation * rng.as_deref_mut().map_or(Quat::IDENTITY, muzzle_spread);
                let muzzle_exit_velocity = (exit_rotation * Vec3::NEG_Z) * config.muzzle_speed;
                let linear_velocity = muzzle_exit_velocity + inertia_vel;

                // Sub-tick exactness: a shot due `lead` seconds into this tick
                // starts one lead-time of muzzle-exit travel BEHIND the muzzle,
                // so after this tick's integration it sits exactly where a
                // bullet fired at the due moment would - the stream stays
                // uniformly spaced at any ship velocity. (The ship-motion terms
                // cancel: spawn = muzzle + (v_muzzle - v_bullet) * lead, and
                // v_bullet - v_muzzle is the muzzle exit velocity.)
                //
                // So the offset is backed off along the round's OWN exit
                // velocity, spread included, and not along the nominal bearing:
                // the identity it rests on is about the velocity this round
                // actually carries. Back it off along the nominal line instead
                // and the round no longer starts on its own ray through the
                // muzzle - back-projecting it misses the barrel sideways, which
                // is the one place the scatter would be visible as a defect
                // rather than as a gun.
                let lead = dt - excess;
                let projectile_transform = Transform {
                    translation: muzzle_position - muzzle_exit_velocity * lead,
                    // The round points where it FLIES, which also keeps the
                    // rotation easing seeded below a no-op: start equals end.
                    rotation: exit_rotation,
                    ..default()
                };

                let mut projectile = commands.spawn((
                    Name::new("Turret Projectile"),
                    TurretBulletProjectileMarker,
                    ProjectileOwner(*spaceship),
                    projectile_transform,
                    // NOT a rigid body, and not a collider either. A round is
                    // swept by `nova_gameplay::rounds`: it has no contact
                    // response to configure, no mass to neutralize, no
                    // speculative margin to generate phantom pairs against
                    // every other round in the air, and it cannot tunnel. The
                    // whole bundle a body needed - RigidBody, Collider, Sensor,
                    // CollisionEventsEnabled, ActiveCollisionHooks, Mass,
                    // AngularInertia - existed to make avian carry something it
                    // was never allowed to influence.
                    RoundVelocity(linear_velocity),
                    // The fired round comes from the turret's loaded-ammo slot,
                    // not a hardcoded type, so a future ammo switch changes
                    // what this stamps. The closing-speed scaling is applied
                    // at the HIT, not here - the target is not known yet.
                    ProjectileDamage::new(bullet_damage, bullet_kind),
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
                            start: Some(exit_rotation),
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
        // The production order, all on the fixed clock: solve, ease, write the
        // joint pose, then fire along it.
        app.add_systems(
            FixedUpdate,
            (
                (update_turret_aim_point, update_turret_target_joints_system)
                    .chain()
                    .before(SmoothLookRotationSystems::Sync),
                sync_turret_joint_rotation.after(SmoothLookRotationSystems::Sync),
                shoot_spawn_projectile.after(sync_turret_joint_rotation),
            ),
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
    fn a_turret_not_fully_deployed_holds_its_fire() {
        // The stow gate sits BEFORE the safety and its point-defence
        // exemption: any phase short of Deployed holds fire, on an otherwise
        // free-firing unmanaged rig - a gun inside its housing has no line
        // of fire.
        for phase in [
            TurretStowPhase::Stowed,
            TurretStowPhase::Stowing,
            TurretStowPhase::Deploying,
        ] {
            let mut app = firing_app(1.0);
            let turret = spawn_firing_turret(&mut app, None);
            app.world_mut()
                .entity_mut(turret)
                .insert(TurretStow::new(phase));

            for _ in 0..3 {
                app.update();
            }
            assert_eq!(
                bullet_count(&mut app),
                0,
                "{phase:?} must hold fire on a rig that otherwise fires freely"
            );

            // The same rig with the machine at Deployed proves the gate, not
            // some other silence, held the trigger.
            app.world_mut()
                .entity_mut(turret)
                .insert(TurretStow::new(TurretStowPhase::Deployed));
            for _ in 0..3 {
                app.update();
            }
            assert!(bullet_count(&mut app) > 0, "deployed, the rig fires again");
        }
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
            name: None,
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
            name: None,
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

    /// Bullets from a fast ship must keep an EXACT stride down the exit axis.
    /// The old Update-schedule spawn sampled the EASED muzzle pose at
    /// render-frame shot times with a static 0.01 s nudge, so each shot picked
    /// up a different fraction of a tick of ship motion - at 150 u/s the
    /// stream scattered by whole units ("bullets spew out"). On the raw clock
    /// with sub-tick lead compensation the along-axis spacing is exact: every
    /// consecutive delta projects onto the exit direction as
    /// muzzle_speed * fire_interval, regardless of ship velocity. The 24
    /// rounds/s rate beats against the 64 Hz tick so shots sample every phase
    /// of the tick window - drop the lead compensation and the stride
    /// quantizes to whole ticks (6.25 u and 9.375 u against the true 8.33 u),
    /// which is what this pins.
    ///
    /// Measured ALONG that axis rather than as a collinearity, because
    /// `MUZZLE_SPREAD_RAD` is a LATERAL perturbation and consecutive rounds
    /// are deliberately no longer on one line. The rig runs a seeded
    /// `EntropyPlugin` so the spread is live here instead of quietly switched
    /// off, and the cross-stream half of the old claim survives as the CONE:
    /// a round that has flown `d` down the axis may sit at most
    /// `d * tan(spread)` off it - a fifth of a unit across this whole stream,
    /// where the scatter this test was written against ran to whole units.
    #[test]
    fn bullet_stream_keeps_its_exact_stride_at_high_ship_velocity() {
        use bevy_rand::prelude::EntropyPlugin;
        use nova_gameplay::{
            rounds::NovaRoundPlugin,
            test_support::{settle, unfinished_integrity_physics_app_with},
        };
        // Physics for the SHIP, `NovaRoundPlugin` for the rounds it fires: a
        // round is not a body, so avian moves the rig and the sweep moves the
        // stream. No collision hooks - a round never reaches the pair filter.
        let mut app = unfinished_integrity_physics_app_with(PhysicsPlugins::default());
        app.add_plugins(NovaRoundPlugin);
        // Seeded, so the scatter this asserts a bound on is the same scatter
        // every run.
        app.add_plugins(EntropyPlugin::<WyRand>::with_seed(
            20260821u64.to_ne_bytes(),
        ));
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

        // The RAW stream, which is the whole point of this test: a round's
        // `Transform` has been eased by the time `update` returns, and the
        // easing `end` is exactly the pose the sweep integrated at the close of
        // the last fixed tick. (It used to read avian's `Position`, which a
        // round no longer has.)
        let mut positions: Vec<Vec3> = app
            .world_mut()
            .query_filtered::<&TranslationEasingState, With<TurretBulletProjectileMarker>>()
            .iter(app.world())
            .filter_map(|easing| easing.end)
            .collect();
        assert!(
            positions.len() >= 10,
            "expected a stream, got {} bullets",
            positions.len()
        );

        // Sort along the exit direction (the muzzle's yaw-slewed -Z), then
        // every consecutive delta must project onto it as the real
        // muzzle_speed * interval stride - uniform spacing and true delivery
        // in one check, since a stack of bullets on one point reads a stride
        // of zero.
        let exit_direction = Quat::from_rotation_y(0.3) * Vec3::NEG_Z;
        positions.sort_by(|a, b| a.dot(exit_direction).total_cmp(&b.dot(exit_direction)));
        let expected_spacing = 200.0 / 24.0;
        for window in positions.windows(2) {
            let stride = (window[1] - window[0]).dot(exit_direction);
            assert!(
                (stride - expected_spacing).abs() < 0.05,
                "stride along the exit axis must stay exact at speed: \
                 {stride} vs {expected_spacing}"
            );
        }

        // Cross-stream, the other half: every round is still inside the cone
        // the muzzle threw it in. The apex is the muzzle, which this test never
        // samples - so measure from the NEWEST round instead (the smallest
        // along-projection, fired less than one fire interval ago). It sits
        // within a stride of the apex and carries a lateral offset of its own,
        // both of which fold into the slack term.
        let newest = positions[0];
        let mut max_cross = 0.0f32;
        for &position in &positions {
            let offset = position - newest;
            let along = offset.dot(exit_direction);
            let cross = (offset - along * exit_direction).length();
            let bound = (along + 4.0 * expected_spacing) * MUZZLE_SPREAD_RAD.tan();
            assert!(
                cross <= bound + 1e-3,
                "a round {along} u down the stream may sit at most {bound} u off \
                 the exit axis, got {cross}"
            );
            max_cross = max_cross.max(cross);
        }
        // Delivery guard: the spread is actually ON. With no entropy source the
        // rounds fall on one line and the cone bound above passes vacuously.
        assert!(
            max_cross > 0.01,
            "the rig must fire with live spread, got max cross-stream {max_cross}"
        );
    }

    /// A bullet's FIRST rendered frame must sit on the world's render clock.
    /// The spawn writes the RAW physics pose
    /// (tick-start muzzle minus the sub-tick lead), and a round spawned
    /// mid-tick misses FixedFirst, so its easing `start` is None and the
    /// first frame used to render that raw pose while the ship rendered
    /// EASED - one frame of muzzle pop, cross-stream error up to a full
    /// tick of ship motion (~2.3 u at 150 u/s). With the easing seed the
    /// first render is exactly `lerp(muzzle_tick_start, raw_end, alpha)`:
    /// zero cross-stream offset from the rendered barrel, and along-stream
    /// only ever FORWARD by at most one tick of muzzle-exit travel (a
    /// mid-tick shot has already flown; it must never render BEHIND the
    /// barrel, inside the turret). The raw stream is pinned
    /// separately by `bullet_stream_keeps_its_exact_stride_at_high_ship_velocity`;
    /// this test asserts the render clock, checking every bullet of a
    /// 24 rounds/s stream so the 64 Hz-vs-60 fps beat sweeps the easing
    /// alpha across its range.
    ///
    /// No `EntropyPlugin` on this rig, so `MUZZLE_SPREAD_RAD` is off and the
    /// cross-stream bound below measures the render clock alone. That is the
    /// question here; the spread's own bound is the stride test's.
    #[test]
    fn first_rendered_frame_attaches_the_bullet_to_the_eased_muzzle() {
        use std::collections::HashSet;

        use nova_gameplay::{
            rounds::NovaRoundPlugin,
            test_support::{settle, unfinished_integrity_physics_app_with},
        };
        // Physics for the SHIP, `NovaRoundPlugin` for the rounds it fires: a
        // round is not a body, so avian moves the rig and the sweep moves the
        // stream. No collision hooks - a round never reaches the pair filter.
        let mut app = unfinished_integrity_physics_app_with(PhysicsPlugins::default());
        app.add_plugins(NovaRoundPlugin);
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
            rounds::NovaRoundPlugin,
            test_support::{settle, unfinished_integrity_physics_app_with},
        };
        // Physics for the SHIP, `NovaRoundPlugin` for the rounds it fires: a
        // round is not a body, so avian moves the rig and the sweep moves the
        // stream. No collision hooks - a round never reaches the pair filter.
        let mut app = unfinished_integrity_physics_app_with(PhysicsPlugins::default());
        app.add_plugins(NovaRoundPlugin);
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
