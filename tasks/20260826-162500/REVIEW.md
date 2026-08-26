# Review: the editor input and UI pass

- RANGE: `1028ac3c..53597374` (5 commits, 41 files, +3371/-303)
- TASKS: 20260826-162500 (review fixes), 20260826-162503 (modal input),
  20260826-162506 (the UI pass)
- BRANCH: master

Round 1 is `review-round-1.html` in this folder. It produced the three tasks
above. This is round 2, over what they landed.

## Round 2

- REVIEWER: six lanes (craft, performance, correctness, contracts, red team,
  feel). `--play`, so red team and feel ran.
- VERDICT: REQUEST_CHANGES

The range does what the three task bodies say, and the two hard parts are
right: `hang_at` really is the one place the logical/physical conversion
happens, and the arbiter really does replace the hand-kept denial lists. The
open findings are one defect in the headline control, and a set of places
where a new mode or a new clamp reaches further than its author checked.

The first finding is a BLOCKER because the control it breaks is the one the UI
task was written to add, it fails silently, and the range's own proof cannot
see it.

### Findings

- [x] R2.1 (BLOCKER) crates/nova_editor/src/inspect.rs:1187 - the scrub's
  travel and its snap grid come from two different lookups, so a slow drag
  either moves at twice the declared rate or stops moving. `on_inspector_drag`
  multiplies the frame's pixels by the ROW's declared step, which
  `spawn_axes_row` hands each axis grip as `InspectorDrag(row.nudge)` - for a
  pose row `POSE_STEP` (0.05). `nudge_field` then re-derives a step of its own
  from the WRITE path, which for an axis box ends at `PathStep::Field("x")`
  (`axis_step`, `:1560`). No `FieldSpec` is named or covers `"x"`, so
  `field_spec` answers `None` and the grid falls to `FREE_STEP` (0.1).
  The result is magnitude-dependent, because the write goes back through
  `number_text` and `parse_leaf` as an f32. Reproduced over the exact chain at
  one logical pixel per event:
  | start x | 1 px/event, right | behaviour |
  |-|-|-|
  | 0.0, 0.05, 0.1 | 0.1, 0.2, 0.3, 0.4 | walks at DOUBLE the declared step |
  | 0.6, 1.0, 2.5, 3.0 | 0.7, 0.7, 0.7 / 3, 3, 3 | STALLS after 0-1 steps |
  | 3.14159, 12.75 | 3.2, 3.3, 3.3 | one or two steps, then stalls |
  `0.05f32 / 0.1f32` is exactly 0.5, so each step lands on a half-grid point
  and `f64::round` (half away from zero) carries it back where it came from.
  It is not confined to the axis rows. `CursorMoved.position` is documented in
  LOGICAL pixels and `bevy_picking` derives `Drag::delta` from it, so at scale
  factor 2 a one-physical-pixel move is a 0.5 logical delta - half the grid for
  every row, scalars included. `Radius` at 0.5 px/event over 200 events does
  not move. The panel's numbers stop scrubbing on the display class the other
  half of this range exists to support.
  The same split reaches `check_floor` (`:544`), which also looks up the axis
  path, so a `Limit::AtLeast` declared on a vector field never reaches its
  components.
  Change: pass the step the grip already carries into `nudge_field` so one
  number drives both the travel and the snap, and accumulate sub-step deltas
  rather than discarding a move that rounded away.
  Untested: `a_scrub_lands_on_the_step_the_field_was_declared_with`,
  `dragging_a_rows_name_writes_the_number_into_the_document` and
  `system_field_controls` all scrub `radius`, the one row where both lookups
  resolve to the same `FieldSpec`. `system_field_controls` also teleports the
  cursor 40 px in a single `move_cursor`, which is one Drag event - the one
  cadence that works.

