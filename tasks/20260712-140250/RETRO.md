# Retro: Infinite ammo option for the first (New Game) scenario

- TASK: 20260712-140250
- BRANCH: feature/infinite-ammo-first-scenario (real sprout worktree; squash-landed as 041236e)
- REVIEW ROUNDS: 0 (autonomous flow, self-review only)

## What went well

- Worked in a REAL sprout worktree this time, directly applying this session's
  `landing-checkout-not-yours` lesson. The parallel /flow session kept committing
  to master throughout; the isolated worktree meant its churn never touched my
  branch, and the land was a clean branch-guarded squash. Isolation beat sharing
  the in-place checkout - exactly the promotion the ledger is arguing for.
- The feature fell out of the existing design with almost no new surface: the
  ammo system was already opt-in (`ammo_capacity: None` = unlimited), so
  "infinite ammo" is just overriding that to None at spawn for flagged player
  ships. No fire-system change, no new component or marker. The `speed_cap`
  precedent on `PlayerControllerConfig` gave the exact shape to copy.
- Applied would-it-fail-without-it: the mechanism test (flag -> stripped
  magazine) would still pass if someone set `infinite_ammo: false` in the
  shakedown player, so I added a second test that pins the actual New Game player
  to `infinite_ammo == true`. Two tests, two different failure modes.

## What went wrong

- Adding a required (non-Default) field to `PlayerControllerConfig` broke
  construction sites in TWO waves. `cargo check --workspace` caught the
  nova_editor site, I fixed it and thought I was done - but it does NOT compile
  examples, so six `examples/*.rs` were still broken and only surfaced via the
  editor diagnostics. Root cause: `--workspace` checks libs+bins, not
  examples/tests/benches. `cargo check --workspace --all-targets` catches all of
  them in one pass.
- The harness diagnostics briefly reported "missing field" against the MAIN
  checkout's file paths while my edits lived in the worktree - stale cross-
  checkout noise that could have been mistaken for a real error. `cargo check` on
  the worktree was the authoritative signal; the diagnostics were not.

## What to improve next time

- When adding a non-`Default` field to a widely-constructed struct (config
  structs especially), run `cargo check --workspace --all-targets` up front -
  examples and test/bench targets construct these too and a plain `--workspace`
  check gives a false all-clear.
- When working in a worktree, trust `cargo check` run IN the worktree over the
  harness's path-based diagnostics, which may be indexing the other checkout.

## Action items

- [x] Used a real sprout worktree (applied landing-checkout-not-yours) - reinforces
      the promotion candidate already in the ledger.
- [x] LESSONS.md: added check-all-targets-for-struct-field.
