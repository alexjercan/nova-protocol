# Auditor armament + audit acknowledgments - design record

Task 20260717-143806. The playtest verdict (2026-07-17): keep the
finale's close entrance, downgrade the gun, make balance_audit happy.

## What shipped

- ledger_ch4.content.ron: both Auditor spawn sites (the two mutually
  exclusive ending handlers) swap better_turret_section ->
  light_turret_section. Positions, tube, hulls untouched. Bundle 1.2.0.
- The envelope math that forced a second mechanism: the light gun's 270u
  reach is below the 301.5u spawn distance, but the torpedo tube keeps
  the threat envelope at 1000u (the AI launch cooldown starts elapsed),
  so no gun downgrade alone can clear the close-spawn WARN while the
  tube exists - and the tube IS the finale's drama (the user separately
  asked to fix its clipping, so it stays). Hence:
- balance_audit ACKNOWLEDGMENTS (crates/nova_assets/balance_acks.ron):
  a WARN a human decided is intended carries its reason and deciding
  task; acked findings print tagged ACK and stop counting; ERRORs are
  never ackable (fail-first unit test); each ack spends on exactly ONE
  finding (duplicate spawn branches need one ack each); a stale ack
  fails the CI gate until pruned. The Auditor's two branch WARNs are the
  first entries.

## Numbers after the change

Auditor: 96 dps burst (was 400), 730hp (100 controller + 2x200 hull +
70 thruster + 60 light turret + 100 tube), 1 tube, 301.5u entrance. TTK vs
the 500hp player: 5.2s sustained gunfire plus the screenable torpedo -
the fight is the entrance, the tank, and the ordnance, not the shred.
balance_audit: 11 scenarios, 0 errors, 0 warnings, 2 acked.

## Difficulty found by tests, not review, this cycle

The first run of the new partition matched BOTH branch findings against
the FIRST ack (position() ignored the used flag), leaving ack two stale -
caught immediately by the acks_match_one_warn_each unit test written
alongside (one ack, two identical findings, one must stay active). The
duplicate-spending semantics existed because the test demanded them.
