# Ledger close-out: lint/audit, version bump 1.6.0, ch2 test update, regenerate catalog, doc sweep

- STATUS: OPEN
- PRIORITY: 40
- TAGS: v0.8.0, content, modding, docs

## Story

Close out the Ledger polish: lint/audit clean, bump the bundle version, update
the ch2 fairness rig deliberately, regenerate the portal catalog, and sync every
doc surface. Owner does the LIVE publish/push and over-the-wire install check
(batched to Finish); this task lands everything up to and including the
regenerated catalog to master.

Umbrella: 20260722-212808. Depends on the pacing/depth/ending/look tasks landing.

## Steps

- [ ] `content lint --target the-ledger` + audit clean across the final tree;
      fix findings or ack intended drama with reasons (use the 20260718-152240
      per-mod report format).
- [ ] Bump `the-ledger.bundle.ron` meta.version - content rework = MINOR bump
      per 20260718-231601 (1.5.0 -> 1.6.0); update the mod CHANGELOG.md with the
      diff-accurate change list (deepened ch3, diverging ch4 endings, pacing
      pass, skybox accents) - write prose from the FINAL diff, not intent.
- [ ] Update `ledger_ch2_encounter.rs` assertions DELIBERATELY for anything the
      pacing pass touched (the version assert is already a range; watch spawn
      geometry stayed put). Confirm `cargo test -p nova_assets --test
      ledger_ch2_encounter` and `--test ledger_ch4_ending` green.
- [ ] Regenerate the portal catalog via `scripts/gen-portal.py`; verify the
      catalog entry + thumbnails for the new version locally (do NOT push live).
- [ ] Doc sweep (keep-docs-in-sync x7): mod README.md, the player-wiki Ledger
      flow page, and the v0.8.0 news-post notes - grep the tree for the old
      chapter/ending description and fix EVERY hit, not a hand-picked subset.

## Definition of Done

- Lint + audit clean (acks with reasons). (cmd: `content lint --target
  the-ledger`.)
- Bundle version bumped to 1.6.0, CHANGELOG updated from the diff. (cmd: grep
  meta.version; CHANGELOG entry present.)
- ledger_ch2_encounter + ledger_ch4_ending tests green with deliberate updates.
  (cmd.)
- Portal catalog regenerated and verified locally (entry + thumbnails present);
  live publish/push left to the owner. (cmd: gen-portal.py clean; manual: owner
  publishes at Finish.)
- README + wiki Ledger flow + v0.8.0 news notes synced. (manual: owner reviews.)

## Notes

Landing scope (GOAL.md): land content + version + regenerated catalog to master;
the owner runs the live portal publish and the native+web over-the-wire install
verification at Finish.
