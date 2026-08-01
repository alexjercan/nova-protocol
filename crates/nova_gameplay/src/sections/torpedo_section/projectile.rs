//! In-flight torpedo behavior: target tracking, arming, proportional-navigation
//! guidance (steer/thrust) and proximity detonation.

use super::*;

pub(super) fn update_target_position(
    mut commands: Commands,
    mut q_torpedo: Query<
        (
            Entity,
            Option<&mut TorpedoTargetPosition>,
            &TorpedoTargetEntity,
        ),
        With<TorpedoProjectileMarker>,
    >,
    q_target: Query<&Transform>,
) {
    for (torpedo, torpedo_target_position, target_entity) in &mut q_torpedo {
        let Ok(target_transform) = q_target.get(**target_entity) else {
            // The target died mid-flight. Don't delete the torpedo - that reads as
            // it blinking out of existence. Instead drop the dead target link and
            // let it keep flying toward the last known position (frozen in
            // `TorpedoTargetPosition`) until it arrives and detonates or its
            // lifetime expires. Removing the link also stops this lookup - and its
            // warning - from repeating every frame.
            debug!(
                "update_target_position: target {:?} gone; freezing torpedo {:?} on last known position",
                **target_entity, torpedo
            );
            commands.entity(torpedo).remove::<TorpedoTargetEntity>();
            continue;
        };

        // The position component is added on first lock and updated in place after,
        // so a never-locked torpedo has no `TorpedoTargetPosition` and flies straight.
        match torpedo_target_position {
            Some(mut position) => **position = target_transform.translation,
            None => {
                commands
                    .entity(torpedo)
                    .insert(TorpedoTargetPosition(target_transform.translation));
            }
        }
    }
}

/// Tick each torpedo's arming state so it can detonate only after it has cleared
/// the muzzle (see [`TorpedoArming`]).
pub(super) fn update_torpedo_arming(
    time: Res<Time>,
    mut q_torpedo: Query<(&Transform, &mut TorpedoArming), With<TorpedoProjectileMarker>>,
) {
    let dt = time.delta_secs();
    for (torpedo_transform, mut arming) in &mut q_torpedo {
        arming.tick(dt, torpedo_transform.translation);
    }
}

pub(super) fn torpedo_detonate_system(
    mut commands: Commands,
    q_torpedo: Query<
        (
            Entity,
            &Transform,
            &TorpedoTargetPosition,
            &TorpedoArming,
            &TorpedoBlast,
            &TorpedoSectionPartOf,
            Option<&ProjectileOwner>,
        ),
        // A shot-down torpedo must not detonate in the one-tick gap before
        // despawn_shot_down_torpedoes removes it (see TorpedoShotDownMarker).
        (
            With<TorpedoProjectileMarker>,
            Without<super::TorpedoShotDownMarker>,
        ),
    >,
) {
    for (torpedo, torpedo_transform, torpedo_target_position, arming, blast, part_of, owner) in
        &q_torpedo
    {
        // Do not detonate until the torpedo has armed (cleared the muzzle), so a
        // shot at a nearby target does not blow up on spawn.
        if !arming.is_armed() {
            continue;
        }

        let distance = torpedo_transform
            .translation
            .distance(**torpedo_target_position);

        // Proximity fuze: fire within half the blast radius of the target.
        if distance < blast.radius * 0.5 {
            commands.entity(torpedo).despawn();
            // A nova typed blast (Explosive), not bcs's untyped `blast_damage`:
            // nova owns the falloff + trigger so the blast obeys the resistance
            // table. It carries no bcs BlastDamageMarker, so bcs's blast observer
            // stays dormant and the damage is not double-counted.
            let mut blast_entity = commands.spawn((
                nova_blast(blast.radius, blast.damage, DamageType::Explosive),
                Transform::from_translation(torpedo_transform.translation),
                part_of.clone(),
                TempEntity(0.1),
            ));
            // The blast inherits the torpedo's owner so the damage it deals
            // stays attributable to the ship that fired the torpedo: nova
            // populates HealthApplyDamage.source with the blast collider,
            // and the AI threat model resolves that source to a shooter
            // through ProjectileOwner.
            if let Some(&owner) = owner {
                blast_entity.insert(owner);
            }
        }
    }
}

