# Retro: Web: rework the site onto the PHOSPHOR skin only (drop the hardware material)

- TASK: 20260731-154015
- BRANCH: fix/web-phosphor-only
- REVIEW ROUNDS: 2 (round 1 REQUEST_CHANGES, round 2 APPROVE)

## What went well

- The out-of-context reviewer found a defect the in-context author could not
  have seen: the new consumption check was satisfied by two palette tokens the
  OLD sheet already read. Its method - replicate the check, run it against the
  rejected input - is what exposed it.
- Fixing the review findings by making each new assertion FAIL on purpose (the
  old sheet, a planted alias) turned two "looks right" checks into proven ones.
- Looking at renders rather than exit codes caught the mermaid diagram staying
  grey through two config edits that both compiled and passed every test.

## What went wrong

- The port that landed earlier today (20260731-143918) chose the wrong one of
  the two skins the shared PoC ships. The decision seemed sound then:
  DECISION.md framed the choice as "cheap retint vs full material port" and
  picked the full port, which is the right answer to THAT question. The question
  was wrong - the PoC's `:root` IS the hardware skin's vocabulary, and the
  file's own comment says so, while the phosphor overrides live 250 lines
  further down in a block that was never read. `:root` being the obvious,
  top-of-file thing to mirror is what made the miss easy.
- The first version of check (e) was vacuous, written by someone who already
  knew what the answer should be, and never run against the sheet it was meant
  to reject.
- Seven recessed surfaces drew their hairline twice - and NOTES.md had already
  written down the rule that would have prevented it, one section above.

## What to improve next time

- When a shared source offers more than one variant, name the variant in the
  plan, not just the source file. "Mirror the PoC" was not a specification.
- A new "the code must USE X" assertion is only meaningful once it has been run
  against a tree where X is absent. Do that in the sitting it is written.
- A rule written into NOTES.md mid-task is worth one grep across the diff before
  committing; the second violation is usually already in it.

## Action items

- Ledger: bumped `assert-the-new-vocabulary-is-consumed` to x2 and sharpened it
  with the fail-first requirement.
- Ledger: added `name-the-variant-when-the-source-ships-several` (x1).
- No follow-up task: DoD 6 stays a pending owner check.
