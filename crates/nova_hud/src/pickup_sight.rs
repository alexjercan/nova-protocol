//! The pickup sight: a face plate on the player's cargo intake and a line
//! reaching one cargo canister near the player's ship, drawn in world space
//! the way [`docking_sight`](super::docking_sight) draws its own pair -
//! alignment you fly INTO rather than a number you read.
//!
//! # Where the numbers come from
//!
//! [`CargoPickupReadiness`], the same fixed pass that decides a take
//! (`run_cargo_intakes` in `nova_ship::sections::cargo_intake_section`). It
//! publishes every live intake/canister pair, in or out of range. The
//! sight reads those published pairs instead of computing a separate gate.
//! A pair is trusted only while both ends still validate live (the canister
//! with positive [`Health::current`], the intake with
//! [`CargoIntakeSectionMarker`], without [`SectionInactiveMarker`], and still
//! parented to the same ship the pair names): `CargoPickupReadiness` is a
//! snapshot from the last fixed pass and `Update` runs between fixed ticks. A
//! take despawns the canister in the same fixed pass that marks it ready, so
//! the sight clears with it; the take sound, not the sight, confirms the
//! take.
//!
//! Every pose the sight draws or measures is the interpolated
//! `GlobalTransform` of the ship, the intake and the canister, never the raw
//! fixed-tick pose: the sight is drawn over the rendered models, so the face
//! comes from [`cargo_intake_face`] on the intake's rendered pose.
//!
//! # Which pair is drawn
//!
//! Only pairs whose canister lies within [`CARGO_PICKUP_SIGHT_RANGE`] of the
//! player's ship, so the line shows before a lock and before the intake's
//! own detection range. A travel-locked canister in range is drawn first; a
//! lock on anything else - a beacon, a ship, a dead canister, a canister out
//! of range - leaves the pick to distance. The pair with the nearer intake
//! face to its canister wins, then the lower entity IDs. That pick is a
//! display convenience only: the intake triggers and hold room, not
//! distance, decide which intake actually takes which canister.

use bevy::{light::NotShadowCaster, prelude::*};
use nova_events::units::prelude::*;
use nova_gameplay::prelude::*;
use nova_ship::prelude::*;

use super::holo_instruments::segment_transform;

/// `PickupSightPlugin` and the component its parts carry.
pub mod prelude {
    pub use super::{PickupSightPart, PickupSightPlugin};
}

/// How far from the player's ship a canister draws the sight: past the base
/// intake's 40 m detection range, so the line guides the approach before
/// the door opens and without a lock.
const CARGO_PICKUP_SIGHT_RANGE: Meters = Meters(200.0);

/// Arm span of the intake face plate, world units. The canister's own
/// footprint sets the scale: [`CargoPickupReadiness`] does not publish the
/// aperture, so the plate is sized to the thing being aligned, not the slot
/// it goes through.
const PLATE_SPAN: f32 = CARGO_CANISTER_SIZE.x * 1.5;

/// How far off the intake face a plate floats, world units - clear of the
/// hull plating it is drawn on, the docking sight's own plate lift.
const PLATE_LIFT: f32 = 0.05;

/// Line and plate tube radius, world units.
const LINE_RADIUS: f32 = 0.03;

/// Opacity of every part of the sight.
const SIGHT_ALPHA: f32 = 0.85;

/// One drawn part of the sight, and its IDENTITY for the reconcile pass: a
/// part is spawned when it starts being drawn, despawned when it stops, and
/// only re-posed in between.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, Reflect)]
#[reflect(Component)]
pub enum PickupSightPart {
    /// One arm of the cross drawn across the intake's face.
    Plate {
        /// Which arm of the cross.
        arm: u8,
    },
    /// The line from the intake face to the canister.
    Line,
}

/// Shared mesh and material, built on the first frame a sight is drawn. A
/// `Resource` rather than a `Local`, so an app that never draws a sight
/// never allocates them.
#[derive(Resource, Default)]
struct PickupSightAssets {
    line_mesh: Option<Handle<Mesh>>,
    line_material: Option<Handle<StandardMaterial>>,
}

impl PickupSightAssets {
    fn line_mesh(&mut self, meshes: &mut Assets<Mesh>) -> Handle<Mesh> {
        self.line_mesh
            .get_or_insert_with(|| meshes.add(Cylinder::new(LINE_RADIUS, 1.0)))
            .clone()
    }

