# The agent bench

How an external agent plays a scenario toward a goal with no human at the
keyboard, and how the run is scored. The bench is the `bench` subcommand of
the game binary, built with `--features debug`, and lives in `crates/nova_bench`
with one TypeScript relay in `tools/nova_bench/pi/`.

```
                 referee protocol                 process channel
  +-----------+  (unix socket, JSONL)  +------------+  (stdin/stdout JSONL)  +----------------+
  |  agent    | <--------------------> | nova_bench | <--------------------> |  nova-protocol |
  |  (pi, a   |  observe / act / finish|  (referee) |  tick / input / aim ...|  --norender    |
  |  script)  |                        |            |  <- snapshot           |  --channel step|
  +-----------+                        +-----+------+                        +----------------+
                                             |
                                 audit.jsonl, score.json, log
```

Three processes, one referee. The agent never touches the game and the game
never sees the agent. `nova_bench` sits between them: it owns the clock,
condenses what the agent sees, logs every line in both directions, and scores
the run from the game's own snapshots. An agent cannot grade itself, because
the referee reads the world, not the agent's report.

The game side is the process channel (`nova_channel`, see
[Architecture](architecture.md)) in `--norender --channel step` mode: the
world moves only inside a step, so an agent may think for a minute and its
observation never goes stale.

`nova_bench` depends on no `nova_*` crate. It speaks JSON over pipes and a
socket, so its tests link neither Bevy nor the game.

## Running a play

```sh
cargo run --features debug bench play <scenario> --agent baseline
cargo run --features debug bench play <scenario> --agent pi --model gpt-5.6-luna --thinking low
cargo run --features debug bench play <scenario> --agent 'cmd:python3 my_agent.py'
```

`<scenario>` is an installed scenario id, or a path ending in `.ron` loaded
as a loose content file. The bench ships two fixtures under
`crates/nova_bench/scenarios/`:

- `hunt.content.ron`: one player gunship, one hostile raider parked 2600 m
  ahead, an objective and both outcomes. The scenario scores itself.
- `range.content.ron`: an open range with no objective and no victory. A
  600 m planetoid 7 km ahead, a small rock and a big rock, a nav beacon and
  an unarmed hostile derelict hauler. The goal comes from `--goal`, the run
  is bounded by `--ticks` and `--deadline`, and a reader grades the `end`
  block of the score: "orbit the planetoid", "park on the derelict",
  "destroy the derelict", "fly to the work mark and stop". The rocks are
  places to fly to, not targets: a turret round flies through a rock and
  takes nothing off it, so there is no carving goal.

