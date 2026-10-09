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

use avian3d::prelude::Collider;
use bevy::prelude::*;
use nova_gameplay::prelude::{
    AssetRef, Health, HealthZeroMarker, IntegrityDestroyMarker, IntegrityDisabledMarker,
    SectionInactiveMarker, UnsettledBody,
};

use super::{
    ammo::prelude::{SectionAmmo, SectionReload, SuspendedSectionAmmo},
    cargo_intake_section::prelude::CargoIntakeEjectionQueue,
    railgun_section::prelude::RailgunCharge,
    section_animation::prelude::{
        FrozenSectionAnimations, SectionAnimationRigDirty, SectionAnimations,
    },
    shell_shape::prelude::ShellShape,
    shell_skin::{frozen_plate_body, ShipSkinMarker},
    skin_decor::{frozen_decor_body, ShipDecorMarker},
    turret_section::prelude::{FrozenTurretHinges, TurretStow},
};

/// `FrozenSection`, `freeze_section` and `thaw_section`.
pub mod prelude {
    pub use super::{freeze_section, thaw_section, FrozenFixture, FrozenSection};
}

/// A fixture still bolted on when its body froze: a skin plate on a section,
/// or a decoration on a plate.
///
/// Recursive: a plate's own `children` is where its frozen decor lives, so
/// one [`FrozenSection::fixtures`] walk captures a whole cladding tree.
#[derive(Clone, Debug)]
pub struct FrozenFixture {
    kind: FrozenFixtureKind,
    pose: Transform,
    health: Health,
    collider: Collider,
    children: Vec<FrozenFixture>,
}

#[derive(Clone, Debug)]
enum FrozenFixtureKind {
    Plate(ShellShape),
    Decor {
        name: String,
        model: AssetRef<WorldAsset>,
    },
}

/// A section's durable runtime state when its body froze.
#[derive(Clone, Debug)]
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
/// piece of decor, or is missing the `Transform`/`Health`/`Collider` every
/// fixture of either kind carries - callers only ever reach this through
/// [`is_fixture`]'s filter, so that would mean a fixture body changed shape
/// out from under this module.
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
        FrozenFixtureKind::Decor {
            name: name.as_str().to_string(),
            model: model.clone(),
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
    let Some(collider) = world.get::<Collider>(entity) else {
        panic!("freeze_fixture: fixture {entity:?} carries no Collider");
    };

    Ok(FrozenFixture {
        kind,
        pose: *pose,
        health: health.clone(),
        collider: collider.clone(),
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
        FrozenFixtureKind::Plate(shape) => parent.spawn(frozen_plate_body(
            shape,
            frozen.pose,
            frozen.health,
            frozen.collider,
        )),
        FrozenFixtureKind::Decor { name, model } => parent.spawn(frozen_decor_body(
            name,
            model,
            frozen.pose,
            frozen.health,
            frozen.collider,
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
