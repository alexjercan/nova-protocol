# Review: The Ledger - four-chapter campaign mod

- TASK: 20260716-123535
- BRANCH: content/the-ledger

## Round 1

- VERDICT: REQUEST_CHANGES

Verified independently: all four chapters load recursively through the
real loaders; the real deploy invocation publishes the-ledger 1.0.0;
scripted cross-checks show zero unknown prototypes, dangling chain
targets, or unspawned filter ids. Re-derived the branch logic: the ch4
endings have disjoint choice guards, the Auditor cannot exist before
act 2, and every Defeat handler is gated below its chapter's win act so
a post-victory death cannot overwrite a win. Salvage semantics checked
at the source: crates are NOT engine-consumed (salvage.rs's contract:
the scenario despawns them), so an early black-box touch cannot
soft-lock chapter one.

- [x] R1.1 (MAJOR) webmods/the-ledger/ledger_ch1.content.ron - the three
  quota-crate handlers count and complete objectives but never
  DespawnScenarioObject the crate (the shipped shakedown pattern pairs
  them). A collected crate stays in the world and CAN BE RE-ENTERED
  while act == 1, so the quota can complete off one crate entered three
  times, leaving containers floating "uncollected". Add
  `DespawnScenarioObject((id: "ledger_crate_N"))` to each pickup
  handler, and the same for the black box in its act-2 handler.
  - Response: fixed - DespawnScenarioObject added to all three quota
    handlers and the black-box handler (4 sites), webmods_validation
    re-run green.
- [x] R1.2 (NIT) The campaign has no human playthrough; the close notes
  say so honestly and route tuning to the first playtest. No action on
  this branch; recorded so the verdict's scope is clear.
  - Response: acknowledged - the close notes carry the same limit.

## Round 2

- VERDICT: APPROVE

R1.1 verified: all four pickup handlers now open with
DespawnScenarioObject on their own crate (the shakedown pairing), so a
crate cannot be re-entered or double-counted and collected containers
leave the world; the bundle still loads recursively. R1.2 stands as a
recorded limit. No new findings.
