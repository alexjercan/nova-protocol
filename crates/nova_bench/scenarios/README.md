# Agent bench sandboxes

These loose scenarios are development fixtures. They are not installed in the
scenario picker and do not ship as game content. Pass the file path to
`bench play`.

```sh
cargo run --features dev bench play crates/nova_bench/scenarios/slingshot.content.ron \
  --agent pi --model gpt-5.6-sol --thinking medium --seed 7 \
  --ticks 25000 --deadline 1000 --goal "Reach EXIT after a close gravity assist around Kestrel. Do not fire."
```

A sandbox has no Victory condition. The agent calls `finish`, and the final
`score.json` state, `audit.jsonl`, and optional recording are the evidence.
Keep goals observable. Name contacts, bodies, and beacons exactly as they
appear in the scenario.

## Fixtures

- `hunt.content.ron`: a short scored gunfight. Use it as the control.
- `range.content.ron`: a gunship, a planetoid, two rocks, a beacon, and an
  inert hostile hauler. Use it for basic navigation, orbit, parking, and PDC
  tests.
- `slingshot.content.ron`: Kestrel sits off the direct line to EXIT. A hostile
  pursuer wakes after 45 seconds. Use it for gravity assists, unsafe speed,
  collision, evasion, and route-cheese tests.
- `arsenal.content.ron`: a player warship with point defence, torpedoes, and
  two railguns. A carvable rock blocks an inert target barge, a carrier-sized
  ram target sits to port, and a raider wakes after 60 seconds. Use it for
  weapon choice, carving, NOVA OS commands, cheats, and destructive tests.

## Goal deck

Copy one quoted goal into `--goal`. These prompts state the evidence expected
from the run, but do not tell the agent how to drive.

### Slingshot

- "Reach EXIT after passing within 1,500 m of Kestrel's surface. Use its
  gravity to bend the route, spend no ammunition, take no damage, and leave
  the pursuer alive. Finish only within 500 m of EXIT."
- "Make the funniest safe orbit you can around Kestrel, then escape to EXIT
  before Late Fee catches you. Describe the maneuver using only values you
  observed."
- "Cheese this course. Reach EXIT by any game mechanic except firing a weapon.
  Try shortcuts and odd command-shell behavior, but do not arm cheats. Report
  every rule you bent and every input the game rejected."
- "Red-team gravity and collision. Try to obtain impossible speed, pass through
  Kestrel, confuse ORBIT, or survive a surface impact. Preserve enough ship to
  report what worked. A crash or NaN is a finding, not success."

### Arsenal

- "Be a weapon sommelier. Demonstrate point defence, one torpedo bay, and one
  railgun. Explain which target suited each weapon, then destroy the Safety
  Inspector with the least waste you can manage."
- "Carve a visible tunnel through Quarry with the railguns, then fly Borrowed
  Authority through the hole. Do not use cheats. Record this run; the footage,
  not the text report, judges the hole."
- "Use the NOVA OS command shell to inspect the ship and world before touching
  the controls. Discover commands through help. Change one harmless setting,
  prove the change, restore it, and park at DO NOT PRESS."
- "You are allowed to cheat. Arm cheats, make the player's ammunition
  unlimited, remove its speed cap, and cause as much controlled destruction as
  possible. The run must remain marked as cheated. Do not hide the mark."
- "Try to cheat without getting the run marked. Attack the command parser,
  command ordering, scenario reload, aliases, malformed values, and state
  transitions. Do not edit files or invoke external tools. Success means a
  cheat changes game state while `score.json` still says `cheated: false`; if
  none works, report the tested cases."
- "Win without weapons: destroy or neutralize Target Barge by ramming or by
  manipulating other bodies and ships. Spend zero ammunition. Stop if the game
  crashes or creates impossible health, velocity, or position values."
- "Break the referee, not the computer. Send adversarial but valid gesture
  sequences: shared inputs, held controls across NOVA OS, huge legal aim spans,
  repeated commands, and finish races. Seek state changes that the audit or
  score fails to record. Keep each experiment identifiable in your narration."

## Red-team rules

- A discovered defect is not automatically a completed goal. Preserve the
  smallest replayable sequence in the agent report.
- Never ask the model to alter repository files or use tools outside the four
  bench tools. The target is the game and referee protocol.
- Pin `--seed`. Keep the first failing `audit.jsonl`. Replay it before changing
  code.
- Grade claims from world state and footage. The agent's report is a lead, not
  proof.
- Turn a confirmed bug into an asserted `examples/systems/bug_*` range before
  fixing it.

## Recording the run

`--record <dir>` on a play saves every stepped tick and the bench draws the
run's own action rail over the frames before encoding `<dir>.mp4`. Remake the
movie of frames already on disk without replaying anything:

```sh
cargo run --features dev bench movie <run dir>/audit.jsonl --frames <dir>
```

The rail reads the audit, so a red-team run shows its own evidence: `cheat`
lines in amber, refusals in red, `nova` shell lines apart from helm `act`
lines. Grade the footage against those rows, not against the agent's report.
See [the bench guide](../../../docs/agent-bench.md#the-action-rail).
