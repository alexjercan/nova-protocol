# Story campaign mod (The Ledger) polish + extension: longer, more interesting narrative arc; re-publish the portal mod

- STATUS: OPEN
- PRIORITY: 48
- TAGS: v0.8.0,content,modding,scenario

## Story

As a player who installed The Ledger from the portal, I want a longer, richer
salvage arc whose chapters look and feel distinct and whose two-ending choice
pays off, so that the flagship portal mod plays like a real campaign and shows
what the modding platform can carry.

The Ledger already ships five scenario files
(`webmods/the-ledger/ledger_ch1..ch4 + ch2b.content.ron`) - a four-chapter
salvage story with comms-driven beats, a checkpointed two-act chapter two, and
a two-ending finale, at bundle version 1.5.0. Make it richer and longer, and
re-publish the portal bundle. Data/scenario + mod-resource work only; no new
engine features.

## Steps

- [ ] Playtest the full Ledger chain; note pacing/narrative/difficulty weak
      spots and any dangling or unclear beats (ch2b checkpoint, the ch4
      branch); write the findings into this task before authoring.
- [ ] Deepen the arc: additional chapter(s) or expanded existing ones,
      stronger narrative beats, more encounter variety, and payoff on the
      two-ending choice (today both endings converge on fighting the Auditor;
      give the choice consequence). Lean on the v0.7.0 authoring stack
      (Outcome frames, allegiance, comms, `scenario_elapsed` timed beats).
- [ ] Give chapters their own look via mod-carried resources (`self://`
      skybox/texture variety landed in v0.7.0; chapters currently reuse base
      art via `dep://base`).
- [ ] Re-run `content lint --target the-ledger` + audit (use the 20260718-152240
      report if it has landed); fix findings or ack intended drama with
      reasons.
- [ ] Bump the mod version following the semantics being written down in
      20260718-231601 (content rework = minor bump) and update the mod's own
      CHANGELOG; check whether `tests/ledger_ch2_encounter.rs` pins the
      version or geometry you changed and update the assertions deliberately,
      not reactively.
- [ ] Re-publish through the portal generator (Rust bin or the new Python
      script 20260718-152247); verify the published catalog entry, thumbnails,
      and an over-the-wire install on native and web.
- [ ] Sync docs surfaces in the same task: CHANGELOG entry, anything the
      player wiki says about the Ledger flow, and notes for the v0.8.0 news
      post.

## Definition of Done

- The arc is at least one chapter (or one act per existing chapter) richer
  than 1.5.0, the two endings visibly diverge, and each chapter has a distinct
  look carried by mod resources.
- Lint + audit are clean (acks only with reasons), the encounter tests pass
  with deliberate updates, and the bundle version is bumped per convention.
- The updated mod installs from Explore online on native and web, and an
  existing install updates in place keeping its enabled state.
- Playtest questions for the owner are listed in this task, not silently
  decided.

## Notes

- Dogfoods the portal pipeline end to end (multi-file bundle), which is also
  the best test of the modding platform.
- `tests/ledger_ch2_encounter.rs` pins spawn ranges, bearing cones, loadout
  discipline and the act-split retry - treat it as the fairness contract for
  ch2; extend the same style to new chapters.
- Feel/balance is ultimately the user's call; deliver content + a first tuning
  pass, flag questions.
