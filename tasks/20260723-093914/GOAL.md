# Goal: campaign-grouped, ordered scenario picker

- DATE: 20260723
- UMBRELLA TASK: 20260723-093914
- LANDING SCOPE: squash-merge each task to local `master` via `sprout land`; no push (user's call at Finish).

## Goal

The Scenarios picker (`nova_menu`) currently lists every visible scenario as a
flat, alphabetically-sorted list (`listed_scenarios` sorts by `name` then
`id`). That scrambles the base storyline: the three Ledger chapter-heads show
as "Broadside" (ch2), "Lifeline" (ch3), "Shakedown Run" (ch1) in that order,
with the standalone "Asteroid Field" mixed in. A player cannot tell which
scenario starts the story or what order the chapters run.

This run adds first-class CAMPAIGN metadata to a scenario and teaches the
picker to GROUP scenarios by campaign and ORDER them within a campaign, so the
base storyline chapter-heads read "Shakedown Run" (1), "Broadside" (2),
"Lifeline" (3) contiguously in order, each row carrying an inline campaign
prefix (e.g. "Nova Protocol 1 - Shakedown Run"). Uncampaigned scenarios
(Asteroid Field, mod scenarios that opt out) still list, grouped separately in
a stable order below the campaigns. The metadata is serde-driven so mods can
tag their own campaigns.

### Decisions (from the plan gate, 20260723)

- Base storyline campaign name is "Nova Protocol" (kept distinct from the
  separate installable "The Ledger" webmod).
- Row display for now is an INLINE PREFIX ("Nova Protocol 1 - <title>"), no
  section header. A richer collapsible campaign-header/dropdown UI + a
  campaign->scenario mapping in mods (for replaying any scenario in a campaign
  without the whole arc) is deferred to a filed follow-up backlog task.
- Scope: tag the base storyline only (shakedown/broadside/lifeline). The webmod
  "The Ledger" is out of scope this run (its ch2-4 are hidden anyway).

## Done means

1. `ScenarioConfig` carries optional campaign metadata (a campaign key/name and
   an intra-campaign order index), serde-defaulted so pre-existing scenarios
   and mods still parse unchanged. (test: nova_scenario loader roundtrip/default test; cmd: `cargo test -p nova_scenario`)
2. The base builders tag the three Ledger chapter-heads with campaign "Ledger"
   and order 1/2/3 (shakedown_run=1, broadside=2, lifeline=3); the generated
   `*.content.ron` regenerate to match and the parity test passes. (cmd: `cargo run -p nova_assets --bin content -- gen` leaves a clean tree; cmd: `cargo test -p nova_assets`)
3. The picker groups visible scenarios by campaign and orders within a campaign
   by the order index (campaign members contiguous and in order); uncampaigned
   scenarios group separately in a stable order below. (test: nova_menu grouping/order test proving Nova Protocol appears 1-2-3, not alphabetical)
4. Each campaigned row carries an inline campaign prefix (e.g. "Nova Protocol 1
   - Shakedown Run") so a player sees "first scenario of the campaign" at a
   glance. (test: nova_menu row-label test; manual: user sees the grouped, prefixed picker)
5. The New Game fallback chain and selection-repair still work against the new
   grouped order (first LISTED scenario is well-defined and deterministic). (cmd: `cargo test -p nova_menu`)

Overall: `cargo check` clean, `cargo fmt --check` clean, all newly written/touched tests pass, generated content parity holds, and the picker visibly reads as grouped + ordered.

## Tasks

Updated as tasks land (one line per land).

- [x] 20260723-095849 (p30, nova_scenario) Campaign metadata on ScenarioConfig (serde data model)
      landed 3ca3f4c7; 1 review round (out-of-context APPROVE, no findings); added ScenarioCampaign + campaign field, 6 exhaustive literals fixed
- [ ] 20260723-095909 (p28, nova_assets) Tag base storyline chapter-heads as Nova Protocol 1/2/3 + regen content  [depends: 095849]
- [ ] 20260723-095930 (p26, nova_menu) Picker: group + order by campaign, inline position prefix  [depends: 095849, 095909]

Deferred follow-up (filed, not part of this run):
- 20260723-095951 (backlog) Scenarios tab: collapsible campaign headers + campaign->scenario mapping (replayability)

## Decisions (load-bearing, architectural)

- 20260723-095849 DECISION.md: campaign metadata as one Option<ScenarioCampaign> (nested struct, atomic membership) over two loose Option fields (ACCEPTED)

## Manual acceptance (batched for the user at Finish)

- (pending) task C: user opens the Scenarios picker and confirms the Ledger
  chapters appear grouped and in 1-2-3 order with legible position markers, and
  standalone scenarios read cleanly.
