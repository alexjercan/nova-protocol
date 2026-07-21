# Retro: Rust Tally destroy crash (20260721-224506)

## What went well

- The reproduction nailed the exact bug before any fix. Rather than a heavy
  end-to-end kill rig, `chain_ignore_deferred` + `FallbackErrorHandler(panic)`
  reproduced the precise frame order (a despawn buffer applied before
  mark_section_meshes's insert buffer) deterministically - and it panicked with
  the IDENTICAL error and call site the owner reported. Fail-first, aimed, cheap.
- The fix was already in the repo's vocabulary. `try_insert` (=
  `queue_silenced(insert)`) is the Bevy-documented remedy for this despawn race
  and was already used in gravity.rs/camera_controller.rs, so the fix matched
  house style instead of inventing one. The Bevy error message even named it.
- Held the class-audit line. The reviewer pressed on whether glue.rs/explode.rs
  should also be converted; the answer (verified) is no - those are synchronous
  `On<Add,_>` observers where the entity is guaranteed alive, so only the
  buffered `Added<Material>` query system races. Converting the observers
  speculatively would have masked potential ordering bugs for no gain. Fixing by
  MECHANISM (buffered-query-vs-async-despawn), not by "every insert in the area".
- Two evidence layers: the unit reproduction proves the mechanism; the broadside
  probe proved the REAL path (gunship broken -> explode over all children ->
  Victory, 0 panic lines), so the fix holds end to end, not just in the rig.

## What went wrong / was tricky

- Getting the reproduction's frame order right took understanding Bevy's
  auto-inserted sync points: a plain `despawn.before(mark)` would get a sync
  point (despawn applies, mark's query never sees the mesh -> no repro). Only
  `chain_ignore_deferred` keeps them in one apply window with the crashing order.
  A first instinct (`GLOBAL_ERROR_HANDLER`) was the wrong API for 0.19; the
  repo-proven `FallbackErrorHandler(panic)` resource was the right one.

## Lessons / what to do differently

- For a deferred-command-vs-despawn crash, reproduce the ORDER, not the whole
  gameplay: `chain_ignore_deferred` to co-locate the buffers + a panic fallback
  handler is a compact, deterministic rig - far cheaper than driving a full kill,
  and it pins the exact call site.
- `commands.entity(e).insert(..)` panics on a despawned `e`; `try_insert` does
  not. Any system that BUFFERS an insert on an entity a query found (not an
  observer's just-changed entity) is a despawn-race candidate - reach for
  `try_insert` there by default when the entity can be destroyed mid-frame.

## Follow-ups

- None. Manual owner replay (destroy the Rust Tally in Broadside) batches at
  Finish. Class audit found no other buffered-insert site with this hazard.
