# Review: Auditor armament + audit acknowledgments

- TASK: 20260717-143806
- BRANCH: fix/auditor-armament

## Round 1

- VERDICT: APPROVE

Scope check first: the ch4 content edit is exactly what the playtest
verdict ordered. Both Auditor spawn sites (the OnEnter(handoff_berth)
handler at ledger_ch4.content.ron:212 and the OnEnter(burn_buoy) handler
at :297) swap better_turret_section -> light_turret_section; the
non-comment diff in that file is exactly those two source lines. The
remaining better_turret_section at :81 is the PLAYER's own gun -
correctly untouched. Tube position (0.0, 0.0, 0.5) and both spawn
positions (0.0, 30.0, -260.0) are byte-identical to master at both
sites, so no scope bleed into sibling 20260717-151208 (still OPEN, owns
the bay move). Bundle 1.1.0 -> 1.2.0. Envelope math checks out: light
turret effective range = 0.9 x 60 x 5.0 = 270u < 301.5u spawn distance
(sqrt(300^2 + 30^2)), and the tube pins threat_envelope() at 1000u, so
the WARN survives any gun choice - the ack is the honest mechanism, not
a workaround.

The ack mechanism, adversarially:

- ERROR suppression: impossible. The matches closure in
  partition_findings (balance.rs:259) requires
  `finding.severity == BalanceSeverity::Warn` as its first conjunct, so
  an Error-grade finding can never enter `acked`; it always lands in
  `active`, where both the bin (errors counted only over active, exit
  FAILURE) and the gate (asserts no active Error) fail on it. The
  an_ack_never_suppresses_an_error test (balance.rs:809) is a genuine
  tripwire: its ack matches the synthetic error on all four identity
  fields (b/s/x/spawned-dead), so deleting the severity conjunct would
  move the error into `acked`, tripping `assert_eq!(active.len(), 1)`
  and the stale assert both. Not decorative.
- One-ack-one-finding: re-derived the `.enumerate().position(...)` at
  balance.rs:274. position() returns the count of items consumed before
  the first match; since the enumerate is unfiltered, that count equals
  the closure's enumerate index, so `used[i]` and `&acks[i]` address the
  same ack. Correct as written (see R1.2 for the readability nit), and
  acks_match_one_warn_each (balance.rs:794) pins the semantics: one
  ack, two identical findings, one stays active.
- Stale detection: the gate asserts `stale.is_empty()`
  (balance_audit_gate.rs) and the bin returns FAILURE on
  `!stale.is_empty()` while also counting stale into the warning tally.
  A typo'd ack fails CI loudly; an ack aimed at an ERROR is doubly loud
  (error counted AND ack stale).
- Matching is exact String equality on (bundle, scenario, hostile,
  kind) - no wildcard, prefix, or glob path exists. Hostile ids are
  scoped by scenario and bundle in the key, so no cross-scenario bleed.
- The shipped two-entry ack pairing is deterministic and correct:
  findings() iterates events in file order (handoff handler precedes
  burn), partition_findings consumes findings in order and takes the
  first unused ack, so ack 1 (full reason) prints against
  OnEnter(handoff_berth) and ack 2 ("second ending branch") against
  OnEnter(burn_buoy) - verified in the live output below. The ack
  schema has no trigger discriminator, so reordering the acks file
  would swap the reasons across branches; both reasons are
  branch-agnostic in substance, so nothing is misattributed today.

Judgment on the design: honest. Acked findings still print (tagged ACK
with reason and task), errors are structurally unackable and tested,
stale acks fail both the bin and the CI gate, and findings() stays pure
(the partition is a report/gate-layer concern). This is an exception
LIST, not a suppression switch - the failure modes all fail loud. The
wiki paragraph and CHANGELOG describe exactly this behavior.

History note on the NOTES claim: the branch carries one implementation
commit (0bf7dd87), so the "first run matched both findings to the first
ack, caught by acks_match_one_warn_each" bug->fix cycle happened inside
the working session and cannot be confirmed from git history. What is
verifiable: the shipped test demands exactly the semantics the bug
would have violated, so the rule is pinned regardless.

- [ ] R1.1 (MINOR) tasks/20260717-143806/NOTES.md:27 - the Auditor's hp
  is recorded as 660; the resolved section sum is 730 (controller 100 +
  2x reinforced hull 200 + thruster 70 + light turret 60 + tube 100).
  The 660 figure looks like the thruster was dropped from the sum. The
  error is in the conservative direction (the boss is tankier than
  recorded, TTK for the player ~1.8s at 400dps, not ~1.65s), but this
  is the design record future rebalances will consult - correct the
  line to 730hp. (Also note the gun swap itself shed 70hp: turret
  section health 130 -> 60, so the boss went 800 -> 730 total.)
  - Response: fixed - `(0..acks.len()).find(..)` indexes directly. fixed - NOTES records 730hp with the section-by-section sum.

