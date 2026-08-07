# Findings master - deduplicated and ranked

Every defect from the six review reports plus the clippy audit
(`09` through `15`), merged into one list. Findings reported by more than one
agent get **one** row.

**Every `file:line` below was re-opened against the tree on 2026-08-07 at
`4a8b55aa` and confirmed to still say what the source note claims.** Where it
did not, the row says so. Corrections and withdrawals are at the bottom - read
them, they are as informative as the findings.

## How the ranking works

Ranked by **expected harm**, not by severity label:

```
expected harm ~= (what is lost when it fires) x (how likely it is to fire) x (how far it reaches)
```

So a certain-confidence data-loss bug outranks a certain-confidence cosmetic
one, and a speculative crash does not outrank a confirmed silent-corruption
path. The probe findings lead the list because their blast radius is *every
other lane in this epic* - they decide whether a green CI run means anything.

Columns:

- **Conf** - certain (read and reproduced in source), likely (traced, one
  inferential step), speculative (plausible, unconfirmed)
- **Radius** - files a fix touches
- **Indep** - independent of the structural refactor? `Y` = fixable today with
  no bearing on any move. `N` = cheaper as part of a move, or blocked by one
- **Lane** - see `17-lanes.md`

## Tier 1 - the CI gate is blind (fix first, everything else is verified by it)

| id | Site | Defect | Sev | Conf | Radius | Indep | Lane |
| --- | --- | --- | --- | --- | --- | --- | --- |
| F01 | `nova_probe/src/run_report/artifacts.rs:44` | `RunArtifacts::load` is all-or-nothing: any parse error is `?`-propagated, so `finish_report` returns `Err` and `report.html`/`checks.json` are never written - **after `clean_out_dir` deleted the previous ones**. One truncated `trace.json` discards the clean pass's timeline, invariants and log evidence. Directly contradicts the comment at `run.rs:211` ("Failures degrade to no trace - a successful clean pass is never discarded") | bug | certain | 1 | Y | L1 |
| F02 | `nova_probe/src/bin/probe/native/sweep.rs:187` | `build_row` takes `verdict` verbatim from `checks.json` and stores `run_error` as an independent field that never influences it. If `run()` fails before `clean_out_dir` (`run.rs:66` `create_dir_all`, `:69` `canonicalize`), the **previous** run's `checks.json` is still on disk: the row reports `OK` with an error attached and `aggregate_exit` (`sweep.rs:266`) returns SUCCESS. **CI passes on a commit that was never probed** | bug | certain | 1 | Y | L1 |
| F03 | `nova_probe/src/run_report/artifacts.rs:65` | The loader deliberately excludes `web-run.log` as "chromium's own output, not the game's". **It is both** - the repo's own `stats.rs:708` parses the game's `nova perf:` line out of a chromium `INFO:CONSOLE` line. No `run.log` exists on a web run, so `log_clean` returns SKIPPED and a panicking wasm app verdicts **OK, exit 0** | bug | certain | 1 | Y | L1 |
| F04 | `nova_probe/src/recorder.rs:126` + `nova_autopilot/src/completion.rs:152` | `completion_watch` writes `AppExit` in `Last`; `record_run_end` and `record_invariant_summary` read `MessageReader<AppExit>` in `Last`. Nothing orders them, and bevy exits after the frame in which `AppExit` is written - there is no next frame. On an unfavourable ordering a **completely healthy run** reports "timeline truncated (no run_end)" and the whole `--all` sweep exits non-zero | bug | certain | 2 | Y | L1 |
| F05 | `nova_probe/src/bin/probe/native/run.rs:29` | `RUN_ARTIFACTS` is 12 literal filenames with no `run-<n>.log` glob, while `RunArtifacts::load` (`artifacts.rs:74-92`) globs and concatenates them. Stale sweep-cell logs from a previous run present as this run's evidence. Fails **closed** (false FAIL), which is better than F01-F03, but it trains people to distrust the gate. The comment at `run.rs:26` claims the opposite | bug | certain | 1 | Y | L1 |

F01 and F03 share a root cause: the loader in front of a good pipeline has no
per-artifact error isolation. A failed parse should degrade **that one
artifact** to `None` and let its own check report the failure. One change
fixes F01 and, in spirit, F03.

## Tier 2 - permanent data loss and process death from untrusted input