/// Proportional-navigation steering direction.
///
/// Returns the unit direction the torpedo should point its nose (and thrust)
/// toward to intercept the target. `rel_pos` is the line-of-sight `target - torpedo`
/// and `target_vel` / `missile_vel` are world-space velocities.
///
/// The command is anchored on the line of sight, not on the torpedo's velocity:
///
/// - Base course: the constant-bearing intercept ("lead collision course"). Split
///   the target's velocity into the component across the line of sight and match
///   it, spending the rest of the torpedo's speed closing along the line of sight:
///   `lead = (target_perp + los * sqrt(speed^2 - |target_perp|^2)) / speed`. For a
///   stationary target this is exactly "point at the target"; for a crossing
///   target it is the exact intercept heading at the given speed.
/// - PN damping: the classic LOS-rate term. With `omega = cross(rel_pos, rel_vel)
///   / dot(rel_pos, rel_pos)`, add `nav_constant * cross(omega, heading)` (clamped)
///   to null residual line-of-sight rotation - drift, disturbances, target
///   maneuvers.
///
/// Anchoring on the LOS matters because the torpedo launches slowly *sideways* out
/// of the bay: a velocity-anchored command (`V + N * cross(omega, V)`) from that
/// state keeps re-commanding the current drift direction (omega is tiny while the
/// target is far), so the torpedo climbs away instead of turning onto the target.
/// The LOS-anchored form points at/ahead of the target from any initial velocity.
pub(super) fn pn_steer_direction(
    rel_pos: Vec3,
    target_vel: Vec3,
    missile_vel: Vec3,
    nav_constant: f32,
) -> Vec3 {
    let heading = missile_vel.try_normalize();

    let Some(los) = rel_pos.try_normalize() else {
        // Target coincident with the torpedo: keep the current heading.
        return heading.unwrap_or(Vec3::NEG_Z);
    };

    // Constant-bearing lead. Plan with at least the target's speed so the lead
    // stays defined while the torpedo is still accelerating up to speed.
    let target_perp = target_vel - target_vel.dot(los) * los;
    let planning_speed = missile_vel.length().max(target_vel.length()).max(1e-3);
    let closing = (planning_speed * planning_speed - target_perp.length_squared())
        .max(0.0)
        .sqrt();
    let lead = (target_perp + los * closing) / planning_speed;

    // PN damping: null the residual line-of-sight rotation.
    let pn_correction = match heading {
        Some(heading) => {
            let los_rate = rel_pos.cross(target_vel - missile_vel) / rel_pos.length_squared();
            (nav_constant * los_rate.cross(heading)).clamp_length_max(1.0)
        }
        None => Vec3::ZERO,
    };

    (lead + pn_correction).try_normalize().unwrap_or(los)
}

/// Compute each torpedo's PN steering direction into [`TorpedoSteering`], using the
/// target entity's velocity (zero once the target is lost, so PN degrades to
/// pursuit of the frozen target position).
pub(super) fn torpedo_pn_guidance(
    mut q_torpedo: Query<
        (
            &Transform,
            Option<&TorpedoTargetPosition>,
            &LinearVelocity,
            Option<&TorpedoTargetEntity>,
            &TorpedoGuidance,
            &mut TorpedoSteering,
        ),
        With<TorpedoProjectileMarker>,
    >,
    q_target_velocity: Query<&LinearVelocity>,
) {
    for (transform, target_position, velocity, target_entity, guidance, mut steering) in
        &mut q_torpedo
    {
        // No target locked (or ever locked): fly straight ahead, holding heading,
        // rather than steering toward the world origin.
        let Some(target_position) = target_position else {
            **steering = transform.forward().into();
            continue;
        };

        let target_velocity = target_entity
            .and_then(|target| q_target_velocity.get(**target).ok())
            .map(|v| **v)
            .unwrap_or(Vec3::ZERO);

        let rel_pos = **target_position - transform.translation;

        **steering =
            pn_steer_direction(rel_pos, target_velocity, **velocity, guidance.nav_constant);
    }
}

