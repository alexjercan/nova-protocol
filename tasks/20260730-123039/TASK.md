# NOVA OS map app: clicks miss their targets where the ship app's land

- STATUS: OPEN
- PRIORITY: 42
- TAGS: v0.9.0,bug,ui,nova_os,feedback

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

- [ ] Reproduce first, at the highest fidelity available: a rig that drives the
      forwarded pointer at a known on-screen position over the map app and
      asserts the intended blip is selected. Aim it at a CORNER contact and a
      CENTRE contact - if hypothesis 1 is right, the corner case fails and the
      centre case passes. Record the measured miss distance in px in NOTES.md.
- [ ] Measure the barrel round-trip directly: feed a grid of uv points through
      the shader's forward warp and this inverse, and record the residual across
      the screen. This is a pure-math check and settles hypothesis 1 with
      numbers rather than argument.
- [ ] Only then fix the cause the evidence names. If it is the warp: invert the
      forward map properly (closed form or a couple of Newton steps) and keep
      the forward shader and the inverse derived from ONE definition so they
      cannot drift apart again.
- [ ] Independently of the cause, close the gap the comparison exposes: give the
      map label the same solid, padded, button-child target the ship app's pill
      has, and remove the dead gap between dot and label. The owner's 99% figure
      is the bar.
- [ ] Check the clip and overlap paths against the rig before closing - a fix to
      the warp that still loses edge contacts to clipping is not a fix.
- [ ] Sweep the other NOVA OS apps for the same bare-text-target shape.

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

## Flow State

- FLOW STEP: PLANNED
