# Review: the nova_input range

- RANGE: `e0a5092a..155afa73` (25 commits, 11,404 code lines)
- TASKS: 20260820-174148 (this one; steps 1 through 3.6), 20260824-120527 (the
  Settings rebind screen, closed)
- BRANCH: master

Round 1 is `findings.html` in this folder, published as
<https://claude.ai/code/artifact/4c67ebc2-8ffe-4bff-b174-6834e0a20f81>. It is
the reasoning; this record is the disposition.

## Round 1

- REVIEWER: six lanes (craft, performance, correctness, contracts, red team,
  feel). `--play`, so red team and feel ran.
- VERDICT: REQUEST_CHANGES - 3 blockers, 22 majors, 30 minors.

Nothing was found in the architecture. `nova_input` is a real leaf, the context
rule holds under audit, the registry hot path is `HashMap` lookups, and the
frame is render-bound with the whole main world at about 5 ms. Every blocker
and most majors sat in the surfaces built on top of it, and three of the major
clusters were the same omission made at three different rebind surfaces.

### Findings

Fixed in `9d224349`, `1a79bf21`, `819d5f06`, `489e68ab` and `7df3e75d`.

- [x] B1 (BLOCKER) `crates/nova_ship/src/input/bindings.rs:255` - the pinned
  readout test was red on master: the tip commit gave `rcs_aim` a left stick
  and left the expectation on `Unbound`.
- [x] B2 (BLOCKER) `crates/nova_menu/src/settings.rs:50` - an armed rebind
  outlived the panel and took the next key pressed anywhere.
- [x] B3 (BLOCKER) one menu test un-isolated the config store for the rest of
  the process, so the developer's own `settings.ron` was what the suite
  asserted on.

The capture has no owner - nothing scoped an armed rebind to the thing that
armed it:

- [x] M1 Escape cancelled the capture AND closed the pause overlay in one
  press. It now raises `EscapeOwner`, as both other capture surfaces already
  did.
- [x] M2 The refusal rendered below the fold. The group bar and the refusal
  moved into the strip that does not scroll.
- [x] M3 The ship viewer captured Left Mouse on a click-driven surface, and
  armed for one frame rather than waiting for release.
- [x] M4 `captured_pad()` handed out Start, the fixed pause chord, which no
  registry row can undo.
- [x] M5 Two older captures picked an arbitrary key out of a `HashSet`. All
  three capture surfaces now go through `InputSources::captured_desk`, which
  takes `.min()` and breaks a same-frame tie the same way everywhere.

The table could go inconsistent behind your back:

- [x] M6 The rebind guard never saw live section bindings, so Main Drive could
  be bound to the turret trigger the ship was flying with.
- [x] M7 `apply_overrides` ran no conflict check on load. It now applies every
  override and reconciles the WHOLE table, so a legitimate key swap survives
  and a stored row a moved default landed on is put back.
- [x] M8 The whole-table guard covered 32 of 33 actions - `scenario_bindings`
  was not chained in.
- [x] M9 The editor's pad capture read a resource bevy 0.19 does not register.

A rebind did not reach everything:

- [x] M10 Three HUD dock rows printed fixed strings. Two now read the live
  table; `component_cycle` stays a literal on purpose - the wheel belongs to
  the ACTION, not its `BindingSpec`, so no rebind can move it. Said in the code
  at the row.
- [x] M11 An autopilot verb bound to a mouse button lost its HUD chip.
- [x] M12 The scenario rig was never rebuilt on a rebind.
- [x] M13 The rebuild despawned rigs without releasing them, so a gesture held
  across it was stranded down.
- [x] M14 The OBJECTIVES affordance had no text fallback: a key with no art
  drew an empty chip.

The wrong vocabulary reached the player:

- [x] M15 Refusals printed raw `KeyCode` variant names. Every refusal string
  now uses `readout_label()`.

The art was drawn but never judged:

- [x] M16 Read-only rows looked exactly as changeable as live ones - the tint
  reached the text branch alone.
- [x] M17 Glyph height was pinned, so portrait art became a speck.
- [x] M18 The group tab bar was inside the scrolling body.

