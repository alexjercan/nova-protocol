//! The combat transients a save keeps: what is in flight when the world is
//! saved, and how a Load brings it back.
//!
//! A save freezes every live top-level [`TempEntity`] body by its kind. A
//! body in a multi-frame process makes the save wait, and a body this module
//! has no record for panics, so no transient is dropped without a decision.
//! A body with no physics, such as a spark or a light, is visual only and is
//! not saved.
//!
//! A Load keeps the records in [`ResumedTransients`] and holds the clocks
//! ([`FreezeOwner::WorldResume`]) until every desired sector is live and every
//! saved reference resolves. It then spawns them all on one frame and
//! releases the clocks. A Load that cannot get there within
//! [`WORLD_RESUME_SECONDS_MAX`] real seconds is refused
//! ([`WorldResumeRefused`]).

use std::{collections::HashMap, time::Duration};

use avian3d::prelude::RigidBody;
use bevy::{
    ecs::{
        query::QueryFilter,
        system::{RunSystemOnce, SystemState},
    },
    prelude::*,
};
use nova_events::prelude::EntityId;
use nova_gameplay::prelude::{
    resumed_lifetime, CargoCanisterRuntimeId, CarvedChunkMarker, Clocks, DetachedPieceMarker,
    FreezeOwner, NovaBlast, RailgunSlugProjectileMarker, RoundVelocity, SavedBodyRef,
    SavedLifetime, SavedOwner, SavedSectionRef, SavedTargetRef, SpaceshipRootMarker, TempEntity,
    TorpedoProjectileMarker, TransientFreezeFault, TurretBulletProjectileMarker, UnsettledBody,
};
use nova_scenario::prelude::{
    freeze_rock_chunk, thaw_rock_chunk, AsteroidSurfaceMaterial, FrozenRockChunk,
};
use nova_ship::prelude::{
    freeze_detached_piece, freeze_round, freeze_shed_fixture, freeze_torpedo, thaw_detached_piece,
    thaw_round, thaw_shed_fixture, thaw_torpedo, FrozenDetachedPiece, FrozenRound,
    FrozenShedFixture, FrozenTorpedo, PlaceholderArt, SavedTorpedoTarget, ShedFixtureMarker,
    SkinAssets, TorpedoTargetEntity,
};
use nova_world::{
    desired_sectors, live_sectors,
    prelude::{CurrentSector, SectorCoord, WorldConfig},
    SectorRoot, SectorSnapshotError,
};
use serde::{Deserialize, Serialize};

use super::{ResumedWorld, WorldSaveSession};
use crate::{GameStyles, NovaLayeredWorld};

/// The real seconds a Load may take to bring its window and transients back
/// before it is refused. A hosted runner fills the 125-cell window at about
/// 1.8 fps, which can take more than 69 s with steady progress.
pub const WORLD_RESUME_SECONDS_MAX: f32 = 240.0;

/// One saved transient: its remaining lifetime and its body.
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FrozenTransient {
    /// The authored lifetime and what is left of it.
    pub lifetime: SavedLifetime,
    /// The body, by kind.
    pub body: FrozenTransientType,
}

/// A saved transient body, by kind.
#[derive(Debug, Serialize, Deserialize)]
pub enum FrozenTransientType {
    /// A turret round or a railgun slug.
    Round(FrozenRound),
    /// A torpedo in flight.
    Torpedo(FrozenTorpedo),
    /// A plate or a piece of decor shed off a live hull.
    ShedFixture(FrozenShedFixture),
    /// A chunk carved off a rock.
    RockChunk(FrozenRockChunk),
    /// A section that came off a destroyed ship, wearing its art.
    DetachedPiece(FrozenDetachedPiece),
}

