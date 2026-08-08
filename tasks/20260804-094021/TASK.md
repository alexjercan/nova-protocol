# Rebuild ui/ to drive real widgets with pointer input and assert the live tree

- STATUS: CLOSED
- PRIORITY: 83
- TAGS: v0.10.0, content, examples, testing, ui

## Story

Rebuild `ui/` so the runs DRIVE the interface with synthesized pointer input
instead of asserting around it, and check the live tree afterwards so nothing
ghosts or duplicates on a state change.

Pointer synthesis (`click_at`, `move_cursor`, `press_mouse`) landed with the
predicate autopilot (`20260802-120025`); without it the `ui/` contract was
assertion-only.

## Steps

Ordered: the shared vocabulary first, then the five runs, then the test and the
docs. Every run step ends with the example RUN under Xvfb :99, not checked.

- [x] Add the `Name`-resolved pointer vocabulary to
      `crates/nova_autopilot/src/input.rs` (DECISION D2), exported from the
      crate prelude: `ui_node_centre(world, name) -> Option<Vec2>` (logical px
      centre from `GlobalTransform` + `ComputedNode`), `click_named(name)` and
      `hover_named(name)`, both warn-and-continue on a missing name the way
      `move_cursor` does without a window. Unit tests beside the existing ones
      in that module's `mod tests`: a named node resolves to its centre, a
      click on a name lands there, an absent name warns and does not panic.
- [x] Promote the smoke sentinel to `pub const REACHED_PLAYING: &str =
      "nova harness: reached Playing"` in `crates/nova_debug/src/harness.rs`,
      and name it from `crates/nova_debug/src/lib.rs:131`, `widget_zoo` and
      `tests/examples_smoke.rs`'s stderr grep (DECISION D3).
- [x] `widget_zoo` joins the fleet: `app.init_state::<GameStates>()` plus a
      `Startup` system setting `NextState` to `Playing`, a `#[cfg(feature =
      "debug")]` harness block matching its siblings (`nova_probe::nova_timeline
      / nova_invariants / nova_frametime`, all inert without their env, plus
      `nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>` named
      FULLY QUALIFIED - `examples_name_drivers_through_the_nova_harness` fails a
      bare name), and the `REACHED_PLAYING` log on
      `OnEnter(GameStates::Playing)` gated on `NOVA_AUTOPILOT`.
- [x] Give the driven `widget_zoo` widgets `Name`s (it has none today): the two
      Skin segmented options, the `DemoLevel` segmented options, the four
      `clickable` checks/toggles (`widget_zoo.rs:322-327`), the slider, and one
      hoverable button from the States row.
- [x] Drive `widget_zoo` as a step script, one beat per gesture:
      hover a button and assert its hover face; press it and assert the pressed
      face; click `Hardware` and assert `UiSkin` flipped; click a `DemoLevel`
      option and assert the resource; click one checkbox and one toggle and
      assert `ZooChecks`; drag the slider (`hover_named` -> `press_mouse` ->
      `move_cursor` along the track from `ui_node_centre` -> `release_mouse`)
      and assert `SliderValue` moved.
- [x] Assert `widget_zoo`'s LIVE TREE after the two rebuild-triggering beats
      (reskin and check flip - `rebuild_body` despawns and respawns the body,
      `widget_zoo.rs:194-231`): exactly one `ZooBody`, exactly one entity per
      driven `Name`, and no `TextShadow` anywhere under the root (nova_ui
      refuses it on purpose - `widget/button.rs:654`).
- [x] Deepen `examples/ui/editor.rs` into build-and-inspect: create the ship,
      click a hull section CARD by name (replacing the
      `world.entity_mut(button).insert(Pressed)` shortcut at
      `editor.rs:~170`), place TWO sections through the real pointer, then
      inspect - click `Select Section Button`, click a placed section, assert
      the selection/tooltip surface names it - then `Delete Section Button` and
      assert the count drops back. Delete the bespoke `send_pointer` /
      `PointerInput` synthesis (`editor.rs:~260-281`) in favour of
      `move_cursor` / `press_mouse` / `release_mouse`, which write the same
      `WindowEvent` the picking backend reads.
