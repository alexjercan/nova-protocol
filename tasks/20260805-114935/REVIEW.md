# Review: Name the signal when a smoke example dies without an exit code

- TASK: 20260805-114935
- BRANCH: test/name-signal-on-example-death

## Round 1

- REVIEWER: out-of-context
- VERDICT: APPROVE

- [ ] R1.1 (NIT) crates/nova_autopilot/src/exit.rs:34-40 - the six named arms
  each allocate (`"SIGSEGV".to_string()`) only to be consumed by
  `format!("was killed by {name}")` on the next line. Fold the prefix into the
  match so the named arms yield a `&'static str` phrase and only the fallback
  allocates, dropping the separate `name` binding.
  - Response:
- [ ] R1.2 (NIT) crates/nova_autopilot/src/exit.rs:52 - the
  `None => "exited with no code"` arm has no test; `mod tests` is
  `cfg(all(test, unix))` and no unix raw status reaches it. Add a comment on
  that line naming it as the non-unix signal-death path, unreachable on the
  tested target.
  - Response:

- Out of scope: `tail(&stderr)` is still duplicated across
  `tests/examples_smoke.rs` and
  `crates/nova_autopilot/tests/autopilot_example.rs`; deferred in `NOTES.md`.
- Out of scope: `nova_probe`'s supervisor and the `nova_assets` /
  `portal_install` command tests still format a bare `status.code()`; ruled out
  in `DECISION.md`.
- Process signal: scope matched the plan exactly - one public fn, three real
  callers, no unrequested knob or abstraction. Both deviations from the literal
  plan (`cfg(all(test, unix))` on the test module rather than per-test, rustfmt
  import reorder) are smaller than what was written and disclosed in the
  close-out.

Verified in the worktree, independently of the reviewer: all four DoD `cmd:`
proofs green (`pub mod exit;` at `lib.rs:85`, `pub fn describe` at
`exit.rs:27`, `cargo test -p nova_autopilot --lib exit::tests` = 5 passed, the
negated `status.code()` grep finds nothing in either test file); `cargo check
--test examples_smoke` clean, proving the bare-`cargo test` catalog path still
compiles with the unconditional dev-dep. Tests assert exact message text and
would fail on any rewording of `describe`; every `status.success()` predicate
is byte-identical, so no existing assertion was weakened. Close-out claims
match observed behavior.

Not run locally, per repo policy: clippy and the full example smoke suite (CI
owns both). The new message has therefore been observed only on synthesized
statuses, never on a real signal death.