| id | Site | Defect | Sev | Conf | Radius | Indep | Lane |
| --- | --- | --- | --- | --- | --- | --- | --- |
| F06 | `nova_assets/src/mod_cache.rs:593` (also `:104`, `:117`) | `read_index_at(root).unwrap_or_default()` folds a corrupt or failed read into an empty `Vec`, and the next write persists that empty base plus one record. A truncated `installed.mods.ron` plus one install **permanently erases every other installed mod** from `DownloadedMods`. Their bytes stay on disk as orphans `remove_mod` can never sweep, because no record names them | bug | certain | 1 | Y | L3 |
| F07 | `nova_assets/src/mod_cache.rs:521`, `persist.rs:91`, `portal/catalog.rs:197`, `bin/content.rs:103` | Every persisted store is written with a bare `std::fs::write` - truncate-in-place, no temp file, no rename, no fsync. A kill mid-serialize leaves a zero-length or half-RON file, **which is exactly the input F06 turns into permanent loss**. Same exposure for `enabled_mods.ron` (whole mod selection) and `settings.ron`. `nova_probe/src/recorder.rs:213` and `contract.rs:164` already carry the correct pattern | bug | certain | 4+1 helper | Y | L3 |
| F08 | `nova_mod_format/src/deps.rs:25` | `transitive_deps`'s inner `visit` recurses once per graph edge with no depth bound, over a graph built from untrusted `catalog.json` (`install.rs:425`). `PortalCatalog` has no entry-count cap - `MAX_FILE_COUNT` bounds files *per entry*. A long chain overflows the stack on Install: **a stack overflow aborts the process and cannot be caught**, and it runs before `validate_entry`'s caps | bug | likely | 1-2 | Y | L3 |
| F09 | `nova_scenario/src/variables.rs:66`, `filters.rs:164` | Both DSLs are `Box`-recursive with no depth limit in the RON decode or in `evaluate`. Deeply nested `*.content.ron` overflows the stack inside `ron::de::from_bytes` **on the asset-loader task during boot**. The mod never has to be enabled - the catalog loads every installed bundle's content as a dependency | bug | likely | 2 | Y | L3 |
| F10 | `nova_gameplay/src/sections/turret_section/setup.rs:64` | `let interval = 1.0 / muzzle.fire_rate;` into `Timer::from_seconds`. `fire_rate` is a plain required `f32` on the serde-deserialized turret config; `0.0` gives `+inf` and `Duration::from_secs_f32(inf)` **panics the moment the ship spawns**. Negative panics the same way. `lint/ship.rs:128` lints the hinge axis and muzzle presence but never `fire_rate`. **The sibling live-retune path at `:192` already guards it with `.max(f32::EPSILON)`** - the asymmetry is verified | bug | certain | 1-2 | Y | L3 |
| F11 | `nova_editor/src/placement.rs:42,100` (+ `panic!` at `:46,:104,:205`) | `sections.get_section("reinforced_hull_section").unwrap()` and the `basic_controller_section` twin, plus three `panic!`s on kind mismatch or missing id. A mod overlay redefining or dropping either id **panics the process** on "New Hull Ship". Every other catalog lookup in the codebase logs and skips. **Five panic sites, not the four the review reported** | bug | certain | 1 | Y | L6 |
| F12 | `nova_scenario/src/actions/spawn.rs:317` (field `:244`) | `ScatterObjectsConfig::count` is an unvalidated `u32` from mod RON driving an uncapped spawn loop, and `lint/scenario.rs` never inspects it. `count: 50000000` clones the template 50M times and OOMs - **from data that passed both the static lint and the runtime content gate**. With `min_separation` the rejection sampler is additionally O(count^2) | bug | certain | 2 | Y | L3 |
| F13 | `nova_assets/src/portal/catalog.rs:71` + `transport.rs:31` | The catalog body is read fully into memory with no size bound and parsed **twice** (`SchemaProbe`, then `PortalCatalog`). The 256 KiB cap in `last_good_store` gates persistence only, never the fetch. A large response OOMs the client before the schema gate can reject anything. `install.rs:181` acknowledges the per-file variant for downloads; the catalog request has no declared size to check against at all | bug | certain | 2 | Y | L3 |
| F14 | `nova_events/src/engine.rs:170` | `GameEventInfo::from_data` maps a `serde_json::to_value` failure to `None` **with no log at any level**. `EntityFilterConfig::filter` (`nova_scenario/src/filters.rs:71`) reads `data: None` as "does not match", so every entity-filtered handler for that kind **stops firing permanently and the scenario silently never advances**. Today's vocabulary is all-`String`, so this is one added float field away from live | bug | certain | 1 | Y | L3 |

## Tier 3 - player-visible correctness

