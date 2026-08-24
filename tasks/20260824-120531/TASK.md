# Multi-cell sections: spike the design, then ship the huge thrusters

- STATUS: OPEN
- PRIORITY: 55
- TAGS: v0.12.0,ship,content,editor,spike

v0.12.0. Opens the multi-cell section question that `20260817-090834`
explicitly defers (THRUSTERS.md follow-up 3). Owner decision 2026-08-24:
multi-cell is a v0.12.0 deliverable, run SPIKE-FIRST inside the release -
the audit found real design forks, not just work. Research:
`tasks/20260815-231945/CONTENT-AND-ART.md` section 2.

## Goal

One section spanning WxHxL cells, and the two huge thruster shells shipped
as placeable drives: `shell_bank` (3x3x1) and `shell_capital` (5x5x3), both
already landed as art in `art/part-candidates/shells/`.

## The question as written (tasks/20260816-200255/THRUSTERS.md:304-307)

> Multi-cell sections (L) - one section spanning WxHxL cells: reading
> (`PlacedPart` grows a footprint), wfc tiles, clearance (one lane per
> exhaust column), integrity, collider, editor placement. Designed
> separately; this spike only fixes its requirement.

## Phase 1 - the spike (the gate)

Settle the four forks with throwaway prototypes and record the verdict here:

1. **WFC strategy** - the hardest. `tile()` REJECTS any part whose rotated
   collider leaves the unit cell (examples/playable/shared/wfc.rs:272-279);
   the grid is one-tile-per-cell (:416-466). Options: meta-tile
   decomposition (large state-space change) vs stamp-big-drives-then-collapse
   -around-them (much cheaper, fits "capital stern with a 3x3x1"). Prototype
   the cheap one first.
2. **Per-cell exits.** `SectionExit` is one Vec3 per section entity
   (nova_ship/src/sections/clearance.rs:80-90); a 5x5 exhaust face needs 25
   `ShipExit` columns. Price the consumer set: clearance.rs:160-230, skin
   exit_pocket, editor refusal, wfc erode.
3. **PlacedPart footprint.** One position today (shell_skin.rs:798-810),
   bucketed once by `read_cells` (:830-845). Fix the shape (footprint field
   or cells iterator); all six constructor sites change together (live
   spawner, editor ghost x2, probe snapshot, clearance tests).
4. **Flank sockets** (THRUSTERS.md follow-up 1) - decide it HERE. It is
   independent of multi-cell and pays off on 1x1 immediately, but it changes
   wfc adjacency legality and replaces the one-socket test; big shells may
   need it to look right.

## Already free (do not redesign)

- Mass: collider volume IS mass at density 1
  (nova_ship/src/sections/base_section.rs:45-49) - a 3x3x1 collider gets
  mass 9 with zero new code.
- Skin reading: `SkinStructure::insert_section` already adds into per-cell
  buckets (shell_skin.rs:180) - call it once per footprint cell.
- Editor mating: link-point snapping, collider-AABB overlap refusal and lane
  clearance all work PER LINK POINT (nova_editor/src/snap.rs) - a multi-cell
  prototype with correct link points and collider mostly snaps today.
- Exhaust render: one bell per exhaust cell vs one sheet is already
  authorable (`ThrusterExhaustShape::Rect`,
  thruster_section.rs:153-162).
- Integrity: one entity dying removes the whole block - accept it (a drive
  block is one machine) unless the spike finds a reason not to.

## Phase 2 - implement (after the spike verdict)

Two to three lanes: reading + clearance; wfc; content + editor polish.
Author footprints on the prototypes, link points on exposed cell faces, and
promote the two big shells.

**Cut line: if the spike verdict prices phase 2 out of the release, phase 2
is the cut and the shells stay art - the spike verdict still lands here.**

## Done when

- Spike verdict recorded: wfc strategy chosen with evidence, footprint shape
  fixed, per-cell exits priced, flank-socket call made.
- Big drives place in the editor with correct refusals and lanes; wfc ships
  can carry them (or the stamp strategy is shipped); gallery and an
  arena/bench render prove the look.
- `content lint` and the determinism gates stay green.
