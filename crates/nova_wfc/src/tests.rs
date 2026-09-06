//! What the collapse has to keep doing, checked against the shipped content.
//!
//! The catalog and the grammar come out of the BUILDERS rather than off disk,
//! so these run without an asset server and fail on a content change rather
//! than on a missing file.

use bevy::prelude::UVec3;
use nova_scenario::prelude::{SectionSource, ShipHull, SpaceshipSectionConfig};
use nova_ship::prelude::{
    GameGrammars, GameSections, GrammarGrid, GrammarPart, GrammarZone, ShipGrammarConfig,
    STANDARD_HULL_GRAMMAR_ID,
};

use crate::{
    check::{place, unmated_contacts},
    grid::FACES,
    prelude::*,
    tiles::{compatible, upright_tile, VACUUM},
};

/// The shipped catalog and the shipped grammar read against each other - the
/// same pair the running game builds.
fn catalog_tiles() -> (GameSections, TileSet) {
    let sections = GameSections(nova_authoring::generation::build_section_catalog());
    let grammars = GameGrammars(nova_authoring::generation::build_grammars());
    let tiles = TileSet::from_catalog(&sections, &grammars, STANDARD_HULL_GRAMMAR_ID)
        .expect("the shipped grammar reads against the shipped catalog");
    (sections, tiles)
}

/// Nothing meets without mating, and the hull derives a graph.
fn refuse_unmated_contacts(hull: &ShipHull, sections: &GameSections) {
    let placed = place(hull, sections).expect("every generated section is a prototype");
    let unmated = unmated_contacts(&placed, hull, &|_, _| false).expect("the hull derives a graph");
    assert!(
        unmated.is_empty(),
        "sections meet without mating:\n  {}",
        unmated.join("\n  ")
    );
}

#[test]
fn the_two_cell_bay_becomes_a_pair_of_tiles_joined_across_their_shared_face() {
    let (_, set) = catalog_tiles();
    let tiles = &set.tiles;
    let bays: Vec<usize> = (0..tiles.len())
        .filter(|index| {
            tiles[*index]
                .part
                .as_ref()
                .is_some_and(|part| part.prototype == "torpedo_section")
        })
        .collect();
    assert!(
        !bays.is_empty(),
        "the 1x1x2 bay produced no tiles at all, which is the regression that emptied every \
         generated hull of bays"
    );

    let hull = upright_tile(tiles, "reinforced_hull_section").expect("the hull cube tiles");
    for &index in &bays {
        let tile = &tiles[index];
        let joints: Vec<usize> = (0..FACES.len())
            .filter(|face| tile.joints[*face].is_some())
            .collect();
        assert_eq!(joints.len(), 1, "a two-cell segment has exactly one joint");
        let face = joints[0];
        let partner = tile.joints[face].expect("just filtered on it");

        assert_eq!(
            tiles[partner].joints[face ^ 1],
            Some(index),
            "the partner joints back across the shared face"
        );
        assert!(
            tile.emits != tiles[partner].emits,
            "exactly one segment of the pair emits the placed section"
        );
        assert!(
            compatible(tiles, index, face, partner),
            "a joint accepts its own partner"
        );
        assert!(
            !compatible(tiles, index, face, VACUUM),
            "a joint refuses vacuum: half a bay is not a part"
        );
        assert!(
            !compatible(tiles, index, face, hull),
            "a joint refuses a hull cube pressed into the part's inside"
        );
    }

    for &index in bays.iter().filter(|index| tiles[**index].emits) {
        assert!(
            tiles[index].exit.is_some(),
            "the emitting segment is the muzzle end, and it carries the exit"
        );
    }
}

#[test]
fn generated_hulls_roll_two_cell_bays() {
    let (sections, tiles) = catalog_tiles();
    let carries_bay = |hull: &ShipHull| {
        hull.sections.iter().any(|section| {
            matches!(&section.source, SectionSource::Prototype(id) if id == "torpedo_section")
        })
    };
    let armed = (0..64)
        .map(|seed| tiles.hull(seed, true, None).expect("the seed collapses"))
        .find(carries_bay)
        .expect("no hull in seeds 0..64 rolled a bay, which is what starved the arena draft");
    refuse_unmated_contacts(&armed, &sections);
}

