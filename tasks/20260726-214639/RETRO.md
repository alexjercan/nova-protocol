# Retro - NOVA OS sound (20260726-214639)

Landed: the in-game NOVA OS terminal's full voice (10 WAV cues + ambient bed).
Review APPROVEd round 1 (MINOR/NIT only); all findings addressed before land.

## What went well

- Delegating the WAV generator to a background subagent in parallel with the Rust
  wiring was the right split: it is a self-contained, independently verifiable
  artifact (deterministic, byte-diffable), and the two streams never touched the
  same files.
- Reusing `objective_feedback::sfx_app`'s SoundBank + PlaySfx-capture pattern
  (`reuse-known-good-stack`) made "which cue fired" assertions mechanical and
  needed no audio device.
- Returning a semantic `TerminalSubmitOutcome` from `submit` (rather than having
  the pure model play sounds) kept the model audio-agnostic and made the
  cue-mapping trivially correct and reviewable.
- The bed's freeze exemption fell out for free: `pause_loops` queries only the
  thruster/RCS markers, so a distinct `NovaOsBedSfx` marker is exempt by
  construction - no guard to get wrong.

## What went wrong

- A new non-`Option` `Res<NovaOsMonitorSettings>` on the `on_nova_os_app_close`
  OBSERVER panicked one bare-app test rig that didn't init the resource. I had
  correctly made `setup_drawer`'s param `Option` for exactly this reason on the
  previous task, but didn't apply the same instinct to the new observer up front.
  Fixed by making it `Option`. Lesson reinforced: any Res added to an
  observer/system that a lean rig triggers should be `Option` unless every rig is
  known to init it.
- Three MINORs from review were all real and worth fixing (mute-path consistency,
  the third exit route, and a missing pure test for the bed gain) - a sign the
  first pass under-tested the bed's volume path (I leaned on "by construction"
  where a small pure helper + test was cheap).

## What to improve next time

- When adding a resource param to ANY observer/system, immediately grep its
  trigger/rig sites and decide `Option` vs init-in-rig in the same edit, instead
  of discovering the panic in the regression run. (Same class as last task's
  `setup_drawer` `Option` call - now a repeated instinct to internalize.)
- For a self-set-every-frame loop SINK, copy the sibling's mute path
  (`output_gain(mute)`) from the start - the thruster/RCS loops already encode
  the `HarnessMute` requirement, and `settings.rs` documents it explicitly. I
  reached for `factor()` and review caught the muted-run hum.

## Follow-ups / notes

- MINOR-2's third exit route (`NovaOsAppInputOutcome::Exit`) is now consistent
  but remains latent - no shipped app returns `Exit`. When the first app with its
  own quit key lands, add a capture-backed test for its coil.
- Manual acceptance owed: an in-browser (trunk) listen with SND on, and an
  in-game A/B against the PoC. Headless can't listen; the `wav` decoder already
  ships on web and every cue rides the existing UI-SFX path.
</content>
