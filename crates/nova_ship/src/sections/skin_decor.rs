//! DECORATION: the greebles a style places on a derived skin.
//!
//! A decoration is a [`SectionFixture`] in the full sense - health, mass, a
//! collider, shot off and gone - and it is a child of the PLATE it stands on,
//! so a plate coming off takes its greebles and a destroyed section takes both.
//! Nothing here is behaviour: if shooting a thing off should cost the ship a
//! capability it is a section, not decoration.
//!
//! # Placement is a seeded solve
//!
//! [`solve_decor`] builds the [`skin_surface`] graph of the plates, runs
//! [`solve_greebles`] with the style's compiled rules and a seed hashed from
//! the style id alone, and turns each placed greeble into a plate index, a
//! fixture index and a quarter turn in the plate's own frame. There is no random
//! stream: every draw hashes the seed and the plate's key, so the same hull
//! wearing the same style always wears the same greebles, and the editor can
//! re-solve on every structure change without a flicker.
//!
//! # The plate fits the piece
//!
//! The solve runs BEFORE the plates are meshed. A plate that takes a piece is
//! reshaped so its plateau covers the piece's turned footprint
//! ([`ShellShape::seated`]): its chamfers, ramp ends and corner caps steepen,
//! and its feet stay put, so its seams with the neighbours keep their heights.
//! A piece no slope can seat on a plate is not a candidate for that plate: the
//! solve never draws it there. A piece wider than a cell seats nowhere, and
//! content lint refuses the style that names it.
//!
//! [`ShellShape::seated`]: super::shell_shape::ShellShape::seated

use std::f32::consts::FRAC_PI_2;

use avian3d::prelude::{Collider, ColliderDensity};
use bevy::prelude::*;
use nova_gameplay::prelude::{destructible_body, AssetRef, Health};

use crate::sections::{
    fixture::prelude::SectionFixture,
    greeble_solver::{greeble_seed, solve_greebles, GreebleSolution},
    shell_shape::REACH,
    shell_skin::{SkinPlate, SkinStructure},
    skin_style::{ShipStyleConfig, StyleFixtureConfig, StyleGreebles, DECOR_DENSITY},
    skin_surface::{skin_surface, slot_direction, SkinSurface},
};

/// The solve, the placement it answers with, the decoration bundle, and the
/// per-fixture tally a style author tunes against.
pub mod prelude {
    pub use super::{
        decor_body, decor_footprint, decor_pose, decor_tally, solve_decor, solve_seated_greebles,
        DecorPlacement, ShipDecorMarker,
    };
}

/// The smallest a decoration's collider box may be on any axis, in cells.
///
/// A zero-volume collider has no mass and nothing to hit, so a fixture authored
/// without one would be art a round passes through. Clamped rather than refused:
/// a style is data, and bad data must not stop a ship spawning.
const MIN_COLLIDER: f32 = 0.02;

/// One decoration the solve placed, as INDICES into the plates and the style's
/// fixtures.
///
/// Indices rather than entities or clones, because the same answer has to serve
/// a live ship (which spawns fixtures) and the editor's preview (which spawns
/// pictures of them), and neither should have to hand the solve its world.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct DecorPlacement {
    /// Which plate takes the piece, indexing the plates passed in.
    pub plate: usize,
    /// Which of the style's fixtures it is, indexing
    /// [`ShipStyleConfig::fixtures`].
    pub fixture: usize,
    /// Quarter turns about the plate's own outward axis that point the
    /// model's `+Z` at the cell slot holding the greeble's `up` link point.
    pub turns: u8,
}

