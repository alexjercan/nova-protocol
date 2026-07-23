# Tag base storyline chapter-heads as Nova Protocol 1/2/3 + regen content

- STATUS: CLOSED
- PRIORITY: 28
- TAGS: v0.8.0, scenario, content

## Story

As a player, I want the shipped base storyline to declare itself as one ordered
campaign, so the picker can show Shakedown Run / Broadside / Lifeline as
"Nova Protocol" chapters 1, 2, 3 instead of an alphabetical jumble.

Tags the three VISIBLE base storyline chapter-heads with the new
`campaign` metadata (from task A) in their Rust builders, then regenerates the
base content RON so the generated files and parity tests match.

## Steps

- [x] In `crates/nova_assets/src/scenario/shakedown.rs` (`shakedown_run`
      builder), set `campaign: Some(ScenarioCampaign { name: "Nova Protocol".into(), order: 1 })`.
- [x] In `crates/nova_assets/src/scenario/broadside.rs` (`broadside` builder -
      the VISIBLE part-one head, id `broadside`), set campaign Nova Protocol
      order 2. Leave `broadside_gunship` (hidden continuation) untagged.
- [x] In `crates/nova_assets/src/scenario/lifeline.rs` (`lifeline` builder),
      set campaign Nova Protocol order 3. Leave `final_tally` (hidden
      continuation) untagged.
- [x] Regenerate base content: `nix develop --command cargo run -p nova_assets
      --bin content -- gen`, then confirm `git status` shows only the three
      expected `*.content.ron` changed and the diff adds the campaign key.
      (confirmed: exactly shakedown_run/broadside/lifeline .content.ron changed,
      each adds `campaign: Some((name: "Nova Protocol", order: N))`)
- [x] Run the content parity + lint tests: `nix develop --command cargo test -p
      nova_assets` (and `content -- lint` stays clean).
      (content_ron_parity 2/2 PASS; `content -- lint` 0 errors/0 warnings,
      13 scenarios audited; one UNRELATED pre-existing failure in
      content_lint_gate - filed as 20260723-103523, fails identically on master)

## Definition of Done

- shakedown_run/broadside/lifeline builders emit
  `campaign: Some((name: "Nova Protocol", order: 1|2|3))`; the generated RON
  matches. (cmd: `content -- gen` leaves a clean tree after commit)
- Content parity/lint tests pass. (cmd: `nix develop --command cargo test -p nova_assets`)
- Only the three intended scenarios are tagged; hidden continuations
  (broadside_gunship, final_tally, asteroid_next) stay untagged. (manual: diff review)
- `cargo fmt --check` clean.

## Notes

- Depends on: 20260723-095849 (task A - the field must exist first).
- Edit the BUILDER and regenerate in the same commit; never hand-edit generated
  RON (AGENTS.md: generated-content rule).
- Umbrella: 20260723-093914.

## Close-out (20260723)

What changed: tagged the three VISIBLE base storyline chapter-heads with
`campaign: Some(ScenarioCampaign { name: "Nova Protocol", order })` in their
builders - shakedown_run=1, broadside=2, lifeline=3 - and regenerated the base
content RON. The hidden continuations (broadside_gunship, final_tally,
asteroid_next) stay untagged: they are reached only by NextScenario chaining,
never listed in the picker, so campaign membership would be meaningless for
them.

Verification: `cargo run -p nova_assets --bin content -- gen` changed exactly
the three expected `*.content.ron`, each adding the campaign key with the right
order (diff reviewed). `content_ron_parity` integration test passes 2/2
(builder <-> RON parity holds). `content -- lint`: 0 errors, 0 warnings, 13
scenarios balance-audited. `cargo fmt --check` clean.

Difficulty: the full `cargo test -p nova_assets` surfaced ONE failure,
`content_lint_gate::target_mode_lints_one_mod_in_repo_or_external`, asserting a
the-ledger ch4 "mutually exclusive" warn that no longer exists. Per the
merge-red discipline I checked it against master (`git branch --show-current`
= master, ran the test there): it FAILS identically on master, at the same
line 48. So it is INHERITED, not caused by this branch (which does not touch
the-ledger), and is filed as its own task 20260723-103523. This branch's own
guards (parity, lint) are green.

Self-reflection: proactively running the narrow guards (content_ron_parity,
content lint) FIRST, before the full suite, made it obvious the failure was
outside my change's blast radius - the parity guard is the real proof for a
content-regen task, and it was green immediately. Checking master before
blaming the branch (the merge-red lesson) took one extra test run and saved a
false diagnosis.