impl FrozenTransient {
    /// # Errors
    ///
    /// A lifetime that is not finite, not positive, or has more left than
    /// its total, a pose or velocity that is not finite, a torpedo part
    /// health outside `(0, max]`, a chunk mesh that cannot be drawn, or a
    /// piece node whose parent comes after it.
    pub(crate) fn validate(&self) -> Result<(), String> {
        let SavedLifetime { total, remaining } = self.lifetime;
        if !(total.is_finite() && total > 0.0 && (0.0..=total).contains(&remaining)) {
            return Err(format!(
                "a lifetime of {remaining} s left of {total} s is not a lifetime"
            ));
        }
        match &self.body {
            FrozenTransientType::Round(round) => {
                let flight = &round.flight;
                if !(flight.translation.is_finite()
                    && flight.rotation.is_finite()
                    && flight.velocity.is_finite())
                {
                    return Err("a round's pose or velocity is not finite".to_string());
                }
            }
            FrozenTransientType::Torpedo(torpedo) => {
                if !(torpedo.translation.is_finite()
                    && torpedo.rotation.is_finite()
                    && torpedo.linear.is_finite()
                    && torpedo.angular.is_finite()
                    && torpedo.steering.is_finite())
                {
                    return Err("a torpedo's pose, velocity or steering is not finite".to_string());
                }
                let max = torpedo.config.projectile_health;
                for (part, health) in [
                    ("controller", torpedo.controller_health),
                    ("thruster", torpedo.thruster_health),
                ] {
                    if !(health.is_finite() && health > 0.0 && health <= max) {
                        return Err(format!(
                            "a torpedo's {part} health {health} is not in (0, {max}]"
                        ));
                    }
                }
            }
            FrozenTransientType::ShedFixture(shed) => {
                if !(shed.translation.is_finite()
                    && shed.rotation.is_finite()
                    && shed.linear.is_finite()
                    && shed.angular.is_finite())
                {
                    return Err("a shed fixture's pose or velocity is not finite".to_string());
                }
            }
            FrozenTransientType::RockChunk(chunk) => chunk.validate()?,
            FrozenTransientType::DetachedPiece(piece) => piece.validate()?,
        }
        Ok(())
    }
}

/// How far a Load is: the live sectors of the window it waits for, both
/// zero until the world arms. Exists while the Load holds the clocks; the
/// Loading screen shows it.
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorldResumeProgress {
    /// The desired sectors that are live.
    pub live: usize,
    /// The sectors of the window.
    pub desired: usize,
}

/// A Load that did not come back: the clocks are released and no transient
/// spawned. The menu drops the session and shows `reason` on the world's
/// Load row; nothing was written.
#[derive(Resource, Debug, Clone, PartialEq, Eq)]
pub struct WorldResumeRefused {
    /// The world folder.
    pub slug: String,
    /// What did not come back.
    pub reason: String,
}

/// The saved transients of a Load, waiting for their window. Removed when
/// they spawn, when the Load is refused, or when the session ends.
#[derive(Resource, Debug)]
pub(crate) struct ResumedTransients {
    pub(super) transients: Vec<FrozenTransient>,
    /// `Time<Real>` elapsed when the Load began.
    started: Duration,
}

/// Keep `transients` for the window this Load streams, and hold the clocks
/// until they are back.
pub(crate) fn hold_resumed_transients(world: &mut World, transients: Vec<FrozenTransient>) {
    let started = world.resource::<Time<Real>>().elapsed();
    world.insert_resource(ResumedTransients {
        transients,
        started,
    });
    // The window is not known until the world arms.
    world.insert_resource(WorldResumeProgress {
        live: 0,
        desired: 0,
    });
    world.remove_resource::<WorldResumeRefused>();
    world
        .run_system_once(|mut clocks: Clocks| clocks.hold(FreezeOwner::WorldResume))
        .expect("the clocks exist wherever a world is resumed");
}

