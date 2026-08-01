# KISS: nova_assets - design record

## Structure

Three files held more than one concern. Split by cohesion; public paths and
prelude exports are unchanged.

### `lib.rs` 2683 -> 84

| New module | Concern |
| --- | --- |
| `collections.rs` | `BootAssets`/`GameAssets` + the Processing systems that publish their handles |
| `plugin.rs` | `GameAssetsStates` + `GameAssetsPlugin` wiring |
| `mod_set.rs` | shipped catalog + downloaded cache -> `ModCatalog` / `EnabledMods` |
| `merge.rs` | `register_bundles`, `merge_bundles`, the overlay rules |
| `scenario_generation.rs` | the RON generation surface (was an inline `pub mod`) |
| `lint_walk.rs` | the content-lint tree walk (was an inline `pub mod`) |

`lib.rs` now holds only the crate doc, the `mod` declarations, the prelude and
the re-exports that keep every public path identical. The four
`collections.rs` systems the plugin schedules became `pub(crate)`; nothing else
changed visibility.

### `portal.rs` 1773 -> a folder module

| File | Concern |
| --- | --- |
| `portal/mod.rs` | the channel bridge, `poll_portal_messages`, `PortalPlugin`, re-exports |
| `portal/config.rs` | `PortalConfig` + the pure URL-derivation helpers |
| `portal/transport.rs` | the `PortalTransport` seam and `EhttpTransport` |
| `portal/catalog.rs` | catalog fetch, schema gate, last-good store |
| `portal/install.rs` | validate/stage/commit, uninstall, stalled-fetch recovery |

Cross-module helpers became `pub(super)`; the `pub` surface `nova_menu` and the
integration rigs bind to is byte-identical (`portal/mod.rs` re-exports it).

### `scenario/shakedown.rs` 2843 -> a folder module

Production script (1221 lines, one concern) in `shakedown/mod.rs`; the 1600
lines of tests split into `shakedown/tests/pins.rs` (static cross-checks over
the built config) and `shakedown/tests/walk.rs` (the scripted-`App` beat walk),
with the two shared helpers in `shakedown/tests/mod.rs`.

### Result

No file in the crate exceeds 1500 lines. Largest remaining:

```
1221 src/scenario/shakedown/mod.rs   one concern: the shakedown script
1178 src/mod_cache.rs                one concern: the local cache + mods:// source
 988 src/scenario/shakedown/tests/walk.rs
 954 src/mod_refs.rs
 897 src/balance.rs
```

Both 1000+ files are single-concern and were left whole per the epic's rubric
("a 900-line file with one concern stays").

## Comments

275 HUID-bearing comment lines went to 6, every one a deliberate marker:

| Marker | Sites |
| --- | --- |
| `TODO(20260715-220011)` | `scenario.rs:424`, `broadside.rs:589`, `broadside.rs:828`, `lifeline.rs:797`, `final_tally.rs:720`, `shakedown/mod.rs:1211` - the five scenarios' placeholder banner art, plus the asteroid-field sandbox's. These were prose ("real art is task 20260715-220011"), i.e. deferred work; the rubric promotes those to `TODO`. |
| `TODO(20260525-133028)` | `collections.rs:236` - `update_nova_hud_assets` wants a refactor. Pre-existing, already correctly formed. |

Also stripped: the short-form provenance the DoD grep does not cover
(`task 163508`, `review 142906 R1.1`, bare `R1.4`), same rule - the clause goes,
the constraint stays.

The `//! ... ```text` fenced CLI usage block in `bin/content.rs`, the numbered
lists in the ledger test docs and the `balance_acks.ron` header were edited by
hand so the fences and list indentation survived.

Narration deletions were deliberately narrow: three `// OnStart: Create the
...` labels in `scenario.rs` that restated the `EventConfig::OnStart` literal
directly below. The rest of this crate's non-doc comments are rationale - why a
constant has its value, which Bevy ordering a system depends on, what a wire
gate is defending against - which both the epic rubric and the repo's global
guidance say to keep.

## Defect found, not fixed

`scenario::shakedown::tests::walk::an_early_derelict_kill_skips_to_the_fight`
fails on the "delivery guard: the rehearsal was mid-lesson" assert. Verified
PRE-EXISTING on master at `e038c34e` (same failure, pre-split path
`scenario::shakedown::tests::...`). Filed as **20260801-122138**; the other 95
lib tests and all 24 integration binaries pass.

## Verification

| Proof | Result |
| --- | --- |
| `cargo check --workspace --all-targets` | clean |
| `cargo fmt --check` | clean |
| `grep -rnE '//.*[0-9]{8}-[0-9]{6}' crates/nova_assets/` | 6 hits, all `TODO(...)`, tabled above |
| `wc -l` over `crates/nova_assets/` | max 1221, no exception needed |
| `cargo test -p nova_assets --tests --no-fail-fast` | 24/24 integration binaries green; lib 95/96 (the pre-existing failure above) |
