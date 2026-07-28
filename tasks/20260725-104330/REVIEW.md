# Review: Epic - NOVA OS terminal drawer for v0.9.0

- TASK: 20260725-104330
- BRANCH: (epic container - reviewed as the aggregate of its child tasks)

## Round 1

- VERDICT: APPROVE
- REVIEWER: aggregate (each child task carries its own out-of-context REVIEW.md)

This is an epic container with no code diff of its own; its acceptance is the
sum of its children, each of which went through its own /review cycle to APPROVE
and landed on master. Verified at close:

- All 7 child tasks are STATUS: CLOSED and landed:
  - 20260726-115320 monitor shell + visual treatment
  - 20260726-115324 terminal input + command shell
  - 20260726-134738 match drawer to terminal PoC
  - 20260726-115330 terminal output commands (help/log/objectives/ship)
  - 20260726-115334 app runtime (app takeover + isolated input + exit)
  - 20260724-102320 map app (stretch) - later extended by 20260728-160001
  - 20260726-115339 ship viewer app + `ship` CLI verbs (stretch) - later
    extended by 20260728-125510
- Every "Done Means" criterion maps to a landed child (see TASK.md).
- Doc-surface spot-check: `web/src/wiki/hud.md` describes the one-screen NOVA OS
  model ("The old side panels are now one inset cockpit screen..."); no live doc
  still advertises the old two-panel/permanent-side-panel drawer. Done Means
  item 6 satisfied.
- Owner confirmed the three Manual Acceptance items at close (see TASK.md).

No open findings. The epic delivered its stated core plus both stretch apps.