/// A seed is a NAME for a hull. The arena pins one and replays a matchup from
/// it, and the editor shows the builder the number that made what is on screen;
/// both are lies if the same seed can come out twice.
#[test]
fn one_seed_names_one_hull() {
    let (_, tiles) = catalog_tiles();
    for seed in [0, 7, 20_260_816] {
        let first = tiles.hull(seed, true, None).expect("the seed collapses");
        let second = tiles.hull(seed, true, None).expect("the seed collapses");
        assert_eq!(first.sections.len(), second.sections.len());
        for (a, b) in first.sections.iter().zip(&second.sections) {
            assert_eq!(a.id, b.id, "seed {seed} placed a different section");
            assert!(a.position.abs_diff_eq(b.position, GRID_EPSILON));
            assert!(a.rotation.abs_diff_eq(b.rotation, GRID_EPSILON));
        }
    }
}

/// The grammar is read at BUILD time, so a mod that names a part the catalog
/// does not hold gets a line rather than a hull with a hole in its draw.
#[test]
fn a_grammar_naming_an_absent_prototype_is_refused_with_the_id_in_the_line() {
    let sections = GameSections(nova_authoring::generation::build_section_catalog());
    let mut grammar = nova_authoring::generation::build_grammars()
        .into_iter()
        .find(|grammar| grammar.id == STANDARD_HULL_GRAMMAR_ID)
        .expect("the base grammar ships");
    grammar.parts[0].prototype = "no_such_section".to_string();

    let Err(error) = TileSet::build(&sections, &grammar) else {
        panic!("an absent prototype is refused")
    };
    assert!(
        error.contains("no_such_section"),
        "the refusal names the id that did not resolve: {error}"
    );
}

/// The contradiction path, which `lib.rs` makes a headline claim about:
/// failing rather than photographing a half-collapsed grid.
///
/// The unary filters cannot reach it on their own - `VACUUM` passes all of
/// them and is compatible with every solid, so a domain always keeps at least
/// emptiness. What CAN reach it is a JOINT: a multi-cell part is seeded by its
/// minimum corner and propagation lays the rest of the block out, and across a
/// joint face vacuum is not an option, so the partner segment is the only
/// thing that fits. Zone a multi-cell part into a region too short to hold it
/// and the cell past the boundary has the partner struck and nothing else
/// left.
///
/// A three-cell lance seeded on the bow of a six-cell grid: the bow third is
/// `z * 3 < 6`, so `z = 2` is amidships and the lance's last segment has
/// nowhere to be. `runnable` passes it - the grid IS long enough for the gun
/// and the drive - which is what makes this the solver's own refusal rather
/// than the gate's.
#[test]
fn a_grammar_that_collapses_into_a_contradiction_is_refused_rather_than_photographed() {
    let sections = GameSections(nova_authoring::generation::build_section_catalog());
    let mut grammar = shipped_grammar();
    grammar.grid.length = 6;
    grammar.keel.bow_gun = Some(LANCE.to_string());
    grammar.parts.push(GrammarPart {
        prototype: LANCE.to_string(),
        weight: 1.0,
        aim: None,
        zone: Some(GrammarZone::Bow),
    });

    let refused = TileSet::build(&sections, &grammar).and_then(|set| set.hull(0, false, None));
    let Err(line) = refused else {
        panic!("a grid that cannot hold the block it seeds has to come back as a line");
    };
    assert!(
        line.contains("collapsed to nothing"),
        "refused, but for `{line}` rather than for the contradiction"
    );

    // The same grammar with the zone lifted collapses, so it is the zone that
    // cut the block and not the shorter grid.
    grammar.parts.last_mut().expect("just pushed").zone = None;
    assert!(
        TileSet::build(&sections, &grammar)
            .and_then(|set| set.hull(0, false, None))
            .is_ok(),
        "the six-cell grid itself is fine; only the zone that cuts the lance is not"
    );
}

