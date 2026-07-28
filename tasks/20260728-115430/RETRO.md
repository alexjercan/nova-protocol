# Retro: NOVA OS ship app - side inspector panel

- TASK: 20260728-115430
- BRANCH: feature/ship-inspector-panel
- REVIEW ROUNDS: 2 (R1 out-of-context REQUEST_CHANGES; R2 in-session APPROVE)

See TASK.md Work Log for what changed; this file is process only.

## What went well

- Kept the buttons in lockstep with the action handler by deriving enablement +
  reason strings in one `panel_action_state` helper from the SAME conditions
  `apply_action_to_section` enforces, rather than re-implementing button-enable
  logic that could drift. The reviewer specifically verified no drift.
- Test-first on the pure helpers, and a real GPU screenshot confirmed the panel
  layout + the disabled-with-reason UX (Reload dimmed + "no ammo feed" for the
  controller) before calling it done.
- The out-of-context round-1 reviewer did its job: it caught a coverage gap the
  implementing session was blind to (below), which no in-session "review
  carefully" would have surfaced.

## What went wrong

- R1.1 (MAJOR): I ticked Step 6 - which named "a live-tree test that runs
  `update_ship_panel`" - having only written the PURE-helper tests
  (`panel_detail_text`, `panel_action_state`) and the observer test. The system
  that WIRES those helpers into the panel tree and caches the
  `panel_repair_enabled`/`panel_reload_enabled` flags the observers read was never
  run by a test; it would have survived being reverted to a no-op. Root cause: I
  treated "the helpers it calls are tested" as satisfying a step that named a
  system-level test. The pieces were pinned; the wiring was not.
- R1.2 (MINOR): I pinned only the Reload button observer, not Repair - even though
  `pin-each-caller-not-just-shared-core` was already in the ledger and I had cited
  it. Two symmetric entry points, one pinned.

## What to improve next time

- When a Step names a specific test artifact (a live-tree/system test of X), the
  tick requires THAT artifact - a nearby pure-helper test does not satisfy it.
  For any per-frame system that maps helpers into the tree or caches state other
  code reads, ask "would this test pass if the system were a no-op?" before
  ticking.
- When a change adds N symmetric entry points (2 button observers), pin each one
  in the same pass - apply the pin-each-caller lesson proactively, not after a
  reviewer points at the missing half.

## Action items

- [x] Ledger: added `test-the-wiring-system-not-just-its-pure-helpers`; bumped
  `pin-each-caller-not-just-shared-core` to x2.
- Manual acceptance (owner playtest) still open - listed in REVIEW.md R2. The
  armed-ship pip/enabled-Reload visual remains the standing capture-fixture gap
  (`new-render-primitive-verify-on-gpu`).
