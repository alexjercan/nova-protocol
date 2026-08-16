//! The DUMP: a derived skin read back as facts, so a defect can be READ rather
//! than eyeballed.
//!
//! Every skin defect found so far was found by rendering a picture and looking
//! at it, and two rules were adopted from reasoning and later disproved by a
//! render. This module is the other half of that loop: given a structure, it
//! says what the derivation did to it and why, in numbers.
//!
//! It is worth building HERE and for this subsystem in particular, because the
//! skin is a PURE FUNCTION of structure - no RNG, the scatter hashed off the
//! cell. So a dumped structure is a COMPLETE repro: the derivation can be
//! re-run offline against it and pinned in a test, which is not true of most of
//! the game.
//!
//! Three things a reader wants and the derivation throws away:
//!
//! - per CLAD cell: the eight boundary samples, the shape they spell, what the
//!   top is shaped like, the zone facts the scatter reads, and which pieces
//!   landed on it and by which of the two paths;
//! - per BARE cell a reader would expect to be clad: why it is not, in one
//!   word ([`BareReason`]);
//! - the STATISTICS a rule is tuned against, above all how much of a hull the
//!   creased plates make up.
//!
//! It is a THIRD pass, after [`derive_skin`](super::shell_skin::derive_skin)
//! and [`read_plates`](super::skin_reading::read_plates), and it takes both of
//! their answers as arguments rather than recomputing them - a caller that
//! clads a ship already holds them, and a report that re-derived could disagree
//! with the skin it describes.

use bevy::{platform::collections::HashSet, prelude::*};

use crate::sections::{
    shell_shape::ShellShape,
    shell_skin::{cladding_cells, exit_pocket, SkinPlate, SkinStructure, FACES},
    skin_decor::{decor_reach, scatter_decor, DecorReason},
    skin_reading::{fallen_corners, is_diagonal_saddle, PlateReading, PlateRelief, RELIEFS},
    skin_style::ShipStyleConfig,
};

/// The prelude: `skin_report`, `skin_summary`, `SkinReport`, `PlateReport`,
/// `DecorReport`, `BareCell`, `BareReason`, `ReliefCount` and `RuleCount`.
pub mod prelude {
    pub use super::{
        skin_report, skin_summary, BareCell, BareReason, DecorReport, PlateReport, ReliefCount,
        RuleCount, SkinReport,
    };
}

/// How far a plate's top may lean off its own out face before a piece standing
/// upright on it reads as leaning, in radians - about 15 degrees.
///
/// Nothing tilts a decoration to the surface under it
/// ([`decor_pose`](super::skin_decor::decor_pose) only lifts and yaws), so this
/// is the line between a piece that looks bolted down and one that looks
/// balanced. Used for the summary only; nothing places by it.
pub const UPRIGHT_TILT: f32 = 0.26;

/// Everything one derived skin is, as facts.
#[derive(Clone, Debug, PartialEq)]
pub struct SkinReport {
    /// One row per clad cell, in the derivation's own order (sorted by cell).
    pub plates: Vec<PlateReport>,
    /// One row per cell a reader would expect to be clad and which is not.
    pub bare: Vec<BareCell>,
    /// How many faces of STRUCTURE look straight at vacuum. The coverage rule's
    /// own claim is that this is zero; every other number here is a measurement,
    /// and this one is a defect count.
    pub bare_faces: usize,
    /// How many distinct shapes the skin came out as, which is how many meshes
    /// it costs.
    pub shapes: usize,
    /// How many plates wear each relief, in [`RELIEFS`] order.
    pub relief: Vec<ReliefCount>,
    /// How many plates are the diagonal SADDLE, which lives inside
    /// [`Spur`](PlateRelief::Spur). See
    /// [`is_diagonal_saddle`](super::skin_reading::is_diagonal_saddle).
    pub saddles: usize,
    /// How many plates have a WHOLE flat top - a surface rather than a cone.
    pub coplanar: usize,
    /// The mean of every plate's [`flat_area`](ShellShape::flat_area), in
    /// square cells: how much room the average plate offers a piece.
    pub flat_area: f32,
    /// How many plates lean more than [`UPRIGHT_TILT`] off their own out face.
    pub leaning: usize,
    /// What each of the style's rules took and what it could have taken, in the
    /// style's own order. Empty when no style is worn.
    pub rules: Vec<RuleCount>,
    /// How many pieces the scatter placed in total.
    pub decor: usize,
    /// How many of those stand on a plate that is NOT
    /// [`Flat`](PlateRelief::Flat) - the placement complaint, counted.
    pub decor_off_flat: usize,
    /// How many stand on a plate whose top is creased rather than one surface.
    pub decor_on_creased: usize,
    /// How many pieces stand on each relief, in [`RELIEFS`] order.
    pub decor_relief: Vec<ReliefCount>,
}

