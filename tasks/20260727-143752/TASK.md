# Fix catalog_matches_disk: smoke-list screenshot_nova_os example

- STATUS: OPEN
- PRIORITY: 45
- TAGS: v0.9.0,bug,test

`tests/examples_smoke.rs::catalog_matches_disk` fails on master: the
`screenshot_nova_os` example is present in the Cargo.toml `[[example]]` catalog
(and on disk under examples/screenshots/) but is in NONE of the smoke lists
(SECTIONS/GAMEPLAY/UI/SCREENSHOTS) nor in NOT_SMOKED. The catalog test asserts
`accounted == catalog_names`, so it fails: "smoke lists (+ NOT_SMOKED) and the
catalog disagree".

Introduced by commit a98de8ed (task 20260726-180807), which added
`screenshot_nova_os` without listing it. Discovered mid-flow during task
20260727-135204 (CRT frame polish).

## Flow State

- FLOW STEP: PLANNED
- PLAN STATUS: APPROVED

## Steps

- [ ] Add `"screenshot_nova_os"` to the `SCREENSHOTS` list in
      `tests/examples_smoke.rs` (it is a harnessed screenshot producer like its
      siblings screenshot_reel/ui/combat/...), so `catalog_matches_disk` passes.
      (If it should NOT smoke, add it to `NOT_SMOKED` with a reason instead - but
      it runs a full harnessed cycle headless like the others, so SCREENSHOTS is
      right.)
- [ ] Confirm: `cargo test --test examples_smoke catalog_matches_disk` passes.

## Definition of Done

- catalog_matches_disk is green. (cmd: nix develop --command cargo test --test examples_smoke catalog_matches_disk)
