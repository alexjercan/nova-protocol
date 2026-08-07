# Plan - one file per lane

Implementation outlines for the lanes in `../notes/17-lanes.md`. Findings are
referenced by their `../notes/16-findings-master.md` ids.

| File | Lane | Baseline | Depends on | Size |
| --- | --- | --- | --- | --- |
| `lane00.md` | L0 Fix the map, close the CI gaps | **BLOCKS** (before) | - | S |
| `lane01.md` | L1 Unblind the probe gate | NEUTRAL | - | M |
| `lane02.md` | L2 Build and baseline the benchmark | is the gate | L0 | M + owner |
| `lane03.md` | L3 Untrusted input, data loss, persistence | NEUTRAL | L1 | L |
| `lane04.md` | L4 Reconciler discipline and terminal input | NEUTRAL | L1 | M |
| `lane05.md` | L5 Delete the dead and lying surface | **BLOCKS** (after) | L2 | M |
| `lane06.md` | L6 nova_editor | NEUTRAL | L1 | S |
| `lane07.md` | L7 `nova_ui::screen` extraction | **BLOCKS** (after) | L2 | M |
| `lane08.md` | L8 nova_probe restructure | **BLOCKS** (after) | L1, L2 | L |
| `lane09.md` | L9 nova_gameplay four-way split | **BLOCKS** (after) | L2, L4, L5, L8 | XL |
| `lane10.md` | L10 nova_assets / nova_scenario cleanup | **BLOCKS** (after) | L2, L3 | L |
| `lane11.md` | L11 Perf and small correctness | NEUTRAL | L1 | M |

## How to read the outlines

Each file lists the concrete surface the lane changes. Notation:

| Marker | Meaning |
| --- | --- |
| `NEW` | item does not exist today |
| `CHANGE` | item exists at the cited `file:line`; the signature or body changes |
| `MOVE` | same item, new path |
| `DELETE` | item is removed outright |

Signatures for `CHANGE` and `DELETE` items were read out of the tree at
`4a8b55aa` and are quoted as they stand. Signatures for `NEW` items are
proposals - the shape the lane should land, not a contract.

## Re-review of the lanes, 2026-08-07

Six gaps found and closed in `../notes/17-lanes.md`:

1. **L2's dependencies contradicted themselves** - the summary table said
   `L0, L1` while the lane body said "not blocked by L1". The body was right;
   the table is corrected to `L0`.
2. **L9 was missing its L5 dependency.** Rule 10's "16 unordered sets" count is
   only 16 *after* `TweenSystems` and `StatusBarPluginSystems` die with F45/F46.
   Without the edge, L9 orders two sets that L5 then deletes.
3. **L8 renames the gate's own invocation** and no lane owned the callers.
   `cargo run -p nova_probe -- run --all` becomes `-p nova_probe_cli`;
   `.github/workflows/ci.yaml`, `AGENTS.md` and every doc quoting the command
   move in the same commit or CI breaks.
4. **F84 had no lane and no disposition** - the only finding in the set with
   neither. Added to "What is deliberately NOT a lane" as its own tracking task.
5. **L7 holds two player-visible bugs behind owner review time.** F17 and F28
   are behavior defects gated only because their fix happens to be an
   extraction. An escape hatch is now written into the lane.
6. **`RunArtifacts::load`'s doc comment defends the behavior F01 reports.** It
   says corrupt-but-present artifacts are *deliberately* hard errors. F01 is
   still right, but the fix must preserve that intent rather than reverse it -
   see `lane01.md`.

Everything else checks out. All 86 findings are assigned: F01-F82 and F85-F86
have a lane, F83 routes to the conventions workstream, F84 to its own task.
No finding appears in two lanes.
