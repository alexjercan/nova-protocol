# NOVA OS map app: clicks miss their targets where the ship app's land

- PRIORITY: 42
- TAGS: v0.9.0, bug, ui, nova_os, feedback
- KIND: TASK
- ACTIVITY: COMPOUNDING
- GATES: PLAN REVIEW RETRO
- RESOLUTION: DONE

## Story

Owner playtest 2026-07-30 (feedback wave):

> sometimes it's hard to click on things (in the map -> the ship GUI app works
> much better, clicks work almost 99% of the time on labels, so maybe there is
> a diff in the apps)

Two NOVA OS apps, same pointer plumbing, very different hit rates. The owner's
own comparison is the best lead we have: whatever the map does differently from
`ship` is the bug.

## Understanding (2026-07-30) - the differences found, none yet proven

Both apps composite through the same forwarded pointer
(`crates/nova_gameplay/src/hud/nova_os.rs`, `forward_nova_os_pointer`) and both
select via a `Button` + `Activate` observer rather than `Interaction` polling.
Candidate differences, in rough order of suspicion:

1. **The CRT warp inverse is approximate.** `forward_nova_os_pointer` maps the
   cursor into image space through `nova_os_inverse_barrel(local, WARP)`, which
   applies `c / (1 + a*r^2)` - the SAME form as the forward warp, not its true
   inverse (inverting that map means solving a cubic). The residual error is
   zero at the screen centre and grows with r^2 toward the corners. The ship
   app's blips cluster around the ship schematic near the middle; the map
   spreads contacts across the whole viewport including the corners. That
   asymmetry fits the report shape exactly. MEASURE the residual in px before
   believing it.
2. **Hit-target size.** Map: `MAP_BLIP_PX = 12` dot, and the label is a bare
   `Text` child at `left: 16px` - a 4 px dead gap, and the text node is the only
   thing backing the label. Ship: `SHIP_BLIP_PX = 12` dot, but the label is a
   padded pill NODE (`padding: 4x1`, its own background) at `left: 14px` with
   the text nested inside - a bigger, solid child of the button. "Clicks work on
   labels" in the ship app is consistent with the pill being the real target.
3. **Clipping.** The map viewport is `Overflow::clip()`; UI picking respects
   clip rects, so a blip near the viewport edge is only pickable over its
   unclipped part.
4. **Overlap / z-order.** Map contacts move and their labels routinely overlap
   neighbouring blips; the topmost node blocks lower ones. The ship schematic's
   sections do not drift over each other the same way.

## Steps

- [x] Reproduce first, at the highest fidelity available: a rig that drives the
      forwarded pointer at a known on-screen position over the map app and
      asserts the intended blip is selected. Aim it at a CORNER contact and a
      CENTRE contact - if hypothesis 1 is right, the corner case fails and the
      centre case passes. Record the measured miss distance in px in NOTES.md.
- [x] Measure the barrel round-trip directly: feed a grid of uv points through
      the shader's forward warp and this inverse, and record the residual across
      the screen. This is a pure-math check and settles hypothesis 1 with
      numbers rather than argument.
- [x] Only then fix the cause the evidence names. If it is the warp: invert the
      forward map properly (closed form or a couple of Newton steps) and keep
      the forward shader and the inverse derived from ONE definition so they
      cannot drift apart again.
- [x] Independently of the cause, close the gap the comparison exposes: give the
      map label the same solid, padded, button-child target the ship app's pill
      has, and remove the dead gap between dot and label. The owner's 99% figure
      is the bar.
- [x] Check the clip and overlap paths against the rig before closing - a fix to
      the warp that still loses edge contacts to clipping is not a fix.
- [x] Sweep the other NOVA OS apps for the same bare-text-target shape.

## Definition of Done

1. The failing case from step 1 passes: a click at a known position over a map
   blip - centre AND corner of the viewport - selects that contact (test: the
   pointer rig, which failed first).
2. The forward warp and the pointer inverse round-trip within a stated pixel
   budget across the whole screen, budget recorded (test: the math rig).
3. Map blip labels are as clickable as their dots (test: a click on the label
   selects the contact).
4. No other NOVA OS app leaves a bare `Text` node as its only hit target
   (cmd: `rg -n 'Button' -A 20 crates/nova_gameplay/src/hud/nova_os*.rs`
   reviewed by hand).
5. Owner clicks map contacts as reliably as ship sections (manual).

## Notes

The four hypotheses above are LEADS, not a diagnosis. The reproduction comes
first and the fix follows the evidence; if the rig shows the warp is fine, say
so and chase the next one.

## Outcome (2026-07-31)

Diagnosis CONFIRMED for hypothesis 1, but not for the stated reason, and hypothesis
2 was FALSIFIED. Full record: `NOTES.md`.

### What it actually was

`forward_nova_os_pointer` did not carry an approximate inverse of the warp - it
carried the wrong map in two ways at once. The shader's chain maps SCREEN uv to
IMAGE uv directly (`barrel()` then a 0.93 overscan pull), so the pointer needed
that forward map; it applied the barrel INVERSE instead, and never applied the
overscan at all because `NOVA_OS_OVERSCAN` was a WGSL-local `const` the Rust side
could not see. The overscan term is near-linear in radius, which is why the miss
was not confined to the corners.

