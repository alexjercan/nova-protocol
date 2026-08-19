# Audit: "dumb things"

Read-only sweep for the class `crates/nova_probe_cli/src/evaluation/budgets.rs`
belongs to. Three faults hunted:

1. Layering inversion - production code that knows about downstream artifacts.
2. A machine asserting a human's judgement - a gate on an environmental number.
3. Baked magic that should be data or absent.

Every finding below was read in the file. Line numbers are from
`smell-audit` at `7ad6ff3a`.

Verdict up front: **the class is not isolated, but it is CONCENTRATED.** The
timing-gate fault (2) has one sibling that is worse than the known offender.
The layering fault (1) has one clear sibling, in the same crate family. The
magic-value fault (3) is the systemic one: it is spread across the shipped
game and one instance can brick the boot.

---

## Findings

### 1. `sandbox_soak` asserts wall-clock physics cost, in the pass CI gates on

`examples/systems/sandbox_soak.rs:244` and `:256`, thresholds at `:227` and
`:233`, wired as a script beat at `:150-153` and `:169-172`.

```rust
const FIXED_TIMESTEP_MS: f64 = 1000.0 / 64.0;   // 15.625
const IDLE_CONTACT_BUDGET_MS: f64 = 5.0;
...
assert!(contacts_ms < IDLE_CONTACT_BUDGET_MS, ...);
assert!(step_ms < FIXED_TIMESTEP_MS, ...);
```

Both numbers are read from avian's smoothed wall-clock diagnostics
(`avian/total_step_time`, `avian/collision/update_contacts`, via `avian_ms` at
`:204`). **Fault 2**, and this one is worse than `budgets.rs` on all three
axes that matter:

- **`budgets.rs` does not gate CI; this does.** CI runs
  `probe run --all --correctness-only` (`.github/workflows/ci.yaml:120-123`),
  which skips the fps pass and therefore skips `frame_within_budget`.
  `sandbox_soak` is a cataloged `systems/` example, so
  `the_range_is_still_affordable` runs in the correctness pass, on a shared
  2-core GitHub runner, under Xvfb + lavapipe software rendering.
- **`budgets.rs` declines to grade a row from another host/profile/backend**
  (`FrameBudget::profile`/`backend`, the `gradeable()` filter). These two
  asserts have no guard at all. `FIXED_TIMESTEP_MS` in particular has zero
  margin: it claims a real avian step under lavapipe fits in 15.625 ms.
- **It is dressed as correctness.** Both asserts carry `probe_marker`
  `outcome:` slugs (`:251`, `:262`) which are on the roster in
  `crates/nova_probe_cli/tests/catalog_drift.rs:269-275`. Deleting the assert
  fails a second test. The false precision is locked in by design.

`IDLE_CONTACT_BUDGET_MS` is the defensible half: its doc (`:229-232`) argues a
100x margin makes the claim about the SHAPE of the work, not the speed of the
box. `FIXED_TIMESTEP_MS` makes no such argument and has no such margin.

Concretely: a busy runner panics the range, `process_exit` and `log_clean` both
fail, CI goes red, and the diagnosis is a physics regression that did not
happen. That is the phantom-bug mechanism, with a shorter fuse than
`budgets.rs` had.

Fix: measure and report both (the file already has a reporting path -
`sample_the_soak` at `:279-321` logs exactly these numbers). Drop the two
asserts and their roster slugs, or replace them with a claim about work SHAPE
that is not a clock reading (contact-pair count, constraint count - both
already sampled at `:239-240`). A function plus two roster lines.

### 2. Seven base-mod art paths hardcoded in an `AssetCollection`, no failure state

`crates/nova_assets/src/collections.rs:88-108`:

```rust
#[asset(path = "base/textures/cubemap.png")]     // :89
#[asset(path = "base/textures/asteroid.png")]    // :92
#[asset(path = "base/gltf/hull-01.glb#Scene0")]  // :95
#[asset(path = "base/gltf/turret-yaw-01.glb#Scene0")]    // :98
#[asset(path = "base/gltf/turret-pitch-01.glb#Scene0")]  // :101
#[asset(path = "base/gltf/turret-barrel-01.glb#Scene0")] // :104
#[asset(path = "base/gltf/torpedo-bay-01.glb#Scene0")]   // :107
```

