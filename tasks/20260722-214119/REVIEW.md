# REVIEW - 20260722-214119 Ledger close-out

Out-of-context adversarial review. I was not the implementer. Reviewed
`git diff master...HEAD`, cross-checked every doc claim against the actual
content RON, and re-ran lint / the ledger test suite / the catalog regen.

## Round 1

### Verification results (all re-run on this branch)

- **Version bump**: `the-ledger.bundle.ron` meta.version `1.5.0 -> 1.6.0`.
  Content rework = MINOR bump. CORRECT.
- **Lint**: `content lint --target the-ledger` -> `0 error(s), 0 warning(s),
  0 finding(s), 5 scenario(s) balance-audited, 1 acked`. The single ack is the
  ch4 SELL-branch Auditor close-spawn (301u), telegraphed with 8s engage_delay +
  warning line; the burn-branch duplicate ack was pruned. CLEAN.
- **Tests**: gen_portal_gate 4, ledger_ch2_encounter 12, ledger_ch3_channel 9,
  ledger_ch4_ending 10, ledger_skybox 6. All GREEN, matching NOTES.
- **Catalog regen**: `gen-portal.py ... --out $CLAUDE_JOB_DIR/tmp/review-catalog`
  EXIT 0. `catalog.json` has a `the-ledger` entry at version `1.6.0`
  (meta.version 1.6.0, 8 files with sha256, 432604 bytes). VERIFIED.
- **CHANGELOG accuracy**: every `## 1.6.0` claim checks out against the content
  diff - 8s engage_delay grace (ch4 line 456), two invulnerable boulders (ch3
  lines 474/490), SetSkybox on ch1/ch3/ch4 only and ch2/ch2b/burn unchanged
  (grep counts 1/0/0/1/1). No overclaims, no omitted major change. ACCURATE.
- **README accuracy (the critical check)**: the new ch4 line correctly states
  the divergence - SELL calls the Auditor, BURN "the Auditor never comes, no
  fight, but no payout either. Clear, and broke." Confirmed against the shipped
  `ledger_ch4.content.ron`: exactly ONE `id: "auditor"` spawn (line 449), inside
  the SELL branch (header line 409); the BURN branch (line 1110+) spawns no
  Auditor and latches act->3 with its own terminal Victory. The old "then survive
  the Auditor. Two endings" line is gone. CORRECT and no longer misleading.
- **ch3 / Authoring-notes**: README ch3 line (nav drops in order, debris pinch,
  Magpies, both optional) and the Authoring notes additions (dwell holds, clock-
  paced briefings, lazy objectives, beat_gate spacing, per-chapter cubemap +
  SetSkybox accents) are all truthful to the content. ACCURATE.
- **v0.8.0 news note**: `docs/news-0.8.0-the-ledger.md` exists, is game-centric
  (describes the campaign, no agentic-process angle), and its four bullets are
  accurate to the shipped behavior. GOOD.
- **Scope**: only `docs/news-0.8.0-the-ledger.md`, this task's own
  `tasks/20260722-214119/{TASK,NOTES}.md`, `web/src/wiki/dev/guide-make-a-mod.md`
  (version bump), and the mod's own docs + bundle changed. Root `CHANGELOG.md`
  correctly LEFT ALONE (historical per-release entries). Installed catalog
  `assets/mods.catalog.ron` correctly untouched (live publish deferred to owner).
  No stray `tasks/` edits. CORRECT.

### Findings

- **LOW - sweep table omits the `web/src/news/` directory**
  (`tasks/20260722-214119/NOTES.md:79-94`).
  The doc-sweep grep in NOTES only covered `web/src/wiki/ README.md CHANGELOG.md`.
  My own broader grep found stale-looking Ledger facts in a live surface the table
  never mentions: `web/src/news/0.7.0.md` (line 122 "Four chapters, two endings",
  line 126 "pick the buyer - or the buoy", lines 104/423 "Auditor ... on both
  ending branches"). I investigated: `web/src/news/0.7.0.md` is a titled, dated
  per-release news post ("v0.7.0 - A game you can win or lose") in an append-only
  archive (`0.1.0.md` .. `0.7.0.md`). It describes what shipped AT v0.7.0, when
  the Ledger genuinely was two endings that both fought the Auditor. Like the root
  CHANGELOG, editing it would falsify history, so leaving it is CORRECT - the
  forthcoming v0.8.0 post (scratch = the new `docs/news-0.8.0-the-ledger.md`) is
  where the 1.6.0 divergence gets described. The finding is only that the sweep
  table's completeness claim is not fully substantiated: it should have listed
  `web/src/news/` and recorded the "historical per-release, left alone" verdict
  explicitly (same rationale it already applied to the root CHANGELOG), rather
  than silently scoping the grep to `wiki/`. No live "current-state" surface is
  stale. Suggested change: add a `web/src/news/0.7.0.md` row to the sweep table
  marked "historical release note - left alone (same rule as root CHANGELOG)".

No HIGH or MEDIUM findings. The two lessons this task exists to honor
(`write-prose-from-the-diff`, `keep-docs-in-sync`) are satisfied: the prose is
grounded in the content diff, every current-state live surface is synced, and the
one directory the table missed turns out to be correctly-left-alone history.

## Verdict
APPROVE
