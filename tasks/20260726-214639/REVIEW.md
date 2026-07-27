# REVIEW - NOVA OS sound: terminal SFX + ambient CRT bed (20260726-214639)

Reviewer: out-of-context code review of `feature/nova-os-sound`
(dd833d3e on top of master). Claims re-derived from the code, not the
commit/NOTES.

## Verdict summary

The feature is well built and the three DoD tests pass. Cue wiring is faithful
to the task's cue->event spec, the generator is genuinely deterministic
(byte-identical re-run verified), the bed is exempt from the freeze loop-pause
by construction (verified: `pause_loops`/`resume_loops` query only
`ThrusterLoopSfx`/`RcsLoopSfx`), and the SoundBank/settings params are correctly
`Option`-guarded or backed by an init on every rig that runs them. No MAJOR
correctness or DoD gap found. Findings below are MINOR/NIT.

## Positive confirmations (non-trivial claims verified)

- **Determinism**: ran `python3 scripts/gen-nova-os-sfx.py` a second time into
  the tree; `git status --porcelain assets/sounds/` is empty -> the 10 WAVs are
  byte-identical. The "CI can diff a regeneration" claim holds. Single shared
  `random.Random(SEED)` drawn in a fixed `build_cues` order backs this.
- **Bed loop is seamless**: `render_bed` uses pure phase functions of the sample
  index at `BED_DURATION = 2.0 s` with `BED_WHINE_FREQ 7860` (15720 whole
  cycles) and `BED_MAINS_FREQ 50` (100 whole cycles) -> integer cycles, no fade,
  and the one-pole lowpass is primed over a full loop so its state wraps. Truly
  click-free by construction.
- **Freeze exemption**: `audio::pause_loops`/`resume_loops`
  (audio.rs:1122-1144) query `With<ThrusterLoopSfx>` and `With<RcsLoopSfx>` only.
  `NovaOsBedSfx` is a distinct marker, so the `OnEnter(Drawer)` pause never
  touches the bed. Confirmed.
- **Throttle is load-bearing**: bevy `KeyboardInput` has a `repeat: bool` field,
  and `handle_terminal_keyboard` filters only on `event.state == Pressed`, NOT on
  `event.repeat`. So OS auto-repeat DOES reach the typing branch; the
  `NOVA_OS_KEY_MIN_INTERVAL` + `Local<Option<f32>>` throttle is necessary and the
  first-keystroke-fires guard (`Option`, not `0.0`) is correct.
- **Regression guard**: `terminal_command_app` runs the real
  `handle_terminal_keyboard` with NO `SoundBank` inserted -> proves the
  `Option<Res<SoundBank>>` guard no-ops cleanly, and `init_terminal_input_resources`
  now inits `NovaOsMonitorSettings` so the shared rig is not broken by the new
  non-`Option` `Res<NovaOsMonitorSettings>` param.
- **No other observer/system regression**: the only `Option`-fixed observer is
  `on_nova_os_app_close`. The other new non-`Option` `Res<NovaOsMonitorSettings>`
  users (`close_drawer_from_menu_keys`, `play_nova_os_power_down`,
  `apply_nova_os_bed_volume`, `handle_terminal_keyboard`) run only under the full
  `NovaDrawerPlugin`, which inits the resource (drawer.rs:1773), or in the sound
  test rig which inits it. No lean rig runs any of them standalone. OK.
- **File coverage**: `UI_SFX_FILES` has 15 entries, `every_ui_sfx_key_has_a_file`
  lists all 15 variants, and `git ls-files` shows all 10 `nova_*.wav` tracked.
- **DoD tests pass**: `cargo test -p nova_gameplay --lib nova_os_` -> 26 passed,
  incl. `nova_os_sound_cues_fire_on_terminal_events`,
  `nova_os_ambient_bed_tracks_drawer_state`, `nova_os_snd_off_silences_cues`.

## MINOR

### MINOR-1 - Ambient bed ignores `HarnessMute` (hums in muted smoke/probe runs)
`apply_nova_os_bed_volume` (drawer.rs:2004-2013) scales the bed by
`MasterVolume::factor()`, which is the raw player setting. Every other
self-set-every-frame loop sink uses `MasterVolume::output_gain(mute)`:
`apply_thruster_loop_volume` (audio.rs:983-995) and `apply_rcs_loop_volume`
(audio.rs:1103-1114) both take `Option<Res<HarnessMute>>` and call
`output_gain`, and `settings.rs:59-74` is explicit that these self-writing loop
sinks are exactly the sites that must call `output_gain`, because they bypass
the `GlobalVolume` path that masks freshly-spawned one-shots. The bed is the
same kind of sink but reads `factor()`, so under `HarnessMute` (a probe/smoke
run with `NOVA_MUTE`, or any harness env) the one-shots and the two other loops
go silent while the NOVA OS bed keeps humming if the drawer is opened.
Suggested change: give `apply_nova_os_bed_volume` `mute: Option<Res<HarnessMute>>`
and use `master.output_gain(mute)` instead of `master.factor()`, matching the
two sibling loop-volume systems it was modeled on.

