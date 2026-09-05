# Nova Review of the unpushed v0.13.0 range

- STATUS: OPEN
- PRIORITY: 70
- TAGS: v0.13.0,review

## Goal

Run Nova Review over `origin/master..HEAD` (56 commits, ~52k reviewable lines
after generated RON, `Cargo.lock`, `tasks/`, and art binaries are excluded).
The range is too wide for one pass, so it is split into 14 logical batches that
run in sequence. This file is the shared ledger: the plan, then one findings
block per batch, then the adjudicated verdict and the fix pass.

Owner framing (2026-09-05): rock shading is reviewed SOLO; planetoids are
related but reviewed separately; no `--play`; at most two agents in flight at
any time; batches run in sequence; findings are recorded here between runs and
fixed at the end.

## Rules for this run

- Two agents per batch, dispatched in one message, nothing else concurrent.
  - Lane A: correctness + performance. Holds the measurement slot.
  - Lane B: craft + contracts. Never runs a rendered example or a probe.
- Reviewers are read-only. No edits, no staging, no commits.
- No workspace test suite, no workspace Clippy.
- `assets/base/**/*.content.ron` is generated. Review the Rust builders under
  `crates/nova_authoring/src/`, not the RON.
- HEAD is the truth. The per-commit diffs show intent; a later batch may have
  already replaced code an earlier commit added.

## Batch plan

Run order is engine-first inside the owner's constraints, so a finding in a
load-bearing batch is known before the content that sits on it is judged.

| # | Batch | Commits (oldest first) | Lines |
|-|-|-|-|
| 1 | Asteroid material shading (SOLO) | `b202d69f` | 2561 |
| 2 | Planetoid seeded planet types (SOLO) | `db72da2e` | 3714 |
| 3 | Flight model: RCS, overspeed, speed budget | `2fb42ef9` `7a960c5b` `e2b73a3f` `42343d27` | 1581 |
| 4 | Helm orders and the GOTO arrival model | `6ee44142` `b595472e` `356f3f68` `477d92aa` | 6117 |
| 5 | Scenario scripting engine and the orbit goal | `eb3ea6ed` `68431eb3` | 4441 |
| 6 | Destruction: debris budget, severed roots, magazine | `8e5043e1` `77f963b1` `c9a23872` | 1351 |
| 7 | Campaign replacement: First Shift and Second Shift | `32d00dfe` | 11545 |
| 8 | Campaign pacing and cinematic reliability | `735eea53` `f77db4cc` | 6166 |
| 9 | Campaign scenes and set pieces | `bb1cc37e` `02b8775e` `6c3d4a27` `9e54a25e` `daf3528c` `0fb15839` | 4238 |
| 10 | Campaign beats, dialogue, and placement | `bc10e342` `99b69dbe` `ef6320ba` `f9533456` `2cfaa902` `e2a1eb45` `9f25b29b` `a0d93564` `9ee0de9b` `f05298fe` `392b19a2` | 1250 |
| 11 | Playable benches and the probe roster | `eda1e6ce` `fb347d4b` `568149b2` | 2215 |
| 12 | HUD, comms, and campaign portraits | `8d558cd4` `80d8a237` `6703c967` `758edb7f` `22e00993` `9d25712d` | 873 |
| 13 | Web comic player (SOLO) | `97ffc7b6` | 1882 |
| 14 | Content move and build infrastructure | `db060c1e` `138dfbfc` `6a24692c` `dfb60ab2` | 4877 |

### Not reviewed

Task-only commits carry no code and are excluded by design. Name them so the
skip is on the record, not implied: `bb4a99f7`, `90fe940d`, `e42298b0`,
`7243f222`, `f2cfd0b4`, `9da8b055`, `a8eefd57`.

`--play` was not requested, so the red team and feel lanes do not run. Nothing
in this range is judged for how it feels to play.

## Progress

All 14 batches adjudicated and written up. Verdict written. Fix pass done and
written up below - the range is green on every gate that was red.

The red `no_mainline_handler_posts_an_objective_alongside_a_conversation` is
attributed: `f9533456` and `e2a1eb45`, both in batch 10. Batches 8 and 9 no longer
need to chase it.

## Findings

Each batch appends its adjudicated findings here as it completes.

### Batch 1 - Asteroid material shading (`b202d69f`)

Two lanes, both returned. Every load-bearing claim below was re-derived in the
main session before it was written down.

**BLOCKER - `examples/playable/planet_types.rs:192` - HEAD does not build.**
`AsteroidConfig::material` became `String` (`crates/nova_scenario/src/objects/asteroid.rs:67`)
and this one call site still passes `None`. Every sibling example was migrated
(`asteroid_kinds.rs:133`, `carve_asteroids.rs:817`, `second_shift_map.rs:510`,
`wfc_arena.rs:1115,:1359`). `cargo check --workspace --all-targets` fails with
E0308, which is `.github/workflows/ci.yaml:313`, and Clippy at `:102` too.
The file arrived in `db72da2e` (batch 2), so this is the seam between the two
commits the owner asked to review separately. Fix: `material: KIND_ROCK.to_string()`.

**BLOCKER - `crates/nova_probe_cli/tests/catalog_drift.rs:305` - the invariant roster is stale.**
`examples/systems/system_ship_editor.rs:1536,:1556` emit `outcome: the kind is
picked from a list, not typed` and `outcome: the picked kind is the rock on the
stage`; neither is on the roster and `SYSTEMS_INVARIANTS` is still 228. Verified
in this session: `cargo test -p nova_probe_cli --test catalog_drift` is 1 passed,
1 FAILED. Fix: add both slugs and set the count to 230.

