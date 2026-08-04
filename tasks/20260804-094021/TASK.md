# Rebuild ui/ to drive real widgets with pointer input and assert the live tree

- PRIORITY: 83
- TAGS: v0.10.0, content, examples, testing, ui
- KIND: STORY
- ACTIVITY: WORKING
- GATES: PLAN
- RESOLUTION: -
- PARENT: 20260802-115955
- DEPENDS ON: 20260804-003244, 20260804-093855, 20260804-003301, 20260804-093910

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

- [ ] Add the `Name`-resolved pointer vocabulary to
      `crates/nova_autopilot/src/input.rs` (DECISION D2), exported from the
      crate prelude: `ui_node_centre(world, name) -> Option<Vec2>` (logical px
      centre from `GlobalTransform` + `ComputedNode`), `click_named(name)` and
      `hover_named(name)`, both warn-and-continue on a missing name the way
      `move_cursor` does without a window. Unit tests beside the existing ones
      in that module's `mod tests`: a named node resolves to its centre, a
      click on a name lands there, an absent name warns and does not panic.
- [ ] Promote the smoke sentinel to `pub const REACHED_PLAYING: &str =
      "nova harness: reached Playing"` in `crates/nova_debug/src/harness.rs`,
      and name it from `crates/nova_debug/src/lib.rs:131`, `widget_zoo` and
      `tests/examples_smoke.rs`'s stderr grep (DECISION D3).
- [ ] `widget_zoo` joins the fleet: `app.init_state::<GameStates>()` plus a
      `Startup` system setting `NextState` to `Playing`, a `#[cfg(feature =
      "debug")]` harness block matching its siblings (`nova_probe::nova_timeline
      / nova_invariants / nova_frametime`, all inert without their env, plus
      `nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>` named
      FULLY QUALIFIED - `examples_name_drivers_through_the_nova_harness` fails a
      bare name), and the `REACHED_PLAYING` log on
      `OnEnter(GameStates::Playing)` gated on `NOVA_AUTOPILOT`.
- [ ] Give the driven `widget_zoo` widgets `Name`s (it has none today): the two
      Skin segmented options, the `DemoLevel` segmented options, the four
      `clickable` checks/toggles (`widget_zoo.rs:322-327`), the slider, and one
      hoverable button from the States row.
- [ ] Drive `widget_zoo` as a step script, one beat per gesture:
      hover a button and assert its hover face; press it and assert the pressed
      face; click `Hardware` and assert `UiSkin` flipped; click a `DemoLevel`
      option and assert the resource; click one checkbox and one toggle and
      assert `ZooChecks`; drag the slider (`hover_named` -> `press_mouse` ->
      `move_cursor` along the track from `ui_node_centre` -> `release_mouse`)
      and assert `SliderValue` moved.
- [ ] Assert `widget_zoo`'s LIVE TREE after the two rebuild-triggering beats
      (reskin and check flip - `rebuild_body` despawns and respawns the body,
      `widget_zoo.rs:194-231`): exactly one `ZooBody`, exactly one entity per
      driven `Name`, and no `TextShadow` anywhere under the root (nova_ui
      refuses it on purpose - `widget/button.rs:654`).
- [ ] Deepen `examples/ui/editor.rs` into build-and-inspect: create the ship,
      click a hull section CARD by name (replacing the
      `world.entity_mut(button).insert(Pressed)` shortcut at
      `editor.rs:~170`), place TWO sections through the real pointer, then
      inspect - click `Select Section Button`, click a placed section, assert
      the selection/tooltip surface names it - then `Delete Section Button` and
      assert the count drops back. Delete the bespoke `send_pointer` /
      `PointerInput` synthesis (`editor.rs:~260-281`) in favour of
      `move_cursor` / `press_mouse` / `release_mouse`, which write the same
      `WindowEvent` the picking backend reads.
- [ ] Narrow `examples/ui/menu_newgame.rs` to the boot flow only: `click_named`
      on the menu button, advance until `GameStates::Playing`, assert nothing
      about `shakedown_run`'s contents. Decide and record: keep or drop the
      `NOVA_MENU_PATH=editorplay` branch, now that `editor` owns the
      create-ship-and-Play sequence - dropping it is the default, since two runs
      covering one transition is the duplication the roster spike cut.
- [ ] Deepen `examples/ui/menu_scenarios.rs`: replace both
      `world.trigger(Activate { .. })` calls (rows and the Play button) with
      `click_named`, keep the pane-width measure and its `panic`-on-CHANGED
      verdict exactly as they are, and keep the self-ending completion.
- [ ] Add `rtt_element_renders_its_subtree` to
      `crates/nova_gameplay/src/hud/nova_os/tests/crt.rs` - the ONE POC claim no
      test makes today (DECISION D1): a `NovaOsImageCameraMarker` camera exists,
      its `RenderTarget` is the `NovaOsRtt` image, it sits on
      `RenderLayers::layer(NOVA_OS_RTT_LAYER)` at `NOVA_OS_RTT_CAMERA_ORDER`,
      and the content root's subtree is on that same layer and non-empty - i.e.
      the element displays a subtree that is actually rendered offscreen, not an
      empty target.
- [ ] Move `widget_zoo` from `NOT_SMOKED` to `UI` in `tests/examples_smoke.rs`
      (`:47`, `:74-82`), rewriting the comment to say what "reached Playing"
      means for a widget showcase.
- [ ] Update `web/src/wiki/dev/development.md`'s `ui/` bullet (`:170-173`) for
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
