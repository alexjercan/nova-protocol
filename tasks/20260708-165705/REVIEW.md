# Review: Multi-target tracking + subtarget cycle HUD

- TASK: 20260708-165705
- BRANCH: feature/multi-target-cycle

## Round 1

- VERDICT: APPROVE

Verified independently: `cargo check --workspace` green; `input::` (128) and
`hud::` (69) test filters green in the worktree; the pin/maintenance
interaction re-derived from the diff (pin clears before maintenance when the
pinned ship dies, so the re-rank happens the same frame; hysteresis keeps
applying to the pinned incumbent since it stays the lock); swept the tree
for other ControlLeft/ControlRight/mouse_wheel consumers (none - the camera
uses Alt/RMB/triggers); confirmed SpaceshipPlayerInputPlugin and
SpaceshipTargetingPlugin are only ever added together (input/mod.rs:23-24),
so the resolver's new resource reads cannot dangle. The load-bearing claim
(binding-level Chord + BlockBy exclusivity) is proven by an end-to-end test
that drives the real rig bundle through EnhancedInputPlugin with simulated
keyboard and wheel input, both directions - this is the right kind of test
for the one genuinely fiddly piece.

Findings (non-blocking):

- [x] R1.1 (MINOR) crates/nova_gameplay/src/input/player.rs:200-219 - the
  cycle hints carry fixed non-empty keys ("SCROLL"/"CTRL+SCROLL")
  unconditionally, so rows 4-5 of the cluster render even when no flight rig
  exists, breaking the documented "no rig, no keys, no hints" invariant
  (keybind_hints.rs module doc + cluster_rows_stay_empty_without_a_flight_rig
  test, which only passes because it uses the default resource rather than
  running the resolver). Invisible in practice (the cluster despawns with
  the ship) but a robustness inconsistency: gate the fixed labels on the
  rig existing, e.g. empty key unless `q_stop.single().is_ok()`.
  - Response: fixed - `cycle_label()` empties both keys while the stop
    action (the rig) is missing.
- [x] R1.2 (MINOR) crates/nova_gameplay/src/input/player.rs:1108 - the
  end-to-end test covers the wheel path only; the claim that the pad DPadUp
  binding fires WITHOUT Ctrl (Chord on sibling binding entities does not
  leak to it) rests on reading bevy_enhanced_input's condition-attachment
  docs, not on a test. Gamepad simulation needs connection-event plumbing,
  so a test is heavier; acceptable to leave, but note it in TASK.md as
  untested-by-automation or add the pad case to the e2e test.
  - Response: fixed - the e2e test now connects a simulated gamepad
    (GamepadConnectionEvent + RawGamepadEvent::Button) and asserts DPadUp
    cycles targets with no modifier held and does not touch the component
    cycle; the counter increments, so the stimulus provably fired.

## Round 2

- VERDICT: APPROVE

Both R1 findings verified fixed: cycle_label() empties the wheel-hint keys
without a rig (input::player tests 15/15 green), and the e2e test's new pad
leg drives a simulated gamepad through the real pipeline - DPadUp fired the
target cycle (counter 1 -> 2, delivery guard satisfied) with the component
counter untouched. cargo check --workspace and fmt clean. No new findings.
