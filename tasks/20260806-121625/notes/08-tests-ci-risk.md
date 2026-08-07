# Tests, CI, churn and risk

Tests are **out of scope** for this epic - the owner will create a separate
task. This file exists because the refactor's safety depends on knowing what
does and does not pin behavior.

## Test inventory

~1,638 `#[test]` functions; 31 integration-test files under `crates/*/tests/`
plus `tools/nova_meta_gen/tests/`; 24 cataloged examples, all autopilot/probe
targets.

| Crate | `#[test]` | Integration files | src LOC |
| --- | --- | --- | --- |
| nova_gameplay | 861 | 18 (via `src/*/tests/`) | 77,761 |
| nova_assets | 261 | 22 | 16,702 |
| nova_scenario | 152 | 1 | 14,678 |
| nova_probe | 133 | 2 | 9,890 |
| nova_menu | 76 | 10 (`src/tests/`) | 8,154 |
| nova_autopilot | 55 | 5 | 2,935 |
| nova_ui | 32 | 0 | 3,703 |
| nova_os | 20 | 0 | 2,560 |
| nova_debug | 16 | 0 | 1,643 |
| nova_editor | 13 | 0 | 2,378 |
| nova_mod_format | 9 | 0 | 531 |
| nova_core | 5 | 1 | 585 |
| nova_events | 4 | 0 | 821 |
| nova_modding | 1 | 0 | 439 |
| nova_events_macros | 0 | 0 | 59 |
| nova_info | 0 | 0 | 15 |
| root `src/` | **0** | 0 | 36 |

Effectively untested: `nova_modding` (439 LOC, 1 test), `nova_events` (821 LOC,
4 tests - and it just absorbed a 570-line vendored `engine.rs`), and the root
binary.

Policy (`AGENTS.md:70-95`) is harness-first. Reality is 1,638 unit tests against
24 probe examples - numerically lopsided, though the harness gate is real. The
code vendored in the last 10 commits landed with unit tests only and no new
example.

## Examples

`autoexamples = false` (`Cargo.toml:20`); the root catalog is the single source
of truth, pinned by `crates/nova_probe/tests/catalog_drift.rs`.

`sections/` 5, `ui/` 5, `stress/` 4, `systems/` 3, `screenshots/` 7 = 24 blocks.
26 `.rs` on disk minus 2 non-target modules
(`examples/screenshots/shared/kit.rs`, `examples/sections/turret_section/slider.rs`)
= 24. **No drift, no staleness detected.**

## CI - `.github/workflows/ci.yaml`

Nothing is `continue-on-error` except the artifact upload (`if: always()`,
correct). Steps:

- `cargo fmt --check`
- `cargo clippy --workspace --all-targets --features debug`
- `cargo test --workspace --features debug`
- `xvfb-run ... cargo run -p nova_probe ... run --all` (blocking; `aggregate_exit`
  returns FAILURE past OK/WARN)
- `xvfb-run ... cargo test -p nova_autopilot --test autopilot_example`
- separate `licenses` job: `cargo about generate`

### Three gaps - close before the large moves

**Amended 2026-08-07 - all three were measured rather than assumed. See
`09-clippy-and-lints.md`. Two were smaller than this note implied and one was
wrong.**

| Gap | Predicted here | Measured |
| --- | --- | --- |
| clippy without `-D warnings` | implied a cleanup pass would be needed first | **0 warnings** at the CI configuration. `-D warnings` is a one-line change that passes today. The gap is real; the cost is not |
| no default-features job | "branches never compiled" | CONFIRMED. **11 warnings**, every one dead code in `examples/` unreachable once `debug` is off. So `-D warnings` is free on the debug job but would fail a new default-features job on these 11. Fix: `#[cfg(feature = "debug")]` the 11 items first (~20 min) |
| no wasm job | ~~"wasm paths are probably rotten"~~ | **WRONG.** `cargo check --target wasm32-unknown-unknown` exits 0 across all 14 crates. 7 warnings, one cluster: all of `nova_probe/src/report.rs` is dead on wasm and wants a `cfg(not(target_arch = "wasm32"))` gate. Type-checking is not behavior - the paths are still untested - but the bit-rot prediction did not hold |

1. **Clippy runs without `-D warnings`.** Warnings never fail CI; only a compile
   error does.
2. **No wasm job.** `nova_assets/src/portal/*` and `persist.rs:45` wasm paths
   are compiled by nothing. Five `cfg_attr(not(wasm32), allow(dead_code))` sites
   exist for code CI never builds.