**MAJOR - `crates/nova_authoring/src/base_content/impacts.rs:17` - four of the five kinds sound like ship plate.**
A kind id is now also the impact-table key, but the base table still ships one
material row, `impact_kinetic_rock` for `"rock"`. `metal`, `ice`, `carbon` and
`plain` fall back to `impact_kinetic`, the generic plate sound. That is 24 of the
60 hand-placed First Shift bodies, which used to play `impact_rock.wav`. The
module doc above it ("Only stone earns a second row, because it is the only
thing in the base game that is not ship plate") is now false. Fix: author
Kinetic rows for the other kinds and decide explicitly what `plain` keys to.

**MAJOR - `crates/nova_scenario/src/objects/asteroid_surface.rs:189` - `plain` is not the before picture.**
`AsteroidSurfaceMaterialExt::new` chains `.with_seed(seed)` for every kind, and
the shader spends `jitter` at `asteroid_surface.wgsl:306-312` before any kind knob
can turn it off - a per-seed rotation and offset of the sampling frame that the
pre-change shader could not produce. So three claims are wrong: `plain()`'s doc
("byte for byte what a rock was drawn as before kinds existed",
`asteroid_kind.rs:363`), `examples/playable/asteroid_kinds.rs:11`, and the
capture the task folder rests its judgement on. `the_plain_kind_turns_every_knob_off`
cannot catch it: `jitter` is not a field of `AsteroidKindLook`. Not a BLOCKER
because no shipped content authors `plain` (0 uses across `assets/` and
`webmods/`). Fix: give the look a `jitter` knob `plain` zeroes, or drop the
"before picture" claim from all three places.

**MAJOR - `assets/shaders/asteroid_surface.wgsl:318-344` - `plain` pays for the work it is documented not to do.**
Only the Worley layer sits behind a uniform branch (`:355`). The domain warp,
the 4-octave fBm and the SECOND triplanar read all run unconditionally and are
then multiplied by `warp`, `macro_scale` and `break_up`, which are 0 for `plain`.
The cost is UNMEASURED and neither lane could measure it: there is no committed
probe run for a rock-heavy range and a before arm needs a release build of
`b202d69f^`. Fix: wrap `:318-344` in the same uniform-branch idiom the veins use.

**MAJOR - `crates/nova_scenario/src/objects/asteroid_kind.rs:45` - `KIND_ROCK`'s doc teaches the fallback this change exists to remove.** (both lanes)
"Ordinary stone: the default, and what an unknown id falls back to... a rock that
authored nothing keeps the material id it has always had." The module doc sixteen
lines above says "There is no default kind and nothing resolves an absent one",
`asteroid_kind_look` returns `None` for an unknown id, and the field is required.
This is published rustdoc and the first thing a mod author reads.

**MAJOR - creator docs still document the old optional field.**
`web/src/create/impacts.md:58` - "an asteroid sets `material`; omitted means
`"rock"`". `web/src/create/base-content.md:123` - "The two materials the base game
names are `"hull"` and `"rock"`"; it now names five and ships four in its own
scenarios. A mod author reading the impact reference omits the field and their
bundle refuses to load.

**MAJOR - `web/src/create/objects.md:105` - the migration note exempts the files that need migrating.**
"Migrating a file written before 0.12.0". 0.12.0 IS the last release
(`CHANGELOG.md:342`, 2026-08-31) and this change is in `[Unreleased]`, so every
mod in the wild reads the note as not applying to it.

**MINOR - `crates/nova_scenario/src/actions/spawn.rs:399` - the runtime scatter guard checks weights, not ids.**
A mod authoring `asteroid_kinds: [("granit", 1)]` clears the guard, spawns the
full count, and every body then hits the render refusal at `asteroid.rs:450`: a
field of rocks with colliders, gravity and lock signatures, and no mesh. The
predicate already exists - `is_asteroid_kind`, used by the lint at
`lint/scenario.rs:509`.

**MINOR - `spawn.rs:328` - `asteroid_kinds` has no `serde(default)`, so EVERY `ScatterObjects` must author it,** not only asteroid ones (`asteroid_radius` directly above it does have one). `CHANGELOG.md:97` says "An asteroid template must author one" and carries no `**(breaking)**`, though it breaks every scenario file with a scatter. `web/src/create/actions.md:129` is borderline right ("Empty on any other template"), the changelog is the wrong half.

**MINOR - `spawn.rs:317` - a stale sentence inside the new field's own doc:** "An empty mix... leaves the template's own `material` alone" contradicts the paragraph at `:323` and the guard at `:399`, which refuse to spawn. Same drift at `asteroid_kind.rs:100`.

**MINOR - `crates/nova_scenario/src/lint/scenario.rs:443` - `check_sequence`'s doc block now sits on `check_asteroid_kind`.** (both lanes) The new function was inserted between an existing doc comment and the function it described; `check_sequence` (now `:530`) has none.

**MINOR - `examples/systems/system_ship_editor.rs:1531,:1552` - two assertion messages carry runs of literal spaces** where a `\` line continuation was lost. `rustfmt` does not touch string contents, so they print that way at the one moment they have to read.

**MINOR - `crates/nova_scenario/src/objects/asteroid_surface.rs:142` - `AsteroidSurfaceUniform::new`/`with_seed` is a two-step builder with one caller** whose only valid product is the chained form; `new(look)` alone leaves `jitter: 0.0`, the state the change exists to remove. Fold to `new(look, seed)`.

**MINOR - `crates/nova_editor/src/inspect.rs:1768` - `choose_field` now writes any `String` target unvalidated** while its doc (`:1759`) still says "Switch the enum at `path`... Unit variants only". Only `apply_choice` reaches it today and only over `ASTEROID_KINDS`, so nothing bad arrives - but the guard is written wider than the field it exists for.

**NOTE, not a finding - installed copies of this repo's own webmods stop loading.**
The lane's run logged `Unexpected missing field named 'material' in AsteroidConfig`
for `gauntlet` and all six `the-ledger` chapters from the local install cache,
after which the campaign reports every chapter as missing. The sources under
`webmods/` ARE migrated; the published copies need republishing with the release.

**Verified in this session:** the E0308 site and the `material: String` field; the
`catalog_drift` failure, by running it; the one-material impact table; the
unconditional `with_seed` and the shader's use of `jitter`; zero `plain` uses in
shipped content; the scatter guard; and all four stale doc lines.

**Checked by the lanes:** workspace check with and without `--features debug`;
`content lint` (0 errors, 10 scenarios); generated-RON parity by reading (124
`Asteroid` blocks, all with `material`; every `ScatterObjects` with
`asteroid_kinds`; 165 authored ids all shipped); the example mod migrated by
hand; `AsteroidSurfaceUniform` field-for-field against the WGSL struct (108 B ->
112 at align 16, so dropping the padding is sound); a rendered `asteroid_kinds`
run under Xvfb (shader compiles, six capture steps, exit 0); scoped tests in
`nova_scenario` (64), `nova_editor` (89), `nova_probe_cli`; identifier sweep
across `assets/`, `webmods/`, `examples/`, `web/`, `docs/`, `crates/`.

**Not checked:** any frame-cost number (no before arm exists; recipe recorded in
the MAJOR above); the wasm32 build and a WebGL2 run - the 112-byte layout is
arithmetic, not a green build; the six rendered captures judged as art;
`system_ship_editor` run end to end; whether `asteroid_kind_at` reproduces the
kinds already in the generated RON.

### Batch 2 - Planetoids as seeded planet types (`db72da2e`)

Two lanes, both returned, and both landed on the same blocker independently.

**BLOCKER - `examples/playable/planet_types.rs:142` - the commit's own demonstration example panics at startup.**
`planet_plugin` adds `PlanetSurfacePlugin` through `with_game_plugins`; `AppBuilder::build()`
then adds `NovaScenarioPlugin` -> `ScenarioObjectsPlugin` -> `PlanetPlugin { render: true }`
(`crates/nova_scenario/src/objects/mod.rs:79`) -> `PlanetSurfacePlugin`
(`crates/nova_scenario/src/objects/planet.rs:204`). `PlanetSurfacePlugin` overrides no
`is_unique`, so Bevy panics before the first frame. Only a headless run survives, because
`PlanetPlugin { render: false }` skips its add. One lane confirmed by log that
`ScenarioObjectsPlugin` adds it exactly once on its own, so the second add is unconditional.
`compare_asteroids.rs` and `compare_planets.rs` set the precedent: neither adds a surface
plugin. Nobody has run this example since `PlanetPlugin` was wired in, because the file
does not compile for the batch-1 reason. Fix: delete the add and the comment above it.
PROOF TO RERUN after the build fix: run the example.

**MAJOR - `crates/nova_scenario/src/objects/planet.rs:77` - ~50 ms of main-thread work per planet on the spawn frame, paid headless too.**
`planet_scenario_object` calls `PlanetVisual::build(&config, PLANET_SUBDIVISIONS)` synchronously
inside the command batch. At 48 subdivisions that is 24,012 vertices x 3 `shape.radius()`
samples, each a 6-octave `Fbm<Perlin>` plus a 4-octave `RidgedMulti<Perlin>` - about 720k Perlin
evaluations, on top of a 4,096-direction sweep. Measured differentially at 3 repeats: ~50 ms
per planet at subdiv 48, against 0.02-0.03 s for a whole single-rock test. First Shift spawns
both belt planetoids in one `OnStart` (~100 ms, 6 dropped frames) on the frame the chapter
opens; the menu backdrop spends ~50 ms on the game's first rendered frame, which is the frame
this commit exists to make look good. The build is UNCONDITIONAL while only
`insert_planet_render` is gated on `render`, so a headless probe or integration test pays it
for a mesh nothing reads. Fix: build the visual from the render observer, and either lower
`PLANET_SUBDIVISIONS` or spread the build behind the loading cover.

**MAJOR - `crates/nova_scenario/src/objects/planet.rs:103` - `invulnerable` is a required authored field that does nothing.**
An asteroid enforces the flag by withholding `DamageMarks` and `CollisionEventsEnabled`
(`objects/asteroid.rs:263`); the planet's collider child never gets them. `PlanetInvulnerable`
is written at `:87`, registered at `:200`, and read NOWHERE in the workspace - verified by grep.
A creator authoring `invulnerable: false` and building a mission around destroying the body
gets a clean load, a clean lint, and a body that never emits `OnDestroyed`.
`web/src/create/objects.md:152` promises the opposite, and `crates/nova_authoring/src/balance.rs:586`
counts it as destructible cover. This is the silent-default shape `AGENTS.md` bans, one layer up.
Not a BLOCKER only because no shipped content authors a destructible planet.
Fix: enforce it, or drop the field and make `invulnerable: false` a `check_planet` error.

**MAJOR - `crates/nova_editor/src/preview.rs:317` - scrubbing a planet's radius regenerates the whole surface every frame, for nothing.**
`radius` is in the planet's `drawn_fields`, so a held scrub fires `ObjectBodyStale` once a frame
and `sync_object_views` rebuilds at `PLANET_EDITOR_SUBDIVISIONS`: ~10-15 ms measured, on the
editor frame while the pointer is down. The waste is total - the mesh is unit-space and `radius`
reaches it only through `relief_fraction()` when `relief` is `Some`; for the default authored
planet (`relief: None`) the new mesh is bit-identical to the one just thrown away and only the
scale needed to change. Fix: drop `radius` from the planet's `drawn_fields` and rescale, or gate
on `relief.is_some()`.

**MAJOR - a new object kind and a new render pipeline land with no harnessed range and no `outcome:` markers.**
`examples/systems/README.md` asks a substantial feature for a range whose claims are on the
`catalog_drift.rs` roster. This commit adds `ScenarioObjectKind::Planet`, a gravity-well observer,
a displaced-icosphere mesher and a full fragment shader, swaps three shipped bodies onto them, and
adds zero markers. The 21 new unit tests pass but none reaches `insert_planet_render` (they run
`MinimalPlugins`), so the spawn -> `BodyRadius` -> `GravityWell` -> `Mesh3d`/`MeshMaterial3d`
sequence, the WGSL compile, and the fragment cost are all uncovered. Live evidence gathered with
the measurement slot: `screenshot_planet_editor` under Xvfb, exit 0, no pipeline or naga error,
both captures correct. The shader is right today; nothing holds it there.

**MAJOR - `web/src/create/filters.md:41` and `web/src/create/events.md:342` - the `type_name` catalog omits `"planet"`.**
Both pages enumerate the closed set - six kinds, no planet - while `PLANET_TYPE_NAME` is on every
spawned planet, so `Entity((type_name: Some("planet")))` is legal as of this commit. A creator
reading either page writes an id filter per body instead. `objects.md`, `actions.md` and
`reference.md` were updated; these two were missed.

**MINOR - `crates/nova_editor/src/preview.rs:194` - the editor draws a planet `(1 + relief)` too large.**
`PlanetVisual::build` returns vertices in `[1 - relief, 1 + relief]` of unit space. The game scales
by the MEAN radius (`planet.rs:105`), so the drawn surface is exactly the published `BodyRadius`;
the editor scales by `body_radius()`, which already carries the factor, giving
`radius * (1 + relief)^2`. 5% for DustWorld, 6% for Volcanic, and the pick collider on the same
line no longer matches the drawn mesh. The comment above the branch is what the fix makes true.

**MINOR - `crates/nova_scenario/src/objects/planet_surface.rs:483` - the module doc states the opposite of what this commit wired.** (both lanes)
"NOT added by `ScenarioObjectsPlugin`... An app that wants planets adds this itself." Following
that instruction is exactly what makes the blocker above panic.

**MINOR - `crates/nova_scenario/src/objects/mod.rs:55` - `ScenarioObjectsPlugin`'s doc lists its members and omits the planet** (and `anchor` and `asteroid_carve`, from before this commit), one line under a module header calling it "the only registration point".

**MINOR - `crates/nova_os_ui/src/map/contacts.rs:515` - the new terrain classification has no test,** though its own doc warns that an unlisted kind "vanishes from the map silently"; `map::` passes 17 tests without constructing a `PLANET_TYPE_NAME` contact. `:520` `terrain_name` is also a non-exhaustive `if/else` twin of the exhaustive `is_terrain`, and `:251` carries a stray blank line.

**MINOR - `crates/nova_editor/src/inspect.rs:1264` - observed: the Planet Type tooltip describes the previous type.** In the lane's own capture the `Temperate` chip is selected, the body is temperate, and the tooltip reads `DustWorld`'s doc verbatim. Not isolated to a stale reconcile versus an index mismatch, and possibly generic rather than new - `planet_type` may be the first all-unit-variant enum on an object config.

**MINOR - `crates/nova_scenario/src/actions/spawn.rs:196` - a scattered planet cannot vary.** A rock copy gets its own silhouette seed off a separate stream; a planet's seed is authored, so `count: 20` places twenty identical worlds and `check_planet` says nothing. Either draw a per-copy seed or lint the case.

**MINOR - the `planetoid` builder is written out three times verbatim** (`examples/playable/second_shift_map.rs:472`, `examples/playable/shared/first_shift_stage.rs:92`, `crates/nova_authoring/.../nova_protocol/stage.rs:214`), and `first_shift_stage.rs:11-19` duplicates six belt constants against `nova_authoring` with only a comment holding them in sync. The pattern predates this change, which is why it is MINOR - but this commit doubled what must stay in step.

**MINOR - `crates/nova_authoring/.../nova_protocol/stage.rs:44` - `CONCEALMENT_POS`'s doc still quotes the rock mesh's "1.75-3.0 km hull".** That was `500 m x [3.5, 6.0]`; the body is now exactly 2 373.8 m of surface radius and the ratio is 2.37. Both neighbouring constants were re-derived in the same edit.

**MINOR - `crates/nova_scenario/src/objects/planet_type.rs:596` - `SeedStream::pick` carries two doc comments,** the first a leftover; rustdoc renders both.

**MINOR - `docs/development.md:257,:349` - the example catalog knows neither `planet_types` nor `screenshot_planet_editor`,** and the latter writes `feature-`prefixed captures with no alias in `scripts/gen-web-screenshots.py`, so nothing packages them.

**MINOR - `web/src/create/objects.md:180` - "a body about 1 km across" is the RADIUS** (2 km across). The next sentence gives the right number, which is why it is not higher.

**MINOR - `CHANGELOG.md:64` - `**(breaking)**` on a change that breaks no format.** `ScenarioObjectKind` gained a variant, `AsteroidConfig` is untouched here, every scenario RON that loaded before still loads and behaves the same. Arguable: if the intent is "the base game's planetoids changed shape", say that without the tag.

**Verified in this session:** both `PlanetSurfacePlugin` add sites and the missing `is_unique`;
`PlanetVisual::build` called unconditionally in `planet_scenario_object`; `PlanetInvulnerable`
having no reader; the editor scaling by `body_radius()` against the game's `radius`; and both
stale `type_name` doc lines.

**Checked by the lanes:** `nova_scenario --lib planet` (21/21), `nova_editor --lib` (437/437),
`nova_os_ui --lib map::` (17/17); a live Xvfb run of `screenshot_planet_editor` with both captures
inspected; differential timing of `PlanetVisual::build` at both subdivision counts; body-radius
parity arithmetic for all three re-authored bodies (all inside the 1% the tests assert);
`octave_safe_seed` against `noise-0.9.0`'s `build_sources`, including the truncated-octaves case;
every workspace `match` on `ScenarioObjectKind` carrying a `Planet` arm; the shader's band order,
cap latitude, lattice bounds and seed separation; generated-RON parity by reading; `content lint`
(0/0, 10 scenarios); units (`Meters` throughout, `to_engine`/`from_engine` at stated boundaries);
portability and WebGL2 std140 alignment; the format-break question (additive, nothing to migrate).

**Not checked:** any GPU frame-time number for the new fragment shader - no probe range covers a
planet and the one example that would neither compiles nor would survive the panic; the First
Shift and menu scenes at runtime; whether the shader reaches the web bundle; whether the tooltip
defect predates this commit; whether avian rescales the editor preview's collider (reasoned from
avian 0.7 source, not observed - if that reasoning is wrong, the preview MINOR is a BLOCKER).

### Batch 3 - Flight model: RCS, overspeed, speed budget (`2fb42ef9` `7a960c5b` `e2b73a3f` `42343d27`)

Four passes over one system. HEAD judged as the truth. Both lanes returned.

**BLOCKER - `crates/nova_ship/src/flight/tests/orbit.rs:63` and `:202` - `nova_ship` is red at HEAD.**
Verified in this session: `cargo test -p nova_ship --lib flight::` is 113 passed, 2 FAILED.

1. `strong_gravity_orbit_holds_the_ring_on_the_main_drive_not_rcs` - "in a strong well the orbit
   stays on the main drive - RCS lacks the authority, so it must not engage". Attributable to
   `7a960c5b` by arithmetic: the fixture is `mu = 43_350` on an 85 u body, so the r=140 ring pulls
   `43350/140^2 = 2.212 u/s^2`. The gate is `orbit_gravity_accel < rcs_accel * 0.5`
   (`autopilot.rs:630`). At the old `rcs_accel = 1.5` the threshold was 0.75 and the gate was shut;
   at the new 4.905 it is 2.4525, and `2.212 < 2.4525`, so RCS now takes station-keeping in the well
   that was the original regression subject ("the two menu ambience ships crashed the asteroid").
   The shipped body now sits at 1.11x of the threshold - inside a seed's width of the boundary the
   constant exists to keep it away from.
2. `orbit_engages_from_near_rest_and_holds_the_ring_for_a_lap` - station-keeping never reaches Hold.
   Its sibling `orbit_engages_rcs_only_to_trim_a_sub_cap_residual` passes on the identical well,
   radius and ship, because `42343d27` pinned that one to `RcsSpeedCap(2.0)` and left this one on
   the default. At `rcs_speed_cap = 10 u/s`, `v_circ` at r=50 is 4.9 u/s, so the WHOLE insertion is
   handed to RCS with `demand = 0.0` and no `choose_group` alignment; the trim gain falls from
   `1.5/2.0 = 0.75` to `4.905/10 = 0.49` per second and the residual never reaches
   `orbit_hold_enter` inside the window. The batch spotted this premise breaking on one test and
   fixed only that one.

Fix: decide which side of the gate the menu body belongs on now that RCS is 3.27x stronger, then
move the FIXTURE or re-tune `RCS_ORBIT_GRAVITY_AUTHORITY`; and decide whether ORBIT should hand a
from-rest insertion to RCS at all. Do not widen either assertion.

**MAJOR - `crates/nova_ship/src/flight/autopilot.rs:621` - the RCS settle path has no gravity gate, and this batch widened it 5x.**
`use_rcs_settle = rcs_capable && desired.length() <= stop_speed_epsilon && velocity.length() < rcs_cap`.
The `rcs_has_orbit_authority` term exists only on `use_rcs_orbit` two lines below - verified by
reading. For STOP, `desired` is exactly zero for the whole descent, so once `|v| < 10 u/s` the
branch latches: `demand = 0.0` cools the drive and `!fine && !use_rcs` stops the hull ever facing a
braking engine. The RCS command is proportional, so delivered acceleration is `0.4905 * |v|` and the
equilibrium against a steady inward pull `g` is `|v| = 2.04 g`. At First Shift's inspection planetoid
(`INSPECTION_MASS = 27_000`) that is 24.5 m/s of steady fall at the 500 m standoff and 55 m/s at the
surface - both under the cap, so RCS keeps the ship all the way down and `done` never fires. Before
this batch the cap was 2 u/s and the main drive took back over at 20 m/s. The one range that pairs a
well with an arrival, `tests/goto.rs:121`, calls `withhold_rcs` first, so it deliberately excludes
this path. UNMEASURED - reasoned from the code and the shipped constants. Fix: apply the same
authority term to `use_rcs_settle`, and add a range that engages STOP inside a well on an
RCS-granted hull.

**MAJOR - `crates/nova_ship/src/flight/manual.rs:79` - at the cap, RCS cannot change direction at all.**
`gate.min(step_inside_sphere(residual, push, speed.max(cap)))`: the sphere radius is
`max(|residual|, cap)`, so a hull sitting on the sphere is tangent to it and any perpendicular push
solves to `s = 0` - not tapered, zero. The repo's own test asserts it deliberately
(`manual.rs:417`), so this is a design call rather than a slip; the finding is that the design meets
shipped content badly. First Shift's RCS lesson is a four-mark box whose legs are mutually
perpendicular (`first_shift/marks.rs:117-150`; TRIM A->B is a 220 m hop straight up off a lateral
leg) and whose doc calls each leg "a real maneuver at the 100 m/s RCS cap". A 220 m hop reaches the
cap, and the pilot then finds the next axis dead until they first push back down the one they came
in on. Fix, if the behavior is not wanted: bound the speed, not the direction - allow the tangential
step and clamp the RESULT back onto the sphere.

**MAJOR - `web/src/widgets.ts:111-112` - the docs site's RCS constants are the pre-batch defaults, 5x and 3.3x stale.**
`RCS_ACCEL = 1.5` and `RCS_SPEED_CAP = 2.0` against `state.rs:443-444`'s
`MetersPerSecond(100.0).to_engine()` = 10 u/s and `MetersPerSecondSquared(5.0 * 9.81).to_engine()`
= 4.905 u/s^2 - verified. They are live: `gotoSim` (`widgets.ts:822,826`) hands the leg to RCS at
`v <= RCS_SPEED_CAP` and decelerates at `RCS_ACCEL`. So `/wiki/flight-autopilot/` states 100 m/s at
5 g in prose at line 75 and, 37 lines above, plays an interactive GOTO scope that keeps the main
drive lit to 20 m/s and crawls home at 15 m/s^2. The widget's own note claims "standoff and RCS
settle are the game's own rules". The widget is the half that looks measured.

**MAJOR - First Shift's governor is 150 m/s; five places still say 250.**
`first_shift/marks.rs:48` is `CUTTER_SPEED_CAP = MetersPerSecond(150.0)`, applied at `mod.rs:357` -
verified. Stale: `web/src/wiki/glossary.md:17` and `:102`, `web/src/widgets.ts:287`
(`PLAYER_SPEED_CAP = 250`, rendered at `:6150` and `:6224` as a closing-rate figure),
`crates/nova_authoring/src/base_content/sections/ordnance.rs:24`, and
`crates/nova_ship/src/input/ai/guns.rs:56`. `flight-autopilot.md` and `getting-started.md` WERE
corrected in this batch; the glossary was edited and kept the old number. The two Rust sites are
lead-computation justifications, so the code is not wrong - its stated reasoning is unverifiable.

**MAJOR - `crates/nova_ship/src/flight/autopilot.rs:33` and `crates/nova_ship/src/flight/tests/support.rs:385` - both docs state arithmetic the code contradicts.** (both lanes)
`autopilot.rs:33`: "The menu planetoid's ~2.2 u/s^2 pull far exceeds `rcs_accel * 0.5 = 0.75`, so
its orbits stay on the main drive" - the threshold is 2.4525 and they no longer do.
`support.rs:385`: "`mu/r^2 ~= 2.2 u/s^2` EXCEEDS `rcs_accel` (1.5)" - it is 4.905 and 2.2 does not.
These are the two places a reader checks before touching the gate, and both say the opposite of what
runs. This is the same root as the first blocker; recorded separately because the fix is a
re-derivation, not a re-tune.

**MINOR - `crates/nova_ship/src/flight/manual.rs:195` - the manual burn's budget is evaluated against a full-stick step whatever the throttle.**
`step` is `authority / mass` with no `burn` factor, and the returned scale then multiplies `burn`.
Both `growth` and the sphere term depend on `|push|`, so a 20% throttle across the velocity vector
is tapered as if it were full stick. Convexity keeps the result inside the budget, so it is
conservative rather than unsafe - the ship just stops accelerating earlier than commanded.

**MINOR - `web/src/widgets.ts:192,2855,2912,3384,3410` - five `attitude.rs:<line>` citations went stale by exactly the eight lines `2fb42ef9` inserted.**
Subtract 8 and every range lands. `widgets.ts:96-101` states the rule these five break: "Cited by
FIELD, not by line: the file moves and a stale line number reads like a checked fact."
`web/src/wiki/sections/controller.md:27` was updated in the same commit; `widgets.ts` was not.

**MINOR - `web/src/wiki/sections/controller.md:27` - the new `pd_controller.rs:156-190` citation points at the axis-angle extraction, not the direction rule it names** (`brake_only_past`, `:196-216`).

**MINOR - `crates/nova_ship/src/flight/manual.rs:292` - the RCS taper band's reason for differing from the manual burn's has lapsed.** "Small cap by design, so the manual-burn `.max(1.0)` floor... would swamp it" - the RCS band is now 2.0 u/s against First Shift's manual 3.0, the same order. Neither floor binds at shipped values, so nothing misbehaves; the next person tuning the taper reads a false premise.

**MINOR - `crates/nova_ship/src/flight/state.rs:109` - `RcsSpeedCap` documents a per-hull authoring path nothing reaches.** No insert anywhere in `nova_scenario`, `nova_authoring`, `assets/` or `webmods/` - unlike `FlightSpeedCap`, which has an authored field, a scenario action and a console command. This batch made the gap load-bearing: raising the default forced three tests to insert it by hand (`tests/orbit.rs:82`, `tests/rcs.rs:257,277`), so it is now a test-only fixture knob with a doc claiming otherwise.

**MINOR - `web/src/wiki/flight-autopilot.md:71` and `crates/nova_ship/src/flight/state.rs:402` - the "fine docking thrusters / last few meters" framing contradicts the numbers two lines below.** At 4.905 u/s^2 against the 2-20 u/s^2 drive range the site's own scope offers, RCS is 25-250% of a main drive with a 100 m/s ceiling. Same framing in `glossary.md:113` and `keybinds.md:106`. Nothing states a wrong number - only a wrong impression, and one that stops a player using RCS as a travel mode.

**MINOR - `docs/sections.md:107` - the stack pass's field list omits `sustained_angular_speed` and the one rule about it that is not obvious:** unlike authority it is NOT shared out, because every controller fights the same metal (`controller_section.rs:509`). The unit test catches a wrong "fix"; the chapter that should explain the mechanism does not mention it.

**MINOR - `crates/nova_ship/src/flight/state.rs:444` - `9.81` is written literally in two crates with no shared name,** while both figures it produces are printed to players as G-multiples. `nova_events::scale` already holds `LOAD_LIMIT = MetersPerSecondSquared(8.0 * 9.81)`; a `STANDARD_GRAVITY` beside it is where the house rule puts a cross-crate constant.

**MINOR - `docs/development.md:263` - the `systems/` tour names `system_attitude_hold` but not `system_turn_limit`,** the range that proves the rule this batch added and that `docs/sections.md:90` sends readers to.

**Verified in this session:** the two red flight tests, by running them; the stale `widgets.ts`
constants against `state.rs`; `CUTTER_SPEED_CAP = 150` and all five 250 m/s sites; the stale
`autopilot.rs:33` arithmetic; `use_rcs_settle` carrying no authority term while `use_rcs_orbit` does;
and `speed_budget_scale` returning exactly 0.0 for a tangential push at the cap.

**Checked by the lanes:** all four diffs and every touched file read whole at HEAD; the units seam -
`to_engine` applied once at `FlightSettings::default` and at the `structural_arm` boundary,
`sustained_angular_speed` and `structural_ceiling` scale-invariant, no world-unit slip in new code;
`speed_budget_scale` and `step_inside_sphere` algebra by hand at every edge (zero push, zero
residual, at the cap, well past it, tangential, oblique, retrograde, over-long step, negative and
zero cap); `brake_only_past` edges; fixed-step ordering (all four systems `.chain()`ed in
`FixedUpdate` after `ControllerSectionSystems::SyncStack`, no budget spent twice);
`nova_ship --lib physics::` (31), `sections::controller_section` (17); a live `system_turn_limit`
run under its own Xvfb, reproducing the commit message exactly (1.306 rad/s over 46.0 m intact,
1.485 rad/s over 35.7 m shortened, 14% harder) with its three markers on the roster; the CHANGELOG
entries correctly COLLAPSED (four commits -> one Gameplay & Flight, one Ships & Sections, one Fixes),
all under 200 characters, right subsystems, measured against 0.12.0; `web/src/create/actions.md:546`
stating the new total-speed semantics correctly in m/s.

**Not checked:** the two red tests were not run at their pre-batch parents - attribution is by
arithmetic and by the passing/failing sibling pair, not by bisect; the settle-into-a-well and
cap-tangency findings were not flown, because the mainline scenario cannot load at HEAD;
`balance_throttles`' recruited off-axis delta-v, which no shipped hull exercises; first-tick
behaviour when `ComputedMass` is still unmeasured; frame cost was reasoned, not measured (the batch
adds a handful of `sqrt` per tick, no allocation, no new query pass, and REMOVES a `choose_group`
call). `nova_authoring`'s red pacing test is NOT attributable here: none of the four commits touch
`nova_authoring`, `nova_scenario`, or any scenario content.

### Batch 4 - Helm orders and the GOTO arrival model (`6ee44142` `b595472e` `356f3f68` `477d92aa`)

Both lanes returned. No new red test: this batch's own suites are green
(`nova_authoring --lib first_shift` 21/21, `nova_scenario` filtered 82,
`nova_gameplay --lib integrity` 73, `nova_ship` filtered 121), and the range's
existing failures are attributed elsewhere.

**MAJOR - `crates/nova_ship/src/flight/order.rs:720` - `fail` never gives back the standoff it staged.**
Verified by reading all four terminal paths: `complete` (`:681`) restores, `retire_ship_order_execution`
(`:741`) restores, `fail` (`:720`) only pushes the report, latches `ShipOrderReported`, and removes
`ShipOrderHelmAuthority` and `ShipOrderEngaged`. `b595472e`'s own message says a completed order now
returns its margin "as cancel and interrupt already did" - the fourth path was missed.
Failure: `MoveShipTo` with `arrival_standoff: Some(200 m)` on an AI ship; the last live thruster dies
mid-leg, the autopilot disengages, and `drive_ship_orders` takes the `Move | Stop` arm with
`can_burn == false` and calls `fail`. The hull keeps `FlightArrivalStandoff(20.0)` and a dangling
`SuspendedArrivalStandoff(None)` for the rest of the scene. Because `fail` also hands the helm back,
`update_passive_flight` resumes and the patrol advance gate (`passive.rs:232`) now resolves 200 m
instead of the global 500 m; a later `MoveShipTo` with `None` flies the dead order's margin, and a
later `Some(y)` records the leaked value as "previous" and restores THAT on completion.
The two tests miss it by one line each: `a_completed_move_gives_the_ships_own_standoff_back` stages a
margin and completes; `a_move_that_loses_its_engines_fails_instead_of_reporting_an_arrival` fails with
none staged. Fix: have `fail` call `retire_ship_order_execution`. That also clears the second half -
`fail` leaves `ScriptedAlign` installed, inert today only because `drive_scripted_align`'s guard and
`can_turn` read the same live-controller set.

**MAJOR - the ORBIT band floor is a second arrival distance, and no player page states it.**
`autopilot.rs:522` is `(target_radius + mover_radius + arrival_standoff).max(floor) - target_radius -
mover_radius`, where `floor` is `orbit_band_floor` for any target carrying a `GravityWell`
(`autopilot.rs:506`). The code comment even names the effect: "the margin, or more where the band floor
pushed the leg out". Re-derived in this session for the concealment planetoid (2 373.8 m geometric,
`surface_margin = 1.0` u, `orbit_clearance_factor = 1.5`): floor = `1.5 * (237.38 + 1.0)` = 3 575.7 m
from centre, against the margin model's `2373.8 + 55 + 500` = 2 928.8 m. The floor wins, and the hull
rests about 1 147 m off the surface where `web/src/wiki/flight-autopilot.md:34` promises "about 500 m".
The crossing is at a body radius of roughly `970 m + 2 x hull`, so the inspection planetoid (997.5 m)
does NOT do this - which is why playing the taught chapter will not surface it.
`glossary.md:21` is separately inconsistent with `flight-autopilot.md:34` on the hull term itself: the
commit added "measured from your own hull" to the autopilot page and left the glossary saying margin
plus the target's radius, with no hull. Two pages, two arrival models, in the commit whose subject is
"one arrival model". `glossary.md:19` and `widgets.ts:39` also both promise the ramp "always lands on
the standoff". Fix: name the floor as a condition on the autopilot page, bring the glossary onto the
hull-inclusive wording, and soften "always lands on the standoff" to "on the resolved park distance".

**MINOR - the one arrival model's CENTRE distance is spelled four times by hand.**
`resolved_arrival_standoff` really is the single margin resolve, but `autopilot.rs:327`, `:522` (the
same expression re-derived at the call site, without the `.max(0.0)`), `:777` (a third spelling using
only the band well's radius where `:327` takes `max(BodyRadius, HullRadius, well.body_radius)`) and
`passive.rs:232` are four independent centre distances. The lane worked the algebra and found no
divergence at any reachable input, so this is maintainability, not a defect - but the point of the
commit is that these cannot disagree about one ship, and today they are kept in step by hand.

**MINOR - hull radius is now a live arrival input, and every content figure that budgets for it is a hand-typed literal.**
`first_shift/tests.rs:652` and `:705` type `CUTTER_RADIUS = 55.0` twice; `main_menu/weave.rs:79` reasons
from "the cutter's own 42 m hull" - 42 and 55 are two answers to one question inside one batch;
`marks.rs:319` states 119 m. Nothing ties any of them to `publish_hull_radius`, so a section-layout
change moves the arrival and leaves the corridor test and the trigger sizes behind.

**MINOR - `crates/nova_authoring/.../nova_protocol/stage.rs:167` - the beacon trigger's sizing rule is derived from the model this batch replaced.**
The comment states an invariant - "It MUST contain the autopilot's park point" - and derives 700 m from
"GOTO stops 500 m short of an UNSIZED target". A beacon is sized as of `beacon.rs:86`, and the mover's
hull counts too, so the campaign cutter parks at 575 m, not 500 m. Nothing breaks (575 fits inside 700,
and the trigger is a sphere sensor so the mover radius cancels), but this comment is the rule an author
reuses; applied to a bigger hull or a larger orb it under-sizes the trigger and parks the ship outside
the objective volume it just flew to. No test pins the MUST.

**MINOR - `crates/nova_ship/src/flight/state.rs:14` - `BodyRadius`'s contract is falsified by a producer the same batch added.**
The docstring says the component is "the geometric radius of a SOLID scenario body", "derived from the
actual generated collider ... rather than authored by hand", and what "the AI's patrol legs steer
around". `beacon.rs:86` now publishes it on an entity that is explicitly intangible, has no collider,
takes the number straight from the config, and is then name-excluded from the obstacle query
(`passive.rs:200`). The insert site documents all of that locally; the component's own contract still
denies it, and `docs/scenario-system.md:622` repeats the claim in bold as an "engine fact".

**MINOR - `crates/nova_scenario/src/filters.rs:170` - `ShipOrderFilterConfig` still documents itself as completion-only after growing four more outcomes.**
Six statements in the rustdoc say "completed" where the filter now matches five events. A creator
attaches a bare `ShipOrder` filter to a `Sequence` gate believing it fires only on arrival, and the gate
opens on `OnShipOrderFailed` or `OnShipOrderInterrupted`.

**MINOR - `crates/nova_scenario/src/lint/scenario.rs:1192` - `content lint` accepts a zero AI range the creator doc calls illegal.**
`check_ai_range` rejects only `< 0.0` and non-finite; `web/src/create/actions.md:1023` and `:1047` both
say the range "must be positive", and the sibling `SetAILeash` check at `scenario.rs:927` does reject
`<= 0.0`. The function's own docstring says a bad range "would either never trigger or trigger always,
both silently" - exactly what `Some(0.0)` does. Deliberately not the same as `arrival_standoff:
Some(0.0)`, where zero is authored, documented and tested.

**MINOR - `crates/nova_gameplay/src/integrity/core.rs:132` - the destruction tally pays in release for a line release never prints.**
`tally_a_destroyed_node` clones a `String` and walks a `BTreeMap<String, usize>` for every entity gaining
`IntegrityDestroyMarker`, unconditionally, and `report_destruction_tally` builds the joined `kinds`
string eagerly in a `let` before the `debug!`. A collapse frame destroys 812 nodes. `:178` also drops the
map's capacity every reporting frame. Unmeasured; the same frame already does 812 despawns and a physics
rebuild, and the change is a net win whenever debug logging is on.

**MINOR - `web/src/widgets.ts:76,77,188,465,487,720,820,824` - eight source citations invalidated by this batch's own line shifts,** in a file the batch edited. `state.rs:392/393` -> `:440-441`; `controller_section.rs:487-489` -> `:460` with the arm measurement moved out to `sections/hull_radius.rs:63`; `guidance.rs:307` -> `:322` (pushed by `orbit_band_floor`); `autopilot.rs:568-577` -> `:598-612`, where 568-577 is now the telemetry publish. The lane checked and EXCLUDED three more sets as already stale at `origin/master`.

**MINOR - `crates/nova_ship/src/sections/controller_section.rs:603` - `SyncStack`'s docstring still says it "Runs first"**; `477d92aa` put `SyncTarget` ahead of it (`:669`).

**MINOR - no probe range covers the helm-order family.** No `MoveShipTo`, `PatrolShip`, `OrbitShip`, `ClearShipOrder`, `ForceAlign`, `StopShip` or `OnShipOrder*` anywhere in `examples/`, and no roster slug. `6ee44142` adds five actions, five events, an interruption policy and a durable state machine; the unit tests drive `drive_ship_orders` on hand-set components in an `App::new()` with no autopilot, physics or event bus, so nothing exercises install -> fly -> interrupt -> resume -> complete.

**MINOR - `CHANGELOG.md:91` - one helm-order entry is filed away from its subsystem.** "A COMPLETED helm order gives back the `arrival_standoff` it staged" sits in Scenarios & Objectives; every sibling from this batch is in Modding & Mod Portal (`:189-208`, and the paired `arrival_standoff: Some(0.0)` at `:171`). It is the entry most likely to explain a mod author's regression.

**Verified in this session:** `fail` restoring nothing where `complete` and `retire_ship_order_execution`
both do; the `park_gap` expression and the `floor` term; `orbit_band_floor`'s formula and
`surface_margin = 1.0`; the concealment-planetoid arithmetic; and the two contradicting wiki pages.

**Checked by the lanes:** every named arrival edge (zero-radius target, target larger than the corridor,
hull radius over the margin, target despawned mid-order, completed-and-restaged); `park_gap` against the
arrival's own standoff and the ORBIT handoff ring; every `BodyRadius`/`HullRadius` reader for beacon and
ship sizing fallout; no second arrival distance in `nova_console`, `nova_autopilot`, `nova_info` or the
HUD; system ordering (`publish_hull_radius.before(SyncStack)`, the `SyncTarget/SyncStack/...` chain, AI
interruption ahead of the gated flight writers); the "wreck is gone" case for `477d92aa` (guarded at
`pd_controller.rs:106` and `controller_section.rs:717`, plus `SectionInactiveMarker`); a full identifier
sweep confirming `6ee44142` RENAMES nothing - every new action, event and filter is additive, so no
format break, no `**(breaking)**`, and no `webmods/` migration is owed; `content lint` clean; generated-RON
parity by reading; CHANGELOG per-commit attribution, with `356f3f68` correctly getting no entry
(intra-cycle) and `477d92aa`'s Fixes entry legitimate against 0.12.0; meters discipline in every new lint
message, doc, comment and constant (`AI_WAYPOINT_SLACK` 25.0 as 250 m, `AI_AVOID_MARGIN` 20.0 as 200 m,
`WARSHIP_APPROACH_STANDOFF` 200 m against a 119 m hull giving 319 m).

**Not checked:** the measurement slot was deliberately NOT spent - nothing here plausibly moves a frame a
range covers, and the one new per-frame allocation (the destruction tally) has no covering range; the
batch's live claims from its own task folder; arrival behaviour under real physics beyond the unit rig;
whether `CUTTER_RADIUS = 55.0` matches the shipped cutter's published `HullRadius` - no derivation exists
in the tree to check it against, which is the point of the third finding; the scenario RON round-trip and
editor UI for the new actions beyond `cargo check` and the existing serde tests.

### Batch 5 - Scenario scripting engine and the orbit goal (`eb3ea6ed` `68431eb3`)

Both lanes returned. Two blockers, and together they mean First Shift's orbit
beat cannot be completed as shipped.

**BLOCKER - `crates/nova_scenario/src/loader/trackers.rs:91` - scenario orbit progress is gated on a phase the shipped flight tuning does not reach.**
`let stable = autopilot.phase == AutopilotPhase::Hold;` - verified. Every orbit event and the whole lap
accumulation hangs off it. `Hold` is entered only when `error_speed <= orbit_hold_enter` (0.8 u/s = 8 m/s,
`state.rs:438`). The already-red `orbit_engages_from_near_rest_and_holds_the_ring_for_a_lap` is the proof:
its radius and speed assertions PASS (r within 0.8-1.25 of plan, speed within 35% of `v_circ`) and it fails
at `:202` on exactly `held` - "station-keeping should reach the Hold phase". The ring is flown; the label
never flips.
Consequence: the player enters the approach ring, presses O, and orbits perfectly forever.
`OnOrbitStable` never fires so `orbit_conversation()` never opens; `angular_travel` stays 0 so `OnOrbitLap`
never fires; the return gate is never created and `complete_objective(OBJ_ORBIT)` is unreachable. First
Shift stops at BEAT_ORBIT. This is the batch-3 RCS retune meeting `68431eb3`: the mainline gate was built
on a state the retune made unreachable, and HEAD is where they meet.
Fix: define scenario orbit stability from GEOMETRY - radius inside the planned band plus speed near
`circular_orbit_speed`, which is exactly what the same test proves DOES hold - rather than from an
autopilot presentation enum that flight tuning is free to move. Failing that, make `orbit_hold_enter`
reachable and pin it with a test.

**BLOCKER - `crates/nova_authoring/.../first_shift/marks.rs:288` - the orbit return gate covers only part of the reachable ring band.**
`ORBIT_RETURN_GATE_POS` is 1 595.8 m from `INSPECTION_POS` with a 300 m radius, so it spans orbital radii
1 295.8 - 1 895.8 m. The reachable band is `orbit_target_radius`'s clamp into
`[orbit_band_floor, orbit_band_safety * soi * (1 - fade_fraction)]`. Re-derived in this session from the
shipped constants: floor = `1.5 * (99.75 + 1.0)` = 1 511.3 m, ceiling = `0.9 * 0.85 * 3 286` = 2 514.0 m.
Rings from 1 896 m to 2 514 m are legal and never touch the gate. No `FlightSettings` or `GravitySettings`
override exists anywhere in the repo.
The doc comments are the cause and are wrong twice: `:283` "the widest orbit ring (1.82 km) is inside it
too" and `:288` "spans the inspection body's 1.36-1.82 km stable orbit band". The real band is 1.51-2.51 km.
Failure: `APPROACH_RING_RADIUS` is 2 400 m and the objective posts on entering it, so pressing O on the
prompt plans a 2 400 m ring. The lap completes, `OnOrbitLap` arms the gate, and the gate is 800 m off the
flight path - invisible by design, carrying no objective marker, with the objective text still saying to
orbit. The player has done exactly what was asked, is still doing it, and nothing on screen corrects them.
Fix: size the gate to the band it must intercept (about 610 m at that stand-off, not 300 m), or bound the
ring the beat accepts. Then re-derive both doc comments - they are load-bearing and they are what hid this.

**MAJOR - `crates/nova_scenario/src/objects/area.rs:181` - occupancy is keyed on a body avian stamps once, so a shot-apart ship's own debris holds its exit open.**
`on_collision_start_event` keys the row `(area, collision.body2)`. avian fills `body2` from the persistent
`ContactEdge`, stamped when the broad phase first creates the pair, and never refreshed -
`add_edge_and_key_with` returns `None` for an existing edge. `sever_disconnected_structures` re-parents a
live section onto a fresh fragment root (`integrity.rs:407`), which moves only the BVH proxy; the contact
edge keeps the OLD root in `body2`. So a duellist that sheds a chunk inside the 1 800 m arena keeps every
already-touching collider on that chunk recorded under the SHIP's row. The ship flies out, its own
colliders end, and `colliders.is_empty()` is still false - no `OnExit`, no forfeit. The debris separates at
10 m/s on top of inherited velocity, so the common case is a delayed forfeit and the bad case (a chunk
kicked inward, or a near-rest hull) is indefinite. `forget_collider_occupancy` cannot help: the collider is
alive. This is the exact class of stuck row the commit set out to kill, through a different door.
Live evidence is consistent but not proof: in the lane's backdrop run a ship split into two bodies and the
forfeit landed 7.3 s later, which cannot be separated from ordinary manoeuvring. The mechanism was verified
from avian 0.7.0 source. Fix: re-key on `ColliderOf` change with an observer that moves the row and fires
the implied transitions. Do NOT resolve the body from a live `ColliderOf` query in the handlers - start and
end would then disagree with the stamped row and the leak becomes permanent instead of temporary.

**MAJOR - `crates/nova_scenario/src/lint/scenario.rs:1104` and `crates/nova_scenario/src/actions/ship.rs:783` - the forced-fire diagnostic names a remedy that cannot work.**
Both say "use `non_combatant` for an armed ship that flies itself and never shoots". Verified:
`spaceship.rs:586` inserts `AISpaceshipMarker` UNCONDITIONALLY on the AI branch and `:598` adds
`AINonCombatant` beside it; the marker is never removed, and the batch's own test asserts it
(`spaceship.rs:929`, "and it is still an AI ship"). So the lint's `AI(_)` arm still matches and the runtime
`contains::<AISpaceshipMarker>()` refusal still fires. An author hits the error, sets `non_combatant: true`,
re-lints, and gets the identical error. No authoring input satisfies the message except the
`SpaceshipController::None` it already named. Now specific to the two forced-fire actions - batch 4 split
the helm orders onto `check_orderable_ship`/`orderable_ship`, which refuse only the player.

**MAJOR - `crates/nova_scenario/src/loader/trackers.rs:130,141` - a single tick out of Hold zeroes lap progress.**
`68431eb3` replaced `ORBIT_HOLD_SECS = 13.0` with TAU of signed angular travel, but both the well-change
branch and the stability-transition branch reset `angular_travel` to 0.0, so progress is continuous-hold
time rather than cumulative. At the reachable band that is one orbital period: 74 s at the 1 548 m park
radius, 142 s at the 2 400 m approach ring, 152 s at the band top. Any dip past `orbit_hold_exit` - a nudge,
a grazing hit, one unstable frame at two minutes in - restarts the whole thing silently, with the same
objective text on screen. An authoring decision rather than a bug, but it should be a deliberate one:
either accumulate across dropouts, or give the player a progress readout.

**MAJOR - `web/src/docs-manifest.js:643,668,687,708` - the wiki search index still describes the pre-batch engine.**
Verified: `ForceTorpedoLaunch` survives at `:708` and NOWHERE else in the tree. The summaries claim "All 28
actions", "sixteen event kinds" and "The four scenario filter kinds"; the truth is 42, 24 and 5. None of the
new constructs are indexed - `MoveShipTo`, `ForceAlign`, `StopShip`, `PatrolShip`, `OrbitShip`,
`ClearShipOrder`, `ForceRailgunFire`, `ForceTorpedoFire`, `ShipOrder`, `OnShipOrderComplete`, the orbit lap
material. `reference.md:52` promises "The wiki search indexes every construct name". So a creator following
the changelog's own breaking-change note searches `ForceTorpedoFire` and gets nothing, while the name they
were told to remove still resolves.

**MAJOR - `CHANGELOG.md` - `OnOrbitLap` has no entry at all.** Introduced by `68431eb3` (absent at
`origin/master`), and every sibling is recorded: the ship-order event family, `OnGotoComplete`/
`OnStopComplete`, `Suspend`/`ResumePlayerControl`. Not a policy of skipping event kinds.

**MAJOR - `docs/scenario-system.md:130` - the `Names` sibling list is two variants short.** `names.rs` now
declares seven; `eb3ea6ed` added `Order` (`:63`) and `Section` (`:66`). This is the developer contract for
adding a string field to a config, and neither new marker resolves against the spawn set - so someone
adding the next scripted-ship field reads a closed set that no longer is one and reaches for
`Names::Object`.

**MINOR - `crates/nova_scenario/src/objects/area.rs:122` - pruning an emptied row deletes it, so a body that never left can fire a second `OnEnter`.** If every overlapping collider of a body inside an area despawns in one burst - the destruction path that motivated the module - the row disappears while the body is still physically inside, and the next surviving section to touch the sensor re-creates it and passes the gate. The module's own doc cites the non-idempotent handler this breaks: the salvage crate's `despawn + crates_recovered += 1`.

**MINOR, UNMEASURED - every base backdrop now flies skinned block ships through a sensor volume, and nothing measures it.** The area module's own comment puts a block ship at "90+" colliders against "18 for a small modelled craft" and quotes 270 `CollisionStart`s over one duel; the lane's live run confirms the scale (61 plates over 15 shapes on one hull, 54 over 10 on another). No probe range covers a menu backdrop, and `system_menu_boot` measures boot-to-First-Shift, not backdrop dwell. The collider population of every shipped backdrop moved roughly 5x with no range to notice if it mattered. Unquantified, not cleared.

**MINOR - `crates/nova_editor/src/event.rs:2072` and `inspect.rs:2521` - the empty `Names::Order` arm is justified by a reason that is false for `Order`.** Both say an order key is "not a document-wide name", but `lint/scenario.rs:610` collects `declared.order_keys` across the whole document and `:1450` hard-errors on an undeclared key - the same shape as `timer_keys`, which the editor DOES offer completions for. Only `Names::Section` fits the stated reason.

**MINOR - construct counts contradict the tables under them.** `actions.md:6` "All 38 at a glance" over a 42-row table; `events.md:4` "one of the TWENTY-THREE event kinds" over 24; `reference.md:46` "Actions (40)" listing 40 links and missing `SuspendPlayerControl` and `ResumePlayerControl`. The tables themselves were diffed against the enums and are complete. At `origin/master` all three read accurately.

**MINOR - `crates/nova_scenario/src/objects/spaceship.rs:218` - a third byte-identical `fn is_false` inside one crate** (`loader/mod.rs:345`, `objects/ship.rs:100`, this), plus a fourth in `nova_ship`.

**MINOR - `crates/nova_ship/src/flight/order.rs:720` extends the batch-4 finding:** `fail` skips
`retire_ship_order_execution` ENTIRELY, not just the standoff restore, so a failed order also leaves a live
`Autopilot` and any `ScriptedAlign` on the ship with no order driving them. Same one-line fix; noted so the
fix is not scoped to the standoff alone.

**MINOR - `CHANGELOG.md:96` - an intra-cycle fix stands beside the feature that caused it.** The completed-order standoff guarantee fixes `eb3ea6ed`'s own `MoveShipTo`, and 0.12.0 ships no helm orders at all, so the house rule folds it into the six-helm-actions entry rather than recording it.

**Verified in this session:** the `Hold` gate at `trackers.rs:91` and the red test failing on exactly that
assertion while its radius and speed assertions pass; `orbit_band_floor` and `orbit_target_radius`'s clamp,
and the 1 511.3 - 2 514.0 m band against the gate's 1 295.8 - 1 895.8 m span and the stale doc comments;
`AISpaceshipMarker` inserted unconditionally on the AI branch; `ForceTorpedoLaunch` surviving only in
`docs-manifest.js`.

**Checked by the lanes:** every briefed orbit edge - reversal (signed step nets toward zero, correct),
stall on the ring (zero step, correct), well moves (re-seeds, correct), completes twice (`:160` subtracts
TAU rather than latching, but the First Shift handler is `once: true`); `orbit_plane_normal` always
perpendicular to r-hat, and the gate 1.09 degrees off antipodal to TRANSIT 2, so the PLANE is robust and
the radius is the failure axis; every briefed state-machine edge - script installed twice and two scripts
contending (`install_ship_order` cancels first, one terminal report), scripted ship despawns, scenario
reload mid-script, fence installed around a ship already inside; `fail` -> AI-resume cannot double-report;
a full live duel cycle on Xvfb (arena, sever, `OnExit` forfeit, allegiance flip, `ForceTorpedoFire` through
commit, ignition, point-defense assignment, detonation, teardown, carousel advance), confirming the
occupancy rewrite does fire `OnExit` in a real fight; avian 0.7.0 contact-edge body lifetime read from
source; `nova_ship --lib flight::tests::orbit` (3 pass, the 2 known red); First Shift suite 21/21;
`content lint` 0/0/0; generated-RON parity by reading; the `ForceTorpedoLaunch` -> `ForceTorpedoFire` break
having an accurate migration note and no mod naming the old action; `ActionChoice::ALL` 42 and
`FilterChoice::ALL` 7 both current; meters discipline throughout; the wasm clippy ban list; preludes,
plugin and system-set naming, explicit cross-plugin ordering.

**Not checked - named, because a skip is not a pass:**
- `crates/nova_scenario/src/lint/scenario.rs` (+502) and `lint/fixtures.rs` - the largest single hunk in the
  range. Lane A did not read it; lane B checked its MESSAGES and ran `content lint`, but neither reviewed
  the lint LOGIC. Unreviewed.
- `crates/nova_ship/src/sections/railgun_section/scripted.rs` and its tests; `ForceRailgunFire` is
  unexercised in any form.
- `crates/nova_authoring/.../ships/block.rs` (+367) beyond collider population - geometry, mass and section
  validity unread.
- `crates/nova_scenario/benches/scenario_dispatch.rs`.
- `menu_weave`, `menu_gauntlet` and `menu_waystation` beyond what scrolled past in the duel carousel.
- Any timing number. The measurement slot was spent on a live functional run instead, because no range
  covers the surface these commits changed.
- Whether the red `nova_authoring` pacing test is attributable to `68431eb3`. Lane A said plainly that it
  stopped once the orbit chain proved broken upstream and would rather say so than guess.

### Batch 6 - Destruction: debris budget, severed roots, magazine (`8e5043e1` `77f963b1` `c9a23872`)

Both lanes returned. Lane A spent the measurement slot properly: a gated repeat
set, a probe run and a traced pass. No blocker.

**MAJOR - `examples/systems/stress_hull_collapse.rs:865` - claim 3 asserts a per-FRAME budget against a WALL-CLOCK window, and its failure text misdiagnoses.**
The collapse puts 720 pieces into `ChunkGrace` in one command flush (measured: `peak_pending_activation:
720`). `land_carved_chunks` runs in `Update` and lands at most `CHUNK_ACTIVATIONS_PER_FRAME = 24` per
RENDERED frame (`chunk.rs:312`), so draining needs 30 rendered frames. The window is `SETTLE_SECS = 8.0`
seconds of wall clock, the first `CHUNK_GRACE_SECS = 0.5` of which lands nothing. The assert therefore
requires roughly 4 sustained fps, and below that it panics with "a deferred activation was dropped, and
ghost wreckage is wreckage a ship can fly through" - which is false: nothing was dropped, the host was
slow. CI runs this under `xvfb-run` with mesa lavapipe (`.github/workflows/ci.yaml:143,213`) on a scene
peaking at ~15 000 entities with 720 dynamic bodies. Verified by reading. It held 7/7 here with 391-402
window frames against the ~30 needed, a 13x margin - but the coupling is structural. Fix: hold the verdict
on the queue draining (`ChunkGrace` empty, or a frame budget), or say in the panic text that the window is
host-speed dependent and print the frame count.

**MAJOR - `CHANGELOG.md:789` - `8e5043e1` rewrote an entry inside the RELEASED `[0.11.0]` block, and filed nothing in `[Unreleased]` for the change it made.**
Verified: 0.11.0 spans lines 712-1286 and the edited lines are 789-790. The commit replaced "a
capital-grade siege bay with a ship-killing blast and armored ordnance, hidden from the editor gallery"
with "an experimental, deliberately overpowered siege bay with armored Breakers and a six-round rearming
rack". 0.11.0 shipped `ammo_capacity: None` and `hide_in_editor: true`, so a released note now describes
behaviour that release did not have, and drops the fact that the bay was hidden. Meanwhile the real change
- a shipped prototype going from unlimited fire to a 6-round rack reloading +1 per 10 s, and appearing in
the gallery - has NO `[Unreleased]` entry: verified by grepping lines 16-341. A modder who mounted
`heavy_torpedo_section` for its unlimited fire gets a magazine and no release note saying so.
No `**(breaking)**` is owed - `ammo_capacity`/`reload` are pre-existing optional fields, so only a shipped
VALUE moved. `content lint` is clean and the generated RON already carries `Some(6)`.
The lane ranked this BLOCKER; ADJUDICATED DOWN to MAJOR here, because nothing ships broken and no format
breaks - the defect is a falsified released record plus a missing note. Recorded so the downgrade is on
the record.

**MAJOR - `docs/sections.md:749`, `:515`, `:602` (and `web/src/wiki/ships.md:28`) - the dev-book chapter still describes pre-budget behaviour.**
`:749` "Kinetic and Pierce throw 2 to 7 shards of one fixed size" - after `SHARDS_PER_FRAME = 128` a carve
announced once the frame's allowance is spent throws ZERO. `:515` "it takes the same `ChunkGrace`...
kinematic and colliderless until it has drifted clear" - after `CHUNK_ACTIVATIONS_PER_FRAME = 24` a piece
stays kinematic past its grace, about thirty frames for a 700-piece collapse. `:602` "Every section's own
debris burst fires either way" is false in exactly the case the budget exists for, and `ships.md:28`
carries the same claim to players. Neither ceiling is an authored field, so `/create/` is owed nothing.

**MAJOR - `CHANGELOG.md:277` - the shard-budget entry quotes the ablation arm, not the shipped one.**
"a capital hull raked open threw about 10 000 chips and now throws about 2 000". Verified against the
task's own table (`tasks/20260904-173517/TASK.md:119-121`): 10 666 before, 2 099 with the shard budget
ALONE, 4 231 with both changes - which is what ships. Its summary at `:191` says 10 666 -> 4 231, its
review re-measure reads 4 169/4 189, and it warns in as many words that the rise from 2 099 to 4 231 "is
the budget working as specified rather than a regression". Lane A's independent measurement agrees:
peak shards 4 111-4 573 across six runs. Fix: 2 000 -> 4 000.

**MINOR - `crates/nova_gameplay/src/integrity/spew.rs:527` - the budget truncates a crater instead of dropping it, and can mint the single chip `fewest: 2` exists to forbid.**
`let count = look.count(spew.radius).min(budget.left);` - verified. With `budget.left == 1` and any live
look (`fewest: 2`), exactly one chip is minted, against a doc at `:176` that says "Two and not one: a
single chip reads as a stray particle". The module doc at `:54` also states the wrong rule - "the craters
that arrive late in the frame go unchipped" - when exactly one crater per over-budget frame is served a
partial count. Neither the exactly-at-budget nor the partially-served boundary is covered by the two new
tests (200 craters, and one crater). Fix: drop the crater whole when it does not fit, and align the doc.

**MINOR - `examples/systems/stress_hull_collapse.rs:764` - `frames_over_timestep` is saturated by construction.**
`FIXED_TIMESTEP_MS` is 15.625 and a healthy 60 Hz frame is 16.67 ms, so every frame on any 60 Hz host counts
as "over". Measured across seven runs: 94-100% every time, on a box that was never in trouble. The reading
cannot separate a collapse stall from an ordinary vsynced frame.

**MINOR - `crates/nova_authoring/src/base_content/sections/mod.rs:314` - a balance guard now claims to cover the siege bay and cannot.**
Giving the bay `ammo_capacity: Some(6)` moved it out of the `None` skip and into `graded += 1` of
`no_torpedo_bay_out_sustains_a_point_defense_mount`. The grade divides by
`ROUNDS_PER_WEAVING_INTERCEPT = 369.0`, measured against the shipped 10 hp Serpent; the Breaker carries
`projectile_health: 5000.0`, 500x that. So the bay passes at 0.1 launches/s while no mount in the catalog
can intercept even one of its torpedoes. The imbalance is the authored intent; what is wrong is a guard
silently claiming to cover it. The same commit also made the `None` branch and the `hide_in_editor` skip at
`standard.rs:1716` unreachable for the graded catalog, and their comments still name content that no longer
exists.

**MINOR - `crates/nova_gameplay/src/integrity/spew.rs:261` - `SHARDS_PER_FRAME` bounds creation, not the standing population the module doc names as half the problem.**
The doc says the frame "pays for every one at once AND THEN CARRIES them for `SHARD_LIFETIME_SECS`". Measured
peak live shards: 4 111-4 573, roughly 33x the per-frame cap, each a drawn `Mesh3d` with a kinematic body.
In the traced pass `update_temp_entities` is the second-largest nova span in the run (1 018 calls, 63.9 ms
total) - that is the carry. Either cap the live population or trim the doc's second clause.

**MINOR - `web/src/wiki/sections.md:46` and `sections/torpedo-bay.md:141` - nine "Stats verified against" citations, all exact at `8e5043e1`, all stale at HEAD.** Six lines of the drift are this batch's own: `c9a23872` deleted `SIEGE_RAILGUN_LANCE_SECTION_ID` from `standard.rs:42` without touching the citations its sibling commit had just written. `:842-861` now lands on the Twin PDC block. The NUMBERS all still check out against the builder.

**MINOR - `crates/nova_editor/src/gallery/catalog.rs:276` - the gallery fixture is now the only thing in the tree calling the siege bay hidden** ("scene dressing", the framing this batch retired everywhere else). After this batch nothing in `crates/`, `assets/` or `webmods/` sets `hide_in_editor: true` at all, so the one end-to-end check that shipped content honours the flag is gone.

**MINOR - `spew.rs:261` and `chunk.rs:94` - two new per-frame decoration ceilings outside the repo's stated home for that policy.** `settings.rs:254` calls `GraphicsBudget::for_quality` "The one place the tier->cost policy lives". Both new constants are fixed, so `GraphicsQuality::Low` - the tier that turns particles off and drops render scale to 0.7 - still creates 128 chips and lands 24 colliders per frame, the same as High. Tier-scaling is a design call the task deliberately left "Unsettled"; at minimum `settings.rs` should stop saying "the one place".

**MINOR - `examples/systems/stress_hull_collapse.rs:790` reads the live fixed timestep for its guard and hardcodes it for its record;** `:825` publishes `"rake_radius": 3.0` in world units under a key an authored field owns at 30.0 meters.

**MINOR - `crates/nova_ship/src/sections/integrity.rs:192` - the new doc carries a task's measurement log into permanent API docs** ("92.5 ms over a 33 s run, 36.4 ms inside one 203 ms collapse frame"), one traced dev build on one host under Xvfb, which `docs/performance.md:210` says not to quote. The constraint half - the quadratic walk - belongs and does not rot.

**CORRECTION to a batch-4 finding.** The `crates/nova_gameplay/src/integrity/core.rs:132` destruction-tally
allocation was recorded as an unmeasured MINOR. It is now measured, on the largest collapse in the catalog:
`tally_a_destroyed_node` 720 calls, 0.36 ms TOTAL, worst call 0.007 ms; `report_destruction_tally` 509
calls, 0.95 ms total. It is not a frame cost. Fold it in on style, or not at all.

**Measurement, recorded because the batch makes a frame-cost claim.**
Host gated: 24-core box, no user game process, one-minute load 0.19-1.62 sampled beside each run, all well
under 3.5. Six runs of `stress_hull_collapse` under `NOVA_AUTOPILOT`, plus one `probe run` and one traced
pass. `docs/performance.md` read first; no threshold applied and nothing below is a graded number.

| reading | values | median |
|-|-|-|
| `worst_frame_ms` | 66.6 / 80.4 / 100.5 / 84.6 / 72.0 / 70.9 | 76.2 ms |
| `worst_frame_fixed_steps` | 5 / 5 / 6 / 6 / 5 / 5 | 5 |
| `window_frames` | 391-402 (~49 fps) | 395 |
| `peak_shards` | 4 111-4 573 | ~4 175 |
| `peak_wreck_pieces` | 720 every run | 720 |
| `pending_activation` at verdict | 0 every run | 0 |

`probe run stress_hull_collapse`: verdict OK, measured 6/8, all five markers on the roster,
`worst_frame_ms: 90.48`, `peak_entities: 14 960`, `still_kinematic: 0`.

Traced ranking (tracing inflates absolutes; this ranks):
`advance_rounds` 1 504 calls, worst single call **51.4 ms** - the largest nova span in the run, landing on
the rake frame; `build_ship_integrity_graph` worst 25.5 ms (the spawn frame, not the collapse);
`land_carved_chunks` worst 1.10 ms; `spew_carved_material` worst 0.025 ms.

So the two systems `c9a23872` budgets cost 1.10 ms and 0.025 ms on their worst calls. The budgets are cheap
and they work - and they are NOT what makes the collapse frame expensive. That sits in `advance_rounds` and
in the standing-debris carry. `77f963b1` is confirmed in direction and magnitude:
`queue_depleted_section_sever` is 720 invocations for 0.45 ms total.

**Not checked, and it matters:** there is NO before arm. The batch's central claim - that the budgets moved
the collapse frame - is unmeasured here; the 76 ms median is an after reading with no named reference beside
it. The range declares `without_frametime()`, so `fps_within_baseline` and `capture_simulated` report
`N/A - not claimed`: unmeasured, not passed. No run under lavapipe or on a 2-4 vCPU host, which is the
condition the first finding is about - the 4 fps floor is arithmetic plus this host's 49 fps, not an
observed failure. No `--samply` flamegraph, so command-flush cost is not attributed to the observers that
queued it. The editor gallery was not exercised with the bay now visible. Lane B ran no build, test or
example at all, so the new unit tests and the range are read-only assessments on that side.

### Batch 7 - Campaign replacement: First Shift and Second Shift (`32d00dfe`)

Lane B (craft + contracts) returned first; lane A (correctness + performance) is
still running and appends below. Every claim below was re-checked against HEAD in
the main session, because this commit's files were later MOVED
(`first_shift.rs` is now `first_shift/{mod,marks,story,tests}.rs`) and some of its
defects were repaired by later batches.

**MAJOR - `crates/nova_os/src/commands.rs:229` - the in-game console still tells a player to type a scenario id this commit deleted.**
`examples: &["scenario load shakedown_run"]`, and `CommandSpec::examples` is
documented at `:162` as "Worked examples printed by `help <command>`". So
`help scenario load` hands the player a command that fails. Confirmed at HEAD.
The commit updated the wiki's Commands page and the console's own `world` line
fixtures and missed the one string a player is told to type. `:915`'s rustdoc
carries the same dead id in its worked example. Fix: `first_shift`.

**MAJOR - `crates/nova_authoring/src/generation.rs:52` - public rustdoc on `build_campaigns` still describes the deleted five-chapter campaign.**
Confirmed at HEAD: "the three visible chapter-heads plus the two `hidden` chained
members (broadside_gunship, the phase-two wave; final_tally, the epilogue)". The
campaign has exactly TWO members now, and this commit's own
`campaign_membership.rs:37` asserts `vec!["first_shift", "second_shift"]` with
`assert!(!outcome.scenarios[member].hidden, ...)` - the direct opposite. The commit
rewrote the identical wording in the test's docstring and left the public one.

**MAJOR - `crates/nova_authoring/.../nova_protocol/tests.rs:202` - the regression guard for this commit's own bug is blind to the only case it needs to see.**
`no_scenario_starts_the_player_inside_one_of_its_own_trigger_volumes` exists
because, in the commit author's own words, "a player spawn inside the approach
trigger (the arrival completed an objective before the opening posted it)" was one
of four content bugs found. Its docstring claims "Every volume a scenario spawns in
its own OnStart frame". Its body iterates `event.actions.iter()` at the TOP LEVEL
only, twice.
Verified by bracket-matching the generated RON: First Shift's OnStart holds 68
`SpawnScenarioObject` actions, and the ONLY one carrying `area_radius: Some` is
`work_mark` (300 m), nested at depth SIX inside a `Sequence`. The guard never sees
it. The content is fine - the cutter spawns at (-1100, 0, 2500) and `work_mark` sits
at (-500, 80, 900), 1711 m away - so this is a guard that does not guard, not a live
bug. `EventActionConfig::walk` (`crates/nova_scenario/src/actions/mod.rs:291`)
exists for exactly this, its own docstring says "A second nesting arm therefore
cannot be honoured by one walker and quietly missed by the others", and this
commit's own `all_actions` helper uses it. Fix: use `walk` in both passes.

**MAJOR - live coverage of a chapter's beat sequence is gone and nothing replaced it.**
Deleted: `nova_assets/tests/final_tally_claim.rs`, `lifeline_convoy.rs`,
`nova_authoring/tests/broadside_assault.rs`, and `shakedown/tests/{pins,walk}.rs`.
Those booted a Bevy slice and walked outcomes.
ADJUDICATED DOWN from the lane's framing: the lane reported that NOTHING boots an
app for the new chapters. That is wrong at HEAD.
`crates/nova_assets/tests/neutralized_ships.rs` does boot a slice (`slice_app`,
`MinimalPlugins`) over the generated RON for both chapters and covers the exact
invariant the lane named as the sharpest gap -
`a_first_shift_player_neutralize_is_a_gated_terminal_defeat`,
`a_second_shift_player_neutralize_is_a_gated_terminal_defeat` and
`nothing_but_the_player_can_end_the_second_shift`.
What genuinely has NO successor is the beat WALK: `git grep` for
`walk_end_to_end`, `beat_gated` and `retries_the_current_part` returns nothing in
`crates/`. The 30-odd new tests in `first_shift/tests.rs` are static assertions over
the built `ScenarioConfig` - geometry, ordering, corridor clearance, control
hand-back - and none advances a frame. So "does the chapter's beat chain actually
run from beat 0 to the outro" is now unproven by any test, and only lane A's live
runs cover it. Fix: port one end-to-end walk onto `first_shift`.

**MINOR - four rustdoc sites still name deleted content, all confirmed at HEAD.**
`crates/nova_scenario/src/actions/ship.rs:13` ("the shakedown training governor
releases at beacon 1") and `:109` ("the shakedown withholds GOTO until the first
objective is complete"); `crates/nova_mod_format/src/lib.rs:91`
(`"scenarios/shakedown_run.content.ron"`) and `:113`
(`new_game_scenario: Some("shakedown_run")`). Each has a web twin that this commit
DID update, so the sources and the pages that mirror them now disagree.
`ship.rs:109` additionally says "Flight verbs (STOP/GOTO/ORBIT)" while `FlightVerb`
has six variants and First Shift withholds `Rcs` and `Lock` through this very
action; `web/src/create/actions.md:564` lists all six correctly.

**MINOR - comments that record a deleted thing's history instead of a constraint.**
Confirmed at HEAD: `nova_protocol/stage.rs:41` "the shakedown's proven tutorial
well"; `nova_protocol/tests.rs:106` "The end-to-end timing itself is proven in
shakedown's walk tests" - which now points at tests this same commit deleted, and
which is the load-bearing excuse for the coverage gap above;
`crates/nova_authoring/src/balance.rs:31` cites `broadside_assault.rs`, deleted
here. Also `stage.rs:11` "Layout provenance: ... the spatial benches this stage was
reviewed in", against "Keep module comments short. Explain ownership and
constraints, not code or history." Keep the constraints (the 3.29 km SOI, the
950 m mean-radius reasoning at `:26` which is genuinely load-bearing) and drop the
dead proper nouns.

**MINOR - `web/src/create/actions.md:391` - a `StoryMessage` example still uses `speaker: "Capt. Halloran"`, a cast constant this commit deleted.** Confirmed at HEAD.

**MINOR - `scripts/gen-web-screenshots.py:123` - orphaned capture target.**
`("tutorial-combat-lock.png", "screenshot_combat_lock")` survives, but the commit
deleted the only figure that referenced it. Confirmed: nothing in
`web/src/wiki/getting-started.md` mentions `combat-lock` at HEAD.

**MINOR - `examples/playable/shared/first_shift.rs:174` uses bare `#[allow(dead_code, reason = ...)]`** where `AGENTS.md` requires `#[expect]`. The lane flagged the tension honestly and it is real: `#[expect]` would fire an unfulfilled-expectation warning in whichever bench DOES use `carrier_wreck_fragments`, so `allow` is the only thing that compiles as written. The rule is not wrong; the module shape is. Fix by splitting the shared module, not by swapping the attribute.

**MINOR - `web/src/wiki/flight-autopilot.md:80` states a conditional behaviour unconditionally.**
"**STOP** brakes entirely on RCS without turning the ship whenever it is moving
below 100 m/s." The 100 m/s is right (`flight/state.rs`, `rcs_speed_cap`), but the
behaviour needs the `Rcs` verb - `nova_ship/src/flight/tests/rcs.rs:443` is literally
`stop_terminal_without_rcs_verb_uses_the_main_drive` - and First Shift spawns the
player with `DisableVerb(FlightVerb::Rcs)`.
ADJUDICATED DOWN from the lane's ranking: the page itself says two paragraphs later
that "RCS is a controller verb granted per ship ... and the mainline campaign flies
with it **withheld**". The reader is told; the sentence inside the disclosure just
does not carry the caveat locally. Add four words to it.

**DROPPED - `web/src/create/ships.md` base-ships table.** The lane ranked this MAJOR
after reading the tree at `32d00dfe`, where the table listed five modelled hulls and
none of the eleven block ships this commit added. At HEAD the table is correct: all
fifteen block hulls including the four wreck fragments, with `racer`/`cargoa` moved
below it and correctly framed as The Ledger's modelled craft. A later batch fixed it.
Recorded so the drop is on the record, and as evidence the "HEAD is the truth" rule
in this run's ground rules is earning its place.

**Verified clean by lane B, spot-checked here.** `CHANGELOG.md`: no hunk lands in a
released block (the commit's hunks are at 16/46/91/158, `[0.12.0]` starts at 244 in
the post-commit file), all `[Unreleased]` entries are within 200 characters joined,
the removal is correctly marked `**(breaking)**`, and its migration note is
legitimate because the five removed scenarios genuinely shipped in 0.8.0-0.10.0.
`web/src/widgets.ts`: all four constant edits match their cited Rust, and the two
derived SOI figures are arithmetically right. No world-unit leak in any objective,
comms line or wiki figure the commit touched. `webmods/` needed no migration - the
example mod names the removed scenarios only in provenance comments, never as an id
or a `NextScenario` target. The rewritten `getting-started.md` matches the builder
beat for beat.

**Not checked on the craft/contracts side.** Lane B ran NOTHING - no build, no test,
no example; every finding is a reading. Generated RON was read by me, not the lane.
The new block hulls (`ships/block.rs`, +524) were checked for docstring accuracy but
not for cell connectivity or interpenetration. `neutralized_ships.rs` (296 lines
changed) and `mod_cache_install.rs` were skimmed at the diff-header level only.
Second Shift's patrol and detection LOGIC was deliberately left to lane A.

#### Batch 7, lane A (correctness + performance)

**Attribution settled: the red pacing test is BATCH 10's, not this batch's.**
`no_mainline_handler_posts_an_objective_alongside_a_conversation` was on the plan
as a question for batches 8-10. It is answered. Run at HEAD in this session:
"first_shift: handler #0 (OnStart) posts an objective in the same frame as a comms
line". The offending action pair is at `first_shift/mod.rs:617-627` -
`post_objective(OBJ_BURN, ...)` sharing one step's action list with
`story_message(COPILOT, story::OPEN_COPILOT_MARK)`. `git log -S OPEN_COPILOT_MARK`
returns exactly one commit: **`f9533456` "Rewrite the First Shift departure
briefing"**. The lane additionally parsed the generated RON at every commit in the
range and counted groups holding both a `StoryMessage` and an `Objective`: 0 at
`32d00dfe` through `ef6320ba`, 1 at `f9533456`, 2 at `e2a1eb45`, 2 at HEAD. Batch 7's
content passed this test. Batches 8 and 9 are cleared of it too. Fix it under
batch 10.

**BLOCKER - `.../nova_protocol/second_shift.rs:130-185` - all five Second Shift searchers fly two of their three lane legs THROUGH the concealment planetoid, every lap.**
Re-computed independently in the main session; my numbers match the lane's to the
metre. Body: `CONCEALMENT_POS (4500, 300, -6500)`, body radius **2377 m** - not a
seed accident, it is pinned by `nova_protocol/tests.rs:276`
(`the_belt_planets_keep_the_body_radius_their_rocks_published`).

| searcher | spawn | leg 0->1 | leg 1->2 | leg 2->0 |
|-|-|-|-|-|
| cleanup_skiff | 3228 m | 1767 m, **610 m inside** | 3277 m clear | 1755 m, **622 m inside** |
| cleanup_tug | 3585 m | 2228 m, **149 m inside** | 4411 m clear | 2231 m, **146 m inside** |
| cleanup_picket | 3324 m | 1955 m, **422 m inside** | 3759 m clear | 990 m, **1387 m inside** |
| cleanup_claw | 3772 m | 2297 m, **80 m inside** | 5798 m clear | 2131 m, **246 m inside** |
| cleanup_leader | 4904 m | 1999 m, **378 m inside** | 3551 m clear | 2184 m, **193 m inside** |

Every searcher spawns clear of the body and then flies into it. The planet is solid
and massed: `crates/nova_scenario/src/objects/planet.rs:114` gives it
`Collider::sphere(1.0 + relief)` with `ColliderDensity(1.0)`, it is `invulnerable`
with a gravity well, and `crates/nova_ship/src/flight/state.rs:177` states plainly
"there is no collision avoidance". `PatrolShip` is documented as "the ship visits
every waypoint in order and then returns to the first", so leg 2->0 is flown on every
lap and the handler sends each searcher round again indefinitely.
Ranked BLOCKER rather than MAJOR: this is a shipped chapter whose central mechanic is
five ships sweeping lanes, and all five drive into a body they cannot pass on the
first leg out of spawn. Its own pin cannot see it -
`second_shift.rs:828 no_sweep_mark_sits_inside_something_solid` iterates
`for mark in searcher.route`, endpoints only, never the segments between them.
Fix: move `route[0]` and the first and last legs clear of
`concealment_body_radius()` plus the 100 m `HULL_PAD` the pin already uses, and
extend that pin to test segments. `first_shift/tests.rs:740`
(`the_warship_never_flies_a_leg_through_a_body`) is the segment test already in the
tree - the searchers need the same one.

**MAJOR - `crates/nova_scenario/src/world.rs:617` - a sequence step with `after: 0.0` runs in the SAME FRAME as the step before it, and the pacing rule's own grouping model assumes it cannot.**
Verified by reading. `advance_scenario_sequences` (`actions/sequence.rs:159`) drains
in `while let Some(actions) = world.take_ready_sequence_step(now)`, and
`take_ready_sequence_step` sets `run.since = now` when it hands a step back - so the
next iteration, at the same `now`, computes `waited = 0` and `after: Some(0.0)`
passes `waited >= after` immediately. Both steps run inside one system call.
`ScenarioEventConfig::action_groups` (`loader/mod.rs:436`) is built on the opposite
assumption, in as many words: "A step is a frame of its own - it lands seconds after
the handler that queued it - so every rule phrased 'in the same frame' ... has to
read groups rather than the handler's own list." So the pacing guard treats each step
as a separate frame and cannot see a `0.0` step glued to the one before it.
This is live at HEAD in Second Shift: `second_shift.rs:527-536` ends its opening
conversation with three `open_line`s and then `step(0.0, [post_objective(OBJ_APPROACH,
"Fly in to the wreck field."), attach_objective_marker(...)])` - the objective posts in
the same frame as the last comms line, which is exactly what
`no_mainline_handler_posts_an_objective_alongside_a_conversation` exists to forbid,
and it passes. Fix both halves: give the opening objective a real gap, and teach
`action_groups` that an `after: None`/`after: Some(0.0)` step is continuous with its
predecessor rather than a frame of its own.

**MINOR - `second_shift.rs:383` - the detection beat can outlive the win.**
`detected()` starts `pacing::beat_later("detected", REVEAL_GAP /* 8.4 s */, [...
post_objective(OBJ_ESCAPE, "They have you. Run for the extraction point.")])`.
`pacing::open_outro` does not stop running sequences and the sequence cursor is
beat-agnostic, so reaching the extraction volume within 8.4 s of being seen re-posts
"Run for the extraction point" on top of the victory epilogue. Read, not run.
Fix: gate that step on `beat == 3`, or move the re-post onto the handler that owns
`VAR_SEEN`.

**Two defects the lane found at `32d00dfe` that HEAD has already fixed.** Recorded
because they show the range repairing itself, and because both fixes are the pattern
the live BLOCKER above still needs.
1. The warship's exit leg `WARSHIP_FIRING_POS -> WARSHIP_EXIT_POS` passed 1936 m from
   the concealment centre, 441 m inside the body, on the "burns away unbothered" beat.
   A later commit inserted `WARSHIP_EMERGE_POS`, moved the exit to (12000, 2000, 1000),
   and added `the_warship_never_flies_a_leg_through_a_body`.
2. First Shift beat 4 could complete and consume itself before its objective existed:
   `OnTravelLockStart` created `approach_ring()` at once while `beat_setup` posted
   `OBJ_APPROACH` 4 s later, and an area spawned around a body already inside it DOES
   fire `OnEnter` (pinned at `objects/area.rs:248`). A player who flew to the survey
   body before pressing lock burned the `once: true` handler on a nonexistent
   objective and was left with a permanently stale goal. HEAD moved `approach_ring()`
   into the same step as the post.
3. Also fixed: `OBJ_RETURN` read "Return to the {CARRIER_NAME}." but completed on
   `OnExit(ID_APPROACH_RING)`, 7335 m short of the carrier. HEAD renamed it to
   "Lock and GOTO back to the work site.", which matches the trigger.

**Refinement of the already-recorded orbit finding (not a new defect).** First Shift's
ORBIT beat hangs on exactly one edge: `OnOrbitStable` -> `TimerStart(orbit_hold, 5 s)`
-> `OnTimerEnd`, and `OnOrbitStable` fires only on the transition INTO
`AutopilotPhase::Hold`. The RCS retune does not move the `Hold` predicate; it moves
the actuator. At the inspection well the ring sits at 1.5 x (1000.1 m + 10 m) and
`orbit_gravity_accel = mu / r^2 = 27000 / 151.5^2 = 1.18` engine units. The authority
threshold moved 0.75 -> 2.4525, so 1.18 crossed from above it to below it: the beat
changed hands from the main drive to the RCS trim, which is the actuator the two
failing `nova_ship` orbit tests say cannot hold the ring. Arithmetic from reading.

**MAJOR (coverage) - the removal dropped 45 test functions and replaced them with 18 structural pins.**
Deleted: `final_tally_claim.rs` (7), `lifeline_convoy.rs` (7), `broadside_assault.rs`
(14), `shakedown/tests/pins.rs` (8), `walk.rs` (9). Added: 7 first_shift, 6
second_shift, 5 shared. Two gaps are load-bearing, and they line up with lane B's
finding rather than duplicating it:
- **No behavioural walk.** Neither chapter has a test that advances a frame. The
  BLOCKER above, the `after: 0.0` MAJOR, the beat-4 self-consumption and the orbit
  soft-lock are all things a walk would have caught.
- **No segment-clearance check survived.** `pins.rs` carried `segment_distance_to_box`
  and `belt_knots_keep_every_beat_pocket_clear`. Everything geometric in the new
  content checks POINTS only. The BLOCKER is the direct consequence, and a later
  commit had to re-invent the segment test for the warship alone.
- Nothing shipped now exercises `hidden: true`, nor the "neutralize completes a kill
  objective" contract - the campaign has no kill objectives.

**Frame cost: not measured, and deliberately so.** The lane held the measurement slot
and declined it, correctly: neither chapter has a probe subject with a frametime
capture (the `playable/` map benches declare `without_frametime()`), no `probe-runs/`
entry exists for any campaign scene, and a defensible number needs a gated repeat set.
Unmeasured context only, not a claim: `block_carrier` is 2081 section entries,
`block_warship` 341; First Shift spawns 81 objects at OnStart, Second Shift 104
objects and 34 ships, against the deleted shakedown's 23 objects and 3 ships.
Measuring this needs a probe subject wired for the campaign scenes first.

**Not checked on the correctness side.** Neither chapter was driven in a live app;
findings above are read-and-compute except the one test run. The `armed: false`
premise (that an unarmed searcher can never acquire) was not chased. Whether the 28
wreck fragments foul the searcher lanes was not computed, since the lanes are already
broken on the planetoid. The spawn-then-order-in-one-frame pattern looked safe on
reading but was not confirmed in a running app.

### Batch 8 - Campaign pacing and cinematic reliability (`735eea53` `f77db4cc`)

Lane B (craft + contracts) returned first and ran tests this time; lane A is still
running and appends below. Every claim was re-checked at HEAD in the main session,
and every number below was recomputed here.

**MAJOR - `crates/nova_scenario/src/lint/scenario.rs:1047` - the two new camera actions never reach the lint, so an unrecognized object id in `SetCameraAnchor` is a silent warn-only no-op.**
Verified: `grep -rn 'SetCameraAnchor' crates/nova_scenario/src/lint/` returns NOTHING,
and `check_action` ends in `_ => {}` at `:1047`. Every other scoped-id action goes
through `check_target` (`DespawnScenarioObject:801`, `SetSpeedCap:810`,
`ObjectiveMarkerAttach:783`). The field is even tagged `#[reflect(@Names::Object)]`
(`actions/view.rs:96`), and `names.rs:46` states the contract: "Every one of these has
to resolve against the scenario's own spawns or the handler fires at nothing."
`CameraLookAtConfig::Object(String)` is unchecked and untagged. This is the house rule
"a missing required field or an unrecognized id is an error at lint, then at load"
being broken for a brand-new creator-facing action.
The lane confirmed the shipped behaviour by RUNNING
`cargo test -p nova_scenario --lib actions::view::`:
`set_camera_anchor_on_a_missing_object_leaves_the_camera_alone` passes, 17/17.
Aggravating: `crates/nova_editor/src/event.rs:989` seeds the action with
`anchor: String::new()`, so an unfinished action saved from the editor also passes
lint. Fix: add a `SetCameraAnchor` arm calling `check_target` on `config.anchor` and
on `CameraLookAtConfig::Object(id)`.

**MAJOR - `web/src/create/actions.md:6` - the creator reference states a count four short of its own table.**
"All 38 at a glance:". Counted here: the table has 44 lines including header and
separator, so 42 data rows, and `EventActionConfig` has exactly 42 variants
(`actions/mod.rs:61`). `735eea53` added the `SetCameraAnchor`/`ReleaseCamera` rows and
`f77db4cc` added `SuspendPlayerControl`/`ResumePlayerControl`; neither moved the 38.

**MAJOR - `web/src/create/reference.md:46,:112,:150,:155` - the construct catalog says `Actions (40)`, lists 40, and its A-Z index is missing four constructs these commits added.**
Verified at HEAD: the view group ends "SetCamera, SetCameraAnchor, ReleaseCamera,
Screenshot, SetSkybox" with no `SuspendPlayerControl` or `ResumePlayerControl`, and
`grep` finds neither anywhere in the file. The R block reads "RefillAmmo,
ReleaseCamera, Rename, Ring"; the S block "StopShip, StoryMessage, String, Style,
Subtract". The O block omits `OnGotoComplete`/`OnStopComplete` even though the
`Events (24)` count table at `:44` DOES list both - so the same commit updated one
half of the page and not the other. `:52` promises "The wiki search (sidebar) indexes
every construct name."

**MAJOR - `web/src/docs-manifest.js:717` - the two new control actions are absent from the search index.**
`web/src/docs.ts:65` builds the site-search haystack from exactly this `headings`
field. `735eea53` kept it in sync and added `"ReleaseCamera"`; `f77db4cc` did not.
Confirmed: `ReleaseCamera` is present at `:717`, `SuspendPlayerControl` and
`ResumePlayerControl` are absent from the file. A modder searching the wiki for
`SuspendPlayerControl` gets nothing, though `actions.md` has a full section for it.
Not attributed here but noted: the same entry's summary at `:687` still says "All 28
actions" and its headings still list a nonexistent `ForceTorpedoLaunch` - both predate
this batch.

**MAJOR - `web/src/create/base-content.md:227` - creator docs assign a skybox to a chapter that no longer uses it.**
"`textures/cubemap.png` - the stock skybox (chapter 1)" / "`cubemap_alt.png` - the
alternate skybox (chapter 2)". `735eea53` deliberately put both chapters under one
sky, and `scenarios/mod.rs:25-26` confirms it at HEAD: `first_shift(cubemap(), ...)`
and `second_shift(cubemap(), ...)`. The commit deleted `BaseContentAssets::cubemap_alt`
outright. `cubemap_alt.png` still ships as the `SetSkybox` swap target and the editor
sandbox load, so the fix is to restate it as that, not to delete the line.

**MINOR - `docs/scenario-system.md:173` - "Five are not" above six bullets.**
`735eea53` correctly bumped Four -> Five when it added the `SetCameraAnchor` bullet;
`f77db4cc` added the `SuspendPlayerControl` bullet without bumping Five -> Six.

**MINOR - `web/src/widgets.ts:254-260` - seven `standard.rs:NNN` citations went stale in the lance refactor, and two now land inside the SIEGE gun's block.**
Recomputed here, every one confirmed:

| widget constant | cites | what is actually there | the real line |
|-|-|-|-|
| `LANCE_CHARGE_SECONDS = 1.5` | `:920` | `rake_radius: Meters(10.0)` | `:1150` |
| `LANCE_SLUG_SPEED = 15000` | `:1157` | a comment | `:1151` |
| `LANCE_SLUG_DAMAGE = 300` | `:932` | siege description text (`:937` is siege `slug_damage: 500.0`) | `:908` |
| `LANCE_SLUG_POWER = 1800` | `:938` | a siege comment (`:942` is siege `slug_power: 360_000.0`) | `:914` |
| `LANCE_RAKE_RADIUS = 10` | `:957` | siege TORPEDO description | `:920` |
| `LANCE_SLUG_LIFETIME = 1.2` | `:960` | `health: TORPEDO_BASE_HEALTH` | `:1161` |
| `LANCE_RELOAD_DELAY = 12` | `:976` | `spawn_offset: ...` | `:1177` |

The rendered VALUES are all still correct - the lane confirmed by running
`the_siege_lance_is_a_second_gun_and_not_a_heavier_first_one`, which pins the standard
lance at exactly `(300.0, 1_800.0, Some(Meters(10.0)))`. The danger is the two
citations pointing into the siege block, where a reader checking the number finds 500
and 360 000 instead of 300 and 1800.
Related and CLEAN: the commit's claim "so every other railgun keeps its standard
values" holds - the siege gun is a second catalog prototype off a shared builder, no
prototype was mutated. Verified by that same test.

**MINOR - `.../first_shift/marks.rs:218` - one of the hold mark's four cited measurements does not reproduce, and nothing pins any of them.**
"3.27 km clear of the nearest rock, because the player is meant to be watching the
sky." Recomputed here over all 60 rocks in `stage.rs` (40 `SALVAGE_ROCKS` + 20
`AMBIENT_ROCKS`) against `HOME_MARK.position (2000, -600, 2400)`: the nearest centre
is **3560.3 m** (a salvage rock at (1000, -260, -1000), authored radius 22 m). Worst-case
surface clearance is 3428-3483 m depending on the geometric factor, and from the mark's
own 700 m edge, 2728-2783 m. No reading yields 3.27 km, and the figure was already
wrong when written. The neighbouring two figures check out exactly: 3.06 km to the
Meridian and 2.15 km to the torpedo lane. `first_shift/tests.rs:611
the_hold_frames_the_set_piece_without_standing_in_it` asserts ranges and invariants,
never these numbers, so the docstring drifts silently.

