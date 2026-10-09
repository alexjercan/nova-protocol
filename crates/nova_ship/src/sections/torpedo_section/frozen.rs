//! Save/restore of a torpedo in flight: [`freeze_torpedo`] reads a live
//! projectile into a [`FrozenTorpedo`] and [`thaw_torpedo`] spawns one back
//! through [`spawn_torpedo`] - the same bay-launch bundle a live shot uses,
//! so a resumed torpedo is built the one way the section knows how to build
//! one.
//!
//! What a thaw does NOT do, because a thaw is not a launch: it never
//! triggers [`TorpedoLaunched`] (the launch cues answer that event, and a
//! resumed torpedo was not just fired) and it never inserts
//! [`TorpedoTargetEntity`]. The entity a [`SavedTorpedoTarget::Tracking`]
//! points at may not exist yet on the frame a torpedo thaws (every saved
//! transient and body of the window spawns across the same restore pass),
//! so the save collector resolves `target` and inserts
//! `TorpedoTargetEntity` itself once every transient has landed - the
//! two-phase restore [`SavedTargetRef`] exists for.

use super::*;

/// A torpedo in flight, as a save keeps it.
///
/// `config` is carried whole rather than looked up through `section`: a
/// torpedo outlives the bay that fired it (see [`TorpedoLaunchConfig`]'s own
/// doc), so a dead bay at freeze time would otherwise leave nothing to
/// rebuild the warhead's guidance, blast and child-section durability from.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct FrozenTorpedo {
    /// The bay config this torpedo was launched under.
    pub config: TorpedoSectionConfig,
    /// The ship that fired it, or [`SavedOwner::Gone`].
    pub owner: SavedOwner,
    /// The launching bay, by (ship id, section id). `None` when the bay had
    /// already died by freeze time - the live state a dead
    /// [`TorpedoSectionPartOf`] already degrades to.
    pub section: Option<SavedSectionRef>,
    /// Where the torpedo was.
    pub translation: Vec3,
    /// Which way it was facing.
    pub rotation: Quat,
    /// Its linear velocity.
    pub linear: Vec3,
    /// Its angular velocity.
    pub angular: Vec3,
    /// The shooter's copied `Allegiance`, if it carried one.
    pub allegiance: Option<Allegiance>,
    /// Its arming state.
    pub arming: TorpedoArming,
    /// Its cold-launch countdown, or `None` once it had ignited.
    pub cold: Option<TorpedoColdLaunch>,
    /// Its launch-time targeting decision.
    pub target: SavedTorpedoTarget,
    /// Its current steering command.
    pub steering: Vec3,
    /// Its terminal-weave state.
    pub weave: TorpedoWeave,
    /// Whether its drive had already lit (`cold.is_none()`) - kept
    /// alongside `cold` rather than derived from it, so a reader of the
    /// record does not have to know that rule to ask the question.
    pub ignited: bool,
    /// The `Health.current` of its [`TorpedoControllerMarker`] child at
    /// freeze time.
    pub controller_health: f32,
    /// The `Health.current` of its [`TorpedoThrusterMarker`] child at
    /// freeze time.
    pub thruster_health: f32,
}

/// A live body a saved torpedo was tracking, by the key the save already has
/// for its kind, and the live meanings [`update_target_position`] gives each.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum SavedTorpedoTarget {
    /// No targeting system had processed this torpedo yet: no
    /// [`TorpedoTargetChosen`] at all. Lasts at most the one frame between
    /// launch and the commit system's next pass.
    Unchosen,
    /// The targeting system ran and found no lock: [`TorpedoTargetChosen`]
    /// alone. The torpedo flies straight for life.
    DumbFire,
    /// A live target was locked and is (or was, pending its own restore)
    /// resolvable. `last` is the most recent [`TorpedoTargetPosition`], if
    /// [`update_target_position`] had run at least once before the save.
    Tracking {
        /// The tracked body, by its saved key.
        target: SavedTargetRef,
        /// The last known position, if one was ever set.
        last: Option<Vec3>,
    },
    /// The target died before the save: the entity link is gone, or still
    /// names the despawned target because [`update_target_position`] has
    /// not run since. Only the frozen position remains.
    Frozen(Vec3),
}