/// The lance the test above cuts in half: three cells long, so a zone boundary
/// can fall inside it.
const LANCE: &str = "railgun_lance_section";

/// Every hull the collapse hands out is one the game would ACCEPT. The check
/// is the game's own lint, not a copy of it.
#[test]
fn generated_hulls_clear_the_games_own_content_lint() {
    let (sections, tiles) = catalog_tiles();
    for seed in 0..8u64 {
        let hull = tiles.hull(seed, true, None).expect("the seed collapses");
        let errors = hull_errors(&hull, &sections);
        assert!(
            errors.is_empty(),
            "seed {seed} produced content the game would refuse:\n  {}",
            errors.join("\n  ")
        );
        refuse_unmated_contacts(&hull, &sections);
    }
}

/// The shipped grammar, for a test to bend one field of and watch refused.
fn shipped_grammar() -> ShipGrammarConfig {
    GameGrammars(nova_authoring::generation::build_grammars())
        .get_grammar(STANDARD_HULL_GRAMMAR_ID)
        .expect("the base content ships one")
        .clone()
}

/// A grammar is CONTENT: a mod ships one and the editor's Generate block
/// builds one out of what the builder ticked. Every table below reaches an
/// unchecked subtraction, an index past the end of the grid, or an empty
/// `rand` range if it is let through, and each of those takes the game down.
///
/// The point of the test is the absence of a panic as much as the `Err`: it
/// runs `build`, which is where the collapse would otherwise start indexing.
#[test]
fn a_grammar_the_collapse_cannot_run_in_is_refused_rather_than_run() {
    let sections = GameSections(nova_authoring::generation::build_section_catalog());
    // Each row carries the line it must be refused WITH: an `Err` alone cannot
    // tell "the gate caught this" from "something further in fell over first",
    // and the whole point of the gate is that the caller gets a line naming
    // what to change.
    let bent = [
        ("half_width", "across its half-width", {
            let mut grammar = shipped_grammar();
            grammar.grid.half_width = 1;
            grammar
        }),
        ("height", "cell(s) tall", {
            let mut grammar = shipped_grammar();
            grammar.grid.height = 0;
            grammar
        }),
        ("length", "cell(s) long", {
            let mut grammar = shipped_grammar();
            grammar.grid.length = 1;
            grammar
        }),
        ("a grid too big to hold", "stops at", {
            let mut grammar = shipped_grammar();
            // Exactly 2^32 cells, the product that used to wrap to a grid of
            // nothing rather than be refused.
            grammar.grid = GrammarGrid {
                half_width: 2048,
                height: 2048,
                length: 1024,
            };
            grammar
        }),
        ("a negative weight", "which is not a weight", {
            let mut grammar = shipped_grammar();
            grammar.parts[0].weight = -1.0;
            grammar
        }),
        ("a weight that is not a number", "which is not a weight", {
            let mut grammar = shipped_grammar();
            grammar.parts[0].weight = f32::NAN;
            grammar
        }),
        ("vacuum priced at infinity", "prices vacuum base", {
            let mut grammar = shipped_grammar();
            grammar.vacuum.base = f32::INFINITY;
            grammar
        }),
        ("nothing drawable", "draws nothing", {
            let mut grammar = shipped_grammar();
            for part in &mut grammar.parts {
                part.weight = 0.0;
            }
            grammar
        }),
        ("no parts at all", "draws nothing", {
            let mut grammar = shipped_grammar();
            grammar.parts.clear();
            grammar
        }),
    ];
    for (what, expected, grammar) in bent {
        let refused = TileSet::build(&sections, &grammar).and_then(|set| set.hull(0, false, None));
        let Err(line) = refused else {
            panic!("a grammar with {what} has to come back as a line, not as a hull");
        };
        assert!(
            line.contains(expected),
            "a grammar with {what} is refused, but for `{line}` rather than for {expected}"
        );
    }
}

