//! Freeze/thaw of a section's durable runtime state, for a streamed sector
//! that despawns a live ship and later rebuilds it.
//!
//! A freeze reads a section (and the plates and decor bolted to it) straight
//! off the live `World` into a [`FrozenSection`], a value with no `Entity`
//! anywhere in it, so the caller can despawn the body it came from. A thaw
//! applies that value onto a freshly spawned section in the SAME command
//! batch its own spawner builds it in, overriding the fresh (full-health,
//! fully-clad) state the batch would otherwise leave.
//!
//! [`freeze_section`] refuses (returns [`UnsettledBody`]) while a multi-frame
//! destruction process is still running on the section or one of its
//! fixtures - see the function's doc for the exact conditions - so the
//! caller's only job on a refusal is to ask again next frame.

use avian3d::prelude::{
    AngularVelocity, CenterOfMass, Collider, ComputeMassProperties3d, LinearVelocity,
    NoAutoCenterOfMass, RigidBody,
};
use bevy::prelude::*;
use nova_gameplay::prelude::{
    AssetRef, Health, HealthZeroMarker, IntegrityDestroyMarker, IntegrityDisabledMarker,
    SectionInactiveMarker, TransientFreezeFault, UnsettledBody,
};

use super::{
    ammo::prelude::{SectionAmmo, SectionReload, SuspendedSectionAmmo},
    cargo_intake_section::prelude::CargoIntakeEjectionQueue,
    fixture::prelude::ShedFixtureMarker,
    railgun_section::prelude::RailgunCharge,
    section_animation::prelude::{
        FrozenSectionAnimations, SectionAnimationRigDirty, SectionAnimations,
    },
    shell_shape::prelude::ShellShape,
    shell_skin::{frozen_plate_body, ShipSkinMarker},
    skin_decor::{frozen_decor_body, DecorColliderSize, ShipDecorMarker},
    skin_style::ShipStyle,
    turret_section::prelude::{FrozenTurretHinges, TurretStow},
};

/// `FrozenSection`, `freeze_section`, `thaw_section`, and the shed-fixture
/// transient pair.
pub mod prelude {
    pub use super::{
        freeze_section, freeze_shed_fixture, thaw_section, thaw_shed_fixture, FrozenFixture,
        FrozenSection, FrozenShedFixture,
    };
}

/// A fixture still bolted on when its body froze: a skin plate on a section,
/// or a decoration on a plate.
///
/// Recursive: a plate's own `children` is where its frozen decor lives, so
/// one [`FrozenSection::fixtures`] walk captures a whole cladding tree.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct FrozenFixture {
    kind: FrozenFixtureKind,
    pose: Transform,
    health: Health,
    children: Vec<FrozenFixture>,
}

#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
enum FrozenFixtureKind {
    Plate(ShellShape),
    Decor {
        name: String,
        model: AssetRef<WorldAsset>,
        collider_size: Vec3,
    },
}

/// A section's durable runtime state when its body froze.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct FrozenSection {
    health: Health,
    inactive: bool,
    ammo: Option<SectionAmmo>,
    suspended_ammo: Option<SuspendedSectionAmmo>,
    reload: Option<SectionReload>,
    ejection_queue: Option<CargoIntakeEjectionQueue>,
    turret_stow: Option<TurretStow>,
    railgun_charge: Option<RailgunCharge>,
    animations: FrozenSectionAnimations,
    hinges: FrozenTurretHinges,
    fixtures: Vec<FrozenFixture>,
}

/// Whether `entity` is a fixture [`freeze_fixture`]/[`spawn_frozen_fixture`]
/// know how to handle: a skin plate or a piece of decor. Everything else
/// under a section or a plate (render children like `SkinSurfaceMarker`
/// meshes) is re-derived by the dressing observers and is not part of the
/// frozen record.
fn is_fixture(world: &World, entity: Entity) -> bool {
    world.get::<ShipSkinMarker>(entity).is_some() || world.get::<ShipDecorMarker>(entity).is_some()
}

