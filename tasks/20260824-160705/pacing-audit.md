# Autopilot pacing audit

Read-only inventory of every `.until(frames(..))` call site, taken on the
`autopilot-pacing` sprout at `fec6e441`. No behaviour changed *by the audit*;
what was then implemented off it is in section 7.

**Can we make improvements? Yes - 73 of 145 sites, and the pattern that replaces
them is already written and proven in this tree.** The other 72 are correct as
they stand and need a documented reason, not a rewrite.

## 1. The count

145 `.until(frames(..))` sites across 42 files. `frames` is never called
anywhere else, so `.until` is its whole surface.

| Argument | Sites |
| --- | --- |
| `SETTLE_FRAMES` (shared, 30) | 57 |
| `SETTLE` (local: 3, 10, 12) | 25 |
| `frames(1)` | 28 |
| `SETTLE_FRAMES * 2` / `SETTLE * 2` | 6 |
| other literals (2, 6, 8, 12, 20, 30, 45, 75) | 26 |
| named budgets (`hold_frames()`, `TORPEDO_FLIGHT_FRAMES`, ...) | 3 |

Six different settle constants are in play: the shared `harness::SETTLE_FRAMES`
= 30, plus locals at 3 (`pointer_pin`), 8 (`system_input_modes`), 10
(`widget_zoo`), 12 (`system_nova_os`) and 24 (`system_ui_scale`).

Summed across the fleet a smoke run spends **2927 driven frames** sitting in
these waits.

## 2. Classification

| Class | Sites | Frames | Verdict |
| --- | ---: | ---: | --- |
| **T** Tautological `frames(1)` | 28 | 28 | Delete |
| **P** Post-verdict padding | 2 | 24 | Delete |
| **O** An observable states the real contract | 43 | 673 | Replace |
| **R** Render / capture stillness settle | 48 | 1402 | Keep, document |
| **M** Deterministic media duration | 15 | 713 | Keep, document |
| **S** Deliberate separation | 9 | 87 | Keep, document |

Addressable: **73 sites, 725 frames**. Correct as written: **72 sites, 2202
frames**.

Per-site table in the appendix.

## 3. Findings

### F1. `frames(1)` is exactly "no `.until`", proved from the driver

`autopilot_drive` (`crates/nova_autopilot/src/autopilot.rs:457`) gives a step's
entry its own frame and returns without polling anything. The clock is zeroed
there. The next driven frame increments `step_frames` to 1 and *then* polls. So
`frames(1)` and the default `immediately()` advance on the same frame, always.

28 sites say `frames(1)`. 24 of them follow either an assertion closure or
`loop_start`, where the step is a pure action and the `.until` is noise.
`system_ui_scale.rs` already writes those beats as a bare `.add()`, so the
correct idiom is in the tree and simply was not applied.

### F2. The migration this task proposes has already been done once

`examples/systems/system_ship_editor.rs` is the largest driven walk in the
fleet - well over a hundred beats - and it contains **zero** `frames` waits.
Every beat holds on `ui_node_present`, `pointer_pressed`, `pointer_released`,
`editor_placement_solved`, `editor_gallery_selected` or a local closure, each
with a `deadline`. The gestures are wrapped in an `EditorGestures` trait so one
click reads as one line. `examples/screenshots/shared/ui_walk.rs` carries the
same trait for the menu walks.

The three files the task nominates are exactly the pointer walks that never
adopted it:

```rust
// system_ship_editor.rs - the pattern that works
.step(format!("{label}: press"))
.on_enter(click_named(target))
.until(pointer_pressed())
.deadline(BEAT_DEADLINE_SECS)
.add()

// widget_zoo.rs - the same gesture, ten frames of guessing
.step("zoo: press the hovered button")
.on_enter(press_mouse(MouseButton::Left))
.until(frames(SETTLE))
.add()
```

This is the single most important result of the audit: the work is *applying an
existing local idiom*, not designing a new one.

### F3. None of the three nominated files needs a new probe

- **`widget_zoo`** (22 sites, 176 frames): 6 are `frames(1)`; the other 16 are
  press / release / hover / drag gestures that `pointer_pressed`,
  `pointer_released` and `pointer_at` already ack. Nothing new.
- **`system_nova_os`** (10 sites, 76 frames): 4 are `frames(1)`, 2 are padding
  after a verdict, and the rest are covered by `NovaOsTerminal::active_mode()`
  and `nova_os_openness()` - both already public, and both already used by
  predicates *in the same file*.
