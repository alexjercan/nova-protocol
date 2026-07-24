# Scenarios tab: collapsible campaign headers + campaign->scenario mapping (replayability)

- STATUS: OPEN
- PRIORITY: 64
- TAGS: v0.9.0, menu, scenario, ui, modding, feature


## Story

As a player, I want the Scenarios tab to present campaigns as collapsible
headers (a header row per campaign that expands to show all its scenarios), so I
can browse a campaign as a unit AND jump straight to any scenario in it -
including hidden mid-story chapters - without replaying the whole arc. This
supersedes the interim inline-prefix row style shipped by the campaign-grouping
run (umbrella 20260723-093914, task 20260723-095930).

The campaign->scenario mapping this reads is the first-class `GameCampaigns`
resource built by the dependency task 20260724-193830 (a campaign owns its
ordered member list, hidden ids included). This task is the picker UI + replay
launch + harness coverage on top of that mapping.

## Steps

- [ ] Collapsible campaign header rows: read `GameCampaigns`, render one header
      row per campaign (a `ThemedButton`-based header carrying an
      expand/collapse state component), and under an expanded header list the
      campaign's member scenarios in declared order - INCLUDING hidden
      replayable members. Clicking the header toggles expand/collapse (reuse the
      despawn/respawn refresh pattern). Replace the interim inline-prefix / flat
      `scenario_row_label` style.
- [ ] Uncampaigned scenarios still list, grouped separately below the campaigns
      in a stable order (as today).
- [ ] Replay launch: any listed member (including a hidden one) is directly
      launchable via the existing row-select -> details pane -> Play path.
      Verify `SelectedScenarioId`, the details pane, and Play all resolve for a
      hidden member.
- [ ] Verify standalone playability of the hidden members (broadside_gunship,
      final_tally) launched cold via a harness/probe run. A hidden scenario
      chained via `NextScenario` may depend on carried world state; if one
      cannot replay cold, STOP and surface it to the user (file a follow-up or
      adjust membership) rather than shipping a broken replay entry.
- [ ] New Game first-LISTED-scenario fallback and selection-repair
      (`refresh_scenarios_list`) stay deterministic under the collapsible model
      (a well-defined first launchable scenario).
- [ ] Harness/example coverage: a `nova_menu` widget-tree test asserting the
      spawned header + expand/collapse + member ordering (per the
      `widget-tree-eyeball-for-logical-layout` lesson - assert the spawned Node
      tree, not a pixel screenshot), plus a test that a hidden member is
      launchable. Consider a small `examples/ui` demo if cheap.
- [ ] Docs sync in the SAME task: wiki `web/src/wiki/dev/modding-ron.md` (new
      Campaign content kind + the `scenarios` list), `scenario-system.md`
      (campaign mapping + hidden-replay), the `keeping-docs-in-sync` map, and
      any player-facing scenarios/picker doc; CHANGELOG + news entry.

## Definition of Done

- The Scenarios tab shows each campaign as a collapsible header grouping its
  ordered scenarios; expand and collapse both work. (test: nova_menu widget-tree
  test of header + expand/collapse + order; manual: user browses a campaign,
  expands and collapses it)
- A player can launch any listed scenario of a campaign directly for replay,
  including a hidden mid-campaign chapter. (test: launch path resolves a hidden
  member; manual: user replays a mid-campaign scenario without the earlier ones)
- The New Game fallback chain and selection-repair remain deterministic under
  the new grouped/collapsible order. (cmd: `cargo test -p nova_menu`)
- The interim inline-prefix row style is removed. (cmd: `grep -rn` shows the old
  `"{campaign} {order} - {title}"` label format is gone)
- Every doc surface the change invalidates is updated in this task.
  (cmd: doc-tree grep sweep clean)

Overall: `cargo check` clean, `cargo fmt --check` clean, newly written/touched
tests pass, and the picker visibly reads as collapsible campaign groups.

## Notes

- DEPENDS ON: 20260724-193830 (first-class Campaign content entity /
  `GameCampaigns`). Do not start until that has landed.
- Follow-up from user feedback at the campaign-grouping plan gate (20260723).
- Interim shipped state to supersede: inline "<campaign> <order> - <title>"
  prefix in `spawn_scenario_row` / `scenario_row_label` (task 20260723-095930).
  The per-scenario `ScenarioCampaign` field (task 20260723-095849) is retired by
  the dependency task, not here.
- Picker wiring reference (from planning exploration):
  `crates/nova_menu/src/lib.rs` - `listed_scenarios` (~2139), `spawn_scenario_row`
  (~2242), `scenario_row_label` (~2234), `refresh_scenarios_list` (~2184),
  `on_scenario_row_select` (~2859), `refresh_scenario_details` (~2294). No
  collapsible widget exists; build from `ThemedButton` + `panel_header` +
  despawn/respawn. `hidden` scenarios are filtered by `listed_scenarios` today.
- Related: 20260715-220011 (real per-scenario thumbnail art) shares the picker
  surface.
