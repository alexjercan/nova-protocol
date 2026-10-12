# World first: research note and decision 14 shape

- STATUS: CLOSED
- PRIORITY: 98
- TAGS: v0.16.0, world-first

## User facts
- Epic `20261012-095526`. Owner direction 2026-10-12: the open world is the main mode; scenarios and `nova_events` are a harness that bounds it. Decision 14 on the design page is OPEN; this task closes it with a code-backed note the owner accepts before any code in the strand.
- Design: `tasks/20261012-095526/DESIGN.html` (owner-reviewed page, version 19); codebase facts in `tasks/20261012-095526/RESEARCH-CODEBASE.md`.

## Agent findings
- Arming is keyed on `ScenarioRole::OpenWorld` (`crates/nova_world_base/src/lib.rs:248-252`); the role has four readers (arming, `crates/nova_scenario/src/lint/scenario.rs:88`, `crates/nova_probe/src/capabilities/snapshot.rs:437`, `:2202`, `crates/nova_world_base/src/tests.rs:348-495`).
- The page recommends option 1: a `world` block on the scenario (`Generated { seed: Option<u32> }` or `Void`) with an optional bounds area. Options 2 (role stays the gate, fixed seeds) and 3 (a separate harness content kind) are listed with one consequence each.

## Research note (2026-10-12)
- [RESEARCH.md](RESEARCH.md): shape 1 detailed with types (`ScenarioWorldType::{Void, Generated(GeneratedWorldConfig)}`, `ScenarioBoundsConfig`), the readers that move, what every shipped scenario declares (only `open_world` changes), before/after call graphs, six lint rules, and three owner decisions. Owner pick 2026-10-12: shape 1, `world` defaults to `Void`, fixed-seed chapters listed in the picker. The next three strand tasks name the picked types.

## Delivery gate
- Write `RESEARCH.md` in this folder: exact types and fields for the `world` block, where the loader reads it, how `sync_open_world` arms from it, what the New Game wizard writes, what every shipped chapter and backdrop declares, the bounds semantics, and the lint rules (a generated-world harness with no player spawn; bounds larger than the active window, `crates/nova_world_base/src/lib.rs:139-145`). Compact before/after call graph for New Game, Play scenario and Retry.
- Name what dies (`ScenarioRole::OpenWorld`, the "world is the scenario" framing, the void as implicit default), what may break (the four readers, the picker's no-world assumption) and what must fail loudly.
- Stop for the owner's pick. No code edits.

## Done when
- The note is in this folder, the owner has picked a shape in it, and the next three strand tasks reference the picked types by name.
