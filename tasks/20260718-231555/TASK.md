# Document the scenario-authoring vocabulary the shipped mods use; promote Gauntlet to the worked example

- STATUS: CLOSED
- PRIORITY: 43
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

- [x] Extend the action reference in `web/src/wiki/dev/guide-author-scenario.md`
      (and `scenario-system.md` where it is the deeper reference) with the
      undocumented actions:
  - [x] `SetSkybox` - covered by wiki (verified scenario-system.md +
        guide-author-scenario.md).
  - [x] `ScatterObjects` (`seed`, `asteroid_radius` range) - covered by wiki
        (verified guide-author-scenario.md; also modding-ron.md).
  - [x] `NextScenario` variants (`linger`, `delay`, Outcome +
        `auto_advance_secs`, the non-lingering-switch trap) - covered by wiki
        (verified scenario-system.md + guide-author-scenario.md).
- [x] Document the undocumented scenario-object fields:
  - [x] `Asteroid` `invulnerable` / `surface_gravity` - covered by wiki
        (verified scenario-system.md + guide-author-scenario.md).
  - [x] Per-spawn `impact_sound` / `destroy_sound` on asteroids and spawned
        ships - NEWLY ADDED to the `Asteroid` entry in scenario-system.md
        (grooming said guide-author-section.md, but that only covered a
        section's `base` block, not the Asteroid/scenario-object surface).
  - [x] `allegiance` override on spawned ships - covered by wiki (verified
        guide-author-scenario.md).
- [x] Add a "Scenario patterns" section to the dev wiki covering the two idioms
      both shipped mods rely on, with excerpts from the Gauntlet content file:
  - [x] The gate-counter ordering pattern - NEW section in scenario-system.md
        (real `gate` var, `Equal(gate, N)` filters).
  - [x] The act-gating pattern - NEW section in scenario-system.md (FINISH
        bumps `gate` to `8.0`; Defeat handler guarded `LessThan(gate, 8.0)`).
- [x] Promote Gauntlet to the worked example: content file + `gauntlet_course.rs`
      linked from the guides; `webmods/gauntlet/README.md` points back at the new
      wiki patterns section. Links go both directions.
- [x] Sweep The Ledger's content files - no residual (all vocabulary already
      documented; see close-out).
- [x] `cd web && npm run ci` green; TOC renders for the new sections.

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

## Grooming (2026-07-20): NARROWED + reprioritized 57 -> 43

A tree re-check found this task ~80% already delivered by the wiki: `SetSkybox`
(scenario-system.md), `ScatterObjects` (guide-author-scenario.md),
`NextScenario` linger (scenario-system.md), `Asteroid` invulnerable/
surface_gravity (scenario-system.md) and the per-spawn audio fields
(guide-author-section.md) are ALL now documented. Only two pieces remain and
are the real scope:
  1. the "Scenario patterns" section (gate-counter + act-gating idioms) -
     confirmed absent (`grep -ri 'gate.counter|act.gating' web/src/wiki/dev/`
     is empty);
  2. promoting Gauntlet to the worked example (content-file + test-rig
     cross-links).
Tick the already-done action/field sub-steps as covered-by-wiki when picked
up. No longer a docs-headline task - it is a small patterns + worked-example
pass, hence the demotion below the drift audit (152214) and ephemeral-docs
work.

## Close-out (2026-07-20)

STATUS: CLOSED.

### Grooming verification: already-covered vs newly-added

Verified every action/field sub-item against `web/src/wiki/dev/` before touching
anything:

- ALREADY COVERED (ticked as covered-by-wiki):
  - `SetSkybox` - scenario-system.md (Actions) + guide-author-scenario.md.
  - `ScatterObjects` (`seed`, `asteroid_radius`) - guide-author-scenario.md
    (section 4) + modding-ron.md.
  - `NextScenario` linger/delay + Outcome/`auto_advance_secs` + the
    non-lingering-switch trap - scenario-system.md + guide-author-scenario.md.
  - `Asteroid` `invulnerable` / `surface_gravity` - scenario-system.md
    (Scenario objects) + guide-author-scenario.md.
  - `allegiance: Some(Neutral)` - guide-author-scenario.md (section 4).
