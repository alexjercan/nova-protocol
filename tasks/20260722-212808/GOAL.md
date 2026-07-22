# Goal: The Ledger polish + extension - richer, longer arc with divergent endings

- DATE: 20260722
- UMBRELLA TASK: 20260722-212808
- SUPERSEDES: 20260718-152320 (the original single big task; decomposed here)
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
distinct look, and make the chapter-four sell-vs-burn choice actually diverge:
one ending AVOIDS the Auditor fight (a real consequence) rather than both
converging on the same brawl. Data/scenario + mod-resource work only; no new
engine features. Then bump the bundle version (content rework = minor bump per
20260718-231601) and regenerate the portal catalog so the owner can re-publish.

Owner clarifications (2026-07-22): (a) diagnostic-first pace-map, owner replays
at Finish; (b) deepen existing chapters, no new chapter; (c) one ch4 ending
avoids the Auditor fight; (d) MINIMAL look sourcing - reuse base's two cubemaps
+ mid-scenario SetSkybox accents, NO new self:// art files this pass; (e) land
to master, owner does the live publish.

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
   entirely and the two endings reach distinct terminal outcomes. (test: a new
   ledger_ch4_ending rig asserts the two branches wire to different terminal
   outcomes and only the sell path spawns the Auditor; manual: owner replays
   both endings at Finish.)
3. Each chapter has a distinct look via deliberate cubemap assignment across
   base's two skyboxes + motivated mid-scenario SetSkybox accents (owner chose
   MINIMAL sourcing - no new self:// art files this pass; the self:// mod-art
   path is deferred). (manual: owner sees each chapter/beat rendered at Finish;
   cmd: SetSkybox accents fire in a probe, not just parse.)
4. Pacing follows the beat sheet: no >1-StoryMessage-per-handler or
   StoryMessage+Outcome lint warnings, objectives post a beat after their intro
   line, fights telegraph. (cmd: `content lint --target the-ledger` clean, acks
   only with reasons.)
5. The ch2 fairness rig and the new ch4 ending rig pass with DELIBERATE
   assertion updates (not reactive). (cmd: `cargo test -p nova_assets --test
   ledger_ch2_encounter` and `--test ledger_ch4_ending` green.)
6. Bundle version bumped to 1.6.0 (minor, content rework) and the mod CHANGELOG
   updated from the final diff. (cmd: `the-ledger.bundle.ron` meta.version;
   CHANGELOG entry present.)
7. The portal catalog is regenerated for the new version and verified locally
   (entry + thumbnails). (cmd: `scripts/gen-portal.py` clean.)
8. Docs synced in-task: mod README/CHANGELOG, player-wiki Ledger flow, v0.8.0
   news-post notes. (manual: owner reviews doc surfaces.)
9. Playtest questions for the owner are listed in the tasks, not silently
   decided. (manual.)

Overall: `content lint --target the-ledger` clean (acks with reasons), the full
check suite green on master, and the regenerated catalog ready for the owner to
publish.

## Tasks

Updated as tasks land (one line per land).

- [x] 20260722-214053 (p60, the-ledger) Diagnostic: campaign-wide pace-map + weak-spot brief
      landed 60c5d40a; 1 review round (APPROVE, 5 accuracy nits fixed); the
      pace-map is the shared brief for -214058/-214105/-214110/-214115; 5
      owner playtest questions batched to Finish Manual acceptance.
- [x] 20260722-214058 (p56, the-ledger) Beat-sheet pacing pass: ch1/ch2/ch2b
      landed 803a4e0c; 1 review round (APPROVE). Opening conversations +
      lazy objectives + breathers + first dwell use; ch2/ch2b geometry
      untouched, Victory deferred a beat (act latch stays synchronous, no
      race); ledger_ch2_encounter gained pump_clock (12 passed).
- [x] 20260722-214105 (p54, the-ledger) ch3 depth: opening act + breather corridor + 2nd encounter
      landed def84930; 1 review round (APPROVE). Clock-paced opener +
      breather corridor + debris-pinch hazard (~24u gap, computed pin);
      new ledger_ch3_channel test (9 passed). Owner Q2 (pinch vs staggered
      combat contact) batched to Finish.
- [ ] 20260722-214110 (p52, the-ledger) ch4 diverging endings (+ ending test rig)
- [ ] 20260722-214115 (p46, the-ledger) Per-chapter look: cubemap assignment + SetSkybox accents
- [ ] 20260722-214119 (p40, the-ledger) Close-out: lint, version 1.6.0, ch2 test, catalog, doc sweep

Dependencies: 214053 (diagnostic) feeds 214058/214105/214110. 214115 (look)
sequences AFTER the content passes so its skybox edits rebase cleanly. 214119
(close-out) depends on all others.

## Manual acceptance (batched for the user at Finish)

- (pending) OWNER: replay the full extended chain and confirm pacing/feel (the
  diagnostic-first pace-map fixes read well; rush gone; ch3 no longer thin).
- (pending) OWNER: replay BOTH chapter-four endings and confirm the divergence
  lands (one path genuinely avoids the Auditor fight; endings feel distinct).
- (pending) OWNER: view each chapter and confirm the deliberate cubemap +
  SetSkybox look; decide whether a richer self:// art pass is worth a follow-up.
- (pending) OWNER: run the LIVE portal publish/push and verify an over-the-wire
  install on native AND web, and that an existing install updates in place
  keeping its enabled state.
- (pending) OWNER: review the synced docs (README/CHANGELOG, wiki Ledger flow,
  v0.8.0 news-post notes).
