# The automation does not run the game: cover the 9 uncovered scenarios

- STATUS: CLOSED
- PRIORITY: 0
- TAGS: archive,wontdo

Epic: `20260818-220812`. Found by `20260818-221027`. Full table:
`tasks/20260818-221027/REPORT.md`, "The coverage table".

## The finding

The automation does not run the game.

- **12 scenario ids are player-reachable. The probe runs 3.**
- **5 have zero example or probe coverage**: `broadside`, `broadside_gunship`,
  `lifeline`, `final_tally`, `asteroid_next` - that is the entire shipped
  campaign.
- 4 more are only reached by a random 1-in-4 menu backdrop draw, so a green run
  proves nothing about the other three.
- **34 of the 40 examples load a Rust fixture no player can reach.**

`editor_sandbox` collapsing to 2 fps (`20260819-001252`) was not a freak
uncovered case. It was the first uncovered case somebody happened to fly. The
campaign is in the same state right now and nobody has flown it.

## Why the fixtures happened, and why that is not a defence

A fixture is faster to write, deterministic, and isolates the thing under test.
All true, and none of it justifies 34 of 40. The result is a suite that is green
while the game is unplayable, which is the exact failure this release exists to
stop.

Fixtures are legitimate for a `systems/` range pinning one behaviour. They are
NOT legitimate as the only coverage of a surface a player loads.

## What to do

1. **Cover the campaign.** Five scenarios, zero coverage. Highest value here by
   a distance: it is what a new player plays first.
2. **Make the backdrop draw deterministic under the probe**, so all four
   backdrops get run rather than one at random.
3. **Audit the 34 fixture-only examples** against
   `20260818-221103` (every example is playable by hand, or says why it is
   not). The two tasks are the same audit from opposite ends - one asks whether
   a HUMAN can load it, this asks whether it loads what a human would. Do them
   together.
4. **Make the coverage table a standing artifact**, not a one-off. A
   player-reachable scenario with no case should be visible without an agent
   spending a day finding it.

## Make a contended run detectable, or the numbers keep lying

Added 2026-08-19, after a `frametime.csv` row was read as an editor defect and
filed at p92 when it was really another agent's `rustc` on the same box. Cost:
one task and one agent run.

`frametime.csv` already carries `min_ms` beside `mean_ms`, and that is the tell:
**a run whose MINIMUM frame is several times its own historical minimum is a
contended run, not a regression.** A mean dragged up by real stalls keeps a
normal minimum; a busy host raises the floor.

Make the tooling say so rather than leaving it to prose. The report warned about
host load in words and the very next row in the same document was read as a
finding anyway - a warning nobody applies is not a control.

Cheapest useful version: record the minimum alongside each budget, and have the
budget check report SUSPECT rather than FAIL when the floor has moved as much as
the mean. A contended run should not be able to look like a regression, and it
should not be able to hide one either.

## The structural blocker to solve first

`editor_sandbox` cannot be loaded by any id-driven rig. It is registered into
`GameScenarios` at editor-Play time
(`crates/nova_editor/src/scenario.rs:203-212`), which is AFTER the
`--scenario <id>` membership check runs (`crates/nova_core/src/lib.rs:257-266`).
So `--scenario editor_sandbox` is refused, and the only thing that can reach it
is a rig that clicks through the editor and presses Play.

Any scenario registered late has the same problem. Decide whether late
registration is worth keeping; if it is, the membership check needs to know
about it, and if it is not, register the sandbox up front like everything else.

## CLOSED 2026-08-19 - not doing it

Owner: "I don't care what the first task says we will not do it."

The finding stands and is worth keeping: the probe runs 3 of the 12 scenarios a
player can reach, 5 have no coverage at all, and 34 of 40 examples loaded a
fixture no player could reach. That is how `editor_sandbox` ran at 2 FPS for a
whole cycle without anything noticing.

What changed is the answer. Rather than bolting a roster sweep onto
`scene_baseline`, `scene_baseline` is DELETED and measuring a scenario becomes a
`probe` subcommand - `20260819-123928`. An example is the wrong shape for
"point the tool at this scenario".

The example half of the audit landed separately as the three-way split
(`20260818-221103`), so the fixture-only concern is addressed.

The min_ms contended-run idea recorded here also does not survive: per-example
budgets are gone entirely (`budgets.rs` deleted) because judging a frame rate is
a human reading the report, not a script asserting a number. Detecting a
contended run is now a REPORTING concern - see `20260819-123928`.
