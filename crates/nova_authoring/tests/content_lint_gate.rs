//! The CI half of the content lint: the same tree walk the `content` CLI's
//! `lint` subcommand runs, asserted clean of Error-level issues. Warns are
//! printed but do not fail - they are authoring smells, not broken references.
//! See `nova_scenario::lint` for the check list and `cargo run content lint`
//! for the author CLI.

use nova_scenario::prelude::LintSeverity;

#[test]
fn repo_content_tree_has_no_lint_errors() {
    let issues = nova_authoring::lint_walk::lint_content_tree();
    let mut errors = Vec::new();
    for (bundle, issue) in &issues {
        match issue.severity {
            LintSeverity::Error => errors.push(format!(
                "[{bundle}] scenario '{}': {}",
                issue.scenario, issue.message
            )),
            LintSeverity::Warn => println!(
                "WARN [{bundle}] scenario '{}': {}",
                issue.scenario, issue.message
            ),
        }
    }
    assert!(
        errors.is_empty(),
        "content lint errors (fix the content or the lint):\n{}",
        errors.join("\n")
    );
}

/// `--target` mode scopes the report to exactly one bundle: an in-repo id
/// resolves and reports only its own findings. An EXTERNAL mod directory - the
/// mod-developer case, a tree this repository has never seen - sees the base
/// section catalog (a base prototype passes) while a bad prototype still flags,
/// attributed to the target's dir-name id.
#[test]
fn target_mode_lints_one_mod_in_repo_or_external() {
    // In-repo by id: target mode scopes to exactly that bundle, error-free.
    let dir = nova_authoring::lint_walk::resolve_target("example").expect("example resolves");
    let report = nova_authoring::lint_walk::collect_target(&dir);
    assert_eq!(
        report.error_count(),
        0,
        "the example mod ships error-clean: {:?}",
        report.findings
    );
    assert!(
        report.findings.iter().all(|f| f.bundle == "example")
            && report.acked.iter().all(|a| a.bundle == "example"),
        "target mode scopes findings and acks to the target: {report:?}"
    );

    // External path: a temp mod using a real base prototype AND a bogus one.
    let external = tempfile::tempdir().expect("tempdir");
    let dir = external.path().join("my-mod");
    write_mod(
        &dir,
        r#"[
    Scenario((
        id: "my_scenario",
        name: "My Scenario",
        description: "external lint fixture",
        cubemap: "dep://base/textures/cubemap.png",
        events: [
            (
                name: OnStart,
                actions: [
                    SpawnScenarioObject((
                        base: (id: "ship", name: "Ship", position: (0.0, 0.0, 0.0), rotation: (0.0, 0.0, 0.0, 1.0)),
                        kind: Spaceship((
                            controller: AI(()),
                            hull: Inline((
                                sections: [
                                    (id: "a", position: (0.0, 0.0, 0.0), rotation: (0.0, 0.0, 0.0, 1.0), source: Prototype("basic_controller_section")),
                                    (id: "b", position: (0.0, 0.0, 1.0), rotation: (0.0, 0.0, 0.0, 1.0), source: Prototype("imaginary_hull")),
                                ],
                            )),
                        )),
                    )),
                ],
            ),
        ],
    )),
]"#,
        None,
    );

    let issues = nova_authoring::lint_walk::lint_target(&dir);
    let errors: Vec<_> = issues
        .iter()
        .filter(|(_, i)| i.severity == LintSeverity::Error)
        .collect();
    assert_eq!(
        errors.len(),
        1,
        "only the bogus prototype flags: {issues:?}"
    );
    assert!(errors[0].1.message.contains("imaginary_hull"));
    assert!(
        errors[0].0 == "my-mod",
        "the finding is attributed to the target's dir-name id: {errors:?}"
    );
}