Measured on 1280x720: 0 px at centre, ~8 px a tenth of the way out, **27.1 px in x
/ 15.3 px in y at the corner** - against 12 px blips. The live-tree rig reproduced
it end to end: aiming at a corner contact put the pointer on image px
(79.1, 51.3) with the dot at (65.2, 43.7), a 15.8 px miss, and nothing selected.
That radius dependence IS the map-vs-ship asymmetry the owner reported.

Fix: `nova_os_crt_screen_to_image_uv` mirrors the shader's whole sample-UV chain
(raster collapse, barrel, overscan) and is therefore exact, and the overscan moved
from a WGSL constant into the material uniform this crate fills, so warp and
overscan each have ONE definition. The pointer now also reads `power` from
`NovaOsOpenness`, the same value the shader gets.

### Hypothesis 2 was wrong (recorded so the next session does not re-derive it)

A bare `Text` node IS a hit target: bevy 0.19's `ui_picking` tests the text node's
whole box and falls back to the node entity when the cursor misses a glyph
section. Clicking the middle of the map's bare label always worked. What was real
was the geometry: a 6 px dead band between dot and label, and a box tight to the
glyph run. The sweep found the SHIP app - the one held up as working - carrying
the same defect at 4 px. Both offsets overlooked that an absolutely positioned
child's `left` is measured from the parent's PADDING edge, inside the dot's 2 px
border; both are now `<BLIP>_PX - <BLIP>_BORDER_PX`, and the map label gained the
ship's padded backing pill.

### Difficulties

- The live-tree rig needed `VisibilityPlugin` (`ui_picking` refuses nodes whose
  `InheritedVisibility` is not computed-true, and `MinimalPlugins` propagates
  nothing), which in turn wanted the mesh asset collections; and `ButtonPlugin`
  alone, since `UiWidgetsPlugins`' text-input plugin needs IME messages and an
  `InputFocus` resource a headless rig cannot supply. Three iterations of
  "Resource does not exist" panics with the system name stripped.
- The first seam probe aimed at the exact shared edge between dot and pill, which
  bevy's `contains_point` excludes from BOTH rects. Replaced with a sweep across
  the band at pixel centres, which is also a stronger statement.
- The A/B pass caught a real weakness: once the label pill existed, the corner
  click test PASSED against a restored pre-fix mapping, because the bigger target
  absorbed the 15.8 px miss. The rig now also asserts the pointer landed within
  the dot's radius of the intended image point. Both sabotages were then run
  against the committed fix: the mapping sabotage fails 5 tests, the label-offset
  sabotage fails 2, each naming its own cause.

### Self-reflection

Reading the shader BEFORE theorising would have found this in minutes: the
overscan line sits two lines below the barrel call, and "the pointer inverts a map
that is already the direction it needs" is visible on one screen of WGSL. The task
text framed it as an inverse-accuracy problem and I spent the first pass measuring
that framing rather than checking the premise. Cheap rule for next time: when a
Rust helper claims to mirror a shader, diff it against the shader's actual lines
first, and count the operations on each side.

The A/B also argues for ordering: had I fixed the geometry before the mapping, the
mapping's own regression pin would have been born weak and I would not have
noticed. Sabotage each half separately, not just the change as a whole.

### Proofs

1. `cargo test -p nova_gameplay --lib map_contacts_select_where_the_crt_shows_them`
   (centre AND corner; failed first with the numbers above).
2. `cargo test -p nova_gameplay --lib nova_os_pointer_mapping_matches_the_crt_shader_across_the_screen`
   - budget 0.5 px across a 17x17 grid, stated in the test.
3. `cargo test -p nova_gameplay --lib map_contact_label_and_dot_are_one_unbroken_target`
   plus `ship_section_label_and_dot_are_one_unbroken_target`.
4. Sweep of every `Button` spawn under `crates/nova_gameplay/src/hud/nova_os*.rs`
   (7 sites: map blip, ship blip, ship panel buttons, chin knob, SND, PWR, the
   `[ ESC ]` app close). None leaves a bare `Text` as its only hit target; the two
   blip sites are fixed above, the other five were already padded nodes.
5. MANUAL, pending the owner: clicking map contacts as reliably as ship sections.

Shader validated by RUNNING it (`.wgsl` is not covered by `cargo check`). Two
separate commands, which the first draft of this close-out wrongly ran together
(review R1.3 - the capture is gated on `BCS_REEL`, so the smoke form writes no
PNG):

```
# smoke: does the shader compile at all?
DISPLAY=:99 BCS_AUTOPILOT=1 \
  cargo run --example screenshot_nova_os --features debug          # exits 0

# capture: the frames the pills were eyeballed in
DISPLAY=:99 NOVA_SHOT_DIR=target/reel BCS_AUTOPILOT=1 BCS_REEL=1 \
  cargo run --example screenshot_nova_os --features debug          # writes nova-os-ship.png
```

`nova-os-ship.png` shows the pills flush against their dots. Probe:
`cargo run -p nova_probe -- run screenshot_nova_os` -> OK, with `process_exit` and
`log_clean` PASS and the four timeline/FPS checks SKIPPED (this example captures
no timeline, so they are NOT MEASURED rather than held). Full suite:
`cargo test -p nova_gameplay --lib` 786 tests, 785 pass, 1 pre-existing ignore.