Every one of these is also declared as data in
`assets/base/base.bundle.ron:30-37`. Two declarations, no cross-check.
**Fault 3**, and the "config duplicated in two places where drift is silent"
category.

The damage is the loading state, not the duplication:
`crates/nova_assets/src/plugin.rs:118-123` builds

```rust
LoadingState::new(GameAssetsStates::Loading)
    .continue_to_state(GameAssetsStates::Processing)
    .load_collection::<GameAssets>()
```

with **no `on_failure_continue_to_state`** - the string does not appear
anywhere in `crates/`. Rename or move one base art file and the collection
never resolves, `Loading` never advances, and the shipped game **hangs on the
loading screen forever**: no panic, no menu, one asset-server error line. This
is the highest-damage single item in the audit and it is in the release build.

Fix: add a failure state (one builder call) so a missing asset is a visible
error rather than a hang. Making the list itself derive from the bundle is a
design decision, not required to remove the brick.

### 3. `--features debug` changes the combat model, and every measurement is taken under it

`crates/nova_scenario/src/objects/spaceship.rs:404-419`:

```rust
let flagged = matches!(controller_config,
    SpaceshipController::Player(config) if config.infinite_ammo);
#[cfg(feature = "debug")]
let infinite_ammo = flagged;
#[cfg(not(feature = "debug"))]
let infinite_ammo = { if flagged { warn!(...); } false };
```

Under `debug`, a flagged player ship is built with no `SectionAmmo` at all
(comment at `:395-403`), so nothing gates on a reload. The feature is
documented exactly this way in `crates/nova_scenario/Cargo.toml:46-50`, and
both branches are tested (`:775-787`). It is deliberate and well commented.

The problem is what consumes it. Every example builds `--features debug`; the
probe builds its children `--features debug`
(`crates/nova_probe_cli/Cargo.toml:17-19`); CI's Tests step is `--features
debug`. So **every frame-time capture, every stress range and every probe
verdict is taken against a combat model with no ammo pressure and no reloads**,
and the release binary plays a different one. Against this release's own
standing rule ("the WORST frame ... a performance claim without a before and an
after is not a result", `tasks/20260818-220812/TASK.md`), that is a measurement
validity hole, not a cheat.

This is the only place in the workspace where a build feature changes
simulation - `cfg!(test)` and `#[cfg(not(test))]` are zero occurrences
repo-wide, and `debug_assertions` appears twice, both metadata-only.

Fix: a design decision. Either scope the cheat to a runtime switch the harness
does not set, or record on every capture that it was taken with the cheat on.

### 4. Example-only fixtures live in a library crate, on a justification the repo contradicts

`crates/nova_probe/src/fixtures.rs:1-18`. Module doc:

> They live in `nova_probe` rather than under `examples/` because
> `tests/catalog_drift.rs::catalog_matches_disk` scans every `.rs` directly
> under `examples/*/` and demands a catalog block for each, so a shared
> `examples/support/` module could only exist by hiding from that scan.

**Fault 1**, and the claim is false three ways:

- The named test exempts deeper files explicitly.
  `crates/nova_probe_cli/tests/catalog_drift.rs:36-38`: "Deeper files (e.g.
  systems/turret_gunnery/slider.rs, screenshots/shared/) are modules of a
  sibling root". The scan only reads `.rs` directly under a category dir
  (`:47-52`).
- Two sibling directories already do exactly this and are green:
  `examples/screenshots/shared/kit.rs` (included by 5 examples via
  `#[path = "shared/kit.rs"] mod kit;`, e.g.
  `examples/screenshots/screenshot_scene.rs:45-46`) and
  `examples/playable/shared/{wfc,compare}.rs`.
- All nine consumers are in ONE category
  (`examples/systems/{player_path,scenario_grammar,stress_bullets,
  stress_many_structures,borrowed_battery,stress_torpedoes,neutralized_quiet,
  stress_one_structure,torpedo_launch}.rs`), so `examples/systems/shared/`
  would serve them exactly as `screenshots/shared/` serves its category.

