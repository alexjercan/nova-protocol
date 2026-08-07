# Workspace map

HEAD `4a8b55aa`, 2026-08-07. 16 `nova_*` crates plus root and `tools/`.

## Sizes

`src/**/*.rs` line counts, tests included.

| Crate | LOC | Files |
| --- | --- | --- |
| nova_gameplay | 77,761 | 169 |
| nova_assets | 16,702 | 32 |
| nova_scenario | 14,678 | 30 |
| nova_probe | 9,890 | 37 |
| nova_menu | 8,154 | 21 |
| nova_ui | 3,703 | 17 |
| nova_autopilot | 2,935 | 9 |
| nova_os | 2,560 | 9 |
| nova_editor | 2,378 | 10 |
| nova_debug | 1,643 | 7 |
| nova_events | 821 | 2 |
| nova_core | 585 | 2 |
| nova_mod_format | 531 | 2 |
| nova_modding | 439 | 1 |
| nova_events_macros | 59 | 1 |
| nova_info | 15 | 1 |
| root `src/` | 36 | 2 |

Counting tests and doc lines, the whole of `crates/` is 155,587 lines across 384
`.rs` files.

`nova_gameplay` is half the workspace and has 8 in-workspace dependents.

## Dependency graph

`->` depends on; `*` optional/feature-gated; `d` dev-only.

```
nova-protocol (root) -> nova_core
                     d  nova_autopilot, nova_probe, nova_debug, nova_ui, nova_modding
nova_core      -> assets, debug*, editor, gameplay, info, menu, scenario, events, ui
nova_assets    -> gameplay, ui, scenario, modding, mod_format;  d events
nova_gameplay  -> events, info, os, ui
nova_scenario  -> events, gameplay
nova_modding   -> mod_format, scenario(serde), gameplay(serde)
nova_menu      -> assets, events, mod_format, gameplay(serde), scenario, ui(serde)
nova_editor    -> assets, gameplay, scenario, ui
nova_debug     -> autopilot, events, gameplay(features=["debug"] FORCED), scenario
nova_probe     -> autopilot, gameplay, events, scenario;  d assets
nova_os        -> info
nova_events    -> events_macros
nova_autopilot, nova_ui, nova_mod_format, nova_info, nova_events_macros -> (no nova deps)
tools/nova_meta_gen -> nova_modding
```

Dependent counts: gameplay 8, scenario 7, events 6, ui 6, assets 4, mod_format
3, info 3, autopilot 3, modding 3, os 1, events_macros 1. Zero in-workspace
dependents: core, editor, menu, debug, probe, meta_gen.

`nova_ui` has zero nova dependencies - the one stated invariant that holds
cleanly.

### Inversions

- `nova_modding` ("engine-free-ish mod format loader") depends on the two
  largest gameplay crates just to deserialize `SectionConfig` / `ScenarioConfig`
  (`crates/nova_modding/src/lib.rs:42,47`). The content schema is conceptually
  below gameplay; here it sits above. Repeated as
  `nova_assets -> nova_modding -> nova_gameplay`.
- `nova_scenario -> nova_gameplay` while gameplay emits the events scenario
  dispatches. Only `nova_events` acting as a shared bottom keeps it acyclic.

## Merge and split candidates

| Crate | Verdict |
| --- | --- |
| `nova_info` | **Merge.** 15 LOC, one const (`APP_VERSION`, `src/lib.rs:9`), a build.rs, a prelude, a dead `debug` feature. Three dependents. Snag: nova_os and nova_gameplay reach it without a nova_core dependency, so `APP_VERSION` may need to land in nova_ui instead. |
| `nova_modding` | **Merge** into `nova_mod_format` or `nova_assets`. 439 LOC, one file, asset loaders only. Already re-exports mod_format wholesale (`src/lib.rs:45`). |
| `nova_gameplay` | **Split.** See `03-nova-gameplay.md`. |
| `nova_assets` | **Split.** Holds portal client (1,866), mod cache (1,178), bundle merge (701), mod refs (954) - all of nova_modding's advertised job - plus content builders and the `content` CLI. |
| `nova_debug` | **Reconsider.** `harness.rs` is 574 of its 1,643 LOC and is a test harness used by 20 examples and nova_probe, not debug tooling. It is the reason the debug feature leaks. |
| `nova_events_macros`, `nova_autopilot`, `nova_mod_format`, `nova_ui` | **Keep.** Proc-macro must be separate; the others have real constraints (bevy-only, engine-free, zero-nova-deps). |

