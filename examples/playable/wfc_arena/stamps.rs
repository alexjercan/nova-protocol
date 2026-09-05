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
pub fn stamp_large_drives(hull: &mut ShipHull, seed: u64, sections: &GameSections) {
    const SUPPORT_Z: f32 = 4.0;
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
    let beam = (Vec3::new(0.0, 0.0, SUPPORT_Z), Vec3::new(4.0, 0.5, 0.5));
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
        section.position.z + half.z <= 4.5 + GRID_EPSILON && !inside_beam
    });

    for index in 0..8 {
        let x = -3.5 + index as f32;
        hull.sections.push(stamped_section(
            format!("large_drive_support_{index}"),
            "reinforced_hull_section",
            Vec3::new(x, 0.0, SUPPORT_Z),
        ));
    }
    let centre_z = SUPPORT_Z + 0.5 + length * 0.5;
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
        let (sections, _) = catalog_tiles();
        for (seed, prototype, count) in [
            (0, "capital_thruster_section", 1),
            (1, "vector_thruster_section", 2),
            (2, "vector_thruster_section", 3),
        ] {
            let mut hull = ShipHull::default();
            stamp_large_drives(&mut hull, seed, &sections);
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
                8,
            );
        }
    }

    #[test]
    fn arena_stamp_replaces_only_the_central_support_beam() {
        let (sections, _) = catalog_tiles();
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
        stamp_large_drives(&mut hull, 20_260_829, &sections);
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
            stamp_large_drives(&mut hull, seed, &sections);
            refuse_unmated_contacts(&hull, &sections);
        }
    }
}
