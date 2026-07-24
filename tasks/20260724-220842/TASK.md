# The Ledger campaign: collapsible header + hidden-chapter replay

- STATUS: OPEN
- PRIORITY: 60
- TAGS: v0.9.0,scenario,modding,content,feature

## Story

As a player with "The Ledger" webmod enabled, I want its chapters grouped under a
collapsible campaign header in the Scenarios tab - exactly like the base "Nova
Protocol" campaign - so I can browse the arc as a unit and replay any chapter,
including the hidden mid-story ones, directly.

Follow-up to umbrella 20260724-193016 (which built the Campaign content entity +
collapsible picker but scoped the webmod out). This applies the same treatment to
the `the-ledger` webmod.

## Steps

- [ ] Author `webmods/the-ledger/ledger_campaign.content.ron`: a single
      `Campaign((id: "the_ledger", name: "The Ledger", scenarios: [...]))` listing
      all six chapter scenario ids in play order (ch1, ch2, ch2b, ch3, ch4, ch5).
- [ ] Add the campaign file to `the-ledger.bundle.ron`'s `content` list.
- [ ] Restore `ledger_ch5_the_raid` to `hidden: true` (undo the "temporarily
      visible so the finale can be launched directly" hack - the campaign header
      is now the proper mechanism; update the comment).
- [ ] Strip the redundant baked-in "The Ledger N:" prefixes from the six
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
