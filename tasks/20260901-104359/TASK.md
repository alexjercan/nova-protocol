# Combat balance and AI: reproduce weak combat, then fix what it blames

- STATUS: CLOSED
- PRIORITY: 65
- TAGS: v0.15.0,combat,ai,balance

Rewritten in place on 2026-09-21 at owner direction and scheduled for
v0.15.0. Epic: `20260921-231507`.

## This supersedes the lance-run framing

This task was "Teach the AI to fly a railgun lance run" (split out of
`20260824-125947` on 2026-09-01). That framing is SUPERSEDED, for two
reasons:

1. Its premise has moved. The old body said the AI "lands the occasional
   lance hit and never sets one up", against a system it called
   `railgun_ai_input`. That symbol no longer exists. The AI railgun is now
   `crates/nova_ship/src/input/ai/railgun.rs` with an `AIRailgun` cadence
   component (`railgun.rs:50`), a commit envelope
   (`ai_railgun_envelope`, `railgun.rs:72`), and a fired-shot observer
   (`on_railgun_fired_burn_ai_cadence`, registered at
   `crates/nova_ship/src/input/ai/mod.rs:182`). Owner, 2026-09-21: the AI
   can already fire and land railgun shots.
2. Its scope was too narrow to fix the thing that is actually wrong. One AI
   manoeuvre is a candidate answer, not the question.

The lance run is NOT dropped. It survives below as one candidate lever,
with the old task's open questions preserved intact.

## The problem to reproduce

Owner report, 2026-09-21, in the owner's terms:

- Spamming weapons as the player SEEMS to improve the odds of winning.
- Combat FEELS weak.

Both are impressions. Neither is measured. Nothing in this task may be
balanced until they are.

Owner follow-up, 2026-10-04: the AI's PDC fire feels weak when it stops in
bursts; torpedo pauses feel long, but expensive, powerful torpedoes must not
be dumped all at once; the AI rarely uses the hard-to-aim, high-damage
railgun and should learn to set up its shots. The owner suggested using
range and opportunity rather than blind timers. These are observations and
directions for investigation, not measured failures, approved policies, or
authorization to change combat.

## Reproduce and measure first

Produce a MEASURED, MATCHED BASELINE before changing any value or any
behaviour. The baseline is the deliverable of the task's first half; the
balancing is the second half and is not authorized by this body.

The baseline must at minimum answer:

- What is the player's win rate today, in a repeatable fight, under a
  disciplined firing pattern versus a spam firing pattern? Matched sets, same
  reference, same seeds, same hulls. If spam wins more, the baseline says by
  how much. If it does not, the first reported symptom is closed as not
  reproduced and the task narrows to the second.
- What does "weak" measure as? Time to kill, hits landed per second of
  engagement, damage per exchange, how long a fight runs, how often a fight
  ends by attrition rather than by a decision - pick the figures that make
  the feeling legible and say why they were picked.

Constraints on the measurement, not on its implementation:

- Compare matched repeat sets against a NAMED reference. Never assert timing.
- Run the same fight on both hull sizes if the figures differ by hull.
- The proof implementation is the implementer's call. `nova-probe` for a
  deterministic flow, `nova-bench` for a player flow, or an asserted example -
  whichever actually observes win rate and exchange figures. Do not pick one
  here.
- Closest existing evidence: the `wfc_arena` example
  (`examples/playable/wfc_arena.rs`) is the nearest repeatable fight in the
  tree. There is NO existing player win-rate baseline anywhere in the repo;
  it has to be built.

Preserve the before artifacts. The after set must be comparable to them.

## Candidate levers, evidence-driven only

These are places the measurement MIGHT point. Not one of them is an approved
change, and no target value is set here. Each is listed with the code that
makes it a candidate. Pull a lever only when the baseline blames it.

**1. Player versus AI weapon-use asymmetry.** An AI ship's turrets are gated
by a free-running burst cycle and a line-of-fire check; the player's are not.

- `crates/nova_ship/src/input/ai/guns.rs:69` `AI_BURST_FIRE_SECS` is 1.5 s of
  fire, `AI_BURST_HOLD_SECS` 0.8 s of hold, cyclically (`guns.rs:167`).
- `on_projectile_input` (`guns.rs:269`) applies that cadence, and the
  line-of-fire gate, through a query bounded `With<AISpaceshipMarker>`
  (`guns.rs:289`). A player hull is never found by that query, so neither
  gate applies to it.
