//! The pickup sight: a face plate on each of the player's cargo intakes with
//! a canister in its reach, and a line from that face to every such
//! canister, drawn in world space the way
//! [`docking_sight`](super::docking_sight) draws its own pair - alignment you
//! fly INTO rather than a number you read.
//!
//! # Where the numbers come from
//!
//! [`CargoPickupReadiness`], the same fixed pass that decides a take and the
//! pull (`run_cargo_intakes` in `nova_ship::sections::cargo_intake_section`).
//! It publishes every live intake/canister pair with its `in_reach` flag:
//! the canister is in front of the intake's face and within its
//! `pull_range`. The sight draws exactly the pairs in reach instead of
//! computing a separate gate, so a line shows for every canister the intake
//! reaches. A pair is trusted only while both ends still validate live (the
//! canister with positive [`Health::current`], the intake with
//! [`CargoIntakeSectionMarker`], without [`SectionInactiveMarker`], and still
//! parented to the same ship the pair names): `CargoPickupReadiness` is a
//! snapshot from the last fixed pass and `Update` runs between fixed ticks. A
//! take that empties the canister despawns it in the same fixed pass that
//! marks it ready, so its line clears with it. A take the hold's room cuts
//! short leaves the rest in the same canister, which keeps its line while it
//! is in reach. The take sound, not the sight, confirms the take.
//!
//! Every pose the sight draws or measures is the interpolated
//! `GlobalTransform` of the ship, the intake and the canister, never the raw
//! fixed-tick pose: the sight is drawn over the rendered models, so the face
//! comes from [`cargo_intake_face`] on the intake's rendered pose.
//!
//! # Which intake a line runs to
//!
//! A canister in the reach of two of the player's intakes draws one line, to
//! the nearer rendered face, then the lower intake entity: the pull's own
//! choice. A plate marks each intake with at least one line.

use bevy::{light::NotShadowCaster, prelude::*};
use nova_gameplay::prelude::*;
use nova_ship::prelude::*;

use super::holo_instruments::segment_transform;

/// `PickupSightPlugin` and the component its parts carry.
pub mod prelude {
    pub use super::{PickupSightPart, PickupSightPlugin};
}

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
    /// One arm of the cross drawn across an intake's face.
    Plate {
        /// The intake whose face it marks.
        intake: Entity,
        /// Which arm of the cross.
        arm: u8,
    },
    /// The line from an intake face to one canister in its reach.
    Line {
        /// The canister it reaches.
        canister: Entity,
    },
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

    /// Nav cyan throughout: the sight never reads a pair's `ready` flag, so
    /// it does not mark the pass that takes from a canister.
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

/// One canister's line this frame, from rendered poses.
struct SightLine {
    intake: Entity,
    canister: Entity,
    face: Vec3,
    normal: Vec3,
    canister_position: Vec3,
}

/// Lay out the whole sight: a plate on each intake with a line, and every
/// line. World up (falling back to world forward) is a plate's reference,
/// not the player hull's own: unlike the docking sight a plate has only one
/// end, so there is no shared clocking for it to agree with.
fn draw_sight(lines: &[SightLine]) -> Vec<Drawn> {
    let mut drawn = Vec::new();
    let mut plated: Vec<Entity> = Vec::new();
    for line in lines {
        drawn.push(Drawn {
            part: PickupSightPart::Line {
                canister: line.canister,
            },
            pose: segment_transform(line.face, line.canister_position),
        });
        if plated.contains(&line.intake) {
            continue;
        }
        plated.push(line.intake);
        let Some((first_arm, second_arm)) =
            plate_arms(line.normal, Vec3::Y).or_else(|| plate_arms(line.normal, Vec3::Z))
        else {
            continue;
        };
        let centre = line.face + line.normal * PLATE_LIFT;
        for (arm, direction) in [(0_u8, first_arm), (1, second_arm)] {
            let reach = direction * PLATE_SPAN * 0.5;
            drawn.push(Drawn {
                part: PickupSightPart::Plate {
                    intake: line.intake,
                    arm,
                },
                pose: segment_transform(centre - reach, centre + reach),
            });
        }
    }
    drawn
}