/// How many plates (or pieces) one relief class accounts for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReliefCount {
    /// The class counted.
    pub relief: PlateRelief,
    /// How many.
    pub count: usize,
}

/// What one of a style's rules did to one hull.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuleCount {
    /// The style fixture's id.
    pub id: String,
    /// How many pieces it actually placed.
    pub taken: usize,
    /// How many plates it could have stood on with nothing above it and no
    /// share thinning it - see
    /// [`decor_reach`](super::skin_decor::decor_reach).
    pub reach: usize,
}

/// One clad cell, and everything the derivation knew about it.
#[derive(Clone, Debug, PartialEq)]
pub struct PlateReport {
    /// The cell the plate fills.
    pub cell: IVec3,
    /// The cell of structure it bolts down to.
    pub anchor: IVec3,
    /// The face it shows to space, in the ship's cells.
    pub out: IVec3,
    /// The shape it wears, canonical, in its own frame.
    pub shape: ShellShape,
    /// The neighbourhood the scatter reads.
    pub reading: PlateReading,
    /// Which corner slots die to the cell floor, a bit per slot.
    pub fallen: u8,
    /// Whether it is the diagonal saddle.
    pub saddle: bool,
    /// Whether its top is ONE surface rather than a cone.
    pub coplanar: bool,
    /// The largest unbroken flat piece of its top, in square cells.
    pub flat_area: f32,
    /// How far that piece leans off the out face, in radians.
    pub tilt: f32,
    /// The pieces that landed on it, in the style's own order.
    pub decor: Vec<DecorReport>,
}

/// One piece of decoration, and why it is where it is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DecorReport {
    /// The style fixture's id.
    pub id: String,
    /// Which pass claimed the plate.
    pub reason: DecorReason,
    /// Quarter turns about the plate's out axis.
    pub turns: u8,
}

/// Why a cell that touches the hull carries no plate.
///
/// The four are the whole of [`cladding_cells`]'s refusals, one word each, so a
/// bare hull face in a render can be looked up rather than argued about.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BareReason {
    /// Nothing around it offers a socket the plating could bolt to. An
    /// authored hull face left bare, or the flank of a fitting.
    NoSocket,
    /// Something FIRES into it - a muzzle, a nozzle, the mouth of a bay - so a
    /// plate here would be a ship shooting its own plating.
    FiresInto,
    /// It is WEDGED: every face that offers something to bolt to has structure
    /// on the far side of the cell, so a plate here would be buried in the ship
    /// rather than the last thing before space.
    NoFooting,
    /// It was claimed and then no shape could be found for it, which is a bug
    /// rather than a refusal.
    NoShape,
}

impl BareReason {
    /// The stable lowercase name a dump spells this with.
    pub fn name(self) -> &'static str {
        match self {
            Self::NoSocket => "no_socket",
            Self::FiresInto => "fires_into",
            Self::NoFooting => "no_footing",
            Self::NoShape => "no_shape",
        }
    }
}

/// One cell of vacuum against the hull that carries no plate.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct BareCell {
    /// The empty cell.
    pub cell: IVec3,
    /// Why it is empty.
    pub reason: BareReason,
    /// How many faces of STRUCTURE look into it. A cell refused for a good
    /// reason still leaves this many hull faces showing.
    pub faces: usize,
}

