# Review: Bus-and-route audio pass

- TASK: 20260824-125955
- RANGE: 6418b955..8aef9161 (18 commits)
- BRANCH: master

## Round 1

- REVIEWER: craft, performance, correctness, contracts (three passes, run one
  after the other over the same bundle)
- VERDICT: REQUEST CHANGES (sixteen findings). Fourteen fixed in 6a11d0fd and
  c07eaa1e; the rest are architectural and left for the owner.

The headline question was whether the pass is hacky - whether every sound
really goes through the engine's plugins, whether world sounds are distance
based and whether UI sounds play properly. It does, they are, and they do. The
`AudioPlayer` guard test proves no crate reaches around the engine, and the
guard was itself widened here (it only scanned `crates/` and only the tuple
form). Four route bugs were found; three were a cue on the wrong bus, and one
was a spawn/retire loop that opened a rodio sink every frame.

Findings:

- [x] A1.1 (BLOCKER) `crates/nova_ship/src/ship_audio/loops.rs` - the spawn arm
  and the retire arm used different thresholds, so a source under the retire
  level was spawned on one frame and retired on the next, forever. Idle
  thrusters and coasting torpedoes churned a rodio sink open and shut every
  frame: 600 live `SpatialAudioSink`s in the torpedo stress probe. Both arms now
  read `LOOP_RETIRE_LEVEL`. Fixed in 6a11d0fd, proved by deleting the guard and
  watching `an_idle_thruster_never_opens_a_voice_at_all` and
  `a_ship_that_stops_burning_fades_out_and_retires_its_voice` both fail.
- [x] A1.2 (BLOCKER) `crates/nova_menu/src/settings.rs` - the volume sliders
  re-emitted the same value every drag frame. Bevy's slider rounds only when
  `SliderPrecision` is set, and the repo sets it nowhere
  (`bevy_ui_widgets-0.19.1/src/slider.rs:526-533`, verified against the vendored
  source). Quantised to a 0.05 notch, with a matching `SliderStep` on the row.
  Fixed in c07eaa1e.
- [x] A1.3 (MAJOR) `crates/nova_gameplay/src/audio/voice.rs` - the exterior loop
  cap was a LEVEL threshold, not a rank, so a formation whose sources all sat at
  one range defeated it entirely. Replaced with a ranked cap that keeps the
  loudest `MAX_EXTERIOR_LOOP_VOICES` and silences the rest, tie-broken by entity
  so the same voices survive frame to frame. Fixed in 6a11d0fd.
- [x] A1.4 (MAJOR) `voice.rs` - a one-shot whose asset never yields a sink (the
  handle fails to load, the decode fails) was never despawned. Added
  `AwaitingSink` and a two-second grace reaper. Fixed in 6a11d0fd.
- [x] A1.5 (MAJOR) `crates/nova_ship/src/sections/turret_section/stow.rs` -
  `TurretStow::doors_reported` could go stale on a fold interrupted before the
  lids moved, so the door cue fired on a re-deploy that never moved them. The
  flag is gone; `command_doors` compares the cue it is about to set against the
  one already there. Fixed in c07eaa1e with
  `a_fold_interrupted_before_the_lids_move_re_deploys_in_silence`.
- [x] A1.6 (MAJOR) `crates/nova_scenario/src/objects/salvage.rs:201` - a salvage
  pickup played on `AudioRoute::Interface`. A world event on the UI bus ignores
  distance and the World volume track. Now `AudioRoute::Hull`. Fixed in c07eaa1e.
- [x] A1.7 (MAJOR) `crates/nova_ship/src/sections/ammo.rs` -
  `tick_section_reload` reloaded inactive sections, so a destroyed or stowed
  mount kept cycling its reload and its reload cue. Query gained
  `Without<SectionInactiveMarker>`. Fixed in c07eaa1e.
- [x] A1.8 (MINOR) `voice.rs` - `voice.speed` was pushed to the sink unclamped;
  zero or negative stalls rodio. Clamped to `f32::MIN_POSITIVE`. Fixed in
  6a11d0fd.
- [x] A1.9 (MINOR) `crates/nova_ship/src/ship_audio/combat.rs` - four cues
  resolved their `AssetRef` BEFORE asking the throttle whether to play. A blast
  fans one impact per struck collider onto one cell key, so all but one were
  thrown away after paying for an `AssetServer::load`. The resolve moved after
  the throttle; the authored-or-silent test stays where it was. Fixed in
  6a11d0fd.
