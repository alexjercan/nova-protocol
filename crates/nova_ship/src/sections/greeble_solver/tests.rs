use std::collections::{BTreeMap, BTreeSet};

use super::*;
use crate::sections::{
    shell_skin::{derive_skin, SkinPlate, SkinStructure},
    skin_surface::skin_surface,
};

fn link(id: &str, connects: &[&str]) -> GreebleLinkConfig {
    GreebleLinkConfig {
        id: id.into(),
        connects: connects.iter().map(|id| (*id).into()).collect(),
    }
}

fn greeble(
    id: &str,
    fixture: Option<&str>,
    weight: f32,
    rotate: GreebleRotateType,
    [up, right, down, left]: [&str; 4],
) -> GreebleConfig {
    GreebleConfig {
        id: id.into(),
        fixture: fixture.map(str::to_string),
        weight,
        rotate,
        link_points: GreebleLinkPoints {
            up: up.into(),
            right: right.into(),
            down: down.into(),
            left: left.into(),
        },
        near: Vec::new(),
        max_count: None,
    }
}

fn near(greebles: &[&str], radius: u32, factor: f32) -> GreebleNearConfig {
    GreebleNearConfig {
        greebles: greebles.iter().map(|id| (*id).into()).collect(),
        radius,
        factor,
    }
}

/// The rule set of the accepted language note: empty space, a line with an
/// end piece, and a spaced greeble that bans itself within 3 cells and caps at
/// 3.
fn sample_rules() -> GreebleRules {
    use GreebleRotateType::All;
    let mut crane = greeble(
        "crane",
        Some("industrial_crane"),
        0.6,
        All,
        ["spaced", "spaced", "spaced", "spaced"],
    );
    crane.near = vec![near(&["crane"], 3, 0.0)];
    crane.max_count = Some(3);
    GreebleRules {
        link_types: vec![
            link("any", &["any", "empty"]),
            link("empty", &["any", "empty", "spaced"]),
            link("spaced", &["empty"]),
            link("line", &["line"]),
        ],
        greebles: vec![
            greeble("empty", None, 6.0, GreebleRotateType::None, ["empty"; 4]),
            greeble(
                "hazard_line",
                Some("industrial_hazard_band"),
                8.0,
                All,
                ["line", "any", "line", "any"],
            ),
            greeble(
                "hazard_line_end",
                Some("industrial_hazard_band_end"),
                1.0,
                All,
                ["line", "any", "any", "any"],
            ),
            crane,
        ],
    }
}

fn compiled(rules: &GreebleRules) -> CompiledGreebleRules {
    rules
        .compile()
        .unwrap_or_else(|errors| panic!("the rules compile: {errors:?}"))
}

/// A solid block of cells `x` by `y` by `z`, every face offering a socket.
fn block(x: i32, y: i32, z: i32) -> SkinStructure {
    let mut structure = SkinStructure::default();
    for cell in (0..x).flat_map(|a| (0..y).flat_map(move |b| (0..z).map(move |c| (a, b, c)))) {
        structure.insert(IVec3::new(cell.0, cell.1, cell.2), [true; 6]);
    }
    structure
}

fn surface_of(structure: &SkinStructure) -> SkinSurface {
    skin_surface(structure, &derive_skin(structure))
}

fn index(rules: &CompiledGreebleRules, id: &str) -> usize {
    (0..rules.greeble_count())
        .find(|index| rules.greeble_id(*index).as_str() == id)
        .unwrap_or_else(|| panic!("greeble '{id}' is compiled"))
}

/// The link type `placement` shows on `slot`.
fn shown(rules: &CompiledGreebleRules, placement: GreeblePlacement, slot: usize) -> usize {
    rules
        .variants
        .iter()
        .find(|variant| variant.greeble == placement.greeble && variant.turns == placement.turns)
        .expect("a placement names a compiled variant")
        .sides[slot]
}

fn placed(solution: &GreebleSolution, cell: usize) -> GreeblePlacement {
    solution.cells[cell].expect("a strict solve places every cell")
}

/// Every linked pair of slots shows link types that connect.
fn assert_links_connect(
    surface: &SkinSurface,
    rules: &CompiledGreebleRules,
    solution: &GreebleSolution,
) {
    for cell in 0..surface.len() {
        for slot in 0..SURFACE_SLOTS {
            let Some(link) = surface.link(cell, slot) else {
                continue;
            };
            let here = shown(rules, placed(solution, cell), slot);
            let there = shown(rules, placed(solution, link.cell), usize::from(link.slot));
            assert!(
                rules.connects[here] & (1 << there) != 0,
                "cell {cell} slot {slot} shows link {here}, facing link {there}",
            );
        }
    }
}

