# Scenarios tab: collapsible campaign headers + campaign->scenario mapping (replayability)

- STATUS: OPEN
- PRIORITY: 0
- TAGS: backlog,menu,scenario,ui,modding,feature

## Story

As a player, I want the Scenarios tab to present campaigns as collapsible
headers (a "Header" dropdown per campaign that expands to show all its
scenarios), so I can browse a campaign as a unit AND jump straight to any
scenario in it without replaying the whole arc - a replayability win. This
supersedes the interim inline-prefix row style shipped by the campaign-grouping
run (umbrella 20260723-093914, task 20260723-095930).

This also needs a real campaign->scenario MAPPING in the mod/content model so a
campaign's full ordered membership is known (including scenarios that are
`hidden` from the flat picker but should be reachable/listed under their
campaign header for replay).

## Steps

- [ ] Design the campaign->scenario mapping: does a campaign become a
      first-class content entity (a `Campaign` content kind with an ordered
      scenario id list), or is it still derived from per-scenario `campaign`
      metadata plus a way to include hidden members? Weigh against the existing
      `ScenarioCampaign` field and the bundle manifest ordering. Record in a
      DECISION.md / SPIKE.md.
- [ ] Picker UI: collapsible campaign header rows (expand/collapse), listing
      the campaign's scenarios (including replayable hidden ones) in order
      under the header. Replace the interim inline-prefix row style.
- [ ] Decide the replay policy for `hidden`/chained scenarios: which mid-story
      chapters become individually launchable from the header, and how that
      interacts with NextScenario chaining / progression.
- [ ] Harness/example coverage for the collapsible grouped picker.

## Definition of Done

- Scenarios tab shows each campaign as a collapsible header grouping its
  ordered scenarios. (manual: user browses a campaign, expands/collapses it)
- A campaign's membership (including replayable hidden chapters) is known from a
  real mapping, not display-name parsing. (test: mapping resolves campaign
  members in order)
- A player can launch any listed scenario of a campaign directly for replay.
  (manual: user replays a mid-campaign scenario without the earlier ones)

## Notes

- Follow-up from user feedback at the campaign-grouping plan gate (20260723).
- Interim shipped state to supersede: inline "<campaign> <order> - <title>"
  prefix in `spawn_scenario_row` (task 20260723-095930) and the
  `ScenarioCampaign` per-scenario field (task 20260723-095849).
- Related: 20260715-220011 (real per-scenario thumbnail art) shares the picker
  surface.
