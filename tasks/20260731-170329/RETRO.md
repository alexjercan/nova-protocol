# Retro: KISS: nova_gameplay HUD - combat readout widgets

- TASK: 20260731-170329
- BRANCH: refactor/kiss-hud-combat-readout
- REVIEW ROUNDS: 1

## What went well

Reading the sibling's landed output before applying the rubric. A literal
read of the epic's comment rubric would have converted several hundred bare
`//` comments into markers; child 1 (20260731-170322) had already resolved
that tension in practice, keeping 486 bare comments against 1 NOTE. Checking
what the epic actually produced, rather than what its prose says, kept the
two children consistent and avoided a large pointless diff.

Checking pointers instead of judging prose. "Is this comment narration?" is a
judgment call every time; "does `docs/spikes/lock-dwell.md` exist?" is not.
Seven dead pointers fell out of a check that took one command, and they were
the highest-confidence deletions in the pass.

Proving no-behavior-change mechanically. An item-name multiset (485,
identical) plus a non-comment-line filter over `git diff -U0` reduced the
claim to something a reviewer could re-derive in one command - which the
reviewer did, twice.

## What went wrong

**The doc-warning baseline was captured differently from the measurement it
was compared against.** A `grep -c '^warning'` over a warm-cache `cargo doc`
run reported 2; the post-edit run reported 14, and for a while that read as a
regression I had introduced. The decision seemed sound at the time because
"take a baseline before editing" is the right instinct - but a baseline is
only a baseline if the extraction AND the cache state match the after-run.
The honest version was `git stash` -> `touch crates/nova_gameplay/src/lib.rs`
-> rerun -> `git stash pop`, which put master at the same 14.

**DoD 3's proof command was narrower than the claim it gated.** The grep
looks for `//.*[0-9]{8}-[0-9]{6}`, i.e. tatr IDs. It returned zero and I
reported the provenance sweep complete. Review found four surviving clauses -
two `(playtest 2026-07-13)`, two `(review R1.3)`/`(review R2.1)` - exactly
the same category of provenance, carrying no tatr ID, so the proof could
never have seen them. The proof was written against the most common shape of
the thing, not against the thing.

**A rewrap silently changed rendered output.** Deleting a mid-line clause in
`item_highlights.rs` left the following module-doc line starting with `- `,
which CommonMark parses as a list item. It compiles, formats clean, and
changes what rustdoc renders - inside a pass whose entire claim is "no
behavior change". Neither `cargo check` nor `cargo fmt` can see it.

## What to improve next time

- Capture a before/after baseline with one command run twice, not two
  commands. If the tool caches, force the rebuild on both sides.
- When a `cmd:` proof encodes a pattern, ask what OTHER shapes the same
  defect takes before treating zero hits as done. Here: provenance without an
  ID.
- After bulk comment edits, re-read the touched doc blocks as markdown, not
  just as text. A line that begins with `-`, `#`, `>`, or a number-dot is a
  block-level construct.

## Action items

- 20260731-205553 (backlog) - the pre-existing nova_gameplay warnings this
  pass was forbidden to fix: 4 `ambiguous import visibility` in the NOVA OS
  drawer mods, plus `ammo_readout`'s public-doc link to a private fn.
