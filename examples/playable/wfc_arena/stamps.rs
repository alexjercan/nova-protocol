//! The arena's own post-collapse STAMP: a seeded large-drive stern.
//!
//! Deliberately outside the collapse, and deliberately outside `nova_wfc`. A
//! grammar seeds ONE stern drive, and what the arena wants to bench is one
//! capital drive against two or three vector drives on the same hull - a
//! comparison no single grammar can express.
//!
//! The lance that used to be stamped beside it is a grammar role now
//! (`GrammarKeel::bow_gun`), so the arena names it and the collapse seeds the
//! pair itself. This is what is left.
//!
//! It stays here because the ARENA is what wants it. `wfc_ships` poses the
//! same collapse and never bolts anything onto it.

use bevy::prelude::*;
use nova_protocol::prelude::*;
use nova_wfc::prelude::*;

fn stamped_section(id: String, prototype: &str, position: Vec3) -> SpaceshipSectionConfig {
    SpaceshipSectionConfig {
        id,
        position,
        rotation: Quat::IDENTITY,
        source: SectionSource::Prototype(prototype.to_string()),
        modifications: vec![],
    }
}

/// Replace the arena PoC's unit-cell stern with a seeded large-drive stamp.
///
/// This is deliberately outside the collapse. Large sections are not WFC tiles,
/// and the production ship generator will own a richer grammar later. The
/// arena only needs a deterministic fleet for judging one capital drive against
/// two or three vector drives.
pub fn stamp_large_drives(
    hull: &mut ShipHull,
    seed: u64,
    sections: &GameSections,
    grid: GrammarGrid,
) {
    // Ship-space geometry the stamp has to agree with the collapse on. These
    // were three literals tuned for `half_width: 4, length: 11`, back when the
    // grid was a const in the same file as the collapse. The GRAMMAR authors it
    // now - a mod may overlay `standard_hull` with a longer one - so they are
    // read off the grid the hull was actually collapsed in.
    //
    // `Grid::starboard_half` starts the starboard half at `x = 0.5` and puts
    // the z origin at `-(length - 1) / 2`, so a mirrored hull's outer cell
    // centres sit at +/- (half_width - 0.5) and its aft-most row at
    // (length - 1) / 2. The beam takes the row one forward of that, and the
    // carve clears everything aft of the beam's own forward face.
    let beam_half_x = grid.half_width as f32;
    let support_z = (grid.length as f32 - 1.0) * 0.5 - 1.0;
    let carve_z = support_z + 0.5;
    let widest = |ids: &[&str]| -> f32 {
        ids.iter()
            .map(|id| {
                sections
                    .get_section(id)
                    .unwrap_or_else(|| panic!("wfc_arena: no section prototype '{id}'"))
                    .base
                    .collider
                    .unwrap_or_default()
            })
            .map(|collider| rotated_half_extents(collider, Quat::IDENTITY).x)
            .fold(0.0, f32::max)
    };
    // Refused by NAME rather than stamped crooked. The drive centres below are
    // authored bench positions, not derived ones, so a grid the beam cannot
    // carry is a grid this stamp was never tuned for.
    let needed = widest(&["capital_thruster_section", "vector_thruster_section"]);
    assert!(
        beam_half_x >= needed,
        "wfc_arena: the stamp bolts drives {needed} cell(s) either side of the keel onto a beam          only {beam_half_x} wide; grammar '{STANDARD_HULL_GRAMMAR_ID}' is {} cell(s) across its \
         half-width and this stamp was tuned for 4",
        grid.half_width
    );
    assert!(
        grid.length >= 3,
        "wfc_arena: the stamp clears the aft-most row and the one in front of it; grammar \
         '{STANDARD_HULL_GRAMMAR_ID}' is {} cell(s) long and there would be no hull left",
        grid.length
    );

    let (prototype, length, centres): (&str, f32, Vec<Vec3>) = match seed % 3 {
        0 => (
            "capital_thruster_section",
            3.0,
            vec![Vec3::new(0.5, 0.0, 0.0)],
        ),
        1 => (
            "vector_thruster_section",
            2.0,
            vec![Vec3::new(-1.5, 0.0, 0.0), Vec3::new(1.5, 0.0, 0.0)],
        ),
        _ if seed & 1 == 0 => (
            "vector_thruster_section",
            2.0,
            vec![
                Vec3::new(-3.5, 0.0, 0.0),
                Vec3::new(-0.5, 0.0, 0.0),
                Vec3::new(2.5, 0.0, 0.0),
            ],
        ),
        _ => (
            "vector_thruster_section",
            2.0,
            vec![
                Vec3::new(-2.5, 0.0, 0.0),
                Vec3::new(0.5, 0.0, 0.0),
                Vec3::new(3.5, 0.0, 0.0),
            ],
        ),
    };
    // The carve reads BODIES, not positions: a two-cell bay centred one row
    // forward of the transom still pokes its aft cell into the deck being
    // cleared, and keeping it would stand the stamp's own beam inside it.
    let beam = (
        Vec3::new(0.0, 0.0, support_z),
        Vec3::new(beam_half_x, 0.5, 0.5),
    );
    hull.sections.retain(|section| {
        let SectionSource::Prototype(id) = &section.source else {
            panic!("wfc_arena: every generated section is a catalog prototype");
        };
        let config = sections
            .get_section(id)
            .unwrap_or_else(|| panic!("wfc_arena: no section prototype '{id}'"));
        let half = rotated_half_extents(config.base.collider.unwrap_or_default(), section.rotation);
        let inside_beam = (section.position - beam.0)
            .abs()
            .cmplt(half + beam.1 - Vec3::splat(GRID_EPSILON))
            .all();
        section.position.z + half.z <= carve_z + GRID_EPSILON && !inside_beam
    });

    for index in 0..(2 * grid.half_width) {
        let x = -(beam_half_x - 0.5) + index as f32;
        hull.sections.push(stamped_section(
            format!("large_drive_support_{index}"),
            "reinforced_hull_section",
            Vec3::new(x, 0.0, support_z),
        ));
    }
    let centre_z = carve_z + length * 0.5;
    info!(
        "wfc_arena: seed {seed} stamped with {} `{prototype}` drive(s)",
        centres.len()
    );
    for (index, centre) in centres.into_iter().enumerate() {
        hull.sections.push(stamped_section(
            format!("large_drive_{index}"),
            prototype,
            Vec3::new(centre.x, centre.y, centre_z),
        ));
    }
}