## AGENTS.md crate table is stale

The `nova_modding` row is wrong on 3 of 4 items:

| AGENTS.md claims | Actually at |
| --- | --- |
| bundle merge | `crates/nova_assets/src/merge.rs` (701 LOC) |
| portal client | `crates/nova_assets/src/portal/*` (1,866 LOC), UI in `crates/nova_menu/src/portal.rs` |
| downloads | `crates/nova_assets/src/mod_cache.rs` (1,178 LOC) |
| catalog | half-true - loader is here, builder (`build_mod_catalog`) is in nova_assets |

Real `nova_modding`: one file, `src/lib.rs`, 439 lines. `Content` enum,
`ContentAsset`/`ContentAssetLoader` (`:90,:179`), `BundleAsset`/`BundleAssetLoader`
(`:108,:219`), `CatalogEntry`/`InstalledCatalog`/`CatalogLoader` (`:278,:295,:325`),
`NovaModdingPlugin` (`:363-374`). Its own crate doc (`:1`) calls it "the RON
scenario/mod format", contradicting AGENTS.md.

Also stale: the `nova_events` line (see `01-decisions.md`), and nothing signals
that nova_gameplay is half the workspace.

## Feature flags

`dev` exists only at the root as an alias (`Cargo.toml:262`). `debug` is
declared in 6 crates, used in 3.

| Crate | Declares `debug` | `#[cfg(feature="debug")]` sites |
| --- | --- | --- |
| nova_gameplay | yes | 13 |
| nova_core | yes | 4 |
| nova_info | yes | **0 - dead flag** |
| nova_editor / nova_menu / nova_assets | forwards only | 0 |
| nova_debug | **no feature at all** | 0 |

**The leak:** `nova_debug` is "debug-gated" by being an optional dependency of
nova_core rather than by a feature of its own, yet it hard-forces
`nova_gameplay/debug` (`crates/nova_debug/Cargo.toml:18`). Root lists nova_debug
as an unconditional dev-dependency (`Cargo.toml:224`), so **every
`cargo test` and example build compiles nova_gameplay with `debug` on, with or
without `--features debug`.**

`nova_probe` never declares `debug` but documents that it always builds with it
(`crates/nova_probe/src/contract.rs:17`). Root `src/main.rs:11-30` cfg-gates
`--norender` and `--debugdump`, so they vanish from `--help` in a release build
with no message.

## Prelude discipline

Preludes exist in 12 of 17 crates. Missing: `nova_events_macros` (fine),
`nova_mod_format` (15 deep-path uses), `nova_probe` (**184** - worst in the
workspace), `nova_meta_gen` (bin).

Deep-path `use nova_x::<not prelude>` counts, whole workspace:

probe 184, gameplay 88, ui 86, assets 76, `nova_protocol::nova_debug` 69,
autopilot 16, scenario 15, mod_format 15, debug 9, os 8, events 7, core 4,
events_macros 3, menu 2, info 2, modding 1, editor 1.

Restricted to `crates/*/src` only: gameplay 87, ui 81, probe 46, assets 20,
mod_format 14, scenario 13, autopilot 11, debug 9, os 8, events 6.

- `nova_ui`'s prelude is **effectively dead**: 81 in-src deep uses of
  `theme`/`units`/`hud`/`widget`/`font`/`tween`/`status_bar` against 3 prelude
  imports.
- `nova_gameplay`'s prelude lives in `src/relations.rs:14`, not `lib.rs` - a
  discoverability bug. Leaks: `hud` 37, `sections` 11, `integrity` 8.