| id | Site | Defect | Sev | Conf | Radius | Indep | Lane |
| --- | --- | --- | --- | --- | --- | --- | --- |
| F15 | `nova_gameplay/src/hud/nova_os/input.rs:267` | The `Key::Character(_) \| Key::Space` branch has **no Control guard**, so Ctrl+C, Ctrl+U, Ctrl+W, Ctrl+A and Ctrl+K all insert a literal character at the `nova>` prompt. Every shell line-editing chord a player reflexively tries silently corrupts the line. `handle_nova_os_app_keyboard` (`:355,:374`) deliberately skips Control-held events; the prompt handler chained immediately before it does not. **Most likely finding in the set to be hit by a real player** | bug | certain | 1 | Y | L4 |
| F16 | `nova_gameplay/src/mesh/explode.rs:130` and `:144` | A per-mesh failure inside the collect loop `return`s instead of `continue`s, so one child with a still-loading `Mesh3d` or a non-indexed mesh produces **no `ExplodeFragments` at all** and discards the fragments already built. `integrity/explode.rs:129` skips anything `With<Mesh3d>`, so the fragment handler is the wreck's only despawn path: **a zero-health wreck lingers with its collider live** | bug | certain (control flow) / likely (reaching the `None` arm) | 1 | Y | L4 |
| F17 | `nova_gameplay/src/hud/nova_os/input.rs:430` + `nova_menu/src/widgets.rs:66` | Both `max_*_scroll_y` build their bound from `ComputedNode` **physical** pixels while `ScrollPosition` is **logical** (`bevy_ui-0.19.0/src/layout/mod.rs:346-360`). On a 2x display the maximum is twice the real one. `input.rs:257` `page = size.y * 0.8` is physical too, so one PageUp jumps 1.6 viewports. The codebase knows the rule - `shell.rs:440` and `screen_indicator.rs:418` | bug | certain | 2 (1 after extraction) | N - fixed once by the `nova_ui::screen` extraction | L7 |
| F18 | `nova_gameplay/src/hud/nova_os/shell.rs:379` | `scroll.0.y = f32::MAX` as a pin-to-bottom sentinel is **never cleared**. Bevy writes the clamped value to `ComputedNode` via `bypass_change_detection` and never back to `ScrollPosition` (`layout/mod.rs:365-369`). `(f32::MAX + -page)` is still `f32::MAX` in f32, which clamps to the bottom. Player-visible: **PageUp after running a command needs two presses** | bug | certain | 1 | Y | L4 |
| F19 | `nova_gameplay/src/hud/nova_os/shell.rs:363` | `last_len: Local<usize>` survives shell teardown. `remove_nova_os` (`spawn.rs:710`) calls `terminal.reset_session()` back to 6 welcome rows while `last_len` stays at the old session's 200, so `len > *last_len` is false and auto-scroll stays dead for the next ~190 rows after a respawn. **`reconcile_nova_os_header` (`:288`) and `rebuild_nova_os_footer_hints` (`:320`) both carry the `Added<Marker>` override for exactly this hazard.** Recurrence of the `mode-keyed-reconciler-just-spawned-override` memory at an uncovered site | bug | certain | 1 | Y | L4 |
| F20 | `nova_gameplay/src/audio/cues.rs:99` | `play_safety_engaged_cue`'s `Local<bool>` is process-global and survives the death of the ship it tracked, contradicting its own doc at `:93`. Die while `WeaponsHot(true)`, and the new ship's `WeaponsHot::default()` = false matches `Changed<WeaponsHot>` on the first frame, so a safety-engage click plays with nothing disarmed. **Same pattern as F19** | bug | certain | 1 | Y | L4 |
| F21 | `nova_gameplay/src/audio/loops.rs:188,313` | Loop sinks are volume-driven only while the scenario is live (`SpaceshipSectionSystems` gated on `scenario_is_live`) but the sink entities are session-persistent and never silenced on unload. Menu ambience -> New Game: **the engine hum roars unchanged through the whole scenario load** | bug | certain | 1 | Y | L4 |
| F22 | `nova_menu/src/settings.rs:247` | The settings save is debounced 15 idle frames with **no flush on shutdown**, and `save_settings` has exactly one caller. Drag the volume slider and click Exit (`menu_ui.rs:564` writes `AppExit` immediately) within ~250 ms and the setting is silently lost. A **third** independent persistence defect alongside F06 and F07 | bug | certain | 1 | Y | L3 |
| F23 | `nova_gameplay/src/sections/torpedo_section/projectile.rs:37` | `update_target_position` homes on the target root's raw `Transform::translation` - the ship's **build-spot origin** - rather than `live_structure_anchor`. `sections/mod.rs:38-43` states the rule and every other consumer follows it (`intent.rs:125`, `ai/acquisition.rs:170`, `radar.rs:79`, `camera/framing.rs:52`). Shoot away a large enemy's forward half and the torpedo needs to reach within 15 u of empty space: **a clean miss on a stationary wreck** | bug | likely | 1 | Y | L4 |
| F24 | `nova_gameplay/src/input/ai/mod.rs:107` | The whole AI chain is registered in `Update` while `guns.rs:119`, `behavior.rs:292-308` and `torpedo.rs:158` tick firing-gate `Timer`s off `time.delta_secs()` - and the firing itself happens in `FixedUpdate`. **AI DPS varies with framerate.** The only sweep finding with a player-visible gameplay effect. (Context: the 6-vs-119 `FixedUpdate`/`Update` ratio is NOT a problem - everything touching avian is already fixed-stepped) | bug | certain | 1-3 | Y | L11 |
| F25 | `nova_ui/src/widget/button.rs:496` | `button_on_setting` fires on `On<Add, Pressed>` (mouse-DOWN) while every other button commits on `Activate` (release-over). Press and hold a UI-skin option, drag off, release: the skin has already changed, with no cancel | bug | certain | 1 | Y | L11 |
| F26 | `nova_menu/src/settings.rs:95` (same at `pause.rs:203,286`) | The Settings panel spawns raw `Text` spans with no `nova_ui::widget::UiText` marker, so `apply_ui_font` never routes them through `UiFont`. The "Volume" label, the `NN%` readout, the Controls headers and both keybind columns render in **Bevy's default face** beside siblings in Iosevka Term. `settings.rs` and `pause.rs` are the only menu files that never import `UiText`. Visible in any screenshot | bug | certain | 2 | Y | L11 |
| F27 | `nova_menu/src/settings.rs:228` | The load path clamps `master_volume` but writes `nova_os_bright_detent` / `nova_os_scan_detent` straight through. `components.rs:156` clamps on read so the screen looks right, but `advance` (`:178`) computes `(99+1) % 4 == 0`, so the next BRIGHT click jumps from brightest to dimmest instead of wrapping from what is displayed | bug | certain | 1 | Y | L11 |
| F28 | `nova_menu/src/widgets.rs:75` | `scroll_menu_lists` clamps `ScrollPosition` only in the wheel handler, so nothing re-clamps when content shrinks. Scroll the Scenarios list to the bottom and collapse a campaign header: **the pane renders blank** until the player nudges the wheel | bug | certain | 1 | N - folded into F17's fix | L7 |
| F29 | `nova_editor/src/placement.rs:315` | Placement captures **whatever key happens to be held** as the new section's binding, and the editor camera is driven by those same keys. Hold Space or W while placing a turret and the turret fires on every burn in flight. `ButtonInput::get_pressed()` iterates a HashSet, so W+D makes the bind nondeterministic | bug | certain | 1 | Y | L6 |
| F30 | `nova_editor/src/keybind.rs:60` | Keybind chips are root UI nodes with **no `Pickable` override**, so they block the picking ray to the sections they label. `card.rs:24` and `tooltip.rs:22` define an `IGNORE` Pickable for exactly this. Reads to a player as "clicking randomly does nothing" | bug | certain | 1 | Y | L6 |
| F31 | `nova_editor/src/lib.rs:110` | Re-entering the Editor never resets or rebuilds `PlayerSpaceshipConfig`. Sandbox -> build -> Play -> F1 back to Editor: no preview exists, every click is dropped, yet Play spawns the old ship from the surviving config. **Citation re-anchored** - see corrections | bug | likely | 1-2 | Y | L6 |
| F32 | `nova_editor/src/keybind.rs:187` | Click-to-rebind accepts any key with **no conflict check**. Authored content with that mapping is rejected by `scenario_input_overlaps`, but an editor-built ship is constructed at runtime and never linted | bug | certain | 1 | Y | L6 |
| F33 | `nova_os/src/terminal/view.rs:222` | `prompt_completion_ghost` strips the prefix off the **untrimmed** prompt while `refresh_parse` (`edit.rs:338`) uses the trimmed one, so a leading space turns the prompt green with no ghost rendered. **Found independently by two reviewers** (`10` and `12`) - one row | bug | certain | 1 | Y | L4 |
| F34 | `nova_gameplay/src/hud/nova_os_ship/scene.rs:397` | `ship_input` reads raw `ButtonInput<KeyCode>`, bypassing the app router's Control guard, so `Ctrl+[` both exits the app and cycles selection backwards. Same class as F15 | bug | certain | 1 | Y | L4 |
| F35 | `nova_scenario/src/objects/area.rs:53` | `forget_area_occupancy` prunes only when the AREA despawns. A body destroyed *inside* a live area pins its count above zero forever (avian fires no `CollisionEnd` for a despawned collider - the module says so at `:49-51`), so **a scenario gating on `OnExit` never advances**. `AreaOccupancy` is also never cleared by `teardown_scenario_entities` | bug | certain | 1 | Y | L11 |
| F36 | `nova_scenario/src/lint/scenario.rs:291,348` | `(0.0..=MAX).contains(&secs)` admits `0.0` while the message claims `(0, MAX]`. `auto_advance_secs: Some(0.0)` lints clean, then `outcome.rs:217` builds a `Timer` that finishes on its first tick - the victory banner flashes past unread, and the lint that exists to catch this stays silent | bug | certain | 1 | Y | L11 |

## Tier 4 - performance, all confirmed, all in hot paths