/// A live bay section's [`SavedSectionRef`], or `None` for a dead one.
///
/// # Errors
///
/// [`TransientFreezeFault::NoDurableId`] when the section is alive but
/// carries no [`EntityId`] - every spawned section gets one, so this is a
/// programming error elsewhere, not a normal save-time condition, and the
/// fault surfaces it as a visible failed save rather than a silent `None`.
fn saved_section_ref(
    world: &World,
    section: Entity,
) -> Result<Option<SavedSectionRef>, TransientFreezeFault> {
    let Ok(body) = world.get_entity(section) else {
        return Ok(None);
    };
    let section_id = body
        .get::<EntityId>()
        .ok_or_else(|| TransientFreezeFault::NoDurableId {
            label: format!("torpedo bay {section}"),
        })?;
    let &ChildOf(ship) = body.get::<ChildOf>().unwrap_or_else(|| {
        panic!("saved_section_ref: bay {section} carries no ChildOf; every bay is a direct child of its ship")
    });
    let ship_id = world
        .get::<EntityId>(ship)
        .ok_or_else(|| TransientFreezeFault::NoDurableId {
            label: format!("ship {ship}"),
        })?;
    Ok(Some(SavedSectionRef {
        ship: ship_id.clone(),
        section: section_id.clone(),
    }))
}

/// The `Health.current` of `torpedo`'s direct child marked `M`
/// ([`TorpedoControllerMarker`] or [`TorpedoThrusterMarker`]), named `label`
/// in a panic message.
///
/// # Errors
///
/// [`TransientFreezeFault::Unsettled`] while the part is at zero or carries
/// [`HealthZeroMarker`]. `on_damage` zeroes the pool at once, and the torpedo
/// gets its [`TorpedoShotDownMarker`] two flushes later. A record saved in
/// between could never load, because a Load refuses a part at zero.
///
/// # Panics
///
/// If `torpedo` has no children, no child marked `M`, or that child carries
/// no [`Health`] - [`spawn_torpedo`] always gives both marked children one.
fn part_health<M: Component>(
    world: &World,
    torpedo: Entity,
    label: &str,
) -> Result<f32, TransientFreezeFault> {
    let children = world.get::<Children>(torpedo).unwrap_or_else(|| {
        panic!("freeze_torpedo: torpedo {torpedo} has no children; every launched torpedo spawns a controller and a thruster")
    });
    let part = children
        .iter()
        .find(|&child| world.get::<M>(child).is_some())
        .unwrap_or_else(|| panic!("freeze_torpedo: torpedo {torpedo} carries no {label}"));
    let health = world
        .get::<Health>(part)
        .unwrap_or_else(|| panic!("freeze_torpedo: torpedo {torpedo}'s {label} carries no Health"))
        .current;
    if health <= 0.0 || world.get::<HealthZeroMarker>(part).is_some() {
        return Err(TransientFreezeFault::Unsettled(UnsettledBody {
            reason: "a torpedo part is being destroyed",
        }));
    }
    Ok(health)
}

