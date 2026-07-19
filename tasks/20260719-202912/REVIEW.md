# Review: harness mute

- TASK: 20260719-202912
- BRANCH: feature/harness-mute
- ROUND: 1

## What I tried to break

- **Setting corruption** (the trap the design exists for): `factor()` is
  read by persistence (`settings_store.rs:49`) and the menu slider - a
  mute inside it would save 0.0 over the player's real volume from any
  harness run that persists. Verified the split: persistence and UI still
  call `factor()`, only the three output sites call `output_gain()`, and
  the App test pins it (muted run -> GlobalVolume Linear(0.0), factor()
  still 0.3).
- **Bypass paths**: repo-wide grep for `AudioPlayer|set_volume|AudioSink`
  hits exactly one file (nova_gameplay/src/audio.rs) - one-shots via the
  masked GlobalVolume push, both loops via output_gain. No other crate or
  example plays audio.
- **Overwrite race**: a startup GlobalVolume insert would lose to
  apply_master_volume on frame 1 (resource_changed fires on fresh
  resources); masking INSIDE the push is ordering-proof by construction.
- **Test determinism**: no test touches process env. The mute is a
  resource tests inject; the env logic is a pure fn with its own combo
  table; the app() helper pins HarnessMute(false) so even
  `BCS_AUTOPILOT=1 cargo test` cannot flake the volume tests.
- **Minimal rigs**: appliers take Option<Res<HarnessMute>> defaulting to
  unmuted - audio-only test rigs without the settings plugin keep their
  existing full-volume behavior.

## Findings

- R1.1 (NIT, accepted): the escape hatch is env-only (`NOVA_MUTE=0`) - no
  in-game toggle for a harness run. Correct for the use case; a scripted
  run wanting audio is an operator decision, not a setting.
- R1.2 (NIT, recorded): pause already silences loops via sink pause
  (`pause_loops_behind_overlay`), so the mute and pause paths overlap
  harmlessly (set_volume on a paused sink is inert until resume, then
  masked anyway).

## Verdict

APPROVE - land after the user's by-ear test.
