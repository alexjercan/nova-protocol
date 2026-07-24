# RETRO: drawer-open HUD-hide + gray backdrop (task 20260724-134335)

- OUTCOME: CLOSED, review APPROVE round 1, probe playable OK.
- BRANCH: feat/drawer-hide-hud (one squash commit).

## What changed and why

Reworked how the Tab drawer OPENS: opening it now hides the flight HUD and
deepens the gray backdrop into an inert field, instead of stacking a panel over
a still-visible HUD that fought the drawer for readability.

- HUD hide reuses the existing `HudTier` / `apply_hud_visibility` machinery
  rather than a new axis: a `HudDrawerExempt` marker keeps the top status strip
  and the lower-left keybind hints; everything else hides on `PauseStates::Drawer`
  and restores on close. The restore branch now also keys on `pause.is_changed()`
  so a close un-hides in the same frame, not only on a grave/tilde level change.
- `drive_drawer_slide` generalized to a `DrawerSide` so one system drives both
  the right panel (objectives) and a new left panel (comms/log shell +
  placeholder; content stays in task 102309). Both top-inset to reserve the
  status strip; the left panel is bottom-inset to clear the keybind cluster.
- Backdrop alpha 0.55 -> 0.86.

## Decisions

The written scope asked to ADD a scene blur. At the /flow gate the owner chose
HEAVY GRAY ONLY (no post-process): bevy 0.19 has no UI backdrop-filter, so a
real blur would mean a fullscreen gaussian post-process render node (WebGL2/wasm
risk) or a depth-dependent DoF - neither judged worth it this sprint. Recorded
in DECISION.md so the divergence is auditable. If a future playtest wants a true
blur, the custom-node vs DoF fork is the starting point.

## Difficulties / bugs caught

- Z-ORDER TRAP (caught pre-review). The deepened backdrop (z=10) would dim the
  exempt status strip/keys unless they rose above it. The naive fix - a static
  high `GlobalZIndex` on the exempt chrome - is a REGRESSION: the drawer backdrop
  and the pause overlay BOTH sit at z=10, and `hide_hud_chrome` only fires for
  the MAIN menu, so during the PAUSE menu the HUD stays `HudVisibility::All`. A
  static z=12 would have made the status strip poke OVER the pause overlay
  (z=10) and dialog (z=11). Fix: `lift_exempt_chrome_over_drawer` raises the
  exempt chrome to z=12 ONLY while `PauseStates::Drawer` is active (base z=0
  otherwise); since `Drawer` and `Paused` are mutually exclusive on the one
  freeze enum, the pause overlay always covers the HUD normally. A test pins the
  anti-regression (z drops to 0 in `Paused`).
- Test clock: `drive_drawer_slide` reads `Time<Real>`, which `TimePlugin`
  rewrites from the wall clock every frame - so a manual `advance_by` under
  `MinimalPlugins` gets overwritten and the slide barely moves. Fix: disable
  `TimePlugin` in the slide test and own `Time<Real>` by hand.

## Self-reflection / for next time

- The z-order trap is the kind of thing a static "just bump the z" reflex would
  have shipped; it was only caught by asking "what else lives at this z?" and
  finding the pause overlay shares the tier. Lesson: when raising a widget above
  a modal backdrop, enumerate every overlay at that z tier and every state that
  shows the widget - the shared-z modal is the counterexample. Worth a ledger
  entry (see below).
- Gating the exempt lift on `PauseStates` (not a static z) is the general shape:
  a widget that must out-rank ONE specific modal should be lifted by that
  modal's state, not given a permanently high z.
- The out-of-context reviewer added no new findings but independently
  re-derived the z-order safety and the test soundness - continued evidence
  that `out-of-context-review-pass` is worth the round-1 cost even on a change
  the implementing session audited.

## Ledger candidate

- `lift-widget-by-modal-state-not-static-z`: to keep a widget readable above ONE
  modal backdrop that shares a z tier with OTHER modals, gate its z-lift on that
  modal's state instead of a static high z - a static z poked the flight status
  strip over the pause menu because the drawer backdrop and pause overlay both
  sit at GlobalZIndex(10). 20260724-134335.
