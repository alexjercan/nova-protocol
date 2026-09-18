# After artifacts: the game without the manual speed governor

Run 2026-09-18 against `30b9b8220` (code `94a96a0f2`, docs `30b9b8220`).
Host quiet before each run: no game process, load average under 1.5. Every
measurement was run on its own, one at a time. `ALSA_CONFIG_PATH=empty` on
every command mirrors CI's no-audio host.

`before.txt` is the same tree before the removal: at the cap a held full burn
across the nose delivered EXACTLY zero delta-v.

Artifacts in this folder:

| file | what it is |
| --- | --- |
| `probe-tutorial-checks.json` | probe verdict for the shipped `tutorial` scenario |
| `probe-chapter-one-checks.json` | probe verdict for `system_chapter_one` |
| `bench-tutorial.txt` | Basic Training's opening card, flown |
| `bench-control-feel.txt` | accelerate, coast, turn, cross-burn, STOP |
| `bench-straight-burn.txt` | 12 s of held throttle down the bench range |
| `hud-speed-chip.png` | the rendered frame the chip is read off |
| `hud-speed-chip-crop.png` | the chip, magnified 3x |
| `drive.py` | the deterministic pilot the three bench runs were flown by |

`drive.py` is a `cmd:` agent, not a model: a fixed script, so two runs of one
plan compare. It speaks the referee protocol and reads the world back out of
the view.

---

## PROOF 1 - Basic Training runs to its real outcome

### 1a. The shipped scenario, through probe

```sh
nix develop --command cargo run --features debug -- \
  probe scenario tutorial --correctness-only
```

The clean pass arms `NOVA_PROBE_INVARIANTS=1` and `NOVA_PROBE_TIMELINE` by
itself (`nova_probe_cli::native::env::clean_pass_env`).

```
  process_exit           PASS     1 pass(es), all clean exits
  run_completed          PASS     run_end at frame 379
  reached_playing        PASS     Playing at frame 15
  invariants_held        PASS     0 violations over 379 checked frames
  log_clean              PASS     0 offending lines
  artifacts_loadable     PASS     0 unloadable
```

The timeline shows the opening handler firing and seeding every variable the
card runs on (`beat` 1, `targets_scrapped`, `drones_down`, the per-target
latches), and the range boundary taking all 81 bodies. The invariant summary
is `{"checks": 379, "violations": 0, "health_subjects": 444,
"velocity_subjects": 77}`.

READING. The shipped scenario loads, reaches `Playing` and runs with no
finite-velocity or health-bound break and no error line. This is a BOOT, not
an outcome: `probe scenario` ticks the scenario, it does not fly the card.
That is 1b.

### 1b. The opening card, flown

```sh
nix develop --command cargo run --features debug -- \
  bench play tutorial \
  --agent "cmd:python3 tasks/20260918-110325/proof/drive.py tutorial" \
  --seed 20260918 --ticks 18000 --turns 250 --deadline 900 \
  --out bench-runs/governor-tutorial-2
```

The script holds the throttle from the moment Range Control hands the helm
over, which is the worst case the removal exposes, then taps `[X]` when the
STOP card is posted and keeps its hands off. Score:

```
outcome              none
ended_by             finish
objectives           2/2
objectives_seen      ['burn', 'stop']
objectives_completed ['burn', 'stop']
damage_taken         0.0   sections_lost 0   bad_lines 0
objective_log        posted burn, completed burn, posted stop, completed stop
```

Numbers off `bench-tutorial.txt`:

| | |
| --- | --- |
| peak speed on the ALPHA leg | 210.10 m/s |
| ALPHA card completed | t=22.53 s, at `[0 0 -564]` |
| speed when the STOP card posts | 213.80 m/s, at `[0 0 -1422]` |
| STOP card completed | t=36.58 s |
| at rest | `[-13 0 -2790]` |

Mark ALPHA is authored at `[0 0 -900]` with a 300 m gate, so the gate face is
at z=-600 and the card fires at -564 as it should.

READING. Both cards post and complete, the comms move on to the RCS lesson,
and nothing is damaged. The card is NOT broken by the removal. Two exposures
are real and measured:

1. The leg is now flown at 210 m/s. The old cap was 150 m/s. 210 m/s is the
   CEILING for this beat, not a tail: the trainer picket cannot build more
   than that inside the 600 m to the gate face, so the exposure is bounded.
