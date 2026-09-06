# Architecture: the agent bench

Owner design session, 2026-09-06. The engine that lets an external agent
play a scenario toward a goal, and the benchmark that scores the run.

## The one-line shape

```
cargo run --features debug bench play <scenario> --agent pi --goal "..."
```

```
                 referee protocol                 process channel
  +-----------+  (unix socket, JSONL)  +------------+  (stdin/stdout JSONL)  +----------------+
  |  agent    | <--------------------> |  nova_bench | <--------------------> |  nova-protocol |
  |  (pi, a   |  observe / act / finish|  (referee)  |  tick / input / aim ...|  --norender    |
  |  script)  |                        |             |  <- snapshot           |  --channel step|
  +-----------+                        +------+------+                        +----------------+
                                              |
                              audit.jsonl, score.json, TUI / log
```

Three processes, one referee. The agent never touches the game and the
game never sees the agent: `nova_bench` sits between them, owns the clock,
condenses what the agent sees, logs every line in both directions, and
scores the run from the game's own snapshots. An agent cannot fake a
result, because the referee reads the world, not the agent's report.

## What already exists (and is reused whole)

- `nova-protocol --norender --scenario <id> --channel step` (v0.12.0,
  task `20260820-174148`): the game as a stepped process. One JSON line
  per tick instruction, one snapshot line back. Deterministic under
  `NOVA_SEED`. The world only moves inside a step, so an LLM may think
  for a minute and the observation never goes stale.
- `nova_probe::capture_snapshot`: ships, sections, weapons, ordnance, UI,
  the live input set. The referee reads this and nothing else.
- `nova_probe_cli` (`probe` subcommand): the host-harness precedent. Its
  shape - a debug-only subcommand on the game binary, a run dir per run,
  `index.json` across runs, a `--baseline` comparison - is what the
  benchmark copies.
- `pi` 0.85 with `--mode rpc` (JSONL over stdio) and extensions that
  register tools. The tools run inside pi, which is why the referee has a
  socket and not a pipe.
- The poc drivers (`tasks/20260820-174148/poc/`): `channel.py` is the
  reference client and `agent_loop.py` the scripted policy the in-process
  baseline agent ports.

## Decisions

### Rust crate and a `bench` subcommand, not a Python script

`crates/nova_bench`, linked into the game binary under the `debug`
feature beside `probe`. Reasons:

- The bench spawns the GAME. As a subcommand it spawns `current_exe()`
  with `--norender --channel step`: no build step, no binary lookup, the
  same features guaranteed.
- Tooling in this repo is Rust (`probe`, `content`); `scripts/` is asset
  generation. A first-class tool in a second language is a second
  toolchain to keep green.
- A TUI (ratatui) and a CI-able benchmark both want a compiled tool.
- The crate depends on NO `nova_*` crate. It speaks JSON over pipes and
  sockets, so its tests link neither Bevy nor the game (the same rule
  `nova_autopilot` keeps). The one shared constant, ten meters per world
  unit, is restated with a test that pins it.

The one non-Rust file is the pi extension (`tools/nova_bench/pi/`), a
short TypeScript relay: three tools that forward one JSON line each to the
referee socket and return the reply. pi runs its tools in-process, so the
relay cannot live anywhere else.

### The referee protocol

JSONL request/response over a unix socket at `NOVA_BENCH_SOCKET` (under the
system temp dir, named by the bench's pid: a run dir path can exceed the
socket path limit). One request per connection. Three requests:

| Request | Reply | Meaning |
| --- | --- | --- |
| `{"observe": {}}` | `{"ok": <observation>}` | What the pilot sees now. Free: the clock does not move. |
| `{"act": {"gestures": [...], "ticks": N}}` | `{"ok": <observation>}` | Apply the gestures, run N ticks, observe. The ONLY way time passes. |
| `{"finish": {"status": "done" or "gave_up", "report": "..."}}` | `{"ok": {"over": true}}` | The agent declares it is finished. The referee scores from the world, not from this. |

Any malformed request gets `{"error": "..."}` and the run continues.

Gestures are the channel's lanes minus the tick, which the referee stamps:

| Gesture | Expands to |
| --- | --- |
| `{"press": "flight.main_drive"}` | one `input ... start` line; the referee remembers it as held |
| `{"release": "flight.main_drive"}` | one `input ... stop` line |
| `{"tap": "targeting.radar_hold"}` | start on one tick, stop on the next |
| `{"aim": "flight.rcs_aim", "delta": [x, y], "ticks": K}` | K aim lines, one per tick |
| `{"command": "..."}`, `{"text": "..."}`, `{"key": "..."}`, `{"pointer": {...}}` | passthrough, one line each |

The referee owns the tick counter, so an agent's vocabulary is relative
and a transcript is replayable.