/// The torpedo `entity` is flying, as a value a save can keep.
///
/// `target` resolves a live [`TorpedoTargetEntity`] to its saved key (a
/// ship/rock/wreck id, a canister runtime id, or another saved transient's
/// index - see [`SavedTargetRef`]); the caller owns that resolution because
/// it depends on the whole save's transient list, not on this one torpedo.
/// It returns `None` for a despawned target. The freeze then saves the state
/// [`update_target_position`] leaves when it drops that link: frozen on the
/// last known position, or dumb-fire with none. A save cannot wait for that
/// pass, because the pause gate stops it while a leave save settles.
///
/// # Errors
///
/// [`TransientFreezeFault::Unsettled`] for a torpedo whose body was just shot
/// down ([`TorpedoShotDownMarker`]) and is awaiting removal - the kill is
/// already queued, so a snapshot taken here would save a torpedo that is
/// about to not exist. The same while a part is being destroyed (see
/// [`part_health`]). Otherwise, whatever [`SavedOwner::of`],
/// [`saved_section_ref`] or `target` returns.
///
/// # Panics
///
/// If `entity` carries no [`ProjectileOwner`], [`TorpedoSectionPartOf`],
/// [`Transform`], [`LinearVelocity`], [`TorpedoLaunchConfig`],
/// [`TorpedoArming`], [`TorpedoWeave`] or [`TorpedoSteering`] - every
/// launched torpedo spawns with all seven, so a live one missing any is a
/// bare test spawn, not a save-time condition. Also if `entity` has no
/// [`TorpedoControllerMarker`] or [`TorpedoThrusterMarker`] child, or either
/// child carries no [`Health`] - [`spawn_torpedo`] always gives it both.
pub fn freeze_torpedo(
    world: &World,
    entity: Entity,
    target: impl Fn(Entity) -> Result<Option<SavedTargetRef>, TransientFreezeFault>,
) -> Result<FrozenTorpedo, TransientFreezeFault> {
    if world.get::<TorpedoShotDownMarker>(entity).is_some() {
        return Err(TransientFreezeFault::Unsettled(UnsettledBody {
            reason: "a shot-down torpedo is awaiting removal",
        }));
    }

    let &ProjectileOwner(owner_entity) =
        world.get::<ProjectileOwner>(entity).unwrap_or_else(|| {
            panic!(
                "freeze_torpedo: torpedo {entity} carries no ProjectileOwner; every launched \
             torpedo spawns with one"
            )
        });
    let owner = SavedOwner::of(world, owner_entity)?;

    let &TorpedoSectionPartOf(part_of) =
        world
            .get::<TorpedoSectionPartOf>(entity)
            .unwrap_or_else(|| {
                panic!("freeze_torpedo: torpedo {entity} carries no TorpedoSectionPartOf")
            });
    let section = saved_section_ref(world, part_of)?;

    let transform = world
        .get::<Transform>(entity)
        .copied()
        .unwrap_or_else(|| panic!("freeze_torpedo: torpedo {entity} carries no Transform"));
    let &LinearVelocity(linear) = world
        .get::<LinearVelocity>(entity)
        .unwrap_or_else(|| panic!("freeze_torpedo: torpedo {entity} carries no LinearVelocity"));
    // Zero for a torpedo that never picked one up (avian's required
    // component defaults it, so this is only ever a bare test spawn away
    // from always being present).
    let angular = world
        .get::<AngularVelocity>(entity)
        .map_or(Vec3::ZERO, |velocity| velocity.0);

    let config = world
        .get::<TorpedoLaunchConfig>(entity)
        .unwrap_or_else(|| {
            panic!("freeze_torpedo: torpedo {entity} carries no TorpedoLaunchConfig")
        })
        .0
        .clone();

    let arming = world
        .get::<TorpedoArming>(entity)
        .cloned()
        .unwrap_or_else(|| panic!("freeze_torpedo: torpedo {entity} carries no TorpedoArming"));
    let weave = world
        .get::<TorpedoWeave>(entity)
        .cloned()
        .unwrap_or_else(|| panic!("freeze_torpedo: torpedo {entity} carries no TorpedoWeave"));
    let &TorpedoSteering(steering) = world
        .get::<TorpedoSteering>(entity)
        .unwrap_or_else(|| panic!("freeze_torpedo: torpedo {entity} carries no TorpedoSteering"));

    let cold = world.get::<TorpedoColdLaunch>(entity).copied();
    let ignited = cold.is_none();

    let controller_health =
        part_health::<TorpedoControllerMarker>(world, entity, "a TorpedoControllerMarker child")?;
    let thruster_health =
        part_health::<TorpedoThrusterMarker>(world, entity, "a TorpedoThrusterMarker child")?;

    let tracked = match world.get::<TorpedoTargetEntity>(entity) {
        Some(&TorpedoTargetEntity(target_entity)) => target(target_entity)?,
        None => None,
    };
    let last = world
        .get::<TorpedoTargetPosition>(entity)
        .map(|position| position.0);
    let target_state = match (tracked, last) {
        (Some(target), last) => SavedTorpedoTarget::Tracking { target, last },
        (None, Some(position)) => SavedTorpedoTarget::Frozen(position),
        (None, None) if world.get::<TorpedoTargetChosen>(entity).is_some() => {
            SavedTorpedoTarget::DumbFire
        }
        (None, None) => SavedTorpedoTarget::Unchosen,
    };

    Ok(FrozenTorpedo {
        config,
        owner,
        section,
        translation: transform.translation,
        rotation: transform.rotation,
        linear,
        angular,
        allegiance: world.get::<Allegiance>(entity).copied(),
        arming,
        cold,
        target: target_state,
        steering,
        weave,
        ignited,
        controller_health,
        thruster_health,
    })
}

