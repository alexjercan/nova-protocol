# Retro: base bundle fails to load in-game (untyped extension)

- TASK: 20260714-163342
- BRANCH: fix/bundle-untyped-extension
- REVIEW ROUNDS: 1 (APPROVE)

## What went well

- diagnostic-first paid off: I resisted guessing at the fix and instead traced the
  exact path - read bevy's `get_full_extension`, found the untyped `load_untyped` in
  the bevy_asset_loader derive (`assets.rs:490`), and confirmed `loaders.find`'s
  by-asset-type fallback is what let the typed test pass. The "Asset Type: None" in the
  error was the single most load-bearing clue (untyped load) and I chased it instead of
  assuming the extension registration was simply wrong.
- Reproduced the real failure in-game before touching code (fail-first), and
  sabotage-verified the new guard fails on the old name - so the regression test is
  proven to catch exactly this bug, not just to pass.
- The fix is minimal and convention-level (a filename stem), not a code workaround.

## What went wrong

- The bug shipped in 134119 (folder bundle) and its review APPROVED, because the
  end-to-end test (`demo_scenario`) loaded the bundle with an explicit
  `Handle<BundleAsset>` - a TYPED load - while the game loads every collection field
  UNTYPED through bevy_asset_loader. The test's own doc even claimed it "drives the
  exact wiring the game ships"; it did not. Root cause: the test was written to the
  most convenient API (typed `load`), not the one the production consumer uses. A
  green typed test gave false confidence that the untyped game path worked.
- The single-dot compound-extension trap (`bundle.ron` -> `ron`) is invisible unless
  you know bevy's first-dot rule; the content files dodged it only by accident (they
  carry stems like `demo.content.ron`).

## What to improve next time

- When testing an asset load, exercise the SAME load path the production consumer uses.
  Here the game uses bevy_asset_loader's untyped kickoff, so the guard must load
  untyped. Typed convenience loads can mask extension-resolution failures entirely.
- For any new custom asset extension, name the on-disk file with a STEM so its bevy
  full extension (everything after the first dot) equals the registered compound
  extension. Bare `<ext1>.<ext2>` filenames resolve to `<ext2>` only.

## Action items

- [x] Regression guard loads the bundle untyped (the game's path), sabotage-verified.
- [x] Forward-note on task 134127: `mods.ron` must be stemmed (`*.mods.ron`) or it hits
      the identical untyped-extension failure. (Its plan will be updated when resumed.)
- [x] Lessons ledger: `test-the-production-load-path`, `stemmed-compound-extension`.
