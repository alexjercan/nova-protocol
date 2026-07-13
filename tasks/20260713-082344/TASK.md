# Reconcile targeting docs with the deliberate-radar model

- STATUS: CLOSED
- PRIORITY: 52
- TAGS: v0.5.0, docs, targeting, spike

## Outcome (CLOSED 2026-07-13)

Swept: supersession/reframe banners on tasks/20260709-192358/NOTES.md
(acquisition superseded, fine-lock layer stands), docs/2026-07-10-signature-
lock.md (range model now gates the RADAR picker; 15->5 debris retune noted)
and tasks/20260710-104421/NOTES.md (CombatLock components, deliberate
acquisition); CHANGELOG Unreleased coherence (the CTRL free-aim entry carries
its supersession note; the main radar entry landed with 082330; released
sections left historical); shakedown minimal text fix ("Hold [CTRL], look at
BEACON 3, release to lock it, then press [G]" - no test pinned the string);
fix records appended to spike 20260713-082207 (all four tasks). The
keybind-cluster gesture rows were confirmed deferred to 090653 (082337
honesty note), so no hint-doc changes were needed beyond the CHANGELOG.
Verified: nova_assets tests + 03_scenario autopilot green.

## Goal

After the radar family lands (20260713-082324/-082330/-082337), sweep the
docs so nothing asserts the dead models as current.

## Steps

- [x] Supersession banners / acquisition-section updates on the docs that
      describe passive acquisition or scroll cycling as current:
      tasks/20260710-195952/NOTES.md (the range MODEL survives as the radar
      picker's gate - reframe, don't delete), the component-lock doc
      (tasks/20260709-192358/NOTES.md), tasks/20260710-104421/NOTES.md
      (inset now keys off the CombatLock component), and any spike still
      marked RECOMMENDED that recommends scroll cycling.
- [x] CHANGELOG.md: the Unreleased section now contains contradictory
      targeting entries (sticky ship locks, torpedo cycle, CTRL free-aim);
      rewrite them into one coherent "deliberate radar locking" entry rather
      than stacking corrections.
- [x] Shakedown MINIMAL text correctness (the tutorial must not lie):
      "Lock BEACON 3 and press [G]" (nova_assets/src/scenario/shakedown.rs:583)
      -> radar phrasing ("hold [CTRL], look at BEACON 3, release, then [G]");
      check the pinned scenario tests for text assertions. The full teach-the-
      radar beat + polish is the separate shakedown task (20260713-090653).
- [x] Keybind-hints doc surface: hint rows changed (target-cycle row gone;
      radar/clear/raise rows added) - update any docs describing the cluster.
- [x] Append the fix-record entries in spike 20260713-082207 for the landed
      family, and a one-line status in the superseded 20260712-222610 pointing
      at what shipped instead.

## Notes

- Spike: tasks/20260713-082207/SPIKE.md.
- Depends on: 20260713-082337 (family complete).
- Replaces the dead docs task 20260712-223345 (closed wontdo); its file list
  is a starting inventory.
