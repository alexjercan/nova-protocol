# Review: Port the scripted autopilot driver into nova_autopilot

- TASK: 20260802-183343
- BRANCH: feat/autopilot-driver-port

## Round 1

- REVIEWER: out-of-context subagent; the primary re-ran every check and
  independently re-derived the port's line-for-line equivalence with the BCS
  source and the `MinimalPlugins`/`StatesPlugin` claim.
- VERDICT: APPROVE

The reviewer verified the port against
`bevy-common-systems/src/debug/harness/autopilot.rs` (only the doc text,
`AUTOPILOT_ENV = "NOVA_AUTOPILOT"` and `crate::completion` differ), re-ran
fmt/clippy/rustdoc/tests, and mutation-tested six behaviors -
`.before(InputSystems)`, `AppExit::error` -> `Success`, the disabled loop
early-finish, the dropped `st.elapsed = 0.0`, the dropped `st.done` guard, and
the dropped `write_message(AutopilotLoop)`. Each killed exactly one test, so
all four DoD items are genuinely pinned.

- [x] R1.1 (MINOR) crates/nova_autopilot/src/autopilot.rs:236 - the "skip the
  set when already in the first state" guard was unexercised: `TestState`
  defaults to `Boot` and every test's first `hold` was `Playing`, so the `!=`
  was always true. Replacing the guarded block with an unconditional
  `NextState::set` left all four tests green, while the timeline test's comment
  asserts the property ("no spurious OnExit/OnEnter"). Add a case whose first
  `hold` is the default state and count `OnEnter` runs.
  - Response: fixed. `a_timeline_starting_in_the_current_state_does_not_re_enter_it`
    holds `Boot` first and counts `OnEnter(TestState::Boot)` through a new
    `Seen::boot_enters` field, expecting `init_state`'s single entry. Re-ran
    the reviewer's mutation (guard -> unconditional set): the new test fails
    (2 entries) and the other four stay green. 11 lib tests + 1 doctest pass.

- [ ] R1.2 (NIT) crates/nova_autopilot/src/autopilot.rs:242 - deleting the
  early `return` after `st.started = true` (so the first frame consumes `dt`)
  also leaves the tests green, so "the clock starts a frame later" is unpinned.
  - Response: not fixed, deliberately. The property is a one-frame offset in a
    timeline whose steps are held in wall seconds; pinning it would assert on
    frame-level accounting the drivers do not promise, and no DoD item or
    caller depends on it. Left open rather than silently dropped.

## Checks

- `nix develop --command cargo test -p nova_autopilot`: 11 lib tests + 1
  doctest, all green.
- `cargo fmt --check`, `cargo clippy --all-targets`, and
  `RUSTDOCFLAGS=-Dwarnings cargo doc --no-deps`: clean for the crate.
- Epic standalone proof:
  `test -f crates/nova_autopilot/Cargo.toml && ! rg -n '^(nova_|bevy_common_systems|avian3d)' crates/nova_autopilot/Cargo.toml`
  passes; the manifest is untouched by this branch.

## Pending manual checks

- None.
