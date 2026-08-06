# Prototype 00 - conventions every migration step follows

Read this once. The nine per-migration prototypes assume it and do not repeat it.

## What "vendor" means here (owner's ruling)

- **Logic is verbatim.** Do not change behavior, constants, system ordering,
  observer wiring, or public semantics. Copy the tests and the guard comments
  with the code.
- **File layout is free.** Land the code where nova's tree wants it, not where
  BCS put it. Renaming `helpers/temp.rs` -> `lifetime.rs` is expected; rewriting
  what `TempEntity` does is not.
- **Stay close enough that the compiler does the work.** Every step is
  copy -> `cargo check` -> fix what it names -> stop. If a step needs design
  thought, you have widened the scope; stop and record it instead.

## Source of truth

```
/home/alex/personal/bevy-common-systems/   @ 6f09461  (v0.19.6)
```

Use the **working copy**, not the cargo checkout. NOTES.md was verified against
`e5da687`; HEAD is one commit further (`6f09461`, a doc rewrap in
`modding/events.rs`). No code delta - the NOTES inventory still holds.

Nova has BCS locked at `30d1befa` / `v0.19.5`, ~50 commits behind. Irrelevant:
the dep is being deleted, and the owner wants the newer source.

## The per-step loop

Each prototype is one commit and leaves the workspace compiling.

1. `cp` the files verbatim into the destination path.
2. Rewrite the module header: `use crate::...` for intra-crate items,
   `nova_*` paths for cross-crate ones.
3. Rewrite every rustdoc/doctest that names `bevy_common_systems` (counts are
   listed per prototype - they are `use bevy_common_systems::prelude::*;` lines
   inside ```` ```rust ```` blocks, and they FAIL THE DOCTEST, not just read
   wrong).
4. Add a nova docstring saying *why nova owns this module now*, following
   `crates/nova_gameplay/src/integrity/mod.rs`.
5. Wire the module into its crate's `lib.rs` (`pub mod`, prelude re-export)
   and move the plugin registration off `bevy_common_systems::prelude::` onto
   the local path.
6. Repoint the callsites this prototype lists.
7. Verify (below), then commit.

## Prior art - read before starting

`crates/nova_gameplay/src/integrity/` (commit `5f67c75a`) is the reference for
what "absorbed" looks like in this repo: BCS `health` + `integrity/*` became
nova modules with nova docstrings explaining ownership, the wrapper glue
collapsed into them, and `destructible_body` came along.

`crates/nova_autopilot/` (tasks `20260802-183403`, `20260802-183406`) is the
other precedent.

## Two lint facts that bite copied code

- **Every nova crate has `#![warn(missing_docs)]`; BCS has none.** Copied
  `pub mod prelude { ... }` blocks, pub struct fields and pub fns will emit
  warnings the source never did. CI runs
  `cargo clippy --workspace --all-targets --features debug` - check whether the
  job treats warnings as errors before assuming these are cosmetic. Fix by
  adding the missing `///`, never by loosening the lint.
- **`cargo fmt --check` is a gate.** BCS and nova both format with the pinned
  nightly rustfmt, so a straight copy should be clean; re-run anyway.

## Verification (project rules, non-negotiable)

- `cargo` only runs via `nix develop --command cargo`.
- **Never run the full test suite** - it OOMs the box. Use
  `--lib` with a filter, e.g.
  `nix develop --command cargo test -p nova_gameplay --lib camera::`.
- `nix develop --command cargo check --workspace --all-targets`.
- Examples must be **RUN**, not just checked: `cargo check` does not catch
  duplicate-component panics or a plugin registered twice. Use Xvfb `:99`.
  This matters most for any step that moves plugin registration out of
  `crates/nova_gameplay/src/plugin.rs:81-106`.
- Use the `probe` skill for gameplay-touching steps (camera, physics, mesh).

## Commits

User authorship only. No AI attribution, no co-author trailers. ASCII-adjacent
punctuation in messages (`-`, `--`, `...`, `->`, straight quotes).

## Three corrections to NOTES.md, verified against `6f09461`

NOTES.md is otherwise accurate. These three are wrong and each one is a
compile error if followed:

| NOTES.md says | Reality |
|---|---|
| `src/meth/` - "copy nothing, verified zero references" | True for **nova's** code, false for the **copied** code. Three copied files import it: `camera/chase.rs` (`LerpSnap`), `transform/random_sphere_orbit.rs` (`spherical_to_cartesian`), `mesh/builder.rs` (`slerp`). 143 L must come across. See prototype 03. |
| `serde_json` is "persist + registry only" | `modding/events.rs:156` types `GameEventInfo::data` as `Option<serde_json::Value>`. `nova_events` gains `serde_json` as a NEW direct dep. See prototype 01. |
| `mesh/builder.rs` is one of four files touching `rand` | It touches `noise` only (`noise::NoiseFn`). The `rand` files are three: `mesh/explode.rs`, `camera/shake.rs`, `transform/random_sphere_orbit.rs`. See prototype 06. |

And one item NOTES.md lists as pending that is already done:

- The `nova_probe` `Health` straggler (`invariants.rs`, `capture.rs`) was fixed
  in commit `261c7e71` on this branch; `capture.rs:20-26` now carries a comment
  pinning the `nova_gameplay::prelude` path. Only `recorder.rs` still reaches
  BCS. See prototype 10.

## Scale

~6.5k LOC copied (NOTES.md's "~4.9k" undercounts; the per-file table in each
prototype sums to 6502 including `meth`). The rest of BCS is dropped.

## Dead surface

Copied files carry names nothing uses: `CameraShakeOutput`, `WASDCamera`,
`WASDCameraInput`, `EventHandlerIndex`, `BlastDamageConfig`, several `*Systems`
sets nobody orders against, and `RandomSphereOrbit`'s components (only its
plugin is registered). **Copy them verbatim in this pass.** A dead-code sweep is
a separate follow-up task, not this one.