    /// Nav cyan throughout: a take despawns the canister in the same fixed
    /// pass that marks its pair ready, so no frame ever draws a ready pair.
    fn line_material(
        &mut self,
        materials: &mut Assets<StandardMaterial>,
    ) -> Handle<StandardMaterial> {
        self.line_material
            .get_or_insert_with(|| {
                materials.add(StandardMaterial {
                    base_color: crate::NAV_CYAN.with_alpha(SIGHT_ALPHA),
                    alpha_mode: AlphaMode::Blend,
                    unlit: true,
                    ..default()
                })
            })
            .clone()
    }
}

/// The two in-plane directions of a plate: `reference` flattened onto the
/// face and the axis crossed with it.
///
/// `None` when the reference lies along the axis and leaves nothing to
/// flatten - the caller tries the other reference rather than drawing a
/// cross with an arbitrary clocking.
fn plate_arms(axis: Vec3, reference: Vec3) -> Option<(Vec3, Vec3)> {
    let flattened = (reference - axis * reference.dot(axis)).try_normalize()?;
    Some((flattened, axis.cross(flattened)))
}

/// One part to draw this frame.
struct Drawn {
    part: PickupSightPart,
    pose: Transform,
}

/// Lay out the whole sight for a live pair. World up
/// (falling back to world forward) is the plate's reference, not the player
/// hull's own: unlike
/// the docking sight this plate has only one end, so there is no shared
/// clocking for it to agree with.
fn draw_sight(face: Vec3, normal: Vec3, canister_position: Vec3) -> Vec<Drawn> {
    let mut drawn = Vec::new();

    let Some((first_arm, second_arm)) =
        plate_arms(normal, Vec3::Y).or_else(|| plate_arms(normal, Vec3::Z))
    else {
        return drawn;
    };
    let centre = face + normal * PLATE_LIFT;
    for (arm, direction) in [(0_u8, first_arm), (1, second_arm)] {
        let reach = direction * PLATE_SPAN * 0.5;
        drawn.push(Drawn {
            part: PickupSightPart::Plate { arm },
            pose: segment_transform(centre - reach, centre + reach),
        });
    }

    drawn.push(Drawn {
        part: PickupSightPart::Line,
        pose: segment_transform(face, canister_position),
    });
    drawn
}

/// The intake face, its outward normal and the canister centre to draw for
/// `ship` rendered at `ship_position`, all from rendered poses: among
/// published pairs whose ends still validate live and whose rendered
/// canister lies within [`CARGO_PICKUP_SIGHT_RANGE`] of the ship, a pair
/// naming `locked` first, else any pair. Among the chosen set, the nearest
/// rendered intake face to its canister wins, then the lower canister and
/// intake entity IDs. `pairs` is a snapshot from the last fixed pass and can
/// still name a canister that has died or an intake that has gone inactive,
/// been despawned, or been reparented off `ship`.
fn select_pair(
    pairs: &[CargoPickupPair],
    ship: Entity,
    ship_position: Vec3,
    locked: Option<Entity>,
    q_live_canisters: &Query<
        (&Health, &GlobalTransform),
        (With<CargoCanister>, Without<HealthZeroMarker>),
    >,
    q_live_intakes: &Query<
        (&ChildOf, &GlobalTransform, &SectionCollider),
        (
            With<CargoIntakeSectionMarker>,
            Without<SectionInactiveMarker>,
        ),
    >,
) -> Option<(Vec3, Vec3, Vec3)> {
    let range = CARGO_PICKUP_SIGHT_RANGE.to_engine();
    let candidates = pairs.iter().filter_map(|pair| {
        if pair.ship != ship {
            return None;
        }
        let (health, canister_pose) = q_live_canisters.get(pair.canister).ok()?;
        let (&ChildOf(parent), intake_pose, &collider) = q_live_intakes.get(pair.intake).ok()?;
        let canister_position = canister_pose.translation();
        if health.current <= 0.0
            || parent != ship
            || canister_position.distance_squared(ship_position) > range * range
        {
            return None;
        }
        let (face, normal) =
            cargo_intake_face(intake_pose.translation(), intake_pose.rotation(), collider);
        Some((pair, face, normal, canister_position))
    });
    let candidates: Vec<_> = candidates.collect();
    let nearest = |locked: Option<Entity>| {
        candidates
            .iter()
            .filter(|(pair, ..)| locked.is_none_or(|locked| pair.canister == locked))
            .min_by(|(a, a_face, _, a_canister), (b, b_face, _, b_canister)| {
                a_face
                    .distance_squared(*a_canister)
                    .total_cmp(&b_face.distance_squared(*b_canister))
                    .then(a.canister.cmp(&b.canister))
                    .then(a.intake.cmp(&b.intake))
            })
            .map(|&(_, face, normal, canister)| (face, normal, canister))
    };
    locked
        .and_then(|locked| nearest(Some(locked)))
        .or_else(|| nearest(None))
}