/// Every decoration `style` places on the skin `plates` of `structure`, with
/// each plate that takes a piece reshaped to seat it.
///
/// Run it before the plates are meshed or spawned: a plate's shape is only
/// final once this has seated its piece. Empty, and no plate changed, when
/// the style has no greebles. A style whose rules do not compile places
/// nothing and logs an error: content lint and the mod merge refuse such a
/// style first, so this is reached only by a style built in code.
pub fn solve_decor(
    structure: &SkinStructure,
    plates: &mut [SkinPlate],
    style: &ShipStyleConfig,
) -> Vec<DecorPlacement> {
    let greebles = match style.compile_greebles() {
        Ok(Some(greebles)) => greebles,
        Ok(None) => return Vec::new(),
        Err(faults) => {
            for fault in faults {
                error!("solve_decor: style '{}': {fault}", style.id);
            }
            return Vec::new();
        }
    };
    let surface = skin_surface(structure, plates);
    let solution = solve_seated_greebles(plates, &surface, style, &greebles);
    let placements = decor_placements(plates, &surface, &greebles, &solution);
    for placement in &placements {
        let half = decor_footprint(&style.fixtures[placement.fixture], placement.turns);
        let plate = &mut plates[placement.plate];
        plate.shape = plate
            .shape
            .seated(half)
            .expect("the solve places a piece only on a plate that can seat it");
    }
    placements
}

/// The greeble solve of `surface` under `style`'s rules, drawing each piece
/// only on a plate of `plates` that can seat its turned footprint.
///
/// The solve [`solve_decor`] runs, for a caller that wants the solution itself
/// - a diagnostic drawing of the cells and links.
pub fn solve_seated_greebles(
    plates: &[SkinPlate],
    surface: &SkinSurface,
    style: &ShipStyleConfig,
    greebles: &StyleGreebles,
) -> GreebleSolution {
    solve_greebles(
        surface,
        &greebles.rules,
        greeble_seed(&style.id),
        |cell, placed| {
            greebles.fixture(placed.greeble).is_none_or(|fixture| {
                let plate = &plates[cell];
                let turns = plate_turns(plate, usize::from(placed.turns));
                let half = decor_footprint(&style.fixtures[fixture], turns);
                plate.shape.seated(half).is_some()
            })
        },
    )
}

/// The placements a solved surface holds: one per cell whose greeble has a
/// fixture, turned into the plate frame.
pub(crate) fn decor_placements(
    plates: &[SkinPlate],
    surface: &SkinSurface,
    greebles: &StyleGreebles,
    solution: &GreebleSolution,
) -> Vec<DecorPlacement> {
    debug_assert_eq!(surface.len(), plates.len());
    solution
        .cells
        .iter()
        .enumerate()
        .filter_map(|(plate, placed)| {
            let placed = (*placed)?;
            let fixture = greebles.fixture(placed.greeble)?;
            Some(DecorPlacement {
                plate,
                fixture,
                turns: plate_turns(&plates[plate], usize::from(placed.turns)),
            })
        })
        .collect()
}

/// The quarter turns, in `plate`'s own frame, that point a model's `+Z` at
/// cell slot `slot` of the surface graph.
///
/// The plate frame and the cell frame share the out axis, so one of the four
/// turns lies on the slot direction. The best aligned turn is taken, which is
/// that one.
fn plate_turns(plate: &SkinPlate, slot: usize) -> u8 {
    let target = slot_direction(plate.cell - plate.anchor, slot).as_vec3();
    (0..4u8)
        .max_by(|a, b| {
            let lean = |turns: u8| {
                (plate.rotation * (Quat::from_rotation_y(quarter(turns)) * Vec3::Z)).dot(target)
            };
            lean(*a).total_cmp(&lean(*b))
        })
        .unwrap_or_default()
}

/// The per-fixture tally a debug line prints: `id taken` pairs in the style's
/// own order, `taken` index-aligned with [`ShipStyleConfig::fixtures`].
pub fn decor_tally(style: &ShipStyleConfig, taken: &[usize]) -> String {
    style
        .fixtures
        .iter()
        .zip(taken)
        .map(|(fixture, taken)| format!("{} {taken}", fixture.id))
        .collect::<Vec<_>>()
        .join(", ")
}