- The player's turret fire is a held input with no cadence:
  `on_turret_input` at `crates/nova_ship/src/input/player/weapons.rs:182`.
- Point defence bypasses the cadence deliberately (`guns.rs:305`); that is a
  stated design choice, not an oversight, and should not be "fixed" blind.

If the baseline shows spam beating discipline, this asymmetry is the first
place to look. It does NOT follow that the answer is to cadence-gate the
player. Making the AI unconstrained, changing what sustained fire costs, or
leaving it alone are all live answers.

**2. Standoff and range geometry.** The AI flies a fixed standoff envelope.

- `crates/nova_ship/src/input/ai/maneuver.rs:48` `AI_STANDOFF_CLEARANCE` is
  100 u, `:53` `AI_STANDOFF_BAND` 25 u, `:68` `AI_STANDOFF_OUTER_EDGE` their
  sum. `AIControllerConfig::standoff_clearance` overrides per ship
  (`maneuver.rs:79`).
- `maneuver.rs:35` `AI_ORBIT_AUTHORITY_RESERVE` caps what the orbit term may
  spend.

If fights settle at a range where weapons are weak, or where accuracy
collapses, the band is the lever - and the fire gates have to cover whatever
band is chosen (`maneuver.rs:52` says the sum is what the fire gate must
cover).

**3. The lance run.** Preserved from the superseded body, unchanged in
substance:

- The AI commits a railgun shot when its bore already points inside the
  envelope and the target is in reach. It does not FLY to make that true; it
  fires when its orbit sweeps the nose across a target.
- A lance run would be: break the standoff orbit, roll onto the target's
  line, commit through the charge, fire, peel off.
- The charge window is the tell. A run must be readable from the cockpit
  BEFORE the slug arrives, not after.
- Open question, carried forward: what a raider does when the run is spoiled
  mid-charge - hold the shell, or dump it downrange.
- Open question, carried forward: whether the AI weighs the shot against its
  own recoil, since a lance run spends the attitude budget the orbit needs
  back.

This is a candidate lever for "combat feels weak", because a readable,
frightening verb is a different fix from a numeric one. It is not scheduled
on its own merits.

## 2026-10-04 scouting and proposed sequence (unapproved)

**Verified in source, not in a measured fight:**

- `crates/nova_ship/src/input/ai/mod.rs:203-249` chains acquisition,
  behaviour, combat flight, turret aim/fire, torpedo commit/trigger, and
  railgun trigger in `Update`. The burst clock alone ticks in `FixedUpdate`
  (`mod.rs:190-193`). Sensors and point-defence systems precede this chain.
- `guns.rs:167-213,269-363`: `AIFireCadence` runs a free-running
  1.5-second fire / 0.8-second hold cycle for turrets shooting ships. **Point
  defence already bypasses that cycle and the AI line-of-fire gate.** The
  owner's description of bursty "PDCs" may refer to the same turrets while
  they are shooting ships, not to their point-defence mode. Verify this in a
  run before blaming point defence.
- `crates/nova_ship/src/input/ai/torpedo.rs:18,107-205,226-263` adds a
  10-second **per-bay** AI cooldown above the bay's own reload; only a
  spawned torpedo burns the AI cooldown. The bay spends finite ammunition
  on a real launch (`sections/torpedo_section/bay.rs:195-307`); inventory
  reload is separate (`sections/ammo.rs:262-280`). Independent bays can be
  ready together, so simply shortening the clock can make volleys larger.
  A requested trigger does not prove a launch, hit, or interception.
- `railgun.rs:33,100-169,179-256` has a 14-second AI shot cadence and a
  current-bore commit check, but `maneuver.rs:357-503` flies the normal
  standoff orbit rather than setting up a bore-on-target attack. Once the
  gun begins charging, releasing its trigger does not abort it; safing
  weapons dumps the charge but retains the shell
  (`sections/railgun_section/firing.rs:75-138`). Recoil acts at the muzzle
  (`firing.rs:215-230`), so an off-axis mount can torque the hull.
- `examples/playable/wfc_arena.rs:17-22,55-60,1591-1605,1723-1765`
  pins a generated roster by seed and logs team shots, damage, structure,
  and range. Its scripted walk is AI-vs-AI and refuses a player slot
  (`wfc_arena.rs:374-387`); its generated hulls can have unequal weapons.
  It does not yet prove player win rate or per-shot hits. The arena's
  authored 1.8 km engage gate deliberately opens at gun range
  (`wfc_arena.rs:279-307,1110-1123`); do not mistake that existing tuning
  for a combat result.

