# Review: Make nova_autopilot predicate-driven: a generic scripted state machine

- TASK: 20260802-120025
- BRANCH: refactor/predicate-autopilot

## Round 1

- REVIEWER: out-of-context
- VERDICT: APPROVE

No BLOCKER or MAJOR. All eleven Steps are done as written, both deletion greps
are empty, and every `test:`/`cmd:` proof in the Definition of Done is green.
The findings below are MINOR/NIT polish; none blocks landing. The primary
independently re-derived R1.1 and R1.6 from source and reran fmt,
`check --features debug`, the `--lib` suite and both DoD greps.

- [ ] R1.1 (MINOR) crates/nova_autopilot/src/autopilot.rs:516 - the terminal
  arm calls `HarnessCompletion::done(AUTOPILOT)` unconditionally, but the four
  script-owned examples end their last step on `script_reports_done()`, i.e.
  after the script already cleared `AUTOPILOT`. The driver reports it a second
  time and logs `harness completion: done(autopilot) but it is not pending` in
  every broadside / lifeline / menu_scenarios / screenshot_nova_os run. Guard
  the call with `is_pending(completion::AUTOPILOT)`.
  - Response:

- [ ] R1.2 (MINOR) examples/ui/menu_scenarios.rs:154 - the comment still
  credits `guard_run_completion` with turning an early exit into a panic; this
  diff deleted that function. Point it at the `SCENARIOS_AUTOPILOT_SECS`
  deadline on the "walk the scenarios picker" step.
  - Response:

- [ ] R1.3 (MINOR) crates/nova_autopilot/src/autopilot.rs:833 -
  `loop_point_restarts_at_the_labeled_step_and_resets` scripts a single `hold`,
  so `loop_from` resolves to index 0 and the test cannot distinguish "jumped to
  the named step" from "restarted from the beginning"; no caller uses a
  non-zero loop point either. Add a three-step script whose `loop_from` names
  step 2 and assert step 0 is not re-entered.
  - Response:

- [ ] R1.4 (MINOR) crates/nova_autopilot/src/autopilot.rs:788 -
  `stalled_step_aborts_naming_the_step` asserts only a non-success exit and a
  still-pending collector, but the DoD promises the message names the step, its
  in-step elapsed and the observed state. Only the out-of-process test checks
  the name; nothing checks elapsed or state. Capture the log with a `tracing`
  layer and assert the three fields, or narrow the DoD wording.
  - Response:

- [ ] R1.5 (MINOR) crates/nova_autopilot/src/input.rs:22 - the module docs
  promote "one `press_key` beat and one `release_key` beat, not a press
  repeated every frame", and playable/hud_range now hold CTRL across a whole
  step. But `bevy_input`'s `keyboard_input_system` releases all keycodes on
  `KeyboardFocusLost`; the old per-frame re-press was immune, a single press is
  not, and a dropped modifier becomes a silent deadline stall. Document the
  caveat, or re-assert the press from `each` in the sweeps that depend on it.
  - Response:

- [ ] R1.6 (MINOR) CHANGELOG.md:55 - "com_range, hud_range and playable are
  rewritten onto them ... so a script waits on what the game agreed happened
  rather than on a guessed duration" overstates hud_range: only its first step
  waits on the world (`player_ship_present()`), the other twelve are
  `elapsed(0.2..0.6)` dwells. The example's own module doc is honest ("waiting
  on the world or on its own short dwell"); match the CHANGELOG to it.
  - Response:

- [ ] R1.7 (NIT) examples/gameplay/playable.rs:394 - `combat_lock_live`,
  `travel_lock_live` (:400) and `goto_closing` (:407) spell
  `std::sync::Arc<dyn Fn(&World) -> bool + Send + Sync>` by hand instead of the
  exported `Arc<Predicate>` alias. Use the alias.
  - Response:

- [ ] R1.8 (NIT) crates/nova_autopilot/src/autopilot.rs:285 -
  `StepBuilder::until` takes `Arc<Predicate>`, forcing an explicit `Arc::new`
  plus a spelled-out return type at every ad-hoc predicate (see R1.7). Accept
  `impl Into<Arc<Predicate>>` instead.
  - Response:

- [ ] R1.9 (NIT) crates/nova_autopilot/src/input.rs:70 and
  crates/nova_autopilot/src/predicate.rs:83 - `move_cursor` and `state_is` have
  no caller outside the crate's own tests and the prelude pin, the same
  criterion DECISION.md used to defer `or`, `drag` and `observe`. Both are
  named in the Steps, so this is spec-conformant; noted only as an
  inconsistency to weigh if either stays unused.
  - Response:

### Verification

| Command | Result |
| --- | --- |
| `cargo fmt --all --check` | exit 0 |
| `cargo check --workspace --all-targets --features debug` | exit 0 (only pre-existing `ambiguous_import_visibilities` in `nova_gameplay`) |
| `cargo test -p nova_autopilot --lib` | 30 passed, 0 failed - all five DoD lib tests present |
| `cargo test -p nova_autopilot --test prelude` | 3 passed (reviewer) |
| `cargo test -p nova_autopilot --test autopilot_example` (Xvfb) | 2 passed (reviewer) |
| DoD grep `self_completing\|loop_while_pending` | no hits |
| DoD grep `playing_since\|guard_script_completion\|guard_run_completion` | no hits |
| `cargo run -p nova_probe -- run com_range,hud_range,playable` | exit 0, all three OK (reviewer) |
| `cargo run -p nova_probe -- run broadside` | exit 0, OK; run.log carries R1.1's warning (reviewer) |

Full workspace test suite not run: it OOMs this machine (standing constraint).

Pending `manual:` items: none - the Definition of Done carries only `test:` and
`cmd:` proofs.