### The observation is a pilot's view, in meters

The raw snapshot is a probe artifact: skin plates, fixtures, every
ordnance record. An LLM reading that is spending tokens on greebles. The
referee condenses it to what a pilot perceives:

```json
{
  "tick": 1230, "game_seconds": 20.5, "over": false,
  "outcome": null,
  "objectives": [{"id": "burn", "message": "Burn to the work mark."}],
  "comms": [{"speaker": "Halloran", "text": "..."}],
  "me": {"id": "cutter", "position_m": [0, 0, 0], "speed_mps": 12.4,
         "health": {"current": 800, "max": 800}, "weapons_hot": true,
         "combat_lock": "raider_1", "travel_lock": null,
         "sections": [{"id": "turret_port", "class": "Turret", "alive": true,
                       "weapon": {"kind": "pdc", "ammo": {"rounds": 40, "capacity": 40},
                                  "on_target": true, "firing": false}}],
         "hull_plates": {"total": 44, "damaged": 0, "lost": 0}},
  "contacts": [{"id": "raider_1", "allegiance": "Enemy", "distance_m": 2800,
                "bearing_deg": [-3.2, 0.5], "closing_mps": 14.0,
                "health": {"current": 600, "max": 600}, "defeated": false}],
  "beacons": [{"id": "work_mark", "label": "WORK MARK", "distance_m": 9200, "bearing_deg": [-28, 4]}],
  "ordnance": {"inbound": 0, "outbound": 3},
  "inputs": {"live": ["flight.main_drive", "..."], "held": ["flight.main_drive"]},
  "refused": [{"input": "section.turret_port", "state": "refused", "detail": "..."}]
}
```

The hull's skin plates are folded into `hull_plates`: a gunship carries
forty-odd, and listing them cost more tokens than the rest of the view.

Bearings are azimuth and elevation from the ship's nose, so "turn left
3 degrees" is a sentence the agent can act on. Every length is meters,
every speed meters per second, per the project rule.

The condensed view is the only one an agent gets; `--audit-raw` keeps the
full snapshot in the audit for a reader who wants it.

### The snapshot grows a `mission` block and a `beacons` list

The agent needs its goal and the referee needs the outcome; neither is
in the snapshot today. `nova_probe` adds:

- `mission.objectives` - `GameObjectives`, id and message.
- `mission.outcome` - `CurrentOutcome`: `null`, or `{kind, message}`.
- `mission.comms` - the `StoryFeed` lines, speaker, text and channel.
- `beacons` - every `BeaconMarker`: id, label, position.

Adding keys needs no schema bump (a reader ignores unknown keys). Two
snapshots of one world stay byte-identical: lists are value-ordered like
the rest of the file.

### Agents are pluggable; pi is the first

`--agent` picks who sits on the socket:

- `baseline` - in-process Rust policy, the `agent_loop.py` hunter ported.
  Proves the referee, the scoring and the report with zero tokens, and
  is what CI runs.
- `pi` - spawns `pi --mode rpc --no-session --no-builtin-tools
  --tools observe,act,finish -e tools/nova_bench/pi/index.ts --model <m>
  --thinking <level>`, sends one `prompt` (the manual, the scenario, the
  goal), and consumes the RPC event stream: assistant text, thinking,
  tool calls and usage land in the audit and the TUI. `--model` and
  `--thinking` pass straight through, so `gpt-5.6-luna:low` is a flag,
  not a code change.
- `cmd:<argv>` - any process that speaks the referee protocol, given
  `NOVA_BENCH_SOCKET`. A Python policy, a different LLM runner.

The in-process baseline and the socket server call the same `Referee`
struct. One core, two fronts.

### The run ends when the referee says so