- Clean: events (88 prelude vs 7 deep), scenario (65 vs 15), modding (27 vs 1),
  os (15 vs 8).

## Visibility and dead surface - amended 2026-08-07 by the review

The original note recorded merge/split candidates but said nothing about how
much of each crate's public surface is actually crate-local. The cross-cutting
sweep measured it (`13-review-cross-cutting.md`).

| Metric | Value |
| --- | --- |
| `pub` items never referenced outside their own crate | **633** |
| nova_gameplay share | 358 (~55% of its public surface) |
| nova_assets / nova_probe / nova_scenario / nova_ui | 88 / 61 / 56 / 34 |
| Truly unreferenced anywhere (dead) | **0** - 13 of 54 suspected candidates hand-checked, all used |

Two consequences for this note:

- The `nova_gameplay` "16 pub items referenced only by their own prelude
  re-export" line in `03-nova-gameplay.md` is the tip of a 358-item pattern.
  It is over-broad visibility, not dead code.
- `#![warn(unreachable_pub)]` does **not** catch this class. It needs an
  identifier cross-reference or `cargo-public-api`.

Directly relevant here: **splitting `nova_gameplay` four ways forces this
question anyway**, because each seam must decide what crosses its boundary.
The visibility audit is therefore free work inside the split lanes, not a
separate pass.

**Verified 2026-08-07:** `NovaOsShipSystems` (`hud/nova_os_ship/mod.rs:166`)
and `NovaOsMapSystems` (`nova_os_map/mod.rs:139`) have zero references outside
their own defining file - they are not even prelude-re-exported. They are
declared as `SystemSet`s, used once each in their own `.in_set(..)`, and never
passed to `configure_sets`, so they carry no ordering edge at all. See
`10-review-hud-nova-os.md`.

### Dead surface: two whole features, found by the review

`12-review-ui-layer.md` proved two features dead by consumer count rather than
by identifier reference - a different measurement from the 633 above, and the
reason the two results do not contradict each other:

| Feature | Evidence |
| --- | --- |
| `nova_ui::tween` (421 lines, 11 tests) | **Zero `Tween<T>` spawned anywhere outside its own tests.** VERIFIED 2026-08-07: `grep -rnE "Tween<\|Tween::new\|TweenFinished" crates/ src/ examples/` minus `tween.rs` returns nothing. `TweenPlugin` IS registered (`nova_gameplay/src/hud/mod.rs:301`, verified) and runs four empty queries every frame |
| `nova_ui::status_bar::StatusBarStore` | Declared at `status_bar.rs:133`, `init_resource`d at `:153`, never read or written. VERIFIED - those are the only two hits workspace-wide |

`tween`'s items are *referenced* (by the plugin and by its own tests), which is
why the identifier sweep counted zero dead items and the consumer audit still
found a dead subsystem. Both measurements are correct; they answer different
questions.

## nova_core AppBuilder

`crates/nova_core/src/lib.rs:74-187`. Order in `build()` is explicit, one plugin
per line (`:137-163`), and matches AGENTS.md. What is hard to follow:

- Assembly is **split across two methods**. `new()` silently adds
  `DefaultPlugins` plus a pre-AssetPlugin side effect (`register_mods_source`,
  `:96`) that must precede it. Reading `build()` alone misses half the stack.
- `use_default_plugins` means four things at once: no custom game plugins, add
  the editor, front with the main menu, go to MainMenu not Playing
  (`:151-179`).
- The `OnEnter(Loaded)` handoff is an inline closure capturing `main_menu`,
  tangled with an unrelated `setup_status_ui` in the same tuple (`:168-183`).
- `setup_status_ui` (`:276-303`) is HUD content in the composition root, and is
  the only reason nova_core depends on nova_ui (`:33-36`, deep path).
- `log_filter_str` (`:229-242`) hand-lists nine crate names twice and omits
  nova_menu, nova_editor, nova_modding, nova_os, nova_probe. Drift trap.
