# Document the destruction model on all three doc surfaces

- STATUS: CLOSED
- PRIORITY: 0
- TAGS: archive,docs,destruction

Epic: `20260818-220812`.

`0ee9cbb0` landed the destruction rework as one squashed commit and shipped the
CHANGELOG lines, but the three reader-facing surfaces still describe the game
from before it. `docs/keeping-docs-in-sync.md` says a code change is not
finished until the docs it invalidates are fixed in the SAME task. This is that
task, paid late.

## What changed that the docs do not know

- A body now wears its damage in its own GEOMETRY. There are two damage
  readings, not one: `DamageLevel` (erosion) and `DamageMarks` (craters).
- How a section degrades is AUTHORED per section - `DamageEffect::{Cracks,
  Sparks, Plume}`, defaulting to `[Cracks]` - not special-cased per code path.
- Asteroids CARVE. Marks accumulate, a crater captures a following hit only
  within a unit of itself, and rock is its own material about ten times softer
  than cladding: a radius-3 rock is 2.4 minutes of held PDC fire, not 24.
- Torpedoes fuze on CONTACT against the closest point of the locked body, not
  half a blast radius from its centre of mass. **(breaking)**
- Blast damage is priced on what a body ABSORBS, not what was requested, and
  sections shield against blast pressure. **(breaking)**
- The turret catalog is now the two PDC mounts. The Better turret, the Light
  turret and the ten per-craft prototypes are GONE, and every craft mounts the
  kinetic PDC. **(breaking)** - this one has the widest doc blast radius,
  because any page or catalog naming a removed prototype is now a lie.

## The three surfaces

**Player wiki** (`web/src/wiki/`): `combat-weapons.md`, `sections.md` and its
`sections/` children, `hud.md`, `glossary.md`. What a player needs: shooting a
rock leaves a hole, holding the trigger deepens THAT hole, rocks take real
time to break, torpedoes hit before they burst.

**Creator docs** (`web/src/create/`): `sections.md`, `objects.md`,
`base-content.md`, `reference.md`. The authored fields are the contract -
`DamageEffect`, the asteroid object fields, the turret prototype ids. A mod
author reading a removed prototype id gets a broken mod.

**Dev book** (`docs/`): `sections.md` (ship section internals),
`architecture.md` if the plugin wiring moved. The cost model in
`asteroid_carve.rs`'s `FIELD_RESOLUTION_MAX` doc is currently the best
description of carve cost anywhere in the tree and it belongs in the book -
but coordinate with `PERF-SURFACE`, which will change the numbers.

## Order

Can start NOW. The performance work changes what carving COSTS, not what it
does, so none of the behaviour above is waiting on it. The one exception is the
carve cost model in the dev book, which should be written last or written
against whatever `20260818-221036` leaves behind.

Must land BEFORE `20260818-181812` captures anything - a capture of a page that
is about to be rewritten is a wasted capture.

## Done when

- Every row of the `keeping-docs-in-sync.md` dependency map touched by the
  areas above has been READ and fixed or confirmed correct.
- No surviving mention of `better_turret`, the light turret, or the per-craft
  turret prototypes anywhere under `web/src/` or `docs/`.
- `cd web && npm run ci` clean.