The doc also names a `stress/` category that does not exist on disk. Two of the
fixture's asset ids are baked strings (`:108-109`,
`"base/sounds/impact.wav"` / `"base/sounds/explosion.wav"`).

Damage is bounded: `nova_probe` is a dev-dependency of the root package only,
so nothing ships. But it is the same shape as the known offender, one crate
over: library code whose contents are decided by `examples/`.

Fix: move the module to `examples/systems/shared/fixtures.rs` and include it
with `#[path]` like the other two categories. A file move plus nine import
lines.

### 5. The NOVA OS map re-types `"asteroid"`, and a rename empties the map silently

`crates/nova_os_ui/src/map/contacts.rs:259` and `:366`, both:

```rust
if type_name.0 != "asteroid" { continue; }
```

The source of truth is
`pub const ASTEROID_TYPE_NAME: &str = "asteroid";` at
`crates/nova_scenario/src/objects/asteroid.rs:34`, applied at `:185` and
correctly used by `nova_authoring`
(`base_content/scenarios/sandbox/asteroid_field.rs:318`).

`nova_os_ui` has no `nova_scenario` dependency
(`crates/nova_os_ui/Cargo.toml` lists gameplay/ship/hud/events/os/ui), so it
CANNOT import the const - the literal is forced by the crate graph. **Fault 3**
plus CONVENTIONS Nova rule 5: the string is a de-facto contract between two
crates with nowhere to live.

Rename the const and every asteroid vanishes from the NOVA OS map, from both
the code-mint pass and the render pass, with **no warning at all** - it is a
bare `continue`. The tests re-type the same literal
(`crates/nova_os_ui/src/map/tests.rs:93,170`), so they stay green while the map
goes blank.

Fix: move the type-name consts to `nova_events`, which both crates already
depend on. One const move plus imports.

### 6. A `cargo test` unit test that fails if one frame takes over 50 ms

`crates/nova_core/src/loading_screen.rs:485-519`
(`the_scenario_screen_holds_then_comes_down`). It sleeps
`SCENARIO_MIN_DWELL` (0.6 s, `:61`), runs one long frame to pay the sleep, then
asserts the panel is gone after the NEXT `app.update()`. The production rule it
exercises (`:294`) only drops the panel when `time.delta_secs() <=
SCENARIO_SETTLED_DELTA` (0.05 s, `:66`).

**Fault 2.** On a loaded box no Bevy frame settles under 50 ms, the panel stays,
and the `assert_eq!(count, 0)` at `:514-518` fails. This runs in CI's Tests step
(`cargo test --workspace --features debug`,
`.github/workflows/ci.yaml:84`) with parallel test threads.

Fix: drive it on `TimeUpdateStrategy::ManualDuration` like the rest of the
workspace does (`nova_ship/src/input/targeting/gesture.rs:213-215` is the
pattern). A few lines in one test.

### 7. `nova_authoring` ships in the release binary and bakes the build machine's path

Root `Cargo.toml:332-333`:

```toml
[target.'cfg(not(target_arch = "wasm32"))'.dependencies]
nova_authoring = { path = "crates/nova_authoring" }
```

Not optional, not behind a feature - unlike `nova_probe_cli` on the next line,
which is `optional = true` and gated by `debug` (`Cargo.toml:463-468`).
`src/main.rs:36-40` declares `Command::Content` under
`#[cfg(not(target_arch = "wasm32"))]` only, and `:56` dispatches it. Release
builds are `cargo build --profile dist` with default features
(`.github/workflows/release.yaml:55,68,116,160`).

`docs/architecture.md` states nova_authoring is "The OFFLINE half of the
content pipeline (never shipped)". That is false for every native release.
**Fault 1**, doc-versus-reality.

It also carries `env!("CARGO_MANIFEST_DIR")` into the artifact:

- `crates/nova_authoring/src/cli.rs:105` - `run_gen()` joins it with
  `../../assets` and writes through `write_atomic`.
- `crates/nova_authoring/src/lint_walk.rs:76` - same for `workspace_root()`.