3. **No default-features job.** `cfg(not(feature = "debug"))` branches are never
   compiled. The workflow comment acknowledges this.

**Consequence: unused-import and dead-code fallout from a refactor lands
green.** If the benchmark baseline is taken against a tree CI does not check,
it measures the wrong thing.

## Suppressions

54 total: 37 `clippy::type_complexity`, 9 `too_many_arguments`, 2
`large_enum_variant`, 3 `dead_code`, 1 each `unused_variables` / `missing_docs`.

Per crate: nova_gameplay 38, nova_ui 4, nova_assets 4, nova_scenario 3,
nova_editor 2, examples 3, `src/main.rs` 1.

Broadest: `examples/screenshots/shared/kit.rs:26` `#![allow(dead_code)]` (the
only module-wide one); `nova_assets/src/portal/mod.rs:108`
`#[allow(missing_docs)]`; `nova_gameplay/src/hud/nova_os_ship/sections.rs:318`
`#[allow(dead_code)]`. The wasm-conditional ones (`persist.rs:45`,
`portal/config.rs:104,119,141,153`) are legitimate cfg noise.

No `todo!`, no `unimplemented!`, no `panic!("TODO")`.

**Seven `unreachable!()`**, all match-arm guards. **Corrected 2026-08-07 - this
was overstated.** Four of the five named are inside `#[cfg(test)] mod tests`:
`lint/ship.rs:443,769,772` sit past the `mod tests` opening at `ship.rs:314`,
and `lint/scenario.rs:712` past `scenario.rs:529`. They are assertion helpers
destructuring a config the test literal just built. No mod or scenario input
reaches them.

The one that IS production code is **`nova_gameplay/src/mesh/slice.rs:67`** -
the file's `#[cfg(test)]` does not open until `:82`. That single site keeps the
original concern: a refactor of the matched enum converts a compile-time
exhaustiveness check into a runtime panic.

## Recent churn - `HEAD~40..HEAD`

