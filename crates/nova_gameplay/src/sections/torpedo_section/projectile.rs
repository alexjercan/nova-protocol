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

// TODO(20260525-133023): Add some nice visuals for the explosion itself
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
        ),
        // A shot-down torpedo must not detonate in the one-tick gap before
        // despawn_shot_down_torpedoes removes it (see TorpedoShotDownMarker).
        (
            With<TorpedoProjectileMarker>,
            Without<super::TorpedoShotDownMarker>,
        ),
    >,
) {
    for (torpedo, torpedo_transform, torpedo_target_position, arming, blast, part_of) in &q_torpedo
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
            commands.spawn((
                blast_damage(BlastDamageConfig {
                    radius: blast.radius,
                    max_damage: blast.damage,
                }),
                Transform::from_translation(torpedo_transform.translation),
                part_of.clone(),
                TempEntity(0.1),
            ));
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