fn distance(surface: &SkinSurface, from: usize, to: usize) -> Option<u32> {
    ball(surface, from, surface.len() as u32)
        .into_iter()
        .find(|(cell, _)| *cell == to)
        .map(|(_, distance)| distance)
}

#[test]
fn the_same_surface_rules_and_seed_give_the_same_placements() {
    let structure = block(6, 2, 3);
    let surface = surface_of(&structure);
    let rules = compiled(&sample_rules());
    let seed = greeble_seed("industrial");
    assert_eq!(seed, greeble_seed("industrial"));

    for offset in 0..4 {
        let first = solve_greebles(&surface, &rules, seed + offset, |_, _| true);
        let second = solve_greebles(&surface, &rules, seed + offset, |_, _| true);
        assert_eq!(first, second);
        assert!(!first.fallback);
    }
}

/// A plate's identity across an edit: its cell and the face it shows.
type PlateKey = ([i32; 3], [i32; 3]);

fn plate_key(plate: &SkinPlate) -> PlateKey {
    (
        plate.cell.to_array(),
        (plate.cell - plate.anchor).to_array(),
    )
}

/// Per plate key, the plate keys and back slots across its four slots.
fn link_keys(
    plates: &[SkinPlate],
    surface: &SkinSurface,
) -> BTreeMap<PlateKey, Vec<Option<(PlateKey, u8)>>> {
    (0..surface.len())
        .map(|cell| {
            let links = surface.cells()[cell]
                .links
                .iter()
                .map(|link| link.map(|link| (plate_key(&plates[link.cell]), link.slot)))
                .collect();
            (plate_key(&plates[cell]), links)
        })
        .collect()
}

/// Of the plates after an edit that lie more than `radius` links from every
/// plate the edit touched, how many there are and how many keep their greeble
/// and turn. A plate is touched when it is new or its links changed.
fn kept_outside(
    before: &SkinStructure,
    after: &SkinStructure,
    rules: &CompiledGreebleRules,
    seed: u64,
    radius: u32,
) -> (usize, usize) {
    let (old_plates, new_plates) = (derive_skin(before), derive_skin(after));
    let (old_surface, new_surface) = (
        skin_surface(before, &old_plates),
        skin_surface(after, &new_plates),
    );
    let (old_links, new_links) = (
        link_keys(&old_plates, &old_surface),
        link_keys(&new_plates, &new_surface),
    );
    let old = solve_greebles(&old_surface, rules, seed, |_, _| true);
    let new = solve_greebles(&new_surface, rules, seed, |_, _| true);
    let old_at: BTreeMap<PlateKey, usize> = old_plates
        .iter()
        .enumerate()
        .map(|(index, plate)| (plate_key(plate), index))
        .collect();

    let mut near = BTreeSet::new();
    for (cell, plate) in new_plates.iter().enumerate() {
        let key = plate_key(plate);
        if old_links.get(&key) != new_links.get(&key) {
            near.extend(
                ball(&new_surface, cell, radius)
                    .into_iter()
                    .map(|(at, _)| at),
            );
        }
    }
    let outside: Vec<usize> = (0..new_plates.len())
        .filter(|cell| !near.contains(cell))
        .collect();
    let kept = outside
        .iter()
        .filter(|cell| {
            let was = old_at[&plate_key(&new_plates[**cell])];
            old.cells[was] == new.cells[**cell]
        })
        .count();
    (outside.len(), kept)
}

/// Removing one section keeps the greeble and turn of at least 95% of the
/// plates more than 2 links from every plate the edit touched.
///
/// A share, not every plate: the solve guarantees no radius. The pick order
/// is global, a forced line run carries a change along it, and a `max_count`
/// cap or a factor-0 ban reaches past radius 2. Measured 2026-10-14 on this
/// fixture: 5226 of 5392 kept (96.9%) at radius 2, and 96.7% to 96.9% at
/// radius 0 to 3, so the loss does not shrink with distance.
#[test]
fn removing_a_section_keeps_most_placements_away_from_the_edit() {
    let before = block(8, 2, 4);
    let rules = compiled(&sample_rules());
    let removals = [
        IVec3::new(0, 0, 0),
        IVec3::new(4, 1, 2),
        IVec3::new(3, 0, 0),
        IVec3::new(7, 1, 3),
    ];
    let (mut outside, mut kept) = (0, 0);
    for removed in removals {
        let mut after = SkinStructure::default();
        for cell in before.filled_cells().filter(|cell| *cell != removed) {
            after.insert(cell, [true; 6]);
        }
        for style in 0..16 {
            let seed = greeble_seed(&format!("style_{style}"));
            let (count, same) = kept_outside(&before, &after, &rules, seed, 2);
            outside += count;
            kept += same;
        }
    }
    assert!(outside > 0, "the edits leave plates outside radius 2");
    assert!(
        kept * 100 >= outside * 95,
        "{kept} of {outside} plates outside radius 2 keep their greeble",
    );
}