/// The half extents, in cells along the plate's own `x` and `z`, of the
/// footprint `fixture` covers at `turns` quarter turns: its collider box,
/// turned as [`decor_pose`] turns the piece. An odd turn swaps the two.
///
/// The plate's own axes and not the bedded piece's: on a raked plate the
/// piece's footprint seen from the plate's out axis is shorter than this,
/// never longer, so a plateau that covers this covers the piece.
pub fn decor_footprint(fixture: &StyleFixtureConfig, turns: u8) -> Vec2 {
    let size = fixture.collider.max(Vec3::splat(MIN_COLLIDER));
    let half = Vec2::new(size.x, size.z) * 0.5;
    match turns % 2 {
        0 => half,
        _ => half.yx(),
    }
}

/// `turns` quarter turns as an angle.
fn quarter(turns: u8) -> f32 {
    f32::from(turns) * FRAC_PI_2
}

/// Where a decoration stands on the plate it is bolted to, and which way it is
/// stood up.
///
/// In the PLATE's frame, which is already the frame a greeble is authored in:
/// `+Y` is out of the plate and `y = 0` is the mounting face. The lift is the
/// height of the top under the plate's middle
/// ([`ShellShape::centre_height`]), so the piece stands ON the surface rather
/// than near it.
///
/// The piece is turned onto [`ShellShape::seat_normal`] first and yawed
/// second. Bedding it is what keeps a piece on a raked plate from standing on
/// one edge. Composed in that order because the yaw is a turn ACROSS THE
/// SURFACE - the quarter turn [`plate_turns`] chose against the plate's in-plane
/// axes - and turning first then bedding keeps that choice meaning what it
/// said.
///
/// [`ShellShape::centre_height`]: super::shell_shape::ShellShape::centre_height
/// [`ShellShape::seat_normal`]: super::shell_shape::ShellShape::seat_normal
pub fn decor_pose(plate: &SkinPlate, turns: u8) -> Transform {
    Transform::from_translation(Vec3::Y * (-REACH + plate.shape.centre_height())).with_rotation(
        Quat::from_rotation_arc(Vec3::Y, plate.shape.seat_normal())
            * Quat::from_rotation_y(quarter(turns)),
    )
}