/// Reconcile live readiness against the player's ship and lock.
fn sync_pickup_sight(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut assets: ResMut<PickupSightAssets>,
    readiness: Res<CargoPickupReadiness>,
    q_player: Query<
        (Entity, &GlobalTransform, Option<&TravelLock>),
        (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>),
    >,
    q_live_canisters: Query<
        (&Health, &GlobalTransform),
        (With<CargoCanister>, Without<HealthZeroMarker>),
    >,
    q_live_intakes: Query<
        (&ChildOf, &GlobalTransform, &SectionCollider),
        (
            With<CargoIntakeSectionMarker>,
            Without<SectionInactiveMarker>,
        ),
    >,
    mut q_parts: Query<(Entity, &PickupSightPart, &mut Transform)>,
) {
    let drawn = q_player
        .single()
        .ok()
        .and_then(|(ship, transform, travel)| {
            let (face, normal, canister_position) = select_pair(
                &readiness.pairs,
                ship,
                transform.translation(),
                travel.and_then(|travel| travel.0),
                &q_live_canisters,
                &q_live_intakes,
            )?;
            Some(draw_sight(face, normal, canister_position))
        })
        .unwrap_or_default();

    for (entity, part, mut transform) in &mut q_parts {
        match drawn.iter().find(|d| d.part == *part) {
            Some(d) => *transform = d.pose,
            None => commands.entity(entity).despawn(),
        }
    }

    for d in &drawn {
        if q_parts.iter().any(|(_, part, ..)| *part == d.part) {
            continue;
        }
        let mesh = assets.line_mesh(&mut meshes);
        let material = assets.line_material(&mut materials);
        commands.spawn((
            Name::new("Pickup Sight"),
            // HUD-managed like every other world-space instrument, so the
            // hide-HUD toggle and a cinematic take it away with the rest.
            crate::HudTier::Instrument,
            d.part,
            Mesh3d(mesh),
            MeshMaterial3d(material),
            d.pose,
            NotShadowCaster,
        ));
    }
}

/// Draws the world-space pickup sight for one canister near the player's
/// ship from [`CargoPickupReadiness`]. Inits `PickupSightAssets`, registers
/// [`PickupSightPart`], and runs `sync_pickup_sight` in `Update` within
/// [`super::NovaHudSystems`].
#[derive(Default)]
pub struct PickupSightPlugin;

impl Plugin for PickupSightPlugin {
    fn build(&self, app: &mut App) {
        trace!("PickupSightPlugin: build");

        app.init_resource::<PickupSightAssets>();
        app.register_type::<PickupSightPart>();
        app.add_systems(Update, sync_pickup_sight.in_set(super::NovaHudSystems));
    }
}

#[cfg(test)]
mod tests {
    use avian3d::prelude::*;
    use nova_gameplay::{asset_ref::AssetRef, test_support::unfinished_integrity_physics_app};

    use super::*;

    /// A physics app running only the sight. The readiness resource is
    /// inserted directly by each test rather than through
    /// `CargoIntakeSectionPlugin`: the sight only ever reads what is
    /// published, so its tests stay focused on that contract.
    fn sight_app() -> App {
        let mut app = unfinished_integrity_physics_app();
        app.init_asset::<StandardMaterial>();
        app.insert_resource(CargoPickupReadiness::default());
        app.add_plugins(PickupSightPlugin);
        app.finish();
        app
    }

    fn spawn_player(app: &mut App, at: Vec3) -> Entity {
        app.world_mut()
            .spawn((
                Name::new("player"),
                SpaceshipRootMarker,
                PlayerSpaceshipMarker,
                Transform::from_translation(at),
                GlobalTransform::from(Transform::from_translation(at)),
            ))
            .id()
    }

    /// The collider of every fixture intake: its door face is 0.5 world
    /// units in front of its centre, along its local -Z.
    const INTAKE_COLLIDER: SectionCollider = SectionCollider::Cuboid {
        size: Vec3::new(3.0, 2.0, 1.0),
    };