- [ ] R1.2 (NIT) crates/nova_assets/src/balance.rs:271-274 - the
  `.iter().enumerate().position(|(i, ack)| !used[i] && ...)` construct
  uses two coincident indices: the closure's enumerate `i` for the used
  check and position()'s return value for indexing. They agree only
  because the iterator is unfiltered before position(). Correct today,
  but a future `.filter(...)` inserted upstream would silently
  desynchronize them. `(0..acks.len()).find(|&i| !used[i] &&
  matches(&acks[i], &bundle, &finding))` says the same thing with one
  index.
  - Response:

- [ ] R1.3 (NIT) crates/nova_assets/src/balance.rs:236 - BalanceAck.kind
  is a free String; a typo'd kind parses and only surfaces as a stale
  ack. That failure IS loud (CI fails), and
  shipped_acks_parse_with_valid_kinds additionally pins the shipped
  file, so this is defense-in-depth already. Still, deserializing kind
  directly into FindingKind (serde rename "spawned-dead"/"close-spawn")
  would reject typos at parse time and delete the string-validity test.
  - Response: declined with reasoning - RON deserializes unit enum
    variants from identifiers, not strings, so a typed kind would either
    change the ack file's authoring shape (kind: CloseSpawn) or need a
    custom deserializer; the free String is guarded at parse time by the
    shipped_acks_parse_with_valid_kinds test and at runtime by stale-ack
    CI failure. Revisit if the kind set grows.

- [ ] R1.4 (NIT) balance record, no action required - the finale's gun
  pressure is now the campaign's lowest: Auditor 96 dps / 1 tube at
  301u vs ledger_ch2b's OnStart heavies at 496 dps (922u) and
  broadside_gunship at 800 dps + 2 tubes (1214u). The boss dies to
  ~1.8s of aligned player fire (730hp vs 400dps) while needing 5.2s of
  sustained gunfire itself, and the torpedo is a screenable 100-damage
  hit vs the 500hp player. On raw numbers the finale is UNDER-dramatic
  next to the mid-campaign fights; the counterweights are real (the
  301u entrance with the gun coming online after a ~31u close, the
  immediate torpedo launch, one hard-cover rock) and the user's
  playtest verdict explicitly chose this shape, so this is a recorded
  observation only. If a later playtest finds the finale flat, the
  levers are an hp bump or a second tube, not a gun upgrade (which the
  ack reason correctly notes was downgraded for exactly this spawn).
  - Response: acknowledged - no action per the playtest verdict; the
    levers (hp, second tube) stay on record here.

### Verification record

All commands run in /home/alex/.cache/sprouts/nova-protocol/fix/auditor-armament
at 0bf7dd87. Per standing instruction the full suite runs on CI; only
the task-relevant targets were run locally.

- `cargo test -p nova_assets balance::`
  - `test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 71 filtered out; finished in 0.00s`
  - includes acks_match_one_warn_each, an_ack_never_suppresses_an_error,
    unmatched_acks_are_stale, shipped_acks_parse_with_valid_kinds.
- `cargo test -p nova_assets --test balance_audit_gate`
  - `test shipped_content_carries_no_balance_errors_and_no_stale_acks ... ok`
  - `test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s`
- `cargo run -p nova_assets --bin balance_audit`
  - `ACK   [the-ledger] ledger_ch4_the_buyer: close-spawn: 'auditor' (OnEnter(handoff_berth)) spawns 301u from the player spawn, inside its own 1000u threat envelope - a mid-fight reinforcement arriving on top of the fight | acked by 20260717-143806: The finale's drama entrance: the Auditor drops in at 301u by design. Its gun is the light mook turret (downgraded for exactly this spawn); the WARN is its torpedo tube's 1000u launch envelope, and shooting the incoming torpedo down IS the fight. Playtest verdict 2026-07-17.`
  - `ACK   [the-ledger] ledger_ch4_the_buyer: close-spawn: 'auditor' (OnEnter(burn_buoy)) spawns 301u from the player spawn, inside its own 1000u threat envelope - a mid-fight reinforcement arriving on top of the fight | acked by 20260717-143806: Same acknowledgment for the second ending branch (the two handoff/burn handlers are mutually exclusive spawn sites of the same boss).`
  - `balance_audit: 11 combat scenario(s), 0 error(s), 0 warning(s), 2 acked`
  - exit code 0.
- `cargo run -p nova_assets --bin content_lint`
  - `content_lint: clean (1 warning(s))` - the one warning is the
    pre-existing dual-spawn note on 'auditor' (mutually exclusive
    handlers), expected and referenced by both sibling tasks.
- `cargo test -p nova_assets --test webmods_validation`
  - `test every_webmods_bundle_loads_recursively ... ok`
  - `test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.21s`
