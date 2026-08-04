# The Ledger campaign: collapsible header + hidden-chapter replay

- PRIORITY: 60
- TAGS: v0.9.0, scenario, modding, content, feature
- ACTIVITY: COMPOUNDING
- GATES: PLAN REVIEW RETRO
- RESOLUTION: DONE

## Story

As a player with "The Ledger" webmod enabled, I want its chapters grouped under a
collapsible campaign header in the Scenarios tab - exactly like the base "Nova
Protocol" campaign - so I can browse the arc as a unit and replay any chapter,
including the hidden mid-story ones, directly.

Follow-up to umbrella 20260724-193016 (which built the Campaign content entity +
collapsible picker but scoped the webmod out). This applies the same treatment to
the `the-ledger` webmod.

## Steps

- [x] Author `webmods/the-ledger/ledger_campaign.content.ron`: a single
      `Campaign((id: "the_ledger", name: "The Ledger", scenarios: [...]))` listing
      all six chapter scenario ids in play order (ch1, ch2, ch2b, ch3, ch4, ch5).
- [x] Add the campaign file to `the-ledger.bundle.ron`'s `content` list.
- [x] Restore `ledger_ch5_the_raid` to `hidden: true` (undo the "temporarily
      visible so the finale can be launched directly" hack - the campaign header
      is now the proper mechanism; update the comment).
- [x] Strip the redundant baked-in "The Ledger N:" prefixes from the six
      chapters' `name` fields so rows read cleanly under the "The Ledger" header
      (matching Nova Protocol's bare chapter names). Confirmed the full names are
      referenced nowhere outside the webmod.

## Definition of Done

- With the-ledger enabled, the Scenarios tab shows a "The Ledger" collapsible
  header over its six chapters in order, hidden ones included. (manual: user
  enables the webmod, browses/expands it, replays a hidden chapter)
- The campaign's membership resolves and every member id is a real scenario.
  (test: a nova_assets check resolves the the_ledger campaign in order incl.
  hidden; cmd: `content -- lint` reports 0 findings, incl. the new campaign)
- The webmod still loads recursively. (cmd: `cargo test -p nova_assets --test
  webmods_validation`)

Overall: `cargo check` + `cargo fmt --check` clean, content lint 0 findings,
webmod-load + campaign-resolution tests green.

## Notes

- No engine code - the Campaign content kind + GameCampaigns + collapsible picker
  already shipped (umbrella 20260724-193016). This is content authoring in the
  webmod plus the ch5 visibility fix + name cleanup.
- Ledger chapters (from the bundle): ch1 `ledger_ch1_dead_weight` (visible entry),
  ch2 `ledger_ch2_claim_jumpers` (hidden), ch2b `ledger_ch2b_the_heavies`
  (hidden), ch3 `ledger_ch3_quiet_channel` (hidden), ch4 `ledger_ch4_the_buyer`
  (hidden), ch5 `ledger_ch5_the_raid` (currently temp-visible -> hidden).

## Close-out (what changed and why)

Content-only (no engine code - the Campaign kind/picker shipped in umbrella
20260724-193016):

- Authored `webmods/the-ledger/ledger_campaign.content.ron`: a `Campaign((id:
  "the_ledger", name: "The Ledger", scenarios: [ch1, ch2, ch2b, ch3, ch4, ch5]))`
  in play order; added it to `the-ledger.bundle.ron`.
- Re-hid the finale: `ledger_ch5_the_raid` `hidden: false -> true`, retiring the
  explicit temporary-visible hack whose own comment said "RE-HIDE before release
  - reached only by winning the ch4 fight". The campaign header now provides the
  direct-replay path the hack existed for. Re-pinned the invariant: the old
  `the_raid_is_launchable_for_testing` (`!hidden`) became
  `the_raid_is_hidden_reached_by_playing_or_the_campaign_header` (`hidden`).
- Stripped the redundant baked-in "The Ledger N:" prefixes from the six chapter
  `name` fields (they read redundantly under a "The Ledger" header; Nova Protocol
  uses bare chapter names). Confirmed the full names are referenced nowhere
  outside the webmod (0 hits in docs/news/tests).
- Bumped the mod version `1.12.0 -> 1.13.0` (a portal content change) and updated
  its version-pin test + the `guide-make-a-mod` version-history line; CHANGELOG
  Unreleased gained a Modding entry.

Verification: new `webmods_validation::the_ledger_campaign_lists_its_chapters_in_order`
asserts the 6 members resolve in order with correct hidden flags (fails red if a
member is missing, mis-ordered, or ch5 is un-hidden); `every_webmods_bundle_loads_recursively`
loads the new campaign file; `content -- lint` 0 findings (the_ledger campaign
members all resolve); all Ledger harness tests (ch2/ch3/ch4/ch5) green; version
pin green; `cargo fmt --check` + `cargo check --all-targets` clean.

Left as-is: the news posts (0.7.0/0.8.0) are dated history and describe The
Ledger as it shipped then - not rewritten. Re-publishing to the portal
(`scripts/gen-portal.py`) is a release-time action, not part of this task.

Open manual DoD (batched): user enables The Ledger webmod, browses/expands its
campaign header, and replays a hidden chapter (e.g. The Raid) from it.
