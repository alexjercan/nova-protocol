//! Two display-free source gates over the example catalog, kept where the
//! catalog parser lives (`nova_probe::load_example_catalog`) now that
//! `tests/examples_smoke.rs` is gone and probe is the only verdict on a run.
//!
//! Neither spawns anything: they read `Cargo.toml` and `examples/` off disk, so
//! they run everywhere, including a bare `cargo test` on a headless box.

use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
};

/// The repo root: this crate is `<root>/crates/nova_probe`.
fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("the repo root must resolve from the crate manifest dir")
}

/// Disk and the `Cargo.toml` `[[example]]` catalog must agree exactly.
///
/// One direction is load-bearing and the rest is belt: with
/// `autoexamples = false`, an example file that has NO catalog block does not
/// build at all, and nothing else in the toolchain says so - it is silently
/// dead code (task 20260719-193728). The other direction (a block with no file)
/// already fails the build, and the `examples/<category>/<file>` path shape is
/// pinned by `catalog::tests::refuses_an_uncategorized_path`; both are asserted
/// here anyway because the equality that catches the real case catches them
/// for free.
#[test]
fn catalog_matches_disk() {
    let root = repo_root();

    // Example roots on disk: every .rs file DIRECTLY under a category dir.
    // Deeper files (e.g. sections/turret_section/slider.rs) are modules of
    // their sibling root, and data/ holds no code.
    let mut on_disk = BTreeSet::new();
    for category in std::fs::read_dir(root.join("examples")).unwrap() {
        let category = category.unwrap().path();
        if !category.is_dir() {
            panic!(
                "stray file directly under examples/ (examples live in \
                 category dirs): {}",
                category.display()
            );
        }
        for entry in std::fs::read_dir(&category).unwrap() {
            let path = entry.unwrap().path();
            if path.extension().is_some_and(|e| e == "rs") {
                let name = path.file_stem().unwrap().to_str().unwrap().to_string();
                let rel = path
                    .strip_prefix(&root)
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .to_string();
                on_disk.insert((name, rel));
            }
        }
    }

    // The catalog, via THE parser probe's multi-run specs resolve against -
    // one parser, two consumers, no drift between them. It refuses a manifest
    // without `autoexamples = false` itself, so this unwrap also pins that
    // discovery stays off.
    let catalog = nova_probe::load_example_catalog(&root)
        .expect("the [[example]] catalog must parse (and autoexamples must stay off)");
    let cataloged: BTreeSet<(String, String)> = catalog
        .iter()
        .map(|example| (example.name.clone(), example.path.clone()))
        .collect();
    assert_eq!(
        cataloged, on_disk,
        "Cargo.toml [[example]] catalog and examples/ disagree - every \
         example file needs exactly one catalog block (and vice versa)"
    );
}

/// The `sections/` invariant ROSTER: every invariant each range asserts, named.
///
/// Each entry is `(example, &[invariant slug, ...])`, and each slug must appear
/// in that example's source as a `nova_probe::probe_marker` literal
/// `"outcome: <slug>"` beside the `assert!` it belongs to.
///
/// Three slugs are ROUND-COMPLETION invariants - "the whole set held again on a
/// reloaded rig / in a second scene" - and have no assert of their own to sit
/// beside, because the fact they claim is that the round's OTHER asserts all
/// passed. Each rides the last assertion of its round (guarded on the round
/// label) rather than a step of its own, so it is still emitted only from a
/// line that a failing invariant would never reach:
///
/// - `damage invariants hold after reload` (hull_section)
/// - `turret invariants hold after reload` (turret_section)
/// - `launch chain holds in the crossing scene` (torpedo_section)
///
/// What this test bounds is that every invariant is NAMED. That the invariants
/// HOLD is what the runs themselves prove, by panicking.
const SECTION_ROSTER: &[(&str, &[&str])] = &[
    (
        "controller_section",
        &[
            "attitude command swept",
            "attitude tracks",
            "attitude reconverges after reload",
            "attitude tracks on rig b",
        ],
    ),
    (
        "thruster_section",
        &[
            "burn accelerates",
            "plume material exists",
            "plume follows throttle",
            "partial throttle is proportional",
            "plume returns to idle",
        ],
    ),
    (
        "hull_section",
        &[
            "partial hit exact",
            "section destroyed",
            "root and controller survive",
            "com follows surviving sections",
            "com moved aft",
            "root interpolates",
            "camera anchor tracks com",
            "damage invariants hold after reload",
        ],
    ),
    (
        "turret_section",
        &[
            "turret fired",
            "gate damaged",
            "turret tracks the mover",
            "turret invariants hold after reload",
        ],
    ),
    (
        "torpedo_section",
        &[
            "torpedo fired",
            "torpedo armed",
            "torpedo detonated",
            "gate damaged",
            "launch chain holds in the crossing scene",
            "torpedo leads the crosser",
        ],
    ),
];

/// How many invariants the five `sections/` ranges assert between them.
const SECTION_INVARIANTS: usize = 27;

/// Every `sections/` range names EXACTLY the invariants on its roster.
///
/// The stopping rule of task 20260804-093950 made executable: "deepen" is
/// bounded by a named invariant list per run, so the list has to live somewhere
/// a deletion fails. Dropping an invariant leaves the run green - probe's
/// `invariants_held` counts VIOLATIONS, and a range that asserts less simply
/// violates nothing - and this test is what turns that into a red one. The runs
/// themselves are what prove the invariants HOLD; this only pins WHICH.
///
/// Matched both ways on purpose: a roster slug with no marker is an invariant
/// that was removed, and a marker with no roster slug is one added without
/// saying so. The example set is read from the catalog rather than a second
/// hand-kept list, so a new `sections/` range needs a roster to pass.
#[test]
fn sections_assert_their_invariant_roster() {
    const PREFIX: &str = "\"outcome: ";

    let root = repo_root();
    let listed: usize = SECTION_ROSTER.iter().map(|(_, names)| names.len()).sum();
    assert_eq!(
        listed, SECTION_INVARIANTS,
        "the sections/ roster lists {listed} invariants, not {SECTION_INVARIANTS}"
    );

    let catalog = nova_probe::load_example_catalog(&root).expect("the catalog must parse");
    let sections: BTreeSet<&str> = catalog
        .iter()
        .filter(|example| example.category == "sections")
        .map(|example| example.name.as_str())
        .collect();
    assert_eq!(
        SECTION_ROSTER
            .iter()
            .map(|(example, _)| *example)
            .collect::<BTreeSet<_>>(),
        sections,
        "every sections/ example needs a roster, and only those"
    );

    for (example, names) in SECTION_ROSTER {
        let path = root.join("examples/sections").join(format!("{example}.rs"));
        let source = std::fs::read_to_string(&path).unwrap();
        let marked: BTreeSet<&str> = source
            .match_indices(PREFIX)
            .filter_map(|(at, _)| {
                let rest = &source[at + PREFIX.len()..];
                rest.find('"').map(|end| &rest[..end])
            })
            .collect();
        assert_eq!(
            marked,
            names.iter().copied().collect::<BTreeSet<_>>(),
            "{example} does not emit exactly its roster of invariant markers; \
             each invariant carries one `nova_probe::probe_marker` named \
             `outcome: <slug>` beside its assert"
        );
    }
}