Whichever comes first: the scenario declares an `Outcome` (Victory or
Defeat); the tick budget (`--ticks`, default 18000, five game minutes);
the turn budget (`--turns`, acts); the wall-clock deadline
(`--deadline`); the agent calls `finish`; the agent process exits. The
reason is recorded. After that, `act` is refused with `over: true`, the
game gets EOF (the channel's clean exit), and pi gets `abort` and EOF.

### Scoring: raw metrics, no composite

`score.json` per run, from the snapshot stream only:

| Metric | Source |
| --- | --- |
| `outcome` | `mission.outcome` at the end: `victory`, `defeat`, `none` |
| `objectives_completed`, `objectives_seen` | ids that appeared, then vanished, before any defeat |
| `ticks`, `game_seconds`, `wall_seconds` | the referee's clock |
| `turns`, `gestures` | `act` calls and the lines they expanded to |
| `damage_taken` | sum of health DECREASES on the player ship |
| `sections_lost` | player sections that stopped being alive |
| `ammo_spent` | sum of `rounds` DECREASES across the player's weapons (reloads are increases) |
| `kills` | hostile ships that turned `defeated` |
| `refusals` | channel error lines and refused inputs |
| `llm` | pi only: input/output tokens and cost from `message_end` usage |
| `ended_by` | outcome, ticks, turns, deadline, finish, agent_exit |

A composite number is policy; the report shows the table and a baseline
diff. The task asked for completion, time, damage and ammo, tracked
release over release, and this is those four plus the cost of getting
them.

### Audit: one JSONL, everything, in order

`audit.jsonl` in the run dir, one object per event, flushed per line:

- `run_start` / `run_end` (with the reason and the score)
- `channel_out` (every line sent to the game) / `channel_in` (every
  snapshot, condensed by default; `--audit-raw` keeps the full one)
- `agent_request` / `agent_reply` (the referee protocol, verbatim)
- `agent_text`, `agent_thinking`, `agent_tool` (pi only, from RPC events)
- `refusal`

`channel_out` alone replays the run: `bench replay <audit.jsonl>` feeds
the recorded lines to a fresh game and grades the end against the audit's
last view. Combat is not bit-identical across processes under one seed
(how many rounds land drifts), so the verdict is `match`, `close` (same
tick, outcome and ships down; positions and the health of standing ships
within 5 % or 10 m) or `mismatch`. With `--record <dir>` it is a
real-time movie of the agent's run.

### One event bus, two renderers

The referee emits `BenchEvent`s. The audit sink writes them; the
renderer draws them. `--ui log` prints one line per event to stderr;
`--ui tui` (ratatui) shows three panes: the observation, the agent's
transcript, the clock and score. The referee never knows which is
attached. The TUI is a later phase; the bus is designed in from the
start so it costs nothing to add.

### The benchmark is a sweep of plays

`bench run <scenario>[,<scenario>] --agent <a> --runs N --seed S` runs
each cell N times (seeds `S..S+N`), writes `bench-runs/<sha>/<scenario>/<n>/`
per play, then `index.json` and `index.html` above them, with a
`--baseline <dir>` diff. Same layout rules as `probe-runs`. Release over
release is `bench run --all --baseline bench-runs/<previous sha>`.

## Phases, each independently valuable

1. **Snapshot `mission` + `beacons`.** `nova_probe` only. Unblocks a
   goal an agent can read and an outcome a referee can score.
2. **The referee.** `crates/nova_bench`: game supervision, the clock,
   gesture expansion, the condensed observation, scoring, the audit, the
   log renderer, the `baseline` agent, the socket server and `cmd:`
   agents. `bench play` works end to end without an LLM.
3. **The pi agent.** The extension relay, the RPC driver, the manual
   prompt, usage into the score. `bench play tutorial --agent pi`.
4. **The TUI.** ratatui renderer on the event bus.
5. **The sweep and the report.** `bench run`, `index.json`, HTML,
   `--baseline`.
6. **Replay.** `bench replay <audit> [--record <dir>]`.

## Traps this design steps around

- **Stale observations.** Step mode is the fix; free mode is never
  offered to an agent.
- **The agent grading itself.** `finish` carries a report for the
  findings; the score never reads it.
- **Token spend on greebles.** The condensed view is the default and the
  raw one is opt-in.
- **The clock in the agent's hands.** The referee stamps ticks; an agent
  speaks relative gestures.
- **Profile leakage.** The child game gets an empty profile under the
  run dir, as `probe` does, and `NOVA_SEED` from `--seed`.
- **The SyncWorld queue leak under `--norender`** (channel task notes):
  linear in run length, so the tick budget is a hard cap, not a hint.
- **A wire name that renames.** The manual lists wire names from the
  LIVE set in the observation, not from a copy in the prompt.

## What landed (2026-09-06)

Phases 1, 2, 3 and 6, on branch `agent-bench`. The wire names, the view
and the numbers in the pilot manual (`crates/nova_bench/src/manual.md`)
were measured on the live game, not copied from this page: about 35 px of
`camera.camera_rotate` delta per degree, the hull following at about
18 deg/s, turret rounds landing inside about 2000 m. The bench fixture is
`crates/nova_bench/scenarios/hunt.content.ron`, loaded as a loose file;
the shipped scenarios use the current block hulls and load the same way.
The user-facing page is `docs/agent-bench.md`; proof is in `TASK.md`.

## Open

- Whether the raider AI's engage grace in a bench scenario should be
  authored per benchmark (an `acceptance`-style file per cell) or the
  shipped scenarios are the roster as they are. Start with the shipped
  ones plus the channel acceptance world.
- Whether `autopilot_goto` needs a referee-level macro (it wants a map
  target set through NOVA OS). Not in phase 2.
