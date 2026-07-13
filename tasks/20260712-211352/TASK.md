# Rework the examples to make more sense and to have better testing capabilities

- STATUS: OPEN
- PRIORITY: 80
- TAGS: v0.5.2,refactor,examples,tests

## Goal

The examples were not created with testing in mind initially, so I think it
would be a good idea to re-work them and refactor them (can even remove some of
them and add new ones) to be more "testable". Maybe even to touch on some
corner cases. Also testing things that we know were bugs (when we hit a bug it
might be a good idea to write a "example test" that reproduces it and then fix
it). This could be a good learning thing to think about.

## Baseline (2026-07-13 plan pass)

13 examples, ~4.2k lines. Seven run in CI via tests/examples_smoke.rs
`HARNESSED_EXAMPLES` (03_scenario, 06_torpedo_range, 07_torpedo_guidance,
08_turret_range, 09_editor, 10_gameplay, 13_menu_newgame) asserting exit 0 +
"reached Playing" + "cycle complete"; 03/07/10 also `assert_scenario_loaded`.
11_com_range and 12_hud_range carry the RICHEST multi-stage behavior
assertions in the tree but are NOT in `HARNESSED_EXAMPLES` (custom 8 s
AutopilotPlugin timelines instead of `nova_autopilot()`). 01_scene,
02_thruster_shader, 04_asteroids, 05_directional, 07b_slicer have no harness
at all (07b is marked "TODO: move to bevy_common_systems").

## Steps

- [ ] Audit: one keep / merge / remove / harness decision per example,
      written to this task's NOTES.md with a sentence of reasoning each.
      Inputs: what each demonstrates today, overlap (01_scene vs
      04_asteroids), and whether an interactive-only example still earns
      its keep. Checkpoint the audit table with the user in the progress
      report before executing removals.
- [ ] Find out why 11_com_range and 12_hud_range are not in
      HARNESSED_EXAMPLES (git log / task notes; likely just their custom
      timeline predating the const), then add them or record the blocker
      in NOTES.md.
- [ ] Execute the audit: fold/remove the examples decided against, and
      re-describe the survivors so the sequence reads as a curriculum
      (docs/development.md lists them).
- [ ] Harness every kept example that lacks one, each with at least one
      behavior assertion in its autopilot script - not just reach-Playing
      (presence-vs-behavior lesson).
- [ ] Establish the bug-pin convention: document in docs/development.md how
      a fixed bug becomes an example-level pin (13_menu_newgame /
      20260713-175352 is the precedent), and add pins for the two v0.5.2
      bug fixes where an example-level pin adds coverage beyond their unit
      tests (audio hum attenuation 20260711-183417, teardown despawn race
      20260712-115902 - the latter may already be covered by the
      error-handler-to-panic examples once a scenario unload happens under
      one; verify rather than assume).
- [ ] Update HARNESSED_EXAMPLES, docs/development.md's example list, and
      every other doc that enumerates examples
      (additions-join-doc-indexes lesson - grep docs/ for a sibling's
      name).
- [ ] Full check suite + the smoke test locally (Xvfb) over the reworked
      set.
- [ ] CHANGELOG.md entry under Unreleased if examples change
      names/behavior visibly.

## Notes

- Depends on: 20260711-183417 and 20260712-115902 (bug fixes land first so
  this task can pin them). PRIORITY 90 -> 80 in the v0.5.2 plan pass to
  encode that order; the substance is unchanged.
- 20260710-143138 (CI taffy) depends on THIS task: re-enable the smoke gate
  over the final example set, not the pre-rework one.
- Key files: tests/examples_smoke.rs, crates/nova_debug/src/harness.rs
  (module docs describe the harness contract), examples/*.rs,
  docs/development.md.
- Removing an example must sweep every reference (sweep-then-delete):
  docs/, HARNESSED_EXAMPLES, CI comments, other examples' comments.
