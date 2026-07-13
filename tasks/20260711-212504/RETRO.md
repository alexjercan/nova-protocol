# Retro: Menu ambience thruster-flown AI orbit

- TASK: 20260711-212504
- BRANCH: feat/menu-thruster-orbit (landed 6c2718b)
- REVIEW ROUNDS: 1 (APPROVE; 2 MINOR + 1 NIT, addressed pre-landing)

## What went well

- Net-negative feature: the payoff task DELETED more than it added in
  nova_menu (~160 lines of staging math, a marker, a helper, two tests)
  because the autopilot already owned the runtime-geometry derivation the
  menu hand-rolled. The spike's insistence on finding the existing
  substrate (ORBIT self-plans) is what made the deletion possible.
- The visual verification rig worked and is now recorded: launch via cargo
  run (assets resolve from the manifest, not the binary path), capture the
  game window by X id, diff timed screenshots and confirm the changes
  localize to the ship's track. The flame was plainly visible in the
  crops - this is the only verification layer that could have caught a
  dead ship, wrong framing, or an ugly insertion swing.
- The reviewer's independent SOI/stable-band re-derivation turned "the
  spawn radius is probably fine" into numbers (spawn 140 vs band
  ~122-138..~490-557, SOI ~640-728), closing the spike's open question
  about insertion behavior with math instead of hope.

## What went wrong

- The sweep-then-delete prose lesson from TWO cycles ago recurred in
  miniature: I updated the comment at the orbiter spawn site but missed
  the menu_ambience function-level doc a page up in the SAME file, and the
  CHANGELOG Unreleased entry (R1.1, R1.2). Root cause: I grepped for the
  deleted symbols but not for the deleted BEHAVIOR's description
  ("ballistic"). Symbol sweeps catch code; behavior-word sweeps catch
  prose - both greps are needed.
- Two launch-rig stumbles cost a run each (bare binary path breaks asset
  resolution; full-screen scrot grabs the desktop, not the game). Cheap,
  but only because the rig is now written into the task's close record.

## What to improve next time

- When deleting a mechanism, grep for its describing WORDS (here:
  "ballistic", "seeds") across code comments, module docs, and
  CHANGELOG - not just its symbol names. CHANGELOG Unreleased entries
  are especially easy to falsify silently.
- Scene-facing tasks keep the run-and-watch step; screenshots by window
  id, multiple timestamps, diff-localization as the would-it-fail check.

## Action items

- [x] Ledger: bump sweep-then-delete (x4, behavior-words variant) - this
      is now recurring even WITH the prose variant on record; strengthen
      the pending promotion wording.
