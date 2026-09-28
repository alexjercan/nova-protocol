//! The cargo intake section: an accordion door that takes in drifting cargo
//! canisters and drops the player's jettisoned stock.
//!
//! The intake's volumes are geometry, not colliders. The workspace has no
//! collision layers, so a sensor on the ship body would join the ship's
//! compound for bullets, locks and overlap reads. Instead each tick the
//! intake measures every canister in its own frame against the authored
//! collider's local -Z face ([`cargo_intake_zone`]):
//!
//! - detection: the canister centre is in front of the face and within
//!   `detection_range` of its centre. A canister here opens the door.
//! - capture: the canister's nearest side is at most `capture_gap` from the
//!   face plane, and its rotated footprint fits the aperture less
//!   [`CARGO_APERTURE_MARGIN`] on every side. A slow canister here that is
//!   not moving away from the face is taken whole once the door is fully
//!   open.
//!
//! The capture gap is the no-impact rule: a canister that closes under the
//! capture speed is taken before it can touch the face. Avian computes a
//! contact only within one step's relative travel (at least its contact
//! tolerance), which at the capture speed limit is a few centimeters, far
//! inside the gap. A faster canister crosses the gap and hits the ship. The
//! intake refuses a canister while it touches any part of its own ship, and
//! while it moves away, so a canister slowed by an impact is taken only once
//! it separates and closes on the door again.
//!
//! A take moves the whole canister into the ship's [`ShipInventory`] or does
//! nothing: a canister with more items than the hold has room for stays out.
//! A jettison waits on the intake as a [`CargoIntakeEjection`] until the door
//! is fully open and no canister is near the birth point. The canister is
//! born through `Commands`, so the tick that drops it cannot see it, and with
//! its near side past the capture gap, moving away. From the next tick any
//! intake takes it back the moment it closes on an open door.
//!
//! The door, the drop and the take each trigger an event
//! ([`CargoIntakeDoorMoved`], [`CargoCanisterEjected`],
//! [`CargoCanisterTaken`]) that the ship's audio voices.

use avian3d::prelude::*;
use bevy::prelude::*;
use bevy_transform_interpolation::{RotationEasingState, TranslationEasingState};
use nova_events::units::prelude::*;
use nova_gameplay::{asset_ref::AssetRef, prelude::*};

use super::local_pose_in_root;
use crate::{physics::prelude::rigid_body_point_velocity, prelude::*};

/// The `cargo_intake_section` spawners, its config, marker, pending ejection,
/// events, the canister bundle and `CargoIntakeSectionPlugin` with
/// `CargoIntakeSystems`.
pub mod prelude {
    pub use super::{
        cargo_canister, cargo_intake_section, preview_cargo_intake_section, CargoCanisterEjected,
        CargoCanisterTaken, CargoIntakeDoorMoved, CargoIntakeEjection, CargoIntakeSectionConfig,
        CargoIntakeSectionConfigHelper, CargoIntakeSectionMarker, CargoIntakeSectionPlugin,
        CargoIntakeSystems, CARGO_APERTURE_MARGIN, CARGO_CANISTER_SIZE,
    };
}

/// The canister body's full side lengths in world units (10 m each): long
/// axis X. Measured off `cargo_canister_cuboid.glb`, whose end caps are the
/// widest part.
pub const CARGO_CANISTER_SIZE: Vec3 = Vec3::new(0.94, 0.58, 0.58);

/// The clear space a canister's rotated footprint keeps from each edge of the
/// aperture for a take: a tumbling canister's footprint changes between
/// ticks, and the take must stay off the frame.
pub const CARGO_APERTURE_MARGIN: Meters = Meters(0.5);

/// How far past the capture gap a fresh canister's near side is born, in
/// world units, so the drop starts outside the capture volume.
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
    /// the volumes are measured from).
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
    /// How near the face plane a canister's nearest side must come to be
    /// taken. Less than `detection_range`.
    pub capture_gap: Meters,
    /// The clear opening across the face, along the section's local X, with
    /// the door fully open. A canister's footprint must fit it to be taken.
    pub aperture_width: Meters,
    /// The clear opening across the face, along the section's local Y.
    pub aperture_height: Meters,
    /// The fastest a canister may move relative to the intake's own point
    /// velocity and still be taken.
    pub maximum_capture_speed: MetersPerSecond,
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

