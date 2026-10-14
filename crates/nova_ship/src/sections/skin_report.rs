//! The DUMP: a derived skin read back as facts, so a defect can be READ rather
//! than eyeballed.
//!
//! Every skin defect found so far was found by rendering a picture and looking
//! at it, and two rules were adopted from reasoning and later disproved by a
//! render. This module is the other half of that loop: given a structure, it
//! says what the derivation did to it and why, in numbers.
//!
//! It is worth building HERE and for this subsystem in particular, because the
//! skin is a PURE FUNCTION of structure - no random stream, and the greeble
//! solve hashes a seed from the style id. So a dumped structure is a COMPLETE repro: the derivation can be
//! re-run offline against it and pinned in a test, which is not true of most of
//! the game.
//!
//! Three things a reader wants and the derivation throws away:
//!
//! - per CLAD cell: the shape it wears, what the top is shaped like, the zone
//!   facts around it, and which piece the greeble solve put on it;
//! - per BARE cell a reader would expect to be clad: why it is not, in one
//!   word ([`BareReason`]);
//! - the STATISTICS a style is tuned against, above all how much of a hull the
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
    skin_decor::solve_decor,
    skin_reading::{fallen_corners, PlateReading, PlateRelief, RELIEFS},
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

/// How far a plate's top may lean off its own out face before it counts as a
/// leaning plate, in radians - about 15 degrees.
///
/// A property of the HULL rather than of a piece standing on it:
/// [`decor_pose`](super::skin_decor::decor_pose) beds a decoration onto the
/// plate's own top normal, so this is how much of the ship's surface is raked
/// rather than how much of its decoration is balanced. Used for the summary
/// only; nothing places by it.
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
    /// How many plates have a WHOLE flat top - a surface rather than a cone.
    pub coplanar: usize,
    /// The mean of every plate's [`flat_area`](ShellShape::flat_area), in
    /// square cells: how much room the average plate offers a piece.
    pub flat_area: f32,
    /// How many plates lean more than [`UPRIGHT_TILT`] off their own out face.
    pub leaning: usize,
    /// How many pieces each of the style's fixtures took, in the style's own
    /// order. Empty when no style is worn.
    pub rules: Vec<RuleCount>,
    /// How many pieces the greeble solve placed in total.
    pub decor: usize,
    /// How many of those stand on a plate that is NOT
    /// [`Flat`](PlateRelief::Flat) - the placement complaint, counted.
    pub decor_off_flat: usize,
    /// How many stand on a plate with no seat: a crease or a cap under the
    /// middle, where the piece stands.
    pub decor_on_creased: usize,
    /// The mean [`tilt`](ShellShape::tilt) of the plates carrying a piece, in
    /// radians - how raked the ground under the decoration is.
    ///
    /// The number the placement work is judged on, next to
    /// [`decor_on_creased`](SkinReport::decor_on_creased): a piece is bedded
    /// onto its plate, so this is no longer how far the piece leans off its
    /// own footing, but it is still how far the FOOTING leans off the ship.
    pub decor_tilt: f32,
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

/// How many pieces of one style fixture landed on one hull.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuleCount {
    /// The style fixture's id.
    pub id: String,
    /// How many pieces the greeble solve placed.
    pub taken: usize,
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
    /// The neighbourhood the plate stands in.
    pub reading: PlateReading,
    /// Which corner slots stand beside a falling edge, a bit per slot - see
    /// [`fallen_corners`].
    pub fallen: u8,
    /// Whether its top is ONE surface rather than a cone.
    pub coplanar: bool,
    /// The largest unbroken flat piece of its top, in square cells.
    pub flat_area: f32,
    /// How far that piece leans off the out face, in radians.
    pub tilt: f32,
    /// The piece the greeble solve put on it: none or one.
    pub decor: Vec<DecorReport>,
}

