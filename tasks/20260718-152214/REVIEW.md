# Review: docs-follow-code audit of web/src/wiki/dev/

- TASK: 20260718-152214
- BRANCH: docs/dev-wiki-audit

## Round 1

- VERDICT: APPROVE

Focused diff (8 dev-wiki pages + one example manifest + TASK.md). `npm run ci`
green (prettier + eslint + webpack build), and the `example_scenario` merge
tests (14) pass with the base-dep change. Three out-of-context audit agents plus
a repo-wide stale-token grep drove the findings; I re-verified every load-bearing
claim against the code rather than trusting the agents:

- **NEW_GAME_SCENARIO_ID rewrite** - grep confirms the const is GONE; the
  replacement mechanism is real: `new_game_scenario` in base.bundle.ron:175
  (`Some("shakedown_run")`) -> `NewGameStart` (loader.rs:34), plus the picker's
  `NewGameScenario` override (nova_menu:1148). Both the section-7 body and the
  section-8 back-reference are fixed. This was the one reader-stranding claim.
- **base-dep contradiction** - the example manifest was self-contradictory (its
  own header comment forbids declaring base, line 38 declared it). Dropped to
  `dependencies: []`. Verified: no test asserts the example's declared deps
  (grep empty), base is force-enabled regardless (example_scenario.rs:305/346),
  and the 14 merge tests stay green after the change.
- **Verb lists** - re-counted against source: `FlightVerb` has 5 (Stop/Goto/
  Orbit/Lock/Rcs, controller_section.rs:209+), `ROW_VERBS` has 7 (added RCS,
  keybind_hints.rs:47-55). Both doc lists were understated; now match.
- **Crate maps** - 15 crates confirmed (`ls crates/`); `nova_probe` +
  `nova_meta_gen` added to architecture.md and project-tour.md in each page's
  own style. One-liners reconciled with AGENTS.md.
- **Content CLI** - the new `## Content CLI` section's cited paths were checked:
  `crates/nova_assets/balance_acks.ron` (I corrected a wrong `assets/base/...`
  guess before committing), and the three gate tests (content_ron_parity,
  content_lint_gate, balance_audit_gate) all exist.
- **sections.md / detonation_sound** - `collider: Option<SectionCollider>`
  (Cuboid/Sphere/Capsule/Cylinder, base_section.rs), `render_mesh_transform`
  (hull_section.rs:30), and torpedo `detonation_sound` (mod.rs:136) all verified
  before documenting.

Scope honesty: two steps are marked `[~]` in TASK.md, not `[x]` - (1) documented
commands are source-verified (content CLI + probe were RUN this session; the rest
against their clap defs, not a bare run of each - heavy builds), and (2)
player-facing runtime UI (settings menu, graphics presets) is deliberately left
to the PLAYER wiki (audited clean in the 2026-07-18 review) rather than
duplicated in the dev wiki. Both are honest scoping calls, recorded in the task.
The DoD's drift list is recorded in TASK.md for release-flow tightening.

No blocking or major findings. The one candidate NIT - development.md's
"Consolidated over time" line still names old numbered examples - is intentional
history (it documents what the old `NN_` examples merged into), verified correct,
so it is not a finding.
