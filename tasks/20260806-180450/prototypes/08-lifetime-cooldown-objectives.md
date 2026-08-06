# Prototype 08 - lifetime, cooldown, objectives -> `nova_gameplay`

Three unrelated small modules, grouped because each is one flat file and none
warrants its own commit. The objectives half is the only one with judgement in
it.

## Scope

| From (BCS @ 6f09461) | LOC | To |
|---|---|---|
| `src/helpers/temp.rs` | 115 | `crates/nova_gameplay/src/lifetime.rs` (merged) |
| `src/helpers/despawn.rs` | 56 | `crates/nova_gameplay/src/lifetime.rs` (merged) |
| `src/time/cooldown.rs` | 177 | `crates/nova_gameplay/src/cooldown.rs` |
| `src/ui/objectives.rs` | 171 | `crates/nova_gameplay/src/objectives.rs` (partial) |

## 8a. `lifetime.rs`

Two BCS files, one nova module. `temp.rs` is `TempEntity` (an entity that
despawns after a timer), `despawn.rs` is `DespawnEntity` (a marker that
despawns on insert). Both are the same concern - entity lifetime - and neither
is a "helper".

Kill the `helpers` name entirely; nothing else from `src/helpers/` comes across
(`pointer.rs` is unreferenced, `wasd.rs` went to the camera in prototype 03).

Exports: `TempEntity`, `TempEntityPlugin`, `DespawnEntity`, `DespawnEntityPlugin`.

Callsites:

| File | Line | What |
|---|---|---|
| `nova_gameplay/src/plugin.rs` | 98, 99 | `TempEntityPlugin`, `DespawnEntityPlugin` |
| `nova_gameplay/src/sections/torpedo_section/mod.rs` | 16 | `TempEntity` |
| `nova_gameplay/src/sections/turret_section/firing.rs` | 8 | `TempEntity` |

Both section files import `TempEntity` alongside `rigid_body_point_velocity`
(prototype 05). Sequence 08 before 05 to fix each `use` line once.

`TempEntity` is not in `nova_gameplay/src/lib.rs:77`'s re-export list - the two
callers name BCS directly. Add it to the crate prelude when it lands.

Neither file has an external dep. 1 `bevy_common_systems` doctest string each.

## 8b. `cooldown.rs`

Straight copy to `crates/nova_gameplay/src/cooldown.rs`. `Cooldown` is the only
export that matters (BCS's `time` module holds nothing else worth taking - its
`mod.rs` is a 40-line "timed spawner" recipe for a BCS example; drop it).

`Cooldown` **is** in the prelude re-export list (`lib.rs:77`), so it must keep
resolving from `nova_gameplay::prelude`. Consumers: the torpedo bay's fire gate
and the AI threat memory - `grep -rn 'Cooldown' crates/` to find them; they all
reach it through the prelude glob, so no import churn is expected.

No external dep. 1 `bevy_common_systems` doctest string.

## 8c. `objectives.rs` - partial copy, and a name that frees up

BCS's `ui/objectives.rs` is two things welded together:

- **Mission state** - `Objective`, `GameObjectives`, `ObjectivesPlugin`,
  `ObjectivesPluginSystems`. **Copy this.** It is not a widget; it is the
  objective list the game reasons about.
- **A panel** - `ObjectivesPanelConfig`, `ObjectivesPanelMarker`,
  `objectives_panel`, `ObjectiveMarker`. **Drop this.** Unused: nova draws
  objectives in `crates/nova_gameplay/src/hud/objective_stack.rs`.

Land the mission half at `crates/nova_gameplay/src/objectives.rs`, and merge it
with the existing `crates/nova_gameplay/src/objective_marker.rs` - that file is
70-odd lines of `ObjectiveMarkerTarget` + `ItemHighlight`, the conveyance tags
the scenario attaches to world entities. Same concern, two files.

**A rename becomes possible and you should not take it.**
`objective_marker.rs:19-21` says:

> Named `ObjectiveMarkerTarget`, not `ObjectiveMarker` - bevy_common_systems
> already uses that name for the objectives panel's text lines.

