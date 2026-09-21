# Epic: v0.15.0 decides what a finished Nova is

- STATUS: OPEN
- PRIORITY: 90
- TAGS: v0.15.0,epic

Planned 2026-09-21 with the owner. The v0.14.0 epic (`20260909-214736`,
`tasks/20260909-214736/TASK.md`) is the precedent for shape.

## The release intent

v0.14.0 prepared the game's store presence. v0.15.0 answers a different
question: what makes the game feel DONE, and what can arrive in updates after
1.0.

The cycle is therefore DISCOVERY plus SELECTED MECHANICAL DELIVERY. Two
product spikes lead the board and decide direction; a short list of small,
already-understood tasks ships beside them so the cycle produces playable
change and not only documents.

v0.15.0 does NOT commit to implementing what the spikes propose. A spike
ends in decisions and child tasks. Which child tasks land, and in which
release, is a later owner decision.

## The board

Discovery, highest priority, and the reason this cycle exists:

- p80 `20260824-125943` station, ship inventory, and NOVA OS/UI direction
  (`tasks/20260824-125943/TASK.md`)
- p75 `20260824-125938` seeded procedural free/open-world mode
  (`tasks/20260824-125938/TASK.md`)

These two define the shape of a finished game. They classify their candidate
outcomes as Needed for 1.0, Nice to have, and Later/post-1.0, and those
classifications are PROPOSALS TO VALIDATE, not promises.

Gameplay:

- p65 `20260901-104359` combat balance and AI: reproduce weak combat and
  weapon-spam win odds, then balance (`tasks/20260901-104359/TASK.md`)

Mechanical delivery:

- p60 `20260920-102851` export an SFX sidecar from a capture loop
  (`tasks/20260920-102851/TASK.md`)
- p55 `20260907-173050` verified gameplay statistics in the wiki
  (`tasks/20260907-173050/TASK.md`)
- p50 `20260714-001140` gamepad navigation and a hardware playthrough
  (`tasks/20260714-001140/TASK.md`)
- p45 `20260831-145917` mobile virtual pad for the web build
  (`tasks/20260831-145917/TASK.md`)
- p40 `20260916-141252` non-repeating menu backdrop order
  (`tasks/20260916-141252/TASK.md`)

## Ordering and dependencies

Real dependencies, and only these:

- `20260831-145917` (touch) FOLLOWS `20260714-001140` (gamepad). The virtual
  pad targets interactions proven with a real pad first, so the pad layout
  and the interaction findings have to exist before touch commits to a layout.
- `20260901-104359` (combat) has an internal order of its own: a measured
  matched baseline before any balance change. It does not depend on the
  spikes and does not block them.

Coordination, NOT dependency:

- The two spikes must coordinate on the boundaries they share - persistence,
  content ownership, and UI surface. Neither spike may silently settle the
  other's question. Where they disagree, the disagreement is recorded as an
  open decision on both tasks, not resolved by whichever finishes first.

Explicitly independent:

- `20260907-173050` (wiki truth audit) runs concurrently with the spikes.
  It is a truth audit of what the game does today; it is not gated on, and
  does not gate, what the spikes decide the game should become.
- `20260920-102851` (SFX sidecar) and `20260916-141252` (backdrop order)
  depend on nothing on this board.

The spikes run first because everything after them is cheaper once their
decisions exist. The mechanical tasks do not have to wait for them.

## Not on this board

- `20260908-161328` full season story stays OPEN on backlog at priority 0.
  It is explicitly backburner and NOT v0.15.0.
- `20260824-125951` grown ship cast is CLOSED as WONTDO. The cargo family is
  no longer the base game's cast, and WFC-style ships can be added as needed.
  Its premise was not migrated into another task.

## Definition of done

- Both spikes have produced their decisions, their child tasks, and their
  proposed Needed/Nice/Later classification, with the unsettled choices still
  recorded as choices.
- The shared spike boundaries (persistence, content ownership, UI surface)
  each carry either an agreed decision or a recorded open decision on both
  spike tasks.
- `20260901-104359` has a measured matched baseline recorded before any
  balance change lands, and its closing evidence compares to that baseline.
- Every mechanical task on this board is closed, or explicitly cut with the
  cut recorded on the task itself.

No release date, tag sequence, or store gate is set by this epic.