**MINOR - `.../first_shift/mod.rs:1` - a 38-line module comment that narrates the script the code spells out.**
Against "Keep module comments short. Explain ownership and constraints, not code or
history." The beat-by-beat retelling ("it comes out from behind the large planetoid in
two legs, turns its whole hull onto the carrier, walks six siege torpedoes out of its
bays...") is the code below. The last paragraph - the `story`/`marks` split and the
one-`beat`-counter convention - is the part that earns its place.

**Verified clean.** Neither commit touched a released `CHANGELOG.md` block (all hunks
land at or below line 141; `[0.12.0]` began at `:250` and `:281` in the respective
post-commit files). The four new-modding-surface entries are correct: right subsystem,
under 200 characters, meters throughout.

#### Batch 8, lane A (correctness + performance)

The lane spent the measurement slot properly: it RAN two of the per-beat benches
headless and timed them. Claims re-verified here.

**MAJOR - `.../first_shift/mod.rs:650` and `:665` - the STOP lesson can complete before its objective exists, orphaning the card and the hint for the rest of the chapter.**
Verified by reading at HEAD. `cutter()`'s `controller_gate` (`:340-343`) withholds
exactly four verbs - `Rcs`, `Lock`, `Goto`, `Orbit`. **`Stop` is not among them**, so
it works from frame one. The WORK MARK arrival handler sets `VAR_BEAT` to `BEAT_STOP`
IMMEDIATELY and defers the objective by `INSTRUCTION_GAP` (4.0 s):

    set_variable(VAR_BEAT, number(BEAT_STOP)),
    ...
    beat_setup(BEAT_STOP, INSTRUCTION_GAP,
        vec![post_objective(OBJ_STOP, ...), show_hint_emphasis("STOP")])

