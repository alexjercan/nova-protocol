# REVIEW: drawer-open HUD-hide + gray backdrop (task 20260724-134335)

- BRANCH: feat/drawer-hide-hud
- ROUND 1: out-of-context reviewer (fresh context, re-derived every load-bearing
  claim from source + Bevy 0.19 semantics; compiled `cargo check` + `--tests`)
- VERDICT: APPROVE (no CRITICAL / MAJOR)

## What was verified

1. HUD hide/restore (`apply_hud_visibility`): `shown = level.shows(tier) && (!drawer_open || exempt)`
   traced across every combination. Non-exempt tiered widgets + indicators hide
   on `PauseStates::Drawer`; exempt (readout + keybind) stay (drawer-scoped
   exemption only - still subject to the grave/tilde cycle). Restore keys on
   `level.is_changed() || pause.is_changed()`, so a close un-hides in the same
   frame (state transition applies before Update/PostUpdate). `HudSelfDrivenVisibility`
   widgets keep the pre-existing R1.2 behaviour, no regression.
2. Z-order poke-over CANNOT regress: `PauseStates` is one mutually-exclusive
   enum, so `Drawer` and `Paused` never coexist. `lift_exempt_chrome_over_drawer`
   raises exempt chrome to z=12 ONLY in `Drawer`, else z=0; while the pause
   overlay (z10)/dialog (z11) is up the state is `Paused`, so the strip/keys sit
   at z=0 and are covered normally. z=12 > backdrop z=10, so the deepened gray
   cannot dim them. `set_if_neq` valid on `GlobalZIndex` (derives PartialEq/Eq).
3. Panel layout: right slides from `right`, left from `left`; both top-inset 52;
   left bottom-inset 140 clears the 7-row keybind cluster (~132px) with margin.
   Status strip is centered (width 160) while panels hug the edges (width 340),
   so no overlap regardless; the inset is belt-and-suspenders.
4. Lifecycle: both panels carry `DrawerRootMarker`; `remove_drawer` despawns
   both + the backdrop on player-ship removal. No leaked entities.
5. Tests non-vacuous: hide/restore drives real transitions; the lift test pins
   the exact anti-regression (z drops to 0 in `Paused`); the slide test's manual
   `Time<Real>` is sound (TimePlugin disabled, `advance_by` sets delta directly).
6. Bevy API clean: query disjointness maintained; observers correct.

## Nits (non-blocking, not actioned)

- mod.rs indicator loop: the `HudDrawerExempt` arm is currently dead (no screen
  indicator carries the marker). Kept as forward-proofing / symmetry with the
  root loop.
- `DRAWER_LEFT_BOTTOM_INSET_PX = 140` clears the 7-row cluster by only ~8px; the
  comment ties it to the row count, judged adequate.

## Scope note

Backdrop 0.55 -> 0.86 (heavy gray, no blur) and the shell+placeholder left panel
both match the recorded gate decision (DECISION.md); the divergence from the
written "add a blur" scope is auditable, not silent.
