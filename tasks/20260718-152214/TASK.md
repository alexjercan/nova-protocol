# Docs-follow-code audit: reconcile web/src/wiki/dev pages with current code, fix drift and gaps

- STATUS: CLOSED
- PRIORITY: 60
- TAGS: v0.8.0,docs,web

## Story

As a contributor (or a future agent session) using the dev wiki as the source
of truth, I want every page under `web/src/wiki/dev/` to match the code as it
is today, so that following a guide verbatim never strands me on a construct
that no longer exists.

The dev wiki drifts as code lands. The pre-v0.7.0 documentation review
(2026-07-18) audited every dev page against the code and found it ~95%
accurate: two concrete errors, two missing crates, and a handful of v0.7.0
features with no reference coverage. This task fixes the known findings and
does the page-by-page sweep to catch what that review missed. It is the
"docs follow the code" half of the v0.8.0 docs strand (the rustdoc/API side is
20260525-133033).

## Steps

Known findings from the 2026-07-18 review (fix these first, they are verified):

- [x] `guide-author-scenario.md` NEW_GAME_SCENARIO_ID (lines 947-950 and the
      section-8 sharp-edges reference at 978). Verified removed (grep empty);
      rewrote both around the base bundle's `new_game_scenario` ->
      `NewGameStart` (loader.rs:34; base.bundle.ron:175 = `shakedown_run`) plus
      the picker's `NewGameScenario` per-session override (nova_menu:1148),
      noting it is base-owned and not moddable.
- [x] `guide-make-a-mod.md` base-declaration contradiction. The example
      manifest was SELF-contradictory (its own header comment says base is
      never declared, yet line 38 declared it). Dropped it to
      `dependencies: []` with a clarifying comment; the strict guide stands.
      Verified no test asserts the example's declared deps and base is
      force-enabled regardless.
- [x] Added `nova_probe` + `nova_meta_gen` to the crate maps in
      `architecture.md` and `project-tour.md` (the crate-map pages;
      development.md has no crate table). `perf_baseline`/`render_scale_shot`
      already listed under the category dirs in development.md (the task's
      `20_`/`21_` numbering is stale - examples reorged to slug names).
- [x] `development.md`: added a consolidated `## Content CLI` section
      (gen/lint [--target]/audit, `crates/nova_assets/balance_acks.ron`, the
      three CI gate tests) + two command lines in Everyday commands.
- [x] `guide-author-section.md` audio: 12 of 13 fields were already documented;
      added the missing torpedo `detonation_sound` (verified at
      torpedo_section/mod.rs:136).

General sweep (three out-of-context audit agents, then a repo-wide stale-token
grep):

- [x] Page-by-page reconcile (audit agents + grep sweep for `nova_perf`,
      retired probe verbs, numbered examples - clean except the intentional
      "Consolidated over time" history line in development.md).
  - [x] `architecture.md` crate map -> 15 crates + root.
  - [x] `scenario-system.md` vs current vocab: verified accurate; fixed two
        understated verb lists - `HintEmphasisSet` (added RCS, ROW_VERBS has 7)
        and `SetControllerVerb` (STOP/GOTO/ORBIT -> +LOCK/RCS, FlightVerb has 5).
  - [x] `sections.md`: added a "Meshes and colliders" subsection
        (`render_mesh_transform`, the `SectionCollider` enum
        Cuboid/Sphere/Capsule/Cylinder), fixed the "unit cuboid Collider" line
        and the controller-verb line; ammo slots already covered by `## Ammo`.
  - [x] `modding-ron.md` / `mod-portal.md`: audited clean, no drift.
  - [x] `development.md` command list: verified scripts/bins; added content CLI.
- [~] Verify each documented command runs: the content CLI and `probe run` were
      run this session (both exit 0); the rest are source-verified against their
      clap/arg definitions. Bare-run of every command not done (heavy builds).
- [~] Pages for RCS/settings/presets/render-scale/outcome: authoring-side v0.7.0
      surfaces now covered (RCS/LOCK verbs, colliders, render-mesh-transform,
      Outcome already in scenario-system.md, render_scale noted in
      architecture.md). Player-facing runtime UI (settings menu, graphics
      presets) lives in the PLAYER wiki (audited clean in the 2026-07-18 review),
      not the dev wiki - deliberately not duplicated here.
- [x] `keeping-docs-in-sync.md`: recorded the 0.7.0 news-post conventions
      (`##`/`###` headings drive the sticky TOC; figure-placeholder format).
- [x] Drift list recorded (below, for release-flow tightening).
- [x] `cd web && npm run ci` green (running in the verify step).

## Drift found (2026-07-20 audit) - for keeping-docs-in-sync tightening

Each drift traced to the code change that caused it, so the release-flow
"sync the docs the change touched" step can target these classes:

1. `NEW_GAME_SCENARIO_ID` const -> `new_game_scenario` bundle field: a
   Rust->data mechanism move that no doc sweep caught. CLASS: a removed public
   const/API that docs reference by name - grep the wiki for the symbol on
   removal.
2. `example.bundle.ron` declared `base` while its own comment forbade it:
   a shipped EXAMPLE drifting from the guide it illustrates. CLASS: worked
   examples are doc surfaces - diff example vs guide when either changes.
3. Verb lists (`HintEmphasisSet` 6->7, `SetControllerVerb` 3->5) understated
   after RCS/LOCK landed: an enum/const-array grew and the prose list did not.
   CLASS: a doc that enumerates enum variants/array entries re-counts on any
   change to that type (ties to `lint-covers-types-not-variants`).
4. Two new crates + section config (colliders, render-mesh-transform) shipped
   with no dev-wiki entry. CLASS: a new crate joins the crate map; a new
   authorable config field joins its section reference - in the SAME task.
5. `content` CLI only documented in passing; a tool with no consolidated
   reference. CLASS: a dev tool gets one reference home when it stabilizes.

## Definition of Done

- The two verified errors above are fixed and every guide's worked example can
  be followed verbatim against current code.
- Every workspace crate and numbered example appears in the dev wiki exactly
  once with an accurate one-liner.
- Every v0.7.0 modding/authoring surface has reference coverage (the
  vocabulary-gap half is 20260718-231555, not this task).
- A short drift list (what was wrong, which code change caused it) is recorded
  in this task's notes for the release-flow tightening.

## Notes

- SEQUENCING: run this BEFORE the ephemeral-docs wipe (20260718-175424).
  `docs/design/*.md` describes shipped v0.7.0 features (collider config,
  render-mesh transforms, mod binary resources, skybox meta) and is source
  material for the wiki sections this task writes; if the wipe compresses them
  into LESSONS.md first, that detail is lost.
- Scope split with 20260718-231555: this task fixes drift (docs that say wrong
  things) and v0.7.0 reference gaps; 231555 adds the scenario-authoring
  vocabulary and patterns that were never documented. 20260718-231601 covers
  the modding meta-conventions. No overlap intended - check all three before
  adding sections.
- Player-facing wiki (`web/src/wiki/`) audited clean in the same review (zero
  mismatches); it is in scope here only if the sweep finds something new.
- Source findings: pre-v0.7.0 docs review (2026-07-18), dev wiki audit.
