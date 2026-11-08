# Migrate all Nova crates to Bevy 0.20 after ecosystem gate

- STATUS: OPEN
- PRIORITY: 90
- TAGS: v0.17.0

## User facts
- First v0.17.0 work: investigate Bevy 0.20.0 using the official release notes and migration guide. If all Nova crates can work safely, migrate directly on master and commit. Do not ship a mixed 0.19/0.20 game.

## Agent findings (2026-10-29, read-only)
- Root `Cargo.toml:1534` and 26 internal crate manifests require Bevy 0.19; `Cargo.lock` resolves 0.19.1. The Bevy 0.20.0 sparse index entry exists and needs Rust >=1.97.1; the pinned nightly is rustc 1.98.0-nightly (`rust-toolchain.toml`, `flake.nix:37`).
- Published ecosystem updates for Avian 0.8.0, enhanced-input 0.27.2, asset-loader 0.28.1, rand 0.16.0, transform-interpolation 0.6.0, inspector-egui 0.38.0, and debugdump 0.17.0 declare Bevy 0.20. The `bevy_rand` 0.16.0 index preserves `wyrand`, `chacha20`, `wasm_js` features. These versions have NOT been compiled together in Nova.
- BLOCKER: the latest published `bevy_hanabi` is 0.19.0; its two Bevy dependencies require `^0.19`, not `^0.20` (https://index.crates.io/be/vy/bevy_hanabi). Upstream main has `version = "0.20.0-dev"` and a Bevy `0.20` requirement, but has no 0.20 release/tag (https://github.com/djeedai/bevy_hanabi). Nova uses Hanabi across gameplay, ship, and scenario for mining, pyres, debris, PDC muzzle, torpedoes and railguns (`crates/nova_gameplay/src/plugin.rs:105`, `crates/nova_scenario/src/mining.rs:66`, `crates/nova_ship/src/sections/torpedo_section/render.rs:228`). Mixing Hanabi 0.19 and Bevy 0.20 would retain two engine type trees, not migrate all crates. Silently dropping particles or switching to an untagged development dependency does not satisfy the user's safety gate.
- Official sources: https://bevy.org/news/bevy-0-20/ and https://bevy.org/learn/migration-guides/0-19-to-0-20/. Relevant changes include retained UI rendering, deprecated UI interaction/button, 2D BorderRadius, query-many Result iteration, NextState::set_if_different and rendering/plugin ordering; verify against code and generated content before edits.

## Agent findings (2026-11-08)
- Owner: "migrate to bevy 0.20 ... first validate that is the case, if it's ok then update". GATE PASSES. Sparse index read live 2026-11-08 (`https://index.crates.io/<prefix>/<crate>`), newest non-yanked release per direct Bevy-ecosystem dependency:

| crate | lock now | new | bevy requirement | rust-version |
|---|---|---|---|---|
| bevy | 0.19.1 | 0.20.1 | (self) | 1.97.1 |
| bevy_hanabi | 0.19.0 | 0.20.0 | bevy ^0.20 (no default features; `2d`, `3d` exist) | 1.97.1 |
| avian3d | 0.7.0 | 0.8.0 | bevy/bevy_math/bevy_curve/bevy_shape ^0.20, bevy_heavy ^0.6, bevy_transform_interpolation ^0.6 (`diagnostic_ui` exists) | - |
| bevy_enhanced_input | 0.26.0 | 0.27.2 | bevy ^0.20 | - |
| bevy_asset_loader | 0.27.0 | 0.28.1 | bevy_* ^0.20 | 1.96.0 |
| bevy_rand / bevy_prng | 0.15.0 | 0.16.0 | bevy_ecs/app/reflect ^0.20 (`wyrand`, `chacha20`, `wasm_js` exist) | 1.97.1 |
| bevy_transform_interpolation | 0.5.0 | 0.6.0 | bevy ^0.20 | - |
| bevy-inspector-egui | 0.37.0 | 0.38.0 | bevy_* ^0.20, bevy_egui ^0.43 | - |
| bevy_egui (transitive) | 0.40.1 | 0.43.1 | bevy_* ^0.20 | 1.95.0 |
| bevy_mod_debugdump | 0.16.0 | 0.17.0 | bevy_* ^0.20 | - |
| bevy_heavy / glam_matrix_extras (transitive) | 0.5.0 / 0.3.0 | 0.6.0 / 0.4.0 | ^0.20 | - |

- Bevy feature names Nova enables all exist in 0.20.1: `track_location`, `webp`, `wav`, `reflect_documentation`, `system_clipboard`, `webgpu`, `serialize`, `trace`, `trace_chrome`. Pinned nightly rustc 1.98.0 meets the 1.97.1 floor.
- Manifests: root `Cargo.toml`, 26 `crates/*/Cargo.toml`, and `tools/nova_meta_gen/Cargo.toml` (bevy 0.19, outside `crates/`; missing it left a second 0.19 tree in the first trial). No git or path dependency reaches Bevy.
- Single tree proven in sprout `bevy-020-gate` (manifest bumps + `cargo update`): lockfile holds one version of every `bevy*` crate (0.20.1 family), one `wgpu`/`naga` 30.0.1, one `egui` 0.36.2, one `winit` 0.30.13. `cargo tree -d --workspace --all-features -e normal,build,dev` and `cargo tree -d --workspace --target wasm32-unknown-unknown` list no `bevy*`, `wgpu` or `naga` duplicate. Remaining duplicate `glam` 0.33 (bevy_math) + 0.34 (avian's `obvhs`) is not an engine tree; master already carries glam 0.32 + 0.33.
- Migration guide (https://bevy.org/learn/migration-guides/0-19-to-0-20/) kills no feature Nova needs. Work it implies here: all 9 `assets/shaders/*.wgsl` use naga_oil `#import` and must become WESL (naga_oil is gone); ~213 lifecycle observers `On<Add, T>` -> `On<Add<T>>`; flat pointer events; `iter_many` Result items; `set_if_neq` -> `set_if_different`; deprecated `ui::widgets::Button`/`Interaction`; 2D `BorderRadius`; `TextFont` default size now `Rem(1.)`; `Name: From<&'static str>`; ScreenSpaceTransmission opt-in; weak set ordering in render/UI schedules.

## Delivery
- Gate on a published, compatible Hanabi 0.20 release or an explicitly approved maintained replacement. Recheck all direct and transitive dependencies and a single Bevy version in the resolved graph before editing master. Keep shipped particle effects and authored IDs intact. Once safe, change owning manifests, lockfile, plugin/API call sites, docs and changelog together. Delete obsolete interfaces, never add a 0.19 bridge or silent fallback.
- Expected graph: `AppBuilder -> GameplayPlugin -> HanabiPlugin (0.19) -> Bevy 0.19`; required graph: `AppBuilder -> GameplayPlugin -> supported particles -> Bevy 0.20` with one Bevy tree across native/wasm/examples.

## Verification / Done when
- On master under pinned Nix nightly: resolved graph has one Bevy 0.20 family; all workspace crates/targets compile native and wasm, affected focused tests and examples pass, full release-critical probes and rendered particle/UI paths pass under isolated X (never DISPLAY=:0), and exact-HEAD CI is green. Record Windows/macOS build evidence or explicit limits. Commit coherent changes, not an unsafe partial migration. No implementation has begun while Hanabi lacks a compatible published release.