/// Orient the torpedo's PD controller toward the PN steering direction.
pub(super) fn torpedo_sync_system(
    q_torpedo: Query<&TorpedoSteering, With<TorpedoProjectileMarker>>,
    mut q_controller: Query<
        (&mut ControllerSectionRotationInput, &ChildOf),
        (With<ControllerSectionMarker>, With<TorpedoControllerMarker>),
    >,
) {
    for (mut controller_input, ChildOf(torpedo)) in &mut q_controller {
        if let Ok(steering) = q_torpedo.get(*torpedo) {
            **controller_input = Quat::from_rotation_arc(Vec3::NEG_Z, **steering);
        }
    }
}

/// Width of the taper band below `max_speed` over which thrust fades to zero, in
/// units per second.
const THRUST_TAPER_BAND: f32 = 5.0;

/// Thrust remaining given the velocity component *along the nose*: 1.0 well
/// below `max_speed`, fading linearly to 0.0 over the last
/// [`THRUST_TAPER_BAND`] u/s. Gating on the along-nose speed (not total speed)
/// caps cruise speed without killing steering: at cruise, pointing straight
/// ahead means no thrust, but the moment guidance swings the nose to turn, the
/// along-nose component drops and thrust returns as lateral authority. A cap on
/// total speed instead leaves the torpedo ballistic at cruise - unable to steer
/// at all. Never negative: the cap cuts thrust, it does not brake.
pub(super) fn thrust_headroom(speed_along_nose: f32, max_speed: f32) -> f32 {
    ((max_speed - speed_along_nose) / THRUST_TAPER_BAND).clamp(0.0, 1.0)
}

