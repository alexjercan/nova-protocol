# Review: Focus dwell + component fine-lock state and selection

- TASK: 20260709-192522
- BRANCH: feature/component-fine-lock (implementation commit 1bb08e0)

## Round 1

- VERDICT: APPROVE

Verified independently: fmt clean, `cargo check --workspace` green, 21
targeting / 29 input tests pass. The dwell semantics are right (the
change-detection frame resets without accruing, so FOCUS_TIME is a true
continuous hold); the pin correctly suppresses snap while open and dies with
its section or deadline; hysteresis keeps the incumbent unless a challenger
is decisively closer, including the incumbent-is-best identity case. The
lockable-while-attached decision (including SectionInactiveMarker) is
recorded on the resource doc where the next reader will look. DPad bindings
collide with nothing (no other DPad use in the repo; consume_input is false
regardless). The clear paths use set_if_neq, so the every-frame no-lock case
does not dirty change detection.

- [x] R1.1 (NIT) crates/nova_gameplay/src/input/targeting.rs - the
  unfocused-cycle no-op (step_component_lock's gate) has no test; a cycle
  press before the dwell completes must not select anything. One-assert
  test.
  - Response: fixed in f42301b - cycle_is_a_no_op_before_the_dwell_completes added.

## Round 2

- VERDICT: APPROVE

R1.1 verified; 22 targeting tests green. No new findings.