/// Capture `entity` (a plate or a piece of decor) and every fixture bolted
/// onto IT, recursively.
///
/// # Errors
///
/// [`UnsettledBody`] when `entity` carries [`HealthZeroMarker`]: it already
/// hit zero but has not yet been shed (dropped from `ChildOf`) or despawned,
/// a backlog [`fixture`](super::fixture) bounds per frame rather than
/// draining in one tick. Freezing it would carry a dead plate into the thaw
/// instead of letting the shed finish.
///
/// # Panics
///
/// When `entity` is neither a [`ShipSkinMarker`] plate nor a [`ShipDecorMarker`]
/// piece of decor, is missing the `Transform`/`Health` every fixture of
/// either kind carries, or (decor only) is missing [`DecorColliderSize`] -
/// callers only ever reach this through [`is_fixture`]'s filter, so that
/// would mean a fixture body changed shape out from under this module.
fn freeze_fixture(world: &World, entity: Entity) -> Result<FrozenFixture, UnsettledBody> {
    if world.get::<HealthZeroMarker>(entity).is_some() {
        return Err(UnsettledBody {
            reason: "fixture shed backlog: a dead plate or decor not yet despawned",
        });
    }

    let kind = if let Some(ShipSkinMarker(shape)) = world.get::<ShipSkinMarker>(entity) {
        FrozenFixtureKind::Plate(*shape)
    } else if let Some(ShipDecorMarker(model)) = world.get::<ShipDecorMarker>(entity) {
        let Some(name) = world.get::<Name>(entity) else {
            panic!("freeze_fixture: decor {entity:?} carries no Name");
        };
        let Some(DecorColliderSize(collider_size)) = world.get::<DecorColliderSize>(entity) else {
            panic!("freeze_fixture: decor {entity:?} carries no DecorColliderSize");
        };
        FrozenFixtureKind::Decor {
            name: name.as_str().to_string(),
            model: model.clone(),
            collider_size: *collider_size,
        }
    } else {
        panic!("freeze_fixture: entity {entity:?} is neither a skin plate nor a piece of decor");
    };

    let Some(pose) = world.get::<Transform>(entity) else {
        panic!("freeze_fixture: fixture {entity:?} carries no Transform");
    };
    let Some(health) = world.get::<Health>(entity) else {
        panic!("freeze_fixture: fixture {entity:?} carries no Health");
    };

    Ok(FrozenFixture {
        kind,
        pose: *pose,
        health: health.clone(),
        children: freeze_fixtures(world, entity)?,
    })
}

/// Capture every direct fixture child of `parent` (a section or a plate), in
/// child order.
fn freeze_fixtures(world: &World, parent: Entity) -> Result<Vec<FrozenFixture>, UnsettledBody> {
    let Some(children) = world.get::<Children>(parent) else {
        return Ok(Vec::new());
    };
    children
        .iter()
        .filter(|child| is_fixture(world, *child))
        .map(|child| freeze_fixture(world, child))
        .collect()
}

/// Capture `section`'s durable runtime state: its [`Health`], its
/// [`SectionInactiveMarker`] flag, its ammo/reload (or the suspended
/// magazine the unlimited-ammo cheat parked), any queued intake ejections,
/// its turret stow phase, its railgun charge, every animation track's
/// progress and target, its turret hinge angles, and every fixture still
/// bolted to it.
///
/// Not captured because each re-derives within one frame of a fresh spawn,
/// from state this capture or the design already carries: [`HealthZeroMarker`]
/// and [`IntegrityDisabledMarker`] (both read straight off the restored
/// `Health`), `DamageLevel` (erosion's per-frame read of that same `Health`
/// fraction), and `ConnectedTo` (the integrity graph, rebuilt from link
/// points every spawn batch).
///
/// # Errors
///
/// [`UnsettledBody`] while a multi-frame destruction process has not
/// finished with this section: it carries [`HealthZeroMarker`] (just hit
/// zero, the disable observer has not run yet), [`IntegrityDisabledMarker`]
/// (disabled, awaiting its own destroy-or-shed decision) or
/// [`IntegrityDestroyMarker`] (queued for despawn), or any of its fixtures
/// carries [`HealthZeroMarker`] (see [`freeze_fixture`]). The caller keeps
/// the section live and asks again next frame.
pub fn freeze_section(world: &World, section: Entity) -> Result<FrozenSection, UnsettledBody> {
    if world.get::<HealthZeroMarker>(section).is_some() {
        return Err(UnsettledBody {
            reason: "section at zero health, awaiting its disable observer",
        });
    }
    if world.get::<IntegrityDisabledMarker>(section).is_some() {
        return Err(UnsettledBody {
            reason: "section disabled, awaiting a destroy-or-shed decision",
        });
    }
    if world.get::<IntegrityDestroyMarker>(section).is_some() {
        return Err(UnsettledBody {
            reason: "section queued for destruction",
        });
    }

    let Some(health) = world.get::<Health>(section) else {
        panic!("freeze_section: section {section:?} carries no Health");
    };
    let Some(animations) = world.get::<SectionAnimations>(section) else {
        panic!("freeze_section: section {section:?} carries no SectionAnimations");
    };

    Ok(FrozenSection {
        health: health.clone(),
        inactive: world.get::<SectionInactiveMarker>(section).is_some(),
        ammo: world.get::<SectionAmmo>(section).copied(),
        suspended_ammo: world.get::<SuspendedSectionAmmo>(section).copied(),
        reload: world.get::<SectionReload>(section).copied(),
        ejection_queue: world.get::<CargoIntakeEjectionQueue>(section).cloned(),
        turret_stow: world.get::<TurretStow>(section).copied(),
        railgun_charge: world.get::<RailgunCharge>(section).copied(),
        animations: animations.freeze(),
        hinges: FrozenTurretHinges::freeze(world, section),
        fixtures: freeze_fixtures(world, section)?,
    })
}