/// Read a derived skin back as facts.
///
/// `plates` and `readings` are the derivation's own answers, index-aligned, so
/// the report cannot drift from the skin it describes. `style` is the look the
/// ship wears, or `None` for a ship wearing none - the decoration half is then
/// empty rather than absent.
///
/// A pure function of its arguments, and deterministic in every part: the
/// plates arrive sorted, the bare cells are sorted here, and nothing is
/// sampled. Two reports of one structure are equal.
pub fn skin_report(
    structure: &SkinStructure,
    plates: &[SkinPlate],
    readings: &[PlateReading],
    style: Option<&ShipStyleConfig>,
) -> SkinReport {
    let clad: HashSet<IVec3> = plates.iter().map(|plate| plate.cell).collect();
    let mut rows: Vec<PlateReport> = plates
        .iter()
        .zip(readings)
        .map(|(plate, reading)| PlateReport {
            cell: plate.cell,
            anchor: plate.anchor,
            out: reading.out,
            shape: plate.shape,
            reading: *reading,
            fallen: fallen_corners(&plate.shape),
            saddle: is_diagonal_saddle(&plate.shape),
            coplanar: plate.shape.is_coplanar(),
            flat_area: plate.shape.flat_area(),
            tilt: plate.shape.tilt(),
            decor: Vec::new(),
        })
        .collect();

    let mut rules: Vec<RuleCount> = Vec::new();
    if let Some(style) = style {
        let reach = decor_reach(plates, readings, style);
        let mut taken = vec![0usize; style.fixtures.len()];
        for placement in scatter_decor(plates, readings, style) {
            let fixture = &style.fixtures[placement.fixture];
            rows[placement.plate].decor.push(DecorReport {
                id: fixture.id.clone(),
                reason: placement.reason,
                turns: placement.turns,
            });
            taken[placement.fixture] += 1;
        }
        rules = style
            .fixtures
            .iter()
            .enumerate()
            .map(|(index, fixture)| RuleCount {
                id: fixture.id.clone(),
                taken: taken[index],
                reach: reach.get(index).copied().unwrap_or_default(),
            })
            .collect();
    }

    let bare = bare_cells(structure, &clad);
    let shapes: HashSet<ShellShape> = plates.iter().map(|plate| plate.shape).collect();
    let tally = |counted: &dyn Fn(PlateRelief) -> usize| -> Vec<ReliefCount> {
        RELIEFS
            .into_iter()
            .map(|relief| ReliefCount {
                relief,
                count: counted(relief),
            })
            .collect()
    };
    let decorated = || rows.iter().filter(|row| !row.decor.is_empty());

    SkinReport {
        bare_faces: bare.iter().map(|cell| cell.faces).sum(),
        shapes: shapes.len(),
        relief: tally(&|relief| {
            rows.iter()
                .filter(|row| row.reading.relief == relief)
                .count()
        }),
        saddles: rows.iter().filter(|row| row.saddle).count(),
        coplanar: rows.iter().filter(|row| row.coplanar).count(),
        flat_area: match rows.is_empty() {
            true => 0.0,
            false => rows.iter().map(|row| row.flat_area).sum::<f32>() / rows.len() as f32,
        },
        leaning: rows.iter().filter(|row| row.tilt > UPRIGHT_TILT).count(),
        decor: rows.iter().map(|row| row.decor.len()).sum(),
        decor_off_flat: decorated()
            .filter(|row| row.reading.relief != PlateRelief::Flat)
            .map(|row| row.decor.len())
            .sum(),
        decor_on_creased: decorated()
            .filter(|row| !row.coplanar)
            .map(|row| row.decor.len())
            .sum(),
        decor_relief: tally(&|relief| {
            decorated()
                .filter(|row| row.reading.relief == relief)
                .map(|row| row.decor.len())
                .sum()
        }),
        rules,
        bare,
        plates: rows,
    }
}