    /// A live intake entity, parented to `ship`, carrying exactly the
    /// components `select_pair` reads, and rendered facing -Z with its door
    /// face centred on `face`. `ship` must already carry its rendered pose.
    fn spawn_intake(app: &mut App, ship: Entity, active: bool, face: Vec3) -> Entity {
        let ship_at = app
            .world()
            .get::<GlobalTransform>(ship)
            .expect("the ship has a rendered pose")
            .translation();
        let at = face + Vec3::Z * INTAKE_COLLIDER.aabb_half_extents().z;
        let mut entity = app.world_mut().spawn((
            Name::new("intake"),
            ChildOf(ship),
            CargoIntakeSectionMarker,
            INTAKE_COLLIDER,
            Transform::from_translation(at - ship_at),
            GlobalTransform::from_translation(at),
        ));
        if !active {
            entity.insert(SectionInactiveMarker);
        }
        entity.id()
    }

    /// A canister entity rendered at `at`, carrying exactly the fields
    /// `q_live_canisters` reads: `run_cargo_intakes` gates on
    /// `Health.current` itself, so a dead canister here is zeroed rather
    /// than marker-tagged.
    fn spawn_canister(app: &mut App, alive: bool, at: Vec3) -> Entity {
        let health = if alive {
            Health::new(20.0)
        } else {
            Health {
                current: 0.0,
                max: 20.0,
            }
        };
        app.world_mut()
            .spawn((
                Name::new("canister"),
                CargoCanister::new(ItemType::HullPlate, 1),
                health,
                Transform::from_translation(at),
                GlobalTransform::from_translation(at),
            ))
            .id()
    }

    fn pair(ship: Entity, intake: Entity, canister: Entity, ready: bool) -> CargoPickupPair {
        CargoPickupPair {
            ship,
            intake,
            canister,
            ready,
        }
    }

    fn parts(app: &mut App) -> Vec<(PickupSightPart, Transform)> {
        let world = app.world_mut();
        let mut query = world.query::<(&PickupSightPart, &Transform)>();
        query
            .iter(world)
            .map(|(part, transform)| (*part, *transform))
            .collect()
    }

    fn part(app: &mut App, wanted: PickupSightPart) -> Option<Transform> {
        parts(app)
            .into_iter()
            .find(|(part, _)| *part == wanted)
            .map(|(_, pose)| pose)
    }

    /// The direction a drawn segment runs in, and how long it is - the two
    /// things `segment_transform` encodes.
    fn segment(pose: Transform) -> (Vec3, f32) {
        (pose.rotation * Vec3::Y, pose.scale.y)
    }

    /// The line part's own material colour, read back out of the asset
    /// store rather than compared by handle: the claim under test is the
    /// exact hue.
    fn line_color(app: &mut App) -> Color {
        let world = app.world_mut();
        let mut query = world.query::<(&PickupSightPart, &MeshMaterial3d<StandardMaterial>)>();
        let handle = query
            .iter(world)
            .find(|(part, _)| matches!(part, PickupSightPart::Line))
            .map(|(_, material)| material.0.clone())
            .expect("the line is drawn");
        world
            .resource::<Assets<StandardMaterial>>()
            .get(&handle)
            .expect("the line material is allocated")
            .base_color
    }

    /// With no lock, the canister whose pair has the nearer intake face is
    /// drawn: the sight shows before the pilot locks anything.
    #[test]
    fn without_a_lock_the_nearest_canister_draws_the_sight() {
        let mut app = sight_app();
        let player = spawn_player(&mut app, Vec3::ZERO);
        let face = Vec3::new(0.0, 0.0, -1.0);
        let intake = spawn_intake(&mut app, player, true, face);
        let near_position = Vec3::new(0.0, 0.0, -5.0);
        let far = spawn_canister(&mut app, true, Vec3::new(0.0, 0.0, -15.0));
        let near = spawn_canister(&mut app, true, near_position);
        app.world_mut().resource_mut::<CargoPickupReadiness>().pairs = vec![
            pair(player, intake, far, false),
            pair(player, intake, near, false),
        ];
        app.update();

        let pose = part(&mut app, PickupSightPart::Line).expect("the line is drawn");
        assert!(
            (pose.translation - (face + near_position) * 0.5).length() < 1e-3,
            "the line reaches the nearer canister: {:?}",
            pose.translation
        );
    }