/// Spawn one frozen fixture (and its own frozen children, recursively) as a
/// child of `parent`.
fn spawn_frozen_fixture(parent: &mut ChildSpawnerCommands, frozen: FrozenFixture) {
    let mut spawned = match frozen.kind {
        FrozenFixtureKind::Plate(shape) => {
            parent.spawn(frozen_plate_body(shape, frozen.pose, frozen.health))
        }
        FrozenFixtureKind::Decor {
            name,
            model,
            collider_size,
        } => parent.spawn(frozen_decor_body(
            name,
            model,
            frozen.pose,
            frozen.health,
            collider_size,
        )),
    };
    spawned.with_children(|grandchildren| {
        for child in frozen.children {
            spawn_frozen_fixture(grandchildren, child);
        }
    });
}

/// Apply `frozen` onto `section`, a section entity spawned in the SAME
/// command batch its own spawner built it in: overrides the batch's fresh
/// (full-health, unclad-of-frozen-fixtures) state and spawns every frozen
/// fixture as a child.
///
/// The animation tracks and turret hinges land where they froze, so the
/// restored [`TurretStow`] phase, [`RailgunCharge`] and ejection queue read
/// cue progress that matches them. A kind whose own driving state is not
/// frozen re-steers from that pose: the mining emitter re-arms stowed, and a
/// bay door follows its live trigger. Both restore in a queued command, which
/// runs after the spawner's own commands and the observers they fire have
/// built the tracks and the joint tree.
///
/// Ammo, reload, the suspended magazine and the ejection queue are each
/// removed when the frozen record held none, so a cheat or a queue the fresh
/// spawn does not know about never leaks forward; [`SectionInactiveMarker`]
/// is only ever inserted, never removed, because a derelict design's
/// spawn-time inactive flag is a second, independently valid source for it.
pub fn thaw_section(section: &mut EntityCommands, frozen: FrozenSection) {
    section.insert(frozen.health);
    if frozen.inactive {
        section.insert(SectionInactiveMarker);
    }
    match frozen.ammo {
        Some(ammo) => section.insert(ammo),
        None => section.remove::<SectionAmmo>(),
    };
    match frozen.reload {
        Some(reload) => section.insert(reload),
        None => section.remove::<SectionReload>(),
    };
    match frozen.suspended_ammo {
        Some(suspended) => section.insert(suspended),
        None => section.remove::<SuspendedSectionAmmo>(),
    };
    match frozen.ejection_queue {
        Some(queue) => section.insert(queue),
        None => section.remove::<CargoIntakeEjectionQueue>(),
    };
    if let Some(stow) = frozen.turret_stow {
        // The stow lift joint is code-built, so no scene ready marks the rig
        // that poses it; the armer queues this resolve on a fresh turret.
        section.insert((stow, SectionAnimationRigDirty));
    }
    if let Some(charge) = frozen.railgun_charge {
        section.insert(charge);
    }
    let animations = frozen.animations;
    let hinges = frozen.hinges;
    section.queue(move |mut entity: EntityWorldMut| {
        let Some(mut thawed) = entity.get_mut::<SectionAnimations>() else {
            panic!("thaw_section: section carries no SectionAnimations");
        };
        thawed.thaw(&animations);
        let id = entity.id();
        entity.world_scope(|world| hinges.thaw(world, id));
    });
    section.with_children(|children| {
        for fixture in frozen.fixtures {
            spawn_frozen_fixture(children, fixture);
        }
    });
}

