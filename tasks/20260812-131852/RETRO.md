# Retro

## What shipped

- The parts gallery: a full-screen browser of the section catalog with a live
  3D preview per tile, a category row, a search field and a focus card that
  turntables the part beside its stats. It is the editor's ONLY parts picker
  now - the component drawer is deleted.
- Link points that make one part fit every craft: a roll zero derived from each
  socket's normal (`link_point_up`), frame-to-frame mating in `snap_placement`,
  and cardinal snapping of authored normals (`cardinal_axis`). Plus
  `box_link_points`, so a part authored at its own size mates one of any other.
- `pdc_turret_section` - one compact PDC replacing ten per-craft copies in the
  editor - and `RenderMeshTransform::scale`, which is what let it be assembled
  at its own size rather than balancing unit-cube art on a small mount.
- Editor UX: socket gizmos, Tab to open the gallery, Q to take the part under
  the cursor (in both surfaces), reversible wheel cycling, focus-view zoom and
  orbit, a contextual key legend, a ship heading arrow.
- Two cross-crate bug fixes the feedback exposed: ESC as a back gesture
  (`EscapeOwner`), and TAB no longer arming the NOVA OS where there is no ship.

## What went well

- Measuring before theorising, twice. The "parts only fit their own ship"
  complaint had TWO causes (arbitrary roll, oblique normals) and only arithmetic
  on the real content separated them. The "turret is 1x1x1" complaint was a
  code-level default primitive, not the art - found by projecting the shipped
  camera pose against known geometry and comparing with the pixels.
- The generated-content pipeline made risky edits cheap to audit: every content
  change came out as a diff to read (one line for the mount, 16 hunks confined
  to one section for the resize), and `every_parts_ship_has_one_connected_mate_graph`
  proved the shipped ships survived normal-snapping before anything was run.
- The editor's own harness caught a design error unit tests could not: after
  the filter took focus, Enter no longer reached "place", and the walk hung
  with the gallery still up. That is exactly the class of bug a driven run is
  for.

## Pain / next time

- I answered the owner's first size question from a partial measurement - the
  GLB bounds - and missed that an unmeshed joint gets a code-level default
  primitive. The reading was right about the numbers and wrong about the
  subject. Next time, when a question is "why does this look wrong", measure
  what is RENDERED (walk the spawned children) before measuring what is
  authored.
- Two surfaces sharing a key (Q, Esc) is an ordering question, not a mapping
  question. Both live in `Update`, and the gate that separates them is the very
  state the gesture changes. Explicit `.before()` is the fix; it is worth
  reaching for the moment a second surface claims an existing key.
- The screenshot walk regenerates four figures but only one was in scope, so
  three unrelated menu shots had to be reverted by hand after every capture
  run. A `--only <name>` on the capture script would have paid for itself here.
- Deleting a UI surface (the drawer) breaks every harness that drove it,
  including one in a different example that only shows up under
  `--examples`. Grep the examples for the widget NAMES before removing a
  surface, not after.

## Owner-iterated rounds

Round 1 built the gallery. Rounds 2-5 were all playtest feedback, and every one
of them found something real: the link-point cause behind the PDC glut, a
turret sunk into its own mount, a hull-sized default base plate, and a keyboard
that had no room for shortcuts because the search field ate every keystroke.
The pattern worth keeping: the owner plays, names the SYMPTOM, and the fix is
usually one layer under where the symptom points.
