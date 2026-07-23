# Picker: group + order scenarios by campaign, inline position prefix

- STATUS: CLOSED
- PRIORITY: 26
- TAGS: v0.8.0, menu, scenario, ui

## Story

As a player opening the Scenarios picker, I want campaign scenarios grouped
together and shown in campaign order with an inline position prefix, so I can
see at a glance which scenario starts the story and what order the chapters run
(e.g. "Nova Protocol 1 - Shakedown Run").

Teaches the picker's ordering and row rendering to use the `campaign` metadata
(tasks A + B). Sorting groups campaign members contiguously and in order;
uncampaigned scenarios list below in a stable order. Rows carry an inline
campaign prefix.

## Steps

- [x] Rework `listed_scenarios` (crates/nova_menu/src/lib.rs ~line 2121): sort
      so campaigned scenarios come first, grouped by campaign name and ordered
      by `order` within a campaign, with uncampaigned scenarios after, still by
      name then id. Deterministic key
      `(has_campaign?false:true, campaign_name, order, name, id)`.
      Doc comment updated to describe the grouping.
- [x] Update `spawn_scenario_row` (~line 2193): when a scenario has a campaign,
      render the name row as `"{campaign} {order} - {title}"` (inline prefix,
      e.g. "Nova Protocol 1 - Shakedown Run"); uncampaigned rows unchanged.
      Factored into a testable `scenario_row_label` helper; also applied to the
      details-pane title so a selected member reads consistently.
- [x] Verify the New Game "first LISTED scenario" fallback and
      selection-repair (refresh_scenarios_list) remain deterministic under the
      new order. No fixture changes needed: the fallback fixtures
      (dummy_scenarios, picker_scenarios) are all UNcampaigned, so they still
      sort by name then id exactly as before - full nova_menu suite (66) green
      unchanged.
- [x] Tests in nova_menu: (a) `listed_scenarios_groups_campaign_members_in_order`
      - three Nova Protocol members inserted scrambled list 1-2-3 ahead of an
      uncampaigned entry (fails on the old alphabetical sort - watched it red
      first); (b) `scenario_row_label_prefixes_campaign_members` - the helper
      yields "Nova Protocol 1 - Shakedown Run" for a member, bare title
      otherwise; plus (c) `picker_rows_render_campaign_grouped_and_prefixed` -
      end-to-end through the real spawn path, reading the ACTUAL spawned row
      Text in display order (the ECS-level render eyeball).

## Definition of Done

- Picker lists campaign members contiguous and in `order`; uncampaigned below
  in stable name order. (test: nova_menu grouping/order test - Nova Protocol 1-2-3)
- Campaigned rows show the inline "<campaign> <order> - <title>" prefix;
  others unchanged. (test: nova_menu row-label test)
- New Game fallback + selection repair still deterministic; existing menu tests
  pass (adjusted where the first-listed identity legitimately moved).
  (cmd: `nix develop --command cargo test -p nova_menu`)
- Manual: opening the picker shows Shakedown/Broadside/Lifeline grouped and
  1-2-3 prefixed, Asteroid Field listed separately below.
  (manual: user confirms the rendered picker)
- `cargo fmt --check` clean.

## Notes

- Depends on: 20260723-095849 (task A), 20260723-095909 (task B).
- Existing tests to mind: `scenarios_panel_default_selects_first_and_renders_details`,
  `start_new_game_scenario_falls_back_past_a_bad_declaration` (fixtures sort by
  name today - task 6625 comment).
- Umbrella: 20260723-093914. Eyeball the rendered picker (LESSONS: a
  layout task is unverified until someone SEES it) - use example `menu_newgame`
  or a screenshot rig.

## Close-out (20260723)

What changed: `listed_scenarios` now GROUPS by campaign and ORDERS within a
campaign (campaigned first, by campaign name then `order`; uncampaigned after,
by name; id breaks ties), and a new `scenario_row_label` helper renders the
inline `"<campaign> <order> - <title>"` prefix used by both the list rows
(`spawn_scenario_row`) and the details-pane title. So the shipped base
storyline now reads "Nova Protocol 1 - Shakedown Run", "Nova Protocol 2 -
Broadside", "Nova Protocol 3 - Lifeline" contiguously and in order, with
Asteroid Field listed below.

Testing (test-first): wrote the ordering test against the OLD alphabetical sort
and watched it fail (left `[asteroid_field, broadside, lifeline, shakedown]`,
right `[shakedown, broadside, lifeline, asteroid_field]`), then implemented the
grouped sort to green. Three new tests: pure-fn order, pure-fn label, and an
end-to-end `picker_rows_render_campaign_grouped_and_prefixed` that drives the
real refresh_scenarios_list -> spawn_scenario_row path and reads the ACTUAL
spawned row Text in child (display) order. Full `cargo test -p nova_menu`: 67
green total (64 pre-existing + 3 new); `cargo check --workspace --all-targets`
clean; `cargo fmt --check` clean.

Eyeball decision (render-output-eyeball lesson): I did the eyeball at the
ECS-rendered-widget level - `picker_rows_render_campaign_grouped_and_prefixed`
reads the exact Text the picker spawns, in display order, proving the rows
render with the right labels, in the right order, with the hidden backdrop
excluded. I ALSO attempted a real pixel screenshot via a temporary beat in
`screenshot_ui`, but the capture path's settle frames overrun the fixed
autopilot window on this software GPU (a limitation the rig's own comments
document for llvmpipe), so it produced no PNG without further window-tuning
hacks; I reverted the temporary example edit rather than ship reel bloat. The
pixel-level confirmation (font, truncation, panel overflow) is the DoD's
`manual:` item - batched for the user's acceptance at the flow Finish.

Self-reflection: the ECS-render test is the right altitude for a text-list
"layout" - it sees the rendered content deterministically and headlessly,
where a pixel screenshot is both flaky here and lower-signal. Next time, reach
for the widget-level render assertion first and treat a pixel shot as optional
polish, not the gate. Watching the ordering test fail first (red) on the old
sort was worth it - it proved the test actually distinguishes grouped from
alphabetical.