    /// A travel-locked canister in range is drawn over a nearer unlocked
    /// one: the lock names the canister the pilot wants. Once it renders
    /// past 200 m the lock gives way to the nearer canister still in range.
    #[test]
    fn a_locked_canister_in_range_outranks_a_nearer_one() {
        let mut app = sight_app();
        let player = spawn_player(&mut app, Vec3::ZERO);
        let face = Vec3::new(0.0, 0.0, -1.0);
        let intake = spawn_intake(&mut app, player, true, face);
        let near_position = Vec3::new(0.0, 0.0, -5.0);
        let locked_position = Vec3::new(0.0, 0.0, -15.0);
        let locked = spawn_canister(&mut app, true, locked_position);
        let near = spawn_canister(&mut app, true, near_position);
        app.world_mut().resource_mut::<CargoPickupReadiness>().pairs = vec![
            pair(player, intake, near, false),
            pair(player, intake, locked, false),
        ];
        app.world_mut()
            .entity_mut(player)
            .insert(TravelLock(Some(locked)));
        app.update();

        let pose = part(&mut app, PickupSightPart::Line).expect("the line is drawn");
        assert!(
            (pose.translation - (face + locked_position) * 0.5).length() < 1e-3,
            "the line reaches the locked canister: {:?}",
            pose.translation
        );

        let out_of_range = Transform::from_translation(Vec3::NEG_Z * Meters(201.0).to_engine());
        app.world_mut()
            .entity_mut(locked)
            .insert((out_of_range, GlobalTransform::from(out_of_range)));
        app.update();

        let pose = part(&mut app, PickupSightPart::Line).expect("the line is drawn");
        assert!(
            (pose.translation - (face + near_position) * 0.5).length() < 1e-3,
            "a locked canister past 200 m gives way to the nearer one: {:?}",
            pose.translation
        );
    }

    /// The sight reaches canisters rendered up to and including 200 m from
    /// the rendered ship itself, not from the intake face, and clears once
    /// the canister drifts past.
    #[test]
    fn only_a_canister_within_200_m_of_the_ship_draws_the_sight() {
        let mut app = sight_app();
        let ship_position = Vec3::new(3.0, 0.0, 0.0);
        let player = spawn_player(&mut app, ship_position);
        let intake = spawn_intake(&mut app, player, true, Vec3::new(3.0, 0.0, -1.0));
        let at = |meters: f32| ship_position + Vec3::NEG_Z * Meters(meters).to_engine();
        let canister = spawn_canister(&mut app, true, at(200.0));
        app.world_mut().resource_mut::<CargoPickupReadiness>().pairs =
            vec![pair(player, intake, canister, false)];
        app.update();
        assert!(
            part(&mut app, PickupSightPart::Line).is_some(),
            "a canister exactly 200 m from the ship draws the sight"
        );

        let past = Transform::from_translation(at(201.0));
        app.world_mut()
            .entity_mut(canister)
            .insert((past, GlobalTransform::from(past)));
        app.update();
        assert!(
            parts(&mut app).is_empty(),
            "a canister 201 m from the ship draws nothing"
        );
    }

    /// A lock that names a live canister with no published pair draws
    /// nothing: absence is not "wanting", it is not drawn - a canister no
    /// live intake has measured must never draw guidance.
    #[test]
    fn a_lock_without_a_published_pair_draws_nothing() {
        let mut app = sight_app();
        let player = spawn_player(&mut app, Vec3::ZERO);
        let canister = spawn_canister(&mut app, true, Vec3::new(0.0, 0.0, -3.0));
        app.world_mut()
            .entity_mut(player)
            .insert(TravelLock(Some(canister)));
        app.update();
        assert!(parts(&mut app).is_empty());
    }