/// Every cell of vacuum against the hull that carries no plate, and why.
///
/// The reasons are read in the order [`cladding_cells`] applies them, so each
/// cell is answered by the FIRST rule that refused it - which is the one a
/// reader has to change to clad it.
fn bare_cells(structure: &SkinStructure, clad: &HashSet<IVec3>) -> Vec<BareCell> {
    let mut candidates: HashSet<IVec3> = HashSet::new();
    for cell in structure.filled_cells() {
        for face in FACES {
            let next = cell + face;
            if !structure.filled(next) && !clad.contains(&next) {
                candidates.insert(next);
            }
        }
    }
    // Where the derivation was willing to put skin, which is NOT the same as
    // where a plate ended up: a cell in here with no plate on it is one
    // `plate_for` could find no shape for, and that is a bug rather than a
    // refusal.
    let claimed = cladding_cells(structure);

    let mut bare: Vec<BareCell> = candidates
        .into_iter()
        .map(|cell| {
            let touches_socket = (0..FACES.len())
                .any(|face| structure.offers(cell + FACES[face], face ^ 1) == Some(true));
            // The EXIT is read first, though `cladding_cells` applies the socket
            // test first, and the order is the whole usefulness of the answer:
            // a part may not carry a socket on the face it fires through, so
            // the cell in front of every muzzle fails BOTH tests. "Nothing to
            // bolt to" would then be the answer for every gun well on the ship,
            // and it is the less actionable of the two.
            let reason = if exit_pocket(structure, cell) {
                BareReason::FiresInto
            } else if !touches_socket {
                BareReason::NoSocket
            } else if !claimed.contains(&cell) {
                BareReason::NoFooting
            } else {
                BareReason::NoShape
            };
            BareCell {
                cell,
                reason,
                faces: FACES
                    .iter()
                    .filter(|face| structure.filled(cell - **face))
                    .count(),
            }
        })
        .collect();
    bare.sort_unstable_by_key(|bare| (bare.cell.x, bare.cell.y, bare.cell.z));
    bare
}

/// The report as ONE LINE a human reads, next to the relief and decoration
/// tallies the spawner already logs.
///
/// Deliberately not the relief histogram again - [`relief_tally`] prints that.
/// What is here is what nobody could answer before: how much of a hull is
/// creased, how much of it is the saddle, how much room the average plate
/// offers, and how much of the decoration ended up somewhere flat.
///
/// [`relief_tally`]: super::skin_reading::relief_tally
pub fn skin_summary(report: &SkinReport) -> String {
    let plates = report.plates.len().max(1) as f32;
    let decor = report.decor.max(1) as f32;
    format!(
        "{} plate(s): {} flat-topped ({:.0}%), {} saddle(s) ({:.0}%), {} leaning; \
         mean flat area {:.3} cell^2; {} bare hull face(s) over {} bare cell(s); \
         {} piece(s), {} off flat ({:.0}%), {} on a creased top",
        report.plates.len(),
        report.coplanar,
        100.0 * report.coplanar as f32 / plates,
        report.saddles,
        100.0 * report.saddles as f32 / plates,
        report.leaning,
        report.flat_area,
        report.bare_faces,
        report.bare.len(),
        report.decor,
        report.decor_off_flat,
        100.0 * report.decor_off_flat as f32 / decor,
        report.decor_on_creased,
    )
}

#[cfg(test)]
mod tests {
    use nova_gameplay::prelude::AssetRef;

    use super::*;
    use crate::sections::{
        shell_shape::{FULL, HALF, TOP_FACET_FOOTPRINT},
        shell_skin::derive_skin,
        skin_reading::read_plates,
        skin_style::{ScatterRule, StyleFixtureConfig},
    };

    /// A section that mates on every face, like a hull cube.
    const OPEN: [bool; 6] = [true; 6];

    fn hull(cells: &[IVec3]) -> SkinStructure {
        let mut structure = SkinStructure::default();
        for cell in cells {
            structure.insert(*cell, OPEN);
        }
        structure
    }

    /// A slab of hull `size` cells on a side, one cell thick.
    fn slab(size: i32) -> SkinStructure {
        let cells: Vec<IVec3> = (0..size)
            .flat_map(|x| (0..size).map(move |z| IVec3::new(x, 0, z)))
            .collect();
        hull(&cells)
    }

    fn report(structure: &SkinStructure, style: Option<&ShipStyleConfig>) -> SkinReport {
        let plates = derive_skin(structure);
        let readings = read_plates(structure, &plates);
        skin_report(structure, &plates, &readings, style)
    }

    /// The row for the plate standing in `cell`.
    fn at(report: &SkinReport, cell: IVec3) -> &PlateReport {
        report
            .plates
            .iter()
            .find(|row| row.cell == cell)
            .unwrap_or_else(|| panic!("no plate stands at {cell:?}"))
    }