Once the panel half is dropped, `ObjectiveMarker` is free. Renaming
`ObjectiveMarkerTarget` -> `ObjectiveMarker` is a public API change across
`nova_scenario` and the HUD, and it is not this task. **Update the comment to
say the name is now free and why it was not taken**, and file the rename as a
follow-up if the owner wants it.

Exports that must survive: `GameObjectives`, `Objective`, `ObjectivesPlugin` -
all three are in `lib.rs:77`'s re-export list.

Callsites:

| File | Line | What |
|---|---|---|
| `nova_gameplay/src/hud/nova_os/components.rs` | 2 | `Objective` |
| `nova_gameplay/src/hud/nova_os/content.rs` | 2 | `GameObjectives` |
| `nova_gameplay/src/hud/nova_os/input.rs` | 9 | `GameObjectives` (+ `SoundBank`, prototype 07) |
| `nova_gameplay/src/hud/nova_os/lists.rs` | 2 | `GameObjectives`, `Objective` |
| `nova_gameplay/src/hud/nova_os/mod.rs` | 63 | `GameObjectives` |
| `nova_gameplay/src/hud/nova_os/tests/mod.rs` | 29 | `GameObjectives`, `Objective` |
| `nova_gameplay/src/hud/objective_stack.rs` | 36, 557, 963 | `GameObjectives`, `Objective` |
| `nova_scenario/src/actions/mission.rs` | 5, 13 | glob; the doc at :13 calls `Objective` "the generic `bevy_common_systems` `Objective`" - reword |
| `nova_scenario/src/world.rs` | 58, 411, 485 | `GameObjectives`; the comment at :58 says "the generic bevy_common_systems Objective the HUD renders" - reword |
| `nova_scenario/src/loader/lifecycle.rs` | 716, 791, 795 | `GameObjectives`, `GameEventsPlugin` |
| `nova_scenario/src/loader/clock.rs` | 122, 175, 272, 344, 433 | test-local `GameObjectives` |
| `nova_scenario/src/loader/trackers.rs` | 248, 374, 526 | test-local `GameObjectives` |
| `nova_scenario/src/objects/{area,asteroid,salvage}.rs` | 170, 762, 343 | test-local `GameObjectives` |
| `nova_assets/src/scenario/shakedown/tests/walk.rs`, `nova_assets/tests/*.rs` | - | check the braced lists for `GameObjectives` |

`ObjectivesPlugin` registration: it is **not** in `plugin.rs:81-106`. Find it
(`grep -rn 'ObjectivesPlugin' crates/`) before moving it; keep the count at one.

No external dep. 1 `bevy_common_systems` doctest string.

## Compile hazards, all three

- `#![warn(missing_docs)]` on every copied `pub mod prelude` and pub item.
- The three modules land at the crate root, so `lib.rs` gains
  `pub mod cooldown; pub mod lifetime; pub mod objectives;` and the crate
  docstring (`lib.rs:1-10`) - which enumerates the modules - needs updating.
  `objective_marker` disappears from that list if you merge it.
- Update `lib.rs`'s module list in the same edit as the `pub mod` lines, or the
  doc drifts silently.

## Verification

```
nix develop --command cargo check -p nova_gameplay --all-targets
nix develop --command cargo check --workspace --all-targets
nix develop --command cargo test -p nova_gameplay --lib objectives
nix develop --command cargo test -p nova_gameplay --lib hud::objective_stack
nix develop --command cargo test -p nova_gameplay --lib hud::nova_os
nix develop --command cargo test -p nova_scenario --lib
nix develop --command cargo fmt --check
```

`TempEntityPlugin` / `DespawnEntityPlugin` move out of `plugin.rs:98-99`, so
**run** under Xvfb `:99`. A `ui/` example exercises the NOVA OS objective
lists; a `sections/torpedo_section` run exercises `TempEntity` and `Cooldown`
together (a torpedo is a temp entity fired through a cooldown gate).

## Done when

- `helpers` is gone as a concept; `lifetime.rs` holds both halves.
- `cooldown.rs` and `objectives.rs` exist at the crate root; `objective_marker.rs`
  is merged into the latter.
- The objectives **panel** half was not copied.
- `ObjectiveMarkerTarget` keeps its name, with a comment explaining why.
- `GameObjectives`, `Objective`, `ObjectivesPlugin`, `Cooldown` still resolve
  from `nova_gameplay::prelude`.
