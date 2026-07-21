# Resolve asteroid_field hidden-vs-wiki contradiction (picker sandbox or hidden?)

- STATUS: OPEN
- PRIORITY: 56
- TAGS: v0.8.0,scenario,docs

## Story

The spike (tasks/20260721-155249/SPIKE.md) found the shipped
asteroid_field.content.ron carries `hidden: true` while the player wiki
scenarios.md still describes it as a picker-visible sandbox. One of them is
wrong: either the hiding was an unintended regression, or the wiki is stale.
Small verify-first task so the docs edits in the ch3 tasks build on a
correct scenarios.md.

## Steps

- [ ] Verify-first: `git log -p --follow -S "hidden" -- crates/nova_assets/src/scenario.rs`
      (and the generated RON) to find when and why asteroid_field got
      `hidden: true`; record the evidence (commit + reason) in this file.
- [ ] Decide from the evidence: intentional -> fix web/src/wiki/scenarios.md
      (and any other surface listing the picker) to match; regression ->
      unhide in the asteroid_field builder and `content gen`.
- [ ] Sweep the doc surfaces for the picker list (web/src/wiki/scenarios.md,
      tutorial page, README) so every surface agrees with the shipped flag.
- [ ] Run `cargo run -p nova_assets --bin content -- lint`; parity tests if
      the RON changed.
- [ ] CHANGELOG entry only if player-visible behavior changed (unhide).

## Definition of Done

- The shipped flag and every doc surface agree
  (cmd: `grep -n "hidden" assets/base/scenarios/asteroid_field.content.ron`;
  cmd: `grep -n "Asteroid Field" web/src/wiki/scenarios.md`).
- The decision and its git-history evidence are recorded in this file
  (cmd: `grep -n "Decision:" tasks/20260721-160842/TASK.md`).
- content lint green (cmd: `cargo run -p nova_assets --bin content -- lint`).

## Notes

- Spike: tasks/20260721-155249/SPIKE.md (Open questions).
- Flow umbrella: 20260721-160425.
