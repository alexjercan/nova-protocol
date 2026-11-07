# D3: can a save older than the open_world `Take a job` cue load?

Verdict: UNREACHABLE for every pre-cue save class tested or reasoned about.
The Load refuses it before any scenario runs, so the first-accept WARN
(`remove_objective`, world.rs:638-651) cannot be reached from an older save.
Only a save with the CURRENT catalog loads, and (source) that save always holds
the `take_a_job` objective. Tree is dirty (cue uncommitted); binary built from it.

## Claim and evidence

### 1. The digest covers handlers and actions (source)
- `content_catalog_digest` hashes `(source mod, item)` for every effective Content
  item: crates/nova_assets/src/merge.rs:978-1013. Version and bundle metadata too.
- `Content::Scenario(ScenarioConfig)`: crates/nova_modding/src/lib.rs:84,91.
- `ScenarioConfig` derives `Serialize`; its `events: Vec<ScenarioEventConfig>`
  has no skip: crates/nova_scenario/src/loader/mod.rs:339-345, ~404.
  `ScenarioEventConfig` serialises `name`, `once` (skipped only when false),
  `filters` and `actions` (skipped only when empty): loader/mod.rs:544-585.
  So the new Objective, ObjectiveChipHold and `OnJobAccepted` handler change the bytes.
- The canonical writer tags and length-prefixes every value, sorts maps:
  crates/nova_assets/src/merge/canonical.rs:1-8,37.
- The uncommitted diff of assets/base/scenarios/open_world.content.ron adds exactly those.
- Test that a changed section changes the digest: merge.rs:1595-1606 (not run).

### 2. Every route to a save runs read_header (source)
- `read_header` refuses `header.catalog != loaded.digest.0` as `WorldRefusal::Catalog`;
  text `content changed; saved with <mods>` (save/mod.rs:1852-1878, 441).
  `WORLD_SAVE_VERSION` is "v0.16.0" (mod.rs:100), so the format check does not refuse; the catalog does.
- Only callers of `read_header`: `list_worlds` (mod.rs:494), `open_world` (mod.rs:549),
  `check_world` (mod.rs:584). The only state-file read is `read_state` (mod.rs:598),
  reached only after `read_header` in `open_world`/`check_world`.
- `resume_world` (session.rs:275) takes an already opened `(folder, lock, header, state)`.
  Its callers: load_screen.rs:563 (after `open_world_task`) and leave.rs:174 (after `open_world`).
- Routes:
  - Load row (list): `list_worlds` via menu_ui.rs:93 / load_screen.rs:167,
    plus a whole-state `check_world` per row (load_screen.rs:186). Check runs.
  - Load button: `open_world_task` -> `open_world` (load_screen.rs:355-371,402). Check runs.
    Button gets `InteractionDisabled` on a refused row (load_screen.rs:1375).
  - Continue: `on_continue_world` -> `open_world_task` (load_screen.rs:499-533). Check runs.
  - Pause "Load last save" / defeat "Load last save" / Retry: leave.rs:164 `open_world`. Check runs.
  - leave (Main Menu, Exit) only writes; does not open.
  - No env var or CLI flag loads a world (memory drive-menu-load-on-xvfb, 2026-10-21;
    I grepped all callers above; no other caller exists). No bypass found.

### 3. A current-catalog save with no objective and an unspent accept handler (source)
- Not reachable. Objectives and spent handlers are saved together in one `SavedScenario`
  (resume.rs:40-72; session.rs:743-765).
- OnStart runs the Objective push inside the handler dispatch
  (`push_objective` is direct: actions/mission.rs:39). Only the spawns are queued
  and drained over frames (world.rs:30-50). A save needs one player ship
  (session.rs `snapshot_world`), and the player spawn is queued by that same OnStart,
  so a save cannot exist before the Objective is pushed.
- Order of the handler: open_world.rs:41-50 puts Objective after the spawn list; same dispatch.
- OnResume re-applies the hold only (open_world.rs:43-50); the objective comes from the save.
- OnJobAccepted is handler index 3 (open_world.rs:88-99). After accept it is spent
  and the objective is removed in one dispatch; a later save holds neither.
- Unverified: I did not run a save in the window between accept and the handler.
  Both run in one event dispatch; this is a source claim only.

### 4. 15 changed files vs the v0.16.0 tag (source)
`git diff --stat v0.16.0 HEAD -- assets/base`: base.bundle.ron, items, menu_duel,
sections, ships, stations, trade_profiles, training content, plus README and 6 wav.
These are committed, so even without the uncommitted cue the loaded catalog digest
differs from one a v0.16.0 build wrote (items, ships, sections, stations, trade_profiles,
training and the bundle are Content or bundle metadata hashed at merge.rs:978-1013).
A shipped v0.16.0 save is therefore refused as `Catalog` (source; not run, see Limits).
The wav files are probably not hashed, which does not change this.

## Fixtures (runtime facts)
Header `format` and `game_version` are the same string on all fixtures ("v0.16.0",
"0.16.0"), so they do not tell the build. No fixture was written by a v0.16.0-tag build.