| id | Site | Defect | Sev | Conf | Radius | Indep | Lane |
| --- | --- | --- | --- | --- | --- | --- | --- |
| F37 | `nova_gameplay/src/sections/turret_section/render.rs:129-130` | **Every fired bullet allocates a new `Mesh` and a new `StandardMaterial`.** This is the production path, not a fallback - every shipped turret sets `projectile_render_mesh: None` (`nova_assets/src/sections.rs:286,360,390`, `scenario/craft.rs:257,275,345`). The default turret fires 100 rounds/s per muzzle, so one held trigger creates 100 mesh assets and 100 material assets per second, each forcing a GPU buffer upload and a fresh bind group / pipeline specialization. **Almost certainly the largest single performance defect in the review, and it sits directly under the probe's FPS baseline check.** Fix: a cached handle pair in a resource | bug | certain | 1 | Y | L11 |
| F38 | `nova_gameplay/src/flight/autopilot.rs:877` + `flight/manual.rs:142` | The engine-spool loop is **byte-identical for 16 lines**, and both copies carry the same complexity bug: for every ship, walk every unbound thruster in the world and run `allocation.iter().position(..)` inside that loop - O(ships x thrusters x thrusters_on_this_ship), **every FixedUpdate tick**. One extraction plus a `HashMap<Entity, usize>` kills the duplicate and both copies of the bug together. **Best cost/benefit ratio in the review** | bug | certain | 2 -> 1 | Y | L11 |
| F39 | `nova_gameplay/src/hud/nova_os/crt.rs:219` | `reconcile_nova_os_target` writes `node.width`/`node.height` unconditionally. A `DerefMut` on `Mut<Node>` marks it changed regardless of value equality, so `ui_layout_system` re-upserts into taffy and recomputes the subtree. The only gate is `resource_exists::<NovaOsRtt>`, which lives from ship spawn to despawn - **so it runs every frame while the player is flying and the monitor is hidden**, over a subtree of hundreds of `Text` children. `keybind_dock.rs:537` carries the guard and the explanatory comment | bug | certain | 1 | Y | L4 |
| F40 | `nova_gameplay/src/hud/nova_os/shell.rs:344` | `rebuild_terminal_ui` despawns and respawns **every** scrollback row whenever `NovaOsTerminal` changes, and every edit goes through `ResMut`, so `DerefMut` marks it changed on each keystroke - including caret movement, which changes nothing on screen. Nothing trims `scrollback`. With 400 rows, typing 12 characters despawns and respawns 4,800 `Text` entities. Compounds with F39 | bug | certain | 1 | Y | L4 |
| F41 | `nova_ui/src/status_bar.rs:196` | `update_status_bar_item_ui` writes `Text` and `TextColor` **unconditionally every frame**. The version item's value is a `&'static str`, yet `**text = v.to_string()` allocates and marks `Text` changed, so `measure_text_system` + `text_system` re-measure and re-lay-out both status items every frame, forever. Untested vendored code (0 tests) | bug | certain | 1 | Y | L4 |
| F42 | `nova_gameplay/src/hud/nova_os/shell.rs:442`, `nova_os_ship/scene.rs:750,772` | Same unguarded-`DerefMut` class as F39: `node.left` written every frame while open; unconditional `TextColor` / `BorderColor` / `BackgroundColor` writes in a function that already guards its `Text` write two lines above - inconsistent rather than deliberate | bug | certain | 2 | Y | L4 |
| F43 | `nova_gameplay/src/hud/readout.rs:207` | `format_readout` allocates two Strings per readout per frame (`to_uppercase()` + `format!`) **before** the `if existing.0 != text` compare that usually throws them away. The system's doc comment is explicitly proud of avoiding per-frame entity churn; the allocation just landed on the wrong side of the compare | smell | certain | 1 | Y | L11 |
| F44 | 14 sites, incl. `flight_status.rs:204`, `torpedo_target.rs:180`, `turret_lead.rs:222`, `damage_tint.rs:473,638`, `nova_os_map/scene.rs:104`, `nova_os_ship/scene.rs:213` | `redundant_clone` in per-frame HUD systems. Free allocations every frame, mechanical fix, found by clippy pedantic | smell | certain | 14 | Y | L11 |

## Tier 5 - dead and lying surface (the owner's third deletion target, now concrete)

| id | Site | Defect | Sev | Conf | Radius | Indep | Lane |
| --- | --- | --- | --- | --- | --- | --- | --- |
| F45 | `nova_ui/src/tween.rs` (421 lines, 11 tests) | **The entire `Tween` subsystem has zero consumers workspace-wide.** VERIFIED 2026-08-07: no `Tween<T>` is spawned outside the module's own tests. `TweenPlugin` is registered (`nova_gameplay/src/hud/mod.rs:301`, verified) and runs four empty queries every frame. Latent defect if it ever gains a consumer: `tween.rs:243` inserts `TweenFinished` and nothing removes it, so it latches | dead | certain | 2 | Y | L5 |
| F46 | `nova_ui/src/status_bar.rs:133,153` | `StatusBarStore` is declared and `init_resource`d and **never read or written**. Those two lines are the only hits workspace-wide. The per-entity staging it documents is actually done by `StatusBarItemValue` | dead | certain | 1 | Y | L5 |
| F47 | `nova_gameplay/src/plugin.rs:40` | `NovaGameplayPlugin::render` is documented as gating "meshes, HUD, particles" and is forwarded to **one** plugin (`:109`). Hanabi (`:77`), skybox (`:85`), post (`:86`) and the entire HUD (`:111`) are unconditional. **The advertised headless mode does not exist.** Making it real unblocks HUD-free tests | lying | certain | 1 | Y | L5 |
| F48 | `nova_gameplay/src/objectives.rs:123` | **`rebuild_lines` can never run.** `ObjectivesPanelMarker` appears only inside `objectives.rs` (bundle, `Single` query, its own unit test) - VERIFIED, 4 hits, all in that file. The live objectives HUD is a separate panel (`nova_scenario/src/loader/lifecycle.rs:49-63`). `ObjectivesPlugin`'s only system is a permanent no-op | lying | certain | 1 | Y | L5 |
| F49 | `nova_gameplay/src/sections/torpedo_section/bay.rs:112` | `Without<SectionInactiveMarker>` can never exclude anything: `integrity/glue.rs:49` is the only writer and is guarded by `With<SectionMarker>`, which the spawner does not have. Disable a torpedo bay in place and its cooldown keeps ticking to ready. **The filter reads as a live-safety gate and does nothing** | lying | certain | 1 | Y | L5 |
| F50 | `nova_ui/src/widget/panel.rs:112` | `panel_head` takes a `UiSkin` and discards it (`_skin`). Switching to Hardware repaints the panel to grey but leaves the header a green CRT band. **The `skin` parameter makes every call site believe otherwise** | lying | certain | 1 | Y | L5 |
| F51 | `nova_ui/src/status_bar.rs:238` | The entity the caller spawns with `status_bar_item` is **never parented and never rendered** - the observer copies its data into a brand-new child of the root, leaving the caller's entity a permanent orphan with no `Node`. `nova_core/src/lib.rs:290,297` spawns two. Any future "remove this metric" code operating on the returned handle is a silent no-op | lying | certain | 1 | Y | L5 |
| F52 | `nova_debug/Cargo.toml:18` + root `Cargo.toml:224` | `nova_debug` hard-forces `nova_gameplay/debug` and the root dev-depends on it unconditionally, so **every `cargo test` and example build compiles gameplay with `debug` on** regardless of flags. `nova_info` additionally declares a `debug = []` feature with **zero** cfg sites | lying | certain | 2 | Y | L5 |
| F53 | `nova_gameplay/src/hud/nova_os_ship/mod.rs:166`, `nova_os_map/mod.rs:139` | `NovaOsShipSystems` / `NovaOsMapSystems` are declared as `SystemSet`s and **never passed to `configure_sets`** - VERIFIED: zero references outside their own defining file, not even a prelude re-export. They have no ordering edge to `NovaHudSystems`, which owns both the producer and the consumer of what they write. Whether a `ship repair` result row appears this frame or next is decided by bevy's arbitrary topological order; the `peek_pending_invocation` dance at `nova_os_ship/app.rs:195` exists because of this | lying | certain | 2 | N - the seam split has to answer this anyway | L9 |
| F54 | `nova_debug/src/lib.rs:124`, `inspector.rs:180`, `wireframe.rs:66` | Three separate private `toggle_debug_mode` fns, all registered, all toggling the same `DebugEnabled` on the same F11 press. Works only because three flips of a bool is still a flip - `lib.rs:110` comments "they stay in phase". **A fourth sub-plugin silently breaks the key** | smell | certain | 3 | Y | L5 |
| F55 | `nova_ui/src/widget/register`, `WidgetObserversRegistered` | A first-caller-wins resource standing in for a plugin, alongside two real plugins (`status_bar.rs:147`, `tween.rs:198`). With F45 deleting `TweenPlugin` outright, folding the rest into one `NovaUiPlugin` becomes a two-plugin merge instead of three | smell | certain | 3-4 | N - pairs with F45 | L5 |

## Tier 6 - gate coverage, determinism and correctness-adjacent

| id | Site | Defect | Sev | Conf | Radius | Indep | Lane |
| --- | --- | --- | --- | --- | --- | --- | --- |
| F56 | `nova_assets/src/merge.rs:214` | Undeclared-ref violations are pushed to `undeclared_ref_issues` **only for `Content::Scenario`**. A `Section` or `Campaign` with a bad `self://`/`dep://` ref is logged and then merged anyway, so the runtime gate never sees it: the section lands in `GameSections`, is offered in the editor palette, and fails at the asset server on spawn. The doc at `:145-148` claims the opposite | bug | certain | 1 | Y | L3 |
| F57 | `nova_scenario/src/objects/binding_input.rs:83` | `binding_map_serde::serialize` iterates a `HashMap<SectionId, Vec<Binding>>` straight into serde output - **this is what writes `input_mapping:` into the generated `assets/base/**/*.content.ron`**. Stable today only because bevy's collections use `FixedState`. **A bevy bump reshuffles every generated scenario file at once and `content_ron_parity` fails on a diff nobody authored.** `BTreeMap` or a sorted-key `serialize_map` makes it structural. `lint_walk.rs:380` is the same class (lint output in hash order, pushed unsorted at `:532`) | bug | certain | 2 | Y | L3 |
| F58 | `nova_events_macros/src/lib.rs:37` (same shape at `:42`) | `attr.parse_args()` is consumed via `if let Ok(...)`, so a malformed `#[event_name(...)]` silently falls back to the lowercased ident. `#[event_name = "ondestroyed"]` compiles cleanly with `name() == "ondestroyedevent"`. Dispatch self-matches so nothing breaks loudly, but **every literal-name consumer silently stops matching** - `run_report/html.rs:18` stops filtering `onupdate` noise, and the recorder's `ondestroyed` lookup never fires. `compile_error!` is the correct response | bug | certain | 1 | Y | L1 |
| F59 | `nova_assets/src/portal/mod.rs:176` | `install.entry.files[index]` - the guard above is `if install.files.len() != index { continue }`, which does **not** bound `index` against `entry.files.len()`. A duplicated final-file callback passes the guard and panics. Network-driven index; wants `get(index)` | bug | likely | 1 | Y | L3 |
| F60 | `nova_mod_format/src/deps.rs:104` | `cycle = order.len() != ids.len()` with an un-deduplicated `ids`. Two records with the same id (reachable via a hand-edited index - `mod_set.rs:222 start_downloaded_loads` validates each record but never rejects duplicate ids) make Kahn emit 2 of 3 and report **"a dependency cycle among enabled mods"** for a set with zero declared dependencies. `merge.rs:129` explicitly assumes ids are unique | bug | likely | 1 | Y | L3 |
| F61 | `nova_scenario/src/variables.rs:270` | The `Equal` node of the scenario condition DSL is exact float equality. A mod author writing `Equal(hull_fraction, 0.5)` against any computed value sees the condition essentially never fire, with **no error and no warning**. Untyped-language behavior surfacing as a silent no-op. **Needs an owner decision, not a mechanical fix**: epsilon compare, an explicit `ApproxEqual` node, or documented as-is. Also a benchmark question candidate for the `modder` persona | bug | certain | 1 | Y | L3 (after ruling) |
| F62 | `nova_gameplay/src/camera/skybox.rs:118` | `images.get_mut(&config.cubemap).unwrap()` inside an `On<Insert, SkyboxConfig>` observer. The function already `let Ok(..) else { error!; return }`s for the query one line above, then unwraps an asset that is not guaranteed loaded at insert time | bug | likely | 1 | Y | L11 |
| F63 | `nova_probe/src/run_report/html.rs:217` | `intervals.iter().sum::<f64>() / intervals.len() as f64` with no `is_empty` guard, unlike the identical line at `capture.rs:499`. Prints `NaN` into the report HTML | smell | certain | 1 | Y | L1 |
| F64 | `nova_info/build.rs:11-13` | `expect("failed to get git revision")` + `String::from_utf8(..).unwrap()` - **breaks the build** in a tarball export with no git | bug | certain | 1 | Y | L11 |
| F65 | `nova_gameplay/src/sections/torpedo_section/projectile.rs:94` | Two unordered systems in the same schedule both plain-`despawn()` the same torpedo (`torpedo_detonate_system` in `SpaceshipSectionSystems`, `update_temp_entities` in `TempEntitySystems::Sync`, no edge and no flush between). A torpedo whose lifetime expires on the frame it fuzes gets two queued despawns: the second warns, or **hard-panics under the `FallbackErrorHandler(panic)` the autopilot and probe runs install**. The sibling `despawn_shot_down_torpedoes` (`:43`) already uses `try_despawn` | bug | likely | 1 | Y | L11 |
| F66 | `nova_gameplay/src/sections/torpedo_section/projectile.rs:65` | `torpedo_detonate_system` requires `&TorpedoTargetPosition`, which a dumb-fired torpedo never receives (`intent.rs:209` inserts `TorpedoTargetChosen` alone when `CombatLock` is `None`). **A no-lock launch is physically incapable of detonating** - it flies the full 100 s lifetime, deals only a contact ding, and is silently deleted. The bay still spent the round. May be intended; nothing says so | smell | certain | 1 | Y | L11 |
| F67 | `nova_gameplay/src/sections/thruster_section.rs:353` | Main-drive thrust is a raw impulse **never multiplied by `dt`**, so linear authority is proportional to tick rate while torque and RCS authority are not. Internally consistent today because the flight layer reads `dt` back out. **Halving `Time<Fixed>` from 64 to 32 Hz halves every ship's and torpedo's linear acceleration** and silently rescales every autopilot gain tuned against it | smell | certain | 1-3 | Y | L11 |
| F68 | `nova_assets/src/mod_refs.rs:75` | `self://` refs always rewrite via a raw string join, unlike `dep://` which is membership-gated. Containment rests **entirely** on bevy's `UnapprovedPathMode::Forbid` plus `SandboxedAssetReader` (`mod_cache.rs:342`); the ref layer contributes nothing, and per F56 the scan that would flag it is dropped for Section content. Defense-in-depth gap, not a live escape | smell | certain | 1 | Y | L3 |
| F69 | `nova_assets/src/portal/install.rs:459` | Dependency installs are fired-and-forgotten with no join, so a dependent commits even when a transitive dep's download failed. Documented as accepted at `:452-458`, but the failed job is keyed under the **dependency's** id, not the dependent's, so the UI shows no linked surface | smell | certain | 1 | Y | L3 |
| F70 | `nova_probe/src/capture.rs:522` | The in-app CSV append has **no schema-version guard**, unlike the public writer `append_frametime_row` (`stats.rs:415-426`) which refuses a v3 row under a pre-v3 header and comments on exactly this case. `NOVA_PERF_OUT` into an old results dir appends 18-column rows under an 11-column header; `parse_frametime_csv` then errors, which **via F01 destroys the whole report** rather than rejecting one row | smell | certain | 1 | Y | L1 |
| F71 | `nova_probe/src/bin/probe/native/env.rs:76` + `run.rs:180` | The fps pass rewrites `probe-contract.json` despite the module doc declaring the clean pass owns it: `run.rs:180` strips only `NOVA_PERF_TIMELINE` and `NOVA_PERF_INVARIANTS`, so `NOVA_PERF_CONTRACT` (set at `env.rs:98`) survives. Benign today (same binary, same plugins); real the moment the fps pass diverges in features | smell | certain | 1 | Y | L1 |
| F72 | `nova_scenario/src/loader/mod.rs:144` | `ScenarioConfig::default()` is **invalid by its own documentation** (`:141`: its default `cubemap` is a handle-backed `AssetRef` which errors on serialize). An invalid-state `Default` kept only so 14 builder sites can skip three optional fields, guarded by a comment rather than by the type. `ScenarioConfig::new(id, name, cubemap)` makes the invalid state unrepresentable | smell | certain | 15 | Y | L11 |
| F73 | `nova_os/src/terminal/edit.rs:293` | `completion_matches` iterates a `std::collections::HashMap` and appends without dedup, so **Tab-cycle order for argument candidates varies between processes** | smell | certain | 1 | Y | L4 |
| F74 | `nova_os/src/terminal/edit.rs:109` | Terminal history is unbounded and never deduped; only `reset_session` clears it. 200 submits of `log` means 200 Up presses to reach anything else | smell | certain | 1 | Y | L4 |
| F75 | `nova_gameplay/src/audio/cues.rs:147` | `play_dry_fire_cue`'s `Local<HashMap<Entity, bool>>` is never pruned for despawned turrets, unlike the sibling `SfxThrottle` which has an explicit `prune_sfx_throttle` (`mixing.rs:195`). Memory only - entity generations mean no stale-latch misfire. **Same family as F19/F20** | smell | certain | 1 | Y | L4 |