- [x] Narrow `examples/ui/menu_newgame.rs` to the boot flow only: `click_named`
      on the menu button, advance until `GameStates::Playing`, assert nothing
      about `shakedown_run`'s contents. Decide and record: keep or drop the
      `NOVA_MENU_PATH=editorplay` branch, now that `editor` owns the
      create-ship-and-Play sequence - dropping it is the default, since two runs
      covering one transition is the duplication the roster spike cut.
- [x] Deepen `examples/ui/menu_scenarios.rs`: replace both
      `world.trigger(Activate { .. })` calls (rows and the Play button) with
      `click_named`, keep the pane-width measure and its `panic`-on-CHANGED
      verdict exactly as they are, and keep the self-ending completion.
- [x] Add `rtt_element_renders_its_subtree` to
      `crates/nova_gameplay/src/hud/nova_os/tests/crt.rs` - the ONE POC claim no
      test makes today (DECISION D1): a `NovaOsImageCameraMarker` camera exists,
      its `RenderTarget` is the `NovaOsRtt` image, it sits on
      `RenderLayers::layer(NOVA_OS_RTT_LAYER)` at `NOVA_OS_RTT_CAMERA_ORDER`,
      and the content root's subtree is on that same layer and non-empty - i.e.
      the element displays a subtree that is actually rendered offscreen, not an
      empty target.
- [x] Move `widget_zoo` from `NOT_SMOKED` to `UI` in `tests/examples_smoke.rs`
      (`:47`, `:74-82`), rewriting the comment to say what "reached Playing"
      means for a widget showcase.
- [x] Update `web/src/wiki/dev/development.md`'s `ui/` bullet (`:170-173`) for
      the five-run roster and the pointer idiom, and add a CHANGELOG entry.

## Definition of Done

- The `ui/` fleet drives real widgets with synthesized pointer input and
  completes headlessly, `widget_zoo` included.
  (cmd: `nix develop --command cargo run -p nova_probe -- run ui`)
- No `ui/` run reaches a widget by triggering its observer or inserting its
  state component instead of pointing at it.
  (cmd: `! rg -n 'world.trigger' examples/ui`)
- `widget_zoo` is under CI smoke with the rest of the category.
  (test: `ui_reach_playing_without_panic`)
- Disk, catalog and smoke lists still agree after the move out of `NOT_SMOKED`.
  (test: `catalog_matches_disk`)
- The RTT pipeline keeps element-level coverage after the POC retires.
  (test: `rtt_element_renders_its_subtree`)
- `examples/ui/` holds only runnable examples.
  (cmd: `! ls examples/ui/*.html`)

## Notes

Roster per the spike (`20260804-003244`) - five runs, `nova_os_rtt_poc` retired
by `20260804-093910`:

| Run | Change |
| --- | --- |
| `widget_zoo` | Drive it. Already functional; gains real pointer input (hover, press, reskin, segmented select, checkbox/toggle, slider drag) and live-tree assertions. |
| `hud_range` | KEEP. Already predicate-driven; screen-projected indicators. |
| `editor` | Deepen. Today one editor action; needs a real build-and-inspect sequence. |
| `menu_newgame` | Narrow. Assert only that gameplay state is reached - NOT `shakedown_run`'s content, which is story. |
| `menu_scenarios` | Deepen. Picker navigation plus the pane-width verdict, driven by pointer. |

- `widget_zoo` sits in `NOT_SMOKED` (`tests/examples_smoke.rs:75-78`) because it
  runs its own `App` (`widget_zoo.rs:32-62`: `App::new()` + `DefaultPlugins` +
  `nova_ui::widget::register`, no `AppBuilder`) with NO `GameStates` at all.
  RESOLVED 2026-08-04, owner call: add `GameStates` to the zoo (option A).
  Simple and local. The rejected option B - teaching `nova_autopilot` to drive
  a stateless app - is the more general fix, but nothing else needs it and it
  would change the crate the whole sprint depends on. If a second stateless app
  ever needs driving, B becomes worth its own task.
