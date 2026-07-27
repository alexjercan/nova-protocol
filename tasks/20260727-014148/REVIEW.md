# Review: NOVA OS CRT degauss + micro-effects polish

- TASK: 20260727-014148
- BRANCH: feature/nova-os-crt-degauss

## Round 1

- VERDICT: APPROVE
- REVIEWER: out-of-context

### Verification (not findings)

- Uniform field-order lockstep verified line-by-line. WGSL `NovaOsCrtMaterial`
  (assets/shaders/nova_os_crt.wgsl:11-40) and Rust `NovaOsCrtUniform`
  (crates/nova_gameplay/src/hud/nova_os.rs:588-623) match in order and type
  through the appended trailing `degauss: f32`. No alignment hole (trailing
  scalar after `brightness`). The uniform corruption risk the task flagged is
  not present.
- Degauss self-clears to an exact no-op at rest: shader multiplies both the
  wobble (nova_os_crt.wgsl:103-106) and the white flash (:175) by
  `material.degauss * material.degauss`; envelope reaches exactly 0 once
  `remaining` hits 0 (nova_os.rs envelope()/decay clamps to 0.0). Readability
  preserved at idle. Confirmed.
- Pulse fires ONCE per real mode change, not every frame: `degauss.pulse()`
  (nova_os.rs:1892) sits PAST the diff-guard early-return (:1885-1887), so a
  frame where `desired == current` returns before pulsing. The new test even
  re-runs `sync_nova_os_app_ui` at the mid step with the mode unchanged and
  asserts the envelope keeps decaying (no re-pulse), covering this directly.
- Decay uses `Time<Real>::delta` (nova_os.rs:2327), correct because the sim
  clock is frozen while the computer is open (matches the rest of
  `animate_nova_os_crt`).
- Micro-effect math checks out: retrace `step(beam_y, 1.0)` gates the beam to
  the first ~1/3s of the 7s period (beam_y = period_t*3, lit only while <= 1.0);
  hum bar wraps with `min(dhum, 1.0 - dhum)`; flicker is a bounded multiply
  (0.988..1.012). All read against unwarped `in.uv` ("on the glass") per NOTES.
  All amplitudes are small named constants, each zeroable.
- Test `nova_os_app_mode_change_pulses_and_decays_the_degauss_uniform` is
  meaningful: it would FAIL if the trigger were removed (peak > 0.9 assert), if
  decay were removed (mid < peak assert), or if it never settled (final == 0.0
  assert). It is deterministic (hand-advanced `Time<Real>` + `run_system_once`).
- No existing tests weakened or deleted. Rust diff is additions only; the only
  changes to existing test rigs are added `init_resource::<NovaOsDegauss>()`
  lines (required by the new resource param), matching the production
  `init_resource` in the plugin (nova_os.rs:1165).
- Test proof: `nix develop --command cargo test -p nova_gameplay nova_os` ->
  65 passed, 0 failed, 0 ignored (matches NOTES.md claim).
- NOTES.md claims match the code (resource, duration 0.45s, envelope formula,
  three micro-effects, deliberate omissions, five broken-then-fixed test rigs).

### DoD status

- App launch/exit plays a decaying degauss wobble+flash: test proof PASS;
  manual capture/feel is pending user.
- Hum bar + at least one more always-on micro-effect, centre text readable:
  code present (hum + flicker + retrace); manual native + WebGL2 eyeball pending
  user.
- Uniform field order in lockstep, CRT still renders: test proof PASS (CRT
  uniform + sampling tests green through the new field).
- Touched tests pass: PASS (65/65).

### Findings

- [ ] R1.1 (NIT) crates/nova_gameplay/src/hud/nova_os.rs:2325 - `animate_nova_os_crt`
  reads `degauss.remaining > 0.0` then decays; when `remaining` is already 0 it
  skips the subtract but still calls `degauss.envelope()`, which is fine. Purely
  cosmetic: the `if remaining > 0.0` guard is redundant given `.max(0.0)` would
  already clamp, and it forces a `ResMut` change-tick even at idle every frame
  (the `ResMut` is taken unconditionally regardless). No behavioral impact; could
  drop the guard for simplicity. Not blocking.
  - Response: Keeping the guard. `ResMut` marks the resource changed only on
    DerefMut (the `degauss.remaining = ...` write), not on acquisition, so the
    `if remaining > 0.0` guard DOES avoid a per-frame change-tick on
    `NovaOsDegauss` at idle - it skips the write entirely when the coil is at
    rest. The `envelope()` call is `&self`. So the guard is a (cheap) correctness
    nicety for any future `Changed<NovaOsDegauss>` reader, not redundant churn.
    Left as-is by implementer discretion (NIT).