## Tier 7 - the examples are the harness, so their defects are gate defects

| id | Site | Defect | Sev | Conf | Radius | Indep | Lane |
| --- | --- | --- | --- | --- | --- | --- | --- |
| F76 | `examples/screenshots/screenshot_ui.rs:171` | **The "settings panel is up" assertion cannot fail.** The panel is toggled by `Visibility` only, and `ui_node_rect` (`nova_autopilot/src/input.rs:135-151`) queries `(Name, UiGlobalTransform, ComputedNode)` without checking visibility. Rename the Settings button and `click_named` warns and continues, the panel never opens, the hidden node still exists, the assert passes, and **`wiki-settings.png` ships as a shot of the bare main menu with exit 0** | bug | certain | 1-2 | Y | L1 |
| F77 | `examples/systems/player_path.rs:182` | The capture-loop restart hook (`reload_the_run`, `:379`) does not release held keys, while the in-run restart `replay_the_run` (`:537-543`) does and documents why it must. Under `NOVA_PERF=1` the loop restarts with G latched; `ButtonInput::press` on an already-pressed key raises no `just_pressed` edge, so the looped cycle produces no GOTO edge and error-exits on its 20 s deadline | bug | certain | 1 | Y | L1 |
| F78 | `examples/sections/turret_section.rs:404-406` | `tag_gate` fires on **every** `Add<AsteroidMarker>` and inserts `RangeGateMarker` unconditionally, so the gravity planetoid is tagged as a range gate. A round hitting the planetoid flips `outcome.gate_damaged` and the example reports "a turret round connected with a gate" on a run where no target gate was hit; `report_status` prints 6 gates for 5 | bug | certain | 1 | Y | L1 |
| F79 | 11 items across 8 example files (`hull_section.rs:535,547,563`, `torpedo_section.rs:69,349`, `controller_section.rs:64`, `screenshot_combat.rs:128,134`, `screenshot_sections.rs:199`, `player_path.rs:55`, `many_sections.rs:37`) | Dead code under default features, invisible today because CI never builds without `--features debug`. Each exists **only** to serve debug-feature code, so `#[cfg(feature = "debug")]` is the honest fix and it unblocks adding a default-features CI job | smell | certain | 8 | Y | L0 |

