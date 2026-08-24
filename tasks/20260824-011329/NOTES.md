# Wait on the editor's state - what was built, and what the evidence says

## The premise the task opened on was already half fixed

The task's "Why" section says `system_ship_editor` dies on CI at
`editor: raise a tower, first course: it built`. It does not, and it did not
here either: the range was run FIRST, unchanged, at `fd7955e2` under
`probe run system_ship_editor --render sw --correctness-only`, and it passed -
`OK, measured 6/8, 93s`, 1387 frames, every check green.

That is consistent with the range's own `SETTLE` doc comment, which the task
quotes around: the `raise a tower` failure "was never about time", it was
`aim_at_a_visible_face` picking a different face from one run to the next
because the section list came back in archetype order, and it was fixed at the
source in `placed_sections` (the `by_pose` sort). The task text predates that
fix; the doc comment does not.

So this task's remaining work is the STRUCTURAL half it also asks for, and the
CI-fix framing is stale. Nothing here is a bug fix. What it buys instead:

- the run stops depending on frame counts nobody can justify;
- a beat that goes wrong now fails AT that beat, naming it;
- and the run got three times faster, because a condition is satisfied when the
  app is ready rather than when a fixed number of frames have gone by.

## What was added

`nova_editor` now publishes ONE public, read-only resource, refreshed in
`PostUpdate` and never read back by the editor:

```rust
pub struct EditorProbe {
    pub tool: EditorTool,             // Select | Place(id) | Delete
    pub placement: EditorPlacement,   // None | Solved { prototype, target } | Refused { prototype, reason }
    pub gallery_open: bool,
    pub filter_focused: bool,
    pub selected: Option<String>,     // what Enter would focus, through the live filter
}
```

Solver internals stay `pub(crate)`: `Refused` carries the same `&'static str`
the placement status line shows, so a beat asserts on the words the builder is
told rather than on a private enum. Outside the editor scene the snapshot is the
default, so nothing can read a build that is no longer on screen.

Two invariants make `placement` mean what it says - "what a click would build
RIGHT NOW" - rather than "the last thing the solver said":

- the preview is REBUILT every frame. `clear_placement_preview` runs ungated
  ahead of the solver, so a frame the solver was skipped on (the gallery is up,
  or it is coming down) has no answer at all rather than the build view's last
  one. See R1 in the review section.
- a placement is only ever published FOR the tool in hand, which closes the same
  hole when something changes the tool later in the same `Update` than the solve
  - Escape putting the part down, the gallery arming a different one on its way
  out.

`nova_autopilot::predicate` gained the four generic waits the gestures needed:
`or` (the missing combinator - `and`/`not` were there), `ui_node_present`,
`pointer_pressed`, `pointer_released`. `ui_node_rect` now takes `&World` so one
resolve backs both the gestures and the predicate.

`nova_debug::harness` wraps `EditorProbe` as nine Nova-typed predicates
(`editor_tool_is`, `editor_part_armed`, `editor_placement_solved` / `_refused` /
`_clear`, `editor_gallery_open` / `_closed`, `editor_filter_focused`,
`editor_gallery_selected`). That crate is where the Nova-typed predicates
already live; `nova_autopilot` depends on `bevy` alone and cannot hold them.

## What the proxies became

| Was | Is |
| --- | --- |
| "arming proven only by the next click" | `editor_tool_is(Place(id))` / `editor_part_armed()` |
| `subtree_text(world, "Placement Status")` scraped for a refusal | `EditorPlacement::Refused { reason }` - and the status line is still asserted, as the second claim |
| gallery-open proven by a named node's rect | `editor_gallery_open()` / `editor_gallery_closed()` |
| "type the id and hope the grid narrowed" | `editor_gallery_selected(id)` |
| `frames(SETTLE)` between press and release | `pointer_pressed()` / `pointer_released()` |
| `frames(SETTLE)` before a widget click | `ui_node_present(name)` |
| `frames(SHIP_SETTLE)` for the preview + its colliders | the ship's section count, then the first aim's `editor_placement_solved()` - which cannot hold before avian has prepared the collider the ray must hit |
| `frames(SETTLE_FRAMES)` after re-posing the build camera | `the_build_camera_is_posed()` - the camera has REACHED `EDITOR_EYE` |

## Evidence

All runs are `probe run <name> --render sw --correctness-only` on this machine,
the same lavapipe path CI uses. The AFTER column is the CLEAN-tree run at the
correction commit `2f839ba1`; the artefacts are preserved in `proof/`, and each
one carries that SHA in its own `run.full_git_sha`.

| Range | Before | After (`2f839ba1`) |
| --- | --- | --- |
| `system_ship_editor` | OK, 6/8, 93 s, 1387 frames | OK, 6/8, **30 s**, 180 steps |
| `system_menu_boot` | not measured | OK, 6/8, 11 s |
| `bug_sandbox_soak` | not measured | OK, 6/8, 54 s (45 s of that is the soak itself) |
| `screenshot_menu` | not measured | OK, 6/8, 18 s |
| `screenshot_scenario_picker` | not measured | OK, 6/8, 32 s |
| `screenshot_editor` | not measured | OK, 6/8, 42 s |