while the beat is closed by a raw maneuver edge that is armed the moment `VAR_BEAT`
flips: `OnStopComplete`, `once: true`, `filters: [entity(ID_CUTTER),
number_equals(VAR_BEAT, BEAT_STOP)]`. So for four seconds the completion handler is
live and the objective does not exist. `WORK_MARK.area` is 300 m and the copilot's
"Bring us to a full stop first" plays in the crossing frame, so a player who is
already braking - or who taps STOP on the line - settles well inside the window;
`stop_speed_epsilon` is 2 m/s, so a low-speed drift completes almost at once.
`NovaEventWorld::remove_objective` (`world.rs:489`) only warns on a missing id, so the
early completion is a silent no-op, the `once` handler is spent, and 4 s later the
card and a pulsing STOP hint post with nothing left to clear them. Not a progression
soft-lock - the RCS lesson still opens - but a dead objective and a pulsing hint ride
the panel beside every later objective until teardown.
The author already guards this exact hazard one beat later: the GOTO beat at `:881`
puts `grant(FlightVerb::Goto)` INSIDE the same delayed step that posts its card. STOP
is the one taught verb never withheld. Introduced by `f77db4cc`.
Fix: add `DisableVerb(FlightVerb::Stop)` to `controller_gate` and `grant(FlightVerb::Stop)`
to the `beat_setup(BEAT_STOP, ...)` step, exactly as GOTO is handled -
`every_withheld_control_is_handed_back` then covers it for free.

**MINOR - `crates/nova_scenario/src/actions/view.rs:216` - `ReleaseCamera` cannot deliver half of what its docstring promises.**
Verified at HEAD. `SetCameraAnchor` does `entity.remove::<WASDCameraController>()`
(`:170`) and `ReleaseCameraActionConfig::action` removes only `ScriptedCameraPose` and
`ScriptedCameraAnchor` - nothing puts the free-fly rig back. The only re-insert is
`loader/lifecycle.rs:531 on_player_spaceship_destroyed`, which needs a player ship to
have existed. The docstring says: "Hand the scenario camera back to whatever rig owns
it - the player's chase camera during a run, **the free-fly rig without a player
ship**." In a shipless authored scene an anchor/release pair leaves a camera with no
controller at all, frozen at the last enforced pose. No First Shift path reaches it -
the cutter is always present, so the WASD removal is a no-op - but a headless, probe,
editor or shipless scene does. The existing test
`release_camera_takes_both_overrides_off` asserts the removals and not the restore.
Fix: re-insert `WASDCameraController` when the camera carries no
`SpaceshipCameraController`, or drop the free-fly half of the promise.

**MINOR - `crates/nova_authoring/src/base_content/sections/standard.rs:1133` - the siege lance ships in the player's build drawer, contradicting its own description.**
`railgun_lance_prototype` hard-codes `hide_in_editor: false` for BOTH grades, so
`siege_railgun_lance_section` - 500 damage, 360 000 pierce power, a bore three times as
wide - is placeable on any player-built hull. Its own description string at `:933`
says "Deliberately overpowered siege ordnance for a scripted capital, not a balanced
duel." Verified: `crates/nova_editor/src/gallery/catalog.rs:87` filters on
`!section.base.hide_in_editor`, and the generated `base.content.ron` carries ZERO
`hide_in_editor` keys. `only_the_stolen_warship_mounts_the_siege_lance` pins the
fleet, not the drawer.
Correcting the lane on one point: it cited a comment at `standard.rs:42` saying "this
one is only ever mounted by authored content". That comment is not at HEAD - batch 6
already recorded that `c9a23872` deleted `SIEGE_RAILGUN_LANCE_SECTION_ID` from `:42`.
The description string carries the intent instead, and the finding stands on it.
**This is now a PATTERN across the range.** Batch 6 recorded the siege torpedo bay
leaving `hide_in_editor: true`, and noted that afterwards nothing in `crates/`,
`assets/` or `webmods/` sets the flag at all. Both deliberately-overpowered
scripted-content prototypes are now in the player's drawer, and the one end-to-end
check that shipped content honours the flag went with them. Fix both together: add
`hide_in_editor` to `LanceSpec`, set it true for the siege grade, restore it on the
bay, and keep a fixture that proves the drawer honours it.

**MINOR - `.../first_shift/mod.rs:22` - "every stage hangs off the previous one's completion event" is true of the approach and false of the salvo.**
MEASURED. The lane ran `first_shift_08_attack_salvo` headless on a quiet host
(loadavg 0.12, pre-existing Xvfb). Authored gap -> observed wall gap: bays 1.0 ->
1.09/1.17/1.18/1.22/1.17; cut-to-carrier 1.0 -> 1.26; lances 0.5 -> 0.71;
cut-to-cutter 4.0 -> 5.70; exit 14.0 -> 17.68; aftermath 4.0 -> 5.06. A 17-42%
wall-clock stretch, because `loader/clock.rs:16 tick_scenario_clock` advances on
virtual `Time` with a delta clamp that llvmpipe frames exceed.
**No behavioural defect follows and the lane did not claim one:** FixedUpdate physics
consumes the same clamped delta, so the sequence and the ordnance stretch together;
the carrier died 11 s of scenario time into a 14 s exit gap and every cut landed in
order. `take_ready_sequence_step` also resets `run.since` at fire time, so overshoot
does not accumulate. The defect is the doc claiming a mechanism the salvo half does
not use. Fix: scope the sentence to the approach.

**Verified with no defect found, and worth keeping.**
- The approach chain IS genuinely completion-event driven. Ran
  `first_shift_07_attack_approach`: emerge 34.8 s, approach 33.0 s, align 6.3 s
  (settled 0.0244 rad against a 0.0349 rad tolerance) = 74.1 s against the authored
  34/33/6.
- The `SetCameraAnchor` pose->anchor swap does not blink, though the production order
  is the REVERSE of what `loader/camera.rs:561` tests. It is still correct because
  `EntityWorldMut::remove_with_caller` flushes, so the release observer's deferred
  `try_remove::<ScriptedCameraTransform>()` lands before the insert and `#[require]`
  re-adds it. The test simply does not cover the shipped order.