/// Thrust along the nose: full thrust when the nose is aligned with the steering
/// direction, easing off while the torpedo is still turning onto course, and
/// tapering to zero when already at cruise speed along the nose (see
/// [`thrust_headroom`] and [`TorpedoSectionConfig::max_speed`]).
pub(super) fn torpedo_thrust_system(
    q_torpedo: Query<
        (
            &Transform,
            &TorpedoSteering,
            &LinearVelocity,
            &TorpedoGuidance,
        ),
        With<TorpedoProjectileMarker>,
    >,
    mut q_thruster: Query<
        (&mut ThrusterSectionInput, &ChildOf),
        (With<ThrusterSectionMarker>, With<TorpedoThrusterMarker>),
    >,
) {
    for (mut thruster_input, ChildOf(torpedo)) in &mut q_thruster {
        if let Ok((transform, steering, velocity, guidance)) = q_torpedo.get(*torpedo) {
            let nose = transform.forward();
            let alignment = nose.dot(**steering).clamp(0.0, 1.0);
            let headroom = thrust_headroom(velocity.dot(nose.into()), guidance.max_speed);
            **thruster_input = alignment * headroom;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unarmed_torpedo_does_not_detonate_on_target() {
        // Regression: a torpedo sitting right on its target must not detonate
        // while unarmed - this is the "spawns too close and just dies" bug.
        let mut app = App::new();
        app.add_systems(Update, torpedo_detonate_system);

        let part_of = app.world_mut().spawn_empty().id();
        let torpedo = app
            .world_mut()
            .spawn((
                TorpedoProjectileMarker,
                Transform::from_translation(Vec3::ZERO),
                TorpedoTargetPosition(Vec3::ZERO), // on target: distance 0 < blast radius * 0.5
                TorpedoArming::new(0.5, 5.0, Vec3::ZERO), // not armed
                TorpedoBlast {
                    radius: 30.0,
                    damage: 100.0,
                },
                TorpedoSectionPartOf(part_of),
            ))
            .id();

        app.update();

        assert!(
            app.world().entities().contains(torpedo),
            "unarmed torpedo should survive even on top of its target"
        );
    }

    #[test]
    fn armed_torpedo_detonates_on_target() {
        // Once armed, the same on-target torpedo detonates (despawns).
        let mut app = App::new();
        app.add_systems(Update, torpedo_detonate_system);

        let part_of = app.world_mut().spawn_empty().id();
        let mut arming = TorpedoArming::new(0.5, 5.0, Vec3::ZERO);
        arming.tick(1.0, Vec3::ZERO); // arm via time

        let torpedo = app
            .world_mut()
            .spawn((
                TorpedoProjectileMarker,
                Transform::from_translation(Vec3::ZERO),
                TorpedoTargetPosition(Vec3::ZERO),
                arming,
                TorpedoBlast {
                    radius: 30.0,
                    damage: 100.0,
                },
                TorpedoSectionPartOf(part_of),
            ))
            .id();

        app.update();

        assert!(
            !app.world().entities().contains(torpedo),
            "armed torpedo on its target should detonate and despawn"
        );
    }

    #[test]
    fn the_detonation_blast_inherits_the_torpedo_owner() {
        // The blast entity must stay attributable to the ship that fired
        // the torpedo: nova's typed blast puts the blast collider into
        // HealthApplyDamage.source, and the AI threat model resolves it to
        // a shooter through ProjectileOwner. The blast is a NovaBlast
        // (Explosive), not bcs's untyped BlastDamageMarker volume.
        let mut app = App::new();
        app.add_systems(Update, torpedo_detonate_system);

        let owner = app.world_mut().spawn_empty().id();
        let part_of = app.world_mut().spawn_empty().id();
        let mut arming = TorpedoArming::new(0.5, 5.0, Vec3::ZERO);
        arming.tick(1.0, Vec3::ZERO); // arm via time

        app.world_mut().spawn((
            TorpedoProjectileMarker,
            ProjectileOwner(owner),
            Transform::from_translation(Vec3::ZERO),
            TorpedoTargetPosition(Vec3::ZERO),
            arming,
            TorpedoBlast {
                radius: 30.0,
                damage: 100.0,
            },
            TorpedoSectionPartOf(part_of),
        ));

        app.update();

        let mut q_blast = app
            .world_mut()
            .query_filtered::<(&ProjectileOwner, &NovaBlast), With<NovaBlast>>();
        let (blast_owner, blast) = q_blast
            .single(app.world())
            .expect("the detonation spawned exactly one owned nova blast");
        assert_eq!(**blast_owner, owner);
        assert_eq!(
            blast.kind,
            DamageType::Explosive,
            "a torpedo blast is Explosive"
        );
    }

    #[test]
    fn torpedo_survives_target_loss_and_freezes_position() {
        // Regression: when the target dies mid-flight the torpedo must not vanish.
        // It should keep its last known target position and drop the dead link.
        let mut app = App::new();
        app.add_systems(Update, update_target_position);

        let target = app
            .world_mut()
            .spawn(Transform::from_translation(Vec3::new(1.0, 2.0, 3.0)))
            .id();
        let torpedo = app
            .world_mut()
            .spawn((
                TorpedoProjectileMarker,
                TorpedoTargetPosition(Vec3::ZERO),
                TorpedoTargetEntity(target),
            ))
            .id();

        // Frame 1: target alive -> the torpedo tracks it.
        app.update();
        assert_eq!(
            **app.world().get::<TorpedoTargetPosition>(torpedo).unwrap(),
            Vec3::new(1.0, 2.0, 3.0)
        );

        // Target dies mid-flight.
        app.world_mut().entity_mut(target).despawn();

        // Frame 2: torpedo must survive, freeze on the last known position, and
        // drop the dead target link (so it stops looking it up every frame).
        app.update();
        assert!(
            app.world().entities().contains(torpedo),
            "torpedo must not vanish when its target dies"
        );
        assert_eq!(
            **app.world().get::<TorpedoTargetPosition>(torpedo).unwrap(),
            Vec3::new(1.0, 2.0, 3.0),
            "torpedo should freeze on the last known target position"
        );
        assert!(
            app.world().get::<TorpedoTargetEntity>(torpedo).is_none(),
            "the dead target link should be removed"
        );
    }

    #[test]
    fn pn_leads_a_crossing_target() {
        // Torpedo at origin flying forward (-Z); target ahead, crossing to +X.
        // PN must steer the nose to lead the target (a +X component), not point
        // straight down -Z at where the target is now.
        let missile_vel = Vec3::new(0.0, 0.0, -50.0);
        let rel_pos = Vec3::new(0.0, 0.0, -100.0); // target 100 ahead
        let target_vel = Vec3::new(20.0, 0.0, 0.0); // crossing to +X

        let dir = pn_steer_direction(rel_pos, target_vel, missile_vel, 3.0);

        assert!(
            dir.x > 0.01,
            "PN should lead a +X-crossing target with a +X heading component, got {dir:?}"
        );
        assert!(
            dir.z < 0.0,
            "torpedo should still be heading generally forward"
        );
        assert!(
            dir.is_normalized(),
            "steering direction must be a unit vector"
        );
    }

    #[test]
    fn pn_pursues_a_stationary_target_straight() {
        // Target directly ahead, not moving, torpedo closing straight in: there is
        // no line-of-sight rotation, so PN adds no lead - it points at the target.
        let missile_vel = Vec3::new(0.0, 0.0, -50.0);
        let rel_pos = Vec3::new(0.0, 0.0, -100.0);

        let dir = pn_steer_direction(rel_pos, Vec3::ZERO, missile_vel, 3.0);

        assert!(
            (dir - Vec3::NEG_Z).length() < 1e-3,
            "expected straight pursuit, got {dir:?}"
        );
    }

    #[test]
    fn pn_points_at_a_stationary_target_from_a_sideways_launch() {
        // THE regression for "the torpedo flies off and never turns toward the
        // target": the torpedo leaves the bay slowly and sideways (spawner up,
        // ~1 u/s), i.e. velocity perpendicular to the line of sight. The command
        // must point (essentially) at the target, not along the current velocity.
        // The old velocity-anchored form returned ~(0, 1, 0) here.
        let missile_vel = Vec3::new(0.0, 1.0, 0.0); // slow, straight up
        let rel_pos = Vec3::new(0.0, 0.0, -100.0); // target ahead

        let dir = pn_steer_direction(rel_pos, Vec3::ZERO, missile_vel, 3.0);

        assert!(
            dir.dot(Vec3::NEG_Z) > 0.95,
            "command must point at the target regardless of launch velocity, got {dir:?}"
        );
    }

    #[test]
    fn pn_handles_degenerate_inputs() {
        // Target on top of the torpedo, and a stationary torpedo: both must return
        // a finite unit direction, never NaN.
        let coincident =
            pn_steer_direction(Vec3::ZERO, Vec3::ZERO, Vec3::new(0.0, 0.0, -10.0), 3.0);
        assert!(coincident.is_finite() && coincident.is_normalized());

        let stationary =
            pn_steer_direction(Vec3::new(0.0, 0.0, -50.0), Vec3::ZERO, Vec3::ZERO, 3.0);
        assert!(stationary.is_finite() && stationary.is_normalized());
        assert!(
            (stationary - Vec3::NEG_Z).length() < 1e-3,
            "a stationary torpedo should pursue the target directly"
        );
    }

    /// Closed-loop model of the torpedo the way it actually flies: the nose turns
    /// toward `steer(...)` at up to `max_turn_rate` rad/s, and thrust accelerates
    /// along the nose scaled by nose/command alignment and by the cruise-speed
    /// headroom (mirroring `torpedo_thrust_system`). Starting conditions mirror
    /// the real launch: slow, sideways. Returns the closest approach to the
    /// target over the run.
    #[allow(clippy::too_many_arguments)]
    fn simulate_thrust_intercept(
        mut pos: Vec3,
        mut vel: Vec3,
        mut nose: Vec3,
        mut target: Vec3,
        target_vel: Vec3,
        max_turn_rate: f32,
        accel: f32,
        max_speed: f32,
        damping: f32,
        dt: f32,
        steps: usize,
        steer: impl Fn(Vec3, Vec3, Vec3) -> Vec3,
    ) -> f32 {
        let mut closest = pos.distance(target);
        for _ in 0..steps {
            let desired = steer(target - pos, target_vel, vel);
            let angle = nose.angle_between(desired);
            let axis = nose.cross(desired);
            if axis.length() > 1e-6 && angle > 1e-6 {
                let step = (max_turn_rate * dt).min(angle);
                nose = (Quat::from_axis_angle(axis.normalize(), step) * nose).normalize();
            }
            let thrust =
                nose.dot(desired).clamp(0.0, 1.0) * thrust_headroom(vel.dot(nose), max_speed);
            vel += nose * accel * thrust * dt;
            vel -= vel * damping * dt; // linear drag, as on the real body
            pos += vel * dt;
            target += target_vel * dt;
            closest = closest.min(pos.distance(target));
        }
        closest
    }

    /// The real launch state in the examples: at rest but drifting up at ~1 u/s
    /// (spawner up), nose forward (-Z), then guided by the PN law with the
    /// torpedo's rough turn rate, thrust authority, and cruise-speed cap.
    fn launch_closest_approach(target: Vec3, target_vel: Vec3) -> f32 {
        simulate_thrust_intercept(
            Vec3::ZERO,
            Vec3::new(0.0, 1.0, 0.0), // launched sideways at 1 u/s
            Vec3::NEG_Z,              // nose forward
            target,
            target_vel,
            3.0,  // max turn rate rad/s
            25.0, // thrust acceleration
            35.0, // cruise speed cap
            0.8,  // linear damping, as configured on the projectile
            0.02, // dt
            500,  // 10 s
            |r, tv, v| pn_steer_direction(r, tv, v, 3.0),
        )
    }

    #[test]
    fn thrust_tapers_to_zero_at_cruise_speed() {
        // Below the taper band: full thrust. At/above cruise: none. The cap keeps
        // the turning circle (speed / turn rate) inside the proximity fuze so the
        // torpedo cannot end up orbiting its target at high speed.
        assert_eq!(thrust_headroom(0.0, 35.0), 1.0);
        assert_eq!(thrust_headroom(20.0, 35.0), 1.0);
        assert!((thrust_headroom(32.5, 35.0) - 0.5).abs() < 1e-6);
        assert_eq!(thrust_headroom(35.0, 35.0), 0.0);
        assert_eq!(
            thrust_headroom(50.0, 35.0),
            0.0,
            "cap cuts thrust, never brakes"
        );
    }

    #[test]
    fn pn_turns_a_sideways_launch_onto_a_stationary_target() {
        // Closed-loop version of the reported bug: from the real launch state the
        // torpedo must come around and hit a stationary target ahead, instead of
        // thrusting off along its launch drift.
        let miss = launch_closest_approach(Vec3::new(0.0, 0.0, -60.0), Vec3::ZERO);
        assert!(
            miss < 5.0,
            "torpedo should reach the stationary target, closest was {miss}"
        );
    }

    /// A closest approach that counts as a kill: inside the proximity fuze
    /// (`BLAST_RADIUS * 0.5` = 15). Crossing intercepts from a sideways launch
    /// carry a few units of turn-rate lag at the endgame (measured ~8), which the
    /// fuze absorbs; a broken law misses by the full crossing distance instead.
    const HIT: f32 = 10.0;

    #[test]
    fn pn_intercepts_a_crossing_target() {
        // From the real launch state, intercept a target crossing the range.
        let miss = launch_closest_approach(Vec3::new(-30.0, 0.0, -80.0), Vec3::new(15.0, 0.0, 0.0));
        assert!(
            miss < HIT,
            "PN should intercept the crossing target, closest approach was {miss}"
        );
    }

    #[test]
    fn pn_intercepts_a_target_crossing_either_way() {
        // Guards against a sign bug that only works for one crossing direction.
        for cross in [15.0f32, -15.0] {
            let miss = launch_closest_approach(
                Vec3::new(-2.0 * cross, 0.0, -80.0),
                Vec3::new(cross, 0.0, 0.0),
            );
            assert!(
                miss < HIT,
                "PN should intercept a target crossing at {cross}, miss was {miss}"
            );
        }
    }

    #[test]
    fn untargeted_torpedo_flies_straight_not_toward_origin() {
        // Regression: a torpedo fired with no lock (no TorpedoTargetPosition) must
        // hold its heading, not steer at the world origin. Place it off-origin so
        // "straight ahead" (-Z) is clearly distinct from "toward origin" (-X).
        let mut app = App::new();
        app.add_systems(Update, torpedo_pn_guidance);

        let torpedo = app
            .world_mut()
            .spawn((
                TorpedoProjectileMarker,
                Transform::from_translation(Vec3::new(100.0, 0.0, 0.0)), // forward is -Z
                LinearVelocity(Vec3::new(0.0, 0.0, -40.0)),
                TorpedoGuidance {
                    nav_constant: 3.0,
                    max_speed: 35.0,
                },
                TorpedoSteering(Vec3::NEG_Z),
            ))
            .id();

        app.update();

        let steering = **app.world().get::<TorpedoSteering>(torpedo).unwrap();
        assert!(
            (steering - Vec3::NEG_Z).length() < 1e-3,
            "untargeted torpedo should fly straight ahead (-Z), got {steering:?}"
        );
    }
}
