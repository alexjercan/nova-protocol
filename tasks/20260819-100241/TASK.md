# Ration wreck spawning again: the worst frame got worse without it

- STATUS: OPEN
- PRIORITY: 10
- TAGS: v0.11.0,performance,destruction

Epic: `20260818-220812`. Follow-up to `20260818-224219`, landed as `6a2d6eb5`.

**ON HOLD** by owner decision, 2026-08-19. Filed so it is not lost, not
scheduled. Do not start without the owner saying so.

## The defect

Deleting the slicer also deleted `FINALE_BODY_BUDGET` and the `FinaleQueue`,
because they lived in `mesh/explode.rs` and went out with it. They were not part
of the slicer - they were the RATIONING, and the detach path now has none.

Measured on `wfc_arena`, traced probe run, at the time of landing:

| | before | after |
| --- | --- | --- |
| `mesh::explode::handle_explosion` | 34.07 ms / 210 calls | absent |
| whole death path | 46.2 ms/run | **2.5 ms/run** |
| worst `Main` frame | 84.6 ms | **91.6 ms** |
| median `Main` | 36.0 ms | 38.5 ms |

The death path got 18x cheaper and the worst frame got WORSE. The old budget
spawned at most 8 destroyed bodies' pieces per frame while killing the body
immediately; without it, roughly 200 wrecks stand in a single frame and avian
pays what the slicer used to.

Caveat on the numbers: burst counts differ run to run (196 vs 155 deaths), so
`Main` is not strictly like-for-like. The per-system figures are.

## The fix

Reinstate the rationing on the detach path. The shape is already proven: hold
resolved SPAWN records rather than dying entities, so the body leaves the field
the moment it dies whatever the queue looks like - a zero-health wreck left
standing keeps a live collider and working capabilities. That distinction is
what made the original work and it must survive the rewrite.

## Be honest about what it buys

Rationing spreads arrival cost; it does not remove it. The original queue only
drained 24-32 of about 200 queued bodies inside the measured window, so "the
tail is cheap" was never PROVEN then either and will not be proven by
reinstating it. Bodies arriving over many frames genuinely cost less per frame
than 200 at once - that part is real - but if the goal is a wreck field that is
cheap rather than merely staggered, this is not that task.

## Done when

- Worst `Main` frame in the `wfc_arena` death burst is below the 84.6 ms it was
  before the detach landed, measured, not inferred.
- The body still leaves the field the frame it dies. Assert it.
- Death-path self time stays near 2.5 ms a run.