- `once` handlers are not consumed by an early trigger anywhere ELSE: `engine.rs:476`
  retires only after the filter passes, and every `beat_setup` beat except OBJ_STOP
  has its trigger created by the same delayed step that posts its objective.
- Camera and control survive death, teardown and the outcome overlay.
  `teardown_scenario_entities` queues `resume_player_control`, the scenario camera is
  `ScenarioScopedMarker`, and `on_outcome_advance` is not gated on
  `PlayerControlSuspended`, so a Defeat during the suspended cinematic is dismissible.
- No NEW segment-through-body case. All 21 `first_shift` tests pass. The lane
  hand-checked the `arrival_standoff: Some(200 m)` displacement the test does not
  model: leg 2 clears the concealment body by ~3.9 km, leg 3 by ~4.3 km.
- `735eea53` did NOT move the searchers - it changed only `PLAYER_START_POS`,
  `EXTRACTION_POS`, `ID_PLAYER` and a ship name. The batch-7 patrol BLOCKER is not
  from these commits.
- Performance: nothing here has a frame cost worth measuring, and the lane claims
  none. `track_scripted_camera_anchor` and `enforce_scripted_camera_pose` iterate a
  one-entity query in PostUpdate; `scoped_entity`'s fresh `QueryState` runs about four
  times in the whole chapter.

**Interaction with the recorded orbit soft-lock - it got WORSE, not better.**
`735eea53` replaced the old `OnOrbitStable -> TimerStart(orbit_hold) -> OnTimerEnd`
shape with `OnOrbitLap` plus an `OnEnter` return gate (`mod.rs:1005-1030`). That
WIDENS the `AutopilotPhase::Hold` dependency recorded in batch 5: the old shape needed
one `Hold` edge to arm a timer that then ran to completion regardless of stability;
the new shape needs `Hold` SUSTAINED across a full TAU of angular travel -
`trackers.rs:143` resets `echo.angular_travel = 0.0` on every stability flip - and
then a further half turn to reach the 300 m gate ball. If the ring cannot hold, the
beat now has strictly less chance of clearing than before the retime.

**Not checked on the correctness side.** Finding 1 was not reproduced live -
`first_shift_01_departure` has no input driver - so the fraction of approaches that
hit it is unknown. Whether an arbitrary orbit PLANE reaches `ORBIT_RETURN_GATE_POS`
was not settled: a roughly horizontal orbit in the stable band passes within about
243 m of it by hand, but a steeply inclined one could miss, and nothing in the script
or the tests constrains the plane. The carrier dying BEFORE `SALVO_CUT_TO_CUTTER_AT`
was not tested - it would drop the `ID_CARRIER` anchor and hand the camera to the
chase rig mid-cinematic; the run had 21 s of margin and it was not shrunk. Only
`nova_authoring --lib first_shift` was run; `nova_scenario` and `nova_ship` were not.

### Batch 9 - Campaign scenes and set pieces (`bb1cc37e` `02b8775e` `6c3d4a27` `9e54a25e` `daf3528c` `0fb15839`)

Lane B (craft + contracts) returned first and ran checks; lane A appends below.
This is a craft batch - the through-line is decomposing First Shift into scenes
shared with nine per-beat benches - so the craft lane is the main event.

**MAJOR - `examples/playable/shared/first_shift_stage.rs:92-198` - the commit titled "Share the First Shift map with scene examples" COPIED the production belt instead of sharing it.**
Verified here by parsing both files: the 40 `SALVAGE_ROCKS` and 20 `AMBIENT_ROCKS`
rows are byte-identical between
`crates/nova_authoring/.../nova_protocol/stage.rs` and the example-side copy, as are
`CARRIER_POS`, both `*_POS`, `*_RADIUS`, `*_MASS`, `*_TYPE` and `*_SEED`. `02b8775e`
added a second `belt()` beside `stage::belt`. The cost is already visible in the
range: `db72da2e --stat` touches `nova_protocol/stage.rs | 83 +-` AND
`examples/playable/shared/first_shift_stage.rs | 49 +-`, and `b202d69f` touches the
same pair. Two later commits had to edit the duplicate in lockstep, which is exactly
the burden the sharing was meant to remove.
Fix: re-export the production stage builder to examples the way `first_shift_scene`
already re-exports the chapter, and delete the example-side copy.

**MAJOR - `examples/playable/shared/first_shift_stage.rs:1` - the module doc names a consumer that does not exist.**
"Fixed landmarks and complete belt assembly shared by First Shift scene examples and
both chapter map benches." Verified: `grep -rn 'first_shift_stage' examples/` returns
exactly two hits, `first_shift_map.rs:24` and `second_shift_map.rs:23`. No numbered
scene example includes it - they get their belt from `nova_authoring`. `02b8775e`
wrote the doc for `first_shift_setpiece`/`first_shift_attack`, and `0fb15839` in the
same batch deleted both.

**MAJOR - `Cargo.toml:121` and `CHANGELOG.md:312` - both state a contract the batch does not keep.**
The Cargo block reads "Each file owns only preview ship poses; nova_authoring supplies
the real map, cast, story, objectives, cameras, actions and handlers", and the
changelog entry says the numbered examples "add only preview ship poses and an
explicit end message". Verified: `examples/playable/first_shift_08_attack_salvo.rs` is
**307 lines** with a four-flag clap CLI, `LoopCapturePlugin`, a four-step autopilot
script, and `instrument_salvo` at `:236` which drains, re-times and re-sorts the
production salvo sequence and splices variable writes into `events[0]`. That is not a
pose. The "explicit end message" is also not the examples' - it is authored in
`nova_authoring` (`mod.rs:1494 scene_end_message`). This matters because
`docs/development.md:238` makes the Cargo per-block comment a REVIEWED contract: "what
each category proves is the table above plus the per-block comments in the root
`Cargo.toml`, and review enforces it".
Fix: say eight of the nine add only preview poses, and that the salvo bench also owns
capture instrumentation.

**MINOR - `.../first_shift/mod.rs:104` - the scene seam throws away the struct that holds the answer and re-derives it by counting.**
`first_shift()` builds `FirstShiftScenes` with named fields, flattens it at `:1247`
via `into_campaign()`, and `first_shift_scene()` at `:1272` immediately re-derives the
partition with hard-coded counts:
`departure: take_exact(&mut events, 2, ...)`, `rcs: 5`, `salvage: 2`, `navigation: 3`,
`orbit: 5`, `return: 1`, `attack approach: 3`, `attack salvo: 1`, `aftermath: 1`,
`global: 2`.
ADJUDICATED DOWN from the lane's MAJOR, and its failure mode CORRECTED. The lane wrote
"adding or removing one event inside any scene's literal shifts every later boundary".
That specific case IS caught: the counts sum to 25, so a 26th event leaves one behind
and `assert!(events.next().is_none(), "First Shift scene partition drifted")` fires.
What is NOT caught is a same-total reshuffle - moving a handler from `rcs` to
`salvage` in the `FirstShiftScenes` literal keeps the total at 25, `into_campaign`
flattens it, `from_events` re-splits at the old boundaries, and a scene silently
carries its neighbour's handler with every assert passing.
`reusable_scenes_keep_preview_positions_out_of_production_code` checks the Cutter pose,
the two planetoids, the preview message and two verb grants, all of which survive that.
Fix: extract the `FirstShiftScenes { ... }` literal into its own function; have
`first_shift()` call `into_campaign()` on it and `first_shift_scene()` take the struct
directly. `from_events` and `take_exact` then disappear.

**MINOR - `.../first_shift/mod.rs:1273` - the Departure early return skips `full.description`, so one of the nine previews describes itself as the whole chapter.**
Verified: the early return sets `full.id` and `full.name` and returns; every other
scene reaches `:1411` and gets `full.description = format!("Standalone review of {}.",
...)`. `first_shift_01_departure` keeps "A routine shift on the rock plate, out of the
carrier Meridian."

**MINOR - the duplicated belt is dead code in one of its two consumers.**
Lane B ran `cargo check --features debug` over all nine numbered examples plus both
map benches: the only output is five dead-code warnings, all in `second_shift_map`
(`belt`, `planetoid`, `asteroid`, `INSPECTION_MASS`, `CONCEALMENT_MASS` never used).
`second_shift_map.rs:285` hand-rolls its own planetoid spawns rather than calling the
`belt()` this batch added, and nothing suppresses the warnings. Subsumed by the
duplication finding.

**MINOR - the nine benches do not read the same way, and seven do not say how to run them.**
`first_shift_02_rcs.rs` and `first_shift_08_attack_salvo.rs` carry a purpose paragraph
and a run block. `01`, `04`, `05`, `06`, `07` and `09` are a single `//!` line; `03`
has three lines and no run block. Every other cataloged playable example documents its
keys and its invocation (`docs/development.md:243`). Separately, `08`'s header
documents `--offset` and the `NOVA_CAPTURE` path but never `--capture`, the flag that
actually produces its five `STILL_BEATS` and silently suppresses the rail-hit loop.

**MINOR - `first_shift_08_attack_salvo.rs:192` duplicates `shared/first_shift_scene.rs:51`.**
Two `place_ship` bodies with the same walk, the same `SpawnScenarioObject` match, the
same field writes and the same panic, differing only in signature and panic text. `08`
is the one bench that does not `#[path]`-include the shared module - the single file
that most needed the seam is the one outside it.

