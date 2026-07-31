# Retro: Port the NOVA OS phosphor theme to the web app

- TASK: 20260731-143918
- BRANCH: feat/web-nova-os-theme
- REVIEW ROUNDS: 2

## What went well

- **Writing the parity test first turned a 1600-line restyle into something that
  could not be half-finished.** Its check (d) - every `var()` read resolves -
  found leftover `--panel` reads in rules I had not reached yet, and check (e) -
  the material tokens are actually CONSUMED by the main surfaces - guarded the
  exact failure DECISION.md rejected the cheap option for: a site that takes the
  new colours but keeps the flat structure. A colour-only test would have gone
  green on precisely the wrong result.
- **The test parses the shared source instead of embedding a copy of it.** It
  reads `examples/ui/nova_ui_rework_poc.html` at test time, so the site and the
  game are no longer two hand-synced lists that can quietly diverge.
- **Building the capture rig before touching CSS paid twice.** Once for the
  eyeball itself, and once for the BEFORE set - without which I would have
  reported two pre-existing layout quirks (the mobile SCROLL/LINUX overlap, the
  underlined amber wiki-index cards) as damage I had caused. Cropping both sets
  at identical geometry is what settled it.
- The review's four findings were all real and all cheap. Fixing them rather
  than landing an APPROVE with known defects cost one short round.

## What went wrong

- **A DoD command that could never pass shipped in the approved plan.** DoD 2's
  grep used `\-\-panel\b`, which in ERE also matches the NEW `--panel-radius`
  token, so the "no legacy token survives" proof returned 15 hits on a perfectly
  ported file. At planning time the pattern looked right: it was assembled by
  listing the retired names and adding `\b` to each, and `\b` is the correct
  instinct for "whole token". What that missed is that `--panel-radius` did not
  exist yet - it arrives in the same change - so the regex was written against a
  tree the task was about to invalidate. Diagnosed only at verification, by
  running the command and reading all 15 hits.
- **The new capture rig reported success over error pages.** `chromium
  --screenshot` exits 0 and writes a perfectly valid PNG of a 404, and the rig's
  only guard was `[[ -s "$file" ]]` - a file-size check that such a PNG passes.
  The decision seemed sound because "the file exists and is non-empty" is the
  usual shape of a capture guard, and the failure is invisible in the happy path
  I tested. The reviewer caught it; forcing the failure confirmed a stale path
  produced a full green run. This is a rig whose whole job is to be the proof for
  DoD 4, so a silent pass is the worst possible defect in it.
- **Placeholder text kept a colour my own NOTES.md forbade.** I computed the
  contrast table, wrote the rule ("`--phosphor-muted` is ornament only"), applied
  it to eyebrows, meta, captions and nav - and then set
  `.wiki-search::placeholder` to `--phosphor-muted` anyway. The rule was applied
  where I was thinking about labels; a placeholder did not register as one.

## What to improve next time

- Run a proof command against the CURRENT tree while writing it into a DoD, and
  re-run it once the new names exist. Ten seconds at plan time versus a round of
  confusion at verification.
- When a proof command's pattern mentions a token the task is about to
  introduce, check the new name against the pattern explicitly - a
  retired-name grep and a new-name introduction in one change is exactly where a
  word-boundary assumption breaks.
- For any capture/export tool, decide what a FAILED run looks like before
  writing the happy path, and force that failure once. "The file is non-empty" is
  almost never the right guard.
- After writing a rule into NOTES.md, grep the diff for every site the rule
  covers rather than trusting that I applied it while writing it.

## Action items

- None requiring a follow-up task. The two out-of-scope items the port surfaced
  are already tracked: re-capturing the site's game screenshots
  (20260724-082856) and shipping Iosevka to the browser (20260714-214329). The
  pre-existing wiki-index card styling (whole-card underline, `.prose h3`
  outranking `.wiki-index__cardtitle`) is noted in NOTES.md and left alone
  deliberately; it predates this branch.
- Ledger: bumped `pin-mirrored-list-against-source` (x2) and
  `validate-proof-command-shape-at-plan-time` (x5 - the DoD-grep failure above is
  a 5th occurrence of that already-promoted lesson, not a new one; the /lessons
  pass caught that /compound had filed it as a duplicate slug and merged it).
  Added `capture-rig-succeeds-on-an-error-page` and
  `assert-the-new-vocabulary-is-consumed`.