Only `system_ship_editor` was measured BEFORE the change - it is the one the
task names, and a before-run of the other five would have meant reverting a
dirty shared checkout. Their after-runs are all green, which is the claim that
matters for them.

Every claim the editor range makes still lands, with the same figures as the
93-second run: 8 sections, 7 mates, one connected structure, both refusals in
the editor's own words AND on the status line, and the flown ship re-deriving
the same 7 mates in 26 plates. All 17 `outcome:` markers are in the timeline;
the roster in `crates/nova_probe_cli/tests/catalog_drift.rs` is untouched
because no slug changed. `proof/README.md` lists all of it.

The capture path was run too, because a correctness run walks the shot beats
without writing anything and would not catch a figure that came out wrong:

```text
NOVA_CAPTURE_DIR=... NOVA_AUTOPILOT=1 NOVA_CAPTURE=1 \
  cargo run --example screenshot_editor --features debug
```

It wrote all five artefacts (`feature-editor.png`,
`news-0110-collider-before/after.png`, `wiki-sandbox-range.png`,
`landing-editor-build.webm`, `news-0110-editor-skin.webm`) and both stills were
opened: `feature-editor.png` shows the clad five-section ship - controller, two
hulls, thruster, PDC - with no ghost and no socket clutter, and
`wiki-sandbox-range.png` shows the same ship flying the asteroid range.

The PRESSED branch of `pointer_pressed` is not unit-tested: `PointerPress`'s
fields are private and it has no constructor, so only `bevy_picking`'s own
systems can set one. It is proved by the driven ranges instead - every press
beat in four walks holds on it, and a predicate that never became true would
deadline every one of them. The pointer-IDENTITY half IS unit-tested (see the
review section below), because a foreign pointer's parked state is stageable
where a press is not.

## What was deliberately NOT deleted

The task's audit lists two more constants "of the same shape". Both are
deadlines or stillness, not gesture settles, and deleting them would be the
opposite of what the task asks for:

- `STEP_DEADLINE_SECS = 30` (`examples/screenshots/shared/ui_walk.rs`) is a
  DEADLINE. "Both then fail by DEADLINE, naming the beat" is the goal; this is
  the constant that does it, and the same name is the fleet-wide convention in
  `playable/` and `showcase.rs`. Kept, and now carried by every beat in the kit.
- `SETTLE_FRAMES = 30` (`crates/nova_debug/src/harness.rs`) is the pre-SHOT
  stillness figure, shared by ~20 non-editor capture ranges, and there is no
  editor state to wait on for "the renderer has settled". It stays, and it is
  now the ONLY frame count left in the editor walks - three sites in
  `screenshot_editor`, all in a capture's run-up.

Out of scope and untouched: `system_nova_os.rs` and `widget_zoo.rs` carry their
own `SETTLE` constants, and `crates/nova_autopilot/tests/pointer_pin.rs` uses
one to pace its rig. `screenshot_menu.rs` and `screenshot_scenario_picker.rs`
each still carry one tautological `until(frames(1))`. None is an editor range,
none is on the task's list, and no task was filed for them; the shared kit's own
module doc now claims only what the KIT does, so nothing here documents them
away.

`SETTLE` and `SHIP_SETTLE` are gone from all three ranges that had them
(`system_ship_editor`, `bug_sandbox_soak`, `system_menu_boot`), and
`GESTURE_FRAMES` is gone from `ui_walk`. `system_ship_editor` and
`system_menu_boot` now contain no `frames(..)` call at all.

## One rule this bends, on purpose

`docs/automation-harness.md` says a beat must be strictly WEAKER than the assert
that follows it, so a regression surfaces as the assert's message rather than as
a deadline on the beat's name. Several beats here share their assert's quantity
("the socket filled", "it landed"), because the count is the only observable
there is - the same trade `place_on_the_face` already made and documented before
this task. The doc now carries the exception and the reason: an unmet `until`
names the gesture that missed, where a snapshot assertion several beats later
reported a wrong number and left the cause to be guessed. The asserts stay,
because that is where the claim is written down.

## Review round 1 - reviewer `a4a616969f93`, against `213aef62`

Six findings, all corrected in `2f839ba1`. Two were real contract defects in the
new public surface; the review was right about both.

### R1 (major) - the probe could publish a solve from before the gallery opened

CONFIRMED and fixed. The solver is gated on the gallery being CLOSED and ordered
`.before(gallery_keyboard)`, so on the frame a keystroke arms a part and takes
the overlay down it does not run at all - and `PlacementPreview` still held the
build view's last answer, from a different pointer position, a different camera
and possibly a different part. The `PostUpdate` snapshot then republished it.

