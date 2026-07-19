# Mute audio in harness runs (autopilot/shot/reel + NOVA_MUTE): output_gain seam, settings and persistence untouched

- STATUS: CLOSED
- PRIORITY: 63
- TAGS: v0.8.0,audio,testing,examples

## Goal

Harness runs are silent: headless/scripted examples (smoke suite, probe,
screenshot captures) currently open the REAL audio device - Xvfb hides the
window but not the speakers. Mute the OUTPUT, not the setting.

Seam analysis (why this exact shape):

- Every audible path already consults `MasterVolume`: one-shot SFX via the
  `apply_master_volume` -> `GlobalVolume` push (multiplied into each new
  sink), and the engine-hum/RCS loops via `master.factor()` multiplied
  into their sinks EVERY frame (audio.rs:926, audio.rs:1044 - these
  bypass `GlobalVolume` by design). So one mask at the MasterVolume seam
  silences everything; inserting a zero `GlobalVolume` at startup would
  NOT work (apply_master_volume overwrites it on frame 1).
- `factor()` cannot carry the mask: it is also read by persistence
  (`nova_menu/settings_store.rs:49` - a masked factor would WRITE 0.0
  into the player's saved settings from a harness run) and by the
  settings UI slider/label (nova_menu/lib.rs) - the SETTING is 1.0, only
  the OUTPUT is muted. Hence a separate `MasterVolume::output_gain()`:
  `if harness_muted() { 0.0 } else { self.factor() }`, used ONLY by the
  three output sites.

Mute policy (env, resolved per call - no OnceLock, so tests can flip it):

- `NOVA_MUTE` set and != "0" -> muted (explicit mute for any run).
- `NOVA_MUTE=0` -> NOT muted, even under a harness (the escape hatch).
- `NOVA_MUTE` unset -> muted iff any bcs harness env is active
  (`BCS_AUTOPILOT` / `BCS_SHOT` / `BCS_REEL`): scripted runs have nobody
  listening. This auto-covers the smoke suite and probe (both already set
  BCS_AUTOPILOT for their children) with ZERO changes to them, plus the
  documented manual `BCS_AUTOPILOT=1 cargo run --example ...` commands.

## Steps

- [x] settings.rs: `harness_muted_from(nova_mute: Option<&str>, harness_env_active: bool)`
      as a PURE fn (the repo's probe-env pattern - unit-testable without
      touching process env, no test races), a thin env-reading wrapper,
      and `MasterVolume::output_gain()`. Swap the three output sites:
      `apply_master_volume` (settings.rs:227) + the thruster-hum and RCS
      loop appliers (audio.rs). Persistence and menu UI keep `factor()`.
- [x] Tests: pure-fn combo table (explicit mute, explicit unmute beats
      harness, harness envs alone, nothing set); extend the existing
      GlobalVolume-push App test - pin NOVA_MUTE=0 at its start (makes it
      deterministic even under `BCS_AUTOPILOT=1 cargo test`), then flip
      NOVA_MUTE=1 mid-test, change MasterVolume to retrigger the apply,
      assert GlobalVolume goes Linear(0.0) while MasterVolume stays put;
      restore the env at the end (single test fn owns these vars - no
      parallel-test race).
- [x] Docs: settings.rs module doc (the two-path volume paragraph gains
      the mute sentence), development.md harness/examples section (one
      line: harness runs are silent, NOVA_MUTE=0 to hear), CHANGELOG
      Unreleased bullet.
- [x] Verify: fmt; cargo test -p nova_gameplay with CI's feature set
      (crate-solo-tests-miss-unified-features lesson); by-EAR test is the
      user's (run any example under BCS_AUTOPILOT and hear nothing, then
      NOVA_MUTE=0 and hear the game).

## Notes

- Trigger: user request 2026-07-19 - headless example runs audibly play
  the game; mute them if possible.
- Deliberately NOT disabling bevy's AudioPlugin under harness: without it
  the AudioSource asset type is unregistered, .ogg loads fail, and the
  error lines would trip probe's log_clean check. Volume-zero keeps the
  full pipeline honest (same systems run in harness and real play).
- No probe/smoke changes needed: both set BCS_AUTOPILOT already; the
  auto-mute rides on it. probe's checks are unaffected (no log delta).
- render_scale_shot (BCS_SHOT) and the reel (BCS_REEL) runs mute too -
  same rule, scripted captures have nobody listening.

## Close-out (2026-07-19, branch feature/harness-mute)

Shipped as designed with one improvement over the plan: the mute is a
`HarnessMute` RESOURCE resolved from env once at plugin build, not a
per-call env read - tests inject the resource directly (insert after the
plugin wins), so there is zero process-env mutation in tests and no
parallel-test races; the pure `harness_muted_from` fn carries the
precedence logic and its own combo test. `mute.map(|m| *m)` keeps the
loop appliers' Option pattern (minimal rigs without the settings plugin
default to unmuted, matching their existing full-volume behavior).

Evidence:
- `cargo test -p nova_gameplay settings::` - 8/8 PASS: the two new tests
  (env-precedence combos; muted run pushes Linear(0.0) onto GlobalVolume
  while MasterVolume::factor() stays 0.3) plus all six pre-existing
  settings tests unregressed.
- `cargo check --workspace` clean (prelude export consumed fine).
- Coverage proof: `grep -rln "AudioPlayer|set_volume|AudioSink"` over
  examples/, crates/, src/ returns exactly ONE file -
  nova_gameplay/src/audio.rs - so the two masked paths (GlobalVolume
  push, loop-sink writes) are the WHOLE audio output surface.
- The by-EAR test is the user's: run any example under BCS_AUTOPILOT and
  hear nothing; NOVA_MUTE=0 restores sound.

No probe/smoke/example changes were needed - the auto-mute rides the
BCS_AUTOPILOT env both already set for their children.
