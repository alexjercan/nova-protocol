# Ledger ch4: the Auditor spawns hot (301u inside its 450u envelope, torpedo tube) - decide drama vs fairness

- STATUS: OPEN
- PRIORITY: 44
- TAGS: v0.7.0, scenario, content, balance

Found by the balance audit rig (tasks/20260717-112656, spike
tasks/20260717-111808/SPIKE.md) - the first content nobody had
balance-checked by hand.

The numbers (balance_audit over shipped RON): in ledger_ch4_the_buyer the
Auditor spawns at 301u from the player spawn on BOTH ending branches
(OnEnter(handoff_berth) and OnEnter(burn_buoy)), inside its own 450u
better-turret envelope, carrying 400 dps plus a torpedo tube, against a
500hp player (TTK 1.2s at burst dps) with only 1 hard cover rock in the
scenario. That is the "reinforcement on top of the fight" shape the ch2
rework removed - but ch4 is the finale and the Auditor's entrance may be
INTENDED drama (the player chooses an ending and the consequence arrives
hard).

Decide with a playtest: if intended, keep it and record the verdict here
(the audit's WARN is informational and does not gate); if not, apply the
ch2 pattern (push the spawn to 600u+ on a single bearing, or stage an
approach beat on the scenario clock - the timer primitive from
20260717-112647 ships exactly this tool) and add hard cover near the
berth. Either way the fight's numbers are now on record.

2026-07-17 playtest VERDICT (user): overall difficulty is in a good place
now - the rework family stays as landed. For THIS task the drama-vs-
fairness fork is decided: KEEP the close entrance, make balance_audit
happy by downgrading the armament ("use worse turrets on ships that spawn
really close... give them a worse gun for example"). Note for /work: a
worse GUN alone does not clear the WARN while the torpedo tube keeps the
threat envelope at 1000u (the tube's launch cooldown starts elapsed) -
read the ch4 fight first and pick the honest combination (worse gun +
tube question decided against the fight's actual script; the user gave
latitude with "for example"). Sibling task 20260717-151208 fixes the same
ship's clipping torpedo bay - coordinate but do not merge scopes.
