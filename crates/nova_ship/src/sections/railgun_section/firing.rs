//! The lance's cycle: arming a spawned section, the charge clock, the shot,
//! and the recoil the shot puts back through the hull.

use super::*;

/// Seed a spawned lance with its magazine.
///
/// The reload rides on the magazine, exactly as it does for the turret and the
/// bay: a lance with no `ammo_capacity` is unlimited and never reloads, which
/// is what a bare headless rig wants.
pub(super) fn insert_railgun_section(
    add: On<Add, RailgunSectionMarker>,
    mut commands: Commands,
    q_section: Query<&RailgunSectionConfigHelper, With<RailgunSectionMarker>>,
) {
    let entity = add.entity;
    trace!("insert_railgun_section: entity {:?}", entity);

    let Ok(config) = q_section.get(entity) else {
        error!(
            "insert_railgun_section: entity {:?} not found in q_section",
            entity
        );
        return;
    };

    let Some(capacity) = config.ammo_capacity else {
        return;
    };
    commands.entity(entity).insert(SectionAmmo::new(capacity));
    if let Some(reload) = config.reload {
        commands
            .entity(entity)
            .insert(SectionReload::from_config(reload));
    }
}

/// Run every lance's cycle for one fixed tick: commit on a trigger, advance a
/// running charge, and fire when it completes.
///
/// FIXED CLOCK, and the whole cycle in one system, because the shot and its
/// recoil have to land in the same tick: an impulse applied a tick after the
/// slug left would push a ship that had already moved on from the pose it
/// fired in.
///
/// Ordering against the trigger: the section reads the input written by the
/// player rig, the AI, or a test, and never clears it. A held trigger is
/// therefore a gun cycling at its own cadence rather than a second mechanic.
#[expect(
    clippy::too_many_arguments,
    reason = "the whole cycle is one system so the shot and its recoil share a tick"
)]
pub(super) fn charge_and_fire_railgun(
    mut commands: Commands,
    time: Res<Time>,
    mut q_railgun: Query<
        (
            Entity,
            &ChildOf,
            &RailgunSectionConfigHelper,
            &RailgunSectionInput,
            &mut RailgunCharge,
            Option<&mut SectionAmmo>,
            Option<&mut SectionReload>,
            Option<&mut SectionAnimations>,
        ),
        (With<RailgunSectionMarker>, Without<SectionInactiveMarker>),
    >,
    mut q_root: Query<Forces, With<SpaceshipRootMarker>>,
    q_root_extras: Query<(&ComputedCenterOfMass, Option<&Allegiance>), With<SpaceshipRootMarker>>,
    q_chain: Query<(&Transform, &ChildOf)>,
    q_hot: Query<&WeaponsHot>,
) {
    let dt = time.delta_secs();
    for (
        railgun,
        &ChildOf(spaceship),
        config,
        input,
        mut charge,
        mut ammo,
        mut reload,
        mut animations,
    ) in &mut q_railgun
    {
        // The safety is LIVE, like every other weapon: a gun that goes safe
        // mid-charge holds its shell rather than firing it into a friendly.
        // Ships with no `WeaponsHot` are unmanaged and fire freely (bare rigs).
        let hot = q_hot.get(spaceship).map_or(true, |hot| hot.0);

        match *charge {
            RailgunCharge::Ready => {
                if !hot || !**input {
                    continue;
                }
                // Committing on an EMPTY gun would burn a charge for nothing
                // and hide the reload from the player, so the magazine gates
                // the commit rather than the shot.
                if ammo.as_deref().is_some_and(SectionAmmo::is_empty) {
                    continue;
                }
                *charge = RailgunCharge::Charging { elapsed: 0.0 };
            }
            RailgunCharge::Charging { elapsed } => {
                if !hot {
                    // Safed mid-charge: dump the charge, keep the shell.
                    *charge = RailgunCharge::Ready;
                    if let Some(animations) = animations.as_deref_mut() {
                        animations.snap_cue(SectionAnimationCue::Charge, 0.0);
                    }
                    continue;
                }
                *charge = RailgunCharge::Charging {
                    elapsed: elapsed + dt,
                };
            }
        }

        // The bolt walks the bore on the GAMEPLAY clock, written straight onto
        // the track: the authored charge time is the one clock, and a track
        // travelling at its own speed would promise a shot that had not
        // arrived. See `SectionAnimationCue::Charge`.
        let progress = charge.progress(config.charge_seconds);
        if let Some(animations) = animations.as_deref_mut() {
            animations.snap_cue(SectionAnimationCue::Charge, progress);
        }
        if progress < 1.0 {
            continue;
        }

        let Ok(mut forces) = q_root.get_mut(spaceship) else {
            error!(
                "charge_and_fire_railgun: entity {:?} not found in q_root",
                spaceship
            );
            *charge = RailgunCharge::Ready;
            continue;
        };
        let Ok((center, allegiance)) = q_root_extras.get(spaceship) else {
            error!(
                "charge_and_fire_railgun: entity {:?} not found in q_root_extras",
                spaceship
            );
            *charge = RailgunCharge::Ready;
            continue;
        };

        // The bore on the RAW physics clock, like every other fixed-tick
        // spawner: inside FixedUpdate `GlobalTransform` still holds the
        // previous frame's EASED render pose, which at speed trails the hull.
        let Some((section_local_pos, section_local_rot)) =
            local_pose_in_root(railgun, spaceship, &q_chain)
        else {
            error!(
                "charge_and_fire_railgun: railgun {:?} is not a descendant of ship {:?}",
                railgun, spaceship
            );
            *charge = RailgunCharge::Ready;
            continue;
        };
        let position = forces.position().0;
        let rotation = *forces.rotation();
        let bore_rotation = rotation.0 * section_local_rot;
        let muzzle_position = position
            + rotation.mul_vec3(section_local_pos + section_local_rot * config.muzzle_offset);
        // -Z in the section's frame IS the ship's line of fire: a lance cannot
        // traverse, so this is the whole of its aim.
        let bore_direction = bore_rotation * Vec3::NEG_Z;

        // Spend the shell only now that the shot is certain.
        if let Some(ammo) = ammo.as_deref_mut() {
            if !ammo.try_consume() {
                *charge = RailgunCharge::Ready;
                continue;
            }
            if let Some(reload) = reload.as_deref_mut() {
                reload.on_shot();
            }
        }

        // The slug inherits the muzzle's full motion, swing included: a lance
        // mounted off the centre of mass of a rolling ship throws its shell
        // off the tangent as well as along the bore.
        let center_of_mass = position + rotation.mul_vec3(**center);
        let muzzle_velocity = rigid_body_point_velocity(
            forces.linear_velocity(),
            forces.angular_velocity(),
            center_of_mass,
            muzzle_position,
        );
        let slug_velocity = bore_direction * config.slug_speed + muzzle_velocity;

        let mut slug = commands.spawn((
            Name::new("Railgun Slug"),
            RailgunSlugProjectileMarker,
            ProjectileOwner(spaceship),
            Transform::from_translation(muzzle_position)
                .with_rotation(Quat::from_rotation_arc(Vec3::NEG_Z, bore_direction)),
            RoundVelocity(slug_velocity),
            // POWER IS THE ONLY BOUND. `layers: u32::MAX` is the owner's call
            // made structural: a lance stops when it runs out of thickness to
            // spend, never because it met a layer count.
            ProjectileDamage {
                amount: config.slug_damage,
                power: config.slug_power,
                layers: u32::MAX,
                kind: DamageType::Pierce,
            },
            TempEntity(config.slug_lifetime),
            Visibility::Visible,
        ));
        // Copied, not resolved through `ProjectileOwner`: the slug stays
        // attributable if the gun that fired it dies mid-flight.
        if let Some(&allegiance) = allegiance {
            slug.insert(allegiance);
        }

        // RECOIL, at the MUZZLE and not at the centre of mass. That is the
        // whole mechanic: `apply_linear_impulse_at_point` derives the torque
        // from the lever arm, so a lance bolted off the ship's axis spins the
        // ship every time it fires, and where the builder put it is part of
        // what it costs. Raw impulse, no `dt` - the same units the thruster's
        // magnitude carries.
        if config.recoil_impulse > 0.0 {
            forces.apply_linear_impulse_at_point(
                -bore_direction * config.recoil_impulse,
                muzzle_position,
            );
        }

        *charge = RailgunCharge::Ready;
        if let Some(animations) = animations.as_deref_mut() {
            animations.snap_cue(SectionAnimationCue::Charge, 0.0);
        }
        commands.trigger(RailgunFired {
            entity: railgun,
            muzzle: muzzle_position,
        });
    }
}