| class | path | catalog | written | has take_a_job |
|---|---|---|---|---|
| dev, before the cue | ~/.local/share/nova-protocol/worlds/new-world | 10445671925239469238 | 2026-11-05 14:16, dev build after the tag (tag 2026-10-28) | 0 matches (runtime grep) |
| first-cue build | /tmp/nova-onboard/bench/run/profile/data/nova-protocol/worlds/new-world | 13355318929977045094 | 2026-11-06 09:04 | 1 |
| D2 build | /tmp/nova-onboard/d2/xdg/data/nova-protocol/worlds/new-world | 11602711488800918998 | 2026-11-06 11:24 | 1 |

Searched: target/news-0160-home (no world.ron), target/ and /tmp to depth 8,
~/personal/nova-artifacts (none). Only `visits-world` example profiles and the
/tmp/nova-visits-bench worlds exist besides these; not used (other mechanism/era).
Tag build not made (no worktree approved), so no v0.16.0-tag fixture exists.

## Runs (runtime)
Sandbox: /tmp/nova-onboard/d3/xdg-a/{data,config} (fresh XDG dirs; the three
world folders copied without world.lock as `devnov5`, `firstcue`, `d2build`).
Xvfb :137 (PID 1793936, stopped by PID). Game PID 1811894 (stopped by PID).
Launch, from the repo root:
`env DISPLAY=:137 XDG_DATA_HOME=/tmp/nova-onboard/d3/xdg-a/data XDG_CONFIG_HOME=/tmp/nova-onboard/d3/xdg-a/config ALSA_CONFIG_PATH=/dev/null RUST_LOG=info nix develop --command cargo run --bin nova-protocol > runA.log`
One `cargo run` rebuilt a few crates first (target/debug/nova-protocol was 11:23; no source newer),
then ran the debug binary. First attempt with `VK_DRIVER_FILES=...lvp_icd` panicked
(`Unable to find a GPU`, libLLVM wants GLIBC_2.44; log runA-lvp-fail.log). Second run used the real GPU
(menu scene only; ~6.9 GB free). Driven by xdotool with held clicks (nix shell nixpkgs#xdotool).

A. Pre-cue fixtures
- Frame shots/a02-load_000.png: Load list. `New World` (D2 build) valid; `devnov5` and `firstcue`
  `Cannot load`, tag `[INVALID]`.
- shots/a03-devnov5.png and a04-firstcue.png: detail pane text for both:
  `content changed; saved with base`. This is the WorldRefusal::Catalog text (mod.rs:441).
- shots/a05-load-click-refused.png: clicked Load on `firstcue`; button is dimmed, nothing started,
  menu stays, no log line about a world open (runA.log has no resume/refusal line; only the
  list check is silent).
- Both pre-cue fixtures refuse: the dev-build one (no objective) and the first-cue build one.

B. Positive control (same binary, same steps)
- Main menu showed `Continue / New World, saved 23 min ago` (shots/a01-menu.png): `d2build` is valid.
- Selected `New World`, clicked Load. shots/b01-after-load.png: in the world, chip
  `TAKE A JOB` with `Tab INTERFACE`, 51 fps.
- Log: `open_scenario_run: resumed 'open_world' at 0.0 s: 0 variable(s), 1 objective(s), ...
  1 spent handler(s) skipped` (runA.log line 43).
- The load wrote generation 2: xdg-a/.../d2build/world.ron has `catalog: 11602711488800918998`,
  the current digest, equal to the D2 build fixture. So the tree's digest is 11602711488800918998.
- No WARN naming take_a_job or remove_objective in runA.log (only winit, avian, mod_set, audio, AI order).

C. Not run: A did not load, so no accept was made and no WARN line exists.

## sha256 (originals, before == after, `sha256sum -c`: all OK)
- 4895bf807aa7d3c48ec0f74c6ab169e61d6149a08f1882a847a503a369f10513  ~/.local/share/.../new-world/world.ron
- b135da3779937126f36134db180fb82e685fa516056528b20482c16573c7d9b1  ~/.local/share/.../new-world/state.3.ron
- 0c55e79b97717cdfc02e117d2d51f8281a57016970da6faa08fea88f6030c72b  bench/.../new-world/world.ron
- ba3e24610968a34b19fed7e8cee0a62845efaab2ebc546513b5db80c386728f3  bench/.../new-world/state.1.ron
- d4fa30bc34fdbd97e4555106817e46a8f60c83e9e2c57d17f4a4e62b8d99e77b  d2/xdg/.../new-world/world.ron
- 4214ba906cfc154cef1905ba526959e556e9ff6d8ab9f960caf77e02970aea85  d2/xdg/.../new-world/state.1.ron
Sandbox copies of devnov5 and firstcue still match these hashes (the game wrote nothing to them).
Real ~/.local/share and ~/.config were never given to the game. The sandbox config holds only
enabled_mods.ron.

## Limits
- No save from a v0.16.0-tag build was run; the claim for shipped saves is source-only (section 4).
  The dev-build fixture is the nearest: written after the tag, before the cue, and refused.
- Catalog refusal was shown for 2 fixtures, with different digests. The refusal does not tell which part of the
  catalog differed; (source) the cue alone would suffice.
- Only the menu Load list, a Load click on a refused row and a Load of the valid row were run.
  Continue, pause "Load last save" and Retry were source-read only (they share `open_world`).
- Section 3 timing (accept vs handler) is a source claim.
- One run, real GPU, no timing claims. Frames show the refusal text and the restored chip, not appearance quality.
