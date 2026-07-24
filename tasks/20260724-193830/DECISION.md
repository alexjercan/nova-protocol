# DECISION: campaign->scenario mapping representation + hidden-replay policy

- DATE: 20260724
- TASK: 20260724-193830 (data model) / umbrella 20260724-193016
- STATUS: ACCEPTED

## Context

The interim campaign-grouping run (umbrella 20260723-093914) shipped campaign
membership as a per-scenario field `ScenarioCampaign { name: String, order: u32 }`
on `ScenarioConfig`, and the picker reconstructed grouping/order by sorting on
that field and rendered an inline prefix ("Nova Protocol 1 - Shakedown Run").
That was explicitly the interim style. This run needs a REAL campaign->scenario
mapping so a campaign's full ordered membership - including `hidden` chapters
reachable only via `NextScenario` chaining - is first-class and known, to drive
a collapsible campaign-header picker with per-chapter replay.

## Fork 1: mapping representation (mutually exclusive - single source of truth)

Whichever structure holds the ordered membership becomes the single source of
truth; keeping both a first-class entity and per-scenario order allows two lists
to silently disagree, so this is an either/or.

- Option A - first-class `Content::Campaign` entity: a `CampaignConfig { id,
  name, scenarios: Vec<ScenarioId> }` content kind loaded into a `GameCampaigns`
  resource. Membership + order are explicit (the Vec), campaign name lives once,
  hidden members are listed explicitly, mods declare a campaign as a unit.
- Option B - keep per-scenario `ScenarioCampaign`, extend it: tag hidden
  mid-campaign scenarios with a campaign + a "list-under-header-for-replay" flag;
  the picker DERIVES membership by grouping scenarios that share a campaign name.
  Less new code, but the mapping stays emergent/derived and order stays a fragile
  per-scenario `u32`.

### Decision: Option A - first-class `Content::Campaign` entity.

The task's own DoD asks for membership "known from a real mapping, not
display-name parsing"; Option A delivers exactly that. Explicit Vec ordering
removes the collision/gap fragility of a per-scenario `u32`, a campaign carries
its display name once, and hidden members are unambiguous list entries. The
interim per-scenario `ScenarioCampaign` field is RETIRED (struct, field, and
prelude export removed; builders/RON/exhaustive literals fixed) so there is one
source of truth. More machinery (new content kind, merge router, `GameCampaigns`
resource, parity, lint) is accepted as the correct/maintainable shape.

Rejected within A: keeping a per-scenario back-pointer (`campaign_id` on
`ScenarioConfig`) alongside the entity. Not needed for this run - the picker
reads `GameCampaigns` directly - and it reintroduces a second place order/
membership could be inferred. If a scenario later needs to know its own campaign
cheaply, derive a reverse index from `GameCampaigns` at load time rather than
storing a back-pointer on the scenario.

## Fork 2: which hidden members are individually launchable

- Chosen: ALL hidden chained members are launchable campaign members. The base
  "Nova Protocol" campaign lists all five in narrative order:
  `shakedown_run`, `broadside`, `broadside_gunship`, `lifeline`, `final_tally`.
- Rejected: (a) only the hidden epilogue `final_tally` launchable; (b) visible
  chapters only (would fail DoD item 4).

### Risk carried into the UI task (20260723-095951)

`broadside_gunship` is a mid-battle phase-2 continuation and `final_tally` an
epilogue; both are `hidden` and chained via `NextScenario`, so they may depend on
world state carried from their predecessor. Launching them cold for replay may
play wrong. The UI task VERIFIES standalone cold-launch playability via a
harness/probe run; if a member cannot replay cold, that task STOPS and surfaces
it (adjust membership or file a follow-up) rather than shipping a broken replay
entry. The content model here does not itself guarantee standalone playability -
it only makes the members reachable.

## Consequences

- New content kind `Content::Campaign(CampaignConfig)` (nova_modding), config
  type in nova_scenario, `GameCampaigns` resource (nova_assets registration),
  base "Nova Protocol" campaign builder + generated RON + `base.bundle.ron` entry,
  and a lint check for dangling campaign member ids.
- `ScenarioConfig::campaign` / `ScenarioCampaign` removed; `nova_menu` reverts to
  a flat list in this task and the sibling UI task rebuilds grouping on
  `GameCampaigns`.