#[test]
fn a_line_greeble_runs_to_a_border_or_ends_in_a_line_end_greeble() {
    let structure = block(8, 1, 2);
    let surface = surface_of(&structure);
    let rules = compiled(&sample_rules());
    let line = index(&rules, "hazard_line");
    let end = index(&rules, "hazard_line_end");
    let line_link = rules
        .variants
        .iter()
        .find(|v| v.greeble == line)
        .unwrap()
        .sides[0];

    let mut lines = 0;
    for seed in 0..32 {
        let solution = solve_greebles(&surface, &rules, seed, |_, _| true);
        assert_links_connect(&surface, &rules, &solution);
        for start in 0..surface.len() {
            let placement = placed(&solution, start);
            if placement.greeble != line {
                continue;
            }
            lines += 1;
            for first in
                (0..SURFACE_SLOTS).filter(|slot| shown(&rules, placement, *slot) == line_link)
            {
                // Walk straight along the run: in a line piece, leave by the
                // slot opposite the one entered by.
                let (mut at, mut out) = (start, first);
                for _ in 0..surface.len() {
                    let Some(link) = surface.link(at, out) else {
                        break;
                    };
                    let next = placed(&solution, link.cell);
                    assert!(
                        next.greeble == line || next.greeble == end,
                        "seed {seed}: a line at cell {at} runs into greeble {}",
                        next.greeble,
                    );
                    assert_eq!(shown(&rules, next, usize::from(link.slot)), line_link);
                    if next.greeble == end || link.cell == start {
                        break;
                    }
                    (at, out) = (link.cell, (usize::from(link.slot) + 2) % SURFACE_SLOTS);
                }
            }
        }
    }
    assert!(lines > 0, "the rules place lines on a long block");
}

#[test]
fn a_greeble_whose_link_points_connect_only_to_empty_has_only_empty_neighbours() {
    let mut rules = sample_rules();
    let crane = rules
        .greebles
        .iter_mut()
        .find(|greeble| greeble.id.as_str() == "crane")
        .unwrap();
    crane.weight = 8.0;
    crane.near.clear();
    crane.max_count = None;
    let rules = compiled(&rules);
    let structure = block(6, 2, 4);
    let surface = surface_of(&structure);
    let (crane, empty) = (index(&rules, "crane"), index(&rules, "empty"));

    let mut cranes = 0;
    for seed in 0..32 {
        let solution = solve_greebles(&surface, &rules, seed, |_, _| true);
        for cell in (0..surface.len()).filter(|cell| placed(&solution, *cell).greeble == crane) {
            cranes += 1;
            for link in surface.cells()[cell].links.iter().flatten() {
                assert_eq!(
                    placed(&solution, link.cell).greeble,
                    empty,
                    "seed {seed}: a spaced greeble at cell {cell} has a fixture beside it",
                );
            }
        }
    }
    assert!(cranes > 0, "the spaced greeble is placed");
}

#[test]
fn a_factor_zero_near_term_is_never_violated() {
    use GreebleRotateType::All;
    let mut vent = greeble("vent", Some("vent"), 6.0, All, ["any"; 4]);
    vent.near = vec![near(&["vent"], 3, 0.0)];
    let rules = compiled(&GreebleRules {
        link_types: vec![
            link("any", &["any", "empty"]),
            link("empty", &["any", "empty"]),
        ],
        greebles: vec![
            greeble("empty", None, 1.0, GreebleRotateType::None, ["empty"; 4]),
            vent,
        ],
    });
    let structure = block(6, 2, 4);
    let surface = surface_of(&structure);
    let vent = index(&rules, "vent");

    let mut most = 0;
    for seed in 0..32 {
        let solution = solve_greebles(&surface, &rules, seed, |_, _| true);
        let vents: Vec<usize> = (0..surface.len())
            .filter(|cell| placed(&solution, *cell).greeble == vent)
            .collect();
        most = most.max(vents.len());
        for (position, a) in vents.iter().enumerate() {
            for b in &vents[position + 1..] {
                let apart = distance(&surface, *a, *b);
                assert!(
                    apart.is_none_or(|apart| apart > 3),
                    "seed {seed}: vents at {a} and {b} are {apart:?} apart",
                );
            }
        }
    }
    assert!(most >= 2, "the ban leaves room for several vents");
}