/// A fixture already shed off its ship when its body froze: debris drifting
/// on its own [`TempEntity`](nova_gameplay::prelude::TempEntity) countdown,
/// outside the section/plate tree [`FrozenSection`] walks and with no
/// `ChildOf` of its own.
///
/// `style` is the [`ShipStyle`] id the ship wore at the moment this fixture
/// was shed - the durable key a ship names one by, `None` when it wore none -
/// stamped onto the fixture by `shed_dead_fixtures` (`fixture.rs`) at that
/// moment rather than read here: shedding drops the fixture's only `ChildOf`
/// path to that ancestor, so by the time a save freezes the drifting debris
/// there is no ancestor left to walk.
///
/// Carries no countdown of its own: this fixture's remaining
/// [`TempEntity`] time is kept on the save collector's own
/// `FrozenTransient::lifetime`, the same as a resumed round's.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct FrozenShedFixture {
    /// The fixture's own shape, pose and health, and every fixture still
    /// bolted to it (a greeble riding a shed plate).
    pub fixture: FrozenFixture,
    /// The ship style id this fixture wore when it was shed, or `None` for a
    /// ship that wore none.
    pub style: Option<String>,
    /// World translation at the moment it froze.
    pub translation: Vec3,
    /// World rotation at the moment it froze.
    pub rotation: Quat,
    /// Linear velocity it was drifting with.
    pub linear: Vec3,
    /// Angular velocity it was tumbling with.
    pub angular: Vec3,
}

/// Capture a shed fixture `entity` drifting on its own: the same fixture
/// shape, pose and health [`freeze_fixture`] captures for a bolted-on one,
/// plus the world pose, the drift it inherited when it came off, the style
/// it was stamped with at shed time, and what is left of its countdown.
///
/// # Errors
///
/// [`TransientFreezeFault::Unsettled`] when one of `entity`'s OWN fixture
/// children (a greeble still bolted to a shed plate) carries
/// [`HealthZeroMarker`] and has not yet been shed in its own right - see
/// [`freeze_fixture`]. `entity` itself is never refused on that marker: once
/// shed, a fixture has no `ChildOf` left to leave, so a dead one would sit at
/// zero health forever and block every later save.
///
/// # Panics
///
/// When `entity` is neither a [`ShipSkinMarker`] plate nor a
/// [`ShipDecorMarker`] piece of decor, or is missing the `Transform`,
/// `Health`, [`ShipStyle`], `LinearVelocity` or `AngularVelocity` every shed
/// fixture carries once `shed_dead_fixtures` has stamped it - callers only
/// ever reach this through the transient classification that reads
/// [`ShedFixtureMarker`] off the same entity, so a missing component means a
/// shed fixture's own bundle changed shape out from under this module.
pub fn freeze_shed_fixture(
    world: &World,
    entity: Entity,
) -> Result<FrozenShedFixture, TransientFreezeFault> {
    let kind = if let Some(ShipSkinMarker(shape)) = world.get::<ShipSkinMarker>(entity) {
        FrozenFixtureKind::Plate(*shape)
    } else if let Some(ShipDecorMarker(model)) = world.get::<ShipDecorMarker>(entity) {
        let Some(name) = world.get::<Name>(entity) else {
            panic!("freeze_shed_fixture: decor {entity:?} carries no Name");
        };
        let Some(DecorColliderSize(collider_size)) = world.get::<DecorColliderSize>(entity) else {
            panic!("freeze_shed_fixture: decor {entity:?} carries no DecorColliderSize");
        };
        FrozenFixtureKind::Decor {
            name: name.as_str().to_string(),
            model: model.clone(),
            collider_size: *collider_size,
        }
    } else {
        panic!(
            "freeze_shed_fixture: entity {entity:?} is neither a skin plate nor a piece of decor"
        );
    };

    let Some(pose) = world.get::<Transform>(entity) else {
        panic!("freeze_shed_fixture: shed fixture {entity:?} carries no Transform");
    };
    let Some(health) = world.get::<Health>(entity) else {
        panic!("freeze_shed_fixture: shed fixture {entity:?} carries no Health");
    };
    let Some(ShipStyle(style)) = world.get::<ShipStyle>(entity) else {
        panic!("freeze_shed_fixture: shed fixture {entity:?} carries no ShipStyle");
    };
    let Some(LinearVelocity(linear)) = world.get::<LinearVelocity>(entity) else {
        panic!("freeze_shed_fixture: shed fixture {entity:?} carries no LinearVelocity");
    };
    let Some(AngularVelocity(angular)) = world.get::<AngularVelocity>(entity) else {
        panic!("freeze_shed_fixture: shed fixture {entity:?} carries no AngularVelocity");
    };

    Ok(FrozenShedFixture {
        fixture: FrozenFixture {
            kind,
            pose: *pose,
            health: health.clone(),
            children: freeze_fixtures(world, entity).map_err(TransientFreezeFault::Unsettled)?,
        },
        style: style.clone(),
        translation: pose.translation,
        rotation: pose.rotation,
        linear: *linear,
        angular: *angular,
    })
}

