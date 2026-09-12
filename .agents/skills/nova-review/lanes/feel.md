# Lane: feel

Judge what a player sees and does.

This lane wants a rendered run. Take the measurement slot only after the
performance lane releases it. Reason from the code when you cannot get a run,
and say the judgement is unrendered.

## Look for

- Layering: text or a widget over a panel that should cover it, two competing
  `GlobalZIndex` values, a chip that escapes its container.
- Navigation: how many actions one level of the hierarchy costs, an escape that
  jumps two rungs at once, a screen that refuses a keybind without saying why.
- Feedback: an action with no visible result, a cover that flashes, a stutter
  where a screen belongs, a state change a player cannot see.
- Legibility: contrast, a label truncated at the default width, a value with no
  unit, a scroll that moves too little per notch.
- Panels and inspectors: a setting shown because it exists rather than because
  it matters at this level of the model.

## Running

Probe brings up its own throwaway X server:

```bash
nix develop --command cargo run --features dev probe run <name>
```

Inspect captured frames when the range produces them. Otherwise use an
existing screenshot example, or a recorded bench play following
[Verify](../../verify/SKILL.md). State the player steps and expected visible
feedback before the run. A headless score cannot prove appearance, and a
rendered run with no inspected frames is not visual evidence.

Use only existing fixtures during review. When you start an X server by hand,
record its PID and stop it by that PID.
