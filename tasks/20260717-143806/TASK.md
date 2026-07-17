# Ledger ch4: the Auditor spawns hot (301u inside its 450u envelope, torpedo tube) - decide drama vs fairness

- STATUS: CLOSED
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

## Steps

- [x] webmods/the-ledger/ledger_ch4.content.ron: both Auditor spawn sites
  (the two mutually exclusive ending handlers) swap turret_dorsal from
  better_turret_section to light_turret_section; tube and positions stay
  (the entrance IS the drama; the sibling 20260717-151208 fixes the bay
  placement). Bundle version 1.1.0 -> 1.2.0.
- [x] balance_audit acknowledgments: an in-repo
  crates/nova_assets/balance_acks.ron (bundle, scenario, hostile, kind,
  reason, task) applied at the REPORT/gate layer - findings() stays pure;
  WARN-grade findings matching an ack print as ACK with the reason and do
  not count as warnings; ERROR findings are NEVER ackable; an ack
  matching nothing surfaces as its own WARN so stale acks rot loudly.
- [x] Ack the Auditor's two tube-envelope WARNs (one per ending branch)
  citing this task and the user's drama verdict.
- [x] Tests: ack suppresses a matching WARN; an ack cannot suppress an
  ERROR (fail-first: a synthetic acked ERROR still fails); a stale ack
  produces its own WARN; the shipped acks file round-trips and the real
  tree now reports zero active warnings.
- [x] Docs: guide-author-scenario.md audit paragraph gains the ack
  mechanism; CHANGELOG (Scenarios: finale gun downgrade; Internals &
  Tooling: acks); NOTES.md with the envelope math (light turret 270u <
  301.5u spawn distance; the tube alone keeps 1000u, hence the ack).
- [x] Verify: cargo test -p nova_assets balance:: + --test
  balance_audit_gate; cargo run --bin balance_audit (expect 0 errors, 0
  active warnings, 2 ACK lines); content_lint; fmt last. Full suite on CI.

## Close-out record

All six steps landed; the envelope math, the ack mechanism's rules and
the numbers are in NOTES.md. Verification: balance unit tests 11/11
(including the never-ack-an-error fail-first and the one-ack-per-finding
rule the first run genuinely failed), balance_audit_gate green over the
real tree, balance_audit prints 0 errors / 0 warnings / 2 acked,
content_lint clean, workspace --all-targets green, fmt last. Full suite
on CI per standing instruction.
