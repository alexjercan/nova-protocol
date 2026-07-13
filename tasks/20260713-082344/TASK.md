# Reconcile targeting docs with the deliberate-radar model

- STATUS: OPEN
- PRIORITY: 52
- TAGS: v0.5.0, docs, targeting, spike

## Goal

After the radar family lands (20260713-082324/-082330/-082337), sweep the
docs so nothing asserts the dead models as current.

## Steps

- [ ] Supersession banners / acquisition-section updates on the docs that
      describe passive acquisition or scroll cycling as current:
      docs/2026-07-10-signature-lock.md (the range MODEL survives as the radar
      picker's gate - reframe, don't delete), the component-lock doc
      (docs/2026-07-09-component-lock.md), docs/2026-07-12-target-inset-view.md
      (inset now keys off the CombatLock component), and any spike still
      marked RECOMMENDED that recommends scroll cycling.
- [ ] CHANGELOG.md: the Unreleased section now contains contradictory
      targeting entries (sticky ship locks, torpedo cycle, CTRL free-aim);
      rewrite them into one coherent "deliberate radar locking" entry rather
      than stacking corrections.
- [ ] Shakedown MINIMAL text correctness (the tutorial must not lie):
      "Lock BEACON 3 and press [G]" (nova_assets/src/scenario/shakedown.rs:583)
      -> radar phrasing ("hold [CTRL], look at BEACON 3, release, then [G]");
      check the pinned scenario tests for text assertions. The full teach-the-
      radar beat + polish is the separate shakedown task (20260713-090653).
- [ ] Keybind-hints doc surface: hint rows changed (target-cycle row gone;
      radar/clear/raise rows added) - update any docs describing the cluster.
- [ ] Append the fix-record entries in spike 20260713-082207 for the landed
      family, and a one-line status in the superseded 20260712-222610 pointing
      at what shipped instead.

## Notes

- Spike: docs/spikes/20260713-082207-deliberate-radar-locking.md.
- Depends on: 20260713-082337 (family complete).
- Replaces the dead docs task 20260712-223345 (closed wontdo); its file list
  is a starting inventory.
