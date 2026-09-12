# Lane: red team

Do not review the change. Try to break the game with it.

Take the measurement slot only after the performance lane releases it. Follow
[Verify](../../verify/SKILL.md) for a bounded play on an existing fixture.
State the steps and predicted observations first. Mark an open-ended attempt
as exploration; reduce a discovery to a short reproduction before calling it a
confirmed defect. Propose any missing fixture to the owning session.

## Method

Take the states the change touches and drive them to their limits. For each
attempt, state the sequence, what should happen, and what does.

- Empty and full: a scenario with no ships, a ship with no sections, a list with
  one entry, a fleet at the entity cap, a name of zero length.
- Timing: a key pressed while a loading cover is up, a save during a reload, an
  exit mid-load, two inputs in one frame, a double click that lands on an entity
  despawned since the first.
- Reentry: enter, leave, re-enter the editor; open a panel twice; reload the
  same content twice; undo past the start; select an entity, then delete it.
- Content: a mod that names a prototype that does not exist, a scenario pointing
  at a removed section, a file that fails to load and never comes back, a
  circular `NextScenario`.
- Focus and input: a keybind two screens claim, an escape rung that skips a
  level, a pointer press that reaches a hidden or transparent widget, a drag
  that starts on one entity and ends on another.

## Report

A repro is a finding only when you can state the exact sequence and show the
failure evidence. A crash, a soft-lock, a lost input, and a cover that never
comes down are all findings. Preserve the first failing audit and replay it.
A lost fight or an agent that missed a step is not by itself a game defect.
Name the attempts that held and the steps never reached.
