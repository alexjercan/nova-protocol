# Retro: centralize gameplay HUD palette into nova_ui

- TASK: 20260714-214118
- BRANCH: ui/hud-palette
- REVIEW ROUNDS: 1 (APPROVE)

## What went well

- Checkpointing the scope with the user before touching combat colours was the
  right call. The HUD is semantic, already coherent, and needs visual QA to change
  safely - so "centralize at exact values, defer the recolor" gave a real win
  (one palette source) with zero risk, instead of a risky recolor I couldn't verify.
- Grepping the exact const values first surfaced which colours were TRULY shared
  (THREAT 3x, BACKDROP 2x) vs per-widget tuned variants, so I merged only the
  identical ones - keeping "no hue change" honest.
- A value-pin test in nova_ui converts "trust me, the values match" into a
  regression guard, and made the verdict verifiable without rendering anything.

## What went wrong

- Nothing in the execution. The only friction was upstream: the frame-starved
  capture environment is what forced the scope decision in the first place.

## What to improve next time

- When a task's real risk is VISUAL and the environment can't render, say so early
  and split the safe (verifiable) part from the QA-gated part, rather than pushing
  a change you can't check. That framing made a clean ask for the user.

## Action items

- [x] Landed; nova_ui pin test + workspace check green.
- [x] Follow-up 20260714-225524 filed (web-palette alignment, QA-gated).
- [x] Umbrella 20260714-212139 closed (all three children landed).
