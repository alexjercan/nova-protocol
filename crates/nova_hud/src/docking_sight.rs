//! The docking sight: two plates and the line between them, drawn in world
//! space across the pair of ports `DOCK` would actually take.
//!
//! # What a pilot is solving
//!
//! A capture asks for three things at once - the two faces near enough, the
//! two axes opposed enough, and the two hulls calm enough - and the hull is
//! flown with a mouse that steers and an RCS that translates. Numbers answer
//! none of that in time. What answers it is a shape you fly INTO alignment:
//!
//! - a cross drawn flat across each port's retracted face, so a face reads as
//!   a PLATE rather than a point. Two plates that look parallel are two axes
//!   that are opposed;
//! - the line joining the two faces. Fly until it stands PERPENDICULAR to both
//!   plates and the pair is lined up; then close it and the pair is docked.
//!
//! So the whole approach is one sentence: square the plates, stand the line
//! up, close the gap.
//!
//! # Roll is not drawn
//!
//! Each plate's cross is built from a reference direction shared by both ends
//! (the player hull's own up), NOT from the port's own clocking. A cylindrical
//! port has no keyed feature, the capture envelope ignores roll, and a cross
//! that turned with its hull would show a misalignment that does not exist and
//! cannot be corrected - the pilot has no roll control to correct it with.
//!
//! # The ticks
//!
//! The gap line carries a tick every `capture_distance` out from the target
//! face. They are fixed-length in world units, so perspective shrinks them
//! with range: that is the depth cue. The count is the read - one tick left
//! means the next capture distance is the last one.
//!
//! # Where the numbers come from
//!
//! [`DockingPorts::nearest_pair`], which is the mechanic's own search with the
//! eligibility gate off. The sight therefore draws the pair `DOCK` would pick
//! and grades it against the same envelope, and it appears while the pair is
//! still far out of reach - which is the whole of an approach. An instrument
//! with its own copy of the pose math would eventually disagree with the verb
//! it is drawing.
//!
//! Those poses are avian's, one fixed step behind the eased hull this runs
//! beside. At the metre-per-second speeds a capture happens at, that is well
//! under a centimetre of lag, and it buys a sight that cannot lie about the
//! gate.

use bevy::{light::NotShadowCaster, prelude::*};
use nova_gameplay::prelude::*;
use nova_ship::prelude::*;

use super::holo_instruments::segment_transform;

/// `DockingSightPlugin` and the component its segments carry.
pub mod prelude {
    pub use super::{DockingSightPart, DockingSightPlugin};
}

/// How far out the sight starts drawing, engine units. Eight capture
/// distances: far enough that the plates are already square by the time the
/// gap matters, near enough that it is not scenery.
const DRAW_RANGE: f32 = 8.0;

/// Line radius, world units. The bore sight's, because the two instruments
/// are the same kind of thing - a line you look PAST, not at.
const LINE_RADIUS: f32 = 0.03;

/// Arm span of a face plate, world units.
///
/// Sized against the HULL, not against the port. Your own plate stands on your
/// own bow, and a chase camera looks at that bow THROUGH your ship: a cross
/// that fits inside the hull's silhouette is a cross you never see. At two and
/// a half cells the arms clear a small hull's shoulders, which is what makes
/// the near plate readable at all.
const PLATE_SPAN: f32 = 2.5;

/// How far off its own face a plate floats, world units. Enough that the cross
/// is not coplanar with the hatch it is drawn on - the measurement is the gap
/// line's, and it still runs face to face.
const PLATE_LIFT: f32 = 0.05;

/// Length of a range tick across the gap line, world units.
const TICK_SPAN: f32 = 0.4;

/// How much thinner a tick is than the lines it measures.
const TICK_THINNING: f32 = 0.6;

/// Most ticks drawn on one gap line. [`DRAW_RANGE`] over the smallest useful
/// capture distance; past this the ruler is noise.
const MAX_TICKS: u8 = 12;

/// Opacity of every part of the sight.
const SIGHT_ALPHA: f32 = 0.7;

/// One drawn part of the sight, and its IDENTITY for the reconcile pass: a
/// part is spawned when it starts being drawn, despawned when it stops, and
/// only re-posed in between.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, Reflect)]
#[reflect(Component)]
pub enum DockingSightPart {
    /// One arm of the cross drawn across a port's face. `port` is `0` for the
    /// player's own port and `1` for the locked ship's; `arm` is the two
    /// directions of the cross.
    Plate {
        /// Which end of the pair this plate sits on.
        port: u8,
        /// Which arm of that plate's cross.
        arm: u8,
    },
    /// The line joining the two faces: the angle read and the distance read.
    Gap,
    /// One range tick across the gap line, `step` capture distances out from
    /// the target face.
    Tick {
        /// How many capture distances out this tick stands.
        step: u8,
    },
}

