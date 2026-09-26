# Agent bench sandboxes

These loose scenarios are development fixtures. They are not installed in the
scenario picker and do not ship as game content. Pass the file path to
`bench play`.

```sh
cargo run --features dev bench play crates/nova_bench/scenarios/slingshot.content.ron \
  --agent pi --model gpt-5.6-sol --thinking medium --seed 7 \
  --ticks 25000 --deadline 1000 --goal "Reach EXIT after a close gravity assist around Kestrel. Do not fire."
```

`scripts/bench-plays.sh` is that command with the goal deck below already in
it, recording and all. Reach for it rather than retyping this one.

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
- `arsenal.content.ron`: a player hull authored inline in the fixture - a
  railgun spine, a Serpent bay to port, a Lance bay to starboard and two
  point-defence mounts. A carvable rock blocks an inert target barge, a hauler
  ram target sits to port, and a picket wakes after 60 seconds. Use it for
  weapon choice, carving, NOVA OS commands, cheats, and destructive tests.
- `docking.content.ron`: the `docking_approach` example as a fixture. A
  tender with a bow port and an inert Mooring Spar 120 m ahead, turned 18
  degrees out of square, with the RCS and DOCK granted. Use it for docking,
  RCS and close-quarters tests. `me.docking` is the readout; the score's end
  row says `docked spar` or the gap the run ended at.
- `docking_warship_tender.content.ron`: two catalog ships and a light. The
  player's Line Warship has its starboard collar 115 m from the port collar
  of an inert damaged frame tender, 12 degrees out of square. Use it for
  captures between real multi-section hulls. It has no goal in the deck.

## Goal deck

Every goal these fixtures are played with lives in `scripts/bench-plays.sh`,
one named play each, so a reworded goal cannot mean two different things in
two files. List them, then run one, several, or all:

```sh
nix develop -c scripts/bench-plays.sh --list
nix develop -c scripts/bench-plays.sh sommelier shell cheat
```

| Play | Fixture | What it is for |
| --- | --- | --- |
| `assist` | slingshot | a clean gravity assist under a no-fire, no-damage rule |
| `funny-orbit` | slingshot | free maneuvering, narrated from observed values only |
| `cheese` | slingshot | route and shell abuse, cheats barred |
| `gravity-redteam` | slingshot | impossible speed, tunnelling, ORBIT confusion, impact |
| `sommelier` | arsenal | weapon choice explained per target |
| `carve` | arsenal | a railgun tunnel through Quarry, graded on footage |
| `shell` | arsenal | NOVA OS discovery, one reversible setting change |
| `cheat` | arsenal | armed cheats, destruction, and the mark kept visible |
| `sneaky-cheat` | arsenal | a cheat the score fails to mark |
| `ram` | arsenal | a kill with zero ammunition |
| `referee` | arsenal | valid gestures the audit or score fails to record |
| `dock` | docking | a hand-flown RCS approach and a dock, no firing |

The script pins fixture, goal, agent, model, thinking level, seed and budgets.
It does not pin the run: the model is the other half and does not repeat
itself, so two plays of one goal differ in route, ammunition and cost.

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

The rail reads the audit, so a red-team run shows its own evidence: refusals
in red, `nova` shell lines apart from helm `act` lines, and - when the game
itself marks the run - an amber `CHEATS ARMED` row and a `CHEATED` header from
that tick on. Grade the footage against those rows, not against the report.
See [the bench guide](../../../docs/agent-bench.md#the-action-rail).
