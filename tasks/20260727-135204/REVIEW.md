# Review: NOVA OS curved CRT edge rim + green grain

- TASK: 20260727-135204
- BRANCH: feature/nova-os-crt-frame

## Round 1

- VERDICT: APPROVE
- REVIEWER: out-of-context

DoD proofs run by the reviewer: `nova_os_monitor_has_physical_casing_details`
PASS (1 passed, 689 filtered out); `cargo check -p nova_gameplay --all-targets`
clean. The reviewer read the WGSL rather than re-running the (slow) screenshot
example.

Load-bearing claim re-verified in-session: the shader actually compiles and the
NOVA OS renders. Proven directly by running the real app headless
(`BCS_AUTOPILOT=1 cargo run --example screenshot_nova_os --features debug`):
reached Playing, exited via `AppExit::Success` (only reachable after the
autopilot opens the NOVA OS, which instantiates + renders the CRT material),
zero panic/naga/validation errors. This is stronger than the static WGSL read.

Reviewer confirmed: all shader consts declared before use; `grain * GRAIN_TINT`
and the `rim_add` math are type-correct (scalar broadcasts over vector); the rim
uses the barrel-warped `warped` uv so it bows with the tube (matches
DECISION.md); the screen-border test would fail if the bright frame returned;
both phosphor-rim nodes still exist so the count==2 rim test holds; DECISION.md
is coherent and honest; no dead code.

No BLOCKER / MAJOR / MINOR / NIT findings.

Pending user checks (manual DoD, cleared at flow Finish):
- Owner confirms, against the PoC, that the screen edge reads as a curved 3D
  tube lip (not a flat rectangle) and the grain/noise now carries a green tint.
