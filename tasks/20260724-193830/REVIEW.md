# Review: Campaign content entity - first-class ordered scenario mapping

- TASK: 20260724-193830
- BRANCH: feature/campaign-content-entity

## Round 1

- VERDICT: REQUEST_CHANGES
- REVIEWER: out-of-context (findings), in-session (merge + re-verify)

Proofs run by the out-of-context reviewer, independently re-derived in-session:
`cargo test -p nova_scenario --lib campaign` (4 pass), `cargo test -p nova_assets`
(full suite green incl. the two parity tests + `merged_campaign_resolves_members_in_order_including_hidden`),
`grep -rn 'ScenarioCampaign' --include='*.rs' crates/ src/` (empty),
`content -- gen` leaves a clean tree, `cargo fmt --all --check` clean. Merge/
overlay/dup-guard for campaigns mirrors sections/scenarios; every `Content` match
site is wired (no catch-all, so the passing build proves completeness);
mod_refs is generic serde-json so needs no arm; the nova_menu revert legitimately
drops interim grouping coverage (moved to the sibling UI task) while retaining a
flat-baseline render test; the dangling-member lint test asserts `errors.len()==1`
so it fails red if `lint_campaign` is a no-op. DECISION.md records both forks.

In-session re-verification of the load-bearing doc claim: confirmed
`web/src/wiki/dev/modding-ron.md:13-15` enumerates only `Scenario`/`Section`
content kinds and omits the new `Campaign` kind - the finding is real.

- [x] R1.1 (MAJOR) web/src/wiki/dev/modding-ron.md:13-15 - the content-model doc
  enumerates authorable content kinds as `Scenario((...))` or `Section((...))`
  but this task adds a third, `Campaign((...))`; a mod author has no way to learn
  campaigns are declarable content or their shape. Extend the doc to include
  `Campaign((id, name, scenarios: [...]))`, noting it is an ordered member-
  scenario mapping (hidden members allowed) registering into `GameCampaigns`,
  mirroring how the doc mentions `GameScenarios`. This CONTENT-MODEL doc for the
  new kind belongs to this task (the sibling UI task owns only the player-facing
  picker docs).
  - Response: Fixed. `modding-ron.md` now lists `Campaign((...))` as the third
    content kind plus a bullet describing the ordered mapping, hidden-member
    replay, `GameCampaigns` registration, and the lint, with a worked example.
    Swept the rest of the content-model doc surface too: `guide-make-a-mod.md`
    content-kind enumeration gained `Campaign((..))`, and `scenario-system.md`
    now documents `CampaignConfig` + the `GameCampaigns` registry beside
    `GameScenarios`. No stale `ScenarioCampaign` mentions existed in the docs
    (the interim field was never wiki-documented).

## Round 2

- VERDICT: APPROVE
- REVIEWER: in-session (round-1 finding was a doc gap; the fix is a doc-surface
  addition verified directly against the diff)

Verified R1.1 resolved: `modding-ron.md`, `guide-make-a-mod.md` and
`scenario-system.md` now document the `Campaign` content kind and the
`GameCampaigns` registry; the worked example matches the generated base
`campaigns/nova_protocol.content.ron`. `grep -rniE 'ScenarioCampaign' web/`
returns nothing. No code changed in this round, so the round-1 code proofs
(all green) still hold. No new findings. Verdict: APPROVE.