/// Rebuild a shed fixture `record` froze: the same body [`spawn_frozen_fixture`]
/// builds for a bolted-on fixture, with no `ChildOf` and the debris state
/// `shed_dead_fixtures` (`fixture.rs`) gives a freshly shed one - kinematic,
/// pivoted on its own collider's centre, carrying the velocity it drifted
/// with.
///
/// [`ShipStyle`] rides in the SAME initial bundle as the plate or decor
/// marker, not a later `insert`: the `Add` observer that dresses a plate
/// fires the moment its marker lands, and a thawed fixture has no ancestor to
/// walk, so the style has to already be there for it to find.
///
/// Carries no [`TempEntity`](nova_gameplay::prelude::TempEntity) countdown: a
/// save collector inserts one from its own `FrozenTransient::lifetime` once
/// every transient's refs have resolved, the same as it does for a resumed
/// round.
/// Carries no [`Health`] either: a shed fixture can never take another hit
/// (its collider is already gone), and a fresh `Health` insert on a brand new
/// entity is exactly the shape a "this just happened" observer would read.
pub fn thaw_shed_fixture(commands: &mut Commands, record: &FrozenShedFixture) -> Entity {
    let pose = Transform::from_translation(record.translation).with_rotation(record.rotation);
    let style = ShipStyle(record.style.clone());
    let mut entity = match &record.fixture.kind {
        FrozenFixtureKind::Plate(shape) => commands.spawn((
            frozen_plate_body(*shape, pose, record.fixture.health.clone()),
            style,
            ShedFixtureMarker(Entity::PLACEHOLDER),
            RigidBody::Kinematic,
            NoAutoCenterOfMass,
            LinearVelocity(record.linear),
            AngularVelocity(record.angular),
        )),
        FrozenFixtureKind::Decor {
            name,
            model,
            collider_size,
        } => commands.spawn((
            frozen_decor_body(
                name.clone(),
                model.clone(),
                pose,
                record.fixture.health.clone(),
                *collider_size,
            ),
            style,
            ShedFixtureMarker(Entity::PLACEHOLDER),
            RigidBody::Kinematic,
            NoAutoCenterOfMass,
            LinearVelocity(record.linear),
            AngularVelocity(record.angular),
        )),
    };
    entity.remove::<Health>();
    entity.queue(|mut entity: EntityWorldMut| {
        let id = entity.id();
        let Some(collider) = entity.get::<Collider>() else {
            panic!("thaw_shed_fixture: fixture {id} carries no Collider after its body bundle");
        };
        let center = collider.center_of_mass();
        entity.insert(CenterOfMass(center));
    });
    let children = record.fixture.children.clone();
    entity.with_children(|spawned| {
        for child in children {
            spawn_frozen_fixture(spawned, child);
        }
    });
    entity.id()
}

#[cfg(all(test, feature = "serde"))]
mod tests {
    use super::*;