- Adding `GameStates` means the zoo joins the CI smoke run: 533 lines of
  interactive app asserted to reach Playing and exit clean, on every `cargo
  test` with a display. Expect to find things; budget for it.
- `widget_zoo`'s `NOVA_ZOO_CAPTURE=1` two-skin capture path is a screenshot
  producer living in a `ui/` example. Per the category contract that is a
  `screenshots/` job, and `20260804-093855`'s contract test may reject it.
  Not in this task's Steps - flag it if the contract test fires.
- The `*_poc.html` relocation is owned by epic child `20260804-003301`, not by
  the retire task. This task depends on it for its "only runnable examples"
  end-state.
- `menu_newgame` boots `shakedown_run`, a story scenario. That is NOT story
  coupling as long as it asserts reaching gameplay state and nothing about
  scenario internals. If that assertion ever grows into scenario content, it
  has drifted - the run proves the boot flow and menu teardown, nothing else.
- Pointer input is right HERE and only here. Owner call 2026-08-04, the split:
  `ui/` drives with real synthesized pointer input because the interface IS its
  subject - reachability, hover, press, hit-testing are the thing under test.
  `systems/outcomes` triggers `Activate` directly instead, because its subject
  is the outcome chain and pixel coordinates would only add layout coupling.
  Two idioms, each coupled to what it actually tests.
- The residual risk this leaves, stated plainly: `ui/` runs are now coupled to
  LAYOUT the way the retired story runs were coupled to CONTENT. Moving a panel
  can break several runs at once. That is accepted - it is the cost of testing
  a UI as a UI - but resolve click targets by `Name` rather than by literal
  coordinates wherever possible, so a move is survivable and only a rename
  breaks a run.
- Live-tree assertions are the point: `cargo check` misses duplicate-component
  panics and TextShadow ghosting. Examples must be RUN under Xvfb :99.
- `ui/` carries no fps window.
- `examples/ui/` must hold only runnable examples once `20260804-003301`
  relocates the `*_poc.html` sources.

### Planning findings, 2026-08-04

- DECISIONS D1 (NOVA OS/RTT coverage lands elsewhere, and why), D2 (the
  `Name`-resolved click actions belong in `nova_autopilot::input`) and D3
  (`widget_zoo` reaches `Playing` itself; the smoke sentinel becomes a const)
  are in `DECISION.md`. D1 closes the choose-one Step; the live NOVA OS claim is
  seeded as `20260804-134347` against `systems/`.
- ORDERING: `probe run ui` expands the CATALOG category, which still lists
  `nova_os_rtt_poc` (`Cargo.toml:120-122`). That run is a bare `App` with no
  completion protocol, so the DoD command cannot go green until
  `20260804-093910` deletes it. Added to DEPENDS ON at planning. Nothing else
  in this task blocks on it - the RTT test may land before or after the POC
  goes, and landing first is the safer order.
- `! ls examples/ui/*.html` is ALREADY green: `20260804-003301` landed
  (`caef2c7f`) and the three `*_poc.html` sources now live in `web/design/`.
  It is an inherited end-state to keep, not a change this task makes.
- The smoke contract is three greps, not one (`tests/examples_smoke.rs:300-325`):
  exit success, `nova harness: reached Playing`, and `autopilot: cycle
  complete, no panic` OR `probe: script complete, exiting`, with any
  "Encountered an error in command" failing the run. `widget_zoo` must satisfy
  all of them - hence D3.
- `nova_invariants` is safe in the zoo's bare app: it takes
  `Option<Res<NovaEventWorld>>` (`invariants.rs:152`) and its component queries
  simply find nothing. Enrolling all three probe plugins keeps the zoo's
  harness block identical to its siblings; no special-casing.