/// Whether the gate a part is drawn for is satisfied.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SightTone {
    /// This part of the capture is satisfied right now.
    Holds,
    /// It is not yet.
    Wanting,
}

impl SightTone {
    fn of(holds: bool) -> Self {
        if holds {
            Self::Holds
        } else {
            Self::Wanting
        }
    }

    fn index(self) -> usize {
        match self {
            Self::Holds => 0,
            Self::Wanting => 1,
        }
    }

    /// The hue: nav cyan while a gate is still open, own-green once it is
    /// met. Same two meanings the rest of the HUD gives those hues.
    fn color(self) -> Color {
        match self {
            Self::Holds => nova_ui::theme::semantic::ALLY,
            Self::Wanting => crate::NAV_CYAN,
        }
    }
}

/// Shared mesh and materials, built on the first frame a sight is drawn. A
/// `Resource` rather than a `Local`, so an app that never docks never
/// allocates them.
#[derive(Resource, Default)]
struct DockingSightAssets {
    line_mesh: Option<Handle<Mesh>>,
    /// Indexed by [`SightTone`].
    line_material: [Option<Handle<StandardMaterial>>; 2],
}

impl DockingSightAssets {
    fn line_mesh(&mut self, meshes: &mut Assets<Mesh>) -> Handle<Mesh> {
        self.line_mesh
            .get_or_insert_with(|| meshes.add(Cylinder::new(LINE_RADIUS, 1.0)))
            .clone()
    }

    fn line_material(
        &mut self,
        materials: &mut Assets<StandardMaterial>,
        tone: SightTone,
    ) -> Handle<StandardMaterial> {
        self.line_material[tone.index()]
            .get_or_insert_with(|| {
                materials.add(StandardMaterial {
                    base_color: tone.color().with_alpha(SIGHT_ALPHA),
                    alpha_mode: AlphaMode::Blend,
                    unlit: true,
                    ..default()
                })
            })
            .clone()
    }
}

/// The two in-plane directions of a plate: `reference` flattened onto the face
/// and the axis crossed with it.
///
/// `None` when the reference lies along the axis and leaves nothing to flatten
/// - the caller tries the other reference rather than drawing a cross with an
/// arbitrary clocking.
fn plate_arms(axis: Vec3, reference: Vec3) -> Option<(Vec3, Vec3)> {
    let flattened = (reference - axis * reference.dot(axis)).try_normalize()?;
    Some((flattened, axis.cross(flattened)))
}

/// One part to draw this frame.
struct Drawn {
    part: DockingSightPart,
    pose: Transform,
    tone: SightTone,
}

/// Lay out the whole sight for one pair.
fn draw_pair(pair: &DockingPair, up: Vec3, forward: Vec3) -> Vec<Drawn> {
    let mut drawn = Vec::new();

    let facing = SightTone::of(pair.facing_holds());
    // Both plates take the SAME reference, which is what makes "these two
    // look parallel" mean "these two axes are opposed" and nothing else.
    for (port, pose) in [(0_u8, pair.first), (1, pair.second)] {
        let Some((first_arm, second_arm)) =
            plate_arms(pose.axis, up).or_else(|| plate_arms(pose.axis, forward))
        else {
            continue;
        };
        let centre = pose.face + pose.axis * PLATE_LIFT;
        for (arm, direction) in [(0_u8, first_arm), (1, second_arm)] {
            let reach = direction * PLATE_SPAN * 0.5;
            drawn.push(Drawn {
                part: DockingSightPart::Plate { port, arm },
                pose: segment_transform(centre - reach, centre + reach),
                tone: facing,
            });
        }
    }

    // Docked-close, the gap line has no direction left to point in and the
    // plates already sit on top of each other. Nothing to draw and nothing
    // left to fly.
    let Some(closing) = (pair.first.face - pair.second.face).try_normalize() else {
        return drawn;
    };
    // The gap line answers one question - may I close - and both halves of
    // that answer live on it: near enough, and calm enough to be allowed to.
    let approach = SightTone::of(pair.gap_holds() && pair.motion_holds());
    drawn.push(Drawn {
        part: DockingSightPart::Gap,
        pose: segment_transform(pair.second.face, pair.first.face),
        tone: approach,
    });

    let Some((tick_arm, _)) = plate_arms(closing, up).or_else(|| plate_arms(closing, forward))
    else {
        return drawn;
    };
    let spacing = pair.envelope.capture_distance;
    for step in 1..=MAX_TICKS {
        let out = f32::from(step) * spacing;
        if out >= pair.gap {
            break;
        }
        let centre = pair.second.face + closing * out;
        let reach = tick_arm * TICK_SPAN * 0.5;
        let mut pose = segment_transform(centre - reach, centre + reach);
        pose.scale.x *= TICK_THINNING;
        pose.scale.z *= TICK_THINNING;
        drawn.push(Drawn {
            part: DockingSightPart::Tick { step },
            pose,
            tone: approach,
        });
    }
    drawn
}