/// A lance fired. The seam the render and audio halves hang the muzzle flash
/// and the report on, so the fire path itself stays headless.
#[derive(EntityEvent, Clone, Copy, Debug)]
pub struct RailgunFired {
    /// The gun that fired.
    pub entity: Entity,
    /// World-space bore exit the shot left from.
    pub muzzle: Vec3,
}

/// The lance's authored report, snapshotted onto the section at spawn.
///
/// AUTHORED-OR-SILENT, the seam every weapon sound in the game uses: content
/// owns the sound, and a lance that authors none fires silently.
#[derive(Component, Clone, Debug, Reflect)]
pub struct RailgunSectionFireSound(#[reflect(ignore)] pub Option<AssetRef<AudioSource>>);

/// The lance's authored capacitor loop, snapshotted onto the section at spawn.
/// Read by the audio layer's charge loop, which opens one voice per charging
/// lance and retires it on the shot.
#[derive(Component, Clone, Debug, Reflect)]
pub struct RailgunSectionChargeSound(#[reflect(ignore)] pub Option<AssetRef<AudioSource>>);

/// The lance's authored breech cycle, snapshotted onto the section at spawn.
/// Played when [`SectionReloadComplete`] lands on this gun.
///
/// [`SectionReloadComplete`]: crate::sections::prelude::SectionReloadComplete
#[derive(Component, Clone, Debug, Reflect)]
pub struct RailgunSectionReloadSound(#[reflect(ignore)] pub Option<AssetRef<AudioSource>>);