- [ ] R2.2 (MAJOR) crates/nova_editor/src/ui/inspector.rs:1456 - Bind mode does
  not take WASD off the editor camera, which the task body and the arbiter's
  own docstring both say it does. `WASDCameraController` is
  `bevy_enhanced_input`'s and no run condition reaches it. Three things remove
  it: `hold_camera_while_typing` (keyed on `TextFieldFocused`), the gallery
  park (`gallery/scene.rs:213`) and `frame.rs:206`. Nothing keys on
  `EditorRebind` or `InputMode`. Arm a rebind on a thruster and press `W` - the
  most natural thruster binding there is - and `W` binds to the section while
  the camera flies forward for as long as it is held, so the part the chip
  points at slides off screen mid-gesture. Insert and Browse are both parked;
  Bind, the most exclusive mode there is, is not.
  Change: extend the hold to any mode above Normal, one system reading
  `InputMode`, and delete the two hand-keyed copies.

- [ ] R2.3 (MAJOR) crates/nova_editor/src/ui/mod.rs:478 - at 760x600 the Ship
  menu button is drawn on top of the Play button. `Top Bar Left` is
  `flex_basis: px(0)`, `flex_grow: 1.0`, `min_width: px(0)`, with no
  `flex_shrink` and no clip, so it squeezes below the natural width of `EDITOR`
  plus five menu buttons and its children overflow. The breadcrumb wrapper on
  the other side got `Overflow::clip()` (`:607`) for exactly this reason; the
  left column did not. Rendered at 760x600 the bar reads `Pla`+`Ship`+`leave
  the ship)` - both labels unreadable, both hit areas overlapping.
  760x600 is the narrow landscape bound this task's own Done-when nominates,
  and `system_ui_scale` runs it - but asserts only the chip's gap and lead, so
  the frame was never looked at.
  Change: clip `Top Bar Left`, or give the menu row `flex_shrink: 0.0` and let
  the breadcrumb lose the pixels instead.

- [ ] R2.4 (MAJOR) crates/nova_ui/src/screen/float.rs:50 - `hang_at` clamps an
  anchor that is OUTSIDE the viewport into it, so a label for an off-screen
  node pins to the border. None of the three sites it replaced clamped.
  `Camera::world_to_viewport` errors only on `NoViewportSize`, `InvalidData`,
  `PastFarPlane` and `PastNearPlane` - there is no x/y range check - so a node
  in front of the camera but beside the viewport returns `Ok` with out-of-range
  coordinates, and the callers hide only on `Err`.
  Two consequences. A nameplate for a ship framed out of view now stands on the
  rail or over the Inspector, labelling a ship that is not on screen. And in
  `keybind.rs`, `clear_of` lifts a colliding chip by `CHIP_CLEARANCE_PX.y` and
  records the UNCLAMPED spot in `standing`, so a pile keeps walking above the
  viewport and `hang_at` clamps every one of them to `y = 0` - the chips land
  on one pixel row, which is the single outcome `clear_of` exists to prevent.
  The leader is still drawn from the unclamped `spot` (`keybind.rs:245`), so it
  no longer meets the chip's foot.
  Change: hide when the anchor is outside the viewport, or return that decision
  rather than silently overriding it; feed the clamped corner back into
  `standing` and derive the leader from it.
  `screen/tests.rs::float` covers an anchor NEAR an edge and a node bigger than
  the screen, never an anchor outside the viewport.

- [ ] R2.5 (MAJOR) crates/nova_editor/src/ui/inspector.rs:997 - a box left in
  the refused state is excluded from the repaint forever, so a scrub changes the
  document while the panel keeps showing the refused number. `TextFieldError` is
  inserted and removed in one place only, `apply_inspector_edits` (`:1329`,
  `:1332`), on a typed submit; `sync_inspector` repaints
  `Without<TextFieldError>`; `on_inspector_drag` neither writes through it nor
  clears it. Type `-5` into an asteroid's Radius and press Enter: refused, box
  red, `min 0` in the unit slot. Now drag the Radius row NAME 40 px right. The
  radius moves 3 -> 5 and the rock grows on the stage, and the box still reads
  `-5` in red. The only exits are a successful typed submit into that same box,
  or selecting another node. The panel is lying about what the document holds.
  Change: clear `TextFieldError` on the row a successful scrub writes.