**MINOR - `crates/nova_authoring/src/lib.rs:41` - the prelude docstring was not updated when this batch added a member.**
It enumerates three of the six members ("the content report model, the lint/balance
walk entry points and the RON generation surface") and `0fb15839` added
`built_in_scenarios::*` without touching it. Separately, `built_in_scenarios` is the
only member glob-exported directly rather than through its own `prelude`, against
"Give exporting modules a `prelude` and export it from the crate root".
Related: `first_shift_08_attack_salvo.rs:5` names `nova_authoring::first_shift_scene`,
a path that does not exist - there is no root re-export, so it is
`nova_authoring::prelude::first_shift_scene`.

**MINOR - `docs/development.md:243` - the playable catalog claims to list "what is on disk today, in reading order" and omits all eleven First Shift / Second Shift playables.**
Three were already missing; this batch added nine more cataloged `[[example]]` targets
without touching the roster. Also noted, pre-existing and outside the diff:
`docs/scenario-system.md:753` says "Its `first_shift.rs` builds the New Game starter",
a file that no longer exists.

**What lane B ran.** `cargo test -p nova_authoring --lib first_shift` - 21 passed.
`cargo check --features debug` over all nine numbered examples and both map benches -
compiles, five dead-code warnings as above. `cargo test -p nova_probe_cli --test
catalog_drift` - `catalog_matches_disk` PASSES, so the nine are correctly cataloged
with no dangling targets; only the known batch-1 failure remains. A grep across
`docs/ scripts/ web/ .github/ *.toml` for the deleted `first_shift_attack`,
`first_shift_rcs` and `first_shift_setpiece` found no dangling references outside
`tasks/`.

**Verified clean.** No NEW changelog defect: lane B re-measured the whole
`[Unreleased]` block and found no new over-length or misfiled entry, and the
six-commits-one-change collapse rule WAS honoured here - `:312` replaced two
per-example entries. Player-facing figures in `web/src/wiki/getting-started.md` were
rewritten by later batches to numbers that check out (seven `film()` call sites, and
the A-B-C-D route matches `marks.rs:121-155`).

**Not checked on the craft side.** No rendered output was inspected, so nothing about
how a scene LOOKS is verified. No further wiki or create sweep beyond the pages this
batch touched.

#### Batch 9, lane A (correctness + performance)

The lane ran ALL NINE benches headless plus both map benches. No blocker; production
behaviour in this range is sound. The findings are all about the benches failing to
stage what production stages - which is the one thing this batch exists to guarantee.

**MINOR - `examples/playable/shared/first_shift_stage.rs:131` - the duplicated bench belt has ALREADY DRIFTED, and it renders a belt the chapters no longer have.**
This sharpens lane B's duplication finding from debt into a live defect. Verified here:
production `stage.rs:229-230` builds each rock with `material: kind.to_string()` drawn
from `SALVAGE_MIX` (four kinds) and `AMBIENT_MIX` (rock/carbon/ice), plus
`destroy_sound: Some(...)`. The example copy still builds EVERY rock as:

    material: KIND_ROCK.to_string(),
    destroy_sound: None,

`b202d69f` - batch 1, the rock-shading commit - gave production rocks their materials
one commit after `02b8775e` made the copy, and the copy never followed. So
`first_shift_map` and `second_shift_map` render an all-rock, silent belt. The sting is
that `stage.rs:11` and `marks.rs:9` name those two benches as this stage's "layout
provenance" - the benches a reviewer is told to trust are the ones showing the wrong
belt. Fix: export `stage::belt` and delete the copy.

**MINOR - `.../first_shift/mod.rs:1273` - `first_shift_scene(Departure)` is the only scene that drops the global DEFEAT handlers.**
Verified: `scenes.global` holds `defeat(DEFEAT_DESTROYED, OnDestroyed)` and
`defeat(DEFEAT_NEUTRALIZED, OnNeutralized)` (`:1242-1244`), and the common tail does
`events.extend(scenes.global)` at `:1407`. The Departure early return leaves before
reaching it. So the one bench whose leg starts 1.1 km off the carrier's flank is the
one bench where a collision is silent, while every other scene declares Defeat.
This is the same early return lane B flagged for dropping `full.description`; the
dropped defeat handlers are the load-bearing half. Fix both with one restructure.

**MINOR - `.../first_shift/mod.rs:1390` - the Aftermath bench reproduces the beat's dialogue but not the shot or the control state, and the shot is the entire beat.**
In production, Aftermath inherits the salvo's composition: `salvo()` at `:1667` sets
`film(ID_CUTTER, CINEMA_DEATH_OFFSET, point(stage::CARRIER_POS))` and control has been
suspended since `BEAT_ATTACK` with no `resume_player_control()` in
approach/salvo/aftermath. The standalone arm pushes only
`post_objective(OBJ_SILENCE, ...)` - no `suspend_player_control()`, no `film(...)` -
while the `AttackSalvo` arm at `:1375` DOES add `suspend_player_control()`.
VERIFIED BY RUNNING (`NOVA_AUTOPILOT=1 NOVA_CAPTURE=1 first_shift_09_aftermath`): the
capture is the live chase rig with a live speed readout and the wreck off-frame, the
objective chip reading "Meridian 3.06 km" with an off-screen arrow. Fix: push
`suspend_player_control()` and the `film(...)` into the Aftermath arm.

**MINOR - `.../first_shift/mod.rs:1416 preview_control_grants` hands over verbs the scene's own script is about to grant, so the gate each scene exists to stage is the state no bench reproduces.**
The grants land in `start_actions`, ahead of the entry actions. Where control is not
suspended: the Orbit bench grants `Orbit` at OnStart while production grants it in the
last step of `SEQ_DETOUR_BRIEFING` (`:965`), about 23 s of briefing later; the Salvage
bench grants `Lock` at OnStart while production grants it in `SEQ_THIRD_ROUTE` after
the second crate (`:855`). VERIFIED BY READING and by the `first_shift_05_orbit`
capture: the `O ORBIT` chip is lit at t=6 s with the detour briefing still on its
opening line. `preview_control_grants` carries no docstring saying this is deliberate,
and `reusable_scenes_keep_preview_positions_out_of_production_code` asserts the grants
EXIST rather than that they match the beat.

**Verified with no defect found, and this is the valuable half of the report.**
- **`9e54a25e`'s synchronisation claim HOLDS.** Ran it: `ForceRailgunFire: 'warship'
  section 'railgun_port'` and `'railgun_starboard'` log at `00:27:57.522696` and
  `.522732` - one frame. Both `ScriptedRailgunOrder` inserts go into one command queue
  and `charge_and_fire_railgun` advances both on the same fixed tick with the same `dt`.
- **`bb1cc37e`'s reframed destruction shot survives its subject.**
  `loader/camera.rs:138` drops the whole override when the ANCHOR dies; the anchor is
  the cutter and the aim is `Point`, not `Object`. The one shot anchored on the dying
  hull cuts away at salvo t=10.7 s, and the carrier is not marked destroyed until
  t=21.8 s - 11 s of margin, verified by running.
- **`CINEMA_DEATH_OFFSET`'s framing numbers check out.** A camera-space solve at
  Bevy's default 45-degree vertical FOV puts the cutter at 58% of half-width on 16:9
  and 77% on 4:3, against the documented 63%/80% - inside the frame on both.
- **The documented approach timings are real.** Ran `first_shift_07_attack_approach`
  to completion: emerge 34.4 s, approach 33.0 s, align 6.3 s against the authored
  34/33/6, settling at 0.0241 rad within a 0.0349 rad tolerance. `holds_after_completion`
  keeps `ScriptedAlign` installed, so the bore is under a live hold when the guns fire
  6.5 s later. This independently reproduces batch 8 lane A's numbers.
- **No new segment-through-body case.** All three warship legs clear both planetoids
  and all 60 rocks; worst clearance 565 m.
- **Spawn-then-despawn inside one `OnStart` works.** Five benches stage a mark and run
  the transition that clears it in the same event; no `DespawnScenarioObject: no entity
  with id` warning in any of 19 run logs.
- **All nine benches load clean** headless; all 21 `first_shift` tests pass; the nine
  scene ids are NOT generated into `assets/base/scenarios/`, so they do not reach the
  menu.

**Not checked on the correctness side.** Every run was on an RTX 3060 Ti at load
average <= 0.8 - nothing under llvmpipe or on a loaded host, and no performance number
is quoted. The align-to-fire SEAM is unproven by anything runnable: bench 07 ends
before the guns and bench 08 hand-poses the warship with `facing()` and never installs
`ScriptedAlign`, so whether production slugs strike the Meridian from a
`ForceAlign`-produced pose is covered by no bench. Two `avian3d ... has no mass or
inertia` warnings during the carrier breakup were seen and not chased; they come from
the debris/sever path, not from anything this batch authored.

### Batch 10 - Campaign beats, dialogue and placement (eleven commits)

`bc10e342` `99b69dbe` `ef6320ba` `f9533456` `2cfaa902` `e2a1eb45` `9f25b29b`
`a0d93564` `9ee0de9b` `f05298fe` `392b19a2`

Lane B (craft + contracts) returned first and COMPUTED every cited figure; lane A owns
the failing pacing test and appends below. All 21 `first_shift` tests pass at HEAD, so
the placement commits' own guards hold - the defects are in the prose and the changelog
around them.

**MAJOR - `.../first_shift/marks.rs:170` - the TRANSIT 1 docstring names a flank the coordinates contradict, and the player-facing wiki repeats it.**
"The first transit mark takes the mandatory route around the inspection body's
**western flank**." Recomputed here from `TRANSIT_ONE.position (-6803, -5750, -5061)`
against `INSPECTION_POS (-4500, -400, -6500)`: the offset is 2303 m west, 1439 m
forward and **5350 m DOWN** - a horizontal component of 2716 m against a vertical of
5350 m, putting the mark **63.1 degrees below the horizontal**. It is under the body,
not around its flank. `f05298fe` moved it from `(-8000, -1600, -5000)`, which WAS
dominantly west, and kept the old wording. `web/src/wiki/getting-started.md:71` tells
the player "Hold Ctrl on the mark out west" - from the crate-2 lock position it is
7.0 km west but also 5.85 km down, 38.7 degrees below, which a player sweeping the
horizontal plane will not find. Fix: say "under the inspection body's western flank"
in both, or restore a westward mark.
The rest of that docstring block is correct and was recomputed: TRANSIT 1's centre
distance is exactly 6000 m against a 3286 m SOI, so its 700 m volume clears by 2013 m;
TRANSIT 2 is at 6999.6 m ("7 km") and 0.43 degrees off the Meridian-to-body line, so
"directly behind" holds.

**MAJOR - `CHANGELOG.md:72` and `:62` - two entries document changes to an unreleased chapter, which the collapse rule forbids.**
`:72` "First Shift salvage crates collect only at visible contact: their pickup radius
shrank from 80 m to the 15 m crate envelope" (from `99b69dbe`). The lane verified by
running `git ls-tree -r --name-only v0.12.0 | grep first_shift` - EMPTY - and
`git show v0.12.0:CHANGELOG.md | grep "First Shift"` - empty. The 80 m radius was
authored and replaced entirely inside `[Unreleased]`.
`:62` "First Shift moves WORK SITE and the third crate fully clear of Belt Rock 6's
gravity well, **so the return GOTO can settle and complete**" (from `392b19a2`) names a
bug introduced and fixed inside this same cycle.
`AGENTS.md`: "Collapse several pre-release revisions of one change into a single final
entry. Omit bugs introduced and fixed inside the same release cycle." Both should be
deleted or folded, with no before/after.
The line above them at `:61` - "Nav beacons are radar-acquirable within 12 km by
default, up from 6 km" - IS legitimate and the lane verified it against the release:
`BEACON_LOCK_SIGNATURE` was 20.0 world units = 200 m = 6 km at `v0.12.0`.

**The commit-to-entry map for the recorded six-entry collapse**, so the range-level fix
is actionable. Five of the six are this batch's work on one unshipped chapter:

| entry | origin |
|-|-|
| `:61` nav beacons 12 km | `392b19a2` - KEEP, an engine change with a shipped baseline |
| `:62` WORK SITE / third crate | `392b19a2` - delete |
| `:67` control suspended through teardown | rewritten by `bc10e342` |
| `:69` STOP / RCS marks / GOTO legs | rewritten by `e2a1eb45`, then `a0d93564` |
| `:72` crate pickup radius | `99b69dbe` - delete |
| `:74` two new chapters **(breaking)** | pre-batch |
| `:77` First Shift opens New Game | rewritten by `f9533456`, then `9ee0de9b` |

The two recorded over-200-character entries are also this batch's: `:69` (201) last
written by `a0d93564`, `:77` (201) last written by `9ee0de9b`.

**MINOR - `CHANGELOG.md:68` is a 109-character raw line**, the longest in the whole
`[Unreleased]` block; the next longest is 79 and every other continuation wraps at ~79.
From `bc10e342`. Rewrap.

**MINOR - `.../first_shift/marks.rs:194` and `story.rs:150,154` - the return beat says "the plate" for a site two kilometres clear of it.**
Recomputed here: the new `WORK_SITE (-1200, -1200, -800)` is **1993 m** from the
nearest salvage-rock centre and 1849 m from its worst-case surface. The position
`392b19a2` replaced was 528 m out. The third crate is now 884 m clear of the nearest
rock - FURTHER OUT than crate 1 at 660 m, which the same script calls "on Plate
Seven's near edge". Meanwhile `RETURN_CHIEF` says "Get back on the plate and bring me
that third crate" and `SEARCH_COPILOT` says "Back on the plate", while `story.rs:99`
says the opposite: "The third crate is outside the plate." Fix: pick a return site
inside the field, or retire "back on the plate" from both lines and reword `:194`.
The rest of that docstring is correct and computed: Belt Rock 6 is genuinely the
tightest well at 2844 m against a 1264.9 m SOI, leaving 1579 m for the stated 700 m
volume plus 500 m buffer, and the last crate is 1166 m away ("1.17 km").

**MINOR - `.../first_shift/mod.rs:1670` - the aftermath step re-issues a camera shot identical to the one already running, and its comment describes a move that does not happen.**
Verified at HEAD: `step(SALVO_CUT_TO_CUTTER_AT, [film(ID_CUTTER, CINEMA_DEATH_OFFSET,
point(stage::CARRIER_POS))])` at `:1647`, and `step(SALVO_AFTERMATH_AT, [film(ID_CUTTER,
CINEMA_DEATH_OFFSET, point(stage::CARRIER_POS)), ...])` at `:1667` - byte-identical
arguments with nothing releasing the camera between them. The comment says "Once its
departure reads, **return to** Cutter and the wreck", but it never left. From
`bc10e342`. Note `web/src/wiki/getting-started.md:79` counts "seven authored shots",
which matches the seven `film()` call sites but only SIX distinct compositions.

**MINOR - `nova_protocol/cast.rs:34` - `CUTTER_NAME`'s docstring is false on both of its claims.**
"The player's ship, **named in objective text** and **used as its scenario id**."
Verified by grep: the only uses are `mod.rs:348` and `second_shift.rs:248`, both the
spawned object's `name` field, plus one test assertion. The scenario id is the separate
literal `ID_CUTTER = "cutter"` (`marks.rs:24`), and no objective text reads the
constant - `story.rs` writes the name as a literal. `e2a1eb45` widened the gap:
`OBJ_TEXT_STOP` went from "bring Cutter to rest" to "bring **Cutter One** to rest", so
the HUD readout says "Cutter" while every objective and every line of dialogue says
"Cutter One". Fix the docstring, and decide whether the object should be named
"Cutter One".

**MINOR - `.../first_shift/marks.rs:232` - `TRANSIT_SIGNATURE` is now numerically identical to the global default it overrides.**
Verified at HEAD: `TRANSIT_SIGNATURE = Meters(400.0)` and
`crates/nova_scenario/src/objects/beacon.rs:28 BEACON_LOCK_SIGNATURE = Meters(400.0)`.
`392b19a2` raised both in the same commit (300 -> 400 and 200 -> 400), so
`lock_signature: Some(TRANSIT_SIGNATURE)` on all four marks now produces exactly what
`None` would. `beacon.rs:57` documents `None` as the default, and the house rule
reserves `Option` for "an override whose documentation states what the absence means" -
this is an override that overrides nothing. Fix: one clause saying it is deliberately
pinned at the current default so a global retune cannot silently re-range these legs.

**MINOR - `.../first_shift/mod.rs:229` and `:892` - two comments describe dialogue `a0d93564` deleted.**
"The same gesture again, **with nothing said over it**" on `BEAT_TRANSIT`, and "the
same two keys with **four words** over them". `a0d93564` replaced the single line
"Second mark is up. Same again." with a four-line exchange around this beat.

**Verified correct by computation, and worth recording because it is most of the batch.**
`HOME_MARK`: 3061 m off the Meridian ("3.06 km"), 109.7 degrees between carrier and
firing position ("110 degrees round"), 2151 m off the torpedo lane ("2.15 km"). The
"100 m/s RCS cap" matches `FlightSettings::rcs_speed_cap`. The warship legs are 3396 m
and 4029 m ("3.40 km", "4.03 km"). "Seventeen hundred metres" is 1710.7 m
start-to-work-mark. "70 m abeam" is exactly 70 m and outside the 15 m envelope. The
wiki's 150 m/s cap, 100 m/s RCS, three kilometres to the hold, seven `film()` sites and
two conversation holds all match HEAD. `web/src/create/objects.md:389` and
`web/src/wiki/scenarios.md:11` were updated in lockstep with the 12 km beacon change.
No orphaned story constants - all 91 are used exactly once - speakers match their
lines, no bare `#[allow]`, the one `#[expect]` carries a reason, `coached_beat_setup`
was fully removed rather than left dead, and no example holds a stale copy of a moved
position.

**Not checked on the craft side.** Three pre-existing stale figures in `marks.rs` that
this batch did not touch and that belong to the planetoid conversion: `:284` "GOTO
parks 500 m outside the geometric body (700-1200 m)" and `:296` "the widest mesh the
body can grow (3.0 km)" are rock-era ranges, and `tests.rs`'s "against the widest
geometric radius" docstring is stale because HEAD reads the exact
`stage::inspection_body_radius()`.

#### Batch 10, lane A (correctness + performance)

The lane was tasked with producing the exact fix for the failing pacing test and
delivered it, plus a new MAJOR that generalises batch 8's STOP finding to three more
beats.

**THE PACING TEST FIX (the range's one red test).**
The lane enumerated every action group in the generated RON with a scanner mirroring
`loader/mod.rs:436 action_groups`. Exactly TWO violating groups in `first_shift`, ZERO
in `second_shift`:
- `mod.rs:617` - `[ReleaseCamera, ResumePlayerControl, Objective, StoryMessage,
  SpawnScenarioObject, ObjectiveMarkerAttach]`, `after: Some(4.0)`
- `mod.rs:690` - `[SetControllerVerb, ReleaseCamera, ResumePlayerControl, Objective,
  StoryMessage, HintEmphasisSet, ObjectiveMarkerAttach]`, `after: Some(5.5)`
`git show e2a1eb45` confirms the second is newly created: it lifted
`TRIM_COPILOT_FIRST_MARK` INTO the step that already held
`post_objective(OBJ_TRIM_LATERAL)`.

Both are the terminal step of an opening conversation, where the sequence hands back
camera and helm, plays the closing line, and posts the objective in one list. In both
the LINE is correctly placed and the OBJECTIVE must move a beat later - **together with
the control return**, not after it.

**Why the obvious fix is wrong, and this is the valuable part of the report.** Leaving
`release_camera`/`resume_player_control` with the line and deferring only the objective
INTRODUCES A RACE: `set_variable(VAR_BEAT, ...)` runs in the introducing handler's own
frame, so the next beat's gate is already open, and returning control before the
objective exists lets the player satisfy the next beat first. `remove_objective` then
only warns and the deferred post lands afterwards - a card nothing can complete. This
is the batch-8 STOP defect, reintroduced by the naive fix. **Control return must stay
atomic with the objective.**

The edit is to split each into two steps, the line alone first and the control return
plus objective second, both at `INSTRUCTION_GAP` (= `COMMS_MIN_SECS` = 4.0). Not
`beat_setup`/`beat_later` - these steps are already inside a `Sequence`, and adding a
step is the idiom the chapter already uses at `mod.rs:851` and `:988`. Not `after: 0.0`
- that is the batch-7 same-frame mechanism.
**Nothing else breaks**, verified against every consumer:
`no_mainline_scenario_posts_an_objective_at_onstart` and
`opening_objectives_are_deferred_past_frame_one` only require `index > 0 || after > 0.0`;
`the_panel_never_points_at_two_places_at_once` walks in walk order and the split
preserves it; `the_rcs_briefing_shows_the_complete_four_mark_box_before_control_returns`,
`only_the_conversation_holds_return_control_before_teardown` and
`every_withheld_control_is_handed_back` are step-position blind. `SEQ_OPENING` and
`SEQ_TRIM_BRIEFING` are referenced only at their construction sites.
`cargo test -p nova_authoring --lib` at HEAD is 109 passed / 1 failed and the one
failure is this test. **The fix requires `content -- gen` afterwards** to regenerate
`assets/base/scenarios/first_shift.content.ron`.

**MAJOR - `.../first_shift/mod.rs:723`, `:749`, `:778` - the RCS box can strand up to THREE objective cards, and `e2a1eb45` made it the expected flight.**
The same mechanism as the batch-8 STOP finding, at three further beats. Each box
handler flips `VAR_BEAT` in its own frame but posts the next objective 4 s later, and
all four trim marks are already spawned.
Recomputed here from `marks.rs`: TRIM A `(-200, 80, 900)`, B `(-200, 300, 900)`,
C `(-500, 300, 900)`, D = `WORK_MARK.position` `(-500, 80, 900)`, each `area:
Meters(100.0)`. Centre-to-centre 220 m / 300 m / 220 m, so **boundary-to-boundary free
travel is 20 m / 100 m / 20 m**. At the 100 m/s RCS cap that is 0.2 s, 1 s and 0.2 s -
against a 4.0 s deferral. A player flying the box continuously consumes each next
`once: true` handler before its objective exists; the completion warns and no-ops, and
4 s later the card and a `highlight()` post onto a mark the next handler already
despawned. Nothing completes it for the rest of the chapter.
`e2a1eb45` is what makes this the EXPECTED flight rather than a corner: its new
`TRIM_COPILOT_BOX` line hands the player the whole route up front - "Out to A, up to B,
across to C, then back down to D" - so waiting for each card is no longer natural.
Not a soft-lock: the beat chain still advances. The damage is stuck HUD cards and dead
markers. Verified by reading and arithmetic, not by a piloted run.
**Fix, and the lane is right about the trap:** the 4 s deferral is the point of the
pacing pass, so the GEOMETRY must absorb it - shrink the `TRIM_*` areas to ~50 m
and/or widen the box so the shortest boundary gap exceeds `INSTRUCTION_GAP` at the cap.
Moving `set_variable` into the deferred step is NOT a fix: `OnEnter` is a
`CollisionStart` edge (`objects/area.rs:186`) and will not re-fire for a ship already
inside, which converts a stuck card into a genuine soft-lock.

**MINOR - `.../first_shift/mod.rs:1673` - `bc10e342` collapsed the aftermath objective into a one-frame flash.**
`post_objective(OBJ_SILENCE, ...)` and `set_variable(VAR_BEAT, number(BEAT_DISTRESS))`
now sit in the SAME salvo step, and the aftermath handler is `OnUpdate` + `once: true`
gated on `VAR_BEAT == BEAT_DISTRESS` whose first actions include
`complete_objective(OBJ_SILENCE)`. Posted and completed within one frame, so "Hold
position and keep the channel open." is never readable and the panel is empty for the
whole ~14 s aftermath conversation. `git show bc10e342` removes the step that separated
them and deletes `SALVO_DISTRESS_AT: f64 = 14.0`; before, the card lived 14 s.
Fix is constrained: `the_cinematic_runs_its_shots_in_order_without_returning_attack_control`
now pins `aftermath_cutter == distress`, so the beat flip must stay on the aftermath
shot step and the same test forbids a `StoryMessage` in the salvo chain. So move the
POST, not the flip: drop it from the salvo step and post it in the aftermath handler's
own frame, which carries no comms line and is therefore pacing-legal.

**MINOR - `crates/nova_scenario/src/objects/beacon.rs:28` - `392b19a2` doubled the GLOBAL beacon lock range as collateral.**
Verified: `git log -S` shows `392b19a2` is the sole commit introducing
`BEACON_LOCK_SIGNATURE = Meters(400.0)`, taking every default beacon in every scenario
and every mod from 6 km to 12 km of acquisition range, from a commit scoped to "Clear
First Shift goals from belt gravity". Nothing in First Shift needed it: every mark
carries its own `Some(TRANSIT_SIGNATURE)` override, and the same commit raised
`TRANSIT_SIGNATURE` 300 -> 400, which IS what the moved `TRANSIT_ONE` required (its
longest pinned leg, crate 2 to TRANSIT 1, is 9354 m; the old 300 m bought 9000 m). The
beacons that inherit the default are elsewhere - `main_menu/shared.rs:154` and Second
Shift's `stage::beacon()` calls - and none asked for a longer lock.
This is the other half of lane B's `TRANSIT_SIGNATURE`-equals-default finding: the pair
of raises is why the override became a no-op. Fix: revert
`BEACON_LOCK_SIGNATURE` to `Meters(200.0)`, keep `TRANSIT_SIGNATURE` at 400 - the marks
already override, so nothing in the chapter changes - and restore the "6 km at the
default settings" wording in `beacon.rs:22` and the two web pages the commit edited.
NOTE this contradicts lane B, which read `CHANGELOG.md:61` ("Nav beacons are
radar-acquirable within 12 km by default, up from 6 km") as a legitimate entry with a
shipped baseline. Both are right on their own terms: the entry accurately records what
shipped, and the change itself was unintended collateral. **Decide the change first,
then the entry.** If the revert is taken, `:61` goes with it.

**Checked and clean.** The four placement commits' geometry holds: crate 3 is 1166 m
from `WORK_SITE`, `TRANSIT_TWO` is 7000 m and `TRANSIT_ONE` 6000 m from
`INSPECTION_POS`. `CRATE_SIZE` is the cuboid's full edge, so `99b69dbe`'s 12.99 m
half-diagonal / 2.01 m tolerance claim is exact. First Shift's corridor pins use true
point-to-SEGMENT distance (`tests.rs:773`) - unlike the Second Shift patrol pin in the
batch-7 blocker - and both corridor tests pass with the moved marks. The 15 m crate
pickup is a `Sensor` sphere against section colliders, not a centre-to-centre test, so
the tightening from 80 m is reachable. Spawn-then-attach in one frame is safe and
documented. `9ee0de9b`'s hand-rolled replacement of `pacing::open_outro` still sets
`BEAT_OUTRO` before the outro chain, so `defeat()`'s gate still locks the win in. Every
other beat spawns its trigger in the same deferred step as its objective, so no other
beat can be consumed early.

**Not checked.** No live playthrough - findings 2 and 3 need a piloted run to observe,
which headless cannot provide. No measurement: nothing in this batch is a frame-cost
change, so no numbers are quoted and no GPU time was used. The proposed pacing edits
were not compiled (read-only). Preview-scene `first_shift_scene(...)` OnStart frames DO
post objectives beside comms lines (e.g. `Salvage`, `mod.rs:1298`), but the mainline
pacing test does not cover scene variants and the benches are not shipped content, so
the lane did not treat it as a defect - flagging it here as a judgement call, not an
oversight.

**Flagged forward, outside this batch.** `ORBIT_RETURN_GATE_POS` is a 300 m sphere
1596 m from the inspection body's centre; whether every player-chosen orbit PLANE
crosses it is a real soft-lock question. The position comes from `68431eb3` (batch 5);
`392b19a2` only added a test that reads it. Batch 8's lane A raised the same doubt
independently. This belongs in the verdict.

### Batch 11 - Playable benches and the probe roster (`eda1e6ce` `fb347d4b` `568149b2`)

Lane B (craft + contracts) returned first and ran checks; lane A appends below.

**MAJOR - `examples/playable/shared/first_shift.rs:16` and `first_shift_map.rs:42` - the bench fleet is a stale FORK of shipped base content, and it reproduces the siege gun by multiplication instead of mounting the prototype that exists.**
Every hull the bench hand-authors now exists in base content: `utility_cutter`
(`ships/block.rs:112`), `industrial_carrier` `:245`, `stolen_warship` `:331`, the five
`salvage_*` `:407-489`, and the wreck as four named ships `:523-548`, all registered in
`ships/mod.rs`. The fork has drifted exactly where it matters. Verified at HEAD:
- base content mounts `SIEGE_RAILGUN = SIEGE_RAILGUN_LANCE_SECTION_ID` on the warship's
  spinal guns (`block.rs:42,:379,:385`);
- the bench mounts the STANDARD `RAILGUN = "railgun_lance_section"`
  (`shared/first_shift.rs:16,:416`) and then hand-hacks it at runtime in `tuned_warship`
  (`first_shift_map.rs:379-381`): `slug_damage = 500.0`,
  `slug_power *= 200.0`, `rake_radius = Meters(30.0)`.
Those are the siege prototype's own numbers - the standard lance's `slug_power` is
1800, and 1800 x 200 = 360 000, which is exactly `slug_power` on the siege grade
(`standard.rs:942`). The bench recreates by multiplication a prototype the catalog
already ships, and a developer reviewing weapon fit there is not looking at the shipped
ship.
Two docs assert the opposite of the shipped state: `first_shift_ships.rs:8` says
"These are candidate structures, **not promoted base-content ships**. Iterate here
before the campaign depends on their silhouettes" - the campaign shipped on them,
`CHANGELOG.md:74` - and `Cargo.toml:110` says "posed side by side for free-fly visual
review **before promotion into base content**".
Fix: pose the base-content hulls, delete the copies, drop the 200x hack in favour of
the siege prototype, and correct both doc claims.

**MAJOR - `Cargo.toml:108` promises three ships; the example poses eight.**
"The three fixed candidate ships for Nova Protocol's replacement opening: maintenance
cutter, industrial carrier, and stolen military warship." Verified at HEAD: the
example's own scenario description reads `"Eight candidate ships for the first two Nova
Protocol scenarios"` (`first_shift_ships.rs:114`), and `showcase()` spawns the three
plus five searchers. `fb347d4b` added the five and never touched the block comment.
`docs/development.md:238` makes that comment a reviewed contract.

**MAJOR - `Cargo.toml:111` - `first_shift_ships` is in `playable/` on the one disqualifier the category names.**
The category contract at `Cargo.toml:60` excludes "an example whose only affordance is
the free-fly camera the scenario loader hands every cameraless scene", and
`docs/development.md:219` repeats it as the disqualifier column. The example's own doc
says exactly that: "posed for a free-fly visual review", "Nothing in the example flies
or fights", "Hand-run with the free WASD camera". The only systems it wires are
`load_showcase` and `frame_new_camera`. The two map benches are fine - `--pilot` flies
a ship. Fix: give it an affordance, or move it to `screenshots/`.

**MAJOR - dead code in `examples/playable/shared/first_shift_stage.rs` is denied by the CI clippy gate.**
Verified at HEAD: `.github/workflows/ci.yaml:102` runs
`cargo clippy --workspace --all-targets --features debug -- -D warnings`, and its own
comment says it is "what catches the unused imports and dead code a refactor leaves
behind". Lane B RAN `cargo check -p nova-protocol --example second_shift_map
--features debug` and got five dead-code warnings: `INSPECTION_MASS`,
`CONCEALMENT_MASS`, `planetoid`, `asteroid` and `belt` all never used. This is the
same set batch 9's lane B saw; what is new is the CI consequence.
This is a SECOND independent CI blocker after the `planet_types.rs` build break -
distinct because it survives that fix.
Cause: the module doc claims the belt is "shared by ... both chapter map benches", but
`second_shift_map.rs:282-359` rebuilds the belt inline with its own local `planetoid()`
and `asteroid()`. Fix: make `second_shift_map` call `stage::belt` and delete its two
local helpers - which also fixes the drifted-copy finding from batch 9.

**MAJOR - `examples/playable/second_shift_map.rs:32` claims an exactness the code contradicts.**
"These landmarks and rocks are the **exact fixed stage** from first_shift_map. Only the
chapter-specific ships, wreckage, and review labels change." Three differences: the
rocks are `invulnerable: true` here (`:515`) and `false` through `stage::belt`; the
ambient rocks are named "Ambient Rock {n}" against "Belt Rock {n}"; and the planetoid
masses are re-typed literals `27_000.0`/`20_000.0` instead of the shared constants -
which is precisely why those two constants are dead.

**MINOR - `second_shift_map.rs:51` - 28 authored wreck POSITIONS are dead data.**
`WRECK_PLACEMENTS: [(Meters3, Vec3); 28]` is about 110 lines of position+rotation
pairs, but the loop at `:320` destructures `(_old_position, rotation)` and computes the
position from `stage::CARRIER_POS` and `stage::SALVAGE_ROCKS[index - 1].0 +
WRECK_SCATTER[...]`. Every `Meters3` is discarded and the binding is literally named
`_old_position`. Fix: reduce to `[Vec3; 28]` and rename `WRECK_ROTATIONS`.

**MINOR - `first_shift_ships.rs:173` authors camera positions in raw world units.**
`CAMERA_TARGET: Vec3 = Vec3::new(0.0, 0.0, 15.0)` and
`CAMERA_POSITION: Vec3 = Vec3::new(0.0, 55.0, -70.0)` go straight into a `Transform`
with no `to_engine`/`from_engine` and no local note that this is the engine boundary -
150 m, 550 m and 700 m written as 15/55/70. Ten lines earlier the same file writes the
identical point in meters: `Meters3::new(0.0, 0.0, 150.0)`. Both map benches do it
correctly. This is the units rule, and the file breaks it against its own neighbour.

**MINOR - `first_shift_map.rs:42` - unexplained weapon tuning, unmentioned in either doc.**
The three tuning constants carry no comment saying why those numbers, and neither the
module doc nor `Cargo.toml:115` says the bench flies an overpowered gun. A developer
running `--pilot warship` reads the result as the shipped weapon. Subsumed by the first
finding if the siege prototype is adopted.

**MINOR - public items without docstrings, inconsistent inside one file.**
`shared/first_shift.rs:31 maintenance_cutter`, `:253 industrial_carrier` and
`:330 stolen_warship` have none while all five `salvage_*` siblings do, and the design
prose that should be their docstring sits as a body comment instead. Same pattern in
`first_shift_stage.rs` for `CARRIER_POS`, `INSPECTION_POS`, `CONCEALMENT_POS` and
`AMBIENT_ROCKS`. Fix: promote the body comments to `///`.

**MINOR - `shared/first_shift.rs:332` is a history comment.** "The old seven-wide slab
made the ship read thick beside the carrier instead of fast and military." Same class
at `first_shift_stage.rs:54` ("Fill the former bowl..."). Keep the constraint, drop the
account of the previous revision.

**MINOR - about 150 duplicated lines between the two map benches, while a shared module was created in the same batch.**
`refuse_broken`, `spawn`, `marker`, `beacon`, `ship_object`, `facing`, `set_view`,
`frame_new_camera`, `accelerate_camera` and the 45-line `select_view` are near-verbatim
copies. `fb347d4b` created `shared/first_shift_stage.rs` in the same commit, so the
seam existed and was not used.

**MINOR - the playable catalog gap is bigger than recorded.** The range-level item says
eleven playables are missing from `docs/development.md`. Lane B diffed the `[[example]]`
blocks against the bullet list by script: it is THIRTEEN - the nine numbered
`first_shift_0*`, plus `first_shift_ships`, `first_shift_map` and `second_shift_map`
from this batch, plus `asteroid_kinds` and `planet_types`.

**MINOR - no `CHANGELOG.md` entry for the three benches, and precedent says one is owed.**
On the judgement call "are developer-only benches changelog material" - in this repo,
yes: `[Unreleased]` already carries `:309` "New `railgun_wake_bench` example" and
`:312` for the numbered scenes, and released blocks carry `greeble_catalog`,
`shape_bench` and `block_bench`. None of the three appears anywhere in the file. Per
the collapse rule they are ONE entry, not three.

**The probe roster (`568149b2`) is CLEAN - no finding.** The restored entry
(`catalog_drift.rs:146-152`) names three slugs that all exist as `probe_marker`
literals at `examples/systems/system_turn_limit.rs:399,:510,:521`, its header table
documents them, and the count bump 220 -> 223 matches. The roster IS documented for
contributors: `examples/systems/README.md:107` - "Put its slugs on the
`catalog_drift.rs` roster and fix `SYSTEMS_INVARIANTS`." Lane B ran
`catalog_drift catalog_matches_disk` and it passes, so the three new `[[example]]`
blocks agree with disk. The batch-1 roster failure is a different range going stale,
not a defect in this mechanism.

**Not checked on the craft side.** No example was run. Whether the bench cell plans
differ numerically from base content beyond the weapon prototypes was spot-checked only
(`utility_cutter` identical; `stolen_warship` cells identical, weapons diverge); the
remaining six hulls were not diffed.

#### Batch 11, lane A (correctness + performance)

The lane ran probe against the new benches and ran clippy against the CI gate. It also
cleaned up a 1.5 GB scratch probe-run directory.

**MAJOR - `examples/playable/first_shift_map.rs`, `first_shift_ships.rs`, `second_shift_map.rs` - all three examples this batch adds to the catalog wire NO probe harness, so `probe run` fails on each. They are the only three in the whole catalog like this.**
Verified here: a grep for `nova_probe`, `AutopilotPlugin`, `LoopCapturePlugin`,
`nova_autopilot`, `nova_screenshot` or `nova_frametime` returns **zero** hits in each of
the three, while every other catalog example reaches a harness directly or through
`shared/first_shift_scene.rs`. They call
`AppBuilder::new().with_game_plugins(...).build(); app.run()` and nothing else, so the
app never exits - and `probe run` has no exclusion mechanism ("nothing is excluded").
VERIFIED BY RUNNING:

    probe run first_shift_ships --correctness-only --timeout 60
      probe: run exceeded 60s and was killed
      process_exit    FAIL     1/1 pass(es) failed
      run_completed   SKIPPED  no timeline
      reached_playing SKIPPED  no timeline
      first_shift_ships  FAIL  measured 3/8
    probe run second_shift_map --correctness-only --timeout 60
      second_shift_map   FAIL  measured 3/8

This is a THIRD CI-visible failure in the range, alongside the `planet_types.rs` build
break and the clippy dead-code errors below.
Fix: give each a harness. All three are static-pose scenes, so
`nova_screenshot(nova_autopilot())` - what `shared/first_shift_scene.rs` already uses -
is the minimum; the two maps additionally want `nova_frametime()`, because they are the
most populated authored scenes in the game (60 rocks, 2 planets, up to 34 skinned ships)
and NOTHING measures their frame cost today.

**MAJOR - the dead-code warnings in `shared/first_shift_stage.rs` are hard ERRORS under the CI gate.**
This independently confirms and hardens lane B's finding. The lane ran the gate itself:

    cargo clippy --no-deps --features debug --example second_shift_map -- -D warnings
    error: constant `INSPECTION_MASS` is never used   --> shared/first_shift_stage.rs:14:11
    error: constant `CONCEALMENT_MASS` is never used  --> shared/first_shift_stage.rs:18:11
    error: function `planetoid` is never used         --> shared/first_shift_stage.rs:92:4
    error: function `asteroid` is never used          --> shared/first_shift_stage.rs:114:4
    error: function `belt` is never used              --> shared/first_shift_stage.rs:144:8
    error: could not compile `nova-protocol` (example "second_shift_map") due to 5 previous errors

`.github/workflows/ci.yaml:102` runs `--workspace --all-targets`, which includes this
example. Note `first_shift_map.rs` guards `mod first_shift` with
`#[expect(dead_code, ...)]` but NEITHER map guards `mod stage`. One fix -
`second_shift_map` calling `stage::belt` - clears all five errors, the staging
divergence and batch 9's drifted-copy finding together.

**MINOR - `examples/playable/first_shift_map.rs:33` - the bench's route marks and crate positions no longer match the shipped chapter, so `assert_crates_clear_rocks` proves clearance for positions the game does not use.**
The belt still agrees with production through `stage::`, but every chapter-specific mark
has drifted:

| bench | production (`first_shift/marks.rs`) |
|-|-|
| `CRATE_POSITIONS` (2800,20,-3800), (2300,20,-4250), (1700,20,-4400) | (-200,-60,-1400), (200,100,-3000), (-600,-400,-1400) |
| crate `area_radius: Meters(80.0)`, `pickup_sound: None` | `CRATE_AREA_RADIUS = Meters(15.0)`, `Some(salvage_pickup.wav)` |
| `WARSHIP_POS` (7900,250,-6500) | `WARSHIP_HIDE_POS` (8400,250,-6500) |
| `EMERGENCE_BEACON_POS` (7900,650,-6500) | `WARSHIP_EMERGE_POS` (7600,300,-3200) |
| `APPROACH_POS` (-2500,700,-5700) | `ORBIT_RETURN_GATE_POS` (-3922,-334,-5014) |
| `FLIGHT_BEACON_POS` (0,100,-900) | the RCS box at (-500..-200, 80..300, 900) |

Note the crate `area_radius: Meters(80.0)`: that is the pre-`99b69dbe` value the
changelog says shrank to 15 m. Not a lost guard - production has its own corridor and
crate-clearance tests - but the module doc's claim that the bench "exposes the whole
tutorial route at once" is false at HEAD, and its assertion is now about nothing. Same
class one level up from batch 9's belt-copy finding. `second_shift_map` also draws 28
unique fragment hulls where production repeats four.

**MINOR - `crates/nova_probe_cli/tests/catalog_drift.rs:598` - the roster gate only scans a range's ROOT file, so a marker in a submodule drifts in silently.**
Verified at HEAD: `let path = root.join("examples/systems").join(format!("{example}.rs"));`
reads exactly one file, while five ranges compile submodules in - `system_ship_editor`,
`system_input_modes`, `system_ui_scale`, `bug_sandbox_soak` (`shared/editor_stage.rs`,
`shared/editor_walk.rs`, `shared/section_aim.rs`) and `system_turret_gunnery`
(`system_turret_gunnery/slider.rs`). A new invariant asserted in `shared/editor_walk.rs`
- the natural home for one more editor-walk check, in the very range that is RED today -
would pass both the roster gate and the run. The other direction stays fail-safe.
The hole is reachable but currently unrealized: no marker lives in a submodule today.
Fix: scan the root plus every `#[path]` module it pulls in, or glob
`examples/systems/**` and attribute each file to its root.
Two smaller mechanism notes, both read: `marked` and the roster are compared as
`BTreeSet`s while `SYSTEMS_INVARIANTS` sums `names.len()`, so a slug duplicated inside
one roster entry inflates the count without touching the set (no range does today,
verified across all 41). And nothing at runtime requires a roster marker to FIRE -
`invariants_held` counts `kind == "invariant"` violations, never `kind == "marker"`, so
the only thing binding a slug to an executed assert is each range's hand-wired latch
plus a beat deadline. The pairing is per-range, not systemic.

**`568149b2` itself is CORRECT, and the context is worth recording.** The three restored
slugs match the three `probe_marker` calls, and 220 -> 223 is the right delta. VERIFIED
BY RUNNING `probe run system_turn_limit`: all eight checks pass
(`invariants_held PASS 0 violations over 552 checked frames`) and the timeline carries
exactly the three markers.
The context: `system_turn_limit` entered the catalog at `2fb42ef9` with no roster, so
`systems_ranges_assert_their_invariant_roster` was RED for the twenty commits between
then and this fix. **The gate works; nobody was running it.** That is the same reason
the batch-1 roster drift is red at HEAD, and it belongs in the verdict.

**Not checked on the correctness side.** Frame cost of all three benches is UNMEASURED -
they declare no `nova_frametime`, so probe emits no `frametime.csv` for them. The lane
ran each for 90 s under Xvfb and confirmed clean startup (no panics, no ERROR, 8 and 34
ship skins spawned) but produced no numbers, and says the host was not fully quiet
(load ~2.1 from its own builds) so it would not have quoted one. `probe run
first_shift_map` was not run - the third has identical structure and the same empty
harness. The `--pilot cutter` path was not exercised. The 200x railgun override was not
fired. Only one of the 41 systems ranges was run against its roster, so finding 5 is
argued from the scan path, not a full sweep.

### Batch 12 - HUD, comms, and campaign portraits

`8d558cd4` `80d8a237` `6703c967` `758edb7f` `22e00993` `9d25712d`, 873 lines.
Both lanes ran. Lane A drove a live example under Xvfb; lane B ran scoped
`nova_hud` and `nova_authoring` tests. Every finding below was re-checked
against HEAD in the main session.

**MAJOR - every campaign portrait renders as an empty tile in the code-built
scenario path.** `crates/nova_authoring/src/base_content/scenarios/nova_protocol/cast.rs:56-66`
hardcodes the mod sentinel:

```rust
fn portrait(speaker: &str) -> Option<AssetRef<Image>> {
    let path = match speaker {
        CONTROL => "self://portraits/meridian-control.png",
```

`self://` is not a Bevy asset source. `crates/nova_assets/src/mod_refs.rs:5-7`
states it is "rewritten away before the path ever reaches the `AssetServer`",
and that rewrite runs only on content merged from a bundle. `first_shift_scene`
(`first_shift/mod.rs:1266-1270`) builds the `ScenarioConfig` in Rust and
triggers `LoadScenario` directly, so the literal string reaches
`AssetRef::resolve` -> `asset_server.load(path)`.

The same function already respects this invariant for its other two assets: it
takes `cubemap` and `asteroid_texture` as PARAMETERS so `content gen` can pass
`self://` (`base_content/assets.rs:210-212`) while the preview passes live
handles (`examples/playable/shared/first_shift_scene.rs:40`). The portraits got
no such plumbing.

VERIFIED LIVE. `DISPLAY=:99 ./target/debug/examples/first_shift_01_departure`:
`ERROR bevy_asset::server: Asset Source 'AssetSourceId::Name(self)' does not exist`,
4525 occurrences in ~17 s of comms, ~50/s per visible card at ~49 fps, because
`sync_comms_cards` re-resolves the handle every frame. The capture shows a bare
blue-bordered empty square where the portrait belongs. Reproduced in a second
lane's captures of `first_shift_01_departure` and `first_shift_03_salvage`.

The SHIPPED path is fine: `register_bundles` -> `rewrite_refs` is a generic
string walk that reaches `icon` inside `Sequence` steps, all seven portraits are
declared at `assets/base/base.bundle.ron:130-136`, and all 79 `StoryMessage`
entries in the generated RON carry `icon: Some("self://portraits/...")`. What
breaks is the nine `first_shift_0*` / `second_shift_map` playtest examples - the
same harness this batch's own rendered-review screenshots came from.

Fix: plumb the portraits like the cubemap. Add the seven images to `GameAssets`
(they are in the bundle manifest but not the boot collection), pass a portrait
set into `first_shift(...)`/`second_shift(...)`, have `BaseContentAssets::from_paths()`
supply the `self://` strings and `first_shift_scene` forward live handles.

**MAJOR - the only regression pin `758edb7f` left behind cannot fail.**
`crates/nova_hud/src/comms_panel.rs:721-737`, `comms_arrival_keeps_every_card_at_layout_size`,
reads the scale off `UiTransform`:

```rust
let mut cards = world.query_filtered::<Option<&UiTransform>, With<CommsCardMarker>>();
let scales: Vec<Vec2> = cards.iter(world)
    .map(|transform| transform.map_or(Vec2::ONE, |transform| transform.scale))
    .collect();
assert_eq!(scales, vec![Vec2::ONE, Vec2::ONE]);
```

`HudEmphasis` carries `#[require(UiTransform)]` (`emphasis.rs:50`), so re-adding
it gives the card a DEFAULT `UiTransform` at scale `ONE`. The only writer of
that scale is `drive_hud_emphasis` (`emphasis.rs:182-188`), and `comms_app()`
(`comms_panel.rs:412-421`) registers only `enqueue_new_lines`,
`drive_comms_stack`, `sync_comms_cards`. Restoring the arrival pop would leave
both scales at `ONE` and the test would still pass. The test it REPLACED
asserted on `HudEmphasis::scale()` directly, which needs no driver - the guard
got weaker exactly where it claims to be a pin. Verified by reading and by
running (`cargo test -p nova_hud --lib comms_panel`, 10 passed).

Fix: assert `Option<&HudEmphasis>` is `None` on every `CommsCardMarker`, or add
`drive_hud_emphasis` to `comms_app()`.

**MAJOR - no changelog entry for the comms queue becoming lossless.**
`80d8a237` deleted the SHIPPED `COMMS_QUEUE_CAP` drop-oldest behaviour
(`git show v0.12.0:crates/nova_hud/src/comms_panel.rs` has it at `:75` and
`:248`). The same commit rewrote the creator contract in two places -
`docs/scenario-system.md:347-351` ("a lossless pending queue behind them",
replacing "oldest dropped when the backlog overflows") and
`web/src/create/actions.md:277-279` ("Pending lines wait without being
dropped") - but the entry it filed (`CHANGELOG.md:104`) covers only card width,
text scale, the arrival pop and objective sizing. Grepping `[Unreleased]` for
`comms|lossless|pending|queue` returns only the portrait and card-size entries.
A creator reading the release notes will not learn that a six-line burst no
longer loses its first two lines. `:104` is already 220 characters (recorded
range-level below), so this needs the entry SPLIT, not extended.

**MINOR - `sync_comms_cards` rebuilds the whole card tree every frame.**
`comms_panel.rs:274-296` despawns and respawns unconditionally, with no change
guard:

```rust
commands.entity(entity).despawn_related::<Children>();   // :283 unconditional
if queue.visible.is_empty() {
    *visibility = Visibility::Hidden;                     // :285 marks changed every frame
```

Per frame with three cards: 15 despawns + 15 spawns, six `String` allocations
(`speaker.to_uppercase()` `:334`, `text.clone()` `:340`), three
`AssetServer::load` calls (`:372`). The `load` calls are NEW to this batch -
before `6703c967` every campaign line was `icon: None` and took the fallback
branch. The unguarded `Mut<Visibility>` write runs for the whole game, forcing
propagation every frame the panel is empty; `screen_indicator.rs:587` already
uses `set_if_neq` for the same job. Frame cost UNMEASURED; the measured proxy is
the 100 error-lines/s above, which the MAJOR's fix removes.

Fix: `run_if(resource_changed::<CommsQueue>)`, `set_if_neq` on the visibility
writes, and resolve each icon once on promotion into `visible`.

**MINOR - `CommsQueue::pending` is now unbounded with no back-pressure.**
`comms_panel.rs:233` pushes every new line with the cap gone. Drain rate is
fixed at `COMMS_VISIBLE_CAP` (3) x 8.4 s occupancy ~= 0.36 lines/s and nothing
shortens it. Lane A simulated the shipped campaign against the generated RON:
the largest sequence is 15 lines at gaps `[2,4,4,6,3,2,2,2,3,4,6,2,2,2,4]` s,
peaking at ONE pending line and 4.4 s of lag - the old cap of 4 was never
reached, and only the two `OnStart` handlers are non-`once`, so there is no
repeating producer today. The exposure is mod-facing, and both creator docs were
edited this batch to promise "lossless".

**MINOR - `COMMS_MIN_SECS` documents a pressure valve the panel never
implements.** `comms_panel.rs:64-68` says it is "the floor a showing line holds
even with lines waiting", but `VisibleCommsLine::dwell_secs()` (`:130-136`) and
`expired()` (`:148-150`) never consult `queue.pending`. `grep -rn
COMMS_MIN_SECS` returns the declaration, the prelude re-export, and two
consumers in `nova_authoring/.../pacing.rs` - no read inside `nova_hud`. Dead
since `54ebcc2a`, so pre-existing; it matters now because this batch deleted the
cap and rewrote the docs to lean on the queue as the safety net. Deleting the
constant would change authored beat timing, so the honest fix is to implement
the floor.

**MINOR - two stale "12 px" citations for a 16 px label.**
`crates/nova_hud/src/objective_markers.rs:330` ("thinning 12 px text to 0.7
alpha broke readability") and `:457` ("12 px gold at 0.7 alpha") against
`LABEL_FONT_PX = 16.0` at `:44`. The same commit corrected the identical claim
at `:138`.

**MINOR - the removed ghost's history is written into two doc comments.**
`crates/nova_hud/src/objective_feedback.rs:10-14` and the test docstring at
`:324-328` both narrate the feature that is gone. AGENTS.md: keep module
comments short, explain ownership and constraints, not history. The preceding
"Sound is ALL it owns" lines already carry the constraint.

**MINOR - undocumented override of the shared chip geometry.**
`crates/nova_hud/src/objective_stack.rs:392-394` mutates `chip_node()`
(`nova_ui/src/hud.rs:113-124`, padding `9x4`) to `max_width: 80%` and padding
`12x7` with no reason given, while commenting a much smaller decision two lines
below at `:402`.

**MINOR - portrait coverage is guarded by a hand-maintained list.**
`cast.rs:56-65` ends `_ => return None` and `apply_portrait` leaves that card on
the HUD fallback. `every_campaign_voice_has_a_portrait` (`:101`) iterates the
eight `cast.rs` constants, so a new voice - or a speaker typed as a literal -
silently loses its portrait. Coverage is complete today (69/69 icons in
`first_shift.content.ron`, 10/10 in `second_shift.content.ron`, seven paths, all
declared, all on disk at 512x512). This is also why the test could not catch the
MAJOR above. Fix: assert over the BUILT events, not the constant list.

**MINOR - a new committed-art generator with no docs section and no `--check`.**
`scripts/generate-campaign-portraits.py` writes seven committed SVGs and seven
committed PNGs and is documented only in `art/portraits/README.md`. Every
sibling gets a `docs/development.md` section with a verify mode
(`gen-scenario-thumbnails.py` `:629-646`, `gen-greebles.py` `:648+`,
`gen-web-screenshots.py` `:580+`). Nothing proves the committed PNGs still match
the SVG source, and it shells out to `magick`, whose output is version-sensitive.
`magick` itself IS in the dev shell (imagemagick-7.1.2-27), so that part is not
a finding.

**Out of batch, noted:** `crates/nova_scenario/src/world.rs:205` deep-clones the
whole story log BEFORE its length guard, on a system that runs essentially every
frame of a live scenario. `6703c967` added an `AssetRef::Path(String)` to all 69
First Shift messages, so the clone got strictly worse. Unmeasured, small, and
the fix is free: move the clone inside the guard.

**Lane disagreement, resolved:** none. Lane B read the queue change as a
changelog omission and lane A as a missing bound; both are true and both are
recorded.

### Batch 13 - the web comic player

`97ffc7b6`, 2274 lines, SOLO. Both lanes ran. Lane A built the site, ran the
suite, and drove the reader in headless Chromium; lane B checked the comic
against the shipped campaign. Every finding below was re-checked at HEAD.

**Build and tests are GREEN.** `npm test`, `npm run lint`, `npm run
format:check`, `npm run build` (3.2 s) all pass. Live-verified in headless
Chromium against a served `dist/`: five articles render, four carry `hidden`,
the deep link `#the-record` selects page 05, the counter reads `05 / 05`,
`data-page-next` disables at the end, and the end-page action resolves to
`/play/`. `relativePageIndex` clamps page bounds and every entry point routes
through it, so no out-of-range index is reachable. `loadComicPages` throws
rather than dereferencing `undefined`. No listener or observer leak.

**MAJOR - the build validator accepts page paths the runtime loader can never
resolve, so a green build publishes a dead comic.**

```js
// web/comic-build.js:33
const pathPattern = /^[a-z0-9][a-z0-9/-]*$/;   // slashes allowed, "pages/" not required
// web/comic-build.js:68-78 - then only checks the .ts file EXISTS on disk
// web/src/comics/comic-catalog.ts:37
const pageModules = require.context(".", true, /\/pages\/[^/]+\.ts$/);
```

VERIFIED BY RUNNING. Lane A built a copy of `web/` with a page authored as
`source: "extra/foo"`, file present: `discoverComics()` passes, the entry is
emitted into `#comic-definition`, and the bundle contains exactly the five
`nova-protocol/pages/*.ts` keys - `extra/foo.ts` is absent. Regex behaviour:
`./good/pages/a.ts` true, `./good/extra/foo.ts` false, `./good/pages/sub/b.ts`
false (nested under `pages/` also fails). At runtime `loadComicPages` throws
`Comic page module is missing`, and `web/src/story.ts:12` calls it at top level
with no catch, so `ComicPlayer` never constructs: the viewport stays empty while
the toolbar still reads `Page 01 / 06`.

`docs/development.md:1029-1030` promises the opposite twice - "Webpack discovers
these manifests and fails on invalid or duplicate ids, missing page modules, and
missing cover art" and "Adding a comic therefore needs only its directory,
manifest, TypeScript page modules, and art" - and never states the undocumented
`pages/`-exactly-one-level-deep requirement. House rule is an error at lint then
at load; this errors at neither. Fix: enforce `/^pages\/[a-z0-9][a-z0-9-]*$/` in
the validator, share the literal with the loader, and document `pages/`.

**MAJOR - the comic gives Meridian Control a line the game gives to the player,
inverting the chapter's point.** `web/src/comics/nova-protocol/pages/the-same-belt.ts:39-42`:

```ts
speech("control", "Meridian, Cutter One. I have your beacon. I am coming in."),
```

Shipped scenario, `second_shift.rs:519-528`, with its own comment saying why:

```rust
// Nobody talks back this time. The opening is one voice in
// an empty channel, which is the difference between the two
// chapters stated before anything is flown.
sequence(SEQ_OPENING, vec![
    open_line(OPEN_1_AT, PLAYER, "Meridian, cutter one. I have your beacon. I am coming in."),
```

`PLAYER` is the cutter's captain (`cast.rs:31-37`); `CONTROL` is Meridian
Control, whose silence IS chapter one's ending - and the comic's own previous
page states it (`no-fleet-code.ts:43`: `["MERIDIAN CONTROL", "NO CARRIER", ...]`).
The renderer acts on the string: `comic-renderer.ts:110` adds
`speech--control`, and `style.css:3257` moves that bubble to the left, the same
side chapter one's carrier bubbles sit on. The page reads as the dead carrier
answering. Fix: `speech("captain", ...)`, as `released.ts:66` and
`no-fleet-code.ts:34-37` already do for player lines.

**MAJOR - the reader prints a release fact derived from the chapter count, and
it is false.** `web/comic-build.js:194`:

```js
<span>Current release</span><strong>${comic.chapters.length} playable chapters</strong>
```

`comic.json` has two chapters, so every reader page states "Current release: 2
playable chapters". The last release is `[0.12.0] - 2026-08-31`; both chapters
are in `[Unreleased]` (`CHANGELOG.md:74`, `:80`). The current release has zero
of them. The manifest has an explicit `status` field but nothing for this
number - a silent default over an unrelated quantity. Fix: author it as a
validated manifest field, or drop the pair.

**MINOR - a chapter or page with NO `id` field passes validation**, because
`RegExp.test(undefined)` tests the string `"undefined"`. `comic-build.js:46`
and `:62`. VERIFIED BY RUNNING: a manifest with both `id` keys deleted returns
PASS; the fragment becomes `#undefined`, and a second id-less entry is then
rejected as "invalid/duplicate", which is the wrong message. Exactly the case
AGENTS.md names. Fix: `typeof chapter.id !== "string" || !idPattern.test(...)`.

**MINOR - two more explicit-authoring gaps in the same validator.** Both lanes
found `coverAlt` independently: `comic-build.js:101` falls back to
`comic.title` while `validateComic:35-39` requires only `title`, `summary`,
`status`, `cover`, and `docs/development.md` never mentions the field. And the
comic's DIRECTORY NAME is its public id - URL segment, asset path, dev-server
rewrite - yet is never pattern-checked, unlike chapter and page ids. VERIFIED BY
RUNNING with a directory `Nova Comic`: the build passes and emits
`href="/story/Nova Comic/"` unencoded, plus `new RegExp("^/story/Nova Comic")`
at `webpack.config.js:425`; a directory containing `.`, `+` or `(` corrupts or
throws there.

**MINOR - the new pipeline is gated by nothing.** `.github/workflows/ci.yaml`
triggers on `push: [master]` and `pull_request` with jobs `check, probe,
autopilot, default-features, wasm, licenses` and contains no `npm`, `node` or
`web` step at all. The only place `npm run build` runs is
`deploy-page.yaml:44-45`, whose sole trigger is `workflow_dispatch`, and it runs
`build` only - never `npm test` or `npm run ci`. So `comics.test.js` and the
manifest validation never block a merge, and the validator's first execution is
a manual deploy. The gap is pre-existing for `web/`; this batch is the first to
put load-bearing content validation behind it. Fix: a `web` job on
`paths: [web/**]` running `npm ci && npm run ci`.

**MINOR - rendered output: at 390 px the speech bubbles cover their art, and one
reused SVG bleeds the next page's title.** VERIFIED BY SCREENSHOT at 390x844 and
1440x900. The mobile block keeps `.comic-grid` at two columns
(`style.css:3474`), giving ~86 px half-panels, while `.speech` is
`max-width: min(320px, 72%)`; the copilot line wraps to six lines and fills the
panel, so `trimRoute` and `orbit` are invisible. On desktop the same two SVGs
are roughly half-occluded. Separately `first-shift.svg` bakes in
`<text>ROUTINE / RELEASED</text>` and `<text>NO FLEET CODE</text>` and is the
panel image on THREE pages under `object-fit: cover`, so on page 02 "Released"
and in the `/story/` index thumbnail a clipped `NO FLEET CODE` - the next page's
title - is legible in the art.

**MINOR - three variant/focus ids are unreachable or unstyled.** Both lanes.
`PanelVariant` (`comic-page.ts:3-10`) forbids `attack-main` and `wreck`, yet
`style.css:3227,3228,3234,3237` style `.comic-panel--attack-main` and
`.comic-panel--wreck` - dead selectors. In the other direction `work`, `orbit`,
`silent` and `contacts` are authored across the pages and have ZERO CSS rules,
and `the-same-belt.ts:37` authors `focus: "wreck"` where `style.css:3355-3358`
defines only `carrier` and `warship`. Fix: delete the dead selectors; make
`focus` a closed union so an unrecognized value fails at `tsc`.

**MINOR - `speaker` is write-only except for one magic string.**
`comic-renderer.ts:103-111` sets `speech.dataset.speaker`, which no CSS selector
reads, and only the literal `"control"` changes anything. A reader never sees
who is talking, and a typo silently gets the default bubble - which is how the
MAJOR above would have been caught.

**MINOR - the trim-route diagram's corner letters are one leg off.**
`released.ts:15-30` places `A=[55,135] B=[245,135] C=[245,45] D=[55,45]`, making
A->B the lateral leg; the game's box is start->A lateral, A->B up, B->C lateral,
C->D down (`first_shift/story.rs:59-80`, D is the start). A player who flew it
sees a different box.

**MINOR - no docstrings on the new public API.** `comic-page.ts` 20 exports and
0 JSDoc, `comic-renderer.ts` 2/0, `comic-catalog.ts` 4/0, `story-reader.ts` 3/1.
`comic-page.ts` is the surface `docs/development.md` sends comic authors to.

**MINOR - nothing tests that the validator REJECTS.** All five assertions in
`web/tests/comics.test.js` exercise the happy path, so the guarantee
`docs/development.md:1029` advertises has no coverage - which is why both id
gaps above survived. `comics.test.js:13` asserts the absence of a field no code
path ever writes.

**MINOR - changelog wrap.** `CHANGELOG.md:225` is an 86-character raw line where
every neighbour is <= 79. The entry is 161 characters joined and correctly
placed under `### Web & Platform`; only the wrap is wrong.

**MINOR - craft.** `released.ts:22-27` rebuilds the four-corner `points` array
inside a `.map` callback, duplicating coordinates hard-coded in the four `line()`
calls above. `comic-catalog.ts:40` `documentRoot: Document = document` is a test
seam no test uses. `style.css:3459` and `:3467` are two `.comic-page__header`
rules in the same media block. `the-record.ts:6` puts author's-note voice into
player-facing copy: "It does not turn an industrial crew into galactic saviors."
`comic-build.js:29` sorts comics by title with no authored ordering, and `:105`
hard-codes the plural ("1 chapters").

**Verified correct, worth recording:**

- **"Extensible" holds.** Adding a comic touches three new paths and edits ZERO
  central files: `comic-build.js:14-30` `readdirSync`, `webpack.config.js:319-320`
  and `:412-416`, and `comic-catalog.ts:35` `require.context` all derive from the
  directory. `package.json` needs no edit.
- **Mobile CSS landed in the right place.** All 613 lines were appended (0
  deletions) and the comic's `@media (max-width: 760px)` block at
  `style.css:3431` is the LAST media block in the file, so the known
  later-base-rule hazard is avoided and no existing rule was touched.
- **Navigation and the wiki link resolve.** `_header.html:6` and `_footer.html:5`
  both add `Story`; `wiki/scenarios.md:33` `[Story](../../story/)` resolves
  correctly under any `PUBLIC_PATH`.
- **Canon that does match:** `released.ts:55` is `OPEN_CONTROL_CLEAR` verbatim;
  `no-fleet-code.ts:15` "two railgun strikes" matches `first_shift/mod.rs:23`;
  `the-same-belt.ts:47` lists the three recorders in `second_shift.rs:65-80`
  order; `the-same-belt.ts:16` five cleanup contacts matches `cleanup_group()`.
- **Committed research is not stale.** `STORY_BIBLE.md` events 1-9 match the
  shipped scenarios; `CAMPAIGN_OUTLINE.md` marks chapters 3-5 "Unresolved". The
  task carries one scheduling tag. No `any` in any new TypeScript file.
- **Performance: nothing actionable.** Both SVGs are 3.5 KB and 2.9 KB, the four
  `<img>` tags resolve to two files, only the current page is un-`hidden`, the
  wheel handler self-throttles with a 180 ms lock, and there is no JS resize or
  scroll listener. Latent only: `require.context` pulls EVERY comic's pages into
  the single `story` chunk (13.4 KB of 152 KB today), so the index downloads
  pages it never shows and an Nth comic costs every reader page.

**Lane disagreement, resolved:** none. Both lanes found the validator/loader
mismatch independently; lane A's build proved it ships green, so it is recorded
at lane A's severity.

### Batch 14 - the Kenney fleet move and infrastructure

`138dfbfc` `db060c1e` `6a24692c` `dfb60ab2`, 4877 lines. Both lanes ran. Lane A
linted the whole tree, installed both webmods into a scratch portal cache, and
flew four Ledger scenarios plus the Gauntlet under Xvfb. This is the
best-verified batch in the range and it has NO blocker and no correctness MAJOR.

**The move works, proven not assumed.** `content lint` over the whole tree
returns 0 errors, 0 warnings, 0 findings across 10 scenarios and 15 creative
maps - and lane A proved the lint is NON-VACUOUS by injecting
`dep://base/gltf/parts/racer/nose.glb` into a scratch copy of The Ledger, which
produced `section references undeclared resource ... of dependency 'base'`.
Every `dep://` from `webmods/` resolves to something base still ships. All 24
prototypes `ledger_ships.content.ron` names exist, the one exception
(`pdc_kinetic_turret_section`) being base's. All 21 `self://gltf/parts/**` refs
are declared and on disk. A repo-wide grep for `racer|cargo_a|cargo_b|cargoa|
cargob|gltf/parts` finds nothing in `crates/`, `assets/` or `examples/` outside
the two art-review examples, which read `art/part-candidates/` - so the commit's
"nothing in `crates/` names a Kenney craft any more" holds. Live: CargoA renders
with its GLB meshes in `ledger_ch1_dead_weight`, CargoB in
`ledger_ch2b_the_heavies`, plus `ledger_ch5_the_raid` and `gauntlet_run`, all
with zero ERROR lines. `content_ron_parity` 2/2. Portal publishing handles the
new nesting (40 files including all 21 nested GLBs). Renamed fixtures all pass:
`nova_console` 14/14, `nova_os` 56/56, `nova_editor` 437/437, `nova_assets --lib
merge` 9/9.

**`db060c1e`'s audio fix is correct AND its test can fail.** Lane A built a
standalone bevy_ecs 0.19.1 reproduction of the same-flush race: `try_insert` ->
OK, `insert` -> PANIC, because `EntityCommands::insert` queues through the
fallback handler and `BevyError`'s default severity is `Severity::Panic`. The
remaining `despawn` calls at `voice.rs:275`, `:302`, `:499` route through `warn`,
so a double-despawn logs rather than crashes. 14/14 `audio::voice` tests pass.

**MAJOR - `CHANGELOG.md:219` tells players to install mod versions that no
longer work against this build.**

```
- The Ledger (1.27.0) and Gauntlet Run (1.11.0) are republished in meters.
  Update both: a portal mod installed on 0.12.0 carries the old numbers, and
  this build reads them as meters.
```

At HEAD `the-ledger.bundle.ron:73` is `version: "1.28.0"` and
`gauntlet.bundle.ron:16` is `version: "1.12.0"`. A player who follows this line
installs a Ledger whose ships name `racer_fuselage`, `cargoa_*`, `cargob_*` -
prototypes base no longer holds - which is exactly the failure the mod's own
1.28.0 entry says the bump exists to prevent. The batch added two new Modding
entries at `:211-215` and left the migration instruction pointing at the
superseded pair. This is the collapse rule: the cycle republished each mod twice
and the entry must name the FINAL version. Fix: `The Ledger (1.28.0) and
Gauntlet Run (1.12.0)`.

**MAJOR - the wiki still calls The Ledger's corvette "the shipped corvette",
and one page contradicts itself.** The same commit rewrote
`web/src/wiki/sections/controller.md:27` to "The corvette the widgets fly is The
Ledger's ... base references none of them", but `:35` still opens "The shipped
corvette carries its mass over a 27.6 m arm". `web/src/wiki/flight-autopilot.md:39`
has the same phrase. Base ships no corvette: `assets/base/ships/base.content.ron`
holds only the fifteen `block_*` ids, and grep for `racer|cargoa|cargob` over
base's ships AND sections RON returns 0 and 0. Two more carriers of the same
phrase were found in the main session and are part of the same fix:
`docs/sections.md:65` and `crates/nova_ship/src/physics/attitude.rs:150`.

**MINOR - `crates/nova_info/build.rs:36-48` still freezes the hash when the
branch ref is packed.** The commit's whole point is that a commit must move a
watched path, but the loose branch ref is only registered `if path.exists()`:

```rust
let mut paths = vec![git_path("HEAD"), git_path("packed-refs")];
if let Some(head_ref) = git(["symbolic-ref", "--quiet", "HEAD"]) {
    paths.push(git_path(&head_ref));
}
for path in paths.into_iter().flatten() {
    if path.exists() { println!("cargo:rerun-if-changed={}", path.display()); }
}
```

After `git pack-refs --all` - which `git gc`/`gc.auto` runs unprompted -
`.git/refs/heads/master` does not exist, so the guard drops it. A commit then
creates a LOOSE ref nothing watches, while `.git/HEAD` and `packed-refs` are
untouched; having emitted no dirty path the script never re-runs to register it.
Permanent freeze - the exact bug being fixed. VERIFIED BY RUNNING in a scratch
repo: with a packed ref, both `.git/HEAD` and `.git/packed-refs` were byte- and
mtime-identical across a commit that moved HEAD `d424f3b` -> `0dd828d`. A fresh
clone leaves the branch loose, so the common case works today; this bites
long-lived clones and gc'd CI caches. Fix: also watch `git_path("logs/HEAD")`,
whose content changes on every commit even with a packed ref.

**MINOR - the Gauntlet's hull swap tripled the ship's mass on identical thrust,
and the mod changelog documents only its beam.**
`webmods/gauntlet/gauntlet.content.ron:137` is now `hull: Prototype("block_cutter")`
where the racer's seven-section hull was inlined. Section mass is collider volume
at density 1 (`base_section.rs:470`):

- racer, 7 modelled parts, summed authored cuboids: **8.284**
- `block_cutter`: **26** unit-cube sections (23 `reinforced_hull_section`, 2
  `basic_thruster_section`, 1 `basic_controller_section`) - verified in the main
  session - PLUS a clad skin (`skin: true, style: Some("industrial")`), which the
  ledger hulls do not carry.

Both craft carry exactly two thrusters at `magnitude: 1.0`. `manual.rs` divides
summed forward magnitude by hull mass, so main-drive acceleration drops by at
least **3.14x**: roughly 5.4 s to the 250 m/s cap where the racer took 1.7 s, on
a 6.15 km course. `speed_cap: Some(250.0)` is unchanged so top speed is not
affected; attitude authority is worse still through the moment of inertia.
`webmods/gauntlet/CHANGELOG.md` 1.12.0 mentions only "a hull about twice the
beam, so the slaloms read a little tighter".

The racing-line margin still holds by arithmetic (cutter half-beam ~26 m against
the racer's ~12 m, tightest rock ~90 m past the old margin, leaving ~76 m) - but
NOTHING CHECKS IT ANY MORE. The `gauntlet_course` rig that
`gauntlet.content.ron:21` and `:30` say "asserts pairwise non-overlap" and
"measures this gap for every rock" does not exist at HEAD; confirmed in the main
session, `grep -rl gauntlet_course --include=*.rs` returns nothing, and no Rust
reads `webmods/gauntlet/gauntlet.content.ron` at all. Lane A could not fly the
lap (no input harness), so the handling numbers are arithmetic, not a timed lap.
Fix: re-playtest and record a target time, or restore a rig that pins the gate
geometry against whatever hull the scenario names.

**MINOR - `crates/nova_ship/src/sections/turret_section/aim.rs:22-25`, the
re-justified constant matches neither figure its own doc names.**

```rust
/// Half the beam of a shipped corvette, in world units: the gunship's spine
/// spans x -1.5..1.5, a 30 m core hull, with sponsons reaching 50 m across at
/// midships. What a round has to land inside to hit a ship at all.
pub const HULL_HIT_RADIUS: f32 = 1.6;
```

1.6 was the cargoa's exact half-beam. Rewritten for the gunship it is neither
1.5 nor 2.5, and "half the beam of a shipped corvette" is now false for every
base ship. Behaviour is unchanged and safe - `TURRET_ON_TARGET_RAD` only makes
turrets hold fire slightly longer. Both lanes found this.

**MINOR - `crates/nova_console/src/lookup.rs:133` and `completion.rs:41-42`
claim a section id no shipped raider carries.** The docstring says
"`block_gunship` and `block_raider` both carry a `pdc_aft_port`"; `block_raider`
mounts only `pdc_dorsal` and `pdc_boom`. The pre-move text (`cargoa` /
`cargoa_raider` both carrying `turret_port`) WAS true of shipped content; the
replacement is not.

**MINOR - `crates/nova_editor/src/gallery/catalog.rs:76` documents a prototype
id that exists nowhere.** It cites `hauler_nose` as the example of a part id
carrying its ship family; 0 hits in base's sections RON and 0 in The Ledger's.
After this batch no BASE prototype carries a ship family in its id at all - the
only real example is The Ledger's `cargoa_nose`, i.e. a mod convention.

**MINOR - the `standard.rs:NNN` citations this batch RE-POINTED are still
wrong.** The commit re-numbered after removing 35 lines but the new numbers do
not land on the cited code: `controller.md:27` cites 376 for `max_torque: 1501.0`
which is at `:718`; `railgun.md:109` cites 957 for `rake_radius` at `:920`;
`torpedo-bay.md:35` cites 1207 for `projectile_health` at `:1266`;
`combat-weapons.md:24` cites 1180 for `projectile_lifetime` at `:1239`; and
`widgets.ts:210,211,235,236,237,238` are ALL off by exactly 59. The PDC and hull
citations the same commit touched ARE correct, so this is a partial re-point,
not wholesale drift - the delta was subtracted rather than the numbers
re-derived. `torpedo-bay.md:35` and `:141` now disagree with each other inside
one file.

**MINOR - `docs/sections.md:545` is ungrammatical from the search-and-replace:**
"Without it the the a modelled pod faced its fuselage 36 degrees off -X".

**MINOR - two module comments carry task ids.**
`base_content/sections/mod.rs:6` and `ships/mod.rs:7` both write "(The Ledger,
task 20260824-125959)" into a module doc.

**MINOR - `base_content/sections/mod.rs:28` is now a pure alias.**
`section_catalog` existed to join the standard and semantic halves; with the
semantic half gone it forwards verbatim to `standard_section_prototypes`, which
at `:23` is itself a one-line forward to `standard::standard_section_prototypes`.
Three names for one call chain.

**MINOR - the live widget text keeps the pre-move framing the `.md` fallbacks
lost.** `web/src/widgets.ts:8119` calls `host.replaceChildren()` before
hydrating, so the corrected static prose in `hull.md:51` and `thruster.md:24` is
only ever seen with JS off. The JS that replaces it carries no provenance:
`widgets.ts:6287-6289` lists the rigs as bare `racer`/`cargoa`/`cargob` and the
readout at `:6621-6631` says "Ranked by health alone the CargoA Nose..." with no
mention of a mod.

**MINOR - `dfb60ab2`'s rule is not the rule the code practices for `style`.**
`AGENTS.md:27-29` says an unrecognized id is an error at lint then at load. Ship
ids and section prototype ids honour it (`lint/ship.rs:41`, `:87`). A ship's
`style` id does not: no reference check exists in `crates/nova_scenario/src/lint/`,
`lint_walk.rs:313` registers style ids only for duplicate detection, and
`skin_style.rs:435` `ShipStyle::resolve` returns `None` for an unknown id with a
docstring ENDORSING it ("A named style that nothing authored is a MISS, not a
fallback"). The shipped creator doc `web/src/create/ships.md:59` teaches the
exemption to modders. Either the rule needs the carve-out spelled out or `style`
needs a lint arm - as written, the rule and the code disagree, which is what the
commit set out to stop.

**Verified clean, worth recording:**

- **The hand-written example mod is clean.** `assets/mods/example/` has zero hits
  for `racer|cargo_a|cargo_b|cargoa|cargob|kenney`. Both webmods migrated AND
  bumped with a "Required:" note.
- **`credits/CREDITS.md:54-56`** correctly relocates the Kenney CC0 attribution
  to `webmods/the-ledger/gltf/parts/` and states "the base game uses none of
  them"; the 21 GLBs are listed in the bundle's `resources`.
- **The Gauntlet's own claims hold:** `block_cutter` really is 23 x
  `reinforced_hull_section` + 2 thrusters + 1 controller, matching
  `gauntlet.content.ron:11`'s "base's reinforced_hull_section (200 health)".
- **`web/src/create/ships.md:150-166`**'s "Base ships" table matches
  `assets/base/ships/base.content.ron` exactly: fifteen `block_*` ids.
- **`6a24692c`'s feature routing works**, verified by running
  `cargo test -p nova_core --lib a_debug_build_stamps` both with and without
  `--features debug`.
- **No changelog entry for `db060c1e` is CORRECT.** `audio/voice.rs` does not
  exist at `v0.12.0`, so the crash was introduced and fixed inside this cycle.
- **The `--features debug` entry is correctly KEPT**: `84971c67` deleted the
  feature and is contained in `v0.12.0`, so the missing hash shipped.

**Lane disagreement, resolved:** none. Both lanes independently flagged
`aim.rs:22`; recorded once.

### Range-level: the `[Unreleased]` changelog block breaks its own rules

Surfaced while adjudicating batch 8, but not attributable to any one batch - the
entries come from across the range, so the fix belongs to the range.

**MAJOR - First Shift has SIX separate `[Unreleased]` entries for a chapter that has never been released.**
Lines 62, 67, 69, 72, 74 and 77 at HEAD. `AGENTS.md` is explicit: "Collapse several
pre-release revisions of one change into a single final entry. Omit bugs introduced
and fixed inside the same release cycle." First Shift appears nowhere in `[0.12.0]`
(`:342`) - the chapter ARRIVES in `[Unreleased]` at `:74` under `**(breaking)**`. So
`:62` ("moves WORK SITE and the third crate fully clear of Belt Rock 6's..."), `:67`
("keeps control suspended from the warship reveal through teardown"), `:69` ("makes
physical STOP, four framed RCS marks and two completed GOTO legs...") and `:72`
("salvage crates collect only at visible contact") are all revisions of something
that has not shipped, describing bugs introduced and fixed inside this cycle. They
fold into `:74`/`:77`. Two further First Shift entries at `:238` and `:312` are about
the wiki and the scene decomposition, which are genuinely separate changes.

**MINOR - three `[Unreleased]` entries exceed the 200-character cap.**
Measured here with wrapped lines joined: `:69` is 201, `:77` is 201, `:104` is 220.
The cap is "at most 200 characters once wrapped lines are joined".

### Range-level, found in batch 2, not attributable to it

**BLOCKER - `crates/nova_authoring` is red at HEAD.**
`base_content::scenarios::nova_protocol::tests::no_mainline_handler_posts_an_objective_alongside_a_conversation`
fails: "first_shift: handler #0 (OnStart) posts an objective in the same frame as a comms line -
give the objective a beat of its own (`pacing::beat_later`)". Verified in this session by running
it: 0 passed, 1 failed, 109 filtered out. Nothing in `db72da2e` touches handler pacing, so this
belongs to one of the campaign batches; attribute it there.

**NOTE - installed webmod copies of The Ledger fail to load at HEAD** for the batch-1 `material`
break, dropping all six chapters out of the picker. Same cause, different symptom than the one
already recorded.

### Range-level, found in batch 12, not attributable to it

`cargo clippy -p nova_hud --all-targets --features debug -- -D warnings` fails
before it reaches `nova_hud` at all:

```
error: using `contains()` instead of `iter().any()` is more efficient
   --> crates/nova_input/src/source.rs:413:13
    = note: `-D clippy::manual-contains` implied by `-D warnings`
error: could not compile `nova_input` (lib) due to 1 previous error
```

CI runs `cargo clippy --workspace --all-targets --features debug -- -D warnings`
(`.github/workflows/ci.yaml:102`), so this is a hard CI failure. It is NOT from
this range: the line is byte-identical at `origin/master` (`395302b0`, 56
commits back) and came from `fe92322a` "The command console: reach into the game
by name"; `rust-toolchain.toml` is pinned at `nightly-2026-07-03` and unchanged
in the range. Pre-existing red, but it still blocks the range from merging
green. Fix: `names.contains(&wanted)`.

Because `nova_input` aborts the run, lane A's separate report of `this assertion
has a constant value` at `crates/nova_hud/src/ammo_readout.rs:1410` came from a
run WITHOUT `--features debug` and is UNCONFIRMED under CI's exact flags.

## Verdict

Fourteen batches, 56 commits, both lanes on every batch. Roughly 6 BLOCKERs and
50 MAJORs, of which the ones that stop a release are few and concrete.

### The range does not ship in this state

Five things are red at HEAD. Four were introduced by this range:

1. **The workspace does not build.** `examples/playable/planet_types.rs:192`
   passes `material: None` where `AsteroidConfig::material` is `String`
   (batch 1/2). Every `cargo check --workspace` fails, so CI never reaches any
   other job.
2. **`crates/nova_assets/tests/mod_binary_resources.rs:396-404`** - the
   `AsteroidConfig` fixture has no `material` field, so
   `a_nested_dep_ref_is_rewritten` fails with `MissingStructField`. Same root
   cause as (1): `b202d69f` made the field required and missed this fixture.
   Found by batch 14 lane A, attributable to batch 1.
3. **`catalog_drift::systems_ranges_assert_their_invariant_roster`** - two
   unrostered slugs in `system_ship_editor.rs`; `SYSTEMS_INVARIANTS` is 228 and
   should be 230 (batch 1).
4. **Five clippy `-D warnings` dead-code ERRORS** in
   `examples/playable/shared/first_shift_stage.rs`, because `second_shift_map`
   bypasses `stage::belt` (batch 9/11).
5. **`probe run` fails on all three new benches** - `first_shift_map`,
   `first_shift_ships`, `second_shift_map` wire no harness at all and the run
   never exits. They are the only three in the catalog like this (batch 11).

Plus two red test groups: `no_mainline_handler_posts_an_objective_alongside_a_conversation`
(batch 10, exact fix recorded) and two `nova_ship --lib flight::` orbit tests
(batch 3).

And one pre-existing red that this range did not cause but must clear to merge
green: `crates/nova_input/src/source.rs:413` fails `-D warnings` on
`manual_contains`, byte-identical at `origin/master`.

### The gameplay blockers

- **Second Shift's five searchers fly through a planetoid.** All five patrol
  lanes cross the concealment body (radius 2377 m, deepest 1387 m inside) on two
  of three legs, every lap. Its pin checks route ENDPOINTS, never segments. This
  is the single worst defect in the range and it is in shipped campaign content.
- **Scenario orbit progress is gated on a phase the shipped flight tuning does
  not reach** (`loader/trackers.rs:91`), and the First Shift orbit return gate
  covers only part of the reachable ring band (`marks.rs:288`).

### The recurring defect class

Three separate batches produced the same shape independently: **an objective
posted on a delay while its completion trigger is already armed**, orphaning the
card for the rest of the chapter. The STOP beat (batch 8), three RCS box beats
(batch 10), and the reason the naive pacing fix would have reintroduced it. The
mechanism underneath it is batch 7's: `after: Some(0.0)` runs in the SAME FRAME
as the step before it (`world.rs:617` / `actions/sequence.rs:159`), while
`loader/mod.rs:436` documents the opposite. Fix the engine's documented contract
and the class stops recurring.

### The other pattern worth naming

**Regression pins that cannot fail.** `comms_arrival_keeps_every_card_at_layout_size`
reads a scale nothing in its test app writes. `no_sweep_mark_sits_inside_something_solid`
checks endpoints, not segments. `nova_protocol/tests.rs:202` is blind to the only
case it needs. `web/tests/comics.test.js` never tests a rejection. Four batches,
four guards written for a bug that the guard would not catch. Every one of them
was written in the same commit as the fix it was meant to pin.

### What was verified GOOD

Worth stating, because a review that only lists defects misreports the range.
Batch 14 - the largest single commit, moving three ship families out of the base
game - is the cleanest: lint 0/0/0 with the lint proven non-vacuous, every
`dep://` resolving, four scenarios flown live with meshes rendering, the
hand-written example mod clean, both webmods migrated and bumped, and the
credits correctly relocated. Batch 13's site builds, tests and lints green with
the reader live-verified in headless Chromium. The audio-teardown fix in
`db060c1e` is correct AND its test genuinely fails without it, proven with a
standalone bevy_ecs reproduction.

### Recommended order

1. The five reds above, in order - (1) unblocks everything else.
2. The Second Shift searcher lanes, and a pin that checks SEGMENTS.
3. The `after: 0.0` contract, then the three orphaned-objective sites.
4. The portrait `self://` plumbing - it breaks every playtest example.
5. The false records: changelog mod versions, "shipped corvette", the comic's
   misattributed line, "Current release: 2 playable chapters".
6. The rest of the MINORs.

## Fix pass

One pass over the whole range, in the recommended order. 102 files touched, no
new commit: the tree is left staged-free for the owner to split or squash.

### 1. The five reds - all clear

1. **`examples/playable/planet_types.rs:192`** now passes
   `material: KIND_ROCK.to_string()`. `cargo check --workspace` reaches every
   other job again.
2. **`crates/nova_assets/tests/mod_binary_resources.rs`** - the `AsteroidConfig`
   fixture carries `material`, and the `GameAssets` fixture carries the seven
   new portrait handles. The three sibling `nova_assets` test fixtures were
   given the same fields, so the whole test crate compiles.
3. **`catalog_drift::systems_ranges_assert_their_invariant_roster`** - the two
   `system_ship_editor.rs` slugs are rostered and `SYSTEMS_INVARIANTS` is 230.
4. **The five dead-code clippy ERRORs** are gone: `second_shift_map` goes
   through `stage::belt` like every other bench, so nothing in
   `first_shift_stage.rs` is unreachable. `examples/playable/shared/first_shift.rs`
   was deleted with its last caller: `first_shift_ships` now spawns each hull by
   its CATALOG id, so the row poses the shipped ships instead of a second copy
   of their structures that could drift from them.
5. **`probe run` on the three new benches** - `first_shift_map`,
   `first_shift_ships` and `second_shift_map` each wire `NovaProbePlugin` and an
   autopilot script, so the run reaches its window and exits. They grade like
   every other scene bench.

The pre-existing `nova_input/src/source.rs:413` `manual_contains` red is fixed
too (`names.contains(&wanted)`), because the range cannot merge green without
it.

### 2. The gameplay blockers

- **The five searcher lanes.** `no_sweep_mark_sits_inside_something_solid` now
  measures SEGMENT clearance (`segment_clearance`, `second_shift.rs:895`), not
  endpoints. Each lane went from three waypoints to five and the waypoints were
  moved until no leg crosses the concealment body. First Shift's own route pins
  already measured segments - this was Second Shift only.
- **The orbit phase.** `nova_ship` now pins `Hold` as reachable in
  planetoid-strength gravity
  (`a_strong_well_orbit_reaches_the_hold_phase_the_scenario_layer_reads`), and
  `trackers.rs:88` says out loud that mission progress hangs off that one
  phase, naming the pin.
- **The orbit return gate.** `orbit_radius_band` is exported from `nova_ship`,
  and the gate is sized from it: stand-off at the band's midpoint, radius half
  its width plus margin, so it spans 1 473-2 553 m against a band of
  1 511-2 514 m. Was 300 m at a stand-off that covered part of the band, which
  let a player fly a legal higher ring forever with nothing on screen to
  correct them. `the_orbit_return_gate_intercepts_every_ring_the_verb_can_plan`
  pins it.

### 3. The `after: 0.0` contract and the orphaned-objective class

`SequenceStepConfig::runs_with_the_step_before` states the rule in one place,
and `EventConfig::action_groups` groups by it, so a step with no delay is in
the SAME group as its predecessor - which is what the engine does.

Fixing the contract immediately caught the live violation it exists to catch:
`no_mainline_handler_posts_an_objective_alongside_a_conversation` went red on
Second Shift's opening, which had a trailing `step(0.0, ...)`. It now uses
`INSTRUCTION_GAP`.

The orphaned-objective sites are fixed with the file's own precedents, not
with new machinery:

- **STOP** is withheld by `controller_gate` (`DisableVerb(FlightVerb::Stop)`)
  and granted in the beat that posts its card - the same shape RCS and LOCK
  already had.
- **The four RCS box beats** each raise a separate arrival gate in the
  deferred step that posts the card (`TempMark::raise_gate`,
  `gate_entered`, `clear_gated`), so the trigger cannot be armed before the
  objective exists.

`no_arrival_gate_is_raised_before_the_objective_it_clears` walks the handlers
in beat order and pins the class shut. It carries its own non-vacuity
assertion, because that is the defect this review found four times.

A geometry resize was considered for the RCS boxes and rejected: the free
travel the review wanted (400-600 m) needs a 700 m box, which is a design
change. A variable-gated arm was rejected too - `OnEnter` is a `CollisionStart`
edge, so a player already inside would never get the edge and the chapter would
soft-lock.

### 4. The portrait plumbing

`self://` paths in `cast.rs` were the reason every playtest example showed no
portraits: only the RON generator resolves `self://`, and an in-process caller
hands over handles. The seven portraits are now fields on `GameAssets`
(`collections.rs`) and reach the campaign builders as a `CampaignPortraits`
`AssetRef` set.

### 5. The false records

- The comic's misattributed line is the captain's, not control's.
- The changelog's mod versions, "shipped corvette" and the release-count line
  are corrected; the belt-rock, RCS-limit and comms entries were rewritten to
  the 200-character rule; entries were added for `OnOrbitLap` and for the
  `Planet` `invulnerable: true` requirement, both of which reached the range
  undocumented. No entry in `[Unreleased]` is over 200 characters.
- `Cargo.toml`'s `first_shift_ships` comment describes the eight ships that are
  actually in the row.
- `nova_os` command help said `shakedown_run` in four places; the campaign is
  `first_shift`.
- Wiki, `docs/scenario-system.md`, `docs/sections.md`, `web/src/create/*` and
  `web/src/widgets.ts` citations were corrected against the code they cite -
  `widgets.ts` now names constants instead of line numbers, which is what went
  stale.

### 6. The regression pins that could not fail

All four now read the thing they claim to read:

- `comms_arrival_keeps_every_card_at_layout_size` adds the one system that
  writes `UiTransform::scale` and asserts the emphasis is present, so the
  scale it reads is a scale something wrote.
- `no_sweep_mark_sits_inside_something_solid` measures segments.
- `nova_protocol/tests.rs` walks nested actions and areas, and asserts it found
  a volume at all.
- `web/tests/comics.test.js` tests a rejection.

Two of the four were proven non-vacuous by watching them go red:
`no_mainline_handler_posts_an_objective_alongside_a_conversation` caught Second
Shift's `step(0.0, ...)` the moment `action_groups` was corrected, and the
first draft of the arrival-gate pin failed on `OnOrbitLap` - a legitimate gate
closing an objective posted much earlier - which is why the shipped version
walks beat order instead of matching frames.

### 7. Explicit authoring

Two silent-default sites were converted to the repo rule (error at lint, then
at load):

- **Asteroid `material`** is a required `String` and must be a known kind.
  `ScatterObjects` refuses an empty or unknown weighted mix; every kind has an
  impact row, so no rock sounds like hull.
- **`PlanetConfig::invulnerable`** must be `true`. The lint rejects `false`,
  `planet_scenario_object` refuses to build one, and `PlanetConfig::new`
  produces only the loadable value.

`SetCameraAnchor` reaches the lint at all now - both its anchor and its
`look_at` object are checked, so a cinematic can no longer silently frame the
wrong thing.

### 8. The plain asteroid control

`KIND_PLAIN` is now honestly the before picture: it takes no frame jitter
(`AsteroidKindLook::jitter`), so the texture it shows is the texture, unrotated.
The shader skips the macro field, the second projection and the warp behind one
uniform branch (`material.detail`) for any kind that spends none of them.
Verified live: the kinds grid was captured and read.

### Left, with reasons

Deliberate, and each is a change the owner should choose:

- **`editor/preview.rs:317`** - scrubbing a planet's radius rebuilds the
  surface every frame. The clean fix is a mesh cache keyed on the shaping
  fields; dropping `radius` from `drawn_fields` would break rescaling.
- **`planet.rs:77`** - ~50 ms per planet at spawn. The build belongs behind the
  render observer, which is a bigger move than a review fix.
- **`area.rs:181`** - `AreaOccupancy` is keyed on the body avian stamps once. A
  re-key needs a `ColliderOf` observer.
- **`trackers.rs:130,141`** - one tick out of `Hold` zeroes lap progress. That
  is an authoring decision (forgiving vs strict laps), not a defect to patch.
- **`scripts/generate-campaign-portraits.py`** has no `--check` and no docs.
- **`nova_info/build.rs`** does not watch the packed ref, so a debug build can
  stamp a stale commit after a `git gc`.
- **`ci.yaml` has no `web` job**, so the site's tests never run in CI.
- The Gauntlet's tripled hull mass, the `9.81` literal duplicated in two
  crates, and `docs/development.md:263` are unchanged.
- Comms performance and lifetime: `sync_comms_cards` still rebuilds every card
  every frame, and `CommsQueue::pending` is still unbounded. Both are real and
  neither is reachable from shipped pacing, so they wait for a HUD pass rather
  than a review fix. `objective_feedback.rs:10-14` and
  `objective_stack.rs:392-394` are in the same pass.
- The comic's `released.ts` corner letters and `first-shift.svg`'s bleed are
  unchanged, as is `second_shift.rs:383` - the detection beat can outlive the
  win, which is a pacing decision.
- `CHANGELOG.md:277`'s shard budget is unverified, not wrong: checking it needs
  a measured run, not a read.

Owner's call, flagged not decided:

- **`Cargo.toml:111`** - `first_shift_ships` sits in `playable/` while its only
  affordance is the free-fly camera, which the category contract at
  `Cargo.toml:43-45` disqualifies. Its default camera also frames the carrier
  almost full-screen (pre-existing).
- **`probe-runs/`** holds ~52 GB of gitignored measurement history.

### Gates

Run at the end of the pass, on the whole tree:

| Gate | Result |
| --- | --- |
| `cargo clippy --workspace --all-targets --features debug -- -D warnings` | clean |
| `cargo fmt --all -- --check` | clean |
| `cargo test -p nova_scenario --lib` | 331 passed |
| `cargo test -p nova_ship --lib flight::` | 118 passed |
| `cargo test -p nova_authoring --lib` | 113 passed |
| `cargo test -p nova_assets --test mod_binary_resources` | 7 passed |
| `cargo test -p nova_assets --test example_scenario` | 15 passed |
| `cargo test -p nova_probe_cli --test catalog_drift` | 2 passed |
| `content -- lint` | 0 error(s), 0 warning(s), 0 finding(s), 10 scenario(s) audited |
| `content -- gen` | no diff: the committed RON matches the builders |
| `npm test` / `format:check` / `lint` / `build` (web) | all clean |
| `probe run first_shift_map` | aggregate OK, 6/6 PASS, run_end at frame 496 |
| `probe run first_shift_ships` | aggregate OK, 6/6 PASS, run_end at frame 314 |
| `probe run second_shift_map` | aggregate OK, 6/6 PASS, run_end at frame 496 |

The full workspace test suite was not run - it OOMs this box. Every crate the
pass touched was tested by `--lib` or by the specific integration target,
and the red groups the review named were re-run by name.