## Tier 8 - style and hygiene, routed rather than fixed here

| id | Item | Disposition |
| --- | --- | --- |
| F80 | 37 `#[allow(clippy::type_complexity)]` workspace-wide (35 in `crates/`, 2 in `src/`+`examples/`) against 4 `#[expect(..., reason = "...")]` | Convert to `#[expect]`. Makes suppression rot **self-reporting** via `unfulfilled_lint_expectations` at zero analysis cost, and enforces an existing local convention (`hints.rs:200`, `keybind_dock.rs:569,737,790`). Two already known stale (`ammo_readout.rs:325`, `:510`). Pairs with `-D warnings`. **Lane L0** |
| F81 | 9 `#[allow(clippy::too_many_arguments)]`, 6 genuinely refactorable | `#[derive(SystemParam)]` is already the local idiom (`nova_os_ship/sections.rs:223 ShipSections`). `nova_os_map/scene.rs:259 map_input` and `nova_os_ship/scene.rs:336 ship_input` take an **identical** 6-param cluster - one shared struct removes two suppressions and a duplication. **Lane L9** |
| F82 | 5 `needless_pass_by_ref_mut` in `src/` (`chip_layout_rig.rs:278`, `ai/behavior.rs:909`, `component_lock.rs:403`, `radar.rs:387`, `turret_section/aim.rs:510`) | In Bevy a `&mut` that is never used mutably, **if it reaches a system signature**, declares a write the scheduler must serialize against. Cost is lost parallelism and spurious ambiguity. Verified: `chip_layout_rig.rs:278` is a test helper, not a system param - read the other four before acting. **Lane L11** |
| F83 | clippy bucket 3 - `map_unwrap_or` 190, `suboptimal_flops` 205, `redundant_closure_for_method_calls` 87, `option_if_let_else` 65, `uninlined_format_args` 52, `wildcard_imports` 47, `single_match_else` 47, `semicolon_if_nothing_returned` 33, `items_after_statements` 32, `explicit_iter_loop` 29 | **Do not fix in this epic.** Hand the list to `conventions-prompt.md`; each candidate rule arrives with a free violation count from the 2026-08-07 run |
| F84 | `proc-macro-error2 v2.0.1` future-incompatibility | Transitive dependency, not this code. `-D warnings` does not cover it. Needs its own tracking task; it breaks on a rustc bump |
| F85 | `while_float` x2 (`nova_os_map/tests.rs:842`, `nova_os_ship/tests.rs:1316`); `iter_with_drain` at `mesh/explode.rs:200`; `case_sensitive_file_extension_comparisons` at `run_report/artifacts.rs:81` | Low priority. The float loop conditions can spin forever on a NaN; the extension comparison is irrelevant on Linux CI and real on a case-insensitive filesystem. **Lane L11** |
| F86 | Nits: `transform/directional_sphere_orbit.rs:121` (angle lerp with no wrap handling - latent only because the velocity HUD passes `smoothing: 0.0`); `math.rs:35` (absolute `f32::EPSILON` snap threshold that can never be true at chase-camera scale); `camera/shake.rs:295-296` (offset and kick fed the *same* random sample, so shake reads as 1-D jitter) | **Lane L11**, or drop. None is player-visible today |