- [ ] R2.6 (MAJOR) crates/nova_editor/src/ui/inspector.rs:1296 - every frame of
  a scrub on an object's config row despawns and rebuilds that object's whole
  preview body. `EditTargets::edit`'s `FieldRoot::Config` arm takes
  `objects.get_mut(field.node)`, which marks `ObjectNode` changed
  unconditionally - including when the edit is then refused.
  `drop_edited_views` (`node.rs:961`) watches `Changed<ObjectNode>` and
  despawns the `NodeView`; `sync_object_views` respawns it the same frame
  through `insert_preview_object`, and `sphere_body` mints a fresh mesh asset, a
  fresh material asset and a fresh Avian collider each time. Nothing is cached.
  Drag an asteroid's Radius grip for two seconds: about 120 mesh assets, 120
  materials and 120 collider rebuilds with their uploads. On a `Spaceship`
  object node it rebuilds the whole multi-section hull preview per frame.
  `drop_edited_views` predates this range and its docstring names the
  assumption that broke: "until the inspector existed nothing could change a
  config that already had a body". The scrub turns one commit per typed edit
  into one per frame.
  This compounds with R2.1: at one pixel per event the value does not move, and
  the hull is still rebuilt every frame of the gesture.
  Change: mark `ObjectNode` changed only when the edit took, and give
  `drop_edited_views` a reason to skip a change that did not touch the geometry
  the body is built from.
  Unmeasured. No probe range drives a held drag, so no capture would show it.

- [ ] R2.7 (MAJOR) crates/nova_editor/src/ui/mod.rs:2114 - the key legend
  advertises keys that Bind mode has taken, and Backspace binds itself instead
  of leaving the ship. `sync_key_legend` keys its hints on
  `(SectionChoice, inside)` alone; a rebind changes only the last cell. With a
  rebind armed the legend still reads `IN A SHIP ... Bksp leave ... Q pick a
  part`, but those verbs are now behind `in_input_mode(InputMode::Normal)` and
  `apply_section_rebind` captures every key except Escape. Press Bksp to leave
  the ship and it becomes the thruster's fire key instead, with nothing saying
  either thing happened. `config.rs:348` states the contract this breaks: "the
  keys that do nothing in the current mode are not listed".
  Change: give `sync_key_legend` a Bind arm, the way it already has one per
  tool.

- [ ] R2.8 (MAJOR) crates/nova_editor/src/lib.rs:169 - Ctrl+S goes quiet under
  the parts gallery and nothing on screen can say so. `save_key` moved from
  `not(typing_into_a_field)` to `in_input_mode(InputMode::Normal)`, and
  `declare_editor_keyboard_owner` claims `Browse` whenever the gallery is open.
  The gallery is `percent(100)` at `GALLERY_Z = 20` and the status line sits at
  `FOOT_Z = 15`, so the surface the editor speaks through is covered - a
  refusal could not be shown even if one were written. There is no
  unsaved-changes guard in `nova_editor`. Build for ten minutes, press Tab,
  press Ctrl+S out of habit, close the gallery and leave: nothing was written
  and nothing was said. The task records the decision but not that the silence
  cannot be broken while the gallery is up.
  Change: let `save_key` run under Browse - a save is not a keyboard-ownership
  question - or have the gallery answer Ctrl+S itself.

- [x] R2.9 (MAJOR) crates/nova_editor/src/inspect.rs:791 - an empty or unsigned
  optional number wears a grip that can only refuse, once per drag event, in
  words that name a Rust type. `walk_option` builds `RowValue::Number` off the
  payload TYPE, so a field holding `None` still gets a step and a grip.
  Two cases, both on the stock rock's first screen:
  - `AsteroidConfig::mass` is `Option<f32>` and reads `none`. `number_at` fails
    and `nudge_field` returns `"empty"` on every drag event of the gesture.
  - `AsteroidConfig::seed` is `Option<u32>` declared `Limit::Free`, so no clamp
    applies; the value walks 3, 2, 1, 0 and then every further event says
    `not a u32`.
  Both contradict this range's own stated decision - "a scrub ARRIVES at the
  floor; a typed number is refused by it" - and `EditorSays::refuse` requires a
  reason "phrased as the way out where there is one". `"empty"`, `"gone"` and
  `"not a u32"` are none of those.
  Change: seed an empty optional from its floor on the first pull, or withhold
  the grip and say `type a number first`; give a whole-number scrub the floor
  its type already carries rather than a second declaration per unsigned field.