#[cfg(test)]
mod stamp_tests {

    use super::*;

    /// Every contact a stamped hull leaves has to MATE, with one scoped
    /// exception.
    ///
    /// The stamped drives mate to the reinforced beam, but their wider forward
    /// face may also rest flush against random stern tiles that expose no
    /// matching socket. The game accepts that contact; the generator's own
    /// stricter check would flag it, so those drive-to-random-tile pairs are
    /// exempted here and every stamped BEAM mate is still checked.
    fn refuse_unmated_contacts(hull: &ShipHull, sections: &GameSections) {
        let drive = |index: usize| {
            hull.sections[index].id.starts_with("large_drive_")
                && !hull.sections[index].id.starts_with("large_drive_support_")
        };
        let stamped = |index: usize| hull.sections[index].id.starts_with("large_drive_");
        let exempt = |a: usize, b: usize| (drive(a) && !stamped(b)) || (drive(b) && !stamped(a));

        let placed = place(hull, sections).expect("every generated section is a prototype");
        let unmated =
            unmated_contacts(&placed, hull, &exempt).expect("the stamped hull derives a graph");
        assert!(
            unmated.is_empty(),
            "sections meet without mating:\n  {}",
            unmated.join("\n  ")
        );
    }

    /// The shipped catalog and the shipped grammar read against each other -
    /// the same pair the running example builds, out of the builders rather
    /// than off disk.
    fn catalog_tiles() -> (GameSections, TileSet) {
        let sections = GameSections(nova_authoring::generation::build_section_catalog());
        let grammars = GameGrammars(nova_authoring::generation::build_grammars());
        let tiles = TileSet::from_catalog(&sections, &grammars, STANDARD_HULL_GRAMMAR_ID)
            .expect("the shipped grammar reads against the shipped catalog");
        (sections, tiles)
    }

