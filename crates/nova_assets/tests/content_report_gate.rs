//! The unified content-report gate (task 20260718-152240): the merged
//! `content lint` produces reference/geometry AND balance findings in one
//! report, and every finding names the file + element it is about. A
//! deliberately broken fixture mod plants one of each finding class and this
//! test asserts the report pinpoints each; a second test walks the SHIPPED
//! mods and confirms every located finding points at a file that actually
//! exists in the mod. See `nova_assets::content_report` for the model and
//! `nova_assets::lint_walk::{collect_tree, collect_target}` for the walk.

use nova_assets::{
    content_report::{Category, Severity},
    lint_walk::{collect_target, resolve_target},
};

/// A fixture mod on disk with THREE planted problems, one per finding class:
/// a reference error (a bogus section prototype), a balance error (an armed
/// hostile opening the scenario on top of the player), and an input overlap
/// (a section bound to the flight rig's Space burn). Returns the tempdir (kept
/// alive by the caller) and the single content file's name.
fn broken_mod() -> (tempfile::TempDir, &'static str) {
    let dir = tempfile::tempdir().expect("tempdir");
    let mod_dir = dir.path().join("broken-mod");
    std::fs::create_dir_all(&mod_dir).expect("mod dir");
    std::fs::write(
        mod_dir.join("broken-mod.bundle.ron"),
        r#"(content: ["broken.content.ron"], meta: (name: "Broken Mod", version: "0.1.0"))"#,
    )
    .expect("bundle");
    // The player ship carries the reference error (a bogus prototype) and the
    // input overlap (guns on Space); the OnStart hostile is armed and spawns
    // right on top of the player - the spawned-dead balance error.
    std::fs::write(
        mod_dir.join("broken.content.ron"),
        r#"[
    Scenario((
        id: "broken_scene",
        name: "Broken Scene",
        description: "planted findings",
        cubemap: "dep://base/textures/cubemap.png",
        events: [
            (
                name: OnStart,
                actions: [
                    SpawnScenarioObject((
                        base: (id: "player", name: "Player", position: (0.0, 0.0, 0.0), rotation: (0.0, 0.0, 0.0, 1.0)),
                        kind: Spaceship((
                            controller: Player((
                                input_mapping: {
                                    "guns": [ Keyboard(Space) ],
                                },
                                infinite_ammo: false,
                            )),
                            sections: [
                                (id: "controller", position: (0.0, 0.0, 0.0), rotation: (0.0, 0.0, 0.0, 1.0), source: Prototype("basic_controller_section")),
                                (id: "guns", position: (0.0, 0.0, -1.0), rotation: (0.0, 0.0, 0.0, 1.0), source: Prototype("better_turret_section")),
                                (id: "bad", position: (0.0, 0.0, 1.0), rotation: (0.0, 0.0, 0.0, 1.0), source: Prototype("imaginary_hull")),
                            ],
                        )),
                    )),
                    SpawnScenarioObject((
                        base: (id: "ambush", name: "Ambush", position: (0.0, 0.0, 3.0), rotation: (0.0, 0.0, 0.0, 1.0)),
                        kind: Spaceship((
                            controller: AI(()),
                            sections: [
                                (id: "controller", position: (0.0, 0.0, 0.0), rotation: (0.0, 0.0, 0.0, 1.0), source: Prototype("basic_controller_section")),
                                (id: "turret", position: (0.0, 0.0, -1.0), rotation: (0.0, 0.0, 0.0, 1.0), source: Prototype("better_turret_section")),
                            ],
                        )),
                    )),
                ],
            ),
        ],
    )),
]"#,
    )
    .expect("content");
    (dir, "broken.content.ron")
}

#[test]
fn report_pinpoints_reference_balance_and_input_findings() {
    let (dir, file) = broken_mod();
    let report = collect_target(&dir.path().join("broken-mod"));

    // Reference error: the bogus prototype, located in the content file.
    let reference = report
        .findings
        .iter()
        .find(|f| f.category == Category::Reference && f.message.contains("imaginary_hull"))
        .expect("the bogus prototype is a reference finding");
    assert_eq!(reference.severity, Severity::Error);
    assert_eq!(reference.file.as_deref(), Some(file));
    assert_eq!(reference.bundle, "broken-mod");

    // Balance error: the armed OnStart hostile, spawned dead, located and
    // naming the offending hostile in its element.
    let balance = report
        .findings
        .iter()
        .find(|f| f.category == Category::Balance && f.message.contains("spawned-dead"))
        .expect("the on-top hostile is a spawned-dead balance finding");
    assert_eq!(balance.severity, Severity::Error);
    assert_eq!(balance.file.as_deref(), Some(file));
    assert!(
        balance.element.contains("ambush"),
        "the balance finding names the hostile: {}",
        balance.element
    );

    // Input overlap: the Space binding, located, naming the section.
    let overlap = report
        .findings
        .iter()
        .find(|f| f.category == Category::InputOverlap)
        .expect("guns-on-Space is an input-overlap finding");
    assert_eq!(overlap.severity, Severity::Warn);
    assert_eq!(overlap.file.as_deref(), Some(file));
    assert!(
        overlap.element.contains("guns") && overlap.message.contains("Space"),
        "the overlap names the section and the key: {} / {}",
        overlap.element,
        overlap.message
    );

    // Both error classes gate the CLI; the warning does not.
    assert!(
        report.error_count() >= 2,
        "reference + balance errors both count: {} errors",
        report.error_count()
    );

    // The Markdown document carries all three located findings, not an empty
    // file - the headline deliverable.
    let md = report.to_markdown();
    assert!(md.contains("broken.content.ron"));
    assert!(md.contains("imaginary_hull"));
    assert!(md.contains("spawned-dead"));
    assert!(md.contains("Space"));
    assert!(md.contains("fix:"), "every class carries a suggested fix");
}

/// Provenance holds on real content: for each shipped mod, every finding that
/// claims a file names a content file that the mod actually ships. Proves the
/// report on the-ledger / gauntlet / example points where it says (DoD step).
#[test]
fn shipped_mod_reports_locate_findings_in_real_files() {
    for id in ["the-ledger", "gauntlet", "example"] {
        let dir = resolve_target(id).unwrap_or_else(|| panic!("{id} resolves"));
        let report = collect_target(&dir);
        assert_eq!(report.target.as_deref(), Some(id));
        assert!(
            report.bundles == vec![id.to_string()],
            "the report covers exactly {id}: {:?}",
            report.bundles
        );
        for finding in &report.findings {
            assert!(
                !finding.element.is_empty(),
                "every finding names an element"
            );
            if let Some(file) = &finding.file {
                assert!(
                    dir.join(file).is_file(),
                    "{id} finding points at a real file: {file} ({})",
                    finding.message
                );
            }
        }
        // A clean report is still a document (DoD: not an empty file).
        assert!(report.to_markdown().lines().count() > 3);
    }
}