Fixed at the resource, not at the snapshot, so every consumer benefits:
`clear_placement_preview` (`placement.rs`) runs UNGATED ahead of the solver, so
the invariant is now "the answer is rebuilt every frame, and a frame with no
solve has no answer". The snapshot additionally publishes a placement only for
the tool IN HAND, which closes the same hole for an `Escape` that disarms after
the solve - `escape_puts_down_the_armed_part` has no ordering against the
placement chain either.

The reviewer also objected, correctly, that the old test blessed the stale value
by asserting a re-closed gallery makes the same solve valid again. That test is
gone. In its place:

- `a_gallery_close_publishes_no_placement_from_before_it_opened` - a
  SCHEDULE-level test. It builds the editor's real shape (ungated clear, gated
  solve, both `.before(gallery_keyboard)`, snapshot in `PostUpdate`) and drives
  the real `gallery_keyboard` with a real Enter press. It carries its own
  delivery guard (the rig publishes a solve on an ordinary frame) and asserts
  the preview itself is cleared, not merely hidden by the snapshot.
- `a_placement_for_a_part_nobody_is_holding_is_not_published` - the tool
  coherence guard, over Select, Delete and a different armed part.

Proved to bite: with `clear_placement_preview`'s body removed the schedule test
fails with `left: Solved { prototype: "hull", target: PLACEHOLDER }, right:
None` on the gallery-open frame - the reviewer's defect exactly.

### R2 (major) - `ui_node_present` proved existence, not layout

CONFIRMED and fixed. `ComputedNode::default()` is zero size at the origin - the
pre-layout value, and what a `Display::None` node keeps - and `ui_node_rect`
accepted it. A beat could advance on a spawned-but-unsized node and hand
`click_named` a degenerate rect at the window corner, which is the race the
settles existed for. The old test constructed exactly that and asserted `true`.

`ui_node_rect` now filters to a finite box with area (`is_a_target`), so both
the predicate and every gesture that resolves through it reject a node with no
place on screen. The test asserts the opposite of what it used to: unsized is
NOT a target, and once a real `ComputedNode { size, inverse_scale_factor }` and
a translated `UiGlobalTransform` are installed it is - and `ui_node_centre`
resolves to that box's centre rather than the origin.

### R3 (moderate) - the pointer acks could be answered by the wrong pointer

CONFIRMED and fixed. Both acks now require `PointerId::Mouse`, which is the
pointer the gestures synthesize. The concrete hazard the reviewer named is real:
`crates/nova_os_ui/src/terminal/spawn.rs` parks a `NovaOsForwardedPointer`, and
its `PointerPress` defaults to released - so `pointer_released()` could pass
while the mouse press was still unprocessed.

New test `another_pointer_does_not_answer_for_the_mouse`: a foreign
`PointerId::Touch(1)` alone satisfies neither ack; adding the mouse is what
makes the released ack true. (The PRESSED direction is still unstageable -
`PointerPress` has no constructor - so the multi-pointer test covers the
released half, which is the half the parked pointer actually breaks.)

### R4 (moderate) - the screenshot walk did not wait for the Select tool

CONFIRMED and fixed. `editor_placement_clear()` is true when nothing is armed OR
when nothing is under the pointer, so a missed Select passed as soon as the
pointer reached empty space, with the part still in hand and its link-point
clutter still eligible for the figure. `screenshot_editor` now holds on
`editor_tool_is(EditorTool::Select)` between the click and the park - the seam
`EditorTool` was exposed for.

### R5 (minor) - tautological `frames(1)`, and a doc claim wider than the truth

CONFIRMED and fixed. The driver already gives every entry action its own frame
and polls no predicate until the next one, so `until(frames(1))` waits for
nothing. All four in `screenshot_editor` are gone.

The over-wide claim was mine: `ui_walk.rs`'s module doc said the only frame
counts left in "these walks" were pre-shot stillness, while
`screenshot_menu.rs` and `screenshot_scenario_picker.rs` - which include the kit
but are outside this task's lane - each still carry one. The doc now claims only
what the KIT does, and the two survivors are recorded above rather than
documented away.

### R6 (minor) - the proof was not bound to the implementation commit

CONFIRMED and fixed. The first round's runs were made on a dirty tree, so probe
stamped them with the base SHA `fd7955e2` and they could not prove they
exercised `213aef62`. Everything below was re-run on a CLEAN tree at the
correction commit `2f839ba1`, and the commit-bound artefacts are preserved with
the task under `proof/` rather than left in ephemeral scratch space.

## CI, and the close

Run 32736838936 at `261d3695` is green on all eight jobs - `probe / systems`
at 17.9 min is the bullet this task's Done-when names, and the other two probe
shards passed with it. That is the first CI-side proof of the state waits: the
shard the task opened on had failed at `raise a tower` under the same
lavapipe path.

One follow-up landed on top of the range work: R2's new doc comment on
`ui_node_present` wrapped an aside onto a line starting with `- `, which
`clippy::doc_lazy_continuation` read as an unindented list continuation and
which failed the run at `b9dcb68f`. Reworded, not indented - it was never a
list. Fixed in `261d3695`.
