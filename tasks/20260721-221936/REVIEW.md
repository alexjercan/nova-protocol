# Review: F11 debug-toggle inversion fix (20260721-221936)

## Round 1 (self-review, trivial-diff carve-out)

Small, well-scoped regression fix the owner co-diagnosed from the playtest
(the invert was reported precisely). Diff: three `DebugEnabled` inserts routed
through one const, one cross-crate mirror default, comments, two tests, one
CHANGELOG line. Reviewed against the risks that bite this kind of change:

- **Cross-crate mirror drift (the specific coupling that caused the bug):**
  `AmmoReadoutDebug` lives in nova_gameplay, which cannot depend on nova_debug,
  so it cannot share the const. Mitigated by (a) matching literal + a comment on
  each side pointing at the other, and (b) `f11_flips_the_ammo_debug_flag` now
  pins the mirror default off. The three nova_debug states share
  `DEBUG_LAYER_STARTS_ON`, so they are structurally locked together.
- **Something else assuming default-on:** checked. The reel/screenshot harness
  calls `hide_dev_overlays` (sets them false; idempotent, so a false default is
  a no-op there). `driver_debug_number_follows_the_toggle` sets the flag
  explicitly, not via default. No test asserted a `true` default. Grep for
  `DebugEnabled(true)` / `AmmoReadoutDebug` default assumptions came back clean.
- **avian coverage:** avian PhysicsGizmos + physics UI follow the inspector's
  `DebugEnabled` (bcs `enable_physics_gizmos` / `enable_physics_ui`), which is
  one of the three flipped, so they boot off and rise with F11 - no separate
  flip needed. Confirmed in bcs source.
- **Cursor invariant preserved:** the inspector still defaults off, so
  `sync_inspector_cursor` leaves the flight grab hidden at boot - the
  20260721-211500 behavior holds. Its four tests still pass.

Tests green: nova_debug 11, nova_gameplay 605.

## Verdict: APPROVE

The `manual:` DoD item (owner presses F11 - whole layer appears, then hides,
cursor re-locks) batches for the Finish checkpoint.
