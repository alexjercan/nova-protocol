# Review: Port the single-shot screenshot driver into nova_autopilot

- DATE: 20260802-185500
- TASK: 20260802-183346
- BRANCH: feat/screenshot-port
- WORKTREE: /home/alex/.cache/sprouts/nova-protocol/feat/screenshot-port
- BASE: master (d3d65b0f)

## Round 1

- REVIEWER: fresh `/flow 20260802-183346` session entering at REVIEWING, no
  implementation context (review skill rule 2 default).
- VERDICT: REQUEST_CHANGES

### R1.1 MAJOR - the DoD `cmd` silently skips two of the five DoD tests

`tasks/20260802-183346/TASK.md:110`

The proof command name-filters on `screenshot`:

    nix develop --command cargo test -p nova_autopilot screenshot

`cargo test <filter>` matches on test NAME, so in `tests/screenshot.rs` it runs
only `screenshot_env_resolution_pins_the_primary_window` and
`screenshot_reports_done_after_settling`. Re-derived on the branch:

    Running tests/screenshot.rs
    running 2 tests
    test result: ok. 2 passed; ... 2 filtered out

`unreached_target_state_error_exits` and
`hide_overlay_hook_runs_before_the_capture` are the two filtered out - both are
named DoD criteria (TASK.md:100, TASK.md:103). The DoD command therefore does
not prove two of the criteria it is the proof for, and a future regression in
either would land green.

This is the same class of defect the task already reasoned about one level up
("`--lib` is deliberately absent because it would skip the stand-down test") -
the guard caught the binary-level skip and missed the name-level one. The `rg`
guard does not help: it only rules out a vacuous ZERO-test run.

Both tests do pass when actually run (`cargo test -p nova_autopilot --test
screenshot` -> 4 passed), so this is a proof-instrument defect, not a code
defect.

Change: drop the name filter from the DoD `cmd` so the whole crate suite runs:

    rg -q '^pub const SCREENSHOT_ENV: &str = "NOVA_SHOT";' crates/nova_autopilot/src/screenshot.rs && nix develop --command cargo test -p nova_autopilot

That covers all three binaries plus the doctests, needs no target list to be
maintained as `reel` lands, and is what the close-out's second Evidence bullet
already ran.

### R1.2 MINOR - the Evidence bullet states counts the command does not produce

`tasks/20260802-183346/TASK.md` close-out, Evidence bullet 1

> `nix develop --command cargo test -p nova_autopilot screenshot` - 3 lib + 4
> integration + 1 stand-down, all green

That command yields 3 lib + **2** integration + 1 stand-down. The 4-integration
figure is the unfiltered run's. Correct the bullet alongside R1.1.

### R1.3 MINOR - dead file setup in the hook test

`crates/nova_autopilot/tests/screenshot.rs` (`hide_overlay_hook_runs_before_the_capture`)

The test sets `.path(...)` and brackets itself with two `remove_file` calls,
but never triggers `ScreenshotCaptured`, so no PNG is ever written. The setup
reads as if the test produces a file. Either drop the path plumbing and the two
`remove_file` calls, or leave a one-line note that the path is only there
because the builder needs one. Non-blocking.

### Responses (round 1)

- R1.1 FIXED - the DoD `cmd` drops the `screenshot` name filter and now runs
  `cargo test -p nova_autopilot`, covering all three binaries plus doc-tests.
  The criterion text records why neither `--lib` nor a name filter belongs.
- R1.2 FIXED - the Evidence bullet now quotes the DoD command's real counts
  (14 lib + 4 screenshot + 1 stand-down + 2 doc-tests), with the filtered
  form's undercount kept as the record of what went wrong.
- R1.3 FIXED - `hide_overlay_hook_runs_before_the_capture` drops `.path()` and
  both `remove_file` calls, and carries a note that nothing triggers
  `ScreenshotCaptured` so no file is written.

## Verified claims

Re-derived independently on the branch, not taken from the close-out:

- `cargo test -p nova_autopilot` - 14 lib + 4 screenshot + 1 stand-down + 2
  doc-tests, 0 failed.
- `cargo test -p nova_autopilot --test screenshot` - all 4 pass, 0 filtered.
- `cargo fmt --check -p nova_autopilot` - clean.
- `cargo doc -p nova_autopilot --no-deps` - builds, no warnings, so
  `#![warn(missing_docs)]` and every intra-doc link resolve.
- `rg` guard on `pub const SCREENSHOT_ENV: &str = "NOVA_SHOT";` - matches.

## Findings judged and dismissed

- **`MAX_WAIT_FRAMES` widened from private to `pub`.** Deviates from the
  source, but the give-up test lives in an integration binary and the
  alternative is a hardcoded 1800 that drifts. Documented in the close-out.
  Correct call.
- **The overlay hook is `Fn(&mut World)` rather than a Bevy system.** Matches
  `AutopilotPlugin::input`, the shape this crate already established, and
  avoids erasing a marker generic for one caller. DECISION.md states it; no
  alternative here deletes a concept.
- **The synthesized `ScreenshotCaptured` in the settle test.** Runs both real
  observers, so the ordering claim (PNG on disk, then done reported) is proven
  rather than asserted. Better than the spawn-timing fallback DECISION.md kept
  in reserve.
- **`tests/screenshot.rs` split out of the lib binary.** Not in the plan, but
  forced: `autopilot.rs`'s `arm()` sets `NOVA_AUTOPILOT` process-wide and would
  make all four assertions test an inert plugin. The diagnosis is recorded and
  the accident doubles as a falsification.

## Pending manual checks

None; the task declares no `manual:` proofs.

## Round 2

- REVIEWER: same session, verifying the fix round only (round-1 findings were
  all record/test-hygiene; no independent re-derivation lane applies).
- SCOPE: `abbbb626`, 3 files, +28/-13.
- VERDICT: APPROVE

All three round-1 findings confirmed fixed; no fix regressions.

- R1.1 confirmed. The DoD `cmd` now reads `... && nix develop --command cargo
  test -p nova_autopilot`. Re-ran it verbatim: `rg` guard matches, then 14 lib
  + 4 screenshot + 1 stand-down + 2 doc-tests, 0 failed - and the run names
  `unreached_target_state_error_exits` and
  `hide_overlay_hook_runs_before_the_capture`, the two the old form dropped.
  0 filtered out in every binary.
- R1.2 confirmed. The Evidence bullet quotes the new command's real counts, and
  the undercount is kept as record rather than deleted.
- R1.3 confirmed. `hide_overlay_hook_runs_before_the_capture` no longer sets
  `.path()` or touches the filesystem; the note states why no file is written.
  `shot_path` and the `PathBuf` import are still used by the settle test, so
  nothing is left dangling. `cargo fmt --check -p nova_autopilot` clean.

The `nova_hud` `ambiguous_import_visibilities` warnings surfaced during this
round are in `hud/nova_os_ship` and `hud/nova_os_map`, files this diff does not
touch; out of scope here.

## Inspection

    cd "$(sprout show feat/screenshot-port)"
    git diff master...HEAD
    nix develop --command cargo test -p nova_autopilot
    nix develop --command cargo test -p nova_autopilot screenshot   # shows the 2 filtered out
