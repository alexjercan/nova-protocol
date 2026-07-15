# Review: Sync modding wiki guides + mod metadata to the now-playable scenarios

- TASK: 20260715-224823
- BRANCH: docs/mod-scenarios-sync

## Round 1

- VERDICT: APPROVE

Docs-only diff (2 files: `web/src/wiki/dev/guide-make-a-mod.md` + TASK.md).
Verified with fresh eyes against the shipped content on master:

- The guide's gauntlet publish-template description now equals
  `webmods/gauntlet/gauntlet.bundle.ron`'s meta verbatim, and the embedded
  `demo_mod_arena` description equals the shipped scenario description verbatim
  (checked by grep on both sides). No drift.
- The added `events` comment ("player + three targets -> per-target OnDestroyed
  counter -> one-shot OnUpdate, destroyed > 2") matches the actual arena win
  gate, including the `> 2` hardening from 20260715-224812. It does not claim
  more than the code does.
- Sweep completeness confirmed: `guide-extend-scenarios.md`,
  `guide-author-scenario.md` carry no references to either mod; `modding-ron.md`'s
  "gauntlet" is a cache-record example, not a gameplay description;
  `guide-author-section.md`'s overlay text is still accurate. Nothing else stale.
- No new wiki page, so no `WIKI_DOC_PAGES` / `wiki-pages.ts` change - correct for
  a sync. Markdown code fences balance (14). Only `web/src` edited; `web/dist`
  is generated.

Leaving the gauntlet out of `guide-extend-scenarios` as a worked example was the
right call - that is a documentation feature, not a sync, and belongs in its own
task if wanted. Clean; approving.