    /// A lock on something that is not a canister at all - a beacon, a
    /// ship - never draws the sight to it, even through a pair that names
    /// it: the pick falls back to the nearest live canister.
    #[test]
    fn a_lock_on_a_non_canister_falls_back_to_the_nearest_canister() {
        let mut app = sight_app();
        let player = spawn_player(&mut app, Vec3::ZERO);
        let intake = spawn_intake(&mut app, player, true, Vec3::ZERO);
        let other = app
            .world_mut()
            .spawn((
                Name::new("not a canister"),
                Transform::from_xyz(0.0, 0.0, -3.0),
            ))
            .id();
        let canister_position = Vec3::new(0.0, 0.0, -9.0);
        let canister = spawn_canister(&mut app, true, canister_position);
        app.world_mut().resource_mut::<CargoPickupReadiness>().pairs = vec![
            pair(player, intake, other, true),
            pair(player, intake, canister, false),
        ];
        app.world_mut()
            .entity_mut(player)
            .insert(TravelLock(Some(other)));
        app.update();

        let pose = part(&mut app, PickupSightPart::Line).expect("the line is drawn");
        assert!(
            (pose.translation - canister_position * 0.5).length() < 1e-3,
            "the line reaches the live canister, not the locked non-canister: {:?}",
            pose.translation
        );
    }

    /// A pair published for a different ship's intake never lights up this
    /// ship's sight: readiness is per pair, not per canister.
    #[test]
    fn a_pair_published_for_another_ship_draws_nothing() {
        let mut app = sight_app();
        spawn_player(&mut app, Vec3::ZERO);
        let other_ship = app
            .world_mut()
            .spawn((Name::new("other ship"), Transform::default()))
            .id();
        let intake = spawn_intake(&mut app, other_ship, true, Vec3::ZERO);
        let canister = spawn_canister(&mut app, true, Vec3::new(0.0, 0.0, -3.0));
        app.world_mut().resource_mut::<CargoPickupReadiness>().pairs =
            vec![pair(other_ship, intake, canister, true)];
        app.update();
        assert!(parts(&mut app).is_empty());
    }

    /// A pair naming a canister that has since taken lethal damage (and
    /// would already be despawned by the intake's own observer) is never
    /// drawn, even locked and nearer, while `CargoPickupReadiness` has not
    /// yet refreshed past it: the pick falls back to a live canister.
    #[test]
    fn a_stale_pair_naming_a_dead_canister_falls_back_to_a_live_one() {
        let mut app = sight_app();
        let player = spawn_player(&mut app, Vec3::ZERO);
        let intake = spawn_intake(&mut app, player, true, Vec3::ZERO);
        let dead = spawn_canister(&mut app, false, Vec3::new(0.0, 0.0, -3.0));
        let live_position = Vec3::new(0.0, 0.0, -9.0);
        let live = spawn_canister(&mut app, true, live_position);
        app.world_mut().resource_mut::<CargoPickupReadiness>().pairs = vec![
            pair(player, intake, dead, true),
            pair(player, intake, live, false),
        ];
        app.world_mut()
            .entity_mut(player)
            .insert(TravelLock(Some(dead)));
        app.update();

        let pose = part(&mut app, PickupSightPart::Line).expect("the line is drawn");
        assert!(
            (pose.translation - live_position * 0.5).length() < 1e-3,
            "the line reaches the live canister, not the dead one: {:?}",
            pose.translation
        );
    }

    /// A pair naming an intake that has since gone inactive draws nothing,
    /// for the same reason: the snapshot can be a fixed tick behind the
    /// intake's own live state.
    #[test]
    fn a_stale_pair_naming_an_inactive_intake_draws_nothing() {
        let mut app = sight_app();
        let player = spawn_player(&mut app, Vec3::ZERO);
        let intake = spawn_intake(&mut app, player, false, Vec3::ZERO);
        let canister = spawn_canister(&mut app, true, Vec3::new(0.0, 0.0, -3.0));
        app.world_mut().resource_mut::<CargoPickupReadiness>().pairs =
            vec![pair(player, intake, canister, true)];
        app.update();
        assert!(parts(&mut app).is_empty());
    }

    /// A pair naming an intake that has since been reparented off the ship
    /// it was published for draws nothing: the pair's own `ship` field is a
    /// snapshot, and the intake's live `ChildOf` is the ground truth of
    /// which ship it still belongs to.
    #[test]
    fn a_stale_pair_naming_a_reparented_intake_draws_nothing() {
        let mut app = sight_app();
        let player = spawn_player(&mut app, Vec3::ZERO);
        let other_ship = app
            .world_mut()
            .spawn((Name::new("other ship"), Transform::default()))
            .id();
        let intake = spawn_intake(&mut app, other_ship, true, Vec3::ZERO);
        let canister = spawn_canister(&mut app, true, Vec3::new(0.0, 0.0, -3.0));
        app.world_mut().resource_mut::<CargoPickupReadiness>().pairs =
            vec![pair(player, intake, canister, true)];
        app.update();
        assert!(parts(&mut app).is_empty());
    }