- `AutopilotPlugin::build` returns early without `NOVA_AUTOPILOT`
  (`autopilot.rs:333`), so the zoo's interactive run must not depend on the
  script for its state transition - the `Startup` transition is what makes both
  modes identical.
- `editor`'s 3D placement can drop its bespoke `PointerInput` synthesis:
  `move_cursor` / `press_mouse` write `WindowEvent::CursorMoved` and
  `WindowEvent::MouseButtonInput`, which is exactly what the picking backend
  tracks (`input.rs` module docs). One idiom for UI nodes and world picking.
- `NOVA_ZOO_CAPTURE` stays. `20260804-093855`'s contract test
  (`every_category_has_a_probe_policy`) only checks that each category has a
  policy row; it does not inspect what an example does, so nothing fires. The
  capture path is inert without its env, including under `probe run ui`.
- `menu_scenarios` already resolves targets by `Name`
  (`button_by_name`, `width_by_name`) - the 093934 open question is answered:
  the pattern exists, and this task promotes it into `nova_autopilot` rather
  than writing a fourth copy.

## Close-out, 2026-08-04

### What and why

`ui/` now DRIVES the interface. Every one of the five runs reaches its widgets
by moving a real pointer to a real screen position and pressing there; the
category no longer asserts around an interface it never touched.

- `nova_autopilot::input` gained the resolve half of a click (DECISION D2):
  `ui_node_centre` (physical -> logical via `ComputedNode::inverse_scale_factor`,
  the trap `menu_scenarios::width_by_name` already documented), plus
  `click_named` / `hover_named`, both warn-and-continue on an unknown name the
  way `move_cursor` does without a window, and `ui_node_rect` underneath both
  (the single home of the physical-to-logical conversion). Eight unit tests,
  including a scale-factor-2 case that a resolve skipping the conversion fails
  and a duplicate-name case that fails with the ambiguity warn deleted.
- `nova_debug::harness::REACHED_PLAYING` is the smoke sentinel, named by its two
  emitters (`DebugPlugin`, `widget_zoo`) and the test that greps for it.
- `widget_zoo` carries `GameStates` and steps into `Playing` in `Startup`
  (D3), joins the probe/harness wiring its siblings have, and is driven as 20
  beats: hover -> press -> reskin -> segmented select -> checkbox + toggle ->
  slider drag, with a VERDICT beat after each gesture. Its live tree is checked
  after both rebuild triggers.
- `editor` is a build-and-inspect run: click Sandbox, New Ship, hover the hull
  card and assert the tooltip NAMES it, click the card, place two sections by
  clicking the ship itself, then Select mode (places nothing) and Delete
  (count drops). Its bespoke `PointerInput` synthesis is gone.
- `menu_newgame` is the boot flow and nothing else. `NOVA_MENU_PATH=editorplay`
  is deleted - `editor` owns that sequence, and no caller outside the example
  ever set the variable (grepped).
- `menu_scenarios` clicks by pointer; the pane-width measure, its
  panic-on-CHANGED verdict and the self-ending completion are untouched. What a
  measurement REQUIRES did change (review round 1): a row is only clicked if it
  is inside the list's visible rect, and its widths only count once the picker
  has marked it `Selected`.
- `rtt_element_renders_its_subtree` covers the one POC claim nothing else made.
- Docs: the `ui/` bullet in `web/src/wiki/dev/development.md` and a CHANGELOG
  entry.

### Steps corrected against the code

Two clauses were written against an editor that does not exist, and were
corrected rather than faked (work rule 3):

- "click a placed section, assert the selection/tooltip surface names it".
  There IS no selection surface: `Select Section Button` sets
  `SectionChoice::None`, which is REBIND mode - clicking a section rebinds its
  key, and nothing displays the section's name. The naming claim is carried
  where a naming surface actually exists, on the component-card TOOLTIP
  (`editor: the tooltip names the section`), and select mode is asserted for
  what it observably does: the same click that placed a section a moment ago
  now places nothing.
