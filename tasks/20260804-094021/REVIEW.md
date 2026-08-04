# Review: Rebuild ui/ to drive real widgets with pointer input and assert the live tree

- TASK: 20260804-094021
- BRANCH: feature/ui-pointer-driven

## Round 1

- REVIEWER: out-of-context
- VERDICT: REQUEST_CHANGES

- [x] R1.1 (MAJOR) examples/ui/menu_scenarios.rs:199-219 - the pane-width
  verdict can now go vacuously green. The walk pushes a measurement for every
  row it ATTEMPTED, and nothing checks the click landed: `click_named`
  warns-and-continues, and a row that is scrolled out, occluded or hit-tested
  to another node leaves both panes untouched, so `report()` prints
  "widths HELD" having proven nothing. The `world.trigger(Activate)` this diff
  replaced could not miss - the strength is what regressed, not the coverage.
  Before `state.measured.push(...)`, assert the row named `last` now carries
  `nova_ui::widget::Selected` (inserted by `select_scenario_row`,
  `crates/nova_menu/src/scenarios.rs:596`) and panic under `NOVA_AUTOPILOT`
  when it does not, the same way `report()` already panics on zero
  measurements.
  - Response: fixed, and the finding was right about more than it claimed. The
    walk now records the row it clicked in `pending_measure` (set only when the
    gesture went out) and `assert_selection_landed` requires that row to carry
    `nova_ui::widget::Selected` before its widths count - `panic` under
    `NOVA_AUTOPILOT`, `warn` interactively, matching `report()`.
    On its first run the new assert FIRED, three times out of three: with the
    `webmods/gauntlet` mod and the `ledger_*` scenarios installed the picker
    lists 13 rows against a viewport of y 134..631, and
    `Scenario Row: asteroid_field` lays out at y 1214..1271 - below the fold,
    laid out, and happily resolved by `ui_node_centre` to a coordinate pointing
    at something else. So the walk was measuring the PREVIOUS selection exactly
    as predicted; the only reason nobody saw it is that the previous widths
    matched. Rows are now filtered by `row_is_on_screen` (the row's centre must
    lie inside the `Scenarios List` rect) and skipped with a warn, since the
    picker does not scroll under the harness. Post-fix: 6 consecutive clean
    `menu_scenarios` runs plus a clean `probe run ui` and a clean
    `ui_reach_playing_without_panic`.
- [x] R1.2 (MAJOR) tasks/20260804-094021/TASK.md:277-281 - DoD proof 1 is not
  reliably green, and the close-out records "all five OK" unqualified.
  `menu_scenarios` fails `process_exit` intermittently: a non-zero exit AFTER
  `probe: script complete, exiting` / `harness completion: all collectors done,
  exiting`, with `run_completed`, `reached_playing` and `log_clean` all PASS and
  no panic or ERROR in `run.log`. Reproduced in this review at 2 failures in 6
  runs (the out-of-context reviewer: 1 of 2 `probe run ui`; the recording pass:
  1 of 3 `probe run menu_scenarios`, plus 1 clean `probe run ui`). Attribute it
  - run `probe run menu_scenarios` on master - then either fix it here or state
  in the close-out that it predates the branch. Either way the flake must be
  recorded, not omitted from the evidence.
  - Response: attributed, and the attribution came out DIFFERENT from the
    finding's. The branch half was not a flake at all: it was the R1.1 fold bug
    panicking (`exit 101`, `thread 'main' panicked at menu_scenarios.rs:292`,
    reproduced 3 of 3 directly and 3 of 3 through the sequential smoke, against
    2 of 2 clean on master). It read as a mystery exit code because
    `run.log` DOES NOT CAPTURE PANICS - `grep -c panicked` over the failing
    `probe-runs/*/menu_scenarios/run.log` is 0, which is why probe reported
    `process_exit FAIL` with `log_clean PASS`.
    Fixed here. The master half (1 of 6) is untouched by this branch and stays
    filed as `20260804-174231`, now carrying the panic-capture lead and a
    second bug it exposes: a harnessed run can panic and still report
    `log_clean PASS`. The close-out evidence is rewritten to say all of this
    rather than "all five OK" unqualified.