- [x] A1.10 (MINOR) the `AudioPlayer` guard test scanned `crates/` only and
  matched only the tuple form. It now scans five roots and catches
  `AudioPlayer::new`. Fixed in 6a11d0fd.
- [x] A1.11 (MINOR) `crates/nova_editor/src/cues.rs` - the placement cue kept
  its heard pose in a `Local`, which survives an editor re-entry. Moved to
  `PlacementPoseHeard`, reset alongside `PlacementPose` on
  `OnEnter(ExampleStates::Editor)`. Fixed in c07eaa1e.
- [x] A1.12 (MINOR) the editor scenario path wired no ship collapse sound.
  Fixed in c07eaa1e.
- [x] A1.13 (MINOR) `webmods/**` - 27 stale `impact_sound` fields left over
  after the impact table took that field, and 24 rocks borrowing the hull's
  destroy voice. Dropped and retargeted to `destroy_rock.wav`. Fixed in
  c07eaa1e; `content lint` clean.
- [x] A1.14 (MINOR) documentation drift: `scripts/gen-world-sfx.py` had the two
  PDC rates inverted in two places, `assets/sounds/README.md` said eleven
  `nova_*.wav` (ten) and pointed at `audio.rs` (now `audio/mod.rs`),
  `credits/CREDITS.md` overstated one script's dependencies, `web/src/wiki/settings.md`
  described one volume slider where there are four, the modding vocabulary was
  missing the `Impact` content item, `mod-files.md` said five content kinds, and
  `docs/architecture.md` still described `audio` as the menu SFX engine. All
  fixed in c07eaa1e.
- [x] A1.15 (MINOR) `web/src/docs-manifest.js` was left prettier-dirty by
  b94885d3, so `npm run format:check` failed on master. CI does not run the web
  checks, which is why it was not caught. Formatted in c07eaa1e; the whole web
  tree is clean again.
- [x] A1.16 (MINOR) `CHANGELOG.md` - one `[Unreleased]` entry was 207 characters
  against the 200 limit, and one intra-cycle revision of the bore sight was
  documented as its own entry. Trimmed and collapsed in c07eaa1e; longest entry
  is now 197.

Verified (not taken on trust):

- Probe A/B on the same box, 900 frames each, `59048768` against `6a11d0fd`:
  live sinks 600 -> 200, mean frame 26.50 -> 23.08 ms, max 248.99 -> 115.98,
  p95 113.54 -> 49.51, p99 194.17 -> 101.03, mean fps 37.74 -> 43.33, 1% low
  5.15 -> 9.90. `fps_within_baseline` PASS.
- `log_clean` still FAILS: 65 rodio underrun lines, down from 137. That is the
  steady-sink finding below, not a regression.
- `cargo fmt --all -- --check` and `cargo check` across nova_ship, nova_assets,
  nova_authoring, nova_menu, nova_editor, nova_scenario with
  `--all-targets --features debug`: clean.
- `npm run test` in `web/`: site, theme, ron and asset tests all pass.
- `content lint`: 0 errors, 0 warnings, 13 scenarios balance-audited, 1 acked.
- NOT verified locally: the workspace test suite and Clippy (CI only), and no
  listening pass was made - the sounds are proved present and correctly routed,
  not proved to sound right.

## Left open for the owner

Architectural, deliberately not changed:

- The ~200 steady exterior-loop sinks are silenced by the cap but never PAUSED,
  so rodio keeps mixing them and the underruns persist. Two ways out: pause a
  capped sink, or bound the population where voices are created.
- No global one-shot voice budget.
- `drive_sfx_voices` allocates a `HashMap` and a `Vec` per frame;
  `listener.affine().inverse()` is recomputed per voice; `loops.rs` resolves an
  `AssetRef` per source per frame.
- `&mut GlobalTransform` conflict in `PostUpdate` (audio runs before
  `TransformSystems::Propagate`).
- `machinery.rs` door cues carry no throttle key.
- `ThrottleKey` names ship weapons from inside `nova_gameplay`.
- 21 volume constants live in the wrong crate.
- No `ShipAudioSystems`, `MenuCueSystems` or `EditorCueSystems` set, against the
  repo's own naming convention.
- Ten authored sound fields are undocumented, and the wiki and dev book have no
  railgun chapter (the creator reference got one here).