## Cross-cutting patterns - each is a CONVENTIONS.md rule candidate

Route all four to `conventions-prompt.md` as well as recording them here. Each
arrives with a measured violation count, which is what makes a rule defensible
against a codebase this clean (see `07-comments-and-docs.md`).

**1. Stale `Local<T>` - 4 instances.** `Local<T>` in Bevy is per-system and
process-lifetime. **Any use that tracks entity state is a latent bug the moment
that entity can respawn.** Sites: F19 (`hud/nova_os/shell.rs:363`), F20
(`audio/cues.rs:99`), F75 (`audio/cues.rs:147`, unpruned rather than stale),
plus the one already in the owner's memory as
`mode-keyed-reconciler-just-spawned-override`. The tree already contains both
correct fixes: an `Added<Marker>` override (`shell.rs:288,320`) and an explicit
prune (`mixing.rs:195`). A rule here is enforceable by review because the
counter-examples are in-repo.

**2. Unguarded per-frame writes through `DerefMut` - 5+ sites.** Writing
`node.width`, `color.0` or `**text` unconditionally marks the component changed
regardless of value equality, forcing a UI relayout or a text re-measure.
Sites: F39, F40, F41, F42 (two), F43. `keybind_dock.rs:537` is the reference
implementation **and carries the explanatory comment** - the rule can cite it
directly.

**3. Code that lies about its guard - 4 sites across 3 crates.** F49
(`torpedo_section/bay.rs:112`, a `Without<>` filter that excludes nothing), F48
(`objectives.rs:123`, a system that can never run), F50
(`nova_ui/src/widget/panel.rs:112`, a `skin` parameter discarded as `_skin`),
F51 (`nova_ui/src/status_bar.rs:238`, an entity that is never rendered). This
is the owner's "dead and lying surface" deletion target with concrete
instances. The rule is not "delete dead code" - it is **an unused parameter or
an inert filter must be removed, never renamed to `_`**, because the signature
is what the caller believes.

**4. Unvalidated authored values reaching arithmetic - 5 sites.** F10
(`fire_rate` into `Duration::from_secs_f32`, panics), F12
(`ScatterObjectsConfig::count` into an uncapped spawn loop), F08 and F09
(unbounded recursion in the dependency walker and both DSL decoders), F13
(unbounded fetch body). **Mod content is untrusted input**: it arrives from a
remote portal catalog and from files the player may have edited, so a panic
reachable from it is a defect, not an upheld invariant. Note the tree already
has the guarded form for F10 one function away, at `setup.rs:192`.

## Corrections and withdrawals

Kept visible so a later reader does not re-derive the rejected version. Each
was measured, not argued.

