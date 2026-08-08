# NOVA OS CRT degauss + micro-effects polish

- STATUS: CLOSED
- PRIORITY: 40
- TAGS: v0.9.0, feature, ui, hud

## Story

Polish pass on the NOVA OS render-to-texture CRT (landed in 20260726-193233):
add the degauss wobble+flash on app launch/exit and cherry-pick the cheap
micro-effects the sampling pipeline now unlocks, each a uniform-driven term that
must not harm text readability.

The RTT pipeline, sampling shader, forwarded-pointer interaction, power collapse
and all uniforms already exist; this task only adds effect terms + their triggers.

## Steps

- [x] Degauss envelope + trigger. Add a `NovaOsDegauss` resource holding a
      decaying envelope (`remaining: f32`, reset to a `NOVA_OS_DEGAUSS_DURATION`
      constant ~0.45s). Fire it from `sync_nova_os_app_ui` at the point PAST the
      diff-guard early-return, so exactly the real launch/exit/switch mode
      changes pulse it - the same transitions that already play the `NovaOsCoil`
      degauss-coil sound (this is the audio pairing with 20260726-214639, which
      is CLOSED; no new audio work).
- [x] Degauss uniform + shader term. Append a `degauss` field LAST to both
      `NovaOsCrtUniform` (Rust) and the WGSL `NovaOsCrtMaterial` struct
      (field-order lockstep; default 0.0). `animate_nova_os_crt` decays
      `NovaOsDegauss.remaining` by `Time<Real>::delta` and writes the 0..1
      envelope into the uniform. Shader derives a brief horizontal wobble
      (time-oscillated UV displacement, amplitude scaled by the envelope) plus a
      short white flash added to rgb - both multiplied by the envelope so they
      self-clear to an exact no-op at rest (readability preserved).
- [x] Cherry-pick the cheap always-on micro-effects as pure time-driven shader
      terms, each behind a tiny strength constant so it can be tuned or zeroed:
      the slow mains-hum bright bar drifting down the tube, the ~4.5s global
      brightness flicker, and the occasional fast retrace beam. Amplitudes kept
      small enough that centre text stays crisp. Rare vertical-hold jitter and
      the phosphor-strike-on-fresh-line are left OUT of this pass (jitter reads
      as a bug at rest; phosphor-strike needs a fresh-row signal the shader has
      no cheap access to) - noted here so the omission is explicit, not silent.
- [x] Tune the degauss amplitude, hum-bar/flicker/retrace strengths, plus the
      power-collapse curve + barrel-warp + bloom by playtest against
      curvature-vs-readability. Conservative defaults land in code; the final
      feel is the owner's manual acceptance.

## Definition of Done

- App launch/exit (and app-switch) plays a brief degauss wobble+flash, decaying
  to nothing on its own. (test: firing a mode change through
  `sync_nova_os_app_ui` then `animate_nova_os_crt` sets the `degauss` uniform
  > 0 and it decays toward 0 across frames; manual: capture/feel)
- The hum bar + at least one more always-on micro-effect (flicker/retrace) are
  in, and centre text stays readable. (manual: native capture + WebGL2 eyeball)
- Rust/WGSL uniform field order stays in lockstep - the CRT still renders (no
  panic) with the appended `degauss` field. (test: the CRT uniform test stays
  green through the new field.)
- Touched tests pass. (cmd: `nix develop --command cargo test -p nova_gameplay nova_os`)

## Notes

- Builds directly on 20260726-193233 (RTT pipeline) - the shader + uniforms exist.
- Micro-effect inventory + audio pairing detailed in that task's Notes/NOTES.md.
