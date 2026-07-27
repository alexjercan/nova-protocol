# NOTES - NOVA OS CRT degauss + micro-effects polish (20260727-014148)

## What changed and why

A polish pass on the render-to-texture CRT that landed in 20260726-193233. All
of it is additive uniform-driven terms on the existing sampling shader
(`assets/shaders/nova_os_crt.wgsl`) plus their driver systems in
`crates/nova_gameplay/src/hud/nova_os.rs`. No pipeline, interaction, or
power-collapse mechanism change.

### Degauss pulse on app launch/exit/switch

- New `NovaOsDegauss` resource (`remaining: f32`), pulsed to
  `NOVA_OS_DEGAUSS_DURATION` (0.45s) and read back as a `remaining / DURATION`
  0..1 envelope.
- Fired from `sync_nova_os_app_ui` at the point PAST its diff-guard early return,
  so exactly the real launch/exit/switch mode changes kick it - the SAME
  transitions the input handlers already play the `NovaOsCoil` degauss-coil sound
  on. That is the audio pairing the task asked for (task 20260726-214639 is
  CLOSED; no new audio work): the visual wobble+flash lands with the coil thump
  because both hang off the same mode change.
- `animate_nova_os_crt` bleeds `remaining` down by `Time<Real>::delta` (real
  time, because the sim clock is frozen while the computer is open) and writes
  the envelope into a new `degauss` uniform, appended LAST after `brightness` in
  both the Rust `NovaOsCrtUniform` and the WGSL struct (trailing `f32`, no
  alignment hole - `shader-uniform-field-order-must-match-wgsl`).
- Shader: the envelope drives a fast decaying horizontal shear (`sin(y*18 +
  time*90)` scaled by `degauss^2`) applied to the sample UV before the barrel
  warp, plus a brief white lift added to rgb. Both multiply by the envelope, so
  at rest (envelope 0) the degauss is an EXACT no-op - readability preserved.

### Always-on analog micro-effects

Three cheap pure-time-driven terms, each behind a tiny strength constant so it
can be tuned or zeroed independently:

- Mains-hum bar: a soft green band drifting slowly down the tube
  (`HUM_BAR_*`), gaussian in Y with wrap-around.
- ~4.5s mains flicker: a gentle global brightness breathing (`FLICKER_*`), a
  multiply.
- Occasional fast retrace beam: a thin green line falling once per
  `RETRACE_PERIOD` (7s) over ~1/3s, gated by `step(beam_y, 1.0)` so it is dark
  the rest of the period (`RETRACE_*`).

All three read against `in.uv` (the fixed screen face) rather than the warped
content, so they sit on the glass and do not smear with the barrel bow. Hum and
retrace lift the green phosphor tint; the degauss flash lifts white.

Deliberately LEFT OUT of this pass (noted so the omission is explicit, not
silent): the rare vertical-hold jitter slip (reads as a bug at rest) and the
phosphor-strike-on-fresh-line (needs a freshly-printed-row signal the shader has
no cheap access to - it would need a per-line timestamp fed through the content
render, out of proportion to a polish pass).

## Verification

- `cargo test -p nova_gameplay nova_os`: 65 pass. New test
  `nova_os_app_mode_change_pulses_and_decays_the_degauss_uniform` drives a real
  app launch through `sync_nova_os_app_ui` then `animate_nova_os_crt` and asserts
  the `degauss` uniform kicks to near-full, decays partway at half-duration, and
  settles back to an exact 0 past the duration. Driven with `run_system_once` +
  a hand-advanced `Time<Real>` so the decay is deterministic, not wall-clock.
- `cargo check -p nova_gameplay` clean; `cargo fmt` clean.
- Native capture (real GPU, `screenshot_nova_os --features debug`) under
  `shots/nova-os-welcome.png` + `shots/nova-os-active.png`: the WGSL compiles on
  the real GPU (no pipeline error), and the terminal text stays crisp and fully
  readable (help output, ship sections, command list all legible) with the CRT
  curvature/vignette/glow intact. This is the `render-output-eyeball` check.

## Difficulties / bugs hit

- Adding the `degauss` param to `animate_nova_os_crt` and `sync_nova_os_app_ui`
  broke FIVE pre-existing unit tests that build partial apps and run those
  systems without the new resource (`ResMut<NovaOsDegauss>` panics when the
  resource is absent). Fixed by registering `NovaOsDegauss` in every test rig
  that runs the systems (`chin_controls_app`, the CRT uniform test, the app-ui
  test) alongside the production `init_resource` in the plugin. Lesson for next
  time: when a widely-run system gains a required resource param, grep every
  test rig that adds that system BEFORE running, not after the panic.
- The new test panicked in the AssetServer (`bevy_asset .../info.rs`) because it
  has an AssetServer (via AssetPlugin, needed for `Assets<NovaOsCrtMaterial>`)
  and the app-launch spawn loads a font - which panics unless the `Font`/`Image`
  asset types are registered. The sibling app-ui test dodges this by using
  MinimalPlugins with no AssetServer (so `nova_os_font(None)` returns a default
  handle). Fixed by `init_asset::<Font>()` + `init_asset::<Image>()`, mirroring
  `chin_controls_app`.

## Self-reflection

- The two-line fixes above cost a compile+test cycle each. Both were
  predictable from "this system now needs a resource / touches the asset server"
  - a quick grep of the test rigs at the moment I added the param would have
  batched them.
- Left the aesthetic constants conservative on purpose; final feel
  (degauss amplitude, hum/flicker/retrace strengths, curvature-vs-readability)
  is the owner's manual acceptance and easy to nudge since each is one named
  constant.
- The live WebGL2 eyeball remains owner MANUAL ACCEPTANCE (needs a real
  browser), inherited from the parent RTT task - the shader stayed
  derivative-free and fixed-tap, so no new WebGL2 risk was introduced.
