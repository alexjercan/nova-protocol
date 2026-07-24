# Review: Collapsible campaign-header picker + per-chapter replay

- TASK: 20260723-095951
- BRANCH: feature/collapsible-campaign-picker

## Round 1

- VERDICT: APPROVE
- REVIEWER: out-of-context (findings), in-session (re-verification)

The out-of-context reviewer ran every DoD proof and found no defects:
`cargo test -p nova_menu --lib` (70 pass incl. the 3 new collapsible tests + the
untouched New Game / selection / flat-baseline tests), `cargo fmt --all --check`
clean, and `grep -rnE '\{campaign\} \{order\}|Nova Protocol 1 - ' crates/` shows
no live-code hits (only a doc comment noting the interim prefix's removal).

Correctness verified against the code: `CollapsedCampaigns` (absent = expanded)
toggled by `on_campaign_header_toggle`, re-arming the refresh via
`scenarios_list_dirty` (now watching campaigns + collapse state);
`listed_scenarios` unchanged so the New Game fallback + default pick stay
deterministic; selection-repair keeps a selected hidden member via
`selectable_scenario_ids`; `ordered_campaigns` sorts by (name,id) and member
order comes from the campaign Vec, so no HashMap order leaks; a dangling member
id is skipped (no panic); the hidden-launch test has a real `On<LoadScenario>`
delivery guard asserting `LoadedScenario == Some("chap2")`.

In-session re-verification of load-bearing claims: independently re-ran the
nova_menu lib suite (70 green) and confirmed the DECISION's cold-launch risk is
retired - `broadside_gunship` and `final_tally` each `spawn(player_ship())` in
their OnStart, so a cold replay spawns a player and does not depend on carried
state. No member had to be dropped; nothing needed surfacing to the user.

No BLOCKER/MAJOR/MINOR/NIT findings.

Pending user checks (manual DoD items, batched to the flow Finish - APPROVE does
not resolve them):
- user browses a campaign in the Scenarios tab and expands/collapses it;
- user replays a hidden mid-campaign chapter (e.g. the Broadside gunship phase or
  Final Tally) directly from its campaign header without the earlier chapters.