    /// The whole point of the instrument: the plate stands on the rendered
    /// intake face and the line reaches the rendered canister, and both
    /// follow those rendered poses from frame to frame while the fixed
    /// pass's published pair does not change.
    #[test]
    fn the_plate_and_line_follow_the_rendered_intake_face_and_canister() {
        let mut app = sight_app();
        let player = spawn_player(&mut app, Vec3::ZERO);
        let face = Vec3::new(0.0, 0.0, -1.0);
        let intake = spawn_intake(&mut app, player, true, face);
        let canister = spawn_canister(&mut app, true, Vec3::new(0.0, 0.0, -4.0));
        app.world_mut().resource_mut::<CargoPickupReadiness>().pairs =
            vec![pair(player, intake, canister, false)];
        app.world_mut()
            .entity_mut(player)
            .insert(TravelLock(Some(canister)));
        app.update();

        for arm in 0..2 {
            let pose = part(&mut app, PickupSightPart::Plate { arm }).expect("both arms drawn");
            assert!(
                (pose.translation - (face + Vec3::NEG_Z * PLATE_LIFT)).length() < 1e-3,
                "arm {arm} floats off the intake face along its normal: {:?}",
                pose.translation
            );
        }
        let pose = part(&mut app, PickupSightPart::Line).expect("the line is drawn");
        let (direction, length) = segment(pose);
        assert!(
            (length - 3.0).abs() < 1e-3,
            "line spans face to canister: {length}"
        );
        assert!(
            direction.dot(Vec3::NEG_Z) > 0.99,
            "and runs from the face toward the canister: {direction:?}"
        );

        // The next frame renders the intake turned a quarter turn about +Y,
        // so its door faces -X, and the canister beside it. The player sits
        // at the origin, so the intake's local pose is its rendered pose.
        let intake_pose = Transform::from_xyz(2.0, 0.0, -3.0)
            .with_rotation(Quat::from_rotation_y(std::f32::consts::FRAC_PI_2));
        let canister_pose = Transform::from_xyz(-2.5, 0.0, -3.0);
        app.world_mut()
            .entity_mut(intake)
            .insert((intake_pose, GlobalTransform::from(intake_pose)));
        app.world_mut()
            .entity_mut(canister)
            .insert((canister_pose, GlobalTransform::from(canister_pose)));
        app.update();

        let face = Vec3::new(1.5, 0.0, -3.0);
        for arm in 0..2 {
            let pose = part(&mut app, PickupSightPart::Plate { arm }).expect("both arms drawn");
            assert!(
                (pose.translation - (face + Vec3::NEG_X * PLATE_LIFT)).length() < 1e-3,
                "arm {arm} follows the turned face: {:?}",
                pose.translation
            );
        }
        let pose = part(&mut app, PickupSightPart::Line).expect("the line is drawn");
        let (direction, length) = segment(pose);
        assert!(
            (pose.translation - (face + canister_pose.translation) * 0.5).length() < 1e-3,
            "the line follows the rendered face and canister: {:?}",
            pose.translation
        );
        assert!(
            (length - 4.0).abs() < 1e-3,
            "line spans the turned face to the moved canister: {length}"
        );
        assert!(
            direction.dot(Vec3::NEG_X) > 0.99,
            "and runs from the turned face toward the canister: {direction:?}"
        );
    }

    /// When the same canister sits in two of this ship's intakes at once,
    /// the sight follows the pair with the nearer face, even when the
    /// farther intake is the one that is actually ready: the pick is
    /// geometry, not readiness.
    #[test]
    fn two_pairs_for_the_same_canister_draw_the_nearer_face() {
        let mut app = sight_app();
        let player = spawn_player(&mut app, Vec3::ZERO);
        let near_face = Vec3::new(0.0, 0.0, -1.0);
        let near_intake = spawn_intake(&mut app, player, true, near_face);
        let far_intake = spawn_intake(&mut app, player, true, Vec3::new(10.0, 0.0, -10.0));
        let canister_position = Vec3::new(0.0, 0.0, -4.0);
        let canister = spawn_canister(&mut app, true, canister_position);
        app.world_mut().resource_mut::<CargoPickupReadiness>().pairs = vec![
            pair(player, near_intake, canister, false),
            pair(player, far_intake, canister, true),
        ];
        app.world_mut()
            .entity_mut(player)
            .insert(TravelLock(Some(canister)));
        app.update();

        let pose = part(&mut app, PickupSightPart::Line).expect("the nearer pair is drawn");
        let midpoint = (near_face + canister_position) * 0.5;
        assert!(
            (pose.translation - midpoint).length() < 1e-3,
            "the line follows the nearer face, not the farther one: {:?}",
            pose.translation
        );
    }