So the shipped binary contains the CI runner's absolute source path (`strip =
true` does not remove `env!` string literals), and `nova_protocol content gen`
on a player's machine panics on a path that does not exist. **Fault 3.**

Bonus: the doc on `lint_walk.rs:74` says "this crate sits at
`crates/nova_assets`". It sits at `crates/nova_authoring`.

Fix: mark the dependency `optional = true` and gate `Command::Content` on a
feature, matching `Probe`. Two lines plus a feature entry.

### 8. Shipped editor code names section prototype ids; a rename makes the buttons no-op

All in `nova_editor`, all production (its `#[cfg(test)]` blocks start at
`placement.rs:956` and `scenario.rs:712`):

- `crates/nova_editor/src/placement.rs:213` -
  `required_section(&sections, "reinforced_hull_section")` in
  `create_new_spaceship`.
- `crates/nova_editor/src/placement.rs:236` - `"basic_controller_section"` in
  `create_new_spaceship_with_controller`.
- `crates/nova_editor/src/scenario.rs:312-316` -
  `"reinforced_hull_section"` + `"light_hull_section"` x4 (the target hulks).
- `crates/nova_editor/src/scenario.rs:365,371,377,383,391` -
  `"basic_controller_section"`, `"reinforced_hull_section"` x2,
  `"basic_thruster_section"`, `"pdc_kinetic_turret_section"` (the pickets).
- `crates/nova_editor/src/scenario.rs:174,184` - `"base/textures/cubemap_alt.png"`,
  `"base/textures/cubemap.png"`; `:276-277`, `:477-478` -
  `"base/sounds/impact.wav"`, `"base/sounds/explosion.wav"`.

The ids are authored in `crates/nova_authoring/src/base_content/sections/`.
**Fault 3** plus Nova rule 5. Failure is soft and silent-ish: `required_section`
(`placement.rs:61-67`) warns and returns `None`, so **the New Ship button
becomes a click that does nothing**; the scenario path
(`crates/nova_scenario/src/objects/spaceship.rs:432-441`) logs `error!` and
`continue`s, so the sandbox spawns five invisible hulks and three pickets with
no controller, no thruster and no gun.

Note what is NOT the finding here: `nova_editor/src/scenario.rs` building a
whole scenario in Rust is fine - it is generated from the runtime
`PlayerSpaceshipConfig` (the ship you just built) and cannot be static data.
The baked ids are the issue.

Fix: a design decision (a named-role indirection in the section catalog) or, at
minimum, escalate the miss from `warn!` to a visible editor error.

### 9. Render layers and camera orders are a shared numeric namespace with no home

Four private consts, two crates, no allocation table:

- `crates/nova_os_ui/src/terminal/crt.rs:130` - `NOVA_OS_RTT_LAYER = 20`,
  `:133` - `NOVA_OS_RTT_CAMERA_ORDER = -20`
- `crates/nova_os_ui/src/map/mod.rs:55` - `MAP_LAYER = 21`, `:58` -
  `MAP_CAMERA_ORDER = -30`
- `crates/nova_os_ui/src/ship/mod.rs:68` - `SHIP_LAYER = 22`, `:71` -
  `SHIP_CAMERA_ORDER = -31`
- `crates/nova_menu/src/ambience.rs:26` - `MENU_UI_LAYER = 23`, plus
  `order: 100` at `:45`

**Fault 3.** Each doc restates its neighbours' numbers as prose:
`ambience.rs:26` asserts "nova_os_ui owns 20-22", `map/mod.rs:56-57` names 0 and
20, `ship/mod.rs:67` names 0 and 21. Nothing enforces any of it. Add a fourth
layer in `nova_os_ui` and it silently collides with the menu overlay; the
symptom is a rendering bug that only shows in a live run.

Fix: one allocation table in `nova_ui` (the leaf both crates already depend on)
that hands out layer and order. A module.

### 10. Harness env-var NAMES hand-copied into shipped crates, guarded by nothing executable

`crates/nova_gameplay/src/settings.rs:100-105`:

```rust
// NOTE: string literals, not `nova_autopilot`'s consts. `nova_gameplay`
// is a shipping crate and does not take a dev-tooling dependency for
// three strings; the migration task's absence grep guards the drift.
let harness_env_active = ["NOVA_AUTOPILOT", "NOVA_SHOT", "NOVA_CAPTURE"]
```

