# Retro: section catalog as data

- TASK: 20260714-113408
- BRANCH: modding/section-catalog
- REVIEW ROUNDS: 1 (APPROVE)

Process only; what/why in TASK.md, family status in the spike (110502) fix-record.

## What went well

- Faithful mirror of the already-reviewed `ScenarioAsset` pattern (asset + loader +
  generator + parity test + GameAssets wiring) made this fast and clean - one review
  round, APPROVE. Copying a proven shape beats reinventing.
- Applied prior lessons directly: `generate-data-from-code` (serialize `build_sections`,
  guard with `sections_ron_parity`), `check-examples-skips-tests` (ran `cargo test
  --workspace --no-run`), out-of-context review. The compounding paid off.
- Reused the existing `demo_scenario` end-to-end harness to also cover the catalog
  load + a `GameSections` assertion, instead of a new test rig.

## What went wrong

- **Cargo.lock left stale after landing.** The branch work commit staged explicit
  paths (`git add crates/... assets/...`) and omitted `Cargo.lock`, so the
  `nova_modding -> nova_gameplay` dep edge the new `Cargo.toml` required was never
  committed; it surfaced as a dirty lock on master post-merge (a `--locked`/CI build
  would regenerate it). Root cause: explicit-path staging - safe against stray files
  (the `no-worktree-stage-explicit-paths` habit) - silently drops legitimately-related
  generated files like the lock. Fixed with a follow-up commit on master.
- `sprout rm` could not remove the worktree (untracked build artifacts); needed a
  manual `git worktree remove --force` + `rm -rf`. Minor.

## What to improve next time

- When a commit changes a `Cargo.toml` dependency list, stage `Cargo.lock` in the
  same commit. More generally: after `git add <explicit paths>`, glance at
  `git status` for related generated files (lock, generated data) before committing.

## Action items

- [x] Cargo.lock edge committed on master (1647839).
- [x] Lessons ledger: added `stage-lock-with-manifest`.
- Family continues at tatr 20260714-113411 (prototype references + Modification model
  - the big dedup).
