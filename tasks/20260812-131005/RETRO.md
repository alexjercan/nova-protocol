# Retro

## What shipped

- Editor placement by MATING link points, replacing the fixed `normal * 1.0`
  step: the part's chosen socket snaps onto the socket nearest the pointer,
  normals oppose, and the builder keeps the roll and which socket does the
  mating. Refusals for an occupied socket, an ambiguous one, and a part that
  would sit inside a section it does not mate with - each in words.
- The placement ghost draws the part's real mesh at the pose a click commits.
- Semantic Racer/CargoA/CargoB parts unhidden from the palette, which is what
  the whole task was gating.
- `cut-obj-into-parts.py` proposes link-point candidates from recipe seams,
  with recipe overrides; shipped gameplay sockets stay hand-authored in Rust.

## What went well

- Solving the pose ONCE per frame and feeding both the ghost and the click
  removed a whole class of bug: what the builder sees is what the click
  commits, by construction rather than by keeping two paths in step.
- Splitting `candidate_link_point_mates` from `derive_link_point_graph` was the
  key move. A ship under assembly is legitimately disconnected and legitimately
  has a socket with two suitors; the strict derivation answers "is this ship
  valid", and placement needed "what is taken, and would this make it worse".

## Pain / next time

- The task shipped its code and then sat IN_PROGRESS for two days because its
  notes ended in a "Decisions to review" list rather than a verdict. Parking
  open questions for the owner is right; leaving the STATUS to imply the WORK
  is unfinished is not. Next time: land the code, close the task, and file the
  parked questions where they will be seen - the status field is not a
  question queue.
- Three of the four parked items were then answered incidentally by the
  follow-up rounds on the gallery task (`20260812-131852`), because both tasks
  touch the same placement path. Two open tasks over one seam meant the review
  items drifted to whichever one the owner happened to be playing.
- One DoD line closes unproven: nothing walks the NOVA OS `MATES` overlay
  against an editor-built ship. Named in NOTES rather than ticked.

## Follow-on

The owner's playtest of the gallery drove the deeper fix this task's placement
made possible: parts from different craft did not mate square, because the roll
was left to a shortest-arc rotation and the authored normals were oblique. That
landed in `20260812-131852` (`link_point_up`, `cardinal_axis`,
`box_link_points`), and it is what makes the placement built here usable across
the whole catalog rather than within one craft.
