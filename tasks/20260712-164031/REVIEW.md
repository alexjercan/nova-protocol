# Review: Turret free-aim while holding CTRL

- TASK: 20260712-164031
- BRANCH: feature/turret-free-aim-ctrl

## Round 1

- VERDICT: APPROVE

Out-of-context (fresh-context agent) review. Verified: `update_turret_target_input`
forces the `ray_tier` when `pressed(ControlLeft) || pressed(ControlRight)`, else the
untouched `component_tier.or(lock_tier).unwrap_or(ray_tier)` - matches the goal;
`pressed` (held state) is correct for a free-aim hold. Scope contained: the
same-named `ai.rs::update_turret_target_input` is a DISTINCT function (no keys
param, unaffected) and both torpedo feeds are untouched. No missing-resource panic
risk - production has Bevy InputPlugin; both `run_system_once` callsites in tests
init `ButtonInput<KeyCode>`. The new test sets a component lock AND holds CTRL and
asserts the ray point (falsifiable), with a release half asserting snap-back.
CTRL/ship-lock-cycle overlap is a feel issue only (the cycle writes lock state, the
feed reads it - different state, no hard bug), correctly flagged for playtest.

Checks: `cargo check --workspace --all-targets` clean; input::player tests 17/17
(incl. the free-aim test); `cargo fmt --check` clean.

One NIT (no change): with free_aim on and no lock, behavior equals normal unlocked
(already the ray) - a harmless no-op, no distinct unlocked+CTRL state needed.