- [ ] R2.10 (MAJOR) web/src/wiki/keybinds.md:316 - the player wiki still
  describes the pre-v0.12.0 editor, and this range - the last of the three
  input and UI tasks - touched no document at all. `:325` sends the player to a
  rail row named Parts that does not exist (`GalleryAction::Open` is a Ship
  menu row). `:365` names a "Tools block" that does not exist. The table lists
  no `Del`, `Ctrl+S`, `F`, `Backspace` or `F5`. This range then makes `Ctrl+S`
  and `Del` inert while the gallery is up, which is an undocumented change to
  keys a builder has memorised. Neither `web/src/wiki/` nor `web/src/create/`
  mentions the Inspector, so the new scrub gesture and its units are documented
  nowhere. `docs/keeping-docs-in-sync.md` rule 2 says this lands in the same
  task.
  Not higher: no shipped format changed, and most of the rot landed earlier in
  the cycle. This is the last range before release.

- [ ] R2.11 (MINOR) crates/nova_os_ui/src/map/scene.rs:413 and
  crates/nova_os_ui/src/ship/scene.rs:569 - before layout the new unit
  conversion turns "cull every blip" into "stack every blip in the corner,
  visible". `inverse_scale_factor()` is 0 for an unmeasured node - this repo
  documents that itself at `crates/nova_autopilot/src/input.rs:321` - so `size`
  collapses to zero AND every projected point becomes `Vec2::ZERO`, which then
  PASSES `p.x >= 0.0 && p.x <= size.x`. Previously the projection kept its
  physical coordinates and the same filter rejected them. One frame of every
  contact piled on the panel's top-left corner, each time the map opens.
  Change: treat `inverse_scale_factor() <= 0.0` as "not measured yet" and
  return early.

- [ ] R2.12 (MINOR) crates/nova_editor/src/lib.rs:226 - the mode resolves one
  frame after the state that defines it, and the ordering edge that used to
  cover the gap was deleted. `declare_editor_keyboard_owner` runs in `PreUpdate`
  off `EditorRebind`, which `on_rebind_action` writes in `Update`, so on the
  arming frame the mode is still `Normal`. In that frame `escape_backs_out`
  (`in_input_mode(Normal)`) and `apply_section_rebind` (`owns_or_enters(Bind)`)
  both run, with no ordering between them and with the old
  `rebind.target.is_some()` check removed - one Escape cancels the capture AND
  puts the armed part down. The comment at `:228` says one press cannot reach
  two owners; that is true from the frame after the claim, not on it.
  Change: write the `Bind` claim where the arming happens, so the mode is never
  a frame behind its state.
  Not higher: the window is one frame and needs a click-release and an Escape
  inside it.

- [ ] R2.13 (MINOR) examples/systems/system_input_modes.rs:262 - the Insert
  beat cannot have reached the field it credits. `press_key` writes
  `ButtonInput<KeyCode>` alone, while `text_field_keyboard` reads
  `MessageReader<KeyboardInput>` - which is why the Escape beat presses both
  halves. So the Delete never reaches the focused Name field: the beat proves
  only that no verb answered, and its log line "the field took Delete"
  describes something that did not happen. Gate `text_field_keyboard` on a mode
  it can never hold and the beat, its assert and its `outcome:` marker all stay
  green.
  Change: pair the press with `press_edit_key(Key::Delete)` and assert the Name
  row lost a character.

