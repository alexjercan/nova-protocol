# Prototype-reference lint: unknown section prototype ids must fail the mod gates

- STATUS: OPEN
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

## Notes

- Discovered in: tasks/20260716-123535 close notes.
- The declared-but-not-loaded lesson family; same class as the
  validate-in-every-domain ledger entry.
