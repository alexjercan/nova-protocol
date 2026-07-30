# Retro: the objective chip IS the posting

- TASK: 20260729-211200
- BRANCH: feat/objective-chip-is-the-posting
- ROUNDS: 2 (round 1 REQUEST_CHANGES on 1 MAJOR + 1 MINOR + 1 NIT, round 2
  APPROVE)

## What went well

**The DECISION.md did the thinking before the branch existed.** The plan gate
had already established that "keep the module dormant" could not stand on its
own - with no card, no tuck ever arrives, so every posting would sit out the
`REVEAL_TOTAL_SECS` fallback, which is the opposite of the immediacy the owner
asked for. Because that fork was settled and written down, the implementation
had nothing to discover: delete, ungate, sweep. Every mid-build decision was
mechanical. This is what the flow guideline about confirming the ARTIFACT (not
just the goal) is FOR, and it paid here.

**Fail-first was cheap and real.** Rewriting
`a_posting_shows_the_objective_text_not_a_count` to assert the chip on the
posting frame and running it against the unmodified tree took one command and
produced `left: [] right: ["SALVAGE THE WRECK"]` - red at the gate, not at a
typo. Cheap because the assertion the task wanted was already almost the
assertion the test made; only the timing changed.

**Salvaging assertions out of deleted tests worked as designed.** Four tests
died with the card. Reading each ASSERTION rather than each test found two that
were pinning stack behaviour nothing else covered, and they came back as
`each_posting_runs_its_own_dwell` and
`a_re_worded_objective_shows_its_chip_on_the_same_frame`. The out-of-context
reviewer independently checked both for revert-sensitivity and agreed. Net
-4/+2 tests with no coverage hole, on a diff that removed ~330 lines.

## What went wrong

**The doc sweep grepped SYMBOLS and missed PROSE (R1.1, MAJOR).** I swept for
`objective_reveal`, `NovaOsTabAnchor`, `ObjectiveRevealTucked` and the word
"reveal", fixed the four hits that produced, and ticked Step 5. But
`web/src/wiki/hud.md:66` describes the card for a whole sentence - "gets the
cockpit moment: it appears slightly rotated on the HUD, holds ... then tucks
into the objective stack ... A chip pops as the card lands" - without ever
using the module's name or the word "reveal". A symbol grep cannot reach it.

Root cause: I let the DELETED IDENTIFIERS define the search space. For a
player-facing feature that is exactly wrong - the wiki describes what the
player SEES, in the player's words, and never names the module. The correct
search space is the feature's OBSERVABLE description: what did this thing look
like, and what words would a writer use for it? "rotated", "holds", "tucks",
"card". I found the same page's line 31 only because it happened to contain the
word "reveal"; line 66, the fuller description, was invisible to me and would
have shipped a wiki page selling a feature the diff deletes.

Note this is `keep-docs-in-sync-with-code` (x8, already promoted to AGENTS.md)
recurring in a NEW shape. The promoted prose says "grep the whole doc tree",
and I did grep the whole tree - just for the wrong strings. The rule as written
guards the SCOPE of the sweep and says nothing about its QUERY, which is the
half that failed here.

**Stale source comments in a file the diff barely touched (R1.2).** Three
comments in `nova_os.rs` still described the anchor and named a test that no
longer exists. I swept that file for the symbol I was deleting and stopped;
`doc-sweep-covers-source-doc-comments` is a ledger lesson already, and it
applied to a file I had edited by three lines.

**A leftover refactor shim (R1.3).** `let age = shown.age_secs;` was the
mechanical residue of `let popped_for = *pop;`. Harmless, but it is the kind of
thing a fresh reader trips on, and I did not re-read the collapsed function as
a whole after the edit.

All three are the same shape: after the change, I verified the things I had
CHANGED and did not re-read the neighbourhoods I had changed them IN.

## What to improve next time

When deleting a user-visible feature, sweep the docs for its **description**,
not only its identifiers: write down 3-5 phrases a writer would use for the
thing (its shape, its motion, its moment - here "rotated", "holds", "tucks",
"cockpit moment", "as the card") and grep those across the live doc tree
BEFORE ticking the doc step. The identifier grep proves the CODE is gone; only
the prose grep proves the DOCS are. That is the new lesson below, and it
generalises past this task: any removal of something a player can see.

Second, smaller: after collapsing a data structure (two clocks to one, an
`Option` to a plain field), re-READ every function that touched the old shape
end to end rather than only the lines the compiler forced. The compiler caught
every type error and zero of the readability residue.

## Ledger

- New: `sweep-docs-for-the-feature-description-not-just-its-symbols` (x1).
- Bumped: `keep-docs-in-sync-with-code` (x8 -> x9), noting the query-vs-scope
  distinction the new lesson sharpens.
- Bumped: `deleting-a-test-salvage-live-assertions` (x2 -> x3) - it worked as
  intended here, which is itself the evidence that it belongs in the standard
  routine for any deletion diff. This takes it to three occurrences, so it goes
  to Pending promotions targeted at the work skill's verify step.

## Follow-ups

- Task 20260730-111146 (backlog): `web/src/tutorial.html` still names the
  objective PANEL retired in 20260724-134312. Pre-existing drift found by the
  reviewer's wider grep, filed rather than folded in. Its DoD was strengthened
  during round 2 after the reviewer noted the single-hit grep could be
  satisfied without actually sweeping the page.
- Task 20260729-182853 (backlog): its premise - a screenshot beat for the
  card-to-chip handover - was deleted by this task. Annotated in place with a
  dated note and a re-scope instruction rather than closed, since the chip's
  own arrival is still unseeable and still worth capturing.
- DoD 5 remains open: the owner playtest. Worth watching there that
  `OBJECTIVE_DWELL_SECS` (12 s) now runs from the posting, so an objective is
  on screen ~3.2 s less than before.
