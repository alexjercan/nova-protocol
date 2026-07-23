# Picker: group + order scenarios by campaign, inline position prefix

- STATUS: OPEN
- PRIORITY: 26
- TAGS: v0.8.0,menu,scenario,ui

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

- [ ] Rework `listed_scenarios` (crates/nova_menu/src/lib.rs ~line 2121): sort
      so campaigned scenarios come first, grouped by campaign name and ordered
      by `order` within a campaign, with uncampaigned scenarios after, still by
      name then id. Deterministic key, e.g.
      `(campaign.is_none() as u8, campaign.name, campaign.order, name, id)`.
      Keep the doc comment truthful.
- [ ] Update `spawn_scenario_row` (~line 2193): when a scenario has a campaign,
      render the name row as `"{name} {order} - {title}"` (inline prefix, e.g.
      "Nova Protocol 1 - Shakedown Run"); uncampaigned rows unchanged. Factor
      the label into a small helper so a test can assert on it directly.
- [ ] Verify the New Game "first LISTED scenario" fallback and
      selection-repair (refresh_scenarios_list) remain deterministic under the
      new order; update the existing fallback tests' expectations/fixtures if
      the first-listed identity legitimately changes, and comment why.
- [ ] Tests in nova_menu: (a) a grouping/order test with fixtures proving three
      Nova Protocol scenarios list in 1-2-3 order (NOT alphabetical) and ahead
      of an uncampaigned scenario; (b) a row-label test asserting the inline
      prefix helper yields "Nova Protocol 1 - <title>" for a campaigned
      scenario and the bare title for an uncampaigned one.

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