- [x] R1.3 (MINOR) web/src/wiki/dev/development.md:174 and CHANGELOG.md:81-84 -
  both claim all five `ui/` runs DRIVE the interface with synthesized pointer
  input. `hud_range` clicks no widget; its only synthesized pointer call is
  `press_mouse(MouseButton::Right)` as a flight gesture
  (`examples/ui/hud_range.rs:381`). Reword both to "four of the five drive the
  interface by pointer; `hud_range` stays predicate-driven over its
  screen-projected indicators".
  - Response: fixed. Both surfaces now say four of the five drive by pointer and
    name `hud_range` as the predicate-driven one, with its reason (its subject
    is where an indicator lands on screen, not what a pointer does to it).
- [x] R1.4 (MINOR) tasks/20260804-094021/TASK.md:279 - the recorded
  "widget_zoo 89s" matches no rig. Three `probe run ui` passes measured
  widget_zoo at 8s, 9s and 9s; every other number on that line (editor,
  hud_range, menu_newgame, menu_scenarios) reproduces. Correct it to the
  measured value.
  - Response: fixed - 9s, which is what every run since has measured (8s, 8s, 9s
    across this pass).
- [x] R1.5 (MINOR) crates/nova_autopilot/src/input.rs:112 - `ui_node_centre`
  uses `find`, so two nodes sharing a `Name` silently resolve to whichever the
  query yields first and the click lands on a ghost with the run still green -
  the exact failure `widget_zoo`'s local `named()`
  (`examples/ui/widget_zoo.rs:868`) panics on, and the failure mode this whole
  task exists to make visible. Collect the matches and `warn!` when more than
  one node carries the name before returning the first.
  - Response: fixed. `ui_node_centre` collects the matches and `warn!`s the
    count when more than one laid-out node carries the name, then returns the
    first. Covered by `a_duplicated_name_resolves_to_one_of_them`, which pins
    that the resolve still lands on one of the two rather than vanishing.
- [x] R1.6 (MINOR) examples/ui/editor.rs:328 - `place_a_section` is the label
  on two beats that place nothing ("click the ship in select mode",
  "click the ship in delete mode"). Rename the trait method to
  `click_the_ship`, matching the trait's own name.
  - Response: fixed - the trait method is `click_the_ship` at all five call
    sites, with a doc line saying it is named for the gesture because the same
    three beats also drive the select-mode and delete-mode clicks.
- [x] R1.7 (NIT) examples/ui/menu_scenarios.rs:189,226,243 - each site resolves
  the node twice (`ui_node_centre(...).is_some()`, then `click_named(...)`
  resolves it again). Bind the centre once and click at it.
  - Response: fixed - all three sites bind the centre from `ui_node_centre` and
    call `click_at`.

Verification, recording pass (all in the worktree, `DISPLAY=:99`):

- Proofs, run independently of the reviewer: `! rg -n 'world.trigger'
  examples/ui` green; `! ls examples/ui/*.html` green (five `.rs` only);
  `cargo test -p nova_autopilot --lib input::` 10 passed; `cargo test -p
  nova_gameplay --lib rtt_element_renders_its_subtree` passed; `cargo test
  --test examples_smoke catalog_matches_disk` passed; `cargo test --test
  examples_smoke ui_reach_playing_without_panic` passed in 68.7s including
  widget_zoo; `cargo run -p nova_probe -- run ui` all five OK 5/6 (fps SKIPPED
  by design). `cargo fmt --all -- --check` and `cargo check --examples --tests
  --features debug` clean.
- The verdict lines are real, not vacuous, in the run logs: zoo hover
  0.05 -> 0.12, pressed 0.12 -> 0.2, skin Hardware, level Minimal, checks
  `[false, false, false, false]`, slider 0.5 -> 0.7083333; editor tooltip names
  `Reinforced Hull Section`, placed 2 (1 -> 3), select inert (3), deleted
  (3 -> 2); menu_newgame teardown; scenarios HELD across 6 real row clicks.
- Re-derived independently: `UiGlobalTransform`'s translation IS physical px.
  `bevy_ui-0.19.0/src/picking_backend.rs:133` multiplies the pointer's logical
  position by `camera.target_scaling_factor()` before comparing it against the
  node transform, so `ui_node_centre`'s `inverse_scale_factor` conversion is
  the correct direction and `a_named_node_resolves_to_logical_pixels_on_a_scaled_window`
  pins the right claim. DECISION.md D2's sizing note
  ("`GlobalTransform.translation().xy()` ... already logical") is the claim that
  was wrong; the code shipped is right. Not raised as a finding - `tasks/` is
  append-only history.
- No existing test was deleted or weakened: the only removed functions are the
  three `button_by_name` copies and the two bespoke autopilot state machines
  the task set out to consolidate, and no `assert` line is removed anywhere
  under `crates/` or `tests/`.
