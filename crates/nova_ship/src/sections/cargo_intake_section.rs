//! The cargo intake section: an accordion door that takes in drifting cargo
//! canisters and drops the player's jettisoned stock.
//!
//! Each live intake owns a trigger: a detached static avian [`Sensor`] box
//! that fills the capture gap in front of the authored collider's local -Z
//! face ([`cargo_intake_face`]), `aperture_width` by `aperture_height` across
//! it. It is not a child collider of the ship: the workspace has no collision
//! layers, so a sensor in the ship's compound would join it for mass, bullets,
//! locks and overlap reads. Weapons and sight lines already pass through every
//! `Sensor`. The fixed pass writes the trigger's pose from the ship's
//! physics pose before physics steps, and the intake's despawn takes the
//! trigger with it.
//!
//! - detection: the canister centre is in front of the face and within
//!   `detection_range` of its centre. A canister here, or a waiting drop,
//!   opens the door.
//! - take: an avian contact point has nonnegative penetration into the
//!   trigger. Speculative separated contacts do not count. Speed, rotation
//!   and the door play no part. The report is one physics step old, and a
//!   canister that closes on the face ends its step against the face, inside
//!   the trigger, so a fast head-on canister is still reported.
//!
//! The same fixed pass publishes [`CargoPickupReadiness`] for every live
//! intake/canister pair, including canisters outside detection. Each pair's
//! `ready` flag is the take decision. A pair carries no pose: the pass reads
//! raw fixed-tick poses, so a reader on the render clock recomputes the face
//! with [`cargo_intake_face`] from rendered poses. A zero-health canister is
//! destroyed rather than offered to the intake or shown as a pickup
//! candidate.
//!
//! A take moves the whole canister into the ship's [`ShipInventory`] or does
//! nothing: a canister heavier than the hold's free mass stays out. A
//! canister in two triggers goes to the first intake in entity order.
//! A jettison waits on the intake in a [`CargoIntakeEjectionQueue`]. The
//! intake drops the front canister once its door is fully open and no
//! canister is near the birth point, so the next waits until the last drifts
//! clear. The canister is born through `Commands` with its near side past
//! the trigger, moving away, so it is taken back only if it turns and enters
//! the trigger again.
//!
//! The door, the drop and the take each trigger an event
//! ([`CargoIntakeDoorMoved`], [`CargoCanisterEjected`],
//! [`CargoCanisterTaken`]) that the ship's audio voices.

use std::collections::VecDeque;

use avian3d::prelude::*;
use bevy::prelude::*;
use bevy_transform_interpolation::{RotationEasingState, TranslationEasingState};
use nova_events::units::prelude::*;
use nova_gameplay::{asset_ref::AssetRef, prelude::*};

use super::local_pose_in_root;
use crate::{physics::prelude::rigid_body_point_velocity, prelude::*};

/// The `cargo_intake_section` spawners, its face helper, config, marker,
/// ejection queue, events, the canister bundle and
/// `CargoIntakeSectionPlugin` with `CargoIntakeSystems`.
pub mod prelude {
    pub use super::{
        cargo_canister, cargo_intake_face, cargo_intake_section, preview_cargo_intake_section,
        CargoCanisterEjected, CargoCanisterTaken, CargoIntakeDoorMoved, CargoIntakeEjectionQueue,
        CargoIntakeSectionConfig, CargoIntakeSectionConfigHelper, CargoIntakeSectionMarker,
        CargoIntakeSectionPlugin, CargoIntakeSystems, CargoPickupPair, CargoPickupReadiness,
        CARGO_CANISTER_SIZE,
    };
}

/// The canister body's full side lengths in world units (10 m each): long
/// axis X. Measured off `cargo_canister_cuboid.glb`, whose end caps are the
/// widest part.
pub const CARGO_CANISTER_SIZE: Vec3 = Vec3::new(0.94, 0.58, 0.58);