- [ ] R2.14 (MINOR) crates/nova_editor/src/gizmo.rs:382 - `GizmoReach` is keyed
  on a scale nothing can change. Sub-task 6 added `scale` to the cache key
  because "the Inspector's Scale field resizes a node with the rig still up".
  There is no Scale field - `pose_rows` builds Position and Rotation only, and
  `inspect.rs:1495` says there will not be one. What does resize a node is its
  config (an asteroid's `radius`, a crate's `size`, a beacon's `radius`), all of
  which rebuild the mesh and leave `Transform.scale` at `Vec3::ONE`. Drag a
  rock's Radius to 13 and the gizmo arms stay sized for radius 3 until the
  selection changes.
  Change: key the reach on the measured `ColliderAabb`, or invalidate on
  `Changed<ObjectNode>`.

- [x] R2.15 (MINOR) crates/nova_editor/src/inspect.rs:1188 - `nudge_field`
  writes unconditionally, so a nudge that lands on the value it started from
  still writes - and still triggers R2.6's rebuild. `on_inspector_drag` guards
  only `by == 0.0`, not "the snapped result equals what is held". Under R2.1
  this is most of a slow gesture.
  Change: compare `moved` against `held` and return `Ok(())` when they match.

- [ ] R2.16 (MINOR) crates/nova_editor/src/inspect.rs:1316 - `curate` now
  allocates a `Vec<String>` plus a `pretty()` `String` per retained row, where
  it used to build one heading list per call. `sync_inspector` runs every frame
  with no change gate, so a `Spaceship` node with an inline hull allocates
  hundreds of small strings per idle frame.
  Note the obvious fix is wrong: `kept` is NOT the old loop-invariant. It is
  built from `row.path`, so hoisting it back out would change which headings
  survive. Compare `row.group` segments against the specs directly instead and
  drop the intermediate `Vec` entirely.

- [ ] R2.17 (MINOR) crates/nova_editor/src/lib.rs:512 - `wheel_placement_pose`
  keeps a dead `not(gallery_open)` guard on top of its mode gate.
  `in_input_mode(InputMode::Normal)` is already false whenever the gallery is
  open, because `declare_editor_keyboard_owner` claims `Browse` for exactly
  that state. This is the last verb still carrying a hand-written denial list -
  the shape the input task set out to delete. The chain-level guard at `:502`
  is legitimate; it gates the draw half.

- [ ] R2.18 (MINOR) crates/nova_editor/src/lib.rs:530 - the comment states the
  opposite of the arbiter's resolution rule. It says "everything less takes the
  keyboard off it"; `resolve_input_mode` keeps the GREATEST claim and
  `Browse < Bind`, so it is Bind that takes the keyboard off Browse. No
  player-reachable path into the state was found, so this is a wrong comment
  rather than a defect - which is what the house comment rule exists to stop.

- [ ] R2.19 (MINOR) crates/nova_editor/src/ui/plate.rs:257 - nameplates pile on
  each other; the keybind chips they now share a placement loop with do not.
  `sync_nameplates` calls `hang_at` per plate with no de-collision, while
  `position_section_keybind_labels` has `clear_of` for exactly this. Rendered at
  1024x768 in the stock range, `Derelict Hulk 1` and `Derelict Hulk 0` overlap
  into `Derelict Hulk 1|t Hulk 0`, and `Derelict Hulk 3`/`2` do the same.
  Change: lift `clear_of` into `nova_ui::screen` beside `hang_at` and let both
  callers use it.

- [ ] R2.20 (MINOR) crates/nova_editor/src/ui/window.rs:444 - the colour picker
  lands on the rail at a narrow width. `left = (size.x - RIGHT_MARGIN -
  WINDOW_W).max(8.0)`; at 760 logical px that is 148, and the window is 300
  wide, so it spans 148..448 against a rail of 0..210 - a 62 px overlap at
  `WINDOW_Z = 30` over `CHROME_Z = 10`. This range moved the picker off a
  literal and onto the panel's width; it did not give it a left bound.
  Not higher: the window is draggable.

- [ ] R2.21 (MINOR) crates/nova_editor/src/ui/mod.rs:908 - the key legend wraps
  to five rows over the narrow stage. `FlexWrap::Wrap` with eight cells and a
  stage band 250 px wide at 760x600 fills the bottom sixth of the buildable
  area and draws through the axis rose. Not higher:
  `Pickable { should_block_lower: false }` keeps it from eating the placement
  ray, so it is clutter rather than a block.

- [ ] R2.22 (MINOR) docs - three chapters the range invalidated.
  `docs/development.md:255` enumerates 25 `systems/` ranges; disk now holds 28,
  and none of `system_ui_scale`, `system_field_controls` or
  `system_input_modes` appears anywhere in `docs/`.
  `docs/automation-harness.md:158` omits `pointer_at` from the predicate
  vocabulary - and task 20260826-162500 records that its absence is what let
  `bug_sandbox_soak`'s founding click land on the panel it had just closed.
  `docs/architecture.md:29` enumerates every module of `nova_ui` and does not
  name `input_mode`, which is app-global and publishes a cross-plugin ordering
  handle.

- [ ] R2.23 (MINOR) examples/systems/system_ui_scale.rs:108 - the two ranges
  added in this range carry byte-identical copies of the same helpers.
  `part_on_screen` and `aim_at_a_section` differ only in their name, as do
  `inside_a_ship`, `the_ship_is_up` and four constants, across
  `system_ui_scale.rs`, `system_input_modes.rs` and `bug_sandbox_soak.rs`. The
  cost is already on the record inside this range: `0b617f5a` had to correct
  the soak's founding click from `(760, 640)` to `(460, 660)`, and
  `system_ship_editor.rs:240` still holds a third literal the same reasoning
  applies to. `system_turret_gunnery.rs:40` shows the repo already has the
  `#[path = ...]` mechanism for sharing.
  Also: `part_on_screen`'s docstring says "of the edited ship" while the query
  sweeps every `SectionMarker`; it is correct only because `sync_ship_focus`
  hides the other ships, and only the `system_input_modes` copy says so.

- [ ] R2.24 (MINOR) crates/nova_ui/src/screen/float.rs:18 - `Hang::above` and
  `Hang::below` each abstract exactly one caller, and the third caller
  (`keybind.rs:58`) builds the struct literal because neither fits. Fold to the
  literal, or to one constructor taking the alignment.

- [ ] R2.25 (MINOR) crates/nova_editor/src/lib.rs:655 - `escape_backs_out`'s
  docstring is welded to `backspace_steps_out` and this range rewrote it in
  place without noticing. Lines 655-662 document Escape's ladder; line 663
  starts "Backspace steps OUT one level" and the item they sit on is
  `backspace_steps_out` (`:671`). `escape_backs_out` (`:677`) has no docstring.
  The misattachment predates the range; the edit that passed through it does
  not.

### Verified

- The `hang_at` extraction is correct where it is used, and the two opposite
  conversions in one commit are both right: `Camera::world_to_viewport` answers
  logical, `ComputedNode::size()` is physical, and the NOVA OS scene camera
  draws into an image sized from `computed.size()`, so the projection really is
  physical there. Cross-read against `bevy_camera-0.19.1`.
- The arbiter replaces the denial lists it set out to replace. Every keyboard
  consumer in `nova_editor` was enumerated and gated, except the
  `bevy_enhanced_input` camera rig (R2.2). `typing_into_a_field` is gone with
  all nine call sites.
- A scrub past a declared `Limit::AtLeast` floor ARRIVES at it while a typed
  value below it is refused - the two gestures do differ as designed.
- `nan` and `inf` cannot reach a pose field. `check_finite` runs before
  `check_floor` on both branches; a scrub cannot mint one; a huge scrub
  round-trips into a value that parses back non-finite and is refused.
- The Escape ladder, rung by rung: each is owned by one mode, and
  `just_pressed` clearing in `PreUpdate` stops a rung being answered twice -
  except on the arming frame (R2.12).
- A drag that starts on a row name and ends elsewhere keeps resolving the same
  grip; a scrub on a node despawned mid-drag answers "gone" with no panic.
- Reentry: `OnEnter(Editor)` resets every editor resource, and the
  `Local`-guarded reconcilers clear on their `Added<Marker>`.
- No shipped format changed. No runtime string id was renamed. No generated
  `*.content.ron` was hand-edited. No `std::time`, `std::thread` or blocking IO
  was added; `cargo check` for `wasm32-unknown-unknown` on the three touched
  crates is clean, and so is a default-features `--all-targets` check.
- `CHANGELOG.md` measured against `v0.11.0`: all 51 `[Unreleased]` entries are
  within 200 characters joined, the two edits are collapses rather than
  additions, and the fixes to code that never shipped correctly get no entry.
- Ranges run: `system_field_controls`, `system_ui_scale` and
  `system_input_modes` all pass, and `catalog_drift` is 2/2 with
  `SYSTEMS_INVARIANTS` 174 -> 182.
- Lib tests: `nova_ui` 48, `nova_editor` 319.

### Not checked

- The workspace test suite and Clippy. Forbidden by the reviewer contract; CI
  owns both.
- `cargo run content lint`. Nothing in the range touches `assets/`,
  `nova_authoring` or a content builder, so it has no input to disagree about.
- A real HiDPI display. Scale factor 2 is exercised only through
  `set_scale_factor_override`, so R2.1's HiDPI half is derived from the
  documented units of `CursorMoved.position`, not observed.
- Bind mode was never driven live - no walk arms a rebind - so R2.2 and R2.7
  are code-grounded, not seen in a frame.
- R2.6 is unmeasured. No probe range drives a held drag, so no capture would
  show it.
- NOVA OS blip placement (R2.11) was read, not rendered.
- Widths between 760 and 1024 were not swept, so the width at which R2.3 starts
  is unknown. Portrait is out of scope by the task's own bound.
- `bug_sandbox_soak`, `system_ship_editor`, `system_menu_boot` and
  `system_nova_os` were not re-run.
- The euler round-trip in `EditTargets::edit`'s Rotation arm near +/-90 degrees
  of pitch - a pre-existing decomposition hazard the continuous scrub makes
  easier to reach, which no lane could ground in a concrete wrong result.

### Note for a re-run

The `system_ui_scale`, `system_field_controls` and `system_input_modes`
binaries in `target/debug/examples/` are PRE-COMMIT builds, and the
`system_ui_scale` one is the task's deliberately mutated build - it fails the
2x beat with `the chip hangs 47.5 over its part instead of 24`. Rebuild from
the tree before trusting any of the three.

### Verdict rationale

REQUEST_CHANGES on R2.1 alone. It is the control the UI task exists to add, it
fails silently, its failure depends on where the node happens to sit, and it
gets worse on exactly the displays the other half of the range was written for.
The range's own proof cannot see it, because that proof scrubs the one row
where the two lookups agree and moves the pointer in one 40 px jump.

R2.2 and R2.7 are the same gap seen twice: the arbiter covers the systems it
can gate, and the two things it cannot gate - an external input rig and a
legend that reads state directly - were not brought along. R2.3 is the
responsiveness half missing its own Done-when at the width that task nominated,
which happened because the range asserted a number at that size without looking
at the frame.

Four of the six lanes found R2.1 independently and rated it MAJOR twice,
BLOCKER once. It is raised to BLOCKER here because the HiDPI reach - which only
one lane had - takes it from a wrong step size on pose rows to a control that
stops working across the whole panel.

## Fixes

One commit per group, each ticking its own findings above.

### One step drives both travel and snap - R2.1, R2.9, R2.15

The row now carries its `Limit` beside its step, and both reach the grip as one
`DragRule`. `nudge_field` takes that rule plus a COUNT of steps, so nothing is
resolved a second time from an axis path where `x` matches no declaration.

- The grip accumulates pixels that do not reach a whole step, which is what a
  HiDPI screen hands it: one physical pixel is half a logical one at 2x, and
  the old truncation threw every frame of the drag away there.
- A whole UNSIGNED field takes the floor its TYPE carries, so a seed walks
  3, 2, 1, 0 and stops instead of saying `not a u32` on every further pixel
  (R2.9).
- A move that lands on the value it started from writes nothing (R2.15).
- The two refusals a scrub can reach are phrased as the way out rather than as
  the Rust word for the hole: `type a number here first`, `that field is gone -
  pick the node again` (R2.9).

A drag that reaches the window edge now WRAPS the pointer to the other side, so
a 0.05 step is not bounded by one screen of travel. `bevy_picking` measures its
delta from the last cursor position it saw, so the warp arrives as a move of its
own and the grip takes exactly that much back on the next frame.

Proof:

- `cargo test -p nova_editor --lib` - 324 pass, five new: an axis scrub moving
  by its row's step, a half-pixel scrub that steps on the second half, the wrap,
  a scrub easing off an edge that does NOT wrap, and an unsigned scrub arriving
  at zero.
- Mutation check: snapping on `FREE_STEP` instead of `rule.step` takes the axis
  test down with `one pixel is one step of 0.05 (got 3)` - the stall itself.
- `system_field_controls` gained the beat the old proof was missing: a pull on
  `Position X`, the row whose step had nowhere to be looked up from. Live under
  Xvfb, `X went -45.625 -> -43.6 on a 40px pull`, cycle complete.
- `system_ship_editor` re-run live, cycle complete.

