# Decisions: example category run policy

## `CategoryPolicy` carries two booleans, not three

**Decision.** `CategoryPolicy { probed: bool, frame_time: bool }`.

The NOTES draft proposed `{ correctness, frame_time, in_all }`. Against the
five settled rows those three collapse to two:

| Category | correctness | frame_time | in_all |
|-|-|-|-|
| `sections/` | yes | no | yes |
| `systems/` | yes | no | yes |
| `stress/` | yes | yes | yes |
| `ui/` | yes | no | yes |
| `screenshots/` | no | no | no |

`correctness` and `in_all` are the same column. There is no row - and no
requirement in this sprint - for a category that runs correctness but is
excluded from `--all`, or that is in `--all` but runs no correctness pass.
Per the concept budget, a field with no caller and no invariant is deferred:
one boolean, `probed`, expresses both, and `frame_time` is the only genuine
second axis.

**Consequence, and the reason this is load-bearing.** With `probed` gating
BOTH `--all` and bare category expansion, `probe run screenshots` must ERROR
("category `screenshots/` is not a probe target") rather than expand to an
empty run. That is the honest reading of "screenshots/ leaves probe's scope",
and it is a user-visible CLI behavior that a three-field design could have
fudged by leaving `screenshots` expandable-but-inert.

If a later category ever needs "runs under an explicit spec but not under
`--all`", split `probed` then, with that caller in hand.

## The policy table is a `match` in `catalog.rs`, not `Cargo.toml` metadata

**Decision.** Carried forward from NOTES unchanged.

A second `[package.metadata.nova_probe]` parser is precisely the thing this
task exists to delete. The category strings become an API either way; putting
the table in code means a new category is a compile-time edit next to
`CatalogExample`, and `every_category_has_a_probe_policy` turns a missing row
into a red test rather than a silent default.

## `gameplay` and `perf` get transitional policy rows

**Decision.** Ship rows for both, marked `# remove with <task-id>`.

This task moves no files, so both directories exist on disk when it lands.
Without rows they would hit the unknown-category default and trip
`every_category_has_a_probe_policy`. The alternative - doing the directory
moves here - would collide with `093910` / `093934` / `094006`, which own the
content and must edit `tests/examples_smoke.rs` atomically with each move.

Accepted cost: `probe run gameplay --fps` loses its frame-time pass during
transit. See TASK.md "Transitional behavior, in the open".
