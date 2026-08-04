# Decide: pin the nightly toolchain date and add -D warnings to the CI clippy step, or stay advisory

- PRIORITY: 0
- TAGS: backlog, ci, tooling
- ACTIVITY: -
- GATES: -
- RESOLUTION: WONTDO

## Goal

Task 20260719-001600 brought CI's clippy gate to zero warnings, but the gate
stays advisory: no `-D warnings`, because `rust-toolchain.toml` floats on
`channel = "nightly"` and a future nightly's new lints would redden CI with
zero code changes (exactly how that warning batch arrived).

## Steps

- [ ] Decide the fork: (a) pin a nightly date in rust-toolchain.toml AND add
      `-- -D warnings` to the Clippy step in .github/workflows/ci.yaml
      (warnings become blocking, toolchain drift becomes a deliberate bump),
      or (b) stay advisory and rely on periodic cleanups.
- [ ] If (a): pick the pin date, verify the workspace is warning-clean under
      it, update the CI comment explaining the pairing, and note the bump
      procedure in web/src/wiki/dev/development.md.

## Definition of Done

- The choice is recorded in this task and, if (a), CI fails on any new clippy
  warning under the pinned toolchain (manual: read the recorded choice; for (a),
  the CI clippy job goes red on a new warning).

## Notes

- Context: tasks/20260719-001600/NOTES.md ("CI gate deliberately NOT
  tightened").


## Dropped

- REASON: old