    /// A real take moves the whole canister into the hold, despawns it and
    /// voices the take, and the sight - drawn with no lock - clears with the
    /// canister: nothing confirms the take on screen after it is gone.
    #[test]
    fn a_real_take_fills_the_hold_and_clears_the_sight() {
        let mut app = unfinished_integrity_physics_app();
        app.init_asset::<StandardMaterial>();
        app.init_resource::<CargoCanisterIdAllocator>();
        app.add_plugins((
            SectionAnimationPlugin,
            CargoIntakeSectionPlugin { render: false },
            PickupSightPlugin,
        ));
        app.finish();
        let player = app
            .world_mut()
            .spawn((
                SpaceshipRootMarker,
                PlayerSpaceshipMarker,
                RigidBody::Dynamic,
                Transform::default(),
                TravelLock(None),
                ShipInventory::new(400_000, []),
            ))
            .id();
        app.world_mut().spawn((
            ChildOf(player),
            Transform::default(),
            Collider::cuboid(1.0, 1.0, 1.0),
            ColliderDensity(1.0),
        ));
        let collider = SectionCollider::Cuboid {
            size: Vec3::new(3.0, 2.0, 1.0),
        };
        let intake = app
            .world_mut()
            .spawn((
                ChildOf(player),
                Transform::from_xyz(0.0, 0.0, -2.0),
                collider,
                collider.to_collider(),
                ColliderDensity(1.0),
                SectionAnimations::new(vec![SectionAnimation {
                    cue: SectionAnimationCue::IntakeDoor,
                    node_prefix: "intake_slat_".to_string(),
                    motion: SectionAnimationMotion::Fold {
                        degrees: 80.0,
                        slat_width: 0.2292,
                        slat_thickness: 0.02,
                    },
                    open_seconds: 1.2,
                    close_seconds: 1.2,
                }]),
                cargo_intake_section(CargoIntakeSectionConfig {
                    render_mesh: AssetRef::from("intake.glb#Scene0"),
                    render_mesh_transform: None,
                    canister_mesh: AssetRef::from("canister.glb#Scene0"),
                    door_sound: AssetRef::from("door.wav"),
                    eject_sound: AssetRef::from("eject.wav"),
                    take_sound: AssetRef::from("take.wav"),
                    detection_range: Meters(40.0),
                    capture_gap: Meters(1.0),
                    aperture_width: Meters(22.2),
                    aperture_height: Meters(15.3),
                    eject_speed: MetersPerSecond(3.0),
                }),
            ))
            .id();
        nova_gameplay::test_support::settle(&mut app);
        let canister = app
            .world_mut()
            .spawn(cargo_canister(
                CargoCanister::new(ItemType::HullPlate, 4),
                Transform::from_xyz(0.0, 0.0, -2.85),
                Vec3::ZERO,
                AssetRef::from("canister.glb#Scene0"),
            ))
            .id();
        let takes = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let observed = takes.clone();
        app.add_observer(move |taken: On<CargoCanisterTaken>| {
            assert_eq!(taken.entity, intake);
            observed.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        });
        app.update();
        assert_eq!(
            line_color(&mut app),
            crate::NAV_CYAN.with_alpha(SIGHT_ALPHA),
            "the sight draws nav cyan before the take"
        );

        app.world_mut()
            .get_mut::<SectionAnimations>(intake)
            .unwrap()
            .snap_cue(SectionAnimationCue::IntakeDoor, 1.0);
        let mut frames = 0;
        while app.world().get_entity(canister).is_ok() {
            app.update();
            frames += 1;
            assert!(
                frames < 30,
                "the open door did not take the canister: {:?}",
                app.world().resource::<CargoPickupReadiness>().pairs
            );
        }
        assert_eq!(
            app.world()
                .get::<ShipInventory>(player)
                .unwrap()
                .count(ItemType::HullPlate),
            4
        );
        assert_eq!(takes.load(std::sync::atomic::Ordering::Relaxed), 1);
        assert!(
            parts(&mut app).is_empty(),
            "the sight clears in the frame the canister is taken"
        );
    }
}