/// One line for every canister in the reach of an intake on `ship`, from
/// rendered poses: among published pairs in reach whose ends still validate
/// live, each canister's nearest rendered intake face, then the lower intake
/// entity. `pairs` is a snapshot from the last fixed pass and can still name
/// a canister that has died or an intake that has gone inactive, been
/// despawned, or been reparented off `ship`.
fn select_lines(
    pairs: &[CargoPickupPair],
    ship: Entity,
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
) -> Vec<SightLine> {
    let mut lines: Vec<SightLine> = Vec::new();
    for pair in pairs {
        if pair.ship != ship || !pair.in_reach {
            continue;
        }
        let Ok((health, canister_pose)) = q_live_canisters.get(pair.canister) else {
            continue;
        };
        let Ok((&ChildOf(parent), intake_pose, &collider)) = q_live_intakes.get(pair.intake) else {
            continue;
        };
        if health.current <= 0.0 || parent != ship {
            continue;
        }
        let (face, normal) =
            cargo_intake_face(intake_pose.translation(), intake_pose.rotation(), collider);
        let line = SightLine {
            intake: pair.intake,
            canister: pair.canister,
            face,
            normal,
            canister_position: canister_pose.translation(),
        };
        let distance = face.distance_squared(line.canister_position);
        match lines.iter_mut().find(|kept| kept.canister == pair.canister) {
            Some(kept) => {
                let kept_distance = kept.face.distance_squared(kept.canister_position);
                if distance
                    .total_cmp(&kept_distance)
                    .then(pair.intake.cmp(&kept.intake))
                    .is_lt()
                {
                    *kept = line;
                }
            }
            None => lines.push(line),
        }
    }
    lines
}

