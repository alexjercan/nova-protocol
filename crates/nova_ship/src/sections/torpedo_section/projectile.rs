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
    q_target: Query<(&Transform, Option<&ComputedCenterOfMass>)>,
) {
    for (torpedo, torpedo_target_position, target_entity) in &mut q_torpedo {
        let Ok((target_transform, target_com)) = q_target.get(**target_entity) else {
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

        // Home on the live structure, not the root origin: the origin is only the
        // build spot of the ship's first sections, so shooting away a large
        // enemy's forward half leaves the torpedo flying at empty space. Every
        // other aim consumer follows the same rule (`sections/mod.rs`).
        let anchor = live_structure_anchor(target_transform, target_com);

        // The position component is added on first lock and updated in place after,
        // so a never-locked torpedo has no `TorpedoTargetPosition` and flies straight.
        match torpedo_target_position {
            Some(mut position) => **position = anchor,
            None => {
                commands
                    .entity(torpedo)
                    .insert(TorpedoTargetPosition(anchor));
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

/// Count each ejecting torpedo down and light its drive.
///
/// This is the whole ignition: dropping [`TorpedoColdLaunch`] restores the
/// torpedo to every system that filters on its absence, so guidance, the
/// weave, the attitude sync, the thruster and the fuze all resume together on
/// the tick the motor catches. Clearing the sections' `ColliderDisabled` is
/// the other half - the warhead becomes solid at the same instant it becomes
/// dangerous, which is the pairing the safety separation is about.
///
/// Runs on the FIXED clock with the launch chain, not with the control inputs:
/// a collider reappearing on a body physics is about to step has to reappear
/// on physics' own clock.
pub(super) fn ignite_cold_torpedoes(
    mut commands: Commands,
    time: Res<Time>,
    q_children: Query<&Children>,
    mut q_cold: Query<(Entity, &mut TorpedoColdLaunch, &Transform), With<TorpedoProjectileMarker>>,
) {
    let dt = time.delta_secs();
    for (torpedo, mut cold, transform) in &mut q_cold {
        cold.remaining -= dt;
        if cold.remaining > 0.0 {
            continue;
        }

        debug!("ignite_cold_torpedoes: torpedo {torpedo:?} lights its drive");
        commands.entity(torpedo).remove::<TorpedoColdLaunch>();
        // The colliders sit on the section children, and `ColliderDisabled`
        // applies only to the entity carrying it - the root has no collider to
        // re-enable.
        if let Ok(children) = q_children.get(torpedo) {
            for &child in children {
                commands.entity(child).remove::<ColliderDisabled>();
            }
        }
        commands.trigger(TorpedoIgnited {
            torpedo,
            at: transform.translation,
        });
    }
}

/// How close the torpedo root gets to its target's SKIN before the warhead fires.
///
/// Three units is still a near-contact burst and delivers 90% of a standard
/// 30-unit warhead's rated pressure at the skin. It also leaves room for the
/// torpedo's own two-unit body and for both bodies to close during the fixed
/// physics step before this render-clock fuze runs again. At one unit, physical
/// contact sometimes destroyed the torpedo first, producing a dud on the hull.
///
/// This is measured to the nearest point on the target, not its centre. The old
/// centre-based half-blast-radius fuze delivered only half pressure and cut rock
/// craters in vacuum beside large targets.
const CONTACT_FUZE: f32 = 3.0;

/// How near a torpedo has to get before it fires, given how much the gap closes
/// in `dt`.
///
/// [`CONTACT_FUZE`] on its own is a window a fast closer can step straight over:
/// a torpedo closing at cruise on a ship running the other way covers several
/// units in one frame, and one that skipped its own window would fly on with the
/// warhead still in it. Never smaller than the step about to be taken, so a
/// torpedo that would pass through the skin fires instead of through it.
///
/// The speed here is the CLOSING speed, not the torpedo's own. They are the same
/// against a station and nothing alike against a crosser, which is the case this
/// window exists for - a target driving into the torpedo shuts the gap with its
/// own velocity, and a window sized on the torpedo alone is the gap it does not
/// cover. That miss is what a contact dud is: the fuze samples once outside the
/// window and once already inside the hull.
pub(super) fn contact_reach(closing_speed: f32, dt: f32) -> f32 {
    CONTACT_FUZE.max(closing_speed * dt)
}

/// Distance from `at` to the nearest point on `target`'s own skin, saturating at
/// `limit`; `None` when that body has no collider in the world.
///
/// Read off the target's OWN collider list, so a torpedo threading a formation
/// cannot fuze on the wrong ship and a ship built out of two hundred section
/// colliders answers with whichever one is nearest the nose. Solid, so a nose
/// already inside the hull reads zero instead of the distance back out to the
/// skin.
///
/// NOT a broad-phase point projection. The world-wide form declines foreign
/// colliders in a predicate, and a declined proxy returns infinity - which
/// never tightens the traversal's search radius, so the walk degrades toward
/// the whole tree (4,663 colliders in a headless duel) for every torpedo, every
/// frame. It was the single worst-attributed system of a measured fight.
///
/// `limit` is the fuze reach, and it is what keeps the direct form cheap: a
/// collider whose bounding box is already further out than the best answer so
/// far cannot hold the nearest point, so it is never projected. The result is
/// exact below `limit` and saturates at it above, which is all a `distance <
/// reach` test can observe.
fn distance_to_skin(
    q_body_colliders: &Query<&RigidBodyColliders>,
    q_collider: &Query<(&Position, &Rotation, &Collider, &ColliderAabb)>,
    target: Entity,
    at: Vec3,
    limit: f32,
) -> Option<f32> {
    let attached = q_body_colliders.get(target).ok()?;
    if attached.is_empty() {
        return None;
    }

    let mut nearest = limit;
    for collider_entity in attached {
        let Ok((position, rotation, collider, aabb)) = q_collider.get(collider_entity) else {
            continue;
        };
        // The AABB encloses the shape, so its distance is a lower bound on the
        // skin's - a box further out than the best answer cannot beat it.
        let outside = (aabb.min - at).max(at - aabb.max).max(Vec3::ZERO);
        if outside.length() >= nearest {
            continue;
        }
        let (point, _) = collider.project_point(position.0, *rotation, at, true);
        nearest = nearest.min(point.distance(at));
    }
    Some(nearest)
}

/// Fuze every armed torpedo that has reached its target: despawn it and spawn
/// the blast.
///
/// Requiring `&TorpedoTargetPosition` is deliberate. A torpedo launched with no
/// lock never receives one (`intent.rs` inserts `TorpedoTargetChosen` alone when
/// `CombatLock` is `None`), so it CANNOT detonate - it flies its full lifetime,
/// deals a contact ding and is deleted. A no-lock launch is a misfire, and the
/// bay still spends the round.
///
/// Two cases, and the difference is whether there is anything to touch:
///
/// - a locked BODY - ship or rock, no distinction - is fuzed on its own skin,
///   almost at contact, which is what puts the warhead's pressure and its crater
///   where the target actually is;
/// - a bare aim POINT is fuzed at half the blast radius, as it always was. A
///   torpedo committed to a position (`stress_torpedoes`, an emplacement) or one
///   whose target died mid-flight has no surface to reach, and "the warhead
///   covers the point" is the only sense "arrived" can have there.
#[expect(
    clippy::type_complexity,
    reason = "the fuze reads the whole projectile: pose, lock, arming, warhead and owner"
)]
pub(super) fn torpedo_detonate_system(
    mut commands: Commands,
    time: Res<Time>,
    q_torpedo: Query<
        (
            Entity,
            &Transform,
            &TorpedoTargetPosition,
            Option<&TorpedoTargetEntity>,
            Option<&LinearVelocity>,
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
            // An unignited warhead cannot fuze. The arming clock has cleared
            // it by now on a stock bay, and arming is about separation from
            // the ship, not about whether the thing is under power.
            Without<super::TorpedoColdLaunch>,
        ),
    >,
    q_body_colliders: Query<&RigidBodyColliders>,
    q_collider: Query<(&Position, &Rotation, &Collider, &ColliderAabb)>,
    // The target's own motion, for the closing speed the fuze window is sized
    // on. Read-only and separate from `q_torpedo`, which reads the torpedo's.
    q_target_motion: Query<&LinearVelocity>,
) {
    let dt = time.delta_secs();
    for (
        torpedo,
        torpedo_transform,
        torpedo_target_position,
        target_entity,
        velocity,
        arming,
        blast,
        part_of,
        owner,
    ) in &q_torpedo
    {
        // Do not detonate until the torpedo has armed (cleared the muzzle), so a
        // shot at a nearby target does not blow up on spawn.
        if !arming.is_armed() {
            continue;
        }

        let at = torpedo_transform.translation;
        // Closing speed, not the torpedo's: a crosser driving into the torpedo
        // shuts the gap with its own velocity too, and a window sized on the
        // torpedo alone leaves exactly that much of the step uncovered. Falls
        // back to the torpedo's own speed when the target has no body to read.
        let torpedo_velocity = velocity.map_or(Vec3::ZERO, |velocity| velocity.0);
        let target_velocity = target_entity
            .and_then(|target| q_target_motion.get(**target).ok())
            .map_or(Vec3::ZERO, |velocity| velocity.0);
        let closing_speed = (torpedo_velocity - target_velocity).length();
        let contact = contact_reach(closing_speed, dt);
        let (distance, reach) = match target_entity.and_then(|target| {
            distance_to_skin(&q_body_colliders, &q_collider, **target, at, contact)
        }) {
            Some(skin) => (skin, contact),
            None => (at.distance(**torpedo_target_position), blast.radius * 0.5),
        };

        if distance < reach {
            debug!(
                "torpedo_detonate_system: {torpedo:?} fuzed {distance:.2}u out, reach {reach:.2}u"
            );
            // try_despawn: the lifetime sweep can queue a despawn for this same
            // torpedo in the same flush (a torpedo that fuzes on the frame its
            // lifetime expires). A plain despawn on the loser of that race is a
            // hard panic under the FallbackErrorHandler(panic) the autopilot and
            // probe runs install.
            commands.entity(torpedo).try_despawn();
            // A nova typed blast (Explosive): nova owns the falloff and the
            // trigger, and it is the only blast path in the app, so the damage
            // is not double-counted.
            let mut blast_entity = commands.spawn((
                nova_blast(blast.radius, blast.damage, DamageType::Explosive),
                Transform::from_translation(torpedo_transform.translation),
                part_of.clone(),
                // The torpedo's own velocity, not the closing speed read above:
                // the ejecta leave the warhead, and the warhead was going this
                // way. The target's motion belongs to the fuze window, not to
                // where the fireball goes.
                BlastMomentum(torpedo_velocity),
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
        (
            With<TorpedoProjectileMarker>,
            Without<super::TorpedoColdLaunch>,
        ),
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

/// Weave amplitude scale by range to the target: full weave while the torpedo
/// is still running in, fading to none on final approach.
///
/// The fade is what separates evasive ordnance from a drunk one. Only the MEAN
/// course of the corkscrew is the intercept, so a torpedo still weaving on the
/// last stretch flies its helix past a target that never moved. Full amplitude
/// beyond three blast radii, linearly to zero at half a blast radius: the run-in
/// is evasive and the last stretch is a straight sprint - which is also the
/// defender's cleanest shot, and deliberately so.
///
/// The band is measured off the blast radius and NOT off the fuze, which is now
/// a contact fuze a hundredth of the distance away (see [`contact_reach`]). The
/// terminal sprint has to start where the corkscrew stops helping - out at
/// point-defense range - not where the warhead finally fires.
pub(super) fn weave_fade(distance_to_target: f32, blast_radius: f32) -> f32 {
    let terminal = blast_radius * 0.5;
    let full = blast_radius * 3.0;
    ((distance_to_target - terminal) / (full - terminal).max(f32::EPSILON)).clamp(0.0, 1.0)
}

/// Tilt `steer` by `angle` toward `offset`, after spinning `offset` by `spin`
/// radians about `steer`. Returns the perturbed command and the advanced
/// offset.
///
/// The command is PERTURBED, never replaced: the result sits exactly `angle`
/// off the guidance solution and reduces to the solution itself at zero angle,
/// so every property the guidance law has (leading a crosser, pursuing a
/// stationary target) survives the weave.
///
/// The offset is carried as state and advanced incrementally rather than being
/// rebuilt from `steer` each step. A basis built from the command
/// (`any_orthonormal_pair`) flips branch as the command crosses an axis, which
/// would snap the corkscrew a half turn mid-flight; spinning and
/// re-orthogonalising an existing offset is continuous by construction.
pub(super) fn weave_steer_direction(
    steer: Vec3,
    offset: Vec3,
    angle: f32,
    spin: f32,
) -> (Vec3, Vec3) {
    let Some(steer) = steer.try_normalize() else {
        return (steer, offset);
    };
    let spun = Quat::from_axis_angle(steer, spin) * offset;
    let offset = (spun - steer * spun.dot(steer))
        .try_normalize()
        .unwrap_or_else(|| steer.any_orthonormal_vector());
    (
        (steer * angle.cos() + offset * angle.sin()).normalize(),
        offset,
    )
}

/// Lay the terminal weave over each torpedo's guidance command (see
/// [`TorpedoWeave`]).
///
/// Requiring `&TorpedoTargetPosition` is what keeps a dumb-fired torpedo
/// straight: with nothing to run in on there is no defender to evade, and a
/// corkscrewing unguided round is just noise. Unarmed torpedoes are still
/// clearing the muzzle, where the guidance is busy turning the body onto course
/// from a slow sideways launch and has no authority to spare.
pub(super) fn torpedo_terminal_weave(
    time: Res<Time>,
    mut q_torpedo: Query<
        (
            &Transform,
            &TorpedoTargetPosition,
            &TorpedoArming,
            &TorpedoBlast,
            &mut TorpedoWeave,
            &mut TorpedoSteering,
        ),
        (
            With<TorpedoProjectileMarker>,
            Without<super::TorpedoColdLaunch>,
        ),
    >,
) {
    let dt = time.delta_secs();
    for (transform, target_position, arming, blast, mut weave, mut steering) in &mut q_torpedo {
        if weave.angle <= 0.0 || !arming.is_armed() {
            continue;
        }
        let distance = transform.translation.distance(**target_position);
        let angle = weave.angle * weave_fade(distance, blast.radius);
        let (command, offset) =
            weave_steer_direction(**steering, weave.offset, angle, weave.rate * dt);
        weave.offset = offset;
        **steering = command;
    }
}

/// Orient the torpedo's PD controller toward the PN steering direction.
/// Deliberately NOT gated on [`TorpedoColdLaunch`], unlike everything else the
/// ejection suspends: attitude is the torpedo's own RCS and not its main drive,
/// so a coasting warhead still holds the nose it left the tube with.
///
/// Gating it was tried and is wrong twice over. `ControllerSectionRotationInput`
/// defaults to `Quat::IDENTITY`, so a torpedo nobody writes to is not left
/// alone - it is commanded to world identity, and the PD controller spent the
/// coast rotating it off its launch attitude. The drive then lit pointing
/// somewhere else and threw the run-in wide, which the real-flight weave test
/// caught as 13 u of swing on the arm that is supposed to fly straight.
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
        (
            With<TorpedoProjectileMarker>,
            Without<super::TorpedoColdLaunch>,
        ),
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
    use avian3d::collider_tree::ColliderTrees;

    use super::*;

    /// The window has to cover the step the GAP takes, not the step the torpedo
    /// takes. A crosser driving into the torpedo shuts the distance with its own
    /// velocity, and sizing the window on the torpedo alone leaves that share
    /// uncovered - the fuze then samples once outside the window and once
    /// already inside the hull, which is what the range counts as a contact dud.
    #[test]
    fn the_fuze_window_covers_a_crossing_target_closing_faster_than_the_torpedo() {
        let dt = 1.0 / 60.0;
        // Both readings have to clear the CONTACT_FUZE floor, or the floor is
        // what the comparison measures instead of the travel.
        let torpedo_speed = 300.0;
        // Head-on: the target adds its own 120 u/s to the closing rate.
        let closing = torpedo_speed + 120.0;

        let torpedo_only = contact_reach(torpedo_speed, dt);
        let with_crosser = contact_reach(closing, dt);

        assert!(
            with_crosser > torpedo_only,
            "the closing window must be wider than the torpedo-only one \
             ({with_crosser} vs {torpedo_only})"
        );
        assert!(
            with_crosser >= closing * dt,
            "the window must cover the whole distance the gap closes in one step \
             ({with_crosser} < {})",
            closing * dt
        );
        // The gap the old torpedo-only window left open is exactly the target's
        // share of the step - the distance a crosser could cross unsampled.
        assert!(
            (with_crosser - torpedo_only - 120.0 * dt).abs() < 1e-4,
            "the miss is the target's own travel over the step"
        );
    }

    /// Against something that is not moving the two readings agree, so the
    /// closing form is a strict widening rather than a different fuze.
    #[test]
    fn the_fuze_window_is_unchanged_against_a_stationary_target() {
        let dt = 1.0 / 60.0;
        assert_eq!(contact_reach(90.0, dt), contact_reach(90.0 - 0.0, dt));
        // Slow enough that the floor governs, which is the standoff the fuze is
        // authored for.
        assert_eq!(contact_reach(10.0, dt), CONTACT_FUZE);
    }

    #[test]
    fn unarmed_torpedo_does_not_detonate_on_target() {
        // Regression: a torpedo sitting right on its target must not detonate
        // while unarmed - this is the "spawns too close and just dies" bug.
        let mut app = App::new();
        // The fuze reads the clock for its swept window and the broad phase for
        // the target's skin; an empty tree means no skin to find, which is what
        // sends these rigs down the frozen-aim-point path.
        app.init_resource::<Time>();
        app.init_resource::<ColliderTrees>();
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

    /// A rig for the ejection countdown, on a manual clock so the delay is the
    /// only thing under test.
    fn ignition_app(remaining: f32) -> (App, Entity, Entity) {
        use std::time::Duration;

        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
            Duration::from_secs_f32(0.1),
        ));
        app.add_systems(Update, ignite_cold_torpedoes);

        let section = app.world_mut().spawn(ColliderDisabled).id();
        let torpedo = app
            .world_mut()
            .spawn((
                TorpedoProjectileMarker,
                Transform::from_translation(Vec3::ZERO),
                TorpedoColdLaunch { remaining },
            ))
            .add_child(section)
            .id();

        // bevy's first manual tick is dt 0, so a warm-up update leaves the
        // countdown untouched and every tick after it is worth 0.1 s.
        app.update();
        (app, torpedo, section)
    }

    fn is_cold(app: &App, torpedo: Entity) -> bool {
        app.world().get::<TorpedoColdLaunch>(torpedo).is_some()
    }

    fn is_solid(app: &App, section: Entity) -> bool {
        app.world().get::<ColliderDisabled>(section).is_none()
    }

    #[test]
    fn an_ejecting_torpedo_stays_inert_until_its_delay_runs_out() {
        let (mut app, torpedo, section) = ignition_app(0.6);
        app.update();
        app.update();
        assert!(is_cold(&app, torpedo), "0.2 s into a 0.6 s ejection");
        assert!(
            !is_solid(&app, section),
            "an unignited warhead has no collider to be shot down by or to hit the ship with"
        );
    }

    #[test]
    fn the_drive_and_the_collider_come_back_together() {
        // The pairing is the safety separation: the warhead becomes dangerous
        // and becomes solid on the same tick, never one before the other.
        let (mut app, torpedo, section) = ignition_app(0.25);
        for _ in 0..3 {
            app.update();
        }
        assert!(!is_cold(&app, torpedo), "the drive lit");
        assert!(is_solid(&app, section), "and the section can be hit again");
    }

    #[test]
    fn a_bay_authoring_no_ejection_delay_ignites_on_its_first_tick() {
        // `ignition_delay` zero is the old spawn-under-power behavior, and it
        // stays reachable without a second launch path.
        let (mut app, torpedo, section) = ignition_app(0.0);
        app.update();
        assert!(!is_cold(&app, torpedo));
        assert!(is_solid(&app, section));
    }

    #[test]
    fn an_ejecting_torpedo_cannot_detonate_on_top_of_its_target() {
        // Armed and on target, which is everything the fuze asks for - and it
        // is still cargo until the motor catches.
        let mut app = App::new();
        app.init_resource::<Time>();
        app.init_resource::<ColliderTrees>();
        app.add_systems(Update, torpedo_detonate_system);

        let part_of = app.world_mut().spawn_empty().id();
        let mut arming = TorpedoArming::new(0.5, 5.0, Vec3::ZERO);
        arming.tick(1.0, Vec3::ZERO);

        let torpedo = app
            .world_mut()
            .spawn((
                TorpedoProjectileMarker,
                Transform::from_translation(Vec3::ZERO),
                TorpedoTargetPosition(Vec3::ZERO),
                arming,
                TorpedoColdLaunch { remaining: 0.6 },
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
            "an unignited warhead must not fuze, armed or not"
        );
    }

    #[test]
    fn armed_torpedo_detonates_on_target() {
        // Once armed, the same on-target torpedo detonates (despawns).
        let mut app = App::new();
        // The fuze reads the clock for its swept window and the broad phase for
        // the target's skin; an empty tree means no skin to find, which is what
        // sends these rigs down the frozen-aim-point path.
        app.init_resource::<Time>();
        app.init_resource::<ColliderTrees>();
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
        // (Explosive), the only blast volume the game spawns.
        let mut app = App::new();
        // The fuze reads the clock for its swept window and the broad phase for
        // the target's skin; an empty tree means no skin to find, which is what
        // sends these rigs down the frozen-aim-point path.
        app.init_resource::<Time>();
        app.init_resource::<ColliderTrees>();
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

    /// The blast is a TRANSIENT and must SAY so. `TempEntity` is not just its
    /// self-cleanup: the scenario loader scopes transients on that component, so
    /// a blast spawned without one belongs to nothing, survives a Retry teardown
    /// and lands its damage in the NEXT scenario (task 20260816-103226). nova_ship
    /// cannot see the loader that depends on this, so the contract is pinned at
    /// the spawn.
    #[test]
    fn the_detonation_blast_is_a_transient() {
        let mut app = App::new();
        // The fuze reads the clock for its swept window and the broad phase for
        // the target's skin; an empty tree means no skin to find, which is what
        // sends these rigs down the frozen-aim-point path.
        app.init_resource::<Time>();
        app.init_resource::<ColliderTrees>();
        app.add_systems(Update, torpedo_detonate_system);

        let part_of = app.world_mut().spawn_empty().id();
        let mut arming = TorpedoArming::new(0.5, 5.0, Vec3::ZERO);
        arming.tick(1.0, Vec3::ZERO);

        app.world_mut().spawn((
            TorpedoProjectileMarker,
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
            .query_filtered::<&TempEntity, With<NovaBlast>>();
        let lifetime = q_blast
            .single(app.world())
            .expect("the detonation blast carries a TempEntity");
        assert!(
            **lifetime > 0.0,
            "the blast's lifetime must be a positive number of seconds, got {}",
            **lifetime
        );
    }

    /// A torpedo that fuzes on the frame its lifetime expires gets two queued
    /// despawns in one flush - the fuze here and the generic `TempEntity` sweep.
    /// With the game's panic-on-command-error fallback, the loser of that race
    /// used to hard-panic mid-run.
    #[test]
    fn a_torpedo_despawned_elsewhere_in_the_same_flush_does_not_panic() {
        use bevy::ecs::error::{panic, FallbackErrorHandler};

        // Stands in for the lifetime sweep, which despawns an expired torpedo
        // knowing nothing about the fuze. It queues FIRST, so the fuze's own
        // despawn is the one that lands on an already-dead entity.
        fn expire_torpedoes(
            mut commands: Commands,
            q: Query<Entity, With<TorpedoProjectileMarker>>,
        ) {
            for torpedo in &q {
                commands.entity(torpedo).despawn();
            }
        }

        let mut app = App::new();
        app.init_resource::<Time>();
        app.init_resource::<ColliderTrees>();
        app.insert_resource(FallbackErrorHandler(panic));
        // chain_ignore_deferred: no sync point between them, so both despawns
        // are applied from the same buffer - the real frame order.
        app.add_systems(
            Update,
            (expire_torpedoes, torpedo_detonate_system).chain_ignore_deferred(),
        );

        let part_of = app.world_mut().spawn_empty().id();
        let mut arming = TorpedoArming::new(0.5, 5.0, Vec3::ZERO);
        arming.tick(1.0, Vec3::ZERO);
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

        assert!(!app.world().entities().contains(torpedo));
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
    fn torpedo_homes_on_the_live_structure_anchor() {
        // The root origin is the build spot of the ship's first sections; once
        // those die the live structure sits somewhere else entirely, and a torpedo
        // aimed at the origin has to reach within its fuze radius of empty space.
        let mut app = App::new();
        app.add_systems(Update, update_target_position);

        let target = app
            .world_mut()
            .spawn((
                Transform::from_translation(Vec3::new(0.0, 0.0, -100.0)),
                ComputedCenterOfMass(Vec3::new(0.0, 0.0, -40.0)),
            ))
            .id();
        let torpedo = app
            .world_mut()
            .spawn((TorpedoProjectileMarker, TorpedoTargetEntity(target)))
            .id();

        app.update();

        assert_eq!(
            **app.world().get::<TorpedoTargetPosition>(torpedo).unwrap(),
            Vec3::new(0.0, 0.0, -140.0),
            "the torpedo should home on the COM-lifted anchor, not the root origin"
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
    #[expect(
        clippy::too_many_arguments,
        reason = "one system steering the whole projectile"
    )]
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
        // the turning circle (speed / turn rate) tight enough that the torpedo
        // cannot end up orbiting its target instead of closing on it.
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

    /// A closest approach that counts as a solved intercept, in units.
    ///
    /// A bound on the GUIDANCE and no longer on the fuze: the rig flies at a
    /// point, and the real fuze fires on a body's skin, so what a real ship's
    /// hull covers of this is the ship's own size. Crossing intercepts from a
    /// sideways launch carry a few units of turn-rate lag at the endgame
    /// (measured ~8); a broken law misses by the full crossing distance instead.
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

    /// The launch rig again, with the terminal weave laid over the guidance
    /// command exactly as `torpedo_terminal_weave` lays it over the real one.
    fn weaving_launch_closest_approach(target: Vec3, target_vel: Vec3, angle: f32) -> f32 {
        use std::cell::Cell;

        // The weave offset is per-torpedo state; a Cell keeps the steer closure
        // `Fn` while still carrying it between steps, as the component does.
        let offset = Cell::new(Vec3::Y);
        let dt = 0.02;
        simulate_thrust_intercept(
            Vec3::ZERO,
            Vec3::new(0.0, 1.0, 0.0),
            Vec3::NEG_Z,
            target,
            target_vel,
            3.0,
            25.0,
            35.0,
            0.8,
            dt,
            500,
            |rel_pos, tv, v| {
                let steer = pn_steer_direction(rel_pos, tv, v, 3.0);
                let faded = angle * weave_fade(rel_pos.length(), 30.0);
                let (command, next) = weave_steer_direction(
                    steer,
                    offset.get(),
                    faded,
                    TorpedoTypeConfig::default().weave_rate * dt,
                );
                offset.set(next);
                command
            },
        )
    }

    #[test]
    fn the_weave_perturbs_the_guidance_command_instead_of_replacing_it() {
        // The whole design rests on this: the weaved command sits EXACTLY
        // `angle` off the guidance solution, so every property the guidance law
        // has survives, and a zero angle is the solution untouched.
        let steer = Vec3::new(0.0, 0.0, -1.0);
        let (unweaved, _) = weave_steer_direction(steer, Vec3::Y, 0.0, 0.0);
        assert!(
            (unweaved - steer).length() < 1e-5,
            "zero amplitude must leave the guidance command alone, got {unweaved:?}"
        );

        let (weaved, _) = weave_steer_direction(steer, Vec3::Y, 0.44, 0.0);
        assert!(weaved.is_normalized());
        assert!(
            (weaved.angle_between(steer) - 0.44).abs() < 1e-4,
            "the command must be tilted by exactly the amplitude, got {} rad",
            weaved.angle_between(steer)
        );
        assert!(
            weaved.dot(steer) > 0.9,
            "and still point down the intercept, got {weaved:?}"
        );
    }

    #[test]
    fn the_weave_spins_around_the_command_and_stays_perpendicular() {
        // A corkscrew, not a wobble: the tilt direction sweeps a full turn
        // about the command, and stays a unit vector perpendicular to it even
        // as the command itself turns.
        let mut offset = Vec3::Y;
        let mut tilts = Vec::new();
        for step in 0..64 {
            // Swing the command around as a closing torpedo's would.
            let steer = Quat::from_rotation_y(step as f32 * 0.02) * Vec3::NEG_Z;
            let (command, next) = weave_steer_direction(
                steer,
                offset,
                0.44,
                TorpedoTypeConfig::default().weave_rate / 30.0,
            );
            offset = next;
            assert!(offset.is_normalized(), "the tilt direction stays unit");
            assert!(
                offset.dot(steer.normalize()).abs() < 1e-3,
                "and perpendicular to the command it tilts off: {}",
                offset.dot(steer.normalize())
            );
            tilts.push((command - steer.normalize() * command.dot(steer.normalize())).normalize());
        }
        // A full turn: some sampled tilt points the opposite way to the first.
        assert!(
            tilts.iter().any(|tilt| tilt.dot(tilts[0]) < -0.9),
            "the tilt must sweep all the way around the command"
        );
    }

    #[test]
    fn the_weave_fades_out_on_final_approach() {
        // Full weave on the run-in, nothing left at the fuze - the difference
        // between evasive ordnance and one that corkscrews past a target that
        // never moved.
        assert_eq!(weave_fade(400.0, 30.0), 1.0, "full weave at range");
        assert_eq!(weave_fade(90.0, 30.0), 1.0, "full weave at three radii");
        assert_eq!(weave_fade(15.0, 30.0), 0.0, "none at the terminal band");
        assert_eq!(weave_fade(0.0, 30.0), 0.0, "and none on top of the target");
        let mid = weave_fade(52.5, 30.0);
        assert!(
            (mid - 0.5).abs() < 1e-5,
            "linear across the band, got {mid}"
        );
        assert!(
            weave_fade(10.0, 0.0).is_finite(),
            "a degenerate blast radius must not divide by zero"
        );
    }

    #[test]
    fn a_weaving_torpedo_still_hits_a_non_manoeuvring_target() {
        // THE trap the design calls out: a weave that survives into the endgame
        // flies its own helix past a stationary target. Same rig as
        // `pn_turns_a_sideways_launch_onto_a_stationary_target`, weave on.
        for angle in [0.44f32, 0.7] {
            let miss =
                weaving_launch_closest_approach(Vec3::new(0.0, 0.0, -160.0), Vec3::ZERO, angle);
            assert!(
                miss < 15.0,
                "a weaving torpedo must still reach the proximity fuze of a \
                 stationary target at amplitude {angle}, closest was {miss}"
            );
        }
    }

    #[test]
    fn a_weaving_torpedo_still_intercepts_a_crossing_target() {
        // And the weave must not cost the lead solution either.
        let miss = weaving_launch_closest_approach(
            Vec3::new(-30.0, 0.0, -120.0),
            Vec3::new(15.0, 0.0, 0.0),
            0.44,
        );
        assert!(
            miss < 15.0,
            "a weaving torpedo must still intercept a crossing target, closest was {miss}"
        );
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

/// What the terminal weave is FOR: the price in point-defense rounds.
///
/// The weave's value is not that it looks evasive, it is that a salvo costs the
/// defender ammunition whether or not it connects. That is a NUMBER, and this
/// module measures it: rounds a mount spends on one straight torpedo versus one
/// weaving torpedo, over the same closing geometry.
///
/// The model is small but it is wired from the production pieces on both sides.
/// The torpedo flies `pn_steer_direction` perturbed by `weave_steer_direction` /
/// `weave_fade` - the same three functions the real projectile systems call -
/// through the turn/thrust/drag model the guidance tests already use. The mount
/// fires on `lead_intercept_point`, the exact solve `update_turret_aim_point`
/// hands every AI turret, under the real [`muzzle_on_target`] gate, at the
/// shipped PDC's rate, speed and damage, inside the shipped point-defense
/// envelope. What it does NOT model is avian, the body's attitude controller,
/// or more than one mount; the absolute round counts are therefore indicative,
/// and the RATIO between the two arms is the result. The amplitude the real
/// body achieves is measured separately and in-engine, by
/// `the_weave_puts_a_visible_bend_in_the_real_flight_path`.
#[cfg(test)]
mod point_defense_cost_tests {
    use super::*;
    use crate::sections::turret_section::{lead_intercept_point, muzzle_on_target};

    /// Default turret `muzzle_speed`.
    const BULLET_SPEED: f32 = 100.0;
    /// Default muzzle `fire_rate`, rounds per second.
    const FIRE_RATE: f32 = 100.0;
    /// The shipped player PDC's authored `bullet_damage`.
    const BULLET_DAMAGE: f32 = 4.0;
    /// Default joint traverse speed (`default_joint_speed`), rad/s.
    const SLEW_RATE: f32 = std::f32::consts::PI;
    /// `AI_POINT_DEFENSE_RANGE`: outside it the guns are on something else.
    const ENGAGE_RANGE: f32 = 150.0;
    /// Half-width of the torpedo's two unit-cube section colliders: a round
    /// passing further than this from the axis is a miss.
    const HIT_RADIUS: f32 = 0.5;
    /// `default_projectile_health` on each of the torpedo's two sections;
    /// either reaching zero kills it.
    const TORPEDO_HEALTH: f32 = 10.0;

    /// The outcome of one closing run.
    #[derive(Debug)]
    struct DefenseRun {
        /// Rounds the mount put in the air before the run ended. Includes the
        /// rounds still in flight at that moment, which is the honest count: a
        /// defender cannot un-fire what it has already committed, and both arms
        /// pay the same pipeline.
        rounds: usize,
        /// Rounds that landed on the torpedo.
        hits: usize,
        /// Seconds the torpedo survived inside the envelope.
        seconds: f32,
        /// Widest the torpedo swung off the straight run-in while the weave was
        /// at full amplitude - the helix radius the flight path actually
        /// achieves once the body's drag and turn rate have had their say. The
        /// VISIBLE amplitude: a weave the player cannot see is not evasion, it
        /// is a rounding error with a good test.
        swing: f32,
        /// Range left to the defender when the run ended.
        range: f32,
        /// Whether the torpedo died before reaching its fuze.
        killed: bool,
    }

    /// One torpedo runs in from [`ENGAGE_RANGE`] at cruise against one mount at
    /// the origin. `weave_angle` of zero flies the bare intercept.
    ///
    /// `max_speed` is a parameter because the two shipped types no longer share
    /// one: the evasive type buys its evasion with a lower cruise cap, and a
    /// rig that assumed 35 everywhere would score it as the old, faster round.
    fn defend(torpedo_type: &TorpedoTypeConfig) -> DefenseRun {
        let TorpedoTypeConfig {
            max_speed,
            weave_angle,
            weave_rate,
            ..
        } = *torpedo_type;
        // The whole run below is engine-side kinematics against avian numbers,
        // so the authored cruise cap crosses once, here.
        let max_speed = max_speed.to_engine();
        let dt = 1.0 / 120.0;
        let interval = 1.0 / FIRE_RATE;
        let blast_radius = 30.0;
        // The mount is modelled as a point, so its skin is the origin and the
        // contact fuze is the whole standoff. The run-in is that much longer
        // than it was under the old half-a-blast-radius fuze, and both arms pay
        // it: the defender gets the extra tenths of a second on every torpedo.
        let contact = CONTACT_FUZE;
        let needed = (TORPEDO_HEALTH / BULLET_DAMAGE).ceil() as usize;

        // The torpedo enters the envelope already at cruise and pointed in -
        // the run-in, not the launch (the launch turn is covered by the
        // guidance tests, and it happens outside any defender's reach).
        let mut pos = Vec3::new(0.0, 0.0, -ENGAGE_RANGE);
        let mut vel = Vec3::Z * max_speed;
        let mut nose = Vec3::Z;
        let mut offset = Vec3::Y;

        // The mount starts on the bearing: this measures the lead solution, not
        // a cold-start slew race.
        let mut barrel = Vec3::NEG_Z;
        let mut fire_timer = 0.0f32;
        // (position, velocity, age)
        let mut bullets: Vec<(Vec3, Vec3, f32)> = Vec::new();

        let mut rounds = 0usize;
        let mut hits = 0usize;
        // The run-in is along +Z to the origin, so the lateral offset off the
        // straight course is simply the distance from the Z axis.
        let mut swing = 0.0f32;

        let mut elapsed = 0.0f32;
        while elapsed < 30.0 {
            elapsed += dt;
            let previous = pos;

            // --- the torpedo, on the production guidance chain ---
            let rel = Vec3::ZERO - pos;
            let mut steer = pn_steer_direction(rel, Vec3::ZERO, vel, 3.0);
            if weave_angle > 0.0 {
                let angle = weave_angle * weave_fade(rel.length(), blast_radius);
                let (command, next) = weave_steer_direction(steer, offset, angle, weave_rate * dt);
                steer = command;
                offset = next;
            }
            let turn = nose.angle_between(steer);
            let axis = nose.cross(steer);
            if axis.length() > 1e-6 && turn > 1e-6 {
                nose = (Quat::from_axis_angle(axis.normalize(), (3.0 * dt).min(turn)) * nose)
                    .normalize();
            }
            let thrust =
                nose.dot(steer).clamp(0.0, 1.0) * thrust_headroom(vel.dot(nose), max_speed);
            vel += nose * 25.0 * thrust * dt;
            vel -= vel * 0.8 * dt;
            pos += vel * dt;
            let torpedo_step_velocity = (pos - previous) / dt;
            if weave_fade(rel.length(), blast_radius) >= 1.0 {
                swing = swing.max(pos.xy().length());
            }

            if pos.length() <= contact {
                break; // leaked through: the warhead is on the defender
            }

            // --- the mount: slew onto the lead solution, fire when aligned ---
            let aim = lead_intercept_point(Vec3::ZERO, pos, torpedo_step_velocity, BULLET_SPEED);
            let desired = aim.normalize();
            let error = barrel.angle_between(desired);
            let slew_axis = barrel.cross(desired);
            if slew_axis.length() > 1e-6 && error > 1e-6 {
                barrel =
                    (Quat::from_axis_angle(slew_axis.normalize(), (SLEW_RATE * dt).min(error))
                        * barrel)
                        .normalize();
            }
            fire_timer += dt;
            let in_envelope = pos.length() <= ENGAGE_RANGE;
            while fire_timer >= interval {
                fire_timer -= interval;
                // The mount sits at the origin, so the barrel's aim point is
                // `aim` itself - the same predicate the section fire path runs.
                if !in_envelope || !muzzle_on_target(barrel, Vec3::ZERO, aim) {
                    continue;
                }
                // Sub-tick lead, as `shoot_spawn_projectile` gives the stream:
                // a round due mid-step starts where it would already be.
                let velocity = barrel * BULLET_SPEED;
                bullets.push((velocity * fire_timer, velocity, fire_timer));
                rounds += 1;
            }

            // --- rounds in flight, against the torpedo's swept step ---
            bullets.retain_mut(|(bullet, velocity, age)| {
                let relative_position = *bullet - previous;
                let relative_velocity = *velocity - torpedo_step_velocity;
                let closest = (-relative_position.dot(relative_velocity)
                    / relative_velocity.length_squared().max(f32::EPSILON))
                .clamp(0.0, dt);
                let miss = (relative_position + relative_velocity * closest).length();
                *bullet += *velocity * dt;
                *age += dt;
                if miss < HIT_RADIUS {
                    hits += 1;
                    return false;
                }
                // The shipped PDC's `projectile_lifetime`.
                *age < 2.0
            });

            if hits >= needed {
                return DefenseRun {
                    rounds,
                    hits,
                    seconds: elapsed,
                    swing,
                    range: pos.length(),
                    killed: true,
                };
            }
        }

        DefenseRun {
            rounds,
            hits,
            seconds: elapsed,
            swing,
            range: pos.length(),
            killed: false,
        }
    }

    /// The straight-running type, as `sections::ordnance::lance` authors it:
    /// no weave, and the reference cruise the evasive type is slowed against.
    fn straight_type() -> TorpedoTypeConfig {
        TorpedoTypeConfig {
            name: "Straight".to_string(),
            max_speed: MetersPerSecond(350.0),
            weave_angle: 0.0,
            weave_rate: 0.0,
            ..Default::default()
        }
    }

    #[test]
    fn a_weaving_torpedo_costs_the_defender_more_rounds_than_a_straight_one() {
        let straight = defend(&straight_type());
        let weaving = defend(&TorpedoTypeConfig::default());
        println!("point-defense cost: straight {straight:?}, weaving {weaving:?}");

        // Control first: a straight torpedo IS stopped, cheaply. Without this
        // the comparison could be measuring a broken mount on both arms.
        assert!(
            straight.killed,
            "a straight torpedo must still be intercepted: {straight:?}"
        );

        // The result. A weaving torpedo either survives the envelope outright
        // or costs several times the ammunition to stop.
        let ratio = weaving.rounds as f32 / straight.rounds.max(1) as f32;
        assert!(
            ratio >= 2.0,
            "the weave must make an intercept meaningfully more expensive: \
             straight {straight:?} vs weaving {weaving:?} (ratio {ratio:.1})"
        );
        // Both arms stop at the same hit count by construction, so the share of
        // FIRED rounds that had landed by then is the accuracy statement.
        let straight_share = straight.hits as f32 / straight.rounds.max(1) as f32;
        let weaving_share = weaving.hits as f32 / weaving.rounds.max(1) as f32;
        assert!(
            weaving_share < straight_share,
            "and the mount must land a smaller share of what it fires: \
             {straight_share:.4} straight vs {weaving_share:.4} weaving"
        );
        // The self-balancing half of the design, measured rather than asserted
        // in prose: the corkscrew is a LONGER path, so the same closing run
        // takes more seconds - the torpedo buys its survivability with exposure
        // time, at no tuning cost.
        assert!(
            weaving.seconds > straight.seconds,
            "a weaving torpedo must spend longer inside the envelope: \
             {:.1}s straight vs {:.1}s weaving",
            straight.seconds,
            weaving.seconds
        );
        // And the consequence a player can watch: the weaving round gets much
        // further in before the guns finally connect.
        assert!(
            weaving.range < straight.range,
            "a weaving torpedo must survive deeper into the envelope: \
             died at {:.0} u straight vs {:.0} u weaving",
            straight.range,
            weaving.range
        );

        // The two ends of "evasive, not drunk", as a number. The floor: a weave
        // the player cannot see is not evasion, it is a rounding error with a
        // passing test - so the path has to swing wider than the torpedo is
        // long. The ceiling: the helix has to fit inside the proximity fuze
        // (half a blast radius), or a torpedo would corkscrew past a target
        // that never moved even before the terminal fade helped it.
        assert!(
            (2.0..15.0).contains(&weaving.swing),
            "the weave must be visible but stay inside the fuze: swing {:.1} u",
            weaving.swing
        );
    }

    #[test]
    fn a_wider_weave_costs_the_defender_more_still() {
        // Monotone in amplitude, which is what makes the knob a tuning dial
        // rather than a coin flip.
        // Amplitude only: same cruise on both arms, or this would be measuring
        // the speed the shipped evasive type also authors.
        let narrow = defend(&TorpedoTypeConfig {
            weave_angle: 0.15,
            ..TorpedoTypeConfig::default()
        });
        let wide = defend(&TorpedoTypeConfig::default());
        println!("amplitude sweep: narrow {narrow:?}, wide {wide:?}");
        let narrow_cost = if narrow.killed {
            narrow.rounds as f32
        } else {
            f32::INFINITY
        };
        let wide_cost = if wide.killed {
            wide.rounds as f32
        } else {
            f32::INFINITY
        };
        assert!(
            wide_cost >= narrow_cost,
            "a wider weave must not be CHEAPER to stop: {narrow:?} vs {wide:?}"
        );
    }
}
