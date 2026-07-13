# Contextual keybind cluster: unavailable rows hidden, HUD-cycle row removed

- STATUS: CLOSED
- PRIORITY: 50
- TAGS: v0.5.0,hud,ux,playtest

## Outcome (CLOSED 2026-07-13)

Playtest (user, 2026-07-13; direct on master): "remove the ~ from the
keybind list; keybinds that are not useful can be not shown instead of
greyed out, and when they become useful show them (like in Arma
Reforger)". Shipped:

- The `[`] HUD` cycle row (HudLevelHintRow, task 20260711-180501) is
  REMOVED from the cluster - the backquote binding itself still works,
  it is just no longer listed (discoverability belongs to the future
  settings/keybinds screen, backlog 20260710-231927/20260711-180511).
- The cluster is CONTEXTUAL: a row renders (Display::Flex) only while its
  verb is actionable; unavailable verbs are Display::None instead of
  DIM_COLOR - the cluster now breathes with the situation (usually just
  [X] STOP + [CTRL] RADAR in open flight; [Z] CANCEL appears while
  engaged, [G] GOTO with a nav lock, [SCROLL] COMPONENT while focused).
- EXCEPTION: an EMPHASIZED verb (HintEmphasis, the scenario tutorial
  spotlight) shows even while unavailable - the shakedown must be able to
  point at a key just before it becomes actionable; its base stays dim
  and the gold pulse rides on top as before.
- update_hint_cluster now also wakes on emphasis changes; the standalone
  HUD-row plumbing (marker, query, block) is deleted.

Tests re-pinned: only-actionable-rows-show (incl. the emphasis override
with a clear-again delivery guard), no-rig hides every row. 471 lib tests
green; fmt clean. Live example still deferred while the user's game
instance runs (see 20260713-130305).

## Notes

- The dim-grey availability color (DIM_COLOR) survives only as the base
  of an emphasized-but-unavailable row.