/// A canister's scanner return in world units. At the default 30x
/// [`TargetingSettings::signature_range_per_unit`] a pilot can travel-lock it
/// from about 102 m, past the 50 m point-blank range of unsigned debris.
const CARGO_CANISTER_LOCK_SIGNATURE: f32 = 0.34;

/// How far past the trigger a fresh canister's near side is born, in world
/// units, so the drop starts outside it.
const CARGO_CANISTER_CLEARANCE: f32 = 0.05;

/// Authorable config for a cargo intake section. Every field is required.
#[derive(Clone, Debug, PartialEq, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CargoIntakeSectionConfig {
    /// The intake's scene: the frame and the `intake_slat_` nodes the
    /// `IntakeDoor` track folds.
    #[reflect(ignore)]
    pub render_mesh: AssetRef<WorldAsset>,
    /// Optional transform applied to the render mesh only (never the collider
    /// the volumes stand on).
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub render_mesh_transform: Option<RenderMeshTransform>,
    /// The scene of each canister this intake drops.
    #[reflect(ignore)]
    pub canister_mesh: AssetRef<WorldAsset>,
    /// The door servo, played on both edges of its travel.
    #[reflect(ignore)]
    pub door_sound: AssetRef<AudioSource>,
    /// Played as a canister leaves the door.
    #[reflect(ignore)]
    pub eject_sound: AssetRef<AudioSource>,
    /// Played as a canister is taken into the hold.
    #[reflect(ignore)]
    pub take_sound: AssetRef<AudioSource>,
    /// How far from the face centre a canister in front of the face opens the
    /// door.
    pub detection_range: Meters,
    /// The trigger's depth out from the face plane. Less than
    /// `detection_range`.
    pub capture_gap: Meters,
    /// The trigger's width: the clear opening across the face, along the
    /// section's local X, with the door fully open.
    pub aperture_width: Meters,
    /// The trigger's height: the clear opening along the section's local Y.
    pub aperture_height: Meters,
    /// The speed a dropped canister leaves the face at, relative to the
    /// intake's own point velocity.
    pub eject_speed: MetersPerSecond,
}

/// Tags a live or previewed cargo intake.
#[derive(Component, Clone, Copy, Debug, Default, Reflect)]
#[reflect(Component)]
pub struct CargoIntakeSectionMarker;

/// The intake's full config, kept on the section entity. Read-only via
/// `Deref`.
#[derive(Component, Clone, Debug, Deref, Reflect)]
pub struct CargoIntakeSectionConfigHelper(CargoIntakeSectionConfig);

/// Jettisoned canisters that wait on the intake for the door, dropped front
/// first. Already removed from the ship's inventory: they share the intake's
/// fate, as the inventory shares the ship's. Never empty: the intake removes
/// the queue as it drops the last canister.
#[derive(Component, Clone, Debug, PartialEq, Eq, Reflect)]
#[reflect(Component)]
pub struct CargoIntakeEjectionQueue(pub VecDeque<CargoCanister>);

/// An intake's door started to move: the change of its target, not of its
/// progress, as the torpedo bay's iris reports.
#[derive(EntityEvent, Clone, Copy, Debug)]
pub struct CargoIntakeDoorMoved {
    /// The intake whose door moved.
    pub entity: Entity,
    /// True when the door is folding open, false when it is closing.
    pub opening: bool,
}

/// An intake dropped a canister.
#[derive(EntityEvent, Clone, Copy, Debug)]
pub struct CargoCanisterEjected {
    /// The intake the canister left through.
    pub entity: Entity,
}

/// An intake took a canister into its ship's hold.
#[derive(EntityEvent, Clone, Copy, Debug)]
pub struct CargoCanisterTaken {
    /// The intake that took it.
    pub entity: Entity,
}

/// The last fixed intake pass's live intake/canister pairs. An absent pair is
/// not ready; preview and inactive intakes never publish pairs.
#[derive(Resource, Default, Debug)]
pub struct CargoPickupReadiness {
    /// Pairs in intake and then canister entity order.
    pub pairs: Vec<CargoPickupPair>,
}