and `crates/nova_scenario/src/actions/view.rs:71` - `"NOVA_SHOT_DIR"`.

The canonical consts are `crates/nova_autopilot/src/autopilot.rs:77`,
`screenshot.rs:59`, `capture.rs:41`, `capture.rs:45`, and they have a contract
test - `crates/nova_autopilot/tests/env_contract.rs:9-12` pins all four values.
**The copies are not on it.** The stated guard ("the migration task's absence
grep") is a one-time grep in a closed task, not a standing check.

Rename `SCREENSHOT_ENV` and the game silently stops muting itself during
captures; rename `SHOT_DIR_ENV` and authored screenshot actions start writing
to the process CWD. Silent-drift duplication, both directions.

Fix: extend `env_contract.rs` to assert the shipped literals too, or move the
four names into a dependency-free crate both sides can name. A test, or a
module.

### 11. `"base"` as a magic mod id across five production files

Declared as data at `assets/mods.catalog.ron` (`id: "base", base: true`),
re-typed at:

- `crates/nova_assets/src/merge.rs:180,182,190`
- `crates/nova_assets/src/mod_refs.rs:100,126`
- `crates/nova_assets/src/portal/install.rs:479,501`
- `crates/nova_menu/src/mods.rs:828` (and `:179`, rendered as badge text)

**Fault 3.** Rename the catalog entry and it compiles clean, then fails four
different soft ways: no `dep://base` registration, every mod's `dep://base/...`
reported as an undeclared dependency, portal installs demanding base be
downloaded, and the menu auto-enabling base as a normal dep. Nothing panics.

Fix: one `pub const BASE_MOD_ID` in `nova_mod_format` (which all four crates
already reach). A line plus four imports.

### 12. A capture-tooling env var in a shipped crate selects which scenario spawns

`crates/nova_menu/src/ambience.rs:126` - `NOVA_MENU_BACKDROP` pins the menu
backdrop pick to one id, ending in `commands.trigger(LoadScenario(pick))` at
`:143`. Registered unconditionally at `crates/nova_menu/src/lib.rs:129`: no
`#[cfg]`, no `debug` gate. Its own comment calls it a "Dev/capture override".

**Fault 1**, low damage (an unknown id warns and falls back to the draw, `:135`,
so it cannot brick the menu). Listed because it is the shipped game reading a
capture-harness switch to decide what to spawn. Fix: one `#[cfg(feature =
"debug")]`.

Same shape, lower still: `crates/nova_scenario/src/actions/view.rs:96-135` -
`ScreenshotActionConfig` is an ungated shipped scenario action that
`create_dir_all`s and writes PNGs, with the directory from `NOVA_SHOT_DIR`. Its
own doc (`:92`) calls it "a dev/marketing tool". Authored content in a release
build can write files.

### 13. Smaller, confirmed

- `crates/nova_os_ui/src/ship/sections.rs:363-367` - `pub(crate) fn resolve`
  with a bare `#[allow(dead_code)]` whose doc admits "the live CLI handler
  resolves without touching Health/Ammo ... so this convenience is used by the
  tests". Sole caller is `crates/nova_os_ui/src/ship/tests.rs:152`. Dead code
  kept alive by a test, plus CONVENTIONS Rust rule 1 (bare `#[allow]`, no
  reason).
- `crates/nova_os/src/terminal/state.rs:409` - `pub fn enter_app` has 15
  references, ALL in `nova_os_ui`'s test modules. A cross-crate `pub` API that
  exists only for another crate's tests, and the `pub` is what stops the
  dead-code lint from saying so.
- `crates/nova_menu/src/tests/scenarios.rs:362` - cites
  `cargo run --example menu_scenarios --features debug` as "the evidence" for
  the test. No such example exists in the 45-entry catalog and no such file is
  on disk. The evidence was deleted; the pointer was not.
- `crates/nova_probe_cli/src/native/fixtures.rs:10-19` - the stand-in catalog
  (`#[cfg(test)]`-gated, so no shipped leak) files real example `scene_baseline`
  under category `"stress"`, and invents categories `"sections"`, `"gameplay"`,
  `"stress"`. The real categories are `playable`, `systems`, `screenshots`. The
  spec-resolution tests at `src/native/spec.rs:130-148` therefore assert against
  a taxonomy the repo abandoned, while reading as if they validated the real
  one.
- `crates/nova_gameplay/src/lifetime.rs:105` - "a hard panic under the game's
  `FallbackErrorHandler(panic)`". The game installs no such handler; only
  examples do (`examples/systems/menu_boot.rs:63`,
  `examples/systems/menu_picker.rs:67`,
  `examples/screenshots/screenshot_ui.rs:119`). The code
  (`try_despawn`) is right anyway; the doc claims a shipped safety property that
  does not exist.
- `crates/nova_ship/src/sections/clearance.rs:29-31` - "ONE copy, two callers.
  The generator ([`examples/playable/wfc_ships.rs`])..." - a production module
  doc naming an example as one of its two callers, written as an intra-doc link
  that cannot resolve (an example path is not an item).
- `crates/nova_probe_cli/tests/catalog_drift.rs:356` -
  `const SYSTEMS_INVARIANTS: usize = 131;`, asserted at `:376` against the sum
  of the roster twenty lines above it. Derived from data in the same file; adds
  a hand edit per roster change and catches nothing the roster does not.

---

## Checked and clean

These came back empty or defensible. Do not spend time here.

**No production crate names an example in executable code.** All 45 catalog
names were grepped against every `crates/**/*.rs`. Every hit in a production
crate is a comment or doc comment (`nova_core/src/lib.rs:71`,
`nova_ui/src/widget/slider.rs:299`, `nova_ship/src/sections/skin_style.rs:46`,
`skin_decor.rs:452`, `nova_gameplay/src/integrity/explode.rs:265`, and ~10
others). The only live-code hardcodes are inside `nova_probe_cli`
(`budgets.rs`, already being deleted; `cli.rs:16,156` usage/error text). No
`crates/**/Cargo.toml` mentions an example. No workflow hardcodes an example
list - CI drives everything through the catalog. The doc citations of measured
example output (`skin_style.rs:46` "308 of 526 plates on the `wfc_ships` row")
are the good version of this: evidence for a value, no code dependency.

**No default/fallback scenario id in production.** New Game resolves entirely
through data: `assets/base/base.bundle.ron`'s `new_game_scenario`, then an
id-free fallback chain in `crates/nova_menu/src/menu_ui.rs:424-478`. The menu
backdrop selects by the `menu_backdrop` flag, never by id
(`crates/nova_menu/src/ambience.rs:67-110`) - its module doc claims "The menu
names no scenario ids" and that is true.

**No ship prototype id and no style id is hardcoded anywhere in production.**
Style resolution is always `GameStyles::get_style(id)` from data; the editor
picks by index. Every `"cargoa"` hit is a test fixture.

**`nova_ship`, `nova_hud`, `nova_core`, `nova_ui`, `nova_info`, `nova_events`,
`nova_modding`, `nova_mod_format`, `nova_os`** carry zero production content
ids.

**`test-support` is never enabled through a normal `[dependencies]` edge.** All
four consumers (`nova_scenario:32`, `nova_assets:68`, `nova_authoring:44`,
`nova_ship:30`) declare it under `[dev-dependencies]`. Both feature definitions
are non-default. Workspace `resolver = "2"`, so a plain `cargo build` of the
root binary does not unify it in.

**No `#[cfg(test)]` item is reachable from non-test code**, and every in-`src`
`mod test_support` is correctly gated. `cfg!(test)` and `#[cfg(not(test))]`:
**zero occurrences repo-wide.** `debug_assertions`: two hits, both in
`nova_probe`, both metadata-only.

**No `#[expect(dead_code)]` or `#[expect(unused)]` anywhere.** All 35
`#[expect(...)]` are clippy lints with reason strings. `dead_code`,
`unused_variables` and `unused_imports` are commented OUT in
`[workspace.lints.rust]` (`Cargo.toml:477-484`), i.e. left at their deny/warn
defaults rather than globally allowed. Of the eight `#[allow(dead_code)]` sites,
six are legitimate cfg-conditional (wasm-only portal helpers, a serde
`skip_serializing_if` helper) and two are in test files. Only
`nova_os_ui/src/ship/sections.rs:366` is a real one (finding 13).

**Production crates do not know about the harness.** Nothing in `crates/**` or
`src/**` reads `NOVA_AUTOPILOT` to change behaviour except the audio mute in
finding 10; nothing installs `FallbackErrorHandler(panic)` - the examples do
that themselves. Every `autopilot` hit in `nova_ship`/`nova_hud` is the in-game
flight autopilot, a game feature.

**The documented crate graph matches reality.** Every `Cargo.toml` was parsed
and compared against `docs/architecture.md`. `nova_ui` is a true leaf. `nova_os`
has no UI dep. `nova_mod_format` is engine-free. `nova_gameplay` depends only on
`nova_events` + `nova_ui`. `nova_scenario -> nova_ship`/`nova_hud` is as
documented. The `AppBuilder` plugin order at
`crates/nova_core/src/lib.rs:188-225` is exactly the order the doc states. One
harmless inaccuracy: the doc says `nova_probe_cli` depends on `nova_assets`; it
is a dev-dependency (`crates/nova_probe_cli/Cargo.toml:35-37`), so reality is
cleaner than the map.

**Seeded RNG discipline holds where it matters.** `crates/nova_gameplay/src/integrity/explode.rs:290`
draws from `Single<&mut WyRand, With<GlobalRng>>`; scenario scatter uses
`StdRng::seed_from_u64` (`crates/nova_scenario/src/actions/spawn.rs:327,333`).
Three `rand::rng()` / `rand::random()` sites exist and all three are
presentation, not gameplay: camera shake offsets
(`crates/nova_gameplay/src/shake.rs:283`), a decorative orbit
(`transform/random_sphere_orbit.rs:107`) and muzzle-flash particle colour
(`crates/nova_ship/src/sections/turret_section/render.rs:65-83`).

**Web-doc number citations are holding.** CONVENTIONS Web rule 2 requires every
game number on a doc page to carry the `file:line` it was verified against.
Four spot checks all landed exactly: `nova_os/src/terminal/edit.rs:23`
(MAX_HISTORY 200), `nova_os/src/terminal/state.rs:16` (MAX_SCROLLBACK_ROWS 500),
`nova_ui/src/units.rs:13` (METRES_PER_UNIT), `nova_gameplay/src/relations.rs:53-61`.

**Constant duplication across crates is mostly false alarm.** Every
`const NAME` defined in two or more files was compared. The HUD ones
(`ARROW_PX`, `STROKE_LEN_PX`, `DIAMOND_PX`, ...) are per-widget styling that
legitimately differs. `BEACON_LOCK_SIGNATURE` differing between
`nova_scenario/src/objects/beacon.rs:28` (20.0, the engine default) and
`nova_editor/src/scenario.rs:192` (30.0) is an authored per-scenario override,
not drift. `IMPACT_MIN_INTERVAL = 0.04` in both
`nova_gameplay/src/juice.rs:58` and `nova_ship/src/ship_audio/mod.rs:83` is two
independent throttles for two subsystems that happen to agree; the docs say
they mirror each other and nothing enforces it, but the coupling is cosmetic.

**Doc-versus-constant drift (CONVENTIONS Comments rule 5) found nothing
material.** Every `const` whose doc contains a number was machine-compared
against its value; ~120 candidates, all inspected samples turned out to be
derivations rather than restatements. `lock_crosshairs.rs:45` cites
`MIN_RETICLE_PX` 32 in prose and `torpedo_target.rs:26` really is 32.

**Frame-count budgets are the right pattern and are used widely.**
`SETTLE_FRAMES = 30` (`nova_debug/src/harness.rs:128`), `MAX_WAIT_FRAMES = 1800`
(`nova_autopilot/src/screenshot.rs:64`), `LOOP_FRAME_CAP = 600` with a pinned
clock (`nova_autopilot/src/loops.rs:75,215`), `ROW_SETTLE_FRAMES`,
`SETTINGS_SAVE_DEBOUNCE_FRAMES`. These get MORE forgiving on a slow host. Not
findings.

**Colour and layout asserts are identity checks, not aesthetic ones.** ~20 test
files assert on `Color::` values; every sampled one asks "does this widget use
theme colour X", which is logical.

---

## Timing gates: the honest picture

Separating this out because the raw count looks alarming and most of it is
fine.

There are roughly **200 wall-clock deadline sites** in the repo, nearly all
`.deadline(N)` calls on autopilot steps (95 in `examples/systems/`, 68 in
`examples/screenshots/`, 29 in `examples/playable/`). Breach logs `error!` and
writes `AppExit::error()`
(`crates/nova_autopilot/src/autopilot.rs:473-489`), which fails both
`process_exit` and `log_clean`. Plus:

- `crates/nova_autopilot/src/completion.rs:95` - `DEFAULT_DEADLINE_SECS = 120.0`
- `crates/nova_probe_cli/src/native/cli.rs:92` - process timeout 180 s
- `crates/nova_debug/src/harness.rs:136` - `SHOT_DEADLINE_SECS = 20.0`
- nine integration tests with `assert!(Instant::now() < deadline)` at 60 s or
  120 s
- four CI `timeout-minutes` caps

**These are hang detectors, not performance gates, and I am not reporting them
as findings.** A step that never completes has to fail somehow. Two of them
carry comments saying exactly that
(`crates/nova_scenario/tests/skybox_swap_e2e.rs:63`,
`crates/nova_core/tests/cubemap_meta_app_config.rs:54`).

Two things about them are worth knowing:

- **The CI margin is 10 seconds.** `.github/workflows/ci.yaml:111` sets
  `NOVA_AUTOPILOT_DEADLINE: 170` against a 180 s process kill
  (`crates/nova_probe_cli/src/native/cli.rs:92`, not overridden in CI), and the
  comment at `:108-110` says the longest screenshot walk already exceeds the
  120 s interactive default under software rendering. That is a ~6% band on a
  shared runner. Not a design fault; worth widening.
- **`fps_within_baseline` gets this right and is the model.**
  `crates/nova_probe_cli/src/evaluation/checks/fps_within_baseline.rs:15,111` -
  a 10% mean-frame-time regression is `CheckStatus::Warn`, never `Fail`, and its
  doc says why ("frame numbers on a shared host are noisy"). Its own test pins
  `overall_verdict == "WARN"` (`:250`). Whatever replaces `budgets.rs` should
  look like this.

Everything else environmental reports rather than asserts:
`gravity.rs:871-912` is `#[ignore]`d with `eprintln!` only;
`sandbox_soak.rs:279-321` and `ship_editor.rs:125-147` log; the `Instant`-timed
paths in `asteroid_carve.rs`, `editor/skin.rs` and `sweep.rs` log.

---

## What I could not check

- **Nothing was built or run.** No `cargo check`, no `cargo test`, no probe run
  (owner instruction: read-only; workspace test is also an OOM risk here). So
  every claim is a source claim. The two I would most want confirmed by a run
  are finding 1 (does `sandbox_soak` actually go red on a loaded box, or does
  avian's smoothing save it?) and finding 2 (does the loading state really hang
  rather than time out somewhere upstream?).
- **`bevy_asset_loader` failure semantics were read from the call site, not
  from the dependency's source.** I confirmed `on_failure_continue_to_state`
  appears nowhere in `crates/`; I did not read the crate to confirm the
  no-failure-state behaviour is "wait forever" rather than "continue anyway".
  That is the load-bearing assumption in finding 2's severity.
- **Runtime id coverage is grep-deep, not load-deep.** The id vocabulary was
  extracted from `assets/base/**/*.content.ron` and the `nova_authoring`
  builders (426 unique tokens) and grepped. An id constructed at runtime by
  concatenation would not have been caught.
- **`webmods/`, `assets/mods/` and `benchmark/`** were only spot-checked.
- **The web surface** (`web/src/**`) was sampled for the number-citation rule,
  not swept. `npm run ci` was not run.
