# Campaign metadata on ScenarioConfig (serde data model)

- STATUS: OPEN
- PRIORITY: 30
- TAGS: v0.8.0,scenario,feature

## Story

As a scenario/mod author, I want to declare that a scenario belongs to a named
campaign at a given position, so the picker and future tooling can group and
order scenarios by campaign instead of relying on alphabetical display-name
sorting or naming conventions baked into strings.

This is the data-model foundation for the campaign-grouped picker (umbrella
20260723-093914). No UI or content changes here - just the serde-driven field
on `ScenarioConfig` plus tests.

## Steps

- [ ] Add a small serde struct `ScenarioCampaign { name: String, order: u32 }`
      in `crates/nova_scenario/src/loader.rs` (feature = "serde" derives,
      matching the file's existing gating style). `name` is the campaign
      DISPLAY name (e.g. "Nova Protocol"); `order` is the 1-based position.
- [ ] Add `campaign: Option<ScenarioCampaign>` to `ScenarioConfig`,
      serde-defaulted with `skip_serializing_if = "Option::is_none"` so
      pre-existing scenarios and mods parse unchanged and clean scenarios omit
      it (mirror the `thumbnail` field's exact attribute treatment).
- [ ] Export `ScenarioCampaign` through the crate prelude alongside
      `ScenarioConfig` (grep the prelude re-exports).
- [ ] Write a DECISION.md recording the shape choice: nested `Option<struct>`
      (atomic membership - you can't set an order without a campaign) vs two
      flat `Option` fields. Chosen: nested struct.
- [ ] Tests in `nova_scenario`: (a) a scenario RON WITH
      `campaign: Some((name: "Nova Protocol", order: 1))` parses and
      round-trips; (b) a scenario RON WITHOUT the field parses to
      `campaign: None`; (c) serializing a `None` campaign omits the key.

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
