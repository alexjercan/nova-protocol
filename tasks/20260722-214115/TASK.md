# Ledger per-chapter look: deliberate cubemap assignment + motivated SetSkybox accents (minimal, no new art)

- STATUS: OPEN
- PRIORITY: 46
- TAGS: v0.8.0, content, scenario

## Story

Give each chapter a distinct look without new art files (owner decision
2026-07-22: minimal - reuse base's two cubemaps + mid-scenario SetSkybox
accents; no self:// mod-carried images this pass). Today all five files point
at base's two cubemaps (ch1/ch4 = cubemap.png, ch2/ch2b/ch3 = cubemap_alt.png) -
only two looks, no per-chapter identity, and no in-scenario shifts.

Umbrella: 20260722-212808. Files: the five `ledger_ch*.content.ron`. Sequenced
AFTER the pacing/depth/ending tasks so its skybox edits rebase cleanly onto the
new handlers.

## Steps

- [ ] Assign base's two cubemaps deliberately per chapter so consecutive
      chapters read as distinct (rather than the current arbitrary split);
      document the intended per-chapter palette in NOTES.
- [ ] Add mid-scenario SetSkybox accents where a beat earns a look shift, e.g.
      ch4 sky darkens/shifts when the Auditor arrives (sell path) or when the
      box burns (burn path); ch3 shifts on entering the debris channel. Gate
      each SetSkybox on the beat that motivates it (not frame 0).
- [ ] Confirm SetSkybox wiring against the shipped mechanism
      (`PendingSkyboxSwap`, loader.rs; gauntlet.content.ron uses SetSkybox
      mid-run) - advertised-is-not-wired: verify the swap actually fires in a
      probe, not just that the action parses.
- [ ] `content lint --target the-ledger` clean; probe a chapter with a
      mid-scenario swap to confirm the skybox actually changes at the beat.

## Definition of Done

- Each chapter has a deliberate skybox identity, and at least the ch4 endings
  (and ideally ch3) carry a motivated mid-scenario SetSkybox accent. (manual:
  owner sees each chapter/beat rendered at Finish.)
- No new image files or self:// resources are added this pass (scope decision
  recorded in GOAL.md); the self:// mod-art path is deferred. (manual.)
- `content lint --target the-ledger` clean; the swap fires in a probe, not just
  parses. (cmd/probe.)

## Notes

Owner chose minimal look sourcing; a richer self:// per-chapter art pass is a
deferred follow-up (file at Finish if the owner wants it). The DoD's original
"self:// distinct look" is deliberately narrowed to cubemap-assignment +
SetSkybox accents.
