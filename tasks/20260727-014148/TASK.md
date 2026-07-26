# NOVA OS CRT degauss + micro-effects polish

- STATUS: OPEN
- PRIORITY: 40
- TAGS: v0.9.0,feature,ui,hud

## Story

Polish pass on the NOVA OS render-to-texture CRT (landed in 20260726-193233):
add the degauss wobble+flash on app launch/exit and cherry-pick the cheap
micro-effects the sampling pipeline now unlocks, each a uniform-driven term that
must not harm text readability.

The RTT pipeline, sampling shader, forwarded-pointer interaction, power collapse
and all uniforms already exist; this task only adds effect terms + their triggers.

## Steps

- [ ] Degauss pulse on app launch/exit: a brief wobble + flash uniform driven
      from the `sync_nova_os_app_ui` mode-change hook (openApp/closeApp).
- [ ] Cherry-pick from the PoC micro-effect inventory where it reads well: slow
      mains-hum bar drifting down the tube, occasional fast retrace beam, the
      ~4.5s brightness flicker, rare vertical-hold jitter slip, phosphor-strike
      flash on freshly printed lines. Each a uniform term; none may harm text
      readability. Pair the audible ones with 20260726-214639.
- [ ] Tune the power-collapse curve + barrel-warp amount + bloom strength by
      playtest (curvature-vs-readability).

## Definition of Done

- App launch/exit plays a brief degauss wobble+flash. (manual: capture/feel)
- At least the hum bar + one more micro-effect are in, readability preserved.
  (manual: native capture + WebGL2 eyeball)
- Touched tests pass. (cmd: `nix develop --command cargo test -p nova_gameplay drawer`)

## Notes

- Builds directly on 20260726-193233 (RTT pipeline) - the shader + uniforms exist.
- Micro-effect inventory + audio pairing detailed in that task's Notes/NOTES.md.
