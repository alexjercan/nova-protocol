# Move the automation harness into nova_autopilot

- STATUS: OPEN
- PRIORITY: 100
- TAGS: v0.10.0, tooling, autopilot, crates
- KIND: EPIC
- FLOW STEP: BACKLOG
- PLAN STATUS: DRAFT
- PARENT: 20260802-115955

## Epic

Create `crates/nova_autopilot` and move Nova's automation driver out of
`bevy-common-systems`. Nova owns the state/input runway, loop and
self-completion behavior, collector completion, the settled screenshot driver,
and the reel driver its capture examples need. The activation contract renames
from `BCS_*` to `NOVA_*` with no compatibility aliases.

Delivered as small children so each lands on its own: shell, three driver
ports, a runnable example, docs, one atomic migration, one cleanup.

## Design Constraints

- Almost standalone: `nova_autopilot` depends on `bevy` and nothing else. No
  `nova_*` crate, no `bevy_common_systems`, no `avian3d`.
- Nova-shaped, not generic: env names, defaults, and API vocabulary are Nova's.
  The `S: States + FreelyMutableState` generic stays, because it is exactly what
  keeps `nova_gameplay::GameStates` out of the crate.
- Game reach-ins become caller hooks: scenario-camera posing, rigid-body
  freezing, HUD and debug-overlay hiding stay in `nova_debug` and are passed in.
- Dependency direction is one way: `nova_debug` and `nova_probe` depend on
  `nova_autopilot`; the crate never depends back.
- The crate carries its own runnable example and tests, so `nova_probe` can
  later add profiling runs on the same seam.

## Done Means

- `nova_autopilot` owns the automation and completion APIs used by Nova.
  (cmd: `nix develop --command cargo test --lib -p nova_autopilot`)
- The crate is standalone.
  (cmd: `! rg -n "nova_|bevy_common_systems|avian3d" crates/nova_autopilot/Cargo.toml`)
- Nothing outside historical task records names a BCS harness path or env.
  (cmd: `! rg -n "BCS_AUTOPILOT|BCS_SHOT|BCS_REEL|BCS_HARNESS_DEADLINE|debug::harness" --glob '!tasks/**' --glob '!web/src/news/**'`)
- The example fleet and a probe run still pass under the renamed contract.
  (cmd: `nix develop --command cargo test --test examples_smoke`)
- Public items are exported through the crate prelude and rustdoc is clean.
  (cmd: `nix develop --command env RUSTDOCFLAGS=-Dwarnings cargo doc -p nova_autopilot --no-deps`)

## Child Tasks

| ID | Priority | Title | Landed result |
| --- | ---: | --- | --- |
| `20260802-183336` | 99 | Scaffold the standalone `nova_autopilot` crate | Pending |
| `20260802-183340` | 98 | Port the harness completion protocol | Pending |
| `20260802-183343` | 97 | Port the scripted autopilot driver | Pending |
| `20260802-183346` | 96 | Port the single-shot screenshot driver | Pending |
| `20260802-183349` | 95 | Move the screenshot reel driver behind caller hooks | Pending |
| `20260802-183352` | 94 | Add a runnable example with a headless integration test | Pending |
| `20260802-183355` | 93 | Document the crate: rustdoc, prelude, dev wiki page | Pending |
| `20260802-183403` | 92 | Migrate `nova_debug`, `nova_probe`, and the example fleet | Pending |
| `20260802-183406` | 91 | Retire the BCS harness surface and refresh the docs | Pending |

## Notes

- Source behavior: `/home/alex/personal/bevy-common-systems/src/debug/harness/`
  and `src/completion.rs`.
- Current Nova wrapper: `crates/nova_debug/src/harness.rs` (presets, scenario
  smoke assertion, reel driver).
- Env rename map: `BCS_AUTOPILOT` -> `NOVA_AUTOPILOT`, `BCS_SHOT` ->
  `NOVA_SHOT`, `BCS_REEL` -> `NOVA_REEL`, `BCS_HARNESS_DEADLINE` ->
  `NOVA_AUTOPILOT_DEADLINE`. `NOVA_SHOT_DIR` is unchanged.
- Deliberate behavior change: the reel joins the completion protocol instead of
  writing `AppExit::Success` itself.
- Nova-specific checkpoint scripting is out of scope here; it is
  `20260802-120025`.
- No changes land in the BCS checkout (epic decision
  `tasks/20260802-115955/DECISION.md`).
