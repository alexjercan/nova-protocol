# Document the scenario-authoring vocabulary the shipped mods use; promote Gauntlet to the worked example

- STATUS: OPEN
- PRIORITY: 57
- TAGS: v0.8.0,docs,web,modding,scenario

## Story

As a mod author who has read the modding guides end to end, I want the docs to
teach the full scenario vocabulary the shipped mods are built on, so that I can
rebuild something like Gauntlet or The Ledger without reverse-engineering their
content files and test rigs.

The pre-v0.7.0 documentation review (2026-07-18) found that Gauntlet and The
Ledger - the two showcase portal mods - depend on at least seven features and
two authoring patterns that no guide explains. A reader who finishes
`guide-make-a-mod.md` and `guide-author-scenario.md` can author a section
overlay and a simple objective loop, but not either shipped mod. The patterns
are currently documented only in test files (`tests/gauntlet_course.rs`) and
content-file comments (`webmods/gauntlet/gauntlet.content.ron`).

## Steps

- [ ] Extend the action reference in `web/src/wiki/dev/guide-author-scenario.md`
      (and `scenario-system.md` where it is the deeper reference) with the
      undocumented actions:
  - [ ] `SetSkybox` (mid-scenario sky swap; Gauntlet's per-act boundaries are
        the example).
  - [ ] `ScatterObjects` (what the `seed` does, the `asteroid_radius` range
        syntax; Gauntlet's belt-wall field is the example).
  - [ ] `NextScenario` variants: `linger: true` retry loops, `linger: false`
        with `delay`, and Outcome + `auto_advance_secs`; include the lint-warned
        trap of pairing an Outcome with a non-lingering switch in one handler.
- [ ] Document the undocumented scenario-object fields:
  - [ ] `Asteroid` `invulnerable` (immovable scenery/cover) and
        `surface_gravity` (gravity-well rocks).
  - [ ] Per-spawn `impact_sound` / `destroy_sound` on asteroids and spawned
        ships (v0.7.0 audio surface).
  - [ ] `allegiance` override on spawned ships (`Some(Neutral)` bystanders).
- [ ] Add a "Scenario patterns" section to the dev wiki covering the two idioms
      both shipped mods rely on, with excerpts from the Gauntlet content file:
  - [ ] The gate-counter ordering pattern (a numeric variable + expression
        filters as a state machine enforcing ordered entry).
  - [ ] The act-gating pattern (guarding Defeat/outcome handlers on
        `act`/`gate` so a post-victory wreck cannot flip the result).
- [ ] Promote Gauntlet to the worked example: link its content file and the
      `gauntlet_course.rs` test rig from the guides as the reference
      implementation, and update `webmods/gauntlet/README.md` to point back at
      the new wiki sections instead of only naming features.
- [ ] Sweep The Ledger's content files for any remaining vocabulary the guides
      still do not cover after the above; document or list follow-ups.
- [ ] `cd web && npm run ci` green; TOC renders for the new sections.

## Definition of Done

- Every action, object field and filter used by `webmods/gauntlet` and
  `webmods/the-ledger` content files appears in the dev wiki with at least a
  syntax example and one sentence of when-to-use.
- The two handler patterns have a named, linkable wiki section with a worked
  Gauntlet excerpt each.
- A reader can trace Gauntlet end to end: guide -> content file -> test rig,
  with links in both directions.

## Notes

- Source findings: pre-v0.7.0 docs review (2026-07-18) mods audit; feature
  matrix showed SetSkybox, ScatterObjects, invulnerable, surface_gravity,
  impact/destroy sounds, NextScenario linger, and both patterns at "not
  documented".
- Why Gauntlet over The Ledger as the tutorial subject: one scenario vs four
  chapters, every pattern in a single content file, self-contained (depends on
  base only since v1.2.0), and its course header already documents its two
  geometric invariants.
- Pairs with 20260716-174729 (Gauntlet time-trial): if that lands first, its
  timer vocabulary belongs in the same reference pass.
- Complements 20260718-152214 (dev wiki drift audit): that task fixes what the
  docs say wrongly; this one adds what they do not say at all.