    /// A frozen plate's collider rebuilds from the `ShellShape` it carries,
    /// and a frozen decoration's rebuilds from the `DecorColliderSize` riding
    /// beside it - neither carries avian's own `Collider` across the freeze,
    /// since that type has no serde impl - and both land with the exact
    /// bounds they had before the freeze. The authored collider size itself
    /// also survives the RON round trip unchanged.
    #[test]
    fn a_frozen_plate_and_decor_thaw_with_the_colliders_they_froze_with() {
        use avian3d::prelude::{Collider, SimpleCollider};

        use crate::sections::{
            shell_shape::prelude::FULL,
            shell_skin::{plate_body, SkinPlate},
            skin_decor::decor_body,
            skin_style::{FixturePlacement, StyleFixtureConfig},
        };

        let mut world = World::new();
        let section = world.spawn(Name::new("section")).id();

        let shape = ShellShape::new([FULL; 4], [FULL; 4]).expect("a legal shape");
        let plate = SkinPlate {
            cell: IVec3::ZERO,
            anchor: IVec3::NEG_Y,
            shape,
            rotation: Quat::IDENTITY,
        };
        let plate_entity = world
            .spawn((plate_body(&plate, Transform::IDENTITY), ChildOf(section)))
            .id();

        let collider_size = Vec3::new(0.3, 1.25, 0.7);
        let fixture = StyleFixtureConfig {
            id: "greeble".to_string(),
            model: AssetRef::from("self://gltf/greebles/placeholder_block.glb#Scene0".to_string()),
            health: 10.0,
            collider: collider_size,
            placement: FixturePlacement::default(),
        };
        let decor_entity = world
            .spawn((
                decor_body(&fixture, Transform::IDENTITY),
                ChildOf(plate_entity),
            ))
            .id();

        let original_plate_aabb = world
            .get::<Collider>(plate_entity)
            .expect("a plate carries a collider")
            .aabb(Vec3::ZERO, Quat::IDENTITY);
        let original_decor_aabb = world
            .get::<Collider>(decor_entity)
            .expect("a decoration carries a collider")
            .aabb(Vec3::ZERO, Quat::IDENTITY);

        let frozen = freeze_fixtures(&world, section).expect("a settled plate and decor freeze");
        let written = ron::to_string(&frozen).expect("a frozen fixture serializes");
        let frozen: Vec<FrozenFixture> = ron::from_str(&written).expect("and parses back");

        let thaw_root = world.spawn_empty().id();
        let mut commands = world.commands();
        commands.entity(thaw_root).with_children(|children| {
            for fixture in frozen {
                spawn_frozen_fixture(children, fixture);
            }
        });
        world.flush();

        let thawed_plate = world
            .get::<Children>(thaw_root)
            .expect("the plate thawed back as a child of the root")
            .iter()
            .find(|&entity| world.get::<ShipSkinMarker>(entity).is_some())
            .expect("a thawed plate among the root's children");
        let thawed_decor = world
            .get::<Children>(thawed_plate)
            .expect("the decoration thawed back as a child of the plate")
            .iter()
            .next()
            .expect("the plate's one thawed decoration");

        assert_eq!(
            world
                .get::<DecorColliderSize>(thawed_decor)
                .expect("the thawed decoration carries its collider size"),
            &DecorColliderSize(collider_size),
            "the authored collider size did not survive the freeze/thaw round trip",
        );

        let thawed_plate_aabb = world
            .get::<Collider>(thawed_plate)
            .expect("the thawed plate carries a collider")
            .aabb(Vec3::ZERO, Quat::IDENTITY);
        assert_eq!(thawed_plate_aabb.min, original_plate_aabb.min);
        assert_eq!(thawed_plate_aabb.max, original_plate_aabb.max);

        let thawed_decor_aabb = world
            .get::<Collider>(thawed_decor)
            .expect("the thawed decoration carries a collider")
            .aabb(Vec3::ZERO, Quat::IDENTITY);
        assert_eq!(thawed_decor_aabb.min, original_decor_aabb.min);
        assert_eq!(thawed_decor_aabb.max, original_decor_aabb.max);
    }

