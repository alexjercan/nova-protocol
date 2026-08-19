# A dead section reads as a grey crate, not as wreckage

- STATUS: CLOSED
- PRIORITY: 0
- TAGS: archive,wontdo

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

## CLOSED 2026-08-19 - not a defect

Owner: "tumbling is fine".

This task was filed on an INFERENCE, not on a complaint. The owner had already
said "for sections we do tumbling which is fine IMO" when the detach was
designed; I then read an arena capture, judged the plain boxes to look wrong,
and opened it at p75 anyway.

The capture that convinced me had TWO kinds of grey box in it - carve shards and
detached wrecks - and I attributed the read to the wrecks. The shards were the
real complaint, and they are gone: `6548ed8c` keyed shards on weapon class so
explosive throws none, and `5d5e1e73` capped the rest. What remains is a
destroyed section tumbling away wearing its own art, which is the design working.

Nothing was implemented. `explode.rs` is behaviourally untouched since this was
filed.

## What is preserved, in case it ever does bother somebody

The unverified hypothesis: cladding is destructible and dies BEFORE the section
under it, so by the time a section dies its plates are already gone and the bare
hull cuboid is all there is to detach. If that is true, "make the art follow" is
a dead end - the plates genuinely are not there - and the options narrow to
giving the wreck a different look, or baked fragment sets (designed and
deliberately unscheduled in `20260818-224219`).

One run answers it. Nobody has done that run.

The related remnant that IS still live and unfiled: shard material is hardcoded
`Color::srgb(0.30, 0.30, 0.33)` (`spew.rs`), so PDC chips off a brown rock read
as foreign grey specks. Fixing it needs a walk from the mark-owner root down to
a child's `MeshMaterial3d` plus a tint-keyed material cache to avoid asset
churn.