- **`pointer_pin`** (4 sites): 3 are the generic pointer acks. The fourth ("a
  foreign pointer event lands") must stay a frame count: the test's whole point
  is that the stray is *overridden*, so by construction there is nothing to
  observe.

The only genuinely new predicate worth adding is for `system_ui_scale`: a
window-shape and scale-factor ack. That is pure `bevy::window` state, so it
belongs in `nova_autopilot::predicate` beside `pointer_at`, not in a Nova probe.

### F4. The biggest category is not a defect

48 sites / 1402 frames are the pre-shot stillness settle: pose the camera, wait
`SETTLE_FRAMES`, `shoot`. There is no cheap observable for "the picture stopped
changing" - it would take a frame-difference probe, which is a larger and
different piece of work. The `shot_written` ack already removed the *write
latency* from these numbers; what is left is the stillness figure alone.

The gap is documentation, not code: the reason is written once, on
`harness::SETTLE_FRAMES`, and the 47 call sites outside that crate inherit it
silently.

15 more sites / 713 frames are encoded video length. An armed loop run pins
`TimeUpdateStrategy::ManualDuration` to `1/fps`, so `frames(75)` *is* 2.5 s of
webm at 30 fps, exactly and reproducibly. `elapsed` would be wrong here:
identical on the armed path, wildly different on the smoke path.
`loop_vfx_range.rs` goes further and budgets its counts against the recorder's
600-frame cap. These are the strongest `frames` call sites in the tree.

### F5. A frame count cannot stall, so it never produces a diagnostic

This is the robustness cost, and it is worth stating plainly. `frames(N)`
always completes. When the guess is too short the run does not fail - it takes a
wrong picture, or a later assertion fails on a symptom several beats away. An
observable wait plus a `deadline` fails *at the beat that went wrong*, by name.

Corollary: 16 `frames()` steps carry a `.deadline(...)` that can only fire if
the app renders slower than about one frame per second. They read as diligence
and are dead code.

### F6. Two settles are pure padding

`system_nova_os.rs:163` and `:192` run an assertion in `on_enter` and then wait
`frames(SETTLE)` - 12 frames each, after the verdict has already been reached.
Same defect as F1 with a bigger number.

### F7. One constraint tension, and how it resolves

The task forbids gating an assertion on the invariant it is meant to prove.
`system_nova_os` has three beats where the naive replacement would do exactly
that - aim through the glass, then assert the forwarded pointer arrived.

It resolves cleanly because the two facts are different: `pointer_pressed()`
answers for the **window** mouse, while `assert_press_through_the_glass` claims
the **forwarded** pointer reached an offscreen widget. Waiting on the first does
not prove the second. A predicate that read the offscreen tree's hover state
*would* be forbidden, and should not be added.

### F8. The nova_os screenshot ranges duplicate a walk that is already correct

`screenshot_nova_os_apps.rs` and `screenshot_nova_os_terminal.rs` re-derive the
same terminal walk with `frames(6)` and `frames(SETTLE_FRAMES)`, while
`system_nova_os.rs` next door holds the same beats on the terminal model's own
`active_mode()`. `type_word` writes every character in one frame, so `frames(6)`
is not a typing rate - it is a guess at how long the shell takes to answer.

## 4. What I would do, in bands

| Band | Work | Sites | Frames | Risk |
| --- | --- | ---: | ---: | --- |
| **A** | Delete the 28 `frames(1)` and the 2 padding settles | 30 | 52 | None - provably identical |
| **B** | Pointer walks onto the acks: `widget_zoo`, `pointer_pin`, `system_nova_os` | 23 | 227 | Low - the `system_ship_editor` pattern, unchanged |
| **C** | nova_os screenshot ranges share the terminal predicates | 9 | 174 | Low |
| **D** | `system_ui_scale`: add `window_size_is` / `window_scale_factor_is` to `nova_autopilot` | 6 | 144 | Medium - new predicates, needs a layout-pass beat after the ack |
| **E** | Everything else: write the reason down, in `frames`' docstring and `docs/automation-harness.md`, and name the categories | 72 | - | None |

Band A is mechanical and I would do it whatever else is decided. B and C are
the ones that buy real robustness: they turn "the guess was too short" into a
named stall. D is the only band that adds API.

One risk to flag on B: today's counts are generous, and an exact ack could
expose an off-by-one the generosity was hiding - a widget repaint that lands one
frame after the pointer ack. The verdict beats are already separate steps, and
a step costs an entry frame, so each verdict already gets that frame for free.
If a beat still needs more, `and(ack, frames(2))` says so honestly; padding a
lone count does not.

## 5. Not doing

- No frame-difference "the picture is still" probe. It would address F4's 1402
  frames, but it is a different task and the task's own scope forbids adding
  probe state to shrink a frame total.
- No change to `frames` itself. It stays public; its docstring gets the
  constrained role written on it.
- No touching the media budgets in `loop_*`. They are the best-justified frame
  counts in the tree.

## 6. Cleanup beyond the frame waits

The `.until(frames(..))` grep is not the whole pacing surface. Widening the
audit to how the walks are *built* turned up more, and bigger.

### C1. A second step machine exists, and it is worse

Five `examples/systems/system_headless_*.rs` ranges - 1679 lines, 56 numbered
step arms - never use `AutopilotPlugin` at all. Each hand-rolls the same
machine: a `Spike { step, wait, phase }` resource, a `match step { 0 => .., 1
=> .. }` driver, a wall-clock `panic!` for a deadline, and `if wait < SETTLE {
hold(world) }` for pacing.

```rust
// system_headless_drag.rs - the harness, rebuilt by hand
6 => {
    if wait == 0 { hover_named(TRACK)(world); }
    if wait < SETTLE { hold(world); } else { advance(world); }
}
```

Everything the step model gives away is lost here. Steps are NUMBERED, so a
stall reads `headless drag: STALLED at step 7` instead of naming the beat. The
deadline is a panic on run-elapsed rather than the driver's named abort. The
pacing is the same guessing the rest of this audit is about, and a
`.until(frames(` grep cannot see any of it.

`system_headless_drag.rs` drives the same slider hover/press/drag/release walk
that `widget_zoo.rs` drives - the same widget, the same gestures, two different
mechanisms, both guessing.

### C2. "Click a widget" has four incompatible spellings

The most-used gesture in the fleet has no canonical version:

| Where | Shape | Release beat waits on |
| --- | --- | --- |
| `shared/ui_walk.rs::Gestures::click` | 3 beats | `pointer_released()` |
| `system_ship_editor.rs::EditorGestures::click_a_widget` | 3 beats, same body | `pointer_released()` |
| `system_input_modes.rs`, `system_ui_scale.rs`, `system_field_controls.rs` | inline, per call | **what the click caused** |
| `widget_zoo.rs`, `system_nova_os.rs` | inline, per call | `frames(SETTLE)` |

The third row is the best of the four and nobody named it: ending the release
beat on `ui_node_present(NAME_FIELD)` or `and(editor_field_focused(),
the_mode_is(Insert))` says what the click was FOR. `EditorGestures::
press_and_release(label, landed)` is that shape already written down.

Canonicalise it as `AutopilotPlugin::click_named(label, name, landed,
deadline)` beside the existing `double_click_named` - everything it needs
(`ui_node_present`, `pointer_pressed`, `pointer_released`) is already in
`nova_autopilot` - and both trait copies go.

### C3. Constants copied instead of shared

- `const STEP_DEADLINE_SECS: f32 = 30.0` is declared **13 times**, always with
  the same value.
- `BEAT_DEADLINE_SECS: f32 = 20.0` four times, one of them already `pub` in
  `shared/editor_walk.rs`.
- Six different settle constants: 3, 8, 10, 12, 24, and the shared 30.

### C4. Two pieces of dead or near-dead public API

`AutopilotPlugin::hold` has **zero call sites** outside the crate's own unit
tests, yet `lib.rs` and `predicate.rs` both give it top billing as the proof
that the step model needs no second mechanism. Even `harness::nova_autopilot()`
- literally "enter Loading, hold N seconds", the case `hold` was written for -
spells the step out instead. Use it in that preset or drop it; today it is a
documented idiom nothing follows.

`script_reports_done` has one user (`bug_menu_picker.rs`) and its own docstring
calls it "a knot only this migration needs". The migration is otherwise over.

### C5. A missing parameter is buying real complexity inside the crate

`each` receives `(&mut World, f32)` - in-step SECONDS only - although the
driver already tracks `step_frames` on the same clock. So `double_click_named`,
the one gesture that has to act on specific frames within a step, smuggles an
`Arc<AtomicU8>` phase counter through three separate closures (`on_enter`,
`each`, and `until`) to reconstruct the frame index the driver is already
holding.

Handing `each` the in-step frame index deletes that atomic and makes
"press this frame, release the next" expressible without one.

### C6. The one genuinely opaque stall

`ui_node_rect` collapses four distinct failures into `None`: no entity of that
name; the entity exists but its ancestry is hidden; it exists and is visible but
carries a zero-size box (layout has not run, or `Display::None`); or a component
it reads is unregistered. A stall on `ui_node_present` therefore prints

```
autopilot: step `open Settings: the widget is up` stalled after 20.0s (run 41.2s, state Playing)
```

and nothing else. Those are three different bugs with three different fixes, and
the predicate already separates them internally. This is the one place the audit
found where the task's "improve a deadline diagnostic" applies: the useful
observed value is right there and is currently thrown away.

Everywhere else the diagnostics are fine. The opposite problem shows up instead:
16 `frames()` steps carry a `.deadline(..)` that can only fire if the app renders
below about one frame per second.

### C7. Priority

| | Work | Size | Why |
| --- | --- | --- | --- |
| 1 | C1 - the headless family onto `AutopilotPlugin` | Large | Removes a whole parallel mechanism; named stalls; brings ~1700 lines under one model |
| 2 | C2 - one `click_named` beat builder, `landed` and all | Medium | Deletes two trait copies and every inline spelling; the release beat starts carrying its purpose |
| 3 | C6 - say WHY `ui_node_rect` returned `None` | Small | The only opaque stall the audit found |
| 4 | C3 - one deadline constant, one settle constant | Small | 13 identical copies |
| 5 | C5 - frame index into `each` | Small | Deletes the `AtomicU8` in the crate |
| 6 | C4 - use or drop `hold` and `script_reports_done` | Small | Docs currently promise an idiom nothing follows |

C1 and C2 are the architectural ones the task is asking after. Bands A-D of
section 4 are the pacing ones. They compose: C2 is what Band B's `widget_zoo`
and `system_nova_os` beats should be rewritten ONTO, so doing C2 first makes
Band B a deletion rather than a rewrite.

## 7. What landed

Implemented on this sprout after the audit was reviewed. The appendix below is
the pre-change inventory and is left as the baseline it was taken as.

| | Before | After |
| --- | ---: | ---: |
| `.until(frames(..))` sites | 145 | 79 (81 after the sync with master) |
| `frames(1)` sites | 28 | 0 |
| settle constants in play | 6 | 2 (`SETTLE_FRAMES`, one local `REFRAME_FRAMES`) |
| `STEP_DEADLINE_SECS` declarations | 13 | 1 (`nova_debug::harness`) |
| hand-rolled step machines | 5 files, 56 numbered arms | 0 |

### Bands

- **A** - all 28 `frames(1)` and both post-verdict paddings deleted. Provably a
  no-op: a step with no `until` already advances on its first driven frame.
- **B** - `widget_zoo` (22 waits, 0 left), `pointer_pin` (1 documented count
  left) and `system_nova_os` (0 left) run on the pointer and layout acks.
  `widget_zoo`'s whole walk now takes 0.3s where the settles cost it seconds,
  and every verdict still reads the value it used to.
- **C** - the two nova_os screenshot ranges wait on the terminal's own answers:
  `raster_open`, `command_line_reads`, `app_owns_the_screen`,
  `the_shell_answered` (a scrollback REVISION, so output that scrolls the top
  rows off still counts). The pre-shot `SETTLE_FRAMES` stillness is unchanged
  and is now its own named step, so the shot's budget is visible instead of
  bundled into the launch.
- **D** - `system_ui_scale` waits on the landings: `window_scale_factor_is` plus
  `ComputedNode::inverse_scale_factor` for a DPI change, `window_size_is` plus
  `Camera::logical_viewport_size` for a reshape. The four camera reframes keep
  their count as `REFRAME_FRAMES`: framing is an ease with no arrival flag.
- **E** - the constrained role of `frames` is written on the predicate and in
  `docs/automation-harness.md`, with the four cases that qualify.

### Cleanup

- **C2** - `AutopilotPlugin::click_named(label, name, landed, deadline)` is the
  one click builder; both trait copies delegate to it and carry no body. It is
  four beats, not three: the added `aim` beat re-hovers every frame and holds
  until `pointer_over_node` says the pointer really picks that widget, so a
  reflow or a fading overlay that took the pick fails there, named by
  `pointer_hover_diagnosis` ("the pointer is over [\"Loading Veil\"], not
  `Play Button`"), instead of pressing into the occluder.
- **C3** - `STEP_DEADLINE_SECS` (30s, a world condition) and
  `BEAT_DEADLINE_SECS` (20s, one gesture) live in `nova_debug::harness`; the 18
  local copies are gone.
- **C4** - `AutopilotPlugin::hold` is deleted (zero callers outside its own
  tests). `script_reports_done` is KEPT: `bug_menu_picker` walks however many
  scenario rows the catalog holds, so its beat list is not known at build time,
  which is the one shape that needs it. Its docstring says that now instead of
  calling itself a migration knot.
- **C5** - `each` and `input` take the in-step frame index. The `AtomicU8` phase
  counter smuggled through three closures in `double_click_named` is gone.
- **C6** - `ui_node_diagnosis` splits `ui_node_rect`'s four failures apart, and
  a stalled layout beat appends it to the abort message.
- **C1** - done. All five `system_headless_*.rs` ranges are on
  `AutopilotPlugin`: 1679 lines and 56 numbered `match step` arms become 1400
  lines of named beats. `Spike { step, wait, .. }`, the wall-clock `panic!`
  deadlines and every `if wait < SETTLE { hold(world) }` are gone. Each stall
  now names its beat - `headless crt: the forwarded pointer reaches the blip`
  rather than `STALLED at step 12` - and carries a `diagnose` where there was a
  periodic `info!`: which codes the map plotted, which wear the selection ring,
  what the pointer is over. The gates are the real conditions the settles were
  guessing at: the forwarded pointer arriving on the blip, `Pressed` on the
  widget, the terminal's own `active_mode`, the registry's own binding.

### Defects the migration found

Two real bugs, both invisible to the grep this audit started from, both found
by moving a hand-rolled range onto the driver and watching it hang.

- **A deadline could not fire behind the pause overlay.** The driver measured
  `deadline` on `Time::delta_secs()` - the app's own clock, which Nova PAUSES
  with the game. Every walk that opens the pause menu or the ship computer
  froze it, so every deadline set from that beat onward stopped counting: a
  stalled step held the run open until the outer harness killed it, with
  nothing naming the step. The clock is now `Time<Real>`, kept as
  `AutopilotClock::step_real`, and
  `a_deadline_still_expires_while_the_app_clock_is_paused` pins it.
- **`system_headless_drag` had been walking a widget that no longer exists.**
  It aims at `"Volume Slider Track"`; the widget was renamed
  `"Master Volume Slider Track"`. `hover_named` warns and CONTINUES on a name
  nothing answers to, so the range spent every run hovering nothing and
  stalling on `wait < SETTLE` counts that could never be satisfied - and its
  hand-rolled 180-second panic was the only thing that ever ended it. Confirmed
  by building master's copy of the file unchanged and watching it panic
  `STALLED at step 5 after 180s`. Both bugs had to be fixed before the migrated
  range could pass.

### Proof

Through `nix develop`, on the sprout, re-run after the sync with master:

- `cargo check --features debug --examples` clean; `cargo check -p nova_autopilot
  -p nova_debug -p nova_os_ui --all-targets` clean.
- `cargo test -p nova_autopilot --lib --test pointer_pin`: 76 + 2 passed.
- `cargo test -p nova_os_ui --lib`: 117 passed.
- Run live on Xvfb :99, all `cycle complete, no panic`: `widget_zoo` (0.3s),
  `system_ui_scale` (4.1s, chip 24/5 at 1x, 2x, 1280x600 and 760x600, 9
  nameplates apart), `system_nova_os` (1.6s), and both nova_os screenshot
  ranges on the smoke path and the capture path.
- The four fleet walks that click through `click_named` re-run on Xvfb :99
  against the new aim beat: `screenshot_menu`, `screenshot_scenario_picker`,
  `screenshot_editor`, `system_ship_editor`.
- The five migrated headless ranges run with NO display at all
  (`NOVA_AUTOPILOT=1`, no `DISPLAY`), each `cycle complete, no panic` in 0.5s:
  `system_headless_pointer`, `system_headless_novaos` (registry holds 33
  actions), `system_headless_rebind` (the registry took J for `main_drive`),
  `system_headless_drag` (`MasterVolume` 0.5 -> 0.64) and `system_headless_crt`
  (clicked AST-1 through the glass, GOTO engaged) - against 180-second
  wall-clock panics and multi-second settle dwells before.
- `loop_vfx_range`, which master rewrote around the lance while this branch was
  open, runs clean on the merged tree (19.1s smoke path).
- Every outcome slug of all five migrated ranges is unchanged, checked by
  diffing the emitted `"outcome: ..."` set against master's copy of each file.
  `cargo test -p nova_probe_cli --test catalog_drift` passes after the sync
  with master; before it, the branch failed that test on master's own
  `SYSTEMS_INVARIANTS` 209 -> 213 bump, which nothing here touches.
- The sync brought two new frame waits in master's `railgun_wake_bench`
  (a pose settle and a `frames(2)` separation before a volley). They post-date
  the inventory and are left alone: the bench's own next beat, `slug_flew`, is
  the real gate, and re-timing a capture its author had just tuned is not this
  task's call.
- `wiki-nova-os-terminal.png` re-captured and compared against the committed
  `web/src/assets/` copy: identical framing and content, only the version
  string and the fps readout differ.

## Appendix: every site

Class key: **T** tautological, **P** post-verdict padding, **O** observable
available, **R** render/capture settle, **M** media duration, **S** deliberate
separation.

| Site | Step | Count | Class | Note |
| --- | --- | --- | --- | --- |
| `crates/nova_autopilot/examples/driven_app.rs:129` | release the button | `1` | **T** | advance-only after `release_mouse` |
| `crates/nova_autopilot/examples/driven_app.rs:133` | settle in Done | `2` | **S** | doc demo: hold `Done` a beat so the state shows in the log |
| `crates/nova_autopilot/tests/pointer_pin.rs:188` | let the target lay out | `SETTLE` | **O** | `ui_node_present(TARGET)` - the rig spawns a laid-out node |
| `crates/nova_autopilot/tests/pointer_pin.rs:192` | click the target | `SETTLE` | **O** | `pointer_pressed()` |
| `crates/nova_autopilot/tests/pointer_pin.rs:196` | a foreign pointer event lands | `SETTLE` | **S** | the stray must get its chance to do damage; no ack exists by construction |
| `crates/nova_autopilot/tests/pointer_pin.rs:200` | release the target | `SETTLE` | **O** | `pointer_released()` |
| `crates/nova_debug/src/harness.rs:373` | nova: settle for the screenshot | `SETTLE_FRAMES` | **R** | the one shared pre-shot stillness settle |
| `examples/playable/block_bench.rs:962` | frame the roster | `SETTLE_FRAMES` | **R** | pose then shoot |
| `examples/playable/compare_asteroids.rs:227` | frame the lineup | `SETTLE_FRAMES` | **R** | pose then shoot |
| `examples/playable/compare_asteroids.rs:241` | dress the focus rock with roster entry {index} | `SETTLE_FRAMES` | **R** | re-dress a mesh then shoot |
| `examples/playable/compare_planets.rs:224` | frame the lineup | `SETTLE_FRAMES` | **R** | pose then shoot |
| `examples/playable/compare_planets.rs:238` | dress the focus sphere with roster entry {index} | `SETTLE_FRAMES` | **R** | re-dress a mesh then shoot |
| `examples/playable/greeble_catalog.rs:1115` | frame the wall | `SETTLE_FRAMES * 2` | **R** | pose then shoot |
| `examples/playable/greeble_catalog.rs:1139` | focus the selected piece | `SETTLE_FRAMES` | **R** | pose then shoot |
| `examples/playable/parts_viewer.rs:783` | viewer: settle the gallery | `SETTLE_FRAMES * 2` | **O** | scenes stream in async; the spawn is observable |
| `examples/playable/parts_viewer.rs:796` | viewer: flip to gallery page {}", page + 1 | `SETTLE_FRAMES` | **R** | state flip then shoot |
| `examples/playable/parts_viewer.rs:821` | viewer: focus a blocks piece | `SETTLE_FRAMES` | **R** | state flip then shoot |
| `examples/playable/parts_viewer.rs:862` | viewer: focus catalog card {label} | `SETTLE_FRAMES` | **R** | state flip then shoot |
| `examples/playable/parts_viewer.rs:880` | viewer: pose catalog card {label} | `SETTLE_FRAMES` | **R** | pose then shoot |
| `examples/playable/parts_viewer.rs:938` | viewer: focus {label} | `SETTLE_FRAMES` | **R** | state flip then shoot |
| `examples/playable/parts_viewer.rs:952` | viewer: pose {label} cut faces toward the camera | `SETTLE_FRAMES` | **R** | pose then shoot |
| `examples/playable/parts_viewer.rs:986` | viewer: show {shot} | `SETTLE_FRAMES` | **R** | state flip then shoot |
| `examples/playable/shape_bench.rs:932` | frame the roster | `SETTLE_FRAMES` | **R** | pose then shoot |
| `examples/playable/wfc_arena.rs:2268` | open the arena loop | `1` | **T** | advance-only after `loop_start` |
| `examples/playable/wfc_ships.rs:681` | frame the row | `SETTLE_FRAMES` | **R** | pose then shoot |
| `examples/playable/widget_zoo.rs:675` | zoo: settle | `SETTLE * 2` | **O** | `ui_node_present(IDLE_BUTTON)` - the first body laid out |
| `examples/playable/widget_zoo.rs:688` | zoo: hover a button | `SETTLE` | **O** | `pointer_at(centre)` / the node reports `Hovered` |
| `examples/playable/widget_zoo.rs:708` | zoo: the hover face is lit | `1` | **T** | advance-only after a verdict |
| `examples/playable/widget_zoo.rs:712` | zoo: press the hovered button | `SETTLE` | **O** | `pointer_pressed()` |
| `examples/playable/widget_zoo.rs:730` | zoo: the pressed face is lit | `1` | **T** | advance-only after a verdict |
| `examples/playable/widget_zoo.rs:734` | zoo: release the button | `SETTLE` | **O** | `pointer_released()` |
| `examples/playable/widget_zoo.rs:740` | zoo: click Hardware | `SETTLE` | **O** | `pointer_pressed()` |
| `examples/playable/widget_zoo.rs:744` | zoo: release on Hardware | `SETTLE` | **O** | `pointer_released()` |
| `examples/playable/widget_zoo.rs:758` | zoo: the skin flipped | `1` | **T** | advance-only after a verdict |
| `examples/playable/widget_zoo.rs:762` | zoo: click a HUD-level option | `SETTLE` | **O** | `pointer_pressed()` |
| `examples/playable/widget_zoo.rs:766` | zoo: release on the HUD-level option | `SETTLE` | **O** | `pointer_released()` |
| `examples/playable/widget_zoo.rs:776` | zoo: the HUD level changed | `1` | **T** | advance-only after a verdict |
| `examples/playable/widget_zoo.rs:780` | zoo: click a checkbox | `SETTLE` | **O** | `pointer_pressed()` |
| `examples/playable/widget_zoo.rs:784` | zoo: release on the checkbox | `SETTLE` | **O** | `pointer_released()` |
| `examples/playable/widget_zoo.rs:788` | zoo: click a toggle | `SETTLE` | **O** | `pointer_pressed()` |
| `examples/playable/widget_zoo.rs:792` | zoo: release on the toggle | `SETTLE` | **O** | `pointer_released()` |
| `examples/playable/widget_zoo.rs:811` | zoo: both flips landed | `1` | **T** | advance-only after a verdict |
| `examples/playable/widget_zoo.rs:817` | zoo: hover the slider | `SETTLE` | **O** | `pointer_at(centre)` |
| `examples/playable/widget_zoo.rs:821` | zoo: press on the slider | `SETTLE` | **O** | `pointer_pressed()` |
| `examples/playable/widget_zoo.rs:838` | zoo: drag along the track | `SETTLE` | **O** | `pointer_at(last drag leg)` |
| `examples/playable/widget_zoo.rs:842` | zoo: release the slider | `SETTLE` | **O** | `pointer_released()` |
| `examples/playable/widget_zoo.rs:865` | zoo: the slider moved | `1` | **T** | advance-only after a verdict |
| `examples/screenshots/loop_cockpit.rs:64` | open the cockpit loop | `1` | **T** | advance-only after `loop_start` |
| `examples/screenshots/loop_cockpit.rs:83` | type the map command | `6` | **M** | inside the recording: how long the typed command sits on screen |
| `examples/screenshots/loop_derived_skin.rs:68` | open the derived-skin loop | `1` | **T** | advance-only after `loop_start` |
| `examples/screenshots/loop_derived_skin.rs:75` | derive the skin | `30` | **M** | inside the recording: the derive beat's screen time |
| `examples/screenshots/loop_derived_skin.rs:83` | stop the turntable | `1` | **T** | advance-only after a resource poke |
| `examples/screenshots/loop_goto_arrival.rs:147` | open the arrival loop | `1` | **T** | advance-only after `loop_start` |
| `examples/screenshots/loop_player_flight.rs:60` | open the player-flight loop | `1` | **T** | advance-only after `loop_start` |
| `examples/screenshots/loop_round_types.rs:116` | open the round-type loop | `1` | **T** | advance-only after `loop_start` |
| `examples/screenshots/loop_round_types.rs:119` | hold the intact layers | `20` | **M** | encoded clip length |
| `examples/screenshots/loop_round_types.rs:123` | fire volley one | `75` | **M** | encoded clip length |
| `examples/screenshots/loop_round_types.rs:127` | fire volley two | `75` | **M** | encoded clip length |
| `examples/screenshots/loop_round_types.rs:131` | fire volley three | `75` | **M** | encoded clip length |
| `examples/screenshots/loop_round_types.rs:135` | fire volley four | `75` | **M** | encoded clip length |
| `examples/screenshots/loop_round_types.rs:139` | fire volley five | `75` | **M** | encoded clip length |
| `examples/screenshots/loop_round_types.rs:142` | hold the two outcomes | `45` | **M** | encoded clip length |
| `examples/screenshots/loop_torpedo_blast.rs:141` | open the loop | `1` | **T** | advance-only after `loop_start` |
| `examples/screenshots/loop_vfx_range.rs:311` | open the vfx loop | `1` | **T** | advance-only after `loop_start` |
| `examples/screenshots/loop_vfx_range.rs:318` | pass {pass}: lay the guns on | `12` | **M** | budgeted against the 600-frame recorder cap |
| `examples/screenshots/loop_vfx_range.rs:322` | pass {pass}: gun burst | `45` | **M** | budgeted against the 600-frame recorder cap |
| `examples/screenshots/loop_vfx_range.rs:326` | pass {pass}: cease fire, watch the rounds land | `30` | **M** | budgeted against the 600-frame recorder cap |
| `examples/screenshots/loop_vfx_range.rs:341` | close on the bay for the launch | `6` | **R** | camera move between two recordings |
| `examples/screenshots/loop_vfx_range.rs:345` | open the cold launch loop | `1` | **T** | advance-only after `loop_start` |
| `examples/screenshots/loop_vfx_range.rs:352` | pass {pass}: torpedo away | `if pass == LAUNCH_PASS {` | **M** | budgeted against the 600-frame recorder cap |
| `examples/screenshots/loop_vfx_range.rs:367` | pass {pass}: the rest of the flight | `TORPEDO_FLIGHT_FRAMES - LAUNCH_LOOP_FRAMES` | **O** | outside the recording: `not(torpedo_in_flight())` |
| `examples/screenshots/loop_vfx_range.rs:373` | pass {pass}: hold the aftermath | `30` | **M** | budgeted against the 600-frame recorder cap |
| `examples/screenshots/screenshot_damage_levels.rs:409` | open the damage-level loop | `1` | **T** | advance-only after `loop_start` |
| `examples/screenshots/screenshot_editor.rs:302` | show the authored colliders | `SETTLE_FRAMES` | **R** | collider gizmos on, then shoot |
| `examples/screenshots/screenshot_editor.rs:388` | hide sandbox collider diagnostics | `SETTLE_FRAMES` | **R** | collider gizmos off, then shoot |
| `examples/screenshots/screenshot_flip_burn.rs:96` | open the controller loop | `1` | **T** | advance-only after `loop_start` |
| `examples/screenshots/screenshot_gravity.rs:91` | frame feature-gravity.png | `SETTLE_FRAMES` | **R** | pose then shoot |
| `examples/screenshots/screenshot_gravity.rs:108` | frame wiki-gravity.png | `SETTLE_FRAMES` | **R** | pose then shoot |
| `examples/screenshots/screenshot_hero_ship.rs:87` | frame wiki-sections.png | `SETTLE_FRAMES` | **R** | pose then shoot |
| `examples/screenshots/screenshot_hull_juice.rs:140` | blow a section off the raider | `12` | **O** | `section_gone(..)` - the blow-off is a real pipeline |
| `examples/screenshots/screenshot_menu.rs:93` | settle the menu and its ambience backdrop | `SETTLE_FRAMES` | **R** | menu + ambience backdrop stillness, then shoot |
| `examples/screenshots/screenshot_menu.rs:105` | settle the settings panel | `SETTLE_FRAMES` | **R** | panel fade-in stillness, then shoot |
| `examples/screenshots/screenshot_menu.rs:112` | the settings panel is up | `1` | **T** | advance-only after a verdict |
| `examples/screenshots/screenshot_menu.rs:124` | settle the controls tab | `SETTLE_FRAMES` | **R** | tab swap stillness, then shoot |
| `examples/screenshots/screenshot_nova_os_apps.rs:67` | open the computer | `SETTLE_FRAMES` | **O** | `nova_os_openness() >= 1.0` already exists |
| `examples/screenshots/screenshot_nova_os_apps.rs:71` | type the map command | `6` | **O** | the terminal input line reads the command |
| `examples/screenshots/screenshot_nova_os_apps.rs:75` | launch the map app | `SETTLE_FRAMES` | **O** | `NovaOsTerminal::active_mode() == App { id }` |
| `examples/screenshots/screenshot_nova_os_apps.rs:88` | type the ship command | `6` | **O** | the terminal input line reads the command |
| `examples/screenshots/screenshot_nova_os_apps.rs:95` | launch the ship app | `SETTLE_FRAMES` | **O** | `NovaOsTerminal::active_mode() == App { id }` |
| `examples/screenshots/screenshot_nova_os_terminal.rs:67` | open the computer | `SETTLE_FRAMES` | **O** | `nova_os_openness() >= 1.0` already exists |
| `examples/screenshots/screenshot_nova_os_terminal.rs:74` | run the help command | `6` | **O** | `active_mode() == Prompt` with the output in scrollback |
| `examples/screenshots/screenshot_nova_os_terminal.rs:78` | run the ship view command | `6` | **O** | `active_mode() == Prompt` with the output in scrollback |
| `examples/screenshots/screenshot_nova_os_terminal.rs:84` | leave an inline-completion prefix | `SETTLE_FRAMES` | **O** | the input line reads the prefix |
| `examples/screenshots/screenshot_radar_lock.rs:222` | open the lock loop | `1` | **T** | advance-only after `loop_start` |
| `examples/screenshots/screenshot_scenario_picker.rs:113` | enable the bundled example mod | `SETTLE_FRAMES * 2` | **O** | the picker relists with the mod's rows |
| `examples/screenshots/screenshot_scenario_picker.rs:116` | settle the menu and its ambience backdrop | `SETTLE_FRAMES` | **R** | menu + ambience backdrop stillness |
| `examples/screenshots/screenshot_scenario_picker.rs:121` | settle the scenarios picker | `SETTLE_FRAMES` | **R** | picker fade-in stillness, then shoot |
| `examples/screenshots/screenshot_scenario_picker.rs:125` | settle the selected chapter's details pane | `SETTLE_FRAMES` | **R** | details pane stillness, then shoot |
| `examples/screenshots/screenshot_scenario_picker.rs:145` | the chapter is the selected row | `1` | **T** | advance-only after a verdict |
| `examples/screenshots/screenshot_scenario_picker.rs:156` | settle the example scenario details | `SETTLE_FRAMES` | **R** | details pane stillness, then shoot |
| `examples/screenshots/screenshot_section_drives.rs:277` | frame {path} | `SETTLE_FRAMES` | **R** | pose then shoot |
| `examples/screenshots/screenshot_section_frame.rs:152` | present {path} | `SETTLE_FRAMES` | **R** | present then shoot |
| `examples/screenshots/screenshot_section_gallery.rs:729` | frame the gallery | `SETTLE_FRAMES * 2` | **R** | pose then shoot |
| `examples/screenshots/screenshot_section_gallery.rs:742` | frame the {slug} row | `SETTLE_FRAMES` | **R** | pose then shoot |
| `examples/screenshots/screenshot_section_trials.rs:343` | frame the range and lay both guns | `12` | **R** | pose + lay guns, then a scarring wait |
| `examples/screenshots/screenshot_section_trials.rs:363` | close on the twin's two streams | `8` | **R** | pose then shoot |
| `examples/screenshots/screenshot_section_trials.rs:375` | cease fire, frame the bay | `8` | **R** | pose then shoot |
| `examples/screenshots/screenshot_section_weapons.rs:141` | present {path} | `SETTLE_FRAMES` | **R** | present then shoot |
| `examples/screenshots/screenshot_thruster_gallery.rs:762` | frame the gallery | `SETTLE_FRAMES * 2` | **R** | pose then shoot |
| `examples/screenshots/screenshot_thruster_gallery.rs:771` | frame the size family | `SETTLE_FRAMES` | **R** | pose then shoot |
| `examples/screenshots/screenshot_thruster_gallery.rs:780` | frame the shell candidates | `SETTLE_FRAMES` | **R** | pose then shoot |
| `examples/screenshots/screenshot_torpedo_run.rs:86` | settle the ordnance hollow | `12` | **R** | hollow stillness, then shoot |
| `examples/systems/stress_point_defense.rs:422` | open the point-defense loop | `1` | **T** | advance-only after `loop_start` |
| `examples/systems/stress_point_defense.rs:433` | hold the saturation | `hold_frames(` | **M** | the saturation clip's length / the perf window |
| `examples/systems/system_input_modes.rs:233` | modes: put the caret at the head of the name | `SETTLE_FRAMES` | **O** | the caret position is readable; the docstring counts four, not five |
| `examples/systems/system_input_modes.rs:246` | modes: press Delete with the caret in the field | `SETTLE_FRAMES` | **S** | the verdict is that NOTHING happened - no message to wait on |
| `examples/systems/system_input_modes.rs:274` | modes: press Delete with the keyboard free | `SETTLE_FRAMES` | **S** | the verdict is that NOTHING happened - no message to wait on |
| `examples/systems/system_input_modes.rs:319` | modes: release Escape again | `SETTLE_FRAMES` | **S** | the verdict is that NOTHING happened - no message to wait on |
| `examples/systems/system_input_modes.rs:436` | modes: press Delete while the capture waits | `SETTLE_FRAMES` | **S** | the verdict is that NOTHING happened - no message to wait on |
| `examples/systems/system_nova_os.rs:150` | nova_os: the computer is open | `1` | **T** | advance-only after a verdict |
| `examples/systems/system_nova_os.rs:154` | nova_os: type the ship command | `SETTLE` | **O** | the terminal input line reads `ship` |
| `examples/systems/system_nova_os.rs:163` | nova_os: the ship app owns the screen | `SETTLE` | **P** | 12 frames of padding AFTER the verdict already ran |
| `examples/systems/system_nova_os.rs:167` | nova_os: aim at the close control through the glass | `SETTLE` | **O** | `pointer_at(window_px)` - the aim's own ack |
| `examples/systems/system_nova_os.rs:171` | nova_os: the pointer reached the offscreen tree | `1` | **T** | advance-only after a verdict |
| `examples/systems/system_nova_os.rs:175` | nova_os: press the close control | `SETTLE` | **O** | `pointer_pressed()` - the WINDOW mouse, not the forwarded one |
| `examples/systems/system_nova_os.rs:179` | nova_os: the press landed on the widget | `1` | **T** | advance-only after a verdict |
| `examples/systems/system_nova_os.rs:192` | nova_os: the click through the glass closed the app | `SETTLE` | **P** | 12 frames of padding AFTER the verdict already ran |
| `examples/systems/system_nova_os.rs:196` | nova_os: type the map command | `SETTLE` | **O** | the terminal input line reads `map` |
| `examples/systems/system_nova_os.rs:205` | nova_os: the app switch left one screen | `1` | **T** | advance-only after a verdict |
| `examples/systems/system_torpedo_launch.rs:957` | frame the bay muzzle | `SETTLE_FRAMES` | **R** | pose then shoot |
| `examples/systems/system_torpedo_launch.rs:1034` | clear the correctness salvo | `2` | **O** | `not(any_entity::<With<Torpedo>>())` - the drain is observable |
| `examples/systems/system_torpedo_launch.rs:1043` | release the comparison pair | `2` | **S** | separation between releasing fire and the next launch |
| `examples/systems/system_torpedo_launch.rs:1056` | frame the terminal weave | `SETTLE_FRAMES` | **R** | pose then shoot |
| `examples/systems/system_torpedo_launch.rs:1060` | open the torpedo type loop | `1` | **T** | advance-only after `loop_start` |
| `examples/systems/system_ui_scale.rs:421` | scale: release Escape | `SETTLE_FRAMES` | **S** | key release with nothing to observe |
| `examples/systems/system_ui_scale.rs:441` | scale: double the scale factor | `SETTLE_FRAMES` | **O** | `Window::scale_factor()` reaching 2.0, then a layout pass |
| `examples/systems/system_ui_scale.rs:446` | scale: reframe at 2x | `SETTLE_FRAMES` | **R** | camera reframe before a measurement |
| `examples/systems/system_ui_scale.rs:459` | scale: back to 1x | `SETTLE_FRAMES` | **O** | `Window::scale_factor()` reaching 1.0, then a layout pass |
| `examples/systems/system_ui_scale.rs:464` | scale: go wide | `SETTLE_FRAMES` | **O** | `Window::resolution` reaching 1280x600, then a layout pass |
| `examples/systems/system_ui_scale.rs:469` | scale: reframe wide | `SETTLE_FRAMES` | **R** | camera reframe before a measurement |
| `examples/systems/system_ui_scale.rs:480` | scale: go narrow | `SETTLE_FRAMES` | **O** | `Window::resolution` reaching 760x600, then a layout pass |
| `examples/systems/system_ui_scale.rs:485` | scale: reframe narrow | `SETTLE_FRAMES` | **R** | camera reframe before a measurement |
| `examples/systems/system_ui_scale.rs:502` | scale: back to the stock shape | `SETTLE_FRAMES` | **O** | `Window::resolution` reaching 1024x768, then a layout pass |
| `examples/systems/system_ui_scale.rs:512` | scale: release Escape again | `SETTLE_FRAMES` | **S** | key release with nothing to observe |
| `examples/systems/system_ui_scale.rs:517` | scale: frame the whole scenario | `SETTLE_FRAMES` | **R** | camera reframe before a measurement |
