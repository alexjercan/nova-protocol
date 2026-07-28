# DECISION: orbit-recenter model and T-reset behavior

- STATUS: ACCEPTED

## Context

The `ship` app orbit camera always centers on the whole-ship centroid. The
playtest request is to orbit around the SELECTED section instead. The task text
flagged one open fork: "decide whether reset (`T`) also returns the center to the
whole-ship centroid (likely yes)."

There is always a selection (it defaults to the first section), so "always ease
the center toward the selected section" and "T returns the center to the centroid"
are mechanically in tension: a naive reconcile that chases `selected` every frame
would immediately undo T's reframe on the next frame, and would also drift the
app off the whole-ship view on the very first frame after open.

## Decision

Retarget the orbit center only when the selection CHANGES, tracked by
`ShipOrbit.centered_on`:

- Init `centered_on` to the default selection with `center_target = centroid`, so
  the app opens framed on the whole ship (the default selection is treated as
  already centered at home).
- On a selection change, ease the center to the newly selected section.
- `T` retargets the center to `center_home` (the centroid) AND sets
  `centered_on = selected`, so the whole-ship reframe STICKS until the player
  picks a section again.

This honors the task's "likely yes" (T returns to the whole-ship centroid) with a
coherent, non-fighting mechanism, and preserves the open-on-whole-ship framing.

## Alternatives considered

- Always ease toward `selected` unconditionally: simplest, but T's reframe is
  undone next frame and the app drifts off the whole-ship view on open. Rejected.
- T clears the selection to fall back to centroid: empties the inspector panel and
  fights the default-selection logic. Rejected.

## Out of scope

- `T` does not reset the zoom/radius (it never did); only theta/phi (existing) and
  now the center are reset.
