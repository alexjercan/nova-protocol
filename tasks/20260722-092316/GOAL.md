# Goal: mainline campaign pacing + non-combatant ship behavior

- DATE: 20260722
- UMBRELLA TASK: 20260722-092316
- LANDING SCOPE: squash-merge each task to local `master` (default flow);
  do NOT push (owner's call). No branch/sprout target requested by the owner.

## Goal

Fix the mainline campaign playtest feedback (owner, 2026-07-22) in two areas:

1. Message/objective pacing. Objectives must appear AFTER the scripted
   conversation that precedes them, never in parallel, and an
   "objective completed" must never be immediately followed by the next
   objective in the same instant - there must be a breathing-room beat
   (complete -> pause / optional conversation -> new objective), sequenced.

2. Non-combatant ship behavior. Neutral scripted bystanders (the Ceres Queen
   in Broadside) must float in place instead of crashing into the gravity
   well; the unarmed ally convoy in Lifeline must loiter/orbit and stay in the
   asteroid-belt region instead of drifting into the planetoid.

Two owner-requested items are deliberately BACKLOG-only and are NOT built in
this goal, only filed: a critical-damage state (a ship is combat-dead when its
weapons + thrusters are gone, player included) and a rethink of the kill
condition (destroying a ship should not require zeroing every section's
health).

## Done means

1. In every mainline scenario, the first objective appears only after the
   opening conversation completes, and no objective-complete is followed the
   same frame by the next objective
   (test: scenario walk assertions; manual: owner replays shakedown/broadside/
   lifeline and the message/objective rush is gone).
2. The gate/breather sequencing helpers live in one shared module, not copied
   per scenario file
   (cmd: `grep -rn "fn stamp_gate\|fn breather" crates/nova_assets/src/scenario`
   shows a single definition site).
3. A controller:None neutral ship in a gravity well holds position and does
   not crash into the well; combat ships + the player keep full gravity
   (test: gravity/scenario harness position-drift assertion + existing gravity
   tests green; manual: owner replays Broadside opening, Ceres Queen floats).
4. The Lifeline convoy haulers loiter/orbit within the belt region, never
   fight, and the "keep the convoy alive" objective + raider waves still work
   (test: lifeline walk in-region + non-engagement asserts + existing walk
   green; manual: owner replays Lifeline, haulers fly around and stay in belt).
5. Two backlog tasks are filed for the deferred critical-damage /
   kill-condition rethink
   (cmd: `nix develop --command tatr list -t backlog` shows 20260722-092320
   and 20260722-092326). DONE at planning time.

Overall: the full check suite passes on master (CI), and content lint is
clean.

## Tasks

- [x] 20260722-092421 (p85, nova_assets) Sequence objectives after
      conversations + breathing room between objective swaps
      landed 0ae5c7f9; 1 review round (APPROVE, out-of-context); new shared
      scenario/pacing.rs unifying the gate mechanism; shakedown opening panel
      now empty (owner decision). 20/20 scenario tests, lint clean.
- [x] 20260722-092427 (p78, nova_gameplay) Non-combatant ships hold station
      instead of falling into gravity wells (Ceres Queen floats)
      landed f328797d; 1 review round (APPROVE). Verify-first partly falsified
      the report (broadside/lifeline have no wells) - fix is the guaranteed
      "unpiloted ships never feel a well" rule (no current-content change).
      Filed follow-up 20260722-105556 (lint guard). The observed convoy drift
      is knockback -> task 092432.
- [ ] 20260722-092432 (p72, nova_assets) Ally convoy haulers loiter/orbit the
      belt instead of drifting into the planetoid (depends on 092427)
- [x] 20260722-114541 (p88, nova_assets) DISCOVERED MID-FLOW + fixed: the
      pacing pass (092421) stamped opening-objective gates with mark_clock at
      OnStart, where scenario_elapsed is undefined -> opening objectives never
      posted + 174 error lines. Found by probing lifeline for task 092432.
      landed d320e1dc; 1 review round (APPROVE); new pacing::open_gate + an
      OnStart-clock-read invariant. Lesson: probe scenario content changes.
- [x] 20260722-092320 (p0, backlog) FILED: critical-damage state feature
- [x] 20260722-092326 (p0, backlog) FILED: rethink kill condition
- [x] 20260722-105556 (p0, backlog) FILED: content-lint guard for a
      controller:None ship inside a well SOI (from task 092427 review)

## Manual acceptance (batched for the user at Finish)

- (pending) 092421: replay shakedown, broadside, lifeline - no objective shows
  during an opening conversation; "completed" and the next objective never pop
  together.
- (pending) 092427: replay Broadside opening - the Ceres Queen floats, does not
  fall into the gravity well.
- (pending) 092432: replay Lifeline - the ally haulers fly around / orbit and
  stay in the belt, never crashing into the planetoid, never fighting.