**Candidate work, conditional on a preserved baseline and owner approval:**

1. Name reference revision, arena seed, team assignments, actual hull seeds,
   weapon loadouts, and hull sizes. Compare matched repeated fights with
   identical aim and opponents, changing only held versus timed player fire.
   Record win/loss, time to decision, engagement range, rounds launched,
   actual hits/damage dealt and taken, torpedoes launched/intercepted,
   railgun opportunities/fired/hits, and remaining ammunition. Extend
   instrumentation only where existing arena logs cannot distinguish them.
   Preserve before artifacts for the after comparison; choose probe, bench,
   or an asserted example only once it can observe those figures.
2. If the burst hold proves harmful, replace the free-running ship-target
   clock with shot-opportunity checks: existing range, lead/alignment and
   clear-line gates, with sustained fire on credible close shots and stricter
   acceptance of marginal distant shots. Keep point defence uninterrupted.
   Compare hit rate and ammunition spent; do not merely shorten the blind
   pause or silently remove an intentional safety gate.
3. If torpedo pauses or simultaneous spending prove harmful, investigate a
   ship/target-level salvo decision that considers already inbound friendly
   rounds, interception or target outcome, available ammunition and future
   exchanges. Retain bay reload and finite stock; allow deliberate multiple
   rounds when one would be defeated by point defence. Define what happens
   when a projectile disappears without a known hit before replacing the
   per-bay AI clock. Do not assume one torpedo is always the right salvo.
4. If rare railgun use is reproduced, evaluate a telegraphed attack run:
   leave orbit, approach and align the actual gun bore, hold through charge,
   fire, and recover to standoff. Keep the existing geometric commit as the
   last shot check. Resolve helm ownership, lost target, threatened ship,
   charge interruption, and recoil before adding a maneuver state. Prove
   bore alignment and fired shots in real flight, not just a trigger request.

Candidate call graph: `sensor -> behaviour -> normal combat flight ->
free-running turret/bay/railgun gates -> weapon spawners` becomes `sensor ->
behaviour -> combat flight (optional railgun run) -> opportunity fire and
coordinated torpedo commit -> existing weapon spawners`. The proposed work
would remove the blind ship-target burst and independent AI bay wait, not the
weapon reload, point-defence protection, safety checks, or finite ammo.
Possible regressions are wasted PDC ammo, torpedo floods against point
defence, lost helm authority, and unannounced railgun shots; proof must
observe these rather than infer success from input or exit status.

## Not decided here

- No target win rate, time to kill, damage figure, cadence, or standoff
  distance is set by this body. Any number in this task is a CURRENT value
  read out of the code, never a goal.
- Whether the answer is a numeric rebalance, an AI behaviour, a player-side
  constraint, or a mix.
- The proof implementation; whether the proposed opportunity, salvo, and
  railgun-run policies are chosen at all; the torpedo reserve/volley rule;
  railgun helm and mid-charge interruption ownership.

Each of these is a decision to bring to the owner WITH the baseline in hand.
The 2026-10-04 proposal is a research plan, not implementation approval.

## Done when

- A matched, repeatable baseline exists and is recorded with this task,
  against a named reference, covering both reported symptoms.
- Each symptom is either reproduced with figures, or explicitly closed as not
  reproduced.
- What the baseline blames is named, with the evidence that blames it.
- The change that follows is approved by the owner before it is made, and its
  after-set is comparable to the preserved before-set.
- The preserved lance-run questions are either answered, scheduled as their
  own task, or explicitly dropped with the reason recorded here.

## Release closure and limits (2026-10-06)

The matched WFC baseline was AI-only, not the planned player held-vs-disciplined
comparison: 60 runs yielded 44 decisions, 10 undecided and 6 invalid, without
a repeat-based variance estimate. It did not reproduce or refute a player spam
advantage. The approved turret and torpedo policies shipped in PR #109; its
focused regressions and master CI passed. Railgun bore-setup trials did not
support a policy change and were reverted. An owned railgun attack run, a
player-action comparison, and hit/PD attribution are cut from v0.15.0 rather
than represented as completed proof. The task's CLOSED status covers this
bounded shipped policy and the recorded investigation, not those unmet
measurement goals. Reopen or create scoped work before claiming them solved.