#[test]
fn max_count_is_never_exceeded() {
    use GreebleRotateType::All;
    let mut vent = greeble("vent", Some("vent"), 20.0, All, ["any"; 4]);
    vent.max_count = Some(2);
    let rules = compiled(&GreebleRules {
        link_types: vec![
            link("any", &["any", "empty"]),
            link("empty", &["any", "empty"]),
        ],
        greebles: vec![
            greeble("empty", None, 1.0, GreebleRotateType::None, ["empty"; 4]),
            vent,
        ],
    });
    let structure = block(4, 2, 3);
    let surface = surface_of(&structure);
    let vent = index(&rules, "vent");

    let mut reached = false;
    for seed in 0..16 {
        let solution = solve_greebles(&surface, &rules, seed, |_, _| true);
        let count = solution
            .cells
            .iter()
            .flatten()
            .filter(|placement| placement.greeble == vent)
            .count();
        assert!(count <= 2, "seed {seed}: {count} vents");
        reached |= count == 2;
    }
    assert!(reached, "a heavy greeble reaches its cap");
}

#[test]
fn compile_refuses_asymmetric_connects_and_unknown_ids() {
    let mut ghost = greeble(
        "ghost",
        Some("ghost"),
        1.0,
        GreebleRotateType::None,
        ["a", "a", "a", "missing"],
    );
    ghost.near = vec![near(&["nobody"], 1, 0.5)];
    let rules = GreebleRules {
        link_types: vec![link("a", &["a", "b"]), link("b", &["b", "unknown"])],
        greebles: vec![
            greeble("empty", None, 1.0, GreebleRotateType::None, ["b"; 4]),
            ghost,
        ],
    };

    let errors = rules.compile().expect_err("the rules are invalid");
    for expected in [
        GreebleRulesError::AsymmetricConnects {
            link: "a".into(),
            other: "b".into(),
        },
        GreebleRulesError::UnknownConnect {
            link: "b".into(),
            target: "unknown".into(),
        },
        GreebleRulesError::UnknownLinkPoint {
            greeble: "ghost".into(),
            link: "missing".into(),
        },
        GreebleRulesError::UnknownNearGreeble {
            greeble: "ghost".into(),
            target: "nobody".into(),
        },
    ] {
        assert!(
            errors.contains(&expected),
            "{expected:?} is missing from {errors:?}"
        );
    }
}

#[test]
fn compile_refuses_a_factor_zero_term_on_a_greeble_a_link_type_can_force() {
    let mut rules = sample_rules();
    rules.greebles[1].near = vec![near(&["hazard_line"], 2, 0.0)];

    let errors = rules.compile().expect_err("a line can be forced");
    assert!(errors.contains(&GreebleRulesError::ForcedBan {
        greeble: "hazard_line".into(),
        target: "hazard_line".into(),
    }));
    compiled(&sample_rules());
}

#[cfg(feature = "serde")]
#[test]
fn the_accepted_rule_language_reads_from_ron() {
    let text = r#"(
        link_types: [
            (id: "any",    connects: ["any", "empty"]),
            (id: "empty",  connects: ["any", "empty", "spaced"]),
            (id: "spaced", connects: ["empty"]),
            (id: "line",   connects: ["line"]),
        ],
        greebles: [
            (id: "empty", fixture: None, weight: 6.0, rotate: None,
             link_points: (up: "empty", right: "empty", down: "empty", left: "empty")),
            (id: "hazard_line", fixture: Some("industrial_hazard_band"), weight: 8.0, rotate: All,
             link_points: (up: "line", right: "any", down: "line", left: "any")),
            (id: "hazard_line_end", fixture: Some("industrial_hazard_band_end"), weight: 1.0, rotate: All,
             link_points: (up: "line", right: "any", down: "any", left: "any")),
            (id: "crane", fixture: Some("industrial_crane"), weight: 0.6, rotate: All,
             link_points: (up: "spaced", right: "spaced", down: "spaced", left: "spaced"),
             near: [(greebles: ["crane"], radius: 3, factor: 0.0)],
             max_count: Some(3)),
        ],
    )"#;
    let rules: GreebleRules = ron::from_str(text).expect("the accepted language parses");
    assert_eq!(rules, sample_rules());
}