The record disagreed with the code:

- [x] M19 The player wiki still taught the two deleted comms keys, left the
  RCS pad cell empty and quoted a refusal string that no longer exists.
- [x] M20 The dev book had no `nova_input` row, and credited `nova_ship` with
  a table deleted in this range.
- [x] M21 `design.html` contradicted the code in 15 of 33 rows.
- [x] M22 A docstring in `nova_os_ui/src/bindings.rs` said the viewer actions
  carry no pad binding, twenty lines above twelve that do.

Minors folded in:

- [x] N1 Press and release each re-resolved the action name, so a rebind
  between them stranded the pressed source down forever. `DrivenPresses`
  records what a press pushed and the release lets up exactly that.
- [x] N2 `ActionAxes.stick` was display-only. `dispatch::apply_stick` writes
  the declared stick's pad axes, so a driven range can exercise it.
- [x] N3 A stored binding was not checked against the column it sat in.
  `rebind` now refuses a spec the rebind screen could not have produced: a
  source in the wrong device column, or a column the action ships empty.
- [x] N4 The glyph coverage test walked keyboard sources only, and two of four
  owners. `nova_hud`'s walks every source of the two owners it can reach;
  `nova_menu`'s new one walks all five lists, pad buttons included.
- [x] N5 `sync_nova_os_contexts` built a fresh `Vec` every frame for an answer
  fixed at startup.
- [x] N6 Three naming collisions: `camera_rotate` is `Camera Aim`,
  `novaos_toggle` is `Open NOVA OS` (the group tab keeps `NOVA OS`), and the
  component pair reads `Lock Next Component` / `Lock Previous Component`.
- [x] N7 The Controls screen had no shipped screenshot. `screenshot_menu` now
  clicks through to the tab and writes `wiki-controls.png`.
- [x] N8 Documentation drift inside the new crate: `register` logs and
  replaces rather than refusing, `primary_source` has no caller yet and says
  so, the harness DOES press a pad-only name, the keycap count is 101 and 226K,
  and the crate carries an optional serde.
- [x] N9 Two changelog entries against a baseline that never shipped: the RCS
  pad entry was written twice, and one entry contrasted against a state that
  only existed between commits inside this cycle.
- [x] N10 `InputSources` took bevy's button resources as non-optional while a
  consumer inserted them for the whole tree. `nova_input` installs them itself
  now.

Not fixed, and why:

- [ ] P1 (MAJOR) The RCS stick carries no `DeadZone`, and `rcs_modifier` is
  that stick's own click, so engaging RCS can spike the axis it steers.
  DEFERRED by the owner: "we will modify default bindings for controller to
  make more sense, I need to do a playtest, for now let's keep it simple."
- [ ] P2 Taking Left Trigger 2 off `rcs_modifier` put it on Left Thumb, which
  the editor's Sandbox-return chord also reads. Same pad default question,
  same deferral. Raised here because this range created it.
- [ ] N11 (MINOR) The fixed 560 px panel clips one group and strands 460 px on
  another. The fixed height is a standing owner instruction ("would keep the
  height the same, so fixed"), so the clipped row is a layout question for the
  owner, not a defect to fix around it.
- [ ] N12 (MINOR) The pad column optically dominates the desk column. A colour
  call, and it is pad art - it waits for the same playtest.
- [ ] N13 (MINOR) `update_flight_verb_hints` allocates seven strings per frame
  before its own change check discards them (16.73 us, pre-existing). Measured
  and accepted: the labels are `String` all the way into `nova_hud`, so the
  saving is the table lookups alone and the churn is not worth it. The keycap
  alpha scan (17.26 ms in one frame, 101 images) runs behind the loading
  screen and is likewise accepted.

### Regression found while fixing

M17's first fix sized EVERY cap by its short axis. The pack's letter caps
measure 0.92 - a hair taller than wide - so all 26 grew two pixels and changed
the dock's row rhythm; `keybind_dock`'s own sizing tests caught it. The rule
now starts below the keycaps (`PORTRAIT_ASPECT`, 0.8) and
`only_portrait_art_is_sized_by_its_width` pins it where the rule lives.
