//! The CI half of the content lint (task 20260716-191543): the same tree
//! walk the `content_lint` bin runs, asserted clean of Error-level issues.
//! Warns are printed but do not fail - they are authoring smells, not
//! broken references. See `nova_scenario::lint` for the check list and
//! `cargo run -p nova_assets --bin content_lint` for the author CLI.

use nova_scenario::prelude::LintSeverity;

#[test]
fn repo_content_tree_has_no_lint_errors() {
    let issues = nova_assets::lint_walk::lint_content_tree();
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

/// `--target` mode (task 20260716-204618): an in-repo id lints exactly that
/// bundle's findings (the-ledger's known cross-handler warn, zero errors),
/// and an EXTERNAL mod directory - the mod-developer case - sees the base
/// section catalog (a base prototype passes) while a bad prototype still
/// flags.
#[test]
fn target_mode_lints_one_mod_in_repo_or_external() {
    // In-repo by id: only ledger findings, no errors.
    let dir = nova_assets::lint_walk::resolve_target("the-ledger").expect("the-ledger resolves");
    let issues = nova_assets::lint_walk::lint_target(&dir);
    assert!(
        issues
            .iter()
            .all(|(bundle, issue)| bundle == "the-ledger" && issue.severity != LintSeverity::Error),
        "{issues:?}"
    );
    assert!(
        issues
            .iter()
            .any(|(_, issue)| issue.message.contains("mutually exclusive")),
        "the known ch4 warn shows in target mode: {issues:?}"
    );

    // External path: a temp mod using a real base prototype AND a bogus one.
    let external = tempfile::tempdir().expect("tempdir");
    let dir = external.path().join("my-mod");
    std::fs::create_dir_all(&dir).expect("mod dir");
    std::fs::write(
        dir.join("my-mod.bundle.ron"),
        r#"(content: ["my.content.ron"], meta: (name: "My Mod", version: "0.1.0"))"#,
    )
    .expect("bundle");
    std::fs::write(
        dir.join("my.content.ron"),
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
                            sections: [
                                (id: "a", position: (0.0, 0.0, 0.0), rotation: (0.0, 0.0, 0.0, 1.0), source: Prototype("basic_controller_section")),
                                (id: "b", position: (0.0, 0.0, 1.0), rotation: (0.0, 0.0, 0.0, 1.0), source: Prototype("imaginary_hull")),
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

    let issues = nova_assets::lint_walk::lint_target(&dir);
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
