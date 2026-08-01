# Review: KISS: nova_scenario

- TASK: 20260731-170427
- BRANCH: refactor/kiss-nova-scenario

## Round 1

- REVIEWER: out-of-context
- VERDICT: REQUEST_CHANGES

- [x] R1.1 (MAJOR) web/src/wiki/dev/guide-extend-scenarios.md:99 - the dev
  guides walk the reader to `crates/nova_scenario/src/loader.rs` (also
  guide-extend-scenarios.md:153 and :246 for `actions.rs`, and
  guide-author-scenario.md:1051 for `loader.rs`), all files this branch
  deleted, so the recipes now name paths that do not exist. AGENTS.md requires
  invalidated docs to ship with the code. Repoint each at the concrete new
  file: the engine-driven events fire from `loader/clock.rs` and
  `loader/trackers.rs`, `NewGameStart` is resolved in `loader/lifecycle.rs`,
  and the `EventActionConfig` variant lives in `actions/mod.rs` with its config
  plus impl in the concern submodule.
  - Response: repointed all four: guide-extend-scenarios.md:99 now names `loader/lifecycle.rs` (OnStart), `loader/clock.rs` (OnUpdate) and `loader/trackers.rs` (OnOrbit/locks); :153 names `actions/` and lists the concern submodules; :246 names `actions/spawn.rs`; guide-author-scenario.md:1051 names `loader/lifecycle.rs`. `grep -rn 'nova_scenario/src/(loader|actions|lint)\.rs' web/ README.md AGENTS.md docs/` is now empty.
- [x] R1.2 (MAJOR) crates/nova_scenario/src/actions/mod.rs:1 - the new module
  header's intra-doc link `[`EventActionConfig`]` does not resolve, so
  `cargo doc -p nova_scenario --no-deps` now emits one warning where master
  emitted none; AGENTS.md:109 requires `cargo doc --workspace --no-deps` to
  stay warning-free. Drop the brackets or write `[`self::EventActionConfig`]`,
  then re-run `cargo doc -p nova_scenario --no-deps` and confirm zero warnings.
  - Response: dropped the brackets - the header now reads `EventActionConfig` as plain code. `cargo doc -p nova_scenario --no-deps` on a wiped `target/doc` emits zero warnings for this crate.
- [x] R1.3 (MINOR) crates/nova_scenario/src/render_scale.rs:19 - the comment
  re-wrap collapsed the module doc's numbered list, so items 2 and 3 now run
  into the middle of the preceding line (`... window_physical`, 2.` at :19 and
  `... awareness, 3. spawns a` at :25) and rustdoc renders one paragraph
  instead of three steps. Put each `1.` / `2.` / `3.` back at the start of its
  own line.
  - Response: restored the three numbered items, each starting its own line with continuations indented under it.
- [x] R1.4 (MINOR) crates/nova_scenario/benches/scenario_dispatch.rs:12 - the
  same re-wrap merged the module doc's three `*` bullets into a run-on
  paragraph (`... entity filters). * condition_eval - ...`). Re-break each
  bullet onto its own line.
  - Response: restored the three `*` bullets, each on its own line with indented continuations.
- [x] R1.5 (MINOR) crates/nova_scenario/src/loader/trackers.rs:40 - `OrbitHold`
  and `LockEcho` (:140) were `pub` under `pub mod loader` on master, so
  `nova_scenario::loader::OrbitHold` was a reachable public path; they are now
  `pub(super)` inside a private module. The narrowing is right (neither is in
  any prelude and nothing outside the crate names them), but the commit message
  and NOTES.md both claim public paths are unchanged. Record the deliberate
  narrowing in NOTES.md.
  - Response: recorded in NOTES.md under the structure table: both were `pub` under `pub mod loader`, neither is in any prelude, nothing outside the crate names them, and the narrowing to `pub(super)` is deliberate.
- [x] R1.6 (NIT) tasks/20260731-170427/NOTES.md:24 - NOTES gives the largest
  remaining file as `objects/asteroid.rs` at 1080 lines; `wc -l` reports 1070.
  Correct the number.
  - Response: corrected to 1070.
- [x] R1.7 (NIT) crates/nova_scenario/src/loader/lifecycle.rs:371 - the
  surviving `NOTE:` on the despawn ordering dropped the pointer to
  `a_player_ships_despawn_does_not_race_the_cameras`, the test that pins the
  ordering the note asserts. A test name is a pin reference, not provenance;
  re-add it.
  - Response: re-added as `(pinned by `a_player_ships_despawn_does_not_race_the_cameras`)`.

Verification run in-session (not findings):

- `cargo check --workspace --all-targets` green; `cargo fmt --check` clean.
  DoD 1 and 2 pass.
- `cargo test -p nova_scenario --lib` 145 passed / 0 failed;
  `--test skybox_swap_e2e` 1 passed. DoD 5 passes and both numbers match the
  close-out exactly.
- DoD 3: `grep -rnE '//.*[0-9]{8}-[0-9]{6}' crates/nova_scenario/` returns zero
  hits, as NOTES claims.
- DoD 4: no file over 1500 lines; largest is `objects/asteroid.rs` at 1070.
- Re-derived independently of the reviewer: `OrbitHold` and `LockEcho` were
  `pub` on master (`git show master:.../loader.rs`); the four stale wiki paths
  exist and point at deleted files; `cargo doc -p nova_scenario --no-deps`
  emits exactly one warning, attributed to the new `actions/mod.rs` header.
- Pure-move claim holds: the reviewer's line-multiset comparison of the three
  split files against their folder modules leaves only `mod` / `use` /
  `pub use` lines and visibility keywords as residue. Test-name parity (90
  names, identical sorted lists) reproduced.
- Crate `prelude`, `loader::prelude`, `lint::prelude` and `actions::prelude`
  are byte-identical to master.

Pending user checks:

- DoD 6 `manual:` - owner skims the diff and agrees no behavior changed.

## Round 2

- REVIEWER: in-session (round-1 fixes are doc paths, comment re-wraps and one
  NOTES entry - no executable line changed, so a fresh reader adds nothing over
  re-running the checks and re-reading the seven sites)
- VERDICT: APPROVE

All seven round-1 findings verified fixed; no regressions found.

- `git diff` for round 2 touches only `web/src/wiki/dev/*.md`, three `//!`
  module headers, one `// NOTE:` body, and NOTES.md. `cargo check --workspace
  --all-targets` and `cargo fmt --check` clean; `cargo test -p nova_scenario
  --lib` 145 passed; `cargo doc -p nova_scenario --no-deps` on a wiped
  `target/doc` emits zero warnings for this crate (the four remaining workspace
  doc warnings are pre-existing `nova_gameplay` glob-visibility warnings on
  master, out of this diff's scope).
- Re-derived R1.1's fix: the stale-path grep over `web/`, `README.md`,
  `AGENTS.md` and `docs/` now returns nothing, and each replacement path exists
  and contains the symbol the guide sends the reader to look for.

Pending user checks:

- DoD 6 `manual:` - owner skims the diff and agrees no behavior changed.
