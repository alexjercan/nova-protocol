# An agent plays the game, and a benchmark scores it

- STATUS: CLOSED
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

## The env-contract gate (2026-09-07)

`tests/env_contract.rs` went red on master: the bench's two variables were
written up in the docs but never declared as constants, and the roster scan
rejects a bare `NOVA_*` literal in `crates/`. They are now
`nova_bench::prelude::{SOCKET_ENV, PI_ENV}`, on the roster with their own
assertion, and the root package takes the dev-dependency edge that lets the
test name them whatever features a run enables. The env-var page's
"shell-only" bullet named `NOVA_BENCH_*` as read by nothing in Rust; it now
names only `benchmark/`'s three coding-benchmark variables.

`cargo test -p nova-protocol --test env_contract`: 9 passed.

## The tutorial round (2026-09-07)

The first agent victory on a SHIPPED scenario, not a bench fixture. pi,
gpt-5.6-sol, thinking medium, `tutorial`, seed 7, `--ticks 50000
--deadline 2000 --record`. The owner's goal text carried three human
hints: sit through the cutscene, do not GOTO the beacon off orbit at the
wrong moment, and do not stand still in front of the drones.

`bench-runs/` is not tracked, so the proof lives here: `tutorial-score.json`
is the score, `tutorial-play.md` is the whole play - narration, thinking
headlines, gestures, sampled ticks and comms, in order.

| Metric | Value |
| --- | --- |
| Outcome | victory, ended by outcome |
| Objectives | 10 of 10 seen, 10 completed |
| Ticks | 24041 (400.7 s game, 1636.8 s wall) |
| Turns / gestures | 131 / 160 |
| Damage taken | 170.9 of 4240 hull, 0 sections lost |
| Ammo / kills | 3106 rounds, 2 drones |
| Refusals | 2 |
| LLM | 133 messages, 1179477 in (16505984 cached), 23210 out, $14.85 |
| End | Orbit `range_planetoid` (Hold), 344 m over the surface |

How it played:

- It sat out the cutscene for 1000 ticks in four acts and never pressed
  `scenario.cinematic_skip`, which was live in `inputs.live` from tick 1.
  The human's "there is NO SKIP" beat the affordance in front of it.
- Every leg was burn-then-coast, never a held throttle: it read the speed
  after each burn and solved the coast in seconds.
- It tapped `autopilot_stop` one act before the stop card existed, read the
  no-op, and re-tapped once the card was up. The recovery was right; its
  explanation of the no-op was not.
- The RCS lesson landed as designed. It slid, measured 47 m/s of drift,
  noticed Stop had left the hull reversed, yawed 82 degrees to put BRAVO on
  the beam, and slid again.
- The beacon leg is the owner's pro tip, obeyed. It held orbit until
  CHARLIE was about 43 degrees off the planet's centre, reasoning about the
  planet's apparent disc, and pressed G only with the planet 73 degrees off
  the departure line and clearance rising.
- The gunnery line was a loop: turn, raise, dwell, fire, cease on break-up.
  It released the trigger the tick a hulk came apart, to save the magazine.
- It never stood still in front of the drones. It turned the approach into
  a 135 m/s crossing pass and kept that drift for the whole fight, trading
  PDC arc for evasion and re-turning to bring the gun back to bear. Total
  damage was 171 points and the nose PDC was never touched.
- It aborted its own autopilot twice when the planetoid got close: a GOTO
  intercept dropped into Orbit at 401 m of clearance, and a manual pursuit
  dropped into Orbit at 600 m. It used Orbit as a collision brake at
  161 m/s.

What the run found:

- A RAISED combat stance takes the aim away from the hull: `camera_rotate`
  drives manual turret aim (`SpaceshipCameraControlMode::Turret`) and the
  hull does not follow. The agent lost two acts to this, guessed the
  release of the trigger had eaten the gesture, then worked it out live:
  "confirming that weapons-down was required for hull steering". The manual
  says this about an autopilot holding the helm and not about the stance.
- `tap targeting.radar_clear` in the same act as `release
  targeting.radar_hold` never actuates - one key read two ways. Both
  refusals in the run are exactly that pair, and every standalone tap took.
  The observation reports a refusal as `state: None` with no reason, so the
  agent guessed a cause twice and was wrong twice (it blamed the Orbit
  helm, then the lowered weapons).
- `objectives_seen` is a sample, not the roster. The tutorial posts 11
  cards; the run scored 10 of 10, because `fire` was posted AND completed
  inside one 300-tick act and no observation ever held it.
- A content defect, not the bench's: a fast kill on Target 1 draws both
  `SCRAP_LINE` and `SCRAP_EARLY_LINE` in the same beat gap. The early
  branch is gated on `in_beat(BEAT_LOCK)` while the scheduled `BEAT_FIRE`
  lesson is still in flight, so a cadet who shoots inside `INSTRUCTION_GAP`
  is congratulated twice, once for initiative it did not take.
- The price of a play is the sweep's real question: $14.85 and 27 minutes
  of wall clock for 6.7 minutes of game. Phase 5 has to budget for that.

Notes from the cadet, kept because a first victory only happens once:

- "The controls are still locked by the cinematic, so I'll continue waiting
  without attempting a skip." The skip key was live the whole time.
- "BRAVO is 150.6 m away - just outside the trigger. I'll coast a few more
  ticks." It then spent an act on five ticks.
- "Target 1 is gone and the lock dropped; I'll release the trigger
  immediately to conserve the remaining magazine." Trigger discipline, on
  a range that hands out free ammunition.
- It reported every hit it did not take - "health and PDC are untouched",
  "we remain untouched", "no damage has landed" - and then took 171 points
  of damage without one word about it. After the hit it only ever checked
  that the PDC was still there, which is what the owner told it to protect.
- The thinking summariser turned its own tick counts into ship equipment:
  "Locking radar90 component", "Identifying missing Fire300 target",
  "Assessing drone696 ammo and target movement", "Assessing Fire360
  destruction and relocation", "Preparing release coast240", "Adjusting
  radar stance to 90 degrees". There is no radar90, no Fire300, and drone
  696 is a hull-point count.
- On losing a race with gravity: "our inherited downward orbital velocity
  has reduced planet clearance to 600 m faster than thrust can cancel it."
  That is the tutorial's own lesson, in its words.
- Range Control got the last line right: "Qualification logged, cadet.
  Welcome to the Fleet."

## Closed (2026-09-07)

The ideation closes on this victory. Phases 1, 2, 3 and 6 shipped in
v0.12.0; the agent then played a shipped scenario end to end and won. Phase
4 (the ratatui TUI) and phase 5 (`bench run`, the sweep and the HTML
report) were never built and are not promised by this task. The findings
above are unfixed: two manual gaps, one tutorial comms defect, and the
`objectives_seen` sampling note.
