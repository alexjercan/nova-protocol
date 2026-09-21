# Full game code review graph

- Task: `20260920-222640`
- Baseline: `b7a56f0586`
- Review slug: `full-game-codebase-state`
- Mode: repository-state review, not a change-range review
- Constraint: no more than two active subagents; no fixes

## System graph

```text
src/main.rs
  -> nova_core::AppBuilder
     -> nova_input
     -> nova_assets -> nova_modding -> nova_mod_format
     -> nova_gameplay -> nova_events
     -> nova_ship -> nova_gameplay + nova_input + nova_events
     -> nova_scenario -> nova_ship + nova_gameplay + nova_assets + nova_events
     -> nova_hud -> nova_ship + nova_gameplay + nova_ui
     -> nova_os_ui -> nova_os + nova_hud
     -> nova_editor -> nova_scenario + nova_ship + nova_ui + nova_wfc
     -> nova_menu -> nova_scenario + nova_hud + nova_os_ui + nova_training
     -> nova_console -> nova_os + nova_os_ui + gameplay surfaces
     -> nova_debug -> nova_autopilot + runtime surfaces

content source
  nova_authoring Rust builders
    -> generated assets/base/**/*.content.ron
    -> nova_modding loaders
    -> nova_assets merge and registries
    -> scenario/ship/UI/training runtime ID consumers

proof path
  examples + nova_autopilot
    -> in-process nova_probe capabilities
    -> nova_probe_cli artifacts and verdicts
  nova_bench agent <-> nova_channel <-> game snapshots
  nova_perf_web -> generated probe reports
```

## Ordering seams to inspect

```text
Bevy -> input -> assets -> gameplay -> ship -> scenario -> HUD -> OS UI -> editor/menu -> console -> debug

Update and FixedUpdate:
SpaceshipSystems::First
  -> SpaceshipInputSystems
  -> SpaceshipSectionSystems
  -> NovaHudSystems
  -> NovaCameraSystems
  -> SpaceshipSystems::Last

NOVA OS frame:
NovaOsSystems::Input -> ConsoleSystems::Dispatch -> Simulate -> Paint

Asset state:
Loading -> Processing/merge -> Loaded
  -> RuntimeScenarioSystems -> boot_into_the_game -> scenario load
```

## Review slices and planned pairs

Each pair covers all mandatory Nova Review lanes. Reviewer A reads `reviewer.md`, `craft.md`, and `performance.md`. Reviewer B reads `reviewer.md`, `correctness.md`, and `contracts.md`. Each report must state exact files read, grep-only surfaces, checks run, and unchecked files. Specialist agents are used only for the ECS, content, and proof slices identified by scouting.

| Batch | Slug | Primary scope | Specialist follow-up trigger |
|---|---|---|---|
| 01 | assembly-input-events | `src/`, `nova_core`, `nova_input`, `nova_events`, `nova_events_macros`, `nova_info` | ECS if startup or schedule ownership is uncertain |
| 02 | gameplay-damage-projectiles | `nova_gameplay` damage, rounds, blast, relations, cheats | ECS for collision/event ordering |
| 03 | gameplay-integrity-physics | `nova_gameplay` integrity, carve/pyre/spew, gravity, bounds, physics hooks, settings | ECS or performance if hot paths survive adjudication |
| 04 | gameplay-render-audio | Remaining `nova_gameplay`: audio, juice, lights, particles, rendering helpers, markers | Performance only with an existing measurable range |
| 05 | ship-flight-input-ai | `nova_ship` flight, player input, AI, targeting, point defense, physics controller | ECS for schedule/state transitions |
| 06 | ship-weapons-sections | `nova_ship` turret, torpedo, ammo, controller, thruster and weapon section paths | ECS for fire/consume/despawn ordering |
| 07 | ship-integrity-skin-camera | Remaining `nova_ship`: integrity, shell/skin, base sections, camera, audio, tests | Performance for rebuild or mesh churn claims |
| 08 | scenario-lifecycle-world-events | `nova_scenario` loader, world, filters, event engine integration, trackers | ECS for lifecycle/gating and reader ownership |
| 09 | scenario-actions-objects-lint | `nova_scenario` actions, objects, lint, test support | Content specialist for authored IDs and lint/load agreement |
| 10 | assets-mods-portal | `nova_assets`, `nova_modding`, `nova_mod_format` | Content specialist for refs, overlays, formats, wasm/native persistence |
| 11 | authoring-base-content | `nova_authoring`, Rust base builders, generated RON parity, `assets/`, `webmods/` IDs | Content specialist required; no generation |
| 12 | ui-foundation-hud | `nova_ui`, `nova_hud` | ECS/performance for reconciler and render-update claims |
| 13 | menu-training | `nova_menu`, `nova_training` | ECS for pause/store/state transitions |
| 14 | os-console | `nova_os`, `nova_os_ui`, `nova_console` | ECS for Input/Dispatch/Simulate/Paint ordering |
| 15 | editor-model-io | `nova_editor` node/document, inspect, scenario, event, bundle, generation and file I/O | Content specialist for IDs/serialization |
| 16 | editor-ui-interaction | `nova_editor` UI, placement, stage, gizmo, skin, gallery, highlight | ECS/performance for reconcile and interaction hot paths |
| 17 | probe-evaluation | `nova_probe`, `nova_probe_cli`, `nova_perf_web`, system-example proof contracts | Proof specialist required for artifact/verdict trust boundaries |
| 18 | bench-channel-debug | `nova_bench`, `nova_autopilot`, `nova_channel`, `nova_debug` | Proof specialist required for process, tick, and snapshot semantics |
| 19 | wfc-examples | `nova_wfc`, representative playable/system/screenshot example infrastructure, catalog contracts | Proof specialist if an example claim is not actually asserted |
| 20 | platform-web-ci-scripts | Cargo features/config, Nix/toolchain, wasm/native gates, workflows, scripts, web TypeScript/build code, code-linked docs | Contracts focus; exclude prose-only editorial review and binary art inspection |
| 21 | systems-examples-a | Full reads of `examples/systems/` from `bug_*` through `system_headless_replay`, including shared/nested modules | Proof specialist after reviewer adjudication if assertions are weak |
| 22 | systems-examples-b | Full reads of remaining `examples/systems/` from `system_helm_orders` through `system_wreck_lock`, including nested modules | Proof specialist after reviewer adjudication if assertions are weak |
| 23 | screenshot-examples | Full reads of every `examples/screenshots/` body and shared module not fully covered in batch 19 | Check capture claims, completion, visual-only limits, and duplicated harness logic |
| 24 | web-scripts-remainder | Full reads of web tests/helpers, remaining scripts/generators, illustration tests, platform build files, and other code-bearing batch-20 gaps | Contracts and deterministic generation focus; no binary-art judgment |
| 25 | final-code-edges | `tools/nova_bench/pi/index.ts`, root tests, remaining build metadata/CSS/JS, hooks and small code-bearing files not explicitly closed above | Final inventory closure; binary assets remain excluded |