/// A mod DECLARES its own balance acknowledgments beside its manifest, and the
/// linter resolves them from the bundle it is linting - no list in this
/// repository names anybody's content.
///
/// The fixture plants one close-spawn WARN (an armed hostile arriving from a
/// TRIGGERED handler inside its own weapon envelope). Unacked it reports as an
/// open warning; with the mod's own ack it reports as ACKED, carrying the
/// reason and task the mod's author wrote; with an ack that names nothing it
/// reports as a STALE-ack Error the author must prune.
#[test]
fn a_mod_declares_its_own_balance_acks_and_the_linter_reads_them_there() {
    let ack = |hostile: &str| {
        format!(
            r#"[(scenario: "acked_scenario", hostile: "{hostile}", kind: "close-spawn", reason: "Intended: the drama entrance.", task: "20260816-122142")]"#
        )
    };

    // 1. No ack file: the finding is an open warning.
    let plain = tempfile::tempdir().expect("tempdir");
    let dir = plain.path().join("ack-mod");
    write_mod(&dir, ACKED_SCENARIO_RON, None);
    let report = nova_authoring::lint_walk::collect_target(&dir);
    assert_eq!(report.error_count(), 0, "a WARN never gates: {report:?}");
    assert!(
        report.acked.is_empty(),
        "nothing is acked without a declaration: {:?}",
        report.acked
    );
    let warned = report
        .findings
        .iter()
        .find(|f| f.message.contains("close-spawn"))
        .expect("the fixture raises a close-spawn WARN");
    assert!(
        warned.element.contains("hostile_1"),
        "the finding names the hostile: {}",
        warned.element
    );

    // 2. The mod's own ack file: the same finding reports as ACKED, with the
    // author's reason and task, and no longer counts as an open warning.
    let acked = tempfile::tempdir().expect("tempdir");
    let dir = acked.path().join("ack-mod");
    write_mod(&dir, ACKED_SCENARIO_RON, Some(&ack("hostile_1")));
    let report = nova_authoring::lint_walk::collect_target(&dir);
    assert_eq!(report.error_count(), 0, "an ack never gates: {report:?}");
    assert!(
        report
            .findings
            .iter()
            .all(|f| !f.message.contains("close-spawn")),
        "the acked finding left the open list: {:?}",
        report.findings
    );
    let entry = report
        .acked
        .iter()
        .find(|a| a.element.contains("hostile_1"))
        .expect("the finding reports as acked");
    assert_eq!(entry.bundle, "ack-mod", "attributed to the declaring mod");
    assert_eq!(entry.ack_task, "20260816-122142");
    assert!(entry.ack_reason.contains("drama entrance"));

    // 3. An ack naming nothing is STALE, and stale is an Error the mod's
    // author must prune - the exception list cannot rot quietly.
    let stale = tempfile::tempdir().expect("tempdir");
    let dir = stale.path().join("ack-mod");
    write_mod(&dir, ACKED_SCENARIO_RON, Some(&ack("nobody")));
    let report = nova_authoring::lint_walk::collect_target(&dir);
    assert!(
        report
            .findings
            .iter()
            .any(|f| f.message.contains("stale ack")),
        "an unmatched ack surfaces as a stale-ack finding: {:?}",
        report.findings
    );
    assert!(
        report.error_count() > 0,
        "a stale ack gates the build: {report:?}"
    );
}

/// A scenario whose TRIGGERED handler brings an armed hostile in inside its own
/// weapon envelope: the close-spawn WARN the ack test acknowledges. Kept
/// deliberately minimal - a player ship, a beacon to enter, and the arrival.
const ACKED_SCENARIO_RON: &str = r#"[
    Scenario((
        id: "acked_scenario",
        name: "Acked Scenario",
        description: "close-spawn ack fixture",
        cubemap: "dep://base/textures/cubemap.png",
        events: [
            (
                name: OnStart,
                actions: [
                    SpawnScenarioObject((
                        base: (id: "player", name: "Player", position: (0.0, 0.0, 0.0), rotation: (0.0, 0.0, 0.0, 1.0)),
                        kind: Spaceship((
                            controller: Player((infinite_ammo: false)),
                            hull: Inline((
                                sections: [
                                    (id: "controller", position: (0.0, 0.0, 0.0), rotation: (0.0, 0.0, 0.0, 1.0), source: Prototype("basic_controller_section")),
                                    (id: "guns", position: (0.0, 0.75, 0.0), rotation: (0.0, 0.0, 0.0, 1.0), source: Prototype("pdc_kinetic_turret_section")),
                                ],
                            )),
                        )),
                    )),
                    SpawnScenarioObject((
                        base: (id: "beacon", name: "Beacon", position: (0.0, 0.0, -50.0), rotation: (0.0, 0.0, 0.0, 1.0)),
                        kind: Beacon((label: "GO", radius: 3.0, color: Srgba((red: 1.0, green: 1.0, blue: 1.0, alpha: 1.0)), area_radius: Some(20.0))),
                    )),
                ],
            ),
            (
                name: OnEnter,
                filters: [Entity((id: Some("beacon"), other_id: Some("player")))],
                actions: [
                    SpawnScenarioObject((
                        base: (id: "hostile_1", name: "Hostile One", position: (0.0, 0.0, -30.0), rotation: (0.0, 0.0, 0.0, 1.0)),
                        kind: Spaceship((
                            controller: AI(()),
                            hull: Inline((
                                sections: [
                                    (id: "controller", position: (0.0, 0.0, 0.0), rotation: (0.0, 0.0, 0.0, 1.0), source: Prototype("basic_controller_section")),
                                    (id: "turret", position: (0.0, 0.75, 0.0), rotation: (0.0, 0.0, 0.0, 1.0), source: Prototype("pdc_kinetic_turret_section")),
                                ],
                            )),
                        )),
                    )),
                ],
            ),
        ],
    )),
]"#;

/// Write a one-content-file mod at `dir` (its dir name is its id, portal-style),
/// optionally with a declared `balance_acks.ron` beside the manifest.
fn write_mod(dir: &std::path::Path, content: &str, acks: Option<&str>) {
    let id = dir.file_name().expect("named dir").to_string_lossy();
    std::fs::create_dir_all(dir).expect("mod dir");
    std::fs::write(
        dir.join(format!("{id}.bundle.ron")),
        r#"(content: ["mod.content.ron"], meta: (name: "Fixture Mod", version: "0.1.0"))"#,
    )
    .expect("bundle");
    std::fs::write(dir.join("mod.content.ron"), content).expect("content");
    if let Some(acks) = acks {
        std::fs::write(dir.join("balance_acks.ron"), acks).expect("acks");
    }
}
