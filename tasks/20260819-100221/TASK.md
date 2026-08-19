# A dead section reads as a grey crate, not as wreckage

- STATUS: OPEN
- PRIORITY: 75
- TAGS: v0.11.0,polish,destruction,ship

Epic: `20260818-220812`. Follow-up to `20260818-224219`, landed as `6a2d6eb5`.

## The defect

A destroyed section detaches and tumbles away as a **smooth untextured grey
box**, next to a live hull covered in plates and greebles. It reads as a ship
shedding crates, not as a ship being destroyed.

Capture from the arena at the time of landing showed it clearly: large plain
cuboids drifting from ships whose surviving geometry is dense and detailed. The
contrast is the problem - not the tumble, which is correct, and not the physics,
which is correct.

This was a KNOWN risk of the detach design, accepted by the owner on
performance grounds so the slicer could be deleted. The performance case is now
settled and banked. This task buys the look back.

## Verify this FIRST - it decides everything else

Hypothesis, raised when the capture was reviewed and **never confirmed**:
cladding is destructible and dies BEFORE the section under it, so by the time a
section dies its plates are already gone and the bare hull cuboid is all that is
left to detach.

The detach code reparents descendant art and strips only colliders
(`crates/nova_gameplay/src/integrity/explode.rs`, the `try_remove::<Collider>`
walk), so the art SHOULD ride down. Either the hypothesis is right and there is
no art left to ride, or the reparent is not doing what it looks like.

- If the plates are genuinely already destroyed: "make the art follow" is a
  dead end. Options narrow to giving the wreck a different look, or to baked
  fragments.
- If the art is there but not drawn: it is a bug in the reparent or the
  material, and it is probably cheap.

Do not choose an approach before answering this. It is one run with the right
thing logged.

## Options, once it is answered

- **A wreck look.** Scorch, a darker material, a burnt variant - something that
  says "this is dead" rather than "this is a placeholder". Cheapest by far and
  it may be enough, because the current read is largely that the cube looks
  UNFINISHED rather than that it lacks detail.
- **Keep some cladding alive to the end.** If plates dying first is the cause,
  a section could keep its last plates rather than shedding them, so what
  detaches still wears something.
- **Baked fragment sets.** The deferred design, written up in
  `20260818-224219`. Most expensive, best-looking, and explicitly NOT scheduled
  - do not start it without the owner saying so.

Prefer the cheapest thing that changes the READ. This is a look problem, not a
fidelity problem.

## Constraints

- Do not reintroduce runtime geometry work on the death path. That is the whole
  point of what landed; a fix that slices, splits or meshes anything at death is
  not a fix.
- Frame rate wins over fidelity where they disagree. That has not changed.

## Done when

The owner looks at a ship dying in `wfc_arena` and it reads as destruction.
Their verdict decides it, not a metric. Attach the capture.

Death-frame cost must not regress: it is 2.5 ms a run at landing, against 46.2
before.