/// A jettisoned stack that waits on the intake for the door. Already removed
/// from the ship's inventory: it shares the intake's fate, as the inventory
/// shares the ship's. One per intake.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, Reflect)]
#[reflect(Component)]
pub struct CargoIntakeEjection(pub CargoCanister);

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
        Collider::cuboid(
            CARGO_CANISTER_SIZE.x,
            CARGO_CANISTER_SIZE.y,
            CARGO_CANISTER_SIZE.z,
        ),
        ColliderDensity(1.0),
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

/// Where a canister is relative to one intake.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CargoIntakeZoneType {
    /// Centre behind the face plane or past the detection range.
    Outside,
    /// Centre in front of the face and within the detection range, not in
    /// capture.
    Detection,
    /// Nearest side within the capture gap and footprint inside the aperture
    /// less its margin.
    Capture,
}

/// The zone of a canister at `centre` with `rotation`, both intake-local in
/// world units, for an intake whose collider has `half_extents` and whose
/// door is its local -Z face. `aperture` is the half opening less the margin.
fn cargo_intake_zone(
    centre: Vec3,
    rotation: Quat,
    half_extents: Vec3,
    detection_range: f32,
    capture_gap: f32,
    aperture: Vec2,
) -> CargoIntakeZoneType {
    let face = Vec3::new(0.0, 0.0, -half_extents.z);
    let depth = face.z - centre.z;
    if depth < 0.0 {
        return CargoIntakeZoneType::Outside;
    }
    // The canister box's half-extent along a local axis, from its rotation.
    let half = CARGO_CANISTER_SIZE * 0.5;
    let extent = |axis: Vec3| {
        (rotation * Vec3::X).dot(axis).abs() * half.x
            + (rotation * Vec3::Y).dot(axis).abs() * half.y
            + (rotation * Vec3::Z).dot(axis).abs() * half.z
    };
    let gap = depth - extent(Vec3::Z);
    if gap <= capture_gap
        && centre.x.abs() + extent(Vec3::X) <= aperture.x
        && centre.y.abs() + extent(Vec3::Y) <= aperture.y
    {
        return CargoIntakeZoneType::Capture;
    }
    if centre.distance(face) <= detection_range {
        return CargoIntakeZoneType::Detection;
    }
    CargoIntakeZoneType::Outside
}

/// One canister as the intake pass reads it.
struct CanisterRead {
    entity: Entity,
    canister: CargoCanister,
    position: Vec3,
    rotation: Quat,
    velocity: Vec3,
    taken: bool,
}