/// The draw is handed a per-cell weight that has already been divided by how
/// many orientations of a part are still standing there, so a table the gate
/// accepts can still cancel to nothing in one cell. `rand` panics on an empty
/// range, so the draw carries its own floor.
#[test]
fn a_cell_whose_taste_cancelled_out_still_draws_something() {
    let sections = GameSections(nova_authoring::generation::build_section_catalog());
    let mut grammar = shipped_grammar();
    // Legal by the gate - one part carries the whole draw - and every other
    // weight is zero, which is what leaves most cells with nothing to spend.
    for part in &mut grammar.parts {
        part.weight = 0.0;
    }
    grammar.parts[0].weight = f32::MIN_POSITIVE;
    grammar.vacuum = nova_ship::prelude::GrammarVacuum::default();
    let set = TileSet::build(&sections, &grammar).expect("the gate accepts this one");
    for seed in 0..8 {
        // Either answer is fine. What is not fine is a panic.
        let _ = set.hull(seed, false, None);
    }
}

/// The shipped grammar with `id` seeded as its stern drive and drawn beside
/// the parts it already draws, in a grid of the given size.
fn grammar_driven_by(id: &str, half_width: u32, height: u32, length: u32) -> ShipGrammarConfig {
    let mut grammar = shipped_grammar();
    grammar.grid.half_width = half_width;
    grammar.grid.height = height;
    grammar.grid.length = length;
    grammar.keel.stern_drive = id.to_string();
    let aim = grammar
        .parts
        .iter()
        .find(|part| part.prototype == "basic_thruster_section")
        .and_then(|part| part.aim);
    assert!(
        aim.is_some(),
        "the shipped grammar aims its drives aft, which is the rule a multi-cell drive has \
         to survive: only its exhaust layer carries an exit"
    );
    if !grammar.parts.iter().any(|part| part.prototype == id) {
        grammar.parts.push(GrammarPart {
            prototype: id.to_string(),
            weight: 4.0,
            aim,
            zone: None,
        });
    }
    grammar
}

/// A drive several cells ACROSS is laid as one block, not refused for being
/// too wide to run along its own z axis.
///
/// The catalog authors the mating face of a big drive as one link point per
/// cell of it - nine for the 3x3x2, twenty-five for the 5x5x3 - so the
/// adjacency rule needs to know nothing about how big a part is. What the
/// collapse needs is the BLOCK: every cell of it claimed, held together by
/// joints that admit exactly the neighbouring segment.
#[test]
fn a_drive_wider_than_one_cell_is_read_as_a_block_of_joined_cells() {
    let sections = GameSections(nova_authoring::generation::build_section_catalog());
    for (id, span) in [
        ("vector_thruster_section", UVec3::new(3, 3, 2)),
        ("capital_thruster_section", UVec3::new(5, 5, 3)),
    ] {
        let set = TileSet::build(&sections, &grammar_driven_by(id, 7, 7, 15))
            .unwrap_or_else(|why| panic!("'{id}' reads against the catalog: {why}"));
        let block: Vec<usize> = (0..set.tiles.len())
            .filter(|index| {
                set.tiles[*index]
                    .part
                    .as_ref()
                    .is_some_and(|part| part.prototype == id)
            })
            .collect();
        let cells = (span.x * span.y * span.z) as usize;
        assert_eq!(
            block.len() % cells,
            0,
            "'{id}' is {span:?} cells, so each of its orientations is {cells} tiles"
        );
        assert_eq!(
            block
                .iter()
                .filter(|index| set.tiles[**index].emits)
                .count(),
            block.len() / cells,
            "exactly one cell of each block emits the placed section"
        );

        // An interior cell of the 5x5x3 is joined on all six faces; a corner of
        // any block on three. Nothing is joined on none, or it would be loose.
        for &index in &block {
            let joints = set.tiles[index]
                .joints
                .iter()
                .filter(|joint| joint.is_some())
                .count();
            assert!(
                (3..=6).contains(&joints),
                "a cell of '{id}' is held by {joints} joints"
            );
            let mut turned = set.tiles[index].span.to_array();
            let mut authored = span.to_array();
            turned.sort_unstable();
            authored.sort_unstable();
            assert_eq!(
                turned, authored,
                "every cell knows the whole part's span, turned into grid axes with it"
            );
        }
    }
}