- NEWLY ADDED (grooming claim was stale for one item):
  - Per-spawn `impact_sound` / `destroy_sound` on ASTEROIDS/scenario objects.
    Grooming pointed at guide-author-section.md, but that page only documents the
    two fields on a section's `base` block, not on the `Asteroid`/scenario-object
    surface the Gauntlet rocks use. Added them to the `Asteroid(AsteroidConfig)`
    bullet in scenario-system.md (confirmed the fields exist on `AsteroidConfig`
    in `crates/nova_scenario/src/objects/asteroid.rs`).

### The two patterns (real names/syntax, from gauntlet.content.ron)

- GATE-COUNTER ordering: one numeric variable `gate` (seeded `1.0` in OnStart,
  range `1..=7`, terminal `8.0`). Each gate's OnEnter carries an `Entity` filter
  plus `Expression((Equal(Term(Factor(Name("gate"))), Term(Factor(Literal(Number(N.0)))))))`
  and, on fire, `VariableSet` bumps `gate` to `N+1` to arm only the next gate.
  Out-of-order or repeat entries match no live handler -> inert. Pinned by the
  rig's `gates_advance_only_in_order`.
- ACT-GATING: the FINISH handler sets `gate` to `8.0` (terminal) BEFORE declaring
  `Outcome(Victory)`; the OnDestroyed Defeat handler is guarded
  `Expression((LessThan(Term(Factor(Name("gate"))), Term(Factor(Literal(Number(8.0)))))))`
  so a wreck after the win declares nothing. Pinned by
  `wrecking_after_the_win_declares_nothing`.

Both idioms now live in a named "Scenario patterns" section in scenario-system.md
(the deeper reference), with verbatim excerpts, headings `The gate-counter
ordering pattern`, `The act-gating pattern`, `The Gauntlet worked example`.

### Worked-example cross-links (both directions)

- scenario-system.md "The Gauntlet worked example" -> content file
  `webmods/gauntlet/gauntlet.content.ron` + rig
  `crates/nova_assets/tests/gauntlet_course.rs` + the objective-loop guide.
- guide-author-scenario.md section 6 -> the Gauntlet worked example + scenario
  patterns anchors.
- webmods/gauntlet/README.md -> the wiki Scenario patterns section + names the
  test rig. (README's stale "v1.1.0" line vs the bundle's 1.2.0 is pre-existing
  and out of scope; not touched.)

### The Ledger sweep: no residual

Inventoried every action, object kind, and field in
`webmods/the-ledger/*.content.ron`. Everything is already documented:
`SalvageCrate` + `pickup_sound`, `modifications` (section overlays), AI
`patrol`/`leash`/`engage_delay`, `lock_signature`, `allegiance`, `StoryMessage`
(`speaker`/`text`), Cylinder scatter `y_min`/`y_max`, `seed`/`seeded`. No
`Conditional`/`Not`/`And`/`Or` filter is used by either mod. No follow-ups.

### Verify

`cd web && npm run ci` -> exit 0 (format:check + lint + build all pass;
`webpack ... compiled successfully`). The built page
`dist/wiki/dev/scenario-system/index.html` carries anchors `scenario-patterns`,
`the-gate-counter-ordering-pattern`, `the-act-gating-pattern`,
`the-gauntlet-worked-example`; the search-index `headings` list in
`web/src/wiki-pages.ts` was extended with these so the nav/search picks them up.
All cross-link anchors were verified against the rendered ids (fixed one: the
objective-loop anchor is `#6-worked-example-an-objective-loop`, numbered).

### Self-reflection

The one real gap (asteroid audio fields) was exactly where the grooming note was
imprecise - "audio in guide-author-section.md" was true for SECTIONS but not for
the scenario-object surface the task actually asked about. Verifying each claim
against the source struct rather than trusting the grep hit caught it. Rendering
the site and grepping the built HTML for the real anchor slugs (rather than
guessing the slugger's rules) caught the numbered-heading anchor mismatch before
it shipped a dead link.