/// Spawns the thawed torpedo and returns it.
///
/// Carries `Allegiance` when the record had one (via [`spawn_torpedo`], the
/// same conditional insert a live launch uses). Writes [`TorpedoTargetChosen`]
/// for every target state but [`SavedTorpedoTarget::Unchosen`] - a dumb-fire,
/// a tracking lock and a lost-target freeze had each already made that
/// decision live, and leaving it unwritten would let the next targeting pass
/// (filtered on `Without<TorpedoTargetChosen>`, same as a fresh launch)
/// re-target a torpedo whose lock already died. Writes
/// [`TorpedoTargetPosition`] for `Tracking` (when `last` is set) and
/// `Frozen`. Never writes [`TorpedoTargetEntity`]: see this module's doc.
///
/// Writes `record.controller_health`/`record.thruster_health` onto the
/// spawned [`TorpedoControllerMarker`]/[`TorpedoThrusterMarker`] children's
/// [`Health::current`] once [`spawn_torpedo`]'s own spawn commands have
/// landed (a deferred world closure, the same way the railgun section
/// reaches a just-spawned child), replacing the full-HP default
/// [`spawn_torpedo`] gives both from `record.config.projectile_health`. No
/// clamping.
pub fn thaw_torpedo(
    commands: &mut Commands,
    record: &FrozenTorpedo,
    owner: Entity,
    section: Option<Entity>,
) -> Entity {
    let torpedo = spawn_torpedo(
        commands,
        TorpedoLaunch {
            config: &record.config,
            owner,
            section,
            spawner: None,
            translation: record.translation,
            rotation: record.rotation,
            linear: record.linear,
            angular: record.angular,
            allegiance: record.allegiance,
            arming: record.arming.clone(),
            cold: record.cold,
            steering: record.steering,
            weave: record.weave.clone(),
        },
    );

    let controller_health = record.controller_health;
    let thruster_health = record.thruster_health;
    commands.queue(move |world: &mut World| {
        let Some(children) = world
            .get::<Children>(torpedo)
            .map(|children| children.iter().collect::<Vec<Entity>>())
        else {
            return;
        };
        for child in children {
            if world.get::<TorpedoControllerMarker>(child).is_some() {
                if let Some(mut health) = world.get_mut::<Health>(child) {
                    health.current = controller_health;
                }
            } else if world.get::<TorpedoThrusterMarker>(child).is_some() {
                if let Some(mut health) = world.get_mut::<Health>(child) {
                    health.current = thruster_health;
                }
            }
        }
    });

    match &record.target {
        SavedTorpedoTarget::Unchosen => {}
        SavedTorpedoTarget::DumbFire => {
            commands.entity(torpedo).insert(TorpedoTargetChosen);
        }
        SavedTorpedoTarget::Tracking { last, .. } => {
            commands.entity(torpedo).insert(TorpedoTargetChosen);
            if let Some(last) = *last {
                commands.entity(torpedo).insert(TorpedoTargetPosition(last));
            }
        }
        SavedTorpedoTarget::Frozen(position) => {
            commands
                .entity(torpedo)
                .insert((TorpedoTargetChosen, TorpedoTargetPosition(*position)));
        }
    }

    torpedo
}

#[cfg(test)]
mod tests {
    use nova_gameplay::{
        projectile_hooks::ProjectileHooks,
        test_support::{settle, unfinished_integrity_physics_app_with},
    };

    use super::*;

    /// A ready-to-fire ship + torpedo bay. The bay carries exactly one
    /// round, so exactly one torpedo exists to find once it fires.
    fn real_launch_app(config: TorpedoSectionConfig) -> (App, Entity, Entity, Entity) {
        let mut app = unfinished_integrity_physics_app_with(
            PhysicsPlugins::default().with_collision_hooks::<ProjectileHooks>(),
        );
        app.add_plugins(TorpedoSectionPlugin { render: false });
        app.finish();

        let ship = app
            .world_mut()
            .spawn((
                SpaceshipRootMarker,
                EntityId::new("owner_ship"),
                RigidBody::Dynamic,
                Transform::default(),
                Collider::cuboid(1.0, 1.0, 1.0),
                ColliderDensity(1.0),
            ))
            .id();
        let section = app
            .world_mut()
            .spawn((
                torpedo_section(config),
                EntityId::new("torpedo_bay"),
                Transform::IDENTITY,
                ChildOf(ship),
            ))
            .id();
        app.world_mut().flush();
        app.world_mut()
            .entity_mut(section)
            .insert(TorpedoSectionInput(true));
        settle(&mut app);

        app.update();
        let torpedo = app
            .world_mut()
            .query_filtered::<Entity, With<TorpedoProjectileMarker>>()
            .iter(app.world())
            .next()
            .expect("the one-round bay launched its torpedo");

        (app, ship, section, torpedo)
    }

