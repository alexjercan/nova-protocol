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
//! sight reads that pass's decision instead of computing a separate gate.
//! A pair is trusted only while both ends still validate live (the canister
//! with positive [`Health::current`], the intake with
//! [`CargoIntakeSectionMarker`], without [`SectionInactiveMarker`], and still
//! parented to the same ship the pair names): `CargoPickupReadiness` is a
//! snapshot from the last fixed pass and `Update` runs between fixed ticks. A
//! take despawns the canister in the same fixed pass that marks it ready, so
//! the sight clears with it; the take sound, not the sight, confirms the
//! take.
//!
//! # Which pair is drawn
//!
//! Only pairs whose canister lies within [`CARGO_PICKUP_SIGHT_RANGE`] of the
//! player's ship, so the line shows before a lock and before the intake's
//! own detection range. A travel-locked canister in range is drawn first; a
//! lock on anything else - a beacon, a ship, a dead canister, a canister out
//! of range - leaves the pick to distance. The pair with the nearer intake
//! face to its canister wins, then the lower entity IDs. That pick is a
//! display convenience only: the mechanic's own zone and speed gates, not
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

/// The published pair to draw for `ship` at `ship_position`: among pairs
/// whose canister lies within [`CARGO_PICKUP_SIGHT_RANGE`] of the ship and
/// whose ends still validate live, a pair naming `locked` first, else any
/// pair. Among the chosen set, the nearest intake face to its canister wins,
/// then the lower canister and intake entity IDs. `readiness` is a snapshot
/// from the last fixed pass and can still name a canister that has died or
/// an intake that has gone inactive, been despawned, or been reparented off
/// `ship`.
fn select_pair<'a>(
    pairs: &'a [CargoPickupPair],
    ship: Entity,
    ship_position: Vec3,
    locked: Option<Entity>,
    q_live_canisters: &Query<&Health, (With<CargoCanister>, Without<HealthZeroMarker>)>,
    q_live_intakes: &Query<
        &ChildOf,
        (
            With<CargoIntakeSectionMarker>,
            Without<SectionInactiveMarker>,
        ),
    >,
) -> Option<&'a CargoPickupPair> {
    let range = CARGO_PICKUP_SIGHT_RANGE.to_engine();
    let candidates = pairs.iter().filter(move |pair| {
        pair.ship == ship
            && pair.canister_position.distance_squared(ship_position) <= range * range
            && q_live_canisters
                .get(pair.canister)
                .is_ok_and(|health| health.current > 0.0)
            && q_live_intakes
                .get(pair.intake)
                .is_ok_and(|&ChildOf(parent)| parent == ship)
    });
    let nearest = |pairs: &mut dyn Iterator<Item = &'a CargoPickupPair>| {
        pairs.min_by(|a, b| {
            let da = a.face.distance_squared(a.canister_position);
            let db = b.face.distance_squared(b.canister_position);
            da.total_cmp(&db)
                .then(a.canister.cmp(&b.canister))
                .then(a.intake.cmp(&b.intake))
        })
    };
    locked
        .and_then(|locked| nearest(&mut candidates.clone().filter(|pair| pair.canister == locked)))
        .or_else(|| nearest(&mut candidates.clone()))
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
    q_live_canisters: Query<&Health, (With<CargoCanister>, Without<HealthZeroMarker>)>,
    q_live_intakes: Query<
        &ChildOf,
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
            let pair = select_pair(
                &readiness.pairs,
                ship,
                transform.translation(),
                travel.and_then(|travel| travel.0),
                &q_live_canisters,
                &q_live_intakes,
            )?;
            Some(draw_sight(pair.face, pair.normal, pair.canister_position))
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

    /// A live intake entity, parented to `ship` and carrying exactly the
    /// components `select_pair` checks for liveness.
    fn spawn_intake(app: &mut App, ship: Entity, active: bool) -> Entity {
        let mut entity =
            app.world_mut()
                .spawn((Name::new("intake"), ChildOf(ship), CargoIntakeSectionMarker));
        if !active {
            entity.insert(SectionInactiveMarker);
        }
        entity.id()
    }

    /// A canister entity carrying exactly the field `q_live_canisters`
    /// checks for liveness: `run_cargo_intakes` gates on `Health.current`
    /// itself, so a dead canister here is zeroed rather than marker-tagged.
    fn spawn_canister(app: &mut App, alive: bool) -> Entity {
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
            ))
            .id()
    }

    fn pair(
        ship: Entity,
        intake: Entity,
        canister: Entity,
        face: Vec3,
        canister_position: Vec3,
        ready: bool,
    ) -> CargoPickupPair {
        CargoPickupPair {
            ship,
            intake,
            canister,
            face,
            normal: Vec3::NEG_Z,
            canister_position,
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
        let intake = spawn_intake(&mut app, player, true);
        let far = spawn_canister(&mut app, true);
        let near = spawn_canister(&mut app, true);
        let face = Vec3::new(0.0, 0.0, -1.0);
        let near_position = Vec3::new(0.0, 0.0, -5.0);
        app.world_mut().resource_mut::<CargoPickupReadiness>().pairs = vec![
            pair(player, intake, far, face, Vec3::new(0.0, 0.0, -15.0), false),
            pair(player, intake, near, face, near_position, false),
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
    /// one: the lock names the canister the pilot wants. Past 200 m the
    /// lock gives way to the nearer canister still in range.
    #[test]
    fn a_locked_canister_in_range_outranks_a_nearer_one() {
        let mut app = sight_app();
        let player = spawn_player(&mut app, Vec3::ZERO);
        let intake = spawn_intake(&mut app, player, true);
        let locked = spawn_canister(&mut app, true);
        let near = spawn_canister(&mut app, true);
        let face = Vec3::new(0.0, 0.0, -1.0);
        let near_position = Vec3::new(0.0, 0.0, -5.0);
        let locked_position = Vec3::new(0.0, 0.0, -15.0);
        app.world_mut().resource_mut::<CargoPickupReadiness>().pairs = vec![
            pair(player, intake, near, face, near_position, false),
            pair(player, intake, locked, face, locked_position, false),
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

        let out_of_range = Vec3::NEG_Z * Meters(201.0).to_engine();
        app.world_mut().resource_mut::<CargoPickupReadiness>().pairs = vec![
            pair(player, intake, near, face, near_position, false),
            pair(player, intake, locked, face, out_of_range, false),
        ];
        app.update();

        let pose = part(&mut app, PickupSightPart::Line).expect("the line is drawn");
        assert!(
            (pose.translation - (face + near_position) * 0.5).length() < 1e-3,
            "a locked canister past 200 m gives way to the nearer one: {:?}",
            pose.translation
        );
    }

    /// The sight reaches canisters up to and including 200 m from the ship
    /// itself, not from the intake face, and clears once the canister
    /// drifts past.
    #[test]
    fn only_a_canister_within_200_m_of_the_ship_draws_the_sight() {
        let mut app = sight_app();
        let ship_position = Vec3::new(3.0, 0.0, 0.0);
        let player = spawn_player(&mut app, ship_position);
        let intake = spawn_intake(&mut app, player, true);
        let canister = spawn_canister(&mut app, true);
        let face = Vec3::new(3.0, 0.0, -1.0);
        let at = |meters: f32| ship_position + Vec3::NEG_Z * Meters(meters).to_engine();
        app.world_mut().resource_mut::<CargoPickupReadiness>().pairs =
            vec![pair(player, intake, canister, face, at(200.0), false)];
        app.update();
        assert!(
            part(&mut app, PickupSightPart::Line).is_some(),
            "a canister exactly 200 m from the ship draws the sight"
        );

        app.world_mut().resource_mut::<CargoPickupReadiness>().pairs =
            vec![pair(player, intake, canister, face, at(201.0), false)];
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
        let canister = spawn_canister(&mut app, true);
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
        let intake = spawn_intake(&mut app, player, true);
        let other = app.world_mut().spawn(Name::new("not a canister")).id();
        let canister = spawn_canister(&mut app, true);
        let canister_position = Vec3::new(0.0, 0.0, -9.0);
        app.world_mut().resource_mut::<CargoPickupReadiness>().pairs = vec![
            pair(
                player,
                intake,
                other,
                Vec3::ZERO,
                Vec3::new(0.0, 0.0, -3.0),
                true,
            ),
            pair(
                player,
                intake,
                canister,
                Vec3::ZERO,
                canister_position,
                false,
            ),
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
        let other_ship = app.world_mut().spawn(Name::new("other ship")).id();
        let intake = spawn_intake(&mut app, other_ship, true);
        let canister = spawn_canister(&mut app, true);
        app.world_mut().resource_mut::<CargoPickupReadiness>().pairs = vec![pair(
            other_ship,
            intake,
            canister,
            Vec3::ZERO,
            Vec3::new(0.0, 0.0, -3.0),
            true,
        )];
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
        let intake = spawn_intake(&mut app, player, true);
        let dead = spawn_canister(&mut app, false);
        let live = spawn_canister(&mut app, true);
        let live_position = Vec3::new(0.0, 0.0, -9.0);
        app.world_mut().resource_mut::<CargoPickupReadiness>().pairs = vec![
            pair(
                player,
                intake,
                dead,
                Vec3::ZERO,
                Vec3::new(0.0, 0.0, -3.0),
                true,
            ),
            pair(player, intake, live, Vec3::ZERO, live_position, false),
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
        let intake = spawn_intake(&mut app, player, false);
        let canister = spawn_canister(&mut app, true);
        app.world_mut().resource_mut::<CargoPickupReadiness>().pairs = vec![pair(
            player,
            intake,
            canister,
            Vec3::ZERO,
            Vec3::new(0.0, 0.0, -3.0),
            true,
        )];
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
        let other_ship = app.world_mut().spawn(Name::new("other ship")).id();
        let intake = spawn_intake(&mut app, other_ship, true);
        let canister = spawn_canister(&mut app, true);
        app.world_mut().resource_mut::<CargoPickupReadiness>().pairs = vec![pair(
            player,
            intake,
            canister,
            Vec3::ZERO,
            Vec3::new(0.0, 0.0, -3.0),
            true,
        )];
        app.update();
        assert!(parts(&mut app).is_empty());
    }

    /// The whole point of the instrument: the plate stands on the intake
    /// face and the line reaches exactly the published canister position.
    #[test]
    fn the_plate_sits_on_the_face_and_the_line_reaches_the_canister() {
        let mut app = sight_app();
        let player = spawn_player(&mut app, Vec3::ZERO);
        let intake = spawn_intake(&mut app, player, true);
        let canister = spawn_canister(&mut app, true);
        let face = Vec3::new(0.0, 0.0, -1.0);
        let canister_position = Vec3::new(0.0, 0.0, -4.0);
        app.world_mut().resource_mut::<CargoPickupReadiness>().pairs = vec![pair(
            player,
            intake,
            canister,
            face,
            canister_position,
            false,
        )];
        app.world_mut()
            .entity_mut(player)
            .insert(TravelLock(Some(canister)));
        app.update();

        for arm in 0..2 {
            let pose = part(&mut app, PickupSightPart::Plate { arm }).expect("both arms drawn");
            assert!(
                (pose.translation - face).length() < PLATE_LIFT + 1e-3,
                "arm {arm} stands on the intake face: {:?}",
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
    }

    /// When the same canister sits in two of this ship's intakes at once,
    /// the sight follows the pair with the nearer face, even when the
    /// farther intake is the one that is actually ready: the pick is
    /// geometry, not readiness.
    #[test]
    fn two_pairs_for_the_same_canister_draw_the_nearer_face() {
        let mut app = sight_app();
        let player = spawn_player(&mut app, Vec3::ZERO);
        let near_intake = spawn_intake(&mut app, player, true);
        let far_intake = spawn_intake(&mut app, player, true);
        let canister = spawn_canister(&mut app, true);
        let near_face = Vec3::new(0.0, 0.0, -1.0);
        let far_face = Vec3::new(10.0, 0.0, -10.0);
        let canister_position = Vec3::new(0.0, 0.0, -4.0);
        app.world_mut().resource_mut::<CargoPickupReadiness>().pairs = vec![
            pair(
                player,
                near_intake,
                canister,
                near_face,
                canister_position,
                false,
            ),
            pair(
                player,
                far_intake,
                canister,
                far_face,
                canister_position,
                true,
            ),
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
                ShipInventory::new(400, []),
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
                    maximum_capture_speed: MetersPerSecond(5.0),
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
