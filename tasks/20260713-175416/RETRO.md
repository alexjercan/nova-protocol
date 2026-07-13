# Retro: Apply cubemap.png.meta in the real app via AssetMetaCheck::Paths

- TASK: 20260713-175416
- BRANCH: fix/cubemap-meta-check-paths (landed as 2518a7b)
- REVIEW ROUNDS: 1

## What went well

- A smooth cycle: three-line fix, one round, no rework. The plan carried
  verify-first facts (Paths semantics cited to bevy_asset source, trunk's
  copy-dir checked in index.html, the asset_loader derive alternative
  pre-discarded with the race argument), so implementation was mechanical.
- The regression test's design is the lesson made executable: it differs
  from the pre-existing test only in `..assets_plugin()` - the single
  variable that was broken. The A/B numbers (1 layer under Never, 6 under
  Paths) fell out for free.
- Review verified the consumer side, not just the diff: the pinned
  bevy-common-systems rev's skybox guard (`array_layer_count() == 1`)
  confirms an already-6-layer image no-ops - the July design finally
  reached in the shipped app.

## What went wrong

- Nothing new in this cycle. The bug itself is the July cycle's process
  failure surfacing: the cubemap fix was "verified" by a test whose
  AssetPlugin config diverged from the app's (default meta_check vs the
  app's Never), so it shipped broken and sat undetected for three days -
  until a browser log showed the canary warning. Root cause of the
  original escape: the rig was production-faithful in scheduling and data
  but not in CONFIGURATION.

## What to improve next time

- When a regression test guards behavior that depends on app configuration,
  build the rig from the shipped config constructor (here
  `nova_core::assets_plugin()`), not from defaults - config is part of the
  production-faithful-rigs rule, same as scheduling and hierarchy.

## Action items

- [x] LESSONS.md: bump `production-faithful-rigs` to x6 with the
      configuration variant sharpened into the sentence
- [ ] User confirms on the deployed web build that the single-layer warning
      is gone and the skybox renders (wasm meta HTTP fetch is only
      exercisable there)