| Flag | Default | Meaning |
| --- | --- | --- |
| `--goal` | complete the objectives, take little damage, waste no ammo | the goal text the agent is given |
| `--model`, `--thinking` | unset: pi's own configured model and thinking level | passed straight to `pi`; `pi` agent only |
| `--seed` | unset | `NOVA_SEED` for the game, so a play replays |
| `--ticks` | 18000 | the tick budget, five game minutes |
| `--turns` | 300 | the `act` budget |
| `--deadline` | 1800 | wall-clock seconds |
| `--out` | `bench-runs/<sha>/<scenario>/<agent>-<n>/` | the run directory |
| `--record` | unset | draw every tick offscreen into this directory and stitch `<dir>.mp4`; see [The movie](#the-movie) |
| `--ui` | `log` | `log` prints one line per event to stderr; `quiet` prints nothing |
| `--audit-raw` | off | keep the full snapshot in every `channel_in` event |

The run directory holds `audit.jsonl`, `score.json`, `game.log`, `agent.log`
(a `cmd:` agent's stdout and stderr) and `profile/`, an empty settings
profile the child game is pointed at so a play never reads or writes yours.
The command prints the run directory and the score table on stdout and exits
non-zero only when the game or the agent process failed, never on a Defeat.

## The referee protocol

JSONL request and reply over the unix socket named by `NOVA_BENCH_SOCKET`.
One request per connection, one line each way.

| Request | Reply | Meaning |
| --- | --- | --- |
| `{"observe": {}}` | `{"ok": <view>}` | The pilot's view now. Free: the clock does not move. |
| `{"act": {"gestures": [...], "ticks": N}}` | `{"ok": <view>}` | Apply the gestures, run N ticks (default 30), return the view. The only way time passes. |
| `{"finish": {"status": "done" or "gave_up", "report": "..."}}` | `{"ok": {"over": true}}` | The agent is finished. The report lands in the score; the metrics do not read it. |

A malformed request gets `{"error": "..."}` and the run continues. Every
view carries `over`, `ended_by` and `budget_left`; once `over` is true, `act`
moves nothing.

A gesture is one object with one verb. The referee stamps the tick, so an
agent's vocabulary is relative and a transcript is replayable.

| Gesture | Expands to |
| --- | --- |
| `{"press": "<wire>"}` | one `input ... start` line; the referee remembers it as held |
| `{"release": "<wire>"}` | one `input ... stop` line |
| `{"tap": "<wire>"}` | start on one tick, stop on the next |
| `{"aim": "<wire>", "delta": [x, y], "ticks": K}` | K aim lines, one per tick |
| `{"command": "..."}`, `{"text": "..."}`, `{"key": "..."}`, `{"pointer": {...}}` | passthrough, one line each |

Gestures in one act land on the same tick, in order. A held input stays held
across acts until released.

## The view

The raw snapshot is a probe artifact: every hull plate, every ordnance record.
The referee condenses it to what a pilot perceives, in meters, meters per
second and degrees:

- `tick`, `game_seconds`, `over`, `ended_by`, `budget_left`, `game_errors`.
- `objectives`, `outcome` (`null` until Victory or Defeat), `comms` (the last
  twelve radio lines).
- `me`: id, `position_m`, `speed_mps`, `velocity_bearing_deg`,
  `turn_rate_dps`, `health`, `weapons_hot`, `combat_lock`, `travel_lock`,
  `autopilot` (`engaged`: action, target and phase, or null; `completed`:
  the last action that finished), `gravity_well` (the dominant well's id),
  `sections` (the bridge, the drives, each mount with its `weapon`: kind,
  ammo, `on_target`, `firing`) and `hull_plates` as `{total, damaged, lost}`.
- `contacts`: every other ship with `allegiance` (`Enemy`, `Player`,
  `Neutral`), `distance_m`, `bearing_deg`, `closing_mps`, `health`,
  `defeated`, `weapons_hot`, `ai_target`.
- `beacons` with `distance_m` and `bearing_deg`.
- `bodies`: every asteroid and planet with `kind`, `radius_m`, `distance_m`
  to the centre, `surface_m` to the surface, `bearing_deg` and
  `invulnerable`. A shot rock is carved, so its radius shrinks; a rock
  carved away leaves the list.
- `ordnance` as `{inbound, outbound}` counts.
- `inputs.live` (the wire names the game accepts now) and `inputs.held`.
- `refused` (inputs the game did not take last act) and `commands` (answers
  to `command` gestures).

`bearing_deg` is `[azimuth, elevation]` from the nose: azimuth positive to
starboard, elevation positive up. The wire names come from the live set in
the view, never from a copy in the prompt, so a renamed input cannot go
stale.

The snapshot itself grew a `mission` block (objectives, outcome, comms), a
`beacons` list, a `bodies` list and the ship's `autopilot` and
`gravity_well` for this; see `nova_probe::capabilities::snapshot`.

## Agents

- `baseline`: an in-process Rust policy. It waits for a hostile, raises the
  weapons, holds the radar until a combat lock stands, closes to 2500 m and
  holds every mount's trigger while the lock stands. Zero tokens; it proves
  the referee, the score and the audit, and is what a CI check would run.
- `pi`: spawns `pi --mode rpc` with the built-in tools off and the relay
  extension on, sends one prompt (the pilot manual in
  `crates/nova_bench/src/manual.md`, the scenario, the goal, the first view)
  and consumes the RPC event stream: assistant text, thinking, tool calls and
  usage land in the audit. `NOVA_BENCH_PI` names the binary when `pi` is
  not on the path. If pi settles with the run still open it is nudged once;
  a second settle ends the run as `agent_exit`.
- `cmd:<argv>`: any process that speaks the referee protocol. It gets
  `NOVA_BENCH_SOCKET` in its environment; its stdout and stderr go to
  `agent.log`. The run ends when the process exits.

## The run ends when the referee says so

Whichever comes first: the scenario declares an outcome; the tick, turn or
wall-clock budget runs out; the agent calls `finish`; the agent process
exits. `ended_by` names it. After that the game gets EOF (the channel's clean
exit) and pi gets an abort.

## The score

`score.json`, from the snapshot stream only:

| Metric | Source |
| --- | --- |
| `outcome` | the mission outcome at the end: `victory`, `defeat`, `none` |
| `objectives_seen`, `objectives_completed` | ids that appeared, and ids that then vanished before any defeat |
| `ticks`, `game_seconds`, `wall_seconds` | the referee's clock |
| `turns`, `gestures` | `act` calls, and the wire lines they expanded to |
| `damage_taken` | the sum of health decreases on the player ship |
| `sections_lost` | player sections that stopped being alive |
| `ammo_spent` | the sum of `rounds` decreases across the player's mounts; a reload is an increase and does not count |
| `kills` | `Enemy` ships that turned `defeated` |
| `refusals` | channel error lines and refused inputs |
| `llm` | pi only: messages, input, output, cached tokens and cost from the usage events |
| `ended_by`, `agent_status`, `agent_report` | why it ended, and what the agent said |
| `end` | where things stood at the end, for a goal the scenario does not score: the autopilot engaged and completed, the dominant well, speed, the travel lock, and the range to every contact, beacon and body |

There is no composite number. A composite is policy; the table is the
record.

## The audit

`audit.jsonl`, one object per event with a wall-clock `t`, flushed per line:

- `run_start`, `run_end` (the reason and the score)
- `channel_out` (every line sent to the game), `channel_in` (every snapshot,
  condensed; `--audit-raw` keeps the full one beside it)
- `agent_request`, `agent_reply` (the referee protocol, verbatim)
- `agent_text`, `agent_thinking`, `agent_tool`, `agent_usage` (pi)
- `refusal`, `note`

## Replay

```sh
cargo run --features debug bench replay <run dir>/audit.jsonl [--record <dir>]
```

Replay feeds the audit's `channel_out` lines to a fresh game under the
recorded seed and compares where the world ended with the audit's last view.
Two plays of one seed are not byte-identical once rounds fly: how many land
drifts between processes. The verdict has three grades:

| Verdict | Meaning | Exit |
| --- | --- | --- |
| `replay_match` | byte for byte | 0 |
| `replay_close` | same tick, outcome and ships down; positions and the health of the ships still standing within 5 % or 10 m | 0 |
| `replay_mismatch` | anything else | 1 |

With `--record` the replay is the agent's run as a movie, stitched the same
way a play's is.

## The movie

`--record <dir>` on a play or a replay hands the directory to the game's own
`--record`: the offscreen assembly draws every stepped tick with the real
render stack and the full HUD, and saves it as `<dir>/frame_%06d.png`. When
the game has exited the bench runs

```sh
ffmpeg -y -framerate 60 -i <dir>/frame_%06d.png -pix_fmt yuv420p <dir>.mp4
```

One tick is one frame and one tick is 1/60 s, so the movie runs in real
time however long the agent thought between acts. The bench prints the
movie's path, frame count and length after the score table and notes them
in the audit. Without ffmpeg, or when ffmpeg refuses, the frames stay on
disk and the note carries the command line to run by hand; the run's exit
code does not change.

Recording needs a GPU and a display. Run the bench from the dev shell
(`nix develop`), which is where the Vulkan loader finds the driver, on an
X display: your own, or an Xvfb (`DISPLAY=:99`). A play without `--record`
needs neither.

## Measured on the hunt fixture, seed 7

| Agent | Outcome | Ticks | Turns | Ammo | Damage | Wall | Cost |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `baseline` | victory | 661 | 22 | 3630 | 0 | 5.4 s | 0 |
| `pi` gpt-5.6-luna, low thinking | victory | 481 | 5 | 2940 | 0 | 33 s | $0.005 |
| `pi` gpt-5.6-luna, low thinking | victory | 971 | 12 | 4128 | 66 | 66 s | $0.012 |
| `pi` gpt-5.6-luna, low thinking, `--record` | victory | 1241 | 11 | 3416 | 413 | 114 s | $0.010 |

Three pi plays of one seed, one row each; the third was recorded and stitched into a 20.7 s movie at 1280x720. The first play's opening `act` was malformed (two verbs in one gesture); the
relay returned the parse error as a tool error and the model corrected
itself on the next call. Both audits replay `close`.

## Measured on the range fixture, seed 7

The range has no objective, so the goal comes from `--goal` and the reader
grades it from the `end` row. All plays `--ticks 12000 --deadline 900`;
the pi rows are gpt-5.6-luna with low thinking.

| Goal | Agent | Ended by | Ticks | Turns | Ammo | Damage | Wall | Cost | End row |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| orbit the planetoid | `pi`, first manual | agent exit | 2041 | 10 | 0 | 7508 | 70 s | $0.012 | helm manual, well planetoid, body planetoid 7 m |
| orbit the planetoid | `pi`, manual with the speed band | finish, done | 1646 | 7 | 0 | 0 | 41 s | $0.007 | helm Orbit planetoid (Hold), body planetoid 2123 m |
| park on the derelict | `pi` | finish, done | 4581 | 21 | 0 | 0 | 115 s | $0.024 | helm manual, contact derelict 172 m |
| destroy the derelict | `cmd:` probe | finish, 1 kill | 3371 | 34 | 3000 | 0 | 25 s | 0 | no contact left |
| destroy the derelict | `pi`, first manual | finish, gave up | 10903 | 21 | 5136 | 2748 | 143 s | $0.029 | helm manual, contact derelict 5787 m |
| destroy the derelict | `pi`, manual with range control | finish, done, 1 kill | 2626 | 11 | 1096 | 0 | 65 s | $0.011 | no contact left |

The first orbit play approached at 572 m/s, tapped the orbit helm as the
well appeared and dove into the surface in Burn; pi stopped talking with
the bridge and the main drive gone. The manual now gives the speed band
(60 ticks of drive is 80 m/s, 240 is 320, the helm takes a ring from 320
but not from 550) and the same model held a ring at 2.1 km on the next
play. The first destroy play locked the derelict in 100 ticks, then burned
to 240 m/s, tapped Stop at 2 km, overshot at 157 m, kept the guns on it to
4578 hp while it receded, and burned to 1650 m/s trying to get back. The
scripted probe closes at 80 m/s, stops at 1.6 km and holds the triggers:
the hauler breaks up and leaves `contacts` after 56 game seconds. With
the fight steps rewritten around range control, pi closed at 160 m/s,
tapped Stop at 1.96 km, settled 1.1 km off and broke the hauler up in
900 ticks of fire.

## Not built yet

A `--ui tui` renderer on the same event bus, and `bench run`: a sweep of
plays over a scenario set with an `index.json`, an HTML report and a
`--baseline` diff release over release. The design record is
`tasks/20260824-125933/ARCHITECTURE.md`.