    fn style(fixtures: Vec<StyleFixtureConfig>) -> ShipStyleConfig {
        ShipStyleConfig {
            id: "test".to_string(),
            name: "Test".to_string(),
            surfaces: Vec::new(),
            fixtures,
        }
    }

    fn fixture(id: &str, scatter: ScatterRule) -> StyleFixtureConfig {
        StyleFixtureConfig {
            id: id.to_string(),
            model: AssetRef::from("self://gltf/greebles/placeholder_block.glb#Scene0".to_string()),
            health: 10.0,
            density: 0.1,
            collider: Vec3::new(0.2, 0.1, 0.2),
            scatter,
        }
    }

    /// An L of hull cubes with arms `arm` cells long and `depth` cells thick,
    /// lying in x/y - the owner's own build.
    fn elbow(arm: i32, depth: i32) -> SkinStructure {
        let mut cells: Vec<IVec3> = (0..arm).map(|x| IVec3::new(x, 0, 0)).collect();
        cells.extend((1..arm).map(|y| IVec3::new(arm - 1, y, 0)));
        hull(
            &cells
                .into_iter()
                .flat_map(|cell| (0..depth).map(move |z| cell + IVec3::Z * z))
                .collect::<Vec<IVec3>>(),
        )
    }

    /// THE OWNER'S L, and the two things he saw on it: a bare section at the
    /// inside angle, and a hull that reads as a field of spikes.
    ///
    /// The inside angle is CLAD, at every thickness. It used to be dropped for
    /// having no direction left to show, which is a refusal
    /// [`cladding_cells`] no longer makes - the plate on the roof and the plate
    /// up the wall hug different walls of the same corner and neither is in the
    /// other's way.
    ///
    /// The spikes are the OTHER reading, and they survive the corner being
    /// filled, which is the owner's hypothesis refuted in both directions:
    ///
    /// - an elbow with arms of TWO is just as spiky and never had a bare cell;
    /// - the same L three cells thick is much less spiky and had THREE, one per
    ///   slice.
    ///
    /// They are two independent readings of one property: the L has no interior.
    /// [`cladding_cells`] never looks at a height - it runs before any sample is
    /// read - so a shape cannot decide which cells are clad.
    #[test]
    fn the_inside_angle_of_an_l_is_clad_and_the_spikes_are_a_separate_reading() {
        let report = report(&elbow(3, 1), None);

        assert_eq!(
            report.bare,
            vec![],
            "the owner's bare section is bare again"
        );
        assert_eq!(report.bare_faces, 0, "an L has hull looking at vacuum");
        // The corner plate itself: bolted to the run below it, standing full
        // height where it closes against the upright.
        let angle = at(&report, IVec3::new(1, 1, 0));
        assert_eq!(angle.anchor, IVec3::new(1, 0, 0));
        assert_eq!(
            angle.shape.corners.iter().filter(|h| **h == FULL).count(),
            2,
            "`{}` does not close against the upright",
            angle.shape.id(),
        );

        // ...and the spikes, which are the whole hull rather than the corner: an
        // L is one cell thick everywhere, so almost no corner of any plate has
        // four cells around it that carry the surface on.
        assert_eq!(report.plates.len(), 21);
        assert_eq!(report.coplanar, 0, "not one plate on the L is a surface");
        assert_eq!(report.saddles, 0, "and not one of them is the saddle");
        assert!(report.flat_area < 0.3);

        // The hypothesis, both ways round. Arms of two: as spiky, nothing bare.
        let short = self::report(&elbow(2, 1), None);
        assert_eq!(short.bare, vec![], "an elbow of twos is clad all over");
        assert!(
            short.coplanar <= 1 && short.flat_area < 0.35,
            "and it is no less spiky for it: {} flat-topped, mean {}",
            short.coplanar,
            short.flat_area,
        );

        // Three cells thick: less spiky, and the three cells that used to be
        // dropped at the angle are the three that carry its fillet.
        let deep = self::report(&elbow(3, 3), None);
        assert!(
            deep.coplanar > 0 && deep.flat_area > report.flat_area,
            "thickening the L should take spikes off it",
        );
        assert_eq!(
            deep.bare,
            vec![],
            "a thick L still drops cells at the angle"
        );
        for z in 0..3 {
            assert_eq!(at(&deep, IVec3::new(1, 1, z)).anchor, IVec3::new(1, 0, z));
        }
    }