/// One candidate measured by the same fixed pass that decides a take.
#[derive(Clone, Copy, Debug)]
pub struct CargoPickupPair {
    /// Ship carrying the intake.
    pub ship: Entity,
    /// Live intake section.
    pub intake: Entity,
    /// Live canister candidate, including candidates outside detection.
    pub canister: Entity,
    /// True exactly when this pass can take this candidate through this intake.
    pub ready: bool,
}

/// On an intake's trigger: the intake that owns it.
#[derive(Component, Debug)]
#[relationship(relationship_target = CargoIntakeTrigger)]
struct CargoIntakeTriggerOf(Entity);

/// On a live intake: its trigger, despawned with the intake.
#[derive(Component, Debug)]
#[relationship_target(relationship = CargoIntakeTriggerOf, linked_spawn)]
struct CargoIntakeTrigger(Entity);

#[derive(Component, Clone, Debug, Deref, Reflect)]
struct CargoIntakeSectionRenderMesh(#[reflect(ignore)] AssetRef<WorldAsset>);

#[derive(Component, Clone, Debug, Deref, Reflect)]
struct CargoCanisterRenderMesh(#[reflect(ignore)] AssetRef<WorldAsset>);

/// A live cargo intake.
pub fn cargo_intake_section(config: CargoIntakeSectionConfig) -> impl Bundle {
    trace!("cargo_intake_section: config {:?}", config);

    preview_cargo_intake_section(config)
}

/// The render-only half of an intake, for editor views. The runtime system
/// reads only intakes on a spaceship root, so a preview never takes or drops.
pub fn preview_cargo_intake_section(config: CargoIntakeSectionConfig) -> impl Bundle {
    trace!("preview_cargo_intake_section: config {:?}", config);

    (
        CargoIntakeSectionMarker,
        SectionClass::CargoIntake,
        SectionRenderMeshTransform(config.render_mesh_transform),
        CargoIntakeSectionRenderMesh(config.render_mesh.clone()),
        CargoIntakeSectionConfigHelper(config),
    )
}

/// A drifting cargo canister holding `canister`, at `transform` and moving at
/// `velocity` in world units per second.
pub fn cargo_canister(
    canister: CargoCanister,
    transform: Transform,
    velocity: Vec3,
    mesh: AssetRef<WorldAsset>,
) -> impl Bundle {
    (
        Name::new("Cargo Canister"),
        canister,
        transform,
        Visibility::Visible,
        RigidBody::Dynamic,
        // Avian sleeps a body under 1.5 m/s after half a second and freezes
        // it in place, so a slow drift would stop short of a trigger.
        SleepingDisabled,
        Collider::cuboid(
            CARGO_CANISTER_SIZE.x,
            CARGO_CANISTER_SIZE.y,
            CARGO_CANISTER_SIZE.z,
        ),
        ColliderDensity(1.0),
        Health::new(20.0),
        LockSignature(CARGO_CANISTER_LOCK_SIGNATURE),
        // Seeded like a launched torpedo: a body spawned mid-tick misses
        // FixedFirst, so without a `start` its first rendered frame would show
        // the raw pose while the ship it leaves renders eased.
        (
            TransformInterpolation,
            TranslationEasingState {
                start: Some(transform.translation),
                end: None,
            },
            RotationEasingState {
                start: Some(transform.rotation),
                end: None,
            },
        ),
        LinearVelocity(velocity),
        CargoCanisterRenderMesh(mesh),
    )
}

/// The centre and outward normal of the door face of an intake at `position`
/// with `rotation`: the collider's local -Z face. The centre is in the units
/// and frame of `position`; the normal is a unit vector in that frame. The
/// collider is not scaled, so the pose must carry no scale: no ship root or
/// section entity is scaled, only render-mesh children.
pub fn cargo_intake_face(
    position: Vec3,
    rotation: Quat,
    collider: SectionCollider,
) -> (Vec3, Vec3) {
    let normal = rotation * Vec3::NEG_Z;
    (position + normal * collider.aabb_half_extents().z, normal)
}

/// The world pose of the trigger of an intake at `position` with `rotation`:
/// the centre of the capture gap in front of the door face, square to it.
fn cargo_intake_trigger_pose(
    position: Vec3,
    rotation: Quat,
    collider: SectionCollider,
    config: &CargoIntakeSectionConfig,
) -> (Vec3, Quat) {
    let (face, normal) = cargo_intake_face(position, rotation, collider);
    (
        face + normal * (config.capture_gap.to_engine() * 0.5),
        rotation,
    )
}

/// Keep one trigger on every live intake on a spaceship root, posed from the
/// ship's physics pose before physics steps; despawn the trigger of an
/// intake that is inactive or off a ship.
fn sync_cargo_intake_triggers(
    mut commands: Commands,
    q_ships: Query<(&Position, &Rotation), With<SpaceshipRootMarker>>,
    q_intakes: Query<
        (
            Entity,
            Option<&ChildOf>,
            &CargoIntakeSectionConfigHelper,
            &SectionCollider,
            Has<SectionInactiveMarker>,
            Option<&CargoIntakeTrigger>,
        ),
        With<CargoIntakeSectionMarker>,
    >,
    mut q_triggers: Query<
        (&mut Position, &mut Rotation),
        (With<CargoIntakeTriggerOf>, Without<SpaceshipRootMarker>),
    >,
    q_chain: Query<(&Transform, &ChildOf)>,
) {
    for (intake, child_of, config, collider, inactive, trigger) in &q_intakes {
        let pose = child_of.filter(|_| !inactive).and_then(|&ChildOf(ship)| {
            let (position, rotation) = q_ships.get(ship).ok()?;
            let (local_position, local_rotation) = local_pose_in_root(intake, ship, &q_chain)?;
            Some(cargo_intake_trigger_pose(
                position.0 + rotation.0 * local_position,
                rotation.0 * local_rotation,
                *collider,
                config,
            ))
        });
        match (pose, trigger) {
            (Some((translation, rotation)), Some(&CargoIntakeTrigger(trigger))) => {
                let Ok((mut position, mut trigger_rotation)) = q_triggers.get_mut(trigger) else {
                    error!(
                        "sync_cargo_intake_triggers: trigger {trigger:?} of intake {intake:?} \
                         has no pose"
                    );
                    continue;
                };
                position.0 = translation;
                trigger_rotation.0 = rotation;
            }
            (Some((translation, rotation)), None) => {
                commands.spawn((
                    Name::new("Cargo Intake Trigger"),
                    CargoIntakeTriggerOf(intake),
                    RigidBody::Static,
                    Collider::cuboid(
                        config.aperture_width.to_engine(),
                        config.aperture_height.to_engine(),
                        config.capture_gap.to_engine(),
                    ),
                    Sensor,
                    Transform::from_translation(translation).with_rotation(rotation),
                ));
            }
            (None, Some(&CargoIntakeTrigger(trigger))) => {
                commands.entity(trigger).despawn();
            }
            (None, None) => {}
        }
    }
}

/// One canister as the intake pass reads it.
struct CanisterRead {
    entity: Entity,
    canister: CargoCanister,
    position: Vec3,
    taken: bool,
}

/// Run every live intake on a spaceship root: steer its door, take every
/// canister overlapping its trigger that fits the hold, and drop the front of its
/// ejection queue once the door is open and no canister is near the birth
/// point.
///
/// A missing `IntakeDoor` track counts as an open door, as a doorless torpedo
/// bay launches at once: content lint requires the track, so only a
/// code-built fixture can lack one.
fn run_cargo_intakes(
    mut commands: Commands,
    mut q_ships: Query<
        (
            &Position,
            &Rotation,
            &LinearVelocity,
            &AngularVelocity,
            &ComputedCenterOfMass,
            &mut ShipInventory,
        ),
        With<SpaceshipRootMarker>,
    >,
    mut q_intakes: Query<
        (
            Entity,
            &ChildOf,
            &CargoIntakeSectionConfigHelper,
            &SectionCollider,
            &mut SectionAnimations,
            Option<&CargoIntakeTrigger>,
            Option<&mut CargoIntakeEjectionQueue>,
        ),
        (
            With<CargoIntakeSectionMarker>,
            Without<SectionInactiveMarker>,
        ),
    >,
    q_canisters: Query<(Entity, &CargoCanister, &Position, &Health), Without<HealthZeroMarker>>,
    q_chain: Query<(&Transform, &ChildOf)>,
    collisions: Collisions,
    mut readiness: ResMut<CargoPickupReadiness>,
    mut canister_ids: ResMut<CargoCanisterIdAllocator>,
) {
    readiness.pairs.clear();
    let mut canisters: Vec<CanisterRead> = q_canisters
        .iter()
        .filter(|(.., health)| health.current > 0.0)
        .map(|(entity, canister, position, _)| CanisterRead {
            entity,
            canister: canister.clone(),
            position: position.0,
            taken: false,
        })
        .collect();
    canisters.sort_by_key(|read| read.entity);

    let mut intakes: Vec<Entity> = q_intakes.iter().map(|(entity, ..)| entity).collect();
    intakes.sort();
    for intake in intakes {
        let Ok((_, &ChildOf(ship), config, collider, mut animations, trigger, mut ejections)) =
            q_intakes.get_mut(intake)
        else {
            continue;
        };
        let Ok((position, rotation, lin_vel, ang_vel, center, mut inventory)) =
            q_ships.get_mut(ship)
        else {
            continue;
        };
        let Some((local_position, local_rotation)) = local_pose_in_root(intake, ship, &q_chain)
        else {
            error!("run_cargo_intakes: intake {intake:?} is not a descendant of ship {ship:?}");
            continue;
        };
        let intake_position = position.0 + rotation.0 * local_position;
        let intake_rotation = rotation.0 * local_rotation;
        let center_of_mass = position.0 + rotation.0 * center.0;
        let point_velocity =
            |point: Vec3| rigid_body_point_velocity(lin_vel.0, ang_vel.0, center_of_mass, point);
        let (face_centre, normal) = cargo_intake_face(intake_position, intake_rotation, *collider);
        let detection_range = config.detection_range.to_engine();

        let wanted = ejections.is_some()
            || canisters.iter().any(|read| {
                let offset = read.position - face_centre;
                !read.taken && offset.dot(normal) >= 0.0 && offset.length() <= detection_range
            });
        let target = if wanted { 1.0 } else { 0.0 };
        if let Some(was) = animations.cue_target(SectionAnimationCue::IntakeDoor) {
            if was != target {
                commands.trigger(CargoIntakeDoorMoved {
                    entity: intake,
                    opening: target > was,
                });
            }
        }
        animations.set_cue(SectionAnimationCue::IntakeDoor, target);
        let open = animations
            .cue_progress(SectionAnimationCue::IntakeDoor)
            .is_none_or(|progress| progress >= 1.0);

        for read in canisters.iter_mut().filter(|read| !read.taken) {
            let ready = trigger.is_some_and(|&CargoIntakeTrigger(trigger)| {
                collisions.get(trigger, read.entity).is_some_and(|pair| {
                    pair.manifolds.iter().any(|manifold| {
                        manifold.points.iter().any(|point| point.penetration >= 0.0)
                    })
                })
            }) && inventory.free_g() >= read.canister.total_mass_g();
            readiness.pairs.push(CargoPickupPair {
                ship,
                intake,
                canister: read.entity,
                ready,
            });
            if !ready {
                continue;
            }
            for (item, count) in read.canister.stacks() {
                inventory.add(item, count);
            }
            commands.entity(read.entity).despawn();
            commands.trigger(CargoCanisterTaken { entity: intake });
            read.taken = true;
        }

        let birth = face_centre
            + normal
                * (CARGO_CANISTER_SIZE.z * 0.5
                    + config.capture_gap.to_engine()
                    + CARGO_CANISTER_CLEARANCE);
        // Two canisters whose centres are a full diagonal apart cannot overlap.
        let birth_clear = canisters.iter().all(|read| {
            read.taken || read.position.distance(birth) >= CARGO_CANISTER_SIZE.length()
        });
        if let Some(queue) = ejections.as_mut().filter(|_| open && birth_clear) {
            let canister = queue
                .0
                .pop_front()
                .expect("a CargoIntakeEjectionQueue is never empty");
            if queue.0.is_empty() {
                commands.entity(intake).remove::<CargoIntakeEjectionQueue>();
            }
            let velocity = point_velocity(birth) + normal * config.eject_speed.to_engine();
            commands.spawn((
                cargo_canister(
                    canister,
                    Transform::from_translation(birth).with_rotation(intake_rotation),
                    velocity,
                    config.canister_mesh.clone(),
                ),
                canister_ids.next(),
            ));
            commands.trigger(CargoCanisterEjected { entity: intake });
        }
    }
}

/// Spawn an intake's scene.
fn insert_cargo_intake_section_render(
    add: On<Add, CargoIntakeSectionMarker>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    q_intakes: Query<(&CargoIntakeSectionRenderMesh, &SectionRenderMeshTransform)>,
) {
    let entity = add.entity;
    let Ok((render_mesh, render_mesh_transform)) = q_intakes.get(entity) else {
        error!("insert_cargo_intake_section_render: entity {entity:?} not found in q_intakes");
        return;
    };
    let transform = render_mesh_transform
        .map(RenderMeshTransform::to_transform)
        .unwrap_or_default();
    commands.entity(entity).insert(children![(
        Name::new("Cargo Intake Section Body"),
        transform,
        SectionRenderOf(entity),
        WorldAssetRoot(render_mesh.resolve(&asset_server)),
    )]);
}

fn despawn_destroyed_canister(
    add: On<Add, HealthZeroMarker>,
    mut commands: Commands,
    canisters: Query<(), With<CargoCanister>>,
) {
    if canisters.contains(add.entity) {
        commands.entity(add.entity).despawn();
    }
}

/// Spawn a canister's scene.
fn insert_cargo_canister_render(
    add: On<Add, CargoCanister>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    q_canisters: Query<&CargoCanisterRenderMesh>,
) {
    let entity = add.entity;
    let Ok(render_mesh) = q_canisters.get(entity) else {
        error!("insert_cargo_canister_render: entity {entity:?} not found in q_canisters");
        return;
    };
    commands.entity(entity).insert(children![(
        Name::new("Cargo Canister Body"),
        WorldAssetRoot(render_mesh.resolve(&asset_server)),
    )]);
}

/// System set for the trigger sync and the intake pass, on the fixed clock
/// with the other section systems, so a trigger pose, a take and a drop land
/// before physics steps.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct CargoIntakeSystems;

/// Adds cargo intakes: their triggers, the door driver, takes and drops,
/// readiness, and (when `render`) the intake and canister scenes.
#[derive(Default)]
pub struct CargoIntakeSectionPlugin {
    /// Whether the render-side half is added (false on headless servers).
    pub render: bool,
}

impl Plugin for CargoIntakeSectionPlugin {
    fn build(&self, app: &mut App) {
        trace!("CargoIntakeSectionPlugin: build");

        app.register_type::<CargoIntakeSectionMarker>();
        app.register_type::<CargoIntakeEjectionQueue>();
        app.register_type::<CargoCanister>();
        app.init_resource::<CargoPickupReadiness>();
        app.add_observer(despawn_destroyed_canister);

        app.configure_sets(
            FixedUpdate,
            CargoIntakeSystems.in_set(super::SpaceshipSectionSystems),
        );
        app.add_systems(
            FixedUpdate,
            (sync_cargo_intake_triggers, run_cargo_intakes)
                .chain()
                .in_set(CargoIntakeSystems),
        );

        if self.render {
            app.add_observer(insert_cargo_intake_section_render);
            app.add_observer(insert_cargo_canister_render);
        }
    }
}

#[cfg(test)]
mod tests;
