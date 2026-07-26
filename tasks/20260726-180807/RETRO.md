# Retro: NOVA OS computer HTML-fidelity pass

- TASK: 20260726-180807
- BRANCH: feature/nova-os-computer-fidelity
- REVIEW ROUNDS: 1 (APPROVE, out-of-context)

## What went well

- Built the capture harness (`screenshot_nova_os` example) as the FIRST step,
  then tuned every visual change against a real render. Every claim about
  readability/contrast/completion was screenshot-backed, not asserted.
- Traced the mechanism before touching numbers: the wash was the CRT overlay's
  centre `glow` + double-stacked fallback filming the text, not "colours too
  dim". One structural fix (kill glow, edge-only vignette, no double-stack) beat
  the prior task's six rounds of alpha nudging.
- Out-of-context review round 1 came back APPROVE with only two NITs, both cheap
  and addressed same-round; the reviewer independently re-ran all DoD proofs.

## What went wrong

- Nothing in THIS cycle, but the cycle only exists because
  `20260726-134738`/`20260726-142635` landed 7 combined rounds WITHOUT a render.
  Root cause there: no capture rig existed for the drawer, so "verify the render"
  silently degraded to "the widget tree matches" + blind number tweaks. The rig
  was treated as optional; it was the gate.
- Minor self-catch: the first example draft gated `bevy::prelude` behind the
  `debug` cfg, which would have broken a non-debug build of the example. Caught
  before it mattered by comparing against the sibling `screenshot_combat` import
  pattern.

## What to improve next time

- For any readability/CRT/contrast task on a surface with no existing capture
  path, building the capture path is step 1 of the task, not an optional extra -
  a widget-tree test cannot stand in for it. (Reinforces `render-output-eyeball`.)

## Action items

- [x] Bump `render-output-eyeball` in LESSONS.md (x5 -> x6) and sharpen it to
  say: if no capture rig exists for the surface, building it IS the first step.
- No follow-up code tasks: `map` / `ship viewer` command parity stays owned by
  the existing stretch tasks `20260724-102320` / `20260726-115339` by design.
