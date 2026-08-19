# Split the examples three ways: playable, systems, screenshots

- STATUS: OPEN
- PRIORITY: 70
- TAGS: v0.11.0, example, docs

Epic: `20260818-220812`. Owner: "I want some examples where both the player and
autopilot can play them: e.g I want to be able to carve an asteroid by hand,
but also have the autopilot do it for screenshots/gif; but there are a lot of
examples that only work for autopilot, some of which are fine ... but there are
some which don't do anything if I load them as a player".

Owner, 2026-08-19, setting the shape: a THREE way split, not two.

## The three categories

An example is sorted by **who it is for**, not by what it happens to be able to
do.

1. **`playable`** - made for a HUMAN. A person loads it and does the thing it
   demonstrates. It MAY also carry autopilot, and often should: the autopilot
   is a second driver for captures and gates, never the only one. Owner's
   example: the greeble gallery belongs here, and it is still worth having
   screenshots of.
2. **`systems`** - AUTOPILOT ONLY. Correctness. Reproduces a found bug or pins
   a system already under test - sections, integrity, old regressions. Never
   playable, and nobody should expect to load one by hand.
3. **`screenshots`** - AUTOPILOT ONLY. Its output is IMAGES. A scripted shot
   needs a scripted camera, and making it playable would break the shot.

The test between `playable` and the other two is: **would a human loading this
expect to do something?** If the name promises a verb, it owes the verb.

The test between `systems` and `screenshots` is what the run PRODUCES: an
assertion, or a picture.

An example that silently does nothing when a human loads it is the defect being
fixed. After this task, that state does not exist - it is either playable, or
it is declared as autopilot-only where a human sees the declaration before
loading it.

## This CHANGES a convention

`CONVENTIONS.md`, Nova rule 1, currently ends: "`systems/` is where correctness
lives; `screenshots/` is where content does. **There is no third category**, and
a range that only measures is not a range."

That rule is superseded by this task and must be rewritten IN THE SAME CHANGE as
the tree move - the docs routing rule applies to `CONVENTIONS.md` like anything
else. Keep the part that still holds: a range that only measures is still not a
range, and `systems` still owns correctness.

## The move

`examples/playable/` is a new directory beside `examples/systems/` and
`examples/screenshots/`. Sorting an example into `playable` means MOVING the
file, not tagging it.

Watch for things that break on a move, because ids here are runtime strings and
nothing type-checks them (`CONVENTIONS.md`, Nova rule 3):

- the roster in `crates/nova_probe_cli/tests/catalog_drift.rs`
- `probe run <category>` and whatever enumerates categories
- `Cargo.toml` example paths
- CI workflow invocations
- any doc naming an example by path

## Audit first

`examples/screenshots/` (22) and `examples/systems/` (23). For each: load it as
a human, and record which of the three it is and what it would take to make it
playable if it is close. **The audit table is the deliverable of the first
pass** - do not start moving files before the list exists and the owner has seen
it.

Known calls, from the owner:

- `carve_asteroids` -> **playable**. Carving a rock by hand is the thing the
  whole destruction epic shipped and right now it can only be watched.
- `greeble_catalog` -> **playable**, keeping its captures.
- `screenshot_combat` -> **screenshots**. Explicitly fine as a rig.

Strong candidates worth checking early, not decided: `wfc_arena`,
`parts_viewer`, `thruster_gallery`, `damage_levels`, `widget_zoo`.

Note the overlap with `20260819-012153` (scenario coverage): that task asks
whether the automation loads what PLAYERS load, this one asks whether a human
can load what the automation runs. Same audit from opposite ends. Do them
together or do this one first, but do not do them independently and reconcile
later.

## Rules for the conversion

- Examples doubling as gates KEEP their gates. Playability is added alongside
  the assertions, never instead of them.
- Do not convert rigs for symmetry. A scripted shot that a human cannot
  meaningfully fly stays a rig.
- The description is the surface a human reads. Whatever lists examples must
  show it - check that the description actually reaches a player and fix it if
  it does not.

## Done when

- The audit table exists in this task, every example in one of the three.
- `examples/playable/` exists and every example in it has been loaded BY HAND
  and played - verified by loading, not by reading the code.
- Every `systems` and `screenshots` example says in one line what it is for and
  that it is autopilot-only.
- `CONVENTIONS.md` Nova rule 1 describes the three categories.
- `catalog_drift` green, `probe run` still finds everything it did before.