/// Reconcile the sight against the world, the way every other world-space
/// instrument does: a lock drops, a port is destroyed and a dock completes
/// between frames, and none of those are events this can subscribe to.
fn sync_docking_sight(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut assets: ResMut<DockingSightAssets>,
    ports: DockingPorts,
    q_player: Query<
        (
            Entity,
            &GlobalTransform,
            Option<&TravelLock>,
            Option<&ShipCapabilities>,
            Has<DockedShip>,
        ),
        (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>),
    >,
    // `Without<ChildOf>` is what keeps this disjoint from the mount chain
    // [`DockingPorts`] walks, which reads `Transform` on every parented
    // entity in the world. A sight part is spawned at the world root and has
    // no mount, so the two can never name the same entity - but Bevy grades
    // access by filter, not by archetype, and needs to be told.
    mut q_parts: Query<
        (
            Entity,
            &DockingSightPart,
            &mut Transform,
            &mut MeshMaterial3d<StandardMaterial>,
        ),
        Without<ChildOf>,
    >,
) {
    let drawn = q_player
        .single()
        .ok()
        .and_then(|(ship, global, travel, capabilities, docked)| {
            // A held hull has nothing to line up: the joint is already the
            // alignment, and the one key that still does anything undocks.
            if docked || !capabilities.copied().unwrap_or_default().dock_enabled {
                return None;
            }
            // The lock is the target selector. Without one there is no pair
            // to draw, which is also why `DOCK` needs one.
            let target = travel.and_then(|travel| travel.0)?;
            let pair = ports.nearest_pair(ship, target)?;
            (pair.gap <= DRAW_RANGE).then(|| {
                let (_, rotation, _) = global.to_scale_rotation_translation();
                draw_pair(&pair, rotation * Vec3::Y, rotation * Vec3::NEG_Z)
            })
        })
        .unwrap_or_default();

    // Re-pose what is already there; drop what stopped being drawn. The
    // material is written only when the tone actually changes, so a sight
    // holding one state does not flag itself dirty every frame.
    for (entity, part, mut transform, mut material) in &mut q_parts {
        match drawn.iter().find(|d| d.part == *part) {
            Some(d) => {
                *transform = d.pose;
                let wanted = assets.line_material(&mut materials, d.tone);
                if material.0 != wanted {
                    material.0 = wanted;
                }
            }
            None => commands.entity(entity).despawn(),
        }
    }

    for d in &drawn {
        if q_parts.iter().any(|(_, part, ..)| *part == d.part) {
            continue;
        }
        let mesh = assets.line_mesh(&mut meshes);
        let material = assets.line_material(&mut materials, d.tone);
        commands.spawn((
            Name::new("Docking Sight"),
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

/// Draws the world-space docking sight across the port pair `DOCK` would take
/// between the player ship and its lock. Inits `DockingSightAssets`, registers
/// [`DockingSightPart`], and runs `sync_docking_sight` in `Update` within
/// [`super::NovaHudSystems`].
#[derive(Default)]
pub struct DockingSightPlugin;

impl Plugin for DockingSightPlugin {
    fn build(&self, app: &mut App) {
        trace!("DockingSightPlugin: build");

        app.init_resource::<DockingSightAssets>();
        app.register_type::<DockingSightPart>();
        app.add_systems(Update, sync_docking_sight.in_set(super::NovaHudSystems));
    }
}

#[cfg(test)]
mod tests {
    use avian3d::prelude::*;
    use nova_gameplay::test_support::{settle, unfinished_integrity_physics_app};

    use super::*;

    /// A physics app running the ports and the sight, and nothing else. Real
    /// avian, because the poses the sight draws are avian's own.
    fn sight_app() -> App {
        let mut app = unfinished_integrity_physics_app();
        // The shared harness brings meshes but not materials.
        app.init_asset::<StandardMaterial>();
        app.add_plugins(DockingSectionPlugin { render: false });
        app.add_plugins(DockingSightPlugin);
        app.finish();
        app
    }

    /// One hull with one port on its nose, facing local -Z.
    fn spawn_ported_hull(app: &mut App, at: Vec3, rotation: Quat, player: bool) -> Entity {
        let mut hull = app.world_mut().spawn((
            Name::new("hull"),
            SpaceshipRootMarker,
            RigidBody::Dynamic,
            Transform::from_translation(at).with_rotation(rotation),
        ));
        if player {
            hull.insert(PlayerSpaceshipMarker);
        }
        let ship = hull.id();
        app.world_mut().spawn((
            ChildOf(ship),
            Name::new("hull block"),
            Transform::default(),
            Collider::cuboid(1.0, 1.0, 1.0),
            ColliderDensity(1.0),
        ));
        app.world_mut().spawn((
            ChildOf(ship),
            Name::new("port"),
            Transform::default(),
            docking_section(DockingSectionConfig::default()),
        ));
        ship
    }

    /// Two hulls nose to nose down Z, `gap` engine units between the two
    /// retracted faces, with the first locked onto the second.
    fn locked_pair(app: &mut App, gap: f32) -> (Entity, Entity) {
        let player = spawn_ported_hull(app, Vec3::ZERO, Quat::IDENTITY, true);
        let target = spawn_ported_hull(
            app,
            Vec3::NEG_Z * (1.0 + gap),
            Quat::from_rotation_y(std::f32::consts::PI),
            false,
        );
        app.world_mut()
            .entity_mut(player)
            .insert(TravelLock(Some(target)));
        settle(app);
        app.update();
        (player, target)
    }

    /// Every part the sight is drawing, with its pose.
    fn parts(app: &mut App) -> Vec<(DockingSightPart, Transform)> {
        let world = app.world_mut();
        let mut query = world.query::<(&DockingSightPart, &Transform)>();
        query
            .iter(world)
            .map(|(part, transform)| (*part, *transform))
            .collect()
    }

    fn part(app: &mut App, wanted: DockingSightPart) -> Option<Transform> {
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

    /// The whole point of the instrument: it is up while the pair is still
    /// far OUT of the envelope, because that is when a pilot is flying the
    /// approach. A sight that only drew eligible pairs would appear at the
    /// moment it stopped being needed.
    #[test]
    fn the_sight_is_up_while_the_pair_is_still_out_of_reach() {
        let mut app = sight_app();
        // Three cells apart against a one-cell capture distance.
        locked_pair(&mut app, 3.0);

        let drawn = parts(&mut app);
        assert!(
            drawn
                .iter()
                .any(|(part, _)| matches!(part, DockingSightPart::Gap)),
            "the gap line is drawn before the gap closes"
        );
        for port in 0..2 {
            for arm in 0..2 {
                assert!(
                    drawn
                        .iter()
                        .any(|(part, _)| *part == DockingSightPart::Plate { port, arm }),
                    "both faces are drawn as plates: {port}/{arm}"
                );
            }
        }
    }

    /// The gap line IS the measurement: it runs face to face, so its length
    /// is the number the capture is graded on.
    #[test]
    fn the_gap_line_runs_between_the_two_faces() {
        let mut app = sight_app();
        let gap = 2.0;
        locked_pair(&mut app, gap);

        let pose = part(&mut app, DockingSightPart::Gap).expect("the gap line is drawn");
        let (direction, length) = segment(pose);
        assert!(
            (length - gap).abs() < 1e-3,
            "the line is as long as the gap is wide: {length}"
        );
        assert!(
            direction.dot(Vec3::Z) > 0.99,
            "and runs from the target's face back to the player's: {direction:?}"
        );
    }

    /// A plate is the port's FACE. Both its arms therefore lie in that face,
    /// which is what makes "the line stands perpendicular to the plate" the
    /// same statement as "the two axes are lined up".
    #[test]
    fn a_plate_lies_flat_across_its_own_port_face() {
        let mut app = sight_app();
        // A quarter turn of yaw on the target, so the two axes are NOT
        // opposed and the two plates must visibly disagree.
        let player = spawn_ported_hull(&mut app, Vec3::ZERO, Quat::IDENTITY, true);
        let target = spawn_ported_hull(
            &mut app,
            Vec3::NEG_Z * 3.0,
            Quat::from_rotation_y(std::f32::consts::FRAC_PI_2),
            false,
        );
        app.world_mut()
            .entity_mut(player)
            .insert(TravelLock(Some(target)));
        settle(&mut app);
        app.update();

        for (port, axis) in [(0_u8, Vec3::NEG_Z), (1, Vec3::NEG_X)] {
            for arm in 0..2 {
                let pose = part(&mut app, DockingSightPart::Plate { port, arm })
                    .expect("both arms of both plates are drawn");
                let (direction, _) = segment(pose);
                assert!(
                    direction.dot(axis).abs() < 1e-3,
                    "arm {arm} of plate {port} lies in the face: {direction:?} against {axis:?}"
                );
            }
        }
    }

    /// The ruler: one tick per capture distance out from the target face, so
    /// the count answers "how many more of these to go".
    #[test]
    fn the_gap_line_carries_one_tick_per_capture_distance() {
        let mut app = sight_app();
        // The authored capture distance is one cell, so a three-cell gap
        // leaves ticks at one and two - never one ON the player's own face.
        locked_pair(&mut app, 3.0);

        let ticks: Vec<u8> = parts(&mut app)
            .into_iter()
            .filter_map(|(part, _)| match part {
                DockingSightPart::Tick { step } => Some(step),
                _ => None,
            })
            .collect();
        assert_eq!(ticks.len(), 2, "two whole capture distances fit: {ticks:?}");

        // The target's face stands at z -3.5 (half a cell off a hull centred
        // four cells out), so the first tick belongs one cell nearer.
        let pose = part(&mut app, DockingSightPart::Tick { step: 1 }).expect("the first tick");
        assert!(
            (pose.translation.z - (-2.5)).abs() < 1e-3,
            "and the first stands one capture distance off the target face: {}",
            pose.translation.z
        );
    }

    /// The colour IS the gate: each part is drawn in one of two materials, and
    /// which one says whether the half of the capture it belongs to holds.
    #[test]
    fn a_plate_changes_material_when_the_axes_come_square() {
        let mut app = sight_app();
        // A quarter turn off: the two axes are nowhere near opposed.
        let player = spawn_ported_hull(&mut app, Vec3::ZERO, Quat::IDENTITY, true);
        let target = spawn_ported_hull(
            &mut app,
            Vec3::NEG_Z * 3.0,
            Quat::from_rotation_y(std::f32::consts::FRAC_PI_2),
            false,
        );
        app.world_mut()
            .entity_mut(player)
            .insert(TravelLock(Some(target)));
        settle(&mut app);
        app.update();
        let wanting = plate_material(&mut app);

        // Square, and nothing else changed.
        app.world_mut()
            .entity_mut(target)
            .insert(Rotation(Quat::from_rotation_y(std::f32::consts::PI)));
        settle(&mut app);
        app.update();
        let holds = plate_material(&mut app);

        assert_ne!(
            wanting, holds,
            "the plates change colour when the facing gate comes good"
        );
    }

    /// The material handle on the first plate arm.
    fn plate_material(app: &mut App) -> Handle<StandardMaterial> {
        let world = app.world_mut();
        let mut query = world.query::<(&DockingSightPart, &MeshMaterial3d<StandardMaterial>)>();
        query
            .iter(world)
            .find(|(part, _)| matches!(part, DockingSightPart::Plate { .. }))
            .map(|(_, material)| material.0.clone())
            .expect("a plate is drawn")
    }

    /// Everything that takes the sight away, in the states it is actually
    /// taken away in.
    #[test]
    fn the_sight_is_down_without_a_lock_in_range_and_while_docked() {
        let mut app = sight_app();
        let (player, target) = locked_pair(&mut app, 2.0);
        assert!(!parts(&mut app).is_empty(), "the sight starts up");

        app.world_mut().entity_mut(player).insert(TravelLock(None));
        app.update();
        assert!(
            parts(&mut app).is_empty(),
            "the lock is the target selector: no lock, no pair to draw"
        );

        app.world_mut()
            .entity_mut(player)
            .insert(TravelLock(Some(target)));
        app.world_mut().entity_mut(player).insert(ShipCapabilities {
            dock_enabled: false,
            ..ShipCapabilities::default()
        });
        app.update();
        assert!(
            parts(&mut app).is_empty(),
            "a hull that may not dock is not offered a line to fly"
        );

        app.world_mut()
            .entity_mut(player)
            .insert(ShipCapabilities::default());
        app.update();
        assert!(!parts(&mut app).is_empty(), "and gets it back");

        // Far past the draw range: the instrument is an approach aid, not a
        // permanent line between two hulls that share a system.
        app.world_mut()
            .entity_mut(target)
            .insert(Position(Vec3::NEG_Z * 40.0));
        settle(&mut app);
        app.update();
        assert!(
            parts(&mut app).is_empty(),
            "and it is down again out of range"
        );
    }
}