    /// A BARE HULL FACE is named, with the reason it is bare and how much hull
    /// it leaves showing.
    ///
    /// The owner's standing complaint is "hulls that are bare", and until now
    /// the only way to find one was to look at a picture of the ship. A drive
    /// firing off the deck is the commonest cause: the coverage rule refuses
    /// the cell in front of a muzzle on purpose, and the face under it is then
    /// structure looking at vacuum.
    #[test]
    fn a_bare_hull_face_is_named_with_the_reason_it_is_bare() {
        let mut structure = slab(5);
        // A drive standing on the deck, bolted down through its base and firing
        // straight up out of the top of its own cell.
        let mut drive = [false; 6];
        drive[3] = true;
        structure.insert(IVec3::new(2, 1, 2), drive);
        structure.insert_exit(IVec3::new(2, 1, 2), 2);
        let report = report(&structure, None);

        assert_eq!(
            report.bare,
            vec![BareCell {
                cell: IVec3::new(2, 2, 2),
                reason: BareReason::FiresInto,
                faces: 1,
            }],
            "a drive on a deck should leave its own nozzle mouth open and \
             nothing else",
        );
        assert_eq!(report.bare_faces, 1);

        // A plain deck leaves NOTHING bare, which is what makes the count above
        // a defect reading rather than a constant.
        assert_eq!(self::report(&slab(5), None).bare_faces, 0);

        // The muzzle cell fails BOTH refusals - a part may not socket the face
        // it fires through - and the report answers with the one a reader can
        // act on. Take the exit away and the same cell reads the other way.
        let mut blind = slab(5);
        blind.insert(IVec3::new(2, 1, 2), drive);
        let bare = self::report(&blind, None).bare;
        assert_eq!(bare.len(), 1);
        assert_eq!(bare[0].reason, BareReason::NoSocket);
    }

    /// The RAISE next to a fitting is not where reasoning puts it.
    ///
    /// The owner's complaint is "raises next to turrets", and this pins where
    /// the raise actually comes from. A fitting stands PROUD of the plating and
    /// offers nothing to mate on its flanks, so the skin runs past it at its own
    /// height and the deck beside a nozzle stays flat and level. What raises the
    /// deck is structure the plating has to CLIMB - a hull block standing proud -
    /// and even then only its four DIAGONAL neighbours come out creased, because
    /// an edge neighbour climbs it as one clean ramp.
    ///
    /// Both halves were guessed the other way round before the dump could
    /// answer, which is the whole reason it exists.
    #[test]
    fn a_fitting_leaves_the_deck_level_and_a_proud_block_creases_its_diagonals() {
        let mut structure = slab(7);
        let mut drive = [false; 6];
        drive[3] = true;
        structure.insert(IVec3::new(5, 1, 5), drive);
        structure.insert_exit(IVec3::new(5, 1, 5), 2);
        // A hull block on the same deck, which offers a socket on every face.
        structure.insert(IVec3::new(2, 1, 2), OPEN);
        let report = report(&structure, None);
        let roof = |cell: IVec3| at(&report, cell);

        for beside in [IVec3::new(4, 1, 5), IVec3::new(5, 1, 4)] {
            let row = roof(beside);
            assert_eq!(row.reading.fitting, 1, "{beside:?} is beside the nozzle");
            assert_eq!(
                row.reading.relief,
                PlateRelief::Flat,
                "the skin runs PAST a fitting rather than climbing it",
            );
            assert!(row.coplanar && row.tilt == 0.0, "and stays level doing it");
        }

        // The block, though, is climbed. Square-on that is one clean ramp...
        let ramp = roof(IVec3::new(1, 1, 2));
        assert_eq!(ramp.reading.relief, PlateRelief::Step);
        assert!(ramp.coplanar, "an edge neighbour climbs a block as a ramp");
        assert_eq!(ramp.flat_area, 1.0);

        // ...and on the DIAGONAL it is a cone, with three quarters of the plate
        // creased away. This is the shape a greeble beside a raise stands on.
        let corner = roof(IVec3::new(1, 1, 1));
        assert_eq!(corner.reading.relief, PlateRelief::Step);
        assert!(!corner.coplanar, "the diagonal of a raise is a crease");
        assert_eq!(corner.flat_area, 0.25);
    }

