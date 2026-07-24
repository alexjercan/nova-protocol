# Campaign content entity: first-class ordered scenario mapping

- STATUS: OPEN
- PRIORITY: 66
- TAGS: v0.9.0,scenario,modding,content,feature

## Story

As a scenario/mod author and as the picker, I want a campaign to be a
first-class content entity that owns its ordered list of member scenario ids
(including `hidden` ones), so a campaign's full membership and order are known
from a real mapping instead of being reconstructed from per-scenario display
metadata. This is the data-model foundation for the collapsible campaign-header
picker (umbrella 20260724-193016, sibling UI task 20260723-095951).

Decision (from the plan gate, 20260724, recorded in DECISION.md): the campaign
becomes a first-class `Content::Campaign` entity that is the SINGLE SOURCE OF
TRUTH for membership + order; the interim per-scenario `ScenarioCampaign { name,
order }` field is RETIRED (keeping both would allow two lists to diverge). All
hidden chained members are launchable, so the base "Nova Protocol" campaign
lists all five in narrative order: shakedown_run, broadside, broadside_gunship,
lifeline, final_tally.

No picker UI work here beyond keeping `nova_menu` compiling: this task removes
the retired field and reverts the picker to a flat list; the sibling UI task
20260723-095951 builds the collapsible header UI that reads `GameCampaigns`.

## Steps

- [ ] Add `CampaignConfig { id: String, name: String, scenarios: Vec<String> }`
      in `crates/nova_scenario/src/loader.rs` (serde derives gated like
      `ScenarioConfig`). `id` is the stable campaign key; `name` is the display
      name; `scenarios` is the ORDERED member scenario-id list (may include
      hidden ids). Export through the crate prelude.
- [ ] Add `Content::Campaign(CampaignConfig)` variant to the `Content` enum in
      `crates/nova_modding/src/lib.rs`. Route it through `merge_bundles()` +
      `merge_content_item()` in `crates/nova_assets/src/lib.rs` (with a
      `seen_campaigns` duplicate guard), and register a `GameCampaigns` resource
      (keyed by campaign id, PRESERVING declared member order) in
      `register_bundles()`.
- [ ] Retire the interim per-scenario campaign metadata: remove the `campaign:
      Option<ScenarioCampaign>` field from `ScenarioConfig`, delete the
      `ScenarioCampaign` struct and its prelude export, and fix every reference
      - the builders (shakedown/broadside/lifeline), the generated
      `*.content.ron`, and any exhaustive `ScenarioConfig { .. }` literals.
      Revert `nova_menu`'s `listed_scenarios` sort + `scenario_row_label` inline
      prefix to the pre-campaign flat behavior so the crate compiles (the
      sibling task rebuilds the UI on `GameCampaigns`).
- [ ] Add the base "Nova Protocol" campaign builder (order: shakedown_run,
      broadside, broadside_gunship, lifeline, final_tally) to the content
      generators; wire its generated `*.content.ron` into `base.bundle.ron`;
      regenerate and confirm the parity test passes.
- [ ] Add a content-lint check (in the existing lint): every scenario id listed
      in a campaign's `scenarios` must resolve to a loaded scenario (dangling
      campaign-member ref), mirroring the existing dangling-target lint pattern.
- [ ] Write DECISION.md recording: first-class Campaign entity as single source
      of truth (ScenarioCampaign retired), all-hidden-members-launchable policy,
      and the ordered `scenarios` list shape (vs a per-scenario back-pointer).
- [ ] Tests: (a) a campaign RON with `scenarios: [...]` parses and round-trips
      (nova_scenario or nova_assets); (b) `GameCampaigns` resolves the Nova
      Protocol members in declared order INCLUDING the hidden ones
      (broadside_gunship, final_tally); (c) the lint flags a campaign that
      references an unknown scenario id (fails red first).

## Definition of Done

- `CampaignConfig` is a serde-driven content kind; a campaign RON parses and
  round-trips. (test: `cargo test -p nova_scenario`; cmd: `cargo test -p nova_assets`)
- `GameCampaigns` resolves a campaign's ordered membership, including hidden
  members, from the real mapping (not display-name parsing). (test: resolves
  Nova Protocol in declared order with hidden ids present)
- The base "Nova Protocol" campaign generates and content parity holds.
  (cmd: `cargo run -p nova_assets --bin content -- gen` leaves a clean tree;
  cmd: `cargo test -p nova_assets`)
- The interim per-scenario `ScenarioCampaign` field is fully removed; no live
  references remain. (cmd: `grep -rn 'ScenarioCampaign' --include='*.rs'
  crates/ src/` returns nothing outside `tasks/`; `cargo check` clean)
- The lint flags a campaign referencing an unknown scenario id. (test)
- DECISION.md present recording the mapping shape + replay policy. (artifact)

Overall: `cargo check` clean, `cargo fmt --check` clean, newly written/touched
tests pass, generated content parity holds.

## Notes

- Depends on: nothing. Blocks: 20260723-095951 (collapsible picker UI reads
  `GameCampaigns`).
- Supersedes the data-model half of the interim campaign-grouping run (umbrella
  20260723-093914, task 20260723-095849 which added `ScenarioCampaign`).
- Content-kind wiring reference (from planning exploration): `Content` enum at
  `crates/nova_modding/src/lib.rs:68`; merge routers at
  `crates/nova_assets/src/lib.rs` `merge_bundles()` (~837) + `merge_content_item()`
  (~893); `register_bundles()` (~544); parity test
  `crates/nova_assets/tests/content_ron_parity.rs`.
- Hidden/chain facts: `broadside_gunship` and `final_tally` are `hidden: true`,
  `campaign: None`, reached via `NextScenario`. Their standalone-replay
  playability is verified in the sibling UI task, not here.
