# Cross-cutting pattern sweep

Source: dedicated sweep agent, 2026-08-07. Method: grep the whole workspace
per pattern with `#[cfg(test)]` / `#[test]` blocks masked by brace-matching,
count, then read a sample at each site to classify. Corpus 412 `.rs` files,
~170k lines.

**Read this before proposing any lint rule.** Most of the patterns people
assume are problems in this codebase are not, and the numbers say so.

## The headline: the suspected patterns are clean

| Pattern | Raw hits | Sampled | Genuinely bad | Verdict |
| --- | --- | --- | --- | --- |
| `as` casts (truncating/wrapping) | 110 | 8 | **0** | Not a problem |
| Float equality | 14 | 6 | **0** | Not a problem |
| Division by possibly-zero | 18 | 8 | 1 | Not a problem |
| `unwrap`/`expect`/`panic!` non-test | 23 real | 12 | 3 | Not a class problem |
| Unbounded indexing/slicing | ~80 | 8 | 1 | Not a problem |
| Duplicate system registrations | 42 candidates | all | **0** | Not a problem |
| Dead `pub` items (never used anywhere) | - | 13 | **0** | No dead code |

This corroborates `07-comments-and-docs.md`: the codebase's hygiene is
consistently better than a first impression suggests. Two examples worth
keeping:

- **Every float-to-int cast read was clamped within 2 lines**, several with a
  comment naming the reason. `hud/readout.rs:65` has `value.max(0.0)` two lines
  above with an explanation. This matches my own independent sample in
  `09-clippy-and-lints.md`, which reached the same conclusion from the clippy
  side. Two methods, same answer.
- **All 14 float-equality sites are exact-zero *event* tests**, not computed
  comparisons - four are `!= 0.0` on a raw `MouseWheel` delta, which is exactly
  the right test. A blanket `clippy::float_cmp` rule would be 14 false
  positives.

Note the sweep's float-equality set does **not** include
`nova_scenario/src/variables.rs:270`, the `Equal` node of the scenario
condition DSL that clippy flagged. The sweep grepped for `== 0.0`-shaped
literals; `l == r` on two bound variables does not match. **That finding
still stands** - see `09-clippy-and-lints.md`. The two results are
complementary, not contradictory.

## The single highest-leverage item in the whole review

**Convert 36 `#[allow(clippy::type_complexity)]` to `#[expect(..., reason =
"...")]`.**

The codebase already uses `#[expect]` with a reason in 4 places -
`input/player/hints.rs:200`, `keybind_dock.rs:569,737,790` - so this enforces
an existing local convention rather than importing a new one. VERIFIED: 36
`allow` against 4 `expect`.

