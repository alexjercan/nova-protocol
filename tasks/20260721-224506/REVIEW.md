# Review: Rust Tally destroy crash (20260721-224506)

## Round 1 (out-of-context reviewer)

Reviewer examined `git diff master...HEAD` on `bug/section-tint-despawn-race`,
focused on (a) does the fix mask bugs, (b) is the reproduction a true fail-first
pin, (c) did the class audit leave a real crash unfixed.

### Findings: NONE

- **`try_insert`/`try_remove` correctness:** SOUND. `try_insert =
  queue_silenced(insert)` (the documented remedy, already the repo idiom in
  gravity.rs/camera_controller.rs). Both damage_tint sites converted. The silent
  drop is benign: a section mesh despawned mid-frame is not rendered, so the
  missing tint is meaningless - it hides no real bug. `remove` already used
  `queue_handled(_, warn)` (never panicked); `try_remove` just makes the whole
  despawned path silent and consistent.
- **Reproduction faithful:** SOUND, no false-green. `chain_ignore_deferred`
  guarantees no sync point, so the despawn buffer applies before mark's insert
  buffer - the order cannot flip. `FallbackErrorHandler(panic)` matches the
  shipped binary. Fails pre-fix with the exact reported error, passes post-fix.
  No global-state leak (the fallback handler is a local App resource).
- **Class audit:** SOUND. The section-render builders (hull/thruster/turret/
  controller) and the integrity observers (`glue.rs on_section_disable`,
  `explode.rs on_add_explodable_entity`) are SYNCHRONOUS `On<Add, _>` observers -
  the entity is guaranteed alive during execution, no deferred buffer between
  trigger and action, so they cannot hit a despawned entity. Only
  `mark_section_meshes` (a regular buffered system querying `Added<Material>`
  across all live entities) races an independent same-frame chain-destroy - that
  is the one fixed. Leaving the observers as `.insert` is correct.
- **CHANGELOG:** accurate, no over-claim.

Evidence: fail-first reproduction (exact reported panic) + broadside probe
(real gunship kill: "gunship broken" -> explode over all its children ->
"declaring Victory", 0 panic/ERROR lines).

## Verdict: APPROVE

The `manual:` DoD item (owner replays Broadside and destroys the Rust Tally)
batches for the Finish checkpoint.
