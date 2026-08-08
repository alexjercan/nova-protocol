# Fix catalog_matches_disk: smoke-list screenshot_nova_os example

- STATUS: CLOSED
- PRIORITY: 45
- TAGS: v0.9.0, bug, test

`tests/examples_smoke.rs::catalog_matches_disk` fails on master: the
`screenshot_nova_os` example is present in the Cargo.toml `[[example]]` catalog
(and on disk under examples/screenshots/) but is in NONE of the smoke lists
(SECTIONS/GAMEPLAY/UI/SCREENSHOTS) nor in NOT_SMOKED. The catalog test asserts
`accounted == catalog_names`, so it fails: "smoke lists (+ NOT_SMOKED) and the
catalog disagree".

Introduced by commit a98de8ed (task 20260726-180807), which added
`screenshot_nova_os` without listing it. Discovered mid-flow during task
20260727-135204 (CRT frame polish).

## Steps

- [x] Add `"screenshot_nova_os"` to the `SCREENSHOTS` list in
      `tests/examples_smoke.rs` (it is a harnessed screenshot producer like its
      siblings screenshot_reel/ui/combat/...).
- [x] BROADER than filed: the assertion diff surfaced a SECOND unlisted example,
      `nova_os_rtt_poc` (added by the same RTT work, task 20260726-193233). It is
      a standalone feasibility prototype (own `App`, own OK/FAIL verdict,
      auto-exits - never reaches `GameStates::Playing`), so it goes in
      `NOT_SMOKED` with a reason, not a smoke list.
- [x] Confirm: `cargo test --test examples_smoke catalog_matches_disk` passes.

## Definition of Done

- catalog_matches_disk is green. (cmd: nix develop --command cargo test --test examples_smoke catalog_matches_disk)

## Close-out

What changed and why:
- `tests/examples_smoke.rs::catalog_matches_disk` was red on master: the catalog
  had two examples that were in no smoke list nor NOT_SMOKED. Fixed both:
  `screenshot_nova_os` -> SCREENSHOTS (a harnessed screenshot producer that
  reaches Playing, like its siblings); `nova_os_rtt_poc` -> NOT_SMOKED with a
  reason (a standalone RTT feasibility prototype with its own App + OK/FAIL
  verdict + auto-exit, so it never reaches Playing).
- Repro/pin: the failing catalog_matches_disk assertion WAS the reproduction
  (its set diff named exactly the two offenders); it now passes as the pin.

Difficulties:
- Filed for one example; the assertion diff (`left`/`right` BTreeSet compare)
  revealed a second (`nova_os_rtt_poc`). Reading the actual sets via `--nocapture`
  is what surfaced it - the bare "smoke lists disagree" message alone would have
  led to a half-fix (add one, still red).

Self-reflection: a "disagree" set-equality test earns its keep by NAMING the
mismatch - always read the printed sets, do not assume the filed symptom is the
whole gap.