    /// The diagonal SADDLE is counted apart from the rest of `Spur`.
    ///
    /// The next lane wants to refuse to decorate it, and the falsifier for that
    /// is "the saddle is common enough that refusing it starves a style" - which
    /// is a number, and this is where it comes from.
    #[test]
    fn the_diagonal_saddle_is_counted_apart_from_the_rest_of_spur() {
        let shape = |corners: [u8; 4], midpoints: [u8; 4]| {
            ShellShape::new(corners, midpoints).expect("a legal shape")
        };
        // Two corners across the diagonal on the floor: falls two ways, and the
        // two ways cancel, so nothing can be leaned off it.
        assert!(is_diagonal_saddle(&shape([HALF, 0, HALF, 0], [1, 1, 1, 1])));
        // Three corners down is a spar tip, not a saddle.
        assert!(!is_diagonal_saddle(&shape([HALF, 0, 0, 0], [1, 0, 0, 1])));
        // The same diagonal pattern with a sample at the whole cell is a STEP -
        // it climbs something, and the mask is not read at all.
        assert!(!is_diagonal_saddle(&shape(
            [FULL, 0, HALF, 0],
            [2, 1, 1, 2]
        )));

        // On a real hull the saddles are a subset of the spurs, always.
        let mut structure = slab(6);
        structure.insert(IVec3::new(2, 1, 2), OPEN);
        structure.insert(IVec3::new(4, 1, 4), OPEN);
        let report = report(&structure, None);
        let spurs = report
            .relief
            .iter()
            .find(|count| count.relief == PlateRelief::Spur)
            .map(|count| count.count)
            .unwrap_or_default();
        assert!(
            report.saddles <= spurs,
            "{} saddle(s) against {spurs} spur(s)",
            report.saddles,
        );
        for row in &report.plates {
            assert!(
                !row.saddle || row.reading.fall == IVec3::ZERO,
                "the saddle at {:?} claims to fall {}",
                row.cell,
                row.reading.fall,
            );
        }
    }

    /// The decoration half says WHICH pass put each piece down.
    ///
    /// A rule carried entirely by the per-patch floor looks exactly like a
    /// well-tuned one from a screenshot, and the two want opposite fixes.
    #[test]
    fn a_piece_says_whether_its_own_share_or_the_floor_put_it_there() {
        let structure = slab(8);
        let thin = ScatterRule {
            relief: vec![PlateRelief::Flat],
            chance: 0.02,
            patch: 3,
            ..default()
        };
        let report = report(&structure, Some(&style(vec![fixture("panel", thin)])));

        let placed: Vec<&DecorReport> = report
            .plates
            .iter()
            .flat_map(|row| row.decor.iter())
            .collect();
        assert!(!placed.is_empty(), "the floor should put pieces back");
        assert!(
            placed
                .iter()
                .any(|piece| piece.reason == DecorReason::Patch),
            "a share of 0.02 over a 8x8 deck is carried by the floor",
        );
        assert_eq!(report.decor, placed.len());
        assert_eq!(report.rules[0].taken, placed.len());
        assert!(report.rules[0].reach > placed.len(), "the reach is a bound");
    }

    /// The report is a pure function of the structure, exactly as the skin is.
    #[test]
    fn the_same_structure_always_reports_the_same() {
        let cells: Vec<IVec3> = (0..3)
            .flat_map(|x| (0..3).flat_map(move |z| (0..2).map(move |y| IVec3::new(x, y, z))))
            .collect();
        let mut reversed = cells.clone();
        reversed.reverse();
        let worn = style(vec![fixture(
            "vent",
            ScatterRule {
                chance: 0.5,
                ..default()
            },
        )]);

        let forwards = report(&hull(&cells), Some(&worn));
        let backwards = report(&hull(&reversed), Some(&worn));
        assert_eq!(forwards, backwards, "the report depends on insertion order",);
        assert_eq!(
            skin_summary(&forwards),
            skin_summary(&backwards),
            "the summary depends on insertion order",
        );
    }