/// Run every live intake on a spaceship root: steer its door, take slow,
/// closing canisters that do not touch the ship through a fully open door,
/// and drop a pending ejection once the door is open and no canister is near
/// the birth point.
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
            Option<&CargoIntakeEjection>,
        ),
        (
            With<CargoIntakeSectionMarker>,
            Without<SectionInactiveMarker>,
        ),
    >,
    q_canisters: Query<(
        Entity,
        &CargoCanister,
        &Position,
        &Rotation,
        &LinearVelocity,
    )>,
    q_chain: Query<(&Transform, &ChildOf)>,
    collisions: Collisions,
) {
    let mut canisters: Vec<CanisterRead> = q_canisters
        .iter()
        .map(
            |(entity, canister, position, rotation, velocity)| CanisterRead {
                entity,
                canister: *canister,
                position: position.0,
                rotation: rotation.0,
                velocity: velocity.0,
                taken: false,
            },
        )
        .collect();
    canisters.sort_by_key(|read| read.entity);

    let mut intakes: Vec<Entity> = q_intakes.iter().map(|(entity, ..)| entity).collect();
    intakes.sort();
    for intake in intakes {
        let Ok((_, &ChildOf(ship), config, collider, mut animations, ejection)) =
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
        let to_local = |world: Vec3| intake_rotation.inverse() * (world - intake_position);
        let center_of_mass = position.0 + rotation.0 * center.0;
        let point_velocity =
            |point: Vec3| rigid_body_point_velocity(lin_vel.0, ang_vel.0, center_of_mass, point);

        let half_extents = collider.aabb_half_extents();
        let detection_range = config.detection_range.to_engine();
        let capture_gap = config.capture_gap.to_engine();
        let margin = CARGO_APERTURE_MARGIN.to_engine();
        let aperture = Vec2::new(
            config.aperture_width.to_engine() * 0.5 - margin,
            config.aperture_height.to_engine() * 0.5 - margin,
        );
        let maximum_speed = config.maximum_capture_speed.to_engine();
        let zones: Vec<CargoIntakeZoneType> = canisters
            .iter()
            .map(|read| {
                if read.taken {
                    CargoIntakeZoneType::Outside
                } else {
                    cargo_intake_zone(
                        to_local(read.position),
                        intake_rotation.inverse() * read.rotation,
                        half_extents,
                        detection_range,
                        capture_gap,
                        aperture,
                    )
                }
            })
            .collect();

        let wanted = ejection.is_some()
            || zones
                .iter()
                .any(|zone| *zone != CargoIntakeZoneType::Outside);
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

        let normal = intake_rotation * Vec3::NEG_Z;
        if open {
            for (read, zone) in canisters.iter_mut().zip(&zones) {
                if *zone != CargoIntakeZoneType::Capture {
                    continue;
                }
                let relative = read.velocity - point_velocity(read.position);
                // Any contact impulse of the last physics step came from a
                // manifold, and avian flags every pair with a manifold as
                // touching, speculative ones too. The flags persist until the
                // next step, after this pass.
                let touching_ship = collisions
                    .collisions_with(read.entity)
                    .any(|pair| pair.body1 == Some(ship) || pair.body2 == Some(ship));
                // A refused canister stays drifting with its tag. One that hit
                // the ship is taken only once it separates and closes on the
                // door again.
                if touching_ship
                    || relative.length() > maximum_speed
                    || relative.dot(normal) > 0.0
                    || inventory.free() < read.canister.count
                {
                    continue;
                }
                inventory.add(read.canister.item, read.canister.count);
                commands.entity(read.entity).despawn();
                commands.trigger(CargoCanisterTaken { entity: intake });
                read.taken = true;
            }
        }

        let face_centre = intake_position + normal * half_extents.z;
        let birth = face_centre
            + normal * (CARGO_CANISTER_SIZE.z * 0.5 + capture_gap + CARGO_CANISTER_CLEARANCE);
        // Two canisters whose centres are a full diagonal apart cannot overlap.
        let birth_clear = canisters.iter().all(|read| {
            read.taken || read.position.distance(birth) >= CARGO_CANISTER_SIZE.length()
        });
        let pending = ejection.map(|&CargoIntakeEjection(canister)| canister);
        if let Some(canister) = pending.filter(|_| open && birth_clear) {
            let velocity = point_velocity(birth) + normal * config.eject_speed.to_engine();
            commands.entity(intake).remove::<CargoIntakeEjection>();
            commands.spawn(cargo_canister(
                canister,
                Transform::from_translation(birth).with_rotation(intake_rotation),
                velocity,
                config.canister_mesh.clone(),
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

/// System set for the intake pass, on the fixed clock with the other section
/// systems, so a take and a drop land before physics steps.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct CargoIntakeSystems;

/// Adds cargo intakes: the door driver, takes and drops, and (when `render`)
/// the intake and canister scenes.
#[derive(Default)]
pub struct CargoIntakeSectionPlugin {
    /// Whether the render-side half is added (false on headless servers).
    pub render: bool,
}

impl Plugin for CargoIntakeSectionPlugin {
    fn build(&self, app: &mut App) {
        trace!("CargoIntakeSectionPlugin: build");

        app.register_type::<CargoIntakeSectionMarker>();
        app.register_type::<CargoIntakeEjection>();
        app.register_type::<CargoCanister>();

        app.configure_sets(
            FixedUpdate,
            CargoIntakeSystems.in_set(super::SpaceshipSectionSystems),
        );
        app.add_systems(FixedUpdate, run_cargo_intakes.in_set(CargoIntakeSystems));

        if self.render {
            app.add_observer(insert_cargo_intake_section_render);
            app.add_observer(insert_cargo_canister_render);
        }
    }
}

#[cfg(test)]
mod tests;