- Doc sweep: `NOVA_MENU_PATH` survives only in the CHANGELOG entry announcing
  its removal and in the example doc-comment explaining it; `nova harness:
  reached Playing` remains only as literal prose in "look for" doc comments,
  with all three code callers on `REACHED_PLAYING`.
- Process signal: the diff carried a compile break inherited from a prior
  context (`editor.rs` with an unbalanced `.add())` and three names that do not
  resolve, recorded in the task's Difficulties). A checkpoint commit that does
  not build is what the checkpoint rule exists to prevent.
- Process signal: two Step clauses were written against an editor that does not
  exist (no selection surface; `SectionChoice` is `pub(crate)`) and were
  corrected against the code rather than faked. The plan asserted an internal
  API an example cannot name - worth a planning check that a Step's assertion
  target is reachable from where the Step runs.
- No `manual:` proofs in this task.

## Round 2

- REVIEWER: out-of-context
- VERDICT: REQUEST_CHANGES

All seven round-1 findings are confirmed fixed and ticked: R1.1's
`assert_selection_landed` really does require the clicked row to carry
`nova_ui::widget::Selected`, R1.2's attribution and the master-side filing
(`20260804-174231`) check out, R1.3/R1.4's doc and timing corrections are in,
R1.5's `ui_node_centre` collects and warns, R1.6's `click_the_ship` is at the
trait and all five call sites, and R1.7's three sites bind the centre once.
Round 2's findings are all regressions or gaps that arrived WITH those fixes.

- [x] R2.1 (MAJOR) examples/ui/menu_scenarios.rs:156 - `row_is_on_screen`
  misfires on the FIRST row the walk attempts, so a row that is plainly in view
  is silently dropped and the run's coverage varies. Measured in this review,
  both environments, `DISPLAY=:99`: under `probe run ui` the picker lists SIX
  rows (`probe-runs/edcb6dca/menu_scenarios/run.log`: 1 fold warn + 5
  `pane widths:` lines), and `Scenario Row: asteroid_field` - the first name in
  the sorted walk order - is skipped as "below the fold" anyway. Six rows of
  ~57 px cannot overflow a list whose viewport is ~497 px tall, so that skip is
  not a fold; it is `node_rect` being consulted before the reopened list has
  settled (`row_is_on_screen` returns false when EITHER rect is missing, and a
  missing `Scenarios List` rect is indistinguishable from an off-screen row).
  The out-of-context reviewer measured the complementary case on the same tree:
  one invocation measured 6, the next 5. Fix at menu_scenarios.rs:283-287: do
  not push an off-screen row to `visited` on first sight - leave it unvisited
  and retry it for a bounded number of driven frames (a per-row settle counter)
  before skipping, and make the skip distinguish "no list rect yet" from
  "row centre outside the list rect" so the warn names which one it is.
  - Response: remedy fixed, premise DISPUTED. Implemented as asked: a per-row
    `ROW_SETTLE_FRAMES` (10) budget, the row left unvisited until it is spent,
    and a three-way `RowPlacement` whose warn names which case fired. The
    premise that the skip "is not a fold" is falsified by the rects that fix
    made printable - in the PROBE environment, `DISPLAY=:99`, on all 10 settle
    frames identically: `Scenarios List` y 134..631, `Scenario Row:
    asteroid_field` y 659..716. The row is 28 px below the box, stably, after a
    full settle budget, so its skip is correct. The `~57 px` arithmetic assumed
    uniform rows; all six were measured and they are not. shakedown_run
    174..245, broadside 245..345, broadside_gunship 345..445, lifeline
    445..545, final_tally 545..659, asteroid_field 659..716 - 71..114 px each,
    542 px of content in a 497 px box, because a campaign-member row carries a
    blurb line under its name. What the finding got right is the VARIANCE - a
    single-frame look at a
    just-rebuilt list could not tell "no rect yet" from "past the fold" - and
    that is now gone: 5 measured, 1 fold skip, on every run since.
- [x] R2.2 (MAJOR) tasks/20260804-094021/TASK.md:328-331 - the close-out
  explains the 5-of-6 count as "the eight mod-supplied rows push
  `asteroid_field` past the list's fold" and asserts that "on a tree without the
  `webmods/gauntlet` and `ledger_*` scenarios the whole shipped set is in view
  and the walk measures 6". The second half is falsified by this branch's own
  probe run: that environment does NOT list the mod rows (six rows total) and
  the walk still measures 5, skipping `asteroid_field`. The 13-row figure is
  real - three direct `cargo run --example menu_scenarios` invocations in this
  review each listed 13 and skipped 8 - but it is the DIRECT-run environment,
  not the one DoD proof 1 runs in, and it is not what makes the count 5.
  Rewrite the paragraph to state both environments and their measured counts,
  or fix R2.1 and record the stable number that results.
  - Response: fixed. The close-out paragraph now says the mod-free-tree-measures-6
    claim is WRONG, and a new "Why `asteroid_field` is skipped - measured, not
    reasoned" section records the probe-environment rects above and the stable
    5-of-6. The round-1 13-row direct-run figure is marked as standing but not
    re-measured this round, rather than restated as if it were.
- [x] R2.3 (MAJOR) examples/ui/menu_scenarios.rs:372 - `report()` has no
  coverage FLOOR, so R2.1's skip path can turn the verdict vacuous from the
  other side. Its only guard is `measured.first()`; with one surviving
  measurement both spreads are `max - min` over a single value, i.e. 0, so the
  `<= 0.5` branch prints `widths HELD` for a split that is only testable ACROSS
  selections. Re-derived by reading `report()` directly, not inferred from
  R2.1. Today's counts (5 of 6, 5 of 13) leave margin, but nothing in the code
  keeps them there - the skip is unbounded. Add a floor beside the existing
  zero-measurement panic: under `NOVA_AUTOPILOT`, `panic!` when
  `state.measured.len() < 2`, naming the count and the skipped rows.
  - Response: fixed exactly as specified - `report()` panics under
    `NOVA_AUTOPILOT` below two measurements, naming the count and every skipped
    row with its reason (the new `skipped` field carries the reason). Went one
    step further on the finding's own logic: the coverage string is on the HELD
    and CHANGED verdicts too, so a PASSING run states what it covered instead of
    leaving that to a warn buried mid-log. Sabotage-proven: with the floor
    raised to `< 6` against a run that measures 5, the harnessed run exits 101
    naming all 8 skipped rows.
- [x] R2.4 (MINOR) crates/nova_autopilot/src/input.rs:373 -
  `a_duplicated_name_resolves_to_one_of_them` passes unchanged if R1.5's fix is
  reverted to the original `find`: it asserts only that the resolved centre is
  one of the two nodes, never that the duplicate was DETECTED. The warn is the
  whole point of the finding and nothing pins it, so R1.5's "Covered by" and the
  test's own doc line "the resolve must not pick silently" both overclaim.
  Install a `tracing` capture layer in the test and assert the "laid-out UI
  nodes are named" line is emitted, so the test fails with the fix deleted.
  - Response: fixed, and the overclaim is withdrawn rather than reworded. The
    test is now `a_duplicated_name_warns_and_resolves_to_one_of_them` and
    asserts the warn is emitted ONCE, naming the count, through a captured
    subscriber. The capture helper was not written a second time: `LogBuf` /
    `capturing_logs` already existed in `completion.rs`'s test module and is
    lifted into `crates/nova_autopilot/src/log_capture.rs`, a `#[cfg(test)]`
    module both use. A companion test pins that a UNIQUE name resolves
    SILENTLY, so the warn cannot degrade into firing on every resolve.
    Sabotage-proven: delete the warn block and the test fails (exit 101).
- [x] R2.5 (MINOR) tasks/20260804-094021/TASK.md:221,332 - two counts in the
  close-out went stale when R1.5 added a sixth test: the summary says "Four unit
  tests" where `git diff master...HEAD -- crates/nova_autopilot/src/input.rs`
  adds six (`a_named_ui_node_resolves_to_its_centre`,
  `a_duplicated_name_resolves_to_one_of_them`, `click_named_lands_on_the_named_node`,
  `hover_named_positions_without_pressing`, `a_click_on_an_absent_name_is_harmless`,
  `a_named_node_resolves_to_logical_pixels_on_a_scaled_window`), and the evidence
  line records "10 passed" where the filter now reports 11. Correct both to the
  shipped numbers.
  - Response: fixed, to the numbers as they now ship rather than as they were
    when the finding was written (this round added two more tests and renamed
    one). The summary says eight unit tests; the evidence line says 13 for the
    `input::` filter, measured this round.
- [x] R2.6 (MINOR) examples/ui/menu_scenarios.rs:4 - the module doc still says
  the run "selects every listed scenario row in turn", which R1.1's fix made
  false: rows outside the list's visible box are now skipped with a warn.
  Reword to say it selects every row inside the list's visible box, and why
  (the picker does not scroll under the harness).
  - Response: fixed, with the coverage floor named too - the doc now says the
    run fails below two reached rows, so a reader learns the guarantee and not
    only the skip.
- [x] R2.7 (NIT) examples/ui/menu_scenarios.rs:136 - `node_rect` is the third
  copy of the physical-to-logical conversion (`width_by_name:126`,
  `nova_autopilot::input::ui_node_centre:118`), and `width_by_name` is now
  exactly `node_rect(..).width()`. Build the rect from
  `ui_node_centre(world, name)` plus a size lookup and derive both pane widths
  from it, so the conversion lives in one place.
  - Response: fixed, one level deeper than asked. Rather than rebuild a rect in
    the example, `nova_autopilot::input::ui_node_rect` IS the primitive and
    `ui_node_centre` is now `ui_node_rect(..).center()`; the conversion exists
    once, in the crate. Both `width_by_name` and `node_rect` are deleted from
    the example and both pane widths are read off the shared rect. Exported
    from the `nova_autopilot` and `nova_debug` preludes, and recorded in the
    CHANGELOG and `development.md` beside the rest of the vocabulary.
- [x] R2.8 (NIT) examples/ui/menu_scenarios.rs:250 - if either width read
  returns `None`, `pending_measure` is left set, control falls through to the
  next-row click and that click overwrites it: the measurement is dropped with
  no log at all. Add a `warn!` on the `else` of the width read.
  - Response: fixed - the `else` warns, naming the row and which of the two
    panes failed to lay out.

Verification, recording pass (worktree, `DISPLAY=:99`, run independently of the
out-of-context reviewer):

- Proofs: `! rg -n 'world.trigger' examples/ui` green; `! ls examples/ui/*.html`
  green (five `.rs` only); `cargo test -p nova_autopilot --lib input::` 11
  passed; `cargo test -p nova_gameplay --lib rtt_element_renders_its_subtree`
  passed; `cargo test --test examples_smoke catalog_matches_disk` passed;
  `cargo test --test examples_smoke ui_reach_playing_without_panic` passed
  twice, 49.9 s and 55.15 s, with `widget_zoo` in the list. `cargo fmt --all --
  --check` and `cargo check --examples --tests --features debug` clean.
- DoD proof 1, `cargo run -p nova_probe -- run ui`, went green on one
  invocation (all five OK 5/6, fps SKIPPED by design - widget_zoo 8s, editor
  14s, hud_range 10s, menu_newgame 8s, menu_scenarios 10s) and RED on another.
  The red one is recorded rather than re-rolled away: `menu_scenarios ERROR` on
  `timeline.jsonl: malformed timeline line 143`. Attributed, not waved through.
  The RUN was clean - `probe: script complete, exiting`, `autopilot: cycle
  complete, no panic`, `harness completion: all collectors done, exiting`, and
  zero `panicked` or `ERROR` lines in `run.log`. The failure is a TORN
  `timeline.jsonl`: line 143 splices a `frame 168` record at `t_real 5.08` into
  a `frame 168` record at `t_real 4.79`, i.e. two `ProbeTimeline` writers on one
  path, the second `File::create`ing from offset 0 while the first's
  `BufWriter` keeps writing at its own offset
  (`crates/nova_probe/src/recorder.rs:200`). That file is not in this diff -
  `git diff --name-only master...HEAD` touches no `nova_probe` source - and
  afterwards the branch reproduced clean 3 of 3 against master clean 3 of 3, too
  rare to attribute to either tree from six runs and in a component the branch
  does not modify. Recorded on `20260804-174231`, which already owns
  "`menu_scenarios` fails a probe check on an otherwise clean run", as a second
  symptom. Not a finding against this diff, but it does mean proof 1 is not
  reliably green and the close-out must keep saying so.
- Load-bearing claims re-derived rather than accepted. First: an out-of-context
  reviewer reported R2.1 as a pure layout race and called the 13-row figure
  false. Three direct `cargo run --example menu_scenarios --features debug` runs
  here listed 13 rows and skipped 8 every time, deterministically - so the
  13-row diagnosis in the close-out is REAL and the flat contradiction of it is
  wrong. What survives is narrower and is what R2.1 records: the probe
  environment lists six rows, six rows fit, and `asteroid_field` is skipped
  there anyway. R2.2 is reworded to that evidence instead. Second: R2.3, read
  straight out of `report()` - `measured.first()` is the ONLY floor - and R2.4,
  by reading the duplicate-name test against the pre-fix `find`, which satisfies
  its assertion identically.
- No existing test was weakened or deleted by the round-1 fixes: the fix commit
  (`d4595dd9`) adds `a_duplicated_name_resolves_to_one_of_them` and removes no
  `assert` anywhere under `crates/`, `tests/` or `examples/`.
- Doc sweep on the round-1 renames: `place_a_section`, `button_by_name` and
  `send_pointer` survive nowhere outside `tasks/`; `NOVA_MENU_PATH` survives
  only in the CHANGELOG entry announcing its removal and the example
  doc-comment explaining it.
- Round 2 adds findings only for fix regressions, per the round rules. Every one
  of R2.1 to R2.8 is a property of an R1 fix or its collateral in the file that
  fix landed in; no unchanged part of the diff was reopened for a new opinion.
- Round-1 status verified against the code, not the Responses: R1.1, R1.2, R1.3,
  R1.4, R1.6 and R1.7 are confirmed fixed and their boxes are ticked in Round 1.
  R1.5 is PARTIAL - `ui_node_centre` does collect and warn, but the test cited as
  covering it does not - so its box stays open and it is carried as R2.4.
- Process signal: R1.1's fix traded a vacuous-green failure for a
  silent-coverage-loss failure, and both were invisible because the guard only
  `warn!`s. A driven run that cannot reach a target it was told to reach is the
  same class of defect as one that measures the wrong thing - the skip path
  deserves the same panic-under-`NOVA_AUTOPILOT` treatment the assert path got,
  once a settle retry has genuinely been exhausted. R2.3 is the same lesson
  again: a hardening fix that adds a SKIP wants its coverage floor raised in the
  same change.
- Process signal: the branch's evidence was gathered in the direct-run
  environment (13 scenarios, mods installed) while DoD proof 1 runs in a
  six-row one. Close-out numbers should name the environment that produced
  them.
- No `manual:` proofs in this task.

## Round 3

- REVIEWER: out-of-context
- VERDICT: REQUEST_CHANGES

All eight round-2 findings are confirmed fixed and ticked, and R1.5 - carried
open as R2.4 - closes with them. R2.1's PUSHBACK is accepted: the skipped row
is a genuine fold, not a settle race. Round 3's findings are regressions and
gaps that arrived WITH the round-2 fixes.

- [x] R3.1 (MAJOR) crates/nova_autopilot/src/input.rs:126 - R2.7's new size
  conversion is untested, which is the exact defect class R2.4 was raised for,
  now sitting on the primitive round 2 introduced.
  `a_named_node_resolves_to_its_logical_rect` (`input.rs:432`) spawns through
  `spawn_named_node` at scale factor 1, where `computed.size() * scale` is the
  identity, and the only scaled test
  (`a_named_node_resolves_to_logical_pixels_on_a_scaled_window:537`) asserts
  the CENTRE alone. Sabotage-confirmed in this pass: delete `* scale` from the
  size term of `Rect::from_center_size` in `ui_node_rect` and all 13 `input::`
  tests still pass. It is load-bearing - `menu_scenarios` reads both pane
  widths off `rect.width()` and decides the fold with
  `list.contains(row.center())`, so on a scale-2 display the widths and the
  list box both double, silently moving the fold and the recorded verdict
  numbers - and the test's own doc ("it converts the node's SIZE as well as
  its centre") claims coverage it does not have. Spawn the rect test through
  `spawn_named_node_at_scale(&mut app, "Scenarios List", Vec2::new(400.0,
  120.0), 2.0)` and keep `assert_eq!(rect.size(), NODE_SIZE)`, so only the
  converted read passes.
  - Response: fixed as specified, and re-proven both ways.
    `a_named_node_resolves_to_its_logical_rect` now spawns through
    `spawn_named_node_at_scale(.., 2.0)` and keeps `assert_eq!(rect.size(),
    NODE_SIZE)`, so the node lays out at `NODE_SIZE * 2` physical and only the
    converted read passes. Sabotage: with `* scale` cut from the size term of
    `Rect::from_center_size`, that test FAILS (12 passed, 1 failed, exit 101);
    restored, 13 pass. Before the fix the same sabotage left all 13 green.
- [x] R3.2 (MINOR) examples/ui/menu_scenarios.rs:391 - R2.3's coverage string
  can misreport R2.8's drop path. A selection whose panes fail to lay out is
  warned about at `:283-289` and then lost when the next click overwrites
  `pending_measure`, but that row lands in neither `measured` nor `skipped`
  (the only `skipped.push` is at `:319`), so `report()` prints "4 rows, none
  skipped" for a picker that listed five. A verdict stating a coverage it did
  not have is the very thing R2.3's string exists to prevent. Push the dropped
  row into `state.skipped` with a "panes never laid out" reason in that `else`
  branch, so the count, the named list and the floor at `:407` all see it.
  - Response: fixed, with one addition the finding did not name.
    The `else` pushes `(last, "its pane widths never laid out")` onto
    `state.skipped`, so the coverage string, the named list and the `< 2` floor
    all account for the row. `pending_measure` is CLEARED with it rather than
    left for the next click to overwrite: the `else` runs every frame while it
    is set, and the next row is not always clicked on the next frame (a row
    inside its settle budget returns early), so leaving it set would push the
    same row onto `skipped` once per frame. Clearing it records the row
    exactly once.
- [x] R3.3 (NIT) crates/nova_autopilot/src/input.rs:508 - round 2 inserted
  `NODE_SIZE` between `spawn_named_node_at_scale`'s doc comment and the
  function, so that doc ("A UI node as the resolve sees it...") now documents
  the const and runs on into the const's own doc, while the helper is left
  undocumented. Move `const NODE_SIZE` and its three doc lines above
  `:508`.
  - Response: fixed - `NODE_SIZE` and its three doc lines now sit above the
    helper, and "A UI node as the resolve sees it..." documents
    `spawn_named_node_at_scale` again.

Verification, recording pass (worktree, `DISPLAY=:99`, run independently of the
out-of-context reviewer):

- Proofs: `cargo run -p nova_probe -- run ui` OK on the first invocation, all
  five 5/6 (fps SKIPPED by design) - widget_zoo 9s, editor 10s, hud_range 10s,
  menu_newgame 7s, menu_scenarios 9s; `! rg -n 'world.trigger' examples/ui`
  green; `! ls examples/ui/*.html` green; `cargo test -p nova_autopilot --lib`
  38 passed (`input::` 13); `cargo test -p nova_gameplay --lib
  rtt_element_renders_its_subtree` passed; `cargo test --test examples_smoke
  catalog_matches_disk` passed; `cargo test --test examples_smoke
  ui_reach_playing_without_panic` passed, 56.7 s. `cargo fmt --all -- --check`
  and `cargo check --examples --tests --features debug` clean.
- R3.1 re-derived by SABOTAGE rather than from the reviewer's reading: with
  `computed.size() * scale` cut to `computed.size()` in `ui_node_rect`,
  `cargo test -p nova_autopilot --lib input::` still reports 13 passed, exit 0.
  Restored via `git checkout HEAD --`.
- R2.1's pushback independently checked before accepting it. The rects were
  re-measured in the probe environment with a temporary `warn!` in
  `row_placement`: `Scenarios List` y 134..631 (497 px) against six rows -
  shakedown_run 174..245, broadside 245..345, broadside_gunship 345..445,
  lifeline 445..545, final_tally 545..659, asteroid_field 659..716 - i.e. 542
  px of content in a 497 px box, rows 71..114 px rather than the uniform ~57
  px R2.1's arithmetic assumed. `asteroid_field` is 28 px below the box after
  the full settle budget, and the logged reason is "the row's centre is outside
  the list's box", not "has not laid out yet". The skip is a fold. R2.1's
  remedy is still right and is in, and the count is now stable: 5 measured, 1
  skipped, on 4 consecutive probe runs.
- R2.3's floor was sabotage-checked too, independently of the implementer's
  own: truncating `scenario_row_names` to one row makes the harnessed run exit
  101 on `scenarios pane widths: TOO FEW measurements`, naming the skipped row
  and its reason.
- No existing test was weakened or deleted by the round-2 fixes: the only
  removals under `crates/`, `tests/` or `examples/` are `width_by_name`,
  `node_rect` and `row_is_on_screen`, all folded into `ui_node_rect` /
  `row_placement`, and the `LogBuf` helper moved out of `completion.rs`'s test
  module rather than copied.
- Doc sweep on round 2's rename: `width_by_name`, `node_rect` and
  `row_is_on_screen` survive nowhere outside `tasks/`. `ui_node_rect` is in
  both preludes, the CHANGELOG entry and the `ui/` bullet in
  `web/src/wiki/dev/development.md`.
- Process signal: the picker DOES support wheel scroll (`scroll_menu_lists`,
  `crates/nova_ui/src/widgets.rs:72`), so the permanent 5-of-6 coverage is a
  HARNESS gap - `nova_autopilot::input` synthesizes no wheel - and not a
  property of the UI. A row past the fold is unreachable only because the
  vocabulary cannot scroll to it. Worth a follow-up task, not this branch.
- Process signal: R2.7 moved a conversion into a shared crate primitive and
  the round shipped it with the same untested-warn shape R2.4 had just been
  raised for. Consolidating logic into a shared home raises the bar on its
  tests, because every caller now inherits the gap.
- No `manual:` proofs in this task.

## Round 4

- REVIEWER: out-of-context
- VERDICT: APPROVE

All three round-3 findings are confirmed fixed and ticked above. Round 4's two
findings are narrowings that arrived WITH the R3.2 fix; both are below the
blocking bar, so the branch is approved with them open.

- [ ] R4.1 (MINOR) examples/ui/menu_scenarios.rs:295-301 - the R3.2 fix removes
  the only retry path for a measurement, so a one-frame layout miss is now
  permanent. Before it, `pending_measure` survived a failed measure, and while
  the next row was still settling the `else` re-ran on later frames and could
  still measure the panes. The walk is now asymmetric with itself: a ROW gets
  `ROW_SETTLE_FRAMES` (10) driven frames to lay out precisely because a
  post-selection rebuild leaves the list without a rect (R2.1), while a
  MEASUREMENT gets exactly one look at the same rebuild race. Give the
  measurement the same budget: count consecutive failed measure frames for
  `last` the way `state.settling` counts them for a row, and only
  `skipped.push(..)` + clear `pending_measure` once it has failed for
  `ROW_SETTLE_FRAMES`. That keeps R3.2's property - accounted exactly once,
  never once per frame - without shrinking coverage on a slow frame.
  - Response:
- [ ] R4.2 (NIT) examples/ui/menu_scenarios.rs:288-294 - the `warn!` in that
  branch still ends "dropping that measurement", which the same commit made
  false: the row is now recorded as skipped and named in the verdict's
  coverage string. Reword the tail to "recording it as skipped" so the log and
  the state agree.
  - Response:

Verification, recording pass (worktree at `7ff4b0e7`, `DISPLAY=:99`, run
independently of the out-of-context reviewer):

- Proofs, all six: `cargo run -p nova_probe -- run ui` aggregate OK on the
  first invocation, all five 5/6 (fps SKIPPED by design) - widget_zoo 9s,
  editor 10s, hud_range 10s, menu_newgame 6s, menu_scenarios 9s;
  `! rg -n 'world.trigger' examples/ui` green; `! ls examples/ui/*.html`
  green; `cargo test --test examples_smoke catalog_matches_disk` passed;
  `cargo test -p nova_gameplay --lib rtt_element_renders_its_subtree` passed;
  `cargo test -p nova_autopilot --lib` 38 passed. `cargo fmt --all -- --check`
  and `cargo check --examples --tests --features debug` clean.
- R3.2 re-derived here rather than read off the Response or the reviewer: the
  measure block precedes the click block and falls through to
  `insert_resource`, so an uncleared `pending_measure` would re-enter the
  `else` every frame while the next row settles - the clear is load-bearing,
  not tidying. `state.skipped` is the same vector `report()` reads at `:396`
  for the coverage string and at `:413` for the `< 2` floor, and the row
  cannot be double-counted against the `RowPlacement::Unreached` skip because
  `visited` is pushed at click time and never re-offers it.
- R3.1 was sabotage-confirmed by the out-of-context reviewer (size term cut to
  `computed.size()` -> 12 passed, 1 failed, `left: Vec2(240.0, 80.0) right:
  Vec2(120.0, 40.0)` at `input.rs:444`; restored -> 13 passed), matching the
  Response's claim.
- One `cargo test -p nova_autopilot --lib` run in this pass reported 37
  passed / 1 failed on `a_named_node_resolves_to_its_logical_rect`. It
  overlapped the reviewer's sabotage of `input.rs` in the shared worktree, not
  a defect: the tree is clean at `7ff4b0e7` and three consecutive runs
  afterwards report 38 passed. Recorded because a red run that is dismissed
  without evidence is how a real flake gets buried.
- Process signal: round 3's wheel-scroll gap (`nova_autopilot::input`
  synthesizes no wheel, so the picker's 5-of-6 coverage is a harness limit and
  not a UI property) still has no follow-up task. It belongs in the retro's
  seeds, not on this branch.
- No `manual:` proofs in this task.