2. The pattern beat ends 1 890 m PAST mark ALPHA. The next card ("slide
   across to mark BRAVO") is authored as a short RCS translation from ALPHA
   to `[350 0 -900]`, and the RCS caps at 100 m/s. A player who holds the
   throttle now starts that lesson about two kilometres off station. The card
   is reachable - the main drive is not withheld - but the authored geometry
   no longer matches where the ship stops.

NOT PROVEN. The full card to the Victory banner was not flown. The beats
after BRAVO are the radar, the gun, five hulks and two drones; none of them
reads ship speed, and flying them needs a piloted run this lane did not
spend. What IS proven is the two beats the governor actually sat on.

---

## PROOF 2 - season one chapter one runs to its real outcome

```sh
nix develop --command cargo run --features debug -- \
  probe run system_chapter_one --correctness-only
```

```
  process_exit           PASS     1 pass(es), all clean exits
  run_completed          PASS     run_end at frame 9883
  reached_playing        PASS     Playing at frame 51
  invariants_held        PASS     0 violations over 9883 checked frames
  log_clean              PASS     0 offending lines
  artifacts_loadable     PASS     0 unloadable
  system_chapter_one     OK       measured 6/8   31s
```

All seven `outcome:` markers fired, in order, off the timeline:

```
1970 the opening scene hands the helm back and posts the lane
     {"card": "lane", "gate": "lane_one_gate"}
2404 each mark comes down as the next one goes up
     {"gates": ["lane_one_gate","lane_two_gate","lane_three_gate","lane_four_gate"]}
7798 the collar is staged inside the capture envelope
     {"gap_m": 5.0, "capture_m": 10.0, "off_axis_deg": 0.028}
7819 the clamp completes the docking card and starts the transfer
8058 letting go and clamping again leaves no card behind
9881 every card posts and completes in the authored order
     {"board": ["+lane","-lane","+reach","-reach","+dock","-dock",
                "+hold","-hold","+hold","-hold","+release","-release"]}
9881 the chapter ends in victory with the board clear and both hulls alive
     {"banner": "Three people are coming home. ...", "last_card": "release"}
```

Run log: `Outcome: declaring Victory`, then
`chapter one: PASS the chapter is won, board clear`.

READING. The chapter wins, the board clears, both hulls live, and 9 883
frames of continuous invariants hold. The chapter is not broken by the
removal.

CAVEAT, and it matters for the exposure the task named. This range STAGES the
lane: it teleports Kaveri between the four gates rather than flying six
kilometres of rock (`examples/systems/system_chapter_one.rs:24`), and it
writes the travel lock rather than earning it. So this proof says the chapter
STATE MACHINE is intact without the governor. It does NOT test whether the
rock lane can now be blown past at speed. Nothing in the repository flies
that lane by hand, so that exposure is UNMEASURED here.

---

## PROOF 3 - control feel

```sh
nix develop --command cargo run --features debug -- \
  bench play crates/nova_bench/scenarios/range.content.ron \
  --agent "cmd:python3 tasks/20260918-110325/proof/drive.py control-feel" \
  --seed 20260918 --ticks 9000 --turns 200 --deadline 600 \
  --out bench-runs/governor-control-feel-clean
```

The plan turns off the planetoid's line FIRST, so no number below is a
collision or a gravity well. Then: 6 s of held throttle, release, coast 5 s,
yaw 90 degrees to starboard, hold the throttle for 10 s with the velocity
across the nose, then `[X]`. Full trace in `bench-control-feel.txt`.

```
  speed after a 6 s burn from rest  : 484.60 m/s
  speed 5 s after releasing the burn: 491.20 m/s
  speed after the 90 deg turn       : 491.20 m/s
  velocity bearing after the turn   : [-92.8, 0.3]
  along-nose before the cross-burn  : -23.99 m/s
  along-nose after the cross-burn   : 797.29 m/s
  along-nose delta-v while crossing : 821.29 m/s
  speed at the end of the cross-burn: 936.10 m/s
  STOP converged in                 : 16.03 s, final speed 0.400 m/s
```

The turn, sampled once a second while the hull swings:

```
turn t=18.05s speed=491.20 vel_bearing=[-49.0, 0.3]
turn t=19.05s speed=491.20 vel_bearing=[-83.5, 0.3]
turn t=20.05s speed=491.20 vel_bearing=[-91.0, 0.3]
turn t=21.05s speed=491.20 vel_bearing=[-92.8, 0.3]
turn t=22.05s speed=491.20 vel_bearing=[-92.8, 0.3]
```

READING. This is the question `before.txt` asked, answered.

- The drive ANSWERS when the ship is already fast and pointing off its
  velocity. 821.29 m/s of along-nose delta-v in ten seconds, from a state
  where the velocity was 92.8 degrees off the nose. The before artifact
  measured 0.00 for the same gesture.
- Acceleration is flat: 82.5 m/s per second from rest, and the same
  82.5 m/s per second at 900 m/s. Nothing tapers.
- Turning alone does not change the velocity. The speed reads 491.20 m/s at
  every sample through a 93 degree swing.
- Releasing the throttle preserves the velocity. 484.60 m/s at release,
  491.20 m/s five seconds later - the 6.6 m/s is the thruster spool-down
  tail, not drift, and the number is then identical across the next eleven
  seconds.
- STOP still converges: 936.10 m/s to 0.4 m/s in 16.03 s, flip and burn, and
  it hands the helm back.

### The other end of it: what 12 s of throttle now reaches

```sh
nix develop --command cargo run --features debug -- \
  bench play crates/nova_bench/scenarios/range.content.ron \
  --agent "cmd:python3 tasks/20260918-110325/proof/drive.py straight-burn" ...
```

The first attempt at the control-feel plan simply held the throttle down the
range's own nose. `bench-straight-burn.txt` is that walk, re-run under its own
plan name:

```
burn t= 6.03s speed= 483.50  near: Planet surface=4952 m closing=484 m/s
burn t=10.03s speed= 817.10  near: Planet surface=2357 m closing=817 m/s
burn t=12.03s speed=1004.40  near: Planet surface= 543 m closing=1004 m/s
after t=13.03s speed=118.20 vel_bearing=[-133.3, 29.2] pos=[-38 64 -6327]
hull_plates: {'total': 44, 'damaged': 0, 'lost': 0} health 10260.0/10260.0
```

`game.log`: `impacts: 6 contacts for 44527.17 damage`.

READING, and this is an exposure. Twelve seconds of held throttle from rest
now puts the bench gunship at 1 004 m/s and 5.8 km downrange, which is into
Kestrel, seven kilometres ahead of the spawn. The ship hits the planetoid at
a kilometre a second and BOUNCES: 44 527 damage is logged across six
contacts, the planetoid is invulnerable, and the player hull comes out with
44 of 44 plates and full health. The longer first walk, which stayed in
contact while it tumbled, recorded 85.08 damage taken and no sections lost.

So the high-speed collision the task predicted is REACHABLE, and the damage
model does not answer it: a 1 km/s strike on a planetoid costs the player
almost nothing. That is not a governor question and no governor should hide
it. It is recorded here, not fixed.

---

## PROOF 4 - the HUD speed chip, visually

Headless output cannot make this claim, so a frame was rendered and read.

```sh
DISPLAY=:99 NOVA_CAPTURE_DIR=<scratch>/shots NOVA_AUTOPILOT=1 NOVA_CAPTURE=1 \
  nix develop --command cargo run --example lesson_readouts --features debug
```

`lesson_readouts` is the producer whose whole subject is the readouts: it
writes the player's velocity to 120 m/s and shoots the chip beside the
velocity sphere close. An Xvfb was already up on `:99` (pid 511649, not this
lane's - it was reused and left running).

```
nova capture: start_units.png
Screenshot saved to <scratch>/shots/start_units.png
autopilot: cycle complete, no panic (t=7.5s)
```

I OPENED BOTH PNGs AND LOOKED AT THEM.

- `hud-speed-chip.png` is the full 1920x1080 frame. The chip sits to the right
  of the hull, level with the reticle.
- `hud-speed-chip-crop.png` is that chip at 3x.

The chip reads exactly:

```
120.0 m/s
```

One number, one unit, one green box. There is no ` / rated`, no second
figure, no denominator and no slash anywhere in the chip. For contrast, the
locked contact's own card in the same frame still prints its two lines
(`DST 351 m`, `CLS +117.0 m/s`), so the frame is not one where text is
missing - only the rated half of the speed chip is gone.

---

## Defects found, not fixed

Two stale comments still name the deleted governor. Neither changes behavior;
both are prose in live example files, and both were missed by the sweep.

- `examples/screenshots/lesson_readouts.rs:81` - the `DRIFT_SPEED` doc says
  the figure is "inside the 150 m/s cap Basic Training flies under". Basic
  Training no longer flies under a cap; proof 1b measured 210 m/s on its
  first leg.
- `examples/playable/wfc_arena.rs:1106` - "No speed cap (the arena is open
  space)" describes not setting a field that no longer exists.

A wider sweep of `speed cap` / `150 m/s` over `crates/`, `examples/`,
`assets/`, `webmods/`, `web/src/wiki`, `web/src/create`, `docs/` and
`CHANGELOG.md` found nothing else: the remaining hits are the RCS speed cap,
the torpedo cruise cap, the 150 m/s REFERENCE RUNNER the gun and torpedo
tables are sized against, and the `nova_scenario` test that proves a leftover
`speed_cap:` key is refused.

## Recorded failures

The first `tutorial` plan finished a beat early. It watched the live
`objectives` list for a change and treated the STOP card being POSTED as the
card being completed, so it tapped `[X]` before the capability was handed
over and scored 1/2. The driver was fixed to read `objective_log` for
`completed`, and the run above is the re-run. The broken run is not kept; it
proved nothing about the game, only about the driver.