    /// A wide deck's middle is one flat surface and a lone cube's plates are
    /// cones, which is what the flat area is FOR: it separates the two without
    /// a render.
    #[test]
    fn the_flat_area_separates_a_deck_from_a_stud() {
        let deck = report(&slab(5), None);
        let middle = at(&deck, IVec3::new(2, 1, 2));
        assert_eq!(middle.flat_area, 1.0, "the middle of a deck is one surface");
        assert!(middle.coplanar);
        assert_eq!(middle.tilt, 0.0);

        let cube = report(&hull(&[IVec3::ZERO]), None);
        assert_eq!(cube.plates.len(), 6);
        assert_eq!(cube.coplanar, 0, "every face of a lone cube is a stud");
        // A stud is a four-sided pyramid: each side is two facets of the fan,
        // so the widest flat piece of it is a quarter of the cell.
        assert!(
            (cube.flat_area - 2.0 * TOP_FACET_FOOTPRINT).abs() < 1e-5,
            "a stud offers a pyramid side, read {}",
            cube.flat_area,
        );
        assert_eq!(cube.bare_faces, 0, "a lone cube is wrapped");
    }

    /// WHICH relief classes are actually creased, which is the measurement the
    /// interior work is sized against.
    ///
    /// Measured on three generated hulls (526 plates) at the same time as this
    /// was written, and the split is the same one: `Flat` and `Brink` are whole
    /// flat surfaces, `Bevel`, `Ridge` and `Spur` are cones every time, and
    /// `Step` goes both ways - square-on to a raise it is a ramp, on the
    /// diagonal it is a cone. So the relief class alone does NOT say whether a
    /// plate has a surface to stand a piece on, and a rule that assumed it
    /// would be wrong about a fifth of a hull.
    ///
    /// Every flat area is 1.0 or 0.25 and nothing between, on those hulls and
    /// on this one: a plate is one whole surface or a cone with one facet pair
    /// usable. There is no middle to tune a seat size against.
    #[test]
    fn the_creased_classes_are_bevel_ridge_and_spur_and_half_the_steps() {
        // A deck with a raise on it and a corner bitten out: enough to make
        // every class but the stud.
        let mut cells: Vec<IVec3> = (0..6)
            .flat_map(|x| (0..6).map(move |z| IVec3::new(x, 0, z)))
            .collect();
        cells.retain(|cell| *cell != IVec3::new(5, 0, 5));
        cells.push(IVec3::new(2, 1, 2));
        let report = report(&hull(&cells), None);

        let mut split: Vec<(PlateRelief, usize, usize)> = Vec::new();
        for relief in RELIEFS {
            let of: Vec<&PlateReport> = report
                .plates
                .iter()
                .filter(|row| row.reading.relief == relief)
                .collect();
            split.push((
                relief,
                of.iter().filter(|row| row.coplanar).count(),
                of.iter().filter(|row| !row.coplanar).count(),
            ));
        }
        let count = |wanted: PlateRelief| {
            split
                .iter()
                .find(|(relief, _, _)| *relief == wanted)
                .map(|(_, flat, creased)| (*flat, *creased))
                .expect("every class is listed")
        };

        assert_eq!(count(PlateRelief::Flat).1, 0, "a flat panel is a surface");
        assert_eq!(count(PlateRelief::Brink).1, 0, "so is a hull edge - a ramp");
        assert!(count(PlateRelief::Brink).0 > 8, "the deck has four edges");
        assert_eq!(count(PlateRelief::Bevel).0, 0, "a bevel is always a cone");
        assert_eq!(count(PlateRelief::Spur).0, 0, "and so is a spar tip");
        assert!(count(PlateRelief::Spur).1 > 0, "the deck has outer corners");
        let (ramps, cones) = count(PlateRelief::Step);
        assert!(
            ramps > 0 && cones > 0,
            "a step is a ramp square-on to a raise and a cone on its diagonal: \
             {ramps} against {cones}",
        );

        for row in &report.plates {
            assert!(
                row.flat_area == 1.0 || row.flat_area == 2.0 * TOP_FACET_FOOTPRINT,
                "`{}` offers {} - there is supposed to be no middle",
                row.shape.id(),
                row.flat_area,
            );
            assert_eq!(
                row.coplanar,
                row.flat_area == 1.0,
                "`{}` disagrees with itself about its own top",
                row.shape.id(),
            );
        }
    }
}