    #[test]
    fn arena_stamps_one_capital_or_two_to_three_vector_drives() {
        let (sections, tiles) = catalog_tiles();
        for (seed, prototype, count) in [
            (0, "capital_thruster_section", 1),
            (1, "vector_thruster_section", 2),
            (2, "vector_thruster_section", 3),
        ] {
            let mut hull = ShipHull::default();
            stamp_large_drives(&mut hull, seed, &sections, tiles.grid());
            assert_eq!(
                hull.sections
                    .iter()
                    .filter(|section| {
                        matches!(&section.source, SectionSource::Prototype(id) if id == prototype)
                    })
                    .count(),
                count,
            );
            assert_eq!(
                hull.sections
                    .iter()
                    .filter(|section| section.id.starts_with("large_drive_support_"))
                    .count(),
                2 * tiles.grid().half_width as usize,
            );
        }
    }

    /// The stamp follows the GRID, which is content now.
    ///
    /// `half_width` and `length` were three literals in this file - the beam's
    /// x extent, the row it takes, and the plane the carve clears to - tuned
    /// for the 4x5x11 the collapse then held as consts. A mod overlaying
    /// `standard_hull` with a longer hull used to move the transom out from
    /// under a beam that stayed where it was: the carve stripped rows that
    /// should have survived, and the beam was planted cells inside the hull.
    #[test]
    fn the_stamp_moves_its_beam_when_the_grammar_retunes_the_grid() {
        let sections = GameSections(nova_authoring::generation::build_section_catalog());
        let mut grammar = GameGrammars(nova_authoring::generation::build_grammars())
            .get_grammar(STANDARD_HULL_GRAMMAR_ID)
            .expect("the base content ships one")
            .clone();
        grammar.grid.half_width = 5;
        grammar.grid.length = 13;
        let tiles = TileSet::build(&sections, &grammar).expect("the retuned grid reads");

        let mut hull = tiles.hull(0, false, None).expect("the seed collapses");
        stamp_large_drives(&mut hull, 0, &sections, tiles.grid());

        let supports: Vec<&SpaceshipSectionConfig> = hull
            .sections
            .iter()
            .filter(|section| section.id.starts_with("large_drive_support_"))
            .collect();
        // 13 cells long puts the aft-most row at z = 6, so the beam takes 5.
        assert_eq!(
            supports.len(),
            10,
            "a hull 10 cells across takes 10 supports"
        );
        assert!(
            supports
                .iter()
                .all(|section| (section.position.z - 5.0).abs() < GRID_EPSILON),
            "the beam has to sit one row forward of the transom, not where an 11-cell hull \
             put it: {:?}",
            supports.iter().map(|s| s.position.z).collect::<Vec<_>>()
        );
        assert_eq!(
            supports
                .iter()
                .map(|section| section.position.x)
                .fold(f32::NEG_INFINITY, f32::max),
            4.5,
            "and it has to reach the outboard cell of the wider hull"
        );
        // The whole point of moving it: nothing the collapse laid forward of
        // the beam is carved, and nothing aft of it survives.
        assert!(
            hull.sections
                .iter()
                .any(|section| section.position.z > 3.0 && section.position.z < 5.0),
            "the carve stripped rows a 13-cell hull should have kept"
        );
        refuse_unmated_contacts(&hull, &sections);
    }

    #[test]
    fn arena_stamp_replaces_only_the_central_support_beam() {
        let (sections, tiles) = catalog_tiles();
        let mut hull = ShipHull {
            sections: vec![
                stamped_section(
                    "old_support_cell".to_string(),
                    "reinforced_hull_section",
                    Vec3::new(2.5, 0.0, 4.0),
                ),
                stamped_section(
                    "outside_drive_face".to_string(),
                    "reinforced_hull_section",
                    Vec3::new(2.5, -2.0, 4.0),
                ),
            ],
            ..default()
        };
        stamp_large_drives(&mut hull, 20_260_829, &sections, tiles.grid());
        assert!(hull
            .sections
            .iter()
            .all(|section| section.id != "old_support_cell"));
        assert!(hull
            .sections
            .iter()
            .any(|section| section.id == "outside_drive_face"));
    }

    #[test]
    fn seeded_large_drive_stamps_mate_to_generated_sterns() {
        let (sections, tiles) = catalog_tiles();
        for seed in [6, 7, 8, 20_260_829] {
            let mut hull = tiles.hull(seed, true, None).expect("the seed collapses");
            stamp_large_drives(&mut hull, seed, &sections, tiles.grid());
            refuse_unmated_contacts(&hull, &sections);
        }
    }
}