/// Every live top-level transient body, frozen.
///
/// # Errors
///
/// [`SectorSnapshotError::Unsettled`] while a Load still waits for its saved
/// transients, while a blast resolves, and while a body is mid-process.
/// [`SectorSnapshotError::NoDurableId`] when a transient points at a live
/// body with no id.
///
/// # Panics
///
/// A top-level transient with a physics body that has no record here.
pub(crate) fn freeze_transients(
    world: &mut World,
) -> Result<Vec<FrozenTransient>, SectorSnapshotError> {
    let coord = world.resource::<CurrentSector>().0;
    let unsettled = |label: String, reason: &'static str| SectorSnapshotError::Unsettled {
        label,
        coord,
        reason: UnsettledBody { reason },
    };
    // Before anything else: a save now would write the window without the
    // transients the Load has not spawned yet, over the save that has them.
    if world.contains_resource::<ResumedTransients>() {
        return Err(unsettled(
            "the saved transients".to_string(),
            "the world is still resuming",
        ));
    }
    let bodies: Vec<(Entity, String, TransientKind)> = world
        .query_filtered::<(Entity, NameOrEntity, TransientKindQuery), (With<TempEntity>, Without<ChildOf>)>()
        .iter(world)
        .map(|(entity, name, kind)| (entity, name.to_string(), kind.kind()))
        .collect();
    // A torpedo can track another saved transient, which it names by its
    // index in the list this freeze writes: the bodies in order, less the
    // visual ones. A blast fails the whole freeze, so it shifts no index.
    let indices: HashMap<Entity, usize> = bodies
        .iter()
        .filter(|(.., kind)| *kind != TransientKind::Visual)
        .enumerate()
        .map(|(index, (entity, ..))| (*entity, index))
        .collect();
    let world: &World = world;
    let target = |target: Entity| {
        let Ok(body) = world.get_entity(target) else {
            return Ok(None);
        };
        if let Some(id) = body.get::<EntityId>() {
            Ok(Some(SavedTargetRef::Body(SavedBodyRef(id.clone()))))
        } else if let Some(&id) = body.get::<CargoCanisterRuntimeId>() {
            Ok(Some(SavedTargetRef::Canister(id)))
        } else if let Some(&index) = indices.get(&target) {
            Ok(Some(SavedTargetRef::Transient(index)))
        } else {
            Err(TransientFreezeFault::NoDurableId {
                label: body.get::<Name>().map_or_else(
                    || format!("target {target}"),
                    |name| format!("target '{name}'"),
                ),
            })
        }
    };
    let mut frozen = Vec::new();
    for (entity, label, kind) in bodies {
        let body = match kind {
            TransientKind::Visual => continue,
            TransientKind::Round => freeze_round(world, entity).map(FrozenTransientType::Round),
            TransientKind::Blast => {
                return Err(unsettled(label, "a blast is resolving"));
            }
            TransientKind::Torpedo => {
                freeze_torpedo(world, entity, &target).map(FrozenTransientType::Torpedo)
            }
            TransientKind::ShedFixture => {
                freeze_shed_fixture(world, entity).map(FrozenTransientType::ShedFixture)
            }
            TransientKind::RockChunk => {
                freeze_rock_chunk(world, entity).map(FrozenTransientType::RockChunk)
            }
            TransientKind::DetachedPiece => {
                freeze_detached_piece(world, entity).map(FrozenTransientType::DetachedPiece)
            }
            TransientKind::Unknown => panic!(
                "nova_world_base: '{label}' is a transient with a physics body and no save record"
            ),
        };
        let body = body.map_err(|fault| match fault {
            TransientFreezeFault::Unsettled(reason) => SectorSnapshotError::Unsettled {
                label: label.clone(),
                coord,
                reason,
            },
            TransientFreezeFault::NoDurableId { label: reference } => {
                SectorSnapshotError::NoDurableId {
                    label: format!("'{label}': {reference}"),
                }
            }
        })?;
        let lifetime = SavedLifetime::of(world, entity)
            .expect("a TempEntity body has its countdown from the frame it spawned");
        frozen.push(FrozenTransient { lifetime, body });
    }
    Ok(frozen)
}

/// What a top-level transient body is, for the save.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TransientKind {
    Round,
    Blast,
    Torpedo,
    ShedFixture,
    RockChunk,
    DetachedPiece,
    /// A physics body with no record.
    Unknown,
    /// No physics: a spark, a light, an effect.
    Visual,
}

