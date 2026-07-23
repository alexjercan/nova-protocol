# Tag base storyline chapter-heads as Nova Protocol 1/2/3 + regen content

- STATUS: OPEN
- PRIORITY: 28
- TAGS: v0.8.0,scenario,content

## Story

As a player, I want the shipped base storyline to declare itself as one ordered
campaign, so the picker can show Shakedown Run / Broadside / Lifeline as
"Nova Protocol" chapters 1, 2, 3 instead of an alphabetical jumble.

Tags the three VISIBLE base storyline chapter-heads with the new
`campaign` metadata (from task A) in their Rust builders, then regenerates the
base content RON so the generated files and parity tests match.

## Steps

- [ ] In `crates/nova_assets/src/scenario/shakedown.rs` (`shakedown_run`
      builder), set `campaign: Some(ScenarioCampaign { name: "Nova Protocol".into(), order: 1 })`.
- [ ] In `crates/nova_assets/src/scenario/broadside.rs` (`broadside` builder -
      the VISIBLE part-one head, id `broadside`), set campaign Nova Protocol
      order 2. Leave `broadside_gunship` (hidden continuation) untagged.
- [ ] In `crates/nova_assets/src/scenario/lifeline.rs` (`lifeline` builder),
      set campaign Nova Protocol order 3. Leave `final_tally` (hidden
      continuation) untagged.
- [ ] Regenerate base content: `nix develop --command cargo run -p nova_assets
      --bin content -- gen`, then confirm `git status` shows only the three
      expected `*.content.ron` changed and the diff adds the campaign key.
- [ ] Run the content parity + lint tests: `nix develop --command cargo test -p
      nova_assets` (and `content -- lint` stays clean).

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
