# Retro: Mod loading + load-order overlay + a demo mod

- TASK: 20260714-134127
- BRANCH: modding/mods-overlay
- REVIEW ROUNDS: 1 (APPROVE)

## What went well

- The mechanism dropped out of the existing shape: `ModList` is `BundleAsset` one
  level up (deps = mod bundles instead of content files), so the loader, the
  VisitAssetDependencies gating, and the naming rule were all copy-the-pattern. The
  generic-first sequencing across the family kept paying off.
- The just-fixed untyped-extension bug (163342) landed one step before this task
  would have re-hit it: I folded the stem rule into the plan up front
  (`enabled.mods.ron`, `demo.bundle.ron`) and added an untyped-load guard for the
  enable-list, so the mods layer shipped without the same trap.
- Verification went past the tests to the real production path: after noticing my
  first integration test called `merge_bundles` directly (bypassing the ModList ->
  register_bundles path the game runs), I added `register_bundles_applies_enabled_mods`
  driving the real system with a populated ModList, AND did a live in-game run with
  the demo mod ENABLED (0 errors). That is exactly the `test-the-production-load-path`
  lesson applied the same day it was written.
- Independent adversarial review (out-of-context) plus a self re-derivation of the
  recursive load-gating (bevy_asset_loader's `is_loaded_with_dependencies`) gave real
  confidence rather than green-test confidence.

## What went wrong

- Nothing broke, but a stray uncommitted `PRIORITY: 32 -> 36` edit sat in the shared
  main checkout the whole time (a parallel session's metadata bump) and aborted the
  squash-merge at landing. Resolved by confirming it was ONLY the priority bump (which
  my branch's TASK.md already carried) and discarding it. Cost a minute; a reminder
  that the shared checkout can hold edits that aren't mine.
- My first pass framed the intra-bundle conflict as a "mod authoring error"; the
  reviewer correctly noted it also governs the base bundle (its content files flatten
  into one bundle), a latent first-kept-vs-last-wins difference from master. No dup ids
  exist today, so it is invisible - but the doc now says so instead of implying
  mod-only.

## What to improve next time

- When a task builds directly on a just-fixed bug, re-read the fix's forward-note and
  bake it into the plan BEFORE implementing (done here - worth keeping as habit).
- A test that constructs the intermediate data directly (here `merge_bundles([..])`)
  is not the production path; add the one that drives the real system resource
  (`register_bundles` reading `Res<Assets<ModList>>`) too.

## Action items

- [x] Full-path test + live enabled-mod run added.
- [x] `merge_bundles` doc generalized to note the base bundle is also conflict-checked.
- [ ] The demo mod ships DISABLED (`enabled.mods.ron` = `(mods: [])`) to preserve the
      behavior gate - offer to enable it for the modding showcase (flagged to user).
- [ ] 134115 (ship kind) remains deferred until a real consumer; the demo mod could
      become that consumer.