    /// `TorpedoArming`'s fields, for an equality check the type itself has no
    /// `PartialEq` for (accessible here only because this module is a
    /// descendant of `torpedo_section`, the same reach `freeze_torpedo`
    /// already relies on to read them).
    fn arming_fields(arming: &TorpedoArming) -> (f32, f32, Vec3, f32, f32, bool) {
        (
            arming.min_time,
            arming.min_distance,
            arming.origin,
            arming.launcher_clearance,
            arming.elapsed,
            arming.armed,
        )
    }

    #[test]
    fn a_resumed_torpedo_keeps_its_target_arming_and_cold_launch() {
        // Quick-arming, slow-igniting: by the time the torpedo locks on it is
        // already armed but still coasting cold, which is the "mid-flight in
        // cold launch" window the save has to carry faithfully.
        let config = TorpedoSectionConfig {
            fire_rate: 100.0,
            ammunition: AmmoCapacity::Limited(1),
            arm_time: 0.01,
            arm_distance: Meters(0.0),
            // Small enough that the launcher-clearance half of arming (hull
            // radius + blast radius, cleared at the ejection speed below)
            // is satisfied within the first couple of ticks too - a stock
            // 300 m blast would hold this torpedo unarmed for the whole
            // cold coast.
            blast_radius: Meters(5.0),
            spawner_speed: MetersPerSecond(300.0),
            ignition_delay: 5.0,
            ..default()
        };
        let (mut app, ship, section, torpedo) = real_launch_app(config);

        let target = app
            .world_mut()
            .spawn((
                Transform::from_translation(Vec3::new(0.0, 0.0, -200.0)),
                ComputedCenterOfMass(Vec3::ZERO),
            ))
            .id();
        app.world_mut()
            .entity_mut(torpedo)
            .insert((TorpedoTargetChosen, TorpedoTargetEntity(target)));
        // A couple more ticks: update_target_position locks
        // TorpedoTargetPosition, update_torpedo_arming clears arm_time and
        // (at 300 m/s over 1/60s steps) the 5 m launcher clearance, and the
        // torpedo is still coasting (ignition_delay 5.0s).
        app.update();
        app.update();

        // Damage the controller alone, through the real pipeline, so the
        // two parts' saved health differ from the authored max AND from
        // each other - freezing a torpedo that only ever had full HP on
        // both parts could not catch a thaw that mixed them up or ignored
        // them.
        let children = app
            .world()
            .get::<Children>(torpedo)
            .expect("spawn_torpedo gives the torpedo a controller and a thruster child")
            .iter()
            .collect::<Vec<Entity>>();
        let controller = children
            .iter()
            .copied()
            .find(|&child| app.world().get::<TorpedoControllerMarker>(child).is_some())
            .expect("the torpedo has a controller child");
        let thruster = children
            .iter()
            .copied()
            .find(|&child| app.world().get::<TorpedoThrusterMarker>(child).is_some())
            .expect("the torpedo has a thruster child");
        app.world_mut().trigger(HealthApplyDamage {
            entity: controller,
            source: None,
            amount: 4.0,
        });
        app.world_mut().flush();
        let controller_health_before = app
            .world()
            .get::<Health>(controller)
            .expect("the controller keeps its Health")
            .current;
        let thruster_health_before = app
            .world()
            .get::<Health>(thruster)
            .expect("the thruster keeps its Health")
            .current;
        assert_ne!(
            controller_health_before, thruster_health_before,
            "only the controller took damage"
        );

        let cold_before = app
            .world()
            .get::<TorpedoColdLaunch>(torpedo)
            .copied()
            .expect("still coasting: ignition_delay 5.0s has not elapsed");
        assert!(
            cold_before.remaining > 0.0 && cold_before.remaining < 5.0,
            "mid-flight: ticked down from the authored 5.0s but not yet ignited, got {}",
            cold_before.remaining
        );
        let arming_before = app
            .world()
            .get::<TorpedoArming>(torpedo)
            .expect("update_torpedo_arming runs regardless of cold launch")
            .clone();
        assert!(
            arming_before.is_armed(),
            "arm_time 0.01s must have elapsed by the first tick"
        );
        let weave_before = app
            .world()
            .get::<TorpedoWeave>(torpedo)
            .expect("spawn_torpedo always inserts a weave")
            .clone();
        let steering_before = **app
            .world()
            .get::<TorpedoSteering>(torpedo)
            .expect("spawn_torpedo always inserts a steering command");

        let record = freeze_torpedo(app.world(), torpedo, |entity| {
            assert_eq!(
                entity, target,
                "freeze_torpedo must resolve the live target it was given"
            );
            Ok(Some(SavedTargetRef::Body(SavedBodyRef(EntityId::new(
                "target_ship",
            )))))
        })
        .expect("a mid-flight, armed-but-cold torpedo freezes cleanly");
        let last_before = match &record.target {
            SavedTorpedoTarget::Tracking {
                target: SavedTargetRef::Body(id),
                last,
            } => {
                assert_eq!(id.0 .0, "target_ship");
                last.expect("update_target_position had already run once")
            }
            other => panic!("expected a Tracking target, got {other:?}"),
        };
        assert!(!record.ignited, "the drive had not lit yet at freeze time");
        assert!(record.cold.is_some());
        assert_eq!(
            record.controller_health, controller_health_before,
            "freeze_torpedo must read the controller's live, damaged Health"
        );
        assert_eq!(
            record.thruster_health, thruster_health_before,
            "freeze_torpedo must read the thruster's live, undamaged Health"
        );

        // `on_damage` zeroes a part at once and marks it a flush later; the
        // torpedo is shot down a flush after that. In that window the
        // freeze waits rather than save a part a Load would refuse.
        let no_target = |_| -> Result<Option<SavedTargetRef>, TransientFreezeFault> {
            unreachable!("a torpedo being destroyed is refused before its target")
        };
        let being_destroyed = |world: &World| {
            matches!(
                freeze_torpedo(world, torpedo, no_target),
                Err(TransientFreezeFault::Unsettled(_))
            )
        };
        app.world_mut()
            .get_mut::<Health>(controller)
            .unwrap()
            .current = 0.0;
        assert!(being_destroyed(app.world()), "a zeroed part is unsettled");
        app.world_mut()
            .get_mut::<Health>(controller)
            .unwrap()
            .current = controller_health_before;

        app.world_mut().entity_mut(torpedo).despawn();

        let thawed = {
            let mut commands = app.world_mut().commands();
            thaw_torpedo(&mut commands, &record, ship, Some(section))
        };
        app.world_mut().flush();
        // The collector's job, done by hand here: resolve the saved target
        // key back to the live entity it names.
        app.world_mut()
            .entity_mut(thawed)
            .insert(TorpedoTargetEntity(target));

        let thawed_children = app
            .world()
            .get::<Children>(thawed)
            .expect("thaw_torpedo gives the torpedo a controller and a thruster child")
            .iter()
            .collect::<Vec<Entity>>();
        let thawed_controller_health = thawed_children
            .iter()
            .copied()
            .find(|&child| app.world().get::<TorpedoControllerMarker>(child).is_some())
            .and_then(|child| app.world().get::<Health>(child))
            .expect("the thawed controller carries Health")
            .current;
        let thawed_thruster_health = thawed_children
            .iter()
            .copied()
            .find(|&child| app.world().get::<TorpedoThrusterMarker>(child).is_some())
            .and_then(|child| app.world().get::<Health>(child))
            .expect("the thawed thruster carries Health")
            .current;
        assert_eq!(
            thawed_controller_health, controller_health_before,
            "thaw_torpedo must write the saved controller health back onto the thawed child"
        );
        assert_eq!(
            thawed_thruster_health, thruster_health_before,
            "thaw_torpedo must write the saved thruster health back onto the thawed child"
        );

        let cold_after = app
            .world()
            .get::<TorpedoColdLaunch>(thawed)
            .copied()
            .expect("thaw_torpedo must carry the cold-launch countdown across");
        assert_eq!(
            cold_after.remaining, cold_before.remaining,
            "the cold-launch countdown must survive the round trip exactly"
        );
        let arming_after = app
            .world()
            .get::<TorpedoArming>(thawed)
            .expect("thaw_torpedo must carry arming across");
        assert_eq!(
            arming_fields(arming_after),
            arming_fields(&arming_before),
            "arming must survive the round trip exactly"
        );
        let weave_after = app
            .world()
            .get::<TorpedoWeave>(thawed)
            .expect("thaw_torpedo must carry the weave across");
        assert_eq!(weave_after.angle, weave_before.angle);
        assert_eq!(weave_after.rate, weave_before.rate);
        assert_eq!(weave_after.offset, weave_before.offset);
        let steering_after = **app
            .world()
            .get::<TorpedoSteering>(thawed)
            .expect("thaw_torpedo must carry the steering command across");
        assert_eq!(steering_after, steering_before);
        assert_eq!(
            app.world()
                .get::<TorpedoTargetPosition>(thawed)
                .map(|p| p.0),
            Some(last_before),
            "the thaw must carry the last known target position across"
        );

        // Detonation attribution survives too: let the drive catch up
        // (ignition_delay 5.0s, several 1/60s ticks), teleport onto the
        // target - proving data, not flight - and let the real fuze run.
        for _ in 0..310 {
            app.update();
            if app.world().get::<TorpedoColdLaunch>(thawed).is_none() {
                break;
            }
        }
        assert!(
            app.world().get::<TorpedoColdLaunch>(thawed).is_none(),
            "the drive must have lit by now"
        );
        let target_position = **app
            .world()
            .get::<TorpedoTargetPosition>(thawed)
            .expect("target is still tracked");
        app.world_mut()
            .entity_mut(thawed)
            .insert(Transform::from_translation(target_position));
        app.world_mut()
            .entity_mut(thawed)
            .insert(Position(target_position));
        app.update();

        assert!(
            !app.world().entities().contains(thawed),
            "an armed, ignited torpedo on its target must detonate"
        );
        let mut q_blast = app
            .world_mut()
            .query_filtered::<&ProjectileOwner, With<NovaBlast>>();
        let blast_owner = q_blast
            .single(app.world())
            .expect("the detonation spawned exactly one owned blast");
        assert_eq!(
            **blast_owner, ship,
            "the blast stays attributed to the firing ship"
        );
    }

