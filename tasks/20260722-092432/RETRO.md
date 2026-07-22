# Retro: convoy loiters as unarmed non-combatant AI ships

- Landed: aeaa3761 (squash), 1 review round, out-of-context APPROVE.

## What changed and why

The Lifeline convoy (two unarmed haulers) drifted off / "crashed" when raiders
shoved them - they were `controller: None`, at rest, with no thrust to recover.
Now they are unarmed AI ships that loiter a slow patrol loop through the belt
and hold their ground under fire, without ever fighting.

Delivered a small NON-COMBATANT AI feature:
1. `AINonCombatant` marker (nova_gameplay): `update_ai_target` skips it and keeps
   its `AITarget` clear, so `update_behavior_state` reads "nothing hostile" and
   holds the passive routine - even under fire (the None early-return precedes
   the recently-damaged engage branch, which is exactly "hold ground").
2. Auto-detect (nova_scenario): an AI ship with no turret/torpedo section gets
   the tag at spawn. Chose auto-detect over an AIControllerConfig field (would
   have churned ~13 construction sites, and the loadout IS the truth).
3. Content (lifeline): `convoy_hauler` -> `controller: AI` with a per-hauler
   loiter loop; the cargoa hull already had thrusters, so no craft change.

## Difficulties / findings

- VERIFY-FIRST re-sharpened the premise (like the sibling gravity task): lifeline
  has no gravity well/planetoid, so the owner's "crash into the planetoid" is
  knockback DRIFT, not gravity. That ruled out "orbit the body" (no well) and
  pointed at active loiter + drift-recovery as the real fix.
- The FSM does NOT gate Engage on having weapons - a weaponless AI ship would
  chase raiders it can't shoot. That is why a real non-combatant gate was needed
  (a small leash would not reliably keep them passive: a raider inside the leash
  still trips Engage).
- BLOCKED MID-TASK by a task-1 regression the probe surfaced (OnStart gate stamps
  read undefined scenario_elapsed -> opening objectives never posted + 174 error
  lines). Fixed it as its own task (20260722-114541), landed, then rebased this
  branch onto it. The lifeline probe is clean only with BOTH changes - a good
  reminder that probing one feature can expose a latent bug in another.

## Self-reflection - what to do differently

- The probe was the hero twice this task: it proved the loiter behaviour (haulers
  stay in-region under real physics) AND caught the task-1 regression. Reaching
  for the highest-fidelity harness EARLY (not just at the end) would have surfaced
  the regression before it reached master. Pairs with the 114541 lesson: probe
  scenario content, don't assume "data-only" is behaviour-safe.
- Auto-detect from loadout is the right seam and sets up the critical-damage
  backlog feature (weapons-destroyed => non-combatant is the dynamic version of
  the same predicate) - a nice compounding hook, noted for 20260722-092320.

## Follow-ups / known-minor

- Review LOW note (not filed, harmless + edge-case): `mirror_ai_combat_state`
  still gives a non-combatant a `WeaponsRaised(true)`/`CombatLock` when a torpedo
  is inbound, and it still computes an `AIPointDefenseTarget`. No guns consume
  either, so it is cosmetic; worth a tidy if a future non-combatant has an
  audible/visible weapons-hot tell. Left as a known-minor, not task noise.
- `AINonCombatant` is not `register_type`'d for reflection - consistent with
  the sibling AI markers (AILeash/AISpaceshipMarker), so intentionally left.
- Owner manual acceptance: replay Lifeline - the haulers fly around / stay in the
  belt and never fight.