| # | Original claim | What settled it |
| --- | --- | --- |
| W1 | "Useless comments all over the code" (the task's founding premise) | Measured twice, independently: **83% why-comments, 11% restatement, 0 commented-out code, 3 TODOs**. A strict purge yields ~440 lines of 155,587. Premise **rejected**; the deletion target was redirected to stale narrative, duplicated implementations and dead surface. `07-comments-and-docs.md` |
| W2 | "nova_events is unused inside nova_gameplay, so the coupling doctrine is not real" | **Wrong.** `nova_events/src/lib.rs:1-9` states it is the scenario/modding vocabulary, and usage matches (nova_scenario 50 refs, nova_gameplay 10, and those 10 are exactly the scenario-observable moments). AGENTS.md:102's wording is what misled the audit. **Reword, do not migrate.** `01-decisions.md` |
| W3 | "The never-compiled wasm paths have probably rotted" | **Prediction wrong.** All 14 crates type-check clean on `wasm32-unknown-unknown`, exit 0, 7 warnings. The `Storage`-trait work is still justified by testability and gate removal - just not by latent breakage. `09-clippy-and-lints.md` |
| W4 | "Seven `unreachable!()` match guards are a refactor hazard" | **Overstated.** Four (`lint/ship.rs:443,769,772`, `lint/scenario.rs:712`) are inside `#[cfg(test)] mod tests` opened at `ship.rs:314` / `scenario.rs:529` - test assertion helpers. **Only `nova_gameplay/src/mesh/slice.rs:67` is production.** Confirmed independently by two reviewers. `08`, `05`, `11` all amended |
| W5 | "Three duplicated scroll clamps" (`06-ui-layer.md`) | **Count wrong, argument stronger.** There are **two** `max_*_scroll_y` and they agree with each other; the `nova_editor` third site is unclamped and is a different defect. And both copies carry the physical-vs-logical unit bug (F17), so deduplicating fixes it once instead of twice. Plus a fourth defect in the same neighbourhood (F28) |
| W6 | "`run_report/` is the best code in the crate. Rename, do not rebuild." (`04-nova-probe.md`) | **Amended.** The *shape* is still right and the rename recommendation stands. But the loader in front of it carries four gate defects, three failing **open** (F01-F03, F05). "Do not rebuild" was correct about the structure and wrong about the confidence |
| W7 | An audit agent reported findings under `src/bin/probe/run_report/` | **That path does not exist.** The real path is `crates/nova_probe/src/run_report/`. Recorded in `04-nova-probe.md`; repeated here because it is the reason every citation in this file was re-opened |
| W8 | `nova_editor/src/lib.rs:110` cited as the `DespawnOnExit(ExampleStates::Editor)` on the preview ship (`12-review-ui-layer.md`) | **Citation imprecise; finding stands.** `lib.rs:110` is the `OnEnter(ExampleStates::Editor)` registration - which is where the missing reset belongs. The `DespawnOnExit` markers are at `placement.rs:32,90`. Re-anchored in F31 |
| W9 | "36 `#[allow(clippy::type_complexity)]`" (`13-review-cross-cutting.md`) | **37 workspace-wide** (35 in `crates/`, 2 in `src/`+`examples/`). `08-tests-ci-risk.md`'s figure of 37 was right. The recommendation is unaffected |
| W10 | "The `nova_editor/src/placement.rs` unwrap was independently flagged by the cross-cutting sweep" (`12-review-ui-layer.md`) | **Cross-reference withdrawn.** `13`'s three named bad-unwrap sites are `portal/mod.rs:176`, `camera/skybox.rs:118` and `nova_info/build.rs:11` - `placement.rs` is not among them. One agent found it, not two. **The finding itself is verified and stands** (F11), and the re-check turned up a fifth panic site the review missed (`placement.rs:205`) |
| W11 | "`tween.rs` is untested vendored code" (implied by `08-tests-ci-risk.md`) | **Inverted.** It has **11** tests. The problem is the opposite: well-tested code with **zero consumers** (F45) |
| W12 | "One bad field in the settings store wipes all settings" | **Dead.** Every field has a serde default, `load` returns `None` (not a reset-to-default write) on parse failure, and `save_to` creates the parent dir. Four tests pin exactly this. The real settings defect is a different one (F22) |
| W13 | "`recorder.rs`'s `File::create` truncation comment may not match the code" (`08-tests-ci-risk.md`) | **Hypothesis wrong.** `truncate(false)` -> `try_lock` -> `set_len(0)`, correct order, cross-process lock. This is the atomic-write **reference** the nova_assets fix (F07) should copy |
| W14 | "`nova_events/src/engine.rs` dispatch may drop events / order by HashMap / recurse unboundedly / unwrap on the dispatch path" | **All four wrong.** Actions get only `&mut W` and cannot enqueue; no list is mutated during iteration; no unwrap on the path. The one real `engine.rs` defect is F14, which is a different thing entirely |
| W15 | Byte-vs-char UTF-8 panic in the `nova_os` terminal | **Dead, three independent confirmations.** `insert_text`/`backspace`/`delete`/`move_cursor_*` all go through `char_indices()`; the slicing `debug_assert!`s hold; tab-completion cycling is index-safe even if the candidate set shrinks mid-cycle |
| W16 | `clippy::suspicious_operation_groupings` at `hud/key_glyphs.rs:166` | **False positive.** The lint expects field symmetry across a chain that deliberately spans two different objects |
| W17 | `cast_possible_truncation` / `cast_sign_loss` (37 of clippy's 51 "real signal" hits) as a cleanup target | **Not a target.** Sampled from two directions - clippy-side and grep-side - and every float-to-int cast read was clamped within 2 lines, several with a comment naming the reason. `settings.rs:227-229` is the model site. **Do not spend the epic on them** |

## What came back clean - do not re-audit

- **No reachable `unwrap`/`expect`/runtime-sized indexing panic in non-test
  code**, anywhere in the audited scope. **Four independent confirmations.**
  The named exceptions (F11, F59, F62, F64) are individual sites, not a class.
- **The simulation core is sound.** Flight guidance, the QP throttle balancer,
  the bisection projection, the RCS cap sign logic, the PD controller's
  inertia-frame composition, gravity's hysteresis and the whole integrity
  cascade were audited deeply and came back clean. `balance_throttles` always
  returns `engines.len()` entries, so the `throttles[i]` indexing in
  `autopilot.rs:884` and `manual.rs:149` **cannot panic**.
- **Path traversal / zip-slip in the mod stack is unusually well done** -
  `is_safe_id` / `is_safe_rel_path` applied in the shared `validate_file_op`
  **before** the cfg dispatch and re-applied at each fs boundary.
- **Overlay precedence and event dispatch do not depend on `HashMap` iteration
  order.** (Two *generated-content* paths do - that is F57.)
- `hud/keybind_dock.rs` is the most careful reconciler in the tree:
  `set_if_neq` throughout, guarded `Node` writes, `Added<DockChip>` overrides,
  real `.after()` edges. **Use it as the reference when fixing F19, F39-F42.**
- Pause `OnEnter`/`OnExit` symmetry is correct on both axes; no stranded
  cursor, no double-spawn.
- `aggregate_exit` **does** propagate correctly - `run.rs:347` and
  `report.rs:67` both fail closed, and `verdict_severity` ranks unknown
  verdicts as FAIL. F01-F05 corrupt the *input* to it, not the logic.