    #[test]
    fn a_resumed_torpedo_with_a_frozen_target_keeps_its_last_known_position() {
        let config = TorpedoSectionConfig {
            fire_rate: 100.0,
            ammunition: AmmoCapacity::Limited(1),
            arm_time: 0.01,
            arm_distance: Meters(0.0),
            ignition_delay: 5.0,
            ..default()
        };
        let (mut app, ship, section, torpedo) = real_launch_app(config);

        let target = app
            .world_mut()
            .spawn((
                Transform::from_translation(Vec3::new(0.0, 0.0, -200.0)),
                ComputedCenterOfMass(Vec3::ZERO),
            ))
            .id();
        app.world_mut()
            .entity_mut(torpedo)
            .insert((TorpedoTargetChosen, TorpedoTargetEntity(target)));
        // Lock on, then lose it: the save must freeze on the last known
        // position, exactly as a live torpedo's own guidance does.
        app.update();
        app.world_mut().entity_mut(target).despawn();
        app.update();

        assert!(
            app.world().get::<TorpedoTargetEntity>(torpedo).is_none(),
            "update_target_position must have dropped the dead target link"
        );
        let last_position = **app
            .world()
            .get::<TorpedoTargetPosition>(torpedo)
            .expect("the position freezes in place rather than vanishing");

        let record = freeze_torpedo(app.world(), torpedo, |_| {
            panic!("a dead target link must not call the resolver")
        })
        .expect("a torpedo whose target already died still freezes cleanly");
        match record.target {
            SavedTorpedoTarget::Frozen(position) => assert_eq!(position, last_position),
            other => panic!("expected a Frozen target, got {other:?}"),
        }

        app.world_mut().entity_mut(torpedo).despawn();
        let thawed = {
            let mut commands = app.world_mut().commands();
            thaw_torpedo(&mut commands, &record, ship, Some(section))
        };
        app.world_mut().flush();

        assert!(
            app.world().get::<TorpedoTargetChosen>(thawed).is_some(),
            "a thawed Frozen target must carry TorpedoTargetChosen, or the \
             next targeting pass (Without<TorpedoTargetChosen>) re-locks it \
             onto a new target - contradicting the frozen-on-death invariant"
        );
        assert_eq!(
            app.world()
                .get::<TorpedoTargetPosition>(thawed)
                .map(|p| p.0),
            Some(last_position),
        );
        assert!(
            app.world().get::<TorpedoTargetEntity>(thawed).is_none(),
            "thaw_torpedo never writes TorpedoTargetEntity - that is the \
             collector's job, and there is nothing live to resolve here"
        );
    }
}