- The "build tool is armed" assertion cannot read `SectionChoice`: it is
  `pub(crate)` to `nova_editor`, so an example cannot name the type. Arming is
  proven the only way it is observable from outside - the next two clicks place
  sections, and the same click places nothing once Select disarms it. Same for
  the ship's controller: the editor's markers are crate-private, so the beat
  asserts the section COUNT is 1 after New Ship.

### Difficulties

- A click is TWO beats everywhere. `Activate` fires on RELEASE over the same
  widget, so every gesture in every script is press-beat + release-beat; a
  single `click_named` beat leaves the widget pressed and never activates it.
  This is now stated in each script's comments because it is the first thing a
  reader gets wrong.
- `menu_scenarios`'s walk is an `each` state machine, not a step list, so the
  two-beat click needed a `pending_release` flag rather than a second step.
- The prior context left `editor.rs` with an unbalanced `.add())` and three
  names that do not resolve; the run could not have compiled. Fixed before
  anything was verified.
- A LAID-OUT NODE IS NOT A CLICKABLE NODE, and this is the branch's real bug.
  Review R1.1 asked for an assert that a clicked row actually became
  `Selected`; the assert fired immediately, 3 runs of 3. With the
  `webmods/gauntlet` mod and the `ledger_*` scenarios installed the picker
  lists 13 rows, the `Scenarios List` viewport is y 134..631, and
  `Scenario Row: asteroid_field` lays out at y 1214..1271. `ui_node_centre`
  resolves it happily - a `Name` and a `UiGlobalTransform` say nothing about
  visibility - so the walk clicked a coordinate occupied by something else and
  then measured the PREVIOUS selection's panes. It stayed invisible because the
  widths matched, which is exactly the vacuous-green R1.1 predicted.
  The walk now filters rows through `row_is_on_screen` (the row's centre must
  lie inside the list's rect) and skips the rest with a warn, since the picker
  does not scroll under the harness. The diagnosis needed the rects printed;
  guessing from the symptom pointed at layout timing, and a retry-on-settle
  built for that theory turned out to fix nothing and was removed.
  The general lesson, worth carrying to every `Name`-resolved click: resolving a
  target proves the node EXISTS, not that a pointer can reach it.

### Evidence

- `cargo run -p nova_probe -- run ui`: all five OK, 5/6 checks each (fps
  SKIPPED - `ui/` carries no baseline, as designed). widget_zoo 9s, editor
  11s, hud_range 10s, menu_newgame 7s, menu_scenarios 9s.
  QUALIFIED, twice over, and the qualifications are the honest part of this
  line.
  First: what round 1 recorded as a `menu_scenarios` FLAKE on this branch was
  not a flake. It was this branch panicking - `exit 101`, `thread 'main'
  panicked at menu_scenarios.rs:292` - on the assert R1.1 asked for, catching
  the real bug described under Difficulties. It read as an unexplained exit code
  because `run.log` DOES NOT CAPTURE PANICS (`grep -c panicked` over the failing
  `probe-runs/*/menu_scenarios/run.log` is 0), so probe reported `process_exit
  FAIL` with `log_clean PASS` and nothing to read. Fixed; 6 consecutive clean
  `menu_scenarios` runs, a clean `probe run ui` and a clean
  `ui_reach_playing_without_panic` since.
  Second: a SEPARATE flake does exist on master at 1 of 6, is untouched by this
  branch, and stays filed as `20260804-174231` - which now also carries the
  larger bug this turned up: a harnessed run can panic and still report
  `log_clean PASS`.
- The verdicts are real, not vacuous - from the run logs:
  `zoo: hover face lit (0.05 -> 0.12)`, `zoo: pressed face lit (0.12 -> 0.2)`,
  `zoo: skin is Hardware`, `zoo: demo level is Minimal`,
  `zoo: checks are [false, false, false, false]`,
  `zoo: slider dragged 0.5 -> 0.7083333`;
  `editor: tooltip names 'Reinforced Hull Section'`,
  `editor: placed 2 sections (1 -> 3)`,
  `editor: select mode is inert for placement (3 sections)`,
  `editor: deleted a section (3 -> 2)`;
  `menu_newgame: the menu tore down and gameplay state is up`;
  `scenarios pane widths HELD across 5 selections (list=331.0 details=481.0)`,
  each one a click verified to have landed by `assert_selection_landed`. The
  count is 5 rather than 6. CORRECTED in round 2 (R2.2) - see "Why
  `asteroid_field` is skipped" below; the reason given here, that a mod-free
  tree would measure 6, is wrong: both environments measure 5, and the probe
  environment overflows too.
- `cargo test -p nova_autopilot --lib input::` - 10 passed. CORRECTED in round 2
  (R2.5): 13, with the tests R2.4 added.
- `cargo test -p nova_gameplay --lib rtt_element_renders_its_subtree` - passed.
- `cargo test --test examples_smoke ui_reach_playing_without_panic` - passed
  (67.6 s, five examples including `widget_zoo`, under Xvfb :99).
- `cargo test --test examples_smoke catalog_matches_disk` - passed.
- `! rg -n 'world.trigger' examples/ui` - green (no matches).
- `! ls examples/ui/*.html` - green.
- `cargo check --examples --tests --features debug` clean; `cargo fmt --all`.

### Reflection

The live-tree check earned its place the moment `widget_zoo` started rebuilding
its body under a driven run: `cargo check` cannot see a ghost, and the only
reason the reskin and check-flip beats are trustworthy is that the tree is
counted immediately after each. The bigger lesson is the two-beat click - the
whole category's idiom hinges on a detail of `Activate` that is invisible from
the type signatures, and writing it down in three files is cheaper than the
next reader rediscovering it.

## Round 2 close-out, 2026-08-04

Finding numbers are the ones in Round 2 of `REVIEW.md`, R2.1 to R2.8.

### What changed

- `nova_autopilot::input::ui_node_rect` is the new primitive and the ONE home of
  the physical-to-logical conversion; `ui_node_centre` is its centre. Three
  copies of that conversion collapse into it (R2.7): `menu_scenarios`'s
  `width_by_name` and `node_rect` are both gone and both pane widths are read
  off the rect. Exported from the `nova_autopilot` and `nova_debug` preludes
  beside the rest of the pointer vocabulary.
- `crates/nova_autopilot/src/log_capture.rs` is a test-only `tracing` capture,
  lifted out of `completion.rs`'s test module where it already existed rather
  than written a second time. The duplicate-name test now asserts the warn is
  EMITTED, once, naming the count (R2.4) - proven by sabotage, below - and a
  companion test pins that a UNIQUE name resolves silently, so the warn cannot
  degrade into firing on every resolve.
- `menu_scenarios`'s walk gives each row `ROW_SETTLE_FRAMES` (10) driven frames
  to lay out inside the list before skipping it, and `RowPlacement` keeps the
  three not-clickable cases apart so the warn names which one fired (R2.1).
- `report()` has a coverage FLOOR: under `NOVA_AUTOPILOT` it panics below two
  measurements, since the property is a comparison ACROSS selections and one
  measurement makes both spreads zero (R2.3). The `skipped` field exists to make
  that message specific, and the coverage string is now on the HELD and CHANGED
  verdicts too - a passing run states what it covered, not only a failing one.
- A dropped measurement (either pane width missing) warns instead of vanishing
  (R2.8), and the module doc says the walk selects every row inside the list's
  visible box, not every listed row (R2.6).
- Stale close-out counts corrected (R2.5), and the 5-of-6 explanation rewritten
  against measured rects (R2.2), below.

### Why `asteroid_field` is skipped - measured, not reasoned

R2.2 is right that round 1's explanation was wrong. The corrected explanation is
not R2.1's either. Rects printed from `row_placement` under
`probe run menu_scenarios`, `DISPLAY=:99`, identical on all 10 settle frames:

| node | y extent |
| --- | --- |
| `Scenarios List` box | 134..631 (497 px) |
| `Scenario Row: shakedown_run` | 174..245 |
| `Scenario Row: broadside` | 245..345 |
| `Scenario Row: broadside_gunship` | 345..445 |
| `Scenario Row: lifeline` | 445..545 |
| `Scenario Row: final_tally` | 545..659 |
| `Scenario Row: asteroid_field` | 659..716 |

Measured / skipped: 5 / 1. The last row sits 28 px BELOW the bottom of the list
box, stably, after a full settle budget, so the skip is a real fold and is
correct. R2.1's arithmetic ("six rows of ~57 px cannot overflow a ~497 px
viewport") assumed uniform row heights: the rows run 71..114 px, because a
campaign-member row carries a blurb line under its name, and six of them are
542 px of content in a 497 px box. `final_tally` straddles the fold and is
clicked because its CENTRE is still inside.

The 13-row direct-run figure recorded in round 1 stands; it was not re-measured
this round.

### Pushback

R2.1's premise - "that skip is not a fold; it is `node_rect` being consulted
before the reopened list has settled" - is falsified by the rects above, in the
very environment R2.1 measured. `asteroid_field` is below the fold there.

The finding's REMEDY is implemented in full anyway, because its other half is
right and is what made the falsification possible: a single-frame look at a
just-rebuilt list could not tell "no rect yet" from "past the fold", which is
how one invocation measured 6 and the next 5. With the settle retry and the
three-way `RowPlacement` the count is deterministic and the warn names which
case fired. No row a pointer can reach is dropped.

### Evidence

All in the worktree, `DISPLAY=:99` (Xvfb :99, PID 3314249).

- Sabotage, R2.4: with the `rects.len() > 1` warn block DELETED from
  `ui_node_rect`, `a_duplicated_name_warns_and_resolves_to_one_of_them` FAILS
  (exit 101, assert on the captured log); restored, it passes. The test round 1
  cited could not have failed that way, which is what R2.4 said.
- Sabotage, R2.3: with the floor raised to `< 6` against a run that measures 5,
  the harnessed run panics with
  `TOO FEW measurements - only 5 of the picker's rows were measured ... 8
  skipped: Scenario Row: asteroid_field (the row's centre is outside the list's
  box), ...` - so the floor fires under `NOVA_AUTOPILOT` and names its coverage.
  Restored to `< 2`.
- `cargo test -p nova_autopilot --lib` - 38 passed (`input::` filter: 13).
- `cargo test -p nova_gameplay --lib rtt_element_renders_its_subtree` - passed.
- `cargo test --test examples_smoke catalog_matches_disk` - passed.
- `cargo test --test examples_smoke ui_reach_playing_without_panic` - passed,
  56.34 s, five examples including `widget_zoo`.
- `cargo fmt --all -- --check` clean; `cargo check --examples --tests --features
  debug` clean (the 4 `nova_gameplay` ambiguous-import warnings predate this
  branch and are untouched by it).
- `! rg -n 'world.trigger' examples/ui` green; `! ls examples/ui/*.html` green.
- DoD proof 1, `cargo run -p nova_probe -- run ui`: RED once, then GREEN. The
  red run was `menu_scenarios process_exit FAIL` - a non-zero exit AFTER
  `pane widths HELD`, `reached Playing`, `probe: script complete, exiting` and
  `harness completion: all collectors done, exiting`, with `log_clean PASS` and
  `rg -c 'panicked|ERROR' run.log` = 0. That is the exact signature of
  `20260804-174231` (non-zero exit on an otherwise clean `menu_scenarios` run),
  already filed off round 1 and reproduced on master there.
  Rate measured this round: 1 failure in 7 `menu_scenarios` probe runs on this
  branch. Not attributed to this diff, whose only touch outside
  `examples/ui/menu_scenarios.rs` and the input vocabulary is moving a test
  helper out of `completion.rs`'s test module - no non-test line changed there.
  The rerun of `probe run ui` was OK on all five.

### Reflection

Two of this round's findings were about a fix's SIDE, not its centre: R1.1
hardened the assert and opened a skip path, and the skip path then needed both
a settle budget (R2.1) and a coverage floor (R2.3) before the verdict meant
anything again. The generalizable rule: a fix that adds a way to SKIP work owes
the same round a floor under how much work may be skipped, and a run that can
under-cover should state its coverage on the PASSING path - a warn buried
mid-log is not a report.

The other lesson is cheaper: a test that pins a warn needs the log captured. The
round-1 test looked like coverage and was satisfied by the code it was meant to
pin, which is why R2.4 asked for capture and why the sabotage above is recorded
rather than asserted.

## Round 3 close-out, 2026-08-04

### What changed

- `a_named_node_resolves_to_its_logical_rect` spawns at scale factor 2 (R3.1).
  Round 2 moved the physical-to-logical conversion into
  `nova_autopilot::input::ui_node_rect` and shipped the SIZE half of it
  untested: at scale 1 `computed.size() * scale` is the identity, so the test
  was satisfied by code with the conversion deleted. It is load-bearing -
  `menu_scenarios` reads both pane widths off `rect.width()` and decides the
  fold with `list.contains(row.center())`, so a scale-2 display would double
  both and move the fold silently.
- A selection whose panes never lay out is now pushed onto `state.skipped`
  with its reason, and `pending_measure` is cleared with it (R3.2). Otherwise
  the row landed in neither `measured` nor `skipped` and `report()` would
  state a coverage the run did not have - the one thing that string exists to
  prevent. Clearing matters because the `else` runs every frame while
  `pending_measure` is set, and the next row is not always clicked on the next
  frame.
- `NODE_SIZE` moved above `spawn_named_node_at_scale`, which had lost its doc
  comment to the const round 2 inserted under it (R3.3).

### Evidence

Worktree, `DISPLAY=:99`.

- Sabotage, R3.1, run BEFORE and AFTER the fix: cutting `* scale` from the
  size term of `Rect::from_center_size` left all 13 `input::` tests green
  before, and fails `a_named_node_resolves_to_its_logical_rect` after (12
  passed, 1 failed, exit 101). The test now pins what its doc claims.
- `cargo run -p nova_probe -- run ui` - all five OK, 5/6 each (fps SKIPPED by
  design): widget_zoo 12s, editor 11s, hud_range 10s, menu_newgame 6s,
  menu_scenarios 8s. The verdict line carries its coverage:
  `widths HELD across 5 selections (list=331.0 details=481.0) - coverage: 5
  rows measured, 1 skipped: Scenario Row: asteroid_field (the row's centre is
  outside the list's box)`.
- `cargo test -p nova_autopilot --lib` 38 passed; `cargo test -p nova_gameplay
  --lib rtt_element_renders_its_subtree` passed; `cargo test --test
  examples_smoke catalog_matches_disk` passed; `cargo test --test
  examples_smoke ui_reach_playing_without_panic` passed, 50.1 s.
  `! rg -n 'world.trigger' examples/ui` and `! ls examples/ui/*.html` green.
  `cargo fmt --all -- --check` and `cargo check --examples --tests --features
  debug` clean.

### Follow-up, not fixed here

The round-3 reviewer's process signal is worth a task: the picker DOES support
wheel scroll (`scroll_menu_lists`, `crates/nova_ui/src/widgets.rs:72`), so the
permanent 5-of-6 coverage is a HARNESS gap - `nova_autopilot::input`
synthesizes no wheel event - rather than a property of the UI. Adding wheel
synthesis would let the walk reach every row instead of skipping the tail. Out
of scope for this branch, which is about driving what the pointer can already
reach.

### Reflection

Three rounds, three findings of the same shape: a guard that cannot fail.
R1.1's measurement could not fail when a click missed, R2.4's test could not
fail with its warn deleted, R3.1's test could not fail with its conversion
deleted. Each was caught only by deleting the mechanism and re-running - never
by reading. The rule this task earns: when a change moves logic into a shared
home, re-run its tests with that logic removed BEFORE claiming coverage,
because every caller now inherits the gap.
