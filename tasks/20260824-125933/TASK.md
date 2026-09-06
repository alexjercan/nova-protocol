# An agent plays the game, and a benchmark scores it

- STATUS: OPEN
- PRIORITY: 50
- TAGS: v0.13.0,autopilot,tooling

Ideation, broad on purpose. Stays a future promise (owner, 2026-08-31).
Absorbed its duplicate `20260824-125929` on the same date.

Follow-up of `20260820-174148` (landed v0.12.0): an external agent (an
LLM or a script) plays a full scenario over the stdin/stdout channel in
--norender step mode - reads snapshots, writes inputs, completes
objectives with no human at the keyboard. On top of that, a benchmark
harness scores autonomous runs - completion, time, damage taken, ammo
spent - across the scenario set, tracked release over release.

## The engine round (2026-09-06)

Phases 1, 2, 3 and 6 of `ARCHITECTURE.md` landed on `agent-bench`; the TUI
(phase 4) and the sweep (phase 5) stay open.

- `crates/nova_bench` + the `bench` subcommand (`--features debug`):
  `bench play <scenario> --agent baseline|pi|cmd:<argv>` and
  `bench replay <audit.jsonl>`. No `nova_*` dependency; 35 unit tests.
- `nova_probe` snapshot: `mission` (objectives, outcome, comms) and
  `beacons`, with a test.
- `tools/nova_bench/pi/index.ts`: the three-tool relay for pi's RPC mode.
- `crates/nova_bench/scenarios/hunt.content.ron`: the bench fixture.
- Docs: `docs/agent-bench.md`, the env-var page, the changelog.

Proof, hunt fixture, seed 7:

| Agent | Outcome | Ticks | Turns | Ammo | Damage | Wall | Cost |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `baseline` | victory | 661 | 22 | 3630 | 0 | 5.4 s | 0 |
| `pi` gpt-5.6-luna, low | victory | 481 | 5 | 2940 | 0 | 33 s | $0.005 |
| `pi` gpt-5.6-luna, low | victory | 971 | 12 | 4128 | 66 | 66 s | $0.012 |
| `pi` gpt-5.6-luna, low, `--record` | victory | 1241 | 11 | 3416 | 413 | 114 s | $0.010 |

Both audits replay `close` (same tick and outcome, the raider down; the
leftover health of the wreck differs by up to 30 % between processes, so a
defeated ship's health is not compared).

Three pi plays of one seed are the three rows: the model's choices, not the
world, are the variance.

Findings from the pi plays:

- Its first `act` put two verbs in one gesture. The relay returned the
  parser's error as a tool error and the model fixed the call at once, so
  the error text is doing the teaching; keep it precise.
- The manual's turning numbers were measured live, not guessed: about 27 px
  of `camera_rotate` delta per degree once the hull has settled (the first
  reading of 35 was taken before it had), and the hull follows at about
  18 deg/s. Turret rounds land inside about 2000 m, while `on_target`
  reports from 2500 m.
- The nix `pi` wrapper appends `--extension` flags of its own, so
  `--no-extensions` does not keep them out; `--tools observe,act,finish`
  does, and their `extension_ui_request` events are ignored.
- Combat is not bit-deterministic across processes under one seed. A
  replay verdict is a grade, not an equality.
- `--record` gives the movie the owner asked about: the third play left
  1241 frames at 1280x720 and the bench stitched `rec-pi.mp4`, 20.7 s of
  real time for 114 s of wall clock. It needs the dev shell (the Vulkan
  loader) and a display (Xvfb :99 here); outside the shell the game finds
  no GPU and the play exits 1 with the reason in `game.log`.

## The range round (2026-09-06)

The owner asked for a scenario that tests goals the game itself does not
score: an orbit, a kill, a park, bounded by ticks and a deadline.
`crates/nova_bench/scenarios/range.content.ron` is that range: a 600 m
planetoid 7 km ahead, two rocks, a nav beacon and an unarmed hostile
derelict hauler. No objective, no victory; Defeat only when the gunship
breaks up. The score gains an `end` block (helm, well, speed, lock, the
range to every contact, beacon and body) and an `end` row in the table, so
a reader grades the goal from where things stood.

Measured with scripted `cmd:` probes before pi flew, all seed 7:

- Turret rounds fly straight through a rock, rendered or headless: with
  the small rock dead ahead at 600 m and all six PDCs on target, the
  stream sat at every range from the muzzle to 2000 m and `radius_m`
  never moved. `rounds.rs` says a collider with no health is a wall, so
  this is a game defect, not the bench's; there is no carving goal, and
  the manual says so.
- A `controller: None` ship with no allegiance takes no lock. Flagged
  `allegiance: Some(Enemy)` the derelict locks after the dwell, takes PDC
  fire, and DESPAWNS when it breaks up rather than reading `defeated`, so
  the scorer now counts a hostile that leaves the world as a kill.
- The radar dwell grows with range: about 60 ticks at 2500 m, not the
  20 the manual claimed. Holding 40 ticks and reading no lock looked like
  "unlockable" for two probes.
- A trigger pressed on the same tick as `combat_stance` is dropped with
  no refusal (the safety denies the press while the ship is still cold).
  The manual now says raise, then fire in a later act.
- While an autopilot holds the helm the hull does not follow the aim; the
  camera moves alone. The manual says `autopilot_off` first.
- The orbit helm takes a ring from a 325 m/s approach (Align, Burn, Hold
  at 2427 m over the surface, 114 m/s, steady for 6000 ticks) but not from
  572 m/s: pi's first orbit attempt dove to the surface in Burn and lost
  the bridge and the main drive. Stopping first is worse: inside the well
  the Stop helm shed 77 m/s over 3400 ticks while gravity pulled the ship
  onto the surface. The manual now gives the speed band and says not to
  stop.
- `PlayerAutopilotCompleted` lives one frame (the scenario tracker
  consumes it), so `me.autopilot.completed` is almost never in a view.
  The manual reads the park as `engaged` back to `null` at low speed.
- `camera_rotate` is about 27 px per degree once the hull settles; the
  earlier 35 was read before it had.
- The Stop helm turns the hull to face the drift before it burns: from
  80 m/s it is still within 300 ticks and 500 m, from 240 m/s pi's first
  destroy play tapped it at 2 km and passed the derelict at 157 m. The
  turrets kept the lock while the nose swung; range, not heading, is what
  the pilot has to manage. The manual's fight steps now say so.

pi on the range, gpt-5.6-luna low, seed 7, `--ticks 12000 --deadline 900`:

| Goal | Agent | Ended by | Ticks | Turns | Ammo | Damage | Wall | Cost | End row |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| orbit | `pi`, first manual | agent exit | 2041 | 10 | 0 | 7508 | 70 s | $0.012 | helm manual, well planetoid, body planetoid 7 m |
| orbit | `pi`, manual with the speed band | finish, done | 1646 | 7 | 0 | 0 | 41 s | $0.007 | helm Orbit planetoid (Hold), body planetoid 2123 m |
| park | `pi` | finish, done | 4581 | 21 | 0 | 0 | 115 s | $0.024 | helm manual, contact derelict 172 m |
| destroy | `cmd:` probe | finish, 1 kill | 3371 | 34 | 3000 | 0 | 25 s | 0 | no contact left |
| destroy | `pi`, first manual | finish, gave up | 10903 | 21 | 5136 | 2748 | 143 s | $0.029 | helm manual, contact derelict 5787 m |
| destroy | `pi`, manual with range control | finish, done, 1 kill | 2626 | 11 | 1096 | 0 | 65 s | $0.011 | no contact left |

Run directories were the session scratchpad; the audits are not kept.
