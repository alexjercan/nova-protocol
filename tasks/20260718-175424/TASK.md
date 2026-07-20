# Ephemeral docs/ model: release-time compiles docs/ into LESSONS.md entries then wipes it; retire docs/plans in favor of tatr tasks

- STATUS: CLOSED
- PRIORITY: 56
- TAGS: v0.8.0,docs,tooling,release,refactor

## Story

As the project owner, I want docs/ to be free scratch space during a cycle and
empty (except LESSONS.md) at every tag, so that durable knowledge has exactly
two homes - the wiki for reference, LESSONS.md for lessons - and nothing rots
in a junk drawer between them.

Redefine the docs/ model on the changelog_entries pattern: docs/ becomes a
free scratch space during development (agents write whatever documentation they
feel like, no structure required), and at RELEASE time a step compiles
everything in docs/ into LESSONS.md entries and then removes the folder's
contents. LESSONS.md is the only durable output; docs/ is empty at every tag.
Plans stop living in docs/plans and become their own tatr tasks. This replaces
the "distribute junk to its correct home" approach (20260718-152225, which is
folded into this) with "fold into the lessons ledger, then wipe."

## Steps

- [x] Define the model precisely and rewrite docs/README.md to describe it:
      docs/ is transient scratch during a cycle; LESSONS.md is the compiled
      durable record; nothing in docs/ survives a release except LESSONS.md
      itself.
- [x] Build the release-time compile step (Rust bin or scripts/ Python): read
      every file left in docs/ (root notes, design/, any dated
      investigations), distill each into one or more LESSONS.md ledger entries
      (same format /compound appends), append them, then clear docs/ back to
      just LESSONS.md. Mirror how the news-fragment/changelog_entries flow
      compiles-then-clears.
- [x] Decide the fate of docs/design/*: reference-grade content goes to a wiki
      dev page, lesson-grade content into LESSONS.md, then remove - the point
      is docs/ holds nothing durable. See the sequencing note below: the wiki
      half of this is largely 20260718-152214's job and must happen first.
- [x] Retire docs/plans/: migrate existing release plans into tatr tasks (a
      plan is a task with the strand breakdown in its body, or a parent task
      linking the per-strand tasks), update any references, and stop writing
      new plans to docs/. Wire "plan lives in a tatr task" into the plan/flow
      workflow docs.
- [x] Add the enforcement guard (a pre-tag/CI check that docs/ contains only
      LESSONS.md) - the piece 20260718-152225 was narrowed to; keep them
      consistent.
- [x] Update the release checklist (web/src/wiki/dev/development.md +
      keeping-docs-in-sync.md) with the compile-and-wipe step.

## Definition of Done

- docs/README.md describes the ephemeral model and nothing else.
- One command compiles docs/ into LESSONS.md entries and leaves docs/ holding
  only LESSONS.md; running it on an already-clean docs/ is a no-op.
- docs/plans/ no longer exists; every live plan is a tatr task and no repo
  reference points at docs/plans.
- The guard (20260718-152225) fails a release when docs/ holds anything else,
  and the release checklist names the compile step right before it.

## Notes

- SEQUENCING (from the 2026-07-18 docs review): run the dev-wiki audit
  20260718-152214 BEFORE the first compile-and-wipe. docs/design currently
  holds shipped-feature write-ups (collider config, render-mesh transforms,
  mod binary resources, skybox meta) whose substance belongs in the dev wiki
  at full detail; compressing them straight into LESSONS.md entries would lose
  it. Wipe only after the wiki has absorbed what it needs.
- Supersedes the design of 20260718-152225 (distribute-to-correct-home); that
  task is reduced to the CI guard under this model.
- The v0.7.0 pre-release cleanup (20260718-152329) was the interim manual
  pass; once this lands, that manual step is replaced by the automated
  compile+wipe.
- LESSONS.md format + append behavior: see docs/LESSONS.md and the /compound
  skill.

## Implementation (2026-07-20)

Decisions (user): agent-distill + a mechanical wipe command (a script cannot
summarize free-form notes into good lessons); FULL wipe now (including the live
v0.8.0 plan). docs/ permanently keeps TWO meta files - LESSONS.md (ledger) and
README.md (the model's own doc) - reconciling the DoD's "README describes the
model" with "only LESSONS.md survives".

- Model + README: rewrote docs/README.md to the ephemeral model.
- Compile step: `scripts/wipe-docs.sh` clears docs/ to LESSONS.md + README.md,
  idempotent (verified: 2 entries cleared, re-run a no-op). The "compile" is the
  agent-distill pre-step, documented in the README + release checklist.
- docs/design/ (9 docs, assessed by an out-of-context pass): 4 already in the
  wiki + 2 superseded/transient -> deleted; 2 (craft-ships) migrated to
  guide-author-section.md (base ships prototypes mods reference; Inline vs
  Prototype); 1 (meta-always) distilled to the `asset-meta-always-web-cost`
  domain lesson. All 9 removed.
- docs/plans/ retired: the LIVE v0.8.0 plan folded into a `release`/`meta`
  tracker task (20260720-142428); v0.4-v0.7 plans + the applied
  sdlc-suggestions removed (history in git). All live docs/plans + docs/design
  references swept and redirected (AGENTS.md, development.md, keeping-docs-in-
  sync.md, Trunk.toml, nova_meta_gen, mod_refs.rs, the example mod README);
  tasks/* historical refs left frozen.
- Guard (absorbs 20260718-152225, now CLOSED): `scripts/check-docs-clean.sh` +
  a `guard-docs` job in release.yaml the build waits on. Verified exit 0 clean /
  1 on junk.
- Release checklist: development.md "Cutting a release" step 1 + keeping-docs-
  in-sync.md "When you cut a release" step 1 now name the compile-and-wipe.
- Workflow wiring: AGENTS.md records the ephemeral model + plans-are-tasks.