/// One piece of decoration on a plate.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DecorReport {
    /// The style fixture's id.
    pub id: String,
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
            coplanar: plate.shape.is_coplanar(),
            flat_area: plate.shape.flat_area(),
            tilt: plate.shape.tilt(),
            decor: Vec::new(),
        })
        .collect();

    let mut rules: Vec<RuleCount> = Vec::new();
    if let Some(style) = style {
        let mut taken = vec![0usize; style.fixtures.len()];
        for placement in solve_decor(structure, plates, style) {
            let fixture = &style.fixtures[placement.fixture];
            rows[placement.plate].decor.push(DecorReport {
                id: fixture.id.clone(),
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
            .filter(|row| !row.reading.seat)
            .map(|row| row.decor.len())
            .sum(),
        decor_tilt: match decorated().map(|row| row.decor.len()).sum::<usize>() {
            0 => 0.0,
            placed => {
                decorated()
                    .map(|row| row.tilt * row.decor.len() as f32)
                    .sum::<f32>()
                    / placed as f32
            }
        },
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
/// creased, how much room the average plate offers, and how much of the
/// decoration ended up somewhere flat.
///
/// [`relief_tally`]: super::skin_reading::relief_tally
pub fn skin_summary(report: &SkinReport) -> String {
    let plates = report.plates.len().max(1) as f32;
    let decor = report.decor.max(1) as f32;
    format!(
        "{} plate(s): {} flat-topped ({:.0}%), {} leaning; \
         mean flat area {:.3} cell^2; {} bare hull face(s) over {} bare cell(s); \
         {} piece(s), {} off flat ({:.0}%), {} on a creased top, mean tilt under \
         a piece {:.3} rad",
        report.plates.len(),
        report.coplanar,
        100.0 * report.coplanar as f32 / plates,
        report.leaning,
        report.flat_area,
        report.bare_faces,
        report.bare.len(),
        report.decor,
        report.decor_off_flat,
        100.0 * report.decor_off_flat as f32 / decor,
        report.decor_on_creased,
        report.decor_tilt,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sections::{
        shell_shape::{ShellEdgeType, FULL},
        shell_skin::derive_skin,
        skin_reading::read_plates,
        skin_style::block_style,
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

    /// THE OWNER'S L: the inside angle is CLAD, at every thickness.
    ///
    /// Dropping it for having no direction left to show is a refusal
    /// [`cladding_cells`] does not make: the plate on the roof and the plate up
    /// the wall hug different walls of the same corner and neither is in the
    /// other's way.
    #[test]
    fn the_inside_angle_of_an_l_is_clad_at_every_thickness() {
        let report = report(&elbow(3, 1), None);

        assert_eq!(
            report.bare,
            vec![],
            "the owner's bare section is bare again"
        );
        assert_eq!(report.bare_faces, 0, "an L has hull looking at vacuum");
        // The corner plate itself: bolted to the run below it, at the whole
        // cell and holding it along the edge it closes against the upright.
        let angle = at(&report, IVec3::new(1, 1, 0));
        assert_eq!(angle.anchor, IVec3::new(1, 0, 0));
        assert_eq!(angle.shape.level, FULL, "`{}`", angle.shape.id());
        assert!(
            angle.shape.edges.contains(&ShellEdgeType::Wall),
            "`{}` does not close against the upright",
            angle.shape.id(),
        );

        for arm in [2, 3] {
            for depth in [1, 3] {
                let thick = self::report(&elbow(arm, depth), None);
                assert_eq!(
                    thick.bare,
                    vec![],
                    "an elbow of {arm} by {depth} drops a cell"
                );
                for z in 0..depth {
                    assert_eq!(
                        at(&thick, IVec3::new(arm - 2, 1, z)).anchor,
                        IVec3::new(arm - 2, 0, z)
                    );
                }
            }
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

    /// A fitting leaves the deck beside it LEVEL.
    ///
    /// The owner's complaint was "raises next to turrets". A fitting stands
    /// PROUD of the plating and offers nothing to mate on its flanks, so the
    /// skin runs past it at its own height. What raises the deck is structure
    /// the plating has to CLIMB - a hull block standing proud.
    #[test]
    fn a_fitting_leaves_the_deck_beside_it_level() {
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

        // The block, though, is climbed.
        assert_eq!(roof(IVec3::new(1, 1, 2)).reading.relief, PlateRelief::Step);
    }

    /// The report is a pure function of the structure, exactly as the skin is.
    #[test]
    fn the_same_structure_always_reports_the_same() {
        let cells: Vec<IVec3> = (0..3)
            .flat_map(|x| (0..3).flat_map(move |z| (0..2).map(move |y| IVec3::new(x, y, z))))
            .collect();
        let mut reversed = cells.clone();
        reversed.reverse();
        let worn = block_style(Vec3::splat(0.2));

        let forwards = report(&hull(&cells), Some(&worn));
        let backwards = report(&hull(&reversed), Some(&worn));
        assert_eq!(forwards, backwards, "the report depends on insertion order",);
        assert_eq!(
            skin_summary(&forwards),
            skin_summary(&backwards),
            "the summary depends on insertion order",
        );
    }

    /// A wide deck's middle is one flat surface, and a plate climbing a block
    /// is a plateau with chamfers, which is what the flat area is FOR: it
    /// separates the two without a render.
    #[test]
    fn the_flat_area_separates_a_deck_from_a_chamfered_plate() {
        let mut structure = slab(5);
        structure.insert(IVec3::new(2, 1, 2), OPEN);
        let deck = report(&structure, None);
        let middle = at(&deck, IVec3::new(1, 1, 3));
        assert_eq!(middle.flat_area, 1.0, "the middle of a deck is one surface");
        assert!(middle.coplanar);
        assert_eq!(middle.tilt, 0.0);

        let climb = at(&deck, IVec3::new(1, 1, 2));
        assert!(!climb.coplanar, "`{}` is chamfered", climb.shape.id());
        assert!(
            climb.flat_area < 1.0 && climb.flat_area >= 0.25,
            "`{}` offers {}",
            climb.shape.id(),
            climb.flat_area,
        );
        assert_eq!(climb.tilt, 0.0, "its widest piece is the level plateau");
    }
}