/// A big drive reaches a hull, and it reaches it on EVERY seed.
///
/// It is seeded rather than rolled for: it wants nine or twenty-five exhaust
/// lanes clear at once, which the collapse only stumbles on, and every one
/// drawn into the middle of a hull is eroded again for firing into it. Seeding
/// puts it where a drive goes and leaves the transom around it to the roll.
#[test]
fn a_drive_bigger_than_one_cell_reaches_the_transom_of_every_hull() {
    let sections = GameSections(nova_authoring::generation::build_section_catalog());
    for (id, half_width, height, length) in [
        // The 3x3x2 fits the grid the game already ships.
        ("vector_thruster_section", 4, 5, 11),
        ("capital_thruster_section", 6, 5, 13),
    ] {
        let grammar = grammar_driven_by(id, half_width, height, length);
        let set = TileSet::build(&sections, &grammar).expect("the grammar builds");
        for seed in 0..8u64 {
            let hull = set
                .hull(seed, true, None)
                .unwrap_or_else(|why| panic!("'{id}' seed {seed}: {why}"));
            let drives = hull
                .sections
                .iter()
                .filter(|section| matches!(&section.source, SectionSource::Prototype(p) if p == id))
                .count();
            assert_eq!(
                drives, 2,
                "'{id}' is seeded once and mirrored, so seed {seed} owes the hull a PAIR"
            );
            let errors = hull_errors(&hull, &sections);
            assert!(
                errors.is_empty(),
                "'{id}' seed {seed} produced content the game would refuse:\n  {}",
                errors.join("\n  ")
            );
            refuse_unmated_contacts(&hull, &sections);
        }
    }
}

/// A grid too small for the drive it seeds is refused with the size it wants,
/// rather than collapsing into a domain that empties deep in the solve.
#[test]
fn a_grid_too_small_for_its_stern_drive_says_how_big_it_has_to_be() {
    let sections = GameSections(nova_authoring::generation::build_section_catalog());
    for (id, half_width, height, length, wanted) in [
        // 5 across needs a sixth column for the seam beside it.
        ("capital_thruster_section", 4, 5, 11, "needs 6"),
        ("capital_thruster_section", 6, 4, 11, "needs 5"),
        ("capital_thruster_section", 6, 5, 3, "needs 4"),
    ] {
        let grammar = grammar_driven_by(id, half_width, height, length);
        let refused = TileSet::build(&sections, &grammar)
            .err()
            .unwrap_or_else(|| panic!("a {half_width}x{height}x{length} grid cannot hold '{id}'"));
        assert!(
            refused.contains(id) && refused.contains(wanted),
            "the line has to name the drive and the size it wants, and reads: {refused}"
        );
    }
}

