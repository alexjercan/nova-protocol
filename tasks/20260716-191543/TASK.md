# Prototype-reference lint: unknown section prototype ids must fail the mod gates

- STATUS: CLOSED
- PRIORITY: 55
- TAGS: v0.7.0, modding, testing

## Goal

`SectionSource::Prototype("<id>")` resolves at SPAWN, so a scenario
referencing a prototype that does not exist loads green through every
gate (webmods_validation, the portal generator) and ships ships that
half-spawn. Found while authoring The Ledger (task 20260716-123535):
`Prototype("basic_hull_section")` - which does not exist - passed
validation; only a manual catalog cross-check caught it. Add a lint that
walks loaded content's prototype references against the section registry
(base + the mod's own sections) in webmods_validation (and/or the portal
generator if it can know the shipped catalog's sections), failing loud
with the unknown id.

## Direction (spike 20260716-193858, 2026-07-16)

Widened by user direction into the static half of a two-tier design:
a pure lint core in nova_scenario (`lint_scenario -> Vec<LintIssue>`,
Error|Warn) consumed by (1) a `content_lint` bin in nova_assets walking
assets/base + assets/mods + webmods/ (author CLI, gen_content shape),
(2) a CI test gate failing on Error issues, and later (3) the runtime
merge gate (task 20260716-193949). v1 checks - Error: unknown section
prototypes, dangling NextScenario targets (bundle+base-visible set),
duplicate spawned object ids, filter/action target ids nothing in the
scenario can spawn (ScatterObjects prefixes count as satisfied); Warn:
expression variables never set in the scenario, ObjectiveComplete
without a matching Objective. Full reasoning and rejected placements in
the spike.

Direction-level; /plan breaks it into steps when picked up.

## Steps

- [x] `crates/nova_scenario/src/lint.rs`: `LintSeverity` (Error|Warn),
      `LintIssue`, and pure `lint_scenario(&ScenarioConfig,
      known_sections: &HashSet<String>, known_scenarios:
      &HashSet<String>) -> Vec<LintIssue>` implementing the v1 checks
      (unknown prototypes; dangling NextScenario targets; duplicate
      spawned ids; filter/marker/despawn target ids nothing can spawn,
      ScatterObjects prefixes satisfying; Warn: unset expression
      variables via AST walk, ObjectiveComplete without Objective).
      Export in prelude.
- [x] Unit tests in lint.rs: one failing synthetic scenario per check
      (would-it-fail), plus a clean scenario yielding zero issues.
- [x] `nova_assets::lint_walk` (doc(hidden) module, gen_content shape):
      walk assets/base + assets/mods/* + webmods/* bundle manifests,
      parse content RON, build known sets (base sections + each
      bundle's own; scenario ids across all walked bundles), lint every
      scenario, return issues.
- [x] `crates/nova_assets/src/bin/content_lint.rs`: run the walk, print
      issues human-readably, exit non-zero on Error.
- [x] `crates/nova_assets/tests/content_lint_gate.rs`: the CI gate -
      same walk, assert zero Error issues (print Warns).
- [x] Docs: authoring guide gains a "lint your content" note naming the
      bin; CHANGELOG Unreleased.
- [x] Verify: check --all-targets, fmt, new tests green, bin run over
      the real tree (expect clean after the-ledger fixes).

## Notes

- Discovered in: tasks/20260716-123535 close notes.
- The declared-but-not-loaded lesson family; same class as the
  validate-in-every-domain ledger entry.

## Close notes (2026-07-16)

What changed: nova_scenario::lint (pure core, ~350 lines incl. 7 unit
tests - one failing case per check plus a clean baseline);
nova_assets::lint_walk (parses every bundle in assets/base,
assets/mods/*, webmods/* straight from disk, known-sections = base +
own + declared dependencies', known-scenarios = all walked bundles);
the content_lint bin (author CLI, non-zero exit on Error) and the
content_lint_gate CI test (fails on Error, prints Warns). Authoring
guide + CHANGELOG updated.

Design refinement found by running on the REAL tree: the duplicate-id
check initially flagged The Ledger ch4's choice fork (the same boss id
spawned by two mutually exclusive handlers) - a correct pattern. Split
the check: same-handler duplicates are Error, cross-handler duplicates
are Warn ('fine only if the handlers are mutually exclusive'). The real
tree now lints clean with exactly that one accurate warning.

Evidence: gate A/B - re-introducing the original near-miss class
(unknown thruster prototype in ch1) failed the gate naming ship,
section and prototype; restored, green. Unit tests 7/7; check
--all-targets + fmt clean. Full suite is CI's job per the standing
instruction.

Reflection: linting the real tree DURING development (not just
synthetic tests) is what surfaced the over-strict duplicate check;
lint rules need a real-corpus pass before they gate anything.