/// Reconcile live readiness against the player's ship.
fn sync_pickup_sight(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut assets: ResMut<PickupSightAssets>,
    readiness: Res<CargoPickupReadiness>,
    q_player: Query<Entity, (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>)>,
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
        .map(|ship| {
            draw_sight(&select_lines(
                &readiness.pairs,
                ship,
                &q_live_canisters,
                &q_live_intakes,
            ))
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

/// Draws the world-space pickup sight for every canister in the reach of the
/// player's intakes from [`CargoPickupReadiness`]. Inits
/// `PickupSightAssets`, registers [`PickupSightPart`], and runs
/// `sync_pickup_sight` in `Update` within [`super::NovaHudSystems`].
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
    use nova_events::units::prelude::*;
    use nova_gameplay::{
        asset_ref::AssetRef,
        test_support::{test_items, unfinished_integrity_physics_app},
    };

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
    /// components `select_lines` reads, and rendered facing -Z with its door
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
        let id = app
            .world_mut()
            .get_resource_or_init::<CargoCanisterIdAllocator>()
            .mint();
        app.world_mut()
            .spawn((
                Name::new("canister"),
                CargoCanister::new(&test_items(), &ItemRoleType::HullPlate.into(), 1),
                id,
                health,
                Transform::from_translation(at),
                GlobalTransform::from_translation(at),
            ))
            .id()
    }

    /// A published pair in the intake's reach, not ready.
    fn in_reach(ship: Entity, intake: Entity, canister: Entity) -> CargoPickupPair {
        CargoPickupPair {
            ship,
            intake,
            canister,
            in_reach: true,
            ready: false,
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

    fn line(app: &mut App, canister: Entity) -> Option<Transform> {
        part(app, PickupSightPart::Line { canister })
    }

    /// The direction a drawn segment runs in, and how long it is - the two
    /// things `segment_transform` encodes.
    fn segment(pose: Transform) -> (Vec3, f32) {
        (pose.rotation * Vec3::Y, pose.scale.y)
    }

    /// The colour of the first line part's own material, read back out of
    /// the asset store rather than compared by handle: the claim under test
    /// is the exact hue.
    fn line_color(app: &mut App) -> Color {
        let world = app.world_mut();
        let mut query = world.query::<(&PickupSightPart, &MeshMaterial3d<StandardMaterial>)>();
        let handle = query
            .iter(world)
            .find(|(part, _)| matches!(part, PickupSightPart::Line { .. }))
            .map(|(_, material)| material.0.clone())
            .expect("a line is drawn");
        world
            .resource::<Assets<StandardMaterial>>()
            .get(&handle)
            .expect("the line material is allocated")
            .base_color
    }

    /// Every canister the fixed pass publishes in reach draws its own line
    /// from the intake face, and one plate marks the face; a canister out of
    /// reach draws none.
    #[test]
    fn every_canister_in_reach_draws_a_line_and_none_out_of_reach() {
        let mut app = sight_app();
        let player = spawn_player(&mut app, Vec3::ZERO);
        let face = Vec3::new(0.0, 0.0, -1.0);
        let intake = spawn_intake(&mut app, player, true, face);
        let reached: Vec<(Entity, Vec3)> = [
            Vec3::new(0.0, 0.0, -5.0),
            Vec3::new(4.0, 1.0, -9.0),
            Vec3::new(-6.0, 0.0, -11.0),
        ]
        .into_iter()
        .map(|at| (spawn_canister(&mut app, true, at), at))
        .collect();
        let out = spawn_canister(&mut app, true, Vec3::new(0.0, 0.0, -14.0));
        let mut pairs: Vec<CargoPickupPair> = reached
            .iter()
            .map(|&(canister, _)| in_reach(player, intake, canister))
            .collect();
        pairs.push(CargoPickupPair {
            in_reach: false,
            ..in_reach(player, intake, out)
        });
        app.world_mut().resource_mut::<CargoPickupReadiness>().pairs = pairs;
        app.update();

        for &(canister, at) in &reached {
            let pose = line(&mut app, canister).expect("a canister in reach draws a line");
            assert!(
                (pose.translation - (face + at) * 0.5).length() < 1e-3,
                "the line runs from the face to {canister:?}: {:?}",
                pose.translation
            );
        }
        assert!(
            line(&mut app, out).is_none(),
            "a canister out of reach draws no line"
        );
        let plates = parts(&mut app)
            .iter()
            .filter(|(part, _)| matches!(part, PickupSightPart::Plate { .. }))
            .count();
        assert_eq!(plates, 2, "one cross on the one intake face");
    }

    /// A canister without a published pair draws nothing: a canister no
    /// live intake has measured must never draw guidance.
    #[test]
    fn a_canister_without_a_published_pair_draws_nothing() {
        let mut app = sight_app();
        let player = spawn_player(&mut app, Vec3::ZERO);
        spawn_intake(&mut app, player, true, Vec3::ZERO);
        spawn_canister(&mut app, true, Vec3::new(0.0, 0.0, -3.0));
        app.update();
        assert!(parts(&mut app).is_empty());
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
            vec![in_reach(other_ship, intake, canister)];
        app.update();
        assert!(parts(&mut app).is_empty());
    }

    /// A pair naming a canister that has since taken lethal damage (and
    /// would already be despawned by the intake's own observer) draws no
    /// line while `CargoPickupReadiness` has not yet refreshed past it; a
    /// live canister beside it keeps its line.
    #[test]
    fn a_stale_pair_naming_a_dead_canister_draws_no_line() {
        let mut app = sight_app();
        let player = spawn_player(&mut app, Vec3::ZERO);
        let intake = spawn_intake(&mut app, player, true, Vec3::ZERO);
        let dead = spawn_canister(&mut app, false, Vec3::new(0.0, 0.0, -3.0));
        let live_position = Vec3::new(0.0, 0.0, -9.0);
        let live = spawn_canister(&mut app, true, live_position);
        app.world_mut().resource_mut::<CargoPickupReadiness>().pairs = vec![
            in_reach(player, intake, dead),
            in_reach(player, intake, live),
        ];
        app.update();

        assert!(
            line(&mut app, dead).is_none(),
            "the dead canister has no line"
        );
        let pose = line(&mut app, live).expect("the live canister keeps its line");
        assert!(
            (pose.translation - live_position * 0.5).length() < 1e-3,
            "{:?}",
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
            vec![in_reach(player, intake, canister)];
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
            vec![in_reach(player, intake, canister)];
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
            vec![in_reach(player, intake, canister)];
        app.update();

        for arm in 0..2 {
            let pose =
                part(&mut app, PickupSightPart::Plate { intake, arm }).expect("both arms drawn");
            assert!(
                (pose.translation - (face + Vec3::NEG_Z * PLATE_LIFT)).length() < 1e-3,
                "arm {arm} floats off the intake face along its normal: {:?}",
                pose.translation
            );
        }
        let pose = line(&mut app, canister).expect("the line is drawn");
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
            let pose =
                part(&mut app, PickupSightPart::Plate { intake, arm }).expect("both arms drawn");
            assert!(
                (pose.translation - (face + Vec3::NEG_X * PLATE_LIFT)).length() < 1e-3,
                "arm {arm} follows the turned face: {:?}",
                pose.translation
            );
        }
        let pose = line(&mut app, canister).expect("the line is drawn");
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

    /// When the same canister is in the reach of two of this ship's intakes
    /// at once, it draws one line, to the nearer face, even when the farther
    /// intake is the one that is actually ready: the pick is the pull's
    /// geometry, not readiness. Only the nearer intake is plated.
    #[test]
    fn a_canister_in_reach_of_two_intakes_draws_one_line_to_the_nearer_face() {
        let mut app = sight_app();
        let player = spawn_player(&mut app, Vec3::ZERO);
        let near_face = Vec3::new(0.0, 0.0, -1.0);
        let near_intake = spawn_intake(&mut app, player, true, near_face);
        let far_intake = spawn_intake(&mut app, player, true, Vec3::new(10.0, 0.0, -10.0));
        let canister_position = Vec3::new(0.0, 0.0, -4.0);
        let canister = spawn_canister(&mut app, true, canister_position);
        app.world_mut().resource_mut::<CargoPickupReadiness>().pairs = vec![
            in_reach(player, near_intake, canister),
            CargoPickupPair {
                ready: true,
                ..in_reach(player, far_intake, canister)
            },
        ];
        app.update();

        let lines = parts(&mut app)
            .iter()
            .filter(|(part, _)| matches!(part, PickupSightPart::Line { .. }))
            .count();
        assert_eq!(lines, 1, "one line per canister");
        let pose = line(&mut app, canister).expect("the nearer pair is drawn");
        let midpoint = (near_face + canister_position) * 0.5;
        assert!(
            (pose.translation - midpoint).length() < 1e-3,
            "the line follows the nearer face, not the farther one: {:?}",
            pose.translation
        );
        assert!(part(
            &mut app,
            PickupSightPart::Plate {
                intake: far_intake,
                arm: 0
            }
        )
        .is_none());
    }

    /// A real take into a hold with room for every item empties the
    /// canister into the hold, despawns it and voices the take, and the
    /// sight the real pass drew for it clears with the canister: nothing
    /// confirms the take on screen after it is gone.
    #[test]
    fn a_real_take_fills_the_hold_and_clears_the_sight() {
        let mut app = unfinished_integrity_physics_app();
        app.init_asset::<StandardMaterial>();
        app.init_resource::<CargoCanisterIdAllocator>();
        app.insert_resource(test_items());
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
                ShipInventory::new(&test_items(), 400_000, []),
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
                    capture_gap: Meters(1.0),
                    aperture_width: Meters(22.2),
                    aperture_height: Meters(15.3),
                    eject_speed: MetersPerSecond(3.0),
                    pull_range: Meters(40.0),
                    pull_speed: MetersPerSecond(5.0),
                }),
            ))
            .id();
        nova_gameplay::test_support::settle(&mut app);
        // Every spawned canister carries a minted runtime id, as production's
        // drops do: the intake pass reads no canister without one.
        let id = app
            .world_mut()
            .resource_mut::<CargoCanisterIdAllocator>()
            .mint();
        let canister = app
            .world_mut()
            .spawn((
                cargo_canister(
                    CargoCanister::new(&test_items(), &ItemRoleType::HullPlate.into(), 4),
                    Transform::from_xyz(0.0, 0.0, -2.85),
                    Vec3::ZERO,
                    AssetRef::from("canister.glb#Scene0"),
                ),
                id,
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
                .count(&ItemRoleType::HullPlate.into()),
            4
        );
        assert_eq!(takes.load(std::sync::atomic::Ordering::Relaxed), 1);
        assert!(
            parts(&mut app).is_empty(),
            "the sight clears in the frame the canister is taken"
        );
    }
}
