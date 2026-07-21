# Changelog

All notable changes to this project are documented here.

The format is loosely based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)
but groups each release by subsystem (Gameplay & Flight, Combat & Weapons,
Ships & Sections, Scenarios & Objectives, Modding & Mod Portal, Interface & HUD,
Web & Platform, Audio & Visuals, Performance, Fixes, Internals & Tooling) rather
than by Added/Changed/Fixed. This project adheres to
[Semantic Versioning](https://semver.org/spec/v2.0.0.html). Breaking changes are
tagged **(breaking)**.

## [Unreleased]

### Scenarios & Objectives

- New base-campaign scenario **Lifeline** (chapter three, part one): the Tallyman's raiders hit the belt's supply convoy in revenge for the Rust Tally. Screen two stalled haulers (the Ceres Queen among them) through three telegraphed raider waves until the relief wing arrives - a live `RELIEF mm:ss` countdown on the HUD, wave gates that ride the scenario clock AND the previous wave's clears (no stacking on a slow run), an early-clear win for aggressive play, and Victory banners that track the convoy's fate. The convoy is the first shipped ally content: `controller: None` haulers flying `allegiance: Some(Player)`, so raider AI genuinely hunts them. Breaking the Broadside gunship now chains into Lifeline instead of dead-ending; Lifeline is picker-visible as the chapter head. New autopilot example `lifeline` (defeat/retry/three-waves/victory walk) wired for probe and the smoke suite.
- New base-campaign scenario **Final Tally** (chapter three, part two - the finale): Lifeline's trace ends at the gang's claim, a cracked megahauler anchorage berthed deep in a planetoid's gravity well - the base chain's first combat gravity well, ringed by a scattered belt (the Ring region's first combat use). Survey the anchorage by holding a travel lock on its bow, break the two-ship orbital picket (guards on rails - the orbit directive's first combat use), and finish the gang's flagship when it casts off with an escort: the campaign's only simultaneous capital+escort fight. The flagship kill opens a paced epilogue (confirm line, the guild's close, then the banner) and the campaign completes properly - "End of the base campaign - for now." Lifeline's victories chain here; the New Game arc is now five scenarios: Shakedown Run, Broadside (two parts), Lifeline, Final Tally. The `lifeline` autopilot example now walks BOTH parts of chapter three end to end.
- The base campaign found its voice: Broadside's story now plays over the comms panel instead of riding inside objective text - Capt. Halloran's distress call opens the chapter (the same call the Shakedown banner promised), she calls the ambush and the first corvette kill, the Rust Tally taunts on arrival, and Belt Relay reports the Ceres Queen's beacon going dark. Objectives shrink to imperative goals ("Find the hauler Ceres Queen."). The Victory banners now acknowledge whether the Ceres Queen survived the fight - each part tracks its own hauler and picks the matching banner line. First voiced cast of the base chain: Capt. Halloran, Rust Tally, Belt Relay (names pending the owner's nod).
- The **Asteroid Field** sandbox is back in the Scenarios picker (with the placeholder thumbnail its siblings carry). It had been hidden as "a mid-story stage reached by chaining" - a premise that was never true: it was the original New Game scenario until the Shakedown Run replaced it, and nothing chains into it, so hiding it made finished content unreachable. Its relay continuation (`Asteroid Field - Next`) stays hidden as before.
- New `HudReadout` scenario action: show a scenario VARIABLE on the HUD - the display half of the variable vocabulary, usable by any mod. A named `slot` binds a `variable` (read live every frame), with a `format` (`Number` one-decimal, `Integer` rounded, or `Time` as `mm:ss.s`), an optional `label`, and `visible` to show or clear it. It renders as an Instrument-tier top-center readout, freezes on pause and behind the outcome overlay (the bound variable stops), and clears at scenario teardown so it cannot leak into the next scenario or the menu. One fire is enough for a live readout. Documented in the scenario action reference and the authoring guide.

### Modding & Mod Portal

- Gauntlet Run is now a TIME-TRIAL (bundle 1.3.0): a live `mm:ss.s` run clock (a `HudReadout` on the engine `scenario_elapsed` clock) shows from the START gate and freezes at the final time behind the Victory banner, and a clean-run bonus - hazard-zone grazes bump a `crash` counter, and crossing FINISH clean (`crash == 0`) earns a CLEAN RUN banner where a grazed run gets the plain finish. Both branches are gated `Outcome(Victory)` handlers, no new engine machinery beyond the readout. `scenario_elapsed` resets on retry, so the clock does too.

### Fixes

- The mouse cursor is now hidden and locked while flying, in dev builds too. Flight always captured the cursor in a shipped build, but the whole grab was compiled out under the `debug` feature so the F11 inspector stayed clickable - which left a stray cursor floating over every `--features dev` playtest. Flight now hides the cursor unconditionally; the F11 debug inspector defaults off and hands the cursor back only while its panel is up. Menus, pause, and the win/lose overlay free it as before.

### Internals & Tooling

- `content lint` is now the single content-validation command: the balance `audit` subcommand was folded into it (balance is a kind of lint), so one pass runs the reference/geometry checks, the combat balance/fairness audit (with the `balance_acks.ron` acknowledgment mechanism, and a stale ack now an ERROR) and a new flight-rig INPUT-OVERLAP check that flags a content `input_mapping` section bound to a key the always-on flight rig also drives (W/Space/RightTrigger burn, autopilot, ...) - the silent double-drive behind the 10_playable "guns on Space" regression. `lint --target <mod> --report <path>` writes a per-mod document (Markdown, or HTML for a `.html` path / `--format html`) that pinpoints, for every finding, the source file + offending element + explanation + suggested fix. Old `content audit` invocations become `content lint`.
- The lessons ledger moved out of `docs/` to `LESSONS.md` at the repo root (flow v2 conventions). `docs/` now wipes to only its `README.md` at a release (`wipe-docs.sh` / `check-docs-clean.sh` / release guard updated), AGENTS.md gained a Development flow section, and the historical task records were normalized to pass `tatr check` (verdict lines, severity tokens, closed-task step boxes).
- Probe timelines now show the FEATURE working, not just the process surviving: the section ranges emit outcome markers at their assertion sites (turret fired/gate damaged, the torpedo fire->arm->detonate->hit chain, hull partial-hit-exact + destroyed-ship-survives, attitude tracking error, burn speeds, COM/camera drifts) and broadside marks all 11 script stages - each with its values on the record.
- The whole example fleet is probe-evaluable: every cataloged example (except the real-GPU `render_scale_shot`) now carries the run-timeline recorder, the continuous invariant checks and the frame-time capture - all inert without probe's env - so `run_completed`, `reached_playing` and `invariants_held` are MEASURED fleet-wide and `--fps` works on any example. Frame rows record their build profile (CSV schema v3; v1/v2 files still load) and the report badges `dev` rows as not-a-baseline.
- Probe runs whole fleets: `probe run playable,scenario` (comma list), `probe run gameplay` (a category dir), or `probe run --all` (every cataloged example minus a reasoned NOT_PROBED list) run sequentially with continue-on-failure and write an aggregated status index above the per-example run dirs - `index.html` (verdict, measured n/total, per-check status and duration per row, each linking its own report), `index.json` for machine consumers, and a `probe-all.json` manifest gating `probe report` re-renders. The aggregate verdict is the worst row and the exit code mirrors it; a bare `probe run` errors with the catalog listing instead of accidentally starting a fleet sweep.
- The frame capture always gets its window, and the window measures activity: harness collectors now NEGOTIATE the exit (bcs completion protocol, v0.19.3/v0.19.4 - the app ends when the autopilot AND the capture are both done, with a deadline that names laggards), `--fps` runs as a dedicated capture-only pass (the correctness recorder's per-entry flush no longer contaminates frame numbers), and enrolled scenes (scenario, playable) reload + replay while the capture fills - scene-reload intervals are excluded from the stats (their count is host-speed-dependent) and reported as their own mean/max line in the report.
- `probe run gameplay --fps` no longer bare-FAILs on narrative examples: a one-shot scripted scene like `broadside` (die/retry/win, ~181 frames, cannot loop) can never fill a capture window, so it is marked fps-EXEMPT in the root `Cargo.toml` (`[package.metadata.nova_probe] fps_exempt = [...]`). Exempt examples run the clean + profiled correctness passes and the report shows an honest "fps-exempt" note instead of idling to the harness deadline and timing out with no data. Outside `perf/`, `--fps` now defaults to a shorter 60/240 capture window so a bare run fits the deadline under software rendering (`perf/` and the sweep matrix keep the full 180/900 baseline window; an operator's own `NOVA_PERF_WARMUP`/`NOVA_PERF_FRAMES` always win).
- `--baseline` now works across a group: `probe run <category>`/`--all --fps --baseline <dir>` treats `<dir>` as a baseline ROOT (a prior group's out dir) and compares each example against `<dir>/<example>/frametime.csv` when present, skipping (SKIPPED, not error) the examples with no prior capture - so a whole fleet regression-checks in one command against a previous `--all` run. A single-example `--baseline` still means that one run dir; previously any multi-run + `--baseline` was rejected outright.
- `probe run perf_baseline --fps` no longer false-FAILs in a dev build: the harness completion deadline is now SIZED to the requested capture window instead of a flat 120s. probe sets `BCS_HARNESS_DEADLINE` for the fps pass to `(warmup + frames) / ~2fps + margin` (and raises its own supervisor timeout above it), so a legitimately-slow-but-progressing capture - a heavy scene under software rendering - completes rather than tripping the hang detector; a genuine hang still fails at a window-appropriate bound, and an operator's `BCS_HARNESS_DEADLINE` still wins. Separately, every example's `main` now returns `AppExit`, so a completion-deadline expiry (or any harness error-exit) is a non-zero process exit the `process_exit` check reports, instead of exiting 0 and being caught only by the log scan.
- Harness runs are silent: any bcs harness env (`BCS_AUTOPILOT`/`BCS_SHOT`/`BCS_REEL`) zeroes the audio output, so headless smoke tests and probe runs no longer play the game through the speakers. Only the output gain is masked - the volume setting, its persistence and the settings menu are untouched. `NOVA_MUTE=0` forces sound through a harness run, `NOVA_MUTE=1` mutes a normal one.
- The examples moved into purpose directories with bevy-style slug names (`examples/sections|gameplay|ui|screenshots|perf/`; `08_scenario` is now `scenario`, `20_perf_baseline` is `perf_baseline`, and so on - number prefixes are gone). The `[[example]]` catalog in the root Cargo.toml is the single source of truth (auto-discovery off), the smoke suite runs per category (`cargo test --test examples_smoke sections`), and a display-free `catalog_matches_disk` test pins disk == catalog == smoke lists so a new example cannot silently skip either.
- Unified run report: probe assembles whatever a run captured (timeline, frame stats, trace, log - each optional) into one self-contained report.html plus a machine-readable checks.json - auto checks (process exit, run completed, reached Playing, invariants held, FPS vs baseline as a soft gate, log scan) with a provisional OK/WARN/FAIL/NO_DATA verdict, `measured: n/total` coverage, and a reviewer checklist that owns the final call. `probe report <run-dir>` re-renders, gated on the run manifest so stale hand-assembled dirs are refused.
- Profiled pass: `probe run <example> --profile` captures bevy's per-system spans into a Perfetto-openable chrome trace (new root `trace` feature) and the report renders a top-N costliest-systems table (`probe report` re-renders it from the run dir); `--samply` adds a named flamegraph via the dedicated `profiling` cargo profile (full DWARF + frame pointers). Separate passes from the FPS capture on purpose - tracing overhead inflates frame times.
- Frame-time capture and sweeps: percentile stats with run metadata (backend, GPU, resolution, preset, git SHA, host) in every row - pre-metadata result files still load - and the scenario x preset sweep matrix is `probe run perf_baseline --fps --release --scenario ... --preset ...` (`--render sw` for the lavapipe floor; `--platform web` scrapes the web/WebGPU frame line through Trunk + headless Chromium). The perf-baseline.sh/perf-web.sh/perf-profile.sh scripts and the run_report/perf_report/perf_trace bins retired into the one probe front door; probe's whole verb surface is `run` and `report`, and retired commands (`sweep`/`web`/`profile`/`trace`) error with a pointer to their `run` form.
- Run-harness hardening for unattended use: `probe run` starts from a surgically cleaned run dir and writes a `probe-run.json` manifest (identity, passes, exit/timeout outcomes) that the report reads - a hung or crashed run now still produces a FAILing report instead of no report, the child's exit status is a first-class check, zero-evidence dirs verdict NO_DATA (nonzero exit) with `measured: n/total` alongside every verdict, FPS improvements no longer read as regressions, monotonic invariants survive scenario reloads, and checks.json carries structured per-check data for machine consumers.
- One front door for the run-harness: `cargo run -p nova_probe -- run <example>` executes the post-feature check end to end - clean pass (run timeline + invariants + log, throwaway Xvfb), optional `--profile` traced pass and `--samply` flamegraph (separate builds, so profiling overhead never touches the clean numbers), then the run report with its OK/WARN/FAIL verdict.
- Continuous invariant checks in `nova_probe`: arm with `NOVA_PERF_INVARIANTS=1` (or `=strict` to panic at the moment of corruption) and every frame asserts engine guarantees - health bounds, velocity finiteness plus an absurd-speed bound, scenario-variable finiteness, opt-in monotonic variables, and an entity-count leak bound. Violations warn and land on the run timeline; the run report's `invariants held` check reads the tally.
- New run-timeline recorder in `nova_probe`: arm any wired autopilot example with `NOVA_PERF_TIMELINE=<out.jsonl>` and the run records an ordered JSONL timeline - state transitions, every fired scenario event with payload, scenario-variable changes (old/new) and the script's own beat markers - flushed per entry so a panicked run keeps everything up to the panic. The correctness half of the run-harness; playable is the worked example.
- bevy_common_systems 0.19.1 -> 0.19.2: `GameEvent` gained public read accessors (`name()`/`info()`) so external observers (the run recorder) can see events pass by without touching the dispatch queue.
- The `nova_perf` crate is now `nova_probe`, the run-harness crate the v0.8.0 tooling strand grows (frame-time capture, reporting, and - as follow-up tasks land - run-correctness recording, invariant checks and profiling). Bin names, `NOVA_PERF_*` env vars and output formats are unchanged.
- Perf captures now record run metadata alongside the numbers (wgpu backend + adapter, resolution, graphics preset, git SHA, host), so a results file names its own renderer instead of leaning on its directory name; the report shows it, and pre-metadata result files (the v0.7.0 baseline) still load.

## [0.7.0] - 2026-07-18

### Gameplay & Flight

- A reaction-control system (RCS) for fine docking translation: hold SHIFT and steer with the mouse (lateral and fore/aft) and the scroll wheel (up/down) to nudge the ship along its own axes under a gentle speed cap, with no rotation - the helm and camera hold still while you translate, the velocity sphere turns violet, and a soft RCS burn loop tells you it is live. It is a computer-assisted trim, not free thrust: each ship-local axis caps around 2 u/s, so RCS settles the last few meters of an approach rather than replacing the main drive. RCS is a controller verb like the autopilot ones, granted per ship - the mainline campaign flies with it withheld (the `[SHIFT] RCS` hint only appears when a scenario grants the verb), so a ship opts in per scenario.
- The GOTO and STOP autopilot verbs now settle their arrival on RCS: in the last stretch, once the desired velocity is near zero, the autopilot brakes with the fine thrusters instead of coarse main-drive pulses, so a ship eases to a stop instead of pulsing on the spot (the same RCS burn loop plays while it does).

### Combat & Weapons

- Turrets can now carry more than one barrel: a mount fires and aims every muzzle it has, so twin-barrel PDCs (and stranger multi-gun mounts) throw two streams at once, each at its own fire rate. All the barrels share the turret's one magazine, so a twin barrel empties it twice as fast.
- Hostiles now respect line of fire: a gunner with an asteroid (or any other tangible body) between its muzzle and you holds fire instead of hosing the cover, and holds torpedo launches the same way - its normal attack orbit keeps it circling, so the pressure resumes only once that motion brings the angle back. Breaking line of sight behind hard cover now actually relieves the pressure instead of just soaking bullets. Intangible volumes (beacon rings, trigger zones) still stop nothing, and point-defense fire at inbound torpedoes is exempt on purpose.
- Weapons auto-reload: a magazine that runs dry now refills on its own instead of leaving the weapon dead for the rest of the scenario. The PDC turrets dump their magazine then reload to full after a few seconds; the torpedo bay slowly rearms one torpedo at a time. Because a spent weapon always comes back, magazine size is now a fire-pacing beat rather than a hard fail, so finite ammo is on by default - the New Game (Shakedown Run) player flies with real ammo and the diegetic ammo readout instead of the old unlimited-ammo intro.
- Locking is no longer instant: once the radar settles on a target you now hold steady through a short lock-on dwell before it commits, and the dwell is longer the farther away the target is. Sweeping off the target before it fills cancels the acquisition, and re-designating to a new target earns a fresh dwell while the old lock holds - so a lock is something you earn by keeping the target under your aim, not granted the moment you point at it.

### Ships & Sections

- Enemy ships now show battle damage too: knock out one of a hostile's sections and it reads burnt-black, so you can see at a glance which of its components you have already destroyed. Unlike your own ship - which reddens and darkens gradually as each section wears down - enemies only show the black endpoint, staying pristine until a section is actually destroyed or disabled. No intermediate red on the enemy hull.

### Scenarios & Objectives

- The whole campaign and The Ledger got a storytelling pass: every fight now announces itself and its attackers fly in on a readable arrival grace before going hot (the base ambushes and the ledger's waves alike; ships that already spawn far keep an ungraced entrance, and the finale's Auditor keeps its by-design hot drop); opening comms beats are spaced on the scenario clock instead of arriving as a burst; and the closing lines that used to fire behind the victory banner (where nobody could ever read them) now live in the banner itself. The beat-sheet convention is documented for scenario authors, and the content lint enforces its two hard rules.
- Scenario transitions have a middle gear now: a `NextScenario` with `linger: false` can carry a `delay` - the world keeps playing for those seconds (long enough to read the closing line) before the cut - and a Victory/Defeat banner can carry `auto_advance_secs`, advancing its queued chain by itself after that many real seconds if the player does not click first. A content lint warns on the classic trap of pairing an Outcome with a non-lingering switch in one handler (the overlay gets swallowed - or with a delay, frozen).
- The Ledger finale's Auditor traded its top-tier turret for the light mook gun (both ending branches): its drama is the close entrance and the torpedo you screen with your PDC, not an instant shred - the playtest-decided answer to the balance audit's finding on the finale.
- Broadside joined the checkpoint rework: the chapter now plays as two scenarios - break the corvette ambush (a Victory beat marks the checkpoint), then Continue into the gunship fight, which retries itself on death instead of sending you back through the whole approach and ambush. Five invulnerable boulders now anchor the hauler fight and the gunship lane (the destructible rock field stays as chaff), so the new hold-fire-behind-cover rules have something solid to work with in chapter two's biggest fight.
- The Ledger chapter two was rebuilt around fair fights (the first content pass of the difficulty rework): wave one's Magpies now carry the light mook gun instead of the top-tier turret, burn in from a single ~600u lane instead of materializing point-blank on both flanks, and an invulnerable boulder field between their lane and the Dray Mule gives the new line-of-fire rules something to work with. The chapter now plays in two acts across two scenarios - breaking wave one is a checkpoint, so dying to the reinforced heavies (who arrive from the opposite lane, ~950u out, with exactly one big gun between them) retries the heavies, never the whole chapter. The Mule holds station high off the fight plane, out of the stray-fire lanes, and Okono now tells you the play: their guns only want you, so drag the fire wide of her.
- Every combat scenario now flies real, auto-reloading ammo: Broadside and the example mod's arena joined Shakedown Run in dropping the old unlimited-ammo setting, so you manage magazines and reloads (and see the diegetic ammo gauge) everywhere you fight. Unlimited ammo remains available to scenario authors for testing and debug ships.
- Scenarios can declare a win/lose: the new `Outcome` action shows a VICTORY/DEFEAT overlay with real buttons (Continue/Retry riding a queued lingering `NextScenario`, Main Menu always) instead of the old silent press-Enter-to-restart; dying in Shakedown Run now presents Defeat + Retry. The overlay freezes the simulation the same way the pause menu does - physics, AI, weapons and timers stop behind the banner (Enter/Continue/Retry stay live) instead of the world running on underneath it.
- Broadside, chapter two of the base storyline: answer a neutral hauler's distress call, break a two-corvette ambush, then screen the gang gunship's torpedoes with your PDC and break it apart - the capital-combat slice, in the Scenarios picker and chained from Shakedown Run's victory screen.
- Ships in scenario RON can author their side: an optional `allegiance` field (`Some(Neutral)` for bystanders like Broadside's hauler) overrides the controller default.
- The Asteroid Field sandbox joined the outcome frame: reaching the zone declares Victory (Continue reruns the field) and dying declares Defeat + Retry, replacing its silent switches.
- The base Demo Scenario (the ship-less RON authoring demo) was removed from the game and the Scenarios picker; the example mod's arena is the worked hand-authored RON example now.

### Fixes

- The Rust Tally's side mounts now actually mount: both flank turrets and both torpedo tubes roll so their bases seat against the gunship's spine instead of all pointing the same way off into space, tube hatches face outboard, and the port/starboard section ids match the sides they are on (they were swapped). The roll also reshapes its firing arcs the way a broadside gunship should fight: one gun per beam, both on the bow, and the previously blind stern covered.
- The Ledger finale's Auditor no longer carries its torpedo bay INSIDE its hull: the bay was authored half-embedded in the spine (visibly clipping through the controller and bow sections) and now mounts flush on the bow's flank, seated base-to-hull. A new content-lint check errors on any ship whose unit-cube sections overlap, so this whole class of authoring slip is caught at build time now.
- Broadside's deep-field skybox (and the gauntlet/ledger mods that reuse it via `dep://base`) now gets its 6-face cube layout at load time: the alt cubemap's `.meta` sidecar was ignored by the asset config, leaving the raw 4096x24576 stacked image to be uploaded as-is if the scenario tore down mid-load - a fatal error on GPUs with a 16384 texture limit (WebGL2-class hardware, CI's software renderer).
- Mods that ship their OWN skybox now get the same protection: the asset config reads every asset's `.meta` sidecar (not just a hand-listed set of base paths), so a mod cubemap's `RowCount` layout applies at load time whether the mod is shipped or portal-downloaded. Previously a mod's own cube loaded as a raw stacked image and hit the same WebGL2 upload crash if a scenario tore down mid-load.
- Restarting a scenario (Retry, from the pause menu or the Victory/Defeat frame) no longer loses the objective text: the objectives HUD is now reset on scenario teardown, so a restart that re-posts the same objectives repaints the panel instead of leaving it blank.
- A ship that loses its last section outside the damage path no longer lingers as a targetable 0-HP ghost hull: the health aggregate now carries a structural death backstop (no living sections = dead), closing a soft-lock where Broadside's act gates waited on a kill that never registered.
- Scenario `OnUpdate` handlers now freeze while the game is paused (the pause menu or the Victory/Defeat frame): the per-frame pulse is gated on the unpaused state, so an already-satisfied handler no longer keeps re-applying its action every frame behind the pause.

### Modding & Mod Portal

- Turret mounts are now an arbitrary joint tree **(breaking)**: a `Turret` section describes its gun as a `root` joint with recursive `children` instead of the old fixed yaw/pitch/barrel fields, so a mount can have extra hinges, a rotator chain, or elevation that lives several joints down - each joint is an `offset`, an optional `axis` to make it a hinge the aim solver steers, an optional `render_mesh`, and an optional `muzzle` fire point. The aim solver now distributes the lead across whatever hinges a muzzle hangs below, so novel mounts track targets without bespoke code. Existing turret content is migrated (the stock turrets aim and fire exactly as before); a mod authoring a turret must move its `yaw_speed`/`*_offset`/`render_mesh_*`/`fire_rate` fields into the joint tree (the dev wiki's section-authoring guide has the new shape). `fire_rate` is now per-muzzle; `muzzle_speed` stays section-wide. The content lint checks turret joint trees for the mistakes the parser accepts but the game cannot use - a hinge with a zero axis or non-positive speed, min/max limits the wrong way round, or a mount with no muzzle at all - so a broken turret fails the build instead of misbehaving in play.
- Scenarios can tell time **(breaking)**: the engine now maintains a reserved `scenario_elapsed` variable - seconds of live, unpaused scenario time (pausing freezes it; a retry restarts it) - readable from any expression filter, so authors can finally write timed story beats, breathers and late reinforcement waves with the existing gate vocabulary. Writing the reserved clock is a content-lint error (a pre-existing mod that used the name as its own variable must rename it); the example mod's arena ships a timed comms nudge and a timed bonus spawn as the copy-me pattern.
- The two `OnOrbit` / lock-echo timing windows are now author-tunable per ship instead of a fixed 5 seconds: an AI controller can set `orbit_hold_secs: Some(8.0)` for how long a ship must hold its orbit before `OnOrbit` fires (and re-fires), and the player controller can set `lock_refire_secs: Some(8.0)` for how often a held travel/combat lock re-fires. Both default to 5s when omitted (existing scenarios are unchanged), and a non-positive value is a content-lint error.
- Mods can ship their own art: a bundle manifest gains a `resources` list of the binary files (textures, skyboxes, models, audio) it packages, and content references them with a `self://` path (`cubemap: "self://textures/nebula.png"`) that resolves against the mod's OWN folder instead of the base game - so a mod's look is its own, whether shipped or downloaded. A `self://` ref must name a declared resource or the mod fails its gates.
- Cross-mod and base-game art references, and one canonical way to write them **(breaking)**: an asset path in content now always carries a scheme - `self://` for your own folder, `dep://<id>/` for a file a DECLARED dependency ships (a shared art pack, reused without copying bytes), and `dep://base/` for a base-game asset. The base game became a normal self-contained mod - its art moved under `assets/base/` and it is referenced like any dependency (`dep://base/textures/cubemap.png`, `base` is implicit so you never declare it). The old bare, scheme-less path (which used to mean "a base-game file") is retired: a bare asset ref is now an error at author/publish time. Mod authors reusing base art must write `dep://base/<path>` instead of a bare path.
- One `example` mod is now the single copy-me tutorial mod, folding together the earlier scattered demonstrations: it shows a section overlay, a brand-new section, a playable arena scenario, its own shipped skybox and texture via `self://`, a `StoryMessage` comms beat, Victory/Defeat `Outcome`s, and a `menu_backdrop` scene - a little of everything a modder can do, in one self-contained folder to copy.
- Scenarios gained a `menu_backdrop` flag: on every menu entry the game picks one flagged scenario at random, so mods can ship their own menu ambience scenes (the base backdrop is just the first of them). The New Game start moved from code into the base bundle's `new_game_scenario` declaration - honored only from the base game, so mods cannot redirect it.
- Two new menu backdrops join the rotation: Waystation Traffic (a hauler convoy circling a freight stop under amber dock lights) and Scrapyard Drift (a quiet salvage yard - drifting crates, two wrecks and a lone tug).
- Scenarios can speak: the new `StoryMessage` action shows speaker-attributed dialog in a HUD comms panel (`OKONO > Strip it clean.`) - the story-text surface campaign mods build on.
- Broken mod content fails loud, not weird: scenarios with reference errors (unknown prototypes, dangling chapter chains) refuse to start with a FAILED TO START report naming each problem, and the menu backdrop rotation skips them.
- `cargo run -p nova_assets --bin content -- lint` lints every scenario in the repo (base, shipped mods, portal mods) for the reference bugs the loaders cannot see - unknown section prototypes, dangling chapter chains, filter targets nothing spawns; CI enforces it. Mod developers can check just their own mod with `--target <mod dir or id>`.
- The Ledger, the first campaign mod on the portal: a four-chapter salvage arc (strip the wreck, break the claim jumpers, run the quiet channel, pick the buyer - or the buoy) with comms-driven story beats and a two-ending finale. Install from Mods > Explore.
- Mods can ship and reference sounds like any other binary resource: list a wav in `resources` and reference it with `self://` (or `dep://base/sounds/<name>.wav` for a base cue). Weapon and controller sections now fully own their sounds: the turret's `fire_sound` and `dry_fire_sound`, the torpedo bay's `launch_sound`, the controller's radar/lock/safety ticks (`lock_on_sound`, `lock_off_sound`, `radar_deny_sound`, `radar_retarget_sound`, `safety_on_sound`), and every damage target's hit/destruction voice (`impact_sound` + `destroy_sound` on any section's base block and on asteroids, `detonation_sound` on the torpedo bay - per-target means per-material: a rock, a light hull and a reinforced hull can each sound different), the thruster's engine hum (`loop_sound` - thrusters sharing a sound share one loop, so two mods' ships can hum differently side by side), and the salvage crate's `pickup_sound` are authorable asset refs like the meshes. Every world sound is now content-owned - a sound plays because something you can author declares it - a modded turret sounds like its own gun, and a section that authors no sound is silent (the base sections author all of them, so the stock game sounds the same). More section sounds are on the way. The base game's world sounds ship with the base mod under `assets/base/sounds/`; UI sounds (menu clicks, objective chimes) are engine chrome at the asset root, not mod content.

### Interface & HUD

- Story radio chatter is readable now: comms lines queue and display in arrival order (a burst no longer overwrites its own first line), each line fades in with a soft blip, holds the screen for its dwell (authors can set a per-line hold), and yields gracefully when another line waits. Newly posted objectives flash gold in the ghost column beside the panel, so a mid-fight objective change registers at a glance; leftover ghost lines now vanish with the scenario instead of fading over the menu.
- The diegetic ammo gauge now shows reload: while a weapon reloads, the pips above its live rounds fill back up as a sweep in a dimmer shade of the loaded round's color, proportional to reload progress - a spent turret ring fills from empty to full, a rearming torpedo bay lights the rounds coming back. Loaded-type color and remaining-rounds count are unchanged.
- The pause menu (Esc) gained a Retry button: restart the current scenario from scratch without going back through the main menu. Offered only while a scenario is live, so the editor's build mode does not show it.
- The Settings menu is real content instead of a placeholder: a draggable master audio volume slider, a Low/Medium/High graphics-quality preset, and a read-only reference of the current keyboard and gamepad bindings (flight, targeting, camera, pause). Reachable from BOTH the main menu and the pause menu (the same modal, both entry points), and remembered across restarts - a config file on native, browser storage on the web. The graphics preset tunes the combat juice (High keeps camera shake and hit flashes, Medium drops the shake, Low turns the juice off) and now also gates the heavier visuals for low-end machines (see Performance): Low is spawn-less (no particle bursts) and renders the world at a reduced internal resolution, Medium keeps particles.

### Performance

- The Low/Medium graphics-quality presets now skip expensive visuals for low-end machines, not just the combat juice: Low is spawn-less (torpedo blast/launch and turret muzzle particle bursts are not spawned at all), and Medium keeps particles. High is unchanged.
- The Low graphics preset now also renders the world at a reduced internal resolution (~70%, roughly half the pixels) and upscales it to the window, a lever aimed at weak fill-bound hardware (laptop iGPUs, phones), at the cost of a slightly softer world (the HUD and menus stay crisp and fully clickable). Medium and High are untouched - they render at full native resolution as before. Native and web both honor the setting. (On a strong discrete GPU the win is small - that hardware is not fill-bound - so this is a knob for the low end, not a general speed-up.)

### Internals & Tooling

- The content authoring tools are one CLI now: `cargo run -p nova_assets --bin content -- <gen|lint|audit>` replaces the former separate `gen_content`, `content_lint` and `balance_audit` binaries. Same behavior per subcommand (`gen` writes the base `.content.ron`, `lint [--target <mod>]` runs the reference/geometry checks, `audit` grades combat balance); one tool to remember instead of three.
- The content lint now checks mount seating: a turret or torpedo bay whose base face (local -Y under its authored rotation) does not sit against an occupied neighbor cell of its own ship is an Error, at build time and in the in-game mod gate alike - the exact class behind both recent wrong-roll bugs (the Auditor's embedded bay, the Rust Tally's four side mounts). Non-quarter-turn mount rotations are skipped with a warning, matching the overlap check's conservative stance on free angles.
- The two 5-second scenario-event windows - the orbit-hold check behind `OnOrbit` and the player-lock re-fire behind `OnTravelLock`/`OnCombatLock` - now measure their elapsed time against the engine scenario clock (`scenario_elapsed`) instead of each accumulating its own frame delta. Pause, teardown and retry now freeze and reset both windows in one place, so their pause-correctness no longer rides the implicit "virtual time is frozen while paused" assumption. No change to when the events fire.
- The balance audit learned acknowledgments: a warning that is intended drama can be acked in `balance_acks.ron` with a reason and the deciding task - it still prints (tagged ACK) but stops counting, errors can never be acked, and a stale ack fails CI until pruned. The Auditor's torpedo-envelope warning is the first acked entry.
- `cargo run -p nova_assets --bin content -- audit` derives every combat scenario's fairness sheet from the shipped content (hostile dps, weapon envelopes, spawn distances, time-to-kill vs the player ship, cover tiers) and grades two findings: an armed hostile opening a scenario inside its own effective range of the player spawn is a CI-failing error, and a triggered hostile spawning inside its own envelope is a warning. First run already earned its keep: it surfaced the Ledger finale's Auditor spawning hot on both ending branches (now tracked for a playtest verdict).
- Firing no longer spams the log with an avian "no mass or inertia" warning per round: turret bullets, which use a near-zero mass and a sensor collider, now carry an explicit angular inertia, removing the warning and its NaN risk.
- Debug builds (`--features debug`) bind F12 to a screenshot that saves the primary window to your Downloads directory as `<timestamp>.png`.

## [0.6.0] - 2026-07-16

### Scenarios & Objectives

- Scenarios picker on the main menu: two-pane overlay listing every base and mod-added scenario, with a details pane and Play button; scenarios gained optional `thumbnail` and `hidden` fields.
- New `SetSkybox` action swaps the skybox cubemap mid-scenario, deferred until the image loads so a bad path leaves the sky unchanged.

### Modding & Mod Portal

- Mod dependencies resolve end to end: installs auto-pull missing deps, enabling a mod auto-enables its transitive deps, disabling a still-depended-on mod is refused, and merge order is dependency-respecting topological (ids only, no version constraints yet).
- Static mod portal now publishes on every deploy: validated, sha256-hashed bundles under `/mods/<id>/<version>/` with a generated `catalog.json` (first mod: Gauntlet Run).
- Local mod cache foundation: a `mods://` asset source, a RON installed index, and downloaded bundles that load and merge through the same pipeline as shipped ones.
- Portal client fetches `catalog.json` and installs/uninstalls over the wire on native and web, staged installs verified against size + sha256 and committed only once every file checks out.
- Explore online tab is real: browse the portal in-game and install/update/uninstall on native and web, with per-file progress, offline catalog fallback, and enabled-state-preserving updates.
- Mods menu is a two-pane Factorio-style screen: `Installed` | `Explore online` tabs over the scrollable list with per-row enable checkboxes and a details panel from bundle meta.
- Mods menu hides dev/tooling mods: `hidden: true` entries stay installed and code-enableable by id but drop out of the player list.
- **(breaking)** Mod metadata moved into each `*.bundle.ron` as a `meta` block; `assets/mods.catalog.ron` slimmed to a thin pointer list (catalog and out-of-tree bundle format break).

### Web & Platform

- Particle effects (muzzle flash, projectile trail, torpedo launch/detonation bursts) now render in the web build after moving from WebGL2 to WebGPU.
- Browsers without WebGPU get a clear "WebGPU required" message instead of a black canvas, plus a heads-up under the landing page's "Play in browser" button.

### Performance

- Modding event dispatch is indexed by event name (upstreamed to bevy-common-systems rev 4c81117): 17-24% faster under bursts of 500-5000 handlers, neutral at the realistic one-event-per-frame rate.
- Added a criterion benchmark for the scenario-dispatch hot path (`cargo bench -p nova_scenario --bench scenario_dispatch`); the measure-first gate is documented in `tasks/20260714-083331/modding-perf-report.md`.
- Sibling filter-key-interning and condition-eval-compile optimizations were measured and deferred: at realistic event rates their per-handler cost is noise, kept as documented insurance.

### Fixes

- Scenarios picker no longer crashes the renderer on a non-2D thumbnail: such thumbnails are skipped with a warning, and images mount only once loaded.
- Local mod-portal web testing no longer needs a cross-origin `?portal=` override: `scripts/preview-web.sh` serves the portal same-origin as the game, matching production.

### Internals & Tooling

- Screenshot Reel capture set no longer ships in the game assets: its scenario moved into the example that films it, so players and the web build stop downloading a capture tool.

## [0.5.2] - 2026-07-14

### Gameplay & Flight

- Enemies can be authored to ARRIVE instead of appearing: a scenario ship with an `engage_delay` grace flies its patrol and holds its fire for those seconds before going hot - shoot it and the courtesy ends instantly and permanently, and its point-defense never stops watching for torpedoes. Paired with a warning comms beat, a spawn now reads as an approach you were told about, not an ambush from nowhere.
- Gamepad bindings rounded out: ORBIT -> South, scenario-advance confirm -> DPadDown, and HUD cycle / pause / back-to-editor gained buttons.

### Web & Platform

- The site grew a full wiki (gameplay, ship sections, keybinds, world and meta pages), two new devlogs, and a tutorial trimmed to first-scenario onboarding.

### Fixes

- Thruster hum now attenuates with distance per ship, so another ship's or torpedo's burn no longer plays at full volume from anywhere (your own ship stays exempt).
- Scenario teardown no longer logs an "Entity despawned" command error when a ship despawns with autopilot engaged: the telemetry-cleanup observer uses the fallible remove variant, pinned by regression tests.
- Debug inspector's window-camera placement fix moved upstream into bevy-common-systems (rev 4a743b2), deleting nova's local workaround.

### Internals & Tooling

- CI's example smoke suite is BLOCKING again: the GitHub-runner-only taffy panic is gone, so the 12-example suite gates every push.
- Examples are a testable curriculum: twelve numbered examples from controller PD through the boot flow, each self-driving under BCS_AUTOPILOT with behavior assertions and a completion backstop, all on the CI smoke list.
- Example smoke now fails on ANY logged command error, closing the gap where handled remove/despawn warnings (the stale-entity teardown race) sailed past the panic gate.
- Weapon test ranges fire again: the weapons safety had silently disarmed both, so their scripts raise the stance first, with new assertions pinning the fire -> hit chain.

## [0.5.1] - 2026-07-13

### Fixes

- Web build no longer quits with a fatal render validation error on New Game / editor Play: the target inset dropped its `view_formats` override (unsupported on WebGL2) for a plain sRGB target.
- Skybox cubemap's `.meta` loader settings now actually apply in the shipped app: `AssetMetaCheck::Never` had silently ignored them, resurrecting the oversized-upload race on GPUs with a 16384 texture limit.

## [0.5.0] - 2026-07-13

### Gameplay & Flight

- Diegetic flight readouts replace the bottom-left status text: speed and engaged-mode chips beside the velocity sphere, an ORBIT radius spoke, and a nav-cyan tint while the autopilot flies.
- Keybind cluster is contextual: rows appear only while their verb can act; scenario-emphasized keys show early, pulsing gold.

### Combat & Weapons

- Deliberate radar locking replaces all passive targeting: hold CTRL to sweep and live-lock what you look at, stance picking the slot (lowered NAV vs raised combat) and a tap to clear; locks stick until the target dies, leaves range, or goes cold.
- Typed damage (Kinetic / AP / EMP / Explosive) against per-section resistance tables; each turret carries a loaded-ammo slot setting its rounds' type, with a color-coded ammo readout.
- Lock language is slot-colored: RED bracket = combat lock, WHITE = nav lock; relation tint and reticle pips retired, turrets hold the combat lock even during manual aim.
- Turret rounds curve through gravity wells, like ships and torpedoes.
- PDC retuned to point defense: per-hit damage 20 -> 4, so the stream chips targets down over a visible burst instead of one-shotting them.

### Ships & Sections

- Editor: placed sections show their bound key as a chip and can be clicked to rebind (keys or mouse buttons); the build panel scrolls.
- Editor play-test ship is now a passive target instead of an AI combatant.

### Scenarios & Objectives

- Shakedown Run: New Game starts a ~12-beat tutorial (burn, freelook, stop, salvage, GOTO, gravity coast, ORBIT, radar lock, live-fire rehearsal, scavenger fight); each beat teaches one gesture and completes the instant it lands.
- Objective conveyance: gold marker chip with live distance to the current target, salvage-crate glow and brackets, keybind emphasis pulses, completion chime and posting blip.
- Scenario primitives: nav beacons and salvage crates (authorable radar signatures), despawn-by-id, and new events/actions (OnOrbit, OnTravelLock/OnCombatLock, SetSpeedCap, objective markers, hint emphasis).

### Interface & HUD

- Target viewfinder: a corner inset renders a live magnified 3D view of the combat lock via a second camera; red armed frame while hot, NO-SIGNAL for non-scopeable bodies, and a ~2 s freeze-frame kill cam on death, the fine-locked section glowing in both views.
- Main menu (live ambient scene: an AI ship flying a thruster-driven orbit) and ESC pause menu; the game now boots into the menu (new `nova_menu` crate).
- HUD visibility levels: grave/tilde cycles ALL -> MINIMAL -> NONE.

### Web & Platform

- Web landing site (`web/`: TypeScript + Webpack + Tailwind); the Pages deploy serves it at the root with the game under `/play/`.

### Fixes

- Debug builds no longer crash when a scenario advances (the smoke assertion now covers only the boot load), and teardown no longer warns about despawned entities.
- Asteroid kills emit OnDestroyed under the scenario id so scripts can react (this had soft-locked the derelict beat).
- Target inset zooms combat bodies only (ships, torpedoes, asteroids), not beacons, and frames section-less bodies by collider bounds.
- Debug inspector stays on a window camera instead of leaking into render-to-texture cameras.
- HUD apparent-size elements (reticles, brackets) measure real colliders, not invisible trigger volumes.
- Turret bullets are sensor projectiles: same damage, no physical shove, expended on first contact.
- Many Shakedown playtest fixes: park points inside beacon triggers, orbit-hold completion, scavenger spawn timing and a 150u combat leash, invulnerable planetoids, speed-cap teaching, objective text wrapping and sound pacing, readable gold pulses.
- F1 back-to-editor is sandbox-only; the debug ammo readout no longer lingers when debug mode is off.

## [0.4.1] - 2026-07-10

### Internals & Tooling

- release-flow: install the `x86_64-apple-darwin` std for the pinned nightly, fixing the macOS universal build.
- CI: one `--features debug` feature set across clippy/tests/examples (one Bevy build), cache saved on failure, windowed examples smoke as a separate non-blocking step.

## [0.4.0] - 2026-07-10

### Gameplay & Flight

- Flight-assist overhaul: assisted velocity-hold mode (WASDQE nudges, X brake latch, soft cap), Z direct Newtonian mode, RCS budget, FA/speed readout.
- Flight computer balances thrust through the live center of mass: differential throttle nulls burn torque, off-axis thrusters recruited for counter-torque.
- Mass-legible handling: turn rate derived from the torque budget and live inertia (stripped ships snap, heavy builds lumber); max_torque 100 -> 40.
- Chase camera: smoothing on all gameplay modes plus a burn push-back lean.

### Combat & Weapons

- Torpedo guidance: proportional navigation, angular lock-on aim-assist, arming gate, blast-radius visual, launch particle burst.
- Player targeting arc: signature auto-acquisition (550 m), focus dwell, per-section fine-lock with aim-snap and cycling, HUD lock markers and focus meter.
- Turret auto-aim with true intercept lead, fed by fine-locked section, else live structure, else camera ray.
- Faction/relation model (hostile/neutral/own) drives acquisition, projectile allegiance, and reticle tint.
- AI combat wave: behavior state machine (Idle/Patrol/Engage/Evade/Retreat), fire discipline, point-defense priority on inbound torpedoes, standoff orbit/strafe, patrol routes, threat-memory evasion, enveloped torpedo launches.
- Player lock range 2 km -> 20 km (AI sensor scan unchanged).

### Interface & HUD

- HUD substrate: screen-projected indicators (entity/point anchors, apparent-size sizing, clamp-to-edge arrows), turret lead pips, and a locked-target readout (range, closing speed, health).

### Audio & Visuals

- First audio: placeholder SFX (explosions, impacts, turret fire, torpedo launch, throttle-tracking thruster loop) with distance attenuation and throttling.
- Combat juice: trauma-model camera shake and expanding hit/impact flash rings, distance-attenuated and throttled.
- The SFX/juice listener is an explicit marker on the gameplay camera, not "any Camera3d".

### Fixes

- Skybox cubemap reinterpreted into a 6-layer array at load time; the raw 24576 px image exceeded smaller GPUs' texture limit and killed the app.
- Blast damage reaches every body overlapping the blast, not just one.
- Ships, asteroids and torpedoes interpolate between physics ticks (no camera twitch); the chase camera anchors on the live center of mass.
- Projectiles no longer collide with their shooter; shot-down torpedoes die whole and blast-free; destroyed asteroids no longer leave rigid-body husks.
- Section overkill is absorbed instead of propagated; a disabled controller stops torquing the hull.
- Bullet lead solved in the shooter's frame so a moving shooter's rounds land; the AI helm writes slewed absolute rotation commands.
- One hit plays one cue: audio/juice observers ignore propagation re-entry.
- Misc: editor preview controller made inert, turret resting position, one-frame origin snap on camera-mode switch.

### Internals & Tooling

- Example test ranges (`06_torpedo_range`, `08_turret_range`, `10_gameplay`, `11_com_range`) with live tuning sliders, FPS/version overlay, and a headless autopilot + screenshot smoke harness that asserts scenario init.
- CI workflow: fmt, clippy, and the workspace test suite (windowed examples under Xvfb) on every PR and push to master.
- Integrity, health, blast and mesh-slicer systems consumed from `bevy-common-systems` instead of in-tree copies; torpedo section split into its own module.

## [0.3.1] - 2026-07-07

### Audio & Visuals

- Post-processing camera component.

### Internals & Tooling

- **(breaking)** Upgrade to Bevy 0.19 (avian3d 0.7, bevy_rand 0.15, bevy_enhanced_input 0.26, rand 0.10); anything built against nova, including from-source mods, must move to the matching versions.
- `bevy_common_systems` externalized as a git dependency; vendored copy removed.
- Added `AGENTS.md` and a `docs/` folder (architecture, scenario system, sections, development, migration notes).

## [0.3.0] - 2025-11-29

### Combat & Weapons

- Torpedo bay section and blast damage.

### Ships & Sections

- Per-section health system.

### Scenarios & Objectives

- OnEnter/OnExit zone events.

### Audio & Visuals

- Improved directional and thruster shaders.

## [0.2.1] - 2025-11-15

### Modding & Mod Portal

- Modding documentation and examples; event system refactor.

## [0.2.0] - 2025-11-08

### Modding & Mod Portal

- Game events and a queue system; scenario and modding capabilities.

### Scenarios & Objectives

- Asteroids with procedural mesh and dynamic destruction.

## [0.1.0] - 2025-10-21

### Ships & Sections

- Basic spaceship sections.

### Scenarios & Objectives

- Editor and simulation scenes.

[unreleased]: https://github.com/alexjercan/nova-protocol/compare/v0.7.0...HEAD
[0.7.0]: https://github.com/alexjercan/nova-protocol/compare/v0.6.0...v0.7.0
[0.6.0]: https://github.com/alexjercan/nova-protocol/compare/v0.5.2...v0.6.0
[0.5.2]: https://github.com/alexjercan/nova-protocol/compare/v0.5.1...v0.5.2
[0.5.1]: https://github.com/alexjercan/nova-protocol/compare/v0.5.0...v0.5.1
[0.5.0]: https://github.com/alexjercan/nova-protocol/compare/v0.4.1...v0.5.0
[0.4.1]: https://github.com/alexjercan/nova-protocol/compare/v0.4.0...v0.4.1
[0.4.0]: https://github.com/alexjercan/nova-protocol/compare/v0.3.1...v0.4.0
[0.3.1]: https://github.com/alexjercan/nova-protocol/compare/v0.3.0...v0.3.1
[0.3.0]: https://github.com/alexjercan/nova-protocol/compare/v0.2.1...v0.3.0
[0.2.1]: https://github.com/alexjercan/nova-protocol/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/alexjercan/nova-protocol/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/alexjercan/nova-protocol/releases/tag/v0.1.0