The payoff is that rustc's `unfulfilled_lint_expectations` then reports every
**stale** suppression on the next clippy run, at zero analysis cost. The sweep
already found two by eye (`hud/ammo_readout.rs:325` and `:510`, where the worst
type is `Query<(Entity, &ChildOf, &SectionAmmo), With<TorpedoSectionMarker>>` -
well under clippy's threshold). The compiler will find the rest for free.

This pairs directly with the `-D warnings` recommendation in
`09-clippy-and-lints.md`. Together they turn suppression rot from something
nobody audits into something CI reports.

Of the 9 `too_many_arguments` suppressions, **6 are genuinely refactorable**,
and the codebase already has the idiom: `#[derive(SystemParam)]`, used at
`nova_os_ship/sections.rs:224 ShipSections`. `nova_os_map/scene.rs:259
map_input` and `nova_os_ship/scene.rs:336 ship_input` take an *identical*
6-param input cluster (pause, terminal, keys, mouse_buttons, motion, wheel) -
one shared `SystemParam` struct removes two suppressions and the duplication.

## Real findings

### 1. Two order-dependent hash iterations feed generated content

`crates/nova_scenario/src/objects/binding_input.rs:83` -
`binding_map_serde::serialize` iterates a `HashMap<SectionId, Vec<Binding>>`
straight into serde output. **This is what writes `input_mapping:` into the
generated `assets/base/**/*.content.ron`.**

Stable today because bevy's collections use `FixedState`, and the four
scenario files do share one key order. But that order is a hashbrown/bevy
implementation detail. **A bevy bump reshuffles every generated scenario file
at once and `content_ron_parity` fails with a diff nobody authored.** A
`BTreeMap`, or `serialize_map` over a sorted key vec, makes it structural.

Given the memory `base-content-ron-is-generated` (never hand-edit those files;
edit the builders and run `content -- gen`), this is a direct threat to the
content pipeline's only integrity gate. Cheap insurance.

`crates/nova_assets/src/lint_walk.rs:380` is the same class - InputOverlap
findings are built by iterating `player.input_mapping` and pushed to `findings`
at `:532` unsorted (the file's only `sort` is `dirs.sort()` at `:127`). Lint
output comes out in hash order.

The rest of the codebase's determinism hygiene is strong: explicit `sort()`s,
name tiebreaks (`nova_probe/src/profile.rs:117` sorts by `total_ms` desc with a
`.then_with(|| a.name.cmp(&b.name))`), and `deps.rs`'s topo sort deliberately
re-scans the input `Vec` each round with a comment saying why.

### 2. One duplicate that is also the workspace's only real complexity bug

`crates/nova_gameplay/src/flight/autopilot.rs:877` and
`crates/nova_gameplay/src/flight/manual.rs:142` - the engine-spool loop is
**byte-identical for 16 lines**. Both are in `NovaFlightSystems`, both already
share `spool` and `BalanceEngine`.

Both copies also carry the same complexity bug: for every ship, walk **every
unbound thruster in the world**, and inside that loop run
`allocation.iter().position(|(e, _)| *e == thruster)` - a linear scan. That is
O(ships x thrusters x thrusters_on_this_ship), every FixedUpdate tick. VERIFIED
at `manual.rs:142`.

One extraction plus a `HashMap<Entity, usize>` or a `Children` walk kills the
duplicate and both copies of the perf bug together. **Best cost/benefit ratio
in the review.**

### 3. AI firing cadence is framerate-dependent

`crates/nova_gameplay/src/input/ai/mod.rs:107` registers the whole AI chain in
`Update` (VERIFIED), and `guns.rs:119`, `behavior.rs:292-308`, `torpedo.rs:158`
tick firing-gate `Timer`s off `time.delta_secs()` - while the firing itself
happens in `FixedUpdate`.

**AI DPS therefore varies with framerate.** This is the only pattern in the
sweep with a player-visible gameplay effect.

The fix is narrow: move the cadence systems to `FixedUpdate`, or tick them off
`Time<Fixed>`, and leave the rest of the AI chain where it is.

Important context: the 6-vs-119 `FixedUpdate`/`Update` ratio looks alarming and
is **not** a problem. Everything touching avian is already fixed-stepped -
`gravity.rs:241`, `physics/pd_controller.rs:359,361`, `sections/mod.rs:132`,
both `shoot_spawn_projectile`s, and the whole `NovaFlightSystems` set with an
explicit `.before(SpaceshipSectionSystems)`. `flight/manual.rs:80` even
comments "Raw-clock pose (avian Rotation) - this is FixedUpdate." The physics
core is correctly placed.

A secondary, lower-amplitude case: `ai/maneuver.rs:199` and
`player/intent.rs:73` compute `max_step = turn_rate * time.delta_secs()` then
write an attitude setpoint the `FixedUpdate` PD controller chases. The
per-second rate is right, but the setpoint the fixed tick samples differs by
framerate.

### 4. `ScenarioConfig::default()` is invalid by its own documentation

`crates/nova_scenario/src/loader/mod.rs:144`. The doc at `:141` says: "a FULLY
default `ScenarioConfig` is not serializable: its default `cubemap` is a
handle-backed `AssetRef`, which errors on serialize... do not serialize
`ScenarioConfig::default()` directly."

An invalid-state `Default`, kept only so 14 builder sites can skip
`thumbnail`/`hidden`/`menu_backdrop`, guarded by a comment rather than by the
type. A `ScenarioConfig::new(id, name, cubemap)` with defaulted optionals makes
the invalid state unrepresentable.

The same 14 sites (`scenario.rs:431,458`, `scenario/menu.rs:127,157,221,380,498`,
`lifeline.rs:215,243`, `final_tally.rs:242,283`, `shakedown/mod.rs:651,1400`,
`broadside.rs:202`) are the "new field silently appears in 14 generated
scenarios" hazard. `content_ron_parity` does catch the resulting diff, so this
is review noise rather than a correctness hole.

### 5. Smaller confirmed items

| Site | Issue |
| --- | --- |
| `nova_assets/src/portal/mod.rs:176` | `install.entry.files[index]` - the guard above is `if install.files.len() != index { continue }`, which does NOT bound `index` against `entry.files.len()`. A duplicated final-file callback passes the guard and panics. Network-driven index; wants `get(index)` |
| `nova_gameplay/src/camera/skybox.rs:118` | `images.get_mut(&config.cubemap).unwrap()` inside an `On<Insert, SkyboxConfig>` observer. The fn already `let Ok(..) else { error!; return }`s for the query one line above, then unwraps an asset that is not guaranteed loaded at insert time |
| `nova_info/build.rs:11-13` | `expect("failed to get git revision")` + `String::from_utf8(..).unwrap()` - breaks the build in a tarball export with no git |
| `nova_probe/src/run_report/html.rs:217` | `intervals.iter().sum::<f64>() / intervals.len() as f64` with no `is_empty` guard, unlike the identical line at `capture.rs:499`. Prints `NaN` into the report HTML |
| `nova_scenario/src/world.rs:64,85` | `objectives.clone()` and `story_messages.clone()` in `state_to_world_system`, which the module doc says runs every frame - deep-cloned **purely to release the borrow before the diff compare**, then discarded when nothing differs. Cheap in bytes; worth fixing as clarity since a scoped `let` removes them |
| `nova_gameplay/src/hud/readout.rs:207` | `format_readout` allocates two Strings per readout per frame (`to_uppercase()` + `format!`) **before** the `if existing.0 != text` compare that usually throws them away. The system's doc comment is explicitly proud of avoiding per-frame entity churn; the allocation just landed on the wrong side of the compare |
| `nova_assets/src/scenario/{broadside,final_tally,lifeline}.rs` | `fn player_ship()` is ~20 identical lines in three scenario builders, differing only in `infinite_ammo`. Generated-content input, so a divergence silently ships three different player ships |
| `nova_probe/src/run_report/checks/{reached_playing,run_completed,invariants_held}.rs:30-35` | The `capability.wiring()` / `Input::NotArmed` dispatch preamble repeats verbatim in three modules |
| `nova_debug/src/lib.rs:124`, `inspector.rs:180`, `wireframe.rs:66` | Three separate private `toggle_debug_mode` fns, all registered, all toggling the same `DebugEnabled` on the same F11 press. Works only because three flips of a bool is still a flip; `lib.rs:110` comments "they stay in phase". **A fourth sub-plugin silently breaks the key** |

## Over-broad visibility - real, but zero correctness impact

633 `pub` items are never referenced outside their own crate. nova_gameplay
358 (~55% of its public surface), nova_assets 88, nova_probe 61, nova_scenario
56, nova_ui 34.

**Truly unreferenced anywhere: 0.** The agent hand-checked 13 of 54 "suspected
dead" candidates and all 13 were used - either once inside the crate or via a
prelude re-export. **There is no dead code here**, only over-broad visibility.

Worst offenders are HUD-internal markers that could be `pub(crate)`:
`hud/item_highlights.rs:53`, `hud/flight_status.rs:57`, `hud/beacon_chips.rs:44`,
plus `objectives.rs:100 ObjectivesPluginSystems` (a system-set enum exposed for
external ordering that nothing orders against) and the whole
`content_report.rs` `Severity`/`Finding`/`ContentReport` surface.

`#![warn(unreachable_pub)]` does **not** catch this class - these are
reachable, just unused. It needs the identifier cross-reference or
`cargo-public-api` in CI.

Low priority. The payoff is a smaller API to keep stable and faster
incremental rebuilds, not correctness. But it is directly relevant to the
crate-splitting lanes: **splitting `nova_gameplay` four ways forces this
question anyway**, since each seam has to decide what crosses its boundary.

## The five worth fixing, in the sweep's own ranking

1. `#[allow]` -> `#[expect(reason)]` on 36 sites. Nearly free, makes
   suppression rot self-reporting.
2. Extract the shared engine-spool loop from `flight/{manual,autopilot}.rs`.
   One edit kills a 16-line duplicate and the only real per-tick complexity bug.
3. `BTreeMap` for `binding_input.rs:83`, `sort()` for `lint_walk.rs:380`.
   Protects the content pipeline's integrity gate from a bevy bump.
4. Tick AI cadence timers on the fixed clock. The only player-visible item.
5. Retire `ScenarioConfig::default()` for a constructor.

## Explicitly not worth fixing

`as` casts, float equality, division-by-zero, unwrap/expect/panic as a class,
duplicate system registrations, and the 633 crate-local `pub` items. Each was
measured and sampled; see the table at the top.