229 files, +14,161 / -1,573. Dominated by one task, `20260806-180450` "vendor
bevy-common-systems", commits `5afac831`..`4f18e571`, with `4f18e571` deleting
the dependency. It included one rewind (`cb56bcaf`, "steps 7-10 are
undelivered").

| Area | Change |
| --- | --- |
| `crates/nova_gameplay` | +6,487 / -765 across **103 files** |
| `tasks/` | +5,002 (docs, no risk) |
| `nova_ui` | +808 / 7 files |
| `nova_events` | +590 / 3 files |
| `nova_debug` | +415 / 6 files |

Largest new files: `nova_gameplay/src/camera/shake.rs` (+579),
`physics/pd_controller.rs` (+572), `nova_events/src/engine.rs` (+570),
`mesh/builder.rs` (+506), `integrity/core.rs` (+450), `nova_ui/src/tween.rs`
(+421), `nova_debug/src/inspector.rs` (+321).

**Vendored with ZERO tests:** `nova_ui/src/status_bar.rs` (365 new lines),
`nova_gameplay/src/camera/chase.rs` (242),
`nova_gameplay/src/camera/wasd_controller.rs` (233). Thin coverage:
`mesh/builder.rs` 3 tests, `mesh/explode.rs` 3, `objectives.rs` 1,
`nova_events/src/engine.rs` 4 tests for 570 lines.

## Risk register

Ranked by likelihood of a silent break.

1. **`nova_gameplay` as a god crate.** 77,761 LOC, 169 files, 23 top-level
   modules, 103 of them changed in the last 40 commits. `hud/` alone is 33,756.
   Any move touches everything, and 38 of 54 lint suppressions hide signal here.
2. **The freshly vendored bevy-common-systems code.** 10 commits, one review
   round, one rewind. `nova_ui/src/status_bar.rs`, `camera/chase.rs`,
   `camera/wasd_controller.rs` have **zero** tests - nothing pins their behavior
   if they move. They sit inside the seams being cut.
3. **`nova_events/src/engine.rs`.** 570 vendored lines, 4 tests, and it is the
   mandated scenario dispatch path. A regression is silent everywhere.
4. **`nova_gameplay/src/mesh/slice.rs:67`** - the one production `unreachable!()`.
   Downgraded from the original "seven match guards": the four in
   `nova_scenario/src/lint/` are test-only and carry no runtime risk.
5. **wasm-only and default-features paths**, compiled by no CI job. Combined
   with clippy lacking `-D warnings`, refactor fallout lands green.

## Risk register, re-ranked 2026-08-07 against what the review actually found

The register above was written **before** the code review. The original list is
kept intact; this is the same exercise re-run with evidence. Two entries move
down because the review cleared them, one entirely new entry goes to the top,
and one crate that was not on the list at all now belongs on it.

| # | Risk | Change | Why |
| --- | --- | --- | --- |
| **1** | **`nova_probe` is a blind CI gate.** Four defects, **three failing OPEN** (`run_report/artifacts.rs:44` all-or-nothing load; `artifacts.rs:65` excludes `web-run.log`, so a panicking wasm app verdicts OK; `native/run.rs:29` `RUN_ARTIFACTS` misses the `run-<n>.log` glob; `native/sweep.rs:187` lets an errored row inherit a stale OK verdict). Plus `recorder.rs:126` vs `nova_autopilot/src/completion.rs:152`, an unordered same-schedule `AppExit` write/read that is a latent CI flake | **NEW - straight to #1** | Every other lane in this epic is verified by this gate. A green sweep after a large refactor currently means less than it appears to. `15-review-probe.md` |
| 2 | `nova_gameplay` as a god crate | unchanged | 77,761 LOC, 103 of 169 files changed in 40 commits, 38 of 54 suppressions. Now also carries ~11 confirmed defects across all four seams - see `03-nova-gameplay.md` |
| 3 | The freshly vendored bevy-common-systems code | **partly confirmed, partly inverted** | `status_bar.rs` / `camera/chase.rs` / `camera/wasd_controller.rs` re-counted at **0 tests each** - confirmed, and `status_bar.rs` is where the reviewer found **three** defects (`:196`, `:238`, `:118`), so the aim was good. But `tween.rs` has **11** tests and the real problem there is the opposite: it is well-tested code with **zero consumers**. See `12-review-ui-layer.md` |
| **4** | **`nova_editor`** - 5 defects in 2,378 LOC against 13 tests, the worst defect density in the tree, including five `unwrap`/`panic!` sites in `placement.rs` reachable from mod content | **NEW - was not on the register at all** | `12-review-ui-layer.md`. It also has no in-workspace dependents, so nothing else pins it |
| 5 | `nova_events/src/engine.rs` | **confirmed, and the defect found** | 570 vendored lines, 4 tests, the mandated dispatch path. `engine.rs:170` maps a serialization failure to `data: None`, which `nova_scenario/src/filters.rs:71` then reads as "does not match" - so every entity-filtered handler for that kind stops firing and the scenario silently never advances. One added float field away from live. The register was aimed correctly |
| 6 | `nova_gameplay/src/mesh/slice.rs:67` - the one production `unreachable!()` | unchanged | Still the only one. The four in `nova_scenario/src/lint/` are test-only (see the correction above) |
| 7 | wasm-only and default-features paths | **downgraded** | Both measured. wasm type-checks clean; default-features produces 11 warnings, all in examples. The "silently rotted" fear did not hold. The residual risk is that neither is *tested*, which is real but smaller than stated |

**What the review cleared, and is worth not re-deriving:**

- **No reachable `unwrap`/`expect`/indexing panic in non-test code** anywhere in
  the audited scope - **four independent confirmations** (HUD/input, flight/
  sections, cross-cutting sweep, UI layer). The exceptions are named
  individually above; there is no *class* problem.
- The simulation core - flight guidance, the QP throttle balancer, the PD
  controller, gravity, integrity - was audited deeply and came back clean.
  `balance_throttles` always returns `engines.len()` entries, so the
  `throttles[i]` indexing in `autopilot.rs:884` and `manual.rs:149` cannot
  panic.
- Byte-vs-char UTF-8 arithmetic in the `nova_os` terminal: **no reachable
  panic**, three independent confirmations.
- Overlay precedence and event dispatch do not depend on `HashMap` iteration
  order. (Two *generated-content* paths do - see `05-assets-scenario.md`.)

## Running the suite (for reference - do not run locally)

CI-equivalent headless form, per `AGENTS.md`:

```sh
env -u DISPLAY -u WAYLAND_DISPLAY cargo test --workspace --features debug
```

The windowed half is `cargo run -p nova_probe -- run --all`. Never raise `-j`
past the `.cargo/config.toml` cap - concurrent rust-lld links OOM the box. All
cargo commands go through `nix develop --command`.
