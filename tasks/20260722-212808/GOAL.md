# Goal: The Ledger polish + extension - richer, longer arc with divergent endings

- DATE: 20260722
- UMBRELLA TASK: 20260722-212808
- LANDING SCOPE: squash-merge each task to local master via `sprout land` (no
  push). Content + mod-resource + version bump + regenerated portal catalog all
  land to master. The LIVE portal publish/push and the over-the-wire native+web
  install verification are the OWNER's step (batched into Manual acceptance at
  Finish), per the 2026-07-22 clarification.

## Goal

Polish and extend The Ledger portal mod (currently bundle v1.5.0, four chapters
in five scenario files under `webmods/the-ledger/`) so the flagship portal mod
plays like a real campaign. Deepen the EXISTING chapters (no new chapter) with
stronger beats and more encounter variety - especially the thin chapter three -
apply the Shakedown beat-sheet pacing discipline throughout, give each chapter a
distinct look via mod-carried `self://` resources, and make the chapter-four
sell-vs-burn choice actually diverge: one ending AVOIDS the Auditor fight (a
real consequence) rather than both converging on the same brawl. Data/scenario +
mod-resource work only; no new engine features. Then bump the bundle version
(content rework = minor bump per 20260718-231601) and regenerate the portal
catalog so the owner can re-publish.

Carry forward the Shakedown learnings (owner playtests, tasks 20260712-110730,
20260717-155740/163033/163042/163050/163058, 20260721-211506): beat-sheet rhythm
(announce -> breathe -> arrive -> fight -> confirm -> breathe -> next), opening
conversation with clock-paced dwell, breathers between beats, simpler objective
text in the goal register, enemy telegraphs (warn + far spawn + engage_delay),
one StoryMessage per handler, objectives never share a frame with conversation,
and a clock-pumping test helper for any time-gated content.

## Done means

1. The arc is richer than 1.5.0: existing chapters gain acts/beat-depth and more
   encounter variety, with chapter three (the thinnest) materially deepened.
   (manual: owner replay at Finish confirms the added depth reads well; cmd:
   `git diff --stat` over the ledger RON shows substantial authored growth.)
2. The chapter-four choice visibly diverges: one ending avoids the Auditor fight
   entirely and the two endings reach distinct final situations. (test: a new
   ledger_ch4 encounter/branch rig asserts the two branches wire to different
   terminal outcomes; manual: owner replays both endings at Finish.)
3. Each chapter has a distinct look carried by mod resources (`self://` skybox/
   texture variety), not all reusing `dep://base` art. (manual: owner sees each
   chapter rendered at Finish; cmd: bundle/content references `self://` art per
   chapter.)
4. Pacing follows the beat sheet: no >1-StoryMessage-per-handler or
   StoryMessage+Outcome lint warnings, objectives post a beat after their intro
   line, fights telegraph. (cmd: `content lint --target the-ledger` clean, acks
   only with reasons.)
5. The ch2 fairness rig and any new/extended encounter tests pass with
   DELIBERATE assertion updates (not reactive). (cmd: `cargo test -p nova_assets
   --test ledger_ch2_encounter` and any new ledger tests green.)
6. Bundle version bumped per convention (minor bump for content rework) and the
   mod CHANGELOG updated. (cmd: `the-ledger.bundle.ron` meta.version bumped;
   CHANGELOG entry present.)
7. The portal catalog is regenerated for the new version. (cmd:
   `scripts/gen-portal.py` run clean, catalog entry + thumbnails present.)
8. Docs synced in-task: mod README/CHANGELOG, player-wiki Ledger flow, v0.8.0
   news-post notes. (manual: owner reviews doc surfaces.)
9. Playtest questions for the owner are listed in the tasks, not silently
   decided.

Overall: `content lint --target the-ledger` clean (acks with reasons), the full
check suite green on master, and the regenerated catalog ready for the owner to
publish.

## Tasks

Updated as tasks land (one line per land).

(planned in the /plan phase - see below)

## Manual acceptance (batched for the user at Finish)

- (pending) OWNER: replay the full extended chain and confirm pacing/feel (the
  diagnostic-first pace-map fixes read well; rush gone; ch3 no longer thin).
- (pending) OWNER: replay BOTH chapter-four endings and confirm the divergence
  lands (one path genuinely avoids the Auditor fight; endings feel distinct).
- (pending) OWNER: view each chapter and confirm the distinct `self://` look.
- (pending) OWNER: run the LIVE portal publish/push and verify an over-the-wire
  install on native AND web, and that an existing install updates in place
  keeping its enabled state.
- (pending) OWNER: review the synced docs (README/CHANGELOG, wiki Ledger flow,
  v0.8.0 news-post notes).
