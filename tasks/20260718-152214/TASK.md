# Docs-follow-code audit: reconcile web/src/wiki/dev pages with current code, fix drift and gaps

- STATUS: OPEN
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

- [ ] `guide-author-scenario.md` "Launch it" section (~line 948): says to point
      the `NEW_GAME_SCENARIO_ID` constant in `crates/nova_menu/src/lib.rs` at
      your scenario "(one line of Rust)". That constant no longer exists; New
      Game start is the base bundle's `new_game_scenario` declaration (honored
      only from base; in code it lands in the private `NewGameScenario`
      resource, `crates/nova_menu/src/lib.rs` ~line 1148). Rewrite the section
      around the declaration; this is the only doc claim that actively strands
      a verbatim reader.
- [ ] `guide-make-a-mod.md` (lines ~56 and ~82) says base "is an implicit
      dependency and is never declared", but the shipped example manifest
      `assets/mods/example/example.bundle.ron` (~line 38) declares
      `dependencies: ["base"]`. Resolve the contradiction: preferred fix is
      dropping the declaration from the example so the strict doc stands;
      either way doc and example must agree.
- [ ] `project-tour.md` / `development.md`: add the two undocumented dev crates
      `nova_probe` (run-harness: frame-time capture + report) and `nova_meta_gen` (.meta
      sidecar generator, Trunk post_build hook), and list examples
      `20_perf_baseline` and `21_render_scale_shot` (noting they are not on the
      CI smoke list, unlike 01-19).
- [ ] `development.md`: add a consolidated reference for the `content` CLI
      (`gen`/`lint [--target <mod>]`/`audit`, plus `balance_acks.ron`) - today
      it is only mentioned in passing inside two guides.
- [ ] `guide-author-section.md`: document the v0.7.0 content-owned audio
      fields (turret `fire_sound`/`dry_fire_sound`, torpedo bay
      `launch_sound`/`detonation_sound`, controller radar/lock/safety sounds,
      thruster `loop_sound` and its shared-loop semantics, section
      `impact_sound`/`destroy_sound`, salvage `pickup_sound`).

General sweep (the part the review sampled rather than exhausted):

- [ ] Page-by-page reconcile each `web/src/wiki/dev/*.md` with the code it
      describes. Known suspects to verify:
  - [ ] `architecture.md` crate map vs the 15 real crates and plugin order.
  - [ ] `scenario-system.md` vs current actions/events/filters (Outcome, area
        OnEnter/OnExit, allegiance, `scenario_elapsed`, `orbit_hold_secs` /
        `lock_refire_secs`) shipped in v0.7.0.
  - [ ] `sections.md` vs render-mesh-transform + configurable colliders
        (v0.7.0 tasks 20260718-113307/121205/102022) and ammo slots.
  - [ ] `modding-ron.md` / `mod-portal.md` vs the bundle model and
        mod-relative resource refs.
  - [ ] `development.md` command list vs the real scripts/bins and the
        settings / RCS features added in v0.7.0.
- [ ] Verify each documented command actually runs; correct any that changed.
- [ ] Where a v0.7.0 feature has no page/section at all, add one (RCS, settings
      menu, graphics presets, render-scale lever, outcome frame).
- [ ] While editing `keeping-docs-in-sync.md`, record the news-page conventions
      the v0.7.0 post now sets: h2 sections + h3 subsections (the build derives
      the sticky TOC from them) and the figure-placeholder format.
- [ ] Record the drift found so the release-flow "keeping-docs-in-sync" step
      can be tightened to prevent recurrence.
- [ ] `cd web && npm run ci` green.

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
