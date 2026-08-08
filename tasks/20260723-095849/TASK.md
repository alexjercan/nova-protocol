# Campaign metadata on ScenarioConfig (serde data model)

- STATUS: CLOSED
- PRIORITY: 30
- TAGS: v0.8.0, scenario, feature

## Story

As a scenario/mod author, I want to declare that a scenario belongs to a named
campaign at a given position, so the picker and future tooling can group and
order scenarios by campaign instead of relying on alphabetical display-name
sorting or naming conventions baked into strings.

This is the data-model foundation for the campaign-grouped picker (umbrella
20260723-093914). No UI or content changes here - just the serde-driven field
on `ScenarioConfig` plus tests.

## Steps

- [x] Add a small serde struct `ScenarioCampaign { name: String, order: u32 }`
      in `crates/nova_scenario/src/loader.rs` (feature = "serde" derives,
      matching the file's existing gating style). `name` is the campaign
      DISPLAY name (e.g. "Nova Protocol"); `order` is the 1-based position.
- [x] Add `campaign: Option<ScenarioCampaign>` to `ScenarioConfig`,
      serde-defaulted with `skip_serializing_if = "Option::is_none"` so
      pre-existing scenarios and mods parse unchanged and clean scenarios omit
      it (mirror the `thumbnail` field's exact attribute treatment).
- [x] Export `ScenarioCampaign` through the crate prelude alongside
      `ScenarioConfig` (loader::prelude re-export).
- [x] Write a DECISION.md recording the shape choice: nested `Option<struct>`
      (atomic membership - you can't set an order without a campaign) vs two
      flat `Option` fields. Chosen: nested struct.
- [x] Tests in `nova_scenario`: (a) a scenario RON WITH
      `campaign: Some((name: "Nova Protocol", order: 1))` parses and
      round-trips; (b) a scenario RON WITHOUT the field parses to
      `campaign: None`; (c) serializing a `None` campaign omits the key.
      (new test `campaign_defaults_when_absent_and_parses_from_authored_ron`)
- [x] Fix the 6 exhaustive `ScenarioConfig { .. }` literals (no
      `..Default::default()`) that a new field breaks, found via
      `cargo check --workspace --all-targets`: nova_scenario/world.rs +
      5 in nova_assets (balance, broadside x2, final_tally, lifeline). All get
      `campaign: None`; task B sets the real values on the base builders.

## Definition of Done

- `ScenarioConfig` carries `campaign: Option<ScenarioCampaign>`, serde-defaulted
  and skip-when-none. (test: nova_scenario loader parse/default/roundtrip test)
- Pre-existing scenario RON (no campaign key) still parses unchanged.
  (cmd: `nix develop --command cargo test -p nova_scenario`)
- `ScenarioCampaign` is reachable through the prelude. (cmd: `nix develop --command cargo check`)
- `cargo fmt --check` clean.

## Notes

- File: `crates/nova_scenario/src/loader.rs` (`ScenarioConfig` at ~line 87;
  copy the `thumbnail`/`hidden` serde attribute pattern verbatim).
- Do NOT hand-edit generated `assets/base/**/*.content.ron`; those regenerate
  in task B from the Rust builders.
- Umbrella: 20260723-093914. This task blocks tasks B and C.

## Close-out (20260723)

What changed: added `ScenarioCampaign { name: String, order: u32 }`
(serde-gated, `PartialEq, Eq`) and `campaign: Option<ScenarioCampaign>` on
`ScenarioConfig`, with the same serde-default + skip-when-none treatment as
`thumbnail`. Exported `ScenarioCampaign` through the loader prelude. Documented
the author-facing RON syntax on both the struct and the field
(`campaign: Some((name: "Nova Protocol", order: 1))`).

Shape decision (DECISION.md): one nested `Option<struct>` over two loose
`Option` fields, so campaign membership is atomic (no order without a campaign)
and there is a single serde-defaulted field to keep the back-compat surface
minimal.

Testing: new `campaign_defaults_when_absent_and_parses_from_authored_ron`
proves (a) a legacy RON with no campaign key parses to `None`, (b) a
HAND-WRITTEN member RON string parses to the right name+order - not a
self-authored round-trip, per the `roundtrip-hides-shared-bug` lesson - and
(c) the field round-trips and is omitted when `None`. Full `cargo test -p
nova_scenario`: 138 lib + 1 integration pass; `cargo fmt --check` clean;
`cargo check --workspace --all-targets` clean.

Difficulties: the only real footgun was `check-all-targets-for-struct-field` -
a plain `cargo check` compiles none of the exhaustive test/builder literals.
`cargo check --workspace --all-targets` surfaced exactly 6 that lack
`..Default::default()`; the rest of the ~50 `ScenarioConfig` literals spread
across examples/tests use `..Default::default()` and needed nothing. Fixed the
6 with `campaign: None`.

Self-reflection: went smoothly. One thing to watch for task C: the New Game
"first LISTED scenario" fallback tests currently rely on name-sort order; once
campaign grouping changes the sort, those fixtures may need updating (already
flagged in task C's Notes).