## Coverage accounting

### Crates

| Crate | Batch |
|---|---|
| nova_assets | 10 |
| nova_authoring | 11 |
| nova_autopilot | 18 |
| nova_bench | 18 |
| nova_channel | 18 |
| nova_console | 14 |
| nova_core | 01 |
| nova_debug | 18 |
| nova_editor | 15-16 |
| nova_events | 01 |
| nova_events_macros | 01 |
| nova_gameplay | 02-04 |
| nova_hud | 12 |
| nova_info | 01 |
| nova_input | 01 |
| nova_menu | 13 |
| nova_modding | 10 |
| nova_mod_format | 10 |
| nova_os | 14 |
| nova_os_ui | 14 |
| nova_perf_web | 17 |
| nova_probe | 17 |
| nova_probe_cli | 17 |
| nova_scenario | 08-09 |
| nova_ship | 05-07 |
| nova_training | 13 |
| nova_ui | 12 |
| nova_wfc | 19 |

### Other code-bearing surfaces

| Surface | Batch |
|---|---|
| Root binary/library and workspace features | 01, 20 |
| Generated base content and source builders | 11 |
| Mod examples and webmods | 10-11 |
| Examples and probe catalog | 17, 19 |
| Web TypeScript/build system | 20 |
| CI, Nix, Cargo config, scripts | 20 |
| Code-linked developer/player/creator docs | Owning batch, final contract audit |

### Explicit exclusions

- Binary meshes, textures, sounds, fonts, screenshots, and videos are not decoded or judged as code.
- Narrative and marketing prose is checked only when it states a code contract.
- Dependency vulnerability auditing is not part of this code review.
- No workspace test, workspace Clippy, content generation, source edit, or content edit.
- Runtime or GPU measurements run only if a reviewer identifies a plausible cost and an existing focused range. Only one measurement process may run at once.

## Scout coverage

Six scouts ran in three pairs. They mapped:

1. app assembly, plugin order, dependency tiers, and startup;
2. runtime gameplay, ship, input, events, scenario hooks, channel and autopilot;
3. content builders, loaders, merge, lint, registries, IDs, mods and portal;
4. UI, HUD, menu, NOVA OS, console, editor and training;
5. probes, evaluation, bench, autopilot, channel, examples and CI proof boundaries;
6. root CLI/features, wasm/native boundaries, workflows, scripts, web build, assets and examples catalog.

Scout claims are leads, not findings. Reviewers must re-read source and ground any finding independently.

## Completion rule

After batch 20, audit every report's `Checked` and `Not checked` sections against this graph. Add paired review batches for material files that were only grep-scanned, sampled, or omitted. Then write `REVIEW-full-game-codebase-state.md` with adjudicated findings, duplicates merged, unsupported claims dropped, and residual coverage limits named.