#[derive(bevy::ecs::query::QueryData)]
struct TransientKindQuery {
    round: Has<TurretBulletProjectileMarker>,
    slug: Has<RailgunSlugProjectileMarker>,
    blast: Has<NovaBlast>,
    torpedo: Has<TorpedoProjectileMarker>,
    shed: Has<ShedFixtureMarker>,
    chunk: Has<CarvedChunkMarker>,
    piece: Has<DetachedPieceMarker>,
    swept: Has<RoundVelocity>,
    body: Has<avian3d::prelude::RigidBody>,
}

impl TransientKindQueryItem<'_, '_> {
    fn kind(&self) -> TransientKind {
        if self.round || self.slug {
            TransientKind::Round
        } else if self.blast {
            TransientKind::Blast
        } else if self.torpedo {
            TransientKind::Torpedo
        } else if self.shed {
            TransientKind::ShedFixture
        } else if self.chunk {
            TransientKind::RockChunk
        } else if self.piece {
            TransientKind::DetachedPiece
        } else if self.swept || self.body {
            TransientKind::Unknown
        } else {
            TransientKind::Visual
        }
    }
}

/// Why a saved id does not name one live body.
#[derive(Debug)]
enum TransientRefFault {
    /// No live body has it yet.
    Missing,
    /// More than one live body has it.
    Ambiguous(usize),
}

/// The one live body matching `filter` with the id `id`.
fn live_by_id<F: QueryFilter>(
    world: &mut World,
    id: &EntityId,
) -> Result<Entity, TransientRefFault> {
    let matches: Vec<Entity> = world
        .query_filtered::<(Entity, &EntityId), F>()
        .iter(world)
        .filter(|(_, live)| live.0 == id.0)
        .map(|(entity, _)| entity)
        .collect();
    match matches.as_slice() {
        [entity] => Ok(*entity),
        [] => Err(TransientRefFault::Missing),
        many => Err(TransientRefFault::Ambiguous(many.len())),
    }
}

/// The desired sectors of the armed window that are not live, and the size
/// of the window. `None` while the world is not armed or its saved ledger is
/// not restored yet.
fn window_still_missing(world: &mut World) -> Option<(Vec<SectorCoord>, usize)> {
    if world.contains_resource::<ResumedWorld>() {
        return None;
    }
    let radius = world
        .get_resource::<WorldConfig<NovaLayeredWorld>>()?
        .active_radius;
    let centre = world.get_resource::<CurrentSector>()?.0;
    let desired = desired_sectors(centre, radius);
    let live = live_sectors(world.query::<(Entity, &SectorRoot)>().iter(world));
    let missing = desired
        .iter()
        .filter(|coord| !live.contains_key(coord))
        .copied()
        .collect();
    Some((missing, desired.len()))
}

/// Spawn the saved transients once the exact desired window is live and
/// every saved reference resolves, then release the clocks. Past
/// [`WORLD_RESUME_SECONDS_MAX`] real seconds, refuse the Load instead.
///
/// Runs after streaming, so the sector spawns of this frame are applied.
pub(crate) fn restore_resumed_transients(world: &mut World) {
    let Some(slug) = world
        .get_resource::<WorldSaveSession>()
        .map(|session| session.folder().slug.clone())
    else {
        // The session ended under the Load: nothing is left to resume into.
        end_resume(world);
        return;
    };
    let window = window_still_missing(world);
    if let Some((missing, desired)) = &window {
        world.insert_resource(WorldResumeProgress {
            live: desired - missing.len(),
            desired: *desired,
        });
    }
    let waiting = match window {
        None => vec!["the world is not armed yet".to_string()],
        Some((missing, _)) if !missing.is_empty() => {
            let cells: Vec<String> = missing.iter().map(ToString::to_string).collect();
            vec![format!("the sectors {} are not live", cells.join(", "))]
        }
        Some(_) => match resolve_all(world) {
            Ok(refs) => {
                spawn_resumed(world, refs);
                end_resume(world);
                return;
            }
            Err(Resolve::Wait(reasons)) => reasons,
            Err(Resolve::Refuse(reason)) => {
                refuse(world, slug, reason);
                return;
            }
        },
    };
    let started = world.resource::<ResumedTransients>().started;
    let elapsed = world
        .resource::<Time<Real>>()
        .elapsed()
        .saturating_sub(started);
    if elapsed.as_secs_f32() > WORLD_RESUME_SECONDS_MAX {
        let reason = format!(
            "the world did not come back within {WORLD_RESUME_SECONDS_MAX} s: {}",
            waiting.join("; ")
        );
        refuse(world, slug, reason);
    }
}