/// The spinal lance the bench used to BOLT ON after the fact is a seeded role
/// now, and it reaches the nose of every hull.
///
/// Seeded for the reason the stern drive is. A gun the whole ship aims fires
/// down its own axis, `erode_blocked_exits` takes off anything whose lane is
/// not clear, and a lane three cells long is only ever clear at an end of the
/// hull. Standing it in the keel column puts its muzzle in the bow-most row,
/// where the lane leaves the grid and nothing can block it - so unlike the
/// bench stamp, nothing is carved.
#[test]
fn a_seeded_bow_gun_stands_on_the_nose_of_every_hull() {
    let sections = GameSections(nova_authoring::generation::build_section_catalog());
    let mut grammar = shipped_grammar();
    grammar.keel.bow_gun = Some("railgun_lance_section".to_string());
    let set = TileSet::build(&sections, &grammar).expect("the grammar builds");

    for seed in 0..8u64 {
        let hull = set
            .hull(seed, true, None)
            .unwrap_or_else(|why| panic!("seed {seed}: {why}"));
        let guns: Vec<&SpaceshipSectionConfig> = hull
            .sections
            .iter()
            .filter(|section| {
                matches!(&section.source, SectionSource::Prototype(p) if p == "railgun_lance_section")
            })
            .collect();
        assert_eq!(
            guns.len(),
            2,
            "the gun stands ON the keel line, so the mirror owes seed {seed} a PAIR"
        );
        let bow = set.bow_face();
        for gun in &guns {
            // The lance is 1x1x3 and fires -z, so its body reaches the bow face
            // exactly. Anything further aft would have hull in front of it.
            assert!(
                (gun.position.z - 1.5 - bow).abs() < GRID_EPSILON,
                "seed {seed}: the lance sits at z {} with the bow face at {bow}",
                gun.position.z
            );
        }
        let errors = hull_errors(&hull, &sections);
        assert!(
            errors.is_empty(),
            "seed {seed} produced content the game would refuse:\n  {}",
            errors.join("\n  ")
        );
        refuse_unmated_contacts(&hull, &sections);
    }
}

/// A zone is a unary constraint on WHERE a part stands, and it holds for every
/// cell of the part rather than for the one it is placed by.
#[test]
fn a_zoned_part_stands_only_in_the_region_it_is_zoned_to() {
    let sections = GameSections(nova_authoring::generation::build_section_catalog());
    for (zone, holds) in [
        (GrammarZone::Dorsal, (|y: f32| y > 0.0) as fn(f32) -> bool),
        (GrammarZone::Ventral, |y: f32| y < 0.0),
    ] {
        let mut grammar = shipped_grammar();
        for part in &mut grammar.parts {
            if part.prototype.starts_with("pdc_") {
                part.zone = Some(zone);
            }
        }
        let set = TileSet::build(&sections, &grammar).expect("the grammar builds");
        let mut seen = 0;
        for seed in 0..8u64 {
            let hull = set
                .hull(seed, true, None)
                .unwrap_or_else(|why| panic!("{zone:?} seed {seed}: {why}"));
            for section in &hull.sections {
                let SectionSource::Prototype(id) = &section.source else {
                    continue;
                };
                if !id.starts_with("pdc_") {
                    continue;
                }
                seen += 1;
                assert!(
                    holds(section.position.y),
                    "{zone:?} seed {seed}: '{id}' stands at y {}",
                    section.position.y
                );
            }
        }
        // A rule nothing was subject to proves nothing.
        assert!(seen > 0, "{zone:?} put no turret on any of eight hulls");
    }
}

/// Both ends of the keel are seeded, and what is left between them has to hold
/// a keel. A grid that cannot is refused by name rather than clamped.
#[test]
fn a_grid_whose_two_seeded_ends_meet_is_refused() {
    let sections = GameSections(nova_authoring::generation::build_section_catalog());
    let mut grammar = shipped_grammar();
    grammar.keel.bow_gun = Some("railgun_lance_section".to_string());
    grammar.grid.length = 4;
    let Err(refusal) = TileSet::build(&sections, &grammar) else {
        panic!("a grid 4 cells long holds no keel between the two seeded ends");
    };
    assert!(
        refusal.contains("leave no keel between them"),
        "the refusal names what is wrong: {refusal}"
    );

    // A spinal gun stands in the keel COLUMN, so it cannot be a block.
    let mut grammar = shipped_grammar();
    grammar.keel.bow_gun = Some("vector_thruster_section".to_string());
    let Err(refusal) = TileSet::build(&sections, &grammar) else {
        panic!("a 3x3x2 block cannot stand in the keel column as a spinal gun");
    };
    assert!(
        refusal.contains("stands on the keel line"),
        "the refusal names what is wrong: {refusal}"
    );
}
