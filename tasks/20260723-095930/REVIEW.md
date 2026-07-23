# Review: Picker - group + order scenarios by campaign, inline position prefix

- TASK: 20260723-095930
- BRANCH: feature/picker-campaign-grouping

## Round 1

- VERDICT: APPROVE
- REVIEWER: out-of-context

No BLOCKER / MAJOR / MINOR findings. One NIT (below), fixed.

Verified independently (out-of-context reviewer; matches the in-session runs):
`cargo test -p nova_menu` = 67 passed / 0 failed (3 new tests + the two named
fallback tests green); `cargo fmt --check` clean; the diff touches only
`crates/nova_menu/src/lib.rs` and `tasks/.../TASK.md` - the temporary
`screenshot_ui` edit was reverted (empty diff there), no leftover debug code.

Correctness confirmed by reasoning: the sort key
`(has_campaign?false:true, campaign_name, order, name, id)` is total and
deterministic (campaigned first; `None` placeholders `("",0)` never collide
with real data because `has_campaign=true` sorts them last; id breaks ties).
`scenario_row_label` yields exactly "Nova Protocol 1 - Shakedown Run" for a
member and the bare name otherwise, and is reused for both the row and the
details-pane title. Test quality confirmed: the ordering test genuinely fails
under the old alphabetical sort (fixture names sort differently from campaign
order), and `picker_rows_render_campaign_grouped_and_prefixed` reads the actual
spawned row Text in child order (a real ECS render assertion), asserting the
hidden backdrop is excluded. No regressions: the fallback fixtures are
uncampaigned, so first-listed identity is unchanged; no tests weakened.

Pending user check (NOT resolved by APPROVE): the DoD `manual:` item - user
visually confirms the rendered picker shows Shakedown/Broadside/Lifeline grouped
and 1-2-3 prefixed with Asteroid Field below. Batched to the flow Finish. The
close-out honestly documents the abandoned pixel screenshot (llvmpipe
autopilot-window limit) and the reverted temp edit, both confirmed.

- [x] R1.1 (NIT) tasks/20260723-095930/TASK.md close-out - test-count miscount
  ("66 pre-existing + 3 new"; the suite is 67 total = 64 pre-existing + 3 new).
  - Response: fixed - corrected to "67 green total (64 pre-existing + 3 new)".
