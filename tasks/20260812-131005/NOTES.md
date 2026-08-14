# Notes - link-point snapping in the editor

## What placement is now

`normal * 1.0` is gone. A click mates two sockets:

- the TARGET is the socket nearest the pointer's hit on the hovered section;
- the SOURCE is one of the placed part's own sockets, cycled with `F`;
- the pose comes from `snap_placement` (nova_ship): positions coincident,
  normals opposed, and the one remaining degree of freedom - the roll about
  the mating axis - is the builder's, cycled with `R`.

Because the editor mates the same sockets the runtime derives from, it can no
longer build a ship the integrity graph would reject.

## Shape of the change

- `nova_ship::sections::link_points` gains two pure functions:
  `snap_placement` (the pose from one mate) and `candidate_link_point_mates`
  (every coincident/opposed pair WITHOUT the ambiguity + connectivity gates -
  a ship under assembly is legitimately disconnected, so the strict
  derivation cannot answer "is this socket taken").
- `nova_editor::snap` is the pure solver: target socket, pose, and the
  refusal (`Occupied`, `Ambiguous`, `Overlap`, plus the two "no sockets"
  cases). Collider bounds appear ONLY in the overlap refusal, under the ship
  lint's rule - interpenetration is fine exactly where a mate says the
  interface is intentional.
- The four pointer observers collapse into one solve per frame
  (`PlacementPreview`) that both the ghost and the click read, so a click can
  only build what is on screen. The ghost is the part's real mesh through the
  shared preview spawner, plus a bounds box in the colour of its verdict; the
  refusal is spelled out in a `Placement Status` line under the build area,
  which otherwise names the mate (`<target socket> <- <source socket>`).
- `preview::PreviewRole` splits "a section of the ship" from "scenery": the
  ghost and the gallery tiles drop `SectionMarker` and their collider, so they
  are never counted, picked or saved as part of the ship. (The count bug this
  fixes is exactly what the editor harness caught.)
- `preview_section` now carries the authored `SectionCollider` like a live
  section does, so the overlap check reads authored extents rather than
  decoding an avian collider.

## Content

- The Racer / CargoA / CargoB semantic parts are unhidden
  (`hide_in_editor: false` in the ship builders, `content gen` re-run: the
  only diff is 31 dropped `hide_in_editor: true` lines). `content lint` stays
  at 0 errors.
- `scripts/cut-obj-into-parts.py` proposes link-point candidates from recipe
  seams: parts whose bounds meet within 0.05 on one axis and overlap on the
  other two get one socket each at the centre of the shared face. A recipe
  part may author `link_points` in ship space instead, which replaces the
  generated list. Both paths have focused checks in `--self-test`.
- The three recipe manifests were regenerated. The `.glb` bytes are
  byte-identical (verified with `cmp`), so the diff is the new `link_points`
  block alone. The per-object packs (`blocks/`, `quaternius/`) were left
  alone: their parts are separate models, not a cut ship, so seam candidates
  would be noise.

## Decisions to review

1. The source socket defaults to index 0 and cycles with `F`. There is no
   "guess the right socket" heuristic - for the semantic parts the natural
   pairing is by authored id (`to_fuselage` meets `to_nose`), and matching on
   that would need the target part's identity in the socket id, which the
   schema does not carry. Cheap to add if the cycling reads badly.
2. Refusal is per-placement, not per-hover-highlight: nothing paints the
   ship's free sockets. A socket overlay (the NOVA OS `MATES` view already
   draws mates) would make the rule visible before the pointer meets it.
3. `R`/`F` are unbound elsewhere in the editor but are NOT in the keybinds
   wiki page yet - see the gallery task's note; both wait on the owner's
   verdict.
4. On a unit cube every source socket gives the same pose (the cube is
   symmetric), so `F` only reads as doing something on semantic parts. That is
   honest, but it does mean the key looks inert on the starter hull.

## Coverage

- nova_ship: snap poses mate under the runtime derivation, including a rolled
  mate and an off-centre socket on a rotated target; candidates survive
  disconnection and report both suitors of an over-subscribed socket.
- nova_editor: the solver's five refusals and the socket/roll wrapping.
- `examples/ui/editor.rs` (live, needs a display): assembles a ship by
  snapping, rolls one mate with `R`, asserts the runtime derivation sees ONE
  connected structure, mounts a semantic module from the gallery (the
  unhide), meets the `socket occupied` refusal in words and proves the
  refused click built nothing, then presses Play and asserts the FLOWN ship
  re-derives the same mate graph from the flat saved poses.
- `probe run ui --correctness-only`: editor, widget_zoo, hud_range,
  menu_newgame, menu_scenarios all OK (log_clean + invariants).
- `content lint`: 0 errors, 0 warnings.

## Decisions, resolved (2026-08-14, at close)

The four review items above were parked for the owner. Three of them were
answered by the follow-up rounds on `20260812-131852`, which shares this
placement path:

1. Source-socket default. No longer index 0: `snap::natural_source` starts the
   cycle at the socket already facing the other way, so a part arrives in the
   orientation it was AUTHORED in and `F` steps away from that rather than
   toward it. The id-matching heuristic this note floated is not needed.
2. Free sockets are painted now. `placement::draw_link_points` rings every free
   socket with a stub along its normal while a part is armed, bright under the
   pointer, plus the armed part's own mating socket on the ghost - so the rule
   is visible before the pointer meets it.
3. `R`/`F` are in the keybinds wiki, in its Editor section, alongside the
   reversible wheel equivalents (wheel rolls, Ctrl+wheel cycles).
4. STANDS: on a symmetric unit cube every source socket gives the same pose, so
   `F` still looks inert on the starter hull. Honest, and now cheaper to live
   with - the socket gizmos show there is nothing to change.

## Not demonstrated at close

The DoD line "NOVA OS `MATES` shows the assembled graph" has no recorded proof
against an editor-built ship. The view exists (`nova_os_ui/src/ship`, `G: MATES`)
and the underlying derivation is covered both ways - the editor asserts the
FLOWN ship re-derives the same graph from the flat saved poses - but nothing
walks the overlay itself. Closed on the owner's call with that gap named rather
than quietly ticked.