enum Resolve {
    /// A reference does not resolve yet.
    Wait(Vec<String>),
    /// A reference can never resolve.
    Refuse(String),
}

/// The live bodies one saved transient points at.
struct ResolvedRefs {
    /// The shooter of a round or torpedo. A dead shooter, and every kind
    /// with no shooter, gets a handle that names no entity.
    owner: Entity,
    /// A torpedo's live bay.
    section: Option<Entity>,
    /// A torpedo's live tracked body or canister. A tracked transient
    /// resolves after the spawn, from the spawned list.
    target: Option<Entity>,
}

/// The live bodies every saved transient points at, in order, and the
/// render resources and styles the records need.
fn resolve_all(world: &mut World) -> Result<Vec<ResolvedRefs>, Resolve> {
    type Refs = (
        Option<SavedOwner>,
        Option<SavedSectionRef>,
        Option<SavedTargetRef>,
    );
    let transients = &world.resource::<ResumedTransients>().transients;
    let mut needs: Vec<(&'static str, bool)> = Vec::new();
    let refs: Vec<Refs> = transients
        .iter()
        .map(|transient| match &transient.body {
            FrozenTransientType::Round(round) => (Some(round.flight.owner.clone()), None, None),
            FrozenTransientType::Torpedo(torpedo) => {
                let target = match &torpedo.target {
                    SavedTorpedoTarget::Tracking { target, .. } => Some(target.clone()),
                    _ => None,
                };
                (Some(torpedo.owner.clone()), torpedo.section.clone(), target)
            }
            FrozenTransientType::ShedFixture(_)
            | FrozenTransientType::RockChunk(_)
            | FrozenTransientType::DetachedPiece(_) => (None, None, None),
        })
        .collect();
    // A saved style the pinned catalog does not have is invalid authoring
    // and refuses first; a missing render resource is this game's fault.
    let styles = world.get_resource::<GameStyles>();
    for (index, transient) in transients.iter().enumerate() {
        let style = match &transient.body {
            FrozenTransientType::ShedFixture(shed) => shed.style.as_deref(),
            FrozenTransientType::DetachedPiece(piece) => piece.style(),
            _ => None,
        };
        if let Some(style) = style {
            if styles.and_then(|styles| styles.get_style(style)).is_none() {
                return Err(Resolve::Refuse(format!(
                    "transient {index} wears the style '{style}', which is not loaded"
                )));
            }
        }
    }
    let has = |kind: fn(&FrozenTransientType) -> bool| {
        transients.iter().any(|transient| kind(&transient.body))
    };
    if has(|body| matches!(body, FrozenTransientType::RockChunk(_))) {
        needs.extend([
            ("AssetServer", world.contains_resource::<AssetServer>()),
            ("Assets<Mesh>", world.contains_resource::<Assets<Mesh>>()),
            (
                "Assets<AsteroidSurfaceMaterial>",
                world.contains_resource::<Assets<AsteroidSurfaceMaterial>>(),
            ),
        ]);
    }
    if has(|body| matches!(body, FrozenTransientType::DetachedPiece(_))) {
        needs.extend([
            ("AssetServer", world.contains_resource::<AssetServer>()),
            (
                "PlaceholderArt",
                world.contains_resource::<PlaceholderArt>(),
            ),
            ("SkinAssets", world.contains_resource::<SkinAssets>()),
            ("Assets<Mesh>", world.contains_resource::<Assets<Mesh>>()),
            (
                "Assets<StandardMaterial>",
                world.contains_resource::<Assets<StandardMaterial>>(),
            ),
        ]);
    }
    if let Some((name, _)) = needs.iter().find(|(_, present)| !present) {
        return Err(Resolve::Refuse(format!(
            "this game has no {name} to rebuild the saved transients with"
        )));
    }

    let mut resolved = Vec::with_capacity(refs.len());
    let mut waiting = Vec::new();
    for (owner, section, target) in refs {
        // An owner that died before the save thaws to a handle that names
        // no entity, as the live projectile held.
        let owner = match owner {
            Some(SavedOwner::Ship(id)) => {
                match live_by_id::<With<SpaceshipRootMarker>>(world, &id) {
                    Ok(entity) => entity,
                    Err(TransientRefFault::Missing) => {
                        waiting.push(format!("the ship '{}' is not in the window", id.0));
                        Entity::PLACEHOLDER
                    }
                    Err(TransientRefFault::Ambiguous(count)) => {
                        return Err(Resolve::Refuse(format!(
                            "{count} live ships have the id '{}'",
                            id.0
                        )));
                    }
                }
            }
            Some(SavedOwner::Gone) | None => Entity::PLACEHOLDER,
        };
        let section = match section {
            None => None,
            Some(SavedSectionRef { ship, section }) => {
                match live_by_id::<With<SpaceshipRootMarker>>(world, &ship) {
                    Ok(ship_entity) => {
                        let bays: Vec<Entity> = world
                            .query::<(Entity, &EntityId, &ChildOf)>()
                            .iter(world)
                            .filter(|(_, id, parent)| {
                                id.0 == section.0 && parent.parent() == ship_entity
                            })
                            .map(|(entity, ..)| entity)
                            .collect();
                        match bays.as_slice() {
                            [bay] => Some(*bay),
                            [] => {
                                waiting.push(format!(
                                    "the section '{}' of the ship '{}' is not live",
                                    section.0, ship.0
                                ));
                                None
                            }
                            many => {
                                return Err(Resolve::Refuse(format!(
                                    "{} live sections of the ship '{}' have the id '{}'",
                                    many.len(),
                                    ship.0,
                                    section.0
                                )));
                            }
                        }
                    }
                    Err(TransientRefFault::Missing) => {
                        waiting.push(format!("the ship '{}' is not in the window", ship.0));
                        None
                    }
                    Err(TransientRefFault::Ambiguous(count)) => {
                        return Err(Resolve::Refuse(format!(
                            "{count} live ships have the id '{}'",
                            ship.0
                        )));
                    }
                }
            }
        };
        let target = match target {
            None | Some(SavedTargetRef::Transient(_)) => None,
            Some(SavedTargetRef::Body(SavedBodyRef(id))) => {
                match live_by_id::<With<RigidBody>>(world, &id) {
                    Ok(entity) => Some(entity),
                    Err(TransientRefFault::Missing) => {
                        waiting.push(format!("the body '{}' is not in the window", id.0));
                        None
                    }
                    Err(TransientRefFault::Ambiguous(count)) => {
                        return Err(Resolve::Refuse(format!(
                            "{count} live bodies have the id '{}'",
                            id.0
                        )));
                    }
                }
            }
            Some(SavedTargetRef::Canister(id)) => {
                let canisters: Vec<Entity> = world
                    .query::<(Entity, &CargoCanisterRuntimeId)>()
                    .iter(world)
                    .filter(|(_, live)| **live == id)
                    .map(|(entity, _)| entity)
                    .collect();
                match canisters.as_slice() {
                    [canister] => Some(*canister),
                    [] => {
                        waiting.push(format!("the canister {} is not in the window", id.0));
                        None
                    }
                    many => {
                        return Err(Resolve::Refuse(format!(
                            "{} live canisters have the runtime id {}",
                            many.len(),
                            id.0
                        )));
                    }
                }
            }
        };
        resolved.push(ResolvedRefs {
            owner,
            section,
            target,
        });
    }
    if waiting.is_empty() {
        Ok(resolved)
    } else {
        waiting.sort();
        waiting.dedup();
        Err(Resolve::Wait(waiting))
    }
}

/// The render resources a thaw borrows. `resolve_all` checks that each one
/// the saved records need exists.
type ThawResources<'w, 's> = (
    Commands<'w, 's>,
    Option<Res<'w, AssetServer>>,
    Option<Res<'w, PlaceholderArt>>,
    Option<Res<'w, GameStyles>>,
    Option<ResMut<'w, SkinAssets>>,
    Option<ResMut<'w, Assets<Mesh>>>,
    Option<ResMut<'w, Assets<StandardMaterial>>>,
    Option<ResMut<'w, Assets<AsteroidSurfaceMaterial>>>,
);

/// Spawn every saved transient with its references and remaining lifetime,
/// then link each tracking torpedo to its target. Both passes queue on one
/// command buffer, applied once, so no tick sees a torpedo without its
/// target.
fn spawn_resumed(world: &mut World, refs: Vec<ResolvedRefs>) {
    let ResumedTransients { transients, .. } = world
        .remove_resource::<ResumedTransients>()
        .expect("run only with saved transients");
    let mut state = SystemState::<ThawResources>::new(world);
    let (
        mut commands,
        asset_server,
        placeholder,
        styles,
        mut skin_assets,
        mut meshes,
        mut materials,
        mut rock_materials,
    ) = state
        .get_mut(world)
        .expect("every thaw resource is an optional borrow");
    let checked = "resolve_all checks the resources the records need";
    let mut spawned = Vec::with_capacity(transients.len());
    for (transient, refs) in transients.iter().zip(&refs) {
        let entity = match &transient.body {
            FrozenTransientType::Round(round) => thaw_round(&mut commands, round, refs.owner),
            FrozenTransientType::Torpedo(torpedo) => {
                thaw_torpedo(&mut commands, torpedo, refs.owner, refs.section)
            }
            FrozenTransientType::ShedFixture(shed) => thaw_shed_fixture(&mut commands, shed),
            FrozenTransientType::RockChunk(chunk) => thaw_rock_chunk(
                &mut commands,
                meshes.as_mut().expect(checked),
                rock_materials.as_mut().expect(checked),
                asset_server.as_ref().expect(checked),
                chunk,
            ),
            FrozenTransientType::DetachedPiece(piece) => thaw_detached_piece(
                &mut commands,
                asset_server.as_ref().expect(checked),
                placeholder.as_ref().expect(checked),
                styles.as_deref(),
                skin_assets.as_mut().expect(checked),
                meshes.as_mut().expect(checked),
                materials.as_mut().expect(checked),
                piece,
            ),
        };
        commands
            .entity(entity)
            .insert(resumed_lifetime(transient.lifetime));
        spawned.push(entity);
    }
    for ((transient, refs), &torpedo) in transients.iter().zip(&refs).zip(&spawned) {
        let FrozenTransientType::Torpedo(FrozenTorpedo {
            target: SavedTorpedoTarget::Tracking { target, .. },
            ..
        }) = &transient.body
        else {
            continue;
        };
        let target = match target {
            SavedTargetRef::Transient(index) => spawned[*index],
            SavedTargetRef::Body(_) | SavedTargetRef::Canister(_) => refs
                .target
                .expect("resolve_all resolves every live target before the spawn"),
        };
        commands.entity(torpedo).insert(TorpedoTargetEntity(target));
    }
    state.apply(world);
    info!(
        "nova_world_base: the saved world is back with {} transient(s)",
        transients.len()
    );
}

/// Refuse the Load: no transient spawns, the clocks run, and the menu takes
/// the session down.
fn refuse(world: &mut World, slug: String, reason: String) {
    warn!("nova_world_base: the Load of '{slug}' is refused: {reason}");
    world.insert_resource(WorldResumeRefused { slug, reason });
    end_resume(world);
}

/// Drop what the Load kept and release its hold.
fn end_resume(world: &mut World) {
    world.remove_resource::<ResumedTransients>();
    world.remove_resource::<WorldResumeProgress>();
    world
        .run_system_once(|mut clocks: Clocks| clocks.release(FreezeOwner::WorldResume))
        .expect("the clocks exist wherever a world is resumed");
}