/// Marks a decoration and carries the model that draws it.
///
/// The model rides on the marker so the render half can be an observer with
/// nothing but this component to read, exactly as a plate's shape rides on
/// `ShipSkinMarker`. A headless server spawns the marker and stops there.
#[derive(Component, Clone, Debug, Reflect)]
#[reflect(Component)]
pub struct ShipDecorMarker(#[reflect(ignore)] pub AssetRef<WorldAsset>);

/// The authored collider size a decoration's [`decor_collider`] was built
/// from. Carried alongside [`ShipDecorMarker`] so a freeze can rebuild the
/// collider at thaw instead of storing the un-serializable avian `Collider`
/// itself.
#[derive(Component, Clone, Copy, Debug, PartialEq, Reflect)]
pub struct DecorColliderSize(pub Vec3);

/// The whole of a decoration that is not a look: what takes damage, weighs
/// something and stops a round.
///
/// A [`SectionFixture`], so it is never structure, never in the integrity
/// graph, never in the ship's health and - through the ancestor walk in
/// `damage_cracks` - never graded by the damage of the hull under it. Its damage
/// read is that it comes off.
///
/// GAMEPLAY, all of it: a headless server builds this and never loads a mesh.
pub fn decor_body(fixture: &StyleFixtureConfig, pose: Transform) -> impl Bundle {
    (
        Name::new(format!("Skin Decor {}", fixture.id)),
        SectionFixture,
        ShipDecorMarker(fixture.model.clone()),
        DecorColliderSize(fixture.collider),
        pose,
        // Health, density and `Visibility::Inherited` in one bundle.
        destructible_body(fixture.health, DECOR_DENSITY),
        decor_collider(fixture.collider),
    )
}

/// A decoration rebuilt from a frozen record rather than placed fresh: the
/// exact `name`, `model`, `pose` and `health` [`frozen::freeze_section`](super::frozen::freeze_section)
/// captured, in place of the full-health piece [`decor_body`] would hand a
/// freshly placed one. The collider is rebuilt from `collider_size`, the
/// same way a fresh decoration's is, rather than carried in the frozen record.
///
/// `ShipDecorMarker` still lands on it, so [`dress_skin_decor`] resolves its
/// model exactly as it resolves a freshly placed one.
pub(crate) fn frozen_decor_body(
    name: String,
    model: AssetRef<WorldAsset>,
    pose: Transform,
    health: Health,
    collider_size: Vec3,
) -> impl Bundle {
    (
        Name::new(name),
        SectionFixture,
        ShipDecorMarker(model),
        DecorColliderSize(collider_size),
        pose,
        health,
        ColliderDensity(DECOR_DENSITY),
        Visibility::Inherited,
        decor_collider(collider_size),
    )
}

/// The box a decoration stops rounds with: the authored size, standing ON the
/// mount face rather than centred on the entity.
///
/// A CUBOID and not the model. A greeble is a handful of small solids with gaps
/// between them, a convex hull of one would swallow the gaps anyway, and the
/// difference at this size is centimeters. A COMPOUND of one box because that
/// is the only way to offset a collider from the entity carrying it, and a
/// piece is authored with its foot at `y = 0`.
fn decor_collider(size: Vec3) -> Collider {
    let size = size.max(Vec3::splat(MIN_COLLIDER));
    Collider::compound(vec![(
        Vec3::Y * size.y * 0.5,
        Quat::IDENTITY,
        Collider::cuboid(size.x, size.y, size.z),
    )])
}

/// Give a decoration the model that draws it, the moment one appears.
///
/// The RENDER half, and the whole of it. The model goes on the decoration's OWN
/// entity rather than on a child: a greeble is authored in exactly the frame the
/// fixture stands in, so a child would carry an identity transform and cost an
/// entity per piece over a hull that has hundreds.
pub(crate) fn dress_skin_decor(
    add: On<Add, ShipDecorMarker>,
    mut commands: Commands,
    q_decor: Query<&ShipDecorMarker>,
    asset_server: Res<AssetServer>,
) {
    let decor = add.entity;
    let Ok(ShipDecorMarker(model)) = q_decor.get(decor) else {
        return;
    };
    commands
        .entity(decor)
        .insert(WorldAssetRoot(model.resolve(&asset_server)));
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::sections::{
        shell_skin::derive_skin, skin_style::block_style, skin_surface::SURFACE_SLOTS,
    };

    /// A section that mates on every face, like a hull cube.
    const OPEN: [bool; 6] = [true; 6];

    /// A slab of hull `size` cells on a side, one cell thick.
    fn slab(size: i32) -> SkinStructure {
        let mut structure = SkinStructure::default();
        for x in 0..size {
            for z in 0..size {
                structure.insert(IVec3::new(x, 0, z), OPEN);
            }
        }
        structure
    }

    /// The same hull wearing the same style is solved to the same pieces on
    /// the same plates, and the block style dresses nearly every plate.
    #[test]
    fn the_same_hull_always_wears_the_same_decoration() {
        let mut structure = slab(6);
        structure.insert(IVec3::new(2, 1, 2), OPEN);
        let plates = derive_skin(&structure);
        let style = block_style(Vec3::splat(0.2));
        let first = solve_decor(&structure, &mut plates.clone(), &style);
        assert_eq!(first, solve_decor(&structure, &mut plates.clone(), &style));
        assert!(
            first.len() * 10 >= plates.len() * 9,
            "{} of {} plates dressed",
            first.len(),
            plates.len(),
        );
    }

    /// A placed model's `+Z` lies on the cell slot its turn names, on every
    /// plate of a hull with walls, steps and corners. This is the link between
    /// the solve's cell frame and the model's own frame.
    #[test]
    fn a_turn_points_the_model_at_its_slot_on_every_plate() {
        let mut structure = slab(4);
        structure.insert(IVec3::new(1, 1, 1), OPEN);
        structure.insert(IVec3::new(1, 2, 1), OPEN);
        for plate in derive_skin(&structure) {
            for slot in 0..SURFACE_SLOTS {
                let facing = plate.rotation
                    * (Quat::from_rotation_y(quarter(plate_turns(&plate, slot))) * Vec3::Z);
                let target = slot_direction(plate.cell - plate.anchor, slot).as_vec3();
                assert!(
                    facing.abs_diff_eq(target, 1e-4),
                    "plate {} slot {slot}: model +Z {facing}, slot {target}",
                    plate.cell,
                );
            }
        }
    }

    /// A piece is BEDDED on the plate it stands on: its own `+Y` comes out along
    /// the plane under the plate's middle, so it lies on a raked surface rather
    /// than standing upright in the cell.
    #[test]
    fn a_piece_is_bedded_on_the_surface_it_stands_on() {
        // A deck with a block on it: a level middle, and plates climbing the
        // block whose middle is their level plateau.
        let mut structure = slab(6);
        structure.insert(IVec3::new(2, 1, 2), OPEN);
        let plates = derive_skin(&structure);
        let at = |cell: IVec3| {
            plates
                .iter()
                .find(|plate| plate.cell == cell)
                .expect("a plate stands here")
        };
        for cell in [IVec3::new(4, 1, 4), IVec3::new(1, 1, 2)] {
            let plate = at(cell);
            let up = plate.rotation * decor_pose(plate, 0).rotation * Vec3::Y;
            assert!(
                up.abs_diff_eq(Vec3::Y, 1e-4),
                "a piece on the level middle of `{}` was turned to {up}",
                plate.shape.id(),
            );
        }

        // The low side of a diagonal stair is one raked surface, and a piece
        // on it leans exactly as far as the plate does.
        use crate::sections::shell_shape::{ShellEdgeType, ShellShape, FULL};
        let ramp = SkinPlate {
            cell: IVec3::ZERO,
            anchor: IVec3::NEG_Y,
            shape: ShellShape::new(
                FULL,
                [FULL, FULL, 0, FULL],
                [
                    ShellEdgeType::Wall,
                    ShellEdgeType::Plain,
                    ShellEdgeType::Ramp,
                    ShellEdgeType::Plain,
                ],
                [FULL; 4],
            )
            .expect("a legal shape"),
            rotation: Quat::IDENTITY,
        };
        assert!(ramp.shape.is_coplanar() && ramp.shape.tilt() > 0.7);
        let pose = decor_pose(&ramp, 0);
        assert!(
            (pose.rotation * Vec3::Y).abs_diff_eq(ramp.shape.top_normal(), 1e-4),
            "a piece on a ramp stands up {} against a surface facing {}",
            pose.rotation * Vec3::Y,
            ramp.shape.top_normal(),
        );
        let leaned = (pose.rotation * Vec3::Y)
            .dot(Vec3::Y)
            .clamp(-1.0, 1.0)
            .acos();
        assert!(
            (leaned - ramp.shape.tilt()).abs() < 1e-4,
            "the piece turned {leaned} rad on a plate leaning {} rad",
            ramp.shape.tilt(),
        );
    }

    /// A decoration stands ON the plate's surface, not inside it and not
    /// floating above it.
    #[test]
    fn a_piece_sits_on_the_surface_of_its_plate() {
        let structure = slab(5);
        let plates = derive_skin(&structure);
        let flat = plates
            .iter()
            .find(|plate| plate.cell == IVec3::new(2, 1, 2))
            .expect("a 5x5 deck has flat plate on its roof");
        let pose = decor_pose(flat, 0);
        assert!(
            (pose.translation.y - (-REACH + 0.5)).abs() < 1e-5,
            "a piece at {} is not on the plate's own top",
            pose.translation,
        );
    }
}