### MINOR-2 - App self-exit route (`NovaOsAppInputOutcome::Exit`) drops the coil
The task's cue spec is "app launch AND app exit -> coil", and `exit_app`'s doc
(drawer.rs:1342-1345) names three exit routes: Escape/close-control AND "an
app's own `NovaOsAppInputOutcome::Exit`". Two routes fire the coil
(`close_drawer_from_menu_keys` drawer.rs:2040-2050, `on_nova_os_app_close`
drawer.rs:2299-2311), but the third does not: `handle_nova_os_app_keyboard`
calls `terminal.exit_app()` at drawer.rs:2286 with no cue. So an app that
returns `Exit` from `handle_key` returns to the prompt silently. Currently
LATENT, not audible in the shipped game: only the two test apps (drawer.rs:7122,
7145) return `Exit`; the shipped map/ship apps use the default
`handle_key` -> `Continue` (drawer.rs:629-632). Still an inconsistency that will
bite the first real app with its own quit key.
Suggested change: in `handle_nova_os_app_keyboard`, when `exit` fires and
`terminal.exit_app()` returns true, play `NovaOsCoil` via `play_nova_os_cue`
(the system would need `Commands` + `Option<Res<SoundBank<UiSfx>>>` +
`Res<NovaOsMonitorSettings>`), mirroring the other two routes. Add a test app
exit assertion to `nova_os_sound_cues_fire_on_terminal_events`.

### MINOR-3 - `apply_nova_os_bed_volume` has no test coverage
The DoD lists "SND off = fully silent NOVA OS" and "bed volume follows master +
SND live". `nova_os_snd_off_silences_cues` only asserts the one-SHOT capture is
empty; it never exercises `apply_nova_os_bed_volume`, and that system is not even
added to `nova_os_sound_app`'s schedule (only `play_nova_os_power_down` is). So
the bed-silencing half of the SND-off DoD and the live-master-follow claim are
unverified. An `AudioSink` cannot be built headless, but the volume decision is
pure branching on `settings.sound_enabled`/`master` and could be factored into a
tiny pure `bed_target_volume(enabled, master)` helper with a unit test (the same
split-for-testability pattern `HumLevels`/`ThrusterHumVolume` already use).

### MINOR-4 - Bed-survives-freeze is asserted "by construction", not exercised
`nova_os_ambient_bed_tracks_drawer_state` transitions to `PauseStates::Unpaused`
and checks the bed despawns; it never runs `audio::pause_loops` to prove the bed
keeps playing through a freeze. The exemption is real (verified above by reading
the queries), but the DoD line "survives the virtual-time freeze without stutter"
is not what the test checks. A stronger test would spawn a bed entity, run
`pause_loops`, and assert `NovaOsBedSfx` is still present / not paused (needs a
sink, so this may be construction-only by necessity - acceptable, but the test
comment overclaims).

## NIT

### NIT-1 - Empty-submit cue deviates from the PoC (intentionally, undocumented)
The PoC fires `Sound.enter()` on EVERY submit including an empty line
(poc html form submit handler). In-game, `TerminalSubmitOutcome::Empty`
suppresses the thunk (drawer.rs:2117). This is arguably better (a bare Enter
staying silent), and NOTES explains the thunk+outcome layering, but the
empty-line divergence from the PoC is not called out. Fine to keep; a one-line
comment at the `!= Empty` guard would record it.

### NIT-2 - Stale doc comment in `handle_terminal_keyboard`
drawer.rs:2095-2097 says "Cue helper closured over the bank/settings so each
branch is one line" but there is no closure - `play_nova_os_cue` is a free
function called inline in each branch. Update or drop the comment.

### NIT-3 - Tab/Escape no longer play the PoC key click
In the PoC, Tab and Escape also play `Sound.key()` (poc keydown handler). The
in-game mapping follows the TASK spec (Tab->tick-when-advancing, Escape->silent
at prompt), which is a deliberate refinement, not a bug. Noting only for the
record - no change requested.

## DoD checklist

- [x] Typing/submit/error/open/close each produce a cue + ambient bed while open
      - `nova_os_sound_cues_fire_on_terminal_events` asserts each, would fail if
      the mapping broke.
- [x] Bed starts/stops with the drawer, survives the freeze - spawn/despawn
      tested; freeze-survival by construction (MINOR-4).
- [x] SND off = silent - one-shots tested; bed-silence path untested (MINOR-3),
      and the bed still ignores HarnessMute (MINOR-1).

No MAJOR correctness or DoD gap. All findings are quality/consistency/coverage
improvements that do not block.

- VERDICT: APPROVE

## Round 1 responses (author, addressed before land)

- MINOR-1 FIXED: `apply_nova_os_bed_volume` now takes `mute: Option<Res<HarnessMute>>`
  and uses `master.output_gain(mute)`, matching `apply_thruster_loop_volume` /
  `apply_rcs_loop_volume`. A HarnessMute'd run now silences the bed too.
- MINOR-2 FIXED: `handle_nova_os_app_keyboard` now plays `NovaOsCoil` when its
  `NovaOsAppInputOutcome::Exit` route actually exits, so all three app-exit
  routes are consistent. (Latent path; its rig has no SoundBank so the existing
  app tests exercise the code path with the cue no-op'd.)
- MINOR-3 FIXED: the bed volume decision is factored into the pure
  `nova_os_bed_gain(sound_enabled, master)` helper, unit-tested by
  `nova_os_bed_gain_respects_snd_and_master` (SND off -> 0, master/mute 0 -> 0,
  half master scales).
- MINOR-4 ADDRESSED: reworded the `nova_os_ambient_bed_tracks_drawer_state`
  comment so it no longer implies it exercises the freeze pause; the exemption is
  documented as structural (a sink assertion needs an audio device).
- NIT-1 FIXED: added a comment at the `!= Empty` guard recording the deliberate
  empty-submit divergence from the PoC.
- NIT-2 FIXED: dropped the stale "closured" comment in `handle_terminal_keyboard`.
- NIT-3: no change (deliberate spec refinement, as noted).