    /// A plate and a piece of decor shed off a STYLED ship through the real
    /// `shed_dead_fixtures` path, each saved and brought back, must keep the
    /// art they wore, the countdown they were partway through, and the pose
    /// they drifted to - and must NOT come back with `Health`, since neither
    /// can ever take another hit with its collider already gone.
    #[test]
    fn a_resumed_shed_fixture_keeps_its_art_style_and_grace() {
        use avian3d::prelude::{Collider, SimpleCollider};
        use bevy::time::TimeUpdateStrategy;
        use bevy_rand::prelude::{EntropyPlugin, WyRand};
        use nova_gameplay::prelude::{
            resumed_lifetime, HealthApplyDamage, NovaHealthPlugin, SavedLifetime, TempEntityPlugin,
        };

        use crate::sections::{
            shell_shape::prelude::FULL,
            shell_skin::{plate_body, ShipSkinPlugin, SkinPlate},
            skin_decor::decor_body,
            skin_style::{
                FixturePlacement, GameStyles, ShipStyleConfig, StyleFixtureConfig, StylePalette,
                SurfaceFinish,
            },
        };

        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            bevy::asset::AssetPlugin::default(),
            TransformPlugin,
            NovaHealthPlugin,
            TempEntityPlugin,
            EntropyPlugin::<WyRand>::with_seed(11u64.to_ne_bytes()),
        ));
        app.init_asset::<Mesh>();
        app.init_asset::<StandardMaterial>();
        app.init_asset::<WorldAsset>();
        app.add_plugins(ShipSkinPlugin { render: true });

        // A distinctive finish neither `ShellSurface::colour()` default ever
        // produces, so finding it on the thawed plate's material proves the
        // STYLE was read, not the built-in look.
        let dyed = SurfaceFinish {
            color: Color::srgb(0.9, 0.2, 0.05),
            roughness: 0.4,
            metallic: 0.8,
        };
        app.insert_resource(GameStyles(vec![ShipStyleConfig {
            id: "raider".to_string(),
            name: "Raider".to_string(),
            palette: StylePalette {
                top: dyed.clone(),
                wall: dyed.clone(),
            },
            fixtures: Vec::new(),
        }]));

        let ship = app
            .world_mut()
            .spawn((Name::new("ship"), ShipStyle(Some("raider".to_string()))))
            .id();
        let section = app.world_mut().spawn(ChildOf(ship)).id();

        let shape = ShellShape::new([FULL; 4], [FULL; 4]).expect("a legal shape");
        let plate = SkinPlate {
            cell: IVec3::ZERO,
            anchor: IVec3::NEG_Y,
            shape,
            rotation: Quat::IDENTITY,
        };
        let plate_pose = Transform::from_translation(Vec3::new(1.0, 2.0, 3.0));
        let plate_entity = app
            .world_mut()
            .spawn((plate_body(&plate, plate_pose), ChildOf(section)))
            .id();

        let decor_collider = Vec3::new(0.3, 1.25, 0.7);
        let decor_cfg = StyleFixtureConfig {
            id: "greeble".to_string(),
            model: AssetRef::from("self://gltf/greebles/placeholder_block.glb#Scene0".to_string()),
            health: 10.0,
            collider: decor_collider,
            placement: FixturePlacement::default(),
        };
        let decor_pose = Transform::from_translation(Vec3::new(4.0, 5.0, 6.0));
        let decor_entity = app
            .world_mut()
            .spawn((decor_body(&decor_cfg, decor_pose), ChildOf(section)))
            .id();

        let original_plate_aabb = app
            .world()
            .get::<Collider>(plate_entity)
            .expect("a plate carries a collider")
            .aabb(Vec3::ZERO, Quat::IDENTITY);

        // Fixed-step timing, as `fixture`'s own shed rig uses: the first
        // `ManualDuration` frame has a zero delta, so the warm-up update below
        // is what lets the SECOND one - the kill - run exactly one tick.
        let timestep = app.world().resource::<Time<Fixed>>().timestep();
        app.insert_resource(TimeUpdateStrategy::ManualDuration(timestep));
        app.update();

        for entity in [plate_entity, decor_entity] {
            app.world_mut().trigger(HealthApplyDamage {
                entity,
                source: None,
                amount: 1000.0,
            });
        }
        app.update();

        assert!(
            app.world().get::<ChildOf>(plate_entity).is_none(),
            "the real shed path must drop the plate's ChildOf"
        );
        assert!(
            app.world().get::<ChildOf>(decor_entity).is_none(),
            "the real shed path must drop the decoration's ChildOf"
        );

        let plate_pose_before = *app.world().get::<Transform>(plate_entity).unwrap();
        let decor_pose_before = *app.world().get::<Transform>(decor_entity).unwrap();
        let plate_lifetime_before = SavedLifetime::of(app.world(), plate_entity)
            .expect("a shed fixture is a TempEntity the moment it sheds");
        let decor_lifetime_before = SavedLifetime::of(app.world(), decor_entity)
            .expect("a shed fixture is a TempEntity the moment it sheds");

        let frozen_plate = {
            let written = ron::to_string(
                &freeze_shed_fixture(app.world(), plate_entity)
                    .expect("a freshly shed plate is always settled"),
            )
            .expect("a frozen shed plate serializes");
            ron::from_str::<FrozenShedFixture>(&written).expect("and parses back")
        };
        let frozen_decor = {
            let written = ron::to_string(
                &freeze_shed_fixture(app.world(), decor_entity)
                    .expect("a freshly shed decoration is always settled"),
            )
            .expect("a frozen shed decoration serializes");
            ron::from_str::<FrozenShedFixture>(&written).expect("and parses back")
        };

        app.world_mut().despawn(plate_entity);
        app.world_mut().despawn(decor_entity);

        // The collector carries no countdown on the frozen body itself: it
        // inserts one from its own `FrozenTransient::lifetime` after the
        // thaw, the same as it does for every other resumed transient.
        let thawed_plate = {
            let mut commands = app.world_mut().commands();
            let entity = thaw_shed_fixture(&mut commands, &frozen_plate);
            commands
                .entity(entity)
                .insert(resumed_lifetime(plate_lifetime_before));
            entity
        };
        app.world_mut().flush();
        let thawed_decor = {
            let mut commands = app.world_mut().commands();
            let entity = thaw_shed_fixture(&mut commands, &frozen_decor);
            commands
                .entity(entity)
                .insert(resumed_lifetime(decor_lifetime_before));
            entity
        };
        app.world_mut().flush();

        // Collider: the plate's rebuilds to the same bounds the original wore.
        let thawed_plate_aabb = app
            .world()
            .get::<Collider>(thawed_plate)
            .expect("the thawed plate carries a collider")
            .aabb(Vec3::ZERO, Quat::IDENTITY);
        assert_eq!(thawed_plate_aabb.min, original_plate_aabb.min);
        assert_eq!(thawed_plate_aabb.max, original_plate_aabb.max);
        assert_eq!(
            app.world()
                .get::<DecorColliderSize>(thawed_decor)
                .expect("the thawed decoration carries its collider size"),
            &DecorColliderSize(decor_collider),
        );

        // Material: one of the thawed plate's dressed surfaces (top and wall
        // both dyed the same in this style; the floor keeps the engine's own
        // finish regardless of style, same as the original plate's floor
        // surface) wears the style's dye - proof the stamped `ShipStyle` was
        // there for `dress_skin_plate` to find on an ancestor-less entity.
        let materials = app.world().resource::<Assets<StandardMaterial>>();
        let wears_the_dye = app
            .world()
            .get::<Children>(thawed_plate)
            .expect("the thawed plate was dressed with surface children")
            .iter()
            .any(|child| {
                app.world()
                    .get::<MeshMaterial3d<StandardMaterial>>(child)
                    .and_then(|material| materials.get(&material.0))
                    .is_some_and(|material| material.base_color == dyed.color)
            });
        assert!(
            wears_the_dye,
            "a thawed plate with a stamped ShipStyle must dress at least one \
             surface in that style's colour"
        );

        // Lifetime and pose: the countdown a resumed transient is given
        // back, and where it was standing, both survive the RON round trip
        // unchanged.
        assert_eq!(
            SavedLifetime::of(app.world(), thawed_plate)
                .expect("the thawed plate carries its resumed countdown"),
            plate_lifetime_before
        );
        assert_eq!(
            SavedLifetime::of(app.world(), thawed_decor)
                .expect("the thawed decoration carries its resumed countdown"),
            decor_lifetime_before
        );
        assert_eq!(
            app.world().get::<Transform>(thawed_plate).unwrap(),
            &plate_pose_before
        );
        assert_eq!(
            app.world().get::<Transform>(thawed_decor).unwrap(),
            &decor_pose_before
        );

        // The shed marker comes back as a placeholder: the section it came off
        // no longer exists to name.
        assert_eq!(
            app.world()
                .get::<ShedFixtureMarker>(thawed_plate)
                .unwrap()
                .0,
            Entity::PLACEHOLDER
        );
        assert_eq!(
            app.world()
                .get::<ShedFixtureMarker>(thawed_decor)
                .unwrap()
                .0,
            Entity::PLACEHOLDER
        );

        // Neither thawed fixture can take another hit - its collider has
        // already been given up - so neither carries `Health`.
        assert!(app.world().get::<Health>(thawed_plate).is_none());
        assert!(app.world().get::<Health>(thawed_decor).is_none());
    }
}
